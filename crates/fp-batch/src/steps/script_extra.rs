//! `script.code` and `script.external` (B-4, `docs/BATCH.md` §3.12).
//!
//! - `script.code` runs a JSF snippet against the item's document in its own headless session
//!   (like `script.command`). By default the snippet runs **sandboxed**: a prelude replaces `Files`
//!   and `System` with objects that refuse every call, wraps `fw`/`app` and the document objects so
//!   that file, export, launch and save methods throw, and lets `fp.run` refuse the blocked command
//!   groups (`file.`, `batch.`, `commands.`, `app.` …). `allowIo: true` in the step turns the
//!   sandbox off. Globals: `batch.index`, `batch.total`, `batch.source`.
//! - `script.external` runs an external program with `{in}`/`{out}` placeholders (no shell; the
//!   command line is split like a Windows command line with double quotes), a timeout and the test
//!   profile environment. It is **opt-in**: the step needs `allowExternal: true`, otherwise the
//!   validation refuses it. `{in}` is a temporary PNG/TIFF/JPEG of the current image; with
//!   `result: replace` the file at `{out}` becomes the new image.

use crate::error::{BatchError, Result};
use crate::item::{Item, ItemKind};
use crate::output::{self, OutputFormat};
use crate::step::{Issue, ParamDef, Step, StepContext, StepGroup, StepRegistry, validate_params};
use crate::steps::script::{BLOCKED_PREFIXES, run_on_item};
use fp_engine::Session;
use serde_json::{Map, Value, json};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// ID "JSF code".
pub const SCRIPT_CODE: &str = "script.code";
/// ID "External program".
pub const SCRIPT_EXTERNAL: &str = "script.external";

/// Method names that the sandbox refuses on `fw` and documents (prefix match, see
/// [`SANDBOX_ALLOWED`]).
pub const SANDBOX_DENIED_PREFIXES: [&str; 17] = [
    "open",
    "save",
    "export",
    "import",
    "launch",
    "browse",
    "quit",
    "runScript",
    "createFile",
    "createDirectory",
    "deleteFile",
    "readline",
    "readPanelState",
    "writePanelState",
    "getenv",
    "getTempFilePath",
    "setPref",
];

/// Names that match a denied prefix but touch no files.
pub const SANDBOX_ALLOWED: [&str; 6] = [
    "saveSelection",
    "savedSelections",
    "exportOptions",
    "exportSettings",
    "exportFormatOptions",
    "exportOptionsToUse",
];

/// Registers both steps.
pub fn register(r: &mut StepRegistry) {
    r.register(Arc::new(ScriptCode));
    r.register(Arc::new(ScriptExternal));
}

fn p_str<'a>(p: &'a Value, k: &str, d: &'a str) -> &'a str {
    p.get(k).and_then(Value::as_str).unwrap_or(d)
}
fn p_bool(p: &Value, k: &str) -> bool {
    p.get(k).and_then(Value::as_bool).unwrap_or(false)
}

/// JavaScript prelude of the sandbox.
pub fn sandbox_prelude() -> String {
    let denied = serde_json::to_string(&SANDBOX_DENIED_PREFIXES).unwrap_or_default();
    let allowed = serde_json::to_string(&SANDBOX_ALLOWED).unwrap_or_default();
    let blocked = serde_json::to_string(&BLOCKED_PREFIXES).unwrap_or_default();
    format!(
        r#"(function (g) {{
  var denied = {denied}, allowed = {allowed}, blocked = {blocked};
  function refuse(n) {{ return function () {{ throw new Error("sandbox: " + n + " is not allowed in a batch (allowIo: false)"); }}; }}
  function isDenied(k) {{
    if (typeof k !== "string" || allowed.indexOf(k) >= 0) return false;
    for (var i = 0; i < denied.length; i++) if (k.indexOf(denied[i]) === 0) return true;
    return false;
  }}
  var cache = [];
  function wrap(o, deep) {{
    if (o === null || (typeof o !== "object" && typeof o !== "function")) return o;
    for (var i = 0; i < cache.length; i++) if (cache[i][0] === o) return cache[i][1];
    var p = new Proxy(o, {{
      get: function (t, k) {{
        if (isDenied(k)) return refuse(String(k));
        var v = t[k];
        if (typeof v === "function") {{
          return function () {{
            var r = v.apply(t, arguments);
            return deep ? wrap(r, false) : r;
          }};
        }}
        return deep ? wrap(v, false) : v;
      }},
      set: function (t, k, v) {{ t[k] = v; return true; }}
    }});
    cache.push([o, p]);
    return p;
  }}
  var none = new Proxy({{}}, {{ get: function (t, k) {{ return refuse(String(k)); }} }});
  g.Files = none;
  g.System = none;
  var fw = wrap(g.fw, true);
  g.fw = fw; g.app = fw; g.App = fw; g.firepixel = fw;
  if (g.fp && g.fp.run) {{
    var run = g.fp.run;
    g.fp = {{
      run: function (name) {{
        for (var i = 0; i < blocked.length; i++)
          if (String(name).indexOf(blocked[i]) === 0) return refuse(String(name))();
        return run.apply(null, arguments);
      }},
      available: g.fp.available,
      t: g.fp.t
    }};
  }}
}})(this);"#
    )
}

// =================================================================== script.code

struct ScriptCode;

impl Step for ScriptCode {
    fn id(&self) -> &str {
        SCRIPT_CODE
    }
    fn group(&self) -> StepGroup {
        StepGroup::Advanced
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::text("code", "").beginner(),
            ParamDef::bool("allowIo", false),
        ]
    }
    fn validate(&self, params: &Value) -> Vec<Issue> {
        let mut out = validate_params(&self.params(), params);
        if p_str(params, "code", "").trim().is_empty() {
            out.push(Issue::param("code", fp_i18n::t("batch-issue-code-empty")));
        }
        out
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let code = p_str(params, "code", "").to_string();
        if code.trim().is_empty() {
            return Err(BatchError::Invalid(fp_i18n::t("batch-issue-code-empty")));
        }
        let opts = fp_script::ScriptOptions {
            globals: vec![(
                "batch".into(),
                json!({
                    "index": ctx.index,
                    "total": ctx.total,
                    "source": ctx.info.path.to_string_lossy(),
                }),
            )],
            prelude: (!p_bool(params, "allowIo")).then(sandbox_prelude),
            confirm_answer: true,
            loop_limit: 50_000_000,
            ..fp_script::ScriptOptions::default()
        };
        run_on_item(item, ctx, true, &|s: &mut Session| {
            let o = fp_script::run_source(s, &code, &opts);
            if o.ok {
                Ok(())
            } else {
                let mut msg = o.error.unwrap_or_else(|| fp_i18n::t("err-script-failed"));
                if let Some(d) = o.diagnostics.last() {
                    msg = format!("{msg} ({d})");
                }
                Err(BatchError::Invalid(fp_i18n::tr!(
                    "batch-err-code-failed",
                    detail = msg
                )))
            }
        })
    }
    fn summary(&self, params: &Value) -> String {
        p_str(params, "code", "")
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or_default()
            .chars()
            .take(40)
            .collect()
    }
}

// =================================================================== script.external

/// Splits a command line into arguments (double quotes group, `""` inside quotes = `"`).
pub fn split_command_line(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut any = false;
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '"' if quoted && it.peek() == Some(&'"') => {
                cur.push('"');
                it.next();
            }
            '"' => {
                quoted = !quoted;
                any = true;
            }
            c if c.is_whitespace() && !quoted => {
                if any || !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                    any = false;
                }
            }
            c => cur.push(c),
        }
    }
    if any || !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Arguments with `{in}`, `{out}`, `{name}` replaced.
pub fn expand(args: &[String], input: &Path, out: &Path, name: &str) -> Vec<String> {
    args.iter()
        .map(|a| {
            a.replace("{in}", &input.to_string_lossy())
                .replace("{out}", &out.to_string_lossy())
                .replace("{name}", name)
        })
        .collect()
}

#[cfg(windows)]
fn hide_console(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_console(_cmd: &mut Command) {}

fn temp_dir() -> Result<PathBuf> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let d = std::env::temp_dir().join(format!(
        "firepixel-batch-ext-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&d).map_err(|e| BatchError::Io(e.to_string()))?;
    Ok(d)
}

struct ScriptExternal;

impl Step for ScriptExternal {
    fn id(&self) -> &str {
        SCRIPT_EXTERNAL
    }
    fn group(&self) -> StepGroup {
        StepGroup::Advanced
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::text("command", "").beginner(),
            ParamDef::choice("inputFormat", &["png", "tiff", "jpeg"], "png"),
            ParamDef::choice("result", &["replace", "ignore"], "replace"),
            ParamDef::int("timeout", 1, 3600, 60).unit("s"),
            ParamDef::bool("allowExternal", false),
        ]
    }
    fn validate(&self, params: &Value) -> Vec<Issue> {
        let mut out = validate_params(&self.params(), params);
        if !p_bool(params, "allowExternal") {
            out.push(Issue::param(
                "allowExternal",
                fp_i18n::t("batch-issue-external-optin"),
            ));
        }
        let args = split_command_line(p_str(params, "command", ""));
        if args.is_empty() {
            out.push(Issue::param(
                "command",
                fp_i18n::t("batch-issue-external-empty"),
            ));
        } else if p_str(params, "result", "replace") == "replace"
            && !args.iter().any(|a| a.contains("{out}"))
        {
            out.push(Issue::param(
                "command",
                fp_i18n::t("batch-issue-external-no-out"),
            ));
        }
        out
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn preview_ok(&self) -> bool {
        false
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        if !p_bool(params, "allowExternal") {
            return Err(BatchError::Invalid(fp_i18n::t(
                "batch-issue-external-optin",
            )));
        }
        let args = split_command_line(p_str(params, "command", ""));
        let Some(program) = args.first().cloned() else {
            return Err(BatchError::Invalid(fp_i18n::t(
                "batch-issue-external-empty",
            )));
        };
        let fmt = match p_str(params, "inputFormat", "png") {
            "tiff" => OutputFormat::Tiff,
            "jpeg" => OutputFormat::Jpeg,
            _ => OutputFormat::Png,
        };
        let dir = temp_dir()?;
        let result = (|| -> Result<()> {
            let ext = fmt.extension(false);
            let input = dir.join(format!("in.{ext}"));
            let out = dir.join(format!("out.{ext}"));
            let (bytes, _) = output::encode(item, fmt, &Map::new())?;
            std::fs::write(&input, bytes).map_err(|e| BatchError::Io(e.to_string()))?;
            let name = ctx.info.file_name();
            let full = expand(&args, &input, &out, &name);
            let mut cmd = Command::new(&full[0]);
            cmd.args(&full[1..])
                .current_dir(&dir)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::piped());
            fp_i18n::profile::child_env(&mut cmd);
            hide_console(&mut cmd);
            let mut child = cmd.spawn().map_err(|e| {
                BatchError::Io(fp_i18n::tr!(
                    "batch-err-external-start",
                    program = program.clone(),
                    error = e.to_string()
                ))
            })?;
            let timeout = Duration::from_secs(
                params
                    .get("timeout")
                    .and_then(Value::as_u64)
                    .unwrap_or(60)
                    .clamp(1, 3600),
            );
            let t0 = Instant::now();
            let status = loop {
                match child.try_wait() {
                    Ok(Some(s)) => break s,
                    Ok(None) if ctx.cancel.is_cancelled() => {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(BatchError::cancelled());
                    }
                    Ok(None) if t0.elapsed() > timeout => {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(BatchError::Io(fp_i18n::tr!(
                            "batch-err-external-timeout",
                            program = program.clone(),
                            seconds = timeout.as_secs()
                        )));
                    }
                    Ok(None) => std::thread::sleep(Duration::from_millis(20)),
                    Err(e) => return Err(BatchError::Io(e.to_string())),
                }
            };
            if !status.success() {
                let mut err = String::new();
                if let Some(mut s) = child.stderr.take() {
                    use std::io::Read;
                    let _ = s.read_to_string(&mut err);
                }
                let tail: String = err.trim().chars().rev().take(300).collect::<String>();
                let tail: String = tail.chars().rev().collect();
                return Err(BatchError::Io(fp_i18n::tr!(
                    "batch-err-external-failed",
                    program = program.clone(),
                    code = status.code().unwrap_or(-1),
                    detail = tail
                )));
            }
            if p_str(params, "result", "replace") != "replace" {
                return Ok(());
            }
            let data = std::fs::read(&out).map_err(|_| {
                BatchError::Io(fp_i18n::tr!(
                    "batch-err-external-no-output",
                    program = program.clone()
                ))
            })?;
            let mut new = crate::item::decode_bytes(&format!("out.{ext}"), &data)?;
            new.flatten_document()?;
            let meta = item.meta().cloned();
            if let (Some(m), Some(nm)) = (meta, new.meta_mut()) {
                let has_t = nm.has_transparency;
                *nm = m;
                nm.has_transparency = has_t;
            }
            *item = new;
            Ok(())
        })();
        let _ = std::fs::remove_dir_all(&dir);
        result
    }
    fn summary(&self, params: &Value) -> String {
        split_command_line(p_str(params, "command", ""))
            .first()
            .and_then(|p| {
                Path::new(p)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_lines() {
        assert_eq!(
            split_command_line(r#""C:\Program Files\x.exe" -i {in} -o "{out}" """#),
            vec![r"C:\Program Files\x.exe", "-i", "{in}", "-o", "{out}", ""]
        );
        assert_eq!(split_command_line("  a  b "), vec!["a", "b"]);
        assert_eq!(split_command_line(r#""a ""q"" b""#), vec![r#"a "q" b"#]);
        let e = expand(
            &["{in}".into(), "x{out}".into()],
            Path::new("i.png"),
            Path::new("o.png"),
            "n",
        );
        assert_eq!(e, vec!["i.png", "xo.png"]);
    }
}

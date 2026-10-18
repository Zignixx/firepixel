//! `script.code` and `script.external` (B-4, `docs/BATCH.md` §3.12).
//!
//! - `script.code` runs a JSF snippet against the item's document in its own headless session
//!   (like `script.command`). By default the snippet runs **sandboxed** (deep guard, see
//!   [`sandbox_prelude`]): `Files`/`System` refuse every call, every API prototype has its file,
//!   export, launch and save methods replaced by throwing stubs, every object returned from the API
//!   graph is wrapped recursively, and `fp.run` only calls commands of an allowlist of safe groups.
//!   `allowIo: true` in the step turns the sandbox off (refused for MCP callers,
//!   [`crate::security`]). Globals: `batch.index`, `batch.total`, `batch.source`.
//! - `script.external` runs an external program with `{in}`/`{out}` placeholders (no shell; the
//!   command line is split like a Windows command line with double quotes), a timeout and the test
//!   profile environment. It is **opt-in**: the step needs `allowExternal: true`, otherwise the
//!   validation refuses it; MCP callers are always refused ([`crate::security`]). `{in}` is a temporary PNG/TIFF/JPEG of the current image; with
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

/// Method names that the sandbox refuses on every object of the API graph (prefix match, see
/// [`SANDBOX_ALLOWED`]).
pub const SANDBOX_DENIED_PREFIXES: &[&str] = &[
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
    "eval",
    "clip",
    "doBatchProcess",
    "takeScreenshot",
    "associate",
    "updateHTML",
    "sendEmail",
    "loadColorPalette",
    "newDocumentFromTemplate",
    "revertDocument",
    "addProjectEntry",
    "logChangeInProject",
    "removeAllProjectEntries",
    "findApp",
    "chooseBrowser",
    "locateDocDialog",
    "getHTMLFileForScript",
    "copyHtmlWizard",
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

/// API classes whose every method and property the sandbox refuses (file system, system,
/// open files).
pub const SANDBOX_SEALED_CLASSES: [&str; 3] = ["Files", "System", "FileReference"];

/// Command groups `fp.run` may call inside the sandbox (allowlist; everything else is refused).
pub const SANDBOX_SAFE_GROUPS: [&str; 18] = [
    "autoShape.",
    "bitmap.",
    "color.",
    "document.",
    "draw.",
    "edit.",
    "element.",
    "filter.",
    "guide.",
    "history.",
    "layer.",
    "modify.",
    "page.",
    "path.",
    "pi.",
    "select.",
    "text.",
    "transform.",
];

/// Commands inside the safe groups that still touch files, preferences or the clipboard
/// (prefix match).
pub const SANDBOX_DENIED_COMMANDS: [&str; 8] = [
    "edit.preferences",
    "edit.copy",
    "edit.paste",
    "edit.insert.bitmapViaCopy",
    "history.export",
    "pi.brush.save",
    "pi.export.",
    "pi.symbol.import",
];

/// May `fp.run` call `name` inside the sandbox?
pub fn sandbox_allows_command(name: &str) -> bool {
    SANDBOX_SAFE_GROUPS.iter().any(|g| name.starts_with(g))
        && !SANDBOX_DENIED_COMMANDS.iter().any(|d| name.starts_with(d))
        && !BLOCKED_PREFIXES.iter().any(|b| name.starts_with(b))
}

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

/// JavaScript prelude of the sandbox (deep guard).
///
/// 1. **Prototypes**: every class prototype of the API (`__fp_protos`, also the ones built later)
///    is rewritten once: denied methods and every member of the sealed classes throw; all other
///    native methods and property accessors get a wrapper that unwraps guarded `this`/arguments
///    before the native call and guards the result. So no path to a native file, export or launch
///    method is left, whatever object or prototype a script reaches.
/// 2. **Objects**: every object returned from the guarded graph is wrapped in a memoised proxy
///    (recursively on property reads) that refuses denied method names; assignments store the
///    unwrapped value.
/// 3. **Globals**: `Files`/`System` refuse everything, every global alias of `fw` and the other API
///    singletons are guarded, `fp.run` only calls commands of [`SANDBOX_SAFE_GROUPS`].
pub fn sandbox_prelude() -> String {
    let denied = serde_json::to_string(SANDBOX_DENIED_PREFIXES).unwrap_or_default();
    let allowed = serde_json::to_string(&SANDBOX_ALLOWED).unwrap_or_default();
    let sealed = serde_json::to_string(&SANDBOX_SEALED_CLASSES).unwrap_or_default();
    let safe = serde_json::to_string(&SANDBOX_SAFE_GROUPS).unwrap_or_default();
    let denied_cmds = serde_json::to_string(&SANDBOX_DENIED_COMMANDS).unwrap_or_default();
    let blocked = serde_json::to_string(&BLOCKED_PREFIXES).unwrap_or_default();
    format!(
        r#"(function (g) {{
  var denied = {denied}, allowed = {allowed}, sealed = {sealed};
  var safe = {safe}, deniedCmds = {denied_cmds}, blocked = {blocked};
  function refuse(n) {{ return function () {{ throw new Error("sandbox: " + n + " is not allowed in a batch (allowIo: false)"); }}; }}
  function starts(s, list) {{
    for (var i = 0; i < list.length; i++) if (s.indexOf(list[i]) === 0) return true;
    return false;
  }}
  function isDenied(k) {{
    return typeof k === "string" && allowed.indexOf(k) < 0 && starts(k, denied);
  }}
  function cmdAllowed(n) {{
    return starts(n, safe) && !starts(n, deniedCmds) && !starts(n, blocked);
  }}
  var toProxy = new WeakMap(), toRaw = new WeakMap();
  function isObj(v) {{ return v !== null && (typeof v === "object" || typeof v === "function"); }}
  function raw(v) {{ return isObj(v) && toRaw.has(v) ? toRaw.get(v) : v; }}
  function rawArgs(a) {{
    var out = [];
    for (var i = 0; i < a.length; i++) {{
      var v = raw(a[i]);
      if (Array.isArray(v)) {{
        var c = [];
        for (var j = 0; j < v.length; j++) c.push(raw(v[j]));
        v = c;
      }}
      out.push(v);
    }}
    return out;
  }}
  function guard(v) {{
    if (v === null || typeof v !== "object") return v;
    if (toRaw.has(v)) return v;
    var p = toProxy.get(v);
    if (p) return p;
    p = new Proxy(v, {{
      get: function (t, k) {{
        var x = t[k];
        if (typeof x === "function") return isDenied(k) ? refuse(String(k)) : x;
        return guard(x);
      }},
      set: function (t, k, x) {{ t[k] = raw(x); return true; }},
      defineProperty: function (t, k, d) {{
        if ("value" in d) d.value = raw(d.value);
        return Reflect.defineProperty(t, k, d);
      }},
      getOwnPropertyDescriptor: function (t, k) {{
        var d = Reflect.getOwnPropertyDescriptor(t, k);
        if (d && d.configurable && "value" in d) d.value = guard(d.value);
        return d;
      }}
    }});
    toProxy.set(v, p);
    toRaw.set(p, v);
    return p;
  }}
  var protos = g.__fp_protos, done = {{}}, seen = -1;
  function native(f) {{
    return function () {{
      var r = f.apply(raw(this), rawArgs(arguments));
      scan();
      return guard(r);
    }};
  }}
  function seal(cls, proto) {{
    var all = sealed.indexOf(cls) >= 0;
    var names = Object.getOwnPropertyNames(proto);
    for (var i = 0; i < names.length; i++) {{
      var k = names[i];
      if (k === "__fpc" || k === "constructor") continue;
      var d = Object.getOwnPropertyDescriptor(proto, k);
      if (!d || !d.configurable) continue;
      if (typeof d.value === "function") {{
        d.value = all || isDenied(k) ? refuse(cls + "." + k) : native(d.value);
      }} else if (d.get || d.set) {{
        if (all) {{
          d.get = refuse(cls + "." + k);
          if (d.set) d.set = refuse(cls + "." + k);
        }} else {{
          if (d.get) d.get = native(d.get);
          if (d.set) d.set = native(d.set);
        }}
      }} else continue;
      Object.defineProperty(proto, k, d);
    }}
  }}
  function scan() {{
    if (!protos) return;
    var names = Object.getOwnPropertyNames(protos);
    if (names.length === seen) return;
    seen = names.length;
    for (var i = 0; i < names.length; i++) {{
      var n = names[i];
      if (done[n]) continue;
      done[n] = true;
      var p = protos[n];
      if (isObj(p)) seal(n, p);
    }}
  }}
  var rawFw = g.fw;
  scan();
  var none = new Proxy({{}}, {{ get: function (t, k) {{ return refuse(String(k)); }} }});
  var fw = guard(rawFw);
  var globals = Object.getOwnPropertyNames(g);
  for (var i = 0; i < globals.length; i++) {{
    try {{ if (g[globals[i]] === rawFw) g[globals[i]] = fw; }} catch (e) {{}}
  }}
  g.fw = fw; g.app = fw; g.App = fw; g.firepixel = fw;
  g.Files = none;
  g.System = none;
  var singles = ["$", "Document", "Tools", "Errors", "Widget"];
  for (var s = 0; s < singles.length; s++) {{
    try {{ if (isObj(g[singles[s]])) g[singles[s]] = guard(g[singles[s]]); }} catch (e) {{}}
  }}
  if (g.fp && g.fp.run) {{
    var run = g.fp.run;
    g.fp = {{
      run: function (name) {{
        var n = String(name);
        if (!cmdAllowed(n)) return refuse(n)();
        var r = run.apply(null, rawArgs(arguments));
        scan();
        return guard(r);
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

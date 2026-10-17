//! Gruppe „Erweitert“ (SPEC-BATCH §3.13, Paket BATCH-4): `script.registry` (beliebige Befehle des
//! Registers) und `script.command` (gespeicherter Befehl aus dem Befehlsordner, JSF).
//!
//! Beide Schritte (und `filter.registry` aus `adjust.rs`) laufen in einer **eigenen headless
//! Sitzung** je Element (SPEC §5.2 Nr. 7), nie in der Sitzung der Oberfläche:
//! - Dokumente (FW-PNG/PSD/SVG) werden direkt bearbeitet und bleiben Dokumente (bearbeitbar).
//! - Einzelbilder und jedes Frame einer Animation werden zu einem Dokument mit einer Bitmap, die
//!   Befehle laufen darauf, danach wird die Leinwand flach gerechnet (wie der Export).
//!
//! Gesperrt sind Befehle, die Dateien, die Anwendung oder einen weiteren Batch betreffen
//! ([`BLOCKED_PREFIXES`]); Skripte laufen über `commands.runScript`.

use crate::error::{BatchError, Result};
use crate::item::{Item, ItemKind, Still};
use crate::output;
use crate::step::{
    Issue, ParamDef, Severity, Step, StepContext, StepGroup, StepRegistry, validate_params,
};
use crate::steps::geometry::{finish, p_bool, p_str, place, refresh_transparency};
use fp_engine::{CommandRegistry, Preferences, Session};
use fp_raster::{CLEAR, RgbaImage};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

/// ID „Firepixel-Befehl“.
pub const SCRIPT_REGISTRY: &str = "script.registry";
/// ID „Gespeicherter Befehl“.
pub const SCRIPT_COMMAND: &str = "script.command";

/// Befehlsgruppen, die im Batch nicht erlaubt sind (Dateien öffnen/speichern, Anwendung, Fenster,
/// Hilfe, ein Batch im Batch, Skripte – dafür `script.command`).
pub const BLOCKED_PREFIXES: [&str; 6] =
    ["batch.", "file.", "commands.", "app.", "help.", "window."];

/// Markierungen im Kopf (erste 511 Byte) einer Befehlsdatei, die sie vom Batch ausschließen
/// (`re\21-batch.md` §4.2).
pub const NOT_FOR_BATCH_MARKERS: [&str; 4] = [
    "//--not for batch--",
    "Batch Script Template",
    "var gBatchOperationsArray =",
    "var gBatchSettings =",
];

/// Gemeinsames Befehlsregister (einmal je Prozess gebaut).
pub fn command_registry() -> Arc<CommandRegistry> {
    static REG: OnceLock<Arc<CommandRegistry>> = OnceLock::new();
    REG.get_or_init(|| Arc::new(CommandRegistry::with_defaults()))
        .clone()
}

/// Alle umgesetzten `filter.*`-Befehle des Registers (ohne „Filter wiederholen“), sortiert.
pub fn filter_commands() -> &'static [String] {
    static LIST: OnceLock<Vec<String>> = OnceLock::new();
    LIST.get_or_init(|| {
        let mut v: Vec<String> = command_registry()
            .iter()
            .map(|c| c.info())
            .filter(|i| i.implemented && i.name.starts_with("filter.") && i.name != "filter.repeat")
            .map(|i| i.name.clone())
            .collect();
        v.sort();
        v
    })
}

/// Ist der Befehl im Batch gesperrt?
pub fn is_blocked(name: &str) -> bool {
    BLOCKED_PREFIXES.iter().any(|p| name.starts_with(p))
}

/// Prüft einen Befehlsnamen. `None` = in Ordnung, sonst übersetzte Meldung.
pub fn check_command(name: &str) -> Option<String> {
    if name.trim().is_empty() {
        return Some(fp_i18n::t("batch-issue-script-no-command"));
    }
    if is_blocked(name) {
        return Some(fp_i18n::tr!(
            "batch-issue-script-blocked",
            command = name.to_string()
        ));
    }
    match command_registry().get(name) {
        None => Some(fp_i18n::tr!(
            "batch-issue-script-unknown",
            command = name.to_string()
        )),
        Some(c) if !c.info().implemented => Some(fp_i18n::tr!(
            "batch-issue-script-unknown",
            command = name.to_string()
        )),
        _ => None,
    }
}

/// Neue headless Sitzung (gemeinsames Register; mit JSF-Laufzeit, wenn `script`).
pub fn new_session(script: bool) -> Session {
    let mut s = Session::with_registry(Preferences::new(), command_registry());
    if script {
        fp_script::install_host(&mut s);
    }
    s
}

fn engine_err(e: fp_engine::EngineError) -> BatchError {
    BatchError::Invalid(e.to_string())
}

/// Führt `f` in einer eigenen Sitzung auf dem Element aus (siehe Moduldoku). `f` bekommt die
/// Sitzung mit dem Element als aktivem Dokument.
pub(crate) fn run_on_item(
    item: &mut Item,
    ctx: &StepContext,
    script: bool,
    f: &dyn Fn(&mut Session) -> Result<()>,
) -> Result<()> {
    let run_doc = |doc: fp_model::Document| -> Result<fp_model::Document> {
        let mut s = new_session(script);
        s.add_document(doc, None);
        f(&mut s)?;
        let d = s.doc().map_err(engine_err)?;
        Ok(d.doc.as_ref().clone())
    };
    let render_still = |image: &RgbaImage, meta: &crate::item::Meta| -> Result<RgbaImage> {
        let doc = output::item_document(&Item::Still(Still {
            image: image.clone(),
            meta: meta.clone(),
        }))?;
        let out = run_doc(doc)?;
        crate::item::render_document(&out)
    };
    match item {
        Item::Document(d) => {
            let doc = run_doc(d.doc.as_ref().clone())?;
            d.doc = Arc::new(doc);
        }
        Item::Still(s) => {
            s.image = render_still(&s.image, &s.meta)?;
        }
        Item::Animation(a) => {
            let meta = a.meta.clone();
            let mut out = Vec::with_capacity(a.frames.len());
            for fr in &a.frames {
                ctx.check_cancel()?;
                out.push(render_still(&fr.image, &meta)?);
            }
            // gleiche Größe für alle Frames (größtes Frame, oben links)
            let w = out.iter().map(|i| i.width()).max().unwrap_or(0) as i64;
            let h = out.iter().map(|i| i.height()).max().unwrap_or(0) as i64;
            for (fr, img) in a.frames.iter_mut().zip(out) {
                fr.image = if img.width() as i64 == w && img.height() as i64 == h {
                    img
                } else {
                    place(&img, w, h, 0, 0, CLEAR)?
                };
            }
        }
        Item::FileOnly(_) => return Ok(()),
    }
    finish(item);
    refresh_transparency(item);
    Ok(())
}

/// Liste `[{command, params}]` aus `command`/`params` und `commands`.
fn command_list(p: &Value) -> Vec<(String, Value)> {
    let mut v = Vec::new();
    let first = p_str(p, "command", "").trim();
    if !first.is_empty() {
        v.push((
            first.to_string(),
            p.get("params").cloned().unwrap_or(json!({})),
        ));
    }
    if let Some(a) = p.get("commands").and_then(Value::as_array) {
        for c in a {
            match c {
                Value::String(s) => v.push((s.clone(), json!({}))),
                Value::Object(o) => v.push((
                    o.get("command")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    o.get("params").cloned().unwrap_or(json!({})),
                )),
                _ => v.push((String::new(), json!({}))),
            }
        }
    }
    v
}

// =================================================================== script.registry

/// `script.registry`: ein oder mehrere Befehle des Registers als ein Schritt.
struct ScriptRegistry;

impl Step for ScriptRegistry {
    fn id(&self) -> &str {
        SCRIPT_REGISTRY
    }
    fn group(&self) -> StepGroup {
        StepGroup::Advanced
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::text("command", "").beginner(),
            ParamDef::json("params", json!({})).beginner(),
            ParamDef::json("commands", json!([])),
            ParamDef::bool("selectAll", true),
        ]
    }
    fn validate(&self, params: &Value) -> Vec<Issue> {
        let mut out = validate_params(&self.params(), params);
        let list = command_list(params);
        if list.is_empty() {
            out.push(Issue::param(
                "command",
                fp_i18n::t("batch-issue-script-no-command"),
            ));
        }
        for (name, p) in &list {
            if let Some(m) = check_command(name) {
                out.push(Issue::param("command", m));
            } else if !(p.is_object() || p.is_null()) {
                out.push(Issue::param(
                    "params",
                    fp_i18n::t("batch-issue-params-not-object"),
                ));
            }
        }
        out
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let list = command_list(params);
        for (name, _) in &list {
            if let Some(m) = check_command(name) {
                return Err(BatchError::Invalid(m));
            }
        }
        let select_all = p_bool(params, "selectAll", true);
        run_on_item(item, ctx, false, &|s: &mut Session| {
            for (name, p) in &list {
                if select_all {
                    s.execute("select.all", json!({})).map_err(engine_err)?;
                }
                s.execute(name, if p.is_null() { json!({}) } else { p.clone() })
                    .map_err(engine_err)?;
            }
            Ok(())
        })
    }
    fn summary(&self, params: &Value) -> String {
        let list = command_list(params);
        match list.len() {
            0 => String::new(),
            1 => list[0].0.clone(),
            n => fp_i18n::tr!("batch-summary-script-registry-many", count = n as i64),
        }
    }
}

// =================================================================== script.command

/// `script.command`: gespeicherter Befehl (`.jsf` im Befehlsordner bzw. Datei).
struct ScriptCommand;

/// Ordner der gespeicherten Befehle (`Commands`, in Tests über `FP_COMMANDS_DIR` umlenkbar).
pub fn commands_dir() -> PathBuf {
    fp_engine::commands::misc::commands_dir()
}

/// Ist die Befehlsdatei vom Batch ausgeschlossen (Markierung in den ersten 511 Byte)?
pub fn excluded_from_batch(code: &[u8]) -> bool {
    let head = String::from_utf8_lossy(&code[..code.len().min(511)]).to_string();
    NOT_FOR_BATCH_MARKERS.iter().any(|m| head.contains(m))
}

/// Gespeicherte Befehle, die im Batch erlaubt sind: Name (relativ, `/` als Trenner, ohne `.jsf`) und Pfad.
pub fn batch_commands() -> Vec<(String, PathBuf)> {
    let dir = commands_dir();
    let mut out = Vec::new();
    for e in walkdir::WalkDir::new(&dir)
        .max_depth(4)
        .into_iter()
        .flatten()
    {
        let p = e.path();
        if !p.is_file() || !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("jsf")) {
            continue;
        }
        let Ok(code) = std::fs::read(p) else { continue };
        if excluded_from_batch(&code) {
            continue;
        }
        let rel = p.strip_prefix(&dir).unwrap_or(p).with_extension("");
        let name = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("/");
        out.push((name, p.to_path_buf()));
    }
    out.sort_by(|a, b| crate::item::natural_cmp(&a.0, &b.0));
    out
}

/// Pfad der Befehlsdatei aus `path` bzw. `name` (Name ohne `..`).
fn command_path(p: &Value) -> Option<PathBuf> {
    let path = p_str(p, "path", "").trim();
    if !path.is_empty() {
        return Some(PathBuf::from(path));
    }
    let name = p_str(p, "name", "").trim().trim_end_matches(".jsf");
    if name.is_empty()
        || Path::new(name)
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return None;
    }
    Some(commands_dir().join(format!("{name}.jsf")))
}

impl Step for ScriptCommand {
    fn id(&self) -> &str {
        SCRIPT_COMMAND
    }
    fn group(&self) -> StepGroup {
        StepGroup::Advanced
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::text("name", "").beginner(),
            ParamDef::path("path"),
        ]
    }
    fn validate(&self, params: &Value) -> Vec<Issue> {
        let mut out = validate_params(&self.params(), params);
        match command_path(params) {
            None => out.push(Issue::param(
                "name",
                fp_i18n::t("batch-issue-script-no-file"),
            )),
            Some(p) => match std::fs::read(&p) {
                Err(_) => {
                    let mut i = Issue::param(
                        "name",
                        fp_i18n::tr!("batch-issue-script-missing", path = p.display().to_string()),
                    );
                    i.severity = Severity::Warning;
                    out.push(i);
                }
                Ok(code) if excluded_from_batch(&code) => out.push(Issue::param(
                    "name",
                    fp_i18n::t("batch-issue-script-not-for-batch"),
                )),
                Ok(_) => {}
            },
        }
        out
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let path = command_path(params)
            .ok_or_else(|| BatchError::Invalid(fp_i18n::t("batch-issue-script-no-file")))?;
        let code = std::fs::read(&path).map_err(|_| {
            BatchError::Io(fp_i18n::tr!(
                "batch-issue-script-missing",
                path = path.display().to_string()
            ))
        })?;
        if excluded_from_batch(&code) {
            return Err(BatchError::Invalid(fp_i18n::t(
                "batch-issue-script-not-for-batch",
            )));
        }
        let arg = json!({"path": path.to_string_lossy()});
        run_on_item(item, ctx, true, &|s: &mut Session| {
            s.execute("commands.runScript", arg.clone())
                .map_err(engine_err)?;
            Ok(())
        })
    }
    fn summary(&self, params: &Value) -> String {
        command_path(params)
            .and_then(|p| p.file_stem().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_default()
    }
}

/// Registriert die Schritte der Gruppe.
pub fn register(r: &mut StepRegistry) {
    r.register(Arc::new(ScriptRegistry));
    r.register(Arc::new(ScriptCommand));
}

//! Menüs „Befehle“, „Fenster“ (Fensteranordnung) und „Hilfe“ sowie Dateibrowser und Scannen.
//!
//! * Befehle: Skript ausführen und gespeicherte Befehle laufen über die eingesteckte
//!   [`crate::session::ScriptHost`] (fp-script) als **ein** Verlaufsschritt. Gespeicherte Befehle sind
//!   `.jsf`-Dateien im Ordner `Commands` neben der Voreinstellungsdatei (oder `FP_COMMANDS_DIR`),
//!   Erweiterungen liegen in `Extensions` (oder `FP_EXTENSIONS_DIR`).
//! * Fenster: Anordnung und zusätzliche Fenster stehen in [`crate::session::UiState`]; die
//!   Oberfläche zeigt sie an.
//! * Hilfe: lokale Anleitung (`MANUAL.<sprache>.md`, z. B. `MANUAL.de.md`, sonst `MANUAL.en.md`/`MANUAL.md`,
//!   neben dem Programm bzw. im Ordner `docs`; die alten Namen `ANLEITUNG*.md` gelten weiter), sonst eine
//!   eingebaute Kurzhilfe. Firepixel sendet keine Nutzungsdaten.
//! * Durchsuchen: listet öffnbare Bilddateien eines Ordners (Ersatz für den externen Dateibrowser).
//! * Scannen: Windows-Bilderfassung (WIA) über PowerShell; das erfasste Bild wird ein neues Dokument.

use super::export::{open_with_system, temp_dir};
use crate::command::{CommandOutput, CommandRegistry, ParamKind, Params, cmd};
use crate::error::{EngineError, Result};
use crate::session::{Session, SessionEvent};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

fn launch_allowed(p: &Params) -> Result<bool> {
    Ok(p.bool_or("open", true)? && crate::shell::allowed_or_count())
}

fn bad(p: &Params, msg: impl Into<String>) -> EngineError {
    EngineError::InvalidParams {
        name: p.command.clone(),
        message: msg.into(),
    }
}

fn config_dir() -> PathBuf {
    crate::prefs::Preferences::default_path()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| std::env::temp_dir().join("Firepixel"))
}

/// Vom Prozess gesetzte Ordner (Tests): `std::env::set_var` ist in Edition 2024 `unsafe` und im
/// Workspace verboten, deshalb dieser Schalter wie [`crate::shell::disable`].
static DIR_OVERRIDES: std::sync::RwLock<(Option<PathBuf>, Option<PathBuf>)> =
    std::sync::RwLock::new((None, None));

/// Legt für diesen Prozess den Befehls- und den Erweiterungsordner fest (Tests: nie das echte
/// Nutzerprofil lesen oder beschreiben). Hat Vorrang vor `FP_COMMANDS_DIR`/`FP_EXTENSIONS_DIR`.
pub fn set_profile_dirs(commands: PathBuf, extensions: PathBuf) {
    if let Ok(mut g) = DIR_OVERRIDES.write() {
        *g = (Some(commands), Some(extensions));
    }
}

thread_local! {
    /// Befehlsordner nur für diesen Thread (Tests mit eigenem Ordner, z. B. das Befehle-Menü, das
    /// sonst parallel laufende Tests mit ihren gespeicherten Befehlen stören).
    static THREAD_COMMANDS_DIR: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

/// Legt den Befehlsordner nur für den aufrufenden Thread fest (`None` = wieder der des Prozesses).
/// Hat Vorrang vor [`set_profile_dirs`].
pub fn set_thread_commands_dir(dir: Option<PathBuf>) {
    THREAD_COMMANDS_DIR.with(|d| *d.borrow_mut() = dir);
}

/// Ordner der gespeicherten Befehle.
pub fn commands_dir() -> PathBuf {
    if let Some(d) = THREAD_COMMANDS_DIR.with(|d| d.borrow().clone()) {
        return d;
    }
    if let Some(d) = DIR_OVERRIDES.read().ok().and_then(|g| g.0.clone()) {
        return d;
    }
    std::env::var_os("FP_COMMANDS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| config_dir().join("Commands"))
}

/// Ordner der Erweiterungen.
pub fn extensions_dir() -> PathBuf {
    if let Some(d) = DIR_OVERRIDES.read().ok().and_then(|g| g.1.clone()) {
        return d;
    }
    std::env::var_os("FP_EXTENSIONS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| config_dir().join("Extensions"))
}

/// Gespeicherte Befehle (Name ohne `.jsf`, sortiert).
pub fn saved_commands() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(commands_dir())
        .map(|it| {
            it.filter_map(|e| e.ok())
                .filter_map(|e| {
                    let p = e.path();
                    (p.extension()
                        .and_then(|x| x.to_str())
                        .is_some_and(|x| x.eq_ignore_ascii_case("jsf")))
                    .then(|| p.file_stem().map(|s| s.to_string_lossy().to_string()))
                    .flatten()
                })
                .collect()
        })
        .unwrap_or_default();
    v.sort_by_key(|a| a.to_lowercase());
    v
}

/// Gespeicherte Befehle des Nutzers: wie [`saved_commands`], ohne die mitgelieferten Skripte des
/// Befehle-Menüs (Original: nur Befehle aus Verlauf ▸ „Als Befehl speichern“, PM-5 D25).
pub fn user_saved_commands(s: &Session) -> Vec<String> {
    let builtin = s
        .script_host()
        .map(|h| h.builtin_commands())
        .unwrap_or_default();
    saved_commands()
        .into_iter()
        .filter(|n| !builtin.iter().any(|b| b.eq_ignore_ascii_case(n)))
        .collect()
}

fn safe_file_name(n: &str) -> Result<String> {
    let t = n.trim();
    if t.is_empty()
        || t.contains(['\\', '/', ':', '*', '?', '"', '<', '>', '|'])
        || t == "."
        || t == ".."
    {
        return Err(EngineError::Failed(fp_i18n::tr!(
            "err-invalid-name",
            name = n
        )));
    }
    Ok(t.to_string())
}

/// Führt Skriptquelltext über die Skript-Laufzeit als einen Verlaufsschritt aus.
pub fn run_script_source(
    s: &mut Session,
    code: &str,
    path: Option<&Path>,
    label: &str,
) -> Result<Value> {
    let host = s
        .script_host()
        .ok_or(EngineError::BackendMissing("err-backend-script-runtime"))?;
    let mut out = Value::Null;
    if s.active().is_some() {
        s.run_group(label, |s| {
            out = host.run_script(s, code, path)?;
            Ok(())
        })?;
    } else {
        out = host.run_script(s, code, path)?;
    }
    Ok(out)
}

/// Wie [`run_script_source`]; mit `prompts` (aus der Oberfläche) beantworten diese der Reihe nach
/// die Fragen von `prompt()`. Fragt das Skript öfter, bleibt das Dokument unverändert und das
/// Ergebnis meldet die offene Frage: `{"promptNeeded": {"text", "default"}, "prompts": [...]}`.
fn run_script_prompting(
    s: &mut Session,
    p: &Params,
    code: &str,
    path: Option<&Path>,
    label: &str,
) -> Result<Value> {
    let answers: Option<Vec<Value>> = match p.value.get("prompts") {
        None | Some(Value::Null) => None,
        Some(Value::Array(a)) => Some(a.clone()),
        Some(_) => return Err(bad(p, "prompts")),
    };
    let Some(answers) = answers else {
        return run_script_source(s, code, path, label);
    };
    let texts: Vec<Option<String>> = answers
        .iter()
        .map(|v| match v {
            Value::Null => None,
            Value::String(t) => Some(t.clone()),
            other => Some(other.to_string()),
        })
        .collect();
    if !crate::script_prompt::begin(texts) {
        return run_script_source(s, code, path, label);
    }
    let r = run_script_source(s, code, path, label);
    match crate::script_prompt::end() {
        Some((text, default)) => {
            Ok(json!({"promptNeeded": {"text": text, "default": default}, "prompts": answers}))
        }
        None => r,
    }
}

/// Themen der Hilfe (Titel `msg-help-topic-<id>`, Text der Kurzhilfe `msg-help-body-<id>`).
const HELP_TOPICS: &[&str] = &[
    "start",
    "tools",
    "menus",
    "panels",
    "export",
    "automation",
    "support",
];

/// Kurzhilfe (Markdown) in der aktuellen Sprache, falls keine `MANUAL.md` gefunden wird.
fn builtin_help() -> String {
    let mut out = format!(
        "# {}\n\n{}\n",
        fp_i18n::t("msg-help-title"),
        fp_i18n::t("msg-help-intro")
    );
    for id in HELP_TOPICS {
        out.push_str(&format!(
            "\n## {}\n\n{}\n",
            fp_i18n::t(&format!("msg-help-topic-{id}")),
            fp_i18n::t(&format!("msg-help-body-{id}"))
        ));
    }
    out
}

/// Sucht die lokale Anleitung (`FP_HELP`, neben dem Programm, `docs` darüber); sonst die Kurzhilfe
/// im temporären Ordner.
pub fn help_file() -> PathBuf {
    if let Some(p) = std::env::var_os("FP_HELP")
        .map(PathBuf::from)
        .filter(|p| p.is_file())
    {
        return p;
    }
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        let mut d = exe.parent().map(Path::to_path_buf);
        for _ in 0..5 {
            let Some(x) = d else { break };
            dirs.push(x.clone());
            dirs.push(x.join("docs"));
            if cfg!(target_os = "macos") {
                // macOS-App-Paket: Anleitungen in `Firepixel.app/Contents/Resources`
                dirs.push(x.join("Resources"));
            }
            d = x.parent().map(Path::to_path_buf);
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd.join("docs"));
        dirs.push(cwd);
    }
    if let Some(f) = find_help(&dirs, &fp_i18n::current_language()) {
        return f;
    }
    let f = temp_dir("Help").join("QUICK_HELP.md");
    let _ = std::fs::write(&f, builtin_help());
    f
}

/// Grundname der Anleitung (`MANUAL.md`, `MANUAL.<sprache>.md`).
const MANUAL_STEM: &str = "MANUAL";
/// Früherer Grundname (vor TE-2); wird als Rückfall weiter gefunden.
const LEGACY_MANUAL_STEM: &str = "ANLEITUNG";

/// Dateinamen der Anleitung in Suchreihenfolge für eine Oberflächensprache: `MANUAL.<sprache>.md`
/// (auch für Deutsch: `MANUAL.de.md`), dann `MANUAL.en.md`, zuletzt `MANUAL.md` (im Repository die
/// englische Fassung, früher die deutsche). Zu jeder Sprachfassung folgt direkt der alte Name
/// (`ANLEITUNG.<sprache>.md`) als Rückfall.
fn help_names(lang: &str) -> Vec<String> {
    let base = lang
        .split(['-', '_'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    let mut suffixes: Vec<String> = Vec::new();
    if base == fp_i18n::SOURCE_LANG {
        suffixes.push(format!(".{base}"));
    } else if !base.is_empty() {
        if base != "en" {
            suffixes.push(format!(".{base}"));
        }
        suffixes.push(".en".to_string());
    }
    suffixes.push(String::new());
    suffixes
        .iter()
        .flat_map(|sfx| [MANUAL_STEM, LEGACY_MANUAL_STEM].map(|stem| format!("{stem}{sfx}.md")))
        .collect()
}

/// Erste Anleitung in `dirs` (je Ordner die Sprachfassung vor dem deutschen Rückfall).
fn find_help(dirs: &[PathBuf], lang: &str) -> Option<PathBuf> {
    let names = help_names(lang);
    dirs.iter()
        .find_map(|d| names.iter().map(|n| d.join(n)).find(|f| f.is_file()))
}

/// Abschnitte (`#`-Überschriften) einer Markdown-Datei.
fn headings(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| l.starts_with('#'))
        .map(|l| l.trim_start_matches('#').trim().to_string())
        .collect()
}

fn help_topic(s: &mut Session, p: &Params, default_topic: &str) -> Result<CommandOutput> {
    let f = help_file();
    let text = std::fs::read_to_string(&f).unwrap_or_else(|_| builtin_help());
    let topic = p
        .opt_str("topic")?
        .unwrap_or_else(|| default_topic.to_string());
    // Abschnitt über den Titel in der Sprache der Oberfläche oder (Anleitung auf Deutsch) im Deutschen
    let key = format!("msg-help-topic-{topic}");
    let titles: Vec<String> = if HELP_TOPICS.contains(&topic.as_str()) {
        vec![
            fp_i18n::t(&key),
            fp_i18n::with_language(fp_i18n::SOURCE_LANG, || fp_i18n::t(&key)),
        ]
    } else {
        vec![topic.clone()]
    };
    let heads = headings(&text);
    let section = heads
        .iter()
        .find(|h| {
            titles
                .iter()
                .any(|t| h.to_lowercase().contains(&t.to_lowercase()))
        })
        .cloned();
    if launch_allowed(p)? {
        open_with_system(&f, None)?;
    }
    s.emit(SessionEvent::PanelsChanged);
    Ok(CommandOutput::value(
        json!({"path": f.to_string_lossy(), "topic": topic, "section": section, "sections": heads}),
    ))
}

fn list_extensions() -> Vec<Value> {
    let mut v: Vec<Value> = std::fs::read_dir(extensions_dir())
        .map(|it| {
            it.filter_map(|e| e.ok())
                .map(|e| {
                    let p = e.path();
                    json!({"name": p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(), "folder": p.is_dir(), "path": p.to_string_lossy()})
                })
                .collect()
        })
        .unwrap_or_default();
    v.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    v
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)? {
        let e = e?;
        let p = e.path();
        let t = to.join(e.file_name());
        if p.is_dir() {
            copy_dir(&p, &t)?;
        } else {
            std::fs::copy(&p, &t)?;
        }
    }
    Ok(())
}

fn manage_extensions(p: &Params) -> Result<CommandOutput> {
    let dir = extensions_dir();
    let action = p.opt_str("action")?;
    if action.as_deref().is_some_and(|a| a != "list") {
        fp_i18n::profile::guard_write(&dir);
    }
    match action.as_deref().unwrap_or("list") {
        "list" => Ok(CommandOutput::value(
            json!({"folder": dir.to_string_lossy(), "extensions": list_extensions()}),
        )),
        "install" => {
            let src = PathBuf::from(p.str("path")?);
            let name = src
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .ok_or_else(|| bad(p, "path"))?;
            std::fs::create_dir_all(&dir)?;
            if src.is_dir() {
                copy_dir(&src, &dir.join(&name))?;
            } else {
                std::fs::copy(&src, dir.join(&name))?;
            }
            Ok(CommandOutput::session_with(
                json!({"installed": name, "extensions": list_extensions()}),
            ))
        }
        "remove" => {
            let name = safe_file_name(&p.str("name")?)?;
            let t = dir.join(&name);
            if t.is_dir() {
                std::fs::remove_dir_all(&t)?;
            } else if t.is_file() {
                std::fs::remove_file(&t)?;
            } else {
                return Err(EngineError::NotFound(fp_i18n::tr!(
                    "err-nf-extension",
                    name = name.as_str()
                )));
            }
            Ok(CommandOutput::session_with(
                json!({"removed": name, "extensions": list_extensions()}),
            ))
        }
        _ => Err(bad(p, fp_i18n::t("err-param-action-ext"))),
    }
}

const OPENABLE: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "bmp", "tif", "tiff", "tga", "psd", "webp", "jxl", "qoi", "ico",
    "svg", "svgz", "wbmp", "ai", "eps", "avif", "heic",
];

/// Name eines Katalogbefehls zur Befehls-ID (die Namensgebung im Katalog bleibt maßgeblich).
fn catalog_name(fw_id: u32) -> Option<String> {
    crate::catalog::entries()
        .iter()
        .find(|e| e.fw_id == Some(fw_id))
        .map(|e| e.name.clone())
}

pub(super) fn register(r: &mut CommandRegistry) {
    // ---------------------------------------------------------------- Befehle
    r.add(
        cmd("commands.runScript")
            .opt("path", ParamKind::Path, "Skriptdatei (.jsf)")
            .opt("code", ParamKind::String, "Skriptquelltext (statt path)")
            .opt(
                "prompts",
                ParamKind::Json,
                "Antworten der Oberfläche für prompt() (Liste aus Text/null); ohne: Vorgabewerte",
            )
            .run(|s, p| {
                let (code, path) = match (p.opt_str("code")?, p.opt_str("path")?) {
                    (Some(c), _) => (c, None),
                    (None, Some(f)) => (std::fs::read_to_string(&f)?, Some(PathBuf::from(f))),
                    _ => return Err(bad(p, fp_i18n::t("err-param-path-or-code"))),
                };
                let label = path
                    .as_ref()
                    .and_then(|f| f.file_stem())
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Skript".into());
                let v = run_script_prompting(s, &p, &code, path.as_deref(), &label)?;
                Ok(CommandOutput::session_with(json!({"result": v})))
            }),
    );
    r.add(
        cmd("commands.run")
            .label("Gespeicherten Befehl ausführen")
            .param("name", ParamKind::String, "Name des gespeicherten Befehls")
            .opt(
                "prompts",
                ParamKind::Json,
                "Antworten der Oberfläche für prompt() (Liste aus Text/null); ohne: Vorgabewerte",
            )
            .run(|s, p| {
                let name = safe_file_name(&p.str("name")?)?;
                let f = commands_dir().join(format!("{name}.jsf"));
                let code = std::fs::read_to_string(&f).map_err(|_| {
                    EngineError::NotFound(fp_i18n::tr!(
                        "err-nf-saved-command",
                        name = name.as_str()
                    ))
                })?;
                let v = run_script_prompting(s, &p, &code, Some(&f), &name)?;
                Ok(CommandOutput::session_with(json!({"result": v})))
            }),
    );
    r.add(
        cmd("commands.manageSavedCommands")
            .opt_default("action", ParamKind::Enum { values: vec!["list".into(), "save".into(), "rename".into(), "delete".into()] }, json!("list"), "Aktion")
            .opt("name", ParamKind::String, "Befehlsname")
            .opt("newName", ParamKind::String, "neuer Name (rename)")
            .opt("code", ParamKind::String, "Skriptquelltext (save); ohne: Verlauf des Dokuments als Skript")
            .run(|s, p| {
                let dir = commands_dir();
                let action = p.opt_str("action")?;
                if action.as_deref().is_some_and(|a| a != "list") {
                    fp_i18n::profile::guard_write(&dir);
                }
                match action.as_deref().unwrap_or("list") {
                    "list" => return Ok(CommandOutput::value(json!({"folder": dir.to_string_lossy(), "commands": user_saved_commands(s)}))),
                    "save" => {
                        let name = safe_file_name(&p.str("name")?)?;
                        let code = match p.opt_str("code")? {
                            Some(c) => c,
                            None => crate::history::to_script(&s.doc()?.history.export_all()),
                        };
                        std::fs::create_dir_all(&dir)?;
                        std::fs::write(dir.join(format!("{name}.jsf")), code)?;
                    }
                    "rename" => {
                        let (a, b) = (safe_file_name(&p.str("name")?)?, safe_file_name(&p.str("newName")?)?);
                        std::fs::rename(dir.join(format!("{a}.jsf")), dir.join(format!("{b}.jsf"))).map_err(|_| EngineError::NotFound(fp_i18n::tr!("err-nf-saved-command", name = a.as_str())))?;
                    }
                    "delete" => {
                        let a = safe_file_name(&p.str("name")?)?;
                        std::fs::remove_file(dir.join(format!("{a}.jsf"))).map_err(|_| EngineError::NotFound(fp_i18n::tr!("err-nf-saved-command", name = a.as_str())))?;
                    }
                    _ => return Err(bad(p, "action")),
                }
                s.emit(SessionEvent::PanelsChanged);
                Ok(CommandOutput::session_with(json!({"commands": user_saved_commands(s)})))
            }),
    );
    for id in [35590u32, 35637] {
        if let Some(name) = catalog_name(id) {
            r.add(
                cmd(&name)
                    .opt_default(
                        "action",
                        ParamKind::Enum {
                            values: vec!["list".into(), "install".into(), "remove".into()],
                        },
                        json!("list"),
                        "Aktion",
                    )
                    .opt(
                        "path",
                        ParamKind::Path,
                        "Datei/Ordner der Erweiterung (install)",
                    )
                    .opt("name", ParamKind::String, "Name (remove)")
                    .run(|_, p| manage_extensions(p)),
            );
        }
    }
    if let Some(name) = catalog_name(7013) {
        r.add(
            cmd(&name)
                .opt_default(
                    "open",
                    ParamKind::Bool,
                    json!(true),
                    "Ordner im Explorer öffnen",
                )
                .run(|_, p| {
                    let dir = extensions_dir();
                    fp_i18n::profile::guard_write(&dir);
                    std::fs::create_dir_all(&dir)?;
                    if launch_allowed(p)? {
                        open_with_system(&dir, None)?;
                    }
                    Ok(CommandOutput::value(
                        json!({"folder": dir.to_string_lossy(), "extensions": list_extensions()}),
                    ))
                }),
        );
    }
    r.add(cmd("window.extensions").run(|_, _| {
        Ok(CommandOutput::value(
            json!({"folder": extensions_dir().to_string_lossy(), "extensions": list_extensions()}),
        ))
    }));

    // ---------------------------------------------------------------- Befehle, die nur in Tastenkürzel-Sets stehen
    r.add(
        cmd("file.saveCopyAs")
            .label("Kopie speichern unter")
            .fw_cmd("cmd_SaveCopyAs")
            .requires(super::DOC)
            .param("path", ParamKind::Path, "Zieldatei")
            .run(|s, p| {
                let path = PathBuf::from(p.str("path")?);
                let d = s.doc()?;
                let (doc, page) = (std::sync::Arc::clone(&d.doc), d.page);
                super::file::save_document(s, &doc, page, &path)?;
                Ok(CommandOutput::value(
                    json!({"path": path.to_string_lossy()}),
                ))
            }),
    );
    for (name, fwc, dx, dy) in [
        ("edit.cloneNudge.up", "cmd_CloneNudgeUp", 0.0, -1.0),
        ("edit.cloneNudge.down", "cmd_CloneNudgeDown", 0.0, 1.0),
        ("edit.cloneNudge.left", "cmd_CloneNudgeLeft", -1.0, 0.0),
        ("edit.cloneNudge.right", "cmd_CloneNudgeRight", 1.0, 0.0),
        ("edit.cloneNudge.bigUp", "cmd_CloneBigNudgeUp", 0.0, -10.0),
        (
            "edit.cloneNudge.bigDown",
            "cmd_CloneBigNudgeDown",
            0.0,
            10.0,
        ),
        (
            "edit.cloneNudge.bigLeft",
            "cmd_CloneBigNudgeLeft",
            -10.0,
            0.0,
        ),
        (
            "edit.cloneNudge.bigRight",
            "cmd_CloneBigNudgeRight",
            10.0,
            0.0,
        ),
    ] {
        r.add(
            cmd(name)
                .label("Kopie verschieben")
                .fw_cmd(fwc)
                .requires(super::SEL)
                .run(move |s, _| super::edit::move_selection(s, dx, dy, true)),
        );
    }
    r.add(
        cmd("edit.fillClear")
            .label("Löschen")
            .fw_cmd("cmd_FillClear")
            .requires(&[
                crate::command::Requirement::Document,
                crate::command::Requirement::SelectionOrPixels,
            ])
            .run(|s, p| {
                let c = s
                    .registry()
                    .get("edit.clear")
                    .cloned()
                    .ok_or_else(|| EngineError::UnknownCommand("edit.clear".into()))?;
                c.apply(s, p)
            }),
    );
    r.add(
        cmd("window.answers")
            .label("Antworten")
            .fw_cmd("cmd_Panels_Answers")
            .opt_default("open", ParamKind::Bool, json!(true), "Anleitung öffnen")
            .run(|s, p| help_topic(s, p, "start")),
    );
    r.add(
        cmd("view.togglePlayback")
            .label("Abspielen/Stopp")
            .fw_cmd("cmd_VCR_TogglePlayOrStop")
            .requires(super::DOC)
            .checked(|s| s.ui.playing)
            .run(|s, _| {
                s.ui.playing = !s.ui.playing;
                s.emit(SessionEvent::PanelsChanged);
                Ok(CommandOutput::session_with(
                    json!({"playing": s.ui.playing}),
                ))
            }),
    );

    // ---------------------------------------------------------------- Fenster
    for (name, layout) in [("window.cascade", "cascade"), ("window.tile", "tile")] {
        r.add(
            cmd(name)
                .requires(super::DOC)
                .checked(move |s| s.ui.window_layout == layout)
                .run(move |s, _| {
                    s.ui.window_layout = layout.to_string();
                    s.emit(SessionEvent::PanelsChanged);
                    Ok(CommandOutput::session_with(json!({"layout": layout})))
                }),
        );
    }
    r.add(
        cmd("window.tabs")
            .label("Als Registerkarten")
            .requires(super::DOC)
            .checked(|s| s.ui.window_layout.is_empty() || s.ui.window_layout == "tabs")
            .run(|s, _| {
                s.ui.window_layout = "tabs".into();
                s.emit(SessionEvent::PanelsChanged);
                Ok(CommandOutput::session_with(json!({"layout": "tabs"})))
            }),
    );
    r.add(cmd("window.duplicateWindow").requires(super::DOC).run(|s, _| {
        let id = s.doc()?.id;
        s.ui.extra_windows.push(id);
        s.emit(SessionEvent::PanelsChanged);
        Ok(CommandOutput::session_with(json!({"doc": id.0, "windows": s.ui.extra_windows.iter().filter(|d| **d == id).count() + 1})))
    }));
    r.add(
        cmd("window.closeDuplicate")
            .label("Zusätzliches Fenster schließen")
            .requires(super::DOC)
            .run(|s, _| {
                let id = s.doc()?.id;
                if let Some(i) = s.ui.extra_windows.iter().rposition(|d| *d == id) {
                    s.ui.extra_windows.remove(i);
                }
                s.emit(SessionEvent::PanelsChanged);
                Ok(CommandOutput::session())
            }),
    );

    // ---------------------------------------------------------------- Hilfe
    for (id, topic) in [(57667u32, "start"), (35323, "support"), (35580, "support")] {
        if let Some(name) = catalog_name(id) {
            r.add(
                cmd(&name)
                    .opt(
                        "topic",
                        ParamKind::Enum {
                            values: HELP_TOPICS.iter().map(|k| k.to_string()).collect(),
                        },
                        "Thema",
                    )
                    .opt_default("open", ParamKind::Bool, json!(true), "Anleitung öffnen")
                    .run(move |s, p| help_topic(s, p, topic)),
            );
        }
    }
    if let Some(name) = catalog_name(7028) {
        r.add(cmd(&name).opt("participate", ParamKind::Bool, "Teilnahme (Firepixel sendet nie Daten; der Wert wird nur gespeichert)").run(|s, p| {
            if let Some(b) = p.opt_bool("participate")? {
                s.prefs.set("ProductImprovementOptIn", crate::prefs::PrefValue::Bool(b));
                s.prefs_changed();
            }
            let v = s.prefs.get("ProductImprovementOptIn").map(|v| v.to_json()).unwrap_or(json!(false));
            Ok(CommandOutput::session_with(json!({"participate": v, "dataSent": false, "text": fp_i18n::t("msg-no-usage-data")})))
        }));
    }

    // ---------------------------------------------------------------- Durchsuchen / Scannen
    r.add(
        cmd("file.browseInBridge")
            .opt("folder", ParamKind::Path, "Ordner (Vorgabe: Ordner des aktiven bzw. zuletzt geöffneten Dokuments)")
            .opt_default("open", ParamKind::Bool, json!(false), "Ordner im Explorer öffnen")
            .run(|s, p| {
                let folder = match p.opt_str("folder")? {
                    Some(f) => PathBuf::from(f),
                    None => s
                        .active()
                        .and_then(|d| d.path.as_ref())
                        .or(s.prefs.recent_files.first())
                        .and_then(|p| p.parent().map(Path::to_path_buf))
                        .or_else(|| std::env::current_dir().ok())
                        .unwrap_or_default(),
                };
                let mut files: Vec<Value> = std::fs::read_dir(&folder)
                    .map_err(|e| EngineError::Io(format!("{}: {e}", folder.display())))?
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().and_then(|x| x.to_str()).is_some_and(|x| OPENABLE.contains(&x.to_ascii_lowercase().as_str())))
                    .map(|e| {
                        let m = e.metadata().ok();
                        let modified = m.as_ref().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs());
                        json!({"name": e.file_name().to_string_lossy(), "path": e.path().to_string_lossy(), "bytes": m.map(|m| m.len()), "modified": modified})
                    })
                    .collect();
                files.sort_by(|a, b| a["name"].as_str().map(str::to_lowercase).cmp(&b["name"].as_str().map(str::to_lowercase)));
                if launch_allowed(p)? {
                    open_with_system(&folder, None)?;
                }
                Ok(CommandOutput::value(json!({"folder": folder.to_string_lossy(), "files": files})))
            }),
    );
    r.add(cmd("file.twainSelect").opt("device", ParamKind::String, "Geräte-ID (ohne: Auswahldialog von Windows)").run(|s, p| {
        let id = match p.opt_str("device")? {
            Some(d) => d,
            None => wia("$d = New-Object -ComObject WIA.CommonDialog; $dev = $d.ShowSelectDevice(1, $true, $false); if ($dev) { $dev.DeviceID }")?.trim().to_string(),
        };
        if id.is_empty() {
            return Err(EngineError::Failed(fp_i18n::t("err-no-scanner")));
        }
        s.prefs.set("ScannerDeviceId", crate::prefs::PrefValue::Str(id.clone()));
        s.prefs_changed();
        Ok(CommandOutput::session_with(json!({"device": id})))
    }));
    r.add(
        cmd("file.twainAcquire")
            .opt("file", ParamKind::Path, "bereits erfasstes Bild (PNG) statt des Scannerdialogs")
            .run(|s, p| {
                let file = match p.opt_str("file")? {
                    Some(f) => PathBuf::from(f),
                    None => {
                        let out = temp_dir("Scan").join("scan.png");
                        let _ = std::fs::remove_file(&out);
                        let target = out.to_string_lossy().replace('\'', "''");
                        // FormatID PNG; Gerät aus „Scanner auswählen“ wird vom Windows-Dialog vorgeschlagen
                        wia(&format!("$d = New-Object -ComObject WIA.CommonDialog; $i = $d.ShowAcquireImage(1, 1, 131072, '{{B96B3CAF-0728-11D3-9D7B-0000F81EF32E}}', $false, $true, $false); if ($i) {{ $i.SaveFile('{target}') }}"))?;
                        if !out.is_file() {
                            return Err(EngineError::Failed(fp_i18n::t("err-scan-cancelled")));
                        }
                        out
                    }
                };
                let doc = super::file::load_document(&file)?;
                let id = s.add_document(doc, None);
                Ok(CommandOutput::session_with(json!({"doc": id.0, "source": file.to_string_lossy()})))
            }),
    );
}

/// Führt ein PowerShell-Stück für die Windows-Bilderfassung aus und liefert die Ausgabe.
fn wia(script: &str) -> Result<String> {
    #[cfg(windows)]
    {
        let mut c = std::process::Command::new("powershell.exe");
        c.args(["-NoProfile", "-NonInteractive", "-STA", "-Command", script]);
        let out = crate::shell::run(c)?
            .ok_or_else(|| EngineError::Failed(fp_i18n::t("err-scan-shell-disabled")))?;
        if !out.status.success() {
            return Err(EngineError::Failed(fp_i18n::tr!(
                "err-scan-unavailable",
                detail = String::from_utf8_lossy(&out.stderr).trim().to_string()
            )));
        }
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    }
    #[cfg(not(windows))]
    {
        let _ = script;
        Err(EngineError::Failed(fp_i18n::t("err-scan-windows-only")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// UE-5: Hilfe-Menü wählt die Anleitung in der Oberflächensprache (Rückfall Deutsch), ohne etwas zu öffnen.
    #[test]
    fn manual_per_language() {
        let root = std::env::temp_dir().join(format!("fp-ue5-hilfe-{}", std::process::id()));
        let (a, b) = (root.join("a"), root.join("b"));
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        std::fs::write(a.join("MANUAL.md"), "# Anleitung").unwrap();
        std::fs::write(a.join("MANUAL.en.md"), "# Manual").unwrap();
        std::fs::write(b.join("MANUAL.md"), "# Anleitung").unwrap();
        let dirs = vec![a.clone(), b.clone()];
        assert_eq!(find_help(&dirs, "de"), Some(a.join("MANUAL.md")));
        assert_eq!(find_help(&dirs, "en"), Some(a.join("MANUAL.en.md")));
        assert_eq!(find_help(&dirs, "en-US"), Some(a.join("MANUAL.en.md")));
        assert_eq!(find_help(&dirs, "fr"), Some(a.join("MANUAL.en.md")));
        assert_eq!(find_help(&dirs, "qps"), Some(a.join("MANUAL.en.md")));
        // nur deutsche Fassung vorhanden → Rückfall Deutsch
        assert_eq!(find_help(&dirs[1..], "en"), Some(b.join("MANUAL.md")));
        // eigene Sprachfassung vor Englisch
        std::fs::write(a.join("MANUAL.fr.md"), "# Manuel").unwrap();
        assert_eq!(find_help(&dirs, "fr"), Some(a.join("MANUAL.fr.md")));
        assert_eq!(find_help(&[root.join("fehlt")], "en"), None);
        // deutsche Fassung unter eigenem Namen (`MANUAL.de.md`, `MANUAL.md` dann englisch)
        std::fs::write(a.join("MANUAL.de.md"), "# Anleitung").unwrap();
        assert_eq!(find_help(&dirs, "de"), Some(a.join("MANUAL.de.md")));
        assert_eq!(find_help(&dirs, "de-AT"), Some(a.join("MANUAL.de.md")));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// TE2-E: Die alten Handbuchnamen `ANLEITUNG*.md` werden weiter gefunden; der neue Name gewinnt
    /// bei gleicher Sprache, die Sprachfassung geht aber vor dem Namen.
    #[test]
    fn manual_legacy_names() {
        let root = std::env::temp_dir().join(format!("fp-te2e-manual-{}", std::process::id()));
        let (old, mixed) = (root.join("old"), root.join("mixed"));
        std::fs::create_dir_all(&old).unwrap();
        std::fs::create_dir_all(&mixed).unwrap();
        std::fs::write(old.join("ANLEITUNG.md"), "# Anleitung").unwrap();
        std::fs::write(old.join("ANLEITUNG.en.md"), "# Manual").unwrap();
        assert_eq!(
            find_help(std::slice::from_ref(&old), "de"),
            Some(old.join("ANLEITUNG.md"))
        );
        assert_eq!(
            find_help(std::slice::from_ref(&old), "en"),
            Some(old.join("ANLEITUNG.en.md"))
        );
        assert_eq!(
            find_help(std::slice::from_ref(&old), "fr"),
            Some(old.join("ANLEITUNG.en.md"))
        );
        // alter englischer Name vor neuem deutschem
        std::fs::write(mixed.join("MANUAL.md"), "# Anleitung").unwrap();
        std::fs::write(mixed.join("ANLEITUNG.en.md"), "# Manual").unwrap();
        assert_eq!(
            find_help(std::slice::from_ref(&mixed), "en"),
            Some(mixed.join("ANLEITUNG.en.md"))
        );
        assert_eq!(
            find_help(std::slice::from_ref(&mixed), "de"),
            Some(mixed.join("MANUAL.md"))
        );
        // neuer Name vor altem bei gleicher Sprache
        std::fs::write(mixed.join("MANUAL.en.md"), "# Manual").unwrap();
        std::fs::write(mixed.join("ANLEITUNG.md"), "# Anleitung").unwrap();
        assert_eq!(
            find_help(std::slice::from_ref(&mixed), "en"),
            Some(mixed.join("MANUAL.en.md"))
        );
        assert_eq!(
            find_help(std::slice::from_ref(&mixed), "de"),
            Some(mixed.join("MANUAL.md"))
        );
        assert_eq!(
            help_names("de"),
            vec![
                "MANUAL.de.md",
                "ANLEITUNG.de.md",
                "MANUAL.md",
                "ANLEITUNG.md"
            ]
        );
        assert_eq!(
            help_names("en"),
            vec![
                "MANUAL.en.md",
                "ANLEITUNG.en.md",
                "MANUAL.md",
                "ANLEITUNG.md"
            ]
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Über den Befehl: Pfad der englischen Anleitung bei englischer Oberfläche (FP_HELP nicht gesetzt,
    /// `open: false`, also kein Fremdprogramm).
    #[test]
    fn help_command_returns_english_manual() {
        if std::env::var_os("FP_HELP").is_some() {
            return;
        }
        let f = fp_i18n::with_language("en", help_file);
        let name = f
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        // im Repository liegen beide Fassungen unter docs\ (über current_dir/current_exe gefunden)
        // (neuer Name `MANUAL*.md` oder bis zur Umbenennung der Dokumente der alte `ANLEITUNG*.md`)
        if let Some(d) = f.parent() {
            if d.join("MANUAL.en.md").is_file() {
                assert_eq!(name, "MANUAL.en.md", "{f:?}");
            } else if d.join("ANLEITUNG.en.md").is_file() {
                assert_eq!(name, "ANLEITUNG.en.md", "{f:?}");
            }
        }
        let de = fp_i18n::with_language("de", help_file);
        let de_name = de
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        if let Some(d) = de.parent() {
            if d.join("MANUAL.de.md").is_file() {
                assert_eq!(de_name, "MANUAL.de.md", "{de:?}");
            } else if d.join("MANUAL.md").is_file() {
                assert_eq!(de_name, "MANUAL.md", "{de:?}");
            } else if d.join("ANLEITUNG.md").is_file() {
                assert_eq!(de_name, "ANLEITUNG.md", "{de:?}");
            }
        }
    }
}

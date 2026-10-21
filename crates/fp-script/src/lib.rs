//! JSF-Kompatibilität von Firepixel: JSF-JavaScript (`.jsf`) über boa_engine.
//!
//! Globale Objekte wie im Original (03 §2): `fw` (= `firepixel`, `app`; Klasse `Firepixel`, bewusste
//! Abweichung: Programmname in Klassen-, Global- und Methodennamen durch „Firepixel“ ersetzt, z. B.
//! `createFirepixelDocument`), Document-DOM
//! (`fw.getDocumentDOM()`, `fw.documents`), `Files`/FileReference, `Errors`, `Tools`, `System`,
//! `$`, `FwArray`, `alert`/`confirm`/`prompt`/`write`. Klassen, Methoden, Parameter, Vorgaben und
//! read-only-Angaben stammen aus der Bindungstabelle (565 Methoden, 721 Properties,
//! [`bindings`]). Aufrufsemantik nach 03 §10.3/§10.11: fehlende Argumente aus Vorgaben, sonst
//! `EBadParamCount`; überzählige werden ignoriert; lockere String↔Zahl-Konvertierung; boolean aus
//! Strings (`"true"` → true, sonst false); Fehler als HRESULT-**Zahl**; Zuweisung an read-only wird
//! still ignoriert; `getDocumentDOM()` liefert je Aufruf ein neues Wrapper-Objekt; customData-Skalare
//! werden nach der Regel aus 10.12 typisiert; Listen aus customData sind FwArray.
//!
//! Die Methoden bilden auf das Befehlsregister von fp-engine ab (dieselben Befehle wie Menüs, CLI
//! und MCP). Nicht umgesetzte Member werfen `Error("nicht implementiert: Klasse.member")`; die Liste
//! der umgesetzten Member liefert [`coverage`].
//!
//! # Beispiel
//!
//! ```
//! let mut s = fp_engine::Session::default();
//! let out = fp_script::run_source(&mut s, r##"
//!     var d = fw.createFirepixelDocument({x: 100, y: 80}, {pixelsPerUnit: 72, units: "inch"}, "#ffffff");
//!     d = fw.getDocumentDOM();
//!     d.addNewRectangle({left: 10, top: 10, right: 50, bottom: 40}, 0);
//!     d.setFillColor("#ff0000");
//!     fw.selection.length + " " + fw.selection[0];
//! "##, &fp_script::ScriptOptions::default());
//! assert!(out.ok, "{:?}", out.error);
//! assert_eq!(out.result.as_deref(), Some("1 [object RectanglePrimitive]"));
//! ```

pub mod autoshape;
pub mod bindings;
pub mod builtin_commands;
#[rustfmt::skip]
mod bindings_data;
pub mod conv;
pub mod coverage;
pub mod fileio;
pub mod imp;
pub mod runtime;
pub mod state;
pub mod urls;

pub use builtin_commands::install_builtin_commands;
pub use coverage::{coverage, coverage_markdown};

use boa_engine::Source;
use fp_engine::{Preferences, Session};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Einstellungen eines Skriptlaufs.
#[derive(Clone, Debug)]
pub struct ScriptOptions {
    /// Pfad der Skriptdatei (für `fw.currentScriptDir`/`currentScriptFileName`).
    pub script_path: Option<PathBuf>,
    /// Ganzes Skript als **ein** Rückgängig-Schritt (wie „Befehl ausführen“ im Original, Laufzeit
    /// 10 T6). Dann wirken `fw.undo()`/`historyPalette` innerhalb des Skripts nicht auf eigene Schritte.
    pub single_undo_step: bool,
    /// Code, der vor dem Skript läuft (z. B. Prolog der Testumgebung).
    pub prelude: Option<String>,
    /// Code, der nach dem Skript läuft (auch nach einem Fehler, im selben Kontext).
    pub epilogue: Option<String>,
    /// Globale Variablen (Name, JSON-Wert).
    pub globals: Vec<(String, Value)>,
    /// Lese-Umleitungen: Existiert eine Datei unter dem ersten Präfix nicht, wird unter dem zweiten
    /// gelesen (Testumgebung mit Eingabedateien an anderem Ort).
    pub read_fallback: Vec<(PathBuf, PathBuf)>,
    /// Obergrenze für Schleifendurchläufe (Schutz vor Endlosschleifen).
    pub loop_limit: u64,
    /// Antwort von `confirm()`/`fw.yesNoDialog()` ohne Oberfläche.
    pub confirm_answer: bool,
    /// Sprachordner für `Files.getLanguageDirectory()` (enthält `JSFStrings`); Vorgabe im Benutzerprofil.
    pub language_dir: Option<PathBuf>,
}

impl Default for ScriptOptions {
    fn default() -> Self {
        ScriptOptions {
            script_path: None,
            single_undo_step: false,
            prelude: None,
            epilogue: None,
            globals: Vec::new(),
            read_fallback: Vec::new(),
            loop_limit: 200_000_000,
            confirm_answer: true,
            language_dir: None,
        }
    }
}

/// Ergebnis eines Skriptlaufs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScriptOutcome {
    /// Ohne nicht abgefangene Ausnahme beendet.
    pub ok: bool,
    /// Wert des letzten Ausdrucks (als Text).
    pub result: Option<String>,
    /// Nicht abgefangene Ausnahme (Text bzw. Zahl).
    pub error: Option<String>,
    /// Geworfene Zahl (HRESULT), falls die Ausnahme eine Zahl war.
    pub thrown: Option<f64>,
    /// Texte aus `alert()`.
    pub alerts: Vec<String>,
    /// Texts from `fp.saveAs(text, fileName)` that the host offers to save (file name, text).
    pub save_as: Vec<(String, String)>,
    /// Ausgaben aus `write()`/`writeLog()`.
    pub log: Vec<String>,
    /// Erklärungen zu Fehlern der API-Aufrufe (auch abgefangene).
    pub diagnostics: Vec<String>,
    /// Aufgerufene, aber nicht umgesetzte Member.
    pub not_implemented: Vec<String>,
    /// Benutzte API-Member.
    pub used: Vec<String>,
}

impl ScriptOutcome {
    /// Als JSON (MCP, CLI).
    pub fn to_json(&self) -> Value {
        json!({
            "ok": self.ok, "result": self.result, "error": self.error, "thrown": self.thrown,
            "alerts": self.alerts, "log": self.log, "diagnostics": self.diagnostics,
            "notImplemented": self.not_implemented,
        })
    }

    fn failed(msg: impl Into<String>) -> ScriptOutcome {
        ScriptOutcome {
            ok: false,
            error: Some(msg.into()),
            ..ScriptOutcome::default()
        }
    }
}

/// Führt JSF-Quelltext auf der Sitzung aus.
pub fn run_source(session: &mut Session, source: &str, opts: &ScriptOptions) -> ScriptOutcome {
    if state::is_installed() {
        return ScriptOutcome::failed("es läuft bereits ein Skript auf diesem Thread");
    }
    // Q-58 (RE spec 58 section 2.2): the tool attributes follow the selection before the script
    // (idle) and after it, never inside it
    session.follow_selection();
    session.hold_tool_follow();
    let outcome = run_source_held(session, source, opts);
    session.release_tool_follow();
    session.follow_selection();
    outcome
}

fn run_source_held(session: &mut Session, source: &str, opts: &ScriptOptions) -> ScriptOutcome {
    if opts.single_undo_step && session.active().is_some() {
        let mut outcome = ScriptOutcome::default();
        let label = opts
            .script_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Skript".into());
        let r = session.run_group(&label, |s| {
            outcome = run_inner(s, source, opts);
            Ok(())
        });
        if let Err(e) = r {
            outcome.diagnostics.push(e.to_string());
        }
        return outcome;
    }
    run_inner(session, source, opts)
}

/// Skript-Laufzeit für fp-engine („Befehle ▸ Skript ausführen“, gespeicherte Befehle,
/// `commands.runScript`/`commands.run` aus CLI, MCP und Oberfläche).
#[derive(Clone, Copy, Debug, Default)]
pub struct JsfHost;

impl fp_engine::ScriptHost for JsfHost {
    fn run_script(
        &self,
        session: &mut Session,
        code: &str,
        path: Option<&Path>,
    ) -> fp_engine::Result<Value> {
        let opts = ScriptOptions {
            script_path: path.map(Path::to_path_buf),
            ..ScriptOptions::default()
        };
        let o = run_source(session, code, &opts);
        let unavailable = state::take_last_unavailable();
        if o.ok {
            Ok(
                json!({"result": o.result, "alerts": o.alerts, "log": o.log, "notImplemented": o.not_implemented,
                    "saveAs": o.save_as.iter().map(|(name, text)| json!({"name": name, "text": text})).collect::<Vec<_>>()}),
            )
        } else {
            // CT-A-03/04: abgebrochen, weil ein Befehl im Skript nicht passte (keine passende Auswahl,
            // kein Dokument): als „nicht verfügbar“ mit Begründung bzw. „kein Dokument“ melden
            let thrown = o.thrown.map(|t| t as i64 as i32 as u32);
            if thrown == Some(bindings::hr::E_BAD_SELECTION)
                && let Some(reason) = unavailable
            {
                return Err(fp_engine::EngineError::Unavailable {
                    name: "commands.runScript".into(),
                    reason,
                });
            }
            if thrown == Some(bindings::hr::E_NO_ACTIVE_DOCUMENT) {
                return Err(fp_engine::EngineError::NoDocument);
            }
            let mut msg = o.error.unwrap_or_else(|| fp_i18n::t("err-script-failed"));
            if let Some(d) = o.diagnostics.last() {
                msg = format!("{msg} ({d})");
            }
            Err(fp_engine::EngineError::Failed(fp_i18n::tr!(
                "err-script",
                detail = msg
            )))
        }
    }

    fn auto_shape(&self, session: &mut Session, op: &Value) -> fp_engine::Result<Value> {
        // läuft innerhalb von Session::execute (Befehle autoShape.*): Verlauf macht der Befehl
        autoshape::op(session, op)
    }

    fn builtin_commands(&self) -> Vec<String> {
        // nur die oberste Ebene: Unterordner sind Untermenüs und werden nie als gespeichert gelistet
        builtin_commands::BUILTIN
            .iter()
            .map(|(rel, _)| *rel)
            .chain(builtin_commands::BUILTIN_EN.iter().copied())
            .chain(builtin_commands::RENAMED_DE.iter().map(|(_, old)| *old))
            .filter(|rel| !rel.contains('/'))
            .map(|rel| rel.trim_end_matches(".jsf").to_string())
            .collect()
    }
}

/// Steckt die JSF-Laufzeit in eine Sitzung (falls noch keine eingesteckt ist).
pub fn install_host(session: &mut Session) {
    if session.script_host().is_none() {
        session.set_script_host(Some(Arc::new(JsfHost)));
    }
}

/// Darf ein Skript etwas extern öffnen (Browser, E-Mail, Explorer)? Nicht, wenn `FP_NO_SHELL_OPEN`
/// gesetzt ist (Tests, Automatisierung).
pub fn shell_open_allowed() -> bool {
    std::env::var_os("FP_NO_SHELL_OPEN").is_none()
}

/// Führt eine `.jsf`-Datei aus (Kodierung UTF-8, sonst Latin-1; CR-Zeilenenden erlaubt).
pub fn run_file(session: &mut Session, path: &Path, opts: &ScriptOptions) -> ScriptOutcome {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => return ScriptOutcome::failed(format!("{}: {e}", path.display())),
    };
    let src = decode_text(&bytes);
    let mut o = opts.clone();
    if o.script_path.is_none() {
        o.script_path = Some(path.to_path_buf());
    }
    run_source(session, &src, &o)
}

/// Text aus Bytes: UTF-8 (mit/ohne BOM), sonst Latin-1.
pub fn decode_text(bytes: &[u8]) -> String {
    let b = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    match std::str::from_utf8(b) {
        Ok(s) => s.to_string(),
        Err(_) => b.iter().map(|&c| c as char).collect(),
    }
}

fn run_inner(session: &mut Session, source: &str, opts: &ScriptOptions) -> ScriptOutcome {
    let placeholder = Session::with_registry(Preferences::new(), Arc::clone(session.registry()));
    let real = std::mem::replace(session, placeholder);
    let mut st = state::State::new(real, opts.clone());
    // TW-139: reads see the open text editor's live state (ended again before handing back)
    st.session.begin_live_text_view();
    if let Some(p) = &opts.script_path {
        st.script_stack.push(p.clone());
    }
    state::install(Some(st));
    let _ = state::take_last_unavailable();
    let mut outcome = ScriptOutcome::default();
    match runtime::new_context(opts.loop_limit) {
        Ok(mut ctx) => {
            for (k, v) in &opts.globals {
                if let Ok(jv) = conv::json_to_js(v, &mut ctx) {
                    let _ = ctx.register_global_property(
                        boa_engine::JsString::from(k.as_str()),
                        jv,
                        boa_engine::property::Attribute::all(),
                    );
                }
            }
            let mut ok = true;
            if let Some(p) = &opts.prelude {
                ok = eval_into(&mut ctx, p, &mut outcome);
            }
            if ok {
                let src = source.replace("\r\n", "\n").replace('\r', "\n");
                eval_into(&mut ctx, &src, &mut outcome);
            }
            if let Some(e) = &opts.epilogue {
                let mut tmp = ScriptOutcome::default();
                if !eval_into(&mut ctx, e, &mut tmp) && outcome.ok {
                    outcome.ok = false;
                    outcome.error = tmp.error;
                }
            }
        }
        Err(e) => {
            outcome.ok = false;
            outcome.error = Some(format!("Laufzeit konnte nicht starten: {e}"));
        }
    }
    if let Some(st) = state::install(None) {
        outcome.alerts = st.alerts;
        outcome.save_as = st.save_as;
        outcome.log = st.log;
        outcome.diagnostics = st.diagnostics;
        outcome.not_implemented = st.not_implemented.into_iter().collect();
        outcome.used = st.used.into_iter().collect();
        let mut real = st.session;
        real.end_live_text_view();
        *session = real;
    }
    outcome
}

fn eval_into(ctx: &mut boa_engine::Context, src: &str, out: &mut ScriptOutcome) -> bool {
    match ctx.eval(Source::from_bytes(src)) {
        Ok(v) => {
            out.ok = true;
            out.result = if v.is_undefined() {
                None
            } else {
                v.to_string(ctx).ok().map(|s| s.to_std_string_escaped())
            };
            true
        }
        Err(e) => {
            out.ok = false;
            let opaque = e.clone().into_opaque(ctx).ok();
            if let Some(x) = opaque.as_ref().and_then(|v| v.as_number()) {
                out.thrown = Some(x);
                out.error = Some(conv::fmt_num(x));
            } else {
                let text = match &opaque {
                    Some(v) => v
                        .to_string(ctx)
                        .map(|s| s.to_std_string_escaped())
                        .unwrap_or_else(|_| e.to_string()),
                    None => e.to_string(),
                };
                out.error = Some(text);
            }
            false
        }
    }
}

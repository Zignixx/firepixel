//! Gemeinsame Teile der Gesamttests (HG): Testdokument mit allen Objektarten, Auswahl-Kandidaten,
//! Ausführen einer Aktion wie per Mausklick und Bewertung der Wirkung (inkl. Rückgängig-Prüfung).
#![allow(dead_code)]

use fp_ui::app_state::{Action, AppState, UiSettings};
use fp_ui::menu::{ItemAction, Node};
use serde_json::{json, Value};
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};

/// Kennungen der Testobjekte.
#[derive(Default, Clone, Debug)]
pub struct Fix {
    pub rect: Vec<u64>,
    pub ellipse: Vec<u64>,
    pub path: Vec<u64>,
    pub path2: Vec<u64>,
    pub text: Vec<u64>,
    pub bitmap: Vec<u64>,
    pub over_bitmap: Vec<u64>,
    pub group: Vec<u64>,
    pub slice: Vec<u64>,
    pub hotspot: Vec<u64>,
    pub instance: Vec<u64>,
    pub instance2: Vec<u64>,
    pub button: Vec<u64>,
    pub anim: Vec<u64>,
    pub mask: Vec<u64>,
    pub two: Vec<u64>,
    pub three: Vec<u64>,
    /// gespeicherte Testdatei (für Wiederherstellen, Öffnen, zuletzt geöffnet)
    pub file: PathBuf,
    /// Ziel für „Speichern unter“
    pub save_to: PathBuf,
}

pub fn sel(st: &AppState) -> Vec<u64> {
    st.session.active().map(|d| d.selection.elements.iter().map(|e| e.0).collect()).unwrap_or_default()
}

pub fn ex(st: &mut AppState, name: &str, p: Value) -> Vec<u64> {
    let _ = st.session.execute(name, p);
    sel(st)
}

pub fn select(st: &mut AppState, ids: &[u64]) {
    let _ = st.session.execute("select.set", json!({ "ids": ids }));
}

fn first(v: &[u64]) -> u64 {
    v.first().copied().unwrap_or(0)
}

/// Eindeutiger Ordner für Testdateien.
pub fn temp_dir() -> PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    let i = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d = std::env::temp_dir().join("firepixel-gesamttest").join(format!("{}-{i}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d
}

/// Testdokument: 800 × 600, zwei Seiten, zwei Status, alle Objektarten, Zwischenablage gefüllt,
/// Zoom 50 %, eine gespeicherte Testdatei in „zuletzt geöffnet“.
pub fn fixture() -> (AppState, egui::Context, Fix) {
    // keine Fremdprogramme aus Tests (AGENTEN_REGELN §4a)
    fp_engine::shell::disable();
    let session = fp_ui::ui_commands::new_session(fp_engine::Preferences::new());
    let ctx = egui::Context::default();
    let mut st = AppState::new(session, UiSettings::default(), None);
    st.sync_render = true;
    st.os_clipboard = false;
    let dir = temp_dir();
    let mut f = Fix { file: dir.join("vorlage.fw.png"), save_to: dir.join("gespeichert.fw.png"), ..Fix::default() };
    // kleine gespeicherte Datei (Öffnen/Vorlage/Importieren/zuletzt geöffnet)
    let _ = st.session.execute("file.new", json!({ "width": 120, "height": 80 }));
    let _ = st.session.execute("draw.rectangle", json!({ "rect": [10, 10, 60, 50], "fillColor": "#123456" }));
    // ein Symbol in der Datei (Bibliothek importieren/aktualisieren)
    let _ = st.session.execute("modify.symbol.convertToSymbol", json!({ "name": "Bibliothekssymbol", "kind": "graphic" }));
    let _ = st.session.execute("file.saveAs", json!({ "path": f.file.to_string_lossy() }));
    let _ = st.session.execute("file.close", json!({ "discard": true }));
    st.session.prefs.recent_files = vec![f.file.clone()];
    let script = dir.join("test.jsf");
    let _ = std::fs::write(&script, "var d = fw.getDocumentDOM(); d.addNewRectanglePrimitive({left: 1, top: 1, right: 20, bottom: 20}, 0);");
    let html = dir.join("test.htm");
    let _ = std::fs::write(&html, "<html><body></body></html>");
    let exe = dir.join("browser.exe");
    let _ = std::fs::write(&exe, b"");
    st.file_override = Some(fp_ui::files::FilePickOverride { open: vec![f.file.clone(), script, html, exe], save: Some(f.save_to.clone()) });

    let _ = st.session.execute("file.new", json!({ "width": 800, "height": 600 }));
    f.rect = ex(&mut st, "draw.rectangle", json!({ "rect": [40, 40, 200, 160], "fillColor": "#ff0000" }));
    f.ellipse = ex(&mut st, "draw.ellipse", json!({ "rect": [240, 40, 400, 160], "fillColor": "#00a0ff" }));
    f.path = ex(&mut st, "draw.line", json!({ "from": [40, 200], "to": [300, 260] }));
    f.path2 = ex(&mut st, "draw.line", json!({ "from": [320, 200], "to": [420, 260] }));
    f.text = ex(&mut st, "draw.text", json!({ "text": "Firepixel Test", "anchor": [40, 330], "size": 24 }));
    // Bitmap: Kopie eines Rechtecks reduzieren
    let tmp = ex(&mut st, "draw.rectangle", json!({ "rect": [440, 40, 560, 160], "fillColor": "#30c030" }));
    select(&mut st, &tmp);
    f.bitmap = ex(&mut st, "modify.flattenSelection", json!({}));
    if f.bitmap == tmp {
        f.bitmap.clear();
    }
    f.over_bitmap = ex(&mut st, "draw.rectangle", json!({ "rect": [460, 60, 500, 100], "fillColor": "#ffff00" }));
    // Gruppe
    let a = ex(&mut st, "draw.rectangle", json!({ "rect": [600, 40, 650, 90] }));
    let b = ex(&mut st, "draw.ellipse", json!({ "rect": [660, 40, 720, 90] }));
    select(&mut st, &[first(&a), first(&b)]);
    f.group = ex(&mut st, "modify.group", json!({}));
    // Maskengruppe
    let a = ex(&mut st, "draw.rectangle", json!({ "rect": [600, 120, 700, 200], "fillColor": "#8040ff" }));
    let b = ex(&mut st, "draw.ellipse", json!({ "rect": [610, 130, 690, 190], "fillColor": "#ffffff" }));
    select(&mut st, &[first(&a), first(&b)]);
    f.mask = ex(&mut st, "modify.mask.groupAsMask", json!({}));
    // Symbolinstanzen (Grafik mit zweiter Instanz, Schaltfläche, Animation)
    let s = ex(&mut st, "draw.rectangle", json!({ "rect": [40, 400, 140, 480], "fillColor": "#ffa000" }));
    select(&mut st, &s);
    f.instance = ex(&mut st, "modify.symbol.convertToSymbol", json!({ "name": "Grafik", "kind": "graphic" }));
    f.instance2 = ex(&mut st, "edit.duplicate", json!({}));
    if f.instance2 == f.instance {
        f.instance2.clear();
    }
    let s = ex(&mut st, "draw.rectangle", json!({ "rect": [160, 400, 260, 440], "fillColor": "#0080ff" }));
    select(&mut st, &s);
    f.button = ex(&mut st, "modify.symbol.convertToSymbol", json!({ "name": "Schaltfläche", "kind": "button" }));
    let s = ex(&mut st, "draw.ellipse", json!({ "rect": [160, 500, 220, 560], "fillColor": "#ff00ff" }));
    select(&mut st, &s);
    f.anim = ex(&mut st, "modify.animation.animateSelection", json!({ "name": "Animation" }));
    // Web-Objekte
    let s = ex(&mut st, "draw.rectangle", json!({ "rect": [300, 400, 400, 460] }));
    select(&mut st, &s);
    f.slice = ex(&mut st, "edit.insert.slice", json!({}));
    let s = ex(&mut st, "draw.rectangle", json!({ "rect": [420, 400, 520, 460] }));
    select(&mut st, &s);
    f.hotspot = ex(&mut st, "edit.insert.hotspot", json!({}));
    f.two = vec![first(&f.rect), first(&f.ellipse)];
    f.three = vec![first(&f.rect), first(&f.ellipse), first(&f.group)];
    // Zwischenablage
    select(&mut st, &f.ellipse);
    let _ = st.session.execute("edit.copy", json!({}));
    // zweite Seite und zweiter Status, dann zurück
    let _ = st.session.execute("page.add", json!({}));
    let _ = st.session.execute("page.setCurrent", json!({ "index": 0 }));
    let _ = st.session.execute("state.add", json!({}));
    let _ = st.session.execute("state.setCurrent", json!({ "index": 0 }));
    let _ = st.session.execute("select.set", json!({ "ids": [] }));
    let _ = st.session.execute("view.setZoom", json!({ "zoom": 0.5 }));
    // ein Stil aus dem Rechteck
    select(&mut st, &f.rect);
    let _ = st.session.execute("pi.style.new", json!({ "name": "Teststil" }));
    let _ = st.session.execute("select.set", json!({ "ids": [] }));
    if let Some(d) = st.session.active_mut() {
        d.history.clear();
    }
    // genau ein Verlaufsschritt (für Verlauf-Bedienfeld/Wiederholen)
    let _ = st.session.execute("draw.rectangle", json!({ "rect": [700, 500, 720, 520] }));
    let _ = st.session.execute("select.set", json!({ "ids": [] }));
    st.process_actions(&ctx);
    let _ = st.session.drain_events();
    st.status = None;
    st.dialogs = Default::default();
    // Auswahl in den Bedienfeldern (Verlaufsschritt, Stil, Bibliothekssymbol, Palettenfarbe)
    st.dialogs.panel_sel.history.insert(0);
    st.dialogs.panel_sel.styles = vec!["Teststil".into()];
    st.dialogs.panel_sel.symbol = Some(0);
    st.dialogs.panel_sel.palette = vec![0xffff_0000];
    (st, ctx, f)
}

/// Eine Testsituation: Vorbereitungsbefehle, Auswahl, optional Pixelauswahl.
#[derive(Clone, Debug)]
pub struct Candidate {
    pub label: &'static str,
    pub prep: Vec<(&'static str, Value)>,
    pub ids: Vec<u64>,
    pub pixels: bool,
}

fn c(label: &'static str, ids: Vec<u64>) -> Candidate {
    Candidate { label, prep: vec![], ids, pixels: false }
}

/// Auswahl-Kandidaten in sinnvoller Reihenfolge.
pub fn candidates(f: &Fix) -> Vec<Candidate> {
    let save = json!({ "path": f.save_to.to_string_lossy() });
    vec![
        c("ohne Auswahl", vec![]),
        c("Rechteck", f.rect.clone()),
        c("Rechteck + Ellipse", f.two.clone()),
        c("drei Objekte", f.three.clone()),
        c("Text", f.text.clone()),
        c("Text + Ellipse", vec![first(&f.text), first(&f.ellipse)]),
        c("Bitmap", f.bitmap.clone()),
        c("Rechteck über Bitmap", f.over_bitmap.clone()),
        c("Gruppe", f.group.clone()),
        c("Maskengruppe", f.mask.clone()),
        c("Grafiksymbol", f.instance.clone()),
        c("zwei Instanzen", vec![first(&f.instance), first(&f.instance2)]),
        c("Schaltfläche", f.button.clone()),
        c("Animationssymbol", f.anim.clone()),
        c("Segment", f.slice.clone()),
        c("Hotspot", f.hotspot.clone()),
        c("Pfad", f.path.clone()),
        c("zwei offene Pfade", vec![first(&f.path), first(&f.path2)]),
        c("Ellipse", f.ellipse.clone()),
        Candidate { label: "Pixelauswahl", prep: vec![], ids: vec![], pixels: true },
        Candidate { label: "Bitmap + Pixelauswahl", prep: vec![], ids: f.bitmap.clone(), pixels: true },
        Candidate { label: "gespeichert + geändert", prep: vec![("file.saveAs", save.clone()), ("draw.rectangle", json!({ "rect": [10, 10, 30, 30] }))], ids: vec![], pixels: false },
        Candidate { label: "nach Rückgängig", prep: vec![("draw.rectangle", json!({ "rect": [10, 10, 30, 30] })), ("edit.undo", json!({}))], ids: vec![], pixels: false },
        Candidate { label: "nach Filter", prep: vec![("select.set", json!({ "ids": f.bitmap })), ("filter.adjustColor.invert", json!({}))], ids: f.bitmap.clone(), pixels: false },
        Candidate {
            label: "Text auf Pfad",
            prep: vec![("select.set", json!({ "ids": [first(&f.text), first(&f.ellipse)] })), ("text.attachToPath", json!({}))],
            ids: vec![],
            pixels: false,
        },
        Candidate {
            label: "Segment mit Pop-up-Menü",
            prep: vec![("select.set", json!({ "ids": f.slice })), ("modify.popUpMenu.addPopUpMenu", json!({ "items": [{ "text": "Eins", "url": "eins.htm" }] }))],
            ids: f.slice.clone(),
            pixels: false,
        },
        Candidate {
            label: "mit JPEG-Maske",
            prep: vec![("select.pixels", json!({ "shape": "rect", "rect": [40, 40, 120, 120] })), ("modify.selectiveJpeg.saveSelectionAsJpegMask", json!({})), ("select.pixels", json!({ "shape": "none" }))],
            ids: vec![],
            pixels: false,
        },
        Candidate { label: "zwei Ebenen", prep: vec![("layer.add", json!({}))], ids: vec![], pixels: false },
        Candidate { label: "freigegebene Ebene", prep: vec![("panel.layers.shareToAllPages", json!({})), ("page.setCurrent", json!({ "index": 1 }))], ids: vec![], pixels: false },
        Candidate { label: "Masterseite", prep: vec![("pi.page.setMaster", json!({ "index": 0 })), ("page.setCurrent", json!({ "index": 1 }))], ids: vec![], pixels: false },
        Candidate { label: "importiertes Symbol", prep: vec![("panel.library.import", json!({ "path": f.file.to_string_lossy() }))], ids: vec![], pixels: false },
        Candidate { label: "zweites Dokument", prep: vec![("file.new", json!({ "width": 100, "height": 100 }))], ids: vec![], pixels: false },
    ]
}

pub fn apply_candidate(st: &mut AppState, cand: &Candidate) {
    for (n, p) in &cand.prep {
        let _ = st.session.execute(n, p.clone());
    }
    if cand.label != "Text auf Pfad" && cand.label != "zweites Dokument" {
        select(st, &cand.ids);
    }
    if cand.pixels {
        let _ = st.session.execute("select.pixels", json!({ "shape": "rect", "rect": [440, 40, 520, 120], "mode": "replace" }));
        if !cand.ids.is_empty() {
            // Pixelauswahl ersetzt evtl. die Objektauswahl: Objekt wieder dazunehmen
            let _ = st.session.execute("select.set", json!({ "ids": cand.ids, "mode": "add" }));
        }
    }
    if cand.label == "importiertes Symbol" {
        let n = st.session.active().map(|d| d.doc.symbols.len()).unwrap_or(0);
        st.dialogs.panel_sel.symbol = n.checked_sub(1);
    }
    let _ = st.session.drain_events();
    st.status = None;
}

pub fn available(st: &AppState, name: &str) -> bool {
    match fp_ui::actions::ui_command_state(st, name) {
        Some((en, _)) => en,
        None => st.session.is_available(name),
    }
}
/// Befehle, die externe Programme öffnen könnten (Browser, Drucker, Hilfe-Webseiten): nur von Hand.
pub fn external(name: &str) -> bool {
    let n = name.to_lowercase();
    if std::env::var("FP_NO_SHELL_OPEN").as_deref() == Ok("1") {
        // die Engine startet dann keine Fremdprogramme (AGENTEN_REGELN §4a, `shell::disable`)
        return false;
    }
    let risky = n.contains("browser") || n.starts_with("help.") && n != "help.about" || n.contains("print") || n.contains("bridge") || n.contains("twain") || n.contains("scan") || n.contains("mail") || n.contains("extension") || n.contains("exchange");
    // ohne FP_NO_SHELL_OPEN=1 (AGENTEN_REGELN §4a) zusätzlich alles, was evtl. Programme startet
    risky || std::env::var("FP_NO_SHELL_OPEN").as_deref() != Ok("1") && (n.contains("preview") || n.contains("open") && n.contains("folder"))
}

#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// Verlaufsschritt; Rückgängig stellt exakt wieder her.
    Doc,
    /// Dokument geändert, Rückgängig stellt nicht exakt her.
    UndoBroken,
    /// Dokument geändert ohne Verlaufsschritt.
    DocNoHistory,
    Dialog,
    Ui,
    Nothing(String),
    NotImplemented,
    Error(String),
    Panic(String),
    NeverAvailable,
    NoFunction,
    Manual,
}

impl Outcome {
    pub fn ok(&self) -> bool {
        matches!(self, Outcome::Doc | Outcome::Dialog | Outcome::Ui)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Probe {
    /// aktives Dokument und Anzahl
    pub docs: String,
    /// Dateien im Testordner (Name, Größe)
    pub files: String,
    /// Kopierbefehle an die Systemzwischenablage
    pub os_clipboard: usize,
    pub generation: u64,
    pub undo: usize,
    pub doc: Value,
    pub ui: String,
}

thread_local! {
    /// egui-Kontext des laufenden Tests (Systemzwischenablage = Ausgabebefehle).
    pub static OS_CLIP: std::cell::RefCell<Option<egui::Context>> = const { std::cell::RefCell::new(None) };
}

pub fn probe(st: &AppState) -> Probe {
    let (generation, undo, doc) = match st.session.active() {
        Some(d) => (d.generation, d.history.undo_count(), serde_json::to_value(&*d.doc).unwrap_or(Value::Null)),
        None => (0, 0, Value::Null),
    };
    let view = st.session.active().map(|d| format!("{:?}|{}|{}|{:?}|{}|{:?}|{:?}|{:?}|{}|{}", d.view, d.page, d.frame, d.layer, d.bitmap_mode, d.selection.elements, d.selection.pixels.is_some(), d.path, d.saved_generation, d.title)).unwrap_or_default();
    let ui = format!(
        "{:?}|{:?}|{:?}|{:?}|{}|{:?}|{}|{:?}|{:?}|{}",
        st.settings,
        st.session.ui,
        st.session.tools.active,
        (st.session.tools.stroke_color, st.session.tools.fill_color),
        view,
        st.session.active_id(),
        st.session.documents().len(),
        (st.session.clipboard.elements.iter().map(|e| std::sync::Arc::as_ptr(e) as usize).collect::<Vec<_>>(), st.session.clipboard.pixels.is_some(), st.session.prefs.recent_files.len()),
        st.session.prefs.theme,
        st.close_window,
    ) + &format!("|{:?}|{:?}", st.session.prefs, st.dialogs.panel_sel);
    let docs = format!("{:?}|{}", st.session.active_id(), st.session.documents().len());
    let files = st
        .file_override
        .as_ref()
        .and_then(|o| o.save.as_ref())
        .and_then(|p| p.parent())
        .and_then(|d| std::fs::read_dir(d).ok())
        .map(|rd| {
            let mut v: Vec<String> = rd.flatten().map(|e| format!("{}:{}", e.file_name().to_string_lossy(), e.metadata().map(|m| m.len()).unwrap_or(0))).collect();
            v.sort();
            v.join(",")
        })
        .unwrap_or_default();
    let os_clipboard = OS_CLIP.with(|c| c.borrow().as_ref().map(|ctx| ctx.output(|o| o.commands.len())).unwrap_or(0));
    Probe { docs, files, os_clipboard, generation, undo, doc, ui }
}

pub fn status_text(st: &AppState) -> String {
    st.status.as_ref().map(|s| s.0.clone()).unwrap_or_default()
}

/// Führt einen Menübefehl aus und bewertet das Ergebnis.
pub fn run_menu_command(name: &str) -> (Outcome, String) {
    run_item(Action::Run(name.to_string(), json!({})), Some(name))
}

/// Bewertet die Wirkung nach einer Aktion (`before` = Zustand vorher).
pub fn judge(st: &mut AppState, before: &Probe, used: &str) -> (Outcome, String) {
    let status = status_text(st);
    if status.contains("noch nicht umgesetzt") || status.contains("nicht verfügbar") && status.contains("noch") {
        return (Outcome::NotImplemented, format!("{used}: {status}"));
    }
    if st.dialogs.any_open() || st.dialogs.pending_form.is_some() {
        let what = st.dialogs.message.as_ref().map(|m| m.1.clone()).unwrap_or_default();
        if what.contains("noch nicht") {
            return (Outcome::NotImplemented, format!("{used}: Meldung „{what}“"));
        }
        return (Outcome::Dialog, format!("{used}{}", if what.is_empty() { String::new() } else { format!(": Meldung „{what}“") }));
    }
    let after = probe(st);
    if after.docs != before.docs {
        return (Outcome::Ui, format!("{used}: Dokument geöffnet/gewechselt"));
    }
    if after.undo > before.undo {
        let _ = st.session.undo();
        let back = probe(st);
        if back.doc == before.doc {
            return (Outcome::Doc, used.into());
        }
        return (Outcome::UndoBroken, format!("{used}: Rückgängig stellt das Dokument nicht exakt her"));
    }
    if after.doc != before.doc {
        return (Outcome::DocNoHistory, format!("{used}: Dokument geändert ohne Verlaufsschritt"));
    }
    if after.ui != before.ui || after.generation != before.generation {
        return (Outcome::Ui, used.into());
    }
    if after.files != before.files {
        return (Outcome::Ui, format!("{used}: Datei geschrieben"));
    }
    if after.os_clipboard != before.os_clipboard {
        return (Outcome::Ui, format!("{used}: in die Zwischenablage"));
    }
    if !status.is_empty() {
        let low = status.to_lowercase();
        let bad = ["fehlt", "kein", "nicht", "fehler", "ungültig", "bitte", "kann "].iter().any(|w| low.contains(w));
        if !bad {
            return (Outcome::Ui, format!("{used}: Meldung „{status}“"));
        }
        return (Outcome::Error(status.clone()), format!("{used}: {status}"));
    }
    (Outcome::Nothing(String::new()), format!("{used}: keine erkennbare Wirkung"))
}

/// Befehle, die einen Bearbeitungsmodus umschalten (Dokumentzustand ohne Verlaufsschritt ist richtig).
pub fn mode_command(name: &str) -> bool {
    matches!(name, "modify.symbol.editSymbol" | "modify.symbol.editInPlace" | "edit.editBitmap" | "edit.exitBitmapMode" | "panel.library.edit" | "file.revert")
}

/// Führt eine Menüaktion aus (`avail` = Befehlsname für die Verfügbarkeit) und bewertet sie.
/// Probiert alle Testsituationen, in denen der Eintrag verfügbar ist, bis eine Wirkung zeigt.
pub fn run_item(action: Action, avail: Option<&str>) -> (Outcome, String) {
    let name = avail.unwrap_or("");
    if avail.is_some() && external(name) {
        return (Outcome::Manual, "öffnet externe Programme – GUI-Sitzung".into());
    }
    let (_, _, f0) = fixture();
    let mut first_fail: Option<(Outcome, String)> = None;
    for cand in candidates(&f0) {
        let (mut st, ctx, _f) = fixture();
        // Design-Einträge: vorher ein anderes Design setzen
        if let Action::SetTheme(k) = &action {
            let other = fp_ui::theme::ThemeKind::ALL.iter().copied().find(|x| x != k).unwrap_or(*k);
            st.set_theme(other);
        }
        // „Vollbildmodus beenden“: vorher Vollbild
        if name == "view.exitFullScreen" {
            st.session.ui.screen_mode = 1;
        }
        // Arbeitsbereich-Einträge: vorher einen anderen Arbeitsbereich einstellen
        if matches!(action, Action::LoadWorkspace(_) | Action::ResetWorkspace) {
            st.settings.workspace.dock_mode = fp_ui::workspace::DockMode::Iconic;
            st.settings.workspace.name = "Test".into();
        }
        apply_candidate(&mut st, &cand);
        if let Action::Run(n, p) = &action
            && n == "ui.closeGroup"
            && let Some(name) = p.get("panel").and_then(|v| v.as_str())
            && let Some(pid) = fp_ui::panels::PanelId::ALL.iter().find(|x| format!("{x:?}") == name)
        {
            st.settings.workspace.show(*pid);
        }
        if avail.is_some() && !available(&st, name) {
            continue;
        }
        st.status = None;
        let before = probe(&st);
        OS_CLIP.with(|c| *c.borrow_mut() = Some(ctx.clone()));
        let act = action.clone();
        let res = std::panic::catch_unwind(AssertUnwindSafe(move || {
            st.push(act);
            st.process_actions(&ctx);
            st
        }));
        let mut st = match res {
            Ok(s) => s,
            Err(e) => return (Outcome::Panic(panic_msg(&e)), cand.label.into()),
        };
        let (mut o, note) = judge(&mut st, &before, cand.label);
        if matches!(o, Outcome::Nothing(_)) && avail.is_some() && st.session.command_state(name).1 == Some(true) {
            o = Outcome::Ui;
        }
        if o == Outcome::DocNoHistory && mode_command(name) {
            o = Outcome::Ui;
        }
        if o.ok() || matches!(o, Outcome::UndoBroken | Outcome::NotImplemented) {
            return (o, note);
        }
        if first_fail.is_none() {
            let shell = ["browser", "help.", "mail", "bridge", "exchange", "extension", "print", "twain", "pagesetup"].iter().any(|k| name.to_lowercase().contains(k));
            let suppressed = matches!(&o, Outcome::Error(e) if e.contains("abgeschaltet") || e.contains("FP_NO_SHELL_OPEN"));
            if shell && (matches!(o, Outcome::Nothing(_)) || suppressed) {
                // Befehl selbst ausführen: liefert er das Ziel (Datei, Adresse), das gestartet würde,
                // ist alles außer dem Start des Fremdprogramms geprüft (im Test abgeschaltet)
                let (mut st2, _c2, _f2) = fixture();
                apply_candidate(&mut st2, &cand);
                let params = match &action {
                    Action::Run(_, p) => p.clone(),
                    _ => json!({}),
                };
                let target = st2.session.execute(name, params).ok().map(|out| out.value).filter(|v| !v.is_null());
                first_fail = Some(match target {
                    Some(v) => {
                        let s = v.to_string();
                        let s: String = s.chars().take(140).collect();
                        (Outcome::Ui, format!("{}: Befehl liefert {s} – Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN)", cand.label))
                    }
                    None => (Outcome::Manual, format!("{}: Befehl ausgeführt, Start des Fremdprogramms im Test abgeschaltet (FP_NO_SHELL_OPEN) – Sichtprüfung in echter Sitzung", cand.label)),
                });
            } else {
                first_fail = Some((o, note));
            }
        }
    }
    first_fail.unwrap_or((Outcome::NeverAvailable, "in keiner Testsituation verfügbar".into()))
}
pub fn panic_msg(e: &Box<dyn std::any::Any + Send>) -> String {
    e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default()
}

/// Ein Eintrag im Bericht.
#[derive(Clone, Debug)]
pub struct Row {
    pub path: String,
    pub command: String,
    pub outcome: Outcome,
    pub note: String,
}

pub fn walk(nodes: &[Node], prefix: &str, out: &mut Vec<(String, ItemAction)>) {
    for n in nodes {
        match n {
            Node::Separator => {}
            Node::Item { label, action, .. } => out.push((format!("{prefix} ▸ {label}"), action.clone())),
            Node::Sub { label, children, .. } => walk(children, &format!("{prefix} ▸ {label}"), out),
        }
    }
}

/// Menüeinträge ausführen und bewerten (Schriftenliste: nur die ersten 3 Einträge).
pub fn run_nodes(items: Vec<(String, ItemAction)>) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut fonts = 0;
    for (path, action) in items {
        if path.contains("Schrift ▸") {
            fonts += 1;
            if fonts > 3 {
                continue;
            }
        }
        let row = match action {
            ItemAction::Command(c) => {
                let (o, note) = run_menu_command(&c);
                Row { path, command: c, outcome: o, note }
            }
            ItemAction::Ui(a) => {
                let cmd = match &a {
                    Action::Run(n, p) => format!("{n} {p}"),
                    other => format!("{other:?}"),
                };
                let avail = match &a {
                    Action::Run(n, _) => Some(n.clone()),
                    _ => None,
                };
                let (o, note) = run_item(a, avail.as_deref());
                Row { path, command: cmd, outcome: o, note }
            }
            ItemAction::None if path.rsplit(" ▸ ").next().is_some_and(|l| l.starts_with("Keine")) => Row { path, command: String::new(), outcome: Outcome::Ui, note: "Hinweiseintrag (leere Laufzeitliste)".into() },
            ItemAction::None => Row { path, command: String::new(), outcome: Outcome::NoFunction, note: "Eintrag ohne Befehl (gesperrt)".into() },
        };
        rows.push(row);
    }
    rows
}

pub fn report_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("docs").join("funktionstest")
}

pub fn symbol(o: &Outcome) -> &'static str {
    if o.ok() {
        "✅"
    } else if matches!(o, Outcome::Manual) {
        "🔶"
    } else {
        "❌"
    }
}

/// Schreibt einen Bericht als Markdown-Tabelle; liefert (ok, gesamt).
pub fn write_report(file: &str, title: &str, source: &str, rows: &[Row]) -> (usize, usize) {
    let ok = rows.iter().filter(|r| r.outcome.ok()).count();
    let mut md = String::new();
    md.push_str(&format!("# {title} – automatischer Gesamttest\n\n"));
    md.push_str(&format!("Erzeugt von `crates\\fp-ui\\tests\\{source}` (`cargo test -p fp-ui --test {}`). Nicht von Hand ändern.\n\n", source.trim_end_matches(".rs")));
    md.push_str(&format!("**{ok}/{} Einträge ✅**\n\n| Eintrag | Befehl | Ergebnis | Hinweis |\n|---|---|---|---|\n", rows.len()));
    for r in rows {
        md.push_str(&format!("| {} | `{}` | {} {:?} | {} |\n", r.path, r.command.replace('|', "/"), symbol(&r.outcome), r.outcome, r.note.replace('|', "/").replace('\n', " ")));
    }
    let dir = report_dir();
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join(file), md);
    (ok, rows.len())
}

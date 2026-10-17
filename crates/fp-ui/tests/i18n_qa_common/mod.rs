//! Prüfgerüst der Mehrsprachigkeit (Agent I18N-QA, Phase 6, `docs\I18N_PRUEFUNG.md`).
//!
//! Geht die ganze Oberfläche offscreen (egui_kittest) in einer gewählten Sprache durch –
//! Hauptfenster in allen Designs, jedes Menü und Untermenü, jedes Bedienfeld mit Bedienfeldmenü,
//! den Inspektor für jede Elementart und jedes Werkzeug, jeden über Menüs erreichbaren Dialog,
//! die Kontextmenüs und die Statuszeile – und prüft je Bild:
//!
//! * (a) **Rohschlüssel**: sichtbarer Text, der wie ein Schlüssel aussieht (`dlg-canvas-size`),
//! * (b) **unmarkierter Text** in der Testsprache `qps` (jeder Text aus `fp_i18n::t` trägt `[!! … !!]`;
//!   ohne Markierung ist er fest im Code; Eigennamen, Formatnamen, Zahlen und Symbole ausgenommen),
//! * (c) **Überlauf**: Text waagerecht abgeschnitten (ohne „…“), Text über den Fensterrand, Texte
//!   überlappen sich, Text ragt teilweise in ein fremdes Bedienelement, Bedienelement über den Rand,
//! * (d) **Deutsch unverändert**: Liste aller gezeichneten Texte mit Lage je Bild gegen den Stand
//!   vor Phase 6 (Commit `30ddaec`, erzeugt mit `FP_QA_MODE=baseline`).
//!
//! Umgebungsvariablen (TE2-E, englisch; die alten deutschen Namen und Werte werden weiter gelesen):
//! `FP_QA_MODE` (`current`/`baseline`, alt `FP_QA_MODUS` mit `aktuell`/`basis`), `FP_QA_OUT`
//! (Ausgabeordner, alt `FP_QA_AUS`), `FP_QA_ONLY` (Teil der Bild-ID, alt `FP_QA_NUR`), `FP_QA_NEW`
//! (Dialoge neu suchen, alt `FP_QA_NEU`).
//!
//! Ergebnisse als JSON nach `target-i18n-qa\i18n-qa\<modus>\<sprache>_<bereich>.json`; der Test
//! `i18n_qa_bericht` fasst sie in `docs\I18N_PRUEFUNG.md` zusammen.
#![allow(dead_code)]

use egui::epaint::Shape;
use egui::{Event, Id, LayerId, Modifiers, PointerButton, Pos2, Rect, Sense};
use egui_kittest::Harness;
use fp_ui::app_state::{Action, AppState, UiSettings};
use fp_ui::menu::{ItemAction, Node};
use fp_ui::panels::PanelId;
use fp_ui::theme::ThemeKind;
use fp_ui::FirepixelApp;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::panic::AssertUnwindSafe;
use std::path::PathBuf;

pub type H = Harness<'static, FirepixelApp>;

// ------------------------------------------------------------------ Konfiguration

/// Fenstergröße (Gerätepixel) und Skalierung.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cfg {
    pub w: f32,
    pub h: f32,
    pub ppp: f32,
}

impl Cfg {
    pub fn label(&self) -> String {
        format!("{}x{}@{:.0}", self.w as u32, self.h as u32, self.ppp * 100.0)
    }
}

/// Größe für die Textliste (Deutsch unverändert) und die Text-/Schlüsselprüfung.
pub const CFG_TEXT: Cfg = Cfg { w: 1600.0, h: 1000.0, ppp: 1.0 };
/// Größen für die Überlaufprüfung (Auftrag: 1024×768 und 1920×1080, 100 % und 150 %).
pub const CFG_LAYOUT: [Cfg; 4] = [
    Cfg { w: 1024.0, h: 768.0, ppp: 1.0 },
    Cfg { w: 1920.0, h: 1080.0, ppp: 1.0 },
    Cfg { w: 1024.0, h: 768.0, ppp: 1.5 },
    Cfg { w: 1920.0, h: 1080.0, ppp: 1.5 },
];

/// Wurzel des Repositorys.
pub fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// Wert einer Umgebungsvariablen; ohne den neuen (englischen) Namen der alte deutsche als Rückfall.
pub fn env_var(name: &str, legacy: &str) -> Option<String> {
    std::env::var(name).or_else(|_| std::env::var(legacy)).ok()
}

/// Modus `current` (Standard).
pub const MODE_CURRENT: &str = "current";
/// Modus `baseline` (Stand vor Phase 6 erzeugen).
pub const MODE_BASELINE: &str = "baseline";

/// Modus: `current` (Standard) oder `baseline` (Stand vor Phase 6 erzeugen); aus `FP_QA_MODE`
/// (alt `FP_QA_MODUS`, alte Werte `aktuell`/`basis` werden umgesetzt).
pub fn mode() -> String {
    match env_var("FP_QA_MODE", "FP_QA_MODUS").as_deref() {
        Some("baseline" | "basis") => MODE_BASELINE.into(),
        Some("current" | "aktuell") | None => MODE_CURRENT.into(),
        Some(other) => other.into(),
    }
}

/// Ergebnisordner eines Modus; liegt dort noch nichts, der Ordner mit dem früheren deutschen
/// Namen (`aktuell`/`basis`), falls vorhanden.
pub fn mode_dir(mode: &str) -> PathBuf {
    let dir = out_root().join(mode);
    let legacy = match mode {
        MODE_CURRENT => "aktuell",
        MODE_BASELINE => "basis",
        _ => return dir,
    };
    let old = out_root().join(legacy);
    if !dir.is_dir() && old.is_dir() { old } else { dir }
}

/// Ausgabeordner der Ergebnisse (`FP_QA_OUT`, alt `FP_QA_AUS`, sonst `<repo>\target-i18n-qa\i18n-qa`).
pub fn out_root() -> PathBuf {
    match env_var("FP_QA_OUT", "FP_QA_AUS") {
        Some(p) => PathBuf::from(p),
        None => repo().join("target-i18n-qa").join("i18n-qa"),
    }
}

/// Nur ausgewählte Szenen/Bilder (`FP_QA_ONLY`, alt `FP_QA_NUR`; Teilzeichenkette der ID).
pub fn only_filter() -> Option<String> {
    env_var("FP_QA_ONLY", "FP_QA_NUR").filter(|s| !s.is_empty())
}

pub fn out_dir() -> PathBuf {
    let d = out_root().join(mode());
    let _ = std::fs::create_dir_all(&d);
    d
}

/// Nur ausgewählte Bilder (`FP_QA_ONLY=menues:3` o. ä., Teilzeichenkette der Bild-ID).
fn only() -> Option<String> {
    only_filter()
}

/// Sprache setzen (vor dem Aufbau der Oberfläche).
pub fn set_lang(lang: &str) {
    fp_engine::shell::disable();
    assert!(fp_i18n::set_language(lang), "Sprache {lang} unbekannt");
}

// ------------------------------------------------------------------ Testdokument

/// Kennungen der Testobjekte.
#[derive(Default, Clone, Debug)]
pub struct Fix {
    pub rect: Vec<u64>,
    pub ellipse: Vec<u64>,
    pub path: Vec<u64>,
    pub path2: Vec<u64>,
    pub text: Vec<u64>,
    pub bitmap: Vec<u64>,
    pub group: Vec<u64>,
    pub slice: Vec<u64>,
    pub hotspot: Vec<u64>,
    pub instance: Vec<u64>,
    pub instance2: Vec<u64>,
    pub button: Vec<u64>,
    pub anim: Vec<u64>,
    pub mask: Vec<u64>,
    pub autoshape: Vec<u64>,
    pub file: PathBuf,
    pub save_to: PathBuf,
}

pub fn sel(st: &AppState) -> Vec<u64> {
    st.session.active().map(|d| d.selection.elements.iter().map(|e| e.0).collect()).unwrap_or_default()
}

fn ex(st: &mut AppState, name: &str, p: Value) -> Vec<u64> {
    let _ = st.session.execute(name, p);
    sel(st)
}

pub fn select(st: &mut AppState, ids: &[u64]) {
    let _ = st.session.execute("select.set", json!({ "ids": ids }));
}

fn first(v: &[u64]) -> u64 {
    v.first().copied().unwrap_or(0)
}

fn temp_dir() -> PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let i = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d = std::env::temp_dir().join("firepixel-i18n-qa").join(format!("{}-{i}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d
}

/// Testdokument 800 × 600 mit allen Objektarten (wie die Gesamttests), zwei Seiten, zwei Status,
/// Zwischenablage, Stil, zuletzt geöffnete Datei.
pub fn fixture() -> (AppState, egui::Context, Fix) {
    fp_engine::shell::disable();
    let session = fp_ui::ui_commands::new_session(fp_engine::Preferences::new());
    let ctx = egui::Context::default();
    let mut st = AppState::new(session, UiSettings::default(), None);
    st.sync_render = true;
    st.os_clipboard = false;
    let dir = temp_dir();
    let mut f = Fix { file: dir.join("vorlage.fw.png"), save_to: dir.join("gespeichert.fw.png"), ..Fix::default() };
    let _ = st.session.execute("file.new", json!({ "width": 120, "height": 80 }));
    let _ = st.session.execute("draw.rectangle", json!({ "rect": [10, 10, 60, 50], "fillColor": "#123456" }));
    let _ = st.session.execute("modify.symbol.convertToSymbol", json!({ "name": "Bibliothekssymbol", "kind": "graphic" }));
    let _ = st.session.execute("file.saveAs", json!({ "path": f.file.to_string_lossy() }));
    let _ = st.session.execute("file.close", json!({ "discard": true }));
    st.session.prefs.recent_files = vec![f.file.clone()];
    let script = dir.join("test.jsf");
    let _ = std::fs::write(&script, "var d = fw.getDocumentDOM();");
    st.file_override = Some(fp_ui::files::FilePickOverride { open: vec![f.file.clone(), script], save: Some(f.save_to.clone()) });

    let _ = st.session.execute("file.new", json!({ "width": 800, "height": 600 }));
    f.rect = ex(&mut st, "draw.rectangle", json!({ "rect": [40, 40, 200, 160], "fillColor": "#ff0000" }));
    f.ellipse = ex(&mut st, "draw.ellipse", json!({ "rect": [240, 40, 400, 160], "fillColor": "#00a0ff" }));
    f.path = ex(&mut st, "draw.line", json!({ "from": [40, 200], "to": [300, 260] }));
    f.path2 = ex(&mut st, "draw.line", json!({ "from": [320, 200], "to": [420, 260] }));
    f.text = ex(&mut st, "draw.text", json!({ "text": "Firepixel Test", "anchor": [40, 330], "size": 24 }));
    let tmp = ex(&mut st, "draw.rectangle", json!({ "rect": [440, 40, 560, 160], "fillColor": "#30c030" }));
    select(&mut st, &tmp);
    f.bitmap = ex(&mut st, "modify.flattenSelection", json!({}));
    if f.bitmap == tmp {
        f.bitmap.clear();
    }
    let a = ex(&mut st, "draw.rectangle", json!({ "rect": [600, 40, 650, 90] }));
    let b = ex(&mut st, "draw.ellipse", json!({ "rect": [660, 40, 720, 90] }));
    select(&mut st, &[first(&a), first(&b)]);
    f.group = ex(&mut st, "modify.group", json!({}));
    let a = ex(&mut st, "draw.rectangle", json!({ "rect": [600, 120, 700, 200], "fillColor": "#8040ff" }));
    let b = ex(&mut st, "draw.ellipse", json!({ "rect": [610, 130, 690, 190], "fillColor": "#ffffff" }));
    select(&mut st, &[first(&a), first(&b)]);
    f.mask = ex(&mut st, "modify.mask.groupAsMask", json!({}));
    let s = ex(&mut st, "draw.rectangle", json!({ "rect": [40, 400, 140, 480], "fillColor": "#ffa000" }));
    select(&mut st, &s);
    f.instance = ex(&mut st, "modify.symbol.convertToSymbol", json!({ "name": "Grafik", "kind": "graphic" }));
    f.instance2 = ex(&mut st, "edit.duplicate", json!({}));
    if f.instance2 == f.instance {
        f.instance2.clear();
    }
    let s = ex(&mut st, "draw.rectangle", json!({ "rect": [160, 400, 260, 440], "fillColor": "#0080ff" }));
    select(&mut st, &s);
    f.button = ex(&mut st, "modify.symbol.convertToSymbol", json!({ "name": "Knopf", "kind": "button" }));
    let s = ex(&mut st, "draw.ellipse", json!({ "rect": [160, 500, 220, 560], "fillColor": "#ff00ff" }));
    select(&mut st, &s);
    f.anim = ex(&mut st, "modify.animation.animateSelection", json!({ "name": "Animation" }));
    let s = ex(&mut st, "draw.rectangle", json!({ "rect": [300, 400, 400, 460] }));
    select(&mut st, &s);
    f.slice = ex(&mut st, "edit.insert.slice", json!({}));
    let s = ex(&mut st, "draw.rectangle", json!({ "rect": [420, 400, 520, 460] }));
    select(&mut st, &s);
    f.hotspot = ex(&mut st, "edit.insert.hotspot", json!({}));
    f.autoshape = ex(&mut st, "autoShape.insert", json!({ "shape": "Star", "at": [620, 300] }));
    select(&mut st, &f.ellipse);
    let _ = st.session.execute("edit.copy", json!({}));
    let _ = st.session.execute("page.add", json!({}));
    let _ = st.session.execute("page.setCurrent", json!({ "index": 0 }));
    let _ = st.session.execute("state.add", json!({}));
    let _ = st.session.execute("state.setCurrent", json!({ "index": 0 }));
    let _ = st.session.execute("select.set", json!({ "ids": [] }));
    let _ = st.session.execute("view.setZoom", json!({ "zoom": 0.5 }));
    select(&mut st, &f.rect);
    let _ = st.session.execute("pi.style.new", json!({ "name": "Teststil" }));
    let _ = st.session.execute("select.set", json!({ "ids": [] }));
    if let Some(d) = st.session.active_mut() {
        d.history.clear();
    }
    let _ = st.session.execute("draw.rectangle", json!({ "rect": [700, 500, 720, 520] }));
    let _ = st.session.execute("select.set", json!({ "ids": [] }));
    st.process_actions(&ctx);
    let _ = st.session.drain_events();
    st.status = None;
    st.dialogs = Default::default();
    st.dialogs.panel_sel.history.insert(0);
    st.dialogs.panel_sel.styles = vec!["Teststil".into()];
    st.dialogs.panel_sel.symbol = Some(0);
    st.dialogs.panel_sel.palette = vec![0xffff_0000];
    (st, ctx, f)
}

/// Inhalte des Testdokuments (Nutzerdaten, nie übersetzt).
pub const FIXTURE_TEXTS: &[&str] = &["Firepixel Test", "Bibliothekssymbol", "Grafik", "Knopf", "Animation", "Teststil", "vorlage", "gespeichert", "Eins", "Mein Arbeitsbereich", "Testbefehl", ".htm"];

/// Eine Testsituation (Auswahl, Vorbereitungsbefehle).
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

/// Auswahl-Situationen: Inspektor je Elementart und Verfügbarkeit der Menübefehle.
pub fn candidates(f: &Fix) -> Vec<Candidate> {
    let save = json!({ "path": f.save_to.to_string_lossy() });
    vec![
        c("ohne Auswahl", vec![]),
        c("Rechteck", f.rect.clone()),
        c("Rechteck + Ellipse", vec![first(&f.rect), first(&f.ellipse)]),
        c("drei Objekte", vec![first(&f.rect), first(&f.ellipse), first(&f.group)]),
        c("Text", f.text.clone()),
        c("Text + Ellipse", vec![first(&f.text), first(&f.ellipse)]),
        c("Bitmap", f.bitmap.clone()),
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
        c("Automatische Form", f.autoshape.clone()),
        Candidate { label: "Pixelauswahl", prep: vec![], ids: vec![], pixels: true },
        Candidate { label: "Bitmap + Pixelauswahl", prep: vec![], ids: f.bitmap.clone(), pixels: true },
        Candidate { label: "gespeichert + geändert", prep: vec![("file.saveAs", save), ("draw.rectangle", json!({ "rect": [10, 10, 30, 30] }))], ids: vec![], pixels: false },
        Candidate { label: "nach Rückgängig", prep: vec![("draw.rectangle", json!({ "rect": [10, 10, 30, 30] })), ("edit.undo", json!({}))], ids: vec![], pixels: false },
        Candidate { label: "Text auf Pfad", prep: vec![("select.set", json!({ "ids": [first(&f.text), first(&f.ellipse)] })), ("text.attachToPath", json!({}))], ids: vec![], pixels: false },
        Candidate {
            label: "Segment mit Pop-up-Menü",
            prep: vec![("select.set", json!({ "ids": f.slice })), ("modify.popUpMenu.addPopUpMenu", json!({ "items": [{ "text": "Eins", "url": "eins.htm" }] }))],
            ids: f.slice.clone(),
            pixels: false,
        },
        Candidate { label: "zwei Ebenen", prep: vec![("layer.add", json!({}))], ids: vec![], pixels: false },
        Candidate { label: "Masterseite", prep: vec![("pi.page.setMaster", json!({ "index": 0 })), ("page.setCurrent", json!({ "index": 1 }))], ids: vec![], pixels: false },
        Candidate { label: "importiertes Symbol", prep: vec![("panel.library.import", json!({ "path": f.file.to_string_lossy() }))], ids: vec![], pixels: false },
    ]
}

pub fn apply_candidate(st: &mut AppState, cand: &Candidate) {
    for (n, p) in &cand.prep {
        let _ = st.session.execute(n, p.clone());
    }
    if cand.label != "Text auf Pfad" {
        select(st, &cand.ids);
    }
    if cand.pixels {
        let _ = st.session.execute("select.pixels", json!({ "shape": "rect", "rect": [440, 40, 520, 120], "mode": "replace" }));
        if !cand.ids.is_empty() {
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

pub fn candidate_by_label(f: &Fix, label: &str) -> Option<Candidate> {
    candidates(f).into_iter().find(|c| c.label == label)
}

pub fn available(st: &AppState, name: &str) -> bool {
    match fp_ui::actions::ui_command_state(st, name) {
        Some((en, _)) => en,
        None => st.session.is_available(name),
    }
}

/// Befehle, die im Prüflauf nie ausgeführt werden (Fremdprogramme, Beenden, Fensteraktionen).
pub fn skip_command(name: &str) -> bool {
    let n = name.to_lowercase();
    n.contains("browser") || n.contains("print") || n.contains("bridge") || n.contains("twain") || n.contains("scan") || n.contains("mail") || n.contains("extension") || n.contains("exchange") || n == "file.exit" || n == "file.quit" || n.contains("help.") && n != "help.about" || n.contains("preview") && n.contains("browser") || n.contains("openfolder")
}

// ------------------------------------------------------------------ Harness

pub fn harness_for(st: AppState, cfg: Cfg, wgpu: bool) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, fp_engine::Session::default(), UiSettings::default(), false);
    app.state = st;
    let b = Harness::builder().renderer(crate::gpu_common::renderer()).with_size(egui::vec2(cfg.w / cfg.ppp, cfg.h / cfg.ppp)).with_pixels_per_point(cfg.ppp).with_max_steps(40).with_step_dt(1.0 / 60.0);
    let b = if wgpu { b.renderer(crate::gpu_common::renderer()) } else { b };
    let mut h = b.build_ui_state(
        |ui, app: &mut FirepixelApp| {
            app.root_ui(ui);
            capture_texts(ui.ctx());
        },
        app,
    );
    h.run_steps(4);
    h
}

pub fn st(h: &H) -> &AppState {
    &h.state().state
}

pub fn stm(h: &mut H) -> &mut AppState {
    &mut h.state_mut().state
}

pub fn widget_rect(h: &H, id: Id) -> Option<Rect> {
    h.ctx.viewport(|v| v.prev_pass.widgets.get(id).map(|w| w.interact_rect))
}

pub fn hover(h: &mut H, p: Pos2) {
    h.event(Event::PointerMoved(p));
    h.run_steps(3);
}

pub fn click(h: &mut H, p: Pos2, button: PointerButton) {
    h.event(Event::PointerMoved(p));
    h.run_steps(1);
    h.event(Event::PointerButton { pos: p, button, pressed: true, modifiers: Modifiers::NONE });
    h.run_steps(1);
    h.event(Event::PointerButton { pos: p, button, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(4);
}

// ------------------------------------------------------------------ Texte und Prüfungen

/// Ein gezeichneter Text mit seiner Ebene (Fenster, Popup, Hintergrund).
#[derive(Clone, Debug)]
pub struct TextItem {
    pub text: String,
    pub rect: Rect,
    pub clip: Rect,
    pub layer: LayerId,
}

fn walk_shape(s: &Shape, clip: Rect, layer: LayerId, out: &mut Vec<TextItem>) {
    match s {
        Shape::Text(t) => {
            let text = t.galley.text().to_string();
            if text.trim().is_empty() || t.opacity_factor <= 0.01 {
                return;
            }
            out.push(TextItem { text, rect: t.visual_bounding_rect(), clip, layer });
        }
        Shape::Vec(v) => v.iter().for_each(|x| walk_shape(x, clip, layer, out)),
        _ => {}
    }
}

const TEXTS_ID: &str = "i18n-qa-texts";

/// Am Ende jedes Bildes (vor dem Abschluss des Durchgangs): alle Texte je Ebene merken.
pub fn capture_texts(ctx: &egui::Context) {
    let mut layers: Vec<LayerId> = ctx.memory(|m| m.layer_ids().collect());
    let extra: Vec<LayerId> = ctx.viewport(|v| v.this_pass.widgets.layers().map(|(l, _)| *l).chain(v.prev_pass.widgets.layers().map(|(l, _)| *l)).collect());
    for l in std::iter::once(LayerId::background()).chain(extra) {
        if !layers.contains(&l) {
            layers.insert(0, l);
        }
    }
    let mut out = Vec::new();
    ctx.graphics(|g| {
        for l in &layers {
            if let Some(pl) = g.get(*l) {
                for cs in pl.all_entries() {
                    walk_shape(&cs.shape, cs.clip_rect, *l, &mut out);
                }
            }
        }
    });
    ctx.data_mut(|d| d.insert_temp(Id::new(TEXTS_ID), std::sync::Arc::new(out)));
}

/// Texte des letzten Bildes (in Zeichenreihenfolge der Ebenen).
pub fn texts(h: &H) -> Vec<TextItem> {
    h.ctx.data(|d| d.get_temp::<std::sync::Arc<Vec<TextItem>>>(Id::new(TEXTS_ID))).map(|a| (*a).clone()).unwrap_or_default()
}

/// Bereichspräfixe der Schlüssel (`crates\fp-i18n\README.md`).
pub const KEY_PREFIXES: &[&str] = &["language", "btn", "fmt", "unit", "key", "test", "menu", "cmd", "ctx", "pmenu", "tool", "shape", "dlg", "pi", "panel", "ui", "err", "msg", "ai", "filter", "effect", "blend", "lang", "welcome"];

/// Rohschlüssel im Text (z. B. `dlg-canvas-size`, `cmd-file-open.tooltip`).
pub fn raw_keys(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for tok in text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '_')) {
        let tok = tok.trim_matches(|c: char| c == '.' || c == '-');
        let Some((pre, rest)) = tok.split_once('-') else { continue };
        if !KEY_PREFIXES.contains(&pre) || rest.is_empty() {
            continue;
        }
        let ok = tok.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.' || c == '_');
        if ok && tok.chars().any(|c| c.is_ascii_lowercase()) {
            out.push(tok.to_string());
        }
    }
    out
}

/// Eigennamen, Formatnamen, Einheiten, Symbole (dürfen in `qps` unmarkiert sein).
pub const PROPER: &[&str] = &[
    "Firepixel", "FW-PNG", "PNG", "PNG8", "PNG24", "PNG32", "GIF", "JPEG", "JPG", "WebP", "AVIF", "BMP", "TIFF", "TIF", "PSD", "PDF", "SVG", "ICO", "WBMP", "PICT", "JSF", "HTML", "CSS", "XML", "SWF", "AI", "EPS", "RGB", "RGBA", "HSB", "HSL",
    "CMYK", "HEX", "px", "pt", "dpi", "ppi", "K", "M", "MB", "KB", "GB", "kB", "Bit", "Bits", "fps", "ms", "s", "x", "X", "Y", "W", "H", "B", "R", "G", "A", "Arial", "Tahoma", "Verdana", "Segoe", "UI", "Times", "New", "Roman", "Courier", "Sans", "Serif", "Regular",
    "Bold", "Italic", "Normal", "Mono", "Consolas", "Calibri", "Cambria", "Georgia", "Impact", "Symbol", "Wingdings", "Webdings", "Marlett", "MCP", "SAM", "BiRefNet", "ONNX", "URL", "OK", "mm", "cm", "Myriad", "Pro", "Apache-2", "Hiera", "Tiny", "Small", "lite", "Matting", "HR", "jQuery", "Twain", "WebSnap", "Lossless",
];

/// Schriftfamilien und -stile des Systems sowie die Eigennamen aller Sprachen.
pub fn proper_names() -> &'static BTreeSet<String> {
    static N: std::sync::OnceLock<BTreeSet<String>> = std::sync::OnceLock::new();
    N.get_or_init(|| {
        let mut s = BTreeSet::new();
        let mut fams = fp_ui::inspector::fonts::families();
        for _ in 0..200 {
            if !fams.is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
            fams = fp_ui::inspector::fonts::families();
        }
        for f in fams.iter() {
            s.insert(f.name.clone());
            for (st, _) in &f.styles {
                s.insert(st.clone());
            }
        }
        // Sprachnamen stehen in ihrer eigenen Sprache (nie übersetzt)
        for l in fp_i18n::languages() {
            s.insert(l.name);
        }
        s
    })
}

/// Darf ein Text in der Testsprache unmarkiert sein?
pub fn exempt_unmarked(text: &str) -> bool {
    let t = text.trim();
    if !t.chars().any(|c| c.is_alphabetic()) {
        return true;
    }
    if FIXTURE_TEXTS.iter().any(|f| t == *f || (f.len() > 9 && t.contains(f))) {
        return true;
    }
    // Schriftnamen (Systemschriften, Stile) und Eigennamen der Sprachen
    if proper_names().contains(t) {
        return true;
    }
    // Pfade, Dateinamen, Farben
    if t.contains(":\\") || t.contains('/') && !t.contains(' ') || t.ends_with(".png") || t.ends_with(".jsf") || t.starts_with('#') && t.len() <= 9 {
        return true;
    }
    // nur aus Eigennamen/Einheiten/Zahlen/Symbolen, Kürzeln (≤ 3 Großbuchstaben, F-Tasten,
    // Papierformate), Sprachcodes (de_DE)
    let short = |w: &str| (w.chars().count() <= 3 && w.chars().all(|c| c.is_uppercase() || c.is_ascii_digit())) || (w.starts_with('F') && w.len() <= 3 && w[1..].chars().all(|c| c.is_ascii_digit()));
    t.split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_').filter(|w| !w.is_empty()).all(|w| {
        w.chars().all(|c| c.is_ascii_digit()) || PROPER.contains(&w) || short(w) || w.chars().count() == 1 || w.len() == 5 && w.as_bytes()[2] == b'_' || w.chars().all(|c| c.is_ascii_hexdigit()) && w.len() >= 6
    })
}

pub fn marked(text: &str) -> bool {
    text.contains("[!!") || text.contains("!!]")
}

/// Arten von Befunden.
pub const KINDS: &[(&str, &str)] = &[
    ("rohschluessel", "Rohschlüssel sichtbar"),
    ("unmarkiert", "unmarkierter Text (qps)"),
    ("abgeschnitten", "Text waagerecht abgeschnitten (ohne „…“)"),
    ("fensterrand", "Text über den Fensterrand"),
    ("ueberlappung", "Texte überlappen"),
    ("in_element", "Text ragt in ein fremdes Bedienelement"),
    ("element_rand", "Bedienelement über den Fensterrand"),
];

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Finding {
    pub kind: String,
    pub scene: String,
    pub cfg: String,
    pub text: String,
    pub detail: String,
}

fn r(x: Rect) -> String {
    format!("[{:.0},{:.0}–{:.0},{:.0}]", x.min.x, x.min.y, x.max.x, x.max.y)
}

/// Ebene, zu der ein Text gehört: oberstes Fenster/Popup, dessen Fläche den Clip-Bereich enthält.
fn layer_of(h: &H, clip: Rect, screen: Rect) -> Option<LayerId> {
    h.ctx.memory(|m| {
        let mut best = None;
        for l in m.layer_ids() {
            let area = m.area_rect(l.id).unwrap_or(screen);
            if area.expand(3.0).contains_rect(clip.intersect(screen)) {
                best = Some(l);
            }
        }
        best
    })
}

/// Prüft das aktuelle Bild; `text_checks` = (a)/(b), `layout_checks` = (c).
pub fn check(h: &H, lang: &str, scene: &str, cfg: &Cfg, items: &[TextItem], text_checks: bool, layout_checks: bool) -> Vec<Finding> {
    let mut out = Vec::new();
    let cl = cfg.label();
    let mut push = |kind: &str, text: &str, detail: String| out.push(Finding { kind: kind.into(), scene: scene.into(), cfg: cl.clone(), text: text.into(), detail });
    if text_checks {
        let mut seen = BTreeSet::new();
        for t in items {
            if !seen.insert(t.text.clone()) {
                continue;
            }
            let keys = raw_keys(&t.text);
            if !keys.is_empty() {
                push("rohschluessel", &t.text, keys.join(", "));
            }
            if lang == fp_i18n::PSEUDO_LANG && !marked(&t.text) && !exempt_unmarked(&t.text) {
                push("unmarkiert", &t.text, r(t.rect));
            }
        }
    }
    if !layout_checks {
        return out;
    }
    let screen = h.ctx.content_rect();
    // sichtbarer Teil je Text; verdeckte Texte (unter einem Popup/Fenster) zählen nicht
    let order_of = |l: &LayerId| -> usize { h.ctx.memory(|m| m.layer_ids().position(|x| x == *l).unwrap_or(0)) + l.order as usize * 10_000 };
    let occluded: Vec<bool> = items
        .iter()
        .map(|t| match h.ctx.layer_id_at(t.rect.center()) {
            Some(top) => top != t.layer && order_of(&top) > order_of(&t.layer),
            None => false,
        })
        .collect();
    let vis: Vec<Rect> = items.iter().enumerate().map(|(i, t)| if occluded[i] { Rect::NOTHING } else { t.rect.intersect(t.clip).intersect(screen) }).collect();
    for (i, t) in items.iter().enumerate() {
        let v = vis[i];
        if !v.is_positive() || v.height() < 0.5 * t.rect.height() {
            continue;
        }
        // Pfade in Eingabefeldern dürfen abgeschnitten sein (Feld rollt)
        let ell = t.text.trim_end().ends_with('…') || t.text.trim_end().ends_with("...") || t.text.contains(":\\");
        let cut_x = t.rect.min.x < t.clip.min.x - 1.0 || t.rect.max.x > t.clip.max.x + 1.0;
        let at_screen = t.rect.min.x < screen.min.x - 0.5 || t.rect.max.x > screen.max.x + 0.5 || t.rect.min.y < screen.min.y - 0.5 || t.rect.max.y > screen.max.y + 0.5;
        if at_screen {
            push("fensterrand", &t.text, format!("{} Fenster {}", r(t.rect), r(screen)));
        } else if cut_x && !ell && t.rect.width() - v.width() > 2.0 {
            push("abgeschnitten", &t.text, format!("{} sichtbar {}", r(t.rect), r(v)));
        }
    }
    // Text über Text (gleicher Clip-Bereich, kein Duplikat)
    let shrink = |x: Rect| Rect::from_min_max(x.min + egui::vec2(1.0, 2.0), x.max - egui::vec2(1.0, 2.0));
    let mut pairs = BTreeSet::new();
    for i in 0..items.len() {
        for j in (i + 1)..items.len() {
            let (a, b) = (&items[i], &items[j]);
            if a.layer != b.layer {
                continue;
            }
            let x = shrink(vis[i]).intersect(shrink(vis[j]));
            if !x.is_positive() || x.width() < 2.0 || x.height() < 3.0 {
                continue;
            }
            if a.text == b.text && (a.rect.center() - b.rect.center()).length() < 4.0 {
                continue;
            }
            let key = (a.text.clone(), b.text.clone());
            if pairs.insert(key) {
                push("ueberlappung", &format!("{} ⟷ {}", a.text, b.text), format!("{} × {}", r(a.rect), r(b.rect)));
            }
        }
    }
    // Text ragt teilweise in ein fremdes Bedienelement derselben Ebene
    let widgets: Vec<(LayerId, Rect)> = h.ctx.viewport(|vp| {
        let mut v = Vec::new();
        for (layer, rects) in vp.prev_pass.widgets.layers() {
            for w in rects {
                if w.enabled && (w.sense.contains(Sense::click()) || w.sense.contains(Sense::drag()) || w.sense.is_focusable()) && w.interact_rect.is_positive() {
                    v.push((*layer, w.interact_rect));
                }
            }
        }
        v
    });
    let mut seen = BTreeSet::new();
    for (i, t) in items.iter().enumerate() {
        let v = shrink(vis[i]);
        if !v.is_positive() {
            continue;
        }
        let tl = t.layer;
        for (wl, w) in &widgets {
            if *wl != tl || w.area() > 0.25 * screen.area() {
                continue;
            }
            let x = v.intersect(*w);
            if !x.is_positive() || x.width() < 4.0 || x.height() < 4.0 {
                continue;
            }
            // eigener Text im Element oder Element im Text (z. B. Symbolknopf in einer Zeile)
            if w.expand(1.5).contains_rect(v) || v.expand(1.5).contains_rect(*w) {
                continue;
            }
            // Text, der das Element nur am Rand berührt (Beschriftung neben dem Feld)
            if x.width() < 6.0 || x.area() < 0.15 * v.area().min(w.area()) {
                continue;
            }
            if seen.insert(t.text.clone()) {
                push("in_element", &t.text, format!("Text {} Element {}", r(t.rect), r(*w)));
            }
        }
    }
    // Bedienelemente über den Fensterrand
    let outside: Vec<Rect> = widgets.iter().filter(|(_, w)| !screen.expand(1.0).contains_rect(*w)).map(|(_, w)| *w).collect();
    if !outside.is_empty() {
        push("element_rand", "", format!("{} Elemente, z. B. {}", outside.len(), r(outside[0])));
    }
    out
}

/// Zeile der Textliste (Deutsch unverändert): Text mit Lage.
pub fn dump_line(t: &TextItem) -> String {
    format!("{}\t{:.0},{:.0},{:.0},{:.0}", normalize_text(&t.text).replace('\n', "⏎"), t.rect.min.x, t.rect.min.y, t.rect.width(), t.rect.height())
}

/// Laufabhängige Teile (Temp-Ordner mit Prozessnummer) vereinheitlichen.
pub fn normalize_text(s: &str) -> String {
    match s.find("firepixel-i18n-qa\\") {
        Some(i) => {
            let rest = &s[i + "firepixel-i18n-qa\\".len()..];
            let tail = rest.find('\\').map(|j| &rest[j..]).unwrap_or("");
            format!("{}firepixel-i18n-qa\\X{}", &s[..i], tail)
        }
        None => s.to_string(),
    }
}

/// Neue Einträge, die Phase 6 bewusst hinzufügt (Ansicht ▸ Sprache): im Vergleich nicht als Abweichung zählen.
pub const EXPECTED_NEW: &[&str] = &["Sprache"];

// ------------------------------------------------------------------ Bilder (Szenen)

/// Ein Prüfbild: ID (sprachunabhängig), Beschreibung, Vorbereitung des Zustands, Bedienung.
pub struct Scene {
    pub id: String,
    pub label: String,
    pub prep: PrepFn,
    pub ui: UiFn,
}

/// State preparation of a [`Scene`].
pub type PrepFn = Box<dyn Fn(&mut AppState, &Fix) + Send + Sync>;
/// UI interaction of a [`Scene`].
pub type UiFn = Box<dyn Fn(&mut H, &Fix) -> Result<(), String> + Send + Sync>;

pub fn scene(id: impl Into<String>, label: impl Into<String>, prep: impl Fn(&mut AppState, &Fix) + Send + Sync + 'static, ui: impl Fn(&mut H, &Fix) -> Result<(), String> + Send + Sync + 'static) -> Scene {
    Scene { id: id.into(), label: label.into(), prep: Box::new(prep), ui: Box::new(ui) }
}

fn no_ui(_: &mut H, _: &Fix) -> Result<(), String> {
    Ok(())
}

/// Hauptfenster in allen Designs.
pub fn scenes_main() -> Vec<Scene> {
    let mut v = Vec::new();
    for k in ThemeKind::ALL {
        let name = format!("{k:?}").to_lowercase();
        v.push(scene(format!("haupt:{name}"), format!("Hauptfenster, Design {name}"), move |st, _| st.set_theme(k), no_ui));
        v.push(scene(
            format!("haupt:{name}:auswahl"),
            format!("Hauptfenster mit Auswahl, Design {name}"),
            move |st, f| {
                st.set_theme(k);
                select(st, &f.rect);
            },
            no_ui,
        ));
    }
    v.push(scene(
        "haupt:leer",
        "Hauptfenster ohne Dokument",
        |st, _| {
            while st.session.active_id().is_some() {
                let _ = st.session.execute("file.close", json!({ "discard": true }));
            }
        },
        no_ui,
    ));
    v.push(scene(
        "haupt:symbolmodus",
        "Bedienfelder als Symbole mit Namen",
        |st, _| {
            st.settings.workspace.dock_mode = fp_ui::workspace::DockMode::IconicWithNames;
        },
        no_ui,
    ));
    v
}

/// Statuszeile: Zeigerposition über der Leinwand, Meldung nach einem Befehl, Werkzeughinweise.
pub fn scenes_status() -> Vec<Scene> {
    let mut v = Vec::new();
    v.push(scene("status:zeiger", "Statuszeile, Zeiger über der Leinwand", |_, _| {}, |h, _| {
        let c = canvas_rect(h).ok_or("keine Leinwand")?;
        hover(h, c.center());
        Ok(())
    }));
    for (id, cmd, cand) in [("status:gruppieren", "modify.group", "Rechteck"), ("status:einfuegen", "edit.paste", "ohne Auswahl"), ("status:auswahl", "select.all", "ohne Auswahl")] {
        v.push(scene(id, format!("Statuszeile nach {cmd}"), move |st, f| {
            if let Some(c) = candidate_by_label(f, cand) {
                apply_candidate(st, &c);
            }
            st.run(cmd, json!({}));
        }, no_ui));
    }
    for (id, tool) in [("status:stift", fp_engine::ToolId::Pen), ("status:zauberstab", fp_engine::ToolId::MagicWand), ("status:text", fp_engine::ToolId::Text)] {
        v.push(scene(id, format!("Statuszeile mit Werkzeug {tool:?} über der Leinwand"), move |st, _| st.push(Action::SetTool(tool)), |h, _| {
            let c = canvas_rect(h).ok_or("keine Leinwand")?;
            hover(h, c.center());
            Ok(())
        }));
    }
    v
}

pub fn canvas_rect(h: &H) -> Option<Rect> {
    let id = st(h).session.active_id()?;
    widget_rect(h, Id::new(("canvas", id.0)))
}

/// Pfade aller Untermenüs (Indizes in `build_main_menu`), Menü selbst = `[i]`.
pub fn menu_paths(st: &AppState) -> Vec<(Vec<usize>, String)> {
    fn rec(nodes: &[Node], path: &mut Vec<usize>, label: &str, out: &mut Vec<(Vec<usize>, String)>, depth: usize) {
        for (k, n) in nodes.iter().enumerate() {
            if let Node::Sub { label: l, children, enabled, .. } = n {
                if !*enabled || children.is_empty() {
                    continue;
                }
                path.push(k);
                let lab = format!("{label} ▸ {l}");
                out.push((path.clone(), lab.clone()));
                if depth < 3 {
                    rec(children, path, &lab, out, depth + 1);
                }
                path.pop();
            }
        }
    }
    let mut out = Vec::new();
    for (i, (title, nodes)) in fp_ui::menu::build_main_menu(st).iter().enumerate() {
        out.push((vec![i], title.clone()));
        let mut p = vec![i];
        rec(nodes, &mut p, title, &mut out, 1);
    }
    out
}

/// Öffnet Hauptmenü `path[0]` und fährt nacheinander über die Untermenü-Einträge.
pub fn open_menu_path(h: &mut H, path: &[usize]) -> Result<(), String> {
    let menus = fp_ui::menu::build_main_menu(st(h));
    let (title, mut nodes): (String, Vec<Node>) = menus.get(path[0]).cloned().ok_or("Menü fehlt")?;
    let first_label = |nodes: &[Node]| -> Option<String> {
        nodes.iter().find_map(|n| match n {
            Node::Item { label, .. } | Node::Sub { label, .. } => Some(label.clone()),
            Node::Separator => None,
        })
    };
    let want0 = first_label(&nodes);
    egui::Popup::open_id(&h.ctx, Id::new(("fp-main-menu", path[0])));
    h.run_steps(3);
    let shown = |h: &H, w: &Option<String>| w.as_ref().is_none_or(|w| texts(h).iter().any(|t| &t.text == w));
    if !shown(h, &want0) {
        // schmales Fenster: das Menü steht im Überlaufmenü „»“ (Popup mit kleinerem Index)
        egui::Popup::close_id(&h.ctx, Id::new(("fp-main-menu", path[0])));
        h.run_steps(1);
        let mut ok = false;
        let mut seen_title = Vec::new();
        // Index des Überlaufmenüs „»“ = Anzahl der Menüs, die in die Leiste passen (≤ path[0])
        for j in (0..=path[0]).rev() {
            egui::Popup::open_id(&h.ctx, Id::new(("fp-main-menu", j)));
            h.run_steps(3);
            let ts = texts(h);
            if let Some(t) = ts.iter().rev().find(|t| t.text == title && t.rect.top() > 30.0) {
                seen_title.push(format!("j{j}@{:.0},{:.0}", t.rect.center().x, t.rect.center().y));
                let p = t.rect.center();
                hover(h, p);
                h.run_steps(2);
                if shown(h, &want0) {
                    ok = true;
                    break;
                }
            }
            egui::Popup::close_id(&h.ctx, Id::new(("fp-main-menu", j)));
            h.run_steps(1);
        }
        if !ok {
            return Err(format!("Menü „{title}“ öffnet nicht (auch nicht über „»“; Titel im Überlauf: {seen_title:?}, erster Eintrag {want0:?})"));
        }
    }
    for &k in &path[1..] {
        let Some(Node::Sub { label, children, .. }) = nodes.get(k).cloned() else { return Err(format!("Untermenü {k} fehlt")) };
        // Eintrag im zuletzt gezeichneten Popup suchen (letztes Vorkommen in Zeichenreihenfolge)
        let ts = texts(h);
        let t = ts.iter().rev().find(|t| t.text == label).ok_or_else(|| format!("Eintrag „{label}“ nicht gezeichnet"))?;
        let p = t.rect.center();
        hover(h, p);
        h.run_steps(3);
        nodes = children;
        // erster Eintrag des Untermenüs sichtbar?
        let want: Option<String> = nodes.iter().find_map(|n| match n {
            Node::Item { label, .. } | Node::Sub { label, .. } => Some(label.clone()),
            Node::Separator => None,
        });
        if let Some(w) = want
            && !texts(h).iter().any(|t| t.text == w)
        {
            return Err(format!("Untermenü „{label}“ öffnet nicht"));
        }
    }
    Ok(())
}

/// Auswahl-Situationen, in denen die Menüs geöffnet werden (damit Untermenüs aktiv sind).
pub const MENU_STATES: &[&str] = &["Rechteck + Ellipse", "Text", "Bitmap", "Segment", "Pixelauswahl", "ohne Auswahl"];

pub fn scenes_menus(_st0: &AppState) -> Vec<Scene> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for cand in MENU_STATES {
        let (mut st, ctx, f) = fixture();
        if let Some(c) = candidate_by_label(&f, cand) {
            apply_candidate(&mut st, &c);
        }
        st.process_actions(&ctx);
        for (path, label) in menu_paths(&st) {
            let id = format!("menu:{}", path.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("/"));
            if !seen.insert(id.clone()) {
                continue;
            }
            let p2 = path.clone();
            let cand: &'static str = cand;
            out.push(scene(
                id,
                format!("{label} ({cand})"),
                move |st, f| {
                    if let Some(c) = candidate_by_label(f, cand) {
                        apply_candidate(st, &c);
                    }
                },
                move |h, _| open_menu_path(h, &p2),
            ));
        }
    }
    out
}

/// Alle Einträge des Menübaums (auch gesperrte und nicht geöffnete Untermenüs), der
/// Bedienfeldmenüs und der Leinwand-Kontextmenüs als Text: (a) Rohschlüssel, (b) unmarkiert.
pub fn menu_tree_findings(lang: &str) -> Vec<Finding> {
    fn rec(nodes: &[Node], path: &str, out: &mut Vec<(String, String)>) {
        for n in nodes {
            match n {
                Node::Separator => {}
                Node::Item { label, shortcut, .. } => {
                    out.push((path.to_string(), label.clone()));
                    if let Some(s) = shortcut {
                        out.push((format!("{path} ▸ {label} (Kürzel)"), s.clone()));
                    }
                }
                Node::Sub { label, children, .. } => {
                    out.push((path.to_string(), label.clone()));
                    rec(children, &format!("{path} ▸ {label}"), out);
                }
            }
        }
    }
    let mut entries = Vec::new();
    for cand in MENU_STATES {
        let (mut st, ctx, f) = fixture();
        if let Some(c) = candidate_by_label(&f, cand) {
            apply_candidate(&mut st, &c);
        }
        st.process_actions(&ctx);
        for (title, nodes) in fp_ui::menu::build_main_menu(&st) {
            entries.push(("Menüleiste".to_string(), title.clone()));
            rec(&nodes, &title, &mut entries);
        }
        for p in PanelId::ALL {
            rec(&fp_ui::menu::build_panel_menu(&st, *p), &format!("Bedienfeldmenü {p:?}"), &mut entries);
        }
        for id in ["6906", "6910", "6912", "6913", "6914", "6915", "6916", "6925", "49120"] {
            rec(&fp_ui::canvas::context::build(&st, fp_ui::canvas::context::entries(id)), &format!("Kontextmenü {id}"), &mut entries);
        }
    }
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for (path, text) in entries {
        // Dokumentliste im Fenster-Menü (Nutzerdaten)
        if text.contains("Unbenannt") || text.contains(".png") || !seen.insert(text.clone()) {
            continue;
        }
        let keys = raw_keys(&text);
        if !keys.is_empty() {
            out.push(Finding { kind: "rohschluessel".into(), scene: format!("menübaum: {path}"), cfg: "Baum".into(), text: text.clone(), detail: keys.join(", ") });
        }
        if lang == fp_i18n::PSEUDO_LANG && !marked(&text) && !exempt_unmarked(&text) {
            out.push(Finding { kind: "unmarkiert".into(), scene: format!("menübaum: {path}"), cfg: "Baum".into(), text, detail: String::new() });
        }
    }
    out
}

/// Bedienfelder (vorn im Dock) und ihre Bedienfeldmenüs (≡).
pub fn scenes_panels() -> Vec<Scene> {
    let mut v = Vec::new();
    for p in PanelId::ALL.iter().copied() {
        let name = format!("{p:?}");
        let cand = match p {
            PanelId::SymbolProperties | PanelId::DocumentLibrary => "Grafiksymbol",
            PanelId::AutoShapeProperties => "Automatische Form",
            PanelId::Url | PanelId::Behaviors => "Segment",
            PanelId::Path => "Pfad",
            PanelId::ImageEditing => "Bitmap",
            _ => "Rechteck",
        };
        v.push(scene(format!("panel:{name}"), format!("Bedienfeld {name}"), move |st, f| {
            if let Some(c) = candidate_by_label(f, cand) {
                apply_candidate(st, &c);
            }
            st.push(Action::ShowPanel(p));
        }, no_ui));
        v.push(scene(format!("pmenu:{name}"), format!("Bedienfeldmenü {name}"), move |st, f| {
            if let Some(c) = candidate_by_label(f, cand) {
                apply_candidate(st, &c);
            }
            st.push(Action::ShowPanel(p));
        }, move |h, _| {
            h.run_steps(2);
            let loc = st(h).settings.workspace.locate(p);
            let id = match loc {
                Some(fp_ui::workspace::Location::Dock { group, .. }) => Id::new(("dock-group", group)).with("menu"),
                Some(fp_ui::workspace::Location::Floating { group, .. }) => {
                    let first = st(h).settings.workspace.floating.get(group).and_then(|f| f.group.panels.first().copied());
                    Id::new(("float", group, first)).with("g").with("menu")
                }
                // unten neben dem Inspektor: Menü des Inspektors (eigene Bilder unter „Inspektor“)
                _ => return Ok(()),
            };
            let r = widget_rect(h, id).ok_or("Menüknopf nicht gefunden")?;
            click(h, r.center(), PointerButton::Primary);
            Ok(())
        }));
    }
    v
}

/// Inspektor je Elementart (alle Auswahl-Situationen) und je Werkzeug.
pub fn scenes_inspector() -> Vec<Scene> {
    let mut v = Vec::new();
    let (_, _, f0) = fixture();
    for cand in candidates(&f0) {
        let label = cand.label;
        v.push(scene(format!("pi:{label}"), format!("Inspektor: {label}"), move |st, f| {
            if let Some(c) = candidate_by_label(f, label) {
                apply_candidate(st, &c);
            }
        }, no_ui));
    }
    for t in fp_engine::tools::TOOLS {
        let id = t.id;
        v.push(scene(format!("werkzeug:{}", t.name), format!("Werkzeug {}", t.name), move |st, _| {
            let _ = st.session.execute("select.set", json!({ "ids": [] }));
            st.push(Action::SetTool(id));
        }, no_ui));
    }
    v
}

/// Kontextmenüs: Leinwand je Zustand, Dokumentreiter, Ebenenzeile.
pub fn scenes_context() -> Vec<Scene> {
    use fp_engine::ToolId as T;
    let mut v = Vec::new();
    let cases: Vec<(&str, &str, Option<T>, bool)> = vec![
        ("nichts", "ohne Auswahl", None, false),
        ("objekt", "Rechteck", None, false),
        ("text", "Text", None, false),
        ("pixel", "Pixelauswahl", Some(T::Marquee), false),
        ("skalieren", "Rechteck", Some(T::Scale), false),
        ("zuschneiden", "ohne Auswahl", Some(T::Crop), false),
        ("zuschneiden_bitmap", "Bitmap", Some(T::Crop), true),
        ("zoom", "ohne Auswahl", Some(T::Zoom), false),
        ("pipette", "ohne Auswahl", Some(T::Eyedropper), false),
    ];
    for (id, cand, tool, bitmap_mode) in cases {
        v.push(scene(format!("ctx:leinwand:{id}"), format!("Kontextmenü Leinwand: {id}"), move |st, f| {
            if let Some(c) = candidate_by_label(f, cand) {
                apply_candidate(st, &c);
            }
            if bitmap_mode {
                let _ = st.session.execute("edit.editBitmap", json!({}));
            }
            if let Some(t) = tool {
                st.push(Action::SetTool(t));
            }
        }, |h, _| {
            let c = canvas_rect(h).ok_or("keine Leinwand")?;
            // leere Stelle links oben in der Leinwandfläche (neben der Seite)
            click(h, c.min + egui::vec2(12.0, 12.0), PointerButton::Secondary);
            Ok(())
        }));
    }
    v.push(scene("ctx:reiter", "Kontextmenü Dokumentreiter", |_, _| {}, |h, _| {
        let id = st(h).session.active_id().ok_or("kein Dokument")?;
        let r = widget_rect(h, Id::new(("doc-tab", id.0))).ok_or("Reiter nicht gefunden")?;
        click(h, r.center(), PointerButton::Secondary);
        Ok(())
    }));
    for (pid, what) in [(PanelId::Layers, "ebene"), (PanelId::Pages, "seite"), (PanelId::States, "status")] {
        v.push(scene(format!("ctx:{what}"), format!("Kontextmenü Zeile im Bedienfeld {pid:?}"), move |st, _| st.push(Action::ShowPanel(pid)), move |h, _| {
            h.run_steps(2);
            let name = {
                let d = st(h).session.active().ok_or("kein Dokument")?;
                match pid {
                    PanelId::Layers => fp_engine::docops::own_layers(&d.doc, d.page).into_iter().map(|(_, l)| l.name.clone()).next(),
                    PanelId::Pages => d.doc.pages.first().and_then(|p| p.name.clone()),
                    _ => None,
                }
            };
            let ts = texts(h);
            let dock_x = ts.iter().map(|t| t.rect.center().x).fold(0.0f32, f32::max) - 400.0;
            let t = match name {
                Some(n) => ts.iter().filter(|t| t.text == n).max_by(|a, b| a.rect.left().total_cmp(&b.rect.left())).cloned(),
                // Status: erste Zeile mit Ziffer im Dock
                None => ts.iter().filter(|t| t.rect.center().x > dock_x && t.text.trim().chars().next().is_some_and(|c| c.is_ascii_digit())).min_by(|a, b| a.rect.top().total_cmp(&b.rect.top())).cloned(),
            };
            let t = t.ok_or("Zeile nicht gefunden")?;
            click(h, t.rect.center(), PointerButton::Secondary);
            Ok(())
        }));
    }
    v
}

// ------------------------------------------------------------------ Dialoge

/// Bauart des offenen Dialogs (sprachunabhängig).
pub fn dialog_kind(st: &AppState, cmd: &str) -> Option<String> {
    let d = &st.dialogs;
    if let Some(f) = &d.form {
        return Some(format!("form:{}", f.id));
    }
    if let Some(p) = &d.pending_form {
        return Some(format!("pending:{p:?}"));
    }
    if d.message.is_some() {
        return Some(format!("meldung:{cmd}"));
    }
    if d.confirm.is_some() {
        return Some(format!("rueckfrage:{cmd}"));
    }
    let named: [(&str, bool); 20] = [
        ("neues_dokument", d.new_document.is_some()),
        ("ungespeichert", d.unsaved.is_some()),
        ("voreinstellungen", d.preferences.is_some()),
        ("ueber", d.about),
        ("arbeitsbereich_speichern", d.save_workspace.is_some()),
        ("leinwandgroesse", d.canvas_size.is_some()),
        ("bildgroesse", d.image_size.is_some()),
        ("numerisch", d.numeric_transform.is_some()),
        ("tonwert", d.levels.is_some()),
        ("kurven", d.curves.is_some()),
        ("export", d.export.is_some()),
        ("bildvorschau", d.preview.is_some()),
        ("tastenkuerzel", d.shortcuts.is_some()),
        ("texteditor", d.text_editor.is_some()),
        ("html_segment", d.html_slice.is_some()),
        ("befehle_verwalten", d.saved_commands.is_some()),
        ("rechtschreibung", d.spelling.is_some()),
        ("popup_editor", d.popup_editor.is_some()),
        ("ki_fehlt", d.ai.missing.is_some()),
        ("ki_verwalten", d.ai.manager.is_some()),
    ];
    named.iter().find(|(_, open)| *open).map(|(n, _)| n.to_string())
}

/// Ein gefundener Dialog: Befehl, Auswahl-Situation, Bauart.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DialogCase {
    pub command: String,
    pub candidate: String,
    pub kind: String,
}

/// Alle Befehle aus Menüs, Bedienfeldmenüs und Leinwand-Kontextmenüs.
pub fn all_commands(st: &AppState) -> Vec<String> {
    fn rec(nodes: &[Node], out: &mut Vec<String>) {
        for n in nodes {
            match n {
                Node::Item { action: ItemAction::Command(c), .. } => out.push(c.clone()),
                Node::Item { action: ItemAction::Ui(Action::Run(c, _)), .. } => out.push(c.clone()),
                Node::Sub { children, .. } => rec(children, out),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    for (_, nodes) in fp_ui::menu::build_main_menu(st) {
        rec(&nodes, &mut out);
    }
    for p in PanelId::ALL {
        rec(&fp_ui::menu::build_panel_menu(st, *p), &mut out);
    }
    for id in ["6906", "6910", "6912", "6913", "6914", "6915", "6916", "6925", "49120"] {
        rec(&fp_ui::canvas::context::build(st, fp_ui::canvas::context::entries(id)), &mut out);
    }
    let mut seen = BTreeSet::new();
    out.retain(|c| seen.insert(c.clone()) && !skip_command(c));
    out
}

/// Sucht zu jedem Befehl einen Dialog (zwischengespeichert in `dialoge.json`; neu mit `FP_QA_NEW=1`, alt `FP_QA_NEU`).
pub fn discover_dialogs() -> Vec<DialogCase> {
    let cache = out_root().join("dialoge.json");
    if env_var("FP_QA_NEW", "FP_QA_NEU").is_none()
        && let Ok(s) = std::fs::read_to_string(&cache)
        && let Ok(v) = serde_json::from_str::<Vec<DialogCase>>(&s)
    {
        return v;
    }
    let (st0, _, f0) = fixture();
    let cmds = all_commands(&st0);
    drop(st0);
    let cands = candidates(&f0);
    // Verfügbarkeit je Situation einmal bestimmen
    let mut avail: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (ci, cand) in cands.iter().enumerate() {
        let (mut st, _, _) = fixture();
        apply_candidate(&mut st, cand);
        for cmd in &cmds {
            if available(&st, cmd) {
                avail.entry(cmd.clone()).or_default().push(ci);
            }
        }
    }
    let mut found: Vec<DialogCase> = Vec::new();
    let mut kinds = BTreeSet::new();
    for cmd in cmds {
        let mut tries = 0;
        for &ci in avail.get(&cmd).map(|v| v.as_slice()).unwrap_or(&[]) {
            let cand = &cands[ci];
            let (mut st, ctx, _) = fixture();
            apply_candidate(&mut st, cand);
            tries += 1;
            let c2 = cmd.clone();
            let res = std::panic::catch_unwind(AssertUnwindSafe(move || {
                st.run(&c2, json!({}));
                st.process_actions(&ctx);
                st
            }));
            let Ok(st) = res else { break };
            if let Some(k) = dialog_kind(&st, &cmd) {
                if kinds.insert(k.clone()) {
                    found.push(DialogCase { command: cmd.clone(), candidate: cand.label.into(), kind: k });
                }
                break;
            }
            if tries >= 3 {
                break;
            }
        }
    }
    // Dialoge ohne eigenen Menübefehl
    for (cmd, kind) in [("ui.unsaved", "ungespeichert"), ("ui.saveWorkspace", "arbeitsbereich_speichern")] {
        if kinds.insert(kind.into()) {
            found.push(DialogCase { command: cmd.into(), candidate: "gespeichert + geändert".into(), kind: kind.into() });
        }
    }
    let _ = std::fs::create_dir_all(out_root());
    let _ = std::fs::write(&cache, serde_json::to_string_pretty(&found).unwrap_or_default());
    found
}

pub fn scenes_dialogs() -> Vec<Scene> {
    let mut v = Vec::new();
    for dc in discover_dialogs() {
        let DialogCase { command, candidate, kind } = dc.clone();
        let cmd2 = command.clone();
        v.push(scene(format!("dlg:{kind}"), format!("Dialog {kind} ({command})"), move |st, f| {
            if let Some(c) = candidate_by_label(f, &candidate) {
                apply_candidate(st, &c);
            }
            match cmd2.as_str() {
                "ui.unsaved" => {
                    if let Some(id) = st.session.active_id() {
                        st.dialogs.unsaved = Some(fp_ui::dialogs::UnsavedFlow::new(vec![id], fp_ui::dialogs::PendingClose::CloseDocs));
                    }
                }
                "ui.saveWorkspace" => st.dialogs.save_workspace = Some("Mein Arbeitsbereich".into()),
                c => st.run(c, json!({})),
            }
        }, no_ui));
    }
    // Live-Filter-Dialoge (Inspektor „+“): jeder einstellbare Effekt einmal
    let mut fx_kinds = BTreeSet::new();
    for i in 0..80usize {
        let (mut st, _, f) = fixture();
        select(&mut st, &f.rect);
        fp_ui::dialogs::effects::begin_add(&mut st, i);
        let Some(form) = &st.dialogs.form else { continue };
        if !fx_kinds.insert(form.id.clone()) {
            continue;
        }
        v.push(scene(format!("fx:{}", form.id), format!("Live-Filter-Dialog Nr. {i}"), move |st, f| {
            select(st, &f.rect);
            fp_ui::dialogs::effects::begin_add(st, i);
        }, no_ui));
    }
    // Menü „+“ der Live-Filter im Inspektor, Untermenüs nacheinander
    for k in 0..9usize {
        v.push(scene(format!("fx:menue:{k}"), format!("Live-Filter-Menü „+“, Untermenü {k}"), |st, f| select(st, &f.rect), move |h, _| {
            let r = widget_rect(h, Id::new("pi-fx-add")).ok_or("Knopf „+“ nicht gefunden")?;
            let before: BTreeSet<String> = texts(h).into_iter().map(|t| t.text).collect();
            click(h, r.center(), PointerButton::Primary);
            if k == 0 {
                return Ok(());
            }
            let mut items: Vec<TextItem> = texts(h).into_iter().filter(|t| !before.contains(&t.text)).collect();
            items.sort_by(|a, b| a.rect.top().total_cmp(&b.rect.top()));
            if let Some(t) = items.get(k - 1) {
                hover(h, t.rect.center());
            }
            Ok(())
        }));
    }
    // Voreinstellungen: jede Kategorie
    for i in 0..7usize {
        v.push(scene(format!("dlg:voreinstellungen:{i}"), format!("Voreinstellungen, Kategorie {i}"), |st, _| st.run("edit.preferences", json!({})), move |h, _| {
            h.run_steps(2);
            let r = widget_rect(h, Id::new(("prefcat", i))).ok_or("Kategorie nicht gefunden")?;
            click(h, r.center(), PointerButton::Primary);
            Ok(())
        }));
    }
    v
}

// ------------------------------------------------------------------ Ablauf

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AreaResult {
    pub lang: String,
    pub area: String,
    pub mode: String,
    pub scenes: usize,
    pub configs: Vec<String>,
    /// Bilder, die sich nicht herstellen ließen (Szene: Grund).
    pub failed: Vec<String>,
    pub findings: Vec<Finding>,
    /// Textliste je Bild (nur Deutsch, `CFG_TEXT`).
    pub dump: BTreeMap<String, Vec<String>>,
    /// Abweichungen gegenüber dem Stand vor Phase 6 (nur Deutsch).
    pub de_changed: Vec<String>,
    pub seconds: f64,
}

/// Ein Bild herstellen: Testdokument, Vorbereitung, Harness, Bedienung.
pub fn build_scene(sc: &Scene, cfg: Cfg, wgpu: bool) -> Result<H, String> {
    let (mut st, ctx, f) = fixture();
    (sc.prep)(&mut st, &f);
    st.process_actions(&ctx);
    let mut h = std::panic::catch_unwind(AssertUnwindSafe(|| harness_for(st, cfg, wgpu))).map_err(|e| format!("PANIK {}", panic_msg(&e)))?;
    std::panic::catch_unwind(AssertUnwindSafe(|| (sc.ui)(&mut h, &f))).map_err(|e| format!("PANIK {}", panic_msg(&e)))??;
    std::panic::catch_unwind(AssertUnwindSafe(|| h.run_steps(3))).map_err(|e| format!("PANIK {}", panic_msg(&e)))?;
    Ok(h)
}

pub fn panic_msg(e: &Box<dyn std::any::Any + Send>) -> String {
    e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default()
}

/// Prüft alle Bilder eines Bereichs in einer Sprache und schreibt das Ergebnis.
pub fn run_area(lang: &str, area: &str, scenes: Vec<Scene>) -> AreaResult {
    run_area_with(lang, area, scenes, Vec::new())
}

/// Wie [`run_area`], mit zusätzlichen Befunden (z. B. aus dem Menübaum).
pub fn run_area_with(lang: &str, area: &str, scenes: Vec<Scene>, extra: Vec<Finding>) -> AreaResult {
    let t0 = std::time::Instant::now();
    let filter = only();
    let scenes: Vec<Scene> = scenes.into_iter().filter(|s| filter.as_ref().is_none_or(|f| s.id.contains(f.as_str()))).collect();
    let mut res = AreaResult { lang: lang.into(), area: area.into(), mode: mode(), scenes: scenes.len(), ..Default::default() };
    let layout_cfgs: Vec<Cfg> = if mode() == MODE_BASELINE { vec![] } else { CFG_LAYOUT.to_vec() };
    res.configs = std::iter::once(CFG_TEXT).chain(layout_cfgs.iter().copied()).map(|c| c.label()).collect();
    res.findings.extend(extra);
    for sc in &scenes {
        // Text-/Schlüsselprüfung und Textliste
        match build_scene(sc, CFG_TEXT, false) {
            Ok(h) => {
                let items = texts(&h);
                res.findings.extend(check(&h, lang, &sc.id, &CFG_TEXT, &items, true, false));
                if lang == "de" {
                    let mut lines: Vec<String> = items.iter().map(dump_line).collect();
                    lines.sort();
                    res.dump.insert(sc.id.clone(), lines);
                }
            }
            Err(e) => res.failed.push(format!("{} ({}): {e}", sc.id, CFG_TEXT.label())),
        }
        for cfg in &layout_cfgs {
            match build_scene(sc, *cfg, false) {
                Ok(h) => {
                    let items = texts(&h);
                    res.findings.extend(check(&h, lang, &sc.id, cfg, &items, false, true));
                }
                Err(e) => res.failed.push(format!("{} ({}): {e}", sc.id, cfg.label())),
            }
        }
    }
    if lang == "de" && mode() != MODE_BASELINE {
        res.de_changed = compare_with_basis(area, &res.dump);
    }
    res.seconds = t0.elapsed().as_secs_f64();
    let name = if filter.is_some() { format!("{lang}_{area}_teil.json") } else { format!("{lang}_{area}.json") };
    let _ = std::fs::write(out_dir().join(name), serde_json::to_string_pretty(&res).unwrap_or_default());
    eprintln!("{lang}/{area}: {} Bilder, {} Befunde, {} nicht herstellbar, {} Abweichungen Deutsch, {:.0} s", res.scenes, res.findings.len(), res.failed.len(), res.de_changed.len(), res.seconds);
    res
}

/// Bilder ausgewählter Szenen (wgpu) nach `docs\screenshots\<prefix><datei>.png`; mit `vergleich`
/// zusätzlich Pixelvergleich gegen vorhandene Bilder `<vergleich><datei>.png` (Anteil abweichender
/// Pixel je Bild). Ohne Grafikadapter leer.
pub fn render_images(scenes: &[Scene], pick: &[(&str, &str)], cfg: Cfg, prefix: &str, compare: Option<&str>) -> Vec<String> {
    let dir = repo().join("docs").join("screenshots");
    let mut report = Vec::new();
    for (id, file) in pick {
        let Some(sc) = scenes.iter().find(|s| s.id == *id) else {
            report.push(format!("{id}: Szene fehlt"));
            continue;
        };
        let mut h = match build_scene(sc, cfg, true) {
            Ok(h) => h,
            Err(e) => {
                report.push(format!("{id}: {e}"));
                continue;
            }
        };
        h.run_steps(4);
        let img = match h.render() {
            Ok(i) => i,
            Err(e) => {
                report.push(format!("kein Grafikadapter: {e}"));
                return report;
            }
        };
        let path = dir.join(format!("{prefix}{file}.png"));
        if let Some(cmp) = compare {
            match load_png(&dir.join(format!("{cmp}{file}.png"))) {
                Some((w, hh, old)) => {
                    if (w, hh) != (img.width(), img.height()) {
                        report.push(format!("{file}: Größe {w}×{hh} → {}×{}", img.width(), img.height()));
                    } else {
                        let n = old.as_chunks::<4>().0.iter().zip(img.as_raw().as_chunks::<4>().0.iter()).filter(|(a, b)| a.iter().zip(b.iter()).any(|(x, y)| x.abs_diff(*y) > 8)).count();
                        let total = (w * hh) as f64;
                        report.push(format!("{file}: {n} abweichende Pixel ({:.3} %)", 100.0 * n as f64 / total));
                    }
                }
                None => report.push(format!("{file}: kein Vergleichsbild")),
            }
        }
        let _ = img.save(&path);
    }
    report
}

/// Standardbilder: Hauptfenster je Design, ein Menü, ein Dialog, Inspektor (Rechteck), Normal und Dunkel.
pub fn images_standard(prefix: &str, compare: Option<&str>) -> Vec<String> {
    let mut all = scenes_main();
    let (st0, _, _) = fixture();
    all.extend(scenes_menus(&st0).into_iter().filter(|s| s.id == "menu:4" || s.id == "menu:0"));
    drop(st0);
    all.extend(scenes_dialogs().into_iter().filter(|s| s.id == "dlg:leinwandgroesse" || s.id == "dlg:neues_dokument" || s.id == "dlg:voreinstellungen:0"));
    all.extend(scenes_inspector().into_iter().filter(|s| s.id == "pi:Rechteck" || s.id == "pi:Text"));
    // Dunkel-Varianten von Dialog und Inspektor
    let dark = |id: &str, scenes: Vec<Scene>| -> Vec<Scene> {
        scenes
            .into_iter()
            .filter(|s| s.id == id)
            .map(|s| {
                let Scene { id, label, prep, ui } = s;
                Scene {
                    id: format!("{id}:dunkel"),
                    label: format!("{label} (Dunkel)"),
                    prep: Box::new(move |st: &mut AppState, f: &Fix| {
                        st.set_theme(ThemeKind::Dark);
                        prep(st, f)
                    }),
                    ui,
                }
            })
            .collect()
    };
    all.extend(dark("dlg:leinwandgroesse", scenes_dialogs()));
    all.extend(dark("pi:Rechteck", scenes_inspector()));
    let pick = [
        ("haupt:normal:auswahl", "hauptfenster_normal"),
        ("haupt:dark:auswahl", "hauptfenster_dunkel"),
        ("haupt:oled:auswahl", "hauptfenster_oled"),
        ("menu:0", "menue_datei"),
        ("menu:4", "menue_modifizieren"),
        ("dlg:leinwandgroesse", "dialog_leinwandgroesse_normal"),
        ("dlg:leinwandgroesse:dunkel", "dialog_leinwandgroesse_dunkel"),
        ("dlg:neues_dokument", "dialog_neues_dokument"),
        ("dlg:voreinstellungen:0", "dialog_voreinstellungen"),
        ("pi:Rechteck", "inspektor_rechteck_normal"),
        ("pi:Rechteck:dunkel", "inspektor_rechteck_dunkel"),
        ("pi:Text", "inspektor_text"),
    ];
    render_images(&all, &pick, Cfg { w: 1600.0, h: 1000.0, ppp: 1.0 }, prefix, compare)
}

/// Die Prüftests eines Sprachlaufs (je Bereich ein Test, parallel ausführbar).
#[macro_export]
macro_rules! qa_tests {
    ($lang:expr) => {
        #[test]
        #[ignore]
        fn hauptfenster() {
            i18n_qa_common::set_lang($lang);
            i18n_qa_common::run_area($lang, "hauptfenster", i18n_qa_common::scenes_main());
        }
        #[test]
        #[ignore]
        fn statuszeile() {
            i18n_qa_common::set_lang($lang);
            i18n_qa_common::run_area($lang, "statuszeile", i18n_qa_common::scenes_status());
        }
        #[test]
        #[ignore]
        fn menues() {
            i18n_qa_common::set_lang($lang);
            let (st, _, _) = i18n_qa_common::fixture();
            let sc = i18n_qa_common::scenes_menus(&st);
            drop(st);
            let tree = i18n_qa_common::menu_tree_findings($lang);
            i18n_qa_common::run_area_with($lang, "menues", sc, tree);
        }
        #[test]
        #[ignore]
        fn bedienfelder() {
            i18n_qa_common::set_lang($lang);
            i18n_qa_common::run_area($lang, "bedienfelder", i18n_qa_common::scenes_panels());
        }
        #[test]
        #[ignore]
        fn inspektor() {
            i18n_qa_common::set_lang($lang);
            i18n_qa_common::run_area($lang, "inspektor", i18n_qa_common::scenes_inspector());
        }
        #[test]
        #[ignore]
        fn dialoge() {
            i18n_qa_common::set_lang($lang);
            i18n_qa_common::run_area($lang, "dialoge", i18n_qa_common::scenes_dialogs());
        }
        #[test]
        #[ignore]
        fn kontextmenues() {
            i18n_qa_common::set_lang($lang);
            i18n_qa_common::run_area($lang, "kontextmenues", i18n_qa_common::scenes_context());
        }
    };
}

/// PNG als RGBA8 laden.
pub fn load_png(path: &std::path::Path) -> Option<(u32, u32, Vec<u8>)> {
    let dec = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).ok()?));
    let mut rd = dec.read_info().ok()?;
    let mut buf = vec![0; rd.output_buffer_size()?];
    let info = rd.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());
    match info.color_type {
        png::ColorType::Rgba if info.bit_depth == png::BitDepth::Eight => Some((info.width, info.height, buf)),
        png::ColorType::Rgb if info.bit_depth == png::BitDepth::Eight => Some((info.width, info.height, buf.as_chunks::<3>().0.iter().flat_map(|p| [p[0], p[1], p[2], 255]).collect())),
        _ => None,
    }
}

/// Vergleicht die deutsche Textliste mit dem Stand vor Phase 6.
pub fn compare_with_basis(area: &str, dump: &BTreeMap<String, Vec<String>>) -> Vec<String> {
    let path = mode_dir(MODE_BASELINE).join(format!("de_{area}.json"));
    let Ok(s) = std::fs::read_to_string(&path) else { return vec![format!("keine Basis {}", path.display())] };
    let Ok(base) = serde_json::from_str::<AreaResult>(&s) else { return vec!["Basis unlesbar".into()] };
    let mut out = Vec::new();
    for (id, lines) in dump {
        let Some(old) = base.dump.get(id) else {
            out.push(format!("{id}: neu (nicht in der Basis)"));
            continue;
        };
        // Texte mit Temp-Pfad: Breite hängt von der Prozessnummer ab → nur Text vergleichen
        let norm = |v: &Vec<String>| -> BTreeSet<String> { v.iter().map(|l| normalize_text(l)).map(|l| if l.contains("firepixel-i18n-qa\\X") { l.split('\t').next().unwrap_or("").to_string() } else { l }).filter(|l| !EXPECTED_NEW.iter().any(|e| l.split('\t').next() == Some(*e))).collect() };
        let a: BTreeSet<String> = norm(old);
        let b: BTreeSet<String> = norm(lines);
        if a == b {
            continue;
        }
        // je geänderte Zeile ein Eintrag („−“ vorher, „+“ jetzt), damit gleiche Änderungen über
        // viele Bilder im Bericht zusammengefasst werden können
        for g in a.difference(&b).take(10) {
            out.push(format!("{id}: − {g}"));
        }
        for n in b.difference(&a).take(10) {
            out.push(format!("{id}: + {n}"));
        }
    }
    for id in base.dump.keys() {
        if !dump.contains_key(id) && only().is_none() {
            out.push(format!("{id}: fehlt (war in der Basis)"));
        }
    }
    out
}

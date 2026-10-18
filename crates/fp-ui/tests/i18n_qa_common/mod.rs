//! Test harness for translatability (agent I18N-QA, phase 6, `docs\I18N_PRUEFUNG.md`).
//!
//! Walks the whole user interface offscreen (egui_kittest) in a chosen language – main window in
//! all themes, every menu and submenu, every panel with its panel menu, the inspector for every
//! element kind and every tool, every dialog reachable from the menus, the context menus and the
//! status bar – and checks per image:
//!
//! * (a) **raw keys**: visible text that looks like a key (`dlg-canvas-size`),
//! * (b) **unmarked text** in the test language `qps` (every text from `fp_i18n::t` carries `[!! … !!]`;
//!   without the marker it is hard-coded; proper names, format names, numbers and symbols are exempt),
//! * (c) **overflow**: text clipped horizontally (without "…"), text beyond the window edge, texts
//!   overlapping, text reaching into a foreign widget, widget beyond the window edge,
//! * (d) **German unchanged**: list of all drawn texts with their position per image against the
//!   state before phase 6 (commit `30ddaec`, produced with `FP_QA_MODE=baseline`).
//!
//! Environment variables (TE2-E; the old German names and values are still read):
//! `FP_QA_MODE` (`current`/`baseline`, old `FP_QA_MODUS` with `aktuell`/`basis`), `FP_QA_OUT`
//! (output folder, old `FP_QA_AUS`), `FP_QA_ONLY` (part of the image id, old `FP_QA_NUR`), `FP_QA_NEW`
//! (search the dialogs again, old `FP_QA_NEU`).
//!
//! Results as JSON in `target-i18n-qa\i18n-qa\<mode>\<language>_<area>.json`; the test
//! `i18n_qa_report` sums them up in `docs\I18N_PRUEFUNG.md`.
//!
//! Identifiers (BA-086): area names, scene ids, finding kinds, the dialog cache (`dialogs.json`)
//! and image names are English. Results written before (German names) stay readable:
//! [`load_result`] falls back to the old file names and translates old scene ids and finding kinds
//! ([`LEGACY_SEGMENTS`], [`LEGACY_KINDS`]).
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

// ------------------------------------------------------------------ Configuration

/// Window size (device pixels) and scale.
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

/// Size for the text list (German unchanged) and the text/key check.
pub const CFG_TEXT: Cfg = Cfg { w: 1600.0, h: 1000.0, ppp: 1.0 };
/// Sizes for the overflow check (task: 1024×768 and 1920×1080, 100 % and 150 %).
pub const CFG_LAYOUT: [Cfg; 4] = [
    Cfg { w: 1024.0, h: 768.0, ppp: 1.0 },
    Cfg { w: 1920.0, h: 1080.0, ppp: 1.0 },
    Cfg { w: 1024.0, h: 768.0, ppp: 1.5 },
    Cfg { w: 1920.0, h: 1080.0, ppp: 1.5 },
];

/// Root of the repository.
pub fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// Value of an environment variable; without the new (English) name the old German one as fallback.
pub fn env_var(name: &str, legacy: &str) -> Option<String> {
    std::env::var(name).or_else(|_| std::env::var(legacy)).ok()
}

/// Mode `current` (default).
pub const MODE_CURRENT: &str = "current";
/// Mode `baseline` (produce the state before phase 6).
pub const MODE_BASELINE: &str = "baseline";

/// Mode: `current` (default) or `baseline` (produce the state before phase 6); from `FP_QA_MODE`
/// (old `FP_QA_MODUS`, old values `aktuell`/`basis` are translated).
pub fn mode() -> String {
    match env_var("FP_QA_MODE", "FP_QA_MODUS").as_deref() {
        Some("baseline" | "basis") => MODE_BASELINE.into(),
        Some("current" | "aktuell") | None => MODE_CURRENT.into(),
        Some(other) => other.into(),
    }
}

/// Result folder of a mode; if there is nothing yet, the folder with the former German name
/// (`aktuell`/`basis`/`basislayout`), if present.
pub fn mode_dir(mode: &str) -> PathBuf {
    let dir = out_root().join(mode);
    let legacy = match mode {
        MODE_CURRENT => "aktuell",
        MODE_BASELINE => "basis",
        MODE_BASELINE_LAYOUT => "basislayout",
        _ => return dir,
    };
    let old = out_root().join(legacy);
    if !dir.is_dir() && old.is_dir() { old } else { dir }
}

/// Output folder of the results (`FP_QA_OUT`, old `FP_QA_AUS`, else `<repo>\target-i18n-qa\i18n-qa`).
pub fn out_root() -> PathBuf {
    match env_var("FP_QA_OUT", "FP_QA_AUS") {
        Some(p) => PathBuf::from(p),
        None => repo().join("target-i18n-qa").join("i18n-qa"),
    }
}

/// Only selected scenes/images (`FP_QA_ONLY`, old `FP_QA_NUR`; substring of the id).
pub fn only_filter() -> Option<String> {
    env_var("FP_QA_ONLY", "FP_QA_NUR").filter(|s| !s.is_empty())
}

pub fn out_dir() -> PathBuf {
    let d = out_root().join(mode());
    let _ = std::fs::create_dir_all(&d);
    d
}

/// Only selected images (`FP_QA_ONLY=menu:3` or similar, substring of the image id).
fn only() -> Option<String> {
    only_filter()
}

/// Set the language (before the user interface is built).
pub fn set_lang(lang: &str) {
    fp_engine::shell::disable();
    assert!(fp_i18n::set_language(lang), "unknown language {lang}");
}

// ------------------------------------------------------------------ Test document

/// Ids of the test objects.
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

/// Test document 800 × 600 with all object kinds (like the full tests), two pages, two states,
/// clipboard, style, recently opened file. Names inside the document (`vorlage.fw.png`,
/// `Bibliothekssymbol`, `Knopf` …) are user data and stay German on purpose: they are part of the
/// German text lists compared against the baseline.
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

/// Contents of the test document (user data, never translated).
pub const FIXTURE_TEXTS: &[&str] = &["Firepixel Test", "Bibliothekssymbol", "Grafik", "Knopf", "Animation", "Teststil", "vorlage", "gespeichert", "Eins", "Mein Arbeitsbereich", "Testbefehl", ".htm"];

/// A test situation (selection, preparation commands).
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

/// Selection situations: inspector per element kind and availability of the menu commands.
pub fn candidates(f: &Fix) -> Vec<Candidate> {
    let save = json!({ "path": f.save_to.to_string_lossy() });
    vec![
        c("no selection", vec![]),
        c("rectangle", f.rect.clone()),
        c("rectangle + ellipse", vec![first(&f.rect), first(&f.ellipse)]),
        c("three objects", vec![first(&f.rect), first(&f.ellipse), first(&f.group)]),
        c("text", f.text.clone()),
        c("text + ellipse", vec![first(&f.text), first(&f.ellipse)]),
        c("bitmap", f.bitmap.clone()),
        c("group", f.group.clone()),
        c("mask group", f.mask.clone()),
        c("graphic symbol", f.instance.clone()),
        c("two instances", vec![first(&f.instance), first(&f.instance2)]),
        c("button", f.button.clone()),
        c("animation symbol", f.anim.clone()),
        c("slice", f.slice.clone()),
        c("hotspot", f.hotspot.clone()),
        c("path", f.path.clone()),
        c("two open paths", vec![first(&f.path), first(&f.path2)]),
        c("ellipse", f.ellipse.clone()),
        c("auto shape", f.autoshape.clone()),
        Candidate { label: "pixel selection", prep: vec![], ids: vec![], pixels: true },
        Candidate { label: "bitmap + pixel selection", prep: vec![], ids: f.bitmap.clone(), pixels: true },
        Candidate { label: "saved + modified", prep: vec![("file.saveAs", save), ("draw.rectangle", json!({ "rect": [10, 10, 30, 30] }))], ids: vec![], pixels: false },
        Candidate { label: "after undo", prep: vec![("draw.rectangle", json!({ "rect": [10, 10, 30, 30] })), ("edit.undo", json!({}))], ids: vec![], pixels: false },
        Candidate { label: "text on path", prep: vec![("select.set", json!({ "ids": [first(&f.text), first(&f.ellipse)] })), ("text.attachToPath", json!({}))], ids: vec![], pixels: false },
        Candidate {
            label: "slice with pop-up menu",
            prep: vec![("select.set", json!({ "ids": f.slice })), ("modify.popUpMenu.addPopUpMenu", json!({ "items": [{ "text": "Eins", "url": "eins.htm" }] }))],
            ids: f.slice.clone(),
            pixels: false,
        },
        Candidate { label: "two layers", prep: vec![("layer.add", json!({}))], ids: vec![], pixels: false },
        Candidate { label: "master page", prep: vec![("pi.page.setMaster", json!({ "index": 0 })), ("page.setCurrent", json!({ "index": 1 }))], ids: vec![], pixels: false },
        Candidate { label: "imported symbol", prep: vec![("panel.library.import", json!({ "path": f.file.to_string_lossy() }))], ids: vec![], pixels: false },
    ]
}

pub fn apply_candidate(st: &mut AppState, cand: &Candidate) {
    for (n, p) in &cand.prep {
        let _ = st.session.execute(n, p.clone());
    }
    if cand.label != "text on path" {
        select(st, &cand.ids);
    }
    if cand.pixels {
        let _ = st.session.execute("select.pixels", json!({ "shape": "rect", "rect": [440, 40, 520, 120], "mode": "replace" }));
        if !cand.ids.is_empty() {
            let _ = st.session.execute("select.set", json!({ "ids": cand.ids, "mode": "add" }));
        }
    }
    if cand.label == "imported symbol" {
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

/// Commands the check run never executes (external programs, quit, window actions).
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

// ------------------------------------------------------------------ Texts and checks

/// A drawn text with its layer (window, popup, background).
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

/// At the end of every frame (before the pass ends): remember all texts per layer.
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

/// Texts of the last frame (in the paint order of the layers).
pub fn texts(h: &H) -> Vec<TextItem> {
    h.ctx.data(|d| d.get_temp::<std::sync::Arc<Vec<TextItem>>>(Id::new(TEXTS_ID))).map(|a| (*a).clone()).unwrap_or_default()
}

/// Area prefixes of the keys (`crates\fp-i18n\README.md`).
pub const KEY_PREFIXES: &[&str] = &["language", "btn", "fmt", "unit", "key", "test", "menu", "cmd", "ctx", "pmenu", "tool", "shape", "dlg", "pi", "panel", "ui", "err", "msg", "ai", "filter", "effect", "blend", "lang", "welcome"];

/// Raw keys in the text (e.g. `dlg-canvas-size`, `cmd-file-open.tooltip`).
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

/// Proper names, format names, units, symbols (may be unmarked in `qps`).
pub const PROPER: &[&str] = &[
    "Firepixel", "FW-PNG", "PNG", "PNG8", "PNG24", "PNG32", "GIF", "JPEG", "JPG", "WebP", "AVIF", "BMP", "TIFF", "TIF", "PSD", "PDF", "SVG", "ICO", "WBMP", "PICT", "JSF", "HTML", "CSS", "XML", "SWF", "AI", "EPS", "RGB", "RGBA", "HSB", "HSL",
    "CMYK", "HEX", "px", "pt", "dpi", "ppi", "K", "M", "MB", "KB", "GB", "kB", "Bit", "Bits", "fps", "ms", "s", "x", "X", "Y", "W", "H", "B", "R", "G", "A", "Arial", "Tahoma", "Verdana", "Segoe", "UI", "Times", "New", "Roman", "Courier", "Sans", "Serif", "Regular",
    "Bold", "Italic", "Normal", "Mono", "Consolas", "Calibri", "Cambria", "Georgia", "Impact", "Symbol", "Wingdings", "Webdings", "Marlett", "MCP", "SAM", "BiRefNet", "ONNX", "URL", "OK", "mm", "cm", "Myriad", "Pro", "Apache-2", "Hiera", "Tiny", "Small", "lite", "Matting", "HR", "jQuery", "Twain", "WebSnap", "Lossless",
];

/// Font families and styles of the system plus the native names of all languages.
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
        // language names are written in their own language (never translated)
        for l in fp_i18n::languages() {
            s.insert(l.name);
        }
        s
    })
}

/// May a text be unmarked in the test language?
pub fn exempt_unmarked(text: &str) -> bool {
    let t = text.trim();
    if !t.chars().any(|c| c.is_alphabetic()) {
        return true;
    }
    if FIXTURE_TEXTS.iter().any(|f| t == *f || (f.len() > 9 && t.contains(f))) {
        return true;
    }
    // font names (system fonts, styles) and native names of the languages
    if proper_names().contains(t) {
        return true;
    }
    // paths, file names, colours
    if t.contains(":\\") || t.contains('/') && !t.contains(' ') || t.ends_with(".png") || t.ends_with(".jsf") || t.starts_with('#') && t.len() <= 9 {
        return true;
    }
    // only proper names/units/numbers/symbols, abbreviations (≤ 3 capitals, F keys,
    // paper sizes), language codes (de_DE)
    let short = |w: &str| (w.chars().count() <= 3 && w.chars().all(|c| c.is_uppercase() || c.is_ascii_digit())) || (w.starts_with('F') && w.len() <= 3 && w[1..].chars().all(|c| c.is_ascii_digit()));
    t.split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_').filter(|w| !w.is_empty()).all(|w| {
        w.chars().all(|c| c.is_ascii_digit()) || PROPER.contains(&w) || short(w) || w.chars().count() == 1 || w.len() == 5 && w.as_bytes()[2] == b'_' || w.chars().all(|c| c.is_ascii_hexdigit()) && w.len() >= 6
    })
}

pub fn marked(text: &str) -> bool {
    text.contains("[!!") || text.contains("!!]")
}

/// Kinds of findings.
pub const KINDS: &[(&str, &str)] = &[
    (KIND_RAW_KEY, "raw key visible"),
    (KIND_UNMARKED, "unmarked text (qps)"),
    (KIND_CLIPPED, "text clipped horizontally (without \"…\")"),
    (KIND_WINDOW_EDGE, "text beyond the window edge"),
    (KIND_OVERLAP, "texts overlap"),
    (KIND_IN_WIDGET, "text reaches into a foreign widget"),
    (KIND_WIDGET_EDGE, "widget beyond the window edge"),
];

pub const KIND_RAW_KEY: &str = "raw_key";
pub const KIND_UNMARKED: &str = "unmarked";
pub const KIND_CLIPPED: &str = "clipped";
pub const KIND_WINDOW_EDGE: &str = "window_edge";
pub const KIND_OVERLAP: &str = "overlap";
pub const KIND_IN_WIDGET: &str = "in_widget";
pub const KIND_WIDGET_EDGE: &str = "widget_edge";

/// Finding kinds that are overflows (layout).
pub const LAYOUT_KINDS: &[&str] = &[KIND_CLIPPED, KIND_WINDOW_EDGE, KIND_OVERLAP, KIND_IN_WIDGET, KIND_WIDGET_EDGE];

/// Former German finding kinds (results written before BA-086) → current kind.
pub const LEGACY_KINDS: &[(&str, &str)] = &[
    ("rohschluessel", KIND_RAW_KEY),
    ("unmarkiert", KIND_UNMARKED),
    ("abgeschnitten", KIND_CLIPPED),
    ("fensterrand", KIND_WINDOW_EDGE),
    ("ueberlappung", KIND_OVERLAP),
    ("in_element", KIND_IN_WIDGET),
    ("element_rand", KIND_WIDGET_EDGE),
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

/// Layer a text belongs to: topmost window/popup whose area contains the clip rectangle.
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

/// Checks the current image; `text_checks` = (a)/(b), `layout_checks` = (c).
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
                push(KIND_RAW_KEY, &t.text, keys.join(", "));
            }
            if lang == fp_i18n::PSEUDO_LANG && !marked(&t.text) && !exempt_unmarked(&t.text) {
                push(KIND_UNMARKED, &t.text, r(t.rect));
            }
        }
    }
    if !layout_checks {
        return out;
    }
    let screen = h.ctx.content_rect();
    // visible part per text; covered texts (below a popup/window) do not count
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
        // paths in input fields may be clipped (the field scrolls)
        let ell = t.text.trim_end().ends_with('…') || t.text.trim_end().ends_with("...") || t.text.contains(":\\");
        let cut_x = t.rect.min.x < t.clip.min.x - 1.0 || t.rect.max.x > t.clip.max.x + 1.0;
        let at_screen = t.rect.min.x < screen.min.x - 0.5 || t.rect.max.x > screen.max.x + 0.5 || t.rect.min.y < screen.min.y - 0.5 || t.rect.max.y > screen.max.y + 0.5;
        if at_screen {
            push(KIND_WINDOW_EDGE, &t.text, format!("{} window {}", r(t.rect), r(screen)));
        } else if cut_x && !ell && t.rect.width() - v.width() > 2.0 {
            push(KIND_CLIPPED, &t.text, format!("{} visible {}", r(t.rect), r(v)));
        }
    }
    // text over text (same clip rectangle, no duplicate)
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
                push(KIND_OVERLAP, &format!("{} ⟷ {}", a.text, b.text), format!("{} × {}", r(a.rect), r(b.rect)));
            }
        }
    }
    // text partly reaches into a foreign widget of the same layer
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
            // own text inside the widget or widget inside the text (e.g. icon button in a row)
            if w.expand(1.5).contains_rect(v) || v.expand(1.5).contains_rect(*w) {
                continue;
            }
            // text that only touches the widget at its edge (label next to the field)
            if x.width() < 6.0 || x.area() < 0.15 * v.area().min(w.area()) {
                continue;
            }
            if seen.insert(t.text.clone()) {
                push(KIND_IN_WIDGET, &t.text, format!("text {} widget {}", r(t.rect), r(*w)));
            }
        }
    }
    // widgets beyond the window edge
    let outside: Vec<Rect> = widgets.iter().filter(|(_, w)| !screen.expand(1.0).contains_rect(*w)).map(|(_, w)| *w).collect();
    if !outside.is_empty() {
        push(KIND_WIDGET_EDGE, "", format!("{} widgets, e.g. {}", outside.len(), r(outside[0])));
    }
    out
}

/// Line of the text list (German unchanged): text with position.
pub fn dump_line(t: &TextItem) -> String {
    format!("{}\t{:.0},{:.0},{:.0},{:.0}", normalize_text(&t.text).replace('\n', "⏎"), t.rect.min.x, t.rect.min.y, t.rect.width(), t.rect.height())
}

/// Normalise run-dependent parts (temp folder with process id).
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

/// New entries phase 6 adds on purpose (View ▸ Language, German UI text): not counted as a difference.
pub const EXPECTED_NEW: &[&str] = &["Sprache"];

// ------------------------------------------------------------------ Images (scenes)

/// A check image: id (language independent), description, state preparation, interaction.
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

/// Main window in all themes.
pub fn scenes_main() -> Vec<Scene> {
    let mut v = Vec::new();
    for k in ThemeKind::ALL {
        let name = format!("{k:?}").to_lowercase();
        v.push(scene(format!("main:{name}"), format!("main window, theme {name}"), move |st, _| st.set_theme(k), no_ui));
        v.push(scene(
            format!("main:{name}:selection"),
            format!("main window with selection, theme {name}"),
            move |st, f| {
                st.set_theme(k);
                select(st, &f.rect);
            },
            no_ui,
        ));
    }
    v.push(scene(
        "main:empty",
        "main window without document",
        |st, _| {
            while st.session.active_id().is_some() {
                let _ = st.session.execute("file.close", json!({ "discard": true }));
            }
        },
        no_ui,
    ));
    v.push(scene(
        "main:iconic",
        "panels as icons with names",
        |st, _| {
            st.settings.workspace.dock_mode = fp_ui::workspace::DockMode::IconicWithNames;
        },
        no_ui,
    ));
    v
}

/// Status bar: pointer position over the canvas, message after a command, tool hints.
pub fn scenes_status() -> Vec<Scene> {
    let mut v = Vec::new();
    v.push(scene("status:pointer", "status bar, pointer over the canvas", |_, _| {}, |h, _| {
        let c = canvas_rect(h).ok_or("no canvas")?;
        hover(h, c.center());
        Ok(())
    }));
    for (id, cmd, cand) in [("status:group", "modify.group", "rectangle"), ("status:paste", "edit.paste", "no selection"), ("status:select_all", "select.all", "no selection")] {
        v.push(scene(id, format!("status bar after {cmd}"), move |st, f| {
            if let Some(c) = candidate_by_label(f, cand) {
                apply_candidate(st, &c);
            }
            st.run(cmd, json!({}));
        }, no_ui));
    }
    for (id, tool) in [("status:pen", fp_engine::ToolId::Pen), ("status:magic_wand", fp_engine::ToolId::MagicWand), ("status:text", fp_engine::ToolId::Text)] {
        v.push(scene(id, format!("status bar with tool {tool:?} over the canvas"), move |st, _| st.push(Action::SetTool(tool)), |h, _| {
            let c = canvas_rect(h).ok_or("no canvas")?;
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

/// Paths of all submenus (indices into `build_main_menu`), the menu itself = `[i]`.
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

/// Opens main menu `path[0]` and hovers the submenu entries one after the other.
pub fn open_menu_path(h: &mut H, path: &[usize]) -> Result<(), String> {
    let menus = fp_ui::menu::build_main_menu(st(h));
    let (title, mut nodes): (String, Vec<Node>) = menus.get(path[0]).cloned().ok_or("menu missing")?;
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
        // narrow window: the menu sits in the overflow menu "»" (popup with a lower index)
        egui::Popup::close_id(&h.ctx, Id::new(("fp-main-menu", path[0])));
        h.run_steps(1);
        let mut ok = false;
        let mut seen_title = Vec::new();
        // index of the overflow menu "»" = number of menus that fit into the bar (≤ path[0])
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
            return Err(format!("menu \"{title}\" does not open (not via \"»\" either; titles in the overflow: {seen_title:?}, first entry {want0:?})"));
        }
    }
    for &k in &path[1..] {
        let Some(Node::Sub { label, children, .. }) = nodes.get(k).cloned() else { return Err(format!("submenu {k} missing")) };
        // find the entry in the popup drawn last (last occurrence in paint order)
        let ts = texts(h);
        let t = ts.iter().rev().find(|t| t.text == label).ok_or_else(|| format!("entry \"{label}\" not drawn"))?;
        let p = t.rect.center();
        hover(h, p);
        h.run_steps(3);
        nodes = children;
        // first entry of the submenu visible?
        let want: Option<String> = nodes.iter().find_map(|n| match n {
            Node::Item { label, .. } | Node::Sub { label, .. } => Some(label.clone()),
            Node::Separator => None,
        });
        if let Some(w) = want
            && !texts(h).iter().any(|t| t.text == w)
        {
            return Err(format!("submenu \"{label}\" does not open"));
        }
    }
    Ok(())
}

/// Selection situations in which the menus are opened (so that submenus are enabled).
pub const MENU_STATES: &[&str] = &["rectangle + ellipse", "text", "bitmap", "slice", "pixel selection", "no selection"];

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

/// All entries of the menu tree (also disabled and unopened submenus), of the panel menus and
/// of the canvas context menus as text: (a) raw keys, (b) unmarked.
pub fn menu_tree_findings(lang: &str) -> Vec<Finding> {
    fn rec(nodes: &[Node], path: &str, out: &mut Vec<(String, String)>) {
        for n in nodes {
            match n {
                Node::Separator => {}
                Node::Item { label, shortcut, .. } => {
                    out.push((path.to_string(), label.clone()));
                    if let Some(s) = shortcut {
                        out.push((format!("{path} ▸ {label} (shortcut)"), s.clone()));
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
            entries.push(("menu bar".to_string(), title.clone()));
            rec(&nodes, &title, &mut entries);
        }
        for p in PanelId::ALL {
            rec(&fp_ui::menu::build_panel_menu(&st, *p), &format!("panel menu {p:?}"), &mut entries);
        }
        for id in ["6906", "6910", "6912", "6913", "6914", "6915", "6916", "6925", "49120"] {
            rec(&fp_ui::canvas::context::build(&st, fp_ui::canvas::context::entries(id)), &format!("context menu {id}"), &mut entries);
        }
    }
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for (path, text) in entries {
        // document list in the Window menu (user data; "Unbenannt" is the German UI text)
        if text.contains("Unbenannt") || text.contains(".png") || !seen.insert(text.clone()) {
            continue;
        }
        let keys = raw_keys(&text);
        if !keys.is_empty() {
            out.push(Finding { kind: KIND_RAW_KEY.into(), scene: format!("menu_tree: {path}"), cfg: "tree".into(), text: text.clone(), detail: keys.join(", ") });
        }
        if lang == fp_i18n::PSEUDO_LANG && !marked(&text) && !exempt_unmarked(&text) {
            out.push(Finding { kind: KIND_UNMARKED.into(), scene: format!("menu_tree: {path}"), cfg: "tree".into(), text, detail: String::new() });
        }
    }
    out
}

/// Panels (in front in the dock) and their panel menus (≡).
pub fn scenes_panels() -> Vec<Scene> {
    let mut v = Vec::new();
    for p in PanelId::ALL.iter().copied() {
        let name = format!("{p:?}");
        let cand = match p {
            PanelId::SymbolProperties | PanelId::DocumentLibrary => "graphic symbol",
            PanelId::AutoShapeProperties => "auto shape",
            PanelId::Url | PanelId::Behaviors => "slice",
            PanelId::Path => "path",
            PanelId::ImageEditing => "bitmap",
            _ => "rectangle",
        };
        v.push(scene(format!("panel:{name}"), format!("panel {name}"), move |st, f| {
            if let Some(c) = candidate_by_label(f, cand) {
                apply_candidate(st, &c);
            }
            st.push(Action::ShowPanel(p));
        }, no_ui));
        v.push(scene(format!("pmenu:{name}"), format!("panel menu {name}"), move |st, f| {
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
                // at the bottom next to the inspector: menu of the inspector (own images under "inspector")
                _ => return Ok(()),
            };
            let r = widget_rect(h, id).ok_or("menu button not found")?;
            click(h, r.center(), PointerButton::Primary);
            Ok(())
        }));
    }
    v
}

/// Inspector per element kind (all selection situations) and per tool.
pub fn scenes_inspector() -> Vec<Scene> {
    let mut v = Vec::new();
    let (_, _, f0) = fixture();
    for cand in candidates(&f0) {
        let label = cand.label;
        v.push(scene(format!("pi:{label}"), format!("inspector: {label}"), move |st, f| {
            if let Some(c) = candidate_by_label(f, label) {
                apply_candidate(st, &c);
            }
        }, no_ui));
    }
    for t in fp_engine::tools::TOOLS {
        let id = t.id;
        v.push(scene(format!("tool:{}", t.name), format!("tool {}", t.name), move |st, _| {
            let _ = st.session.execute("select.set", json!({ "ids": [] }));
            st.push(Action::SetTool(id));
        }, no_ui));
    }
    v
}

/// Context menus: canvas per state, document tab, layer row.
pub fn scenes_context() -> Vec<Scene> {
    use fp_engine::ToolId as T;
    let mut v = Vec::new();
    let cases: Vec<(&str, &str, Option<T>, bool)> = vec![
        ("none", "no selection", None, false),
        ("object", "rectangle", None, false),
        ("text", "text", None, false),
        ("pixel", "pixel selection", Some(T::Marquee), false),
        ("scale", "rectangle", Some(T::Scale), false),
        ("crop", "no selection", Some(T::Crop), false),
        ("crop_bitmap", "bitmap", Some(T::Crop), true),
        ("zoom", "no selection", Some(T::Zoom), false),
        ("eyedropper", "no selection", Some(T::Eyedropper), false),
    ];
    for (id, cand, tool, bitmap_mode) in cases {
        v.push(scene(format!("ctx:canvas:{id}"), format!("context menu canvas: {id}"), move |st, f| {
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
            let c = canvas_rect(h).ok_or("no canvas")?;
            // empty spot at the top left of the canvas area (next to the page)
            click(h, c.min + egui::vec2(12.0, 12.0), PointerButton::Secondary);
            Ok(())
        }));
    }
    v.push(scene("ctx:tab", "context menu document tab", |_, _| {}, |h, _| {
        let id = st(h).session.active_id().ok_or("no document")?;
        let r = widget_rect(h, Id::new(("doc-tab", id.0))).ok_or("tab not found")?;
        click(h, r.center(), PointerButton::Secondary);
        Ok(())
    }));
    for (pid, what) in [(PanelId::Layers, "layer"), (PanelId::Pages, "page"), (PanelId::States, "state")] {
        v.push(scene(format!("ctx:{what}"), format!("context menu of a row in panel {pid:?}"), move |st, _| st.push(Action::ShowPanel(pid)), move |h, _| {
            h.run_steps(2);
            let name = {
                let d = st(h).session.active().ok_or("no document")?;
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
                // states: first row with a digit in the dock
                None => ts.iter().filter(|t| t.rect.center().x > dock_x && t.text.trim().chars().next().is_some_and(|c| c.is_ascii_digit())).min_by(|a, b| a.rect.top().total_cmp(&b.rect.top())).cloned(),
            };
            let t = t.ok_or("row not found")?;
            click(h, t.rect.center(), PointerButton::Secondary);
            Ok(())
        }));
    }
    v
}

// ------------------------------------------------------------------ Dialogs

/// Kind of the open dialog (language independent).
pub fn dialog_kind(st: &AppState, cmd: &str) -> Option<String> {
    let d = &st.dialogs;
    if let Some(f) = &d.form {
        return Some(format!("form:{}", f.id));
    }
    if let Some(p) = &d.pending_form {
        return Some(format!("pending:{p:?}"));
    }
    if d.message.is_some() {
        return Some(format!("message:{cmd}"));
    }
    if d.confirm.is_some() {
        return Some(format!("confirm:{cmd}"));
    }
    let named: [(&str, bool); 20] = [
        ("new_document", d.new_document.is_some()),
        ("unsaved", d.unsaved.is_some()),
        ("preferences", d.preferences.is_some()),
        ("about", d.about),
        ("save_workspace", d.save_workspace.is_some()),
        ("canvas_size", d.canvas_size.is_some()),
        ("image_size", d.image_size.is_some()),
        ("numeric_transform", d.numeric_transform.is_some()),
        ("levels", d.levels.is_some()),
        ("curves", d.curves.is_some()),
        ("export", d.export.is_some()),
        ("image_preview", d.preview.is_some()),
        ("shortcuts", d.shortcuts.is_some()),
        ("text_editor", d.text_editor.is_some()),
        ("html_slice", d.html_slice.is_some()),
        ("saved_commands", d.saved_commands.is_some()),
        ("spelling", d.spelling.is_some()),
        ("popup_editor", d.popup_editor.is_some()),
        ("ai_missing", d.ai.missing.is_some()),
        ("ai_manager", d.ai.manager.is_some()),
    ];
    named.iter().find(|(_, open)| *open).map(|(n, _)| n.to_string())
}

/// A dialog found: command, selection situation, kind.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DialogCase {
    pub command: String,
    pub candidate: String,
    pub kind: String,
}

/// All commands from menus, panel menus and canvas context menus.
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

/// Searches a dialog for every command (cached in `dialogs.json`; again with `FP_QA_NEW=1`, old `FP_QA_NEU`).
/// A cache with the former German name (`dialoge.json`) is not read: its kinds and candidates
/// carry the old German names, so it is simply rebuilt.
pub fn discover_dialogs() -> Vec<DialogCase> {
    let cache = out_root().join("dialogs.json");
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
    // determine the availability per situation once
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
    // dialogs without a menu command of their own
    for (cmd, kind) in [("ui.unsaved", "unsaved"), ("ui.saveWorkspace", "save_workspace")] {
        if kinds.insert(kind.into()) {
            found.push(DialogCase { command: cmd.into(), candidate: "saved + modified".into(), kind: kind.into() });
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
        v.push(scene(format!("dlg:{kind}"), format!("dialog {kind} ({command})"), move |st, f| {
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
    // live filter dialogs (inspector "+"): every adjustable effect once
    let mut fx_kinds = BTreeSet::new();
    for i in 0..80usize {
        let (mut st, _, f) = fixture();
        select(&mut st, &f.rect);
        fp_ui::dialogs::effects::begin_add(&mut st, i);
        let Some(form) = &st.dialogs.form else { continue };
        if !fx_kinds.insert(form.id.clone()) {
            continue;
        }
        v.push(scene(format!("fx:{}", form.id), format!("live filter dialog no. {i}"), move |st, f| {
            select(st, &f.rect);
            fp_ui::dialogs::effects::begin_add(st, i);
        }, no_ui));
    }
    // menu "+" of the live filters in the inspector, submenus one after the other
    for k in 0..9usize {
        v.push(scene(format!("fx:menu:{k}"), format!("live filter menu \"+\", submenu {k}"), |st, f| select(st, &f.rect), move |h, _| {
            let r = widget_rect(h, Id::new("pi-fx-add")).ok_or("button \"+\" not found")?;
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
    // preferences: every category
    for i in 0..7usize {
        v.push(scene(format!("dlg:preferences:{i}"), format!("preferences, category {i}"), |st, _| st.run("edit.preferences", json!({})), move |h, _| {
            h.run_steps(2);
            let r = widget_rect(h, Id::new(("prefcat", i))).ok_or("category not found")?;
            click(h, r.center(), PointerButton::Primary);
            Ok(())
        }));
    }
    v
}

// ------------------------------------------------------------------ Run

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AreaResult {
    pub lang: String,
    pub area: String,
    pub mode: String,
    pub scenes: usize,
    pub configs: Vec<String>,
    /// Images that could not be produced (scene: reason).
    pub failed: Vec<String>,
    pub findings: Vec<Finding>,
    /// Text list per image (German only, `CFG_TEXT`).
    pub dump: BTreeMap<String, Vec<String>>,
    /// Differences against the state before phase 6 (German only).
    pub de_changed: Vec<String>,
    pub seconds: f64,
}

/// Produce an image: test document, preparation, harness, interaction.
pub fn build_scene(sc: &Scene, cfg: Cfg, wgpu: bool) -> Result<H, String> {
    let (mut st, ctx, f) = fixture();
    (sc.prep)(&mut st, &f);
    st.process_actions(&ctx);
    let mut h = std::panic::catch_unwind(AssertUnwindSafe(|| harness_for(st, cfg, wgpu))).map_err(|e| format!("PANIC {}", panic_msg(&e)))?;
    std::panic::catch_unwind(AssertUnwindSafe(|| (sc.ui)(&mut h, &f))).map_err(|e| format!("PANIC {}", panic_msg(&e)))??;
    std::panic::catch_unwind(AssertUnwindSafe(|| h.run_steps(3))).map_err(|e| format!("PANIC {}", panic_msg(&e)))?;
    Ok(h)
}

pub fn panic_msg(e: &Box<dyn std::any::Any + Send>) -> String {
    e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default()
}

/// Checks all images of an area in one language and writes the result.
pub fn run_area(lang: &str, area: &str, scenes: Vec<Scene>) -> AreaResult {
    run_area_with(lang, area, scenes, Vec::new())
}

/// Like [`run_area`], with additional findings (e.g. from the menu tree).
pub fn run_area_with(lang: &str, area: &str, scenes: Vec<Scene>, extra: Vec<Finding>) -> AreaResult {
    let t0 = std::time::Instant::now();
    let filter = only();
    let scenes: Vec<Scene> = scenes.into_iter().filter(|s| filter.as_ref().is_none_or(|f| s.id.contains(f.as_str()))).collect();
    let mut res = AreaResult { lang: lang.into(), area: area.into(), mode: mode(), scenes: scenes.len(), ..Default::default() };
    let layout_cfgs: Vec<Cfg> = if mode() == MODE_BASELINE { vec![] } else { CFG_LAYOUT.to_vec() };
    res.configs = std::iter::once(CFG_TEXT).chain(layout_cfgs.iter().copied()).map(|c| c.label()).collect();
    res.findings.extend(extra);
    for sc in &scenes {
        // text/key check and text list
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
        res.de_changed = compare_with_baseline(area, &res.dump);
    }
    res.seconds = t0.elapsed().as_secs_f64();
    let name = if filter.is_some() { format!("{lang}_{area}_partial.json") } else { format!("{lang}_{area}.json") };
    let _ = std::fs::write(out_dir().join(name), serde_json::to_string_pretty(&res).unwrap_or_default());
    eprintln!("{lang}/{area}: {} images, {} findings, {} not producible, {} German differences, {:.0} s", res.scenes, res.findings.len(), res.failed.len(), res.de_changed.len(), res.seconds);
    res
}

/// Images of selected scenes (wgpu) to `docs\screenshots\<prefix><file>.png`; with `compare`
/// also a pixel comparison against existing images `<compare><file>.png` (share of differing
/// pixels per image; images with the names used before BA-086 are found as well). Empty without
/// a graphics adapter.
pub fn render_images(scenes: &[Scene], pick: &[(&str, &str)], cfg: Cfg, prefix: &str, compare: Option<&str>) -> Vec<String> {
    let dir = repo().join("docs").join("screenshots");
    let mut report = Vec::new();
    for (id, file) in pick {
        let Some(sc) = scenes.iter().find(|s| s.id == *id) else {
            report.push(format!("{id}: scene missing"));
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
                report.push(format!("no graphics adapter: {e}"));
                return report;
            }
        };
        let path = dir.join(format!("{prefix}{file}.png"));
        if let Some(cmp) = compare {
            match load_png(&dir.join(format!("{cmp}{file}.png"))).or_else(|| load_png(&dir.join(format!("{}{}.png", legacy_image_prefix(cmp), legacy_image_name(file))))) {
                Some((w, hh, old)) => {
                    if (w, hh) != (img.width(), img.height()) {
                        report.push(format!("{file}: size {w}×{hh} → {}×{}", img.width(), img.height()));
                    } else {
                        let n = old.as_chunks::<4>().0.iter().zip(img.as_raw().as_chunks::<4>().0.iter()).filter(|(a, b)| a.iter().zip(b.iter()).any(|(x, y)| x.abs_diff(*y) > 8)).count();
                        let total = (w * hh) as f64;
                        report.push(format!("{file}: {n} differing pixels ({:.3} %)", 100.0 * n as f64 / total));
                    }
                }
                None => report.push(format!("{file}: no comparison image")),
            }
        }
        let _ = img.save(&path);
    }
    report
}

/// Standard images: main window per theme, a menu, a dialog, inspector (rectangle), normal and dark.
pub fn images_standard(prefix: &str, compare: Option<&str>) -> Vec<String> {
    let mut all = scenes_main();
    let (st0, _, _) = fixture();
    all.extend(scenes_menus(&st0).into_iter().filter(|s| s.id == "menu:4" || s.id == "menu:0"));
    drop(st0);
    all.extend(scenes_dialogs().into_iter().filter(|s| s.id == "dlg:canvas_size" || s.id == "dlg:new_document" || s.id == "dlg:preferences:0"));
    all.extend(scenes_inspector().into_iter().filter(|s| s.id == "pi:rectangle" || s.id == "pi:text"));
    // dark variants of dialog and inspector
    let dark = |id: &str, scenes: Vec<Scene>| -> Vec<Scene> {
        scenes
            .into_iter()
            .filter(|s| s.id == id)
            .map(|s| {
                let Scene { id, label, prep, ui } = s;
                Scene {
                    id: format!("{id}:dark"),
                    label: format!("{label} (dark)"),
                    prep: Box::new(move |st: &mut AppState, f: &Fix| {
                        st.set_theme(ThemeKind::Dark);
                        prep(st, f)
                    }),
                    ui,
                }
            })
            .collect()
    };
    all.extend(dark("dlg:canvas_size", scenes_dialogs()));
    all.extend(dark("pi:rectangle", scenes_inspector()));
    let pick = [
        ("main:normal:selection", "main_window_normal"),
        ("main:dark:selection", "main_window_dark"),
        ("main:oled:selection", "main_window_oled"),
        ("menu:0", "menu_file"),
        ("menu:4", "menu_modify"),
        ("dlg:canvas_size", "dialog_canvas_size_normal"),
        ("dlg:canvas_size:dark", "dialog_canvas_size_dark"),
        ("dlg:new_document", "dialog_new_document"),
        ("dlg:preferences:0", "dialog_preferences"),
        ("pi:rectangle", "inspector_rectangle_normal"),
        ("pi:rectangle:dark", "inspector_rectangle_dark"),
        ("pi:text", "inspector_text"),
    ];
    render_images(&all, &pick, Cfg { w: 1600.0, h: 1000.0, ppp: 1.0 }, prefix, compare)
}

/// Image prefix of the German baseline images.
pub const IMAGE_PREFIX_BASELINE_DE: &str = "phase6_baseline_de_";

/// Image names (without prefix) used before BA-086 → current name.
pub const LEGACY_IMAGE_NAMES: &[(&str, &str)] = &[
    ("hauptfenster_normal", "main_window_normal"),
    ("hauptfenster_dunkel", "main_window_dark"),
    ("hauptfenster_oled", "main_window_oled"),
    ("menue_datei", "menu_file"),
    ("menue_modifizieren", "menu_modify"),
    ("dialog_leinwandgroesse_normal", "dialog_canvas_size_normal"),
    ("dialog_leinwandgroesse_dunkel", "dialog_canvas_size_dark"),
    ("dialog_neues_dokument", "dialog_new_document"),
    ("dialog_voreinstellungen", "dialog_preferences"),
    ("inspektor_rechteck_normal", "inspector_rectangle_normal"),
    ("inspektor_rechteck_dunkel", "inspector_rectangle_dark"),
    ("inspektor_text", "inspector_text"),
];

/// Former name of an image (for comparisons against images written before BA-086).
pub fn legacy_image_name(name: &str) -> &str {
    LEGACY_IMAGE_NAMES.iter().find(|(_, new)| *new == name).map_or(name, |(old, _)| old)
}

/// Former image prefix (`phase6_baseline_de_` was `phase6_basis_de_`).
pub fn legacy_image_prefix(prefix: &str) -> String {
    prefix.replace("_baseline_", "_basis_")
}

/// The check tests of a language run (one test per area, can run in parallel).
#[macro_export]
macro_rules! qa_tests {
    ($lang:expr) => {
        #[test]
        #[ignore]
        fn main_window() {
            i18n_qa_common::set_lang($lang);
            i18n_qa_common::run_area($lang, "main_window", i18n_qa_common::scenes_main());
        }
        #[test]
        #[ignore]
        fn status_bar() {
            i18n_qa_common::set_lang($lang);
            i18n_qa_common::run_area($lang, "status_bar", i18n_qa_common::scenes_status());
        }
        #[test]
        #[ignore]
        fn menus() {
            i18n_qa_common::set_lang($lang);
            let (st, _, _) = i18n_qa_common::fixture();
            let sc = i18n_qa_common::scenes_menus(&st);
            drop(st);
            let tree = i18n_qa_common::menu_tree_findings($lang);
            i18n_qa_common::run_area_with($lang, "menus", sc, tree);
        }
        #[test]
        #[ignore]
        fn panels() {
            i18n_qa_common::set_lang($lang);
            i18n_qa_common::run_area($lang, "panels", i18n_qa_common::scenes_panels());
        }
        #[test]
        #[ignore]
        fn inspector() {
            i18n_qa_common::set_lang($lang);
            i18n_qa_common::run_area($lang, "inspector", i18n_qa_common::scenes_inspector());
        }
        #[test]
        #[ignore]
        fn dialogs() {
            i18n_qa_common::set_lang($lang);
            i18n_qa_common::run_area($lang, "dialogs", i18n_qa_common::scenes_dialogs());
        }
        #[test]
        #[ignore]
        fn context_menus() {
            i18n_qa_common::set_lang($lang);
            i18n_qa_common::run_area($lang, "context_menus", i18n_qa_common::scenes_context());
        }
    };
}

/// Load a PNG as RGBA8.
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

/// Compares the German text list with the state before phase 6.
pub fn compare_with_baseline(area: &str, dump: &BTreeMap<String, Vec<String>>) -> Vec<String> {
    let base = match load_result(MODE_BASELINE, "de", area) {
        Ok(Some(b)) => b,
        Ok(None) => return vec![format!("no baseline {}", mode_dir(MODE_BASELINE).join(format!("de_{area}.json")).display())],
        Err(_) => return vec!["baseline unreadable".into()],
    };
    let mut out = Vec::new();
    for (id, lines) in dump {
        let Some(old) = base.dump.get(id) else {
            out.push(format!("{id}: new (not in the baseline)"));
            continue;
        };
        // texts with a temp path: the width depends on the process id → compare the text only
        let norm = |v: &Vec<String>| -> BTreeSet<String> { v.iter().map(|l| normalize_text(l)).map(|l| if l.contains("firepixel-i18n-qa\\X") { l.split('\t').next().unwrap_or("").to_string() } else { l }).filter(|l| !EXPECTED_NEW.iter().any(|e| l.split('\t').next() == Some(*e))).collect() };
        let a: BTreeSet<String> = norm(old);
        let b: BTreeSet<String> = norm(lines);
        if a == b {
            continue;
        }
        // one entry per changed line ("−" before, "+" now), so that the same change across many
        // images can be summed up in the report
        for g in a.difference(&b).take(10) {
            out.push(format!("{id}: − {g}"));
        }
        for n in b.difference(&a).take(10) {
            out.push(format!("{id}: + {n}"));
        }
    }
    for id in base.dump.keys() {
        if !dump.contains_key(id) && only().is_none() {
            out.push(format!("{id}: missing (was in the baseline)"));
        }
    }
    out
}

// ------------------------------------------------------------------ Names before BA-086

/// Mode folder with the German layout results before phase 6 (used by the report).
pub const MODE_BASELINE_LAYOUT: &str = "baseline_layout";

/// Former German area names (results written before BA-086) → current name.
pub const LEGACY_AREAS: &[(&str, &str)] = &[
    ("hauptfenster", "main_window"),
    ("menues", "menus"),
    ("bedienfelder", "panels"),
    ("inspektor", "inspector"),
    ("dialoge", "dialogs"),
    ("kontextmenues", "context_menus"),
    ("statuszeile", "status_bar"),
];

/// Former German parts of scene ids (between `:`, candidate labels included) → current part.
pub const LEGACY_SEGMENTS: &[(&str, &str)] = &[
    ("haupt", "main"),
    ("auswahl", "selection"),
    ("leer", "empty"),
    ("symbolmodus", "iconic"),
    ("zeiger", "pointer"),
    ("gruppieren", "group"),
    ("einfuegen", "paste"),
    ("stift", "pen"),
    ("zauberstab", "magic_wand"),
    ("werkzeug", "tool"),
    ("leinwand", "canvas"),
    ("nichts", "none"),
    ("objekt", "object"),
    ("skalieren", "scale"),
    ("zuschneiden", "crop"),
    ("zuschneiden_bitmap", "crop_bitmap"),
    ("pipette", "eyedropper"),
    ("reiter", "tab"),
    ("ebene", "layer"),
    ("seite", "page"),
    ("meldung", "message"),
    ("rueckfrage", "confirm"),
    ("neues_dokument", "new_document"),
    ("ungespeichert", "unsaved"),
    ("voreinstellungen", "preferences"),
    ("ueber", "about"),
    ("arbeitsbereich_speichern", "save_workspace"),
    ("leinwandgroesse", "canvas_size"),
    ("bildgroesse", "image_size"),
    ("numerisch", "numeric_transform"),
    ("tonwert", "levels"),
    ("kurven", "curves"),
    ("bildvorschau", "image_preview"),
    ("tastenkuerzel", "shortcuts"),
    ("texteditor", "text_editor"),
    ("html_segment", "html_slice"),
    ("befehle_verwalten", "saved_commands"),
    ("rechtschreibung", "spelling"),
    ("ki_fehlt", "ai_missing"),
    ("ki_verwalten", "ai_manager"),
    ("menue", "menu"),
    ("dunkel", "dark"),
    ("ohne Auswahl", "no selection"),
    ("Rechteck", "rectangle"),
    ("Rechteck + Ellipse", "rectangle + ellipse"),
    ("drei Objekte", "three objects"),
    ("Text", "text"),
    ("Text + Ellipse", "text + ellipse"),
    ("Bitmap", "bitmap"),
    ("Gruppe", "group"),
    ("Maskengruppe", "mask group"),
    ("Grafiksymbol", "graphic symbol"),
    ("zwei Instanzen", "two instances"),
    ("Schaltfläche", "button"),
    ("Animationssymbol", "animation symbol"),
    ("Segment", "slice"),
    ("Hotspot", "hotspot"),
    ("Pfad", "path"),
    ("zwei offene Pfade", "two open paths"),
    ("Ellipse", "ellipse"),
    ("Automatische Form", "auto shape"),
    ("Pixelauswahl", "pixel selection"),
    ("Bitmap + Pixelauswahl", "bitmap + pixel selection"),
    ("gespeichert + geändert", "saved + modified"),
    ("nach Rückgängig", "after undo"),
    ("Text auf Pfad", "text on path"),
    ("Segment mit Pop-up-Menü", "slice with pop-up menu"),
    ("zwei Ebenen", "two layers"),
    ("Masterseite", "master page"),
    ("importiertes Symbol", "imported symbol"),
];

/// Current scene id of a scene id written before BA-086 (current ids pass through unchanged).
pub fn legacy_scene_id(id: &str) -> String {
    // `status:auswahl` (after "select all") is the only id whose part means something else
    if id == "status:auswahl" {
        return "status:select_all".into();
    }
    if let Some(rest) = id.strip_prefix("menübaum: ") {
        return format!("menu_tree: {rest}");
    }
    id.split(':').map(|p| LEGACY_SEGMENTS.iter().find(|(old, _)| *old == p).map_or(p, |(_, new)| new)).collect::<Vec<_>>().join(":")
}

/// Current finding kind of a kind written before BA-086.
pub fn legacy_kind(kind: &str) -> &str {
    LEGACY_KINDS.iter().find(|(old, _)| *old == kind).map_or(kind, |(_, new)| new)
}

/// Translates a result written before BA-086 (German area, scene ids and finding kinds) in place.
pub fn upgrade_result(r: &mut AreaResult) {
    if let Some((_, new)) = LEGACY_AREAS.iter().find(|(old, _)| *old == r.area) {
        r.area = (*new).into();
    }
    for f in &mut r.findings {
        f.kind = legacy_kind(&f.kind).to_string();
        f.scene = legacy_scene_id(&f.scene);
    }
    r.dump = std::mem::take(&mut r.dump).into_iter().map(|(k, v)| (legacy_scene_id(&k), v)).collect();
    for c in &mut r.de_changed {
        if let Some((id, rest)) = c.split_once(": ") {
            *c = format!("{}: {rest}", legacy_scene_id(id));
        }
    }
}

/// Loads `<language>_<area>.json` of a mode; without it the file with the former German area name.
/// `Ok(None)`: no file, `Err`: file unreadable. Old results are translated with [`upgrade_result`].
pub fn load_result(mode: &str, lang: &str, area: &str) -> Result<Option<AreaResult>, String> {
    let dir = mode_dir(mode);
    let mut names = vec![format!("{lang}_{area}.json")];
    names.extend(LEGACY_AREAS.iter().filter(|(_, new)| *new == area).map(|(old, _)| format!("{lang}_{old}.json")));
    for n in names {
        let Ok(s) = std::fs::read_to_string(dir.join(&n)) else { continue };
        let mut r = serde_json::from_str::<AreaResult>(&s).map_err(|e| format!("{n}: {e}"))?;
        upgrade_result(&mut r);
        return Ok(Some(r));
    }
    Ok(None)
}

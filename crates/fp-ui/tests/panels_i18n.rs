//! Bedienfelder mehrsprachig (Phase 6): jedes Bedienfeld in Deutsch, Englisch und der Testsprache
//! `qps` (+40 % Textlänge) bei schmalem (Mindestbreite des Docks) und breitem Dock zeichnen und die
//! gezeichneten Texte prüfen:
//! - kein Rohschlüssel (`panel-…`, `cmd-…` …) sichtbar,
//! - kein Text ragt über den sichtbaren Bereich hinaus, ohne mit „…“ gekürzt zu sein,
//! - keine zwei Texte überlappen sich,
//! - in `en` keine deutschen Reste der Bedienfeldtexte (Stichprobe aus `panels.ftl`).
//!
//! Die Sprache ist global; deshalb läuft alles in einem Test nacheinander (eigene Testdatei).

use egui::epaint::Shape;
use egui::{Rect, Vec2};
use egui_kittest::Harness;
use fp_engine::Session;
use fp_ui::app_state::UiSettings;
use fp_ui::panels::PanelId;
use fp_ui::FirepixelApp;
use serde_json::json;

// Sprachzustand ist global: die Tests dieser Datei laufen nacheinander.
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Breite des Bedienfeldinhalts: Dock-Mindestbreite 180 − Rand 8, Standard 268 − 8, breit 420.
const NARROW: f32 = 172.0;
const WIDE: f32 = 420.0;

/// Bedienfelder mit fester Anordnung nach Vorlage (DLU), die schon auf Deutsch breiter als die
/// Dock-Mindestbreite sind; geprüft werden sie ab ihrer natürlichen Breite (das Dock schneidet
/// darunter Felder und Texte gleichermaßen ab, unabhängig von der Sprache).
fn min_width(p: PanelId) -> f32 {
    match p {
        PanelId::Align | PanelId::ColorPalette => 196.0,
        PanelId::Info | PanelId::Url => 216.0,
        PanelId::FindReplace => 218.0,
        PanelId::Optimize => 228.0,
        PanelId::Layers => 232.0,
        PanelId::ColorMixer => 246.0,
        _ => NARROW,
    }
}

fn session() -> Session {
    let mut s = Session::default();
    fp_script::install_host(&mut s);
    s.execute("file.new", json!({"width": 300, "height": 200})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80]})).expect("rect");
    s
}

#[derive(Debug)]
struct Text {
    text: String,
    rect: Rect,
    clip: Rect,
}

fn texts(h: &Harness<'_, FirepixelApp>) -> Vec<Text> {
    let mut out = Vec::new();
    for cs in &h.output().shapes {
        collect(&cs.shape, cs.clip_rect, &mut out);
    }
    out
}

fn collect(s: &Shape, clip: Rect, out: &mut Vec<Text>) {
    match s {
        Shape::Text(t) => {
            let text = t.galley.text().to_string();
            if !text.trim().is_empty() {
                out.push(Text { text, rect: t.visual_bounding_rect(), clip });
            }
        }
        Shape::Vec(v) => {
            for x in v {
                collect(x, clip, out);
            }
        }
        _ => {}
    }
}

fn is_raw_key(s: &str) -> bool {
    const PREFIXES: [&str; 17] = ["panel-", "pmenu-", "ctx-", "cmd-", "menu-", "pi-", "ui-", "dlg-", "err-", "msg-", "tool-", "shape-", "btn-", "key-", "filter-", "effect-", "blend-"];
    PREFIXES.iter().any(|p| s.starts_with(p)) && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.')
}

fn harness(p: PanelId, width: f32, prepare: &dyn Fn(&mut Session)) -> Harness<'static, FirepixelApp> {
    let ctx = egui::Context::default();
    let mut s = session();
    prepare(&mut s);
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.sync_render = true;
    let mut first = true;
    Harness::builder().with_size(Vec2::new(width, 460.0)).with_pixels_per_point(1.0).with_max_steps(8).build_ui_state(
        move |ui, app: &mut FirepixelApp| {
            if first {
                fp_ui::theme::install_fonts(ui.ctx());
                first = false;
                ui.ctx().request_repaint();
                return;
            }
            fp_ui::panels::show(ui, &mut app.state, p);
        },
        a,
    )
}

fn prepare_for(p: PanelId) -> Box<dyn Fn(&mut Session)> {
    match p {
        PanelId::AutoShapeProperties => Box::new(|s: &mut Session| {
            let _ = s.execute("autoShape.insert", json!({"shape": "Star", "rect": [140, 20, 220, 100]}));
        }),
        PanelId::DocumentLibrary => Box::new(|s: &mut Session| {
            let _ = s.execute("select.all", json!({}));
            let _ = s.execute("modify.symbol.convertToSymbol", json!({"name": "Ein Symbol mit einem recht langen Namen", "type": "graphic"}));
        }),
        _ => Box::new(|s: &mut Session| {
            let _ = s.execute("select.all", json!({}));
        }),
    }
}

/// Probleme eines gezeichneten Bedienfelds.
fn check(p: PanelId, lang: &str, width: f32, h: &Harness<'_, FirepixelApp>) -> Vec<String> {
    let mut problems = Vec::new();
    let ts = texts(h);
    let screen = Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(width, 460.0));
    for t in &ts {
        if is_raw_key(&t.text) {
            problems.push(format!("{p:?} {lang} {width}: Rohschlüssel „{}“", t.text));
        }
        // sichtbarer Bereich = Clip-Rechteck ∩ Bildschirm; waagerecht darf nichts hinausragen
        let vis = t.clip.intersect(screen);
        if !vis.is_positive() || t.rect.bottom() < vis.top() || t.rect.top() > vis.bottom() {
            continue; // ganz ausgeblendet (z. B. weggescrollt)
        }
        let cut = t.rect.right() > vis.right() + 1.0 || t.rect.left() < vis.left() - 1.0;
        if cut && !t.text.ends_with('…') {
            problems.push(format!("{p:?} {lang} {width}: „{}“ abgeschnitten ({:.0}–{:.0}, sichtbar {:.0}–{:.0})", t.text, t.rect.left(), t.rect.right(), vis.left(), vis.right()));
        }
    }
    for (i, a) in ts.iter().enumerate() {
        for b in ts.iter().skip(i + 1) {
            let o = a.rect.intersect(b.rect);
            if o.width() > 2.0 && o.height() > 3.0 && a.clip.intersects(b.rect) && b.clip.intersects(a.rect) {
                problems.push(format!("{p:?} {lang} {width}: „{}“ überlappt „{}“", a.text, b.text));
            }
        }
    }
    problems
}

#[test]
fn panels_fit_in_every_language_and_width() {
    let _g = LOCK.lock();
    let mut problems = Vec::new();
    for lang in ["de", "en", fp_i18n::PSEUDO_LANG] {
        assert!(fp_i18n::set_language(lang), "{lang}");
        for &p in PanelId::ALL {
            for width in [min_width(p), WIDE] {
                let prep = prepare_for(p);
                let mut h = harness(p, width, &*prep);
                h.run_steps(4);
                let ts = texts(&h);
                // Gegenprobe: das Bedienfeld hat wirklich Text gezeichnet, in qps markiert
                if !matches!(p, PanelId::SpecialCharacters | PanelId::ColorPalette | PanelId::Swatches | PanelId::Styles | PanelId::AutoShapes) {
                    assert!(!ts.is_empty(), "{p:?} {lang}: keine Texte gezeichnet");
                    // URL zeigt nur Eigennamen („URL:“, „URLs.htm“)
                    if lang == fp_i18n::PSEUDO_LANG && p != PanelId::Url {
                        assert!(ts.iter().any(|t| t.text.contains("[!!")), "{p:?}: keine übersetzten Texte: {:?}", ts.iter().map(|t| &t.text).collect::<Vec<_>>());
                    }
                }
                problems.extend(check(p, lang, width, &h));
            }
        }
    }
    fp_i18n::set_language("de");
    problems.dedup();
    assert!(problems.is_empty(), "{} Befunde:\n{}", problems.len(), problems.join("\n"));
}

/// Gegenprobe der Prüfung: ungekürzter, zu langer Text und Überlappung werden erkannt.
#[test]
fn checker_detects_cut_and_overlap() {
    let ctx = egui::Context::default();
    let a = FirepixelApp::with_session(&ctx, Session::default(), UiSettings::default(), false);
    let mut h = Harness::builder().with_size(Vec2::new(120.0, 100.0)).with_pixels_per_point(1.0).build_ui_state(
        |ui, _app: &mut FirepixelApp| {
            let f = egui::FontId::proportional(13.0);
            ui.painter().text(egui::pos2(4.0, 10.0), egui::Align2::LEFT_TOP, "Ein viel zu langer Text ohne Kürzung", f.clone(), egui::Color32::WHITE);
            ui.painter().text(egui::pos2(4.0, 40.0), egui::Align2::LEFT_TOP, "Links", f.clone(), egui::Color32::WHITE);
            ui.painter().text(egui::pos2(12.0, 42.0), egui::Align2::LEFT_TOP, "Darüber", f.clone(), egui::Color32::WHITE);
            ui.painter().text(egui::pos2(4.0, 70.0), egui::Align2::LEFT_TOP, "panel-title-layers", f, egui::Color32::WHITE);
        },
        a,
    );
    h.run_steps(2);
    let probs = check(PanelId::Layers, "test", 120.0, &h);
    assert!(probs.iter().any(|p| p.contains("abgeschnitten")), "{probs:#?}");
    assert!(probs.iter().any(|p| p.contains("überlappt")), "{probs:#?}");
    assert!(probs.iter().any(|p| p.contains("Rohschlüssel")), "{probs:#?}");
}

#[test]
fn panel_titles_and_texts_follow_the_language() {
    let _g = LOCK.lock();
    let _ = fp_i18n::set_language("de");
    let de: Vec<&str> = PanelId::ALL.iter().map(|p| p.title()).collect();
    assert_eq!(PanelId::Layers.title(), "Ebenen");
    assert_eq!(PanelId::AutoShapeProperties.title(), "Eigenschaften für automatische Form");
    assert_eq!(fp_ui::panels::states::loop_text(0), "Endlos");
    assert!(fp_i18n::set_language("en"));
    assert_eq!(PanelId::Layers.title(), "Layers");
    assert_eq!(PanelId::FindReplace.title(), "Find");
    assert_eq!(fp_ui::panels::states::loop_text(-1), "No Looping");
    assert_eq!(fp_ui::panels::optimize::preset_label("JPEG – Höhere Qualität"), "JPEG - Better Quality");
    assert_eq!(fp_ui::panels::autoshapes::param_label("Rundung"), "Roundness");
    assert_eq!(fp_ui::panels::autoshapes::shape_key("3D Box"), "shape-3d-box");
    let en: Vec<&str> = PanelId::ALL.iter().map(|p| p.title()).collect();
    assert!(de.iter().zip(&en).filter(|(a, b)| a != b).count() >= 20, "{en:?}");
    assert!(fp_i18n::set_language(fp_i18n::PSEUDO_LANG));
    assert!(PanelId::Pages.title().starts_with("[!! Seiten"));
    fp_i18n::set_language("de");
    assert_eq!(PanelId::Pages.title(), "Seiten");
}

//! UI-Größentest in mehreren Sprachen (I18N-SHELL, AGENTEN_REGELN §1b/§4c), offscreen mit
//! egui_kittest und den echten Systemschriften.
//!
//! Für Deutsch, Englisch und die Testsprache `qps` (Texte markiert und etwa 40 % länger) wird das
//! Hauptfenster von 800×600 bis 3840×2160 (dazu Skalierung 125/150/200 %) gezeichnet. Geprüft wird
//! in **Anwendungsleiste**, **Ansichtsleiste** und **Fußleiste**:
//!
//! * kein gezeichneter Text ist abgeschnitten (liegt ganz in seinem Clip-Bereich und im Fenster),
//! * keine zwei Texte überlappen sich,
//! * die vier Ansichtsreiter (Original/Vorschau/2-fach/4-fach) sind alle da,
//! * die Menüs der Anwendungsleiste sind alle erreichbar.
//!
//! Dazu: Sprachwechsel zur Laufzeit (alles sofort in der neuen Sprache, gespeicherte Meldung weg).
//!
//! Die Sprache ist prozessweit; alle Prüfungen dieser Datei laufen deshalb nacheinander (`LOCK`)
//! in einer eigenen Test-Binärdatei, getrennt von den deutschen Gesamttests.

use egui::{Id, Pos2, Rect};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::theme::{Theme, ThemeKind};
use fp_ui::FirepixelApp;
use serde_json::json;
use std::sync::Mutex;

static LOCK: Mutex<()> = Mutex::new(());

const SIZES: &[(f32, f32, f32)] = &[
    (800.0, 600.0, 1.0),
    (1024.0, 768.0, 1.0),
    (1366.0, 768.0, 1.0),
    (1920.0, 1080.0, 1.0),
    (2560.0, 1440.0, 1.0),
    (3840.0, 2160.0, 1.0),
    (1920.0, 1080.0, 1.25),
    (2560.0, 1440.0, 1.5),
    (3840.0, 2160.0, 2.0),
];

const LANGS: &[&str] = &["de", "en", "qps"];

fn app() -> FirepixelApp {
    fp_engine::shell::disable();
    let ctx = egui::Context::default();
    let mut session = fp_ui::ui_commands::new_session(fp_engine::Preferences::new());
    let _ = session.execute("file.new", json!({ "width": 800, "height": 600 }));
    let _ = session.execute("draw.rectangle", json!({ "rect": [40, 40, 200, 160], "fillColor": "#ff0000" }));
    let mut app = FirepixelApp::with_session(&ctx, session, UiSettings::default(), false);
    app.state.os_clipboard = false;
    app
}

fn harness(app: FirepixelApp, w: f32, h: f32, ppp: f32) -> Harness<'static, FirepixelApp> {
    Harness::builder().with_size(egui::vec2(w / ppp, h / ppp)).with_pixels_per_point(ppp).with_max_steps(12).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app)
}

/// Ein gezeichneter Text: Inhalt, sichtbares Rechteck, Clip-Bereich.
struct Painted {
    text: String,
    rect: Rect,
    clip: Rect,
}

fn collect(shape: &egui::Shape, clip: Rect, out: &mut Vec<Painted>) {
    match shape {
        egui::Shape::Vec(v) => v.iter().for_each(|s| collect(s, clip, out)),
        egui::Shape::Text(t) if !t.galley.text().trim().is_empty() => out.push(Painted { text: t.galley.text().to_string(), rect: t.visual_bounding_rect(), clip }),
        _ => {}
    }
}

fn painted_texts(h: &Harness<'_, FirepixelApp>) -> Vec<Painted> {
    let mut out = Vec::new();
    for cs in &h.output().shapes {
        collect(&cs.shape, cs.clip_rect, &mut out);
    }
    out
}

fn widget_rect(ctx: &egui::Context, id: Id) -> Option<Rect> {
    ctx.viewport(|v| v.prev_pass.widgets.get(id).map(|w| w.rect))
}

/// Prüft die Texte eines Streifens (Anwendungsleiste, Ansichtsleiste, Fußleiste).
fn check_band(name: &str, band: Rect, screen: Rect, texts: &[Painted], problems: &mut Vec<String>) {
    let inside: Vec<&Painted> = texts.iter().filter(|t| band.contains(t.rect.center())).collect();
    if std::env::var_os("FP_LANGUAGES_DEBUG").or_else(|| std::env::var_os("FP_SPRACHEN_DEBUG")).is_some() {
        eprintln!("{name} [{:.0},{:.0}–{:.0},{:.0}]: {:?}", band.min.x, band.min.y, band.max.x, band.max.y, inside.iter().map(|t| t.text.as_str()).collect::<Vec<_>>());
    }
    for t in &inside {
        if !t.clip.expand(1.0).contains_rect(t.rect) || !screen.expand(1.0).contains_rect(t.rect) {
            problems.push(format!("{name}: „{}“ abgeschnitten ([{:.0},{:.0}–{:.0},{:.0}], Clip [{:.0},{:.0}–{:.0},{:.0}])", t.text, t.rect.min.x, t.rect.min.y, t.rect.max.x, t.rect.max.y, t.clip.min.x, t.clip.min.y, t.clip.max.x, t.clip.max.y));
        }
    }
    for (i, a) in inside.iter().enumerate() {
        for b in inside.iter().skip(i + 1) {
            let x = a.rect.intersect(b.rect);
            if x.is_positive() && x.area() > 4.0 {
                problems.push(format!("{name}: „{}“ überlappt „{}“", a.text, b.text));
            }
        }
    }
}

/// Alle Prüfungen für eine Sprache und Größe; liefert die Befunde.
fn check(lang: &str, w: f32, hgt: f32, ppp: f32, kind: ThemeKind) -> Vec<String> {
    assert!(fp_i18n::set_language(lang), "Sprache {lang} fehlt");
    let mut a = app();
    a.state.set_theme(kind);
    let mut h = harness(a, w, hgt, ppp);
    h.run_steps(4);
    let ctx = h.ctx.clone();
    let screen = ctx.content_rect();
    let theme = Theme::new(kind);
    let texts = painted_texts(&h);
    let mut problems = Vec::new();
    // Anwendungsleiste
    let appbar = Rect::from_min_max(screen.min, Pos2::new(screen.right(), screen.top() + theme.metrics.app_bar_height));
    check_band("Anwendungsleiste", appbar, screen, &texts, &mut problems);
    // Menüs der Anwendungsleiste: jede Menüüberschrift ist sichtbar oder liegt im Überlaufmenü „»“
    let overflow = texts.iter().any(|t| appbar.contains(t.rect.center()) && t.text == "»");
    for (m, _) in fp_ui::menu::build_main_menu(&h.state().state) {
        if !overflow && !texts.iter().any(|t| appbar.contains(t.rect.center()) && t.text == m) {
            problems.push(format!("Anwendungsleiste: Menü „{m}“ nicht sichtbar"));
        }
    }
    // Ansichtsleiste: vier Ansichtsreiter und Seitenknopf (Streifen ab dem linken Rand der Leinwand)
    let left = widget_rect(&ctx, Id::new(("view-mode", 0usize))).map(|r| r.left() - 4.0).unwrap_or(screen.left());
    match widget_rect(&ctx, Id::new("page-nav")) {
        Some(nav) => {
            let band = Rect::from_min_max(Pos2::new(left, nav.top() - 3.0), Pos2::new(nav.right() + 4.0, nav.bottom() + 3.0));
            check_band("Ansichtsleiste", band, screen, &texts, &mut problems);
            for i in 0..4usize {
                if widget_rect(&ctx, Id::new(("view-mode", i))).is_none() {
                    problems.push(format!("Ansichtsleiste: Ansichtsreiter {} fehlt", i + 1));
                }
            }
        }
        None => problems.push("Ansichtsleiste fehlt".into()),
    }
    // Fußleiste
    match widget_rect(&ctx, Id::new("footer-zoom")) {
        Some(z) => {
            let band = Rect::from_min_max(Pos2::new(left, z.top() - 1.0), Pos2::new(z.right() + 2.0, z.bottom() + 1.0));
            check_band("Fußleiste", band, screen, &texts, &mut problems);
        }
        None => problems.push("Fußleiste fehlt".into()),
    }
    problems
}

#[test]
fn languages_sizes_without_overflow() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut all = Vec::new();
    let mut n = 0;
    for lang in LANGS {
        for (w, hgt, ppp) in SIZES {
            n += 1;
            for p in check(lang, *w, *hgt, *ppp, ThemeKind::Normal) {
                all.push(format!("[{lang} {w}×{hgt} @ {:.0} %] {p}", ppp * 100.0));
            }
        }
        // alle Designs in der kleinsten Größe
        for kind in [ThemeKind::Dark, ThemeKind::Oled] {
            n += 1;
            for p in check(lang, 800.0, 600.0, 1.0, kind) {
                all.push(format!("[{lang} 800×600 {}] {p}", kind.label()));
            }
        }
    }
    fp_i18n::set_language("de");
    eprintln!("Sprachen × Größen: {n} Kombinationen, {} Befunde", all.len());
    assert!(all.is_empty(), "Überlauf in Anwendungs-/Ansichts-/Fußleiste:\n{}", all.join("\n"));
}

#[test]
fn language_switch_at_runtime() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    assert!(fp_i18n::set_language("de"));
    let mut h = harness(app(), 1366.0, 768.0, 1.0);
    h.run_steps(3);
    let has = |h: &Harness<'_, FirepixelApp>, s: &str| painted_texts(h).iter().any(|t| t.text == s);
    assert!(has(&h, "Vorschau") && has(&h, "4-fach"), "deutsche Ansichtsreiter");
    h.state_mut().state.set_status("Meldung auf Deutsch", 0.0);
    // Wechsel ohne Neustart: sofort alles in Englisch, gespeicherte Meldung verworfen
    assert!(fp_i18n::set_language("en"));
    h.run_steps(2);
    assert!(has(&h, "Preview") && has(&h, "4-Up"), "englische Ansichtsreiter");
    assert!(!has(&h, "Vorschau"), "kein deutscher Rest in der Ansichtsleiste");
    assert!(h.state().state.status.is_none(), "Statusmeldung der alten Sprache verworfen");
    // Fußleiste: Auswahlinfo in Englisch
    let _ = h.state_mut().state.session.execute("select.all", json!({}));
    h.run_steps(2);
    assert!(painted_texts(&h).iter().any(|t| t.text.ends_with("(Document)") || t.text == "Rectangle"), "englische Auswahlinfo");
    // zurück
    assert!(fp_i18n::set_language("de"));
    h.run_steps(2);
    assert!(has(&h, "Vorschau"));
    let _: &AppState = &h.state().state;
}

/// Texte in `area`, die abgeschnitten sind (weder ganz sichtbar noch mit „…“ gekürzt).
fn cut_texts(texts: &[Painted], area: Rect) -> Vec<String> {
    texts
        .iter()
        .filter(|t| area.contains(t.rect.center()) && !t.text.ends_with('…'))
        .filter(|t| {
            let vis = t.clip.intersect(area);
            // ganz weggescrollte (senkrechte Listen) zählen nicht
            vis.is_positive() && t.rect.bottom() > vis.top() && t.rect.top() < vis.bottom() && (t.rect.right() > vis.right() + 1.0 || t.rect.left() < vis.left() - 1.0)
        })
        .map(|t| format!("„{}“ ({:.0}–{:.0}, sichtbar bis {:.0})", t.text, t.rect.left(), t.rect.right(), t.clip.intersect(area).right()))
        .collect()
}

/// Jedes Bedienfeld bei kleinster Dockbreite (angedockt) und als sehr kleine schwebende Gruppe:
/// das Dock bzw. die Gruppe wird nie schmaler als das Bedienfeld, nichts wird abgeschnitten.
#[test]
fn panels_at_minimal_dock_width() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut problems = Vec::new();
    for lang in LANGS {
        assert!(fp_i18n::set_language(lang));
        for &p in fp_ui::panels::PanelId::ALL {
            if p == fp_ui::panels::PanelId::SymbolProperties {
                continue;
            }
            for floating in [false, true] {
                let mut a = app();
                let _ = a.state.session.execute("select.all", json!({}));
                {
                    let ws = &mut a.state.settings.workspace;
                    ws.dock_width = 60.0;
                    if floating {
                        ws.float_panel(p, [300.0, 150.0]);
                        if let Some(f) = ws.floating.last_mut() {
                            f.size = [20.0, 380.0];
                        }
                    } else {
                        ws.move_to_dock_group(p, 0, 0);
                        ws.show(p);
                    }
                }
                let mut h = harness(a, 1600.0, 900.0, 1.0);
                h.run_steps(4);
                let ctx = h.ctx.clone();
                let screen = ctx.content_rect();
                let area = if floating {
                    let st = &h.state().state;
                    let f = st.settings.workspace.floating.last().cloned();
                    let w = f.as_ref().map(|f| f.size[0].max(fp_ui::docking::floating_min_width(st, &f.group))).unwrap_or(0.0);
                    if w + 1.0 < fp_ui::docking::panel_min_width(st, p) + 2.0 * fp_ui::docking::PANEL_INSET {
                        problems.push(format!("[{lang}] {p:?}: schwebende Gruppe {w:.0} px schmaler als das Bedienfeld"));
                    }
                    Rect::from_min_size(Pos2::new(300.0, 150.0), egui::vec2(w, 380.0))
                } else {
                    // Dock: rechts neben dem Greifer der Dockbreite
                    let left = widget_rect(&ctx, Id::new("dock-width")).map(|r| r.left()).unwrap_or(screen.right() - 300.0);
                    let w = screen.right() - left;
                    let need = fp_ui::docking::panel_min_width(&h.state().state, p) + 2.0 * fp_ui::docking::PANEL_INSET;
                    if w + 1.0 < need {
                        problems.push(format!("[{lang}] {p:?}: Dock {w:.0} px schmaler als das Bedienfeld ({need:.0} px)"));
                    }
                    Rect::from_min_max(Pos2::new(left, screen.top() + 60.0), screen.max)
                };
                for c in cut_texts(&painted_texts(&h), area) {
                    problems.push(format!("[{lang}] {p:?} {}: {c}", if floating { "schwebend" } else { "angedockt" }));
                }
            }
        }
    }
    fp_i18n::set_language("de");
    problems.dedup();
    assert!(problems.is_empty(), "{} Befunde:\n{}", problems.len(), problems.join("\n"));
}

#[test]
fn german_wording_unchanged() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    assert!(fp_i18n::set_language("de"));
    assert_eq!(fp_ui::appbar::workspace_display_name("Erweiterter Modus"), "Erweiterter Modus");
    assert_eq!(fp_ui::appbar::workspace_display_name("Mein Layout"), "Mein Layout");
    assert_eq!(fp_i18n::tr!("ui-sel-document", format = "GIF"), "GIF (Dokument)");
    assert_eq!(fp_i18n::tr!("ui-file-save-failed", error = "x"), "Die Datei konnte nicht gespeichert werden:\nx");
    assert_eq!(ThemeKind::Dark.name(), "Dunkel");
    assert!(fp_i18n::set_language("en"));
    assert_eq!(fp_ui::appbar::workspace_display_name("Erweiterter Modus"), "Expanded Mode");
    assert_eq!(ThemeKind::Dark.name(), "Dark");
    assert!(fp_ui::cli_help().contains("Usage: firepixel"));
    assert!(fp_i18n::set_language("de"));
}

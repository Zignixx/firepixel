//! Größe des Dialogs „Bitmap nachzeichnen…“ (FV-12, Nutzerbefund N20): standardmäßig außen
//! ≈ 1050 × 830 (vorher 902 × 670), „Aufräumen“ und „Erweitert“ ohne Bildlauf sichtbar; in kleinen
//! Hauptfenstern höchstens 90 % davon, mit Mindestgröße; eine vom Nutzer gezogene Größe gilt beim
//! nächsten Öffnen wieder (Voreinstellung `TraceDialogSize`). Ziehen wie ein Mensch über viele Bilder.
//! Wie alle Testläufe mit `FP_NO_SHELL_OPEN=1` starten (AGENTEN_REGELN §4a).

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use fp_ui::app_state::{Action, AppState, UiSettings};
use fp_ui::dialogs::{self, trace};
use fp_ui::theme::ThemeKind;
use fp_ui::ui_kit::sp;
use fp_ui::FirepixelApp;
use serde_json::json;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Sprache und Design sind global: Tests dieser Datei laufen nacheinander.
static LOCK: Mutex<()> = Mutex::new(());

struct Guard(#[allow(dead_code)] std::sync::MutexGuard<'static, ()>);

impl Drop for Guard {
    fn drop(&mut self) {
        fp_i18n::set_language("de");
    }
}

fn lock() -> Guard {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    fp_i18n::set_language("de");
    Guard(g)
}

type H = Harness<'static, FirepixelApp>;

/// Bitmap 200 × 120 (Kreis und Rechteck auf Weiß), ausgewählt.
fn session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("Dokument");
    let mut img = fp_raster::RgbaImage::filled(fp_geom::IRect::from_xywh(20, 30, 200, 120), fp_color::Rgba8::new(255, 255, 255, 255)).expect("Bild");
    let w = img.width();
    for (i, px) in img.data_mut().iter_mut().enumerate() {
        let (x, y) = ((i % w) as f64 + 0.5, (i / w) as f64 + 0.5);
        if (x - 60.0).hypot(y - 60.0) < 40.0 {
            *px = [20, 60, 200, 255];
        } else if (120.0..180.0).contains(&x) && (30.0..100.0).contains(&y) {
            *px = [220, 30, 40, 255];
        }
    }
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    s.execute("edit.paste", json!({})).expect("Bitmap einfügen");
    s
}

fn harness_full(w: f32, h: f32, ppp: f32, prefs: Option<&str>, wgpu: bool, theme: ThemeKind) -> H {
    let ctx = egui::Context::default();
    let mut s = session();
    if let Some(v) = prefs {
        s.prefs.set(trace::PREF_SIZE, fp_engine::prefs::PrefValue::Str(v.to_string()));
    }
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    app.state.set_theme(theme);
    let b = Harness::builder().renderer(crate::gpu_common::renderer()).with_size(egui::vec2(w, h)).with_pixels_per_point(ppp).with_max_steps(8).with_step_dt(1.0 / 60.0);
    let b = if wgpu { b.renderer(crate::gpu_common::renderer()) } else { b };
    let mut h = b.build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h
}

fn harness_sized(w: f32, h: f32, ppp: f32, prefs: Option<&str>) -> H {
    harness_full(w, h, ppp, prefs, false, ThemeKind::Normal)
}

fn st(h: &mut H) -> &mut AppState {
    &mut h.state_mut().state
}

fn open(h: &mut H) {
    st(h).push(Action::Run("modify.traceBitmap".into(), json!({})));
    h.run_steps(3);
    assert!(st(h).dialogs.trace.is_some(), "Dialog offen");
}

fn close(h: &mut H) {
    st(h).dialogs.trace = None;
    h.run_steps(3);
}

fn wait_idle(h: &mut H) {
    let t0 = Instant::now();
    loop {
        h.run_steps(1);
        let busy = st(h).dialogs.trace.as_ref().is_some_and(|d| d.is_busy());
        if !busy || t0.elapsed() > Duration::from_secs(60) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    h.run_steps(2);
}

fn outer(h: &H) -> egui::Rect {
    dialogs::dialog_rect(&h.ctx, trace::DIALOG_ID).expect("Dialog gezeichnet")
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

/// Ziehen wie ein Mensch: Drücken, `steps` Bewegungen über mehrere Bilder, Loslassen.
fn drag(h: &mut H, from: Pos2, to: Pos2, steps: usize) {
    ev(h, Event::PointerMoved(from));
    ev(h, Event::PointerButton { pos: from, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    for i in 1..=steps {
        let p = from + (to - from) * (i as f32 / steps as f32);
        ev(h, Event::PointerMoved(p));
    }
    ev(h, Event::PointerButton { pos: to, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

/// Reine Größenrechnung: Vorgabe, 90 %-Grenze, Mindestgröße, gemerkte Größe.
#[test]
fn size_rules() {
    let frame = egui::vec2(2.0, dialogs::DIALOG_TITLE_H + 2.0);
    let (min, init) = trace::dialog_sizes(egui::vec2(1920.0, 1080.0), None);
    assert_eq!(init, trace::DEFAULT_CLIENT, "großes Fenster: Vorgabe");
    assert_eq!(init + frame, egui::vec2(1050.0, 830.0), "außen 1050 × 830");
    assert_eq!(min, trace::MIN_CLIENT);
    for screen in [egui::vec2(1366.0, 768.0), egui::vec2(1024.0, 768.0), egui::vec2(800.0, 600.0), egui::vec2(1024.0 / 1.5, 768.0 / 1.5), egui::vec2(480.0, 400.0)] {
        let (min, init) = trace::dialog_sizes(screen, None);
        let o = init + frame;
        // höchstens 90 %, außer die Mindestgröße verlangt mehr (dann wie vorher: Fenster − 16 px)
        assert!(o.x <= screen.x * 0.9 + 0.5 || init.x == min.x, "{screen:?}: Breite {o:?} ≤ 90 %");
        assert!(o.y <= screen.y * 0.9 + 0.5 || init.y == min.y, "{screen:?}: Höhe {o:?} ≤ 90 %");
        assert!(o.x <= screen.x - 14.0 && o.y <= screen.y - 14.0, "{screen:?}: passt ins Fenster");
        assert!(init.x >= min.x && init.y >= min.y, "{screen:?}: Ausgangsgröße ≥ Mindestgröße");
        assert!(min.x >= 320.0 && min.y >= 300.0, "{screen:?}: sinnvolle Mindestgröße {min:?}");
        assert!(min == trace::MIN_CLIENT || min.x <= screen.x - 16.0 || min.y <= screen.y - 16.0, "{screen:?}: Mindestgröße");
    }
    // gemerkte Größe: gilt, solange sie passt; zu groß → 90 %; zu klein → Mindestgröße
    assert_eq!(trace::dialog_sizes(egui::vec2(1920.0, 1080.0), Some(egui::vec2(1200.0, 900.0))).1, egui::vec2(1200.0, 900.0));
    assert_eq!(trace::dialog_sizes(egui::vec2(1920.0, 1080.0), Some(egui::vec2(3000.0, 3000.0))).1, egui::vec2(1726.0, 942.0));
    assert_eq!(trace::dialog_sizes(egui::vec2(1920.0, 1080.0), Some(egui::vec2(100.0, 100.0))).1, trace::MIN_CLIENT);
}

/// Standardgröße im großen Hauptfenster (100 % und 150 %): außen 1050 × 830, zentriert; „Aufräumen“
/// und „Erweitert“ stehen ohne Bildlauf über der Knopfleiste (de und en).
#[test]
fn default_size_shows_clean_up_without_scrolling() {
    let _g = lock();
    for (w, hh, ppp) in [(1920.0, 1080.0, 1.0), (2560.0 / 1.5, 1440.0 / 1.5, 1.5), (2560.0, 1440.0, 1.0)] {
        for lang in ["de", "en"] {
            fp_i18n::set_language(lang);
            let mut h = harness_sized(w, hh, ppp, None);
            open(&mut h);
            wait_idle(&mut h);
            let r = outer(&h);
            assert!((r.width() - 1050.0).abs() <= 1.0 && (r.height() - 830.0).abs() <= 1.0, "{lang} {w}×{hh}: außen {:?}", r.size());
            let screen = h.ctx.content_rect();
            assert!((r.center() - screen.center()).length() <= 2.0, "zentriert");
            let bar_top = r.bottom() - 1.0 - sp::BUTTON_BAR_H;
            for (role, label) in [(egui::accesskit::Role::CheckBox, fp_i18n::t("trace-refine")), (egui::accesskit::Role::Button, fp_i18n::t("trace-dlg-advanced"))] {
                let n = h.get_by_role_and_label(role, &label);
                let lr = n.rect();
                assert!(lr.bottom() <= bar_top - sp::S + 0.5 && lr.top() >= r.top(), "{lang} {w}×{hh}: „{label}“ {lr:?} ohne Bildlauf sichtbar (Knopfleiste ab {bar_top})");
            }
            // Vorschau: deutlich größer als vorher (rechte Spalte ≥ 640 px breit)
            let seg = h.get_by_label(&fp_i18n::t("trace-view-split")).rect();
            assert!(r.right() - seg.left() >= 640.0, "{lang}: Vorschau breit genug ({:.0})", r.right() - seg.left());
        }
    }
}

/// Kleine Hauptfenster: höchstens 90 %, ganz sichtbar, Mindestgröße greift beim Verkleinern.
#[test]
fn small_windows_cap_at_ninety_percent() {
    let _g = lock();
    for (w, hh, ppp) in [(1366.0, 768.0, 1.0), (1024.0, 768.0, 1.0), (1024.0 / 1.5, 768.0 / 1.5, 1.5), (800.0, 600.0, 1.0)] {
        let mut h = harness_sized(w, hh, ppp, None);
        open(&mut h);
        h.run_steps(2);
        let r = outer(&h);
        let screen = h.ctx.content_rect();
        let (min, _) = trace::dialog_sizes(screen.size(), None);
        let at_min_w = (r.width() - 2.0 - min.x).abs() < 1.0;
        let at_min_h = (r.height() - 2.0 - dialogs::DIALOG_TITLE_H - min.y).abs() < 1.0;
        assert!((r.width() <= screen.width() * 0.9 + 1.0 || at_min_w) && (r.height() <= screen.height() * 0.9 + 1.0 || at_min_h), "{w}×{hh}@{ppp}: {:?} ≤ 90 % von {:?} (oder Mindestgröße)", r.size(), screen.size());
        assert!(screen.contains_rect(r), "{w}×{hh}: ganz sichtbar");
        assert!(r.width() >= 320.0 && r.height() >= 300.0, "Mindestgröße");
        // weiter verkleinern geht nicht unter die Mindestgröße
        drag(&mut h, r.max - egui::vec2(3.0, 3.0), r.max - egui::vec2(600.0, 600.0), 20);
        let r2 = outer(&h);
        assert!(r2.width() >= min.x + 1.0 && r2.height() >= min.y + 1.0 + dialogs::DIALOG_TITLE_H, "{w}×{hh}: nicht kleiner als {min:?}: {:?}", r2.size());
    }
}

/// Eine gezogene Größe wird gemerkt und gilt beim nächsten Öffnen (auch in einer neuen Sitzung mit
/// denselben Voreinstellungen); ohne Ziehen bleibt nichts gespeichert.
#[test]
fn dragged_size_is_remembered() {
    let _g = lock();
    let mut h = harness_sized(1920.0, 1080.0, 1.0, None);
    open(&mut h);
    h.run_steps(3);
    close(&mut h);
    assert_eq!(st(&mut h).session.prefs.get_str(trace::PREF_SIZE, ""), "", "ohne Ziehen nichts gemerkt");
    open(&mut h);
    let r = outer(&h);
    // rechte untere Ecke um (+120, +60) ziehen, in 24 Schritten
    drag(&mut h, r.max - egui::vec2(3.0, 3.0), r.max + egui::vec2(117.0, 57.0), 24);
    let r2 = outer(&h);
    assert!((r2.width() - (r.width() + 120.0)).abs() <= 1.0 && (r2.height() - (r.height() + 60.0)).abs() <= 1.0, "Ecke folgt der Maus: {:?} → {:?}", r.size(), r2.size());
    let saved = st(&mut h).session.prefs.get_str(trace::PREF_SIZE, "");
    assert_eq!(saved, format!("{} {}", r2.width() - 2.0, r2.height() - 2.0 - dialogs::DIALOG_TITLE_H), "gemerkt");
    close(&mut h);
    open(&mut h);
    let r3 = outer(&h);
    assert_eq!(r3.size(), r2.size(), "beim nächsten Öffnen dieselbe Größe");
    close(&mut h);
    // neue Sitzung mit denselben Voreinstellungen, kleines Fenster: auf 90 % begrenzt
    let mut h2 = harness_sized(1024.0, 768.0, 1.0, Some(&saved));
    open(&mut h2);
    h2.run_steps(2);
    let r4 = outer(&h2);
    assert!(r4.width() <= 1024.0 * 0.9 + 1.0 && r4.height() <= 768.0 * 0.9 + 1.0, "{:?}", r4.size());
    // die begrenzte Größe überschreibt die gemerkte nicht
    assert_eq!(st(&mut h2).session.prefs.get_str(trace::PREF_SIZE, ""), saved);
    let mut h3 = harness_sized(1920.0, 1080.0, 1.0, Some(&saved));
    open(&mut h3);
    h3.run_steps(2);
    assert_eq!(outer(&h3).size(), r2.size(), "neue Sitzung: gemerkte Größe");
}

/// Bilder für NUTZERBEFUNDE (N20): Standardgröße bei 1920 × 1080, Normal/Dunkel/OLED, de/en.
/// Nur auf Wunsch: `cargo test -p fp-ui --test trace_size_fv12 -- --ignored`.
#[test]
#[ignore]
fn n20_screenshots() {
    let _g = lock();
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/screenshots/nutzerbefunde");
    for lang in ["de", "en"] {
        fp_i18n::set_language(lang);
        for (theme, tname) in [(ThemeKind::Normal, "normal"), (ThemeKind::Dark, "dunkel"), (ThemeKind::Oled, "oled")] {
            let mut h = harness_full(1920.0, 1080.0, 1.0, None, true, theme);
            open(&mut h);
            wait_idle(&mut h);
            h.run_steps(3);
            match h.render() {
                Ok(img) => {
                    let _ = img.save(out.join(format!("N20_nachzeichnen_standard_{tname}_{lang}.png")));
                }
                Err(e) => eprintln!("kein Grafikadapter: {e}"),
            }
        }
    }
}

//! Offscreen-Tests des Hauptfensters mit egui_kittest: Aufbau in allen drei Designs, mit und ohne
//! Dokument; mit wgpu (WARP/GPU) werden Bilder nach `target\fp-ui-snapshots\` geschrieben und auf
//! die Grundfarben des Designs geprüft. Ohne Grafikadapter wird nur der Aufbau geprüft.

use egui_kittest::Harness;
use fp_ui::app_state::UiSettings;
use fp_ui::theme::{Theme, ThemeKind};
use fp_ui::FirepixelApp;
use std::path::{Path, PathBuf};

fn corpus_file() -> Option<PathBuf> {
    let corpus = std::env::var_os("FP_CORPUS_DIR").map(std::path::PathBuf::from).unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("..").join("korpus"));
    let p = corpus.join("First Run").join("Templates").join("Mobile").join("Sample.png");
    p.is_file().then_some(p)
}

fn make_app(kind: ThemeKind, open: bool) -> FirepixelApp {
    let mut session = fp_engine::Session::default();
    session.prefs.theme = kind.to_engine();
    if open {
        match corpus_file() {
            Some(p) => {
                let _ = session.execute("file.open", serde_json::json!({ "path": p.to_string_lossy() }));
            }
            None => {
                let _ = session.execute("file.new", serde_json::json!({ "width": 800, "height": 600 }));
                let _ = session.execute("draw.rectangle", serde_json::json!({ "rect": [80, 80, 340, 280] }));
            }
        }
    }
    let settings = UiSettings { theme: kind, ..UiSettings::default() };
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, session, settings, false);
    app.state.set_theme(kind);
    app
}

fn harness(app: FirepixelApp, gpu: bool) -> Harness<'static, FirepixelApp> {
    let b = Harness::builder().with_size(egui::vec2(1800.0, 1130.0)).with_pixels_per_point(1.0).with_max_steps(12);
    let b = if gpu { b.wgpu() } else { b };
    b.build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app)
}

#[test]
fn main_window_builds_in_all_themes() {
    for kind in ThemeKind::ALL {
        for open in [false, true] {
            let mut h = harness(make_app(kind, open), false);
            h.run_steps(4);
            let st = &h.state().state;
            assert_eq!(st.settings.theme, kind);
            assert_eq!(st.session.documents().is_empty(), !open);
            assert_eq!(st.settings.workspace.dock.len(), 5);
        }
    }
}

#[test]
fn theme_switch_at_runtime() {
    let mut h = harness(make_app(ThemeKind::Normal, true), false);
    h.run_steps(2);
    h.state_mut().state.push(fp_ui::app_state::Action::SetTheme(ThemeKind::Oled));
    h.run_steps(3);
    assert_eq!(h.state().state.settings.theme, ThemeKind::Oled);
    assert_eq!(Theme::current(&h.ctx).kind, ThemeKind::Oled);
    assert_eq!(h.state().state.session.prefs.theme, fp_engine::Theme::Oled);
}

#[test]
fn panels_toggle_from_window_menu_command() {
    let mut h = harness(make_app(ThemeKind::Normal, true), false);
    h.run_steps(2);
    assert!(!h.state().state.settings.workspace.is_visible(fp_ui::panels::PanelId::Info));
    h.state_mut().state.run("window.info", serde_json::json!({}));
    h.run_steps(2);
    assert!(h.state().state.settings.workspace.is_visible(fp_ui::panels::PanelId::Info));
    assert_eq!(h.state().state.session.command_state("window.info").1, Some(true));
}

/// Bildschirmfotos für die Dokumentation (`docs\screenshots\phase5_hg_*.png`, 1800 × 1130):
/// Hauptfenster in allen drei Designs, dazu 4-fach-Vorschau, Text-Inspektor mit Fenster-Menü-Dialog.
/// Läuft nur auf Wunsch: `cargo test -p fp-ui --test layout bildschirmfotos -- --ignored`.
#[test]
#[ignore]
fn bildschirmfotos() {
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("docs").join("screenshots");
    let shots: Vec<(String, ThemeKind, u8)> = ThemeKind::ALL.iter().map(|k| (format!("phase5_hg_{}.png", k.label().to_lowercase()), *k, 0u8)).chain([("phase5_hg_vierfach.png".to_string(), ThemeKind::Normal, 1u8), ("phase5_hg_dialog_oled.png".to_string(), ThemeKind::Oled, 2u8)]).collect();
    for (name, kind, extra) in shots {
        let mut app = make_app(kind, true);
        match extra {
            1 => {
                if let Some(d) = app.state.session.active_mut() {
                    d.view.preview_mode = fp_engine::view::PreviewMode::FourUp;
                }
            }
            2 => {
                let _ = app.state.session.execute("select.all", serde_json::json!({}));
                app.state.run("modify.canvas.canvasSize", serde_json::json!({}));
            }
            _ => {}
        }
        let mut h = harness(app, true);
        h.run_steps(8);
        match h.render() {
            Ok(img) => {
                let _ = img.save(out.join(&name));
            }
            Err(e) => {
                eprintln!("kein Grafikadapter: {e}");
                return;
            }
        }
    }
}

/// Offscreen-Bilder aller drei Designs (braucht einen wgpu-Adapter; ohne Adapter übersprungen).
#[test]
fn snapshots_all_themes() {
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("target").join("fp-ui-snapshots");
    let _ = std::fs::create_dir_all(&out);
    for kind in ThemeKind::ALL {
        let app = make_app(kind, true);
        let mut h = harness(app, true);
        h.run_steps(6);
        let img = match h.render() {
            Ok(i) => i,
            Err(e) => {
                eprintln!("kein Grafikadapter, Bild übersprungen: {e}");
                return;
            }
        };
        let name = format!("hauptfenster_{}.png", kind.label().to_lowercase());
        let _ = img.save(out.join(&name));
        // Rahmenfarbe des Designs unten links im Werkzeugbedienfeld
        let t = Theme::new(kind);
        let px = img.get_pixel(30, 1100).0;
        let want = t.colors.frame_bg;
        assert!(
            (px[0] as i32 - want.r() as i32).abs() <= 2 && (px[1] as i32 - want.g() as i32).abs() <= 2,
            "{name}: Rahmenfarbe {px:?}, erwartet {want:?}"
        );
    }
}

//! Gesamttest der Oberfläche mit egui_kittest (HG, Phase 5): das ganze Hauptfenster läuft offscreen
//! mit dem Testdokument in allen drei Designs. Geprüft wird ohne Panik:
//!
//! * jedes der Bedienfelder sichtbar (Dock, schwebend, Symbolmodus),
//! * der Eigenschaften-Inspektor für jede Objektart (Layout je Auswahl) und je Werkzeug,
//! * jeder Dialog, den ein Menübefehl öffnet (zwei Bilder lang gezeichnet, dann Esc),
//! * Vorschau/2-fach/4-fach, Bildschirmmodi, Bedienfelder ausblenden, Symbolmodus.
//!
//! Ergebnis: `docs\funktionstest\oberflaeche_auto.md`.

use crate::full_common;

use egui_kittest::Harness;
use fp_engine::view::PreviewMode;
use fp_ui::app_state::{Action, AppState, UiSettings};
use fp_ui::panels::PanelId;
use fp_ui::theme::ThemeKind;
use fp_ui::FirepixelApp;
use full_common::*;
use serde_json::json;
use std::panic::AssertUnwindSafe;

fn app_with(st: AppState, kind: ThemeKind) -> FirepixelApp {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, fp_engine::Session::default(), UiSettings::default(), false);
    app.state = st;
    app.state.set_theme(kind);
    app
}

fn harness(app: FirepixelApp) -> Harness<'static, FirepixelApp> {
    Harness::builder().with_size(egui::vec2(1600.0, 1000.0)).with_pixels_per_point(1.0).with_max_steps(8).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app)
}

/// Läuft `frames` Bilder; `Err` mit Panikmeldung.
fn frames(h: &mut Harness<'static, FirepixelApp>, n: usize) -> Result<(), String> {
    std::panic::catch_unwind(AssertUnwindSafe(|| h.run_steps(n))).map_err(|e| panic_msg(&e))
}

fn row(path: String, cmd: &str, r: Result<String, String>) -> Row {
    match r {
        Ok(note) => Row { path, command: cmd.into(), outcome: Outcome::Ui, note },
        Err(e) => Row { path, command: cmd.into(), outcome: Outcome::Panic(e), note: String::new() },
    }
}

#[test]
fn full_ui() {
    let mut rows = Vec::new();
    for kind in ThemeKind::ALL {
        let k = kind.label();
        // Bedienfelder
        for p in PanelId::ALL {
            let (mut st, _ctx, f) = fixture();
            select(&mut st, &f.rect);
            st.settings.workspace.show(*p);
            let mut h = harness(app_with(st, kind));
            let r = frames(&mut h, 3).map(|_| if h.state().state.settings.workspace.is_visible(*p) { "sichtbar".to_string() } else { "nicht sichtbar".into() });
            let r = match r {
                Ok(s) if s == "sichtbar" => Ok(s),
                Ok(s) => Err(s),
                Err(e) => Err(e),
            };
            rows.push(row(format!("{k} ▸ Bedienfeld ▸ {}", p.title()), &format!("{p:?}"), r));
        }
        // Inspektor je Objektart
        let (_, _, f0) = fixture();
        let kinds: Vec<(&str, Vec<u64>)> = vec![
            ("Dokument (keine Auswahl)", vec![]),
            ("Rechteck", f0.rect.clone()),
            ("Ellipse", f0.ellipse.clone()),
            ("Pfad", f0.path.clone()),
            ("Text", f0.text.clone()),
            ("Bitmap", f0.bitmap.clone()),
            ("Gruppe", f0.group.clone()),
            ("Maskengruppe", f0.mask.clone()),
            ("Grafiksymbol", f0.instance.clone()),
            ("Schaltfläche", f0.button.clone()),
            ("Animationssymbol", f0.anim.clone()),
            ("Segment", f0.slice.clone()),
            ("Hotspot", f0.hotspot.clone()),
            ("Mehrfachauswahl", f0.three.clone()),
        ];
        for (label, ids) in kinds {
            let (mut st, _ctx, _f) = fixture();
            select(&mut st, &ids);
            let mut h = harness(app_with(st, kind));
            rows.push(row(format!("{k} ▸ Inspektor ▸ {label}"), "inspector", frames(&mut h, 3).map(|_| "gezeichnet".into())));
        }
        // Inspektor je Werkzeug
        for t in fp_engine::tools::TOOLS {
            let (mut st, _ctx, _f) = fixture();
            st.push(Action::SetTool(t.id));
            let mut h = harness(app_with(st, kind));
            rows.push(row(format!("{k} ▸ Inspektor ▸ Werkzeug {}", t.label), t.name, frames(&mut h, 3).map(|_| "gezeichnet".into())));
        }
        // Ansichtsmodi, Bildschirmmodi, Dock
        for (label, setup) in [
            ("Vorschau", 0usize),
            ("2-fach", 1),
            ("4-fach", 2),
            ("Vollbild mit Menüs", 3),
            ("Vollbild", 4),
            ("Bedienfelder ausblenden", 5),
            ("Symbolmodus", 6),
            ("Symbolmodus mit Namen", 7),
        ] {
            let (mut st, _ctx, _f) = fixture();
            match setup {
                0..=2 => {
                    if let Some(d) = st.session.active_mut() {
                        d.view.preview_mode = [PreviewMode::Preview, PreviewMode::TwoUp, PreviewMode::FourUp][setup];
                    }
                }
                3 | 4 => st.session.ui.screen_mode = (setup - 2) as u8,
                5 => st.settings.workspace.panels_hidden = true,
                6 => st.settings.workspace.dock_mode = fp_ui::workspace::DockMode::Iconic,
                _ => st.settings.workspace.dock_mode = fp_ui::workspace::DockMode::IconicWithNames,
            }
            let mut h = harness(app_with(st, kind));
            rows.push(row(format!("{k} ▸ Ansicht ▸ {label}"), "view", frames(&mut h, 3).map(|_| "gezeichnet".into())));
        }
    }
    // Dialoge aus dem Hauptmenü (Design Normal)
    let (st, _ctx, _f) = fixture();
    let mut items = Vec::new();
    for (label, nodes) in fp_ui::menu::build_main_menu(&st) {
        walk(&nodes, &label, &mut items);
    }
    drop(st);
    for (path, a) in items {
        let fp_ui::menu::ItemAction::Command(c) = a else { continue };
        if external(&c) {
            continue;
        }
        let (_, _, f) = fixture();
        let mut opened = false;
        for cand in candidates(&f) {
            let (mut st, ctx, _f) = fixture();
            apply_candidate(&mut st, &cand);
            if !available(&st, &c) {
                continue;
            }
            st.push(Action::Run(c.clone(), json!({})));
            st.process_actions(&ctx);
            if !(st.dialogs.any_open() || st.dialogs.pending_form.is_some()) {
                break;
            }
            opened = true;
            let mut h = harness(app_with(st, ThemeKind::Normal));
            let mut r = frames(&mut h, 2);
            if r.is_ok() {
                h.key_press(egui::Key::Escape);
                r = frames(&mut h, 2);
            }
            rows.push(row(format!("Dialog ▸ {path}"), &c, r.map(|_| format!("{}: gezeichnet, Esc", cand.label))));
            break;
        }
        let _ = opened;
    }
    let (ok, n) = write_report("oberflaeche_auto.md", "Oberfläche (Bedienfelder, Inspektor, Dialoge, Ansichten; 3 Designs)", "full_ui.rs", &rows);
    eprintln!("Oberfläche: {ok}/{n} ✅");
    let bad: Vec<_> = rows.iter().filter(|r| !r.outcome.ok()).map(|r| format!("{} {:?}", r.path, r.outcome)).collect();
    assert!(bad.iter().all(|b| !b.contains("Panic")), "Paniken: {bad:#?}");
}

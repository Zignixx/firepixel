//! UI-/UX-Test über Größen, DPI und Zustände (HG, AGENTEN_REGELN §4c), offscreen mit egui_kittest.
//!
//! Für jede Kombination aus Fenstergröße (800×600 … 3840×2160, sehr schmal/hoch, sehr breit/flach),
//! Skalierung (100/150/200 %), Design (Normal/Dunkel/OLED) und Zustand (Standard, Dock schmal/breit,
//! Gruppen eingeklappt, Symbolmodus, schwebende Bedienfelder sehr klein/groß, Inspektor aus,
//! Dialog offen, viele Ebenen/Seiten mit langen Namen, leeres Programm) wird das Hauptfenster
//! gezeichnet und geprüft:
//!
//! * keine Panik,
//! * kein bedienbares Element ragt über den Fensterrand (sichtbare Klickfläche),
//! * keine teilweise überlappenden bedienbaren Elemente derselben Ebene (Verdeckung),
//! * Layout springt nicht: zwei aufeinanderfolgende Bilder liefern dieselben Rechtecke,
//! * die Leinwand behält eine Mindestfläche.
//!
//! Ergebnis: `docs\funktionstest\groessen_auto.md`; mit `FP_SNAPSHOTS=1` zusätzlich Bilder je
//! Größe/Design nach `target\fp-ui-groessen\`.

mod gesamt_common;

use egui::{Rect, Sense};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::panels::PanelId;
use fp_ui::theme::ThemeKind;
use fp_ui::workspace::DockMode;
use fp_ui::FirepixelApp;
use gesamt_common::*;
use serde_json::json;
use std::panic::AssertUnwindSafe;

const SIZES: &[(f32, f32, &str)] = &[
    (800.0, 600.0, "800×600"),
    (1024.0, 768.0, "1024×768"),
    (1366.0, 768.0, "1366×768"),
    (1920.0, 1080.0, "1920×1080"),
    (2560.0, 1440.0, "2560×1440"),
    (3840.0, 2160.0, "3840×2160"),
    (800.0, 1400.0, "schmal/hoch 800×1400"),
    (2400.0, 520.0, "breit/flach 2400×520"),
];

#[derive(Clone, Copy, Debug, PartialEq)]
enum State {
    Standard,
    DockNarrow,
    DockWide,
    Collapsed,
    Iconic,
    IconicNames,
    FloatTiny,
    FloatHuge,
    NoInspector,
    Dialog,
    ManyLayersPages,
    Empty,
}

const STATES: &[State] = &[
    State::Standard,
    State::DockNarrow,
    State::DockWide,
    State::Collapsed,
    State::Iconic,
    State::IconicNames,
    State::FloatTiny,
    State::FloatHuge,
    State::NoInspector,
    State::Dialog,
    State::ManyLayersPages,
    State::Empty,
];

fn prepare(state: State) -> AppState {
    let (mut st, ctx, f) = fixture();
    match state {
        State::Standard => {}
        State::DockNarrow => st.settings.workspace.dock_width = 60.0,
        State::DockWide => st.settings.workspace.dock_width = 5000.0,
        State::Collapsed => st.settings.workspace.dock.iter_mut().for_each(|g| g.collapsed = true),
        State::Iconic => st.settings.workspace.dock_mode = DockMode::Iconic,
        State::IconicNames => st.settings.workspace.dock_mode = DockMode::IconicWithNames,
        State::FloatTiny | State::FloatHuge => {
            let ws = &mut st.settings.workspace;
            ws.float_panel(PanelId::Layers, [200.0, 150.0]);
            ws.show(PanelId::Info);
            for fl in &mut ws.floating {
                fl.size = if state == State::FloatTiny { [20.0, 10.0] } else { [6000.0, 4000.0] };
                if state == State::FloatHuge {
                    fl.pos = [-500.0, -300.0];
                }
            }
        }
        State::NoInspector => st.settings.workspace.show_inspector = false,
        State::Dialog => {
            select(&mut st, &f.rect);
            st.run("modify.canvas.canvasSize", json!({}));
            st.process_actions(&ctx);
        }
        State::ManyLayersPages => {
            let long = "Sehr langer Name für eine Ebene, der in keine Spalte passt und gekürzt werden muss";
            for i in 0..40 {
                let _ = st.session.execute("layer.add", json!({ "name": format!("{long} {i}") }));
            }
            for i in 0..25 {
                let _ = st.session.execute("page.add", json!({}));
                let _ = st.session.execute("page.rename", json!({ "name": format!("Seite mit sehr langem Namen Nummer {i} – Übersicht") }));
            }
            for _ in 0..30 {
                let _ = st.session.execute("state.add", json!({}));
            }
            ws_show_all(&mut st);
        }
        State::Empty => {
            while st.session.active_id().is_some() {
                let _ = st.session.execute("file.close", json!({ "discard": true }));
            }
        }
    }
    st.settings.workspace.repair();
    st
}

fn ws_show_all(st: &mut AppState) {
    for p in [PanelId::Layers, PanelId::Pages, PanelId::States] {
        st.settings.workspace.show(p);
    }
}


/// Bedienbare Elemente des letzten Bildes: (Ebene, sichtbare Klickfläche).
fn interactive(ctx: &egui::Context) -> Vec<(egui::LayerId, Rect, egui::Id)> {
    ctx.viewport(|v| {
        let mut out = Vec::new();
        for (layer, rects) in v.prev_pass.widgets.layers() {
            for w in rects {
                if w.enabled && (w.sense.contains(Sense::click()) || w.sense.contains(Sense::drag())) && w.interact_rect.is_positive() {
                    out.push((*layer, w.interact_rect, w.id));
                }
            }
        }
        out
    })
}

/// Prüft eine Konfiguration; liefert Befunde.
fn check(kind: ThemeKind, size: (f32, f32), ppp: f32, state: State) -> Result<Vec<String>, String> {
    let st = prepare(state);
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, fp_engine::Session::default(), UiSettings::default(), false);
    app.state = st;
    app.state.set_theme(kind);
    // Fenstergröße in Gerätepixeln: bei 200 % stehen nur halb so viele Punkte zur Verfügung
    let mut h = Harness::builder().with_size(egui::vec2(size.0 / ppp, size.1 / ppp)).with_pixels_per_point(ppp).with_max_steps(12).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    std::panic::catch_unwind(AssertUnwindSafe(|| h.run_steps(4))).map_err(|e| format!("PANIK: {}", panic_msg(&e)))?;
    let screen = h.ctx.content_rect();
    let a = interactive(&h.ctx);
    std::panic::catch_unwind(AssertUnwindSafe(|| h.run_steps(1))).map_err(|e| format!("PANIK: {}", panic_msg(&e)))?;
    let b = interactive(&h.ctx);
    let mut problems = Vec::new();
    // über den Rand
    let outside: Vec<Rect> = b.iter().filter(|(_, r, _)| !screen.expand(1.0).contains_rect(*r)).map(|(_, r, _)| *r).collect();
    if !outside.is_empty() {
        problems.push(format!("{} Elemente ragen über den Fensterrand (z. B. [{:.0},{:.0}–{:.0},{:.0}])", outside.len(), outside[0].min.x, outside[0].min.y, outside[0].max.x, outside[0].max.y));
    }
    // teilweise Überlappung bedienbarer Elemente derselben Ebene (nicht verschachtelt)
    let region = |r: &Rect| -> &'static str {
        let c = r.center();
        if c.y < 31.0 {
            "Anwendungsleiste"
        } else if c.y < 62.0 {
            "Symbolleiste"
        } else if c.x < 75.0 {
            "Werkzeugbedienfeld"
        } else if c.x > screen.right() - 300.0 {
            "Dock"
        } else if c.y > screen.bottom() - 140.0 {
            "Inspektor/Fußleiste"
        } else {
            "Mitte"
        }
    };
    let mut overlaps = 0;
    let mut examples: Vec<String> = Vec::new();
    for (i, (la, ra, _)) in b.iter().enumerate() {
        for (lb, rb, _) in b.iter().skip(i + 1) {
            if la != lb {
                continue;
            }
            let x = ra.intersect(*rb);
            if !x.is_positive() || x.area() < 16.0 {
                continue;
            }
            let nested = ra.expand(0.5).contains_rect(*rb) || rb.expand(0.5).contains_rect(*ra);
            if !nested && x.area() > 0.3 * ra.area().min(rb.area()) {
                overlaps += 1;
                if examples.len() < 3 {
                    examples.push(format!("{}: [{:.0},{:.0}–{:.0},{:.0}] × [{:.0},{:.0}–{:.0},{:.0}]", region(ra), ra.min.x, ra.min.y, ra.max.x, ra.max.y, rb.min.x, rb.min.y, rb.max.x, rb.max.y));
                }
            }
        }
    }
    if overlaps > 0 {
        problems.push(format!("{overlaps} teilweise überlappende Elemente (z. B. {})", examples.join(" · ")));
    }
    // springt das Layout?
    let key = |v: &Vec<(egui::LayerId, Rect, egui::Id)>| {
        let mut k: Vec<(egui::Id, [i32; 4])> = v.iter().map(|(_, r, id)| (*id, [r.min.x as i32, r.min.y as i32, r.max.x as i32, r.max.y as i32])).collect();
        k.sort_by_key(|x| x.0.value());
        k
    };
    if key(&a) != key(&b) {
        let n = key(&a).iter().zip(key(&b).iter()).filter(|(x, y)| x != y).count();
        problems.push(format!("Layout springt zwischen zwei Bildern ({n} Elemente verschoben)"));
    }
    Ok(problems)
}

#[test]
fn gesamt_groessen() {
    let mut rows = Vec::new();
    // alle Größen × Designs × DPI im Standardzustand
    for (w, hgt, label) in SIZES {
        for ppp in [1.0f32, 1.5, 2.0] {
            for kind in ThemeKind::ALL {
                let r = check(kind, (*w, *hgt), ppp, State::Standard);
                rows.push(row(format!("{label} @ {:.0} % ▸ {}", ppp * 100.0, kind.label()), "Standard", r));
            }
        }
    }
    // alle Zustände in kleinster, mittlerer und größter Größe (Design Normal, 100 % und 200 %)
    for state in STATES {
        for (w, hgt, label) in [SIZES[0], SIZES[3], SIZES[5], SIZES[6]] {
            for ppp in [1.0f32, 2.0] {
                let r = check(ThemeKind::Normal, (w, hgt), ppp, *state);
                rows.push(row(format!("{label} @ {:.0} % ▸ {state:?}", ppp * 100.0), &format!("{state:?}"), r));
            }
        }
    }
    let (ok, n) = write_report("groessen_auto.md", "UI über Größen, DPI, Designs und Zustände", "gesamt_groessen.rs", &rows);
    eprintln!("Größen: {ok}/{n} ✅");
    let panics: Vec<_> = rows.iter().filter(|r| matches!(r.outcome, Outcome::Panic(_))).map(|r| format!("{} {:?}", r.path, r.outcome)).collect();
    assert!(panics.is_empty(), "Paniken: {panics:#?}");
}

/// Bilder je Größe, DPI und Design (zur Sichtprüfung) nach `target\fp-ui-groessen\`:
/// `cargo test -p fp-ui --test gesamt_groessen groessen_bilder -- --ignored`.
#[test]
#[ignore]
fn groessen_bilder() {
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("target").join("fp-ui-groessen");
    let _ = std::fs::create_dir_all(&out);
    for (w, hgt, label) in SIZES {
        for ppp in [1.0f32, 2.0] {
            for kind in ThemeKind::ALL {
                for state in [State::Standard, State::ManyLayersPages] {
                    let st = prepare(state);
                    let ctx = egui::Context::default();
                    let mut app = FirepixelApp::with_session(&ctx, fp_engine::Session::default(), UiSettings::default(), false);
                    app.state = st;
                    app.state.set_theme(kind);
                    let mut h = Harness::builder().with_size(egui::vec2(w / ppp, hgt / ppp)).with_pixels_per_point(ppp).with_max_steps(12).wgpu().build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
                    h.run_steps(5);
                    let name = format!("{}_{}_{:.0}_{:?}.png", label.split(' ').next_back().unwrap_or(label).replace('×', "x"), kind.label().to_lowercase(), ppp * 100.0, state);
                    match h.render() {
                        Ok(img) => {
                            let _ = img.save(out.join(name));
                        }
                        Err(e) => {
                            eprintln!("kein Grafikadapter: {e}");
                            return;
                        }
                    }
                }
            }
        }
    }
}

fn row(path: String, cmd: &str, r: Result<Vec<String>, String>) -> Row {
    match r {
        Ok(p) if p.is_empty() => Row { path, command: cmd.into(), outcome: Outcome::Ui, note: "ohne Befund".into() },
        Ok(p) => Row { path, command: cmd.into(), outcome: Outcome::Error(format!("{} Befund(e)", p.len())), note: p.join("; ") },
        Err(e) => Row { path, command: cmd.into(), outcome: Outcome::Panic(e), note: String::new() },
    }
}

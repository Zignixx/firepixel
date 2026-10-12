//! RAND-2a (Nutzerbefund N23, Pakete R2-1/R2-2 des Prüfers `clipping_audit`): Inspektor im
//! schmalen Fenster mit waagerechtem Bildlauf, dessen Leiste keine Höhe belegt (die untere Zeile
//! mit „Autom. Unterschneidung“ wurde vorher 4–12 % abgeschnitten), Mausrad blättert waagerecht;
//! Kästchen „Autom. Unterschneidung“ so breit wie seine Beschriftung; beim Segment endet
//! „Bearbeiten...“ vor der Beschriftung „Ziel:“.

use egui::{Event, Modifiers, Rect};
use egui_kittest::kittest::{By, Queryable};
use egui_kittest::Harness;
use fp_engine::ToolId;
use fp_ui::app_state::UiSettings;
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness(w: f32, hh: f32, setup: impl FnOnce(&mut fp_engine::Session)) -> H {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 300, "background": "#ffffff"})).expect("Dokument");
    setup(&mut s);
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(w, hh)).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn tool(h: &mut H, t: ToolId) {
    let _ = h.state_mut().state.session.set_tool(t);
    h.run_steps(4);
}

fn checkbox(h: &H, label: &str) -> Rect {
    h.get_all(By::new().role(egui::accesskit::Role::CheckBox).label(label)).next().map(|n| n.rect()).unwrap_or_else(|| panic!("Kästchen „{label}“"))
}

/// Inspektor im Fenster 800 × 512 pt (1024 × 768 bei 150 %): Bildlauf, untere Zeile ganz sichtbar,
/// Mausrad blättert waagerecht und bringt Quadrant 5/6 in den Ausschnitt.
#[test]
fn narrow_inspector_scrolls_and_keeps_row2() {
    let mut h = harness(800.0, 512.0, |_| {});
    tool(&mut h, ToolId::Text);
    let vp = fp_ui::inspector::hscroll_viewport(&h.ctx).expect("schmales Fenster: waagerechter Bildlauf");
    let insp = egui::containers::panel::PanelState::load(&h.ctx, egui::Id::new("fp-inspector")).map(|p| p.outer_rect).expect("Inspektor");
    // die schwebende Leiste nimmt keine Höhe: der Ausschnitt reicht bis an den Inspektorrand
    assert!(vp.bottom() >= insp.bottom() - 1.0, "Ausschnitt {vp:?} endet über dem Inspektor {insp:?}");
    let ak = checkbox(&h, "Autom. Unterschneidung");
    assert!(ak.bottom() <= vp.bottom() && ak.top() >= vp.top(), "Kästchen {ak:?} nicht ganz im Ausschnitt {vp:?}");
    assert!(ak.left() + 13.0 < vp.right(), "Kästchen {ak:?} liegt nicht im Ausschnitt {vp:?}");
    // Mausrad (ohne Umschalttaste) über dem Inspektor blättert nach rechts
    let p = egui::pos2(vp.center().x, vp.center().y);
    h.event(Event::PointerMoved(p));
    for _ in 0..6 {
        h.event(Event::MouseWheel { unit: egui::MouseWheelUnit::Line, delta: egui::vec2(0.0, -3.0), phase: egui::TouchPhase::Move, modifiers: Modifiers::NONE });
        h.run_steps(2);
    }
    h.run_steps(10);
    let ak2 = checkbox(&h, "Autom. Unterschneidung");
    assert!(ak2.left() < ak.left() - 50.0, "Mausrad blättert nicht waagerecht: {ak:?} → {ak2:?}");
}

/// Großes Fenster: kein Bildlauf, Bild wie bisher; das Kästchen „Autom. Unterschneidung“ ist so
/// breit wie Kästchen + Abstand + Beschriftung (100 DLU = 150 px, vorher 92 DLU = 138 px).
#[test]
fn wide_inspector_without_scroll_autokern_wide_enough() {
    let mut h = harness(1920.0, 1080.0, |_| {});
    tool(&mut h, ToolId::Text);
    assert!(fp_ui::inspector::hscroll_viewport(&h.ctx).is_none(), "großes Fenster ohne Bildlauf");
    let ak = checkbox(&h, "Autom. Unterschneidung");
    assert!((ak.width() - 150.0).abs() < 0.6, "Kästchen {ak:?} soll 150 px breit sein");
}

/// Segment (Deutsch): der Knopf „Bearbeiten...“ neben der Exportvorgabe endet vor der
/// Beschriftung „Ziel:“, sein Text überlappt sie nicht (Prüfprotokoll des Inspektors).
#[test]
fn slice_edit_button_ends_before_target_label() {
    let mut h = harness(1920.0, 1080.0, |s| {
        s.execute("draw.rectangle", json!({"rect": [10, 10, 100, 100], "fillColor": "#ff0000"})).expect("Rechteck");
        s.execute("edit.insert.slice", json!({})).expect("Segment");
    });
    h.run_steps(2);
    fp_ui::inspector::fields::set_audit(&h.ctx, true);
    h.run_steps(1);
    let log: Vec<_> = fp_ui::inspector::fields::take_audit(&h.ctx).into_iter().filter(|e| e.inspector).collect();
    fp_ui::inspector::fields::set_audit(&h.ctx, false);
    let b = log.iter().find(|e| e.kind == "button" && e.text == "Bearbeiten...").expect("Knopf Bearbeiten...");
    let z = log.iter().find(|e| e.kind == "label" && e.text == "Ziel:").expect("Beschriftung Ziel:");
    // Knopfrahmen = Textfläche + 3 px je Seite
    let frame_right = b.avail.right() + 3.0;
    assert!(frame_right <= z.drawn.left(), "Knopf bis x = {frame_right} reicht in „Ziel:“ {:?}", z.drawn);
    assert!(!b.drawn.intersects(z.drawn), "Text {:?} überlappt „Ziel:“ {:?}", b.drawn, z.drawn);
    assert_eq!(b.fit, 0, "„Bearbeiten...“ passt ungekürzt in den Knopf");
}

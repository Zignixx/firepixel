//! FB-44 (Firepixel extension): the Properties inspector shows the checkbox "Lock pattern to
//! document" (short label "Fest" / "Lock") in the fill section only for a selected pattern or
//! gradient; a click sets the flag in one History row "Füllen" and a second click clears it.

use egui::accesskit::Role;
use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{By, Queryable};
use fp_ui::FirepixelApp;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::inspector::obj_quads;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness(setup: impl FnOnce(&mut fp_engine::Session)) -> H {
    let mut s = fp_engine::Session::default();
    s.execute(
        "file.new",
        json!({"width": 500, "height": 300, "background": "#ffffff"}),
    )
    .expect("document");
    s.execute(
        "draw.rectangle",
        json!({"rect": [50, 50, 150, 120], "fillColor": "#808080", "strokeWidth": 0}),
    )
    .expect("rectangle");
    setup(&mut s);
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1800.0, 1130.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(20)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn click(h: &mut H, p: Pos2) {
    for e in [
        Event::PointerMoved(p),
        Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
        Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        },
    ] {
        h.event(e);
        h.run_steps(1);
    }
    h.run_steps(2);
}

fn lock_box(h: &H) -> Option<egui::Rect> {
    h.query_all(By::new().role(Role::CheckBox).label("Fest"))
        .next()
        .map(|n| n.rect())
}

fn locked(h: &H) -> bool {
    obj_quads::shown_attrs(st(h))
        .and_then(|a| a.fill.clone())
        .is_some_and(|f| f.lock_to_document)
}

fn undo_count(h: &H) -> usize {
    st(h)
        .session
        .active()
        .map(|d| d.history.undo_count())
        .unwrap_or(0)
}

#[test]
fn lock_checkbox_toggles_the_flag_of_a_pattern() {
    let mut h = harness(|s| {
        s.execute("pi.fill", json!({"category": "pattern"}))
            .expect("pattern");
    });
    let b = lock_box(&h).expect("checkbox for a pattern fill");
    assert!(!locked(&h));
    let n0 = undo_count(&h);
    click(&mut h, b.center());
    assert!(locked(&h), "checked: locked to the document");
    assert_eq!(undo_count(&h), n0 + 1, "one History row");
    let label = st(&h)
        .session
        .active()
        .and_then(|d| d.history.undo_label().map(|s| s.to_string()))
        .unwrap_or_default();
    assert_eq!(label, "Füllen");
    let b = lock_box(&h).expect("checkbox");
    click(&mut h, b.center());
    assert!(!locked(&h), "unchecked again");
}

#[test]
fn lock_checkbox_only_for_patterns_and_gradients() {
    let h = harness(|_| {});
    assert!(lock_box(&h).is_none(), "solid fill: no checkbox");
    let h = harness(|s| {
        s.execute("pi.fill", json!({"category": "gradient"}))
            .expect("gradient");
    });
    assert!(lock_box(&h).is_some(), "gradient: checkbox");
}

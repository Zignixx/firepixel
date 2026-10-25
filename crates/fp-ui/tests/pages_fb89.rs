//! FB-89 (Firepixel extension): dragging one of several selected rows of the Pages panel moves
//! all selected pages as a block (order kept, one History row "Move Pages", a label with the
//! count beside the pointer); dragging an unselected row moves only that page.

use egui::{Event, Modifiers, PointerButton, Pos2, Rect, Shape};
use egui_kittest::Harness;
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use fp_ui::panels::PanelId;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

/// Pages P1…P4, P1 current, P3 added to the selection.
fn harness() -> H {
    fp_i18n::set_language("de");
    let mut s = fp_engine::Session::default();
    s.execute(
        "file.new",
        json!({"width": 400, "height": 300, "background": "#ffffff"}),
    )
    .expect("doc");
    for _ in 0..3 {
        s.execute("page.add", json!({})).expect("page");
    }
    for (i, n) in ["P1", "P2", "P3", "P4"].iter().enumerate() {
        s.execute("page.rename", json!({"index": i, "name": n}))
            .expect("name");
    }
    s.execute("page.setCurrent", json!({"index": 0}))
        .expect("current");
    s.execute("page.select", json!({"index": 2, "mode": "toggle"}))
        .expect("select");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    app.state.settings.workspace.show(PanelId::Pages);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1600.0, 1000.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(20)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn button(h: &mut H, p: Pos2, down: bool) {
    ev(
        h,
        Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed: down,
            modifiers: Modifiers::NONE,
        },
    );
}

fn painted_texts(h: &H) -> Vec<(String, Rect)> {
    fn walk(s: &Shape, out: &mut Vec<(String, Rect)>) {
        match s {
            Shape::Text(t) => out.push((t.galley.text().to_string(), t.visual_bounding_rect())),
            Shape::Vec(v) => v.iter().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let mut v = Vec::new();
    for cs in &h.output().shapes {
        walk(&cs.shape, &mut v);
    }
    v
}

/// Rectangle of the Pages panel row label ("02  P2") ending in `name`.
fn row(h: &H, name: &str) -> Rect {
    painted_texts(h)
        .into_iter()
        .find(|(t, _)| {
            t.len() > name.len()
                && t.ends_with(name)
                && t[..t.len() - name.len()].trim().parse::<u32>().is_ok()
        })
        .map(|(_, r)| r)
        .unwrap_or_else(|| panic!("row {name}"))
}

/// Drags from `from` to `to`; returns whether `ghost` was painted while dragging.
fn drag(h: &mut H, from: Pos2, to: Pos2, ghost: &str) -> bool {
    ev(h, Event::PointerMoved(from));
    button(h, from, true);
    let mut seen = false;
    for i in 1..=12 {
        ev(
            h,
            Event::PointerMoved(from + (to - from) * (i as f32 / 12.0)),
        );
        seen |= painted_texts(h).iter().any(|(t, _)| t == ghost);
    }
    button(h, to, false);
    h.run_steps(3);
    seen
}

fn names(h: &H) -> Vec<String> {
    h.state()
        .state
        .session
        .active()
        .expect("doc")
        .doc
        .pages
        .iter()
        .map(|p| p.display_name().to_string())
        .collect()
}

fn labels(h: &H) -> Vec<String> {
    let s = &h.state().state.session;
    let d = s.active().expect("doc");
    d.history.steps()[..d.history.undo_count()]
        .iter()
        .map(|st| s.step_label(st))
        .collect()
}

#[test]
fn dragging_a_selected_row_moves_all_selected_pages_as_a_block() {
    let mut h = harness();
    let before = labels(&h).len();
    let p1 = row(&h, "P1");
    let p4 = row(&h, "P4");
    // P1 (selected, current) below P4: P1 and P3 move together, in their order
    let ghost = drag(
        &mut h,
        p1.center(),
        Pos2::new(p1.center().x, p4.bottom() + 14.0),
        "2 Seiten",
    );
    assert_eq!(names(&h), ["P2", "P4", "P1", "P3"]);
    assert!(ghost, "the drag label shows the count");
    assert_eq!(
        labels(&h)[before..],
        ["Seiten verschieben"],
        "one History row"
    );
    let d = h.state().state.session.active().expect("doc");
    assert_eq!(
        d.doc.pages[d.page].display_name(),
        "P1",
        "the current page stays current"
    );
    assert_eq!(d.selected_pages(), [2, 3]);
    h.state_mut()
        .state
        .session
        .execute("edit.undo", json!({}))
        .expect("undo");
    h.run_steps(2);
    assert_eq!(names(&h), ["P1", "P2", "P3", "P4"]);
}

#[test]
fn dragging_an_unselected_row_moves_only_that_page() {
    let mut h = harness();
    let before = labels(&h).len();
    let p1 = row(&h, "P1");
    let p4 = row(&h, "P4");
    let ghost = drag(
        &mut h,
        p4.center(),
        Pos2::new(p4.center().x, p1.top() - 4.0),
        "2 Seiten",
    );
    assert_eq!(names(&h), ["P4", "P1", "P2", "P3"]);
    assert!(!ghost);
    assert_eq!(labels(&h)[before..], ["Seite verschieben"]);
}

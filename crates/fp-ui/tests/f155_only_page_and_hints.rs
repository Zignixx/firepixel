//! F155 (FB-145, FB-147; TW-289, TW-290): Delete Page on the only page asks first ("… replaced
//! with a brand new page instead. Proceed?", OK / Cancel); a page without a stored name (a flat
//! image opened as a document) is listed as "Page 1" instead of a bare "01"; the Subselection
//! shows its hollow (white) arrow on the canvas; Subselection and Mask Element Selection share
//! their keys as a cycling group, so both flyout rows read "(A, 1)".

use egui::accesskit::Role;
use egui::{Event, Modifiers, PointerButton, Pos2, Rect, Shape};
use egui_kittest::Harness;
use egui_kittest::kittest::{By, Queryable};
use fp_engine::tools::{Cursor, ToolId};
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use fp_ui::panels::PanelId;
use serde_json::json;
use std::sync::Arc;

type H = Harness<'static, FirepixelApp>;

/// One page "Only" with a rectangle, Pages panel shown, English UI.
fn harness() -> H {
    fp_i18n::set_language("en");
    let mut s = fp_engine::Session::default();
    s.execute(
        "file.new",
        json!({"width": 400, "height": 300, "background": "#ffffff"}),
    )
    .expect("doc");
    s.execute("draw.rectangle", json!({"rect": [100, 80, 200, 160]}))
        .expect("rect");
    s.execute("page.rename", json!({"index": 0, "name": "Only"}))
        .expect("name");
    harness_of(s)
}

fn harness_of(s: fp_engine::Session) -> H {
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

fn click_at(h: &mut H, p: Pos2) {
    h.event(Event::PointerMoved(p));
    h.run_steps(1);
    for down in [true, false] {
        h.event(Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed: down,
            modifiers: Modifiers::NONE,
        });
        h.run_steps(1);
    }
    h.run_steps(3);
}

fn names(h: &H) -> Vec<String> {
    let d = h.state().state.session.active().expect("doc");
    d.doc
        .pages
        .iter()
        .map(|p| p.display_name().to_string())
        .collect()
}

fn elements(h: &H) -> usize {
    let d = h.state().state.session.active().expect("doc");
    fp_engine::docops::own_layers(&d.doc, 0)
        .into_iter()
        .map(|(_, l)| l.cels.first().map_or(0, |c| c.elements.len()))
        .sum()
}

fn delete_button(h: &H) -> Rect {
    let tip = fp_i18n::t("panel-pages-delete");
    h.query_all(By::new().label(tip.as_str()))
        .map(|n| n.rect())
        .find(|r| r.width() < 40.0)
        .expect("Delete Page button")
}

fn dialog_button(h: &H, label: &str) -> Rect {
    h.query_all(By::new().role(Role::Button).label(label))
        .map(|n| n.rect())
        .find(|r| r.width() > 40.0)
        .unwrap_or_else(|| panic!("button {label}"))
}

fn painted_texts(h: &H) -> Vec<String> {
    fn walk(s: &Shape, out: &mut Vec<String>) {
        match s {
            Shape::Text(t) => out.push(t.galley.text().to_string()),
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

#[test]
fn deleting_the_only_page_asks_first() {
    let mut h = harness();
    let b = delete_button(&h).center();
    click_at(&mut h, b);
    {
        let w = h
            .state()
            .state
            .dialogs
            .warning
            .as_ref()
            .expect("question before deleting the only page");
        assert_eq!(
            w.warning.text,
            "Attempting to delete the only page in the file will have it replaced with a brand \
             new page instead. Proceed?"
        );
    }
    assert_eq!(names(&h), ["Only"], "nothing deleted before the answer");
    // Cancel keeps the page
    let cancel = dialog_button(&h, "Cancel").center();
    click_at(&mut h, cancel);
    assert!(h.state().state.dialogs.warning.is_none());
    assert_eq!(names(&h), ["Only"]);
    assert_eq!(elements(&h), 1);
    // OK replaces it with an empty "Page 1" in one History row
    click_at(&mut h, b);
    let ok = dialog_button(&h, "OK").center();
    click_at(&mut h, ok);
    assert!(h.state().state.dialogs.warning.is_none());
    assert_eq!(names(&h), ["Page 1"]);
    assert_eq!(elements(&h), 0);
    // Undo brings the page back with its name and content
    h.state_mut()
        .state
        .session
        .execute("edit.undo", json!({}))
        .expect("undo");
    h.run_steps(3);
    assert_eq!(names(&h), ["Only"], "undo restores the page name");
    assert_eq!(elements(&h), 1);
}

#[test]
fn several_pages_delete_without_the_question() {
    let mut h = harness();
    h.state_mut()
        .state
        .session
        .execute("page.add", json!({}))
        .expect("page");
    h.run_steps(3);
    let b = delete_button(&h).center();
    click_at(&mut h, b);
    assert!(h.state().state.dialogs.warning.is_none());
    assert_eq!(names(&h).len(), 1);
}

#[test]
fn a_page_without_a_stored_name_is_listed_as_page_1() {
    fp_i18n::set_language("en");
    let mut s = fp_engine::Session::default();
    s.execute(
        "file.new",
        json!({"width": 400, "height": 300, "background": "#ffffff"}),
    )
    .expect("doc");
    {
        let d = s.active_mut().expect("doc");
        let doc = Arc::make_mut(&mut d.doc);
        Arc::make_mut(&mut doc.pages[0]).name = None;
    }
    let mut h = harness_of(s);
    h.run_steps(10);
    let texts = painted_texts(&h);
    assert!(
        texts.iter().any(|t| t == "01  Page 1"),
        "row with the default name: {texts:?}"
    );
}

#[test]
fn subselection_shows_the_hollow_arrow_on_the_canvas() {
    fp_i18n::set_language("en");
    let mut s = fp_engine::Session::default();
    s.execute(
        "file.new",
        json!({"width": 400, "height": 300, "background": "#ffffff"}),
    )
    .expect("doc");
    s.execute("draw.rectangle", json!({"rect": [100, 80, 200, 160]}))
        .expect("rect");
    s.execute("select.set", json!({"ids": [], "mode": "replace"}))
        .expect("none");
    let mut h = harness_of(s);
    let hover = |h: &mut H, x: f32, y: f32| {
        let st = &h.state().state;
        let d = st.session.active().expect("doc");
        let area = st.canvases.get(&d.id).and_then(|c| c.area).expect("canvas");
        let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(x, y));
        let p = Pos2::new(area.min.x + sx as f32, area.min.y + sy as f32);
        h.event(Event::PointerMoved(p));
        h.run_steps(2);
        h.state().state.tool_cursor
    };
    h.state_mut()
        .state
        .session
        .set_tool(ToolId::Pointer)
        .expect("tool");
    h.run_steps(3);
    assert_eq!(hover(&mut h, 300.0, 250.0), Cursor::Arrow, "Pointer: black");
    h.state_mut()
        .state
        .session
        .set_tool(ToolId::Subselection)
        .expect("tool");
    h.run_steps(3);
    assert_eq!(
        hover(&mut h, 302.0, 250.0),
        Cursor::Subselect,
        "Subselection over empty canvas: hollow arrow"
    );
    assert_eq!(
        hover(&mut h, 150.0, 120.0),
        Cursor::Subselect,
        "Subselection over an object: hollow arrow"
    );
}

#[test]
fn subselection_and_mask_element_selection_share_their_keys() {
    let h = harness();
    let st = &h.state().state;
    assert_eq!(
        fp_ui::tools_panel::tool_title(st, ToolId::Subselection),
        "Subselection (A, 1)"
    );
    assert_eq!(
        fp_ui::tools_panel::tool_title(st, ToolId::MaskElementSelect),
        "Mask Element Selection (A, 1)",
        "one cycling group like Pointer / Select Behind (V, 0)"
    );
    assert_eq!(
        fp_ui::tools_panel::tool_title(st, ToolId::Pointer),
        "Pointer (V, 0)"
    );
}

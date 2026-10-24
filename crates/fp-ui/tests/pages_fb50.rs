//! FB-50 (RE spec 70 §2): the Pages panel routes of New Page and Duplicate Page with real input.
//! The footer button, the panel menu and the row context menu insert the new page directly after
//! the current page; a right-click makes the row current first (History row "Change Page
//! Selection"); dragging a row onto the New Page button duplicates that page right after it.

use egui::accesskit::Role;
use egui::{Event, Modifiers, PointerButton, Pos2, Rect, Shape};
use egui_kittest::Harness;
use egui_kittest::kittest::{By, Queryable};
use fp_ui::FirepixelApp;
use fp_ui::app_state::{Action, UiSettings};
use fp_ui::menu::{ItemAction, Node};
use fp_ui::panels::PanelId;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

/// Pages P1, P2, P3 with page `cur` current, Pages panel shown.
fn harness(cur: usize) -> H {
    fp_i18n::set_language("de");
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 400, "height": 300, "background": "#ffffff"})).expect("doc");
    s.execute("page.add", json!({})).expect("page");
    s.execute("page.add", json!({})).expect("page");
    for (i, n) in ["P1", "P2", "P3"].iter().enumerate() {
        s.execute("page.rename", json!({"index": i, "name": n})).expect("name");
    }
    s.execute("page.setCurrent", json!({"index": cur})).expect("current");
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
    assert!(try_row(&h, "P2").is_some(), "Pages panel shows the rows");
    h
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn button(h: &mut H, p: Pos2, b: PointerButton, down: bool) {
    ev(h, Event::PointerButton { pos: p, button: b, pressed: down, modifiers: Modifiers::NONE });
}

fn click(h: &mut H, p: Pos2, b: PointerButton) {
    ev(h, Event::PointerMoved(p));
    button(h, p, b, true);
    button(h, p, b, false);
    h.run_steps(3);
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

/// Centre of the Pages panel row whose label (with numbering "02  P2") ends in `name`.
fn try_row(h: &H, name: &str) -> Option<Pos2> {
    painted_texts(h)
        .into_iter()
        .find(|(t, _)| t.len() > name.len() && t.ends_with(name) && t[..t.len() - name.len()].trim().parse::<u32>().is_ok())
        .map(|(_, r)| r.center())
}

fn row(h: &H, name: &str) -> Pos2 {
    try_row(h, name).unwrap_or_else(|| panic!("row {name}"))
}

/// The New Page footer button (small icon with that tooltip).
fn new_button(h: &H) -> Rect {
    let tip = fp_i18n::t("panel-pages-new");
    h.query_all(By::new().label(tip.as_str()))
        .map(|n| n.rect())
        .find(|r| r.width() < 40.0)
        .expect("New Page button")
}

/// An entry of the open context menu (wide button).
fn menu_item(h: &H, label: &str) -> Option<Rect> {
    h.query_all(By::new().role(Role::Button).label(label)).map(|n| n.rect()).find(|r| r.width() > 80.0)
}

fn names(h: &H) -> Vec<String> {
    h.state().state.session.active().expect("doc").doc.pages.iter().map(|p| p.display_name().to_string()).collect()
}

fn current(h: &H) -> usize {
    h.state().state.session.active().expect("doc").page
}

fn labels(h: &H) -> Vec<String> {
    let s = &h.state().state.session;
    let d = s.active().expect("doc");
    d.history.steps()[..d.history.undo_count()].iter().map(|st| s.step_label(st)).collect()
}

fn new_labels(h: &H, before: usize) -> Vec<String> {
    labels(h)[before..].to_vec()
}

#[test]
fn panel_button_inserts_after_the_current_page() {
    for (cur, want) in [
        (0, ["P1", "Seite 1", "P2", "P3"]),
        (1, ["P1", "P2", "Seite 1", "P3"]),
        (2, ["P1", "P2", "P3", "Seite 1"]),
    ] {
        let mut h = harness(cur);
        let before = labels(&h).len();
        let b = new_button(&h).center();
        click(&mut h, b, PointerButton::Primary);
        assert_eq!(names(&h), want, "page {cur} current");
        assert_eq!(current(&h), cur + 1, "the new page becomes current");
        assert_eq!(new_labels(&h, before), ["Seite hinzufügen"], "one History row");
        assert!(try_row(&h, "Seite 1").is_some(), "the panel shows the new row");
    }
}

#[test]
fn panel_menu_new_and_duplicate() {
    let run_item = |h: &mut H, label: &str| {
        let nodes = fp_ui::menu::build_panel_menu(&h.state().state, PanelId::Pages);
        let cmd = nodes
            .iter()
            .find_map(|n| match n {
                Node::Item { label: l, action: ItemAction::Command(c), enabled: true, .. } if l == label => Some(c.clone()),
                _ => None,
            })
            .unwrap_or_else(|| panic!("panel menu entry {label}"));
        h.state_mut().state.push(Action::Run(cmd, json!({})));
        h.run_steps(3);
    };
    let mut h = harness(1);
    let before = labels(&h).len();
    run_item(&mut h, "Neue Seite");
    assert_eq!(names(&h), ["P1", "P2", "Seite 1", "P3"]);
    assert_eq!(current(&h), 2);
    assert_eq!(new_labels(&h, before), ["Seite hinzufügen"]);
    let mut h = harness(1);
    let before = labels(&h).len();
    run_item(&mut h, "Seite duplizieren");
    assert_eq!(names(&h), ["P1", "P2", "P2 Kopie", "P3"]);
    assert_eq!(current(&h), 2);
    assert_eq!(new_labels(&h, before), ["Seite duplizieren"]);
}

#[test]
fn context_menu_selects_the_row_then_acts_on_it() {
    // P2 current, right-click on P3 → New Page: P1 P2 P3 Seite 1; two History rows
    let mut h = harness(1);
    let before = labels(&h).len();
    let p3 = row(&h, "P3");
    click(&mut h, p3, PointerButton::Secondary);
    assert_eq!(current(&h), 2, "the right-click makes P3 current");
    let item = menu_item(&h, "Neue Seite").expect("context menu entry New Page");
    click(&mut h, item.center(), PointerButton::Primary);
    assert_eq!(names(&h), ["P1", "P2", "P3", "Seite 1"]);
    assert_eq!(current(&h), 3);
    assert_eq!(new_labels(&h, before), ["Seitenauswahl ändern", "Seite hinzufügen"]);
    // P2 current, right-click on P3 → Duplicate Page: the copy after P3
    let mut h = harness(1);
    let before = labels(&h).len();
    let p3 = row(&h, "P3");
    click(&mut h, p3, PointerButton::Secondary);
    let item = menu_item(&h, "Seite duplizieren").expect("context menu entry Duplicate Page");
    click(&mut h, item.center(), PointerButton::Primary);
    assert_eq!(names(&h), ["P1", "P2", "P3", "P3 Kopie"]);
    assert_eq!(current(&h), 3);
    assert_eq!(new_labels(&h, before), ["Seitenauswahl ändern", "Seite duplizieren"]);
}

#[test]
fn dragging_a_row_onto_the_new_page_button_duplicates_it() {
    let mut h = harness(1);
    let before = labels(&h).len();
    let from = row(&h, "P3");
    let to = new_button(&h).center();
    ev(&mut h, Event::PointerMoved(from));
    button(&mut h, from, PointerButton::Primary, true);
    for i in 1..=12 {
        ev(&mut h, Event::PointerMoved(from + (to - from) * (i as f32 / 12.0)));
    }
    button(&mut h, to, PointerButton::Primary, false);
    h.run_steps(3);
    assert_eq!(names(&h), ["P1", "P2", "P3", "P3 Kopie"], "copy directly after the dragged page");
    assert_eq!(current(&h), 3, "the copy becomes current");
    assert_eq!(new_labels(&h, before), ["Seitenauswahl ändern", "Seite duplizieren"]);
}

#[test]
fn plain_click_on_a_row_is_a_history_row() {
    let mut h = harness(0);
    let before = labels(&h).len();
    let p3 = row(&h, "P3");
    click(&mut h, p3, PointerButton::Primary);
    assert_eq!(current(&h), 2);
    assert_eq!(new_labels(&h, before), ["Seitenauswahl ändern"]);
}

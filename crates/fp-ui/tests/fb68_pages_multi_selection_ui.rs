//! FB-68 (RE spec 71 §4) with real input in the Pages panel: Shift+click selects the range from
//! the current page, Ctrl+click adds or removes a page (each a History row "Change Page
//! Selection"), Delete in the list removes the selected pages in one row; with every page
//! selected the Delete button opens "Delete all pages?" (Firepixel extension) with the first page
//! as the default page that stays.

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect, Shape};
use egui_kittest::Harness;
use egui_kittest::kittest::{By, Queryable};
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use fp_ui::panels::PanelId;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

/// Pages P1…P4, P1 current, Pages panel shown, English UI.
fn harness() -> H {
    fp_i18n::set_language("en");
    let mut s = fp_engine::Session::default();
    s.execute(
        "file.new",
        json!({"width": 400, "height": 300, "background": "#ffffff"}),
    )
    .expect("doc");
    for _ in 0..3 {
        s.execute("page.add", json!({})).expect("page");
    }
    for i in 0..4 {
        s.execute(
            "page.rename",
            json!({"index": i, "name": format!("P{}", i + 1)}),
        )
        .expect("name");
    }
    s.execute("page.setCurrent", json!({"index": 0}))
        .expect("current");
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

fn click(h: &mut H, name: &str, modifiers: Modifiers) {
    h.run_steps(30);
    let p = row(h, name);
    h.input_mut()
        .events
        .push(Event::ModifiersChanged(modifiers));
    ev(h, Event::PointerMoved(p));
    let e = |down| Event::PointerButton {
        pos: p,
        button: PointerButton::Primary,
        pressed: down,
        modifiers,
    };
    ev(h, e(true));
    ev(h, e(false));
    h.input_mut()
        .events
        .push(Event::ModifiersChanged(Modifiers::NONE));
    h.run_steps(5);
}

fn click_at(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    for down in [true, false] {
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
fn row(h: &H, name: &str) -> Pos2 {
    painted_texts(h)
        .into_iter()
        .find(|(t, _)| {
            t.len() > name.len()
                && t.ends_with(name)
                && t[..t.len() - name.len()].trim().parse::<u32>().is_ok()
        })
        .map(|(_, r)| r.center())
        .unwrap_or_else(|| panic!("row {name}"))
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

fn selected(h: &H) -> Vec<usize> {
    h.state()
        .state
        .session
        .active()
        .expect("doc")
        .selected_pages()
}

fn labels(h: &H) -> Vec<String> {
    let s = &h.state().state.session;
    let d = s.active().expect("doc");
    d.history.steps()[..d.history.undo_count()]
        .iter()
        .map(|st| s.step_label(st))
        .collect()
}

/// The Delete Page footer button (small icon with that tooltip).
fn delete_button(h: &H) -> Rect {
    let tip = fp_i18n::t("panel-pages-delete");
    h.query_all(By::new().label(tip.as_str()))
        .map(|n| n.rect())
        .find(|r| r.width() < 40.0)
        .expect("Delete Page button")
}

#[test]
fn shift_and_ctrl_click_select_pages_and_delete_key_removes_them() {
    let mut h = harness();
    let before = labels(&h).len();
    click(&mut h, "P3", Modifiers::SHIFT);
    assert_eq!(
        selected(&h),
        [0, 1, 2],
        "Shift: range from the current page"
    );
    assert_eq!(
        h.state().state.session.active().expect("doc").page,
        0,
        "P1 stays current"
    );
    click(&mut h, "P2", Modifiers::COMMAND);
    assert_eq!(selected(&h), [0, 2], "Ctrl removes P2");
    click(&mut h, "P4", Modifiers::COMMAND);
    assert_eq!(selected(&h), [0, 2, 3], "Ctrl adds P4");
    assert_eq!(
        labels(&h)[before..],
        ["Change Page Selection"; 3],
        "every click is a History row"
    );
    // Delete in the focused page list: P1, P3 and P4 go in one row
    let before = labels(&h).len();
    ev(
        &mut h,
        Event::Key {
            key: Key::Delete,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        },
    );
    ev(
        &mut h,
        Event::Key {
            key: Key::Delete,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: Modifiers::NONE,
        },
    );
    h.run_steps(3);
    assert_eq!(names(&h), ["P2"]);
    assert_eq!(labels(&h)[before..], ["Delete Page"]);
}

#[test]
fn deleting_all_pages_asks_which_page_stays() {
    let mut h = harness();
    click(&mut h, "P4", Modifiers::SHIFT);
    assert_eq!(selected(&h), [0, 1, 2, 3]);
    let b = delete_button(&h).center();
    click_at(&mut h, b);
    assert_eq!(
        names(&h),
        ["P1", "P2", "P3", "P4"],
        "nothing deleted before the answer"
    );
    {
        let f = h
            .state()
            .state
            .dialogs
            .form
            .as_ref()
            .expect("Delete all pages? dialog");
        assert_eq!(f.id, "pages-delete-all");
        assert_eq!(f.title, "Delete all pages?");
        assert_eq!(f.get("keep"), json!(0), "default: the first page stays");
    }
    // choose P3 and press Delete
    if let Some(f) = h.state_mut().state.dialogs.form.as_mut()
        && let Some(k) = f.field_mut("keep")
    {
        k.value = json!(2);
    }
    h.run_steps(3);
    let ok = h
        .query_all(By::new().role(Role::Button).label("Delete"))
        .map(|n| n.rect())
        .find(|r| r.width() > 40.0)
        .expect("Delete button of the dialog");
    click_at(&mut h, ok.center());
    assert_eq!(names(&h), ["P3"], "the chosen page stays");
    assert!(h.state().state.dialogs.form.is_none());
    // Cancel keeps every page
    let mut h = harness();
    click(&mut h, "P4", Modifiers::SHIFT);
    let b = delete_button(&h).center();
    click_at(&mut h, b);
    let cancel = h
        .query_all(By::new().role(Role::Button).label("Cancel"))
        .map(|n| n.rect())
        .next()
        .expect("Cancel");
    click_at(&mut h, cancel.center());
    assert_eq!(names(&h), ["P1", "P2", "P3", "P4"]);
}

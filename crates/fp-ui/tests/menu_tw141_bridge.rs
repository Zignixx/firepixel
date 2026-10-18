//! TW-141: menu rows expose their enabled and toggled state to the bridge (`ui_find`): Snap to
//! Pixel in the Modify menu is grey without a selection and enabled with a rectangle selected, a
//! checkable View-menu row reports `toggled`.

use egui::{Event, Id, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::Harness;
use fp_ui::app_state::UiSettings;
use fp_ui::menu::{self, ItemAction, Node};
use fp_ui::FirepixelApp;
use serde_json::{json, Value};

type H = Harness<'static, FirepixelApp>;

fn harness(select_rect: bool) -> H {
    let ctx = egui::Context::default();
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    let id = s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 100], "name": "R1"})).expect("rect").value["id"].as_u64().expect("id");
    let ids: Vec<u64> = if select_rect { vec![id] } else { vec![] };
    s.execute("select.set", json!({ "ids": ids })).expect("selection");
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1600.0, 1000.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h
}

fn click(h: &mut H, p: Pos2) {
    for e in [
        Event::PointerMoved(p),
        Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE },
        Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE },
    ] {
        h.event(e);
        h.run_steps(1);
    }
    h.run_steps(2);
}

/// First main menu with a top-level row matching `pick`: (menu index, row label).
fn find_row(h: &H, pick: impl Fn(&ItemAction, Option<bool>) -> bool) -> (usize, String) {
    let menus = menu::build_main_menu(&h.state().state);
    for (i, (_, nodes)) in menus.iter().enumerate() {
        for n in nodes {
            if let Node::Item { label, action, checked, .. } = n
                && pick(action, *checked)
            {
                return (i, label.clone());
            }
        }
    }
    panic!("no matching menu row");
}

fn open_menu(h: &mut H, i: usize) {
    let titles: Vec<Rect> = h.ctx.data(|d| d.get_temp(Id::new(menu::MENU_TITLES_ID))).expect("menu titles");
    click(h, titles[i].center());
    assert!(egui::Popup::is_id_open(&h.ctx, Id::new(("fp-main-menu", i))), "menu {i} open");
}

/// `ui_find` (bridge search) for `label` on one more frame of the open menu. The harness hands
/// the AccessKit update of its own frames to kittest, so this frame runs on the same context
/// (AccessKit on, like the bridge turns it on for a `ui_find`) and keeps its full output.
fn row_hit(h: &mut H, label: &str) -> Value {
    let ctx = h.ctx.clone();
    let raw = egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1600.0, 1000.0))), ..Default::default() };
    let app = h.state_mut();
    let out = ctx.run_ui(raw, |ui| app.root_ui(ui));
    assert!(out.platform_output.accesskit_update.is_some(), "AccessKit on");
    let hits = fp_ui::bridge::find_in_output(&json!({"text": label, "exact": true}), &out);
    hits.iter().find(|v| v["source"] == "accesskit").cloned().unwrap_or_else(|| panic!("„{label}“ as accessibility node: {hits:?}"))
}

fn is_snap(a: &ItemAction, _: Option<bool>) -> bool {
    matches!(a, ItemAction::Command(c) if c == "modify.snapToPixel")
}

#[test]
fn snap_to_pixel_disabled_without_selection() {
    let mut h = harness(false);
    let (i, label) = find_row(&h, is_snap);
    open_menu(&mut h, i);
    let hit = row_hit(&mut h, &label);
    assert_eq!(hit["enabled"], json!(false), "{hit}");
    assert!(hit.get("toggled").is_none(), "{hit}");
}

#[test]
fn snap_to_pixel_enabled_with_rectangle() {
    let mut h = harness(true);
    let (i, label) = find_row(&h, is_snap);
    open_menu(&mut h, i);
    let hit = row_hit(&mut h, &label);
    assert_eq!(hit["enabled"], json!(true), "{hit}");
}

#[test]
fn view_menu_checkable_row_reports_toggled() {
    let mut h = harness(false);
    let (i, label) = find_row(&h, |a, c| c.is_some() && matches!(a, ItemAction::Command(c) if c.starts_with("view.")));
    let expected = {
        let menus = menu::build_main_menu(&h.state().state);
        menus[i].1.iter().find_map(|n| match n {
            Node::Item { label: l, checked: Some(c), .. } if *l == label => Some(*c),
            _ => None,
        })
    }
    .expect("checked state");
    open_menu(&mut h, i);
    let hit = row_hit(&mut h, &label);
    assert_eq!(hit["toggled"], json!(expected), "{hit}");
    assert_eq!(hit["role"], json!("check_box"), "{hit}");
}

// TW-142: panel ≡ menus (and the other pop-up menus drawn by the same row painter) expose the
// same enabled / toggled state as the main menu rows.

/// Opens the ≡ menu of panel `p` (the ≡ button in the row of its tab).
fn open_panel_menu(h: &mut H, p: fp_ui::panels::PanelId) {
    use egui_kittest::kittest::{By, Queryable};
    h.state_mut().state.push(fp_ui::app_state::Action::ShowPanel(p));
    h.run_steps(4);
    let tab = h.query_all(By::new().label(&fp_i18n::t(p.title_key()))).map(|n| n.rect()).next().expect("panel tab");
    let button = h
        .query_all(By::new().label(&fp_i18n::t("ui-dock-panel-menu")))
        .map(|n| n.rect())
        .filter(|r| r.center().y >= tab.top() && r.center().y <= tab.bottom())
        .min_by(|a, b| (a.left() - tab.right()).abs().total_cmp(&(b.left() - tab.right()).abs()))
        .expect("≡ button in the tab row");
    click(h, button.center());
    assert!(egui::Popup::is_any_open(&h.ctx), "{p:?} ≡ menu open");
}

/// Top-level item rows of the ≡ menu of `p`: (label, enabled, checked).
fn panel_rows(h: &H, p: fp_ui::panels::PanelId) -> Vec<(String, bool, Option<bool>)> {
    menu::build_panel_menu(&h.state().state, p)
        .into_iter()
        .filter_map(|n| match n {
            Node::Item { label, enabled, checked, .. } => Some((label, enabled, checked)),
            _ => None,
        })
        .collect()
}

#[test]
fn url_panel_menu_rows_report_enabled_state() {
    use fp_ui::panels::PanelId;
    let mut h = harness(false);
    let rows = panel_rows(&h, PanelId::Url);
    let off = rows.iter().find(|r| !r.1).expect("grey URL menu entry (no URL selected)").0.clone();
    let on = rows.iter().find(|r| r.1).expect("enabled URL menu entry").0.clone();
    open_panel_menu(&mut h, PanelId::Url);
    let hit = row_hit(&mut h, &off);
    assert_eq!(hit["enabled"], json!(false), "{off}: {hit}");
    let hit = row_hit(&mut h, &on);
    assert_eq!(hit["enabled"], json!(true), "{on}: {hit}");
}

#[test]
fn panel_menu_checkable_row_reports_toggled() {
    use fp_ui::panels::PanelId;
    let mut h = harness(true);
    let (p, label, on) = PanelId::ALL
        .iter()
        .find_map(|&p| panel_rows(&h, p).into_iter().find_map(|(l, _, c)| c.map(|c| (p, l, c))))
        .expect("a panel menu with a checkable row");
    open_panel_menu(&mut h, p);
    let hit = row_hit(&mut h, &label);
    assert_eq!(hit["toggled"], json!(on), "{p:?} {label}: {hit}");
    assert_eq!(hit["role"], json!("check_box"), "{p:?} {label}: {hit}");
}

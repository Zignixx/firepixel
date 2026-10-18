//! TW-123: panel context menus and panel drop-down menus size to their content. Before, the
//! colour-table context menu of the Optimize panel took egui's 600 px default (list items fill the
//! available width), while the reference menu is about 270 px wide. Each menu here must be
//! narrower than 1.5 × its widest entry (label + check column + margins).

use egui::{Event, Id, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::Harness;
use fp_ui::FirepixelApp;
use fp_ui::app_state::{Action, UiSettings};
use fp_ui::menu::Node;
use fp_ui::panels::PanelId;
use serde_json::json;

fn harness() -> Harness<'static, FirepixelApp> {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200}))
        .expect("doc");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80]}))
        .expect("rect");
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.sync_render = true;
    Harness::builder()
        .with_size(egui::vec2(1600.0, 1000.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(16)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a)
}

fn press(h: &mut Harness<'_, FirepixelApp>, p: Pos2, button: PointerButton) {
    h.input_mut().events.push(Event::PointerMoved(p));
    for pressed in [true, false] {
        h.input_mut().events.push(Event::PointerButton {
            pos: p,
            button,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    h.run_steps(4);
}

/// Rect of the popup opened from the widget `id` (egui's default popup id).
fn popup_rect(ctx: &egui::Context, id: Id) -> Rect {
    let pid = id.with("popup");
    assert!(egui::Popup::is_id_open(ctx, pid), "menu of {id:?} open");
    ctx.memory(|m| m.area_rect(pid)).expect("menu area")
}

/// Widest entry: check column, label in the menu font, right margin.
fn widest(h: &Harness<'_, FirepixelApp>, labels: &[String]) -> f32 {
    let font = fp_ui::theme::Theme::current(&h.ctx).menu_font();
    labels
        .iter()
        .map(|l| {
            h.ctx.fonts_mut(|f| {
                f.layout_no_wrap(l.clone(), font.clone(), egui::Color32::WHITE)
                    .size()
                    .x
            }) + 40.0
        })
        .fold(0.0, f32::max)
}

fn labels(nodes: &[Node]) -> Vec<String> {
    nodes
        .iter()
        .filter_map(|n| match n {
            Node::Item { label, .. } | Node::Sub { label, .. } => Some(label.clone()),
            Node::Separator => None,
        })
        .collect()
}

fn assert_fits(name: &str, rect: Rect, widest: f32) {
    eprintln!("{name}: menu {:.0} px, widest entry {widest:.0} px", rect.width());
    assert!(
        rect.width() < 1.5 * widest,
        "{name}: menu {:.0} px wide, widest entry {widest:.0} px",
        rect.width()
    );
}

fn show(h: &mut Harness<'_, FirepixelApp>, p: PanelId) {
    h.state_mut().state.push(Action::ShowPanel(p));
    h.run_steps(4);
}

#[test]
fn optimize_colour_table_context_menu_fits_its_entries() {
    fp_i18n::set_language("de");
    let mut h = harness();
    show(&mut h, PanelId::Optimize);
    let table = h
        .ctx
        .read_response(Id::new("opt-table"))
        .expect("colour table")
        .rect;
    press(&mut h, table.center(), PointerButton::Secondary);
    let r = popup_rect(&h.ctx, Id::new("opt-table"));
    let nodes = fp_ui::panels::common::resource_menu_nodes(&h.state().state, "6942");
    let w = widest(&h, &labels(&nodes));
    assert_fits("optimize 6942", r, w);
    // reference (DE): 270 px including the 1 px border
    assert!(
        (r.width() - 270.0).abs() <= 4.0,
        "optimize 6942: {:.0} px, reference 270 px",
        r.width()
    );
}

#[test]
fn panel_drop_down_and_context_menus_fit_their_entries() {
    fp_i18n::set_language("de");
    // Frames panel: disposal context menu 6922 (same helper as in the panel)
    let h = harness();
    let nodes = fp_ui::panels::common::resource_menu_nodes(&h.state().state, "6922");
    let w = widest(&h, &labels(&nodes));
    let mut acts = Vec::new();
    let mut mini = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .with_pixels_per_point(1.0)
        .build_ui(|ui| {
            let resp = ui.interact(
                Rect::from_min_size(Pos2::new(10.0, 10.0), egui::vec2(200.0, 40.0)),
                Id::new("frame-row"),
                egui::Sense::click(),
            );
            fp_ui::panels::common::nodes_context_menu(&resp, &nodes, &mut acts);
        });
    let p = Pos2::new(50.0, 30.0);
    mini.input_mut().events.push(Event::PointerMoved(p));
    for pressed in [true, false] {
        mini.input_mut().events.push(Event::PointerButton {
            pos: p,
            button: PointerButton::Secondary,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    mini.run_steps(4);
    let r = popup_rect(&mini.ctx, Id::new("frame-row"));
    drop(mini);
    assert_fits("frames 6922", r, w);

    // Swatches panel: drop-down of the file menu next to the swatch set
    let mut h = harness();
    show(&mut h, PanelId::Swatches);
    let anchor = h
        .ctx
        .read_response(Id::new("swatch-file"))
        .expect("swatches file menu")
        .rect;
    press(&mut h, anchor.center(), PointerButton::Primary);
    let r = popup_rect(&h.ctx, Id::new("swatch-file"));
    let names: Vec<String> = [
        "panel-swatches-add",
        "panel-swatches-replace",
        "panel-swatches-save",
        "panel-swatches-clear",
        "panel-swatches-sort",
        "panel-swatches-add-fill",
    ]
    .iter()
    .map(|k| fp_i18n::t(k))
    .collect();
    let w = widest(&h, &names).max(anchor.width());
    assert_fits("swatches file menu", r, w);
}

/// Opens the ≡ menu of panel `p` (the ≡ button in the row of its tab) and returns the rect of
/// the menu area that opened, with the labels of the panel menu.
fn panel_menu_rect(h: &mut Harness<'_, FirepixelApp>, p: PanelId) -> (Rect, Vec<String>) {
    use egui_kittest::kittest::{By, Queryable};
    show(h, p);
    let names = labels(&fp_ui::menu::build_panel_menu(&h.state().state, p));
    let tab = h
        .query_all(By::new().label(&fp_i18n::t(p.title_key())))
        .map(|n| n.rect())
        .next()
        .expect("panel tab");
    let button = h
        .query_all(By::new().label(&fp_i18n::t("ui-dock-panel-menu")))
        .map(|n| n.rect())
        .filter(|r| r.center().y >= tab.top() && r.center().y <= tab.bottom())
        .min_by(|a, b| (a.left() - tab.right()).abs().total_cmp(&(b.left() - tab.right()).abs()))
        .expect("≡ button in the tab row");
    let layers = |h: &Harness<'_, FirepixelApp>| {
        h.ctx.memory(|m| m.areas().visible_layer_ids())
    };
    let before = layers(h);
    press(h, button.center(), PointerButton::Primary);
    let rect = h.ctx.memory(|m| {
        m.areas()
            .visible_layer_ids()
            .into_iter()
            .filter(|l| !before.contains(l))
            .filter_map(|l| m.area_rect(l.id))
            .find(|r| r.top() >= button.top() - 2.0)
    });
    (rect.expect("menu area"), names)
}

/// TW-125: short panel menus are as wide as their widest entry, not the 230 px minimum of the
/// main menus. Reference (DE, run 17): Common Library 179 px, States 207 px, Swatches 226 px.
#[test]
fn short_panel_menus_size_to_content() {
    fp_i18n::set_language("de");
    let mut bad = Vec::new();
    for (p, reference) in [
        (PanelId::CommonLibrary, 179.0),
        (PanelId::States, 207.0),
        (PanelId::Swatches, 226.0),
    ] {
        let mut h = harness();
        let (r, names) = panel_menu_rect(&mut h, p);
        eprintln!("{p:?} ≡: {:.0} px ({names:?})", r.width());
        if (r.width() - reference).abs() > 5.0 {
            bad.push(format!("{p:?} ≡: {:.0} px, reference {reference} px", r.width()));
        }
    }
    assert!(bad.is_empty(), "{bad:?}");
}

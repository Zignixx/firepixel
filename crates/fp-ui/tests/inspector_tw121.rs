//! TW-121: property inspector of a button symbol instance – header "Button", name field = slice
//! export name, "Text:" disabled without a text block in the symbol, full-width export combo with
//! the slice preset, no "Edit..." button, an "Add Filters..." button instead of the filter list,
//! Hyperlink/Alt/Target/"Show Down state on load".

use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::Harness;
use fp_ui::app_state::UiSettings;
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

const W: f32 = 1800.0;
const HH: f32 = 1130.0;

fn harness(setup: impl FnOnce(&mut fp_engine::Session)) -> H {
    fp_i18n::set_language("de");
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 300, "background": "#ffffff"})).expect("doc");
    setup(&mut s);
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(W, HH)).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

/// (rect, value, enabled) of the inspector's nodes with `role`.
fn nodes(h: &H, role: egui::accesskit::Role) -> Vec<(egui::Rect, String, String, bool)> {
    h.get_all(By::new().role(role)).filter(|n| n.rect().top() > HH - 160.0).map(|n| (n.rect(), n.value().unwrap_or_default(), n.accesskit_node().label().unwrap_or_default(), !n.accesskit_node().is_disabled())).collect()
}

fn button_doc(s: &mut fp_engine::Session) {
    s.execute("draw.rectangle", json!({"rect": [260, 20, 360, 60], "fillColor": "#808080", "strokeWidth": 0})).expect("rect");
    s.execute("modify.symbol.convertToSymbol", json!({"name": "Knopf1", "kind": "button"})).expect("button");
    s.execute("element.setName", json!({"name": "BTN"})).expect("name");
}

#[test]
fn button_instance_tab_layout() {
    let mut h = harness(button_doc);
    h.run_steps(3);
    let inputs = nodes(&h, egui::accesskit::Role::TextInput);
    // name field: slice export name (unsaved document ⇒ automatic "_r…_c…"), not "BTN"/"Knopf1"
    assert!(inputs.iter().all(|(_, v, _, _)| v != "BTN" && v != "Knopf1"), "{inputs:?}");
    assert!(inputs.iter().any(|(_, v, _, _)| v.contains("_r") && v.contains("_c")), "auto slice name: {inputs:?}");
    // "Text:" disabled: the symbol has no text block (field at DLU x 25, width 124 → 186 px)
    let text_field = inputs.iter().find(|(r, _, _, _)| (r.width() - 183.0).abs() < 8.0 && r.top() < HH - 100.0).unwrap_or_else(|| panic!("Text field: {inputs:?}"));
    assert!(!text_field.3, "Text: disabled {text_field:?}");
    // no "Edit..." button; an enabled "Add Filters..." button
    let buttons = nodes(&h, egui::accesskit::Role::Button);
    let edit = fp_i18n::t("pi-edit");
    assert!(!buttons.iter().any(|(_, _, l, _)| *l == edit), "no Edit...: {buttons:?}");
    let add = fp_i18n::t("pi-btn-add-filters");
    assert!(buttons.iter().any(|(_, _, l, en)| *l == add && *en), "Add Filters...: {buttons:?}");
    // full-width enabled export combo below the text field
    let combos = nodes(&h, egui::accesskit::Role::ComboBox);
    let export = combos.iter().find(|(r, _, _, _)| (r.width() - 183.0).abs() < 8.0 && (r.left() - text_field.0.left()).abs() < 8.0).expect("export combo");
    assert!(export.3 && !export.1.is_empty(), "export combo enabled with a preset: {export:?}");
    // Hyperlink, Target and the down-state checkbox are there and enabled
    assert!(combos.iter().filter(|(_, _, _, en)| *en).count() >= 3, "{combos:?}");
    let checks = nodes(&h, egui::accesskit::Role::CheckBox);
    let down = fp_i18n::t("pi-btn-show-down");
    assert!(checks.iter().any(|(_, _, l, en)| *l == down && *en), "{checks:?}");
    // the filter list "+"/"−" of the generic instance tab is absent
    let plus = fp_i18n::t("pi-fx-add");
    assert!(!buttons.iter().any(|(_, _, l, _)| *l == plus), "no filter list: {buttons:?}");
}

#[test]
fn button_export_combo_shows_symbol_preset() {
    let mut h = harness(button_doc);
    let presets = fp_export::options::builtin_presets();
    let p = presets.iter().find(|p| fp_ui::panels::optimize::preset_label(p.name) == "PNG32").expect("PNG32 preset");
    let model = p.settings.to_model(&fp_model::ExportSettings::default());
    h.state_mut().state.session.execute("pi.button", json!({"exportSettings": serde_json::to_value(&model).expect("json")})).expect("pi.button");
    h.run_steps(3);
    let combos = nodes(&h, egui::accesskit::Role::ComboBox);
    assert!(combos.iter().any(|(_, v, _, en)| v == "PNG32" && *en), "{combos:?}");
}

#[test]
fn button_with_text_block_enables_text_field() {
    let mut h = harness(|s| {
        s.execute("draw.rectangle", json!({"rect": [260, 20, 360, 60], "fillColor": "#808080", "strokeWidth": 0})).expect("rect");
        s.execute("draw.text", json!({"text": "Go", "anchor": [280, 45], "size": 16})).expect("text");
        s.execute("select.all", json!({})).expect("all");
        s.execute("modify.symbol.convertToSymbol", json!({"name": "Knopf2", "kind": "button"})).expect("button");
    });
    h.run_steps(3);
    let inputs = nodes(&h, egui::accesskit::Role::TextInput);
    let text_field = inputs.iter().find(|(r, _, _, _)| (r.width() - 183.0).abs() < 8.0 && r.top() < HH - 100.0).unwrap_or_else(|| panic!("Text field: {inputs:?}"));
    assert!(text_field.3, "Text: enabled {text_field:?}");
}

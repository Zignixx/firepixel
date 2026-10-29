//! F138: the PSD-style Stroke live effect in the live filter menu "+" of the Properties
//! inspector: submenu "PSD Live Effects" after the separator with "Stroke" ("Kontur"); its dialog
//! (size, position, blending mode, opacity, fill type, colour) adds the effect with one History row
//! named "Stroke", the edit dialog of the existing effect shows its values and changes the same
//! entry.

use egui_kittest::Harness;
use egui_kittest::kittest::{By, Queryable};
use fp_ui::FirepixelApp;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::effects;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness_with_rect() -> (H, u64) {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 300, "background": "#ffffff"}))
        .expect("doc");
    let v = s
        .execute(
            "draw.rectangle",
            json!({"rect": [40, 40, 300, 200], "fillColor": "#336699", "strokeWidth": 0}),
        )
        .expect("rect");
    let id = v.value.get("id").and_then(|x| x.as_u64()).expect("id");
    s.execute("select.set", json!({"ids": [id]})).expect("select");
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
    (h, id)
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn effects_of(h: &H, id: u64) -> Vec<fp_model::Effect> {
    let d = st(h).session.active().expect("doc");
    fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id))
        .expect("element")
        .common
        .effects
        .clone()
        .map(|l| l.effects)
        .unwrap_or_default()
}

fn ok(h: &mut H) {
    let b: Vec<_> = h.get_all(By::new().label("OK")).collect();
    b.last().expect("OK").click();
    drop(b);
    h.run_steps(3);
}

#[test]
fn stroke_menu_entry_dialog_and_edit() {
    fp_i18n::set_language("en");
    for (lang, name) in [("de", "Kontur"), ("en", "Stroke")] {
        assert_eq!(fp_i18n::with_language(lang, || fp_i18n::t("effect-stroke")), name);
    }
    let (mut h, id) = harness_with_rect();
    let i = effects::FX
        .iter()
        .position(|f| f.label == "effect-stroke")
        .expect("stroke entry");
    assert_eq!(effects::FX[i].group, effects::PSD_GROUP);
    let rows = st(&h).session.active().expect("doc").history.undo_count();
    effects::begin_add(&mut h.state_mut().state, i);
    h.run_steps(2);
    {
        let f = h.state_mut().state.dialogs.form.as_mut().expect("dialog open");
        // defaults of the PSD format: 3 px outside, normal, 100 %, red
        assert_eq!(f.get("size").as_f64(), Some(3.0));
        assert_eq!(f.get("opacity").as_f64(), Some(100.0));
        f.field_mut("size").expect("size").value = json!(6.0);
        f.field_mut("position").expect("position").value = json!(1);
        f.field_mut("mode").expect("mode").value = json!(1);
        f.field_mut("opacity").expect("opacity").value = json!(70.0);
        f.field_mut("color").expect("colour").value = json!("#00ff00ff");
    }
    h.run_steps(2);
    ok(&mut h);
    assert!(st(&h).dialogs.form.is_none(), "dialog closed");
    let d = st(&h).session.active().expect("doc");
    assert_eq!(d.history.undo_count(), rows + 1, "one history row");
    assert_eq!(
        st(&h).session.step_label(d.history.steps()[rows]),
        "Stroke"
    );
    let fx = effects_of(&h, id);
    assert_eq!(fx.len(), 1);
    assert_eq!(fx[0].kind(), fp_model::EffectKind::PhotoshopLive);
    assert_eq!(
        fx[0].get("stroke"),
        Some("1&70.000000&6.000000&multiply&RGBColor;0.000000;255.000000;0.000000&insetFrame&solidColor&")
    );
    assert_eq!(effects::effect_label(&fx[0]), "Stroke");
    assert!(effects::editable(&fx[0]));
    // edit: the values come back, OK changes the same entry
    effects::begin_edit(&mut h.state_mut().state, 0, &fx[0]);
    h.run_steps(2);
    {
        let f = h.state_mut().state.dialogs.form.as_mut().expect("edit dialog");
        assert_eq!(f.get("size").as_f64(), Some(6.0));
        assert_eq!(f.get("position"), json!("inside"));
        assert_eq!(f.get("mode"), json!("multiply"));
        f.field_mut("position").expect("position").value = json!(2);
    }
    h.run_steps(2);
    ok(&mut h);
    let fx = effects_of(&h, id);
    assert_eq!(fx.len(), 1, "same entry edited");
    let r = fx[0].get("stroke").expect("stroke");
    assert!(r.contains("&centeredFrame&") && r.starts_with("1&70.000000&6.000000&multiply"), "{r}");
}

//! K-4: inspector of a mask group (re 29b §1 b, CT-B observation): it looks like a normal group,
//! "Gruppieren (2)" for three selected objects, no mask options.

use egui::epaint::Shape;
use egui::vec2;
use egui_kittest::Harness;
use fp_ui::app_state::UiSettings;
use fp_ui::theme::ThemeKind;
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

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
fn k4_mask_group_inspector_like_a_group() {
    fp_engine::shell::disable();
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 250, "background": "#ffffff"})).expect("doc");
    let mut ids = Vec::new();
    for (cmd, r) in [("draw.rectangle", [50, 50, 150, 150]), ("draw.ellipse", [90, 90, 190, 190]), ("draw.rectangle", [70, 70, 170, 170])] {
        ids.push(s.execute(cmd, json!({"rect": r})).expect("draw").value["id"].as_u64().unwrap_or(0));
    }
    s.execute("select.set", json!({"ids": ids})).expect("sel");
    s.execute("modify.mask.groupAsMask", json!({})).expect("mask group");
    let ctx = egui::Context::default();
    let app = FirepixelApp::with_session(&ctx, s, UiSettings { theme: ThemeKind::Normal, ..UiSettings::default() }, false);
    let mut h = Harness::builder().with_size(vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(8).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(5);
    let texts = painted_texts(&h);
    assert!(texts.iter().any(|t| t == "Gruppieren (2)"), "Kopfzeile: {texts:?}");
    let mask = fp_i18n::t("pi-mask");
    assert!(!texts.iter().any(|t| *t == mask || t == "Pfadumriss" || t == "Graustufendarstellung"), "keine Maskenoptionen: {texts:?}");
}

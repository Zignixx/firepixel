//! FX-1 (Nutzerbefund N28): Schatten- und Glühen-Dialoge schreiben dieselben Werte wie das Original
//! (`re\23-effekte.md`, Gruppe 1): Menü-Vorgaben, Deckkraft = `BevelContrast` beim (Inneren) Glühen,
//! Inneres Glühen Breite < 3 → 2, `element.addEffect {effect: "glow"}` ergibt Glühen „Normal“.
//! Dazu §4b: Deckkraft 50× ändern → ein Effekt mit dem letzten Wert, ein Verlaufsschritt.

use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use fp_engine::Session;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::effects;
use fp_ui::FirepixelApp;
use serde_json::{json, Value};

fn session_with_rect() -> Session {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80]})).expect("rect");
    s
}

fn harness() -> Harness<'static, FirepixelApp> {
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, session_with_rect(), UiSettings::default(), false);
    a.state.sync_render = true;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(16)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a);
    h.run_steps(2);
    h
}

fn effects_in(s: &Session) -> Vec<fp_model::effects::Effect> {
    let d = s.active().expect("doc");
    let id = *fp_engine::docops::selectable_ids(&d.doc, d.page, d.frame).last().expect("Element");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("element");
    e.common.effects.clone().map(|l| l.effects).unwrap_or_default()
}

fn effects_of(st: &AppState) -> Vec<fp_model::effects::Effect> {
    effects_in(&st.session)
}

fn undo_count(st: &AppState) -> usize {
    st.session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn set_field(h: &mut Harness<'_, FirepixelApp>, key: &str, v: Value) {
    let f = h.state_mut().state.dialogs.form.as_mut().expect("Dialog offen");
    f.field_mut(key).unwrap_or_else(|| panic!("Feld {key}")).value = v;
    h.run_steps(1);
}

fn press_ok(h: &mut Harness<'_, FirepixelApp>) {
    h.run_steps(1);
    let b: Vec<_> = h.get_all(egui_kittest::kittest::By::new().label("OK")).collect();
    b.last().expect("OK").click();
    drop(b);
    h.run_steps(3);
}

fn fx_index(label: &str) -> usize {
    effects::FX.iter().position(|f| f.menu_text() == label).unwrap_or_else(|| panic!("{label}"))
}

/// Effekt über das Menü mit den Vorgaben des Dialogs hinzufügen (OK ohne Änderung).
fn add_with_defaults(label: &str) -> fp_model::effects::Effect {
    let mut h = harness();
    effects::begin_add(&mut h.state_mut().state, fx_index(label));
    h.run_steps(2);
    press_ok(&mut h);
    let fx = effects_of(&h.state().state);
    assert_eq!(fx.len(), 1, "{label}");
    fx[0].clone()
}

#[test]
fn menu_defaults_like_original() {
    let g = add_with_defaults("Glühen");
    for (k, v) in [("BevelType", 4.0), ("GlowWidth", 5.0), ("BevelContrast", 65.0), ("MaskSoftness", 12.0), ("GlowStartDistance", 0.0), ("EdgeThreshold", 1.0)] {
        assert_eq!(g.get_f64(k), Some(v), "Glühen {k}");
    }
    assert_eq!(g.get("OuterBevelColor"), Some("#df0000"));

    let ig = add_with_defaults("Inneres Glühen");
    for (k, v) in [("GlowWidth", 4.0), ("BevelContrast", 65.0), ("MaskSoftness", 4.0), ("GlowStartDistance", 0.0)] {
        assert_eq!(ig.get_f64(k), Some(v), "Inneres Glühen {k}");
    }
    assert_eq!(ig.get("OuterBevelColor"), Some("#000000"));

    let ds = add_with_defaults("Schlagschatten");
    assert_eq!(ds.get("ShadowColor"), Some("#000000a2"), "Schlagschatten 64 %");
    assert_eq!(ds.get_f64("ShadowDistance"), Some(7.0));
    assert_eq!(ds.get_f64("ShadowBlur"), Some(4.0));
    assert_eq!(ds.get_f64("ShadowAngle"), Some(315.0));
    let is = add_with_defaults("Innerer Schatten");
    assert_eq!(is.get("ShadowColor"), Some("#000000a5"), "Innerer Schatten 65 %");
}

#[test]
fn glow_opacity_is_bevel_contrast_50_changes_one_effect() {
    for label in ["Glühen", "Inneres Glühen"] {
        let mut h = harness();
        let before = undo_count(&h.state().state);
        effects::begin_add(&mut h.state_mut().state, fx_index(label));
        h.run_steps(2);
        for k in 1..=50 {
            set_field(&mut h, "opacity", json!(k * 2));
            assert!(effects_of(&h.state().state).len() <= 1, "{label}: {k}");
        }
        set_field(&mut h, "opacity", json!(37));
        press_ok(&mut h);
        let fx = effects_of(&h.state().state);
        assert_eq!(fx.len(), 1, "{label}");
        assert_eq!(fx[0].get_f64("BevelContrast"), Some(37.0), "{label}: Deckkraft = BevelContrast");
        assert_eq!(fx[0].get("OuterBevelColor").map(|c| c.len()), Some(7), "{label}: Farbe ohne Alpha");
        assert_eq!(undo_count(&h.state().state), before + 1, "{label}: ein Schritt");
        // Bearbeiten zeigt die gespeicherte Deckkraft
        let e = fx[0].clone();
        effects::begin_edit(&mut h.state_mut().state, 0, &e);
        h.run_steps(2);
        let shown = h.state().state.dialogs.form.as_ref().and_then(|f| f.raw_values().get("opacity").and_then(|v| v.as_f64()));
        assert_eq!(shown, Some(37.0), "{label}: Dialog zeigt BevelContrast");
    }
}

#[test]
fn inner_glow_width_below_three_becomes_two() {
    for (w, want) in [(0, 2.0), (1, 2.0), (2, 2.0), (3, 3.0), (30, 30.0)] {
        let mut h = harness();
        effects::begin_add(&mut h.state_mut().state, fx_index("Inneres Glühen"));
        h.run_steps(2);
        set_field(&mut h, "width", json!(w));
        press_ok(&mut h);
        assert_eq!(effects_of(&h.state().state)[0].get_f64("GlowWidth"), Some(want), "Breite {w}");
    }
}

#[test]
fn add_effect_by_name_uses_menu_preset() {
    let mut s = session_with_rect();
    s.execute("element.addEffect", json!({"effect": "glow"})).expect("glow");
    s.execute("element.addEffect", json!({"effect": "glow", "params": {"GlowWidth": "9"}})).expect("glow 9");
    s.execute("element.addEffect", json!({"effect": "dropShadow"})).expect("drop");
    let fx = effects_in(&s);
    assert_eq!(fx.len(), 3);
    assert_eq!(fx[0].get_f64("BevelType"), Some(4.0), "glow ohne Parameter ist Glühen, keine Abschrägung");
    assert_eq!(fx[0].get_f64("GlowWidth"), Some(5.0));
    assert_eq!(fx[0].get_f64("EdgeThreshold"), Some(1.0));
    assert_eq!(fx[1].get_f64("GlowWidth"), Some(9.0), "Parameter überschreiben die Vorgabe");
    assert_eq!(fx[1].get_f64("MaskSoftness"), Some(12.0));
    assert_eq!(fx[2].get("ShadowColor"), Some("#000000a2"));
}

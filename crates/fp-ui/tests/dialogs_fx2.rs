//! FX-2: Dialoge Abschrägung innen/außen und Prägung erhaben/vertieft (re\23 Gruppe 2, Nutzerbefund N28).
//! Vorgaben und Feldzuordnung wie im Original (Prägung: Weichheit ↔ MaskSoftness), vollständige
//! Vorgabeliste beim Anlegen, Reihenfolge und Grenzen der Felder, Logikfälle nach AGENTEN_REGELN §4b.

use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use fp_engine::Session;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::effects;
use fp_ui::dialogs::form::FieldKind;
use fp_ui::FirepixelApp;
use serde_json::{json, Value};
use std::sync::Arc;

fn app_with_rect() -> FirepixelApp {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80]})).expect("rect");
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.sync_render = true;
    a
}

fn harness(a: FirepixelApp) -> Harness<'static, FirepixelApp> {
    Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(16).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a)
}

fn st<'a>(h: &'a Harness<'_, FirepixelApp>) -> &'a AppState {
    &h.state().state
}

fn effects_of(st: &AppState) -> Vec<fp_model::effects::Effect> {
    let d = st.session.active().expect("doc");
    let id = *fp_engine::docops::selectable_ids(&d.doc, d.page, d.frame).last().expect("Element");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("element");
    e.common.effects.clone().map(|l| l.effects).unwrap_or_default()
}

fn undo_count(st: &AppState) -> usize {
    st.session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn doc(st: &AppState) -> Arc<fp_model::Document> {
    Arc::clone(&st.session.active().expect("doc").doc)
}

fn set_field(h: &mut Harness<'_, FirepixelApp>, key: &str, v: Value) {
    let f = h.state_mut().state.dialogs.form.as_mut().expect("Dialog offen");
    f.field_mut(key).unwrap_or_else(|| panic!("Feld {key}")).value = v;
    h.run_steps(1);
}

fn click_button(h: &mut Harness<'_, FirepixelApp>, label: &str) {
    h.run_steps(1);
    let b: Vec<_> = h.get_all(egui_kittest::kittest::By::new().label(label)).collect();
    b.last().unwrap_or_else(|| panic!("Knopf {label}")).click();
    drop(b);
    h.run_steps(3);
}

/// Menüeintrag zum `BevelType`.
fn fx_index(bevel: i32) -> usize {
    effects::FX
        .iter()
        .position(|f| matches!(f.kind, effects::FxKind::Effect { name: "bevel", bevel: Some(b) } if b == bevel))
        .unwrap_or_else(|| panic!("BevelType {bevel}"))
}

/// Neuen Effekt über das Menü öffnen.
fn open_new(h: &mut Harness<'_, FirepixelApp>, bevel: i32) {
    effects::begin_add(&mut h.state_mut().state, fx_index(bevel));
    h.run_steps(2);
}

fn keys(h: &Harness<'_, FirepixelApp>) -> Vec<String> {
    st(h).dialogs.form.as_ref().expect("Dialog").fields.iter().map(|f| f.key.clone()).collect()
}

fn field(h: &Harness<'_, FirepixelApp>, key: &str) -> Value {
    st(h).dialogs.form.as_ref().expect("Dialog").get(key)
}

fn txt(e: &fp_model::effects::Effect, k: &str) -> String {
    e.get(k).unwrap_or_else(|| panic!("Schlüssel {k} fehlt")).to_string()
}

#[test]
fn dialog_defaults_limits_and_field_order() {
    // (BevelType, Breite, Weichheit, Felder in Reihenfolge)
    let cases: [(i32, f64, f64, &[&str]); 4] = [
        (0, 10.0, 3.0, &["type", "slope", "width", "contrast", "soft", "angle", "button"]),
        (1, 10.0, 3.0, &["type", "slope", "width", "color", "contrast", "soft", "angle", "button"]),
        (2, 2.0, 2.0, &["type", "width", "contrast", "soft", "angle", "show"]),
        (3, 2.0, 2.0, &["type", "width", "contrast", "soft", "angle", "show"]),
    ];
    for (bt, width, soft, order) in cases {
        let mut h = harness(app_with_rect());
        h.run_steps(2);
        open_new(&mut h, bt);
        assert_eq!(keys(&h), order.iter().map(|s| s.to_string()).collect::<Vec<_>>(), "Typ {bt}: Reihenfolge");
        assert_eq!(field(&h, "width").as_f64(), Some(width), "Typ {bt}: Breite");
        assert_eq!(field(&h, "contrast").as_f64(), Some(75.0), "Typ {bt}: Kontrast");
        assert_eq!(field(&h, "soft").as_f64(), Some(soft), "Typ {bt}: Weichheit");
        assert_eq!(field(&h, "angle").as_f64(), Some(135.0), "Typ {bt}: Winkel");
        // Grenzen der Felder (re\23): Breite 0–35, Kontrast 0–100, Weichheit 0–10, Winkel 0–360
        let f = st(&h).dialogs.form.as_ref().expect("Dialog");
        for (k, lo, hi) in [("width", 0.0, 35.0), ("contrast", 0.0, 100.0), ("soft", 0.0, 10.0), ("angle", 0.0, 360.0)] {
            let fld = f.fields.iter().find(|x| x.key == k).expect("Feld");
            match &fld.kind {
                FieldKind::Number { min, max, .. } => assert_eq!((*min, *max), (lo, hi), "Typ {bt}: Grenzen {k}"),
                other => panic!("Typ {bt}: {k} ist {other:?}"),
            }
        }
        if bt < 2 {
            let n = f.fields.iter().find(|x| x.key == "slope").map(|x| match &x.kind {
                FieldKind::Choice(v) => v.len(),
                _ => 0,
            });
            assert_eq!(n, Some(7), "7 Kantenformen");
            let n = f.fields.iter().find(|x| x.key == "button").map(|x| match &x.kind {
                FieldKind::Choice(v) => v.len(),
                _ => 0,
            });
            assert_eq!(n, Some(4), "4 Tastenstatus");
        }
        click_button(&mut h, "OK");
        let fx = effects_of(st(&h));
        assert_eq!(fx.len(), 1);
        // vollständige Vorgabeliste wie EFDv (re\23 Rücklesung „vorgabe“); Außenfarbe = Dialogfarbe
        for (k, v) in effects::bevel_defaults(bt as i64) {
            if k == "OuterBevelColor" && bt == 1 {
                continue;
            }
            assert_eq!(txt(&fx[0], k), v, "Typ {bt}: {k}");
        }
    }
}

#[test]
fn emboss_softness_writes_mask_softness() {
    for bt in [2, 3] {
        let mut h = harness(app_with_rect());
        h.run_steps(2);
        let before = undo_count(st(&h));
        open_new(&mut h, bt);
        // §4b: 50 Änderungen (wie Tippen/Ziehen) → ein Effekt mit dem letzten Wert
        for k in 0..50 {
            set_field(&mut h, "soft", json!(k % 11));
            assert!(effects_of(st(&h)).len() <= 1, "Vorschau addiert");
        }
        set_field(&mut h, "soft", json!(7));
        click_button(&mut h, "OK");
        let fx = effects_of(st(&h));
        assert_eq!(fx.len(), 1);
        assert_eq!(fx[0].get_f64("MaskSoftness"), Some(7.0), "Prägung: Weichheit = MaskSoftness");
        assert_eq!(fx[0].get_f64("AngleSoftness"), Some(3.0), "Prägung: AngleSoftness bleibt Vorgabe");
        assert_eq!(undo_count(st(&h)), before + 1, "genau ein Verlaufsschritt");
        h.state_mut().state.session.undo().expect("undo");
        assert!(effects_of(st(&h)).is_empty(), "Rückgängig entfernt den Effekt");
    }
}

#[test]
fn bevel_softness_slope_and_button_state() {
    for bt in [0, 1] {
        let mut h = harness(app_with_rect());
        h.run_steps(2);
        open_new(&mut h, bt);
        set_field(&mut h, "soft", json!(9));
        set_field(&mut h, "slope", json!(6));
        set_field(&mut h, "button", json!(2));
        click_button(&mut h, "OK");
        let fx = effects_of(st(&h));
        assert_eq!(fx.len(), 1);
        assert_eq!(fx[0].get_f64("AngleSoftness"), Some(9.0));
        assert_eq!(fx[0].get_f64("MaskSoftness"), Some(0.0));
        assert_eq!(fx[0].get_f64("SlopeType"), Some(6.0), "Kantenform Rüsche");
        assert_eq!(fx[0].get_f64("ButtonState"), Some(2.0), "Tastenstatus gedrückt");
    }
}

#[test]
fn emboss_kind_switch_sets_face_color_undo_redo() {
    let mut h = harness(app_with_rect());
    h.run_steps(2);
    open_new(&mut h, 3);
    click_button(&mut h, "OK");
    assert_eq!(txt(&effects_of(st(&h))[0], "EmbossFaceColor"), "#0000001e", "vertieft: graue Fläche");
    let d_inset = doc(st(&h));
    // vorhandenen Effekt bearbeiten: vertieft → erhaben
    let e = effects_of(st(&h))[0].clone();
    effects::begin_edit(&mut h.state_mut().state, 0, &e);
    h.run_steps(2);
    assert_eq!(field(&h, "soft").as_f64(), Some(2.0), "Bearbeiten zeigt MaskSoftness");
    set_field(&mut h, "type", json!(0));
    click_button(&mut h, "OK");
    let fx = effects_of(st(&h));
    assert_eq!(fx.len(), 1, "derselbe Eintrag");
    assert_eq!(txt(&fx[0], "BevelType"), "2");
    assert_eq!(txt(&fx[0], "EmbossFaceColor"), "#00000000", "erhaben: Fläche transparent");
    let d_raised = doc(st(&h));
    h.state_mut().state.session.undo().expect("undo");
    assert!(*doc(st(&h)) == *d_inset, "Rückgängig stellt vertieft exakt her");
    h.state_mut().state.session.redo().expect("redo");
    assert!(*doc(st(&h)) == *d_raised, "Wiederholen");
    // Bearbeiten ohne Artwechsel ändert nur das Feld
    let e = effects_of(st(&h))[0].clone();
    effects::begin_edit(&mut h.state_mut().state, 0, &e);
    h.run_steps(2);
    set_field(&mut h, "width", json!(5));
    click_button(&mut h, "OK");
    let fx = effects_of(st(&h));
    assert_eq!(txt(&fx[0], "EmbossFaceColor"), "#00000000");
    assert_eq!(fx[0].get_f64("BevelWidth"), Some(5.0));
    assert_eq!(fx.len(), 1);
}

#[test]
fn cancel_restores_and_selection_change_keeps_target() {
    for bt in 0..4 {
        let mut h = harness(app_with_rect());
        h.run_steps(2);
        let d0 = doc(st(&h));
        let before = undo_count(st(&h));
        open_new(&mut h, bt);
        for k in 1..=10 {
            set_field(&mut h, "width", json!(k));
            set_field(&mut h, "soft", json!(k % 10));
        }
        click_button(&mut h, "Abbrechen");
        assert!(*doc(st(&h)) == *d0, "Typ {bt}: Abbrechen stellt nicht her");
        assert_eq!(undo_count(st(&h)), before, "Typ {bt}: Verlauf");
    }
    // Auswahlwechsel mitten im Dialog: Ziel bleibt das beim Öffnen gewählte Element
    let mut h = harness(app_with_rect());
    h.state_mut().state.session.execute("draw.rectangle", json!({"rect": [150, 20, 250, 80]})).expect("rect2");
    h.run_steps(2);
    let ids = {
        let d = st(&h).session.active().expect("doc");
        fp_engine::docops::selectable_ids(&d.doc, d.page, d.frame)
    };
    assert_eq!(ids.len(), 2);
    h.state_mut().state.session.execute("select.set", json!({"ids": [ids[0].0]})).expect("select.set");
    h.run_steps(1);
    open_new(&mut h, 2);
    h.state_mut().state.session.execute("select.all", json!({})).expect("all");
    set_field(&mut h, "width", json!(4));
    click_button(&mut h, "OK");
    let d = st(&h).session.active().expect("doc");
    let n_fx: Vec<usize> = ids
        .iter()
        .map(|id| fp_engine::docops::element_on_page(&d.doc, d.page, *id).and_then(|e| e.common.effects.clone()).map(|l| l.effects.len()).unwrap_or(0))
        .collect();
    assert_eq!(n_fx, vec![1, 0], "Effekt nur auf dem beim Öffnen gewählten Element");
}

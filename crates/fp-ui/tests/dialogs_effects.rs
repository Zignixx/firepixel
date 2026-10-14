//! Effekt- und Filter-Dialoge mit Live-Vorschau (AGENTEN_REGELN §4b, Nutzerbefund „50× Glühen“):
//! Wert vielfach ändern → genau ein Effekt mit dem letzten Wert, OK = genau ein Verlaufsschritt,
//! Abbrechen = Ausgangszustand exakt; Bearbeiten eines vorhandenen Effekts ändert denselben Eintrag.

use egui_kittest::Harness;
use fp_engine::Session;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::effects;
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

/// Effekte des ersten Elements des Dokuments (unabhängig von der Auswahl).
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

/// Setzt ein Feld des offenen Formulars und lässt ein Bild laufen (wie eine Eingabe).
fn set_field(h: &mut Harness<'_, FirepixelApp>, key: &str, v: Value) {
    let f = h.state_mut().state.dialogs.form.as_mut().expect("Dialog offen");
    f.field_mut(key).unwrap_or_else(|| panic!("Feld {key}")).value = v;
    h.run_steps(1);
}

/// Klick auf „OK“ (wie ein Nutzer; Klicks dürfen nicht auf die Leinwand durchfallen).
fn press_ok(h: &mut Harness<'_, FirepixelApp>) {
    click_button(h, "OK");
}

/// Klick auf „Abbrechen“.
fn press_cancel(h: &mut Harness<'_, FirepixelApp>) {
    click_button(h, "Abbrechen");
}

/// Klick auf einen Knopf des Dialogs (Beschriftung).
fn click_button(h: &mut Harness<'_, FirepixelApp>, label: &str) {
    h.run_steps(1);
    let b: Vec<_> = h.get_all(egui_kittest::kittest::By::new().label(label)).collect();
    b.last().unwrap_or_else(|| panic!("Knopf {label}")).click();
    drop(b);
    h.run_steps(3);
}
use egui_kittest::kittest::Queryable;

/// Index eines Eintrags im Live-Filter-Menü.
fn fx_index(label: &str) -> usize {
    effects::FX.iter().position(|f| f.menu_text() == label).unwrap_or_else(|| panic!("{label}"))
}

const EFFECT_DIALOGS: &[(&str, &str, &str)] = &[
    ("Glühen", "width", "GlowWidth"),
    ("Schlagschatten", "dist", "ShadowDistance"),
    ("Innerer Schatten", "dist", "ShadowDistance"),
    ("Inneres Glühen", "width", "GlowWidth"),
    ("Abgeflachte Kante innen", "width", "BevelWidth"),
    ("Abgeflachte Kante außen", "width", "BevelWidth"),
    ("Relief erhaben", "width", "BevelWidth"),
    ("Relief vertieft", "width", "BevelWidth"),
    ("Einfarbiger Schatten", "dist", "distance"),
    ("Farbfüllung", "opacity", "Opacity"),
];

#[test]
fn new_effect_dialog_50_changes_one_effect_one_step() {
    for (label, field, key) in EFFECT_DIALOGS {
        let mut h = harness(app_with_rect());
        h.run_steps(2);
        let before = undo_count(st(&h));
        effects::begin_add(&mut h.state_mut().state, fx_index(label));
        h.run_steps(2);
        for k in 1..=50 {
            set_field(&mut h, field, json!(k % 30 + 1));
            assert!(effects_of(st(&h)).len() <= 1, "{label}: nach {k} Änderungen {} Effekte", effects_of(st(&h)).len());
        }
        set_field(&mut h, field, json!(17));
        press_ok(&mut h);
        let fx = effects_of(st(&h));
        assert_eq!(fx.len(), 1, "{label}: Effekte nach OK");
        assert_eq!(fx[0].get_f64(key), Some(17.0), "{label}: letzter Wert");
        assert_eq!(undo_count(st(&h)), before + 1, "{label}: genau ein Verlaufsschritt");
        assert!(st(&h).dialogs.form.is_none(), "{label}: Dialog geschlossen");
        h.state_mut().state.session.undo().expect("undo");
        assert!(effects_of(st(&h)).is_empty(), "{label}: Rückgängig entfernt den Effekt");
    }
}

#[test]
fn new_effect_dialog_cancel_restores_exactly() {
    for (label, field, _) in EFFECT_DIALOGS {
        let mut h = harness(app_with_rect());
        h.run_steps(2);
        let d0 = doc(st(&h));
        let before = undo_count(st(&h));
        effects::begin_add(&mut h.state_mut().state, fx_index(label));
        h.run_steps(2);
        for k in 1..=10 {
            set_field(&mut h, field, json!(k));
        }
        press_cancel(&mut h);
        assert!(*doc(st(&h)) == *d0, "{label}: Abbrechen stellt nicht her");
        assert_eq!(undo_count(st(&h)), before, "{label}: Verlauf nach Abbrechen");
        assert!(effects_of(st(&h)).is_empty());
    }
}

#[test]
fn edit_existing_effect_changes_same_entry() {
    let mut h = harness(app_with_rect());
    h.run_steps(2);
    // Schatten + Glühen anlegen, dann Glühen 50× bearbeiten
    h.state_mut().state.session.execute("element.addEffect", json!({"effect": "dropShadow"})).expect("Schatten");
    h.state_mut().state.session.execute("element.addEffect", json!({"effect": "glow", "params": {"BevelType": "4", "GlowWidth": "4"}})).expect("Glühen");
    h.run_steps(2);
    let before = undo_count(st(&h));
    let fx = effects_of(st(&h));
    assert_eq!(fx.len(), 2);
    effects::begin_edit(&mut h.state_mut().state, 1, &fx[1]);
    h.run_steps(2);
    for k in 1..=50 {
        set_field(&mut h, "width", json!(k % 35));
        assert_eq!(effects_of(st(&h)).len(), 2, "nach {k} Änderungen");
    }
    set_field(&mut h, "width", json!(23));
    press_ok(&mut h);
    let fx = effects_of(st(&h));
    assert_eq!(fx.len(), 2);
    assert_eq!(fx[1].get_f64("GlowWidth"), Some(23.0));
    assert_eq!(undo_count(st(&h)), before + 1);
    // Abbrechen beim Bearbeiten
    let d0 = doc(st(&h));
    let fx = effects_of(st(&h));
    effects::begin_edit(&mut h.state_mut().state, 1, &fx[1]);
    h.run_steps(2);
    for k in 1..=20 {
        set_field(&mut h, "width", json!(k));
    }
    press_cancel(&mut h);
    assert!(*doc(st(&h)) == *d0, "Abbrechen beim Bearbeiten");
}

#[test]
fn filter_dialogs_live_and_destructive_preview() {
    for (label, field) in [("Gaußscher Weichz. ...", "radius"), ("Rauschen hinzufügen...", "amount"), ("Unscharf maskieren...", "amount"), ("Farbton/Sättigung...", "hue"), ("Helligkeit/Kontrast...", "brightness"), ("Bewegungsunschärfe…", "distance"), ("Radiales Weichz. ...", "amount"), ("Zentriertes Weichz. ...", "amount")] {
        // als Live-Effekt
        let mut h = harness(app_with_rect());
        h.run_steps(2);
        let before = undo_count(st(&h));
        effects::begin_add(&mut h.state_mut().state, fx_index(label));
        h.run_steps(2);
        for k in 1..=30 {
            set_field(&mut h, field, json!(k));
        }
        press_ok(&mut h);
        assert_eq!(effects_of(st(&h)).len(), 1, "{label}: ein Live-Filter, Status {:?}", st(&h).status);
        assert_eq!(undo_count(st(&h)), before + 1, "{label}: ein Schritt");
        // Abbrechen
        let d0 = doc(st(&h));
        effects::begin_add(&mut h.state_mut().state, fx_index(label));
        h.run_steps(2);
        for k in 1..=10 {
            set_field(&mut h, field, json!(k + 2));
        }
        press_cancel(&mut h);
        assert!(*doc(st(&h)) == *d0, "{label}: Abbrechen");
    }
}

#[test]
fn dialog_follows_selection_at_open_time() {
    // Auswahlwechsel bei offenem Dialog: die Vorschau bleibt am ursprünglichen Objekt
    let mut h = harness(app_with_rect());
    h.run_steps(2);
    let first = st(&h).session.active().expect("d").selection.elements[0];
    effects::begin_add(&mut h.state_mut().state, fx_index("Glühen"));
    h.run_steps(2);
    set_field(&mut h, "width", json!(5));
    // anderes Objekt anlegen/auswählen ist bei modalem Dialog nicht möglich; über die Sitzung erzwingen
    h.state_mut().state.session.execute("draw.rectangle", json!({"rect": [150, 20, 250, 80]})).ok();
    set_field(&mut h, "width", json!(9));
    press_ok(&mut h);
    let d = st(&h).session.active().expect("d");
    let count = |id| fp_engine::docops::element_on_page(&d.doc, d.page, id).and_then(|e| e.common.effects.clone()).map(|l| l.effects.len()).unwrap_or(0);
    let total: usize = fp_engine::docops::selectable_ids(&d.doc, d.page, d.frame).into_iter().map(count).sum();
    assert_eq!(total, 1, "genau ein Glühen im Dokument");
    assert_eq!(count(first), 1, "am ursprünglich gewählten Objekt");
}


/// Klick auf „OK“ über der Leinwand darf die Auswahl nicht ändern und keinen weiteren
/// Verlaufsschritt erzeugen (Durchklicken auf die Leinwand, Befund an LW: `canvas\mod.rs`).
#[test]
fn ok_click_does_not_reach_canvas() {
    let mut h = harness(app_with_rect());
    h.run_steps(2);
    let before = undo_count(st(&h));
    let sel = st(&h).session.active().map(|d| d.selection.elements.clone());
    effects::begin_add(&mut h.state_mut().state, fx_index("Glühen"));
    h.run_steps(2);
    set_field(&mut h, "width", json!(12));
    click_button(&mut h, "OK");
    assert_eq!(st(&h).session.active().map(|d| d.selection.elements.clone()), sel);
    assert_eq!(undo_count(st(&h)), before + 1);
}
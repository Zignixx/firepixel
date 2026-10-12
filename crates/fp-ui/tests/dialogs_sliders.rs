//! Paritätsabgleich UX-B: Schieber mit der Kennlinie des Originals, Popup-Schieber ▾, Vorgaben und
//! gemerkte Werte aus den Voreinstellungen, Symbolknöpfe für Ecken/Endkappen. Realistische
//! Eingaben (1/60 s je Bild, Tippen als `Event::Text`, Tab übernimmt, Klicks und Ziehen).

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{By, Queryable};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness() -> H {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80]})).expect("rect");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn key(h: &mut H, k: Key, m: Modifiers) {
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: m });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: m });
}

fn press(h: &mut H, p: Pos2, down: bool) {
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: down, modifiers: Modifiers::NONE });
}

fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    press(h, p, true);
    press(h, p, false);
    h.run_steps(2);
}

fn drag(h: &mut H, a: Pos2, b: Pos2, frames: usize) {
    ev(h, Event::PointerMoved(a));
    press(h, a, true);
    for i in 1..=frames {
        ev(h, Event::PointerMoved(a + (b - a) * (i as f32 / frames as f32)));
    }
    press(h, b, false);
    h.run_steps(2);
}

fn open(h: &mut H, cmd: &str) {
    h.state_mut().state.run(cmd, json!({}));
    h.run_steps(3);
    assert!(st(h).dialogs.form.is_some(), "{cmd}: Formular offen");
}

fn field(h: &H, key: &str) -> f64 {
    st(h).dialogs.form.as_ref().expect("Formular").get(key).as_f64().unwrap_or(f64::NAN)
}

fn sliders(h: &H) -> Vec<(egui::Rect, String)> {
    h.get_all(By::new().role(egui::accesskit::Role::Slider)).map(|n| (n.rect(), n.value().unwrap_or_default())).collect()
}

/// Gaußscher Weichzeichner: 4,1 steht bei Position 41 von 145 (nicht ganz links wie linear);
/// Ziehen an den rechten Rand = 250, in die Mitte = 9,4 (Abschnitt 5–10 in 0,2er-Schritten).
#[test]
fn gaussian_radius_slider_uses_original_curve() {
    let mut h = harness();
    open(&mut h, "filter.blur.gaussian");
    // Feld ist beim Öffnen fokussiert und markiert: „4,1“ tippen, Tab übernimmt
    ev(&mut h, Event::Text("4,1".into()));
    key(&mut h, Key::Tab, Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(field(&h, "radius"), 4.1);
    let s = sliders(&h);
    assert_eq!(s.len(), 1, "eine Schieberleiste");
    let (r, v) = s[0].clone();
    assert_eq!(v, "41", "Position 41 von 145 wie im Original");
    // ganz nach rechts ziehen
    let knob = Pos2::new(r.left() + r.width() * 41.0 / 145.0, r.center().y);
    drag(&mut h, knob, Pos2::new(r.right() + 40.0, r.center().y), 20);
    assert_eq!(field(&h, "radius"), 250.0);
    // zurück in die Mitte: Position ≈ 72 → Wert im 0,2er-Abschnitt
    let s = sliders(&h);
    let (r, _) = s[0].clone();
    drag(&mut h, Pos2::new(r.right() - 2.0, r.center().y), Pos2::new(r.center().x, r.center().y), 20);
    let v = field(&h, "radius");
    assert!((8.0..=10.0).contains(&v), "Mitte = {v} (5–10er-Abschnitt)");
    assert!(((v * 10.0).round() as i64) % 2 == 0, "0,2er-Schritte: {v}");
    key(&mut h, Key::Escape, Modifiers::NONE);
}

/// Unscharf maskieren: Menge 250 → Schieber am Ende (170), Feld behält 250.
#[test]
fn sharpen_amount_slider_stops_at_170() {
    let mut h = harness();
    open(&mut h, "filter.sharpen.unsharpMask");
    ev(&mut h, Event::Text("250".into()));
    key(&mut h, Key::Tab, Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(field(&h, "amount"), 250.0);
    let s = sliders(&h);
    assert_eq!(s[0].1, "170", "Schieber am Ende");
    assert_eq!(s[1].1, "10", "Pixelradius 1,0 = Position 10");
    key(&mut h, Key::Escape, Modifiers::NONE);
}

/// Bewegungsunschärfe: Popup-Schieber ▾ statt fester Leiste; Ziehen im Popup setzt das Feld.
#[test]
fn motion_blur_uses_popup_sliders() {
    let mut h = harness();
    open(&mut h, "filter.blur.motion");
    assert!(sliders(&h).is_empty(), "keine festen Leisten");
    // ▾ rechts am ersten Feld: Feld suchen, Pfeil liegt 1 px dahinter
    let fields: Vec<egui::Rect> = h.get_all(By::new().role(egui::accesskit::Role::TextInput)).map(|n| n.rect()).collect();
    let dr = fp_ui::dialogs::dialog_rect(&h.ctx, &fp_ui::dialogs::last_dialog_id(&h.ctx).expect("Dialog")).expect("Rechteck");
    let first = fields.into_iter().filter(|r| dr.contains(r.center())).min_by(|a, b| a.top().total_cmp(&b.top())).expect("Feld");
    click(&mut h, Pos2::new(first.right() + 10.0, first.center().y));
    let s = sliders(&h);
    assert_eq!(s.len(), 1, "Popup-Schieber offen");
    let (r, _) = s[0].clone();
    drag(&mut h, Pos2::new(r.center().x, r.center().y), Pos2::new(r.left() - 30.0, r.center().y), 15);
    assert_eq!(field(&h, "angle"), 0.0, "Popup ganz links = 0");
    key(&mut h, Key::Escape, Modifiers::NONE);
}

/// Auswahlrahmen erweitern: Vorgabe 10, getippter Wert wird gemerkt und beim nächsten Öffnen gezeigt.
#[test]
fn expand_marquee_default_10_and_remembered() {
    let mut h = harness();
    open(&mut h, "select.expandMarquee");
    assert_eq!(field(&h, "amount"), 10.0, "Vorgabe 10 wie im Original");
    ev(&mut h, Event::Text("25".into()));
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(4);
    assert!(st(&h).dialogs.form.is_none(), "OK");
    assert_eq!(st(&h).session.prefs.get("ExpandSelectionAmount").and_then(|v| v.as_int()), Some(25));
    open(&mut h, "select.expandMarquee");
    assert_eq!(field(&h, "amount"), 25.0, "zuletzt benutzter Wert");
    // Abbrechen schreibt nichts zurück
    ev(&mut h, Event::Text("77".into()));
    key(&mut h, Key::Tab, Modifiers::NONE);
    key(&mut h, Key::Escape, Modifiers::NONE);
    h.run_steps(3);
    assert_eq!(st(&h).session.prefs.get("ExpandSelectionAmount").and_then(|v| v.as_int()), Some(25));
}

/// Strich erweitern: Ecken/Endkappen als Symbolknöpfe, Vorgabe rund (Code 0) bzw. stumpf, Breite 10;
/// Wahl per Klick landet im Befehl und in `ExpandJoinType`.
#[test]
fn expand_stroke_icon_buttons_and_prefs() {
    let mut h = harness();
    open(&mut h, "modify.alterPath.expandStroke");
    let f = st(&h).dialogs.form.clone().expect("Formular");
    assert_eq!(f.get("width").as_f64(), Some(10.0));
    assert_eq!(f.get("join"), json!("round"), "Vorgabe-Code 0 = rund");
    assert_eq!(f.get("cap"), json!("butt"));
    // Symbolknopf „Gehrung“ (erster) anklicken
    let icons: Vec<egui::Rect> = h.get_all(By::new().role(egui::accesskit::Role::RadioButton)).map(|n| n.rect()).filter(|r| r.width() < 30.0).collect();
    assert!(icons.len() >= 6, "3 Ecken + 3 Endkappen als Symbolknöpfe: {}", icons.len());
    let first = icons.iter().min_by(|a, b| (a.top(), a.left()).partial_cmp(&(b.top(), b.left())).expect("cmp")).copied().expect("Knopf");
    click(&mut h, first.center());
    assert_eq!(st(&h).dialogs.form.as_ref().expect("offen").get("join"), json!("miter"));
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(4);
    assert_eq!(st(&h).session.prefs.get("ExpandJoinType").and_then(|v| v.as_int()), Some(2), "Gehrung = Code 2");
}

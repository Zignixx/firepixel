//! Paritätsabgleich UX-B: Schieber mit der Kennlinie des Originals, Popup-Schieber ▾ (senkrecht wie
//! im Inspektor), Vorgaben und gemerkte Werte aus den Voreinstellungen, Symbolknöpfe für
//! Ecken/Endkappen. Realistische Eingaben (1/60 s je Bild, Tippen als `Event::Text`, Tab übernimmt,
//! Klicks und Ziehen über viele Bilder).

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::FirepixelApp;
use serde_json::json;
use std::sync::Arc;

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

fn undo_count(h: &H) -> usize {
    st(h).session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn doc(h: &H) -> Arc<fp_model::Document> {
    Arc::clone(&st(h).session.active().expect("doc").doc)
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

/// Ziehen über `frames` Bilder; `during` wird nach jedem Bild mit gedrückter Taste aufgerufen.
fn drag_with(h: &mut H, a: Pos2, b: Pos2, frames: usize, mut during: impl FnMut(&H)) {
    ev(h, Event::PointerMoved(a));
    press(h, a, true);
    for i in 1..=frames {
        ev(h, Event::PointerMoved(a + (b - a) * (i as f32 / frames as f32)));
        during(h);
    }
    press(h, b, false);
    h.run_steps(2);
}

fn drag(h: &mut H, a: Pos2, b: Pos2, frames: usize) {
    drag_with(h, a, b, frames, |_| {});
}

fn open(h: &mut H, cmd: &str) {
    h.state_mut().state.run(cmd, json!({}));
    h.run_steps(3);
    assert!(st(h).dialogs.form.is_some(), "{cmd}: Formular offen");
}

fn field(h: &H, key: &str) -> f64 {
    st(h).dialogs.form.as_ref().expect("Formular").get(key).as_f64().unwrap_or(f64::NAN)
}

/// Rechteck des offenen Dialogs.
fn dialog_rect(h: &H) -> egui::Rect {
    fp_ui::dialogs::dialog_rect(&h.ctx, &fp_ui::dialogs::last_dialog_id(&h.ctx).expect("Dialog")).expect("Rechteck")
}

/// Schieber (Rechteck, Position). egui meldet die Position als Zahlenwert (`numeric_value`), nicht
/// als Text – deshalb lesen wir diesen.
fn sliders(h: &H) -> Vec<(egui::Rect, f64)> {
    h.query_all(By::new().role(Role::Slider)).map(|n| (n.rect(), n.accesskit_node().numeric_value().unwrap_or(f64::NAN))).collect()
}

/// Zahlenfelder im Dialog, von oben nach unten (Rechteck, angezeigter Text).
fn text_fields(h: &H) -> Vec<(egui::Rect, String)> {
    let dr = dialog_rect(h);
    let mut v: Vec<(egui::Rect, String)> = h.get_all(By::new().role(Role::TextInput)).map(|n| (n.rect(), n.value().unwrap_or_default())).filter(|(r, _)| dr.contains(r.center())).collect();
    v.sort_by(|a, b| a.0.top().total_cmp(&b.0.top()));
    v
}

/// Klick (Zeiger) auf ein Element des Dialogs mit dieser Beschriftung.
fn click_label(h: &mut H, label: &str) {
    h.run_steps(1);
    let dr = dialog_rect(h);
    let mut r = h.query_all(By::new().label(label)).map(|n| n.rect()).filter(|r| dr.contains(r.center())).next_back();
    if r.is_none() {
        // Kästchen/Optionsfelder zeichnen ihre Beschriftung selbst (ohne Eintrag im
        // Barrierefreiheitsbaum): Lage aus dem Layout-Protokoll der Felder
        fp_ui::inspector::fields::set_audit(&h.ctx, true);
        h.run_steps(1);
        r = fp_ui::inspector::fields::take_audit(&h.ctx).into_iter().filter(|a| a.text == label && matches!(a.kind, "check" | "radio") && dr.contains(a.drawn.center())).map(|a| a.drawn).next_back();
        fp_ui::inspector::fields::set_audit(&h.ctx, false);
    }
    let r = r.unwrap_or_else(|| panic!("{label}"));
    click(h, r.center());
}

fn ok(h: &mut H) {
    click_label(h, "OK");
    h.run_steps(2);
}

fn cancel(h: &mut H) {
    click_label(h, "Abbrechen");
    h.run_steps(2);
}

/// Gaußscher Weichzeichner: 4,1 steht bei Position 41 von 145 (nicht ganz links wie linear);
/// Ziehen an den rechten Rand = 250, in die Mitte = 9,4/9,6 (Abschnitt 5–10 in 0,2er-Schritten).
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
    let (r, v) = s[0];
    assert_eq!(v, 41.0, "Position 41 von 145 wie im Original");
    // ganz nach rechts ziehen
    let knob = Pos2::new(r.left() + r.width() * 41.0 / 145.0, r.center().y);
    drag(&mut h, knob, Pos2::new(r.right() + 40.0, r.center().y), 20);
    assert_eq!(field(&h, "radius"), 250.0);
    assert_eq!(sliders(&h)[0].1, 145.0);
    // zurück in die Mitte: Position ≈ 72 → Wert im 0,2er-Abschnitt
    let (r, _) = sliders(&h)[0];
    drag(&mut h, Pos2::new(r.right() - 2.0, r.center().y), Pos2::new(r.center().x, r.center().y), 20);
    let v = field(&h, "radius");
    assert!((8.0..=10.0).contains(&v), "Mitte = {v} (5–10er-Abschnitt)");
    assert!(((v * 10.0).round() as i64) % 2 == 0, "0,2er-Schritte: {v}");
    // ganz links: kleinster Wert 0,1 (Position 0)
    let (r, _) = sliders(&h)[0];
    drag(&mut h, Pos2::new(r.center().x, r.center().y), Pos2::new(r.left() - 40.0, r.center().y), 20);
    assert_eq!(field(&h, "radius"), 0.1);
    key(&mut h, Key::Escape, Modifiers::NONE);
}

/// Unscharf maskieren: Menge 250 → Schieber am Ende (170), Feld behält 250; Pixelradius nach Kennlinie.
#[test]
fn sharpen_amount_slider_stops_at_170() {
    let mut h = harness();
    open(&mut h, "filter.sharpen.unsharpMask");
    ev(&mut h, Event::Text("250".into()));
    key(&mut h, Key::Tab, Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(field(&h, "amount"), 250.0);
    let s = sliders(&h);
    assert_eq!(s.len(), 3, "Menge, Pixelradius, Schwellenwert");
    assert_eq!(s[0].1, 170.0, "Schieber am Ende");
    let r = field(&h, "radius");
    let expected = fp_ui::inspector::scale::BLUR_RADIUS.to_pos(r, 0.1, 250.0);
    assert_eq!(s[1].1, expected, "Pixelradius {r} = Position {expected}");
    assert_eq!(s[2].1, field(&h, "threshold"), "Schwellenwert: Position = Wert");
    key(&mut h, Key::Escape, Modifiers::NONE);
}

/// Schieber über viele Bilder ziehen: OK ergibt genau einen Verlaufsschritt; Abbrechen nach dem
/// Ziehen stellt das Dokument exakt wieder her.
#[test]
fn gaussian_drag_ok_one_step_cancel_restores() {
    let mut h = harness();
    let (d0, n0) = (doc(&h), undo_count(&h));
    open(&mut h, "filter.blur.gaussian");
    let (r, _) = sliders(&h)[0];
    drag(&mut h, Pos2::new(r.left() + 5.0, r.center().y), Pos2::new(r.left() + r.width() * 0.4, r.center().y), 40);
    h.run_steps(3);
    let v = field(&h, "radius");
    assert!(v > 1.0 && v < 20.0, "Wert nach dem Ziehen: {v}");
    cancel(&mut h);
    assert!(st(&h).dialogs.form.is_none(), "Abbrechen schließt");
    assert!(*doc(&h) == *d0, "Abbrechen nach Ändern = keine Änderung");
    assert_eq!(undo_count(&h), n0, "Verlauf unverändert");
    open(&mut h, "filter.blur.gaussian");
    let (r, _) = sliders(&h)[0];
    drag(&mut h, Pos2::new(r.left() + 5.0, r.center().y), Pos2::new(r.left() + r.width() * 0.3, r.center().y), 40);
    h.run_steps(3);
    ok(&mut h);
    assert!(st(&h).dialogs.form.is_none(), "OK schließt");
    assert_eq!(undo_count(&h), n0 + 1, "genau ein Verlaufsschritt, Status {:?}", st(&h).status);
    assert!(*doc(&h) != *d0, "Filter angewandt");
}

/// Bewegungsunschärfe: Popup-Schieber ▾ statt fester Leiste, **senkrecht** wie im Inspektor (oben =
/// Maximum). Beim Ziehen zeigt das Feld den Wert live, übernommen wird beim Loslassen.
#[test]
fn motion_blur_uses_vertical_popup_sliders() {
    let mut h = harness();
    let (d0, n0) = (doc(&h), undo_count(&h));
    open(&mut h, "filter.blur.motion");
    assert!(sliders(&h).is_empty(), "keine festen Leisten");
    let angle0 = field(&h, "angle");
    // ▾ rechts am ersten Feld (Winkel), 1 px dahinter
    let first = text_fields(&h)[0].0;
    click(&mut h, Pos2::new(first.right() + 8.0, first.center().y));
    let s = sliders(&h);
    assert_eq!(s.len(), 1, "Popup-Schieber offen");
    let (r, pos) = s[0];
    assert!(r.height() > 150.0 && r.width() < 40.0, "senkrecht: {r:?}");
    assert_eq!(pos, angle0, "Position = Winkel beim Öffnen");
    // Knopf steht beim aktuellen Wert auf Höhe des Pfeils
    let track = fp_ui::inspector::fields::VSLIDER_TRACK;
    let pad = fp_ui::inspector::fields::VSLIDER_PAD;
    let knob_y = r.top() + pad + (1.0 - (angle0 / 360.0) as f32) * track;
    assert!((knob_y - first.center().y).abs() <= 3.0, "Knopf {knob_y} auf Höhe des Felds {}", first.center().y);
    // nach unten über 30 Bilder ziehen: Feld zeigt live, Formularwert bleibt bis zum Loslassen
    let mut live_seen = Vec::new();
    drag_with(&mut h, Pos2::new(r.center().x, knob_y), Pos2::new(r.center().x, r.bottom() + 40.0), 30, |h| {
        live_seen.push((text_fields(h)[0].1.clone(), field(h, "angle")));
    });
    assert!(live_seen.iter().all(|(_, v)| *v == angle0), "Formularwert erst beim Loslassen");
    assert!(live_seen.iter().any(|(t, _)| t != &format!("{angle0}")), "Feld zeigt den Wert live: {live_seen:?}");
    assert_eq!(live_seen.last().map(|x| x.0.as_str()), Some("0"));
    assert_eq!(field(&h, "angle"), 0.0, "Popup ganz unten = 0");
    assert!(sliders(&h).is_empty(), "Loslassen schließt das Popup");
    // nach oben = Maximum
    click(&mut h, Pos2::new(first.right() + 8.0, first.center().y));
    let (r, _) = sliders(&h)[0];
    drag(&mut h, Pos2::new(r.center().x, r.bottom() - 7.0), Pos2::new(r.center().x, r.top() - 40.0), 30);
    assert_eq!(field(&h, "angle"), 360.0, "Popup ganz oben = 360");
    // Abbrechen: keine Änderung am Dokument
    cancel(&mut h);
    assert!(*doc(&h) == *d0, "Abbrechen nach Ändern = keine Änderung");
    assert_eq!(undo_count(&h), n0);
}

/// Popup-Schieber in Folge (Winkel, dann Abstand), OK = genau ein Verlaufsschritt.
#[test]
fn motion_blur_popup_ok_one_step() {
    let mut h = harness();
    let n0 = undo_count(&h);
    open(&mut h, "filter.blur.motion");
    for (i, _) in ["angle", "distance"].iter().enumerate() {
        let f = text_fields(&h)[i].0;
        click(&mut h, Pos2::new(f.right() + 8.0, f.center().y));
        let (r, _) = sliders(&h)[0];
        drag(&mut h, Pos2::new(r.center().x, r.center().y + 30.0), Pos2::new(r.center().x, r.center().y - 20.0), 25);
    }
    let (a, d) = (field(&h, "angle"), field(&h, "distance"));
    assert!(a > 0.0 && a < 360.0 && d > 1.0 && d < 100.0, "Winkel {a}, Abstand {d}");
    ok(&mut h);
    assert!(st(&h).dialogs.form.is_none());
    assert_eq!(undo_count(&h), n0 + 1, "genau ein Verlaufsschritt, Status {:?}", st(&h).status);
}

/// Kästchen „Vorschau“ der Bewegungsunschärfe wird gemerkt (`MotionBlurShowPreview`), nur bei OK.
#[test]
fn motion_blur_preview_checkbox_remembered() {
    let mut h = harness();
    open(&mut h, "filter.blur.motion");
    assert!(st(&h).dialogs.form.as_ref().expect("offen").preview_on, "Vorgabe an");
    click_label(&mut h, "Vorschau");
    assert!(!st(&h).dialogs.form.as_ref().expect("offen").preview_on);
    ok(&mut h);
    assert_eq!(st(&h).session.prefs.get("MotionBlurShowPreview").and_then(|v| v.as_bool()), Some(false));
    open(&mut h, "filter.blur.motion");
    assert!(!st(&h).dialogs.form.as_ref().expect("offen").preview_on, "gemerkt: aus");
    // einschalten und abbrechen: bleibt aus
    click_label(&mut h, "Vorschau");
    cancel(&mut h);
    assert_eq!(st(&h).session.prefs.get("MotionBlurShowPreview").and_then(|v| v.as_bool()), Some(false));
    open(&mut h, "filter.blur.motion");
    assert!(!st(&h).dialogs.form.as_ref().expect("offen").preview_on, "Abbrechen merkt nichts");
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
    open(&mut h, "select.expandMarquee");
    assert_eq!(field(&h, "amount"), 25.0, "nach Abbrechen weiter der gemerkte Wert");
    key(&mut h, Key::Escape, Modifiers::NONE);
}

/// Die übrigen Auswahl-/Pfaddialoge: Vorgabe 10 aus den Voreinstellungen.
#[test]
fn selection_and_path_dialogs_default_10() {
    let mut h = harness();
    for (cmd, key_) in [("select.contractMarquee", "amount"), ("select.borderMarquee", "amount"), ("select.smoothMarquee", "amount"), ("modify.alterPath.simplify", "amount"), ("modify.alterPath.insetPath", "distance"), ("modify.alterPath.expandStroke", "width")] {
        open(&mut h, cmd);
        assert_eq!(field(&h, key_), 10.0, "{cmd}");
        key(&mut h, Key::Escape, Modifiers::NONE);
        h.run_steps(2);
        assert!(st(&h).dialogs.form.is_none(), "{cmd}: Esc schließt");
    }
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
    let icons: Vec<egui::Rect> = h.get_all(By::new().role(Role::RadioButton)).map(|n| n.rect()).filter(|r| r.width() < 30.0).collect();
    assert!(icons.len() >= 6, "3 Ecken + 3 Endkappen als Symbolknöpfe: {}", icons.len());
    let first = icons.iter().min_by(|a, b| (a.top(), a.left()).partial_cmp(&(b.top(), b.left())).expect("cmp")).copied().expect("Knopf");
    click(&mut h, first.center());
    assert_eq!(st(&h).dialogs.form.as_ref().expect("offen").get("join"), json!("miter"));
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(4);
    assert_eq!(st(&h).session.prefs.get("ExpandJoinType").and_then(|v| v.as_int()), Some(2), "Gehrung = Code 2");
    open(&mut h, "modify.alterPath.expandStroke");
    assert_eq!(st(&h).dialogs.form.as_ref().expect("offen").get("join"), json!("miter"), "gemerkt");
    key(&mut h, Key::Escape, Modifiers::NONE);
}

/// Pfad einlassen: Breite per Popup-Schieber, Ecke „abgeschrägt“, OK → `InsetAmount`/`InsetJoinType`;
/// zweimal öffnen zeigt die gemerkten Werte, Gehrungsgrenze bleibt eine Ganzzahl-Voreinstellung.
#[test]
fn inset_path_remembers_values() {
    let mut h = harness();
    open(&mut h, "modify.alterPath.insetPath");
    let f = text_fields(&h)[0].0;
    click(&mut h, Pos2::new(f.right() + 8.0, f.center().y));
    let (r, _) = sliders(&h)[0];
    drag(&mut h, Pos2::new(r.center().x, r.top() + 80.0), Pos2::new(r.center().x, r.top() + 60.0), 20);
    let dist = field(&h, "distance");
    assert!(dist != 10.0, "Breite geändert: {dist}");
    click_label(&mut h, "Abgeschrägt");
    assert_eq!(st(&h).dialogs.form.as_ref().expect("offen").get("join"), json!("bevel"));
    ok(&mut h);
    let p = &st(&h).session.prefs;
    assert_eq!(p.get("InsetAmount").and_then(|v| v.as_int()), Some(dist as i64));
    assert_eq!(p.get("InsetJoinType").and_then(|v| v.as_int()), Some(1));
    assert!(matches!(p.get("InsetMiterLimit"), Some(fp_engine::PrefValue::Int(10))), "Ganzzahl wie im Original");
    open(&mut h, "modify.alterPath.insetPath");
    assert_eq!(field(&h, "distance"), dist);
    assert_eq!(st(&h).dialogs.form.as_ref().expect("offen").get("join"), json!("bevel"));
    key(&mut h, Key::Escape, Modifiers::NONE);
}

/// Leinwandfarbe: benutzerdefinierte Farbe aus `CustomCanvasColor` (Vorgabe schwarz) bei weißer Leinwand.
#[test]
fn canvas_color_custom_from_prefs() {
    let mut h = harness();
    open(&mut h, "modify.canvas.canvasColor");
    let f = st(&h).dialogs.form.clone().expect("offen");
    assert_eq!(f.get("_mode").as_i64(), Some(0), "weiß gewählt");
    assert_eq!(f.get("_color").as_str().map(|s| s[..7].to_ascii_lowercase()), Some("#000000".into()), "schwarz wie im Original");
    key(&mut h, Key::Escape, Modifiers::NONE);
    h.state_mut().state.session.prefs.set("CustomCanvasColor", fp_engine::PrefValue::Str("#336699".into()));
    open(&mut h, "modify.canvas.canvasColor");
    assert_eq!(st(&h).dialogs.form.as_ref().expect("offen").get("_color").as_str().map(|s| s[..7].to_ascii_lowercase()), Some("#336699".into()));
    // „Benutzerdefiniert“ + OK: Leinwand wird so, Farbe bleibt gemerkt; ein Verlaufsschritt
    let n0 = undo_count(&h);
    click_label(&mut h, "Benutzerdefiniert:");
    ok(&mut h);
    assert_eq!(undo_count(&h), n0 + 1);
    assert_eq!(st(&h).session.prefs.get_str("CustomCanvasColor", "").to_ascii_lowercase(), "#336699");
}

/// Bildgröße, Leinwandgröße, Numerisch transformieren: Kästchen werden gemerkt (nur bei OK).
#[test]
fn size_dialog_checkboxes_remembered() {
    let mut h = harness();
    // Bildgröße: „Proportionen beibehalten“ aus, OK
    h.state_mut().state.run("modify.canvas.imageSize", json!({}));
    h.run_steps(3);
    assert!(st(&h).dialogs.image_size.as_ref().expect("Bildgröße").constrain);
    click_label(&mut h, "Proportionen beibehalten");
    assert!(!st(&h).dialogs.image_size.as_ref().expect("offen").constrain);
    ok(&mut h);
    assert!(st(&h).dialogs.image_size.is_none());
    assert_eq!(st(&h).session.prefs.get("ResizeConstrain").and_then(|v| v.as_bool()), Some(false));
    h.state_mut().state.run("modify.canvas.imageSize", json!({}));
    h.run_steps(3);
    assert!(!st(&h).dialogs.image_size.as_ref().expect("offen").constrain, "gemerkt");
    // wieder an, abbrechen: bleibt aus
    click_label(&mut h, "Proportionen beibehalten");
    cancel(&mut h);
    assert_eq!(st(&h).session.prefs.get("ResizeConstrain").and_then(|v| v.as_bool()), Some(false));
    // Leinwandgröße: „Nur aktuelle Seite“
    h.state_mut().state.run("modify.canvas.canvasSize", json!({}));
    h.run_steps(3);
    assert!(st(&h).dialogs.canvas_size.as_ref().expect("Leinwandgröße").current_page_only);
    click_label(&mut h, "Nur aktuelle Seite");
    ok(&mut h);
    assert_eq!(st(&h).session.prefs.get("ResizeCanvasCurrentPageOnly").and_then(|v| v.as_bool()), Some(false));
    h.state_mut().state.run("modify.canvas.canvasSize", json!({}));
    h.run_steps(3);
    assert!(!st(&h).dialogs.canvas_size.as_ref().expect("offen").current_page_only, "gemerkt");
    cancel(&mut h);
    // Numerisch transformieren (Rechteck gewählt)
    h.state_mut().state.session.execute("select.all", json!({})).ok();
    h.state_mut().state.run("modify.transform.numericTransform", json!({}));
    h.run_steps(3);
    assert!(st(&h).dialogs.numeric_transform.as_ref().expect("Transformieren").constrain);
    click_label(&mut h, "Proportionen beibehalten");
    ok(&mut h);
    assert_eq!(st(&h).session.prefs.get("TransformConstrain").and_then(|v| v.as_bool()), Some(false));
    h.state_mut().state.run("modify.transform.numericTransform", json!({}));
    h.run_steps(3);
    assert!(!st(&h).dialogs.numeric_transform.as_ref().expect("offen").constrain, "gemerkt");
}

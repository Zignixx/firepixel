//! DLG-8 (P-DLG8 „Auswahl“, Messung PM-6, PARITAET_B D35–D39):
//! - Beschriftungen wie im Original: „Radius:“, „Erweitern um:“, „Verkleinern um:“, „Breite:“,
//!   „Auswahlradius:“, dahinter „Pixel“ (Deutsch und Englisch, ohne Überlauf auch in qps)
//! - Anordnung der Vorlagen 5976/19105–19108: Beschriftung rechtsbündig, Feld, „Pixel“, OK und
//!   Abbrechen rechts untereinander; Größe wie im Original
//! - Grenzen 1…100 (19105–19108, OK-Klemmung des Originals) bzw. 1…1000 (Weiche Kante)
//! - Wirkung auf die Auswahl 330,210,380,260 wie in PM-6 gemessen und Verlaufsnamen des Originals
//!
//! Realistische Eingaben: Klick ins Feld, Strg+A, Tippen wie egui-winit (Taste + Text), Tab; OK per
//! Mausklick.

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::kittest::{By, Queryable};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::{dialog_rect, last_dialog_id};
use fp_ui::FirepixelApp;
use serde_json::{json, Value};

type H = Harness<'static, FirepixelApp>;

/// Auswahl aus PM-6 (D35–D39).
const SEL: [i32; 4] = [330, 210, 380, 260];

/// Dokument 500 × 400 mit der Pixelauswahl [`SEL`].
fn session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 400, "background": "#ffffff"})).expect("doc");
    s.execute("select.pixels", json!({"shape": "rect", "rect": SEL})).expect("Pixelauswahl");
    s
}

fn harness() -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, session(), UiSettings::default(), false);
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

fn key_m(h: &mut H, k: Key, m: Modifiers) {
    if m.any() {
        h.event(Event::ModifiersChanged(m));
    }
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: m });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: m });
    if m.any() {
        ev(h, Event::ModifiersChanged(Modifiers::NONE));
    }
}

fn key(h: &mut H, k: Key) {
    key_m(h, k, Modifiers::NONE);
}

/// Tippen wie egui-winit: Taste drücken + Text, Taste loslassen.
fn typ(h: &mut H, s: &str) {
    for c in s.chars() {
        let k = Key::from_name(&c.to_string());
        if let Some(k) = k {
            h.event(Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: Modifiers::NONE });
        }
        ev(h, Event::Text(c.to_string()));
        if let Some(k) = k {
            ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: Modifiers::NONE });
        }
    }
}

fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
}

fn open(h: &mut H, cmd: &str) {
    h.state_mut().state.run(cmd, json!({}));
    h.run_steps(4);
    assert!(st(h).dialogs.any_open(), "{cmd}: Dialog offen");
}

fn close(h: &mut H) {
    key(h, Key::Escape);
    h.run_steps(4);
    assert!(!st(h).dialogs.any_open(), "Esc schließt");
}

fn dlg_rect(h: &H) -> Rect {
    dialog_rect(&h.ctx, &last_dialog_id(&h.ctx).expect("Dialog")).expect("Rechteck")
}

/// Zahlenfelder des Dialogs von oben nach unten (Rechteck, Text).
fn text_fields(h: &H) -> Vec<(Rect, String)> {
    let dr = dlg_rect(h);
    let mut v: Vec<(Rect, String)> = h.get_all(By::new().role(Role::TextInput)).map(|n| (n.rect(), n.value().unwrap_or_default())).filter(|(r, _)| dr.contains(r.center())).collect();
    v.sort_by(|a, b| a.0.top().total_cmp(&b.0.top()));
    v
}

fn button(h: &H, label: &str) -> Rect {
    let dr = dlg_rect(h);
    h.query_all(By::new().role(Role::Button).label(label)).map(|n| n.rect()).find(|r| dr.contains(r.center())).unwrap_or_else(|| panic!("Knopf {label}"))
}

/// Gezeichnete Beschriftungen im Dialog (Text → Rechteck).
fn labels(h: &mut H) -> Vec<(String, Rect)> {
    let dr = dlg_rect(h);
    fp_ui::inspector::fields::set_audit(&h.ctx, true);
    h.run_steps(1);
    let v = fp_ui::inspector::fields::take_audit(&h.ctx).into_iter().filter(|a| a.kind == "label" && dr.contains(a.drawn.center())).map(|a| (a.text, a.drawn)).collect();
    fp_ui::inspector::fields::set_audit(&h.ctx, false);
    v
}

/// Feld eintippen: Klick, Strg+A, Text, Tab.
fn type_field(h: &mut H, text: &str) {
    let r = text_fields(h)[0].0;
    click(h, r.center());
    key_m(h, Key::A, Modifiers { ctrl: true, command: true, ..Default::default() });
    typ(h, text);
    key(h, Key::Tab);
    h.run_steps(2);
}

fn form_value(h: &H, key: &str) -> Value {
    st(h).dialogs.form.as_ref().expect("Formular").get(key)
}

fn press_ok(h: &mut H) {
    let r = button(h, "OK");
    click(h, r.center());
    h.run_steps(4);
    assert!(!st(h).dialogs.any_open(), "OK schließt");
}

fn undo_label(s: &fp_engine::Session) -> Option<String> {
    s.active().and_then(|d| d.history.undo_label().map(str::to_string))
}

fn sel_bounds(s: &fp_engine::Session) -> [i32; 4] {
    let m = s.active().and_then(|d| d.selection.pixels.clone()).expect("Pixelauswahl");
    let b = m.tight_bounds();
    [b.left, b.top, b.right, b.bottom]
}

fn sel_at(s: &fp_engine::Session, x: i32, y: i32) -> u8 {
    s.active().and_then(|d| d.selection.pixels.as_ref().map(|m| m.get(x, y))).unwrap_or(0)
}

/// Befehl, Feldschlüssel, Beschriftung de/en, Breite des Originals (Fensteraußenmaß PM-6), Obergrenze.
const DIALOGS: [(&str, &str, &str, &str, f32, f64); 5] = [
    ("select.feather", "radius", "Radius:", "Radius:", 314.0, 1000.0),
    ("select.expandMarquee", "amount", "Erweitern um:", "Expand by:", 327.0, 100.0),
    ("select.contractMarquee", "amount", "Verkleinern um:", "Contract by:", 327.0, 100.0),
    ("select.borderMarquee", "amount", "Breite:", "Width:", 287.0, 100.0),
    ("select.smoothMarquee", "amount", "Auswahlradius:", "Sample radius:", 345.0, 100.0),
];

/// D35–D39: Beschriftung und „Pixel“ wie im Original, in Englisch übersetzt; „Rand:“ und
/// „Stichprobenradius:“ sind weg.
#[test]
fn labels_like_original() {
    let mut h = harness();
    for (cmd, _, de, _, _, _) in DIALOGS {
        open(&mut h, cmd);
        let l: Vec<String> = labels(&mut h).into_iter().map(|x| x.0).collect();
        assert!(l.iter().any(|x| x == de), "{cmd}: „{de}“ fehlt in {l:?}");
        assert!(l.iter().any(|x| x == "Pixel"), "{cmd}: „Pixel“ fehlt in {l:?}");
        assert!(!l.iter().any(|x| x == "Rand:" || x == "Stichprobenradius:"), "{cmd}: alte Beschriftung {l:?}");
        close(&mut h);
    }
    fp_i18n::set_language("en");
    for (cmd, _, _, en, _, _) in DIALOGS {
        open(&mut h, cmd);
        let l: Vec<String> = labels(&mut h).into_iter().map(|x| x.0).collect();
        assert!(l.iter().any(|x| x == en), "{cmd}: „{en}“ fehlt in {l:?}");
        close(&mut h);
    }
    fp_i18n::set_language("de");
}

/// D35–D39: Beschriftung rechtsbündig vor dem Feld, „Pixel“ dahinter, OK oben rechts und Abbrechen
/// darunter; Breite wie im Original (± 12 px), quer. In en und qps ohne Überlauf.
#[test]
fn layout_like_original() {
    let mut h = harness();
    for lang in ["de", "en", "qps"] {
        fp_i18n::set_language(lang);
        for (cmd, _, _, _, width, _) in DIALOGS {
            open(&mut h, cmd);
            let _ = fp_ui::dialogs::take_overflows(&h.ctx);
            let f = text_fields(&h);
            assert_eq!(f.len(), 1, "{cmd}: ein Feld");
            let fr = f[0].0;
            let l = labels(&mut h);
            assert_eq!(l.len(), 2, "{cmd} ({lang}): Beschriftung und Einheit {l:?}");
            let (lab, unit) = if l[0].1.left() < l[1].1.left() { (l[0].1, l[1].1) } else { (l[1].1, l[0].1) };
            assert!(lab.right() <= fr.left() && fr.left() - lab.right() <= 10.0, "{cmd} ({lang}): Beschriftung {lab:?} direkt vor dem Feld {fr:?}");
            assert!(unit.left() >= fr.right() && unit.left() - fr.right() <= 10.0, "{cmd} ({lang}): Einheit {unit:?} direkt hinter dem Feld");
            assert!((lab.center().y - fr.center().y).abs() <= 3.0 && (unit.center().y - fr.center().y).abs() <= 3.0, "{cmd} ({lang}): eine Zeile");
            assert!(fr.width() <= 34.0, "{cmd}: schmales Feld wie im Original ({fr:?})");
            let ok_l = fp_i18n::t("btn-ok");
            let cancel_l = fp_i18n::t("btn-cancel");
            let (ok, cancel) = (button(&h, &ok_l), button(&h, &cancel_l));
            assert!(ok.left() > unit.right(), "{cmd} ({lang}): Knöpfe rechts der Einheit");
            assert!(ok.top() < fr.top() && cancel.top() > ok.bottom() && (cancel.left() - ok.left()).abs() < 1.0, "{cmd} ({lang}): OK oben, Abbrechen darunter");
            let dr = dlg_rect(&h);
            assert!(dr.width() > 2.0 * dr.height(), "{cmd}: quer ({dr:?})");
            if lang == "de" {
                assert!((dr.width() - width).abs() <= 12.0, "{cmd}: Breite {} statt ≈ {width}", dr.width());
            }
            h.run_steps(1);
            let o = fp_ui::dialogs::take_overflows(&h.ctx);
            assert!(o.is_empty(), "{cmd} ({lang}): Überlauf {o:?}");
            close(&mut h);
        }
    }
    fp_i18n::set_language("de");
}

/// Grenzen: 19105–19108 1…100 (Original klemmt erst beim OK, FORM-1: Feld zeigt bis dahin den
/// getippten Wert, `form_form1` prüft das OK), Weiche Kante 1…1000 sofort; Vorgabe 10.
#[test]
fn limits_clamp() {
    let mut h = harness();
    for (cmd, key, _, _, _, max) in DIALOGS {
        let on_ok = max == 100.0;
        open(&mut h, cmd);
        assert_eq!(form_value(&h, key).as_f64(), Some(10.0), "{cmd}: Vorgabe 10");
        type_field(&mut h, "5000");
        assert_eq!(form_value(&h, key).as_f64(), Some(if on_ok { 5000.0 } else { max }), "{cmd}: oben");
        type_field(&mut h, "0");
        assert_eq!(form_value(&h, key).as_f64(), Some(if on_ok { 0.0 } else { 1.0 }), "{cmd}: unten");
        close(&mut h);
    }
}

/// Wirkung über den Dialog (Tippen 5, OK): Grenzen wie in PM-6 gemessen, ein Verlaufsschritt mit
/// dem Namen des Originals, Rückgängig stellt die Auswahl her.
#[test]
fn ok_applies_with_original_step_names() {
    let mut h = harness();
    for (cmd, step) in [("select.expandMarquee", "Auswahl erweitern"), ("select.contractMarquee", "Auswahl verkleinern"), ("select.borderMarquee", "Rand auswählen"), ("select.smoothMarquee", "Auswahl glätten"), ("select.feather", "Weiche Auswahlkante")] {
        let n = st(&h).session.active().unwrap().history.undo_count();
        open(&mut h, cmd);
        type_field(&mut h, "5");
        press_ok(&mut h);
        let s = &st(&h).session;
        assert_eq!(s.active().unwrap().history.undo_count(), n + 1, "{cmd}: ein Schritt");
        assert_eq!(undo_label(s).as_deref(), Some(step), "{cmd}: Verlaufsname");
        if cmd == "select.expandMarquee" {
            // PM-6 D36: 330,210,380,260 → 324,204,386,266 (Radius trunc(1,2 · 5) = 6, re\05 §2.2)
            assert_eq!(sel_bounds(s), [324, 204, 386, 266]);
        }
        if cmd == "select.contractMarquee" {
            // PM-6 D37: → 335,215,375,255
            assert_eq!(sel_bounds(s), [335, 215, 375, 255]);
        }
        h.state_mut().state.session.undo().expect("undo");
        h.run_steps(2);
        assert_eq!(sel_bounds(&st(&h).session), SEL, "{cmd}: Rückgängig");
    }
}

/// Wirkung der Befehle (Engine): Erweitern/Verkleinern um 1/5/10 wie die Morphologie in re\05 §2.2
/// (Erweitern +1/+6/+11, Verkleinern −1/−5/−10), Begrenzen ein Ring ohne Mitte, Glätten rundet die
/// Ecken, Weiche Kante macht den Rand weich; englische Verlaufsnamen.
#[test]
fn command_effects() {
    for (amount, grow, shrink) in [(1, 1, 1), (5, 6, 5), (10, 11, 10)] {
        let mut s = session();
        s.execute("select.expandMarquee", json!({"amount": amount})).expect("erweitern");
        assert_eq!(sel_bounds(&s), [SEL[0] - grow, SEL[1] - grow, SEL[2] + grow, SEL[3] + grow], "erweitern {amount}");
        let mut s = session();
        s.execute("select.contractMarquee", json!({"amount": amount})).expect("verkleinern");
        assert_eq!(sel_bounds(&s), [SEL[0] + shrink, SEL[1] + shrink, SEL[2] - shrink, SEL[3] - shrink], "verkleinern {amount}");
    }
    let mut s = session();
    s.execute("select.borderMarquee", json!({"amount": 5})).expect("begrenzen");
    let b = sel_bounds(&s);
    assert!(b[0] < SEL[0] && b[2] > SEL[2], "Rand reicht über die Auswahl hinaus: {b:?}");
    assert_eq!(sel_at(&s, 355, 235), 0, "Mitte frei");
    assert!(sel_at(&s, SEL[0], 235) > 200, "Kante gewählt");
    let mut s = session();
    s.execute("select.pixels", json!({"shape": "rect", "rect": [100, 100, 140, 140], "mode": "add"})).expect("zweites Rechteck");
    s.execute("select.smoothMarquee", json!({"amount": 5})).expect("glätten");
    assert_eq!(sel_at(&s, SEL[0], SEL[1]), 0, "Ecke abgerundet");
    assert_eq!(sel_at(&s, 355, 235), 255, "Mitte bleibt");
    let mut s = session();
    s.execute("select.feather", json!({"radius": 5})).expect("weiche Kante");
    let edge = sel_at(&s, SEL[0], 235);
    assert!(edge > 0 && edge < 255, "weicher Rand ({edge})");
    assert_eq!(sel_at(&s, 355, 235), 255, "Mitte voll");
    fp_i18n::set_language("en");
    for (cmd, step) in [("select.feather", "Feather Selection"), ("select.expandMarquee", "Expand Selection"), ("select.contractMarquee", "Contract Selection"), ("select.borderMarquee", "Border Selection"), ("select.smoothMarquee", "Smooth Selection")] {
        let mut s = session();
        s.execute(cmd, json!({})).unwrap_or_else(|e| panic!("{cmd}: {e}"));
        assert_eq!(undo_label(&s).as_deref(), Some(step), "{cmd}");
    }
    fp_i18n::set_language("de");
}

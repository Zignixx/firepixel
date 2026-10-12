//! DLG-6 (P-DLG6, Messung PM-6, PARITAET_B D47/D49/D50): Weichzeichner-Wirkung und Dialoge.
//! - Grenzen der Ergebnis-Bitmap nach OK wie im Original: Bewegungsunschärfe wächst nur in
//!   Bewegungsrichtung, Radiales Weichzeichnen auf den sichtbaren Inhalt zugeschnitten, Zentriertes
//!   Weichzeichnen unverändert
//! - Beschriftungen „Umfang“ (D47), „Menge“ (D49), „Anzahl“ (D50), Winkel ohne „°“
//! - Anordnung: Felder mit ▾ links, OK/Abbrechen rechts oben, „Vorschau“ darunter; Startfokus OK
//!
//! Realistische Eingaben: Klick ins Feld, Strg+A, Tippen wie egui-winit (Taste + Text), Tab, OK per
//! Mausklick bzw. Enter.

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::kittest::{By, Queryable};
use egui_kittest::Harness;
use fp_raster::IRect;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::{dialog_rect, last_dialog_id};
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

/// Dokument 600 × 400 mit der Bitmap B1 320,200,420,300 (aus einem Rechteck reduziert), ausgewählt.
fn session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 600, "height": 400, "background": "#ffffff"})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [320, 200, 420, 300], "fillColor": "#3366cc", "strokeWidth": 0, "name": "B1"})).expect("rect");
    s.execute("modify.flattenSelection", json!({})).expect("reduzieren");
    assert_eq!(b1_bounds(&s), IRect::new(320, 200, 420, 300), "B1 wie in der Messung");
    s
}

/// Pixelgrenzen der (einzigen) ausgewählten Bitmap.
fn b1_bounds(s: &fp_engine::Session) -> IRect {
    let d = s.active().expect("doc");
    assert_eq!(d.selection.elements.len(), 1, "B1 ausgewählt");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, d.selection.elements[0]).expect("B1");
    let fp_model::ElementKind::Bitmap(b) = &e.kind else { panic!("B1 ist keine Bitmap") };
    fp_engine::docops::bitmap_image(b).expect("Pixel").bounds()
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

/// Gezeichnete Beschriftungen bzw. Kästchen im Dialog (Layout-Protokoll der Felder).
fn audit(h: &mut H) -> Vec<(String, &'static str, Rect)> {
    let dr = dlg_rect(h);
    fp_ui::inspector::fields::set_audit(&h.ctx, true);
    h.run_steps(1);
    let v = fp_ui::inspector::fields::take_audit(&h.ctx).into_iter().filter(|a| dr.contains(a.drawn.center())).map(|a| (a.text, a.kind, a.drawn)).collect();
    fp_ui::inspector::fields::set_audit(&h.ctx, false);
    v
}

/// Zahl `i` (von oben) eintippen: Klick, Strg+A, Text, Tab.
fn type_field(h: &mut H, i: usize, text: &str) {
    let r = text_fields(h)[i].0;
    click(h, r.center());
    key_m(h, Key::A, Modifiers { ctrl: true, command: true, ..Default::default() });
    typ(h, text);
    key(h, Key::Tab);
    h.run_steps(2);
}

fn press_ok(h: &mut H) {
    let r = button(h, "OK");
    click(h, r.center());
    h.run_steps(4);
    assert!(!st(h).dialogs.any_open(), "OK schließt");
}

fn undo_count(h: &H) -> usize {
    st(h).session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

/// Gemessene Fälle (PM-6): Dialog, getippte Werte, Grenzen von B1 danach.
const MEASURED: &[(&str, [&str; 2], IRect)] = &[
    ("filter.blur.motion", ["90", "20"], IRect { left: 320, top: 200, right: 420, bottom: 319 }),
    ("filter.blur.radial", ["10", "20"], IRect { left: 317, top: 197, right: 424, bottom: 304 }),
    ("filter.blur.zoom", ["10", "20"], IRect { left: 320, top: 200, right: 420, bottom: 300 }),
    ("filter.blur.gaussian", ["2", ""], IRect { left: 315, top: 195, right: 425, bottom: 305 }),
];

/// D47/D49/D50 (und D48 zum Vergleich): Werte im Dialog tippen, OK → Grenzen der Bitmap wie im
/// Original, genau ein Verlaufsschritt „Bild filtern“; Rückgängig stellt 320,200,420,300 her.
#[test]
fn result_bounds_like_the_original() {
    let mut h = harness();
    for (cmd, vals, want) in MEASURED {
        let n0 = undo_count(&h);
        open(&mut h, cmd);
        for (i, v) in vals.iter().enumerate().filter(|(_, v)| !v.is_empty()) {
            type_field(&mut h, i, v);
        }
        press_ok(&mut h);
        assert_eq!(b1_bounds(&st(&h).session), *want, "{cmd}: Grenzen nach OK");
        assert_eq!(undo_count(&h), n0 + 1, "{cmd}: ein Verlaufsschritt");
        assert_eq!(st(&h).session.active().unwrap().history.undo_label(), Some("Bild filtern"), "{cmd}");
        key_m(&mut h, Key::Z, Modifiers { ctrl: true, command: true, ..Default::default() });
        h.run_steps(3);
        assert_eq!(b1_bounds(&st(&h).session), IRect::new(320, 200, 420, 300), "{cmd}: Strg+Z stellt B1 her");
    }
}

/// Die Wirkung bleibt nach Wiederholung gleich: Bewegungsunschärfe zweimal (Filter wiederholen)
/// wächst jeweils nur nach unten.
#[test]
fn motion_blur_twice_grows_downward_only() {
    let mut s = session();
    s.execute("filter.blur.motion", json!({"angle": 90, "distance": 20})).expect("1");
    assert_eq!(b1_bounds(&s), IRect::new(320, 200, 420, 319));
    s.execute("filter.blur.motion", json!({"angle": 90, "distance": 20})).expect("2");
    // die neue letzte Zeile (Alpha 12) trägt eine Zeile weiter nur noch 12 / 20 = 0 bei
    assert_eq!(b1_bounds(&s), IRect::new(320, 200, 420, 337));
    s.execute("filter.blur.motion", json!({"angle": 0, "distance": 10})).expect("3");
    assert_eq!(b1_bounds(&s), IRect::new(311, 200, 420, 337), "Winkel 0: Unschärfe nach links");
}

/// Beschriftungen wie im Original: „Winkel“/„Umfang“ (ohne „°“), „Menge“/„Qualität“,
/// „Anzahl“/„Qualität“; im Englischen „Distance“ und „Amount“.
#[test]
fn labels_like_the_original() {
    let mut h = harness();
    for (cmd, want, not) in [
        ("filter.blur.motion", &["Winkel:", "Umfang:"][..], &["Abstand:", "°"][..]),
        ("filter.blur.radial", &["Menge:", "Qualität:"][..], &["Stärke:", "Anzahl:"][..]),
        ("filter.blur.zoom", &["Anzahl:", "Qualität:"][..], &["Stärke:", "Menge:"][..]),
    ] {
        open(&mut h, cmd);
        let l: Vec<String> = audit(&mut h).into_iter().map(|a| a.0).collect();
        for w in want {
            assert!(l.iter().any(|x| x == w), "{cmd}: „{w}“ fehlt in {l:?}");
        }
        for n in not {
            assert!(!l.iter().any(|x| x == n), "{cmd}: „{n}“ noch da: {l:?}");
        }
        close(&mut h);
    }
    fp_i18n::set_language("en");
    assert_eq!(fp_i18n::t("dlg-lbl-motion-distance"), "Distance:");
    assert_eq!(fp_i18n::t("dlg-lbl-zoom-amount"), "Amount:");
    fp_i18n::set_language("de");
}

/// Anordnung D47/D49/D50 wie im Original (Bilder `pm6_D47/D49/D50`): zwei Felder mit ▾
/// untereinander links, OK und Abbrechen rechts daneben (OK oben), „Vorschau“ in der Knopfspalte
/// unter Abbrechen, Dialog quer.
#[test]
fn layout_fields_left_buttons_right() {
    let mut h = harness();
    for cmd in ["filter.blur.motion", "filter.blur.radial", "filter.blur.zoom"] {
        open(&mut h, cmd);
        let f = text_fields(&h);
        assert_eq!(f.len(), 2, "{cmd}: zwei Felder");
        assert!(f[1].0.top() > f[0].0.bottom() && (f[0].0.left() - f[1].0.left()).abs() < 1.0, "{cmd}: untereinander");
        let (ok, cancel) = (button(&h, "OK"), button(&h, "Abbrechen"));
        let right = f.iter().map(|x| x.0.right()).fold(0.0, f32::max);
        assert!(ok.left() > right + 14.0, "{cmd}: Knöpfe rechts neben Feld und ▾ ({ok:?}, Feld bis {right})");
        assert!(ok.top() <= f[0].0.top() + 2.0, "{cmd}: OK oben");
        assert!(cancel.top() > ok.bottom() && (cancel.left() - ok.left()).abs() < 1.0, "{cmd}: Abbrechen unter OK");
        let a = audit(&mut h);
        let preview = a.iter().find(|x| x.1 == "check" && x.0 == "Vorschau").unwrap_or_else(|| panic!("{cmd}: Vorschau {a:?}")).2;
        assert!(preview.left() >= ok.left() - 1.0 && preview.top() > cancel.bottom(), "{cmd}: Vorschau in der Knopfspalte");
        let dr = dlg_rect(&h);
        assert!(dr.width() > dr.height(), "{cmd}: quer ({dr:?})");
        assert!(ok.right() <= dr.right() && preview.right() <= dr.right() + 0.5, "{cmd}: nichts ragt heraus");
        close(&mut h);
    }
}

/// Startfokus OK (PM-6 D47/D49/D50): kein Feld fokussiert, Enter = OK mit den Vorgaben.
#[test]
fn start_focus_ok() {
    let mut h = harness();
    for cmd in ["filter.blur.motion", "filter.blur.radial", "filter.blur.zoom"] {
        let n0 = undo_count(&h);
        open(&mut h, cmd);
        assert!(!h.ctx.text_edit_focused(), "{cmd}: Startfokus OK, kein Feld");
        key(&mut h, Key::Enter);
        h.run_steps(4);
        assert!(!st(&h).dialogs.any_open(), "{cmd}: Enter = OK");
        assert_eq!(undo_count(&h), n0 + 1, "{cmd}: ein Verlaufsschritt");
    }
}

//! CF-FP4 (`docs\funktionstest\fp4_2026-10-17.md`): Alt-Tipp mit der Ereignisfolge, die
//! egui-winit 0.36 bei `keybd_event` VK_MENU liefert (FP4-04), und der Pfeil ▾ an den Feldern des
//! Farbmischers (R-7 `mixer.field.r.slider`).

use egui::{Event, Key, Modifiers};
use egui_kittest::Harness;
use fp_ui::app_state::UiSettings;
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness() -> H {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h
}

fn key(k: Key, pressed: bool, m: Modifiers) -> Event {
    Event::Key { key: k, physical_key: Some(k), pressed, repeat: false, modifiers: m }
}

/// Was winit 0.30 bei `keybd_event(VK_MENU)` down/up schickt: je Nachricht erst
/// `ModifiersChanged`, dann `KeyboardInput` mit `KeyCode::AltLeft`; egui-winit macht daraus
/// `Event::Key { key: AltLeft }` (seit egui 0.36). Das Drücken von Alt selbst brach den Tipp ab.
fn alt_tap() -> Vec<Event> {
    vec![Event::ModifiersChanged(Modifiers::ALT), key(Key::AltLeft, true, Modifiers::ALT), Event::ModifiersChanged(Modifiers::NONE), key(Key::AltLeft, false, Modifiers::NONE)]
}

fn kbd(h: &H) -> fp_ui::menu_kbd::KbdMenu {
    fp_ui::menu_kbd::state(&h.ctx)
}

#[test]
fn fp4_04_alt_tap_with_alt_key_events_activates_menu_bar() {
    let mut h = harness();
    for e in alt_tap() {
        h.event(e);
    }
    h.run_steps(2);
    let s = kbd(&h);
    assert!(s.active && s.path.is_empty(), "Alt tap (with Key::AltLeft) activates the menu bar ({s:?})");
    for e in alt_tap() {
        h.event(e);
    }
    h.run_steps(2);
    assert!(!kbd(&h).active, "second Alt tap: menu bar inactive");
}

#[test]
fn fp4_04_alt_tap_across_frames_with_repeats_then_letter_opens_menu() {
    let mut h = harness();
    let tool0 = h.state().state.session.tools.active;
    // Alt drücken, gehalten (Wiederholungen), im nächsten Bild loslassen, dann A
    h.event(Event::ModifiersChanged(Modifiers::ALT));
    h.event(key(Key::AltLeft, true, Modifiers::ALT));
    h.run_steps(1);
    h.event(Event::Key { key: Key::AltLeft, physical_key: Some(Key::AltLeft), pressed: true, repeat: true, modifiers: Modifiers::ALT });
    h.run_steps(1);
    h.event(Event::ModifiersChanged(Modifiers::NONE));
    h.event(key(Key::AltLeft, false, Modifiers::NONE));
    h.run_steps(1);
    assert!(kbd(&h).active, "Alt tap across frames activates the menu bar");
    for e in [key(Key::A, true, Modifiers::NONE), Event::Text("a".into()), key(Key::A, false, Modifiers::NONE)] {
        h.event(e);
    }
    h.run_steps(2);
    assert_eq!(h.state().state.session.tools.active, tool0, "A is an access key, not a tool shortcut");
}

#[test]
fn fp4_04_alt_with_letter_is_no_tap() {
    let mut h = harness();
    for e in [
        Event::ModifiersChanged(Modifiers::ALT),
        key(Key::AltLeft, true, Modifiers::ALT),
        key(Key::T, true, Modifiers::ALT),
        key(Key::T, false, Modifiers::ALT),
        Event::ModifiersChanged(Modifiers::NONE),
        key(Key::AltLeft, false, Modifiers::NONE),
    ] {
        h.event(e);
    }
    h.run_steps(2);
    let s = kbd(&h);
    assert!(s.active && !s.path.is_empty(), "Alt+T opens the menu, release is no tap ({s:?})");
}

/// R-7: Jedes Kanalfeld des Farbmischers hat rechts einen Pfeil ▾ (Popup-Schieber); Pfeil und
/// Modellauswahl überlappen nicht.
#[test]
fn mixer_fields_have_slider_arrows() {
    let area = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(240.0, 96.0));
    for i in 0..3 {
        let (f, a) = fp_ui::panels::color::mixer_field_rects(area, i);
        assert_eq!(a.left(), f.right(), "Pfeil direkt rechts am Feld {i}");
        assert_eq!(a.height(), f.height());
        assert!(a.width() >= 9.0);
        assert!(a.right() < fp_ui::panels::color::mixer_model_left(area), "Pfeil {i} vor der Modellauswahl");
    }
}

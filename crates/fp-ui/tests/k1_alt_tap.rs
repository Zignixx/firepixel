//! K-1 (FP2-03): Alt allein mit echten Systemtasten. Drücken und Loslassen kommen dann oft im
//! selben Bild (egui-winit meldet Alt nur als `ModifiersChanged`, nie als `Event::Key`); der
//! Alt-Tipp muss trotzdem die Menüleiste aktivieren, ein folgender Buchstabe wirkt als
//! Zugriffstaste und nicht als Werkzeugkürzel.

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

fn kbd(h: &H) -> fp_ui::menu_kbd::KbdMenu {
    fp_ui::menu_kbd::state(&h.ctx)
}

#[test]
fn k1_alt_tap_in_one_frame_activates_menu_bar() {
    let mut h = harness();
    h.event(Event::ModifiersChanged(Modifiers::ALT));
    h.event(Event::ModifiersChanged(Modifiers::NONE));
    h.run_steps(2);
    let s = kbd(&h);
    assert!(s.active && s.path.is_empty(), "Alt tap within one frame: menu bar active, no menu open ({s:?})");
    // zweiter Tipp im selben Bild schaltet wieder aus
    h.event(Event::ModifiersChanged(Modifiers::ALT));
    h.event(Event::ModifiersChanged(Modifiers::NONE));
    h.run_steps(2);
    assert!(!kbd(&h).active, "second Alt tap: menu bar inactive");
}

#[test]
fn k1_alt_tap_then_letter_same_frame_opens_menu() {
    let mut h = harness();
    // Alt, Alt los, T, T los – alles in einem Bild (keybd_event ohne Pause)
    for e in [Event::ModifiersChanged(Modifiers::ALT), Event::ModifiersChanged(Modifiers::NONE), key(Key::T, true, Modifiers::NONE), Event::Text("t".into()), key(Key::T, false, Modifiers::NONE)] {
        h.event(e);
    }
    h.run_steps(2);
    let s = kbd(&h);
    assert!(s.active && !s.path.is_empty(), "T after Alt tap opens a menu ({s:?})");
    let titles: Vec<String> = fp_ui::menu::build_main_menu_where(&h.state().state, &|_| false).into_iter().map(|(t, _)| t).collect();
    let want = titles.iter().position(|t| fp_ui::menu_kbd::access_of(&format!("/{t}")).map(|a| a.1) == Some('t')).expect("menu with access key T");
    assert_eq!(s.top, want, "menu with access key T");
}

#[test]
fn k1_alt_with_letter_in_one_frame_is_no_tap() {
    let mut h = harness();
    // Alt+T in einem Bild: Zugriffstaste, beim Loslassen kein Alt-Tipp (Menü bleibt offen)
    for e in [Event::ModifiersChanged(Modifiers::ALT), key(Key::T, true, Modifiers::ALT), key(Key::T, false, Modifiers::ALT), Event::ModifiersChanged(Modifiers::NONE)] {
        h.event(e);
    }
    h.run_steps(2);
    let s = kbd(&h);
    assert!(s.active && !s.path.is_empty(), "Alt+T opens the menu, release is no tap ({s:?})");
}

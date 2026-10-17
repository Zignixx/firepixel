//! FIX-E (CT-3, DES-2): Im ganzen Hauptfenster schaltete Tab auf dem Startbildschirm die
//! Bedienfelder aus/ein (Kürzel „Bedienfelder ausblenden“), und die Pfeiltasten bewegten den Fokus
//! nicht. Auf dem Startbildschirm gilt Tab nur für die Fokusfolge der Einträge; Pfeiltasten wandern
//! räumlich zwischen Karten und Einträgen. Mit Dokument bleibt Tab das Kürzel.

use egui::{Event, Id, Key, Modifiers};
use egui_kittest::Harness;
use fp_ui::app_state::UiSettings;
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness(s: fp_engine::Session) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1280.0, 860.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn key(h: &mut H, k: Key, m: Modifiers) {
    for pressed in [true, false] {
        h.event(Event::Key { key: k, physical_key: Some(k), pressed, repeat: false, modifiers: m });
        h.run_steps(1);
    }
    h.run_steps(1);
}

fn focused(h: &H) -> Option<Id> {
    h.ctx.memory(|m| m.focused())
}

fn new_doc_id() -> Id {
    Id::new(("fp-welcome-new", "welcome-new-document"))
}

fn template_id() -> Id {
    Id::new(("fp-welcome-new", "welcome-new-template"))
}

#[test]
fn tab_on_the_start_page_moves_focus_and_keeps_the_panels() {
    fp_engine::shell::disable();
    let mut h = harness(fp_engine::Session::default());
    assert!(h.state().state.session.documents().is_empty());
    assert!(!h.state().state.settings.workspace.panels_hidden);
    key(&mut h, Key::Tab, Modifiers::NONE);
    assert!(!h.state().state.settings.workspace.panels_hidden, "Tab blendet die Bedienfelder nicht aus");
    assert_eq!(focused(&h), Some(new_doc_id()), "Tab: erster Eintrag des Startbildschirms");
    key(&mut h, Key::Tab, Modifiers::NONE);
    assert_eq!(focused(&h), Some(template_id()), "Tab: nächster Eintrag");
    key(&mut h, Key::Tab, Modifiers::SHIFT);
    assert_eq!(focused(&h), Some(new_doc_id()), "Umschalt+Tab: zurück");
    // Pfeiltasten: ab → Aus Vorlage, auf → zurück
    key(&mut h, Key::ArrowDown, Modifiers::NONE);
    assert_eq!(focused(&h), Some(template_id()), "Pfeil ab");
    key(&mut h, Key::ArrowUp, Modifiers::NONE);
    assert_eq!(focused(&h), Some(new_doc_id()), "Pfeil auf");
    // Wiederholung (§4b): 30× Tab, Fokus bleibt im Startbildschirm, Bedienfelder bleiben
    for _ in 0..30 {
        key(&mut h, Key::Tab, Modifiers::NONE);
        assert!(!h.state().state.settings.workspace.panels_hidden);
    }
    assert!(focused(&h).is_some(), "Fokus nicht verloren");
    // Eingabe löst den fokussierten Eintrag aus (Fokus zurück auf „Firepixel-Dokument“)
    while focused(&h) != Some(new_doc_id()) {
        key(&mut h, Key::Tab, Modifiers::NONE);
    }
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert_eq!(fp_ui::welcome::last_triggered(&h.ctx).as_deref(), Some("file.new"));
}

#[test]
fn arrows_without_focus_start_at_the_first_entry() {
    fp_engine::shell::disable();
    let mut h = harness(fp_engine::Session::default());
    assert_eq!(focused(&h), None);
    key(&mut h, Key::ArrowDown, Modifiers::NONE);
    assert_eq!(focused(&h), Some(new_doc_id()));
}

#[test]
fn with_a_document_tab_still_hides_the_panels() {
    fp_engine::shell::disable();
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 40, "height": 30})).expect("doc");
    let mut h = harness(s);
    key(&mut h, Key::Tab, Modifiers::NONE);
    assert!(h.state().state.settings.workspace.panels_hidden, "Tab = Bedienfelder ausblenden wie im Original");
    key(&mut h, Key::Tab, Modifiers::NONE);
    assert!(!h.state().state.settings.workspace.panels_hidden);
}

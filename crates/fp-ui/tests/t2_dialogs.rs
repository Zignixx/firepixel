//! T-2 (UX-B D01–D25, BEFUNDE Z. 546): Funktionspunkte der Dialoge, die beim Abgleich am 15.10.
//! noch offen waren. D16 Voreinstellungen ▸ Text: „Größe der Schriftvorschau“ mit Kombifeld
//! Klein/Mittelgroß/Groß; gespeichert wird wie im Original der angezeigte Text (re\14 §3.4).

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{By, Queryable};
use egui_kittest::Harness;
use fp_ui::app_state::UiSettings;
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness() -> H {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("Dokument");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1800.0, 1130.0)).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn click(h: &mut H, p: Pos2) {
    for e in [
        Event::PointerMoved(p),
        Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE },
        Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE },
    ] {
        h.event(e);
        h.run_steps(1);
    }
    h.run_steps(2);
}

#[test]
fn d16_font_preview_size_combo_stores_text() {
    fp_i18n::set_language("de");
    // gespeicherte Texte (auch der anderen Sprache) → Eintrag; leer/unbekannt = Mittelgroß
    assert_eq!(fp_ui::dialogs::font_preview_index("Klein"), 0);
    assert_eq!(fp_ui::dialogs::font_preview_index("Large"), 2);
    assert_eq!(fp_ui::dialogs::font_preview_index(""), 1);
    assert_eq!(fp_ui::dialogs::font_preview_index("Mittelgroß"), 1);

    let mut h = harness();
    let mut d = fp_ui::dialogs::PreferencesDialog::new(&h.state().state);
    d.category = 3;
    h.state_mut().state.dialogs.preferences = Some(d);
    h.run_steps(4);
    let combo = h.get_all(By::new().role(egui::accesskit::Role::ComboBox)).find(|n| n.value().as_deref() == Some("Mittelgroß")).map(|n| n.rect()).expect("Kombifeld „Mittelgroß“ neben „Größe der Schriftvorschau“");
    click(&mut h, combo.center());
    let item = h.get_by_label("Groß").rect();
    click(&mut h, item.center());
    let d = h.state().state.dialogs.preferences.clone().expect("Dialog offen");
    let mut prefs = fp_engine::Preferences::new();
    d.apply(&mut prefs);
    assert_eq!(prefs.get_str("FontPreviewSize", ""), "Groß", "lokalisierter Text wie im Original");
    assert!(prefs.get_bool("ShowFontPreview", true), "Kästchen bleibt an");
}

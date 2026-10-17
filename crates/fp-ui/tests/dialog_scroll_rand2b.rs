//! RAND-2b (R2-3): Dialoge, die höher bzw. breiter als das Hauptfenster sind (Tastaturbefehle,
//! Voreinstellungen, KI-Modelle bei 800 × 512 pt), werden auf das Fenster begrenzt; der Inhalt
//! läuft mit Bildlauf, die Knopfzeile bleibt sichtbar und bedienbar.

use egui::Rect;
use egui_kittest::kittest::{By, Queryable};
use egui_kittest::Harness;
use fp_engine::Session;
use fp_ui::app_state::UiSettings;
use fp_ui::dialogs::{dialog_rect, last_dialog_id};
use fp_ui::FirepixelApp;
use serde_json::json;

fn harness(w: f32, h: f32) -> Harness<'static, FirepixelApp> {
    let ctx = egui::Context::default();
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200})).expect("doc");
    let a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    Harness::builder().with_size(egui::vec2(w, h)).with_pixels_per_point(1.0).with_max_steps(16).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a)
}

fn open(h: &mut Harness<'static, FirepixelApp>, which: &str) {
    let st = &mut h.state_mut().state;
    match which {
        "prefs" => st.dialogs.preferences = Some(fp_ui::dialogs::PreferencesDialog::new(st)),
        cmd => {
            assert!(fp_ui::dialogs::open_for_command(st, cmd), "{cmd} öffnet einen Dialog");
        }
    }
    h.run_steps(4);
}

/// unterster Knopf mit der Beschriftung (der Dialog kann das ganze Fenster bedecken)
fn button(h: &Harness<'static, FirepixelApp>, key: &str, within: Rect) -> Rect {
    let label = fp_i18n::t(key);
    h.query_all(By::new().label(&label)).map(|n| n.rect()).filter(|r| within.contains_rect(*r)).max_by(|a, b| a.bottom().total_cmp(&b.bottom())).unwrap_or_else(|| panic!("Knopf „{label}“ im Dialog {within:?}"))
}

#[test]
fn tall_dialogs_fit_the_window_and_keep_their_buttons() {
    for (which, ok) in [("prefs", "btn-ok"), ("edit.keyboardShortcuts", "btn-ok"), ("ai.manageModels", "btn-close")] {
        let mut h = harness(800.0, 512.0);
        open(&mut h, which);
        let screen = h.ctx.content_rect();
        let id = last_dialog_id(&h.ctx).expect("Dialog gezeichnet");
        let r = dialog_rect(&h.ctx, &id).expect("Dialogrechteck");
        assert!(screen.expand(0.5).contains_rect(r), "{which}: Dialog {r:?} ragt aus {screen:?}");
        let b = button(&h, ok, r);
        assert!(b.bottom() <= r.bottom() && b.bottom() > r.bottom() - 48.0, "{which}: Knopf {b:?} unten im Dialog {r:?}");
        // Knopf wirkt: Klick schließt den Dialog
        let p = b.center();
        h.event(egui::Event::PointerMoved(p));
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE });
        }
        h.run_steps(4);
        assert!(!h.state().state.dialogs.any_open(), "{which}: {ok} schließt den Dialog");
    }
}

#[test]
fn small_dialogs_are_unchanged_in_a_large_window() {
    let mut h = harness(1500.0, 950.0);
    open(&mut h, "edit.keyboardShortcuts");
    let id = last_dialog_id(&h.ctx).expect("Dialog gezeichnet");
    let r = dialog_rect(&h.ctx, &id).expect("Dialogrechteck");
    // volle Höhe 560 + Titelleiste 28 + Rahmen 2
    assert!(r.height() >= 589.0, "Tastaturbefehle ungekürzt: {r:?}");
}

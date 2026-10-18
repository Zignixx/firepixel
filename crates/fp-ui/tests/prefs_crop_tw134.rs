//! TW-134: the "delete objects when cropping" check box (Preferences ▸ Edit) writes the preference
//! key the engine reads when setting a new canvas rectangle.

use egui_kittest::kittest::Queryable;
use fp_engine::Session;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::PreferencesDialog;

const KEY: &str = "CropDocCropsImages";

#[test]
fn preferences_crop_option_writes_the_engine_key() {
    fp_i18n::set_language("en");
    let st = AppState::new(Session::default(), UiSettings::default(), None);
    assert!(st.session.prefs.get_bool(KEY, false), "factory default on");
    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1100.0, 800.0))
        .build_ui_state(|ui, st: &mut AppState| fp_ui::dialogs::show_all(ui.ctx(), st), st);
    fp_ui::theme::install_fonts(&harness.ctx);
    harness.run_steps(2);
    let mut d = PreferencesDialog::new(harness.state());
    d.category = 1; // Edit
    harness.state_mut().dialogs.preferences = Some(d);
    harness.run_steps(3);
    harness.get_by_label(&fp_i18n::t("dlg-pref-cropdoccropsimages")).click();
    harness.run_steps(3);
    harness.get_by_label(&fp_i18n::t("btn-ok")).click();
    harness.run_steps(3);
    assert!(harness.state().dialogs.preferences.is_none(), "OK closes");
    assert!(
        !harness.state().session.prefs.get_bool(KEY, true),
        "unchecking stores the key the crop routine reads"
    );
}

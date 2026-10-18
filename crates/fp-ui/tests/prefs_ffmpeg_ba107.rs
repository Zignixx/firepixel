//! BA-107: Preferences show the ffmpeg file row; it reads and writes the same setting as the Batch
//! Studio's settings (`fp_batch::video::ffmpeg::configured_path`).

use egui_kittest::kittest::Queryable;
use fp_engine::Session;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::PreferencesDialog;
use fp_ui::dialogs::PREFERENCE_FFMPEG_KEY as FFMPEG_KEY;
use fp_engine::PrefValue;

#[test]
fn preferences_ffmpeg_row_reads_and_writes_the_batch_setting() {
    fp_i18n::set_language("en");
    let before = fp_batch::video::ffmpeg::configured_path();
    let st = AppState::new(Session::default(), UiSettings::default(), None);
    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1100.0, 800.0))
        .build_ui_state(|ui, st: &mut AppState| fp_ui::dialogs::show_all(ui.ctx(), st), st);
    fp_ui::theme::install_fonts(&harness.ctx);
    harness.run_steps(2);
    // the row sits on the last page ("Plug-ins", with the other folders)
    let page = fp_ui::dialogs::PREFERENCE_CATEGORIES.len() - 1;
    let file = std::env::temp_dir().join(format!("fp-ba107-{}", std::process::id())).join("ffmpeg.exe");
    let mut d = PreferencesDialog::new(harness.state());
    d.category = page;
    d.values.insert(FFMPEG_KEY.into(), PrefValue::Str(file.display().to_string()));
    harness.state_mut().dialogs.preferences = Some(d);
    harness.run_steps(3);
    assert!(harness.query_all_by_value(&file.display().to_string()).next().is_some(), "row shows the typed file");
    harness.get_by_label(&fp_i18n::t("btn-ok")).click();
    harness.run_steps(3);
    assert!(harness.state().dialogs.preferences.is_none(), "OK closes");
    assert_eq!(fp_batch::video::ffmpeg::configured_path(), Some(file.clone()));
    assert!(harness.state().session.prefs.get(FFMPEG_KEY).is_none(), "not a preference");
    // the reopened dialog shows the stored file
    let st = harness.state_mut();
    let mut d = PreferencesDialog::new(st);
    d.category = page;
    st.dialogs.preferences = Some(d);
    harness.run_steps(3);
    assert!(harness.query_all_by_value(&file.display().to_string()).next().is_some(), "stored file shown");
    let _ = fp_batch::video::ffmpeg::set_configured_path(before.as_deref());
}

//! FIX-R3 (13.10.): HTML-Einrichtung, Register „Allgemein“ – die rechte Spalte rückt bei langen
//! Texten (Testsprache `qps`) nach rechts, statt dass „Kommentare“ in „CSS für Pop-up-Menüs“
//! läuft; Deutsch und Englisch bleiben unverändert (rechte Spalte 190 + 18 px neben der linken).

use egui_kittest::kittest::Queryable;
use fp_engine::Session;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::html_setup::{self, HtmlSetupDialog};
use serde_json::json;

fn plain() -> AppState {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 200, "height": 120})).expect("file.new");
    AppState::new(s, UiSettings::default(), None)
}

/// Abstand linke → rechte Spalte und Breite des Kommentar-Texts.
fn columns(lang: &str) -> (f32, f32) {
    fp_i18n::with_language(lang, || {
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1400.0, 700.0)).build_ui_state(|ui, st: &mut AppState| html_setup::show(ui.ctx(), st), plain());
        let d = HtmlSetupDialog::new(h.state()).unwrap();
        h.state_mut().dialogs.html_setup = Some(d);
        h.run_steps(3);
        let (com_l, pop_l) = (fp_i18n::t("dlg-hs-comments"), fp_i18n::t("dlg-hs-css-popups"));
        let (cr, pr) = (h.get_by_label(&com_l).rect(), h.get_by_label(&pop_l).rect());
        let font = fp_ui::theme::Theme::current(&h.ctx).ui_font();
        let text_w = h.ctx.fonts_mut(|f| f.layout_no_wrap(com_l.clone(), font, egui::Color32::WHITE).size().x);
        ((pr.left() - cr.left()) as f32, cr.left() as f32 + 18.0 + text_w - pr.left() as f32)
    })
}

#[test]
fn html_setup_columns_do_not_overlap_with_long_texts() {
    for lang in ["de", "en"] {
        let (gap, _) = columns(lang);
        assert!((gap - 208.0).abs() < 0.6, "{lang}: Spaltenabstand unverändert 208, ist {gap}");
    }
    assert!(fp_i18n::with_language(fp_i18n::PSEUDO_LANG, || fp_i18n::t("dlg-hs-comments")).contains("[!!"), "Testsprache aktiv");
    let (gap, overlap) = columns(fp_i18n::PSEUDO_LANG);
    assert!(gap >= 208.0 - 0.6, "qps: {gap}");
    assert!(overlap < 0.0, "qps: Kommentar-Text endet vor der rechten Spalte ({overlap})");
}

//! FIX-G2 (TWEAKS 13.10.): HTML-Einrichtung mit „CSS in externe Datei schreiben“ und Anhang
//! „scroll“ (TW-03/TW-06), Rauschen-Vorgabe nach Neustart (TW-09), vorbelegter Pfad des
//! persönlichen Wörterbuchs (TW-13) und Exportvorgabe ohne Segmente (Q-07).

use fp_engine::Session;
use fp_export::doc_export::FilesStyle;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::export::{self, style_index, ExportDialog};
use fp_ui::dialogs::html_setup::{self, HtmlSetupDialog};
use egui_kittest::kittest::{NodeT, Queryable};
use serde_json::json;

fn plain() -> AppState {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 200, "height": 120})).expect("file.new");
    s.execute("draw.rectangle", json!({"rect": [10, 10, 100, 100], "fillColor": "#ff0000"})).expect("rect");
    AppState::new(s, UiSettings::default(), None)
}

fn html_settings(st: &AppState) -> fp_model::HtmlSettings {
    let d = st.session.active().unwrap();
    d.doc.pages[d.page].root.export.as_ref().and_then(|e| e.html.as_deref()).cloned().unwrap_or_else(fp_model::HtmlSettings::cs6_default)
}

#[test]
fn html_setup_external_css_and_scroll() {
    let mut st = plain();
    let mut d = HtmlSetupDialog::new(&st).expect("Dialog");
    assert_eq!(d.attachment, 1, "Anhang-Vorgabe „scroll“");
    assert!(d.external_css, "CSS in externe Datei: Vorgabe an");
    assert!(d.unchanged());
    d.external_css = false;
    d.attachment = 0;
    html_setup::apply(&mut st, &d);
    let h = html_settings(&st);
    assert_eq!(h.external_css, Some(0));
    assert_eq!(h.background_attachment, Some(0), "fixed = 0");
    let d2 = HtmlSetupDialog::new(&st).unwrap();
    assert!(!d2.external_css);
    assert_eq!(d2.attachment, 0);
}

#[test]
fn html_setup_draws_two_columns_in_both_languages() {
    for lang in ["de", "en"] {
        fp_i18n::set_language(lang);
        let mut harness = egui_kittest::Harness::builder().with_size(egui::vec2(900.0, 700.0)).build_ui_state(|ui, st: &mut AppState| html_setup::show(ui.ctx(), st), plain());
        for popups in [true, false] {
            let mut d = HtmlSetupDialog::new(harness.state()).unwrap();
            d.css_popups = popups;
            harness.state_mut().dialogs.html_setup = Some(d);
            harness.run_steps(3);
            assert!(harness.state().dialogs.html_setup.is_some(), "{lang}");
            let (ext_l, pop_l, com_l) = (fp_i18n::t("dlg-hs-external-css"), fp_i18n::t("dlg-hs-css-popups"), fp_i18n::t("dlg-hs-comments"));
            let ext = harness.get_by_label(&ext_l);
            let pop = harness.get_by_label(&pop_l);
            let com = harness.get_by_label(&com_l);
            let (er, pr, cr) = (ext.rect(), pop.rect(), com.rect());
            assert!((er.left() - pr.left()).abs() < 1.0 && er.top() > pr.top(), "{lang}: externe CSS unter CSS-Pop-ups");
            assert!(pr.left() > cr.right() - 1.0 && (pr.top() - cr.top()).abs() < 1.0, "{lang}: zweite Spalte rechts neben den Kommentaren");
            assert_eq!(ext.accesskit_node().is_disabled(), !popups, "{lang}: grau, solange CSS-Pop-ups aus sind");
        }
    }
    fp_i18n::set_language("de");
}

#[test]
fn noise_menu_dialog_starts_at_50_after_restart() {
    let mut p = fp_engine::Preferences::new();
    p.set("AddNoiseAmount", fp_engine::PrefValue::Int(20));
    let mut menu = fp_ui::dialogs::catalog::filter_form("filter.noise.addNoise", false).unwrap();
    menu.load_prefs(&p);
    assert_eq!(menu.get("amount").as_f64(), Some(20.0), "in der Sitzung gemerkt");
    let restarted = fp_engine::Preferences::from_json(&p.to_json()).unwrap();
    let mut menu = fp_ui::dialogs::catalog::filter_form("filter.noise.addNoise", false).unwrap();
    menu.load_prefs(&restarted);
    assert_eq!(menu.get("amount").as_f64(), Some(50.0), "nach Neustart die Vorgabe 50");
}

#[test]
fn spelling_setup_prefills_personal_dictionary() {
    let mut st = plain();
    let d = fp_ui::dialogs::spelling_setup::SpellingSetupDialog::new(&mut st).expect("Dialog");
    if let Some(want) = fp_engine::commands::spelling::default_personal_dictionary() {
        assert_eq!(d.personal_path, want.to_string_lossy());
        assert!(d.personal_path.ends_with("Personal.dic"), "{}", d.personal_path);
    }
}

#[test]
fn export_without_slices_defaults_to_images_only_and_remembers() {
    let mut st = plain();
    let d = ExportDialog::new(&st).unwrap();
    assert_eq!(d.kind(), FilesStyle::ImagesOnly, "ohne Segmente: Nur Bilder / HTML Ohne");
    // letzte Wahl merken
    let mut d2 = d.clone();
    d2.set_style(style_index(FilesStyle::HtmlAndImages));
    export::remember(&mut st, &d2);
    assert_eq!(ExportDialog::new(&st).unwrap().kind(), FilesStyle::HtmlAndImages);
    d2.set_style(style_index(FilesStyle::ImagesOnly));
    export::remember(&mut st, &d2);
    assert_eq!(ExportDialog::new(&st).unwrap().kind(), FilesStyle::ImagesOnly);
    // mit Segment und ohne bisherige Wahl: HTML und Bilder
    let mut st = plain();
    st.session.execute("edit.insert.slice", json!({})).expect("slice");
    assert_eq!(ExportDialog::new(&st).unwrap().kind(), FilesStyle::HtmlAndImages);
}

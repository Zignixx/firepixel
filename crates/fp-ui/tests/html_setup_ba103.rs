//! BA-103: HTML setup "Set Defaults" (used by New Document and export), page styles in an
//! external `styles.css` and the neutral "Generic" HTML5 style.

use egui_kittest::kittest::Queryable;
use fp_engine::Session;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::html_setup::{self, HtmlSetupDialog};
use serde_json::json;

fn plain() -> AppState {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 200, "height": 120})).expect("file.new");
    s.execute("draw.rectangle", json!({"rect": [10, 10, 100, 100], "fillColor": "#ff0000"})).expect("rect");
    s.execute("edit.insert.slice", json!({})).expect("slice");
    s.execute("pi.behavior.add", json!({"action": 8})).expect("rollover");
    s.execute("state.add", json!({"count": 1})).expect("state");
    s.execute("state.setCurrent", json!({"index": 0})).expect("state 0");
    AppState::new(s, UiSettings::default(), None)
}

fn page_html(st: &AppState) -> Option<fp_model::HtmlSettings> {
    let d = st.session.active()?;
    d.doc.pages[d.page].root.export.as_ref().and_then(|e| e.html.as_deref()).cloned()
}

#[test]
fn set_defaults_reaches_new_document_and_export() {
    fp_i18n::set_language("en");
    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1000.0, 700.0))
        .build_ui_state(|ui, st: &mut AppState| html_setup::show(ui.ctx(), st), plain());
    let mut d = HtmlSetupDialog::new(harness.state()).expect("dialog");
    assert_eq!(d.style, 1, "XHTML by default");
    assert!(!d.page_css);
    d.style = 2;
    d.page_css = true;
    d.alt = "Picture".into();
    harness.state_mut().dialogs.html_setup = Some(d);
    harness.run_steps(3);
    harness.get_by_label(&fp_i18n::t("dlg-hs-set-defaults")).click();
    harness.run_steps(3);
    let msg = harness.state().dialogs.message.clone();
    assert_eq!(msg.map(|m| m.1), Some(fp_i18n::t("dlg-hs-defaults-set")));
    // the dialog stays open; Cancel leaves the current document untouched
    assert!(harness.state().dialogs.html_setup.is_some());
    harness.key_press(egui::Key::Escape);
    harness.run_steps(3);
    let st = harness.state_mut();
    st.dialogs.html_setup = None;
    st.dialogs.message = None;
    assert_ne!(page_html(st).and_then(|h| h.xhtml), Some(2), "current document unchanged");

    // New Document takes the defaults
    st.session.execute("file.new", json!({"width": 120, "height": 80})).expect("file.new");
    let h = page_html(st).expect("new document has HTML settings");
    assert_eq!(h.xhtml, Some(2));
    assert_eq!(h.external_css_for_doc, Some(true));
    assert_eq!(h.default_alt.as_deref(), Some("Picture"));
    let d2 = HtmlSetupDialog::new(st).expect("dialog");
    assert_eq!(d2.style, 2);
    assert!(d2.page_css);

    // export writes HTML5 + styles.css next to the HTML
    st.session.execute("draw.rectangle", json!({"rect": [10, 10, 60, 60], "fillColor": "#00ff00"})).expect("rect");
    st.session.execute("edit.insert.slice", json!({})).expect("slice");
    let dir = std::env::temp_dir().join(format!("fp-ba103-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    st.session
        .execute("file.export", json!({"path": dir.display().to_string(), "style": "htmlAndImages", "baseName": "page"}))
        .expect("export");
    let html = std::fs::read_to_string(dir.join("page.htm")).expect("page.htm");
    assert!(html.starts_with("<!DOCTYPE html>"), "{html}");
    assert!(html.contains("href=\"styles.css\""), "{html}");
    assert!(html.contains("alt=\"Picture\""), "{html}");
    let css = std::fs::read_to_string(dir.join("styles.css")).expect("styles.css");
    assert!(css.contains("td img"), "{css}");
    let _ = std::fs::remove_dir_all(&dir);

    // reset: built-in defaults again
    st.session.execute("file.htmlSetupDefaults", json!({"reset": true})).expect("reset");
    st.session.execute("file.new", json!({"width": 50, "height": 50})).expect("file.new");
    assert_ne!(page_html(st).and_then(|h| h.xhtml), Some(2));
}

#[test]
fn generic_style_and_page_css_round_trip_through_ok() {
    let mut st = plain();
    let mut d = HtmlSetupDialog::new(&st).expect("dialog");
    d.style = 2;
    d.page_css = true;
    html_setup::apply(&mut st, &d);
    let h = page_html(&st).expect("settings");
    assert_eq!(h.xhtml, Some(2));
    assert_eq!(h.external_css_for_doc, Some(true));
    let d2 = HtmlSetupDialog::new(&st).unwrap();
    assert!(d2.unchanged() && d2.style == 2 && d2.page_css);
}

#[test]
fn dialog_draws_new_controls_in_both_languages() {
    for lang in ["de", "en"] {
        fp_i18n::set_language(lang);
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1000.0, 700.0))
            .build_ui_state(|ui, st: &mut AppState| html_setup::show(ui.ctx(), st), plain());
        let d = HtmlSetupDialog::new(harness.state()).unwrap();
        harness.state_mut().dialogs.html_setup = Some(d);
        harness.run_steps(3);
        harness.get_by_label(&fp_i18n::t("dlg-hs-page-css"));
        harness.get_by_label(&fp_i18n::t("dlg-hs-set-defaults"));
    }
}

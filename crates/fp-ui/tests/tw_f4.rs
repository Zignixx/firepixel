//! F4 (docs\parity\TWEAKS_2026-10-14.md): TW-21 Rückfrage „Dieser Vorgang konvertiert Vektoren in
//! Bitmaps.“ vor einem Filter auf Vektorobjekte (OK/Abbrechen/Nicht mehr anzeigen, zurücksetzbar),
//! TW-24 „Neu“ schlägt die Größe der internen Zwischenablage vor.

use fp_ui::app_state::{Action, AppState, UiSettings};
use serde_json::{json, Value};

fn state() -> (AppState, egui::Context) {
    fp_engine::shell::disable();
    fp_i18n::profile::use_test_profile();
    let session = fp_ui::ui_commands::new_session(fp_engine::Preferences::new());
    let mut st = AppState::new(session, UiSettings::default(), None);
    st.sync_render = true;
    st.os_clipboard = false;
    st.session.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("Dokument");
    (st, egui::Context::default())
}

fn run(st: &mut AppState, ctx: &egui::Context, name: &str, p: Value) {
    st.push(Action::Run(name.into(), p));
    st.process_actions(ctx);
}

fn selected_kind_is_bitmap(st: &AppState) -> bool {
    let d = st.session.active().expect("Dokument");
    let id = d.selection.elements[0];
    fp_engine::docops::element_on_page(&d.doc, d.page, id).is_some_and(|e| matches!(e.kind, fp_model::ElementKind::Bitmap(_)))
}

#[test]
fn filter_on_vector_asks_first() {
    fp_i18n::with_language("de", || {
        let (mut st, ctx) = state();
        st.session.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80]})).expect("Rechteck");
        let steps = st.session.active().map(|d| d.history.len()).unwrap_or(0);
        run(&mut st, &ctx, "filter.blur.blur", json!({}));
        let w = st.dialogs.warning.clone().expect("Rückfrage offen");
        assert_eq!(w.warning.key, fp_ui::dialogs::warning::FILTER_VECTORS);
        assert_eq!(w.warning.text, "Dieser Vorgang konvertiert Vektoren in Bitmaps.");
        assert_eq!(w.warning.choices.len(), 1, "OK, dazu Abbrechen");
        assert!(w.warning.pref.is_some(), "mit „Nicht mehr anzeigen“");
        assert_eq!(st.session.active().map(|d| d.history.len()).unwrap_or(0), steps, "noch nichts geändert");
        assert!(!selected_kind_is_bitmap(&st));

        // OK (wie der Knopf: In Bitmap konvertieren, dann der Filter ohne erneute Frage)
        st.dialogs.warning = None;
        st.dialogs.filter_vectors_ok = true;
        st.push(Action::Run("modify.flattenSelection".into(), json!({})));
        run(&mut st, &ctx, "filter.blur.blur", json!({}));
        assert!(st.dialogs.warning.is_none());
        assert!(selected_kind_is_bitmap(&st), "in Bitmap konvertiert");
        // Bitmap: keine Rückfrage
        run(&mut st, &ctx, "filter.blur.blur", json!({}));
        assert!(st.dialogs.warning.is_none());

        // „Nicht mehr anzeigen“ und „Warndialoge zurücksetzen“
        st.session.execute("draw.rectangle", json!({"rect": [150, 20, 250, 80]})).expect("Rechteck");
        st.session.set_warning_disabled(&w.warning, true);
        run(&mut st, &ctx, "filter.adjustColor.invert", json!({}));
        st.process_actions(&ctx);
        assert!(st.dialogs.warning.is_none(), "abgeschaltet");
        assert!(selected_kind_is_bitmap(&st), "abgeschaltet: trotzdem konvertiert");
        st.session.execute("draw.rectangle", json!({"rect": [20, 100, 120, 180]})).expect("Rechteck");
        run(&mut st, &ctx, "ui.resetWarnings", json!({}));
        run(&mut st, &ctx, "filter.adjustColor.invert", json!({}));
        assert!(st.dialogs.warning.is_some(), "nach dem Zurücksetzen wieder da");
    });
}

#[test]
fn new_document_uses_clipboard_size() {
    let (mut st, ctx) = state();
    run(&mut st, &ctx, "file.new", json!({}));
    let d = st.dialogs.new_document.take().expect("Dialog");
    let (w0, h0) = (d.width.clone(), d.height.clone());
    st.session.execute("draw.rectangle", json!({"rect": [50, 50, 150, 120]})).expect("Rechteck");
    st.session.execute("draw.rectangle", json!({"rect": [60, 60, 70, 70]})).expect("Rechteck");
    st.session.execute("select.all", json!({})).expect("alles");
    st.session.execute("edit.copy", json!({})).expect("kopieren");
    run(&mut st, &ctx, "file.new", json!({}));
    let d = st.dialogs.new_document.take().expect("Dialog");
    assert_eq!((d.width.as_str(), d.height.as_str()), ("100", "70"), "vorher {w0} × {h0}");
}

//! RT-2 (re\27 E3, BA-036): Die HTML-Exportart gehört zum Dokument (`exportFilesStyle`) und wird
//! nicht als Voreinstellung für neue Dokumente gemerkt; ein neues Dokument startet mit der
//! Werksvorgabe „HTML und Bilder“ (mit Segment; ohne Segment „Nur Bilder“, Q-07).

use fp_engine::Session;
use fp_export::doc_export::FilesStyle;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::export::{self, style_index, ExportDialog};
use serde_json::json;

fn new_doc_with_slice(s: &mut Session) {
    s.execute("file.new", json!({"width": 200, "height": 120})).expect("file.new");
    s.execute("draw.rectangle", json!({"rect": [10, 10, 100, 100], "fillColor": "#ff0000"})).expect("rect");
    s.execute("edit.insert.slice", json!({})).expect("slice");
}

fn stored_style(st: &AppState) -> Option<i32> {
    let d = st.session.active().unwrap();
    d.doc.pages[d.page].root.export.as_ref().and_then(|e| e.html.as_deref()).and_then(|h| h.export_files_style)
}

#[test]
fn export_style_is_per_document_not_remembered_for_new_documents() {
    let mut s = Session::default();
    new_doc_with_slice(&mut s);
    let mut st = AppState::new(s, UiSettings::default(), None);
    let a = st.session.active().unwrap().id;
    let mut d = ExportDialog::new(&st).unwrap();
    assert_eq!(d.kind(), FilesStyle::HtmlAndImages, "Werksvorgabe mit Segment");
    d.set_style(style_index(FilesStyle::ImagesOnly));
    export::remember(&mut st, &d);
    assert_eq!(ExportDialog::new(&st).unwrap().kind(), FilesStyle::ImagesOnly, "im Dokument A gemerkt");
    let style_a = stored_style(&st);
    // Datei ▸ Neu: keine Übernahme der zuletzt gewählten Art
    new_doc_with_slice(&mut st.session);
    assert_ne!(st.session.active().unwrap().id, a);
    assert_eq!(ExportDialog::new(&st).unwrap().kind(), FilesStyle::HtmlAndImages, "neues Dokument: Werksvorgabe");
    assert_ne!(stored_style(&st), style_a, "Art nicht ins neue Dokument übernommen");
    assert!(!st.session.prefs.to_json().contains("FilesStyle"), "keine Voreinstellung für die Exportart");
    // zurück zu A: dessen Wahl bleibt
    st.session.activate(a).expect("A");
    assert_eq!(ExportDialog::new(&st).unwrap().kind(), FilesStyle::ImagesOnly);
}

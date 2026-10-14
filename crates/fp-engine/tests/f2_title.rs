//! F2 (TW-30): Ein neues Dokument heißt wie im Original „Unbenannt-1.fw.png“ (Endung nach „.fw.png anhängen“,
//! ab Werk an); der Zähler läuft weiter, geöffnete Dateien behalten ihren Dateinamen.

use serde_json::json;

use crate::common::*;

#[test]
fn new_document_title_has_fw_png_extension() {
    use fp_engine::{Preferences, Session};
    fp_i18n::set_language("de");
    let mut s = Session::new(Preferences::new());
    ex(&mut s, "file.new", json!({"width": 200, "height": 100}));
    assert_eq!(s.active().map(|d| d.title.clone()).as_deref(), Some("Unbenannt-1.fw.png"));
    ex(&mut s, "file.new", json!({"width": 200, "height": 100}));
    assert_eq!(s.active().map(|d| d.title.clone()).as_deref(), Some("Unbenannt-2.fw.png"));
    fp_i18n::set_language("en");
    ex(&mut s, "file.new", json!({"width": 200, "height": 100}));
    assert_eq!(s.active().map(|d| d.title.clone()).as_deref(), Some("Untitled-3.fw.png"));
    fp_i18n::set_language("de");
    // mit Pfad: Dateiname bleibt
    let doc = s.active().map(|d| (*d.doc).clone()).expect("Dokument");
    let id = s.add_document(doc, Some(std::path::PathBuf::from("C:\\x\\bild.png")));
    let _ = s.activate(id);
    assert_eq!(s.active().map(|d| d.title.clone()).as_deref(), Some("bild.png"));
}

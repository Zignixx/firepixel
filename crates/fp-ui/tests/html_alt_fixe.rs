//! FIX-E (Fix-Liste 1, CT-3): Der „Standard-ALT-Tag“ der HTML-Einrichtung (Dokumentspezifisch)
//! überlebt Speichern als FW-PNG, Schließen und Öffnen (eigener Tag `DALs` in der Erweiterung
//! `fpSv`, weil der Tag des Originals unbekannt ist).

use fp_ui::app_state::{AppState, UiSettings};
use serde_json::json;
use std::path::PathBuf;

fn state() -> AppState {
    fp_engine::shell::disable();
    let session = fp_ui::ui_commands::new_session(fp_engine::Preferences::new());
    let mut st = AppState::new(session, UiSettings::default(), None);
    st.sync_render = true;
    st.os_clipboard = false;
    st
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fp-fixe-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn alt_of(st: &AppState) -> Option<String> {
    let d = st.session.active()?;
    d.doc.pages.first()?.root.export.as_ref()?.html.as_ref()?.default_alt.clone()
}

#[test]
fn default_alt_survives_save_close_open() {
    let mut st = state();
    let dir = tmp("alt");
    st.session.execute("file.new", json!({"width": 40, "height": 30})).unwrap();
    // wie der Dialog: nur geänderte Felder; mehrfach geändert (§4b) → der letzte Wert zählt
    for i in 0..20 {
        fp_ui::ui_commands::execute(&mut st.session, "file.htmlSetup", json!({"settings": {"default_alt": format!("Entwurf {i}")}})).unwrap();
    }
    fp_ui::ui_commands::execute(&mut st.session, "file.htmlSetup", json!({"settings": {"default_alt": "Logo „Firepixel“ – ä"}})).unwrap();
    assert_eq!(alt_of(&st).as_deref(), Some("Logo „Firepixel“ – ä"));
    let p = dir.join("alt.fw.png");
    st.session.execute("file.saveAs", json!({"path": p.to_string_lossy()})).unwrap();
    st.session.execute("file.close", json!({})).unwrap();
    assert!(st.session.documents().is_empty());
    st.session.execute("file.open", json!({"path": p.to_string_lossy()})).unwrap();
    assert_eq!(alt_of(&st).as_deref(), Some("Logo „Firepixel“ – ä"), "nach Öffnen");
    // nicht doppelt als unbekannter Tag, zweites Speichern behält ihn
    let doc = fp_fwpng::read(&std::fs::read(&p).unwrap()).unwrap();
    assert!(!doc.pages[0].root.export.as_ref().unwrap().extra.has_raw(fp_model::TagKey::new(b"fpSv")));
    let p2 = dir.join("wieder.fw.png");
    st.session.execute("file.saveAs", json!({"path": p2.to_string_lossy()})).unwrap();
    let again = fp_fwpng::read(&std::fs::read(&p2).unwrap()).unwrap();
    assert_eq!(again.pages[0].root.export.as_ref().and_then(|e| e.html.as_ref()).and_then(|h| h.default_alt.clone()).as_deref(), Some("Logo „Firepixel“ – ä"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn without_default_alt_no_extension_is_written() {
    // ohne Sprite-Felder und ohne ALT-Text bleibt die Datei frei von der eigenen Erweiterung
    let h = fp_model::HtmlSettings::cs6_default();
    assert!(h.default_alt.is_none());
    let mut e = fp_model::ExportSettings::default();
    e.html = Some(Box::new(h));
    let nodes = fp_fwpng::write::write_export(&e);
    assert!(!nodes.iter().any(|n| n.key == fp_fwpng::write::SPRITE_EXT));
}

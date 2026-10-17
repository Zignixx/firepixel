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
    let e = fp_model::ExportSettings {
        html: Some(Box::new(h)),
        ..Default::default()
    };
    let nodes = fp_fwpng::write::write_export(&e);
    assert!(!nodes.iter().any(|n| n.key == fp_fwpng::write::SPRITE_EXT));
}

/// CT-3 (niedrig): *Datei ▸ HTML aktualisieren* mit einer Textdatei meldete den Fehler nur in der
/// Statuszeile mit dem Vorspann „ungültige Parameter für „file.updateHtml“: …“. Jetzt: Meldung im
/// Dialog, ohne technischen Vorspann; die Datei bleibt unverändert. Wiederholt (§4b): eine Meldung.
#[test]
fn update_html_with_a_text_file_shows_a_plain_message() {
    let mut st = state();
    let dir = tmp("upd");
    st.session.execute("file.new", json!({"width": 40, "height": 30})).unwrap();
    let txt = dir.join("notiz.txt");
    std::fs::write(&txt, "nur Text").unwrap();
    st.file_override = Some(fp_ui::files::FilePickOverride { open: vec![txt.clone()], save: None });
    let ctx = egui::Context::default();
    for _ in 0..3 {
        st.dialogs.message = None;
        st.run("file.updateHtml", json!({}));
        st.process_actions(&ctx);
        let (title, msg) = st.dialogs.message.clone().expect("Meldung im Dialog");
        assert_eq!(title, "HTML aktualisieren");
        assert!(msg.contains("keine HTML-Seite"), "{msg}");
        assert!(!msg.contains("Parameter") && !msg.contains("file.updateHtml"), "ohne technischen Vorspann: {msg}");
        assert!(!st.status.as_ref().is_some_and(|(m, _)| m.contains("file.updateHtml")), "nicht in der Statuszeile");
    }
    assert_eq!(std::fs::read_to_string(&txt).unwrap(), "nur Text");
    // Englisch
    fp_i18n::set_language("en");
    st.dialogs.message = None;
    st.run("file.updateHtml", json!({}));
    st.process_actions(&ctx);
    let (title, msg) = st.dialogs.message.clone().expect("message");
    fp_i18n::set_language("de");
    assert_eq!(title, "Update HTML");
    assert!(msg.contains("is not an HTML page") && !msg.contains("invalid parameters"), "{msg}");
    let _ = std::fs::remove_dir_all(&dir);
}

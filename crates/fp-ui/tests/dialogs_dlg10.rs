//! DLG-10: Datei ▸ Datei-Info (D10, Metadaten-Fenster statt Meldung, XMP im Dokument, Verlauf) und
//! Text ▸ Rechtschreibprüfung einrichten (D24, Wörterbuchliste, persönliches Wörterbuch, 4 Optionen).

use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use fp_engine::Session;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::spelling_setup::SpellingSetupDialog;
use fp_ui::FirepixelApp;
use serde_json::{json, Value};
use std::path::PathBuf;

type H = Harness<'static, FirepixelApp>;

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fp-dlg10-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("Ordner");
    d
}

fn session() -> Session {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [10, 10, 60, 60]})).expect("R1");
    s
}

fn harness(s: Session) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn meta(h: &mut H) -> Value {
    h.state_mut().state.session.execute("file.fileInfo", json!({})).expect("info").value["metadata"].clone()
}

fn history_len(h: &H) -> usize {
    st(h).session.active().expect("doc").history.undo_count()
}

fn click_button(h: &mut H, label: &str) {
    h.get_by_label(label).click();
    h.run_steps(4);
}

#[test]
fn file_info_is_a_window_with_metadata_fields() {
    let mut h = harness(session());
    h.state_mut().state.run("file.fileInfo", json!({}));
    h.run_steps(3);
    assert!(st(&h).dialogs.message.is_none(), "keine Meldung mehr");
    assert!(st(&h).dialogs.file_info.is_some(), "Datei-Info-Fenster offen");
    // fünf Eingabefelder (Titel, Autor, Beschreibung, Schlüsselwörter, Copyright)
    let inputs = h.query_all_by_role(egui::accesskit::Role::TextInput).count() + h.query_all_by_role(egui::accesskit::Role::MultilineTextInput).count();
    assert!(inputs >= 5, "{inputs} Eingabefelder");
    // Startfokus im ersten Feld: Tippen ersetzt den (leeren) Titel
    h.event(egui::Event::Text("Plakat".into()));
    h.run_steps(2);
    assert_eq!(st(&h).dialogs.file_info.as_ref().expect("offen").title, "Plakat");
    // Esc bricht ab, nichts geändert
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    assert!(st(&h).dialogs.file_info.is_none());
    assert_eq!(meta(&mut h)["title"], "");
}

#[test]
fn file_info_ok_stores_xmp_in_document_as_one_undo_step() {
    let mut h = harness(session());
    let before = history_len(&h);
    h.state_mut().state.run("file.fileInfo", json!({}));
    h.run_steps(3);
    {
        let d = h.state_mut().state.dialogs.file_info.as_mut().expect("offen");
        d.title = "Plakat".into();
        d.author = "Anna".into();
        d.description = "Zwei\nZeilen".into();
        d.keywords = "rot, blau; grün".into();
        d.copyright = "© 2026 Anna".into();
    }
    click_button(&mut h, "OK");
    assert!(st(&h).dialogs.file_info.is_none());
    assert_eq!(history_len(&h), before + 1, "genau ein Verlaufsschritt");
    let m = meta(&mut h);
    assert_eq!(m["title"], "Plakat");
    assert_eq!(m["author"], "Anna");
    assert_eq!(m["description"], "Zwei\nZeilen");
    assert_eq!(m["keywords"], json!(["rot", "blau", "grün"]));
    assert_eq!(m["copyright"], "© 2026 Anna");
    let chunk = st(&h).session.active().expect("doc").doc.file_meta.xmp_chunk.clone().expect("XMP-Chunk");
    assert_eq!(&chunk.kind, b"iTXt");
    let text = String::from_utf8_lossy(&chunk.data).to_string();
    assert!(text.contains("<?xpacket ") && text.contains("<dc:title>"), "{text}");
    // erneut öffnen: Felder zeigen die gespeicherten Werte; OK ohne Änderung = kein Schritt
    h.state_mut().state.run("file.fileInfo", json!({}));
    h.run_steps(3);
    let d = st(&h).dialogs.file_info.clone().expect("offen");
    assert_eq!((d.title.as_str(), d.keywords.as_str()), ("Plakat", "rot, blau, grün"));
    click_button(&mut h, "OK");
    assert_eq!(history_len(&h), before + 1, "OK ohne Änderung legt keinen Schritt an");
    // Rückgängig / Wiederholen
    h.state_mut().state.session.execute("edit.undo", json!({})).expect("undo");
    assert_eq!(meta(&mut h)["title"], "");
    assert!(st(&h).session.active().expect("doc").doc.file_meta.xmp_chunk.is_none());
    h.state_mut().state.session.execute("edit.redo", json!({})).expect("redo");
    assert_eq!(meta(&mut h)["title"], "Plakat");
}

#[test]
fn file_info_repeated_changes_do_not_duplicate_and_survive_save() {
    let mut s = session();
    for i in 0..30 {
        s.execute("file.fileInfo", json!({"title": format!("T{i}"), "keywords": ["a", "b"]})).expect("set");
    }
    let x = s.active().expect("doc").doc.file_meta.xmp_chunk.clone().expect("XMP");
    let text = String::from_utf8_lossy(&x.data).to_string();
    assert_eq!(text.matches("<dc:title>").count(), 1, "{text}");
    assert_eq!(text.matches("<dc:subject>").count(), 1);
    let dir = tmp("save");
    let p = dir.join("meta.fw.png");
    s.execute("file.saveAs", json!({"path": p.to_string_lossy()})).expect("speichern");
    let mut s2 = Session::default();
    s2.execute("file.open", json!({"path": p.to_string_lossy()})).expect("öffnen");
    let m = s2.execute("file.fileInfo", json!({})).expect("info").value["metadata"].clone();
    assert_eq!(m["title"], "T29");
    assert_eq!(m["keywords"], json!(["a", "b"]));
}

fn dict_dir(name: &str) -> PathBuf {
    let dir = tmp(name);
    std::fs::write(dir.join("xx_TEST.aff"), "SET UTF-8\n").expect("aff");
    std::fs::write(dir.join("xx_TEST.dic"), "3\nHallo\nder\nBaum\n").expect("dic");
    std::fs::write(dir.join("yy_TEST.aff"), "SET UTF-8\n").expect("aff");
    std::fs::write(dir.join("yy_TEST.dic"), "1\nWald\n").expect("dic");
    dir
}

fn spell_session(dir: &std::path::Path) -> Session {
    let mut s = session();
    s.execute("text.spellingSetup", json!({"languages": ["xx_TEST"], "dictionaryDir": dir.to_string_lossy()})).expect("setup");
    s
}

#[test]
fn spelling_setup_dialog_has_list_path_button_and_four_options() {
    let dir = dict_dir("d1");
    let mut h = harness(spell_session(&dir));
    h.state_mut().state.run("text.spellingSetup", json!({}));
    h.run_steps(3);
    let d = st(&h).dialogs.spelling_setup.clone().expect("Dialog offen");
    assert!(st(&h).dialogs.form.is_none(), "kein Formular mehr");
    assert_eq!(d.dictionaries, vec![("xx_TEST".to_string(), true), ("yy_TEST".to_string(), false)]);
    assert!(d.find_duplicates && d.ignore_numbers && d.ignore_urls && d.ignore_uppercase, "Vorgaben wie im Original: alle vier an");
    for l in ["Persönliches Wörterbuch bearbeiten", "Doppelte Wörter finden", "Wörter mit Zahlen ignorieren", "Internet- und Dateiadressen ignorieren", "Wörter in GROSSBUCHSTABEN ignorieren", "xx_TEST", "yy_TEST"] {
        assert!(h.query_by_label(l).is_some(), "{l}");
    }
    // Kästchen bedienen: zweites Wörterbuch an, GROSSBUCHSTABEN aus
    click_button(&mut h, "yy_TEST");
    click_button(&mut h, "Wörter in GROSSBUCHSTABEN ignorieren");
    let d = st(&h).dialogs.spelling_setup.clone().expect("offen");
    assert_eq!(d.dictionaries[1], ("yy_TEST".to_string(), true));
    assert!(!d.ignore_uppercase);
    // Unterdialog „Persönliches Wörterbuch bearbeiten“
    click_button(&mut h, "Persönliches Wörterbuch bearbeiten");
    assert!(st(&h).dialogs.spelling_setup.as_ref().expect("offen").editing.is_some(), "Unterdialog offen");
    h.state_mut().state.dialogs.spelling_setup.as_mut().expect("offen").editing = Some("Firepixel\nWlet\n\nFirepixel".into());
    h.run_steps(2);
    let oks = h.query_all_by_label("OK").count();
    assert_eq!(oks, 2, "OK im Haupt- und Unterdialog");
    h.query_all_by_label("OK").last().expect("OK").click();
    h.run_steps(4);
    let d = st(&h).dialogs.spelling_setup.clone().expect("offen");
    assert!(d.editing.is_none());
    assert_eq!(d.words, vec!["Firepixel".to_string(), "Wlet".to_string()]);
    // OK übernimmt alles in die Voreinstellungen
    click_button(&mut h, "OK");
    assert!(st(&h).dialogs.spelling_setup.is_none());
    let v = h.state_mut().state.session.execute("text.spellingSetup", json!({})).expect("setup").value;
    assert_eq!(v["languages"], json!(["xx_TEST", "yy_TEST"]));
    assert_eq!(v["ignoreUppercase"], false);
    assert_eq!(v["findDuplicates"], true);
    assert_eq!(v["userWords"], json!(["Firepixel", "Wlet"]));
}

#[test]
fn spelling_setup_cancel_changes_nothing() {
    let dir = dict_dir("d2");
    let mut h = harness(spell_session(&dir));
    h.state_mut().state.run("text.spellingSetup", json!({}));
    h.run_steps(3);
    click_button(&mut h, "Doppelte Wörter finden");
    click_button(&mut h, "Abbrechen");
    assert!(st(&h).dialogs.spelling_setup.is_none());
    let v = h.state_mut().state.session.execute("text.spellingSetup", json!({})).expect("setup").value;
    assert_eq!(v["findDuplicates"], true);
}

fn misspelled(s: &mut Session) -> Vec<(String, bool)> {
    let v = s.execute("text.checkSpelling", json!({})).expect("check").value;
    v["misspelled"].as_array().expect("Liste").iter().map(|x| (x["word"].as_str().unwrap_or("").to_string(), x["duplicate"].as_bool().unwrap_or(false))).collect()
}

#[test]
fn spelling_options_take_effect() {
    let dir = dict_dir("d3");
    let mut s = spell_session(&dir);
    s.execute("draw.text", json!({"text": "der der Baum WLET www.baum.de Baum2 Wald", "anchor": [10, 150]})).expect("Text");
    // Vorgaben: doppeltes „der“ gemeldet, GROSS/Adresse/Ziffern übersprungen, „Wald“ fehlt in xx_TEST
    assert_eq!(misspelled(&mut s), vec![("der".to_string(), true), ("Wald".to_string(), false)]);
    // zweites Wörterbuch aktiv: „Wald“ ist richtig
    s.execute("text.spellingSetup", json!({"languages": ["xx_TEST", "yy_TEST"]})).expect("setup");
    assert_eq!(misspelled(&mut s), vec![("der".to_string(), true)]);
    // alle Optionen aus
    s.execute("text.spellingSetup", json!({"findDuplicates": false, "ignoreUppercase": false, "ignoreUrls": false, "ignoreNumbers": false})).expect("setup");
    let w: Vec<String> = misspelled(&mut s).into_iter().map(|x| x.0).collect();
    assert_eq!(w, vec!["WLET", "www", "baum", "de", "Baum2"]);
    // doppeltes Wort zusammenfassen (Löschen im Prüfdialog)
    s.execute("text.spellingSetup", json!({"findDuplicates": true, "ignoreUppercase": true, "ignoreUrls": true, "ignoreNumbers": true})).expect("setup");
    let v = s.execute("text.checkSpelling", json!({"replace": [{"word": "der", "with": "", "duplicate": true}]})).expect("ersetzen").value;
    assert_eq!(v["replaced"], 1);
    assert!(misspelled(&mut s).is_empty());
    s.execute("edit.undo", json!({})).expect("undo");
    assert_eq!(misspelled(&mut s), vec![("der".to_string(), true)]);
}

#[test]
fn personal_dictionary_file_is_used() {
    let dir = dict_dir("d4");
    let mut s = spell_session(&dir);
    s.execute("text.spellingSetup", json!({"addWords": ["Wlet"]})).expect("Wort");
    let file = dir.join("persoenlich").join("Personal.dic");
    // Pfad setzen: vorhandene Wörter ziehen in die Datei um
    s.execute("text.spellingSetup", json!({"personalDictionary": file.to_string_lossy()})).expect("Pfad");
    assert_eq!(std::fs::read_to_string(&file).expect("Datei").trim(), "Wlet");
    s.execute("text.spellingSetup", json!({"addWords": ["Firepixel"]})).expect("Wort");
    assert_eq!(std::fs::read_to_string(&file).expect("Datei"), "Wlet\nFirepixel\n");
    s.execute("draw.text", json!({"text": "Hallo Wlet Firepixel Xyz", "anchor": [10, 150]})).expect("Text");
    assert_eq!(misspelled(&mut s), vec![("Xyz".to_string(), false)]);
    // Dialog liest Pfad und Wörter
    let v = s.execute("text.spellingSetup", json!({})).expect("setup").value;
    let d = SpellingSetupDialog::from_value(&v);
    assert_eq!(d.personal_path, file.to_string_lossy());
    assert_eq!(d.words, vec!["Wlet".to_string(), "Firepixel".to_string()]);
}

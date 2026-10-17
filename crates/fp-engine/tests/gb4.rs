//! G-B4 (Paket B4 aus docs\berichte\b2b.md), Engine-Teil:
//! - BA-127 Vorgabe-Kürzelsatz „Web Standard“ (auch als Rückfall)
//! - BA-027 Optimieren-Menü bei GIF: Sperrung/Bearbeitungen nur mit gesperrten bzw. bearbeiteten Farben
//! - BA-023 Datei-Info: weitere Felder der Seite „Beschreibung“ über `file.fileInfo`

use fp_engine::prefs::{PrefValue, Preferences, DEFAULT_SHORTCUT_SET};
use fp_engine::Session;
use serde_json::json;

#[test]
fn ba127_default_shortcut_set_is_web_standard() {
    assert_eq!(DEFAULT_SHORTCUT_SET, "Web Standard");
    let p = Preferences::default();
    assert_eq!(p.shortcut_set(), "Web Standard");
    let mut s = Session::default();
    assert_eq!(s.shortcut_set().name, "Web Standard");
    // unbekannter gespeicherter Satz: Rückfall auf die Vorgabe
    s.prefs.set("CurrentShortcutSet", PrefValue::Str("gibt es nicht".into()));
    assert_eq!(s.shortcut_set().name, "Web Standard");
}

#[test]
fn ba027_unlock_and_remove_edits_need_locked_or_edited_colors() {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 100, "height": 60})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [10, 10, 50, 50], "fillColor": "#ff0000"})).expect("rect");
    s.execute("pi.export.settings", json!({"settings": serde_json::to_value(fp_model::ExportSettings::default()).expect("json")})).expect("gif");
    assert!(!s.command_state("panel.optimize.unlockAll").0, "ohne gesperrte Farbe grau");
    assert!(!s.command_state("panel.optimize.removeAllEdits").0, "ohne Bearbeitung grau");
    assert_eq!(s.command_state("panel.optimize.removeUnusedColors"), (true, Some(true)), "Vorgabe an");
    assert_eq!(s.command_state("panel.optimize.swatchFeedback").1, Some(true), "Vorgabe an");
    s.execute("panel.optimize.lockColors", json!({"colors": ["#ff0000"]})).expect("sperren");
    assert!(s.command_state("panel.optimize.unlockAll").0, "mit gesperrter Farbe aktiv");
    assert!(!s.command_state("panel.optimize.removeAllEdits").0);
    s.execute("panel.optimize.editColor", json!({"color": "#ff0000", "newColor": "#00ff00"})).expect("bearbeiten");
    assert!(s.command_state("panel.optimize.removeAllEdits").0, "mit bearbeiteter Farbe aktiv");
    s.execute("panel.optimize.unlockAll", json!({})).expect("entsperren");
    assert!(!s.command_state("panel.optimize.unlockAll").0);
}

#[test]
fn ba023_file_info_extra_fields() {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 100, "height": 60})).expect("doc");
    let p = json!({"authorTitle": "Grafik", "captionWriter": "Ben", "rating": 3, "copyrightStatus": "publicDomain", "copyrightUrl": "https://example.org"});
    let m = s.execute("file.fileInfo", p).expect("info").value["metadata"].clone();
    assert_eq!(m["authorTitle"], "Grafik");
    assert_eq!(m["captionWriter"], "Ben");
    assert_eq!(m["rating"], 3);
    assert_eq!(m["copyrightStatus"], "publicDomain");
    assert_eq!(m["copyrightUrl"], "https://example.org");
    assert!(s.execute("file.fileInfo", json!({"rating": 6})).is_err(), "Bewertung höchstens 5");
    s.execute("file.fileInfo", json!({"copyrightStatus": "unknown", "rating": 0})).expect("zurück");
    let m = s.execute("file.fileInfo", json!({})).expect("info").value["metadata"].clone();
    assert_eq!((m["copyrightStatus"].as_str(), m["rating"].as_u64()), (Some("unknown"), Some(0)));
}

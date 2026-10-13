//! `prompt()` in Befehlsskripten (CT-1b MN-5, FIX-B): ohne `prompts` Vorgabewert; mit `prompts`
//! (Lauf aus der Oberfläche) Antworten der Reihe nach, fehlt eine, bricht der Lauf ohne Änderung ab
//! und meldet die Frage.

use fp_engine::Session;
use serde_json::{json, Value};

fn setup() -> Session {
    let mut s = Session::default();
    fp_script::install_host(&mut s);
    s.execute("file.new", json!({"width": 300, "height": 200})).unwrap_or_else(|e| panic!("{e}"));
    s
}

const CODE: &str = "var w = prompt('Breite?', '40'); var h = prompt('Höhe?', 30);\n\
var d = fw.getDocumentDOM();\n\
if (w != null && h != null) d.addNewRectanglePrimitive({left: 0, top: 0, right: Number(w), bottom: Number(h)}, 0);";

fn count(s: &Session) -> usize {
    let v = serde_json::to_value(&*s.active().expect("Dokument").doc).unwrap_or(Value::Null);
    v.to_string().matches("\"common\"").count()
}

fn widths(s: &Session) -> Vec<f64> {
    let d = s.active().expect("Dokument");
    let mut v = Vec::new();
    d.doc.walk_elements(|_, _, e| {
        if let Some(b) = fp_engine::docops::element_bounds(&d.doc, e) {
            v.push((b.right - b.left) as f64);
        }
    });
    v
}

#[test]
fn headless_uses_default_values() {
    let mut s = setup();
    let n = count(&s);
    let out = s.execute("commands.runScript", json!({"code": CODE})).unwrap_or_else(|e| panic!("{e}"));
    assert!(out.value["result"]["promptNeeded"].is_null(), "{:?}", out.value);
    assert_eq!(count(&s), n + 1);
    assert_eq!(widths(&s), vec![40.0]);
}

#[test]
fn ui_run_asks_each_prompt_then_uses_answers() {
    let mut s = setup();
    let n = count(&s);
    let undo = s.active().map(|d| d.history.undo_count()).unwrap_or(0);
    // 1. Lauf ohne Antworten: erste Frage, Dokument unverändert, kein Verlaufsschritt
    let o = s.execute("commands.runScript", json!({"code": CODE, "prompts": []})).unwrap_or_else(|e| panic!("{e}"));
    let q = &o.value["result"]["promptNeeded"];
    assert_eq!(q["text"], "Breite?");
    assert_eq!(q["default"], "40");
    assert_eq!(count(&s), n);
    assert_eq!(s.active().map(|d| d.history.undo_count()).unwrap_or(0), undo);
    // 2. mit einer Antwort: zweite Frage (Vorgabe als Zahl → Text)
    let o = s.execute("commands.runScript", json!({"code": CODE, "prompts": ["120"]})).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(o.value["result"]["promptNeeded"]["text"], "Höhe?");
    assert_eq!(o.value["result"]["promptNeeded"]["default"], "30");
    assert_eq!(count(&s), n);
    // 3. beide Antworten: ein Rechteck mit der eingegebenen Breite, genau ein Verlaufsschritt
    let o = s.execute("commands.runScript", json!({"code": CODE, "prompts": ["120", "50"]})).unwrap_or_else(|e| panic!("{e}"));
    assert!(o.value["result"]["promptNeeded"].is_null());
    assert_eq!(count(&s), n + 1);
    assert_eq!(widths(&s), vec![120.0]);
    assert_eq!(s.active().map(|d| d.history.undo_count()).unwrap_or(0), undo + 1);
    // Abbrechen (null) → prompt liefert null, das Skript tut nichts
    let mut s = setup();
    let o = s.execute("commands.runScript", json!({"code": CODE, "prompts": [null]})).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(o.value["result"]["promptNeeded"]["text"], "Höhe?");
    let _ = s.execute("commands.runScript", json!({"code": CODE, "prompts": [null, "5"]})).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(count(&s), n);
    // danach wieder ohne Oberfläche: Vorgabewerte (kein Zustand bleibt hängen)
    let _ = s.execute("commands.runScript", json!({"code": CODE})).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(widths(&s), vec![40.0]);
}

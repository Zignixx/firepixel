//! F25: Deckkraft 8-bittig (Nutzerentscheidung 17.10., re 29 §7), TW-02 Textläufe nach einer Bearbeitung
//! (Lauf 10 `menu-text/02,03,15,19`) und Enter auf der Leinwand (Lauf 10 `xs-shortcuts/22`).
//! Die Fälle mit `par_text.fw.png` werden übersprungen, wenn die Testdatei fehlt (anderer Rechner).

use fp_engine::{Session, ToolEvent, ToolKey};
use fp_script::{ScriptOptions, run_source};
use serde_json::json;

fn run(s: &mut Session, src: &str) -> String {
    let o = run_source(s, src, &ScriptOptions::default());
    assert!(o.ok, "Fehler: {:?}\nDiagnose: {:?}", o.error, o.diagnostics);
    o.result.unwrap_or_default()
}

const NEW: &str = "var dom = fw.createFirepixelDocument({x: 300, y: 200}, {pixelsPerUnit: 72, units: 'inch'}, '#ffffff'); dom = fw.getDocumentDOM();\n";

fn first_selected(s: &Session) -> fp_model::Element {
    let d = s.active().expect("Dokument");
    let id = d.selection.elements[0];
    let mut out = None;
    d.doc.walk_elements(|_, _, e| {
        if e.id == id && out.is_none() {
            out = Some((**e).clone());
        }
    });
    out.expect("Element")
}

// ------------------------------------------------------------------ Deckkraft 8 bit

#[test]
fn opacity_is_stored_as_byte_and_round_trips_30_77_30() {
    let mut s = Session::default();
    run(
        &mut s,
        &format!(
            "{NEW} dom.addNewRectangle({{left: 10, top: 10, right: 60, bottom: 60}}, 0); dom.setOpacity(30); 0"
        ),
    );
    assert_eq!(first_selected(&s).common.opacity.0, 77, "30 % → 77/255");
    assert_eq!(
        run(&mut s, "fw.selection[0].opacity"),
        "30.19607843137255",
        "Skript liest b·100/255 (re 40 §3)"
    );
    // Inspektor-Weg (element.setOpacity) ebenso
    let id = s.active().unwrap().selection.elements[0];
    s.execute("element.setOpacity", json!({"opacity": 30, "ids": [id.0]}))
        .expect("opacity");
    assert_eq!(first_selected(&s).common.opacity.0, 77);
    // fw.png schreiben und lesen: OPAi = ⌊77·1000/255⌋ = 301 → wieder 77 → 30 %
    let p = std::env::temp_dir().join(format!("fp-f25-{}-op.fw.png", std::process::id()));
    s.execute("file.saveAs", json!({"path": p.to_string_lossy()}))
        .expect("speichern");
    let mut t = Session::default();
    t.execute("file.open", json!({"path": p.to_string_lossy()}))
        .expect("öffnen");
    let _ = std::fs::remove_file(&p);
    let mut op = None;
    t.active().unwrap().doc.walk_elements(|_, _, e| {
        if matches!(e.kind, fp_model::ElementKind::Path(_)) {
            op = Some(e.common.opacity.0);
        }
    });
    assert_eq!(op, Some(77), "fw.png-Round-Trip");
    assert_eq!(fp_model::Opacity(77).to_permille(), 301);
}

#[test]
fn opacity_percent_to_byte_like_original() {
    use fp_model::Opacity;
    for (p, b) in [
        (0.0, 0u8),
        (30.0, 77),
        (50.0, 128),
        (100.0, 255),
        (30.196, 77),
        (-5.0, 0),
        (120.0, 255),
    ] {
        assert_eq!(Opacity::from_percent(p).0, b, "{p} %");
    }
    // ganze Prozent überleben den Weg über das Byte ohne Verlust
    for p in 0..=100 {
        assert_eq!(
            Opacity::from_percent(p as f64).to_percent().round() as i32,
            p
        );
    }
}

#[test]
fn old_permille_opacity_in_session_data_is_converted() {
    let mut s = Session::default();
    run(
        &mut s,
        &format!(
            "{NEW} dom.addNewRectangle({{left: 10, top: 10, right: 60, bottom: 60}}, 0); dom.setOpacity(30); 0"
        ),
    );
    let e = first_selected(&s);
    let mut v = serde_json::to_value(&e).expect("json");
    assert_eq!(
        v["common"]["opacity"],
        json!(77),
        "neu gespeichert als Byte"
    );
    let back: fp_model::Element = serde_json::from_value(v.clone()).expect("lesen");
    assert_eq!(back.common.opacity.0, 77);
    v["common"]["opacity"] = json!(300);
    let old: fp_model::Element = serde_json::from_value(v.clone()).expect("Promille lesen");
    assert_eq!(old.common.opacity.0, 77, "300 ‰ → 77");
    v["common"]["opacity"] = json!(1000);
    let old: fp_model::Element = serde_json::from_value(v).expect("Promille lesen");
    assert_eq!(old.common.opacity.0, 255);
}

// ------------------------------------------------------------------ TW-02 Textläufe

fn par_text() -> Option<Session> {
    let src = std::path::Path::new(
        r"C:\Users\Administrator\Desktop\REA\re\_testdateien\par\par_text.fw.png",
    );
    if !src.is_file() {
        eprintln!("übersprungen: {src:?} fehlt");
        return None;
    }
    let p = std::env::temp_dir().join(format!(
        "fp-f25-{}-{:?}-par_text.fw.png",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::copy(src, &p).expect("Kopie");
    let mut s = Session::default();
    s.execute("file.open", json!({"path": p.to_string_lossy()}))
        .expect("öffnen");
    let _ = std::fs::remove_file(&p);
    Some(s)
}

fn select_named(s: &mut Session, name: &str) {
    run(
        s,
        &format!(
            "fw.getDocumentDOM().selectNone(); var f = null; var l = fw.getDocumentDOM().layers; for (var i = 0; i < l.length; i++) {{ var e = l[i].frames[0].elements; for (var j = 0; j < e.length; j++) if (e[j].name == '{name}') f = e[j]; }} fw.selection = [f]; 0"
        ),
    );
}

const RUNS: &str = "var t = fw.selection[0].textRuns, out = [];\n\
    for (var i = 0; i < t.textRuns.length; i++) { var c = [], a = t.textRuns[i].changedAttrs; for (var n in a) c.push(n); out.push(c.length + ':' + a.leading); }\n\
    t.initialAttrs.leading + '|' + out.join('|')";

#[test]
fn untouched_text_keeps_file_values() {
    let Some(mut s) = par_text() else { return };
    select_named(&mut s, "TX1");
    // Lauf 10 `menu-text/19`: unbearbeitet 1.2 und 15 Eigenschaften im Lauf
    assert_eq!(run(&mut s, RUNS), "1.2|15:1.2");
}

#[test]
fn edited_text_reports_all_attrs_and_32bit_leading() {
    let Some(mut s) = par_text() else { return };
    select_named(&mut s, "TX1");
    for (cmd, want) in [
        ("text.size.36", "1.1999999152289496"),
        ("text.size.24", "1.1999999682108562"),
    ] {
        s.execute(cmd, json!({})).expect(cmd);
        assert_eq!(run(&mut s, RUNS), format!("{want}|18:{want}"), "{cmd}");
    }
    // Lauf 10 `menu-text/15`: 14 pt, mehrere Läufe – jeder Lauf mit allen 18 Eigenschaften
    select_named(&mut s, "TX2");
    s.execute("text.style.fauxBold", json!({})).expect("fett");
    let got = run(&mut s, RUNS);
    assert!(
        got.starts_with("1.1999999455043249|18:1.1999999455043249"),
        "{got}"
    );
    assert!(
        got.split('|').skip(1).all(|r| r == "18:1.1999999455043249"),
        "{got}"
    );
    // Glättungswerte stehen jetzt auch im Lauf
    assert_eq!(
        run(
            &mut s,
            "var a = fw.selection[0].textRuns.textRuns[0].changedAttrs; [a.antiAliasSharpness, a.antiAliasStrength, a.overSample].join(',')"
        ),
        "192,64,8"
    );
}

// ------------------------------------------------------------------ Enter auf der Leinwand

#[test]
fn enter_on_canvas_with_group_selected_clears_selection() {
    let mut s = Session::default();
    run(
        &mut s,
        &format!(
            "{NEW} dom.addNewRectangle({{left: 10, top: 10, right: 60, bottom: 60}}, 0); dom.addNewRectangle({{left: 80, top: 10, right: 140, bottom: 60}}, 0); dom.selectAll(); dom.group(); 0"
        ),
    );
    assert_eq!(
        run(
            &mut s,
            "fw.selection.length ? fw.selection[0].toString() : null"
        ),
        "[object Group]"
    );
    s.tool_event(&ToolEvent::key(ToolKey::Enter))
        .expect("Enter");
    // Lauf 10 `xs-shortcuts/22`: Original `null`
    assert_eq!(
        run(
            &mut s,
            "fw.selection.length ? fw.selection[0].toString() : null"
        ),
        "null"
    );
    // Enter ohne Auswahl tut nichts
    s.tool_event(&ToolEvent::key(ToolKey::Enter))
        .expect("Enter");
    assert_eq!(run(&mut s, "fw.selection.length"), "0");
}

// ------------------------------------------------------------------ M10 (re 43 §4)

/// `setTextRuns` setzt den Text neu wie eine Bearbeitung: 18 Eigenschaften, Zeilenabstand f32;
/// der unberührte Text bleibt in der Datei-Form. Nach Speichern/Neuladen wieder 15 und 1.2.
#[test]
fn t11_set_text_runs_relays_and_reload_restores_file_form() {
    let Some(mut s) = par_text() else { return };
    select_named(&mut s, "TX1");
    run(
        &mut s,
        "var d = fw.getDocumentDOM(); d.setTextRuns(fw.selection[0].textRuns); 0",
    );
    assert_eq!(
        run(&mut s, RUNS),
        "1.1999999682108562|18:1.1999999682108562"
    );
    select_named(&mut s, "TX2");
    run(
        &mut s,
        "var d = fw.getDocumentDOM(); d.setTextRuns({initialAttrs: {}, textRuns: [{changedAttrs: {}, characters: 'Neuer Text'}]}); 0",
    );
    assert!(
        run(&mut s, RUNS).starts_with("1.1999999455043249|18:"),
        "{}",
        run(&mut s, RUNS)
    );
    select_named(&mut s, "TX3");
    assert!(
        run(&mut s, RUNS).ends_with("|15:1.2"),
        "{}",
        run(&mut s, RUNS)
    );
    // speichern und neu laden: Datei-Form (relaid wird nicht gespeichert)
    let p = std::env::temp_dir().join(format!("fp-t11-{}-t2c.fw.png", std::process::id()));
    s.execute("file.saveAs", json!({"path": p.to_string_lossy()}))
        .expect("speichern");
    let mut s2 = Session::default();
    s2.execute("file.open", json!({"path": p.to_string_lossy()}))
        .expect("öffnen");
    let _ = std::fs::remove_file(&p);
    for n in ["TX1", "TX2"] {
        select_named(&mut s2, n);
        assert!(
            run(&mut s2, RUNS).starts_with("1.2|15:1.2"),
            "{n}: {}",
            run(&mut s2, RUNS)
        );
    }
    assert_eq!(
        run(&mut s2, "fw.selection[0].textRuns.textRuns[0].characters"),
        "Neuer Text"
    );
}

/// Neu angelegter Text: `initialAttrs` mit 18 Eigenschaften und Zeilenabstand in f32-Form.
#[test]
fn t11_new_text_reports_18_initial_attrs() {
    let mut s = Session::default();
    let got = run(
        &mut s,
        &format!(
            "{NEW}dom.addNewText({{left: 10, top: 10, right: 200, bottom: 60}}, true); var a = fw.selection[0].textRuns.initialAttrs, c = 0; for (var n in a) c++; c + '|' + (a.leading != 1.2)"
        ),
    );
    assert_eq!(got, "18|true");
}

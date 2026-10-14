//! F4 (docs\parity\TWEAKS_2026-10-14.md): TW-17 Leinwand zuschneiden nur innerhalb der Leinwand,
//! TW-26 Ergebnisse der Pfadoperationen ohne Namen, TW-27 Namen bei „In Bitmap/Symbol
//! konvertieren“, TW-31 „Ähnliches auswählen“ mit Deckkraft, Verlaufsnamen deutsch (np1 Nr. 5).

use crate::common;

use common::*;
use fp_engine::Session;
use fp_model::ElementId;
use serde_json::json;

fn name(s: &Session, id: ElementId) -> Option<String> {
    el(s, id).common.name.clone()
}

fn set_name(s: &mut Session, id: ElementId, n: &str) {
    ex(s, "element.setName", json!({"ids": [id.0], "name": n}));
}

fn selected(s: &Session) -> Vec<ElementId> {
    s.active().map(|d| d.selection.elements.clone()).unwrap_or_default()
}

fn canvas_size(s: &Session) -> (f32, f32) {
    let d = s.active().expect("Dokument");
    let p = &d.doc.pages[d.page];
    (p.canvas.width, p.canvas.height)
}

/// TW-17: Ein Objekt ragt über den rechten Rand (Original: 750 × 336 statt 796 × 336).
#[test]
fn trim_canvas_stays_inside_canvas() {
    let mut s = doc_session(800.0, 600.0);
    rect(&mut s, 50.0, 50.0, 150.0, 120.0);
    rect(&mut s, 654.0, 205.0, 846.0, 386.0);
    ex(&mut s, "select.none", json!({}));
    ex(&mut s, "modify.canvas.trimCanvas", json!({}));
    assert_eq!(canvas_size(&s), (750.0, 336.0));
}

/// TW-26: Vereinigen, Überschneiden, Ausstanzen, Zuschneiden, Teilen, Verbinden, Strich
/// erweitern und Pfad einlassen liefern namenlose Objekte; Vereinfachen behält den Namen.
#[test]
fn path_operations_drop_the_name() {
    for op in ["union", "intersect", "punch", "crop"] {
        let mut s = doc_session(400.0, 300.0);
        let a = rect(&mut s, 10.0, 10.0, 110.0, 80.0);
        let b = rect(&mut s, 60.0, 40.0, 160.0, 120.0);
        set_name(&mut s, a, "R1");
        set_name(&mut s, b, "R2");
        select(&mut s, &[a, b]);
        ex(&mut s, &format!("modify.combinePaths.{op}"), json!({}));
        let sel = selected(&s);
        assert!(!sel.is_empty(), "{op}: Ergebnis gewählt");
        for id in sel {
            assert_eq!(name(&s, id), None, "{op}: Ergebnis ohne Namen");
        }
    }
    // Verbinden (zwei geschlossene Pfade → ein zusammengesetzter), dann Teilen
    let mut s = doc_session(400.0, 300.0);
    let a = rect(&mut s, 10.0, 10.0, 50.0, 50.0);
    let b = rect(&mut s, 100.0, 10.0, 150.0, 50.0);
    set_name(&mut s, a, "R1");
    set_name(&mut s, b, "R2");
    select(&mut s, &[a, b]);
    ex(&mut s, "modify.combinePaths.join", json!({}));
    let j = selected(&s);
    assert_eq!(j.len(), 1);
    assert_eq!(name(&s, j[0]), None, "Verbinden ohne Namen");
    set_name(&mut s, j[0], "J");
    ex(&mut s, "modify.combinePaths.split", json!({}));
    let parts = selected(&s);
    assert_eq!(parts.len(), 2);
    assert!(parts.iter().all(|id| name(&s, *id).is_none()), "Teilen ohne Namen");

    for (cmd, keeps) in [("modify.alterPath.expandStroke", false), ("modify.alterPath.insetPath", false), ("modify.alterPath.simplify", true)] {
        let mut s = doc_session(400.0, 300.0);
        let a = rect(&mut s, 10.0, 10.0, 110.0, 80.0);
        ex(&mut s, "element.setStroke", json!({"ids": [a.0], "color": "#000000", "width": 4}));
        set_name(&mut s, a, "P1");
        select(&mut s, &[a]);
        ex(&mut s, cmd, json!({}));
        let id = selected(&s)[0];
        assert_eq!(name(&s, id).is_some(), keeps, "{cmd}");
    }
}

/// TW-27: In Bitmap konvertieren behält den Namen, die Symbolinstanz heißt wie das Symbol.
#[test]
fn convert_keeps_or_sets_names() {
    let mut s = doc_session(400.0, 300.0);
    let a = rect(&mut s, 10.0, 10.0, 110.0, 80.0);
    set_name(&mut s, a, "ST");
    select(&mut s, &[a]);
    let v = ex(&mut s, "modify.flattenSelection", json!({}));
    let id = ElementId(v["id"].as_u64().expect("id"));
    assert_eq!(name(&s, id).as_deref(), Some("ST"));

    let b = rect(&mut s, 150.0, 10.0, 250.0, 80.0);
    set_name(&mut s, b, "R2");
    select(&mut s, &[b]);
    let v = ex(&mut s, "modify.symbol.convertToSymbol", json!({"name": "Symbol"}));
    let id = ElementId(v["id"].as_u64().expect("id"));
    assert_eq!(name(&s, id).as_deref(), Some("Symbol"));
}

/// TW-31: R2 unterscheidet sich nur in der Deckkraft (par_vector) und wird nicht mitgewählt.
#[test]
fn select_similar_respects_opacity() {
    let mut s = doc_session(400.0, 300.0);
    let r1 = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
    let r2 = rect(&mut s, 200.0, 50.0, 300.0, 120.0);
    let r3 = rect(&mut s, 50.0, 150.0, 150.0, 220.0);
    ex(&mut s, "element.setOpacity", json!({"ids": [r2.0], "opacity": 50}));
    select(&mut s, &[r1]);
    ex(&mut s, "select.similar", json!({}));
    let sel = selected(&s);
    assert!(sel.contains(&r1) && sel.contains(&r3), "{sel:?}");
    assert!(!sel.contains(&r2), "R2 (50 %) nicht mitgewählt");
}

/// np1 Nr. 5: Verlaufsname eines mit dem Rechteck-Werkzeug gezeichneten Rechtecks ist in der
/// deutschen Oberfläche „Rechteck“, auch wenn die Ereignisse über die Brücke (englisch) kommen.
#[test]
fn history_names_are_german() {
    fp_i18n::with_language("de", || {
        let mut s = doc_session(400.0, 300.0);
        fp_i18n::with_language("en", || {
            ex(&mut s, "draw.rectangle", json!({"rect": [10, 10, 60, 60]}));
            ex(&mut s, "draw.ellipse", json!({"rect": [70, 10, 120, 60]}));
            // wie `tool_event` der Brücke: Rechteck-Werkzeug ziehen
            s.set_tool(fp_engine::tools::ToolId::Rectangle).expect("Werkzeug");
            events(&mut s, &[fp_engine::ToolEvent::down(150.0, 10.0), fp_engine::ToolEvent::drag(170.0, 30.0), fp_engine::ToolEvent::drag(200.0, 60.0), fp_engine::ToolEvent::up(200.0, 60.0)]);
        });
        let d = s.active().expect("Dokument");
        let labels: Vec<String> = d.history.steps().iter().map(|st| st.label.clone()).collect();
        eprintln!("Verlauf: {labels:?}");
        assert!(labels.iter().filter(|l| *l == "Rechteck").count() >= 2, "{labels:?}");
        assert!(!labels.iter().any(|l| l == "Rectangle"), "{labels:?}");
    });
}

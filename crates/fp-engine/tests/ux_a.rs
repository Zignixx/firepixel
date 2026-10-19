//! UX-Abgleich A: Einfügeposition und Einpassen wie im Original (Messwerte aus
//! `docs\UX_ABGLEICH_A.md`, Szenarien C01–C04, Z09–Z11).

use fp_engine::Session;
use serde_json::json;

fn session(w: u32, h: u32) -> Session {
    fp_engine::shell::disable();
    let mut s = Session::default();
    s.execute("file.new", json!({"width": w, "height": h, "background": "#ffffff"})).expect("neu");
    s
}

fn rect(s: &mut Session, r: [i32; 4]) -> u64 {
    let v = s.execute("draw.rectangle", json!({"rect": r, "fillColor": "#ff0000", "strokeWidth": 0})).expect("Rechteck");
    v.value["id"].as_u64().expect("id")
}

fn sel_bounds(s: &Session) -> [f32; 4] {
    let d = s.active().expect("d");
    let b = fp_engine::docops::ids_bounds(&d.doc, d.page, &d.selection.elements).expect("Grenzen");
    [b.left, b.top, b.right, b.bottom]
}

#[test]
fn paste_keeps_position_when_visible() {
    let mut s = session(600, 400);
    rect(&mut s, [50, 50, 150, 120]);
    s.execute("edit.copy", json!({})).expect("kopieren");
    s.execute("select.set", json!({"ids": []})).expect("aufheben");
    s.execute("edit.paste", json!({})).expect("einfügen");
    assert_eq!(sel_bounds(&s), [50.0, 50.0, 150.0, 120.0]);
    // zweites Einfügen: wieder dieselbe Stelle (kein Versatz)
    s.execute("edit.paste", json!({})).expect("einfügen");
    assert_eq!(sel_bounds(&s), [50.0, 50.0, 150.0, 120.0]);
}

#[test]
fn paste_centers_when_outside_target_canvas() {
    let mut s = session(600, 400);
    rect(&mut s, [400, 300, 500, 380]);
    s.execute("edit.copy", json!({})).expect("kopieren");
    // Original: 300×200 → 100,60–200,140
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("neu");
    s.execute("edit.paste", json!({})).expect("einfügen");
    assert_eq!(sel_bounds(&s), [100.0, 60.0, 200.0, 140.0]);
    // Original: 450×350 → teilweise drin, bleibt
    s.execute("file.new", json!({"width": 450, "height": 350, "background": "#ffffff"})).expect("neu");
    s.execute("edit.paste", json!({})).expect("einfügen");
    assert_eq!(sel_bounds(&s), [400.0, 300.0, 500.0, 380.0]);
    // Original: 400×300 → berührt nur den Rand, bleibt
    s.execute("file.new", json!({"width": 400, "height": 300, "background": "#ffffff"})).expect("neu");
    s.execute("edit.paste", json!({})).expect("einfügen");
    assert_eq!(sel_bounds(&s), [400.0, 300.0, 500.0, 380.0]);
    // Original: größer als das Ziel (50×30) → -25,-25–75,55
    s.execute("file.new", json!({"width": 50, "height": 30, "background": "#ffffff"})).expect("neu");
    s.execute("edit.paste", json!({})).expect("einfügen");
    assert_eq!(sel_bounds(&s), [-25.0, -25.0, 75.0, 55.0]);
}

#[test]
fn paste_centers_union_of_several_objects() {
    let mut s = session(600, 400);
    let a = rect(&mut s, [400, 300, 500, 380]);
    let b = rect(&mut s, [350, 250, 380, 290]);
    s.execute("select.set", json!({"ids": [a, b]})).expect("auswahl");
    s.execute("edit.copy", json!({})).expect("kopieren");
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("neu");
    s.execute("edit.paste", json!({})).expect("einfügen");
    // Original: Vereinigung 350,250–500,380 → 75,35–225,165
    assert_eq!(sel_bounds(&s), [75.0, 35.0, 225.0, 165.0]);
    // ein Verlaufsschritt, Rückgängig entfernt beide
    s.execute("edit.undo", json!({})).expect("rückgängig");
    let d = s.active().expect("d");
    let n: usize = fp_engine::docops::own_layers(&d.doc, d.page).iter().map(|(_, l)| l.cels[0].elements.len()).sum();
    assert_eq!(n, 0);
}

#[test]
fn paste_centers_in_visible_part_when_zoomed() {
    let mut s = session(600, 400);
    rect(&mut s, [10, 10, 60, 40]);
    s.execute("edit.copy", json!({})).expect("kopieren");
    s.execute("view.setViewport", json!({"width": 2222, "height": 1132})).expect("sicht");
    s.execute("view.setZoom", json!({"zoom": 8.0, "at": [300, 200]})).expect("zoom");
    {
        let d = s.active_mut().expect("d");
        d.view.origin = (300.0 - 2222.0 / 16.0, 200.0 - 1132.0 / 16.0);
    }
    s.execute("edit.paste", json!({})).expect("einfügen");
    // Original (800 %, Blick auf die Mitte): 275,185–325,215
    assert_eq!(sel_bounds(&s), [275.0, 185.0, 325.0, 215.0]);
}

#[test]
fn fit_all_and_selection_are_stepless_with_margin() {
    let mut s = session(600, 400);
    s.execute("view.setViewport", json!({"width": 2222, "height": 1132})).expect("sicht");
    s.execute("view.fitAll", json!({})).expect("alles");
    assert!((s.active().expect("d").view.zoom - 2.755).abs() < 1e-6);
    rect(&mut s, [10, 10, 60, 40]);
    s.execute("view.fitSelection", json!({})).expect("auswahl");
    assert!((s.active().expect("d").view.zoom - 1102.0 / 30.0).abs() < 1e-6);
    // ohne Auswahl wie Alles anpassen
    s.execute("select.set", json!({"ids": []})).expect("aufheben");
    s.execute("view.fitSelection", json!({})).expect("auswahl");
    assert!((s.active().expect("d").view.zoom - 2.755).abs() < 1e-6);
}

fn undo_count(s: &Session) -> usize {
    s.active().expect("d").history.undo_count()
}

/// Script-driven nudges and moves (`edit.nudge.*`, `edit.move` over JSF/MCP) stay one history row
/// per call; only arrow keys on the canvas merge (TW-148, see `re51_nudge_rows.rs`).
#[test]
fn script_nudges_are_one_step_each() {
    let mut s = session(600, 400);
    rect(&mut s, [50, 50, 150, 120]);
    let n0 = undo_count(&s);
    for _ in 0..3 {
        s.execute("edit.nudge.right", json!({})).expect("right");
    }
    s.execute("edit.nudge.bigDown", json!({})).expect("down");
    assert_eq!(sel_bounds(&s), [53.0, 60.0, 153.0, 130.0]);
    assert_eq!(undo_count(&s), n0 + 4);
    let steps = s.active().expect("d").history.steps().iter().map(|x| x.command.clone()).collect::<Vec<_>>();
    assert_eq!(&steps[n0..], ["edit.nudge.right", "edit.nudge.right", "edit.nudge.right", "edit.nudge.bigDown"]);
    // undo steps back one press at a time
    s.execute("edit.undo", json!({})).expect("undo");
    assert_eq!(sel_bounds(&s), [53.0, 50.0, 153.0, 120.0]);
    s.execute("edit.undo", json!({})).expect("undo");
    assert_eq!(sel_bounds(&s), [52.0, 50.0, 152.0, 120.0]);
    s.execute("edit.redo", json!({})).expect("redo");
    assert_eq!(sel_bounds(&s), [53.0, 50.0, 153.0, 120.0]);
    let n1 = undo_count(&s);
    s.execute("edit.move", json!({"dx": 10, "dy": 0})).expect("move");
    s.execute("edit.move", json!({"dx": 10, "dy": 0})).expect("move");
    assert_eq!(undo_count(&s), n1 + 2);
}

#[test]
fn nudge_after_other_step_or_undo_starts_new_step() {
    let mut s = session(600, 400);
    rect(&mut s, [50, 50, 150, 120]);
    s.execute("edit.nudge.right", json!({})).expect("rechts");
    let n1 = undo_count(&s);
    s.execute("edit.duplicate", json!({})).expect("duplizieren");
    s.execute("edit.nudge.right", json!({})).expect("rechts");
    assert_eq!(undo_count(&s), n1 + 2);
    s.execute("edit.undo", json!({})).expect("rückgängig");
    s.execute("edit.nudge.down", json!({})).expect("unten");
    assert_eq!(undo_count(&s), n1 + 2);
    assert_eq!(s.active().expect("d").history.redo_count(), 0);
}

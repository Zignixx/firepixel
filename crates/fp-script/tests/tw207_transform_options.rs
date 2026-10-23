//! TW-207 (RE spec 61 §1.5): `scaleSelection`, `rotateSelection`, `setQuadrangle` and
//! `transformSelection` transform the fill handles only when the options string contains
//! `transformAttributes`; with `"autoTrimImages"` alone the handles stay in the document. The
//! History rows Firepixel writes for the numeric dialog and the menu rotations carry the matching
//! `scaleAttributes`, so replaying a row gives the same handle behaviour.

use fp_script::{ScriptOptions, run_source};
use serde_json::{Value, json};

fn run(s: &mut fp_engine::Session, src: &str) -> String {
    let o = run_source(s, src, &ScriptOptions::default());
    assert!(
        o.ok,
        "error: {:?}\ndiagnostics: {:?}",
        o.error, o.diagnostics
    );
    o.result.unwrap_or_default()
}

/// Rectangle 100,100–260,220 without a stroke and with a linear gradient fill, selected.
fn session_with_gradient_rect() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute(
        "file.new",
        json!({"width": 600, "height": 400, "background": "#ffffff"}),
    )
    .expect("document");
    s.execute(
        "draw.rectangle",
        json!({"rect": [100, 100, 260, 220], "fillColor": "#336699", "strokeWidth": 0}),
    )
    .expect("rectangle");
    s.execute("pi.fill", json!({"category": "gradient"}))
        .expect("gradient");
    s
}

/// The three fill handles through the script accessors `pathAttributes.fillHandle1..3`.
fn handles(s: &mut fp_engine::Session) -> [(f64, f64); 3] {
    let t = run(
        s,
        "var p = fw.selection[0].pathAttributes; JSON.stringify([p.fillHandle1, p.fillHandle2, p.fillHandle3])",
    );
    let v: Value = serde_json::from_str(&t).expect("json");
    let pt = |i: usize| {
        (
            v[i]["x"].as_f64().expect("x"),
            v[i]["y"].as_f64().expect("y"),
        )
    };
    [pt(0), pt(1), pt(2)]
}

fn bounds(s: &mut fp_engine::Session) -> String {
    run(
        s,
        "var b = fw.getDocumentDOM().getSelectionBounds(); [b.left, b.top, b.right, b.bottom].map(Math.round).join(',')",
    )
}

fn assert_near(got: [(f64, f64); 3], want: [(f64, f64); 3], what: &str) {
    for (g, w) in got.iter().zip(want.iter()) {
        assert!(
            (g.0 - w.0).abs() < 0.01 && (g.1 - w.1).abs() < 0.01,
            "{what}: {got:?} != {want:?}"
        );
    }
}

fn map(h: [(f64, f64); 3], f: impl Fn((f64, f64)) -> (f64, f64)) -> [(f64, f64); 3] {
    h.map(f)
}

const C: (f64, f64) = (180.0, 160.0);

#[test]
fn scale_selection_transforms_handles_only_with_transform_attributes() {
    let mut s = session_with_gradient_rect();
    let h0 = handles(&mut s);
    assert!(
        h0.iter()
            .any(|h| (h.0 - C.0).abs() > 1.0 || (h.1 - C.1).abs() > 1.0),
        "handles away from the centre: {h0:?}"
    );
    run(
        &mut s,
        "fw.getDocumentDOM().scaleSelection(1.5, 1.5, 'autoTrimImages transformAttributes')",
    );
    assert_eq!(bounds(&mut s), "60,70,300,250");
    assert_near(
        handles(&mut s),
        map(h0, |(x, y)| (C.0 + 1.5 * (x - C.0), C.1 + 1.5 * (y - C.1))),
        "scaled about the centre",
    );

    let mut s = session_with_gradient_rect();
    run(
        &mut s,
        "fw.getDocumentDOM().scaleSelection(1.5, 1.5, 'autoTrimImages')",
    );
    assert_eq!(bounds(&mut s), "60,70,300,250", "the object still scales");
    assert_near(handles(&mut s), h0, "handles stay in the document");
}

#[test]
fn rotate_selection_transforms_handles_only_with_transform_attributes() {
    let rot = |(x, y): (f64, f64)| {
        let (c, sn) = (30f64.to_radians().cos(), 30f64.to_radians().sin());
        let (dx, dy) = (x - C.0, y - C.1);
        (C.0 + dx * c - dy * sn, C.1 + dx * sn + dy * c)
    };
    let mut s = session_with_gradient_rect();
    let h0 = handles(&mut s);
    run(
        &mut s,
        "fw.getDocumentDOM().rotateSelection(30, 'autoTrimImages transformAttributes')",
    );
    assert_near(handles(&mut s), map(h0, rot), "rotated about the centre");

    let mut s = session_with_gradient_rect();
    run(
        &mut s,
        "fw.getDocumentDOM().rotateSelection(30, 'autoTrimImages')",
    );
    assert_ne!(
        bounds(&mut s),
        "100,100,260,220",
        "the object still rotates"
    );
    assert_near(handles(&mut s), h0, "handles stay in the document");
}

#[test]
fn set_quadrangle_transforms_handles_only_with_transform_attributes() {
    // the selection bounds scaled by 1.25 about the top-left corner (Free Transform corner drag)
    let corner = |(x, y): (f64, f64)| (100.0 + 1.25 * (x - 100.0), 100.0 + 1.25 * (y - 100.0));
    let mut s = session_with_gradient_rect();
    let h0 = handles(&mut s);
    // four points as in the API table, the options as fifth argument
    run(
        &mut s,
        "fw.getDocumentDOM().setQuadrangle({x: 100, y: 100}, {x: 300, y: 100}, {x: 300, y: 250}, {x: 100, y: 250}, 'autoTrimImages transformAttributes')",
    );
    assert_eq!(bounds(&mut s), "100,100,300,250");
    assert_near(handles(&mut s), map(h0, corner), "four-point form");
    let mut s = session_with_gradient_rect();
    run(
        &mut s,
        "fw.getDocumentDOM().setQuadrangle({x: 100, y: 100}, {x: 300, y: 100}, {x: 300, y: 250}, {x: 100, y: 250}, 'autoTrimImages')",
    );
    assert_near(handles(&mut s), h0, "four-point form without the option");
}

#[test]
fn transform_selection_honours_transform_attributes_but_moves_always() {
    let mut s = session_with_gradient_rect();
    let h0 = handles(&mut s);
    run(
        &mut s,
        "fw.getDocumentDOM().transformSelection({matrix: [2, 0, 0, 0, 1, 0, -100, 0, 1]}, 'autoTrimImages transformAttributes')",
    );
    assert_eq!(bounds(&mut s), "100,100,420,220");
    assert_near(
        handles(&mut s),
        map(h0, |(x, y)| (2.0 * x - 100.0, y)),
        "matrix applied",
    );
    let mut s = session_with_gradient_rect();
    run(
        &mut s,
        "fw.getDocumentDOM().transformSelection({matrix: [2, 0, 0, 0, 1, 0, -100, 0, 1]}, 'autoTrimImages')",
    );
    assert_eq!(bounds(&mut s), "100,100,420,220");
    assert_near(handles(&mut s), h0, "handles stay in the document");
    // a pure move always takes the handles along
    run(
        &mut s,
        "fw.getDocumentDOM().transformSelection({matrix: [1, 0, 0, 0, 1, 0, 10, 5, 1]}, 'autoTrimImages')",
    );
    assert_near(
        handles(&mut s),
        map(h0, |(x, y)| (x + 10.0, y + 5.0)),
        "move",
    );
}

/// The row of the last step as "Copy Steps" writes it.
fn last_row(s: &mut fp_engine::Session) -> String {
    let n = s.active().map(|d| d.history.undo_count()).unwrap_or(0);
    let v = s
        .execute("panel.history.copySteps", json!({"steps": [n - 1]}))
        .expect("copy steps")
        .value;
    v["script"].as_str().unwrap_or_default().trim().to_string()
}

#[test]
fn numeric_dialog_and_rotate_rows_carry_the_handle_behaviour() {
    // Numeric Transform 150 % with "Scale attributes" on / off (the dialog's parameters)
    for attrs in [true, false] {
        let mut s = session_with_gradient_rect();
        let h0 = handles(&mut s);
        s.execute(
            "modify.transform.numericTransform",
            json!({"mode": "scale", "scaleX": 150.0, "scaleY": 150.0, "constrain": true, "scaleAttributes": attrs}),
        )
        .expect("numeric transform");
        let after = handles(&mut s);
        let row = last_row(&mut s);
        assert!(
            row.starts_with("fp.run(\"modify.transform.numericTransform\"")
                && row.contains(&format!("\"scaleAttributes\":{attrs}")),
            "{row}"
        );
        // replaying the row after undo gives the same handles
        s.execute("edit.undo", json!({})).expect("undo");
        assert_near(handles(&mut s), h0, "undo");
        run(&mut s, &row);
        assert_near(handles(&mut s), after, "replayed row");
        if attrs {
            assert_near(
                after,
                map(h0, |(x, y)| (C.0 + 1.5 * (x - C.0), C.1 + 1.5 * (y - C.1))),
                "on",
            );
        } else {
            assert_near(after, h0, "off");
        }
    }

    // Modify ▸ Transform ▸ Rotate 90° CW always transforms the handles (measured row with
    // "autoTrimImages transformAttributes"); its row has no option to lose
    let mut s = session_with_gradient_rect();
    let h0 = handles(&mut s);
    s.execute("modify.transform.rotate90DegreeCw", json!({}))
        .expect("rotate");
    let after = handles(&mut s);
    assert_near(
        after,
        map(h0, |(x, y)| (C.0 - (y - C.1), C.1 + (x - C.0))),
        "rotated about the centre",
    );
    let row = last_row(&mut s);
    assert_eq!(row, "fp.run(\"modify.transform.rotate90DegreeCw\", {});");
    s.execute("edit.undo", json!({})).expect("undo");
    run(&mut s, &row);
    assert_near(handles(&mut s), after, "replayed rotation");
}

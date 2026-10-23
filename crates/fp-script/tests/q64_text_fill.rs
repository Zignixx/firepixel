//! Q-64 (RE spec 62 §2.4): `scaleSelection` scales a text's fill handles about the centre with and
//! without `transformAttributes`; `moveSelectionTo` (the text's X/Y fields) takes them along; the
//! handles of a new linear gradient on a text are top centre, bottom centre and h1 + (height, 0).

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

/// "Ag" ArialMT 60 px with a linear gradient fill, selected.
fn session_with_gradient_text() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute(
        "file.new",
        json!({"width": 600, "height": 400, "background": "#ffffff"}),
    )
    .expect("document");
    let v = s
        .execute(
            "draw.text",
            json!({"text": "Ag", "anchor": [100, 150], "size": 60, "font": "ArialMT", "color": "#000000"}),
        )
        .expect("text")
        .value;
    s.execute("select.set", json!({"ids": [v["id"].clone()]}))
        .expect("select");
    s.execute("pi.fill", json!({"category": "gradient"}))
        .expect("gradient");
    s
}

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

fn bounds(s: &mut fp_engine::Session) -> [f64; 4] {
    let t = run(
        s,
        "var b = fw.getDocumentDOM().getSelectionBounds(); JSON.stringify([b.left, b.top, b.right, b.bottom])",
    );
    let v: Value = serde_json::from_str(&t).expect("json");
    [0, 1, 2, 3].map(|i| v[i].as_f64().expect("number"))
}

fn assert_near(got: [(f64, f64); 3], want: [(f64, f64); 3], what: &str) {
    for (g, w) in got.iter().zip(want.iter()) {
        assert!(
            (g.0 - w.0).abs() < 0.01 && (g.1 - w.1).abs() < 0.01,
            "{what}: {got:?} != {want:?}"
        );
    }
}

#[test]
fn new_text_gradient_handles_follow_the_object_bounds() {
    let mut s = session_with_gradient_text();
    let [l, t, r, b] = bounds(&mut s);
    let cx = (l + r) / 2.0;
    assert_near(
        handles(&mut s),
        [(cx, t), (cx, b), (cx + (b - t), t)],
        "h1 top centre, h2 bottom centre, h3 = h1 + (height, 0)",
    );
}

#[test]
fn scale_selection_scales_text_handles_with_and_without_transform_attributes() {
    for opts in ["autoTrimImages transformAttributes", "autoTrimImages"] {
        let mut s = session_with_gradient_text();
        let h0 = handles(&mut s);
        let [l, t, r, b] = bounds(&mut s);
        let c = ((l + r) / 2.0, (t + b) / 2.0);
        run(
            &mut s,
            &format!("fw.getDocumentDOM().scaleSelection(1.5, 1.5, \"{opts}\")"),
        );
        let want = h0.map(|(x, y)| (c.0 + (x - c.0) * 1.5, c.1 + (y - c.1) * 1.5));
        assert_near(handles(&mut s), want, opts);
    }
}

#[test]
fn move_selection_to_takes_text_handles_along() {
    let mut s = session_with_gradient_text();
    let h0 = handles(&mut s);
    let [l, t, _, _] = bounds(&mut s);
    run(
        &mut s,
        &format!(
            "fw.getDocumentDOM().moveSelectionTo({{x:{}, y:{}}}, false, false)",
            l + 100.0,
            t + 37.0
        ),
    );
    assert_near(
        handles(&mut s),
        h0.map(|(x, y)| (x + 100.0, y + 37.0)),
        "moveSelectionTo",
    );
}

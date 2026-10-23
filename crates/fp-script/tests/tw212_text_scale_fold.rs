//! TW-212 (RE spec 66): `scaleSelection` folds a pure axis scale into the font size and horizontal
//! scale of every selected text; `setQuadrangle` takes its points relative to the unit square of
//! the selection bounds (side max(width, height) about their centre) and folds a single text only
//! with the option `rememberQuad`.

use fp_model::{ElementKind, TextElem};
use fp_script::{ScriptOptions, run_source};
use serde_json::json;

fn run(s: &mut fp_engine::Session, src: &str) -> String {
    let o = run_source(s, src, &ScriptOptions::default());
    assert!(
        o.ok,
        "error: {:?}\ndiagnostics: {:?}",
        o.error, o.diagnostics
    );
    o.result.unwrap_or_default()
}

fn session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute(
        "file.new",
        json!({"width": 600, "height": 400, "background": "#ffffff"}),
    )
    .expect("document");
    s
}

fn arial() -> bool {
    let ok = fp_text::FontDb::shared().find_ps("ArialMT").is_some();
    if !ok {
        eprintln!("skipped: ArialMT not installed");
    }
    ok
}

/// "Ag" ArialMT 60 px at the baseline `x`/`y`, selected.
fn add_text(s: &mut fp_engine::Session, x: f32, y: f32) -> fp_model::ElementId {
    let v = s
        .execute(
            "draw.text",
            json!({"text": "Ag", "anchor": [x, y], "size": 60, "font": "ArialMT", "color": "#000000"}),
        )
        .expect("text")
        .value;
    fp_model::ElementId(v["id"].as_u64().unwrap_or(0))
}

fn text(s: &fp_engine::Session, id: fp_model::ElementId) -> TextElem {
    let d = s.active().expect("document");
    match fp_engine::docops::element_on_page(&d.doc, d.page, id).map(|e| e.kind.clone()) {
        Some(ElementKind::Text(t)) => *t,
        _ => panic!("no text"),
    }
}

fn size_hs(t: &TextElem) -> (f32, f32) {
    (
        t.initial.size.unwrap_or(0.0),
        t.initial.horizontal_scale.unwrap_or(1.0),
    )
}

fn sel_bounds(s: &mut fp_engine::Session) -> [f64; 4] {
    let r = run(
        s,
        "var b = fw.getDocumentDOM().getSelectionBounds(); JSON.stringify([b.left, b.top, b.right, b.bottom])",
    );
    serde_json::from_str(&r).expect("bounds")
}

/// `setQuadrangle` call mapping the selection bounds `b` onto `to` (document coordinates), with the
/// points written relative to the unit square.
fn quad_call(b: [f64; 4], to: [(f64, f64); 4], opts: &str) -> String {
    let side = (b[2] - b[0]).max(b[3] - b[1]);
    let (x0, y0) = (
        (b[0] + b[2]) / 2.0 - side / 2.0,
        (b[1] + b[3]) / 2.0 - side / 2.0,
    );
    let p: Vec<String> = to
        .iter()
        .map(|(x, y)| format!("{{x: {}, y: {}}}", (x - x0) / side, (y - y0) / side))
        .collect();
    format!(
        "fw.getDocumentDOM().setQuadrangle({}, '{opts}')",
        p.join(", ")
    )
}

#[test]
fn scale_selection_folds_every_selected_text() {
    if !arial() {
        return;
    }
    let mut s = session();
    let a = add_text(&mut s, 100.0, 150.0);
    let b = add_text(&mut s, 300.0, 300.0);
    s.execute("select.set", json!({"ids": [a.0, b.0]}))
        .expect("select");
    run(
        &mut s,
        "fw.getDocumentDOM().scaleSelection(1.5, 1.5, 'autoTrimImages transformAttributes')",
    );
    for id in [a, b] {
        let t = text(&s, id);
        assert_eq!(size_hs(&t), (90.0, 1.0));
        assert_eq!(t.transform, None);
    }
    // without transformAttributes the same; 2/1 → horizontal scale 2
    run(
        &mut s,
        "fw.getDocumentDOM().scaleSelection(2, 1, 'autoTrimImages')",
    );
    assert_eq!(size_hs(&text(&s, a)), (90.0, 2.0));
}

#[test]
fn set_quadrangle_points_are_relative_to_the_unit_square() {
    // rectangle 100 × 60: unchanged it is x 0…1, y 0.2…0.8 (RE spec 66 §2.4)
    let mut s = session();
    s.execute(
        "draw.rectangle",
        json!({"rect": [100, 100, 200, 160], "fillColor": "#336699", "strokeWidth": 0}),
    )
    .expect("rectangle");
    let bounds = |s: &mut fp_engine::Session| -> Vec<i64> {
        sel_bounds(s).iter().map(|v| v.round() as i64).collect()
    };
    assert_eq!(bounds(&mut s), [100, 100, 200, 160]);
    run(
        &mut s,
        "fw.getDocumentDOM().setQuadrangle({x: 0, y: 0.2}, {x: 1, y: 0.2}, {x: 1, y: 0.8}, {x: 0, y: 0.8}, 'autoTrimImages transformAttributes')",
    );
    assert_eq!(bounds(&mut s), [100, 100, 200, 160], "identity quad");
    run(
        &mut s,
        "fw.getDocumentDOM().setQuadrangle({x: 0, y: 0.2}, {x: 2, y: 0.2}, {x: 2, y: 0.8}, {x: 0, y: 0.8}, 'autoTrimImages transformAttributes')",
    );
    assert_eq!(bounds(&mut s), [100, 100, 300, 160], "twice as wide");
}

#[test]
fn set_quadrangle_folds_a_single_text_only_with_remember_quad() {
    if !arial() {
        return;
    }
    let scaled = |b: [f64; 4], k: f64| {
        let (w, h) = (b[2] - b[0], b[3] - b[1]);
        [
            (b[0], b[1]),
            (b[0] + k * w, b[1]),
            (b[0] + k * w, b[1] + k * h),
            (b[0], b[1] + k * h),
        ]
    };
    // with rememberQuad: 90 pt like the Scale tool
    let mut s = session();
    let id = add_text(&mut s, 100.0, 150.0);
    s.execute("select.set", json!({"ids": [id.0]}))
        .expect("select");
    let b = sel_bounds(&mut s);
    run(
        &mut s,
        &quad_call(
            b,
            scaled(b, 1.5),
            "autoTrimImages transformAttributes rememberQuad",
        ),
    );
    let t = text(&s, id);
    assert_eq!(size_hs(&t), (90.0, 1.0), "{:?}", t.transform);
    // "rememberQuad" alone works too
    s.execute("edit.undo", json!({})).expect("undo");
    run(&mut s, &quad_call(b, scaled(b, 1.5), "rememberQuad"));
    assert_eq!(size_hs(&text(&s, id)).0, 90.0);
    // without the option: 60 pt and a matrix
    s.execute("edit.undo", json!({})).expect("undo");
    assert_eq!(size_hs(&text(&s, id)).0, 60.0, "undo");
    run(
        &mut s,
        &quad_call(b, scaled(b, 1.5), "autoTrimImages transformAttributes"),
    );
    let t = text(&s, id);
    assert_eq!(size_hs(&t).0, 60.0);
    let m = t.transform.expect("matrix");
    assert!(
        (m.m[0][0] - 1.5).abs() < 1e-3 && (m.m[1][1] - 1.5).abs() < 1e-3,
        "{m:?}"
    );
    // two texts: a matrix even with rememberQuad
    let mut s = session();
    let a = add_text(&mut s, 100.0, 150.0);
    let c = add_text(&mut s, 300.0, 300.0);
    s.execute("select.set", json!({"ids": [a.0, c.0]}))
        .expect("select");
    let b = sel_bounds(&mut s);
    run(&mut s, &quad_call(b, scaled(b, 1.5), "rememberQuad"));
    for id in [a, c] {
        let t = text(&s, id);
        assert_eq!(size_hs(&t).0, 60.0);
        assert!(t.transform.is_some());
    }
}

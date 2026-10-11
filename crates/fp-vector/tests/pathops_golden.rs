//! Golden-Tests der Pfadoperationen gegen die Python-Referenz (`re\pfade\`).
//! Daten: `tools\golden\fp-vector\gen_pathops.py` → `tests\data\pathops_golden.json`.

use fp_geom::offset::{CapKind, JoinKind, OffsetOptions};
use fp_geom::Vec2;
use fp_vector::pathops::shapes;
use fp_vector::pathops::*;
use serde_json::Value;

const DATA: &str = include_str!("data/pathops_golden.json");

pub fn contour(v: &Value) -> Contour {
    let nodes = v["nodes"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|n| {
                    let g = |i: usize| n[i].as_f64().unwrap_or(0.0);
                    Node::new(Vec2::new(g(0), g(1)), Vec2::new(g(2), g(3)), Vec2::new(g(4), g(5)))
                })
                .collect()
        })
        .unwrap_or_default();
    Contour::new(nodes, v["closed"].as_bool().unwrap_or(false))
}

pub fn contours(v: &Value) -> Vec<Contour> {
    v.as_array().map(|a| a.iter().map(contour).collect()).unwrap_or_default()
}

/// Größte Abweichung zweier Konturlisten (gleiche Struktur vorausgesetzt), sonst `None`.
pub fn max_diff(a: &[Contour], b: &[Contour]) -> Option<f64> {
    if a.len() != b.len() {
        return None;
    }
    let mut m: f64 = 0.0;
    for (x, y) in a.iter().zip(b) {
        if x.nodes.len() != y.nodes.len() || x.closed != y.closed {
            return None;
        }
        for (p, q) in x.nodes.iter().zip(&y.nodes) {
            for (u, v) in [(p.anchor, q.anchor), (p.ctrl_in, q.ctrl_in), (p.ctrl_out, q.ctrl_out)] {
                m = m.max((u.x - v.x).abs()).max((u.y - v.y).abs());
            }
        }
    }
    Some(m)
}

fn check(name: &str, got: &[Contour], want: &[Contour], tol: f64) -> bool {
    match max_diff(got, want) {
        Some(d) if d <= tol => true,
        d => {
            eprintln!("{name}: Abweichung {d:?} (Konturen {} / {})", got.len(), want.len());
            false
        }
    }
}

#[test]
fn boolean_against_reference() {
    let data: Value = serde_json::from_str(DATA).expect("JSON");
    let (mut n, mut ok) = (0, 0);
    for c in data["boolean"].as_array().expect("boolean") {
        let a = Region::with_eof(contours(&c["a"]), c["eo_a"].as_bool().unwrap_or(true));
        let b = Region::with_eof(contours(&c["b"]), c["eo_b"].as_bool().unwrap_or(true));
        for (op, bop) in [("union", BoolOp::Union), ("intersect", BoolOp::Intersect), ("subtract", BoolOp::Subtract), ("xor", BoolOp::Xor)] {
            let got = boolean(&a, &b, bop, boolean::GRID);
            let want = contours(&c["res"][op]);
            n += 1;
            ok += check(&format!("{} {op}", c["name"]), &got, &want, 1e-9) as usize;
        }
    }
    println!("boolesche Operationen: {ok} von {n} gleich der Referenz");
    assert_eq!(ok, n);
}

#[test]
fn offset_against_reference() {
    let data: Value = serde_json::from_str(DATA).expect("JSON");
    let (mut n, mut ok) = (0, 0);
    for c in data["expand"].as_array().expect("expand") {
        let got = expand_stroke(
            &contours(&c["src"]),
            c["width"].as_f64().unwrap_or(0.0),
            JoinKind::from_u32(c["join"].as_u64().unwrap_or(0) as u32),
            c["miter"].as_f64().unwrap_or(4.0),
            CapKind::from_u32(c["cap"].as_u64().unwrap_or(0) as u32),
            &OffsetOptions::REFERENCE,
        );
        n += 1;
        ok += check(&format!("expand {}", c["name"]), &got, &contours(&c["res"]), 1e-7) as usize;
    }
    for c in data["inset"].as_array().expect("inset") {
        let got = inset_path(
            &contours(&c["src"]),
            c["d"].as_f64().unwrap_or(0.0),
            JoinKind::from_u32(c["join"].as_u64().unwrap_or(0) as u32),
            c["miter"].as_f64().unwrap_or(4.0),
            c["expand"].as_bool().unwrap_or(false),
            &OffsetOptions::REFERENCE,
        );
        n += 1;
        ok += check(&format!("inset {}", c["name"]), &got, &contours(&c["res"]), 1e-7) as usize;
    }
    println!("Strich erweitern / Einlassen: {ok} von {n} gleich der Referenz");
    assert_eq!(ok, n);
}

#[test]
fn simplify_against_reference() {
    let data: Value = serde_json::from_str(DATA).expect("JSON");
    let (mut n, mut ok) = (0, 0);
    for c in data["simplify"].as_array().expect("simplify") {
        let got = simplify(&contours(&c["src"]), c["amount"].as_f64().unwrap_or(10.0));
        n += 1;
        ok += check(&format!("simplify {}", c["name"]), &got, &contours(&c["res"]), 1e-7) as usize;
    }
    println!("Vereinfachen: {ok} von {n} gleich der Referenz");
    assert_eq!(ok, n);
}

#[test]
fn shapes_against_reference() {
    let data: Value = serde_json::from_str(DATA).expect("JSON");
    let s = &data["shapes"];
    assert_eq!(s["kappa"].as_f64(), Some(shapes::KAPPA));
    assert!(check("ellipse", &[shapes::ellipse(10.0, 20.0, 30.0, 15.0, shapes::EllipseStart::Left)], &[contour(&s["ellipse"])], 1e-12));
    assert!(check("rounded", &[shapes::rounded_rect(1.0, 2.0, 41.0, 22.0, 6.0)], &[contour(&s["rounded"])], 1e-12));
    let st = shapes::polygon_tool(Vec2::new(0.0, 0.0), Vec2::new(10.0, 5.0), 5, true, None).expect("Stern");
    assert!(check("star", &[st], &[contour(&s["star_auto"])], 1e-9));
    let p7 = shapes::polygon_tool(Vec2::new(3.0, 4.0), Vec2::new(3.0, -6.0), 7, false, None).expect("Polygon");
    assert!(check("polygon7", &[p7], &[contour(&s["polygon7"])], 1e-9));
}

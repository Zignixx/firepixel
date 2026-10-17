//! Golden-Tests gegen die Python-Referenzen (`re\pfade\bezier.py`, `flatten.py`, `offset.py`,
//! `re\vektor\fwraster.py`). Testvektoren erzeugt `tools\golden\fp-geom\gen_geom.py`.

use fp_geom::contour::{signed_area, winding_number};
use fp_geom::flatten::{FlattenOptions, flatten_chord, flatten_contour, flatten_segment};
use fp_geom::offset::{OffsetOptions, offset_segment};
use fp_geom::{BezierContour, BezierNode, CubicBez, Point, Vec2};
use serde_json::Value;

fn load() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/geom_golden.json");
    let text =
        std::fs::read_to_string(path).expect("geom_golden.json fehlt (gen_geom.py ausführen)");
    serde_json::from_str(&text).expect("JSON")
}

fn f(v: &Value) -> f64 {
    v.as_f64().expect("Zahl")
}

fn v2(v: &Value) -> Vec2 {
    Vec2::new(f(&v[0]), f(&v[1]))
}

fn p32(v: &Value) -> Point {
    Point::new(f(&v[0]) as f32, f(&v[1]) as f32)
}

fn seg(v: &Value) -> CubicBez {
    CubicBez::new(v2(&v[0]), v2(&v[1]), v2(&v[2]), v2(&v[3]))
}

fn arr(v: &Value) -> &Vec<Value> {
    v.as_array().expect("Liste")
}

/// Relativer/absoluter Vergleich für Werte, die über `hypot`/`sqrt` laufen (letzte Stelle kann
/// zwischen Python und Rust abweichen).
fn near(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() <= eps * (1.0 + a.abs().max(b.abs()))
}

#[test]
fn bezier_basics_exact() {
    let data = load();
    let ts = [0.0, 0.1, 0.33, 0.5, 0.77, 1.0];
    let mut n = 0;
    for case in arr(&data["bezier"]) {
        let s = seg(&case["seg"]);
        for (i, &t) in ts.iter().enumerate() {
            assert_eq!(
                s.eval(t),
                v2(&case["point"][i]),
                "eval {t} {:?}",
                case["seg"]
            );
            assert_eq!(s.deriv(t), v2(&case["deriv"][i]), "deriv {t}");
            assert_eq!(s.deriv2(t), v2(&case["deriv2"][i]), "deriv2 {t}");
            let tg = s.tangent(t);
            let r = v2(&case["tangent"][i]);
            assert!(
                near(tg.x, r.x, 1e-14) && near(tg.y, r.y, 1e-14),
                "tangent {t}: {tg:?} {r:?}"
            );
        }
        let (a, b) = s.split(f(&case["tsplit"]));
        assert_eq!(a, seg(&case["split"][0]));
        assert_eq!(b, seg(&case["split"][1]));
        assert_eq!(
            s.subsegment(f(&case["t0"]), f(&case["t1"])),
            seg(&case["sub"])
        );
        let ex: Vec<f64> = arr(&case["extrema"]).iter().map(f).collect();
        assert_eq!(s.extrema(), ex);
        let bb = s.bounds();
        let rb: Vec<f64> = arr(&case["bounds"]).iter().map(f).collect();
        assert_eq!(
            [bb.min.x, bb.min.y, bb.max.x, bb.max.y],
            [rb[0], rb[1], rb[2], rb[3]]
        );
        let hb = s.hull_bounds();
        let rh: Vec<f64> = arr(&case["hull"]).iter().map(f).collect();
        assert_eq!(
            [hb.min.x, hb.min.y, hb.max.x, hb.max.y],
            [rh[0], rh[1], rh[2], rh[3]]
        );
        let len = s.length();
        assert!(
            near(len, f(&case["length"]), 1e-12),
            "length {len} {}",
            f(&case["length"])
        );
        let nr = s.nearest(v2(&case["p"]), 16);
        assert!(
            near(nr.t, f(&case["near_t"]), 1e-9),
            "near t {} {}",
            nr.t,
            f(&case["near_t"])
        );
        assert!(near(nr.distance, f(&case["near_d"]), 1e-9));
        n += 1;
    }
    assert_eq!(n, 250);
}

#[test]
fn original_flatten_matches_reference_bitwise() {
    let data = load();
    let mut total = 0;
    let mut same = 0;
    for (key, opts) in [
        ("flatten", FlattenOptions::ORIGINAL),
        (
            "flatten_noquirk",
            FlattenOptions {
                quirk: false,
                ..FlattenOptions::ORIGINAL
            },
        ),
    ] {
        for case in arr(&data[key]) {
            let s = arr(&case["seg"]);
            let p = [p32(&s[0]), p32(&s[1]), p32(&s[2]), p32(&s[3])];
            let n = case["n"].as_u64().unwrap() as u32;
            let mut out = Vec::new();
            flatten_segment(p, n, &opts, &mut out);
            let refp: Vec<Point> = arr(&case["pts"]).iter().map(p32).collect();
            total += 1;
            if out == refp {
                same += 1;
            } else {
                eprintln!(
                    "Abweichung {key}: {:?} n={n}: {} statt {} Punkte",
                    case["seg"],
                    out.len(),
                    refp.len()
                );
            }
        }
    }
    assert_eq!(
        same, total,
        "Original-Abflachung: {same} von {total} Segmenten bitgleich"
    );
}

#[test]
fn original_flatten_contours() {
    let data = load();
    for case in arr(&data["flatten_contour"]) {
        let nodes: Vec<BezierNode> = arr(&case["nodes"])
            .iter()
            .map(|n| BezierNode::new(p32(&n[0]), p32(&n[1]), p32(&n[2])))
            .collect();
        let c = BezierContour::new(nodes, case["closed"].as_bool().unwrap());
        let counts: Vec<u32> = arr(&case["counts"])
            .iter()
            .map(|v| v.as_u64().unwrap() as u32)
            .collect();
        let poly = flatten_contour(&c, Some(&counts), &FlattenOptions::ORIGINAL);
        let refp: Vec<Point> = arr(&case["pts"]).iter().map(p32).collect();
        assert_eq!(poly.points, refp);
    }
}

#[test]
fn legacy_chord_flatten() {
    let data = load();
    for case in arr(&data["chord"]) {
        let s = seg(&case["seg"]);
        let mut out = Vec::new();
        flatten_chord(&s, 0.054, 10, &mut out);
        let refp: Vec<Vec2> = arr(&case["pts"]).iter().map(v2).collect();
        assert_eq!(out.len(), refp.len(), "{:?}", case["seg"]);
        for (a, b) in out.iter().zip(refp.iter()) {
            assert_eq!(a, b);
        }
    }
}

#[test]
fn offset_segment_matches_reference() {
    let data = load();
    let mut cases = 0;
    let mut same_count = 0;
    for case in arr(&data["offset"]) {
        let s = seg(&case["seg"]);
        let d = f(&case["d"]);
        let out = offset_segment(&s, d, &OffsetOptions::REFERENCE);
        let refs: Vec<CubicBez> = arr(&case["out"]).iter().map(seg).collect();
        cases += 1;
        if out.len() != refs.len() {
            eprintln!(
                "Offset: {} statt {} Teilsegmente für {:?} d={d}",
                out.len(),
                refs.len(),
                case["seg"]
            );
            continue;
        }
        same_count += 1;
        for (a, b) in out.iter().zip(refs.iter()) {
            for (p, q) in [(a.p0, b.p0), (a.p1, b.p1), (a.p2, b.p2), (a.p3, b.p3)] {
                assert!(
                    near(p.x, q.x, 1e-9) && near(p.y, q.y, 1e-9),
                    "{p:?} {q:?} in {:?}",
                    case["seg"]
                );
            }
        }
    }
    assert_eq!(
        same_count, cases,
        "Offset: {same_count} von {cases} mit gleicher Teilung"
    );
}

#[test]
fn polygon_area_and_winding() {
    let data = load();
    for case in arr(&data["polygons"]) {
        let pts: Vec<Point> = arr(&case["pts"]).iter().map(p32).collect();
        assert!(near(signed_area(&pts), f(&case["area"]), 1e-12));
        for (i, pr) in arr(&case["probes"]).iter().enumerate() {
            let w = winding_number([&pts[..]], v2(pr));
            assert_eq!(w as i64, case["winding"][i].as_i64().unwrap());
        }
    }
}

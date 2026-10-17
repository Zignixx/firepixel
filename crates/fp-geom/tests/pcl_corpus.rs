//! Korpustest (`#[ignore]`): Original-Abflachung gegen alle gespeicherten `PCLv`-Konturen des
//! Testkorpus (wie `re\scripts\pfade\test_pcl.py`). Eingabe erzeugt
//! `tools\golden\fp-geom\gen_pcl_corpus.py` nach `tools\golden\fp-geom\out\pcl_corpus.jsonl`
//! (oder Pfad in der Umgebungsvariablen `FP_PCL_CORPUS`).
//!
//! Aufruf: `cargo test -p fp-geom --test pcl_corpus -- --ignored --nocapture`

use fp_geom::flatten::{FlattenOptions, flatten_contour};
use fp_geom::{BezierContour, BezierNode, Point};
use serde_json::Value;

fn pt(v: &Value) -> Point {
    Point::new(
        v[0].as_f64().unwrap_or(0.0) as f32,
        v[1].as_f64().unwrap_or(0.0) as f32,
    )
}

/// Schließende Punkte (Abstand < 1e-4 zum Start) entfernen, wie `norm()` in `test_pcl.py`.
fn norm(mut pts: Vec<Point>, closed: bool) -> Vec<Point> {
    while closed && pts.len() > 1 {
        let (a, b) = (pts[0], pts[pts.len() - 1]);
        if ((a.x as f64 - b.x as f64).hypot(a.y as f64 - b.y as f64)) < 1e-4 {
            pts.pop();
        } else {
            break;
        }
    }
    pts
}

fn same(a: &[Point], b: &[Point]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(p, q)| (p.x as f64 - q.x as f64).hypot(p.y as f64 - q.y as f64) <= 1e-3)
}

#[test]
#[ignore]
fn pcl_corpus() {
    let path = std::env::var("FP_PCL_CORPUS").unwrap_or_else(|_| {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/golden/fp-geom/out/pcl_corpus.jsonl"
        )
        .into()
    });
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!("Korpusdatei fehlt: {path} (gen_pcl_corpus.py ausführen) – Test übersprungen");
        return;
    };
    let (mut total, mut same_pcl, mut same_py_bits, mut bzc64, mut bzc64_same) =
        (0u32, 0u32, 0u32, 0u32, 0u32);
    let mut bad = Vec::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let rec: Value = serde_json::from_str(line).expect("JSON-Zeile");
        let closed = rec["closed"].as_bool().unwrap_or(false);
        let nodes: Vec<BezierNode> = rec["nodes"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|n| {
                        let g = |i: usize| n[i].as_f64().unwrap_or(0.0) as f32;
                        BezierNode::new(
                            Point::new(g(0), g(1)),
                            Point::new(g(2), g(3)),
                            Point::new(g(4), g(5)),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        let counts: Vec<u32> = rec["counts"]
            .as_array()
            .map(|a| a.iter().map(|v| v.as_u64().unwrap_or(1) as u32).collect())
            .unwrap_or_default();
        let pcl: Vec<Point> = rec["pcl"]
            .as_array()
            .map(|a| a.iter().map(pt).collect())
            .unwrap_or_default();
        let py: Vec<Point> = match rec.get("py").and_then(|v| v.as_array()) {
            Some(a) => a.iter().map(pt).collect(),
            None => pcl.clone(),
        };
        let got = flatten_contour(
            &BezierContour::new(nodes, closed),
            Some(&counts),
            &FlattenOptions::ORIGINAL,
        )
        .points;
        total += 1;
        let ok = same(&norm(got.clone(), closed), &norm(pcl.clone(), closed));
        same_pcl += ok as u32;
        same_py_bits += (got == py) as u32;
        if counts.iter().any(|&c| c > 1) {
            bzc64 += 1;
            bzc64_same += ok as u32;
        }
        if !ok && bad.len() < 20 {
            bad.push(format!(
                "{} ({} statt {} Punkte)",
                rec["f"],
                got.len(),
                pcl.len()
            ));
        }
    }
    println!(
        "PCLv-Korpus: {same_pcl} von {total} Konturen punktgenau (≤ 0,001 px); BZCi > 1: {bzc64_same} von {bzc64}"
    );
    println!("bitgleich zur Python-Referenz flatten.py: {same_py_bits} von {total}");
    for b in &bad {
        println!("  Abweichung: {b}");
    }
    assert_eq!(
        same_py_bits, total,
        "Rust und flatten.py müssen bitgleich sein"
    );
    assert!(
        same_pcl >= 28_095,
        "erwartet mindestens 28.095 punktgenaue Konturen (Referenz)"
    );
}

//! Korpustest (`#[ignore]`): Compound Shapes des Korpus des Originals und die Laufzeitdateien
//! `re\_testdateien\16` (wie `re\scripts\pfade\test_compound.py`, `test_stale_shift.py`,
//! `test_t16_laufzeit.py`). Daten: `tools\golden\fp-vector\gen_pathops.py` →
//! `tools\golden\fp-vector\out\compound_corpus.json` (oder `FP_COMPOUND_CORPUS`).
//!
//! Aufruf: `cargo test -p fp-vector --release --test compound_corpus -- --ignored --nocapture`

use fp_geom::offset::{CapKind, JoinKind, OffsetOptions};
use fp_geom::Vec2;
use fp_vector::pathops::geom::{bounds, flatten};
use fp_vector::pathops::*;
use serde_json::Value;

fn contour(v: &Value) -> Contour {
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

fn contours(v: &Value) -> Vec<Contour> {
    v.as_array().map(|a| a.iter().map(contour).collect()).unwrap_or_default()
}

/// 4×4-Teilabtastung (Mittelpunkte) der abgeflachten Konturen über `bx` → Bitmaske.
fn sample_mask(cs: &[Contour], even_odd: bool, bx: (i64, i64, i64, i64)) -> Vec<bool> {
    let polys = flatten(cs, 0.02);
    let (x0, y0, x1, y1) = bx;
    let (w, h) = (((x1 - x0) * 4) as usize, ((y1 - y0) * 4) as usize);
    let mut out = vec![false; w * h];
    let mut xs: Vec<(f64, i32)> = Vec::new();
    for j in 0..h {
        let y = y0 as f64 + (j as f64 + 0.5) / 4.0;
        xs.clear();
        for (pts, _) in &polys {
            let n = pts.len();
            for i in 0..n {
                let (a, b) = (pts[i], pts[(i + 1) % n]);
                if (a.y <= y && y < b.y) || (b.y <= y && y < a.y) {
                    xs.push((a.x + (y - a.y) * (b.x - a.x) / (b.y - a.y), if b.y > a.y { 1 } else { -1 }));
                }
            }
        }
        xs.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut wn = 0;
        for k in 0..xs.len() {
            wn += xs[k].1;
            let inside = if even_odd { (k + 1) % 2 == 1 } else { wn != 0 };
            if inside && k + 1 < xs.len() {
                let (a, b) = (xs[k].0, xs[k + 1].0);
                for i in 0..w {
                    let x = x0 as f64 + (i as f64 + 0.5) / 4.0;
                    if x >= a && x < b {
                        out[j * w + i] = true;
                    }
                }
            }
        }
    }
    out
}

fn anchors(cs: &[Contour]) -> Vec<Vec2> {
    cs.iter().flat_map(|c| c.nodes.iter().map(|n| n.anchor)).collect()
}

fn hausdorff(a: &[Vec2], b: &[Vec2]) -> f64 {
    if a.is_empty() || b.is_empty() {
        return f64::INFINITY;
    }
    let d = |p: &[Vec2], q: &[Vec2]| p.iter().map(|x| q.iter().map(|y| x.distance(*y)).fold(f64::INFINITY, f64::min)).fold(0.0, f64::max);
    d(a, b).max(d(b, a))
}

fn same_seq(a: &Contour, b: &Contour, tol: f64) -> bool {
    a.nodes.len() == b.nodes.len()
        && a.nodes.iter().zip(&b.nodes).all(|(x, y)| {
            [(x.anchor, y.anchor), (x.ctrl_in, y.ctrl_in), (x.ctrl_out, y.ctrl_out)].iter().all(|(u, v)| u.distance(*v) <= tol)
        })
}

fn max_diff(a: &[Contour], b: &[Contour]) -> Option<f64> {
    if a.len() != b.len() {
        return None;
    }
    let mut m: f64 = 0.0;
    for (x, y) in a.iter().zip(b) {
        if x.nodes.len() != y.nodes.len() {
            return None;
        }
        for (p, q) in x.nodes.iter().zip(&y.nodes) {
            for (u, v) in [(p.anchor, q.anchor), (p.ctrl_in, q.ctrl_in), (p.ctrl_out, q.ctrl_out)] {
                m = m.max(u.distance(v));
            }
        }
    }
    Some(m)
}

fn load() -> Option<Value> {
    let path = std::env::var("FP_COMPOUND_CORPUS")
        .unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/golden/fp-vector/out/compound_corpus.json").into());
    match std::fs::read_to_string(&path) {
        Ok(t) => serde_json::from_str(&t).ok(),
        Err(_) => {
            eprintln!("Korpusdatei fehlt: {path} (gen_pathops.py ausführen) – Test übersprungen");
            None
        }
    }
}

fn operands(c: &Value) -> Vec<Operand> {
    c["operands"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|o| Operand::new(contours(&o["contours"]), o["eof"].as_bool().unwrap_or(true), PathOp::from_u32(o["pop"].as_u64().unwrap_or(0) as u32)))
                .collect()
        })
        .unwrap_or_default()
}

fn area_diff(got: &[Contour], stored: &[Contour], stored_eof: bool) -> usize {
    let Some((lo, hi)) = bounds(stored, true) else { return usize::MAX };
    let bx = (lo.x.floor() as i64 - 2, lo.y.floor() as i64 - 2, hi.x.ceil() as i64 + 2, hi.y.ceil() as i64 + 2);
    let a = sample_mask(got, true, bx);
    let b = sample_mask(stored, stored_eof, bx);
    a.iter().zip(&b).filter(|(x, y)| x != y).count()
}

#[test]
#[ignore]
fn compound_shapes_of_corpus() {
    let Some(data) = load() else { return };
    let cases = data["compound"].as_array().cloned().unwrap_or_default();
    let (mut n, mut area_ok, mut hd_ok, mut counts_ok, mut order_ok, mut order_counts_ok, mut py_ok) = (0, 0, 0, 0, 0, 0, 0);
    let (mut seq_ok, mut seq_total) = (0, 0);
    let (mut stale_n, mut stale_area, mut stale_seq, mut stale_seq_total) = (0, 0, 0, 0);
    for c in &cases {
        let ops = operands(c);
        let stored = contours(&c["stored"]);
        let python = contours(&c["python"]);
        let got = evaluate_compound(&ops);
        n += 1;
        py_ok += max_diff(&got, &python).is_some_and(|d| d < 1e-6) as usize;
        let ad = area_diff(&got, &stored, c["stored_eof"].as_bool().unwrap_or(true));
        area_ok += (ad == 0) as usize;
        hd_ok += (hausdorff(&anchors(&got), &anchors(&stored)) < 5e-5) as usize;
        let mut nr: Vec<usize> = stored.iter().map(|c| c.nodes.len()).collect();
        let mut ng: Vec<usize> = got.iter().map(|c| c.nodes.len()).collect();
        order_counts_ok += (nr == ng) as usize;
        nr.sort_unstable();
        ng.sort_unstable();
        counts_ok += (nr == ng) as usize;
        seq_total += stored.len();
        seq_ok += stored.iter().filter(|r| got.iter().any(|g| same_seq(r, g, 0.005))).count();
        order_ok += (stored.len() == got.len() && stored.iter().zip(&got).all(|(a, b)| same_seq(a, b, 0.005))) as usize;
        if let Some(st) = c.get("stale_shift") {
            // veraltetes Ergebnis: Operand zurückschieben und neu auswerten (test_stale_shift.py)
            let i = st["operand_from_bottom"].as_u64().unwrap_or(0) as usize;
            let d = Vec2::new(st["delta"][0].as_f64().unwrap_or(0.0), st["delta"][1].as_f64().unwrap_or(0.0));
            let mut ops2 = ops.clone();
            let k = ops2.len() - 1 - i;
            for cc in &mut ops2[k].contours {
                for nd in &mut cc.nodes {
                    nd.anchor += d;
                    nd.ctrl_in += d;
                    nd.ctrl_out += d;
                }
            }
            let g2 = evaluate_compound(&ops2);
            stale_n += 1;
            stale_area += (area_diff(&g2, &stored, true) == 0) as usize;
            stale_seq_total += stored.len();
            stale_seq += stored.iter().filter(|r| g2.iter().any(|g| same_seq(r, g, 0.005))).count();
        }
    }
    println!("Compound Shapes: {n} verschiedene");
    println!("  Fläche exakt (4×4-Teilabtastung): {area_ok} von {n}");
    println!("  Anker-Hausdorff < 5e-5 px: {hd_ok} von {n}");
    println!("  Knotenzahl je Kontur gleich (Multimenge): {counts_ok} von {n}; Reihenfolge der Knotenzahlen: {order_counts_ok} von {n}");
    println!("  Konturen mit gleicher Knotenfolge (±0,005): {seq_ok} von {seq_total}; Konturreihenfolge identisch: {order_ok} von {n}");
    println!("  bitnah zur Python-Referenz (< 1e-6): {py_ok} von {n}");
    println!(
        "  veraltete Fälle nach Zurückschieben: Fläche exakt {stale_area} von {stale_n}, Konturen gleiche Knotenfolge {stale_seq} von {stale_seq_total}"
    );
    println!("  → erklärt: {} von {n}", area_ok + stale_area);
    assert_eq!(py_ok, n);
    assert_eq!(area_ok + stale_area, n);
}

fn anchors3(c: &Contour) -> Vec<(f64, f64)> {
    c.nodes.iter().map(|n| (n.anchor.x, n.anchor.y)).collect()
}

fn same_cycle(a: &[(f64, f64)], b: &[(f64, f64)], tol: f64) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let rev: Vec<(f64, f64)> = b.iter().rev().copied().collect();
    for seq in [b.to_vec(), rev] {
        let m = seq.len();
        for s in 0..m {
            if (0..a.len()).all(|i| (a[i].0 - seq[(i + s) % m].0).abs() <= tol && (a[i].1 - seq[(i + s) % m].1).abs() <= tol) {
                return true;
            }
        }
    }
    false
}

#[test]
#[ignore]
fn t16_runtime_files() {
    let Some(data) = load() else { return };
    let t = &data["t16"];
    let (mut n, mut ok) = (0, 0);
    let mut cmp = |name: &str, fw: &[Contour], got: &[Contour]| {
        let m = fw.iter().filter(|c| got.iter().any(|g| same_cycle(&anchors3(c), &anchors3(g), 0.011))).count();
        println!("{name:<28} Konturen FW {} / Rust {}, Knotenfolgen gleich {m}", fw.len(), got.len());
        n += fw.len();
        ok += m;
    };
    for e in t["expand"].as_array().cloned().unwrap_or_default() {
        let src = contours(&e["src"]);
        let got = expand_stroke(&src, 10.0, JoinKind::Miter, 10.0, CapKind::Butt, &OffsetOptions::REFERENCE);
        cmp(if src.first().is_some_and(|c| c.closed) { "expand geschlossen" } else { "expand offen" }, &contours(&e["fw"]), &got);
    }
    let u = &t["union"];
    let a = Operand::new(contours(&u["a"]), true, PathOp::Normal);
    let b = Operand::new(contours(&u["b"]), true, PathOp::Normal);
    let got = union_paths(&[a, b]).unwrap_or_default();
    cmp("union", &contours(&u["fw"]), &got);
    let th = &t["touch_hole"];
    let punch = Operand::new(contours(&th["punch"]), true, PathOp::Normal);
    let target = Operand::new(contours(&th["target"]), true, PathOp::Normal);
    let got = punch_paths(&[punch, target]).unwrap_or_default();
    for fw in th["fw"].as_array().cloned().unwrap_or_default() {
        cmp("punch (touch_hole)", &contours(&fw), got.first().map(|v| v.as_slice()).unwrap_or(&[]));
    }
    println!("T16 Geometrie gesamt: {ok} von {n} Konturen mit gleicher Knotenfolge");
    assert_eq!(ok, n);
    // Verbinden: Die gespeicherten GUI-Dateien enthalten den Zustand vor dem Verbinden; das Ergebnis
    // steht im JSF-Protokoll (16 §5.1): (b) 180,80 → 110,80 → 90,50 → 20,50; (c) 110,80 → 180,80 → 20,50 → 90,50.
    let vor = t["join_vor"].as_array().cloned().unwrap_or_default();
    let mut join_ok = 0;
    if vor.len() == 2 {
        let p0 = contours(&vor[0]);
        let p1 = contours(&vor[1]);
        // A = (20,50)→(90,50) unten, B = (110,80)→(180,80) oben (zuletzt gezeichnet)
        let (a, b) = if p0[0].nodes[0].anchor.y < p1[0].nodes[0].anchor.y { (p0, p1) } else { (p1, p0) };
        let expect_b = [(180.0, 80.0), (110.0, 80.0), (90.0, 50.0), (20.0, 50.0)];
        let expect_c = [(110.0, 80.0), (180.0, 80.0), (20.0, 50.0), (90.0, 50.0)];
        for (name, ends, exp) in [("(b)", [End::First, End::Last], expect_b), ("(c)", [End::Last, End::First], expect_c)] {
            if let JoinResult::Connected(c) = join_paths(&[b.clone(), a.clone()], Some(ends)) {
                let exact = anchors3(&c) == exp;
                println!("Verbinden {name}: Knotenfolge wie Protokoll (Start, Richtung): {exact}");
                join_ok += exact as usize;
            }
        }
        if let JoinResult::Compound(cs) = join_paths(&[b, a], None) {
            println!("Verbinden (a) Zeigerauswahl: zusammengesetzter Pfad mit {} offenen Konturen", cs.len());
            join_ok += (cs.len() == 2 && cs.iter().all(|c| !c.closed)) as usize;
        }
    }
    println!("Verbinden: {join_ok} von 3 Fällen wie das Original");
    assert_eq!(join_ok, 3);
}

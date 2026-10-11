//! Korpustests (`#[ignore]`): Pfad-Rendering gegen die Render-Caches des Originals, Messweise wie
//! `re\scripts\vektor\test_paths.py 40000 all` (alle Pfade der Wurzel 0 mit Cache, ohne sichtbare
//! Effekte, ≤ 40.000 Pixel) und `test_tex.py` (texturierte Füllungen).
//!
//! Eingabe: `tools\golden\fp-vector\gen_path_corpus.py` → `tools\golden\fp-vector\out\path_corpus.jsonl`
//! (oder `FP_PATH_CORPUS`). Aufruf: `cargo test -p fp-vector --release --test corpus -- --ignored --nocapture`

mod common;

use std::collections::BTreeMap;

use common::*;
use fp_geom::IRect;
use serde_json::Value;

fn corpus_path() -> String {
    std::env::var("FP_PATH_CORPUS").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/golden/fp-vector/out/path_corpus.jsonl").into())
}

#[derive(Default)]
struct Cat {
    cases: usize,
    exact: usize,
    px: usize,
    same: usize,
    d1: usize,
    d8: usize,
    py_same: usize,
}

type Cats = BTreeMap<(String, String), Cat>;

fn run(as_f32: bool, set: &str) -> Option<(Cats, Vec<String>)> {
    let path = corpus_path();
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!("Korpusdatei fehlt: {path} (gen_path_corpus.py ausführen) – Test übersprungen");
        return None;
    };
    let mut cats: Cats = BTreeMap::new();
    let mut fails = Vec::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let rec: Value = serde_json::from_str(line).expect("JSON-Zeile");
        if rec["set"].as_str() != Some(set) {
            continue;
        }
        let win = &rec["win"];
        let g = |i: usize| win[i].as_i64().unwrap_or(0) as i32;
        let rect = IRect::from_xywh(g(0), g(1), g(2), g(3));
        let extra = Extra {
            texture: rec.get("texture").and_then(mask_from),
            pattern: rec.get("pattern").and_then(image_from),
            stroke_texture: None,
        };
        let p = vector_path(Tags::from(&rec["pth"]), &extra, as_f32);
        let (px, _missing) = fp_vector::render_path_pixels(&p, rect);
        let cache = b64(rec["cache"].as_str().unwrap_or(""));
        let py = match rec["py"].as_str() {
            Some(s) => b64(s),
            None => cache.clone(),
        };
        let (same, d1, d8, n) = compare(&px, &cache);
        let (pys, _, _, _) = compare(&px, &py);
        let cat = rec["cat"].as_array().cloned().unwrap_or_default();
        let key = (cat.first().and_then(|v| v.as_str()).unwrap_or("").to_string(), cat.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string());
        let c = cats.entry(key.clone()).or_default();
        c.cases += 1;
        c.exact += (same == n) as usize;
        c.px += n;
        c.same += same;
        c.d1 += d1;
        c.d8 += d8;
        c.py_same += (pys == n) as usize;
        if pys != n && fails.len() < 40 {
            fails.push(format!("{} {:?} {:?}: {} von {} Pixeln wie Python", rec["name"], key.0, key.1, pys, n));
        }
    }
    Some((cats, fails))
}

fn report(title: &str, cats: &Cats) -> (usize, usize, usize) {
    println!("{title}");
    let (mut t, mut e, mut py) = (0, 0, 0);
    let (mut tp, mut ts) = (0usize, 0usize);
    let mut rows: Vec<_> = cats.iter().collect();
    rows.sort_by_key(|(_, c)| std::cmp::Reverse(c.cases));
    for (k, c) in rows {
        println!(
            "{:<28} {:<34} {:>4} Fälle, {:>4} exakt | Pixel {:>7}: exakt {:.4}  ±1 {:.4}  ±8 {:.4} | wie Python {}",
            k.0,
            k.1,
            c.cases,
            c.exact,
            c.px,
            c.same as f64 / c.px.max(1) as f64,
            c.d1 as f64 / c.px.max(1) as f64,
            c.d8 as f64 / c.px.max(1) as f64,
            c.py_same
        );
        t += c.cases;
        e += c.exact;
        py += c.py_same;
        tp += c.px;
        ts += c.same;
    }
    println!("GESAMT {t} Fälle, {e} pixelgenau, {ts}/{tp} Pixel exakt, {py} bitgleich zur Python-Referenz");
    (t, e, py)
}

#[test]
#[ignore]
fn paths_against_render_caches() {
    let Some((cats, fails)) = run(false, "paths") else { return };
    let (t, e, py) = report("Pfade gegen Render-Cache (Koordinaten als double wie die Referenz):", &cats);
    for f in &fails {
        println!("  abweichend von Python: {f}");
    }
    assert_eq!(py, t, "Rust muss bitgleich zur Python-Referenz sein");
    assert!(e >= 1687, "erwartet mindestens 1.687 pixelgenaue Pfade");
}

#[test]
#[ignore]
fn paths_with_f32_coordinates() {
    let Some((cats, _)) = run(true, "paths") else { return };
    report("Pfade gegen Render-Cache (Koordinaten als f32 wie das Modell):", &cats);
}

#[test]
#[ignore]
fn textured_fills() {
    let Some((cats, fails)) = run(false, "tex") else { return };
    let (t, e, _) = report("Texturierte Füllungen (test_tex.py):", &cats);
    for f in &fails {
        println!("  {f}");
    }
    assert_eq!(e, t);
}

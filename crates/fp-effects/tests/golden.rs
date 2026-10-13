//! Golden-Tests gegen die Python-Referenzen `re\effekte\fx.py`, `bevel.py`, `psfx.py`
//! (Daten: `tools\golden\fp-effects\gen_golden.py` → `tests\data\golden`). Pixelgenau, alle vier
//! Kanäle; beide Pixel transparent gilt als gleich (wie die Prüfskripte von 05).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use fp_effects::params::{EffectParams, ParamValue};
use fp_effects::{apply_chain, EffectContext, EffectEntry, LiveEffect};
use fp_raster::{IRect, RgbaImage};
use serde_json::Value;

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("data").join("golden")
}

fn load_png(p: &Path, x: i32, y: i32) -> RgbaImage {
    let dec = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(p).expect("png")));
    let mut r = dec.read_info().expect("png info");
    let mut buf = vec![0; r.output_buffer_size().expect("size")];
    let info = r.next_frame(&mut buf).expect("frame");
    assert_eq!(info.color_type, png::ColorType::Rgba);
    let bytes = &buf[..info.buffer_size()];
    RgbaImage::from_rgba_bytes(IRect::from_xywh(x, y, info.width as i32, info.height as i32), bytes).expect("img")
}

fn origin(v: &Value, k: &str) -> (i32, i32) {
    (v[k][0].as_i64().unwrap_or(0) as i32, v[k][1].as_i64().unwrap_or(0) as i32)
}

/// (gleiche Pixel, verglichene Pixel, größte Abweichung) über die Vereinigung beider Rahmen.
fn compare(a: &RgbaImage, b: &RgbaImage) -> (u64, u64, u8) {
    let r = a.bounds().union(&b.bounds());
    let c = a.compare(b, r);
    (c.equal, c.total, c.max_diff)
}

#[test]
fn golden_against_python_reference() {
    let dir = data_dir();
    let text = std::fs::read_to_string(dir.join("manifest.json")).expect("manifest.json (gen_golden.py ausführen)");
    let cases: Vec<Value> = serde_json::from_str(&text).expect("json");
    assert!(cases.len() > 250);
    let mut per_kind: BTreeMap<String, (u64, u64, usize, usize)> = BTreeMap::new();
    let mut failures = Vec::new();
    for c in &cases {
        let name = c["name"].as_str().unwrap_or("?");
        // PS-Schlagschatten mit Überfüllen: die Python-Referenz (A) überfüllt vor dem Weichzeichnen;
        // Laufzeit T4 (Überfüllen 50 %) zeigt Weichzeichnen mit voller Größe und danach die
        // Kennlinie m·100/(100 − Überfüllen) – Firepixel folgt der Laufzeit (ps_fit.rs, Fehler 33,8 → 2,0).
        if name.starts_with("ps_drop") && name.split('_').any(|p| p.len() > 1 && p.starts_with('c') && p[1..].chars().all(|c| c.is_ascii_digit()) && p != "c0") {
            continue;
        }
        // Glühen mit Versatz > 0: die Python-Referenz (A) beginnt das Band bei Abstand ≥ Versatz; die
        // Messreihe des Originals (re\23 Gruppe 1, „halo“/„gross“) zeigt Abstand > Versatz – Firepixel
        // folgt dem Original (FX-1, Prüfung in tests\fx1_parity.rs).
        if name.starts_with("glow_") && name.split('_').any(|p| p.starts_with("st") && p != "st0" && p[2..].chars().all(|c| c.is_ascii_digit())) {
            continue;
        }
        let (ix, iy) = origin(c, "input_origin");
        let (ox, oy) = origin(c, "output_origin");
        let input = load_png(&dir.join(c["input"].as_str().unwrap_or("")), ix, iy);
        let expect = load_png(&dir.join(c["output"].as_str().unwrap_or("")), ox, oy);
        let entries: Vec<EffectEntry> = c["effects"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|e| {
                        let params: EffectParams = e["params"]
                            .as_object()
                            .map(|o| o.iter().map(|(k, v)| (k.clone(), ParamValue::Text(v.as_str().unwrap_or("").to_string()))).collect())
                            .unwrap_or_default();
                        EffectEntry { effect: LiveEffect::parse(e["moa_id"].as_str().unwrap_or(""), &params), visible: true }
                    })
                    .collect()
            })
            .unwrap_or_default();
        let ctx = EffectContext { legacy_cs5: c["legacy_cs5"].as_bool().unwrap_or(false), ..EffectContext::default() };
        let got = apply_chain(&input, &entries, &ctx);
        assert!(got.issues.is_empty(), "{name}: {:?}", got.issues);
        let (eq, tot, mx) = compare(&got.image, &expect);
        let kind = name.split('_').next().unwrap_or("?").to_string();
        let e = per_kind.entry(kind).or_default();
        e.0 += eq;
        e.1 += tot;
        e.2 += 1;
        if eq == tot {
            e.3 += 1;
        } else {
            failures.push(format!("{name}: {eq}/{tot} max {mx}"));
        }
    }
    for (k, (eq, tot, n, ok)) in &per_kind {
        println!("{k:8} Fälle {ok}/{n} exakt, Pixel {eq}/{tot}");
    }
    for f in &failures {
        println!("ABWEICHUNG {f}");
    }
    assert!(failures.is_empty(), "{} von {} Fällen weichen ab", failures.len(), cases.len());
}

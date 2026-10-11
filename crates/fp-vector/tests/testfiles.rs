//! Laufzeit-Testdateien `re\_testdateien\07` (`#[ignore]`): T2 natürliche Pinsel, T5 Farb-Alpha,
//! T6 Verlaufsformen/weiche Kante/Konturverlauf, T7 `BFBi = 2`. Seite wie `vektor/page.py`: Pfade in
//! Zeichenreihenfolge mit Deckkraft (MMX-over) bzw. Mischmodus, Hintergrund zuletzt unterlegt;
//! Vergleich mit dem flachen IDAT (Original) und mit der Python-Referenz.
//!
//! Daten: `tools\golden\fp-vector\gen_testfiles.py` → `tools\golden\fp-vector\out\testfiles.json`
//! (oder `FP_TESTFILES`). Aufruf: `cargo test -p fp-vector --release --test testfiles -- --ignored --nocapture`

mod common;

use common::*;
use fp_color::blend::{blend_pixel, over_mmx, OverVariant};
use fp_color::{BlendMode, Opacity, Rgba8};
use fp_geom::IRect;
use fp_vector::render_bounds;
use serde_json::Value;

fn load() -> Option<Vec<Value>> {
    let path = std::env::var("FP_TESTFILES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/golden/fp-vector/out/testfiles.json").into());
    match std::fs::read_to_string(&path) {
        Ok(t) => serde_json::from_str::<Value>(&t).ok().and_then(|v| v.as_array().cloned()),
        Err(_) => {
            eprintln!("Datei fehlt: {path} (gen_testfiles.py ausführen) – Test übersprungen");
            None
        }
    }
}

/// Seite rendern (ohne Hintergrund), dann Hintergrund zuletzt unterlegen.
fn render_page(f: &Value, as_f32: bool) -> Vec<[u8; 4]> {
    let w = f["w"].as_i64().unwrap_or(0) as i32;
    let h = f["h"].as_i64().unwrap_or(0) as i32;
    let ox = f["xlc"].as_f64().unwrap_or(0.0).floor() as i32;
    let oy = f["ylc"].as_f64().unwrap_or(0.0).floor() as i32;
    let canvas = IRect::from_xywh(ox, oy, w, h);
    let mut cv = vec![Rgba8::TRANSPARENT; (w * h) as usize];
    for e in f["elements"].as_array().cloned().unwrap_or_default() {
        let extra = Extra {
            texture: e.get("texture").and_then(mask_from),
            pattern: None,
            stroke_texture: e.get("stroke_texture").and_then(mask_from),
        };
        let p = vector_path(Tags::from(&e["pth"]), &extra, as_f32);
        let Some(rb) = render_bounds(&p) else { continue };
        let rect = rb.intersect(&canvas);
        if rect.is_empty() {
            continue;
        }
        let (px, _) = fp_vector::render_path_pixels(&p, rect);
        let op = e["opa"].as_u64().map(|v| Opacity::from_permille(v as u32).0).unwrap_or(255);
        let bld = e["bld"].as_u64().unwrap_or(0) as u8;
        let rw = rect.width() as usize;
        for (k, s) in px.iter().enumerate() {
            if s[3] == 0 {
                continue;
            }
            let x = rect.left + (k % rw) as i32 - ox;
            let y = rect.top + (k / rw) as i32 - oy;
            let d = &mut cv[(y * w + x) as usize];
            if bld == 0 || bld == 55 {
                let a = if op != 255 { fp_color::blend::mul(s[3], op) } else { s[3] };
                *d = over_mmx(*d, [s[0], s[1], s[2]], a);
            } else {
                *d = blend_pixel(*d, Rgba8::new(s[0], s[1], s[2], s[3]), BlendMode(bld), op, 255, OverVariant::Mmx);
            }
        }
    }
    let bgc = f["bgc"].as_u64().unwrap_or(0) as u32;
    let bg = Rgba8::new((bgc >> 16) as u8, (bgc >> 8) as u8, bgc as u8, (bgc >> 24) as u8);
    cv.iter()
        .map(|p| {
            let o = if bg.a != 0 { if p.a != 0 { over_mmx(bg, [p.r, p.g, p.b], p.a) } else { bg } } else { *p };
            [o.r, o.g, o.b, o.a]
        })
        .collect()
}

fn diff_stats(a: &[[u8; 4]], b: &[u8]) -> (usize, usize, usize, i32) {
    let (same, d1, d8, n) = compare(a, b);
    let mut mx = 0;
    for k in 0..n {
        let (p, q) = (a[k], &b[k * 4..k * 4 + 4]);
        if p[3] == 0 && q[3] == 0 {
            continue;
        }
        mx = mx.max((0..4).map(|c| (p[c] as i32 - q[c] as i32).abs()).max().unwrap_or(0));
    }
    let _ = (d1, d8);
    (same, d8, n, mx)
}

fn write_png(path: &std::path::Path, w: u32, h: u32, rgba: &[u8]) {
    let Ok(file) = std::fs::File::create(path) else { return };
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    if let Ok(mut wr) = enc.write_header() {
        let _ = wr.write_image_data(rgba);
    }
}

/// Hilfe für die Analyse: `FP_DUMP_DIR` und `FP_DUMP_FILTER` (Teil des Dateinamens) setzen; schreibt
/// je Datei `<name>_rust.png`, `<name>_idat.png` und ein Differenzbild (rot = nur Original, grün = nur Rust).
#[test]
#[ignore]
fn dump_testfiles() {
    let (Ok(dir), Some(files)) = (std::env::var("FP_DUMP_DIR"), load()) else { return };
    let filt = std::env::var("FP_DUMP_FILTER").unwrap_or_default();
    for f in files.iter().filter(|f| f["name"].as_str().unwrap_or("").contains(&filt)) {
        let name = f["name"].as_str().unwrap_or("x").replace(".fw.png", "");
        let (w, h) = (f["w"].as_u64().unwrap_or(0) as u32, f["h"].as_u64().unwrap_or(0) as u32);
        let got = render_page(f, false);
        let flat: Vec<u8> = got.iter().flatten().copied().collect();
        let idat = b64(f["idat"].as_str().unwrap_or(""));
        let d = std::path::Path::new(&dir);
        write_png(&d.join(format!("{name}_rust.png")), w, h, &flat);
        write_png(&d.join(format!("{name}_idat.png")), w, h, &idat);
        let _ = std::fs::write(d.join(format!("{name}_rust.rgba")), &flat);
        let _ = std::fs::write(d.join(format!("{name}_idat.rgba")), &idat);
        let mut diff = vec![0u8; flat.len()];
        for k in 0..(w * h) as usize {
            // Deckung als „Tinte“ auf Weiß: 255 − Luminanz des über Weiß gelegten Pixels
            let lum = |p: &[u8]| {
                let a = p[3] as u32;
                let c = |v: u8| (v as u32 * a + 255 * (255 - a)) / 255;
                255 - (c(p[0]) * 299 + c(p[1]) * 587 + c(p[2]) * 114) as i32 / 1000
            };
            let a = lum(&idat[k * 4..k * 4 + 4]);
            let b = lum(&flat[k * 4..k * 4 + 4]);
            diff[k * 4] = (a - b).clamp(0, 255) as u8 * 2;
            diff[k * 4 + 1] = (b - a).clamp(0, 255) as u8 * 2;
            diff[k * 4 + 2] = (a.min(b) / 3) as u8;
            diff[k * 4 + 3] = 255;
        }
        write_png(&d.join(format!("{name}_diff.png")), w, h, &diff);
    }
}

#[test]
#[ignore]
fn testfiles_07() {
    let Some(files) = load() else { return };
    let mut totals = std::collections::BTreeMap::<String, (usize, usize, usize, usize, usize)>::new();
    let mut region_totals = std::collections::BTreeMap::<String, (usize, usize, usize, usize)>::new();
    for f in &files {
        let name = f["name"].as_str().unwrap_or("").to_string();
        let idat = b64(f["idat"].as_str().unwrap_or(""));
        let py = b64(f["python"].as_str().unwrap_or(""));
        let got = render_page(f, false);
        let (same, d8, n, mx) = diff_stats(&got, &idat);
        let (py_same, _, _, _) = diff_stats(&got, &py);
        let (pyf, _, _, _) = {
            let pyv: Vec<[u8; 4]> = py.as_chunks::<4>().0.to_vec();
            diff_stats(&pyv, &idat)
        };
        let got32 = render_page(f, true);
        let (same32, _, _, _) = diff_stats(&got32, &idat);
        // Strichbereich: Pixel, die im Original oder Rust vom Hintergrund abweichen
        let bgc = f["bgc"].as_u64().unwrap_or(0) as u32;
        let bgp = [(bgc >> 16) as u8, (bgc >> 8) as u8, bgc as u8, (bgc >> 24) as u8];
        let (mut reg, mut reg_same, mut reg_d8, mut fw_only, mut rust_only) = (0, 0, 0, 0, 0);
        for k in 0..n {
            let q = [idat[k * 4], idat[k * 4 + 1], idat[k * 4 + 2], idat[k * 4 + 3]];
            let p = got[k];
            let ink = |x: [u8; 4]| if bgp[3] == 0 { x[3] != 0 } else { x != bgp };
            let (fw_ink, rs_ink) = (ink(q), ink(p));
            if fw_ink || rs_ink {
                reg += 1;
                let d = (0..4).map(|c| (p[c] as i32 - q[c] as i32).abs()).max().unwrap_or(0);
                reg_same += (d == 0) as usize;
                reg_d8 += (d <= 8) as usize;
                fw_only += (fw_ink && !rs_ink) as usize;
                rust_only += (rs_ink && !fw_ink) as usize;
            }
        }
        if name.starts_with("t2") {
            println!(
                "   Strichbereich {reg:>6} px: exakt {reg_same:>6} ({:.1} %), ±8 {reg_d8:>6} ({:.1} %), nur Original {fw_only}, nur Rust {rust_only}",
                100.0 * reg_same as f64 / reg.max(1) as f64,
                100.0 * reg_d8 as f64 / reg.max(1) as f64
            );
            let t = region_totals.entry("t2".into()).or_default();
            t.0 += reg;
            t.1 += reg_same;
            t.2 += reg_d8;
            t.3 += fw_only + rust_only;
        }
        println!(
            "{name:<38} Rust {same:>6}/{n:<6} (±8 {d8:>6}, max {mx:>3}) | f32 {same32:>6} | Python {pyf:>6} | Rust=Python {py_same:>6} | fehlt Python {:?}",
            f["missing"]
        );
        let cat = name.split('_').next().unwrap_or("").to_string();
        let t = totals.entry(cat).or_default();
        t.0 += 1;
        t.1 += (same == n) as usize;
        t.2 += same;
        t.3 += n;
        t.4 += pyf;
    }
    for (k, (files, exact, same, n, py)) in totals {
        println!("{k}: {exact} von {files} Dateien pixelgenau, Pixel {same}/{n} ({:.2} %), Python {py}/{n}", 100.0 * same as f64 / n.max(1) as f64);
    }
    for (k, (reg, same, d8, xor)) in region_totals {
        println!(
            "{k} Strichbereich: {same}/{reg} exakt ({:.1} %), ±8 {d8} ({:.1} %), Deckungsunterschied {xor} px",
            100.0 * same as f64 / reg.max(1) as f64,
            100.0 * d8 as f64 / reg.max(1) as f64
        );
    }
}

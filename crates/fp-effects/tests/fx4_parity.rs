//! FX-4: Gruppe 4 der Effekt-Messreihe (re\23 „Farbe anpassen, Rauschen, Sonstige“) gegen die
//! Bilder des Originals. Eingabe = Objektpuffer aus dem Export ohne Wirkung (Vektor 75,55–165,125
//! mit weißen durchsichtigen Pixeln, Text 23,46–167,112 schwarz, Bitmap 72,53–168,127 = Alpha-Grenzen
//! mit den gespeicherten Farben durchsichtiger Pixel), Erwartung = Ausschnitt
//! des Original-Exports (`tests\data\fx4`, erzeugt aus `re\_testdateien\par\fx\4-farbe-rauschen-sonstige`).
//!
//! Geprüft wird je Fall der Live-Effekt (fp-effects) und – für dieselben Parameter – der Filter
//! (`fp_raster::Filter`, Menübefehl „Filter“): Ziel höchstens 3 Stufen je Kanal (Farbe dort, wo
//! beide Alpha ≥ 16 haben), Rauschen zusätzlich statistisch (Standardabweichung).

use std::path::{Path, PathBuf};

use fp_effects::params::{EffectParams, ParamValue};
use fp_effects::{EffectContext, EffectEntry, LiveEffect, apply_chain};
use fp_raster::filters::PixelFilter;
use fp_raster::{Filter, IRect, RgbaImage};
use serde_json::Value;

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("fx4")
}

fn load_png(p: &Path, x: i32, y: i32) -> RgbaImage {
    let dec = png::Decoder::new(std::io::BufReader::new(
        std::fs::File::open(p).expect("png"),
    ));
    let mut r = dec.read_info().expect("png info");
    let mut buf = vec![0; r.output_buffer_size().expect("size")];
    let info = r.next_frame(&mut buf).expect("frame");
    assert_eq!(info.color_type, png::ColorType::Rgba, "{}", p.display());
    RgbaImage::from_rgba_bytes(
        IRect::from_xywh(x, y, info.width as i32, info.height as i32),
        &buf[..info.buffer_size()],
    )
    .expect("img")
}

fn params_of(e: &Value) -> EffectParams {
    let mut p: EffectParams = e["params"]
        .as_object()
        .map(|o| {
            o.iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        ParamValue::Text(v.as_str().unwrap_or("").to_string()),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    if let Some(pts) = e.get("points").and_then(|v| v.as_object()) {
        for (k, v) in pts {
            let list = v
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|q| {
                            (
                                q[0].as_f64().unwrap_or(0.0) as f32,
                                q[1].as_f64().unwrap_or(0.0) as f32,
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();
            p.insert(k.clone(), ParamValue::Points(list));
        }
    }
    p
}

/// (größte Alpha-Abweichung, größte Farbabweichung bei Alpha ≥ 16, Pixel über 3 Stufen).
fn diff(a: &RgbaImage, b: &RgbaImage) -> (u8, u8, usize) {
    assert_eq!(a.bounds(), b.bounds());
    let (mut am, mut cm, mut n) = (0u8, 0u8, 0usize);
    for (p, q) in a.data().iter().zip(b.data()) {
        let da = p[3].abs_diff(q[3]);
        let dc = if p[3] >= 16 && q[3] >= 16 {
            (0..3).map(|c| p[c].abs_diff(q[c])).max().unwrap_or(0)
        } else {
            0
        };
        am = am.max(da);
        cm = cm.max(dc);
        n += usize::from(da > 3 || dc > 3);
    }
    (am, cm, n)
}

fn std_dev(img: &RgbaImage) -> f64 {
    let v: Vec<f64> = img
        .data()
        .iter()
        .filter(|p| p[3] >= 16)
        .flat_map(|p| [p[0] as f64, p[1] as f64, p[2] as f64])
        .collect();
    let m = v.iter().sum::<f64>() / v.len().max(1) as f64;
    (v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / v.len().max(1) as f64).sqrt()
}

#[test]
fn group4_matches_reference() {
    let dir = data_dir();
    let cases: Vec<Value> =
        serde_json::from_str(&std::fs::read_to_string(dir.join("cases.json")).expect("cases.json"))
            .expect("json");
    assert_eq!(cases.len(), 54);
    let mut bad = Vec::new();
    for c in &cases {
        let name = c["name"].as_str().unwrap_or("?");
        let (x, y) = (
            c["origin"][0].as_i64().unwrap_or(0) as i32,
            c["origin"][1].as_i64().unwrap_or(0) as i32,
        );
        let input = load_png(&dir.join(c["input"].as_str().unwrap_or("")), x, y);
        let expect = load_png(&dir.join(c["output"].as_str().unwrap_or("")), x, y);
        let effects: Vec<(String, EffectParams)> = c["effects"]
            .as_array()
            .expect("effects")
            .iter()
            .map(|e| (e["moa_id"].as_str().unwrap_or("").to_string(), params_of(e)))
            .collect();
        let entries: Vec<EffectEntry> = effects
            .iter()
            .map(|(m, p)| EffectEntry {
                effect: LiveEffect::parse(m, p),
                visible: true,
            })
            .collect();
        let live = apply_chain(&input, &entries, &EffectContext::default());
        assert!(live.issues.is_empty(), "{name}: {:?}", live.issues);
        let live = live.image.cropped(input.bounds());
        let (am, cm, n) = diff(&live, &expect);
        println!("{name:44} live  A {am:3} RGB {cm:3} >3: {n}");
        if am > 3 || cm > 3 {
            bad.push(format!("{name} live: A {am} RGB {cm} ({n} px)"));
        }
        // derselbe Effekt als Filter (Menü „Filter“ bzw. Batch): gleiches Ergebnis wie live
        let mut f_img = input.clone();
        for (m, p) in &effects {
            let f =
                Filter::from_effect(m, p).unwrap_or_else(|| panic!("{name}: kein Filter für {m}"));
            f_img = f.apply(&f_img, None);
        }
        let f_img = f_img.cropped(input.bounds());
        let (fa, fc, fnn) = diff(&f_img, &expect);
        if fa > 3 || fc > 3 {
            bad.push(format!("{name} filter: A {fa} RGB {fc} ({fnn} px)"));
        }
        if name.starts_with("rauschen") {
            let (s0, s1) = (std_dev(&expect), std_dev(&live));
            assert!((s0 - s1).abs() < 0.5, "{name}: Streuung {s0:.2} ↔ {s1:.2}");
        }
    }
    for b in &bad {
        println!("ABWEICHUNG {b}");
    }
    assert!(bad.is_empty(), "{} Abweichungen", bad.len());
}

/// re\23: Fehlt im Original eine der vier Punktlisten, schlägt das Rendern fehl. Firepixel rechnet
/// weiter (fehlende Liste = Identität) und liefert nie einen Fehler.
#[test]
fn curves_missing_point_list_does_not_fail() {
    let input = RgbaImage::from_raw(
        IRect::new(0, 0, 2, 1),
        vec![[10, 128, 250, 255], [200, 50, 0, 128]],
    )
    .unwrap();
    let mut p = EffectParams::new();
    p.insert(
        "rgb_points".into(),
        ParamValue::Points(vec![(0.0, 0.0), (128.0, 145.0), (255.0, 255.0)]),
    );
    let fx = LiveEffect::parse("{3439b08e-1923-11d3-9bde00e02910d580}", &p);
    let out = apply_chain(
        &input,
        &[EffectEntry {
            effect: fx,
            visible: true,
        }],
        &EffectContext::default(),
    );
    assert!(out.issues.is_empty());
    assert_eq!(out.image.pixel(0, 0).g, 145);
    // ganz ohne Listen: unverändert
    let fx = LiveEffect::parse(
        "{3439b08e-1923-11d3-9bde00e02910d580}",
        &EffectParams::new(),
    );
    let out = apply_chain(
        &input,
        &[EffectEntry {
            effect: fx,
            visible: true,
        }],
        &EffectContext::default(),
    );
    assert_eq!(out.image, input);
}

/// re\23: Auto-Tonwert macht einfarbige Objekte schwarz (unten = oben je Kanal).
#[test]
fn auto_levels_single_colour_turns_black() {
    let input = RgbaImage::filled(
        IRect::new(5, 5, 15, 12),
        fp_color::Rgba8::new(58, 123, 213, 255),
    )
    .unwrap();
    let out = Filter::AutoLevels.apply(&input, None);
    assert!(out.data().iter().all(|p| *p == [0, 0, 0, 255]));
}

/// Farbfüllung: halbtransparente Pixel bekommen das Gewicht `DIV[a][…]` (gemessen 20 %, Alpha 2 → fast
/// volle Füllfarbe); Live-Effekt und Filter rechnen gleich.
#[test]
fn color_fill_soft_edges_live_equals_filter() {
    let data: Vec<[u8; 4]> = (0..=255u16)
        .step_by(5)
        .map(|a| [38, 251, 68, a as u8])
        .collect();
    let input = RgbaImage::from_raw(IRect::new(0, 0, data.len() as i32, 1), data).unwrap();
    let mut p = EffectParams::new();
    for (k, v) in [("Color", "#0000ff"), ("Opacity", "20"), ("Blendmode", "0")] {
        p.insert(k.into(), ParamValue::Text(v.into()));
    }
    let live = apply_chain(
        &input,
        &[EffectEntry {
            effect: LiveEffect::parse("{dd54adc0-a279-11d3-b92a000502f3fdbe}", &p),
            visible: true,
        }],
        &EffectContext::default(),
    )
    .image;
    let filt = Filter::from_effect("{dd54adc0-a279-11d3-b92a000502f3fdbe}", &p)
        .unwrap()
        .apply(&input, None);
    assert_eq!(live, filt);
    // Messpunkt des Originals: (38, 251, 68, 2) → (1, 10, 248, 2) ± 3
    let q = fp_raster::filters::ColorFillParams {
        color: fp_color::Rgba8::rgb(0, 0, 255),
        opacity: 20,
        blend_mode: fp_color::BlendMode(0),
    }
    .apply_px([38, 251, 68, 2]);
    assert!(q[0] <= 4 && q[1] <= 13 && q[2] >= 245, "{q:?}");
}

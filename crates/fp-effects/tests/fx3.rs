//! FX-3: Weichzeichnen/Schärfen gegen die Messreihe des Originals (`re\23-effekte.md`, Gruppe 3).
//!
//! Eingabe ist das Objekt so, wie das Original es ohne Wirkung exportiert (Fall „radial klein“,
//! Menge 1/Qualität 1 = keine Änderung), bzw. für die Bitmap das Quellbild selbst. Der Effekt
//! läuft über [`fp_effects::apply_chain`], das Ergebnis wird auf die transparente Leinwand
//! 240 × 180 gesetzt und pixelweise mit dem PNG-32-Export des Originals verglichen.
//!
//! - `golden_fx3`: eingecheckte Referenzen unter `tests\data\fx3` (je Effekt Vorgabe + Extremfall).
//! - `sweep_fx3` (`#[ignore]`): alle 72 Bilder aus `FP_FX3_REF` (Ordner der Messreihe), schreibt
//!   die Ergebnisse nach `FP_FX3_OUT` und druckt eine Tabelle.

use std::path::{Path, PathBuf};

use fp_effects::params::{EffectParams, ParamValue};
use fp_effects::{EffectContext, EffectEntry, LiveEffect, apply_chain};
use fp_raster::{IRect, RgbaImage};
use serde_json::Value;

const CANVAS: IRect = IRect {
    left: 0,
    top: 0,
    right: 240,
    bottom: 180,
};

fn load_png(p: &Path) -> RgbaImage {
    let dec = png::Decoder::new(std::io::BufReader::new(
        std::fs::File::open(p).unwrap_or_else(|e| panic!("{}: {e}", p.display())),
    ));
    let mut r = dec.read_info().expect("png info");
    let mut buf = vec![0; r.output_buffer_size().expect("size")];
    let info = r.next_frame(&mut buf).expect("frame");
    let bytes = &buf[..info.buffer_size()];
    let (w, h) = (info.width as i32, info.height as i32);
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => bytes.to_vec(),
        png::ColorType::Rgb => bytes
            .chunks(3)
            .flat_map(|c| [c[0], c[1], c[2], 255])
            .collect(),
        t => panic!("{}: Farbtyp {t:?}", p.display()),
    };
    RgbaImage::from_rgba_bytes(IRect::from_xywh(0, 0, w, h), &rgba).expect("img")
}

fn save_png(p: &Path, img: &RgbaImage) {
    let f = std::fs::File::create(p).expect("create");
    let mut e = png::Encoder::new(
        std::io::BufWriter::new(f),
        img.width() as u32,
        img.height() as u32,
    );
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    e.write_header()
        .expect("hdr")
        .write_image_data(img.as_bytes())
        .expect("data");
}

/// Objektbild in Dokumentkoordinaten aus der unveränderten Leinwand bzw. dem Bitmap-Quellbild.
fn object_input(obj: &str, plain: &RgbaImage, bitmap_src: Option<&RgbaImage>) -> RgbaImage {
    match obj {
        "bitmap" => {
            let mut b = bitmap_src.expect("bitmap src").clone();
            b.set_origin(70, 50);
            // Das Original rechnet Bitmaps auf ihren Pixelgrenzen (Alpha > 0), nicht auf dem ganzen Bild.
            b.cropped(b.alpha_bounds())
        }
        // Pfadpuffer des Originals: durchsichtige Pixel weiß (so exportiert es auch die Leinwand).
        "vektor" => clear_transparent(
            plain.cropped(IRect::new(75, 55, 165, 125)),
            [255, 255, 255, 0],
        ),
        // Textpuffer: Pixelgrenzen + 15 px (rechts 16), durchsichtig schwarz.
        _ => {
            let p: i32 = std::env::var("FP_FX3_TEXTPAD")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(TEXT_PAD);
            let b = plain.alpha_bounds();
            let r = IRect::new(b.left - p, b.top - p, b.right + p + 1, b.bottom + p);
            clear_transparent(plain.cropped(b).extended_to(r).expect("ext"), [0; 4])
        }
    }
}

fn clear_transparent(mut img: RgbaImage, clear: [u8; 4]) -> RgbaImage {
    for p in img.data_mut() {
        if p[3] == 0 {
            *p = clear;
        }
    }
    img
}

fn effects_from_json(v: &Value) -> Vec<EffectEntry> {
    v.as_array()
        .map(|a| {
            a.iter()
                .map(|e| {
                    let mut p = EffectParams::new();
                    let mut id = String::new();
                    for (k, val) in e.as_object().into_iter().flatten() {
                        match k.as_str() {
                            "EffectMoaID" => id = val.as_str().unwrap_or("").to_string(),
                            "EffectIsVisible" => {}
                            _ => {
                                let s = match val {
                                    Value::String(s) => s.clone(),
                                    other => other.to_string(),
                                };
                                p.insert(k.clone(), ParamValue::Text(s));
                            }
                        }
                    }
                    EffectEntry {
                        effect: LiveEffect::parse(&id, &p),
                        visible: true,
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Ergebnis auf die Leinwand setzen (transparenter Hintergrund, Export ohne Zuschnitt).
fn on_canvas(img: &RgbaImage) -> RgbaImage {
    let mut c = RgbaImage::new(CANVAS).expect("canvas");
    c.paste(&img.cropped(img.bounds().intersect(&CANVAS)));
    c
}

/// (max. Abweichung Alpha, max. Abweichung Farbe bei beidseitig Alpha > 0, Pixel mit Abweichung > 3)
fn diff(a: &RgbaImage, b: &RgbaImage) -> (u8, u8, usize) {
    let (mut ma, mut mc, mut n) = (0u8, 0u8, 0usize);
    for (p, q) in a.data().iter().zip(b.data()) {
        if p[3] == 0 && q[3] == 0 {
            continue;
        }
        let da = p[3].abs_diff(q[3]);
        ma = ma.max(da);
        let mut d = da;
        if p[3] > 0 && q[3] > 0 {
            for c in 0..3 {
                // Farbe gewichtet mit der kleineren Deckkraft (unsichtbare Farbunterschiede zählen nicht voll)
                let dc = (p[c].abs_diff(q[c]) as u32 * p[3].min(q[3]) as u32 / 255) as u8;
                mc = mc.max(dc);
                d = d.max(dc);
            }
        }
        if d > 3 {
            n += 1;
        }
    }
    (ma, mc, n)
}

fn run_case(effects: &[EffectEntry], input: &RgbaImage) -> RgbaImage {
    on_canvas(&apply_chain(input, effects, &EffectContext::default()).image)
}

#[test]
#[ignore]
fn sweep_fx3() {
    let refdir = PathBuf::from(std::env::var("FP_FX3_REF").expect("FP_FX3_REF"));
    let out = std::env::var("FP_FX3_OUT").ok().map(PathBuf::from);
    let j: Value =
        serde_json::from_str(&std::fs::read_to_string(refdir.join("params.json")).expect("params"))
            .expect("json");
    let src = load_png(&refdir.join("..").join("base").join("fx_bitmap_src.png"));
    let only = std::env::var("FP_FX3_ONLY").unwrap_or_default();
    for c in j["cases"].as_array().expect("cases") {
        let (eff, set) = (
            c["effect"].as_str().unwrap_or(""),
            c["setting"].as_str().unwrap_or(""),
        );
        if !only.is_empty() && !format!("{eff}_{set}").contains(&only) {
            continue;
        }
        let entries = effects_from_json(&c["effects"]);
        for obj in ["vektor", "text", "bitmap"] {
            let plain = load_png(&refdir.join(format!("radial_klein_{obj}.png")));
            let input = object_input(obj, &plain, Some(&src));
            let got = run_case(&entries, &input);
            let name = format!("{eff}_{set}_{obj}");
            let want = load_png(&refdir.join(format!("{name}.png")));
            let (ma, mc, n) = diff(&got, &want);
            println!(
                "{name:42} alpha {ma:3} farbe {mc:3} px>3 {n:5}  bbox fp {:?} orig {:?}",
                got.alpha_bounds(),
                want.alpha_bounds()
            );
            if let Some(o) = &out {
                save_png(&o.join(format!("{name}.fp.png")), &got);
            }
        }
    }
}

/// Rand des Textpuffers um die Pixelgrenzen (rechts +1), siehe `fp-render` `effect_input`.
const TEXT_PAD: i32 = 15;

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("fx3")
}

/// Bekannte Restabweichungen (begründet in `docs\berichte\fx3.md`): (Fall, max. Stufen, Pixel > 3).
const KNOWN: &[(&str, u8, usize)] = &[
    // Text-Puffergröße des Originals nur angenähert (inneres Gauß normiert am Pufferrand).
    ("unsharp_large_text", 5, 12),
    // ein Pixel am durchsichtigen Rand, Farbe bei Alpha 2–3
    ("unsharp_large_bitmap", 5, 1),
];

/// Je Effekt Vorgabe + Extremfall an Vektor, Text und Bitmap (54 Bilder), Ziel ≤ 3 Stufen je Kanal.
#[test]
fn golden_fx3() {
    let dir = data_dir();
    let m: Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join("manifest.json")).expect("manifest"),
    )
    .expect("json");
    let src = load_png(&dir.join("bitmap_src.png"));
    let mut bad = Vec::new();
    let mut n = 0;
    for c in m["cases"].as_array().expect("cases") {
        let name = c["name"].as_str().unwrap_or("?");
        let entries = effects_from_json(&c["effects"]);
        assert!(
            entries
                .iter()
                .all(|e| matches!(e.effect, LiveEffect::Filter(_))),
            "{name}: kein Filter erkannt"
        );
        for (obj, file) in [("vektor", "vector"), ("text", "text"), ("bitmap", "bitmap")] {
            let plain = if obj == "bitmap" {
                src.clone()
            } else {
                load_png(&dir.join(format!("plain_{file}.png")))
            };
            let input = object_input(obj, &plain, Some(&src));
            let got = run_case(&entries, &input);
            let want = load_png(&dir.join(format!("{name}_{file}.png")));
            let (ma, mc, px) = diff(&got, &want);
            let case = format!("{name}_{file}");
            let (lim, lim_px) = KNOWN
                .iter()
                .find(|k| k.0 == case)
                .map(|k| (k.1, k.2))
                .unwrap_or((3, 0));
            if ma > lim || mc > lim || px > lim_px {
                bad.push(format!("{case}: alpha {ma} farbe {mc} px>3 {px}"));
            }
            n += 1;
        }
    }
    assert_eq!(n, 54);
    assert!(bad.is_empty(), "{bad:#?}");
}

/// Wiederholung: dieselbe Effektliste mehrfach auf das unveränderte Objekt ergibt dasselbe Bild
/// (die Kette ersetzt, sie addiert nicht); Endwert nach 50 Änderungen = einmal mit dem Endwert.
#[test]
fn chain_is_replaced_not_accumulated() {
    let dir = data_dir();
    let src = load_png(&dir.join("bitmap_src.png"));
    let input = object_input("bitmap", &src, Some(&src));
    let mk = |r: f64| {
        let mut p = EffectParams::new();
        p.insert(
            "gaussian_blur_radius".into(),
            ParamValue::Text(format!("{r}")),
        );
        vec![EffectEntry {
            effect: LiveEffect::parse("{d04ef8c0-71b3-11d1-8c8200a024cdc039}", &p),
            visible: true,
        }]
    };
    let once = run_case(&mk(7.5), &input);
    let mut last = None;
    for i in 0..50 {
        last = Some(run_case(&mk(0.5 + i as f64 * 0.25), &input));
    }
    let fin = run_case(&mk(7.5), &input);
    assert_eq!(once, fin);
    assert_ne!(last.expect("last"), once);
    // unsichtbarer Filter: Objekt unverändert
    let mut hidden = mk(7.5);
    hidden[0].visible = false;
    assert_eq!(
        apply_chain(&input, &hidden, &EffectContext::default()).image,
        input
    );
}

/// Grenzfälle der Parameter laufen ohne Panik und mit den gemessenen Grenzen.
#[test]
fn extreme_parameters() {
    let img = RgbaImage::filled(
        IRect::new(75, 55, 165, 125),
        fp_color::Rgba8::rgb(58, 123, 213),
    )
    .expect("img");
    let run = |id: &str, kv: &[(&str, &str)]| {
        let p: EffectParams = kv
            .iter()
            .map(|(k, v)| (k.to_string(), ParamValue::Text(v.to_string())))
            .collect();
        let e = vec![EffectEntry {
            effect: LiveEffect::parse(id, &p),
            visible: true,
        }];
        apply_chain(&img, &e, &EffectContext::default()).image
    };
    // Gauß 250 (Dialoggrenze) und 210: deckend in der Mittelfarbe, Objektrechteck
    for r in ["210", "250"] {
        let o = run(
            "{d04ef8c0-71b3-11d1-8c8200a024cdc039}",
            &[("gaussian_blur_radius", r)],
        );
        assert_eq!(o.alpha_bounds(), img.bounds());
        assert_eq!(o.pixel(80, 60), fp_color::Rgba8::rgb(58, 123, 213));
    }
    // Bewegung 0°/1: 1 px nach links, Alpha 127; 90°/10: 9 px nach unten
    let o = run(
        "{538016b6-ffd6-418f-a10c590e6ee841ae}",
        &[("motion_blur_angle", "0"), ("motion_blur_distance", "1")],
    );
    assert_eq!(o.alpha_bounds(), IRect::new(74, 55, 165, 125));
    assert_eq!(o.pixel(74, 90).a, 127);
    let o = run(
        "{538016b6-ffd6-418f-a10c590e6ee841ae}",
        &[("motion_blur_angle", "90"), ("motion_blur_distance", "10")],
    );
    assert_eq!(o.alpha_bounds(), IRect::new(75, 55, 165, 134));
    // Radial/Zoom 1/1: keine Wirkung; Zoom 100/100 weit über das Objekt hinaus
    for id in [
        "{d1c33142-2ad8-4215-a39fc5d934c8fc0c}",
        "{94dbd663-9b2e-44bf-8064ab6efdd2d327}",
    ] {
        let k = if id.starts_with("{d1") {
            "radial_blur"
        } else {
            "zoom_blur"
        };
        let o = run(
            id,
            &[
                (&format!("{k}_amount"), "1"),
                (&format!("{k}_quality"), "1"),
            ],
        );
        assert_eq!(o.cropped(img.bounds()), img);
    }
    let o = run(
        "{94dbd663-9b2e-44bf-8064ab6efdd2d327}",
        &[("zoom_blur_amount", "100"), ("zoom_blur_quality", "100")],
    );
    assert_eq!(o.alpha_bounds(), IRect::new(32, 21, 207, 158));
    // Unscharf maskieren 500 %/250/255: deckende Fläche bleibt
    let o = run(
        "{f1cfce44-718e-11d1-8c8200a024cdc039}",
        &[
            ("unsharp_mask_radius", "250"),
            ("unsharp_mask_amount", "500"),
            ("unsharp_mask_threshold", "255"),
        ],
    );
    assert_eq!(o, img);
}

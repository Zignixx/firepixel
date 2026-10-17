//! FX-1: Schatten und Glühen gegen die Messreihe des Originals (`re\23-effekte.md`, Gruppe 1).
//!
//! Eingabe ist das Objektbild aus der Basisdatei (zusammengesetztes Bild der FW-PNG, auf das
//! Alpha-Rechteck = `pixelRect` beschnitten), Ergebnis wird wie der Export des Originals auf die
//! Leinwand 240 × 180 gelegt und mit dem Referenz-PNG verglichen.
//!
//! - `golden_fx1` (immer): Referenzen aus `tests\data\fx1` (Ergebnisbilder, ohne Oberfläche).
//! - `sweep_fx1` (`--ignored`): alle Fälle der Messreihe direkt aus `re\_testdateien\par\fx\`,
//!   mit `FX1_OUT=<ordner>` werden die Firepixel-Bilder dorthin geschrieben.

use std::path::{Path, PathBuf};

use fp_effects::params::{EffectParams, ParamValue};
use fp_effects::{EffectContext, EffectEntry, LiveEffect, apply_chain};
use fp_raster::{IRect, RgbaImage};
use serde_json::Value;

const W: i32 = 240;
const H: i32 = 180;

fn load_png(p: &Path) -> RgbaImage {
    let dec = png::Decoder::new(std::io::BufReader::new(
        std::fs::File::open(p).unwrap_or_else(|e| panic!("{p:?}: {e}")),
    ));
    let mut r = dec.read_info().expect("png info");
    let mut buf = vec![0; r.output_buffer_size().expect("size")];
    let info = r.next_frame(&mut buf).expect("frame");
    assert_eq!(info.color_type, png::ColorType::Rgba, "{p:?}");
    RgbaImage::from_rgba_bytes(
        IRect::from_xywh(0, 0, info.width as i32, info.height as i32),
        &buf[..info.buffer_size()],
    )
    .expect("img")
}

fn save_png(p: &Path, img: &RgbaImage) {
    let b = img.bounds();
    let f = std::fs::File::create(p).expect("create");
    let mut e = png::Encoder::new(
        std::io::BufWriter::new(f),
        b.width() as u32,
        b.height() as u32,
    );
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    let mut w = e.write_header().expect("hdr");
    let bytes: Vec<u8> = img.data().iter().flat_map(|p| *p).collect();
    w.write_image_data(&bytes).expect("data");
}

/// Objektbild aus der Basisdatei: auf das Alpha-Rechteck beschnitten.
fn object(base: &Path) -> RgbaImage {
    let full = load_png(base);
    full.copy_rect(full.alpha_bounds()).expect("crop")
}

fn entries(effects: &Value) -> Vec<EffectEntry> {
    effects
        .as_array()
        .expect("effects")
        .iter()
        .map(|e| {
            let o = e.as_object().expect("obj");
            let params: EffectParams = o
                .iter()
                .filter(|(k, _)| k.as_str() != "EffectMoaID" && k.as_str() != "EffectIsVisible")
                .map(|(k, v)| {
                    let s = match v {
                        Value::String(s) => s.clone(),
                        Value::Bool(b) => b.to_string(),
                        other => other.to_string(),
                    };
                    (k.clone(), ParamValue::Text(s))
                })
                .collect();
            EffectEntry {
                effect: LiveEffect::parse(o["EffectMoaID"].as_str().unwrap_or(""), &params),
                visible: true,
            }
        })
        .collect()
}

/// Kennzahlen: Alpha (max, Mittel), Farbe vormultipliziert (max, Mittel), Pixel mit Abweichung > 3.
#[derive(Debug, Default, Clone, Copy)]
struct Stats {
    a_max: u8,
    a_mean: f64,
    c_max: u8,
    c_mean: f64,
    over3: u32,
}

fn stats(got: &RgbaImage, refi: &RgbaImage) -> Stats {
    let mut s = Stats::default();
    let (mut asum, mut csum, mut n) = (0u64, 0u64, 0u64);
    for y in 0..H {
        for x in 0..W {
            let a = got.pixel(x, y);
            let b = refi.pixel(x, y);
            if a.a == 0 && b.a == 0 {
                continue;
            }
            n += 1;
            let da = a.a.abs_diff(b.a);
            let pm = |c: u8, al: u8| ((c as u32 * al as u32 + 127) / 255) as u8;
            let dc = [
                pm(a.r, a.a).abs_diff(pm(b.r, b.a)),
                pm(a.g, a.a).abs_diff(pm(b.g, b.a)),
                pm(a.b, a.a).abs_diff(pm(b.b, b.a)),
            ];
            let dcm = *dc.iter().max().unwrap_or(&0);
            s.a_max = s.a_max.max(da);
            s.c_max = s.c_max.max(dcm);
            asum += da as u64;
            csum += dcm as u64;
            if da.max(dcm) > 3 {
                s.over3 += 1;
            }
        }
    }
    if n > 0 {
        s.a_mean = asum as f64 / n as f64;
        s.c_mean = csum as f64 / n as f64;
    }
    s
}

fn render(obj: &RgbaImage, fx: &[EffectEntry]) -> RgbaImage {
    let r = apply_chain(obj, fx, &EffectContext::default());
    assert!(r.issues.is_empty(), "{:?}", r.issues);
    r.image.copy_rect(IRect::new(0, 0, W, H)).expect("canvas")
}

fn re_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("re")
        .join("_testdateien")
        .join("par")
        .join("fx")
}

const OBJS: [(&str, &str); 3] = [
    ("vektor", "fx_vector.fw.png"),
    ("text", "fx_text.fw.png"),
    ("bitmap", "fx_bitmap.fw.png"),
];

#[test]
#[ignore = "braucht re\\_testdateien\\par\\fx (Messreihe des Originals)"]
fn sweep_fx1() {
    let root = re_root();
    let grp = root.join("1-schatten-gluehen");
    let pj: Value = serde_json::from_str(
        &std::fs::read_to_string(grp.join("params.json")).expect("params.json"),
    )
    .expect("json");
    let out = std::env::var_os("FX1_OUT").map(PathBuf::from);
    let filter = std::env::var("FX1_FILTER").unwrap_or_default();
    println!(
        "{:<44} {:>5} {:>6} {:>5} {:>6} {:>6}",
        "Fall", "A max", "A mitt", "C max", "C mitt", ">3"
    );
    for c in pj["cases"].as_array().expect("cases") {
        let name = format!(
            "{}_{}",
            c["effect"].as_str().unwrap_or(""),
            c["setting"].as_str().unwrap_or("")
        );
        if !name.contains(&filter) {
            continue;
        }
        let fx = entries(&c["effects"]);
        for (on, base) in OBJS {
            let mut obj = object(&root.join("base").join(base));
            if on == "text" && std::env::var_os("FX1_PAD_TEXT").is_some() {
                obj = obj.copy_rect(obj.bounds().inflate(1, 1)).expect("pad");
            }
            let got = render(&obj, &fx);
            let file = format!("{name}_{on}.png");
            let refi = load_png(&grp.join(&file));
            let s = stats(&got, &refi);
            println!(
                "{file:<44} {:>5} {:>6.3} {:>5} {:>6.3} {:>6}",
                s.a_max, s.a_mean, s.c_max, s.c_mean, s.over3
            );
            if let Some(o) = &out {
                save_png(&o.join(&file), &got);
            }
        }
    }
}

/// Golden-Fälle (Referenzen in `tests\data\fx1`): Datei, Effektliste (JSON), erlaubte Abweichung.
fn golden_cases() -> Vec<(String, Value)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("fx1");
    let m: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("cases.json")).expect("cases.json"))
            .expect("json");
    m.as_array()
        .expect("arr")
        .iter()
        .map(|c| (c["file"].as_str().unwrap_or("").to_string(), c.clone()))
        .collect()
}

#[test]
fn golden_fx1() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("fx1");
    let mut fails = Vec::new();
    for (file, c) in golden_cases() {
        let obj = object(&dir.join(c["base"].as_str().expect("base")));
        let got = render(&obj, &entries(&c["effects"]));
        let s = stats(&got, &load_png(&dir.join(&file)));
        let tol = c["max"].as_u64().unwrap_or(3) as u8;
        println!(
            "{file:<44} A {:>3} {:>6.3}  C {:>3} {:>6.3}  >3 {}",
            s.a_max, s.a_mean, s.c_max, s.c_mean, s.over3
        );
        if s.a_max > tol || s.c_max > tol {
            fails.push(format!(
                "{file}: A max {} C max {} (erlaubt {tol})",
                s.a_max, s.c_max
            ));
        }
    }
    assert!(fails.is_empty(), "{fails:#?}");
}

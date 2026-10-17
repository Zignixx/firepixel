//! Korpus- und Laufzeittests (`cargo test -p fp-raster -- --ignored --nocapture`).
//!
//! - Korpus-Paare (`re\filter\_scratch\pairs`, 06 §9), aufbereitet von
//!   `tools\golden\fp-raster\extract_corpus.py` nach `tests\data\corpus\pairs.json`.
//! - Testdateien des Originals `re\_testdateien\06\*.fw.png` (Laufzeittests 06 §11.2): Eingaben und
//!   Lagen aus `tests\data\corpus\fw06.json`, Referenz = flaches IDAT bzw. gespeicherte Pixel,
//!   gemessen wie `re\scripts\filter\eval_t*.py`.
//!
//! „Gleich“ = alle vier Kanäle gleich oder beide Alpha 0 (06 §9).

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use fp_color::Rgba8;
use fp_raster::filters::{
    EffectParams, Filter, GaussPrecision, ParamValue, PixelFilter, SharpenParams, apply_chain,
};
use fp_raster::{CompareResult, IRect, Mask, RgbaImage};
use serde_json::Value;

const RE: &str = r"C:\Users\Administrator\Desktop\REA\re";

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("corpus")
}

fn json(name: &str) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(corpus_dir().join(name)).expect("extract_corpus.py ausführen"),
    )
    .unwrap()
}

fn effect_params(v: &Value) -> EffectParams {
    let mut p = EffectParams::new();
    if let Some(o) = v.as_object() {
        for (k, val) in o {
            let pv = match val {
                Value::String(s) => ParamValue::Text(s.clone()),
                Value::Array(a) => ParamValue::Points(
                    a.iter()
                        .map(|q| {
                            (
                                q[0].as_f64().unwrap_or(0.0) as f32,
                                q[1].as_f64().unwrap_or(0.0) as f32,
                            )
                        })
                        .collect(),
                ),
                other => ParamValue::Text(other.to_string()),
            };
            p.insert(k.clone(), pv);
        }
    }
    p
}

/// Sichtbare Effekte → Filter; `None`, wenn einer unbekannt ist.
fn filters_of(effects: &Value) -> Option<Vec<Filter>> {
    let mut out = Vec::new();
    for e in effects.as_array()? {
        if !e["visible"].as_bool().unwrap_or(true) {
            continue;
        }
        out.push(Filter::from_effect(
            e["moa_id"].as_str()?,
            &effect_params(&e["params"]),
        )?);
    }
    Some(out)
}

fn with_precision(fs: &[Filter], prec: GaussPrecision) -> Vec<Filter> {
    fs.iter()
        .map(|f| match f {
            Filter::GaussianBlur(p) => {
                Filter::GaussianBlur(fp_raster::filters::GaussianBlurParams {
                    precision: prec,
                    ..*p
                })
            }
            o => o.clone(),
        })
        .collect()
}

fn idat(file: &str, page: (i32, i32)) -> RgbaImage {
    common::load_png(
        &Path::new(RE).join("_testdateien").join("06").join(file),
        page.0,
        page.1,
    )
}

struct Fw06 {
    file: String,
    page: (i32, i32),
    elements: Vec<(String, RgbaImage, Value)>,
}

fn fw06() -> Vec<Fw06> {
    let v = json("fw06.json");
    v.as_array()
        .unwrap()
        .iter()
        .map(|f| Fw06 {
            file: f["file"].as_str().unwrap().to_string(),
            page: (
                f["page_x"].as_i64().unwrap() as i32,
                f["page_y"].as_i64().unwrap() as i32,
            ),
            elements: f["elements"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| {
                    let img = common::load_png(
                        &corpus_dir().join(e["pixels"].as_str().unwrap()),
                        e["x"].as_i64().unwrap() as i32,
                        e["y"].as_i64().unwrap() as i32,
                    );
                    (
                        e["name"].as_str().unwrap().to_string(),
                        img,
                        e["effects"].clone(),
                    )
                })
                .collect(),
        })
        .collect()
}

fn file<'a>(all: &'a [Fw06], name: &str) -> &'a Fw06 {
    all.iter()
        .find(|f| f.file == name)
        .unwrap_or_else(|| panic!("{name}"))
}

/// Live-Filterkette eines Elements rechnen und mit dem IDAT vergleichen (Ergebnisgrenzen ∩ Leinwand).
fn live_vs_idat(f: &Fw06, elem: &str, prec: GaussPrecision) -> CompareResult {
    let (_, img, eff) = f
        .elements
        .iter()
        .find(|e| e.0 == elem)
        .unwrap_or_else(|| panic!("{elem}"));
    let chain = with_precision(&filters_of(eff).expect("bekannte Filter"), prec);
    let out = apply_chain(img, &chain);
    let refimg = idat(&f.file, f.page);
    let r = out.bounds().intersect(&refimg.bounds());
    out.compare(&refimg, r)
}

#[test]
#[ignore]
fn corpus_pairs() {
    let pairs = json("pairs.json");
    let mut sums: BTreeMap<String, (u64, u64, u8, u32)> = BTreeMap::new();
    for p in pairs.as_array().unwrap() {
        let name = p["name"].as_str().unwrap();
        let inp = common::load_png(&corpus_dir().join(p["input"].as_str().unwrap()), 0, 0);
        let exp = common::load_png(&corpus_dir().join(p["output"].as_str().unwrap()), 0, 0);
        let Some(chain) = filters_of(&p["effects"]) else {
            println!("{name}: unbekannter Filter");
            continue;
        };
        let got = apply_chain(&inp, &chain);
        let r = got.compare(&exp, exp.bounds());
        let label: Vec<&str> = p["effects"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["name"].as_str().unwrap_or(""))
            .collect();
        let label = label.join("+");
        println!("{name:36} {label:34} {r}");
        let s = sums.entry(label).or_default();
        s.0 += r.equal;
        s.1 += r.total;
        s.2 = s.2.max(r.max_diff);
        s.3 += 1;
    }
    println!("--- Summe");
    for (k, (eq, tot, mx, n)) in &sums {
        println!("{k:40} {n} Fälle: {eq}/{tot} exakt, max. Abw. {mx}");
    }
    // Bitmap-Paare (ohne Gruppen-Komposit und ohne den veralteten Real-Estate-Cache) exakt wie die Referenz
    for (k, (eq, tot, _, _)) in &sums {
        if k == "Hue/Saturation..." || k == "Brightness/Contrast..." || k.starts_with("Curves...+")
        {
            continue;
        }
        assert_eq!(eq, tot, "{k}");
    }
}

#[test]
#[ignore]
fn t1_gauss_and_hue() {
    let all = fw06();
    let f = file(&all, "t1a_gauss_bmp.fw.png");
    for elem in ["bmp_r35", "bmp_r2"] {
        for prec in [GaussPrecision::Double, GaussPrecision::Single] {
            println!("T1 {elem} {prec:?}: {}", live_vs_idat(f, elem, prec));
        }
    }
    let r35 = live_vs_idat(f, "bmp_r35", GaussPrecision::Single);
    assert_eq!((r35.equal, r35.total), (1209, 1209));
    let hue = live_vs_idat(file(&all, "t1c_hue.fw.png"), "hue", GaussPrecision::Single);
    println!("T1c Farbton/Sättigung: {hue}");
    assert_eq!(hue.equal, hue.total);
}

#[test]
#[ignore]
fn t2_noise() {
    let all = fw06();
    for name in ["t2_noise.fw.png", "t2b_noise.fw.png"] {
        let f = file(&all, name);
        for elem in ["noise_grau", "noise_farbe"] {
            let r = live_vs_idat(f, elem, GaussPrecision::Single);
            println!("T2 {name} {elem}: {r}");
            assert_eq!(r.equal, r.total);
        }
    }
}

#[test]
#[ignore]
fn t3_sharpen() {
    let all = fw06();
    let f = file(&all, "t3_sharpen.fw.png");
    let orig = &f.elements.iter().find(|e| e.0 == "G_original").unwrap().1;
    for elem in ["A_live_sharpen", "B_live_sharpenmore"] {
        let r = live_vs_idat(f, elem, GaussPrecision::Single);
        println!("T3 {elem} (live, s = 1): {r}");
        assert_eq!(r.equal, r.total);
    }
    for (elem, s) in [
        ("C_menu_sharpen", 1),
        ("D_menu_sharpenmore", 1),
        ("E_byname_sharpen", 4),
        ("F_byname_sharpenmore", 8),
    ] {
        let stored = &f.elements.iter().find(|e| e.0 == elem).unwrap().1;
        let got = SharpenParams { amount: s }.apply(
            &orig.translated(
                stored.origin().0 - orig.origin().0,
                stored.origin().1 - orig.origin().1,
            ),
            None,
        );
        let r = got.compare(stored, stored.bounds());
        println!("T3 {elem} (destruktiv, s = {s}): {r}");
        assert_eq!(r.equal, r.total);
    }
}

/// Zentriertes Weichzeichnen (T4). Seit FX-3 (`73c3379`) wächst das Ergebnis wie im Original
/// (Ausgabegrenzen nach `@00b2c9e0`: Rechteck + Menge, dann Quadrat um die Mitte der Eingabe; gemessen
/// gegen neue Aufnahmen in Golden fx3, `fp-effects	estsx3.rs`). Die alten IDAT-Bilder dieser Dateien
/// enden an den Eingabegrenzen (das Original lief teils in die Zeitbegrenzung) – verglichen wird daher
/// nur innerhalb der Eingabe; die Grenzen prüft der Test gegen die FX-3-Formel (B-6).
#[test]
#[ignore]
fn t4_zoom() {
    let all = fw06();
    let f = file(&all, "t4_zoom.fw.png");
    let refimg = idat(&f.file, f.page);
    // (Element, erwartete Ausgabegrenzen links, oben, rechts, unten)
    for (elem, want) in [
        ("zoom_100_100", (-90, -250, 510, 350)),
        ("zoom_30_20", (-71, -81, 191, 181)),
    ] {
        let (_, img, eff) = f
            .elements
            .iter()
            .find(|e| e.0 == elem)
            .unwrap_or_else(|| panic!("{elem}"));
        let out = apply_chain(
            img,
            &with_precision(
                &filters_of(eff).expect("bekannte Filter"),
                GaussPrecision::Single,
            ),
        );
        let b = out.bounds();
        assert_eq!(
            (b.left, b.top, b.right, b.bottom),
            want,
            "{elem}: Ausgabegrenzen nach FX-3"
        );
        let a = out.alpha_bounds();
        let i = img.bounds();
        assert!(
            a.left < i.left && a.top < i.top && a.right > i.right && a.bottom > i.bottom,
            "{elem}: Ergebnis wächst über die Eingabe ({a:?})"
        );
        let r = out.compare(&refimg, i);
        println!("T4 {elem} innerhalb der Eingabe: {r}; Ausgabe {b:?}, deckend {a:?}");
        assert_eq!(r.equal, r.total, "{elem}");
    }
    // gewachsener Bereich: 30/20 links ganz bis zur Überlappung mit 100/100 (x ≥ 111) exakt; 100/100
    // rechts davon nur zur Information (das Original lief dort in die Zeitbegrenzung)
    let fx = |elem: &str| {
        let (_, img, eff) = f.elements.iter().find(|e| e.0 == elem).expect("Element");
        apply_chain(img, &filters_of(eff).expect("bekannte Filter"))
    };
    let r = fx("zoom_30_20").compare(&refimg, IRect::new(5, 15, 111, 85));
    println!("T4 zoom_30_20 gewachsener Bereich 5,15–111,85: {r}");
    assert_eq!((r.equal, r.total), (7_420, 7_420));
    println!(
        "T4 zoom_100_100 gewachsener Bereich 117,0–308,109 (Information): {}",
        fx("zoom_100_100").compare(&refimg, IRect::new(117, 0, 308, 109))
    );
    // T4b (100/100 einzeln): Referenz endet an den Eingabegrenzen und weicht auch innen ab (älterer Lauf
    // mit Zeitbegrenzung) – nur Grenzen prüfen, Vergleich zur Information
    let f = file(&all, "t4b_zoom_100.fw.png");
    let (_, img, eff) = f.elements.iter().find(|e| e.0 == "zoom").expect("zoom");
    let out = apply_chain(
        img,
        &with_precision(
            &filters_of(eff).expect("bekannte Filter"),
            GaussPrecision::Single,
        ),
    );
    let b = out.bounds();
    assert_eq!(
        (b.left, b.top, b.right, b.bottom),
        (-90, -100, 510, 500),
        "T4b: Ausgabegrenzen nach FX-3"
    );
    println!(
        "T4b Zoom 100/100 (Information, Referenz veraltet): innerhalb der Eingabe {}",
        out.compare(&idat(&f.file, f.page), img.bounds())
    );
}

#[test]
#[ignore]
fn t5_real_estate() {
    // Eingabe = Komposit der Kinder aus dem Korpus-Paar, Kurven → Farbton/Sättigung; Referenz = neues IDAT
    let pairs = json("pairs.json");
    let p = pairs
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "G_Real_Estate_551e9ec74e")
        .unwrap();
    let (x, y) = (50, 132);
    let inp = common::load_png(&corpus_dir().join(p["input"].as_str().unwrap()), x, y);
    let old_cache = common::load_png(&corpus_dir().join(p["output"].as_str().unwrap()), x, y);
    let got = apply_chain(&inp, &filters_of(&p["effects"]).unwrap());
    let neu = idat("t5_realestate_neu.fw.png", (0, 0));
    let alt = common::load_png(
        &std::env::var_os("FP_CORPUS_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("..")
                    .join("..")
                    .join("..")
                    .join("korpus")
            })
            .join("First Run")
            .join("Templates")
            .join("Web")
            .join("Real Estate.png"),
        0,
        0,
    );
    let all = got.compare(&neu, got.bounds());
    let (mut vis, mut ok) = (0u64, 0u64);
    for yy in y..y + 280 {
        for xx in x..x + 700 {
            let c = old_cache.pixel(xx, yy);
            if c.a == 255 && alt.pixel(xx, yy) == c {
                vis += 1;
                ok += (got.pixel(xx, yy) == neu.pixel(xx, yy)) as u64;
            }
        }
    }
    println!("T5 Real Estate: ganzer Instanzbereich {all}; unverdeckte Pixel {ok}/{vis}");
    assert_eq!(ok, vis);
    assert_eq!(vis, 114_798);
}

#[test]
#[ignore]
fn t6_gauss_large_radius() {
    let all = fw06();
    for (name, exact) in [
        ("t6_gauss210.fw.png", true),
        ("t6b_gauss201.fw.png", true),
        ("t6b_gauss199.fw.png", false),
    ] {
        let f = file(&all, name);
        let elem = f.elements[0].0.clone();
        for prec in [GaussPrecision::Double, GaussPrecision::Single] {
            let r = live_vs_idat(f, &elem, prec);
            println!("T6 {name} {prec:?}: {r}");
            if exact {
                assert_eq!(r.equal, r.total);
            }
        }
    }
}

#[test]
#[ignore]
fn perf_smoke() {
    // Laufzeiten großer Flächen (nur Ausgabe, keine Prüfung)
    let mut img = RgbaImage::new(IRect::new(0, 0, 2000, 1500)).unwrap();
    for (i, p) in img.data_mut().iter_mut().enumerate() {
        *p = [(i * 7) as u8, (i * 13) as u8, (i / 2000) as u8, 255];
    }
    for f in [
        Filter::GaussianBlur(fp_raster::filters::GaussianBlurParams {
            radius: 10.0,
            ..Default::default()
        }),
        Filter::HueSaturation(fp_raster::filters::HueSaturationParams {
            hue: 30,
            ..Default::default()
        }),
        Filter::Sharpen(SharpenParams::MENU),
        Filter::MotionBlur(fp_raster::filters::MotionBlurParams::default()),
    ] {
        let t = std::time::Instant::now();
        let out = f.apply(&img, None);
        println!("{:?}: {:?} ({:?})", f.moa_id(), t.elapsed(), out.bounds());
    }
}

#[test]
#[ignore]
fn t7_soft_selection_invert() {
    let all = fw06();
    let mut orig = RgbaImage::new(IRect::new(10, 10, 110, 110)).unwrap();
    for y in 0..100 {
        for x in 0..100 {
            orig.set_pixel(
                10 + x,
                10 + y,
                Rgba8::rgb((x as f64 * 2.55) as u8, (y as f64 * 2.55) as u8, 128),
            );
        }
    }
    let mask = Mask::from_rect(IRect::new(35, 35, 85, 85))
        .unwrap()
        .feather(10)
        .unwrap();
    let got = Filter::Invert.apply(&orig, Some(&mask));
    for name in [
        "t7g_gui.fw.png",
        "t7g_weich_umkehren.fw.png",
        "t7_weich_umkehren.fw.png",
        "t7b_ohne_neuauswahl.fw.png",
    ] {
        let stored = &file(&all, name).elements[0].1;
        let r = got.compare(stored, orig.bounds());
        println!("T7 {name} (Bitmap {:?}): {r}", stored.bounds());
        if name == "t7g_gui.fw.png" {
            assert_eq!((r.equal, r.total), (10_000, 10_000));
        }
    }
}

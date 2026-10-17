//! Golden-Tests gegen die Python-Referenz (`tools\golden\fp-text\gen_golden.py` →
//! `tests\data\golden_*.json`). Tests, die Systemschriften brauchen, werden übersprungen, wenn
//! Myriad Pro/Arial fehlen.

mod common;

use common::*;
use fp_geom::IRect;
use fp_text::*;
use serde_json::Value;

fn load(name: &str) -> Vec<Value> {
    let s = std::fs::read_to_string(data_dir().join(name)).expect("Golden-Datei fehlt");
    serde_json::from_str::<Value>(&s)
        .expect("JSON")
        .as_array()
        .cloned()
        .unwrap_or_default()
}

#[test]
fn golden_custom_luts() {
    let cases = load("golden_luts.json");
    assert!(cases.len() > 300);
    for c in &cases {
        let (n, s, w) = (
            i(c, "n").unwrap() as u32,
            i(c, "s").unwrap() as u8,
            i(c, "w").unwrap() as u8,
        );
        let want: Vec<u8> = c["lut"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u8)
            .collect();
        assert_eq!(antialias::custom_lut(n, s, w), want, "N {n} S {s} W {w}");
    }
}

#[test]
fn golden_font_metrics() {
    if !have_corpus_fonts() {
        eprintln!("übersprungen: Korpusschriften fehlen");
        return;
    }
    let mut checked = 0;
    for c in load("golden_fonts.json") {
        let ps = c["ps"].as_str().unwrap();
        let Some(id) = fonts().find_ps(ps) else {
            continue;
        };
        let f = fonts().load(id).unwrap();
        assert_eq!(f.units_per_em() as i64, i(&c, "upm").unwrap(), "{ps}");
        assert_eq!(f.ascent(), c["asc"].as_f64().unwrap(), "{ps} asc");
        assert_eq!(f.descent(), c["desc"].as_f64().unwrap(), "{ps} desc");
        for g in c["glyphs"].as_array().unwrap() {
            let ch = g["ch"].as_str().unwrap().chars().next().unwrap();
            let gid = f.glyph_index(ch);
            assert_eq!(gid as i64, i(g, "gid").unwrap(), "{ps} gid {ch}");
            assert_eq!(f.advance(gid) as i64, i(g, "adv").unwrap(), "{ps} adv {ch}");
            let ink = f.outline(gid).ink.map(|b| (b[0], b[1]));
            match g["ink"].as_array() {
                Some(v) => {
                    let (a, b) = ink.unwrap_or_else(|| panic!("{ps} {ch}: Tinte fehlt"));
                    assert!(
                        (a - v[0].as_f64().unwrap()).abs() < 1e-9
                            && (b - v[1].as_f64().unwrap()).abs() < 1e-9,
                        "{ps} Tinte {ch}"
                    );
                }
                None => assert!(ink.is_none(), "{ps} {ch}: Tinte unerwartet"),
            }
        }
        for k in c["kern"].as_array().unwrap() {
            let p: Vec<char> = k["pair"].as_str().unwrap().chars().collect();
            let got = f.kerning(f.glyph_index(p[0]), f.glyph_index(p[1]));
            assert_eq!(got as i64, i(k, "kern").unwrap(), "{ps} Kerning {:?}", p);
        }
        checked += 1;
    }
    assert!(checked >= 5, "zu wenige Schriften geprüft: {checked}");
}

#[test]
fn golden_layout_matches_python() {
    if !have_corpus_fonts() {
        eprintln!("übersprungen: Korpusschriften fehlen");
        return;
    }
    let opts = LayoutOptions::default();
    let cases = load("golden_layout.json");
    let mut n = 0;
    for c in &cases {
        let block = block_from(c);
        let lay = layout(&block, fonts(), &opts);
        let name = c["name"].as_str().unwrap_or("");
        let py: Vec<f64> = c["py_frame"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        assert_eq!(
            frame_arr(&lay.frame).to_vec(),
            py,
            "Rahmen {name} {:?}",
            block.text()
        );
        let want = c["py_glyphs"].as_array().unwrap();
        assert_eq!(lay.glyphs.len(), want.len(), "Glyphenzahl {name}");
        for (g, w) in lay.glyphs.iter().zip(want) {
            let w = w.as_array().unwrap();
            assert_eq!(g.glyph_id as u64, w[0].as_u64().unwrap(), "gid {name}");
            assert!(
                (g.x - w[1].as_f64().unwrap()).abs() < 1e-9,
                "x {name}: {} gegen {}",
                g.x,
                w[1]
            );
            assert!(
                (g.y - w[2].as_f64().unwrap()).abs() < 1e-9,
                "y {name}: {} gegen {}",
                g.y,
                w[2]
            );
        }
        n += 1;
    }
    assert!(n > 250);
}

#[test]
fn golden_raster_counts() {
    if !have_corpus_fonts() {
        eprintln!("übersprungen: Korpusschriften fehlen");
        return;
    }
    // Die Python-Referenz rastert ungehintet
    let opts = LayoutOptions {
        hinting: false,
        ..LayoutOptions::default()
    };
    for c in load("golden_raster.json") {
        let r = &c["raster"];
        let (x, y, w, h, n) = (
            i(r, "x").unwrap(),
            i(r, "y").unwrap(),
            i(r, "w").unwrap(),
            i(r, "h").unwrap(),
            i(r, "n").unwrap(),
        );
        let block = block_from(&c);
        let lay = layout(&block, fonts(), &opts);
        let counts = render::sample_layout_n(
            &lay,
            IRect::new(x as i32, y as i32, (x + w) as i32, (y + h) as i32),
            n as u32,
        );
        let want: Vec<u16> = r["counts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u16)
            .collect();
        let diff = counts.iter().zip(&want).filter(|(a, b)| a != b).count();
        assert_eq!(diff, 0, "{:?} N {n}: {diff} Pixel abweichend", block.text());
    }
}

//! Golden-Tests gegen die Python-Referenzen (`re\export\fw_palette.py`, `fw_rgba.py`, `fw_gif.py`).
//! Daten: `tests/data/golden.json`, erzeugt von `tools\golden\fp-export\gen_golden.py`.

#![allow(clippy::field_reassign_with_default)]

use fp_export::options::{Dither, ExportFormat, OptimizeSettings, PaletteColor, PaletteKind, Transparency};
use fp_export::pipeline::{build_quantizer, remap_pixels, ExportContext};
use serde_json::Value;

fn load() -> Value {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("data").join("golden.json");
    serde_json::from_str(&std::fs::read_to_string(p).expect("golden.json")).expect("JSON")
}

fn u32s(v: &Value) -> Vec<u32> {
    v.as_array().map(|a| a.iter().map(|x| x.as_u64().unwrap_or(0) as u32).collect()).unwrap_or_default()
}

fn settings(o: &Value) -> OptimizeSettings {
    let i = |k: &str| o[k].as_i64().unwrap_or(0) as i32;
    let b = |k: &str| o[k].as_bool().unwrap_or(false);
    let mut s = OptimizeSettings::default();
    s.format = if i("exportFormat") == 2 { ExportFormat::Png } else { ExportFormat::Gif };
    s.palette_kind = PaletteKind::from_code(i("paletteMode"));
    s.max_colors = i("numEntriesRequested");
    s.grid_levels = i("numGridEntries");
    s.dither = match i("ditherMode") {
        1 => Dither::Diffusion(i("ditherPercent")),
        2 => Dither::Pattern2x2,
        _ => Dither::None,
    };
    s.transparency = Transparency::from_code(i("paletteTransparency"));
    s.web_adaptive = b("webSnapAdaptive");
    s.websnap_tolerance = i("webSnapTolerance");
    s.optimized = b("optimized");
    s.palette = o["palette"]
        .as_array()
        .map(|a| a.iter().map(|e| PaletteColor { locked: e["locked"].as_bool().unwrap_or(false), ..PaletteColor::new(e["color"].as_u64().unwrap_or(0) as u32) }).collect())
        .unwrap_or_default();
    s
}

#[test]
fn palettes_and_remap_like_python() {
    let g = load();
    let (w, h) = (g["w"].as_u64().unwrap() as usize, g["h"].as_u64().unwrap() as usize);
    let ctx = ExportContext { canvas: 0xFFFF_FFFF, ..ExportContext::default() };
    let mut ok_pal = 0;
    let mut ok_idx = 0;
    let cases = g["cases"].as_array().unwrap();
    let mut fails = Vec::new();
    for c in cases {
        let px = u32s(&g["images"][c["image"].as_str().unwrap()]);
        let s = settings(&c["opts"]);
        let mut built = build_quantizer(&px, w, &s, &ctx);
        let idx = remap_pixels(&mut built, &px, w, h, &s, &ctx);
        let pal_eq = built.q.palette == u32s(&c["palette"]);
        let idx_eq = idx.iter().map(|&v| v as u32).collect::<Vec<_>>() == u32s(&c["indices"]);
        ok_pal += usize::from(pal_eq);
        ok_idx += usize::from(idx_eq);
        if !(pal_eq && idx_eq) {
            fails.push(format!("{} (Palette {pal_eq}, Indizes {idx_eq})", c["name"].as_str().unwrap_or("")));
        }
    }
    println!("Golden: Paletten {ok_pal}/{}, Indexbilder {ok_idx}/{}", cases.len(), cases.len());
    assert!(fails.is_empty(), "{fails:?}");
}

#[test]
fn lossy_and_lzw_like_python() {
    let g = load();
    let l = &g["lossy"];
    let pal: Vec<[u8; 3]> = l["palette"].as_array().unwrap().iter().map(|p| [p[0].as_u64().unwrap() as u8, p[1].as_u64().unwrap() as u8, p[2].as_u64().unwrap() as u8]).collect();
    let idx: Vec<u8> = u32s(&l["indices"]).into_iter().map(|v| v as u8).collect();
    for (k, want) in l["out"].as_object().unwrap() {
        let lossy: i32 = k.parse().unwrap();
        let got = fp_export::gif::lossy_simulate(&idx, &pal, lossy);
        assert_eq!(got.iter().map(|&v| v as u32).collect::<Vec<_>>(), u32s(want), "Verlust {lossy}");
    }
    let z = &g["lzw"];
    let zi: Vec<u8> = u32s(&z["indices"]).into_iter().map(|v| v as u8).collect();
    assert_eq!(fp_export::gif::lzw_encode(&zi, 4).iter().map(|&v| v as u32).collect::<Vec<_>>(), u32s(&z["data"]));
}

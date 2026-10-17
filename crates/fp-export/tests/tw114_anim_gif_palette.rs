//! TW-114 (spec 44): animated GIF with the "Adaptive" palette builds one global palette over all
//! states (histogram summed over every rendered frame, no fine-tuning, no tolerance-3 web snap),
//! capped at 255 colours and sorted by luminance descending with the CRT quicksort.
//!
//! Expected values: `tests/data/tw114_anim_gif_palette.txt` (global colour table and a CRC-32 of the
//! frame-1 indices; reference measurement 2026-10-17). Input: composite image of the test document
//! `par_export.fw.png` as state 1 and an empty (fully transparent) state 2, like the document's
//! export. Skipped when the test documents are not present.

mod common;

use common::{re_dir, read_gif, read_png};
use fp_export::animation::{AnimFrameSrc, animated_gif};
use fp_export::options::{AnimSettings, Dither, ExportFormat, OptimizeSettings, PaletteKind};
use fp_export::pipeline::ExportContext;
use fp_model::FrameInfo;

struct Expected {
    size: (usize, usize),
    crc: u32,
    gct: Vec<[u8; 3]>,
}

fn expected() -> Expected {
    let text = include_str!("data/tw114_anim_gif_palette.txt");
    let mut e = Expected {
        size: (0, 0),
        crc: 0,
        gct: Vec::new(),
    };
    for line in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let parts: Vec<&str> = line.split_whitespace().collect();
        match parts.as_slice() {
            ["size", w, h] => e.size = (w.parse().unwrap_or(0), h.parse().unwrap_or(0)),
            ["frame1_crc32", c] => e.crc = u32::from_str_radix(c, 16).unwrap_or(0),
            [hex] => {
                let v = u32::from_str_radix(hex, 16).unwrap_or(0);
                e.gct.push([(v >> 16) as u8, (v >> 8) as u8, v as u8]);
            }
            _ => {}
        }
    }
    e
}

#[test]
fn tw114_global_luminance_sorted_palette() {
    let exp = expected();
    assert_eq!(exp.gct.len(), 256);
    let src = re_dir()
        .join("_testdateien")
        .join("par")
        .join("par_export.fw.png");
    let Some((w, h, px)) = read_png(&src) else {
        eprintln!("test document missing, skipped: {}", src.display());
        return;
    };
    assert_eq!((w, h), exp.size);
    let info = FrameInfo {
        delay: 7,
        disposal: 1,
        ..FrameInfo::default()
    };
    let frames = vec![
        AnimFrameSrc {
            pixels: px,
            info: info.clone(),
        },
        AnimFrameSrc {
            pixels: vec![0; w * h],
            info,
        },
    ];
    // document options: adaptive, 256 colours, no transparency, web-adaptive off (tolerance 14),
    // no dither, optimised, auto difference, loop forever; the measurement run switched auto crop off
    let s = OptimizeSettings {
        format: ExportFormat::AnimatedGif,
        palette_kind: PaletteKind::Adaptive,
        web_adaptive: false,
        websnap_tolerance: 14,
        max_colors: 256,
        dither: Dither::None,
        optimized: true,
        anim: AnimSettings {
            loop_count: 0,
            auto_crop: false,
            auto_difference: true,
            frames: Vec::new(),
        },
        ..OptimizeSettings::default()
    };
    let ctx = ExportContext {
        canvas: 0x00FF_FFFF,
        ..ExportContext::default()
    };
    let bytes = animated_gif(&frames, w, h, &s, &ctx).expect("GIF");
    let dir = std::env::temp_dir().join(format!("fp_tw114_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("anim.gif");
    std::fs::write(&path, &bytes).expect("write");
    let g = read_gif(&path).expect("read GIF");
    let _ = std::fs::remove_dir_all(&dir);

    let same = g.gct.iter().zip(&exp.gct).filter(|(a, b)| a == b).count();
    for (i, (a, b)) in g.gct.iter().zip(&exp.gct).enumerate() {
        if a != b {
            eprintln!("entry {i}: {a:?} expected {b:?}");
        }
    }
    assert_eq!(g.gct.len(), 256, "global colour table size");
    assert_eq!(same, 256, "palette entries at the same position");
    let f1 = &g.frames[0];
    assert_eq!((f1.w, f1.h), (w, h));
    assert_eq!(
        crc32fast::hash(&f1.indices),
        exp.crc,
        "frame-1 indices differ from the reference"
    );
}

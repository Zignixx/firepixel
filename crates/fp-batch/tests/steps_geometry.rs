//! Schritte der Gruppe „Geometrie“ (BATCH-4, SPEC-BATCH §3.3): `resize`, `crop`, `trim`, `rotate`,
//! `canvas` – Ergebnis-Größen und -Pixel, Golden-Hashes, Logikfälle nach AGENTEN_REGELN §4b
//! (0×0, riesig, Transparenz, Animation, Wiederholung, Vorschau, Determinismus).

mod batch4_common;

use batch4_common::*;
use fp_batch::item::Item;
use fp_batch::pipeline::{preview, PreviewOptions};
use fp_batch::steps::geometry::{exif_orientation, resample, ResizeFilter};
use fp_batch::{CancelToken, Recipe, StepRegistry};
use fp_raster::RgbaImage;
use serde_json::json;

fn dims(item: &Item) -> (u32, u32) {
    item.dimensions()
}

// ------------------------------------------------------------------ resize

#[test]
fn resize_modes_compute_the_right_size() {
    let cases = [
        (json!({"mode": "percent", "percent": 50}), (200, 150)),
        (json!({"mode": "width", "width": 100}), (100, 75)),
        (json!({"mode": "height", "height": 60}), (80, 60)),
        (json!({"mode": "longEdge", "size": 200}), (200, 150)),
        (json!({"mode": "shortEdge", "size": 150}), (200, 150)),
        (json!({"mode": "fit", "width": 100, "height": 100}), (100, 75)),
        (json!({"mode": "fill", "width": 100, "height": 100}), (100, 100)),
        (json!({"mode": "exact", "width": 50, "height": 20}), (50, 20)),
        (json!({"mode": "none"}), (400, 300)),
    ];
    for (p, want) in cases {
        let mut it = still(gradient(400, 300));
        apply("resize", p.clone(), &mut it).expect("resize");
        assert_eq!(dims(&it), want, "{p}");
        assert_eq!(first(&it).origin(), (0, 0));
    }
}

#[test]
fn resize_only_shrink_keeps_small_images() {
    let mut it = still(gradient(400, 300));
    let before = hash(&first(&it));
    apply("resize", json!({"mode": "longEdge", "size": 1600}), &mut it).expect("resize");
    assert_eq!(dims(&it), (400, 300));
    assert_eq!(hash(&first(&it)), before, "unverändert, bitgleich");
    apply("resize", json!({"mode": "longEdge", "size": 1600, "onlyShrink": false}), &mut it).expect("resize");
    assert_eq!(dims(&it), (1600, 1200));
    // fill + nur verkleinern: auf das Seitenverhältnis zuschneiden, nicht vergrößern
    let mut it = still(gradient(400, 300));
    apply("resize", json!({"mode": "fill", "width": 1000, "height": 1000}), &mut it).expect("fill");
    assert_eq!(dims(&it), (300, 300));
}

#[test]
fn resize_golden_hashes() {
    let src = gradient(64, 48);
    let cancel = CancelToken::new();
    let got: Vec<(ResizeFilter, u32, u32, u64)> = [
        (ResizeFilter::Lanczos3, 32, 24),
        (ResizeFilter::Bicubic, 32, 24),
        (ResizeFilter::Bilinear, 32, 24),
        (ResizeFilter::Nearest, 32, 24),
        (ResizeFilter::Lanczos3, 100, 75),
        (ResizeFilter::Bicubic, 7, 5),
    ]
    .into_iter()
    .map(|(f, w, h)| {
        let r = resample(&src, w, h, f, &cancel).expect("resample");
        assert_eq!((r.width() as u32, r.height() as u32), (w, h));
        (f, w, h, hash(&r))
    })
    .collect();
    let want: [u64; 6] = GOLDEN_RESIZE;
    for g in &got {
        println!("{:?} {}x{} -> {:#018x}", g.0, g.1, g.2, g.3);
    }
    assert_eq!(got.iter().map(|g| g.3).collect::<Vec<_>>(), want.to_vec(), "Golden-Hashes");
}

/// Golden-Hashes (FNV-1a über Maße + Pixel) des Verlaufs 64 × 48; bei bewusster Änderung des
/// Resamplers neu erzeugen (`-- --nocapture` zeigt die Werte).
const GOLDEN_RESIZE: [u64; 6] = [0x6047d47ab8d074b3, 0x66a29aa02629c6d9, 0x4c37ae693ce4d08e, 0x3eaf8b77daf71f5d, 0x73c18f046841b1a0, 0x4d81bce367d939d8];

#[test]
fn resize_uniform_image_stays_uniform_and_alpha_has_no_dark_fringe() {
    // einfarbig bleibt exakt einfarbig (normierte Gewichte, kein Überschwingen)
    let flat = image(37, 23, |_, _| [200, 100, 50, 255]);
    let cancel = CancelToken::new();
    for f in [ResizeFilter::Lanczos3, ResizeFilter::Bicubic, ResizeFilter::Bilinear] {
        for (w, h) in [(10, 7), (80, 51), (1, 1)] {
            let r = resample(&flat, w, h, f, &cancel).expect("resample");
            assert!(r.data().iter().all(|p| *p == [200, 100, 50, 255]), "{f:?} {w}x{h}");
        }
    }
    // rote Hälfte neben transparentem Schwarz: Randpixel bleiben rot (vormultipliziert gerechnet)
    let half = image(40, 10, |x, _| if x < 20 { [255, 0, 0, 255] } else { [0, 0, 0, 0] });
    let r = resample(&half, 13, 4, ResizeFilter::Lanczos3, &cancel).expect("resample");
    for p in r.data() {
        if p[3] > 8 {
            assert!(p[0] >= 250 && p[1] <= 3 && p[2] <= 3, "kein dunkler Saum: {p:?}");
        }
    }
    assert!(r.data().iter().any(|p| p[3] == 0) && r.data().iter().any(|p| p[3] == 255));
}

#[test]
fn resize_auto_uses_nearest_for_pixel_art() {
    let checker = image(8, 8, |x, y| if (x + y) % 2 == 0 { [0, 0, 0, 255] } else { [255, 255, 255, 255] });
    let mut it = still(checker);
    apply("resize", json!({"mode": "percent", "percent": 300, "onlyShrink": false}), &mut it).expect("resize");
    assert_eq!(dims(&it), (24, 24));
    let img = first(&it);
    assert!(img.data().iter().all(|p| *p == [0, 0, 0, 255] || *p == [255, 255, 255, 255]), "harte Kanten");
    // Foto-artig (viele Farben) → Lanczos, Zwischenwerte entstehen
    let mut it = still(gradient(300, 200));
    apply("resize", json!({"mode": "percent", "percent": 33}), &mut it).expect("resize");
    assert_eq!(dims(&it), (99, 66));
}

#[test]
fn resize_huge_target_is_an_error_not_a_panic() {
    let mut it = still(gradient(10, 10));
    let e = apply("resize", json!({"mode": "exact", "width": 32768, "height": 32768, "onlyShrink": false}), &mut it).expect_err("zu groß");
    assert!(!e.to_string().is_empty());
    assert_eq!(dims(&it), (10, 10), "Element unverändert");
    // percent 10000 eines 2000×2000-Bildes ebenso (ohne riesige Quelle: Maße aus dem Kopf)
    let mut it = still(gradient(200, 200));
    assert!(apply("resize", json!({"mode": "percent", "percent": 10000, "onlyShrink": false}), &mut it).is_err());
}

#[test]
fn zero_sized_images_pass_through_all_geometry_steps() {
    for (id, p) in [
        ("resize", json!({"mode": "percent", "percent": 50})),
        ("crop", json!({"mode": "aspect", "ratio": "16:9"})),
        ("trim", json!({})),
        ("rotate", json!({"angle": 33})),
        ("canvas", json!({"mode": "square"})),
    ] {
        let mut it = still(RgbaImage::empty_at(0, 0));
        apply(id, p, &mut it).unwrap_or_else(|e| panic!("{id}: {e}"));
        assert_eq!(dims(&it), (0, 0), "{id}");
    }
}

#[test]
fn resize_is_deterministic_across_thread_counts() {
    let src = gradient(301, 207);
    let cancel = CancelToken::new();
    for f in [ResizeFilter::Lanczos3, ResizeFilter::Bicubic, ResizeFilter::Nearest] {
        let (a, b) = single_and_multi(|| hash(&resample(&src, 97, 61, f, &cancel).expect("r")));
        assert_eq!(a, b, "{f:?}");
    }
}

#[test]
fn resize_animation_resizes_every_frame_and_keeps_timing() {
    let frames = (0..3).map(|i| (image(40, 30, move |x, y| [(x * 6) as u8, (y * 8) as u8, i * 80, 255]), 10 * (i as u32 + 1))).collect();
    let mut it = animation(frames);
    apply("resize", json!({"mode": "percent", "percent": 50}), &mut it).expect("resize");
    let Item::Animation(a) = &it else { panic!("bleibt Animation") };
    assert_eq!((a.width, a.height), (20, 15));
    assert_eq!(a.frames.len(), 3);
    assert_eq!(a.frames.iter().map(|f| f.delay_ms).collect::<Vec<_>>(), vec![10, 20, 30]);
    assert!(a.frames.iter().all(|f| (f.image.width(), f.image.height()) == (20, 15)));
    // die Frames bleiben verschieden (jedes Frame einzeln gerechnet)
    assert_ne!(hash(&a.frames[0].image), hash(&a.frames[2].image));
}

#[test]
fn resize_dpi_is_set_in_metadata() {
    let mut it = still(gradient(20, 20));
    apply("resize", json!({"mode": "none", "dpi": 300}), &mut it).expect("dpi");
    assert_eq!(it.meta().and_then(|m| m.dpi), Some(300.0));
}

#[test]
fn resize_repeated_param_changes_equal_final_value() {
    // §4b.1: Wert 50× ändern → Rezept enthält den Schritt einmal, Ergebnis wie einmal mit dem Endwert
    let mut r = Recipe::named("t");
    let i = r.add_step("resize", json!({"mode": "longEdge"}));
    for v in 1..=50 {
        assert!(r.set_param(i, "size", json!(v * 4)));
    }
    assert_eq!(r.steps.len(), 1);
    let reg = StepRegistry::global();
    let pipe = fp_batch::Pipeline::new(&r, &reg).expect("pipeline");
    let mut it = still(gradient(400, 300));
    let inf = info(400, 300);
    let cancel = CancelToken::new();
    let mut ctx = fp_batch::StepContext::new(0, 1, &inf, &cancel, fp_batch::OutputFormat::Png);
    pipe.apply(&mut it, &mut ctx).expect("apply");
    assert_eq!(dims(&it), (200, 150));
    // nochmal anwenden: schon klein genug, bleibt (kein Doppel-Effekt)
    let h = hash(&first(&it));
    pipe.apply(&mut it, &mut ctx).expect("apply 2");
    assert_eq!(hash(&first(&it)), h);
}

#[test]
fn preview_scales_pixel_parameters_with_the_proxy() {
    let dir = std::env::temp_dir().join(format!("fp-batch4-preview-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    let path = dir.join("big.png");
    let item = still(gradient(2048, 1024));
    let (bytes, _) = fp_batch::output::encode(&item, fp_batch::OutputFormat::Png, &Default::default()).expect("png");
    std::fs::write(&path, bytes).expect("write");
    let inf = fp_batch::probe(&path).expect("probe");
    let mut r = Recipe::named("t");
    r.add_step("resize", json!({"mode": "longEdge", "size": 1600}));
    r.add_step("canvas", json!({"mode": "addBorder", "border": 100}));
    let reg = StepRegistry::global();
    let res = preview(&r, &inf, &reg, &PreviewOptions::default()).expect("preview");
    // Proxy 1024 × 512 (Maßstab ½): 1600 → 800, Rand 100 → 50
    let after = res.after.expect("after");
    assert_eq!((after.width(), after.height()), (900, 500));
    assert_eq!(res.after_size, (1800, 1000), "hochgerechnet wie das echte Ergebnis");
    // zweimal Vorschau = gleich (nichts addiert sich)
    let res2 = preview(&r, &inf, &reg, &PreviewOptions::default()).expect("preview 2");
    assert_eq!(hash(&res2.after.expect("after")), hash(&after));
    // volle Rechnung
    let full = preview(&r, &inf, &reg, &PreviewOptions { encode_full: true, ..PreviewOptions::default() }).expect("full");
    assert_eq!(full.after_size, (1800, 1000));
    let _ = std::fs::remove_dir_all(&dir);
}

// ------------------------------------------------------------------ crop

#[test]
fn crop_aspect_with_anchor() {
    let src = gradient(400, 300);
    let mut it = still(src.clone());
    apply("crop", json!({"mode": "aspect", "ratio": "1:1"}), &mut it).expect("crop");
    assert_eq!(dims(&it), (300, 300));
    assert_eq!(px(&first(&it), 0, 0), px(&src, 50, 0), "mittig");
    let mut it = still(src.clone());
    apply("crop", json!({"mode": "aspect", "ratio": "1:1", "anchor": "right"}), &mut it).expect("crop");
    assert_eq!(px(&first(&it), 0, 0), px(&src, 100, 0));
    for (ratio, want) in [("16:9", (400, 225)), ("9:16", (169, 300)), ("4:3", (400, 300)), ("3:2", (400, 267))] {
        let mut it = still(src.clone());
        apply("crop", json!({"mode": "aspect", "ratio": ratio}), &mut it).expect("crop");
        assert_eq!(dims(&it), want, "{ratio}");
    }
    let mut it = still(src.clone());
    apply("crop", json!({"mode": "aspect", "ratio": "free", "ratioWidth": 2, "ratioHeight": 1}), &mut it).expect("crop");
    assert_eq!(dims(&it), (400, 200));
}

#[test]
fn crop_rect_and_edges_in_px_and_percent() {
    let src = gradient(400, 300);
    let mut it = still(src.clone());
    apply("crop", json!({"mode": "rect", "x": 10, "y": 20, "width": 100, "height": 50}), &mut it).expect("crop");
    assert_eq!(dims(&it), (100, 50));
    assert_eq!(px(&first(&it), 0, 0), px(&src, 10, 20));
    let mut it = still(src.clone());
    apply("crop", json!({"mode": "rect", "unit": "percent", "x": 25, "y": 0, "width": 50, "height": 100}), &mut it).expect("crop");
    assert_eq!(dims(&it), (200, 300));
    // über das Bild hinaus → beschnitten
    let mut it = still(src.clone());
    apply("crop", json!({"mode": "rect", "x": 350, "y": 250, "width": 1000, "height": 1000}), &mut it).expect("crop");
    assert_eq!(dims(&it), (50, 50));
    // ganz außerhalb → Warnung, unverändert
    let mut it = still(src.clone());
    let w = apply("crop", json!({"mode": "rect", "x": 5000, "y": 0, "width": 10, "height": 10}), &mut it).expect("crop");
    assert_eq!(dims(&it), (400, 300));
    assert_eq!(w.len(), 1);
    let mut it = still(src.clone());
    apply("crop", json!({"mode": "edges", "top": 10, "bottom": 20, "left": 30, "right": 40}), &mut it).expect("edges");
    assert_eq!(dims(&it), (330, 270));
    assert_eq!(px(&first(&it), 0, 0), px(&src, 30, 10));
    let mut it = still(src);
    let w = apply("crop", json!({"mode": "edges", "unit": "percent", "left": 60, "right": 60}), &mut it).expect("edges");
    assert_eq!(dims(&it), (400, 300), "nichts übrig → unverändert");
    assert_eq!(w.len(), 1);
}

#[test]
fn crop_animation_same_rect_for_all_frames() {
    let frames = (0..4).map(|i| (gradient(60, 40), 50 + i)).collect();
    let mut it = animation(frames);
    apply("crop", json!({"mode": "aspect", "ratio": "1:1"}), &mut it).expect("crop");
    let Item::Animation(a) = &it else { panic!() };
    assert_eq!((a.width, a.height, a.frames.len()), (40, 40, 4));
    assert!(a.frames.iter().all(|f| f.image.width() == 40 && f.image.height() == 40));
}

// ------------------------------------------------------------------ trim

fn framed(w: u32, h: u32, margin: u32, bg: [u8; 4]) -> RgbaImage {
    image(w, h, move |x, y| if x >= margin && y >= margin && x < w - margin && y < h - margin { [10, 200, 30, 255] } else { bg })
}

#[test]
fn trim_transparent_color_both_and_padding() {
    let mut it = still(framed(50, 40, 5, [0, 0, 0, 0]));
    apply("trim", json!({}), &mut it).expect("trim");
    assert_eq!(dims(&it), (40, 30));
    assert!(first(&it).data().iter().all(|p| *p == [10, 200, 30, 255]));
    // Polster: transparent um den Inhalt
    let mut it = still(framed(50, 40, 5, [0, 0, 0, 0]));
    apply("trim", json!({"padding": 2}), &mut it).expect("trim");
    assert_eq!(dims(&it), (44, 34));
    // Polster größer als der Rand: wächst transparent über das Bild hinaus
    let mut it = still(framed(50, 40, 5, [0, 0, 0, 0]));
    apply("trim", json!({"padding": 8}), &mut it).expect("trim");
    assert_eq!(dims(&it), (56, 46));
    assert_eq!(px(&first(&it), 0, 0), [0, 0, 0, 0]);
    // Farbe der Ecke (weiß, mit Toleranz)
    let mut it = still(image(30, 30, |x, y| if (10..20).contains(&x) && (5..25).contains(&y) { [0, 0, 255, 255] } else { [250, 252, 255, 255] }));
    apply("trim", json!({"what": "color", "tolerance": 8}), &mut it).expect("trim");
    assert_eq!(dims(&it), (10, 20));
    // „transparent“ lässt einen weißen Rand stehen
    let mut it = still(framed(30, 30, 4, [255, 255, 255, 255]));
    apply("trim", json!({"what": "transparent"}), &mut it).expect("trim");
    assert_eq!(dims(&it), (30, 30));
    // beides: transparenter Außenrand + weißer Innenrand
    // beides: weißer Außenrand (Farbe der Ecke) + transparenter Innenrand
    let mut it = still(image(30, 30, |x, y| {
        let inner = (3..27).contains(&x) && (3..27).contains(&y);
        let core = (8..22).contains(&x) && (8..22).contains(&y);
        if core {
            [9, 9, 9, 255]
        } else if inner {
            [0, 0, 0, 0]
        } else {
            [255, 255, 255, 255]
        }
    }));
    apply("trim", json!({"what": "both"}), &mut it).expect("trim");
    assert_eq!(dims(&it), (14, 14));
}

#[test]
fn trim_fully_empty_image_stays_with_warning() {
    let mut it = still(image(20, 20, |_, _| [0, 0, 0, 0]));
    let w = apply("trim", json!({}), &mut it).expect("trim");
    assert_eq!(dims(&it), (20, 20));
    assert_eq!(w.len(), 1);
}

#[test]
fn trim_animation_uses_union_of_all_frames() {
    let f1 = image(50, 50, |x, y| if (10..20).contains(&x) && (10..20).contains(&y) { [255, 0, 0, 255] } else { [0, 0, 0, 0] });
    let f2 = image(50, 50, |x, y| if (30..40).contains(&x) && (25..45).contains(&y) { [0, 255, 0, 255] } else { [0, 0, 0, 0] });
    let mut it = animation(vec![(f1, 100), (f2, 200)]);
    apply("trim", json!({}), &mut it).expect("trim");
    let Item::Animation(a) = &it else { panic!() };
    assert_eq!((a.width, a.height), (30, 35));
    assert!(a.frames.iter().all(|f| f.image.width() == 30 && f.image.height() == 35));
    assert_eq!(px(&a.frames[0].image, 0, 0), [255, 0, 0, 255]);
    assert_eq!(a.frames[1].delay_ms, 200);
}

// ------------------------------------------------------------------ rotate

#[test]
fn rotate_quarters_and_flips_map_pixels_exactly() {
    // 3×2: Ecke oben links rot
    let src = image(3, 2, |x, y| [(x * 100) as u8, (y * 100) as u8, 7, 255]);
    type Case = (serde_json::Value, (u32, u32), [(usize, usize, (u32, u32)); 2]);
    let cases: [Case; 6] = [
        // (Parameter, neue Größe, [(x, y) im Ergebnis ← (x, y) der Quelle])
        (json!({"angle": 90}), (2, 3), [(1, 0, (0, 0)), (0, 0, (0, 1))]),
        (json!({"angle": 180}), (3, 2), [(2, 1, (0, 0)), (0, 0, (2, 1))]),
        (json!({"angle": 270}), (2, 3), [(0, 2, (0, 0)), (1, 2, (0, 1))]),
        (json!({"angle": -90}), (2, 3), [(0, 2, (0, 0)), (1, 2, (0, 1))]),
        (json!({"angle": 0, "flip": "horizontal"}), (3, 2), [(2, 0, (0, 0)), (0, 1, (2, 1))]),
        (json!({"angle": 0, "flip": "vertical"}), (3, 2), [(0, 1, (0, 0)), (2, 0, (2, 1))]),
    ];
    for (p, size, checks) in cases {
        let mut it = still(src.clone());
        apply("rotate", p.clone(), &mut it).expect("rotate");
        assert_eq!(dims(&it), size, "{p}");
        let out = first(&it);
        for (x, y, (sx, sy)) in checks {
            assert_eq!(px(&out, x, y), px(&src, sx as usize, sy as usize), "{p} ({x},{y})");
        }
    }
    // 360° und 0° ohne Spiegeln: bitgleich
    for a in [0, 360, -360] {
        let mut it = still(src.clone());
        apply("rotate", json!({"angle": a}), &mut it).expect("rotate");
        assert_eq!(hash(&first(&it)), hash(&src));
    }
    // viermal 90° = Ausgangsbild
    let mut it = still(gradient(17, 9));
    for _ in 0..4 {
        apply("rotate", json!({"angle": 90}), &mut it).expect("rotate");
    }
    assert_eq!(hash(&first(&it)), hash(&gradient(17, 9)));
}

#[test]
fn rotate_free_angle_grows_and_fills_background() {
    let mut it = still(image(100, 50, |_, _| [0, 0, 255, 255]));
    apply("rotate", json!({"angle": 45}), &mut it).expect("rotate");
    let (w, h) = dims(&it);
    assert_eq!((w, h), (107, 107));
    let img = first(&it);
    assert_eq!(px(&img, 0, 0)[3], 0, "Ecke transparent");
    assert_eq!(px(&img, 53, 53), [0, 0, 255, 255], "Mitte bleibt");
    assert!(it.meta().is_some_and(|m| m.has_transparency));
    let mut it = still(image(100, 50, |_, _| [0, 0, 255, 255]));
    apply("rotate", json!({"angle": 30, "background": "#ffffff"}), &mut it).expect("rotate");
    let img = first(&it);
    assert_eq!(px(&img, 0, 0), [255, 255, 255, 255]);
    assert!(img.data().iter().all(|p| p[3] == 255), "deckend");
    // deterministisch
    let (a, b) = single_and_multi(|| {
        let mut it = still(gradient(80, 60));
        apply("rotate", json!({"angle": 12.5}), &mut it).expect("rotate");
        hash(&first(&it))
    });
    assert_eq!(a, b);
}

fn exif_tiff(orientation: u16) -> Vec<u8> {
    let mut t = b"Exif\0\0II*\0".to_vec();
    t.extend(8u32.to_le_bytes());
    t.extend(1u16.to_le_bytes());
    t.extend(0x0112u16.to_le_bytes());
    t.extend(3u16.to_le_bytes());
    t.extend(1u32.to_le_bytes());
    t.extend(orientation.to_le_bytes());
    t.extend([0, 0]);
    t.extend(0u32.to_le_bytes());
    t
}

#[test]
fn rotate_auto_exif_from_metadata_once() {
    let src = image(4, 2, |x, y| [(x * 60) as u8, (y * 120) as u8, 0, 255]);
    let mut it = still(src.clone());
    if let Some(m) = it.meta_mut() {
        m.exif = Some(exif_tiff(6));
    }
    apply("rotate", json!({"angle": 0}), &mut it).expect("rotate");
    assert_eq!(dims(&it), (2, 4), "EXIF 6 = 90° im Uhrzeigersinn");
    assert_eq!(px(&first(&it), 1, 0), px(&src, 0, 0));
    let exif = it.meta().and_then(|m| m.exif.clone()).expect("exif");
    assert_eq!(exif_orientation(&exif), Some(1), "Tag danach 1");
    // zweiter Schritt dreht nicht noch einmal
    apply("rotate", json!({"angle": 0}), &mut it).expect("rotate 2");
    assert_eq!(dims(&it), (2, 4));
    // autoExif aus: keine Wirkung
    let mut it = still(src.clone());
    if let Some(m) = it.meta_mut() {
        m.exif = Some(exif_tiff(3));
    }
    apply("rotate", json!({"angle": 0, "autoExif": false}), &mut it).expect("rotate");
    assert_eq!(hash(&first(&it)), hash(&src));
    // alle acht Werte: Größe vertauscht bei 5–8
    for v in 1..=8u16 {
        let mut it = still(src.clone());
        if let Some(m) = it.meta_mut() {
            m.exif = Some(exif_tiff(v));
        }
        apply("rotate", json!({"angle": 0}), &mut it).expect("rotate");
        assert_eq!(dims(&it), if v >= 5 { (2, 4) } else { (4, 2) }, "EXIF {v}");
    }
}

#[test]
fn rotate_auto_exif_never_rotates_twice() {
    // Die Datei sagt Ausrichtung 8, aber am Element sind keine EXIF-Daten (beim Laden schon angewendet
    // bzw. von metadata.edit entfernt) → der Schritt liest die Datei NICHT erneut, dreht nicht doppelt.
    let dir = std::env::temp_dir().join(format!("fp-batch4-exif-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let path = dir.join("photo.jpg");
    let tiff = &exif_tiff(8);
    let mut d = vec![0xFF, 0xD8, 0xFF, 0xE1];
    d.extend(((tiff.len() + 2) as u16).to_be_bytes());
    d.extend(tiff);
    d.extend([0xFF, 0xD9]);
    std::fs::write(&path, &d).expect("write");
    assert_eq!(exif_orientation(&d), Some(8), "Kopf lesbar");
    let src = image(4, 2, |x, _| [(x * 60) as u8, 0, 0, 255]);
    let mut it = still(src.clone());
    let mut inf = info(4, 2);
    inf.path = path;
    apply_with("rotate", json!({"angle": 0}), &mut it, &inf, false).expect("rotate");
    assert_eq!(hash(&first(&it)), hash(&src), "unverändert");
    // beim Laden angewendet (Ausrichtung 1 + Merker): ebenfalls keine Wirkung
    let mut it = still(src.clone());
    if let Some(m) = it.meta_mut() {
        m.exif = Some(exif_tiff(1));
        m.extra.insert(fp_batch::steps::geometry::EXIF_APPLIED_KEY.into(), json!(6));
    }
    apply("rotate", json!({"angle": 0}), &mut it).expect("rotate");
    assert_eq!(hash(&first(&it)), hash(&src));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn rotate_animation_all_frames() {
    let mut it = animation(vec![(gradient(30, 10), 40), (gradient(30, 10), 60)]);
    apply("rotate", json!({"angle": 90}), &mut it).expect("rotate");
    let Item::Animation(a) = &it else { panic!() };
    assert_eq!((a.width, a.height), (10, 30));
    assert_eq!(a.frames.iter().map(|f| f.delay_ms).collect::<Vec<_>>(), vec![40, 60]);
}

// ------------------------------------------------------------------ canvas

#[test]
fn canvas_modes() {
    let src = image(40, 20, |_, _| [9, 9, 9, 255]);
    let mut it = still(src.clone());
    apply("canvas", json!({"border": 10}), &mut it).expect("canvas");
    assert_eq!(dims(&it), (60, 40));
    let img = first(&it);
    assert_eq!(px(&img, 0, 0), [0, 0, 0, 0]);
    assert_eq!(px(&img, 10, 10), [9, 9, 9, 255]);
    assert!(it.meta().is_some_and(|m| m.has_transparency));
    // einzelne Seiten
    let mut it = still(src.clone());
    apply("canvas", json!({"border": 0, "left": 5, "bottom": 7}), &mut it).expect("canvas");
    assert_eq!(dims(&it), (45, 27));
    assert_eq!(px(&first(&it), 5, 0), [9, 9, 9, 255]);
    // auf Größe (kleiner = beschneiden), Anker unten rechts
    let mut it = still(gradient(40, 20));
    apply("canvas", json!({"mode": "toSize", "width": 30, "height": 30, "anchor": "bottomRight", "color": "#ffffff"}), &mut it).expect("canvas");
    assert_eq!(dims(&it), (30, 30));
    assert_eq!(px(&first(&it), 0, 0), [255, 255, 255, 255]);
    assert_eq!(px(&first(&it), 0, 10), px(&gradient(40, 20), 10, 0));
    // Seitenverhältnis und Quadrat
    let mut it = still(src.clone());
    apply("canvas", json!({"mode": "toAspect", "ratio": "1:1"}), &mut it).expect("canvas");
    assert_eq!(dims(&it), (40, 40));
    let mut it = still(src.clone());
    apply("canvas", json!({"mode": "toAspect", "ratio": "16:9"}), &mut it).expect("canvas");
    assert_eq!(dims(&it), (40, 23));
    let mut it = still(image(20, 50, |_, _| [1, 2, 3, 255]));
    apply("canvas", json!({"mode": "square", "color": "#ff0000"}), &mut it).expect("canvas");
    assert_eq!(dims(&it), (50, 50));
    assert_eq!(px(&first(&it), 0, 0), [255, 0, 0, 255]);
    assert!(!it.meta().is_some_and(|m| m.has_transparency));
    // Farbe hinter halbtransparenten Pixeln (gemischt)
    let mut it = still(image(2, 2, |_, _| [0, 0, 0, 128]));
    apply("canvas", json!({"border": 1, "color": "#ffffff"}), &mut it).expect("canvas");
    let p = px(&first(&it), 1, 1);
    assert_eq!(p[3], 255);
    assert!((126..=129).contains(&p[0]), "{p:?}");
}

#[test]
fn canvas_too_large_is_an_error() {
    let mut it = still(gradient(10, 10));
    assert!(apply("canvas", json!({"mode": "toSize", "width": 32768, "height": 32768}), &mut it).is_err());
    // 20 010² Pixel > 2²⁸
    assert!(apply("canvas", json!({"border": 10000}), &mut it).is_err());
    assert_eq!(dims(&it), (10, 10), "Element unverändert");
}

// ------------------------------------------------------------------ Lauf mit echten Dateien

#[test]
fn full_run_web_shrink_with_still_and_animation() {
    use fp_batch::recipe::{TargetKind, TargetSpec};
    let dir = std::env::temp_dir().join(format!("fp-batch4-run-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    let a = dir.join("foto.png");
    let (bytes, _) = fp_batch::output::encode(&still(gradient(800, 400)), fp_batch::OutputFormat::Png, &Default::default()).expect("png");
    std::fs::write(&a, bytes).expect("write");
    let g = dir.join("anim.gif");
    let anim = animation((0..3).map(|i| (image(60, 40, move |x, y| [(x * 4) as u8, (y * 6) as u8, i * 100, 255]), 100)).collect());
    let (bytes, _) = fp_batch::output::encode(&anim, fp_batch::OutputFormat::Gif, &Default::default()).expect("gif");
    std::fs::write(&g, bytes).expect("write");
    let orig = std::fs::read(&a).expect("read");
    let mut r = Recipe::named("web");
    r.add_step("rotate", json!({"angle": 90}));
    r.add_step("resize", json!({"mode": "longEdge", "size": 200}));
    r.add_step("canvas", json!({"border": 5, "color": "#ffffff"}));
    r.output.format = "same".into();
    r.output.target = TargetSpec { kind: TargetKind::Folder, path: Some(dir.join("out")), ..TargetSpec::default() };
    let items = fp_batch::collect(&[a.clone(), g.clone()], false, false, &[]).items;
    let rep = fp_batch::run(&r, &items, &fp_batch::RunOptions { threads: Some(2), ..Default::default() }).expect("run");
    assert_eq!(rep.totals.done, 2, "{:#?}", rep.items);
    let read = |p: &std::path::Path| fp_batch::item::decode_bytes(&p.to_string_lossy(), &std::fs::read(p).expect("read")).expect("decode");
    let out_a = read(&dir.join("out").join("foto.png"));
    assert_eq!(out_a.dimensions(), (110, 210), "gedreht, längste Seite 200, Rand 5");
    let out_g = read(&dir.join("out").join("anim.gif"));
    let Item::Animation(an) = &out_g else { panic!("Animation bleibt Animation") };
    assert_eq!(an.frames.len(), 3);
    assert_eq!((an.width, an.height), (50, 70), "klein: nur gedreht (nur verkleinern), Rand 5");
    assert!(an.frames.iter().all(|f| f.delay_ms == 100));
    assert_eq!(std::fs::read(&a).expect("read"), orig, "Original unverändert");
    let _ = std::fs::remove_dir_all(&dir);
}

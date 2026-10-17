//! Schritte der Gruppe „Farbe und Effekte“ (BATCH-4, SPEC-BATCH §3.4): Ergebnis-Pixel, Gleichheit
//! mit den Filtern des Registers, Golden-Hashes und Logikfälle (Vorgaben = bitgleich, Transparenz,
//! Animation, Vorschau ersetzt statt addiert, Determinismus).

mod batch4_common;

use batch4_common::*;
use fp_batch::item::Item;
use fp_batch::{CancelToken, OutputFormat, Pipeline, Recipe, StepContext, StepRegistry};
use fp_raster::filters::{BrightnessContrastParams, HueSaturationParams, UnsharpMaskParams};
use fp_raster::{Filter, PixelFilter, RgbaImage};
use serde_json::json;

fn photo() -> RgbaImage {
    image(48, 32, |x, y| {
        [
            (40 + x * 3) as u8,
            (60 + y * 4) as u8,
            ((x * y) % 200 + 20) as u8,
            255,
        ]
    })
}

fn translucent() -> RgbaImage {
    image(24, 16, |x, y| {
        [
            (x * 10) as u8,
            (y * 15) as u8,
            99,
            if x < 6 { 0 } else { (x * 10) as u8 },
        ]
    })
}

fn run(id: &str, p: serde_json::Value, img: &RgbaImage) -> RgbaImage {
    let mut it = still(img.clone());
    apply(id, p, &mut it).unwrap_or_else(|e| panic!("{id}: {e}"));
    first(&it)
}

#[test]
fn defaults_leave_images_bit_identical() {
    for img in [photo(), translucent()] {
        let h = hash(&img);
        for (id, p) in [
            ("adjust.basic", json!({})),
            ("adjust.levels", json!({})),
            ("adjust.curves", json!({})),
            ("filter.sharpen", json!({"amount": 0})),
            ("filter.blur", json!({"radius": 0})),
            ("filter.blur", json!({"kind": "motion", "radius": 0})),
        ] {
            assert_eq!(hash(&run(id, p.clone(), &img)), h, "{id} {p}");
        }
    }
}

#[test]
fn basic_matches_the_registry_filters() {
    let img = photo();
    let want = Filter::BrightnessContrast(BrightnessContrastParams {
        brightness: 30,
        contrast: -20,
    })
    .apply(&img, None);
    assert_eq!(
        hash(&run(
            "adjust.basic",
            json!({"brightness": 30, "contrast": -20}),
            &img
        )),
        hash(&want)
    );
    let want = Filter::HueSaturation(HueSaturationParams {
        hue: 40,
        saturation: 25,
        lightness: 0,
        colorize: false,
    })
    .apply(&img, None);
    assert_eq!(
        hash(&run(
            "adjust.basic",
            json!({"hue": 40, "saturation": 25}),
            &img
        )),
        hash(&want)
    );
    // Sättigung −100 = grau
    let g = run("adjust.basic", json!({"saturation": -100}), &img);
    assert!(
        g.data()
            .iter()
            .all(|p| p[0].abs_diff(p[1]) <= 1 && p[1].abs_diff(p[2]) <= 1)
    );
    // Gamma > 1 hellt Mitteltöne auf, Schwarz/Weiß bleiben
    let ramp = image(256, 1, |x, _| [x as u8, x as u8, x as u8, 255]);
    let r = run("adjust.basic", json!({"gamma": 2.0}), &ramp);
    assert_eq!(px(&r, 0, 0), [0, 0, 0, 255]);
    assert_eq!(px(&r, 255, 0), [255, 255, 255, 255]);
    assert!(px(&r, 128, 0)[0] > 170);
    // warm: Rot hoch, Blau runter; Tönung +: Grün runter
    let mid = image(1, 1, |_, _| [128, 128, 128, 255]);
    assert_eq!(
        px(
            &run("adjust.basic", json!({"temperature": 100}), &mid),
            0,
            0
        ),
        [168, 128, 88, 255]
    );
    assert_eq!(
        px(&run("adjust.basic", json!({"tint": 50}), &mid), 0, 0),
        [128, 108, 128, 255]
    );
}

#[test]
fn invert_twice_is_identity_and_keeps_alpha() {
    let img = translucent();
    let once = run("adjust.invert", json!({}), &img);
    assert!(
        once.data()
            .iter()
            .zip(img.data())
            .all(|(a, b)| a[3] == b[3] && a[0] == 255 - b[0])
    );
    assert_eq!(hash(&run("adjust.invert", json!({}), &once)), hash(&img));
}

#[test]
fn grayscale_sepia_duotone() {
    let img = photo();
    let g = run("adjust.grayscale", json!({}), &img);
    assert!(g.data().iter().all(|p| p[0] == p[1] && p[1] == p[2]));
    assert_eq!(
        px(&g, 0, 0)[0],
        ((299 * 40 + 587 * 60 + 114 * 20 + 500) / 1000) as u8
    );
    let s = run(
        "adjust.grayscale",
        json!({"style": "sepia"}),
        &image(1, 1, |_, _| [100, 100, 100, 255]),
    );
    assert_eq!(px(&s, 0, 0), [135, 120, 94, 255]);
    let bw = image(2, 1, |x, _| {
        if x == 0 {
            [0, 0, 0, 255]
        } else {
            [255, 255, 255, 77]
        }
    });
    let d = run(
        "adjust.grayscale",
        json!({"style": "duotone", "dark": "#102030", "light": "#f0e0d0"}),
        &bw,
    );
    assert_eq!(px(&d, 0, 0), [0x10, 0x20, 0x30, 255]);
    assert_eq!(px(&d, 1, 0), [0xf0, 0xe0, 0xd0, 77], "Alpha bleibt");
}

#[test]
fn auto_stretches_low_contrast_and_balances_white() {
    let flat = image(32, 32, |x, y| {
        let v = 100 + ((x + y) * 50 / 62) as u8;
        [v, v, v, 255]
    });
    let r = run("adjust.auto", json!({}), &flat);
    let (mn, mx) = r
        .data()
        .iter()
        .fold((255u8, 0u8), |(a, b), p| (a.min(p[0]), b.max(p[0])));
    assert!(mn <= 5 && mx >= 250, "gestreckt: {mn}..{mx}");
    // Blaustich → Kanalmittel gleichen sich an
    let blue = image(16, 16, |x, y| {
        [(80 + x * 4) as u8, (80 + y * 4) as u8, 200, 255]
    });
    let r = run(
        "adjust.auto",
        json!({"levels": false, "contrast": false, "whiteBalance": true}),
        &blue,
    );
    let mean = |img: &RgbaImage, c: usize| {
        img.data().iter().map(|p| p[c] as f64).sum::<f64>() / img.data().len() as f64
    };
    let spread = |img: &RgbaImage| (mean(img, 0) - mean(img, 2)).abs();
    assert!(
        spread(&r) < spread(&blue) / 4.0,
        "{} → {}",
        spread(&blue),
        spread(&r)
    );
    // alles aus = bitgleich
    assert_eq!(
        hash(&run(
            "adjust.auto",
            json!({"levels": false, "contrast": false}),
            &blue
        )),
        hash(&blue)
    );
}

#[test]
fn auto_animation_measures_once_for_all_frames() {
    // zwei Frames mit verschiedenen Tonwerten bekommen dieselbe Tabelle (kein Flackern)
    let a = image(8, 8, |_, _| [100, 100, 100, 255]);
    let b = image(8, 8, |_, _| [150, 150, 150, 255]);
    let mut it = animation(vec![(a, 50), (b, 50)]);
    apply(
        "adjust.auto",
        json!({"levels": true, "contrast": false}),
        &mut it,
    )
    .expect("auto");
    let Item::Animation(an) = &it else { panic!() };
    assert_eq!(px(&an.frames[0].image, 0, 0)[0], 0);
    assert_eq!(px(&an.frames[1].image, 0, 0)[0], 255);
}

#[test]
fn levels_and_curves() {
    let ramp = image(256, 1, |x, _| [x as u8, x as u8, x as u8, 255]);
    let r = run(
        "adjust.levels",
        json!({"inputLow": 50, "inputHigh": 200}),
        &ramp,
    );
    assert_eq!(px(&r, 50, 0)[0], 0);
    assert_eq!(px(&r, 200, 0)[0], 255);
    assert_eq!(px(&r, 30, 0)[0], 0);
    let inv = run(
        "adjust.curves",
        json!({"points": [[0, 255], [255, 0]]}),
        &ramp,
    );
    assert_eq!(px(&inv, 0, 0)[0], 255);
    assert_eq!(px(&inv, 255, 0)[0], 0);
    let obj = run(
        "adjust.curves",
        json!({"points": [{"x": 0, "y": 255}, {"x": 255, "y": 0}]}),
        &ramp,
    );
    assert_eq!(hash(&obj), hash(&inv));
    // alle Kanäle als JSON
    let lv = run(
        "adjust.levels",
        json!({"channels": {"rgb": {"source_low": 0, "source_high": 255, "dest_low": 0, "dest_high": 255, "gamma": 1.0}, "red": {"source_low": 0, "source_high": 255, "dest_low": 0, "dest_high": 0.0, "gamma": 1.0}, "green": {"source_low": 0, "source_high": 255, "dest_low": 0, "dest_high": 255, "gamma": 1.0}, "blue": {"source_low": 0, "source_high": 255, "dest_low": 0, "dest_high": 255, "gamma": 1.0}}}),
        &ramp,
    );
    assert!(lv.data().iter().all(|p| p[0] == 0));
    // ungültige Angaben → Validierungsfehler
    assert!(!errors("adjust.curves", json!({"points": [[1, 2]]})).is_empty());
    assert!(!errors("adjust.curves", json!({"points": "x"})).is_empty());
    assert!(!errors("adjust.levels", json!({"channels": {"rgb": 3}})).is_empty());
    assert!(errors("adjust.levels", json!({"inputLow": 10})).is_empty());
}

#[test]
fn sharpen_and_blur_use_the_registry_filters_and_keep_size() {
    let img = photo();
    let want = Filter::UnsharpMask(UnsharpMaskParams {
        radius: 1.0,
        amount: 60,
        threshold: 0,
        ..UnsharpMaskParams::default()
    })
    .apply(&img, None);
    let got = run("filter.sharpen", json!({}), &img);
    assert_eq!(hash(&got), hash(&want.cropped(img.bounds())));
    for kind in ["gaussian", "motion", "radial", "zoom"] {
        let r = run("filter.blur", json!({"kind": kind, "radius": 3}), &img);
        assert_eq!((r.width(), r.height()), (48, 32), "{kind}");
        assert_eq!(r.origin(), (0, 0));
        assert_ne!(hash(&r), hash(&img), "{kind} wirkt");
    }
}

#[test]
fn color_golden_hashes() {
    let img = photo();
    let got: Vec<u64> = [
        ("adjust.auto", json!({"whiteBalance": true})),
        ("adjust.basic", json!({"brightness": 20, "contrast": 30, "saturation": -40, "gamma": 1.4, "temperature": 25})),
        ("adjust.grayscale", json!({"style": "sepia"})),
        ("filter.sharpen", json!({"amount": 150, "radius": 2})),
        ("filter.blur", json!({"radius": 2.5})),
        ("effect.live", json!({"effect": "dropShadow"})),
    ]
    .iter()
    .map(|(id, p)| {
        let h = hash(&run(id, p.clone(), &img));
        println!("{id} {p} -> {h:#018x}");
        h
    })
    .collect();
    assert_eq!(got, GOLDEN_COLOR.to_vec());
}

/// Golden-Hashes (FNV-1a) für das Testbild `photo()`; bei bewusster Änderung neu erzeugen
/// (`-- --nocapture` zeigt die Werte).
const GOLDEN_COLOR: [u64; 6] = [
    0x944abc292ffd4b0b,
    0x414109921af93540,
    0x72646409df3438d6,
    0x31adf846d9f5b0cb,
    0x6b855cd3608dca78,
    0xed4133a9856f8b11,
];

#[test]
fn color_steps_are_deterministic_across_thread_counts() {
    let img = gradient(211, 97);
    for (id, p) in [
        ("adjust.basic", json!({"brightness": 10, "gamma": 0.7})),
        ("filter.blur", json!({"radius": 4})),
        ("effect.live", json!({"effect": "glow"})),
    ] {
        let (a, b) = single_and_multi(|| hash(&run(id, p.clone(), &img)));
        assert_eq!(a, b, "{id}");
    }
}

// ------------------------------------------------------------------ effect.live

#[test]
fn live_effect_expands_canvas_or_keeps_size() {
    let obj = image(40, 30, |x, y| {
        if (5..35).contains(&x) && (5..25).contains(&y) {
            [200, 30, 30, 255]
        } else {
            [0, 0, 0, 0]
        }
    });
    let grown = run("effect.live", json!({"effect": "dropShadow"}), &obj);
    assert!(grown.width() >= 40 && grown.height() >= 30);
    assert_eq!(grown.origin(), (0, 0));
    assert_ne!(hash(&grown), hash(&obj), "Schatten sichtbar");
    let same = run(
        "effect.live",
        json!({"effect": "dropShadow", "expandCanvas": false}),
        &obj,
    );
    assert_eq!((same.width(), same.height()), (40, 30));
    // ein Glühen am Bildrand vergrößert die Fläche
    let full = image(20, 20, |_, _| [0, 0, 255, 255]);
    let g = run(
        "effect.live",
        json!({"effect": "glow", "params": {"GlowWidth": "6"}}),
        &full,
    );
    assert!(
        g.width() > 20 && g.height() > 20,
        "{}x{}",
        g.width(),
        g.height()
    );
    // jeder Effekt der Liste rechnet ohne Fehler
    for e in fp_batch::steps::adjust::LIVE_EFFECTS {
        let r = run("effect.live", json!({"effect": e}), &obj);
        assert!(r.width() >= 40, "{e}");
    }
    // unbekannter Effekt → Validierungsfehler
    assert!(!errors("effect.live", json!({"effect": "sparkles"})).is_empty());
    assert!(!errors("effect.live", json!({"params": [1, 2]})).is_empty());
}

#[test]
fn live_effect_in_preview_replaces_instead_of_adding() {
    // §4b.2: dieselbe Vorschau zweimal (nach Parameteränderung zurück) = ein Effekt, kein zweiter
    let obj = image(30, 30, |x, y| {
        if (8..22).contains(&x) && (8..22).contains(&y) {
            [0, 0, 0, 255]
        } else {
            [0, 0, 0, 0]
        }
    });
    let mut r = Recipe::named("t");
    let i = r.add_step("effect.live", json!({"effect": "dropShadow"}));
    let reg = StepRegistry::global();
    let run_recipe = |r: &Recipe| {
        let pipe = Pipeline::new(r, &reg).expect("pipe");
        let mut it = still(obj.clone());
        let inf = info(30, 30);
        let cancel = CancelToken::new();
        let mut ctx = StepContext::new(0, 1, &inf, &cancel, OutputFormat::Png);
        ctx.preview = true;
        pipe.apply(&mut it, &mut ctx).expect("apply");
        hash(&first(&it))
    };
    let once = run_recipe(&r);
    for _ in 0..50 {
        r.set_param(i, "params", json!({"ShadowDistance": "12"}));
        r.set_param(i, "params", json!({}));
    }
    assert_eq!(r.steps.len(), 1);
    assert_eq!(run_recipe(&r), once);
}

#[test]
fn live_effect_animation_frames_share_one_size() {
    let a = image(20, 20, |x, _| {
        if x < 10 {
            [255, 0, 0, 255]
        } else {
            [0, 0, 0, 0]
        }
    });
    let b = image(20, 20, |x, _| {
        if x >= 10 {
            [0, 255, 0, 255]
        } else {
            [0, 0, 0, 0]
        }
    });
    let mut it = animation(vec![(a, 70), (b, 90)]);
    apply("effect.live", json!({"effect": "dropShadow"}), &mut it).expect("fx");
    let Item::Animation(an) = &it else { panic!() };
    assert_eq!(an.frames.len(), 2);
    let s0 = (an.frames[0].image.width(), an.frames[0].image.height());
    assert_eq!(
        s0,
        (an.frames[1].image.width(), an.frames[1].image.height())
    );
    assert_eq!((an.width as usize, an.height as usize), s0);
    assert_eq!(an.frames[1].delay_ms, 90);
}

// ------------------------------------------------------------------ filter.registry

#[test]
fn filter_registry_lists_filters_and_runs_them_like_the_menu() {
    let list = fp_batch::steps::script::filter_commands();
    assert!(list.iter().any(|c| c == "filter.blur.gaussian"));
    assert!(
        list.iter()
            .all(|c| c.starts_with("filter.") && c != "filter.repeat")
    );
    let img = photo();
    // Umkehren über das Register = Umkehren direkt
    let r = run(
        "filter.registry",
        json!({"command": "filter.adjustColor.invert"}),
        &img,
    );
    assert_eq!(hash(&r), hash(&Filter::Invert.apply(&img, None)));
    // Gauß mit Parametern wie der Dialog: gleich wie filter.blur (Toleranz für den Rand)
    let a = run(
        "filter.registry",
        json!({"command": "filter.blur.gaussian", "params": {"radius": 2.0}}),
        &img,
    );
    let b = run("filter.blur", json!({"radius": 2.0}), &img);
    assert!(max_diff(&a, &b) <= 2, "max diff {}", max_diff(&a, &b));
    // Prüfung: unbekannt, kein Filter
    assert!(!errors("filter.registry", json!({"command": "filter.nope"})).is_empty());
    assert!(!errors("filter.registry", json!({"command": "edit.copy"})).is_empty());
    assert!(
        errors(
            "filter.registry",
            json!({"command": "filter.sharpen.sharpen"})
        )
        .is_empty()
    );
}

#[test]
fn filter_registry_on_transparent_image_keeps_size_and_alpha() {
    let img = translucent();
    let r = run(
        "filter.registry",
        json!({"command": "filter.adjustColor.invert"}),
        &img,
    );
    assert_eq!((r.width(), r.height()), (24, 16));
    for (a, b) in r.data().iter().zip(img.data()) {
        if b[3] >= 2 {
            assert_eq!(a[3], b[3]);
        }
    }
}

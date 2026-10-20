//! GIF-Optimierer (BATCH-8, SPEC-BATCH §3.10/§5.6): Rundreise über den GIF-Dekodierer von fp-io,
//! Bilddifferenz/Zuschnitt/Entsorgung, zeitliche Stabilität, Verlust, Palette auto, Zielgröße,
//! Golden-Werte (`tools\golden\batch\gif\golden.json`) und Logikfälle nach AGENTEN_REGELN §4b.

mod gifopt_clips;

use fp_batch::item::{Animation, Frame, Item, Meta};
use fp_batch::recipe::{TargetKind, TargetSpec};
use fp_batch::steps::gifopt::{self, GifOptions, Source};
use fp_batch::{OutputFormat, Recipe, RunOptions, StepRegistry, executor};
use fp_raster::{IRect, RgbaImage};
use gifopt_clips::*;
use serde_json::{Map, Value, json};
use std::path::PathBuf;

fn opts(v: Value) -> Map<String, Value> {
    v.as_object().cloned().unwrap_or_default()
}

fn run(clip: &Clip, v: Value) -> gifopt::Optimized {
    gifopt::optimize(&clip.item(), &opts(v)).expect("optimize")
}

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fp-batch-gifopt-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("temp");
    d
}

/// Wenige flache Farben ohne Verlust → Pixel für Pixel gleich (auch Transparenz, Entsorgung).
#[test]
fn lossless_settings_round_trip_exactly() {
    for clip in [pixelart(64, 48, 12), cutout(60, 40, 14), screen(96, 64, 8)] {
        for palette in ["global", "perFrame", "auto"] {
            let r = run(
                &clip,
                json!({"quality": 100, "lossy": 0, "dither": "none", "palette": palette, "diffThreshold": 0}),
            );
            let (w, h, dec) = decode(&r.bytes);
            assert_eq!((w, h), (clip.w, clip.h));
            let src: Vec<(Vec<Px>, u32)> = clip
                .frames
                .iter()
                .map(|f| (f.clone(), clip.delay_ms))
                .collect();
            let a = timeline(&src, 100);
            let b = timeline(&dec, 100);
            assert_eq!(a.len(), b.len(), "{} {palette}: Dauer", clip.name);
            for (k, (x, y)) in a.iter().zip(&b).enumerate() {
                for (i, (p, q)) in x.iter().zip(y.iter()).enumerate() {
                    let same = if p[3] < 128 { q[3] < 128 } else { p == q };
                    assert!(
                        same,
                        "{} {palette}: Zeit {k} Pixel {i}: {p:?} vs {q:?}",
                        clip.name
                    );
                }
            }
        }
    }
}

#[test]
fn diff_transparency_and_crop_shrink_frames() {
    let clip = screen(160, 120, 16);
    let full = run(
        &clip,
        json!({"diffTransparency": false, "cropChanged": false, "lossy": 0}),
    );
    let diff = run(&clip, json!({"lossy": 0}));
    assert!(
        diff.bytes.len() * 2 < full.bytes.len(),
        "{} vs {}",
        diff.bytes.len(),
        full.bytes.len()
    );
    let st = &diff.stats;
    // ab Frame 1 nur kleine Rechtecke (Zeiger, Eingabezeile, Balken)
    for f in &st.frames[1..] {
        assert!(f.rect.w * f.rect.h < 160 * 120 / 2, "{:?}", f.rect);
    }
    assert!(
        full.stats
            .frames
            .iter()
            .all(|f| (f.rect.w, f.rect.h) == (160, 120))
    );
}

#[test]
fn alpha_animation_uses_background_disposal_correctly() {
    let clip = cutout(80, 40, 20);
    let r = run(&clip, json!({"lossy": 0, "dither": "none"}));
    assert!(
        r.stats.frames.iter().any(|f| f.disposal == 2),
        "{:?}",
        r.stats.frames
    );
    let (_, _, dec) = decode(&r.bytes);
    assert_eq!(dec.len(), clip.frames.len());
    for (k, (src, (got, _))) in clip.frames.iter().zip(&dec).enumerate() {
        for (p, q) in src.iter().zip(got) {
            assert_eq!(p[3] >= 128, q[3] >= 128, "Frame {k}");
            if p[3] >= 128 {
                assert_eq!(p, q, "Frame {k}");
            }
        }
    }
}

/// Statische Bereiche bleiben über alle Frames exakt gleich (kein Flimmern, kein Kriechen).
#[test]
fn static_regions_do_not_flicker() {
    let (w, h, n) = (96usize, 64usize, 10usize);
    let frames: Vec<Vec<Px>> = (0..n)
        .map(|f| {
            (0..w * h)
                .map(|i| {
                    let (x, y) = (i % w, i / w);
                    if y >= 40 && x.abs_diff(f * 8) < 6 {
                        [250, 30, 30, 255]
                    } else {
                        [(x * 255 / w) as u8, (y * 4) as u8, 140, 255]
                    }
                })
                .collect()
        })
        .collect();
    let clip = Clip {
        name: "grad",
        w,
        h,
        frames,
        delay_ms: 100,
    };
    for dither in ["temporal", "blueNoise", "ordered", "floydSteinberg"] {
        let r = run(&clip, json!({"dither": dither, "colors": 32}));
        let (_, _, dec) = decode(&r.bytes);
        for (got, _) in &dec[1..] {
            for y in 0..36 {
                for x in 0..w {
                    assert_eq!(got[y * w + x], dec[0].0[y * w + x], "{dither}: ({x},{y})");
                }
            }
        }
    }
}

#[test]
fn lossy_makes_smaller_and_keeps_quality() {
    for clip in [pan(120, 80, 10), cartoon(120, 80, 12)] {
        let l0 = run(&clip, json!({"lossy": 0}));
        let l80 = run(&clip, json!({"lossy": 80}));
        let l200 = run(&clip, json!({"lossy": 200}));
        if clip.name == "pan" {
            // Korn: Verlust macht deutlich kleiner
            assert!(
                l80.bytes.len() < l0.bytes.len(),
                "{}: {} vs {}",
                clip.name,
                l80.bytes.len(),
                l0.bytes.len()
            );
            assert!(l200.bytes.len() < l80.bytes.len());
        } else {
            // Flächen/Verläufe: Verlust bringt wenig, macht aber nie größer (BATCH-8b: je Teilbild das kleinere)
            assert!(
                l80.bytes.len() <= l0.bytes.len() && l200.bytes.len() <= l0.bytes.len(),
                "{}: {} {} {}",
                clip.name,
                l0.bytes.len(),
                l80.bytes.len(),
                l200.bytes.len()
            );
        }
        let (s0, s80) = (mean_ssim(&clip, &l0.bytes), mean_ssim(&clip, &l80.bytes));
        assert!(s0 > 0.9 && s80 > 0.8, "{}: {s0} {s80}", clip.name);
        assert!(s80 <= s0 + 0.01);
    }
}

/// Getrennte Änderungen als Teilbilder (BATCH-8b): zu jedem Frame-Zeitpunkt exakt das Quellbild,
/// Gesamtdauer gleich, Teilbilder ≥ 20 ms, kleiner als ohne Aufteilung; `splitFrames: false` und
/// „best“ teilen nicht; ein bewegtes Objekt (alte + neue Lage) wird nie getrennt.
#[test]
fn split_frames_are_exact_at_frame_times() {
    let clip = screen(160, 120, 12);
    let lossless = json!({"quality": 90, "lossy": 0, "dither": "none", "diffThreshold": 0});
    let split = run(&clip, lossless.clone());
    let mut whole_opts = lossless.clone();
    whole_opts["splitFrames"] = json!(false);
    let whole = run(&clip, whole_opts);
    assert!(
        split.stats.frames.len() > clip.frames.len(),
        "{:?}",
        split.stats.frames
    );
    assert_eq!(whole.stats.frames.len(), clip.frames.len());
    assert!(
        split.bytes.len() * 10 < whole.bytes.len() * 8,
        "{} vs {}",
        split.bytes.len(),
        whole.bytes.len()
    );
    assert!(split.stats.frames.iter().all(|f| f.delay_cs >= 2));
    let total = |r: &gifopt::Optimized| {
        r.stats
            .frames
            .iter()
            .map(|f| f.delay_cs as u32)
            .sum::<u32>()
    };
    assert_eq!(total(&split), clip.delay_ms / 10 * clip.frames.len() as u32);
    assert_eq!(total(&split), total(&whole));
    // zu jedem Frame-Zeitpunkt exakt das Quellbild
    let (_, _, dec) = decode(&split.bytes);
    let tl = timeline(&dec, clip.delay_ms);
    for (k, src) in clip.frames.iter().enumerate() {
        let got = tl[k * (clip.delay_ms / 10) as usize];
        assert!(
            got == src,
            "Frame {k} zur Zeit {} ms nicht exakt",
            k as u32 * clip.delay_ms
        );
    }
    assert!(full_ssim(&clip, &split.bytes) > 0.995);
    // „best“ teilt nicht
    assert_eq!(
        run(&clip, json!({"preset": "best"})).stats.frames.len(),
        clip.frames.len()
    );
    // bewegtes Objekt: nie getrennt (Ball alte + neue Lage)
    let ball = cartoon(160, 120, 10);
    assert_eq!(
        run(&ball, json!({"preset": "balanced"})).stats.frames.len(),
        ball.frames.len()
    );
    // Option 50× umschalten = einmal mit dem Endwert
    let mut m = Map::new();
    for k in 0..50 {
        m.insert("splitFrames".into(), json!(k % 2 == 0));
    }
    for (k, v) in lossless.as_object().cloned().unwrap_or_default() {
        m.insert(k, v);
    }
    assert_eq!(
        gifopt::optimize(&clip.item(), &m).expect("50x").bytes,
        whole.bytes
    );
}

/// Inhalt steuert die Voreinstellung (BATCH-8b): Grafik wenige Farben/wenig Verlust, Foto mehr Farben
/// und mehr Verlust; kleine Teilbilder bekommen eine kleinere LZW-Codelänge.
#[test]
fn content_class_and_code_size() {
    let g = run(&cartoon(160, 120, 8), json!({"preset": "balanced"}));
    let p = run(&pan(160, 120, 8), json!({"preset": "balanced"}));
    let s = run(&screen(160, 120, 8), json!({"preset": "balanced"}));
    assert!(
        g.stats.photo < 0.01 && s.stats.photo < 0.01 && p.stats.photo > 0.99,
        "{} {} {}",
        g.stats.photo,
        s.stats.photo,
        p.stats.photo
    );
    assert!(g.stats.global_colors < p.stats.global_colors && g.stats.lossy < p.stats.lossy);
    assert!(g.stats.global_colors <= 44 && p.stats.global_colors > 64);
    // Grafik mit 40+ Farben: Ball-Teilbilder brauchen nur die vorderen Indizes
    let full = g.stats.frames[0].min_code_size;
    assert!(
        g.stats.frames[1..].iter().any(|f| f.min_code_size < full),
        "{:?}",
        g.stats.frames
    );
    // ausdrücklich gesetzte Farben gelten unabhängig vom Inhalt
    let fixed = run(
        &cartoon(160, 120, 8),
        json!({"preset": "balanced", "colors": 128}),
    );
    assert!(fixed.stats.global_colors > 64);
}

/// Größe fällt mit der Qualität (Voraussetzung der Zielgröße), für Grafik und Foto.
#[test]
fn size_falls_with_quality() {
    for clip in [cartoon(128, 96, 8), pan(96, 72, 6)] {
        let sizes: Vec<usize> = [10, 30, 50, 70, 90]
            .iter()
            .map(|q| run(&clip, json!({"quality": q})).bytes.len())
            .collect();
        assert!(
            sizes.windows(2).all(|w| w[0] <= w[1] + w[1] / 50),
            "{}: {sizes:?}",
            clip.name
        );
    }
}

#[test]
fn auto_palette_switches_on_scene_change_only() {
    let clip = scene_change(64, 48, 8);
    let auto = run(&clip, json!({"palette": "auto", "colors": 64}));
    assert!(
        auto.stats.frames.iter().any(|f| f.local_palette),
        "{:?}",
        auto.stats.frames
    );
    assert!(!auto.stats.frames[0].local_palette);
    let global = run(&clip, json!({"palette": "global", "colors": 64}));
    assert!(global.stats.frames.iter().all(|f| !f.local_palette));
    assert!(mean_ssim(&clip, &auto.bytes) > mean_ssim(&clip, &global.bytes));
    // ruhige Animation ohne Szenenwechsel: keine lokale Palette
    let calm = run(&cartoon(64, 48, 8), json!({"palette": "auto"}));
    assert!(calm.stats.frames.iter().all(|f| !f.local_palette));
    let per = run(&clip, json!({"palette": "perFrame", "colors": 64}));
    assert!(per.stats.frames[1..].iter().all(|f| f.local_palette));
}

#[test]
fn target_size_is_reached_or_reported() {
    let clip = pan(128, 96, 12);
    let free = run(&clip, json!({}));
    let limit_kb = (free.bytes.len() / 1024 * 6 / 10).max(1);
    let r = run(&clip, json!({"maxKB": limit_kb}));
    assert!(
        r.reached && r.bytes.len() <= limit_kb * 1024,
        "{} > {}",
        r.bytes.len(),
        limit_kb * 1024
    );
    assert!(r.encodes <= gifopt::size::MAX_ENCODES);
    // Schätzung über ln(Größe) statt Halbierung: wenige Kodierungen, nah am Ziel (BATCH-8b)
    assert!(
        r.encodes <= 6 && r.frame_step == 1 && r.scale == 1.0,
        "{} Kodierungen, Schritt {}, Faktor {}",
        r.encodes,
        r.frame_step,
        r.scale
    );
    assert!(r.quality < 80 || r.scale < 1.0 || r.frame_step > 1);
    // schon klein genug → eine Kodierung, Qualität unverändert
    let big = run(&clip, json!({"maxKB": 100_000}));
    assert_eq!(
        (big.encodes, big.quality, big.bytes.len()),
        (1, 80, free.bytes.len())
    );
    // unerreichbar → kleinstes Ergebnis, gemeldet, höchstens 8 Kodierungen
    let tiny = run(&clip, json!({"maxKB": 1}));
    assert!(
        tiny.encodes <= 8 && tiny.reached == (tiny.bytes.len() <= 1024),
        "{} {}",
        tiny.reached,
        tiny.bytes.len()
    );
    assert!(tiny.bytes.len() < free.bytes.len() / 3);
    assert!(!decode(&tiny.bytes).2.is_empty());
    // Rauschen lässt sich nicht auf 1 KB bringen → nicht erreicht, kleinstes Ergebnis
    let noise = Clip {
        name: "noise",
        w: 200,
        h: 200,
        frames: (0..30u32)
            .map(|f| {
                (0..40_000u32)
                    .map(|i| {
                        let v = (i.wrapping_mul(2_654_435_761) ^ f.wrapping_mul(40_503))
                            .rotate_left(i % 13);
                        [v as u8, (v >> 8) as u8, (v >> 16) as u8, 255]
                    })
                    .collect()
            })
            .collect(),
        delay_ms: 100,
    };
    let r = run(&noise, json!({"maxKB": 1}));
    assert!(
        !r.reached && r.encodes <= 8 && r.bytes.len() > 1024 && r.scale <= 0.1 + 1e-6,
        "{} {} {}",
        r.reached,
        r.encodes,
        r.bytes.len()
    );
}

#[test]
fn deterministic_and_idempotent_options() {
    let clip = cartoon(80, 60, 8);
    let a = run(&clip, json!({"preset": "smallest"}));
    let b = run(&clip, json!({"preset": "smallest"}));
    assert_eq!(a.bytes, b.bytes);
    // Werte 50× setzen (wie ein Schieber) = einmal mit dem Endwert
    let mut m = Map::new();
    for q in 1..=50 {
        m.insert("quality".into(), json!(q + 20));
        m.insert("lossy".into(), json!(q));
    }
    let once =
        gifopt::optimize(&clip.item(), &opts(json!({"quality": 70, "lossy": 50}))).expect("once");
    assert_eq!(
        gifopt::optimize(&clip.item(), &m).expect("50x").bytes,
        once.bytes
    );
    // Voreinstellungen ordnen sich nach Größe
    let best = run(&clip, json!({"preset": "best"}));
    let bal = run(&clip, json!({"preset": "balanced"}));
    assert!(a.bytes.len() < bal.bytes.len() && bal.bytes.len() < best.bytes.len());
    assert!(mean_ssim(&clip, &best.bytes) >= mean_ssim(&clip, &a.bytes));
    // Optionen-Lesen: ungültige Werte ignoriert, Zahlen begrenzt
    let o = GifOptions::from_map(&opts(
        json!({"colors": 9999, "dither": "foo", "maxKB": 0, "lossy": "300"}),
    ));
    assert_eq!(
        (o.colors, o.dither, o.max_kb, o.lossy),
        (Some(256), None, None, Some(200))
    );
}

#[test]
fn merge_similar_keeps_duration() {
    let mut clip = cartoon(48, 32, 6);
    let dup = clip.frames[2].clone();
    clip.frames.insert(3, dup.clone());
    clip.frames.insert(3, dup);
    let r = run(&clip, json!({"mergeSimilar": true}));
    assert_eq!(r.stats.frames.len(), 6);
    let total: u32 = r.stats.frames.iter().map(|f| f.delay_cs as u32).sum();
    assert_eq!(total * 10, clip.delay_ms * 8);
    let r2 = run(&clip, json!({"mergeSimilar": false}));
    assert_eq!(r2.stats.frames.len(), 8);
}

#[test]
fn edge_cases() {
    // Standbild, 1×1, Animation mit 1 Frame, Frames anderer Größe, leere Animation, Matte
    let one = Item::Still(fp_batch::item::Still {
        image: RgbaImage::from_raw(IRect::new(0, 0, 1, 1), vec![[9, 8, 7, 255]]).expect("img"),
        meta: Meta::default(),
    });
    let r = gifopt::optimize(&one, &Map::new()).expect("1x1");
    let (w, h, dec) = decode(&r.bytes);
    assert_eq!((w, h, dec[0].0[0]), (1, 1, [9, 8, 7, 255]));
    let f = |w: i32, h: i32, c: Px| Frame {
        image: RgbaImage::from_raw(IRect::new(0, 0, w, h), vec![c; (w * h) as usize]).expect("img"),
        delay_ms: 50,
    };
    let mixed = Item::Animation(Animation {
        width: 10,
        height: 8,
        frames: vec![
            f(10, 8, [200, 0, 0, 255]),
            f(4, 4, [0, 200, 0, 255]),
            f(20, 20, [0, 0, 200, 255]),
        ],
        loop_count: 0,
        meta: Meta::default(),
    });
    let r = gifopt::optimize(&mixed, &opts(json!({"dither": "none"}))).expect("mixed");
    let (w, h, dec) = decode(&r.bytes);
    assert_eq!((w, h, dec.len()), (10, 8, 3));
    assert_eq!(dec[1].0[0], [0, 200, 0, 255]);
    assert_eq!(dec[1].0[9], [0, 0, 0, 0]);
    assert_eq!(dec[2].0[79], [0, 0, 200, 255]);
    let single = Item::Animation(Animation {
        width: 10,
        height: 8,
        frames: vec![f(10, 8, [1, 2, 3, 255])],
        loop_count: 0,
        meta: Meta::default(),
    });
    assert_eq!(
        decode(
            &gifopt::optimize(&single, &Map::new())
                .expect("single")
                .bytes
        )
        .2
        .len(),
        1
    );
    let empty = Item::Animation(Animation {
        width: 10,
        height: 8,
        frames: vec![],
        loop_count: 0,
        meta: Meta::default(),
    });
    assert!(gifopt::optimize(&empty, &Map::new()).is_err());
    // Transparenz aus → alles deckend auf Matte
    let clip = cutout(40, 30, 4);
    let r = run(
        &clip,
        json!({"transparency": false, "background": "#00ff00"}),
    );
    let (_, _, dec) = decode(&r.bytes);
    assert!(dec.iter().all(|(px, _)| px.iter().all(|p| p[3] == 255)));
    assert_eq!(dec[0].0[39], [0, 255, 0, 255]);
    // Quelle aus einem Element mit Dokument geht nicht verloren
    assert!(Source::from_item(&Item::FileOnly("x.bin".into())).is_err());
}

#[test]
fn registered_as_gif_encoder_and_used_by_batch_run() {
    let reg = StepRegistry::global();
    assert!(reg.contains(gifopt::OUTPUT_GIF) && reg.contains(gifopt::STABILIZE));
    let clip = cartoon(40, 30, 5);
    let item = clip.item();
    let o = opts(json!({"quality": 70}));
    let (bytes, _) = fp_batch::output::encode(&item, OutputFormat::Gif, &o).expect("encode");
    assert_eq!(bytes, gifopt::encode_item(&item, &o).expect("direct"));
    // Lauf: Rezept „Speichern als GIF“ mit Optionen, Datei dekodierbar
    let dir = temp("run");
    let src = dir.join("clip.gif");
    std::fs::write(&src, &bytes).expect("write");
    let mut r = Recipe::named("gif");
    r.output.format = "gif".into();
    r.output.options = opts(json!({"preset": "smallest", "maxKB": 50}));
    r.output.target = TargetSpec {
        kind: TargetKind::Folder,
        path: Some(dir.join("out")),
        ..TargetSpec::default()
    };
    r.add_step(gifopt::STABILIZE, json!({"threshold": 4}));
    let info = fp_batch::probe(&src).expect("probe");
    let rep = executor::run(
        &r,
        &[info],
        &RunOptions {
            threads: Some(1),
            ..RunOptions::default()
        },
    )
    .expect("run");
    assert_eq!(rep.totals.done, 1, "{:#?}", rep.items);
    let out = std::fs::read(dir.join("out").join("clip.gif")).expect("out");
    assert!(decode(&out).2.len() >= 2);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn stabilize_step_is_idempotent() {
    let reg = StepRegistry::global();
    let step = reg.get(gifopt::STABILIZE).expect("step");
    let clip = pan(40, 30, 4);
    let info = fp_batch::ItemInfo {
        path: "x.gif".into(),
        root: None,
        kind: fp_batch::ItemKind::Animation,
        format: "gif".into(),
        bytes: 0,
        width: 40,
        height: 30,
        frames: 4,
        modified: None,
        exif: None,
    };
    let cancel = fp_batch::CancelToken::new();
    let mut ctx = fp_batch::StepContext::new(0, 1, &info, &cancel, OutputFormat::Gif);
    let mut once = clip.item();
    step.apply(&mut once, &json!({"threshold": 6}), &mut ctx)
        .expect("apply");
    let mut many = clip.item();
    for _ in 0..50 {
        step.apply(&mut many, &json!({"threshold": 6}), &mut ctx)
            .expect("apply");
    }
    let px = |i: &Item| match i {
        Item::Animation(a) => a
            .frames
            .iter()
            .map(|f| f.image.data().to_vec())
            .collect::<Vec<_>>(),
        _ => vec![],
    };
    assert_eq!(px(&once), px(&many));
    assert_ne!(px(&once), px(&clip.item()), "Rauschen wurde eingefroren");
    assert!(!step.summary(&json!({"threshold": 6})).is_empty());
}

/// Golden-Werte: Größe ± 5 %, SSIM nicht schlechter als gespeichert − 0,005 (FP_BLESS=1 schreibt neu).
#[test]
fn golden_sizes_and_quality() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/golden/batch/gif/golden.json");
    let clips = [screen(160, 120, 12), pan(128, 96, 12), cartoon(128, 96, 12)];
    let mut now = Map::new();
    for clip in &clips {
        for preset in ["smallest", "balanced", "best"] {
            let r = run(clip, json!({"preset": preset}));
            let ssim = mean_ssim(clip, &r.bytes);
            now.insert(
                format!("{}/{preset}", clip.name),
                json!({"bytes": r.bytes.len(), "ssim": (ssim * 10000.0).round() / 10000.0}),
            );
        }
    }
    let bless = std::env::var_os("FP_BLESS").is_some() || !path.exists();
    if bless {
        std::fs::create_dir_all(path.parent().expect("dir")).expect("dir");
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&Value::Object(now)).expect("json") + "\n",
        )
        .expect("write golden");
        return;
    }
    let golden: Map<String, Value> =
        serde_json::from_slice(&std::fs::read(&path).expect("golden")).expect("golden json");
    for (k, g) in &golden {
        let n = now.get(k).unwrap_or_else(|| panic!("{k} fehlt"));
        let (gb, nb) = (
            g["bytes"].as_f64().unwrap_or(0.0),
            n["bytes"].as_f64().unwrap_or(0.0),
        );
        assert!((nb - gb).abs() <= gb * 0.05, "{k}: {nb} B statt {gb} B");
        assert!(
            n["ssim"].as_f64().unwrap_or(0.0) >= g["ssim"].as_f64().unwrap_or(1.0) - 0.005,
            "{k}: SSIM {} < {}",
            n["ssim"],
            g["ssim"]
        );
    }
}

/// Langsame Blende (gleichgerichtete kleine Änderungen) ist Inhalt: kein Glätten, kein Verschlucken.
#[test]
fn slow_fade_is_kept() {
    let frames: Vec<Vec<Px>> = (0..4u8).map(|k| vec![[k * 20, 50, 100, 255]; 24]).collect();
    let clip = Clip {
        name: "fade",
        w: 6,
        h: 4,
        frames,
        delay_ms: 100,
    };
    for preset in ["smallest", "balanced", "best"] {
        let r = run(&clip, json!({"preset": preset}));
        let (_, _, dec) = decode(&r.bytes);
        let reds: Vec<u8> = dec.iter().map(|(px, _)| px[0][0]).collect();
        assert_eq!(reds, vec![0, 20, 40, 60], "{preset}: {:?}", r.stats.frames);
    }
}

/// BA-110: lossy LZW with an SSIM bound (`lossyMinSsim`) – at "best" (loss-free, SSIM ≥ bound)
/// smaller than flexible parsing alone on the cartoon (never larger on the screen recording, whose
/// flat colours gain nothing from loss),
/// and every frame keeps SSIM ≥ the bound against its source frame (within `SLACK` for areas
/// drawn by earlier frames), unless palette and dithering alone already lose more (then the frame
/// stays close to the loss-free encode).
#[test]
fn lossy_ssim_bound_is_smaller_and_respected() {
    use fp_batch::steps::gifopt::metrics::ssim;
    let bound = 0.99;
    // the bound holds where a frame draws; unchanged areas keep the (bounded) loss of earlier
    // frames, so the whole frame may sit slightly under it
    const SLACK: f64 = 0.002;
    for clip in [screen(160, 120, 12), cartoon(128, 96, 12), pan(96, 72, 8)] {
        for preset in ["best", "balanced"] {
            // one decoded frame per source frame (no merging, no split parts)
            let base = json!({"preset": preset, "lzw": "flexible", "mergeSimilar": false, "splitFrames": false});
            let with = |extra: Value| {
                let mut o = opts(base.clone());
                o.extend(opts(extra));
                gifopt::optimize(&clip.item(), &o).expect("optimize")
            };
            let flexible = with(json!({}));
            let lossless = with(json!({"lossy": 0}));
            let bounded = with(json!({"lossyMinSsim": bound}));
            if clip.name == "cartoon" && preset == "best" {
                assert!(
                    bounded.bytes.len() < flexible.bytes.len(),
                    "{} {preset}: {} vs {}",
                    clip.name,
                    bounded.bytes.len(),
                    flexible.bytes.len()
                );
            }
            if clip.name != "pan" && preset == "best" {
                assert!(
                    bounded.bytes.len() <= flexible.bytes.len(),
                    "{} {preset}",
                    clip.name
                );
            }
            let (_, _, dec) = decode(&bounded.bytes);
            let (_, _, dec0) = decode(&lossless.bytes);
            assert_eq!(dec.len(), clip.frames.len(), "{} {preset}", clip.name);
            for (i, src) in clip.frames.iter().enumerate() {
                let s = ssim(src, &dec[i].0, clip.w, clip.h);
                let s0 = ssim(src, &dec0[i].0, clip.w, clip.h);
                assert!(
                    s >= bound - SLACK || (s0 < bound && s >= s0 - 0.005),
                    "{} {preset} frame {i}: SSIM {s:.5} (loss-free {s0:.5})",
                    clip.name
                );
            }
        }
    }
    // off by default, 0 = off: byte-identical to the output without the option
    let clip = cartoon(64, 48, 6);
    let plain = run(&clip, json!({"quality": 70}));
    let zero = run(&clip, json!({"quality": 70, "lossyMinSsim": 0}));
    assert_eq!(plain.bytes, zero.bytes);
    assert_eq!(
        GifOptions::from_map(&opts(json!({"lossyMinSsim": 2}))).lossy_min_ssim,
        Some(gifopt::MAX_SSIM_BOUND as f32)
    );
}

/// BA-110: cancel inside one GIF encode – the encode stops within about one frame of the cancel
/// (checked per frame, per search step and per LZW row) and reports "cancelled"; in a batch run the
/// item is cancelled and neither the output nor its temporary file is left behind.
#[test]
fn cancel_stops_inside_one_gif_encode() {
    use std::time::{Duration, Instant};
    let n = 24;
    let clip = pan(192, 144, n);
    let item = clip.item();
    let o = opts(json!({"preset": "best", "lossyMinSsim": 0.99}));
    let t = Instant::now();
    gifopt::encode_item(&item, &o).expect("full encode");
    let total = t.elapsed();
    let per_frame = total / n as u32;
    let mut seen = false;
    for div in [3u32, 6, 12] {
        let token = fp_batch::CancelToken::new();
        let c2 = token.clone();
        let delay = total / div;
        let h = std::thread::spawn(move || {
            std::thread::sleep(delay);
            c2.cancel();
            Instant::now()
        });
        let r = fp_batch::output::with_encode_cancel(&token, || gifopt::encode_item(&item, &o));
        let end = Instant::now();
        let at = h.join().expect("cancel thread");
        match r {
            Ok(_) => assert!(at >= end, "cancelled during the encode but not stopped"),
            Err(e) => {
                assert!(e.is_cancelled(), "{e}");
                let lag = end.saturating_duration_since(at);
                assert!(
                    lag <= per_frame * 2 + Duration::from_millis(100),
                    "lag {lag:?}, frame {per_frame:?}, total {total:?}"
                );
                seen = true;
                break;
            }
        }
    }
    assert!(seen, "no cancel landed inside the encode ({total:?})");

    // batch run: cancel while the item is being encoded
    let dir = temp("cancel");
    let src = dir.join("pan.gif");
    std::fs::write(
        &src,
        gifopt::encode_item(&item, &opts(json!({"quality": 60})))
            .expect("source")
            .as_slice(),
    )
    .expect("write");
    let mut r = Recipe::named("gif");
    r.output.format = "gif".into();
    r.output.options = o.clone();
    r.output.target = TargetSpec {
        kind: TargetKind::Folder,
        path: Some(dir.join("out")),
        ..TargetSpec::default()
    };
    r.add_step(gifopt::STABILIZE, json!({"threshold": 0}));
    let info = fp_batch::probe(&src).expect("probe");
    let (tx, rx) = std::sync::mpsc::channel();
    let cancel = fp_batch::CancelToken::new();
    let ro = RunOptions {
        threads: Some(1),
        cancel: cancel.clone(),
        progress: Some(tx),
        ..RunOptions::default()
    };
    let job = std::thread::spawn(move || executor::run(&r, &[info], &ro));
    // the steps are done when the progress reports an empty step: then the encode runs
    let mut cancelled_at = None;
    for p in rx.iter() {
        if let fp_batch::Progress::ItemProgress { step, .. } = &p
            && step.is_empty()
        {
            std::thread::sleep(total / 4);
            cancel.cancel();
            cancelled_at = Some(Instant::now());
            break;
        }
    }
    let rep = job.join().expect("join").expect("run");
    let end = Instant::now();
    assert_eq!(rep.totals.cancelled, 1, "{:#?}", rep.items);
    let left: Vec<_> = std::fs::read_dir(dir.join("out"))
        .map(|d| d.flatten().map(|e| e.file_name()).collect())
        .unwrap_or_default();
    assert!(left.is_empty(), "{left:?}");
    if let Some(at) = cancelled_at {
        assert!(
            end.saturating_duration_since(at) <= per_frame * 2 + Duration::from_millis(250),
            "run went on {:?} after the cancel",
            end.saturating_duration_since(at)
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

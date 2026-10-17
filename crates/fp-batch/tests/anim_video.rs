//! Animation und Video (BATCH-7, SPEC-BATCH §3.8, §5.5): Schritte `anim.*`, Zeitleisten-Modell,
//! Video über ffmpeg (mit der Attrappe `examples\fake_ffmpeg.rs`), Logikfälle nach
//! AGENTEN_REGELN §4b: Zeiten bleiben stimmig, 0 Frames, 1 Frame, Abbrechen, Wiederholung.
//!
//! Ein echtes ffmpeg wird nie heruntergeladen. `real_ffmpeg_if_installed` läuft nur, wenn eines
//! gefunden wird (Einstellung, `FIREPIXEL_FFMPEG`, `PATH`), sonst meldet er „übersprungen“.

mod support;

use fp_batch::item::{Animation, Frame, Item, ItemKind, Meta};
use fp_batch::output::{OutputFormat, encode};
use fp_batch::steps::anim::{self, Timeline};
use fp_batch::video::ffmpeg::{self, Ffmpeg, FoundBy};
use fp_batch::video::{self, VideoInfo, VideoParams};
use fp_batch::{BatchError, CancelToken, ItemStatus, Recipe, RunOptions, StepRegistry, executor};
use fp_raster::{IRect, RgbaImage};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use support::*;

// ------------------------------------------------------------------ Hilfen

/// Sperre für Tests, die die prozessweite ffmpeg-Suche benutzen (Override/Einstellung).
fn global_lock() -> MutexGuard<'static, ()> {
    static L: Mutex<()> = Mutex::new(());
    L.lock().unwrap_or_else(|e| e.into_inner())
}

fn solid(w: u32, h: u32, px: [u8; 4]) -> RgbaImage {
    let bytes: Vec<u8> = (0..w * h).flat_map(|_| px).collect();
    RgbaImage::from_rgba_bytes(IRect::new(0, 0, w as i32, h as i32), &bytes).expect("img")
}

/// Animation mit `delays.len()` Frames; Frame i hat Rot = i * 20.
fn anim(delays: &[u32]) -> Item {
    let frames = delays
        .iter()
        .enumerate()
        .map(|(i, &d)| Frame {
            image: solid(6, 4, [(i * 20) as u8, 50, 100, 255]),
            delay_ms: d,
        })
        .collect();
    Item::Animation(Animation {
        width: 6,
        height: 4,
        frames,
        loop_count: 0,
        meta: Meta {
            source_format: "gif".into(),
            ..Meta::default()
        },
    })
}

fn frames_of(item: &Item) -> &[Frame] {
    match item {
        Item::Animation(a) => &a.frames,
        other => panic!("keine Animation: {:?}", other.kind()),
    }
}

fn reds(item: &Item) -> Vec<u8> {
    frames_of(item)
        .iter()
        .map(|f| f.image.as_bytes()[0])
        .collect()
}

fn delays(item: &Item) -> Vec<u32> {
    frames_of(item).iter().map(|f| f.delay_ms).collect()
}

/// Wendet einen Schritt direkt an (Zielformat `fmt`).
fn apply(
    id: &str,
    item: &mut Item,
    params: Value,
    fmt: OutputFormat,
) -> fp_batch::Result<Vec<String>> {
    let reg = StepRegistry::global();
    let step = reg.get(id).expect("step").clone();
    let p = fp_batch::step::with_defaults(&step.params(), &params);
    let info = fp_batch::ItemInfo {
        path: PathBuf::from("x.gif"),
        root: None,
        kind: ItemKind::Animation,
        format: "gif".into(),
        bytes: 0,
        width: 6,
        height: 4,
        frames: 0,
        modified: None,
        exif: None,
    };
    let cancel = CancelToken::new();
    let mut ctx = fp_batch::StepContext::new(0, 1, &info, &cancel, fmt);
    step.apply(item, &p, &mut ctx)?;
    Ok(ctx.warnings)
}

fn write_anim(path: &Path, item: &Item, fmt: OutputFormat) {
    let (bytes, _) = encode(item, fmt, &Default::default()).expect("encode");
    std::fs::write(path, bytes).expect("write");
}

fn load(path: &Path) -> Item {
    let data = std::fs::read(path).expect("read");
    fp_batch::item::decode_bytes(&path.to_string_lossy(), &data).expect("decode")
}

fn run(recipe: &Recipe, paths: &[PathBuf]) -> fp_batch::Report {
    let items = fp_batch::collect(paths, false, false, &[]).items;
    executor::run(
        recipe,
        &items,
        &RunOptions {
            threads: Some(2),
            ..RunOptions::default()
        },
    )
    .expect("run")
}

fn recipe(out: &Path, format: &str) -> Recipe {
    let mut r = Recipe::named("anim-test");
    r.output.format = format.into();
    r.output.target = fp_batch::recipe::TargetSpec {
        kind: fp_batch::recipe::TargetKind::Folder,
        path: Some(out.to_path_buf()),
        ..Default::default()
    };
    r
}

/// Pfad der ffmpeg-Attrappe (`cargo test -p fp-batch` baut die Beispiele mit).
fn fake() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let debug = exe.parent()?.parent()?;
    let p = debug.join("examples").join(if cfg!(windows) {
        "fake_ffmpeg.exe"
    } else {
        "fake_ffmpeg"
    });
    if p.is_file() {
        Some(p)
    } else {
        eprintln!(
            "übersprungen: ffmpeg-Attrappe fehlt ({}) – `cargo test -p fp-batch` baut sie",
            p.display()
        );
        None
    }
}

fn fake_video(dir: &Path, name: &str, spec: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, spec).expect("write video");
    p
}

// ------------------------------------------------------------------ anim.trim

#[test]
fn trim_by_time_keeps_times_consistent() {
    let mut it = anim(&[100, 100, 100, 100, 100]);
    apply(
        anim::TRIM,
        &mut it,
        json!({"start": 150, "end": 420}),
        OutputFormat::Apng,
    )
    .expect("trim");
    assert_eq!(
        delays(&it),
        vec![50, 100, 100, 20],
        "angeschnittene Frames werden gekürzt"
    );
    assert_eq!(
        delays(&it).iter().sum::<u32>(),
        270,
        "Dauer = Ende − Anfang"
    );
    assert_eq!(reds(&it), vec![20, 40, 60, 80]);
    // Zeitangaben als Text
    let mut it = anim(&[100, 100, 100, 100, 100]);
    apply(
        anim::TRIM,
        &mut it,
        json!({"start": "0:00.200", "end": "0.4s"}),
        OutputFormat::Apng,
    )
    .expect("trim");
    assert_eq!(delays(&it), vec![100, 100]);
}

#[test]
fn trim_by_frame_numbers_and_remove_frames() {
    let mut it = anim(&[100, 110, 120, 130, 140, 150]);
    apply(
        anim::TRIM,
        &mut it,
        json!({"unit": "frame", "start": 2, "end": 5, "removeFrames": "3, 5-"}),
        OutputFormat::Apng,
    )
    .expect("trim");
    assert_eq!(reds(&it), vec![20, 60], "Bilder 2 und 4 bleiben");
    assert_eq!(
        delays(&it),
        vec![110, 130],
        "Dauern entfernter Bilder fallen weg"
    );
}

#[test]
fn trim_to_zero_frames_is_an_error_and_one_frame_stays_animation() {
    let mut it = anim(&[100, 100, 100]);
    let e = apply(
        anim::TRIM,
        &mut it,
        json!({"start": 5000}),
        OutputFormat::Gif,
    )
    .expect_err("leer");
    assert!(matches!(e, BatchError::Invalid(_)), "{e:?}");
    let mut it = anim(&[100, 100, 100]);
    let e = apply(
        anim::TRIM,
        &mut it,
        json!({"removeFrames": "1-"}),
        OutputFormat::Gif,
    )
    .expect_err("alles entfernt");
    assert!(e.to_string().contains(&fp_i18n::t("batch-anim-err-empty")));
    // genau ein Bild
    let mut it = anim(&[100, 100, 100]);
    apply(
        anim::TRIM,
        &mut it,
        json!({"unit": "frame", "start": 2, "end": 2}),
        OutputFormat::Gif,
    )
    .expect("1 frame");
    assert_eq!(reds(&it), vec![20]);
    assert_eq!(it.kind(), ItemKind::Animation);
    // 1 Frame lässt sich als GIF/APNG/WebP schreiben
    for fmt in [OutputFormat::Gif, OutputFormat::Apng, OutputFormat::WebP] {
        let (bytes, _) = encode(&it, fmt, &Default::default()).expect("encode 1 frame");
        assert!(!bytes.is_empty());
    }
}

#[test]
fn trim_validation() {
    let reg = StepRegistry::global();
    let mut r = Recipe::named("v");
    r.add_step(anim::TRIM, json!({"start": 500, "end": 200}));
    assert!(
        r.validate(&reg)
            .iter()
            .any(|i| i.param.as_deref() == Some("end")),
        "Ende vor Anfang"
    );
    let mut r = Recipe::named("v");
    r.add_step(anim::TRIM, json!({"removeFrames": "3-1, x"}));
    assert!(
        r.validate(&reg)
            .iter()
            .any(|i| i.param.as_deref() == Some("removeFrames"))
    );
    let mut r = Recipe::named("v");
    r.add_step(anim::TRIM, json!({"start": "abc"}));
    assert!(
        r.validate(&reg)
            .iter()
            .any(|i| i.param.as_deref() == Some("start"))
    );
    let mut r = Recipe::named("v");
    r.add_step(
        anim::TRIM,
        json!({"start": "0:01", "end": "0:02.5", "removeFrames": "1-3, 10, 20-"}),
    );
    let errors: Vec<_> = r
        .validate(&reg)
        .into_iter()
        .filter(|i| i.severity == fp_batch::Severity::Error)
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn frame_ranges_roundtrip() {
    let m = anim::parse_frame_ranges("1-3, 10, 20-", 22).expect("ranges");
    assert_eq!(m.iter().filter(|&&x| x).count(), 3 + 1 + 3);
    assert_eq!(anim::format_frame_ranges(&m), "1-3, 10, 20-22");
    assert_eq!(
        anim::parse_frame_ranges("-2", 5).expect("r"),
        vec![true, true, false, false, false]
    );
    assert!(anim::parse_frame_ranges("0", 5).is_err());
    assert!(
        anim::parse_frame_ranges("", 3)
            .expect("leer")
            .iter()
            .all(|x| !x)
    );
}

// ------------------------------------------------------------------ anim.speed

#[test]
fn speed_factor_and_fps_keep_total_time() {
    let mut it = anim(&[100, 100, 100, 100]);
    apply(
        anim::SPEED,
        &mut it,
        json!({"factor": 2.0}),
        OutputFormat::Apng,
    )
    .expect("speed");
    assert_eq!(delays(&it), vec![50, 50, 50, 50]);
    let mut it = anim(&[100, 100, 100]);
    apply(
        anim::SPEED,
        &mut it,
        json!({"factor": 3.0}),
        OutputFormat::Apng,
    )
    .expect("speed");
    assert_eq!(
        delays(&it).iter().sum::<u32>(),
        100,
        "Fehlerübertrag: 33+33+34"
    );
    let mut it = anim(&[40, 60, 200]);
    apply(anim::SPEED, &mut it, json!({"fps": 25}), OutputFormat::Apng).expect("fps");
    assert_eq!(delays(&it), vec![40, 40, 40]);
}

#[test]
fn speed_merges_too_short_frames_and_rounds_for_gif() {
    // 10 × 10 ms, Browser würden < 20 ms als 100 ms zeigen → zusammenfassen, Gesamtzeit bleibt
    let mut it = anim(&[10; 10]);
    apply(anim::SPEED, &mut it, json!({}), OutputFormat::Apng).expect("speed");
    assert_eq!(delays(&it), vec![20; 5]);
    assert_eq!(
        reds(&it),
        vec![0, 40, 80, 120, 160],
        "jedes zweite Bild bleibt"
    );
    // GIF: Vielfache von 10 ms, Summe stimmt
    let mut it = anim(&[33, 33, 34, 33, 33, 34]);
    apply(
        anim::SPEED,
        &mut it,
        json!({"minDelay": 0}),
        OutputFormat::Gif,
    )
    .expect("speed gif");
    assert!(delays(&it).iter().all(|d| d % 10 == 0), "{:?}", delays(&it));
    assert_eq!(delays(&it).iter().sum::<u32>(), 200);
    // langsamer: 0,5×
    let mut it = anim(&[30, 30]);
    apply(
        anim::SPEED,
        &mut it,
        json!({"factor": 0.5}),
        OutputFormat::Gif,
    )
    .expect("speed");
    assert_eq!(delays(&it), vec![60, 60]);
}

#[test]
fn speed_param_changed_fifty_times_equals_final_value() {
    let dir = temp("anim-speed50");
    let src = dir.join("a.gif");
    write_anim(&src, &anim(&[100, 100, 100, 100]), OutputFormat::Gif);
    let mut r1 = recipe(&dir.join("o1"), "gif");
    let i = r1.add_step(anim::SPEED, json!({}));
    for k in 1..=50 {
        r1.set_param(i, "factor", json!(0.5 + k as f64 * 0.03));
    }
    assert_eq!(r1.steps.len(), 1, "ein Schritt");
    let mut r2 = recipe(&dir.join("o2"), "gif");
    r2.add_step(anim::SPEED, json!({"factor": 0.5 + 50.0 * 0.03}));
    assert_eq!(run(&r1, std::slice::from_ref(&src)).totals.done, 1);
    assert_eq!(run(&r2, std::slice::from_ref(&src)).totals.done, 1);
    assert_eq!(
        checksum(&dir.join("o1").join("a.gif")),
        checksum(&dir.join("o2").join("a.gif"))
    );
    let out = load(&dir.join("o1").join("a.gif"));
    assert_eq!(delays(&out), vec![50, 50, 50, 50], "100 ms / 2,0");
    let _ = std::fs::remove_dir_all(&dir);
}

// ------------------------------------------------------------------ anim.loop

#[test]
fn loop_reverse_ping_pong_and_count() {
    let mut it = anim(&[10, 20, 30, 40]);
    apply(
        anim::LOOP,
        &mut it,
        json!({"reverse": true}),
        OutputFormat::Apng,
    )
    .expect("loop");
    assert_eq!(reds(&it), vec![60, 40, 20, 0]);
    assert_eq!(delays(&it), vec![40, 30, 20, 10], "Dauern wandern mit");
    let mut it = anim(&[10, 20, 30, 40]);
    apply(
        anim::LOOP,
        &mut it,
        json!({"pingPong": true, "mode": "once"}),
        OutputFormat::Apng,
    )
    .expect("loop");
    assert_eq!(
        reds(&it),
        vec![0, 20, 40, 60, 40, 20],
        "Endbilder nicht doppelt"
    );
    let Item::Animation(a) = &it else {
        unreachable!()
    };
    assert_eq!(a.loop_count, -1);
    let mut it = anim(&[10, 20, 30]);
    apply(
        anim::LOOP,
        &mut it,
        json!({"mode": "count", "count": 5}),
        OutputFormat::Gif,
    )
    .expect("loop");
    let Item::Animation(a) = &it else {
        unreachable!()
    };
    assert_eq!(a.loop_count, 5);
    // 1 Bild: Pingpong ändert nichts
    let mut it = anim(&[10]);
    apply(
        anim::LOOP,
        &mut it,
        json!({"pingPong": true, "reverse": true}),
        OutputFormat::Gif,
    )
    .expect("loop 1");
    assert_eq!(reds(&it), vec![0]);
}

// ------------------------------------------------------------------ anim.dropFrames / frameDelay

#[test]
fn drop_frames_every_nth_and_target_fps_keep_total_time() {
    let mut it = anim(&[100; 7]);
    apply(
        anim::DROP_FRAMES,
        &mut it,
        json!({"keepEvery": 3, "mergeSimilar": false}),
        OutputFormat::Apng,
    )
    .expect("drop");
    assert_eq!(reds(&it), vec![0, 60, 120]);
    assert_eq!(delays(&it), vec![300, 300, 100]);
    let mut it = anim(&[100; 10]);
    apply(
        anim::DROP_FRAMES,
        &mut it,
        json!({"targetFps": 5, "mergeSimilar": false}),
        OutputFormat::Apng,
    )
    .expect("fps");
    assert_eq!(delays(&it), vec![200; 5]);
    assert_eq!(reds(&it), vec![0, 40, 80, 120, 160]);
}

#[test]
fn drop_frames_merges_similar_frames() {
    let frames: Vec<Frame> = [0u8, 0, 1, 200, 200]
        .iter()
        .map(|&r| Frame {
            image: solid(4, 4, [r, 0, 0, 255]),
            delay_ms: 50,
        })
        .collect();
    let mut it = Item::Animation(Animation {
        width: 4,
        height: 4,
        frames,
        loop_count: 0,
        meta: Meta::default(),
    });
    apply(anim::DROP_FRAMES, &mut it, json!({}), OutputFormat::Apng).expect("merge");
    // 0,0,1 sind „gleich“ (Unterschied 0,1 % < 0,5 %), 200,200 auch
    assert_eq!(delays(&it), vec![150, 100]);
    assert_eq!(reds(&it), vec![0, 200]);
}

#[test]
fn frame_delay_all_and_pause_at_end() {
    let mut it = anim(&[10, 20, 30]);
    apply(
        anim::FRAME_DELAY,
        &mut it,
        json!({"delay": 80, "first": 500, "last": 2000}),
        OutputFormat::Apng,
    )
    .expect("delay");
    assert_eq!(delays(&it), vec![500, 80, 2000]);
    let mut it = anim(&[15, 25]);
    apply(
        anim::FRAME_DELAY,
        &mut it,
        json!({"mode": "keep"}),
        OutputFormat::Gif,
    )
    .expect("delay gif");
    assert!(delays(&it).iter().all(|d| d % 10 == 0));
    assert_eq!(delays(&it).iter().sum::<u32>(), 40);
}

// ------------------------------------------------------------------ Bildfolge ↔ Animation

#[test]
fn animation_to_frames_writes_one_file_per_frame() {
    let dir = temp("anim-toframes");
    let a = dir.join("clip.gif");
    let b = dir.join("other.gif");
    write_anim(&a, &anim(&[100, 100, 100]), OutputFormat::Gif);
    write_anim(&b, &anim(&[50, 50]), OutputFormat::Gif);
    let out = dir.join("out");
    let mut r = recipe(&out, "png");
    r.add_step(anim::TO_FRAMES, json!({}));
    let rep = run(&r, &[a, b]);
    assert_eq!(
        rep.totals.done, 2,
        "beide Quellen verarbeitet: {:#?}",
        rep.items
    );
    let mut names: Vec<String> = files_under(&out)
        .iter()
        .filter(|p| p.extension().is_some_and(|e| e == "png"))
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec![
            "clip_0001.png",
            "clip_0002.png",
            "clip_0003.png",
            "other_0001.png",
            "other_0002.png"
        ]
    );
    assert_eq!(pixels(&out.join("clip_0002.png"))[0], 20, "Bild 2 von clip");
    assert!(!has_temp_files(&dir));
    // Muster
    assert_eq!(
        anim::frame_name("{name}-f{frame:2}-{n}", "a", 3, 1),
        "a-f03-1"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn frames_to_animation_apng_and_webp() {
    let dir = temp("anim-fromframes");
    let paths: Vec<PathBuf> = (0..3)
        .map(|k| {
            let p = dir.join(format!("f{k}.png"));
            let it = fp_batch::item::still_from_rgba(8 + 2 * k, 6, &rgba(8 + 2 * k, 6, false))
                .expect("still");
            write_anim(&p, &it, OutputFormat::Png);
            p
        })
        .collect();
    for fmt in ["apng", "webp", "gif"] {
        let out = dir.join(format!("out-{fmt}"));
        let mut r = recipe(&out, fmt);
        r.add_step(anim::FROM_FRAMES, json!({"delay": 120, "size": "largest"}));
        let rep = run(&r, &paths);
        assert_eq!(rep.totals.done, 3, "{fmt}: {:#?}", rep.items);
        let ext = if fmt == "apng" { "png" } else { fmt };
        let file = files_under(&out)
            .into_iter()
            .find(|p| p.extension().is_some_and(|e| e == ext))
            .expect("Ausgabe");
        let it = load(&file);
        assert_eq!(it.dimensions(), (12, 6), "{fmt}: größtes Bild");
        assert_eq!(frames_of(&it).len(), 3, "{fmt}");
        assert_eq!(delays(&it), vec![120, 120, 120], "{fmt}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn sprite_sheet_roundtrip() {
    let mut it = anim(&[10, 20, 30, 40, 50]);
    let orig = it.clone();
    apply(
        anim::SPRITE_SHEET,
        &mut it,
        json!({"spacing": 1, "padding": 2}),
        OutputFormat::Png,
    )
    .expect("sheet");
    let Item::Still(s) = &it else {
        panic!("Sheet ist ein Bild")
    };
    // 3 Spalten × 2 Zeilen à 6×4, Abstand 1, Rand 2
    assert_eq!(
        (s.image.width(), s.image.height()),
        (2 * 2 + 3 * 6 + 2, 2 * 2 + 2 * 4 + 1)
    );
    let atlas = &s.meta.extra[anim::EXTRA_ATLAS];
    assert_eq!(atlas["frames"].as_array().map(Vec::len), Some(5));
    assert_eq!(atlas["frames"][4]["x"], json!(2 + 7));
    apply(anim::SPRITE_SHEET, &mut it, json!({"direction": "fromSheet", "columns": 3, "rows": 2, "spacing": 1, "padding": 2, "count": 5, "delay": 70}), OutputFormat::Gif).expect("frames");
    assert_eq!(reds(&it), reds(&orig));
    assert_eq!(delays(&it), vec![70; 5]);
    let a = frames_of(&it);
    assert_eq!(a[3].image, frames_of(&orig)[3].image, "Pixel gleich");
    // Raster passt nicht
    let mut st = fp_batch::item::still_from_rgba(4, 4, &rgba(4, 4, false)).expect("still");
    assert!(
        apply(
            anim::SPRITE_SHEET,
            &mut st,
            json!({"direction": "fromSheet", "columns": 10, "rows": 1}),
            OutputFormat::Gif
        )
        .is_err()
    );
}

// ------------------------------------------------------------------ Lauf und Vorschau

#[test]
fn gif_trim_and_speed_through_the_run_keeps_times() {
    let dir = temp("anim-run");
    let src = dir.join("walk.gif");
    write_anim(
        &src,
        &anim(&[100, 100, 100, 100, 100, 100]),
        OutputFormat::Gif,
    );
    let sum = checksum(&src);
    let out = dir.join("out");
    let mut r = recipe(&out, "same");
    r.add_step(anim::TRIM, json!({"start": 100, "end": 500}));
    r.add_step(anim::SPEED, json!({"factor": 2}));
    r.add_step(anim::LOOP, json!({"pingPong": true}));
    let rep = run(&r, std::slice::from_ref(&src));
    assert_eq!(rep.totals.done, 1, "{:#?}", rep.items);
    let it = load(&out.join("walk.gif"));
    assert_eq!(reds(&it), vec![20, 40, 60, 80, 60, 40]);
    assert_eq!(delays(&it), vec![50; 6]);
    assert_eq!(checksum(&src), sum, "Original unverändert");
    // zweiter Lauf: gleiches Ergebnis, keine doppelten Effekte
    let out2 = dir.join("out2");
    let mut r2 = r.clone();
    r2.output.target.path = Some(out2.clone());
    run(&r2, std::slice::from_ref(&src));
    assert_eq!(
        checksum(&out.join("walk.gif")),
        checksum(&out2.join("walk.gif"))
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn zero_frames_in_run_fails_item_without_leftovers() {
    let dir = temp("anim-zero");
    let src = dir.join("a.gif");
    write_anim(&src, &anim(&[100, 100]), OutputFormat::Gif);
    let out = dir.join("out");
    let mut r = recipe(&out, "gif");
    r.add_step(anim::TRIM, json!({"start": 9000}));
    let rep = run(&r, std::slice::from_ref(&src));
    assert_eq!(rep.items[0].status, ItemStatus::Failed);
    assert!(
        files_under(&out)
            .iter()
            .all(|p| p.extension().is_none_or(|e| e != "gif")),
        "keine leere Datei"
    );
    assert!(!has_temp_files(&dir));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn preview_shows_trimmed_animation_and_is_repeatable() {
    let dir = temp("anim-preview");
    let src = dir.join("a.gif");
    write_anim(&src, &anim(&[100; 8]), OutputFormat::Gif);
    let info = fp_batch::probe(&src).expect("probe");
    let mut r = recipe(&dir.join("out"), "gif");
    r.add_step(anim::TRIM, json!({"unit": "frame", "start": 3, "end": 5}));
    let reg = StepRegistry::global();
    let p1 = fp_batch::preview(&r, &info, &reg, &Default::default()).expect("preview");
    let p2 = fp_batch::preview(&r, &info, &reg, &Default::default()).expect("preview 2");
    let a1 = p1.after_item.expect("after");
    assert_eq!(reds(&a1), vec![40, 60, 80]);
    assert_eq!(
        reds(&a1),
        reds(&p2.after_item.expect("after 2")),
        "Vorschau ersetzt, statt zu addieren"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ------------------------------------------------------------------ Zeitleiste

#[test]
fn timeline_model_snaps_and_matches_trim() {
    let mut tl = Timeline::new(vec![100, 50, 150, 100]);
    assert_eq!(tl.total_ms(), 400);
    assert_eq!(tl.frame_at(0), 0);
    assert_eq!(tl.frame_at(149), 1);
    assert_eq!(tl.frame_at(10_000), 3);
    assert_eq!(tl.snap(130), 150);
    // Griff 50× ziehen (jede Bewegung ein Ereignis) = einmal mit dem Endwert
    for x in 0..50 {
        tl.set_start(x * 3);
    }
    let mut once = Timeline::new(vec![100, 50, 150, 100]);
    once.set_start(49 * 3);
    assert_eq!(tl, once);
    assert_eq!(tl.start_ms, 150);
    tl.set_end(260);
    assert_eq!(tl.end_ms, 300);
    // Ende nie vor dem Anfang
    tl.set_end(0);
    assert!(tl.end_ms > tl.start_ms);
    tl.set_end(400);
    tl.toggle_removed(3);
    let (kept, ms) = tl.kept();
    assert_eq!((kept, ms), (1, 150));
    // Parameter ↔ Modell ↔ Schritt
    let p = tl.trim_params();
    assert_eq!(p["removeFrames"], "4");
    assert_eq!(p["end"], Value::Null);
    let mut back = Timeline::new(vec![100, 50, 150, 100]);
    back.apply_params(&p);
    assert_eq!(back, tl);
    let frames = (0..4)
        .map(|i| Frame {
            image: solid(2, 2, [i as u8, 0, 0, 255]),
            delay_ms: [100, 50, 150, 100][i],
        })
        .collect();
    let out = anim::trim_frames(frames, &p).expect("trim");
    assert_eq!(out.len(), kept);
    assert_eq!(out.iter().map(|f| f.delay_ms as u64).sum::<u64>(), ms);
}

#[test]
fn quantize_keeps_sum() {
    let d = anim::quantize_delays(&[33.3, 33.3, 33.4, 16.7, 16.7, 16.6], 10);
    assert_eq!(d.iter().sum::<u32>(), 150);
    assert!(d.iter().all(|x| x % 10 == 0));
    assert_eq!(video::frame_delays(3, 30.0), vec![33, 34, 33]);
    assert_eq!(video::frame_delays(12, 12.0).iter().sum::<u32>(), 1000);
}

// ------------------------------------------------------------------ Video: Parameter

#[test]
fn video_params_window_size_and_times() {
    let p = VideoParams::from_json(&json!({"start": "0:01.5", "end": 4000, "fps": 10}));
    assert_eq!((p.start_ms, p.end_ms), (1500, Some(4000)));
    assert_eq!(p.frame_count(Some(10_000)), 25);
    assert_eq!(p.frame_count(Some(2_000)), 5, "Video kürzer als das Ende");
    assert_eq!(
        p.frame_count(Some(1_000)),
        0,
        "Anfang nach dem Ende → 0 Frames"
    );
    let p = VideoParams {
        max_frames: 7,
        ..VideoParams::default()
    };
    assert_eq!(p.frame_count(None), 7, "unbekannte Dauer → Höchstzahl");
    let p = VideoParams {
        max_size: 320,
        ..VideoParams::default()
    };
    assert_eq!(p.output_size(1920, 1080, 10), (320, 180));
    let p = VideoParams {
        memory_cap: 1920 * 1080 * 4 * 10,
        ..VideoParams::default()
    };
    let (w, h) = p.output_size(1920, 1080, 40);
    assert!(
        (w as u64 * h as u64 * 4 * 40) <= p.memory_cap + 1920 * 8,
        "Speichergrenze: {w}x{h}"
    );
    assert_eq!(
        VideoParams::default().output_size(64, 48, 600),
        (64, 48),
        "nie vergrößern"
    );
    assert_eq!(video::parse_time_ms(&json!("1:02:03.5")), Some(3_723_500));
    assert_eq!(video::parse_time_ms(&json!("x")), None);
    assert_eq!(video::format_time_ms(62_500), "1:02.500");
}

// ------------------------------------------------------------------ Video über ffmpeg (Attrappe)

#[test]
fn ffmpeg_version_probe_and_status_with_fake() {
    let Some(fake) = fake() else { return };
    let ff = Ffmpeg::at(&fake);
    assert_eq!(ff.version().expect("version"), "9.9-fake");
    let dir = temp("ff-probe");
    let v = fake_video(&dir, "a.mp4", "FAKEVIDEO 64 48 2500 25");
    let info = ff.probe(&v).expect("probe");
    assert_eq!(
        info,
        VideoInfo {
            duration_ms: Some(2500),
            width: 64,
            height: 48,
            fps: Some(25.0),
            codec: "h264".into()
        }
    );
    let r = fake_video(&dir, "r.mov", "FAKEVIDEO 64 48 1000 30 rotate90");
    assert_eq!(
        ff.probe(&r).map(|i| (i.width, i.height)).expect("rot"),
        (48, 64),
        "Handyvideo hochkant"
    );
    let bad = fake_video(&dir, "bad.mp4", "FAKEFAIL");
    let e = ff.probe(&bad).expect_err("kaputt");
    assert!(e.to_string().contains("Invalid data"), "{e}");
    let _g = global_lock();
    ffmpeg::set_override(Some(fake.clone()));
    let st = ffmpeg::status();
    assert!(st.is_ok(), "{st:?}");
    assert_eq!(st.found_by, Some(FoundBy::Override));
    assert!(st.message.contains("9.9-fake"));
    ffmpeg::set_override(None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn video_decode_window_fps_and_timing() {
    let Some(fake) = fake() else { return };
    let dir = temp("ff-decode");
    let v = fake_video(&dir, "clip.mp4", "FAKEVIDEO 64 48 5000 25");
    let ff = Ffmpeg::at(&fake);
    let p = VideoParams::from_json(&json!({"start": 500, "end": 1500, "fps": 10}));
    let it = video::decode_with(&ff, &v, &p, &CancelToken::new()).expect("decode");
    assert_eq!(frames_of(&it).len(), 10);
    assert_eq!(
        delays(&it),
        vec![100; 10],
        "Zeiten stimmig: 10 × 100 ms = Ausschnitt"
    );
    assert_eq!(reds(&it), (0..10).collect::<Vec<u8>>());
    let greens: Vec<u8> = frames_of(&it)
        .iter()
        .map(|f| f.image.as_bytes()[1])
        .collect();
    assert_eq!(greens[0], 50, "erstes Bild bei 500 ms");
    assert_eq!(greens[9], 140, "letztes Bild bei 1400 ms");
    let meta = it.meta().expect("meta");
    assert_eq!(meta.extra[video::EXTRA_PARAMS], p.to_json());
    // Größe und Höchstzahl
    let p = VideoParams::from_json(&json!({"maxSize": 32, "maxFrames": 4}));
    let it = video::decode_with(&ff, &v, &p, &CancelToken::new()).expect("decode small");
    assert_eq!(it.dimensions(), (32, 24));
    assert_eq!(frames_of(&it).len(), 4);
    // genau 1 Frame
    let p = VideoParams::from_json(&json!({"end": 1000, "fps": 1}));
    let it = video::decode_with(&ff, &v, &p, &CancelToken::new()).expect("1 frame");
    assert_eq!(frames_of(&it).len(), 1);
    assert_eq!(delays(&it), vec![1000]);
    // 0 Frames: Anfang hinter dem Ende des Videos
    let p = VideoParams::from_json(&json!({"start": 9000}));
    let e = video::decode_with(&ff, &v, &p, &CancelToken::new()).expect_err("leer");
    assert!(
        e.to_string().contains(&fp_i18n::tr!(
            "batch-anim-err-video-no-frames",
            path = v.display().to_string()
        )),
        "{e}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn video_cancel_stops_ffmpeg_cleanly() {
    let Some(fake) = fake() else { return };
    let dir = temp("ff-cancel");
    let v = fake_video(&dir, "long.mp4", "FAKEVIDEO 32 24 600000 25 slow");
    let progress = dir.join("long.mp4.progress");
    let cancel = CancelToken::new();
    let c2 = cancel.clone();
    let ff = Ffmpeg::at(&fake);
    let t0 = Instant::now();
    let h = std::thread::spawn(move || {
        video::decode_with(
            &ff,
            &v,
            &VideoParams::from_json(&json!({"maxFrames": 3000})),
            &c2,
        )
    });
    std::thread::sleep(Duration::from_millis(400));
    cancel.cancel();
    let r = h.join().expect("thread");
    assert!(
        matches!(r, Err(ref e) if e.is_cancelled()),
        "{:?}",
        r.as_ref().err()
    );
    assert!(
        t0.elapsed() < Duration::from_secs(5),
        "Abbrechen wirkt sofort"
    );
    // Prozess ist beendet: Fortschritt wächst nicht mehr
    std::thread::sleep(Duration::from_millis(150));
    let a = std::fs::read_to_string(&progress).unwrap_or_default();
    std::thread::sleep(Duration::from_millis(400));
    let b = std::fs::read_to_string(&progress).unwrap_or_default();
    assert_eq!(a, b, "ffmpeg läuft nach dem Abbrechen nicht weiter");
    let n: u32 = a.trim().parse().unwrap_or(0);
    assert!(n < 3000, "abgebrochen nach {n} Bildern");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn video_cancel_while_ffmpeg_is_silent() {
    let Some(fake) = fake() else { return };
    let dir = temp("ff-stall");
    let v = fake_video(&dir, "stall.mp4", "FAKEVIDEO 32 24 5000 25 stall");
    let cancel = CancelToken::new();
    let c2 = cancel.clone();
    let ff = Ffmpeg::at(&fake);
    let h = std::thread::spawn(move || video::decode_with(&ff, &v, &VideoParams::default(), &c2));
    std::thread::sleep(Duration::from_millis(300));
    let t0 = Instant::now();
    cancel.cancel();
    let r = h.join().expect("thread");
    assert!(matches!(r, Err(ref e) if e.is_cancelled()));
    assert!(
        t0.elapsed() < Duration::from_secs(3),
        "auch ohne Daten sofort beendet: {:?}",
        t0.elapsed()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn video_to_gif_apng_webp_through_the_run() {
    let Some(fake) = fake() else { return };
    let _g = global_lock();
    ffmpeg::set_override(Some(fake));
    let dir = temp("ff-run");
    let v = fake_video(&dir, "clip.mp4", "FAKEVIDEO 40 30 3000 30");
    let info = fp_batch::probe(&v).expect("probe");
    assert_eq!(info.kind, ItemKind::Video);
    for fmt in ["gif", "apng", "webp"] {
        let out = dir.join(format!("out-{fmt}"));
        let mut r = recipe(&out, fmt);
        r.add_step(
            anim::VIDEO_IMPORT,
            json!({"start": 1000, "end": 2000, "fps": 10, "maxSize": 20}),
        );
        r.add_step(anim::SPEED, json!({"factor": 2}));
        let rep = run(&r, std::slice::from_ref(&v));
        assert_eq!(rep.totals.done, 1, "{fmt}: {:#?}", rep.items);
        let ext = if fmt == "apng" { "png" } else { fmt };
        let it = load(&out.join(format!("clip.{ext}")));
        assert_eq!(it.dimensions(), (20, 15), "{fmt}");
        assert_eq!(frames_of(&it).len(), 10, "{fmt}");
        assert_eq!(
            delays(&it).iter().sum::<u32>(),
            500,
            "{fmt}: 1 s Video, doppelt so schnell"
        );
    }
    // Vorschau: Video lesen mit Vorschau-Grenzen, zweimal gleich
    let mut r = recipe(&dir.join("pv"), "gif");
    r.add_step(anim::VIDEO_IMPORT, json!({"end": 1000, "fps": 5}));
    let reg = StepRegistry::global();
    let a = fp_batch::preview(&r, &info, &reg, &Default::default()).expect("preview");
    let b = fp_batch::preview(&r, &info, &reg, &Default::default()).expect("preview");
    assert_eq!(frames_of(a.after_item.as_ref().expect("a")).len(), 5);
    assert_eq!(
        delays(a.after_item.as_ref().expect("a")),
        delays(b.after_item.as_ref().expect("b"))
    );
    ffmpeg::set_override(None);
    assert!(!has_temp_files(&dir));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ffmpeg_setting_is_saved_in_the_test_profile() {
    let Some(fake) = fake() else { return };
    let _g = global_lock();
    ffmpeg::set_override(None);
    let file = ffmpeg::settings_path().expect("profile");
    assert!(!fp_i18n::profile::is_real(&file), "nie ins echte Profil");
    ffmpeg::set_configured_path(Some(&fake)).expect("save");
    assert_eq!(ffmpeg::configured_path().as_deref(), Some(fake.as_path()));
    let found = ffmpeg::find().expect("find");
    assert_eq!(
        (found.path.as_path(), found.found_by),
        (fake.as_path(), FoundBy::Setting)
    );
    // Ordner statt Datei wird auch verstanden (ffmpeg.exe darin)
    let dir = temp("ff-setting");
    let exe = dir.join(if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    });
    std::fs::copy(&fake, &exe).expect("copy");
    ffmpeg::set_configured_path(Some(&dir)).expect("dir");
    let found = ffmpeg::find().expect("find dir");
    assert_eq!((found.path, found.found_by), (exe, FoundBy::Setting));
    ffmpeg::set_configured_path(None).expect("remove");
    assert_eq!(ffmpeg::configured_path(), None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn video_without_ffmpeg_reports_how_to_fix_it() {
    let _g = global_lock();
    ffmpeg::set_override(None);
    let _ = ffmpeg::set_configured_path(None);
    if let Ok(f) = ffmpeg::find() {
        eprintln!(
            "übersprungen: ein ffmpeg ist installiert ({})",
            f.path.display()
        );
        return;
    }
    let dir = temp("ff-missing");
    let v = fake_video(&dir, "clip.mp4", "FAKEVIDEO 40 30 3000 30");
    let st = ffmpeg::status();
    assert!(!st.is_ok());
    assert_eq!(st.message, fp_i18n::t("batch-anim-err-ffmpeg-missing"));
    let out = dir.join("out");
    let mut r = recipe(&out, "gif");
    r.add_step(anim::VIDEO_IMPORT, json!({}));
    let rep = run(&r, std::slice::from_ref(&v));
    assert_eq!(rep.items[0].status, ItemStatus::Failed);
    assert!(
        rep.items[0]
            .error
            .as_deref()
            .unwrap_or("")
            .contains(&fp_i18n::t("batch-anim-err-ffmpeg-missing")),
        "{:?}",
        rep.items[0].error
    );
    assert!(!has_temp_files(&dir));
    // auf Englisch
    let en = fp_i18n::with_language("en", || ffmpeg::missing_error().to_string());
    assert!(en.starts_with("ffmpeg not found"), "{en}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn real_ffmpeg_if_installed() {
    let _g = global_lock();
    ffmpeg::set_override(None);
    let Ok(ff) = ffmpeg::find() else {
        eprintln!(
            "übersprungen: kein ffmpeg installiert (Einstellung, FIREPIXEL_FFMPEG, PATH) – es wird nichts heruntergeladen"
        );
        return;
    };
    let version = ff.version().expect("version");
    eprintln!("echtes ffmpeg {version}: {}", ff.path.display());
    let dir = temp("ff-real");
    let v = dir.join("test.mp4");
    // Testvideo mit dem ffmpeg des Nutzers erzeugen (lavfi testsrc, 2 s)
    let args: Vec<String> = [
        "-hide_banner",
        "-loglevel",
        "error",
        "-y",
        "-f",
        "lavfi",
        "-i",
        "testsrc=size=160x120:rate=25:duration=2",
        "-pix_fmt",
        "yuv420p",
    ]
    .iter()
    .map(|s| s.to_string())
    .chain([v.to_string_lossy().to_string()])
    .collect();
    let st = ff.command().args(&args).status().expect("ffmpeg");
    assert!(st.success());
    let info = ff.probe(&v).expect("probe");
    assert_eq!((info.width, info.height), (160, 120));
    let it = video::decode_with(
        &ff,
        &v,
        &VideoParams::from_json(&json!({"start": 500, "end": 1500, "fps": 10})),
        &CancelToken::new(),
    )
    .expect("decode");
    assert_eq!(frames_of(&it).len(), 10);
    assert_eq!(delays(&it).iter().sum::<u32>(), 1000);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn catalog_lists_animation_steps_with_texts_in_both_languages() {
    let reg = StepRegistry::global();
    for id in [
        anim::VIDEO_IMPORT,
        anim::TRIM,
        anim::SPEED,
        anim::LOOP,
        anim::DROP_FRAMES,
        anim::FRAME_DELAY,
        anim::TO_FRAMES,
        anim::FROM_FRAMES,
        anim::SPRITE_SHEET,
    ] {
        let s = reg.get(id).expect(id);
        assert_eq!(s.group(), fp_batch::StepGroup::Animation);
        for lang in ["de", "en"] {
            fp_i18n::with_language(lang, || {
                let k = fp_batch::step::step_key(id);
                assert!(fp_i18n::has(&k), "{lang}: {k}");
                for p in s.params() {
                    let pk = fp_batch::step::param_key(id, &p.name);
                    assert!(fp_i18n::has(&pk), "{lang}: {pk}");
                    if let fp_batch::ParamType::Enum { values } = &p.ty {
                        for v in values {
                            let vk = fp_batch::step::value_key(id, &p.name, v); // BATCH-11a: ein Schema
                            assert!(fp_i18n::has(&vk), "{lang}: {vk}");
                        }
                    }
                }
            });
        }
    }
    let _ = Arc::clone(&reg);
}

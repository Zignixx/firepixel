//! Lauf eines Rezepts (SPEC-BATCH §5.2): Kodieren, Ziele, Konflikte, Sicherung, Abbrechen,
//! Fehler mitten in der Liste, Trockenlauf, reine Dateischritte, Speicherbudget, Sammel-Schritt,
//! Rückfrage, Fortschritt, Vorschau – mit Logikfällen nach AGENTEN_REGELN §4b.

mod support;

use fp_batch::item::{Item, ItemKind};
use fp_batch::naming::PlanOptions;
use fp_batch::recipe::{Conflict, NamePart, TargetKind, TargetSpec};
use fp_batch::step::{ParamDef, Step, StepContext, StepGroup, StepKind, StepRegistry};
use fp_batch::{
    AskChoice, BatchError, CancelToken, ItemStatus, Progress, Recipe, RunOptions, executor,
};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use support::*;

/// Testschritt: kehrt die Farben um (einmal angewendet = einmal umgekehrt).
struct Invert;
impl Step for Invert {
    fn id(&self) -> &str {
        "test.invert"
    }
    fn group(&self) -> StepGroup {
        StepGroup::Color
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, _p: &Value, ctx: &mut StepContext) -> fp_batch::Result<()> {
        item.map_images(|img| {
            ctx.check_cancel()?;
            for px in img.data_mut() {
                px[0] = 255 - px[0];
                px[1] = 255 - px[1];
                px[2] = 255 - px[2];
            }
            Ok(())
        })
    }
}

/// Testschritt: schlägt bei Dateien fehl, deren Name `bad` enthält.
struct FailBad;
impl Step for FailBad {
    fn id(&self) -> &str {
        "test.failBad"
    }
    fn group(&self) -> StepGroup {
        StepGroup::Advanced
    }
    fn apply(&self, _item: &mut Item, _p: &Value, ctx: &mut StepContext) -> fp_batch::Result<()> {
        if ctx.info.file_name().contains("bad") {
            Err(BatchError::Invalid("boom".into()))
        } else {
            Ok(())
        }
    }
}

/// Testschritt: bricht beim Element mit Index `at` ab (wie der Abbrechen-Knopf mitten im Lauf).
struct CancelAt;
impl Step for CancelAt {
    fn id(&self) -> &str {
        "test.cancelAt"
    }
    fn group(&self) -> StepGroup {
        StepGroup::Advanced
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::int("at", 0, 1000, 0)]
    }
    fn apply(&self, _item: &mut Item, p: &Value, ctx: &mut StepContext) -> fp_batch::Result<()> {
        if ctx.index as i64 == p["at"].as_i64().unwrap_or(0) {
            ctx.cancel.cancel();
        }
        ctx.check_cancel()
    }
}

/// Sammel-Schritt: alle Elemente → ein Bild (Breite = Summe der Breiten).
struct Merge;
impl Step for Merge {
    fn id(&self) -> &str {
        "test.merge"
    }
    fn group(&self) -> StepGroup {
        StepGroup::Overlay
    }
    fn kind(&self) -> StepKind {
        StepKind::Collect
    }
    fn apply(&self, _item: &mut Item, _p: &Value, _ctx: &mut StepContext) -> fp_batch::Result<()> {
        Ok(())
    }
    fn collect(
        &self,
        items: Vec<Item>,
        _p: &Value,
        _ctx: &mut StepContext,
    ) -> fp_batch::Result<Vec<Item>> {
        let w: u32 = items.iter().map(|i| i.dimensions().0).sum();
        let h = items.iter().map(|i| i.dimensions().1).max().unwrap_or(1);
        Ok(vec![fp_batch::item::still_from_rgba(
            w,
            h,
            &rgba(w, h, false),
        )?])
    }
}

fn registry() -> Arc<StepRegistry> {
    let mut r = StepRegistry::with_defaults();
    r.register(Arc::new(Invert));
    r.register(Arc::new(FailBad));
    r.register(Arc::new(CancelAt));
    r.register(Arc::new(Merge));
    Arc::new(r)
}

fn opts() -> RunOptions {
    RunOptions {
        registry: Some(registry()),
        threads: Some(3),
        ..RunOptions::default()
    }
}

fn folder_recipe(out: &Path, format: &str) -> Recipe {
    let mut r = Recipe::named("test");
    r.output.format = format.into();
    r.output.target = TargetSpec {
        kind: TargetKind::Folder,
        path: Some(out.to_path_buf()),
        ..TargetSpec::default()
    };
    r
}

fn sources(dir: &Path, n: usize) -> Vec<PathBuf> {
    (0..n)
        .map(|k| {
            let p = dir.join(format!("bild{k}.png"));
            write_png(&p, 8 + k as u32, 6);
            p
        })
        .collect()
}

fn items(paths: &[PathBuf]) -> Vec<fp_batch::ItemInfo> {
    fp_batch::collect(paths, true, false, &[]).items
}

#[test]
fn converts_into_new_folder_and_keeps_originals() {
    let dir = temp("convert");
    let src = sources(&dir, 4);
    let sums: Vec<u64> = src.iter().map(|p| checksum(p)).collect();
    let mut r = Recipe::named("web");
    r.output.format = "webp".into();
    r.add_step("test.invert", json!({}));
    let rep = executor::run(&r, &items(&src), &opts()).expect("run");
    assert_eq!(rep.totals.done, 4, "{:#?}", rep.items);
    let out = dir.join("batch-output");
    for (k, p) in src.iter().enumerate() {
        assert_eq!(checksum(p), sums[k], "Original unverändert");
        let t = out.join(format!("bild{k}.webp"));
        assert_eq!(dims(&t), (8 + k as u32, 6));
    }
    // Bericht im Ausgabeordner, Schlüssel englisch
    let report: Value = serde_json::from_str(
        &std::fs::read_to_string(out.join("batch-report.json")).expect("report"),
    )
    .expect("json");
    assert_eq!(report["format"], "firepixel-batch-report");
    assert_eq!(report["items"].as_array().map(Vec::len), Some(4));
    for k in [
        "source", "target", "status", "bytesIn", "bytesOut", "ms", "warnings",
    ] {
        assert!(report["items"][0].get(k).is_some(), "{k}");
    }
    assert!(!has_temp_files(&dir));
    // zweiter Lauf → neuer Ordner batch-output-2, nie doppelte Effekte im Bild
    let rep2 = executor::run(&r, &items(&src), &opts()).expect("run 2");
    assert_eq!(rep2.totals.done, 4);
    let a = pixels(&out.join("bild0.webp"));
    let b = pixels(&dir.join("batch-output-2").join("bild0.webp"));
    assert_eq!(a, b, "zweiter Lauf rechnet wieder vom Original");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn effect_applied_exactly_once_per_run_png_exact() {
    let dir = temp("once");
    let src = sources(&dir, 1);
    let mut r = folder_recipe(&dir.join("out"), "png");
    r.add_step("test.invert", json!({}));
    executor::run(&r, &items(&src), &opts()).expect("run");
    let orig = pixels(&src[0]);
    let got = pixels(&dir.join("out").join("bild0.png"));
    let expect: Vec<u8> = orig
        .chunks(4)
        .flat_map(|p| [255 - p[0], 255 - p[1], 255 - p[2], p[3]])
        .collect();
    assert_eq!(got, expect);
    // Lauf zweimal in denselben Ordner → name (2), gleiche Pixel
    executor::run(&r, &items(&src), &opts()).expect("run 2");
    assert_eq!(pixels(&dir.join("out").join("bild0 (2).png")), expect);
    // Regel „überspringen“ → dritter Lauf schreibt nichts Neues
    r.output.conflict = Conflict::Skip;
    let rep = executor::run(&r, &items(&src), &opts()).expect("run 3");
    assert_eq!(rep.totals.skipped, 1);
    assert_eq!(
        files_under(&dir.join("out"))
            .iter()
            .filter(|p| p.extension().is_some_and(|e| e == "png"))
            .count(),
        2
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn broken_file_in_the_middle_does_not_stop_the_run() {
    let dir = temp("broken");
    let mut src = sources(&dir, 3);
    let bad = dir.join("kaputt.png");
    std::fs::write(&bad, b"\x89PNG\r\n\x1a\nnot really").expect("bad");
    src.insert(1, bad);
    let r = folder_recipe(&dir.join("out"), "jpeg");
    let infos = items(&src);
    assert_eq!(infos.len(), 4);
    let rep = executor::run(&r, &infos, &opts()).expect("run");
    assert_eq!(rep.totals.done, 3, "{:#?}", rep.items);
    assert_eq!(rep.totals.failed, 1);
    assert_eq!(rep.items[1].status, ItemStatus::Failed);
    assert!(rep.items[1].error.is_some());
    assert_eq!(rep.exit_code(), 1);
    assert!(!has_temp_files(&dir));
    // Fehler eines Schritts: Schritt im Bericht
    let mut r = folder_recipe(&dir.join("out2"), "png");
    r.add_step("test.failBad", json!({}));
    let p = dir.join("bad_one.png");
    write_png(&p, 3, 3);
    let rep = executor::run(&r, &items(&[p, src[0].clone()]), &opts()).expect("run");
    assert_eq!(rep.items[0].step.as_deref(), Some("test.failBad"));
    assert_eq!(rep.items[1].status, ItemStatus::Done);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn zero_byte_and_unknown_files_are_not_collected() {
    let dir = temp("zero");
    std::fs::write(dir.join("leer.png"), b"").expect("zero");
    std::fs::write(dir.join("notiz.txt"), b"hallo").expect("txt");
    write_png(&dir.join("ok.png"), 2, 2);
    let c = fp_batch::collect(std::slice::from_ref(&dir), false, false, &[]);
    assert_eq!(c.items.len(), 1);
    assert_eq!(c.skipped.len(), 2);
    // Doppelte werden nicht erneut aufgenommen
    let again = fp_batch::collect(&[dir.join("ok.png")], false, false, &c.items);
    assert!(again.items.is_empty());
    assert_eq!(again.duplicates.len(), 1);
    // nur Dateischritte → alle Dateien erlaubt
    let all = fp_batch::collect(std::slice::from_ref(&dir), false, true, &[]);
    assert_eq!(all.items.len(), 3);
    let _ = std::fs::remove_dir_all(&dir);
}

/// BA-106: an external cancel (Ctrl+C in the CLI, window close in the UI) from another thread
/// mid-run stops scheduling, and no `.<name>.fptmp` survives, not even a stale one left by an
/// earlier interrupted run for a planned target.
#[test]
fn external_cancel_mid_run_removes_all_temp_files() {
    let dir = temp("cancel_ext");
    let src = sources(&dir, 10);
    let out = dir.join("out");
    std::fs::create_dir_all(&out).expect("out");
    for k in 6..10 {
        std::fs::write(out.join(format!(".bild{k}.png.fptmp")), b"stale").expect("stale");
    }
    let mut o = opts();
    o.threads = Some(1);
    let h = executor::spawn(folder_recipe(&out, "png"), items(&src), o);
    for ev in h.events.iter() {
        if matches!(ev, Progress::ItemDone { .. }) {
            h.cancel(); // like the Ctrl+C handler
        }
        if matches!(ev, Progress::Finished(_)) {
            break;
        }
    }
    let rep = h.join().expect("run");
    assert!(rep.cancelled);
    assert!(
        rep.totals.done >= 1 && rep.totals.done < 10,
        "{:?}",
        rep.totals
    );
    assert!(rep.totals.cancelled > 0);
    assert!(
        !has_temp_files(&dir),
        "no .fptmp left: {:?}",
        files_under(&dir)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cancel_mid_run_leaves_no_temp_files_and_complete_outputs() {
    let dir = temp("cancel");
    let src = sources(&dir, 12);
    let sums: Vec<u64> = src.iter().map(|p| checksum(p)).collect();
    let mut r = folder_recipe(&dir.join("out"), "png");
    r.add_step("test.cancelAt", json!({"at": 5}));
    let mut o = opts();
    o.threads = Some(1);
    let rep = executor::run(&r, &items(&src), &o).expect("run");
    assert!(rep.cancelled);
    assert_eq!(rep.exit_code(), 3);
    assert_eq!(
        rep.totals.done,
        5,
        "{:#?}",
        rep.items.iter().map(|i| i.status).collect::<Vec<_>>()
    );
    assert_eq!(rep.totals.cancelled, 7);
    assert!(!has_temp_files(&dir), "keine .fptmp-Reste");
    for p in files_under(&dir.join("out"))
        .iter()
        .filter(|p| p.extension().is_some_and(|e| e == "png"))
    {
        let _ = dims(p); // vollständig lesbar
    }
    for (k, p) in src.iter().enumerate() {
        assert_eq!(checksum(p), sums[k], "Originale unverändert");
    }
    assert!(rep.summary_text().contains('5'));
    // Abbruch vor dem Start: nichts geschrieben
    let o2 = opts();
    o2.cancel.cancel();
    let rep =
        executor::run(&folder_recipe(&dir.join("out3"), "png"), &items(&src), &o2).expect("run");
    assert_eq!(rep.totals.cancelled, 12);
    assert!(files_under(&dir.join("out3")).is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn dry_run_writes_nothing() {
    let dir = temp("dry");
    let src = sources(&dir, 3);
    let o = RunOptions {
        dry_run: true,
        ..opts()
    };
    let rep = executor::run(&Recipe::default(), &items(&src), &o).expect("run");
    assert!(rep.dry_run);
    assert_eq!(rep.totals.planned, 3);
    assert!(!dir.join("batch-output").exists());
    assert!(rep.items.iter().all(|i| {
        i.target
            .as_ref()
            .is_some_and(|t| t.starts_with(dir.join("batch-output")))
    }));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn file_only_copy_and_move_with_counter_names() {
    let dir = temp("fileonly");
    std::fs::write(dir.join("b.txt"), b"bbb").expect("b");
    std::fs::write(dir.join("a.txt"), b"aaa").expect("a");
    write_png(&dir.join("c.png"), 2, 2);
    let mut r = folder_recipe(&dir.join("out"), "same");
    r.add_step("file.copyOnly", json!({"action": "copy"}));
    r.rename.parts = vec![
        NamePart::Text {
            text: "doc_".into(),
        },
        NamePart::Counter {
            start: 1,
            step: 1,
            digits: 2,
            per_folder: false,
        },
    ];
    let reg = registry();
    assert!(fp_batch::pipeline::is_file_only(&r, &reg));
    let infos = fp_batch::collect(std::slice::from_ref(&dir), false, true, &[]).items;
    let rep = executor::run(&r, &infos, &opts()).expect("run");
    assert_eq!(rep.totals.done, 3);
    assert_eq!(
        std::fs::read(dir.join("out").join("doc_01.txt")).expect("a"),
        b"aaa",
        "natürlich sortiert, Bytes unverändert"
    );
    assert_eq!(
        std::fs::read(dir.join("out").join("doc_02.txt")).expect("b"),
        b"bbb"
    );
    assert_eq!(
        checksum(&dir.join("out").join("doc_03.png")),
        checksum(&dir.join("c.png"))
    );
    assert!(dir.join("a.txt").exists());
    // verschieben
    r.set_param(0, "action", json!("move"));
    r.output.target.path = Some(dir.join("moved"));
    let infos = fp_batch::collect(&[dir.join("a.txt")], false, true, &[]).items;
    let rep = executor::run(&r, &infos, &opts()).expect("move");
    assert_eq!(rep.totals.done, 1);
    assert!(!dir.join("a.txt").exists());
    assert_eq!(
        std::fs::read(dir.join("moved").join("doc_01.txt")).expect("moved"),
        b"aaa"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn replace_original_moves_it_to_backup_folder() {
    let dir = temp("replace");
    let src = sources(&dir, 1);
    let before = checksum(&src[0]);
    let mut r = Recipe::named("in place");
    r.output.target = TargetSpec {
        kind: TargetKind::SameAsSource,
        ..TargetSpec::default()
    };
    r.output.conflict = Conflict::Overwrite;
    r.output.backup.incremental = true;
    r.add_step("test.invert", json!({}));
    let rep = executor::run(&r, &items(&src), &opts()).expect("run");
    assert_eq!(rep.totals.done, 1, "{:#?}", rep.items);
    let backup = dir.join("Original Files").join("bild0.png");
    assert_eq!(checksum(&backup), before, "Sicherung = Original");
    assert_ne!(checksum(&src[0]), before, "Original ersetzt");
    assert_eq!(rep.items[0].backup.as_deref(), Some(backup.as_path()));
    assert!(
        rep.report_file.is_none(),
        "am Originalort kein Bericht in den Quellordner"
    );
    // zweites Mal: inkrementelle Sicherung bild0-1.png
    executor::run(&r, &items(&src), &opts()).expect("run 2");
    assert!(dir.join("Original Files").join("bild0-1.png").exists());
    assert_eq!(
        pixels(&src[0]),
        pixels(&backup),
        "zweimal umgekehrt = Original"
    );
    assert!(!has_temp_files(&dir));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_only_source_and_unicode_names() {
    let dir = temp("unicode");
    let p = dir.join("Ürlaub 🌴 Bild.png");
    write_png(&p, 5, 5);
    let mut perm = std::fs::metadata(&p).expect("meta").permissions();
    perm.set_readonly(true);
    std::fs::set_permissions(&p, perm.clone()).expect("ro");
    let rep = executor::run(
        &folder_recipe(&dir.join("aus gabe"), "webp"),
        &items(std::slice::from_ref(&p)),
        &opts(),
    )
    .expect("run");
    assert_eq!(rep.totals.done, 1, "{:#?}", rep.items);
    assert!(dir.join("aus gabe").join("Ürlaub 🌴 Bild.webp").exists());
    #[allow(clippy::permissions_set_readonly_false)]
    perm.set_readonly(false);
    let _ = std::fs::set_permissions(&p, perm);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn long_output_paths_are_written() {
    let dir = temp("long");
    let src = sources(&dir, 1);
    let mut out = dir.clone();
    for k in 0..12 {
        out = out.join(format!("sehr-langer-ordnername-nummer-{k:02}"));
    }
    assert!(out.to_string_lossy().len() > 300);
    let rep = executor::run(&folder_recipe(&out, "png"), &items(&src), &opts()).expect("run");
    assert_eq!(rep.totals.done, 1, "{:#?}", rep.items);
    assert!(
        rep.items[0]
            .warnings
            .iter()
            .any(|w| w == &fp_batch::naming::NameHint::LongPath.message())
    );
    assert!(fp_batch::output::long_path(&out.join("bild0.png")).exists());
    let _ = std::fs::remove_dir_all(fp_batch::output::long_path(&dir));
}

#[test]
fn tiny_memory_budget_and_thread_count_do_not_change_the_result() {
    let dir = temp("budget");
    let src = sources(&dir, 6);
    let r1 = folder_recipe(&dir.join("one"), "png");
    let r4 = folder_recipe(&dir.join("four"), "png");
    let o1 = RunOptions {
        threads: Some(1),
        memory_budget: Some(1),
        ..opts()
    };
    let o4 = RunOptions {
        threads: Some(4),
        ..opts()
    };
    let a = executor::run(&r1, &items(&src), &o1).expect("one");
    let b = executor::run(&r4, &items(&src), &o4).expect("four");
    assert_eq!(a.totals.done, 6);
    assert_eq!(b.totals.done, 6);
    let names = |r: &fp_batch::Report| {
        r.items
            .iter()
            .map(|i| {
                i.target
                    .as_ref()
                    .and_then(|t| t.file_name())
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        names(&a),
        names(&b),
        "Ziele unabhängig von der Parallelität"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ask_rule_uses_the_answer_for_all() {
    let dir = temp("ask");
    let src = sources(&dir, 3);
    let mut r = folder_recipe(&dir.join("out"), "png");
    executor::run(&r, &items(&src), &opts()).expect("first");
    r.output.conflict = Conflict::Ask;
    let asked = Arc::new(AtomicUsize::new(0));
    let a2 = asked.clone();
    let o = RunOptions {
        ask: Some(Arc::new(move |_e| {
            a2.fetch_add(1, Ordering::SeqCst);
            (AskChoice::Skip, true)
        })),
        ..opts()
    };
    let rep = executor::run(&r, &items(&src), &o).expect("ask");
    assert_eq!(
        asked.load(Ordering::SeqCst),
        1,
        "„für alle übernehmen“ fragt nur einmal"
    );
    assert_eq!(rep.totals.skipped, 3);
    // ohne Rückfrage: wie „neuen Namen wählen“
    let rep = executor::run(&r, &items(&src), &opts()).expect("no ask");
    assert_eq!(rep.totals.done, 3);
    assert!(dir.join("out").join("bild0 (2).png").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn collect_step_runs_in_second_phase() {
    let dir = temp("collect");
    let src = sources(&dir, 3);
    let mut r = folder_recipe(&dir.join("out"), "png");
    r.add_step("test.invert", json!({}));
    r.add_step("test.merge", json!({}));
    let rep = executor::run(&r, &items(&src), &opts()).expect("run");
    assert_eq!(rep.totals.done, 3);
    assert_eq!(dims(&dir.join("out").join("bild0.png")), (8 + 9 + 10, 6));
    assert_eq!(
        files_under(&dir.join("out"))
            .iter()
            .filter(|p| p.extension().is_some_and(|e| e == "png"))
            .count(),
        1
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn progress_events_and_background_job() {
    let dir = temp("progress");
    let src = sources(&dir, 4);
    let handle = executor::spawn(folder_recipe(&dir.join("out"), "png"), items(&src), opts());
    let mut done = 0;
    let mut started = false;
    let mut finished = false;
    for ev in handle.events.iter() {
        match ev {
            Progress::Started { total } => {
                started = true;
                assert_eq!(total, 4);
            }
            Progress::ItemDone { .. } => done += 1,
            Progress::Finished(r) => {
                finished = true;
                assert_eq!(r.totals.done, 4);
            }
            _ => {}
        }
    }
    assert!(started && finished);
    assert_eq!(done, 4);
    assert_eq!(handle.join().expect("join").totals.done, 4);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn animation_round_trip_and_single_frame() {
    let dir = temp("anim");
    let frames: Vec<fp_batch::item::Frame> = (0..3)
        .map(|k| {
            let mut px = rgba(6, 4, false);
            for p in px.chunks_mut(4) {
                p[2] = (k * 80) as u8;
            }
            let Item::Still(s) = fp_batch::item::still_from_rgba(6, 4, &px).expect("frame") else {
                unreachable!()
            };
            fp_batch::item::Frame {
                image: s.image,
                delay_ms: 100,
            }
        })
        .collect();
    let anim = Item::Animation(fp_batch::item::Animation {
        width: 6,
        height: 4,
        frames,
        loop_count: 0,
        meta: Default::default(),
    });
    let (bytes, _) =
        fp_batch::output::encode(&anim, fp_batch::OutputFormat::Gif, &Default::default())
            .expect("gif");
    let gif = dir.join("clip.gif");
    std::fs::write(&gif, bytes).expect("write");
    let info = fp_batch::probe(&gif).expect("probe");
    assert_eq!((info.width, info.height), (6, 4));
    let mut r = folder_recipe(&dir.join("out"), "same");
    r.add_step("test.invert", json!({}));
    let rep = executor::run(&r, &[info], &opts()).expect("run");
    assert_eq!(rep.totals.done, 1, "{:#?}", rep.items);
    let out = std::fs::read(dir.join("out").join("clip.gif")).expect("out");
    match fp_batch::item::decode_bytes("clip.gif", &out).expect("decode") {
        Item::Animation(a) => assert_eq!(a.frames.len(), 3),
        other => panic!("keine Animation: {:?}", other.kind()),
    }
    // Animation mit einem Bild = Standbild; Animation → JPEG: nur erstes Bild + Warnung
    let one = Item::Animation(fp_batch::item::Animation {
        width: 6,
        height: 4,
        frames: vec![],
        loop_count: 0,
        meta: Default::default(),
    });
    assert!(one.preview_image().is_err());
    let mut rj = folder_recipe(&dir.join("jpg"), "jpeg");
    rj.add_step("test.invert", json!({}));
    let rep = executor::run(&rj, &[fp_batch::probe(&gif).expect("probe")], &opts()).expect("jpeg");
    assert!(
        rep.items[0].warnings.iter().any(|w| w.contains("JPEG")),
        "{:?}",
        rep.items[0].warnings
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn preview_replaces_instead_of_adding_and_respects_generation() {
    let dir = temp("preview");
    let src = sources(&dir, 1);
    let big = dir.join("big.png");
    write_png(&big, 2000, 1000);
    let mut r = folder_recipe(&dir.join("out"), "png");
    r.add_step("test.invert", json!({}));
    let reg = registry();
    let info = fp_batch::probe(&src[0]).expect("probe");
    let o = fp_batch::PreviewOptions::default();
    let a = fp_batch::preview(&r, &info, &reg, &o).expect("preview");
    let b = fp_batch::preview(&r, &info, &reg, &o).expect("preview 2");
    assert_eq!(
        a.after.as_ref().map(|i| i.as_bytes().to_vec()),
        b.after.as_ref().map(|i| i.as_bytes().to_vec()),
        "zweite Vorschau = erste (nicht doppelt)"
    );
    let orig = pixels(&src[0]);
    let inv: Vec<u8> = orig
        .chunks(4)
        .flat_map(|p| [255 - p[0], 255 - p[1], 255 - p[2], p[3]])
        .collect();
    assert_eq!(a.after.expect("after").as_bytes(), &inv[..]);
    // Proxy: längste Seite 1024, hochgerechnete Größe stimmt
    let binfo = fp_batch::probe(&big).expect("probe big");
    let p = fp_batch::preview(
        &r,
        &binfo,
        &reg,
        &fp_batch::PreviewOptions {
            encode_full: true,
            ..Default::default()
        },
    )
    .expect("big");
    assert_eq!(p.before.width(), 1024);
    assert_eq!(p.after_size, (2000, 1000));
    assert!(p.bytes_after.is_some_and(|b| b > 0));
    // neue Generation bricht die alte ab
    let generation = Arc::new(AtomicU64::new(1));
    let tok = CancelToken::for_generation(generation.clone(), 1);
    generation.store(2, Ordering::SeqCst);
    let e = fp_batch::preview(
        &r,
        &info,
        &reg,
        &fp_batch::PreviewOptions {
            cancel: tok,
            ..Default::default()
        },
    )
    .expect_err("veraltet");
    assert!(e.is_cancelled());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn engine_commands_through_the_batch_host() {
    let dir = temp("host");
    let src = sources(&dir, 2);
    let mut s = fp_engine::Session::default();
    fp_batch::install_host(&mut s);
    let steps = s
        .execute("batch.steps", json!({"language": "en"}))
        .expect("steps")
        .value;
    assert!(
        steps["steps"]
            .as_array()
            .is_some_and(|a| a.iter().any(|x| x["id"] == "file.copyOnly"))
    );
    let v = s
        .execute(
            "batch.validate",
            json!({"recipe": {"output": {"format": "bogus"}}}),
        )
        .expect("validate")
        .value;
    assert_eq!(v["ok"], false);
    let recipe = json!({"output": {"format": "webp"}});
    let plan = s
        .execute(
            "batch.plan",
            json!({"recipe": recipe, "files": [src[0], src[1]], "out": dir.join("o")}),
        )
        .expect("plan")
        .value;
    assert_eq!(plan["entries"].as_array().map(Vec::len), Some(2));
    assert!(!dir.join("o").exists());
    let run = s
        .execute(
            "batch.run",
            json!({"recipe": recipe, "folder": dir, "recursive": false, "out": dir.join("o")}),
        )
        .expect("run")
        .value;
    assert_eq!(run["totals"]["done"], 2, "{run}");
    assert_eq!(run["ok"], true);
    let prev = s
        .execute(
            "batch.preview",
            json!({"recipe": recipe, "file": src[0], "maxSize": 64}),
        )
        .expect("preview")
        .value;
    assert_eq!(prev["before"]["width"], 8);
    assert!(
        prev["after"]["png"]
            .as_str()
            .is_some_and(|p| p.starts_with("iVBOR"))
    );
    // Rezept als Datei
    let rp = dir.join("r.fpbatch.json");
    Recipe::from_value(recipe.clone())
        .expect("r")
        .save(&rp)
        .expect("save");
    let run = s
        .execute(
            "batch.run",
            json!({"recipe": rp, "files": [src[0]], "dryRun": true}),
        )
        .expect("dry")
        .value;
    assert_eq!(run["totals"]["planned"], 1);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = PlanOptions::default();
}

// ------------------------------------------------------------------ F5 (BATCH-T-Reste)

/// Abbruch: Summe vorher/nachher nur über fertige Dateien (vorher „1,9 MB → 0 Byte (−100 %)“) und
/// Meldung „Abgebrochen: n fertig, m nicht verarbeitet“ (SPEC §2.8).
#[test]
fn f5_cancelled_report_sums_only_done_items() {
    let mut rep = fp_batch::Report {
        format: fp_batch::report::REPORT_FORMAT.into(),
        version: 1,
        recipe: "x".into(),
        started: String::new(),
        finished: String::new(),
        dry_run: false,
        cancelled: true,
        totals: Default::default(),
        items: Vec::new(),
        output_dirs: Vec::new(),
        warnings: Vec::new(),
        report_file: None,
        report_html: None,
    };
    let mut done = fp_batch::report::ItemReport::new(0, Path::new("a.png"), 1000);
    done.status = ItemStatus::Done;
    done.bytes_out = 400;
    let mut rest = fp_batch::report::ItemReport::new(1, Path::new("b.png"), 9000);
    rest.status = ItemStatus::Cancelled;
    let mut wait = fp_batch::report::ItemReport::new(2, Path::new("c.png"), 5000);
    wait.status = ItemStatus::Waiting;
    rep.items = vec![done, rest, wait];
    rep.recount();
    assert_eq!((rep.totals.done, rep.totals.cancelled), (1, 2));
    assert_eq!(
        (rep.totals.bytes_in, rep.totals.bytes_out),
        (1000, 400),
        "nur die fertige Datei zählt"
    );
    let txt = fp_i18n::with_language("de", || rep.summary_text());
    assert_eq!(txt, "Abgebrochen: 1 fertig, 2 nicht verarbeitet");
}

/// KI-Schritte begrenzen die gleichzeitig laufenden Elemente (BATCH-T: 24 Fotos liefen alle sofort).
#[test]
fn f5_ai_steps_limit_parallel_items() {
    let reg = StepRegistry::with_defaults();
    let mut r = Recipe::named("ki");
    r.add_step("resize", json!({}));
    assert_eq!(
        fp_batch::Pipeline::new(&r, &reg).expect("p").max_parallel(),
        None,
        "ohne KI frei"
    );
    r.add_step("background.remove", json!({}));
    assert_eq!(
        fp_batch::Pipeline::new(&r, &reg).expect("p").max_parallel(),
        Some(fp_batch::steps::background::AI_PARALLEL)
    );
    let mut c = Recipe::named("farbe");
    c.add_step("background.replace", json!({"method": "color"}));
    assert_eq!(
        fp_batch::Pipeline::new(&c, &reg).expect("p").max_parallel(),
        None,
        "Farbschlüssel ist kein KI-Schritt"
    );
}

/// Reines Umbenennen/Kopieren: Vorschau „Nachher“ = unveränderte Kopie, nicht neu kodiert (BATCH-T B9).
#[test]
fn f5_preview_of_rename_only_reports_source_size() {
    let dir = temp("f5-rename");
    let p = dir.join("foto.png");
    write_png(&p, 40, 30);
    let mut r = folder_recipe(&dir.join("out"), "same");
    r.rename.parts = vec![
        NamePart::Text {
            text: "neu_".into(),
        },
        NamePart::Counter {
            start: 1,
            step: 1,
            digits: 2,
            per_folder: false,
        },
    ];
    let info = items(std::slice::from_ref(&p)).remove(0);
    let reg = StepRegistry::with_defaults();
    let opts = fp_batch::PreviewOptions {
        encode_full: true,
        ..fp_batch::PreviewOptions::default()
    };
    let res = fp_batch::preview(&r, &info, &reg, &opts).expect("preview");
    assert_eq!(
        res.bytes_after,
        Some(std::fs::metadata(&p).expect("m").len())
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// BA-105: the size token names the **output** size (after `resize`), and the name conflicts
/// this creates are resolved at that point (`name (2)` …, or skip).
#[test]
fn size_token_uses_output_size_and_resolves_new_conflicts() {
    use fp_batch::recipe::SizeWhich;
    let dir = temp("size_token");
    let src = sources(&dir, 3); // 8x6, 9x6, 10x6 → distinct planned names
    let out = dir.join("out");
    let recipe = |conflict| {
        let mut r = folder_recipe(&out, "png");
        r.rename.parts = vec![
            NamePart::Text {
                text: "img_".into(),
            },
            NamePart::Size {
                which: SizeWhich::Both,
            },
        ];
        r.output.conflict = conflict;
        r.add_step(
            "resize",
            json!({"mode": "exact", "width": 20, "height": 10, "onlyShrink": false}),
        );
        r
    };
    let rep = executor::run(&recipe(Conflict::Rename), &items(&src), &opts()).expect("run");
    assert_eq!(rep.totals.done, 3, "{:#?}", rep.items);
    let pngs = || {
        files_under(&out)
            .into_iter()
            .filter(|p| p.extension().is_some_and(|e| e == "png"))
            .collect::<Vec<_>>()
    };
    let mut names: Vec<String> = pngs()
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    names.sort();
    assert_eq!(
        names,
        ["img_20x10 (2).png", "img_20x10 (3).png", "img_20x10.png"]
    );
    for i in &rep.items {
        let t = i.target.as_ref().expect("target");
        assert!(t.exists(), "report names the written file: {}", t.display());
        assert_eq!(dims(t), (20, 10));
    }
    // existing files: "skip" leaves them alone and skips every item
    let rep = executor::run(&recipe(Conflict::Skip), &items(&src), &opts()).expect("run");
    assert_eq!(rep.totals.done, 0, "{:#?}", rep.items);
    assert_eq!(rep.totals.skipped, 3);
    assert_eq!(pngs().len(), 3);
    assert!(!has_temp_files(&dir));
    // without a size step the source size stays (name unchanged)
    let mut r = recipe(Conflict::Rename);
    r.steps.clear();
    let rep = executor::run(&r, &items(&src[..1]), &opts()).expect("run");
    let t = rep.items[0].target.clone().expect("target");
    assert_eq!(t.file_name().unwrap().to_string_lossy(), "img_8x6.png");
    let _ = std::fs::remove_dir_all(&dir);
}

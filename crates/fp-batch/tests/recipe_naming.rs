//! Rezept-Modell (JSON, Migration, Validierung, Bearbeiten) und Namensbildung/Plan (SPEC-BATCH
//! §2.5, §2.7, §4.4) inklusive Logikfällen nach AGENTEN_REGELN §4b.

mod support;

use chrono::NaiveDate;
use fp_batch::item::ItemInfo;
use fp_batch::naming::{self, build_name, split_extension, NameContext, NameHint, PlanAction, PlanOptions};
use fp_batch::recipe::*;
use fp_batch::{ItemKind, Recipe, Severity, StepRegistry};
use serde_json::json;
use std::path::{Path, PathBuf};

const SPEC_JSON: &str = r##"{
  "format": "firepixel-batch-recipe",
  "version": 1,
  "name": "Für das Web",
  "description": "",
  "steps": [
    { "id": "resize", "enabled": true, "params": { "mode": "longEdge", "size": 1600, "onlyShrink": true, "filter": "lanczos3" } },
    { "id": "metadata.edit", "enabled": true, "params": { "keep": "all", "remove": ["gps"] } },
    { "id": "background.flatten", "enabled": false, "params": { "color": "#ffffff" } }
  ],
  "rename": { "mode": "builder", "parts": [ { "kind": "original" }, { "kind": "text", "text": "_web" } ], "case": "lower", "spaces": "_" },
  "output": {
    "format": "webp", "options": { "quality": 82, "lossless": false },
    "target": { "kind": "newFolder", "path": null, "folderName": "batch-output", "keepStructure": true },
    "conflict": "rename", "backup": { "enabled": false, "incremental": false },
    "keepTimestamps": false
  }
}"##;

fn now() -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 10, 13).and_then(|d| d.and_hms_opt(9, 5, 7)).expect("date")
}

fn info(path: &str) -> ItemInfo {
    ItemInfo { path: PathBuf::from(path), root: None, kind: ItemKind::Image, format: "png".into(), bytes: 10, width: 640, height: 480, frames: 1, modified: None, exif: None }
}

fn ctx<'a>(i: &'a ItemInfo, idx: usize, ext: &str) -> NameContext<'a> {
    NameContext { info: i, list_index: idx, folder_index: idx, now: now(), out_ext: ext.into(), out_format: ext.into() }
}

#[test]
fn spec_example_round_trips_and_keeps_unknown_keys() {
    let mut v: serde_json::Value = serde_json::from_str(SPEC_JSON).expect("json");
    v["futureKey"] = json!({"x": 1});
    v["steps"][0]["uiColor"] = json!("cyan");
    v["output"]["target"]["newOption"] = json!(true);
    let r = Recipe::from_value(v).expect("recipe");
    assert_eq!(r.name, "Für das Web");
    assert_eq!(r.steps.len(), 3);
    assert!(!r.steps[2].enabled);
    assert_eq!(r.rename.case, CaseMode::Lower);
    assert_eq!(r.rename.spaces.as_deref(), Some("_"));
    assert_eq!(r.output.format, "webp");
    assert_eq!(r.output.target.folder_name, "batch-output");
    // unbekannte Schlüssel bleiben beim Speichern erhalten
    let back: serde_json::Value = serde_json::from_str(&r.to_json()).expect("json");
    assert_eq!(back["futureKey"]["x"], 1);
    assert_eq!(back["steps"][0]["uiColor"], "cyan");
    assert_eq!(back["output"]["target"]["newOption"], true);
    assert_eq!(back["format"], RECIPE_FORMAT);
    assert_eq!(Recipe::from_json(&r.to_json()).expect("again"), r, "speichern → laden ist verlustfrei");
}

#[test]
fn migration_and_version_checks() {
    // Version 0: ohne format/version, Schritte als Zeichenketten
    let r = Recipe::from_json(r#"{"steps": ["file.copyOnly"], "name": "alt"}"#).expect("v0");
    assert_eq!(r.version, RECIPE_VERSION);
    assert_eq!(r.steps[0].id, "file.copyOnly");
    assert!(r.steps[0].enabled);
    // neuere Version → abgelehnt
    let e = Recipe::from_json(r#"{"format": "firepixel-batch-recipe", "version": 99}"#).expect_err("newer");
    assert!(e.to_string().contains("99"), "{e}");
    // fremdes Format, kein Objekt, BOM
    assert!(Recipe::from_json(r#"{"format": "other"}"#).is_err());
    assert!(Recipe::from_json("[1]").is_err());
    assert!(Recipe::from_json("\u{feff}{}").is_ok());
    // leeres Objekt = Vorgaben (Ziel batch-output, Konflikt rename, Format wie Quelle)
    let r = Recipe::from_json("{}").expect("empty");
    assert_eq!(r.output.target.kind, TargetKind::NewFolder);
    assert_eq!(r.output.conflict, Conflict::Rename);
    assert_eq!(r.output.format, "same");
    assert_eq!(r.rename.parts.len(), 1);
}

#[test]
fn unknown_step_loads_with_warning_and_keeps_params() {
    let reg = StepRegistry::global();
    let r = Recipe::from_json(r#"{"steps": [{"id": "future.magic", "params": {"x": 5}}]}"#).expect("recipe");
    let issues = r.validate(&reg);
    assert_eq!(issues.len(), 1, "{issues:?}");
    assert_eq!(issues[0].severity, Severity::Warning);
    assert_eq!(issues[0].code, "unknownStep");
    assert_eq!(r.steps[0].params["x"], 5);
    let p = fp_batch::Pipeline::new(&r, &reg).expect("pipeline");
    assert_eq!(p.unknown, vec!["future.magic".to_string()]);
}

#[test]
fn validation_catches_bad_params_regex_folder_format_and_copy_only_mix() {
    let reg = StepRegistry::global();
    let mut r = Recipe::default();
    r.add_step("file.copyOnly", json!({"action": "teleport"}));
    let i = r.validate(&reg);
    assert!(i.iter().any(|x| x.severity == Severity::Error && x.param.as_deref() == Some("action")), "{i:?}");
    r.set_param(0, "action", json!("move"));
    assert!(r.validate(&reg).is_empty());
    // copyOnly + anderes Zielformat → Fehler
    r.output.format = "webp".into();
    assert!(r.validate(&reg).iter().any(|x| x.code == "copyOnlyCombined"));
    r.output.format = "nonsense".into();
    assert!(r.validate(&reg).iter().any(|x| x.code == "unknownFormat"));
    r.output.format = "same".into();
    r.rename.mode = RenameMode::Regex;
    r.rename.regex.find = "(".into();
    assert!(r.validate(&reg).iter().any(|x| x.code == "invalidRegex"));
    r.rename.mode = RenameMode::Builder;
    r.output.target.kind = TargetKind::Folder;
    assert!(r.validate(&reg).iter().any(|x| x.code == "missingFolder"));
    // Endkarten gehören nicht in die Schrittliste
    let mut r = Recipe::default();
    r.add_step("output", json!({}));
    assert!(r.validate(&reg).iter().any(|x| x.code == "finalStepInList"));
}

#[test]
fn editing_is_idempotent_and_leaves_no_rests() {
    // §4b.1: Parameter 50× ändern → Schritt einmal, Endwert; 50× hinzufügen/entfernen → keine Reste
    let mut r = Recipe::default();
    let i = r.add_step("file.copyOnly", json!({}));
    for k in 0..50 {
        r.set_param(i, "action", json!(if k % 2 == 0 { "copy" } else { "move" }));
    }
    assert_eq!(r.steps.len(), 1);
    assert_eq!(r.steps[0].params["action"], "move");
    assert!(!r.set_param(i, "action", json!("move")), "gleicher Wert ändert nichts");
    for _ in 0..50 {
        let j = r.add_step("file.copyOnly", json!({"action": "copy"}));
        assert!(r.remove_step(j).is_some());
    }
    assert_eq!(r.steps.len(), 1);
    assert!(r.remove_step(7).is_none());
    // Reihenfolge
    r.add_step("x.a", json!({}));
    r.add_step("x.b", json!({}));
    assert!(r.move_step(2, 0));
    assert_eq!(r.steps.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), ["x.b", "file.copyOnly", "x.a"]);
    assert!(!r.move_step(0, 9));
}

#[test]
fn split_extension_treats_fw_png_as_one() {
    assert_eq!(split_extension("bild.fw.png"), ("bild".into(), "fw.png".into()));
    assert_eq!(split_extension("a.b.jpg"), ("a.b".into(), "jpg".into()));
    assert_eq!(split_extension(".hidden"), (".hidden".into(), String::new()));
    assert_eq!(split_extension("noext"), ("noext".into(), String::new()));
}

#[test]
fn builder_order_follows_spec_4_4() {
    let i = info("C:\\fotos\\Urlaub Mai\\IMG 0042.JPG");
    let spec = RenameSpec {
        parts: vec![
            NamePart::Text { text: "pre ".into() },
            NamePart::Original { take: OriginalTake::Whole, count: 0 },
            NamePart::Counter { start: 1, step: 1, digits: 3, per_folder: false },
            NamePart::Size { which: SizeWhich::Both },
            NamePart::Folder,
            NamePart::Format,
        ],
        replacements: vec![Replacement { find: "img".into(), replace: "Bild".into(), all: true, match_case: false }],
        spaces: Some("_".into()),
        prefix: "Ä ".into(),
        suffix: "-End".into(),
        transliterate: true,
        case: CaseMode::Lower,
        ..RenameSpec::default()
    };
    let n = build_name(&spec, &ctx(&i, 4, "webp")).expect("name");
    // Ersetzen vor Leerzeichen, Präfix/Suffix danach (Präfix-Leerzeichen bleibt), dann Umlaute, dann klein
    assert_eq!(n.file_name, "ae pre_bild_0042005640x480urlaub_maiwebp-end.webp");
    assert!(n.hints.is_empty());
}

#[test]
fn counter_date_and_original_parts() {
    let i = info("D:\\x\\Foto_2024_123.png");
    let mk = |parts: Vec<NamePart>| RenameSpec { parts, ..RenameSpec::default() };
    let s = mk(vec![NamePart::Counter { start: 10, step: 5, digits: 4, per_folder: false }]);
    assert_eq!(build_name(&s, &ctx(&i, 3, "png")).expect("n").file_name, "0025.png");
    let s = mk(vec![NamePart::Date { source: DateSource::Today, format: "YYYY-MM-DD_HH-mm-ss YY".into() }]);
    assert_eq!(build_name(&s, &ctx(&i, 0, "png")).expect("n").stem, "2026-10-13_09-05-07 26");
    let s = mk(vec![NamePart::Original { take: OriginalTake::NoTrailingDigits, count: 0 }]);
    assert_eq!(build_name(&s, &ctx(&i, 0, "png")).expect("n").stem, "Foto_2024");
    let s = mk(vec![NamePart::Original { take: OriginalTake::First, count: 4 }, NamePart::Original { take: OriginalTake::Last, count: 3 }]);
    assert_eq!(build_name(&s, &ctx(&i, 0, "png")).expect("n").stem, "Foto123");
    // EXIF-Datum fehlt → Rückfall + Hinweis
    let s = mk(vec![NamePart::Date { source: DateSource::Exif, format: "YYYY".into() }]);
    let n = build_name(&s, &ctx(&i, 0, "png")).expect("n");
    assert!(n.hints.contains(&NameHint::ExifDateMissing));
    // EXIF-Datum vorhanden
    let mut j = info("D:\\x\\a.jpg");
    j.exif = Some(fp_batch::item::ExifSummary { date_taken: NaiveDate::from_ymd_opt(2019, 7, 1).and_then(|d| d.and_hms_opt(12, 0, 0)), ..Default::default() });
    let s = mk(vec![NamePart::Date { source: DateSource::Exif, format: "YYYY-MM-DD".into() }, NamePart::Text { text: "_".into() }, NamePart::Counter { start: 1, step: 1, digits: 3, per_folder: false }]);
    assert_eq!(build_name(&s, &ctx(&j, 0, "jpg")).expect("n").file_name, "2019-07-01_001.jpg");
}

#[test]
fn regex_mode_with_groups_and_placeholders() {
    let i = info("C:\\in\\IMG_2024-10-13 x.png");
    let mut spec = RenameSpec { mode: RenameMode::Regex, ..RenameSpec::default() };
    spec.regex = RegexRename { find: r"^IMG_(\d{4})-(\d\d)-(\d\d)".into(), replace: "$3.$2.$1_{n:3}_{w}x{h}".into(), ignore_case: false, all: true, whole_name: false };
    assert_eq!(build_name(&spec, &ctx(&i, 6, "png")).expect("n").file_name, "13.10.2024_007_640x480 x.png");
    // Groß/klein ignorieren, nur erster Treffer
    spec.regex = RegexRename { find: "x".into(), replace: "Y".into(), ignore_case: true, all: false, whole_name: false };
    let j = info("C:\\in\\aXbx.png");
    assert_eq!(build_name(&spec, &ctx(&j, 0, "png")).expect("n").stem, "aYbx");
    // `$` in eingesetzten Werten ist kein Gruppenverweis
    spec.regex = RegexRename { find: "^(.*)$".into(), replace: "{name}-$1".into(), ..RegexRename::default() };
    let k = info("C:\\in\\a$1b.png");
    assert_eq!(build_name(&spec, &ctx(&k, 0, "png")).expect("n").stem, "a$1b-a$1b");
    // ungültiger Ausdruck → Fehler
    spec.regex.find = "[".into();
    assert!(build_name(&spec, &ctx(&k, 0, "png")).is_err());
}

#[test]
fn invalid_reserved_empty_dot_and_long_names() {
    let mut hints = Vec::new();
    assert_eq!(naming::sanitize_stem("a<b>:c|d?*", &mut hints), "a_b__c_d__");
    assert!(hints.contains(&NameHint::InvalidChars));
    let mut h = Vec::new();
    assert_eq!(naming::sanitize_stem("name. ", &mut h), "name__");
    let mut h = Vec::new();
    assert_eq!(naming::sanitize_stem("con", &mut h), "con_");
    assert!(h.contains(&NameHint::ReservedName));
    let mut h = Vec::new();
    assert_eq!(naming::sanitize_stem("", &mut h), "_");
    assert!(h.contains(&NameHint::EmptyName));
    let mut h = Vec::new();
    assert_eq!(naming::sanitize_stem(".bashrc", &mut h), "_bashrc");
    assert!(h.contains(&NameHint::LeadingDot));
    // Länge > 255 → gekürzt, Endung bleibt
    let long = "ä".repeat(300);
    let i = info(&format!("C:\\in\\{long}.png"));
    let n = build_name(&RenameSpec::default(), &ctx(&i, 0, "webp")).expect("n");
    assert_eq!(n.file_name.chars().count(), 255);
    assert!(n.file_name.ends_with(".webp"));
    assert!(n.hints.contains(&NameHint::Truncated));
    // Unicode/Emoji bleiben
    let e = info("C:\\in\\Ürlaub 🌴 éé.png");
    assert_eq!(build_name(&RenameSpec::default(), &ctx(&e, 0, "png")).expect("n").file_name, "Ürlaub 🌴 éé.png");
}

#[test]
fn case_modes() {
    use naming::apply_case;
    assert_eq!(apply_case("hello big World", CaseMode::Title), "Hello Big World");
    assert_eq!(apply_case("hello World", CaseMode::Sentence), "Hello world");
    assert_eq!(apply_case("Hello big_World", CaseMode::Kebab), "hello-big-world");
    assert_eq!(apply_case("Hello big-World", CaseMode::Snake), "hello_big_world");
    assert_eq!(apply_case("hello big world", CaseMode::Camel), "helloBigWorld");
    assert_eq!(apply_case("myFileName", CaseMode::Snake), "my_file_name");
    assert_eq!(naming::transliterate("Größe Ärger café Øre"), "Groesse Aerger cafe Ore");
}

fn plan_for(recipe: &Recipe, items: &[ItemInfo]) -> naming::Plan {
    naming::plan(recipe, items, false, &PlanOptions { now: Some(now()) }).expect("plan")
}

#[test]
fn conflicts_inside_the_run_are_resolved_in_list_order() {
    let dir = support::temp("conflicts");
    let mut r = Recipe::default();
    r.output.format = "webp".into();
    r.output.target = TargetSpec { kind: TargetKind::Folder, path: Some(dir.join("out")), ..TargetSpec::default() };
    let items = [info(&dir.join("a.png").to_string_lossy()), info(&dir.join("a.jpg").to_string_lossy()), info(&dir.join("A.gif").to_string_lossy())];
    let p = plan_for(&r, &items);
    let names: Vec<String> = p.entries.iter().map(|e| e.target.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()).collect();
    if cfg!(windows) {
        assert_eq!(names, ["a.webp", "a (2).webp", "A (3).webp"]);
    } else {
        assert_eq!(names[..2], ["a.webp".to_string(), "a (2).webp".to_string()]);
    }
    assert_eq!(p.entries[1].conflict_with, Some(0));
    assert!(p.entries[1].hints.contains(&NameHint::Conflict));
    // deterministisch: gleicher Plan bei erneutem Aufruf
    assert_eq!(plan_for(&r, &items), p);
    // Regel „überspringen“: der zweite wird übersprungen
    r.output.conflict = Conflict::Skip;
    let p = plan_for(&r, &items);
    assert_eq!(p.entries[1].action, PlanAction::Skip);
    // „überschreiben“ überschreibt nie die Ausgabe eines früheren Elements im selben Lauf
    r.output.conflict = Conflict::Overwrite;
    let p = plan_for(&r, &items);
    assert_eq!(p.entries[1].action, PlanAction::Write);
    assert!(p.entries[1].target.to_string_lossy().ends_with("a (2).webp"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn existing_targets_follow_the_rule_and_same_place_needs_backup() {
    let dir = support::temp("existing");
    support::write_png(&dir.join("x.png"), 4, 4);
    std::fs::create_dir_all(dir.join("out")).expect("out");
    std::fs::write(dir.join("out").join("x.png"), b"old").expect("old");
    std::fs::write(dir.join("out").join("x (2).png"), b"old").expect("old");
    let items = [fp_batch::probe(&dir.join("x.png")).expect("probe")];
    let mut r = Recipe::default();
    r.output.target = TargetSpec { kind: TargetKind::Folder, path: Some(dir.join("out")), ..TargetSpec::default() };
    let p = plan_for(&r, &items);
    assert!(p.entries[0].target.ends_with("x (3).png"), "{:?}", p.entries[0].target);
    r.output.conflict = Conflict::Overwrite;
    assert_eq!(plan_for(&r, &items).entries[0].action, PlanAction::Overwrite);
    r.output.conflict = Conflict::Ask;
    let p = plan_for(&r, &items);
    assert_eq!(p.entries[0].action, PlanAction::Ask);
    assert!(p.entries[0].alt_target.as_ref().is_some_and(|a| a.ends_with("x (3).png")));
    // gleicher Ort: rename → name (2); overwrite → Original ersetzen mit erzwungener Sicherung
    r.output.target = TargetSpec { kind: TargetKind::SameAsSource, ..TargetSpec::default() };
    r.output.conflict = Conflict::Rename;
    assert!(plan_for(&r, &items).entries[0].target.ends_with("x (2).png"));
    r.output.conflict = Conflict::Overwrite;
    let p = plan_for(&r, &items);
    assert_eq!(p.entries[0].action, PlanAction::ReplaceOriginal);
    assert!(p.entries[0].hints.contains(&NameHint::BackupForced));
    r.output.backup.enabled = true;
    assert!(!plan_for(&r, &items).entries[0].hints.contains(&NameHint::BackupForced));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn new_folder_next_to_sources_with_numbering_and_structure() {
    let dir = support::temp("newfolder");
    support::write_png(&dir.join("sub").join("deep").join("a.png"), 2, 2);
    support::write_png(&dir.join("b.png"), 2, 2);
    let c = fp_batch::collect(std::slice::from_ref(&dir), true, false, &[]);
    assert_eq!(c.items.len(), 2);
    let r = Recipe::default();
    let p = plan_for(&r, &c.items);
    let out = dir.join("batch-output");
    assert!(p.entries.iter().any(|e| e.target == out.join("b.png")), "{:?}", p.entries);
    assert!(p.entries.iter().any(|e| e.target == out.join("sub").join("deep").join("a.png")));
    // vorhandener, nicht leerer Ordner → batch-output-2; leerer Ordner wird wiederverwendet
    std::fs::create_dir_all(&out).expect("out");
    assert!(plan_for(&r, &c.items).entries.iter().all(|e| e.target.starts_with(&out)), "leer → wiederverwenden");
    std::fs::write(out.join("keep.txt"), b"x").expect("x");
    assert!(plan_for(&r, &c.items).entries.iter().all(|e| e.target.starts_with(dir.join("batch-output-2"))));
    // Struktur nicht beibehalten
    let mut flat = Recipe::default();
    flat.output.target.keep_structure = false;
    assert!(plan_for(&flat, &c.items).entries.iter().any(|e| e.target == dir.join("batch-output-2").join("a.png")));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn same_names_from_two_folders_and_extension_options() {
    let mut r = Recipe::default();
    r.output.format = "jpeg".into();
    r.output.target = TargetSpec { kind: TargetKind::Folder, path: Some(PathBuf::from("Z:\\nowhere\\out")), keep_structure: false, ..TargetSpec::default() };
    let items = [info("Z:\\nowhere\\a\\pic.png"), info("Z:\\nowhere\\b\\pic.png")];
    let p = plan_for(&r, &items);
    assert!(p.entries[0].target.ends_with("pic.jpg"));
    assert!(p.entries[1].target.ends_with("pic (2).jpg"));
    r.rename.jpeg_as_jpg = false;
    r.rename.extension = ExtCase::Upper;
    assert!(plan_for(&r, &items).entries[0].target.ends_with("pic.JPEG"));
    // .fw.png als Ziel
    r.output.format = "fwpng".into();
    r.rename.extension = ExtCase::Lower;
    assert!(plan_for(&r, &items).entries[0].target.ends_with("pic.fw.png"));
    // „wie Quelle“ bei nicht schreibbarem Format → PNG mit Hinweis
    r.output.format = "same".into();
    let mut heic = info("Z:\\nowhere\\a\\foto.heic");
    heic.format = "heif".into();
    let p = plan_for(&r, &[heic]);
    assert!(p.entries[0].target.ends_with("foto.png"));
    assert!(p.entries[0].hints.contains(&NameHint::FormatFallback));
    // reine Dateischritte: Originalendung bleibt
    let p = naming::plan(&r, &[info("Z:\\nowhere\\doc.TXT")], true, &PlanOptions::default()).expect("plan");
    assert!(p.entries[0].target.ends_with("doc.txt"));
    assert!(p.file_only);
}

#[test]
fn long_target_path_is_flagged() {
    let mut r = Recipe::default();
    let deep = format!("C:\\{}", "verzeichnis\\".repeat(25));
    r.output.target = TargetSpec { kind: TargetKind::Folder, path: Some(PathBuf::from(&deep)), ..TargetSpec::default() };
    let p = plan_for(&r, &[info("C:\\in\\a.png")]);
    assert!(p.entries[0].hints.contains(&NameHint::LongPath));
    assert!(fp_batch::output::long_path(&p.entries[0].target).to_string_lossy().starts_with(if cfg!(windows) { r"\\?\" } else { "C:" }));
}

#[test]
fn plan_of_10000_files_is_fast() {
    let mut r = Recipe::default();
    r.output.format = "webp".into();
    r.output.target = TargetSpec { kind: TargetKind::Folder, path: Some(PathBuf::from("Z:\\nowhere\\out")), ..TargetSpec::default() };
    r.rename.parts.push(NamePart::Counter { start: 1, step: 1, digits: 5, per_folder: false });
    let items: Vec<ItemInfo> = (0..10_000).map(|k| info(&format!("Z:\\nowhere\\in\\bild{}.png", k % 500))).collect();
    let t = std::time::Instant::now();
    let p = plan_for(&r, &items);
    let ms = t.elapsed().as_millis();
    assert_eq!(p.entries.len(), 10_000);
    let unique: std::collections::HashSet<String> = p.entries.iter().map(|e| e.target.to_string_lossy().to_lowercase()).collect();
    assert_eq!(unique.len(), 10_000, "alle Ziele verschieden");
    assert!(ms < 2000, "Plan dauerte {ms} ms");
}

#[test]
fn catalog_lists_core_steps_in_both_languages() {
    let reg = StepRegistry::global();
    for id in ["file.copyOnly", "rename", "output"] {
        assert!(reg.contains(id), "{id}");
    }
    let de = fp_i18n::with_language("de", || reg.catalog_json());
    let en = fp_i18n::with_language("en", || reg.catalog_json());
    let name = |v: &serde_json::Value, id: &str| v["steps"].as_array().and_then(|a| a.iter().find(|s| s["id"] == id)).map(|s| s["name"].as_str().unwrap_or_default().to_string()).unwrap_or_default();
    assert_eq!(name(&de, "file.copyOnly"), "Nur kopieren/verschieben");
    assert_eq!(name(&en, "file.copyOnly"), "Copy/Move Only");
    assert_eq!(name(&en, "output"), "Save As");
    let _ = Path::new("");
}

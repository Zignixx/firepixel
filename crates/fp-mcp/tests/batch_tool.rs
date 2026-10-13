//! MCP-Werkzeug `batch` und Ressource `firepixel://docs/batch.md` (BATCH-9, SPEC-BATCH §5.8):
//! Vorlagen, Schritte, Prüfen, Plan, Vorschau, Lauf (blockierend und im Hintergrund mit Fortschritt
//! und Abbrechen), Sprache der Antworten.

use fp_mcp::tools::{call, Block, ToolEnv};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fp-mcp-batch-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("dir");
    d
}

/// Logo-PNG w × h (weißer Grund, rotes Quadrat in der Mitte).
fn write_logo(p: &Path, w: u32, h: u32) {
    let mut v = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let inner = x > w / 4 && x < 3 * w / 4 && y > h / 4 && y < 3 * h / 4;
            v.extend_from_slice(if inner { &[200, 20, 30, 255] } else { &[255, 255, 255, 255] });
        }
    }
    let item = fp_batch::item::still_from_rgba(w, h, &v).expect("item");
    let (bytes, _) = fp_batch::output::encode(&item, fp_batch::OutputFormat::Png, &Default::default()).expect("png");
    std::fs::write(p, bytes).expect("write");
}

fn sources(dir: &Path, n: usize) -> Vec<String> {
    (0..n)
        .map(|k| {
            let p = dir.join(format!("logo{k}.png"));
            write_logo(&p, 40 + k as u32, 30);
            p.to_string_lossy().to_string()
        })
        .collect()
}

fn env() -> ToolEnv {
    ToolEnv { language: "en".into(), ..ToolEnv::default() }
}

fn ok(v: &fp_mcp::tools::ToolOutput) -> Value {
    assert!(!v.is_error, "{}", v.text());
    v.structured.clone().unwrap_or_default()
}

#[test]
fn tool_is_listed_and_translated() {
    for lang in ["de", "en"] {
        let defs = fp_i18n::with_language(lang, fp_mcp::tools::tool_defs);
        let d = defs.iter().find(|d| d.name == "batch").expect("batch");
        assert!(!d.title.is_empty() && !d.title.starts_with("mcp-tool-") && d.description.len() > 60, "{lang}");
        assert!(d.schema["properties"]["recipe"]["description"].as_str().is_some_and(|s| !s.is_empty()), "{lang}");
        let res = fp_i18n::with_language(lang, fp_mcp::resources::resources);
        assert!(res.iter().any(|r| r.uri == "firepixel://docs/batch.md" && !r.title.starts_with("mcp-res-")));
    }
    assert_eq!(fp_i18n::with_language("de", fp_mcp::tools::tool_defs).iter().find(|d| d.name == "batch").map(|d| d.title.clone()).as_deref(), Some("Batch-Verarbeitung"));
    // Ressource: Vorlagen und Schritte, in der Antwortsprache
    let reg = fp_engine::Session::default();
    let md = fp_i18n::with_language("en", || fp_mcp::resources::read("firepixel://docs/batch.md", reg.registry())).expect("md");
    assert!(md.starts_with("# Batch processing") && md.contains("`logos-to-svg`") && md.contains("`vector.trace`"), "{}", &md[..300.min(md.len())]);
    let md = fp_i18n::with_language("de", || fp_mcp::resources::read("firepixel://docs/batch.md", reg.registry())).expect("md");
    assert!(md.contains("Logos nachzeichnen → SVG"));
}

#[test]
fn templates_steps_validate_plan() {
    let mut s = fp_engine::Session::default();
    let e = env();
    let v = ok(&call(&mut s, &e, "batch", &json!({"action": "templates"})));
    let list = v["templates"].as_array().expect("templates");
    assert!(list.iter().any(|t| t["id"] == "logos-to-svg" && t["title"] == "Trace logos → SVG"), "{v}");
    let v = ok(&call(&mut s, &e, "batch", &json!({"action": "templates", "language": "de"})));
    assert!(v["templates"].as_array().is_some_and(|a| a.iter().any(|t| t["title"] == "Logos nachzeichnen → SVG")));
    let v = ok(&call(&mut s, &e, "batch", &json!({"action": "steps"})));
    assert!(v["steps"].as_array().is_some_and(|a| a.iter().any(|st| st["id"] == "vector.trace")));
    // Prüfen mit Vorlagennamen und mit fehlerhaftem Rezept
    let v = ok(&call(&mut s, &e, "batch", &json!({"action": "validate", "recipe": "logos-to-svg"})));
    assert_eq!(v["ok"], true, "{v}");
    let v = ok(&call(&mut s, &e, "batch", &json!({"action": "validate", "recipe": {"steps": [{"id": "vector.trace", "params": {"preset": "comic"}}]}})));
    assert_eq!(v["ok"], false, "{v}");
    // Plan schreibt nichts
    let dir = temp("plan");
    let files = sources(&dir, 2);
    let v = ok(&call(&mut s, &e, "batch", &json!({"action": "plan", "recipe": "logos-to-svg", "files": files})));
    assert!(v.to_string().contains("logo0.svg"), "{v}");
    assert!(!dir.join("batch-output").exists());
    // Fehler: unbekannte Aktion, fehlende jobId, unbekannter Auftrag, Vorlage unbekannt
    for (a, needle) in [(json!({"action": "explode"}), "Unknown action"), (json!({"action": "status"}), "jobId"), (json!({"action": "status", "jobId": 99999}), "Unknown job")] {
        let r = call(&mut s, &e, "batch", &a);
        assert!(r.is_error && r.text().contains(needle), "{}", r.text());
    }
    let r = call(&mut s, &e, "batch", &json!({"action": "run", "recipe": "no-such-template", "files": files}));
    assert!(r.is_error);
    assert!(s.documents().is_empty(), "nie ein Dokument der Sitzung");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn preview_returns_before_and_after_images() {
    let mut s = fp_engine::Session::default();
    let dir = temp("preview");
    let files = sources(&dir, 1);
    let r = call(&mut s, &env(), "batch", &json!({"action": "preview", "recipe": "logos-to-svg", "file": files[0], "maxSize": 64}));
    let v = ok(&r);
    assert_eq!(v["format"], "svg");
    assert_eq!(v["after"]["width"], 40);
    let images: Vec<&Vec<u8>> = r.blocks.iter().filter_map(|b| if let Block::Image(p) = b { Some(p) } else { None }).collect();
    assert_eq!(images.len(), 2);
    assert!(images.iter().all(|p| p.starts_with(&[0x89, b'P', b'N', b'G'])));
    let r = call(&mut s, &env(), "batch", &json!({"action": "preview", "recipe": "logos-to-svg"}));
    assert!(r.is_error);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn run_blocking_and_in_background_with_progress() {
    let mut s = fp_engine::Session::default();
    let e = env();
    let dir = temp("run");
    let files = sources(&dir, 3);
    // blockierend (Vorgabe-Aktion run)
    let out = dir.join("svg");
    let v = ok(&call(&mut s, &e, "batch", &json!({"recipe": "logos-to-svg", "files": files, "out": out.to_string_lossy()})));
    assert_eq!(v["ok"], true, "{v}");
    for k in 0..3 {
        assert!(std::fs::read_to_string(out.join(format!("logo{k}.svg"))).expect("svg").contains("<path"));
    }
    // im Hintergrund: jobId, status mit Warten → fertig mit Bericht
    let out2 = dir.join("web");
    let v = ok(&call(&mut s, &e, "batch", &json!({"recipe": "web-webp", "files": files, "out": out2.to_string_lossy(), "async": true})));
    let id = v["jobId"].as_u64().expect("jobId");
    assert_eq!(v["state"], "running");
    let v = ok(&call(&mut s, &e, "batch", &json!({"action": "status", "jobId": id, "wait_ms": 60000})));
    assert_eq!(v["state"], "finished", "{v}");
    assert_eq!((v["done"].clone(), v["total"].clone(), v["percent"].clone()), (json!(3), json!(3), json!(100.0)));
    assert_eq!(v["report"]["ok"], true);
    assert!(out2.join("logo2.webp").exists());
    // status ist wiederholbar (gleiches Ergebnis, nichts doppelt gezählt)
    for _ in 0..5 {
        let w = ok(&call(&mut s, &e, "batch", &json!({"action": "status", "jobId": id})));
        assert_eq!((w["done"].clone(), w["state"].clone()), (json!(3), json!("finished")));
    }
    // Trockenlauf im Hintergrund schreibt nichts
    let out3 = dir.join("dry");
    let v = ok(&call(&mut s, &e, "batch", &json!({"recipe": "web-webp", "files": files, "out": out3.to_string_lossy(), "async": true, "dryRun": true})));
    let id = v["jobId"].as_u64().expect("jobId");
    let v = ok(&call(&mut s, &e, "batch", &json!({"action": "status", "jobId": id, "wait_ms": 60000})));
    assert_eq!(v["state"], "finished", "{v}");
    assert!(!out3.join("logo0.webp").exists());
    // Abbrechen: Auftrag endet, keine Temporärdateien
    let many: Vec<String> = (0..40)
        .map(|k| {
            let p = dir.join("many").join(format!("b{k}.png"));
            std::fs::create_dir_all(p.parent().expect("dir")).expect("dir");
            write_logo(&p, 300, 200);
            p.to_string_lossy().to_string()
        })
        .collect();
    let out4 = dir.join("cancel");
    let v = ok(&call(&mut s, &e, "batch", &json!({"recipe": "logos-to-svg", "files": many, "out": out4.to_string_lossy(), "async": true, "threads": 1})));
    let id = v["jobId"].as_u64().expect("jobId");
    let v = ok(&call(&mut s, &e, "batch", &json!({"action": "cancel", "jobId": id, "wait_ms": 60000})));
    assert!(v["state"] == "cancelled" || v["state"] == "finished", "{v}");
    if v["state"] == "cancelled" {
        assert!(v["processed"].as_u64().unwrap_or(0) <= 40);
    }
    let tmp = std::fs::read_dir(&out4).map(|rd| rd.flatten().filter(|e| e.file_name().to_string_lossy().ends_with(".fptmp")).count()).unwrap_or(0);
    assert_eq!(tmp, 0, "keine Temporärdateien");
    let _ = std::fs::remove_dir_all(&dir);
}

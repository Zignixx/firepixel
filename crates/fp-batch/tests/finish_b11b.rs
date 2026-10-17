//! BATCH-11b: Felder der Endkarte „Speichern als“ je Format, GIF-Optionen ohne JSON,
//! „Zusätzlich als FW-PNG“ (SVG **und** FW-PNG in einem Lauf, Befund BATCH-9), HTML-Bericht,
//! Warnung „Zielgröße nicht erreichbar“, Vorlage „Video → kleines GIF“. Logikfälle nach §4b
//! (Wiederholung, Konflikte, Rückfall bei fehlender Datei).

mod batch4_common;
mod support;

use batch4_common::*;
use fp_batch::executor::{self, RunOptions};
use fp_batch::output::{OutputFormat, encode};
use fp_batch::recipe::{Conflict, Recipe, TargetKind, TargetSpec};
use fp_batch::steps::{files, gifopt};
use fp_batch::{ItemStatus, ParamDef, Severity, StepRegistry, templates};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

fn defs(id: &str) -> Vec<ParamDef> {
    StepRegistry::global().get(id).expect("Schritt").params()
}

/// Sichtbare Felder (Name, Vorgabe) des Schemas für diese Werte.
fn visible(id: &str, params: Value) -> Vec<(String, Value)> {
    let d = defs(id);
    d.iter()
        .filter(|p| p.is_visible(&d, &params))
        .map(|p| (p.name.clone(), p.default.clone()))
        .collect()
}

fn names(v: &[(String, Value)]) -> Vec<&str> {
    v.iter().map(|(n, _)| n.as_str()).collect()
}

#[test]
fn speichern_als_zeigt_nur_felder_des_formats() {
    let jpeg = visible(files::OUTPUT, json!({"format": "jpeg"}));
    assert_eq!(
        names(&jpeg),
        [
            "format",
            "quality",
            "progressive",
            "subsampling",
            "background",
            "alsoFwpng"
        ]
    );
    assert_eq!(jpeg[1].1, json!(85));
    // Qualität je Format mit der Vorgabe des Kodierers
    let q = |f: &str| {
        visible(files::OUTPUT, json!({"format": f}))
            .into_iter()
            .find(|(n, _)| n == "quality")
            .map(|(_, v)| v)
    };
    assert_eq!(
        (q("jpeg"), q("webp"), q("avif"), q("png"), q("gif")),
        (
            Some(json!(85)),
            Some(json!(82)),
            Some(json!(60)),
            None,
            None
        )
    );
    // PNG: Palette nur bei 8 Bit (Kette format → bitDepth → palette), Kompression ohne TIFF-Werte
    assert!(!names(&visible(files::OUTPUT, json!({"format": "png"}))).contains(&"palette"));
    assert!(
        names(&visible(
            files::OUTPUT,
            json!({"format": "png", "bitDepth": "8"})
        ))
        .contains(&"palette")
    );
    assert!(
        !names(&visible(
            files::OUTPUT,
            json!({"format": "jpeg", "bitDepth": "8"})
        ))
        .contains(&"palette")
    );
    let comp = |f: &str| {
        let d = defs(files::OUTPUT);
        let p = json!({"format": f});
        d.into_iter()
            .filter(|x| x.name == "compression" && x.is_visible(&defs(files::OUTPUT), &p))
            .map(|x| match x.ty {
                fp_batch::ParamType::Enum { values } => values,
                _ => Vec::new(),
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(comp("png"), vec![vec!["default", "fast", "max"]]);
    assert_eq!(
        comp("tiff"),
        vec![vec!["default", "none", "lzw", "deflate", "packbits"]]
    );
    assert_eq!(comp("tga"), vec![vec!["default", "none"]]);
    assert!(comp("jpeg").is_empty());
    // GIF: keine doppelten Felder im Hauptschema, die Optionen stehen in `output.gif`
    assert_eq!(
        names(&visible(files::OUTPUT, json!({"format": "gif"}))),
        ["format", "alsoFwpng"]
    );
    // FW-PNG: kein „Zusätzlich als FW-PNG“
    assert_eq!(
        names(&visible(files::OUTPUT, json!({"format": "fwpng"}))),
        ["format"]
    );
    // je Format höchstens ein sichtbares Feld je Name
    for f in OutputFormat::ALL {
        let v = visible(files::OUTPUT, json!({"format": f.name(), "bitDepth": "8"}));
        let mut n = names(&v);
        n.sort();
        let len = n.len();
        n.dedup();
        assert_eq!(n.len(), len, "{}: doppelte Felder", f.name());
    }
}

#[test]
fn gif_optionen_sind_eigene_felder_mit_automatik() {
    let d = defs(gifopt::OUTPUT_GIF);
    let get = |n: &str| {
        d.iter()
            .find(|p| p.name == n)
            .unwrap_or_else(|| panic!("{n} fehlt"))
    };
    for n in [
        "preset",
        "quality",
        "maxKB",
        "splitFrames",
        "colors",
        "lossy",
        "dither",
        "palette",
        "disposal",
        "transparency",
    ] {
        let p = get(n);
        assert!(!matches!(p.ty, fp_batch::ParamType::Json), "{n} ist JSON");
    }
    assert!(get("preset").beginner && get("quality").beginner && get("maxKB").beginner);
    // abhängig von Qualität und Inhalt → „Automatisch“ (kein fester Wert, der die Automatik abschaltet)
    for n in ["splitFrames", "colors", "lossy"] {
        assert!(get(n).default.is_null(), "{n}");
    }
    assert_eq!(get("maxKB").unit.as_deref(), Some("KB"));
    // Hintergrund nur ohne GIF-Transparenz
    let vis = |p: Value| {
        d.iter()
            .filter(|x| x.is_visible(&d, &p))
            .map(|x| x.name.clone())
            .collect::<Vec<_>>()
    };
    assert!(!vis(json!({})).contains(&"background".to_string()));
    assert!(vis(json!({"transparency": false})).contains(&"background".to_string()));
    // Texte für alle Felder (de + en)
    for lang in ["de", "en"] {
        fp_i18n::with_language(lang, || {
            for p in &d {
                let k = fp_batch::step::param_key(gifopt::OUTPUT_GIF, &p.name);
                assert!(fp_i18n::has(&k), "{lang}: {k}");
            }
            assert!(fp_i18n::has("batch-param-output-alsoFwpng"), "{lang}");
        });
    }
}

/// Animiertes GIF w×h mit `n` Bildern Rauschen (schwer zu komprimieren).
fn noisy_gif(path: &Path, w: u32, h: u32, n: u32) {
    let frames: Vec<_> = (0..n)
        .map(|k| {
            (
                image(w, h, |x, y| {
                    let v =
                        (x.wrapping_mul(2654435761) ^ y.wrapping_mul(40503) ^ k.wrapping_mul(97))
                            .wrapping_mul(2246822519);
                    [(v >> 8) as u8, (v >> 16) as u8, (v >> 24) as u8, 255]
                }),
                80,
            )
        })
        .collect();
    let (bytes, _) = encode(
        &animation(frames),
        OutputFormat::Gif,
        &serde_json::Map::new(),
    )
    .expect("gif");
    std::fs::write(path, bytes).expect("write");
}

fn run(recipe: &Recipe, src: &[PathBuf], out: &Path) -> fp_batch::Report {
    let mut r = recipe.clone();
    r.output.target = TargetSpec {
        kind: TargetKind::Folder,
        path: Some(out.to_path_buf()),
        ..TargetSpec::default()
    };
    let items = fp_batch::collect(src, false, false, &[]).items;
    executor::run(
        &r,
        &items,
        &RunOptions {
            threads: Some(2),
            ..RunOptions::default()
        },
    )
    .expect("run")
}

#[test]
fn zielgroesse_nicht_erreichbar_meldet_eine_warnung() {
    let dir = support::temp("b11b-gif-target");
    let src = dir.join("rauschen.gif");
    noisy_gif(&src, 220, 220, 8);
    let mut r = Recipe::default();
    r.output.format = "gif".into();
    r.output.options.insert("maxKB".into(), json!(1));
    let rep = run(&r, std::slice::from_ref(&src), &dir.join("out"));
    let it = &rep.items[0];
    assert_eq!(it.status, ItemStatus::Done, "{:?}", it.error);
    assert!(it.bytes_out > 1024);
    assert!(
        it.warnings
            .iter()
            .any(|w| w.contains("1")
                && (w.contains("nicht erreichbar") || w.contains("not reachable"))),
        "{:?}",
        it.warnings
    );
    // ohne Zielgröße keine Warnung
    r.output.options.remove("maxKB");
    let rep = run(&r, &[src], &dir.join("out2"));
    assert!(
        rep.items[0]
            .warnings
            .iter()
            .all(|w| !w.contains("nicht erreichbar") && !w.contains("not reachable"))
    );
}

fn logo_png(path: &Path) {
    let img = image(48, 48, |x, y| {
        let (dx, dy) = (x as i32 - 20, y as i32 - 20);
        if dx * dx + dy * dy <= 12 * 12 {
            [220, 30, 40, 255]
        } else {
            [255, 255, 255, 255]
        }
    });
    let (bytes, _) = encode(&still(img), OutputFormat::Png, &serde_json::Map::new()).expect("png");
    std::fs::write(path, bytes).expect("write");
}

#[test]
fn svg_und_fwpng_in_einem_lauf() {
    let dir = support::temp("b11b-also-fwpng");
    let src: Vec<PathBuf> = (0..2)
        .map(|k| {
            let p = dir.join(format!("logo {k}.png"));
            logo_png(&p);
            p
        })
        .collect();
    let mut r = templates::recipe("logos-to-svg").expect("Vorlage");
    r.output
        .options
        .insert(files::ALSO_FWPNG.into(), json!(true));
    let out = dir.join("out");
    let rep = run(&r, &src, &out);
    assert_eq!(
        rep.totals.done,
        2,
        "{:?}",
        rep.items.iter().map(|i| &i.error).collect::<Vec<_>>()
    );
    for (k, it) in rep.items.iter().enumerate() {
        let svg = it.target.clone().expect("Ziel");
        assert!(svg.to_string_lossy().ends_with(".svg"));
        assert!(
            std::fs::read_to_string(&svg)
                .expect("svg")
                .contains("<path")
        );
        assert_eq!(it.extra_targets.len(), 1, "{k}");
        let fw = &it.extra_targets[0];
        assert_eq!(
            fw.file_name().and_then(|n| n.to_str()),
            Some(format!("logo {}.fw.png", it.index).as_str())
        );
        // die FW-PNG ist ein bearbeitbares Dokument mit Pfaden
        let item =
            fp_batch::item::load(&fp_batch::probe(fw).expect("probe"), false).expect("laden");
        assert_eq!(item.kind(), fp_batch::ItemKind::Document);
    }
    // im JSON-Bericht steht die Zusatzdatei
    let json = std::fs::read_to_string(out.join("batch-report.json")).expect("json");
    assert!(json.contains("extraTargets") && json.contains(".fw.png"));
    // zweiter Lauf (Regel „umbenennen“): neue Namen, nichts überschrieben, keine Temporärdateien
    let rep2 = run(&r, &src, &out);
    assert_eq!(rep2.totals.done, 2);
    for it in &rep2.items {
        let fw = it.extra_targets[0]
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        assert!(fw.ends_with(" (2).fw.png"), "{fw}");
    }
    assert!(!support::has_temp_files(&out));
    // Regel „überspringen“: Hauptdatei übersprungen bzw. Zusatz mit Hinweis, nie überschrieben
    let sums: Vec<u64> = support::files_under(&out)
        .iter()
        .filter(|p| p.to_string_lossy().ends_with(".fw.png"))
        .map(|p| support::checksum(p))
        .collect();
    r.output.conflict = Conflict::Skip;
    let rep3 = run(&r, &src, &out);
    assert_eq!(rep3.totals.done + rep3.totals.skipped, 2);
    let sums2: Vec<u64> = support::files_under(&out)
        .iter()
        .filter(|p| p.to_string_lossy().ends_with(".fw.png"))
        .map(|p| support::checksum(p))
        .collect();
    assert_eq!(sums, sums2);
    // ohne Haken keine Zusatzdatei; bei Ausgabe FW-PNG keine doppelte
    let mut plain = templates::recipe("logos-to-svg").expect("Vorlage");
    let rep4 = run(&plain, &src[..1], &dir.join("plain"));
    assert!(rep4.items[0].extra_targets.is_empty());
    plain.output.format = "fwpng".into();
    plain
        .output
        .options
        .insert(files::ALSO_FWPNG.into(), json!(true));
    let rep5 = run(&plain, &src[..1], &dir.join("fw"));
    assert!(rep5.items[0].extra_targets.is_empty());
    assert_eq!(
        support::files_under(&dir.join("fw"))
            .iter()
            .filter(|p| p.to_string_lossy().ends_with(".fw.png"))
            .count(),
        1
    );
}

#[test]
fn zusatzname_folgt_dem_ziel() {
    assert_eq!(
        executor::extra_fwpng_target(Path::new("C:\\o\\a.svg")),
        PathBuf::from("C:\\o\\a.fw.png")
    );
    assert_eq!(
        executor::extra_fwpng_target(Path::new("C:\\o\\a.b.webp")),
        PathBuf::from("C:\\o\\a.b.fw.png")
    );
    assert_eq!(
        executor::extra_fwpng_target(Path::new("C:\\o\\x.FW.PNG")),
        PathBuf::from("C:\\o\\x.fw.png")
    );
    assert_eq!(
        executor::extra_fwpng_target(Path::new("C:\\o\\ohne")),
        PathBuf::from("C:\\o\\ohne.fw.png")
    );
}

#[test]
fn html_bericht_mit_vorschaubildern_groessen_und_fehlern() {
    let dir = support::temp("b11b-html");
    let a = dir.join("foto a.png");
    let b = dir.join("foto & b.png");
    support::write_png(&a, 120, 80);
    support::write_png(&b, 60, 90);
    let bad = dir.join("kaputt.png");
    std::fs::write(&bad, b"\x89PNG\r\n\x1a\nkaputt").expect("write");
    let mut r = Recipe {
        name: "Web <klein>".into(),
        ..Default::default()
    };
    r.steps.push(fp_batch::StepConfig::new(
        "resize",
        json!({"mode": "longEdge", "size": 40}),
    ));
    r.output.format = "png".into();
    let out = dir.join("out");
    let rep = fp_i18n::with_language("de", || run(&r, &[a.clone(), b.clone(), bad.clone()], &out));
    let html_path = out.join("batch-report.html");
    assert_eq!(rep.report_html.as_deref(), Some(html_path.as_path()));
    let html = std::fs::read_to_string(&html_path).expect("html");
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("lang=\"de\"") && html.contains("Batch-Bericht"));
    // Name des Rezepts und der Dateien sicher maskiert
    assert!(
        html.contains("Web &lt;klein&gt;")
            && html.contains("foto &amp; b.png")
            && !html.contains("<klein>")
    );
    // Vorschaubilder vorher und nachher (2 fertige Dateien → mindestens 4 Bilder)
    assert!(
        html.matches("data:image/png;base64,").count() >= 4,
        "{}",
        html.matches("data:image/png;base64,").count()
    );
    // Maße vorher/nachher und Größen
    assert!(html.contains("120×80") && html.contains("40×27") && html.contains(" → "));
    // Fehlerliste mit der kaputten Datei
    let failed = rep
        .items
        .iter()
        .filter(|i| i.status == ItemStatus::Failed)
        .count();
    assert!(
        failed >= 1,
        "{:?}",
        rep.items.iter().map(|i| i.status).collect::<Vec<_>>()
    );
    assert!(html.contains("class=\"errors\"") && html.contains("kaputt.png"));
    // relativer Verweis auf die Ausgabe (Leerzeichen kodiert)
    assert!(html.contains("href=\"foto%20a.png\""), "Verweis fehlt");
    // JSON nennt den HTML-Bericht
    let json: Value = serde_json::from_str(
        &std::fs::read_to_string(out.join("batch-report.json")).expect("json"),
    )
    .expect("parse");
    assert!(
        json["reportHtml"]
            .as_str()
            .is_some_and(|p| p.ends_with("batch-report.html"))
    );
    // Wiederholung: Bericht wird ersetzt, nicht angehängt; ausschaltbar
    let rep2 = fp_i18n::with_language("en", || run(&r, std::slice::from_ref(&a), &out));
    let html2 = std::fs::read_to_string(&html_path).expect("html");
    assert!(
        html2.contains("Batch report")
            && !html2.contains("Batch-Bericht")
            && html2.matches("<article").count() == 1
    );
    assert!(rep2.report_html.is_some());
    r.output.report_html = false;
    let out3 = dir.join("out3");
    let rep3 = run(&r, &[a], &out3);
    assert!(
        rep3.report_html.is_none()
            && !out3.join("batch-report.html").exists()
            && out3.join("batch-report.json").exists()
    );
}

#[test]
fn vorlage_video_zu_kleinem_gif() {
    let tpl = templates::find("video-to-gif").expect("Vorlage");
    let ids: Vec<&str> = tpl.recipe.steps.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, ["video.import", "resize"]);
    assert_eq!(tpl.recipe.steps[0].params["fps"], json!(12));
    assert_eq!(tpl.recipe.steps[1].params["size"], json!(480));
    assert_eq!(tpl.recipe.output.format, "gif");
    assert_eq!(
        tpl.recipe.output.options.get("preset"),
        Some(&json!("balanced"))
    );
    let issues = tpl.recipe.validate(&StepRegistry::global());
    assert!(
        issues.iter().all(|i| i.severity != Severity::Error),
        "{issues:?}"
    );
    for lang in ["de", "en"] {
        let t = fp_i18n::with_language(lang, || templates::find("video-to-gif").expect("Vorlage"));
        assert!(
            !t.title.is_empty() && t.title != "Video to small GIF" || lang == "en",
            "{lang}: {}",
            t.title
        );
        assert!(t.description.contains("480"));
    }
    // auch über den deutschen Titel auffindbar
    assert_eq!(
        fp_i18n::with_language("de", || templates::find("Video → kleines GIF")).map(|t| t.id),
        Some("video-to-gif")
    );
}

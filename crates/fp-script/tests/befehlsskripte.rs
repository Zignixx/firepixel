//! Laufzeittest mit den ausgelieferten Befehlsskripten des Originals
//! (`korpus\Configuration\Commands\**\*.jsf`, Korpus = Installationskopie, `FP_CORPUS_DIR`). Die Skripte werden nur gelesen und ausgeführt,
//! nicht eingebettet. Ausgangslage: Dokument 300×200 mit Rechteck, Ellipse, Text und Bitmap,
//! alles ausgewählt; Sprachordner `korpus\German` (JSFStrings). Der Korpus dient zugleich als
//! Ressourcenordner des Originals (`FP_ORIGINAL_DIR`), damit die Kompatibilitäts-Aliase greifen.
//!
//! „Fehlerfrei“ = keine nicht abgefangene Ausnahme und kein `alert` mit Fehlermeldung.
//! Aufruf: `cargo test -p fp-script --test befehlsskripte -- --ignored --nocapture`

use fp_script::{ScriptOptions, run_file};
use serde_json::json;
use std::path::{Path, PathBuf};

fn rea() -> PathBuf {
    // FP_REA: Projektordner (enthält re\ und korpus\), z. B. bei einem Schatten-Workspace
    std::env::var_os("FP_REA")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("..")
        })
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut v: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
    v.sort();
    for p in v {
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("jsf")) {
            out.push(p);
        }
    }
}

fn session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    let steps = [
        (
            "file.new",
            json!({"width": 300, "height": 200, "background": "#ffffff"}),
        ),
        (
            "draw.rectangle",
            json!({"rect": [10, 10, 110, 80], "fillColor": "#3366cc"}),
        ),
        (
            "draw.ellipse",
            json!({"rect": [130, 20, 220, 90], "fillColor": "#cc3333"}),
        ),
        (
            "draw.text",
            json!({"text": "Firepixel", "anchor": [20, 150], "size": 24}),
        ),
        ("select.all", json!({})),
    ];
    for (c, p) in steps {
        if let Err(e) = s.execute(c, p) {
            panic!("{c}: {e}");
        }
    }
    // Bitmap über die Zwischenablage
    if let Ok(img) = fp_raster::RgbaImage::filled(
        fp_geom::IRect::new(230, 120, 290, 180),
        fp_color::Rgba8 {
            r: 20,
            g: 160,
            b: 60,
            a: 255,
        },
    ) {
        s.clipboard.pixels = Some(img);
        let _ = s.execute("edit.paste", json!({}));
    }
    let _ = s.execute("select.all", json!({}));
    s
}

#[test]
#[ignore]
fn befehlsskripte_des_originals() {
    let root = std::env::var_os("FP_CORPUS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| rea().join("korpus"));
    if std::env::var_os(fp_color::install::ENV_DIR).is_none() {
        fp_color::install::set_configured_dir(Some(root.clone()));
    }
    let mut files = Vec::new();
    collect(&root.join("Configuration").join("Commands"), &mut files);
    assert!(!files.is_empty(), "keine Skripte unter {}", root.display());
    let opts = ScriptOptions {
        language_dir: Some(root.join("German")),
        loop_limit: 20_000_000,
        confirm_answer: true,
        ..ScriptOptions::default()
    };
    let mut clean = 0;
    let mut no_abort = 0;
    for f in &files {
        let mut s = session();
        let o = run_file(&mut s, f, &opts);
        let alert_err = o.alerts.iter().any(|a| {
            a.to_lowercase().contains("fehler") || a.contains("error") || a.contains("nicht")
        });
        let name = f
            .strip_prefix(&root)
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        if o.ok {
            no_abort += 1;
        }
        if o.ok && !alert_err {
            clean += 1;
            println!("OK     {name}");
        } else {
            println!(
                "FEHLER {name}: {} {}",
                o.error.unwrap_or_default(),
                o.alerts.join(" / ").chars().take(160).collect::<String>()
            );
            if !o.not_implemented.is_empty() {
                println!(
                    "        nicht implementiert: {}",
                    o.not_implemented.join(", ")
                );
            }
        }
    }
    println!(
        "\n{clean} von {} Befehlsskripten des Originals laufen fehlerfrei; {no_abort} ohne Abbruch.",
        files.len()
    );
}

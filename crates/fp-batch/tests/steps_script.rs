//! Schritte der Gruppe „Erweitert“ (BATCH-4, SPEC-BATCH §3.13): `script.registry` und
//! `script.command` – eigene headless Sitzung je Element, gesperrte Befehle, Markierung
//! „nicht für Batch“, Dokumente bleiben Dokumente.

mod batch4_common;

use batch4_common::*;
use fp_batch::item::{DocItem, Item, Meta};
use fp_batch::steps::script::{batch_commands, check_command, excluded_from_batch};
use serde_json::json;
use std::sync::Arc;

fn temp(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("fp-batch4-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("dir");
    d
}

#[test]
fn registry_command_runs_on_still_and_animation() {
    let img = gradient(20, 10);
    let mut it = still(img.clone());
    apply(
        "script.registry",
        json!({"command": "filter.adjustColor.invert"}),
        &mut it,
    )
    .expect("run");
    assert_eq!(px(&first(&it), 3, 4)[0], 255 - px(&img, 3, 4)[0]);
    // zwei Befehle als ein Schritt: zweimal umkehren = unverändert
    let mut it = still(img.clone());
    apply("script.registry", json!({"command": "", "commands": ["filter.adjustColor.invert", {"command": "filter.adjustColor.invert"}]}), &mut it).expect("run");
    assert_eq!(hash(&first(&it)), hash(&img));
    // Animation: jedes Frame, Zeiten bleiben
    let mut it = animation(vec![(img.clone(), 30), (img.clone(), 40)]);
    apply(
        "script.registry",
        json!({"command": "filter.adjustColor.invert"}),
        &mut it,
    )
    .expect("run");
    let Item::Animation(a) = &it else {
        panic!("Animation")
    };
    assert_eq!(
        a.frames.iter().map(|f| f.delay_ms).collect::<Vec<_>>(),
        vec![30, 40]
    );
    assert!(
        a.frames
            .iter()
            .all(|f| px(&f.image, 0, 0)[0] == 255 - px(&img, 0, 0)[0])
    );
}

#[test]
fn registry_command_changes_canvas_size() {
    // Befehle, die die Leinwand ändern, bestimmen die Ergebnisgröße
    let mut it = still(gradient(30, 20));
    apply(
        "script.registry",
        json!({"command": "modify.canvas.rotate90DegreeCw"}),
        &mut it,
    )
    .expect("rotate");
    assert_eq!(it.dimensions(), (20, 30));
    let mut it = still(gradient(30, 20));
    apply(
        "script.registry",
        json!({"command": "modify.canvas.imageSize", "params": {"width": 15, "height": 10}}),
        &mut it,
    )
    .expect("image size");
    assert_eq!(it.dimensions(), (15, 10));
}

#[test]
fn registry_blocks_files_batch_and_unknown_commands() {
    for c in [
        "file.save",
        "file.exportAs",
        "batch.run",
        "commands.runScript",
        "app.quit",
    ] {
        assert!(check_command(c).is_some(), "{c} gesperrt");
        assert!(
            !errors("script.registry", json!({"command": c})).is_empty(),
            "{c}"
        );
    }
    assert!(!errors("script.registry", json!({"command": "does.not.exist"})).is_empty());
    assert!(!errors("script.registry", json!({})).is_empty(), "leer");
    assert!(
        !errors(
            "script.registry",
            json!({"command": "filter.adjustColor.invert", "params": 5})
        )
        .is_empty()
    );
    assert!(
        errors(
            "script.registry",
            json!({"command": "filter.adjustColor.invert"})
        )
        .is_empty()
    );
    // gesperrt auch beim Ausführen (ohne vorherige Prüfung)
    let mut it = still(gradient(4, 4));
    assert!(apply("script.registry", json!({"command": "file.save"}), &mut it).is_err());
}

#[test]
fn registry_keeps_documents_editable() {
    let item = still(gradient(16, 8));
    let doc = fp_batch::output::item_document(&item).expect("doc");
    let mut it = Item::Document(DocItem {
        doc: Arc::new(doc),
        meta: Meta::default(),
    });
    apply(
        "script.registry",
        json!({"command": "filter.adjustColor.invert"}),
        &mut it,
    )
    .expect("run");
    assert!(matches!(it, Item::Document(_)), "bleibt Dokument");
    let img = first(&it);
    assert_eq!(px(&img, 2, 3)[0], 255 - px(&gradient(16, 8), 2, 3)[0]);
}

#[test]
fn saved_command_runs_jsf_and_respects_not_for_batch() {
    let dir = temp("cmd");
    let ok = dir.join("Rotate.jsf");
    std::fs::write(&ok, "fw.getDocumentDOM().rotateDocument(90);\n").expect("write");
    let mut it = still(gradient(30, 20));
    apply(
        "script.command",
        json!({"path": ok.to_string_lossy()}),
        &mut it,
    )
    .expect("jsf");
    assert_eq!(it.dimensions(), (20, 30), "Dokument gedreht");
    // „nicht für Batch“ → Prüffehler und Fehler beim Ausführen
    let bad = dir.join("NoBatch.jsf");
    std::fs::write(
        &bad,
        "//--not for batch--\nfw.getDocumentDOM().rotateDocument(90);\n",
    )
    .expect("write");
    assert!(excluded_from_batch(&std::fs::read(&bad).expect("read")));
    assert!(!errors("script.command", json!({"path": bad.to_string_lossy()})).is_empty());
    let mut it = still(gradient(30, 20));
    assert!(
        apply(
            "script.command",
            json!({"path": bad.to_string_lossy()}),
            &mut it
        )
        .is_err()
    );
    assert_eq!(it.dimensions(), (30, 20));
    // gespeicherte Batch-Skripte selbst sind ebenfalls ausgeschlossen
    assert!(excluded_from_batch(b"// Batch Script Template\nvar x;"));
    assert!(excluded_from_batch(b"var gBatchSettings = {};"));
    assert!(!excluded_from_batch(b"fw.getDocumentDOM();"));
    // fehlende Datei: Warnung bei der Prüfung, Fehler beim Ausführen
    assert!(
        errors(
            "script.command",
            json!({"path": dir.join("missing.jsf").to_string_lossy()})
        )
        .is_empty()
    );
    assert!(
        apply(
            "script.command",
            json!({"path": dir.join("missing.jsf").to_string_lossy()}),
            &mut it
        )
        .is_err()
    );
    // ohne Name und Pfad, Name mit ..
    assert!(!errors("script.command", json!({})).is_empty());
    assert!(!errors("script.command", json!({"name": "../evil"})).is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn batch_commands_list_skips_excluded_files() {
    // nur lesen: die Liste aus dem (Test-)Befehlsordner ist sortiert und enthält keine ausgeschlossenen
    for (name, path) in batch_commands() {
        assert!(!name.is_empty());
        assert!(!excluded_from_batch(
            &std::fs::read(&path).unwrap_or_default()
        ));
    }
}

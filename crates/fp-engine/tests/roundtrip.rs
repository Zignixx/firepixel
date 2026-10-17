//! Rundreise: Korpusdatei öffnen → Befehle → speichern → mit fp-fwpng wieder lesen.

use crate::common;
use common::*;
use fp_engine::Session;
use serde_json::json;
use std::path::{Path, PathBuf};

fn corpus_root() -> PathBuf {
    let m = std::fs::canonicalize(env!("CARGO_MANIFEST_DIR")).unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    let corpus = std::env::var_os("FP_CORPUS_DIR").map(PathBuf::from).unwrap_or_else(|| m.join("../../../korpus"));
    corpus.join("First Run/Common Library")
}

fn temp_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fp-engine-rt-{tag}-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d
}

fn top_count(doc: &fp_model::Document, page: usize) -> usize {
    fp_engine::docops::own_layers(doc, page).iter().filter_map(|(_, l)| l.cels.first().map(|c| c.elements.len())).sum()
}

/// Eine Korpusdatei: öffnen, Befehle, speichern, wieder lesen und vergleichen.
fn roundtrip_file(src: &Path, out: &Path) -> Result<(), String> {
    let mut s = Session::default();
    s.execute("file.open", json!({"path": src.to_string_lossy()})).map_err(|e| format!("öffnen: {e}"))?;
    let (pages, n0) = {
        let d = s.active().ok_or("kein Dokument")?;
        (d.doc.pages.len(), top_count(&d.doc, d.page))
    };
    s.execute("select.all", json!({})).map_err(|e| e.to_string())?;
    if s.active().is_some_and(|d| !d.selection.elements.is_empty()) {
        s.execute("edit.move", json!({"dx": 3, "dy": 4})).map_err(|e| format!("verschieben: {e}"))?;
    }
    s.execute("draw.rectangle", json!({"rect": [1, 2, 31, 22], "name": "Firepixel-Test"})).map_err(|e| e.to_string())?;
    s.execute("modify.canvas.canvasColor", json!({"color": "#336699"})).map_err(|e| e.to_string())?;
    s.execute("file.saveAs", json!({"path": out.to_string_lossy()})).map_err(|e| format!("speichern: {e}"))?;
    let bytes = std::fs::read(out).map_err(|e| e.to_string())?;
    let back = fp_fwpng::read(&bytes).map_err(|e| format!("wieder lesen: {e}"))?;
    if back.pages.len() != pages {
        return Err(format!("Seiten {} statt {pages}", back.pages.len()));
    }
    let page = s.active().map(|d| d.page).unwrap_or(0);
    let n1 = top_count(&back, page);
    if n1 != n0 + 1 {
        return Err(format!("Elemente {n1} statt {}", n0 + 1));
    }
    let mut named = false;
    back.walk_elements(|_, _, e| named |= e.common.name.as_deref() == Some("Firepixel-Test"));
    if !named {
        return Err("neues Rechteck fehlt".into());
    }
    if back.pages[page].root.background.0 != 0xff33_6699 {
        return Err("Leinwandfarbe".into());
    }
    Ok(())
}

#[test]
fn corpus_file_roundtrip() {
    let src = corpus_root().join("Cursors/Resize-W.graphic.png");
    if !src.exists() {
        println!("Korpusdatei fehlt, Test übersprungen: {}", src.display());
        return;
    }
    let dir = temp_dir("one");
    let r = roundtrip_file(&src, &dir.join("out.fw.png"));
    let _ = std::fs::remove_dir_all(&dir);
    if let Err(e) = r {
        panic!("{e}");
    }
}

#[test]
fn new_document_roundtrip() {
    let dir = temp_dir("new");
    let path = dir.join("neu.fw.png");
    let mut s = doc_session(160.0, 90.0);
    rect(&mut s, 10.0, 10.0, 40.0, 30.0);
    ellipse(&mut s, 50.0, 10.0, 80.0, 40.0);
    ex(&mut s, "draw.text", json!({"text": "Text", "anchor": [10, 70], "size": 14}));
    ex(&mut s, "select.all", json!({}));
    ex(&mut s, "modify.group", json!({}));
    ex(&mut s, "edit.insert.state", json!({}));
    ex(&mut s, "edit.insert.page", json!({"name": "Zwei"}));
    ex(&mut s, "file.saveAs", json!({"path": path.to_string_lossy()}));
    let back = fp_fwpng::read(&std::fs::read(&path).unwrap_or_default()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(back.pages.len(), 2);
    assert_eq!(back.pages[0].root.frame_count, 2);
    assert_eq!(back.pages[1].display_name(), "Zwei");
    let mut groups = 0;
    back.walk_elements(|_, _, e| {
        if e.js_type() == "Group" {
            groups += 1;
        }
    });
    assert_eq!(groups, 1);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Korpus: die ersten 80 FW-PNGs der Allgemeinen Bibliothek (Rundreise wie oben).
#[test]
#[ignore]
fn corpus_roundtrip_many() {
    let mut files = Vec::new();
    let mut stack = vec![corpus_root()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("png")) {
                files.push(p);
            }
        }
    }
    files.sort();
    let files: Vec<_> = files.into_iter().filter(|p| std::fs::read(p).is_ok_and(|b| fp_fwpng::is_fwpng(&b))).take(80).collect();
    let dir = temp_dir("many");
    let mut ok = 0;
    let mut errs = Vec::new();
    for (i, f) in files.iter().enumerate() {
        match roundtrip_file(f, &dir.join(format!("{i}.fw.png"))) {
            Ok(()) => ok += 1,
            Err(e) => errs.push(format!("{}: {e}", f.display())),
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    println!("Rundreise Korpus: {ok}/{} Dateien", files.len());
    for e in &errs {
        println!("  {e}");
    }
    assert!(errs.is_empty());
}

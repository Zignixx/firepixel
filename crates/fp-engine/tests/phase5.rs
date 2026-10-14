//! Fachtests der in Phase 5 verdrahteten Befehle (Ergebnis am Dokument bzw. an den Dateien).

use crate::common;

use common::*;
use fp_engine::Session;
use fp_model::ElementId;
use serde_json::json;
use std::path::PathBuf;

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join("fp_phase5").join(name);
    let _ = std::fs::remove_dir_all(&d);
    let _ = std::fs::create_dir_all(&d);
    d
}

fn base() -> (Session, ElementId, ElementId) {
    let mut s = doc_session(200.0, 120.0);
    let r = rect(&mut s, 10.0, 10.0, 60.0, 50.0);
    let e = ellipse(&mut s, 80.0, 20.0, 150.0, 90.0);
    (s, r, e)
}

#[test]
fn export_styles_write_valid_files() {
    let (mut s, _, _) = base();
    ex(&mut s, "web.addSlice", json!({"shape": "rect", "rect": [0, 0, 100, 60]}));
    let dir = tmp("export");
    for (style, ext, magic) in [
        ("imagesOnly", "png", &b"\x89PNG"[..]),
        ("psd", "psd", &b"8BPS"[..]),
        ("pdf", "pdf", &b"%PDF"[..]),
        ("svg", "svg", &b"<"[..]),
    ] {
        let v = ex(&mut s, "file.export", json!({"path": dir.join(format!("bild.{ext}")).to_string_lossy(), "style": style, "settings": {"format": "png32"}}));
        let f = v["files"][0].as_str().unwrap_or_default().to_string();
        let b = std::fs::read(&f).unwrap_or_default();
        assert!(b.starts_with(magic), "{style}: {f}");
    }
    // HTML mit Segmenten in einen Ordner
    let v = ex(&mut s, "file.export", json!({"path": dir.join("web").to_string_lossy(), "style": "htmlAndImages", "baseName": "seite"}));
    let files: Vec<String> = v["files"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
    assert!(files.iter().any(|f| f.ends_with("seite.htm")), "{files:?}");
    // Vorgabe neuer Dokumente: PNG 32 (BA-033)
    assert!(files.iter().filter(|f| f.ends_with(".png")).count() >= 2, "{files:?}");
    // Exportbereich: Pixel der Ellipse (rot vorgegeben? Füllfarbe des Werkzeugs) – Größe genau
    let v = ex(&mut s, "file.exportArea", json!({"rect": [80, 20, 150, 90], "path": dir.join("bereich.png").to_string_lossy(), "settings": {"format": "png32"}}));
    assert_eq!((v["width"].as_u64(), v["height"].as_u64()), (Some(70), Some(70)));
    // Bildvorschau mit Zielgröße
    let v = ex(&mut s, "file.imagePreview", json!({"settings": {"format": "jpeg", "quality": 90}, "targetSize": 1500}));
    assert!(v["bytes"].as_u64().unwrap_or(u64::MAX) <= 3000, "{v}");
}

#[test]
fn renderer_draws_vectors_for_export() {
    let (mut s, _, _) = base();
    ex(&mut s, "select.all", json!({}));
    ex(&mut s, "element.setFill", json!({"color": "#ff0000"}));
    let dir = tmp("render");
    let p = dir.join("a.png");
    ex(&mut s, "file.exportArea", json!({"rect": [30, 25, 31, 26], "path": p.to_string_lossy(), "settings": {"format": "png32"}}));
    // Pixel aus dem gerenderten Rechteck (fp-render, nicht nur Bitmaps)
    let r = fp_engine::commands::load_document(&p).expect("png");
    let img = fp_export::render::SimpleRenderer;
    use fp_export::render::PageRenderer as _;
    let px = img.render(&r, &fp_export::render::RenderRequest::default()).expect("render");
    assert_eq!(px.pixel(0, 0), fp_color::Rgba8::new(255, 0, 0, 255));
}

#[test]
fn symbol_edit_roundtrip_is_one_undo_step() {
    let (mut s, _, e) = base();
    select(&mut s, &[e]);
    ex(&mut s, "modify.symbol.convertToSymbol", json!({"name": "Kreis"}));
    let parent = s.active_id().unwrap_or(fp_engine::DocId(0));
    let undo_before = s.active().map(|d| d.history.undo_count()).unwrap_or(0);
    ex(&mut s, "modify.symbol.editSymbol", json!({}));
    assert_ne!(s.active_id(), Some(parent));
    ex(&mut s, "draw.rectangle", json!({"rect": [0, 0, 5, 5]}));
    ex(&mut s, "modify.symbol.exitEdit", json!({}));
    assert_eq!(s.active_id(), Some(parent));
    let d = s.active().expect("doc");
    assert_eq!(d.history.undo_count(), undo_before + 1);
    let sym = d.doc.symbols.iter().find(|x| x.name == "Kreis").expect("Symbol");
    let n: usize = sym.root.layers.iter().filter_map(|l| if let fp_model::LayerSlot::Own(l) = l { l.cels.first().map(|c| c.elements.len()) } else { None }).sum();
    assert_eq!(n, 2, "Symbol hat Ellipse + neues Rechteck");
    ex(&mut s, "edit.undo", json!({}));
    let d = s.active().expect("doc");
    let sym = d.doc.symbols.iter().find(|x| x.name == "Kreis").expect("Symbol");
    let n: usize = sym.root.layers.iter().filter_map(|l| if let fp_model::LayerSlot::Own(l) = l { l.cels.first().map(|c| c.elements.len()) } else { None }).sum();
    assert_eq!(n, 1);
}

#[test]
fn convert_text_to_paths_gives_glyph_outlines() {
    let mut s = doc_session(300.0, 100.0);
    let t = ElementId(ex(&mut s, "draw.text", json!({"text": "AB", "anchor": [10, 60], "size": 40, "font": "Arial", "color": "#00ff00"}))["id"].as_u64().unwrap_or(0));
    select(&mut s, &[t]);
    ex(&mut s, "text.convertToPaths", json!({}));
    let g = top_elements(&s)[0].clone();
    let fp_model::ElementKind::Group(gr) = &g.kind else { panic!("keine Gruppe: {}", g.js_type()) };
    assert_eq!(gr.children.len(), 2);
    let b = fp_engine::docops::element_bounds(&s.active().expect("doc").doc, &g).expect("Grenzen");
    assert!(b.left >= 9.0 && b.right < 120.0 && b.top > 20.0 && b.bottom <= 61.0, "{b:?}");
    let a = fp_engine::docops::path_attrs_of(&gr.children[0]).expect("Attribute");
    assert_eq!(a.fill_color, fp_color::Argb(0xff00ff00));
}

#[test]
fn spelling_with_hunspell_dictionary() {
    let dir = tmp("dict");
    std::fs::write(dir.join("xx_TEST.aff"), "SET UTF-8\n").ok();
    std::fs::write(dir.join("xx_TEST.dic"), "3\nHallo\nWelt\nWald\n").ok();
    let mut s = doc_session(200.0, 100.0);
    ex(&mut s, "text.spellingSetup", json!({"language": "xx_TEST", "dictionaryDir": dir.to_string_lossy()}));
    ex(&mut s, "draw.text", json!({"text": "Hallo Wlet", "anchor": [10, 50]}));
    let v = ex(&mut s, "text.checkSpelling", json!({}));
    assert_eq!(v["dictionary"], true);
    assert_eq!(v["misspelled"][0]["word"], "Wlet", "{v}");
    assert!(v["misspelled"][0]["suggestions"].as_array().is_some_and(|a| a.iter().any(|x| x == "Welt")), "{v}");
    let v = ex(&mut s, "text.checkSpelling", json!({"replace": [{"word": "Wlet", "with": "Welt"}]}));
    assert_eq!(v["replaced"], 1);
    assert!(v["misspelled"].as_array().is_some_and(|a| a.is_empty()), "{v}");
}

#[test]
fn custom_shortcut_set_is_saved_and_used() {
    let mut s = doc_session(50.0, 50.0);
    let name = format!("Test-{}", std::process::id());
    ex(&mut s, "edit.keyboardShortcuts", json!({"duplicate": name}));
    ex(&mut s, "edit.keyboardShortcuts", json!({"command": "modify.group", "shortcuts": ["Strg+Alt+Umschalt+Q"]}));
    let sc = fp_engine::Shortcut::parse("Ctrl+Alt+Shift+Q").expect("Kürzel");
    assert_eq!(s.command_for_shortcut(&sc).as_deref(), Some("modify.group"));
    assert!(fp_engine::shortcuts::custom_set_names().contains(&name));
    ex(&mut s, "edit.keyboardShortcuts", json!({"deleteSet": name}));
    assert!(!fp_engine::shortcuts::custom_set_names().contains(&name));
}

#[test]
fn shared_layer_contents_are_editable() {
    let (mut s, r, _) = base();
    ex(&mut s, "page.add", json!({}));
    ex(&mut s, "page.setCurrent", json!({"index": 0}));
    ex(&mut s, "panel.layers.shareToAllPages", json!({}));
    // auf Seite 2 auswählen und verschieben – wirkt auf die geteilte Ebene (beide Seiten)
    ex(&mut s, "page.setCurrent", json!({"index": 1}));
    select(&mut s, &[r]);
    ex(&mut s, "edit.move", json!({"dx": 5, "dy": 0}));
    assert!((bounds(&s, r).left - 15.0).abs() < 0.01);
    ex(&mut s, "page.setCurrent", json!({"index": 0}));
    assert!((bounds(&s, r).left - 15.0).abs() < 0.01);
}

#[test]
fn import_psd_and_open_formats() {
    // PSD mit zwei Ebenen über den eigenen Export erzeugen, dann importieren und öffnen
    let (mut s, _, _) = base();
    ex(&mut s, "layer.add", json!({"name": "Zweite"}));
    rect(&mut s, 100.0, 60.0, 140.0, 100.0);
    let dir = tmp("psd");
    let psd = dir.join("ebenen.psd");
    ex(&mut s, "file.export", json!({"path": psd.to_string_lossy(), "style": "psd"}));
    let mut t = doc_session(300.0, 300.0);
    let v = ex(&mut t, "file.import", json!({"path": psd.to_string_lossy(), "at": [0, 0]}));
    assert!(v["ids"].as_array().is_some_and(|a| !a.is_empty()), "{v}");
    ex(&mut t, "file.open", json!({"path": psd.to_string_lossy()}));
    assert_eq!(t.documents().len(), 2);
    let layers = fp_engine::docops::own_layers(&t.active().expect("doc").doc, 0).len();
    assert!(layers >= 2, "Ebenen: {layers}");
}
#[test]
fn popup_menu_exported_as_html() {
    let mut s = doc_session(200.0, 100.0);
    let sl = ElementId(ex(&mut s, "web.addSlice", json!({"shape": "rect", "rect": [0, 0, 80, 30]}))["id"].as_u64().unwrap_or(0));
    select(&mut s, &[sl]);
    ex(&mut s, "modify.popUpMenu.addPopUpMenu", json!({"items": [{"text": "Startseite", "url": "index.htm"}]}));
    let v = ex(&mut s, "edit.copyHtmlCode", json!({}));
    let html = v["html"].as_str().unwrap_or_default();
    assert!(html.contains("Startseite") || v["images"].as_array().is_some_and(|a| !a.is_empty()), "{html}");
}

/// Gesperrte Objekte: nicht auswählbar, Befehle lassen sie unverändert.
#[test]
fn locked_objects_are_not_edited() {
    let (mut s, r, e) = base();
    select(&mut s, &[r]);
    ex(&mut s, "element.setLocked", json!({"locked": true}));
    let before = bounds(&s, r);
    // Auswahl über IDs nimmt das gesperrte Objekt nicht auf
    ex(&mut s, "select.set", json!({"ids": [r.0, e.0]}));
    assert_eq!(s.active().map(|d| d.selection.elements.clone()), Some(vec![e]));
    // auch eine bestehende Auswahl mit gesperrtem Objekt verändert es nicht
    s.doc_mut().expect("doc").selection.set(vec![r, e]);
    ex(&mut s, "modify.transform.flipHorizontal", json!({}));
    ex(&mut s, "modify.transform.rotate90DegreeCw", json!({}));
    assert_eq!(bounds(&s, r), before);
    // entsperren über die ID geht
    ex(&mut s, "element.setLocked", json!({"ids": [r.0], "locked": false}));
    assert!(!el(&s, r).common.locked);
}

/// Nutzerbefund: Glühen-Größe mehrmals geändert → viele Glühen. `element.setEffect` ersetzt.
#[test]
fn set_effect_replaces_instead_of_appending() {
    let mut s = Session::default();
    s.prefs.set("MaxUndos", fp_engine::PrefValue::Int(100));
    ex(&mut s, "file.new", json!({"width": 200, "height": 120}));
    let r = rect(&mut s, 10.0, 10.0, 60.0, 50.0);
    select(&mut s, &[r]);
    for w in 1..=50 {
        ex(&mut s, "element.setEffect", json!({"effect": "glow", "params": {"GlowWidth": w.to_string()}}));
    }
    let e = el(&s, r);
    let l = e.common.effects.clone().expect("Effekte");
    assert_eq!(l.effects.len(), 1, "genau ein Glühen");
    assert_eq!(l.effects[0].params.get_str("GlowWidth"), Some("50"));
    // auch die Kopie in der Rechteck-Grundform (PRIv/EFLv)
    if let fp_model::ElementKind::Path(p) = &e.kind
        && let Some(pr) = &p.primitive
    {
        assert_eq!(pr.effects.as_ref().map(|x| x.effects.len()), Some(1));
    }
    // 50 Schritte rückgängig → kein Effekt
    for _ in 0..50 {
        let _ = s.undo();
    }
    assert!(el(&s, r).common.effects.is_none());
}
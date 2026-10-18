//! RT-2 (re\27 E1, BA-121): Schaltflächen-Instanzen liefern im HTML-Export ihr Rollover aus den
//! Symboloptionen (re\13 §7.5, re\15 §2.5): „Down-Status einschließen“ aus ⇒ Einfaches Rollover
//! (`MM_swapImage`/`MM_swapImgRestore`), an ⇒ Navigationsleistenbild (`MM_nbGroup`), Statusleistentext
//! ⇒ `MM_displayStatusMsg`; je benutztem Status ein eigenes Segmentbild aus dem Status des Symbols.

use fp_color::Argb;
use fp_export::doc_export::{
    DocExportOptions, ExportFile, FilesStyle, button_state_doc, export_document,
};
use fp_export::export::EncodeExtras;
use fp_export::render::RenderRequest;
use fp_export::slices::collect_slices;
use fp_geom::{IRect, Matrix3, Rect};
use fp_model::{
    Document, Element, ElementKind, LayerKind, LayerSlot, PathAttrs, Symbol, SymbolKind,
};
use fp_raster::RgbaImage;
use std::sync::Arc;

/// Seite 200 × 100 mit einer Schaltflächen-Instanz bei (50, 30). Status k des Symbols enthält k + 1
/// Rechtecke (0,0–40,20), so lässt sich der gezeigte Status am Bild ablesen.
fn doc(include_down: bool, over_down: bool, message: Option<&str>) -> Document {
    let mut d = Document::new(200.0, 100.0, Argb(0xffff_ffff));
    let mut ids = std::mem::take(&mut d.ids);
    let sid = d.new_symbol_id();
    let mut sym = Symbol::new(&mut ids, "Knopf", sid, SymbolKind::Button);
    sym.button.url = Some("https://example.org/".into());
    sym.button_options.include_down = include_down;
    sym.button_options.include_over_down = over_down;
    sym.status_message = message.map(str::to_string);
    for slot in &mut sym.root.layers {
        let LayerSlot::Own(l) = slot else { continue };
        let l = Arc::make_mut(l);
        if l.kind != LayerKind::Normal {
            continue;
        }
        for (k, cel) in l.cels.iter_mut().enumerate() {
            let cel = Arc::make_mut(cel);
            for _ in 0..=k {
                let r = Element::new_rectangle(
                    &mut ids,
                    Rect::new(0.0, 0.0, 40.0, 20.0),
                    0.0,
                    PathAttrs::solid_fill(Argb(0xff33_6699)),
                );
                cel.elements.push(Arc::new(r));
            }
        }
    }
    let inst = Element::new_instance(&mut ids, sid, Matrix3::translation(50.0, 30.0));
    d.ids = ids;
    d.symbols.push(Arc::new(sym));
    let p = d.page_mut(0).expect("Seite");
    if let LayerSlot::Own(l) = &mut p.root.layers[0] {
        let l = Arc::make_mut(l);
        Arc::make_mut(&mut l.cels[0]).elements.push(Arc::new(inst));
    }
    d
}

/// Anzahl der Rechtecke im ersten Status des Symbols der (einzigen) Instanz der Seite.
fn shown_rects(d: &Document) -> usize {
    let mut n = 0;
    d.walk_elements(|_, path, e| {
        if matches!(path.root, fp_model::RootRef::Page(0))
            && let ElementKind::Instance(i) = &e.kind
            && let Some((_, s)) = d.symbol_by_id(i.symbol_id)
        {
            for slot in &s.root.layers {
                if let LayerSlot::Own(l) = slot
                    && l.kind == LayerKind::Normal
                {
                    n = l.cels.first().map(|c| c.elements.len()).unwrap_or(0);
                }
            }
        }
    });
    n
}

/// Renderer für den Test: ganze Leinwand in einem Grau, das vom gezeigten Status abhängt.
fn render(d: &Document, req: &RenderRequest) -> Result<RgbaImage, String> {
    let g = (shown_rects(d) * 40) as u8;
    let r = IRect::new(0, 0, 200, 100);
    let bytes: Vec<u8> = (0..200 * 100).flat_map(|_| [g, g, g, 255]).collect();
    let _ = req;
    RgbaImage::from_rgba_bytes(r, &bytes).map_err(|e| e.to_string())
}

fn export(d: &Document) -> (String, Vec<ExportFile>) {
    let mut o = DocExportOptions::new("knopf");
    o.style = FilesStyle::HtmlAndImages;
    let files = export_document(d, &render, &o, &EncodeExtras::default()).expect("Export");
    let h = files
        .iter()
        .find(|f| f.path.ends_with(".htm") || f.path.ends_with(".html"))
        .expect("HTML-Datei");
    (String::from_utf8(h.bytes.clone()).expect("UTF-8"), files)
}

fn file<'a>(files: &'a [ExportFile], needle: &str) -> Option<&'a ExportFile> {
    files.iter().find(|f| f.path.contains(needle))
}

#[test]
fn simple_rollover_from_symbol_options() {
    let d = doc(false, false, None);
    let s = collect_slices(&d, 0);
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].behaviors.len(), 1);
    assert_eq!(s[0].behaviors[0].action, 8, "Einfaches Rollover");
    let (h, files) = export(&d);
    assert!(h.contains("MM_swapImage("), "Bild austauschen fehlt:\n{h}");
    assert!(
        h.contains("MM_swapImgRestore()"),
        "Wiederherstellen fehlt:\n{h}"
    );
    assert!(h.contains("MM_preloadImages("), "Vorladen fehlt:\n{h}");
    assert!(!h.contains("MM_nbGroup"), "keine Navigationsleiste:\n{h}");
    // Grundbild und Status 2 (Over) aus dem Symbol, keine weiteren Status
    let base = file(&files, "knopf_r2_c2.").expect("Grundbild");
    let over = file(&files, "knopf_r2_c2_s2.").expect("Over-Bild");
    assert!(h.contains(&over.path), "Over-Bild im HTML:\n{h}");
    assert_ne!(
        base.bytes, over.bytes,
        "Over-Bild zeigt Status 2 des Symbols"
    );
    assert!(file(&files, "_s3.").is_none() && file(&files, "_s4.").is_none());
}

#[test]
fn nav_bar_with_down_and_over_while_down() {
    let d = doc(true, true, Some("Hallo Welt"));
    let s = collect_slices(&d, 0);
    let acts: Vec<i32> = s[0].behaviors.iter().map(|b| b.action).collect();
    assert_eq!(acts, vec![0, 1], "Navigationsleistenbild + Statusmeldung");
    assert_eq!(s[0].behaviors[0].flags, Some(3), "ROLi = 2 | OverDown");
    let (h, files) = export(&d);
    assert!(h.contains("MM_nbGroup('over'"), "Nav-Over fehlt:\n{h}");
    assert!(h.contains("MM_nbGroup('out')"), "Nav-Restore fehlt:\n{h}");
    assert!(
        h.contains("MM_nbGroup('down','navbar1'"),
        "Nav-Down fehlt:\n{h}"
    );
    assert!(
        h.contains("MM_displayStatusMsg('Hallo Welt')"),
        "Statusmeldung fehlt:\n{h}"
    );
    // Over (2), Down (3) und Over-While-Down (4) als eigene Bilder, je aus dem Symbolstatus
    let imgs: Vec<&ExportFile> = ["knopf_r2_c2.", "_s2.", "_s3.", "_s4."]
        .iter()
        .map(|n| file(&files, n).unwrap_or_else(|| panic!("{n} fehlt")))
        .collect();
    for i in 0..imgs.len() {
        for j in i + 1..imgs.len() {
            assert_ne!(
                imgs[i].bytes, imgs[j].bytes,
                "{} und {} gleich",
                imgs[i].path, imgs[j].path
            );
        }
    }
}

#[test]
fn nav_bar_without_over_down_uses_down_as_highlight() {
    let d = doc(true, false, None);
    let (h, files) = export(&d);
    assert!(h.contains("MM_nbGroup('over'"), "{h}");
    assert!(file(&files, "_s2.").is_some() && file(&files, "_s3.").is_some());
    assert!(
        file(&files, "_s4.").is_none(),
        "ohne Over-While-Down kein Status 4"
    );
}

#[test]
fn down_on_load_instance_overrides_symbol() {
    let mut d = doc(true, false, None);
    let p = d.page_mut(0).expect("Seite");
    if let LayerSlot::Own(l) = &mut p.root.layers[0] {
        let l = Arc::make_mut(l);
        let e = Arc::make_mut(&mut Arc::make_mut(&mut l.cels[0]).elements[0]);
        if let ElementKind::Instance(i) = &mut e.kind {
            i.show_down_on_load = Some(true);
        }
    }
    let s = collect_slices(&d, 0);
    assert_eq!(s[0].behaviors[0].checked, Some(true));
    let (h, _) = export(&d);
    assert!(
        h.contains("MM_nbGroup('init','navbar1'"),
        "Down beim Laden:\n{h}"
    );
}

/// re\27 E2 (BA-122): automatischer Segmentname wie im Inspektor des Originals – Dokument ohne
/// Datei ⇒ „_r2_c2“, mit Dateiname „test“ ⇒ „test_r2_c2“; eigener Name ⇒ kein automatischer.
#[test]
fn auto_slice_name_like_original() {
    use fp_export::slices::{NameDate, auto_name, auto_name_of};
    let mut d = Document::new(300.0, 300.0, Argb(0xffff_ffff));
    let mut ids = std::mem::take(&mut d.ids);
    let a = Element::new_slice(&mut ids, Rect::new(100.0, 100.0, 200.0, 200.0));
    let b = Element::new_slice(&mut ids, Rect::new(200.0, 0.0, 300.0, 50.0));
    let (ida, idb) = (a.id, b.id);
    d.ids = ids;
    let p = d.page_mut(0).expect("Seite");
    for slot in &mut p.root.layers {
        if let LayerSlot::Own(l) = slot
            && l.kind == LayerKind::Web
        {
            let l = Arc::make_mut(l);
            let cel = Arc::make_mut(&mut l.cels[0]);
            cel.elements.push(Arc::new(a.clone()));
            cel.elements.push(Arc::new(b.clone()));
        }
    }
    let date = NameDate {
        year: 2026,
        month: 10,
        day: 15,
    };
    // Spalten 0/100/200/300, Zeilen 0/50/100/200/300 ⇒ Segment a in Zeile 3, Spalte 2
    assert_eq!(
        auto_name_of(&d, 0, ida, "", date).as_deref(),
        Some("_r3_c2")
    );
    assert_eq!(
        auto_name_of(&d, 0, idb, "", date).as_deref(),
        Some("_r1_c3")
    );
    assert_eq!(
        auto_name_of(&d, 0, ida, "test", date).as_deref(),
        Some("test_r3_c2")
    );
    // regeln.py-Selbsttests
    assert_eq!(
        auto_name(&[6, 15, 1, 0, 0, 0], 2, 2, "", 1, 1, date),
        "_r2_c2"
    );
    assert_eq!(
        auto_name(&[6, 15, 1, 0, 0, 0], 1, 3, "test", 1, 1, date),
        "test_r1_c3"
    );
    // Dokumentname über 240 Zeichen ⇒ leer; Gesamtlänge ≤ 240
    let long = "x".repeat(241);
    assert_eq!(auto_name(&[6, 15, 1], 2, 2, &long, 1, 1, date), "_r2_c2");
    let max = "y".repeat(240);
    assert_eq!(
        auto_name(&[6, 15, 1], 2, 2, &max, 1, 1, date)
            .chars()
            .count(),
        240
    );
    // laufende Nummer zweistellig, ab 99 Segmenten dreistellig; Datum ohne führende Nullen
    assert_eq!(auto_name(&[5], 1, 1, "", 7, 98, date), "07");
    assert_eq!(auto_name(&[5], 1, 1, "", 7, 99, date), "007");
    assert_eq!(
        auto_name(
            &[8],
            1,
            1,
            "",
            1,
            1,
            NameDate {
                year: 2026,
                month: 3,
                day: 5
            }
        ),
        "3526"
    );
    // eigener Name: kein automatischer
    let mut d2 = d.clone();
    let path = d2.find_element(ida).expect("Segment");
    if let Some(e) = d2.element_at_mut(&path)
        && let ElementKind::WebObject(w) = &mut e.kind
    {
        w.file_name = Some("eigen".into());
    }
    assert_eq!(auto_name_of(&d2, 0, ida, "", date), None);
}

#[test]
fn state_doc_shows_symbol_state() {
    let d = doc(true, true, None);
    let id = match &d.pages[0].root.layers[0] {
        LayerSlot::Own(l) => l.cels[0].elements[0].id,
        _ => unreachable!(),
    };
    assert_eq!(shown_rects(&d), 1);
    for k in 1..4 {
        let sd = button_state_doc(&d, id, k).expect("Schaltfläche");
        assert_eq!(shown_rects(&sd), k + 1, "Status {}", k + 1);
    }
    // das Original bleibt unverändert
    assert_eq!(shown_rects(&d), 1);
}

/// BA-103: "Generic" (HTML5) style with page styles in `styles.css` – the nav bar of a button
/// symbol (up/over/down) is written as plain HTML + CSS + JS next to its state images.
#[test]
fn generic_style_nav_bar_with_external_page_css() {
    let mut d = doc(true, false, None);
    let p = d.page_mut(0).expect("Seite");
    let e = p.root.export.get_or_insert_with(Default::default);
    let mut h = fp_model::HtmlSettings::cs6_default();
    h.xhtml = Some(fp_export::doc_export::HTML_STYLE_GENERIC);
    h.external_css_for_doc = Some(true);
    h.background_url = Some("bg.png".into());
    e.html = Some(Box::new(h));
    let (h, files) = export(&d);
    assert!(
        h.starts_with("<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">"),
        "{h}"
    );
    assert!(!h.contains("<style"), "no inline styles:\n{h}");
    assert!(
        h.contains("<link rel=\"stylesheet\" type=\"text/css\" href=\"styles.css\">"),
        "{h}"
    );
    assert!(!h.contains(" />"), "HTML5 without XML closers:\n{h}");
    assert!(h.contains("onmouseover=\"MM_nbGroup('over'"), "{h}");
    assert!(h.contains("onclick=\"MM_nbGroup('down','navbar1'"), "{h}");
    assert!(
        h.contains("function MM_nbGroup("),
        "nav-bar script in the page:\n{h}"
    );
    let css = file(&files, "styles.css").expect("styles.css");
    let css = String::from_utf8_lossy(&css.bytes);
    assert!(css.contains("td img {display: block;}"), "{css}");
    assert!(css.contains("background-image: url(\"bg.png\");"), "{css}");
    for n in ["knopf_r2_c2.", "_s2.", "_s3."] {
        let f = file(&files, n).unwrap_or_else(|| panic!("{n} missing"));
        assert!(h.contains(&f.path), "{} referenced:\n{h}", f.path);
    }
}

/// BA-103: "Set defaults" – pages without their own HTML settings use the stored defaults.
#[test]
fn stored_defaults_apply_to_pages_without_settings() {
    let mut d = doc(false, false, None);
    if let Some(e) = d.page_mut(0).expect("Seite").root.export.as_mut() {
        e.html = None;
    }
    let mut o = DocExportOptions::new("knopf");
    o.style = FilesStyle::HtmlAndImages;
    let mut h = fp_model::HtmlSettings::cs6_default();
    h.xhtml = Some(fp_export::doc_export::HTML_STYLE_GENERIC);
    h.external_css_for_doc = Some(true);
    o.html_prefs.defaults = Some(h);
    let files = export_document(&d, &render, &o, &EncodeExtras::default()).expect("Export");
    let html = files
        .iter()
        .find(|f| f.path == "knopf.htm")
        .expect("knopf.htm");
    assert!(String::from_utf8_lossy(&html.bytes).starts_with("<!DOCTYPE html>"));
    assert!(files.iter().any(|f| f.path == "styles.css"));
    // without defaults: XHTML and inline styles as before
    let (h, files) = export(&d);
    assert!(h.contains("XHTML 1.0") && h.contains("<style"), "{h}");
    assert!(!files.iter().any(|f| f.path == "styles.css"));
}

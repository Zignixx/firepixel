//! B-5 (BA-071/BA-072): Schaltflächen-Instanzen sind im HTML-Export Segmente wie auf der Leinwand
//! (TW-19). Fläche = aktiver Bereich des Schaltflächensymbols (Web-Ebene des Symbols, re\13 §1.4),
//! sonst die Instanzgrenzen; Link/Alt/Ziel der Instanz mit Rückfall auf das Symbol.

use fp_color::Argb;
use fp_export::doc_export::{export_document, DocExportOptions, FilesStyle};
use fp_export::export::EncodeExtras;
use fp_export::render::SimpleRenderer;
use fp_export::slices::{button_active_area, collect_slices, BoundsFn};
use fp_geom::{Matrix3, Rect};
use fp_model::{Document, Element, LayerKind, LayerSlot, PathAttrs, Symbol, SymbolKind};
use std::sync::Arc;

/// Seite 200 × 100 mit einer Schaltflächen-Instanz bei (50, 30), Symbolinhalt 0,0–40,20.
/// `active`: aktiver Bereich im Symbol (Web-Ebene), `web_slice`: zusätzlich ein Segment der Seite.
fn doc(active: Option<Rect>, web_slice: Option<Rect>) -> Document {
    let mut d = Document::new(200.0, 100.0, Argb(0xffff_ffff));
    let mut ids = std::mem::take(&mut d.ids);
    let sid = d.new_symbol_id();
    let mut sym = Symbol::new(&mut ids, "Knopf", sid, SymbolKind::Button);
    sym.button.url = Some("https://example.org/".into());
    sym.button.alt = Some("Symboltext".into());
    let rect = Element::new_rectangle(&mut ids, Rect::new(0.0, 0.0, 40.0, 20.0), 0.0, PathAttrs::solid_fill(Argb(0xff33_6699)));
    let area = active.map(|r| Element::new_slice(&mut ids, r));
    for slot in &mut sym.root.layers {
        let LayerSlot::Own(l) = slot else { continue };
        let l = Arc::make_mut(l);
        let cel = Arc::make_mut(&mut l.cels[0]);
        match l.kind {
            LayerKind::Normal => cel.elements.push(Arc::new(rect.clone())),
            LayerKind::Web => cel.elements.extend(area.clone().map(Arc::new)),
        }
    }
    let mut inst = Element::new_instance(&mut ids, sid, Matrix3::translation(50.0, 30.0));
    if let fp_model::ElementKind::Instance(i) = &mut inst.kind {
        i.button.alt = Some("Instanztext".into());
    }
    let page_slice = web_slice.map(|r| Element::new_slice(&mut ids, r));
    d.ids = ids;
    d.symbols.push(Arc::new(sym));
    let p = d.page_mut(0).expect("Seite");
    for slot in &mut p.root.layers {
        let LayerSlot::Own(l) = slot else { continue };
        let l = Arc::make_mut(l);
        let cel = Arc::make_mut(&mut l.cels[0]);
        match l.kind {
            LayerKind::Normal => cel.elements.push(Arc::new(inst.clone())),
            LayerKind::Web => cel.elements.extend(page_slice.clone().map(Arc::new)),
        }
    }
    d
}

fn html(d: &Document, bounds: Option<BoundsFn>) -> String {
    let mut o = DocExportOptions::new("knopf");
    o.style = FilesStyle::HtmlAndImages;
    o.bounds = bounds;
    let files = export_document(d, &SimpleRenderer, &o, &EncodeExtras::default()).expect("Export");
    let h = files.iter().find(|f| f.path.ends_with(".htm") || f.path.ends_with(".html")).expect("HTML-Datei");
    String::from_utf8(h.bytes.clone()).expect("UTF-8")
}

#[test]
fn button_instance_is_a_slice_with_instance_bounds() {
    let d = doc(None, None);
    let s = collect_slices(&d, 0);
    assert_eq!(s.len(), 1, "eine Schaltfläche = ein Segment");
    assert_eq!(s[0].rect, [50.0, 30.0, 90.0, 50.0]);
    // Instanz überschreibt Alt, Link kommt vom Symbol
    assert_eq!(s[0].url, "https://example.org/");
    assert_eq!(s[0].alt, "Instanztext");
    let h = html(&d, None);
    assert!(h.contains("href=\"https://example.org/\""), "Link im HTML:\n{h}");
    assert!(h.contains("alt=\"Instanztext\""), "Alt-Text im HTML:\n{h}");
    // Tabelle: Spalten 50/40/110, Zeilen 30/20/50 (Segment in der Mitte)
    for w in ["width=\"50\"", "width=\"40\"", "width=\"110\"", "height=\"30\"", "height=\"20\"", "height=\"50\""] {
        assert!(h.contains(w), "{w} fehlt:\n{h}");
    }
}

#[test]
fn active_area_of_the_symbol_wins() {
    // aktiver Bereich −5,−5–45,25 im Symbol → 45,25–95,55 auf der Seite (BA-072)
    let d = doc(Some(Rect::new(-5.0, -5.0, 45.0, 25.0)), None);
    let inst = match &d.pages[0].root.layers[0] {
        LayerSlot::Own(l) => Arc::clone(&l.cels[0].elements[0]),
        _ => unreachable!(),
    };
    assert_eq!(button_active_area(&d, &inst), Some(Rect::new(45.0, 25.0, 95.0, 55.0)));
    let s = collect_slices(&d, 0);
    assert_eq!(s[0].rect, [45.0, 25.0, 95.0, 55.0]);
    let h = html(&d, None);
    for w in ["width=\"45\"", "width=\"50\"", "width=\"105\"", "height=\"25\"", "height=\"30\"", "height=\"45\""] {
        assert!(h.contains(w), "{w} fehlt:\n{h}");
    }
}

#[test]
fn web_slices_first_buttons_get_following_ids_and_bounds_callback() {
    let d = doc(None, Some(Rect::new(150.0, 0.0, 200.0, 20.0)));
    let s = collect_slices(&d, 0);
    assert_eq!(s.len(), 2);
    assert_eq!(s[0].rect, [150.0, 0.0, 200.0, 20.0], "Segment der Web-Ebene zuerst");
    assert!(s[1].id > s[0].id, "Schaltfläche mit eigener Kennung");
    // Grenzen von außen (Engine) haben Vorrang vor der Näherung
    fn fixed(_: &Document, _: &Element) -> Option<Rect> {
        Some(Rect::new(0.0, 0.0, 10.0, 10.0))
    }
    let s = fp_export::slices::collect_slices_with(&d, 0, fixed);
    assert_eq!(s[1].rect, [0.0, 0.0, 10.0, 10.0]);
    let h = html(&d, Some(fixed));
    assert!(h.contains("width=\"10\""), "Grenzen aus dem Rückruf:\n{h}");
}

#[test]
fn hidden_web_layer_or_hidden_instance_has_no_button_slice() {
    let mut d = doc(None, None);
    let p = d.page_mut(0).expect("Seite");
    for slot in &mut p.root.layers {
        if let LayerSlot::Own(l) = slot
            && l.kind == LayerKind::Web
        {
            Arc::make_mut(l).visibility = vec![false];
        }
    }
    assert!(collect_slices(&d, 0).is_empty(), "ausgeblendete Web-Ebene: keine Schaltflächen-Segmente");
    let mut d = doc(None, None);
    let p = d.page_mut(0).expect("Seite");
    if let LayerSlot::Own(l) = &mut p.root.layers[0] {
        let l = Arc::make_mut(l);
        let cel = Arc::make_mut(&mut l.cels[0]);
        Arc::make_mut(&mut cel.elements[0]).common.visible = false;
    }
    assert!(collect_slices(&d, 0).is_empty(), "ausgeblendete Instanz");
}

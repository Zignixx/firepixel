//! Befehle mit Wirkung: Dokument vorher/nachher.

use crate::common;
use common::*;
use fp_engine::{EngineError, Session};
use fp_geom::Rect;
use fp_model::{ElementId, ElementKind};
use serde_json::json;

fn kind_of(s: &Session, id: ElementId) -> &'static str {
    el(s, id).js_type()
}

// ------------------------------------------------------------------ Zeichnen

#[test]
fn draw_shapes() {
    let mut s = doc_session(300.0, 200.0);
    let a = rect(&mut s, 10.0, 10.0, 60.0, 40.0);
    assert!(rect_approx(bounds(&s, a), Rect::new(10.0, 10.0, 60.0, 40.0)));
    assert_eq!(kind_of(&s, a), "RectanglePrimitive");
    let b = ellipse(&mut s, 100.0, 20.0, 160.0, 80.0);
    assert!(rect_approx(bounds(&s, b), Rect::new(100.0, 20.0, 160.0, 80.0)));
    let v = ex(&mut s, "draw.polygon", json!({"center": [50, 150], "corner": [70, 150], "sides": 6}));
    let p = ElementId(v["id"].as_u64().unwrap_or(0));
    if let ElementKind::Path(pe) = &el(&s, p).kind {
        assert_eq!(pe.contours[0].nodes.len(), 6);
    }
    let v = ex(&mut s, "draw.line", json!({"from": [0, 0], "to": [30, 40]}));
    let l = ElementId(v["id"].as_u64().unwrap_or(0));
    assert!(rect_approx(bounds(&s, l), Rect::new(0.0, 0.0, 30.0, 40.0)));
    let v = ex(&mut s, "draw.path", json!({"contours": [{"closed": true, "nodes": [[0, 0], [10, 0], {"anchor": [10, 10], "in": [12, 5], "out": [8, 12]}]}]}));
    assert!(v["id"].as_u64().is_some());
    let v = ex(&mut s, "draw.text", json!({"text": "Hallo", "anchor": [20, 100], "size": 20}));
    let t = ElementId(v["id"].as_u64().unwrap_or(0));
    assert_eq!(kind_of(&s, t), "Text");
    assert_eq!(top_elements(&s).len(), 6);
    // neues Objekt ist ausgewählt und liegt oben
    assert_eq!(s.active().map(|d| d.selection.elements.clone()), Some(vec![t]));
    assert_eq!(top_elements(&s)[0].id, t);
    let v = ex(&mut s, "web.addHotspot", json!({"shape": "circle", "rect": [1, 2, 11, 12], "url": "x.html"}));
    let h = ElementId(v["id"].as_u64().unwrap_or(0));
    assert_eq!(kind_of(&s, h), "Hotspot");
    let v = ex(&mut s, "web.addSlice", json!({"shape": "polygon", "points": [[0, 0], [10, 0], [5, 8]]}));
    assert_eq!(kind_of(&s, ElementId(v["id"].as_u64().unwrap_or(0))), "SliceHotspot");
}

// ------------------------------------------------------------------ Gruppieren, Anordnen, Ausrichten

#[test]
fn group_ungroup_arrange() {
    let mut s = doc_session(300.0, 200.0);
    let a = rect(&mut s, 0.0, 0.0, 10.0, 10.0);
    let b = rect(&mut s, 20.0, 0.0, 30.0, 10.0);
    let c = rect(&mut s, 40.0, 0.0, 50.0, 10.0);
    let order = |s: &Session| top_elements(s).iter().map(|e| e.id).collect::<Vec<_>>();
    assert_eq!(order(&s), vec![c, b, a]);
    select(&mut s, &[a]);
    ex(&mut s, "modify.arrange.bringToFront", json!({}));
    assert_eq!(order(&s), vec![a, c, b]);
    ex(&mut s, "modify.arrange.sendBackward", json!({}));
    assert_eq!(order(&s), vec![c, a, b]);
    ex(&mut s, "modify.arrange.sendToBack", json!({}));
    assert_eq!(order(&s), vec![c, b, a]);
    ex(&mut s, "modify.arrange.bringForward", json!({}));
    assert_eq!(order(&s), vec![c, a, b]);
    select(&mut s, &[a, b]);
    let g = ElementId(ex(&mut s, "modify.group", json!({}))["id"].as_u64().unwrap_or(0));
    assert_eq!(order(&s), vec![c, g]);
    assert_eq!(el(&s, g).children().len(), 2);
    assert!(rect_approx(bounds(&s, g), Rect::new(0.0, 0.0, 30.0, 10.0)));
    ex(&mut s, "modify.ungroup", json!({}));
    assert_eq!(order(&s), vec![c, a, b]);
    assert_eq!(s.active().map(|d| d.selection.elements.len()), Some(2));
}

#[test]
fn align_and_distribute() {
    let mut s = doc_session(300.0, 200.0);
    let a = rect(&mut s, 10.0, 10.0, 20.0, 20.0);
    let b = rect(&mut s, 50.0, 30.0, 70.0, 60.0);
    let c = rect(&mut s, 100.0, 5.0, 110.0, 15.0);
    select(&mut s, &[a, b, c]);
    ex(&mut s, "modify.align.left", json!({}));
    for id in [a, b, c] {
        assert!(approx(bounds(&s, id).left, 10.0));
    }
    ex(&mut s, "modify.align.bottom", json!({}));
    for id in [a, b, c] {
        assert!(approx(bounds(&s, id).bottom, 60.0));
    }
    ex(&mut s, "modify.align.centerHorizontal", json!({}));
    let cy = bounds(&s, b).center().y;
    assert!(approx(bounds(&s, a).center().y, cy));
    // Verteilen: Abstände gleich
    let mut s = doc_session(300.0, 200.0);
    let a = rect(&mut s, 0.0, 0.0, 10.0, 10.0);
    let b = rect(&mut s, 15.0, 0.0, 25.0, 10.0);
    let c = rect(&mut s, 90.0, 0.0, 100.0, 10.0);
    select(&mut s, &[a, b, c]);
    ex(&mut s, "modify.align.distributeWidths", json!({}));
    let g1 = bounds(&s, b).left - bounds(&s, a).right;
    let g2 = bounds(&s, c).left - bounds(&s, b).right;
    assert!(approx(g1, g2) && approx(g1, 35.0));
    // ein Objekt: Menü wie im Original grau (an der Leinwand ausrichten: Bedienfeld pi.align)
    select(&mut s, &[a]);
    assert!(!s.is_available("modify.align.right"));
}

// ------------------------------------------------------------------ Transformieren

#[test]
fn transforms() {
    let mut s = doc_session(300.0, 200.0);
    let a = rect(&mut s, 10.0, 10.0, 50.0, 30.0);
    ex(&mut s, "modify.transform.flipHorizontal", json!({}));
    assert!(rect_approx(bounds(&s, a), Rect::new(10.0, 10.0, 50.0, 30.0)));
    ex(&mut s, "modify.transform.rotate90DegreeCw", json!({}));
    let b = bounds(&s, a);
    assert!(approx(b.width(), 20.0) && approx(b.height(), 40.0));
    assert!(approx(b.center().x, 30.0) && approx(b.center().y, 20.0));
    ex(&mut s, "modify.transform.rotate90DegreeCcw", json!({}));
    ex(&mut s, "modify.transform.rotate180Degree", json!({}));
    assert!(rect_approx(bounds(&s, a), Rect::new(10.0, 10.0, 50.0, 30.0)));
    ex(&mut s, "modify.transform.numericTransform", json!({"mode": "scale", "scaleX": 200}));
    assert!(rect_approx(bounds(&s, a), Rect::new(-10.0, 0.0, 70.0, 40.0)));
    ex(&mut s, "modify.transform.numericTransform", json!({"mode": "resize", "width": 40, "height": 20, "constrain": false}));
    assert!(rect_approx(bounds(&s, a), Rect::new(10.0, 10.0, 50.0, 30.0)));
    ex(&mut s, "transform.matrix", json!({"matrix": [1, 0, 0, 1, 5, 7]}));
    assert!(rect_approx(bounds(&s, a), Rect::new(15.0, 17.0, 55.0, 37.0)));
    ex(&mut s, "element.setBounds", json!({"x": 0, "y": 0, "width": 100, "height": 50}));
    assert!(rect_approx(bounds(&s, a), Rect::new(0.0, 0.0, 100.0, 50.0)));
    ex(&mut s, "transform.distort", json!({"rect": [0, 0, 100, 50], "quad": [[0, 0], [100, 0], [120, 60], [0, 50]]}));
    assert!(approx(bounds(&s, a).right, 120.0));
    ex(&mut s, "modify.transform.freeTransform", json!({}));
    assert_eq!(s.tools.active, fp_engine::ToolId::Scale);
}

#[test]
fn snap_to_pixel_like_original() {
    // Laufzeit 14: 10,3/20,7 – 60,7/51,2 → 11/21 – 61/51; −3,5/2,5 – 7,5/13,49 → −3/2 – 8/13
    let mut s = doc_session(300.0, 200.0);
    let a = rect(&mut s, 10.3, 20.7, 60.7, 51.2);
    ex(&mut s, "modify.snapToPixel", json!({}));
    assert!(rect_approx(bounds(&s, a), Rect::new(11.0, 21.0, 61.0, 51.0)), "{:?}", bounds(&s, a));
    let b = rect(&mut s, -3.5, 2.5, 7.5, 13.49);
    ex(&mut s, "modify.snapToPixel", json!({}));
    assert!(rect_approx(bounds(&s, b), Rect::new(-3.0, 2.0, 8.0, 13.0)), "{:?}", bounds(&s, b));
    // offene Linie bleibt unverändert
    let v = ex(&mut s, "draw.line", json!({"from": [5.25, 7.75], "to": [40.5, 30.5]}));
    let l = ElementId(v["id"].as_u64().unwrap_or(0));
    let before = bounds(&s, l);
    ex(&mut s, "modify.snapToPixel", json!({}));
    assert_eq!(bounds(&s, l), before);
}

// ------------------------------------------------------------------ Bearbeiten

#[test]
fn clipboard_duplicate_clone_move() {
    let mut s = doc_session(300.0, 200.0);
    let a = rect(&mut s, 10.0, 10.0, 20.0, 20.0);
    ex(&mut s, "edit.duplicate", json!({}));
    let d = s.active().map(|d| d.selection.elements[0]).unwrap_or_default();
    assert_ne!(d, a);
    assert!(rect_approx(bounds(&s, d), Rect::new(20.0, 20.0, 30.0, 30.0)));
    ex(&mut s, "edit.clone", json!({}));
    let c = s.active().map(|d| d.selection.elements[0]).unwrap_or_default();
    assert!(rect_approx(bounds(&s, c), bounds(&s, d)));
    assert_eq!(top_elements(&s).len(), 3);
    ex(&mut s, "edit.copy", json!({}));
    ex(&mut s, "edit.clear", json!({}));
    assert_eq!(top_elements(&s).len(), 2);
    ex(&mut s, "edit.paste", json!({}));
    assert_eq!(top_elements(&s).len(), 3);
    ex(&mut s, "edit.cut", json!({}));
    assert_eq!(top_elements(&s).len(), 2);
    select(&mut s, &[a]);
    ex(&mut s, "edit.nudge.right", json!({}));
    ex(&mut s, "edit.nudge.bigDown", json!({}));
    assert!(rect_approx(bounds(&s, a), Rect::new(11.0, 20.0, 21.0, 30.0)));
    ex(&mut s, "edit.move", json!({"dx": -1, "dy": -10, "copy": true}));
    assert_eq!(top_elements(&s).len(), 3);
    assert!(rect_approx(bounds(&s, a), Rect::new(11.0, 20.0, 21.0, 30.0)));
    // Attribute einfügen
    select(&mut s, &[a]);
    ex(&mut s, "element.setOpacity", json!({"opacity": 50}));
    ex(&mut s, "edit.copy", json!({}));
    let b = rect(&mut s, 100.0, 100.0, 120.0, 120.0);
    ex(&mut s, "edit.pasteAttributes", json!({}));
    assert_eq!(el(&s, b).common.opacity.0, 128);
}

#[test]
fn paste_inside_and_as_mask() {
    let mut s = doc_session(300.0, 200.0);
    let a = rect(&mut s, 0.0, 0.0, 50.0, 50.0);
    let b = ellipse(&mut s, 10.0, 10.0, 40.0, 40.0);
    select(&mut s, &[a]);
    ex(&mut s, "edit.copy", json!({}));
    select(&mut s, &[b]);
    let v = ex(&mut s, "edit.pasteInside", json!({}));
    // wie im Original danach nichts ausgewählt (M1 Q-14); die ID kommt aus dem Ergebnis
    assert!(s.active().is_some_and(|d| d.selection.elements.is_empty()));
    let g = fp_model::ElementId(v["id"].as_u64().unwrap_or(0));
    match &el(&s, g).kind {
        ElementKind::Group(gr) => {
            assert!(gr.kind.is_mask());
            assert_eq!(gr.mask.as_ref().map(|m| m.id), Some(b));
            assert_eq!(gr.children.len(), 1);
        }
        _ => panic!("keine Maskengruppe"),
    }
    let c = rect(&mut s, 100.0, 100.0, 150.0, 150.0);
    select(&mut s, &[c]);
    ex(&mut s, "edit.pasteAsMask", json!({}));
    // T-6 (re 28 S5, re 31 §5.2): danach ist die Maske gewählt, nicht mehr die Maskengruppe
    let m = s.active().map(|d| d.selection.elements[0]).unwrap_or_default();
    let d = s.active().expect("d");
    let g2 = fp_engine::docops::mask_owner(&d.doc, d.page, m).expect("gewählt ist die Maske eines Elements");
    assert!(matches!(&g2.kind, ElementKind::Group(gr) if gr.kind.is_mask() && gr.children[0].id == c));
}

#[test]
fn insert_objects() {
    let mut s = doc_session(300.0, 200.0);
    let a = rect(&mut s, 10.0, 10.0, 50.0, 30.0);
    ex(&mut s, "edit.insert.hotspot", json!({}));
    let h = s.active().map(|d| d.selection.elements[0]).unwrap_or_default();
    assert!(rect_approx(bounds(&s, h), Rect::new(10.0, 10.0, 50.0, 30.0)));
    select(&mut s, &[a]);
    ex(&mut s, "edit.insert.slice", json!({}));
    select(&mut s, &[a]);
    ex(&mut s, "edit.insert.polygonSlice", json!({}));
    let ps = s.active().map(|d| d.selection.elements[0]).unwrap_or_default();
    assert!(matches!(&el(&s, ps).kind, ElementKind::WebObject(w) if w.polygon.is_some() && w.slice.is_some()));
    let n_layers = s.active().map(|d| fp_engine::docops::own_layers(&d.doc, d.page).len()).unwrap_or(0);
    ex(&mut s, "edit.insert.layer", json!({"name": "Neu"}));
    assert_eq!(s.active().map(|d| fp_engine::docops::own_layers(&d.doc, d.page).len()), Some(n_layers + 1));
    ex(&mut s, "edit.insert.state", json!({"count": 2}));
    assert_eq!(s.active().map(|d| (d.frame_count(), d.frame)), Some((3, 1)));
    ex(&mut s, "edit.insert.page", json!({}));
    assert_eq!(s.active().map(|d| (d.doc.pages.len(), d.page)), Some((2, 1)));
    ex(&mut s, "edit.insert.emptyBitmap", json!({}));
    assert!(s.active().is_some_and(|d| d.bitmap_mode));
    ex(&mut s, "modify.exitBitmapMode", json!({}));
    select(&mut s, &[]);
    ex(&mut s, "edit.insert.newSymbol", json!({"name": "Leer"}));
    assert_eq!(s.active().map(|d| d.doc.symbols.len()), Some(1));
    ex(&mut s, "edit.insert.newButton", json!({}));
    assert_eq!(s.active().map(|d| d.doc.symbols[1].kind()), Some(fp_model::SymbolKind::Button));
}

// ------------------------------------------------------------------ Leinwand

#[test]
fn canvas_commands() {
    let mut s = doc_session(300.0, 200.0);
    // keep objects that end up outside the canvas ("delete objects when cropping" off, TW-134)
    s.prefs.set("CropDocCropsImages", fp_engine::PrefValue::Bool(false));
    let a = rect(&mut s, 10.0, 10.0, 50.0, 30.0);
    let canvas = |s: &Session| s.active().map(|d| (d.doc.pages[d.page].canvas.width, d.doc.pages[d.page].canvas.height)).unwrap_or_default();
    ex(&mut s, "modify.canvas.canvasSize", json!({"width": 400, "height": 300, "anchor": "topLeft"}));
    assert_eq!(canvas(&s), (400.0, 300.0));
    assert!(rect_approx(bounds(&s, a), Rect::new(10.0, 10.0, 50.0, 30.0)));
    ex(&mut s, "modify.canvas.canvasSize", json!({"width": 200, "height": 100}));
    assert!(rect_approx(bounds(&s, a), Rect::new(-90.0, -90.0, -50.0, -70.0)));
    ex(&mut s, "modify.canvas.trimCanvas", json!({}));
    assert_eq!(canvas(&s), (40.0, 20.0));
    assert!(rect_approx(bounds(&s, a), Rect::new(0.0, 0.0, 40.0, 20.0)));
    ex(&mut s, "modify.canvas.imageSize", json!({"percent": 50}));
    assert_eq!(canvas(&s), (20.0, 10.0));
    assert!(rect_approx(bounds(&s, a), Rect::new(0.0, 0.0, 20.0, 10.0)));
    ex(&mut s, "modify.canvas.rotate90DegreeCw", json!({}));
    assert_eq!(canvas(&s), (10.0, 20.0));
    assert!(rect_approx(bounds(&s, a), Rect::new(0.0, 0.0, 10.0, 20.0)));
    ex(&mut s, "modify.canvas.rotate90DegreeCcw", json!({}));
    ex(&mut s, "modify.canvas.rotate180Degree", json!({}));
    assert_eq!(canvas(&s), (20.0, 10.0));
    ex(&mut s, "modify.canvas.canvasColor", json!({"color": "#ff0000"}));
    assert_eq!(s.active().map(|d| d.doc.pages[0].root.background.0), Some(0xffff_0000));
    rect(&mut s, 30.0, 30.0, 40.0, 40.0);
    ex(&mut s, "modify.canvas.fitCanvas", json!({}));
    assert_eq!(canvas(&s), (40.0, 40.0));
    ex(&mut s, "edit.cropDocument", json!({"rect": [0, 0, 10, 10]}));
    assert_eq!(canvas(&s), (10.0, 10.0));
}

// ------------------------------------------------------------------ Auswählen

#[test]
fn select_commands() {
    let mut s = doc_session(300.0, 200.0);
    let a = rect(&mut s, 10.0, 10.0, 20.0, 20.0);
    let b = rect(&mut s, 30.0, 10.0, 40.0, 20.0);
    let c = ellipse(&mut s, 100.0, 100.0, 120.0, 120.0);
    let sel = |s: &Session| s.active().map(|d| d.selection.elements.clone()).unwrap_or_default();
    ex(&mut s, "select.all", json!({}));
    assert_eq!(sel(&s).len(), 3);
    ex(&mut s, "select.none", json!({}));
    assert!(sel(&s).is_empty());
    select(&mut s, &[a]);
    // without a pixel selection: warning, object selection unchanged (TW-133)
    assert!(s.execute("select.inverse", json!({})).is_err());
    assert_eq!(sel(&s), vec![a]);
    select(&mut s, &[a]);
    ex(&mut s, "select.similar", json!({}));
    assert_eq!(sel(&s).len(), 2);
    ex(&mut s, "select.inRect", json!({"rect": [0, 0, 50, 50]}));
    assert_eq!(sel(&s).len(), 2);
    ex(&mut s, "select.pointAt", json!({"point": [110, 110]}));
    assert_eq!(sel(&s), vec![c]);
    ex(&mut s, "select.next", json!({}));
    assert_eq!(sel(&s).len(), 1);
    select(&mut s, &[a, b]);
    ex(&mut s, "modify.group", json!({}));
    ex(&mut s, "select.subselect", json!({}));
    assert_eq!(sel(&s).len(), 2);
    ex(&mut s, "select.superselect", json!({}));
    assert_eq!(sel(&s).len(), 1);
    assert!(matches!(el(&s, sel(&s)[0]).kind, ElementKind::Group(_)));
}

#[test]
fn pixel_selection_commands() {
    let mut s = doc_session(300.0, 200.0);
    let has = |s: &Session| s.active().is_some_and(|d| d.selection.has_pixels());
    let pb = |s: &Session| s.active().and_then(|d| d.selection.pixels.as_ref().map(|m| m.tight_bounds())).unwrap_or_default();
    ex(&mut s, "select.pixels", json!({"shape": "rect", "rect": [10, 10, 30, 30]}));
    assert_eq!(pb(&s), fp_geom::IRect::new(10, 10, 30, 30));
    ex(&mut s, "select.pixels", json!({"shape": "ellipse", "rect": [20, 20, 40, 40], "mode": "add", "antialias": true}));
    assert_eq!(pb(&s), fp_geom::IRect::new(10, 10, 40, 40));
    ex(&mut s, "select.pixels", json!({"shape": "rect", "rect": [10, 10, 30, 30]}));
    ex(&mut s, "select.expandMarquee", json!({"amount": 2}));
    assert!(pb(&s).left < 10);
    ex(&mut s, "select.pixels", json!({"shape": "rect", "rect": [10, 10, 30, 30]}));
    ex(&mut s, "select.contractMarquee", json!({"amount": 2}));
    assert!(pb(&s).left > 10);
    ex(&mut s, "select.borderMarquee", json!({"amount": 2}));
    ex(&mut s, "select.smoothMarquee", json!({"amount": 2}));
    ex(&mut s, "select.pixels", json!({"shape": "polygon", "points": [[0, 0], [20, 0], [0, 20]]}));
    ex(&mut s, "select.feather", json!({"radius": 3}));
    assert!(has(&s));
    ex(&mut s, "select.pixels", json!({"shape": "rect", "rect": [10, 10, 30, 30]}));
    ex(&mut s, "select.saveBitmapSelection", json!({"name": "A"}));
    ex(&mut s, "select.inverse", json!({}));
    assert_eq!(pb(&s), fp_geom::IRect::new(0, 0, 300, 200));
    ex(&mut s, "select.restoreBitmapSelection", json!({"name": "A"}));
    assert_eq!(pb(&s), fp_geom::IRect::new(10, 10, 30, 30));
    ex(&mut s, "modify.selectiveJpeg.saveSelectionAsJpegMask", json!({}));
    ex(&mut s, "select.convertMarqueeToPath", json!({}));
    let p = s.active().map(|d| d.selection.elements[0]).unwrap_or_default();
    assert!(rect_approx(bounds(&s, p), Rect::new(10.0, 10.0, 30.0, 30.0)));
    assert!(!has(&s));
    ex(&mut s, "modify.selectiveJpeg.restoreJpegMaskAsSelection", json!({}));
    assert!(has(&s));
    ex(&mut s, "modify.selectiveJpeg.removeJpegMask", json!({}));
    ex(&mut s, "select.deleteBitmapSelection", json!({"name": "A"}));
    assert!(s.execute("select.restoreBitmapSelection", json!({"name": "A"})).is_err());
    select(&mut s, &[p]);
    ex(&mut s, "modify.convertPathToMarquee", json!({}));
    assert!(has(&s));
    assert_eq!(top_elements(&s).len(), 0, "DeletePaths = true");
}

// ------------------------------------------------------------------ Bitmaps und Filter

fn px(s: &Session, id: ElementId, x: u32, y: u32) -> [u8; 4] {
    match &el(s, id).kind {
        ElementKind::Bitmap(b) => b.image.pixel(x, y).map(|c| [c.r, c.g, c.b, c.a]).unwrap_or([0; 4]),
        _ => [0; 4],
    }
}

#[test]
fn bitmap_tools_and_filters() {
    let mut s = doc_session(300.0, 200.0);
    let b = bitmap(&mut s, 10, 10, 20, 20, [255, 0, 0, 255]);
    assert_eq!(px(&s, b, 0, 0), [255, 0, 0, 255]);
    ex(&mut s, "filter.adjustColor.invert", json!({}));
    assert_eq!(px(&s, b, 0, 0), [0, 255, 255, 255]);
    ex(&mut s, "filter.repeat", json!({}));
    assert_eq!(px(&s, b, 0, 0), [255, 0, 0, 255]);
    ex(&mut s, "filter.adjustColor.hueSaturation", json!({"lightness": -100}));
    assert_eq!(&px(&s, b, 5, 5)[..3], &[0, 0, 0]);
    // Malen mit Strichfarbe
    ex(&mut s, "color.set", json!({"stroke": "#00ff00", "fill": "#0000ff"}));
    ex(&mut s, "bitmap.paint", json!({"points": [[12, 12], [20, 12]], "size": 3, "antialias": false}));
    assert_eq!(px(&s, b, 5, 2), [0, 255, 0, 255]);
    ex(&mut s, "bitmap.floodFill", json!({"point": [25, 25], "tolerance": 0}));
    assert_eq!(px(&s, b, 15, 15), [0, 0, 255, 255]);
    ex(&mut s, "bitmap.erase", json!({"points": [[11.5, 25.5]], "size": 2}));
    assert_eq!(px(&s, b, 1, 15)[3], 0);
    ex(&mut s, "bitmap.gradient", json!({"from": [10, 10], "to": [30, 10], "colors": ["#000000", "#ffffff"]}));
    // ohne Pixelauswahl wächst die Bitmap auf die Leinwand (PAR 07/07)
    let l = px(&s, b, 10, 15);
    let r = px(&s, b, 29, 15);
    assert!(l[0] < 20 && r[0] > 230, "{l:?} {r:?}");
    // Zauberstab, Bitmap durch Kopieren, Zuschneiden
    ex(&mut s, "select.magicWand", json!({"point": [10, 15], "tolerance": 255}));
    assert!(s.active().is_some_and(|d| d.selection.has_pixels()));
    ex(&mut s, "select.pixels", json!({"shape": "rect", "rect": [10, 10, 15, 15]}));
    select(&mut s, &[b]);
    ex(&mut s, "edit.insert.bitmapViaCopy", json!({}));
    let nb = s.active().map(|d| d.selection.elements[0]).unwrap_or_default();
    assert!(rect_approx(bounds(&s, nb), Rect::new(10.0, 10.0, 15.0, 15.0)));
    ex(&mut s, "select.pixels", json!({"shape": "rect", "rect": [20, 20, 25, 25]}));
    select(&mut s, &[b]);
    ex(&mut s, "edit.cropSelectedBitmap", json!({}));
    assert!(rect_approx(bounds(&s, b), Rect::new(20.0, 20.0, 25.0, 25.0)));
    ex(&mut s, "filter.blur.gaussian", json!({"radius": 2}));
    assert!(bounds(&s, b).width() > 5.0, "Gauß vergrößert die Grenzen");
    // Live-Filter auf Vektor
    let a = rect(&mut s, 100.0, 100.0, 120.0, 120.0);
    ex(&mut s, "filter.blur.gaussian", json!({"radius": 3}));
    let e = el(&s, a);
    assert_eq!(e.common.effects.as_ref().map(|l| l.effects.len()), Some(1));
    // Kopie in PRIv/EFLv für Rechteck-Grundformen
    if let ElementKind::Path(p) = &e.kind {
        assert_eq!(p.primitive.as_ref().and_then(|x| x.effects.as_ref()).map(|l| l.effects.len()), Some(1));
    }
}

#[test]
fn flatten_and_merge_bitmaps() {
    let mut s = doc_session(100.0, 100.0);
    let a = bitmap(&mut s, 0, 0, 10, 10, [255, 0, 0, 255]);
    let b = bitmap(&mut s, 5, 5, 10, 10, [0, 0, 255, 255]);
    select(&mut s, &[a, b]);
    let v = ex(&mut s, "modify.flattenSelection", json!({}));
    let f = ElementId(v["id"].as_u64().unwrap_or(0));
    assert!(rect_approx(bounds(&s, f), Rect::new(0.0, 0.0, 15.0, 15.0)));
    assert_eq!(px(&s, f, 7, 7), [0, 0, 255, 255]);
    assert_eq!(px(&s, f, 1, 1), [255, 0, 0, 255]);
    assert_eq!(top_elements(&s).len(), 1);
    let c = bitmap(&mut s, 20, 20, 5, 5, [0, 255, 0, 255]);
    select(&mut s, &[c]);
    ex(&mut s, "modify.mergeDown", json!({}));
    assert_eq!(top_elements(&s).len(), 1);
    // Vektoren brauchen einen Renderer
    rect(&mut s, 0.0, 0.0, 5.0, 5.0);
    let r = s.execute("modify.flattenSelection", json!({}));
    #[cfg(feature = "render")]
    assert!(r.is_ok(), "{r:?}");
    #[cfg(not(feature = "render"))]
    assert!(matches!(r, Err(EngineError::BackendMissing(_))));
    s.set_renderer(None);
    rect(&mut s, 0.0, 0.0, 5.0, 5.0);
    assert!(matches!(s.execute("modify.flattenSelection", json!({})), Err(EngineError::BackendMissing(_))));
}

// ------------------------------------------------------------------ Masken und Symbole

#[test]
fn masks() {
    let mut s = doc_session(200.0, 200.0);
    let a = rect(&mut s, 0.0, 0.0, 50.0, 50.0);
    let b = ellipse(&mut s, 10.0, 10.0, 40.0, 40.0);
    select(&mut s, &[a, b]);
    let g = ElementId(ex(&mut s, "modify.mask.groupAsMask", json!({}))["id"].as_u64().unwrap_or(0));
    assert!(matches!(&el(&s, g).kind, ElementKind::Group(gr) if gr.kind == fp_model::GroupKind::VectorMask && gr.mask.as_ref().map(|m| m.id) == Some(b)));
    ex(&mut s, "modify.mask.disableMask", json!({}));
    assert!(matches!(&el(&s, g).kind, ElementKind::Group(gr) if !gr.mask_flags.enabled));
    // RE spec 68 section 8: no check mark, the menu text swaps to "Enable Mask" instead
    assert_eq!(s.command_state("modify.mask.disableMask").1, None);
    ex(&mut s, "modify.mask.deleteMask", json!({}));
    assert_eq!(top_elements(&s).iter().map(|e| e.id).collect::<Vec<_>>(), vec![a]);
    select(&mut s, &[a]);
    ex(&mut s, "modify.mask.revealAll", json!({}));
    let g = s.active().map(|d| d.selection.elements[0]).unwrap_or_default();
    assert!(matches!(&el(&s, g).kind, ElementKind::Group(gr) if gr.kind == fp_model::GroupKind::BitmapMask));
    let c = rect(&mut s, 100.0, 100.0, 150.0, 150.0);
    ex(&mut s, "modify.mask.hideAll", json!({}));
    ex(&mut s, "select.pixels", json!({"shape": "rect", "rect": [100, 100, 120, 120]}));
    let d = rect(&mut s, 60.0, 60.0, 90.0, 90.0);
    ex(&mut s, "select.pixels", json!({"shape": "rect", "rect": [60, 60, 70, 70]}));
    select(&mut s, &[d]);
    ex(&mut s, "modify.mask.revealSelection", json!({}));
    let _ = c;
}

#[test]
fn symbols_and_animation() {
    let mut s = doc_session(200.0, 200.0);
    let a = rect(&mut s, 0.0, 0.0, 20.0, 20.0);
    let i = ElementId(ex(&mut s, "modify.symbol.convertToSymbol", json!({"name": "Knopf"}))["id"].as_u64().unwrap_or(0));
    assert_eq!(kind_of(&s, i), "Instance");
    assert!(rect_approx(bounds(&s, i), Rect::new(0.0, 0.0, 20.0, 20.0)));
    let _ = a;
    ex(&mut s, "edit.duplicate", json!({}));
    let j = s.active().map(|d| d.selection.elements[0]).unwrap_or_default();
    ex(&mut s, "edit.move", json!({"dx": 90, "dy": 0}));
    select(&mut s, &[j, i]);
    let v = ex(&mut s, "modify.symbol.tweenInstances", json!({"steps": 2}));
    assert_eq!(v["ids"].as_array().map(|a| a.len()), Some(2));
    assert_eq!(top_elements(&s).len(), 4);
    // zweites Symbol und Austauschen
    let b = ellipse(&mut s, 50.0, 50.0, 60.0, 60.0);
    select(&mut s, &[b]);
    ex(&mut s, "modify.symbol.convertToSymbol", json!({"name": "Kreis"}));
    select(&mut s, &[i]);
    ex(&mut s, "modify.symbol.swapSymbol", json!({"symbol": "Kreis"}));
    // L-3: Symbolursprung = Mitte des Inhalts (re 28 S2), die Instanz behält ihre Matrix T(10,10)
    // (re 13 §1: Instanzen behalten Matrix und Werte) → „Kreis“ (−5…5) steht mittig auf der
    // Mitte von „Knopf“; vorher lag der Ursprung in Dokumentkoordinaten (50…60)
    assert!(rect_approx(bounds(&s, i), Rect::new(5.0, 5.0, 15.0, 15.0)));
    ex(&mut s, "modify.symbol.breakApart", json!({}));
    let g = s.active().map(|d| d.selection.elements[0]).unwrap_or_default();
    assert_eq!(kind_of(&s, g), "Group");
    // Animation
    let c = rect(&mut s, 0.0, 100.0, 10.0, 110.0);
    select(&mut s, &[c]);
    let v = ex(&mut s, "modify.animation.animateSelection", json!({"frames": 4, "offsetX": 50}));
    let an = ElementId(v["id"].as_u64().unwrap_or(0));
    assert_eq!(s.active().map(|d| d.frame_count()), Some(4));
    ex(&mut s, "modify.animation.settings", json!({"frames": 6}));
    assert_eq!(s.active().map(|d| d.frame_count()), Some(6));
    assert!(matches!(&el(&s, an).kind, ElementKind::Instance(x) if x.anim.frames == 6 && x.anim.offset.x == 50.0));
    ex(&mut s, "modify.animation.removeAnimation", json!({}));
    assert!(matches!(&el(&s, an).kind, ElementKind::Instance(x) if x.anim.frames == 1));
}

// ------------------------------------------------------------------ Pfade

#[test]
fn path_commands() {
    let mut s = doc_session(200.0, 200.0);
    let a = rect(&mut s, 0.0, 0.0, 20.0, 20.0);
    let b = rect(&mut s, 10.0, 0.0, 30.0, 20.0);
    select(&mut s, &[a, b]);
    ex(&mut s, "modify.combinePaths.join", json!({}));
    let j = s.active().map(|d| d.selection.elements[0]).unwrap_or_default();
    assert!(matches!(&el(&s, j).kind, ElementKind::Path(p) if p.contours.len() == 2));
    ex(&mut s, "modify.combinePaths.split", json!({}));
    assert_eq!(top_elements(&s).len(), 2);
    let v = ex(&mut s, "draw.line", json!({"from": [0, 50], "to": [50, 50]}));
    let l = ElementId(v["id"].as_u64().unwrap_or(0));
    ex(&mut s, "modify.combinePaths.join", json!({}));
    assert!(matches!(&el(&s, l).kind, ElementKind::Path(p) if p.contours[0].closed));
    ex(&mut s, "modify.alterPath.featherFill", json!({"amount": 4}));
    ex(&mut s, "modify.alterPath.hardFill", json!({}));
    #[cfg(feature = "vector")]
    {
        let mut s = doc_session(200.0, 200.0);
        let a = rect(&mut s, 0.0, 0.0, 20.0, 20.0);
        let b = rect(&mut s, 10.0, 0.0, 30.0, 20.0);
        select(&mut s, &[a, b]);
        ex(&mut s, "modify.combinePaths.union", json!({}));
        assert_eq!(top_elements(&s).len(), 1);
        let u = top_elements(&s)[0].id;
        assert!(rect_approx(bounds(&s, u), Rect::new(0.0, 0.0, 30.0, 20.0)));
        let c = rect(&mut s, 5.0, 5.0, 15.0, 15.0);
        select(&mut s, &[c, u]);
        ex(&mut s, "modify.combinePaths.punch", json!({}));
        assert_eq!(top_elements(&s).len(), 1);
        let a2 = rect(&mut s, 0.0, 0.0, 20.0, 20.0);
        select(&mut s, &[a2, u]);
        ex(&mut s, "modify.combinePaths.intersect", json!({}));
        let x = top_elements(&s)[0].id;
        assert!(rect_approx(bounds(&s, x), Rect::new(0.0, 0.0, 20.0, 20.0)));
        let k = rect(&mut s, 0.0, 0.0, 10.0, 10.0);
        select(&mut s, &[k, x]);
        ex(&mut s, "modify.combinePaths.crop", json!({}));
        assert!(rect_approx(bounds(&s, x), Rect::new(0.0, 0.0, 10.0, 10.0)));
        select(&mut s, &[x]);
        ex(&mut s, "modify.alterPath.insetPath", json!({"distance": 2}));
        assert!(rect_approx(bounds(&s, x), Rect::new(2.0, 2.0, 8.0, 8.0)), "{:?}", bounds(&s, x));
        ex(&mut s, "modify.alterPath.expandStroke", json!({"width": 2}));
        ex(&mut s, "modify.alterPath.simplify", json!({"amount": 1}));
    }
}

// ------------------------------------------------------------------ Text

#[test]
fn text_commands() {
    let mut s = doc_session(300.0, 200.0);
    let v = ex(&mut s, "draw.text", json!({"text": "Hallo", "anchor": [10, 50], "size": 12}));
    let t = ElementId(v["id"].as_u64().unwrap_or(0));
    let attrs = |s: &Session| match &el(s, t).kind {
        ElementKind::Text(x) => x.resolved_runs()[0].0.clone(),
        _ => Default::default(),
    };
    ex(&mut s, "text.size.36", json!({}));
    assert_eq!(attrs(&s).size, Some(36.0));
    ex(&mut s, "text.size.larger", json!({}));
    assert_eq!(attrs(&s).size, Some(37.0));
    ex(&mut s, "text.size.smaller", json!({}));
    assert_eq!(attrs(&s).size, Some(36.0));
    ex(&mut s, "text.size.other", json!({"size": 17}));
    assert_eq!(attrs(&s).size, Some(17.0));
    ex(&mut s, "text.style.fauxBold", json!({}));
    assert_eq!(attrs(&s).bold, Some(true));
    assert_eq!(s.command_state("text.style.fauxBold").1, Some(true));
    ex(&mut s, "text.style.underline", json!({}));
    ex(&mut s, "text.style.plain", json!({}));
    assert_eq!((attrs(&s).bold, attrs(&s).underline), (Some(false), Some(false)));
    ex(&mut s, "text.align.right", json!({}));
    assert_eq!(attrs(&s).alignment, Some(2));
    ex(&mut s, "text.align.centeredVertically", json!({}));
    assert_eq!(attrs(&s).alignment, Some(0x11));
    ex(&mut s, "text.font", json!({"font": "Verdana"}));
    assert_eq!(attrs(&s).font.as_deref(), Some("Verdana"));
    ex(&mut s, "text.setText", json!({"text": "Welt"}));
    assert!(matches!(&el(&s, t).kind, ElementKind::Text(x) if x.plain_text() == "Welt"));
    ex(&mut s, "text.setAttributes", json!({"tracking": 5, "color": "#ff0000"}));
    assert_eq!(attrs(&s).tracking, Some(5.0));
    let p = ellipse(&mut s, 50.0, 50.0, 150.0, 150.0);
    select(&mut s, &[t, p]);
    ex(&mut s, "text.attachToPath", json!({}));
    assert_eq!(top_elements(&s).len(), 1);
    ex(&mut s, "text.orientation.skewVertical", json!({}));
    assert!(matches!(&el(&s, t).kind, ElementKind::Text(x) if x.on_path.as_ref().and_then(|o| o.mode) == Some(2)));
    ex(&mut s, "text.reverseDirection", json!({}));
    ex(&mut s, "text.detachFromPath", json!({}));
    assert_eq!(top_elements(&s).len(), 2);
}

/// TW-133: Text size larger/smaller step exactly 1 pt (minimum 1 pt); "stretched" cannot be
/// stored and becomes left on point and fixed-width text alike (re 50 section 1).
#[test]
fn text_size_step_and_stretch_becomes_left() {
    let mut s = doc_session(300.0, 200.0);
    let v = ex(&mut s, "draw.text", json!({"text": "Abc 123", "anchor": [10, 50], "size": 24}));
    let t = ElementId(v["id"].as_u64().unwrap_or(0));
    let attrs = |s: &Session, id: ElementId| match &el(s, id).kind {
        ElementKind::Text(x) => x.resolved_runs()[0].0.clone(),
        _ => Default::default(),
    };
    ex(&mut s, "text.size.larger", json!({}));
    assert_eq!(attrs(&s, t).size, Some(25.0));
    ex(&mut s, "text.size.smaller", json!({}));
    assert_eq!(attrs(&s, t).size, Some(24.0));
    ex(&mut s, "text.size.other", json!({"size": 1}));
    ex(&mut s, "text.size.smaller", json!({}));
    assert_eq!(attrs(&s, t).size, Some(1.0), "clamped at 1 pt");

    ex(&mut s, "text.align.right", json!({}));
    ex(&mut s, "text.align.stretched", json!({}));
    assert_eq!(attrs(&s, t).alignment, Some(0), "point text: stretched becomes left");

    let v = ex(&mut s, "draw.text", json!({"text": "Rahmen", "rect": [10, 100, 200, 140], "size": 14}));
    let f = ElementId(v["id"].as_u64().unwrap_or(0));
    assert!(matches!(&el(&s, f).kind, ElementKind::Text(x) if x.frame_type == Some(1)));
    select(&mut s, &[f]);
    ex(&mut s, "text.align.stretched", json!({}));
    assert_eq!(attrs(&s, f).alignment, Some(0), "fixed-width text: stretched becomes left");
}

// ------------------------------------------------------------------ Ebenen, Seiten, Status

#[test]
fn layers_pages_states() {
    let mut s = doc_session(100.0, 100.0);
    let v = ex(&mut s, "layer.add", json!({"name": "Oben"}));
    let l = fp_model::LayerId(v["layer"].as_u64().unwrap_or(0));
    let a = rect(&mut s, 0.0, 0.0, 10.0, 10.0);
    let layer_of = |s: &Session, id: ElementId| s.active().and_then(|d| fp_engine::docops::find_on_page(&d.doc, d.page, id)).map(|p| p.slot);
    let slot = layer_of(&s, a);
    ex(&mut s, "layer.rename", json!({"name": "X"}));
    ex(&mut s, "layer.setVisible", json!({"visible": false}));
    ex(&mut s, "layer.setLocked", json!({"locked": true}));
    let lay = s.active().and_then(|d| d.doc.find_layer(l)).unwrap_or_else(|| panic!("Ebene"));
    assert_eq!(lay.name, "X");
    assert!(!lay.is_visible());
    ex(&mut s, "layer.duplicate", json!({"layer": l.0}));
    ex(&mut s, "layer.move", json!({"layer": l.0, "index": 0}));
    assert_ne!(layer_of(&s, a), slot);
    ex(&mut s, "layer.delete", json!({"layer": l.0}));
    assert!(s.active().and_then(|d| d.doc.find_layer(l)).is_none());
    ex(&mut s, "state.add", json!({"count": 2}));
    assert_eq!(s.active().map(|d| d.frame_count()), Some(3));
    ex(&mut s, "state.setCurrent", json!({"index": 0}));
    rect(&mut s, 0.0, 0.0, 5.0, 5.0);
    ex(&mut s, "state.duplicate", json!({}));
    assert_eq!(s.active().map(|d| (d.frame_count(), d.frame)), Some((4, 1)));
    assert!(!top_elements(&s).is_empty());
    ex(&mut s, "state.delete", json!({}));
    ex(&mut s, "view.state.next", json!({}));
    ex(&mut s, "view.state.previous", json!({}));
    ex(&mut s, "page.add", json!({"name": "Zwei"}));
    ex(&mut s, "page.rename", json!({"name": "Zweite"}));
    ex(&mut s, "page.duplicate", json!({}));
    assert_eq!(s.active().map(|d| d.doc.pages.len()), Some(3));
    ex(&mut s, "view.page.previous", json!({}));
    ex(&mut s, "page.setCurrent", json!({"index": 0}));
    ex(&mut s, "page.delete", json!({"index": 2}));
    assert_eq!(s.active().map(|d| d.doc.pages.len()), Some(2));
    ex(&mut s, "modify.flattenLayers", json!({}));
}

// ------------------------------------------------------------------ Attribute, Ansicht, Farben

#[test]
fn element_attributes() {
    let mut s = doc_session(100.0, 100.0);
    let a = rect(&mut s, 0.0, 0.0, 10.0, 10.0);
    ex(&mut s, "element.setFill", json!({"color": "#123456"}));
    ex(&mut s, "element.setStroke", json!({"color": "#ff0000", "width": 3, "placement": "inside"}));
    ex(&mut s, "element.setBlendMode", json!({"mode": "multiply"}));
    ex(&mut s, "element.setName", json!({"name": "Box"}));
    let e = el(&s, a);
    let pa = fp_engine::docops::path_attrs_of(&e).unwrap_or_default();
    assert_eq!(pa.fill_color.0, 0xff12_3456);
    assert_eq!(pa.brush.as_ref().map(|b| b.diameter), Some(3));
    assert_eq!(e.common.blend, fp_color::BlendMode(2));
    assert_eq!(e.common.name.as_deref(), Some("Box"));
    ex(&mut s, "element.addEffect", json!({"effect": "dropShadow", "params": {"ShadowBlur": "4"}}));
    ex(&mut s, "element.setEffectVisible", json!({"index": 0, "visible": false}));
    assert!(el(&s, a).common.effects.as_ref().is_some_and(|l| !l.effects[0].is_visible()));
    ex(&mut s, "element.removeEffect", json!({"index": 0}));
    assert!(el(&s, a).common.effects.is_none());
    ex(&mut s, "element.setGradient", json!({"from": [0, 0], "to": [10, 0], "colors": ["#000000", "#ffffff"]}));
    assert!(fp_engine::docops::path_attrs_of(&el(&s, a)).and_then(|p| p.fill.as_ref().map(|f| f.shape)) == Some(2));
    ex(&mut s, "element.setFill", json!({"kind": "none"}));
    ex(&mut s, "element.setVisible", json!({"visible": false}));
    ex(&mut s, "view.showAll", json!({}));
    assert!(el(&s, a).common.visible);
    ex(&mut s, "view.hideSelection", json!({}));
    assert!(!el(&s, a).common.visible);
    ex(&mut s, "view.showAll", json!({}));
    select(&mut s, &[a]);
    ex(&mut s, "modify.lockSelection", json!({}));
    assert!(el(&s, a).common.locked);
    ex(&mut s, "element.setLocked", json!({"ids": [a.0], "locked": false}));
}

#[test]
fn view_and_colors() {
    let mut s = doc_session(1000.0, 1000.0);
    ex(&mut s, "view.zoomIn", json!({}));
    assert_eq!(s.active().map(|d| d.view.zoom), Some(1.5));
    ex(&mut s, "view.zoomOut", json!({}));
    ex(&mut s, "view.zoomOut", json!({}));
    assert_eq!(s.active().map(|d| d.view.zoom), Some(0.66));
    ex(&mut s, "view.magnification.400", json!({}));
    assert_eq!(s.command_state("view.magnification.400").1, Some(true));
    ex(&mut s, "view.setViewport", json!({"width": 500, "height": 500}));
    ex(&mut s, "view.fitAll", json!({}));
    // stufenlos mit 15 px Rand wie im Original (UX_ABGLEICH_A Z10): (500 − 30) / 1000
    assert!(s.active().is_some_and(|d| (d.view.zoom - 0.47).abs() < 1e-9));
    ex(&mut s, "view.pan", json!({"dx": 10, "dy": 0}));
    ex(&mut s, "view.grid.showGrid", json!({}));
    assert_eq!(s.command_state("view.grid.showGrid").1, Some(true));
    ex(&mut s, "view.grid.editGrid", json!({"width": 10, "height": 12, "color": "#ff0000"}));
    assert_eq!(s.active().and_then(|d| d.doc.pages[0].root.grid.as_ref().map(|g| (g.size.x, g.size.y))), Some((10.0, 12.0)));
    ex(&mut s, "guide.add", json!({"orientation": "horizontal", "position": 40}));
    ex(&mut s, "guide.add", json!({"orientation": "vertical", "position": 60}));
    ex(&mut s, "guide.move", json!({"orientation": "vertical", "index": 0, "position": 70}));
    assert_eq!(s.active().and_then(|d| d.doc.pages[0].root.guides.as_ref().map(|g| g.vertical.clone())), Some(vec![70.0]));
    ex(&mut s, "view.guides.lockGuides", json!({}));
    assert!(s.execute("guide.add", json!({"orientation": "vertical", "position": 1})).is_err());
    ex(&mut s, "view.guides.clearGuides", json!({}));
    ex(&mut s, "color.set", json!({"stroke": "#111111", "fill": "#222222"}));
    ex(&mut s, "color.swap", json!({}));
    assert_eq!((s.tools.stroke_color.0, s.tools.fill_color.0), (0xff22_2222, 0xff11_1111));
    ex(&mut s, "color.defaults", json!({}));
    assert_eq!((s.tools.stroke_color.0, s.tools.fill_color.0), (0xff00_0000, 0xffff_ffff));
    ex(&mut s, "color.none", json!({}));
    ex(&mut s, "tool.select", json!({"tool": "Stift"}));
    assert_eq!(s.tools.active, fp_engine::ToolId::Pen);
    let b = bitmap(&mut s, 0, 0, 4, 4, [1, 2, 3, 255]);
    let _ = b;
    let v = ex(&mut s, "color.sample", json!({"point": [1, 1], "samples": 3}));
    assert_eq!(v["color"], "#010203");
    assert_eq!(s.tools.fill_color.0, 0xff01_0203);
    ex(&mut s, "window.layers", json!({}));
    assert_eq!(s.command_state("window.layers").1, Some(false));
    ex(&mut s, "window.hidePanels", json!({}));
    ex(&mut s, "edit.keyboardShortcuts", json!({"set": "Photoshop"}));
    assert_eq!(s.prefs.shortcut_set(), "Photoshop");
    ex(&mut s, "edit.preferences", json!({"key": "MaxUndos", "value": 7, "theme": "oled"}));
    assert_eq!(s.prefs.max_undos(), 7);
    assert_eq!(s.prefs.theme, fp_engine::Theme::Oled);
}

// ------------------------------------------------------------------ Datei

#[test]
fn file_commands() {
    let dir = std::env::temp_dir().join(format!("fp-engine-test-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("a.fw.png");
    let mut s = doc_session(120.0, 80.0);
    rect(&mut s, 10.0, 10.0, 30.0, 30.0);
    assert!(s.execute("file.save", json!({})).is_err(), "ohne Namen");
    ex(&mut s, "file.saveAs", json!({"path": path.to_string_lossy()}));
    assert!(!s.active().is_some_and(|d| d.is_dirty()));
    rect(&mut s, 50.0, 10.0, 60.0, 30.0);
    assert!(s.active().is_some_and(|d| d.is_dirty()));
    assert!(s.execute("file.close", json!({})).is_err(), "ungespeichert");
    ex(&mut s, "file.revert", json!({}));
    assert_eq!(top_elements(&s).len(), 1);
    ex(&mut s, "file.close", json!({}));
    assert!(s.active().is_none());
    ex(&mut s, "file.open", json!({"path": path.to_string_lossy()}));
    assert_eq!(top_elements(&s).len(), 1);
    assert_eq!(s.prefs.recent_files.first(), Some(&path));
    ex(&mut s, "file.openRecent", json!({"index": 0}));
    assert_eq!(s.documents().len(), 2);
    let info = ex(&mut s, "file.fileInfo", json!({}));
    assert_eq!(info["pages"], 1);
    ex(&mut s, "file.saveAll", json!({}));
    ex(&mut s, "file.saveAsTemplate", json!({"path": dir.join("t.fw.png").to_string_lossy()}));
    ex(&mut s, "file.newFromTemplate", json!({"path": dir.join("t.fw.png").to_string_lossy()}));
    assert!(s.active().is_some_and(|d| d.path.is_none()));
    ex(&mut s, "file.closeAll", json!({"discard": true}));
    assert!(s.documents().is_empty());
    ex(&mut s, "file.exit", json!({}));
    assert!(s.ui.quit_requested);
    let _ = std::fs::remove_dir_all(&dir);
}

// Der frühere Rauchtest über das ganze Register ist in `tests\all_commands.rs` aufgegangen (Wirkung,
// Verlauf und Datei je Befehl).

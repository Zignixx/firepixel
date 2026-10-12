//! Gemeinsame Hilfen der Tests.
#![allow(dead_code)]

use fp_engine::{Session, ToolEvent};
use fp_geom::Rect;
use fp_model::{Element, ElementId};
use serde_json::{json, Value};
use std::sync::Arc;

/// Sitzung mit einem neuen Dokument (weißer Hintergrund).
pub fn doc_session(w: f32, h: f32) -> Session {
    fp_engine::shell::disable();
    fp_i18n::profile::use_test_profile(); // nie ins echte Nutzerprofil (EB-3)
    let mut s = Session::default();
    ex(&mut s, "file.new", json!({"width": w, "height": h, "background": "#ffffff"}));
    s
}

/// Führt einen Befehl aus und bricht mit Meldung ab, wenn er scheitert.
pub fn ex(s: &mut Session, name: &str, p: Value) -> Value {
    match s.execute(name, p.clone()) {
        Ok(o) => o.value,
        Err(e) => panic!("Befehl {name} {p} scheiterte: {e}"),
    }
}

/// Rechteck zeichnen, liefert die ID.
pub fn rect(s: &mut Session, l: f32, t: f32, r: f32, b: f32) -> ElementId {
    let v = ex(s, "draw.rectangle", json!({"rect": [l, t, r, b]}));
    ElementId(v["id"].as_u64().unwrap_or(0))
}

/// Ellipse zeichnen.
pub fn ellipse(s: &mut Session, l: f32, t: f32, r: f32, b: f32) -> ElementId {
    let v = ex(s, "draw.ellipse", json!({"rect": [l, t, r, b]}));
    ElementId(v["id"].as_u64().unwrap_or(0))
}

/// Bitmap aus einer Farbe einfügen (über Einfügen aus Pixeln der Zwischenablage).
pub fn bitmap(s: &mut Session, l: i32, t: i32, w: i32, h: i32, rgba: [u8; 4]) -> ElementId {
    let img = fp_raster::RgbaImage::filled(fp_geom::IRect::from_xywh(l, t, w, h), fp_color::Rgba8::new(rgba[0], rgba[1], rgba[2], rgba[3])).unwrap_or_else(|_| fp_raster::RgbaImage::empty_at(0, 0));
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    let v = ex(s, "edit.paste", json!({}));
    ElementId(v["ids"][0].as_u64().unwrap_or(0))
}

/// Oberste Elemente der aktuellen Seite im aktuellen Status, oben zuerst (alle Ebenen).
pub fn top_elements(s: &Session) -> Vec<Arc<Element>> {
    let d = s.active().expect("Dokument");
    let mut out = Vec::new();
    for (_, l) in fp_engine::docops::own_layers(&d.doc, d.page).into_iter().rev() {
        let ci = if l.shared_across_frames { 0 } else { d.frame };
        if let Some(c) = l.cels.get(ci) {
            out.extend(c.elements.iter().cloned());
        }
    }
    out
}

/// Element per ID.
pub fn el(s: &Session, id: ElementId) -> Arc<Element> {
    let d = s.active().expect("Dokument");
    fp_engine::docops::element_on_page(&d.doc, d.page, id).unwrap_or_else(|| panic!("Element {} fehlt", id.0))
}

/// Geometrische Grenzen eines Elements; bei Texten der gespeicherte Satzrahmen ohne den
/// angezeigten 2-px-Textrand (`docops::TEXT_MARGIN`, EB-1). Angezeigte Grenzen: [`visible_bounds`].
pub fn bounds(s: &Session, id: ElementId) -> Rect {
    let e = el(s, id);
    let b = visible_bounds(s, id);
    match &e.kind {
        fp_model::element::ElementKind::Text(t) if t.transform.is_none_or(|m| m.is_identity()) => {
            let m = fp_engine::docops::TEXT_MARGIN;
            Rect::new(b.left + m, b.top + m, b.right - m, b.bottom - m)
        }
        _ => b,
    }
}

/// Angezeigte Grenzen wie Inspektor/Info (`docops::element_bounds`, Texte mit 2 px Rand).
pub fn visible_bounds(s: &Session, id: ElementId) -> Rect {
    let d = s.active().expect("Dokument");
    fp_engine::docops::element_bounds(&d.doc, &el(s, id)).unwrap_or_default()
}

/// Auswahl setzen.
pub fn select(s: &mut Session, ids: &[ElementId]) {
    ex(s, "select.set", json!({"ids": ids.iter().map(|i| i.0).collect::<Vec<_>>()}));
}

/// Ereignisfolge an das aktive Werkzeug.
pub fn events(s: &mut Session, evs: &[ToolEvent]) {
    for e in evs {
        if let Err(x) = s.tool_event(e) {
            panic!("Werkzeugereignis {e:?} scheiterte: {x}");
        }
    }
}

/// Näherungsgleich.
pub fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

/// Rechteck näherungsgleich.
pub fn rect_approx(a: Rect, b: Rect) -> bool {
    approx(a.left, b.left) && approx(a.top, b.top) && approx(a.right, b.right) && approx(a.bottom, b.bottom)
}

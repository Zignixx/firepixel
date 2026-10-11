//! Menü „Bearbeiten“: Rückgängig/Wiederholen, Zwischenablage, Löschen, Duplizieren/Klonen,
//! Objekt einfügen, Zuschneiden, Verschieben (Pfeiltasten).

use super::{ids_json, insert_new, selected_elements, sel_err, take_ids, with_doc, DOC, SEL};
use crate::command::{cmd, CommandOutput, CommandRegistry, ParamKind, Requirement};
use crate::docops::{self, bitmap_image, set_bitmap_image};
use crate::error::{EngineError, Result};
use crate::selection::ClipAttributes;
use crate::session::{DocumentState, Session};
use fp_geom::{IRect, Point};
use fp_model::element::{Element, ElementKind};
use fp_model::layer::{Layer, LayerKind, LayerSlot};
use fp_model::symbol::{Symbol, SymbolKind};
use fp_model::{ElementId, Matrix3};
use fp_raster::{Mask, RgbaImage};
use serde_json::json;
use std::sync::Arc;

/// Ausgewählte Bitmaps (Adresse, Element).
pub(crate) fn selected_bitmaps(d: &DocumentState) -> Vec<(fp_model::traverse::ElementPath, Arc<Element>)> {
    selected_elements(d).into_iter().filter(|(_, e)| matches!(e.kind, ElementKind::Bitmap(_))).collect()
}

/// Pixel einer Bitmap innerhalb der Maske (Alpha · Maske), beschnitten auf die Maskengrenzen.
pub(crate) fn masked_pixels(img: &RgbaImage, mask: &Mask) -> RgbaImage {
    let r = img.bounds().intersect(&mask.tight_bounds());
    let mut out = img.cropped(r);
    let (l, t) = out.origin();
    let w = out.width();
    for (i, px) in out.data_mut().iter_mut().enumerate() {
        let (x, y) = (l + (i % w.max(1)) as i32, t + (i / w.max(1)) as i32);
        let m = mask.get(x, y) as u32;
        px[3] = ((px[3] as u32 * m + 127) / 255) as u8;
    }
    out
}

/// Löscht die Pixel unter der Maske (Alpha · (255 − Maske)).
pub(crate) fn clear_masked(img: &mut RgbaImage, mask: &Mask) {
    let (l, t) = img.origin();
    let w = img.width().max(1);
    for (i, px) in img.data_mut().iter_mut().enumerate() {
        let (x, y) = (l + (i % w) as i32, t + (i / w) as i32);
        let m = mask.get(x, y) as u32;
        if m > 0 {
            px[3] = ((px[3] as u32 * (255 - m) + 127) / 255) as u8;
        }
    }
}

fn copy_to_clipboard(s: &mut Session, vectors_only: bool) -> Result<CommandOutput> {
    let d = s.doc()?;
    let res = d.doc.pages.get(d.page).map(|p| p.canvas.resolution);
    if !vectors_only
        && let Some(mask) = d.selection.pixels.as_ref().filter(|_| d.selection.has_pixels())
        && let Some((_, e)) = selected_bitmaps(d).first()
        && let ElementKind::Bitmap(b) = &e.kind
        && let Some(img) = bitmap_image(b)
    {
        let px = masked_pixels(&img, mask);
        s.clipboard.clear();
        s.clipboard.pixels = Some(px);
        s.clipboard.source_resolution = res;
        return Ok(CommandOutput::value(json!({"pixels": true})));
    }
    let els: Vec<Arc<Element>> = selected_elements(d)
        .into_iter()
        .map(|(_, e)| e)
        .filter(|e| !vectors_only || !matches!(e.kind, ElementKind::Bitmap(_)))
        .collect();
    if els.is_empty() {
        return Err(sel_err("nichts zu kopieren"));
    }
    let attrs = els.first().map(|e| ClipAttributes { common: e.common.clone(), path: docops::path_attrs_of(e) });
    s.clipboard.clear();
    s.clipboard.elements = els;
    s.clipboard.attributes = attrs;
    s.clipboard.source_resolution = res;
    Ok(CommandOutput::value(json!({"elements": s.clipboard.elements.len()})))
}

/// Entfernt die Auswahl bzw. die ausgewählten Pixel.
pub(crate) fn clear_selection(d: &mut DocumentState) -> Result<()> {
    let page = d.page;
    if d.selection.has_pixels() {
        let bms = selected_bitmaps(d);
        if !bms.is_empty() {
            let mask = d.selection.pixels.clone().unwrap_or_default();
            let doc = Arc::make_mut(&mut d.doc);
            for (path, _) in bms {
                docops::edit_at(doc, &path, |e| {
                    if let ElementKind::Bitmap(b) = &mut e.kind
                        && let Some(mut img) = bitmap_image(b)
                    {
                        clear_masked(&mut img, &mask);
                        set_bitmap_image(b, &img);
                    }
                });
            }
            return Ok(());
        }
    }
    let ids = d.selection.elements.clone();
    if ids.is_empty() {
        return Err(sel_err("nichts ausgewählt"));
    }
    let doc = Arc::make_mut(&mut d.doc);
    docops::remove_ids(doc, page, &ids);
    d.selection.clear();
    Ok(())
}

/// Fügt Kopien von Elementen oben in die aktive Ebene ein (neue IDs), wählt sie aus.
pub(crate) fn paste_elements(d: &mut DocumentState, els: &[Arc<Element>], offset: (f32, f32)) -> Result<Vec<ElementId>> {
    let (page, frame, layer) = (d.page, d.frame, d.layer);
    let doc = Arc::make_mut(&mut d.doc);
    let mut ids = Vec::new();
    for (i, e) in els.iter().enumerate() {
        let mut c = docops::reassign(doc, e);
        docops::translate_element(&mut c, offset.0, offset.1);
        ids.push(c.id);
        docops::insert_element(doc, page, frame, layer, i, Arc::new(c)).ok_or_else(|| EngineError::Failed("Einfügen fehlgeschlagen".into()))?;
    }
    d.selection.set(ids.clone());
    Ok(ids)
}

/// Einfügeversatz wie im Original (gemessen, `docs\UX_ABGLEICH_A.md` C02/C03): Liegt der Inhalt
/// (Vereinigung der Grenzen) ganz außerhalb des sichtbaren Teils der Leinwand (Sichtbereich ∩
/// Leinwand, Ränder zählen als innen), wird er auf dessen Mitte zentriert, sonst bleibt er an seiner
/// Stelle. Beispiele: Objekt 400,300–500,380 in ein 300×200-Dokument → 100,60–200,140; bei 800 %
/// mit Blick auf die Mitte landet ein Objekt von 10,10 mittig um 300,200.
pub(crate) fn paste_offset(d: &DocumentState, bounds: Option<fp_geom::Rect>) -> (f32, f32) {
    let Some(b) = bounds.filter(|b| b.left.is_finite() && b.right >= b.left && b.bottom >= b.top) else { return (0.0, 0.0) };
    let canvas = super::canvas_rect(&d.doc, d.page);
    let vis = d.view.visible_rect();
    let region = fp_geom::Rect::new(canvas.left.max(vis.left), canvas.top.max(vis.top), canvas.right.min(vis.right), canvas.bottom.min(vis.bottom));
    let region = if region.right > region.left && region.bottom > region.top { region } else { canvas };
    let touches = b.left <= region.right && b.right >= region.left && b.top <= region.bottom && b.bottom >= region.top;
    if touches {
        return (0.0, 0.0);
    }
    let (cx, cy) = ((region.left + region.right) / 2.0, (region.top + region.bottom) / 2.0);
    let (bw, bh) = (b.right - b.left, b.bottom - b.top);
    ((cx - bw / 2.0).round() - b.left, (cy - bh / 2.0).round() - b.top)
}

/// Neue Bitmap aus Pixeln oben in der aktiven Ebene.
pub(crate) fn insert_bitmap(d: &mut DocumentState, img: &RgbaImage) -> Result<ElementId> {
    insert_bitmap_at(d, img, (0, 0))
}

/// Neue Bitmap aus Pixeln, um `offset` verschoben.
pub(crate) fn insert_bitmap_at(d: &mut DocumentState, img: &RgbaImage, offset: (i32, i32)) -> Result<ElementId> {
    let r = img.bounds();
    let doc = Arc::make_mut(&mut d.doc);
    let mut ids = take_ids(doc);
    let mut next = docops::tile_counter();
    let e = Element::new_bitmap(&mut ids, Point::new((r.left + offset.0) as f32, (r.top + offset.1) as f32), img.width() as u32, img.height() as u32, img.as_bytes(), &mut next);
    doc.ids = ids;
    insert_new(d, e)
}

fn duplicate(s: &mut Session, offset: (f32, f32)) -> Result<CommandOutput> {
    with_doc(s, |d| {
        let els: Vec<_> = selected_elements(d);
        if els.is_empty() {
            return Err(sel_err("nichts ausgewählt"));
        }
        let page = d.page;
        let doc = Arc::make_mut(&mut d.doc);
        let mut new_ids = Vec::new();
        // je Element direkt darüber einfügen, unten beginnen, damit Indizes gültig bleiben
        for (path, e) in els.iter().rev() {
            let mut c = docops::reassign(doc, e);
            docops::translate_element(&mut c, offset.0, offset.1);
            let id = c.id;
            if let Some(cur) = docops::find_on_page(doc, page, e.id)
                && let Some((list, idx)) = docops::parent_list_mut(doc, &cur)
            {
                list.insert(idx, Arc::new(c));
                new_ids.push(id);
            } else {
                let _ = path;
            }
        }
        new_ids.reverse();
        d.selection.set(new_ids.clone());
        Ok(CommandOutput::doc_with(json!({"ids": ids_json(&new_ids)})))
    })
}

/// Verschiebt die Auswahl (Pfeiltasten, Zeiger-Ziehen; `copy` = mit Alt eine Kopie ziehen).
pub(crate) fn move_selection(s: &mut Session, dx: f32, dy: f32, copy: bool) -> Result<CommandOutput> {
    if copy {
        duplicate(s, (dx, dy))?;
        return Ok(CommandOutput::doc().labeled("Kopie verschieben"));
    }
    with_doc(s, |d| {
        let page = d.page;
        let ids = d.selection.elements.clone();
        let doc = Arc::make_mut(&mut d.doc);
        for id in &ids {
            docops::edit_by_id(doc, page, *id, |e| docops::translate_element(e, dx, dy));
        }
        if let Some(m) = &mut d.selection.pixels
            && ids.is_empty()
        {
            // Auswahlrahmen allein verschieben
            let b = m.bounds().translate(dx.round() as i32, dy.round() as i32);
            if let Ok(nm) = Mask::from_raw(b, m.data().to_vec()) {
                *m = nm;
            }
        }
        Ok(CommandOutput::doc().labeled("Verschieben"))
    })
}

/// Symbol aus Elementen (Koordinaten bleiben, Instanz mit Einheitsmatrix).
pub(crate) fn make_symbol(d: &mut DocumentState, name: &str, kind: SymbolKind, ids: &[ElementId]) -> Result<ElementId> {
    let page = d.page;
    let paths = docops::paths_sorted(&d.doc, page, ids);
    let insert_at = paths.first().cloned();
    let doc = Arc::make_mut(&mut d.doc);
    let removed = docops::remove_ids(doc, page, ids);
    let sid = doc.new_symbol_id();
    let mut idg = take_ids(doc);
    let mut sym = Symbol::new(&mut idg, name, sid, kind);
    if let Some(LayerSlot::Own(l)) = sym.root.layers.first_mut() {
        let l = Arc::make_mut(l);
        if let Some(c) = l.cels.first_mut() {
            Arc::make_mut(c).elements = removed.iter().map(|(_, e)| Arc::clone(e)).collect();
        }
    }
    let inst = Element::new_instance(&mut idg, sid, Matrix3::IDENTITY);
    doc.ids = idg;
    doc.symbols.push(Arc::new(sym));
    let iid = inst.id;
    match insert_at {
        Some(p) if !removed.is_empty() => {
            let mut cel_path = p.clone();
            cel_path.steps.truncate(1);
            let idx = match cel_path.steps.first() {
                Some(fp_model::PathStep::Child(i)) => *i,
                _ => 0,
            };
            let cel = docops::cel_mut(doc, page, p.slot, p.cel).ok_or_else(|| EngineError::Failed("Ebene".into()))?;
            let i = idx.min(cel.elements.len());
            cel.elements.insert(i, Arc::new(inst));
        }
        _ => {
            let (frame, layer) = (d.frame, d.layer);
            let doc = Arc::make_mut(&mut d.doc);
            docops::insert_element(doc, page, frame, layer, 0, Arc::new(inst));
        }
    }
    d.selection.set(vec![iid]);
    Ok(iid)
}

fn web_from_selection(s: &mut Session, slice: bool, polygon: bool) -> Result<CommandOutput> {
    with_doc(s, |d| {
        let page = d.page;
        let b = docops::ids_bounds(&d.doc, page, &d.selection.elements).ok_or_else(|| sel_err("keine Grenzen"))?;
        let poly = if polygon {
            selected_elements(d).first().and_then(|(_, e)| match &e.kind {
                ElementKind::Path(pe) if pe.contours.len() == 1 => {
                    Some(crate::geometry::polygon_contour(&crate::geometry::flatten(&pe.contours[0]), true))
                }
                _ => None,
            })
        } else {
            None
        };
        let doc = Arc::make_mut(&mut d.doc);
        let mut idg = take_ids(doc);
        let mut e = if slice { Element::new_slice(&mut idg, b) } else { Element::new_hotspot(&mut idg, b) };
        doc.ids = idg;
        if polygon && let ElementKind::WebObject(w) = &mut e.kind {
            w.shape = fp_model::WebShape::Polygon;
            w.polygon = Some(poly.unwrap_or_else(|| crate::geometry::rect_contour(b, 0.0)));
        }
        let id = insert_new(d, e)?;
        Ok(CommandOutput::doc_with(json!({"id": id.0})))
    })
}

/// Setzt die Leinwand der aktuellen Seite auf `r` (alte Dokumentkoordinaten): alle Elemente und
/// Hilfslinien werden so verschoben, dass `r` bei der Leinwand-Ursprungsecke liegt.
pub(crate) fn crop_page(d: &mut DocumentState, r: IRect) -> Result<()> {
    if r.is_empty() {
        return Err(EngineError::Failed("leerer Bereich".into()));
    }
    let page = d.page;
    let doc = Arc::make_mut(&mut d.doc);
    let p = doc.page_mut(page).ok_or_else(|| EngineError::NotFound("Seite".into()))?;
    let (ox, oy) = (p.canvas.origin.x, p.canvas.origin.y);
    let (dx, dy) = (ox - r.left as f32, oy - r.top as f32);
    p.canvas.width = r.width() as f32;
    p.canvas.height = r.height() as f32;
    if let Some(g) = &mut p.root.guides {
        for v in &mut g.vertical {
            *v += dx;
        }
        for h in &mut g.horizontal {
            *h += dy;
        }
    }
    shift_page(doc, page, dx, dy);
    Ok(())
}

/// Verschiebt alle Elemente einer Seite (alle Ebenen und Status).
pub(crate) fn shift_page(doc: &mut fp_model::Document, page: usize, dx: f32, dy: f32) {
    if dx == 0.0 && dy == 0.0 {
        return;
    }
    let m = Matrix3::translation(dx, dy);
    transform_page(doc, page, &m, fp_raster::Sampler::Nearest);
}

/// Transformiert alle Elemente einer Seite (alle Ebenen und Status).
pub(crate) fn transform_page(doc: &mut fp_model::Document, page: usize, m: &Matrix3, sampler: fp_raster::Sampler) {
    let Some(p) = doc.page_mut(page) else { return };
    for slot in &mut p.root.layers {
        if let LayerSlot::Own(l) = slot {
            let l = Arc::make_mut(l);
            for cel in &mut l.cels {
                for e in &mut Arc::make_mut(cel).elements {
                    docops::transform_element(Arc::make_mut(e), m, sampler);
                }
            }
        }
    }
}

pub(super) fn register(r: &mut CommandRegistry) {
    r.add(cmd("edit.undo").requires(&[Requirement::Document, Requirement::CanUndo]).run(|s, _| {
        s.undo()?;
        Ok(CommandOutput::session())
    }));
    r.add(cmd("edit.redo").requires(&[Requirement::Document, Requirement::CanRedo]).run(|s, _| {
        s.redo()?;
        Ok(CommandOutput::session())
    }));
    r.add(cmd("edit.copy").requires(&[Requirement::Document, Requirement::SelectionOrPixels]).run(|s, _| copy_to_clipboard(s, false)));
    r.add(cmd("edit.copyAsVectors").requires(SEL).run(|s, _| copy_to_clipboard(s, true)));
    r.add(cmd("edit.cut").requires(&[Requirement::Document, Requirement::SelectionOrPixels]).run(|s, _| {
        copy_to_clipboard(s, false)?;
        with_doc(s, clear_selection)?;
        Ok(CommandOutput::doc())
    }));
    r.add(cmd("edit.clear").requires(&[Requirement::Document, Requirement::SelectionOrPixels]).run(|s, _| {
        with_doc(s, clear_selection)?;
        Ok(CommandOutput::doc())
    }));
    r.add(cmd("edit.paste").requires(&[Requirement::Document, Requirement::Clipboard]).run(|s, _| {
        let clip = s.clipboard.clone();
        with_doc(s, |d| {
            if let Some(px) = &clip.pixels {
                let b = px.bounds();
                let r = fp_geom::Rect::new(b.left as f32, b.top as f32, b.right as f32, b.bottom as f32);
                let (dx, dy) = paste_offset(d, Some(r));
                let id = insert_bitmap_at(d, px, (dx.round() as i32, dy.round() as i32))?;
                return Ok(CommandOutput::doc_with(json!({"ids": [id.0]})));
            }
            let union = clip.elements.iter().filter_map(|e| docops::element_bounds(&d.doc, e)).reduce(|a, b| a.union(&b));
            let off = paste_offset(d, union);
            let ids = paste_elements(d, &clip.elements, off)?;
            Ok(CommandOutput::doc_with(json!({"ids": ids_json(&ids)})))
        })
    }));
    r.add(cmd("edit.pasteAsMask").requires(&[Requirement::Document, Requirement::Selection, Requirement::Clipboard]).run(|s, _| {
        let clip = s.clipboard.clone();
        let mask_el = clip.elements.first().cloned().ok_or_else(|| sel_err("Zwischenablage enthält kein Objekt"))?;
        with_doc(s, |d| {
            let target = d.selection.elements.first().copied().ok_or_else(|| sel_err("kein Objekt"))?;
            let page = d.page;
            let doc = Arc::make_mut(&mut d.doc);
            let mask = Arc::new(docops::reassign(doc, &mask_el));
            let vector = !matches!(mask.kind, ElementKind::Bitmap(_));
            let path = docops::find_on_page(doc, page, target).ok_or(EngineError::ElementNotFound(target.0))?;
            let (list, idx) = docops::parent_list_mut(doc, &path).ok_or_else(|| EngineError::Failed("Elternliste".into()))?;
            let el = list.remove(idx);
            let mut idg = std::mem::take(&mut doc.ids);
            let g = Element::new_mask_group(&mut idg, mask, vec![el], vector);
            doc.ids = idg;
            let gid = g.id;
            if let Some((list, _)) = docops::parent_list_mut(doc, &path) {
                list.insert(idx.min(list.len()), Arc::new(g));
            }
            d.selection.set(vec![gid]);
            Ok(CommandOutput::doc())
        })
    }));
    r.add(cmd("edit.pasteInside").requires(&[Requirement::Document, Requirement::Selection, Requirement::Clipboard]).run(|s, _| {
        let clip = s.clipboard.clone();
        if clip.elements.is_empty() {
            return Err(sel_err("Zwischenablage enthält kein Objekt"));
        }
        with_doc(s, |d| {
            let target = d.selection.elements.first().copied().ok_or_else(|| sel_err("kein Objekt"))?;
            let page = d.page;
            let doc = Arc::make_mut(&mut d.doc);
            let path = docops::find_on_page(doc, page, target).ok_or(EngineError::ElementNotFound(target.0))?;
            let children: Vec<Arc<Element>> = clip.elements.iter().map(|e| Arc::new(docops::reassign(doc, e))).collect();
            let (list, idx) = docops::parent_list_mut(doc, &path).ok_or_else(|| EngineError::Failed("Elternliste".into()))?;
            let mask = list.remove(idx);
            let vector = !matches!(mask.kind, ElementKind::Bitmap(_));
            let mut idg = std::mem::take(&mut doc.ids);
            let g = Element::new_mask_group(&mut idg, mask, children, vector);
            doc.ids = idg;
            let gid = g.id;
            if let Some((list, _)) = docops::parent_list_mut(doc, &path) {
                list.insert(idx.min(list.len()), Arc::new(g));
            }
            d.selection.set(vec![gid]);
            Ok(CommandOutput::doc())
        })
    }));
    r.add(cmd("edit.pasteAttributes").requires(&[Requirement::Document, Requirement::Selection, Requirement::Clipboard]).run(|s, _| {
        let attrs = s.clipboard.attributes.clone().ok_or_else(|| sel_err("keine Attribute in der Zwischenablage"))?;
        with_doc(s, |d| {
            let (page, ids) = (d.page, d.selection.elements.clone());
            let doc = Arc::make_mut(&mut d.doc);
            for id in ids {
                docops::edit_by_id(doc, page, id, |e| {
                    e.common.opacity = attrs.common.opacity;
                    e.common.blend = attrs.common.blend;
                    super::draw::set_effects(e, attrs.common.effects.clone());
                    e.common.style = attrs.common.style.clone();
                    if let Some(pa) = &attrs.path {
                        let pa = Arc::clone(pa);
                        docops::edit_path_attrs(e, &move |a| *a = (*pa).clone());
                    }
                });
            }
            Ok(CommandOutput::doc())
        })
    }));
    r.add(cmd("edit.duplicate").requires(SEL).run(|s, _| duplicate(s, (10.0, 10.0))));
    r.add(cmd("edit.clone").requires(SEL).run(|s, _| duplicate(s, (0.0, 0.0))));
    r.add(
        cmd("edit.move")
            .label("Verschieben")
            .requires(&[Requirement::Document])
            .param("dx", ParamKind::Number { min: None, max: None }, "Verschiebung x")
            .param("dy", ParamKind::Number { min: None, max: None }, "Verschiebung y")
            .opt_default("copy", ParamKind::Bool, json!(false), "Kopie verschieben (Alt)")
            .opt("ids", ParamKind::Ids, "Elemente (Vorgabe: Auswahl; die Auswahl bleibt unverändert)")
            .run(|s, p| super::with_targets(s, p, |s, p| move_selection(s, p.f64("dx")? as f32, p.f64("dy")? as f32, p.bool_or("copy", false)?))),
    );
    for (name, dx, dy) in [
        ("edit.nudge.left", -1.0, 0.0),
        ("edit.nudge.right", 1.0, 0.0),
        ("edit.nudge.up", 0.0, -1.0),
        ("edit.nudge.down", 0.0, 1.0),
        ("edit.nudge.bigLeft", -10.0, 0.0),
        ("edit.nudge.bigRight", 10.0, 0.0),
        ("edit.nudge.bigUp", 0.0, -10.0),
        ("edit.nudge.bigDown", 0.0, 10.0),
    ] {
        r.add(cmd(name).requires(&[Requirement::Document, Requirement::SelectionOrPixels]).run(move |s, _| move_selection(s, dx, dy, false)));
    }

    // Objekt einfügen
    r.add(cmd("edit.insert.newSymbol").requires(DOC).opt("name", ParamKind::String, "Symbolname").run(|s, p| {
        let name = p.opt_str("name")?.unwrap_or_else(|| "Symbol".into());
        with_doc(s, |d| {
            let ids = d.selection.elements.clone();
            let id = make_symbol(d, &name, SymbolKind::Graphic, &ids)?;
            Ok(CommandOutput::doc_with(json!({"id": id.0})))
        })
    }));
    r.add(cmd("edit.insert.newButton").requires(DOC).opt("name", ParamKind::String, "Name der Schaltfläche").run(|s, p| {
        let name = p.opt_str("name")?.unwrap_or_else(|| "Schaltfläche".into());
        with_doc(s, |d| {
            let ids = d.selection.elements.clone();
            let id = make_symbol(d, &name, SymbolKind::Button, &ids)?;
            Ok(CommandOutput::doc_with(json!({"id": id.0})))
        })
    }));
    r.add(cmd("edit.insert.hotspot").requires(SEL).run(|s, _| web_from_selection(s, false, false)));
    r.add(cmd("edit.insert.slice").requires(SEL).run(|s, _| web_from_selection(s, true, false)));
    r.add(cmd("edit.insert.polygonSlice").requires(SEL).run(|s, _| web_from_selection(s, true, true)));
    r.add(cmd("edit.insert.emptyBitmap").requires(DOC).run(|s, _| {
        with_doc(s, |d| {
            let img = RgbaImage::empty_at(0, 0);
            let id = insert_bitmap(d, &img)?;
            d.bitmap_mode = true;
            Ok(CommandOutput::doc_with(json!({"id": id.0})))
        })
    }));
    for (name, cut) in [("edit.insert.bitmapViaCopy", false), ("edit.insert.bitmapViaCut", true)] {
        r.add(cmd(name).requires(&[Requirement::Document, Requirement::PixelSelection]).run(move |s, _| {
            with_doc(s, |d| {
                let mask = d.selection.pixels.clone().unwrap_or_default();
                let (path, e) = selected_bitmaps(d).into_iter().next().ok_or_else(|| sel_err("keine Bitmap ausgewählt"))?;
                let ElementKind::Bitmap(b) = &e.kind else { return Err(sel_err("keine Bitmap")) };
                let img = bitmap_image(b).ok_or_else(|| EngineError::Failed("Bitmap".into()))?;
                let px = masked_pixels(&img, &mask);
                if cut {
                    let doc = Arc::make_mut(&mut d.doc);
                    docops::edit_at(doc, &path, |e| {
                        if let ElementKind::Bitmap(b) = &mut e.kind {
                            let mut im = img.clone();
                            clear_masked(&mut im, &mask);
                            set_bitmap_image(b, &im);
                        }
                    });
                }
                d.selection.pixels = None;
                let id = insert_bitmap(d, &px)?;
                Ok(CommandOutput::doc_with(json!({"id": id.0})))
            })
        }));
    }
    r.add(cmd("edit.insert.layer").requires(DOC).opt("name", ParamKind::String, "Ebenenname").run(|s, p| {
        let name = p.opt_str("name")?;
        with_doc(s, |d| {
            let id = add_layer(d, name.as_deref(), LayerKind::Normal)?;
            Ok(CommandOutput::doc_with(json!({"layer": id.0})))
        })
    }));
    r.add(
        cmd("edit.insert.state")
            .requires(DOC)
            .opt_default("count", ParamKind::Integer { min: Some(1), max: Some(1000) }, json!(1), "Anzahl")
            .run(|s, p| {
                let n = p.i64_or("count", 1)?.max(1) as usize;
                with_doc(s, |d| {
                    let (page, at) = (d.page, d.frame + 1);
                    docops::insert_frames(Arc::make_mut(&mut d.doc), page, at, n).ok_or_else(|| EngineError::NotFound("Seite".into()))?;
                    d.frame = at;
                    d.selection.clear();
                    Ok(CommandOutput::doc_with(json!({"state": at})))
                })
            }),
    );
    r.add(cmd("edit.insert.page").requires(DOC).opt("name", ParamKind::String, "Seitenname").run(|s, p| {
        let name = p.opt_str("name")?;
        with_doc(s, |d| {
            let n = d.doc.pages.len() + 1;
            let name = name.unwrap_or_else(|| format!("Seite {n}"));
            let doc = Arc::make_mut(&mut d.doc);
            let at = doc.add_page(&name);
            d.page = at;
            d.frame = 0;
            d.layer = docops::top_normal_layer(&d.doc, at);
            d.selection.clear();
            Ok(CommandOutput::doc_with(json!({"page": at})))
        })
    }));
    r.add(cmd("edit.findAndReplace").checked(|s| s.ui.panels.get("edit.findAndReplace").copied().unwrap_or(false)).run(|s, _| {
        let v = !s.ui.panels.get("edit.findAndReplace").copied().unwrap_or(false);
        s.ui.panels.insert("edit.findAndReplace".into(), v);
        s.emit(crate::session::SessionEvent::PanelsChanged);
        Ok(CommandOutput::session())
    }));

    // Zuschneiden
    r.add(cmd("edit.cropSelectedBitmap").requires(&[Requirement::Document, Requirement::AnyOf(crate::command::ObjectKind::Bitmap)]).run(|s, _| {
        with_doc(s, |d| {
            let mask = d.selection.pixels.clone();
            let bms = selected_bitmaps(d);
            let doc = Arc::make_mut(&mut d.doc);
            for (path, _) in bms {
                docops::edit_at(doc, &path, |e| {
                    if let ElementKind::Bitmap(b) = &mut e.kind
                        && let Some(img) = bitmap_image(b)
                    {
                        let r = match &mask {
                            Some(m) => m.tight_bounds(),
                            None => img.alpha_bounds(),
                        };
                        set_bitmap_image(b, &img.cropped(img.bounds().intersect(&r)));
                    }
                });
            }
            d.selection.pixels = None;
            Ok(CommandOutput::doc())
        })
    }));
    r.add(
        cmd("edit.cropDocument")
            .requires(DOC)
            .opt("rect", ParamKind::Rect, "Zuschnitt (Vorgabe: Pixelauswahl bzw. Grenzen der Auswahl)")
            .run(|s, p| {
                let rect = p.opt_rect("rect")?;
                with_doc(s, |d| {
                    let r = match rect {
                        Some(r) => r.normalized().round_nearest(),
                        None => match &d.selection.pixels {
                            Some(m) if d.selection.has_pixels() => m.tight_bounds(),
                            _ => docops::ids_bounds(&d.doc, d.page, &d.selection.elements).ok_or_else(|| sel_err("kein Zuschneidebereich"))?.round_out(),
                        },
                    };
                    crop_page(d, r)?;
                    d.selection.pixels = None;
                    Ok(CommandOutput::doc())
                })
            }),
    );
}

/// Neue Ebene über der aktiven; wird aktiv.
pub(crate) fn add_layer(d: &mut DocumentState, name: Option<&str>, kind: LayerKind) -> Result<fp_model::LayerId> {
    let page = d.page;
    let cur_slot = d.layer.and_then(|l| docops::layer_slot(&d.doc, page, l));
    let doc = Arc::make_mut(&mut d.doc);
    let frames = doc.pages.get(page).map(|p| p.root.frame_count as usize).unwrap_or(1);
    let count = docops::own_layers(doc, page).len();
    let label = name.map(|s| s.to_string()).unwrap_or_else(|| format!("Ebene {}", count));
    let mut idg = take_ids(doc);
    let l = Layer::new(&mut idg, &label, kind, frames);
    doc.ids = idg;
    let id = l.id;
    let p = doc.page_mut(page).ok_or_else(|| EngineError::NotFound("Seite".into()))?;
    // über der aktiven Ebene, aber unter der Web-Ebene
    let web = p.root.layers.iter().position(|s| matches!(s, LayerSlot::Own(l) if l.kind == LayerKind::Web)).unwrap_or(p.root.layers.len());
    let at = cur_slot.map(|s| s + 1).unwrap_or(web).min(web);
    p.root.layers.insert(at, LayerSlot::Own(Arc::new(l)));
    d.layer = Some(id);
    Ok(id)
}


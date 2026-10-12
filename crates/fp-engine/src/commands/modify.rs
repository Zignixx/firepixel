//! Menü „Modifizieren“: Leinwand, Animation, Symbol, Popupmenü, Maske, selektives JPEG, Sperren,
//! Reduzieren, Transformieren, Anordnen, Ausrichten, Pfade, An Pixeln ausrichten, Gruppieren.

use super::edit::{crop_page, make_symbol, transform_page};
use super::select::mask_to_data;
use super::{canvas_rect, ids_json, selected_elements, selected_paths, sel_err, take_ids, with_doc, DOC, SEL};
use crate::command::{cmd, CommandOutput, CommandRegistry, ObjectKind, ParamKind, Params, Requirement};
use crate::docops::{self, bitmap_image};
use crate::error::{EngineError, Result};
use crate::geometry::{self, PathBoolOp, PathShape};
use crate::session::{DocumentState, Session};
use crate::tools::ToolId;
use fp_geom::{Matrix3, Point, Rect};
use fp_model::element::{Element, ElementKind};
use fp_model::layer::{LayerKind, LayerSlot};
use fp_model::symbol::SymbolKind;
use fp_model::traverse::ElementPath;
use fp_model::{Contour, ElementId, Extra, PathStep, SavedSelection};
use fp_raster::{CompositeOptions, Mask, RgbaImage};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

fn sampler(s: &Session) -> fp_raster::Sampler {
    s.prefs.edit_sampler()
}

/// Transformiert die ausgewählten Elemente mit `m`.
pub(crate) fn transform_selection(s: &mut Session, m: Matrix3) -> Result<CommandOutput> {
    let sm = sampler(s);
    with_doc(s, |d| {
        let (page, ids) = (d.page, d.selection.elements.clone());
        if ids.is_empty() {
            return Err(sel_err("err-sel-nothing"));
        }
        let doc = Arc::make_mut(&mut d.doc);
        for id in ids {
            docops::edit_by_id(doc, page, id, |e| docops::transform_element(e, &m, sm));
        }
        Ok(CommandOutput::doc())
    })
}

fn sel_bounds(s: &Session) -> Result<Rect> {
    let d = s.doc()?;
    docops::ids_bounds(&d.doc, d.page, &d.selection.elements).ok_or_else(|| sel_err("err-sel-no-bounds"))
}

fn about_selection(s: &mut Session, m: Matrix3) -> Result<CommandOutput> {
    let c = sel_bounds(s)?.center();
    transform_selection(s, docops::about(c, &m))
}

fn matrix_from(p: &Params, key: &str) -> Result<Matrix3> {
    let v: Vec<f32> = p.parse(key)?.ok_or_else(|| EngineError::InvalidParams { name: p.command.clone(), message: fp_i18n::tr!("err-param-absent", key = key) })?;
    match v.len() {
        6 => Ok(Matrix3::from_affine(v[0], v[1], v[2], v[3], v[4], v[5])),
        9 => Ok(Matrix3::from_rows([[v[0], v[1], v[2]], [v[3], v[4], v[5]], [v[6], v[7], v[8]]])),
        _ => Err(EngineError::InvalidParams { name: p.command.clone(), message: fp_i18n::t("err-param-matrix") }),
    }
}

// ------------------------------------------------------------------ Anordnen / Gruppieren

/// Ausgewählte Elemente gruppiert nach ihrer Elternliste: Schlüssel = Adresse der Liste.
fn by_parent(paths: &[ElementPath]) -> HashMap<(usize, usize, Vec<PathStep>), Vec<usize>> {
    let mut m: HashMap<(usize, usize, Vec<PathStep>), Vec<usize>> = HashMap::new();
    for p in paths {
        if let Some((PathStep::Child(i), prefix)) = p.steps.split_last() {
            m.entry((p.slot, p.cel, prefix.to_vec())).or_default().push(*i);
        }
    }
    m
}

fn arrange(s: &mut Session, how: &'static str) -> Result<CommandOutput> {
    with_doc(s, |d| {
        let page = d.page;
        let paths = selected_paths(d);
        let groups = by_parent(&paths);
        let doc = Arc::make_mut(&mut d.doc);
        for ((slot, cel, prefix), mut idx) in groups {
            let mut probe = docops::slot_path(doc, page, slot, cel, prefix.clone());
            probe.steps.push(PathStep::Child(0));
            let Some((list, _)) = docops::parent_list_mut(doc, &probe) else { continue };
            idx.sort_unstable();
            let n = list.len();
            match how {
                "front" | "back" => {
                    let sel: Vec<Arc<Element>> = idx.iter().filter_map(|&i| list.get(i).cloned()).collect();
                    let rest: Vec<Arc<Element>> = list.iter().enumerate().filter(|(i, _)| !idx.contains(i)).map(|(_, e)| Arc::clone(e)).collect();
                    *list = if how == "front" { sel.into_iter().chain(rest).collect() } else { rest.into_iter().chain(sel).collect() };
                }
                "forward" => {
                    let mut selected: Vec<bool> = (0..n).map(|i| idx.contains(&i)).collect();
                    for i in 1..n {
                        if selected[i] && !selected[i - 1] {
                            list.swap(i - 1, i);
                            selected.swap(i - 1, i);
                        }
                    }
                }
                _ => {
                    let mut selected: Vec<bool> = (0..n).map(|i| idx.contains(&i)).collect();
                    for i in (0..n.saturating_sub(1)).rev() {
                        if selected[i] && !selected[i + 1] {
                            list.swap(i, i + 1);
                            selected.swap(i, i + 1);
                        }
                    }
                }
            }
        }
        Ok(CommandOutput::doc())
    })
}

/// Gruppiert die Auswahl (Maskengruppe: oberstes Element = Maske).
fn group(d: &mut DocumentState, as_mask: bool) -> Result<ElementId> {
    let page = d.page;
    let paths = selected_paths(d);
    let first = paths.first().cloned().ok_or_else(|| sel_err("err-sel-nothing"))?;
    if as_mask && paths.len() < 2 {
        return Err(sel_err("err-sel-two-objects"));
    }
    let ids = d.selection.elements.clone();
    let doc = Arc::make_mut(&mut d.doc);
    let removed: Vec<Arc<Element>> = docops::remove_ids(doc, page, &ids).into_iter().map(|(_, e)| e).collect();
    let mut idg = take_ids(doc);
    let g = if as_mask {
        let mut it = removed.into_iter();
        let mask = it.next().ok_or_else(|| sel_err("err-sel-empty"))?;
        let vector = !matches!(mask.kind, ElementKind::Bitmap(_));
        Element::new_mask_group(&mut idg, mask, it.collect(), vector)
    } else {
        Element::new_group(&mut idg, removed)
    };
    doc.ids = idg;
    let gid = g.id;
    let (list, idx) = docops::parent_list_mut(doc, &first).ok_or_else(|| EngineError::Failed(fp_i18n::t("err-parent-list")))?;
    list.insert(idx.min(list.len()), Arc::new(g));
    d.selection.set(vec![gid]);
    Ok(gid)
}

fn ungroup(d: &mut DocumentState, only_masks: bool) -> Result<Vec<ElementId>> {
    let page = d.page;
    let mut new_sel = Vec::new();
    let paths = selected_paths(d);
    let doc = Arc::make_mut(&mut d.doc);
    // von unten nach oben, damit Indizes stimmen
    for p in paths.iter().rev() {
        let Some(e) = doc.element_at(p).cloned() else { continue };
        let ElementKind::Group(g) = &e.kind else {
            new_sel.push(e.id);
            continue;
        };
        if only_masks && !g.kind.is_mask() {
            new_sel.push(e.id);
            continue;
        }
        let mut items: Vec<Arc<Element>> = Vec::new();
        if !only_masks && let Some(m) = &g.mask {
            items.push(Arc::clone(m));
        }
        items.extend(g.children.iter().cloned());
        let Some(cur) = docops::find_on_page(doc, page, e.id) else { continue };
        let Some((list, idx)) = docops::parent_list_mut(doc, &cur) else { continue };
        list.remove(idx);
        for (k, it) in items.iter().enumerate() {
            list.insert(idx + k, Arc::clone(it));
            new_sel.push(it.id);
        }
    }
    d.selection.set(new_sel.clone());
    Ok(new_sel)
}

// ------------------------------------------------------------------ Ausrichten / Verteilen

/// Liegt direkt unter dem untersten ausgewählten Objekt eine Bitmap (Mit darunter liegender reduzieren)?
fn bitmap_below_selection(s: &Session) -> bool {
    let Some(d) = s.active() else { return false };
    let Some((last, _)) = selected_elements(d).into_iter().last() else { return false };
    let mut bp = last;
    match bp.steps.last_mut() {
        Some(PathStep::Child(i)) => *i += 1,
        _ => return false,
    }
    d.doc.element_at(&bp).is_some_and(|e| matches!(e.kind, ElementKind::Bitmap(_)))
}

fn align(s: &mut Session, how: &'static str) -> Result<CommandOutput> {
    with_doc(s, |d| {
        let page = d.page;
        let items: Vec<(ElementId, Rect)> = selected_elements(d).into_iter().filter_map(|(_, e)| docops::element_bounds(&d.doc, &e).map(|b| (e.id, b))).collect();
        if items.is_empty() {
            return Err(sel_err("err-sel-no-objects-with-bounds"));
        }
        let reference = if items.len() == 1 {
            canvas_rect(&d.doc, page)
        } else {
            items.iter().skip(1).fold(items[0].1, |a, (_, b)| a.union(b))
        };
        let mut moves: Vec<(ElementId, f32, f32)> = Vec::new();
        match how {
            "distributeWidths" | "distributeHeights" => {
                let horiz = how == "distributeWidths";
                let mut v = items.clone();
                v.sort_by(|a, b| {
                    let (ka, kb) = if horiz { (a.1.left, b.1.left) } else { (a.1.top, b.1.top) };
                    ka.partial_cmp(&kb).unwrap_or(std::cmp::Ordering::Equal)
                });
                if v.len() >= 3 {
                    let total: f32 = v.iter().map(|(_, b)| if horiz { b.width() } else { b.height() }).sum();
                    let span = if horiz { reference.width() } else { reference.height() };
                    let gap = (span - total) / (v.len() - 1) as f32;
                    let mut pos = if horiz { reference.left } else { reference.top };
                    for (id, b) in v {
                        let (cur, size) = if horiz { (b.left, b.width()) } else { (b.top, b.height()) };
                        if horiz {
                            moves.push((id, pos - cur, 0.0));
                        } else {
                            moves.push((id, 0.0, pos - cur));
                        }
                        pos += size + gap;
                    }
                }
            }
            _ => {
                for (id, b) in &items {
                    let (dx, dy) = match how {
                        "left" => (reference.left - b.left, 0.0),
                        "right" => (reference.right - b.right, 0.0),
                        "centerVertical" => (reference.center().x - b.center().x, 0.0),
                        "top" => (0.0, reference.top - b.top),
                        "bottom" => (0.0, reference.bottom - b.bottom),
                        _ => (0.0, reference.center().y - b.center().y),
                    };
                    moves.push((*id, dx, dy));
                }
            }
        }
        let doc = Arc::make_mut(&mut d.doc);
        for (id, dx, dy) in moves {
            docops::edit_by_id(doc, page, id, |e| docops::translate_element(e, dx, dy));
        }
        Ok(CommandOutput::doc())
    })
}

// ------------------------------------------------------------------ Pfade

fn selected_path_shapes(d: &DocumentState) -> Vec<(ElementPath, Arc<Element>, PathShape)> {
    selected_elements(d)
        .into_iter()
        .filter_map(|(p, e)| match &e.kind {
            ElementKind::Path(pe) => {
                let shape = PathShape { contours: pe.contours.clone(), even_odd: pe.even_odd };
                Some((p, e, shape))
            }
            _ => None,
        })
        .collect()
}

fn set_contours(e: &mut Element, contours: Vec<Contour>, even_odd: Option<bool>) {
    if let ElementKind::Path(p) = &mut e.kind {
        p.contours = contours;
        p.polyline = None;
        p.primitive = None;
        if let Some(eo) = even_odd {
            p.even_odd = eo;
        }
    }
}

fn combine(s: &mut Session, op: PathBoolOp) -> Result<CommandOutput> {
    let geo = Arc::clone(&s.geometry_arc());
    with_doc(s, |d| {
        let page = d.page;
        let items = selected_path_shapes(d);
        if items.len() < 2 {
            return Err(sel_err("err-sel-two-paths"));
        }
        let shapes: Vec<PathShape> = items.iter().map(|(_, _, sh)| sh.clone()).collect();
        let res = geo.combine(op, &shapes)?;
        let doc = Arc::make_mut(&mut d.doc);
        match op {
            PathBoolOp::Union | PathBoolOp::Intersect => {
                let contours = res.into_iter().next().unwrap_or_default();
                let (bottom_path, bottom) = (items[items.len() - 1].0.clone(), Arc::clone(&items[items.len() - 1].1));
                let others: Vec<ElementId> = items[..items.len() - 1].iter().map(|(_, e, _)| e.id).collect();
                let _ = bottom_path;
                docops::remove_ids(doc, page, &others);
                if contours.is_empty() {
                    docops::remove_ids(doc, page, &[bottom.id]);
                    d.selection.clear();
                } else {
                    docops::edit_by_id(doc, page, bottom.id, |e| set_contours(e, contours, Some(true)));
                    d.selection.set(vec![bottom.id]);
                }
            }
            PathBoolOp::Punch | PathBoolOp::Crop => {
                let tool = items[0].1.id;
                docops::remove_ids(doc, page, &[tool]);
                let mut keep = Vec::new();
                for ((_, e, _), contours) in items[1..].iter().zip(res) {
                    if contours.is_empty() {
                        docops::remove_ids(doc, page, &[e.id]);
                    } else {
                        docops::edit_by_id(doc, page, e.id, |x| set_contours(x, contours, Some(true)));
                        keep.push(e.id);
                    }
                }
                d.selection.set(keep);
            }
        }
        Ok(CommandOutput::doc())
    })
}

fn alter_paths(s: &mut Session, f: impl Fn(&dyn geometry::GeometryOps, &Element, &[Contour]) -> Result<Vec<Contour>>) -> Result<CommandOutput> {
    let geo = Arc::clone(&s.geometry_arc());
    with_doc(s, |d| {
        let page = d.page;
        let items = selected_path_shapes(d);
        if items.is_empty() {
            return Err(sel_err("err-sel-no-paths-selected"));
        }
        let mut results = Vec::new();
        for (_, e, sh) in &items {
            results.push((e.id, f(geo.as_ref(), e, &sh.contours)?));
        }
        let doc = Arc::make_mut(&mut d.doc);
        for (id, c) in results {
            docops::edit_by_id(doc, page, id, |e| set_contours(e, c, Some(true)));
        }
        Ok(CommandOutput::doc())
    })
}

fn join(d: &mut DocumentState) -> Result<()> {
    let page = d.page;
    let items = selected_path_shapes(d);
    let doc = Arc::make_mut(&mut d.doc);
    match items.len() {
        0 => Err(sel_err("err-sel-no-paths-selected")),
        1 => {
            let (_, e, sh) = &items[0];
            let open: Vec<usize> = sh.contours.iter().enumerate().filter(|(_, c)| !c.closed).map(|(i, _)| i).collect();
            if open.len() != 1 || sh.contours.len() != 1 {
                return Err(sel_err("err-sel-no-single-open-contour"));
            }
            docops::edit_by_id(doc, page, e.id, |x| {
                if let ElementKind::Path(p) = &mut x.kind
                    && let Some(c) = p.contours.first_mut()
                {
                    c.closed = true;
                }
            });
            Ok(())
        }
        _ => {
            let pts = &d.selection.points;
            let end_of = |e: &Element, c: &Contour| -> Option<bool> {
                let last = c.nodes.len().checked_sub(1)?;
                if pts.iter().any(|p| p.element == e.id && p.contour == 0 && p.node == last) {
                    Some(true)
                } else if pts.iter().any(|p| p.element == e.id && p.contour == 0 && p.node == 0) {
                    Some(false)
                } else {
                    None
                }
            };
            let connect = items.len() == 2
                && items.iter().all(|(_, _, sh)| sh.contours.len() == 1 && !sh.contours[0].closed)
                && items.iter().all(|(_, e, sh)| end_of(e, &sh.contours[0]).is_some());
            let bottom = Arc::clone(&items[items.len() - 1].1);
            let contours: Vec<Contour> = if connect {
                let orient = |e: &Element, c: &Contour, want_last: bool| -> Contour {
                    let is_last = end_of(e, c).unwrap_or(true);
                    if is_last == want_last { c.clone() } else { geometry::from_bezier(&geometry::to_bezier(c).reversed()) }
                };
                let top = orient(&items[0].1, &items[0].2.contours[0], true);
                let bot = orient(&items[1].1, &items[1].2.contours[0], false);
                let mut nodes = top.nodes;
                nodes.extend(bot.nodes);
                vec![Contour { closed: false, bsl: None, nodes, extra: Extra::default() }]
            } else {
                items.iter().flat_map(|(_, _, sh)| sh.contours.iter().cloned()).collect()
            };
            let others: Vec<ElementId> = items[..items.len() - 1].iter().map(|(_, e, _)| e.id).collect();
            docops::remove_ids(doc, page, &others);
            docops::edit_by_id(doc, page, bottom.id, |x| set_contours(x, contours, None));
            d.selection.set(vec![bottom.id]);
            Ok(())
        }
    }
}

fn split(d: &mut DocumentState) -> Result<()> {
    let page = d.page;
    let items = selected_path_shapes(d);
    let doc = Arc::make_mut(&mut d.doc);
    let mut sel = Vec::new();
    for (_, e, sh) in items {
        if sh.contours.len() < 2 {
            sel.push(e.id);
            continue;
        }
        let Some(path) = docops::find_on_page(doc, page, e.id) else { continue };
        let Some((list, idx)) = docops::parent_list_mut(doc, &path) else { continue };
        list.remove(idx);
        let mut parts = Vec::new();
        for c in sh.contours.iter().rev() {
            let mut n = (*e).clone();
            set_contours(&mut n, vec![c.clone()], None);
            parts.push(n);
        }
        let mut idg = std::mem::take(&mut doc.ids);
        let parts: Vec<Element> = parts.into_iter().map(|mut p| {
            p.id = idg.element();
            p.cache = None;
            p
        }).collect();
        doc.ids = idg;
        if let Some((list, _)) = docops::parent_list_mut(doc, &path) {
            for (k, p) in parts.into_iter().enumerate() {
                sel.push(p.id);
                list.insert((idx + k).min(list.len()), Arc::new(p));
            }
        }
    }
    d.selection.set(sel);
    Ok(())
}

/// „An Pixeln ausrichten“ (14 Laufzeit, Frage 7, A): rechte/untere Kante kaufmännisch runden,
/// Breite/Höhe zur geraden Zahl runden, linke/obere Kante = Kante − Größe. Nur geschlossene Pfade.
pub fn snap_rect(b: Rect) -> Rect {
    let r = b.right.round();
    let bo = b.bottom.round();
    let w = b.width().round_ties_even();
    let h = b.height().round_ties_even();
    Rect::new(r - w, bo - h, r, bo)
}

fn snap_to_pixel(d: &mut DocumentState) -> Result<()> {
    let page = d.page;
    let items = selected_path_shapes(d);
    let doc = Arc::make_mut(&mut d.doc);
    for (_, e, sh) in items {
        if sh.contours.is_empty() || sh.contours.iter().any(|c| !c.closed) {
            continue;
        }
        let Some(b) = geometry::contours_bounds(&sh.contours) else { continue };
        let t = snap_rect(b);
        if t.width() <= 0.0 || t.height() <= 0.0 || t == b {
            continue;
        }
        let m = docops::rect_to_rect(b, t);
        docops::edit_by_id(doc, page, e.id, |x| docops::transform_element(x, &m, fp_raster::Sampler::Nearest));
    }
    Ok(())
}

// ------------------------------------------------------------------ Masken

fn mask_bitmap(d: &mut DocumentState, mask_img: RgbaImage, target: ElementId) -> Result<ElementId> {
    let page = d.page;
    let doc = Arc::make_mut(&mut d.doc);
    let path = docops::find_on_page(doc, page, target).ok_or(EngineError::ElementNotFound(target.0))?;
    let mut idg = take_ids(doc);
    let mut next = docops::tile_counter();
    let r = mask_img.bounds();
    let mask = Element::new_bitmap(&mut idg, Point::new(r.left as f32, r.top as f32), mask_img.width() as u32, mask_img.height() as u32, mask_img.as_bytes(), &mut next);
    let (list, idx) = docops::parent_list_mut(doc, &path).ok_or_else(|| EngineError::Failed(fp_i18n::t("err-parent-list")))?;
    let el = list.remove(idx);
    let g = Element::new_mask_group(&mut idg, Arc::new(mask), vec![el], false);
    let gid = g.id;
    list.insert(idx, Arc::new(g));
    doc.ids = idg;
    d.selection.set(vec![gid]);
    Ok(gid)
}

fn mask_from_selection(s: &mut Session, reveal_all: Option<bool>) -> Result<CommandOutput> {
    with_doc(s, |d| {
        let target = d.selection.elements.first().copied().ok_or_else(|| sel_err("err-sel-no-object"))?;
        let e = docops::element_on_page(&d.doc, d.page, target).ok_or(EngineError::ElementNotFound(target.0))?;
        let b = docops::element_bounds(&d.doc, &e).ok_or_else(|| sel_err("err-sel-no-bounds-short"))?.round_out();
        let img = match reveal_all {
            Some(reveal) => RgbaImage::filled(b, if reveal { fp_color::Rgba8::new(255, 255, 255, 255) } else { fp_color::Rgba8::new(0, 0, 0, 255) })?,
            None => return Err(EngineError::Failed(fp_i18n::t("err-internal"))),
        };
        let id = mask_bitmap(d, img, target)?;
        Ok(CommandOutput::doc_with(json!({"id": id.0})))
    })
}

pub(crate) fn mask_from_pixels(s: &mut Session, reveal: bool) -> Result<CommandOutput> {
    with_doc(s, |d| {
        let target = d.selection.elements.first().copied().ok_or_else(|| sel_err("err-sel-no-object"))?;
        let m = d.selection.pixels.clone().ok_or_else(|| sel_err("err-req-pixel-selection"))?;
        let e = docops::element_on_page(&d.doc, d.page, target).ok_or(EngineError::ElementNotFound(target.0))?;
        let b = docops::element_bounds(&d.doc, &e).ok_or_else(|| sel_err("err-sel-no-bounds-short"))?.round_out().union(&m.tight_bounds());
        let mut img = RgbaImage::new(b)?;
        let w = img.width().max(1);
        for (i, px) in img.data_mut().iter_mut().enumerate() {
            let (x, y) = (b.left + (i % w) as i32, b.top + (i / w) as i32);
            let v = m.get(x, y);
            let g = if reveal { v } else { 255 - v };
            *px = [g, g, g, 255];
        }
        d.selection.pixels = None;
        let id = mask_bitmap(d, img, target)?;
        Ok(CommandOutput::doc_with(json!({"id": id.0})))
    })
}

// ------------------------------------------------------------------ Reduzieren

fn composite_bitmaps(doc: &fp_model::Document, els_top_first: &[Arc<Element>]) -> Option<RgbaImage> {
    let mut out: Option<RgbaImage> = None;
    for e in els_top_first.iter().rev() {
        let ElementKind::Bitmap(b) = &e.kind else { return None };
        if !e.common.visible {
            continue;
        }
        let img = bitmap_image(b)?;
        let opt = CompositeOptions { mode: e.common.blend, opacity: e.common.opacity, ..CompositeOptions::default() };
        out = Some(match out {
            None => {
                let base = RgbaImage::new(img.bounds()).ok()?;
                fp_raster::composite_extend(&base, &img, None, &opt)
            }
            Some(o) => fp_raster::composite_extend(&o, &img, None, &opt),
        });
    }
    let _ = doc;
    out
}

fn flatten_elements(s: &Session, els: &[Arc<Element>]) -> Result<RgbaImage> {
    let d = s.doc()?;
    if let Some(img) = composite_bitmaps(&d.doc, els) {
        return Ok(img);
    }
    let r = s.renderer().ok_or(EngineError::BackendMissing("err-backend-renderer-vector"))?;
    r.render_elements(&d.doc, d.page, d.frame, els).ok_or(EngineError::BackendMissing("err-backend-renderer"))
}

fn replace_with_bitmap(d: &mut DocumentState, remove: &[ElementId], img: &RgbaImage, at: &ElementPath) -> Result<ElementId> {
    let page = d.page;
    let doc = Arc::make_mut(&mut d.doc);
    let idx = match at.steps.last() {
        Some(PathStep::Child(i)) => *i,
        _ => 0,
    };
    docops::remove_ids(doc, page, remove);
    let mut idg = take_ids(doc);
    let mut next = docops::tile_counter();
    let r = img.bounds();
    let e = Element::new_bitmap(&mut idg, Point::new(r.left as f32, r.top as f32), img.width() as u32, img.height() as u32, img.as_bytes(), &mut next);
    doc.ids = idg;
    let id = e.id;
    let mut probe = at.clone();
    if let Some(last) = probe.steps.last_mut() {
        *last = PathStep::Child(0);
    }
    match docops::parent_list_mut(doc, &probe) {
        Some((list, _)) => list.insert(idx.min(list.len()), Arc::new(e)),
        None => {
            let (frame, layer) = (d.frame, d.layer);
            docops::insert_element(Arc::make_mut(&mut d.doc), page, frame, layer, 0, Arc::new(e));
        }
    }
    d.selection.set(vec![id]);
    Ok(id)
}

// ------------------------------------------------------------------ Leinwand

fn rotate_canvas(s: &mut Session, deg: i32) -> Result<CommandOutput> {
    let sm = sampler(s);
    with_doc(s, |d| {
        let page = d.page;
        let doc = Arc::make_mut(&mut d.doc);
        let pg = doc.pages.get(page).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
        let (ox, oy, w, h) = (pg.canvas.origin.x, pg.canvas.origin.y, pg.canvas.width, pg.canvas.height);
        let rot = Matrix3::rotation_degrees(deg as f64);
        let shift = match deg {
            90 => (ox + h, oy),
            -90 => (ox, oy + w),
            _ => (ox + w, oy + h),
        };
        let m = Matrix3::translation(-ox, -oy).then(&rot).then(&Matrix3::translation(shift.0, shift.1));
        transform_page(doc, page, &m, sm);
        if let Some(p) = doc.page_mut(page) {
            if deg != 180 {
                std::mem::swap(&mut p.canvas.width, &mut p.canvas.height);
            }
            if let Some(g) = &mut p.root.guides {
                let (hz, vt) = (std::mem::take(&mut g.horizontal), std::mem::take(&mut g.vertical));
                match deg {
                    90 => {
                        g.horizontal = vt.iter().map(|x| x - ox + oy).collect();
                        g.vertical = hz.iter().map(|y| ox + h - (y - oy)).collect();
                    }
                    -90 => {
                        g.vertical = hz.iter().map(|y| y - oy + ox).collect();
                        g.horizontal = vt.iter().map(|x| oy + w - (x - ox)).collect();
                    }
                    _ => {
                        g.vertical = vt.iter().map(|x| 2.0 * ox + w - x).collect();
                        g.horizontal = hz.iter().map(|y| 2.0 * oy + h - y).collect();
                    }
                }
            }
        }
        d.selection.pixels = None;
        Ok(CommandOutput::doc())
    })
}

fn anchor_factor(a: &str) -> (f32, f32) {
    match a {
        "topLeft" => (0.0, 0.0),
        "top" => (0.5, 0.0),
        "topRight" => (1.0, 0.0),
        "left" => (0.0, 0.5),
        "right" => (1.0, 0.5),
        "bottomLeft" => (0.0, 1.0),
        "bottom" => (0.5, 1.0),
        "bottomRight" => (1.0, 1.0),
        _ => (0.5, 0.5),
    }
}

// ------------------------------------------------------------------ Symbole

fn selected_instances(d: &DocumentState) -> Vec<(ElementPath, Arc<Element>)> {
    selected_elements(d).into_iter().filter(|(_, e)| matches!(e.kind, ElementKind::Instance(_))).collect()
}

fn ensure_frames(d: &mut DocumentState, n: usize) {
    let page = d.page;
    let cur = d.frame_count();
    if n > cur {
        docops::insert_frames(Arc::make_mut(&mut d.doc), page, cur, n - cur);
    }
}

/// Entfernt leere Status am Ende, solange mehr als `keep` (und mehr als die längste Animation der
/// Seite) vorhanden sind – so ist „Animation einstellen“ mit kleinerer Statuszahl idempotent.
fn trim_empty_frames(d: &mut DocumentState, keep: usize) {
    let page = d.page;
    let mut need = keep.max(1);
    d.doc.walk_elements(|_, path, e| {
        if path.root == fp_model::RootRef::Page(page)
            && let ElementKind::Instance(i) = &e.kind
            && d.doc.symbol_by_id(i.symbol_id).is_some_and(|(_, s)| s.kind() == SymbolKind::Animation)
        {
            need = need.max(i.anim.frames as usize);
        }
    });
    loop {
        let n = d.frame_count();
        if n <= need {
            break;
        }
        let last = n - 1;
        let empty = docops::own_layers(&d.doc, page).iter().all(|(_, l)| l.shared_across_frames || l.cels.get(last).is_none_or(|c| c.elements.is_empty()));
        if !empty {
            break;
        }
        if docops::delete_frame(Arc::make_mut(&mut d.doc), page, last).is_none() {
            break;
        }
        if d.frame >= last {
            d.frame = last.saturating_sub(1);
        }
    }
}

fn anim_params(p: &Params, base: fp_model::AnimParams) -> Result<fp_model::AnimParams> {
    let mut a = base;
    if let Some(v) = p.opt_i64("frames")? {
        a.frames = v.clamp(1, 1000) as u32;
    }
    if let Some(v) = p.opt_f64("offsetX")? {
        a.offset.x = v as f32;
    }
    if let Some(v) = p.opt_f64("offsetY")? {
        a.offset.y = v as f32;
    }
    if let Some(v) = p.opt_i64("rotation")? {
        a.rotation = v as i32;
    }
    if let Some(v) = p.opt_f64("scale")? {
        a.scale = v as f32;
    }
    if let Some(v) = p.opt_f64("startOpacity")? {
        a.start_opacity = (v.clamp(0.0, 100.0) * 10.0).round() as u32;
    }
    if let Some(v) = p.opt_f64("endOpacity")? {
        a.end_opacity = (v.clamp(0.0, 100.0) * 10.0).round() as u32;
    }
    Ok(a)
}

fn with_anim_params(b: crate::command::CommandBuilder) -> crate::command::CommandBuilder {
    b.opt("frames", ParamKind::Integer { min: Some(1), max: Some(1000) }, "Anzahl Status")
        .opt("offsetX", ParamKind::Number { min: None, max: None }, "Versatz x bis zum letzten Status")
        .opt("offsetY", ParamKind::Number { min: None, max: None }, "Versatz y")
        .opt("rotation", ParamKind::Integer { min: None, max: None }, "Drehung in Grad (im Uhrzeigersinn)")
        .opt("scale", ParamKind::Number { min: Some(0.0), max: None }, "Endskalierung in %")
        .opt("startOpacity", ParamKind::Number { min: Some(0.0), max: Some(100.0) }, "Anfangsdeckkraft in %")
        .opt("endOpacity", ParamKind::Number { min: Some(0.0), max: Some(100.0) }, "Enddeckkraft in %")
}

pub(super) fn register(r: &mut CommandRegistry) {
    // Leinwand
    r.add(
        cmd("modify.canvas.imageSize")
            .requires(DOC)
            .opt("width", ParamKind::Number { min: Some(1.0), max: None }, "neue Breite in Pixel")
            .opt("height", ParamKind::Number { min: Some(1.0), max: None }, "neue Höhe in Pixel")
            .opt("percent", ParamKind::Number { min: Some(0.1), max: None }, "Skalierung in %")
            .opt_default("constrain", ParamKind::Bool, json!(true), "Proportionen beibehalten")
            .opt_default("resample", ParamKind::Bool, json!(true), "Bild neu auflösen")
            .opt("resolution", ParamKind::Number { min: Some(1.0), max: None }, "Auflösung (Pixel/Zoll)")
            .run(|s, p| {
                let sm = s.prefs.image_size_sampler();
                let (wq, hq, pc) = (p.opt_f64("width")?, p.opt_f64("height")?, p.opt_f64("percent")?);
                let constrain = p.bool_or("constrain", true)?;
                let resample = p.bool_or("resample", true)?;
                let res = p.opt_f64("resolution")?;
                with_doc(s, |d| {
                    let page = d.page;
                    let doc = Arc::make_mut(&mut d.doc);
                    let pg = doc.pages.get(page).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
                    let (ox, oy, w, h) = (pg.canvas.origin.x as f64, pg.canvas.origin.y as f64, pg.canvas.width as f64, pg.canvas.height as f64);
                    let (mut sx, mut sy) = match pc {
                        Some(pc) => (pc / 100.0, pc / 100.0),
                        None => (wq.map(|v| v / w.max(1e-9)).unwrap_or(f64::NAN), hq.map(|v| v / h.max(1e-9)).unwrap_or(f64::NAN)),
                    };
                    if sx.is_nan() {
                        sx = if constrain && !sy.is_nan() { sy } else { 1.0 };
                    }
                    if sy.is_nan() {
                        sy = if constrain { sx } else { 1.0 };
                    }
                    if resample {
                        let m = Matrix3::translation(-ox as f32, -oy as f32).then(&Matrix3::scaling(sx as f32, sy as f32)).then(&Matrix3::translation(ox as f32, oy as f32));
                        transform_page(doc, page, &m, sm);
                    }
                    let p = doc.page_mut(page).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
                    if resample {
                        p.canvas.width = (w * sx).round().max(1.0) as f32;
                        p.canvas.height = (h * sy).round().max(1.0) as f32;
                    }
                    if let Some(rv) = res {
                        p.canvas.resolution = rv as f32;
                    } else if !resample {
                        p.canvas.resolution = (p.canvas.resolution as f64 * sx) as f32;
                    }
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(
        cmd("modify.canvas.canvasSize")
            .requires(DOC)
            .param("width", ParamKind::Number { min: Some(1.0), max: None }, "neue Breite")
            .param("height", ParamKind::Number { min: Some(1.0), max: None }, "neue Höhe")
            .opt_default(
                "anchor",
                ParamKind::Enum { values: ["topLeft", "top", "topRight", "left", "center", "right", "bottomLeft", "bottom", "bottomRight"].iter().map(|s| s.to_string()).collect() },
                json!("center"),
                "Verankerung",
            )
            .run(|s, p| {
                let (w, h) = (p.f64("width")? as f32, p.f64("height")? as f32);
                let (ax, ay) = anchor_factor(&p.opt_str("anchor")?.unwrap_or_default());
                with_doc(s, |d| {
                    let c = canvas_rect(&d.doc, d.page);
                    let x0 = c.left + (c.width() - w) * ax;
                    let y0 = c.top + (c.height() - h) * ay;
                    crop_page(d, Rect::new(x0, y0, x0 + w, y0 + h).round_nearest())?;
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(cmd("modify.canvas.canvasColor").requires(DOC).param("color", ParamKind::Color, "Leinwandfarbe (transparent: #ffffff00)").run(|s, p| {
        let c = p.color("color")?;
        with_doc(s, |d| {
            let page = d.page;
            Arc::make_mut(&mut d.doc).page_mut(page).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?.root.background = c;
            Ok(CommandOutput::doc())
        })
    }));
    r.add(cmd("modify.canvas.trimCanvas").requires(DOC).run(|s, _| {
        with_doc(s, |d| {
            let b = docops::page_content_bounds(&d.doc, d.page).ok_or_else(|| sel_err("err-sel-page-empty"))?;
            crop_page(d, b.round_out())?;
            Ok(CommandOutput::doc())
        })
    }));
    r.add(cmd("modify.canvas.fitCanvas").requires(DOC).run(|s, _| {
        with_doc(s, |d| {
            // Leinwand an die Objekte anpassen (vergrößern und verkleinern, wie im Original)
            let c = canvas_rect(&d.doc, d.page);
            let b = docops::page_content_bounds(&d.doc, d.page).unwrap_or(c);
            crop_page(d, b.round_out())?;
            Ok(CommandOutput::doc())
        })
    }));
    r.add(cmd("modify.canvas.rotate180Degree").requires(DOC).run(|s, _| rotate_canvas(s, 180)));
    r.add(cmd("modify.canvas.rotate90DegreeCw").requires(DOC).run(|s, _| rotate_canvas(s, 90)));
    r.add(cmd("modify.canvas.rotate90DegreeCcw").requires(DOC).run(|s, _| rotate_canvas(s, -90)));

    // Animation
    r.add(with_anim_params(cmd("modify.animation.animateSelection").requires(SEL)).opt("name", ParamKind::String, "Symbolname").run(|s, p| {
        let name = p.opt_str("name")?.unwrap_or_else(|| "Animation".into());
        let a = anim_params(p, fp_model::AnimParams::default())?;
        with_doc(s, |d| {
            let ids = d.selection.elements.clone();
            let id = make_symbol(d, &name, SymbolKind::Animation, &ids)?;
            let page = d.page;
            docops::edit_by_id(Arc::make_mut(&mut d.doc), page, id, |e| {
                if let ElementKind::Instance(i) = &mut e.kind {
                    i.anim = a;
                }
            });
            ensure_frames(d, a.frames as usize);
            Ok(CommandOutput::doc_with(json!({"id": id.0})))
        })
    }));
    r.add(with_anim_params(cmd("modify.animation.settings").requires(&[Requirement::Document, Requirement::AllOf(ObjectKind::AnimationInstance)])).run(|s, p| {
        with_doc(s, |d| {
            let page = d.page;
            let insts = selected_instances(d);
            let mut maxf = 1;
            let doc = Arc::make_mut(&mut d.doc);
            for (_, e) in insts {
                if let ElementKind::Instance(i) = &e.kind {
                    let a = anim_params(p, i.anim)?;
                    maxf = maxf.max(a.frames as usize);
                    docops::edit_by_id(doc, page, e.id, |x| {
                        if let ElementKind::Instance(i) = &mut x.kind {
                            i.anim = a;
                        }
                    });
                }
            }
            ensure_frames(d, maxf);
            trim_empty_frames(d, maxf);
            Ok(CommandOutput::doc())
        })
    }));
    r.add(cmd("modify.animation.removeAnimation").requires(&[Requirement::Document, Requirement::AnyOf(ObjectKind::AnimationInstance)]).run(|s, _| {
        with_doc(s, |d| {
            let page = d.page;
            let insts = selected_instances(d);
            let doc = Arc::make_mut(&mut d.doc);
            for (_, e) in insts {
                let ElementKind::Instance(i) = &e.kind else { continue };
                if let Some((k, _)) = doc.symbol_by_id(i.symbol_id) {
                    let sym = Arc::make_mut(&mut doc.symbols[k.0]);
                    if sym.kind() == SymbolKind::Animation {
                        sym.kind_code = None;
                    }
                }
                docops::edit_by_id(doc, page, e.id, |x| {
                    if let ElementKind::Instance(i) = &mut x.kind {
                        i.anim = fp_model::AnimParams { frames: 1, ..fp_model::AnimParams::default() };
                    }
                });
            }
            Ok(CommandOutput::doc())
        })
    }));

    // Symbole
    r.add(
        cmd("modify.symbol.convertToSymbol")
            .requires(SEL)
            .opt("name", ParamKind::String, "Symbolname")
            .opt_default("kind", ParamKind::Enum { values: vec!["graphic".into(), "button".into(), "animation".into()] }, json!("graphic"), "Symboltyp")
            .run(|s, p| {
                let name = p.opt_str("name")?.unwrap_or_else(|| "Symbol".into());
                let kind = match p.opt_str("kind")?.as_deref() {
                    Some("button") => SymbolKind::Button,
                    Some("animation") => SymbolKind::Animation,
                    _ => SymbolKind::Graphic,
                };
                with_doc(s, |d| {
                    let ids = d.selection.elements.clone();
                    let id = make_symbol(d, &name, kind, &ids)?;
                    Ok(CommandOutput::doc_with(json!({"id": id.0})))
                })
            }),
    );
    r.add(
        cmd("modify.symbol.tweenInstances")
            // im Original immer aktiv; ohne zwei Instanzen Meldung statt Wirkung
            .requires(DOC)
            .opt_default("steps", ParamKind::Integer { min: Some(1), max: Some(1000) }, json!(1), "Zwischenschritte")
            .run(|s, p| {
                let steps = p.i64_or("steps", 1)?.clamp(1, 1000) as usize;
                with_doc(s, |d| {
                    let page = d.page;
                    let insts = selected_instances(d);
                    if insts.len() != 2 {
                        return Err(sel_err("err-sel-two-instances"));
                    }
                    let (top_path, a) = (insts[0].0.clone(), Arc::clone(&insts[0].1));
                    let b = Arc::clone(&insts[1].1);
                    let (ElementKind::Instance(ia), ElementKind::Instance(ib)) = (&a.kind, &b.kind) else { return Err(sel_err("err-sel-instances")) };
                    if ia.symbol_id != ib.symbol_id {
                        return Err(sel_err("err-sel-same-symbol"));
                    }
                    let doc = Arc::make_mut(&mut d.doc);
                    let mut new_ids = Vec::new();
                    // b liegt unter a: Zwischenschritte zwischen beiden einfügen (von b nach a)
                    let Some(bp) = docops::find_on_page(doc, page, b.id) else { return Err(EngineError::ElementNotFound(b.id.0)) };
                    let _ = top_path;
                    for k in 1..=steps {
                        let t = k as f64 / (steps + 1) as f64;
                        let mut n = docops::reassign(doc, &b);
                        if let ElementKind::Instance(i) = &mut n.kind {
                            i.transform = fp_geom::decompose::interpolate_matrices(&ib.transform, &ia.transform, t);
                            i.alt_transform = Some(i.transform);
                        }
                        let o = b.common.opacity.0 as f64 * (1.0 - t) + a.common.opacity.0 as f64 * t;
                        n.common.opacity = fp_color::Opacity(o.round().clamp(0.0, 255.0) as u8);
                        new_ids.push(n.id);
                        if let Some((list, idx)) = docops::parent_list_mut(doc, &bp) {
                            list.insert(idx, Arc::new(n));
                        }
                    }
                    let mut sel = vec![a.id];
                    sel.extend(new_ids.iter().copied());
                    sel.push(b.id);
                    d.selection.set(sel);
                    Ok(CommandOutput::doc_with(json!({"ids": ids_json(&new_ids)})))
                })
            }),
    );
    r.add(cmd("modify.symbol.breakApart").requires(DOC).run(|s, _| {
        let sm = sampler(s);
        with_doc(s, |d| {
            let page = d.page;
            let insts = selected_instances(d);
            if insts.is_empty() {
                return Err(sel_err("err-sel-no-instance"));
            }
            let mut sel = Vec::new();
            for (_, e) in insts.iter().rev() {
                let ElementKind::Instance(i) = &e.kind else { continue };
                let doc = Arc::make_mut(&mut d.doc);
                let Some((_, sym)) = doc.symbol_by_id(i.symbol_id).map(|(k, s)| (k, Arc::clone(s))) else { continue };
                let mut kids: Vec<Arc<Element>> = Vec::new();
                for slot in sym.root.layers.iter().rev() {
                    if let LayerSlot::Own(l) = slot
                        && l.kind == LayerKind::Normal
                        && let Some(c) = l.cels.first()
                    {
                        for k in &c.elements {
                            let mut n = docops::reassign(doc, k);
                            docops::transform_element(&mut n, &i.transform, sm);
                            n.common.opacity = fp_color::Opacity(((n.common.opacity.0 as u32 * e.common.opacity.0 as u32 + 127) / 255) as u8);
                            kids.push(Arc::new(n));
                        }
                    }
                }
                let mut idg = take_ids(doc);
                let g = Element::new_group(&mut idg, kids);
                doc.ids = idg;
                let gid = g.id;
                if let Some(path) = docops::find_on_page(doc, page, e.id)
                    && let Some((list, idx)) = docops::parent_list_mut(doc, &path)
                {
                    list[idx] = Arc::new(g);
                    sel.push(gid);
                }
            }
            d.selection.set(sel);
            Ok(CommandOutput::doc())
        })
    }));
    r.add(
        cmd("modify.symbol.swapSymbol")
            .requires(&[Requirement::Document, Requirement::AllOf(ObjectKind::Instance)])
            .param("symbol", ParamKind::String, "Name oder ID des neuen Symbols")
            .run(|s, p| {
                let key = p.str("symbol")?;
                with_doc(s, |d| {
                    let page = d.page;
                    let sid = d
                        .doc
                        .symbol_by_name(&key)
                        .map(|(_, s)| s.id)
                        .or_else(|| key.parse::<u32>().ok().map(fp_model::SymbolId).filter(|id| d.doc.symbol_by_id(*id).is_some()))
                        .ok_or_else(|| EngineError::NotFound(fp_i18n::tr!("err-nf-symbol-named", name = key.as_str())))?;
                    let ids = d.selection.elements.clone();
                    let doc = Arc::make_mut(&mut d.doc);
                    for id in ids {
                        docops::edit_by_id(doc, page, id, |e| {
                            if let ElementKind::Instance(i) = &mut e.kind {
                                i.symbol_id = sid;
                            }
                        });
                    }
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(cmd("modify.popUpMenu.deletePopUpMenu").requires(&[Requirement::Document, Requirement::AnyOf(ObjectKind::WebObject)]).run(|s, _| {
        with_doc(s, |d| {
            let (page, ids) = (d.page, d.selection.elements.clone());
            let doc = Arc::make_mut(&mut d.doc);
            for id in ids {
                docops::edit_by_id(doc, page, id, |e| {
                    if let ElementKind::WebObject(w) = &mut e.kind
                        && let Some(b) = &mut w.behaviors
                    {
                        b.items.retain(|x| x.action != fp_model::BehaviorAction::PopupMenu.code());
                    }
                });
            }
            Ok(CommandOutput::doc())
        })
    }));

    // Maske
    // wie im Original nur bei genau einem Objekt
    r.add(cmd("modify.mask.revealAll").requires(SEL).available(|s| s.active().is_some_and(|d| d.selection.elements.len() == 1)).run(|s, _| mask_from_selection(s, Some(true))));
    r.add(cmd("modify.mask.hideAll").requires(SEL).available(|s| s.active().is_some_and(|d| d.selection.elements.len() == 1)).run(|s, _| mask_from_selection(s, Some(false))));
    r.add(cmd("modify.mask.revealSelection").requires(&[Requirement::Document, Requirement::Selection, Requirement::PixelSelection]).run(|s, _| mask_from_pixels(s, true)));
    r.add(cmd("modify.mask.hideSelection").requires(&[Requirement::Document, Requirement::Selection, Requirement::PixelSelection]).run(|s, _| mask_from_pixels(s, false)));
    r.add(cmd("modify.mask.groupAsMask").requires(&[Requirement::Document, Requirement::MinSelected(2)]).run(|s, _| {
        with_doc(s, |d| {
            let id = group(d, true)?;
            Ok(CommandOutput::doc_with(json!({"id": id.0})))
        })
    }));
    r.add(
        cmd("modify.mask.disableMask")
            .requires(&[Requirement::Document, Requirement::AnyOf(ObjectKind::MaskGroup)])
            .checked(|s| {
                s.active().is_some_and(|d| {
                    super::selected_elements(d).iter().any(|(_, e)| matches!(&e.kind, ElementKind::Group(g) if g.kind.is_mask() && !g.mask_flags.enabled))
                })
            })
            .run(|s, _| {
                with_doc(s, |d| {
                    let (page, ids) = (d.page, d.selection.elements.clone());
                    let doc = Arc::make_mut(&mut d.doc);
                    for id in ids {
                        docops::edit_by_id(doc, page, id, |e| {
                            if let ElementKind::Group(g) = &mut e.kind
                                && g.kind.is_mask()
                            {
                                g.mask_flags.enabled = !g.mask_flags.enabled;
                            }
                        });
                    }
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(cmd("modify.mask.deleteMask").requires(&[Requirement::Document, Requirement::AnyOf(ObjectKind::MaskGroup)]).run(|s, _| {
        with_doc(s, |d| {
            ungroup(d, true)?;
            Ok(CommandOutput::doc())
        })
    }));

    // Selektives JPEG
    // Selektives JPEG: im Original immer aktiv (gemessen ohne Auswahl, BEFUNDE EB Z11–Z13):
    // Speichern ohne Pixelauswahl tut still nichts, Wiederherstellen und Entfernen ohne Maske
    // legen trotzdem den Verlaufsschritt „JPEG-Maske laden“ bzw. „JPEG-Maske löschen“ an.
    r.add(cmd("modify.selectiveJpeg.saveSelectionAsJpegMask").requires(DOC).run(|s, _| {
        if !s.active().is_some_and(|d| d.selection.has_pixels()) {
            return Ok(CommandOutput::value(serde_json::Value::Null));
        }
        with_doc(s, |d| {
            let m = d.selection.pixels.clone().filter(|_| d.selection.has_pixels()).ok_or_else(|| sel_err("err-req-pixel-selection"))?;
            let page = d.page;
            let (md, l, t) = mask_to_data(&m);
            Arc::make_mut(&mut d.doc).page_mut(page).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?.jpeg_mask =
                Some(SavedSelection { mask: Some(md), left: l, top: t, name: None, extra: Extra::default() });
            Ok(CommandOutput::doc())
        })
    }));
    r.add(
        cmd("modify.selectiveJpeg.restoreJpegMaskAsSelection")
            .requires(DOC)
            .run(|s, _| {
                s.force_history_step();
                with_doc(s, |d| {
                    if let Some(jm) = d.doc.pages.get(d.page).and_then(|p| p.jpeg_mask.clone()) {
                        let m = jm.mask.as_ref().and_then(|md| super::select::data_to_mask(md, jm.left, jm.top)).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-mask")))?;
                        d.selection.pixels = Some(m);
                    }
                    Ok(CommandOutput::doc().labeled(fp_i18n::t("msg-step-jpeg-mask-load")))
                })
            }),
    );
    r.add(
        cmd("modify.selectiveJpeg.removeJpegMask")
            .requires(DOC)
            .run(|s, _| {
                s.force_history_step();
                with_doc(s, |d| {
                    let page = d.page;
                    if d.doc.pages.get(page).is_some_and(|p| p.jpeg_mask.is_some()) {
                        Arc::make_mut(&mut d.doc).page_mut(page).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?.jpeg_mask = None;
                    }
                    Ok(CommandOutput::doc().labeled(fp_i18n::t("msg-step-jpeg-mask-delete")))
                })
            }),
    );

    // Sperren / Reduzieren
    r.add(cmd("modify.lockSelection").requires(SEL).run(|s, _| {
        with_doc(s, |d| {
            let (page, ids) = (d.page, d.selection.elements.clone());
            let doc = Arc::make_mut(&mut d.doc);
            for id in ids {
                docops::edit_by_id(doc, page, id, |e| e.common.locked = true);
            }
            d.selection.clear();
            Ok(CommandOutput::doc())
        })
    }));
    r.add(cmd("modify.flattenSelection").requires(SEL).note("Vektorobjekte brauchen einen Renderer (fp-render)").run(|s, _| {
        let (els, first, ids) = {
            let d = s.doc()?;
            let els = selected_elements(d);
            let first = els.first().map(|(p, _)| p.clone()).ok_or_else(|| sel_err("err-sel-nothing"))?;
            (els.iter().map(|(_, e)| Arc::clone(e)).collect::<Vec<_>>(), first, d.selection.elements.clone())
        };
        let img = flatten_elements(s, &els)?;
        with_doc(s, |d| {
            let id = replace_with_bitmap(d, &ids, &img, &first)?;
            Ok(CommandOutput::doc_with(json!({"id": id.0})))
        })
    }));
    // wie im Original nur, wenn unter dem untersten ausgewählten Objekt eine Bitmap liegt
    r.add(cmd("modify.mergeDown").requires(SEL).available(bitmap_below_selection).note("Vektorobjekte brauchen einen Renderer (fp-render)").run(|s, _| {
        let (els, below_path, ids) = {
            let d = s.doc()?;
            let els = selected_elements(d);
            let (last_path, _) = els.last().cloned().ok_or_else(|| sel_err("err-sel-nothing"))?;
            let mut bp = last_path.clone();
            match bp.steps.last_mut() {
                Some(PathStep::Child(i)) => *i += 1,
                _ => return Err(sel_err("err-sel-no-object-below")),
            }
            let below = d.doc.element_at(&bp).cloned().ok_or_else(|| sel_err("err-sel-no-object-below"))?;
            let mut all: Vec<Arc<Element>> = els.iter().map(|(_, e)| Arc::clone(e)).collect();
            all.push(Arc::clone(&below));
            let mut ids = d.selection.elements.clone();
            ids.push(below.id);
            (all, bp, ids)
        };
        let img = flatten_elements(s, &els)?;
        with_doc(s, |d| {
            let mut at = below_path.clone();
            if let Some(PathStep::Child(i)) = at.steps.last_mut() {
                *i = i.saturating_sub(els.len() - 1);
            }
            let id = replace_with_bitmap(d, &ids, &img, &at)?;
            Ok(CommandOutput::doc_with(json!({"id": id.0})))
        })
    }));
    // wie im Original erst ab zwei normalen Ebenen
    r.add(cmd("modify.flattenLayers").requires(DOC).available(|s| s.active().is_some_and(|d| docops::own_layers(&d.doc, d.page).iter().filter(|(_, l)| l.kind == LayerKind::Normal).count() >= 2)).run(|s, _| {
        with_doc(s, |d| {
            let page = d.page;
            let doc = Arc::make_mut(&mut d.doc);
            let p = doc.page_mut(page).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
            let frames = p.root.frame_count as usize;
            let normal: Vec<usize> =
                p.root.layers.iter().enumerate().filter(|(_, s)| matches!(s, LayerSlot::Own(l) if l.kind == LayerKind::Normal)).map(|(i, _)| i).collect();
            let Some(&target) = normal.first() else { return Ok(CommandOutput::doc()) };
            let mut per_frame: Vec<Vec<Arc<Element>>> = vec![Vec::new(); frames.max(1)];
            for &si in normal.iter().rev() {
                if let LayerSlot::Own(l) = &p.root.layers[si] {
                    for (f, slot) in per_frame.iter_mut().enumerate() {
                        let ci = if l.shared_across_frames { 0 } else { f };
                        if let Some(c) = l.cels.get(ci) {
                            slot.extend(c.elements.iter().cloned());
                        }
                    }
                }
            }
            if let LayerSlot::Own(l) = &mut p.root.layers[target] {
                let l = Arc::make_mut(l);
                l.shared_across_frames = false;
                l.cels = per_frame.into_iter().map(|els| Arc::new(fp_model::Cel { elements: els, ..fp_model::Cel::empty() })).collect();
            }
            for &si in normal.iter().skip(1).rev() {
                p.root.layers.remove(si);
            }
            d.layer = docops::top_normal_layer(&d.doc, page);
            Ok(CommandOutput::doc())
        })
    }));

    // Transformieren
    for (name, tool) in [
        ("modify.transform.freeTransform", ToolId::Scale),
        ("modify.transform.scale", ToolId::Scale),
        ("modify.transform.skew", ToolId::Skew),
        ("modify.transform.distort", ToolId::Distort),
    ] {
        r.add(cmd(name).requires(SEL).run(move |s, _| {
            s.set_tool(tool)?;
            Ok(CommandOutput::session())
        }));
    }
    r.add(
        cmd("modify.transform.numericTransform")
            .requires(SEL)
            .opt_default("mode", ParamKind::Enum { values: vec!["scale".into(), "resize".into(), "rotate".into()] }, json!("scale"), "Art")
            .opt("scaleX", ParamKind::Number { min: None, max: None }, "Skalierung x in %")
            .opt("scaleY", ParamKind::Number { min: None, max: None }, "Skalierung y in %")
            .opt("width", ParamKind::Number { min: Some(0.0), max: None }, "neue Breite (resize)")
            .opt("height", ParamKind::Number { min: Some(0.0), max: None }, "neue Höhe (resize)")
            .opt("angle", ParamKind::Number { min: None, max: None }, "Winkel in Grad (rotate)")
            .opt_default("constrain", ParamKind::Bool, json!(true), "Proportionen beibehalten")
            .run(|s, p| {
                let b = sel_bounds(s)?;
                let c = b.center();
                let constrain = p.bool_or("constrain", true)?;
                let m = match p.opt_str("mode")?.as_deref() {
                    Some("rotate") => Matrix3::rotation_degrees(p.f64_or("angle", 0.0)?),
                    Some("resize") => {
                        let mut sx = p.opt_f64("width")?.map(|w| w as f32 / b.width().max(1e-6));
                        let mut sy = p.opt_f64("height")?.map(|h| h as f32 / b.height().max(1e-6));
                        if constrain {
                            sx = sx.or(sy);
                            sy = sy.or(sx);
                        }
                        Matrix3::scaling(sx.unwrap_or(1.0), sy.unwrap_or(1.0))
                    }
                    _ => {
                        let mut sx = p.opt_f64("scaleX")?;
                        let mut sy = p.opt_f64("scaleY")?;
                        if constrain {
                            sx = sx.or(sy);
                            sy = sy.or(sx);
                        }
                        Matrix3::scaling((sx.unwrap_or(100.0) / 100.0) as f32, (sy.unwrap_or(100.0) / 100.0) as f32)
                    }
                };
                transform_selection(s, docops::about(c, &m))
            }),
    );
    r.add(cmd("modify.transform.rotate180Degree").requires(SEL).run(|s, _| about_selection(s, Matrix3::rotation_degrees(180.0))));
    r.add(cmd("modify.transform.rotate90DegreeCw").requires(SEL).run(|s, _| about_selection(s, Matrix3::rotation_degrees(90.0))));
    r.add(cmd("modify.transform.rotate90DegreeCcw").requires(SEL).run(|s, _| about_selection(s, Matrix3::rotation_degrees(-90.0))));
    r.add(cmd("modify.transform.flipHorizontal").requires(SEL).run(|s, _| about_selection(s, Matrix3::scaling(-1.0, 1.0))));
    r.add(cmd("modify.transform.flipVertical").requires(SEL).run(|s, _| about_selection(s, Matrix3::scaling(1.0, -1.0))));
    // Bitmaps haben im Original keine entfernbaren Transformationen
    r.add(cmd("modify.transform.removeTransformations").requires(SEL).available(|s| s.active().is_some_and(|d| selected_elements(d).iter().any(|(_, e)| !matches!(e.kind, ElementKind::Bitmap(_))))).run(|s, _| {
        with_doc(s, |d| {
            let (page, ids) = (d.page, d.selection.elements.clone());
            let doc = Arc::make_mut(&mut d.doc);
            for id in ids {
                docops::edit_by_id(doc, page, id, |e| match &mut e.kind {
                    ElementKind::Instance(i) => {
                        i.transform = Matrix3::translation(i.transform.m[2][0], i.transform.m[2][1]);
                        i.alt_transform = Some(i.transform);
                    }
                    ElementKind::Text(t) => t.transform = None,
                    _ => {}
                });
            }
            Ok(CommandOutput::doc())
        })
    }));
    r.add(
        cmd("transform.matrix")
            .label("Transformieren")
            .requires(SEL)
            .param("matrix", ParamKind::Json, "[a, b, c, d, tx, ty] (Zeilenvektor wie MTXv) oder 9 Werte")
            .opt("center", ParamKind::Point, "Bezugspunkt (Matrix wird um diesen Punkt angewandt)")
            .run(|s, p| {
                let m = matrix_from(p, "matrix")?;
                let m = match p.opt_point("center")? {
                    Some(c) => docops::about(c, &m),
                    None => m,
                };
                transform_selection(s, m)
            }),
    );
    r.add(
        cmd("transform.distort")
            .label("Verzerren")
            .requires(SEL)
            .param("rect", ParamKind::Rect, "Ausgangsrechteck")
            .param("quad", ParamKind::Json, "vier Zielpunkte [[x, y] × 4] (lo, ro, ru, lu)")
            .run(|s, p| {
                let rr = p.rect("rect")?;
                let q = p.points("quad")?;
                if q.len() != 4 {
                    return Err(EngineError::InvalidParams { name: p.command.clone(), message: fp_i18n::t("err-param-four-points") });
                }
                let m = Matrix3::rect_to_quad(&rr, &[q[0], q[1], q[2], q[3]]).ok_or_else(|| EngineError::Failed(fp_i18n::t("err-degenerate-quad")))?;
                transform_selection(s, m)
            }),
    );

    // Anordnen / Ausrichten
    r.add(cmd("modify.arrange.bringToFront").requires(SEL).run(|s, _| arrange(s, "front")));
    r.add(cmd("modify.arrange.bringForward").requires(SEL).run(|s, _| arrange(s, "forward")));
    r.add(cmd("modify.arrange.sendBackward").requires(SEL).run(|s, _| arrange(s, "backward")));
    r.add(cmd("modify.arrange.sendToBack").requires(SEL).run(|s, _| arrange(s, "back")));
    for (name, how) in [
        ("modify.align.left", "left"),
        ("modify.align.centerVertical", "centerVertical"),
        ("modify.align.right", "right"),
        ("modify.align.top", "top"),
        ("modify.align.centerHorizontal", "centerHorizontal"),
        ("modify.align.bottom", "bottom"),
    ] {
        // Menü: wie im Original ab zwei Objekten (an der Leinwand ausrichten: Bedienfeld `pi.align`)
        r.add(cmd(name).requires(&[Requirement::Document, Requirement::MinSelected(2)]).run(move |s, _| align(s, how)));
    }
    r.add(cmd("modify.align.distributeWidths").requires(&[Requirement::Document, Requirement::MinSelected(3)]).run(|s, _| align(s, "distributeWidths")));
    r.add(cmd("modify.align.distributeHeights").requires(&[Requirement::Document, Requirement::MinSelected(3)]).run(|s, _| align(s, "distributeHeights")));

    // Pfade
    r.add(cmd("modify.convertPathToMarquee").requires(&[Requirement::Document, Requirement::AnyOf(ObjectKind::Path)]).run(|s, _| {
        let geo = Arc::clone(&s.geometry_arc());
        let delete = s.prefs.get_bool("DeletePaths", true);
        with_doc(s, |d| {
            let items = selected_path_shapes(d);
            let mut acc: Option<Mask> = None;
            for (_, _, sh) in &items {
                let b = geometry::contours_bounds(&sh.contours).map(|r| r.round_out()).unwrap_or_default();
                let m = geo.coverage(sh, b.inflate(1, 1), true);
                acc = Some(match acc {
                    None => m,
                    Some(a) => a.union(&m)?,
                });
            }
            if delete {
                let ids: Vec<_> = items.iter().map(|(_, e, _)| e.id).collect();
                let page = d.page;
                docops::remove_ids(Arc::make_mut(&mut d.doc), page, &ids);
                d.selection.elements.clear();
            }
            d.selection.pixels = acc;
            Ok(CommandOutput::doc())
        })
    }));
    r.add(cmd("modify.combinePaths.join").requires(SEL).available(|s| s.active().is_some_and(|d| selected_elements(d).iter().any(|(_, e)| matches!(e.kind, ElementKind::Path(_) | ElementKind::Text(_))))).run(|s, _| {
        with_doc(s, join)?;
        Ok(CommandOutput::doc())
    }));
    // nur zusammengesetzte Pfade (mehr als eine Kontur) lassen sich teilen
    r.add(cmd("modify.combinePaths.split").requires(SEL).available(|s| s.active().is_some_and(|d| selected_elements(d).iter().any(|(_, e)| matches!(&e.kind, ElementKind::Path(p) if p.contours.len() > 1)))).run(|s, _| {
        with_doc(s, split)?;
        Ok(CommandOutput::doc())
    }));
    // im Original immer aktiv; ohne zwei Pfade Meldung statt Wirkung
    let paths2 = DOC;
    r.add(cmd("modify.combinePaths.union").requires(paths2).run(|s, _| combine(s, PathBoolOp::Union)));
    r.add(cmd("modify.combinePaths.intersect").requires(paths2).run(|s, _| combine(s, PathBoolOp::Intersect)));
    r.add(cmd("modify.combinePaths.punch").requires(paths2).run(|s, _| combine(s, PathBoolOp::Punch)));
    r.add(cmd("modify.combinePaths.crop").requires(paths2).run(|s, _| combine(s, PathBoolOp::Crop)));
    // im Original immer aktiv; ohne Pfade Meldung statt Wirkung
    let paths1 = DOC;
    r.add(
        cmd("modify.alterPath.simplify")
            .requires(paths1)
            .opt_default("amount", ParamKind::Number { min: Some(0.0), max: Some(100.0) }, json!(1), "Stärke")
            .run(|s, p| {
                let a = p.f64_or("amount", 1.0)?;
                alter_paths(s, move |g, _, c| g.simplify(c, a))
            }),
    );
    r.add(
        cmd("modify.alterPath.expandStroke")
            .requires(paths1)
            .opt("width", ParamKind::Number { min: Some(0.1), max: Some(1000.0) }, "Breite (Vorgabe: Strichbreite)")
            .opt_default("join", ParamKind::Enum { values: vec!["bevel".into(), "round".into(), "miter".into()] }, json!("miter"), "Ecken")
            .opt_default("cap", ParamKind::Enum { values: vec!["butt".into(), "square".into(), "round".into()] }, json!("butt"), "Enden")
            .opt_default("miter", ParamKind::Number { min: Some(1.0), max: Some(10.0) }, json!(10), "Gehrungsgrenze")
            .run(|s, p| {
                let w = p.opt_f64("width")?;
                let join = match p.opt_str("join")?.as_deref() {
                    Some("bevel") => 0,
                    Some("round") => 1,
                    _ => 2,
                };
                let cap = match p.opt_str("cap")?.as_deref() {
                    Some("square") => 1,
                    Some("round") => 2,
                    _ => 0,
                };
                let miter = p.f64_or("miter", 10.0)?;
                alter_paths(s, move |g, e, c| {
                    let width = w.or_else(|| docops::path_attrs_of(e).and_then(|a| a.brush.as_ref().map(|b| b.diameter as f64))).unwrap_or(1.0);
                    g.expand_stroke(c, width, join, cap, miter)
                })
            }),
    );
    r.add(
        cmd("modify.alterPath.insetPath")
            .requires(paths1)
            .opt_default("distance", ParamKind::Number { min: Some(0.0), max: Some(1000.0) }, json!(1), "Abstand in Pixel")
            .opt_default("direction", ParamKind::Enum { values: vec!["inside".into(), "outside".into()] }, json!("inside"), "Richtung")
            .opt_default("join", ParamKind::Enum { values: vec!["bevel".into(), "round".into(), "miter".into()] }, json!("miter"), "Ecken")
            .opt_default("miter", ParamKind::Number { min: Some(4.0), max: Some(10.0) }, json!(10), "Gehrungsgrenze")
            .run(|s, p| {
                let dist = p.f64_or("distance", 1.0)?;
                let expand = p.opt_str("direction")?.as_deref() == Some("outside");
                let join = match p.opt_str("join")?.as_deref() {
                    Some("bevel") => 0,
                    Some("round") => 1,
                    _ => 2,
                };
                let miter = p.f64_or("miter", 10.0)?;
                alter_paths(s, move |g, _, c| g.inset(c, dist, expand, join, miter))
            }),
    );
    for (name, edge) in [("modify.alterPath.hardFill", 0), ("modify.alterPath.antiAliasFill", 1), ("modify.alterPath.featherFill", 2)] {
        r.add(
            cmd(name)
                // wie im Original nur für Texte und freie Pfade (nicht Rechteck-Grundformen)
                .requires(SEL)
                .available(|s| s.active().is_some_and(|d| selected_elements(d).iter().any(|(_, e)| matches!(&e.kind, ElementKind::Text(_) | ElementKind::CompoundShape(_)) || matches!(&e.kind, ElementKind::Path(p) if p.primitive.is_none()))))
                .opt("amount", ParamKind::Integer { min: Some(0), max: Some(1000) }, "weiche Kante in px (nur Weiche-Kante-Füllung)")
                .checked(move |s| {
                    s.active().is_some_and(|d| {
                        super::selected_elements(d).iter().any(|(_, e)| docops::path_attrs_of(e).and_then(|a| a.fill.as_ref().map(|f| f.edge)) == Some(edge))
                    })
                })
                .run(move |s, p| {
                    let amount = p.i64_or("amount", 10)?.clamp(0, 1000) as i32;
                    with_doc(s, |d| {
                        let (page, ids) = (d.page, d.selection.elements.clone());
                        let doc = Arc::make_mut(&mut d.doc);
                        for id in ids {
                            docops::edit_by_id(doc, page, id, |e| {
                                docops::edit_path_attrs(e, &|a| {
                                    if let Some(f) = &mut a.fill {
                                        f.edge = edge;
                                        if edge == 2 {
                                            f.feather = amount;
                                        }
                                    }
                                });
                            });
                        }
                        Ok(CommandOutput::doc())
                    })
                }),
        );
    }
    r.add(cmd("modify.snapToPixel").requires(&[Requirement::Document, Requirement::AnyOf(ObjectKind::Path)]).run(|s, _| {
        with_doc(s, snap_to_pixel)?;
        Ok(CommandOutput::doc())
    }));
    r.add(cmd("modify.group").requires(SEL).run(|s, _| {
        with_doc(s, |d| {
            let id = group(d, false)?;
            Ok(CommandOutput::doc_with(json!({"id": id.0})))
        })
    }));
    // Gruppen auflösen; Rechteck-Grundformen werden wie im Original zu freien Pfaden
    r.add(cmd("modify.ungroup").requires(SEL).available(|s| s.active().is_some_and(|d| selected_elements(d).iter().any(|(_, e)| matches!(e.kind, ElementKind::Group(_)) || matches!(&e.kind, ElementKind::Path(p) if p.primitive.is_some())))).run(|s, _| {
        with_doc(s, |d| {
            let page = d.page;
            let prims: Vec<ElementId> = selected_elements(d).into_iter().filter(|(_, e)| matches!(&e.kind, ElementKind::Path(p) if p.primitive.is_some())).map(|(_, e)| e.id).collect();
            if !prims.is_empty() {
                let doc = Arc::make_mut(&mut d.doc);
                for id in &prims {
                    docops::edit_by_id(doc, page, *id, |e| {
                        if let ElementKind::Path(p) = &mut e.kind {
                            p.primitive = None;
                        }
                    });
                }
            }
            let has_groups = selected_elements(d).iter().any(|(_, e)| matches!(e.kind, ElementKind::Group(_)));
            if !has_groups {
                return Ok(CommandOutput::doc_with(json!({"ids": ids_json(&prims)})));
            }
            let ids = ungroup(d, false)?;
            Ok(CommandOutput::doc_with(json!({"ids": ids_json(&ids)})))
        })
    }));
    r.add(cmd("modify.editBitmap").requires(&[Requirement::Document, Requirement::AnyOf(ObjectKind::Bitmap)]).run(|s, _| {
        let d = s.doc_mut()?;
        d.bitmap_mode = true;
        Ok(CommandOutput::session())
    }));
    r.add(cmd("modify.exitBitmapMode").requires(DOC).available(|s| s.active().is_some_and(|d| d.bitmap_mode || d.selection.has_pixels())).run(|s, _| {
        with_doc(s, |d| {
            d.bitmap_mode = false;
            d.selection.pixels = None;
            Ok(CommandOutput::doc())
        })
    }));
    r.add(
        cmd("path.movePoints")
            .label("Punkte verschieben")
            .requires(DOC)
            .param("dx", ParamKind::Number { min: None, max: None }, "Δx")
            .param("dy", ParamKind::Number { min: None, max: None }, "Δy")
            .available(|s| s.active().is_some_and(|d| !d.selection.points.is_empty()))
            .run(|s, p| {
                let (dx, dy) = (p.f64("dx")? as f32, p.f64("dy")? as f32);
                with_doc(s, |d| {
                    let (page, pts) = (d.page, d.selection.points.clone());
                    let doc = Arc::make_mut(&mut d.doc);
                    for pr in pts {
                        docops::edit_by_id(doc, page, pr.element, |e| {
                            if let ElementKind::Path(pe) = &mut e.kind
                                && let Some(n) = pe.contours.get_mut(pr.contour).and_then(|c| c.nodes.get_mut(pr.node))
                            {
                                for q in [&mut n.anchor, &mut n.pred, &mut n.succ] {
                                    q.x += dx;
                                    q.y += dy;
                                }
                                pe.primitive = None;
                                pe.polyline = None;
                            }
                        });
                    }
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(
        cmd("path.setContours")
            .label("Pfad nachzeichnen")
            .requires(DOC)
            .opt("id", ParamKind::Integer { min: Some(0), max: None }, "Pfad (Vorgabe: ausgewählter Pfad)")
            .param("contours", ParamKind::Json, "Konturen wie bei draw.path")
            .run(|s, p| {
                let contours = super::draw::parse_contours(p)?;
                with_doc(s, |d| {
                    let id = match p.opt_i64("id")? {
                        Some(i) => ElementId(i as u64),
                        None => d.selection.elements.first().copied().ok_or_else(|| sel_err("err-sel-no-path"))?,
                    };
                    let page = d.page;
                    docops::edit_by_id(Arc::make_mut(&mut d.doc), page, id, |e| set_contours(e, contours, None)).ok_or(EngineError::ElementNotFound(id.0))?;
                    Ok(CommandOutput::doc())
                })
            }),
    );
}

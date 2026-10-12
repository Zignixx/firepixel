//! Bausteine für Dokumentänderungen: Elemente suchen, einfügen, entfernen, umordnen, Grenzen,
//! Transformationen, Attribute, Bitmaps, Ebenen, Status und Seiten.
//!
//! Alle Funktionen arbeiten Copy-on-Write auf [`fp_model::Document`] (Änderung über
//! `Arc::make_mut`); Render-Caches der geänderten Elemente und ihrer Vorfahren werden verworfen
//! ([`Element::edit`]). Befehle in [`crate::commands`] setzen sich daraus zusammen.

use crate::geometry::{self, GeometryOps, PathShape};
use fp_geom::{IRect, Matrix3, Point, Rect};
use fp_model::element::{BitmapElem, Element, ElementKind};
use fp_model::layer::{Cel, Layer, LayerKind, LayerSlot};
use fp_model::traverse::{walk_root, ElementPath, PathStep, RootRef};
use fp_model::{Document, ElementId, LayerId, PathAttrs, TileGrid};
use fp_raster::{RgbaImage, Sampler};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// Zähler für neue Kachel-IDs (ein Schreiber nummeriert ohnehin neu).
static TILE_IDS: AtomicU32 = AtomicU32::new(0x4000_0000);

/// Nächste freie Kachel-ID-Basis für [`TileGrid::from_rgba`].
pub fn tile_counter() -> u32 {
    TILE_IDS.fetch_add(1 << 16, Ordering::Relaxed)
}

// ------------------------------------------------------------------ Suchen

/// Adresse eines Elements auf einer Seite (auch in über Seiten geteilten Ebenen dieser Seite).
pub fn find_on_page(doc: &Document, page: usize, id: ElementId) -> Option<ElementPath> {
    let p = doc.pages.get(page)?;
    let mut found = None;
    walk_root(&p.root, RootRef::Page(page), &mut |_, path, e| {
        if found.is_none() && e.id == id {
            found = Some(path.clone());
        }
    });
    if found.is_some() {
        return found;
    }
    // geteilte Ebenen: Adresse mit `RootRef::SharedLayer`, `slot` = Platz auf der Seite (Stapelfolge)
    let refs: Vec<(usize, usize)> = p.root.layers.iter().enumerate().filter_map(|(slot, s)| match s {
        LayerSlot::Shared(r) => shared_def_index(doc, &r.id).map(|i| (i, slot)),
        _ => None,
    }).collect();
    if refs.is_empty() {
        return None;
    }
    doc.walk_elements(|_, path, e| {
        if found.is_none()
            && e.id == id
            && let RootRef::SharedLayer(i) = path.root
            && let Some((_, slot)) = refs.iter().find(|(d, _)| *d == i)
        {
            let mut p = path.clone();
            p.slot = *slot;
            found = Some(p);
        }
    });
    found
}

/// Ist das Element an `path` gesperrt (das oberste Objekt selbst oder seine Ebene im Status)?
pub fn is_locked(doc: &Document, page: usize, path: &ElementPath) -> bool {
    let top = ElementPath { steps: path.steps.iter().take(1).cloned().collect(), ..path.clone() };
    if doc.element_at(&top).is_some_and(|e| e.common.locked) || doc.element_at(path).is_some_and(|e| e.common.locked) {
        return true;
    }
    own_layers(doc, page).into_iter().find(|(s, _)| *s == path.slot).and_then(|(_, l)| l.cels.get(path.cel).map(|c| c.locked)).unwrap_or(false)
}

/// Index der Definition einer geteilten Ebene in `doc.shared_layers`.
pub fn shared_def_index(doc: &Document, id: &fp_model::Guid) -> Option<usize> {
    doc.shared_layers.iter().position(|d| d.id.same_as(id))
}

/// Adresse eines Elements in Ebene `slot` der Seite (eigene oder geteilte Ebene).
pub fn slot_path(doc: &Document, page: usize, slot: usize, cel: usize, steps: Vec<PathStep>) -> ElementPath {
    let root = match doc.pages.get(page).and_then(|p| p.root.layers.get(slot)) {
        Some(LayerSlot::Shared(r)) => shared_def_index(doc, &r.id).map(RootRef::SharedLayer).unwrap_or(RootRef::Page(page)),
        _ => RootRef::Page(page),
    };
    ElementPath { root, slot, cel, steps }
}

/// Element zu einer ID auf einer Seite.
pub fn element_on_page(doc: &Document, page: usize, id: ElementId) -> Option<Arc<Element>> {
    find_on_page(doc, page, id).and_then(|p| doc.element_at(&p).cloned())
}

/// Adressen vieler IDs einer Seite in **einem** Durchlauf (PERF-1): je ID dasselbe Ergebnis wie
/// [`find_on_page`], aber bei großer Auswahl O(Objekte) statt O(Objekte × Auswahl).
pub fn find_many_on_page(doc: &Document, page: usize, ids: &[ElementId]) -> Vec<Option<ElementPath>> {
    if ids.len() < 4 {
        return ids.iter().map(|id| find_on_page(doc, page, *id)).collect();
    }
    let Some(p) = doc.pages.get(page) else { return vec![None; ids.len()] };
    let wanted: std::collections::HashSet<ElementId> = ids.iter().copied().collect();
    let mut found: std::collections::HashMap<ElementId, ElementPath> = std::collections::HashMap::with_capacity(wanted.len());
    walk_root(&p.root, RootRef::Page(page), &mut |_, path, e| {
        if wanted.contains(&e.id) && !found.contains_key(&e.id) {
            found.insert(e.id, path.clone());
        }
    });
    if found.len() < wanted.len() {
        // geteilte Ebenen wie in `find_on_page`
        let refs: Vec<(usize, usize)> = p.root.layers.iter().enumerate().filter_map(|(slot, s)| match s {
            LayerSlot::Shared(r) => shared_def_index(doc, &r.id).map(|i| (i, slot)),
            _ => None,
        }).collect();
        if !refs.is_empty() {
            let mut shared: std::collections::HashMap<ElementId, ElementPath> = std::collections::HashMap::new();
            doc.walk_elements(|_, path, e| {
                if wanted.contains(&e.id)
                    && !found.contains_key(&e.id)
                    && !shared.contains_key(&e.id)
                    && let RootRef::SharedLayer(i) = path.root
                    && let Some((_, slot)) = refs.iter().find(|(d, _)| *d == i)
                {
                    let mut p = path.clone();
                    p.slot = *slot;
                    shared.insert(e.id, p);
                }
            });
            found.extend(shared);
        }
    }
    ids.iter().map(|id| found.get(id).cloned()).collect()
}

/// Elemente vieler IDs einer Seite in einem Durchlauf, in der Reihenfolge von `ids`; fehlende
/// fallen weg (wie `ids.iter().filter_map(|id| element_on_page(…))`, PERF-1).
pub fn elements_on_page(doc: &Document, page: usize, ids: &[ElementId]) -> Vec<Arc<Element>> {
    find_many_on_page(doc, page, ids).into_iter().flatten().filter_map(|p| doc.element_at(&p).cloned()).collect()
}

/// Liegt das Element direkt in einem Cel (nicht in einer Gruppe)?
pub fn is_top_level(path: &ElementPath) -> bool {
    path.steps.len() == 1
}

/// Sortierschlüssel der Stapelreihenfolge: kleiner = weiter oben (obere Ebene zuerst, innerhalb einer
/// Liste kleinerer Index zuerst).
pub fn z_key(path: &ElementPath) -> (std::cmp::Reverse<usize>, usize, Vec<usize>) {
    let idx = path
        .steps
        .iter()
        .map(|s| match s {
            PathStep::Child(i) | PathStep::Replacement(i) | PathStep::SubLayerCel(i) => *i,
            _ => 0,
        })
        .collect();
    (std::cmp::Reverse(path.slot), path.cel, idx)
}

/// Adressen der ausgewählten Elemente auf einer Seite, oben zuerst.
pub fn paths_sorted(doc: &Document, page: usize, ids: &[ElementId]) -> Vec<ElementPath> {
    let mut v: Vec<ElementPath> = find_many_on_page(doc, page, ids).into_iter().flatten().collect();
    v.sort_by_key(z_key);
    v
}

// ------------------------------------------------------------------ Zugriff

/// Veränderlicher Cel einer eigenen Ebene (Copy-on-Write).
pub fn cel_mut(doc: &mut Document, page: usize, slot: usize, cel: usize) -> Option<&mut Cel> {
    let l = layer_mut(doc, page, slot)?;
    Some(Arc::make_mut(l.cels.get_mut(cel)?))
}

fn descend_list<'a>(list: &'a mut Vec<Arc<Element>>, steps: &[PathStep]) -> Option<&'a mut Vec<Arc<Element>>> {
    match steps {
        [] => Some(list),
        [PathStep::Child(k), PathStep::SubLayerCel(c), rest @ ..] => {
            let e = Element::edit(list.get_mut(*k)?);
            let ElementKind::SubLayer(l) = &mut e.kind else { return None };
            let cel = Arc::make_mut(Arc::make_mut(l).cels.get_mut(*c)?);
            drop_order(&mut cel.elements_extra);
            descend_list(&mut cel.elements, rest)
        }
        [PathStep::Child(k), rest @ ..] => {
            let e = Element::edit(list.get_mut(*k)?);
            match &mut e.kind {
                ElementKind::Group(g) => {
                    drop_order(&mut g.children_extra);
                    descend_list(&mut g.children, rest)
                }
                ElementKind::CompoundShape(c) => {
                    drop_order(&mut c.operands_extra);
                    descend_list(&mut c.operands, rest)
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// Verwirft die gespeicherte Tag-Reihenfolge einer Elementliste, wenn sie keine Rohknoten einordnen
/// muss: nach Einfügen/Entfernen/Umordnen passt sie nicht mehr zur Liste, und der Schreiber würde
/// die Elemente nach Tag-Art umsortieren (z. B. Textkopie unter die Pfade).
fn drop_order(x: &mut fp_model::Extra) {
    if x.nodes.is_empty() {
        x.order = None;
    }
}

/// Elternliste eines Elements (Cel, Gruppenkinder, Operanden, Unterebene) und sein Index darin.
/// Die Render-Caches aller Vorfahren werden verworfen.
pub fn parent_list_mut<'a>(doc: &'a mut Document, path: &ElementPath) -> Option<(&'a mut Vec<Arc<Element>>, usize)> {
    let (last, prefix) = path.steps.split_last()?;
    let PathStep::Child(idx) = *last else { return None };
    let cel = match path.root {
        RootRef::Page(pi) => cel_mut(doc, pi, path.slot, path.cel)?,
        RootRef::SharedLayer(i) => {
            doc.root_copy = None;
            let l = Arc::make_mut(doc.shared_layers.get_mut(i)?.layer.as_mut()?);
            Arc::make_mut(l.cels.get_mut(path.cel)?)
        }
        _ => return None,
    };
    drop_order(&mut cel.elements_extra);
    let list = descend_list(&mut cel.elements, prefix)?;
    Some((list, idx))
}

/// Ändert ein Element (Render-Cache des Elements und der Vorfahren wird verworfen).
pub fn edit_at<R>(doc: &mut Document, path: &ElementPath, f: impl FnOnce(&mut Element) -> R) -> Option<R> {
    if let Some((list, idx)) = parent_list_mut(doc, path) {
        let e = Element::edit(list.get_mut(idx)?);
        return Some(f(e));
    }
    let e = doc.element_at_mut(path)?;
    e.cache = None;
    Some(f(e))
}

/// Ändert ein Element per ID auf einer Seite.
pub fn edit_by_id<R>(doc: &mut Document, page: usize, id: ElementId, f: impl FnOnce(&mut Element) -> R) -> Option<R> {
    let path = find_on_page(doc, page, id)?;
    edit_at(doc, &path, f)
}

/// Entfernt ein Element und liefert es.
pub fn remove_at(doc: &mut Document, path: &ElementPath) -> Option<Arc<Element>> {
    let (list, idx) = parent_list_mut(doc, path)?;
    if idx < list.len() { Some(list.remove(idx)) } else { None }
}

/// Entfernt Elemente per ID (oben zuerst) und liefert sie mit ihrer bisherigen Adresse.
pub fn remove_ids(doc: &mut Document, page: usize, ids: &[ElementId]) -> Vec<(ElementPath, Arc<Element>)> {
    let mut out = Vec::new();
    // von unten nach oben entfernen, damit Indizes gültig bleiben
    let mut paths = paths_sorted(doc, page, ids);
    paths.reverse();
    for p in paths {
        if let Some(e) = remove_at(doc, &p) {
            out.push((p, e));
        }
    }
    out.reverse();
    out
}

// ------------------------------------------------------------------ Ebenen

/// Bearbeitbare Ebenen einer Seite: (Slot, Ebene), unten → oben – eigene Ebenen und über Seiten
/// geteilte Ebenen (deren Inhalt in `doc.shared_layers` liegt).
pub fn own_layers(doc: &Document, page: usize) -> Vec<(usize, Arc<Layer>)> {
    doc.pages
        .get(page)
        .map(|p| {
            p.root
                .layers
                .iter()
                .enumerate()
                .filter_map(|(i, s)| match s {
                    LayerSlot::Own(l) => Some((i, Arc::clone(l))),
                    LayerSlot::Shared(r) => doc.shared_layer(&r.id).and_then(|d| d.layer.clone()).map(|l| (i, l)),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Slot einer Ebene.
pub fn layer_slot(doc: &Document, page: usize, id: LayerId) -> Option<usize> {
    own_layers(doc, page).into_iter().find(|(_, l)| l.id == id).map(|(s, _)| s)
}

/// Veränderliche Ebene.
pub fn layer_mut(doc: &mut Document, page: usize, slot: usize) -> Option<&mut Layer> {
    let shared = match doc.pages.get(page)?.root.layers.get(slot)? {
        LayerSlot::Shared(r) => Some(shared_def_index(doc, &r.id)?),
        LayerSlot::Own(_) => None,
        _ => return None,
    };
    if let Some(i) = shared {
        doc.root_copy = None;
        return Some(Arc::make_mut(doc.shared_layers.get_mut(i)?.layer.as_mut()?));
    }
    let p = doc.page_mut(page)?;
    match p.root.layers.get_mut(slot)? {
        LayerSlot::Own(l) => Some(Arc::make_mut(l)),
        _ => None,
    }
}

/// Oberste normale (nicht Web-)Ebene.
pub fn top_normal_layer(doc: &Document, page: usize) -> Option<LayerId> {
    own_layers(doc, page).into_iter().rev().find(|(_, l)| l.kind == LayerKind::Normal).map(|(_, l)| l.id)
}

/// Aktive Ebene nach einem Seitenwechsel: die bisherige, wenn es sie auf der Seite gibt (über Seiten
/// geteilte Ebene), sonst die oberste normale Ebene.
pub fn layer_on_page(doc: &Document, page: usize, current: Option<LayerId>) -> Option<LayerId> {
    match current {
        Some(c) if own_layers(doc, page).iter().any(|(_, l)| l.id == c) => Some(c),
        _ => top_normal_layer(doc, page),
    }
}

/// Web-Ebene der Seite.
pub fn web_layer(doc: &Document, page: usize) -> Option<LayerId> {
    own_layers(doc, page).into_iter().find(|(_, l)| l.kind == LayerKind::Web).map(|(_, l)| l.id)
}

/// Fügt ein Element in eine Ebene ein (`index` 0 = ganz oben). Web-Objekte landen immer in der
/// Web-Ebene, andere Elemente nie (dann oberste normale Ebene). Liefert die Adresse.
pub fn insert_element(doc: &mut Document, page: usize, frame: usize, layer: Option<LayerId>, index: usize, e: Arc<Element>) -> Option<ElementPath> {
    let is_web = matches!(e.kind, ElementKind::WebObject(_));
    let target = if is_web {
        web_layer(doc, page).or(layer)
    } else {
        match layer.and_then(|l| own_layers(doc, page).into_iter().find(|(_, x)| x.id == l)) {
            Some((_, l)) if l.kind == LayerKind::Normal => Some(l.id),
            _ => top_normal_layer(doc, page),
        }
    };
    let target = match target {
        Some(t) => t,
        None => {
            // Seite ohne passende Ebene: eine anlegen
            let frames = doc.pages.get(page)?.root.frame_count as usize;
            let mut ids = std::mem::take(&mut doc.ids);
            let l = Layer::new(&mut ids, &if is_web { fp_i18n::t("msg-default-web-layer") } else { fp_i18n::tr!("msg-default-layer", n = "1") }, if is_web { LayerKind::Web } else { LayerKind::Normal }, frames);
            doc.ids = ids;
            let id = l.id;
            let p = doc.page_mut(page)?;
            p.root.layers.push(LayerSlot::Own(Arc::new(l)));
            id
        }
    };
    let slot = layer_slot(doc, page, target)?;
    let frames = doc.pages.get(page)?.root.frame_count as usize;
    let l = layer_mut(doc, page, slot)?;
    let cel_index = if l.shared_across_frames { 0 } else { frame.min(frames.saturating_sub(1)) };
    while l.cels.len() <= cel_index {
        l.cels.push(Arc::new(Cel::empty()));
    }
    let cel = Arc::make_mut(l.cels.get_mut(cel_index)?);
    drop_order(&mut cel.elements_extra);
    let i = index.min(cel.elements.len());
    cel.elements.insert(i, e);
    Some(slot_path(doc, page, slot, cel_index, vec![PathStep::Child(i)]))
}

/// Neue Laufzeit-IDs aus dem Dokument-Zähler vergeben (rekursiv).
pub fn reassign(doc: &mut Document, e: &Element) -> Element {
    let mut ids = std::mem::take(&mut doc.ids);
    let mut d = e.duplicate(&mut ids);
    doc.ids = ids;
    detach_text_stories(&mut d);
    d
}

/// Löst Textkopien von der gemeinsamen ATE-Story des Originals (sonst teilen sich Original und
/// Kopie beim Speichern eine Story, und die Kopie landet beim Lesen an der falschen Stelle).
pub fn detach_text_stories(e: &mut Element) {
    match &mut e.kind {
        ElementKind::Text(t) => {
            if t.story_index.is_some() {
                t.detach_story();
            }
        }
        ElementKind::Group(g) => {
            if let Some(m) = g.mask.as_mut() {
                detach_text_stories(Element::edit(m));
            }
            for c in g.children.iter_mut() {
                detach_text_stories(Element::edit(c));
            }
        }
        _ => {}
    }
}

/// Neue Element-ID.
pub fn new_id(doc: &mut Document) -> ElementId {
    doc.ids.element()
}

// ------------------------------------------------------------------ Grenzen

fn union_opt(a: Option<Rect>, b: Option<Rect>) -> Option<Rect> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.union(&y)),
        (x, None) => x,
        (None, y) => y,
    }
}

/// Textrand je Seite: Das Original zeigt Texte (Inspektor, Info, Auswahlrahmen, JSF `left/top/width/height`)
/// mit 2 px Rand um den gespeicherten Textrahmen (Rahmen 50,170–200,194 → X 48, Y 168, B 154, H 28).
/// Der gespeicherte Rahmen bleibt exakt (re\09 §5.3, JSF `rawLeft/rawTop/rawWidth/rawHeight`).
pub const TEXT_MARGIN: f32 = 2.0;

/// Sichtbare Grenzen eines Textrahmens (ohne Transformation): Rahmen + [`TEXT_MARGIN`] je Seite.
pub fn text_visible_frame(frame: &Rect) -> Rect {
    Rect::new(frame.left - TEXT_MARGIN, frame.top - TEXT_MARGIN, frame.right + TEXT_MARGIN, frame.bottom + TEXT_MARGIN)
}

/// Grenzen eines Elements in Dokumentkoordinaten (ohne Strich- und Effektausdehnung).
/// Texte mit [`TEXT_MARGIN`] (sichtbare Grenzen wie im Original).
pub fn element_bounds(doc: &Document, e: &Element) -> Option<Rect> {
    bounds_depth(doc, e, 0)
}

fn bounds_depth(doc: &Document, e: &Element, depth: u32) -> Option<Rect> {
    if depth > 24 {
        return None;
    }
    match &e.kind {
        ElementKind::Path(p) => geometry::contours_bounds(&p.contours),
        ElementKind::Text(t) => {
            let b = text_visible_frame(&t.bounds);
            Some(match t.transform {
                Some(m) if !m.is_identity() => m.transform_rect_bounds(&b),
                _ => b,
            })
        }
        ElementKind::Bitmap(b) => {
            let p = b.image.position();
            Some(Rect::from_xywh(p.x, p.y, b.image.grid.width as f32, b.image.grid.height as f32))
        }
        ElementKind::Group(g) => {
            let mut r = None;
            for c in &g.children {
                r = union_opt(r, bounds_depth(doc, c, depth + 1));
            }
            if g.kind.is_mask() && g.mask_flags.enabled
                && let (Some(cr), Some(m)) = (r, g.effective_mask())
                && let Some(mr) = bounds_depth(doc, m, depth + 1)
            {
                let i = cr.intersect(&mr);
                r = Some(if i.is_empty() { mr } else { i });
            }
            r
        }
        ElementKind::CompoundShape(c) => match &c.result {
            Some(res) => bounds_depth(doc, res, depth + 1),
            None => c.operands.iter().fold(None, |acc, o| union_opt(acc, bounds_depth(doc, o, depth + 1))),
        },
        ElementKind::Instance(i) => {
            let (_, sym) = doc.symbol_by_id(i.symbol_id)?;
            let mut r = None;
            for s in &sym.root.layers {
                if let LayerSlot::Own(l) = s
                    && l.kind == LayerKind::Normal
                    && let Some(cel) = l.cels.first()
                {
                    for c in &cel.elements {
                        r = union_opt(r, bounds_depth(doc, c, depth + 1));
                    }
                }
            }
            let b = r?;
            let base = i.transform.transform_rect_bounds(&b);
            if sym.kind() != fp_model::SymbolKind::Animation || i.anim.frames <= 1 {
                return Some(base);
            }
            // Animationsinstanz: Grenzen schließen den Bewegungsweg ein (alle Status, wie das
            // Original `pixelRect` meldet, PM-6 D28); Zustand je Status wie beim Zeichnen
            // (Skalierung/Drehung um die Symbolmitte, dann Instanzmatrix, dann Versatz)
            let c = b.center();
            let mut out = base;
            for k in 0..i.anim.frames.min(360) as usize {
                let Some(af) = i.anim.frame(k, sym.root.frame_count) else { continue };
                let local = Matrix3::translation(-c.x, -c.y)
                    .then(&Matrix3::scaling(af.scale as f32, af.scale as f32))
                    .then(&Matrix3::rotation_degrees(af.rotation))
                    .then(&Matrix3::translation(c.x, c.y));
                let m = local.then(&i.transform).then(&Matrix3::translation(af.offset.x, af.offset.y));
                out = out.union(&m.transform_rect_bounds(&b));
            }
            Some(out)
        }
        ElementKind::WebObject(w) => match &w.polygon {
            Some(c) if !c.nodes.is_empty() => geometry::contours_bounds(std::slice::from_ref(c)),
            _ => Some(w.bounds),
        },
        ElementKind::SubLayer(l) => {
            let mut r = None;
            if let Some(cel) = l.cels.first() {
                for c in &cel.elements {
                    r = union_opt(r, bounds_depth(doc, c, depth + 1));
                }
            }
            r
        }
        ElementKind::Raw(_) => None,
    }
}

/// Vereinigte Grenzen mehrerer Elemente einer Seite.
pub fn ids_bounds(doc: &Document, page: usize, ids: &[ElementId]) -> Option<Rect> {
    elements_on_page(doc, page, ids).iter().fold(None, |acc, e| union_opt(acc, element_bounds(doc, e)))
}

/// Grenzen aller Elemente einer Seite (alle Status, ohne Web-Ebene).
pub fn page_content_bounds(doc: &Document, page: usize) -> Option<Rect> {
    let mut r = None;
    for (_, l) in own_layers(doc, page) {
        if l.kind == LayerKind::Web {
            continue;
        }
        for cel in &l.cels {
            for e in &cel.elements {
                r = union_opt(r, element_bounds(doc, e));
            }
        }
    }
    r
}

// ------------------------------------------------------------------ Transformationen

/// Verschiebt ein Element um (dx, dy). Render-Caches werden mitverschoben, nicht verworfen.
pub fn translate_element(e: &mut Element, dx: f32, dy: f32) {
    if dx == 0.0 && dy == 0.0 {
        return;
    }
    e.move_cache(dx, dy);
    let mv = |p: &mut Point| {
        p.x += dx;
        p.y += dy;
    };
    let t = Matrix3::translation(dx, dy);
    match &mut e.kind {
        ElementKind::Path(p) => {
            for c in &mut p.contours {
                for n in &mut c.nodes {
                    mv(&mut n.anchor);
                    mv(&mut n.pred);
                    mv(&mut n.succ);
                }
            }
            if let Some(pl) = &mut p.polyline {
                for l in pl {
                    for q in &mut l.points {
                        mv(&mut q.pos);
                    }
                }
            }
            mv(&mut p.fill_handles.start);
            mv(&mut p.fill_handles.end);
            mv(&mut p.fill_handles.second);
            mv(&mut p.texture_offset);
            if let Some(pr) = &mut p.primitive {
                pr.transform = pr.transform.then(&t);
                mv(&mut pr.fill_handles.start);
                mv(&mut pr.fill_handles.end);
                mv(&mut pr.fill_handles.second);
            }
        }
        ElementKind::Text(tx) => {
            tx.bounds = tx.bounds.translate(dx, dy);
            if let Some(a) = &mut tx.anchor_rect {
                a.left += dx;
                a.top += dy;
                if a.right != 0.0 || a.bottom != 0.0 {
                    a.right += dx;
                    a.bottom += dy;
                }
            }
            mv(&mut tx.fill_handles.start);
            mv(&mut tx.fill_handles.end);
            mv(&mut tx.fill_handles.second);
            mv(&mut tx.texture_offset);
            if let Some(m) = &mut tx.transform {
                *m = m.then(&t);
            }
            if let Some(op) = &mut tx.on_path
                && let Some(pe) = &mut op.path
            {
                translate_element(Arc::make_mut(pe), dx, dy);
            }
        }
        ElementKind::Bitmap(b) => {
            mv(&mut b.image.origin);
            if let Some(r) = &mut b.effect_result {
                mv(&mut Arc::make_mut(r).bitmap.image.origin);
            }
        }
        ElementKind::Group(g) => {
            if let Some(m) = &mut g.mask {
                translate_element(Arc::make_mut(m), dx, dy);
            }
            for c in &mut g.children {
                translate_element(Arc::make_mut(c), dx, dy);
            }
            if let Some(r) = &mut g.replacement {
                for c in r {
                    translate_element(Arc::make_mut(c), dx, dy);
                }
            }
            if let Some(s) = &mut g.smart_shape {
                if let Some(m) = &mut s.transform {
                    *m = m.then(&t);
                }
                if let Some(cps) = &mut s.control_points {
                    for cp in cps {
                        mv(&mut cp.position);
                    }
                }
            }
        }
        ElementKind::CompoundShape(c) => {
            for x in c.result.iter_mut().chain(c.operands.iter_mut()).chain(c.proxy.iter_mut()) {
                translate_element(Arc::make_mut(x), dx, dy);
            }
        }
        ElementKind::Instance(i) => {
            i.transform = i.transform.then(&t);
            if let Some(a) = &mut i.alt_transform {
                *a = a.then(&t);
            }
        }
        ElementKind::WebObject(w) => {
            w.bounds = w.bounds.translate(dx, dy);
            if let Some(c) = &mut w.polygon {
                for n in &mut c.nodes {
                    mv(&mut n.anchor);
                    mv(&mut n.pred);
                    mv(&mut n.succ);
                }
            }
        }
        ElementKind::SubLayer(l) => {
            let l = Arc::make_mut(l);
            for cel in &mut l.cels {
                for x in &mut Arc::make_mut(cel).elements {
                    translate_element(Arc::make_mut(x), dx, dy);
                }
            }
        }
        ElementKind::Raw(_) => {}
    }
}

/// Bitmap als RGBA-Bild in Dokumentkoordinaten (Lage auf ganze Pixel gerundet).
pub fn bitmap_image(b: &BitmapElem) -> Option<RgbaImage> {
    let (rgba, _) = b.image.to_rgba();
    let p = b.image.position();
    let r = IRect::from_xywh(p.x.round() as i32, p.y.round() as i32, b.image.grid.width as i32, b.image.grid.height as i32);
    RgbaImage::from_rgba_bytes(r, &rgba).ok()
}

/// Ersetzt die Pixel einer Bitmap (Lage aus den Bildgrenzen, Effekt-Ergebnis verworfen).
pub fn set_bitmap_image(b: &mut BitmapElem, img: &RgbaImage) {
    let r = img.bounds();
    let mut next = tile_counter();
    b.image.origin = Point::new(r.left as f32, r.top as f32);
    b.image.offset = Point::default();
    b.image.grid = TileGrid::from_rgba(img.width() as u32, img.height() as u32, img.as_bytes(), &mut next);
    b.effect_result = None;
}

fn apply_pt(m: &Matrix3, p: &mut Point) {
    *p = m.apply(*p);
}

/// Transformiert ein Element mit einer Matrix (Zeilenvektor-Konvention). Reine Verschiebungen
/// laufen über [`translate_element`]; Bitmaps werden mit `sampler` neu abgetastet.
pub fn transform_element(e: &mut Element, m: &Matrix3, sampler: Sampler) {
    if m.is_translation(1e-6) {
        translate_element(e, m.m[2][0], m.m[2][1]);
        return;
    }
    e.cache = None;
    match &mut e.kind {
        ElementKind::Path(p) => {
            for c in &mut p.contours {
                for n in &mut c.nodes {
                    apply_pt(m, &mut n.anchor);
                    apply_pt(m, &mut n.pred);
                    apply_pt(m, &mut n.succ);
                }
            }
            p.polyline = None;
            apply_pt(m, &mut p.fill_handles.start);
            apply_pt(m, &mut p.fill_handles.end);
            apply_pt(m, &mut p.fill_handles.second);
            apply_pt(m, &mut p.texture_offset);
            if let Some(pr) = &mut p.primitive {
                pr.transform = pr.transform.then(m);
                apply_pt(m, &mut pr.fill_handles.start);
                apply_pt(m, &mut pr.fill_handles.end);
                apply_pt(m, &mut pr.fill_handles.second);
            }
        }
        ElementKind::Text(t) => {
            t.transform = Some(t.transform.unwrap_or(Matrix3::IDENTITY).then(m));
            if let Some(op) = &mut t.on_path
                && let Some(pe) = &mut op.path
            {
                transform_element(Arc::make_mut(pe), m, sampler);
            }
        }
        ElementKind::Bitmap(b) => {
            if let Some(img) = bitmap_image(b) {
                let out = fp_raster::resample::transform(&img, m, sampler, None);
                set_bitmap_image(b, &out);
            }
        }
        ElementKind::Group(g) => {
            if let Some(x) = &mut g.mask {
                transform_element(Arc::make_mut(x), m, sampler);
            }
            for c in &mut g.children {
                transform_element(Arc::make_mut(c), m, sampler);
            }
            if let Some(r) = &mut g.replacement {
                for c in r {
                    transform_element(Arc::make_mut(c), m, sampler);
                }
            }
            if let Some(s) = &mut g.smart_shape {
                if let Some(t) = &mut s.transform {
                    *t = t.then(m);
                }
                if let Some(cps) = &mut s.control_points {
                    for cp in cps {
                        apply_pt(m, &mut cp.position);
                    }
                }
            }
        }
        ElementKind::CompoundShape(c) => {
            for x in c.result.iter_mut().chain(c.operands.iter_mut()).chain(c.proxy.iter_mut()) {
                transform_element(Arc::make_mut(x), m, sampler);
            }
        }
        ElementKind::Instance(i) => {
            i.transform = i.transform.then(m);
            if let Some(a) = &mut i.alt_transform {
                *a = a.then(m);
            }
        }
        ElementKind::WebObject(w) => {
            w.bounds = m.transform_rect_bounds(&w.bounds);
            if let Some(c) = &mut w.polygon {
                for n in &mut c.nodes {
                    apply_pt(m, &mut n.anchor);
                    apply_pt(m, &mut n.pred);
                    apply_pt(m, &mut n.succ);
                }
            }
        }
        ElementKind::SubLayer(l) => {
            let l = Arc::make_mut(l);
            for cel in &mut l.cels {
                for x in &mut Arc::make_mut(cel).elements {
                    transform_element(Arc::make_mut(x), m, sampler);
                }
            }
        }
        ElementKind::Raw(_) => {}
    }
}

/// Matrix „Rechteck `from` auf Rechteck `to` abbilden“ (Skalieren + Verschieben).
pub fn rect_to_rect(from: Rect, to: Rect) -> Matrix3 {
    let sx = if from.width() != 0.0 { to.width() / from.width() } else { 1.0 };
    let sy = if from.height() != 0.0 { to.height() / from.height() } else { 1.0 };
    Matrix3::translation(-from.left, -from.top).then(&Matrix3::scaling(sx, sy)).then(&Matrix3::translation(to.left, to.top))
}

/// Matrix „um Punkt `c` mit `m` transformieren“.
pub fn about(c: Point, m: &Matrix3) -> Matrix3 {
    Matrix3::translation(-c.x, -c.y).then(m).then(&Matrix3::translation(c.x, c.y))
}

// ------------------------------------------------------------------ Attribute

/// Wendet `f` auf die Pfadattribute (Strich/Füllung) eines Elements an; Gruppen und Compound
/// Shapes rekursiv. Liefert, ob etwas geändert wurde.
pub fn edit_path_attrs(e: &mut Element, f: &dyn Fn(&mut PathAttrs)) -> bool {
    e.cache = None;
    match &mut e.kind {
        ElementKind::Path(p) => {
            f(Arc::make_mut(&mut p.attrs));
            true
        }
        ElementKind::Text(t) => {
            f(Arc::make_mut(&mut t.attrs));
            true
        }
        ElementKind::Group(g) => {
            let mut any = false;
            for c in &mut g.children {
                any |= edit_path_attrs(Element::edit(c), f);
            }
            any
        }
        ElementKind::CompoundShape(c) => {
            let mut any = false;
            if let Some(r) = &mut c.result {
                any |= edit_path_attrs(Element::edit(r), f);
            }
            for o in &mut c.operands {
                any |= edit_path_attrs(Element::edit(o), f);
            }
            any
        }
        _ => false,
    }
}

/// Pfadattribute eines Elements (erstes passendes in Gruppen).
pub fn path_attrs_of(e: &Element) -> Option<Arc<PathAttrs>> {
    match &e.kind {
        ElementKind::Path(p) => Some(Arc::clone(&p.attrs)),
        ElementKind::Text(t) => Some(Arc::clone(&t.attrs)),
        ElementKind::Group(g) => g.children.iter().find_map(|c| path_attrs_of(c)),
        ElementKind::CompoundShape(c) => c.result.as_ref().and_then(|r| path_attrs_of(r)).or_else(|| c.operands.iter().find_map(|o| path_attrs_of(o))),
        _ => None,
    }
}

// ------------------------------------------------------------------ Treffertest

/// Optionen des Treffertests.
#[derive(Clone, Copy, Debug)]
pub struct HitOptions {
    /// Toleranz in Dokumentpixeln (Markierungsabstand / Zoom).
    pub tolerance: f64,
    /// Gesperrte Elemente/Ebenen überspringen.
    pub skip_locked: bool,
    /// Ausgeblendete Elemente/Ebenen überspringen.
    pub skip_hidden: bool,
}

impl Default for HitOptions {
    fn default() -> Self {
        HitOptions { tolerance: 3.0, skip_locked: true, skip_hidden: true }
    }
}

/// Trifft `p` das Element?
pub fn hit_element(doc: &Document, e: &Element, p: Point, geo: &dyn GeometryOps, opt: &HitOptions) -> bool {
    hit_depth(doc, e, p, geo, opt, 0)
}

fn hit_depth(doc: &Document, e: &Element, p: Point, geo: &dyn GeometryOps, opt: &HitOptions, depth: u32) -> bool {
    if depth > 24 || (opt.skip_hidden && !e.common.visible) {
        return false;
    }
    match &e.kind {
        ElementKind::Path(pe) => {
            let shape = PathShape { contours: pe.contours.clone(), even_odd: pe.even_odd };
            let stroke = pe.attrs.brush.as_ref().map(|b| b.diameter as f64).unwrap_or(0.0);
            let filled = pe.attrs.fill.is_some() && pe.contours.iter().any(|c| c.closed || c.nodes.len() > 2);
            geo.hit_test(&shape, p, stroke, filled, opt.tolerance)
        }
        ElementKind::Group(g) => {
            if g.kind.is_mask()
                && g.mask_flags.enabled
                && let Some(m) = g.effective_mask()
                && let Some(mb) = element_bounds(doc, m)
                && !mb.inflate(opt.tolerance as f32, opt.tolerance as f32).contains(p)
            {
                return false;
            }
            g.children.iter().any(|c| hit_depth(doc, c, p, geo, opt, depth + 1))
        }
        ElementKind::CompoundShape(c) => match &c.result {
            Some(r) => hit_depth(doc, r, p, geo, opt, depth + 1),
            None => c.operands.iter().any(|o| hit_depth(doc, o, p, geo, opt, depth + 1)),
        },
        // Bitmaps treffen wie im Original in ihrem ganzen Rahmen, auch auf durchsichtigen Pixeln
        // (FV-11/FV-14: Klick in ein radiertes Loch bzw. die Ecke einer Ellipsen-Bitmap wählt sie)
        ElementKind::Bitmap(_) => element_bounds(doc, e).is_some_and(|r| r.contains(p)),
        _ => element_bounds(doc, e).is_some_and(|r| r.inflate(opt.tolerance as f32, opt.tolerance as f32).contains(p)),
    }
}

/// Alle getroffenen Elemente direkt in den Cels des Status `frame`, oben zuerst (für Zeiger und
/// „Hintere auswählen“).
pub fn hits_at(doc: &Document, page: usize, frame: usize, p: Point, geo: &dyn GeometryOps, opt: &HitOptions) -> Vec<ElementPath> {
    let mut out = Vec::new();
    for (slot, l) in own_layers(doc, page).into_iter().rev() {
        if opt.skip_hidden && !l.is_visible() {
            continue;
        }
        let ci = if l.shared_across_frames { 0 } else { frame };
        let Some(cel) = l.cels.get(ci) else { continue };
        if opt.skip_locked && cel.locked {
            continue;
        }
        for (i, e) in cel.elements.iter().enumerate() {
            if opt.skip_locked && e.common.locked {
                continue;
            }
            if hit_element(doc, e, p, geo, opt) {
                out.push(slot_path(doc, page, slot, ci, vec![PathStep::Child(i)]));
            }
        }
    }
    out
}

/// Tiefster getroffener Nachfahre (Teilauswahl): steigt in Gruppen ab.
pub fn deep_hit(doc: &Document, path: &ElementPath, p: Point, geo: &dyn GeometryOps, opt: &HitOptions) -> ElementPath {
    let mut cur = path.clone();
    for _ in 0..32 {
        let Some(e) = doc.element_at(&cur) else { break };
        let ElementKind::Group(g) = &e.kind else { break };
        let Some(i) = g.children.iter().position(|c| hit_element(doc, c, p, geo, opt)) else { break };
        cur.steps.push(PathStep::Child(i));
    }
    cur
}

/// IDs aller obersten Elemente des Status `frame` (sichtbar, nicht gesperrt), oben zuerst.
pub fn selectable_ids(doc: &Document, page: usize, frame: usize) -> Vec<ElementId> {
    let mut out = Vec::new();
    for (_, l) in own_layers(doc, page).into_iter().rev() {
        if !l.is_visible() {
            continue;
        }
        let ci = if l.shared_across_frames { 0 } else { frame };
        let Some(cel) = l.cels.get(ci) else { continue };
        if cel.locked {
            continue;
        }
        for e in &cel.elements {
            if e.common.visible && !e.common.locked {
                out.push(e.id);
            }
        }
    }
    out
}

// ------------------------------------------------------------------ Status

/// Fügt `count` leere Status bei `at` ein (alle eigenen, nicht über Status geteilten Ebenen).
pub fn insert_frames(doc: &mut Document, page: usize, at: usize, count: usize) -> Option<()> {
    let p = doc.page_mut(page)?;
    let n = p.root.frame_count as usize;
    let at = at.min(n);
    for s in &mut p.root.layers {
        if let LayerSlot::Own(l) = s {
            let l = Arc::make_mut(l);
            if l.shared_across_frames {
                continue;
            }
            while l.cels.len() < n {
                l.cels.push(Arc::new(Cel::empty()));
            }
            for _ in 0..count {
                l.cels.insert(at, Arc::new(Cel::empty()));
            }
        }
    }
    p.root.frame_count = (n + count) as u32;
    Some(())
}

/// Entfernt den Status `index` (mindestens einer bleibt).
pub fn delete_frame(doc: &mut Document, page: usize, index: usize) -> Option<()> {
    let p = doc.page_mut(page)?;
    let n = p.root.frame_count as usize;
    if n <= 1 || index >= n {
        return None;
    }
    for s in &mut p.root.layers {
        if let LayerSlot::Own(l) = s {
            let l = Arc::make_mut(l);
            if !l.shared_across_frames && index < l.cels.len() {
                l.cels.remove(index);
            }
        }
    }
    p.root.frame_count = (n - 1) as u32;
    Some(())
}

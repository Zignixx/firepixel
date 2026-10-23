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
use fp_model::traverse::{ElementPath, PathStep, RootRef, walk_root};
use fp_model::{Document, ElementId, LayerId, PathAttrs, TileGrid};
use fp_raster::{RgbaImage, Sampler};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

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
    let refs: Vec<(usize, usize)> = p
        .root
        .layers
        .iter()
        .enumerate()
        .filter_map(|(slot, s)| match s {
            LayerSlot::Shared(r) => shared_def_index(doc, &r.id).map(|i| (i, slot)),
            _ => None,
        })
        .collect();
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
    let top = ElementPath {
        steps: path.steps.iter().take(1).cloned().collect(),
        ..path.clone()
    };
    if doc.element_at(&top).is_some_and(|e| e.common.locked)
        || doc.element_at(path).is_some_and(|e| e.common.locked)
    {
        return true;
    }
    own_layers(doc, page)
        .into_iter()
        .find(|(s, _)| *s == path.slot)
        .and_then(|(_, l)| l.cels.get(path.cel).map(|c| c.locked))
        .unwrap_or(false)
}

/// Index der Definition einer geteilten Ebene in `doc.shared_layers`.
pub fn shared_def_index(doc: &Document, id: &fp_model::Guid) -> Option<usize> {
    doc.shared_layers.iter().position(|d| d.id.same_as(id))
}

/// Adresse eines Elements in Ebene `slot` der Seite (eigene oder geteilte Ebene).
pub fn slot_path(
    doc: &Document,
    page: usize,
    slot: usize,
    cel: usize,
    steps: Vec<PathStep>,
) -> ElementPath {
    let root = match doc.pages.get(page).and_then(|p| p.root.layers.get(slot)) {
        Some(LayerSlot::Shared(r)) => shared_def_index(doc, &r.id)
            .map(RootRef::SharedLayer)
            .unwrap_or(RootRef::Page(page)),
        _ => RootRef::Page(page),
    };
    ElementPath {
        root,
        slot,
        cel,
        steps,
    }
}

/// Element zu einer ID auf einer Seite.
/// Ist `id` die Maske eines Elements (Maskenbearbeitung, re 31 §5.3 / RT-3 S5)? Liefert das
/// maskierte Element (Einzelelement bzw. Maskengruppe), dessen Maske `id` ist.
pub fn mask_owner(doc: &Document, page: usize, id: ElementId) -> Option<Arc<Element>> {
    let mut p = find_on_page(doc, page, id)?;
    if !matches!(p.steps.last(), Some(fp_model::traverse::PathStep::Mask)) {
        return None;
    }
    p.steps.pop();
    doc.element_at(&p).cloned()
}

pub fn element_on_page(doc: &Document, page: usize, id: ElementId) -> Option<Arc<Element>> {
    find_on_page(doc, page, id).and_then(|p| doc.element_at(&p).cloned())
}

/// Adressen vieler IDs einer Seite in **einem** Durchlauf (PERF-1): je ID dasselbe Ergebnis wie
/// [`find_on_page`], aber bei großer Auswahl O(Objekte) statt O(Objekte × Auswahl).
pub fn find_many_on_page(
    doc: &Document,
    page: usize,
    ids: &[ElementId],
) -> Vec<Option<ElementPath>> {
    if ids.len() < 4 {
        return ids.iter().map(|id| find_on_page(doc, page, *id)).collect();
    }
    let Some(p) = doc.pages.get(page) else {
        return vec![None; ids.len()];
    };
    let wanted: std::collections::HashSet<ElementId> = ids.iter().copied().collect();
    let mut found: std::collections::HashMap<ElementId, ElementPath> =
        std::collections::HashMap::with_capacity(wanted.len());
    walk_root(&p.root, RootRef::Page(page), &mut |_, path, e| {
        if wanted.contains(&e.id) && !found.contains_key(&e.id) {
            found.insert(e.id, path.clone());
        }
    });
    if found.len() < wanted.len() {
        // geteilte Ebenen wie in `find_on_page`
        let refs: Vec<(usize, usize)> = p
            .root
            .layers
            .iter()
            .enumerate()
            .filter_map(|(slot, s)| match s {
                LayerSlot::Shared(r) => shared_def_index(doc, &r.id).map(|i| (i, slot)),
                _ => None,
            })
            .collect();
        if !refs.is_empty() {
            let mut shared: std::collections::HashMap<ElementId, ElementPath> =
                std::collections::HashMap::new();
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

/// Visits the elements `ids` of a page (each at the address [`find_on_page`] gives) without
/// collecting or sorting them; `f` returns `true` to stop (BA-099: early exit of menu checks with
/// thousands of selected objects). Order: own layers in stacking walk order, then the rest.
pub fn visit_on_page(
    doc: &Document,
    page: usize,
    ids: &[ElementId],
    f: &mut dyn FnMut(&ElementPath, &Arc<Element>) -> bool,
) {
    if ids.len() < 4 {
        for id in ids {
            if let Some(p) = find_on_page(doc, page, *id)
                && let Some(e) = doc.element_at(&p)
                && f(&p, e)
            {
                return;
            }
        }
        return;
    }
    let Some(pg) = doc.pages.get(page) else {
        return;
    };
    let mut wanted: std::collections::HashSet<ElementId> = ids.iter().copied().collect();
    let mut stop = false;
    walk_root(&pg.root, RootRef::Page(page), &mut |_, path, e| {
        if !stop && wanted.remove(&e.id) {
            stop = f(path, e);
        }
    });
    if stop || wanted.is_empty() {
        return;
    }
    // not on the own layers (shared layers): the general lookup
    let rest: Vec<ElementId> = ids
        .iter()
        .copied()
        .filter(|id| wanted.contains(id))
        .collect();
    for p in find_many_on_page(doc, page, &rest).into_iter().flatten() {
        if let Some(e) = doc.element_at(&p)
            && f(&p, e)
        {
            return;
        }
    }
}

/// Elemente vieler IDs einer Seite in einem Durchlauf, in der Reihenfolge von `ids`; fehlende
/// fallen weg (wie `ids.iter().filter_map(|id| element_on_page(…))`, PERF-1).
pub fn elements_on_page(doc: &Document, page: usize, ids: &[ElementId]) -> Vec<Arc<Element>> {
    find_many_on_page(doc, page, ids)
        .into_iter()
        .flatten()
        .filter_map(|p| doc.element_at(&p).cloned())
        .collect()
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
    let mut v: Vec<ElementPath> = find_many_on_page(doc, page, ids)
        .into_iter()
        .flatten()
        .collect();
    v.sort_by_key(z_key);
    v
}

/// Oberstes der Elemente `ids` in der Stapelreihenfolge (`fw.selection[0]` im Original).
pub fn top_selected(doc: &Document, page: usize, ids: &[ElementId]) -> Option<ElementId> {
    find_many_on_page(doc, page, ids)
        .into_iter()
        .zip(ids)
        .filter_map(|(p, id)| p.map(|p| (z_key(&p), *id)))
        .min_by(|a, b| a.0.cmp(&b.0))
        .map(|x| x.1)
}

// ------------------------------------------------------------------ Zugriff

/// Veränderlicher Cel einer eigenen Ebene (Copy-on-Write).
pub fn cel_mut(doc: &mut Document, page: usize, slot: usize, cel: usize) -> Option<&mut Cel> {
    let l = layer_mut(doc, page, slot)?;
    Some(Arc::make_mut(l.cels.get_mut(cel)?))
}

fn descend_list<'a>(
    list: &'a mut Vec<Arc<Element>>,
    steps: &[PathStep],
) -> Option<&'a mut Vec<Arc<Element>>> {
    match steps {
        [] => Some(list),
        [PathStep::Child(k), PathStep::SubLayerCel(c), rest @ ..] => {
            let e = Element::edit(list.get_mut(*k)?);
            let ElementKind::SubLayer(l) = &mut e.kind else {
                return None;
            };
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
pub fn parent_list_mut<'a>(
    doc: &'a mut Document,
    path: &ElementPath,
) -> Option<(&'a mut Vec<Arc<Element>>, usize)> {
    let (last, prefix) = path.steps.split_last()?;
    let PathStep::Child(idx) = *last else {
        return None;
    };
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
pub fn edit_at<R>(
    doc: &mut Document,
    path: &ElementPath,
    f: impl FnOnce(&mut Element) -> R,
) -> Option<R> {
    if let Some((list, idx)) = parent_list_mut(doc, path) {
        let e = Element::edit(list.get_mut(idx)?);
        return Some(f(e));
    }
    let e = doc.element_at_mut(path)?;
    e.cache = None;
    Some(f(e))
}

/// Verschiebt ein Element um (dx, dy) wie `edit_at(…, translate_element)`, behält aber seinen
/// gehaltenen Render-Cache (er wird mitverschoben, BA-099: `edit_at` verwarf ihn vorher, das
/// Verschieben des Caches in [`translate_element`] lief dadurch ins Leere).
pub fn translate_at(doc: &mut Document, path: &ElementPath, dx: f32, dy: f32) -> Option<()> {
    if let Some((list, idx)) = parent_list_mut(doc, path) {
        translate_element(Arc::make_mut(list.get_mut(idx)?), dx, dy);
        return Some(());
    }
    translate_element(doc.element_at_mut(path)?, dx, dy);
    Some(())
}

/// Ändert ein Element per ID auf einer Seite.
pub fn edit_by_id<R>(
    doc: &mut Document,
    page: usize,
    id: ElementId,
    f: impl FnOnce(&mut Element) -> R,
) -> Option<R> {
    let path = find_on_page(doc, page, id)?;
    edit_at(doc, &path, f)
}

/// Entfernt ein Element und liefert es.
pub fn remove_at(doc: &mut Document, path: &ElementPath) -> Option<Arc<Element>> {
    let (list, idx) = parent_list_mut(doc, path)?;
    if idx < list.len() {
        Some(list.remove(idx))
    } else {
        None
    }
}

/// Entfernt Elemente per ID (oben zuerst) und liefert sie mit ihrer bisherigen Adresse.
pub fn remove_ids(
    doc: &mut Document,
    page: usize,
    ids: &[ElementId],
) -> Vec<(ElementPath, Arc<Element>)> {
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
                    LayerSlot::Shared(r) => doc
                        .shared_layer(&r.id)
                        .and_then(|d| d.layer.clone())
                        .map(|l| (i, l)),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Slot einer Ebene.
pub fn layer_slot(doc: &Document, page: usize, id: LayerId) -> Option<usize> {
    own_layers(doc, page)
        .into_iter()
        .find(|(_, l)| l.id == id)
        .map(|(s, _)| s)
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

/// Element-ID der Unterebene mit der Ebenen-ID `id` auf einer Seite (Unterebenen sind Elemente
/// `ElementKind::SubLayer` im Cel ihrer Elternebene).
pub fn sublayer_element(doc: &Document, page: usize, id: LayerId) -> Option<ElementId> {
    let p = doc.pages.get(page)?;
    let mut found = None;
    walk_root(&p.root, RootRef::Page(page), &mut |_, _, e| {
        if found.is_none()
            && let ElementKind::SubLayer(l) = &e.kind
            && l.id == id
        {
            found = Some(e.id);
        }
    });
    found
}

/// Blendet eine Ebene samt aller Unterebenen ein bzw. aus (TW-54): `VIFv`, `VISb` der Cels und bei
/// Unterebenen auch das Sichtbar-Flag des Unterebenen-Elements.
pub fn set_layer_tree_shown(l: &mut Layer, v: bool) {
    l.set_shown(v);
    for c in &mut l.cels {
        let has_sub = c
            .elements
            .iter()
            .any(|e| matches!(e.kind, ElementKind::SubLayer(_)));
        if !has_sub {
            continue;
        }
        for e in &mut Arc::make_mut(c).elements {
            if let ElementKind::SubLayer(_) = &e.kind {
                let e = Arc::make_mut(e);
                e.common.visible = v;
                if let ElementKind::SubLayer(sl) = &mut e.kind {
                    set_layer_tree_shown(Arc::make_mut(sl), v);
                }
            }
        }
    }
}

/// Ebene bzw. Unterebene `id` ein- oder ausblenden, wie das Auge im Ebenen-Bedienfeld (TW-54). Bei
/// einer Unterebene gilt auch das Sichtbar-Flag ihres Elements. `None`, wenn es die Ebene nicht gibt.
pub fn set_layer_shown(doc: &mut Document, page: usize, id: LayerId, v: bool) -> Option<()> {
    if let Some(slot) = layer_slot(doc, page, id) {
        return layer_mut(doc, page, slot).map(|l| l.set_shown(v));
    }
    let el = sublayer_element(doc, page, id)?;
    edit_by_id(doc, page, el, |e| {
        e.common.visible = v;
        if let ElementKind::SubLayer(l) = &mut e.kind {
            Arc::make_mut(l).set_shown(v);
        }
    })
}

/// „Alles ausblenden/anzeigen“: alle Ebenen der Seite einschließlich Unterebenen und über Seiten
/// geteilter Ebenen (TW-54).
pub fn set_all_layers_shown(doc: &mut Document, page: usize, v: bool) {
    let slots: Vec<usize> = own_layers(doc, page).into_iter().map(|(s, _)| s).collect();
    for s in slots {
        if let Some(l) = layer_mut(doc, page, s) {
            set_layer_tree_shown(l, v);
        }
    }
}

/// Sichtbarkeit einer Ebene oder Unterebene im Status `frame`, wie das Auge sie zeigt (TW-54).
pub fn layer_shown(doc: &Document, page: usize, id: LayerId, frame: usize) -> Option<bool> {
    if let Some((_, l)) = own_layers(doc, page).into_iter().find(|(_, l)| l.id == id) {
        return Some(l.is_shown(frame));
    }
    let e = element_on_page(doc, page, sublayer_element(doc, page, id)?)?;
    match &e.kind {
        ElementKind::SubLayer(l) => Some(e.common.visible && l.is_shown(frame)),
        _ => None,
    }
}

/// Ändert eine Ebene per ID: eine Ebene der Seite (eigene oder über Seiten geteilte) oder eine
/// Unterebene darin. `None`, wenn es die Ebene auf der Seite nicht gibt.
pub fn edit_layer_by_id<R>(
    doc: &mut Document,
    page: usize,
    id: LayerId,
    f: impl FnOnce(&mut Layer) -> R,
) -> Option<R> {
    if let Some(slot) = layer_slot(doc, page, id) {
        return layer_mut(doc, page, slot).map(f);
    }
    let el = sublayer_element(doc, page, id)?;
    let mut f = Some(f);
    edit_by_id(doc, page, el, |e| match &mut e.kind {
        ElementKind::SubLayer(l) => f.take().map(|f| f(Arc::make_mut(l))),
        _ => None,
    })
    .flatten()
}

/// Oberste normale (nicht Web-)Ebene.
pub fn top_normal_layer(doc: &Document, page: usize) -> Option<LayerId> {
    own_layers(doc, page)
        .into_iter()
        .rev()
        .find(|(_, l)| l.kind == LayerKind::Normal)
        .map(|(_, l)| l.id)
}

/// Aktive Ebene nach einem Seitenwechsel: die bisherige, wenn es sie auf der Seite gibt (über Seiten
/// geteilte Ebene), sonst die oberste normale Ebene.
pub fn layer_on_page(doc: &Document, page: usize, current: Option<LayerId>) -> Option<LayerId> {
    match current {
        Some(c)
            if own_layers(doc, page).iter().any(|(_, l)| l.id == c)
                || sublayer_element(doc, page, c).is_some() =>
        {
            Some(c)
        }
        _ => top_normal_layer(doc, page),
    }
}

/// One entry of a page's flat layer list in script `dom.layers` order (RE spec 64 §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlatLayer {
    pub id: LayerId,
    /// Locked in the state, itself or through a parent layer (a sublayer inherits the lock,
    /// RE spec 64 O13).
    pub locked: bool,
    pub web: bool,
}

/// The page's layers flat as `dom.layers` lists them: index 0 = the bottom layer, a layer's
/// sublayers right after it (bottom one first, nested ones after their parent), the web layer
/// last. Master-page slots have no layer of their own and are left out.
pub fn flat_layers(doc: &Document, page: usize, frame: usize) -> Vec<FlatLayer> {
    fn subs(l: &Layer, frame: usize, locked: bool, out: &mut Vec<FlatLayer>) {
        let Some(cel) = l.cel(frame).or_else(|| l.cel(0)) else {
            return;
        };
        for e in cel.elements.iter().rev() {
            if let ElementKind::SubLayer(sl) = &e.kind {
                let lk = locked || e.common.locked || sl.cel(frame).is_some_and(|c| c.locked);
                out.push(FlatLayer {
                    id: sl.id,
                    locked: lk,
                    web: false,
                });
                subs(sl, frame, lk, out);
            }
        }
    }
    let mut out = Vec::new();
    for (_, l) in own_layers(doc, page) {
        let locked = l.cel(frame).is_some_and(|c| c.locked);
        out.push(FlatLayer {
            id: l.id,
            locked,
            web: l.kind == LayerKind::Web,
        });
        subs(&l, frame, locked, &mut out);
    }
    out
}

/// Current layer after opening or reverting a file (RE spec 64 §3 rule 1, Q-65): the first layer
/// in flat `dom.layers` order that is not locked in state 0; visibility does not matter and the
/// web layer counts as never locked (its lock is not kept, O12). Without any, flat index 0.
pub fn bottom_unlocked_layer(doc: &Document, page: usize) -> Option<LayerId> {
    let list = flat_layers(doc, page, 0);
    list.iter()
        .find(|l| !l.locked || l.web)
        .or_else(|| list.first())
        .map(|l| l.id)
}

/// The current layer for the chosen layer `want` (RE spec 65 rule 3): the first unlocked layer in
/// flat order from `want` upward (`want` itself first, sublayers inheriting their parent's lock,
/// the web layer included), then from flat index 0 up to below `want`; with every layer locked flat
/// index 0. `None` when `want` is not in the page's flat list.
pub fn resolve_layer(doc: &Document, page: usize, frame: usize, want: LayerId) -> Option<LayerId> {
    let list = flat_layers(doc, page, frame);
    let at = list.iter().position(|l| l.id == want)?;
    list[at..]
        .iter()
        .chain(&list[..at])
        .find(|l| !l.locked)
        .or_else(|| list.first())
        .map(|l| l.id)
}

/// Does the layer `l` (or one of its sublayers, at any depth) have the id `id`?
pub fn layer_tree_contains(l: &Layer, id: LayerId) -> bool {
    l.id == id
        || l.cels.iter().any(|c| {
            c.elements.iter().any(|e| match &e.kind {
                ElementKind::SubLayer(sl) => layer_tree_contains(sl, id),
                _ => false,
            })
        })
}

/// Does the layer hold objects of its own in the state (its sublayers do not count)? The Layers
/// panel draws the selection circle of a layer without them grey (RE spec 65 rule 2).
pub fn layer_has_own_objects(l: &Layer, frame: usize) -> bool {
    l.cel(frame).or_else(|| l.cel(0)).is_some_and(|c| {
        c.elements
            .iter()
            .any(|e| !matches!(e.kind, ElementKind::SubLayer(_)))
    })
}

/// Is the layer or sublayer `id` locked in the state (itself or through a parent)?
pub fn layer_locked_in(doc: &Document, page: usize, frame: usize, id: LayerId) -> bool {
    flat_layers(doc, page, frame)
        .iter()
        .any(|l| l.id == id && l.locked)
}

/// Layer or sublayer an element at `path` belongs to (RE spec 64 §3 rule 4): the innermost
/// sublayer on the way to it, otherwise the layer of the page slot; an object inside a group
/// belongs to the group's layer. A selected sublayer element stands for that sublayer.
pub fn owning_layer(doc: &Document, page: usize, path: &ElementPath) -> Option<LayerId> {
    let mut layer = match path.root {
        RootRef::SharedLayer(i) => doc.shared_layers.get(i)?.layer.as_ref()?.id,
        _ => match doc.pages.get(page)?.root.layers.get(path.slot)? {
            LayerSlot::Own(l) => l.id,
            LayerSlot::Shared(r) => doc.shared_layer(&r.id)?.layer.as_ref()?.id,
            _ => return None,
        },
    };
    for i in 0..path.steps.len() {
        let inner = i + 1 == path.steps.len()
            || matches!(path.steps.get(i + 1), Some(PathStep::SubLayerCel(_)));
        if !inner || matches!(path.steps[i], PathStep::SubLayerCel(_)) {
            continue;
        }
        let pre = ElementPath {
            steps: path.steps[..=i].to_vec(),
            ..path.clone()
        };
        if let Some(e) = doc.element_at(&pre)
            && let ElementKind::SubLayer(sl) = &e.kind
        {
            layer = sl.id;
        }
    }
    Some(layer)
}

/// Ebene per ID: eine Ebene der Seite (eigene oder über Seiten geteilte) oder eine Unterebene darin.
pub fn layer_by_id(doc: &Document, page: usize, id: LayerId) -> Option<Arc<Layer>> {
    if let Some((_, l)) = own_layers(doc, page).into_iter().find(|(_, l)| l.id == id) {
        return Some(l);
    }
    let el = element_on_page(doc, page, sublayer_element(doc, page, id)?)?;
    match &el.kind {
        ElementKind::SubLayer(l) => Some(Arc::clone(l)),
        _ => None,
    }
}

/// Slot der Ebene der obersten Stufe, die die Ebene `id` ist bzw. die Unterebene `id` enthält.
pub fn top_slot_of(doc: &Document, page: usize, id: LayerId) -> Option<usize> {
    layer_slot(doc, page, id)
        .or_else(|| find_on_page(doc, page, sublayer_element(doc, page, id)?).map(|p| p.slot))
}

/// Ist `id` eine Unterebene (und keine Ebene der obersten Stufe) dieser Seite? Liefert ihr Element.
pub fn as_sublayer(doc: &Document, page: usize, id: LayerId) -> Option<ElementId> {
    if layer_slot(doc, page, id).is_some() {
        return None;
    }
    sublayer_element(doc, page, id)
}

/// Kopiert die Unterebene `el` `n`-mal (neue IDs, Name „Kopie von …“) in ihre Elternliste.
/// `at(index, len)` liefert die Einfügestelle in der Liste (0 = oben) aus dem Index des Originals
/// und der Listenlänge. Ergebnis: Ebenen-ID der zuletzt eingefügten Kopie.
pub fn duplicate_sublayer(
    doc: &mut Document,
    page: usize,
    el: ElementId,
    n: usize,
    at: impl Fn(usize, usize) -> usize,
) -> Option<LayerId> {
    let path = find_on_page(doc, page, el)?;
    let src = doc.element_at(&path).cloned()?;
    let mut last = None;
    for _ in 0..n.max(1) {
        let mut c = reassign(doc, &src);
        if let ElementKind::SubLayer(l) = &mut c.kind {
            let l = Arc::make_mut(l);
            l.name = fp_i18n::tr!("msg-default-copy", name = l.name.as_str());
            last = Some(l.id);
        }
        let (list, idx) = parent_list_mut(doc, &path)?;
        let i = at(idx, list.len()).min(list.len());
        list.insert(i, Arc::new(c));
    }
    last
}

/// Verschiebt die Unterebene `el` in ihrer Elternliste an Position `to`, gezählt von unten
/// (0 = unterste) wie die Ebenenliste.
pub fn move_sublayer(doc: &mut Document, page: usize, el: ElementId, to: usize) -> Option<()> {
    let path = find_on_page(doc, page, el)?;
    let (list, idx) = parent_list_mut(doc, &path)?;
    if idx >= list.len() {
        return None;
    }
    let e = list.remove(idx);
    let n = list.len();
    list.insert(n - to.min(n), e);
    Some(())
}

/// Web-Ebene der Seite.
pub fn web_layer(doc: &Document, page: usize) -> Option<LayerId> {
    own_layers(doc, page)
        .into_iter()
        .find(|(_, l)| l.kind == LayerKind::Web)
        .map(|(_, l)| l.id)
}

/// Fügt ein Element in eine Ebene ein (`index` 0 = ganz oben). Web-Objekte landen immer in der
/// Web-Ebene, andere Elemente nie (dann oberste normale Ebene). Liefert die Adresse.
pub fn insert_element(
    doc: &mut Document,
    page: usize,
    frame: usize,
    layer: Option<LayerId>,
    index: usize,
    e: Arc<Element>,
) -> Option<ElementPath> {
    let is_web = matches!(e.kind, ElementKind::WebObject(_));
    // aktive Unterebene (B-2 BA-030: im Original kann eine Unterebene aktiv sein): neue Elemente
    // oben in ihr Cel des aktuellen Status
    if !is_web
        && let Some(sub) = layer.filter(|l| layer_slot(doc, page, *l).is_none())
        && sublayer_element(doc, page, sub).is_some()
    {
        let id = e.id;
        let frames = doc.pages.get(page)?.root.frame_count as usize;
        let mut e = Some(e);
        edit_layer_by_id(doc, page, sub, |l| {
            let ci = if l.shared_across_frames {
                0
            } else {
                frame.min(frames.saturating_sub(1))
            };
            while l.cels.len() <= ci {
                l.cels.push(Arc::new(Cel::empty()));
            }
            if let (Some(c), Some(e)) = (l.cels.get_mut(ci), e.take()) {
                let cel = Arc::make_mut(c);
                drop_order(&mut cel.elements_extra);
                cel.elements.insert(index.min(cel.elements.len()), e);
            }
        })?;
        return find_on_page(doc, page, id);
    }
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
            let l = Layer::new(
                &mut ids,
                &if is_web {
                    fp_i18n::t("msg-default-web-layer")
                } else {
                    fp_i18n::tr!("msg-default-layer", n = "1")
                },
                if is_web {
                    LayerKind::Web
                } else {
                    LayerKind::Normal
                },
                frames,
            );
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
    let cel_index = if l.shared_across_frames {
        0
    } else {
        frame.min(frames.saturating_sub(1))
    };
    while l.cels.len() <= cel_index {
        l.cels.push(Arc::new(Cel::empty()));
    }
    let cel = Arc::make_mut(l.cels.get_mut(cel_index)?);
    drop_order(&mut cel.elements_extra);
    let i = index.min(cel.elements.len());
    cel.elements.insert(i, e);
    Some(slot_path(
        doc,
        page,
        slot,
        cel_index,
        vec![PathStep::Child(i)],
    ))
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
    Rect::new(
        frame.left - TEXT_MARGIN,
        frame.top - TEXT_MARGIN,
        frame.right + TEXT_MARGIN,
        frame.bottom + TEXT_MARGIN,
    )
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
            Some(Rect::from_xywh(
                p.x,
                p.y,
                b.image.grid.width as f32,
                b.image.grid.height as f32,
            ))
        }
        ElementKind::Group(g) => {
            let mut r = None;
            for c in &g.children {
                r = union_opt(r, bounds_depth(doc, c, depth + 1));
            }
            if g.kind.is_mask()
                && g.mask_flags.enabled
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
            None => c.operands.iter().fold(None, |acc, o| {
                union_opt(acc, bounds_depth(doc, o, depth + 1))
            }),
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
            // BA-020 (F16): nach dem Animieren-Dialog schließen die Grenzen den Weg **waagerecht**
            // um „Verschieben“ ein, unabhängig von „Richtung“ (B-2b/PM-6: Richtung 30, 72 px →
            // 50,50–222,120); nach JSF `convertToAnimSymbol` den Weg als Vektor (B-2: 62/−36 →
            // 20,−16,182,100). Merker `bounds_horizontal` setzt der Befehl je nach Eingang.
            let len = i.anim.offset.x.hypot(i.anim.offset.y);
            let horizontal = i.anim.bounds_horizontal;
            let c = b.center();
            let mut out = base;
            for k in 0..i.anim.frames.min(360) as usize {
                let Some(mut af) = i.anim.frame(k, sym.root.frame_count) else {
                    continue;
                };
                if horizontal {
                    af.offset = Point::new((af.t * len as f64) as f32, 0.0);
                }
                let local = Matrix3::translation(-c.x, -c.y)
                    .then(&Matrix3::scaling(af.scale as f32, af.scale as f32))
                    .then(&Matrix3::rotation_degrees(af.rotation))
                    .then(&Matrix3::translation(c.x, c.y));
                let m = local
                    .then(&i.transform)
                    .then(&Matrix3::translation(af.offset.x, af.offset.y));
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

/// Rechteck des Auswahlrahmens bzw. der Hervorhebung eines Elements. Bei Bitmaps die sichtbaren
/// Pixel (Alpha ≥ 2), nicht die gespeicherten Objektgrenzen (re\22 §1.2 Regel 3); eine leere Bitmap
/// fällt auf die Objektgrenzen zurück. Sonst wie [`element_bounds`].
pub fn element_frame_bounds(doc: &Document, e: &Element) -> Option<Rect> {
    match &e.kind {
        ElementKind::Bitmap(b) => b.image.visible_bounds().or_else(|| element_bounds(doc, e)),
        _ => element_bounds(doc, e),
    }
}

/// Toleranz für [`element_pixel_rect`]: Kanten, die weniger als ein Tausendstel neben einer
/// Pixelgrenze liegen, gelten als auf ihr – nur an rechter/unterer Kante (F22/TW-79).
const PIXEL_EPS: f32 = 1e-3;

/// `pixelRect` eines Elements wie im Original: bei Bitmaps das Rechteck der sichtbaren Pixel (leere
/// Bitmap: `None`, das Original meldet dann ±2³¹), sonst die nach außen gerundeten Grenzen.
pub fn element_pixel_rect(doc: &Document, e: &Element) -> Option<Rect> {
    match &e.kind {
        ElementKind::Bitmap(b) => b.image.visible_bounds().map(|r| {
            Rect::new(
                r.left.floor(),
                r.top.floor(),
                r.right.ceil(),
                r.bottom.ceil(),
            )
        }),
        // Pfade: mit dem Strich wie im Original (mittig + halbe, außen + ganze Pinselstärke; innen
        // nichts, PAR 17/08)
        ElementKind::Path(p) => {
            let r = element_bounds(doc, e)?;
            let Some(b) = &p.attrs.brush else {
                return Some(Rect::new(
                    r.left.floor(),
                    r.top.floor(),
                    (r.right - PIXEL_EPS).ceil(),
                    (r.bottom - PIXEL_EPS).ceil(),
                ));
            };
            let d = b.diameter.max(0);
            let grow = match p.stroke_placement {
                1 => d as f32 / 2.0,
                2 => d as f32,
                _ => 0.0,
            };
            // Rundung wie im Original (TW-05, aus menu-edit/02 u. a. abgeleitet): Strichkante auf die
            // nächste Pixelgrenze (½ nach innen), also E1 (geschlossen, 1 px mittig) 50,170,150,240 und
            // PG 465.36…534.64 → 465…535; offene Pfade mit ungerader Strichstärke liegen auf
            // Pixelmitten und reichen rechts/unten 1 px weiter (L1 50,300,301,361; P1 200,170,421,241).
            // Nur der gemeldete Rahmen, die Pixelausgabe ist davon unberührt.
            let open = p.contours.iter().any(|c| !c.closed);
            let extra = if open && d % 2 == 1 { 1.0 } else { 0.0 };
            // F20/TW-68: Rechenrauschen nach Transformationen (z. B. 250.00002 nach Angleichen) zählt
            // nicht als angeschnittenes Pixel. Nur rechts/unten: links/oben rundet das Original knapp
            // unter der Grenze nach außen (F22/TW-79, PSD-Pfad 9,99… → 9)
            let lo = |v: f32| (v - grow + 0.5).floor();
            let hi = |v: f32| (v + grow - 0.5 - PIXEL_EPS).ceil() + extra;
            Some(Rect::new(lo(r.left), lo(r.top), hi(r.right), hi(r.bottom)))
        }
        _ => element_bounds(doc, e).map(|r| {
            Rect::new(
                r.left.floor(),
                r.top.floor(),
                r.right.ceil(),
                r.bottom.ceil(),
            )
        }),
    }
}

/// Vereinigte Grenzen mehrerer Elemente einer Seite.
pub fn ids_bounds(doc: &Document, page: usize, ids: &[ElementId]) -> Option<Rect> {
    elements_on_page(doc, page, ids)
        .iter()
        .fold(None, |acc, e| union_opt(acc, element_bounds(doc, e)))
}

/// Transform frame of the Scale, Skew and Distort tools (also Free Transform and Modify ▸
/// Transform ▸ Scale), RE spec 67 §3.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransformFrame {
    /// The frame, unrounded: in document coordinates, or in the text's own space when
    /// [`TransformFrame::matrix`] is set.
    pub rect: Rect,
    /// Text space → document for a turned, sheared or distorted text (the frame is then a
    /// four-cornered shape); `None` = `rect` is in document coordinates.
    pub matrix: Option<Matrix3>,
    /// The frame of exactly one text (its set box scaled by 1.3).
    pub text: bool,
}

impl TransformFrame {
    /// The four corners in document coordinates (order of [`Rect::corners`]).
    pub fn quad(&self) -> [Point; 4] {
        match self.matrix {
            Some(m) => m.transform_quad(&self.rect),
            None => self.rect.corners(),
        }
    }
}

/// Transform frame of a selection (RE spec 67 §3): exactly one text → its set box
/// ([`text_transform_box`]) scaled by 1.3 about its centre (0.15 × width added left and right,
/// 0.15 × height top and bottom; an empty box grows by 2 instead), unrounded, mapped through the
/// text's matrix; paths, bitmaps, groups (also with a text inside) and several objects → the
/// (united) object bounds without a margin.
pub fn transform_frame(
    doc: &Document,
    page: usize,
    ids: &[ElementId],
    fit: Option<TextFitFn>,
) -> Option<TransformFrame> {
    if let [id] = ids
        && let Some(e) = element_on_page(doc, page, *id)
        && let ElementKind::Text(t) = &e.kind
    {
        return Some(text_transform_frame(t, fit));
    }
    ids_bounds(doc, page, ids).map(|rect| TransformFrame {
        rect,
        matrix: None,
        text: false,
    })
}

/// Set box of a text for its transform frame (RE spec 67 §2.1–2.4, before the matrix): point
/// text = the layout frame (`raw` rect); fixed-width text = the box around the set content from
/// [`crate::render::TextFitBoxes::transform_box`] (without a layout: the frame).
pub fn text_transform_box(t: &fp_model::TextElem, fit: Option<TextFitFn>) -> [f64; 4] {
    let r = t.bounds;
    let frame = [r.left as f64, r.top as f64, r.right as f64, r.bottom as f64];
    if matches!(t.frame_type, None | Some(0)) {
        return frame;
    }
    fit.and_then(|f| f(t))
        .map(|b| b.transform_box)
        .unwrap_or(frame)
}

/// Transform frame of one text, see [`transform_frame`]. A text matrix that only scales and moves
/// along the axes gives a frame in document coordinates.
pub fn text_transform_frame(t: &fp_model::TextElem, fit: Option<TextFitFn>) -> TransformFrame {
    let b = text_transform_box(t, fit);
    let (w, h) = (b[2] - b[0], b[3] - b[1]);
    let (dx, dy) = if w <= 0.0 || h <= 0.0 {
        (2.0, 2.0) // the reference's code (§2.11), not measured
    } else {
        (0.15 * w, 0.15 * h)
    };
    let f = [b[0] - dx, b[1] - dy, b[2] + dx, b[3] + dy];
    let rect = Rect::new(f[0] as f32, f[1] as f32, f[2] as f32, f[3] as f32);
    match t.transform {
        Some(m) if !m.is_identity() => {
            if m.is_affine() && m.m[0][1] == 0.0 && m.m[1][0] == 0.0 {
                let p = |x: f64, y: f64| m.apply_vec2(fp_geom::Vec2::new(x, y));
                let (a, c) = (p(f[0], f[1]), p(f[2], f[3]));
                TransformFrame {
                    rect: Rect::new(
                        a.x.min(c.x) as f32,
                        a.y.min(c.y) as f32,
                        a.x.max(c.x) as f32,
                        a.y.max(c.y) as f32,
                    ),
                    matrix: None,
                    text: true,
                }
            } else {
                TransformFrame {
                    rect,
                    matrix: Some(m),
                    text: true,
                }
            }
        }
        _ => TransformFrame {
            rect,
            matrix: None,
            text: true,
        },
    }
}

/// Grenzen, die der Inspektor in B/H/X/Y zeigt: bei einem maskierten Einzelbild
/// ([`Element::masked_single`]) die des **Inhalts** (Bildgrenzen, nicht der Maskenbereich; re 29b
/// §2, BA-006), sonst [`element_bounds`].
pub fn info_bounds(doc: &Document, e: &Element) -> Option<Rect> {
    match e.masked_single() {
        Some((c, _)) => element_bounds(doc, c),
        None => mask_group_bounds(doc, e).or_else(|| element_bounds(doc, e)),
    }
}

/// Objektgrenzen einer Maskengruppe wie im Original (re 29b §1b): die **Vereinigung der Kinder**
/// ohne Beschnitt durch die Maske (R4/E1 unter M1 → 50,50, 140 × 140; `pixelRect` bleibt der
/// Maskenbereich, also [`element_bounds`]). Für andere Elemente und maskierte Einzelelemente
/// ([`Element::masked_single`], dort gilt [`info_bounds`]) `None`.
pub fn mask_group_bounds(doc: &Document, e: &Element) -> Option<Rect> {
    let ElementKind::Group(g) = &e.kind else {
        return None;
    };
    if !g.kind.is_mask() || e.masked_single().is_some() {
        return None;
    }
    g.children
        .iter()
        .fold(None, |acc, c| union_opt(acc, bounds_depth(doc, c, 1)))
}

/// Vereinigte [`info_bounds`] mehrerer Elemente einer Seite (Inspektor, „Position und Größe“).
pub fn ids_info_bounds(doc: &Document, page: usize, ids: &[ElementId]) -> Option<Rect> {
    elements_on_page(doc, page, ids)
        .iter()
        .fold(None, |acc, e| union_opt(acc, info_bounds(doc, e)))
}

/// Vereinigte Rahmen-Grenzen mehrerer Elemente einer Seite wie [`element_frame_bounds`] (Bitmaps:
/// sichtbare Pixel). Grundlage für Griffe, Verschiebe-Vorschau und Transformationsrahmen, damit sie
/// mit dem gezeichneten Auswahlrahmen übereinstimmen (N32).
pub fn ids_frame_bounds(doc: &Document, page: usize, ids: &[ElementId]) -> Option<Rect> {
    elements_on_page(doc, page, ids)
        .iter()
        .fold(None, |acc, e| union_opt(acc, element_frame_bounds(doc, e)))
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
    // re 31 §5.4 (Slot 2): eine Maske ohne Verknüpfung bleibt stehen, nur der Inhalt bewegt sich;
    // ein gehaltener Render-Cache stimmt dann nicht mehr und wird verworfen
    let unlinked = matches!(&e.kind, ElementKind::Group(g) if g.kind.is_mask() && g.mask.is_some() && !g.mask_flags.linked);
    if unlinked {
        e.cache = None;
    } else {
        e.move_cache(dx, dy);
    }
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
            if let Some(m) = &mut g.mask
                && (g.mask_flags.linked || !g.kind.is_mask())
            {
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
            for x in c
                .result
                .iter_mut()
                .chain(c.operands.iter_mut())
                .chain(c.proxy.iter_mut())
            {
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

/// Vorgabe der Voreinstellung `PixelTrimThreshold` (Pixel zählt, wenn Alpha > Wert).
pub const PIXEL_TRIM_THRESHOLD: i64 = 1;

/// Beschnitt einer Bitmap auf ihre deckenden Pixel wie im Original (re 27 L3/L8): Rechteck der
/// Pixel mit Alpha > `threshold` (Vorgabe 1, also Alpha ≥ 2); weicht es von den Bildgrenzen ab,
/// wird der Puffer darauf umkopiert, ein leeres Rechteck ergibt einen leeren Puffer an der linken
/// oberen Ecke. `None` = nichts geändert. Läuft nach Menüfiltern **ohne Pixelauswahl** (mit
/// Pixelauswahl, auch über die ganze Leinwand, bleiben die Objektgrenzen; Live-Filter beschneiden
/// nie, re 29b §4), In Bitmap konvertieren,
/// Zusammenfügen nach unten, an der Kopie beim Kopieren/Ausschneiden und beim Verlassen des
/// Bitmap-Modus nach einer Bearbeitung (Esc/Zeigerklick, auch nach Pinsel-/Radierzügen, re 28 L3);
/// der Zug selbst beschneidet nicht, solange der Modus läuft.
pub fn trim_to_alpha(img: &RgbaImage, threshold: i64) -> Option<RgbaImage> {
    let min = (threshold.clamp(0, 254) + 1) as u8;
    let b = img.bounds();
    let a = img.alpha_bounds_min(min);
    if a.is_empty() {
        return (!b.is_empty()).then(|| RgbaImage::empty_at(b.left, b.top));
    }
    (a != b).then(|| img.cropped(a))
}

/// Kopie eines Elements mit **jeder** Bitmap darin auf ihre deckenden Pixel beschnitten, auch in
/// Gruppen (rekursiv, Maske ausgenommen); Gruppengrenzen folgen dem Inhalt, weil sie aus den
/// Kindern berechnet werden (re 29b §4 C1/C2: Kopieren beschneidet auch in Gruppen). Unverändert
/// bleibt das übergebene Element.
pub fn trimmed_copy(e: &Arc<Element>, threshold: i64) -> Arc<Element> {
    match &e.kind {
        ElementKind::Bitmap(b) => {
            let mut b = b.clone();
            if !trim_bitmap(&mut b, threshold) {
                return Arc::clone(e);
            }
            let mut c = (**e).clone();
            c.kind = ElementKind::Bitmap(b);
            c.cache = None;
            Arc::new(c)
        }
        ElementKind::Group(g) => {
            let children: Vec<Arc<Element>> = g
                .children
                .iter()
                .map(|c| trimmed_copy(c, threshold))
                .collect();
            if children
                .iter()
                .zip(&g.children)
                .all(|(a, b)| Arc::ptr_eq(a, b))
            {
                return Arc::clone(e);
            }
            let mut c = (**e).clone();
            if let ElementKind::Group(cg) = &mut c.kind {
                cg.children = children;
            }
            c.cache = None;
            Arc::new(c)
        }
        _ => Arc::clone(e),
    }
}

/// [`trim_to_alpha`] auf ein Bitmap-Element; `true`, wenn sich etwas geändert hat.
pub fn trim_bitmap(b: &mut BitmapElem, threshold: i64) -> bool {
    let Some(img) = bitmap_image(b) else {
        return false;
    };
    match trim_to_alpha(&img, threshold) {
        Some(t) => {
            set_bitmap_image(b, &t);
            true
        }
        None => false,
    }
}

/// Bitmap als RGBA-Bild in Dokumentkoordinaten (Lage auf ganze Pixel gerundet).
pub fn bitmap_image(b: &BitmapElem) -> Option<RgbaImage> {
    let (rgba, _) = b.image.to_rgba();
    let p = b.image.position();
    let r = IRect::from_xywh(
        p.x.round() as i32,
        p.y.round() as i32,
        b.image.grid.width as i32,
        b.image.grid.height as i32,
    );
    RgbaImage::from_rgba_bytes(r, &rgba).ok()
}

/// Ersetzt die Pixel einer Bitmap (Lage aus den Bildgrenzen, Effekt-Ergebnis verworfen).
pub fn set_bitmap_image(b: &mut BitmapElem, img: &RgbaImage) {
    let r = img.bounds();
    let mut next = tile_counter();
    b.image.origin = Point::new(r.left as f32, r.top as f32);
    b.image.offset = Point::default();
    b.image.grid = TileGrid::from_rgba(
        img.width() as u32,
        img.height() as u32,
        img.as_bytes(),
        &mut next,
    );
    b.effect_result = None;
}

fn apply_pt(m: &Matrix3, p: &mut Point) {
    *p = m.apply(*p);
}

/// Pure positive scale (plus move) of a bitmap with the bicubic or soft setting: separable
/// resampling like the original (re 35 §3). Target rectangle: left/top = floor, width/height =
/// floor(v + 0.5) of the transformed float bounds. `None` = other transformation or sampler
/// (bilinear and nearest neighbour stay on the per-pixel sampler).
fn scale_bitmap_axis_aligned(
    img: &fp_raster::RgbaImage,
    m: &Matrix3,
    sampler: Sampler,
) -> Option<fp_raster::RgbaImage> {
    if !matches!(sampler, Sampler::Bicubic | Sampler::Soft) || img.is_empty() {
        return None;
    }
    let a = &m.m;
    if a[0][1] != 0.0
        || a[1][0] != 0.0
        || a[0][2] != 0.0
        || a[1][2] != 0.0
        || a[2][2] != 1.0
        || a[0][0] <= 0.0
        || a[1][1] <= 0.0
    {
        return None;
    }
    let r = m.transform_rect_bounds(&img.bounds().to_rect());
    let (l, t) = ((r.left as f64).floor(), (r.top as f64).floor());
    let (w, h) = (
        ((r.width() as f64) + 0.5).floor(),
        ((r.height() as f64) + 0.5).floor(),
    );
    if !(l.is_finite() && t.is_finite() && w >= 1.0 && h >= 1.0 && w < 1.0e6 && h < 1.0e6) {
        return None;
    }
    Some(fp_raster::resample::scale_separable(
        img,
        l as i32,
        t as i32,
        w as u32,
        h as u32,
        sampler == Sampler::Soft,
    ))
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
                let out = scale_bitmap_axis_aligned(&img, m, sampler)
                    .unwrap_or_else(|| fp_raster::resample::transform(&img, m, sampler, None));
                set_bitmap_image(b, &out);
            }
        }
        ElementKind::Group(g) => {
            // Skalieren/Drehen wie Verschieben: die Maske folgt nur verknüpft (re 31 §5.4, A)
            if let Some(x) = &mut g.mask
                && (g.mask_flags.linked || !g.kind.is_mask())
            {
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
            for x in c
                .result
                .iter_mut()
                .chain(c.operands.iter_mut())
                .chain(c.proxy.iter_mut())
            {
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

/// Layout boxes of a text for [`fold_text_scale`] (usually the session renderer's
/// [`crate::render::PageRenderer::text_fit_boxes`]).
pub type TextFitFn<'a> = &'a dyn Fn(&fp_model::TextElem) -> Option<crate::render::TextFitBoxes>;

/// A leftover scale whose set box differs from the target by less than this (px, per axis) becomes
/// the identity with a whole-pixel origin (RE spec 66 §3.3: 0.074 px measured as identity, 0.25 px
/// kept as a matrix; the threshold in between is our choice).
const FOLD_IDENTITY_PX: f64 = 0.15;

/// [`transform_element`] that folds a pure axis scale into the attributes of every text in `e`
/// (also inside groups, at any depth) with [`fold_text_scale`]; texts it cannot fold and every other
/// element get the matrix as before. Used by the paths of the reference that fold (RE spec 66 §3.1):
/// the numeric scale for every text of the selection, the Scale tool and `setQuadrangle` with
/// `rememberQuad` for a single selected text.
pub fn transform_element_folding_text(
    e: &mut Element,
    m: &Matrix3,
    sampler: Sampler,
    fit: TextFitFn,
) {
    if m.is_translation(1e-6) {
        transform_element(e, m, sampler);
        return;
    }
    match &mut e.kind {
        ElementKind::Text(t) => {
            if fold_text_scale(t, m, fit) {
                e.cache = None;
                return;
            }
        }
        ElementKind::Group(g) => {
            // the group's own parts (mask, replacement, smart shape) take the matrix; the children
            // are folded one by one
            let children = std::mem::take(&mut g.children);
            transform_element(e, m, sampler);
            if let ElementKind::Group(g) = &mut e.kind {
                g.children = children;
                for c in &mut g.children {
                    transform_element_folding_text(Arc::make_mut(c), m, sampler, fit);
                }
            }
            return;
        }
        _ => {}
    }
    transform_element(e, m, sampler);
}

/// Folds the transformation `m` (document coordinates, row vectors) into the attributes of the
/// text `t` instead of its matrix, as the reference does for a pure axis scale (RE spec 66 §3):
///
/// - only a scale `diag(sx, sy)` with `sx, sy > 0` (plus a move) folds; the text's existing matrix
///   may rotate, reflect and scale along its axes but not shear; the scale is turned into text
///   space and must stay diagonal there (uniform, or the text turned by a multiple of 90°);
/// - size × sy (to 0.01 pt, at least 4 pt), horizontal scale × sx/sy (to 0.001), exact leading ×
///   sy (percentage leading unchanged), range kerning × sx/sy, indent × sx, space before/after ×
///   sy; baseline shift, frame width and auto-expand stay;
/// - the text is laid out again and a leftover scale maps the new set box exactly onto the scaled
///   old one (point text: the layout frame `raw`, i.e. the sum of the rounded advances + 1 px wide
///   and the rounded set height; fixed-width text: the widest line, which only approximates the
///   reference's target, §2.5/§5); the rotation
///   and reflection of the old matrix stay. A leftover of about 1 on an unrotated text becomes no
///   matrix at all, the move goes into a whole-pixel origin.
///
/// Point text and fixed-width text written horizontally only; text on or in a path and vertical
/// text keep the matrix (not measured). Returns whether it folded (`false` = nothing changed).
pub fn fold_text_scale(t: &mut fp_model::TextElem, m: &Matrix3, fit: TextFitFn) -> bool {
    let a = &m.m;
    let (sx, sy) = (a[0][0] as f64, a[1][1] as f64);
    let tol = 1e-5 * sx.abs().max(sy.abs()).max(1.0);
    if !m.is_affine()
        || m.is_translation(1e-6)
        || (a[0][1] as f64).abs() > tol
        || (a[1][0] as f64).abs() > tol
        || sx <= 0.0
        || sy <= 0.0
    {
        return false;
    }
    let point = matches!(t.frame_type, None | Some(0));
    if !(point || t.frame_type == Some(1)) || t.on_path.is_some() || !matches!(t.orientation, 0 | 2)
    {
        return false;
    }
    let old_m = t.transform.unwrap_or(Matrix3::IDENTITY);
    if !old_m.is_affine() {
        return false;
    }
    // old matrix = D·R (row vectors): D = lengths of the rows, R = their directions
    let r0 = (old_m.m[0][0] as f64, old_m.m[0][1] as f64);
    let r1 = (old_m.m[1][0] as f64, old_m.m[1][1] as f64);
    let (dx, dy) = (r0.0.hypot(r0.1), r1.0.hypot(r1.1));
    if dx < 1e-9 || dy < 1e-9 {
        return false;
    }
    let (u0, u1) = ((r0.0 / dx, r0.1 / dx), (r1.0 / dy, r1.1 / dy));
    if (u0.0 * u1.0 + u0.1 * u1.1).abs() > 1e-4 {
        return false; // shear
    }
    // the scale in text space: R·diag(sx, sy)·Rᵀ
    let s00 = u0.0 * u0.0 * sx + u0.1 * u0.1 * sy;
    let s01 = u0.0 * u1.0 * sx + u0.1 * u1.1 * sy;
    let s11 = u1.0 * u1.0 * sx + u1.1 * u1.1 * sy;
    if s01.abs() > 1e-5 * s00.max(s11) {
        return false; // a non-uniform scale across a turned text shears it
    }
    let Some(old) = fit(t) else {
        return false;
    };
    let mut n = t.clone();
    scale_text_attrs(&mut n, s00, s11);
    let Some(new) = fit(&n) else {
        return false;
    };
    // set boxes: point text = the layout frame (`raw`, width = sum of the rounded advances + 1,
    // height from the set; RE spec 66 §3.3, run 40: tool 1.3957 → [0.993, 1.005], t 5.085/23.883),
    // fixed-width text = the widest line
    let set_box = |b: &crate::render::TextFitBoxes| {
        if point {
            let f = b.frame;
            [f.left as f64, f.top as f64, f.right as f64, f.bottom as f64]
        } else {
            b.lines
        }
    };
    let (ob, nb) = (set_box(&old), set_box(&new));
    let wh = |b: &[f64; 4]| ((b[2] - b[0]).max(1e-6), (b[3] - b[1]).max(1e-6));
    let ((ow, oh), (nw, nh)) = (wh(&ob), wh(&nb));
    let (tw, th) = (ow * dx * s00, oh * dy * s11);
    let (fx, fy) = (tw / nw, th / nh);
    // corner of the old set box after the old matrix and the scale
    let p0 = m.apply_vec2(old_m.apply_vec2(fp_geom::Vec2::new(ob[0], ob[1])));
    let unturned = (u0.0 - 1.0).abs() < 1e-6
        && u0.1.abs() < 1e-6
        && u1.0.abs() < 1e-6
        && (u1.1 - 1.0).abs() < 1e-6;
    let identity =
        unturned && (tw - nw).abs() < FOLD_IDENTITY_PX && (th - nh).abs() < FOLD_IDENTITY_PX;
    n.bounds = new.frame;
    if identity {
        // whole-pixel origin (81.5 → 82 measured; 75.5 → 75 in a group is open, §5)
        let ddx = ((p0.x - nb[0]) + 0.5 + 1e-6).floor() as f32;
        let ddy = ((p0.y - nb[1]) + 0.5 + 1e-6).floor() as f32;
        n.bounds = n.bounds.translate(ddx, ddy);
        if let Some(r) = &mut n.anchor_rect {
            r.left += ddx;
            r.top += ddy;
            if r.right != 0.0 || r.bottom != 0.0 {
                r.right += ddx;
                r.bottom += ddy;
            }
        }
        n.transform = None;
    } else {
        let (a, b, c, d) = (fx * u0.0, fx * u0.1, fy * u1.0, fy * u1.1);
        let (nx, ny) = (nb[0], nb[1]);
        let tx = p0.x - (nx * a + ny * c);
        let ty = p0.y - (nx * b + ny * d);
        n.transform = Some(Matrix3::from_affine(
            a as f32, b as f32, c as f32, d as f32, tx as f32, ty as f32,
        ));
    }
    n.detach_story();
    n.relaid = true;
    *t = n;
    true
}

/// The attribute part of [`fold_text_scale`] for the scale `(sx, sy)` in text space (RE spec 66
/// §3.2), in the initial attributes and every run.
fn scale_text_attrs(t: &mut fp_model::TextElem, sx: f64, sy: f64) {
    let ratio = sx / sy;
    let size = |v: f32| ((v as f64 * sy * 100.0).round() / 100.0).max(4.0) as f32;
    let hs = |v: f32| ((v as f64 * ratio * 1000.0).round() / 1000.0) as f32;
    let by = |v: f32, k: f64| (v as f64 * k) as f32;
    let defaults = fp_text::CharAttrs::default();
    // percentage leading (mode 0) stays; exact and additional leading are pixels and scale
    let scale = |at: &mut fp_model::TextAttrs, mode: &mut i32| {
        if let Some(m) = at.leading_mode {
            *mode = m;
        }
        at.size = at.size.map(size);
        at.horizontal_scale = at.horizontal_scale.map(hs);
        at.tracking = at.tracking.map(|v| by(v, ratio));
        at.indent = at.indent.map(|v| by(v, sx));
        at.space_before = at.space_before.map(|v| by(v, sy));
        at.space_after = at.space_after.map(|v| by(v, sy));
        if *mode != 0 {
            at.leading = at.leading.map(|v| by(v, sy));
        }
    };
    // the initial attributes carry the size and horizontal scale every run inherits
    if t.initial.size.is_none() {
        t.initial.size = Some(defaults.size as f32);
    }
    if t.initial.horizontal_scale.is_none() && (ratio - 1.0).abs() > 1e-9 {
        t.initial.horizontal_scale = Some(defaults.h_scale as f32);
    }
    let mut mode = 0;
    scale(&mut t.initial, &mut mode);
    for r in &mut t.runs {
        scale(&mut r.attrs, &mut mode);
    }
    if let Some(r) = &mut t.trailing_attrs {
        scale(&mut r.attrs, &mut mode);
    }
}

/// Puts the fill handles of `e` (paths, also inside groups and compound shapes) back to those of
/// `orig`, the same element before a transformation. This is a scale or rotation with "Scale
/// attributes" off: the fill handles stay in the document (RE spec 61 §1.5). Text is not touched:
/// [`transform_element`] never moves a text's fill handles except in a pure move (RE spec 62 §2.4).
pub fn restore_fill_handles(e: &mut Element, orig: &Element) {
    match (&mut e.kind, &orig.kind) {
        (ElementKind::Path(p), ElementKind::Path(o)) => {
            p.fill_handles = o.fill_handles;
            if let (Some(pr), Some(op)) = (&mut p.primitive, &o.primitive) {
                pr.fill_handles = op.fill_handles;
            }
        }
        (ElementKind::Group(g), ElementKind::Group(o)) => {
            for (c, oc) in g.children.iter_mut().zip(&o.children) {
                restore_fill_handles(Arc::make_mut(c), oc);
            }
        }
        (ElementKind::CompoundShape(c), ElementKind::CompoundShape(o)) => {
            for (x, ox) in c.result.iter_mut().zip(&o.result) {
                restore_fill_handles(Arc::make_mut(x), ox);
            }
            for (x, ox) in c.operands.iter_mut().zip(&o.operands) {
                restore_fill_handles(Arc::make_mut(x), ox);
            }
        }
        _ => {}
    }
}

/// Applies `m` to the fill handles (document coordinates) of the texts in `e`, also inside groups.
/// [`transform_element`] leaves a text's fill handles where they are for anything but a pure move,
/// as the Scale tool and Free Transform do in the reference; the numeric transform
/// (`scaleSelection`) transforms them about the centre whatever "Scale attributes" says
/// (RE spec 62 §2.4, §2.6 Nr. 4/5). Call it after [`transform_element`] with the same matrix.
pub fn transform_text_fill_handles(e: &mut Element, m: &Matrix3) {
    if m.is_translation(1e-6) {
        // a pure move already took the handles along (translate_element)
        return;
    }
    e.cache = None;
    match &mut e.kind {
        ElementKind::Text(t) => {
            apply_pt(m, &mut t.fill_handles.start);
            apply_pt(m, &mut t.fill_handles.end);
            apply_pt(m, &mut t.fill_handles.second);
        }
        ElementKind::Group(g) => {
            for c in &mut g.children {
                transform_text_fill_handles(Arc::make_mut(c), m);
            }
        }
        _ => {}
    }
}

/// Puts the fill handles of the texts in `e` (also inside groups) back to those of `orig`, the same
/// element before a transform-tool session that only moved it: the Scale tool and Free Transform
/// leave a text's fill handles in the document even then (RE spec 62 §2.4, §2.6 Nr. 4).
pub fn restore_text_fill_handles(e: &mut Element, orig: &Element) {
    match (&mut e.kind, &orig.kind) {
        (ElementKind::Text(t), ElementKind::Text(o)) => {
            if t.fill_handles != o.fill_handles {
                t.fill_handles = o.fill_handles;
                e.cache = None;
            }
        }
        (ElementKind::Group(g), ElementKind::Group(o)) => {
            for (c, oc) in g.children.iter_mut().zip(&o.children) {
                restore_text_fill_handles(Arc::make_mut(c), oc);
            }
        }
        _ => {}
    }
}

/// Matrix „Rechteck `from` auf Rechteck `to` abbilden“ (Skalieren + Verschieben).
pub fn rect_to_rect(from: Rect, to: Rect) -> Matrix3 {
    let sx = if from.width() != 0.0 {
        to.width() / from.width()
    } else {
        1.0
    };
    let sy = if from.height() != 0.0 {
        to.height() / from.height()
    } else {
        1.0
    };
    Matrix3::translation(-from.left, -from.top)
        .then(&Matrix3::scaling(sx, sy))
        .then(&Matrix3::translation(to.left, to.top))
}

/// Matrix „um Punkt `c` mit `m` transformieren“.
pub fn about(c: Point, m: &Matrix3) -> Matrix3 {
    Matrix3::translation(-c.x, -c.y)
        .then(m)
        .then(&Matrix3::translation(c.x, c.y))
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
        ElementKind::CompoundShape(c) => c
            .result
            .as_ref()
            .and_then(|r| path_attrs_of(r))
            .or_else(|| c.operands.iter().find_map(|o| path_attrs_of(o))),
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
        HitOptions {
            tolerance: 3.0,
            skip_locked: true,
            skip_hidden: true,
        }
    }
}

/// Trifft `p` das Element?
pub fn hit_element(
    doc: &Document,
    e: &Element,
    p: Point,
    geo: &dyn GeometryOps,
    opt: &HitOptions,
) -> bool {
    hit_depth(doc, e, p, geo, opt, 0)
}

fn hit_depth(
    doc: &Document,
    e: &Element,
    p: Point,
    geo: &dyn GeometryOps,
    opt: &HitOptions,
    depth: u32,
) -> bool {
    if depth > 24 || (opt.skip_hidden && !e.common.visible) {
        return false;
    }
    match &e.kind {
        ElementKind::Path(pe) => {
            let shape = PathShape {
                contours: pe.contours.clone(),
                even_odd: pe.even_odd,
            };
            let stroke = pe
                .attrs
                .brush
                .as_ref()
                .map(|b| b.diameter as f64)
                .unwrap_or(0.0);
            let filled = pe.attrs.fill.is_some()
                && pe.contours.iter().any(|c| c.closed || c.nodes.len() > 2);
            geo.hit_test(&shape, p, stroke, filled, opt.tolerance)
        }
        ElementKind::Group(g) => {
            if g.kind.is_mask()
                && g.mask_flags.enabled
                && let Some(m) = g.effective_mask()
                && let Some(mb) = element_bounds(doc, m)
                && !mb
                    .inflate(opt.tolerance as f32, opt.tolerance as f32)
                    .contains(p)
            {
                return false;
            }
            // „Füllung und Pinselstrich anzeigen“: auch die sichtbare Maske trifft (re 26 §3.1)
            if g.kind == fp_model::element::GroupKind::VectorMask
                && g.mask_flags.enabled
                && !g.mask_flags.hide_attrs
                && g.effective_mask()
                    .is_some_and(|m| hit_depth(doc, m, p, geo, opt, depth + 1))
            {
                return true;
            }
            g.children
                .iter()
                .any(|c| hit_depth(doc, c, p, geo, opt, depth + 1))
        }
        ElementKind::CompoundShape(c) => match &c.result {
            Some(r) => hit_depth(doc, r, p, geo, opt, depth + 1),
            None => c
                .operands
                .iter()
                .any(|o| hit_depth(doc, o, p, geo, opt, depth + 1)),
        },
        // Bitmaps treffen nur auf Pixeln mit Alpha > 0 (B-2b BA-007: Klick in die durchsichtige
        // Ecke einer gerasterten Ellipse wählt nichts, Ziehen von dort zieht einen Auswahlrahmen auf)
        ElementKind::Bitmap(b) => bitmap_hit(b, p),
        _ => element_bounds(doc, e).is_some_and(|r| {
            r.inflate(opt.tolerance as f32, opt.tolerance as f32)
                .contains(p)
        }),
    }
}

/// Treffer auf einer Bitmap: das Pixel unter `p` hat Alpha > 0 (Schwelle 0, BA-007).
fn bitmap_hit(b: &BitmapElem, p: Point) -> bool {
    let o = b.image.position();
    let (x, y) = ((p.x - o.x).floor(), (p.y - o.y).floor());
    if x < 0.0 || y < 0.0 {
        return false;
    }
    b.image.pixel(x as u32, y as u32).is_some_and(|px| px.a > 0)
}

/// Alle getroffenen Elemente direkt in den Cels des Status `frame`, oben zuerst (für Zeiger und
/// „Hintere auswählen“).
pub fn hits_at(
    doc: &Document,
    page: usize,
    frame: usize,
    p: Point,
    geo: &dyn GeometryOps,
    opt: &HitOptions,
) -> Vec<ElementPath> {
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
pub fn deep_hit(
    doc: &Document,
    path: &ElementPath,
    p: Point,
    geo: &dyn GeometryOps,
    opt: &HitOptions,
) -> ElementPath {
    let mut cur = path.clone();
    for _ in 0..32 {
        let Some(e) = doc.element_at(&cur) else { break };
        let ElementKind::Group(g) = &e.kind else {
            break;
        };
        let Some(i) = g
            .children
            .iter()
            .position(|c| hit_element(doc, c, p, geo, opt))
        else {
            break;
        };
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

/// Verschiebt den Status `from` an die Stelle `to` (Index nach dem Herausnehmen, CT-C-02): Inhalte
/// aller eigenen, nicht statusübergreifenden Ebenen und die Statusangaben (Name, Verzögerung …)
/// wandern mit.
pub fn move_frame(doc: &mut Document, page: usize, from: usize, to: usize) -> Option<()> {
    let p = doc.page_mut(page)?;
    let n = p.root.frame_count as usize;
    if from >= n || to >= n {
        return None;
    }
    if from == to {
        return Some(());
    }
    for s in &mut p.root.layers {
        if let LayerSlot::Own(l) = s {
            let l = Arc::make_mut(l);
            if l.shared_across_frames {
                continue;
            }
            while l.cels.len() < n {
                l.cels.push(Arc::new(Cel::empty()));
            }
            let c = l.cels.remove(from);
            l.cels.insert(to, c);
        }
    }
    if let Some(e) = p.root.export.as_mut()
        && !e.frames.is_empty()
    {
        while e.frames.len() < n {
            e.frames.push(Default::default());
        }
        let f = e.frames.remove(from);
        e.frames.insert(to, f);
    }
    Some(())
}

/// Verschiebt die Seite `from` an die Stelle `to` (Index nach dem Herausnehmen, CT-C-03).
pub fn move_page(doc: &mut Document, from: usize, to: usize) -> Option<()> {
    let n = doc.pages.len();
    if from >= n || to >= n {
        return None;
    }
    if from == to {
        return Some(());
    }
    let pg = doc.pages.remove(from);
    doc.pages.insert(to, pg);
    doc.current_page = moved_index(doc.current_page, from, to);
    doc.invalidate_root_copy();
    Some(())
}

/// Neue Lage des Eintrags `i`, wenn `from` nach `to` verschoben wird.
pub fn moved_index(i: usize, from: usize, to: usize) -> usize {
    if i == from {
        to
    } else if from < i && i <= to {
        i - 1
    } else if to <= i && i < from {
        i + 1
    } else {
        i
    }
}

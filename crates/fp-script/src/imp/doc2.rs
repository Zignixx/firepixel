//! Document-DOM, Teil 2: Ansicht/Raster/Hilfslinien, Auswahl, Ebenen/Seiten/Status, Symbole, Text,
//! Pfade, Masken, Web-Objekte, Stile und automatische Formen. Methoden, die `doc.rs` nicht kennt,
//! landen hier; was auch hier fehlt, meldet „nicht implementiert“.

use super::doc::{arg_str, bounds, current_layer_index, flat_entries, flat_entry, jserr, layer_id, on_selection, rect_arr, selection, top_layer_id};
use super::{element, exec, ni, wrap_ids};
use crate::bindings::hr;
use crate::conv::{self, Arg};
use crate::state::{st, with_state, FwErr, FwResult, Handle};
use boa_engine::{Context, JsValue};
use fp_engine::DocId;
use fp_model::{ElementId, ElementKind, LayerSlot};
use serde_json::{json, Value};
use std::sync::Arc;

const CLS: &str = "Document";

fn u() -> FwResult<JsValue> {
    Ok(JsValue::undefined())
}

fn ex(doc: DocId, name: &str, p: Value) -> FwResult<JsValue> {
    exec(Some(doc), name, p).map(|_| JsValue::undefined())
}

/// Ansichtszustand lesen.
fn view<R>(doc: DocId, f: impl FnOnce(&fp_engine::view::ViewState) -> R) -> FwResult<R> {
    st(|s| s.session.document(doc).map(|d| f(&d.view)).ok_or_else(|| FwErr::bad(fp_i18n::t("err-script-doc-closed"))))
}

/// Schalter über den Umschaltbefehl auf `want` bringen.
fn toggle_to(doc: DocId, cmd: &str, cur: bool, want: bool) -> FwResult<JsValue> {
    if cur != want {
        exec(Some(doc), cmd, json!({}))?;
    }
    u()
}

fn page_root<R>(doc: DocId, f: impl FnOnce(&fp_model::Page, usize, usize) -> R) -> FwResult<R> {
    st(|s| {
        let d = s.session.document(doc).ok_or_else(|| FwErr::bad(fp_i18n::t("err-script-doc-closed")))?;
        let p = d.doc.pages.get(d.page).ok_or_else(|| FwErr::bad("Seite"))?;
        Ok(f(p, d.page, d.frame))
    })
}

fn hex(c: fp_color::Argb) -> String {
    fp_engine::commands::argb_hex(c)
}

fn color_arg(a: &Arg) -> Option<String> {
    a.str().and_then(|c| conv::norm_color(&c))
}

/// Alle Elemente der aktuellen Seite/des aktuellen Status (tief), mit Namen.
fn all_ids(doc: DocId) -> FwResult<Vec<(ElementId, Option<String>)>> {
    st(|s| {
        let d = s.session.document(doc).ok_or_else(|| FwErr::bad("Dokument"))?;
        let mut out = Vec::new();
        fn walk(e: &fp_model::Element, out: &mut Vec<(ElementId, Option<String>)>) {
            out.push((e.id, e.common.name.clone()));
            if let ElementKind::Group(g) = &e.kind {
                for c in &g.children {
                    walk(c, out);
                }
            }
        }
        if let Some(p) = d.doc.pages.get(d.page) {
            for slot in &p.root.layers {
                if let LayerSlot::Own(l) = slot
                    && let Some(c) = l.cel(d.frame)
                {
                    for e in &c.elements {
                        walk(e, &mut out);
                    }
                }
            }
        }
        Ok(out)
    })
}

fn ids_named(doc: DocId, name: &str) -> FwResult<Vec<ElementId>> {
    Ok(all_ids(doc)?.into_iter().filter(|(_, n)| n.as_deref() == Some(name)).map(|(i, _)| i).collect())
}

fn ids_json(ids: &[ElementId]) -> Value {
    json!(ids.iter().map(|i| i.0).collect::<Vec<_>>())
}

/// Aktive Ebene setzen (für Befehle, die auf der aktiven Ebene arbeiten).
/// `idx` zählt flach wie `dom.layers`; bei einer Unterebene wird ihre Ebene der obersten Stufe aktiv.
fn activate_layer(doc: DocId, idx: i64) -> FwResult<()> {
    if idx >= 0
        && let Some(id) = top_layer_id(doc, idx)?
    {
        exec(Some(doc), "layer.setActive", json!({"layer": id}))?;
    }
    Ok(())
}

/// Dupliziert eine Unterebene `n`-mal, jede Kopie direkt darüber (neue Element- und Ebenen-IDs).
fn duplicate_sublayer(doc: DocId, sub: ElementId, n: usize) -> FwResult<()> {
    super::edit_doc(doc, |d| {
        let page = d.page;
        let m = d.doc_mut();
        for _ in 0..n {
            let Some(path) = fp_engine::docops::find_on_page(m, page, sub) else { return };
            let Some(e) = m.element_at(&path).cloned() else { return };
            let c = fp_engine::docops::reassign(m, &e);
            if let Some((list, idx)) = fp_engine::docops::parent_list_mut(m, &path) {
                list.insert(idx, Arc::new(c));
            }
        }
    })
}

/// Verschiebt eine Unterebene an die Stelle eines anderen Eintrags der flachen Ebenenliste.
/// `below`: unter das Ziel (es rückt in der flachen Liste dahinter), sonst darüber.
fn move_sublayer(doc: DocId, sub: ElementId, target: &super::elem::FlatLayer, below: bool) -> FwResult<()> {
    if target.sub == Some(sub) {
        return Ok(());
    }
    super::edit_doc(doc, |d| {
        let page = d.page;
        let m = d.doc_mut();
        let Some(from) = fp_engine::docops::find_on_page(m, page, sub) else { return };
        // Ziel innerhalb der verschobenen Unterebene: nichts tun
        if let Some(t) = target.sub
            && fp_engine::docops::find_on_page(m, page, t).is_some_and(|p| p.slot == from.slot && p.steps.starts_with(&from.steps))
        {
            return;
        }
        let removed = fp_engine::docops::remove_ids(m, page, &[sub]);
        let Some((_, e)) = removed.into_iter().next() else { return };
        match target.sub {
            Some(t) => {
                if let Some(p) = fp_engine::docops::find_on_page(m, page, t)
                    && let Some((list, idx)) = fp_engine::docops::parent_list_mut(m, &p)
                {
                    list.insert(if below { idx + 1 } else { idx }.min(list.len()), e);
                }
            }
            None => {
                // Ziel = Ebene der obersten Stufe (B-2b BA-123, gemessen): die Unterebene verlässt
                // ihre Elternebene und wird Ebene der obersten Stufe direkt unter dem Ziel (flach
                // vor dem Ziel), in beiden Richtungen
                let frames = m.pages.get(page).map(|p| p.root.frame_count as usize).unwrap_or(1).max(1);
                let ElementKind::SubLayer(sl) = &e.kind else { return };
                let mut l = (**sl).clone();
                if !l.shared_across_frames {
                    while l.cels.len() < frames {
                        l.cels.push(Arc::new(fp_model::Cel::empty()));
                    }
                }
                if let Some(p) = m.page_mut(page) {
                    let at = target.slot.min(p.root.layers.len());
                    p.root.layers.insert(at, fp_model::LayerSlot::Own(Arc::new(l)));
                }
            }
        }
    })
}

/// Verschiebt Elemente oben in eine Unterebene (aktueller Status), Reihenfolge bleibt.
/// Die Unterebene selbst und ihre Vorfahren bleiben, wo sie sind.
fn move_into_sublayer(doc: DocId, ids: &[ElementId], sub: ElementId) -> FwResult<()> {
    super::edit_doc(doc, |d| {
        let (page, frame) = (d.page, d.frame);
        let m = d.doc_mut();
        let Some(target) = fp_engine::docops::find_on_page(m, page, sub) else { return };
        // nicht die Unterebene in sich selbst verschieben
        let blocked: Vec<ElementId> = ids.iter().copied().filter(|id| *id == sub || fp_engine::docops::find_on_page(m, page, *id).is_some_and(|p| target.steps.starts_with(&p.steps) && p.slot == target.slot)).collect();
        let movable: Vec<ElementId> = ids.iter().copied().filter(|id| !blocked.contains(id)).collect();
        // in Stapelfolge (oben zuerst) wie in der Ebene
        let order: Vec<ElementId> = fp_engine::docops::selectable_ids(m, page, frame).into_iter().filter(|i| movable.contains(i)).collect();
        let mut removed = fp_engine::docops::remove_ids(m, page, &movable);
        removed.sort_by_key(|(_, e)| order.iter().position(|i| *i == e.id).unwrap_or(usize::MAX));
        let els: Vec<Arc<fp_model::Element>> = removed.into_iter().map(|(_, e)| e).collect();
        fp_engine::docops::edit_by_id(m, page, sub, |e| {
            if let ElementKind::SubLayer(l) = &mut e.kind {
                let l = Arc::make_mut(l);
                let ci = if frame < l.cels.len() { frame } else { 0 };
                if let Some(c) = l.cels.get_mut(ci) {
                    let c = Arc::make_mut(c);
                    for (k, x) in els.into_iter().enumerate() {
                        c.elements.insert(k.min(c.elements.len()), x);
                    }
                }
            }
        });
    })
}

fn pt_arg(a: &Arg) -> FwResult<(f64, f64)> {
    a.point().ok_or_else(|| FwErr::Hr(hr::E_BAD_PARAM_TYPE, "Punkt".into()))
}

fn text_attr(doc: DocId, key: &str, v: Value) -> FwResult<JsValue> {
    on_selection(doc, "text.setAttributes", json!({ key: v }))
}

/// Erstes ausgewähltes Element mit automatischer Form.
fn selected_shape(doc: DocId) -> FwResult<ElementId> {
    for id in selection(doc)? {
        let e = element(doc, id)?;
        if matches!(&e.kind, ElementKind::Group(g) if g.smart_shape.is_some()) {
            return Ok(id);
        }
    }
    Err(FwErr::Hr(hr::E_BAD_SELECTION, "keine automatische Form ausgewählt".into()))
}

/// Automatische Formen über fp-script (`crate::autoshape`) im Zieldokument.
fn autoshape(doc: DocId, p: Value) -> FwResult<Value> {
    st(|s| {
        let prev = s.session.active_id();
        if prev != Some(doc) {
            s.session.activate(doc).map_err(FwErr::from)?;
        }
        let r = crate::autoshape::op_with_history(&mut s.session, &p).map_err(|e| FwErr::generic(e.to_string()));
        if let Some(p) = prev
            && p != doc
        {
            let _ = s.session.activate(p);
        }
        r
    })
}

/// Elementmaske der Auswahl ändern (direkt im Modell).
fn edit_mask_groups(doc: DocId, f: impl Fn(&mut fp_model::element::GroupElem)) -> FwResult<()> {
    let ids = selection(doc)?;
    super::edit_doc(doc, |d| {
        let page = d.page;
        let m = d.doc_mut();
        for id in ids {
            fp_engine::docops::edit_by_id(m, page, id, |e| {
                if let ElementKind::Group(g) = &mut e.kind
                    && g.kind.is_mask()
                {
                    f(g);
                    e.cache = None;
                }
            });
        }
    })
}

fn web_params(doc: DocId, p: Value) -> FwResult<JsValue> {
    on_selection(doc, "pi.web", p)
}

/// Pfadknoten (Kontur, Knoten) eines ausgewählten Pfads ändern.
fn edit_first_path(doc: DocId, f: impl FnOnce(&mut Vec<fp_model::element::Contour>) -> FwResult<()>) -> FwResult<()> {
    let sel = selection(doc)?;
    let id = sel.into_iter().find(|i| element(doc, *i).is_ok_and(|e| matches!(e.kind, ElementKind::Path(_)))).ok_or_else(|| FwErr::Hr(hr::E_BAD_SELECTION, "kein Pfad ausgewählt".into()))?;
    let e = element(doc, id)?;
    let mut cs = match &e.kind {
        ElementKind::Path(p) => p.contours.clone(),
        _ => Vec::new(),
    };
    f(&mut cs)?;
    let js: Vec<Value> = cs
        .iter()
        .map(|c| json!({"closed": c.closed, "nodes": c.nodes.iter().map(|n| json!({"anchor": [n.anchor.x, n.anchor.y], "in": [n.pred.x, n.pred.y], "out": [n.succ.x, n.succ.y]})).collect::<Vec<_>>()}))
        .collect();
    exec(Some(doc), "path.setContours", json!({"id": id.0, "contours": js}))?;
    Ok(())
}

fn node(x: f64, y: f64) -> fp_model::element::ContourNode {
    let p = fp_geom::Point::new(x as f32, y as f32);
    fp_model::element::ContourNode { anchor: p, pred: p, succ: p, curve: false, random: None, segment_length: None, pressure: None, extra: Default::default() }
}

/// Elemente der Auswahl in andere Status kopieren/verschieben (direkt im Modell).
fn selection_to_frames(doc: DocId, frames: Vec<usize>, keep: bool) -> FwResult<()> {
    let ids = selection(doc)?;
    if ids.is_empty() {
        return Ok(());
    }
    super::edit_doc(doc, |d| {
        let (page, cur) = (d.page, d.frame);
        let m = d.doc_mut();
        let Some(pg) = m.pages.get(page).cloned() else { return };
        let count = pg.root.frame_count.max(1) as usize;
        for (slot, ls) in pg.root.layers.iter().enumerate() {
            let LayerSlot::Own(l) = ls else { continue };
            let Some(cel) = l.cel(cur) else { continue };
            let picked: Vec<Arc<fp_model::Element>> = cel.elements.iter().filter(|e| ids.contains(&e.id)).cloned().collect();
            if picked.is_empty() {
                continue;
            }
            for &f in &frames {
                if f >= count || f == cur {
                    continue;
                }
                let copies: Vec<Arc<fp_model::Element>> = picked.iter().map(|e| Arc::new(fp_engine::docops::reassign(m, e))).collect();
                if let Some(c) = fp_engine::docops::cel_mut(m, page, slot, f) {
                    for (k, e) in copies.into_iter().enumerate() {
                        c.elements.insert(k, e);
                    }
                }
            }
            if !keep && let Some(c) = fp_engine::docops::cel_mut(m, page, slot, cur) {
                c.elements.retain(|e| !ids.contains(&e.id));
            }
        }
        if !keep {
            d.selection.elements.clear();
        }
    })
}

pub(super) fn method(doc: DocId, name: &str, args: &[Arg], ctx: &mut Context) -> FwResult<JsValue> {
    let a = |i: usize| args.get(i).cloned().unwrap_or(Arg::Null);
    match name {
        // ---------------------------------------------------------- Ansicht
        "getShowGrid" => Ok(JsValue::from(view(doc, |v| v.show_grid)?)),
        "getSnapToGrid" => Ok(JsValue::from(view(doc, |v| v.snap_to_grid)?)),
        "getShowGuides" => Ok(JsValue::from(view(doc, |v| v.show_guides)?)),
        "getSnapToGuides" => Ok(JsValue::from(view(doc, |v| v.snap_to_guides)?)),
        "getLockGuides" => Ok(JsValue::from(view(doc, |v| v.lock_guides)?)),
        "getShowSmartGuides" => Ok(JsValue::from(view(doc, |v| v.show_smart_guides)?)),
        "getSnapToSmartGuides" => Ok(JsValue::from(view(doc, |v| v.snap_to_smart_guides)?)),
        "getShowRulers" => Ok(JsValue::from(view(doc, |v| v.show_rulers)?)),
        "getShowTooltips" => Ok(JsValue::from(with_state(|s| s.session.ui.tooltips)?)),
        "setShowGrid" => toggle_to(doc, "view.grid.showGrid", view(doc, |v| v.show_grid)?, a(0).bool()),
        "setSnapToGrid" => toggle_to(doc, "view.grid.snapToGrid", view(doc, |v| v.snap_to_grid)?, a(0).bool()),
        "setShowGuides" => toggle_to(doc, "view.guides.showGuides", view(doc, |v| v.show_guides)?, a(0).bool()),
        "setSnapToGuides" => toggle_to(doc, "view.guides.snapToGuides", view(doc, |v| v.snap_to_guides)?, a(0).bool()),
        "lockGuides" => toggle_to(doc, "view.guides.lockGuides", view(doc, |v| v.lock_guides)?, a(0).bool()),
        "setShowSmartGuides" => toggle_to(doc, "view.smartGuides.showSmartGuides", view(doc, |v| v.show_smart_guides)?, a(0).bool()),
        "setSnapToSmartGuides" => toggle_to(doc, "view.smartGuides.snapToSmartGuides", view(doc, |v| v.snap_to_smart_guides)?, a(0).bool()),
        "setShowRulers" => toggle_to(doc, "view.rulers", view(doc, |v| v.show_rulers)?, a(0).bool()),
        "setShowSliceGuides" => toggle_to(doc, "view.sliceGuides", view(doc, |v| v.show_slice_guides)?, a(0).bool()),
        "setShowSliceOverlay" => toggle_to(doc, "view.sliceOverlay", view(doc, |v| v.show_slice_overlay)?, a(0).bool()),
        "setShowEdges" => toggle_to(doc, "view.edges", view(doc, |v| v.show_edges)?, a(0).bool()),
        "setShowGammaPreview" => toggle_to(doc, "view.macintoshGamma", view(doc, |v| v.gamma_preview)?, a(0).bool()),
        "setWebObjectsVisibility" => toggle_to(doc, "view.toggleSlices", !view(doc, |v| v.hide_slices)?, a(0).bool()),
        "lockNineScale" | "lockAllNineScale" => toggle_to(doc, "view.nineSliceScalingGuides.lockGuides", view(doc, |v| v.lock_nine_slice_guides)?, a(0).bool()),
        "setShowTooltips" => {
            let cur = with_state(|s| s.session.ui.tooltips)?;
            toggle_to(doc, "view.tooltips", cur, a(0).bool())
        }
        "getSmartGuidesColor" => Ok(conv::s(&with_state(|s| s.session.prefs.get_str("SmartGuideColor", "#ff00ff"))?)),
        "setSmartGuidesColor" | "setSliceGuideColor" | "setNineScaleGuideColor" => {
            let c = color_arg(&a(0)).unwrap_or_else(|| "#ff00ff".into());
            let key = match name {
                "setSmartGuidesColor" => "SmartGuideColor",
                "setSliceGuideColor" => "SliceGuideColor",
                _ => "NineScaleGuideColor",
            };
            exec(Some(doc), "edit.preferences", json!({"key": key, "value": c}))?;
            u()
        }
        "setGridColor" => ex(doc, "view.grid.editGrid", json!({"color": color_arg(&a(0)).unwrap_or_else(|| "#666666".into())})),
        "setGridSize" => {
            let (w, h) = pt_arg(&a(0))?;
            ex(doc, "view.grid.editGrid", json!({"width": w.max(1.0), "height": h.max(1.0)}))
        }
        "setGridOrigin" => {
            let (x, y) = pt_arg(&a(0))?;
            ex(doc, "view.grid.editGrid", json!({"origin": [x, y]}))
        }
        "setGuideColor" => ex(doc, "view.guides.editGuides", json!({"color": color_arg(&a(0)).unwrap_or_else(|| "#00ffff".into())})),
        "addGuide" => {
            let pos = a(0).f64();
            let horiz = arg_str(args, 1).eq_ignore_ascii_case("horizontal");
            ex(doc, "guide.add", json!({"orientation": if horiz { "horizontal" } else { "vertical" }, "position": pos}))
        }
        "removeGuide" | "changeGuide" => {
            // removeGuide(position, kind) / changeGuide(oldPos, newPos, kind)
            let horiz_arg = if name == "removeGuide" { 1 } else { 2 };
            let horiz = arg_str(args, horiz_arg).eq_ignore_ascii_case("horizontal");
            let old = a(0).f64() as f32;
            let new_pos = a(1).f64() as f32;
            super::edit_doc(doc, |d| {
                let page = d.page;
                if let Some(p) = d.doc_mut().page_mut(page) {
                    let g = p.root.guides.get_or_insert_with(Default::default);
                    let list = if horiz { &mut g.horizontal } else { &mut g.vertical };
                    if let Some(i) = list.iter().position(|v| (*v - old).abs() < 0.5) {
                        if name == "removeGuide" {
                            list.remove(i);
                        } else {
                            list[i] = new_pos;
                        }
                    }
                }
            })?;
            u()
        }
        "removeAllGuides" => {
            let kind = arg_str(args, 0).to_lowercase();
            if kind.is_empty() {
                return ex(doc, "view.guides.clearGuides", json!({}));
            }
            super::edit_doc(doc, |d| {
                let page = d.page;
                if let Some(p) = d.doc_mut().page_mut(page)
                    && let Some(g) = &mut p.root.guides
                {
                    if kind.starts_with('h') { g.horizontal.clear() } else { g.vertical.clear() }
                }
            })?;
            u()
        }
        "redraw" | "rebuildColorTable" | "setUseAGMRenderingForSelection" => {
            if name == "setUseAGMRenderingForSelection" {
                let on = a(0).bool();
                let ids = selection(doc)?;
                super::edit_doc(doc, |d| {
                    let page = d.page;
                    for id in ids {
                        fp_engine::docops::edit_by_id(d.doc_mut(), page, id, |e| {
                            if let ElementKind::Path(p) = &mut e.kind {
                                p.agm = on;
                                e.cache = None;
                            }
                        });
                    }
                })?;
            }
            u()
        }
        // ---------------------------------------------------------- Auswahl
        "selectChildren" => on_selection(doc, "select.subselect", json!({})),
        "selectParents" => on_selection(doc, "select.superselect", json!({})),
        "selectSimilar" => on_selection(doc, "select.similar", json!({})),
        "selectSimilarFromPoint" => {
            let (x, y) = pt_arg(&a(0))?;
            ex(doc, "select.magicWand", json!({"point": [x, y], "mode": "add", "tolerance": a(1).i64().clamp(0, 255)}))
        }
        "selectAdjustPixelSel" => {
            let how = arg_str(args, 0).to_lowercase();
            let amt = a(1).i64().max(1);
            let c = if how.contains("contract") {
                "select.contractMarquee"
            } else if how.contains("border") {
                "select.borderMarquee"
            } else if how.contains("smooth") {
                "select.smoothMarquee"
            } else {
                "select.expandMarquee"
            };
            let has = with_state(|s| s.session.document(doc).is_some_and(|d| d.selection.has_pixels()))?;
            if has { ex(doc, c, json!({"amount": amt})) } else { u() }
        }
        "convertMarqueeToPath" => {
            let has = with_state(|s| s.session.document(doc).is_some_and(|d| d.selection.has_pixels()))?;
            if has { ex(doc, "select.convertMarqueeToPath", json!({})) } else { u() }
        }
        "saveSelection" => ex(doc, "select.saveBitmapSelection", json!({"name": arg_str(args, 0)})),
        "restoreSelection" => {
            let mut p = json!({});
            match a(0) {
                Arg::Num(n) => p["index"] = json!(n as i64),
                other => p["name"] = json!(other.str().unwrap_or_default()),
            }
            ex(doc, "select.restoreBitmapSelection", p)
        }
        "deleteSavedSelection" => {
            let mut p = json!({});
            match a(0) {
                Arg::Num(n) => p["index"] = json!(n as i64),
                other => p["name"] = json!(other.str().unwrap_or_default()),
            }
            ex(doc, "select.deleteBitmapSelection", p)
        }
        "findNamedElements" => {
            let ids = ids_named(doc, &arg_str(args, 0))?;
            wrap_ids(ctx, doc, &ids)
        }
        "setElementVisibleByName" | "setElementLockedByName" => {
            let ids = ids_named(doc, &arg_str(args, 0))?;
            if ids.is_empty() {
                return u();
            }
            let flag = a(1).bool();
            if name == "setElementVisibleByName" {
                ex(doc, "element.setVisible", json!({"ids": ids_json(&ids), "visible": flag}))
            } else {
                ex(doc, "element.setLocked", json!({"ids": ids_json(&ids), "locked": flag}))
            }
        }
        "filterSelectionByID" => {
            // (id, effectObject): Filter über die Effekt-ID
            let mut e = a(1).json();
            if let Value::Object(m) = &mut e {
                m.entry("EffectMoaID").or_insert(json!(arg_str(args, 0)));
            }
            super::doc::method(doc, "filterSelection", &[Arg::Json(e)], ctx)
        }
        "isSelectionDirectlyAboveBitmapObject" => {
            let ids = selection(doc)?;
            let r = st(|s| {
                let d = s.session.document(doc).ok_or_else(|| FwErr::bad("Dokument"))?;
                let Some(first) = ids.first() else { return Ok(false) };
                let Some(p) = d.doc.pages.get(d.page) else { return Ok(false) };
                for slot in &p.root.layers {
                    if let LayerSlot::Own(l) = slot
                        && let Some(c) = l.cel(d.frame)
                        && let Some(i) = c.elements.iter().position(|e| e.id == *first)
                    {
                        return Ok(c.elements.get(i + 1).is_some_and(|e| matches!(e.kind, ElementKind::Bitmap(_))));
                    }
                }
                Ok(false)
            })?;
            Ok(JsValue::from(r))
        }
        "deleteAllInDocument" => {
            ex(doc, "select.all", json!({}))?;
            on_selection(doc, "edit.clear", json!({}))
        }
        // ---------------------------------------------------------- Ebenen
        "moveSelectionToLayer" | "moveSelectionToNewLayer" => {
            let ids = selection(doc)?;
            if ids.is_empty() {
                return u();
            }
            // layerIndex flach wie `dom.layers` (Unterebenen mitgezählt, Befund F6-1)
            let (target, sub) = if name == "moveSelectionToNewLayer" {
                exec(Some(doc), "layer.add", json!({}))?;
                (layer_id(doc, -1)?, None)
            } else if a(0).i64() < 0 {
                (layer_id(doc, -1)?, None)
            } else {
                let e = flat_entry(doc, a(0).i64())?;
                (e.id, e.sub)
            };
            let Some(layer) = target else { return Err(FwErr::bad("Ebene")) };
            let copy = if name == "moveSelectionToLayer" { a(1).bool() } else { a(0).bool() };
            if copy {
                exec(Some(doc), "edit.duplicate", json!({}))?;
            }
            let ids = selection(doc)?;
            if let Some(sub) = sub {
                move_into_sublayer(doc, &ids, sub)?;
                return u();
            }
            for (k, id) in ids.iter().enumerate() {
                exec(Some(doc), "pi.element.moveTo", json!({"id": id.0, "layer": layer, "index": k}))?;
            }
            u()
        }
        "duplicateLayer" => {
            let idx = a(0).i64();
            let n = a(1).i64().clamp(1, 100);
            // Unterebene: Kopie direkt darüber in derselben Elternebene (BA-075)
            if idx >= 0
                && let Some(sub) = flat_entry(doc, idx)?.sub
            {
                duplicate_sublayer(doc, sub, n as usize)?;
                return u();
            }
            let id = layer_id(doc, idx)?.ok_or_else(|| FwErr::bad("Ebene"))?;
            for _ in 0..n {
                exec(Some(doc), "layer.duplicate", json!({"layer": id}))?;
            }
            u()
        }
        "reorderLayer" => {
            // beide Nummern flach wie `dom.layers`; verschoben werden Ebenen der obersten Stufe,
            // das Ziel zählt als Platz seiner Ebene der obersten Stufe
            let from = flat_entry(doc, a(0).i64())?;
            let list = flat_entries(doc)?;
            let to = a(1).i64().max(0);
            // Unterebene: an die Stelle des Ziels in dessen Elternebene (Ziel = Ebene der obersten
            // Stufe: oben in sie hinein); nicht in sich selbst (BA-075)
            if let Some(sub) = from.sub {
                let target = list.get(to as usize).or(list.last()).ok_or_else(|| FwErr::bad("Ebene"))?;
                // flach nach vorn (in der Ebene nach unten): unter das Ziel, sonst darüber
                move_sublayer(doc, sub, target, a(0).i64() > to)?;
                // nun Ebene der obersten Stufe: sie wird aktiv (`currentLayerNum` = ihr Index, BA-123)
                if target.sub.is_none()
                    && let Some(id) = from.id
                {
                    exec(Some(doc), "layer.setActive", json!({"layer": id}))?;
                }
                return u();
            }
            let id = from.id.ok_or_else(|| FwErr::bad("Ebene"))?;
            let slot = match list.get(to as usize) {
                Some(e) => e.slot,
                // hinter dem Ende: wie bisher ans Ende (layer.move begrenzt)
                None => list.last().map(|e| e.slot + 1).unwrap_or(0) + (to as usize).saturating_sub(list.len()),
            };
            ex(doc, "layer.move", json!({"layer": id, "index": slot}))
        }
        "addNewSubLayer" => {
            // unter der Ebene `index` (auch einer Unterebene: verschachtelt); die neue Unterebene
            // wird aktiv, Rückgabe ihr Name (B-2 BA-030)
            let idx = a(0).i64();
            if idx >= 0
                && let Some(id) = flat_entry(doc, idx)?.id
            {
                exec(Some(doc), "layer.setActive", json!({"layer": id}))?;
            }
            let mut p = json!({});
            if let Some(n) = a(1).str().filter(|n| !n.is_empty()) {
                p["name"] = json!(n);
            }
            exec(Some(doc), "pi.layer.addSub", p)?;
            Ok(conv::s(&super::doc::active_layer_name(doc)?))
        }
        "getParentLayerNum" => {
            // flacher Index der Elternebene, -1 auf der obersten Stufe (re\api\fw_api.json)
            let e = flat_entry(doc, a(0).i64())?;
            Ok(conv::n(e.parent.map(|p| p as f64).unwrap_or(-1.0)))
        }
        "setLayerDisclosure" => {
            let e = flat_entry(doc, if a(0).i64() < 0 { current_layer_index(doc)?.max(0) } else { a(0).i64() })?;
            if e.sub.is_some()
                && let Some(id) = e.id
            {
                exec(Some(doc), "layer.setExpanded", json!({"layer": id, "expanded": a(1).bool()}))?;
            } else {
                super::set_layers_expanded(doc, Some(e.slot), a(1).bool())?;
            }
            u()
        }
        "shareLayerToPages" | "shareWebLayerToPages" => {
            if name == "shareLayerToPages" {
                activate_layer(doc, a(0).i64())?;
            } else if let Some(w) = st(|s| Ok(s.session.document(doc).and_then(|d| fp_engine::docops::web_layer(&d.doc, d.page))))? {
                exec(Some(doc), "layer.setActive", json!({"layer": w.0}))?;
            }
            let pages = a(if name == "shareLayerToPages" { 1 } else { 0 }).json();
            ex(doc, "panel.layers.shareToPages", json!({"pages": pages}))
        }
        "shareLayerToAllPages" => {
            activate_layer(doc, a(0).i64())?;
            ex(doc, "panel.layers.shareToAllPages", json!({}))
        }
        "shareWebLayerToAllPages" => {
            if let Some(w) = st(|s| Ok(s.session.document(doc).and_then(|d| fp_engine::docops::web_layer(&d.doc, d.page))))? {
                exec(Some(doc), "layer.setActive", json!({"layer": w.0}))?;
            }
            ex(doc, "panel.layers.shareToAllPages", json!({}))
        }
        "detachSharedLayer" | "deleteSharedLayer" | "excludeSharedLayer" => {
            activate_layer(doc, a(0).i64())?;
            let c = match name {
                "excludeSharedLayer" => "panel.layers.excludeShared",
                "deleteSharedLayer" => "panel.layers.delete",
                _ => "panel.layers.detachShared",
            };
            ex(doc, c, json!({}))
        }
        "addMasterPageLayer" => ex(doc, "panel.layers.addMasterPageLayer", json!({})),
        "removeMasterPageLayer" => {
            let r = page_root(doc, |p, _, _| p.root.layers.iter().position(|l| matches!(l, LayerSlot::MasterPage(_))))?;
            if let Some(slot) = r {
                super::edit_doc(doc, |d| {
                    let page = d.page;
                    if let Some(p) = d.doc_mut().page_mut(page) {
                        p.root.layers.remove(slot);
                    }
                })?;
            }
            u()
        }
        "isMasterPageLayer" => {
            let idx = a(0).i64();
            let r = idx >= 0 && flat_entries(doc)?.get(idx as usize).is_some_and(|e| e.master);
            Ok(JsValue::from(r))
        }
        // ---------------------------------------------------------- Seiten
        "setMasterPage" => ex(doc, "pi.page.setMaster", json!({"index": a(0).i64().max(0), "master": true})),
        "unsetMasterPage" => {
            let i = st(|s| Ok(s.session.document(doc).and_then(|d| d.doc.pages.iter().position(|p| p.is_master == Some(true)))))?;
            match i {
                Some(i) => ex(doc, "pi.page.setMaster", json!({"index": i, "master": false})),
                None => u(),
            }
        }
        "hasMasterPage" => Ok(JsValue::from(st(|s| Ok(s.session.document(doc).is_some_and(|d| d.doc.pages.iter().any(|p| p.is_master == Some(true)))))?)),
        "setLinkPage" => {
            let (i, on) = (a(0).i64().max(0) as usize, a(1).bool());
            super::edit_doc(doc, |d| {
                if let Some(p) = d.doc_mut().pages.get_mut(i) {
                    Arc::make_mut(p).linked_to_master = Some(on);
                }
            })?;
            u()
        }
        "reorderPages" => {
            let (from, to) = (a(0).i64().max(0) as usize, a(1).i64().max(0) as usize);
            super::edit_doc(doc, |d| {
                let cur = d.page;
                let m = d.doc_mut();
                if from < m.pages.len() && to < m.pages.len() && from != to {
                    let p = m.pages.remove(from);
                    m.pages.insert(to, p);
                    let new_cur = if cur == from { to } else if from < cur && to >= cur { cur - 1 } else if from > cur && to <= cur { cur + 1 } else { cur };
                    m.current_page = new_cur;
                    m.invalidate_root_copy();
                    d.page = new_cur;
                }
            })?;
            u()
        }
        "getPageSetter" => crate::runtime::wrap(ctx, "SetCurrentPage", crate::state::Handle::Document(doc)),
        // ---------------------------------------------------------- Status
        "reorderFrame" => {
            let (from, to, copy) = (a(0).i64().max(0) as usize, a(1).i64().max(0) as usize, a(2).bool());
            super::edit_doc(doc, |d| {
                let page = d.page;
                let m = d.doc_mut();
                let n = m.pages.get(page).map(|p| p.root.frame_count.max(1) as usize).unwrap_or(1);
                if from >= n || to > n {
                    return;
                }
                if let Some(p) = m.page_mut(page) {
                    if copy {
                        p.root.frame_count += 1;
                    }
                    for slot in &mut p.root.layers {
                        if let LayerSlot::Own(l) = slot {
                            let l = Arc::make_mut(l);
                            if from < l.cels.len() {
                                let c = if copy { l.cels[from].clone() } else { l.cels.remove(from) };
                                let at = to.min(l.cels.len());
                                l.cels.insert(at, c);
                            }
                        }
                    }
                }
            })?;
            u()
        }
        "moveSelectionToFrame" => {
            let f = a(0).i64().max(0) as usize;
            selection_to_frames(doc, vec![f], a(1).bool())?;
            u()
        }
        "duplicateSelectionToFrames" | "duplicateSelectionToFrameRange" => {
            let (n, cur) = page_root(doc, |p, _, f| (p.root.frame_count.max(1) as usize, f))?;
            let frames: Vec<usize> = if name == "duplicateSelectionToFrameRange" {
                let (s, e) = (a(0).i64().max(0) as usize, a(1).i64().max(0) as usize);
                (s.min(e)..=s.max(e)).collect()
            } else {
                match arg_str(args, 0).to_lowercase().as_str() {
                    "all frames" | "all" => (0..n).collect(),
                    "next frame" | "next" => vec![cur + 1],
                    "previous frame" | "prev" => cur.checked_sub(1).into_iter().collect(),
                    "end" => vec![n.saturating_sub(1)],
                    _ => (0..n).collect(),
                }
            };
            selection_to_frames(doc, frames, true)?;
            u()
        }
        "distributeSelectionToFrames" | "distributeLayerToFrames" => {
            if name == "distributeLayerToFrames" {
                super::doc::method(doc, "selectAllOnLayer", &[a(0), Arg::Bool(false), Arg::Bool(false)], ctx)?;
            }
            ex(doc, "panel.states.distribute", json!({}))
        }
        "setOnionSkinning" => ex(doc, "pi.state.onionSkin", json!({"before": a(0).i64().max(0), "after": a(1).i64().max(0)})),
        // ---------------------------------------------------------- Symbole
        "addNewSymbol" => {
            let kind = arg_str(args, 0).to_lowercase();
            let c = if kind == "button" { "edit.insert.newButton" } else { "edit.insert.newSymbol" };
            ex(doc, c, json!({"name": arg_str(args, 1)}))
        }
        "editSymbol" => ex(doc, "modify.symbol.editSymbol", json!({})),
        "exitSymbolEdit" => {
            let in_edit = with_state(|s| s.session.registry().get("modify.symbol.exitEdit").is_some())?;
            if in_edit && with_state(|s| s.session.is_available("modify.symbol.exitEdit"))? { ex(doc, "modify.symbol.exitEdit", json!({})) } else { u() }
        }
        "detachInstanceFromSymbol" => on_selection(doc, "modify.symbol.breakApart", json!({})),
        "convertToAnimSymbol" => {
            // (uiName, numFrames, offsetDistPt{x,y}, rotationAmount, scaleAmount, startOpacity, endOpacity)
            // wie im Original (BA-020): per Skript entstehen keine neuen Status; die Instanzgrenzen
            // schließen den Bewegungsweg ein (fp-engine `element_bounds`)
            let off = a(2).point().unwrap_or((10.0, 10.0));
            let mut p = json!({"frames": a(1).i64().clamp(1, 1000), "offsetX": off.0, "offsetY": off.1, "rotation": a(3).i64(), "scale": if a(4).is_null() { 100.0 } else { a(4).f64() }, "startOpacity": if a(5).is_null() { 100.0 } else { a(5).f64() }, "endOpacity": if a(6).is_null() { 100.0 } else { a(6).f64() }, "addStates": false});
            let ui_name = arg_str(args, 0);
            if !ui_name.trim().is_empty() {
                p["name"] = json!(ui_name);
            }
            on_selection(doc, "modify.animation.animateSelection", p)
        }
        "convertAnimSymbolToGraphicSymbol" => on_selection(doc, "modify.animation.removeAnimation", json!({})),
        "setAnimInstanceNumFrames" => on_selection(doc, "modify.animation.settings", json!({"frames": a(0).i64().clamp(1, 1000)})),
        "setAnimInstanceOffsetDist" => {
            let (x, y) = a(0).point().unwrap_or((0.0, 0.0));
            on_selection(doc, "modify.animation.settings", json!({"offsetX": x, "offsetY": y}))
        }
        "setAnimInstanceRotationAmount" => on_selection(doc, "modify.animation.settings", json!({"rotation": a(0).i64()})),
        "setAnimInstanceScaleAmount" => on_selection(doc, "modify.animation.settings", json!({"scale": a(0).f64()})),
        "setAnimInstanceStartEndOpacity" => on_selection(doc, "modify.animation.settings", json!({"startOpacity": a(0).f64(), "endOpacity": a(1).f64()})),
        "setAnimInstanceLoopCount" | "setAnimInstanceStartFrame" => u(),
        "tween" => on_selection(doc, "modify.symbol.tweenInstances", json!({"steps": a(0).i64().max(1)})),
        "deleteSymbol" | "duplicateSymbol" | "duplicateSymbolForAlias" | "updateSymbol" | "enableNineScale" | "removeNineScale" | "resetNineScale" => {
            let sym = arg_str(args, 0);
            let idx = st(|s| Ok(s.session.document(doc).and_then(|d| d.doc.symbols.iter().position(|x| x.name == sym || x.id.0.to_string() == sym))))?;
            let Some(idx) = idx else { return Err(FwErr::Hr(hr::E_BAD_PARAM, format!("Symbol „{sym}“ nicht gefunden"))) };
            match name {
                "deleteSymbol" => ex(doc, "pi.symbol.delete", json!({"index": idx})),
                "duplicateSymbol" | "duplicateSymbolForAlias" => ex(doc, "panel.library.duplicate", json!({"index": idx})),
                "updateSymbol" => ex(doc, "panel.library.update", json!({"index": idx})),
                "enableNineScale" => ex(doc, "panel.library.nineSlice", json!({"index": idx, "on": if a(1).is_null() { true } else { a(1).bool() }})),
                _ => ex(doc, "panel.library.nineSlice", json!({"index": idx, "on": false})),
            }
        }
        "insertSymbolAt" => {
            let sym = arg_str(args, 0);
            let (x, y) = a(1).point().unwrap_or((0.0, 0.0));
            let idx = st(|s| Ok(s.session.document(doc).and_then(|d| d.doc.symbols.iter().position(|x| x.name == sym || x.id.0.to_string() == sym))))?;
            let Some(idx) = idx else { return Err(FwErr::Hr(hr::E_BAD_PARAM, format!("Symbol „{sym}“ nicht gefunden"))) };
            ex(doc, "pi.symbol.place", json!({"index": idx, "at": [x, y]}))
        }
        "importSymbol" | "importSymbolButNotAsAlias" => {
            let path = super::fw::resolve_read(&crate::urls::url_to_path(&arg_str(args, 0)));
            ex(doc, "pi.symbol.import", json!({"path": path.display().to_string(), "place": true}))
        }
        "setSymbolProperties" => {
            // (oldName, type, newName, enable9Slice)
            let sym = arg_str(args, 0);
            let idx = st(|s| Ok(s.session.document(doc).and_then(|d| d.doc.symbols.iter().position(|x| x.name == sym))))?;
            let Some(idx) = idx else { return Err(FwErr::Hr(hr::E_BAD_PARAM, format!("Symbol „{sym}“ nicht gefunden"))) };
            let kind = arg_str(args, 1).to_lowercase();
            let mut p = json!({"index": idx, "name": arg_str(args, 2)});
            if ["graphic", "button", "animation"].contains(&kind.as_str()) {
                p["kind"] = json!(kind);
            }
            if !a(3).is_null() {
                p["nineSlice"] = json!(a(3).bool());
            }
            ex(doc, "panel.library.properties", p)
        }
        // ---------------------------------------------------------- Text
        "setTextCharSpacing" => text_attr(doc, "tracking", json!(a(0).f64())),
        "setTextLeading" => text_attr(doc, "leading", json!(a(0).f64())),
        "setTextLeadingMode" => text_attr(doc, "leadingMode", json!(if arg_str(args, 0).contains("exact") { 1 } else { 0 })),
        "setTextHorizontalScale" => text_attr(doc, "horizontalScale", json!(a(0).f64() * if a(0).f64() <= 10.0 { 100.0 } else { 1.0 })),
        "setTextParaIndent" => text_attr(doc, "indent", json!(a(0).f64())),
        "setTextParaSpacingBefore" => text_attr(doc, "spaceBefore", json!(a(0).f64())),
        "setTextParaSpacingAfter" => text_attr(doc, "spaceAfter", json!(a(0).f64())),
        "adjustFontSize" => {
            let d = a(0).f64();
            let ids = selection(doc)?;
            for id in ids {
                let e = element(doc, id)?;
                if let ElementKind::Text(t) = &e.kind {
                    let cur = t.initial.size.unwrap_or(12.0) as f64;
                    exec(Some(doc), "text.setAttributes", json!({"ids": [id.0], "size": (cur + d).max(1.0)}))?;
                }
            }
            u()
        }
        "applyFontMarkup" => {
            let attr = arg_str(args, 0);
            let v = a(1).json();
            let p = match attr.as_str() {
                "face" | "font" => json!({"font": v}),
                "size" => json!({"size": conv::leading_number(&v.as_str().map(String::from).unwrap_or_else(|| v.to_string()))}),
                "color" | "fillColor" => json!({"color": v.as_str().and_then(conv::norm_color)}),
                _ => return u(),
            };
            on_selection(doc, "text.setAttributes", p)
        }
        "removeFontMarkup" => u(),
        "getFontMarkup" => {
            let attr = arg_str(args, 0);
            let id = selection(doc)?.into_iter().next();
            let Some(id) = id else { return Ok(JsValue::null()) };
            let e = element(doc, id)?;
            let ElementKind::Text(t) = &e.kind else { return Ok(JsValue::null()) };
            Ok(match attr.as_str() {
                "face" | "font" => t.initial.font.as_deref().map(conv::s).unwrap_or_else(JsValue::null),
                "size" => t.initial.size.map(|s| conv::s(&format!("{}pt", conv::fmt_num(s as f64)))).unwrap_or_else(JsValue::null),
                "color" | "fillColor" => t.initial.color.map(|c| conv::s(&hex(fp_color::Argb(c)))).unwrap_or_else(JsValue::null),
                _ => JsValue::null(),
            })
        }
        "applyCharacterMarkup" | "removeCharacterMarkup" => {
            let tag = arg_str(args, 0).to_lowercase();
            let on = name == "applyCharacterMarkup";
            let key = match tag.as_str() {
                "b" | "bold" => "bold",
                "i" | "italic" => "italic",
                "u" | "underline" => "underline",
                _ => return u(),
            };
            text_attr(doc, key, json!(on))
        }
        "hasCharacterMarkup" => {
            let tag = arg_str(args, 0).to_lowercase();
            let id = selection(doc)?.into_iter().next();
            let Some(id) = id else { return Ok(JsValue::from(false)) };
            let e = element(doc, id)?;
            let ElementKind::Text(t) = &e.kind else { return Ok(JsValue::from(false)) };
            let v = match tag.as_str() {
                "b" | "bold" => t.initial.bold,
                "i" | "italic" => t.initial.italic,
                "u" | "underline" => t.initial.underline,
                _ => None,
            };
            Ok(JsValue::from(v.unwrap_or(false)))
        }
        "getTextAlignment" => {
            let id = selection(doc)?.into_iter().next();
            let Some(id) = id else { return Ok(JsValue::null()) };
            let e = element(doc, id)?;
            let ElementKind::Text(t) = &e.kind else { return Ok(JsValue::null()) };
            Ok(conv::s(match t.initial.alignment.unwrap_or(0) {
                1 => "center",
                2 => "right",
                3 => "justify",
                4 => "stretch",
                _ => "left",
            }))
        }
        "enableTextAntiAliasing" => super::doc::method(doc, "setTextAntiAliasing", &[Arg::Str(if a(0).bool() { "smooth".into() } else { "none".into() })], ctx),
        "setTextRectangle" | "setTextRectangleAuto" | "setTextRectangleAutoFromPoint" => {
            let ids = selection(doc)?;
            if ids.is_empty() {
                return u();
            }
            match name {
                "setTextRectangle" => {
                    let r = a(0).rect().ok_or_else(|| FwErr::Hr(hr::E_BAD_PARAM_TYPE, "Rechteck".into()))?;
                    exec(Some(doc), "text.setProperties", json!({"frameType": "fixed", "width": (r[2] - r[0]).max(1.0)}))?;
                    exec(Some(doc), "element.setBounds", json!({"x": r[0], "y": r[1]}))?;
                }
                _ => {
                    exec(Some(doc), "text.setProperties", json!({"frameType": "auto"}))?;
                    if name == "setTextRectangleAutoFromPoint"
                        && let Some((x, y)) = a(0).point()
                    {
                        exec(Some(doc), "element.setBounds", json!({"x": x, "y": y}))?;
                    }
                }
            }
            u()
        }
        "setTextFlow" | "setTextStream" => {
            let o = super::orientation_name(super::orientation_code_from_jsf(&arg_str(args, 0)));
            on_selection(doc, "text.setProperties", json!({"orientation": o}))
        }
        "insertText" | "replaceTextString" => {
            let ids = selection(doc)?;
            for id in ids {
                let e = element(doc, id)?;
                if let ElementKind::Text(t) = &e.kind {
                    let cur = t.plain_text();
                    let next = if name == "insertText" {
                        format!("{cur}{}", arg_str(args, 0))
                    } else {
                        cur.replace(&arg_str(args, 0), &arg_str(args, 1))
                    };
                    exec(Some(doc), "text.setText", json!({"id": id.0, "text": next}))?;
                }
            }
            u()
        }
        "convertToPaths" => on_selection(doc, "text.convertToPaths", json!({})),
        // ---------------------------------------------------------- Pfade
        "addNewSinglePointPath" => {
            let (x, y) = pt_arg(&a(0))?;
            ex(doc, "draw.path", json!({"contours": [{"closed": false, "nodes": [{"anchor": [x, y], "in": [x, y], "out": [x, y]}]}]}))
        }
        "appendPointToPath" => {
            // (contourIndex, pred, pt, succ)
            let ci = a(0).i64().max(0) as usize;
            let (p, q, r) = (a(1).point(), pt_arg(&a(2))?, a(3).point());
            edit_first_path(doc, |cs| {
                let c = cs.get_mut(ci).ok_or_else(|| FwErr::Hr(hr::E_ARRAY_INDEX, "Kontur".into()))?;
                let mut n = node(q.0, q.1);
                if let Some(p) = p {
                    n.pred = fp_geom::Point::new(p.0 as f32, p.1 as f32);
                }
                if let Some(r) = r {
                    n.succ = fp_geom::Point::new(r.0 as f32, r.1 as f32);
                }
                c.nodes.push(n);
                Ok(())
            })?;
            u()
        }
        "insertPointInPath" => {
            // (contourIndex, segmentIndex, t)
            let ids = selection(doc)?;
            let Some(id) = ids.first() else { return u() };
            ex(doc, "path.insertNode", json!({"id": id.0, "contour": a(0).i64().max(0), "segment": a(1).i64().max(0), "t": a(2).f64().clamp(0.0, 1.0)}))
        }
        "deletePointOnPath" => {
            let ids = selection(doc)?;
            let Some(id) = ids.first() else { return u() };
            ex(doc, "path.removeNode", json!({"id": id.0, "contour": a(0).i64().max(0), "node": a(1).i64().max(0)}))
        }
        "modifyPointOnPath" => {
            // (contourIndex, nodeIndex, pred, pt, succ)
            let (ci, ni_) = (a(0).i64().max(0) as usize, a(1).i64().max(0) as usize);
            let (p, q, r) = (a(2).point(), a(3).point(), a(4).point());
            edit_first_path(doc, |cs| {
                let n = cs.get_mut(ci).and_then(|c| c.nodes.get_mut(ni_)).ok_or_else(|| FwErr::Hr(hr::E_ARRAY_INDEX, "Knoten".into()))?;
                if let Some(q) = q {
                    n.anchor = fp_geom::Point::new(q.0 as f32, q.1 as f32);
                }
                if let Some(p) = p {
                    n.pred = fp_geom::Point::new(p.0 as f32, p.1 as f32);
                }
                if let Some(r) = r {
                    n.succ = fp_geom::Point::new(r.0 as f32, r.1 as f32);
                }
                Ok(())
            })?;
            u()
        }
        "moveSelectedBezierPointsBy" => {
            let (dx, dy) = pt_arg(&a(0))?;
            ex(doc, "path.movePoints", json!({"dx": dx, "dy": dy}))
        }
        "moveBezierHandleBy" => {
            // (contour, node, isPred, delta)
            let (ci, ni_, pred) = (a(0).i64().max(0) as usize, a(1).i64().max(0) as usize, a(2).bool());
            let (dx, dy) = a(3).point().unwrap_or((0.0, 0.0));
            edit_first_path(doc, |cs| {
                let n = cs.get_mut(ci).and_then(|c| c.nodes.get_mut(ni_)).ok_or_else(|| FwErr::Hr(hr::E_ARRAY_INDEX, "Knoten".into()))?;
                let h = if pred { &mut n.pred } else { &mut n.succ };
                h.x += dx as f32;
                h.y += dy as f32;
                Ok(())
            })?;
            u()
        }
        "knifeElementsFromPoint" | "knifeElementsFromPoints" => {
            let pts: Vec<Value> = if name == "knifeElementsFromPoint" {
                let (x0, y0) = pt_arg(&a(0))?;
                let (x1, y1) = pt_arg(&a(1))?;
                vec![json!([x0, y0]), json!([x1, y1])]
            } else {
                a(0).json().as_array().map(|v| v.iter().filter_map(|p| conv::point_of(p).map(|(x, y)| json!([x, y]))).collect()).unwrap_or_default()
            };
            let ids = ids_json(&selection(doc)?);
            ex(doc, "path.knife", json!({"ids": ids, "points": pts}))
        }
        "createCompoundShape" | "combineCompoundShape" => {
            let op = arg_str(args, 0).to_lowercase();
            if name == "createCompoundShape" && !op.is_empty() {
                let code = match op.as_str() {
                    "union" => 1,
                    "subtract" | "punch" => 2,
                    "intersect" => 3,
                    "crop" => 4,
                    _ => 1,
                };
                let _ = code;
            }
            on_selection(doc, "pi.compound.combine", json!({}))
        }
        "setPathOperation" => {
            let op = match arg_str(args, 0).to_lowercase().as_str() {
                "union" => 1,
                "intersect" => 2,
                "subtract" | "punch" => 3,
                "crop" => 4,
                _ => 0,
            };
            ex(doc, "pi.compound.setOp", json!({"op": op}))
        }
        "setRectRoundness" => on_selection(doc, "pi.roundness", json!({"value": a(0).f64().max(0.0)})),
        "setRectRoundnessMode" => {
            let px = arg_str(args, 0).to_lowercase().contains("pixel");
            let ids = selection(doc)?;
            for id in ids {
                let e = element(doc, id)?;
                if let ElementKind::Path(p) = &e.kind
                    && let Some(pr) = &p.primitive
                {
                    exec(Some(doc), "pi.roundness", json!({"ids": [id.0], "value": pr.roundness, "pixels": px}))?;
                }
            }
            u()
        }
        "setRectSides" | "ungroupPrimitives" => u(),
        "setQuadrangle" => {
            let q = a(0).json();
            let pts: Vec<Value> = ["topLeft", "topRight", "bottomRight", "bottomLeft"]
                .iter()
                .filter_map(|k| q.get(*k).and_then(conv::point_of).map(|(x, y)| json!([x, y])))
                .collect();
            if pts.len() == 4 { on_selection(doc, "transform.distort", json!({"quad": pts})) } else { Err(FwErr::Hr(hr::E_BAD_PARAM, "Viereck {topLeft, topRight, bottomRight, bottomLeft}".into())) }
        }
        // ---------------------------------------------------------- Füllung, Strich, Stile
        "applyCurrentFill" => {
            let c = with_state(|s| hex(s.session.tools.fill_color))?;
            on_selection(doc, "element.setFill", json!({"kind": "solid", "color": c}))
        }
        "removeFill" => on_selection(doc, "element.setFill", json!({"kind": "none"})),
        "removeBrush" => on_selection(doc, "element.setStroke", json!({"width": 0})),
        "setDefaultBrushAndFillColors" => ex(doc, "color.defaults", json!({})),
        "swapBrushAndFillColors" => ex(doc, "color.swap", json!({})),
        "setBrushName" | "setGradientName" | "setEffectName" => u(),
        "setFillVectorStart" => {
            let (x, y) = pt_arg(&a(0))?;
            on_selection(doc, "element.setFillHandles", json!({"start": [x, y]}))
        }
        "moveFillVectorHandleBy" | "rotateFillVectorHandle" => {
            // moveFillVectorHandleBy(delta, whichHandle, …) / rotateFillVectorHandle(angle, …)
            let ids = selection(doc)?;
            let Some(id) = ids.first() else { return u() };
            let e = element(doc, *id)?;
            let h = match &e.kind {
                ElementKind::Path(p) => p.fill_handles,
                ElementKind::Text(t) => t.fill_handles,
                _ => return u(),
            };
            let (mut s, mut en, mut sc) = (h.start, h.end, h.second);
            if name == "moveFillVectorHandleBy" {
                let (dx, dy) = a(0).point().unwrap_or((0.0, 0.0));
                let which = arg_str(args, 1).to_lowercase();
                let mv = |p: &mut fp_geom::Point| {
                    p.x += dx as f32;
                    p.y += dy as f32;
                };
                match which.as_str() {
                    "end" | "handle2" => mv(&mut en),
                    "skew" | "second" | "handle3" => mv(&mut sc),
                    "start" | "handle1" => mv(&mut s),
                    _ => {
                        mv(&mut s);
                        mv(&mut en);
                        mv(&mut sc);
                    }
                }
            } else {
                let ang = a(0).f64().to_radians() as f32;
                let (c, sn) = (ang.cos(), ang.sin());
                let rot = |p: fp_geom::Point| fp_geom::Point::new(s.x + (p.x - s.x) * c - (p.y - s.y) * sn, s.y + (p.x - s.x) * sn + (p.y - s.y) * c);
                en = rot(en);
                sc = rot(sc);
            }
            on_selection(doc, "element.setFillHandles", json!({"start": [s.x, s.y], "end": [en.x, en.y], "second": [sc.x, sc.y]}))
        }
        "applyStyle" => {
            let n = match a(0) {
                Arg::Num(i) => st(|s| Ok(s.session.document(doc).and_then(|d| d.doc.styles.as_ref().and_then(|v| v.get(i as usize).map(|x| x.name.clone())))))?.unwrap_or_default(),
                other => other.str().unwrap_or_default(),
            };
            on_selection(doc, "pi.style.apply", json!({"name": n}))
        }
        "redefineStyleByName" | "redefineStyleByIndex" | "redefineStyleFromSelection" => {
            let n = match a(0) {
                Arg::Num(i) => st(|s| Ok(s.session.document(doc).and_then(|d| d.doc.styles.as_ref().and_then(|v| v.get(i as usize).map(|x| x.name.clone())))))?.unwrap_or_default(),
                other => other.str().unwrap_or_default(),
            };
            on_selection(doc, "panel.styles.redefine", json!({"name": n}))
        }
        "breakLink" => on_selection(doc, "panel.styles.breakLink", json!({})),
        "motionBlurSelection" => on_selection(doc, "filter.blur.motion", json!({"angle": a(0).i64(), "distance": a(1).i64().max(1)})),
        "convolveSelection" => on_selection(doc, "filter.sharpen.sharpen", json!({})),
        "fillSelectedPixels" => {
            let c = a(0).json().get("color").and_then(|v| v.as_str()).and_then(conv::norm_color).or_else(|| with_state(|s| hex(s.session.tools.fill_color)).ok());
            let has = with_state(|s| s.session.document(doc).is_some_and(|d| d.selection.has_pixels()))?;
            if has { ex(doc, "bitmap.floodFill", json!({"fillSelection": true, "color": c, "point": [0, 0]})) } else { u() }
        }
        "cropSelection" => {
            let r = a(0).rect().or(bounds(doc, &selection(doc)?)?);
            match r {
                Some(r) => ex(doc, "edit.cropDocument", json!({"rect": rect_arr(r)})),
                None => u(),
            }
        }
        "setDocumentCanvasSizeToSelection" => {
            let r = bounds(doc, &selection(doc)?)?;
            match r {
                Some(r) => ex(doc, "edit.cropDocument", json!({"rect": rect_arr(r)})),
                None => u(),
            }
        }
        "setDocumentCanvasSizeToDocumentExtents" => ex(doc, if a(0).bool() { "modify.canvas.fitCanvas" } else { "modify.canvas.trimCanvas" }, json!({})),
        "clipCopyAsPaths" => on_selection(doc, "edit.copyAsVectors", json!({})),
        "clipCopyFormats" => on_selection(doc, "edit.copy", json!({})),
        "clipCopyJsToExecute" => {
            let code = st(|s| Ok(s.session.document(doc).map(|d| fp_engine::history::to_script(&d.history.export_all())).unwrap_or_default()))?;
            with_state(|s| s.log.push(code))?;
            u()
        }
        "clipPasteAttributes" => on_selection(doc, "edit.pasteAttributes", json!({})),
        "clipPasteFromChannelToChannel" => ex(doc, "edit.paste", json!({})),
        "addNewImageViaCopy" => ex(doc, "edit.insert.bitmapViaCopy", json!({})),
        "addNewImageViaCut" => ex(doc, "edit.insert.bitmapViaCut", json!({})),
        "enterPaintMode" => on_selection(doc, "modify.editBitmap", json!({})),
        "exitPaintMode" => {
            let avail = with_state(|s| s.session.is_available("modify.exitBitmapMode"))?;
            if avail { ex(doc, "modify.exitBitmapMode", json!({})) } else { u() }
        }
        "getPixelMask" => Ok(JsValue::null()),
        "setPixelMask" => u(),
        // ---------------------------------------------------------- Masken
        "addElementMask" => {
            let mode = arg_str(args, 0).to_lowercase();
            let has_pixels = with_state(|s| s.session.document(doc).is_some_and(|d| d.selection.has_pixels()))?;
            let c = match (mode.contains("hide"), has_pixels) {
                (true, true) => "modify.mask.hideSelection",
                (true, false) => "modify.mask.hideAll",
                (false, true) => "modify.mask.revealSelection",
                (false, false) => "modify.mask.revealAll",
            };
            on_selection(doc, c, json!({}))
        }
        "removeElementMask" => {
            let has = selection(doc)?.into_iter().any(|id| element(doc, id).is_ok_and(|e| matches!(&e.kind, ElementKind::Group(g) if g.kind.is_mask())));
            if has { ex(doc, "modify.mask.deleteMask", json!({})) } else { u() }
        }
        "enableElementMask" => {
            let on = a(0).bool();
            edit_mask_groups(doc, |g| g.mask_flags.enabled = on)?;
            u()
        }
        "linkElementMask" => {
            let on = a(0).bool();
            edit_mask_groups(doc, |g| g.mask_flags.linked = on)?;
            u()
        }
        "setElementMaskShowAttrs" => {
            let on = a(0).bool();
            edit_mask_groups(doc, |g| g.mask_flags.hide_attrs = !on)?;
            u()
        }
        "setElementMaskMode" => {
            let alpha = arg_str(args, 0).to_lowercase().contains("alpha");
            edit_mask_groups(doc, |g| {
                if g.kind.is_mask() {
                    g.kind = if alpha { fp_model::element::GroupKind::BitmapMask } else { fp_model::element::GroupKind::VectorMask };
                }
            })?;
            u()
        }
        "moveElementMaskBy" | "moveMaskGroupContentsBy" => {
            let (dx, dy) = pt_arg(&a(0))?;
            let mask = name == "moveElementMaskBy";
            edit_mask_groups(doc, |g| {
                if mask {
                    if let Some(m) = &mut g.mask {
                        fp_engine::docops::translate_element(Arc::make_mut(m), dx as f32, dy as f32);
                    }
                } else {
                    for c in &mut g.children {
                        fp_engine::docops::translate_element(Arc::make_mut(c), dx as f32, dy as f32);
                    }
                }
            })?;
            u()
        }
        "setSelectionMask" => {
            if a(0).is_null() {
                return ex(doc, "select.deleteBitmapSelection", json!({}));
            }
            // Maske als Rechteck/Auswahl der Grenzen übernehmen
            let r = a(0).json().get("bounds").and_then(conv::rect_of);
            match r {
                Some(r) => ex(doc, "select.pixels", json!({"shape": "rect", "rect": rect_arr(r), "mode": "replace"})),
                None => u(),
            }
        }
        // ---------------------------------------------------------- Web
        "setHotspotAltTag" => web_params(doc, json!({"alt": arg_str(args, 0)})),
        "setHotspotTarget" => web_params(doc, json!({"target": arg_str(args, 0)})),
        "setHotspotText" => web_params(doc, json!({"url": arg_str(args, 0)})),
        "setHotspotColor" => web_params(doc, json!({"color": color_arg(&a(0)).unwrap_or_else(|| "#00ff00".into())})),
        "setHotspotShape" => {
            let s = match arg_str(args, 0).to_lowercase().as_str() {
                "circle" => 1,
                "polygon" => 2,
                _ => 0,
            };
            web_params(doc, json!({"shape": s}))
        }
        "setHotspotRectangle" => {
            let r = a(0).rect().ok_or_else(|| FwErr::Hr(hr::E_BAD_PARAM_TYPE, "Rechteck".into()))?;
            on_selection(doc, "element.setBounds", json!({"x": r[0], "y": r[1], "width": (r[2] - r[0]).max(1.0), "height": (r[3] - r[1]).max(1.0)}))
        }
        "setSliceFilename" => web_params(doc, json!({"fileName": arg_str(args, 0)})),
        "setSliceHtml" => web_params(doc, json!({"html": arg_str(args, 0)})),
        "setSliceIsHtml" | "setSliceType" => {
            let html = if name == "setSliceIsHtml" { a(0).bool() } else { arg_str(args, 0).to_lowercase().contains("html") };
            web_params(doc, json!({"sliceType": if html { 1 } else { 0 }}))
        }
        "setSliceAutonaming" | "setSliceExportOptions" => u(),
        "copyToHotspot" => {
            let kind = arg_str(args, 0).to_lowercase();
            let c = if kind.contains("polyslice") || kind.contains("polygon slice") {
                "edit.insert.polygonSlice"
            } else if kind.contains("slice") {
                "edit.insert.slice"
            } else {
                "edit.insert.hotspot"
            };
            on_selection(doc, c, json!({}))
        }
        "addBehavior" => {
            let b = a(0).json();
            let ev = b.get("event").and_then(|v| v.as_i64()).unwrap_or(0);
            let act = b.get("action").and_then(|v| v.as_i64()).unwrap_or(0);
            let mut p = json!({"event": ev, "action": act});
            if let Some(t) = b.get("statusText").and_then(|v| v.as_str()) {
                p["statusText"] = json!(t);
            }
            on_selection(doc, "pi.behavior.add", p)
        }
        "removeBehavior" => on_selection(doc, "pi.behavior.remove", json!({"index": a(0).i64().max(0)})),
        "addSwapImageBehaviorFromPoint" => {
            let (x, y) = pt_arg(&a(0))?;
            let ids = st(|s| {
                let d = s.session.document(doc).ok_or_else(|| FwErr::bad("Dokument"))?;
                let all = fp_engine::docops::selectable_ids(&d.doc, d.page, d.frame);
                Ok(all
                    .into_iter()
                    .filter(|id| {
                        fp_engine::docops::element_on_page(&d.doc, d.page, *id)
                            .is_some_and(|e| matches!(e.kind, ElementKind::WebObject(_)) && fp_engine::docops::element_bounds(&d.doc, &e).is_some_and(|b| (b.left as f64) <= x && (b.right as f64) >= x && (b.top as f64) <= y && (b.bottom as f64) >= y))
                    })
                    .collect::<Vec<_>>())
            })?;
            if ids.is_empty() {
                return u();
            }
            ex(doc, "pi.behavior.add", json!({"ids": ids_json(&ids), "event": 0, "action": 1, "frame": a(1).i64().max(0)}))
        }
        "setButtonShowDownOnLoad" => on_selection(doc, "pi.button", json!({"showDownOnLoad": a(0).bool()})),
        "setButtonAutoSlice" | "setButtonIncludeDownState" | "setButtonIncludeOverWhileDownState" | "setButtonOptions" => u(),
        "replaceButtonTextStrings" | "replaceButtonTextStringsInInstances" => {
            let t = arg_str(args, 0);
            on_selection(doc, "pi.button", json!({"text": t}))
        }
        "changeSliceGuide" | "changeNineScaleGuide" | "moveNineScaleGuide" | "scalingGridRect" => u(),
        // ---------------------------------------------------------- Dokument, Export
        "saveCopyAs" => {
            let p = crate::urls::url_to_path(&arg_str(args, 0));
            super::doc::save_copy(doc, &p)?;
            Ok(JsValue::from(true))
        }
        "setMatteColor" => {
            let (use_matte, c) = (a(0).bool(), color_arg(&a(1)));
            with_state(|s| {
                let e = s.export_options.entry(doc.0).or_insert_with(|| json!({}));
                if !e.is_object() {
                    *e = json!({});
                }
                e["useMatteColor"] = json!(use_matte);
                if let Some(c) = c {
                    e["matteColor"] = json!(c);
                }
            })?;
            u()
        }
        "setExportSettings" => {
            let v = a(0).json();
            with_state(|s| {
                s.export_options.insert(doc.0, v.clone());
            })?;
            let _ = exec(Some(doc), "pi.export.settings", json!({"settings": v}));
            u()
        }
        "adjustExportToSize" => ex(doc, "panel.optimize.optimizeToSize", json!({"kb": a(0).f64().max(1.0)})),
        "previewInBrowser" | "previewAllPagesInBrowser" => {
            let c = if name == "previewAllPagesInBrowser" { "file.previewInBrowser.previewAllPagesInPrimaryBrowser" } else { "file.previewInBrowser.previewInPrimaryBrowser" };
            ex(doc, c, json!({"open": crate::shell_open_allowed()}))
        }
        "sendEmail" => ex(doc, "file.sendToEMail.fwPng", json!({"open": crate::shell_open_allowed()})),
        "exportElements" => {
            // (elements, folderURL, prefix)
            let dir = crate::urls::url_to_path(&arg_str(args, 1));
            let prefix = arg_str(args, 2);
            let els = super::ids_from_arg(&a(0))?;
            for (k, (d, id)) in els.iter().enumerate() {
                let e = element(*d, *id)?;
                let n = e.common.name.clone().unwrap_or_else(|| format!("{prefix}{}", k + 1));
                let path = dir.join(format!("{n}.png"));
                export_element(*d, *id, &path)?;
            }
            Ok(JsValue::from(true))
        }
        "copyHtmlWizard" => ex(doc, "edit.copyHtmlCode", json!({"style": "htmlAndImages"})),
        "makeFind" => {
            let mut p = a(0).json();
            if !p.is_object() {
                p = json!({});
            }
            p["__doc"] = json!(doc.0);
            let i = with_state(|s| {
                s.finds.push((p, 0));
                s.finds.len() - 1
            })?;
            crate::runtime::wrap(ctx, "Find", Handle::Find(i))
        }
        "InsertPageForImport" => ex(doc, "page.add", json!({})),
        "setTransformMode" | "setGroupType" => {
            let v = arg_str(args, 0);
            let ids: Vec<u64> = selection(doc)?.iter().map(|i| i.0).collect();
            if !ids.is_empty() {
                super::exec_group(doc, vec![("element.setCustomData", json!({"ids": ids, "data": {"__transformMode": v}}))])?;
            }
            u()
        }
        "saveJPEGMask" => {
            let has = with_state(|s| s.session.document(doc).is_some_and(|d| d.selection.has_pixels()))?;
            if has { ex(doc, "modify.selectiveJpeg.saveSelectionAsJpegMask", json!({})) } else { u() }
        }
        "restoreJPEGMask" => {
            let has = page_root(doc, |p, _, _| p.jpeg_mask.is_some())?;
            if has { ex(doc, "modify.selectiveJpeg.restoreJpegMaskAsSelection", json!({})) } else { u() }
        }
        "clearJPEGMask" => {
            let has = page_root(doc, |p, _, _| p.jpeg_mask.is_some())?;
            if has { ex(doc, "modify.selectiveJpeg.removeJpegMask", json!({})) } else { u() }
        }
        "distribute" => {
            let how = arg_str(args, 0).to_lowercase();
            let c = if how.contains("vert") || how.contains("height") { "modify.align.distributeHeights" } else { "modify.align.distributeWidths" };
            on_selection(doc, c, json!({}))
        }
        "moveSelectionMaskBy" => {
            let (dx, dy) = pt_arg(&a(0))?;
            super::edit_doc(doc, |d| {
                if let Some(m) = &d.selection.pixels {
                    let b = m.bounds().translate(dx.round() as i32, dy.round() as i32);
                    if let Ok(mut n) = fp_raster::Mask::from_raw(b, m.data().to_vec()) {
                        n.set_soft_edge(m.soft_edge());
                        d.selection.pixels = Some(n);
                    }
                }
            })?;
            u()
        }
        "appendPointToSlice" | "appendPointToHotspot" | "movePointOnHotspotBy" | "movePointOnHotspotByWithFlags" => {
            let ids = selection(doc)?;
            let Some(id) = ids.into_iter().find(|i| element(doc, *i).is_ok_and(|e| matches!(e.kind, ElementKind::WebObject(_)))) else {
                return Err(FwErr::Hr(hr::E_BAD_SELECTION, "kein Hotspot/Slice ausgewählt".into()));
            };
            let append = name.starts_with("append");
            let (x, y) = if append { pt_arg(&a(0))? } else { a(1).point().unwrap_or((0.0, 0.0)) };
            let idx = a(0).i64().max(0) as usize;
            super::edit_doc(doc, |d| {
                let page = d.page;
                fp_engine::docops::edit_by_id(d.doc_mut(), page, id, |e| {
                    if let ElementKind::WebObject(w) = &mut e.kind {
                        if w.polygon.is_none() {
                            let b = w.bounds;
                            w.polygon = Some(fp_model::element::Contour {
                                closed: true,
                                bsl: None,
                                nodes: vec![node(b.left as f64, b.top as f64), node(b.right as f64, b.top as f64), node(b.right as f64, b.bottom as f64), node(b.left as f64, b.bottom as f64)],
                                extra: Default::default(),
                            });
                            w.shape = fp_model::WebShape::Polygon;
                        }
                        if let Some(poly) = &mut w.polygon {
                            if append {
                                poly.nodes.push(node(x, y));
                            } else if let Some(n) = poly.nodes.get_mut(idx) {
                                for p in [&mut n.anchor, &mut n.pred, &mut n.succ] {
                                    p.x += x as f32;
                                    p.y += y as f32;
                                }
                            }
                            w.bounds = fp_model::new::contour_bounds(std::slice::from_ref(poly));
                        }
                    }
                });
            })?;
            u()
        }
        // ---------------------------------------------------------- automatische Formen
        "insertSmartShapeAt" => {
            let (x, y) = pt_arg(&a(1))?;
            let v = autoshape(doc, json!({"op": "insert", "shape": arg_str(args, 0), "at": [x, y]}))?;
            let _ = v;
            u()
        }
        "dragControlPoint" => {
            let id = selected_shape(doc)?;
            let (x, y) = pt_arg(&a(1))?;
            autoshape(doc, json!({"op": "dragControlPoint", "id": id.0, "index": a(0).i64(), "to": [x, y], "shift": a(2).bool(), "ctrl": a(3).bool(), "alt": a(4).bool()}))?;
            u()
        }
        "redrawSmartShape" => {
            if let Ok(id) = selected_shape(doc) {
                autoshape(doc, json!({"op": "redraw", "id": id.0}))?;
            }
            u()
        }
        _ => {
            let _ = jserr;
            ni(CLS, name)
        }
    }
}

pub(super) fn get(doc: DocId, name: &str, ctx: &mut Context) -> FwResult<JsValue> {
    match name {
        "gridOrigin" | "gridSize" => {
            let g = page_root(doc, |p, _, _| p.root.grid.clone().unwrap_or_default())?;
            let p = if name == "gridOrigin" { g.origin } else { g.size };
            conv::point_js(p.x as f64, p.y as f64, ctx).map_err(jserr)
        }
        "gridColor" => Ok(conv::s(&hex(page_root(doc, |p, _, _| p.root.grid.clone().unwrap_or_default().color)?))),
        "guides" => {
            let g = page_root(doc, |p, _, _| p.root.guides.clone().unwrap_or_default())?;
            // TW-28: Die wirksame Sperre steht im Ansichtszustand (Ansicht ▸ Hilfslinien sperren).
            let locked = view(doc, |v| v.lock_guides)? || g.locked.unwrap_or(false);
            let v = json!({
                "locked": locked, "color": hex(g.color.unwrap_or(fp_color::Argb(0xff00_ffff))),
                "hGuides": g.horizontal, "vGuides": g.vertical,
            });
            conv::json_to_js(&v, ctx).map_err(jserr)
        }
        "savedSelections" => {
            let names: Vec<Value> = page_root(doc, |p, _, _| p.saved_selections.as_ref().map(|v| v.iter().enumerate().map(|(i, s)| json!(s.name.clone().unwrap_or_else(|| format!("Auswahl {}", i + 1)))).collect()).unwrap_or_default())?;
            conv::json_to_js(&Value::Array(names), ctx).map_err(jserr)
        }
        "textures" | "brushes" | "fills" | "gradients" | "patterns" => {
            let names: Vec<String> = page_root(doc, |p, _, _| match name {
                "brushes" => p.presets.brushes.as_ref().map(|v| v.iter().map(|b| b.name.clone()).collect()).unwrap_or_default(),
                "fills" => p.presets.fills.as_ref().map(|v| v.iter().map(|b| b.name.clone()).collect()).unwrap_or_default(),
                "gradients" => p.presets.gradients.as_ref().map(|v| v.iter().map(|b| b.name.clone()).collect()).unwrap_or_default(),
                _ => Vec::new(),
            })?;
            let mut names = names;
            if names.is_empty() {
                names = resource_names(name);
            }
            conv::json_to_js(&json!(names), ctx).map_err(jserr)
        }
        _ => ni(CLS, name),
    }
}

pub(super) fn set(doc: DocId, name: &str, a: Arg, ctx: &mut Context) -> FwResult<()> {
    let _ = ctx;
    match name {
        "gridOrigin" => {
            let (x, y) = a.point().ok_or_else(|| FwErr::Hr(hr::E_BAD_PARAM_TYPE, "Punkt".into()))?;
            exec(Some(doc), "view.grid.editGrid", json!({"origin": [x, y]})).map(|_| ())
        }
        "gridSize" => {
            let (x, y) = a.point().ok_or_else(|| FwErr::Hr(hr::E_BAD_PARAM_TYPE, "Punkt".into()))?;
            exec(Some(doc), "view.grid.editGrid", json!({"width": x.max(1.0), "height": y.max(1.0)})).map(|_| ())
        }
        "gridColor" => exec(Some(doc), "view.grid.editGrid", json!({"color": color_arg(&a).unwrap_or_else(|| "#666666".into())})).map(|_| ()),
        "left" | "top" => {
            super::exec_group(doc, vec![("document.setOrigin", json!({name: a.f64()}))])
        }
        _ => ni(CLS, &format!("{name} =")),
    }
}

/// Namen von Texturen/Mustern aus dem Ressourcenordner des Originals (falls eingerichtet).
fn resource_names(kind: &str) -> Vec<String> {
    let sub = match kind {
        "textures" => "Configuration\\Textures",
        "patterns" => "Configuration\\Patterns",
        _ => return Vec::new(),
    };
    let Some(dir) = fp_color::install::original_subdir(sub) else { return Vec::new() };
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .map(|it| it.flatten().filter_map(|e| e.path().file_stem().map(|s| s.to_string_lossy().to_string())).collect())
        .unwrap_or_default();
    v.sort();
    v
}

/// Ein Element einzeln als PNG (transparent, auf seine Grenzen zugeschnitten).
fn export_element(doc: DocId, id: ElementId, path: &std::path::Path) -> FwResult<()> {
    let (model, page, frame) = st(|s| {
        let d = s.session.document(doc).ok_or_else(|| FwErr::bad("Dokument"))?;
        Ok((Arc::clone(&d.doc), d.page, d.frame))
    })?;
    let e = element(doc, id)?;
    let b = fp_engine::docops::element_bounds(&model, &e).ok_or_else(|| FwErr::bad("Element ohne Grenzen"))?;
    let mut m = (*model).clone();
    let top_level = model.pages.get(page).is_some_and(|p| p.root.layers.iter().any(|l| matches!(l, LayerSlot::Own(l) if l.cel(frame).is_some_and(|c| c.elements.iter().any(|x| x.id == id)))));
    if top_level && let Some(p) = m.page_mut(page) {
        for slot in &mut p.root.layers {
            if let LayerSlot::Own(l) = slot {
                let l = Arc::make_mut(l);
                for c in &mut l.cels {
                    Arc::make_mut(c).elements.retain(|x| x.id == id);
                }
            }
        }
    }
    let r = fp_render::Renderer::new(fp_render::RenderOptions { background: fp_render::BackgroundMode::Transparent, ..fp_render::RenderOptions::default() });
    let area = fp_geom::IRect::new(b.left.floor() as i32, b.top.floor() as i32, b.right.ceil() as i32, b.bottom.ceil() as i32);
    let img = r.render_rect(&m, page, frame, area).image;
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, img.width() as u32, img.height() as u32);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().map_err(|e| FwErr::generic(e.to_string()))?;
        w.write_image_data(img.as_bytes()).map_err(|e| FwErr::generic(e.to_string()))?;
    }
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    std::fs::write(path, out).map_err(|e| FwErr::generic(format!("{}: {e}", path.display())))
}
// ------------------------------------------------------------------ Find, SetCurrentPage, Document (statisch), Widget

pub(super) fn find_method(name: &str, h: &Handle) -> FwResult<JsValue> {
    let Handle::Find(i) = h else { return Err(FwErr::bad("kein Suchobjekt")) };
    let p = with_state(|s| s.finds.get(*i).map(|x| x.0.clone()))?.ok_or_else(|| FwErr::bad("Suchobjekt"))?;
    match name {
        "replaceAll" => {
            let doc = DocId(p["__doc"].as_u64().unwrap_or(0));
            let n = super::fw::replace_in_doc(doc, &p, None)?;
            with_state(|s| {
                if let Some(f) = s.finds.get_mut(*i) {
                    f.1 = n;
                }
            })?;
            Ok(JsValue::from(n > 0))
        }
        _ => ni("Find", name),
    }
}

pub(super) fn find_get(name: &str, h: &Handle) -> FwResult<JsValue> {
    let Handle::Find(i) = h else { return Err(FwErr::bad("kein Suchobjekt")) };
    match name {
        "numItemsReplaced" => Ok(JsValue::from(with_state(|s| s.finds.get(*i).map(|x| x.1).unwrap_or(0))? as i32)),
        _ => ni("Find", name),
    }
}

pub(super) fn page_setter_method(name: &str, h: &Handle) -> FwResult<JsValue> {
    let Handle::Document(doc) = h else { return Err(FwErr::bad("kein Dokument")) };
    match name {
        // reset(): zurück zur ersten Seite
        "reset" => {
            exec(Some(*doc), "page.setCurrent", json!({"index": 0}))?;
            Ok(JsValue::from(true))
        }
        _ => ni("SetCurrentPage", name),
    }
}

pub(super) fn page_setter_get(name: &str, h: &Handle) -> FwResult<JsValue> {
    let Handle::Document(doc) = h else { return Err(FwErr::bad("kein Dokument")) };
    match name {
        "pageNum" => Ok(JsValue::from(page_root(*doc, |_, p, _| p)? as i32)),
        _ => ni("SetCurrentPage", name),
    }
}

/// Exporteinstellungen zu einem Vorgabenamen („GIF Websichere 128“, „JPEG – Höhere Qualität“ …).
fn export_preset(name: &str) -> Value {
    let n = name.to_lowercase();
    let digits: String = n.chars().filter(|c| c.is_ascii_digit()).collect();
    let num = digits.parse::<u32>().ok();
    if n.contains("jpeg") || n.contains("jpg") {
        let q = if n.contains("klein") || n.contains("smaller") { 60 } else if n.contains("höher") || n.contains("better") || n.contains("higher") { 80 } else { num.unwrap_or(80) };
        json!({"exportFormat": "JPEG", "jpegQuality": q.min(100), "colorMode": "24 bit", "name": name})
    } else if n.contains("png") {
        let bits = if n.contains("32") { "32 bit" } else if n.contains("24") { "24 bit" } else { "indexed" };
        json!({"exportFormat": "PNG", "colorMode": bits, "numPaletteEntries": if bits == "indexed" { 256 } else { 0 }, "name": name})
    } else {
        let colors = num.unwrap_or(256).clamp(2, 256);
        json!({"exportFormat": "GIF", "colorMode": "indexed", "numPaletteEntries": colors, "paletteMode": if n.contains("web") { "websnap adaptive" } else { "adaptive" }, "name": name})
    }
}

pub(super) fn static_method(name: &str, args: &[Arg], ctx: &mut Context) -> FwResult<JsValue> {
    match name {
        "findExportFormatOptionsByName" => {
            let n = args.first().and_then(|a| a.str()).unwrap_or_default();
            conv::json_to_js(&export_preset(&n), ctx).map_err(jserr)
        }
        "makeGoodNativeFilePath" => {
            let p = args.first().and_then(|a| a.str()).unwrap_or_default();
            let clean: String = p.chars().map(|c| if matches!(c, '<' | '>' | '"' | '|' | '?' | '*') || (c as u32) < 32 { '_' } else { c }).collect();
            Ok(conv::s(&clean))
        }
        _ => ni("DocumentStatic", name),
    }
}

/// Ausgewählte Rich-Symbol-Instanz (`Widget.elem`) mit Skript und Werten.
/// (Dokument, Instanz, Skript des Symbols, Instanzwerte).
type WidgetRef = (DocId, ElementId, Option<String>, Option<String>);

fn widget_instance() -> FwResult<Option<WidgetRef>> {
    st(|s| {
        let Some(d) = s.session.active() else { return Ok(None) };
        for id in &d.selection.elements {
            if let Some(e) = fp_engine::docops::element_on_page(&d.doc, d.page, *id)
                && let ElementKind::Instance(i) = &e.kind
            {
                let script = d.doc.symbols.iter().find(|x| x.id == i.symbol_id).and_then(|x| x.widget_script.clone());
                return Ok(Some((d.id, *id, script, i.widget_params.clone())));
            }
        }
        Ok(None)
    })
}

pub(super) fn widget_method(name: &str, args: &[Arg], ctx: &mut Context) -> FwResult<JsValue> {
    match name {
        "isWidgetSelected" => Ok(JsValue::from(widget_instance()?.is_some_and(|w| w.2.is_some()))),
        "UpdateWidget" => {
            // Werte neu anwenden: Instanz neu zeichnen (Cache verwerfen)
            if let Some((doc, id, _, _)) = widget_instance()? {
                super::edit_doc(doc, |d| {
                    let page = d.page;
                    fp_engine::docops::edit_by_id(d.doc_mut(), page, id, |e| e.cache = None);
                })?;
            }
            Ok(JsValue::undefined())
        }
        "GetObjectByName" => {
            let n = args.first().and_then(|a| a.str()).unwrap_or_default();
            let Some((doc, _, _, _)) = widget_instance()? else { return Ok(JsValue::null()) };
            let ids = ids_named(doc, &n)?;
            match ids.first() {
                Some(id) => {
                    let e = element(doc, *id)?;
                    super::wrap_element(ctx, doc, &e)
                }
                None => Ok(JsValue::null()),
            }
        }
        _ => ni("Widget", name),
    }
}

pub(super) fn widget_get(name: &str, ctx: &mut Context) -> FwResult<JsValue> {
    let w = widget_instance()?;
    match name {
        "elem" => match w {
            Some((doc, id, _, _)) => {
                let e = element(doc, id)?;
                super::wrap_element(ctx, doc, &e)
            }
            None => Ok(JsValue::null()),
        },
        "opCode" => Ok(JsValue::from(2)),
        "propString" => Ok(w.and_then(|w| w.3).map(|s| conv::s(&s)).unwrap_or_else(|| conv::s(""))),
        "jsCode" => Ok(w.and_then(|w| w.2).map(|s| conv::s(&s)).unwrap_or_else(|| conv::s(""))),
        _ => ni("Widget", name),
    }
}

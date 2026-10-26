//! Ebenen-Bedienfeld (`CLayerInspectorPanel`, Vorlage 4127, 11 §2.2): Mischmodus und Deckkraft der
//! Auswahl, Baumliste Ebenen/Unterebenen → Objekte mit Sichtbarkeit, Sperre, Miniaturen (fp-render),
//! Ausklappen, Umbenennen per Doppelklick, Ziehen zum Umsortieren (Objekte zwischen Ebenen, Ebenen
//! in der Ebenenliste), Web-Ebene, Teilen über Status (Kennzeichen), Fußleiste mit Status-Knopf,
//! Neue Unterebene, Neue/Doppelte Ebene, Neues Bitmap-Bild, Maske hinzufügen, Löschen.

use super::common::{self, FooterButton};
use super::thumbs;
use crate::app_state::{Action, AppState};
use crate::inspector::fields::{self, Dl};
use crate::inspector::glyphs::{self, Glyph};
use crate::inspector::obj_quads::{BLEND_ITEMS, blend_index, blend_names};
use crate::theme::Theme;
use egui::{Align2, Color32, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use fp_model::{Element, ElementKind, LayerId, LayerKind, LayerOrigin};
use serde_json::json;
use std::collections::BTreeSet;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
enum RowKind {
    Layer {
        id: LayerId,
        editable: bool,
        web: bool,
    },
    Element {
        id: fp_model::ElementId,
        layer: LayerId,
        index: usize,
        editable: bool,
    },
}

#[derive(Clone, Debug)]
struct RowInfo {
    kind: RowKind,
    rect: Rect,
    /// Ebene im aktuellen Status gesperrt (Ziel „hinein“ verworfen, re 42 §2).
    locked: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum Drag {
    Element {
        id: fp_model::ElementId,
    },
    /// `y0` = Druckpunkt (senkrechte Zugschwelle, re 41 §3.2).
    Layer {
        id: LayerId,
        y0: f32,
    },
}

/// Rows of master-page layers (and sublayers inside shared or master-page layers) whose disclosure
/// the user flipped, per document (document id, row key). Own and shared layers and the sublayers of
/// own layers keep their disclosure in the document (`DISb`, `layer.setExpanded`); the others are
/// not reachable for that command on this page, so their triangle only flips the view of this
/// document.
const FLIPPED: &str = "layers-disclosure-flipped";
const DRAG: &str = "layers-drag";
const RENAME: &str = "layers-rename";
const ANCHOR: &str = "layers-anchor";
/// Documents whose list was already scrolled to the current layer once (on opening).
const SCROLL_SEEN: &str = "layers-scroll-seen";

/// Key of a list row for the selection anchor (FB-30, RE spec 61 §3). Layer and sublayer rows carry
/// their layer id, object rows their element id (two separate number spaces).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RowKey {
    Layer(LayerId),
    Element(fp_model::ElementId),
}

/// What a click on an object row does with the selection (RE spec 61 §3.3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ClickMods {
    /// Shift: range from the anchor, replaces the selection (Ctrl+Shift behaves the same).
    range: bool,
    /// Ctrl (Cmd on macOS): toggle the row, the anchor stays.
    toggle: bool,
}

/// Maps the keyboard modifiers of a row click. `command` is Ctrl on Windows/Linux and Cmd on macOS
/// (egui sets `mac_cmd` there as well); the Ctrl key itself also toggles, so a Ctrl+click on a Mac
/// does what the tester expected instead of nothing.
fn click_mods(m: &egui::Modifiers) -> ClickMods {
    ClickMods {
        range: m.shift,
        toggle: m.command || m.mac_cmd || m.ctrl,
    }
}

/// One object of the page's object order (RE spec 63 §1.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SeqItem {
    id: fp_model::ElementId,
    /// The layer or sublayer whose own object it is.
    owner: LayerId,
    /// Left out of ranges and not clickable: the object itself, its layer or a parent layer is
    /// locked, or the layer is not the page's own (shared from another page, master page).
    locked: bool,
}

/// The object order of the current page and state as the Layers panel lists it top to bottom
/// (layers top to bottom, sublayers at their place inside the parent layer), independent of
/// whether a layer is collapsed (TW-208 rest, RE spec 63 §1.3).
#[derive(Clone, Debug, Default)]
struct ObjSeq {
    items: Vec<SeqItem>,
    /// Layers and sublayers whose rows ignore clicks (locked themselves or through a parent, or
    /// not the page's own).
    locked_layers: BTreeSet<LayerId>,
}

impl ObjSeq {
    /// Builds the order from the page's layers (`layers` bottom to top as stored).
    fn build(layers: &[fp_model::ResolvedLayer], frame: usize) -> ObjSeq {
        fn walk(
            els: &[Arc<Element>],
            owner: LayerId,
            locked: bool,
            frame: usize,
            out: &mut ObjSeq,
        ) {
            for e in els {
                if let ElementKind::SubLayer(sl) = &e.kind {
                    // a locked parent locks its sublayers (RE spec 63 §1.2 `sublocked`)
                    let sub_locked =
                        locked || e.common.locked || sl.cel(frame).is_some_and(|c| c.locked);
                    if sub_locked {
                        out.locked_layers.insert(sl.id);
                    }
                    if let Some(cel) = sl.cel(frame) {
                        walk(&cel.elements, sl.id, sub_locked, frame, out);
                    }
                } else {
                    out.items.push(SeqItem {
                        id: e.id,
                        owner,
                        locked: locked || e.common.locked,
                    });
                }
            }
        }
        let mut out = ObjSeq::default();
        for rl in layers.iter().rev() {
            let l = &rl.layer;
            let locked = rl.origin != LayerOrigin::Own || l.cel(frame).is_some_and(|c| c.locked);
            if locked {
                out.locked_layers.insert(l.id);
            }
            if let Some(cel) = l.cel(frame) {
                walk(&cel.elements, l.id, locked, frame, &mut out);
            }
        }
        out
    }

    fn pos(&self, id: fp_model::ElementId) -> Option<usize> {
        self.items.iter().position(|it| it.id == id)
    }

    /// Clicks on this row change nothing (RE spec 63 §1.3 rule 1).
    fn row_locked(&self, row: RowKey) -> bool {
        match row {
            RowKey::Layer(l) => self.locked_layers.contains(&l),
            RowKey::Element(id) => self.items.iter().any(|it| it.id == id && it.locked),
        }
    }

    /// Positions a row stands for in a range: an object row its object, a layer or sublayer row
    /// the first to the last of its own objects (without those of its sublayers).
    fn span(&self, row: RowKey) -> Option<(usize, usize)> {
        match row {
            RowKey::Element(id) => self.pos(id).map(|p| (p, p)),
            RowKey::Layer(l) => {
                let mut own = self
                    .items
                    .iter()
                    .enumerate()
                    .filter(|(_, it)| it.owner == l)
                    .map(|(i, _)| i);
                let first = own.next()?;
                Some((first, own.next_back().unwrap_or(first)))
            }
        }
    }

    /// The layer's own objects a click on its row selects or toggles (locked ones left out).
    fn own_unlocked(&self, l: LayerId) -> Vec<fp_model::ElementId> {
        self.items
            .iter()
            .filter(|it| it.owner == l && !it.locked)
            .map(|it| it.id)
            .collect()
    }

    /// The anchor in effect (RE spec 63 §1.3 rule 5): the stored one while its object (a layer
    /// anchor: one of its own objects) is selected, else the bottom-most selected object of the
    /// order, with nothing selected the bottom-most object of the page.
    fn effective_anchor(
        &self,
        selected: &[fp_model::ElementId],
        anchor: Option<RowKey>,
    ) -> Option<RowKey> {
        let valid = anchor.filter(|a| match *a {
            RowKey::Element(id) => selected.contains(&id) && self.pos(id).is_some(),
            RowKey::Layer(l) => self
                .items
                .iter()
                .any(|it| it.owner == l && selected.contains(&it.id)),
        });
        valid.or_else(|| {
            selected
                .iter()
                .filter_map(|id| self.pos(*id))
                .max()
                .or_else(|| self.items.len().checked_sub(1))
                .map(|p| RowKey::Element(self.items[p].id))
        })
    }

    /// `ids` in the order of the panel; objects outside the order keep their place after them.
    fn sorted(&self, mut ids: Vec<fp_model::ElementId>) -> Vec<fp_model::ElementId> {
        ids.sort_by_key(|id| self.pos(*id).unwrap_or(usize::MAX));
        ids
    }
}

/// Selection after a click on the row `row` (object, layer or sublayer row) with Shift or Ctrl,
/// or a plain click on an object row (FB-30, TW-208; RE spec 61 §3.3, RE spec 63 §1.3).
///
/// `selected` is the current selection, `anchor` the stored anchor (checked here, rule 5).
/// Returns the new selection in panel order (`None`: unchanged) and the anchor to store (`None`:
/// keep the stored one).
///
/// - A row of a locked object or layer: nothing changes.
/// - Shift (also Ctrl+Shift): range over the page's object order from the anchor's span to the
///   row's span (a layer row spans its own objects), locked objects left out, objects of hidden,
///   collapsed and shared layers and of sublayers included; replaces the selection, the anchor
///   stays.
/// - Ctrl/Cmd: toggles the object, on a layer row each own object one by one; the anchor stays.
/// - Plain click on an object row: only this object, anchor = this row; a click on a row of a
///   multi-selection keeps it. A plain click on a layer row selects its own objects and makes the
///   layer the anchor (the panel also makes it the current layer).
fn row_click(
    seq: &ObjSeq,
    selected: &[fp_model::ElementId],
    anchor: Option<RowKey>,
    row: RowKey,
    mods: ClickMods,
) -> (Option<Vec<fp_model::ElementId>>, Option<RowKey>) {
    if seq.row_locked(row) {
        return (None, None);
    }
    if mods.range {
        let Some(a) = seq.effective_anchor(selected, anchor) else {
            return (None, None);
        };
        let (Some((a0, a1)), Some((b0, b1))) = (seq.span(a), seq.span(row)) else {
            return (None, None);
        };
        let (lo, hi) = (a0.min(b0), a1.max(b1));
        let ids = seq.items[lo..=hi]
            .iter()
            .filter(|it| !it.locked)
            .map(|it| it.id)
            .collect();
        return (Some(ids), Some(a));
    }
    if mods.toggle {
        let flip = match row {
            RowKey::Element(id) => vec![id],
            RowKey::Layer(l) => seq.own_unlocked(l),
        };
        if flip.is_empty() {
            return (None, None);
        }
        let mut ids: Vec<fp_model::ElementId> = selected.to_vec();
        for id in flip {
            if ids.contains(&id) {
                ids.retain(|x| *x != id);
            } else {
                ids.push(id);
            }
        }
        return (Some(seq.sorted(ids)), None);
    }
    match row {
        RowKey::Element(id) if selected.len() > 1 && selected.contains(&id) => (None, None),
        RowKey::Element(id) => (Some(vec![id]), Some(row)),
        RowKey::Layer(l) => (Some(seq.own_unlocked(l)), Some(row)),
    }
}

fn element_label(e: &Element) -> String {
    if let Some(n) = &e.common.name
        && !n.is_empty()
    {
        return n.clone();
    }
    // maskiertes Einzelelement (Bitmap/Pfad mit Maske): wie im Original Zeile des Inhalts (FIX-G5, Q-02)
    if let Some((child, _)) = e.masked_single() {
        return element_label(child);
    }
    let t = common::t;
    match &e.kind {
        ElementKind::Bitmap(_) => t("panel-layers-kind-bitmap"),
        ElementKind::Path(p) if p.primitive.is_some() => t("panel-layers-kind-rectangle"),
        ElementKind::Path(_) => t("panel-layers-kind-path"),
        ElementKind::Text(tx) => {
            let s = tx.plain_text();
            let s: String = s.chars().take(24).collect();
            if s.trim().is_empty() {
                t("panel-layers-kind-text")
            } else {
                s
            }
        }
        ElementKind::Group(_) => t("panel-layers-kind-group"),
        ElementKind::CompoundShape(_) => t("panel-layers-kind-compound"),
        ElementKind::Instance(_) => t("panel-layers-kind-symbol"),
        ElementKind::WebObject(w) if w.slice.is_some() => t("panel-layers-kind-slice"),
        ElementKind::WebObject(_) => t("panel-layers-kind-hotspot"),
        ElementKind::SubLayer(l) => l.name.clone(),
        ElementKind::Raw(_) => t("panel-layers-kind-object"),
    }
}

/// Zeichnet das Bedienfeld.
pub fn show(ui: &mut Ui, st: &mut AppState) {
    let theme = Theme::current(ui.ctx());
    let c = theme.colors.clone();
    let Some(d) = st.session.active() else {
        return;
    };
    let doc = Arc::clone(&d.doc);
    let (doc_id, doc_gen, page, frame) = (d.id, d.generation, d.page, d.frame);
    let layers = doc.resolved_layers(page);
    let selected: Vec<fp_model::ElementId> = d.selection.elements.clone();
    // Nachschlagen je Zeile ohne lineare Suche (PERF-1: Tausende Zeilen × Tausende gewählte)
    let selected_set: std::collections::HashSet<fp_model::ElementId> =
        selected.iter().copied().collect();
    let active_layer = d.layer;
    // „Ebene gewählt“: Auswahl = alle (ungesperrten) Objekte der aktiven Ebene, auch leer
    // auch eine aktive Unterebene (B-9)
    let layer_objects = |lid: LayerId| -> Vec<fp_model::ElementId> {
        let objs = |l: &fp_model::Layer| {
            l.cel(frame)
                .map(|cel| own_objects(&cel.elements))
                .unwrap_or_default()
        };
        match layers.iter().find(|rl| rl.layer.id == lid) {
            Some(rl) => objs(&rl.layer),
            None => fp_engine::docops::layer_by_id(&doc, page, lid)
                .map(|l| objs(&l))
                .unwrap_or_default(),
        }
    };
    let layer_selected = active_layer.is_some_and(|lid| {
        let mut a = layer_objects(lid);
        let mut b = selected.clone();
        a.sort();
        b.sort();
        a == b
    });
    // RE spec 64 §2.5: the layers and sublayers holding a selected object (their own objects,
    // objects in groups count for the group's layer) – filled dot and light tint
    let sel_layers: std::collections::HashSet<LayerId> = if selected.is_empty() {
        Default::default()
    } else {
        fp_engine::docops::find_many_on_page(&doc, page, &selected)
            .into_iter()
            .flatten()
            .filter_map(|p| fp_engine::docops::owning_layer(&doc, page, &p))
            .collect()
    };
    // with nothing selected the current layer's row is highlighted and bold; with a selection the
    // current layer and the layers of the selection are tinted light, not bold
    let row_marks = |id: LayerId| -> (bool, bool) {
        let cur = active_layer == Some(id);
        if selected.is_empty() {
            (cur, false)
        } else {
            (false, cur || sel_layers.contains(&id))
        }
    };
    // RE spec 64 §2.5: a circle at the right of every layer row, filled where the layer holds a
    // selected object; it does not mark the current layer. RE spec 65 rule 2: grey where the layer
    // has no objects of its own (its sublayers do not count), the web layer alike
    let paint_dot = |ui: &Ui, center: Pos2, id: LayerId, grey: bool| {
        let col = if grey { c.icon_disabled } else { c.icon_fg };
        ui.painter()
            .circle_stroke(center, 4.5, Stroke::new(1.0, col));
        if sel_layers.contains(&id) {
            ui.painter().circle_filled(center, 2.0, col);
        }
    };
    // on opening a document the list scrolls to the current layer's row (RE spec 64 §1.1)
    let scroll_seen: BTreeSet<u64> = ui
        .data(|dd| dd.get_temp(Id::new(SCROLL_SEEN)))
        .unwrap_or_default();
    let scroll_to_current = !scroll_seen.contains(&doc_id.0);
    if scroll_to_current {
        let mut seen = scroll_seen.clone();
        seen.insert(doc_id.0);
        ui.data_mut(|dd| dd.insert_temp(Id::new(SCROLL_SEEN), seen));
    }
    let first_sel = selected
        .first()
        .and_then(|id| fp_engine::docops::element_on_page(&doc, page, *id));
    // Kopf: Mischmodus (4261), Deckkraft (4260)
    let (head, _) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::hover());
    let dl = Dl::at(head.min + vec2(2.0, 1.0));
    let en = first_sel.is_some();
    // both act on the selected objects (layers have no opacity or blend mode of their own); mixed
    // values of several selected objects show a blank field (RE-29, re 61 §4)
    let sel_elems: Vec<Arc<fp_model::Element>> = selected
        .iter()
        .filter_map(|id| fp_engine::docops::element_on_page(&doc, page, *id))
        .collect();
    let mixed_op = sel_elems
        .windows(2)
        .any(|w| w[0].common.opacity != w[1].common.opacity);
    let mixed_bl = sel_elems
        .windows(2)
        .any(|w| w[0].common.blend != w[1].common.blend);
    let names = blend_names();
    let bl = first_sel
        .as_ref()
        .and_then(|e| blend_index(e.common.blend))
        .unwrap_or(0);
    if let Some(i) = fields::combo(
        ui,
        "layers-blend",
        dl.r(1.0, 2.0, 80.0, 12.0),
        &names,
        (!mixed_bl).then_some(bl),
        en,
    ) && BLEND_ITEMS[i].1 >= 0
    {
        st.run(
            "element.setBlendMode",
            json!({"mode": BLEND_ITEMS[i].1.to_string()}),
        );
    }
    common::label_fit(
        ui,
        dl.r(84.0, 3.0, 35.0, 8.0),
        &common::t("panel-layers-opacity"),
        true,
    );
    let op = first_sel
        .as_ref()
        .map(|e| e.common.opacity.to_percent().round());
    // whole per cents at 2 px each like the reference's opacity pop-up (R-32, RE spec 61 §4.1)
    if let Some(v) = fields::opacity_number(
        ui,
        "layers-opacity",
        dl.r(120.0, 2.0, 21.0, 12.0),
        dl.r(142.0, 2.0, 9.0, 12.0),
        if mixed_op { None } else { op.or(Some(100.0)) },
        en,
    ) {
        st.run("element.setOpacity", json!({"opacity": v}));
    }
    // live canvas preview of the dragged value like the inspector's opacity slider (FB-28,
    // TW-205); applied on release / pop-up close, the Blend mode menu stays a plain commit
    if en && let Some(v) = fields::slider_live(ui.ctx(), "layers-opacity") {
        st.inspector_preview = crate::inspector::preview::InspectorPreview::for_selection(
            st,
            crate::inspector::preview::PreviewValue::Opacity(v),
        );
    }
    // R-32: the disclosure triangle shows the document's flag (`DISb`), it is not panel state that
    // would carry over to another document whose layers have the same ids
    let flipped: BTreeSet<(u64, u64)> = ui
        .data(|dd| dd.get_temp(Id::new(FLIPPED)))
        .unwrap_or_default();
    let mut toggle_flip: Option<u64> = None;
    let mut actions: Vec<Action> = Vec::new();
    let mut rows: Vec<RowInfo> = Vec::new();
    let mut start_drag: Option<Drag> = None;
    let mut rename_start: Option<RowKind> = None;
    // Zeilen für das Kontextmenü 6923 (Rechtsklick auf Ebenen- oder Objektzeile, wie im Original)
    let mut menu_rows: Vec<egui::Response> = Vec::new();
    // Ziehen über die Augenspalte (wie im Original): alle überstrichenen Ebenen erhalten den Zustand,
    // den die erste Ebene durch den Klick bekommen hat
    let eye_key = Id::new("layers-eye-drag");
    // (Zielzustand, schon geschaltete Ebenen, Zeiger-y des Vorframes, schon ein Schritt im Verlauf)
    let mut eye_drag: Option<(bool, BTreeSet<u64>, Option<f32>, bool)> =
        ui.data(|dd| dd.get_temp(eye_key));
    let (pointer, primary_down) = ui.input(|i| (i.pointer.latest_pos(), i.pointer.primary_down()));
    if !primary_down {
        eye_drag = None;
    }
    // TW-69: überstrichene Höhe seit dem letzten Frame (schnelles Ziehen überspringt sonst Zeilen)
    let sweep_y = pointer.map(|p| {
        let prev = eye_drag.as_ref().and_then(|d| d.2).unwrap_or(p.y);
        (prev.min(p.y), prev.max(p.y))
    });
    let swept = |r: Rect, eye: Rect| {
        pointer.is_some_and(|p| p.x >= eye.left() - 2.0 && p.x <= eye.right() + 2.0)
            && sweep_y.is_some_and(|(y0, y1)| r.top() <= y1 && r.bottom() > y0)
    };
    let press_origin = ui.input(|i| i.pointer.press_origin());
    let rename: Option<RowKind> = ui.data(|dd| dd.get_temp(Id::new(RENAME)));
    // FB-30 (RE spec 61 §3): Shift = range from the anchor, Ctrl/Cmd = toggle
    let mods = ui.input(|i| click_mods(&i.modifiers));
    // anchor of the last plain row click in this document; whether it still counts depends on
    // the selection when a Shift+click uses it (TW-208 rest, RE spec 63 §1.3 rule 5)
    let anchor: Option<RowKey> = ui
        .data(|dd| dd.get_temp::<(u64, RowKey)>(Id::new(ANCHOR)))
        .filter(|(doc, _)| *doc == doc_id.0)
        .map(|(_, k)| k);
    let mut new_anchor: Option<RowKey> = None;
    // row clicks (name part of object, layer and sublayer rows) are resolved after the list is
    // built, over the page's whole object order (collapsed layers included)
    let mut pending_click: Option<(RowKey, ClickMods)> = None;
    let alt = ui.input(|i| i.modifiers.alt);
    // Firepixel extension (FB-59 to FB-61): group rows expand to their objects, masked objects to
    // the objects of their mask; off = the rows of the reference only
    let tree_on = fp_engine::session::layers_group_tree(&st.session.prefs);
    let tree_open: BTreeSet<(u64, u64)> = if tree_on {
        ui.data(|dd| dd.get_temp(Id::new(TREE_OPEN)))
            .unwrap_or_default()
    } else {
        BTreeSet::new()
    };
    let is_tree_open = |k: u64| tree_open.contains(&(doc_id.0, k));
    let mut tree_toggles: Vec<u64> = Vec::new();
    let mut tree_click: Option<TreeClick> = None;
    let yellow_bg = group_child_selected_bg(&theme);
    // Miniaturoptionen (0 = keine; Vorgabe 32 → 24 px wie im Original „mittel“)
    let tpx = (st
        .session
        .prefs
        .get_int("LayersList_ThumbnailSize", 32)
        .clamp(0, 128) as f32
        * 0.75)
        .round();
    let erow_h = if tpx > 0.0 { tpx + 6.0 } else { 20.0 };
    let list_rect = common::list_frame(ui, "layers-list", common::FOOTER_H, |ui| {
        for rl in layers.iter().rev() {
            let l = &rl.layer;
            let editable = rl.origin == LayerOrigin::Own;
            let web = l.kind == LayerKind::Web;
            let (is_active, tinted) = row_marks(l.id);
            let (rect, resp) = common::row_tinted(ui, 23.0, is_active, tinted);
            if scroll_to_current && active_layer == Some(l.id) {
                ui.scroll_to_rect(rect, None);
            }
            // Sichtbarkeit, Sperre
            let eye_r = Rect::from_min_size(rect.min, vec2(20.0, rect.height()));
            let lock_r = Rect::from_min_size(
                pos2(rect.left() + 21.0, rect.top()),
                vec2(19.0, rect.height()),
            );
            ui.painter().vline(
                rect.left() + 20.5,
                rect.y_range(),
                Stroke::new(1.0, c.list_row_line),
            );
            ui.painter().vline(
                rect.left() + 40.5,
                rect.y_range(),
                Stroke::new(1.0, c.list_row_line),
            );
            // TW-54: wie Leinwand und Skript (`frames[n].visible`): Status-Eintrag und Cel
            let visible = l.is_shown(frame);
            if visible {
                glyphs::paint(ui.painter(), eye_r.shrink(3.0), Glyph::Eye, &theme, true);
            }
            let locked = l.cel(frame).is_some_and(|cel| cel.locked);
            if locked {
                glyphs::paint(ui.painter(), lock_r.shrink(4.0), Glyph::Lock, &theme, true);
            }
            let tri =
                Rect::from_center_size(pos2(rect.left() + 50.0, rect.center().y), vec2(10.0, 10.0));
            // own and shared layers: the flag of the document; master-page layers: flipped per document
            let model_disclosure = matches!(rl.origin, LayerOrigin::Own | LayerOrigin::Shared);
            let is_open =
                l.expanded != (!model_disclosure && flipped.contains(&(doc_id.0, l.id.0)));
            let is_collapsed = !is_open;
            paint_triangle(ui, tri, is_open, c.icon_fg);
            glyphs::paint(
                ui.painter(),
                Rect::from_center_size(pos2(rect.left() + 66.0, rect.center().y), vec2(16.0, 14.0)),
                Glyph::Folder,
                &theme,
                true,
            );
            let name = if web {
                common::t("panel-layers-web-layer")
            } else {
                l.name.clone()
            };
            let name_r = Rect::from_min_max(
                pos2(rect.left() + 78.0, rect.top() + 2.0),
                pos2(rect.right() - 22.0, rect.bottom() - 2.0),
            );
            let renaming = rename
                .as_ref()
                .is_some_and(|k| matches!(k, RowKind::Layer { id, .. } if *id == l.id));
            if renaming {
                if let Some(res) =
                    common::inline_rename(ui, Id::new(("layer-rename", l.id.0)), name_r, &name)
                {
                    if let Some(n) = res {
                        actions.push(Action::Run(
                            "layer.rename".into(),
                            json!({"layer": l.id.0, "name": n}),
                        ));
                    }
                    ui.data_mut(|dd| dd.remove::<RowKind>(Id::new(RENAME)));
                }
            } else {
                let font = if is_active {
                    theme.bold(theme.fonts.ui)
                } else {
                    theme.ui_font()
                };
                let col = if editable { c.text } else { c.text_weak };
                crate::widgets::text_clipped(
                    ui.painter(),
                    pos2(name_r.left(), rect.center().y),
                    Align2::LEFT_CENTER,
                    &name,
                    font,
                    col,
                    name_r.width(),
                );
            }
            // Kennzeichen rechts: über Status geteilt / geteilte Ebene / Masterseite; the web layer
            // (always shared across states) shows its selection circle (RE spec 64 §2.5)
            let mark = Rect::from_center_size(
                pos2(rect.right() - 11.0, rect.center().y),
                vec2(12.0, 12.0),
            );
            let dot_shown = web || !(l.shared_across_frames || rl.origin != LayerOrigin::Own);
            if !dot_shown {
                glyphs::paint(
                    ui.painter(),
                    mark,
                    if rl.origin == LayerOrigin::Own {
                        Glyph::Distribute
                    } else {
                        Glyph::Link
                    },
                    &theme,
                    true,
                );
            } else {
                paint_dot(
                    ui,
                    mark.center(),
                    l.id,
                    !fp_engine::docops::layer_has_own_objects(l, frame),
                );
            }
            let kind = RowKind::Layer {
                id: l.id,
                editable,
                web,
            };
            if resp.clicked()
                && let Some(p) = resp.interact_pointer_pos()
            {
                if eye_r.contains(p) && editable && alt {
                    // TW-54: Alt+Klick aufs Auge wirkt wie im Original auf alle Ebenen samt Unterebenen
                    let cmd = if visible {
                        "panel.layers.hideAll"
                    } else {
                        "panel.layers.showAll"
                    };
                    actions.push(Action::Run(cmd.into(), json!({})));
                } else if eye_r.contains(p) && editable {
                    actions.push(Action::Run(
                        "layer.setVisible".into(),
                        json!({"layer": l.id.0, "visible": !visible}),
                    ));
                } else if lock_r.contains(p) && editable {
                    actions.push(Action::Run(
                        "layer.setLocked".into(),
                        json!({"layer": l.id.0, "locked": !locked}),
                    ));
                } else if tri.expand(3.0).contains(p) {
                    if model_disclosure {
                        actions.push(Action::Run(
                            "layer.setExpanded".into(),
                            json!({"layer": l.id.0, "expanded": !is_open}),
                        ));
                    } else {
                        toggle_flip = Some(l.id.0);
                    }
                } else if editable && dot_shown && mark.expand(2.0).contains(p) {
                    // RE spec 65 rule 1 (TW-217): the circle moves the selection into this layer
                    // (Alt: copies); Shift and Ctrl change nothing; the engine leaves the no-op
                    // cases (nothing selected, filled circle, locked target, web layer) alone
                    actions.push(Action::Run(
                        "selection.moveToLayer".into(),
                        json!({"layer": l.id.0, "copy": alt}),
                    ));
                } else if editable {
                    // a click selects the layer's own objects, not those of its sublayers (re 28
                    // S3); Shift ranges to them, Ctrl toggles them; a locked layer ignores it (re 63)
                    pending_click = Some((RowKey::Layer(l.id), mods));
                } else if rl.origin == LayerOrigin::Shared {
                    actions.push(Action::Run(
                        "pi.layer.setActiveShared".into(),
                        json!({"layer": l.id.0}),
                    ));
                }
            }
            // Q-59 (RE spec 58 §3.2): a right-click on a layer row makes it the current layer (the
            // web layer too) before the context menu forms its states; the object selection stays
            if resp.secondary_clicked() && active_layer != Some(l.id) {
                if editable {
                    actions.push(Action::Run(
                        "layer.setActive".into(),
                        json!({"layer": l.id.0}),
                    ));
                } else if rl.origin == LayerOrigin::Shared {
                    actions.push(Action::Run(
                        "pi.layer.setActiveShared".into(),
                        json!({"layer": l.id.0}),
                    ));
                }
            }
            // egui wertet einen Doppelklick kurz nach einem Klick anderswo als Dreifachklick
            if (resp.double_clicked() || resp.triple_clicked()) && editable && !web {
                rename_start = Some(kind.clone());
            }
            if resp.drag_started() && press_origin.is_some_and(|p| eye_r.contains(p)) && editable {
                // Klick hat die Ebene schon umgeschaltet, sobald das Ziehen beginnt: Ziel = neuer Zustand
                let mut done = BTreeSet::new();
                done.insert(l.id.0);
                let stepped = eye_drag.as_ref().is_some_and(|d| d.3);
                if eye_drag.is_none() {
                    actions.push(Action::Run(
                        "layer.setVisible".into(),
                        json!({"layer": l.id.0, "visible": !visible}),
                    ));
                }
                eye_drag = Some((!visible, done, None, eye_drag.is_none() || stepped));
            } else if resp.drag_started()
                && press_origin.is_some_and(|p| eye_r.contains(p))
                && eye_drag.is_none()
            {
                // TW-54: Ziehen ab einem nicht schaltbaren Auge (geteilte Ebene) schaltet nur die
                // überstrichenen Ebenen, Ziel ist der Gegenzustand der Startzeile
                eye_drag = Some((!visible, BTreeSet::new(), None, false));
            } else if resp.drag_started()
                && editable
                && !web
                && let Some(p0) =
                    press_origin.filter(|p| p.x >= tri.right() + 3.0 && p.x < rect.right() - 22.0)
            {
                // re 41 §3.2: ein Zug beginnt nur auf dem Namens-/Miniatur-Teil der Zeile (nicht auf Auge,
                // Schloss oder Aufklapper); die Webebene lässt sich nicht verschieben (re 42 §2)
                start_drag = Some(Drag::Layer { id: l.id, y0: p0.y });
            }
            if let Some((target, done, _, stepped)) = eye_drag.as_mut()
                && editable
                && swept(rect, eye_r)
                && !done.contains(&l.id.0)
            {
                done.insert(l.id.0);
                if visible != *target {
                    // TW-69: ein Verlaufsschritt für den ganzen Zug
                    actions.push(Action::Run(
                        "layer.setVisible".into(),
                        json!({"layer": l.id.0, "visible": *target, "dragContinue": *stepped}),
                    ));
                    *stepped = true;
                }
            }
            menu_rows.push(resp.clone());
            rows.push(RowInfo { kind, rect, locked });
            if is_collapsed {
                continue;
            }
            if let Some(cel) = l.cel(frame) {
                // Unterebenen wie im Original als Ordner mit Aufklapper, ihre Objekte eingerückt darunter
                let mut flat = Vec::new();
                // sublayers of own layers: the document's flag; inside shared or master-page layers
                // (`layer.setExpanded` reaches only the page's own tree): flipped per document
                let sub_open = |e: &Element, sl: &fp_model::Layer| {
                    sl.expanded != (!editable && flipped.contains(&(doc_id.0, sub_key(e.id))))
                };
                flatten(&cel.elements, frame, 0, &sub_open, &mut flat);
                for (depth, idx, e) in flat {
                    let e = &e;
                    let sel = selected_set.contains(&e.id);
                    if let ElementKind::SubLayer(sl) = &e.kind {
                        // current sublayer marked like a current layer (B-9, RE spec 64 §2.5); a
                        // selected sublayer element is highlighted like a selected object
                        let (cur_row, tint_row) = row_marks(sl.id);
                        let hl = sel || cur_row;
                        common::want_width(
                            ui,
                            SUB_INDENT * (depth + 1) as f32 + 78.0 + MIN_NAME_W + 22.0,
                        );
                        let (r2, resp2) = common::row_tinted(ui, 23.0, hl, tint_row);
                        if scroll_to_current && active_layer == Some(sl.id) {
                            ui.scroll_to_rect(r2, None);
                        }
                        let eye2 = Rect::from_min_size(r2.min, vec2(20.0, r2.height()));
                        let lock2 = Rect::from_min_size(
                            pos2(r2.left() + 21.0, r2.top()),
                            vec2(19.0, r2.height()),
                        );
                        ui.painter().vline(
                            r2.left() + 20.5,
                            r2.y_range(),
                            Stroke::new(1.0, c.list_row_line),
                        );
                        ui.painter().vline(
                            r2.left() + 40.5,
                            r2.y_range(),
                            Stroke::new(1.0, c.list_row_line),
                        );
                        // TW-54: Element-Flag und Sichtbarkeit der Unterebene selbst
                        let sub_visible = e.common.visible && sl.is_shown(frame);
                        if sub_visible {
                            glyphs::paint(ui.painter(), eye2.shrink(3.0), Glyph::Eye, &theme, true);
                        }
                        if e.common.locked {
                            glyphs::paint(
                                ui.painter(),
                                lock2.shrink(4.0),
                                Glyph::Lock,
                                &theme,
                                true,
                            );
                        }
                        let x0 = r2.left() + SUB_INDENT * (depth + 1) as f32;
                        let tri2 = Rect::from_center_size(
                            pos2(x0 + 50.0, r2.center().y),
                            vec2(10.0, 10.0),
                        );
                        let open = sub_open(e, sl);
                        paint_triangle(ui, tri2, open, c.icon_fg);
                        glyphs::paint(
                            ui.painter(),
                            Rect::from_center_size(
                                pos2(x0 + 66.0, r2.center().y),
                                vec2(16.0, 14.0),
                            ),
                            Glyph::Folder,
                            &theme,
                            true,
                        );
                        let nr = Rect::from_min_max(
                            pos2(x0 + 78.0, r2.top() + 2.0),
                            pos2(r2.right() - 22.0, r2.bottom() - 2.0),
                        );
                        // Kennzeichen rechts wie bei Ebenen: über Status geteilt bzw. aktiv
                        let mark2 = Rect::from_center_size(
                            pos2(r2.right() - 11.0, r2.center().y),
                            vec2(12.0, 12.0),
                        );
                        if sl.shared_across_frames {
                            glyphs::paint(ui.painter(), mark2, Glyph::Distribute, &theme, true);
                        } else {
                            paint_dot(
                                ui,
                                mark2.center(),
                                sl.id,
                                !fp_engine::docops::layer_has_own_objects(sl, frame),
                            );
                        }
                        // Umbenennen per Doppelklick wie bei Ebenen und Objekten (fixg5 Q-05)
                        let renaming = rename.as_ref().is_some_and(
                            |k| matches!(k, RowKind::Element { id, .. } if *id == e.id),
                        );
                        if renaming {
                            if let Some(res) = common::inline_rename(
                                ui,
                                Id::new(("sub-rename", e.id.0)),
                                nr,
                                &sl.name,
                            ) {
                                if let Some(n) = res.filter(|n| !n.is_empty()) {
                                    actions.push(Action::Run(
                                        "element.setName".into(),
                                        json!({"ids": [e.id.0], "name": n}),
                                    ));
                                }
                                ui.data_mut(|dd| dd.remove::<RowKind>(Id::new(RENAME)));
                            }
                        } else {
                            let font = if hl {
                                theme.bold(theme.fonts.ui)
                            } else {
                                theme.ui_font()
                            };
                            crate::widgets::text_clipped(
                                ui.painter(),
                                pos2(nr.left(), r2.center().y),
                                Align2::LEFT_CENTER,
                                &sl.name,
                                font,
                                if editable { c.text } else { c.text_weak },
                                nr.width(),
                            );
                        }
                        let kind = RowKind::Element {
                            id: e.id,
                            layer: l.id,
                            index: idx,
                            editable,
                        };
                        if resp2.clicked()
                            && let Some(p) = resp2.interact_pointer_pos()
                        {
                            if eye2.contains(p) && editable {
                                if alt {
                                    let cmd = if sub_visible {
                                        "panel.layers.hideAll"
                                    } else {
                                        "panel.layers.showAll"
                                    };
                                    actions.push(Action::Run(cmd.into(), json!({})));
                                } else {
                                    actions.push(Action::Run(
                                        "layer.setVisible".into(),
                                        json!({"layer": sl.id.0, "visible": !sub_visible}),
                                    ));
                                }
                            } else if lock2.contains(p) && editable {
                                actions.push(Action::Run(
                                    "element.setLocked".into(),
                                    json!({"ids": [e.id.0], "locked": !e.common.locked}),
                                ));
                            } else if tri2.expand(3.0).contains(p) {
                                if editable {
                                    actions.push(Action::Run(
                                        "layer.setExpanded".into(),
                                        json!({"layer": sl.id.0, "expanded": !open}),
                                    ));
                                } else {
                                    toggle_flip = Some(sub_key(e.id));
                                }
                            } else if editable
                                && !sl.shared_across_frames
                                && mark2.expand(2.0).contains(p)
                            {
                                // RE spec 65 rule 1: the circle as on a layer row
                                actions.push(Action::Run(
                                    "selection.moveToLayer".into(),
                                    json!({"layer": sl.id.0, "copy": alt}),
                                ));
                            } else if editable {
                                // like a layer row: the sublayer becomes current (new objects land
                                // in it) and its own objects are selected (B-9); Shift/Ctrl as on a
                                // layer row (re 63 §1.3)
                                pending_click = Some((RowKey::Layer(sl.id), mods));
                            }
                        }
                        if (resp2.double_clicked() || resp2.triple_clicked())
                            && editable
                            && !tri2
                                .expand(3.0)
                                .contains(resp2.interact_pointer_pos().unwrap_or(r2.center()))
                        {
                            rename_start = Some(kind.clone());
                        }
                        if resp2.drag_started() && press_origin.is_some_and(|p| eye2.contains(p)) {
                            // TW-54/TW-94: Ziehen ab dem Auge einer Unterebene schaltet wie im Original
                            // (Lauf 10, `panel-layers/04`: Start „Gemeinsam“) die Unterebene beim Drücken
                            // um und die überstrichenen Ebenen auf deren neuen Zustand; die Startzeile ist
                            // damit erledigt und wird beim Loslassen nicht noch einmal geschaltet
                            if eye_drag.is_none() {
                                if editable {
                                    actions.push(Action::Run(
                                        "layer.setVisible".into(),
                                        json!({"layer": sl.id.0, "visible": !sub_visible}),
                                    ));
                                }
                                eye_drag =
                                    Some((!sub_visible, BTreeSet::from([sl.id.0]), None, editable));
                            }
                        } else if resp2.drag_started() && editable && depth == 0 {
                            start_drag = Some(Drag::Element { id: e.id });
                        }
                        // TW-69: überstrichene Unterebenen erhalten denselben Zustand
                        if let Some((target, done, _, stepped)) = eye_drag.as_mut()
                            && editable
                            && swept(r2, eye2)
                            && !done.contains(&sl.id.0)
                        {
                            done.insert(sl.id.0);
                            if sub_visible != *target {
                                actions.push(Action::Run("layer.setVisible".into(), json!({"layer": sl.id.0, "visible": *target, "dragContinue": *stepped})));
                                *stepped = true;
                            }
                        }
                        menu_rows.push(resp2.clone());
                        if depth == 0 {
                            rows.push(RowInfo {
                                kind,
                                rect: r2,
                                locked: false,
                            });
                        }
                        continue;
                    }
                    // BA-099: rows outside the visible part of the list only take their place
                    // (pages with thousands of objects built every row each frame)
                    let next =
                        Rect::from_min_size(ui.cursor().min, vec2(ui.available_width(), erow_h));
                    common::want_width(
                        ui,
                        78.0 + THUMB_INDENT * depth as f32 + tpx + 6.0 + MIN_NAME_W,
                    );
                    let renaming_row = rename
                        .as_ref()
                        .is_some_and(|k| matches!(k, RowKind::Element { id, .. } if *id == e.id));
                    // the open group and mask toggles of this row and the rows below it
                    let mut tree: Vec<TreeItem> = Vec::new();
                    if tree_on {
                        tree_items(e, 0, 0, None, &is_tree_open, &mut tree);
                    }
                    let tc = TreeCtx {
                        st,
                        theme: &theme,
                        doc: &doc,
                        doc_id,
                        doc_gen,
                        tpx,
                        row_h: erow_h,
                        selected: &selected_set,
                        rename: &rename,
                        open: &tree_open,
                        editable,
                        layer: l.id,
                        top: e.id,
                        thumb_x: next.left() + 78.0 + THUMB_INDENT * depth as f32,
                        mods,
                    };
                    if !ui.is_rect_visible(next) && !renaming_row {
                        ui.advance_cursor_after_rect(next);
                        if !tree.is_empty() {
                            let mut out = TreeOut {
                                actions: &mut actions,
                                toggles: &mut tree_toggles,
                                click: &mut tree_click,
                                rename_start: &mut rename_start,
                                menu_rows: &mut menu_rows,
                            };
                            tree_rows(ui, &tc, &mut out, &tree);
                        }
                        if depth == 0 {
                            rows.push(RowInfo {
                                kind: RowKind::Element {
                                    id: e.id,
                                    layer: l.id,
                                    index: idx,
                                    editable,
                                },
                                rect: next,
                                locked: false,
                            });
                        }
                        continue;
                    }
                    // RE spec 70 section 4.3: a selected object frames its own thumbnail blue;
                    // while its mask is being edited (mask thumbnail, Edit Mask) the row is
                    // highlighted as well, the mask thumbnail framed green
                    let obj_sel = sel;
                    let sel = sel
                        || e.element_mask()
                            .is_some_and(|m| mask_selected(&selected_set, m.element));
                    // FB-60: a group whose object is subselected gets a pale yellow row
                    let yellow = tree_on
                        && !sel
                        && !selected_set.is_empty()
                        && fp_engine::docops::has_selected_descendant(e, &|id| {
                            selected_set.contains(&id)
                        });
                    let (r2, resp2) =
                        common::row_fill(ui, erow_h, sel, yellow.then_some(yellow_bg));
                    let eye2 = Rect::from_min_size(r2.min, vec2(20.0, r2.height()));
                    let lock2 = Rect::from_min_size(
                        pos2(r2.left() + 21.0, r2.top()),
                        vec2(19.0, r2.height()),
                    );
                    ui.painter().vline(
                        r2.left() + 20.5,
                        r2.y_range(),
                        Stroke::new(1.0, c.list_row_line),
                    );
                    ui.painter().vline(
                        r2.left() + 40.5,
                        r2.y_range(),
                        Stroke::new(1.0, c.list_row_line),
                    );
                    if e.common.visible {
                        glyphs::paint(
                            ui.painter(),
                            eye2.shrink2(vec2(3.0, 8.0)),
                            Glyph::Eye,
                            &theme,
                            true,
                        );
                    }
                    if e.common.locked {
                        glyphs::paint(
                            ui.painter(),
                            lock2.shrink2(vec2(4.0, 8.0)),
                            Glyph::Lock,
                            &theme,
                            true,
                        );
                    } else if e
                        .element_mask()
                        .is_some_and(|m| mask_selected(&selected_set, m.element))
                    {
                        // mask editing: mask icon in the second column of the row (RE spec 68
                        // section 7)
                        glyphs::paint(
                            ui.painter(),
                            lock2.shrink2(vec2(3.0, 7.0)),
                            Glyph::Mask,
                            &theme,
                            true,
                        );
                    }
                    let thumb = Rect::from_min_size(
                        pos2(
                            r2.left() + 78.0 + THUMB_INDENT * depth as f32,
                            r2.top() + 3.0,
                        ),
                        vec2(tpx, tpx),
                    );
                    // Miniaturen nur für sichtbare Zeilen (FV-8: bei Seiten mit Tausenden Objekten
                    // rechnete und lud jedes Bild sonst ständig Miniaturen nach)
                    if tpx > 0.0 && ui.is_rect_visible(thumb) {
                        crate::widgets::checker(ui.painter(), thumb, 4.0, &theme);
                        let size = (tpx as u32 * 2).max(16);
                        // a masked object: canvas-related, with or without the mask and its
                        // underlay (RE spec 73 §0.1)
                        let tex = if e.element_mask().is_some() {
                            thumbs::masked_object_thumb(ui.ctx(), st, doc_id, &doc, e, size)
                        } else {
                            thumbs::element_thumb(ui.ctx(), st, doc_id, doc_gen, &doc, e, size)
                        };
                        if let Some(tex) = tex {
                            let ts = tex.size_vec2();
                            let s = (thumb.width() / ts.x).min(thumb.height() / ts.y);
                            ui.painter().image(
                                tex.id(),
                                Rect::from_center_size(thumb.center(), ts * s),
                                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                                Color32::WHITE,
                            );
                        }
                        ui.painter().rect_stroke(
                            thumb,
                            0.0,
                            if obj_sel {
                                Stroke::new(2.0, c.focus)
                            } else {
                                Stroke::new(1.0, c.list_border)
                            },
                            StrokeKind::Inside,
                        );
                    }
                    // Maske am Objekt: zweite Miniatur (Maske) rechts daneben wie im Original
                    // re 31 §5.3: Element-Miniatur, Kette, Masken-Miniatur (Einzelelement und Maskengruppe)
                    let mask_thumb = e.element_mask().map(|m| Arc::clone(m.element));
                    let mut mask_hit: Option<(Rect, Rect, fp_model::ElementId)> = None;
                    let thumb = match &mask_thumb {
                        Some(m) if tpx > 0.0 => {
                            let (mt, chain) = mask_thumb_pair(
                                ui,
                                st,
                                &theme,
                                doc_id,
                                &doc,
                                e,
                                m,
                                thumb,
                                &selected_set,
                            );
                            mask_hit = Some((mt, chain, m.id));
                            mt
                        }
                        _ => thumb,
                    };
                    let name = element_label(e);
                    // Umbenennen: unbenannte Objekte beginnen mit leerem Feld (der Typname wird nicht vorbelegt)
                    let own_name = e.common.name.clone().unwrap_or_default();
                    let nr = Rect::from_min_max(
                        pos2(thumb.right() + 6.0, r2.top() + 5.0),
                        pos2(r2.right() - 4.0, r2.bottom() - 5.0),
                    );
                    common::want_width(ui, nr.left() - r2.left() + MIN_NAME_W);
                    let renaming = rename
                        .as_ref()
                        .is_some_and(|k| matches!(k, RowKind::Element { id, .. } if *id == e.id));
                    let mut hits = (None, None);
                    if renaming {
                        if let Some(res) =
                            common::inline_rename(ui, Id::new(("el-rename", e.id.0)), nr, &own_name)
                        {
                            if let Some(n) = res {
                                actions.push(Action::Run(
                                    "element.setName".into(),
                                    json!({"ids": [e.id.0], "name": n}),
                                ));
                            }
                            ui.data_mut(|dd| dd.remove::<RowKind>(Id::new(RENAME)));
                        }
                    } else {
                        let font = if sel {
                            theme.bold(theme.fonts.ui)
                        } else {
                            theme.ui_font()
                        };
                        let toggles = if tree_on {
                            RowToggles {
                                group: group_toggle(e).map(|_| is_tree_open(e.id.0)),
                                mask: e.element_mask().map(|_| is_tree_open(mask_key(e.id))),
                            }
                        } else {
                            RowToggles::default()
                        };
                        hits = name_with_toggles(
                            ui,
                            nr,
                            r2.center().y,
                            &name,
                            font,
                            if editable { c.text } else { c.text_weak },
                            c.icon_fg,
                            toggles,
                        );
                    }
                    let kind = RowKind::Element {
                        id: e.id,
                        layer: l.id,
                        index: idx,
                        editable,
                    };
                    if resp2.clicked()
                        && let Some(p) = resp2.interact_pointer_pos()
                    {
                        if let Some(k) = toggle_at(hits, e.id, p) {
                            // FB-60/61: the group or mask toggle after the name
                            tree_toggles.push(k);
                        } else if let Some((mt, chain, mid)) = mask_hit.filter(|(mt, chain, _)| {
                            editable && (mt.contains(p) || chain.contains(p))
                        }) {
                            // the mask thumbnail works like Edit Mask (RE spec 68 section 7):
                            // the masked element, then the mask-select command (one History row
                            // "Modify Mask", a bitmap mask goes into bitmap editing); the chain
                            // toggles the link
                            if mt.contains(p) {
                                if !selected_set.contains(&mid) {
                                    actions.push(Action::Run(
                                        "select.set".into(),
                                        json!({"ids": [e.id.0]}),
                                    ));
                                    actions.push(Action::Run(
                                        "panel.layers.editMask".into(),
                                        json!({}),
                                    ));
                                }
                            } else if chain.contains(p) {
                                actions.push(Action::Run(
                                    "modify.mask.link".into(),
                                    json!({"ids": [e.id.0]}),
                                ));
                            }
                        } else if eye2.contains(p) && editable {
                            actions.push(Action::Run(
                                "element.setVisible".into(),
                                json!({"ids": [e.id.0], "visible": !e.common.visible}),
                            ));
                        } else if lock2.contains(p) && editable {
                            actions.push(Action::Run(
                                "element.setLocked".into(),
                                json!({"ids": [e.id.0], "locked": !e.common.locked}),
                            ));
                        } else if editable {
                            // resolved after the loop: a Shift range needs the rows below as well
                            pending_click = Some((RowKey::Element(e.id), mods));
                        }
                    }
                    // Q-59 (RE spec 58 §3.2): a right-click on an object row selects the object like a
                    // plain click before the context menu forms its states (Flatten Selection, Add
                    // Mask, Merge Down follow that selection); an object that is already part of the
                    // selection keeps the selection as it is
                    if resp2.secondary_clicked() && editable && !selected_set.contains(&e.id) {
                        actions.push(Action::Run("select.set".into(), json!({"ids": [e.id.0]})));
                        new_anchor = Some(RowKey::Element(e.id));
                    }
                    if (resp2.double_clicked() || resp2.triple_clicked())
                        && editable
                        && resp2
                            .interact_pointer_pos()
                            .is_none_or(|p| toggle_at(hits, e.id, p).is_none())
                    {
                        rename_start = Some(kind.clone());
                    }
                    if resp2.drag_started() && editable && depth == 0 {
                        start_drag = Some(Drag::Element { id: e.id });
                    }
                    menu_rows.push(resp2.clone());
                    // Ziehen/Ablegen nur in der obersten Ebene der Objekte (Index in der Ebene)
                    if depth == 0 {
                        rows.push(RowInfo {
                            kind,
                            rect: r2,
                            locked: false,
                        });
                    }
                    // FB-59 to FB-61: the objects of the open group and mask toggles below
                    if !tree.is_empty() {
                        let mut out = TreeOut {
                            actions: &mut actions,
                            toggles: &mut tree_toggles,
                            click: &mut tree_click,
                            rename_start: &mut rename_start,
                            menu_rows: &mut menu_rows,
                        };
                        tree_rows(ui, &tc, &mut out, &tree);
                    }
                }
            }
        }
    });
    if let Some((row, m)) = pending_click {
        let seq = ObjSeq::build(&layers, frame);
        let (ids, a) = row_click(&seq, &selected, anchor, row, m);
        // a plain click on an unlocked layer or sublayer row also makes it the current layer
        // and selects its own objects in one History row "Layer Change" (RE spec 64 §2.4)
        let ids: Option<Vec<u64>> = ids.map(|v| v.iter().map(|x| x.0).collect());
        match (row, ids) {
            (RowKey::Layer(lid), ids) if m == ClickMods::default() && !seq.row_locked(row) => {
                actions.push(Action::Run(
                    "layer.setActive".into(),
                    json!({"layer": lid.0, "select": ids.unwrap_or_default()}),
                ));
            }
            (_, Some(ids)) => {
                actions.push(Action::Run("select.set".into(), json!({"ids": ids})));
            }
            _ => {}
        }
        if new_anchor.is_none() {
            new_anchor = a;
        }
    }
    if let Some(k) = new_anchor {
        ui.data_mut(|dd| dd.insert_temp(Id::new(ANCHOR), (doc_id.0, k)));
    }
    // FB-60/61: a click on a row of the group tree; rows below a locked object or layer ignore it
    if let Some(tcl) = tree_click
        && !ObjSeq::build(&layers, frame).row_locked(RowKey::Element(tcl.top))
    {
        let owner = match tcl.role {
            TreeRole::Mask { owner, .. } => fp_engine::docops::element_on_page(&doc, page, owner),
            TreeRole::Child => None,
        };
        actions.extend(tree_click_actions(&tcl, &selected, owner.as_deref()));
    }
    if !tree_toggles.is_empty() {
        let mut open = tree_open.clone();
        for k in tree_toggles {
            if !open.remove(&(doc_id.0, k)) {
                open.insert((doc_id.0, k));
            }
        }
        ui.data_mut(|dd| dd.insert_temp(Id::new(TREE_OPEN), open));
    }
    ui.data_mut(|dd| match eye_drag {
        Some(mut v) => {
            v.2 = pointer.map(|p| p.y);
            dd.insert_temp(eye_key, v);
        }
        None => dd.remove::<(bool, BTreeSet<u64>, Option<f32>, bool)>(eye_key),
    });
    for r in &menu_rows {
        common::resource_context_menu(r, st, "6923");
    }
    if let Some(key) = toggle_flip {
        let mut s = flipped.clone();
        if !s.remove(&(doc_id.0, key)) {
            s.insert((doc_id.0, key));
        }
        ui.data_mut(|dd| dd.insert_temp(Id::new(FLIPPED), s));
    }
    if let Some(k) = rename_start {
        ui.data_mut(|dd| dd.insert_temp(Id::new(RENAME), k));
    }
    // Ziehen zum Umsortieren
    if let Some(dg) = start_drag {
        ui.data_mut(|dd| dd.insert_temp(Id::new(DRAG), dg));
    }
    let drag: Option<Drag> = ui.data(|dd| dd.get_temp(Id::new(DRAG)));
    if let Some(dg) = drag {
        let pointer = ui.input(|i| i.pointer.hover_pos());
        let released = ui.input(|i| i.pointer.any_released());
        let target = pointer
            .filter(|p| list_rect.contains(*p) && beyond_threshold(&dg, p.y, 23.0))
            .and_then(|p| drop_target(&rows, p, &dg));
        if let Some((y, act)) = &target {
            ui.painter().hline(
                list_rect.x_range().shrink(2.0),
                *y,
                Stroke::new(2.0, c.focus),
            );
            // Ziehen auf die Mitte einer Ebene: deren Ordnersymbol gelb hervorgehoben (re 40 §2a)
            if let Some(r) = into_highlight(&rows, act) {
                ui.painter().rect_filled(r, 2.0, LAYER_INTO_HIGHLIGHT);
            }
        }
        if released {
            if let Some((_, act)) = target {
                actions.push(act);
            }
            ui.data_mut(|dd| dd.remove::<Drag>(Id::new(DRAG)));
        }
    }
    // Fußleiste
    let frame_text = super::state_name(st, frame);
    let can_delete_layer = active_layer.is_some() && (layer_selected || selected.is_empty());
    let del_action = if !selected.is_empty() && !layer_selected {
        Some(Action::Run("edit.clear".into(), json!({})))
    } else {
        active_layer.map(|l| Action::Run("layer.delete".into(), json!({"layer": l.0})))
    };
    let new_layer = if alt {
        Action::Run("layer.duplicate".into(), json!({}))
    } else {
        // above the current layer on its level, beside a current sublayer (RE spec 64 §1.3)
        Action::Run("layer.add".into(), json!({"sibling": true}))
    };
    let tips: [String; 5] = [
        "panel-layers-new-sublayer",
        "panel-layers-new-layer",
        "panel-layers-new-bitmap",
        "panel-layers-add-mask",
        "panel-layers-delete",
    ]
    .map(common::t);
    let buttons = [
        FooterButton {
            glyph: Glyph::NewSubLayer,
            tip: &tips[0],
            enabled: true,
            action: Some(Action::Run("pi.layer.addSub".into(), json!({}))),
        },
        FooterButton {
            glyph: Glyph::NewLayer,
            tip: &tips[1],
            enabled: true,
            action: Some(new_layer),
        },
        FooterButton {
            glyph: Glyph::Bitmap,
            tip: &tips[2],
            enabled: true,
            action: Some(Action::Run("edit.insert.emptyBitmap".into(), json!({}))),
        },
        FooterButton {
            glyph: Glyph::Mask,
            tip: &tips[3],
            enabled: !selected.is_empty(),
            action: Some(Action::Run("modify.mask.revealAll".into(), json!({}))),
        },
        FooterButton {
            glyph: Glyph::Trash,
            tip: &tips[4],
            enabled: !selected.is_empty() || can_delete_layer,
            action: del_action,
        },
    ];
    if let Some(a) = footer_with_state(ui, st, &frame_text, &buttons) {
        actions.push(a);
    }
    for a in actions {
        st.push(a);
    }
}

/// Frame of the mask thumbnail while the mask is being edited: 2 px #66FF00 in place of the
/// normal 1 px frame, growing 1 px outward (RE spec 73 §0.4 and §6; RE spec 68 section 7).
pub const MASK_EDIT_FRAME: Color32 = Color32::from_rgb(0x66, 0xFF, 0x00);

/// The mask half of the row of a masked object (re 31 section 5.3): the chain icon of a linked
/// mask after the object thumbnail `thumb`, the mask thumbnail (framed green while the mask is
/// edited, a red X while it is disabled). Shared by the object rows and, Firepixel extension
/// FB-78, the rows of masked objects inside the group tree. Returns the mask thumbnail rect and
/// the rect of the chain between the thumbnails.
#[allow(clippy::too_many_arguments)]
fn mask_thumb_pair(
    ui: &Ui,
    st: &AppState,
    theme: &Theme,
    doc_id: fp_engine::DocId,
    doc: &Arc<fp_model::Document>,
    e: &Element,
    m: &Arc<Element>,
    thumb: Rect,
    selected: &std::collections::HashSet<fp_model::ElementId>,
) -> (Rect, Rect) {
    let c = &theme.colors;
    // verknüpfte Maske: Kettensymbol zwischen den Miniaturen wie im Original (N35)
    let linked = matches!(&e.kind, ElementKind::Group(g) if g.mask_flags.linked);
    let gap = if linked { 12.0 } else { 6.0 };
    let mt = Rect::from_min_size(pos2(thumb.right() + gap, thumb.top()), thumb.size());
    let chain = Rect::from_min_max(
        pos2(thumb.right(), thumb.top()),
        pos2(mt.left(), mt.bottom()),
    );
    if linked && ui.is_rect_visible(mt) {
        let cx = thumb.right() + gap / 2.0;
        let cy = thumb.center().y;
        let s = Stroke::new(1.0, c.icon_fg);
        for dy in [-2.5f32, 2.5] {
            let link = Rect::from_center_size(pos2(cx, cy + dy), vec2(4.0, 7.0));
            ui.painter().rect_stroke(link, 2.0, s, StrokeKind::Inside);
        }
    }
    if ui.is_rect_visible(mt) {
        ui.painter().rect_filled(mt, 0.0, Color32::BLACK);
        let size = (thumb.width() as u32 * 2).max(16);
        if let Some(tex) = thumbs::mask_thumb(ui.ctx(), st, doc_id, doc, e, m, size) {
            let ts = tex.size_vec2();
            let s = (mt.width() / ts.x).min(mt.height() / ts.y);
            ui.painter().image(
                tex.id(),
                Rect::from_center_size(mt.center(), ts * s),
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        // a vector mask (path, group) carries the pen badge bottom right, a bitmap mask none
        // (RE spec 73 §0.3)
        if !matches!(m.kind, ElementKind::Bitmap(_)) {
            paint_pen_badge(ui.painter(), mt);
        }
        // selected mask (mask editing, RE spec 68 section 7, RE spec 73 §6): the 2 px green
        // frame covers the normal frame line and 1 px outside it, the image area stays; the same
        // for bitmap and vector masks
        if mask_selected(selected, m) {
            ui.painter().rect_stroke(
                mt.expand(1.0),
                0.0,
                Stroke::new(2.0, MASK_EDIT_FRAME),
                StrokeKind::Inside,
            );
        } else {
            ui.painter()
                .rect_stroke(mt, 0.0, Stroke::new(1.0, c.list_border), StrokeKind::Inside);
        }
        // disabled mask: red X over the mask thumbnail (RE spec 68 section 8, M-10)
        if matches!(&e.kind, ElementKind::Group(g) if !g.mask_flags.enabled) {
            paint_mask_off(ui.painter(), mt);
        }
    }
    (mt, chain)
}

/// Size of the pen badge of a vector mask thumbnail (about 10 × 10 like the reference).
pub const PEN_BADGE: f32 = 10.0;

/// Pen badge bottom right in the mask thumbnail `r` of a vector mask (RE spec 73 §0.3): a white
/// box with a black border and a black pen nib.
fn paint_pen_badge(p: &egui::Painter, r: Rect) {
    let b = Rect::from_min_size(
        r.right_bottom() - vec2(PEN_BADGE + 1.0, PEN_BADGE + 1.0),
        vec2(PEN_BADGE, PEN_BADGE),
    );
    p.rect_filled(b, 0.0, Color32::WHITE);
    p.rect_stroke(b, 0.0, Stroke::new(1.0, Color32::BLACK), StrokeKind::Inside);
    let ink = Stroke::new(1.5, Color32::BLACK);
    let (l, t) = (b.left(), b.top());
    p.line_segment([pos2(l + 2.5, t + 7.5), pos2(l + 7.0, t + 3.0)], ink);
    p.line_segment([pos2(l + 2.5, t + 7.5), pos2(l + 3.0, t + 5.0)], ink);
    p.line_segment([pos2(l + 2.5, t + 7.5), pos2(l + 5.0, t + 7.0)], ink);
}

/// Is the mask `m` (or, for a group mask, one of its descendants) selected?
fn mask_selected(selected: &std::collections::HashSet<fp_model::ElementId>, m: &Element) -> bool {
    if selected.contains(&m.id) {
        return true;
    }
    match &m.kind {
        ElementKind::Group(g) => g.children.iter().any(|c| mask_selected(selected, c)),
        _ => false,
    }
}

/// Red X over the thumbnail of a disabled mask (RE spec 68 section 8).
fn paint_mask_off(p: &egui::Painter, r: Rect) {
    let s = Stroke::new(2.0, Color32::from_rgb(0xE0, 0x20, 0x20));
    let r = r.shrink(2.0);
    p.line_segment([r.left_top(), r.right_bottom()], s);
    p.line_segment([r.right_top(), r.left_bottom()], s);
}

/// Einrückung einer Unterebene (Aufklapper/Ordner) bzw. ihrer Objekt-Miniaturen je Stufe, gemessen am
/// Original (panel-layers/01: Aufklapper 50 → 78 px, Miniatur 82 → 98 px).
const SUB_INDENT: f32 = 28.0;
/// Width a row keeps for the start of its name and its toggles; a deeper indented row asks the
/// list for more width and the list scrolls horizontally (FB-77).
const MIN_NAME_W: f32 = 72.0;
const THUMB_INDENT: f32 = 16.0;

/// Key of a sublayer row in the set of flipped rows (apart from layer ids).
fn sub_key(id: fp_model::ElementId) -> u64 {
    id.0 | (1 << 62)
}

/// Objects of a layer with its open sublayers in order: (depth, index in its own list, object).
/// `open` tells whether a sublayer (element, layer) shows its children.
fn flatten(
    els: &[Arc<Element>],
    frame: usize,
    depth: usize,
    open: &dyn Fn(&Element, &fp_model::Layer) -> bool,
    out: &mut Vec<(usize, usize, Arc<Element>)>,
) {
    for (i, e) in els.iter().enumerate() {
        out.push((depth, i, Arc::clone(e)));
        if let ElementKind::SubLayer(l) = &e.kind
            && open(e, l)
            && let Some(cel) = l.cel(frame)
        {
            flatten(&cel.elements, frame, depth + 1, open, out);
        }
    }
}

fn paint_triangle(ui: &Ui, r: Rect, open: bool, col: Color32) {
    let c = r.center();
    let pts = if open {
        vec![
            pos2(c.x - 4.0, c.y - 2.0),
            pos2(c.x + 4.0, c.y - 2.0),
            pos2(c.x, c.y + 3.0),
        ]
    } else {
        vec![
            pos2(c.x - 2.0, c.y - 4.0),
            pos2(c.x + 3.0, c.y),
            pos2(c.x - 2.0, c.y + 4.0),
        ]
    };
    ui.painter()
        .add(egui::Shape::convex_polygon(pts, col, Stroke::NONE));
}

/// Hervorhebung des Ordnersymbols beim Ziehen einer Ebene „hinein“ (gelb, re 40 §2a).
const LAYER_INTO_HIGHLIGHT: Color32 = Color32::from_rgba_premultiplied(120, 110, 0, 120);

/// Zone einer Ebenenzeile beim Ziehen einer Ebene (re 41 §3.2, `@00ff74a0`, TW-86): obere 6 px = über
/// die Zielebene, untere 6 px = unter die Zielebene, dazwischen = als oberste Unterebene hinein.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LayerDropZone {
    Above,
    Into,
    Below,
}

fn layer_drop_zone(row: Rect, y: f32) -> LayerDropZone {
    if y < row.top() + 6.0 {
        LayerDropZone::Above
    } else if y >= row.bottom() - 6.0 {
        LayerDropZone::Below
    } else {
        LayerDropZone::Into
    }
}

/// Senkrechte Zugschwelle der Ebenenliste (re 41 §3.2): ein Ziel entsteht erst ab
/// `max(5, ⌊¾ Zeilenhöhe⌋)` px senkrecht vom Druckpunkt (Zeile 22 px ⇒ 16).
fn beyond_threshold(dg: &Drag, y: f32, row_h: f32) -> bool {
    match dg {
        Drag::Layer { y0, .. } => (y - y0).abs() >= (row_h * 3.0 / 4.0).floor().max(5.0),
        Drag::Element { .. } => true,
    }
}

/// Rechteck des Ordnersymbols der Zielzeile, wenn `act` eine Ebene hineinschiebt.
fn into_highlight(rows: &[RowInfo], act: &Action) -> Option<Rect> {
    let Action::Run(name, p) = act else {
        return None;
    };
    if name != "layer.move" {
        return None;
    }
    let target = p.get("into")?.as_u64()?;
    let r = rows
        .iter()
        .find(|r| matches!(r.kind, RowKind::Layer { id, .. } if id.0 == target))?;
    Some(Rect::from_center_size(
        pos2(r.rect.left() + 66.0, r.rect.center().y),
        vec2(18.0, 16.0),
    ))
}

/// Zielposition beim Ziehen: (y der Einfügelinie, Befehl).
fn drop_target(rows: &[RowInfo], p: egui::Pos2, dg: &Drag) -> Option<(f32, Action)> {
    let i = rows.iter().position(|r| r.rect.contains(p))?;
    let r = &rows[i];
    let upper = p.y < r.rect.center().y;
    if let (
        Drag::Layer { id, .. },
        RowKind::Layer {
            id: target,
            editable: true,
            web: false,
        },
    ) = (dg, &r.kind)
        && id != target
        && layer_drop_zone(r.rect, p.y) == LayerDropZone::Into
    {
        // re 42 §2: „hinein“ in eine gesperrte Ebene wird verworfen („über“/„unter“ gehen)
        if r.locked {
            return None;
        }
        // Doppellinie unter der Kopfzeile, Ordner gelb (re 40 §2a)
        return Some((
            r.rect.bottom(),
            Action::Run(
                "layer.move".into(),
                json!({"layer": id.0, "index": 0, "into": target.0}),
            ),
        ));
    }
    let upper = match (dg, &r.kind) {
        (Drag::Layer { .. }, RowKind::Layer { .. }) => {
            layer_drop_zone(r.rect, p.y) == LayerDropZone::Above
        }
        _ => upper,
    };
    match (dg, &r.kind) {
        (
            Drag::Element { id },
            RowKind::Element {
                layer,
                index,
                editable: true,
                ..
            },
        ) => {
            if r.kind
                == (RowKind::Element {
                    id: *id,
                    layer: *layer,
                    index: *index,
                    editable: true,
                })
            {
                return None;
            }
            let idx = if upper { *index } else { index + 1 };
            Some((
                if upper { r.rect.top() } else { r.rect.bottom() },
                Action::Run(
                    "pi.element.moveTo".into(),
                    json!({"id": id.0, "layer": layer.0, "index": idx}),
                ),
            ))
        }
        (
            Drag::Element { id },
            RowKind::Layer {
                id: lid,
                editable: true,
                web: false,
            },
        ) => Some((
            r.rect.bottom(),
            Action::Run(
                "pi.element.moveTo".into(),
                json!({"id": id.0, "layer": lid.0, "index": 0}),
            ),
        )),
        (
            Drag::Layer { id, .. },
            RowKind::Layer {
                id: target,
                editable: true,
                ..
            },
        ) if id != target => {
            // Ebenenliste unten → oben; die Anzeige ist oben → unten
            let layer_rows: Vec<LayerId> = rows
                .iter()
                .filter_map(|r| match r.kind {
                    RowKind::Layer { id, .. } => Some(id),
                    _ => None,
                })
                .collect();
            let n = layer_rows.len();
            let pos_display = layer_rows.iter().position(|x| x == target)?;
            let from_display = layer_rows.iter().position(|x| x == id)?;
            let mut to_display = if upper { pos_display } else { pos_display + 1 };
            if from_display < to_display {
                to_display -= 1;
            }
            let index = n.saturating_sub(1).saturating_sub(to_display);
            Some((
                if upper { r.rect.top() } else { r.rect.bottom() },
                Action::Run("layer.move".into(), json!({"layer": id.0, "index": index})),
            ))
        }
        _ => None,
    }
}

/// Fußleiste mit „Status n“-Menüknopf (4286) links.
fn footer_with_state(
    ui: &mut Ui,
    st: &mut AppState,
    text: &str,
    buttons: &[FooterButton<'_>],
) -> Option<Action> {
    let theme = Theme::current(ui.ctx());
    let rect = ui.available_rect_before_wrap();
    // nicht unter die Knöpfe rechts reichen (schmale Bedienfelder)
    let free = rect.width() - 20.0 - 4.0 - 20.0 * buttons.len() as f32 - 4.0;
    let sr = Rect::from_min_size(
        pos2(rect.left() + 20.0, rect.bottom() - common::FOOTER_H + 3.0),
        vec2(free.clamp(16.0, 100.0), 16.0),
    );
    let resp = ui.interact(sr, Id::new("layers-state-btn"), Sense::click());
    if resp.hovered() {
        ui.painter()
            .rect_filled(sr, 2.0, theme.colors.button_hover_bg);
    }
    let mut out = common::footer(ui, 26.0, text, buttons);
    let n = st.session.active().map(|d| d.frame_count()).unwrap_or(1);
    let names: Vec<String> = (0..n).map(|i| super::state_name(st, i)).collect();
    egui::Popup::menu(&resp)
        .frame(crate::menu::menu_frame(&theme))
        .show(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            for (i, nm) in names.iter().enumerate() {
                if fields::list_item(ui, nm, false).clicked() {
                    out = Some(Action::SetFrame(i));
                    ui.close();
                }
            }
        });
    out
}

// ------------------------------------------------------------- group and mask tree (FB-59 to FB-61)

/// Expanded group and mask rows of the group tree (Firepixel extension, FB-59 to FB-61), per
/// document: (document id, key) with key = element id (group contents) or [`mask_key`] (mask
/// objects). Panel state only: kept while the program runs, never in the file or in History.
const TREE_OPEN: &str = "layers-tree-open";

/// Indentation of the rows of the group tree per level (thumbnail and name move right).
pub const TREE_INDENT: f32 = 16.0;

/// Background of a group row while one of its objects is subselected (FB-60, Firepixel colour):
/// pale yellow in the light themes.
pub const GROUP_CHILD_SELECTED_LIGHT: Color32 = Color32::from_rgb(0xFF, 0xF4, 0xB8);
/// The same in the dark themes: a dark olive, so the light row text stays readable.
pub const GROUP_CHILD_SELECTED_DARK: Color32 = Color32::from_rgb(0x5C, 0x52, 0x1E);

/// Background of a group row whose object is subselected in the theme `theme`.
pub fn group_child_selected_bg(theme: &Theme) -> Color32 {
    let t = theme.colors.text;
    let light_text = u32::from(t.r()) + u32::from(t.g()) + u32::from(t.b()) > 3 * 128;
    if light_text {
        GROUP_CHILD_SELECTED_DARK
    } else {
        GROUP_CHILD_SELECTED_LIGHT
    }
}

/// Key of the mask toggle of an object in [`TREE_OPEN`] (apart from the group toggle's key).
fn mask_key(id: fp_model::ElementId) -> u64 {
    id.0 | (1 << 61)
}

/// What a row of the group tree stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TreeRole {
    /// An object listed under a group row (FB-60) or the content of a masked object (FB-80): a
    /// click selects it like the Subselection tool.
    Child,
    /// An object of the mask of `owner` (FB-61): a click selects the mask for editing like a click
    /// on the mask thumbnail; `in_group`: one of the objects of a mask group (then subselected);
    /// `bitmap`: the row "Bitmap mask".
    Mask {
        owner: fp_model::ElementId,
        in_group: bool,
        bitmap: bool,
    },
}

/// A row of the group tree below an object row.
#[derive(Clone, Debug)]
struct TreeItem {
    /// 1 = directly below the object row.
    level: usize,
    el: Arc<Element>,
    role: TreeRole,
    /// The masked object of a "Bitmap mask" row (its mask thumbnail).
    owner: Option<Arc<Element>>,
    /// Bracket kind per tree line (bit `j`: the line of level `j + 1`): set = a mask bracket
    /// (double line, FB-84) for a mask element row and the rows nested below it, clear = the
    /// single line of content and group rows.
    mask_lines: u64,
}

/// The group toggle "›" of a row: the objects of a group. A masked object has none; its mask
/// toggle lists its mask elements and its content (FB-81).
fn group_toggle(e: &Element) -> Option<&[Arc<Element>]> {
    if e.element_mask().is_some() {
        None
    } else {
        fp_engine::docops::tree_children(e)
    }
}

/// The rows of the group tree below the row of `e` (at `level`, tree lines `lines`) in display
/// order, each nested level below its row: the contents of an open group toggle; for an open
/// mask toggle (FB-81, FB-84) first the mask element(s) with the mask bracket, then the content
/// object(s) of the masked object — also the one content of a masked single object — with the
/// normal bracket. `inherit`: the role of the row of `e` (objects inside a mask group stay mask
/// objects).
fn tree_items(
    e: &Arc<Element>,
    level: usize,
    lines: u64,
    inherit: Option<TreeRole>,
    open: &dyn Fn(u64) -> bool,
    out: &mut Vec<TreeItem>,
) {
    let mask_bit = 1u64 << level.min(63);
    let content_lines = lines & !mask_bit;
    // the role of a content row: the inherited mask role inside a mask, else a child
    let content_role = match inherit {
        Some(TreeRole::Mask { owner, .. }) => TreeRole::Mask {
            owner,
            in_group: true,
            bitmap: false,
        },
        _ => TreeRole::Child,
    };
    let push = |out: &mut Vec<TreeItem>, c: &Arc<Element>, role, owner, lines: u64| {
        out.push(TreeItem {
            level: level + 1,
            el: Arc::clone(c),
            role,
            owner,
            mask_lines: lines,
        });
        // a "Bitmap mask" row has no rows of its own
        if !matches!(role, TreeRole::Mask { bitmap: true, .. }) {
            tree_items(c, level + 1, lines, Some(role), open, out);
        }
    };
    if let Some(cs) = group_toggle(e)
        && open(e.id.0)
    {
        for c in cs {
            push(out, c, content_role, None, content_lines);
        }
    }
    if !open(mask_key(e.id)) {
        return;
    }
    if let Some(m) = fp_engine::docops::mask_members(e) {
        let mask = |in_group, bitmap| TreeRole::Mask {
            owner: e.id,
            in_group,
            bitmap,
        };
        let ml = lines | mask_bit;
        match m {
            fp_engine::docops::MaskMembers::Bitmap(x) => {
                push(out, x, mask(false, true), Some(Arc::clone(e)), ml);
            }
            fp_engine::docops::MaskMembers::Single(x) => {
                push(out, x, mask(false, false), None, ml);
            }
            fp_engine::docops::MaskMembers::Group(cs) => {
                for c in cs {
                    push(out, c, mask(true, false), None, ml);
                }
            }
        }
    }
    if let Some(cs) = fp_engine::docops::mask_content(e) {
        for c in cs {
            push(out, c, content_role, None, content_lines);
        }
    }
}

/// A click on the name part of a row of the group tree, resolved after the list is built.
#[derive(Clone, Copy, Debug)]
struct TreeClick {
    id: fp_model::ElementId,
    /// The object row the tree hangs from (its lock decides whether the click counts).
    top: fp_model::ElementId,
    role: TreeRole,
    /// Shift (or Ctrl/Cmd): add or remove the object like Shift+click with the Subselection.
    toggle: bool,
}

/// Selection commands of a click on a row of the group tree (FB-60, FB-61). `selected`: the
/// current selection; `owner`: the masked object of a mask row.
fn tree_click_actions(
    click: &TreeClick,
    selected: &[fp_model::ElementId],
    owner: Option<&Element>,
) -> Vec<Action> {
    let pick = |id: u64, toggle: bool| {
        Action::Run(
            "tool.pick".into(),
            json!({"command": "select.set", "params": {
                "ids": [id],
                "mode": if toggle { "toggle" } else { "replace" },
            }}),
        )
    };
    match click.role {
        // like a Subselection click on the object: Shift adds or removes it, a plain click
        // selects only it (no History row, like clicks on the canvas)
        TreeRole::Child => {
            if click.toggle {
                vec![pick(click.id.0, true)]
            } else if selected == [click.id] {
                Vec::new()
            } else {
                vec![pick(click.id.0, false)]
            }
        }
        TreeRole::Mask {
            owner: oid,
            in_group,
            ..
        } => {
            let editing = owner.is_some_and(|o| {
                fp_engine::docops::mask_has_selected(o, &|id| selected.contains(&id))
            });
            let mut out = Vec::new();
            // the mask thumbnail's way (RE spec 68 section 7): the masked object, then Edit Mask
            if !editing {
                out.push(Action::Run("select.set".into(), json!({"ids": [oid.0]})));
                out.push(Action::Run("panel.layers.editMask".into(), json!({})));
            }
            // an object of a mask group: subselected like on the canvas (E-1)
            if in_group && !(editing && !click.toggle && selected == [click.id]) {
                out.push(pick(click.id.0, click.toggle && editing));
            }
            out
        }
    }
}

/// Paints a chevron "›" (closed) or "⌄" (open) centred at `c` with half size `s`.
fn paint_chevron(p: &egui::Painter, c: Pos2, open: bool, s: f32, stroke: Stroke) {
    let pts = if open {
        [
            c + vec2(-s, -s * 0.5),
            c + vec2(0.0, s * 0.5),
            c + vec2(s, -s * 0.5),
        ]
    } else {
        [
            c + vec2(-s * 0.5, -s),
            c + vec2(s * 0.5, 0.0),
            c + vec2(-s * 0.5, s),
        ]
    };
    p.line_segment([pts[0], pts[1]], stroke);
    p.line_segment([pts[1], pts[2]], stroke);
}

/// Toggles of an object row of the group tree: `Some(open)` where the row has one.
#[derive(Clone, Copy, Debug, Default)]
struct RowToggles {
    group: Option<bool>,
    mask: Option<bool>,
}

/// Width the toggles take after the name.
fn toggles_width(t: RowToggles) -> f32 {
    t.group.map_or(0.0, |_| 14.0) + t.mask.map_or(0.0, |_| 16.0)
}

/// Paints the name of an object row at the left of `nr` and, with the group tree on, its
/// toggles right after the name: "›"/"⌄" for the group contents, a circled arrow for the
/// objects of the mask (FB-60, FB-61). Returns the hit rects of the group and the mask toggle.
#[allow(clippy::too_many_arguments)]
fn name_with_toggles(
    ui: &Ui,
    nr: Rect,
    cy: f32,
    name: &str,
    font: egui::FontId,
    col: Color32,
    icon: Color32,
    t: RowToggles,
) -> (Option<Rect>, Option<Rect>) {
    let tw = toggles_width(t);
    let text = crate::widgets::text_clipped(
        ui.painter(),
        pos2(nr.left(), cy),
        Align2::LEFT_CENTER,
        name,
        font,
        col,
        (nr.width() - tw).max(8.0),
    );
    if tw == 0.0 {
        return (None, None);
    }
    let mut x = text.right() + 4.0;
    let group = t.group.map(|open| {
        let r = Rect::from_center_size(pos2(x + 5.0, cy), vec2(12.0, 16.0));
        paint_chevron(ui.painter(), r.center(), open, 3.5, Stroke::new(1.5, icon));
        x += 14.0;
        r
    });
    let mask = t.mask.map(|open| {
        let r = Rect::from_center_size(pos2(x + 7.0, cy), vec2(14.0, 16.0));
        ui.painter()
            .circle_stroke(r.center(), 5.5, Stroke::new(1.0, icon));
        paint_chevron(ui.painter(), r.center(), open, 2.2, Stroke::new(1.2, icon));
        r
    });
    (group, mask)
}

/// Which toggle of `hits` (group, mask) lies under `p`: its key in [`TREE_OPEN`].
fn toggle_at(hits: (Option<Rect>, Option<Rect>), id: fp_model::ElementId, p: Pos2) -> Option<u64> {
    let on = |r: Option<Rect>| r.is_some_and(|r| r.expand(2.0).contains(p));
    if on(hits.0) {
        Some(id.0)
    } else if on(hits.1) {
        Some(mask_key(id))
    } else {
        None
    }
}

/// What the rows of the group tree need from the panel.
struct TreeCtx<'a> {
    st: &'a AppState,
    theme: &'a Theme,
    doc: &'a Arc<fp_model::Document>,
    doc_id: fp_engine::DocId,
    doc_gen: u64,
    tpx: f32,
    row_h: f32,
    selected: &'a std::collections::HashSet<fp_model::ElementId>,
    rename: &'a Option<RowKind>,
    open: &'a BTreeSet<(u64, u64)>,
    editable: bool,
    layer: LayerId,
    /// The object row the tree hangs from and the left edge of its thumbnail.
    top: fp_model::ElementId,
    thumb_x: f32,
    mods: ClickMods,
}

/// What the rows of the group tree hand back to the panel.
struct TreeOut<'a> {
    actions: &'a mut Vec<Action>,
    toggles: &'a mut Vec<u64>,
    click: &'a mut Option<TreeClick>,
    rename_start: &'a mut Option<RowKind>,
    menu_rows: &'a mut Vec<egui::Response>,
}

/// Paints the rows of the group tree `items` below an object row (FB-59 to FB-61): indented one
/// step per level with a thin line at the left (a double line for mask elements, FB-84), eye
/// and lock (objects of groups), thumbnail (with the mask thumbnail of a masked object, FB-78),
/// name and the row's own toggles. Rows outside the visible part only take their place
/// (BA-099). The rows are no drag sources and no drop targets.
fn tree_rows(ui: &mut Ui, tc: &TreeCtx<'_>, out: &mut TreeOut<'_>, items: &[TreeItem]) {
    let c = &tc.theme.colors;
    let is_open = |k: u64| tc.open.contains(&(tc.doc_id.0, k));
    for it in items {
        let e = &it.el;
        let next = Rect::from_min_size(ui.cursor().min, vec2(ui.available_width(), tc.row_h));
        common::want_width(
            ui,
            tc.thumb_x - next.left() + TREE_INDENT * it.level as f32 + tc.tpx + 6.0 + MIN_NAME_W,
        );
        let renaming = tc
            .rename
            .as_ref()
            .is_some_and(|k| matches!(k, RowKind::Element { id, .. } if *id == e.id));
        if !ui.is_rect_visible(next) && !renaming {
            ui.advance_cursor_after_rect(next);
            continue;
        }
        // like an object row (RE spec 70 section 4.3): the own thumbnail framed blue when the
        // object is selected, the row highlighted while its mask is edited as well (FB-78)
        let obj_sel = tc.selected.contains(&e.id);
        let sel = obj_sel
            || e.element_mask()
                .is_some_and(|m| mask_selected(tc.selected, m.element));
        let yellow = !sel
            && !tc.selected.is_empty()
            && fp_engine::docops::has_selected_descendant(e, &|id| tc.selected.contains(&id));
        let (r, resp) = common::row_fill(
            ui,
            tc.row_h,
            sel,
            yellow.then(|| group_child_selected_bg(tc.theme)),
        );
        let eye = Rect::from_min_size(r.min, vec2(20.0, r.height()));
        let lock = Rect::from_min_size(pos2(r.left() + 21.0, r.top()), vec2(19.0, r.height()));
        for x in [20.5, 40.5] {
            ui.painter()
                .vline(r.left() + x, r.y_range(), Stroke::new(1.0, c.list_row_line));
        }
        let child = it.role == TreeRole::Child;
        if child && e.common.visible {
            glyphs::paint(
                ui.painter(),
                eye.shrink2(vec2(3.0, 8.0)),
                Glyph::Eye,
                tc.theme,
                true,
            );
        }
        if child && e.common.locked {
            glyphs::paint(
                ui.painter(),
                lock.shrink2(vec2(4.0, 8.0)),
                Glyph::Lock,
                tc.theme,
                true,
            );
        }
        // the tree line: one thin vertical line per level at the left of the indented part; a
        // mask bracket is a double line (FB-84)
        for j in 0..it.level {
            let x = tc.thumb_x + TREE_INDENT * j as f32 + 6.5;
            let st = Stroke::new(1.0, c.list_border);
            if it.mask_lines & (1u64 << j.min(63)) != 0 {
                ui.painter().vline(x - 1.5, r.y_range(), st);
                ui.painter().vline(x + 1.5, r.y_range(), st);
            } else {
                ui.painter().vline(x, r.y_range(), st);
            }
        }
        let thumb = Rect::from_min_size(
            pos2(tc.thumb_x + TREE_INDENT * it.level as f32, r.top() + 3.0),
            vec2(tc.tpx, tc.tpx),
        );
        let bitmap_mask = matches!(it.role, TreeRole::Mask { bitmap: true, .. });
        if tc.tpx > 0.0 && ui.is_rect_visible(thumb) {
            let size = (tc.tpx as u32 * 2).max(16);
            let tex = match (&it.owner, bitmap_mask) {
                (Some(o), true) => {
                    ui.painter().rect_filled(thumb, 0.0, Color32::BLACK);
                    thumbs::mask_thumb(ui.ctx(), tc.st, tc.doc_id, tc.doc, o, e, size)
                }
                _ => {
                    crate::widgets::checker(ui.painter(), thumb, 4.0, tc.theme);
                    if e.element_mask().is_some() {
                        thumbs::masked_object_thumb(ui.ctx(), tc.st, tc.doc_id, tc.doc, e, size)
                    } else {
                        thumbs::element_thumb(
                            ui.ctx(),
                            tc.st,
                            tc.doc_id,
                            tc.doc_gen,
                            tc.doc,
                            e,
                            size,
                        )
                    }
                }
            };
            if let Some(tex) = tex {
                let ts = tex.size_vec2();
                let s = (thumb.width() / ts.x).min(thumb.height() / ts.y);
                ui.painter().image(
                    tex.id(),
                    Rect::from_center_size(thumb.center(), ts * s),
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            ui.painter().rect_stroke(
                thumb,
                0.0,
                if obj_sel {
                    Stroke::new(2.0, c.focus)
                } else {
                    Stroke::new(1.0, c.list_border)
                },
                StrokeKind::Inside,
            );
        }
        // FB-78: a masked object shows the mask half of an object row (chain, mask thumbnail)
        let mut mask_hit: Option<(Rect, Rect, fp_model::ElementId)> = None;
        let thumb = match e.element_mask() {
            Some(m) if tc.tpx > 0.0 && !bitmap_mask => {
                let (mt, chain) = mask_thumb_pair(
                    ui,
                    tc.st,
                    tc.theme,
                    tc.doc_id,
                    tc.doc,
                    e,
                    m.element,
                    thumb,
                    tc.selected,
                );
                mask_hit = Some((mt, chain, m.element.id));
                mt
            }
            _ => thumb,
        };
        let nr = Rect::from_min_max(
            pos2(thumb.right() + 6.0, r.top() + 5.0),
            pos2(r.right() - 4.0, r.bottom() - 5.0),
        );
        common::want_width(ui, nr.left() - r.left() + MIN_NAME_W);
        let toggles = RowToggles {
            group: group_toggle(e).map(|_| is_open(e.id.0)),
            mask: e.element_mask().map(|_| is_open(mask_key(e.id))),
        };
        let mut hits = (None, None);
        if renaming {
            let own = e.common.name.clone().unwrap_or_default();
            if let Some(res) = common::inline_rename(ui, Id::new(("tree-rename", e.id.0)), nr, &own)
            {
                if let Some(n) = res {
                    out.actions.push(Action::Run(
                        "element.setName".into(),
                        json!({"ids": [e.id.0], "name": n}),
                    ));
                }
                ui.data_mut(|dd| dd.remove::<RowKind>(Id::new(RENAME)));
            }
        } else {
            let name = if bitmap_mask && e.common.name.as_deref().is_none_or(str::is_empty) {
                common::t("panel-layers-kind-bitmap-mask")
            } else {
                element_label(e)
            };
            let font = if sel {
                tc.theme.bold(tc.theme.fonts.ui)
            } else {
                tc.theme.ui_font()
            };
            hits = name_with_toggles(
                ui,
                nr,
                r.center().y,
                &name,
                font,
                if tc.editable { c.text } else { c.text_weak },
                c.icon_fg,
                toggles,
            );
        }
        if resp.clicked()
            && let Some(p) = resp.interact_pointer_pos()
        {
            let row_locked = child && e.common.locked;
            if let Some(k) = toggle_at(hits, e.id, p) {
                out.toggles.push(k);
            } else if let Some((mt, _, mid)) = mask_hit.filter(|(mt, chain, _)| {
                tc.editable && !row_locked && (mt.contains(p) || chain.contains(p))
            }) {
                // FB-78: the mask thumbnail edits the mask like on an object row (the masked
                // object, then Edit Mask), the chain toggles the link
                if mt.contains(p) {
                    *out.click = Some(TreeClick {
                        id: mid,
                        top: tc.top,
                        role: TreeRole::Mask {
                            owner: e.id,
                            in_group: false,
                            bitmap: false,
                        },
                        toggle: false,
                    });
                } else {
                    out.actions.push(Action::Run(
                        "modify.mask.link".into(),
                        json!({"ids": [e.id.0]}),
                    ));
                }
            } else if child && eye.contains(p) && tc.editable {
                out.actions.push(Action::Run(
                    "element.setVisible".into(),
                    json!({"ids": [e.id.0], "visible": !e.common.visible}),
                ));
            } else if child && lock.contains(p) && tc.editable {
                out.actions.push(Action::Run(
                    "element.setLocked".into(),
                    json!({"ids": [e.id.0], "locked": !e.common.locked}),
                ));
            } else if tc.editable && p.x > lock.right() && !row_locked {
                *out.click = Some(TreeClick {
                    id: e.id,
                    top: tc.top,
                    role: it.role,
                    toggle: tc.mods.range || tc.mods.toggle,
                });
            }
        }
        // a right-click on an object of a group selects it like a plain click before the
        // context menu forms its states (Q-59 for object rows)
        if resp.secondary_clicked() && tc.editable && child && !sel && !e.common.locked {
            *out.click = Some(TreeClick {
                id: e.id,
                top: tc.top,
                role: it.role,
                toggle: false,
            });
        }
        if (resp.double_clicked() || resp.triple_clicked())
            && tc.editable
            && resp
                .interact_pointer_pos()
                .is_none_or(|p| toggle_at(hits, e.id, p).is_none() && p.x > lock.right())
        {
            *out.rename_start = Some(RowKind::Element {
                id: e.id,
                layer: tc.layer,
                index: 0,
                editable: tc.editable,
            });
        }
        out.menu_rows.push(resp);
    }
}

/// Objekte, die ein Klick auf eine (Unter-)Ebenenzeile wählt: wie im Original nur die eigenen,
/// nicht gesperrten Objekte der Ebene, ohne Unterebenen und deren Objekte (re 28 S3: Klick auf
/// „Ebene 1“ mit Unterebene „U“ wählt nur R1).
pub fn own_objects(elements: &[std::sync::Arc<fp_model::Element>]) -> Vec<fp_model::ElementId> {
    elements
        .iter()
        .filter(|e| !e.common.locked && !matches!(e.kind, ElementKind::SubLayer(_)))
        .map(|e| e.id)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sub(ids: &mut fp_model::IdGen, name: &str, children: Vec<Arc<Element>>) -> Arc<Element> {
        let mut l = fp_model::Layer::new(ids, name, LayerKind::Normal, 1);
        if let Some(c) = l.cels.first_mut() {
            Arc::make_mut(c).elements = children;
        }
        Arc::new(Element::new(
            ids.element(),
            ElementKind::SubLayer(Arc::new(l)),
        ))
    }

    #[test]
    fn t9_layer_drop_zones_thirds() {
        // re 40 §2a: Zeile 250…272 – 254 oben (über), 261 Mitte (hinein), 269 unten (unter)
        let row = Rect::from_min_max(pos2(0.0, 250.0), pos2(200.0, 272.0));
        assert_eq!(layer_drop_zone(row, 254.0), LayerDropZone::Above);
        assert_eq!(layer_drop_zone(row, 261.0), LayerDropZone::Into);
        assert_eq!(layer_drop_zone(row, 269.0), LayerDropZone::Below);
        let rows = vec![
            RowInfo {
                kind: RowKind::Layer {
                    id: LayerId(3),
                    editable: true,
                    web: false,
                },
                rect: row,
                locked: false,
            },
            RowInfo {
                kind: RowKind::Layer {
                    id: LayerId(2),
                    editable: true,
                    web: false,
                },
                rect: Rect::from_min_max(pos2(0.0, 272.0), pos2(200.0, 294.0)),
                locked: false,
            },
            RowInfo {
                kind: RowKind::Layer {
                    id: LayerId(1),
                    editable: true,
                    web: false,
                },
                rect: Rect::from_min_max(pos2(0.0, 346.0), pos2(200.0, 368.0)),
                locked: false,
            },
        ];
        let dg = Drag::Layer {
            id: LayerId(1),
            y0: 357.0,
        };
        let run = |y: f32| match drop_target(&rows, pos2(100.0, y), &dg) {
            Some((_, Action::Run(n, p))) => (n, p),
            other => panic!("{other:?}"),
        };
        // oben: über Ebene 3 = oberste Stelle (Index 2 von 0…2)
        assert_eq!(run(254.0).1, json!({"layer": 1, "index": 2}));
        // Mitte: als Unterebene hinein, Ordner hervorgehoben
        let (n, p) = run(261.0);
        assert_eq!(
            (n.as_str(), p.clone()),
            ("layer.move", json!({"layer": 1, "index": 0, "into": 3}))
        );
        assert!(into_highlight(&rows, &Action::Run(n, p)).is_some());
        // unten: direkt unter Ebene 3
        assert_eq!(run(269.0).1, json!({"layer": 1, "index": 1}));
    }

    /// T-10 (re 41 §3, re 42 §2): Zonen 6 px oben/unten, senkrechte Schwelle max(5, ¾ Zeile),
    /// „hinein“ in eine gesperrte Ebene verworfen, „über“/„unter“ gehen.
    #[test]
    fn t10_layer_drag_zones_threshold_locked() {
        let row = Rect::from_min_max(pos2(0.0, 250.0), pos2(200.0, 272.0));
        assert_eq!(layer_drop_zone(row, 250.0), LayerDropZone::Above);
        assert_eq!(layer_drop_zone(row, 255.9), LayerDropZone::Above);
        assert_eq!(layer_drop_zone(row, 256.0), LayerDropZone::Into);
        assert_eq!(layer_drop_zone(row, 265.9), LayerDropZone::Into);
        assert_eq!(layer_drop_zone(row, 266.0), LayerDropZone::Below);
        // Zeile 22 ⇒ 16, Zeile 23 ⇒ 17, kleine Zeilen mindestens 5
        let dg = Drag::Layer {
            id: LayerId(1),
            y0: 100.0,
        };
        assert!(
            !beyond_threshold(&dg, 115.9, 22.0)
                && beyond_threshold(&dg, 116.0, 22.0)
                && beyond_threshold(&dg, 84.0, 22.0)
        );
        assert!(!beyond_threshold(&dg, 116.0, 23.0) && beyond_threshold(&dg, 117.0, 23.0));
        assert!(!beyond_threshold(&dg, 104.0, 4.0) && beyond_threshold(&dg, 105.0, 4.0));
        let rows = vec![
            RowInfo {
                kind: RowKind::Layer {
                    id: LayerId(3),
                    editable: true,
                    web: false,
                },
                rect: row,
                locked: true,
            },
            RowInfo {
                kind: RowKind::Layer {
                    id: LayerId(1),
                    editable: true,
                    web: false,
                },
                rect: Rect::from_min_max(pos2(0.0, 346.0), pos2(200.0, 368.0)),
                locked: false,
            },
        ];
        let dg = Drag::Layer {
            id: LayerId(1),
            y0: 357.0,
        };
        assert!(
            drop_target(&rows, pos2(100.0, 261.0), &dg).is_none(),
            "into a locked layer is discarded"
        );
        assert!(
            matches!(drop_target(&rows, pos2(100.0, 254.0), &dg), Some((_, Action::Run(n, p))) if n == "layer.move" && p == json!({"layer": 1, "index": 1}))
        );
        assert!(
            matches!(drop_target(&rows, pos2(100.0, 269.0), &dg), Some((_, Action::Run(n, p))) if n == "layer.move" && p == json!({"layer": 1, "index": 0}))
        );
    }

    /// Unterebenen als Ordner mit ihren Objekten eingerückt darunter; zugeklappt ohne Kinder (FIX-G5, Q-05).
    #[test]
    fn sublayers_flatten_with_depth_and_collapse() {
        let mut ids = fp_model::IdGen::default();
        let inner = sub(&mut ids, "Pfad", Vec::new());
        let a = sub(&mut ids, "Gemeinsam", vec![Arc::clone(&inner)]);
        let b = sub(&mut ids, "Unterebene", Vec::new());
        let els = vec![Arc::clone(&a), Arc::clone(&b)];
        let mut out = Vec::new();
        flatten(&els, 0, 0, &|_, l| l.expanded, &mut out);
        let got: Vec<(usize, usize, u64)> = out.iter().map(|(d, i, e)| (*d, *i, e.id.0)).collect();
        assert_eq!(
            got,
            vec![(0, 0, a.id.0), (1, 0, inner.id.0), (0, 1, b.id.0)]
        );
        let mut out = Vec::new();
        flatten(&els, 0, 0, &|e, l| l.expanded && e.id != a.id, &mut out);
        assert_eq!(
            out.iter().map(|(d, _, e)| (*d, e.id.0)).collect::<Vec<_>>(),
            vec![(0, a.id.0), (0, b.id.0)]
        );
    }

    fn eid(n: u64) -> fp_model::ElementId {
        fp_model::ElementId(n)
    }

    /// Measured list (RE spec 61 §3.1): web layer, Layer 1, O4…O1 (ids 4…1).
    fn measured_seq() -> ObjSeq {
        ObjSeq {
            items: [4, 3, 2, 1]
                .into_iter()
                .map(|n| SeqItem {
                    id: eid(n),
                    owner: LayerId(101),
                    locked: false,
                })
                .collect(),
            locked_layers: BTreeSet::new(),
        }
    }

    const CLICK: ClickMods = ClickMods {
        range: false,
        toggle: false,
    };
    const SHIFT: ClickMods = ClickMods {
        range: true,
        toggle: false,
    };
    const CTRL: ClickMods = ClickMods {
        range: false,
        toggle: true,
    };
    const CTRL_SHIFT: ClickMods = ClickMods {
        range: true,
        toggle: true,
    };

    /// Replays a click sequence like the panel does (selection and anchor carried over).
    fn replay(steps: &[(u64, ClickMods)]) -> Vec<Vec<u64>> {
        let seq = measured_seq();
        let (mut sel, mut anchor): (Vec<fp_model::ElementId>, Option<RowKey>) = (Vec::new(), None);
        let mut out = Vec::new();
        for (n, m) in steps {
            let (ids, na) = row_click(&seq, &sel, anchor, RowKey::Element(eid(*n)), *m);
            if let Some(ids) = ids {
                sel = ids;
            }
            anchor = na.or(anchor);
            out.push(sel.iter().map(|x| x.0).collect());
        }
        out
    }

    /// FB-30 / TW-208: the measured sequences of RE spec 61 §3.2.
    #[test]
    fn fb30_row_click_rules_as_measured() {
        let got = replay(&[(4, CLICK), (1, SHIFT), (2, CTRL), (2, CTRL), (3, SHIFT)]);
        assert_eq!(
            got,
            vec![
                vec![4],
                vec![4, 3, 2, 1],
                vec![4, 3, 1],
                vec![4, 3, 2, 1],
                vec![4, 3]
            ]
        );
        assert_eq!(
            replay(&[(1, CLICK), (4, CTRL), (2, SHIFT)]),
            vec![vec![1], vec![4, 1], vec![2, 1]]
        );
        assert_eq!(
            replay(&[(1, CLICK), (4, CTRL), (2, CTRL_SHIFT)]),
            vec![vec![1], vec![4, 1], vec![2, 1]],
            "Ctrl+Shift behaves like Shift"
        );
        assert_eq!(
            replay(&[(1, CLICK), (3, CTRL_SHIFT)]),
            vec![vec![1], vec![3, 2, 1]]
        );
        // all four selected, a click on O3 keeps them, Shift+click O3 ranges from the anchor O1
        assert_eq!(
            replay(&[(1, CLICK), (4, SHIFT), (3, CLICK), (3, SHIFT)]),
            vec![vec![1], vec![4, 3, 2, 1], vec![4, 3, 2, 1], vec![3, 2, 1]]
        );
        assert_eq!(
            replay(&[(3, CLICK), (3, SHIFT), (3, CTRL)]),
            vec![vec![3], vec![3], vec![]]
        );
    }

    /// The six-object document of RE spec 63 §1.1, built with commands: "L1" (bottom) with e, f,
    /// "L2" with c, d, "L3" (top) with a, b; in each layer the first object above the second.
    /// Variants of L2: normal, hidden, locked, collapsed, shared (across states), sub (sublayer
    /// "U2" with g, h at the top of L2), sublocked (sub, L2 locked). Returns the object order and
    /// the row keys by name.
    fn doc3(variant: &str) -> (ObjSeq, Vec<(String, RowKey)>) {
        fn ex(s: &mut fp_engine::Session, c: &str, p: serde_json::Value) -> serde_json::Value {
            s.execute(c, p).unwrap_or_else(|e| panic!("{c}: {e}")).value
        }
        let mut s = fp_engine::Session::default();
        ex(
            &mut s,
            "file.new",
            json!({"width": 700, "height": 400, "background": "#ffffff"}),
        );
        let mut keys: Vec<(String, RowKey)> = Vec::new();
        let mut rect = |s: &mut fp_engine::Session, name: &str, x: f64| {
            let v = ex(
                s,
                "draw.rectangle",
                json!({"rect": [x, 20.0, x + 60.0, 80.0]}),
            );
            let id = v["id"].as_u64().expect("id");
            ex(s, "element.setName", json!({"ids": [id], "name": name}));
            keys.push((name.into(), RowKey::Element(eid(id))));
        };
        let active = |s: &fp_engine::Session| s.active().and_then(|d| d.layer).expect("layer");
        let l1 = active(&s);
        ex(&mut s, "layer.rename", json!({"layer": l1.0, "name": "L1"}));
        rect(&mut s, "f", 20.0);
        rect(&mut s, "e", 100.0);
        let l2 = LayerId(
            ex(&mut s, "layer.add", json!({"name": "L2"}))["layer"]
                .as_u64()
                .expect("L2"),
        );
        rect(&mut s, "d", 180.0);
        rect(&mut s, "c", 260.0);
        let l3 = LayerId(
            ex(&mut s, "layer.add", json!({"name": "L3"}))["layer"]
                .as_u64()
                .expect("L3"),
        );
        rect(&mut s, "b", 340.0);
        rect(&mut s, "a", 420.0);
        let mut layer_keys = vec![("L1", l1), ("L2", l2), ("L3", l3)];
        if variant.starts_with("sub") {
            ex(&mut s, "layer.setActive", json!({"layer": l2.0}));
            ex(&mut s, "pi.layer.addSub", json!({"name": "U2"}));
            let u2 = active(&s);
            rect(&mut s, "h", 500.0);
            rect(&mut s, "g", 580.0);
            layer_keys.push(("U2", u2));
        }
        match variant {
            "hidden" => {
                ex(
                    &mut s,
                    "layer.setVisible",
                    json!({"layer": l2.0, "visible": false}),
                );
            }
            "locked" | "sublocked" => {
                ex(
                    &mut s,
                    "layer.setLocked",
                    json!({"layer": l2.0, "locked": true}),
                );
            }
            "collapsed" => {
                ex(
                    &mut s,
                    "layer.setExpanded",
                    json!({"layer": l2.0, "expanded": false}),
                );
            }
            "shared" => {
                ex(
                    &mut s,
                    "layer.setShared",
                    json!({"layer": l2.0, "shared": true}),
                );
            }
            _ => {}
        }
        for (n, l) in layer_keys {
            keys.push((n.into(), RowKey::Layer(l)));
        }
        let d = s.active().expect("doc");
        (ObjSeq::build(&d.doc.resolved_layers(d.page), d.frame), keys)
    }

    /// Replays a step list on a `doc3` variant and returns the selection after each step as the
    /// object names in panel order. Steps: `x` click on row x, `+x` Shift, `^x` Ctrl, `=x` canvas
    /// click on x (`=` alone: canvas click into empty space), `=+x` Shift+canvas click on x.
    fn replay3(variant: &str, steps: &str) -> Vec<String> {
        let (seq, keys) = doc3(variant);
        let key = |n: &str| {
            keys.iter()
                .find(|(k, _)| k == n)
                .map(|(_, r)| *r)
                .unwrap_or_else(|| panic!("row {n}"))
        };
        let el = |n: &str| match key(n) {
            RowKey::Element(id) => id,
            RowKey::Layer(_) => panic!("{n} is a layer"),
        };
        let name = |id: &fp_model::ElementId| {
            keys.iter()
                .find(|(_, r)| *r == RowKey::Element(*id))
                .map(|(k, _)| k.clone())
                .unwrap_or_default()
        };
        let (mut sel, mut anchor): (Vec<fp_model::ElementId>, Option<RowKey>) = (Vec::new(), None);
        let mut out = Vec::new();
        for st in steps.split_whitespace() {
            if let Some(rest) = st.strip_prefix('=') {
                // a canvas selection change does not touch the panel's anchor
                if let Some(x) = rest.strip_prefix('+') {
                    let id = el(x);
                    if sel.contains(&id) {
                        sel.retain(|y| *y != id);
                    } else {
                        sel.push(id);
                    }
                } else if rest.is_empty() {
                    sel.clear();
                } else {
                    sel = vec![el(rest)];
                }
            } else {
                let (m, n) = match st.as_bytes()[0] {
                    b'+' => (SHIFT, &st[1..]),
                    b'^' => (CTRL, &st[1..]),
                    _ => (CLICK, st),
                };
                let (ids, na) = row_click(&seq, &sel, anchor, key(n), m);
                if let Some(ids) = ids {
                    sel = ids;
                }
                anchor = na.or(anchor);
            }
            let mut names: Vec<String> = seq.sorted(sel.clone()).iter().map(name).collect();
            names.retain(|n| !n.is_empty());
            out.push(names.join(" "));
        }
        out
    }

    /// TW-208 rest: every measured sequence of RE spec 63 §1.2 (`fw.selection` after each step).
    #[test]
    fn layers_range_over_document_order_as_measured() {
        let all = "a b c d e f";
        let cases: &[(&str, &str, &str, &[&str])] = &[
            ("V0", "normal", "a +f", &["a", all]),
            ("T3", "normal", "f +a", &["f", all]),
            ("T4", "normal", "c +a +e", &["c", "a b c", "c d e"]),
            ("V1", "hidden", "a +f", &["a", all]),
            ("T11", "hidden", "a +d", &["a", "a b c d"]),
            ("T18", "hidden", "c", &["c"]),
            ("V2", "locked", "a +f", &["a", "a b e f"]),
            ("T15", "locked", "e +a", &["e", "a b e"]),
            ("T1", "locked", "a +c", &["a", "a"]),
            ("T13", "locked", "a ^c", &["a", "a"]),
            ("T14", "locked", "a +L2", &["a", "a"]),
            ("T2", "locked", "c +f", &["", "f"]),
            ("V3", "collapsed", "a +f", &["a", all]),
            ("T17", "collapsed", "a +L2", &["a", "a b c d"]),
            ("V4", "shared", "a +f", &["a", all]),
            ("V5", "sub", "a +f", &["a", "a b g h c d e f"]),
            ("T16", "sublocked", "a +f", &["a", "a b e f"]),
            ("T5", "normal", "a +L1", &["a", all]),
            ("T6", "normal", "a +L2", &["a", "a b c d"]),
            ("T21", "sub", "a +L2", &["a", "a b g h c d"]),
            ("T21b", "sub", "f +L2", &["f", "c d e f"]),
            ("T7", "normal", "L2 +f", &["c d", "c d e f"]),
            ("T12", "normal", "L2 +a", &["c d", "a b c d"]),
            (
                "T8",
                "normal",
                "a +f ^c ^L2",
                &["a", all, "a b d e f", "a b c e f"],
            ),
            ("T9", "normal", "a +L1 ^e", &["a", all, "a b c d f"]),
            ("T10", "normal", "a ^e +c", &["a", "a e", "a b c"]),
            ("T19", "normal", "a =e +c", &["a", "e", "c d e"]),
            ("T20", "normal", "a =+e +c", &["a", "a e", "a b c"]),
            ("T22", "normal", "a =e =+c +f", &["a", "e", "c e", "e f"]),
            ("T23", "normal", "e =+b +c", &["e", "b e", "c d e"]),
            ("T24", "normal", "a =+e ^c +f", &["a", "a e", "a c e", all]),
            ("T26", "normal", "e ^c ^e +a", &["e", "c e", "c", "a b c"]),
            (
                "T29",
                "normal",
                "e ^c ^a ^e +f",
                &["e", "c e", "a c e", "a c", "c d e f"],
            ),
            (
                "T29b",
                "normal",
                "e ^c ^a ^e +d",
                &["e", "c e", "a c e", "a c", "c d"],
            ),
            ("T25", "normal", "a = +c", &["a", "", "c d e f"]),
            ("T27", "normal", "a ^a +c", &["a", "", "c d e f"]),
            ("T28", "normal", "+a", &[all]),
            ("T28b", "normal", "+c", &["c d e f"]),
        ];
        for (nr, variant, steps, want) in cases {
            assert_eq!(replay3(variant, steps), *want, "{nr} ({variant}): {steps}");
        }
    }

    /// The object order counts collapsed layers and sublayers in place; locks reach sublayers.
    #[test]
    fn layers_object_order_with_locks() {
        let (seq, keys) = doc3("sublocked");
        let key = |n: &str| keys.iter().find(|(k, _)| k == n).map(|(_, r)| *r).expect(n);
        let names: Vec<String> = seq
            .items
            .iter()
            .map(|it| {
                keys.iter()
                    .find(|(_, r)| *r == RowKey::Element(it.id))
                    .map(|(k, _)| k.clone())
                    .unwrap_or_default()
            })
            .collect();
        assert_eq!(names.join(" "), "a b g h c d e f");
        for n in ["L2", "U2", "g", "c"] {
            assert!(seq.row_locked(key(n)), "{n} locked through L2");
        }
        for n in ["L1", "L3", "a", "f"] {
            assert!(!seq.row_locked(key(n)), "{n} not locked");
        }
        // a layer row spans its own objects only (U2's g, h are not part of L2's span)
        let (seq, keys) = doc3("sub");
        let key = |n: &str| keys.iter().find(|(k, _)| k == n).map(|(_, r)| *r).expect(n);
        assert_eq!(seq.span(key("L2")), Some((4, 5)));
        assert_eq!(seq.span(key("U2")), Some((2, 3)));
        assert_eq!(seq.own_unlocked(LayerId(u64::MAX)), Vec::new());
    }

    /// Ctrl on Windows/Linux and Cmd on macOS (egui `command`/`mac_cmd`) toggle; the Ctrl key on a
    /// Mac toggles too; Shift ranges.
    #[test]
    fn fb30_click_modifiers_per_platform() {
        assert_eq!(click_mods(&egui::Modifiers::NONE), CLICK);
        assert_eq!(click_mods(&egui::Modifiers::SHIFT), SHIFT);
        // Windows/Linux: Ctrl sets ctrl and command
        let win_ctrl = egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        };
        assert_eq!(click_mods(&win_ctrl), CTRL);
        // macOS: Cmd sets mac_cmd and command, the Ctrl key only ctrl
        let mac_cmd = egui::Modifiers {
            mac_cmd: true,
            command: true,
            ..Default::default()
        };
        assert_eq!(click_mods(&mac_cmd), CTRL);
        assert_eq!(click_mods(&egui::Modifiers::CTRL), CTRL);
        assert_eq!(
            click_mods(&(egui::Modifiers::SHIFT | egui::Modifiers::COMMAND)),
            CTRL_SHIFT
        );
        assert_eq!(click_mods(&egui::Modifiers::ALT), CLICK);
    }
}

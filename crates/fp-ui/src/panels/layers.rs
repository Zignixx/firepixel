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
use egui::{Align2, Color32, Id, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
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

const COLLAPSED: &str = "layers-collapsed";
const DRAG: &str = "layers-drag";
const RENAME: &str = "layers-rename";
const ANCHOR: &str = "layers-anchor";

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

/// Selection after a click on the object row `clicked` (FB-30, TW-208, RE spec 61 §3.3).
///
/// `order` lists the panel's rows top to bottom as shown (rows of collapsed layers left out), each
/// with the object it selects in a range (`None`: layer/sublayer row, object of a layer that is not
/// editable, locked object). `anchor` is the last plain click, already checked for validity by the
/// caller. Returns the new selection in panel order (`None`: unchanged) and the new anchor.
///
/// - Click: only this row, anchor = this row; a click on a row of a multi-selection keeps it.
/// - Shift (also Ctrl+Shift): range anchor…row across the shown rows (objects of layers and
///   sublayers in between included), replaces the selection; without a usable anchor the topmost
///   selected row is the anchor, without a selection it is a plain click.
/// - Ctrl/Cmd: toggles the row, the anchor stays.
fn object_row_click(
    order: &[(RowKey, Option<fp_model::ElementId>)],
    selected: &[fp_model::ElementId],
    anchor: Option<RowKey>,
    clicked: fp_model::ElementId,
    mods: ClickMods,
) -> (Option<Vec<fp_model::ElementId>>, Option<RowKey>) {
    let pos = |k: RowKey| order.iter().position(|(r, _)| *r == k);
    let here = RowKey::Element(clicked);
    let plain = (Some(vec![clicked]), Some(here));
    if mods.range {
        let start = anchor.filter(|a| pos(*a).is_some()).or_else(|| {
            order
                .iter()
                .find(|(r, _)| matches!(r, RowKey::Element(id) if selected.contains(id)))
                .map(|(r, _)| *r)
        });
        let (Some(a), Some(b)) = (start, pos(here)) else {
            return plain;
        };
        let Some(a_pos) = pos(a) else {
            return plain;
        };
        let (lo, hi) = (a_pos.min(b), a_pos.max(b));
        let ids: Vec<fp_model::ElementId> = order[lo..=hi]
            .iter()
            .filter_map(|(r, sel)| sel.or((*r == here).then_some(clicked)))
            .collect();
        return (Some(ids), Some(a));
    }
    if mods.toggle {
        let mut ids: Vec<fp_model::ElementId> = selected.to_vec();
        if ids.contains(&clicked) {
            ids.retain(|x| *x != clicked);
        } else {
            ids.push(clicked);
        }
        // panel order, like the original's selection after a toggle (RE spec 61 §3.2); objects
        // whose rows are not shown keep their place after them
        let rank = |id: &fp_model::ElementId| pos(RowKey::Element(*id)).unwrap_or(usize::MAX);
        ids.sort_by_key(rank);
        return (Some(ids), anchor.or(Some(here)));
    }
    if selected.len() > 1 && selected.contains(&clicked) {
        return (None, anchor);
    }
    plain
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
    if let Some(v) = fields::number(
        ui,
        "layers-opacity",
        dl.r(120.0, 2.0, 21.0, 12.0),
        Some(dl.r(142.0, 2.0, 9.0, 12.0)),
        if mixed_op { None } else { op.or(Some(100.0)) },
        (0.0, 100.0),
        0,
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
    let collapsed: BTreeSet<u64> = ui
        .data(|dd| dd.get_temp(Id::new(COLLAPSED)))
        .unwrap_or_default();
    let mut toggle_collapse: Option<u64> = None;
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
    // anchor of the last plain click with the selection the panel left behind (sorted ids); it
    // counts while the document and that selection are unchanged (a selection made on the canvas
    // or by a command drops it, a Shift+click then ranges from the topmost selected row)
    let mut sel_sorted: Vec<u64> = selected.iter().map(|x| x.0).collect();
    sel_sorted.sort_unstable();
    let anchor: Option<RowKey> = ui
        .data(|dd| dd.get_temp::<(u64, RowKey, Vec<u64>)>(Id::new(ANCHOR)))
        .filter(|(doc, _, snap)| *doc == doc_id.0 && *snap == sel_sorted)
        .map(|(_, k, _)| k);
    let mut new_anchor: Option<(RowKey, Vec<u64>)> = None;
    // shown rows top to bottom for Shift ranges; object clicks are resolved after the list is built
    let mut order: Vec<(RowKey, Option<fp_model::ElementId>)> = Vec::new();
    let mut pending_click: Option<(fp_model::ElementId, ClickMods)> = None;
    let alt = ui.input(|i| i.modifiers.alt);
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
            let is_active = active_layer == Some(l.id) && (layer_selected || selected.is_empty());
            let (rect, resp) = common::row(ui, 23.0, is_active);
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
            let is_collapsed = collapsed.contains(&l.id.0);
            paint_triangle(ui, tri, !is_collapsed, c.icon_fg);
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
            // Kennzeichen rechts: über Status geteilt / geteilte Ebene / Masterseite
            let mark = Rect::from_center_size(
                pos2(rect.right() - 11.0, rect.center().y),
                vec2(12.0, 12.0),
            );
            if l.shared_across_frames || rl.origin != LayerOrigin::Own {
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
            } else if active_layer == Some(l.id) {
                ui.painter()
                    .circle_stroke(mark.center(), 4.5, Stroke::new(1.0, c.icon_fg));
                ui.painter().circle_filled(mark.center(), 2.0, c.icon_fg);
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
                    toggle_collapse = Some(l.id.0);
                } else if editable {
                    // wie im Original: Klick auf die Ebenenzeile wählt die eigenen Objekte der Ebene aus,
                    // nicht die ihrer Unterebenen (re 28 S3)
                    let ids: Vec<u64> = l
                        .cel(frame)
                        .map(|cel| {
                            own_objects(&cel.elements)
                                .into_iter()
                                .map(|id| id.0)
                                .collect()
                        })
                        .unwrap_or_default();
                    actions.push(Action::Run(
                        "layer.setActive".into(),
                        json!({"layer": l.id.0}),
                    ));
                    new_anchor = Some((RowKey::Layer(l.id), ids.clone()));
                    actions.push(Action::Run("select.set".into(), json!({"ids": ids})));
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
            order.push((RowKey::Layer(l.id), None));
            if is_collapsed {
                continue;
            }
            if let Some(cel) = l.cel(frame) {
                // Unterebenen wie im Original als Ordner mit Aufklapper, ihre Objekte eingerückt darunter
                let mut flat = Vec::new();
                flatten(&cel.elements, frame, 0, &collapsed, &mut flat);
                for (depth, idx, e) in flat {
                    let e = &e;
                    let sel = selected_set.contains(&e.id);
                    if let ElementKind::SubLayer(sl) = &e.kind {
                        // aktive Unterebene hervorheben wie eine aktive Ebene (B-9)
                        let sub_active = active_layer == Some(sl.id);
                        let hl = sel || (sub_active && (layer_selected || selected.is_empty()));
                        let (r2, resp2) = common::row(ui, 23.0, hl);
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
                        let open = !collapsed.contains(&sub_key(e.id));
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
                        } else if sub_active {
                            ui.painter().circle_stroke(
                                mark2.center(),
                                4.5,
                                Stroke::new(1.0, c.icon_fg),
                            );
                            ui.painter().circle_filled(mark2.center(), 2.0, c.icon_fg);
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
                                toggle_collapse = Some(sub_key(e.id));
                            } else if editable {
                                // wie eine Ebenenzeile: Unterebene wird aktiv (neue Objekte landen
                                // darin), ihre Objekte werden ausgewählt (B-9)
                                let ids: Vec<u64> = sl
                                    .cel(frame)
                                    .map(|cel| {
                                        own_objects(&cel.elements)
                                            .into_iter()
                                            .map(|id| id.0)
                                            .collect()
                                    })
                                    .unwrap_or_default();
                                actions.push(Action::Run(
                                    "layer.setActive".into(),
                                    json!({"layer": sl.id.0}),
                                ));
                                new_anchor = Some((RowKey::Layer(sl.id), ids.clone()));
                                actions.push(Action::Run("select.set".into(), json!({"ids": ids})));
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
                        order.push((RowKey::Layer(sl.id), None));
                        continue;
                    }
                    // object row in a Shift range: own editable layer, not locked
                    order.push((
                        RowKey::Element(e.id),
                        (editable && !e.common.locked).then_some(e.id),
                    ));
                    // BA-099: rows outside the visible part of the list only take their place
                    // (pages with thousands of objects built every row each frame)
                    let next =
                        Rect::from_min_size(ui.cursor().min, vec2(ui.available_width(), erow_h));
                    let renaming_row = rename
                        .as_ref()
                        .is_some_and(|k| matches!(k, RowKind::Element { id, .. } if *id == e.id));
                    if !ui.is_rect_visible(next) && !renaming_row {
                        ui.advance_cursor_after_rect(next);
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
                    let (r2, resp2) = common::row(ui, erow_h, sel);
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
                        if let Some(tex) = thumbs::element_thumb(
                            ui.ctx(),
                            st,
                            doc_id,
                            doc_gen,
                            &doc,
                            e,
                            (tpx as u32 * 2).max(16),
                        ) {
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
                            Stroke::new(1.0, c.list_border),
                            StrokeKind::Inside,
                        );
                    }
                    // Maske am Objekt: zweite Miniatur (Maske) rechts daneben wie im Original
                    // re 31 §5.3: Element-Miniatur, Kette, Masken-Miniatur (Einzelelement und Maskengruppe)
                    let mask_thumb = e.element_mask().map(|m| Arc::clone(m.element));
                    let mut mask_hit: Option<(Rect, Rect, fp_model::ElementId)> = None;
                    let thumb = match &mask_thumb {
                        Some(m) if tpx > 0.0 => {
                            // verknüpfte Maske: Kettensymbol zwischen den Miniaturen wie im Original (N35)
                            let linked =
                                matches!(&e.kind, ElementKind::Group(g) if g.mask_flags.linked);
                            let gap = if linked { 12.0 } else { 6.0 };
                            let mt = Rect::from_min_size(
                                pos2(thumb.right() + gap, thumb.top()),
                                thumb.size(),
                            );
                            mask_hit = Some((
                                mt,
                                Rect::from_min_max(
                                    pos2(thumb.right(), thumb.top()),
                                    pos2(mt.left(), mt.bottom()),
                                ),
                                m.id,
                            ));
                            if linked && ui.is_rect_visible(mt) {
                                let cx = thumb.right() + gap / 2.0;
                                let cy = thumb.center().y;
                                let s = Stroke::new(1.0, c.icon_fg);
                                for dy in [-2.5f32, 2.5] {
                                    let link =
                                        Rect::from_center_size(pos2(cx, cy + dy), vec2(4.0, 7.0));
                                    ui.painter().rect_stroke(link, 2.0, s, StrokeKind::Inside);
                                }
                            }
                            if ui.is_rect_visible(mt) {
                                ui.painter().rect_filled(mt, 0.0, Color32::BLACK);
                                if let Some(tex) = thumbs::mask_thumb(
                                    ui.ctx(),
                                    st,
                                    doc_id,
                                    &doc,
                                    e,
                                    m,
                                    (tpx as u32 * 2).max(16),
                                ) {
                                    let ts = tex.size_vec2();
                                    let s = (mt.width() / ts.x).min(mt.height() / ts.y);
                                    ui.painter().image(
                                        tex.id(),
                                        Rect::from_center_size(mt.center(), ts * s),
                                        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                                        Color32::WHITE,
                                    );
                                }
                                // gewählte Maske (Maskenbearbeitung, S5): Rahmen in der Fokusfarbe
                                let mask_sel = selected_set.contains(&m.id);
                                ui.painter().rect_stroke(
                                    mt,
                                    0.0,
                                    Stroke::new(
                                        if mask_sel { 2.0 } else { 1.0 },
                                        if mask_sel { c.focus } else { c.list_border },
                                    ),
                                    StrokeKind::Inside,
                                );
                            }
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
                    let renaming = rename
                        .as_ref()
                        .is_some_and(|k| matches!(k, RowKind::Element { id, .. } if *id == e.id));
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
                        crate::widgets::text_clipped(
                            ui.painter(),
                            pos2(nr.left(), r2.center().y),
                            Align2::LEFT_CENTER,
                            &name,
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
                        if let Some((mt, chain, mid)) = mask_hit.filter(|(mt, chain, _)| {
                            editable && (mt.contains(p) || chain.contains(p))
                        }) {
                            // Masken-Miniatur wählt die Maske (re 31 §5.3, RT-3 S5); Kette schaltet die Verknüpfung
                            if mt.contains(p) {
                                actions.push(Action::Run(
                                    "select.set".into(),
                                    json!({"ids": [mid.0]}),
                                ));
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
                            pending_click = Some((e.id, mods));
                        }
                    }
                    // Q-59 (RE spec 58 §3.2): a right-click on an object row selects the object like a
                    // plain click before the context menu forms its states (Flatten Selection, Add
                    // Mask, Merge Down follow that selection); an object that is already part of the
                    // selection keeps the selection as it is
                    if resp2.secondary_clicked() && editable && !selected_set.contains(&e.id) {
                        actions.push(Action::Run("select.set".into(), json!({"ids": [e.id.0]})));
                        new_anchor = Some((RowKey::Element(e.id), vec![e.id.0]));
                    }
                    if (resp2.double_clicked() || resp2.triple_clicked()) && editable {
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
                }
            }
        }
    });
    if let Some((id, m)) = pending_click {
        let (ids, a) = object_row_click(&order, &selected, anchor, id, m);
        let after: Vec<u64> = match ids {
            Some(ids) => {
                let ids: Vec<u64> = ids.iter().map(|x| x.0).collect();
                actions.push(Action::Run(
                    "select.set".into(),
                    json!({"ids": ids.clone()}),
                ));
                ids
            }
            None => sel_sorted.clone(),
        };
        if new_anchor.is_none() {
            new_anchor = a.map(|k| (k, after));
        }
    }
    if let Some((k, mut snap)) = new_anchor {
        snap.sort_unstable();
        ui.data_mut(|dd| dd.insert_temp(Id::new(ANCHOR), (doc_id.0, k, snap)));
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
    if let Some(id) = toggle_collapse {
        let mut s = collapsed.clone();
        if !s.remove(&id) {
            s.insert(id);
        }
        ui.data_mut(|dd| dd.insert_temp(Id::new(COLLAPSED), s));
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
        Action::Run("layer.add".into(), json!({}))
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

/// Einrückung einer Unterebene (Aufklapper/Ordner) bzw. ihrer Objekt-Miniaturen je Stufe, gemessen am
/// Original (panel-layers/01: Aufklapper 50 → 78 px, Miniatur 82 → 98 px).
const SUB_INDENT: f32 = 28.0;
const THUMB_INDENT: f32 = 16.0;

/// Schlüssel einer Unterebene in der Menge der zugeklappten Zeilen (getrennt von Ebenen-IDs).
fn sub_key(id: fp_model::ElementId) -> u64 {
    id.0 | (1 << 62)
}

/// Objekte einer Ebene mit ihren aufgeklappten Unterebenen der Reihe nach: (Tiefe, Index in der
/// eigenen Liste, Objekt).
fn flatten(
    els: &[Arc<Element>],
    frame: usize,
    depth: usize,
    collapsed: &BTreeSet<u64>,
    out: &mut Vec<(usize, usize, Arc<Element>)>,
) {
    for (i, e) in els.iter().enumerate() {
        out.push((depth, i, Arc::clone(e)));
        if let ElementKind::SubLayer(l) = &e.kind
            && !collapsed.contains(&sub_key(e.id))
            && let Some(cel) = l.cel(frame)
        {
            flatten(&cel.elements, frame, depth + 1, collapsed, out);
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
        flatten(&els, 0, 0, &BTreeSet::new(), &mut out);
        let got: Vec<(usize, usize, u64)> = out.iter().map(|(d, i, e)| (*d, *i, e.id.0)).collect();
        assert_eq!(
            got,
            vec![(0, 0, a.id.0), (1, 0, inner.id.0), (0, 1, b.id.0)]
        );
        let mut closed = BTreeSet::new();
        closed.insert(sub_key(a.id));
        let mut out = Vec::new();
        flatten(&els, 0, 0, &closed, &mut out);
        assert_eq!(
            out.iter().map(|(d, _, e)| (*d, e.id.0)).collect::<Vec<_>>(),
            vec![(0, a.id.0), (0, b.id.0)]
        );
    }

    fn eid(n: u64) -> fp_model::ElementId {
        fp_model::ElementId(n)
    }

    /// Measured list (RE spec 61 §3.1): web layer, Layer 1, O4…O1 (ids 4…1).
    fn measured_order() -> Vec<(RowKey, Option<fp_model::ElementId>)> {
        let mut v = vec![
            (RowKey::Layer(LayerId(100)), None),
            (RowKey::Layer(LayerId(101)), None),
        ];
        for n in [4, 3, 2, 1] {
            v.push((RowKey::Element(eid(n)), Some(eid(n))));
        }
        v
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
        let order = measured_order();
        let (mut sel, mut anchor): (Vec<fp_model::ElementId>, Option<RowKey>) = (Vec::new(), None);
        let mut out = Vec::new();
        for (n, m) in steps {
            let (ids, na) = object_row_click(&order, &sel, anchor, eid(*n), *m);
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

    /// Range from a layer anchor and across layer/sublayer rows: the object rows in between count,
    /// layer rows and objects of other non-editable or locked rows do not.
    #[test]
    fn fb30_range_across_layers_and_fallback_anchor() {
        let order = vec![
            (RowKey::Layer(LayerId(1)), None),
            (RowKey::Element(eid(10)), Some(eid(10))),
            (RowKey::Layer(LayerId(2)), None), // sublayer row
            (RowKey::Element(eid(11)), Some(eid(11))),
            (RowKey::Element(eid(12)), None), // locked object
            (RowKey::Layer(LayerId(3)), None),
            (RowKey::Element(eid(13)), Some(eid(13))),
        ];
        let (ids, a) =
            object_row_click(&order, &[], Some(RowKey::Element(eid(10))), eid(13), SHIFT);
        assert_eq!(ids, Some(vec![eid(10), eid(11), eid(13)]));
        assert_eq!(a, Some(RowKey::Element(eid(10))), "Shift keeps the anchor");
        let (ids, _) =
            object_row_click(&order, &[], Some(RowKey::Layer(LayerId(1))), eid(11), SHIFT);
        assert_eq!(
            ids,
            Some(vec![eid(10), eid(11)]),
            "range from a layer anchor"
        );
        // no anchor: topmost selected row
        let (ids, a) = object_row_click(&order, &[eid(13), eid(11)], None, eid(10), SHIFT);
        assert_eq!(ids, Some(vec![eid(10), eid(11)]));
        assert_eq!(a, Some(RowKey::Element(eid(11))));
        // nothing selected, no anchor: plain click
        let (ids, a) = object_row_click(&order, &[], None, eid(11), SHIFT);
        assert_eq!(ids, Some(vec![eid(11)]));
        assert_eq!(a, Some(RowKey::Element(eid(11))));
        // the clicked row itself is part of the range even when locked
        let (ids, _) =
            object_row_click(&order, &[], Some(RowKey::Element(eid(11))), eid(12), SHIFT);
        assert_eq!(ids, Some(vec![eid(11), eid(12)]));
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

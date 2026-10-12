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
use crate::inspector::obj_quads::{blend_index, blend_names, BLEND_ITEMS};
use crate::theme::Theme;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, Sense, Stroke, StrokeKind, Ui};
use fp_model::{Element, ElementKind, LayerId, LayerKind, LayerOrigin};
use serde_json::json;
use std::collections::BTreeSet;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
enum RowKind {
    Layer { id: LayerId, editable: bool, web: bool },
    Element { id: fp_model::ElementId, layer: LayerId, index: usize, editable: bool },
}

#[derive(Clone, Debug)]
struct RowInfo {
    kind: RowKind,
    rect: Rect,
}

#[derive(Clone, Debug, PartialEq)]
enum Drag {
    Element { id: fp_model::ElementId },
    Layer { id: LayerId },
}

const COLLAPSED: &str = "layers-collapsed";
const DRAG: &str = "layers-drag";
const RENAME: &str = "layers-rename";

fn element_label(e: &Element) -> String {
    if let Some(n) = &e.common.name
        && !n.is_empty()
    {
        return n.clone();
    }
    let t = common::t;
    match &e.kind {
        ElementKind::Bitmap(_) => t("panel-layers-kind-bitmap"),
        ElementKind::Path(p) if p.primitive.is_some() => t("panel-layers-kind-rectangle"),
        ElementKind::Path(_) => t("panel-layers-kind-path"),
        ElementKind::Text(tx) => {
            let s = tx.plain_text();
            let s: String = s.chars().take(24).collect();
            if s.trim().is_empty() { t("panel-layers-kind-text") } else { s }
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
    let active_layer = d.layer;
    // „Ebene gewählt“: Auswahl = alle (ungesperrten) Objekte der aktiven Ebene, auch leer
    let layer_objects = |lid: LayerId| -> Vec<fp_model::ElementId> {
        layers.iter().find(|rl| rl.layer.id == lid).and_then(|rl| rl.layer.cel(frame)).map(|cel| cel.elements.iter().filter(|e| !e.common.locked).map(|e| e.id).collect()).unwrap_or_default()
    };
    let layer_selected = active_layer.is_some_and(|lid| {
        let mut a = layer_objects(lid);
        let mut b = selected.clone();
        a.sort();
        b.sort();
        a == b
    });
    let first_sel = selected.first().and_then(|id| fp_engine::docops::element_on_page(&doc, page, *id));
    // Kopf: Mischmodus (4261), Deckkraft (4260)
    let (head, _) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::hover());
    let dl = Dl::at(head.min + vec2(2.0, 1.0));
    let en = first_sel.is_some();
    let names = blend_names();
    let bl = first_sel.as_ref().and_then(|e| blend_index(e.common.blend)).unwrap_or(0);
    if let Some(i) = fields::combo(ui, "layers-blend", dl.r(1.0, 2.0, 80.0, 12.0), &names, Some(bl), en)
        && BLEND_ITEMS[i].1 >= 0
    {
        st.run("element.setBlendMode", json!({"mode": BLEND_ITEMS[i].1.to_string()}));
    }
    common::label_fit(ui, dl.r(84.0, 3.0, 35.0, 8.0), &common::t("panel-layers-opacity"), true);
    let op = first_sel.as_ref().map(|e| e.common.opacity.to_percent().round());
    if let Some(v) = fields::number(ui, "layers-opacity", dl.r(120.0, 2.0, 21.0, 12.0), Some(dl.r(142.0, 2.0, 9.0, 12.0)), op.or(Some(100.0)), (0.0, 100.0), 0, en) {
        st.run("element.setOpacity", json!({"opacity": v}));
    }
    let collapsed: BTreeSet<u64> = ui.data(|dd| dd.get_temp(Id::new(COLLAPSED))).unwrap_or_default();
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
    let mut eye_drag: Option<(bool, BTreeSet<u64>)> = ui.data(|dd| dd.get_temp(eye_key));
    let (pointer, primary_down) = ui.input(|i| (i.pointer.latest_pos(), i.pointer.primary_down()));
    if !primary_down {
        eye_drag = None;
    }
    let press_origin = ui.input(|i| i.pointer.press_origin());
    let rename: Option<RowKind> = ui.data(|dd| dd.get_temp(Id::new(RENAME)));
    let shift = ui.input(|i| i.modifiers.shift || i.modifiers.command);
    // Miniaturoptionen (0 = keine; Vorgabe 32 → 24 px wie im Original „mittel“)
    let tpx = (st.session.prefs.get_int("LayersList_ThumbnailSize", 32).clamp(0, 128) as f32 * 0.75).round();
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
            let lock_r = Rect::from_min_size(pos2(rect.left() + 21.0, rect.top()), vec2(19.0, rect.height()));
            ui.painter().vline(rect.left() + 20.5, rect.y_range(), Stroke::new(1.0, c.list_row_line));
            ui.painter().vline(rect.left() + 40.5, rect.y_range(), Stroke::new(1.0, c.list_row_line));
            let visible = l.is_visible();
            if visible {
                glyphs::paint(ui.painter(), eye_r.shrink(3.0), Glyph::Eye, &theme, true);
            }
            let locked = l.cel(frame).is_some_and(|cel| cel.locked);
            if locked {
                glyphs::paint(ui.painter(), lock_r.shrink(4.0), Glyph::Lock, &theme, true);
            }
            let tri = Rect::from_center_size(pos2(rect.left() + 50.0, rect.center().y), vec2(10.0, 10.0));
            let is_collapsed = collapsed.contains(&l.id.0);
            paint_triangle(ui, tri, !is_collapsed, c.icon_fg);
            glyphs::paint(ui.painter(), Rect::from_center_size(pos2(rect.left() + 66.0, rect.center().y), vec2(16.0, 14.0)), Glyph::Folder, &theme, true);
            let name = if web { common::t("panel-layers-web-layer") } else { l.name.clone() };
            let name_r = Rect::from_min_max(pos2(rect.left() + 78.0, rect.top() + 2.0), pos2(rect.right() - 22.0, rect.bottom() - 2.0));
            let renaming = rename.as_ref().is_some_and(|k| matches!(k, RowKind::Layer { id, .. } if *id == l.id));
            if renaming {
                if let Some(res) = common::inline_rename(ui, Id::new(("layer-rename", l.id.0)), name_r, &name) {
                    if let Some(n) = res {
                        actions.push(Action::Run("layer.rename".into(), json!({"layer": l.id.0, "name": n})));
                    }
                    ui.data_mut(|dd| dd.remove::<RowKind>(Id::new(RENAME)));
                }
            } else {
                let font = if is_active { theme.bold(theme.fonts.ui) } else { theme.ui_font() };
                let col = if editable { c.text } else { c.text_weak };
                crate::widgets::text_clipped(ui.painter(), pos2(name_r.left(), rect.center().y), Align2::LEFT_CENTER, &name, font, col, name_r.width());
            }
            // Kennzeichen rechts: über Status geteilt / geteilte Ebene / Masterseite
            let mark = Rect::from_center_size(pos2(rect.right() - 11.0, rect.center().y), vec2(12.0, 12.0));
            if l.shared_across_frames || rl.origin != LayerOrigin::Own {
                glyphs::paint(ui.painter(), mark, if rl.origin == LayerOrigin::Own { Glyph::Distribute } else { Glyph::Link }, &theme, true);
            } else if active_layer == Some(l.id) {
                ui.painter().circle_stroke(mark.center(), 4.5, Stroke::new(1.0, c.icon_fg));
                ui.painter().circle_filled(mark.center(), 2.0, c.icon_fg);
            }
            let kind = RowKind::Layer { id: l.id, editable, web };
            if resp.clicked()
                && let Some(p) = resp.interact_pointer_pos()
            {
                if eye_r.contains(p) && editable {
                    actions.push(Action::Run("layer.setVisible".into(), json!({"layer": l.id.0, "visible": !visible})));
                } else if lock_r.contains(p) && editable {
                    actions.push(Action::Run("layer.setLocked".into(), json!({"layer": l.id.0, "locked": !locked})));
                } else if tri.expand(3.0).contains(p) {
                    toggle_collapse = Some(l.id.0);
                } else if editable {
                    // wie im Original: Klick auf die Ebenenzeile wählt alle Objekte der Ebene aus
                    let ids: Vec<u64> = l.cel(frame).map(|cel| cel.elements.iter().filter(|e| !e.common.locked).map(|e| e.id.0).collect()).unwrap_or_default();
                    actions.push(Action::Run("layer.setActive".into(), json!({"layer": l.id.0})));
                    actions.push(Action::Run("select.set".into(), json!({"ids": ids})));
                } else if rl.origin == LayerOrigin::Shared {
                    actions.push(Action::Run("pi.layer.setActiveShared".into(), json!({"layer": l.id.0})));
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
                if eye_drag.is_none() {
                    actions.push(Action::Run("layer.setVisible".into(), json!({"layer": l.id.0, "visible": !visible})));
                }
                eye_drag = Some((!visible, done));
            } else if resp.drag_started() && editable && !web {
                start_drag = Some(Drag::Layer { id: l.id });
            }
            if let Some((target, done)) = eye_drag.as_mut()
                && editable
                && pointer.is_some_and(|p| p.y >= rect.top() && p.y < rect.bottom() && p.x >= eye_r.left() - 2.0 && p.x <= eye_r.right() + 2.0)
                && !done.contains(&l.id.0)
            {
                done.insert(l.id.0);
                if visible != *target {
                    actions.push(Action::Run("layer.setVisible".into(), json!({"layer": l.id.0, "visible": *target})));
                }
            }
            menu_rows.push(resp.clone());
            rows.push(RowInfo { kind, rect });
            if is_collapsed {
                continue;
            }
            if let Some(cel) = l.cel(frame) {
                for (idx, e) in cel.elements.iter().enumerate() {
                    let sel = selected.contains(&e.id);
                    let (r2, resp2) = common::row(ui, erow_h, sel);
                    let eye2 = Rect::from_min_size(r2.min, vec2(20.0, r2.height()));
                    let lock2 = Rect::from_min_size(pos2(r2.left() + 21.0, r2.top()), vec2(19.0, r2.height()));
                    ui.painter().vline(r2.left() + 20.5, r2.y_range(), Stroke::new(1.0, c.list_row_line));
                    ui.painter().vline(r2.left() + 40.5, r2.y_range(), Stroke::new(1.0, c.list_row_line));
                    if e.common.visible {
                        glyphs::paint(ui.painter(), eye2.shrink2(vec2(3.0, 8.0)), Glyph::Eye, &theme, true);
                    }
                    if e.common.locked {
                        glyphs::paint(ui.painter(), lock2.shrink2(vec2(4.0, 8.0)), Glyph::Lock, &theme, true);
                    }
                    let thumb = Rect::from_min_size(pos2(r2.left() + 78.0, r2.top() + 3.0), vec2(tpx, tpx));
                    if tpx > 0.0 {
                        crate::widgets::checker(ui.painter(), thumb, 4.0, &theme);
                        if let Some(tex) = thumbs::element_thumb(ui.ctx(), st, doc_id, doc_gen, &doc, e, (tpx as u32 * 2).max(16)) {
                            let ts = tex.size_vec2();
                            let s = (thumb.width() / ts.x).min(thumb.height() / ts.y);
                            ui.painter().image(tex.id(), Rect::from_center_size(thumb.center(), ts * s), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                        }
                        ui.painter().rect_stroke(thumb, 0.0, Stroke::new(1.0, c.list_border), StrokeKind::Inside);
                    }
                    let name = element_label(e);
                    // Umbenennen: unbenannte Objekte beginnen mit leerem Feld (der Typname wird nicht vorbelegt)
                    let own_name = e.common.name.clone().unwrap_or_default();
                    let nr = Rect::from_min_max(pos2(thumb.right() + 6.0, r2.top() + 5.0), pos2(r2.right() - 4.0, r2.bottom() - 5.0));
                    let renaming = rename.as_ref().is_some_and(|k| matches!(k, RowKind::Element { id, .. } if *id == e.id));
                    if renaming {
                        if let Some(res) = common::inline_rename(ui, Id::new(("el-rename", e.id.0)), nr, &own_name) {
                            if let Some(n) = res {
                                actions.push(Action::Run("element.setName".into(), json!({"ids": [e.id.0], "name": n})));
                            }
                            ui.data_mut(|dd| dd.remove::<RowKind>(Id::new(RENAME)));
                        }
                    } else {
                        let font = if sel { theme.bold(theme.fonts.ui) } else { theme.ui_font() };
                        crate::widgets::text_clipped(ui.painter(), pos2(nr.left(), r2.center().y), Align2::LEFT_CENTER, &name, font, if editable { c.text } else { c.text_weak }, nr.width());
                    }
                    let kind = RowKind::Element { id: e.id, layer: l.id, index: idx, editable };
                    if resp2.clicked()
                        && let Some(p) = resp2.interact_pointer_pos()
                    {
                        if eye2.contains(p) && editable {
                            actions.push(Action::Run("element.setVisible".into(), json!({"ids": [e.id.0], "visible": !e.common.visible})));
                        } else if lock2.contains(p) && editable {
                            actions.push(Action::Run("element.setLocked".into(), json!({"ids": [e.id.0], "locked": !e.common.locked})));
                        } else if editable {
                            let mut ids: Vec<u64> = if shift { selected.iter().map(|x| x.0).collect() } else { Vec::new() };
                            if shift && ids.contains(&e.id.0) {
                                ids.retain(|x| *x != e.id.0);
                            } else {
                                ids.push(e.id.0);
                            }
                            actions.push(Action::Run("select.set".into(), json!({"ids": ids})));
                        }
                    }
                    if (resp2.double_clicked() || resp2.triple_clicked()) && editable {
                        rename_start = Some(kind.clone());
                    }
                    if resp2.drag_started() && editable {
                        start_drag = Some(Drag::Element { id: e.id });
                    }
                    menu_rows.push(resp2.clone());
                    rows.push(RowInfo { kind, rect: r2 });
                }
            }
        }
    });
    ui.data_mut(|dd| match eye_drag {
        Some(v) => {
            dd.insert_temp(eye_key, v);
        }
        None => dd.remove::<(bool, BTreeSet<u64>)>(eye_key),
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
        let target = pointer.filter(|p| list_rect.contains(*p)).and_then(|p| drop_target(&rows, p, &dg));
        if let Some((y, _)) = &target {
            ui.painter().hline(list_rect.x_range().shrink(2.0), *y, Stroke::new(2.0, c.focus));
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
    let alt = ui.input(|i| i.modifiers.alt);
    let new_layer = if alt { Action::Run("layer.duplicate".into(), json!({})) } else { Action::Run("layer.add".into(), json!({})) };
    let tips: [String; 5] = ["panel-layers-new-sublayer", "panel-layers-new-layer", "panel-layers-new-bitmap", "panel-layers-add-mask", "panel-layers-delete"].map(common::t);
    let buttons = [
        FooterButton { glyph: Glyph::NewSubLayer, tip: &tips[0], enabled: true, action: Some(Action::Run("pi.layer.addSub".into(), json!({}))) },
        FooterButton { glyph: Glyph::NewLayer, tip: &tips[1], enabled: true, action: Some(new_layer) },
        FooterButton { glyph: Glyph::Bitmap, tip: &tips[2], enabled: true, action: Some(Action::Run("edit.insert.emptyBitmap".into(), json!({}))) },
        FooterButton { glyph: Glyph::Mask, tip: &tips[3], enabled: !selected.is_empty(), action: Some(Action::Run("modify.mask.revealAll".into(), json!({}))) },
        FooterButton { glyph: Glyph::Trash, tip: &tips[4], enabled: !selected.is_empty() || can_delete_layer, action: del_action },
    ];
    if let Some(a) = footer_with_state(ui, st, &frame_text, &buttons) {
        actions.push(a);
    }
    for a in actions {
        st.push(a);
    }
}

fn paint_triangle(ui: &Ui, r: Rect, open: bool, col: Color32) {
    let c = r.center();
    let pts = if open { vec![pos2(c.x - 4.0, c.y - 2.0), pos2(c.x + 4.0, c.y - 2.0), pos2(c.x, c.y + 3.0)] } else { vec![pos2(c.x - 2.0, c.y - 4.0), pos2(c.x + 3.0, c.y), pos2(c.x - 2.0, c.y + 4.0)] };
    ui.painter().add(egui::Shape::convex_polygon(pts, col, Stroke::NONE));
}

/// Zielposition beim Ziehen: (y der Einfügelinie, Befehl).
fn drop_target(rows: &[RowInfo], p: egui::Pos2, dg: &Drag) -> Option<(f32, Action)> {
    let i = rows.iter().position(|r| r.rect.contains(p))?;
    let r = &rows[i];
    let upper = p.y < r.rect.center().y;
    match (dg, &r.kind) {
        (Drag::Element { id }, RowKind::Element { layer, index, editable: true, .. }) => {
            if r.kind == (RowKind::Element { id: *id, layer: *layer, index: *index, editable: true }) {
                return None;
            }
            let idx = if upper { *index } else { index + 1 };
            Some((if upper { r.rect.top() } else { r.rect.bottom() }, Action::Run("pi.element.moveTo".into(), json!({"id": id.0, "layer": layer.0, "index": idx}))))
        }
        (Drag::Element { id }, RowKind::Layer { id: lid, editable: true, web: false }) => Some((r.rect.bottom(), Action::Run("pi.element.moveTo".into(), json!({"id": id.0, "layer": lid.0, "index": 0})))),
        (Drag::Layer { id }, RowKind::Layer { id: target, editable: true, .. }) if id != target => {
            // Ebenenliste unten → oben; die Anzeige ist oben → unten
            let layer_rows: Vec<LayerId> = rows.iter().filter_map(|r| match r.kind {
                RowKind::Layer { id, .. } => Some(id),
                _ => None,
            }).collect();
            let n = layer_rows.len();
            let pos_display = layer_rows.iter().position(|x| x == target)?;
            let from_display = layer_rows.iter().position(|x| x == id)?;
            let mut to_display = if upper { pos_display } else { pos_display + 1 };
            if from_display < to_display {
                to_display -= 1;
            }
            let index = n.saturating_sub(1).saturating_sub(to_display);
            Some((if upper { r.rect.top() } else { r.rect.bottom() }, Action::Run("layer.move".into(), json!({"layer": id.0, "index": index}))))
        }
        _ => None,
    }
}

/// Fußleiste mit „Status n“-Menüknopf (4286) links.
fn footer_with_state(ui: &mut Ui, st: &mut AppState, text: &str, buttons: &[FooterButton<'_>]) -> Option<Action> {
    let theme = Theme::current(ui.ctx());
    let rect = ui.available_rect_before_wrap();
    // nicht unter die Knöpfe rechts reichen (schmale Bedienfelder)
    let free = rect.width() - 20.0 - 4.0 - 20.0 * buttons.len() as f32 - 4.0;
    let sr = Rect::from_min_size(pos2(rect.left() + 20.0, rect.bottom() - common::FOOTER_H + 3.0), vec2(free.clamp(16.0, 100.0), 16.0));
    let resp = ui.interact(sr, Id::new("layers-state-btn"), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(sr, 2.0, theme.colors.button_hover_bg);
    }
    let mut out = common::footer(ui, 26.0, text, buttons);
    let n = st.session.active().map(|d| d.frame_count()).unwrap_or(1);
    let names: Vec<String> = (0..n).map(|i| super::state_name(st, i)).collect();
    egui::Popup::menu(&resp).frame(crate::menu::menu_frame(&theme)).show(|ui| {
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

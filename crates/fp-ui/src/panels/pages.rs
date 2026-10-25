//! Seiten-Bedienfeld (`CPageInspectorPanel`, Vorlage 6306): Seitenliste mit Miniaturen (fp-render),
//! Nummer und Name, Masterseite (Kennzeichen, als Masterseite festlegen), Umbenennen per
//! Doppelklick, Zähler „n Seiten“, Neue/Doppelte Seite, Seite löschen, Kontextmenü der Zeilen (7026),
//! Zeile auf „Neue Seite“ ziehen = Seite duplizieren (RE spec 70 §2). Mehrfachauswahl mit
//! Umschalt/Strg, Löschen und Duplizieren aller gewählten Seiten (FB-68, RE spec 71 §4); eine
//! gewählte Seite ziehen verschiebt alle gewählten Seiten als Block (FB-89).

use super::common::{self, FooterButton};
use super::thumbs;
use crate::app_state::{Action, AppState};
use crate::inspector::glyphs::Glyph;
use crate::theme::Theme;
use egui::{Align2, Color32, Id, Rect, Stroke, StrokeKind, Ui, pos2, vec2};
use serde_json::json;
use std::sync::Arc;

const RENAME: &str = "pages-rename";
/// FB-68: the page list has the keyboard focus (a row was clicked last): Delete deletes the
/// selected pages ([`delete_key_target`]).
const FOCUS: &str = "pages-list-focus";
/// Rectangle of the page list in the last frame (a press outside it ends the focus).
const LIST_RECT: &str = "pages-list-rect";

/// FB-68: a press outside the page list ends its keyboard focus; call once per frame before the
/// keys are handled.
pub fn track_focus(ctx: &egui::Context) {
    let press = ctx.input(|i| {
        i.pointer
            .any_pressed()
            .then(|| i.pointer.interact_pos())
            .flatten()
    });
    if let Some(p) = press {
        let inside = ctx
            .data(|d| d.get_temp::<Rect>(Id::new(LIST_RECT)))
            .is_some_and(|r| r.contains(p));
        if !inside {
            ctx.data_mut(|d| d.insert_temp(Id::new(FOCUS), false));
        }
    }
}

/// FB-68: Delete with the page list focused (a row was clicked last) deletes the selected pages
/// like the panel's Delete button.
pub fn delete_key_target(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(Id::new(FOCUS)))
        .unwrap_or(false)
}

/// Ungültige Zeichen in Seitennamen (String 104).
pub const INVALID_CHARS: &[char] = &['\\', '/', ':', '*', '?', '"', '<', '>', '|'];

/// Seitenname prüfen.
pub fn valid_page_name(s: &str) -> bool {
    !s.trim().is_empty() && !s.chars().any(|c| INVALID_CHARS.contains(&c))
}

/// Zeichnet das Bedienfeld.
pub fn show(ui: &mut Ui, st: &mut AppState) {
    let theme = Theme::current(ui.ctx());
    let c = theme.colors.clone();
    let Some(d) = st.session.active() else {
        return;
    };
    let doc = Arc::clone(&d.doc);
    let (doc_id, doc_gen, current) = (d.id, d.generation, d.page);
    // FB-68 (RE spec 71 §4): several pages can be selected; the current page is the anchor
    let selected = d.selected_pages();
    let multi = selected.len() > 1;
    let mods = ui.input(|i| i.modifiers);
    let n = doc.pages.len();
    let mut actions = Vec::new();
    let rename: Option<usize> = ui.data(|dd| dd.get_temp(Id::new(RENAME)));
    // Miniaturgröße (Optionen für Miniaturansicht, 0 = keine) und Nummerierung
    let tsize = st
        .session
        .prefs
        .get_int("PagesList_ThumbnailSize", 32)
        .clamp(0, 128) as f32;
    let numbering = st.session.prefs.get_bool("PageNumbering", true);
    let row_h = (tsize + 10.0).max(24.0);
    let mut row_rects = Vec::with_capacity(n);
    let mut drag_start = None;
    // a row dragged onto the New Page button duplicates that page (RE spec 70 §2.1)
    let mut row_drag: Option<usize> = None;
    let mut row_dropped: Option<usize> = None;
    let mut menu_rows = Vec::new();
    let list = common::list_frame(ui, "pages-list", common::FOOTER_H, |ui| {
        for (i, pg) in doc.pages.iter().enumerate() {
            // the current page: bold name, blue-framed thumbnail; every selected page: highlight
            let sel = i == current;
            let in_sel = selected.contains(&i);
            let (rect, resp) = common::row(ui, row_h, in_sel);
            row_rects.push(rect);
            if resp.drag_started() && rename != Some(i) {
                drag_start = Some(i);
            }
            if resp.dragged() && rename != Some(i) {
                row_drag = Some(i);
            }
            if resp.drag_stopped() && rename != Some(i) {
                row_dropped = Some(i);
            }
            ui.painter().vline(
                rect.left() + 28.5,
                rect.y_range(),
                Stroke::new(1.0, c.list_row_line),
            );
            if pg.is_master == Some(true) {
                ui.painter().text(
                    pos2(rect.left() + 14.0, rect.center().y),
                    Align2::CENTER_CENTER,
                    "M",
                    theme.bold(theme.fonts.small),
                    c.text,
                );
            } else if pg.linked_to_master == Some(true) {
                crate::inspector::glyphs::paint(
                    ui.painter(),
                    Rect::from_center_size(
                        pos2(rect.left() + 14.0, rect.center().y),
                        vec2(14.0, 12.0),
                    ),
                    Glyph::Link,
                    &theme,
                    true,
                );
            }
            let thumb = Rect::from_min_size(
                pos2(rect.left() + 36.0, rect.top() + 5.0),
                vec2(tsize, tsize),
            );
            if tsize > 0.0 {
                ui.painter().rect_filled(thumb, 0.0, Color32::WHITE);
                if let Some(tex) = thumbs::page_thumb(
                    ui.ctx(),
                    st,
                    doc_id,
                    doc_gen,
                    &doc,
                    i,
                    (tsize as u32 * 2).max(16),
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
                    Stroke::new(
                        if sel { 2.0 } else { 1.0 },
                        if sel { c.selection } else { c.list_border },
                    ),
                    StrokeKind::Inside,
                );
            }
            let name = pg.display_name().to_string();
            let nr = Rect::from_min_max(
                pos2(thumb.right() + 10.0, rect.center().y - 10.0),
                pos2(rect.right() - 6.0, rect.center().y + 10.0),
            );
            if rename == Some(i) {
                if let Some(res) = common::inline_rename(ui, Id::new(("page-rename", i)), nr, &name)
                {
                    match res {
                        Some(nn) if valid_page_name(&nn) => actions.push(Action::Run(
                            "page.rename".into(),
                            json!({"index": i, "name": nn}),
                        )),
                        Some(_) => {
                            actions.push(Action::Status(common::t("panel-pages-invalid-name")))
                        }
                        None => {}
                    }
                    ui.data_mut(|dd| dd.remove::<usize>(Id::new(RENAME)));
                }
            } else {
                let font = if sel {
                    theme.bold(theme.fonts.ui)
                } else {
                    theme.ui_font()
                };
                let label = if numbering {
                    format!("{:02}  {}", i + 1, name)
                } else {
                    name.clone()
                };
                crate::widgets::text_clipped(
                    ui.painter(),
                    pos2(nr.left(), rect.center().y),
                    Align2::LEFT_CENTER,
                    &label,
                    font,
                    if in_sel { c.list_selected_text } else { c.text },
                    nr.width(),
                );
            }
            if resp.clicked() || resp.secondary_clicked() {
                ui.data_mut(|dd| dd.insert_temp(Id::new(FOCUS), true));
            }
            // RE spec 71 §4.2: Shift = range from the current page, Ctrl = add/remove this page,
            // a plain click = this page alone (current); each click is a History row "Change
            // Page Selection"
            if resp.clicked() {
                let mode = if mods.shift {
                    Some("range")
                } else if mods.command || mods.ctrl {
                    Some("toggle")
                } else if i != current || multi {
                    Some("single")
                } else {
                    None
                };
                if let Some(mode) = mode {
                    actions.push(Action::Run(
                        "page.select".into(),
                        json!({"index": i, "mode": mode}),
                    ));
                }
            }
            // RE spec 70 §2.3 point 3 / 71 §4.2: a right-click on an unselected row selects it
            // alone (its own History row "Change Page Selection"); on a selected row the
            // multi-selection stays. Then the context menu (resource 7026, the entries of the
            // panel menu) acts on the selection
            if resp.secondary_clicked() && !in_sel {
                actions.push(Action::Run(
                    "page.select".into(),
                    json!({"index": i, "mode": "single"}),
                ));
            }
            menu_rows.push(resp.clone());
            if resp.double_clicked() {
                ui.data_mut(|dd| dd.insert_temp(Id::new(RENAME), i));
            }
        }
    });
    ui.data_mut(|dd| dd.insert_temp(Id::new(LIST_RECT), list));
    for r in &menu_rows {
        common::resource_context_menu(r, st, "7026");
    }
    // Dropping a row onto the New Page button: the press made the row current, then Duplicate
    // Page inserts the copy directly after it (RE spec 70 §2.1/§2.3 point 2)
    let new_button = common::footer_button_rect_ahead(ui, 0, 2).expand(2.0);
    let pointer = ui.input(|i| i.pointer.interact_pos().or(i.pointer.latest_pos()));
    let over_new = pointer.is_some_and(|p| new_button.contains(p));
    if row_drag.is_some() && over_new {
        ui.painter().rect_stroke(
            new_button.shrink(2.0),
            2.0,
            Stroke::new(1.0, c.focus),
            StrokeKind::Inside,
        );
    }
    let dropped_on_new = row_dropped.filter(|_| over_new);
    // Zeilen per Ziehen umordnen (CT-C-03), ein Verlaufsschritt „Seite verschieben“
    // FB-89 (Firepixel extension): dragging one of several selected pages moves all of them as
    // a block in their order (one History row "Move Pages"); an unselected page moves alone
    let moved = common::reorder_rows_block(ui, "pages", list, &row_rects, drag_start, &selected);
    if multi
        && row_drag.is_some_and(|i| selected.contains(&i))
        && !over_new
        && let Some(p) = pointer
    {
        drag_ghost(ui, p, selected.len(), &theme);
    }
    if let Some(i) = dropped_on_new {
        if i != current {
            actions.push(Action::SetPage(i));
        }
        actions.push(Action::Run("page.duplicate".into(), json!({"index": i})));
    } else if let Some((pages, to)) = moved {
        actions.push(Action::Run(
            "page.move".into(),
            if pages.len() > 1 {
                json!({"pages": pages, "index": to})
            } else {
                json!({"page": pages[0], "index": to})
            },
        ));
    }
    let text = fp_i18n::tr!("panel-pages-count", n = n);
    let alt = ui.input(|i| i.modifiers.alt);
    let (tip_new, tip_del) = (
        common::t("panel-pages-new"),
        common::t("panel-pages-delete"),
    );
    let buttons = [
        FooterButton {
            glyph: Glyph::NewLayer,
            tip: &tip_new,
            enabled: true,
            action: Some(if alt {
                Action::Run("page.duplicate".into(), json!({}))
            } else {
                Action::Run("edit.insert.page".into(), json!({}))
            }),
        },
        FooterButton {
            glyph: Glyph::Trash,
            tip: &tip_del,
            // FB-68 (RE spec 71 §4.3): deletes the selected pages; the only page becomes an
            // empty "Page 1", so the button is always enabled
            enabled: n > 0,
            action: Some(Action::Run("page.delete".into(), json!({}))),
        },
    ];
    if let Some(a) = common::footer(ui, 30.0, &text, &buttons) {
        actions.push(a);
    }
    for a in actions {
        st.push(a);
    }
}

/// FB-89: the label beside the pointer while a block of `n` selected pages is dragged ("3 pages").
fn drag_ghost(ui: &Ui, p: egui::Pos2, n: usize, theme: &Theme) {
    let c = &theme.colors;
    let text = fp_i18n::tr!("panel-pages-count", n = n);
    let painter = ui.ctx().layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        Id::new("pages-drag-ghost"),
    ));
    let galley = painter.layout_no_wrap(text, theme.ui_font(), c.text);
    let rect = Rect::from_min_size(p + vec2(14.0, 10.0), galley.size() + vec2(12.0, 6.0));
    painter.rect_filled(rect, 2.0, c.list_selected_bg);
    painter.rect_stroke(rect, 2.0, Stroke::new(1.0, c.focus), StrokeKind::Inside);
    painter.galley(rect.min + vec2(6.0, 3.0), galley, c.text);
}

#[cfg(test)]
mod tests {
    #[test]
    fn page_names() {
        assert!(super::valid_page_name("Seite 2"));
        assert!(!super::valid_page_name("a/b"));
        assert!(!super::valid_page_name("  "));
    }
}

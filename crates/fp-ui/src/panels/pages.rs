//! Seiten-Bedienfeld (`CPageInspectorPanel`, Vorlage 6306): Seitenliste mit Miniaturen (fp-render),
//! Nummer und Name, Masterseite (Kennzeichen, als Masterseite festlegen), Umbenennen per
//! Doppelklick, Zähler „n Seiten“, Neue/Doppelte Seite, Seite löschen.

use super::common::{self, FooterButton};
use super::thumbs;
use crate::app_state::{Action, AppState};
use crate::inspector::glyphs::Glyph;
use crate::theme::Theme;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, Stroke, StrokeKind, Ui};
use serde_json::json;
use std::sync::Arc;

const RENAME: &str = "pages-rename";

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
    let n = doc.pages.len();
    let mut actions = Vec::new();
    let rename: Option<usize> = ui.data(|dd| dd.get_temp(Id::new(RENAME)));
    // Miniaturgröße (Optionen für Miniaturansicht, 0 = keine) und Nummerierung
    let tsize = st.session.prefs.get_int("PagesList_ThumbnailSize", 32).clamp(0, 128) as f32;
    let numbering = st.session.prefs.get_bool("PageNumbering", true);
    let row_h = (tsize + 10.0).max(24.0);
    common::list_frame(ui, "pages-list", common::FOOTER_H, |ui| {
        for (i, pg) in doc.pages.iter().enumerate() {
            let sel = i == current;
            let (rect, resp) = common::row(ui, row_h, sel);
            ui.painter().vline(rect.left() + 28.5, rect.y_range(), Stroke::new(1.0, c.list_row_line));
            if pg.is_master == Some(true) {
                ui.painter().text(pos2(rect.left() + 14.0, rect.center().y), Align2::CENTER_CENTER, "M", theme.bold(theme.fonts.small), c.text);
            } else if pg.linked_to_master == Some(true) {
                crate::inspector::glyphs::paint(ui.painter(), Rect::from_center_size(pos2(rect.left() + 14.0, rect.center().y), vec2(14.0, 12.0)), Glyph::Link, &theme, true);
            }
            let thumb = Rect::from_min_size(pos2(rect.left() + 36.0, rect.top() + 5.0), vec2(tsize, tsize));
            if tsize > 0.0 {
                ui.painter().rect_filled(thumb, 0.0, Color32::WHITE);
                if let Some(tex) = thumbs::page_thumb(ui.ctx(), st, doc_id, doc_gen, &doc, i, (tsize as u32 * 2).max(16)) {
                    let ts = tex.size_vec2();
                    let s = (thumb.width() / ts.x).min(thumb.height() / ts.y);
                    ui.painter().image(tex.id(), Rect::from_center_size(thumb.center(), ts * s), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                }
                ui.painter().rect_stroke(thumb, 0.0, Stroke::new(if sel { 2.0 } else { 1.0 }, if sel { c.selection } else { c.list_border }), StrokeKind::Inside);
            }
            let name = pg.display_name().to_string();
            let nr = Rect::from_min_max(pos2(thumb.right() + 10.0, rect.center().y - 10.0), pos2(rect.right() - 6.0, rect.center().y + 10.0));
            if rename == Some(i) {
                if let Some(res) = common::inline_rename(ui, Id::new(("page-rename", i)), nr, &name) {
                    match res {
                        Some(nn) if valid_page_name(&nn) => actions.push(Action::Run("page.rename".into(), json!({"index": i, "name": nn}))),
                        Some(_) => actions.push(Action::Status(common::t("panel-pages-invalid-name"))),
                        None => {}
                    }
                    ui.data_mut(|dd| dd.remove::<usize>(Id::new(RENAME)));
                }
            } else {
                let font = if sel { theme.bold(theme.fonts.ui) } else { theme.ui_font() };
                let label = if numbering { format!("{:02}  {}", i + 1, name) } else { name.clone() };
                crate::widgets::text_clipped(ui.painter(), pos2(nr.left(), rect.center().y), Align2::LEFT_CENTER, &label, font, if sel { c.list_selected_text } else { c.text }, nr.width());
            }
            if resp.clicked() && i != current {
                actions.push(Action::SetPage(i));
            }
            if resp.double_clicked() {
                ui.data_mut(|dd| dd.insert_temp(Id::new(RENAME), i));
            }
        }
    });
    let text = fp_i18n::tr!("panel-pages-count", n = n);
    let alt = ui.input(|i| i.modifiers.alt);
    let (tip_new, tip_del) = (common::t("panel-pages-new"), common::t("panel-pages-delete"));
    let buttons = [
        FooterButton { glyph: Glyph::NewLayer, tip: &tip_new, enabled: true, action: Some(if alt { Action::Run("page.duplicate".into(), json!({})) } else { Action::Run("edit.insert.page".into(), json!({})) }) },
        FooterButton { glyph: Glyph::Trash, tip: &tip_del, enabled: n > 1, action: Some(Action::Run("page.delete".into(), json!({}))) },
    ];
    if let Some(a) = common::footer(ui, 30.0, &text, &buttons) {
        actions.push(a);
    }
    for a in actions {
        st.push(a);
    }
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

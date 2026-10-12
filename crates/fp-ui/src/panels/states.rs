//! Status-Bedienfeld (`CFrameInspectorPanel`, Vorlage 4126): Liste mit Nummer, Name und Verzögerung,
//! Umbenennen per Doppelklick, Verzögerung per Doppelklick auf die Zahl (Windoid 6041: 0..9999,
//! „Beim Export einschließen“), Fußleiste: Zwiebelschicht-Menü (6933), GIF-Schleife (6940),
//! Statusanzahl, Auf Status verteilen, Neu/Duplizieren, Löschen.

use super::common::{self, FooterButton};
use crate::app_state::{Action, AppState};
use crate::inspector::fields;
use crate::inspector::glyphs::{self, Glyph};
use crate::theme::Theme;
use egui::{pos2, vec2, Align2, Id, Rect, Sense, Stroke, Ui};
use serde_json::json;

const RENAME: &str = "states-rename";
const DELAY: &str = "states-delay";

/// Schleifen-Menü (6940): `FLPi`-Werte (Anzeige über [`loop_text`]).
pub const LOOPS: &[i32] = &[-1, 1, 2, 3, 4, 5, 10, 20, 0];

fn frame_info(st: &AppState, i: usize) -> fp_model::FrameInfo {
    st.session
        .active()
        .and_then(|d| d.doc.pages.get(d.page))
        .and_then(|p| p.root.export.as_ref())
        .and_then(|e| e.frames.get(i).cloned())
        .unwrap_or_default()
}

/// Text der Schleifenanzeige.
pub fn loop_text(v: i32) -> String {
    match v {
        0 => common::t("panel-states-forever"),
        v if v < 0 => common::t("panel-states-no-loop"),
        v => format!("{v}"),
    }
}

/// Zeichnet das Bedienfeld.
pub fn show(ui: &mut Ui, st: &mut AppState) {
    let theme = Theme::current(ui.ctx());
    let c = theme.colors.clone();
    let Some(d) = st.session.active() else {
        return;
    };
    let n = d.frame_count();
    let current = d.frame;
    let loop_count = d.doc.pages.get(d.page).map(|p| p.loop_count()).unwrap_or(0);
    let (ob, oa) = d.doc.pages.get(d.page).map(|p| (p.root.onion_before, p.root.onion_after)).unwrap_or((0, 0));
    let mut actions = Vec::new();
    let rename: Option<usize> = ui.data(|dd| dd.get_temp(Id::new(RENAME)));
    let delay_edit: Option<usize> = ui.data(|dd| dd.get_temp(Id::new(DELAY)));
    let infos: Vec<fp_model::FrameInfo> = (0..n).map(|i| frame_info(st, i)).collect();
    let names: Vec<String> = (0..n).map(|i| super::state_name(st, i)).collect();
    let disposal_nodes = common::resource_menu_nodes(st, "6922");
    let numbering = st.session.prefs.get_bool("FrameNumbering", true);
    common::list_frame(ui, "states-list", common::FOOTER_H, |ui| {
        for i in 0..n {
            let sel = i == current;
            let (rect, resp) = common::row(ui, 22.0, sel);
            let info = &infos[i];
            // Zwiebelschicht-Spalte
            let in_onion = (ob > 0 || oa > 0) && i != current && (i + ob as usize >= current) && (i <= current + oa as usize);
            if in_onion {
                ui.painter().vline(rect.left() + 10.0, rect.y_range().shrink(3.0), Stroke::new(2.0, c.focus));
            }
            if !info.visible {
                ui.painter().text(pos2(rect.left() + 10.0, rect.center().y), Align2::CENTER_CENTER, "×", theme.ui_font(), c.icon_red);
            }
            let font = if sel { theme.bold(theme.fonts.ui) } else { theme.ui_font() };
            if numbering {
                ui.painter().text(pos2(rect.left() + 22.0, rect.center().y), Align2::LEFT_CENTER, format!("{:02}", i + 1), font.clone(), c.text);
            }
            let nr = Rect::from_min_max(pos2(rect.left() + 46.0, rect.top() + 2.0), pos2(rect.right() - 44.0, rect.bottom() - 2.0));
            if rename == Some(i) {
                if let Some(res) = common::inline_rename(ui, Id::new(("state-rename", i)), nr, &names[i]) {
                    if let Some(nn) = res {
                        actions.push(Action::Run("pi.state.props".into(), json!({"indices": [i], "name": nn})));
                    }
                    ui.data_mut(|dd| dd.remove::<usize>(Id::new(RENAME)));
                }
            } else {
                crate::widgets::text_clipped(ui.painter(), pos2(nr.left(), rect.center().y), Align2::LEFT_CENTER, &names[i], font.clone(), c.text, nr.width());
            }
            let dr = Rect::from_min_max(pos2(rect.right() - 42.0, rect.top()), pos2(rect.right() - 2.0, rect.bottom()));
            ui.painter().text(pos2(dr.right() - 4.0, rect.center().y), Align2::RIGHT_CENTER, info.delay.to_string(), theme.ui_font(), c.text_weak);
            if resp.clicked() && i != current {
                actions.push(Action::SetFrame(i));
            }
            // Kontextmenü 6922 (Löschmethode) für den angeklickten Status
            if resp.secondary_clicked() && i != current {
                actions.push(Action::SetFrame(i));
            }
            resp.context_menu(|ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                for node in &disposal_nodes {
                    if let crate::menu::Node::Item { label, action: crate::menu::ItemAction::Command(cmd), enabled, checked, .. } = node
                        && fields::list_item(ui, label, *checked == Some(true)).clicked()
                        && *enabled
                    {
                        actions.push(Action::Run(cmd.clone(), json!({})));
                        ui.close();
                    }
                }
            });
            if resp.double_clicked() {
                if resp.interact_pointer_pos().is_some_and(|p| dr.contains(p)) {
                    ui.data_mut(|dd| dd.insert_temp(Id::new(DELAY), i));
                } else {
                    ui.data_mut(|dd| dd.insert_temp(Id::new(RENAME), i));
                }
            }
        }
    });
    // Windoid „Statusverzögerung“ (6041)
    if let Some(i) = delay_edit {
        let mut open = true;
        let info = infos.get(i).cloned().unwrap_or_default();
        egui::Window::new(common::t("panel-states-delay-title")).id(Id::new("state-delay-win")).collapsible(false).resizable(false).open(&mut open).show(ui.ctx(), |ui| {
            ui.horizontal(|ui| {
                ui.label(common::t("panel-states-delay-label"));
                let mut v = info.delay;
                if ui.add(egui::DragValue::new(&mut v).range(0..=9999)).changed() {
                    actions.push(Action::Run("pi.state.props".into(), json!({"indices": [i], "delay": v})));
                }
                ui.label(common::t("panel-states-delay-unit"));
            });
            let mut vis = info.visible;
            if ui.checkbox(&mut vis, common::t("panel-states-include-export")).changed() {
                actions.push(Action::Run("pi.state.props".into(), json!({"indices": [i], "visible": vis})));
            }
        });
        if !open {
            ui.data_mut(|dd| dd.remove::<usize>(Id::new(DELAY)));
        }
    }
    // Fußleiste: Zwiebelschicht, Schleife, Anzahl, Knöpfe
    let rect = ui.available_rect_before_wrap();
    let y = rect.bottom() - common::FOOTER_H + 3.0;
    let onion_r = Rect::from_min_size(pos2(rect.left() + 2.0, y), vec2(19.0, 16.0));
    let loop_r = Rect::from_min_size(pos2(rect.left() + 22.0, y), vec2(19.0, 16.0));
    let onion = ui.interact(onion_r, Id::new("states-onion"), Sense::click()).on_hover_text(common::t("panel-states-onion"));
    glyphs::paint(ui.painter(), onion_r.shrink(1.0), Glyph::Onion, &theme, true);
    let lp = ui.interact(loop_r, Id::new("states-loop"), Sense::click()).on_hover_text(common::t("panel-states-loop"));
    glyphs::paint(ui.painter(), loop_r.shrink(1.0), Glyph::Loop, &theme, true);
    crate::ui_kit::icon_button_info(&onion, true, &common::t("panel-states-onion"), None);
    crate::ui_kit::icon_button_info(&lp, true, &common::t("panel-states-loop"), None);
    common::resource_menu(ui, &onion, st, "6933");
    common::resource_menu(ui, &lp, st, "6940");
    let text = loop_text(loop_count);
    let (tip_dist, tip_new, tip_del) = (common::t("panel-states-distribute"), common::t("panel-states-new"), common::t("panel-states-delete"));
    let buttons = [
        FooterButton { glyph: Glyph::Distribute, tip: &tip_dist, enabled: st.session.active().is_some_and(|d| d.selection.elements.len() > 1), action: Some(Action::Run("pi.state.distribute".into(), json!({}))) },
        FooterButton { glyph: Glyph::NewLayer, tip: &tip_new, enabled: true, action: Some(if ui.input(|i| i.modifiers.alt) { Action::Run("state.duplicate".into(), json!({})) } else { Action::Run("state.add".into(), json!({})) }) },
        FooterButton { glyph: Glyph::Trash, tip: &tip_del, enabled: n > 1, action: Some(Action::Run("state.delete".into(), json!({}))) },
    ];
    if let Some(a) = common::footer(ui, 46.0, &text, &buttons) {
        actions.push(a);
    }
    for a in actions {
        st.push(a);
    }
}

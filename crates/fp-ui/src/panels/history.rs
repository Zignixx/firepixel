//! Verlauf-Bedienfeld (`CHistoryPanel`, Vorlage 4152): Schrittliste mit Schieber links (Undo-Grenze,
//! Klick/Ziehen springt), Auswahl von Schritten (Umschalt/Strg), Wiederholen (4171), Schritte
//! kopieren (4212, als Skript in die Zwischenablage), Als Befehl speichern (4214, `.jsf`-Datei).
//! Nicht aufzeichenbare Schritte tragen ein rotes ×.

use super::common::{self, FooterButton};
use crate::app_state::{Action, AppState};
use crate::inspector::glyphs::Glyph;
use crate::theme::Theme;
use crate::tips::HoverTip;
use egui::{Align2, Id, Rect, Stroke, Ui, pos2, vec2};
use serde_json::json;
use std::collections::BTreeSet;

const SEL: &str = "history-sel";

/// Tooltip of the red × of a row that cannot be replayed (FB-92): a row bound to the step before
/// it (`attached`) has no undo step of its own either.
pub fn marker_tip(attached: bool) -> String {
    common::t(if attached {
        "panel-history-attached-tip"
    } else {
        "panel-history-not-replayable-tip"
    })
}

/// Zeichnet das Bedienfeld.
pub fn show(ui: &mut Ui, st: &mut AppState) {
    let theme = Theme::current(ui.ctx());
    let c = theme.colors.clone();
    // FB-46: a press here first discards an open pop-up slider (see `discard_on_history_press`)
    crate::inspector::preview::note_history_rect(ui.ctx(), ui.max_rect());
    let Some(d) = st.session.active() else {
        return;
    };
    let steps: Vec<(String, bool)> = d
        .history
        .steps()
        .iter()
        .map(|s| (st.session.step_label(s), s.recordable))
        .collect();
    // rows bound to the step before them ("Brush Stroke" / "Stroke Color", FB-92)
    let attached: Vec<bool> = d.history.steps().iter().map(|s| s.attached()).collect();
    let pos = d.history.undo_count();
    let mut sel: BTreeSet<usize> = ui.data(|dd| dd.get_temp(Id::new(SEL))).unwrap_or_default();
    sel.retain(|i| *i < steps.len());
    let mut goto = None;
    let mods = ui.input(|i| i.modifiers);
    common::list_frame(ui, "history-list", common::FOOTER_H, |ui| {
        // Anfangszustand (Position 0)
        for (i, (label, rec)) in steps.iter().enumerate() {
            let done = i < pos;
            let selected = sel.contains(&i);
            let (rect, mut resp) = common::row(ui, 21.0, selected);
            // Schieberspalte
            let slider = Rect::from_min_size(rect.min, vec2(16.0, rect.height()));
            ui.painter().vline(
                slider.center().x,
                rect.y_range(),
                Stroke::new(1.0, c.list_row_line),
            );
            if i + 1 == pos {
                let m = slider.center();
                ui.painter().add(egui::Shape::convex_polygon(
                    vec![
                        pos2(m.x - 5.0, m.y + 5.0),
                        pos2(m.x + 5.0, m.y + 5.0),
                        pos2(m.x, m.y - 1.0),
                    ],
                    c.icon_fg,
                    Stroke::NONE,
                ));
            }
            let ir =
                Rect::from_center_size(pos2(rect.left() + 28.0, rect.center().y), vec2(14.0, 14.0));
            crate::inspector::glyphs::paint(ui.painter(), ir, Glyph::StyleNew, &theme, done);
            if !rec {
                let mr = ui.painter().text(
                    ir.right_bottom(),
                    Align2::RIGHT_BOTTOM,
                    "×",
                    theme.bold(11.0),
                    c.icon_red,
                );
                // FB-92: the marker and the row explain the red × on hover; an explanation, shown
                // also with View > Tooltips off (FB-99)
                let tip = marker_tip(attached.get(i).copied().unwrap_or(false));
                let m = ui.interact(mr, Id::new("history-marker").with(i), egui::Sense::hover());
                if m.hovered() {
                    m.explain_tip(&tip);
                } else {
                    resp = resp.explain_tip(&tip);
                }
            }
            let col = if done { c.text } else { c.text_disabled };
            crate::widgets::text_clipped(
                ui.painter(),
                pos2(rect.left() + 42.0, rect.center().y),
                Align2::LEFT_CENTER,
                label,
                theme.ui_font(),
                col,
                rect.width() - 46.0,
            );
            if resp.clicked() {
                if resp
                    .interact_pointer_pos()
                    .is_some_and(|p| slider.contains(p))
                {
                    goto = Some(i + 1);
                } else if mods.shift {
                    let anchor = sel.iter().next().copied().unwrap_or(i);
                    for k in anchor.min(i)..=anchor.max(i) {
                        sel.insert(k);
                    }
                } else if mods.command {
                    if !sel.remove(&i) {
                        sel.insert(i);
                    }
                } else {
                    sel.clear();
                    sel.insert(i);
                }
            }
            if resp.dragged()
                && resp
                    .interact_pointer_pos()
                    .is_some_and(|p| p.x < rect.left() + 16.0)
            {
                goto = Some(i + 1);
            }
        }
    });
    ui.data_mut(|dd| dd.insert_temp(Id::new(SEL), sel.clone()));
    st.dialogs.panel_sel.history = sel.clone();
    if let Some(p) = goto {
        // FB-46: a pending pop-up value is discarded, never applied after the jump
        crate::inspector::preview::discard_pending(ui.ctx(), st);
        if let Err(e) = st.session.history_go_to(p) {
            st.push(Action::Status(e.to_string()));
        }
        crate::inspector::preview::publish_target(ui.ctx(), st);
    }
    let any = !sel.is_empty();
    let replay = sel.iter().all(|i| steps.get(*i).is_some_and(|s| s.1));
    let cmds = |st: &AppState| -> Vec<fp_engine::RecordedCommand> {
        st.session
            .active()
            .map(|d| {
                sel.iter()
                    .flat_map(|i| d.history.export(*i..*i + 1))
                    .collect()
            })
            .unwrap_or_default()
    };
    let rec = cmds(st);
    let (tip_copy, tip_save) = (
        common::t("panel-history-copy-steps"),
        common::t("panel-history-save-command"),
    );
    let buttons = [
        FooterButton {
            glyph: Glyph::Copy,
            tip: &tip_copy,
            enabled: any && replay,
            action: Some(Action::Status("copy-steps".into())),
        },
        FooterButton {
            glyph: Glyph::Save,
            tip: &tip_save,
            enabled: any && replay,
            action: Some(Action::Status("save-steps".into())),
        },
    ];
    // „Wiederholen“ als Textknopf links (4171)
    let rect = ui.available_rect_before_wrap();
    let rr = Rect::from_min_size(
        pos2(rect.left() + 3.0, rect.bottom() - common::FOOTER_H + 3.0),
        vec2(72.0, 16.0),
    );
    if crate::inspector::fields::button(
        ui,
        "history-replay",
        rr,
        &common::t("panel-history-replay"),
        any && replay,
    ) {
        st.run(
            "history.replay",
            json!({"commands": rec.clone(), "label": common::t("panel-history-replay-label")}),
        );
    }
    match common::footer(ui, 80.0, "", &buttons) {
        Some(Action::Status(s)) if s == "copy-steps" => {
            ui.ctx().copy_text(fp_engine::history::to_script(&rec));
            st.push(Action::Status(common::t("panel-history-copied")));
        }
        Some(Action::Status(s)) if s == "save-steps" => {
            // wie im Original: Namensabfrage, Ablage im Befehlsordner (Befehle-Menü)
            let steps: Vec<usize> = sel.iter().copied().collect();
            crate::dialogs::panel_menus::open_save_command(st, &steps);
        }
        Some(a) => st.push(a),
        None => {}
    }
}

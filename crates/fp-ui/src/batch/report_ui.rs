//! Ende-Karte, Plan (Trockenlauf) und Fehlerliste (SPEC-BATCH §2.6, §2.8): Zusammenfassung mit
//! Zählern, Summe vorher/nachher, Dauer; Knöpfe „Ordner öffnen“, „Bericht speichern…“,
//! „Fehlgeschlagene erneut“, „Neues Rezept“; Tabelle Quelle → Ziel mit Hinweisen; Fehler mit
//! Datei, Schritt, Meldung und „Kopieren“.

use super::kit::{self, BtnKind};
use super::{EntryStatus, Inbox, Studio};
use crate::app_state::AppState;
use crate::theme::Theme;
use crate::ui_kit::{self, sp, type_scale, Column, KitIcon, RowIcon, Tone};
use egui::{pos2, vec2, Align2, Id, Rect, Ui};
use fp_batch::{PlanAction, Report};
use fp_i18n::t;
use std::path::Path;

/// Eine Zeile für die Ausgabe-Leiste nach dem Lauf.
pub fn short_summary(r: &Report) -> String {
    let tt = &r.totals;
    let mut s = fp_i18n::tr!("batch-ui-done-counts", done = tt.done, skipped = tt.skipped, failed = tt.failed);
    if tt.bytes_in > 0 && tt.bytes_out > 0 {
        s = format!("{s} · {} → {}", fp_i18n::fmt_size(tt.bytes_in), fp_i18n::fmt_size(tt.bytes_out));
    }
    s
}

/// Bericht speichern (Inbox-Antwort des Datei-Dialogs).
pub fn save_report(s: &mut Studio, path: &Path, now: f64) {
    let Some(r) = &s.report else { return };
    match r.write(path) {
        Ok(()) => s.notify(Tone::Success, fp_i18n::tr!("batch-ui-report-saved", name = path.display().to_string()), now),
        Err(e) => s.notify(Tone::Error, e.to_string(), now),
    }
}

/// Zusammenfassung nach dem Lauf (über der Vorschau).
pub fn summary_card(ui: &mut Ui, st: &mut AppState, s: &mut Studio, r: Rect, now: f64) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let p = ui.painter().clone();
    let Some(rep) = s.report.clone() else { return };
    let tt = &rep.totals;
    let mut y = r.top() + sp::S;
    let (icon, title) = if rep.cancelled {
        (KitIcon::Warning, t("batch-ui-done-cancelled"))
    } else if tt.failed > 0 {
        (KitIcon::Error, t("batch-ui-done-errors"))
    } else {
        (KitIcon::Success, t("batch-ui-done-title"))
    };
    let ir = Rect::from_min_size(pos2(r.left(), y), vec2(32.0, 32.0));
    ui_kit::paint_icon(&p, ir, icon, &crate::icons::IconStyle::new(&theme, None, true));
    kit::text_left(&p, ir.right() + sp::M, ir.center().y, r.width() - 44.0, &title, theme.bold(type_scale::TITLE), c.text);
    y = ir.bottom() + sp::M;
    let rows: Vec<(KitIcon, String)> = vec![
        (KitIcon::Success, fp_i18n::tr!("batch-ui-done-ok", n = tt.done)),
        (KitIcon::Warning, fp_i18n::tr!("batch-ui-done-skipped", n = tt.skipped)),
        (KitIcon::Error, fp_i18n::tr!("batch-ui-done-failed", n = tt.failed)),
    ];
    for (ic, txt) in rows {
        let rr = Rect::from_min_size(pos2(r.left() + 6.0, y + 2.0), vec2(14.0, 14.0));
        ui_kit::paint_icon(&p, rr, ic, &crate::icons::IconStyle::new(&theme, None, true));
        kit::text_left(&p, rr.right() + sp::S, rr.center().y, r.width() - 30.0, &txt, theme.font(type_scale::BODY), c.text);
        y += 20.0;
    }
    y += sp::S;
    if tt.bytes_in > 0 {
        let txt = format!("{} → {} ({})", fp_i18n::fmt_size(tt.bytes_in), fp_i18n::fmt_size(tt.bytes_out), super::preview::percent(tt.bytes_in, tt.bytes_out));
        kit::text_left(&p, r.left(), y + 8.0, r.width(), &txt, theme.bold(type_scale::BODY), c.text);
        y += 20.0;
    }
    let dur = fp_i18n::tr!("batch-ui-done-duration", s = fp_i18n::fmt_number(tt.ms as f64 / 1000.0, 1, 1));
    kit::text_left(&p, r.left(), y + 8.0, r.width(), &dur, theme.font(type_scale::CAPTION + 0.5), kit::weak(&theme));
    y += 20.0;
    if let Some(dir) = rep.output_dirs.first() {
        let txt = dir.display().to_string();
        let shown = ui_kit::elide_middle(ui.ctx(), &txt, &theme.font(type_scale::CAPTION), r.width());
        p.text(pos2(r.left(), y + 8.0), Align2::LEFT_CENTER, shown, theme.font(type_scale::CAPTION), kit::weak(&theme));
        y += 20.0;
    }
    y += sp::M;
    // Knöpfe (zwei Spalten)
    let labels = [t("batch-ui-done-open-folder"), t("batch-ui-done-save-report"), t("batch-ui-done-retry"), t("batch-ui-done-new"), t("batch-ui-done-close")];
    let bw = ((r.width() - sp::S) / 2.0).floor().max(60.0);
    let can_open = fp_engine::shell::allowed() && !rep.output_dirs.is_empty();
    let enabled = [can_open, true, tt.failed > 0, true, true];
    for (k, l) in labels.iter().enumerate() {
        let col = (k % 2) as f32;
        let row = (k / 2) as f32;
        let br = Rect::from_min_size(pos2(r.left() + col * (bw + sp::S), y + row * (kit::BTN_H + sp::S)), vec2(bw, kit::BTN_H));
        if br.bottom() > r.bottom() + 0.5 {
            break;
        }
        let kind = if k == 4 { BtnKind::Ghost } else { BtnKind::Secondary };
        if kit::button(ui, br, Id::new(("fp-batch-done-btn", k)), l, None, kind, enabled[k], l).clicked() {
            match k {
                0 => {
                    if let Some(d) = rep.output_dirs.first() {
                        let _ = fp_engine::shell::shell_open(d, None);
                    }
                }
                1 => {
                    let inbox = s.inbox_sender();
                    crate::files::pick_save(st, &t("batch-ui-done-save-report"), fp_batch::report::REPORT_FILE, rep.output_dirs.first().map(|p| p.as_path()), move |_st, path| {
                        if let Some(p) = path {
                            inbox.lock().unwrap_or_else(|e| e.into_inner()).push(Inbox::SaveReport(p));
                        }
                    });
                }
                2 => {
                    // nur die fehlgeschlagenen erneut: diese aktivieren, alle anderen aus
                    for f in &mut s.files {
                        f.enabled = f.status == EntryStatus::Failed;
                    }
                    s.files_changed();
                    s.show_summary = false;
                    super::output_bar::start(st, now);
                    return;
                }
                3 => {
                    s.set_recipe(fp_batch::Recipe::default(), None);
                    s.show_summary = false;
                }
                _ => s.show_summary = false,
            }
        }
    }
}

/// Reiter „Plan“: Tabelle Quelle → Ziel, Hinweise (Trockenlauf).
pub fn plan_view(ui: &mut Ui, s: &mut Studio, r: Rect) {
    let theme = Theme::current(ui.ctx());
    let p = ui.painter().clone();
    let Some(plan) = s.plan.clone() else {
        let title = t("batch-ui-plan-empty");
        let hint = s.plan_error.clone().unwrap_or_else(|| t("batch-ui-plan-empty-hint"));
        let h = ui_kit::empty_state_height(ui.ctx(), &title, Some(&hint), r.width());
        ui_kit::empty_state_at(&p, Rect::from_min_size(r.min + vec2(0.0, sp::L), vec2(r.width(), h)), KitIcon::EmptyTray, &title, Some(&hint), &theme);
        return;
    };
    let summary = fp_i18n::tr!("batch-ui-plan-summary", n = plan.entries.len(), conflicts = plan.conflicts);
    kit::text_left(&p, r.left(), r.top() + 9.0, r.width(), &summary, theme.font(type_scale::CAPTION + 0.5), kit::weak(&theme));
    let tr = Rect::from_min_max(pos2(r.left(), r.top() + 22.0), r.max);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(tr).layout(egui::Layout::top_down(egui::Align::Min)));
    let w = tr.width();
    let cw = ((w - 90.0) / 2.0).max(60.0);
    let (h1, h2, h3) = (t("batch-ui-plan-source"), t("batch-ui-plan-target"), t("batch-ui-plan-note"));
    let cols = [Column::new(&h1, cw), Column::new(&h2, cw), Column::rest(&h3)];
    let entries = plan.entries.clone();
    ui_kit::table(&mut child, "fp-batch-plan", &cols, entries.len(), 20.0, None, tr.height(), |ui, row, col| {
        let e = &entries[row];
        let name = |p: &Path| p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        match col {
            0 => {
                ui_kit::cell_text(ui, &name(&e.source), false).on_hover_text(e.source.display().to_string());
            }
            1 => {
                let txt = if e.action == PlanAction::Skip { "—".to_string() } else { name(&e.target) };
                ui_kit::cell_text(ui, &txt, false).on_hover_text(e.target.display().to_string());
            }
            _ => {
                let mut notes = e.hint_messages();
                if let Some(o) = e.conflict_with {
                    notes.insert(0, fp_i18n::tr!("batch-ui-plan-conflict-with", n = o + 1));
                }
                let txt = notes.join(" · ");
                let resp = ui_kit::cell_text(ui, &txt, true);
                if !txt.is_empty() {
                    resp.on_hover_text(txt);
                }
            }
        }
    });
}

/// Reiter „Fehler“: Datei, Meldung, Kopieren.
pub fn errors_view(ui: &mut Ui, s: &mut Studio, r: Rect) {
    let theme = Theme::current(ui.ctx());
    let failed: Vec<(String, String)> = s.files.iter().filter(|f| f.status == EntryStatus::Failed).map(|f| (f.info.file_name(), f.message.clone().unwrap_or_default())).collect();
    let copy_l = t("batch-ui-errors-copy");
    let bw = kit::btn_w(ui.ctx(), &copy_l, false);
    let br = Rect::from_min_size(pos2(r.right() - bw, r.top()), vec2(bw, kit::BTN_H));
    if kit::button(ui, br, Id::new("fp-batch-errors-copy"), &copy_l, None, BtnKind::Secondary, !failed.is_empty(), &copy_l).clicked() {
        let txt: Vec<String> = failed.iter().map(|(a, b)| format!("{a}\t{b}")).collect();
        ui.ctx().copy_text(txt.join("\n"));
    }
    let tr = Rect::from_min_max(pos2(r.left(), br.bottom() + sp::S), r.max);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(tr).layout(egui::Layout::top_down(egui::Align::Min)));
    let (h1, h2) = (t("batch-ui-errors-file"), t("batch-ui-errors-message"));
    let cols = [Column::new(&h1, (tr.width() * 0.38).max(80.0)), Column::rest(&h2)];
    ui_kit::table(&mut child, "fp-batch-errors", &cols, failed.len(), 20.0, None, tr.height(), |ui, row, col| {
        let (a, b) = &failed[row];
        let txt = if col == 0 { a } else { b };
        ui_kit::cell_text(ui, txt, col == 1).on_hover_text(txt.as_str());
    });
    let _ = theme;
    let _ = RowIcon::None;
}

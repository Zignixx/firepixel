//! Endkarte „Dateiname“ (SPEC-BATCH §4, BATCH-3): Anfänger-Baukasten aus Bausteinen (Chips: ziehen,
//! × entfernen, anklicken bearbeiten, Strg+←/→ verschieben), Profi-Modus mit regulärem Ausdruck
//! (Klartext-Erklärung des Musters und des Ersetzen-Felds, verständliche Fehlermeldung mit Stelle,
//! Spickzettel), gemeinsame Optionen (Text ersetzen, Leerzeichen, Präfix/Suffix, Schreibweise,
//! Umlaute, Sonderzeichen, Endung), Vorlagen und die **Live-Tabelle alt → neu** mit zeichenweiser
//! Hervorhebung und Hinweisen (Konflikt, ungültige Zeichen, gekürzt, unverändert …).
//!
//! Angemeldet als Kartenformular `rename` ([`register`]); die Logik liegt in `fp_batch::naming`
//! (Namensbildung, Plan) und `fp_batch::explain` (Erklärung, Übersetzung Baukasten → Regex).

use super::Mode;
use super::kit::{self, BtnKind};
use super::params::CardCtx;
use crate::theme::{Theme, mix, with_alpha};
use crate::tips::HoverTip;
use crate::ui_kit::{self, KitIcon, Tone, sp, type_scale};
use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{
    Align2, Color32, CornerRadius, Id, Key, Modifiers, Rect, Sense, Stroke, StrokeKind, Ui, pos2,
    vec2,
};
use fp_batch::explain::{self, DiffPart};
use fp_batch::naming::{self, NameHint, PlanOptions};
use fp_batch::recipe::{
    CaseMode, DateSource, ExifField, ExtCase, NamePart, OriginalTake, RenameMode, RenameSpec,
    Replacement, SizeWhich,
};
use fp_batch::{ItemInfo, Recipe, StepRegistry};
use fp_i18n::{t, tr};
use std::sync::Arc;

/// Zeilen der Live-Tabelle (Rest: „… und n weitere“, alle im Trockenlauf).
pub const TABLE_ROWS: usize = 200;
/// Bis zu so vielen Dateien werden Konflikte und Änderungen für die Zusammenfassung gezählt.
pub const COUNT_LIMIT: usize = 1000;
const CHIP_H: f32 = 24.0;
const ROW_H: f32 = 20.0;
/// Datumsvorlagen (§4.1).
pub const DATE_FORMATS: [&str; 4] = [
    "YYYY-MM-DD",
    "YYYYMMDD",
    "YYYY-MM-DD_HH-mm-ss",
    "DD.MM.YYYY",
];
/// Platzhalter des Ersetzen-Felds (technische Kürzel, nicht übersetzt).
pub const PLACEHOLDERS: &str = "$1  ${name}  {n:3}  {n:3:10:5}  {date:YYYY-MM-DD}  {exif-date:YYYYMMDD}  {w}  {h}  {folder}  {ext}  {name}  {format}";

/// Meldet die Karte beim Studio an (einmal je Prozess genügt, mehrfach schadet nicht).
pub fn register() {
    super::params::register_card_body(fp_batch::steps::files::RENAME, body);
}

/// Kartenformular „Dateiname“. Liefert `true` bei Änderung (ein Rückgängig-Schritt je Geste).
pub fn body(ui: &mut Ui, ctx: &mut CardCtx) -> bool {
    let mut spec: RenameSpec = serde_json::from_value(ctx.params.clone()).unwrap_or_default();
    let before = spec.clone();
    let mode = ctx.mode;
    let enabled = ctx.enabled;
    let mut show_more = *ctx.show_more;
    ui.add_enabled_ui(enabled, |ui| {
        header(ui, &mut spec, mode);
        ui.add_space(sp::XS);
        if spec.mode == RenameMode::Regex {
            if mode == Mode::Pro {
                regex_section(ui, &mut spec);
            } else {
                beginner_regex_note(ui, &mut spec);
            }
        } else {
            builder_section(ui, &mut spec, mode);
        }
        options_section(ui, &mut spec, mode, &mut show_more);
    });
    *ctx.show_more = show_more;
    table_section(ui, &mut spec, ctx.files, ctx.recipe, enabled);
    if spec == before {
        return false;
    }
    match serde_json::to_value(&spec) {
        Ok(v) => {
            *ctx.params = v;
            true
        }
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------- Kopf: Art und Vorlagen

/// Wechselt Baukasten ↔ Regex. Beim Wechsel zum Regex wird der Baukasten in ein gleichwertiges
/// Muster übersetzt, solange das Muster leer bzw. noch eine unveränderte Übersetzung ist (eigene
/// Muster gehen nie verloren; dafür gibt es „Aus Baukasten übernehmen“).
pub fn switch_mode(spec: &mut RenameSpec, regex: bool) {
    if regex {
        if spec.mode != RenameMode::Regex && explain::regex_is_replaceable(spec) {
            spec.regex = explain::builder_to_regex(spec);
        }
        spec.mode = RenameMode::Regex;
    } else {
        spec.mode = RenameMode::Builder;
    }
}

fn header(ui: &mut Ui, spec: &mut RenameSpec, mode: Mode) {
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, 24.0), Sense::hover());
    let pl = t("batch-rn-presets");
    let pw = (kit::btn_w(ui.ctx(), &pl, true)).min(w * 0.45);
    let pr = Rect::from_min_size(pos2(r.right() - pw, r.top()), vec2(pw, 24.0));
    if mode == Mode::Pro {
        let labels = vec![
            t("batch-ui-rename-mode-builder"),
            t("batch-ui-rename-mode-regex"),
        ];
        let cur = usize::from(spec.mode == RenameMode::Regex);
        let seg = Rect::from_min_max(
            r.min,
            pos2(
                (pr.left() - sp::S)
                    .min(r.left() + 300.0)
                    .max(r.left() + 60.0),
                r.bottom(),
            ),
        );
        if let Some(k) = kit::segmented(
            ui,
            seg,
            Id::new("fp-batch-rename-mode"),
            &labels,
            cur,
            false,
        ) {
            switch_mode(spec, k == 1);
        }
    }
    let resp = kit::button(
        ui,
        pr,
        Id::new("fp-batch-rename-presets"),
        &pl,
        Some(ui_kit::RowIcon::Icon(crate::icons::Icon::DropDown)),
        BtnKind::Ghost,
        ui.is_enabled(),
        &pl,
    )
    .hover_tip(t("batch-rn-presets.tooltip"));
    egui::Popup::menu(&resp)
        .id(Id::new("fp-batch-rename-presets-menu"))
        .show(|ui| {
            crate::theme::field_widgets(ui);
            ui.set_min_width(240.0);
            for (key, f) in PRESETS {
                if ui
                    .button(t(key))
                    .hover_tip(t(&format!("{key}.tooltip")))
                    .clicked()
                {
                    f(spec);
                    set_selected(ui.ctx(), None);
                }
            }
        });
}

type Preset = (&'static str, fn(&mut RenameSpec));

/// Vorlagen der Karte („nach Datum“ u. a.).
pub const PRESETS: [Preset; 5] = [
    ("batch-rn-preset-exif-date", preset_exif_date),
    ("batch-rn-preset-number", preset_number),
    ("batch-rn-preset-date-prefix", preset_date_prefix),
    ("batch-rn-preset-web", preset_web),
    ("batch-rn-preset-reset", preset_reset),
];

/// Fotos nach Aufnahmedatum: `{exif-date:YYYY-MM-DD}_{n:3}` → `2026-10-13_001`.
pub fn preset_exif_date(spec: &mut RenameSpec) {
    spec.mode = RenameMode::Builder;
    spec.parts = vec![
        NamePart::Date {
            source: DateSource::Exif,
            format: "YYYY-MM-DD".into(),
        },
        NamePart::Text { text: "_".into() },
        NamePart::Counter {
            start: 1,
            step: 1,
            digits: 3,
            per_folder: false,
        },
    ];
}

/// Durchnummerieren: `Bild_001`.
pub fn preset_number(spec: &mut RenameSpec) {
    spec.mode = RenameMode::Builder;
    spec.parts = vec![
        NamePart::Text {
            text: t("batch-rn-preset-number-text"),
        },
        NamePart::Text { text: "_".into() },
        NamePart::Counter {
            start: 1,
            step: 1,
            digits: 3,
            per_folder: false,
        },
    ];
}

/// Änderungsdatum voranstellen: `2026-10-13_Name`.
pub fn preset_date_prefix(spec: &mut RenameSpec) {
    spec.mode = RenameMode::Builder;
    spec.parts = vec![
        NamePart::Date {
            source: DateSource::Modified,
            format: "YYYY-MM-DD".into(),
        },
        NamePart::Text { text: "_".into() },
        NamePart::Original {
            take: OriginalTake::Whole,
            count: 0,
        },
    ];
}

/// Web-tauglich: klein, Leerzeichen → `-`, Umlaute umschreiben, Sonderzeichen entfernen.
pub fn preset_web(spec: &mut RenameSpec) {
    spec.case = CaseMode::Lower;
    spec.spaces = Some("-".into());
    spec.transliterate = true;
    spec.strip_special = true;
    spec.extension = ExtCase::Lower;
}

/// Zurücksetzen: Name bleibt gleich.
pub fn preset_reset(spec: &mut RenameSpec) {
    let extra = std::mem::take(&mut spec.extra);
    *spec = RenameSpec {
        extra,
        ..RenameSpec::default()
    };
}

// ---------------------------------------------------------------------- Baukasten

fn sel_id() -> Id {
    Id::new("fp-batch-rename-selected-part")
}

/// Gewählter Baustein (zum Bearbeiten).
pub fn selected(ctx: &egui::Context) -> Option<usize> {
    ctx.data(|d| d.get_temp::<Option<usize>>(sel_id()))
        .flatten()
}

pub fn set_selected(ctx: &egui::Context, v: Option<usize>) {
    ctx.data_mut(|d| d.insert_temp(sel_id(), v));
}

fn chip_id(i: usize) -> Id {
    Id::new(("fp-batch-rename-chip", i))
}

/// Verschiebt einen Baustein (`to` = Einfügeposition vor dem Entfernen). Liefert die neue Position.
pub fn move_part(spec: &mut RenameSpec, from: usize, to: usize) -> Option<usize> {
    if from >= spec.parts.len() {
        return None;
    }
    let to = to.min(spec.parts.len());
    let dest = if to > from { to - 1 } else { to };
    if dest == from {
        return None;
    }
    let p = spec.parts.remove(from);
    spec.parts.insert(dest, p);
    Some(dest)
}

/// Beschriftung eines Bausteins (Chip).
pub fn chip_label(p: &NamePart) -> String {
    let now = naming::now_local();
    match p {
        NamePart::Original { take, count } => match take {
            OriginalTake::Whole => t("batch-rn-part-original"),
            OriginalTake::First => tr!("batch-rn-part-original-first", n = *count as i64),
            OriginalTake::Last => tr!("batch-rn-part-original-last", n = *count as i64),
            OriginalTake::NoTrailingDigits => t("batch-rn-part-original-trim"),
        },
        NamePart::Text { text } if text.is_empty() => t("batch-rn-part-text-empty"),
        NamePart::Text { text } => tr!("batch-rn-part-text", text = text.replace(' ', "␣")),
        NamePart::Counter { start, digits, .. } => tr!(
            "batch-rn-part-counter",
            example = format!("{:0w$}", start, w = (*digits).clamp(1, 9) as usize)
        ),
        NamePart::Date { source, format } => {
            let key = match source {
                DateSource::Today => "batch-rn-part-today",
                DateSource::Modified => "batch-rn-part-date",
                DateSource::Exif => "batch-rn-part-exif-date",
            };
            tr!(key, example = naming::format_date(&now, format))
        }
        NamePart::Size { which } => t(match which {
            SizeWhich::Both => "batch-rn-part-size",
            SizeWhich::Width => "batch-rn-part-width",
            SizeWhich::Height => "batch-rn-part-height",
        }),
        NamePart::Folder => t("batch-rn-part-folder"),
        NamePart::Exif { field } => t(&format!("batch-rn-exif-{}", exif_key(*field))),
        NamePart::Format => t("batch-rn-part-format"),
        NamePart::Unknown => "?".into(),
    }
}

fn exif_key(f: ExifField) -> &'static str {
    match f {
        ExifField::Model => "model",
        ExifField::Make => "make",
        ExifField::Lens => "lens",
        ExifField::Iso => "iso",
        ExifField::Aperture => "aperture",
    }
}

fn chip_color(theme: &Theme, p: &NamePart) -> Color32 {
    let c = &theme.colors;
    match p {
        NamePart::Original { .. } => c.focus,
        NamePart::Text { .. } => mix(c.text_weak, c.text, 0.3),
        NamePart::Counter { .. } => c.icon_green,
        NamePart::Date { .. } => c.group_accent[1],
        NamePart::Size { .. } => c.group_accent[0],
        NamePart::Folder => c.icon_yellow,
        NamePart::Exif { .. } => c.group_accent[2],
        NamePart::Format => c.group_accent[3],
        NamePart::Unknown => c.text_disabled,
    }
}

/// Neue Bausteine im Menü „+ Baustein“ (Schlüssel, Baustein).
pub fn new_parts() -> Vec<(&'static str, NamePart)> {
    vec![
        (
            "batch-rn-add-original",
            NamePart::Original {
                take: OriginalTake::Whole,
                count: 0,
            },
        ),
        ("batch-rn-add-text", NamePart::Text { text: "_".into() }),
        (
            "batch-rn-add-counter",
            NamePart::Counter {
                start: 1,
                step: 1,
                digits: 3,
                per_folder: false,
            },
        ),
        (
            "batch-rn-add-date",
            NamePart::Date {
                source: DateSource::Modified,
                format: "YYYY-MM-DD".into(),
            },
        ),
        (
            "batch-rn-add-exif-date",
            NamePart::Date {
                source: DateSource::Exif,
                format: "YYYY-MM-DD".into(),
            },
        ),
        (
            "batch-rn-add-size",
            NamePart::Size {
                which: SizeWhich::Both,
            },
        ),
        ("batch-rn-add-folder", NamePart::Folder),
        (
            "batch-rn-add-exif",
            NamePart::Exif {
                field: ExifField::Model,
            },
        ),
        ("batch-rn-add-format", NamePart::Format),
    ]
}

fn builder_section(ui: &mut Ui, spec: &mut RenameSpec, mode: Mode) {
    let theme = Theme::current(ui.ctx());
    let c = theme.colors.clone();
    ui_kit::section(ui, &t("batch-rn-section-parts"));
    let font = theme.font(type_scale::BODY);
    let avail = ui.available_width().max(80.0);
    let add_l = t("batch-rn-add");
    // Breiten und Fluss-Anordnung
    let labels: Vec<String> = spec.parts.iter().map(chip_label).collect();
    let max_text = (avail - 2.0 * 10.0 - 18.0).max(20.0);
    let shown: Vec<String> = labels
        .iter()
        .map(|l| ui_kit::elide(ui.ctx(), l, &font, max_text))
        .collect();
    let mut widths: Vec<f32> = shown
        .iter()
        .map(|l| (ui_kit::text_w(ui.ctx(), l, &font) + 2.0 * 10.0 + 16.0).ceil())
        .collect();
    let add_shown = ui_kit::elide(ui.ctx(), &add_l, &font, avail - 20.0);
    widths.push(
        (ui_kit::text_w(ui.ctx(), &add_shown, &font) + 2.0 * 10.0)
            .ceil()
            .min(avail),
    );
    let gap = sp::XS + 2.0;
    let mut pos = Vec::with_capacity(widths.len());
    let (mut x, mut y) = (0.0f32, 0.0f32);
    for w in &widths {
        if x > 0.0 && x + w > avail {
            x = 0.0;
            y += CHIP_H + gap;
        }
        pos.push(pos2(x, y));
        x += w + gap;
    }
    let (area, _) = ui.allocate_exact_size(vec2(avail, y + CHIP_H), Sense::hover());
    let rects: Vec<Rect> = pos
        .iter()
        .zip(&widths)
        .map(|(p, w)| Rect::from_min_size(area.min + p.to_vec2(), vec2(*w, CHIP_H)))
        .collect();
    let sel = selected(ui.ctx()).filter(|&i| i < spec.parts.len());
    let enabled = ui.is_enabled();
    let drag_id = Id::new("fp-batch-rename-chip-drag");
    let mut action: Option<ChipAction> = None;
    let pointer = ui.input(|i| i.pointer.interact_pos());
    let dragging: Option<usize> = ui
        .ctx()
        .data(|d| d.get_temp::<usize>(drag_id))
        .filter(|&i| i < spec.parts.len());
    for (i, part) in spec.parts.iter().enumerate() {
        let r = rects[i];
        let id = chip_id(i);
        let resp = ui.interact(
            r,
            id,
            if enabled {
                Sense::click_and_drag()
            } else {
                Sense::hover()
            },
        );
        resp.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::Button,
                enabled,
                sel == Some(i),
                &labels[i],
            )
        });
        let xr = Rect::from_center_size(pos2(r.right() - 11.0, r.center().y), vec2(14.0, 14.0));
        let x_name = tr!("batch-rn-remove-part", part = labels[i].clone());
        let xresp = ui.interact(
            xr,
            id.with("x"),
            if enabled {
                Sense::click()
            } else {
                Sense::hover()
            },
        );
        xresp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, &x_name));
        let col = chip_color(&theme, part);
        let on = sel == Some(i);
        let hot = resp.hovered() || resp.has_focus();
        let fill = if theme.glow {
            with_alpha(
                col,
                if on {
                    60
                } else if hot {
                    40
                } else {
                    22
                },
            )
        } else {
            kit::tint(
                &theme,
                col,
                if on {
                    2.4
                } else if hot {
                    1.6
                } else {
                    1.0
                },
            )
        };
        let border = if on {
            col
        } else if theme.glow {
            with_alpha(col, 150)
        } else {
            mix(kit::card_fill(&theme), col, if hot { 0.7 } else { 0.45 })
        };
        let p = ui.painter();
        let cr = CornerRadius::same((CHIP_H / 2.0) as u8);
        let alpha = if dragging == Some(i) { 0.45 } else { 1.0 };
        p.rect_filled(r, cr, fill.gamma_multiply(alpha));
        p.rect_stroke(
            r,
            cr,
            Stroke::new(if on { 1.5 } else { 1.0 }, border.gamma_multiply(alpha)),
            StrokeKind::Inside,
        );
        if resp.has_focus() {
            p.rect_stroke(
                r.expand(1.5),
                CornerRadius::same((CHIP_H / 2.0 + 2.0) as u8),
                Stroke::new(1.5, c.focus),
                StrokeKind::Outside,
            );
        }
        let txt_col = if enabled { c.text } else { c.text_disabled };
        p.text(
            pos2(r.left() + 10.0, r.center().y),
            Align2::LEFT_CENTER,
            &shown[i],
            font.clone(),
            txt_col,
        );
        // × entfernen
        let xc = if xresp.hovered() {
            c.icon_red
        } else {
            mix(txt_col, kit::card_fill(&theme), 0.35)
        };
        let s = Stroke::new(1.4, xc);
        let xx = xr.shrink(3.5);
        p.line_segment([xx.left_top(), xx.right_bottom()], s);
        p.line_segment([xx.right_top(), xx.left_bottom()], s);
        if xresp.clicked() {
            action = Some(ChipAction::Remove(i));
        } else if resp.clicked() {
            resp.request_focus();
            action = Some(ChipAction::Select(i));
        }
        let tip = if shown[i] != labels[i] {
            format!("{}\n{}", labels[i], t("batch-rn-chip-tooltip"))
        } else {
            t("batch-rn-chip-tooltip")
        };
        let resp = resp.hover_tip(tip);
        if resp.hovered() && enabled {
            ui.ctx().set_cursor_icon(if resp.dragged() {
                egui::CursorIcon::Grabbing
            } else {
                egui::CursorIcon::Grab
            });
        }
        if resp.drag_started() {
            ui.ctx().data_mut(|d| d.insert_temp(drag_id, i));
        }
        if resp.drag_stopped()
            && let Some(p) = pointer
        {
            action = Some(ChipAction::Move(
                i,
                insert_index(&rects[..spec.parts.len()], p),
            ));
        }
        // Tastatur: Strg+←/→ verschiebt den Baustein mit Fokus (Alt+Pfeil nimmt das Studio schon ab)
        if resp.has_focus() {
            if ui.input_mut(|inp| inp.consume_key(Modifiers::COMMAND, Key::ArrowLeft)) && i > 0 {
                action = Some(ChipAction::Move(i, i - 1));
            }
            if ui.input_mut(|inp| inp.consume_key(Modifiers::COMMAND, Key::ArrowRight))
                && i + 1 < spec.parts.len()
            {
                action = Some(ChipAction::Move(i, i + 2));
            }
        }
    }
    // Einfügelinie beim Ziehen
    if let (Some(_), Some(p)) = (dragging, pointer)
        && ui.input(|i| i.pointer.primary_down())
    {
        let k = insert_index(&rects[..spec.parts.len()], p);
        let x = if k < spec.parts.len() {
            rects[k].left() - gap / 2.0
        } else {
            rects[spec.parts.len().saturating_sub(1)].right() + gap / 2.0
        };
        let row = if k < spec.parts.len() {
            rects[k]
        } else {
            rects[spec.parts.len().saturating_sub(1)]
        };
        ui.painter().line_segment(
            [pos2(x, row.top() - 1.0), pos2(x, row.bottom() + 1.0)],
            Stroke::new(2.0, c.focus),
        );
    }
    if !ui.input(|i| i.pointer.any_down()) {
        ui.ctx().data_mut(|d| d.remove::<usize>(drag_id));
    }
    // „+ Baustein“
    let ar = rects[spec.parts.len()];
    let aresp = ui.interact(
        ar,
        Id::new("fp-batch-rename-add"),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    aresp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, &add_l));
    {
        let p = ui.painter();
        let hot = aresp.hovered() || aresp.has_focus();
        if hot {
            p.rect_filled(
                ar,
                CornerRadius::same((CHIP_H / 2.0) as u8),
                kit::tint(&theme, c.focus, 0.8),
            );
        }
        kit::dashed_rect(
            p,
            ar,
            if hot {
                c.focus
            } else {
                mix(c.text_weak, kit::card_fill(&theme), 0.3)
            },
        );
        p.text(
            ar.center(),
            Align2::CENTER_CENTER,
            &add_shown,
            font.clone(),
            if enabled { c.text } else { c.text_disabled },
        );
        if aresp.has_focus() {
            p.rect_stroke(
                ar.expand(1.5),
                CornerRadius::same(14),
                Stroke::new(1.5, c.focus),
                StrokeKind::Outside,
            );
        }
    }
    if aresp.hovered() && enabled {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    egui::Popup::menu(&aresp)
        .id(Id::new("fp-batch-rename-add-menu"))
        .show(|ui| {
            crate::theme::field_widgets(ui);
            ui.set_min_width(200.0);
            for (key, part) in new_parts() {
                if mode == Mode::Beginner
                    && matches!(part, NamePart::Exif { .. } | NamePart::Format)
                {
                    continue;
                }
                if ui
                    .button(t(key))
                    .hover_tip(t(&format!("{key}.tooltip")))
                    .clicked()
                {
                    action = Some(ChipAction::Add(part));
                }
            }
        });
    match action {
        Some(ChipAction::Select(i)) => {
            set_selected(ui.ctx(), if sel == Some(i) { None } else { Some(i) })
        }
        Some(ChipAction::Remove(i)) => {
            spec.parts.remove(i);
            set_selected(
                ui.ctx(),
                match sel {
                    Some(s) if s == i => None,
                    Some(s) if s > i => Some(s - 1),
                    s => s,
                },
            );
        }
        Some(ChipAction::Move(from, to)) => {
            if let Some(dest) = move_part(spec, from, to) {
                set_selected(ui.ctx(), Some(dest));
                ui.memory_mut(|m| m.request_focus(chip_id(dest)));
            }
        }
        Some(ChipAction::Add(part)) => {
            spec.parts.push(part);
            set_selected(ui.ctx(), Some(spec.parts.len() - 1));
        }
        None => {}
    }
    if spec.parts.is_empty() {
        ui.label(
            egui::RichText::new(t("batch-rn-no-parts"))
                .size(type_scale::CAPTION)
                .color(kit::weak(&theme)),
        );
    }
    if let Some(i) = selected(ui.ctx()).filter(|&i| i < spec.parts.len()) {
        ui.add_space(sp::XS);
        part_editor(ui, spec, i, mode);
    }
}

enum ChipAction {
    Select(usize),
    Remove(usize),
    Move(usize, usize),
    Add(NamePart),
}

/// Einfügeposition (0…n) für einen Punkt über den Chips (Fluss-Anordnung, Zeile für Zeile).
pub fn insert_index(rects: &[Rect], p: egui::Pos2) -> usize {
    rects
        .iter()
        .filter(|r| {
            r.center().y < p.y - CHIP_H / 2.0 - 2.0
                || ((r.center().y - p.y).abs() <= CHIP_H / 2.0 + 2.0 && r.center().x < p.x)
        })
        .count()
}

/// Beschriftung-Wert-Zeile im Kartenformular.
fn row<R>(ui: &mut Ui, lw: f32, label: &str, add: impl FnOnce(&mut Ui, f32) -> R) -> R {
    let theme = Theme::current(ui.ctx());
    ui.horizontal(|ui| {
        let (lr, _) = ui.allocate_exact_size(vec2(lw, 22.0), Sense::hover());
        let shown = ui_kit::elide(ui.ctx(), label, &theme.font(type_scale::BODY), lw - sp::XS);
        ui.painter().text(
            lr.left_center(),
            Align2::LEFT_CENTER,
            &shown,
            theme.font(type_scale::BODY),
            kit::weak(&theme),
        );
        if shown != label {
            ui.interact(
                lr,
                ui.id().with(("fp-batch-rn-label", label)),
                Sense::hover(),
            )
            .hover_tip(label);
        }
        let w = (ui.available_width() - 2.0).max(40.0);
        crate::theme::field_widgets(ui);
        add(ui, w)
    })
    .inner
}

fn label_w(ui: &Ui, keys: &[String]) -> f32 {
    let refs: Vec<&str> = keys.iter().map(String::as_str).collect();
    ui_kit::label_width(ui.ctx(), &refs, 60.0).min((ui.available_width() * 0.42).max(60.0))
}

fn text_field(ui: &mut Ui, s: &mut String, w: f32, name: &str, code: bool) -> egui::Response {
    let mut te = egui::TextEdit::singleline(s).desired_width(w);
    if code {
        te = te.code_editor();
    }
    let r = ui.add(te);
    // Name für Barrierefreiheit/`ui_find` (Beschriftung steht links daneben)
    r.widget_info(|| {
        let mut info = egui::WidgetInfo::new(egui::WidgetType::TextEdit);
        info.enabled = true;
        info.label = Some(name.to_string());
        info
    });
    r
}

fn part_editor(ui: &mut Ui, spec: &mut RenameSpec, i: usize, mode: Mode) {
    let theme = Theme::current(ui.ctx());
    let col = chip_color(&theme, &spec.parts[i]);
    let frame = egui::Frame::new()
        .fill(if theme.glow {
            kit::card_fill(&theme)
        } else {
            kit::tint(&theme, col, 0.6)
        })
        .stroke(Stroke::new(
            1.0,
            if theme.glow {
                with_alpha(col, 140)
            } else {
                mix(kit::card_fill(&theme), col, 0.4)
            },
        ))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(egui::Margin::same(8));
    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing = vec2(sp::S, sp::XS);
        let keys = [
            t("batch-rn-opt-take"),
            t("batch-rn-opt-count"),
            t("batch-rn-opt-text"),
            t("batch-rn-opt-start"),
            t("batch-rn-opt-step"),
            t("batch-rn-opt-digits"),
            t("batch-rn-opt-source"),
            t("batch-rn-opt-format"),
            t("batch-rn-opt-which"),
            t("batch-rn-opt-field"),
        ];
        let lw = label_w(ui, &keys);
        match &mut spec.parts[i] {
            NamePart::Original { take, count } => {
                let takes = [
                    (OriginalTake::Whole, "batch-rn-take-whole"),
                    (OriginalTake::First, "batch-rn-take-first"),
                    (OriginalTake::Last, "batch-rn-take-last"),
                    (OriginalTake::NoTrailingDigits, "batch-rn-take-trim"),
                ];
                row(ui, lw, &keys[0], |ui, w| {
                    combo(ui, ("take", i), w, take, &takes)
                });
                if matches!(take, OriginalTake::First | OriginalTake::Last) {
                    if *count == 0 {
                        *count = 8;
                    }
                    row(ui, lw, &keys[1], |ui, _| {
                        let r = ui.add(egui::DragValue::new(count).range(1..=255));
                        r.widget_info(|| {
                            egui::WidgetInfo::labeled(egui::WidgetType::DragValue, true, &keys[1])
                        });
                    });
                }
            }
            NamePart::Text { text } => {
                row(ui, lw, &keys[2], |ui, w| {
                    text_field(ui, text, w, &keys[2], false)
                });
            }
            NamePart::Counter {
                start,
                step,
                digits,
                per_folder,
            } => {
                for (k, v, range) in [(3usize, start, 0..=999_999u64), (4, step, 1..=1000)] {
                    row(ui, lw, &keys[k], |ui, _| {
                        let r = ui.add(egui::DragValue::new(v).range(range));
                        r.widget_info(|| {
                            egui::WidgetInfo::labeled(egui::WidgetType::DragValue, true, &keys[k])
                        });
                    });
                }
                row(ui, lw, &keys[5], |ui, _| {
                    let r = ui.add(egui::DragValue::new(digits).range(1..=9));
                    r.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::DragValue, true, &keys[5])
                    });
                });
                ui.checkbox(per_folder, t("batch-rn-opt-per-folder"));
            }
            NamePart::Date { source, format } => {
                let sources = [
                    (DateSource::Modified, "batch-rn-source-modified"),
                    (DateSource::Exif, "batch-rn-source-exif"),
                    (DateSource::Today, "batch-rn-source-today"),
                ];
                row(ui, lw, &keys[6], |ui, w| {
                    combo(ui, ("source", i), w, source, &sources)
                });
                row(ui, lw, &keys[7], |ui, w| {
                    let now = naming::now_local();
                    let cur = format.clone();
                    egui::ComboBox::from_id_salt(("fp-batch-rn-datefmt", i))
                        .selected_text(naming::format_date(&now, &cur))
                        .width((w - 8.0).min(220.0))
                        .show_ui(ui, |ui| {
                            for f in DATE_FORMATS {
                                ui.selectable_value(
                                    format,
                                    f.to_string(),
                                    format!("{}   ({f})", naming::format_date(&now, f)),
                                );
                            }
                        });
                });
                if mode == Mode::Pro {
                    row(ui, lw, "", |ui, w| {
                        text_field(ui, format, w, &keys[7], true)
                            .hover_tip(t("batch-rn-opt-format.tooltip"))
                    });
                }
                if *source == DateSource::Exif {
                    ui.label(
                        egui::RichText::new(t("batch-rn-exif-fallback"))
                            .size(type_scale::CAPTION)
                            .color(kit::weak(&theme)),
                    );
                }
            }
            NamePart::Size { which } => {
                let whiches = [
                    (SizeWhich::Both, "batch-rn-which-both"),
                    (SizeWhich::Width, "batch-rn-which-width"),
                    (SizeWhich::Height, "batch-rn-which-height"),
                ];
                row(ui, lw, &keys[8], |ui, w| {
                    combo(ui, ("which", i), w, which, &whiches)
                });
                ui.label(
                    egui::RichText::new(t("batch-rn-size-note"))
                        .size(type_scale::CAPTION)
                        .color(kit::weak(&theme)),
                );
            }
            NamePart::Exif { field } => {
                let fields = [
                    (ExifField::Model, "batch-rn-exif-model"),
                    (ExifField::Make, "batch-rn-exif-make"),
                    (ExifField::Lens, "batch-rn-exif-lens"),
                    (ExifField::Iso, "batch-rn-exif-iso"),
                    (ExifField::Aperture, "batch-rn-exif-aperture"),
                ];
                row(ui, lw, &keys[9], |ui, w| {
                    combo(ui, ("field", i), w, field, &fields)
                });
            }
            NamePart::Folder | NamePart::Format | NamePart::Unknown => {
                ui.label(
                    egui::RichText::new(t("batch-rn-part-no-options"))
                        .size(type_scale::CAPTION)
                        .color(kit::weak(&theme)),
                );
            }
        }
    });
}

fn combo<T: PartialEq + Copy>(
    ui: &mut Ui,
    salt: impl std::hash::Hash + std::fmt::Debug,
    w: f32,
    v: &mut T,
    items: &[(T, &str)],
) {
    let cur = items
        .iter()
        .find(|x| x.0 == *v)
        .map(|x| t(x.1))
        .unwrap_or_default();
    egui::ComboBox::from_id_salt(("fp-batch-rn-combo", salt))
        .selected_text(cur)
        .width((w - 8.0).min(240.0))
        .show_ui(ui, |ui| {
            for (val, key) in items {
                ui.selectable_value(v, *val, t(key));
            }
        });
}

// ---------------------------------------------------------------------- Regex (Profi)

fn beginner_regex_note(ui: &mut Ui, spec: &mut RenameSpec) {
    ui_kit::notice(ui, Tone::Info, None, &t("batch-rn-regex-beginner-note"));
    let l = t("batch-rn-to-builder");
    let w = kit::btn_w(ui.ctx(), &l, false).min(ui.available_width());
    let (r, _) = ui.allocate_exact_size(vec2(w, kit::BTN_H), Sense::hover());
    if kit::button(
        ui,
        r,
        Id::new("fp-batch-rn-to-builder"),
        &l,
        None,
        BtnKind::Secondary,
        ui.is_enabled(),
        &l,
    )
    .clicked()
    {
        switch_mode(spec, false);
    }
}

fn caption_wrapped(ui: &mut Ui, text: &str, col: Color32) {
    ui.add(
        egui::Label::new(
            egui::RichText::new(text)
                .size(type_scale::CAPTION)
                .color(col),
        )
        .wrap(),
    );
}

fn regex_section(ui: &mut Ui, spec: &mut RenameSpec) {
    let theme = Theme::current(ui.ctx());
    let c = theme.colors.clone();
    ui_kit::section(ui, &t("batch-rn-section-regex"));
    let keys = [t("batch-ui-rename-find"), t("batch-ui-rename-replace")];
    let lw = label_w(ui, &keys);
    let problem = explain::check_pattern(&spec.regex.find, spec.regex.ignore_case);
    let fr = row(ui, lw, &keys[0], |ui, w| {
        text_field(ui, &mut spec.regex.find, w, &keys[0], true)
    });
    if let Some(p) = &problem {
        ui.painter().rect_stroke(
            fr.rect.expand(1.0),
            3.0,
            Stroke::new(1.5, c.icon_red),
            StrokeKind::Outside,
        );
        ui.horizontal(|ui| {
            ui.add_space(lw + sp::S);
            ui.vertical(|ui| {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(&p.message)
                            .size(type_scale::CAPTION)
                            .color(c.icon_red),
                    )
                    .wrap(),
                )
                .hover_tip(&p.detail);
            });
        });
    } else {
        let ex = explain::explain_pattern(&spec.regex.find).unwrap_or_default();
        let ex = if spec.regex.ignore_case {
            format!("{ex} {}", t("batch-rn-ignore-case-note"))
        } else {
            ex
        };
        ui.horizontal(|ui| {
            ui.add_space(lw + sp::S);
            ui.vertical(|ui| {
                caption_wrapped(
                    ui,
                    &tr!("batch-rn-explain-find", text = ex),
                    kit::weak(&theme),
                )
            });
        });
    }
    row(ui, lw, &keys[1], |ui, w| {
        text_field(ui, &mut spec.regex.replace, w, &keys[1], true)
    });
    ui.horizontal(|ui| {
        ui.add_space(lw + sp::S);
        ui.vertical(|ui| {
            caption_wrapped(
                ui,
                &tr!(
                    "batch-rn-explain-replace",
                    text = explain::explain_replacement(&spec.regex.replace)
                ),
                kit::weak(&theme),
            )
        });
    });
    ui.horizontal_wrapped(|ui| {
        crate::theme::field_widgets(ui);
        ui.checkbox(&mut spec.regex.ignore_case, t("batch-rn-ignore-case"));
        ui.checkbox(&mut spec.regex.all, t("batch-rn-all-matches"));
        ui.checkbox(&mut spec.regex.whole_name, t("batch-rn-whole-name"));
    });
    // Aus Baukasten übernehmen + Spickzettel
    let l = t("batch-rn-from-builder");
    let w = kit::btn_w(ui.ctx(), &l, false).min(ui.available_width());
    let (r, _) = ui.allocate_exact_size(vec2(w, kit::BTN_H), Sense::hover());
    if kit::button(
        ui,
        r,
        Id::new("fp-batch-rn-from-builder"),
        &l,
        None,
        BtnKind::Secondary,
        ui.is_enabled(),
        &l,
    )
    .hover_tip(t("batch-rn-from-builder.tooltip"))
    .clicked()
    {
        spec.regex = explain::builder_to_regex(spec);
    }
    caption_wrapped(
        ui,
        &format!("{}  {PLACEHOLDERS}", t("batch-rn-placeholders")),
        kit::weak(&theme),
    );
    egui::CollapsingHeader::new(t("batch-rn-cheat"))
        .id_salt("fp-batch-rn-cheat")
        .default_open(false)
        .show(ui, |ui| {
            for e in explain::CHEAT_SHEET {
                ui.horizontal(|ui| {
                    let l = t("batch-rn-cheat-insert");
                    let bw = kit::btn_w(ui.ctx(), &l, false);
                    ui.vertical(|ui| {
                        ui.set_width((ui.available_width() - bw - sp::S).max(60.0));
                        caption_wrapped(ui, &t(e.key), c.text);
                        caption_wrapped(
                            ui,
                            &format!(
                                "{}  →  {}",
                                e.find,
                                if e.replace.is_empty() {
                                    "∅"
                                } else {
                                    e.replace
                                }
                            ),
                            kit::weak(&theme),
                        );
                        if let Some(out) = explain::apply_sample(e.find, e.replace, e.sample) {
                            caption_wrapped(
                                ui,
                                &format!("{} → {out}", e.sample),
                                kit::weak(&theme),
                            );
                        }
                    });
                    let (r, _) = ui.allocate_exact_size(vec2(bw, kit::BTN_H), Sense::hover());
                    if kit::button(
                        ui,
                        r,
                        Id::new(("fp-batch-rn-cheat-insert", e.key)),
                        &l,
                        None,
                        BtnKind::Secondary,
                        true,
                        &format!("{l}: {}", t(e.key)),
                    )
                    .clicked()
                    {
                        spec.regex.find = e.find.to_string();
                        spec.regex.replace = e.replace.to_string();
                        spec.regex.whole_name = false;
                    }
                });
                ui.add_space(sp::XS);
            }
        });
}

// ---------------------------------------------------------------------- Optionen

fn options_section(ui: &mut Ui, spec: &mut RenameSpec, mode: Mode, show_more: &mut bool) {
    let theme = Theme::current(ui.ctx());
    ui.add_space(sp::XS);
    ui_kit::section(ui, &t("batch-rn-section-options"));
    let keys = [
        t("batch-rn-spaces"),
        t("batch-rn-prefix"),
        t("batch-rn-suffix"),
        t("batch-ui-rename-case"),
        t("batch-rn-extension"),
        t("batch-rn-replace-title"),
    ];
    let lw = label_w(ui, &keys);
    // Text ersetzen (mehrere Zeilen)
    let mut remove = None;
    if spec.replacements.is_empty() {
        let l = t("batch-rn-replace-add");
        if ui_kit::more_link(ui, &l, Some(t("batch-rn-replace-add.tooltip").as_str())).clicked() {
            spec.replacements.push(Replacement {
                find: String::new(),
                replace: String::new(),
                all: true,
                match_case: true,
            });
        }
    } else {
        for (k, r) in spec.replacements.iter_mut().enumerate() {
            let label = if k == 0 {
                keys[5].clone()
            } else {
                String::new()
            };
            row(ui, lw, &label, |ui, w| {
                let fw = ((w - 16.0 - 22.0 - 2.0 * sp::S) / 2.0).max(30.0);
                text_field(ui, &mut r.find, fw, &t("batch-rn-replace-find"), false)
                    .hover_tip(t("batch-rn-replace-find"));
                ui.label("→");
                text_field(ui, &mut r.replace, fw, &t("batch-rn-replace-with"), false)
                    .hover_tip(t("batch-rn-replace-with"));
                let (xr, _) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
                let name = t("batch-rn-replace-remove");
                let resp = kit::button(
                    ui,
                    xr,
                    Id::new(("fp-batch-rn-rep-x", k)),
                    "×",
                    None,
                    BtnKind::Ghost,
                    ui.is_enabled(),
                    &name,
                )
                .hover_tip(&name);
                if resp.clicked() {
                    remove = Some(k);
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.add_space(lw + sp::S);
                crate::theme::field_widgets(ui);
                ui.checkbox(&mut r.all, t("batch-rn-replace-all"));
                ui.checkbox(&mut r.match_case, t("batch-rn-replace-case"));
            });
        }
        ui.horizontal(|ui| {
            ui.add_space(lw + sp::S);
            if ui_kit::more_link(ui, &t("batch-rn-replace-more"), None).clicked() {
                spec.replacements.push(Replacement {
                    find: String::new(),
                    replace: String::new(),
                    all: true,
                    match_case: true,
                });
            }
        });
    }
    if let Some(k) = remove {
        spec.replacements.remove(k);
    }
    // Leerzeichen
    row(ui, lw, &keys[0], |ui, w| {
        let kinds: [(Option<&str>, &str); 4] = [
            (None, "batch-rn-spaces-keep"),
            (Some("_"), "batch-rn-spaces-underscore"),
            (Some("-"), "batch-rn-spaces-dash"),
            (Some(""), "batch-rn-spaces-none"),
        ];
        let custom = spec
            .spaces
            .as_deref()
            .is_some_and(|s| !matches!(s, "_" | "-" | ""));
        let cur = if custom {
            t("batch-rn-spaces-custom")
        } else {
            kinds
                .iter()
                .find(|k| k.0 == spec.spaces.as_deref())
                .map(|k| t(k.1))
                .unwrap_or_default()
        };
        let cw = if custom {
            (w * 0.6).min(200.0)
        } else {
            (w - 8.0).min(220.0)
        };
        egui::ComboBox::from_id_salt("fp-batch-rn-spaces")
            .selected_text(cur)
            .width(cw)
            .show_ui(ui, |ui| {
                for (v, k) in kinds {
                    if ui
                        .selectable_label(!custom && spec.spaces.as_deref() == v, t(k))
                        .clicked()
                    {
                        spec.spaces = v.map(str::to_string);
                    }
                }
                if ui
                    .selectable_label(custom, t("batch-rn-spaces-custom"))
                    .clicked()
                    && !custom
                {
                    spec.spaces = Some("+".into());
                }
            });
        if custom && let Some(s) = spec.spaces.as_mut() {
            text_field(ui, s, (w - cw - 16.0).clamp(24.0, 80.0), &keys[0], false);
        }
    });
    row(ui, lw, &keys[1], |ui, w| {
        text_field(ui, &mut spec.prefix, w, &keys[1], false)
    });
    row(ui, lw, &keys[2], |ui, w| {
        text_field(ui, &mut spec.suffix, w, &keys[2], false).hover_tip(t("batch-rn-suffix.tooltip"))
    });
    let cases = [
        (CaseMode::Keep, "batch-ui-case-keep"),
        (CaseMode::Lower, "batch-ui-case-lower"),
        (CaseMode::Upper, "batch-ui-case-upper"),
        (CaseMode::Sentence, "batch-ui-case-sentence"),
        (CaseMode::Title, "batch-ui-case-title"),
        (CaseMode::Kebab, "batch-ui-case-kebab"),
        (CaseMode::Snake, "batch-ui-case-snake"),
        (CaseMode::Camel, "batch-ui-case-camel"),
    ];
    row(ui, lw, &keys[3], |ui, w| {
        let cur = cases
            .iter()
            .find(|c| c.0 == spec.case)
            .map(|c| t(c.1))
            .unwrap_or_default();
        egui::ComboBox::from_id_salt("fp-batch-rn-case")
            .selected_text(cur)
            .width((w - 8.0).min(220.0))
            .show_ui(ui, |ui| {
                for (cm, k) in cases {
                    if mode == Mode::Beginner
                        && matches!(cm, CaseMode::Kebab | CaseMode::Snake | CaseMode::Camel)
                        && spec.case != cm
                    {
                        continue;
                    }
                    ui.selectable_value(&mut spec.case, cm, t(k));
                }
            });
    });
    if mode == Mode::Pro || *show_more {
        ui.horizontal_wrapped(|ui| {
            ui.add_space(lw + sp::S);
            crate::theme::field_widgets(ui);
            ui.checkbox(&mut spec.transliterate, t("batch-rn-transliterate"));
            ui.checkbox(&mut spec.strip_special, t("batch-rn-strip-special"))
                .hover_tip(t("batch-rn-strip-special.tooltip"));
        });
        let exts = [
            (ExtCase::Lower, "batch-rn-ext-lower"),
            (ExtCase::Keep, "batch-rn-ext-keep"),
            (ExtCase::Upper, "batch-rn-ext-upper"),
        ];
        row(ui, lw, &keys[4], |ui, w| {
            combo(ui, "ext", w, &mut spec.extension, &exts);
        });
        ui.horizontal_wrapped(|ui| {
            ui.add_space(lw + sp::S);
            crate::theme::field_widgets(ui);
            ui.checkbox(&mut spec.jpeg_as_jpg, t("batch-rn-jpeg-as-jpg"));
        });
    }
    if mode == Mode::Beginner {
        ui.add_space(sp::XS);
        let key = if *show_more {
            "batch-ui-step-less"
        } else {
            "batch-ui-step-more"
        };
        if ui_kit::more_link(ui, &t(key), None).clicked() {
            *show_more = !*show_more;
        }
    }
    let _ = theme;
}

// ---------------------------------------------------------------------- Live-Tabelle

/// Eine Zeile der Live-Tabelle.
#[derive(Clone, Debug, PartialEq)]
pub struct TableRow {
    pub old: String,
    pub new: String,
    pub hints: Vec<NameHint>,
    /// Gleicher Zielname wie Zeile (1-basiert).
    pub conflict_with: Option<usize>,
    pub unchanged: bool,
    old_parts: Vec<DiffPart>,
    new_parts: Vec<DiffPart>,
}

/// Ergebnis der Live-Tabelle (zwischengespeichert, bis sich Rezept oder Dateien ändern).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Table {
    pub rows: Vec<TableRow>,
    /// Gezählte Dateien (höchstens [`COUNT_LIMIT`]) und davon geänderte bzw. mit Konflikt.
    pub counted: usize,
    pub changed: usize,
    pub conflicts: usize,
    pub total: usize,
    /// Ungültiges Muster (Meldung).
    pub error: Option<String>,
}

fn table_key(recipe: &Recipe, files: &[ItemInfo]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    recipe.to_json().hash(&mut h);
    files.len().hash(&mut h);
    for f in files.iter().take(COUNT_LIMIT) {
        f.path.hash(&mut h);
    }
    h.finish()
}

/// Rechnet die Tabelle (Plan über die ersten [`COUNT_LIMIT`] Dateien, Zeilen für die ersten
/// [`TABLE_ROWS`]). Schreibt nichts.
pub fn compute_table(recipe: &Recipe, files: &[ItemInfo]) -> Table {
    let reg = StepRegistry::global();
    let file_only = fp_batch::pipeline::is_file_only(recipe, &reg);
    let counted = files.len().min(COUNT_LIMIT);
    let mut table = Table {
        total: files.len(),
        counted,
        ..Table::default()
    };
    if recipe.rename.mode == RenameMode::Regex
        && let Some(p) =
            explain::check_pattern(&recipe.rename.regex.find, recipe.rename.regex.ignore_case)
    {
        table.error = Some(p.message);
        table.rows = files
            .iter()
            .take(TABLE_ROWS)
            .map(|f| {
                let n = f.file_name();
                TableRow {
                    old_parts: vec![DiffPart {
                        text: n.clone(),
                        changed: false,
                    }],
                    new_parts: Vec::new(),
                    old: n,
                    new: String::new(),
                    hints: Vec::new(),
                    conflict_with: None,
                    unchanged: true,
                }
            })
            .collect();
        return table;
    }
    match naming::plan(
        recipe,
        &files[..counted],
        file_only,
        &PlanOptions::default(),
    ) {
        Ok(plan) => {
            for e in &plan.entries {
                let old = files[e.index].file_name();
                let new = e
                    .target
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let unchanged = old == new;
                if !unchanged {
                    table.changed += 1;
                }
                if e.hints.contains(&NameHint::Conflict) {
                    table.conflicts += 1;
                }
                if e.index < TABLE_ROWS {
                    let (old_parts, new_parts) = explain::char_diff(&old, &new);
                    table.rows.push(TableRow {
                        old,
                        new,
                        hints: e.hints.clone(),
                        conflict_with: e.conflict_with.map(|j| j + 1),
                        unchanged,
                        old_parts,
                        new_parts,
                    });
                }
            }
        }
        Err(e) => table.error = Some(e.to_string()),
    }
    table
}

fn cached_table(ctx: &egui::Context, recipe: &Recipe, files: &[ItemInfo]) -> Arc<Table> {
    let id = Id::new("fp-batch-rename-table-cache");
    let key = table_key(recipe, files);
    if let Some((k, t)) = ctx.data(|d| d.get_temp::<(u64, Arc<Table>)>(id))
        && k == key
    {
        return t;
    }
    let t = Arc::new(compute_table(recipe, files));
    ctx.data_mut(|d| d.insert_temp(id, (key, Arc::clone(&t))));
    t
}

fn severity(h: NameHint) -> u8 {
    match h {
        NameHint::Conflict
        | NameHint::Skipped
        | NameHint::Overwrite
        | NameHint::InvalidChars
        | NameHint::ReservedName
        | NameHint::Truncated
        | NameHint::EmptyName
        | NameHint::LeadingDot
        | NameHint::ReplaceOriginal
        | NameHint::BackupForced => 2,
        NameHint::ExifDateMissing | NameHint::LongPath | NameHint::FormatFallback => 1,
    }
}

fn diff_job(
    parts: &[DiffPart],
    font: &egui::FontId,
    base: Color32,
    changed: Color32,
    bg: Color32,
    strike: bool,
    max_w: f32,
) -> LayoutJob {
    let mut job = LayoutJob {
        wrap: TextWrapping {
            max_width: max_w.max(8.0),
            max_rows: 1,
            break_anywhere: true,
            overflow_character: Some('…'),
        },
        ..Default::default()
    };
    for p in parts {
        let mut f = TextFormat::simple(font.clone(), if p.changed { changed } else { base });
        if p.changed {
            f.background = bg;
            if strike {
                f.strikethrough = Stroke::new(1.0, changed);
            }
        }
        job.append(&p.text, 0.0, f);
    }
    job
}

fn table_section(
    ui: &mut Ui,
    spec: &mut RenameSpec,
    files: &[ItemInfo],
    recipe: &Recipe,
    enabled: bool,
) {
    let theme = Theme::current(ui.ctx());
    let c = theme.colors.clone();
    ui.add_space(sp::XS);
    ui_kit::section(ui, &t("batch-rn-section-preview"));
    if files.is_empty() {
        caption_wrapped(ui, &t("batch-rn-table-empty"), kit::weak(&theme));
        return;
    }
    let r = Recipe {
        rename: spec.clone(),
        ..recipe.clone()
    };
    let table = cached_table(ui.ctx(), &r, files);
    // Zusammenfassung + „Nummer anhängen“
    let mut summary = tr!(
        "batch-rn-table-summary",
        changed = table.changed as i64,
        total = table.counted as i64
    );
    if table.conflicts > 0 {
        summary = format!(
            "{summary} · {}",
            tr!("batch-rn-table-conflicts", n = table.conflicts as i64)
        );
    }
    if table.total > table.counted {
        summary = format!(
            "{summary} · {}",
            tr!("batch-rn-table-counted", n = table.counted as i64)
        );
    }
    if let Some(err) = &table.error {
        ui_kit::notice(
            ui,
            Tone::Error,
            None,
            &format!("{err}\n{}", t("batch-rn-regex-invalid-table")),
        );
    } else {
        caption_wrapped(
            ui,
            &summary,
            if table.conflicts > 0 {
                c.icon_yellow
            } else {
                kit::weak(&theme)
            },
        );
    }
    if table.conflicts > 0 && spec.mode == RenameMode::Builder && enabled {
        let l = t("batch-rn-add-counter-fix");
        let w = kit::btn_w(ui.ctx(), &l, false).min(ui.available_width());
        let (br, _) = ui.allocate_exact_size(vec2(w, kit::BTN_H), Sense::hover());
        if kit::button(
            ui,
            br,
            Id::new("fp-batch-rn-add-counter"),
            &l,
            None,
            BtnKind::Secondary,
            true,
            &l,
        )
        .hover_tip(t("batch-rn-add-counter-fix.tooltip"))
        .clicked()
        {
            spec.parts.push(NamePart::Text { text: "_".into() });
            spec.parts.push(NamePart::Counter {
                start: 1,
                step: 1,
                digits: 3,
                per_folder: false,
            });
        }
    }
    // Kopf
    let font = theme.font(type_scale::BODY);
    let small = theme.font(type_scale::CAPTION);
    let w = ui.available_width();
    let hint_l = t("batch-rn-table-hint");
    let hint_w = (ui_kit::text_w(ui.ctx(), &hint_l, &small) + sp::S).clamp(24.0, 90.0);
    let arrow_w = 18.0;
    let col_w = ((w - arrow_w - hint_w - 14.0) / 2.0).max(30.0);
    let (hr, _) = ui.allocate_exact_size(vec2(w, 18.0), Sense::hover());
    {
        let p = ui.painter();
        let weak = kit::weak(&theme);
        kit::text_left(
            p,
            hr.left() + 4.0,
            hr.center().y,
            col_w - 4.0,
            &t("batch-rn-table-old"),
            theme.bold(type_scale::CAPTION),
            weak,
        );
        kit::text_left(
            p,
            hr.left() + col_w + arrow_w + 4.0,
            hr.center().y,
            col_w - 4.0,
            &t("batch-rn-table-new"),
            theme.bold(type_scale::CAPTION),
            weak,
        );
        kit::text_left(
            p,
            hr.left() + 2.0 * col_w + arrow_w + 4.0,
            hr.center().y,
            hint_w - 2.0,
            &hint_l,
            theme.bold(type_scale::CAPTION),
            weak,
        );
        p.line_segment(
            [pos2(hr.left(), hr.bottom()), pos2(hr.right(), hr.bottom())],
            Stroke::new(1.0, kit::item_border(&theme, c.focus)),
        );
    }
    let n = table.rows.len();
    let green_bg = with_alpha(
        c.icon_green,
        if theme.kind == crate::theme::ThemeKind::Normal {
            60
        } else {
            70
        },
    );
    let red_bg = with_alpha(
        c.icon_red,
        if theme.kind == crate::theme::ThemeKind::Normal {
            40
        } else {
            55
        },
    );
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        egui::ScrollArea::vertical()
            .id_salt("fp-batch-rn-table")
            .max_height((n as f32 * ROW_H).min(10.0 * ROW_H) + 2.0)
            .auto_shrink([false, true])
            .show_rows(ui, ROW_H, n, |ui, range| {
                let w = ui.available_width();
                let col_w = ((w - arrow_w - hint_w - 14.0) / 2.0).max(30.0);
                for i in range {
                    let row = &table.rows[i];
                    let (rr, resp) = ui.allocate_exact_size(vec2(w, ROW_H), Sense::hover());
                    let p = ui.painter();
                    if i % 2 == 1 {
                        p.rect_filled(rr, 0.0, with_alpha(c.text, 8));
                    }
                    let dim = row.unchanged || table.error.is_some();
                    let base = if dim { kit::weak(&theme) } else { c.text };
                    let old_r =
                        Rect::from_min_size(rr.min + vec2(4.0, 0.0), vec2(col_w - 4.0, ROW_H));
                    let g = p.layout_job(diff_job(
                        &row.old_parts,
                        &font,
                        base,
                        c.icon_red,
                        red_bg,
                        true,
                        old_r.width(),
                    ));
                    p.galley(
                        pos2(old_r.left(), rr.center().y - g.size().y / 2.0),
                        g,
                        base,
                    );
                    let ax = rr.left() + col_w + arrow_w / 2.0;
                    p.text(
                        pos2(ax, rr.center().y),
                        Align2::CENTER_CENTER,
                        if row.unchanged && table.error.is_none() {
                            "="
                        } else {
                            "→"
                        },
                        small.clone(),
                        kit::weak(&theme),
                    );
                    let new_r = Rect::from_min_size(
                        pos2(rr.left() + col_w + arrow_w + 4.0, rr.top()),
                        vec2(col_w - 4.0, ROW_H),
                    );
                    if table.error.is_none() {
                        let g = p.layout_job(diff_job(
                            &row.new_parts,
                            &font,
                            base,
                            c.text,
                            green_bg,
                            false,
                            new_r.width(),
                        ));
                        p.galley(
                            pos2(new_r.left(), rr.center().y - g.size().y / 2.0),
                            g,
                            base,
                        );
                    } else {
                        p.text(
                            pos2(new_r.left(), rr.center().y),
                            Align2::LEFT_CENTER,
                            "—",
                            font.clone(),
                            kit::weak(&theme),
                        );
                    }
                    let sev = row.hints.iter().map(|h| severity(*h)).max().unwrap_or(0);
                    let mut tip = format!(
                        "{}\n→ {}",
                        row.old,
                        if table.error.is_some() {
                            "—"
                        } else {
                            &row.new
                        }
                    );
                    if row.unchanged && table.error.is_none() {
                        tip.push_str(&format!("\n{}", t("batch-rn-table-unchanged")));
                    }
                    for h in &row.hints {
                        tip.push_str(&format!("\n⚠ {}", h.message()));
                    }
                    if let Some(j) = row.conflict_with {
                        tip.push_str(&format!(
                            "\n{}",
                            tr!("batch-rn-table-conflict-with", row = j as i64)
                        ));
                    }
                    if sev > 0 {
                        let ir = Rect::from_center_size(
                            pos2(rr.left() + 2.0 * col_w + arrow_w + 4.0 + 8.0, rr.center().y),
                            vec2(14.0, 14.0),
                        );
                        ui_kit::paint_icon(
                            p,
                            ir,
                            if sev >= 2 {
                                KitIcon::Warning
                            } else {
                                KitIcon::Info
                            },
                            &crate::icons::IconStyle::new(&theme, None, true),
                        );
                    }
                    resp.hover_tip(tip);
                }
            });
    });
    if table.total > table.rows.len() {
        caption_wrapped(
            ui,
            &tr!(
                "batch-rn-table-more",
                n = (table.total - table.rows.len()) as i64
            ),
            kit::weak(&theme),
        );
    }
}

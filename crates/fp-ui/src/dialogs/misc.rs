//! Weitere Dialoge: Texteditor (5962), Rückfrage mit Befehl (z. B. Symbol löschen, 6003),
//! HTML-Segment bearbeiten (14139), Strich bearbeiten (5963–5965, vereinfacht), Neuer Stil
//! (19123).

use super::form::{Field, FormDialog};
use super::{DialogClose, dialog_frame, dialog_frame_resizable, push_button, t};
use crate::app_state::{Action, AppState};
use crate::inspector::fields;
use crate::inspector::glyphs::Glyph;
use crate::theme::Theme;
use egui::{Align2, Rect, Stroke, StrokeKind, pos2, vec2};
use serde_json::{Value, json};

/// Rückfrage, die bei „Ja“ einen Befehl ausführt.
#[derive(Clone, Debug, PartialEq)]
pub struct Confirm {
    pub title: String,
    pub text: String,
    pub command: String,
    pub params: Value,
}

/// Zeigt eine Rückfrage.
pub fn show_confirm(ctx: &egui::Context, st: &mut AppState) {
    let Some(c) = st.dialogs.confirm.clone() else {
        return;
    };
    let mut result = None;
    let (yes, no) = (t("btn-yes"), t("btn-no"));
    use crate::ui_kit::{self, BarButton, KitIcon, sp};
    let w = 400f32.max(ui_kit::button_bar_width(
        ctx,
        &[],
        &[BarButton::default_button(&yes), BarButton::new(&no)],
    ));
    // Gestaltung (docs\DESIGN.md §3, Rückfrage): Warnsymbol 32 px links, Text daneben (mittig zum
    // Symbol, längere Texte umbrechen und lassen den Dialog wachsen), Knopfleiste
    let text_x = sp::L + 32.0 + sp::M;
    let th = super::wrapped_h(ctx, &c.text, w - text_x - sp::L);
    let content_h = th.max(32.0);
    let r = dialog_frame(
        ctx,
        "fp-confirm",
        &c.title,
        vec2(w, (sp::L + content_h + sp::L + sp::BUTTON_BAR_H).ceil()),
        |ui, body| {
            let theme = Theme::current(ui.ctx());
            let icon = Rect::from_min_size(body.min + vec2(sp::L, sp::L), vec2(32.0, 32.0));
            ui_kit::paint_icon(
                ui.painter(),
                icon,
                KitIcon::Warning,
                &crate::icons::IconStyle::new(&theme, None, true),
            );
            let ty = if th < 32.0 {
                icon.center().y - th / 2.0
            } else {
                icon.top()
            };
            super::put_left(
                ui,
                Rect::from_min_size(
                    pos2(body.left() + text_x, ty),
                    vec2(body.width() - text_x - sp::L, th + 4.0),
                ),
                egui::Label::new(c.text.as_str()).wrap(),
            );
            let bar = ui_kit::button_bar(
                ui,
                body,
                &[],
                &[BarButton::default_button(&yes), BarButton::new(&no)],
            );
            if bar.right == Some(0) || access(ui, super::access_key(&yes)) {
                result = Some(true);
            }
            if bar.right == Some(1) || access(ui, super::access_key(&no)) {
                result = Some(false);
            }
        },
    );
    match (result, r) {
        (Some(true), _) => {
            st.dialogs.confirm = None;
            st.push(Action::Run(c.command, c.params));
        }
        (Some(false), _) | (_, DialogClose::Cancelled) => st.dialogs.confirm = None,
        _ => {}
    }
}

/// Rückfrage mit eigenen Knöpfen; jeder Knopf führt seinen Befehl aus (`None` = Abbrechen),
/// Esc/Schließen bricht ab.
#[derive(Clone, Debug, PartialEq)]
pub struct ChoiceDialog {
    pub title: String,
    pub text: String,
    /// Warnsymbol links (Meldungsfenster); ohne Symbol z. B. „Aktuell / Alle“.
    pub warning: bool,
    /// Knöpfe von links nach rechts.
    pub buttons: Vec<(String, Option<(String, Value)>)>,
    /// Index of the default button (frame, Enter); usually the first.
    pub default: usize,
}

impl ChoiceDialog {
    /// „Ebene statusübergreifend freigeben“ bei mehr als einem Status (re 40 §2b): Löschwarnung mit
    /// OK / Abbrechen.
    pub fn share_layer(layer: u64, name: &str) -> ChoiceDialog {
        ChoiceDialog {
            title: "Firepixel".into(),
            text: fp_i18n::tr!("dlg-layer-share-confirm", name = name),
            warning: true,
            buttons: vec![
                (
                    t("btn-ok"),
                    Some((
                        "layer.setShared".into(),
                        json!({"layer": layer, "shared": true, "step": "layer-options"}),
                    )),
                ),
                (t("btn-cancel"), None),
            ],
            default: 0,
        }
    }

    /// Freigabe aufheben (re 40 §2b): Daten in den aktuellen Status oder in alle kopieren.
    pub fn unshare_layer(layer: u64, name: &str) -> ChoiceDialog {
        let run = |to: &str| {
            Some((
                "layer.setShared".into(),
                json!({"layer": layer, "shared": false, "copyTo": to, "step": "layer-options"}),
            ))
        };
        ChoiceDialog {
            title: String::new(),
            text: fp_i18n::tr!("dlg-layer-unshare-ask", name = name),
            warning: false,
            buttons: vec![
                (t("dlg-layer-unshare-current"), run("current")),
                (t("dlg-layer-unshare-all"), run("all")),
            ],
            default: 0,
        }
    }

    /// Modify > Mask > Delete Mask (RE spec 68 section 8, M-11): "Apply the mask before removing
    /// it?" with Discard / Cancel / Apply, Apply the default. Apply bakes the mask into a bitmap,
    /// Discard drops the mask and keeps the content, Cancel does nothing. `vector`: the masked
    /// content is not a bitmap (the text says that it becomes one).
    pub fn delete_mask(vector: bool) -> ChoiceDialog {
        let run = |apply: bool| Some(("modify.mask.deleteMask".into(), json!({ "apply": apply })));
        ChoiceDialog {
            title: "Firepixel".into(),
            text: t(if vector {
                "dlg-mask-delete-vector"
            } else {
                "dlg-mask-delete-bitmap"
            }),
            warning: true,
            buttons: vec![
                (t("btn-mask-discard"), run(false)),
                (t("btn-cancel"), None),
                (t("btn-mask-apply"), run(true)),
            ],
            default: 2,
        }
    }
}

/// Zeigt eine [`ChoiceDialog`]-Rückfrage.
pub fn show_choice(ctx: &egui::Context, st: &mut AppState) {
    let Some(c) = st.dialogs.choice.clone() else {
        return;
    };
    use crate::ui_kit::{self, BarButton, KitIcon, sp};
    let bars: Vec<BarButton> = c
        .buttons
        .iter()
        .enumerate()
        .map(|(i, (l, _))| {
            if i == c.default {
                BarButton::default_button(l)
            } else {
                BarButton::new(l)
            }
        })
        .collect();
    let w = 400f32.max(ui_kit::button_bar_width(ctx, &[], &bars));
    let text_x = if c.warning {
        sp::L + 32.0 + sp::M
    } else {
        sp::L
    };
    let th = super::wrapped_h(ctx, &c.text, w - text_x - sp::L);
    let content_h = if c.warning { th.max(32.0) } else { th };
    let mut clicked = None;
    let r = dialog_frame(
        ctx,
        "fp-choice",
        &c.title,
        vec2(w, (sp::L + content_h + sp::L + sp::BUTTON_BAR_H).ceil()),
        |ui, body| {
            let theme = Theme::current(ui.ctx());
            let ty = if c.warning {
                let icon = Rect::from_min_size(body.min + vec2(sp::L, sp::L), vec2(32.0, 32.0));
                ui_kit::paint_icon(
                    ui.painter(),
                    icon,
                    KitIcon::Warning,
                    &crate::icons::IconStyle::new(&theme, None, true),
                );
                if th < 32.0 {
                    icon.center().y - th / 2.0
                } else {
                    icon.top()
                }
            } else {
                body.top() + sp::L
            };
            super::put_left(
                ui,
                Rect::from_min_size(
                    pos2(body.left() + text_x, ty),
                    vec2(body.width() - text_x - sp::L, th + 4.0),
                ),
                egui::Label::new(c.text.as_str()).wrap(),
            );
            let bar = ui_kit::button_bar(ui, body, &[], &bars);
            clicked = bar.right;
        },
    );
    match (clicked, r) {
        (Some(i), _) => {
            st.dialogs.choice = None;
            if let Some((cmd, p)) = c.buttons.get(i).and_then(|b| b.1.clone()) {
                st.push(Action::Run(cmd, p));
            }
        }
        (None, DialogClose::Cancelled) => st.dialogs.choice = None,
        _ => {}
    }
}

/// Zugriffstaste ohne Alt (Meldungsfenster), nur ohne Textfeldfokus.
fn access(ui: &egui::Ui, k: Option<egui::Key>) -> bool {
    let Some(k) = k else { return false };
    !ui.ctx().text_edit_focused() && ui.input(|i| i.key_pressed(k) && !i.modifiers.ctrl)
}

// ---------------------------------------------------------------------- Texteditor

/// Texteditor (5962).
#[derive(Clone, Debug, PartialEq)]
pub struct TextEditorDialog {
    pub id: u64,
    pub text: String,
    pub font: String,
    pub size: f64,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub align: i32,
    pub show_font: bool,
}

impl TextEditorDialog {
    /// Für den ausgewählten Text.
    pub fn new(st: &AppState) -> Option<TextEditorDialog> {
        let e = crate::inspector::obj_quads::selection(st)
            .into_iter()
            .find(|e| matches!(e.kind, fp_model::ElementKind::Text(_)))?;
        let fp_model::ElementKind::Text(t) = &e.kind else {
            return None;
        };
        let a = t
            .resolved_runs()
            .first()
            .map(|(a, _)| a.clone())
            .unwrap_or_else(|| t.initial.clone());
        Some(TextEditorDialog {
            id: e.id.0,
            text: t.plain_text().replace('\r', "\n"),
            font: a.font.clone().unwrap_or_default(),
            size: a.size.unwrap_or(12.0) as f64,
            bold: a.bold.unwrap_or(false),
            italic: a.italic.unwrap_or(false),
            underline: a.underline.unwrap_or(false),
            align: a.alignment.unwrap_or(0),
            show_font: true,
        })
    }

    fn commands(&self) -> Vec<Value> {
        vec![
            json!({"command": "text.setText", "params": {"id": self.id, "text": self.text.replace("\r\n", "\r").replace('\n', "\r")}}),
            json!({"command": "text.setAttributes", "params": {"ids": [self.id], "font": self.font, "size": self.size, "bold": self.bold, "italic": self.italic, "underline": self.underline, "alignment": self.align}}),
        ]
    }
}

/// Zeigt den Texteditor.
pub fn show_text_editor(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.text_editor.clone() else {
        return;
    };
    let mut result = None;
    let mut apply = false;
    let r = dialog_frame_resizable(
        ctx,
        "fp-texteditor",
        &t("dlg-te-title"),
        vec2(560.0, 380.0),
        |ui, body| {
            let theme = Theme::current(ui.ctx());
            let c = &theme.colors;
            let o = body.min + vec2(12.0, 12.0);
            let fams = crate::inspector::fonts::families();
            let (family, style) = crate::inspector::fonts::lookup(&d.font);
            let names: Vec<&str> = fams.iter().map(|f| f.name.as_str()).collect();
            if let Some(i) = crate::inspector::fonts::family_combo(
                ui,
                "te-font",
                Rect::from_min_size(o, vec2(180.0, 20.0)),
                &names,
                names.iter().position(|n| *n == family),
                true,
            ) {
                d.font = crate::inspector::fonts::ps_name(names[i], &style);
            }
            if let Some(v) = fields::number(
                ui,
                "te-size",
                Rect::from_min_size(o + vec2(186.0, 0.0), vec2(44.0, 20.0)),
                Some(Rect::from_min_size(o + vec2(232.0, 0.0), vec2(14.0, 20.0))),
                Some(d.size),
                (1.0, 5000.0),
                1,
                true,
            ) {
                d.size = v;
            }
            let toggles = [
                (Glyph::Bold, d.bold, t("dlg-te-bold")),
                (Glyph::Italic, d.italic, t("dlg-te-italic")),
                (Glyph::Underline, d.underline, t("dlg-te-underline")),
            ];
            for (k, (g, on, tip)) in toggles.iter().enumerate() {
                if fields::toggle(
                    ui,
                    ("te-st", k),
                    Rect::from_min_size(o + vec2(256.0 + k as f32 * 24.0, 0.0), vec2(22.0, 20.0)),
                    *g,
                    *on,
                    true,
                    tip,
                ) {
                    match k {
                        0 => d.bold = !d.bold,
                        1 => d.italic = !d.italic,
                        _ => d.underline = !d.underline,
                    }
                }
            }
            for (k, (g, tip)) in [
                (Glyph::AlignLeft, t("dlg-te-align-left")),
                (Glyph::AlignCenter, t("dlg-te-align-center")),
                (Glyph::AlignRight, t("dlg-te-align-right")),
                (Glyph::AlignJustify, t("dlg-te-align-justify")),
                (Glyph::AlignStretch, t("dlg-te-align-stretch")),
            ]
            .iter()
            .enumerate()
            {
                if fields::toggle(
                    ui,
                    ("te-al", k),
                    Rect::from_min_size(o + vec2(338.0 + k as f32 * 24.0, 0.0), vec2(22.0, 20.0)),
                    *g,
                    d.align == k as i32,
                    true,
                    tip,
                ) {
                    d.align = k as i32;
                }
            }
            let tr = Rect::from_min_max(
                o + vec2(0.0, 30.0),
                pos2(body.right() - 12.0, body.bottom() - 70.0),
            );
            ui.painter().rect_filled(tr, 0.0, c.field_bg);
            ui.painter().rect_stroke(
                tr,
                0.0,
                Stroke::new(1.0, c.field_border),
                StrokeKind::Inside,
            );
            let font = if d.show_font {
                egui::FontId::proportional((d.size as f32).clamp(8.0, 48.0))
            } else {
                theme.ui_font()
            };
            ui.put(
                tr.shrink(4.0),
                egui::TextEdit::multiline(&mut d.text)
                    .frame(egui::Frame::NONE)
                    .font(font)
                    .desired_width(tr.width() - 8.0),
            );
            let show_l = t("dlg-te-show-font");
            let sw = (18.0 + super::text_w(ui.ctx(), &show_l)).max(160.0);
            if let Some(b) = fields::checkbox(
                ui,
                "te-show",
                Rect::from_min_size(pos2(o.x, body.bottom() - 60.0), vec2(sw, 16.0)),
                &show_l,
                d.show_font,
                true,
            ) {
                d.show_font = b;
            }
            ui.painter().text(
                pos2(o.x + sw + 10.0, body.bottom() - 52.0),
                Align2::LEFT_CENTER,
                fp_i18n::tr!("dlg-te-chars", n = d.text.chars().count()),
                theme.small_font(),
                c.text_weak,
            );
            let (apply_l, ok_l, cancel_l) = (t("dlg-te-apply"), t("btn-ok"), t("btn-cancel"));
            let b = super::button_row(
                ui.ctx(),
                body.right() - 16.0,
                body.bottom() - 36.0,
                &[&apply_l, &ok_l, &cancel_l],
                76.0,
                23.0,
                8.0,
            );
            if push_button(ui, b[0], &apply_l, false, true) {
                apply = true;
            }
            if push_button(ui, b[1], &ok_l, true, true) {
                result = Some(true);
            }
            if push_button(ui, b[2], &cancel_l, false, true) {
                result = Some(false);
            }
        },
    );
    if apply || result == Some(true) {
        // Schrift aus dem Texteditor in „zuletzt benutzt“ des Schrift-Menüs (BA-061)
        if !d.font.is_empty() {
            crate::menu::note_recent_font(st, &d.font);
        }
        st.push(Action::Run(
            "history.replay".into(),
            json!({"commands": d.commands(), "label": t("dlg-te-title")}),
        ));
    }
    match (result, r) {
        (Some(_), _) | (_, DialogClose::Cancelled) => st.dialogs.text_editor = None,
        _ => st.dialogs.text_editor = Some(d),
    }
}

// ---------------------------------------------------------------------- HTML-Segment

/// Zeigt „HTML-Segment bearbeiten“ (14139).
pub fn show_html_slice(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut text) = st.dialogs.html_slice.clone() else {
        return;
    };
    let mut result = None;
    let (ok_l, cancel_l) = (t("btn-ok"), t("btn-cancel"));
    let r = dialog_frame_resizable(
        ctx,
        "fp-htmlslice",
        &t("dlg-html-slice-title"),
        vec2(460.0, 300.0),
        |ui, body| {
            let theme = Theme::current(ui.ctx());
            let tr = Rect::from_min_max(
                body.min + vec2(12.0, 12.0),
                pos2(body.right() - 12.0, body.bottom() - 48.0),
            );
            ui.painter().rect_filled(tr, 0.0, theme.colors.field_bg);
            ui.put(
                tr.shrink(3.0),
                egui::TextEdit::multiline(&mut text)
                    .frame(egui::Frame::NONE)
                    .font(egui::FontId::monospace(12.0))
                    .desired_width(tr.width() - 6.0),
            );
            let b = super::button_row(
                ui.ctx(),
                body.right() - 16.0,
                body.bottom() - 36.0,
                &[&ok_l, &cancel_l],
                76.0,
                23.0,
                8.0,
            );
            if push_button(ui, b[0], &ok_l, true, true) {
                result = Some(true);
            }
            if push_button(ui, b[1], &cancel_l, false, true) {
                result = Some(false);
            }
        },
    );
    match (result, r) {
        (Some(true), _) => {
            st.dialogs.html_slice = None;
            st.push(Action::Run("pi.web".into(), json!({"html": text})));
        }
        (Some(false), _) | (_, DialogClose::Cancelled) => st.dialogs.html_slice = None,
        _ => st.dialogs.html_slice = Some(text),
    }
}

// ---------------------------------------------------------------------- Neuer Stil

/// „Neuer Stil“ (19123): Name und Eigenschaftsgruppen. Wie im Original (Lauf 10,
/// `panel-library-styles/11`, TW-92; M10 re 43 §3) ist der Name „Stil“ (ohne Nummer) vorbelegt und
/// markiert; bleibt das Feld leer, heißt der Stil „Stil n“ (eigene Absicherung; das Original prüft
/// nicht). Füllung, Strich und Effekt sind an, die vier Text-Kästchen, wenn das **oberste** gewählte
/// Element ein Text ist oder nichts gewählt ist (re 43 §3); alle Kästchen bleiben bedienbar.
pub fn new_style_form(st: &AppState) -> FormDialog {
    let n = st
        .session
        .active()
        .and_then(|d| d.doc.styles.as_ref().map(|s| s.len()))
        .unwrap_or(0)
        + 1;
    let text =
        !matches!(top_selected(st), Some(e) if !matches!(e.kind, fp_model::ElementKind::Text(_)));
    let fields = vec![
        Field::text("name", &t("dlg-lbl-name-ak"), &t("dlg-style-name-default")),
        Field::caption(t("dlg-style-cap-props").as_str()),
        Field::check("_fillType", t("dlg-style-fill-type").as_str(), true),
        Field::check("_fillColor", t("dlg-style-fill-color").as_str(), true),
        Field::check("_effect", t("dlg-style-effect").as_str(), true),
        Field::check("_strokeType", t("dlg-style-stroke-type").as_str(), true),
        Field::check("_strokeColor", t("dlg-style-stroke-color").as_str(), true),
        Field::check("_font", t("dlg-style-font").as_str(), text),
        Field::check("_size", t("dlg-style-size").as_str(), text),
        Field::check("_style", t("dlg-style-style").as_str(), text),
        Field::check("_other", t("dlg-style-other").as_str(), text),
    ];
    FormDialog::new(
        "new-style",
        &t("dlg-new-style-title"),
        "pi.style.new",
        fields,
    )
    .with_base(json!({ "_defaultName": fp_i18n::tr!("dlg-style-default-name", n = n) }))
    .with_transform(style_params)
    .with_layout(NEW_STYLE_LAYOUT)
}

/// Oberstes gewähltes Element in der Stapelreihenfolge (`fw.selection[0]`, M10 re 43 §3); `None`
/// ohne Auswahl.
fn top_selected(st: &AppState) -> Option<fp_model::Element> {
    let d = st.session.active()?;
    let id = fp_engine::docops::top_selected(&d.doc, d.page, &d.selection.elements)?;
    fp_engine::docops::element_on_page(&d.doc, d.page, id).map(|e| (*e).clone())
}

/// Mustertext der Miniatur wie im Original (sprachunabhängig, re 43 §3).
const STYLE_PREVIEW_TEXT: &str = "ABC";

/// Miniatur von „Neuer Stil“ (re 43 §3): Quadrat mit Füllung/Strich des obersten gewählten
/// Elements, bei Text (und ohne Auswahl, aktuelle Vorgaben) „ABC“ im Textstil.
#[derive(Clone, Debug, PartialEq)]
pub struct StylePreview {
    pub text: bool,
    pub fill: Option<egui::Color32>,
    pub stroke: Option<egui::Color32>,
    pub underline: bool,
}

fn argb32(c: fp_model::Argb) -> egui::Color32 {
    let v = c.0;
    egui::Color32::from_rgba_unmultiplied((v >> 16) as u8, (v >> 8) as u8, v as u8, (v >> 24) as u8)
}

/// Vorschau der Miniatur aus dem obersten gewählten Element bzw. den aktuellen Vorgaben.
pub fn style_preview(st: &AppState) -> StylePreview {
    match top_selected(st) {
        Some(e) => {
            let a = fp_engine::docops::path_attrs_of(&e);
            let fill = a
                .as_ref()
                .filter(|a| a.fill.is_some())
                .map(|a| argb32(a.fill_color));
            let stroke = a
                .as_ref()
                .filter(|a| a.brush.is_some())
                .map(|a| argb32(a.stroke_color));
            match &e.kind {
                fp_model::ElementKind::Text(t) => {
                    let first = t
                        .resolved_runs()
                        .first()
                        .map(|(a, _)| a.clone())
                        .unwrap_or_default();
                    StylePreview {
                        text: true,
                        fill: fill.or(Some(egui::Color32::BLACK)),
                        stroke,
                        underline: first.underline.unwrap_or(false),
                    }
                }
                _ => StylePreview {
                    text: false,
                    fill,
                    stroke,
                    underline: false,
                },
            }
        }
        None => {
            let (_, fill) = st.session.prefs.current_colors();
            StylePreview {
                text: true,
                fill: Some(argb32(fill)),
                stroke: None,
                underline: false,
            }
        }
    }
}

/// Vorlage 19123 in DLU (`dialoge.json`): Client-Bereich 290 × 127, Miniatur links (Rahmen 5922),
/// geätzte Linie bei x 66, Beschriftungen bei x 70, Kästchen in zwei Spalten bei x 131 und 208.
const NS_SIZE: [f32; 2] = [290.0, 127.0];
const NS_NAME_LABEL: [f32; 4] = [70.0, 8.0, 58.0, 8.0];
const NS_NAME_EDIT: [f32; 4] = [131.0, 6.0, 153.0, 12.0];
const NS_THUMB: [f32; 4] = [7.0, 10.0, 51.0, 45.0];
const NS_PROPS_LABEL: [f32; 4] = [70.0, 26.0, 58.0, 11.0];
const NS_LINE: [f32; 4] = [66.0, 5.0, 1.0, 115.0];
/// Kästchen in Feldreihenfolge (`_fillType` … `_other`).
const NS_CHECKS: [[f32; 4]; 9] = [
    [131.0, 26.0, 74.0, 10.0],
    [131.0, 38.0, 74.0, 10.0],
    [131.0, 55.0, 74.0, 10.0],
    [208.0, 26.0, 74.0, 10.0],
    [208.0, 38.0, 74.0, 10.0],
    [208.0, 55.0, 74.0, 10.0],
    [208.0, 67.0, 74.0, 10.0],
    [208.0, 79.0, 74.0, 10.0],
    [208.0, 90.0, 74.0, 10.0],
];
/// OK und Abbrechen (je 50 × 14 DLU).
const NS_OK: [f32; 4] = [177.0, 109.0, 50.0, 14.0];
const NS_CANCEL: [f32; 4] = [235.0, 109.0, 50.0, 14.0];

/// Verschiebung nach rechts, wenn Beschriftungen in der Sprache länger sind als in der Vorlage:
/// (Spalte 1, Spalte 2, zusätzliche Breite des Dialogs).
fn ns_shift(ctx: &egui::Context, d: &FormDialog) -> (f32, f32, f32) {
    let labels: Vec<String> = d
        .fields
        .iter()
        .map(|f| super::access::plain(&f.label))
        .collect();
    if labels.len() < 11 {
        return (0.0, 0.0, 0.0);
    }
    let lab_w = super::max_text_w(ctx, &[labels[0].as_str(), labels[1].as_str()]);
    let room = super::dlu(NS_NAME_EDIT[0] - 3.0 - NS_NAME_LABEL[0], 0.0).x;
    let dx1 = (lab_w - room).max(0.0).ceil();
    let col = |r: std::ops::Range<usize>| {
        labels[r]
            .iter()
            .map(|l| super::text_w(ctx, l) + 18.0)
            .fold(0.0f32, f32::max)
    };
    let col_w = super::dlu(NS_CHECKS[0][2], 0.0).x;
    let dx2 = dx1 + (col(2..5) - col_w).max(0.0).ceil();
    let extra = dx2 + (col(5..11) - col_w).max(0.0).ceil();
    (dx1, dx2, extra)
}

fn ns_size(ctx: &egui::Context, d: &FormDialog) -> egui::Vec2 {
    let (_, _, extra) = ns_shift(ctx, d);
    vec2(
        (super::dlu(NS_SIZE[0], 0.0).x + extra + ns_button_extra(ctx)).ceil(),
        super::dlu(0.0, NS_SIZE[1]).y.ceil(),
    )
}

/// Zusätzliche Breite, wenn OK/Abbrechen in der Sprache breiter sind als 50 DLU (Knöpfe gleich breit,
/// rechtsbündig wie in der Vorlage).
fn ns_button_extra(ctx: &egui::Context) -> f32 {
    let (ok, cancel) = (t("btn-ok"), t("btn-cancel"));
    let gap = super::dlu(NS_CANCEL[0] - NS_OK[0] - NS_OK[2], 0.0).x;
    let need = super::button_row_w(ctx, &[&ok, &cancel], super::dlu(NS_OK[2], 0.0).x, gap);
    let room = super::dlu(NS_CANCEL[0] + NS_CANCEL[2] - NS_OK[0], 0.0).x;
    (need - room).max(0.0).ceil()
}

/// OK und Abbrechen: Vorlagenlage, bei längeren Beschriftungen breiter (gleich breit, rechtsbündig).
fn ns_buttons(ctx: &egui::Context, o: egui::Pos2, extra: f32) -> [Rect; 2] {
    let (ok, cancel) = (t("btn-ok"), t("btn-cancel"));
    let okr = super::dlu_rect(o, NS_OK);
    let right = super::dlu_rect(o, NS_CANCEL).right() + extra + ns_button_extra(ctx);
    let gap = super::dlu(NS_CANCEL[0] - NS_OK[0] - NS_OK[2], 0.0).x;
    let v = super::button_row(
        ctx,
        right,
        okr.top(),
        &[&ok, &cancel],
        okr.width(),
        okr.height(),
        gap,
    );
    [v[0], v[1]]
}

fn ns_draw(ui: &mut egui::Ui, st: &AppState, d: &mut FormDialog, body: Rect) -> (bool, [Rect; 2]) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let o = body.min;
    let (dx1, dx2, extra) = ns_shift(ui.ctx(), d);
    let r = |a: [f32; 4]| super::dlu_rect(o, a);
    let gap = super::dlu(3.0, 0.0).x;
    // Miniatur: weißes Feld mit der Vorschau des Stils (re 43 §3: Quadrat bzw. „ABC“)
    let th = r(NS_THUMB);
    ui.painter().rect_filled(th, 0.0, egui::Color32::WHITE);
    ui.painter().rect_stroke(
        th,
        0.0,
        Stroke::new(1.0, c.field_border),
        StrokeKind::Inside,
    );
    let pv = style_preview(st);
    if pv.text {
        let col = pv.fill.unwrap_or(egui::Color32::BLACK);
        let g = ui.painter().text(
            th.center(),
            Align2::CENTER_CENTER,
            STYLE_PREVIEW_TEXT,
            egui::FontId::proportional((th.height() * 0.42).round()),
            col,
        );
        if pv.underline {
            ui.painter()
                .hline(g.x_range(), g.bottom() - 1.5, Stroke::new(1.0, col));
        }
    } else {
        let side = th.width().min(th.height()) * 0.76;
        let sq = Rect::from_center_size(th.center(), vec2(side, side));
        if let Some(f) = pv.fill {
            ui.painter().rect_filled(sq, 0.0, f);
        }
        if let Some(s) = pv.stroke {
            ui.painter()
                .rect_stroke(sq, 0.0, Stroke::new(1.0, s), StrokeKind::Inside);
        }
    }
    // geätzte Trennlinie
    let lr = r(NS_LINE);
    ui.painter().vline(
        lr.left() + 0.5,
        lr.y_range(),
        Stroke::new(1.0, c.list_row_line),
    );
    let mut changed = false;
    if d.fields.len() < 11 {
        return (changed, ns_buttons(ui.ctx(), o, 0.0));
    }
    // Name
    let nl = r(NS_NAME_LABEL);
    let ne = r(NS_NAME_EDIT);
    let ne = Rect::from_min_max(
        pos2(ne.left() + dx1, ne.top() - 2.0),
        pos2(ne.right() + extra, ne.bottom() + 2.0),
    );
    let name_label = d.fields[0].label.clone();
    super::access::label_right(
        ui,
        Rect::from_min_max(
            pos2(nl.left(), ne.top()),
            pos2(ne.left() - gap, ne.bottom()),
        ),
        &name_label,
        true,
    );
    if super::access::pressed(ui, &name_label) {
        super::access::focus_field(
            ui,
            ("new-style", 0usize),
            d.fields[0].value.as_str().map_or(0, |v| v.chars().count()),
        );
    }
    let cur = d.fields[0].value.as_str().unwrap_or("").to_string();
    if let Some(v) = fields::text(ui, ("new-style", 0usize), ne, &cur, true) {
        d.fields[0].value = json!(v);
        changed = true;
    }
    // „Eigenschaften:“ rechtsbündig vor der ersten Kästchenspalte
    let pl = r(NS_PROPS_LABEL);
    let first = r(NS_CHECKS[0]);
    let props = d.fields[1].label.clone();
    fields::label_right(
        ui,
        Rect::from_min_max(
            pos2(pl.left(), first.center().y - 8.0),
            pos2(first.left() + dx1 - gap, first.center().y + 8.0),
        ),
        &props,
        true,
    );
    // Kästchen in zwei Spalten
    for (k, a) in NS_CHECKS.iter().enumerate() {
        let i = k + 2;
        let dx = if k < 3 { dx1 } else { dx2 };
        let cr = r(*a).translate(vec2(dx, 0.0));
        let label = d.fields[i].label.clone();
        let w = cr
            .width()
            .max(super::text_w(ui.ctx(), &super::access::plain(&label)) + 18.0);
        let cr = Rect::from_center_size(pos2(cr.left() + w / 2.0, cr.center().y), vec2(w, 16.0));
        let on = d.fields[i].value.as_bool().unwrap_or(false);
        if let Some(b) = super::access::checkbox(ui, ("new-style", i), cr, &label, on, true) {
            d.fields[i].value = json!(b);
            changed = true;
        }
    }
    (changed, ns_buttons(ui.ctx(), o, extra))
}

/// Anordnung von „Neuer Stil“ (19123, TW-93): Miniatur links, Kästchen in zwei Spalten.
pub const NEW_STYLE_LAYOUT: super::form::CustomLayout = super::form::CustomLayout {
    size: ns_size,
    draw: ns_draw,
};

/// Parameter für `pi.style.new` (ZSFi/ZSOi-Bits wie in fp-model `Style`).
pub fn style_params(d: &FormDialog) -> Value {
    let on = |k: &str| d.get(k).as_bool().unwrap_or(false);
    // „Füllungstyp“ schaltet wie im Original zwei Bits (0x44, re 39 §3.3)
    let mut flags = 0x1B0u32;
    let mut other = 0u32;
    for (k, bit) in [
        ("_strokeType", 1),
        ("_strokeColor", 2),
        ("_fillType", 0x44),
        ("_fillColor", 8),
    ] {
        if on(k) {
            flags |= bit;
        }
    }
    for (k, bit) in [
        ("_font", 1),
        ("_size", 2),
        ("_style", 4),
        ("_effect", 8),
        ("_other", 16),
    ] {
        if on(k) {
            other |= bit;
        }
    }
    // leeres Namensfeld: Vorgabename „Stil n“ (TW-48, TW-92)
    let name = match d.get("name") {
        Value::String(s) if !s.trim().is_empty() => Value::String(s),
        _ => d.base.get("_defaultName").cloned().unwrap_or(Value::Null),
    };
    json!({"name": name, "flags": flags, "otherFlags": other})
}

/// Hinweis, wenn ein Dialog in der aktuellen Lage nicht geöffnet werden kann.
pub fn not_available_message(st: &mut AppState, what: &str, why: &str) {
    st.dialogs.message = Some((what.to_string(), why.to_string()));
}

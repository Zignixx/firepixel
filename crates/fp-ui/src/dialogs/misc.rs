//! Weitere Dialoge: Texteditor (5962), Rückfrage mit Befehl (z. B. Symbol löschen, 6003),
//! HTML-Segment bearbeiten (14139), Strich bearbeiten (5963–5965, vereinfacht), Neuer Stil
//! (19123).

use super::form::{Field, FormDialog};
use super::{dialog_frame, dialog_frame_resizable, push_button, t, DialogClose};
use crate::app_state::{Action, AppState};
use crate::inspector::fields;
use crate::inspector::glyphs::Glyph;
use crate::theme::Theme;
use egui::{pos2, vec2, Align2, Rect, Stroke, StrokeKind};
use serde_json::{json, Value};

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
    let Some(c) = st.dialogs.confirm.clone() else { return };
    let mut result = None;
    let (yes, no) = (t("btn-yes"), t("btn-no"));
    use crate::ui_kit::{self, sp, BarButton, KitIcon};
    let w = 400f32.max(ui_kit::button_bar_width(ctx, &[], &[BarButton::default_button(&yes), BarButton::new(&no)]));
    // Gestaltung (docs\DESIGN.md §3, Rückfrage): Warnsymbol 32 px links, Text daneben (mittig zum
    // Symbol, längere Texte umbrechen und lassen den Dialog wachsen), Knopfleiste
    let text_x = sp::L + 32.0 + sp::M;
    let th = super::wrapped_h(ctx, &c.text, w - text_x - sp::L);
    let content_h = th.max(32.0);
    let r = dialog_frame(ctx, "fp-confirm", &c.title, vec2(w, (sp::L + content_h + sp::L + sp::BUTTON_BAR_H).ceil()), |ui, body| {
        let theme = Theme::current(ui.ctx());
        let icon = Rect::from_min_size(body.min + vec2(sp::L, sp::L), vec2(32.0, 32.0));
        ui_kit::paint_icon(ui.painter(), icon, KitIcon::Warning, &crate::icons::IconStyle::new(&theme, None, true));
        let ty = if th < 32.0 { icon.center().y - th / 2.0 } else { icon.top() };
        super::put_left(ui, Rect::from_min_size(pos2(body.left() + text_x, ty), vec2(body.width() - text_x - sp::L, th + 4.0)), egui::Label::new(c.text.as_str()).wrap());
        let bar = ui_kit::button_bar(ui, body, &[], &[BarButton::default_button(&yes), BarButton::new(&no)]);
        if bar.right == Some(0) || access(ui, super::access_key(&yes)) {
            result = Some(true);
        }
        if bar.right == Some(1) || access(ui, super::access_key(&no)) {
            result = Some(false);
        }
    });
    match (result, r) {
        (Some(true), _) => {
            st.dialogs.confirm = None;
            st.push(Action::Run(c.command, c.params));
        }
        (Some(false), _) | (_, DialogClose::Cancelled) => st.dialogs.confirm = None,
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
        let e = crate::inspector::obj_quads::selection(st).into_iter().find(|e| matches!(e.kind, fp_model::ElementKind::Text(_)))?;
        let fp_model::ElementKind::Text(t) = &e.kind else { return None };
        let a = t.resolved_runs().first().map(|(a, _)| a.clone()).unwrap_or_else(|| t.initial.clone());
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
    let Some(mut d) = st.dialogs.text_editor.clone() else { return };
    let mut result = None;
    let mut apply = false;
    let r = dialog_frame_resizable(ctx, "fp-texteditor", &t("dlg-te-title"), vec2(560.0, 380.0), |ui, body| {
        let theme = Theme::current(ui.ctx());
        let c = &theme.colors;
        let o = body.min + vec2(12.0, 12.0);
        let fams = crate::inspector::fonts::families();
        let (family, style) = crate::inspector::fonts::lookup(&d.font);
        let names: Vec<&str> = fams.iter().map(|f| f.name.as_str()).collect();
        if let Some(i) = crate::inspector::fonts::family_combo(ui, "te-font", Rect::from_min_size(o, vec2(180.0, 20.0)), &names, names.iter().position(|n| *n == family), true) {
            d.font = crate::inspector::fonts::ps_name(names[i], &style);
        }
        if let Some(v) = fields::number(ui, "te-size", Rect::from_min_size(o + vec2(186.0, 0.0), vec2(44.0, 20.0)), Some(Rect::from_min_size(o + vec2(232.0, 0.0), vec2(14.0, 20.0))), Some(d.size), (1.0, 5000.0), 1, true) {
            d.size = v;
        }
        let toggles = [(Glyph::Bold, d.bold, t("dlg-te-bold")), (Glyph::Italic, d.italic, t("dlg-te-italic")), (Glyph::Underline, d.underline, t("dlg-te-underline"))];
        for (k, (g, on, tip)) in toggles.iter().enumerate() {
            if fields::toggle(ui, ("te-st", k), Rect::from_min_size(o + vec2(256.0 + k as f32 * 24.0, 0.0), vec2(22.0, 20.0)), *g, *on, true, tip) {
                match k {
                    0 => d.bold = !d.bold,
                    1 => d.italic = !d.italic,
                    _ => d.underline = !d.underline,
                }
            }
        }
        for (k, (g, tip)) in [(Glyph::AlignLeft, t("dlg-te-align-left")), (Glyph::AlignCenter, t("dlg-te-align-center")), (Glyph::AlignRight, t("dlg-te-align-right")), (Glyph::AlignJustify, t("dlg-te-align-justify")), (Glyph::AlignStretch, t("dlg-te-align-stretch"))].iter().enumerate() {
            if fields::toggle(ui, ("te-al", k), Rect::from_min_size(o + vec2(338.0 + k as f32 * 24.0, 0.0), vec2(22.0, 20.0)), *g, d.align == k as i32, true, tip) {
                d.align = k as i32;
            }
        }
        let tr = Rect::from_min_max(o + vec2(0.0, 30.0), pos2(body.right() - 12.0, body.bottom() - 70.0));
        ui.painter().rect_filled(tr, 0.0, c.field_bg);
        ui.painter().rect_stroke(tr, 0.0, Stroke::new(1.0, c.field_border), StrokeKind::Inside);
        let font = if d.show_font { egui::FontId::proportional((d.size as f32).clamp(8.0, 48.0)) } else { theme.ui_font() };
        ui.put(tr.shrink(4.0), egui::TextEdit::multiline(&mut d.text).frame(egui::Frame::NONE).font(font).desired_width(tr.width() - 8.0));
        let show_l = t("dlg-te-show-font");
        let sw = (18.0 + super::text_w(ui.ctx(), &show_l)).max(160.0);
        if let Some(b) = fields::checkbox(ui, "te-show", Rect::from_min_size(pos2(o.x, body.bottom() - 60.0), vec2(sw, 16.0)), &show_l, d.show_font, true) {
            d.show_font = b;
        }
        ui.painter().text(pos2(o.x + sw + 10.0, body.bottom() - 52.0), Align2::LEFT_CENTER, fp_i18n::tr!("dlg-te-chars", n = d.text.chars().count()), theme.small_font(), c.text_weak);
        let (apply_l, ok_l, cancel_l) = (t("dlg-te-apply"), t("btn-ok"), t("btn-cancel"));
        let b = super::button_row(ui.ctx(), body.right() - 16.0, body.bottom() - 36.0, &[&apply_l, &ok_l, &cancel_l], 76.0, 23.0, 8.0);
        if push_button(ui, b[0], &apply_l, false, true) {
            apply = true;
        }
        if push_button(ui, b[1], &ok_l, true, true) {
            result = Some(true);
        }
        if push_button(ui, b[2], &cancel_l, false, true) {
            result = Some(false);
        }
    });
    if apply || result == Some(true) {
        st.push(Action::Run("history.replay".into(), json!({"commands": d.commands(), "label": t("dlg-te-title")})));
    }
    match (result, r) {
        (Some(_), _) | (_, DialogClose::Cancelled) => st.dialogs.text_editor = None,
        _ => st.dialogs.text_editor = Some(d),
    }
}

// ---------------------------------------------------------------------- HTML-Segment

/// Zeigt „HTML-Segment bearbeiten“ (14139).
pub fn show_html_slice(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut text) = st.dialogs.html_slice.clone() else { return };
    let mut result = None;
    let (ok_l, cancel_l) = (t("btn-ok"), t("btn-cancel"));
    let r = dialog_frame_resizable(ctx, "fp-htmlslice", &t("dlg-html-slice-title"), vec2(460.0, 300.0), |ui, body| {
        let theme = Theme::current(ui.ctx());
        let tr = Rect::from_min_max(body.min + vec2(12.0, 12.0), pos2(body.right() - 12.0, body.bottom() - 48.0));
        ui.painter().rect_filled(tr, 0.0, theme.colors.field_bg);
        ui.put(tr.shrink(3.0), egui::TextEdit::multiline(&mut text).frame(egui::Frame::NONE).font(egui::FontId::monospace(12.0)).desired_width(tr.width() - 6.0));
        let b = super::button_row(ui.ctx(), body.right() - 16.0, body.bottom() - 36.0, &[&ok_l, &cancel_l], 76.0, 23.0, 8.0);
        if push_button(ui, b[0], &ok_l, true, true) {
            result = Some(true);
        }
        if push_button(ui, b[1], &cancel_l, false, true) {
            result = Some(false);
        }
    });
    match (result, r) {
        (Some(true), _) => {
            st.dialogs.html_slice = None;
            st.push(Action::Run("pi.web".into(), json!({"html": text})));
        }
        (Some(false), _) | (_, DialogClose::Cancelled) => st.dialogs.html_slice = None,
        _ => st.dialogs.html_slice = Some(text),
    }
}

// ---------------------------------------------------------------------- Strich bearbeiten

/// „Strich bearbeiten“ (5963 Optionen, 5964 Form, 5965 Empfindlichkeit – hier die Felder, die
/// das Modell trägt, Empfindlichkeit über `element.setBrushSensitivity`). Wirkt mit Vorschau
/// auf den Pinsel der Auswahl; `None`, wenn kein Objekt mit Strich ausgewählt ist.
pub fn stroke_options_form(st: &AppState) -> Option<FormDialog> {
    let sel = crate::inspector::obj_quads::selection(st);
    // ohne Auswahl: Pinsel der Werkzeugvorgabe (`ToolOptions::new_brush`, ohne Vorschau)
    if sel.is_empty() {
        let b = fp_engine::commands::panel::tool_brush(&st.session.tools.options);
        return Some(FormDialog::new("stroke-options", &t("dlg-stroke-title"), "pi.tool.brush", brush_fields(&b)).with_width(380.0).with_transform(tool_stroke_params));
    }
    let b = sel.iter().find_map(|e| fp_engine::docops::path_attrs_of(e).and_then(|a| a.brush.clone()))?;
    Some(FormDialog::new("stroke-options", &t("dlg-stroke-title"), "pi.brush", brush_fields(&b)).with_preview().with_width(380.0).with_transform(stroke_params))
}

/// Felder von „Strich bearbeiten“ mit den Werten des Pinsels `b`.
fn brush_fields(b: &fp_model::Brush) -> Vec<Field> {
    let pct = |v: i32| (v as f64 / 10.0).round();
    let fields = vec![
        Field::caption(t("dlg-brush-cap-tip").as_str()),
        Field::slider("diameter", t("dlg-brush-tip-size").as_str(), b.diameter as f64, 1.0, 1000.0).unit("px"),
        Field::slider("softness", t("dlg-brush-edge-softness").as_str(), pct(b.softness), 0.0, 100.0).unit("%"),
        Field::choice_of("shape", t("dlg-brush-shape").as_str(), &[t("dlg-brush-shape-square").as_str(), t("dlg-brush-shape-round").as_str()], vec![json!(0), json!(1)], usize::from(b.shape != 0)),
        Field::slider("aspect", t("dlg-brush-aspect").as_str(), pct(b.aspect).max(1.0), 1.0, 100.0).unit("%"),
        Field::slider("angle", t("dlg-lbl-angle").as_str(), b.angle as f64, 0.0, 359.0).unit("°"),
        Field::caption(t("dlg-brush-cap-ink").as_str()),
        Field::slider("spacing", t("dlg-lbl-distance").as_str(), pct(b.spacing), 0.0, 1000.0).unit("%"),
        Field::slider("flowRate", t("dlg-brush-flow").as_str(), pct(b.flow_rate), 0.0, 100.0).unit("%"),
        Field::number("tipCount", t("dlg-brush-tips").as_str(), b.tip_count.max(1) as f64, 1.0, 32.0),
        Field::caption(t("dlg-brush-cap-dash").as_str()),
        Field::choice_of("dashCount", t("dlg-brush-dash").as_str(), &[t("dlg-dash-none").as_str(), t("dlg-dash-one").as_str(), t("dlg-dash-two").as_str(), t("dlg-dash-three").as_str()], vec![json!(0), json!(1), json!(2), json!(3)], b.dash_count.clamp(0, 3) as usize),
        Field::number("_on0", t("dlg-brush-dash-on-1").as_str(), b.dash_on[0] as f64, 0.0, 1000.0).unit("px"),
        Field::number("_off0", t("dlg-brush-dash-off-1").as_str(), b.dash_off[0] as f64, 0.0, 1000.0).unit("px"),
        Field::number("_on1", t("dlg-brush-dash-on-2").as_str(), b.dash_on[1] as f64, 0.0, 1000.0).unit("px"),
        Field::number("_off1", t("dlg-brush-dash-off-2").as_str(), b.dash_off[1] as f64, 0.0, 1000.0).unit("px"),
        Field::number("_on2", t("dlg-brush-dash-on-3").as_str(), b.dash_on[2] as f64, 0.0, 1000.0).unit("px"),
        Field::number("_off2", t("dlg-brush-dash-off-3").as_str(), b.dash_off[2] as f64, 0.0, 1000.0).unit("px"),
        Field::caption(t("dlg-brush-cap-sensitivity").as_str()),
        Field::number("_PZ", t("dlg-brush-size-pressure").as_str(), pct(b.sensitivity(b'P', b'Z')), 0.0, 100.0).unit("%"),
        Field::number("_SZ", t("dlg-brush-size-speed").as_str(), pct(b.sensitivity(b'S', b'Z')), 0.0, 100.0).unit("%"),
        Field::number("_PO", t("dlg-brush-opacity-pressure").as_str(), pct(b.sensitivity(b'P', b'O')), 0.0, 100.0).unit("%"),
        Field::number("_SO", t("dlg-brush-opacity-speed").as_str(), pct(b.sensitivity(b'S', b'O')), 0.0, 100.0).unit("%"),
        Field::number("_RR", t("dlg-brush-scatter-random").as_str(), pct(b.sensitivity(b'R', b'R')), 0.0, 100.0).unit("%"),
    ];
    fields
}

/// Parameter für `pi.tool.brush`: Pinselfelder und Empfindlichkeiten in einem Aufruf.
fn tool_stroke_params(d: &FormDialog) -> Value {
    let v = stroke_params(d);
    let mut m = v["commands"][0]["params"].as_object().cloned().unwrap_or_default();
    m.insert("sensitivity".into(), v["commands"][1]["params"]["values"].clone());
    Value::Object(m)
}

fn stroke_params(d: &FormDialog) -> Value {
    let mut m = serde_json::Map::new();
    for f in &d.fields {
        if !f.key.starts_with('_') {
            m.insert(f.key.clone(), f.param());
        }
    }
    let triple = |p: &str| -> Value { json!((0..3).map(|i| d.get(&format!("_{p}{i}")).as_f64().unwrap_or(0.0).round() as i64).collect::<Vec<_>>()) };
    m.insert("dashOn".into(), triple("on"));
    m.insert("dashOff".into(), triple("off"));
    // Empfindlichkeiten: zweiter Befehl im selben Verlaufsschritt
    let mut sens = serde_json::Map::new();
    for k in ["PZ", "SZ", "PO", "SO", "RR"] {
        sens.insert(k.into(), json!(d.get(&format!("_{k}")).as_f64().unwrap_or(0.0).round() as i64));
    }
    json!({"commands": [{"command": "pi.brush", "params": Value::Object(m)}, {"command": "element.setBrushSensitivity", "params": {"values": Value::Object(sens)}}], "label": t("dlg-stroke-title")})
}

// ---------------------------------------------------------------------- Neuer Stil

/// „Neuer Stil“ (19123): Name und Eigenschaftsgruppen.
pub fn new_style_form(st: &AppState) -> FormDialog {
    let n = st.session.active().and_then(|d| d.doc.styles.as_ref().map(|s| s.len())).unwrap_or(0) + 1;
    let fields = vec![
        Field::text("name", &t("dlg-lbl-name"), &fp_i18n::tr!("dlg-style-default-name", n = n)),
        Field::caption(t("dlg-style-cap-props").as_str()),
        Field::check("_fillType", t("dlg-style-fill-type").as_str(), true),
        Field::check("_fillColor", t("dlg-style-fill-color").as_str(), true),
        Field::check("_effect", t("dlg-style-effect").as_str(), true),
        Field::check("_strokeType", t("dlg-style-stroke-type").as_str(), true),
        Field::check("_strokeColor", t("dlg-style-stroke-color").as_str(), true),
        Field::check("_font", t("dlg-style-font").as_str(), true),
        Field::check("_size", t("dlg-style-size").as_str(), true),
        Field::check("_style", t("dlg-style-style").as_str(), true),
        Field::check("_other", t("dlg-style-other").as_str(), true),
    ];
    FormDialog::new("new-style", &t("dlg-new-style-title"), "pi.style.new", fields).with_transform(style_params)
}

/// Parameter für `pi.style.new` (ZSFi/ZSOi-Bits wie in fp-model `Style`).
pub fn style_params(d: &FormDialog) -> Value {
    let on = |k: &str| d.get(k).as_bool().unwrap_or(false);
    let mut flags = 0x1F0u32;
    let mut other = 0u32;
    for (k, bit) in [("_strokeType", 1), ("_strokeColor", 2), ("_fillType", 4), ("_fillColor", 8)] {
        if on(k) {
            flags |= bit;
        }
    }
    for (k, bit) in [("_font", 1), ("_size", 2), ("_style", 4), ("_effect", 8), ("_other", 16)] {
        if on(k) {
            other |= bit;
        }
    }
    json!({"name": d.get("name"), "flags": flags, "otherFlags": other})
}

/// Hinweis, wenn ein Dialog in der aktuellen Lage nicht geöffnet werden kann.
pub fn not_available_message(st: &mut AppState, what: &str, why: &str) {
    st.dialogs.message = Some((what.to_string(), why.to_string()));
}

//! „Pinselstrich bearbeiten“ (Registerkarten 5963 Optionen, 5964 Form, 5965 Empfindlichkeit;
//! Lage der Felder in DLU wie in `re\bedienfelder\_scratch\dialogs_full.json`). Wirkt auf den
//! Pinsel der Auswahl (`pi.brush {brush}`, ein Verlaufsschritt) oder ohne Auswahl bzw. aus dem
//! Werkzeug-Inspektor auf den Pinsel der Werkzeugvorgabe (`pi.tool.brush {brush}`, kein
//! Verlaufsschritt; Pinsel, Bleistift und Formwerkzeuge malen danach damit). Unten zeigt der Dialog
//! einen Probestrich mit dem bearbeiteten Pinsel (dieselbe Stempel-Engine wie beim Malen).
//!
//! Abstand und Farbfluss sind wie im Original nur bei natürlichen Pinseln bedienbar. Der Dialog
//! ändert die Pinselart (natürlich/einfach) nie: auch Werte, die nur die natürliche Engine kennt
//! (Spitzen, Variation, Breite, Winkel, Empfindlichkeiten), werden bei einem einfachen Pinsel nur
//! gespeichert (re 27 P1); ob sie wirken, regelt die Engine beim Malen. „Tintenmenge“ ist
//! `concentration` in Promille (re 27 P2), Anzeige Wert/10.

use super::{DialogClose, dialog_frame, dlu, dlu_rect, group_box, push_button, t};
use crate::app_state::AppState;
use crate::inspector::fields;
use egui::{Color32, Rect, Ui, vec2};
use fp_model::Brush;
use serde_json::{Value, json};

/// Quellen der Empfindlichkeit (Druck, Geschwindigkeit, horizontal, vertikal, zufällig).
pub const SOURCES: [u8; 5] = *b"PSHVR";
/// Ziele (Größe, Winkel, Deckkraft, Schwärze, Streuung, Farbton, Helligkeit, Sättigung).
pub const TARGETS: [u8; 8] = *b"ZAOBRHLS";

const TAB_KEYS: [&str; 3] = [
    "dlg-brushedit-tab-options",
    "dlg-brushedit-tab-shape",
    "dlg-brushedit-tab-sensitivity",
];
/// Randeffekte in der Reihenfolge der Liste des Originals (re\24 Nr. 10) und `alphaRemap`-Wert.
const EFFECTS: [(&str, i32); 6] = [
    ("dlg-brushedit-effect-none", 0),
    ("dlg-brushedit-effect-harsh-wet", 2),
    ("dlg-brushedit-effect-white-neon", 1),
    ("dlg-brushedit-effect-wavy-gravy", 4),
    ("dlg-brushedit-effect-smooth-neon", 3),
    ("dlg-brushedit-effect-white-neon-edge", 5),
];
const VARIATION_KEYS: [&str; 5] = [
    "dlg-brushedit-var-random",
    "dlg-brushedit-var-uniform",
    "dlg-brushedit-var-complementary",
    "dlg-brushedit-var-hue",
    "dlg-brushedit-var-shadow",
];
const DASH_KEYS: [&str; 4] = [
    "dlg-brushedit-dash-none",
    "dlg-brushedit-dash-one",
    "dlg-brushedit-dash-two",
    "dlg-brushedit-dash-three",
];
/// Liste „Pinselstricheigenschaft“ wie im Original (7 Einträge, re\24 Nr. 12) und Ziel in
/// [`TARGETS`]; der Eintrag „Tintenmenge“ steuert das Ziel `O` (Deckkraft, im Original gemessen: M6 §4c).
const PROPERTIES: [(&str, u8); 7] = [
    ("dlg-brushedit-prop-size", b'Z'),
    ("dlg-brushedit-prop-angle", b'A'),
    ("dlg-brushedit-prop-opacity", b'O'),
    ("dlg-brushedit-prop-scatter", b'R'),
    ("dlg-brushedit-prop-hue", b'H'),
    ("dlg-brushedit-prop-lightness", b'L'),
    ("dlg-brushedit-prop-saturation", b'S'),
];
const SOURCE_KEYS: [&str; 5] = [
    "dlg-brushedit-pressure",
    "dlg-brushedit-speed",
    "dlg-brushedit-horizontal",
    "dlg-brushedit-vertical",
    "dlg-brushedit-random",
];

/// Zustand des Dialogs.
#[derive(Clone, Debug, PartialEq)]
pub struct BrushEditDialog {
    /// Werkzeugvorgabe (`true`) oder Pinsel der Auswahl.
    pub tool: bool,
    /// Bearbeiteter Pinsel.
    pub brush: Brush,
    original: Brush,
    /// Registerkarte: 0 Optionen, 1 Form, 2 Empfindlichkeit.
    pub tab: usize,
    /// Empfindlichkeit: gewählte Eigenschaft (Index in der Liste „Pinselstricheigenschaft“).
    pub property: usize,
}

fn pct(v: i32) -> f64 {
    (v as f64 / 10.0).round()
}

fn permille(v: f64) -> i32 {
    (v * 10.0).round() as i32
}

impl BrushEditDialog {
    /// Für den Pinsel der Werkzeugvorgabe.
    pub fn for_tool(st: &AppState) -> BrushEditDialog {
        let b = fp_engine::commands::panel::tool_brush(&st.session.tools.options);
        BrushEditDialog {
            tool: true,
            brush: b.clone(),
            original: b,
            tab: 0,
            property: 0,
        }
    }

    /// Für den Pinsel der Auswahl; `None` ohne Objekt mit Strich.
    pub fn for_selection(st: &AppState) -> Option<BrushEditDialog> {
        let sel = crate::inspector::obj_quads::selection(st);
        let b = sel
            .iter()
            .find_map(|e| fp_engine::docops::path_attrs_of(e).and_then(|a| a.brush.clone()))?;
        Some(BrushEditDialog {
            tool: false,
            brush: b.clone(),
            original: b,
            tab: 0,
            property: 0,
        })
    }

    /// Empfindlichkeit `source` → `target` in Prozent.
    pub fn sensitivity(&self, source: u8, target: u8) -> f64 {
        pct(self.brush.sensitivity(source, target))
    }

    /// Setzt eine Empfindlichkeit (Prozent; 0 entfernt sie).
    pub fn set_sensitivity(&mut self, source: u8, target: u8, percent: f64) {
        let v = permille(percent.clamp(0.0, 100.0));
        self.brush
            .sensitivities
            .retain(|s| !(s.source == source && s.target == target));
        if v > 0 {
            self.brush.sensitivities.push(fp_model::attrs::Sensitivity {
                source,
                target,
                value: v,
            });
        }
    }

    /// Pinsel, wie er angewandt wird: genau der bearbeitete, die Pinselart bleibt (re 27 P1).
    pub fn effective(&self) -> Brush {
        self.brush.clone()
    }

    /// Befehl beim OK.
    pub fn invocation(&self) -> (String, Value) {
        let b = serde_json::to_value(self.effective()).unwrap_or(Value::Null);
        if self.tool {
            ("pi.tool.brush".into(), json!({"brush": b}))
        } else {
            ("pi.brush".into(), json!({"brush": b}))
        }
    }

    /// Hat sich etwas geändert?
    pub fn changed(&self) -> bool {
        self.brush != self.original
    }
}

/// Öffnet den Dialog (Auswahl bzw. Werkzeugvorgabe). Mit `tool = false` und ohne Objekt mit Strich
/// in der Auswahl gilt die Werkzeugvorgabe, wenn nichts ausgewählt ist; sonst kommt ein Hinweis.
pub fn open(st: &mut AppState, tool: bool) {
    if tool || crate::inspector::obj_quads::selection(st).is_empty() {
        st.dialogs.brush_edit = Some(BrushEditDialog::for_tool(st));
        return;
    }
    match BrushEditDialog::for_selection(st) {
        Some(d) => st.dialogs.brush_edit = Some(d),
        None => super::misc::not_available_message(
            st,
            &t("dlg-brushedit-title"),
            &t("dlg-stroke-need-selection"),
        ),
    }
}

/// Probestrich (S-Bogen) bzw. ein Abdruck mit dem Pinsel als Textur, zwischengespeichert je Pinsel.
fn preview_texture(
    ctx: &egui::Context,
    b: &Brush,
    color: fp_color::Argb,
    w: usize,
    h: usize,
    dot: bool,
) -> Option<egui::TextureHandle> {
    let key = egui::Id::new((
        "fp-brushedit-preview",
        dot,
        w,
        h,
        color.0,
        serde_json::to_string(b).unwrap_or_default(),
    ));
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(key)) {
        return Some(t);
    }
    let (wf, hf) = (w as f32, h as f32);
    let pts: Vec<Value> = if dot {
        vec![json!([wf / 2.0, hf / 2.0])]
    } else {
        (0..=40)
            .map(|i| {
                let x = 10.0 + (wf - 20.0) * i as f32 / 40.0;
                let y = hf / 2.0 + (hf / 4.0) * (i as f32 / 40.0 * std::f32::consts::TAU).sin();
                json!([x, y])
            })
            .collect()
    };
    let mut bb = b.clone();
    // Vorschau im Rahmen: sehr große Spitzen auf die Rahmenhöhe begrenzen
    bb.diameter = bb.diameter.clamp(
        1,
        if dot {
            (h as i32 - 2).max(1)
        } else {
            (h as i32 / 2).max(1)
        },
    );
    let p = json!({"points": pts, "brush": bb, "color": color.to_rgba8().to_hex_rgba(), "trimEnd": false});
    let job =
        fp_engine::paint::PaintJob::from_params(fp_engine::paint::PaintTool::Brush, &p, color, 1.0);
    let img = job.render(None, None, fp_geom::IRect::new(0, 0, w as i32, h as i32));
    let ci = egui::ColorImage::from_rgba_unmultiplied([img.width(), img.height()], img.as_bytes());
    let tex = ctx.load_texture("fp-brushedit-preview", ci, egui::TextureOptions::NEAREST);
    ctx.data_mut(|d| d.insert_temp(key, tex.clone()));
    Some(tex)
}

fn paint_preview(ui: &Ui, r: Rect, b: &Brush, color: fp_color::Argb, dot: bool) {
    ui.painter().rect_filled(r, 0.0, Color32::WHITE);
    ui.painter().rect_stroke(
        r,
        0.0,
        egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
        egui::StrokeKind::Inside,
    );
    let inner = r.shrink(1.0);
    if let Some(tex) = preview_texture(
        ui.ctx(),
        b,
        color,
        inner.width().max(1.0) as usize,
        inner.height().max(1.0) as usize,
        dot,
    ) {
        ui.painter().image(
            tex.id(),
            inner,
            Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }
}

/// Zahlenfeld mit Beschriftung rechtsbündig davor (Lage aus der Vorlage).
#[allow(clippy::too_many_arguments)]
fn num_row(
    ui: &mut Ui,
    o: egui::Pos2,
    id: &str,
    label: &str,
    lr: [f32; 4],
    er: [f32; 4],
    arrow: bool,
    value: f64,
    range: (f64, f64),
    enabled: bool,
) -> Option<f64> {
    num_row_unit(ui, o, id, label, lr, er, arrow, value, range, enabled, "")
}

/// Wie [`num_row`], Wert mit Einheit im Feld (die Prozentfelder des Originals zeigen „100%“, PAR 02/10).
#[allow(clippy::too_many_arguments)]
fn num_row_unit(
    ui: &mut Ui,
    o: egui::Pos2,
    id: &str,
    label: &str,
    lr: [f32; 4],
    er: [f32; 4],
    arrow: bool,
    value: f64,
    range: (f64, f64),
    enabled: bool,
    unit: &str,
) -> Option<f64> {
    fields::label_right(ui, dlu_rect(o, lr), label, enabled);
    let e = dlu_rect(o, er);
    let a = arrow.then(|| {
        Rect::from_min_size(
            egui::pos2(e.right(), e.top()),
            vec2(dlu(10.0, 0.0).x, e.height()),
        )
    });
    fields::number_unit(
        ui,
        ("fp-brushedit", id),
        e,
        a,
        Some(value),
        range,
        0,
        enabled,
        unit,
    )
}

/// Schieber + Zahlenfeld (Registerkarte Form).
fn slider_row(
    ui: &mut Ui,
    o: egui::Pos2,
    id: &str,
    label: &str,
    y: f32,
    value: f64,
    range: (f64, f64),
    edit_range: (f64, f64),
) -> Option<f64> {
    fields::label_right(ui, dlu_rect(o, [0.0, y + 3.0, 46.0, 10.0]), label, true);
    let sr = dlu_rect(o, [49.0, y, 100.0, 15.0]);
    let mut v = value.clamp(range.0, range.1);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(sr));
    child.spacing_mut().slider_width = sr.width() - 8.0;
    let r = crate::inspector::scale::slider(
        &mut child,
        &mut v,
        range,
        crate::inspector::scale::Scale::Linear { step: 1.0 },
    );
    let mut out = r.changed().then_some(v.round());
    if let Some(n) = fields::number(
        ui,
        ("fp-brushedit", id),
        dlu_rect(o, [151.0, y + 1.0, 30.0, 13.0]),
        None,
        Some(value),
        edit_range,
        0,
        true,
    ) {
        out = Some(n);
    }
    out
}

fn options_page(ui: &mut Ui, o: egui::Pos2, d: &mut BrushEditDialog) {
    let b = &mut d.brush;
    // wie im Original: Abstand und Farbfluss nur bei natürlichen Pinseln (einfache Pinsel: grau)
    let natural = b.brush_type == 0;
    if let Some(v) = num_row_unit(
        ui,
        o,
        "ink",
        &t("dlg-brushedit-ink"),
        [1.0, 7.0, 76.0, 11.0],
        [82.0, 5.0, 30.0, 13.0],
        true,
        pct(b.concentration),
        (0.0, 100.0),
        true,
        "%",
    ) {
        b.concentration = permille(v);
    }
    if let Some(v) = num_row_unit(
        ui,
        o,
        "spacing",
        &t("dlg-brushedit-spacing"),
        [2.0, 22.0, 75.0, 8.0],
        [82.0, 20.0, 30.0, 13.0],
        true,
        pct(b.spacing),
        (0.0, 1000.0),
        natural,
        "%",
    ) {
        b.spacing = permille(v);
    }
    if let Some(v) = num_row(
        ui,
        o,
        "flow",
        &t("dlg-brushedit-flow"),
        [2.0, 37.0, 75.0, 8.0],
        [82.0, 35.0, 30.0, 13.0],
        true,
        pct(b.flow_rate),
        (0.0, 100.0),
        natural,
    ) {
        b.flow_rate = permille(v);
    }
    if let Some(on) = fields::checkbox(
        ui,
        ("fp-brushedit", "buildup"),
        dlu_rect(o, [82.0, 52.0, 100.0, 10.0]),
        &t("dlg-brushedit-buildup"),
        b.feedback == 1,
        true,
    ) {
        b.feedback = i32::from(on);
    }
    if let Some(v) = num_row_unit(
        ui,
        o,
        "texture",
        &t("dlg-brushedit-texture"),
        [2.0, 69.0, 75.0, 8.0],
        [82.0, 67.0, 30.0, 13.0],
        true,
        pct(b.texture_blend),
        (0.0, 100.0),
        true,
        "%",
    ) {
        b.texture_blend = permille(v);
    }
    if let Some(v) = num_row_unit(
        ui,
        o,
        "edgetexture",
        &t("dlg-brushedit-edge-texture"),
        [2.0, 84.0, 75.0, 8.0],
        [82.0, 82.0, 30.0, 13.0],
        true,
        pct(b.texture_edge),
        (0.0, 100.0),
        true,
        "%",
    ) {
        b.texture_edge = permille(v);
    }
    fields::label_right(
        ui,
        dlu_rect(o, [2.0, 100.0, 75.0, 8.0]),
        &t("dlg-brushedit-edge-effect"),
        true,
    );
    let effects: Vec<String> = EFFECTS.iter().map(|e| t(e.0)).collect();
    let cur = EFFECTS
        .iter()
        .position(|e| e.1 == b.alpha_remap)
        .unwrap_or(0);
    if let Some(i) = fields::combo(
        ui,
        ("fp-brushedit", "effect"),
        dlu_rect(o, [82.0, 97.0, 105.0, 13.0]),
        &super::refs(&effects),
        Some(cur),
        true,
    ) {
        b.alpha_remap = EFFECTS[i].1;
    }
    if let Some(v) = num_row(
        ui,
        o,
        "tips",
        &t("dlg-brushedit-tips"),
        [20.0, 121.0, 57.0, 8.0],
        [82.0, 118.0, 30.0, 13.0],
        true,
        b.tip_count.max(1) as f64,
        (1.0, 32.0),
        true,
    ) {
        b.tip_count = v as i32;
    }
    let multi = b.tip_count > 1;
    if let Some(v) = num_row_unit(
        ui,
        o,
        "tipspacing",
        &t("dlg-brushedit-tip-spacing"),
        [16.0, 136.0, 61.0, 10.0],
        [82.0, 133.0, 30.0, 13.0],
        true,
        pct(b.tip_spacing),
        (0.0, 1000.0),
        multi,
        "%",
    ) {
        b.tip_spacing = permille(v);
    }
    fields::label_right(
        ui,
        dlu_rect(o, [1.0, 151.0, 76.0, 8.0]),
        &t("dlg-brushedit-variation"),
        multi,
    );
    let vars = super::tl(&VARIATION_KEYS);
    if let Some(i) = fields::combo(
        ui,
        ("fp-brushedit", "variation"),
        dlu_rect(o, [82.0, 148.0, 105.0, 13.0]),
        &super::refs(&vars),
        Some(b.tip_coloring_mode.clamp(0, 4) as usize),
        multi,
    ) {
        b.tip_coloring_mode = i as i32;
    }
    fields::label_right(
        ui,
        dlu_rect(o, [2.0, 167.0, 75.0, 8.0]),
        &t("dlg-brushedit-dashed"),
        true,
    );
    let dashes = super::tl(&DASH_KEYS);
    if let Some(i) = fields::combo(
        ui,
        ("fp-brushedit", "dashes"),
        dlu_rect(o, [82.0, 165.0, 105.0, 13.0]),
        &super::refs(&dashes),
        Some(b.dash_count.clamp(0, 3) as usize),
        true,
    ) {
        b.dash_count = i as i32;
        for k in 0..i {
            if b.dash_on[k] == 0 {
                b.dash_on[k] = 10;
            }
            if b.dash_off[k] == 0 {
                b.dash_off[k] = 5;
            }
        }
    }
    let (on_l, off_l) = (t("dlg-brushedit-dash-on"), t("dlg-brushedit-dash-off"));
    for k in 0..3usize {
        let y = 180.0 + 15.0 * k as f32;
        let en = (k as i32) < b.dash_count;
        if let Some(v) = num_row(
            ui,
            o,
            &format!("on{k}"),
            &on_l,
            [46.0, y + 2.0, 31.0, 8.0],
            [82.0, y, 35.0, 13.0],
            false,
            if en { b.dash_on[k] as f64 } else { 0.0 },
            (0.0, 1000.0),
            en,
        ) {
            b.dash_on[k] = v as i32;
        }
        if let Some(v) = num_row(
            ui,
            o,
            &format!("off{k}"),
            &off_l,
            [121.0, y + 2.0, 20.0, 8.0],
            [143.0, y, 35.0, 13.0],
            false,
            if en { b.dash_off[k] as f64 } else { 0.0 },
            (0.0, 1000.0),
            en,
        ) {
            b.dash_off[k] = v as i32;
        }
    }
}

fn shape_page(ui: &mut Ui, o: egui::Pos2, d: &mut BrushEditDialog, color: fp_color::Argb) {
    let tip = d.effective();
    paint_preview(ui, dlu_rect(o, [5.0, 5.0, 50.0, 50.0]), &tip, color, true);
    let b = &mut d.brush;
    if let Some(on) = fields::checkbox(
        ui,
        ("fp-brushedit", "square"),
        dlu_rect(o, [5.0, 57.0, 123.0, 10.0]),
        &t("dlg-brushedit-square"),
        b.shape == 0,
        true,
    ) {
        b.shape = i32::from(!on);
    }
    if let Some(v) = slider_row(
        ui,
        o,
        "size",
        &t("dlg-brushedit-size"),
        73.0,
        b.diameter as f64,
        (1.0, 100.0),
        (1.0, 1000.0),
    ) {
        b.diameter = (v as i32).clamp(1, 1000);
    }
    if let Some(v) = slider_row(
        ui,
        o,
        "edge",
        &t("dlg-brushedit-edge"),
        91.0,
        pct(b.softness),
        (0.0, 100.0),
        (0.0, 100.0),
    ) {
        b.softness = permille(v.clamp(0.0, 100.0));
    }
    if let Some(v) = slider_row(
        ui,
        o,
        "aspect",
        &t("dlg-brushedit-aspect"),
        109.0,
        pct(b.aspect).max(1.0),
        (1.0, 100.0),
        (1.0, 100.0),
    ) {
        b.aspect = permille(v.clamp(1.0, 100.0));
    }
    if let Some(v) = num_row(
        ui,
        o,
        "angle",
        &t("dlg-brushedit-angle"),
        [55.0, 131.0, 91.0, 10.0],
        [151.0, 129.0, 30.0, 13.0],
        true,
        if b.angle == 0 { 360.0 } else { b.angle as f64 },
        (0.0, 360.0),
        true,
    ) {
        b.angle = (v as i32).rem_euclid(360);
    }
}

fn sensitivity_page(ui: &mut Ui, o: egui::Pos2, d: &mut BrushEditDialog) {
    fields::label(
        ui,
        dlu_rect(o, [3.0, 3.0, 174.0, 9.0]),
        &t("dlg-brushedit-property"),
        true,
    );
    let props: Vec<String> = PROPERTIES.iter().map(|p| t(p.0)).collect();
    if let Some(i) = fields::combo(
        ui,
        ("fp-brushedit", "property"),
        dlu_rect(o, [3.0, 14.0, 188.0, 13.0]),
        &super::refs(&props),
        Some(d.property.min(PROPERTIES.len() - 1)),
        true,
    ) {
        d.property = i;
    }
    group_box(
        ui,
        dlu_rect(o, [3.0, 38.0, 187.0, 109.0]),
        &t("dlg-brushedit-depends"),
    );
    let target = PROPERTIES[d.property.min(PROPERTIES.len() - 1)].1;
    for (k, (src, key)) in SOURCES.iter().zip(SOURCE_KEYS).enumerate() {
        let y = [54.0, 72.0, 89.0, 106.0, 123.0][k];
        let v = d.sensitivity(*src, target);
        if let Some(n) = num_row(
            ui,
            o,
            &format!("sens{}{}", *src as char, target as char),
            &t(key),
            [8.0, y + 2.0, 131.0, 10.0],
            [143.0, y, 31.0, 13.0],
            true,
            v,
            (0.0, 100.0),
            true,
        ) {
            d.set_sensitivity(*src, target, n);
        }
    }
}

/// Zeigt den Dialog (falls offen).
pub fn show(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.brush_edit.clone() else {
        return;
    };
    let color = {
        let c = st.session.tools.stroke_color;
        if c.a() == 0 {
            fp_color::Argb(0xff00_0000)
        } else {
            fp_color::Argb(c.0 | 0xff00_0000)
        }
    };
    let mut result = None;
    let (ok_l, cancel_l) = (t("btn-ok"), t("btn-cancel"));
    let r = dialog_frame(
        ctx,
        "fp-brush-edit",
        &t("dlg-brushedit-title"),
        dlu(210.0, 318.0),
        |ui, body| {
            let o = body.min;
            // Registerkarten
            let tabs = super::tl(&TAB_KEYS);
            let tw: Vec<f32> = tabs
                .iter()
                .map(|s| super::text_w(ui.ctx(), s) + 16.0)
                .collect();
            let mut x = o.x + dlu(6.0, 0.0).x;
            for (i, label) in tabs.iter().enumerate() {
                let rr = Rect::from_min_size(
                    egui::pos2(x, o.y + dlu(0.0, 4.0).y),
                    vec2(tw[i], dlu(0.0, 13.0).y),
                );
                if ui
                    .put(rr, egui::Button::selectable(d.tab == i, label.as_str()))
                    .clicked()
                {
                    d.tab = i;
                }
                x += tw[i] + 2.0;
            }
            let page = dlu_rect(o, [4.0, 18.0, 202.0, 272.0]);
            ui.painter().rect_stroke(
                page,
                0.0,
                egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
                egui::StrokeKind::Inside,
            );
            let po = page.min + dlu(4.0, 4.0);
            match d.tab {
                0 => options_page(ui, po, &mut d),
                1 => shape_page(ui, po, &mut d, color),
                _ => sensitivity_page(ui, po, &mut d),
            }
            // Probestrich mit dem bearbeiteten Pinsel (alle Registerkarten)
            let pr = if d.tab == 0 {
                dlu_rect(po, [13.0, 229.0, 175.0, 32.0])
            } else {
                dlu_rect(po, [8.0, 170.0, 175.0, 32.0])
            };
            paint_preview(ui, pr, &d.effective(), color, false);
            let bw = super::buttons_w(ui.ctx(), &[&ok_l, &cancel_l], dlu(50.0, 0.0).x);
            let b = super::button_row(
                ui.ctx(),
                body.right() - dlu(4.0, 0.0).x,
                dlu_rect(o, [0.0, 297.0, 0.0, 0.0]).top(),
                &[&ok_l, &cancel_l],
                bw,
                dlu(0.0, 14.0).y,
                dlu(5.0, 0.0).x,
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
            st.dialogs.brush_edit = None;
            if d.changed() {
                let (cmd, p) = d.invocation();
                st.run(&cmd, p);
                // Werkzeugvorgabe: das Größenfeld im Inspektor zeigt die neue Größe (CT-2)
                if d.tool {
                    let w = d.effective().diameter.clamp(1, 1000);
                    crate::inspector::obj_quads::edit_tool_attrs(ctx, |a| a.stroke_width = w);
                }
            }
        }
        (Some(false), _) | (_, DialogClose::Cancelled) => st.dialogs.brush_edit = None,
        _ => st.dialogs.brush_edit = Some(d),
    }
}

#[cfg(test)]
mod m6_tests {
    /// M6 §4c (re\33): „Tintenmenge“ wirkt im Original auf die Deckkraft des Strichs (Ziel `O`).
    #[test]
    fn ink_amount_targets_opacity() {
        let ink = super::PROPERTIES
            .iter()
            .find(|(k, _)| *k == "dlg-brushedit-prop-opacity")
            .map(|p| p.1);
        assert_eq!(ink, Some(b'O'));
        assert!(super::TARGETS.contains(&b'O'));
    }
}

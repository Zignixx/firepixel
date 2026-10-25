//! Quadranten für Auswahl-Layouts (11 §2.3.2/§2.3.4): Pfad, Rechteck-Grundform, Gruppe,
//! zusammengesetzte Form, Bitmap, Text, Symbolinstanzen (Grafik, Schaltfläche, Animation),
//! Segment und Hotspot. Die Teilfelder (Füllung 14011, Strich 14010, Deckkraft 14012,
//! Fülltextur 14013, Strichtextur 14103, Live-Filter 14150/4224, Stil 14153, Pfadoperationen
//! 15174, Text 14141/14144) liegen in DLU genau wie in `dialoge.json`.
//!
//! Die Füll- und Strichfelder bedienen auch die Werkzeug-Layouts (Rechteck, Ellipse, Stift …):
//! dann gelten sie für neue Objekte ([`Target::Tool`]).

use super::QuadrantRegistry;
use super::fields::{self, Dl};
use super::glyphs::Glyph;
use super::popups::{self, ColorChoice};
use super::preview::{InspectorPreview, PreviewValue};
use super::texts::{self, refs, t, tl};
use crate::app_state::{Action, AppState};
use crate::theme::Theme;
use crate::ui_commands::{self, BRUSH_PRESETS, GRADIENT_SHAPES};
use egui::{Align2, Color32, Id, Rect, Stroke, StrokeKind, Ui, pos2, vec2};
use fp_color::{Argb, BlendMode};
use fp_model::{Element, ElementKind, PathAttrs};
use serde_json::{Value, json};
use std::sync::Arc;

pub(super) fn register(r: &mut QuadrantRegistry) {
    r.register(14129, path_q3);
    r.register(14130, path_q4);
    r.register(14131, rect_q4);
    r.register(14132, pathops_q5);
    for style in [15101, 15180, 15103, 15104, 15102, 15106] {
        r.register(style, style_q6);
    }
    r.register(14037, opacity_effect_q3);
    r.register(15008, effect_list_q4);
    r.register(15009, effect_list_q4);
    r.register(14045, animation_q3);
    r.register(14095, button_q3);
    r.register(14096, button_q4);
    r.register(14123, slice_q3);
    r.register(14043, slice_q4);
    r.register(14093, hotspot_q3);
    r.register(14124, hotspot_q4);
    r.register(15010, text_q3);
    r.register(14143, text_q4);
    r.register(14009, text_tool_q3);
    r.register(14128, mask_q3);
    r.register(15105, mask_options_q6);
    r.register(14140, path_q3);
    r.register(14098, path_q4);
    r.register(14147, text_q3);
    r.register(14142, text_q4);
}

// ---------------------------------------------------------------------- Auswahl

/// Ausgewählte Elemente (oben zuerst wie die Auswahl).
pub fn selection(st: &AppState) -> Vec<Arc<Element>> {
    // ein Durchlauf statt einer Suche je ID (PERF-1: Tausende gewählte Objekte)
    st.session
        .active()
        .map(|d| fp_engine::docops::elements_on_page(&d.doc, d.page, &d.selection.elements))
        .unwrap_or_default()
}

/// Selected elements in stacking order, topmost first (upper layer first).
pub fn selection_stacked(st: &AppState) -> Vec<Arc<Element>> {
    st.session
        .active()
        .map(|d| {
            fp_engine::docops::paths_sorted(&d.doc, d.page, &d.selection.elements)
                .iter()
                .filter_map(|p| d.doc.element_at(p).cloned())
                .collect()
        })
        .unwrap_or_default()
}

/// Element whose values the inspector shows: with several objects selected the **topmost** one
/// in stacking order (TW-120, re 45 §3).
fn first(st: &AppState) -> Option<Arc<Element>> {
    selection_stacked(st).into_iter().next()
}

fn first_attrs(st: &AppState) -> Option<Arc<PathAttrs>> {
    shown_attrs(st)
}

/// Fill/stroke attributes the inspector shows: with several objects selected those of the
/// **topmost** object in stacking order (TW-120, re 45 §3: R+Text shows the text's black fill,
/// E1+R1 shows E1's stroke whatever the selection order).
pub fn shown_attrs(st: &AppState) -> Option<Arc<PathAttrs>> {
    selection_stacked(st)
        .iter()
        .find_map(|e| fp_engine::docops::path_attrs_of(e))
}

fn placement_of(e: &Element) -> i32 {
    match &e.kind {
        ElementKind::Path(p) => p.stroke_placement,
        ElementKind::Text(t) => t.stroke_placement,
        ElementKind::Group(g) => g.children.first().map(|c| placement_of(c)).unwrap_or(1),
        _ => 1,
    }
}

/// Ziel der Füll-/Strichfelder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Selection,
    Tool,
}

// ---------------------------------------------------------------------- Werkzeugattribute

/// Inspektor-Einstellungen der Werkzeuge, die fp-engine (noch) nicht führt (Füllart, Kante, Textur,
/// Strichvorgabe, Mischmodus …). Farben stehen in der Sitzung (`tools.fill_color/stroke_color`).
#[derive(Clone, Debug, PartialEq)]
pub struct ToolAttrs {
    pub fill_kind: u8,
    pub grad_shape: i32,
    pub edge: i32,
    pub feather: i32,
    pub fill_texture: String,
    pub fill_texture_amount: i32,
    pub fill_texture_transparent: bool,
    /// Muster der Werkzeugvorgabe (Dateiname ohne Endung oder voller Pfad; leer = Vorgabemuster).
    pub pattern: String,
    pub stroke_preset: Option<usize>,
    pub stroke_width: i32,
    pub softness: i32,
    pub placement: i32,
    pub stroke_texture: String,
    pub stroke_texture_amount: i32,
    pub blend: usize,
    pub keep_transparency: bool,
    pub live_marquee: bool,
    pub auto_erase: bool,
    pub retouch_size: i32,
    pub retouch_edge: i32,
    pub retouch_round: bool,
    pub intensity: i32,
    pub range: usize,
    pub exposure: i32,
    pub smudge_color: bool,
    pub whole_doc: bool,
    /// "Confine to boundaries" of the active tool (Firepixel option, `<Tool>ConfineToBounds`;
    /// Smudge FB-15; Blur, Rubber Stamp, Brush and Pencil FB-52).
    pub confine_bounds: bool,
    pub source_aligned: bool,
    pub tolerance: i32,
    pub strength: i32,
    pub colorize: bool,
    pub freeform_size: i32,
    pub pressure: bool,
    pub preview: bool,
    pub velocity: bool,
    pub rate: i32,
    pub color_format: usize,
    pub hotspot_shape: usize,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub align: i32,
}

impl Default for ToolAttrs {
    fn default() -> Self {
        ToolAttrs {
            fill_kind: 1,
            grad_shape: 2,
            edge: 1,
            feather: 0,
            fill_texture: String::new(),
            fill_texture_amount: 0,
            fill_texture_transparent: false,
            pattern: String::new(),
            // Werksvorgabe „Präzise abger.“ wie im Original (FIX-G5; Index in der Vorlagentabelle)
            stroke_preset: fp_engine::commands::panel::preset_index_of(
                fp_engine::commands::panel::DEFAULT_TOOL_PRESET.0,
                fp_engine::commands::panel::DEFAULT_TOOL_PRESET.1,
            ),
            stroke_width: 1,
            softness: 0,
            placement: 1,
            stroke_texture: String::new(),
            stroke_texture_amount: 0,
            blend: 0,
            keep_transparency: false,
            live_marquee: true,
            auto_erase: false,
            retouch_size: 10,
            retouch_edge: 0,
            retouch_round: true,
            intensity: 50,
            range: 1,
            exposure: 50,
            smudge_color: false,
            whole_doc: false,
            confine_bounds: false,
            source_aligned: true,
            tolerance: 70,
            strength: 50,
            colorize: false,
            freeform_size: 10,
            pressure: false,
            preview: true,
            velocity: false,
            rate: 1,
            color_format: 0,
            hotspot_shape: 0,
            bold: false,
            italic: false,
            underline: false,
            align: 0,
        }
    }
}

const TOOL_ATTRS: &str = "pi-tool-attrs";

/// Werkzeugattribute lesen.
pub fn tool_attrs(ctx: &egui::Context) -> ToolAttrs {
    ctx.data(|d| d.get_temp(Id::new(TOOL_ATTRS)))
        .unwrap_or_default()
}

/// Werkzeugattribute ändern.
pub fn edit_tool_attrs(ctx: &egui::Context, f: impl FnOnce(&mut ToolAttrs)) {
    let mut a = tool_attrs(ctx);
    f(&mut a);
    ctx.data_mut(|d| d.insert_temp(Id::new(TOOL_ATTRS), a));
}

// ---------------------------------------------------------------------- Listen

/// Mischmodus-Liste (55 Einträge, `RT_DLGINIT` 14526) und `BLDi` je Eintrag (A: Zuordnung über die Namen).
pub const BLEND_ITEMS: &[(&str, i32)] = &[
    ("Normal", 0),
    ("Durchschnitt", 30),
    ("Auflösen", 7),
    ("-----", -1),
    ("Abdunkeln", 9),
    ("Multiplizieren", 2),
    ("Farbig nachbelichten", 37),
    ("Umgek. farb. nachb.", 39),
    ("Weich nachbelichten", 41),
    ("Linear nachbelichten", 8),
    ("-----", -1),
    ("Aufhellen", 10),
    ("Negativ multiplizieren", 5),
    ("Farbig abwedeln", 36),
    ("Umgek. farb. abw.", 38),
    ("Weich abwedeln", 40),
    ("Linear abwedeln", 18),
    ("-----", -1),
    ("Überlagern", 4),
    ("Weiches Licht", 34),
    ("Unscharfes Licht", 35),
    ("Hartes Licht", 33),
    ("Strahlendes Licht", 19),
    ("Lineares Licht", 20),
    ("Lichtpunkt", 21),
    ("Hart mischen", 22),
    ("-----", -1),
    ("Differenz", 3),
    ("Ausschluss", 32),
    ("Negation", 31),
    ("-----", -1),
    ("Farbton", 11),
    ("Sättigung", 12),
    ("Farbe", 13),
    ("Luminanz", 14),
    ("Rot", 51),
    ("Grün", 52),
    ("Blau", 53),
    ("-----", -1),
    ("Spiegeln", 42),
    ("Glühen", 43),
    ("Abkühlen", 44),
    ("Erwärmen", 45),
    ("-----", -1),
    ("Additiv", 46),
    ("Subtraktiv", 47),
    ("Subtrahieren", 54),
    ("Interpolation", 48),
    ("Stempel", 49),
    ("XOR", 50),
    ("-----", -1),
    ("Umkehren", 17),
    ("Farbton", 29),
    ("-----", -1),
    ("Löschen", 25),
];

/// Namen der Mischmodus-Liste für Objekte in der aktuellen Sprache (wie [`texts::blend_labels_obj`], als
/// `&'static str` für ältere Aufrufer; je Sprachstand einmal abgelegt).
pub fn blend_names() -> Vec<&'static str> {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<HashMap<String, &'static [&'static str]>>> = OnceLock::new();
    // Schlüssel: Sprache und Stand der Sprachdateien (Nachladen eines Sprachordners)
    let lang = format!("{}#{}", fp_i18n::current_language(), fp_i18n::generation());
    let mut m = CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let v = m.entry(lang).or_insert_with(|| {
        let names: Vec<&'static str> = texts::blend_labels_obj()
            .into_iter()
            .map(|s| &*Box::leak(s.into_boxed_str()))
            .collect();
        Box::leak(names.into_boxed_slice())
    });
    v.to_vec()
}

/// Listenindex eines Mischmodus.
pub fn blend_index(m: BlendMode) -> Option<usize> {
    BLEND_ITEMS.iter().position(|b| b.1 == m.0 as i32)
}

/// Fill edge kinds; the fourth, "Dither", is a Firepixel extension (FB-23).
pub const EDGE_ITEMS: [&str; 4] = [
    "pi-edge-hard",
    "pi-edge-antialias",
    "pi-edge-feather-short",
    "pi-edge-dither",
];
const TARGETS: [&str; 4] = ["_blank", "_parent", "_self", "_top"];

fn targets_list() -> Vec<String> {
    TARGETS.iter().map(|s| s.to_string()).collect()
}

/// Bekannte URLs des Dokuments (Hyperlink-Kombifelder).
fn known_urls(st: &AppState) -> Vec<String> {
    let mut v = Vec::new();
    if let Some(d) = st.session.active()
        && let Some(pg) = d.doc.pages.get(d.page)
    {
        fp_model::traverse::walk_root(&pg.root, fp_model::RootRef::Page(d.page), &mut |_, _, e| {
            if let ElementKind::WebObject(w) = &e.kind
                && let Some(u) = &w.url
                && !u.is_empty()
                && !v.contains(u)
            {
                v.push(u.clone());
            }
        });
    }
    v
}

fn c32(c: Argb) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), c.a())
}

// ---------------------------------------------------------------------- Teilfeld Füllung (14011)

fn draw_gradient_swatch(ui: &Ui, r: Rect, f: &fp_model::Fill) {
    let t = Theme::current(ui.ctx());
    let Some(g) = &f.gradient else { return };
    let cs: Vec<(f32, Argb)> = g
        .color_stops()
        .iter()
        .map(|s| (s.position, s.color))
        .collect();
    let inner = r.shrink(2.0);
    crate::widgets::checker(ui.painter(), inner, 3.0, &t);
    let n = 12;
    for i in 0..n {
        let p = (i as f32 + 0.5) / n as f32;
        let col = popups::sample_stops(&cs, p);
        let rr = Rect::from_min_max(
            pos2(
                inner.left() + i as f32 / n as f32 * inner.width(),
                inner.top(),
            ),
            pos2(
                inner.left() + (i + 1) as f32 / n as f32 * inner.width() + 0.5,
                inner.bottom(),
            ),
        );
        ui.painter()
            .rect_filled(rr, 0.0, c32(Argb(col.0 | 0xff00_0000)));
    }
}

/// Schlüssel: Farben des zuletzt im Werkzeug-Füllfarbfeld gezeichneten Verlaufs (Tests).
const TOOL_SWATCH_GRADIENT: &str = "pi-tool-swatch-gradient";

/// Farben (ARGB) des Verlaufs, den das Füllfarbfeld des Werkzeug-Inspektors zuletzt zeigte;
/// `None`, wenn es einfarbig war (nur das Verlauf-Werkzeug zeigt einen Verlauf). Für Tests.
pub fn tool_swatch_gradient(ctx: &egui::Context) -> Option<Vec<u32>> {
    ctx.data(|d| d.get_temp::<Option<Vec<u32>>>(Id::new(TOOL_SWATCH_GRADIENT)))
        .flatten()
}

/// Antwort des Verlaufseditors für das Verlauf-Werkzeug: Farbstopps → eigene Werkzeugfarben
/// (`ToolOptions::gradient_colors`), „Umkehren“, Verlaufsform als Werkzeugattribut.
fn apply_tool_gradient(ui: &Ui, st: &mut AppState, p: &Value) {
    let o = &mut st.session.tools.options;
    if let Some(stops) = p.get("stops").and_then(|v| v.as_array()) {
        let mut v: Vec<(f64, Argb)> = stops
            .iter()
            .filter_map(|s| {
                let pos = s.get(0)?.as_f64()?;
                let c = fp_color::Rgba8::parse_hex(s.get(1)?.as_str()?)?.to_argb();
                Some((pos, Argb(c.0 | 0xff00_0000)))
            })
            .collect();
        v.sort_by(|a, b| a.0.total_cmp(&b.0));
        if v.len() >= 2 {
            o.gradient_colors = v.into_iter().map(|(_, c)| c).collect();
        }
    }
    if p.get("reverse").and_then(|v| v.as_bool()) == Some(true) {
        let mut c = o.gradient_tool_colors();
        c.reverse();
        o.gradient_colors = c;
    }
    if let Some(sh) = p.get("shape").and_then(|v| v.as_i64()) {
        edit_tool_attrs(ui.ctx(), |a| a.grad_shape = sh as i32);
    }
}

/// Merker: Füllung der Auswahl wurde im Inspektor geändert; im nächsten Bild übernimmt das
/// Werkzeug sie (neue Objekte erhalten die zuletzt gesetzte Füllung, PAR 18/22).
const FILL_MIRROR: &str = "pi-fill-mirror";

fn apply_fill(ui: &Ui, st: &mut AppState, tgt: Target, params: Value) {
    match tgt {
        Target::Selection => {
            st.run("pi.fill", params);
            ui.ctx()
                .data_mut(|d| d.insert_temp(Id::new(FILL_MIRROR), true));
        }
        Target::Tool => {
            let ctx = ui.ctx().clone();
            if let Some(c) = params
                .get("color")
                .and_then(|v| v.as_str())
                .and_then(fp_color::Rgba8::parse_hex)
            {
                st.push(Action::SetColors {
                    stroke: None,
                    fill: Some(c.to_argb()),
                });
            }
            edit_tool_attrs(&ctx, |a| {
                match params.get("category").and_then(|v| v.as_str()) {
                    Some("none") => a.fill_kind = 0,
                    Some("solid") => a.fill_kind = 1,
                    Some("gradient") => a.fill_kind = 2,
                    Some("pattern") => a.fill_kind = 3,
                    _ => {}
                }
                if let Some(v) = params.get("pattern").and_then(|v| v.as_str()) {
                    a.fill_kind = 3;
                    a.pattern = v.to_string();
                }
                if let Some(v) = params.get("shape").and_then(|v| v.as_i64()) {
                    a.grad_shape = v as i32;
                }
                if let Some(v) = params.get("edge").and_then(|v| v.as_i64()) {
                    a.edge = v as i32;
                    if !fp_model::Fill::edge_uses_amount(a.edge) {
                        a.feather = 0;
                    }
                }
                if let Some(v) = params.get("feather").and_then(|v| v.as_i64()) {
                    a.feather = v as i32;
                }
                if let Some(v) = params.get("texture").and_then(|v| v.as_str()) {
                    a.fill_texture = v.to_string();
                }
                if let Some(v) = params.get("textureAmount").and_then(|v| v.as_i64()) {
                    a.fill_texture_amount = v as i32;
                }
                if let Some(v) = params.get("textureTransparent").and_then(|v| v.as_bool()) {
                    a.fill_texture_transparent = v;
                }
            });
        }
    }
}

/// Übernimmt die Füllung der Auswahl als Werkzeugvorgabe (nach einer Änderung im Inspektor; das
/// Original zeichnet neue Objekte mit der zuletzt gesetzten Füllung, PAR 18/22).
fn mirror_fill_to_tool(ctx: &egui::Context, st: &mut AppState) {
    if !ctx
        .data(|d| d.get_temp::<bool>(Id::new(FILL_MIRROR)))
        .unwrap_or(false)
    {
        return;
    }
    ctx.data_mut(|d| d.remove::<bool>(Id::new(FILL_MIRROR)));
    let Some(a) = shown_attrs(st) else { return };
    let f = a.fill.as_ref();
    let kind = ui_commands::fill_kind(f);
    edit_tool_attrs(ctx, |ta| {
        ta.fill_kind = if kind == 4 { 1 } else { kind };
        if let Some(f) = f {
            if kind == 2 {
                ta.grad_shape = ui_commands::shape_id(f);
            }
            if kind == 3 {
                ta.pattern = f
                    .pattern
                    .as_ref()
                    .map(|p| p.name.clone())
                    .unwrap_or_default();
            }
            ta.edge = f.edge;
            ta.feather = f.feather;
            ta.fill_texture_amount = f.texture_amount / 10;
            ta.fill_texture_transparent = f.transparent_dither.unwrap_or(false);
        }
        ta.fill_texture = a
            .fill_texture
            .as_ref()
            .map(|t| t.name.clone())
            .filter(|_| ta.fill_texture_amount > 0)
            .unwrap_or_default();
    });
    if kind != 0 {
        st.push(Action::SetColors {
            stroke: None,
            fill: Some(Argb(a.fill_color.0 | 0xff00_0000)),
        });
    }
}

/// Angle of the fill handles of the shown (topmost) object (gradient/pattern popup).
fn selection_fill_angle(st: &AppState) -> Option<f64> {
    selection_stacked(st)
        .iter()
        .find_map(|e| ui_commands::handles_of(e))
        .filter(|h| h.valid)
        .map(|h| ui_commands::handles_angle(&h))
}

/// Verlaufsfüllung der Werkzeugvorgabe (Weiß → Füllfarbe, Form `grad_shape`).
fn tool_gradient_fill(ta: &ToolAttrs, c: Argb) -> fp_model::Fill {
    let mut f = fp_model::Fill::solid();
    let end = if Argb(c.0 | 0xff00_0000) == Argb(0xffff_ffff) {
        Argb(0xff00_0000)
    } else {
        Argb(c.0 | 0xff00_0000)
    };
    f.shape = ta.grad_shape;
    if ta.grad_shape == ui_commands::CONTOUR_SHAPE {
        f.shape = 3;
        f.shape_ext = 17;
    }
    f.gradient = Some(ui_commands::make_gradient(
        ta.grad_shape,
        &[(0.0, Argb(0xffff_ffff)), (1.0, end)],
        None,
    ));
    f
}

/// Popup-Ids des Verlaufs- und des Muster-Popups am Füllfarbfeld der **Werkzeugleiste**.
pub fn toolbar_fill_popup_ids() -> (Id, Id) {
    let id = Id::new("tb-fill");
    (id.with("grad-pop"), id.with("pat-pop"))
}

/// Verlaufs- bzw. Muster-Popup am Füllfarbfeld der Werkzeugleiste (B-2b BA-128): „Verlauf“ und
/// „Muster“ im Farbwähler der Werkzeugleiste öffnen diese Popups, statt sofort umzustellen. Erst eine
/// Wahl darin ändert die Auswahl (gewählte Vektoren) bzw. die Werkzeugvorgabe; Abbrechen lässt alles.
pub fn toolbar_fill_popups(ui: &mut Ui, st: &mut AppState, well: Rect) {
    let ctx = ui.ctx().clone();
    let (gp, pp) = toolbar_fill_popup_ids();
    if !popups::anchored_open(&ctx, gp) && !popups::anchored_open(&ctx, pp) {
        return;
    }
    let vectors = selection(st)
        .iter()
        .any(|e| !matches!(e.kind, ElementKind::Bitmap(_)));
    let tgt = if vectors {
        Target::Selection
    } else {
        Target::Tool
    };
    let ta = tool_attrs(&ctx);
    let sel_fill = if vectors {
        first_attrs(st).and_then(|a| a.fill.clone())
    } else {
        None
    };
    let kind = ui_commands::fill_kind(sel_fill.as_ref());
    let id = Id::new("tb-fill");
    // Verlauf
    let grad = match sel_fill.as_ref().filter(|_| kind == 2) {
        Some(f) => f.clone(),
        None => tool_gradient_fill(&ta, st.session.tools.fill_color),
    };
    let angle = if vectors && kind == 2 {
        selection_fill_angle(st)
    } else {
        None
    };
    if let Some(mut p) = popups::anchored_popup(ui, gp, well, false, |ui| {
        popups::gradient_editor(ui, id.with("grad"), &grad, st, angle)
    })
    .flatten()
    {
        if tgt == Target::Tool || kind != 2 {
            p["category"] = json!("gradient");
        }
        apply_fill(ui, st, tgt, p);
    }
    // Muster
    let cur = match sel_fill
        .as_ref()
        .filter(|_| kind == 3)
        .and_then(|f| f.pattern.as_ref())
    {
        Some(p) => p.name.clone(),
        None if ta.pattern.is_empty() => popups::default_pattern(),
        None => texture_display_name(&ta.pattern),
    };
    let pangle = if vectors && kind == 3 {
        selection_fill_angle(st)
    } else {
        None
    };
    match popups::anchored_popup(ui, pp, well, false, |ui| {
        popups::pattern_editor(ui, id.with("pat"), &cur, pangle)
    })
    .flatten()
    {
        Some(popups::PatternChoice::Named(n)) => apply_fill(ui, st, tgt, json!({"pattern": n})),
        Some(popups::PatternChoice::Angle(a)) => apply_fill(ui, st, tgt, json!({"angle": a})),
        Some(popups::PatternChoice::Other) => {
            let (title, images) = (t("pi-pattern-pick-title"), t("pi-filter-images"));
            crate::dialogs::panel_menus::pick_open(
                st,
                &title,
                &[(images.as_str(), &["png", "jpg", "jpeg", "gif", "bmp"])],
                move |st, p| match tgt {
                    Target::Selection => st.run("pi.fill", json!({"pattern": p.to_string_lossy()})),
                    Target::Tool => edit_tool_attrs(&ctx, |a| {
                        a.fill_kind = 3;
                        a.pattern = p.to_string_lossy().to_string();
                    }),
                },
            );
        }
        None => {}
    }
}

/// Popup-Ids des Verlaufs- und des Muster-Popups eines Füllfelds.
pub fn fill_popup_ids(tool: bool) -> (Id, Id) {
    let id = Id::new(("pi-fill", tool));
    (id.with("grad-pop"), id.with("pat-pop"))
}

/// Colour a selection well shows: grey (luma) while a mask is selected (RE spec 68 §2.1).
fn mask_well(st: &AppState, c: Argb) -> Argb {
    if st.session.mask_selected() {
        fp_engine::tool_follow::mask_gray(c)
    } else {
        c
    }
}

/// Teilfeld „Füllung“ an DLU-Ursprung `d` (Breite 119 DLU, Höhe 29 DLU).
pub fn fill_panel(ui: &mut Ui, st: &mut AppState, d: Dl, tgt: Target, enabled: bool) {
    let theme = Theme::current(ui.ctx());
    if tgt == Target::Selection {
        mirror_fill_to_tool(&ui.ctx().clone(), st);
    }
    let ta = tool_attrs(ui.ctx());
    let attrs = first_attrs(st);
    let (kind, color, fill, edge, feather) = match tgt {
        Target::Selection => {
            let a = attrs.as_deref();
            let fill = a.and_then(|a| a.fill.clone());
            let kind = ui_commands::fill_kind(fill.as_ref());
            let col = a
                .map(|a| a.fill_color)
                .filter(|_| kind != 0)
                .map(|c| mask_well(st, c));
            (
                kind,
                col,
                fill.clone(),
                fill.as_ref().map(|f| f.edge).unwrap_or(1),
                fill.as_ref().map(|f| f.feather).unwrap_or(0),
            )
        }
        Target::Tool => {
            let c = st.session.tools.fill_color;
            let kind = if c.a() == 0 { 0 } else { ta.fill_kind };
            let fill = match kind {
                2 => Some(tool_gradient_fill(&ta, c)),
                3 => {
                    let mut f = fp_model::Fill::solid();
                    f.shape = 15;
                    f.category = "fc_Pattern".into();
                    let name = if ta.pattern.is_empty() {
                        popups::default_pattern()
                    } else {
                        texture_display_name(&ta.pattern)
                    };
                    f.pattern = Some(fp_model::attrs::Pattern {
                        name: name.clone(),
                        user_name: Some(name),
                        image: None,
                        extra: Default::default(),
                    });
                    Some(f)
                }
                _ => None,
            };
            (kind, (c.a() > 0).then_some(c), fill, ta.edge, ta.feather)
        }
    };
    let en = enabled && (tgt == Target::Tool || attrs.is_some());
    let id = Id::new(("pi-fill", tgt == Target::Tool));
    let (grad_pid, pat_pid) = fill_popup_ids(tgt == Target::Tool);
    super::glyphs::paint(
        ui.painter(),
        d.r(0.0, 0.0, 12.0, 11.0),
        Glyph::PaintBucket,
        &theme,
        en,
    );
    let sw = d.r(14.0, 0.0, 15.0, 13.0);
    let angle = match tgt {
        Target::Selection => selection_fill_angle(st),
        Target::Tool => None,
    };
    // Verlauf-Werkzeug: eigenes Farbfeld mit dem Verlauf, den das Werkzeug malt (Werksvorgabe
    // Schwarz → Weiß, unabhängig von Füll- und Strichfarbe; Original gemessen, PARITAET_A W26),
    // Klick öffnet das Verlaufs-Popup; ohne Füllart-Knöpfe (W25)
    let gradient_tool =
        tgt == Target::Tool && st.session.tools.active == fp_engine::ToolId::Gradient;
    if gradient_tool {
        let cols = st.session.tools.options.gradient_tool_colors();
        let n = (cols.len() - 1).max(1) as f32;
        let stops: Vec<(f32, Argb)> = cols
            .iter()
            .enumerate()
            .map(|(i, c)| (i as f32 / n, Argb(c.0 | 0xff00_0000)))
            .collect();
        let mut f = fp_model::Fill::solid();
        f.shape = ta.grad_shape.max(2);
        f.gradient = Some(ui_commands::make_gradient(f.shape, &stops, None));
        let resp = fields::swatch(ui, id.with("sw"), sw, Some(stops[0].1), en);
        draw_gradient_swatch(ui, sw, &f);
        ui.ctx().data_mut(|dd| {
            dd.insert_temp(
                Id::new(TOOL_SWATCH_GRADIENT),
                Some(stops.iter().map(|s| s.1.0).collect::<Vec<u32>>()),
            )
        });
        let out = popups::anchored_popup(ui, grad_pid, sw, resp.clicked(), |ui| {
            popups::gradient_editor(ui, id.with("grad"), &f, st, None)
        })
        .flatten();
        if let Some(p) = out {
            apply_tool_gradient(ui, st, &p);
        }
    } else if kind == 2 {
        let resp = fields::swatch(ui, id.with("sw"), sw, color, en);
        if let Some(f) = &fill {
            draw_gradient_swatch(ui, sw, f);
        }
        let mut out = None;
        if let Some(f) = &fill {
            out = popups::anchored_popup(ui, grad_pid, sw, resp.clicked(), |ui| {
                popups::gradient_editor(ui, id.with("grad"), f, st, angle)
            })
            .flatten();
        }
        if let Some(p) = out {
            apply_fill(ui, st, tgt, p);
        }
    } else if kind == 3 {
        // Muster: Farbfeld zeigt das Muster, Klick öffnet das Muster-Popup (PAR 18/08)
        let cur = fill
            .as_ref()
            .and_then(|f| f.pattern.as_ref())
            .map(|p| p.name.clone())
            .unwrap_or_default();
        let resp = fields::swatch(ui, id.with("sw"), sw, color, en);
        draw_pattern_swatch(ui, sw, &cur);
        let out = popups::anchored_popup(ui, pat_pid, sw, resp.clicked(), |ui| {
            popups::pattern_editor(
                ui,
                id.with("pat"),
                &cur,
                if tgt == Target::Selection {
                    angle
                } else {
                    None
                },
            )
        })
        .flatten();
        match out {
            Some(popups::PatternChoice::Named(n)) => apply_fill(ui, st, tgt, json!({"pattern": n})),
            Some(popups::PatternChoice::Angle(a)) => apply_fill(ui, st, tgt, json!({"angle": a})),
            Some(popups::PatternChoice::Other) => {
                let (title, images) = (t("pi-pattern-pick-title"), t("pi-filter-images"));
                let ctx = ui.ctx().clone();
                crate::dialogs::panel_menus::pick_open(
                    st,
                    &title,
                    &[(images.as_str(), &["png", "jpg", "jpeg", "gif", "bmp"])],
                    move |st, p| match tgt {
                        Target::Selection => {
                            st.run("pi.fill", json!({"pattern": p.to_string_lossy()}))
                        }
                        Target::Tool => edit_tool_attrs(&ctx, |a| {
                            a.fill_kind = 3;
                            a.pattern = p.to_string_lossy().to_string();
                        }),
                    },
                );
            }
            None => {}
        }
    } else {
        let choice = popups::color_button_with(
            ui,
            id.with("sw"),
            sw,
            st,
            color,
            en,
            super::color_picker::Footer::Plain,
        );
        if tgt == Target::Tool {
            ui.ctx()
                .data_mut(|dd| dd.insert_temp(Id::new(TOOL_SWATCH_GRADIENT), None::<Vec<u32>>));
        }
        match choice {
            Some(ColorChoice::Color(c)) => {
                let mut p = json!({"color": ui_commands::hex(c)});
                if kind == 0 {
                    p["category"] = json!("solid");
                }
                apply_fill(ui, st, tgt, p);
            }
            Some(ColorChoice::None) => match tgt {
                Target::Selection => apply_fill(ui, st, tgt, json!({"category": "none"})),
                Target::Tool => st.push(Action::SetColors {
                    stroke: None,
                    fill: Some(Argb(0x00ff_ffff)),
                }),
            },
            Some(_) | None => {}
        }
        // live canvas preview of the pending alpha of the colour pop-up (FB-29, TW-206)
        if tgt == Target::Selection
            && en
            && let Some(c) = popups::color_button_pending(ui.ctx(), id.with("sw"))
        {
            st.inspector_preview = InspectorPreview::for_selection(st, PreviewValue::FillColor(c));
        }
    }
    // Füllart-Knöpfe Keine / Einfarbig / Verlauf / Muster; „Verlauf“ und „Muster“ stellen um und
    // öffnen sofort das zugehörige Popup (Original, PAR 18/01). Der Werkzeug-Inspektor des Verlaufs
    // zeigt nur das Farbfeld (Original, PARITAET_A W25/W26; das Füllwerkzeug hat die Knöpfe)
    let kinds = [
        (54.0, Glyph::FillNone, 0u8, "pi-fill-none", "none"),
        (70.0, Glyph::FillSolid, 1, "pi-fill-solid", "solid"),
        (86.0, Glyph::FillGradient, 2, "pi-fill-gradient", "gradient"),
        (102.0, Glyph::FillPattern, 3, "pi-fill-pattern", "pattern"),
    ];
    for (x, g, k, tip, cat) in kinds.into_iter().filter(|_| !gradient_tool) {
        let tip = t(tip);
        if !fields::toggle(
            ui,
            id.with(("kind", k)),
            d.r(x, 0.0, 16.0, 12.0),
            g,
            kind == k,
            en,
            &tip,
        ) {
            continue;
        }
        if tgt == Target::Tool && k == 0 {
            st.push(Action::SetColors {
                stroke: None,
                fill: Some(Argb(0x00ff_ffff)),
            });
            continue;
        }
        if tgt == Target::Tool && kind == 0 {
            let c = st.session.prefs.default_colors().1;
            st.push(Action::SetColors {
                stroke: None,
                fill: Some(Argb(c.0 | 0xff00_0000)),
            });
        }
        if kind != k {
            apply_fill(ui, st, tgt, json!({"category": cat}));
        }
        match k {
            2 => popups::set_anchored_open(ui.ctx(), grad_pid, true),
            3 => popups::set_anchored_open(ui.ctx(), pat_pid, true),
            _ => {}
        }
    }
    // Rand: Hart / Anti-Alias / Weiche Kante + Breite (Verlauf-Werkzeug: immer bedienbar, es malt
    // stets einen Verlauf); "Dither" (Firepixel extension, FB-23) uses the same width field
    let fen = en && (kind != 0 || gradient_tool);
    let amount_edge = fp_model::Fill::edge_uses_amount(edge);
    fields::label(ui, d.r(0.0, 17.0, 31.0, 8.0), &t("pi-edge"), fen);
    let edge_items = tl(&EDGE_ITEMS);
    let er = d.r(33.0, 15.0, 55.0, 13.0);
    if let Some(i) = fields::combo(
        ui,
        id.with("edge"),
        er,
        &refs(&edge_items),
        Some(edge.clamp(0, 3) as usize),
        fen,
    ) {
        let mut p = json!({"edge": i});
        if fp_model::Fill::edge_uses_amount(i as i32) && feather == 0 {
            p["feather"] = json!(10);
        }
        apply_fill(ui, st, tgt, p);
    }
    tooltip_at(ui, id.with("edge-tip"), er, &t("pi-edge-tip"));
    if let Some(v) = fields::number(
        ui,
        id.with("feather"),
        d.r(89.0, 15.0, 19.0, 13.0),
        Some(d.r(109.0, 15.0, 9.0, 13.0)),
        Some(feather as f64),
        (0.0, 100.0),
        0,
        fen && amount_edge,
    ) {
        apply_fill(ui, st, tgt, json!({"feather": v as i64, "edge": edge}));
    }
    // live canvas preview of the dragged feather / dither amount (FB-21, FB-23, Firepixel
    // enhancement); the preview keeps the edge kind
    preview_slider(
        ui,
        st,
        id.with("feather"),
        tgt == Target::Selection && fen && amount_edge,
        PreviewValue::Feather,
    );
}

/// Live canvas preview of the pending value of the pop-up slider of the number field `field`
/// while its thumb is moved (FB-09/FB-10, FB-21/FB-22; Firepixel enhancement, the reference only
/// updates the field). Applies to the selection; the value itself is applied on release / close.
fn preview_slider(
    ui: &Ui,
    st: &mut AppState,
    field: Id,
    on: bool,
    value: impl FnOnce(f64) -> PreviewValue,
) {
    if on && let Some(v) = fields::slider_live(ui.ctx(), field) {
        st.inspector_preview = InspectorPreview::for_selection(st, value(v));
    }
}

/// Live canvas preview of the family highlighted in the open font list. The document and its
/// history stay untouched until Enter or a click commits `text.applyAttributes`. Highlighting
/// the current family, closing the list, or Esc drops the preview. With the on-canvas text
/// editor open the preview is skipped, same as the text sliders.
pub(crate) fn preview_marked_font(ctx: &egui::Context, st: &mut AppState, family: &str, style: &str) {
    if st.session.text_edit_attrs().is_some() {
        return;
    }
    let Some(marked) = super::fonts::marked_family(ctx) else {
        return;
    };
    if marked.eq_ignore_ascii_case(family) {
        return;
    }
    let ps = super::fonts::ps_name(&marked, style);
    st.inspector_preview =
        InspectorPreview::for_selection(st, PreviewValue::Text(json!({"font": ps})));
}

/// Live canvas preview of a pending text slider value (FB-21/FB-22): the attribute patch of
/// `text.applyAttributes` on the selected text objects. With the text editor open only the field
/// shows the pending value (the marked text is not previewed).
fn preview_text(st: &mut AppState, live: Option<f64>, patch: impl FnOnce(f64) -> Value) {
    if st.session.text_edit_attrs().is_none()
        && let Some(v) = live
    {
        st.inspector_preview = InspectorPreview::for_selection(st, PreviewValue::Text(patch(v)));
    }
}

/// Tooltip über einem Feld (ohne eigene Klickfläche; das Feld bleibt bedienbar).
pub fn tooltip_at(ui: &mut Ui, id: Id, r: Rect, text: &str) {
    let _ = ui.interact(r, id, egui::Sense::hover()).on_hover_text(text);
}

/// Farbfeld mit dem Muster (Ausschnitt des Musterbilds).
fn draw_pattern_swatch(ui: &Ui, r: Rect, name: &str) {
    let inner = r.shrink(2.0);
    if let Some(tex) = popups::pattern_swatch_texture(ui.ctx(), name) {
        let ts = tex.size_vec2();
        if ts.x > 0.0 && ts.y > 0.0 {
            ui.painter().image(
                tex.id(),
                inner,
                Rect::from_min_max(
                    pos2(0.0, 0.0),
                    pos2(inner.width() / ts.x, inner.height() / ts.y),
                ),
                Color32::WHITE,
            );
        }
    }
}

/// Teilfeld „Fülltextur“ (14013).
pub fn fill_texture_panel(ui: &mut Ui, st: &mut AppState, d: Dl, tgt: Target, enabled: bool) {
    let ta = tool_attrs(ui.ctx());
    let attrs = first_attrs(st);
    let lock = attrs
        .as_deref()
        .and_then(|a| a.fill.as_ref())
        .is_some_and(|f| f.lock_to_document);
    let (kind, tex, amount, transparent, dither) = match tgt {
        Target::Selection => {
            let a = attrs.as_deref();
            let f = a.and_then(|a| a.fill.as_ref());
            (
                ui_commands::fill_kind(f),
                a.and_then(|a| a.fill_texture.as_ref())
                    .map(|t| t.user_name.clone().unwrap_or_else(|| t.name.clone()))
                    .unwrap_or_default(),
                f.map(|f| f.texture_amount / 10).unwrap_or(0),
                f.and_then(|f| f.transparent_dither).unwrap_or(false),
                f.and_then(|f| f.gradient.as_ref())
                    .and_then(|g| g.stops.as_ref())
                    .and_then(|s| s.dither)
                    .unwrap_or(false),
            )
        }
        Target::Tool => (
            if st.session.tools.fill_color.a() == 0 {
                0
            } else {
                ta.fill_kind
            },
            ta.fill_texture.clone(),
            ta.fill_texture_amount,
            ta.fill_texture_transparent,
            false,
        ),
    };
    // Verlauf-Werkzeug: Textur immer bedienbar (es malt stets einen Verlauf, Original a_W25)
    let gradient_tool =
        tgt == Target::Tool && st.session.tools.active == fp_engine::ToolId::Gradient;
    let en = enabled && (kind != 0 || gradient_tool) && (tgt == Target::Tool || attrs.is_some());
    let id = Id::new(("pi-ftex", tgt == Target::Tool));
    fields::label(ui, d.r(0.0, 2.0, 30.0, 8.0), &t("pi-texture"), en);
    match texture_combo(ui, id.with("name"), d.r(31.0, 0.0, 55.0, 14.0), &tex, en) {
        Some(TextureChoice::Named(name)) => choose_texture(ui, st, tgt, false, &name),
        Some(TextureChoice::Other) => pick_other_texture(ui, st, tgt, false),
        None => {}
    }
    if let Some(v) = fields::number(
        ui,
        id.with("amt"),
        d.r(87.0, 0.0, 22.0, 13.0),
        Some(d.r(109.0, 0.0, 9.0, 13.0)),
        Some(amount as f64),
        (0.0, 100.0),
        0,
        en,
    ) {
        apply_fill(ui, st, tgt, json!({"textureAmount": v as i64}));
    }
    preview_slider(
        ui,
        st,
        id.with("amt"),
        tgt == Target::Selection && en,
        PreviewValue::FillTextureAmount,
    );
    // the padlock toggle of FB-44 (below) takes the right end of this row: the checkbox then ends
    // in front of it (its text still fits; long translations are fitted)
    let lock_shown = tgt == Target::Selection && (kind == 2 || kind == 3);
    let tr_w = if lock_shown { 52.0 } else { 62.0 };
    if let Some(v) = fields::checkbox(
        ui,
        id.with("tr"),
        d.r(33.0, 15.0, tr_w, 10.0),
        &t("pi-texture-transparent"),
        transparent,
        en && amount > 0,
    ) {
        apply_fill(ui, st, tgt, json!({"textureTransparent": v}));
    }
    if fields::toggle(
        ui,
        id.with("dither"),
        d.r(102.0, 14.0, 16.0, 12.0),
        Glyph::FillPattern,
        dither,
        en && kind == 2 && tgt == Target::Selection,
        &t("pi-gradient-dither"),
    ) {
        st.run("pi.fill", json!({"dither": !dither}));
    }
    // "Lock pattern to document" (Firepixel extension, FB-44): only for a selected pattern or
    // gradient; a padlock toggle in the free space between "Transparent" and the gradient dither
    // toggle (same size, like the fill kind toggles above), since the free corner below the
    // texture label holds the dither density field (FB-38) and has no room for a text checkbox
    if lock_shown
        && fields::toggle(
            ui,
            id.with("lock"),
            d.r(86.0, 14.0, 16.0, 12.0),
            Glyph::Lock,
            lock,
            en,
            &t("pi-fill-lock-tip"),
        )
    {
        st.run("pi.fill", json!({"lockPattern": !lock}));
    }
    // uniform dither density of the whole fill (Firepixel extension, FB-38), below the texture
    // name in the space the reference leaves free; selection only (tool defaults keep 0)
    if tgt == Target::Selection {
        let density = attrs
            .as_deref()
            .and_then(|a| a.fill.as_ref())
            .map(|f| f.dither_density)
            .unwrap_or(0);
        let den = enabled && kind != 0 && attrs.is_some();
        // a small checkerboard icon instead of a text label (the free space holds no label
        // next to a field wide enough for "100%"); the tooltip names the field
        let lr = d.r(0.0, 15.0, 10.0, 13.0);
        dither_icon(ui, lr, den);
        let fr = d.r(11.0, 15.0, 21.0, 13.0);
        if let Some(v) = fields::number_unit(
            ui,
            id.with("density"),
            fr,
            None,
            Some(density as f64),
            (0.0, 100.0),
            0,
            den,
            "%",
        ) {
            apply_fill(ui, st, tgt, json!({"ditherDensity": v as i64}));
        }
        tooltip_at(
            ui,
            id.with("density-tip"),
            lr.union(fr),
            &t("pi-fill-dither-density-tip"),
        );
    }
}

/// Icon of the dither density field (FB-38): a 3 × 3 checkerboard of 3 px cells, centred in `r`.
fn dither_icon(ui: &Ui, r: Rect, enabled: bool) {
    let theme = Theme::current(ui.ctx());
    let col = if enabled {
        theme.colors.text
    } else {
        theme.colors.text_disabled
    };
    let cell = 3.0;
    let o = r.center() - egui::vec2(1.5 * cell, 1.5 * cell);
    let o = pos2(o.x.round(), o.y.round());
    for j in 0..3 {
        for i in 0..3 {
            if (i + j) % 2 == 0 {
                let p = o + egui::vec2(i as f32 * cell, j as f32 * cell);
                ui.painter()
                    .rect_filled(Rect::from_min_size(p, egui::vec2(cell, cell)), 0.0, col);
            }
        }
    }
}

/// Musterdateien aus den Musterordnern ([`popups::pattern_files`]: Original, dessen Benutzervorlage,
/// `%APPDATA%\Firepixel\Patterns`, nur zur Laufzeit gelesen) plus the built-in patterns (pseudo
/// paths, [`fp_engine::resources::pattern_image`]).
pub fn pattern_files() -> Vec<std::path::PathBuf> {
    popups::pattern_files()
        .iter()
        .map(|(_, p)| p.clone())
        .collect()
}

/// Vorgabetextur des Originals: neue Füllungen und Striche tragen sie mit Menge 0 (im Original
/// gemessen: `fillTexture`/`brushTexture` „Vein“, `textureBlend` 0), die Liste zeigt dann deren
/// Namen statt „keine“ (PARITAET_A W21).
pub const DEFAULT_TEXTURE: &str = "Vein";

/// Angezeigter Name einer Textur (Dateiname ohne Endung): übersetzt, wenn die Sprache ihn kennt
/// (`pi-texture-name-<name>`, deutsch z. B. „Ader“), sonst der Dateiname.
pub fn texture_label(name: &str) -> String {
    let mut slug = String::new();
    for c in name.chars().flat_map(|c| c.to_lowercase()) {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    let key = format!("pi-texture-name-{}", slug.trim_end_matches('-'));
    if fp_i18n::has(&key) {
        t(&key)
    } else {
        name.to_string()
    }
}

/// Einträge der Texturliste (Dateiname, angezeigter Name), nach dem angezeigten Namen geordnet wie
/// im Original, und der Platz der aktuellen Textur (leer = Vorgabetextur). Die Vorgabe und eine
/// aktuelle Textur, die nicht im Ordner liegt, stehen immer in der Liste; einen Eintrag „keine“ gibt
/// es wie im Original nicht (Menge 0 = ohne Wirkung).
pub fn texture_items(current: &str) -> (Vec<(String, String)>, usize) {
    let current = if current.is_empty() {
        DEFAULT_TEXTURE
    } else {
        current
    };
    let mut v: Vec<(String, String)> = popups::texture_names()
        .iter()
        .map(|n| (n.clone(), texture_label(n)))
        .collect();
    for n in [DEFAULT_TEXTURE, current] {
        if !v.iter().any(|(x, _)| x.eq_ignore_ascii_case(n)) {
            v.push((n.to_string(), texture_label(n)));
        }
    }
    v.sort_by(|a, b| {
        a.1.to_lowercase()
            .cmp(&b.1.to_lowercase())
            .then_with(|| a.0.cmp(&b.0))
    });
    let sel = v
        .iter()
        .position(|(x, _)| x.eq_ignore_ascii_case(current))
        .unwrap_or(0);
    (v, sel)
}

/// Wahl in der Texturliste.
#[derive(Clone, Debug, PartialEq)]
pub enum TextureChoice {
    /// Eine Textur der Liste (Dateiname ohne Endung).
    Named(String),
    /// „Andere…“: Textur aus einer Datei wählen.
    Other,
}

/// Beschriftungen der Texturliste wie im Original: die Texturen ([`texture_items`]), eine Trennlinie
/// und „Andere…“ (Textur aus Datei). Liefert die Einträge und den Platz der aktuellen Textur.
pub fn texture_list_labels(current: &str) -> (Vec<(String, String)>, Vec<String>, usize) {
    let (items, sel) = texture_items(current);
    let mut labels: Vec<String> = items.iter().map(|(_, l)| l.clone()).collect();
    labels.push("-".into());
    labels.push(t("pi-texture-other"));
    (items, labels, sel)
}

/// Trennlinie in einer Aufklappliste (wie die Trenner in `fields::combo`).
fn list_separator(ui: &mut Ui) {
    let th = Theme::current(ui.ctx());
    let (sr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 7.0), egui::Sense::hover());
    ui.painter().hline(
        sr.x_range().shrink(4.0),
        sr.center().y.floor() + 0.5,
        Stroke::new(1.0, th.colors.menu_separator),
    );
}

/// Kombifeld „Textur“ (Füllung bzw. Strich): Texturen mit Bildlauf, darunter wie im Original eine
/// Trennlinie und „Andere…“ (immer sichtbar, außerhalb des Bildlaufs).
fn texture_combo(
    ui: &mut Ui,
    id: Id,
    r: Rect,
    current: &str,
    enabled: bool,
) -> Option<TextureChoice> {
    let (items, labels, sel) = texture_list_labels(&texture_display_name(current));
    let text = labels.get(sel).cloned().unwrap_or_default();
    let resp = fields::combo_box_frame(ui, id, r, &text, enabled);
    let th = Theme::current(ui.ctx());
    let mut out = None;
    // so breit wie der längste Name (Windows-Kombifeld im Original), nicht nur wie das Feld
    let list_w = fields::list_width(ui, labels.iter().map(|s| s.as_str()), r.width().max(60.0));
    egui::Popup::menu(&resp)
        .frame(crate::menu::menu_frame(&th))
        .width(list_w)
        .show(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            egui::ScrollArea::vertical()
                .max_height(360.0)
                .show(ui, |ui| {
                    for (i, (name, label)) in items.iter().enumerate() {
                        if fields::list_item(ui, label, i == sel).clicked() {
                            if i != sel {
                                out = Some(TextureChoice::Named(name.clone()));
                            }
                            ui.close();
                        }
                    }
                });
            list_separator(ui);
            if fields::list_item(
                ui,
                labels.last().map(|s| s.as_str()).unwrap_or_default(),
                false,
            )
            .clicked()
            {
                out = Some(TextureChoice::Other);
                ui.close();
            }
        });
    out
}

/// Setzt eine Textur der Liste an der Auswahl bzw. als Werkzeugvorgabe. Für die Auswahl wird die
/// Bilddatei aus den Texturordnern mit `element.setTexture` geladen, damit die Textur eine Maske hat
/// und gezeichnet wird; ohne Datei bleibt es beim Namen (`pi.fill`/`pi.stroke`).
fn choose_texture(ui: &Ui, st: &mut AppState, tgt: Target, stroke: bool, name: &str) {
    match (tgt, popups::texture_file(name)) {
        (Target::Selection, Some(f)) => st.run("element.setTexture", json!({"target": if stroke { "stroke" } else { "fill" }, "image": f.to_string_lossy(), "name": name})),
        _ if stroke => apply_stroke(ui, st, tgt, json!({"texture": name})),
        _ => apply_fill(ui, st, tgt, json!({"texture": name})),
    }
}

/// Anzeigename einer Textur-Werkzeugvorgabe: Bei einem vollen Pfad („Andere…“) der Dateiname ohne
/// Endung, sonst der Name selbst.
pub fn texture_display_name(value: &str) -> String {
    let p = std::path::Path::new(value);
    match p.file_stem() {
        Some(stem) if p.is_absolute() => stem.to_string_lossy().to_string(),
        _ => value.to_string(),
    }
}

/// „Andere…“ in der Texturliste: Bilddatei wählen; der Dateiname ohne Endung ist der Texturname.
/// Als Werkzeugvorgabe wird der volle Pfad gemerkt (Anzeige weiter der Name), damit `pi.fill`/
/// `pi.stroke` die Datei für neue Objekte auch außerhalb der Texturordner finden.
fn pick_other_texture(ui: &Ui, st: &mut AppState, tgt: Target, stroke: bool) {
    let (title, images) = (t("pi-texture-pick-title"), t("pi-filter-images"));
    let ctx = ui.ctx().clone();
    crate::dialogs::panel_menus::pick_open(
        st,
        &title,
        &[(images.as_str(), &popups::RESOURCE_IMAGE_EXTS)],
        move |st, p| {
            let name = p
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            if name.is_empty() {
                return;
            }
            match tgt {
            Target::Selection => st.run("element.setTexture", json!({"target": if stroke { "stroke" } else { "fill" }, "image": p.to_string_lossy(), "name": name})),
            Target::Tool => {
                let full = p.to_string_lossy().to_string();
                edit_tool_attrs(&ctx, |a| {
                    if stroke {
                        a.stroke_texture = full.clone();
                    } else {
                        a.fill_texture = full.clone();
                    }
                })
            }
        }
        },
    );
}

// ---------------------------------------------------------------------- Teilfeld Strich (14010)

fn apply_stroke(ui: &Ui, st: &mut AppState, tgt: Target, params: Value) {
    match tgt {
        Target::Selection => st.run("pi.stroke", params),
        Target::Tool => apply_tool_stroke(ui.ctx(), st, params),
    }
}

/// Strich-Felder des Werkzeug-Inspektors (Strichart, Größe, Rand, Textur …) auf die Werkzeugvorgabe
/// anwenden – wie ein Klick im Inspektor (auch für Tests).
pub fn apply_tool_stroke(ctx: &egui::Context, st: &mut AppState, params: Value) {
    if let Some(c) = params
        .get("color")
        .and_then(|v| v.as_str())
        .and_then(fp_color::Rgba8::parse_hex)
    {
        st.push(Action::SetColors {
            stroke: Some(c.to_argb()),
            fill: None,
        });
    }
    if params.get("none").and_then(|v| v.as_bool()) == Some(true) {
        st.push(Action::SetColors {
            stroke: Some(Argb(0x00ff_ffff)),
            fill: None,
        });
    }
    edit_tool_attrs(ctx, |a| {
        if let Some(p) = params.get("preset").and_then(|v| v.as_str()) {
            let f = fp_engine::commands::panel::find_preset(p);
            a.stroke_preset = f.and_then(|f| BRUSH_PRESETS.iter().position(|b| b == f));
            // die Strichart bringt Rand und Texturmenge mit (wie im Original; vorher zeigte
            // „Rand“ nach der Wahl den alten Wert, PAR 01/01)
            if let Some(f) = f {
                a.softness = f.softness / 10;
                a.stroke_texture_amount = f.texture_blend / 10;
            }
        }
        if let Some(v) = params.get("width").and_then(|v| v.as_f64()) {
            a.stroke_width = v.round() as i32;
        }
        if let Some(v) = params.get("softness").and_then(|v| v.as_i64()) {
            a.softness = v as i32;
        }
        if let Some(v) = params.get("placement").and_then(|v| v.as_str()) {
            a.placement = match v {
                "inside" => 0,
                "outside" => 2,
                _ => 1,
            };
        }
        if let Some(v) = params.get("texture").and_then(|v| v.as_str()) {
            a.stroke_texture = v.to_string();
        }
        if let Some(v) = params.get("textureAmount").and_then(|v| v.as_i64()) {
            a.stroke_texture_amount = v as i32;
        }
    });
    // Strichbreite für das Werkzeug: Pinselgröße der Bitmap-Werkzeuge
    if let Some(v) = params.get("width").and_then(|v| v.as_f64()) {
        st.session.tools.options.brush_size = v.max(1.0) as f32;
    }
    sync_tool_brush(st, &params);
}

/// Ist ein Vorgabepinsel gesetzt (`ToolOptions::new_brush`, aus „Strich bearbeiten“), schreiben die
/// Strich-Felder des Inspektors in ihn: Vorgabe, Stärke und Weichheit über `pi.tool.brush`, „kein
/// Strich“ entfernt ihn. Ohne Vorgabepinsel gilt die Strichvorgabe (`new_stroke`) allein.
pub fn sync_tool_brush(st: &mut AppState, params: &Value) {
    if st.session.tools.options.new_brush.is_none() {
        return;
    }
    if params.get("none").and_then(|v| v.as_bool()) == Some(true) {
        st.run("pi.tool.brush", json!({"clear": true}));
        return;
    }
    let mut p = serde_json::Map::new();
    for k in ["preset", "width", "softness"] {
        if let Some(v) = params.get(k) {
            p.insert(k.into(), v.clone());
        }
    }
    if !p.is_empty() {
        st.run("pi.tool.brush", Value::Object(p));
    }
}

/// Vorgabepinsel der Werkzeugvorgabe (`ToolOptions::new_brush`), falls gesetzt.
fn tool_brush_override(st: &AppState) -> Option<fp_model::Brush> {
    st.session
        .tools
        .options
        .new_brush
        .as_ref()
        .and_then(|v| serde_json::from_value::<fp_model::Brush>(v.clone()).ok())
}

/// Was die Strich-Felder des Werkzeug-Inspektors zeigen: (Größe, Strichart, Rand, Strich vorhanden,
/// Vorgabe-Index). Ein Vorgabepinsel (`pi.tool.brush`, Dialog „Pinselstrich bearbeiten“) hat
/// Vorrang – die Felder zeigen den Pinsel, mit dem gemalt wird (PAR 01/01).
pub fn shown_tool_stroke(
    st: &AppState,
    ctx: &egui::Context,
) -> (i32, String, i32, bool, Option<usize>) {
    let ta = tool_attrs(ctx);
    let c = st.session.tools.stroke_color;
    if let Some(b) = tool_brush_override(st) {
        let has = c.a() > 0;
        return (
            b.diameter,
            if has {
                texts::brush_label(&b)
            } else {
                t("pi-none")
            },
            b.softness / 10,
            has,
            texts::brush_preset_index(&b).filter(|_| has),
        );
    }
    let has = c.a() > 0 && ta.stroke_preset.is_some();
    let lab = ta
        .stroke_preset
        .and_then(|i| BRUSH_PRESETS.get(i))
        .map(texts::brush_preset_label)
        .unwrap_or_else(|| t("pi-none"));
    (
        ta.stroke_width,
        if has { lab } else { t("pi-none") },
        ta.softness,
        has,
        ta.stroke_preset.filter(|_| has),
    )
}

/// Kantenvorschau neben „Rand:“ (Original 23 × 23): Spitze in Pinselgröße (Größe 1 → Punkt),
/// größere Spitzen auf das Feld begrenzt, Rand als weicher Verlauf (PAR 02/19).
fn paint_edge_preview(ui: &Ui, pv: Rect, col: Color32, size: i32, soft: i32) {
    let full = (pv.height() / 2.0 - 2.0).max(1.0);
    let r0 = (size.max(1) as f32 / 2.0).clamp(0.5, full);
    let steps = if r0 < 2.0 { 1 } else { 4 };
    for k in 0..steps {
        let f = 1.0 - k as f32 / steps as f32;
        let a = if soft == 0 {
            255
        } else {
            (255.0 * (1.0 - f * soft as f32 / 120.0)).clamp(30.0, 255.0) as u8
        };
        ui.painter()
            .circle_filled(pv.center(), r0 * f, crate::theme::with_alpha(col, a));
    }
}

/// Teilfeld „Strich“ an DLU-Ursprung `d` (130 × 29 DLU).
pub fn stroke_panel(ui: &mut Ui, st: &mut AppState, d: Dl, tgt: Target, enabled: bool) {
    let theme = Theme::current(ui.ctx());
    let ta = tool_attrs(ui.ctx());
    let attrs = first_attrs(st);
    let first_el = first(st);
    let (has, color, width, label, soft, place, cur_preset) = match tgt {
        Target::Selection => {
            let a = attrs.as_deref();
            let b = a.and_then(|a| a.brush.as_ref());
            (
                b.is_some(),
                a.map(|a| a.stroke_color)
                    .filter(|_| b.is_some())
                    .map(|c| mask_well(st, c)),
                b.map(|b| b.diameter).unwrap_or(1),
                b.map(texts::brush_label).unwrap_or_else(|| t("pi-none")),
                b.map(|b| b.softness / 10).unwrap_or(0),
                first_el.as_deref().map(placement_of).unwrap_or(1),
                b.and_then(texts::brush_preset_index),
            )
        }
        Target::Tool => {
            let c = st.session.tools.stroke_color;
            let (width, label, soft, has, preset) = shown_tool_stroke(st, ui.ctx());
            (
                has,
                has.then_some(c),
                width,
                label,
                soft,
                ta.placement,
                preset,
            )
        }
    };
    let en = enabled && (tgt == Target::Tool || attrs.is_some());
    let id = Id::new(("pi-stroke", tgt == Target::Tool));
    super::glyphs::paint(
        ui.painter(),
        d.r(0.0, 0.0, 12.0, 11.0),
        Glyph::Pencil,
        &theme,
        en,
    );
    let footer = super::color_picker::Footer::Stroke {
        placement: (tgt == Target::Selection && has).then_some(place.clamp(0, 2) as u8),
        enabled: en && tgt == Target::Selection,
    };
    match popups::color_button_with(
        ui,
        id.with("sw"),
        d.r(14.0, 0.0, 16.0, 13.0),
        st,
        color,
        en,
        footer,
    ) {
        Some(ColorChoice::Color(c)) => {
            let mut p = json!({"color": ui_commands::hex(c)});
            if !has {
                p["preset"] = json!("Bleistift/1-Pixel weich");
            }
            apply_stroke(ui, st, tgt, p);
        }
        Some(ColorChoice::None) => apply_stroke(ui, st, tgt, json!({"none": true})),
        Some(ColorChoice::Placement(k)) => apply_stroke(
            ui,
            st,
            tgt,
            json!({"placement": crate::inspector::color_picker::placement_name(k)}),
        ),
        Some(_) | None => {}
    }
    // live canvas preview of the pending alpha of the colour pop-up (FB-29, TW-206)
    if tgt == Target::Selection
        && en
        && let Some(c) = popups::color_button_pending(ui.ctx(), id.with("sw"))
    {
        st.inspector_preview = InspectorPreview::for_selection(st, PreviewValue::StrokeColor(c));
    }
    if let Some(v) = fields::number(
        ui,
        id.with("w"),
        d.r(32.0, 0.0, 19.0, 13.0),
        Some(d.r(52.0, 0.0, 9.0, 13.0)),
        has.then_some(width as f64),
        (1.0, 100.0),
        0,
        en && has,
    ) {
        apply_stroke(ui, st, tgt, json!({"width": v}));
    }
    // the tip preview beside "Edge" follows the dragged size (as in the reference); on the canvas
    // a live preview of it (FB-10, Firepixel enhancement)
    let live_w = fields::slider_live(ui.ctx(), id.with("w")).filter(|_| en && has);
    if tgt == Target::Selection
        && let Some(v) = live_w
    {
        st.inspector_preview = InspectorPreview::for_selection(st, PreviewValue::StrokeWidth(v));
    }
    let width = live_w.map(|v| v.round() as i32).unwrap_or(width);
    let saved = saved_brushes(st);
    let cat_r = d.r(64.0, 0.0, 65.0, 14.0);
    let choice = popups::stroke_category(
        ui,
        id.with("cat"),
        cat_r,
        &label,
        has,
        cur_preset,
        en,
        &saved,
    );
    tooltip_at(ui, id.with("cat-tip"), cat_r, &t("pi-stroke-cat-tip"));
    if let Some(choice) = choice {
        match choice {
            None => apply_stroke(ui, st, tgt, json!({"none": true})),
            // „Pinselstrichoptionen…“ am Ende der Liste: Dialog „Pinselstrich bearbeiten“
            Some(popups::STROKE_OPTIONS_ITEM) => match tgt {
                Target::Tool => st.dialogs.open_stroke_options_for_tool(),
                Target::Selection => st.dialogs.open_stroke_options(true),
            },
            Some(i) if i >= BRUSH_PRESETS.len() => {
                if let Some(n) = saved.get(i - BRUSH_PRESETS.len()) {
                    apply_stroke(ui, st, tgt, json!({"preset": n}));
                }
            }
            Some(i) => {
                let b = &BRUSH_PRESETS[i];
                let mut p = json!({"preset": format!("{}/{}", b.category_de, b.name_de)});
                if !has {
                    p["width"] = json!(b.diameter);
                    if tgt == Target::Tool && st.session.tools.stroke_color.a() == 0 {
                        let c = st.session.prefs.default_colors().0;
                        st.push(Action::SetColors {
                            stroke: Some(Argb(c.0 | 0xff00_0000)),
                            fill: None,
                        });
                    }
                }
                apply_stroke(ui, st, tgt, p);
            }
        }
    }
    // Strichposition gibt es wie im Original nur für gewählte Objekte, nicht im Werkzeug-Inspektor
    for (x, g, v, tip, name) in [
        (2.0, Glyph::StrokeInside, 0, "pi-stroke-inside", "inside"),
        (18.0, Glyph::StrokeCenter, 1, "pi-stroke-center", "center"),
        (
            34.0,
            Glyph::StrokeOutside,
            2,
            "pi-stroke-outside",
            "outside",
        ),
    ] {
        if tgt == Target::Tool {
            break;
        }
        if fields::toggle(
            ui,
            id.with(("place", v)),
            d.r(x, 16.0, 16.0, 12.0),
            g,
            place == v,
            en,
            &t(tip),
        ) {
            apply_stroke(ui, st, tgt, json!({"placement": name}));
        }
    }
    fields::label(ui, d.r(53.0, 17.0, 26.0, 8.0), &t("pi-edge"), en && has);
    // Pinselvorschau (Kante)
    let pv = d.r(82.0, 15.0, 15.0, 14.0);
    ui.painter().rect_filled(pv, 0.0, theme.colors.field_bg);
    ui.painter().rect_stroke(
        pv,
        0.0,
        Stroke::new(1.0, theme.colors.field_border),
        StrokeKind::Inside,
    );
    if has {
        let col = color
            .map(|c| Color32::from_rgb(c.r(), c.g(), c.b()))
            .unwrap_or(theme.colors.icon_fg);
        // the tip preview follows the dragged edge softness too
        let tip_soft = fields::slider_live(ui.ctx(), id.with("soft"))
            .filter(|_| en)
            .map(|v| v.round() as i32)
            .unwrap_or(soft);
        paint_edge_preview(ui, pv, col, width, tip_soft);
    }
    if let Some(v) = fields::number(
        ui,
        id.with("soft"),
        d.r(100.0, 16.0, 19.0, 13.0),
        Some(d.r(120.0, 16.0, 9.0, 13.0)),
        has.then_some(soft as f64),
        (0.0, 100.0),
        0,
        en && has,
    ) {
        apply_stroke(ui, st, tgt, json!({"softness": v as i64}));
    }
    // live canvas preview of the dragged edge softness (FB-21, Firepixel enhancement)
    preview_slider(
        ui,
        st,
        id.with("soft"),
        tgt == Target::Selection && en && has,
        PreviewValue::StrokeSoftness,
    );
}

/// Teilfeld „Strichtextur“ (14103).
pub fn stroke_texture_panel(ui: &mut Ui, st: &mut AppState, d: Dl, tgt: Target, enabled: bool) {
    let ta = tool_attrs(ui.ctx());
    let attrs = first_attrs(st);
    let (has, tex, amount) = match tgt {
        Target::Selection => {
            let a = attrs.as_deref();
            let b = a.and_then(|a| a.brush.as_ref());
            (
                b.is_some(),
                a.and_then(|a| a.stroke_texture.as_ref())
                    .map(|t| t.user_name.clone().unwrap_or_else(|| t.name.clone()))
                    .unwrap_or_default(),
                b.map(|b| b.texture_blend / 10).unwrap_or(0),
            )
        }
        Target::Tool => match tool_brush_override(st) {
            Some(b) => (
                st.session.tools.stroke_color.a() > 0,
                ta.stroke_texture.clone(),
                if ta.stroke_texture.is_empty() {
                    b.texture_blend / 10
                } else {
                    ta.stroke_texture_amount
                },
            ),
            None => (
                st.session.tools.stroke_color.a() > 0 && ta.stroke_preset.is_some(),
                ta.stroke_texture.clone(),
                ta.stroke_texture_amount,
            ),
        },
    };
    let en = enabled && has;
    let id = Id::new(("pi-stex", tgt == Target::Tool));
    fields::label(ui, d.r(1.0, 3.0, 30.0, 8.0), &t("pi-texture"), en);
    match texture_combo(ui, id.with("name"), d.r(33.0, 0.0, 64.0, 14.0), &tex, en) {
        Some(TextureChoice::Named(name)) => choose_texture(ui, st, tgt, true, &name),
        Some(TextureChoice::Other) => pick_other_texture(ui, st, tgt, true),
        None => {}
    }
    if let Some(v) = fields::number(
        ui,
        id.with("amt"),
        d.r(100.0, 0.0, 19.0, 13.0),
        Some(d.r(120.0, 0.0, 9.0, 13.0)),
        Some(amount as f64),
        (0.0, 100.0),
        0,
        en,
    ) {
        apply_stroke(ui, st, tgt, json!({"textureAmount": v as i64}));
    }
    preview_slider(
        ui,
        st,
        id.with("amt"),
        tgt == Target::Selection && en,
        PreviewValue::StrokeTextureAmount,
    );
}

/// „Pinselstrich bearbeiten“ + Speichern/Löschen (Zeile unter der Strichtextur).
pub fn stroke_edit_row(ui: &mut Ui, st: &mut AppState, d: Dl, tgt: Target, enabled: bool) {
    let has = match tgt {
        Target::Selection => first_attrs(st).is_some_and(|a| a.brush.is_some()),
        Target::Tool => st.session.tools.stroke_color.a() > 0,
    };
    // Bearbeiten auch für die Werkzeugvorgabe (`pi.tool.brush`); Speichern/Löschen mit Auswahl bzw. im
    // Werkzeug-Inspektor mit offenem Dokument (Original: beide Symbolknöpfe aktiv, PAR 01/01, 02/15)
    let edit_en = enabled && has;
    let en = edit_en && (tgt == Target::Selection || st.session.active().is_some());
    let id = Id::new(("pi-sedit", tgt == Target::Tool));
    // ganzer Text „Pinselstrich bearbeiten“ wie im Original: Knopf 122 px breit wie dort (401–523),
    // vorher 120 px und „Pinselstrich bearbeit…“ (PAR 01/01); passt der Text in der Schrift von Firepixel
    // trotzdem nicht (Nutzerbild 8: „…bearbeit…“), wird sie leicht verkleinert statt gekürzt (BA-055)
    if fields::button_fit(
        ui,
        id.with("edit"),
        d.r(0.0, 2.0, 81.5, 13.0),
        &t("pi-stroke-edit"),
        edit_en,
    ) {
        // Werkzeug-Inspektor: Pinsel der Werkzeugvorgabe, auch wenn eine Bitmap ausgewählt ist (PINSEL-1)
        match tgt {
            Target::Tool => st.dialogs.open_stroke_options_for_tool(),
            Target::Selection => st
                .dialogs
                .open_stroke_options(st.session.active().is_some()),
        }
    }
    if fields::icon_button(
        ui,
        id.with("save"),
        d.r(84.0, 2.0, 13.0, 13.0),
        Glyph::StyleNew,
        en,
        &t("pi-stroke-save"),
    )
    .clicked()
        && en
    {
        let n = saved_brushes(st).len() + 1;
        let (title, name_label, name) = (
            t("pi-stroke-save"),
            t("pi-stroke-save-name"),
            fp_i18n::tr!("pi-stroke-default-name", n = n),
        );
        let mut dlg = crate::dialogs::form::FormDialog::new(
            "5974",
            &title,
            "pi.brush.save",
            vec![crate::dialogs::form::Field::text(
                "name",
                &name_label,
                &name,
            )],
        );
        if tgt == Target::Tool {
            // Pinsel der Werkzeugvorgabe speichern (`pi.brush.save {tool: true}`)
            dlg.base = json!({"tool": true});
        }
        st.dialogs.form = Some(dlg);
    }
    // Löschen: der aktuelle Pinsel ist ein gespeicherter Pinselstrich
    let cur = match tgt {
        Target::Selection => {
            first_attrs(st).and_then(|a| a.brush.as_ref().and_then(|b| b.user_name.clone()))
        }
        Target::Tool => tool_brush_override(st).and_then(|b| b.user_name),
    }
    .filter(|n| saved_brushes(st).contains(n));
    // wie im Original bei jedem Strich bedienbar (PAR 17/02); löscht einen gespeicherten Pinselstrich
    if fields::icon_button(
        ui,
        id.with("del"),
        d.r(98.0, 2.0, 13.0, 13.0),
        Glyph::Trash,
        en,
        &t("pi-stroke-delete"),
    )
    .clicked()
        && let Some(n) = cur
    {
        st.run("pi.brush.delete", json!({"name": n}));
    }
}

/// Namen der im Dokument gespeicherten Pinselstriche.
pub fn saved_brushes(st: &AppState) -> Vec<String> {
    st.session
        .active()
        .and_then(|d| d.doc.pages.get(d.page))
        .and_then(|p| p.presets.brushes.as_ref())
        .map(|v| v.iter().filter_map(|b| b.user_name.clone()).collect())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------- Deckkraft/Mischmodus (14012)

/// Teilfeld „Deckkraft und Mischmodus“ der Auswahl.
pub fn opacity_panel(ui: &mut Ui, st: &mut AppState, d: Dl) {
    let theme = Theme::current(ui.ctx());
    // maskiertes Einzelbild: Werte des Bildes, nicht der Maskengruppe (re 29b §2, BA-006); die
    // Befehle schreiben ebenso an den Inhalt
    // a selected mask: opacity and blend mode belong to the masked element (RE spec 68 §6)
    let owner = st
        .session
        .active()
        .and_then(|ds| match ds.selection.elements.as_slice() {
            [id] => fp_engine::docops::mask_owner(&ds.doc, ds.page, *id),
            _ => None,
        });
    let target_ids = owner.as_ref().map(|o| vec![o.id.0]);
    let sel: Vec<Arc<Element>> = owner
        .map(|o| vec![o])
        .unwrap_or_else(|| selection(st))
        .into_iter()
        .map(|e| e.masked_single().map(|(c, _)| Arc::clone(c)).unwrap_or(e))
        .collect();
    let with_ids = |mut v: Value| {
        if let Some(ids) = &target_ids {
            v["ids"] = json!(ids);
        }
        v
    };
    let f = sel.first().cloned();
    let en = f.is_some();
    // gemischte Werte (Mehrfachauswahl mit verschiedenen Werten) zeigen wie im Original ein leeres Feld
    let mixed_op = sel
        .windows(2)
        .any(|w| w[0].common.opacity != w[1].common.opacity);
    let mixed_bl = sel
        .windows(2)
        .any(|w| w[0].common.blend != w[1].common.blend);
    let op = f
        .as_ref()
        .map(|e| (e.common.opacity.to_percent()).round())
        .unwrap_or(100.0);
    let bl = f
        .as_ref()
        .and_then(|e| blend_index(e.common.blend))
        .unwrap_or(0);
    super::glyphs::paint(
        ui.painter(),
        d.r(0.0, 1.0, 9.0, 11.0),
        Glyph::Opacity,
        &theme,
        en,
    );
    if let Some(v) = fields::opacity_number(
        ui,
        "pi-opacity",
        d.r(11.0, 0.0, 19.0, 13.0),
        d.r(31.0, 0.0, 9.0, 13.0),
        (en && !mixed_op).then_some(op),
        en,
    ) {
        st.run("element.setOpacity", with_ids(json!({"opacity": v})));
    }
    // live preview of the dragged value on the canvas (FB-09, Firepixel enhancement)
    if en && let Some(v) = fields::slider_live(ui.ctx(), "pi-opacity") {
        st.inspector_preview = InspectorPreview::for_selection(st, PreviewValue::Opacity(v));
    }
    // Objekt-Liste: Namen wie im Original („Sprenkeln“ … „Radieren“, PAR 17/07b)
    let names = texts::blend_labels_obj();
    if let Some(i) = fields::combo(
        ui,
        "pi-blend",
        d.r(43.0, 0.0, 81.0, 13.0),
        &refs(&names),
        (!mixed_bl).then_some(bl),
        en,
    ) {
        let m = BLEND_ITEMS[i].1;
        if m >= 0 {
            st.run(
                "element.setBlendMode",
                with_ids(json!({"mode": m.to_string()})),
            );
        }
    }
    fields::tip(
        ui,
        "pi-opacity",
        d.r(11.0, 0.0, 19.0, 13.0),
        &t("pi-tip-opacity"),
    );
    fields::tip(
        ui,
        "pi-blend",
        d.r(43.0, 0.0, 81.0, 13.0),
        &t("pi-tip-blend-mode"),
    );
}

/// Deckkraft/Mischmodus eines Werkzeugs (Werkzeug-Quadranten 15012 ff.).
pub fn tool_opacity_panel(ui: &mut Ui, st: &mut AppState, d: Dl, with_blend: bool) {
    let theme = Theme::current(ui.ctx());
    super::glyphs::paint(
        ui.painter(),
        d.r(0.0, 1.0, 9.0, 11.0),
        Glyph::Opacity,
        &theme,
        true,
    );
    let op = st.session.tools.options.opacity as f64;
    if let Some(v) = fields::opacity_number(
        ui,
        "pi-tool-opacity",
        d.r(10.0, 0.0, 19.0, 13.0),
        d.r(30.0, 0.0, 9.0, 13.0),
        Some(op),
        true,
    ) {
        st.session.tools.options.opacity = v.round().clamp(0.0, 100.0) as u8;
    }
    fields::tip(
        ui,
        "pi-tool-opacity",
        d.r(10.0, 0.0, 19.0, 13.0),
        &t("pi-tip-opacity"),
    );
    if with_blend {
        let ta = tool_attrs(ui.ctx());
        let names = texts::blend_labels();
        if let Some(i) = fields::combo(
            ui,
            "pi-tool-blend",
            d.r(42.0, 0.0, 81.0, 13.0),
            &refs(&names),
            Some(ta.blend),
            true,
        ) && BLEND_ITEMS[i].1 >= 0
        {
            edit_tool_attrs(ui.ctx(), |a| a.blend = i);
        }
        fields::tip(
            ui,
            "pi-tool-blend",
            d.r(42.0, 0.0, 81.0, 13.0),
            &t("pi-tip-blend-mode"),
        );
    }
}

// ---------------------------------------------------------------------- Live-Filter (14150, 4224)

const FX_SEL: &str = "pi-fx-sel";

/// Kopf „Filter: + −“.
pub fn effect_header(ui: &mut Ui, st: &mut AppState, d: Dl) {
    let f = first(st);
    let en = f.is_some();
    fields::label(ui, d.r(3.0, 4.0, 25.0, 8.0), &t("pi-filters"), en);
    let resp = fields::icon_button(
        ui,
        "pi-fx-add",
        d.r(28.0, 2.0, 14.0, 13.0),
        Glyph::Plus,
        en,
        &t("pi-fx-add"),
    );
    let th = Theme::current(ui.ctx());
    ui.painter().rect_stroke(
        d.r(28.0, 2.0, 14.0, 13.0),
        2.0,
        Stroke::new(1.0, th.colors.button_border),
        StrokeKind::Inside,
    );
    if en {
        crate::dialogs::effects::add_menu(ui, &resp, st);
    }
    let n = f
        .as_ref()
        .and_then(|e| e.common.effects.as_ref())
        .map(|l| l.effects.len())
        .unwrap_or(0);
    let sel: Option<usize> = ui.data(|d| d.get_temp(Id::new(FX_SEL)));
    let can_del = en && sel.is_some_and(|i| i < n);
    if fields::icon_button(
        ui,
        "pi-fx-del",
        d.r(42.0, 2.0, 14.0, 13.0),
        Glyph::Minus,
        can_del,
        &t("pi-fx-delete"),
    )
    .clicked()
        && let Some(i) = sel
    {
        st.run("element.removeEffect", json!({"index": i}));
        ui.data_mut(|d| d.remove::<usize>(Id::new(FX_SEL)));
    }
}

/// Filterliste (4224) im Rechteck `r`.
pub fn effect_list(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let t = Theme::current(ui.ctx());
    let c = &t.colors;
    let inner = fields::list_box(ui, r);
    let Some(e) = first(st) else { return };
    let Some(list) = e.common.effects.as_ref() else {
        return;
    };
    let sel: Option<usize> = ui.data(|d| d.get_temp(Id::new(FX_SEL)));
    let row_h = 17.0;
    for (i, fx) in list.effects.iter().enumerate() {
        let rr = Rect::from_min_size(
            pos2(inner.left(), inner.top() + i as f32 * row_h),
            vec2(inner.width(), row_h),
        );
        if rr.bottom() > inner.bottom() + 1.0 {
            break;
        }
        let resp = ui.interact(rr, Id::new(("pi-fx-row", i)), egui::Sense::click());
        if sel == Some(i) {
            ui.painter().rect_filled(rr, 0.0, c.list_selected_bg);
        } else if resp.hovered() {
            ui.painter().rect_filled(rr, 0.0, c.list_hover_bg);
        }
        let cb = Rect::from_min_size(
            pos2(rr.left() + 3.0, rr.top() + 2.0),
            vec2(rr.height() - 4.0, rr.height() - 4.0),
        );
        if let Some(v) = fields::checkbox(ui, ("pi-fx-vis", i), cb, "", fx.is_visible(), true) {
            st.run(
                "element.setEffectVisible",
                json!({"index": i, "visible": v}),
            );
        }
        // ⓘ direkt nach dem Kästchen wie im Original („✓ ⓘ Kurven…“), nur bei Effekten mit
        // Einstellungen; Klick darauf bzw. Doppelklick öffnet sie (CT-1a: vorher Zahnrad am rechten
        // Rand, im schmalen Inspektor außerhalb des sichtbaren Bereichs)
        let info = Rect::from_min_size(pos2(cb.right() + 4.0, rr.top() + 2.0), vec2(13.0, 13.0));
        let label = crate::dialogs::effects::effect_label(fx);
        // Barrierefreiheit (CT-2): Zeile mit dem Namen des Filters, ⓘ als Knopf „Live-Filter
        // bearbeiten“ (nur Zeigen/Tooltip; den Klick wertet weiter die Zeile aus)
        resp.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::SelectableLabel,
                true,
                sel == Some(i),
                label.as_str(),
            )
        });
        if crate::dialogs::effects::editable(fx) {
            super::glyphs::paint(ui.painter(), info, Glyph::Info, &t, true);
            let tip = fp_i18n::t("pi-fx-edit");
            let ir = ui.interact(info, Id::new(("pi-fx-edit", i)), egui::Sense::hover());
            ir.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tip.as_str())
            });
            ir.on_hover_text(tip.as_str());
        }
        ui.painter().text(
            pos2(info.right() + 5.0, rr.center().y),
            Align2::LEFT_CENTER,
            label,
            t.ui_font(),
            c.text,
        );
        if resp.clicked() {
            ui.data_mut(|d| d.insert_temp(Id::new(FX_SEL), i));
            if resp
                .interact_pointer_pos()
                .is_some_and(|p| info.contains(p))
            {
                crate::dialogs::effects::begin_edit(st, i, fx);
            }
        }
        if resp.double_clicked() {
            crate::dialogs::effects::begin_edit(st, i, fx);
        }
    }
}

fn opacity_effect_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    opacity_panel(ui, st, d.offset(260.0, 0.0));
    effect_header(ui, st, d.offset(260.0, 14.0));
}

fn effect_list_q4(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    effect_list(ui, st, d.r(260.0, 0.0, 105.0, 29.0));
}

// ---------------------------------------------------------------------- Pfad / Rechteck

fn path_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    fill_panel(ui, st, d.offset(1.0, 0.0), Target::Selection, true);
    column_seps(ui, d, r);
    stroke_panel(ui, st, d.offset(125.0, 0.0), Target::Selection, true);
    opacity_panel(ui, st, d.offset(260.0, 0.0));
    effect_header(ui, st, d.offset(260.0, 14.0));
}

/// Senkrechte Trenner zwischen Füllung | Strich | Deckkraft (beide Zeilen).
pub fn column_seps(ui: &Ui, d: Dl, r: Rect) {
    fields::vsep(
        ui,
        d.r(122.0, 0.0, 1.0, 1.0).left(),
        r.top() + 1.0,
        r.bottom(),
    );
    fields::vsep(
        ui,
        d.r(257.0, 0.0, 1.0, 1.0).left(),
        r.top() + 1.0,
        r.bottom(),
    );
}

fn path_q4(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    column_seps(ui, d, r);
    fill_texture_panel(ui, st, d.offset(1.0, 0.0), Target::Selection, true);
    stroke_texture_panel(ui, st, d.offset(125.0, 0.0), Target::Selection, true);
    stroke_edit_row(ui, st, d.offset(125.0, 14.0), Target::Selection, true);
    effect_list(ui, st, d.r(260.0, 0.0, 105.0, 29.0));
}

fn rect_q4(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    column_seps(ui, d, r);
    fill_texture_panel(ui, st, d.offset(1.0, 0.0), Target::Selection, true);
    stroke_texture_panel(ui, st, d.offset(125.0, 0.0), Target::Selection, true);
    // Eckenrundung (14876) mit Einheit (15139)
    let prim = first(st).and_then(|e| match &e.kind {
        ElementKind::Path(p) => p.primitive.clone(),
        _ => None,
    });
    let en = prim.is_some();
    let px_key = Id::new("pi-round-px");
    let px_mode: bool = prim
        .as_ref()
        .and_then(|p| p.roundness_pixels)
        .unwrap_or_else(|| ui.data(|d| d.get_temp(px_key)).unwrap_or(false));
    // aktuelle Größe (nach B/H-Änderungen steckt die Skalierung in der Matrix, PAR 17/05)
    let (w, h) = prim
        .as_ref()
        .map(ui_commands::primitive_size)
        .unwrap_or((0.0, 0.0));
    let frac = prim.as_ref().map(|p| p.roundness).unwrap_or(0.0) as f64;
    let shown = if px_mode {
        frac * (w.min(h) as f64) / 2.0
    } else {
        frac * 100.0
    };
    fields::label_right(ui, d.r(128.0, 17.0, 63.0, 8.0), &t("pi-roundness"), en);
    let range = if px_mode {
        (0.0, (w.min(h) / 2.0).max(1.0) as f64)
    } else {
        (0.0, 100.0)
    };
    if let Some(v) = fields::number(
        ui,
        "pi-round",
        d.r(193.0, 15.0, 19.0, 13.0),
        Some(d.r(213.0, 15.0, 9.0, 13.0)),
        en.then_some(shown.round()),
        range,
        0,
        en,
    ) {
        st.run("pi.roundness", json!({"value": v, "pixels": px_mode}));
    }
    // live canvas preview of the dragged roundness (Firepixel enhancement like FB-09)
    if en && let Some(value) = fields::slider_live(ui.ctx(), "pi-round") {
        let v = PreviewValue::Roundness {
            value,
            pixels: px_mode,
        };
        st.inspector_preview = InspectorPreview::for_selection(st, v);
    }
    if let Some(i) = fields::combo(
        ui,
        "pi-round-unit",
        d.r(226.0, 15.0, 29.0, 13.0),
        &["%", "px"],
        Some(px_mode as usize),
        en,
    ) {
        ui.data_mut(|dd| dd.insert_temp(px_key, i == 1));
        st.run("pi.roundness", json!({"value": if i == 1 { frac * (w.min(h) as f64) / 2.0 } else { frac * 100.0 }, "pixels": i == 1}));
    }
    effect_list(ui, st, d.r(260.0, 0.0, 105.0, 29.0));
}

// ---------------------------------------------------------------------- Pfadoperationen (15174)

/// Enthält der aktuelle Status der Seite einen echten Pfad als Objekt (Pfad oder zusammengesetzte
/// Form)? Nur dann wirken die Pfadknöpfe im Werkzeug-Inspektor bedienbar; Masken, auch
/// Vektormasken an Bildern, zählen nicht (re 29b §2).
pub fn page_has_paths(st: &AppState) -> bool {
    st.session.active().is_some_and(|d| {
        d.doc.resolved_layers(d.page).iter().any(|rl| {
            rl.layer.cel(d.frame).is_some_and(|c| {
                c.elements
                    .iter()
                    .any(|e| matches!(e.kind, ElementKind::Path(_) | ElementKind::CompoundShape(_)))
            })
        })
    })
}

fn pathops_q5(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min + vec2(4.0, 0.0));
    let sel = selection(st);
    // Werkzeug-Inspektor (Rechteck, Stift …): wie im Original mit „Kombinieren“ und den 5
    // Pfadknöpfen (Referenzbilder tool-shapes/07, inspector-fill/21, inspector-stroke/20; FIX-G5).
    // Bedienbar wirken sie nur, wenn die Seite Pfade enthält (leeres Dokument: grau, 07); ausgelöst
    // wird im Werkzeug-Inspektor nichts.
    let tool_mode = sel.is_empty();
    if tool_mode {
        let has_paths = page_has_paths(st);
        let buttons = [
            (0.0, Glyph::OpNormal, 0u32, "pi-pathop-normal"),
            (20.0, Glyph::OpUnion, 1, "pi-pathop-union"),
            (40.0, Glyph::OpPunch, 3, "pi-pathop-punch"),
            (60.0, Glyph::OpIntersect, 2, "pi-pathop-intersect"),
            (80.0, Glyph::OpCrop, 4, "pi-pathop-crop"),
        ];
        for (x, g, op, tip) in buttons {
            let tip = t(tip);
            fields::toggle(
                ui,
                ("pi-pathop", op),
                d.r(x, 0.0, 16.0, 12.0),
                g,
                has_paths && op == 0,
                has_paths,
                tip.as_str(),
            );
        }
        fields::button(
            ui,
            "pi-combine",
            d.r(25.0, 15.0, 47.0, 14.0),
            &t("pi-combine"),
            false,
        );
        return;
    }
    let compound = sel.len() == 1 && matches!(sel[0].kind, ElementKind::CompoundShape(_));
    let paths = sel
        .iter()
        .filter(|e| matches!(e.kind, ElementKind::Path(_)))
        .count();
    // wie im Original schon bei einem gewählten Pfad bedienbar (PAR 17/02); zusammengefasst wird
    // erst ab zwei Pfaden
    let en = compound || paths >= 1;
    let can_run = compound || paths >= 2;
    let cur_op = sel.first().and_then(|e| match &e.kind {
        ElementKind::CompoundShape(c) => c.operands.first().map(|o| match &o.kind {
            ElementKind::Path(p) => p.path_op,
            _ => 0,
        }),
        _ => None,
    });
    let buttons = [
        (0.0, Glyph::OpNormal, 0u32, "pi-pathop-normal"),
        (20.0, Glyph::OpUnion, 1, "pi-pathop-union"),
        (40.0, Glyph::OpPunch, 3, "pi-pathop-punch"),
        (60.0, Glyph::OpIntersect, 2, "pi-pathop-intersect"),
        (80.0, Glyph::OpCrop, 4, "pi-pathop-crop"),
    ];
    for (x, g, op, tip) in buttons {
        let tip = t(tip);
        let tip = tip.as_str();
        let on = cur_op == Some(op) || (cur_op.is_none() && op == 0);
        if fields::toggle(
            ui,
            ("pi-pathop", op),
            d.r(x, 0.0, 16.0, 12.0),
            g,
            on,
            en,
            tip,
        ) && en
            && can_run
        {
            st.run("pi.compound.setOp", json!({"op": op}));
        }
    }
    if fields::button(
        ui,
        "pi-combine",
        d.r(25.0, 15.0, 47.0, 14.0),
        &t("pi-combine"),
        compound,
    ) {
        st.run("pi.compound.combine", json!({}));
    }
}

// ---------------------------------------------------------------------- Stil (14153)

fn style_q6(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min + vec2(4.0, 0.0));
    let f = first(st);
    let en = f.is_some();
    let styles: Vec<String> = st
        .session
        .active()
        .and_then(|d| d.doc.styles.as_ref())
        .map(|v| v.iter().map(|s| s.name.clone()).collect())
        .unwrap_or_default();
    let mut items: Vec<String> = vec![t("pi-style-none")];
    items.extend(styles.iter().cloned());
    let cur = f.as_ref().and_then(|e| e.common.style.clone());
    // mehr als ein Objekt: Liste leer wie im Original, auch wenn alle denselben bzw. keinen Stil
    // haben; „Neuer Stil“ ist dann grau (PARITAET_A O16, a_O14_O16)
    let multi = selection(st).len() > 1;
    let sel = cur
        .as_ref()
        .and_then(|n| items.iter().skip(1).position(|x| x == n).map(|i| i + 1))
        .unwrap_or(0);
    let refs: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
    if let Some(i) = fields::combo(
        ui,
        "pi-style",
        d.r(1.0, 0.0, 92.0, 13.0),
        &refs,
        (!multi).then_some(sel),
        en,
    ) {
        st.run(
            "pi.style.apply",
            json!({"name": if i == 0 { String::new() } else { items[i].clone() }}),
        );
    }
    let has = cur.is_some();
    let y = 16.0;
    if fields::icon_button(
        ui,
        "pi-style-new",
        d.r(0.0, y, 16.0, 12.0),
        Glyph::StyleNew,
        en && !multi,
        &t("pi-style-new"),
    )
    .clicked()
    {
        st.dialogs.open_new_style();
    }
    if fields::icon_button(
        ui,
        "pi-style-redef",
        d.r(20.0, y, 16.0, 12.0),
        Glyph::StyleRedefine,
        en && has,
        &t("pi-style-redefine"),
    )
    .clicked()
        && let Some(n) = &cur
    {
        st.run("pi.style.new", json!({"name": n}));
    }
    if fields::icon_button(
        ui,
        "pi-style-clear",
        d.r(40.0, y, 16.0, 12.0),
        Glyph::StyleClear,
        en && has,
        &t("pi-style-clear"),
    )
    .clicked()
        && let Some(n) = &cur
    {
        st.run("pi.style.apply", json!({"name": n}));
    }
    if fields::icon_button(
        ui,
        "pi-style-unlink",
        d.r(60.0, y, 16.0, 12.0),
        Glyph::StyleUnlink,
        en && has,
        &t("pi-style-unlink"),
    )
    .clicked()
    {
        st.run("pi.style.apply", json!({"name": ""}));
    }
    if fields::icon_button(
        ui,
        "pi-style-del",
        d.r(80.0, y, 16.0, 12.0),
        Glyph::Trash,
        en && has,
        &t("pi-style-delete"),
    )
    .clicked()
        && let Some(n) = &cur
    {
        st.run("pi.style.delete", json!({"name": n}));
    }
}

// ---------------------------------------------------------------------- Animation (14045)

fn animation_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    let anim = first(st).and_then(|e| match &e.kind {
        ElementKind::Instance(i) => Some(i.anim),
        _ => None,
    });
    let en = anim.is_some();
    let a = anim.unwrap_or(fp_model::AnimParams {
        frames: 1,
        offset: fp_geom::Point::new(0.0, 0.0),
        rotation: 0,
        scale: 100.0,
        start_opacity: 1000,
        end_opacity: 1000,
        bounds_horizontal: false,
    });
    let mut out: Option<Value> = None;
    fields::label_right(ui, d.r(13.0, 2.0, 32.0, 8.0), &t("pi-anim-states"), en);
    if let Some(v) = fields::number(
        ui,
        "pi-anim-frames",
        d.r(48.0, 0.0, 24.0, 13.0),
        Some(d.r(73.0, 0.0, 9.0, 13.0)),
        Some(a.frames as f64),
        (1.0, 60.0),
        0,
        en,
    ) {
        out = Some(json!({"frames": v as i64}));
    }
    fields::label_right(ui, d.r(88.0, 2.0, 40.0, 8.0), &t("pi-opacity"), en);
    if let Some(v) = fields::number(
        ui,
        "pi-anim-op0",
        d.r(131.0, 0.0, 24.0, 13.0),
        Some(d.r(156.0, 0.0, 9.0, 13.0)),
        Some((a.start_opacity as f64 / 10.0).round()),
        (0.0, 100.0),
        0,
        en,
    ) {
        out = Some(json!({"startOpacity": v}));
    }
    fields::label(ui, d.r(166.0, 2.0, 10.0, 8.0), &t("pi-anim-to"), en);
    if let Some(v) = fields::number(
        ui,
        "pi-anim-op1",
        d.r(177.0, 0.0, 24.0, 13.0),
        Some(d.r(202.0, 0.0, 9.0, 13.0)),
        Some((a.end_opacity as f64 / 10.0).round()),
        (0.0, 100.0),
        0,
        en,
    ) {
        out = Some(json!({"endOpacity": v}));
    }
    fields::label_right(ui, d.r(0.0, 17.0, 45.0, 8.0), &t("pi-anim-scaling"), en);
    if let Some(v) = fields::number(
        ui,
        "pi-anim-scale",
        d.r(48.0, 15.0, 24.0, 13.0),
        Some(d.r(73.0, 15.0, 9.0, 13.0)),
        Some(a.scale as f64),
        (0.0, 250.0),
        0,
        en,
    ) {
        out = Some(json!({"scale": v}));
    }
    fields::label_right(ui, d.r(89.0, 17.0, 39.0, 8.0), &t("pi-anim-rotation"), en);
    let rot = a.rotation;
    if let Some(v) = fields::number(
        ui,
        "pi-anim-rot",
        d.r(131.0, 15.0, 24.0, 13.0),
        Some(d.r(156.0, 15.0, 9.0, 13.0)),
        Some(rot.abs() as f64),
        (0.0, 360.0),
        0,
        en,
    ) {
        out = Some(json!({"rotation": if rot < 0 { -(v as i64) } else { v as i64 }}));
    }
    if fields::toggle(
        ui,
        "pi-anim-cw",
        d.r(177.0, 15.0, 15.0, 13.0),
        Glyph::RotateCw,
        rot >= 0,
        en,
        &t("pi-anim-cw"),
    ) && rot < 0
    {
        out = Some(json!({"rotation": -rot}));
    }
    if fields::toggle(
        ui,
        "pi-anim-ccw",
        d.r(192.0, 15.0, 15.0, 13.0),
        Glyph::RotateCcw,
        rot < 0,
        en,
        &t("pi-anim-ccw"),
    ) && rot > 0
    {
        out = Some(json!({"rotation": -rot}));
    }
    if let Some(p) = out {
        st.run("modify.animation.settings", p);
    }
    opacity_panel(ui, st, d.offset(260.0, 0.0));
    effect_header(ui, st, d.offset(260.0, 14.0));
}

// ---------------------------------------------------------------------- Schaltfläche (14095/14096)

fn button_strings(st: &AppState) -> Option<(fp_model::ButtonStrings, bool)> {
    first(st).and_then(|e| match &e.kind {
        ElementKind::Instance(i) => Some((i.button.clone(), i.show_down_on_load.unwrap_or(false))),
        _ => None,
    })
}

/// Button symbol of the selected instance: export settings stored on the symbol and whether the
/// symbol contains a text block (re 45 §4).
fn button_symbol_info(st: &AppState) -> Option<(Option<fp_model::ExportSettings>, bool)> {
    let e = first(st)?;
    let ElementKind::Instance(i) = &e.kind else {
        return None;
    };
    let d = st.session.active()?;
    let (key, sym) = d.doc.symbol_by_id(i.symbol_id)?;
    let mut has_text = false;
    fp_model::traverse::walk_root(
        &sym.root,
        fp_model::traverse::RootRef::Symbol(key),
        &mut |_, _, e| {
            if matches!(e.kind, ElementKind::Text(_)) {
                has_text = true;
            }
        },
    );
    Some((sym.export().cloned(), has_text))
}

fn button_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    let b = button_strings(st);
    let en = b.is_some();
    let (bs, _) = b.unwrap_or_default();
    let info = button_symbol_info(st);
    // "Text:" only for a button symbol with a text block (re 45 §4, Q3 row 1)
    let text_en = en && info.as_ref().is_some_and(|(_, t)| *t);
    fields::label(ui, d.r(2.0, 2.0, 21.0, 8.0), &t("pi-btn-text"), en);
    if let Some(v) = fields::text(
        ui,
        "pi-btn-text",
        d.r(25.0, 0.0, 124.0, 13.0),
        if text_en {
            bs.text.as_deref().unwrap_or("")
        } else {
            ""
        },
        text_en,
    ) {
        st.run("pi.button", json!({"text": v}));
    }
    // export preset of the button slice: slice icon + full-width combo, no "Edit..." button
    let own = info.and_then(|(e, _)| e);
    slice_export(ui, st, d, own.as_ref(), ExportOwner::Button, en);
    fields::label_right(ui, d.r(152.0, 2.0, 39.0, 8.0), &t("pi-link"), en);
    if let Some(v) = fields::combo_edit(
        ui,
        "pi-btn-url",
        d.r(195.0, 0.0, 156.0, 13.0),
        bs.url.as_deref().unwrap_or(""),
        &known_urls(st),
        en,
    ) {
        st.run("pi.web", json!({"url": v}));
    }
    fields::label_right(ui, d.r(174.0, 16.0, 17.0, 8.0), &t("pi-alt"), en);
    if let Some(v) = fields::text(
        ui,
        "pi-btn-alt",
        d.r(195.0, 14.0, 156.0, 12.0),
        bs.alt.as_deref().unwrap_or(""),
        en,
    ) {
        st.run("pi.web", json!({"alt": v}));
    }
}

fn button_q4(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    let b = button_strings(st);
    let en = b.is_some();
    let (bs, down) = b.unwrap_or_default();
    opacity_panel(ui, st, d.offset(17.0, 1.0));
    // instead of the filter list only an "Add Filters..." button opening the live filter menu
    // (re 45 §4, Q4)
    let resp = fields::button_response(
        ui,
        "pi-btn-add-filters",
        d.r(27.0, 16.0, 87.0, 12.0),
        &t("pi-btn-add-filters"),
        en,
    );
    if en {
        crate::dialogs::effects::add_menu(ui, &resp, st);
    }
    fields::label_right(ui, d.r(147.0, 3.0, 33.0, 8.0), &t("pi-target"), en);
    if let Some(v) = fields::combo_edit(
        ui,
        "pi-btn-target",
        d.r(182.0, 1.0, 106.0, 13.0),
        bs.target.as_deref().unwrap_or(""),
        &targets_list(),
        en,
    ) {
        st.run("pi.web", json!({"target": v}));
    }
    if let Some(v) = fields::checkbox(
        ui,
        "pi-btn-down",
        d.r(182.0, 16.0, 140.0, 10.0),
        &t("pi-btn-show-down"),
        down,
        en,
    ) {
        st.run("pi.button", json!({"showDownOnLoad": v}));
    }
}

// ---------------------------------------------------------------------- Segment / Hotspot

fn web_object(st: &AppState) -> Option<fp_model::WebObjectElem> {
    first(st).and_then(|e| match &e.kind {
        ElementKind::WebObject(w) => Some((**w).clone()),
        _ => None,
    })
}

fn link_alt(
    ui: &mut Ui,
    st: &mut AppState,
    d: Dl,
    w: Option<&fp_model::WebObjectElem>,
    width: f32,
    key: &str,
) {
    let en = w.is_some();
    fields::label(ui, d.r(0.0, 2.0, 39.0, 8.0), &t("pi-link"), en);
    fields::label_right(ui, d.r(16.0, 16.0, 23.0, 8.0), &t("pi-alt"), en);
    let url = w.and_then(|w| w.url.clone()).unwrap_or_default();
    if let Some(v) = fields::combo_edit(
        ui,
        (key, "url"),
        d.r(43.0, 0.0, width, 13.0),
        &url,
        &known_urls(st),
        en,
    ) {
        st.run("pi.web", json!({"url": v}));
    }
    let alt = w.and_then(|w| w.alt_text.clone()).unwrap_or_default();
    if let Some(v) = fields::text(ui, (key, "alt"), d.r(43.0, 14.0, width, 12.0), &alt, en) {
        st.run("pi.web", json!({"alt": v}));
    }
}

#[allow(clippy::too_many_arguments)]
fn target_field(
    ui: &mut Ui,
    st: &mut AppState,
    d: Dl,
    w: Option<&fp_model::WebObjectElem>,
    x_label: f32,
    x_field: f32,
    width: f32,
    key: &str,
) {
    let en = w.is_some();
    fields::label_right(
        ui,
        d.r(x_label, 2.0, x_field - x_label - 4.0, 8.0),
        &t("pi-target"),
        en,
    );
    let tg = w.and_then(|w| w.target.clone()).unwrap_or_default();
    if let Some(v) = fields::combo_edit(
        ui,
        (key, "target"),
        d.r(x_field, 0.0, width, 13.0),
        &tg,
        &targets_list(),
        en,
    ) {
        st.run("pi.web", json!({"target": v}));
    }
}

fn web_color(
    ui: &mut Ui,
    st: &mut AppState,
    r: Rect,
    w: Option<&fp_model::WebObjectElem>,
    key: &str,
) {
    if let Some(ColorChoice::Color(c)) = popups::color_button(
        ui,
        (key, "color"),
        r,
        st,
        w.map(|w| w.color),
        w.is_some(),
        false,
        false,
    ) {
        st.run("pi.web", json!({"color": ui_commands::hex(c)}));
    }
}

fn slice_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    let w = web_object(st);
    let en = w.is_some();
    fields::label(ui, d.r(0.0, 2.0, 23.0, 8.0), &t("pi-slice-type"), en);
    let ty = w
        .as_ref()
        .and_then(|w| w.slice.as_ref())
        .map(|s| s.slice_type.clamp(0, 2) as usize)
        .unwrap_or(0);
    if let Some(i) = fields::combo(
        ui,
        "pi-slice-type",
        d.r(27.0, 0.0, 108.0, 13.0),
        &refs(&[
            t("pi-slice-foreground"),
            t("pi-slice-background"),
            "HTML".to_string(),
        ]),
        Some(ty),
        en,
    ) {
        st.run("pi.web", json!({"sliceType": i}));
    }
    web_color(ui, st, d.r(137.0, 0.0, 14.0, 13.0), w.as_ref(), "pi-slice");
    link_alt(ui, st, d.offset(155.0, 0.0), w.as_ref(), 154.0, "pi-slice");
    // Exportformat direkt unter „Typ“, Bearbeiten-Symbol links davon (TW-61, inspector-web/01)
    let sp = w.as_ref().and_then(|w| w.slice.clone());
    if ty == 0 {
        slice_export(
            ui,
            st,
            d,
            sp.as_ref().and_then(|s| s.export.as_deref()),
            ExportOwner::Slice,
            en,
        );
    }
}

fn slice_q4(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    let w = web_object(st);
    let en = w.is_some();
    let sp = w.as_ref().and_then(|w| w.slice.clone());
    let ty = sp.as_ref().map(|s| s.slice_type).unwrap_or(0);
    match ty {
        2 => {
            fields::label_name(ui, d.r(1.0, 5.0, 24.0, 8.0), "HTML:", en);
            if fields::button(
                ui,
                "pi-slice-html",
                d.r(27.0, 2.0, 54.0, 14.0),
                &t("pi-edit"),
                en,
            ) {
                let html = sp.as_ref().and_then(|s| s.html.clone()).unwrap_or_default();
                st.dialogs.open_html_slice(html);
            }
        }
        1 => {
            let bg = sp.as_ref().and_then(|s| s.background).unwrap_or_default();
            let dd = d.offset(1.0, 0.0);
            fields::label_right(ui, dd.r(0.0, 1.0, 67.0, 8.0), &t("pi-slice-hpos"), en);
            if let Some(i) = fields::combo(
                ui,
                "pi-bg-h",
                dd.r(70.0, 0.0, 55.0, 12.0),
                &["left", "center", "right", "value"],
                Some(bg.h_align.clamp(0, 3) as usize),
                en,
            ) {
                st.run("pi.web", json!({"bgHAlign": i}));
            }
            if let Some(v) = fields::number(
                ui,
                "pi-bg-hv",
                dd.r(127.0, 0.0, 17.0, 12.0),
                None,
                Some(bg.h_value as f64),
                (0.0, 100.0),
                0,
                en && bg.h_align == 3,
            ) {
                st.run("pi.web", json!({"bgHValue": v as i64}));
            }
            fields::label_right(ui, dd.r(0.0, 16.0, 67.0, 8.0), &t("pi-slice-vpos"), en);
            if let Some(i) = fields::combo(
                ui,
                "pi-bg-v",
                dd.r(70.0, 14.0, 55.0, 12.0),
                &["top", "center", "bottom", "value"],
                Some(bg.v_align.clamp(0, 3) as usize),
                en,
            ) {
                st.run("pi.web", json!({"bgVAlign": i}));
            }
            if let Some(v) = fields::number(
                ui,
                "pi-bg-vv",
                dd.r(127.0, 14.0, 17.0, 12.0),
                None,
                Some(bg.v_value as f64),
                (0.0, 100.0),
                0,
                en && bg.v_align == 3,
            ) {
                st.run("pi.web", json!({"bgVValue": v as i64}));
            }
        }
        // Vordergrundbild: Exportformat steht unter „Typ“ (slice_q3)
        _ => {}
    }
    target_field(
        ui,
        st,
        d.offset(154.0, 1.0),
        w.as_ref(),
        15.0,
        45.0,
        143.0,
        "pi-slice",
    );
}

/// Reihenfolge der Exportformate im Segment-Inspektor wie im Original (TW-61); die übrigen
/// eingebauten Vorgaben (PNG8, PNG24, WebP, AVIF, APNG …) folgen nach den eigenen Vorgaben.
const SLICE_PRESET_ORDER: [&str; 8] = [
    "panel-opt-preset-png32",
    "panel-opt-preset-jpeg-better",
    "panel-opt-preset-jpeg-smaller",
    "panel-opt-preset-gif-web216",
    "panel-opt-preset-gif-websnap256",
    "panel-opt-preset-gif-websnap128",
    "panel-opt-preset-gif-adaptive256",
    "panel-opt-preset-anim-gif-websnap128",
];

/// Exportformat des Segments (TW-44/TW-61): Kombifeld unter „Typ“ mit den Vorgaben in der
/// Reihenfolge des Originals, danach die eigenen gespeicherten Einstellungen und die zusätzlichen
/// Formate; ohne eigene Einstellung gilt die der Seite. Links davon das Bearbeiten-Symbol, es öffnet
/// die Bildvorschau nur für das Segment.
/// Who owns the export settings edited by [`slice_export`].
#[derive(Clone, Copy, PartialEq, Eq)]
enum ExportOwner {
    /// A slice: edit icon (opens the image preview) left of the combo.
    Slice,
    /// A button instance (re 45 §4): slice icon (display only) and a full-width combo; the
    /// settings are stored on the button symbol.
    Button,
}

fn slice_export(
    ui: &mut Ui,
    st: &mut AppState,
    d: Dl,
    own_export: Option<&fp_model::ExportSettings>,
    owner: ExportOwner,
    en: bool,
) {
    use crate::panels::optimize;
    let Some(ds) = st.session.active() else {
        return;
    };
    let page_export = ds
        .doc
        .pages
        .get(ds.page)
        .and_then(|p| p.root.export.as_deref().cloned());
    let model = own_export.cloned().or(page_export).unwrap_or_default();
    let cur = own_export
        .map(|e| fp_export::OptimizeSettings::from_model(e, None))
        .unwrap_or_else(|| fp_export::doc_export::page_settings(&ds.doc, ds.page));
    let presets = fp_export::options::builtin_presets();
    let own = crate::ui_commands::export_presets(&st.session.prefs);
    let psel = optimize::preset_index(&cur);
    let norm = |e: &fp_model::ExportSettings| {
        let mut e = e.clone();
        e.frames = Default::default();
        serde_json::to_value(&e).ok()
    };
    let own_sel = if psel.is_none() {
        own.iter().position(|(_, e)| norm(e) == norm(&model))
    } else {
        None
    };
    let label = psel
        .map(|i| optimize::preset_label(presets[i].name))
        .or_else(|| own_sel.map(|i| own[i].0.clone()))
        .unwrap_or_else(|| optimize::settings_label(&cur));
    let theme = Theme::current(ui.ctx());
    let (combo, combo_id) = match owner {
        ExportOwner::Slice => (d.r(27.0, 14.0, 108.0, 13.0), "pi-slice-export"),
        ExportOwner::Button => (d.r(25.0, 14.0, 124.0, 13.0), "pi-btn-export"),
    };
    let resp = fields::combo_box_frame(ui, Id::new(combo_id), combo, &label, en);
    let mut chosen: Option<fp_model::ExportSettings> = None;
    // vorn die Liste des Originals, dann eigene Vorgaben, dann die zusätzlichen Formate
    let front: Vec<usize> = SLICE_PRESET_ORDER
        .iter()
        .filter_map(|k| {
            presets
                .iter()
                .position(|p| optimize::preset_key(p.name) == Some(*k))
        })
        .collect();
    let extra: Vec<usize> = (0..presets.len()).filter(|i| !front.contains(i)).collect();
    let labels: Vec<String> = front
        .iter()
        .map(|i| optimize::preset_label(presets[*i].name))
        .chain(own.iter().map(|(n, _)| n.clone()))
        .chain(
            extra
                .iter()
                .map(|i| optimize::preset_label(presets[*i].name)),
        )
        .collect();
    let list_w = fields::list_width(ui, labels.iter().map(|s| s.as_str()), combo.width());
    egui::Popup::menu(&resp)
        .frame(crate::menu::menu_frame(&theme))
        .width(list_w)
        .show(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            ui.set_min_width(list_w);
            // (eingebaut?, Index): eigene, gespeicherte Einstellungen (Optimieren ▸ „Einstellungen
            // speichern“, wie `par_preset`) zwischen der Liste des Originals und den Zusatzformaten
            let order = front
                .iter()
                .map(|i| (true, *i))
                .chain((0..own.len()).map(|i| (false, i)))
                .chain(extra.iter().map(|i| (true, *i)));
            for (builtin, i) in order {
                let (text, sel) = if builtin {
                    (optimize::preset_label(presets[i].name), Some(i) == psel)
                } else {
                    (own[i].0.clone(), own_sel == Some(i))
                };
                if fields::list_item(ui, &text, sel).clicked() {
                    chosen = Some(if builtin {
                        let mut s = presets[i].settings.clone();
                        s.matte = cur.matte;
                        s.to_model(&model)
                    } else {
                        own[i].1.clone()
                    });
                    ui.close();
                }
            }
        });
    if let Some(e) = chosen
        && let Ok(v) = serde_json::to_value(&e)
    {
        let cmd = match owner {
            ExportOwner::Slice => "pi.web",
            ExportOwner::Button => "pi.button",
        };
        st.run(cmd, json!({"exportSettings": v}));
    }
    if owner == ExportOwner::Button {
        crate::icons::paint(
            ui.painter(),
            d.r(6.0, 15.0, 12.0, 11.0),
            crate::icons::Icon::Slice,
            &crate::icons::IconStyle::new(&theme, None, en),
        );
        return;
    }
    if fields::icon_button(
        ui,
        "pi-slice-export-edit",
        d.r(4.0, 14.0, 14.0, 13.0),
        Glyph::EditImage,
        en,
        &t("pi-slice-export-edit"),
    )
    .clicked()
    {
        // Bildvorschau nur für dieses Segment, OK schreibt in das Segment (TW-44)
        if let Some(id) = first(st).map(|e| e.id) {
            st.dialogs.preview = crate::dialogs::image_preview::ImagePreview::for_slice(st, id);
        }
    }
}

fn hotspot_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    let w = web_object(st);
    let en = w.is_some();
    let dd = d.offset(2.0, 0.0);
    fields::label(ui, dd.r(1.0, 2.0, 22.0, 11.0), &t("pi-shape"), en);
    let sh = w
        .as_ref()
        .map(|w| w.shape.code().clamp(0, 2) as usize)
        .unwrap_or(0);
    if let Some(i) = fields::combo(
        ui,
        "pi-hot-shape",
        dd.r(26.0, 0.0, 106.0, 13.0),
        &refs(&tl(&[
            "pi-shape-rectangle",
            "pi-shape-ellipse",
            "pi-shape-polygon",
        ])),
        Some(sh),
        en,
    ) {
        st.run("pi.web", json!({"shape": i}));
    }
    web_color(ui, st, dd.r(133.0, 0.0, 14.0, 13.0), w.as_ref(), "pi-hot");
    link_alt(ui, st, d.offset(153.0, 0.0), w.as_ref(), 151.0, "pi-hot");
}

fn hotspot_q4(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    let w = web_object(st);
    target_field(
        ui,
        st,
        d.offset(152.0, 1.0),
        w.as_ref(),
        14.0,
        43.0,
        151.0,
        "pi-hot",
    );
}

// ---------------------------------------------------------------------- Text (14141/14144)

struct TextVals {
    font: String,
    size: f32,
    color: Option<Argb>,
    bold: bool,
    italic: bool,
    underline: bool,
    tracking: f32,
    leading: f32,
    leading_px: bool,
    align: i32,
    indent: f32,
    space_before: f32,
    space_after: f32,
    hscale: f32,
    baseline: f32,
    aa: i32,
    auto_kern: bool,
    on_path: Option<f32>,
    /// Textausrichtung `ORIi` (0 horizontal l→r, 1 vertikal r→l, 2 horizontal r→l, 3 vertikal l→r).
    orientation: i32,
}

/// Glättung eines Textobjekts für die Liste: −1 = keine (harte Füllkante `FETi 0`, kein
/// `TAAi`-Wert), sonst `TAAi` (re 9 §3.5: 0 glatt, 1 scharf, 2 stark, 3 System, 4 benutzerdef.).
fn text_aa(t: &fp_model::TextElem) -> i32 {
    if t.attrs.fill.as_ref().is_some_and(|f| f.edge == 0) {
        -1
    } else {
        t.antialias
    }
}

/// Einträge der Glättungsliste (Reihenfolge wie im Original) mit den Namen von
/// `text.setProperties`/`text.setToolDefaults {antialias}`; Index 4 ist der Trenner.
const AA_NAMES: [&str; 6] = ["none", "crisp", "strong", "smooth", "", "custom"];

/// Listenindex zur Glättung ([`text_aa`] bzw. `tool_text_defaults`): System (3) zeigt das Original
/// als „scharf“.
pub fn aa_index(aa: i32) -> usize {
    match aa {
        -1 => 0,
        1 | 3 => 1,
        2 => 2,
        4 => 5,
        _ => 3,
    }
}

fn text_vals(st: &AppState, _ctx: &egui::Context, tgt: Target) -> Option<TextVals> {
    // Vorgaben des Textwerkzeugs für neue Texte (Glättung, Unterschneidung, Richtung; fp-engine)
    let (tool_aa, tool_kern, tool_ori) = fp_engine::commands::tool_text_defaults(&st.session.prefs);
    // Offener Text-Editor: Werte an der Markierung bzw. am Caret (ändern sich live mit)
    if let Some(a) = st.session.text_edit_attrs() {
        let e = first(st);
        let t = e.as_ref().and_then(|e| {
            if let ElementKind::Text(t) = &e.kind {
                Some(t)
            } else {
                None
            }
        });
        // neuer Text (noch kein Objekt): die Werkzeugvorgaben, die er beim Übernehmen bekommt
        let t = t.filter(|_| tgt == Target::Selection);
        return Some(TextVals {
            font: a.font.clone().unwrap_or_default(),
            size: a.size.unwrap_or(12.0),
            color: a.color.map(Argb),
            bold: a.bold.unwrap_or(false),
            italic: a.italic.unwrap_or(false),
            underline: a.underline.unwrap_or(false),
            tracking: a.tracking.unwrap_or(0.0),
            leading: a.leading.unwrap_or(1.0),
            leading_px: a.leading_mode == Some(1),
            align: a.alignment.unwrap_or(0),
            indent: a.indent.unwrap_or(0.0),
            space_before: a.space_before.unwrap_or(0.0),
            space_after: a.space_after.unwrap_or(0.0),
            hscale: a.horizontal_scale.unwrap_or(1.0),
            baseline: a.baseline_shift.unwrap_or(0.0),
            aa: t.map_or(tool_aa, |t| text_aa(t)),
            auto_kern: t.map_or(tool_kern, |t| t.auto_kern),
            on_path: t.and_then(|t| t.on_path.as_ref().map(|o| o.offset.unwrap_or(0.0))),
            orientation: t.map_or(tool_ori, |t| t.orientation),
        });
    }
    match tgt {
        Target::Tool => {
            let o = &st.session.tools.options;
            let (leading, leading_px) = tool_leading(st);
            let pr = |k: &str, d: f64| st.session.prefs.get_real(k, d) as f32;
            Some(TextVals {
                font: o.text_font.clone(),
                size: o.text_size,
                color: Some(o.text_color),
                bold: tool_style(st, "Text_Bold"),
                italic: tool_style(st, "Text_Italic"),
                underline: tool_style(st, "Text_Underline"),
                // stored as field value / 100 (re 56 section 0)
                tracking: pr("Text_RangeKerning", 0.0) * 100.0,
                leading,
                leading_px,
                align: tool_align(st),
                indent: pr("Text_ParagraphIndent", 0.0),
                space_before: pr("Text_ParagraphSpacingBefore", 0.0),
                space_after: pr("Text_ParagraphSpacingAfter", 0.0),
                hscale: pr("Text_HorizScale", 1.0),
                baseline: pr("Text_BaselineShift", 0.0),
                aa: tool_aa,
                auto_kern: tool_kern,
                on_path: None,
                orientation: tool_ori,
            })
        }
        Target::Selection => {
            let e = first(st)?;
            let ElementKind::Text(t) = &e.kind else {
                return None;
            };
            let runs = t.resolved_runs();
            let a = runs
                .first()
                .map(|(a, _)| a.clone())
                .unwrap_or_else(|| t.initial.clone());
            Some(TextVals {
                font: a.font.clone().or(a.ps_font.clone()).unwrap_or_default(),
                size: a.size.unwrap_or(12.0),
                color: Some(a.color.map(Argb).unwrap_or(t.attrs.fill_color)),
                bold: a.bold.unwrap_or(false),
                italic: a.italic.unwrap_or(false),
                underline: a.underline.unwrap_or(false),
                tracking: a.tracking.unwrap_or(0.0),
                leading: a.leading.unwrap_or(1.0),
                leading_px: a.leading_mode == Some(1),
                align: a.alignment.unwrap_or(0),
                indent: a.indent.unwrap_or(0.0),
                space_before: a.space_before.unwrap_or(0.0),
                space_after: a.space_after.unwrap_or(0.0),
                hscale: a.horizontal_scale.unwrap_or(1.0),
                baseline: a.baseline_shift.unwrap_or(0.0),
                aa: text_aa(t),
                auto_kern: t.auto_kern,
                on_path: t.on_path.as_ref().map(|o| o.offset.unwrap_or(0.0)),
                orientation: t.orientation,
            })
        }
    }
}

/// Style default of the text tool (`Text_Bold`, `Text_Italic`, `Text_Underline`; app-wide prefs).
fn tool_style(st: &AppState, key: &str) -> bool {
    st.session.prefs.get_bool(key, false)
}

/// Alignment default of the text tool (`Text_Justification`, factory value left) as new text gets
/// it: a stored "stretched" (4) shows left (TW-133).
fn tool_align(st: &AppState) -> i32 {
    fp_engine::commands::effective_alignment(
        st.session
            .prefs
            .get_int("Text_Justification", 0)
            .clamp(0, 4) as i32,
    )
}

/// Zeilenabstand des Textwerkzeugs aus den Voreinstellungen (fp-engine legt ihn auf neue Texte).
fn tool_leading(st: &AppState) -> (f32, bool) {
    (
        st.session.prefs.get_real("Text_Leading", 1.2) as f32,
        st.session.prefs.get_int("Text_LeadingMode", 0) == 1,
    )
}

/// Absatz-/Zeichenvorgaben des Textwerkzeugs: Voreinstellung und Parameter von `text.setAttributes`
/// (wie im Original in den Voreinstellungen gemerkt, UX-A W15).
/// Not the tracking: new text gets range kerning 0 (R6, re 56 section 3).
const TEXT_TOOL_PREFS: [(&str, &str, f64); 5] = [
    ("Text_ParagraphIndent", "indent", 0.0),
    ("Text_ParagraphSpacingBefore", "spaceBefore", 0.0),
    ("Text_ParagraphSpacingAfter", "spaceAfter", 0.0),
    ("Text_HorizScale", "horizontalScale", 1.0),
    ("Text_BaselineShift", "baselineShift", 0.0),
];

/// Vorgaben des Textwerkzeugs als Parameter für `text.setAttributes` (nur abweichende Werte;
/// Schrift, Größe, Farbe und Zeilenabstand setzt fp-engine selbst).
pub fn text_tool_defaults(_ctx: &egui::Context, st: &AppState) -> serde_json::Map<String, Value> {
    let mut m = serde_json::Map::new();
    for (k, on) in [
        ("bold", tool_style(st, "Text_Bold")),
        ("italic", tool_style(st, "Text_Italic")),
        ("underline", tool_style(st, "Text_Underline")),
    ] {
        if on {
            m.insert(k.into(), json!(true));
        }
    }
    let align = tool_align(st);
    if align != 0 {
        m.insert("alignment".into(), json!(align));
    }
    for (pref, k, d) in TEXT_TOOL_PREFS {
        let v = st.session.prefs.get_real(pref, d);
        if (v - d).abs() > 1e-6 {
            m.insert(k.into(), json!(v));
        }
    }
    m
}

/// Beginnt das Textwerkzeug einen **neuen** Text, bekommt er die Vorgaben aus dem Inspektor
/// (Laufweite, Absatzeinzug, Abstände, Horizontalskala, Grundlinie, F/K/U, Ausrichtung) – über
/// `text.setAttributes` an den Editor (Grundattribute, kein Verlaufsschritt). Einmal je Editor.
pub fn apply_text_tool_defaults(ctx: &egui::Context, st: &mut AppState) {
    let key = Id::new("fv1-text-defaults-applied");
    if !st.session.tool_is_editing_text() {
        ctx.data_mut(|d| d.remove::<bool>(key));
        return;
    }
    if ctx.data(|d| d.get_temp::<bool>(key)).unwrap_or(false) {
        return;
    }
    ctx.data_mut(|d| d.insert_temp(key, true));
    let new_text = st
        .session
        .tool_feedback()
        .into_iter()
        .any(|o| matches!(o, fp_engine::Overlay::TextEdit(v) if v.target.is_none()));
    let p = text_tool_defaults(ctx, st);
    if new_text && !p.is_empty() {
        st.run("text.setAttributes", Value::Object(p));
    }
}

fn set_text(_ui: &Ui, st: &mut AppState, _tgt: Target, p: Value) {
    // fp-engine decides (TW-138): open text editor → the marked text, text objects selected →
    // those texts; in every case the `Text_*` defaults (new text ignores `Text_FillColor`)
    st.run("text.applyAttributes", p);
}

/// Text Zeile 1 (14141): Schriftfamilie, Stil, Größe, Farbe, Laufweite, Zeilenabstand, B/I/U, Ausrichtung.
pub fn text_row1(ui: &mut Ui, st: &mut AppState, d: Dl, tgt: Target) {
    let theme = Theme::current(ui.ctx());
    let tv = text_vals(st, ui.ctx(), tgt);
    let en = tv.is_some();
    let v = tv.unwrap_or(TextVals {
        font: String::new(),
        size: 12.0,
        color: None,
        bold: false,
        italic: false,
        underline: false,
        tracking: 0.0,
        leading: 1.0,
        leading_px: false,
        align: 0,
        indent: 0.0,
        space_before: 0.0,
        space_after: 0.0,
        hscale: 1.0,
        baseline: 0.0,
        aa: 3,
        auto_kern: true,
        on_path: None,
        orientation: 0,
    });
    let id = Id::new(("pi-text", tgt == Target::Tool));
    let fams = super::fonts::families();
    let (family, style) = super::fonts::lookup(&v.font);
    let names: Vec<&str> = fams.iter().map(|f| f.name.as_str()).collect();
    let fsel = names.iter().position(|n| *n == family);
    let fam_r = d.r(4.0, 0.0, 116.0, 13.0);
    if fsel.is_none() && en {
        let resp = fields::combo_box_frame(ui, id.with("fam"), fam_r, &family, en);
        let _ = resp;
    } else {
        if let Some(i) = super::fonts::family_combo(ui, id.with("fam"), fam_r, &names, fsel, en) {
            let ps = super::fonts::ps_name(names[i], &style);
            // auch aus dem Inspektor in „zuletzt benutzt“ des Schrift-Menüs (BA-061)
            crate::menu::note_recent_font(st, &ps);
            set_text(ui, st, tgt, json!({"font": ps}));
        }
        if en {
            preview_marked_font(ui.ctx(), st, &family, &style);
        }
    }
    // descriptive tooltips as measured (RE spec 60 §4); none on the arrows, glyphs and labels
    fields::tip(ui, id.with("fam"), fam_r, &t("pi-tip-font-family"));
    let styles: Vec<String> = fams
        .iter()
        .find(|f| f.name == family)
        .map(|f| f.styles.iter().map(|s| s.0.clone()).collect())
        .unwrap_or_else(|| vec![style.clone()]);
    let srefs: Vec<&str> = styles.iter().map(|s| s.as_str()).collect();
    let ssel = srefs.iter().position(|s| *s == style);
    if let Some(i) = fields::combo(
        ui,
        id.with("style"),
        d.r(122.0, 0.0, 77.0, 13.0),
        &srefs,
        ssel,
        en,
    ) {
        set_text(
            ui,
            st,
            tgt,
            json!({"font": super::fonts::ps_name(&family, srefs[i])}),
        );
    }
    fields::tip(
        ui,
        id.with("style"),
        d.r(122.0, 0.0, 77.0, 13.0),
        &t("pi-tip-font-style"),
    );
    // Feld nimmt 1..5000 an; der Schieber hat wie im Original 8..96 in ganzen Schritten (11 §3.5.5,
    // 09: Schieberbereiche der Text-Inspektorfelder) – vorher 1..5000 auf 130 px ≈ 38 pt je Pixel
    // senkrechter Popup-Schieber 8..96 in ganzen Schritten (Nutzerbefund N14: vorher 100er-Sprünge);
    // beim Ziehen zeigt das Feld den Wert schon mit
    let live = ui.data(|dd| dd.get_temp::<f64>(id.with("size-arrow").with("val")));
    let mut size = fields::number(
        ui,
        id.with("size"),
        d.r(202.0, 0.0, 27.0, 13.0),
        None,
        en.then(|| live.unwrap_or(v.size as f64)),
        (1.0, 5000.0),
        1,
        en,
    );
    if let Some(s) = fields::slider_arrow(
        ui,
        id.with("size-arrow"),
        d.r(230.0, 0.0, 9.0, 13.0),
        (v.size as f64).clamp(8.0, 96.0),
        (8.0, 96.0),
        0,
        en,
    ) {
        size = Some(s);
    }
    if let Some(s) = size {
        set_text(ui, st, tgt, json!({"size": s}));
    }
    // live canvas preview of the dragged size (FB-21, Firepixel enhancement)
    let live = ui.data(|dd| dd.get_temp::<f64>(id.with("size-arrow").with("val")));
    preview_text(st, live.filter(|_| en), |s| json!({"size": s}));
    fields::tip(
        ui,
        id.with("size"),
        d.r(202.0, 0.0, 27.0, 13.0),
        &t("pi-tip-font-size"),
    );
    if let Some(ColorChoice::Color(c)) = popups::color_button(
        ui,
        id.with("color"),
        d.r(240.0, 0.0, 15.0, 13.0),
        st,
        v.color,
        en,
        false,
        true,
    ) {
        set_text(ui, st, tgt, json!({"color": ui_commands::hex(c)}));
    }
    // live canvas preview of the pending alpha of the colour pop-up (FB-29)
    let pending = popups::color_button_pending(ui.ctx(), id.with("color")).filter(|_| en);
    if let Some(c) = pending
        && st.session.text_edit_attrs().is_none()
    {
        st.inspector_preview = InspectorPreview::for_selection(
            st,
            PreviewValue::Text(json!({"color": ui_commands::hex(c)})),
        );
    }
    super::glyphs::paint(
        ui.painter(),
        d.r(1.0, 16.0, 12.0, 11.0),
        Glyph::Tracking,
        &theme,
        en,
    );
    if let Some(x) = fields::number(
        ui,
        id.with("track"),
        d.r(15.0, 15.0, 27.0, 13.0),
        Some(d.r(43.0, 15.0, 9.0, 13.0)),
        en.then_some(v.tracking as f64),
        (-99.0, 200.0),
        0,
        en,
    ) {
        set_text(ui, st, tgt, json!({"tracking": x}));
    }
    let live = fields::slider_live(ui.ctx(), id.with("track")).filter(|_| en);
    preview_text(st, live, |x| json!({"tracking": x}));
    fields::tip(
        ui,
        id.with("track"),
        d.r(15.0, 15.0, 27.0, 13.0),
        &t("pi-tip-tracking"),
    );
    super::glyphs::paint(
        ui.painter(),
        d.r(55.0, 16.0, 12.0, 11.0),
        Glyph::Leading,
        &theme,
        en,
    );
    // whole numbers rounded half up (37.5 px -> 38), re 56 section 5.4
    let lead_shown = fp_engine::commands::leading_field_value(v.leading, v.leading_px);
    if let Some(x) = fields::number(
        ui,
        id.with("lead"),
        d.r(69.0, 15.0, 27.0, 13.0),
        Some(d.r(97.0, 15.0, 9.0, 13.0)),
        en.then_some(lead_shown),
        (0.0, 250.0),
        0,
        en,
    ) {
        let val = if v.leading_px { x } else { x / 100.0 };
        set_text(
            ui,
            st,
            tgt,
            json!({"leading": val, "leadingMode": v.leading_px as i64}),
        );
    }
    let live = fields::slider_live(ui.ctx(), id.with("lead")).filter(|_| en);
    preview_text(st, live, |x| {
        let val = if v.leading_px { x } else { x / 100.0 };
        json!({"leading": val, "leadingMode": v.leading_px as i64})
    });
    fields::tip(
        ui,
        id.with("lead"),
        d.r(69.0, 15.0, 27.0, 13.0),
        &t("pi-tip-leading"),
    );
    if let Some(i) = fields::combo(
        ui,
        id.with("leadu"),
        d.r(109.0, 15.0, 28.0, 13.0),
        &["%", "px"],
        Some(v.leading_px as usize),
        en,
    ) {
        let val = fp_engine::commands::convert_leading(v.leading, v.size, v.leading_px, i == 1);
        set_text(ui, st, tgt, json!({"leading": val, "leadingMode": i}));
    }
    fields::tip(
        ui,
        id.with("leadu"),
        d.r(109.0, 15.0, 28.0, 13.0),
        &t("pi-tip-leading-unit"),
    );
    if fields::toggle(
        ui,
        id.with("b"),
        d.r(140.0, 15.0, 13.0, 12.0),
        Glyph::Bold,
        v.bold,
        en,
        &t("pi-text-bold"),
    ) {
        set_text(ui, st, tgt, json!({"bold": !v.bold}));
    }
    if fields::toggle(
        ui,
        id.with("i"),
        d.r(154.0, 15.0, 13.0, 12.0),
        Glyph::Italic,
        v.italic,
        en,
        &t("pi-text-italic"),
    ) {
        set_text(ui, st, tgt, json!({"italic": !v.italic}));
    }
    if fields::toggle(
        ui,
        id.with("u"),
        d.r(168.0, 15.0, 13.0, 12.0),
        Glyph::Underline,
        v.underline,
        en,
        &t("pi-text-underline"),
    ) {
        set_text(ui, st, tgt, json!({"underline": !v.underline}));
    }
    // Knopf „Textausrichtung festlegen“ (ab/cd ▾, 15077) neben U, rechts davon die vier
    // Ausrichtungen (Teilfeld 14944 ab DLU 200) wie im Original (UX-A W17, a_133); „Erzwungener
    // Blocksatz“ bleibt im Menü Text ▸ Ausrichten
    // im Textwerkzeug die Vorgabe für neue Texte (`text.setToolDefaults`, kein Verlaufsschritt)
    let editing = st.session.tool_is_editing_text();
    if let Some(o) = orientation_button(
        ui,
        id.with("orient"),
        d.r(184.0, 15.0, 13.0, 12.0),
        v.orientation,
        en && (tgt == Target::Tool || !editing),
    ) {
        let name = ORIENTATIONS[o as usize].0;
        match tgt {
            Target::Selection => st.run("text.setProperties", json!({"orientation": name})),
            Target::Tool => st.run("text.setToolDefaults", json!({"orientation": name})),
        }
    }
    let aligns = [
        (200.0, Glyph::AlignLeft, 0, "pi-text-align-left"),
        (214.0, Glyph::AlignCenter, 1, "pi-text-align-center"),
        (228.0, Glyph::AlignRight, 2, "pi-text-align-right"),
        (242.0, Glyph::AlignJustify, 3, "pi-text-align-justify"),
    ];
    for (x, g, a, tip) in aligns {
        if fields::toggle(
            ui,
            id.with(("al", a)),
            d.r(x, 15.0, 13.0, 12.0),
            g,
            v.align == a,
            en,
            &t(tip),
        ) {
            set_text(ui, st, tgt, json!({"alignment": a}));
        }
    }
}

/// Textausrichtungen (`text.setProperties {orientation}`, `ORIi` 0–3) mit Menütext.
pub const ORIENTATIONS: [(&str, &str); 4] = [
    ("horizontalLtr", "pi-text-orient-hlr"),
    ("verticalRtl", "pi-text-orient-vrl"),
    ("horizontalRtl", "pi-text-orient-hrl"),
    ("verticalLtr", "pi-text-orient-vlr"),
];

/// Knopf „ab/cd ▾“: öffnet das Menü der vier Textausrichtungen; liefert die gewählte (0–3).
fn orientation_button(ui: &mut Ui, id: Id, r: Rect, current: i32, enabled: bool) -> Option<i32> {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let resp = ui.interact(
        r,
        id,
        if enabled {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );
    let open = egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&resp));
    let hot = enabled && (resp.hovered() || open);
    ui.painter()
        .rect_filled(r, 2.0, if hot { c.button_hover_bg } else { c.button_bg });
    ui.painter().rect_stroke(
        r,
        2.0,
        Stroke::new(
            1.0,
            if hot {
                c.button_hover_border
            } else {
                c.button_border
            },
        ),
        StrokeKind::Inside,
    );
    let col = if enabled { c.icon_fg } else { c.icon_disabled };
    let f = egui::FontId::proportional(7.5);
    let vertical = current == 1 || current == 3;
    let (a, b) = if vertical { ("ac", "bd") } else { ("ab", "cd") };
    ui.painter().text(
        pos2(r.left() + 3.0, r.top() + 1.0),
        Align2::LEFT_TOP,
        a,
        f.clone(),
        col,
    );
    ui.painter().text(
        pos2(r.left() + 3.0, r.bottom() - 1.0),
        Align2::LEFT_BOTTOM,
        b,
        f,
        col,
    );
    let tri = vec![
        pos2(r.right() - 6.0, r.bottom() - 5.0),
        pos2(r.right() - 2.0, r.bottom() - 5.0),
        pos2(r.right() - 4.0, r.bottom() - 3.0),
    ];
    ui.painter()
        .add(egui::Shape::convex_polygon(tri, col, Stroke::NONE));
    let mut out = None;
    let name = t("pi-text-orientation");
    crate::ui_kit::icon_button_info(&resp, enabled, &name, None);
    fields::log_tip(ui, r, &name);
    let resp = resp.on_hover_text(&name);
    egui::Popup::menu(&resp)
        .frame(crate::menu::menu_frame(&theme))
        .show(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            ui.set_min_width(200.0);
            for (i, (_, key)) in ORIENTATIONS.iter().enumerate() {
                if fields::list_item(ui, &t(key), i as i32 == current).clicked() {
                    out = Some(i as i32);
                    ui.close();
                }
            }
        });
    out
}

/// Text Zeile 2 (14144): Einzug, Abstände, Horizontalskala, Grundlinie, Glättung, Unterschneidung, Textversatz.
pub fn text_row2(ui: &mut Ui, st: &mut AppState, d: Dl, tgt: Target) {
    let theme = Theme::current(ui.ctx());
    let tv = text_vals(st, ui.ctx(), tgt);
    // im Textwerkzeug schon vor dem Tippen bedienbar (Vorgabe für den neuen Text, UX-A W15)
    let en = tv.is_some();
    let Some(v) = tv else { return };
    let editing = st.session.text_edit_attrs().is_some();
    // Glättung und Unterschneidung gehören zum Textobjekt (nicht zum Editor); im Textwerkzeug sind
    // sie die Vorgabe für neue Texte (`text.setToolDefaults`, fp-engine legt sie in `draw.text` an)
    let en_obj = en;
    let id = Id::new(("pi-text2", tgt == Target::Tool));
    super::glyphs::paint(
        ui.painter(),
        d.r(1.0, 1.0, 12.0, 11.0),
        Glyph::Indent,
        &theme,
        en,
    );
    if let Some(x) = fields::number(
        ui,
        id.with("ind"),
        d.r(16.0, 1.0, 26.0, 13.0),
        Some(d.r(43.0, 1.0, 9.0, 13.0)),
        Some(v.indent as f64),
        (-99.0, 100.0),
        0,
        en,
    ) {
        set_text(ui, st, tgt, json!({"indent": x}));
    }
    let live = fields::slider_live(ui.ctx(), id.with("ind")).filter(|_| en);
    preview_text(st, live, |x| json!({"indent": x}));
    fields::tip(
        ui,
        id.with("ind"),
        d.r(16.0, 1.0, 26.0, 13.0),
        &t("pi-tip-para-indent"),
    );
    super::glyphs::paint(
        ui.painter(),
        d.r(56.0, 1.0, 12.0, 11.0),
        Glyph::SpaceBefore,
        &theme,
        en,
    );
    if let Some(x) = fields::number(
        ui,
        id.with("sb"),
        d.r(71.0, 1.0, 23.0, 13.0),
        Some(d.r(95.0, 1.0, 9.0, 13.0)),
        Some(v.space_before as f64),
        (-99.0, 100.0),
        0,
        en,
    ) {
        set_text(ui, st, tgt, json!({"spaceBefore": x}));
    }
    let live = fields::slider_live(ui.ctx(), id.with("sb")).filter(|_| en);
    preview_text(st, live, |x| json!({"spaceBefore": x}));
    fields::tip(
        ui,
        id.with("sb"),
        d.r(71.0, 1.0, 23.0, 13.0),
        &t("pi-tip-space-before"),
    );
    super::glyphs::paint(
        ui.painter(),
        d.r(1.0, 16.0, 12.0, 11.0),
        Glyph::HScale,
        &theme,
        en,
    );
    if let Some(x) = hscale_field(ui, id.with("hs"), d, (v.hscale as f64 * 100.0).round(), en) {
        set_text(ui, st, tgt, json!({"horizontalScale": x / 100.0}));
    }
    let live = ui
        .data(|dd| dd.get_temp::<f64>(id.with("hs").with("arrow").with("val")))
        .filter(|_| en);
    preview_text(st, live, |x| json!({"horizontalScale": x / 100.0}));
    fields::tip(
        ui,
        id.with("hs"),
        d.r(16.0, 16.0, 26.0, 13.0),
        &t("pi-tip-horiz-scale"),
    );
    super::glyphs::paint(
        ui.painter(),
        d.r(56.0, 16.0, 12.0, 11.0),
        Glyph::SpaceAfter,
        &theme,
        en,
    );
    if let Some(x) = fields::number(
        ui,
        id.with("sa"),
        d.r(71.0, 16.0, 23.0, 13.0),
        Some(d.r(95.0, 16.0, 9.0, 13.0)),
        Some(v.space_after as f64),
        (-99.0, 100.0),
        0,
        en,
    ) {
        set_text(ui, st, tgt, json!({"spaceAfter": x}));
    }
    let live = fields::slider_live(ui.ctx(), id.with("sa")).filter(|_| en);
    preview_text(st, live, |x| json!({"spaceAfter": x}));
    fields::tip(
        ui,
        id.with("sa"),
        d.r(71.0, 16.0, 23.0, 13.0),
        &t("pi-tip-space-after"),
    );
    // Grundlinienverschiebung (14618) nur während der Textbearbeitung; am gewählten Textobjekt
    // liegt an derselben Stelle der Strich (Stift 14932 + Strichfarbe 14931, UX-A O7, a_302); im
    // Textwerkzeug ohne Text ist die Stelle leer (a_133)
    if editing {
        super::glyphs::paint(
            ui.painter(),
            d.r(110.0, 1.0, 12.0, 11.0),
            Glyph::Baseline,
            &theme,
            en,
        );
        if let Some(x) = fields::number(
            ui,
            id.with("bl"),
            d.r(124.0, 1.0, 27.0, 13.0),
            Some(d.r(152.0, 1.0, 9.0, 13.0)),
            Some(v.baseline as f64),
            (-99.0, 100.0),
            0,
            en,
        ) {
            set_text(ui, st, tgt, json!({"baselineShift": x}));
        }
    } else if tgt == Target::Selection {
        text_stroke_field(ui, st, d, id);
    }
    let mut aa_items = tl(&[
        "pi-aa-none",
        "pi-aa-crisp",
        "pi-aa-strong",
        "pi-aa-smooth",
        "",
        "pi-aa-custom",
    ]);
    aa_items[4] = "----".into();
    let aa_items = refs(&aa_items);
    // Zuordnung wie `text.setProperties {antialias}` (EB-1, PARITAET_A O6): „Kein Anti-Alias“ ist
    // die harte Füllkante, `TAAi` 0 = glatt
    if let Some(i) = fields::combo(
        ui,
        id.with("aa"),
        d.r(165.0, 1.0, 90.0, 13.0),
        &aa_items,
        Some(aa_index(v.aa)),
        en_obj,
    ) && let Some(name) = AA_NAMES.get(i).filter(|n| !n.is_empty())
    {
        match tgt {
            Target::Selection => st.run("text.setProperties", json!({"antialias": name})),
            Target::Tool => st.run("text.setToolDefaults", json!({"antialias": name})),
        }
    }
    fields::tip(
        ui,
        id.with("aa"),
        d.r(165.0, 1.0, 90.0, 13.0),
        &t("pi-tip-aa-level"),
    );
    if let Some(off) = v.on_path {
        fields::label_right(
            ui,
            d.r(102.0, 18.0, 44.0, 8.0),
            &t("pi-text-offset"),
            en_obj,
        );
        if let Some(x) = fields::number(
            ui,
            id.with("po"),
            d.r(148.0, 16.0, 14.0, 13.0),
            None,
            Some(off as f64),
            (-10000.0, 10000.0),
            0,
            en_obj,
        ) {
            st.run("pi.text.pathOffset", json!({"offset": x}));
        }
    }
    // 100 statt 92 DLU: „Autom. Unterschneidung“ ist 97 DLU breit (RAND-2a, N23)
    if let Some(b) = fields::checkbox(
        ui,
        id.with("ak"),
        d.r(165.0, 18.0, 100.0, 10.0),
        &t("pi-text-autokern"),
        v.auto_kern,
        en_obj,
    ) {
        match tgt {
            Target::Selection => st.run("pi.text.autoKern", json!({"on": b})),
            Target::Tool => st.run("text.setToolDefaults", json!({"autoKern": b})),
        }
    }
    fields::tip(
        ui,
        id.with("ak"),
        d.r(165.0, 18.0, 100.0, 10.0),
        &t("pi-tip-autokern"),
    );
}

/// Strich des Textobjekts (Stift + Farbfeld) in Zeile 2: Farbe wählen legt einen Strich an
/// (1-Pixel weich, wie das Strich-Teilfeld), „keine“ entfernt ihn.
/// Horizontalskala mit „%“ im Feld wie im Original („100%“, PARITAET_A O8), Objekt und Werkzeug:
/// getippt zählen nur die Ziffern („120“ und „120%“ → 120), begrenzt auf 50 … 300; der Pfeil ▾
/// öffnet den Popup-Schieber (beim Ziehen zeigt das Feld den Wert live mit).
fn hscale_field(ui: &mut Ui, id: Id, d: Dl, value: f64, en: bool) -> Option<f64> {
    const RANGE: (f64, f64) = (50.0, 300.0);
    let arrow = id.with("arrow");
    let live = ui.data(|dd| dd.get_temp::<f64>(arrow.with("val")));
    let shown = format!("{}%", fields::fmt_num(live.unwrap_or(value), 0));
    let mut out = fields::text(ui, id, d.r(16.0, 16.0, 26.0, 13.0), &shown, en)
        .and_then(|s| fields::parse_digits(&s))
        .map(|x| fields::clamp_range(x, RANGE));
    if let Some(x) =
        fields::slider_arrow(ui, arrow, d.r(43.0, 16.0, 9.0, 13.0), value, RANGE, 0, en)
    {
        out = Some(x);
    }
    out.filter(|x| (x - value).abs() > f64::EPSILON)
}

fn text_stroke_field(ui: &mut Ui, st: &mut AppState, d: Dl, id: Id) {
    let theme = Theme::current(ui.ctx());
    let attrs = shown_attrs(st);
    let en = attrs.is_some();
    let has = attrs.as_ref().is_some_and(|a| a.brush.is_some());
    let color = attrs.as_ref().filter(|_| has).map(|a| a.stroke_color);
    super::glyphs::paint(
        ui.painter(),
        d.r(121.0, 1.0, 12.0, 11.0),
        Glyph::Pencil,
        &theme,
        en,
    );
    match popups::color_button_with(
        ui,
        id.with("stroke"),
        d.r(135.0, 1.0, 14.0, 13.0),
        st,
        color,
        en,
        super::color_picker::Footer::Stroke {
            placement: None,
            enabled: en,
        },
    ) {
        Some(ColorChoice::Color(c)) => {
            let mut p = json!({"color": ui_commands::hex(c)});
            if !has {
                p["preset"] = json!("Bleistift/1-Pixel weich");
            }
            st.run("pi.stroke", p);
        }
        Some(ColorChoice::None) => st.run("pi.stroke", json!({"none": true})),
        Some(ColorChoice::Placement(k)) => st.run(
            "pi.stroke",
            json!({"placement": crate::inspector::color_picker::placement_name(k)}),
        ),
        Some(_) | None => {}
    }
    // live canvas preview of the pending alpha of the colour pop-up (FB-29)
    if en && let Some(c) = popups::color_button_pending(ui.ctx(), id.with("stroke")) {
        st.inspector_preview = InspectorPreview::for_selection(st, PreviewValue::StrokeColor(c));
    }
}

fn text_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    text_row1(ui, st, d, Target::Selection);
    fields::vsep(
        ui,
        d.r(257.0, 0.0, 1.0, 1.0).left(),
        r.top() + 1.0,
        r.bottom(),
    );
    opacity_panel(ui, st, d.offset(260.0, 0.0));
    effect_header(ui, st, d.offset(260.0, 14.0));
}

fn text_q4(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    let tool = selection(st).is_empty();
    text_row2(
        ui,
        st,
        d,
        if tool {
            Target::Tool
        } else {
            Target::Selection
        },
    );
    if !tool {
        effect_list(ui, st, d.r(260.0, 0.0, 105.0, 29.0));
    }
}

fn text_tool_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    text_row1(ui, st, d, Target::Tool);
    tool_opacity_panel(ui, st, d.offset(261.0, 0.0), true);
}

/// Q3 bei gewählter Bitmapmaske (RE spec 68 §4.2 rule 2): "Mask:" Alpha channel ("mask to path") /
/// Grayscale ("mask to image"), always usable; no opacity or blend mode.
fn mask_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    bitmap_mask_radios(ui, st, d, "pi-bmask");
    // Firepixel extension (FB-35): Invert below the mode radios
    mask_invert_box(ui, st, d.r(2.0, 28.0, 160.0, 12.0));
}

/// Mode of the mask being edited (owner of the selected mask): `Some(true)` = "mask to path"
/// (alpha channel / path outline), `Some(false)` = "mask to image" (grayscale).
pub fn selected_mask_mode(st: &AppState) -> Option<bool> {
    let ds = st.session.active()?;
    let id = ds.selection.elements.first()?;
    let owner = fp_engine::docops::mask_owner(&ds.doc, ds.page, *id)?;
    owner.element_mask().map(|m| m.to_path)
}

/// "Mask:" Alpha channel / Grayscale of a bitmap mask (inspector and the painting tools'
/// sub-panel, RE spec 68 §4.2 rule 2): shows the real mode and switches it (one History row
/// "Modify Mask"). Nothing when no mask is selected.
pub fn bitmap_mask_radios(ui: &mut Ui, st: &mut AppState, d: Dl, id: &str) {
    let Some(to_path) = selected_mask_mode(st) else {
        return;
    };
    fields::label(ui, d.r(2.0, 2.0, 29.0, 8.0), &t("pi-mask"), true);
    if fields::radio(
        ui,
        egui::Id::new((id, "alpha")),
        d.r(39.0, 2.0, 80.0, 10.0),
        &t("pi-mask-alpha"),
        to_path,
        true,
    ) && !to_path
    {
        st.run("modify.mask.options", json!({"mode": "path"}));
    }
    if fields::radio(
        ui,
        egui::Id::new((id, "gray")),
        d.r(39.0, 15.0, 80.0, 10.0),
        &t("pi-mask-gray"),
        !to_path,
        true,
    ) && to_path
    {
        st.run("modify.mask.options", json!({"mode": "image"}));
    }
}

/// Firepixel extension (FB-35, RE spec 68 section 6): "Invert" checkbox of the selected mask
/// (vector and bitmap masks); one History row "Modify Mask" via `modify.mask.options {invert}`.
fn mask_invert_box(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let on = st.session.active().and_then(|ds| {
        let id = ds.selection.elements.first()?;
        let owner = fp_engine::docops::mask_owner(&ds.doc, ds.page, *id)?;
        owner.element_mask().map(|m| m.invert)
    });
    let Some(on) = on else { return };
    if let Some(v) = fields::checkbox(
        ui,
        egui::Id::new("pi-mask-invert"),
        r,
        &t("pi-mask-invert"),
        on,
        true,
    ) {
        st.run("modify.mask.options", json!({ "invert": v }));
    }
}

/// Q6 bei gewählter Vektormaske (re 31 §5.3): „Maske:“ Pfadumriss / Graustufendarstellung und
/// „Füllung und Pinselstrich anzeigen“ des maskierten Elements.
fn mask_options_q6(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min + vec2(4.0, 0.0));
    let owner = st.session.active().and_then(|ds| {
        ds.selection
            .elements
            .first()
            .and_then(|id| fp_engine::docops::mask_owner(&ds.doc, ds.page, *id))
    });
    let Some(m) = owner.as_ref().and_then(|o| o.element_mask()) else {
        return;
    };
    fields::label(ui, d.r(2.0, 2.0, 40.0, 10.0), &t("pi-mask"), true);
    if fields::radio(
        ui,
        egui::Id::new("pi-mask-path"),
        d.r(44.0, 0.0, 110.0, 12.0),
        &t("pi-mask-path-outline"),
        m.to_path,
        true,
    ) && !m.to_path
    {
        st.run("modify.mask.options", json!({"mode": "path"}));
    }
    if fields::radio(
        ui,
        egui::Id::new("pi-mask-image"),
        d.r(44.0, 12.0, 110.0, 12.0),
        &t("pi-mask-grayscale"),
        !m.to_path,
        true,
    ) && m.to_path
    {
        st.run("modify.mask.options", json!({"mode": "image"}));
    }
    if let Some(v) = fields::checkbox(
        ui,
        egui::Id::new("pi-mask-show"),
        d.r(2.0, 24.0, 160.0, 12.0),
        &t("pi-mask-show-attrs"),
        m.show_attrs,
        m.to_path,
    ) {
        st.run("modify.mask.options", json!({"showAttrs": v}));
    }
    mask_invert_box(ui, st, d.r(2.0, 36.0, 160.0, 12.0));
}

/// Für Tests: Text einer Elementart im Inspektor (Q1).
pub fn type_label(e: &Element) -> String {
    let e = super::layouts::insp_elem(e);
    match &e.kind {
        ElementKind::Bitmap(_) => t("pi-type-bitmap"),
        ElementKind::Path(p) if p.primitive.is_some() => t("pi-type-rectangle"),
        ElementKind::Path(_) => t("pi-type-path"),
        ElementKind::Text(_) => t("pi-type-text"),
        ElementKind::Group(g) => fp_i18n::tr!("pi-type-group", n = g.children.len()),
        ElementKind::CompoundShape(_) => t("pi-type-compound"),
        ElementKind::Instance(_) => t("pi-type-symbol"),
        ElementKind::WebObject(w) if w.slice.is_some() => t("pi-type-slice"),
        ElementKind::WebObject(_) => t("pi-type-hotspot"),
        ElementKind::SubLayer(_) => t("pi-type-sublayer"),
        ElementKind::Raw(_) => t("pi-type-object"),
    }
}

/// Bildgröße als Text für Q1-Miniaturen der Mehrfachauswahl.
pub fn swatch_color(c: Argb) -> Color32 {
    c32(c)
}

/// Liste der Verlaufsformen in der aktuellen Sprache (Export für Dialoge).
pub fn gradient_names() -> Vec<String> {
    let _ = GRADIENT_SHAPES;
    texts::gradient_labels()
}

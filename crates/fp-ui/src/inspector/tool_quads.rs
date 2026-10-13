//! Werkzeug-Quadranten (11 §2.3.2 Zeilen 2–18, 43–72): Optionen je Werkzeug, Lage der Felder in
//! DLU wie in `dialoge.json`. Werte, die fp-engine in `ToolOptions` führt, werden dort gesetzt
//! (Pinselgröße, Toleranzen, Ränder, Auswahlrahmen-Stil, Polygon, Rechteckrundung, Pipette …);
//! die übrigen stehen in den Inspektor-Werkzeugattributen ([`super::obj_quads::ToolAttrs`]).

use super::fields::{self, Dl};
use super::glyphs::Glyph;
use super::obj_quads::{self, edit_tool_attrs, tool_attrs, Target};
use super::texts::{refs, tl, t as tx};
use super::QuadrantRegistry;
use crate::app_state::{Action, AppState};
use crate::theme::Theme;
use egui::{Id, Rect, Ui};
use fp_engine::tools::{EdgeMode, MarqueeStyle};
use fp_engine::ToolId;

pub(super) fn register(r: &mut QuadrantRegistry) {
    r.register(14068, rect_tool_q3);
    r.register(14069, rect_tool_q4);
    r.register(14065, pen_q4);
    r.register(14058, line_q3);
    r.register(14059, line_q4);
    r.register(14111, polygon_q3);
    r.register(14113, polygon_q4);
    r.register(14099, path_tool_q3);
    r.register(14106, line_q4);
    r.register(14050, brush_q3);
    r.register(14051, brush_q4);
    r.register(14052, eraser_q3);
    r.register(14053, eyedropper_q3);
    r.register(14057, lasso_q3);
    r.register(14060, wand_q3);
    r.register(14061, marquee_q3);
    r.register(14107, fill_tool_q3);
    r.register(14108, fill_tool_q4);
    r.register(14109, fill_tool_q3);
    r.register(14110, fill_tool_q4);
    r.register(14070, stamp_q3);
    r.register(14067, pencil_q3);
    r.register(14136, blur_q3);
    r.register(14138, blur_q3);
    r.register(14968, dodge_q3);
    r.register(14978, dodge_q3);
    r.register(15006, smudge_q3);
    r.register(15082, red_eye_q3);
    r.register(15091, replace_color_q3);
    r.register(14054, freeform_q3);
    r.register(14055, reshape_q3);
    r.register(14090, scrub_q3);
    r.register(14056, url_tool_q3);
    r.register(14119, mask_sub_q4);
}

fn edge_parts(e: EdgeMode) -> (usize, u32) {
    match e {
        EdgeMode::Hard => (0, 0),
        EdgeMode::AntiAlias => (1, 0),
        EdgeMode::Feather(f) => (2, f),
    }
}

fn edge_from(i: usize, f: u32) -> EdgeMode {
    match i {
        0 => EdgeMode::Hard,
        1 => EdgeMode::AntiAlias,
        _ => EdgeMode::Feather(f.max(1)),
    }
}

const EDGE3: [&str; 3] = ["pi-edge-hard", "pi-edge-antialias", "pi-edge-feather"];

/// Rand-Kombifeld + Breite für Auswahlwerkzeuge; liefert die neue Einstellung.
fn edge_fields(ui: &mut Ui, id: &str, combo: Rect, edit: Rect, arrow: Rect, e: EdgeMode) -> Option<EdgeMode> {
    let (i, f) = edge_parts(e);
    let mut out = None;
    let items = tl(&EDGE3);
    if let Some(n) = fields::combo(ui, (id, "edge"), combo, &refs(&items), Some(i), true) {
        out = Some(edge_from(n, if f == 0 { 10 } else { f }));
    }
    if let Some(v) = fields::number(ui, (id, "feather"), edit, Some(arrow), Some(f as f64), (0.0, 100.0), 0, i == 2) {
        out = Some(EdgeMode::Feather(v.max(1.0) as u32));
    }
    out
}

// ---------------------------------------------------------------------- Vektorwerkzeuge

fn rect_tool_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    obj_quads::fill_panel(ui, st, d.offset(1.0, 0.0), Target::Tool, true);
    obj_quads::column_seps(ui, d, r);
    obj_quads::stroke_panel(ui, st, d.offset(125.0, 0.0), Target::Tool, true);
    obj_quads::tool_opacity_panel(ui, st, d.offset(261.0, 0.0), true);
}

fn rect_tool_q4(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    obj_quads::column_seps(ui, d, r);
    obj_quads::fill_texture_panel(ui, st, d.offset(1.0, 0.0), Target::Tool, true);
    obj_quads::stroke_texture_panel(ui, st, d.offset(125.0, 0.0), Target::Tool, true);
    // Keine Eckrundung im Werkzeug-Inspektor: Im Original sind Feld 14795 und Beschriftung 15079
    // des Quadranten unsichtbar (PARITAET_A W23); die Rundung des gewählten Rechtecks bleibt (O1).
}

fn pen_q4(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    obj_quads::column_seps(ui, d, r);
    obj_quads::fill_texture_panel(ui, st, d.offset(1.0, 0.0), Target::Tool, true);
    obj_quads::stroke_texture_panel(ui, st, d.offset(125.0, 0.0), Target::Tool, true);
    obj_quads::stroke_edit_row(ui, st, d.offset(125.0, 14.0), Target::Tool, true);
}

fn line_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    obj_quads::stroke_panel(ui, st, d.offset(125.0, 0.0), Target::Tool, true);
    obj_quads::tool_opacity_panel(ui, st, d.offset(261.0, 0.0), true);
}

fn line_q4(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    obj_quads::stroke_texture_panel(ui, st, d.offset(125.0, 0.0), Target::Tool, true);
    obj_quads::stroke_edit_row(ui, st, d.offset(125.0, 14.0), Target::Tool, true);
}

fn polygon_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    rect_tool_q3(ui, st, r);
    fields::label_right(ui, d.r(256.0, 17.0, 31.0, 8.0), &tx("pi-shape"), true);
    let star = st.session.tools.options.polygon_star;
    if let Some(i) = fields::combo(ui, "pi-poly-form", d.r(289.0, 15.0, 69.0, 13.0), &refs(&tl(&["pi-poly-star", "pi-poly-polygon"])), Some(if star { 0 } else { 1 }), true) {
        st.session.tools.options.polygon_star = i == 0;
    }
}

fn polygon_q4(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    obj_quads::column_seps(ui, d, r);
    obj_quads::fill_texture_panel(ui, st, d.offset(1.0, 0.0), Target::Tool, true);
    obj_quads::stroke_texture_panel(ui, st, d.offset(125.0, 0.0), Target::Tool, true);
    let o = st.session.tools.options.clone();
    fields::label_right(ui, d.r(256.0, 2.0, 30.0, 8.0), &tx("pi-poly-sides"), true);
    if let Some(v) = fields::number(ui, "pi-poly-sides", d.r(288.0, 0.0, 19.0, 13.0), Some(d.r(308.0, 0.0, 9.0, 13.0)), Some(o.polygon_sides as f64), (3.0, 360.0), 0, true) {
        st.session.tools.options.polygon_sides = v as u32;
    }
    // Winkel und „Autom.“ sind im Original bei beiden Formen bedienbar, auch bei Form = Polygon
    // und mit gesetztem „Autom.“ (Winkel 38, PARITAET_A W19, a_131)
    fields::label_right(ui, d.r(255.0, 17.0, 31.0, 8.0), &tx("pi-poly-angle"), true);
    let auto = o.polygon_spikiness.is_none();
    let ang = o.polygon_spikiness.map(|s| (s * 100.0).round() as f64).unwrap_or(38.0);
    if let Some(v) = fields::number(ui, "pi-poly-angle", d.r(288.0, 14.0, 19.0, 13.0), Some(d.r(308.0, 15.0, 9.0, 13.0)), Some(ang), (0.0, 100.0), 0, true) {
        st.session.tools.options.polygon_spikiness = Some((v / 100.0) as f32);
    }
    if let Some(b) = fields::checkbox(ui, "pi-poly-auto", d.r(320.0, 16.0, 40.0, 10.0), &tx("pi-poly-auto"), auto, true) {
        st.session.tools.options.polygon_spikiness = if b { None } else { Some(0.38) };
    }
}

fn path_tool_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    fields::label_right(ui, d.r(36.0, 2.0, 51.0, 8.0), &tx("pi-path-precision"), true);
    let p = st.session.tools.options.path_precision as f64;
    if let Some(v) = fields::number(ui, "pi-path-prec", d.r(91.0, 0.0, 19.0, 13.0), Some(d.r(111.0, 0.0, 9.0, 13.0)), Some(p), (1.0, 10.0), 0, true) {
        st.session.tools.options.path_precision = v as u32;
    }
    obj_quads::stroke_panel(ui, st, d.offset(125.0, 0.0), Target::Tool, true);
    obj_quads::tool_opacity_panel(ui, st, d.offset(261.0, 0.0), true);
}

// ---------------------------------------------------------------------- Bitmap-Werkzeuge

fn keep_transparency(ui: &mut Ui, r: Rect, key: &str) {
    let ta = tool_attrs(ui.ctx());
    if let Some(b) = fields::checkbox(ui, (key, "kt"), r, &tx("pi-keep-transparency"), ta.keep_transparency, true) {
        edit_tool_attrs(ui.ctx(), |a| a.keep_transparency = b);
    }
}

fn brush_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    obj_quads::stroke_panel(ui, st, d.offset(125.0, 0.0), Target::Tool, true);
    obj_quads::tool_opacity_panel(ui, st, d.offset(261.0, 0.0), true);
    keep_transparency(ui, d.r(260.0, 16.0, 102.0, 10.0), "brush");
}

fn brush_q4(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    obj_quads::stroke_texture_panel(ui, st, d.offset(125.0, 0.0), Target::Tool, true);
    obj_quads::stroke_edit_row(ui, st, d.offset(125.0, 14.0), Target::Tool, true);
    mask_sub(ui, st, d.offset(258.0, 0.0));
}

#[allow(clippy::too_many_arguments)]
fn size_edge_shape(ui: &mut Ui, d: Dl, key: &str, size: f64, edge: f64, round: bool, form_x: f32, size_range: (f64, f64)) -> (Option<f64>, Option<f64>, Option<bool>) {
    let t = Theme::current(ui.ctx());
    fields::label_right(ui, d.r(16.0, 2.0, 40.0, 8.0), &tx("pi-size"), true);
    let s = fields::number(ui, (key, "size"), d.r(58.0, 0.0, 21.0, 13.0), Some(d.r(80.0, 0.0, 9.0, 13.0)), Some(size), size_range, 0, true);
    fields::label_right(ui, d.r(4.0, 17.0, 35.0, 8.0), &tx("pi-edge"), true);
    let pv = d.r(41.0, 15.0, 15.0, 13.0);
    ui.painter().rect_filled(pv, 0.0, t.colors.field_bg);
    ui.painter().circle_filled(pv.center(), (pv.height() / 2.0 - 2.0) * (1.0 - edge as f32 / 200.0), t.colors.icon_fg);
    let e = fields::number(ui, (key, "edge"), d.r(58.0, 15.0, 21.0, 13.0), Some(d.r(80.0, 15.0, 9.0, 13.0)), Some(edge), (0.0, 100.0), 0, true);
    fields::label_right(ui, d.r(form_x - 42.0, 2.0, 40.0, 8.0), &tx("pi-shape"), true);
    let mut shape = None;
    if fields::toggle(ui, (key, "round"), d.r(form_x, 0.0, 16.0, 12.0), Glyph::RoundTip, round, true, &tx("pi-tip-round")) {
        shape = Some(true);
    }
    if fields::toggle(ui, (key, "square"), d.r(form_x + 16.0, 0.0, 16.0, 12.0), Glyph::SquareTip, !round, true, &tx("pi-tip-square")) {
        shape = Some(false);
    }
    (s, e, shape)
}

fn eraser_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    let o = st.session.tools.options.clone();
    // Rand aus `EraserToolSoftEdge` (Werksvorgabe 2, UX-A W3)
    let (s, e, sh) = size_edge_shape(ui, d, "eraser", o.eraser_size as f64, o.eraser_edge as f64, o.eraser_round, 138.0, (1.0, 100.0));
    if let Some(v) = s {
        st.session.tools.options.eraser_size = v as f32;
    }
    if let Some(v) = e {
        st.session.tools.options.eraser_edge = v.clamp(0.0, 100.0) as u32;
        st.session.tools.options.brush_soft = v > 0.0;
    }
    if let Some(v) = sh {
        st.session.tools.options.eraser_round = v;
    }
    let t = Theme::current(ui.ctx());
    super::glyphs::paint(ui.painter(), d.r(259.0, 1.0, 9.0, 11.0), Glyph::Opacity, &t, true);
    if let Some(v) = fields::number(ui, "eraser-op", d.r(270.0, 0.0, 19.0, 13.0), Some(d.r(290.0, 0.0, 9.0, 13.0)), Some(o.eraser_opacity as f64), (0.0, 100.0), 0, true) {
        st.session.tools.options.eraser_opacity = v as u8;
    }
}

fn eyedropper_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    fields::label(ui, d.r(2.0, 3.0, 37.0, 8.0), &tx("pi-eyedrop-sample"), true);
    let cur = match st.session.tools.options.eyedropper_samples {
        3 => 1,
        5 => 2,
        _ => 0,
    };
    if let Some(i) = fields::combo(ui, "pi-eyedrop", d.r(41.0, 1.0, 77.0, 13.0), &refs(&tl(&["pi-eyedrop-1", "pi-eyedrop-3", "pi-eyedrop-5"])), Some(cur), true) {
        st.session.tools.options.eyedropper_samples = [1, 3, 5][i];
        st.session.prefs.set("EyeDropperExtraSamples", fp_engine::PrefValue::Int(i as i64));
    }
    fields::label(ui, d.r(120.0, 3.0, 54.0, 8.0), &tx("pi-color-format"), true);
    let ta = tool_attrs(ui.ctx());
    if let Some(i) = fields::combo(ui, "pi-eyefmt", d.r(177.0, 1.0, 77.0, 13.0), &["HEX", "RGBA"], Some(ta.color_format), true) {
        edit_tool_attrs(ui.ctx(), |a| a.color_format = i);
    }
}

fn live_marquee(ui: &mut Ui, r: Rect, key: &str) {
    let ta = tool_attrs(ui.ctx());
    if let Some(b) = fields::checkbox(ui, (key, "live"), r, &tx("pi-live-marquee"), ta.live_marquee, true) {
        edit_tool_attrs(ui.ctx(), |a| a.live_marquee = b);
    }
}

fn lasso_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    fields::label(ui, d.r(4.0, 2.0, 26.0, 8.0), &tx("pi-edge"), true);
    if let Some(e) = edge_fields(ui, "lasso", d.r(31.0, 0.0, 68.0, 13.0), d.r(101.0, 0.0, 22.0, 13.0), d.r(124.0, 0.0, 9.0, 13.0), st.session.tools.options.lasso_edge) {
        st.session.tools.options.lasso_edge = e;
    }
    live_marquee(ui, d.r(11.0, 17.0, 94.0, 10.0), "lasso");
}

fn wand_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    fields::label_right(ui, d.r(4.0, 2.0, 40.0, 8.0), &tx("pi-tolerance"), true);
    if let Some(v) = fields::number(ui, "wand-tol", d.r(46.0, 0.0, 21.0, 13.0), Some(d.r(68.0, 0.0, 9.0, 13.0)), Some(st.session.tools.options.wand_tolerance as f64), (0.0, 255.0), 0, true) {
        st.session.tools.options.wand_tolerance = v as u32;
    }
    fields::label_right(ui, d.r(83.0, 2.0, 27.0, 8.0), &tx("pi-edge"), true);
    if let Some(e) = edge_fields(ui, "wand", d.r(112.0, 0.0, 65.0, 13.0), d.r(179.0, 0.0, 21.0, 13.0), d.r(202.0, 0.0, 9.0, 13.0), st.session.tools.options.wand_edge) {
        st.session.tools.options.wand_edge = e;
    }
    live_marquee(ui, d.r(10.0, 17.0, 83.0, 10.0), "wand");
}

/// Symbol vor der festen Breite des Auswahlrahmens (Original: ↔, Symbol statt Text).
pub const MARQUEE_W_SYMBOL: &str = "\u{2194}";
/// Symbol vor der festen Höhe des Auswahlrahmens (Original: ↕).
pub const MARQUEE_H_SYMBOL: &str = "\u{2195}";

/// Voreinstellungen der festen Auswahlrahmen-Werte: Verhältnis X/Y, Größe B/H.
const MARQUEE_PREFS: [&str; 4] = ["MarqueeToolModeRatioX", "MarqueeToolModeRatioY", "MarqueeToolModeWidthX", "MarqueeToolModeWidthY"];

fn marquee_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    let style = st.session.tools.options.marquee_style;
    fields::label(ui, d.r(2.0, 2.0, 29.0, 8.0), &tx("pi-marquee-style"), true);
    // bei „Normal“ zeigt das Original 0/0 in grauen Feldern (PARITAET_A W24)
    let (si, w, h) = match style {
        MarqueeStyle::Normal => (0, 0.0, 0.0),
        MarqueeStyle::FixedRatio(a, b) => (1, a, b),
        MarqueeStyle::FixedSize(a, b) => (2, a, b),
    };
    if let Some(i) = fields::combo(ui, "marq-style", d.r(33.0, 0.0, 92.0, 13.0), &refs(&tl(&["pi-marquee-normal", "pi-marquee-fixed-ratio", "pi-marquee-fixed-size"])), Some(si), true) {
        // Werte wie im Original aus den Voreinstellungen (Werksvorgabe Verhältnis 1:2, Größe 64 × 64)
        let p = &st.session.prefs;
        st.session.tools.options.marquee_style = match i {
            1 => MarqueeStyle::FixedRatio(p.get_int(MARQUEE_PREFS[0], 1) as f32, p.get_int(MARQUEE_PREFS[1], 2) as f32),
            2 => MarqueeStyle::FixedSize(p.get_int(MARQUEE_PREFS[2], 64) as f32, p.get_int(MARQUEE_PREFS[3], 64) as f32),
            _ => MarqueeStyle::Normal,
        };
    }
    let en = si != 0;
    // Breite/Höhe wie im Original mit den Symbolen ↔/↕ statt Buchstaben (immer schwarz, auch
    // wenn die Felder grau sind)
    fields::label_name(ui, d.r(21.0, 16.0, 11.0, 8.0), MARQUEE_W_SYMBOL, true);
    let nw = fields::number(ui, "marq-w", d.r(33.0, 14.0, 22.0, 13.0), None, Some(w as f64), (1.0, 30000.0), 0, en);
    fields::label_name(ui, d.r(61.0, 16.0, 10.0, 8.0), MARQUEE_H_SYMBOL, true);
    let nh = fields::number(ui, "marq-h", d.r(72.0, 14.0, 22.0, 13.0), None, Some(h as f64), (1.0, 30000.0), 0, en);
    if nw.is_some() || nh.is_some() {
        let (a, b) = (nw.unwrap_or(w as f64) as f32, nh.unwrap_or(h as f64) as f32);
        st.session.tools.options.marquee_style = if si == 1 { MarqueeStyle::FixedRatio(a, b) } else { MarqueeStyle::FixedSize(a, b) };
        // gemerkt je Stil, damit ein Wechsel hin und zurück die Werte behält
        let k = if si == 1 { 0 } else { 2 };
        st.session.prefs.set(MARQUEE_PREFS[k], fp_engine::prefs::PrefValue::Int(a as i64));
        st.session.prefs.set(MARQUEE_PREFS[k + 1], fp_engine::prefs::PrefValue::Int(b as i64));
    }
    fields::label_right(ui, d.r(129.0, 2.0, 28.0, 8.0), &tx("pi-edge"), true);
    if let Some(e) = edge_fields(ui, "marq", d.r(159.0, 0.0, 71.0, 13.0), d.r(233.0, 0.0, 19.0, 13.0), d.r(253.0, 0.0, 9.0, 13.0), st.session.tools.options.marquee_edge) {
        st.session.tools.options.marquee_edge = e;
    }
    live_marquee(ui, d.r(139.0, 17.0, 93.0, 10.0), "marq");
}

/// Voreinstellung „Ausgewählten Bereich füllen“ je Werkzeug: Farbeimer `FillToolFillsSelection`
/// (Werksvorgabe aus), Verlauf `GradientToolFillsSelection` (Werksvorgabe an, UX-A W25, a_146).
fn fill_selection_pref(tool: ToolId) -> Option<&'static str> {
    match tool {
        ToolId::PaintBucket => Some("FillToolFillsSelection"),
        ToolId::Gradient => Some("GradientToolFillsSelection"),
        _ => None,
    }
}

/// Voreinstellung der Farbtoleranz je Werkzeug (`FillToolTolerance` bzw. `GradientToolTolerance`).
fn tolerance_pref(tool: ToolId) -> Option<&'static str> {
    match tool {
        ToolId::PaintBucket => Some("FillToolTolerance"),
        ToolId::Gradient => Some("GradientToolTolerance"),
        _ => None,
    }
}

/// Beim Wechsel zu Farbeimer bzw. Verlauf gelten dessen eigene Voreinstellungen „Auswahl füllen“
/// und Toleranz (in fp-engine getrennte Felder `fill_selection`/`gradient_fill_selection` und
/// `fill_tolerance`/`gradient_tolerance`; das Original merkt beide Werkzeuge getrennt).
pub fn sync_fill_selection(ctx: &egui::Context, st: &mut AppState) {
    let key = Id::new("fv1-fill-selection-tool");
    let tool = st.session.tools.active;
    if ctx.data(|d| d.get_temp::<ToolId>(key)) == Some(tool) {
        return;
    }
    ctx.data_mut(|d| d.insert_temp(key, tool));
    if let (Some(k), Some(kt)) = (fill_selection_pref(tool), tolerance_pref(tool)) {
        let def = tool == ToolId::Gradient;
        let on = st.session.prefs.get_bool(k, def);
        let tol = st.session.prefs.get_int(kt, 32).clamp(0, 255) as u32;
        let o = &mut st.session.tools.options;
        o.set_fills_selection(tool, on);
        o.set_tolerance_of(tool, tol);
    }
}

fn fill_tool_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    let tool = st.session.tools.active;
    obj_quads::fill_panel(ui, st, d.offset(1.0, 0.0), Target::Tool, true);
    fields::vsep(ui, d.r(122.0, 0.0, 1.0, 1.0).left(), r.top() + 1.0, r.bottom());
    // Toleranz gilt nur ohne „Auswahl füllen“ (Original: dann grau); beide je Werkzeug gemerkt
    let fills_sel = st.session.tools.options.fills_selection(tool);
    let tol_en = !fills_sel;
    fields::label_right(ui, d.r(122.0, 2.0, 43.0, 8.0), &tx("pi-tolerance"), tol_en);
    if let Some(v) = fields::number(ui, "fill-tol", d.r(167.0, 0.0, 21.0, 13.0), Some(d.r(189.0, 0.0, 9.0, 13.0)), Some(st.session.tools.options.tolerance_of(tool) as f64), (0.0, 255.0), 0, tol_en) {
        st.session.tools.options.set_tolerance_of(tool, v as u32);
        if let Some(k) = tolerance_pref(tool) {
            st.session.prefs.set(k, fp_engine::prefs::PrefValue::Int(v as i64));
        }
    }
    if let Some(b) = fields::checkbox(ui, "fill-sel", d.r(167.0, 16.0, 65.0, 10.0), &tx("pi-fill-selection"), fills_sel, true) {
        st.session.tools.options.set_fills_selection(tool, b);
        // je Werkzeug gemerkt (Farbeimer und Verlauf haben eigene Vorgaben)
        if let Some(k) = fill_selection_pref(tool) {
            st.session.prefs.set(k, fp_engine::prefs::PrefValue::Bool(b));
        }
    }
    obj_quads::tool_opacity_panel(ui, st, d.offset(261.0, 0.0), true);
    keep_transparency(ui, d.r(260.0, 16.0, 102.0, 10.0), "fill");
}

fn fill_tool_q4(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    fields::vsep(ui, d.r(122.0, 0.0, 1.0, 1.0).left(), r.top() + 1.0, r.bottom());
    obj_quads::fill_texture_panel(ui, st, d.offset(1.0, 0.0), Target::Tool, true);
    mask_sub(ui, st, d.offset(254.0, 0.0));
}

fn stamp_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    let ta = tool_attrs(ui.ctx());
    let (s, e, _) = size_edge_shape_noform(ui, d, "stamp", ta.retouch_size as f64, ta.retouch_edge as f64);
    if s.is_some() || e.is_some() {
        edit_tool_attrs(ui.ctx(), |a| {
            if let Some(v) = s {
                a.retouch_size = v as i32;
            }
            if let Some(v) = e {
                a.retouch_edge = v as i32;
            }
        });
    }
    if let Some(b) = fields::checkbox(ui, "stamp-al", d.r(131.0, 1.0, 119.0, 10.0), &tx("pi-stamp-aligned"), ta.source_aligned, true) {
        edit_tool_attrs(ui.ctx(), |a| a.source_aligned = b);
    }
    if let Some(b) = fields::checkbox(ui, "stamp-doc", d.r(131.0, 16.0, 119.0, 10.0), &tx("pi-whole-doc"), ta.whole_doc, true) {
        edit_tool_attrs(ui.ctx(), |a| a.whole_doc = b);
    }
    obj_quads::tool_opacity_panel(ui, st, d.offset(261.0, 0.0), true);
}

fn size_edge_shape_noform(ui: &mut Ui, d: Dl, key: &str, size: f64, edge: f64) -> (Option<f64>, Option<f64>, ()) {
    let t = Theme::current(ui.ctx());
    fields::label_right(ui, d.r(16.0, 2.0, 40.0, 8.0), &tx("pi-size"), true);
    let s = fields::number(ui, (key, "size"), d.r(58.0, 0.0, 21.0, 13.0), Some(d.r(80.0, 0.0, 9.0, 13.0)), Some(size), (1.0, 100.0), 0, true);
    fields::label_right(ui, d.r(4.0, 17.0, 35.0, 8.0), &tx("pi-edge"), true);
    let pv = d.r(41.0, 15.0, 15.0, 13.0);
    ui.painter().rect_filled(pv, 0.0, t.colors.field_bg);
    ui.painter().circle_filled(pv.center(), (pv.height() / 2.0 - 2.0) * (1.0 - edge as f32 / 200.0), t.colors.icon_fg);
    let e = fields::number(ui, (key, "edge"), d.r(58.0, 15.0, 21.0, 13.0), Some(d.r(80.0, 15.0, 9.0, 13.0)), Some(edge), (0.0, 100.0), 0, true);
    (s, e, ())
}

fn pencil_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    let t = Theme::current(ui.ctx());
    super::glyphs::paint(ui.painter(), d.r(0.0, 0.0, 12.0, 11.0), Glyph::Pencil, &t, true);
    let c = st.session.tools.stroke_color;
    match super::popups::color_button(ui, "pencil-color", d.r(15.0, 0.0, 14.0, 13.0), st, (c.a() > 0).then_some(c), true, true, true) {
        Some(super::popups::ColorChoice::Color(col)) => st.push(Action::SetColors { stroke: Some(col), fill: None }),
        Some(super::popups::ColorChoice::None) => st.push(Action::SetColors { stroke: Some(fp_color::Argb(0x00ff_ffff)), fill: None }),
        Some(super::popups::ColorChoice::Pipette) => {
            let ctx = ui.ctx().clone();
            super::popups::start_pick(&ctx, st, "color.set", serde_json::json!({}), "stroke");
        }
        None => {}
    }
    if let Some(b) = fields::checkbox(ui, "pencil-aa", d.r(43.0, 2.0, 90.0, 10.0), &tx("pi-antialias"), st.session.tools.options.pencil_antialias, true) {
        st.session.tools.options.pencil_antialias = b;
    }
    let ta = tool_attrs(ui.ctx());
    if let Some(b) = fields::checkbox(ui, "pencil-ae", d.r(43.0, 14.0, 90.0, 10.0), &tx("pi-auto-erase"), ta.auto_erase, true) {
        edit_tool_attrs(ui.ctx(), |a| a.auto_erase = b);
    }
    keep_transparency(ui, d.r(137.0, 2.0, 107.0, 10.0), "pencil");
    obj_quads::tool_opacity_panel(ui, st, d.offset(261.0, 0.0), true);
}

fn blur_q3(ui: &mut Ui, _st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    let ta = tool_attrs(ui.ctx());
    let (s, e, sh) = size_edge_shape(ui, d, "blur", ta.retouch_size as f64, ta.retouch_edge as f64, ta.retouch_round, 153.0, (1.0, 100.0));
    fields::label_right(ui, d.r(99.0, 17.0, 52.0, 8.0), &tx("pi-intensity"), true);
    let i = fields::number(ui, "blur-int", d.r(153.0, 15.0, 21.0, 13.0), Some(d.r(176.0, 15.0, 9.0, 13.0)), Some(ta.intensity as f64), (0.0, 100.0), 0, true);
    edit_retouch(ui, s, e, sh, |a| {
        if let Some(v) = i {
            a.intensity = v as i32;
        }
    });
}

fn edit_retouch(ui: &Ui, s: Option<f64>, e: Option<f64>, sh: Option<bool>, extra: impl FnOnce(&mut obj_quads::ToolAttrs)) {
    edit_tool_attrs(ui.ctx(), |a| {
        if let Some(v) = s {
            a.retouch_size = v as i32;
        }
        if let Some(v) = e {
            a.retouch_edge = v as i32;
        }
        if let Some(v) = sh {
            a.retouch_round = v;
        }
        extra(a);
    });
}

fn dodge_q3(ui: &mut Ui, _st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    let ta = tool_attrs(ui.ctx());
    let (s, e, sh) = size_edge_shape(ui, d, "dodge", ta.retouch_size as f64, ta.retouch_edge as f64, ta.retouch_round, 153.0, (1.0, 100.0));
    fields::label_right(ui, d.r(110.0, 17.0, 40.0, 8.0), &tx("pi-range"), true);
    let rg = fields::combo(ui, "dodge-range", d.r(153.0, 15.0, 69.0, 13.0), &refs(&tl(&["pi-range-shadows", "pi-range-midtones", "pi-range-highlights"])), Some(ta.range), true);
    fields::label_right(ui, d.r(215.0, 2.0, 52.0, 10.0), &tx("pi-exposure"), true);
    let ex = fields::number(ui, "dodge-exp", d.r(269.0, 0.0, 21.0, 13.0), Some(d.r(291.0, 0.0, 9.0, 13.0)), Some(ta.exposure as f64), (0.0, 100.0), 0, true);
    edit_retouch(ui, s, e, sh, |a| {
        if let Some(v) = rg {
            a.range = v;
        }
        if let Some(v) = ex {
            a.exposure = v as i32;
        }
    });
}

fn smudge_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    let ta = tool_attrs(ui.ctx());
    let (s, e, sh) = size_edge_shape(ui, d, "smudge", ta.retouch_size as f64, ta.retouch_edge as f64, ta.retouch_round, 155.0, (1.0, 100.0));
    fields::label_right(ui, d.r(100.0, 17.0, 50.0, 8.0), &tx("pi-pressure"), true);
    let pr = fields::number(ui, "smudge-pr", d.r(153.0, 15.0, 21.0, 13.0), Some(d.r(175.0, 15.0, 9.0, 13.0)), Some(ta.strength as f64), (0.0, 100.0), 0, true);
    let c = st.session.tools.fill_color;
    if let Some(super::popups::ColorChoice::Color(col)) = super::popups::color_button(ui, "smudge-col", d.r(237.0, 0.0, 15.0, 13.0), st, Some(c), ta.smudge_color, false, false) {
        st.push(Action::SetColors { stroke: None, fill: Some(col) });
    }
    let sc = fields::checkbox(ui, "smudge-uc", d.r(255.0, 1.0, 84.0, 10.0), &tx("pi-smudge-color"), ta.smudge_color, true);
    let wd = fields::checkbox(ui, "smudge-doc", d.r(255.0, 16.0, 104.0, 10.0), &tx("pi-whole-doc"), ta.whole_doc, true);
    edit_retouch(ui, s, e, sh, |a| {
        if let Some(v) = pr {
            a.strength = v as i32;
        }
        if let Some(v) = sc {
            a.smudge_color = v;
        }
        if let Some(v) = wd {
            a.whole_doc = v;
        }
    });
}

fn red_eye_q3(ui: &mut Ui, _st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    let ta = tool_attrs(ui.ctx());
    fields::label(ui, d.r(0.0, 2.0, 50.0, 8.0), &tx("pi-tolerance"), true);
    let t = fields::number(ui, "redeye-tol", d.r(52.0, 0.0, 21.0, 13.0), Some(d.r(74.0, 0.0, 9.0, 13.0)), Some(ta.tolerance as f64), (0.0, 100.0), 0, true);
    fields::label(ui, d.r(0.0, 18.0, 50.0, 8.0), &tx("pi-strength"), true);
    let s = fields::number(ui, "redeye-str", d.r(52.0, 15.0, 21.0, 13.0), Some(d.r(74.0, 15.0, 9.0, 13.0)), Some(ta.strength as f64), (0.0, 100.0), 0, true);
    edit_tool_attrs(ui.ctx(), |a| {
        if let Some(v) = t {
            a.tolerance = v as i32;
        }
        if let Some(v) = s {
            a.strength = v as i32;
        }
    });
}

const REPLACE_SRC_PREF: &str = "ReplaceColorToolSrcColor";
const REPLACE_DST_PREF: &str = "ReplaceColorToolDestColor";

/// „Von“- und „in“-Farbe von „Farbe ersetzen“ aus den Voreinstellungen (Werksvorgabe #00FF00 bzw.
/// #FF0000, PARITAET_A W8).
pub fn replace_colors(p: &fp_engine::prefs::Preferences) -> (fp_color::Argb, fp_color::Argb) {
    (p.get_color(REPLACE_SRC_PREF, fp_color::Argb(0xff00_ff00)), p.get_color(REPLACE_DST_PREF, fp_color::Argb(0xffff_0000)))
}

/// Farbe im Format der Voreinstellung (`#RRGGBB`, wie die Werksvorgabe).
fn ui_hex(c: fp_color::Argb) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b())
}

fn replace_color_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    let ta = tool_attrs(ui.ctx());
    fields::label_right(ui, d.r(16.0, 2.0, 40.0, 8.0), &tx("pi-size"), true);
    let s = fields::number(ui, "rc-size", d.r(58.0, 0.0, 21.0, 13.0), Some(d.r(80.0, 0.0, 9.0, 13.0)), Some(ta.retouch_size as f64), (1.0, 100.0), 0, true);
    fields::label_right(ui, d.r(110.0, 2.0, 40.0, 8.0), &tx("pi-shape"), true);
    let mut round = None;
    if fields::toggle(ui, "rc-round", d.r(154.0, 0.0, 15.0, 12.0), Glyph::RoundTip, ta.retouch_round, true, &tx("pi-tip-round-brush")) {
        round = Some(true);
    }
    if fields::toggle(ui, "rc-square", d.r(169.0, 0.0, 15.0, 12.0), Glyph::SquareTip, !ta.retouch_round, true, &tx("pi-tip-square-brush")) {
        round = Some(false);
    }
    fields::label_right(ui, d.r(196.0, 2.0, 52.0, 8.0), &tx("pi-tolerance"), true);
    let tol = fields::number(ui, "rc-tol", d.r(250.0, 0.0, 21.0, 13.0), Some(d.r(272.0, 0.0, 9.0, 13.0)), Some(ta.tolerance as f64), (0.0, 255.0), 0, true);
    fields::label_right(ui, d.r(213.0, 17.0, 35.0, 8.0), &tx("pi-strength"), true);
    let stg = fields::number(ui, "rc-str", d.r(250.0, 15.0, 21.0, 13.0), Some(d.r(272.0, 15.0, 9.0, 13.0)), Some(ta.strength as f64), (0.0, 255.0), 0, true);
    fields::label_right(ui, d.r(19.0, 17.0, 37.0, 8.0), &tx("pi-replace-from"), true);
    let _ = fields::combo(ui, "rc-from", d.r(58.0, 15.0, 56.0, 13.0), &refs(&tl(&["pi-replace-swatch", "pi-replace-image"])), Some(0), true);
    // „Von“/„in“ sind eigene Farben des Werkzeugs (Werksvorgabe #00FF00 → #FF0000, PARITAET_A W8),
    // nicht Strich-/Füllfarbe; das Werkzeug malt mit genau diesen Voreinstellungen
    let (src, dst) = replace_colors(&st.session.prefs);
    if let Some(super::popups::ColorChoice::Color(c)) = super::popups::color_button(ui, "rc-c1", d.r(117.0, 15.0, 14.0, 12.0), st, Some(src), true, false, false) {
        st.session.prefs.set(REPLACE_SRC_PREF, fp_engine::prefs::PrefValue::Str(ui_hex(c)));
    }
    fields::label(ui, d.r(137.0, 17.0, 15.0, 8.0), &tx("pi-replace-to"), true);
    if let Some(super::popups::ColorChoice::Color(c)) = super::popups::color_button(ui, "rc-c2", d.r(153.0, 15.0, 14.0, 12.0), st, Some(dst), true, false, false) {
        st.session.prefs.set(REPLACE_DST_PREF, fp_engine::prefs::PrefValue::Str(ui_hex(c)));
    }
    let col = fields::checkbox(ui, "rc-colorize", d.r(290.0, 16.0, 83.0, 10.0), &tx("pi-colorize"), ta.colorize, true);
    edit_tool_attrs(ui.ctx(), |a| {
        if let Some(v) = s {
            a.retouch_size = v as i32;
        }
        if let Some(v) = round {
            a.retouch_round = v;
        }
        if let Some(v) = tol {
            a.tolerance = v as i32;
        }
        if let Some(v) = stg {
            a.strength = v as i32;
        }
        if let Some(v) = col {
            a.colorize = v;
        }
    });
}

fn freeform_q3(ui: &mut Ui, _st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    let ta = tool_attrs(ui.ctx());
    fields::label_right(ui, d.r(4.0, 2.0, 32.0, 8.0), &tx("pi-size"), true);
    let s = fields::number(ui, "ff-size", d.r(40.0, 0.0, 20.0, 13.0), Some(d.r(60.0, 0.0, 9.0, 13.0)), Some(ta.freeform_size as f64), (1.0, 500.0), 0, true);
    let p = fields::checkbox(ui, "ff-pr", d.r(85.0, 1.0, 106.0, 10.0), &tx("pi-check-pressure"), ta.pressure, true);
    let v = fields::checkbox(ui, "ff-pv", d.r(85.0, 13.0, 106.0, 10.0), &tx("pi-check-preview"), ta.preview, true);
    edit_tool_attrs(ui.ctx(), |a| {
        if let Some(x) = s {
            a.freeform_size = x as i32;
        }
        if let Some(x) = p {
            a.pressure = x;
        }
        if let Some(x) = v {
            a.preview = x;
        }
    });
}

fn reshape_q3(ui: &mut Ui, _st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    let ta = tool_attrs(ui.ctx());
    fields::label_right(ui, d.r(4.0, 2.0, 34.0, 8.0), &tx("pi-size"), true);
    let s = fields::number(ui, "rs-size", d.r(40.0, 0.0, 22.0, 13.0), Some(d.r(62.0, 0.0, 9.0, 13.0)), Some(ta.freeform_size as f64), (1.0, 500.0), 0, true);
    fields::label_right(ui, d.r(1.0, 17.0, 37.0, 8.0), &tx("pi-strength"), true);
    let k = fields::number(ui, "rs-str", d.r(40.0, 15.0, 22.0, 13.0), Some(d.r(62.0, 15.0, 9.0, 13.0)), Some(ta.strength as f64), (1.0, 100.0), 0, true);
    fields::label_right(ui, d.r(92.0, 2.0, 46.0, 8.0), &tx("pi-pressure"), true);
    let p1 = fields::checkbox(ui, "rs-ps", d.r(141.0, 2.0, 60.0, 10.0), &tx("pi-check-size"), ta.pressure, true);
    let p2 = fields::checkbox(ui, "rs-pk", d.r(141.0, 14.0, 60.0, 10.0), &tx("pi-check-strength"), ta.velocity, true);
    edit_tool_attrs(ui.ctx(), |a| {
        if let Some(x) = s {
            a.freeform_size = x as i32;
        }
        if let Some(x) = k {
            a.strength = x as i32;
        }
        if let Some(x) = p1 {
            a.pressure = x;
        }
        if let Some(x) = p2 {
            a.velocity = x;
        }
    });
}

fn scrub_q3(ui: &mut Ui, _st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    let ta = tool_attrs(ui.ctx());
    let p = fields::checkbox(ui, "scrub-p", d.r(9.0, 1.0, 49.0, 10.0), &tx("pi-check-pressure"), ta.pressure, true);
    let v = fields::checkbox(ui, "scrub-v", d.r(9.0, 17.0, 72.0, 10.0), &tx("pi-check-velocity"), ta.velocity, true);
    fields::label_right(ui, d.r(66.0, 2.0, 26.0, 8.0), &tx("pi-rate"), true);
    let rt = fields::number(ui, "scrub-r", d.r(95.0, 0.0, 22.0, 13.0), Some(d.r(117.0, 0.0, 9.0, 13.0)), Some(ta.rate as f64), (1.0, 100.0), 0, true);
    edit_tool_attrs(ui.ctx(), |a| {
        if let Some(x) = p {
            a.pressure = x;
        }
        if let Some(x) = v {
            a.velocity = x;
        }
        if let Some(x) = rt {
            a.rate = x as i32;
        }
    });
}

fn url_tool_q3(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let d = Dl::at(r.min);
    fields::label(ui, d.r(3.0, 1.0, 32.0, 11.0), &tx("pi-shape"), true);
    let cur = match st.session.tools.active {
        ToolId::HotspotCircle => 1,
        ToolId::HotspotPolygon => 2,
        _ => 0,
    };
    if let Some(i) = fields::combo(ui, "url-shape", d.r(38.0, 0.0, 95.0, 13.0), &refs(&tl(&["pi-shape-rectangle", "pi-shape-ellipse", "pi-shape-polygon"])), Some(cur), true) {
        st.push(Action::SetTool([ToolId::HotspotRect, ToolId::HotspotCircle, ToolId::HotspotPolygon][i]));
    }
}

/// Wird gerade eine Maske bearbeitet (Auswahl = Maskenelement einer Gruppe, „Maske bearbeiten“)?
fn editing_mask(st: &AppState) -> bool {
    let Some(d) = st.session.active() else { return false };
    let Some(id) = d.selection.elements.first().copied() else { return false };
    let Some(pg) = d.doc.pages.get(d.page) else { return false };
    let mut found = false;
    fp_model::traverse::walk_root(&pg.root, fp_model::RootRef::Page(d.page), &mut |_, _, e| {
        if let fp_model::ElementKind::Group(g) = &e.kind
            && g.mask.as_ref().is_some_and(|m| m.id == id)
        {
            found = true;
        }
    });
    found
}

/// Masken-Teilfeld (14117): wie im Original nur sichtbar, während eine Maske bearbeitet wird
/// (sonst bleibt die Fläche leer).
fn mask_sub(ui: &mut Ui, st: &AppState, d: Dl) {
    if !editing_mask(st) {
        return;
    }
    fields::label(ui, d.r(2.0, 2.0, 29.0, 8.0), &tx("pi-mask"), false);
    let _ = fields::radio(ui, Id::new("mask-alpha"), d.r(39.0, 2.0, 80.0, 10.0), &tx("pi-mask-alpha"), true, false);
    let _ = fields::radio(ui, Id::new("mask-gray"), d.r(39.0, 15.0, 80.0, 10.0), &tx("pi-mask-gray"), false, false);
}

fn mask_sub_q4(ui: &mut Ui, st: &mut AppState, r: Rect) {
    mask_sub(ui, st, Dl::at(r.min).offset(254.0, 0.0));
}

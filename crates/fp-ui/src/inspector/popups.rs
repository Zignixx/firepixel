//! Popups des Inspektors (11 §2.3.3 „Strich-, Füllungs- und Effekt-Popups“):
//! - Farbfelder-Popup (6010): Farbfelder des Farbfelder-Bedienfelds, Hex-Feld, „Keine“,
//!   Systemfarbwähler und Pipette,
//! - Verlaufs-Editor (5980): Form, Umkehren, Farb- und Deckkraftstopps,
//! - Strichkategorien (4116) mit Vorgaben je Kategorie,
//! - Pipette über die Leinwand (Werkzeug Pipette vorübergehend, danach Rückkehr).

use super::fields::{self, Dl};
use super::glyphs::Glyph;
use crate::app_state::{Action, AppState};
use crate::theme::Theme;
use crate::ui_commands::{self, GRADIENT_SHAPES};
use egui::{pos2, vec2, Align2, Color32, Id, Rect, Response, Sense, Stroke, StrokeKind, Ui};
use fp_color::{Argb, Rgba8};
use fp_engine::ToolId;
use serde_json::{json, Value};

/// Ergebnis des Farbfelder-Popups.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColorChoice {
    Color(Argb),
    /// Keine Farbe (transparent).
    None,
    /// Pipette starten.
    Pipette,
}

fn c32(c: Argb) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), c.a())
}

/// Inhalt des Farbfelder-Popups. `allow_none`: Knopf „Keine“; `allow_pipette`: Pipette.
pub fn color_picker_ui(ui: &mut Ui, id: Id, st: &AppState, current: Option<Argb>, allow_none: bool, allow_pipette: bool) -> Option<ColorChoice> {
    let t = Theme::current(ui.ctx());
    let c = &t.colors;
    let mut out = None;
    let swatches = crate::panels::swatch_colors(ui.ctx(), st);
    let cols = 21usize;
    let cell = 10.0;
    let rows = swatches.len().div_ceil(cols).max(1);
    let width = cols as f32 * cell + 2.0;
    let (head, _) = ui.allocate_exact_size(vec2(width, 24.0), Sense::hover());
    // Vorschau der aktuellen bzw. überfahrenen Farbe
    let hover_key = id.with("hover");
    let hovered: Option<Argb> = ui.data(|d| d.get_temp(hover_key));
    let pv = Rect::from_min_size(head.min + vec2(1.0, 2.0), vec2(34.0, 20.0));
    match hovered.or(current) {
        Some(col) => {
            crate::widgets::checker(ui.painter(), pv, 4.0, &t);
            ui.painter().rect_filled(pv, 0.0, c32(col));
        }
        None => {
            ui.painter().rect_filled(pv, 0.0, Color32::WHITE);
            ui.painter().line_segment([pv.left_bottom(), pv.right_top()], Stroke::new(1.5, c.icon_red));
        }
    }
    ui.painter().rect_stroke(pv, 0.0, Stroke::new(1.0, c.field_border), StrokeKind::Inside);
    let hex_r = Rect::from_min_size(pos2(pv.right() + 6.0, head.top() + 3.0), vec2(70.0, 19.0));
    let cur_hex = hovered.or(current).map(|x| format!("#{:02X}{:02X}{:02X}", x.r(), x.g(), x.b())).unwrap_or_default();
    if let Some(s) = fields::text(ui, id.with("hex"), hex_r, &cur_hex, true) {
        let s = s.trim().to_string();
        let s = if s.starts_with('#') { s } else { format!("#{s}") };
        if let Some(col) = Rgba8::parse_hex(&s) {
            out = Some(ColorChoice::Color(Argb(col.to_argb().0 | 0xff00_0000)));
        }
    }
    let mut x = head.right() - 22.0;
    if allow_pipette {
        if fields::icon_button(ui, id.with("pip"), Rect::from_min_size(pos2(x, head.top() + 2.0), vec2(20.0, 20.0)), Glyph::Eyedropper, true, "Pipette").clicked() {
            out = Some(ColorChoice::Pipette);
        }
        x -= 22.0;
    }
    let sys_key = id.with("sys");
    let mut sys: bool = ui.data(|d| d.get_temp(sys_key)).unwrap_or(false);
    if fields::icon_button(ui, id.with("sysb"), Rect::from_min_size(pos2(x, head.top() + 2.0), vec2(20.0, 20.0)), Glyph::Picker, true, "Systemfarbwähler").clicked() {
        sys = !sys;
        ui.data_mut(|d| d.insert_temp(sys_key, sys));
    }
    x -= 22.0;
    if allow_none && fields::icon_button(ui, id.with("none"), Rect::from_min_size(pos2(x, head.top() + 2.0), vec2(20.0, 20.0)), Glyph::FillNone, true, "Transparent").clicked() {
        out = Some(ColorChoice::None);
    }
    // Farbfelder
    let (grid, resp) = ui.allocate_exact_size(vec2(width, rows as f32 * cell + 2.0), Sense::click());
    ui.painter().rect_filled(grid, 0.0, c.line_dark);
    let mut hov = None;
    for (i, col) in swatches.iter().enumerate() {
        let r = Rect::from_min_size(pos2(grid.left() + 1.0 + (i % cols) as f32 * cell, grid.top() + 1.0 + (i / cols) as f32 * cell), vec2(cell, cell));
        ui.painter().rect_filled(r.shrink(0.5), 0.0, c32(*col));
        if resp.hover_pos().is_some_and(|p| r.contains(p)) {
            hov = Some(*col);
            ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0, Color32::WHITE), StrokeKind::Inside);
        }
    }
    ui.data_mut(|d| match hov {
        Some(h) => {
            d.insert_temp(hover_key, h);
        }
        None => {
            d.remove::<Argb>(hover_key);
        }
    });
    if resp.clicked()
        && let Some(h) = hov
    {
        out = Some(ColorChoice::Color(h));
    }
    if sys {
        let mut rgb = current.map(|x| Color32::from_rgb(x.r(), x.g(), x.b())).unwrap_or(Color32::WHITE);
        ui.add_space(4.0);
        if egui::color_picker::color_picker_color32(ui, &mut rgb, egui::color_picker::Alpha::Opaque) {
            ui.data_mut(|d| d.insert_temp(id.with("syscol"), rgb));
        }
        if let Some(col) = ui.data(|d| d.get_temp::<Color32>(id.with("syscol")))
            && ui.button("Übernehmen").clicked()
        {
            out = Some(ColorChoice::Color(Argb(0xff00_0000 | (col.r() as u32) << 16 | (col.g() as u32) << 8 | col.b() as u32)));
        }
    }
    out
}

/// Farbfeld mit Popup. Liefert die Wahl beim Anklicken eines Farbfelds.
#[allow(clippy::too_many_arguments)]
pub fn color_button(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, r: Rect, st: &AppState, color: Option<Argb>, enabled: bool, allow_none: bool, allow_pipette: bool) -> Option<ColorChoice> {
    let id = Id::new(id);
    let resp = fields::swatch(ui, id, r, color, enabled);
    color_popup(ui, &resp, id, st, color, allow_none, allow_pipette)
}

/// Farbfelder-Popup an einer Antwort.
pub fn color_popup(ui: &mut Ui, resp: &Response, id: Id, st: &AppState, color: Option<Argb>, allow_none: bool, allow_pipette: bool) -> Option<ColorChoice> {
    let t = Theme::current(ui.ctx());
    let mut out = None;
    egui::Popup::from_toggle_button_response(resp)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .frame(crate::menu::menu_frame(&t).inner_margin(egui::Margin::same(4)))
        .show(|ui| {
            out = color_picker_ui(ui, id.with("picker"), st, color, allow_none, allow_pipette);
            if out.is_some() {
                ui.close();
            }
        });
    out
}

// ---------------------------------------------------------------------- Pipette über die Leinwand

#[derive(Clone, Debug)]
struct PickState {
    command: String,
    params: Value,
    key: String,
    prev_tool: ToolId,
    fill: Argb,
    stroke: Argb,
}

const PICK_ID: &str = "pi-pick";

/// Platzhalterfarbe während der Pipette: völlig transparent und daher nie das Ergebnis einer
/// Aufnahme (die Pipette liefert deckende Farben). So wird auch eine Aufnahme erkannt, die
/// dieselbe Farbe wie vorher ergibt.
const PICK_SENTINEL: Argb = Argb(0x0012_3457);

/// Startet die Pipette: das nächste Aufnehmen auf der Leinwand setzt `params[key]` und führt
/// `command` aus (z. B. `pi.fill` mit `color`).
pub fn start_pick(ctx: &egui::Context, st: &mut AppState, command: &str, params: Value, key: &str) {
    let ps = PickState { command: command.into(), params, key: key.into(), prev_tool: st.session.tools.active, fill: st.session.tools.fill_color, stroke: st.session.tools.stroke_color };
    ctx.data_mut(|d| d.insert_temp(Id::new(PICK_ID), ps));
    st.push(Action::SetColors { stroke: Some(PICK_SENTINEL), fill: Some(PICK_SENTINEL) });
    st.push(Action::SetTool(ToolId::Eyedropper));
}

/// Ist die Pipette aktiv?
pub fn picking(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<PickState>(Id::new(PICK_ID))).is_some()
}

/// Jeden Frame aufrufen: übernimmt die aufgenommene Farbe und stellt Werkzeug und Farben wieder her.
pub fn poll_pick(ctx: &egui::Context, st: &mut AppState) {
    let Some(ps) = ctx.data(|d| d.get_temp::<PickState>(Id::new(PICK_ID))) else { return };
    let tools = &st.session.tools;
    if tools.active != ToolId::Eyedropper {
        // erst nach dem Werkzeugwechsel prüfen; anderes Werkzeug gewählt = abbrechen
        if tools.active != ps.prev_tool {
            ctx.data_mut(|d| d.remove::<PickState>(Id::new(PICK_ID)));
            st.push(Action::SetColors { stroke: Some(ps.stroke), fill: Some(ps.fill) });
        }
        return;
    }
    let picked = if tools.fill_color != PICK_SENTINEL {
        Some(tools.fill_color)
    } else if tools.stroke_color != PICK_SENTINEL {
        Some(tools.stroke_color)
    } else {
        None
    };
    if let Some(c) = picked {
        ctx.data_mut(|d| d.remove::<PickState>(Id::new(PICK_ID)));
        st.push(Action::SetColors { stroke: Some(ps.stroke), fill: Some(ps.fill) });
        let mut p = ps.params.clone();
        if let Value::Object(m) = &mut p {
            m.insert(ps.key.clone(), json!(ui_commands::hex(Argb(c.0 | 0xff00_0000))));
        }
        st.push(Action::Run(ps.command.clone(), p));
        st.push(Action::SetTool(ps.prev_tool));
    }
}

// ---------------------------------------------------------------------- Verlaufs-Editor

#[derive(Clone, Debug, PartialEq)]
struct GradEdit {
    colors: Vec<(f32, Argb)>,
    opacities: Vec<(f32, Argb)>,
    sel: Option<(bool, usize)>,
    drag: Option<(bool, usize)>,
}

fn lerp(a: Argb, b: Argb, t: f32) -> Argb {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round().clamp(0.0, 255.0) as u32;
    Argb(m(a.a(), b.a()) << 24 | m(a.r(), b.r()) << 16 | m(a.g(), b.g()) << 8 | m(a.b(), b.b()))
}

/// Farbe eines Stoppverlaufs an Position `p`.
pub fn sample_stops(stops: &[(f32, Argb)], p: f32) -> Argb {
    match stops {
        [] => Argb(0xff00_0000),
        [one] => one.1,
        _ => {
            if p <= stops[0].0 {
                return stops[0].1;
            }
            for w in stops.windows(2) {
                if p <= w[1].0 {
                    let span = (w[1].0 - w[0].0).max(1e-6);
                    return lerp(w[0].1, w[1].1, (p - w[0].0) / span);
                }
            }
            stops[stops.len() - 1].1
        }
    }
}

/// Verlaufs-Editor (Popup 5980). Liefert Parameter für `pi.fill` bzw. das Werkzeug.
pub fn gradient_editor(ui: &mut Ui, id: Id, fill: &fp_model::Fill) -> Option<Value> {
    let t = Theme::current(ui.ctx());
    let c = &t.colors;
    let g = fill.gradient.as_ref();
    let base_c: Vec<(f32, Argb)> = g.map(|g| g.color_stops().iter().map(|s| (s.position, s.color)).collect()).unwrap_or_default();
    let base_o: Vec<(f32, Argb)> = g.map(|g| g.opacity_stops().iter().map(|s| (s.position, s.color)).collect()).unwrap_or_default();
    let key = id.with("edit");
    let mut ge: GradEdit = ui.data(|d| d.get_temp(key)).unwrap_or(GradEdit { colors: base_c.clone(), opacities: base_o.clone(), sel: None, drag: None });
    if ge.drag.is_none() && (ge.colors != base_c || ge.opacities != base_o) && ge.sel.is_none() {
        ge.colors = base_c.clone();
        ge.opacities = base_o.clone();
    }
    let mut out = None;
    // Kopf: Form + Umkehren
    let (head, _) = ui.allocate_exact_size(vec2(240.0, 24.0), Sense::hover());
    let names: Vec<&str> = GRADIENT_SHAPES.iter().map(|g| g.0).collect();
    let cur = GRADIENT_SHAPES.iter().position(|g| g.1 == fill.shape);
    if let Some(i) = fields::combo(ui, id.with("shape"), Rect::from_min_size(head.min + vec2(0.0, 2.0), vec2(150.0, 20.0)), &names, cur, true) {
        out = Some(json!({"shape": GRADIENT_SHAPES[i].1}));
    }
    if fields::button(ui, id.with("rev"), Rect::from_min_size(pos2(head.left() + 156.0, head.top() + 2.0), vec2(84.0, 20.0)), "Umkehren", true) {
        out = Some(json!({"reverse": true}));
    }
    ui.add_space(4.0);
    // Deckkraftstopps (oben), Vorschau, Farbstopps (unten)
    let (area, resp) = ui.allocate_exact_size(vec2(240.0, 56.0), Sense::click_and_drag());
    let bar = Rect::from_min_max(pos2(area.left() + 6.0, area.top() + 16.0), pos2(area.right() - 6.0, area.top() + 38.0));
    crate::widgets::checker(ui.painter(), bar, 5.0, &t);
    let n = 48;
    for i in 0..n {
        let p0 = i as f32 / n as f32;
        let col = sample_stops(&ge.colors, p0 + 0.5 / n as f32);
        let a = if ge.opacities.is_empty() { col.a() } else { sample_stops(&ge.opacities, p0 + 0.5 / n as f32).a() };
        let r = Rect::from_min_max(pos2(bar.left() + p0 * bar.width(), bar.top()), pos2(bar.left() + (i + 1) as f32 / n as f32 * bar.width() + 0.5, bar.bottom()));
        ui.painter().rect_filled(r, 0.0, Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), a));
    }
    ui.painter().rect_stroke(bar, 0.0, Stroke::new(1.0, c.field_border), StrokeKind::Outside);
    let stop_rect = |top: bool, p: f32| {
        let x = bar.left() + p.clamp(0.0, 1.0) * bar.width();
        if top { Rect::from_center_size(pos2(x, area.top() + 7.0), vec2(9.0, 11.0)) } else { Rect::from_center_size(pos2(x, area.bottom() - 7.0), vec2(9.0, 11.0)) }
    };
    for (top, list) in [(true, &ge.opacities), (false, &ge.colors)] {
        for (i, (p, col)) in list.iter().enumerate() {
            let r = stop_rect(top, *p);
            let fillc = if top { let a = col.a(); Color32::from_gray(255 - a) } else { Color32::from_rgb(col.r(), col.g(), col.b()) };
            ui.painter().rect_filled(r, 1.0, fillc);
            let selected = ge.sel == Some((top, i));
            ui.painter().rect_stroke(r, 1.0, Stroke::new(if selected { 2.0 } else { 1.0 }, if selected { c.focus } else { c.icon_fg }), StrokeKind::Inside);
        }
    }
    if (resp.drag_started() || resp.clicked())
        && let Some(pp) = resp.interact_pointer_pos()
    {
        {
            let hit = |top: bool, list: &[(f32, Argb)]| list.iter().position(|(p, _)| stop_rect(top, *p).expand(2.0).contains(pp)).map(|i| (top, i));
            let found = hit(true, &ge.opacities).or_else(|| hit(false, &ge.colors));
            match found {
                Some(s) => {
                    ge.sel = Some(s);
                    if resp.drag_started() {
                        ge.drag = Some(s);
                    }
                }
                None if resp.clicked() => {
                    let pos = ((pp.x - bar.left()) / bar.width()).clamp(0.0, 1.0);
                    let top = pp.y < bar.top();
                    if top {
                        let a = sample_stops(&ge.opacities, pos);
                        ge.opacities.push((pos, a));
                        ge.opacities.sort_by(|a, b| a.0.total_cmp(&b.0));
                    } else {
                        let col = sample_stops(&ge.colors, pos);
                        ge.colors.push((pos, col));
                        ge.colors.sort_by(|a, b| a.0.total_cmp(&b.0));
                    }
                    out = Some(stops_params(&ge));
                    ge.sel = None;
                }
                None => {}
            }
        }
    }
    if let (Some((top, i)), Some(pp)) = (ge.drag, resp.interact_pointer_pos())
        && resp.dragged()
    {
        let pos = ((pp.x - bar.left()) / bar.width()).clamp(0.0, 1.0);
        let list = if top { &mut ge.opacities } else { &mut ge.colors };
        if let Some(s) = list.get_mut(i) {
            s.0 = pos;
        }
    }
    if resp.drag_stopped() && ge.drag.is_some() {
        ge.drag = None;
        ge.colors.sort_by(|a, b| a.0.total_cmp(&b.0));
        ge.opacities.sort_by(|a, b| a.0.total_cmp(&b.0));
        out = Some(stops_params(&ge));
        ge.sel = None;
    }
    // Bearbeiten des gewählten Stopps
    if let Some((top, i)) = ge.sel {
        let (row, _) = ui.allocate_exact_size(vec2(240.0, 24.0), Sense::hover());
        let d = Dl::at(row.min);
        if top {
            let a = ge.opacities.get(i).map(|s| s.1.a() as f64 * 100.0 / 255.0).unwrap_or(100.0);
            fields::label(ui, d.r(0.0, 2.0, 40.0, 10.0), "Deckkraft:", true);
            if let Some(v) = fields::number(ui, id.with("op"), d.r(42.0, 0.0, 24.0, 13.0), Some(d.r(66.0, 0.0, 9.0, 13.0)), Some(a), (0.0, 100.0), 0, true)
                && let Some(s) = ge.opacities.get_mut(i)
            {
                let a8 = (v * 2.55).round().clamp(0.0, 255.0) as u32;
                s.1 = Argb(a8 << 24 | 0x00ff_ffff);
                out = Some(stops_params(&ge));
            }
        } else {
            let cur = ge.colors.get(i).map(|s| s.1);
            let hex = cur.map(|x| format!("#{:02X}{:02X}{:02X}", x.r(), x.g(), x.b())).unwrap_or_default();
            fields::label(ui, d.r(0.0, 2.0, 30.0, 10.0), "Farbe:", true);
            if let Some(s) = fields::text(ui, id.with("hex"), d.r(32.0, 0.0, 44.0, 13.0), &hex, true) {
                let s = if s.starts_with('#') { s } else { format!("#{s}") };
                if let (Some(col), Some(st)) = (Rgba8::parse_hex(&s), ge.colors.get_mut(i)) {
                    st.1 = Argb(col.to_argb().0 | 0xff00_0000);
                    out = Some(stops_params(&ge));
                }
            }
        }
        let list_len = if top { ge.opacities.len() } else { ge.colors.len() };
        if list_len > 2 && fields::button(ui, id.with("del"), d.r(110.0, 0.0, 50.0, 13.0), "Löschen", true) {
            if top {
                ge.opacities.remove(i);
            } else {
                ge.colors.remove(i);
            }
            ge.sel = None;
            out = Some(stops_params(&ge));
        }
        if !top {
            // Farbfelder für den gewählten Stopp
            let sw = [0xffff_ffffu32, 0xff00_0000, 0xffff_0000, 0xffff_9900, 0xffff_ff00, 0xff00_cc00, 0xff00_ccff, 0xff00_33ff, 0xff99_00ff, 0xffff_00cc, 0xff99_9999, 0xff66_3300];
            let (row2, r2) = ui.allocate_exact_size(vec2(240.0, 14.0), Sense::click());
            for (k, col) in sw.iter().enumerate() {
                let rr = Rect::from_min_size(pos2(row2.left() + k as f32 * 14.0, row2.top()), vec2(13.0, 13.0));
                ui.painter().rect_filled(rr, 0.0, c32(Argb(*col)));
                ui.painter().rect_stroke(rr, 0.0, Stroke::new(1.0, c.field_border), StrokeKind::Inside);
                if r2.clicked()
                    && r2.interact_pointer_pos().is_some_and(|p| rr.contains(p))
                    && let Some(st) = ge.colors.get_mut(i)
                {
                    st.1 = Argb(*col);
                    out = Some(stops_params(&ge));
                }
            }
        }
    }
    ui.painter().text(pos2(area.left(), area.bottom() + 2.0), Align2::LEFT_TOP, "", t.small_font(), c.text_weak);
    ui.data_mut(|d| d.insert_temp(key, ge));
    out
}

fn stops_params(ge: &GradEdit) -> Value {
    let cs: Vec<Value> = ge.colors.iter().map(|(p, c)| json!([p, ui_commands::hex(*c)])).collect();
    let os: Vec<Value> = ge.opacities.iter().map(|(p, c)| json!([p, ui_commands::hex(*c)])).collect();
    if os.len() >= 2 { json!({"stops": cs, "opacityStops": os}) } else { json!({"stops": cs}) }
}

// ---------------------------------------------------------------------- Strichkategorien

/// Strichkategorie-Knopf (14606) mit Menü: „Keine“, Kategorien mit Vorgaben.
/// Liefert `Some(None)` für „Keine“ bzw. `Some(Some(index))` in [`ui_commands::BRUSH_PRESETS`].
/// Gespeicherte Pinselstriche des Dokuments stehen unter „Gespeichert“ mit Indizes ab
/// `BRUSH_PRESETS.len()`.
pub fn stroke_category(ui: &mut Ui, id: Id, r: Rect, label: &str, enabled: bool, saved: &[String]) -> Option<Option<usize>> {
    let resp = fields::combo_box_frame(ui, id, r, label, enabled);
    let t = Theme::current(ui.ctx());
    let mut out = None;
    egui::Popup::menu(&resp).frame(crate::menu::menu_frame(&t)).show(|ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        if fields::list_item(ui, "Keine", label == "Ohne").clicked() {
            out = Some(None);
            ui.close();
        }
        for cat in ui_commands::brush_categories() {
            ui.menu_button(cat, |ui| {
                for (i, b) in ui_commands::BRUSH_PRESETS.iter().enumerate().filter(|(_, b)| b.category_de == cat) {
                    if fields::list_item(ui, b.name_de, b.name_de == label).clicked() {
                        out = Some(Some(i));
                        ui.close();
                    }
                }
            });
        }
        if !saved.is_empty() {
            ui.menu_button("Gespeichert", |ui| {
                for (k, n) in saved.iter().enumerate() {
                    if fields::list_item(ui, n, n == label).clicked() {
                        out = Some(Some(ui_commands::BRUSH_PRESETS.len() + k));
                        ui.close();
                    }
                }
            });
        }
    });
    out
}

/// Liest die Texturnamen aus dem Ressourcenordner des Originals zur Laufzeit (nur die Namen;
/// nichts wird eingebettet; Ordner aus `fp_color::install`). Ohne Ordner: leere Liste.
pub fn texture_names() -> &'static [String] {
    static N: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    N.get_or_init(|| {
        let mut v = Vec::new();
        if let Some(base) = fp_color::install::original_subdir("Configuration\\Textures")
            && let Ok(rd) = std::fs::read_dir(base)
        {
            {
                for e in rd.flatten() {
                    if let Some(stem) = e.path().file_stem().map(|s| s.to_string_lossy().to_string()) {
                        v.push(stem);
                    }
                }
            }
        }
        v.sort();
        v.dedup();
        v
    })
}

/// Inhalt eines Popups als Antwort-Platzhalter (für Tests: Fläche reservieren).
pub fn reserve(ui: &mut Ui, size: egui::Vec2) -> Rect {
    ui.allocate_exact_size(size, Sense::hover()).0
}

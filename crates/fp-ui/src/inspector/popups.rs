//! Popups des Inspektors (11 §2.3.3 „Strich-, Füllungs- und Effekt-Popups“):
//! - Farbfeld mit dem Farbwähler-Popup des Originals (Umsetzung in [`super::color_picker`], N27b),
//! - Verlaufs-Popup (5980): Art, Leiste mit Farb- und Deckkraftstopps, Umkehren, Winkel,
//! - Muster-Popup: Musterliste, Vorschau, Winkel,
//! - Strichkategorien (4116) mit Vorgaben je Kategorie,
//! - Pipette über die Leinwand (Werkzeug Pipette vorübergehend, danach Rückkehr).

use super::fields::{self, Dl};
use super::color_picker::Footer;
use super::texts::{self, refs};
use crate::app_state::{Action, AppState};
use crate::theme::Theme;
use crate::ui_commands::{self, GRADIENT_SHAPES};
use egui::{pos2, vec2, Align2, Color32, Id, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui};
use fp_color::Argb;
use fp_engine::ToolId;
use serde_json::{json, Value};

/// Ergebnis des Farbwähler-Popups ([`super::color_picker`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColorChoice {
    Color(Argb),
    /// Keine Farbe (transparent, Knopf „Ohne“).
    None,
    /// Pipette starten (älterer Ablauf; das Popup des Originals ist selbst schon Pipette und liefert
    /// die aufgenommene Farbe direkt als [`ColorChoice::Color`]).
    Pipette,
    /// Werkzeugleiste Füllung: Knopf „Verlauf“.
    Gradient,
    /// Werkzeugleiste Füllung: Knopf „Muster“.
    Pattern,
    /// Kontur: Strichlage 0 innen, 1 mittig, 2 außen.
    Placement(u8),
}

/// Id-Basis des Farbwählers eines Farbfelds (für Tests und Aufräumen; Farbdialog: `.with("sys")`,
/// `.with("sysb")`, `.with("syscol")`).
pub fn picker_id(id: Id) -> Id {
    id.with("picker")
}

/// Farbfeld mit dem Farbwähler-Popup des Originals. `allow_none`: Fußleiste mit „Ohne“; die Pipette
/// ist immer aktiv (`allow_pipette` bleibt nur für ältere Aufrufer).
#[allow(clippy::too_many_arguments)]
pub fn color_button(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, r: Rect, st: &AppState, color: Option<Argb>, enabled: bool, allow_none: bool, allow_pipette: bool) -> Option<ColorChoice> {
    let _ = allow_pipette;
    color_button_with(ui, id, r, st, color, enabled, if allow_none { Footer::NoneOnly } else { Footer::Plain })
}

/// Farbfeld mit dem Farbwähler-Popup und einer bestimmten Fußleiste.
pub fn color_button_with(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, r: Rect, st: &AppState, color: Option<Argb>, enabled: bool, footer: Footer) -> Option<ColorChoice> {
    let id = Id::new(id);
    let resp = fields::swatch(ui, id, r, color, enabled);
    super::color_picker::popup(ui, &resp, st, color, footer)
}

/// Farbwähler-Popup an einer Antwort (öffnet beim Klick darauf).
pub fn color_popup(ui: &mut Ui, resp: &Response, id: Id, st: &AppState, color: Option<Argb>, allow_none: bool, allow_pipette: bool) -> Option<ColorChoice> {
    let _ = (id, allow_pipette);
    super::color_picker::popup(ui, resp, st, color, if allow_none { Footer::NoneOnly } else { Footer::Plain })
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

// ---------------------------------------------------------------------- Füllungs-Popups (Verlauf, Muster)

/// Merker: Id des offenen Füllungs-Popups (Verlauf bzw. Muster), siehe [`fill_popup_open`].
const FILL_POPUP_OPEN: &str = "pi-fill-popup-open";

/// Ist ein Verlaufs- oder Muster-Popup offen? (Esc schließt dann nur das Popup.)
pub fn fill_popup_open(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<Id>(Id::new(FILL_POPUP_OPEN))).is_some()
}

/// Ist das Füllungs-Popup `pid` offen?
pub fn anchored_open(ctx: &egui::Context, pid: Id) -> bool {
    ctx.data(|d| d.get_temp::<bool>(pid.with("open"))).unwrap_or(false)
}

/// Öffnet bzw. schließt das Füllungs-Popup `pid` (z. B. Füllart-Knopf „Verlauf“: umstellen und
/// sofort öffnen wie im Original, PAR 18/01).
pub fn set_anchored_open(ctx: &egui::Context, pid: Id, open: bool) {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| {
        d.insert_temp(pid.with("open"), open);
        if open {
            d.insert_temp(pid.with("pass"), pass);
            d.insert_temp(Id::new(FILL_POPUP_OPEN), pid);
        } else if d.get_temp::<Id>(Id::new(FILL_POPUP_OPEN)) == Some(pid) {
            d.remove::<Id>(Id::new(FILL_POPUP_OPEN));
        }
    });
    ctx.request_repaint();
}

/// Popup wie ein kleines Fenster über dem Farbfeld `anchor` (öffnet nach oben, wie das Verlaufs-
/// und Muster-Popup des Originals). Der Offen-Zustand liegt nicht im Popup-Speicher von egui, damit
/// Listen und der Farbwähler darin (eigene egui-Popups) es nicht schließen (PAR 18/02). Schließt
/// mit Esc, mit einem Klick außerhalb (nicht in einem darüberliegenden Popup) und mit `toggle`.
pub fn anchored_popup<R>(ui: &mut Ui, pid: Id, anchor: Rect, toggle: bool, add: impl FnOnce(&mut Ui) -> R) -> Option<R> {
    let ctx = ui.ctx().clone();
    if toggle {
        set_anchored_open(&ctx, pid, !anchored_open(&ctx, pid));
    }
    if !anchored_open(&ctx, pid) {
        return None;
    }
    ctx.data_mut(|d| d.insert_temp(Id::new(FILL_POPUP_OPEN), pid));
    let th = Theme::current(&ctx);
    let shown = egui::Popup::new(pid, ctx.clone(), anchor, ui.layer_id())
        .open(true)
        .close_behavior(egui::PopupCloseBehavior::IgnoreClicks)
        .align(egui::RectAlign::TOP_START)
        .gap(0.0)
        .frame(crate::menu::menu_frame(&th).inner_margin(egui::Margin::same(6)))
        .show(add);
    let Some(inner) = shown else { return None };
    let opened: u64 = ctx.data(|d| d.get_temp(pid.with("pass"))).unwrap_or(0);
    let mut close = inner.response.should_close();
    if ctx.cumulative_pass_nr() > opened
        && ctx.input(|i| i.pointer.any_click())
        && let Some(p) = ctx.input(|i| i.pointer.interact_pos())
    {
        let area = inner.response.rect;
        // ein Klick in eine Liste oder den Farbwähler, die über dem Popup liegen, zählt nicht
        let child = ctx.layer_id_at(p).is_some_and(|l| l.order == egui::Order::Foreground && l != inner.response.layer_id);
        if !area.contains(p) && !child && !anchor.contains(p) {
            close = true;
        }
    }
    if close {
        set_anchored_open(&ctx, pid, false);
    }
    Some(inner.inner)
}

#[derive(Clone, Debug, PartialEq)]
struct GradEdit {
    colors: Vec<(f32, Argb)>,
    opacities: Vec<(f32, Argb)>,
    sel: Option<(bool, usize)>,
    drag: Option<(bool, usize)>,
    /// Farbstopp, dessen Farbwähler offen ist.
    picker: Option<usize>,
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

/// Breite und Höhe des Inhalts des Verlaufs-Popups (mit Rand ≈ 210 × 155 wie im Original).
pub const GRADIENT_POPUP: egui::Vec2 = egui::vec2(200.0, 124.0);

/// Abstand, ab dem ein aus der Leiste gezogener Stopp gelöscht wird (Original: nach unten aus dem
/// Popup ziehen, PAR 18/05).
const STOP_TEAR_OFF: f32 = 24.0;

/// Verlaufs-Popup (5980) wie im Original: „Verlauf:“ mit der Liste der 12 Arten, Verlaufsleiste mit
/// Deckkraftstopps oben und Farbstopps unten (Klick unter die Leiste = neuer Stopp, Stopp aus dem
/// Popup ziehen = löschen, Klick auf einen Farbstopp = Farbwähler), „Umkehren“, Verschieben (grau)
/// und Winkel (`angle`, `None` = grau). Liefert Parameter für `pi.fill` bzw. das Werkzeug.
pub fn gradient_editor(ui: &mut Ui, id: Id, fill: &fp_model::Fill, st: &AppState, angle: Option<f64>) -> Option<Value> {
    let t = Theme::current(ui.ctx());
    let c = &t.colors;
    let g = fill.gradient.as_ref();
    let base_c: Vec<(f32, Argb)> = g.map(|g| g.color_stops().iter().map(|s| (s.position, s.color)).collect()).unwrap_or_default();
    let base_o: Vec<(f32, Argb)> = g.map(|g| g.opacity_stops().iter().map(|s| (s.position, s.color)).collect()).unwrap_or_default();
    let key = id.with("edit");
    let mut ge: GradEdit = ui.data(|d| d.get_temp(key)).unwrap_or(GradEdit { colors: base_c.clone(), opacities: base_o.clone(), sel: None, drag: None, picker: None });
    if ge.drag.is_none() && ge.picker.is_none() && (ge.colors != base_c || ge.opacities != base_o) {
        ge.colors = base_c.clone();
        ge.opacities = base_o.clone();
        ge.sel = None;
    }
    let mut out = None;
    let (all, _) = ui.allocate_exact_size(GRADIENT_POPUP, Sense::hover());
    let o = all.min;
    // „Verlauf:“ + Liste der Arten (Reihenfolge und Namen wie im Original)
    fields::label(ui, Rect::from_min_size(o + vec2(0.0, 4.0), vec2(44.0, 12.0)), &texts::t("pi-grad-label"), true);
    let names = texts::gradient_labels();
    let cur = GRADIENT_SHAPES.iter().position(|s| s.1 == ui_commands::shape_id(fill));
    if let Some(i) = fields::combo(ui, id.with("shape"), Rect::from_min_size(o + vec2(46.0, 0.0), vec2(154.0, 20.0)), &refs(&names), cur, true)
        && Some(i) != cur
    {
        out = Some(json!({"shape": GRADIENT_SHAPES[i].1}));
    }
    // Leiste: Deckkraftstopps oben, Farbstopps unten
    let bar = Rect::from_min_max(o + vec2(16.0, 40.0), o + vec2(192.0, 74.0));
    let area = Rect::from_min_max(pos2(bar.left() - 8.0, o.y + 26.0), pos2(bar.right() + 8.0, o.y + 90.0));
    let resp = ui.interact(area, id.with("bar"), Sense::click_and_drag());
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
    // Symbole links: Deckkraft (Schachbrett) und Farbe
    let ck = Rect::from_min_size(o + vec2(0.0, 27.0), vec2(10.0, 10.0));
    crate::widgets::checker(ui.painter(), ck, 2.5, &t);
    ui.painter().rect_stroke(ck, 0.0, Stroke::new(1.0, c.field_border), StrokeKind::Inside);
    let stop_rect = |top: bool, p: f32| {
        let x = bar.left() + p.clamp(0.0, 1.0) * bar.width();
        if top { Rect::from_center_size(pos2(x, bar.top() - 7.0), vec2(9.0, 11.0)) } else { Rect::from_center_size(pos2(x, bar.bottom() + 7.0), vec2(9.0, 11.0)) }
    };
    let pointer = resp.interact_pointer_pos();
    let torn = |top: bool, pp: Option<Pos2>| pp.is_some_and(|p| if top { p.y < area.top() - STOP_TEAR_OFF } else { p.y > area.bottom() + STOP_TEAR_OFF });
    for (top, list) in [(true, &ge.opacities), (false, &ge.colors)] {
        for (i, (p, col)) in list.iter().enumerate() {
            if ge.drag == Some((top, i)) && torn(top, pointer) && list.len() > 2 {
                continue;
            }
            let r = stop_rect(top, *p);
            let fillc = if top { Color32::from_gray(255 - col.a()) } else { Color32::from_rgb(col.r(), col.g(), col.b()) };
            ui.painter().rect_filled(r, 1.0, fillc);
            let selected = ge.sel == Some((top, i));
            ui.painter().rect_stroke(r, 1.0, Stroke::new(if selected { 2.0 } else { 1.0 }, if selected { c.focus } else { c.icon_fg }), StrokeKind::Inside);
        }
    }
    let mut open_picker_at = None;
    // Treffer am Ort des Drückens (beim Ziehen ist der Zeiger schon weiter)
    let origin = if resp.drag_started() { ui.ctx().input(|i| i.pointer.press_origin()).or(pointer) } else { pointer };
    if (resp.drag_started() || resp.clicked())
        && let Some(pp) = origin
    {
        let hit = |top: bool, list: &[(f32, Argb)]| list.iter().position(|(p, _)| stop_rect(top, *p).expand(2.0).contains(pp)).map(|i| (top, i));
        match hit(true, &ge.opacities).or_else(|| hit(false, &ge.colors)) {
            Some(s) => {
                ge.sel = Some(s);
                if resp.drag_started() {
                    ge.drag = Some(s);
                } else if !s.0 {
                    // Klick auf einen Farbstopp: Farbwähler des Originals (Raster, HEX, RGB)
                    ge.picker = Some(s.1);
                    open_picker_at = Some(pp.x);
                }
            }
            None if resp.clicked() => {
                let pos = ((pp.x - bar.left()) / bar.width()).clamp(0.0, 1.0);
                if pp.y < bar.top() {
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
    if let (Some((top, i)), Some(pp)) = (ge.drag, pointer)
        && resp.dragged()
    {
        let pos = ((pp.x - bar.left()) / bar.width()).clamp(0.0, 1.0);
        let list = if top { &mut ge.opacities } else { &mut ge.colors };
        if let Some(s) = list.get_mut(i) {
            s.0 = pos;
        }
    }
    if resp.drag_stopped()
        && let Some((top, i)) = ge.drag
    {
        let list = if top { &mut ge.opacities } else { &mut ge.colors };
        // aus dem Popup gezogen: Stopp löschen (mindestens zwei bleiben)
        if torn(top, ui.ctx().input(|inp| inp.pointer.latest_pos())) && list.len() > 2 && i < list.len() {
            list.remove(i);
        }
        ge.drag = None;
        ge.colors.sort_by(|a, b| a.0.total_cmp(&b.0));
        ge.opacities.sort_by(|a, b| a.0.total_cmp(&b.0));
        out = Some(stops_params(&ge));
        ge.sel = None;
    }
    // Farbwähler eines Farbstopps
    if let Some(i) = ge.picker {
        let anchor = ge.colors.get(i).map(|s| stop_rect(false, s.0)).unwrap_or(bar);
        let sid = id.with(("stop-color", i));
        match super::color_picker::show(ui, sid, anchor, open_picker_at, st, ge.colors.get(i).map(|s| s.1), Footer::Plain) {
            Some(ColorChoice::Color(col)) => {
                if let Some(s) = ge.colors.get_mut(i) {
                    s.1 = Argb(col.0 | 0xff00_0000);
                }
                out = Some(stops_params(&ge));
                ge.picker = None;
            }
            Some(_) => ge.picker = None,
            None if open_picker_at.is_none() && !egui::Popup::is_id_open(ui.ctx(), super::color_picker::popup_id(sid)) => ge.picker = None,
            None => {}
        }
    }
    // Deckkraft eines gewählten Deckkraftstopps (unter der Leiste)
    if let Some((true, i)) = ge.sel
        && ge.drag.is_none()
    {
        let a = ge.opacities.get(i).map(|s| s.1.a() as f64 * 100.0 / 255.0).unwrap_or(100.0);
        let d = Dl::at(o + vec2(16.0, 92.0));
        fields::label(ui, d.r(0.0, 1.0, 40.0, 8.0), &texts::t("pi-opacity"), true);
        if let Some(v) = fields::number(ui, id.with("op"), d.r(42.0, 0.0, 24.0, 11.0), Some(d.r(66.0, 0.0, 9.0, 11.0)), Some(a), (0.0, 100.0), 0, true)
            && let Some(s) = ge.opacities.get_mut(i)
        {
            let a8 = (v * 2.55).round().clamp(0.0, 255.0) as u32;
            s.1 = Argb(a8 << 24 | 0x00ff_ffff);
            out = Some(stops_params(&ge));
        }
    }
    // Fußzeile: Umkehren, Verschieben (grau wie im Original), Winkel
    let fy = o.y + GRADIENT_POPUP.y - 20.0;
    if fields::button(ui, id.with("rev"), Rect::from_min_size(pos2(o.x, fy), vec2(64.0, 20.0)), &texts::t("pi-grad-reverse"), true) {
        out = Some(json!({"reverse": true}));
    }
    let small = t.small_font();
    let mv = Rect::from_min_size(pos2(o.x + 70.0, fy), vec2(14.0, 20.0));
    move_symbol(ui.painter(), mv, c.icon_disabled);
    let _ = fields::number_box(ui, id.with("offset"), Rect::from_min_size(pos2(o.x + 86.0, fy + 2.0), vec2(30.0, 16.0)), None, false);
    ui.painter().text(pos2(o.x + 118.0, fy + 10.0), Align2::LEFT_CENTER, "%", small.clone(), c.text_disabled);
    let ar = Rect::from_min_size(pos2(o.x + 132.0, fy), vec2(14.0, 20.0));
    angle_symbol(ui.painter(), ar, if angle.is_some() { c.icon_fg } else { c.icon_disabled });
    if let Some(v) = fields::number_box(ui, id.with("angle"), Rect::from_min_size(pos2(o.x + 148.0, fy + 2.0), vec2(36.0, 16.0)), angle.map(|a| a.round()), angle.is_some()) {
        out = Some(json!({"angle": v.rem_euclid(360.0)}));
    }
    ui.painter().text(pos2(o.x + 187.0, fy + 9.0), Align2::LEFT_CENTER, "°", small, if angle.is_some() { c.text } else { c.text_disabled });
    ui.data_mut(|d| d.insert_temp(key, ge));
    out
}

/// Symbol „Verschieben“ (Kreuz mit Pfeilspitzen).
fn move_symbol(p: &egui::Painter, r: Rect, col: Color32) {
    let cx = r.center();
    let s = Stroke::new(1.0, col);
    p.line_segment([cx - vec2(5.0, 0.0), cx + vec2(5.0, 0.0)], s);
    p.line_segment([cx - vec2(0.0, 5.0), cx + vec2(0.0, 5.0)], s);
    for (a, b, c2) in [(vec2(-5.0, 0.0), vec2(-3.0, -2.0), vec2(-3.0, 2.0)), (vec2(5.0, 0.0), vec2(3.0, -2.0), vec2(3.0, 2.0)), (vec2(0.0, -5.0), vec2(-2.0, -3.0), vec2(2.0, -3.0)), (vec2(0.0, 5.0), vec2(-2.0, 3.0), vec2(2.0, 3.0))] {
        p.line_segment([cx + a, cx + b], s);
        p.line_segment([cx + a, cx + c2], s);
    }
}

/// Symbol „Winkel“ (∠).
fn angle_symbol(p: &egui::Painter, r: Rect, col: Color32) {
    let s = Stroke::new(1.0, col);
    let base = pos2(r.left() + 2.0, r.center().y + 5.0);
    p.line_segment([base, base + vec2(10.0, 0.0)], s);
    p.line_segment([base, base + vec2(8.0, -9.0)], s);
}

fn stops_params(ge: &GradEdit) -> Value {
    let cs: Vec<Value> = ge.colors.iter().map(|(p, c)| json!([p, ui_commands::hex(*c)])).collect();
    let os: Vec<Value> = ge.opacities.iter().map(|(p, c)| json!([p, ui_commands::hex(*c)])).collect();
    if os.len() >= 2 { json!({"stops": cs, "opacityStops": os}) } else { json!({"stops": cs}) }
}

/// Angezeigter Name eines Musters (Dateiname ohne Endung): übersetzt, wenn die Sprache ihn kennt
/// (`pi-pattern-name-<name>`, deutsch wie im Original, z. B. „Baumwolle“), sonst der Dateiname.
pub fn pattern_label(name: &str) -> String {
    let mut slug = String::new();
    for ch in name.chars().flat_map(|c| c.to_lowercase()) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    let key = format!("pi-pattern-name-{}", slug.trim_end_matches('-'));
    if fp_i18n::has(&key) { texts::t(&key) } else { name.to_string() }
}

/// Einträge der Musterliste (Dateiname, angezeigter Name), nach dem angezeigten Namen geordnet wie
/// im Original („Baumwolle“ … „Ölfarbe 3“).
pub fn pattern_items() -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = pattern_files().iter().map(|(n, _)| (n.clone(), pattern_label(n))).collect();
    v.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()).then_with(|| a.0.cmp(&b.0)));
    v
}

/// Wahl im Muster-Popup.
#[derive(Clone, Debug, PartialEq)]
pub enum PatternChoice {
    /// Ein Muster der Liste (Dateiname ohne Endung).
    Named(String),
    /// „Andere…“: Muster aus einer Datei.
    Other,
    /// Winkel in Grad.
    Angle(f64),
}

/// Breite und Höhe des Inhalts des Muster-Popups (Vorschau 250 × 123 wie im Original).
pub const PATTERN_POPUP: egui::Vec2 = egui::vec2(250.0, 175.0);

/// Muster-Popup wie im Original: „Muster:“ mit der Musterliste (+ Trennlinie, „Andere...“),
/// Vorschau des Musters (250 × 123) und Winkel.
pub fn pattern_editor(ui: &mut Ui, id: Id, current: &str, angle: Option<f64>) -> Option<PatternChoice> {
    let t = Theme::current(ui.ctx());
    let c = &t.colors;
    let mut out = None;
    let (all, _) = ui.allocate_exact_size(PATTERN_POPUP, Sense::hover());
    let o = all.min;
    fields::label(ui, Rect::from_min_size(o + vec2(0.0, 4.0), vec2(44.0, 12.0)), &texts::t("pi-pattern-label"), true);
    let items = pattern_items();
    let mut labels: Vec<String> = items.iter().map(|(_, l)| l.clone()).collect();
    labels.push("-".into());
    labels.push(texts::t("pi-pattern-other"));
    let sel = items.iter().position(|(n, _)| n.eq_ignore_ascii_case(current));
    let text = sel.map(|i| labels[i].clone()).unwrap_or_else(|| pattern_label(current));
    let cr = Rect::from_min_size(o + vec2(46.0, 0.0), vec2(PATTERN_POPUP.x - 46.0, 20.0));
    let resp = fields::combo_box_frame(ui, id.with("list"), cr, &text, true);
    let list_w = fields::list_width(ui, labels.iter().map(|s| s.as_str()), cr.width());
    egui::Popup::menu(&resp).frame(crate::menu::menu_frame(&t)).width(list_w).show(|ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
            for (i, (name, label)) in items.iter().enumerate() {
                if fields::list_item(ui, label, Some(i) == sel).clicked() {
                    if Some(i) != sel {
                        out = Some(PatternChoice::Named(name.clone()));
                    }
                    ui.close();
                }
            }
        });
        let (sr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 7.0), Sense::hover());
        ui.painter().hline(sr.x_range().shrink(4.0), sr.center().y.floor() + 0.5, Stroke::new(1.0, c.menu_separator));
        if fields::list_item(ui, labels.last().map(|s| s.as_str()).unwrap_or_default(), false).clicked() {
            out = Some(PatternChoice::Other);
            ui.close();
        }
    });
    // Vorschau (Kacheln in Originalgröße)
    let pv = Rect::from_min_size(o + vec2(0.0, 26.0), vec2(250.0, 123.0));
    ui.painter().rect_filled(pv, 0.0, c.field_bg);
    if let Some(tex) = pattern_swatch_texture(ui.ctx(), current) {
        let ts = tex.size_vec2();
        if ts.x > 0.0 && ts.y > 0.0 {
            ui.painter().with_clip_rect(pv).image(tex.id(), pv, Rect::from_min_max(pos2(0.0, 0.0), pos2(pv.width() / ts.x, pv.height() / ts.y)), Color32::WHITE);
        }
    }
    ui.painter().rect_stroke(pv, 0.0, Stroke::new(1.0, c.field_border), StrokeKind::Inside);
    // Winkel
    let fy = pv.bottom() + 6.0;
    angle_symbol(ui.painter(), Rect::from_min_size(pos2(o.x + 150.0, fy), vec2(14.0, 20.0)), if angle.is_some() { c.icon_fg } else { c.icon_disabled });
    if let Some(v) = fields::number_box(ui, id.with("angle"), Rect::from_min_size(pos2(o.x + 168.0, fy + 2.0), vec2(40.0, 16.0)), angle.map(|a| a.round()), angle.is_some()) {
        out = Some(PatternChoice::Angle(v.rem_euclid(360.0)));
    }
    ui.painter().text(pos2(o.x + 211.0, fy + 9.0), Align2::LEFT_CENTER, "°", t.small_font(), if angle.is_some() { c.text } else { c.text_disabled });
    out
}

/// Bild eines Musters als Textur (wiederholbar), je Name einmal geladen.
pub fn pattern_swatch_texture(ctx: &egui::Context, name: &str) -> Option<egui::TextureHandle> {
    let key = Id::new(("pi-pattern-tex", name.to_lowercase()));
    if let Some(t) = ctx.data(|d| d.get_temp::<Option<egui::TextureHandle>>(key)) {
        return t;
    }
    let path = pattern_files().iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, p)| p.clone());
    let tex = path.and_then(|p| fp_engine::commands::attrs::load_image_file(&p.to_string_lossy()).ok()).map(|img| {
        let ci = egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_bytes());
        ctx.load_texture(format!("pi-pattern-{name}"), ci, egui::TextureOptions { wrap_mode: egui::TextureWrapMode::Repeat, ..egui::TextureOptions::LINEAR })
    });
    ctx.data_mut(|d| d.insert_temp(key, tex.clone()));
    tex
}

// ---------------------------------------------------------------------- Strichkategorien

/// Strichkategorie-Knopf (14606) mit Menü: „Keine“, Kategorien mit Vorgaben.
/// Liefert `Some(None)` für „Keine“ bzw. `Some(Some(index))` in [`ui_commands::BRUSH_PRESETS`].
/// Gespeicherte Pinselstriche des Dokuments stehen unter „Gespeichert“ mit Indizes ab
/// `BRUSH_PRESETS.len()`. `label` ist der angezeigte Name, `has` = es gibt einen Strich,
/// `preset` = Index der aktuellen Vorgabe.
#[allow(clippy::too_many_arguments)]
pub fn stroke_category(ui: &mut Ui, id: Id, r: Rect, label: &str, has: bool, preset: Option<usize>, enabled: bool, saved: &[String]) -> Option<Option<usize>> {
    let resp = fields::combo_box_frame(ui, id, r, label, enabled);
    // Menü wie im Original (PAR 02/01): „Ohne“ · Trenner · Kategorien ▸ (Untermenü rechts daneben) ·
    // Trenner · „Pinselstrichoptionen…“, so breit wie die Einträge (≈ 220 px). Gebaut aus den
    // Menüknoten der Anwendung; die Wahl kommt als Kennung zurück und wird hier ausgewertet.
    let pick = |i: usize| crate::menu::ItemAction::Command(format!("{STROKE_PICK}{i}"));
    let item = |label: String, i: usize, checked: bool| crate::menu::Node::Item { label, shortcut: None, action: pick(i), enabled: true, checked: Some(checked) };
    let mut nodes = vec![item(texts::t("pi-stroke-cat-none"), STROKE_NONE_ITEM, !has), crate::menu::Node::Separator];
    for cat in texts::brush_categories() {
        let children = ui_commands::BRUSH_PRESETS.iter().enumerate().filter(|(_, b)| b.category == cat).map(|(i, b)| item(texts::brush_preset_label(b), i, has && preset == Some(i))).collect();
        nodes.push(crate::menu::Node::Sub { label: texts::brush_category_label(cat), children, enabled: true });
    }
    if !saved.is_empty() {
        let children = saved.iter().enumerate().map(|(k, n)| item(n.clone(), ui_commands::BRUSH_PRESETS.len() + k, n == label)).collect();
        nodes.push(crate::menu::Node::Sub { label: texts::t("pi-stroke-saved"), children, enabled: true });
    }
    nodes.push(crate::menu::Node::Separator);
    nodes.push(item(texts::t("pi-stroke-options-item"), STROKE_OPTIONS_ITEM, false));
    let mut acts = Vec::new();
    crate::menu::popup_menu(ui, &resp, &nodes, &mut acts);
    acts.iter().find_map(|a| match a {
        crate::app_state::Action::Run(n, _) => n.strip_prefix(STROKE_PICK).and_then(|s| s.parse::<usize>().ok()),
        _ => None,
    })
    .map(|i| if i == STROKE_NONE_ITEM { None } else { Some(i) })
}

/// Kennung der Menüwahl im Strichart-Menü (kein Befehl, wird von [`stroke_category`] ausgewertet).
const STROKE_PICK: &str = "\u{1}stroke-pick:";
/// Eintrag „Ohne“ im Strichart-Menü.
const STROKE_NONE_ITEM: usize = usize::MAX - 1;

/// Rückgabewert von [`stroke_category`] für „Pinselstrichoptionen…“.
pub const STROKE_OPTIONS_ITEM: usize = usize::MAX;

/// Name des eigenen Benutzer-Texturordners in `%APPDATA%\Firepixel` (technisch englisch, §1c).
pub const USER_TEXTURES_DIR: &str = fp_engine::resources::TEXTURES;
/// Name des eigenen Benutzer-Musterordners in `%APPDATA%\Firepixel`.
pub const USER_PATTERNS_DIR: &str = fp_engine::resources::PATTERNS;

/// Bildendungen, die als Textur bzw. Muster gelesen werden.
pub const RESOURCE_IMAGE_EXTS: [&str; 5] = fp_engine::resources::IMAGE_EXTS;

/// Ordner einer Ressourcenart (`kind` = `Textures` bzw. `Patterns`) in Leserichtung: der
/// Ressourcenordner des Originals (`Configuration\<kind>`), dessen Vorlage für den
/// Benutzerordner (`First Run\<kind>`; das Original kopiert sie beim ersten Start in seinen
/// Benutzerordner und listet beide zusammen, gemessen 52 + 74 → 77 Texturen bzw. 50 + 60 → 60
/// Muster) und der eigene Benutzerordner `%APPDATA%\Firepixel\<kind>` (`user_base`). Dieselben
/// Ordner nutzt die Engine, um Texturen und Muster nur per Name zu laden ([`fp_engine::resources`]).
pub fn resource_dirs(original: Option<&std::path::Path>, user_base: Option<&std::path::Path>, kind: &str) -> Vec<std::path::PathBuf> {
    fp_engine::resources::dirs_in(original, user_base, kind)
}

/// Bilddateien der Ordner (Name = Dateiname ohne Endung, Pfad), je Name nur die erste Fundstelle
/// (ohne Rücksicht auf Groß-/Kleinschreibung), nach Namen geordnet. Fehlende Ordner zählen nicht.
pub fn resource_files_in(dirs: &[std::path::PathBuf]) -> Vec<(String, std::path::PathBuf)> {
    fp_engine::resources::files_in(dirs)
}

fn resource_files(kind: &str) -> Vec<(String, std::path::PathBuf)> {
    resource_files_in(&fp_engine::resources::dirs(kind))
}

/// Texturen (Name, Datei) aus dem Ressourcenordner des Originals, dessen Benutzervorlage und dem
/// eigenen Benutzerordner `%APPDATA%\Firepixel\Textures` (nur zur Laufzeit gelesen, nichts
/// eingebettet; Ordner aus `fp_color::install`). Ohne Ordner: leere Liste.
pub fn texture_files() -> &'static [(String, std::path::PathBuf)] {
    static N: std::sync::OnceLock<Vec<(String, std::path::PathBuf)>> = std::sync::OnceLock::new();
    N.get_or_init(|| resource_files(USER_TEXTURES_DIR))
}

/// Datei einer Textur nach ihrem Namen (ohne Rücksicht auf Groß-/Kleinschreibung).
pub fn texture_file(name: &str) -> Option<&'static std::path::Path> {
    texture_files().iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, p)| p.as_path())
}

/// Texturnamen (Dateinamen ohne Endung) aus [`texture_files`].
pub fn texture_names() -> &'static [String] {
    static N: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    N.get_or_init(|| texture_files().iter().map(|(n, _)| n.clone()).collect())
}

/// Musterdateien (Name, Datei) wie [`texture_files`], aus `Patterns`.
pub fn pattern_files() -> &'static [(String, std::path::PathBuf)] {
    static N: std::sync::OnceLock<Vec<(String, std::path::PathBuf)>> = std::sync::OnceLock::new();
    N.get_or_init(|| resource_files(USER_PATTERNS_DIR))
}

/// Inhalt eines Popups als Antwort-Platzhalter (für Tests: Fläche reservieren).
pub fn reserve(ui: &mut Ui, size: egui::Vec2) -> Rect {
    ui.allocate_exact_size(size, Sense::hover()).0
}

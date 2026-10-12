//! Farbfelder (`CSwatchesPanel`, 4140), Farbmischer (`CColorMixer`, 4125) und Farbpalette
//! (Flash-Bedienfeld „Color Palette“, nativ nachgebaut).
//!
//! Farbfeldsätze (14 §2.1) und eigene Farbfelder stehen in den Voreinstellungen und werden über
//! die Befehle `panel.swatches.*` (fp-engine) geändert: „Farbwürfel“ (Web 216 im Würfel),
//! „Kontinuierlicher Farbton“ (Web 216 nach Farbton), Windows- und Macintosh-Systempalette,
//! „Graustufen“ 256 Stufen, „Benutzerdefiniert“. Dateien ACT/ASE/ACO/GIF über fp-color.
//! Sortieren nach Farbe wie `@006741a0` (kleinstes H8, Gleichstand → letzter).

use super::common;
use crate::app_state::{Action, AppState};
use crate::inspector::fields;
use crate::theme::Theme;
use crate::ui_commands;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, Sense, Stroke, StrokeKind, Ui};
use fp_color::{Argb, Rgba8};
use serde_json::json;

/// Farbfeldsätze: Anzeigename und Nummer der Voreinstellung `SwatchesPanelColorPaletteChoice`
/// (0 Farbwürfel, 1 benutzerdefiniert, 2 Graustufen, 3 Mac, 4 Windows, 5 Kontinuierlicher Farbton).
/// Erster Wert: Schlüssel des Anzeigenamens in `bedienfelder.ftl`.
pub const SETS: &[(&str, i64)] = &[
    ("panel-swatches-set-cubes", 0),
    ("panel-swatches-set-continuous", 5),
    ("panel-swatches-set-windows", 4),
    ("panel-swatches-set-mac", 3),
    ("panel-swatches-set-grayscale", 2),
    ("panel-swatches-set-custom", 1),
];

fn hsv(h: f32, s: f32, v: f32) -> Argb {
    let h = h.rem_euclid(360.0) / 60.0;
    let i = h.floor() as i32;
    let f = h - h.floor();
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - s * f), v * (1.0 - s * (1.0 - f)));
    let (r, g, b) = match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    let c = |x: f32| (x * 255.0).round().clamp(0.0, 255.0) as u32;
    Argb(0xff00_0000 | c(r) << 16 | c(g) << 8 | c(b))
}

/// Aktuelle Farbfelder (aus den Voreinstellungen, fp-engine `panel.swatches.*`).
pub fn current_swatches(_ctx: &egui::Context, st: &AppState) -> Vec<Argb> {
    ui_commands::current_swatches(&st.session.prefs)
}

pub use ui_commands::sort_by_color;

/// Farbe übernehmen: Werkzeugfarbe und (bei Auswahl) Objektfarbe.
pub fn apply_color(st: &mut AppState, c: Argb, stroke: bool) {
    st.push(if stroke { Action::SetColors { stroke: Some(c), fill: None } } else { Action::SetColors { stroke: None, fill: Some(c) } });
    if st.session.active().is_some_and(|d| !d.selection.elements.is_empty()) {
        st.run(if stroke { "pi.stroke" } else { "pi.fill" }, json!({"color": ui_commands::hex(c)}));
    }
}

/// Farbfelder-Bedienfeld: Satzauswahl, Schnellmenü (wie das Bedienfeldmenü 6919), Raster.
/// Klick übernimmt die Farbe (Umschalt: Strich), Strg+Klick löscht das Farbfeld.
pub fn swatches(ui: &mut Ui, st: &mut AppState) {
    let theme = Theme::current(ui.ctx());
    let choice = st.session.prefs.get_int(ui_commands::SWATCH_SET_KEY, 0);
    let colors = ui_commands::current_swatches(&st.session.prefs);
    let (head, _) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::hover());
    let names_s: Vec<String> = SETS.iter().map(|s| common::t(s.0)).collect();
    let names: Vec<&str> = names_s.iter().map(|s| s.as_str()).collect();
    // schmale Bedienfelder: Satzauswahl und Menü teilen sich die Breite
    let set_w = (head.width() * 0.6).clamp(60.0, 150.0);
    if let Some(i) = fields::combo(ui, "swatch-set", Rect::from_min_size(head.min + vec2(2.0, 2.0), vec2(set_w, 20.0)), &names, SETS.iter().position(|s| s.1 == choice), true) {
        st.run("panel.swatches.setChoice", json!({"set": SETS[i].1}));
    }
    let menu_r = Rect::from_min_size(pos2(head.left() + set_w + 6.0, head.top() + 2.0), vec2((head.width() - set_w - 10.0).max(40.0), 20.0));
    let resp = fields::combo_box_frame(ui, Id::new("swatch-file"), menu_r, &common::t("panel-swatches-menu"), true);
    egui::Popup::menu(&resp).frame(crate::menu::menu_frame(&theme)).show(|ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        for (label, cmd) in [
            ("panel-swatches-add", "panel.swatches.add"),
            ("panel-swatches-replace", "panel.swatches.replace"),
            ("panel-swatches-save", "panel.swatches.save"),
            ("panel-swatches-clear", "panel.swatches.clear"),
            ("panel-swatches-sort", "panel.swatches.sortByColor"),
        ] {
            if fields::list_item(ui, &common::t(label), false).clicked() {
                st.run(cmd, json!({}));
                ui.close();
            }
        }
        if fields::list_item(ui, &common::t("panel-swatches-add-fill"), false).clicked() {
            st.run("panel.swatches.addColor", json!({"color": ui_commands::hex(Argb(st.session.tools.fill_color.0 | 0xff00_0000))}));
            ui.close();
        }
    });
    // Raster
    let cell = 12.0;
    let avail = ui.available_width() - 4.0;
    let cols = ((avail / cell).floor() as usize).max(1);
    let rows = colors.len().div_ceil(cols).max(1);
    let (rect, resp) = ui.allocate_exact_size(vec2(cols as f32 * cell, rows as f32 * cell), Sense::click_and_drag());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, common::t("panel-swatches-grid")));
    ui.painter().rect_filled(rect, 0.0, theme.colors.line_dark);
    let mut hovered = None;
    for (i, c) in colors.iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.left() + (i % cols) as f32 * cell, rect.top() + (i / cols) as f32 * cell), vec2(cell, cell));
        ui.painter().rect_filled(r.shrink(0.5), 0.0, Color32::from_rgb(c.r(), c.g(), c.b()));
        if resp.hover_pos().is_some_and(|p| r.contains(p)) {
            hovered = Some(i);
            ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0, Color32::WHITE), StrokeKind::Inside);
        }
    }
    if let Some(i) = hovered {
        let c = colors[i];
        resp.clone().on_hover_text(format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b()));
    }
    // Farbfeld auf ein Objekt ziehen: Füllfarbe (Umschalt: Strichfarbe)
    if resp.drag_started()
        && let Some(o) = ui.input(|i| i.pointer.press_origin())
        && let Some(i) = (0..colors.len()).find(|i| Rect::from_min_size(pos2(rect.left() + (i % cols) as f32 * cell, rect.top() + (i / cols) as f32 * cell), vec2(cell, cell)).contains(o))
    {
        super::drag_drop::start(ui.ctx(), super::drag_drop::Payload::Color(colors[i].0));
    }
    if resp.clicked()
        && let Some(i) = hovered
    {
        let m = ui.input(|i| i.modifiers);
        if m.command {
            st.run("panel.swatches.removeColor", json!({"index": i}));
        } else {
            apply_color(st, colors[i], m.shift);
        }
    }
}

/// Farbfelder aus Dateibytes (Endung `act`, `ase`, `aco`, `gif`).
pub fn read_swatch_bytes(ext: &str, bytes: &[u8]) -> Option<Vec<Argb>> {
    let cols: Vec<Rgba8> = match ext {
        "aco" => fp_color::swatches::read_aco(bytes).ok()?.iter().filter_map(|c| c.to_rgba8(None)).collect(),
        _ => fp_color::swatches::read_swatches_original(bytes, None).ok()?,
    };
    Some(cols.into_iter().map(|c| Argb(c.to_argb().0 | 0xff00_0000)).collect())
}

/// Farbfelder als Dateibytes (Endung `act`, `ase`, `aco`).
pub fn write_swatch_bytes(ext: &str, colors: &[Argb]) -> Vec<u8> {
    ui_commands::swatch_file_bytes(ext, colors)
}

// ---------------------------------------------------------------------- Farbmischer

const MIXER_ACTIVE: &str = "mixer-active-stroke";

/// Farbmischer: Strich-/Füllfarbe, Kanäle je Modell (14 §1.7), Farbrampe.
pub fn mixer(ui: &mut Ui, st: &mut AppState) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let stroke_active: bool = ui.data(|d| d.get_temp(Id::new(MIXER_ACTIVE))).unwrap_or(false);
    let model_v = st.session.prefs.get_int("ColorMixerColorModel", 1);
    let model = fp_color::models::MixerModel::from_pref(model_v).unwrap_or_default();
    let (area, _) = ui.allocate_exact_size(vec2(ui.available_width(), 96.0), Sense::hover());
    let (sc, fc) = (st.session.tools.stroke_color, st.session.tools.fill_color);
    let cur = if stroke_active { sc } else { fc };
    // Strich- und Füllfeld (4264/4265) mit Symbolen
    for (k, (col, is_stroke)) in [(sc, true), (fc, false)].iter().enumerate() {
        let r = Rect::from_min_size(pos2(area.left() + 24.0, area.top() + 4.0 + k as f32 * 30.0), vec2(30.0, 22.0));
        let g = if *is_stroke { crate::inspector::glyphs::Glyph::Pencil } else { crate::inspector::glyphs::Glyph::PaintBucket };
        crate::inspector::glyphs::paint(ui.painter(), Rect::from_min_size(pos2(area.left() + 4.0, r.top() + 4.0), vec2(14.0, 14.0)), g, &theme, true);
        let resp = fields::swatch(ui, ("mixer-well", k), r, (col.a() > 0).then_some(*col), true);
        if *is_stroke == stroke_active {
            ui.painter().rect_stroke(r.expand(2.0), 0.0, Stroke::new(1.0, c.focus), StrokeKind::Inside);
        }
        if resp.clicked() {
            ui.data_mut(|d| d.insert_temp(Id::new(MIXER_ACTIVE), *is_stroke));
        }
    }
    // Standardfarben / Tauschen
    let br = Rect::from_min_size(pos2(area.left() + 4.0, area.top() + 66.0), vec2(18.0, 16.0));
    if fields::icon_button(ui, "mixer-default", br, crate::inspector::glyphs::Glyph::FillSolid, true, &common::t("panel-mixer-defaults")).clicked() {
        st.run("color.defaults", json!({}));
    }
    if fields::icon_button(ui, "mixer-swap", br.translate(vec2(22.0, 0.0)), crate::inspector::glyphs::Glyph::Redo, true, &common::t("panel-mixer-swap")).clicked() {
        st.run("color.swap", json!({}));
    }
    // Kanäle
    let labels = model.labels();
    let fields_txt = fp_color::models::mixer_fields(model, cur.r(), cur.g(), cur.b());
    let mut new_fields: Vec<String> = fields_txt.clone();
    let mut edited = false;
    for (i, lab) in labels.iter().enumerate() {
        let y = area.top() + 4.0 + i as f32 * 22.0;
        ui.painter().text(pos2(area.left() + 118.0, y + 10.0), Align2::RIGHT_CENTER, *lab, theme.ui_font(), c.text);
        let r = Rect::from_min_size(pos2(area.left() + 122.0, y), vec2(50.0, 20.0));
        if let Some(v) = fields::text(ui, ("mixer-ch", model_v, i), r, &fields_txt[i], true) {
            new_fields[i] = v;
            edited = true;
        }
    }
    if edited {
        let refs: Vec<&str> = new_fields.iter().map(|s| s.as_str()).collect();
        if let Some((r, g, b)) = fp_color::models::mixer_parse(model, &refs) {
            apply_color(st, Argb(0xff00_0000 | (r as u32) << 16 | (g as u32) << 8 | b as u32), stroke_active);
        }
    }
    let mr = Rect::from_min_size(pos2(area.left() + 180.0, area.top() + 4.0), vec2((area.width() - 186.0).max(60.0), 20.0));
    let (hex_s, gray_s) = (common::t("panel-mixer-model-hex"), common::t("panel-mixer-model-grayscale"));
    if let Some(i) = fields::combo(ui, "mixer-model", mr, &["RGB", hex_s.as_str(), "CMY", "HSB", gray_s.as_str()], Some(model_v.clamp(0, 4) as usize), true) {
        let cmd = ["panel.mixer.rgb", "panel.mixer.hex", "panel.mixer.cmy", "panel.mixer.hsb", "panel.mixer.grayscale"][i.min(4)];
        st.run(cmd, json!({}));
    }
    // Farbrampe (3930)
    let ramp = Rect::from_min_max(pos2(area.left(), area.bottom() - 26.0), pos2(area.right() - 2.0, area.bottom() - 1.0));
    paint_ramp(ui, ramp);
    let rr = ui.interact(ramp, Id::new("mixer-ramp"), Sense::click_and_drag());
    if (rr.clicked() || rr.drag_stopped())
        && let Some(p) = rr.interact_pointer_pos()
    {
        apply_color(st, ramp_color(ramp, p), stroke_active);
    }
}

fn ramp_color(r: Rect, p: egui::Pos2) -> Argb {
    let u = ((p.x - r.left()) / r.width()).clamp(0.0, 1.0);
    let v = ((p.y - r.top()) / r.height()).clamp(0.0, 1.0);
    if u > 0.94 {
        let g = (255.0 * (1.0 - v)).round() as u32;
        return Argb(0xff00_0000 | g << 16 | g << 8 | g);
    }
    let h = u / 0.94 * 360.0;
    if v < 0.5 { hsv(h, v * 2.0, 1.0) } else { hsv(h, 1.0, 1.0 - (v - 0.5) * 2.0) }
}

fn paint_ramp(ui: &Ui, r: Rect) {
    let cols = 64;
    let rows = 8;
    for j in 0..rows {
        for i in 0..cols {
            let cell = Rect::from_min_size(pos2(r.left() + i as f32 * r.width() / cols as f32, r.top() + j as f32 * r.height() / rows as f32), vec2(r.width() / cols as f32 + 0.6, r.height() / rows as f32 + 0.6));
            let c = ramp_color(r, cell.center());
            ui.painter().rect_filled(cell, 0.0, Color32::from_rgb(c.r(), c.g(), c.b()));
        }
    }
}

// ---------------------------------------------------------------------- Farbpalette

const PAL_BASE: &str = "palette-base";

/// Farbpalette: Farbkreis mit Grundfarbe und Harmonien (Komplementär, Triade, Analog, Quadrat).
pub fn palette(ui: &mut Ui, st: &mut AppState) {
    let theme = Theme::current(ui.ctx());
    let base: (f32, f32, f32) = ui.data(|d| d.get_temp(Id::new(PAL_BASE))).unwrap_or((200.0, 0.8, 0.9));
    let (area, _) = ui.allocate_exact_size(vec2(ui.available_width(), 230.0), Sense::hover());
    let radius = (area.width().min(170.0) / 2.0 - 8.0).max(30.0);
    let center = pos2(area.left() + radius + 10.0, area.top() + radius + 8.0);
    let n = 48;
    for i in 0..n {
        let a0 = std::f32::consts::TAU * i as f32 / n as f32;
        let a1 = std::f32::consts::TAU * (i + 1) as f32 / n as f32;
        for ring in 0..5 {
            let r0 = radius * ring as f32 / 5.0;
            let r1 = radius * (ring + 1) as f32 / 5.0;
            let col = hsv(a0.to_degrees(), (ring as f32 + 0.5) / 5.0, base.2);
            let pts = vec![center + vec2(a0.cos(), a0.sin()) * r0, center + vec2(a1.cos(), a1.sin()) * r0, center + vec2(a1.cos(), a1.sin()) * r1, center + vec2(a0.cos(), a0.sin()) * r1];
            ui.painter().add(egui::Shape::convex_polygon(pts, Color32::from_rgb(col.r(), col.g(), col.b()), Stroke::NONE));
        }
    }
    let wheel = Rect::from_center_size(center, vec2(radius * 2.0, radius * 2.0));
    let wr = ui.interact(wheel, Id::new("palette-wheel"), Sense::click_and_drag());
    let mut nb = base;
    if let Some(p) = wr.interact_pointer_pos()
        && (wr.clicked() || wr.dragged())
    {
        let v = p - center;
        nb.0 = v.y.atan2(v.x).to_degrees().rem_euclid(360.0);
        nb.1 = (v.length() / radius).clamp(0.0, 1.0);
    }
    let mark = center + vec2(base.0.to_radians().cos(), base.0.to_radians().sin()) * base.1 * radius;
    ui.painter().circle_stroke(mark, 5.0, Stroke::new(2.0, Color32::WHITE));
    ui.painter().circle_stroke(mark, 6.0, Stroke::new(1.0, Color32::BLACK));
    // Helligkeit
    let br = Rect::from_min_size(pos2(center.x + radius + 14.0, area.top() + 8.0), vec2(14.0, radius * 2.0));
    for k in 0..20 {
        let v = 1.0 - k as f32 / 20.0;
        let col = hsv(base.0, base.1, v);
        ui.painter().rect_filled(Rect::from_min_size(pos2(br.left(), br.top() + k as f32 * br.height() / 20.0), vec2(br.width(), br.height() / 20.0 + 0.5)), 0.0, Color32::from_rgb(col.r(), col.g(), col.b()));
    }
    let bresp = ui.interact(br, Id::new("palette-bright"), Sense::click_and_drag());
    if let Some(p) = bresp.interact_pointer_pos()
        && (bresp.clicked() || bresp.dragged())
    {
        nb.2 = (1.0 - (p.y - br.top()) / br.height()).clamp(0.0, 1.0);
    }
    if nb != base {
        ui.data_mut(|d| d.insert_temp(Id::new(PAL_BASE), nb));
    }
    // Harmonien
    let harmonies: [(&str, Vec<f32>); 4] = [("panel-palette-complementary", vec![0.0, 180.0]), ("panel-palette-triad", vec![0.0, 120.0, 240.0]), ("panel-palette-analogous", vec![-30.0, 0.0, 30.0]), ("panel-palette-square", vec![0.0, 90.0, 180.0, 270.0])];
    let mut y = area.top() + radius * 2.0 + 18.0;
    let mut picked = None;
    for (name, offs) in harmonies.iter() {
        if y + 16.0 > area.bottom() {
            break;
        }
        common::fit_text(ui, Rect::from_min_size(pos2(area.left() + 6.0, y), vec2(78.0, 14.0)), Align2::LEFT_CENTER, &common::t(name), theme.small_font(), theme.colors.text);
        for (k, o) in offs.iter().enumerate() {
            let col = hsv(base.0 + o, base.1, base.2);
            let r = Rect::from_min_size(pos2(area.left() + 86.0 + k as f32 * 22.0, y), vec2(20.0, 14.0));
            ui.painter().rect_filled(r, 0.0, Color32::from_rgb(col.r(), col.g(), col.b()));
            ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0, theme.colors.field_border), StrokeKind::Inside);
            if ui.interact(r, Id::new(("palette-h", *name, k)), Sense::click()).clicked() {
                picked = Some(col);
            }
        }
        y += 18.0;
    }
    if let Some(c) = picked {
        apply_color(st, c, ui.input(|i| i.modifiers.shift));
    }
    if fields::button(ui, "palette-add", Rect::from_min_size(pos2(area.left() + 6.0, area.bottom() - 2.0), vec2(150.0_f32.min(area.width() - 10.0).max(40.0), 20.0)), &common::t("panel-palette-add"), true) {
        for (_, offs) in harmonies.iter() {
            for o in offs {
                st.run("panel.swatches.addColor", json!({"color": ui_commands::hex(hsv(base.0 + o, base.1, base.2))}));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sets_and_files() {
        assert_eq!(ui_commands::swatch_set(0).len(), 216);
        assert_eq!(ui_commands::swatch_set(2).len(), 256);
        assert_eq!(ui_commands::swatch_set(3).len(), 256);
        assert_eq!(ui_commands::swatch_set(4).len(), 256);
        assert_eq!(ui_commands::swatch_set(5).len(), 216);
        let cols = vec![Argb(0xffff_0000), Argb(0xff00_ff00), Argb(0xff00_00ff)];
        for ext in ["act", "ase", "aco"] {
            let b = write_swatch_bytes(ext, &cols);
            assert_eq!(read_swatch_bytes(ext, &b), Some(cols.clone()), "{ext}");
        }
        let sorted = sort_by_color(&[Argb(0xff00_00ff), Argb(0xff80_8080), Argb(0xff00_ff00)]);
        assert_eq!(sorted[0], Argb(0xff80_8080));
    }
}

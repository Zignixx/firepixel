//! Kurven (re\19-kurven-tonwert.md §2): quadratisches Kurvenfeld 208 × 208 mit gepunktetem
//! 4×4-Raster, Kurve punktweise je Spalte, Punkte als 4×4-Quadrate; senkrechte und waagrechte
//! Rampe (Klick kehrt die Darstellung um); Felder Eingabe/Ausgabe für den gewählten Punkt;
//! Kanal-Liste; OK, Abbrechen, Auto, drei Pipetten, Vorschau. Kein Histogramm, kein
//! Laden/Speichern, kein Zurücksetzen (gibt es im Original nicht).
//!
//! Die Mausregeln (§2.6) stecken in [`CurvesDialog::press`], [`CurvesDialog::drag_to`] und
//! [`CurvesDialog::release`] und rechnen in Kurveneinheiten (0…255, ganzzahlig gerundet); die
//! Formeln für Auto und Pipetten liegen in `fp_raster::filters::curve_edit`.

use super::form::{apply_preview, revert_preview, FormDialog};
use super::levels_common::{self as common, Pipette};
use super::{dialog_frame, push_button, refs, t, tl, DialogClose, DIALOG_TITLE_H};
use crate::app_state::AppState;
use crate::inspector::fields;
use egui::{pos2, vec2, Color32, Id, Pos2, Rect, Sense, Shape, Ui};
use fp_raster::filters::curve_edit::{self, round_half_up};
use fp_raster::filters::table::curve_table;
use fp_raster::filters::{CurvePoint, CurvesParams};
use serde_json::json;

/// Zeichen- und Mausfläche des Kurvenfelds (Client, ohne den 1-px-Rahmen).
pub const FIELD: f32 = 208.0;
/// Nähe zur Kurve (Kurveneinheiten) für einen neuen Punkt.
const NEAR: f32 = 4.0;
/// Mindestabstand in x zu den Nachbarn (streng größer).
const GAP: f32 = 4.0;
/// Startschwelle beim Ziehen (streng größer, Kurveneinheiten).
const DRAG_START: f32 = 3.0;
/// Trefferrechteck um einen Punkt (Pixel, `x−3 … x+2`).
const HIT: f32 = 3.0;

/// Laufendes Ziehen eines Punkts.
#[derive(Clone, Debug, PartialEq)]
pub struct Drag {
    /// Index des Punkts (bzw. Einfügestelle, solange er vorläufig entfernt ist).
    pub idx: usize,
    /// Kurvenwert beim Drücken.
    pub start: (f32, f32),
    /// Startschwelle überschritten.
    pub moved: bool,
    /// Punkt vorläufig entfernt (aus der Fläche bzw. an/über den Nachbarn gezogen).
    pub removed: bool,
}

/// Zustand des Kurvendialogs.
#[derive(Clone, Debug)]
pub struct CurvesDialog {
    pub params: CurvesParams,
    /// 0 = RGB, 1 = Rot, 2 = Grün, 3 = Blau.
    pub channel: usize,
    /// Gewählter Punkt (wird nicht hervorgehoben).
    pub selected: Option<usize>,
    /// Werte in den Feldern Eingabe/Ausgabe (nach dem Löschen bleibt der letzte gültige Wert stehen).
    pub shown: (f32, f32),
    pub drag: Option<Drag>,
    /// Darstellung umgekehrt (nur Anzeige).
    pub reversed: bool,
    /// Aufnahmemodus einer Pipette.
    pub picking: Option<Pipette>,
    pub preview: FormDialog,
    /// Histogramme R, G, B des Originalbilds (für Auto).
    pub hist: [[u64; 256]; 3],
    pub live: bool,
    /// Vorhandener Live-Effekt, der bearbeitet wird (ⓘ, KV-3); None = Filter bzw. neuer Effekt.
    pub edit: Option<super::effects::LiveEdit>,
}

impl CurvesDialog {
    /// Neuer Dialog (Grundwerte, nichts gemerkt außer dem Vorschau-Schalter).
    pub fn new(st: &AppState, live: bool) -> CurvesDialog {
        CurvesDialog::with_state(common::channel_histograms(st), live, common::preview_pref(st))
    }

    /// Neuer Dialog mit gegebenen Histogrammen und Vorschau-Zustand (ohne Oberfläche, Tests).
    pub fn with_state(hist: [[u64; 256]; 3], live: bool, preview_on: bool) -> CurvesDialog {
        let base = if live { json!({"live": true}) } else { json!({}) };
        let mut preview = FormDialog::new("curves", &t("dlg-curves-title"), "filter.adjustColor.curves", vec![]).with_base(base).with_preview();
        preview.preview_on = preview_on;
        let mut d = CurvesDialog { params: CurvesParams::default(), channel: 0, selected: Some(0), shown: (0.0, 0.0), drag: None, reversed: false, picking: None, preview, hist, live, edit: None };
        d.sync_fields();
        d
    }

    /// Punkte des gewählten Kanals.
    pub fn points(&self) -> &Vec<CurvePoint> {
        curve_edit::channel_points(&self.params, self.channel)
    }

    fn points_mut(&mut self) -> &mut Vec<CurvePoint> {
        curve_edit::channel_points_mut(&mut self.params, self.channel)
    }

    /// Tabelle des gewählten Kanals (Anzeige, Nähe-Test).
    pub fn table(&self) -> [u8; 256] {
        curve_table(self.points())
    }

    /// Befehlsparameter (alle vier Punktlisten).
    pub fn command_params(&self) -> serde_json::Value {
        if let Some(e) = &self.edit {
            return e.invocation(super::effects::curves_effect_params(&self.params));
        }
        let mut v = serde_json::to_value(&self.params).unwrap_or(json!({}));
        if self.live {
            v["live"] = json!(true);
        }
        v
    }

    fn sync_fields(&mut self) {
        if let Some(p) = self.selected.and_then(|i| self.points().get(i)).copied() {
            self.shown = (p.x, p.y);
        }
    }

    /// Kanal wechseln: Punkt 0 wird gewählt.
    pub fn set_channel(&mut self, c: usize) {
        self.channel = c.min(3);
        self.drag = None;
        self.selected = Some(0);
        self.sync_fields();
    }

    /// Mausposition (relativ zur linken oberen Ecke der 208er-Fläche) → Kurvenwerte, kaufmännisch
    /// gerundet; bei umgekehrter Darstellung gespiegelt. Nicht begrenzt (außerhalb der Fläche
    /// liegen Werte unter 0 bzw. über 255).
    pub fn to_curve(&self, px: f32, py: f32) -> (f32, f32) {
        let mut x = px / FIELD * 255.0;
        let mut y = (FIELD - py) / FIELD * 255.0;
        if self.reversed {
            x = 255.0 - x;
            y = 255.0 - y;
        }
        (round_half_up(x), round_half_up(y))
    }

    /// Kurvenpunkt → Pixel in der Fläche (gerundet).
    pub fn to_px(&self, p: CurvePoint) -> (f32, f32) {
        let (mut x, mut y) = (p.x, p.y);
        if self.reversed {
            x = 255.0 - x;
            y = 255.0 - y;
        }
        ((x / 255.0 * FIELD).round(), (FIELD - y / 255.0 * FIELD).round())
    }

    /// Erster Punkt, dessen Trefferrechteck die Pixelstelle enthält.
    pub fn hit(&self, px: f32, py: f32) -> Option<usize> {
        self.points().iter().position(|p| {
            let (cx, cy) = self.to_px(*p);
            px >= cx - HIT && px < cx + HIT && py >= cy - HIT && py < cy + HIT
        })
    }

    /// Liegt (x, y) nahe der Kurve (±4 in beiden Richtungen)?
    pub fn near_curve(&self, x: f32, y: f32) -> bool {
        let tab = self.table();
        let a = (x - NEAR).max(0.0).trunc() as i32;
        let b = (x + NEAR).min(255.0).trunc() as i32;
        (a..=b).any(|i| {
            let v = tab[i.clamp(0, 255) as usize] as f32;
            v - NEAR <= y && y <= v + NEAR
        })
    }

    /// Maustaste in der Fläche gedrückt (Pixel relativ zur Fläche). Treffer auf einen Punkt
    /// wählt ihn; ein Klick nahe der Kurve fügt einen Punkt ein, wenn er zu beiden Nachbarn mehr
    /// als 4 Abstand hat; sonst passiert nichts.
    pub fn press(&mut self, px: f32, py: f32) {
        let (x, y) = self.to_curve(px, py);
        if let Some(i) = self.hit(px, py) {
            self.selected = Some(i);
            self.drag = Some(Drag { idx: i, start: (x, y), moved: false, removed: false });
            self.sync_fields();
            return;
        }
        if !(0.0..=255.0).contains(&x) || !(0.0..=255.0).contains(&y) || !self.near_curve(x, y) {
            return;
        }
        let pts = self.points();
        let Some(k) = pts.iter().position(|p| p.x >= x) else { return };
        if k == 0 || !(pts[k - 1].x + GAP < x && x < pts[k].x - GAP) {
            return;
        }
        self.points_mut().insert(k, CurvePoint { x, y });
        self.selected = Some(k);
        self.drag = Some(Drag { idx: k, start: (x, y), moved: false, removed: false });
        self.sync_fields();
    }

    /// Maus bei gedrückter Taste bewegt (Pixel relativ zur Fläche, auch außerhalb).
    pub fn drag_to(&mut self, px: f32, py: f32) {
        let (x, y) = self.to_curve(px, py);
        let Some(mut dr) = self.drag.clone() else { return };
        if !dr.moved {
            if (x - dr.start.0).abs() <= DRAG_START && (y - dr.start.1).abs() <= DRAG_START {
                return;
            }
            dr.moved = true;
        }
        let i = dr.idx;
        let n = self.points().len();
        if !dr.removed && (i == 0 || i + 1 == n) {
            // Endpunkte: auf 0…255 begrenzt, Abstand zum Nachbarn mindestens 4, nie löschbar
            let pts = self.points();
            let mut nx = x.clamp(0.0, 255.0);
            if n >= 2 {
                if i == 0 {
                    nx = nx.min(pts[1].x - GAP);
                } else {
                    nx = nx.max(pts[n - 2].x + GAP);
                }
            }
            let ny = y.clamp(0.0, 255.0);
            self.points_mut()[i] = CurvePoint { x: nx, y: ny };
        } else {
            // innere Punkte: außerhalb 0…255 oder an/über den Nachbarn → vorläufig entfernt
            let (left, right) = {
                let pts = self.points();
                if dr.removed { (pts[i - 1].x, pts[i].x) } else { (pts[i - 1].x, pts[i + 1].x) }
            };
            let valid = (0.0..=255.0).contains(&x) && (0.0..=255.0).contains(&y) && left + GAP < x && x < right - GAP;
            match (dr.removed, valid) {
                (false, true) => self.points_mut()[i] = CurvePoint { x, y },
                (false, false) => {
                    self.points_mut().remove(i);
                    dr.removed = true;
                }
                // Rückkehr zwischen die Nachbarn (gleicher Index) → wieder eingefügt
                (true, true) if 0.0 < y && y < 255.0 => {
                    self.points_mut().insert(i, CurvePoint { x, y });
                    dr.removed = false;
                }
                _ => {}
            }
        }
        if !dr.removed {
            self.sync_fields();
        }
        self.drag = Some(dr);
    }

    /// Maustaste losgelassen: ein vorläufig entfernter Punkt bleibt gelöscht.
    pub fn release(&mut self) {
        if let Some(dr) = self.drag.take()
            && dr.removed
        {
            self.selected = None;
        }
    }

    /// Felder übernehmen (§2.7): x auf die Nachbarn, y auf 0…255 begrenzt; verletzt der Punkt den
    /// Mindestabstand > 4, ändert sich nichts. Abweichung: das Feld zeigt danach den begrenzten
    /// Wert (das Original lässt den getippten Text stehen).
    pub fn commit_fields(&mut self, input: f64, output: f64) {
        let Some(i) = self.selected else { return };
        let pts = self.points().clone();
        if i >= pts.len() {
            return;
        }
        let left = if i > 0 { Some(pts[i - 1].x) } else { None };
        let right = pts.get(i + 1).map(|p| p.x);
        let x = round_half_up((input as f32).clamp(left.unwrap_or(0.0), right.unwrap_or(255.0)));
        let y = round_half_up((output as f32).clamp(0.0, 255.0));
        let ok = left.is_none_or(|l| l + GAP < x) && right.is_none_or(|r| x < r - GAP);
        if ok {
            self.points_mut()[i] = CurvePoint { x, y };
        }
        self.sync_fields();
    }

    /// „Auto“ (§2.9): Kanalkurven aus den 0,5-%-Grenzen, RGB Identität; Punkt 0 gewählt.
    pub fn auto(&mut self) {
        self.params = curve_edit::auto_points(&self.hist);
        self.drag = None;
        self.selected = Some(0);
        self.sync_fields();
    }

    /// Pipette mit der aufgenommenen (angezeigten) Farbe anwenden (§2.9); Punkt 0 gewählt.
    pub fn apply_pipette(&mut self, which: Pipette, c: [u8; 3]) {
        curve_edit::apply_pipette(&mut self.params, which, c);
        self.picking = None;
        self.drag = None;
        self.selected = Some(0);
        self.sync_fields();
    }
}

/// Senkrechter bzw. waagrechter Graukeil.
fn ramp(ui: &Ui, r: Rect, vertical: bool, dark_first: bool) {
    let (a, b) = if dark_first { (Color32::BLACK, Color32::WHITE) } else { (Color32::WHITE, Color32::BLACK) };
    let mut mesh = egui::epaint::Mesh::default();
    if vertical {
        mesh.colored_vertex(r.left_top(), a);
        mesh.colored_vertex(r.right_top(), a);
        mesh.colored_vertex(r.left_bottom(), b);
        mesh.colored_vertex(r.right_bottom(), b);
    } else {
        mesh.colored_vertex(r.left_top(), a);
        mesh.colored_vertex(r.right_top(), b);
        mesh.colored_vertex(r.left_bottom(), a);
        mesh.colored_vertex(r.right_bottom(), b);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 2, 3);
    ui.painter().add(Shape::mesh(mesh));
    ui.painter().rect_stroke(r, 0.0, egui::Stroke::new(1.0, Color32::from_gray(160)), egui::StrokeKind::Inside);
}

/// Umkehr-Symbol ◁▶ (bzw. ◀▷) in der Mitte der waagrechten Rampe.
fn swap_symbol(ui: &Ui, c: Pos2, reversed: bool) {
    let bg = Rect::from_center_size(c, vec2(22.0, 13.0));
    ui.painter().rect_filled(bg, 0.0, Color32::from_gray(240));
    let left = vec![pos2(c.x - 9.0, c.y), pos2(c.x - 2.0, c.y - 5.0), pos2(c.x - 2.0, c.y + 5.0)];
    let right = vec![pos2(c.x + 9.0, c.y), pos2(c.x + 2.0, c.y - 5.0), pos2(c.x + 2.0, c.y + 5.0)];
    let s = egui::Stroke::new(1.0, Color32::BLACK);
    let (filled, hollow) = if reversed { (left, right) } else { (right, left) };
    ui.painter().add(Shape::convex_polygon(filled, Color32::BLACK, s));
    ui.painter().add(Shape::closed_line(hollow, s));
}

/// Kurvenfeld zeichnen (§2.3): Fläche, gepunktetes Raster, Kurve je Spalte, Punkte 4 × 4.
fn paint_field(ui: &Ui, d: &CurvesDialog, inner: Rect) {
    let p = ui.painter_at(inner);
    p.rect_filled(inner, 0.0, Color32::from_gray(240));
    let w = FIELD as i32;
    let px = |x: i32, y: i32, c: Color32| p.rect_filled(Rect::from_min_size(inner.min + vec2(x as f32, y as f32), vec2(1.0, 1.0)), 0.0, c);
    let grid = Color32::from_gray(0x80);
    for k in 1..4 {
        let at = (w >> 2) * k;
        for s in (0..w).step_by(2) {
            px(at, s, grid);
            px(s, at, grid);
        }
    }
    let tab = d.table();
    for i in 0..w {
        let v = tab[round_half_up(i as f32 / FIELD * 255.0).clamp(0.0, 255.0) as usize] as f32;
        let y = (FIELD - (v / 255.0 * FIELD).round()).min(FIELD - 1.0) as i32;
        let (x, y) = if d.reversed { (w - 1 - i, w - 1 - y) } else { (i, y) };
        px(x, y, Color32::BLACK);
    }
    for q in d.points() {
        let (cx, cy) = d.to_px(*q);
        p.rect_filled(Rect::from_min_size(inner.min + vec2(cx - 2.0, cy - 2.0), vec2(4.0, 4.0)), 0.0, Color32::BLACK);
    }
}

/// Zeigt den Kurvendialog.
pub fn show_curves(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.curves.clone() else { return };
    let mut result = None;
    let channels = tl(&common::CHANNELS);
    let (ch_l, in_l, out_l, auto_l, prev_l, ok_l, cancel_l) = (t("dlg-channel"), t("dlg-curves-input"), t("dlg-curves-output"), t("dlg-curves-auto"), t("dlg-curves-preview"), t("btn-ok"), t("btn-cancel"));
    let (ch_tip, in_tip, out_tip, auto_tip) = (t("dlg-adjust-channel-tip"), t("dlg-curves-input-tip"), t("dlg-curves-output-tip"), t("dlg-curves-auto-tip"));
    // im Aufnahmemodus beendet Esc nur die Aufnahme
    if d.picking.is_some() && common::consume_escape(ctx) {
        d.picking = None;
    }
    let mut dialog_rect = None;
    let mut pref_change = None;
    // Knopfspalte: 98 px wie im Original, breiter nur bei längeren Texten (andere Sprachen)
    let bw = super::buttons_w(ctx, &[&ok_l, &cancel_l, &auto_l], 98.0).ceil();
    let combo_x = 81f32.max(3.0 + 10.0 + super::text_w(ctx, &ch_l) + 8.0).ceil();
    let col = 273f32.max(combo_x + 143.0 + 12.0);
    let pw = (18.0 + super::text_w(ctx, &prev_l)).max(80.0).ceil();
    let client_w = 374f32.max(col + bw + 3.0).max(col - 2.0 + pw + 3.0);
    let r = dialog_frame(ctx, "fp-curves", &t("dlg-curves-title"), vec2(client_w, 317.0), |ui, body| {
        dialog_rect = Some(Rect::from_min_max(body.min - vec2(0.0, DIALOG_TITLE_H), body.max));
        let o = body.min;
        let at = |x: f32, y: f32, w: f32, h: f32| Rect::from_min_size(o + vec2(x, y), vec2(w, h));
        // Gruppe „Kanal:“ mit der Liste in der Überschrift
        super::group_box(ui, at(3.0, 5.0, 258f32.max(combo_x + 146.0), 309.0), &ch_l);
        let cr = at(combo_x, 2.0, 143.0, 21.0);
        if let Some(c) = fields::combo(ui, "cv-ch", cr, &refs(&channels), Some(d.channel), true) {
            d.set_channel(c);
        }
        let _ = ui.interact(cr, Id::new("cv-ch-tip"), Sense::hover()).on_hover_text(&ch_tip);
        // Rampen (Klick kehrt die Darstellung um)
        let vr = at(9.0, 28.0, 14.0, 210.0);
        let hr = at(33.0, 244.0, 210.0, 15.0);
        ramp(ui, vr, true, d.reversed);
        ramp(ui, hr, false, !d.reversed);
        swap_symbol(ui, hr.center(), d.reversed);
        let flip_v = ui.interact(vr, Id::new("cv-ramp-v"), Sense::click()).clicked();
        let flip_h = ui.interact(hr, Id::new("cv-ramp-h"), Sense::click()).clicked();
        if (flip_v || flip_h) && d.picking.is_none() {
            d.reversed = !d.reversed;
        }
        // Kurvenfeld: 1-px-Rahmen, Fläche 208 × 208
        let frame = at(33.0, 28.0, FIELD + 2.0, FIELD + 2.0);
        ui.painter().rect_stroke(frame, 0.0, egui::Stroke::new(1.0, Color32::from_gray(100)), egui::StrokeKind::Inside);
        let inner = frame.shrink(1.0);
        let resp = ui.interact(inner, Id::new("cv-field"), Sense::click_and_drag());
        let (pressed, released, pos) = ui.input(|i| (i.pointer.primary_pressed(), i.pointer.primary_released(), i.pointer.interact_pos()));
        if d.picking.is_none() {
            if let Some(p) = pos {
                let rel = p - inner.min;
                if pressed && ui.rect_contains_pointer(inner) {
                    d.press(rel.x, rel.y);
                } else if d.drag.is_some() {
                    d.drag_to(rel.x, rel.y);
                }
                if released && d.drag.is_some() {
                    d.drag_to(rel.x, rel.y);
                    d.release();
                }
                if resp.hovered() && d.drag.is_none() {
                    let (x, y) = d.to_curve(rel.x, rel.y);
                    if d.hit(rel.x, rel.y).is_some() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Move);
                    } else if d.near_curve(x, y) {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                    }
                }
            }
        } else {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        }
        paint_field(ui, &d, inner);
        // Felder Eingabe/Ausgabe (gewählter Punkt)
        let (inr, outr) = (at(214.0, 263.0, 36.0, 20.0), at(214.0, 286.0, 36.0, 20.0));
        fields::label_right(ui, at(15.0, 266.0, 194.0, 13.0), &in_l.replace('&', ""), true);
        fields::label_right(ui, at(15.0, 289.0, 194.0, 13.0), &out_l.replace('&', ""), true);
        let edit = d.selected.is_some();
        if let Some(v) = fields::number(ui, "cv-in", inr, None, Some(d.shown.0 as f64), (0.0, 99999.0), 0, edit) {
            d.commit_fields(v, d.shown.1 as f64);
        }
        if let Some(v) = fields::number(ui, "cv-out", outr, None, Some(d.shown.1 as f64), (0.0, 99999.0), 0, edit) {
            d.commit_fields(d.shown.0 as f64, v);
        }
        let _ = ui.interact(inr, Id::new("cv-in-tip"), Sense::hover()).on_hover_text(&in_tip);
        let _ = ui.interact(outr, Id::new("cv-out-tip"), Sense::hover()).on_hover_text(&out_tip);
        // Knopfspalte rechts
        if push_button(ui, at(col, 11.0, bw, 23.0), &ok_l, true, true) {
            result = Some(true);
        }
        if push_button(ui, at(col, 44.0, bw, 23.0), &cancel_l, false, true) {
            result = Some(false);
        }
        let ar = at(col, 73.0, bw, 23.0);
        if push_button(ui, ar, &auto_l, false, true) {
            d.picking = None;
            d.auto();
        }
        let _ = ui.interact(ar, Id::new("cv-auto-tip"), Sense::hover()).on_hover_text(&auto_tip);
        if let Some(p) = common::pipette_buttons(ui, "cv", {
            let px = o.x + col + 13.0 + (bw - 98.0) / 2.0;
            [px, px + 23.0, px + 47.0]
        }, o.y + 104.0, d.picking) {
            d.picking = Some(p);
        }
        let pr = at(col - 2.0, 136.0, pw, 16.0);
        if let Some(b) = fields::checkbox(ui, "cv-prev", pr, &prev_l.replace('&', ""), d.preview.preview_on, true) {
            d.preview.preview_on = b;
            pref_change = Some(b);
        }
    });
    if let Some(b) = pref_change {
        common::set_preview_pref(st, b);
    }
    let mut cancelled = r == DialogClose::Cancelled;
    // Aufnahme: ein Klick außerhalb des Dialogs trifft den Hintergrund des modalen Dialogs; er nimmt
    // die angezeigte Farbe auf, statt den Dialog zu schließen
    if let (Some(which), Some(rect)) = (d.picking, dialog_rect)
        && let Some(p) = common::outside_click(ctx, rect)
    {
        cancelled = false;
        if let Some(c) = common::displayed_color_at(st, p) {
            d.apply_pipette(which, c);
        }
    }
    finish(ctx, st, &mut d, result.or(if cancelled { Some(false) } else { None }));
    if result.is_some() || cancelled {
        st.dialogs.curves = None;
    } else {
        st.dialogs.curves = Some(d);
    }
}

/// Vorschau und Abschluss: Vorschau erst nach dem Loslassen der Maus (§2.6), OK = genau ein
/// Verlaufsschritt, Abbrechen nimmt die Vorschau ohne Verlaufsschritt zurück.
fn finish(ctx: &egui::Context, st: &mut AppState, d: &mut CurvesDialog, result: Option<bool>) {
    let params = d.command_params();
    let pv = &mut d.preview;
    pv.base = params.clone();
    match result {
        Some(true) => {
            if !(pv.applied.is_some() && pv.applied_params.as_ref() == Some(&params)) {
                revert_preview(st, pv);
                st.run(&pv.command.clone(), params);
            }
        }
        Some(false) => revert_preview(st, pv),
        None => {
            let busy = d.drag.is_some() || ctx.input(|i| i.pointer.any_down());
            if pv.preview_on && !busy {
                apply_preview(st, pv);
            } else if !pv.preview_on {
                revert_preview(st, pv);
            }
        }
    }
}

//! Kennlinien der Schieber wie im Original (gemeinsam für Dialog-Schieberleisten und die
//! Popup-Schieber ▾ des Inspektors und der Dialoge).
//!
//! Das Original bildet viele Feldwerte nicht linear auf die Schieberposition ab: kleine Werte
//! fein, große grob. Gemessen über die Trackbar-Nachrichten (`re\_testdateien\ux\paritaet\dialoge\
//! o_schieber.txt`, Befund UX-B): Feldwert setzen, dann die Position lesen. Eine Kennlinie ist eine
//! Liste von Stützstellen (Position, Wert); zwischen zwei Stützstellen ist sie linear, die
//! Position ist ganzzahlig, damit ergibt sich die Schrittweite je Abschnitt von selbst.

/// Kennlinie eines Schiebers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Scale {
    /// Position = Wert über den ganzen Feldbereich (Schritt = `step`).
    Linear { step: f64 },
    /// Stückweise linear über Stützstellen (Position, Wert), Positionen aufsteigend ab 0.
    /// Werte außerhalb der Stützstellen bleiben am Ende stehen (das Feld darf weiter reichen).
    Piecewise(&'static [(f64, f64)]),
}

/// Gaußscher Weichzeichner und Pixelradius von Unscharf maskieren (0 … 145):
/// 0,1–5 in 0,1er-, 5–10 in 0,2er-, 10–50 in 1er-, 50–100 in 5er-, 100–250 in 7,5er-Schritten.
pub const BLUR_RADIUS: Scale = Scale::Piecewise(&[
    (0.0, 0.0),
    (50.0, 5.0),
    (75.0, 10.0),
    (115.0, 50.0),
    (125.0, 100.0),
    (145.0, 250.0),
]);
/// Unscharf maskieren, Menge: Position = Wert bis 170, darüber steht der Schieber am Ende (Feld bis 500).
pub const SHARPEN_AMOUNT: Scale = Scale::Piecewise(&[(0.0, 0.0), (170.0, 170.0)]);
/// Schwellenwert (Schieber 0 … 172, Position = Wert; das Feld reicht bis 255).
pub const THRESHOLD: Scale = Scale::Piecewise(&[(0.0, 0.0), (172.0, 172.0)]);
/// Farbton: Position = Wert + 180 (0 … 360).
pub const HUE: Scale = Scale::Piecewise(&[(0.0, -180.0), (360.0, 180.0)]);
/// Sättigung: linear wie Helligkeit, Position = Wert + 100 (0 … 200, 0 in der Mitte, 50 → ¾).
/// Gemessen von PM-6 (Trackbar gelesen, PARITAET_B D41); die frühere Annahme „0 bei ¼“ war falsch.
pub const SATURATION: Scale = Scale::Piecewise(&[(0.0, -100.0), (200.0, 100.0)]);

impl Default for Scale {
    fn default() -> Self {
        Scale::Linear { step: 1.0 }
    }
}

impl Scale {
    /// Linear mit Schritt 1 bzw. 0,1.
    pub fn linear(decimals: usize) -> Scale {
        Scale::Linear {
            step: if decimals == 0 {
                1.0
            } else {
                10f64.powi(-(decimals as i32))
            },
        }
    }

    /// Größte Schieberposition (die kleinste ist 0).
    pub fn max_pos(&self, min: f64, max: f64) -> f64 {
        match self {
            Scale::Linear { step } => ((max - min) / step.max(1e-9)).round(),
            Scale::Piecewise(pts) => pts.last().map(|p| p.0).unwrap_or(0.0),
        }
    }

    /// Schieberposition eines Feldwerts (geklemmt auf den Schieberbereich, ganzzahlig).
    pub fn to_pos(&self, v: f64, min: f64, max: f64) -> f64 {
        match self {
            Scale::Linear { step } => ((v.clamp(min, max) - min) / step.max(1e-9)).round(),
            Scale::Piecewise(pts) => {
                let (Some(first), Some(last)) = (pts.first(), pts.last()) else {
                    return 0.0;
                };
                if v <= first.1 {
                    return first.0;
                }
                if v >= last.1 {
                    return last.0;
                }
                for w in pts.windows(2) {
                    let ((p0, v0), (p1, v1)) = (w[0], w[1]);
                    if v <= v1 {
                        return (p0 + (v - v0) / (v1 - v0) * (p1 - p0)).round();
                    }
                }
                last.0
            }
        }
    }

    /// Feldwert einer Schieberposition (innerhalb von `min … max`).
    pub fn to_value(&self, pos: f64, min: f64, max: f64) -> f64 {
        let v = match self {
            Scale::Linear { step } => min + pos.round() * step,
            Scale::Piecewise(pts) => {
                let pos = pos.round();
                let mut out = pts.last().map(|p| p.1).unwrap_or(min);
                for w in pts.windows(2) {
                    let ((p0, v0), (p1, v1)) = (w[0], w[1]);
                    if pos <= p1 {
                        out = v0 + (pos - p0).max(0.0) / (p1 - p0) * (v1 - v0);
                        break;
                    }
                }
                out
            }
        };
        // Rundungsrauschen der Schritte (0,1 · 41 = 4,1000000001) entfernen
        ((v * 1e6).round() / 1e6).clamp(min, max)
    }
}

/// Schieber mit Kennlinie: bewegt intern die ganzzahlige Position und rechnet in den Feldwert um.
/// Liefert die Antwort des egui-Schiebers (`changed`, `dragged`, `drag_stopped`).
pub fn slider(
    ui: &mut egui::Ui,
    value: &mut f64,
    range: (f64, f64),
    scale: Scale,
) -> egui::Response {
    let (min, max) = (range.0.min(range.1), range.0.max(range.1));
    let mut pos = scale.to_pos(*value, min, max);
    let r = crate::theme::with_field_widgets(ui, |ui| {
        ui.add(
            egui::Slider::new(&mut pos, 0.0..=scale.max_pos(min, max))
                .step_by(1.0)
                .show_value(false),
        )
    });
    if r.changed() {
        *value = scale.to_value(pos, min, max);
    }
    r
}

/// Wie [`slider`], aber **kein Tab-Stopp** (Original D41: „Tab nur Felder“, CT-A-07): `egui::Slider`
/// meldet sich immer für den Tastaturfokus an; diese Leiste reagiert nur auf die Maus und zeichnet
/// sich wie die egui-Leiste (Schiene, Füllung bis zum Griff, Griff).
pub fn slider_no_tab(
    ui: &mut egui::Ui,
    value: &mut f64,
    range: (f64, f64),
    scale: Scale,
) -> egui::Response {
    let (min, max) = (range.0.min(range.1), range.0.max(range.1));
    let max_pos = scale.max_pos(min, max);
    let mut pos = scale.to_pos(*value, min, max);
    let r = crate::theme::with_field_widgets(ui, |ui| {
        let thickness = ui
            .text_style_height(&egui::TextStyle::Body)
            .max(ui.spacing().interact_size.y);
        let size = egui::vec2(ui.spacing().slider_width, thickness);
        let mut resp = ui.allocate_response(size, egui::Sense::CLICK | egui::Sense::DRAG);
        let rect = resp.rect;
        let shape = ui.style().visuals.handle_shape;
        let radius = rect.height() / 2.5;
        let reach = match shape {
            egui::style::HandleShape::Circle => radius,
            egui::style::HandleShape::Rect { aspect_ratio } => radius * aspect_ratio,
        };
        let span = rect.x_range().shrink(reach);
        if let Some(p) = resp.interact_pointer_pos() {
            let t = if span.span() > 0.0 {
                ((p.x - span.min) / span.span()).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let np = (f64::from(t) * max_pos).round();
            if np != pos {
                pos = np;
                resp.mark_changed();
            }
        }
        if ui.is_rect_visible(rect) {
            let visuals = ui.style().interact(&resp);
            let wv = &ui.visuals().widgets;
            let rail_r = (ui.spacing().slider_rail_height / 2.0).max(0.0);
            let rail = egui::Rect::from_min_max(
                egui::pos2(rect.left(), rect.center().y - rail_r),
                egui::pos2(rect.right(), rect.center().y + rail_r),
            );
            let corner = wv.inactive.corner_radius;
            ui.painter().rect_filled(rail, corner, wv.inactive.bg_fill);
            let t = if max_pos > 0.0 {
                (pos / max_pos).clamp(0.0, 1.0) as f32
            } else {
                0.0
            };
            let center = egui::pos2(span.min + t * span.span(), rail.center().y);
            if ui.visuals().slider_trailing_fill {
                let mut fill = rail;
                fill.max.x = center.x + corner.nw as f32;
                ui.painter()
                    .rect_filled(fill, corner, ui.visuals().selection.bg_fill);
            }
            match shape {
                egui::style::HandleShape::Circle => {
                    ui.painter().circle(
                        center,
                        radius + visuals.expansion,
                        visuals.bg_fill,
                        visuals.fg_stroke,
                    );
                }
                egui::style::HandleShape::Rect { aspect_ratio } => {
                    let v = egui::vec2(radius * aspect_ratio, radius)
                        + egui::Vec2::splat(visuals.expansion);
                    ui.painter().rect(
                        egui::Rect::from_center_size(center, 2.0 * v),
                        visuals.corner_radius,
                        visuals.bg_fill,
                        visuals.fg_stroke,
                        egui::StrokeKind::Inside,
                    );
                }
            }
        }
        resp.widget_info(|| egui::WidgetInfo::slider(ui.is_enabled(), pos, ""));
        resp
    });
    if r.changed() {
        *value = scale.to_value(pos, min, max);
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blur_radius_matches_measured_positions() {
        let (lo, hi) = (0.1, 250.0);
        for (v, p) in [
            (0.1, 1.0),
            (4.1, 41.0),
            (5.0, 50.0),
            (10.0, 75.0),
            (20.0, 85.0),
            (50.0, 115.0),
            (100.0, 125.0),
            (250.0, 145.0),
        ] {
            assert_eq!(BLUR_RADIUS.to_pos(v, lo, hi), p, "Wert {v}");
            assert_eq!(BLUR_RADIUS.to_value(p, lo, hi), v, "Position {p}");
        }
        assert_eq!(BLUR_RADIUS.max_pos(lo, hi), 145.0);
        // Schrittweiten je Abschnitt
        assert_eq!(BLUR_RADIUS.to_value(51.0, lo, hi), 5.2);
        assert_eq!(BLUR_RADIUS.to_value(116.0, lo, hi), 55.0);
        assert_eq!(BLUR_RADIUS.to_value(126.0, lo, hi), 107.5);
        // Position 0 ergibt den Feldminimalwert
        assert_eq!(BLUR_RADIUS.to_value(0.0, lo, hi), 0.1);
    }

    #[test]
    fn sharpen_amount_clamps_at_170() {
        for (v, p) in [
            (1.0, 1.0),
            (10.0, 10.0),
            (50.0, 50.0),
            (100.0, 100.0),
            (250.0, 170.0),
            (500.0, 170.0),
        ] {
            assert_eq!(SHARPEN_AMOUNT.to_pos(v, 1.0, 500.0), p);
        }
    }

    #[test]
    fn hue_and_saturation() {
        assert_eq!(HUE.to_pos(0.0, -180.0, 180.0), 180.0);
        assert_eq!(SATURATION.to_pos(0.0, -100.0, 100.0), 100.0);
        assert_eq!(SATURATION.to_pos(50.0, -100.0, 100.0), 150.0);
        assert_eq!(SATURATION.to_value(0.0, -100.0, 100.0), -100.0);
        assert_eq!(SATURATION.to_value(200.0, -100.0, 100.0), 100.0);
    }

    #[test]
    fn linear_roundtrip() {
        let s = Scale::linear(0);
        for v in [0.0, 1.0, 37.0, 100.0] {
            assert_eq!(s.to_value(s.to_pos(v, 0.0, 100.0), 0.0, 100.0), v);
        }
        let s = Scale::linear(1);
        assert_eq!(s.to_value(s.to_pos(4.1, 0.1, 250.0), 0.1, 250.0), 4.1);
    }
}

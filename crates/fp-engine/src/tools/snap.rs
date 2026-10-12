//! Fang (10 §2.2, §6 „zentrale Snapper-Instanz“): Werkzeuge rasten einen Punkt bzw. ein bewegtes
//! Rechteck vor dem Setzen ein – an Hilfslinien (`SnapToGuides`), am Raster (`SnapToGrids`), an
//! intelligenten Hilfslinien (`SnapToSmartGuides`: Kanten und Mitten anderer Objekte sowie Rand und
//! Mitte der Seite) und an ganzen Pixeln. Fangabstand `SnapDistance` Bildschirmpixel (Vorgabe 5),
//! umgerechnet in Dokumentpixel über den Zoom, einschließlich (≤). Die Rückmeldung (magentafarbene Linien wie in CS6,
//! Farbe `SmartGuideColor`) liefert der Fang als [`Overlay::SmartGuide`]; Abstände zu Nachbarn als
//! [`Overlay::Label`].

use super::{fmt_num, Overlay, ToolContext};
use crate::docops;
use fp_geom::{Point, Rect};
use fp_model::layer::LayerKind;
use fp_model::ElementId;

/// Herkunft einer Fanglinie.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Source {
    Guide,
    Grid,
    /// Kante/Mitte eines Objekts oder der Seite (Ausdehnung für die Linie).
    Smart(Rect),
}

#[derive(Clone, Copy, Debug)]
struct Hit {
    /// Zielwert auf der Achse.
    value: f32,
    /// Abstand (Betrag).
    dist: f32,
    /// Vorrang bei gleichem Abstand (kleiner gewinnt).
    rank: u8,
    source: Source,
}

/// Ergebnis des Punktfangs.
#[derive(Clone, Debug, PartialEq)]
pub struct Snapped {
    pub point: Point,
    /// Rückmeldung (intelligente Hilfslinien).
    pub guides: Vec<Overlay>,
    /// Waagerecht bzw. senkrecht eingerastet.
    pub snapped_x: bool,
    pub snapped_y: bool,
}

/// Fang für ein Ereignis (Werte aus Ansicht, Voreinstellungen und Dokument).
#[derive(Clone, Debug)]
pub struct Snapper {
    tol: f32,
    guides_x: Vec<f32>,
    guides_y: Vec<f32>,
    grid: Option<(Point, Point)>,
    targets: Vec<Rect>,
    page: Option<Rect>,
    smart: bool,
    show_smart: bool,
    pixels: bool,
}

impl Snapper {
    /// Fang mit allen eingeschalteten Zielen; `exclude` = bewegte Objekte (keine Ziele).
    pub fn new(ctx: &ToolContext, exclude: &[ElementId]) -> Snapper {
        let v = ctx.view;
        let zoom = ctx.zoom.max(1e-6) as f32;
        let tol = ctx.prefs.get_int("SnapDistance", 5).clamp(1, 50) as f32 / zoom;
        let page = ctx.doc.pages.get(ctx.page);
        let (mut guides_x, mut guides_y) = (Vec::new(), Vec::new());
        if v.snap_to_guides && v.show_guides
            && let Some(g) = page.and_then(|p| p.root.guides.as_ref())
        {
            guides_x = g.vertical.clone();
            guides_y = g.horizontal.clone();
        }
        let grid = if v.snap_to_grid {
            page.map(|p| {
                let g = p.root.grid.clone().unwrap_or_default();
                (g.origin, Point::new(g.size.x.max(1.0), g.size.y.max(1.0)))
            })
        } else {
            None
        };
        let smart = v.snap_to_smart_guides && ctx.has_document;
        let mut targets = Vec::new();
        let page_rect = page.map(|p| Rect::new(p.canvas.origin.x, p.canvas.origin.y, p.canvas.origin.x + p.canvas.width, p.canvas.origin.y + p.canvas.height));
        if smart {
            for (_, l) in docops::own_layers(ctx.doc, ctx.page) {
                if l.kind == LayerKind::Web || !l.is_visible() {
                    continue;
                }
                let ci = if l.shared_across_frames { 0 } else { ctx.frame };
                let Some(cel) = l.cels.get(ci) else { continue };
                for e in &cel.elements {
                    if targets.len() >= 4000 {
                        break;
                    }
                    if !e.common.visible || exclude.contains(&e.id) {
                        continue;
                    }
                    if let Some(b) = docops::element_bounds(ctx.doc, e)
                        && b.is_finite()
                    {
                        targets.push(b);
                    }
                }
            }
        }
        Snapper {
            tol,
            guides_x,
            guides_y,
            grid,
            targets,
            page: page_rect,
            smart,
            show_smart: v.show_smart_guides,
            pixels: ctx.prefs.get_bool("SnapToPixelGrid", true),
        }
    }

    /// Ohne intelligente Hilfslinien (Pixelauswahl, Freihand).
    pub fn without_smart(mut self) -> Snapper {
        self.smart = false;
        self
    }

    /// Ohne Einrasten an ganzen Pixeln.
    pub fn without_pixels(mut self) -> Snapper {
        self.pixels = false;
        self
    }

    /// Verschiebung auf ganze Pixel runden (wenn eingeschaltet).
    pub fn round_delta(&self, dx: f32, dy: f32) -> (f32, f32) {
        if self.pixels { (dx.round(), dy.round()) } else { (dx, dy) }
    }

    fn smart_rects(&self) -> impl Iterator<Item = &Rect> {
        self.targets.iter().chain(self.page.iter())
    }

    /// Beste Fanglinie für einen Wert einer Achse (`vertical` = x-Achse).
    fn best(&self, val: f32, vertical: bool) -> Option<Hit> {
        let mut best: Option<Hit> = None;
        let mut offer = |h: Hit| {
            // Fangabstand einschließlich: im Original rastet ein Abstand von genau 5 Bildschirmpixeln
            // noch ein, 6 nicht mehr (PARITAET_A L16, gemessen)
            if h.dist <= self.tol + 1e-4 && best.is_none_or(|b| h.dist < b.dist - 1e-6 || ((h.dist - b.dist).abs() <= 1e-6 && h.rank < b.rank)) {
                best = Some(h);
            }
        };
        let guides = if vertical { &self.guides_x } else { &self.guides_y };
        for &g in guides {
            offer(Hit { value: g, dist: (val - g).abs(), rank: 0, source: Source::Guide });
        }
        if let Some((o, s)) = self.grid {
            let (o, s) = if vertical { (o.x, s.x) } else { (o.y, s.y) };
            let g = o + ((val - o) / s).round() * s;
            offer(Hit { value: g, dist: (val - g).abs(), rank: 1, source: Source::Grid });
        }
        if self.smart {
            for r in self.smart_rects() {
                let vals = if vertical { [r.left, (r.left + r.right) / 2.0, r.right] } else { [r.top, (r.top + r.bottom) / 2.0, r.bottom] };
                for v in vals {
                    offer(Hit { value: v, dist: (val - v).abs(), rank: 2, source: Source::Smart(*r) });
                }
            }
        }
        best
    }

    /// Rastet einen Punkt ein.
    pub fn point(&self, p: Point) -> Snapped {
        let hx = self.best(p.x, true);
        let hy = self.best(p.y, false);
        let mut q = p;
        if let Some(h) = hx {
            q.x = h.value;
        } else if self.pixels {
            q.x = p.x.round();
        }
        if let Some(h) = hy {
            q.y = h.value;
        } else if self.pixels {
            q.y = p.y.round();
        }
        let mut guides = Vec::new();
        if self.show_smart {
            if let Some(Hit { source: Source::Smart(r), value, .. }) = hx {
                guides.push(Overlay::SmartGuide { from: Point::new(value, q.y.min(r.top)), to: Point::new(value, q.y.max(r.bottom)) });
            }
            if let Some(Hit { source: Source::Smart(r), value, .. }) = hy {
                guides.push(Overlay::SmartGuide { from: Point::new(q.x.min(r.left), value), to: Point::new(q.x.max(r.right), value) });
            }
        }
        Snapped { point: q, guides, snapped_x: hx.is_some(), snapped_y: hy.is_some() }
    }

    /// Rastet ein bewegtes Rechteck ein (Kanten und Mitte); liefert die Korrektur (dx, dy) und die
    /// Rückmeldung für das korrigierte Rechteck.
    pub fn rect(&self, r: Rect) -> (f32, f32, Vec<Overlay>) {
        let r = r.normalized();
        let pick = |vals: [f32; 3], vertical: bool| -> Option<(f32, Hit)> {
            let mut out: Option<(f32, Hit)> = None;
            for s in vals {
                if let Some(h) = self.best(s, vertical)
                    && out.is_none_or(|(_, b)| h.dist < b.dist - 1e-6 || ((h.dist - b.dist).abs() <= 1e-6 && h.rank < b.rank))
                {
                    out = Some((h.value - s, h));
                }
            }
            out
        };
        let cx = (r.left + r.right) / 2.0;
        let cy = (r.top + r.bottom) / 2.0;
        let sx = pick([r.left, cx, r.right], true);
        let sy = pick([r.top, cy, r.bottom], false);
        // ohne Treffer bleibt die Lage (ganze Pixel gelten für die Verschiebung, nicht die Kante)
        let dx = sx.map(|(d, _)| d).unwrap_or(0.0);
        let dy = sy.map(|(d, _)| d).unwrap_or(0.0);
        let m = r.translate(dx, dy);
        let mut guides = Vec::new();
        if self.show_smart {
            let (mcx, mcy) = ((m.left + m.right) / 2.0, (m.top + m.bottom) / 2.0);
            if let Some((_, Hit { source: Source::Smart(t), .. })) = sx {
                // alle übereinstimmenden x-Werte zeigen (Kante und Mitte können gleichzeitig passen)
                for v in [m.left, mcx, m.right] {
                    if [t.left, (t.left + t.right) / 2.0, t.right].iter().any(|w| (w - v).abs() < 0.01) {
                        guides.push(Overlay::SmartGuide { from: Point::new(v, m.top.min(t.top)), to: Point::new(v, m.bottom.max(t.bottom)) });
                    }
                }
            }
            if let Some((_, Hit { source: Source::Smart(t), .. })) = sy {
                for v in [m.top, mcy, m.bottom] {
                    if [t.top, (t.top + t.bottom) / 2.0, t.bottom].iter().any(|w| (w - v).abs() < 0.01) {
                        guides.push(Overlay::SmartGuide { from: Point::new(m.left.min(t.left), v), to: Point::new(m.right.max(t.right), v) });
                    }
                }
            }
            guides.extend(self.gap_labels(&m));
        }
        (dx, dy, guides)
    }

    /// Abstände zu den nächsten Nachbarn links/rechts/oben/unten, die sich mit dem Rechteck
    /// überlappen (Bemaßung wie bei den intelligenten Hilfslinien).
    fn gap_labels(&self, m: &Rect) -> Vec<Overlay> {
        if !self.smart {
            return Vec::new();
        }
        let mut out = Vec::new();
        let overl_y = |t: &Rect| t.top < m.bottom && t.bottom > m.top;
        let overl_x = |t: &Rect| t.left < m.right && t.right > m.left;
        let left = self.targets.iter().filter(|t| overl_y(t) && t.right <= m.left).max_by(|a, b| a.right.total_cmp(&b.right));
        let right = self.targets.iter().filter(|t| overl_y(t) && t.left >= m.right).min_by(|a, b| a.left.total_cmp(&b.left));
        let up = self.targets.iter().filter(|t| overl_x(t) && t.bottom <= m.top).max_by(|a, b| a.bottom.total_cmp(&b.bottom));
        let down = self.targets.iter().filter(|t| overl_x(t) && t.top >= m.bottom).min_by(|a, b| a.top.total_cmp(&b.top));
        let cy = (m.top + m.bottom) / 2.0;
        let cx = (m.left + m.right) / 2.0;
        let max_gap = 400.0;
        if let Some(t) = left
            && m.left - t.right <= max_gap
            && m.left - t.right > 0.5
        {
            out.push(Overlay::SmartGuide { from: Point::new(t.right, cy), to: Point::new(m.left, cy) });
            out.push(Overlay::Label { at: Point::new((t.right + m.left) / 2.0, cy), text: fmt_num(m.left - t.right) });
        }
        if let Some(t) = right
            && t.left - m.right <= max_gap
            && t.left - m.right > 0.5
        {
            out.push(Overlay::SmartGuide { from: Point::new(m.right, cy), to: Point::new(t.left, cy) });
            out.push(Overlay::Label { at: Point::new((t.left + m.right) / 2.0, cy), text: fmt_num(t.left - m.right) });
        }
        if let Some(t) = up
            && m.top - t.bottom <= max_gap
            && m.top - t.bottom > 0.5
        {
            out.push(Overlay::SmartGuide { from: Point::new(cx, t.bottom), to: Point::new(cx, m.top) });
            out.push(Overlay::Label { at: Point::new(cx, (t.bottom + m.top) / 2.0), text: fmt_num(m.top - t.bottom) });
        }
        if let Some(t) = down
            && t.top - m.bottom <= max_gap
            && t.top - m.bottom > 0.5
        {
            out.push(Overlay::SmartGuide { from: Point::new(cx, m.bottom), to: Point::new(cx, t.top) });
            out.push(Overlay::Label { at: Point::new(cx, (t.top + m.bottom) / 2.0), text: fmt_num(t.top - m.bottom) });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapper(targets: Vec<Rect>) -> Snapper {
        Snapper {
            tol: 5.0,
            guides_x: vec![100.0],
            guides_y: vec![],
            grid: None,
            targets,
            page: Some(Rect::new(0.0, 0.0, 300.0, 200.0)),
            smart: true,
            show_smart: true,
            pixels: true,
        }
    }

    #[test]
    fn point_and_rect_snapping() {
        let s = snapper(vec![Rect::new(10.0, 10.0, 50.0, 50.0)]);
        // Hilfslinie x = 100 hat Vorrang, y an der Oberkante des Objekts
        let r = s.point(Point::new(97.0, 12.0));
        assert_eq!(r.point, Point::new(100.0, 10.0));
        assert!(r.guides.iter().any(|g| matches!(g, Overlay::SmartGuide { .. })));
        // außerhalb des Fangabstands: ganze Pixel
        let r = s.point(Point::new(200.4, 77.6));
        assert_eq!(r.point, Point::new(200.0, 78.0));
        // bewegtes Rechteck rastet mit der linken Kante an der rechten Objektkante ein
        let (dx, dy, g) = s.rect(Rect::new(52.0, 60.0, 72.0, 80.0));
        assert_eq!((dx, dy), (-2.0, 0.0));
        assert!(!g.is_empty());
    }
}

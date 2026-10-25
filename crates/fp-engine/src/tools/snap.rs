//! Fang (10 §2.2, §6 „zentrale Snapper-Instanz“): Werkzeuge rasten einen Punkt bzw. ein bewegtes
//! Rechteck vor dem Setzen ein – an Hilfslinien (`SnapToGuides`), am Raster (`SnapToGrids`), an
//! intelligenten Hilfslinien (`SnapToSmartGuides`: Kanten und Mitten anderer Objekte sowie Rand und
//! Mitte der Seite) und an ganzen Pixeln. Fangabstand `SnapDistance` Bildschirmpixel (Vorgabe 5),
//! umgerechnet in Dokumentpixel über den Zoom, einschließlich (≤). Die Rückmeldung (magentafarbene Linien wie in CS6,
//! Farbe `SmartGuideColor`) liefert der Fang als [`Overlay::SmartGuide`]; Abstände zu Nachbarn als
//! [`Overlay::Label`].

use super::{Overlay, ToolContext, fmt_num};
use crate::docops;
use fp_geom::{Point, Rect};
use fp_model::ElementId;
use fp_model::layer::LayerKind;

/// Preference behind View ▸ Snap to Pixels (Firepixel's own switch, default off): whole-pixel
/// rounding of anchor and handle drags with the Subselection tool (and of point drags with the
/// Pointer). Off = free drags at 1/zoom like the reference (re 55 §5, §7.3). Pointer moves, Pen
/// points and the other users of [`Snapper`] always round to whole pixels.
pub const SNAP_TO_PIXELS_PREF: &str = "SnapToPixelGrid";

/// Default of [`SNAP_TO_PIXELS_PREF`]: off, the reference drags points fractionally.
pub const SNAP_TO_PIXELS_DEFAULT: bool = false;

/// State of View ▸ Snap to Pixels.
pub fn snap_to_pixels(prefs: &crate::prefs::Preferences) -> bool {
    prefs.get_bool(SNAP_TO_PIXELS_PREF, SNAP_TO_PIXELS_DEFAULT)
}

/// Preference behind View ▸ Live Preview While Dragging Points (Firepixel's own switch, FB-67):
/// point and handle drags of the Subselection tool (and the Pointer/Pen point drags sharing that
/// code) render the object live ([`super::Overlay::PathLive`]). Off (default) = only the outline
/// follows the drag, the object updates on release.
pub const LIVE_POINT_PREVIEW_PREF: &str = "LivePointPreview";

/// Default of [`LIVE_POINT_PREVIEW_PREF`]: off, outline preview only.
pub const LIVE_POINT_PREVIEW_DEFAULT: bool = false;

/// State of View ▸ Live Preview While Dragging Points.
pub fn live_point_preview(prefs: &crate::prefs::Preferences) -> bool {
    prefs.get_bool(LIVE_POINT_PREVIEW_PREF, LIVE_POINT_PREVIEW_DEFAULT)
}

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

/// Kleinerer Abstand gewinnt, bei gleichem Abstand der kleinere Vorrang (Hilfslinie vor Raster vor
/// Objekt), sonst der zuerst gefundene.
fn better(h: &Hit, b: &Hit) -> bool {
    h.dist < b.dist - 1e-6 || ((h.dist - b.dist).abs() <= 1e-6 && h.rank < b.rank)
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
    /// Rechteck-Fang an Raster, Hilfslinien und Leinwandkante (re 41 §1.2: nur wenn „Am Raster
    /// ausrichten“ oder „An Hilfslinien ausrichten“ an ist).
    edges: bool,
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
        // re 41 §1.2: eine Toleranz `SnapDistance`/Zoom für Raster, Hilfslinien, Leinwandkante und
        // intelligente Hilfslinien; `GridGuideSnapDist` hat im Original keine Wirkung
        let page = ctx.doc.pages.get(ctx.page);
        let (mut guides_x, mut guides_y) = (Vec::new(), Vec::new());
        if v.snap_to_guides
            && v.show_guides
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
        let page_rect = page.map(|p| {
            Rect::new(
                p.canvas.origin.x,
                p.canvas.origin.y,
                p.canvas.origin.x + p.canvas.width,
                p.canvas.origin.y + p.canvas.height,
            )
        });
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
            edges: v.snap_to_grid || v.snap_to_guides,
            guides_x,
            guides_y,
            grid,
            targets,
            page: page_rect,
            smart,
            show_smart: v.show_smart_guides,
            // whole pixels for Pointer moves, Pen points etc. (measured, re 55 §5.2, §7.4);
            // point drags choose with [`Self::with_pixels`]
            pixels: true,
        }
    }

    /// Ohne intelligente Hilfslinien (Pixelauswahl, Freihand).
    pub fn without_smart(mut self) -> Snapper {
        self.smart = false;
        self
    }

    /// Whole-pixel rounding on or off (point drags: View ▸ Snap to Pixels).
    pub fn with_pixels(mut self, on: bool) -> Snapper {
        self.pixels = on;
        self
    }

    /// Verschiebung auf ganze Pixel runden (wenn eingeschaltet).
    pub fn round_delta(&self, dx: f32, dy: f32) -> (f32, f32) {
        if self.pixels {
            (dx.round(), dy.round())
        } else {
            (dx, dy)
        }
    }

    /// Zug beim Verschieben (re 41 §1.2 Schritt 1): Start- und Zeigerpunkt mit `floor` auf ganze
    /// Dokumentpixel, Zug = Zeiger − Start (wenn ganze Pixel eingeschaltet).
    pub fn move_delta(&self, start: Point, cur: Point) -> (f32, f32) {
        if self.pixels {
            (
                cur.x.floor() - start.x.floor(),
                cur.y.floor() - start.y.floor(),
            )
        } else {
            (cur.x - start.x, cur.y - start.y)
        }
    }

    fn smart_rects(&self) -> impl Iterator<Item = &Rect> {
        self.targets.iter().chain(self.page.iter())
    }

    /// Beste Fanglinie für einen Wert einer Achse (`vertical` = x-Achse); mit `smart` auch Kanten
    /// und Mitten anderer Objekte und der Seite (ein Punkt ist ein Rechteck der Breite 0, re 27 L2).
    fn best(&self, val: f32, vertical: bool, smart: bool) -> Option<Hit> {
        self.best_with(val, vertical, smart, true)
    }

    /// Wie [`Self::best`]; `grid = false` lässt das Raster aus (das Rechteck fängt es eigens).
    fn best_with(&self, val: f32, vertical: bool, smart: bool, grid: bool) -> Option<Hit> {
        let mut best: Option<Hit> = None;
        let mut offer = |h: Hit, tol: f32| {
            // Fangabstand einschließlich: im Original rastet ein Abstand von genau 5 Bildschirmpixeln
            // noch ein, 6 nicht mehr (PARITAET_A L16, gemessen)
            if h.dist <= tol + 1e-4 && best.is_none_or(|b| better(&h, &b)) {
                best = Some(h);
            }
        };
        let guides = if vertical {
            &self.guides_x
        } else {
            &self.guides_y
        };
        for &g in guides {
            offer(
                Hit {
                    value: g,
                    dist: (val - g).abs(),
                    rank: 0,
                    source: Source::Guide,
                },
                self.tol,
            );
        }
        if grid && let Some(h) = self.grid_hit(val, vertical) {
            offer(h, self.tol);
        }
        if smart && let Some((_, h)) = self.smart_hit(val, val, vertical, None) {
            offer(h, self.tol);
        }
        best
    }

    /// Nächste Rasterlinie einer Achse (re 39 §4.2): `round((p − Ursprung)/Abstand)·Abstand +
    /// Ursprung`, halb weg von 0 gerundet; ohne Prüfung des Fangabstands.
    fn grid_hit(&self, val: f32, vertical: bool) -> Option<Hit> {
        let (o, s) = self.grid?;
        let (o, s) = if vertical { (o.x, s.x) } else { (o.y, s.y) };
        let g = o + ((val - o) / s).round() * s;
        Some(Hit {
            value: g,
            dist: (val - g).abs(),
            rank: 1,
            source: Source::Grid,
        })
    }

    /// Intelligente Hilfslinien für die Spanne `lo…hi` einer Achse (re 27 L2): je anderem Objekt in
    /// dieser Reihenfolge linke Kante an dessen linke oder rechte Kante, rechte Kante an dessen linke
    /// oder rechte Kante, Mitte an dessen Mitte (Kante an Mitte gibt es nicht). Abstand ≤ Fangabstand,
    /// der kleinste Betrag über alle Objekte gewinnt, bei Gleichstand der zuerst gefundene. Ein
    /// Objekt mit demselben Rechteck wie das bewegte (`skip`) wird übersprungen. Liefert die
    /// Korrektur und den Treffer.
    fn smart_hit(
        &self,
        lo: f32,
        hi: f32,
        vertical: bool,
        skip: Option<&Rect>,
    ) -> Option<(f32, Hit)> {
        if !self.smart {
            return None;
        }
        let c = (lo + hi) / 2.0;
        let n = self.targets.len();
        let mut out: Option<(f32, Hit)> = None;
        for (i, r) in self.smart_rects().enumerate() {
            if i < n && skip.is_some_and(|s| s == r) {
                continue;
            }
            let (a, b) = if vertical {
                (r.left, r.right)
            } else {
                (r.top, r.bottom)
            };
            let m = (a + b) / 2.0;
            // Leinwandmitte (re 41 §1.2 Schritt 4, `@00d9eb10`): auch linke/rechte Kante auf die Mittellinie
            let page_mid: &[(f32, f32)] = if i >= n { &[(lo, m), (hi, m)] } else { &[] };
            for (from, to) in [(lo, a), (lo, b), (hi, a), (hi, b), (c, m)]
                .into_iter()
                .chain(page_mid.iter().copied())
            {
                let d = to - from;
                if d.abs() <= self.tol + 1e-4 && out.is_none_or(|(od, _)| d.abs() < od.abs() - 1e-6)
                {
                    out = Some((
                        d,
                        Hit {
                            value: to,
                            dist: d.abs(),
                            rank: 2,
                            source: Source::Smart(*r),
                        },
                    ));
                }
            }
        }
        out
    }

    /// Rastet einen Punkt ein.
    pub fn point(&self, p: Point) -> Snapped {
        let hx = self.best(p.x, true, true);
        let hy = self.best(p.y, false, true);
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
            if let Some(Hit {
                source: Source::Smart(r),
                value,
                ..
            }) = hx
            {
                guides.push(Overlay::SmartGuide {
                    from: Point::new(value, q.y.min(r.top)),
                    to: Point::new(value, q.y.max(r.bottom)),
                });
            }
            if let Some(Hit {
                source: Source::Smart(r),
                value,
                ..
            }) = hy
            {
                guides.push(Overlay::SmartGuide {
                    from: Point::new(q.x.min(r.left), value),
                    to: Point::new(q.x.max(r.right), value),
                });
            }
        }
        Snapped {
            point: q,
            guides,
            snapped_x: hx.is_some(),
            snapped_y: hy.is_some(),
        }
    }

    /// Rastet ein bewegtes Rechteck ein (Zug 0, siehe [`Self::move_rect`]).
    pub fn rect(&self, r: Rect) -> (f32, f32, Vec<Overlay>) {
        self.move_rect(r, 0.0, 0.0)
    }

    /// Versatz einer Kante an Leinwandkante, Raster und Hilfslinien (re 41 §1.2 Schritt 3,
    /// `@00d86b10`): Leinwandkante nur, wenn die Kante außerhalb liegt oder genau darauf; danach
    /// Rasterlinie und Hilfslinien, jeweils nur bei echt kleinerem Betrag; gültig nur ≤ Toleranz.
    fn edge_offset(&self, v: f32, vertical: bool) -> Option<f32> {
        let mut out: Option<f32> = None;
        let mut offer = |d: f32| {
            if d.abs() <= self.tol + 1e-4 && out.is_none_or(|o| d.abs() < o.abs() - 1e-6) {
                out = Some(d);
            }
        };
        if let Some(pg) = self.page {
            let (a, b) = if vertical {
                (pg.left, pg.right)
            } else {
                (pg.top, pg.bottom)
            };
            if v <= a + 1e-4 {
                offer(a - v);
            }
            if v >= b - 1e-4 {
                offer(b - v);
            }
        }
        if let Some(h) = self.grid_hit(v, vertical) {
            offer(h.value - v);
        }
        for &g in if vertical {
            &self.guides_x
        } else {
            &self.guides_y
        } {
            offer(g - v);
        }
        out
    }

    /// Rastet ein beim Verschieben bewegtes Rechteck ein (re 41 §1.2): `r` = Auswahlrahmen + Zug
    /// (`zx`, `zy`). Je Achse: Raster/Hilfslinien/Leinwandkante für die linke und die rechte Kante,
    /// der kleinere Betrag gewinnt, bei Gleichstand die rechte/untere Kante, Versatz 0 = kein
    /// Treffer; intelligente Hilfslinien (auch Leinwandmitte) eigens; von beiden gewinnt der
    /// kleinere Gesamtzug ab Ausgangslage (intelligent bei Gleichstand). Liefert die Korrektur
    /// (dx, dy) und die Rückmeldung für das korrigierte Rechteck.
    pub fn move_rect(&self, r: Rect, zx: f32, zy: f32) -> (f32, f32, Vec<Overlay>) {
        let r = r.normalized();
        let pick = |lo: f32, hi: f32, z: f32, vertical: bool| -> (f32, Option<(f32, Hit)>) {
            let grid = if self.edges {
                let (l, h) = (
                    self.edge_offset(lo, vertical),
                    self.edge_offset(hi, vertical),
                );
                let off = match (l, h) {
                    (Some(l), Some(h)) => Some(if l.abs() < h.abs() { l } else { h }),
                    (l, h) => l.or(h),
                };
                off.filter(|o| *o != 0.0)
            } else {
                None
            };
            let smart = self.smart_hit(lo, hi, vertical, Some(&r));
            match (grid, smart) {
                (g, Some((s, h))) if g.is_none_or(|g| (z + s).abs() <= (z + g).abs()) => {
                    (s, Some((s, h)))
                }
                (Some(g), _) => (g, None),
                _ => (0.0, None),
            }
        };
        let (dx, sx) = pick(r.left, r.right, zx, true);
        let (dy, sy) = pick(r.top, r.bottom, zy, false);
        let m = r.translate(dx, dy);
        let mut guides = Vec::new();
        if self.show_smart {
            let (mcx, mcy) = ((m.left + m.right) / 2.0, (m.top + m.bottom) / 2.0);
            // Linie zu jeder übereinstimmenden Stelle (entgegengesetzte Kanten eingeschlossen)
            if let Some((
                _,
                Hit {
                    source: Source::Smart(t),
                    ..
                },
            )) = sx
            {
                // alle übereinstimmenden x-Werte zeigen (Kante und Mitte können gleichzeitig passen)
                for (v, c) in [(m.left, false), (mcx, true), (m.right, false)] {
                    let w: &[f32] = if c {
                        &[(t.left + t.right) / 2.0]
                    } else {
                        &[t.left, t.right]
                    };
                    if w.iter().any(|w| (w - v).abs() < 0.01) {
                        guides.push(Overlay::SmartGuide {
                            from: Point::new(v, m.top.min(t.top)),
                            to: Point::new(v, m.bottom.max(t.bottom)),
                        });
                    }
                }
            }
            if let Some((
                _,
                Hit {
                    source: Source::Smart(t),
                    ..
                },
            )) = sy
            {
                for (v, c) in [(m.top, false), (mcy, true), (m.bottom, false)] {
                    let w: &[f32] = if c {
                        &[(t.top + t.bottom) / 2.0]
                    } else {
                        &[t.top, t.bottom]
                    };
                    if w.iter().any(|w| (w - v).abs() < 0.01) {
                        guides.push(Overlay::SmartGuide {
                            from: Point::new(m.left.min(t.left), v),
                            to: Point::new(m.right.max(t.right), v),
                        });
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
        let left = self
            .targets
            .iter()
            .filter(|t| overl_y(t) && t.right <= m.left)
            .max_by(|a, b| a.right.total_cmp(&b.right));
        let right = self
            .targets
            .iter()
            .filter(|t| overl_y(t) && t.left >= m.right)
            .min_by(|a, b| a.left.total_cmp(&b.left));
        let up = self
            .targets
            .iter()
            .filter(|t| overl_x(t) && t.bottom <= m.top)
            .max_by(|a, b| a.bottom.total_cmp(&b.bottom));
        let down = self
            .targets
            .iter()
            .filter(|t| overl_x(t) && t.top >= m.bottom)
            .min_by(|a, b| a.top.total_cmp(&b.top));
        let cy = (m.top + m.bottom) / 2.0;
        let cx = (m.left + m.right) / 2.0;
        let max_gap = 400.0;
        if let Some(t) = left
            && m.left - t.right <= max_gap
            && m.left - t.right > 0.5
        {
            out.push(Overlay::SmartGuide {
                from: Point::new(t.right, cy),
                to: Point::new(m.left, cy),
            });
            out.push(Overlay::Label {
                at: Point::new((t.right + m.left) / 2.0, cy),
                text: fmt_num(m.left - t.right),
            });
        }
        if let Some(t) = right
            && t.left - m.right <= max_gap
            && t.left - m.right > 0.5
        {
            out.push(Overlay::SmartGuide {
                from: Point::new(m.right, cy),
                to: Point::new(t.left, cy),
            });
            out.push(Overlay::Label {
                at: Point::new((t.left + m.right) / 2.0, cy),
                text: fmt_num(t.left - m.right),
            });
        }
        if let Some(t) = up
            && m.top - t.bottom <= max_gap
            && m.top - t.bottom > 0.5
        {
            out.push(Overlay::SmartGuide {
                from: Point::new(cx, t.bottom),
                to: Point::new(cx, m.top),
            });
            out.push(Overlay::Label {
                at: Point::new(cx, (t.bottom + m.top) / 2.0),
                text: fmt_num(m.top - t.bottom),
            });
        }
        if let Some(t) = down
            && t.top - m.bottom <= max_gap
            && t.top - m.bottom > 0.5
        {
            out.push(Overlay::SmartGuide {
                from: Point::new(cx, m.bottom),
                to: Point::new(cx, t.top),
            });
            out.push(Overlay::Label {
                at: Point::new(cx, (t.top + m.bottom) / 2.0),
                text: fmt_num(t.top - m.bottom),
            });
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
            edges: true,
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
        assert!(
            r.guides
                .iter()
                .any(|g| matches!(g, Overlay::SmartGuide { .. }))
        );
        // außerhalb des Fangabstands: ganze Pixel
        let r = s.point(Point::new(200.4, 77.6));
        assert_eq!(r.point, Point::new(200.0, 78.0));
        // bewegtes Rechteck rastet mit der linken Kante an der rechten Objektkante ein
        let (dx, dy, g) = s.rect(Rect::new(52.0, 60.0, 72.0, 80.0));
        assert_eq!((dx, dy), (-2.0, 0.0));
        assert!(!g.is_empty());
    }

    /// re 27 L2: entgegengesetzte Kanten rasten, Kante nicht an Mitte, ≤ Fangabstand, kleinster
    /// Betrag gewinnt, ein Objekt mit identischem Rechteck wird übersprungen.
    #[test]
    fn smart_rect_rules() {
        let s = Snapper {
            guides_x: vec![],
            page: None,
            ..snapper(vec![Rect::new(100.0, 100.0, 200.0, 160.0)])
        };
        // rechte Kante 4 px vor der linken Kante des Objekts
        assert_eq!(s.rect(Rect::new(46.0, 300.0, 96.0, 330.0)).0, 4.0);
        // genau 5 px rastet noch, 6 nicht
        assert_eq!(s.rect(Rect::new(205.0, 300.0, 255.0, 330.0)).0, -5.0);
        assert_eq!(s.rect(Rect::new(206.0, 300.0, 256.0, 330.0)).0, 0.0);
        // linke Kante 4 px neben der Objektmitte 150: keine Kante-an-Mitte
        assert_eq!(s.rect(Rect::new(154.0, 300.0, 254.0, 330.0)).0, 0.0);
        // Mitte an Mitte (Mitte 153 → 150)
        assert_eq!(s.rect(Rect::new(143.0, 300.0, 163.0, 330.0)).0, -3.0);
        // kleinster Betrag: linke Kante 3 px (→ 200) schlägt rechte Kante 4 px
        let two = Snapper {
            guides_x: vec![],
            page: None,
            ..snapper(vec![
                Rect::new(100.0, 100.0, 200.0, 160.0),
                Rect::new(0.0, 0.0, 257.0, 20.0),
            ])
        };
        assert_eq!(two.rect(Rect::new(203.0, 300.0, 253.0, 330.0)).0, -3.0);
        // identisches Rechteck wird übersprungen, nur das andere Objekt zählt
        let same = Snapper {
            guides_x: vec![],
            page: None,
            ..snapper(vec![
                Rect::new(10.0, 300.0, 60.0, 330.0),
                Rect::new(63.0, 0.0, 90.0, 20.0),
            ])
        };
        assert_eq!(same.rect(Rect::new(10.0, 300.0, 60.0, 330.0)).0, 3.0);
        // Punkt (Rechteck der Breite 0) rastet an Kanten und Mitten
        assert_eq!(s.point(Point::new(154.0, 400.0)).point.x, 150.0);
        assert_eq!(s.point(Point::new(204.0, 400.0)).point.x, 200.0);
    }

    /// T-10 (re 41 §1, TW-98, Lauf 10 `xs-canvas/12`): Raster 36, Leinwand 0,0–2400,1600, Toleranz
    /// `SnapDistance`/Zoom. Je Achse gewinnt die Kante mit dem kleineren Versatz (Gleichstand:
    /// rechts/unten), intelligente Hilfslinien fangen auch an der Leinwandmitte, zwischen beiden
    /// gewinnt der kleinere Gesamtzug.
    #[test]
    fn t10_grid_move_edges_and_canvas_centre() {
        let grid = Some((Point::new(0.0, 0.0), Point::new(36.0, 36.0)));
        let page = Some(Rect::new(0.0, 0.0, 2400.0, 1600.0));
        let s = Snapper {
            tol: 5.0 / 0.428,
            guides_x: vec![],
            grid,
            page,
            targets: vec![],
            ..snapper(vec![])
        };
        let mv = |s: &Snapper, zx: f32, zy: f32| {
            let (dx, dy, _) = s.move_rect(Rect::new(zx, zy, 2400.0 + zx, 1600.0 + zy), zx, zy);
            (zx + dx, zy + dy)
        };
        // eingepasste Seite (Zoom ≈ 0,428, Toleranz 11,7): Zug 11/16 ⇒ x: Mitte 1211 → 1200, y: unten 1616 → 1620
        assert_eq!(mv(&s, 11.0, 16.0), (0.0, 20.0));
        assert_eq!(mv(&s, 13.0, 17.0), (12.0, 20.0));
        assert_eq!(mv(&s, 12.0, 16.0), (12.0, 20.0));
        // Zoom 100 % (Toleranz 5): Zug 4/30 ⇒ Kanten −4/−4, Gleichstand rechts (Gesamtzug 0); y bleibt
        let one = Snapper {
            tol: 5.0,
            ..s.clone()
        };
        assert_eq!(mv(&one, 4.0, 30.0), (0.0, 30.0));
        assert_eq!(mv(&one, 13.0, 17.0), (12.0, 20.0));
        // ohne intelligente Hilfslinien: 11 ⇒ rechte Kante 2411 → 2412
        let plain = Snapper {
            smart: false,
            ..s.clone()
        };
        assert_eq!(mv(&plain, 11.0, 16.0), (12.0, 20.0));
        // ohne Raster/Hilfslinien kein Kantenfang
        let off = Snapper {
            edges: false,
            grid: None,
            smart: false,
            ..s.clone()
        };
        assert_eq!(mv(&off, 11.0, 16.0), (11.0, 16.0));
        // Punktfang am Raster je Achse mit derselben Toleranz; halb weg von 0
        let g = Snapper {
            tol: 16.0,
            page: None,
            smart: false,
            ..s.clone()
        };
        let p = g.point(Point::new(30.0, 54.0));
        assert_eq!(p.point, Point::new(36.0, 54.0));
        assert!(p.snapped_x && !p.snapped_y);
        assert_eq!(
            Snapper {
                tol: 20.0,
                ..g.clone()
            }
            .point(Point::new(18.0, -18.0))
            .point,
            Point::new(36.0, -36.0)
        );
        // Zeigerpunkte beim Verschieben mit floor auf ganze Pixel
        assert_eq!(
            s.move_delta(Point::new(100.7, 100.2), Point::new(111.9, 116.99)),
            (11.0, 16.0)
        );
        assert_eq!(
            s.move_delta(Point::new(100.2, 100.0), Point::new(99.9, 100.5)),
            (-1.0, 0.0)
        );
    }
}

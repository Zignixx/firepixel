//! Web-Ebene auf der Leinwand: Segmente (grün) und Hotspots (cyan) als halbtransparente
//! Überlagerung in ihrer Editorfarbe (`FCLi`) sowie die Segment-Hilfslinien (`SliceGuideColor`), wie
//! im Original (UX-A O17, `docs\screenshots\paritaet\a_309`). „Segmente ausblenden“ blendet alles
//! aus; die Hilfslinien folgen `SliceGuidesVisible`.

use egui::{Color32, Pos2, Stroke};
use fp_geom::{Point, Rect};
use fp_model::layer::LayerKind;
use fp_model::{Document, ElementKind};

/// Ein Web-Objekt zum Zeichnen.
pub struct WebItem {
    pub rect: Rect,
    pub polygon: Option<Vec<Point>>,
    pub ellipse: bool,
    pub slice: bool,
    pub color: [u8; 4],
}

/// Sichtbare Web-Objekte der Seite im Status `frame`.
pub fn items(doc: &Document, page: usize, frame: usize) -> Vec<WebItem> {
    let mut out = Vec::new();
    let layers = fp_engine::docops::own_layers(doc, page);
    // die Segmente der Schaltflächen gehören zur Web-Ebene: ist sie ausgeblendet, fehlen auch sie
    let web_visible = layers
        .iter()
        .filter(|(_, l)| l.kind == LayerKind::Web)
        .all(|(_, l)| l.is_visible());
    for (_, l) in layers {
        if !l.is_visible() {
            continue;
        }
        if l.kind != LayerKind::Web {
            if !web_visible {
                continue;
            }
            // Schaltflächen-Instanzen haben ihr eigenes Segment: grüne Überlagerung und
            // Segment-Hilfslinien wie ein normales Segment (TW-19)
            let ci = if l.shared_across_frames { 0 } else { frame };
            if let Some(cel) = l.cels.get(ci).or_else(|| l.cels.first()) {
                button_slices(doc, &cel.elements, &mut out);
            }
            continue;
        }
        let ci = if l.shared_across_frames { 0 } else { frame };
        let Some(cel) = l.cels.get(ci).or_else(|| l.cels.first()) else {
            continue;
        };
        for e in &cel.elements {
            let ElementKind::WebObject(w) = &e.kind else {
                continue;
            };
            if !e.common.visible {
                continue;
            }
            let c = w.color.0;
            out.push(WebItem {
                rect: w.bounds.normalized(),
                polygon: w
                    .polygon
                    .as_ref()
                    .map(|p| p.nodes.iter().map(|n| n.anchor).collect()),
                ellipse: w.shape == fp_model::web::WebShape::Circle,
                slice: w.slice.is_some(),
                color: [(c >> 16) as u8, (c >> 8) as u8, c as u8, (c >> 24) as u8],
            });
        }
    }
    out
}

/// Farbe des Segments einer Schaltfläche (Vorgabe der Segmente, Grün).
pub const BUTTON_SLICE_COLOR: [u8; 4] = [0, 255, 0, 255];

/// Sichtbare Schaltflächen-Instanzen (auch in Gruppen) als Segmente über ihren Grenzen.
fn button_slices(
    doc: &Document,
    elements: &[std::sync::Arc<fp_model::Element>],
    out: &mut Vec<WebItem>,
) {
    for e in elements {
        if !e.common.visible {
            continue;
        }
        match &e.kind {
            ElementKind::Instance(_) => {
                // aktiver Bereich aus dem Schaltflächensymbol, sonst die Instanzgrenzen (BA-072)
                if let Some(r) = fp_export::slices::button_slice_rect(doc, e, |x| {
                    fp_engine::docops::element_bounds(doc, x)
                }) {
                    out.push(WebItem {
                        rect: r,
                        polygon: None,
                        ellipse: false,
                        slice: true,
                        color: BUTTON_SLICE_COLOR,
                    });
                }
            }
            ElementKind::Group(g) => button_slices(doc, &g.children, out),
            _ => {}
        }
    }
}

/// Segment-Hilfslinien als Strecken (Dokumentkoordinaten), wie im Original (TW-41,
/// `menu-view/12`, `tool-web/04`, `a_309`): Die Seite wird rekursiv geteilt, ohne ein Segment zu
/// schneiden. In einem Bereich gilt der Reihe nach:
/// 1. Liegt das oberste Segment tiefer als die Oberkante: waagerecht an seiner Oberkante.
/// 2. Sonst senkrecht an den Kanten der Segmente der ersten Reihe, über die ganze Höhe (erst Spalten).
/// 3. Geht das nicht: waagerecht an der Oberkante des nächsten Segments darunter.
/// 4. Sonst waagerecht an der Unterkante der Segmente der ersten Reihe.
pub fn slice_guides(page: Rect, slices: &[Rect]) -> Vec<(Point, Point)> {
    let mut out = Vec::new();
    cut(page, slices, &mut out, 0);
    out
}

fn overlaps(s: &Rect, r: &Rect) -> bool {
    s.left < r.right - 0.5
        && s.right > r.left + 0.5
        && s.top < r.bottom - 0.5
        && s.bottom > r.top + 0.5
}

/// Gültiger Schnitt: liegt im Inneren des Bereichs und schneidet kein Segment.
fn valid(inside: &[Rect], region: &Rect, v: f32, horizontal: bool) -> bool {
    let (lo, hi) = if horizontal {
        (region.top, region.bottom)
    } else {
        (region.left, region.right)
    };
    v > lo + 0.5
        && v < hi - 0.5
        && !inside.iter().any(|o| {
            if horizontal {
                o.top < v - 0.5 && o.bottom > v + 0.5
            } else {
                o.left < v - 0.5 && o.right > v + 0.5
            }
        })
}

fn cut(region: Rect, slices: &[Rect], out: &mut Vec<(Point, Point)>, depth: usize) {
    let inside: Vec<Rect> = slices
        .iter()
        .copied()
        .filter(|s| overlaps(s, &region))
        .collect();
    if inside.is_empty() || depth > 24 {
        return;
    }
    // ein Bereich, der genau ein Segment ist, wird nicht weiter geteilt
    if inside.len() == 1 {
        let s = inside[0];
        if s.left <= region.left + 0.5
            && s.right >= region.right - 0.5
            && s.top <= region.top + 0.5
            && s.bottom >= region.bottom - 0.5
        {
            return;
        }
    }
    let top = inside
        .iter()
        .map(|s| s.top.max(region.top))
        .fold(f32::INFINITY, f32::min);
    let first_row: Vec<Rect> = inside
        .iter()
        .copied()
        .filter(|s| s.top.max(region.top) <= top + 0.5)
        .collect();
    let mut tries: Vec<(bool, Vec<f32>)> = Vec::new();
    // 1. oberstes Segment liegt tiefer: Oberkante
    tries.push((true, vec![top]));
    // 2. Spalten an den Kanten der ersten Reihe
    tries.push((
        false,
        first_row.iter().flat_map(|s| [s.left, s.right]).collect(),
    ));
    // 3. Oberkante des nächsten Segments darunter
    let mut tops: Vec<f32> = inside
        .iter()
        .map(|s| s.top)
        .filter(|&t| t > top + 0.5)
        .collect();
    tops.sort_by(|a, b| a.total_cmp(b));
    if let Some(&t) = tops.iter().find(|&&t| valid(&inside, &region, t, true)) {
        tries.push((true, vec![t]));
    }
    // 4. Unterkanten der ersten Reihe
    tries.push((true, first_row.iter().map(|s| s.bottom).collect()));
    // Rückfall für verschachtelte Lagen: irgendeine gültige Kante
    tries.push((
        false,
        inside.iter().flat_map(|s| [s.left, s.right]).collect(),
    ));
    tries.push((
        true,
        inside.iter().flat_map(|s| [s.top, s.bottom]).collect(),
    ));
    for (horizontal, cand) in tries {
        let mut cuts: Vec<f32> = cand
            .into_iter()
            .filter(|&v| valid(&inside, &region, v, horizontal))
            .collect();
        if cuts.is_empty() {
            continue;
        }
        cuts.sort_by(|a, b| a.total_cmp(b));
        cuts.dedup_by(|a, b| (*a - *b).abs() < 0.5);
        for &v in &cuts {
            out.push(if horizontal {
                (Point::new(region.left, v), Point::new(region.right, v))
            } else {
                (Point::new(v, region.top), Point::new(v, region.bottom))
            });
        }
        let (lo, hi) = if horizontal {
            (region.top, region.bottom)
        } else {
            (region.left, region.right)
        };
        let mut edges = vec![lo];
        edges.extend(cuts.iter().copied());
        edges.push(hi);
        for w in edges.windows(2) {
            let sub = if horizontal {
                Rect::new(region.left, w[0], region.right, w[1])
            } else {
                Rect::new(w[0], region.top, w[1], region.bottom)
            };
            cut(sub, slices, out, depth + 1);
        }
        return;
    }
}
/// Zeichnet Überlagerung und Hilfslinien.
pub fn paint(
    painter: &egui::Painter,
    items: &[WebItem],
    page: Rect,
    guides: bool,
    guide_color: Color32,
    to_screen: &dyn Fn(Point) -> Pos2,
) {
    for it in items {
        let [r, g, b, _] = it.color;
        // Segmente heller (≈ 30 %), Hotspots kräftiger (≈ 60 %) mit Rand – wie im Original
        let fill = if it.slice {
            Color32::from_rgba_unmultiplied(r, g, b, 80)
        } else {
            Color32::from_rgba_unmultiplied(r, g, b, 150)
        };
        let edge = Stroke::new(1.0, Color32::from_rgb(r / 2, g / 2, b / 2));
        if let Some(poly) = it.polygon.as_ref().filter(|p| p.len() >= 3) {
            let pts: Vec<Pos2> = poly.iter().map(|p| to_screen(*p)).collect();
            painter.add(egui::Shape::Path(egui::epaint::PathShape {
                points: pts,
                closed: true,
                fill,
                stroke: if it.slice {
                    Stroke::NONE.into()
                } else {
                    edge.into()
                },
            }));
        } else {
            let a = to_screen(Point::new(it.rect.left, it.rect.top));
            let z = to_screen(Point::new(it.rect.right, it.rect.bottom));
            let sr = egui::Rect::from_min_max(a, z);
            if it.ellipse {
                painter.add(egui::Shape::ellipse_filled(
                    sr.center(),
                    sr.size() / 2.0,
                    fill,
                ));
                if !it.slice {
                    painter.add(egui::Shape::ellipse_stroke(
                        sr.center(),
                        sr.size() / 2.0,
                        edge,
                    ));
                }
            } else {
                painter.rect_filled(sr, 0.0, fill);
                if !it.slice {
                    painter.rect_stroke(sr, 0.0, edge, egui::StrokeKind::Inside);
                }
            }
        }
    }
    if guides {
        let slices: Vec<Rect> = items.iter().filter(|i| i.slice).map(|i| i.rect).collect();
        if !slices.is_empty() {
            let s = Stroke::new(1.0, guide_color);
            for (a, b) in slice_guides(page, &slices) {
                let (p, q) = (to_screen(a), to_screen(b));
                painter.line_segment(
                    [
                        egui::pos2(p.x.round() + 0.5, p.y.round() + 0.5),
                        egui::pos2(q.x.round() + 0.5, q.y.round() + 0.5),
                    ],
                    s,
                );
            }
            // Seitenrand in Hilfslinienfarbe (das Original zeichnet die Tabellenaußenkante)
            let a = to_screen(Point::new(page.left, page.top));
            let z = to_screen(Point::new(page.right, page.bottom));
            painter.rect_stroke(
                egui::Rect::from_min_max(a, z),
                0.0,
                s,
                egui::StrokeKind::Inside,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// a_309: ein Segment 297,342–377,422 auf der Seite 97,92–697,492 → waagerecht bei 342 über die
    /// ganze Breite, darunter senkrecht bei 297 und 377, in der Spalte waagerecht bei 422.
    #[test]
    fn guides_like_original_a309() {
        let g = slice_guides(
            Rect::new(97.0, 92.0, 697.0, 492.0),
            &[Rect::new(297.0, 342.0, 377.0, 422.0)],
        );
        let has = |a: (f32, f32), b: (f32, f32)| {
            g.iter().any(|(p, q)| (p.x, p.y) == a && (q.x, q.y) == b)
        };
        assert!(has((97.0, 342.0), (697.0, 342.0)), "{g:?}");
        assert!(has((297.0, 342.0), (297.0, 492.0)), "{g:?}");
        assert!(has((377.0, 342.0), (377.0, 492.0)), "{g:?}");
        assert!(has((297.0, 422.0), (377.0, 422.0)), "{g:?}");
        assert_eq!(g.len(), 4, "{g:?}");
    }

    fn sorted(mut g: Vec<(Point, Point)>) -> Vec<(f32, f32, f32, f32)> {
        let mut v: Vec<_> = g.drain(..).map(|(p, q)| (p.x, p.y, q.x, q.y)).collect();
        v.sort_by(|a, b| a.partial_cmp(b).expect("Zahl"));
        v
    }

    /// TW-41, `menu-view/12`: Schaltfläche 260–360 × 20–60 und Segment 20–180 × 200–280 auf 800 × 600.
    /// Erst Spalten: Die Senkrechten bei 260/360 laufen bis unten, die Waagerechte bei 200 endet bei 260.
    #[test]
    fn guides_like_original_menu_view_12_tw41() {
        let g = slice_guides(
            Rect::new(0.0, 0.0, 800.0, 600.0),
            &[
                Rect::new(20.0, 200.0, 180.0, 280.0),
                Rect::new(260.0, 20.0, 360.0, 60.0),
            ],
        );
        let want = vec![
            (0.0, 20.0, 800.0, 20.0),
            (0.0, 200.0, 260.0, 200.0),
            (20.0, 200.0, 20.0, 600.0),
            (20.0, 280.0, 180.0, 280.0),
            (180.0, 200.0, 180.0, 600.0),
            (260.0, 20.0, 260.0, 600.0),
            (260.0, 60.0, 360.0, 60.0),
            (360.0, 20.0, 360.0, 600.0),
        ];
        let mut want = want;
        want.sort_by(|a, b| a.partial_cmp(b).expect("Zahl"));
        assert_eq!(sorted(g), want);
    }

    /// TW-41, `tool-web/04`: Ein neues Segment 300–450 × 450–550 schneidet die Spalte bei 360. Dann endet
    /// die Senkrechte bei 360 an der Waagerechten bei 450, die von 260 bis zum rechten Rand läuft.
    #[test]
    fn guides_like_original_tool_web_04_tw41() {
        let g = slice_guides(
            Rect::new(0.0, 0.0, 800.0, 600.0),
            &[
                Rect::new(20.0, 200.0, 180.0, 280.0),
                Rect::new(260.0, 20.0, 360.0, 60.0),
                Rect::new(300.0, 450.0, 450.0, 550.0),
            ],
        );
        let mut want = vec![
            (0.0, 20.0, 800.0, 20.0),
            (0.0, 200.0, 260.0, 200.0),
            (20.0, 200.0, 20.0, 600.0),
            (20.0, 280.0, 180.0, 280.0),
            (180.0, 200.0, 180.0, 600.0),
            (260.0, 20.0, 260.0, 600.0),
            (260.0, 450.0, 800.0, 450.0),
            (360.0, 20.0, 360.0, 450.0),
            (260.0, 60.0, 360.0, 60.0),
            (300.0, 450.0, 300.0, 600.0),
            (450.0, 450.0, 450.0, 600.0),
            (300.0, 550.0, 450.0, 550.0),
        ];
        want.sort_by(|a, b| a.partial_cmp(b).expect("Zahl"));
        assert_eq!(sorted(g), want);
    }
}

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
    let web_visible = layers.iter().filter(|(_, l)| l.kind == LayerKind::Web).all(|(_, l)| l.is_visible());
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
        let Some(cel) = l.cels.get(ci).or_else(|| l.cels.first()) else { continue };
        for e in &cel.elements {
            let ElementKind::WebObject(w) = &e.kind else { continue };
            if !e.common.visible {
                continue;
            }
            let c = w.color.0;
            out.push(WebItem {
                rect: w.bounds.normalized(),
                polygon: w.polygon.as_ref().map(|p| p.nodes.iter().map(|n| n.anchor).collect()),
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
fn button_slices(doc: &Document, elements: &[std::sync::Arc<fp_model::Element>], out: &mut Vec<WebItem>) {
    for e in elements {
        if !e.common.visible {
            continue;
        }
        match &e.kind {
            ElementKind::Instance(i) if doc.symbol_by_id(i.symbol_id).is_some_and(|(_, s)| s.kind() == fp_model::SymbolKind::Button) => {
                if let Some(r) = fp_engine::docops::element_bounds(doc, e).filter(|r| r.width() > 0.0 && r.height() > 0.0) {
                    out.push(WebItem { rect: r.normalized(), polygon: None, ellipse: false, slice: true, color: BUTTON_SLICE_COLOR });
                }
            }
            ElementKind::Group(g) => button_slices(doc, &g.children, out),
            _ => {}
        }
    }
}

/// Segment-Hilfslinien als Strecken (Dokumentkoordinaten): Schnitte der Seite wie die Tabelle des
/// HTML-Exports (oben quer an den Oberkanten, darunter abwechselnd senkrecht und waagerecht an den
/// Kanten, ohne ein Segment zu schneiden).
pub fn slice_guides(page: Rect, slices: &[Rect]) -> Vec<(Point, Point)> {
    let mut out = Vec::new();
    cut(page, slices, true, true, &mut out, 0);
    out
}

fn cut(region: Rect, slices: &[Rect], horizontal: bool, tops_only: bool, out: &mut Vec<(Point, Point)>, depth: usize) {
    let inside: Vec<Rect> = slices.iter().copied().filter(|s| s.left < region.right && s.right > region.left && s.top < region.bottom && s.bottom > region.top).collect();
    if inside.is_empty() || depth > 12 {
        return;
    }
    let lo = if horizontal { region.top } else { region.left };
    let hi = if horizontal { region.bottom } else { region.right };
    let mut cuts: Vec<f32> = Vec::new();
    for s in &inside {
        let (a, b) = if horizontal { (s.top, s.bottom) } else { (s.left, s.right) };
        for (v, keep) in [(a, true), (b, !tops_only)] {
            if keep && v > lo + 0.5 && v < hi - 0.5 && !inside.iter().any(|o| if horizontal { o.top < v - 0.5 && o.bottom > v + 0.5 } else { o.left < v - 0.5 && o.right > v + 0.5 }) {
                cuts.push(v);
            }
        }
    }
    cuts.sort_by(|a, b| a.total_cmp(b));
    cuts.dedup_by(|a, b| (*a - *b).abs() < 0.5);
    for &v in &cuts {
        out.push(if horizontal { (Point::new(region.left, v), Point::new(region.right, v)) } else { (Point::new(v, region.top), Point::new(v, region.bottom)) });
    }
    let mut edges = vec![lo];
    edges.extend(cuts.iter().copied());
    edges.push(hi);
    for w in edges.windows(2) {
        let sub = if horizontal { Rect::new(region.left, w[0], region.right, w[1]) } else { Rect::new(w[0], region.top, w[1], region.bottom) };
        // ein Teil, der genau ein Segment ist, wird nicht weiter geschnitten
        let only = slices.iter().filter(|s| s.left < sub.right && s.right > sub.left && s.top < sub.bottom && s.bottom > sub.top).count();
        let exact = slices.iter().any(|s| (s.left - sub.left).abs() < 0.5 && (s.right - sub.right).abs() < 0.5 && (s.top - sub.top).abs() < 0.5 && (s.bottom - sub.bottom).abs() < 0.5);
        if only == 0 || (only == 1 && exact) {
            continue;
        }
        cut(sub, slices, !horizontal, false, out, depth + 1);
    }
}

/// Zeichnet Überlagerung und Hilfslinien.
pub fn paint(painter: &egui::Painter, items: &[WebItem], page: Rect, guides: bool, guide_color: Color32, to_screen: &dyn Fn(Point) -> Pos2) {
    for it in items {
        let [r, g, b, _] = it.color;
        // Segmente heller (≈ 30 %), Hotspots kräftiger (≈ 60 %) mit Rand – wie im Original
        let fill = if it.slice { Color32::from_rgba_unmultiplied(r, g, b, 80) } else { Color32::from_rgba_unmultiplied(r, g, b, 150) };
        let edge = Stroke::new(1.0, Color32::from_rgb(r / 2, g / 2, b / 2));
        if let Some(poly) = it.polygon.as_ref().filter(|p| p.len() >= 3) {
            let pts: Vec<Pos2> = poly.iter().map(|p| to_screen(*p)).collect();
            painter.add(egui::Shape::Path(egui::epaint::PathShape { points: pts, closed: true, fill, stroke: if it.slice { Stroke::NONE.into() } else { edge.into() } }));
        } else {
            let a = to_screen(Point::new(it.rect.left, it.rect.top));
            let z = to_screen(Point::new(it.rect.right, it.rect.bottom));
            let sr = egui::Rect::from_min_max(a, z);
            if it.ellipse {
                painter.add(egui::Shape::ellipse_filled(sr.center(), sr.size() / 2.0, fill));
                if !it.slice {
                    painter.add(egui::Shape::ellipse_stroke(sr.center(), sr.size() / 2.0, edge));
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
                painter.line_segment([egui::pos2(p.x.round() + 0.5, p.y.round() + 0.5), egui::pos2(q.x.round() + 0.5, q.y.round() + 0.5)], s);
            }
            // Seitenrand in Hilfslinienfarbe (das Original zeichnet die Tabellenaußenkante)
            let a = to_screen(Point::new(page.left, page.top));
            let z = to_screen(Point::new(page.right, page.bottom));
            painter.rect_stroke(egui::Rect::from_min_max(a, z), 0.0, s, egui::StrokeKind::Inside);
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
        let g = slice_guides(Rect::new(97.0, 92.0, 697.0, 492.0), &[Rect::new(297.0, 342.0, 377.0, 422.0)]);
        let has = |a: (f32, f32), b: (f32, f32)| g.iter().any(|(p, q)| (p.x, p.y) == a && (q.x, q.y) == b);
        assert!(has((97.0, 342.0), (697.0, 342.0)), "{g:?}");
        assert!(has((297.0, 342.0), (297.0, 492.0)), "{g:?}");
        assert!(has((377.0, 342.0), (377.0, 492.0)), "{g:?}");
        assert!(has((297.0, 422.0), (377.0, 422.0)), "{g:?}");
        assert_eq!(g.len(), 4, "{g:?}");
    }
}

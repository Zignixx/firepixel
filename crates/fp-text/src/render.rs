//! Rasterung eines [`TextLayout`]: Deckungsmaske, farbiges Bild und Glyphenkonturen
//! („In Pfade konvertieren“, 09 §5.8).
//!
//! Ablauf (09 §6.1): Glyphenkonturen an ihre Lage abbilden, Faux-Fett als verbreiterte Polygone,
//! Unterstreichungen als Rechtecke, alles zusammen mit Punktabtastung N×N zählen (Non-Zero, also
//! Vereinigung), Trefferzahl über die Tonkurve auf Alpha. Die Maske dient fp-render als Deckung für
//! die Füllung des Text-`PATv` (Vollfarbe, Verlauf, Muster); [`render`] liefert direkt das Bild mit
//! den Laufarben (`FCLi`).

use fp_color::Rgba8;
use fp_geom::{BezierContour, BezierNode, IRect, Point};
use fp_raster::{Mask, RgbaImage};

use crate::antialias;
use crate::font::outline::to_bezier_contour;
use crate::layout::{PositionedGlyph, TextLayout, FAUX_BOLD_STROKE};
use crate::raster::{contours_to_polygons, embolden, push_contour, push_polygon, sample_counts, Edge, RASTER_TOLERANCE};

/// Pixelgrenzen der Tinte (Glyphen inkl. Faux-Fett und Unterstreichungen), nach außen gerundet.
/// Leeres Rechteck, wenn nichts sichtbar ist.
pub fn ink_bounds(layout: &TextLayout) -> IRect {
    let mut r: Option<[f64; 4]> = None;
    let mut add = |x0: f64, y0: f64, x1: f64, y1: f64| {
        r = Some(match r {
            None => [x0, y0, x1, y1],
            Some(o) => [o[0].min(x0), o[1].min(y0), o[2].max(x1), o[3].max(y1)],
        });
    };
    for g in layout.visible_glyphs() {
        if let Some(b) = g.ink_bounds() {
            add(b.left as f64, b.top as f64, b.right as f64, b.bottom as f64);
        }
    }
    for d in &layout.decorations {
        for p in d.quad {
            add(p[0], p[1], p[0], p[1]);
        }
    }
    match r {
        Some(b) if b.iter().all(|v| v.is_finite()) => {
            let c = |v: f64| v.clamp(-1.0e9, 1.0e9);
            IRect::new(c(b[0]).floor() as i32, c(b[1]).floor() as i32, c(b[2]).ceil() as i32, c(b[3]).ceil() as i32)
        }
        _ => IRect::new(0, 0, 0, 0),
    }
}

/// Strecken einer Glyphe relativ zum Ursprung (ox, oy).
fn glyph_edges(g: &PositionedGlyph, ox: f64, oy: f64, edges: &mut Vec<Edge>) {
    if !g.visible || g.is_tab {
        return;
    }
    let outline = g.font.outline(g.glyph_id);
    if outline.is_empty() {
        return;
    }
    let m = g.transform.translated(-ox, -oy);
    if g.faux_bold {
        let mut polys = contours_to_polygons(&outline.contours, &m, RASTER_TOLERANCE);
        embolden(&mut polys, g.size * FAUX_BOLD_STROKE);
        for p in &polys {
            push_polygon(edges, p);
        }
    } else {
        for c in &outline.contours {
            push_contour(edges, c, &m, RASTER_TOLERANCE);
        }
    }
}

/// Trefferzahlen je Pixel (0…N²) und N für die Glyphen, die `filter` zulässt.
fn counts_for(layout: &TextLayout, bounds: IRect, n: u32, filter: &dyn Fn(&PositionedGlyph) -> bool, deco: Option<Rgba8>) -> Vec<u16> {
    let (ox, oy) = (bounds.left as f64, bounds.top as f64);
    let mut edges = Vec::new();
    for g in layout.glyphs.iter().filter(|g| filter(g)) {
        glyph_edges(g, ox, oy, &mut edges);
    }
    for d in &layout.decorations {
        if deco.is_none_or(|c| c == d.color) {
            let q: Vec<[f64; 2]> = d.quad.iter().map(|p| [p[0] - ox, p[1] - oy]).collect();
            push_polygon(&mut edges, &q);
        }
    }
    sample_counts(&edges, bounds.width() as usize, bounds.height() as usize, n)
}

fn checked(bounds: IRect) -> Option<IRect> {
    let (w, h) = (bounds.width() as u64, bounds.height() as u64);
    if w == 0 || h == 0 || w * h > (1 << 26) { None } else { Some(bounds) }
}

/// Deckungsmaske (Alpha nach Tonkurve) in `bounds` (Vorgabe: [`ink_bounds`]).
pub fn coverage(layout: &TextLayout, bounds: Option<IRect>) -> Mask {
    let b = bounds.unwrap_or_else(|| ink_bounds(layout));
    let Some(b) = checked(b) else { return Mask::empty_at(b.left, b.top) };
    let (n, lut) = antialias::sampling(&layout.antialias);
    let counts = counts_for(layout, b, n, &|_| true, None);
    let data: Vec<u8> = counts.iter().map(|&c| lut[(c as usize).min(lut.len() - 1)]).collect();
    Mask::from_raw(b, data).unwrap_or_else(|_| Mask::empty_at(b.left, b.top))
}

/// Trefferzahlen (0…N²) und N der Glättungseinstellung, z. B. für Vergleiche mit Render-Caches.
pub fn sample_layout(layout: &TextLayout, bounds: IRect) -> (Vec<u16>, u32) {
    let (n, _) = antialias::sampling(&layout.antialias);
    (sample_layout_n(layout, bounds, n), n)
}

/// Trefferzahlen (0…n²) je Pixel in `bounds` mit frei gewählter Überabtastung `n`.
pub fn sample_layout_n(layout: &TextLayout, bounds: IRect, n: u32) -> Vec<u16> {
    match checked(bounds) {
        Some(b) => counts_for(layout, b, n, &|_| true, None),
        None => Vec::new(),
    }
}

/// Rendert den Text mit den Laufarben in ein Bild (`bounds`, Vorgabe: [`ink_bounds`]).
///
/// Alpha = Tonkurve(Treffer der Vereinigung aller Glyphen) · Farbalpha. Bei mehreren Farben
/// mittelt ein Pixel die Farben nach ihren Treffern (wie die Verkleinerungsroutine, die die
/// Farbkanäle der gesetzten Unterpixel mittelt, 09 §6.1).
pub fn render(layout: &TextLayout, bounds: Option<IRect>) -> RgbaImage {
    let b = bounds.unwrap_or_else(|| ink_bounds(layout));
    let Some(b) = checked(b) else { return RgbaImage::empty_at(b.left, b.top) };
    let (n, lut) = antialias::sampling(&layout.antialias);
    let mut colors: Vec<Rgba8> = Vec::new();
    for g in layout.visible_glyphs() {
        if !colors.contains(&g.color) {
            colors.push(g.color);
        }
    }
    for d in &layout.decorations {
        if !colors.contains(&d.color) {
            colors.push(d.color);
        }
    }
    let total = counts_for(layout, b, n, &|_| true, None);
    let per: Vec<Vec<u16>> = if colors.len() > 1 {
        colors.iter().map(|c| counts_for(layout, b, n, &|g| g.color == *c, Some(*c))).collect()
    } else {
        Vec::new()
    };
    let mut data = vec![[0u8; 4]; total.len()];
    for (i, px) in data.iter_mut().enumerate() {
        let t = total[i] as usize;
        if t == 0 {
            continue;
        }
        let alpha = lut[t.min(lut.len() - 1)] as u32;
        let c = if colors.len() <= 1 {
            colors.first().copied().unwrap_or(Rgba8::new(0, 0, 0, 255))
        } else {
            let (mut r, mut g, mut bb, mut a, mut s) = (0u32, 0u32, 0u32, 0u32, 0u32);
            for (ci, c) in colors.iter().enumerate() {
                let k = per[ci][i] as u32;
                r += c.r as u32 * k;
                g += c.g as u32 * k;
                bb += c.b as u32 * k;
                a += c.a as u32 * k;
                s += k;
            }
            let avg = |v: u32| (v + s / 2).checked_div(s).map(|q| q as u8);
            match (avg(r), avg(g), avg(bb), avg(a)) {
                (Some(r), Some(g), Some(bb), Some(a)) => Rgba8::new(r, g, bb, a),
                _ => colors[0],
            }
        };
        let a = (alpha * c.a as u32 + 127) / 255;
        *px = [c.r, c.g, c.b, a as u8];
    }
    RgbaImage::from_raw(b, data).unwrap_or_else(|_| RgbaImage::empty_at(b.left, b.top))
}

/// Konturen aller sichtbaren Glyphen in Dokumentkoordinaten (je Glyphe eine Liste), für
/// „In Pfade konvertieren“ und fp-vector. Faux-Fett liefert die verbreiterten Polygone als
/// Eckpunkt-Konturen, Unterstreichungen erscheinen als eigene Rechteck-Konturen am Ende.
pub fn glyph_contours(layout: &TextLayout) -> Vec<Vec<BezierContour>> {
    let mut out = Vec::new();
    for g in layout.visible_glyphs() {
        if g.is_tab {
            continue;
        }
        let o = g.font.outline(g.glyph_id);
        if o.is_empty() {
            continue;
        }
        let m = g.transform;
        if g.faux_bold {
            let mut polys = contours_to_polygons(&o.contours, &m, RASTER_TOLERANCE);
            embolden(&mut polys, g.size * FAUX_BOLD_STROKE);
            out.push(
                polys
                    .iter()
                    .map(|p| {
                        BezierContour::new(
                            p.iter().map(|q| BezierNode::corner(Point::new(q[0] as f32, q[1] as f32))).collect(),
                            true,
                        )
                    })
                    .collect(),
            );
        } else {
            out.push(o.contours.iter().map(|c| to_bezier_contour(&c.iter().map(|s| s.map(|p| m.apply(p))).collect())).collect());
        }
    }
    for d in &layout.decorations {
        out.push(vec![BezierContour::new(
            d.quad.iter().map(|q| BezierNode::corner(Point::new(q[0] as f32, q[1] as f32))).collect(),
            true,
        )]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_layout_renders_empty() {
        let lay = crate::layout::layout(
            &crate::model::TextBlock::default(),
            &crate::font::FontDb::new(),
            &crate::layout::LayoutOptions::default(),
        );
        assert!(ink_bounds(&lay).is_empty());
        assert!(render(&lay, None).is_empty());
        assert_eq!(coverage(&lay, None).width(), 0);
        assert!(glyph_contours(&lay).is_empty());
    }
}

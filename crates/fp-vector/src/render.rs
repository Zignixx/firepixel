//! Pfad mit Attributen → RGBA-Ebene (Spezifikation 07 §2, §7.5; Referenz `pathrender.py`).
//!
//! Reihenfolge: Füllung, darüber der Strich; bei `FOTb = 1` umgekehrt. Die Ebenen werden mit der
//! **skalaren** Source-over-Formel zusammengesetzt (`@006f53c0`; im Pfadpuffer 119 gegenüber 114
//! von 140 Fällen exakt). Das Ergebnis entspricht dem Render-Cache eines Pfads (`IMGv` unter
//! `PTHv`): ohne Objektdeckkraft, ohne Mischmodus, ohne Live-Effekte — die setzt fp-render beim
//! Einsetzen in die Seite.

use crate::attrs::{FillEdge, FillKind, GradientShape, StrokePlacement, VectorPath};
use crate::brush::tip_width;
use crate::coverage::{Coverage, fill_coverage, polys_bounds};
use crate::feather::feather;
use crate::paint::{FillPainter, PaintIssue, fill_pixel};
use crate::stroke::{effective_diameter, effective_placement, stroke_layer};
use fp_color::Rgba8;
use fp_color::blend::over_scalar;
use fp_geom::{IRect, Rect};
use fp_raster::RgbaImage;
use rayon::prelude::*;

/// Was beim Rendern nicht (vollständig) umgesetzt werden konnte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Missing {
    /// Verlauf ohne Farbstopps.
    FillNoStops,
    /// Muster ohne Bild.
    FillNoPattern,
}

impl From<PaintIssue> for Missing {
    fn from(p: PaintIssue) -> Self {
        match p {
            PaintIssue::NoStops => Missing::FillNoStops,
            PaintIssue::NoPattern => Missing::FillNoPattern,
        }
    }
}

/// Deckungsmaske der Füllung über `rect` (mit harter/geglätteter/weicher Kante).
pub fn fill_mask(path: &VectorPath, edge: FillEdge, rect: IRect) -> Coverage {
    let polys = path.polys();
    match edge {
        FillEdge::Hard => fill_coverage(&polys, rect, path.even_odd, path.agm, false),
        FillEdge::AntiAliased => fill_coverage(&polys, rect, path.even_odd, path.agm, true),
        FillEdge::Feather(r) => {
            let r = r.min(crate::feather::MAX_FEATHER);
            let big = fill_coverage(
                &polys,
                rect.inflate(r as i32, r as i32),
                path.even_odd,
                path.agm,
                true,
            );
            feather(&big, r, rect)
        }
    }
}

/// Füllebene als RGBA über `rect` (Referenz `fill_layer`); `Ok(None)` ohne Füllung.
pub fn fill_layer(path: &VectorPath, rect: IRect) -> Result<Option<Vec<[u8; 4]>>, PaintIssue> {
    let Some(fill) = &path.fill else {
        return Ok(None);
    };
    if let FillKind::Gradient(g) = &fill.kind
        && g.shape == GradientShape::Contour
    {
        return crate::contour_gradient::contour_fill_layer(path, fill, g, rect).map(Some);
    }
    let painter = FillPainter::new(path, fill)?;
    let m = fill_mask(path, fill.edge, rect);
    let w = rect.width() as usize;
    let mut out = vec![[0u8; 4]; m.data.len()];
    if w == 0 {
        return Ok(Some(out));
    }
    out.par_chunks_mut(w)
        .zip(m.data.par_chunks(w))
        .enumerate()
        .for_each(|(j, (orow, mrow))| {
            let y = rect.top + j as i32;
            for (i, (o, &c)) in orow.iter_mut().zip(mrow).enumerate() {
                if c == 0 {
                    continue;
                }
                let x = rect.left + i as i32;
                let (argb, c) = painter.shade(x, y, c);
                if c == 0 {
                    continue;
                }
                *o = fill_pixel(argb, c);
            }
        });
    Ok(Some(out))
}

/// Pfad als RGBA-Puffer über `rect` (Zeilen von oben, `[r, g, b, a]`) und Liste fehlender Teile.
pub fn render_path_pixels(path: &VectorPath, rect: IRect) -> (Vec<[u8; 4]>, Vec<Missing>) {
    let n = rect.width() as usize * rect.height() as usize;
    let mut missing = Vec::new();
    let mut layers: Vec<Vec<[u8; 4]>> = Vec::new();
    match fill_layer(path, rect) {
        Ok(Some(f)) => layers.push(f),
        Ok(None) => {}
        Err(e) => missing.push(e.into()),
    }
    if let Some(s) = &path.stroke {
        let l = stroke_layer(path, s, rect);
        if path.fill_over_stroke {
            layers.insert(0, l);
        } else {
            layers.push(l);
        }
    }
    let mut out = vec![[0u8; 4]; n];
    for l in &layers {
        for (d, s) in out.iter_mut().zip(l) {
            if s[3] != 0 {
                let r = over_scalar(Rgba8::new(d[0], d[1], d[2], d[3]), [s[0], s[1], s[2]], s[3]);
                *d = [r.r, r.g, r.b, r.a];
            }
        }
    }
    (out, missing)
}

/// Pfad als [`RgbaImage`] über `rect`.
pub fn render_path(path: &VectorPath, rect: IRect) -> RgbaImage {
    let (px, _) = render_path_pixels(path, rect);
    RgbaImage::from_raw(rect, px).unwrap_or_else(|_| RgbaImage::empty_at(rect.left, rect.top))
}

/// Pfadebene mit Lage, wie fp-render sie braucht.
#[derive(Debug, Clone)]
pub struct PathLayer {
    /// Pixel des Pfads (Füllung + Strich), auf die tatsächlich bedeckten Pixel zugeschnitten.
    pub image: RgbaImage,
    /// Nicht umgesetzte Teile (leer = vollständig).
    pub missing: Vec<Missing>,
}

/// Pfad rendern über [`render_bounds`] und auf die Alpha-Grenzen zuschneiden. `None`, wenn der
/// Pfad keine Punkte hat.
pub fn render_path_layer(path: &VectorPath) -> Option<PathLayer> {
    let rect = render_bounds(path)?;
    let (px, missing) = render_path_pixels(path, rect);
    let img = RgbaImage::from_raw(rect, px).ok()?;
    let ab = img.alpha_bounds();
    let image = if ab.is_empty() {
        RgbaImage::empty_at(rect.left, rect.top)
    } else {
        img.cropped(ab)
    };
    Some(PathLayer { image, missing })
}

/// Rand des Strichs über die Kontur hinaus in Pixeln (konservativ, für Puffergrößen).
pub fn stroke_margin(path: &VectorPath) -> f64 {
    let Some(s) = &path.stroke else { return 0.0 };
    let pl = effective_placement(path, s);
    let dp = effective_diameter(s, pl);
    let b = &s.brush;
    let w = tip_width(dp, b.shape == crate::attrs::TipShape::Circle);
    let mut m = w / 2.0 + 3.0;
    if b.kind == crate::attrs::BrushType::Natural {
        m += 10.0 * dp + b.tip_spacing as f64 * dp / 1000.0 + 2.0;
    }
    if pl == StrokePlacement::Inside {
        m = m.min(w + 3.0);
    }
    m
}

/// Konservatives Pixelrechteck, das alle Pixel des Pfads enthält (Füllung inkl. weicher Kante,
/// Strich inkl. Spitze, Streuung und Mehrfachspitzen). `None` ohne Punkte.
pub fn render_bounds(path: &VectorPath) -> Option<IRect> {
    let polys = path.polys();
    let b = polys_bounds(&polys)?;
    let mut m = 1.0f64;
    if let Some(f) = &path.fill
        && let FillEdge::Feather(r) = f.edge
    {
        m = m.max(r.min(crate::feather::MAX_FEATHER) as f64 + 1.0);
    }
    m = m.max(stroke_margin(path));
    let mi = m.ceil().min(1e6) as i32;
    Some(b.inflate(mi, mi))
}

/// Geometrische Grenzen der Polylinien (ohne Strich), `None` ohne Punkte.
pub fn path_bounds(path: &VectorPath) -> Option<Rect> {
    let mut it = path
        .contours
        .iter()
        .flat_map(|c| c.points.iter())
        .filter(|p| p.x.is_finite() && p.y.is_finite());
    let f = it.next()?;
    let mut r = Rect::new(f.x as f32, f.y as f32, f.x as f32, f.y as f32);
    for p in it {
        r.left = r.left.min(p.x as f32);
        r.top = r.top.min(p.y as f32);
        r.right = r.right.max(p.x as f32);
        r.bottom = r.bottom.max(p.y as f32);
    }
    Some(r)
}

/// Grenzen inklusive Strich (wirksame Platzierung: mittig ± D/2, außen + D, innen ± 0) — für
/// Auswahlrahmen und Info-Bedienfeld (A: Objekt-Bounds des Originals sind nicht untersucht, 16 §1.3).
pub fn path_bounds_with_stroke(path: &VectorPath) -> Option<Rect> {
    let r = path_bounds(path)?;
    let Some(s) = &path.stroke else {
        return Some(r);
    };
    if s.brush.feedback == crate::attrs::Feedback::Background {
        return Some(r);
    }
    let d = s.brush.diameter as f32;
    let e = match effective_placement(path, s) {
        StrokePlacement::Center => d / 2.0,
        StrokePlacement::Outside => d,
        StrokePlacement::Inside => 0.0,
    };
    Some(Rect::new(r.left - e, r.top - e, r.right + e, r.bottom + e))
}

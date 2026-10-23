//! Feather and dither fill edges of text objects (TW-203).
//!
//! The text rasteriser in fp-text knows only Hard (one sample per pixel) and anti-aliased coverage.
//! The Feather and Dither edges of a path post-process the anti-aliased coverage mask with
//! [`fp_vector::feather::feather`] and [`fp_vector::feather::ordered_dither`]; a text gets the same
//! treatment here: its anti-aliased coverage (with its own anti-aliasing level) over the ink
//! rectangle grown by the width on every side, the triangle filter of that width, and for Dither
//! the Bayer 8 × 8 threshold on document pixel coordinates. The run colours apply after the edge
//! (the colour alpha after the dither, as the fill alpha of a path); with several colours every
//! pixel mixes them by their feathered weights.
//!
//! A gradient, pattern or web-dither fill (Q-64, RE spec 62 §2) is painted through the same
//! coverage: [`render_with_paint`] masks the paint a path with the same fill and fill handles
//! would get with the text's glyph coverage.

use fp_color::Rgba8;
use fp_geom::IRect;
use fp_raster::{Mask, RgbaImage};
use fp_text::TextLayout;
use fp_vector::coverage::Coverage;
use fp_vector::feather::{MAX_FEATHER, feather, ordered_dither};
use fp_vector::paint::{FillPainter, PaintIssue, fill_pixel};
use fp_vector::{FillEdge, VectorPath};

/// Pixels a text layer may hold at most before the edge pass gives up (like fp-text's raster
/// limit).
const MAX_PIXELS: u64 = 1 << 26;

/// Feathered (and for Dither quantised) coverage of a text with a Feather or Dither edge.
struct SoftCoverage {
    /// Feather width in pixels.
    r: i32,
    /// Output window grown by the feather width (where the plain coverage is taken).
    big: IRect,
    /// Feathered coverage over the output window.
    total: Coverage,
}

/// [`SoftCoverage`] for a Feather or Dither edge; `None` for Hard and Anti-Alias (and width 0),
/// `Some(Err(at))` when there is no ink or the layer would be too large (empty image at `at`).
fn soft_coverage(layout: &TextLayout, edge: FillEdge) -> Option<Result<SoftCoverage, IRect>> {
    let (r, dither) = match edge {
        FillEdge::Feather(r) if r > 0 => (r, false),
        FillEdge::Dither(r) if r > 0 => (r, true),
        _ => return None,
    };
    let r = r.min(MAX_FEATHER) as i32;
    let ink = fp_text::render::ink_bounds(layout);
    if ink.is_empty() {
        return Some(Err(ink));
    }
    // 2 px safety margin like the plain renderer (hinting and the rounded baseline shift the ink
    // by fractions of a pixel), then the feather reach on every side
    let window = ink.inflate(2 + r, 2 + r);
    let big = window.inflate(r, r);
    let (bw, bh) = (big.width() as u64, big.height() as u64);
    if bw * bh > MAX_PIXELS {
        return Some(Err(window));
    }
    let mut total = feathered(fp_text::render::coverage(layout, Some(big)), r, window);
    if dither {
        ordered_dither(&mut total);
    }
    Some(Ok(SoftCoverage { r, big, total }))
}

/// The triangle filter of width `r` over a plain coverage mask, cut to `window`.
fn feathered(m: Mask, r: i32, window: IRect) -> Coverage {
    let cov = Coverage {
        rect: m.bounds(),
        data: m.data().to_vec(),
    };
    feather(&cov, r as u32, window)
}

/// The image cut to its pixels with alpha plus a 1 px transparent margin (empty at `ink`).
fn trimmed(img: RgbaImage, ink: IRect) -> RgbaImage {
    let ab: IRect = img.alpha_bounds();
    if ab.is_empty() {
        return RgbaImage::empty_at(ink.left, ink.top);
    }
    img.copy_rect(ab.inflate(1, 1)).unwrap_or(img)
}

/// Text image with a Feather or Dither fill edge; `None` for Hard and Anti-Alias (and width 0),
/// which the plain text renderer handles. Without `bounds` the result is cut to the pixels with
/// alpha plus a 1 px transparent margin, like [`fp_text::render::render`].
pub fn render_with_edge(layout: &TextLayout, edge: FillEdge) -> Option<RgbaImage> {
    let SoftCoverage { r, big, total } = match soft_coverage(layout, edge)? {
        Ok(s) => s,
        Err(at) => return Some(RgbaImage::empty_at(at.left, at.top)),
    };
    let window = total.rect;
    let colors = fp_text::render::ink_colors(layout);
    let weights: Vec<Coverage> = if colors.len() > 1 {
        colors
            .iter()
            .map(|c| feathered(fp_text::render::color_coverage(layout, big, *c), r, window))
            .collect()
    } else {
        Vec::new()
    };
    let mut data = vec![[0u8; 4]; total.data.len()];
    for (i, px) in data.iter_mut().enumerate() {
        let alpha = total.data[i] as u32;
        if alpha == 0 {
            continue;
        }
        let c = mix(&colors, &weights, i);
        let a = (alpha * c.a as u32 + 127) / 255;
        *px = [c.r, c.g, c.b, a as u8];
    }
    let img = RgbaImage::from_raw(window, data)
        .unwrap_or_else(|_| RgbaImage::empty_at(window.left, window.top));
    Some(trimmed(img, fp_text::render::ink_bounds(layout)))
}

/// Text image whose glyph coverage masks the paint of a path fill (RE spec 62 §2.6 Nr. 2, Q-64):
/// a gradient, pattern or web-dither fill with the text's fill handles in document coordinates.
/// The coverage is the text's own (Hard, anti-aliased with its level, or the Feather/Dither edge
/// above); colour, texture and transparency dither come from [`FillPainter`] exactly as for a
/// path with the same fill (`a = (alpha * c + 127) / 255`); the run colours do not apply. The
/// result is cut like [`fp_text::render::render`]. A fill the painter cannot paint (no colour
/// stops, pattern without image) is an error.
pub fn render_with_paint(
    layout: &TextLayout,
    edge: FillEdge,
    path: &VectorPath,
) -> Result<RgbaImage, PaintIssue> {
    let Some(fill) = path.fill.as_ref() else {
        return Ok(fp_text::render::render(layout, None));
    };
    let painter = FillPainter::new(path, fill)?;
    let cov = match soft_coverage(layout, edge) {
        Some(Ok(s)) => s.total,
        Some(Err(at)) => return Ok(RgbaImage::empty_at(at.left, at.top)),
        None => {
            let m = fp_text::render::coverage(layout, None);
            Coverage {
                rect: m.bounds(),
                data: m.data().to_vec(),
            }
        }
    };
    let rect = cov.rect;
    let w = rect.width() as usize;
    let mut data = vec![[0u8; 4]; cov.data.len()];
    if w > 0 {
        for (i, (px, &c)) in data.iter_mut().zip(&cov.data).enumerate() {
            if c == 0 {
                continue;
            }
            let (x, y) = (rect.left + (i % w) as i32, rect.top + (i / w) as i32);
            let (argb, c) = painter.shade(x, y, c);
            if c != 0 {
                *px = fill_pixel(argb, c);
            }
        }
    }
    let img = RgbaImage::from_raw(rect, data)
        .unwrap_or_else(|_| RgbaImage::empty_at(rect.left, rect.top));
    Ok(trimmed(img, fp_text::render::ink_bounds(layout)))
}

/// Colour of pixel `i`: the only run colour, or the run colours mixed by their feathered weights
/// (the first colour where all weights rounded to 0).
fn mix(colors: &[Rgba8], weights: &[Coverage], i: usize) -> Rgba8 {
    let first = colors.first().copied().unwrap_or(Rgba8::new(0, 0, 0, 255));
    if weights.is_empty() {
        return first;
    }
    let (mut r, mut g, mut b, mut a, mut s) = (0u32, 0u32, 0u32, 0u32, 0u32);
    for (c, w) in colors.iter().zip(weights) {
        let k = w.data.get(i).copied().unwrap_or(0) as u32;
        r += c.r as u32 * k;
        g += c.g as u32 * k;
        b += c.b as u32 * k;
        a += c.a as u32 * k;
        s += k;
    }
    if s == 0 {
        return first;
    }
    let avg = |v: u32| ((v + s / 2) / s).min(255) as u8;
    Rgba8::new(avg(r), avg(g), avg(b), avg(a))
}

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

use fp_color::Rgba8;
use fp_geom::IRect;
use fp_raster::{Mask, RgbaImage};
use fp_text::TextLayout;
use fp_vector::FillEdge;
use fp_vector::coverage::Coverage;
use fp_vector::feather::{MAX_FEATHER, feather, ordered_dither};

/// Pixels a text layer may hold at most before the edge pass gives up (like fp-text's raster
/// limit).
const MAX_PIXELS: u64 = 1 << 26;

/// Text image with a Feather or Dither fill edge; `None` for Hard and Anti-Alias (and width 0),
/// which the plain text renderer handles. Without `bounds` the result is cut to the pixels with
/// alpha plus a 1 px transparent margin, like [`fp_text::render::render`].
pub fn render_with_edge(layout: &TextLayout, edge: FillEdge) -> Option<RgbaImage> {
    let (r, dither) = match edge {
        FillEdge::Feather(r) if r > 0 => (r, false),
        FillEdge::Dither(r) if r > 0 => (r, true),
        _ => return None,
    };
    let r = r.min(MAX_FEATHER) as i32;
    let ink = fp_text::render::ink_bounds(layout);
    if ink.is_empty() {
        return Some(RgbaImage::empty_at(ink.left, ink.top));
    }
    // 2 px safety margin like the plain renderer (hinting and the rounded baseline shift the ink
    // by fractions of a pixel), then the feather reach on every side
    let window = ink.inflate(2 + r, 2 + r);
    let big = window.inflate(r, r);
    let (bw, bh) = (big.width() as u64, big.height() as u64);
    if bw * bh > MAX_PIXELS {
        return Some(RgbaImage::empty_at(window.left, window.top));
    }
    let soft = |m: Mask| -> Coverage {
        let cov = Coverage {
            rect: m.bounds(),
            data: m.data().to_vec(),
        };
        feather(&cov, r as u32, window)
    };
    let mut total = soft(fp_text::render::coverage(layout, Some(big)));
    if dither {
        ordered_dither(&mut total);
    }
    let colors = fp_text::render::ink_colors(layout);
    let weights: Vec<Coverage> = if colors.len() > 1 {
        colors
            .iter()
            .map(|c| soft(fp_text::render::color_coverage(layout, big, *c)))
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
    let ab: IRect = img.alpha_bounds();
    if ab.is_empty() {
        return Some(RgbaImage::empty_at(ink.left, ink.top));
    }
    Some(img.copy_rect(ab.inflate(1, 1)).unwrap_or(img))
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

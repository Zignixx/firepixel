//! Animation (08 §5.3): Status als Einzelbilder für animiertes GIF (Verzögerung, Schleife `FLPi`,
//! Löschmethoden, Auto-Crop/Auto-Differenz, Differenz-Transparenzindex `#FE0102`), APNG und
//! animiertes WebP.
//!
//! Ablauf animiertes GIF (`@00a5b8a0`):
//! 1. Versteckte Status (`VISb = 0`) fallen weg (A).
//! 2. Ab dem zweiten Bild mit Auto-Crop: Vergleich mit dem Vorgänger (ARGB vor der Quantisierung),
//!    Schranken mit `l = lossyGifAmount`: gleich ⇒ zählt nicht; Abstand² < (l+4)² ⇒ Vorgängerpixel,
//!    zählt nicht; < (l+8)² ⇒ bleibt, zählt nicht; sonst zählt. Bildrechteck = Hülle der zählenden
//!    Pixel (leer ⇒ 1×1). Löschmethode 2 wird zu 1, wenn kein Pixel von deckend nach transparent wechselt.
//! 3. Eine gemeinsame Palette über alle Bilder (A: Bilder untereinander gestapelt quantisiert).
//! 4. Transparenzindex: eigene Transparenz, sonst mit Auto-Differenz `n` (Palettengröße) – Farbe
//!    `#FE0102` nur zusammen mit Auto-Crop (Laufzeit T4); bei Zweierpotenz wächst die Bittiefe, bei 256
//!    gibt es keinen. Mit Auto-Differenz wird je Bild die kleinere von zwei Kodierungen genommen:
//!    ohne Bezug bzw. unveränderte Pixel transparent (A: die dritte Variante von `@00743a90` fehlt).
//! 5. NETSCAPE2.0 nur bei Schleife ≥ 0.

use fp_model::FrameInfo;

use crate::gif::{lzw_size, palette_bits, write_gif, GifFrame, GifOptions};
use crate::options::{ExportFormat, OptimizeSettings};
use crate::pipeline::{quantize_indexed, ExportContext};
use crate::png_out::{write_apng, ApngFrame, PngOptions};
use crate::px::{a, dist2};
use crate::quant::TRANSPARENCY_NONE;
use crate::{ExportError, Result};

/// Ein Status als volles Leinwandbild.
#[derive(Clone, Debug, PartialEq)]
pub struct AnimFrameSrc {
    /// Pixel `0xAARRGGBB`, `w·h`.
    pub pixels: Vec<u32>,
    /// Status-Eigenschaften (Verzögerung, Löschmethode, sichtbar).
    pub info: FrameInfo,
}

/// Rechteck (x, y, w, h).
pub type FrameRect = (usize, usize, usize, usize);

/// Ergebnis der Auto-Crop-Vorstufe je Bild.
#[derive(Clone, Debug, PartialEq)]
pub struct PreparedFrame {
    pub pixels: Vec<u32>,
    pub rect: FrameRect,
    pub disposal: u8,
    pub delay: u16,
}

/// Auto-Crop-Vorstufe (Schritt 2 der Moduldoku).
pub fn prepare_frames(frames: &[AnimFrameSrc], w: usize, h: usize, auto_crop: bool, lossy: i32) -> Vec<PreparedFrame> {
    let vis: Vec<&AnimFrameSrc> = frames.iter().filter(|f| f.info.visible).collect();
    let t1 = ((lossy + 4) as i64).pow(2);
    let t2 = ((lossy + 8) as i64).pow(2);
    let mut out: Vec<PreparedFrame> = Vec::with_capacity(vis.len());
    for (k, f) in vis.iter().enumerate() {
        let mut px = f.pixels.clone();
        px.resize(w * h, 0);
        // Laufzeit 08 T4: „nicht angegeben“ (0) im Dokument wird als 1 („nicht entfernen“) geschrieben.
        let mut disposal = match f.info.disposal.clamp(0, 3) as u8 {
            0 => 1,
            d => d,
        };
        let mut rect = (0, 0, w, h);
        if k > 0 && auto_crop {
            let prev = &out[k - 1].pixels;
            let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0usize, 0usize);
            let mut to_transparent = false;
            for y in 0..h {
                for x in 0..w {
                    let i = y * w + x;
                    let (c, p) = (px[i], prev[i]);
                    if a(p) != 0 && a(c) == 0 {
                        to_transparent = true;
                    }
                    if c == p {
                        continue;
                    }
                    let d = dist2(c, p) as i64 + (a(c) as i64 - a(p) as i64).pow(2);
                    if d < t1 {
                        px[i] = p;
                        continue;
                    }
                    if d < t2 {
                        continue;
                    }
                    x0 = x0.min(x);
                    y0 = y0.min(y);
                    x1 = x1.max(x + 1);
                    y1 = y1.max(y + 1);
                }
            }
            rect = if x0 == usize::MAX { (0, 0, 1, 1) } else { (x0, y0, x1 - x0, y1 - y0) };
            if disposal == 2 && !to_transparent {
                disposal = 1;
            }
        }
        out.push(PreparedFrame { pixels: px, rect, disposal, delay: f.info.delay.clamp(0, 65535) as u16 });
    }
    out
}

fn crop(indices: &[u8], w: usize, r: FrameRect) -> Vec<u8> {
    let (x, y, cw, ch) = r;
    let mut v = Vec::with_capacity(cw * ch);
    for yy in y..y + ch {
        v.extend_from_slice(&indices[yy * w + x..yy * w + x + cw]);
    }
    v
}

/// Animiertes GIF aus Status-Bildern.
pub fn animated_gif(frames: &[AnimFrameSrc], w: usize, h: usize, s: &OptimizeSettings, ctx: &ExportContext) -> Result<Vec<u8>> {
    if w == 0 || h == 0 || w > 65535 || h > 65535 {
        return Err(ExportError::BadSize(w, h));
    }
    let prepared = prepare_frames(frames, w, h, s.anim.auto_crop, s.lossy);
    if prepared.is_empty() {
        return Err(ExportError::Encode("keine sichtbaren Status".into()));
    }
    let mut stacked = Vec::with_capacity(w * h * prepared.len());
    for p in &prepared {
        stacked.extend_from_slice(&p.pixels);
    }
    let mut qs = s.clone();
    qs.format = ExportFormat::AnimatedGif;
    // A (Laufzeit 08 T4: GCT weiß, rot, blau): Passen alle Farben der Status in die gewünschte Zahl,
    // entsteht die Palette wie „Exakt“ (Luminanz absteigend) statt über den Variance-Quantisierer.
    if s.palette_kind == crate::options::PaletteKind::Adaptive
        && let Some(ex) = crate::palettes::palette_exact(&stacked, s.effective_matte(ctx.canvas))
        && ex.len() as i32 <= s.max_colors
    {
        qs.palette_kind = crate::options::PaletteKind::Exact;
    }
    let ix = quantize_indexed(&stacked, w, h * prepared.len(), &qs, ctx);
    let matte = s.effective_matte(ctx.canvas);
    let mut file_pal = ix.palette_rgb(matte);
    let own_transparency = ix.transparent_index.is_some() && s.transparency.code() != TRANSPARENCY_NONE;
    let mut diff_index: Option<usize> = None;
    if !own_transparency && s.anim.auto_difference && file_pal.len() < 256 {
        // Eintrag hinter der Palette; bei einer Zweierpotenz wächst die Bittiefe.
        diff_index = Some(file_pal.len());
        file_pal.push(if s.anim.auto_crop { [0xFE, 0x01, 0x02] } else { [0, 0, 0] });
    }
    let transparent = if own_transparency { ix.transparent_index.map(|t| t.min(255) as u8) } else { diff_index.map(|t| t as u8) };
    let mcs = palette_bits(file_pal.len()).max(2);
    let mut gframes = Vec::with_capacity(prepared.len());
    let mut prev_idx: Option<&[u8]> = None;
    for (k, p) in prepared.iter().enumerate() {
        let idx_all = &ix.indices[k * w * h..(k + 1) * w * h];
        let mut data = crop(idx_all, w, p.rect);
        if let (Some(d), Some(prev), true) = (diff_index, prev_idx, s.anim.auto_crop && k > 0) {
            // Variante 1: unveränderte Pixel transparent.
            let prev_c = crop(prev, w, p.rect);
            let alt: Vec<u8> = data.iter().zip(&prev_c).map(|(&c, &q)| if c == q { d as u8 } else { c }).collect();
            if lzw_size(&alt, mcs) < lzw_size(&data, mcs) {
                data = alt;
            }
        }
        prev_idx = Some(idx_all);
        gframes.push(GifFrame {
            indices: data,
            x: p.rect.0 as u16,
            y: p.rect.1 as u16,
            w: p.rect.2 as u16,
            h: p.rect.3 as u16,
            delay: p.delay,
            disposal: p.disposal,
            transparent,
            palette: None,
        });
    }
    let opts = GifOptions {
        loop_count: if s.anim.loop_count >= 0 { Some(s.anim.loop_count.min(65535) as u16) } else { None },
        comment: None,
        interlaced: s.interlaced,
    };
    Ok(write_gif(w as u16, h as u16, &file_pal, &gframes, &opts))
}

/// APNG aus Status-Bildern: erstes Bild voll, weitere auf das geänderte Rechteck beschnitten
/// (`blend_op = SOURCE`, `dispose_op = NONE`, daher verlustfrei gleichwertig).
pub fn apng(frames: &[AnimFrameSrc], w: usize, h: usize, ctx: &ExportContext, loop_count: i32) -> Result<Vec<u8>> {
    let vis: Vec<&AnimFrameSrc> = frames.iter().filter(|f| f.info.visible).collect();
    if vis.is_empty() {
        return Err(ExportError::Encode("keine sichtbaren Status".into()));
    }
    let mut out = Vec::with_capacity(vis.len());
    let mut prev: Option<Vec<u32>> = None;
    for f in vis {
        let mut px = f.pixels.clone();
        px.resize(w * h, 0);
        let rect = match &prev {
            None => (0, 0, w, h),
            Some(p) => {
                let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
                for y in 0..h {
                    for x in 0..w {
                        if px[y * w + x] != p[y * w + x] {
                            x0 = x0.min(x);
                            y0 = y0.min(y);
                            x1 = x1.max(x + 1);
                            y1 = y1.max(y + 1);
                        }
                    }
                }
                if x0 == usize::MAX { (0, 0, 1, 1) } else { (x0, y0, x1 - x0, y1 - y0) }
            }
        };
        let mut sub = Vec::with_capacity(rect.2 * rect.3);
        for y in rect.1..rect.1 + rect.3 {
            sub.extend_from_slice(&px[y * w + rect.0..y * w + rect.0 + rect.2]);
        }
        out.push(ApngFrame {
            x: rect.0 as u32,
            y: rect.1 as u32,
            w: rect.2 as u32,
            h: rect.3 as u32,
            pixels: sub,
            delay_cs: f.info.delay.clamp(0, 65535) as u16,
            dispose: 0,
            blend: 0,
        });
        prev = Some(px);
    }
    let plays = if loop_count <= 0 { if loop_count == 0 { 0 } else { 1 } } else { loop_count as u32 + 1 };
    write_apng(w as u32, h as u32, &out, plays, &PngOptions::from_context(ctx, false))
}

/// Animiertes WebP aus Status-Bildern.
pub fn animated_webp(frames: &[AnimFrameSrc], w: usize, h: usize, lossless: bool, quality: i32, loop_count: i32) -> Result<Vec<u8>> {
    let fr: Vec<crate::modern::WebpFrame> = frames
        .iter()
        .filter(|f| f.info.visible)
        .map(|f| {
            let mut p = f.pixels.clone();
            p.resize(w * h, 0);
            crate::modern::WebpFrame { pixels: p, delay_cs: f.info.delay.clamp(1, 65535) as u16 }
        })
        .collect();
    let lc = if loop_count < 0 { None } else { Some(loop_count.min(65535) as u16) };
    crate::modern::write_animated_webp(w, h, &fr, lossless, quality, lc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crop_rect_and_disposal() {
        let w = 8;
        let mk = |rx: usize| -> Vec<u32> { (0..64).map(|i| if i % 8 >= rx && i % 8 < rx + 2 && i / 8 < 2 { 0xFFFF0000 } else { 0xFFFFFFFF }).collect() };
        let frames = vec![
            AnimFrameSrc { pixels: mk(0), info: FrameInfo { disposal: 2, ..FrameInfo::default() } },
            AnimFrameSrc { pixels: mk(3), info: FrameInfo { disposal: 2, ..FrameInfo::default() } },
        ];
        let p = prepare_frames(&frames, w, 8, true, 0);
        assert_eq!(p[0].rect, (0, 0, 8, 8));
        assert_eq!(p[1].rect, (0, 0, 5, 2));
        assert_eq!(p[1].disposal, 1);
        let s = OptimizeSettings { format: ExportFormat::AnimatedGif, ..OptimizeSettings::default() };
        let g = animated_gif(&frames, 8, 8, &s, &ExportContext::default()).unwrap();
        assert_eq!(&g[..6], b"GIF89a");
        let ap = apng(&frames, 8, 8, &ExportContext::default(), 0).unwrap();
        assert_eq!(&ap[1..4], b"PNG");
    }
}

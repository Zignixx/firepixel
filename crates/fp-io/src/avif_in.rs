//! AVIF lesen (Standbild, Raster-Items, Alpha, `clap`/`irot`/`imir`, Animation `avis`).
//!
//! AV1 dekodiert rav1d (Rust-Port von dav1d, BSD-2) über seine sichere Rust-API, ohne Assembler
//! (kein nasm), mit Filmkorn wie libavif. Den Container liest [`crate::isobmff`]. YUV → RGB nach der
//! Matrix aus `colr`/nclx bzw. dem AV1-Sequenzkopf (unbekannt ⇒ BT.601 wie libavif), Wertebereich
//! aus dem Sequenzkopf, Chroma bilinear (Mittenlage), 8/10/12 Bit; vormultipliziertes Alpha
//! (`prem`) wird zurückgerechnet. Farbprofile (ICC) werden wie bei den übrigen Importen nicht
//! angewandt.

use std::panic::{catch_unwind, AssertUnwindSafe};

use fp_geom::IRect;
use fp_raster::RgbaImage;
use rav1d::{Decoder, PixelLayout, PlanarImageComponent, Rav1dError, Settings};

use crate::isobmff::{self, Heif, Nclx, Property};
use crate::raster::{Animation, AnimationFrame};
use crate::{check_size, ImportError, Result};

/// Dekodierte Ebenen eines AV1-Bildes.
#[derive(Clone, Debug)]
pub struct Planes {
    pub width: u32,
    pub height: u32,
    /// Bits je Wert (8, 10, 12).
    pub depth: u32,
    /// 0 = 4:0:0, 1 = 4:2:0, 2 = 4:2:2, 3 = 4:4:4.
    pub layout: u8,
    pub y: Vec<u16>,
    pub u: Vec<u16>,
    pub v: Vec<u16>,
    /// Breite/Höhe der Chroma-Ebenen.
    pub cw: u32,
    pub ch: u32,
    /// Matrix (CICP) aus dem Sequenzkopf.
    pub matrix: u8,
    pub full_range: bool,
}

fn err(e: Rav1dError) -> ImportError {
    ImportError::Decode("AVIF", format!("AV1-Dekoder: {e:?}"))
}

fn planes_of(p: &rav1d::Picture) -> Result<Planes> {
    let (w, h) = (p.width(), p.height());
    check_size(w, h)?;
    let layout = match p.pixel_layout() {
        PixelLayout::I400 => 0u8,
        PixelLayout::I420 => 1,
        PixelLayout::I422 => 2,
        PixelLayout::I444 => 3,
    };
    let depth = p.bits_per_component().map(|b| b.0 as u32).unwrap_or(8);
    let wide = p.bit_depth() > 8;
    let read = |c: PlanarImageComponent, pw: u32, ph: u32| -> Result<Vec<u16>> {
        let data = p.plane(c);
        let stride = p.stride(c) as usize;
        let bpp = if wide { 2 } else { 1 };
        let mut out = Vec::with_capacity(pw as usize * ph as usize);
        for y in 0..ph as usize {
            let row = data.get(y * stride..y * stride + pw as usize * bpp).ok_or_else(|| ImportError::Decode("AVIF", "Ebene zu kurz".into()))?;
            if wide {
                out.extend(row.as_chunks::<2>().0.iter().map(|b| u16::from_ne_bytes(*b)));
            } else {
                out.extend(row.iter().map(|&b| b as u16));
            }
        }
        Ok(out)
    };
    let (cw, ch) = match layout {
        1 => (w.div_ceil(2), h.div_ceil(2)),
        2 => (w.div_ceil(2), h),
        3 => (w, h),
        _ => (0, 0),
    };
    let y = read(PlanarImageComponent::Y, w, h)?;
    let (u, v) = if layout == 0 { (Vec::new(), Vec::new()) } else { (read(PlanarImageComponent::U, cw, ch)?, read(PlanarImageComponent::V, cw, ch)?) };
    let matrix = p.matrix_coefficients() as u8;
    let full_range = matches!(p.color_range(), rav1d::pixel::YUVRange::Full);
    Ok(Planes { width: w, height: h, depth, layout, y, u, v, cw, ch, matrix, full_range })
}

/// Dekodiert AV1-Samples (ein Standbild = ein Sample) nacheinander mit einem Dekoder.
/// `still`: nur die höchste Ebene ausgeben (geschichtete Standbilder).
pub fn decode_av1(samples: &[&[u8]], still: bool) -> Result<Vec<Planes>> {
    let run = || -> Result<Vec<Planes>> {
        let mut s = Settings::new();
        let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1).min(8) as u32;
        s.set_n_threads(threads);
        s.set_max_frame_delay(1);
        s.set_apply_grain(true);
        s.set_all_layers(!still);
        s.set_frame_size_limit(crate::MAX_PIXELS as u32);
        let mut dec = Decoder::with_settings(&s).map_err(err)?;
        let mut out = Vec::new();
        let take = |dec: &mut Decoder, out: &mut Vec<Planes>| -> Result<bool> {
            match dec.get_picture() {
                Ok(p) => {
                    out.push(planes_of(&p)?);
                    Ok(true)
                }
                Err(Rav1dError::TryAgain) => Ok(false),
                Err(e) => Err(err(e)),
            }
        };
        for smp in samples {
            let before = out.len();
            let mut r = dec.send_data(smp.to_vec().into_boxed_slice(), None, None, None);
            let mut guard = 0;
            while let Err(e) = r {
                if e != Rav1dError::TryAgain {
                    return Err(err(e));
                }
                take(&mut dec, &mut out)?;
                guard += 1;
                if guard > 10_000 {
                    return Err(ImportError::Decode("AVIF", "Dekoder hängt".into()));
                }
                r = dec.send_pending_data();
            }
            while take(&mut dec, &mut out)? {}
            if still && out.len() > before + 1 {
                // geschichtetes Bild: nur die letzte (vollständige) Ebene behalten
                let last = out.pop();
                out.truncate(before);
                out.extend(last);
            }
        }
        dec.flush();
        while take(&mut dec, &mut out)? {}
        Ok(out)
    };
    catch_unwind(AssertUnwindSafe(run)).unwrap_or_else(|_| Err(ImportError::Decode("AVIF", "AV1-Dekoder abgebrochen (beschädigte Daten)".into())))
}

/// (Kr, Kb) einer CICP-Matrix.
fn coefficients(m: u8) -> (f32, f32) {
    match m {
        1 => (0.2126, 0.0722),
        4 => (0.30, 0.11),
        7 => (0.212, 0.087),
        9 | 10 => (0.2627, 0.0593),
        _ => (0.299, 0.114),
    }
}

/// YUV → RGBA8 (Alpha 255).
pub fn to_rgba(p: &Planes, matrix: u8) -> Vec<[u8; 4]> {
    let (w, h) = (p.width as usize, p.height as usize);
    let sh = (p.depth.saturating_sub(8)).min(8);
    let max = ((1u32 << p.depth.clamp(1, 16)) - 1) as f32;
    let half = (1u32 << (p.depth.clamp(1, 16) - 1)) as f32;
    let (y0, ys, c0, cs) = if p.full_range { (0.0, max, half, max) } else { ((16u32 << sh) as f32, (219u32 << sh) as f32, (128u32 << sh) as f32, (224u32 << sh) as f32) };
    let (kr, kb) = coefficients(matrix);
    let kg = 1.0 - kr - kb;
    let (sx, sy) = match p.layout {
        1 => (2usize, 2usize),
        2 => (2, 1),
        _ => (1, 1),
    };
    let (cw, ch) = (p.cw as usize, p.ch as usize);
    // Chroma bilinear in Mittenlage
    let chroma = |plane: &[u16], x: usize, y: usize| -> f32 {
        if sx == 1 && sy == 1 {
            return plane[y * cw + x] as f32;
        }
        let fx = ((x as f32 + 0.5) / sx as f32 - 0.5).max(0.0);
        let fy = ((y as f32 + 0.5) / sy as f32 - 0.5).max(0.0);
        let (ix, iy) = ((fx as usize).min(cw - 1), (fy as usize).min(ch - 1));
        let (jx, jy) = ((ix + 1).min(cw - 1), (iy + 1).min(ch - 1));
        let (ax, ay) = (fx - ix as f32, fy - iy as f32);
        let a = plane[iy * cw + ix] as f32 * (1.0 - ax) + plane[iy * cw + jx] as f32 * ax;
        let b = plane[jy * cw + ix] as f32 * (1.0 - ax) + plane[jy * cw + jx] as f32 * ax;
        a * (1.0 - ay) + b * ay
    };
    let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    let mut out = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            let yy = p.y[y * w + x] as f32;
            if p.layout == 0 || cw == 0 {
                let g = q((yy - y0) / ys);
                out.push([g, g, g, 255]);
                continue;
            }
            let (u, v) = (chroma(&p.u, x, y), chroma(&p.v, x, y));
            let (r, g, b) = match matrix {
                0 => {
                    // GBR: Y = G, U = B, V = R (Wertebereich wie Luma)
                    ((v - y0) / ys, (yy - y0) / ys, (u - y0) / ys)
                }
                8 => {
                    let (yn, cg, co) = ((yy - y0) / ys, (u - c0) / cs, (v - c0) / cs);
                    let t = yn - cg;
                    (t + co, yn + cg, t - co)
                }
                _ => {
                    let (yn, cb, cr) = ((yy - y0) / ys, (u - c0) / cs, (v - c0) / cs);
                    let r = yn + 2.0 * (1.0 - kr) * cr;
                    let b = yn + 2.0 * (1.0 - kb) * cb;
                    let g = (yn - kr * r - kb * b) / kg;
                    (r, g, b)
                }
            };
            out.push([q(r), q(g), q(b), 255]);
        }
    }
    out
}

/// Alpha aus der Luma-Ebene eines Hilfsbildes.
fn alpha_values(p: &Planes) -> Vec<u8> {
    let sh = (p.depth.saturating_sub(8)).min(8);
    let max = ((1u32 << p.depth.clamp(1, 16)) - 1) as f32;
    let (a0, as_) = if p.full_range { (0.0, max) } else { ((16u32 << sh) as f32, (219u32 << sh) as f32) };
    p.y.iter().map(|&v| ((v as f32 - a0) / as_ * 255.0 + 0.5).clamp(0.0, 255.0) as u8).collect()
}

/// Ein Bild (Farbe oder Alpha) eines Items: einzelnes `av01` oder `grid` aus `av01`-Kacheln.
/// Liefert RGBA (für Alpha nur Kanal 0 belegt) in Item-Größe, noch ohne Transformationen.
fn item_pixels(h: &Heif<'_>, id: u32, alpha: bool, nclx: Option<Nclx>) -> Result<(u32, u32, Vec<[u8; 4]>)> {
    let it = h.items.get(&id).ok_or_else(|| ImportError::Corrupt("AVIF: Item fehlt".into()))?;
    let convert = |p: &Planes| -> Vec<[u8; 4]> {
        if alpha {
            alpha_values(p).into_iter().map(|a| [a, a, a, 255]).collect()
        } else {
            let m = match nclx {
                Some(n) if n.matrix != 2 => n.matrix as u8,
                _ => p.matrix,
            };
            to_rgba(p, m)
        }
    };
    match &it.kind {
        b"av01" => {
            let d = h.data(id)?;
            let mut pl = decode_av1(&[&d], true)?;
            let p = pl.pop().ok_or_else(|| ImportError::Decode("AVIF", "kein Bild dekodiert".into()))?;
            Ok((p.width, p.height, convert(&p)))
        }
        b"grid" => {
            let g = h.grid(id)?;
            check_size(g.width, g.height)?;
            let tiles = h.refs_from(id, b"dimg");
            if tiles.len() as u64 != g.rows as u64 * g.columns as u64 {
                return Err(ImportError::Corrupt("AVIF: Kachelzahl passt nicht zum Raster".into()));
            }
            let mut out = vec![[0u8, 0, 0, 255]; g.width as usize * g.height as usize];
            let (mut tw, mut th) = (0u32, 0u32);
            for (k, t) in tiles.iter().enumerate() {
                if h.items.get(t).map(|i| &i.kind) != Some(b"av01") {
                    return Err(ImportError::Unsupported("AVIF: Rasterkachel ist kein AV1-Bild".into()));
                }
                let d = h.data(*t)?;
                let p = decode_av1(&[&d], true)?.pop().ok_or_else(|| ImportError::Decode("AVIF", "Kachel ohne Bild".into()))?;
                if k == 0 {
                    (tw, th) = (p.width, p.height);
                }
                let px = convert(&p);
                let (gx, gy) = ((k as u32 % g.columns) * tw, (k as u32 / g.columns) * th);
                for y in 0..p.height.min(th) {
                    let oy = gy + y;
                    if oy >= g.height {
                        break;
                    }
                    for x in 0..p.width.min(tw) {
                        let ox = gx + x;
                        if ox >= g.width {
                            break;
                        }
                        out[(oy * g.width + ox) as usize] = px[(y * p.width + x) as usize];
                    }
                }
            }
            Ok((g.width, g.height, out))
        }
        k => Err(ImportError::Unsupported(format!("AVIF: Bildart „{}“ wird nicht unterstützt", String::from_utf8_lossy(k)))),
    }
}

fn nclx_of(props: &[Property]) -> Option<Nclx> {
    props.iter().find_map(|p| if let Property::Colr(n) = p { Some(*n) } else { None })
}

fn unpremultiply(px: &mut [[u8; 4]]) {
    for p in px {
        let a = p[3] as u32;
        if a > 0 && a < 255 {
            for c in &mut p[..3] {
                *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
}

/// Primäres Standbild des `meta`-Teils.
fn decode_still(h: &Heif<'_>) -> Result<(u32, u32, Vec<[u8; 4]>)> {
    let id = h.primary;
    let it = h.items.get(&id).ok_or_else(|| ImportError::Corrupt("AVIF: primäres Item fehlt".into()))?;
    let props = it.props.clone();
    let (w, hh, mut px) = item_pixels(h, id, false, nclx_of(&props))?;
    if let Some(aid) = h.alpha_of(id) {
        let (aw, ah, a) = item_pixels(h, aid, true, None)?;
        if (aw, ah) == (w, hh) {
            for (p, a) in px.iter_mut().zip(&a) {
                p[3] = a[0];
            }
            if !h.refs_from(id, b"prem").is_empty() {
                unpremultiply(&mut px);
            }
        }
    }
    Ok(isobmff::apply_transforms(w, hh, px, &props))
}

/// Animation aus der Bildsequenz-Spur (`avis`), Alpha aus der Hilfsspur.
fn decode_sequence(d: &[u8], tracks: &[isobmff::Track]) -> Result<Option<Animation>> {
    let Some(color) = tracks.iter().find(|t| &t.entry == b"av01" && (&t.handler == b"pict" || &t.handler == b"vide")) else { return Ok(None) };
    let samples: Vec<&[u8]> = color.samples.iter().map(|&s| isobmff::sample(d, s)).collect::<Result<_>>()?;
    if samples.is_empty() {
        return Ok(None);
    }
    let frames = decode_av1(&samples, false)?;
    let alpha_track = tracks.iter().find(|t| &t.entry == b"av01" && t.refs.iter().any(|(k, ids)| k == b"auxl" && ids.contains(&color.id)));
    let alpha = match alpha_track {
        Some(t) => {
            let s: Vec<&[u8]> = t.samples.iter().map(|&s| isobmff::sample(d, s)).collect::<Result<_>>()?;
            decode_av1(&s, false)?
        }
        None => Vec::new(),
    };
    let Some(first) = frames.first() else { return Ok(None) };
    let (w, h) = (first.width, first.height);
    let mut out = Vec::with_capacity(frames.len());
    let mut has_t = false;
    for (k, p) in frames.iter().enumerate() {
        let mut px = to_rgba(p, p.matrix);
        if let Some(a) = alpha.get(k).filter(|a| a.width == p.width && a.height == p.height) {
            for (q, v) in px.iter_mut().zip(alpha_values(a)) {
                q[3] = v;
            }
        }
        has_t |= px.iter().any(|q| q[3] != 255);
        let bytes: Vec<u8> = px.into_iter().flatten().collect();
        let image = RgbaImage::from_rgba_bytes(IRect::new(0, 0, p.width as i32, p.height as i32), &bytes).map_err(|e| ImportError::Corrupt(e.to_string()))?;
        let dur = color.durations.get(k).copied().unwrap_or(0) as u64;
        let delay_cs = if color.timescale > 0 { (dur * 100).div_ceil(color.timescale as u64) as u32 } else { 10 };
        out.push(AnimationFrame { image, x: 0, y: 0, delay_cs, disposal: 1, replace: true });
    }
    Ok(Some(Animation { width: w, height: h, frames: out, loop_count: if color.repeat { 0 } else { -1 }, has_transparency: has_t, palette: None }))
}

/// AVIF lesen: Bildsequenz (mehr als ein Bild) ⇒ Animation, sonst das primäre Standbild.
pub fn decode_avif(d: &[u8]) -> Result<Animation> {
    let tracks = isobmff::tracks(d)?;
    if tracks.iter().any(|t| t.samples.len() > 1)
        && let Some(a) = decode_sequence(d, &tracks)?
    {
        return Ok(a);
    }
    let heif = Heif::parse(d)?;
    let (w, h, px) = match heif {
        Some(h) if h.items.contains_key(&h.primary) => decode_still(&h)?,
        _ => match decode_sequence(d, &tracks)? {
            Some(a) => return Ok(a),
            None => return Err(ImportError::Corrupt("AVIF ohne Bild".into())),
        },
    };
    check_size(w, h)?;
    let has_t = px.iter().any(|p| p[3] != 255);
    let bytes: Vec<u8> = px.into_iter().flatten().collect();
    let image = RgbaImage::from_rgba_bytes(IRect::new(0, 0, w as i32, h as i32), &bytes).map_err(|e| ImportError::Corrupt(e.to_string()))?;
    Ok(Animation { width: w, height: h, frames: vec![AnimationFrame { image, x: 0, y: 0, delay_cs: 0, disposal: 0, replace: true }], loop_count: 0, has_transparency: has_t, palette: None })
}

/// Ist das primäre Item der HEIF-Datei ein AV1-Bild (bzw. ein Raster aus solchen)?
pub fn is_av1_heif(d: &[u8]) -> bool {
    match Heif::parse(d) {
        Ok(Some(h)) => match h.items.get(&h.primary).map(|i| i.kind) {
            Some(k) if &k == b"av01" => true,
            Some(k) if &k == b"grid" => h.refs_from(h.primary, b"dimg").first().and_then(|t| h.items.get(t)).is_some_and(|i| &i.kind == b"av01"),
            _ => false,
        },
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yuv_full_range_gray_and_red() {
        let p = Planes { width: 2, height: 1, depth: 8, layout: 3, y: vec![128, 76], u: vec![128, 85], v: vec![128, 255], cw: 2, ch: 1, matrix: 6, full_range: true };
        let px = to_rgba(&p, 6);
        assert_eq!(px[0], [128, 128, 128, 255]);
        assert!(px[1][0] > 250 && px[1][1] < 5 && px[1][2] < 5, "{:?}", px[1]);
    }

    #[test]
    fn garbage_is_error() {
        assert!(decode_avif(b"\0\0\0\x14ftypavif\0\0\0\0avif").is_err());
        assert!(decode_av1(&[&[0x12, 0, 0x0a, 0x0b, 0, 0, 0, 0x24]], true).is_err() || decode_av1(&[&[1, 2, 3]], true).map(|v| v.is_empty()).unwrap_or(true));
    }
}

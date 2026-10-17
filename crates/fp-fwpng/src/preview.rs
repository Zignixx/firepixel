//! Erzeugung der Vorschau `prVW` (Spezifikation 02 §12.8).
//!
//! Ablauf je Seite (Masterseiten bekommen keinen Chunk):
//! 1. Miniaturbild 112×112, weiß gefüllt. `s = min(108/W, 108/H)` als `f32`, `iw = ceil(W·s)`,
//!    `ih = ceil(H·s)` (Produkt in `f64`), Renderfaktor `s2 = min(ih/H, iw/W)`, Seite mit `s2` gerendert
//!    und bei `(⌊(112−iw)/2⌋, ⌊(112−ih)/2⌋)` eingesetzt.
//! 2. Quantisieren auf die feste Rasterpalette mit 6 Stufen (216 Webfarben, Weiß zuerst) mit der
//!    Original-Fehlerdiffusion (100 %, Serpentine, Muster ±6), Matte Weiß für halbtransparente Pixel.
//! 3. „Optimiert“: nur benutzte Farben, lückenlos in Rasterreihenfolge neu nummeriert; Rest der Palette 0.
//! 4. Indizes im 128er-Raster, ab CS5 Seitenname (UTF-16LE) angehängt, zlib.

use crate::chunks::{PV_STRIDE, Preview};
use fp_model::FlatImage;

/// Kantenlänge des Miniaturbilds.
pub const THUMB: u32 = 112;

/// Lage des Seitenbilds im Miniaturbild.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThumbGeometry {
    /// Größe des gerenderten Seitenbilds.
    pub width: u32,
    pub height: u32,
    /// Einsetzposition im 112×112-Bild.
    pub dx: u32,
    pub dy: u32,
    /// Renderfaktor `s2`.
    pub scale: f64,
}

/// Geometrie des Miniaturbilds für eine Seite der Größe `w`×`h` (02 §12.8, `@00da9ab0`).
pub fn thumbnail_geometry(w: f32, h: f32) -> Option<ThumbGeometry> {
    if !(w > 0.0 && h > 0.0 && w.is_finite() && h.is_finite()) {
        return None;
    }
    let s = (108.0f32 / w).min(108.0f32 / h);
    let ih = ((h as f64) * (s as f64)).ceil().clamp(1.0, THUMB as f64) as u32;
    let iw = ((w as f64) * (s as f64)).ceil().clamp(1.0, THUMB as f64) as u32;
    let s2 = (ih as f64 / h as f64).min(iw as f64 / w as f64);
    Some(ThumbGeometry {
        width: iw,
        height: ih,
        dx: (THUMB - iw) / 2,
        dy: (THUMB - ih) / 2,
        scale: s2,
    })
}

/// Vorschaugröße des Rückfallwegs ohne Miniaturbild (CS5 und älter): Achsen unter 112 auf 112
/// erweitert, `s = max/112`, Größe `ceil(W'/s) × ceil(H'/s)` (02 §12.8 Punkt 5).
pub fn fallback_size(w: f32, h: f32) -> (u16, u16) {
    let w2 = (w as f64).max(112.0);
    let h2 = (h as f64).max(112.0);
    let s = w2.max(h2) / 112.0;
    let pw = (w2 / s - 1e-6).ceil().clamp(1.0, 128.0) as u16;
    let ph = (h2 / s - 1e-6).ceil().clamp(1.0, 128.0) as u16;
    (pw, ph)
}

/// Rasterpalette mit `levels` Stufen (2..6), Reihenfolge Weiß → Schwarz (R außen, B innen).
pub fn grid_palette(levels: u32) -> Vec<[u8; 3]> {
    let l = levels.clamp(2, 6);
    let n = l - 1;
    let mut v = Vec::with_capacity((l * l * l) as usize);
    for r in (0..=n).rev() {
        for g in (0..=n).rev() {
            for b in (0..=n).rev() {
                v.push([
                    (r * 255 / n) as u8,
                    (g * 255 / n) as u8,
                    (b * 255 / n) as u8,
                ]);
            }
        }
    }
    v
}

/// Index der nächsten Rasterfarbe (Original-Formel für Rasterpaletten).
fn grid_index(r: i32, g: i32, b: i32, l: i32) -> usize {
    let f = |v: i32| l - (v * (l - 1) + 0x80) / 0xFF - 1;
    ((f(r) * l + f(g)) * l + f(b)) as usize
}

/// Halbtransparentes Pixel über die Matte-Farbe mischen (08 §2.1).
fn matte_blend(p: [u8; 4], matte: [u8; 3]) -> [i32; 3] {
    let a = p[3] as i32;
    if a == 255 {
        return [p[0] as i32, p[1] as i32, p[2] as i32];
    }
    let mut o = [0i32; 3];
    for k in 0..3 {
        let m = matte[k] as i32;
        let kk = (p[k] as i32 - m) * a + 0x80 + m * 255;
        o[k] = ((kk + (kk >> 8)) >> 8) & 255;
    }
    o
}

/// Original-Fehlerdiffusion (08 §3, `@0074c530`) auf eine Rasterpalette mit `levels` Stufen.
/// Eingabe RGBA8, Matte Weiß; Ausgabe Rasterindizes je Pixel.
pub fn diffuse_to_grid(rgba: &[u8], w: usize, h: usize, levels: u32, percent: i32) -> Vec<u8> {
    const PATTERN: [i32; 16] = [6, 0, -6, 0, 0, 0, 0, 0, -6, 0, 6, 0, 0, 0, 0, 0];
    let l = levels.clamp(2, 6) as i32;
    let pal = grid_palette(levels);
    let (pct, thr) = if percent >= 100 {
        (100, 2)
    } else {
        (percent.max(10), 5)
    };
    let mut out = vec![0u8; w * h];
    let mut up = vec![[0i32; 3]; w + 1];
    let tdiv = |a: i32, b: i32| a / b; // Rust-Division schneidet wie C Richtung 0 ab.
    // Zu kurze Eingaben werden mit Weiß aufgefüllt (keine Panik bei fehlerhaften Aufrufen).
    let padded;
    let rgba = if rgba.len() < w * h * 4 {
        let mut v = rgba.to_vec();
        v.resize(w * h * 4, 255);
        padded = v;
        &padded[..]
    } else {
        rgba
    };
    for y in 0..h {
        let mut carry = [0i32; 3];
        for step in 0..w {
            let x = if y % 2 == 0 { step } else { w - 1 - step };
            let o = (y * w + x) * 4;
            let px = [rgba[o], rgba[o + 1], rgba[o + 2], rgba[o + 3]];
            let mut v = matte_blend(px, [255, 255, 255]);
            let pat = PATTERN[(x & 3) + (y & 3) * 4];
            for k in 0..3 {
                let mut e = up[x][k] * 0x56 + carry[k] * 0xAA;
                e = tdiv(e, 256);
                e = tdiv(e * pct, 100);
                v[k] = v[k] + tdiv(e * pct, 100) + tdiv(e * pat, 10);
            }
            let cl = [v[0].clamp(0, 255), v[1].clamp(0, 255), v[2].clamp(0, 255)];
            let idx = grid_index(cl[0], cl[1], cl[2], l);
            out[y * w + x] = idx as u8;
            let pc = pal[idx.min(pal.len() - 1)];
            for k in 0..3 {
                let diff = v[k] - pc[k] as i32;
                let old = carry[k];
                let mut ne = diff + (tdiv(old, 2) - old);
                if diff.abs() < ne.abs() {
                    ne = diff;
                }
                if ne.abs() < thr {
                    ne = 0;
                }
                ne = ne.clamp(-255, 255);
                carry[k] = ne;
                up[x][k] = ne;
            }
        }
    }
    out
}

/// „Optimiert“: benutzte Rasterindizes lückenlos in Rasterreihenfolge neu nummerieren.
/// Liefert (neue Indizes, Palette 256×RGB mit den benutzten Farben vorn, Rest 0).
pub fn compact_palette(grid_idx: &[u8], levels: u32) -> (Vec<u8>, Vec<u8>) {
    let pal = grid_palette(levels);
    let mut used = [false; 256];
    for &i in grid_idx {
        used[i as usize] = true;
    }
    let mut map = [0u8; 256];
    let mut palette = vec![0u8; 768];
    let mut k = 0usize;
    for (i, u) in used.iter().enumerate() {
        if *u && i < pal.len() {
            map[i] = k as u8;
            palette[k * 3..k * 3 + 3].copy_from_slice(&pal[i]);
            k += 1;
        }
    }
    (grid_idx.iter().map(|&i| map[i as usize]).collect(), palette)
}

/// Baut eine Vorschau aus einem fertigen Bild (`w`×`h` ≤ 128, RGBA8).
pub fn build_preview(rgba: &[u8], w: u16, h: u16, page_name: Option<String>) -> Preview {
    let (wu, hu) = (w as usize, h as usize);
    let idx = diffuse_to_grid(rgba, wu, hu, 6, 100);
    let (compact, palette) = compact_palette(&idx, 6);
    let mut indices = vec![0u8; PV_STRIDE * PV_STRIDE];
    for y in 0..hu {
        indices[y * PV_STRIDE..y * PV_STRIDE + wu].copy_from_slice(&compact[y * wu..y * wu + wu]);
    }
    Preview {
        width: w,
        height: h,
        palette,
        indices,
        page_name,
        raw_len: 0,
    }
}

/// Setzt ein gerendertes Seitenbild (Größe laut `geom`) in ein weißes 112×112-Bild ein.
pub fn compose_thumbnail(page_img: &FlatImage, geom: &ThumbGeometry) -> Vec<u8> {
    let t = THUMB as usize;
    let mut out = vec![255u8; t * t * 4];
    for y in 0..(page_img.height as usize).min(geom.height as usize) {
        let ty = y + geom.dy as usize;
        if ty >= t {
            break;
        }
        for x in 0..(page_img.width as usize).min(geom.width as usize) {
            let tx = x + geom.dx as usize;
            if tx >= t {
                break;
            }
            let s = (y * page_img.width as usize + x) * 4;
            let d = (ty * t + tx) * 4;
            if let Some(src) = page_img.rgba.get(s..s + 4) {
                out[d..d + 4].copy_from_slice(src);
            }
        }
    }
    out
}

/// Rückfall ohne Renderer: verkleinert das flache Seitenbild (Dokumentgröße) per nächstem Nachbarn auf
/// die Miniaturbildgröße. Das Original rendert die Seite neu (Bitmaps per nächstem Nachbarn, Vektoren neu
/// gerastert, Laufzeit 02 T3); das hier ist nur eine Näherung.
pub fn downscale_nearest(img: &FlatImage, geom: &ThumbGeometry) -> FlatImage {
    let (w, h) = (geom.width as usize, geom.height as usize);
    let mut out = vec![0u8; w * h * 4];
    if img.width == 0 || img.height == 0 || geom.scale <= 0.0 {
        return FlatImage {
            width: geom.width,
            height: geom.height,
            rgba: out,
        };
    }
    for y in 0..h {
        let sy = (((y as f64) + 0.5) / geom.scale)
            .floor()
            .clamp(0.0, img.height as f64 - 1.0) as usize;
        for x in 0..w {
            let sx = (((x as f64) + 0.5) / geom.scale)
                .floor()
                .clamp(0.0, img.width as f64 - 1.0) as usize;
            let s = (sy * img.width as usize + sx) * 4;
            let d = (y * w + x) * 4;
            if let Some(src) = img.rgba.get(s..s + 4) {
                out[d..d + 4].copy_from_slice(src);
            }
        }
    }
    FlatImage {
        width: geom.width,
        height: geom.height,
        rgba: out,
    }
}

/// Komplette Vorschau einer Seite aus ihrem gerenderten Bild (Größe laut [`thumbnail_geometry`]).
pub fn preview_from_render(
    page_img: &FlatImage,
    geom: &ThumbGeometry,
    page_name: Option<String>,
) -> Preview {
    let thumb = compose_thumbnail(page_img, geom);
    build_preview(&thumb, THUMB as u16, THUMB as u16, page_name)
}

/// Leere (weiße) Vorschau, z. B. bei leeren Seiten oder Speichermangel.
pub fn blank_preview(page_name: Option<String>) -> Preview {
    let thumb = vec![255u8; (THUMB * THUMB * 4) as usize];
    build_preview(&thumb, THUMB as u16, THUMB as u16, page_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometry() {
        let g = thumbnail_geometry(54.0, 27.0).unwrap();
        assert_eq!((g.width, g.height), (108, 54));
        assert_eq!((g.dx, g.dy), (2, 29));
        let g = thumbnail_geometry(216.0, 216.0).unwrap();
        assert_eq!((g.width, g.height, g.scale), (108, 108, 0.5));
        assert_eq!(fallback_size(50.0, 61.0), (112, 112));
        assert_eq!(fallback_size(800.0, 400.0), (112, 56));
        assert!(thumbnail_geometry(0.0, 5.0).is_none());
    }

    #[test]
    fn palette_order_and_compaction() {
        let p = grid_palette(6);
        assert_eq!(p[0], [255, 255, 255]);
        assert_eq!(p[215], [0, 0, 0]);
        assert_eq!(grid_index(255, 255, 255, 6), 0);
        assert_eq!(grid_index(0, 0, 0, 6), 215);
        let (idx, pal) = compact_palette(&[215, 0, 0, 7], 6);
        assert_eq!(idx, vec![2, 0, 0, 1]);
        assert_eq!(&pal[..3], &[255, 255, 255]);
        assert_eq!(&pal[6..9], &[0, 0, 0]);
    }

    #[test]
    fn efefef_row_dithers_white_and_grey() {
        // Konstante Fläche #EFEFEF: Zeile 0 wechselt zwischen ffffff und cccccc (02 §12.8, 25/25).
        let w = 112;
        let rgba: Vec<u8> = (0..w * 2).flat_map(|_| [0xef, 0xef, 0xef, 0xff]).collect();
        let idx = diffuse_to_grid(&rgba, w, 2, 6, 100);
        let pal = grid_palette(6);
        let row: Vec<[u8; 3]> = idx[..w].iter().map(|&i| pal[i as usize]).collect();
        assert!(
            row.iter()
                .all(|c| *c == [255, 255, 255] || *c == [0xcc, 0xcc, 0xcc])
        );
        assert!(row.contains(&[0xcc, 0xcc, 0xcc]));
    }
}

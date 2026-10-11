//! RGBA-Palette für PNG8 mit Alphakanal (08 §2.7) und Palettennachbearbeitung beim Export (08 §2.6):
//! Exportsortierung, bearbeitete Farben, „Optimiert“ (unbenutzte Farben entfernen).
//!
//! Bitgenau nach `re\export\fw_rgba.py` (Laufzeit T2: Palette 256/256 und Indexbild 16.384/16.384,
//! mit Diffusion über `remove_unused`).

use crate::px::{a, argb, b, g, lum, r, tdiv, websnap};
use crate::quant::{Quantizer, TRANSPARENCY_INDEX_ALPHA, TRANSPARENCY_RGBA};

/// Obergrenze der RGB-Palette je nach Alphastufen (`@0074b000`).
pub fn alpha_cap(levels: u32) -> usize {
    match levels {
        0..=2 => 245,
        3 => 231,
        _ => 200,
    }
}

/// Alphawert der Stufe `j` (0-basiert): `((levels div 2) − 1 + (j+1)·255) div levels`.
pub fn alpha_value(j: u32, levels: u32) -> u32 {
    (levels / 2 - 1 + (j + 1) * 255) / levels
}

/// RGBA-Quantisierer (`@0074b000`, das Original übergibt immer 15 Alphastufen). Liefert den
/// RGB-Quantisierer der Vorstufe und die RGBA-Palette (Eintrag 0 = Weiß mit Alpha 0, sortiert nach
/// Alpha aufsteigend, bei gleichem Alpha Luminanz absteigend).
pub fn quantize_rgba(
    pixels: &[u32],
    n_requested: i32,
    levels: u32,
    websnap_on: bool,
    tolerance: i32,
    locked: &[u32],
    width: usize,
) -> (Quantizer, Vec<u32>) {
    let levels = levels.clamp(1, 15);
    let n = (n_requested.max(0) as usize).min(alpha_cap(levels));
    let mut q = Quantizer::new(TRANSPARENCY_RGBA, 0);
    q.build_adaptive(pixels, n as i32, websnap_on, tolerance, locked, None, width);
    let rgb = q.palette.clone();
    let mut exact = std::collections::HashMap::new();
    for (i, &c) in rgb.iter().enumerate().rev() {
        exact.insert(c, i);
    }
    let mut hist = vec![0u64; levels as usize * 256];
    for &c in pixels {
        let lev = (a(c) * levels + 0x80) / 0xFF;
        if lev == 0 {
            continue;
        }
        let cc = c | 0xFF00_0000;
        let idx = match exact.get(&cc) {
            Some(&i) => i,
            None => q.nearest(cc),
        };
        let k = idx + (lev as usize - 1) * 256;
        if let Some(h) = hist.get_mut(k) {
            *h += 1;
        }
    }
    let mut pal = vec![0x00FF_FFFFu32];
    while pal.len() < 256 {
        let mut best: Option<usize> = None;
        for (k, &v) in hist.iter().enumerate() {
            if v > 0 && best.is_none_or(|bk| hist[bk] < v) {
                best = Some(k);
            }
        }
        let Some(bk) = best else { break };
        let c = rgb.get(bk & 0xFF).copied().unwrap_or(0xFFFF_FFFF);
        pal.push(argb(alpha_value((bk >> 8) as u32, levels), r(c), g(c), b(c)));
        hist[bk] = 0;
    }
    pal.sort_by(|&x, &y| a(x).cmp(&a(y)).then(lum(y).cmp(&lum(x))));
    (q, pal)
}

/// Nächste RGBA-Farbe (`@00741cb0`): Alpha < 9 ⇒ Index 0, sonst erste minimale quadratische
/// Distanz über R, G, B und A (Start `0xFFFFF`, strikt `<`, Abbruch bei 0).
pub fn nearest_rgba(pal: &[u32], c: u32) -> usize {
    if a(c) < 9 {
        return 0;
    }
    let mut best = 0xFFFFFi64;
    let mut bi = 0;
    for (i, &p) in pal.iter().enumerate() {
        let sq = |x: u32, y: u32| (x as i64 - y as i64).pow(2);
        let mut d = sq(r(p), r(c));
        if d >= best {
            continue;
        }
        d += sq(g(p), g(c));
        if d >= best {
            continue;
        }
        d += sq(b(p), b(c));
        if d >= best {
            continue;
        }
        d += sq(a(p), a(c));
        if d < best {
            best = d;
            bi = i;
            if d == 0 {
                break;
            }
        }
    }
    bi
}

/// Umsetzen mit RGBA-Palette (`@007471a0`): ohne Serpentine, vier Kanäle; der Alpha-Fehler wirkt
/// ohne Mustermatrix und nur bei Quellalpha ≠ 0/255. Ohne Dither bleiben alle Fehler 0.
#[allow(clippy::too_many_arguments)]
pub fn remap_rgba(pal: &[u32], pixels: &[u32], w: usize, h: usize, dither: bool, percent: i32, websnap_on: bool, tolerance: i32) -> Vec<u8> {
    const PATTERN: [i32; 16] = [6, 0, -6, 0, 0, 0, 0, 0, -6, 0, 6, 0, 0, 0, 0, 0];
    let p = percent.clamp(10, 100);
    let thr = if p == 100 { 2 } else { 5 };
    let tol = (tolerance / 2).max(0) as u32;
    let mut up = [vec![0i32; w + 1], vec![0i32; w + 1], vec![0i32; w + 1], vec![0i32; w + 1]];
    let mut out = vec![0u8; w * h];
    for y in 0..h {
        let mut left = [0i32; 4];
        for x in 0..w {
            let mut c = pixels[y * w + x];
            if websnap_on {
                c = websnap(c, tol);
            }
            let a0 = a(c) as i32;
            let m = PATTERN[(x & 3) + (y & 3) * 4];
            let mut v = [0i32; 3];
            for (k, s) in [r(c), g(c), b(c)].into_iter().enumerate() {
                let mut e = up[k][x] * 0x56 + left[k] * 0xAA;
                e = tdiv(tdiv(e, 256) * p, 100);
                v[k] = tdiv(e * m, 10) + tdiv(e * p, 100) + s as i32;
            }
            let e = up[3][x] * 0x56 + left[3] * 0xAA;
            let mut av = a0;
            if a0 != 0 && a0 != 255 {
                av = a0 + tdiv(tdiv(tdiv(e, 256) * p, 100) * p, 100);
            }
            let cl = [v[0], v[1], v[2], av].map(|t| t.clamp(0, 255) as u32);
            let idx = nearest_rgba(pal, argb(cl[3], cl[0], cl[1], cl[2]));
            out[y * w + x] = idx.min(255) as u8;
            let pc = pal.get(idx).copied().unwrap_or(0);
            if !dither || (a(pc) == 0 && a0 == 0) {
                left = [0; 4];
                for u in up.iter_mut() {
                    u[x] = 0;
                }
                continue;
            }
            let diffs = [v[0] - r(pc) as i32, v[1] - g(pc) as i32, v[2] - b(pc) as i32, av - a(pc) as i32];
            for k in 0..4 {
                let d = diffs[k];
                let mut ne = d + (tdiv(left[k], 2) - left[k]);
                if d.abs() < ne.abs() {
                    ne = d;
                }
                if ne.abs() < thr {
                    ne = 0;
                }
                ne = ne.clamp(-255, 255);
                left[k] = ne;
                up[k][x] = ne;
            }
        }
    }
    out
}

/// Ein Paletteneintrag der Nachbearbeitung (`paletteInfo`, 08 §1.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaletteSlot {
    /// Farbe `0xAARRGGBB`.
    pub color: u32,
    /// Pixelzähler (Popularität).
    pub count: u64,
    /// Gesperrt.
    pub locked: bool,
    /// Bearbeitet mit neuer Farbe.
    pub edited: Option<u32>,
    /// Transparent markiert.
    pub transparent: bool,
    /// Ursprünglicher Index (für Umnummerierung).
    pub origin: usize,
}

impl PaletteSlot {
    /// Eintrag ohne Flags.
    pub fn new(color: u32, count: u64, origin: usize) -> PaletteSlot {
        PaletteSlot { color, count, locked: false, edited: None, transparent: false, origin }
    }
}

/// Exportsortierung (`@007432e0`, stabiler Bubblesort): 1 = Luminanz absteigend, 2 = Pixelzähler
/// absteigend. Bei RGBA-Transparenz bleibt Eintrag 0 stehen.
pub fn sort_palette(entries: &[PaletteSlot], mode: i32, tmode: i32) -> Vec<PaletteSlot> {
    if mode != 1 && mode != 2 {
        return entries.to_vec();
    }
    let split = if tmode == TRANSPARENCY_RGBA { 1.min(entries.len()) } else { 0 };
    let mut head = entries[..split].to_vec();
    let mut rest = entries[split..].to_vec();
    if mode == 2 {
        rest.sort_by_key(|x| std::cmp::Reverse(x.count));
    } else {
        rest.sort_by_key(|x| std::cmp::Reverse(lum(x.color)));
    }
    head.extend(rest);
    head
}

/// Bearbeitete Farben (`@00743520` → `@00743150`): bearbeitete Einträge bekommen die neue Farbe
/// (`| 0xFF000000`); andere Einträge mit genau dieser Farbe geben ihren Zähler ab; danach werden
/// alte Farben in der Palette durch neue ersetzt (erster Treffer gewinnt).
pub fn apply_edited(entries: &mut [PaletteSlot]) {
    let mut map: Vec<(u32, u32)> = Vec::new();
    for i in 0..entries.len() {
        let Some(nc) = entries[i].edited else { continue };
        let old = entries[i].color;
        let nc = nc | 0xFF00_0000;
        entries[i].color = nc;
        map.push((old, nc));
        for j in 0..entries.len() {
            if j != i && entries[j].color == nc {
                let c = entries[j].count;
                entries[i].count += c;
                entries[j].count = 0;
            }
        }
    }
    if map.is_empty() {
        return;
    }
    for e in entries.iter_mut() {
        if let Some(&(_, n)) = map.iter().find(|(o, _)| *o == e.color) {
            e.color = n;
        }
    }
}

/// „Optimiert“: unbenutzte Farben entfernen (`@00746e90`). Einträge ohne Pixel (außer gesperrte)
/// fallen weg, doppelte Farben werden zusammengelegt (gesperrt gewinnt), mindestens 2 Einträge
/// (Index-Alpha 1); danach ggf. auf `requested` gekürzt (gesperrte zählen +0x0FFFFFFF) und sortiert.
/// Rückgabe: (neue Liste, gekürzt ⇒ neu umsetzen nötig).
pub fn remove_unused(entries: &[PaletteSlot], requested: i32, tmode: i32, sort_mode: i32) -> (Vec<PaletteSlot>, bool) {
    let requested = if requested < 2 { 256 } else { requested as usize };
    let start = if tmode == TRANSPARENCY_RGBA { 1 } else { 0 };
    let mut keep: Vec<PaletteSlot> = Vec::new();
    if tmode == TRANSPARENCY_RGBA
        && let Some(e) = entries.first()
    {
        keep.push(e.clone());
    }
    for e in entries.iter().skip(start) {
        if e.count == 0 && !e.locked {
            continue;
        }
        let mut skip = false;
        for k in keep.iter_mut().skip(start) {
            if k.color != e.color {
                continue;
            }
            if !e.locked {
                skip = true;
                break;
            }
            if !k.locked {
                k.locked = true;
                skip = true;
                break;
            }
        }
        if !skip {
            keep.push(e.clone());
        }
    }
    if keep.len() < 2 {
        let min = if tmode == TRANSPARENCY_INDEX_ALPHA { 1 } else { 2 };
        while keep.len() < min {
            keep.push(PaletteSlot::new(0xFFFF_FFFF, 0, usize::MAX));
        }
    }
    let nlocked = keep.iter().filter(|e| e.locked).count();
    let requested = requested.max(nlocked);
    if requested < keep.len() {
        let head: Vec<PaletteSlot> = keep[..start.min(keep.len())].to_vec();
        let mut rest: Vec<PaletteSlot> = keep[start.min(keep.len())..].to_vec();
        rest.sort_by_key(|e| std::cmp::Reverse(e.count + if e.locked { 0x0FFF_FFFF } else { 0 }));
        let mut all = head;
        all.extend(rest);
        all.truncate(requested);
        return (sort_palette(&all, sort_mode, tmode), true);
    }
    (sort_palette(&keep, sort_mode, tmode), false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpha_levels() {
        let v: Vec<u32> = (0..15).map(|j| alpha_value(j, 15)).collect();
        assert_eq!(v[0], 17);
        assert_eq!(v[14], 255);
        assert_eq!(alpha_cap(15), 200);
    }

    #[test]
    fn rgba_palette_basic() {
        let px: Vec<u32> = (0..256).map(|i| argb(i as u32, 255, 0, 0)).collect();
        let (_q, pal) = quantize_rgba(&px, 256, 15, false, 14, &[], 16);
        assert_eq!(pal[0], 0x00FF_FFFF);
        assert!(pal.windows(2).all(|w| a(w[0]) <= a(w[1])));
        let idx = remap_rgba(&pal, &px, 16, 16, false, 100, false, 14);
        assert_eq!(idx[0], 0);
        assert_eq!(a(pal[idx[255] as usize]), 255);
    }

    #[test]
    fn unused_and_sort() {
        let e = vec![PaletteSlot::new(0xFF000000, 5, 0), PaletteSlot::new(0xFFFFFFFF, 0, 1), PaletteSlot::new(0xFF808080, 9, 2)];
        let (k, cut) = remove_unused(&e, 256, 0, 2);
        assert!(!cut);
        assert_eq!(k.iter().map(|x| x.color).collect::<Vec<_>>(), vec![0xFF808080, 0xFF000000]);
        let s = sort_palette(&e, 1, 0);
        assert_eq!(s[0].color, 0xFFFFFFFF);
        let mut ed = vec![PaletteSlot { edited: Some(0x123456), ..PaletteSlot::new(0xFF000000, 1, 0) }, PaletteSlot::new(0xFF123456, 4, 1)];
        apply_edited(&mut ed);
        assert_eq!(ed[0].color, 0xFF123456);
        assert_eq!(ed[0].count, 5);
        assert_eq!(ed[1].count, 0);
    }
}

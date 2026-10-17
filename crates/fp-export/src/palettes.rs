//! Feste Paletten (08 §2.2): Einheitlich/Web 216, Graustufen/Schwarzweiß, Macintosh, Windows, Exakt.
//!
//! Alle Farben als `0xAARRGGBB` (deckend). Die Mac-Systempalette ist eine Formel (6×6×6-Würfel ohne
//! Schwarz, dann je zehn Rot-/Grün-/Blau-/Grau-Stufen und Schwarz); die Windows-Palette ist eine feste
//! Tabelle (Halbtonpalette in Original-Reihenfolge, `tables_data.rs`).

use crate::px::{a, argb, lum, matte_blend};
use crate::tables_data::WINDOWS_PALETTE;

/// Einheitliche Palette (Raster, `@00746cf0`): `L³` Farben, `L` auf 2…6 begrenzt, Kanalwert
/// `k·255 div (L−1)`, Reihenfolge R absteigend (außen), G, B absteigend (innen) ⇒ Index 0 = Weiß.
pub fn palette_grid(levels: i32) -> Vec<u32> {
    let l = levels.clamp(2, 6) as u32;
    let n = l - 1;
    let mut pal = Vec::with_capacity((l * l * l) as usize);
    for rr in (0..=n).rev() {
        for gg in (0..=n).rev() {
            for bb in (0..=n).rev() {
                pal.push(argb(255, rr * 255 / n, gg * 255 / n, bb * 255 / n));
            }
        }
    }
    pal
}

/// Web 216 = Raster mit 6 Stufen.
pub fn palette_web216() -> Vec<u32> {
    palette_grid(6)
}

/// Graustufen (`@00746be0`): `n` Stufen (2…256), Weiß zuerst; `n = 2` ist Schwarzweiß.
/// Stufe `i = ((n−1) div 2 + (n−1−i)·255) div (n−1)`.
pub fn palette_gray(n: i32) -> Vec<u32> {
    let n = n.clamp(2, 256) as u32;
    let m = n - 1;
    (0..n)
        .map(|i| {
            let v = (m / 2 + (m - i) * 255) / m;
            argb(255, v, v, v)
        })
        .collect()
}

/// Klassische Macintosh-Systempalette (Weiß zuerst, Schwarz zuletzt).
pub fn palette_mac() -> Vec<u32> {
    let mut pal = Vec::with_capacity(256);
    for rr in (0..6u32).rev() {
        for gg in (0..6u32).rev() {
            for bb in (0..6u32).rev() {
                if rr + gg + bb != 0 {
                    pal.push(argb(255, rr * 51, gg * 51, bb * 51));
                }
            }
        }
    }
    const RAMP: [u32; 10] = [0xEE, 0xDD, 0xBB, 0xAA, 0x88, 0x77, 0x55, 0x44, 0x22, 0x11];
    pal.extend(RAMP.iter().map(|&v| argb(255, v, 0, 0)));
    pal.extend(RAMP.iter().map(|&v| argb(255, 0, v, 0)));
    pal.extend(RAMP.iter().map(|&v| argb(255, 0, 0, v)));
    pal.extend(RAMP.iter().map(|&v| argb(255, v, v, v)));
    pal.push(0xFF00_0000);
    pal
}

/// Windows-Systempalette (Schwarz zuerst, `808080` doppelt).
pub fn palette_windows() -> Vec<u32> {
    WINDOWS_PALETTE.iter().map(|&c| 0xFF00_0000 | c).collect()
}

/// Exakte Palette (`@00746720`): alle verschiedenen Farben (nach Matte-Mischung, wenn die Matte
/// Alpha ≠ 0 hat). Mehr als 256 ⇒ `None` (das Original fällt dann auf Adaptiv 256 zurück).
/// Sortierung absteigend nach Luminanz; Gleichstände nach ARGB (`qsort` des Originals ist instabil, A).
pub fn palette_exact(pixels: &[u32], matte: u32) -> Option<Vec<u32>> {
    let use_matte = a(matte) != 0;
    let mut set = std::collections::BTreeSet::new();
    for &c in pixels {
        let c = if use_matte && a(c) != 255 {
            matte_blend(c, matte, false)
        } else {
            c
        };
        set.insert(c);
        if set.len() > 256 {
            return None;
        }
    }
    let mut v: Vec<u32> = set.into_iter().collect();
    v.sort_by(|&x, &y| lum(y).cmp(&lum(x)).then(x.cmp(&y)));
    Some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_palettes() {
        let w = palette_web216();
        assert_eq!(w.len(), 216);
        assert_eq!(w[0], 0xFFFFFFFF);
        assert_eq!(w[215], 0xFF000000);
        assert_eq!(palette_grid(9).len(), 216);
        assert_eq!(
            palette_grid(2),
            vec![
                0xFFFFFFFF, 0xFFFFFF00, 0xFFFF00FF, 0xFFFF0000, 0xFF00FFFF, 0xFF00FF00, 0xFF0000FF,
                0xFF000000
            ]
        );
        assert_eq!(palette_gray(2), vec![0xFFFFFFFF, 0xFF000000]);
        let g = palette_gray(3);
        assert_eq!(g[1], 0xFF808080);
        assert_eq!(palette_mac().len(), 256);
        assert_eq!(palette_mac()[0], 0xFFFFFFFF);
        assert_eq!(palette_windows().len(), 256);
        assert_eq!(palette_windows()[0], 0xFF000000);
        let ex = palette_exact(&[0xFF000000, 0xFFFFFFFF, 0xFF000000], 0).unwrap();
        assert_eq!(ex, vec![0xFFFFFFFF, 0xFF000000]);
    }
}

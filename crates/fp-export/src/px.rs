//! Pixel-Hilfen der Export-Pipeline: Farben als `u32` im Format `0xAARRGGBB` (wie das Original intern und
//! wie die Python-Referenz `fw_palette.py`), Luminanz, Abstand, Web-Snap und Matte-Mischung (08 §2.1).

/// Alpha-Kanal von `0xAARRGGBB`.
#[inline]
pub fn a(c: u32) -> u32 {
    c >> 24
}
/// Rot-Kanal.
#[inline]
pub fn r(c: u32) -> u32 {
    (c >> 16) & 255
}
/// Grün-Kanal.
#[inline]
pub fn g(c: u32) -> u32 {
    (c >> 8) & 255
}
/// Blau-Kanal.
#[inline]
pub fn b(c: u32) -> u32 {
    c & 255
}

/// Setzt `0xAARRGGBB` aus vier Kanälen zusammen (je Kanal auf 8 Bit gekürzt).
#[inline]
pub fn argb(a: u32, r: u32, g: u32, b: u32) -> u32 {
    ((a & 255) << 24) | ((r & 255) << 16) | ((g & 255) << 8) | (b & 255)
}

/// RGBA-Bytes → `0xAARRGGBB`.
#[inline]
pub fn from_rgba(p: [u8; 4]) -> u32 {
    argb(p[3] as u32, p[0] as u32, p[1] as u32, p[2] as u32)
}

/// `0xAARRGGBB` → RGBA-Bytes.
#[inline]
pub fn to_rgba(c: u32) -> [u8; 4] {
    [r(c) as u8, g(c) as u8, b(c) as u8, a(c) as u8]
}

/// Luminanz wie das Original (`@00743110`): `(R·109 + G·366 + B·37) >> 9`.
#[inline]
pub fn lum(c: u32) -> u32 {
    (r(c) * 109 + g(c) * 366 + b(c) * 37) >> 9
}

/// Quadratischer RGB-Abstand.
#[inline]
pub fn dist2(c1: u32, c2: u32) -> u32 {
    let d = |x: u32, y: u32| {
        let v = x as i32 - y as i32;
        (v * v) as u32
    };
    d(r(c1), r(c2)) + d(g(c1), g(c2)) + d(b(c1), b(c2))
}

/// L1-Abstand über R, G, B (LZW-Glättung).
#[inline]
pub fn dist_l1(c1: u32, c2: u32) -> u32 {
    r(c1).abs_diff(r(c2)) + g(c1).abs_diff(g(c2)) + b(c1).abs_diff(b(c2))
}

/// Nächstes Vielfaches von 51 (Grenzen 25/26, 76/77 …; Tabelle `@007415f0`).
#[inline]
pub fn web_channel(v: u32) -> u32 {
    ((v + 25) / 51) * 51
}

/// Ist die Farbe eine Web-216-Farbe (alle Kanäle Vielfache von 51)?
#[inline]
pub fn is_web(c: u32) -> bool {
    r(c).is_multiple_of(51) && g(c).is_multiple_of(51) && b(c).is_multiple_of(51)
}

/// Web-Snap (`@007415f0`): ersetzt `c` durch die nächste Webfarbe, wenn **alle drei** Kanäle
/// `|web(k) − k| < tol` erfüllen. Alpha bleibt.
#[inline]
pub fn websnap(c: u32, tol: u32) -> u32 {
    let (rr, gg, bb) = (r(c), g(c), b(c));
    let (wr, wg, wb) = (web_channel(rr), web_channel(gg), web_channel(bb));
    if wr.abs_diff(rr) < tol && wg.abs_diff(gg) < tol && wb.abs_diff(bb) < tol {
        argb(a(c), wr, wg, wb)
    } else {
        c
    }
}

/// Matte-Mischung eines nicht deckenden Pixels (08 §2.1): je Kanal `k = (p − m)·a + 128 + m·255`,
/// Ergebnis `(k + (k >> 8)) >> 8`. Alpha danach 255 bzw. erhalten (`keep_alpha`).
#[inline]
pub fn matte_blend(c: u32, matte: u32, keep_alpha: bool) -> u32 {
    let al = a(c) as i32;
    if al == 255 {
        return c;
    }
    let ch = |p: u32, m: u32| -> u32 {
        let k = (p as i32 - m as i32) * al + 0x80 + m as i32 * 255;
        (((k + (k >> 8)) >> 8) & 255) as u32
    };
    argb(if keep_alpha { al as u32 } else { 255 }, ch(r(c), r(matte)), ch(g(c), g(matte)), ch(b(c), b(matte)))
}

/// Mischung über Weiß wie der JPEG-Schreiber (08 §4.1): `x = (c − 255)·a + 0xFE81`, `((x >> 8) + x) >> 8`.
#[inline]
pub fn over_white(c: u32) -> u32 {
    let al = a(c) as i32;
    if al == 255 {
        return c;
    }
    let ch = |p: u32| -> u32 {
        let x = (p as i32 - 255) * al + 0xFE81;
        ((((x >> 8) + x) >> 8) & 255) as u32
    };
    argb(255, ch(r(c)), ch(g(c)), ch(b(c)))
}

/// C-Division (Richtung 0), wie das Original sie in den Dither-Schleifen benutzt.
#[inline]
pub fn tdiv(a: i32, b: i32) -> i32 {
    a / b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basics() {
        assert_eq!(lum(0xFFFFFFFF), 255);
        assert_eq!(websnap(0xFF1A3334, 3), 0xFF1A3334);
        assert_eq!(websnap(0xFF323334, 3), 0xFF333333);
        assert_eq!(web_channel(25), 0);
        assert_eq!(web_channel(26), 51);
        assert_eq!(matte_blend(0x80FF0000, 0xFFFFFFFF, false), 0xFFFF7F7F);
        assert_eq!(matte_blend(0x00123456, 0xFFFFFFFF, true), 0x00FFFFFF);
        assert_eq!(over_white(0x00000000), 0xFFFFFFFF);
        assert_eq!(over_white(0xFF102030), 0xFF102030);
        assert_eq!(tdiv(-7, 2), -3);
    }
}

//! Rundungsarten und Ganzzahl-Hilfen des Originals (Spezifikation 14 §1.7, 07 §7.2, 05 §2.5).
//!
//! - [`round_away`] = `@0040e780`: für x ≥ 0 `floor(x+0,5)`, sonst `ceil(x−0,5)`.
//! - [`round_even`] = x87-`fistp` im Standardmodus (halbe Werte zur geraden Zahl).
//! - [`trunc`] = `_ftol2` (Richtung null).
//! - [`cdiv`] = Ganzzahldivision wie in C (Richtung null).
//! - [`mul255`] = `((t + (t>>8)) >> 8)` mit `t = a·b + 128`, exakt `round(a·b/255)`.
//! - [`DIV`] = Divisionstabelle `DAT_01c65758`: `DIV[a][b] = ⌊255·a/b⌋`, 0 bei `b < a` oder `b = 0`.

/// Halbe Werte weg von null runden.
#[inline]
pub fn round_away(x: f64) -> f64 {
    if x >= 0.0 { (x + 0.5).floor() } else { (x - 0.5).ceil() }
}

/// Halbe Werte zur geraden Zahl runden (x87-`fistp`).
#[inline]
pub fn round_even(x: f64) -> f64 {
    x.round_ties_even()
}

/// Richtung null abschneiden, als `i64` (nicht endlich → 0, sonst gesättigt).
#[inline]
pub fn trunc(x: f64) -> i64 {
    if x.is_finite() { x as i64 } else { 0 }
}

/// C-Ganzzahldivision (Richtung null); Division durch 0 ergibt 0.
#[inline]
pub fn cdiv(a: i64, b: i64) -> i64 {
    if b == 0 { 0 } else { a.wrapping_div(b) }
}

/// `round(a·b/255)` in der Original-Form `t = a·b + 128; (t + (t>>8)) >> 8`.
#[inline]
pub const fn mul255(a: u32, b: u32) -> u32 {
    let t = a * b + 128;
    (t + (t >> 8)) >> 8
}

/// `round(v/255)` in der Original-Form (`div255` aus 05 §2.5) für `v ≥ 0`.
#[inline]
pub const fn div255(v: u32) -> u32 {
    let t = v + 128;
    ((t >> 8) + t) >> 8
}

const fn build_div() -> [[u8; 256]; 256] {
    let mut t = [[0u8; 256]; 256];
    let mut a = 0;
    while a < 256 {
        let mut b = 0;
        while b < 256 {
            if b != 0 && b >= a {
                t[a][b] = (255 * a / b) as u8;
            }
            b += 1;
        }
        a += 1;
    }
    t
}

/// Divisionstabelle `DAT_01c65758` (aufgebaut in `@006fa880`): `DIV[a][b] = ⌊255·a/b⌋` für
/// `b ≥ a` und `b > 0`, sonst 0. Benutzt vom Compositor, der Tinten-Akkumulation der Pinsel und
/// der Farbfüllung.
pub static DIV: [[u8; 256]; 256] = build_div();

/// `float32`-Zwischenspeicherung (wie x87-`fstp dword`).
#[inline]
pub fn f32r(x: f64) -> f64 {
    x as f32 as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mul255_is_exact_rounding() {
        for a in 0..256u32 {
            for b in 0..256u32 {
                let exact = ((a * b) as f64 / 255.0).round() as u32;
                assert_eq!(mul255(a, b), exact, "{a} {b}");
            }
        }
        assert_eq!(div255(255 * 255), 255);
    }

    #[test]
    fn div_table() {
        assert_eq!(DIV[0][0], 0);
        assert_eq!(DIV[128][255], 128);
        assert_eq!(DIV[200][100], 0);
        assert_eq!(DIV[100][200], 127);
        assert_eq!(DIV[255][255], 255);
    }
}

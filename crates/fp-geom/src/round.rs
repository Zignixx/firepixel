//! Rundungsarten des Originals (Spezifikation 14 §1.7, 07 §4).
//!
//! Für Bitgleichheit ist die Rundungsart wichtig. Das Original benutzt nebeneinander:
//!
//! - [`round_away`]: halbe Werte weg von null (`floor(x+0,5)` bzw. `ceil(x−0,5)`); auch die
//!   Schnittpunkt-Rundung des Rasterizers (07 §4) und `f64::round` in Rust.
//! - [`round_even`]: x87-`fistp` im Standardmodus, halbe Werte zur geraden Zahl.
//! - [`trunc_i32`]: `_ftol2`, Richtung null.
//! - [`cdiv`]: Ganzzahldivision wie in C (Richtung null); Rusts `/` auf Ganzzahlen tut das bereits.

/// Rundet halbe Werte weg von null: für `x ≥ 0` `floor(x + 0,5)`, sonst `ceil(x − 0,5)`.
#[inline]
pub fn round_away(x: f64) -> f64 {
    if x >= 0.0 {
        (x + 0.5).floor()
    } else {
        (x - 0.5).ceil()
    }
}

/// Rundet halbe Werte zur geraden Zahl (x87-`fistp` im Standardmodus).
#[inline]
pub fn round_even(x: f64) -> f64 {
    x.round_ties_even()
}

/// Schneidet Richtung null ab (`_ftol2`). Nicht endliche Werte ergeben 0.
#[inline]
pub fn trunc_i32(x: f64) -> i32 {
    if x.is_finite() { x as i32 } else { 0 }
}

/// [`round_away`] als `i32` (gesättigt, NaN → 0).
#[inline]
pub fn round_away_i32(x: f64) -> i32 {
    trunc_i32(round_away(x))
}

/// [`round_even`] als `i32` (gesättigt, NaN → 0).
#[inline]
pub fn round_even_i32(x: f64) -> i32 {
    trunc_i32(round_even(x))
}

/// Ganzzahldivision wie in C (Richtung null). Division durch 0 ergibt 0 statt einer Panik.
#[inline]
pub fn cdiv(a: i64, b: i64) -> i64 {
    if b == 0 { 0 } else { a.wrapping_div(b) }
}

/// Rundet einen Wert über `f32` (Zwischenspeicherung als `float`, wie das Original sie oft vornimmt).
#[inline]
pub fn f32r(x: f64) -> f64 {
    x as f32 as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounding_modes() {
        assert_eq!(round_away(2.5), 3.0);
        assert_eq!(round_away(-2.5), -3.0);
        assert_eq!(round_away(-2.4), -2.0);
        assert_eq!(round_even(2.5), 2.0);
        assert_eq!(round_even(3.5), 4.0);
        assert_eq!(trunc_i32(-2.9), -2);
        assert_eq!(trunc_i32(f64::NAN), 0);
        assert_eq!(cdiv(-7, 2), -3);
        assert_eq!(cdiv(7, -2), -3);
        assert_eq!(cdiv(7, 0), 0);
    }
}

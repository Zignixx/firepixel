//! Nachbildung der x87-Rechnung mit 64-Bit-Mantisse an den Stellen, an denen die Abschrägung
//! abschneidet (05 §11, Laufzeit T2): `trunc(a·b·c)` mit Zwischenrundung auf 64 Bit und
//! `cos(t·π[·3])` mit erweitert genauem Produkt.
//!
//! Gerechnet wird exakt mit Ganzzahlen (`u128`) bzw. in doppelt-doppelter Genauigkeit (≈ 106 Bit),
//! danach wie die FPU auf 64 Bit (nächste, bei Gleichstand gerade) und zum Schluss auf `f64` gerundet.

/// Eine Zahl `mant · 2^exp` (Mantisse ohne Vorzeichen, Wert ≥ 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Exact {
    mant: u128,
    exp: i32,
}

/// Zerlegt ein endliches, nicht negatives `f64` exakt.
fn decompose(v: f64) -> Exact {
    let v = v.abs();
    if v == 0.0 || !v.is_finite() {
        return Exact { mant: 0, exp: 0 };
    }
    let bits = v.to_bits();
    let e = ((bits >> 52) & 0x7ff) as i32;
    let f = bits & ((1u64 << 52) - 1);
    if e == 0 {
        Exact {
            mant: f as u128,
            exp: -1074,
        }
    } else {
        Exact {
            mant: (f | (1u64 << 52)) as u128,
            exp: e - 1075,
        }
    }
}

/// Auf `bits` signifikante Bits runden (nächste, bei Gleichstand gerade).
fn round_bits(x: Exact, bits: u32) -> Exact {
    if x.mant == 0 {
        return x;
    }
    let len = 128 - x.mant.leading_zeros();
    if len <= bits {
        return x;
    }
    let sh = len - bits;
    let mut q = x.mant >> sh;
    let rem = x.mant & ((1u128 << sh) - 1);
    let half = 1u128 << (sh - 1);
    if rem > half || (rem == half && q & 1 == 1) {
        q += 1;
    }
    Exact {
        mant: q,
        exp: x.exp + sh as i32,
    }
}

fn mul_exact(a: Exact, b: Exact) -> Exact {
    // Beide Operanden haben höchstens 64 signifikante Bits → Produkt passt in u128.
    let a = round_bits(a, 64);
    let b = round_bits(b, 64);
    Exact {
        mant: a.mant * b.mant,
        exp: a.exp + b.exp,
    }
}

/// Abschneiden zur Null (Wert ≥ 0).
fn trunc_exact(x: Exact) -> i64 {
    if x.mant == 0 {
        return 0;
    }
    if x.exp >= 0 {
        if x.exp >= 40 {
            return i64::MAX;
        }
        let v = x.mant.checked_shl(x.exp as u32).unwrap_or(u128::MAX);
        v.min(i64::MAX as u128) as i64
    } else {
        let s = (-x.exp) as u32;
        if s >= 128 {
            0
        } else {
            (x.mant >> s).min(i64::MAX as u128) as i64
        }
    }
}

fn to_f64(x: Exact) -> f64 {
    if x.mant == 0 {
        return 0.0;
    }
    let m = x.mant as f64; // rundet korrekt (nächste, gerade)
    m * pow2(x.exp)
}

fn pow2(e: i32) -> f64 {
    // 2^e ohne Überlauf in Zwischenschritten.
    let mut r = 1.0f64;
    let mut e = e;
    while e > 1000 {
        r *= 2f64.powi(1000);
        e -= 1000;
    }
    while e < -1000 {
        r *= 2f64.powi(-1000);
        e += 1000;
    }
    r * 2f64.powi(e)
}

/// `trunc(a·b·c)` wie auf der x87 mit 64-Bit-Mantisse (`bevel.x87_mul_trunc`, `a` ganzzahlig,
/// `b`, `c` ≥ 0 als `double`).
pub fn mul_trunc(a: u32, b: f64, c: f64) -> i64 {
    let ea = Exact {
        mant: a as u128,
        exp: 0,
    };
    let p = round_bits(mul_exact(ea, decompose(b)), 64);
    let p = round_bits(mul_exact(p, decompose(c)), 64);
    trunc_exact(p)
}

// ---------------------------------------------------------------- doppelt-doppelte Genauigkeit

#[derive(Debug, Clone, Copy)]
struct Dd {
    hi: f64,
    lo: f64,
}

fn two_sum(a: f64, b: f64) -> Dd {
    let s = a + b;
    let bb = s - a;
    let err = (a - (s - bb)) + (b - bb);
    Dd { hi: s, lo: err }
}

fn quick_two_sum(a: f64, b: f64) -> Dd {
    let s = a + b;
    Dd {
        hi: s,
        lo: b - (s - a),
    }
}

fn two_prod(a: f64, b: f64) -> Dd {
    let p = a * b;
    Dd {
        hi: p,
        lo: a.mul_add(b, -p),
    }
}

impl Dd {
    fn from(x: f64) -> Dd {
        Dd { hi: x, lo: 0.0 }
    }
    fn add(self, o: Dd) -> Dd {
        let s = two_sum(self.hi, o.hi);
        let t = two_sum(self.lo, o.lo);
        let s2 = quick_two_sum(s.hi, s.lo + t.hi);
        quick_two_sum(s2.hi, s2.lo + t.lo)
    }
    fn neg(self) -> Dd {
        Dd {
            hi: -self.hi,
            lo: -self.lo,
        }
    }
    fn mul(self, o: Dd) -> Dd {
        let p = two_prod(self.hi, o.hi);
        let lo = p.lo + (self.hi * o.lo + self.lo * o.hi);
        quick_two_sum(p.hi, lo)
    }
    fn div_f(self, d: f64) -> Dd {
        let q1 = self.hi / d;
        let p = two_prod(q1, d);
        let r = self.add(p.neg());
        let q2 = r.hi / d;
        let r2 = r.add(two_prod(q2, d).neg());
        let q3 = r2.hi / d;
        quick_two_sum(q1, q2).add(Dd::from(q3))
    }
}

/// Exakter Wert → doppelt-doppelt (exakt, sofern höchstens 106 Bit).
fn exact_to_dd(x: Exact) -> Dd {
    let x = round_bits(x, 106);
    let hi_m = round_bits(x, 53);
    let hi = to_f64(hi_m);
    // Rest exakt: x − hi (kann negativ sein).
    let shift_hi = hi_m.exp - x.exp; // ≥ 0
    let hi_scaled = if shift_hi >= 0 {
        (hi_m.mant as i128) << shift_hi.min(100)
    } else {
        hi_m.mant as i128
    };
    let rest = x.mant as i128 - hi_scaled;
    let lo = (rest as f64) * pow2(x.exp);
    Dd { hi, lo }
}

/// Doppelt-doppelt → auf 64 Bit runden → `f64` (zweite Rundung wie beim Speichern als `double`).
fn dd_round64_to_f64(v: Dd) -> f64 {
    let neg = v.hi < 0.0 || (v.hi == 0.0 && v.lo < 0.0);
    let (hi, lo) = if neg { (-v.hi, -v.lo) } else { (v.hi, v.lo) };
    if hi == 0.0 {
        return 0.0;
    }
    // Exakte Summe als ganzzahlige Mantisse auf gemeinsamem Exponenten.
    let eh = decompose(hi);
    let el = decompose(lo.abs());
    let base = if el.mant == 0 {
        eh.exp
    } else {
        eh.exp.min(el.exp)
    };
    let shift_h = (eh.exp - base) as u32;
    let shift_l = (el.exp - base).max(0) as u32;
    let x = if shift_h < 70 && (el.mant == 0 || shift_l < 70) {
        let h = (eh.mant as i128) << shift_h;
        let l = if el.mant == 0 {
            0
        } else {
            (el.mant as i128) << shift_l
        };
        let s = if lo < 0.0 { h - l } else { h + l };
        Exact {
            mant: s.max(0) as u128,
            exp: base,
        }
    } else {
        // Rest liegt weit unter der 64-Bit-Grenze: nur hi zählt.
        eh
    };
    let r = to_f64(round_bits(x, 64));
    if neg { -r } else { r }
}

/// `cos(t·π)` (`k = 1`) bzw. `cos(t·π·3)` (`k = 3`) wie `bevel._cos_x87`: Produkt mit dem
/// `double`-π auf 64 Bit gerundet, ggf. ·3 erneut auf 64 Bit, Kosinus genau, Ergebnis auf 64 Bit und
/// dann auf `double` gerundet.
pub fn cos_pi(t: f64, k: u32) -> f64 {
    if !t.is_finite() {
        return f64::NAN;
    }
    let neg_t = t < 0.0;
    let mut a = round_bits(mul_exact(decompose(t), decompose(std::f64::consts::PI)), 64);
    if k != 1 {
        a = round_bits(
            mul_exact(
                a,
                Exact {
                    mant: k as u128,
                    exp: 0,
                },
            ),
            64,
        );
    }
    let x = exact_to_dd(a);
    let x = if neg_t { x.neg() } else { x };
    // Taylor-Reihe in doppelt-doppelter Genauigkeit.
    let x2 = x.mul(x);
    let mut term = Dd::from(1.0);
    let mut sum = Dd::from(1.0);
    for n in 1..200u32 {
        let d = ((2 * n - 1) * (2 * n)) as f64;
        term = term.mul(x2).div_f(d).neg();
        if term.hi.abs() < 1e-36 {
            break;
        }
        sum = sum.add(term);
    }
    dd_round64_to_f64(sum)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mul_trunc_basics() {
        assert_eq!(mul_trunc(255, 1.0, 0.75), 191);
        assert_eq!(mul_trunc(100, 0.5, 1.0), 50);
        assert_eq!(mul_trunc(0, 0.3, 0.75), 0);
        // 0,1·3 ist in double knapp unter 0,3; x87 mit 64 Bit rundet anders als double.
        let v = mul_trunc(10, 0.1, 3.0);
        assert!(v == 2 || v == 3);
    }

    #[test]
    fn cos_values() {
        assert_eq!(cos_pi(0.0, 1), 1.0);
        assert!((cos_pi(1.0, 1) + 1.0).abs() < 1e-15);
        assert!((cos_pi(0.25, 1) - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-15);
        assert!((cos_pi(0.5, 3) - (1.5 * std::f64::consts::PI).cos()).abs() < 1e-15);
        assert!(cos_pi(0.5, 1) > 0.0);
    }
}

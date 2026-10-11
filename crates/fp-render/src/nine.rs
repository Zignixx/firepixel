//! Punktabbildungen beim Zeichnen: affine Matrizen (Instanzen `MTXv`, Text `MTXv`) und die
//! 9-Slice-Abbildung von Symbolinstanzen (13 §3.3, §7.6 Regel „fw“, Laufzeit T2/T3).
//!
//! Eine [`PointMap`] ist eine Folge von Stufen (innen zuerst). 9-Slice wirkt je Achse:
//! Ränder 1:1, Mitte gestreckt; Hilfslinien vorher gerundet, Rand je Hilfslinie einzeln auf 0, wenn
//! sie außerhalb des Bezugsrechtecks liegt; Rückfall auf normale Skalierung, wenn sich die Ränder
//! überschneiden oder das Ziel schmaler als die Ränder ist.

use fp_geom::{Matrix3, Rect, Vec2};
use fp_model::NineSlice;

/// Abbildung einer Achse.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisMap {
    /// Bezugsbereich `[lo, hi]` (Symbolkoordinaten).
    pub lo: f64,
    pub hi: f64,
    /// Ränder (≥ 0).
    pub m1: f64,
    pub m2: f64,
    /// Zielbereich `[lo', hi']`.
    pub lo2: f64,
    pub hi2: f64,
    /// 9-Slice aktiv (sonst normale Skalierung).
    pub nine: bool,
}

impl AxisMap {
    /// Achse aus Bezugsbereich, gerundeten Hilfslinien und Ziel.
    pub fn new(lo: f64, hi: f64, g1: f64, g2: f64, lo2: f64, hi2: f64) -> AxisMap {
        let m1 = if lo < g1 && g1 < hi { g1 - lo } else { 0.0 };
        let m2 = if lo < g2 && g2 < hi { hi - g2 } else { 0.0 };
        let (gg1, gg2) = (lo + m1, hi - m2);
        let nine = !(gg1 >= gg2 || (hi2 - lo2) < m1 + m2);
        AxisMap { lo, hi, m1, m2, lo2, hi2, nine }
    }

    /// Bildet eine Koordinate ab.
    pub fn apply(&self, x: f64) -> f64 {
        let (g1, g2) = (self.lo + self.m1, self.hi - self.m2);
        if !self.nine {
            if self.hi == self.lo {
                return self.lo2 + (x - self.lo);
            }
            return self.lo2 + (x - self.lo) * (self.hi2 - self.lo2) / (self.hi - self.lo);
        }
        if x <= g1 {
            self.lo2 + (x - self.lo)
        } else if x >= g2 {
            self.hi2 - (self.hi - x)
        } else {
            (self.lo2 + self.m1) + (x - g1) * ((self.hi2 - self.m2) - (self.lo2 + self.m1)) / (g2 - g1)
        }
    }
}

/// Eine Stufe einer Punktabbildung.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Stage {
    /// Affine (bzw. projektive) Matrix in Zeilenvektor-Konvention.
    Affine(Matrix3),
    /// 9-Slice je Achse (Symbolkoordinaten → Elternkoordinaten).
    Nine { x: AxisMap, y: AxisMap },
}

/// Abbildung Elementkoordinaten → Dokumentkoordinaten.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PointMap {
    /// Stufen, innen zuerst.
    pub stages: Vec<Stage>,
}

impl PointMap {
    /// Einheitsabbildung.
    pub fn identity() -> PointMap {
        PointMap { stages: Vec::new() }
    }

    /// Erst `self`, dann `m` (äußere Stufe anhängen).
    pub fn then_affine(&self, m: &Matrix3) -> PointMap {
        let mut s = self.stages.clone();
        if m.is_identity() {
            return PointMap { stages: s };
        }
        if let Some(Stage::Affine(last)) = s.last_mut() {
            *last = last.then(m);
        } else {
            s.push(Stage::Affine(*m));
        }
        PointMap { stages: s }
    }

    /// Innere Stufe davorsetzen (`inner`, dann `self`).
    pub fn prepend(&self, inner: Stage) -> PointMap {
        let mut s = Vec::with_capacity(self.stages.len() + 1);
        s.push(inner);
        s.extend(self.stages.iter().copied());
        // Benachbarte affine Stufen zusammenfassen.
        let mut out: Vec<Stage> = Vec::with_capacity(s.len());
        for st in s {
            match (out.last_mut(), st) {
                (Some(Stage::Affine(a)), Stage::Affine(b)) => *a = a.then(&b),
                _ => out.push(st),
            }
        }
        out.retain(|st| !matches!(st, Stage::Affine(m) if m.is_identity()));
        PointMap { stages: out }
    }

    /// Punkt abbilden.
    pub fn apply(&self, p: Vec2) -> Vec2 {
        let mut v = p;
        for s in &self.stages {
            v = match s {
                Stage::Affine(m) => m.apply_vec2(v),
                Stage::Nine { x, y } => Vec2::new(x.apply(v.x), y.apply(v.y)),
            };
        }
        v
    }

    /// Keine Stufe?
    pub fn is_identity(&self) -> bool {
        self.stages.is_empty()
    }

    /// Enthält eine 9-Slice-Stufe?
    pub fn is_nine_slice(&self) -> bool {
        self.stages.iter().any(|s| matches!(s, Stage::Nine { .. }))
    }

    /// Gesamtmatrix, wenn alle Stufen affin sind.
    pub fn as_affine(&self) -> Option<Matrix3> {
        let mut m = Matrix3::IDENTITY;
        for s in &self.stages {
            match s {
                Stage::Affine(a) => m = m.then(a),
                Stage::Nine { .. } => return None,
            }
        }
        Some(m)
    }

    /// Ganzzahlige Verschiebung (dx, dy), wenn die Abbildung genau eine solche ist.
    pub fn integer_translation(&self) -> Option<(i32, i32)> {
        let m = self.as_affine()?;
        if !m.is_translation(1e-6) || !m.is_affine() {
            return None;
        }
        let (tx, ty) = (m.m[2][0] as f64, m.m[2][1] as f64);
        if tx.fract() != 0.0 || ty.fract() != 0.0 || tx.abs() > 1.0e9 || ty.abs() > 1.0e9 {
            return None;
        }
        Some((tx as i32, ty as i32))
    }

    /// Text unter 9-Slice wird nur verschoben (13 §7.8): jede 9-Slice-Stufe wird durch die
    /// Verschiebung ersetzt, die sie dem Anker `anchor` (Elementkoordinaten) gibt.
    pub fn translate_only(&self, anchor: Vec2) -> PointMap {
        let mut out = PointMap::identity();
        let mut a = anchor;
        for s in &self.stages {
            match s {
                Stage::Affine(m) => {
                    out = out.then_affine(m);
                    a = m.apply_vec2(a);
                }
                Stage::Nine { x, y } => {
                    let b = Vec2::new(x.apply(a.x), y.apply(a.y));
                    out = out.then_affine(&Matrix3::translation((b.x - a.x) as f32, (b.y - a.y) as f32));
                    a = b;
                }
            }
        }
        out
    }
}

/// 9-Slice-Stufe für eine Instanzmatrix `m` und die Hilfslinien des Symbols. `None`, wenn die
/// Matrix dreht/schert/spiegelt oder Daten fehlen (dann gilt die normale affine Abbildung).
pub fn nine_slice_stage(ns: &NineSlice, m: &Matrix3) -> Option<Stage> {
    let g = ns.rounded_guides()?;
    let b: Rect = ns.bounds?;
    let mm = &m.m;
    if !m.is_affine() || mm[0][1] != 0.0 || mm[1][0] != 0.0 || mm[0][0] <= 0.0 || mm[1][1] <= 0.0 {
        return None;
    }
    let (sx, sy, tx, ty) = (mm[0][0] as f64, mm[1][1] as f64, mm[2][0] as f64, mm[2][1] as f64);
    if sx == 1.0 && sy == 1.0 {
        return None;
    }
    let (l, t, r, bo) = (b.left as f64, b.top as f64, b.right as f64, b.bottom as f64);
    let x = AxisMap::new(l, r, g[2], g[3], sx * l + tx, sx * r + tx);
    let y = AxisMap::new(t, bo, g[0], g[1], sy * t + ty, sy * bo + ty);
    Some(Stage::Nine { x, y })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axis_rule() {
        // Bezug −50…50, Hilfslinien ±25, Ziel doppelt so breit (−100…100).
        let a = AxisMap::new(-50.0, 50.0, -25.0, 25.0, -100.0, 100.0);
        assert!(a.nine);
        assert_eq!(a.apply(-50.0), -100.0);
        assert_eq!(a.apply(-25.0), -75.0);
        assert_eq!(a.apply(0.0), 0.0);
        assert_eq!(a.apply(50.0), 100.0);
        // Ziel schmaler als die Ränder → normale Skalierung.
        let b = AxisMap::new(-50.0, 50.0, -25.0, 25.0, -20.0, 20.0);
        assert!(!b.nine);
        assert_eq!(b.apply(50.0), 20.0);
    }

    #[test]
    fn translation_detection() {
        let m = PointMap::identity().then_affine(&Matrix3::translation(3.0, -4.0));
        assert_eq!(m.integer_translation(), Some((3, -4)));
        let m2 = m.then_affine(&Matrix3::translation(0.5, 0.0));
        assert_eq!(m2.integer_translation(), None);
        assert_eq!(PointMap::identity().integer_translation(), Some((0, 0)));
    }
}

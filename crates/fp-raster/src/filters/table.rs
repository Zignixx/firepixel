//! Tabellenfilter (`CAdjustTable`, 06 §3.1): Kurven, Tonwertkorrektur, Auto-Tonwert, dazu
//! Umkehren.
//!
//! Vier 256er-Bytetabellen (RGB, Rot, Grün, Blau), angewendet als
//! `R' = rgb[red[R]]`, `G' = rgb[green[G]]`, `B' = rgb[blue[B]]` für **jedes** Pixel; Alpha bleibt.

use fp_color::round::round_away;
use fp_geom::IRect;

use super::{PixelFilter, start};
use crate::image::RgbaImage;
use crate::mask::Mask;
use crate::par::for_each_row;

/// Die vier Tabellen eines `CAdjustTable`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdjustTables {
    pub rgb: [u8; 256],
    pub red: [u8; 256],
    pub green: [u8; 256],
    pub blue: [u8; 256],
}

/// Identitätstabelle.
pub fn identity_table() -> [u8; 256] {
    let mut t = [0u8; 256];
    for (i, v) in t.iter_mut().enumerate() {
        *v = i as u8;
    }
    t
}

impl Default for AdjustTables {
    fn default() -> Self {
        let id = identity_table();
        Self {
            rgb: id,
            red: id,
            green: id,
            blue: id,
        }
    }
}

impl AdjustTables {
    /// Wendet die Tabellen auf `region` an (Kopie von `src` außerhalb).
    pub fn apply_region(&self, src: &RgbaImage, region: IRect) -> RgbaImage {
        let mut out = start(src);
        let b = out.bounds();
        let w = out.width();
        let r = region.intersect(&b);
        if r.is_empty() {
            return out;
        }
        let (x0, x1) = ((r.left - b.left) as usize, (r.right - b.left) as usize);
        let (y0, y1) = ((r.top - b.top) as usize, (r.bottom - b.top) as usize);
        for_each_row(&mut out.data_mut()[y0 * w..y1 * w], w, |_, row| {
            for p in &mut row[x0..x1] {
                p[0] = self.rgb[self.red[p[0] as usize] as usize];
                p[1] = self.rgb[self.green[p[1] as usize] as usize];
                p[2] = self.rgb[self.blue[p[2] as usize] as usize];
            }
        });
        out
    }
}

// ------------------------------------------------------------------ Kurven

/// Kurvenpunkt (`GPTv{XLCf YLCf}`, `f32`).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CurvePoint {
    pub x: f32,
    pub y: f32,
}

/// Parameter von „Kurven…“ (`CCurvesFilter`, 06 §4.3). Leere Liste = Identität.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct CurvesParams {
    pub rgb_points: Vec<CurvePoint>,
    pub red_points: Vec<CurvePoint>,
    pub green_points: Vec<CurvePoint>,
    pub blue_points: Vec<CurvePoint>,
}

impl Default for CurvesParams {
    fn default() -> Self {
        let d = vec![
            CurvePoint { x: 0.0, y: 0.0 },
            CurvePoint { x: 255.0, y: 255.0 },
        ];
        Self {
            rgb_points: d.clone(),
            red_points: d.clone(),
            green_points: d.clone(),
            blue_points: d,
        }
    }
}

#[inline]
fn f32r(x: f64) -> f64 {
    x as f32 as f64
}

/// `@006968f0`: Punkt (`px`, `py`) um `length` entlang der Richtung (`dx`, `dy`) verschieben,
/// Ergebnis `f32`.
fn offset_along(dx: f64, dy: f64, length: f64, px: f64, py: f64) -> (f64, f64) {
    let h = dx.hypot(dy);
    if h == 0.0 {
        return (f32r(0.0 + px), f32r(0.0 + py));
    }
    if dy.abs() < dx.abs() {
        let t = (length * dx) / h;
        (f32r(t + px), f32r((t * dy) / dx + py))
    } else {
        let t = (dy * length) / h;
        (f32r((dx * t) / dy + px), f32r(t + py))
    }
}

/// `@005cf250`: adaptive Unterteilung einer kubischen Bézierkurve (max. Tiefe 6). Liefert die
/// Endpunkte der flachen Teilstücke als `f32` (ohne Startpunkt).
pub fn flatten_cubic(p: [f64; 8]) -> Vec<(f64, f64)> {
    let [
        mut x0,
        mut y0,
        mut x1,
        mut y1,
        mut x2,
        mut y2,
        mut x3,
        mut y3,
    ] = p;
    let mut depth = 0.0f64;
    let mut stack: Vec<[f64; 9]> = Vec::new();
    let mut out = Vec::new();
    loop {
        if depth < 6.0
            && ((x0 - x1) != 0.0 || (y0 - y1) != 0.0 || (x3 - x2) != 0.0 || (y3 - y2) != 0.0)
        {
            let dx = x0 - x3;
            let dy = y0 - y3;
            let (ax, ay) = (dx.abs(), dy.abs());
            let dist = if ay <= ax {
                ax + (ay + 1.0) * 0.5
            } else {
                ax * 0.5 + ay
            };
            let tol = if dist == 0.0 { -1.0 } else { dist * 0.046875 };
            let d1 = (dy * (x0 - x1) - dx * (y0 - y1)).abs();
            let flat = d1 <= tol && (dy * (x0 - x2) - dx * (y0 - y2)).abs() <= tol;
            if !flat {
                let (x01, y01) = ((x1 + x0) * 0.5, (y1 + y0) * 0.5);
                let (x12, y12) = ((x2 + x1) * 0.5, (y2 + y1) * 0.5);
                let (x23, y23) = ((x3 + x2) * 0.5, (y3 + y2) * 0.5);
                let (x012, y012) = ((x12 + x01) * 0.5, (y12 + y01) * 0.5);
                let (x123, y123) = ((x23 + x12) * 0.5, (y23 + y12) * 0.5);
                let (xm, ym) = ((x123 + x012) * 0.5, (y123 + y012) * 0.5);
                depth += 1.0;
                stack.push([xm, ym, x123, y123, x23, y23, x3, y3, depth]);
                (x1, y1, x2, y2, x3, y3) = (x01, y01, x012, y012, xm, ym);
                continue;
            }
        }
        out.push((f32r(x3), f32r(y3)));
        match stack.pop() {
            Some(s) => {
                [x0, y0, x1, y1, x2, y2, x3, y3, depth] = s;
            }
            None => return out,
        }
    }
}

#[inline]
fn byte_of(v: f64) -> u8 {
    // int(v) & 0xFF (Python-Ganzzahl, Abschneiden)
    if v.is_finite() {
        ((v.trunc() as i64) & 0xFF) as u8
    } else {
        0
    }
}

/// `@00b2a670`: Kurvenpunkte (x aufsteigend) → 256er-Tabelle (06 §4.3). Ungültige Punktlisten
/// liefern nur die Vorbelegung bzw. die Identität.
pub fn curve_table(points: &[CurvePoint]) -> [u8; 256] {
    let mut tab = identity_table();
    if points.is_empty() {
        return tab;
    }
    let pts: Vec<(f64, f64)> = points.iter().map(|p| (p.x as f64, p.y as f64)).collect();
    let (fx, fy) = pts[0];
    if !(fx < 255.0) {
        return tab;
    }
    if 0.0 < fx {
        let mut i = 0usize;
        loop {
            tab[i] = byte_of(fy);
            i += 1;
            if !((i as f32 as f64) < fx) || i > 255 {
                break;
            }
        }
    }
    let (lx, ly) = pts[pts.len() - 1];
    if !(0.0 < lx) {
        return tab;
    }
    let start = lx.trunc();
    if start <= 255.0 {
        for v in tab.iter_mut().skip(start.max(0.0) as usize) {
            *v = byte_of(ly);
        }
    }
    // Spline aus Bézier-Segmenten
    let mut arr: Vec<(f64, f64)> = vec![pts[0], pts[0]];
    for k in 1..pts.len().saturating_sub(1) {
        let (prv, cur, nxt) = (pts[k - 1], pts[k], pts[k + 1]);
        if prv.0 < 0.0 || cur.0 < 1.0 || cur.0 > 254.0 || nxt.0 > 255.0 {
            return tab;
        }
        if cur.0 <= prv.0 || nxt.0 <= cur.0 {
            return tab;
        }
        let dx = nxt.0 - prv.0;
        let dy = nxt.1 - prv.1;
        let cin = offset_along(dx, dy, (prv.0 - cur.0) * 0.5, cur.0, cur.1);
        let cout = offset_along(dx, dy, (nxt.0 - cur.0) * 0.5, cur.0, cur.1);
        arr.push(cin);
        arr.push(cur);
        arr.push(cout);
    }
    arr.push(pts[pts.len() - 1]);
    arr.push(pts[pts.len() - 1]);
    let mut s = 0;
    while s + 3 < arr.len() {
        let seg = [
            arr[s].0,
            arr[s].1,
            arr[s + 1].0,
            arr[s + 1].1,
            arr[s + 2].0,
            arr[s + 2].1,
            arr[s + 3].0,
            arr[s + 3].1,
        ];
        let (mut px_prev, mut py_prev) = arr[s];
        for (qx, qy) in flatten_cubic(seg) {
            let qx = f32r(round_away(qx));
            let qy = f32r(round_away(qy));
            if px_prev < qx && qx <= 255.0 {
                let dx = qx - px_prev;
                let den = if 1.0 < dx { dx } else { 1.0 };
                let slope = (qy - py_prev) / den;
                let mut i = px_prev.trunc() as i64;
                while ((i as f32) as f64) < qx {
                    let v = round_away(((i as f32) as f64 - px_prev) * slope + py_prev);
                    if (0..256).contains(&i) {
                        tab[i as usize] = v.clamp(0.0, 255.0) as u8;
                    }
                    i += 1;
                }
                px_prev = qx;
                py_prev = qy;
            }
        }
        s += 3;
    }
    tab
}

impl CurvesParams {
    /// Die vier Tabellen.
    pub fn tables(&self) -> AdjustTables {
        AdjustTables {
            rgb: curve_table(&self.rgb_points),
            red: curve_table(&self.red_points),
            green: curve_table(&self.green_points),
            blue: curve_table(&self.blue_points),
        }
    }
}

impl PixelFilter for CurvesParams {
    fn process(&self, src: &RgbaImage, region: IRect) -> RgbaImage {
        self.tables().apply_region(src, region)
    }
}

/// „Kurven…“ anwenden.
pub fn curves(img: &RgbaImage, p: &CurvesParams, mask: Option<&Mask>) -> RgbaImage {
    p.apply(img, mask)
}

// ------------------------------------------------------------------ Tonwertkorrektur

/// Tonwerte eines Kanals (06 §4.4). Grenzen werden als **Byte** gelesen (`& 0xFF`), Gamma auf
/// 0,1…10 begrenzt.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LevelsChannel {
    pub source_low: i32,
    pub source_high: i32,
    pub dest_low: i32,
    pub dest_high: i32,
    pub gamma: f64,
}

impl Default for LevelsChannel {
    fn default() -> Self {
        Self {
            source_low: 0,
            source_high: 255,
            dest_low: 0,
            dest_high: 255,
            gamma: 1.0,
        }
    }
}

impl LevelsChannel {
    /// Dialogbereich der Grenzen.
    pub const LEVEL_RANGE: (i32, i32) = (0, 255);
    /// Dialog- und Code-Bereich von Gamma.
    pub const GAMMA_RANGE: (f64, f64) = (0.1, 10.0);

    /// Werte auf die Dialogbereiche begrenzt.
    pub fn dialog_clamped(self) -> Self {
        let c = |v: i32| v.clamp(0, 255);
        Self {
            source_low: c(self.source_low),
            source_high: c(self.source_high),
            dest_low: c(self.dest_low),
            dest_high: c(self.dest_high),
            gamma: if self.gamma.is_nan() {
                1.0
            } else {
                self.gamma.clamp(0.1, 10.0)
            },
        }
    }

    /// `@00b290b0`: Tonwerttabelle.
    pub fn table(&self) -> [u8; 256] {
        let lo = (self.source_low & 0xFF) as f64;
        let hi = (self.source_high & 0xFF) as f64;
        let dl = (self.dest_low & 0xFF) as f64;
        let dh = (self.dest_high & 0xFF) as f64;
        let g = self.gamma;
        let g = if !(g >= 0.1) {
            0.1
        } else if g > 10.0 {
            10.0
        } else {
            g
        };
        let mut tab = [0u8; 256];
        for (i, t_out) in tab.iter_mut().enumerate() {
            let mut x = i as f64;
            if x <= lo {
                x = lo;
            }
            if !(x <= hi) {
                x = hi;
            }
            let mut t = ((x - lo) / (hi - lo)).abs();
            if g != 1.0 && !t.is_nan() {
                t = t.powf(1.0 / g);
            }
            let v = t * (dh - dl) + dl;
            // NaN → fistp 0x80000000 → Byte 0
            *t_out = if v.is_nan() || v.abs() > 2147483648.0 {
                0
            } else {
                ((v.trunc() as i64) & 0xFF) as u8
            };
        }
        tab
    }
}

/// Parameter von „Tonwertkorrektur…“ (`CLevelsFilter`).
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct LevelsParams {
    pub rgb: LevelsChannel,
    pub red: LevelsChannel,
    pub green: LevelsChannel,
    pub blue: LevelsChannel,
}

impl LevelsParams {
    /// Die vier Tabellen.
    pub fn tables(&self) -> AdjustTables {
        AdjustTables {
            rgb: self.rgb.table(),
            red: self.red.table(),
            green: self.green.table(),
            blue: self.blue.table(),
        }
    }
}

impl PixelFilter for LevelsParams {
    fn process(&self, src: &RgbaImage, region: IRect) -> RgbaImage {
        self.tables().apply_region(src, region)
    }
}

/// „Tonwertkorrektur…“ anwenden.
pub fn levels(img: &RgbaImage, p: &LevelsParams, mask: Option<&Mask>) -> RgbaImage {
    p.apply(img, mask)
}

// ------------------------------------------------------------------ Auto-Tonwert

/// „Auto-Tonwertkorrektur“ (`CAutoLevelsFilter`, 06 §4.5), keine wirksamen Parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct AutoLevels;

/// 0,5-%-Grenzen eines Histogramms (`@00b21d70`/`@00b21dc0`).
pub fn histogram_bounds(hist: &[u64; 256]) -> (i32, i32) {
    let total: u64 = hist.iter().sum();
    let thr = ((total as f64 * 0.005) as u64).max(1);
    let mut lo = 0;
    let mut cum = 0u64;
    let mut found = false;
    for (i, &h) in hist.iter().enumerate() {
        cum += h;
        if cum >= thr {
            lo = i as i32;
            found = true;
            break;
        }
    }
    if !found {
        lo = 0;
    }
    let mut hi = 255;
    cum = 0;
    for i in (0..256).rev() {
        cum += hist[i];
        if cum >= thr {
            hi = i as i32;
            break;
        }
    }
    (lo, hi)
}

impl AutoLevels {
    /// Tonwertparameter aus den Histogrammen von `region` (nur Pixel mit Alpha > 0).
    pub fn levels_for(src: &RgbaImage, region: IRect) -> LevelsParams {
        let mut h = [[0u64; 256]; 3];
        let r = region.intersect(&src.bounds());
        for y in r.top..r.bottom {
            let row = src.row(y).unwrap_or(&[]);
            for p in
                &row[(r.left - src.bounds().left) as usize..(r.right - src.bounds().left) as usize]
            {
                if p[3] > 0 {
                    for c in 0..3 {
                        h[c][p[c] as usize] += 1;
                    }
                }
            }
        }
        let ch = |c: usize| {
            let (lo, hi) = histogram_bounds(&h[c]);
            LevelsChannel {
                source_low: lo,
                source_high: hi,
                dest_low: 0,
                dest_high: 255,
                gamma: 1.0,
            }
        };
        LevelsParams {
            rgb: LevelsChannel::default(),
            red: ch(0),
            green: ch(1),
            blue: ch(2),
        }
    }
}

impl PixelFilter for AutoLevels {
    fn process(&self, src: &RgbaImage, region: IRect) -> RgbaImage {
        AutoLevels::levels_for(src, region)
            .tables()
            .apply_region(src, region)
    }
}

/// „Auto-Tonwertkorrektur“ anwenden.
pub fn auto_levels(img: &RgbaImage, mask: Option<&Mask>) -> RgbaImage {
    AutoLevels.apply(img, mask)
}

// ------------------------------------------------------------------ Umkehren

/// „Umkehren“ (`CInvertFilter`, 06 §4.6): `RGB' = 255 − RGB`, Alpha bleibt, auch für
/// transparente Pixel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct Invert;

impl PixelFilter for Invert {
    fn process(&self, src: &RgbaImage, region: IRect) -> RgbaImage {
        let mut t = AdjustTables::default();
        for (i, v) in t.rgb.iter_mut().enumerate() {
            *v = 255 - i as u8;
        }
        t.apply_region(src, region)
    }
}

/// „Umkehren“ anwenden.
pub fn invert(img: &RgbaImage, mask: Option<&Mask>) -> RgbaImage {
    Invert.apply(img, mask)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_curve_and_levels() {
        assert_eq!(
            curve_table(&CurvesParams::default().rgb_points),
            identity_table()
        );
        assert_eq!(LevelsChannel::default().table(), identity_table());
        let inv = LevelsChannel {
            dest_low: 255,
            dest_high: 0,
            ..Default::default()
        }
        .table();
        assert_eq!(inv[0], 255);
        assert_eq!(inv[255], 0);
        // hi = lo → NaN → 0
        let z = LevelsChannel {
            source_low: 100,
            source_high: 100,
            ..Default::default()
        }
        .table();
        assert!(z.iter().all(|&v| v == 0));
    }

    #[test]
    fn histogram_threshold() {
        let mut h = [0u64; 256];
        h[10] = 1000;
        h[200] = 1000;
        assert_eq!(histogram_bounds(&h), (10, 200));
        assert_eq!(histogram_bounds(&[0; 256]), (0, 255));
    }
}

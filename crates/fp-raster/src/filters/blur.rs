//! Bewegungsunschärfe (06 §4.11), Radialer Weichzeichner (§4.12) und Zoom-Weichzeichner
//! („Zentriertes Weichzeichnen“, §4.13).
//!
//! Die Filtercodes begrenzen ihre Werte nicht; nur die Dialoge tun es (06 §8.1, Konstanten
//! `*_RANGE`). Werte aus Dateien werden beim Rendern ungeprüft übernommen.

use fp_geom::IRect;

use super::{PixelFilter, start};
use crate::image::RgbaImage;
use crate::mask::Mask;
use crate::par::for_each_row;

// ------------------------------------------------------------------ Bewegungsunschärfe

/// Parameter von „Bewegungsunschärfe…“ (`CMotionBlurFilter`, `@006e4720` Modus 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct MotionBlurParams {
    /// `motion_blur_angle` in Grad (Dialog 0…360, Winkelrad), Standard 90.
    pub angle: i32,
    /// `motion_blur_distance` in Pixeln (Dialog 1…100), Standard 10.
    pub distance: i32,
}

impl Default for MotionBlurParams {
    fn default() -> Self {
        Self {
            angle: 90,
            distance: 10,
        }
    }
}

impl MotionBlurParams {
    /// Dialogbereich Winkel.
    pub const ANGLE_RANGE: (i32, i32) = (0, 360);
    /// Dialogbereich Umfang (Schieberegler 0…100).
    pub const DISTANCE_RANGE: (i32, i32) = (1, 100);

    /// Werte auf die Dialogbereiche begrenzt.
    pub fn dialog_clamped(self) -> Self {
        Self {
            angle: self.angle.clamp(0, 360),
            distance: self.distance.clamp(1, 100),
        }
    }
}

impl PixelFilter for MotionBlurParams {
    fn output_bounds(&self, input: IRect) -> IRect {
        let d = self.distance.max(0);
        input.inflate(d, d)
    }

    fn process(&self, src: &RgbaImage, region: IRect) -> RgbaImage {
        let mut out = start(src);
        if self.distance < 0 {
            return out;
        }
        let a = self.angle.rem_euclid(360);
        let (s, c) = motion_sin_cos(a);
        let b = src.bounds();
        let r = region.intersect(&b);
        if r.is_empty() {
            return out;
        }
        let (w, h) = (src.width() as i64, src.height() as i64);
        let wu = src.width();
        // Versätze je Probe (abgeschnitten wie `_ftol`), Proben `i = 0 … abstand` (06 §4.11). Der
        // Winkel liegt im Original als `float` vor: sin(90°) bzw. cos(180°) sind dadurch knapp
        // unter 1, `trunc(i·sin)` ergibt `i − 1`, die Unschärfe reicht dort nur `abstand − 1` weit
        // und das Pixel selbst zählt doppelt (FX-3, `re\23` Gruppe 3: 90°/10 → Vektor +9 unten,
        // Alpha-Stufen 255·k/11; 0°/1 → 1 px nach links mit Alpha 127; PM-6 D47 90°/20 → +19).
        let offs: Vec<(i64, i64)> = (0..=self.distance as i64)
            .map(|i| ((i as f64 * c) as i64, (i as f64 * s) as i64))
            .collect();
        let sdata = src.data();
        let (x0, x1) = ((r.left - b.left) as usize, (r.right - b.left) as usize);
        let (y0, y1) = ((r.top - b.top) as usize, (r.bottom - b.top) as usize);
        for_each_row(&mut out.data_mut()[y0 * wu..y1 * wu], wu, |ri, row| {
            let y = (y0 + ri) as i64;
            for x in x0..x1 {
                let mut acc = [0u64; 4];
                let mut cnt = 0u64;
                for &(dx, dy) in &offs {
                    let (sx, sy) = (x as i64 + dx, y - dy);
                    if sx < 0 || sx >= w || sy < 0 || sy >= h {
                        break;
                    }
                    let p = sdata[sy as usize * wu + sx as usize];
                    if p[3] != 0 {
                        for k in 0..4 {
                            acc[k] += p[k] as u64;
                        }
                    }
                    cnt += 1;
                }
                if cnt > 0 {
                    row[x] = acc.map(|v| (v / cnt).min(255) as u8);
                }
            }
        });
        out
    }
}

/// Sinus/Kosinus des Winkels wie im Original: `θ = (a mod 360)·π/180` in einfacher Genauigkeit
/// (`π` als `f32`), die Winkelfunktionen dann in doppelter (so liegen sin 90° und −cos 180° knapp
/// unter 1). Reichweite je Achse damit `trunc(d·cos θ)` bzw. `trunc(d·sin θ)` (re 29 §9, BA-012:
/// 0/20 → 20 px links, 90/180/270 → 19 px).
fn motion_sin_cos(angle: i32) -> (f64, f64) {
    let th = (angle.rem_euclid(360) as f32 * std::f32::consts::PI / 180.0f32) as f64;
    (th.sin(), th.cos())
}

/// Bewegungsunschärfe anwenden (Ergebnis um `distance` größer).
pub fn motion_blur(img: &RgbaImage, p: &MotionBlurParams, mask: Option<&Mask>) -> RgbaImage {
    p.apply(img, mask)
}

// ------------------------------------------------------------------ Radial

/// Parameter von „Radiales Weichzeichnen…“ (`CRadialBlurFilter`, `@00b24e70`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct RadialBlurParams {
    /// `radial_blur_amount` (Dialog 1…100), Standard 30.
    pub amount: i32,
    /// `radial_blur_quality` (Dialog 1…100), Standard 20.
    pub quality: i32,
}

impl Default for RadialBlurParams {
    fn default() -> Self {
        Self {
            amount: 30,
            quality: 20,
        }
    }
}

/// Halbkante `R` des quadratischen Puffers um die Mitte (`@00b24a70`).
fn radial_half(w: i64, h: i64) -> i64 {
    let (cx, cy) = (w / 2, h / 2);
    let (hw, hh) = (w - cx, h - cy);
    if hw == 0 {
        if hh == 0 { 1 } else { hh + 1 }
    } else if hh == 0 {
        hw + 1
    } else {
        ((hw * hw + hh * hh) as f64).sqrt().floor() as i64 + 1
    }
}

impl RadialBlurParams {
    /// Dialogbereich Menge/Qualität.
    pub const RANGE: (i32, i32) = (1, 100);

    /// Werte auf die Dialogbereiche begrenzt.
    pub fn dialog_clamped(self) -> Self {
        Self {
            amount: self.amount.clamp(1, 100),
            quality: self.quality.clamp(1, 100),
        }
    }
}

impl PixelFilter for RadialBlurParams {
    /// Quadrat um die Mitte `(links + B/2, oben + H/2)` mit Halbkante
    /// `R = ⌊√(hw² + hh²)⌋ + 1` (`@00b24d70`).
    fn output_bounds(&self, input: IRect) -> IRect {
        if input.is_empty() {
            return input;
        }
        let (w, h) = (input.width() as i64, input.height() as i64);
        let r = radial_half(w, h);
        let cx = input.left as i64 + w / 2;
        let cy = input.top as i64 + h / 2;
        let c = |v: i64| v.clamp(i32::MIN as i64, i32::MAX as i64) as i32;
        IRect::new(c(cx - r), c(cy - r), c(cx + r), c(cy + r))
    }

    fn process(&self, src: &RgbaImage, region: IRect) -> RgbaImage {
        let mut out = start(src);
        let maxang = (self.amount as f64 / 100.0) * std::f64::consts::FRAC_PI_2 * 0.5;
        let n = (self.quality as f64 / 100.0 * 30.0 + 0.5).floor() as i64;
        if n < 1 {
            return out;
        }
        let b = src.bounds();
        let r = region.intersect(&b);
        if r.is_empty() {
            return out;
        }
        let (w, h) = (src.width() as i64, src.height() as i64);
        let wu = src.width();
        // Mitte im Puffer: der Puffer ist das Quadrat 2R × 2R → (W/2, H/2)
        let (cx, cy) = ((w / 2) as f64, (h / 2) as f64);
        let ts: Vec<f64> = (1..n).map(|k| (k as f64 / n as f64) * maxang).collect();
        let sdata = src.data();
        let (x0, x1) = ((r.left - b.left) as usize, (r.right - b.left) as usize);
        let (y0, y1) = ((r.top - b.top) as usize, (r.bottom - b.top) as usize);
        for_each_row(&mut out.data_mut()[y0 * wu..y1 * wu], wu, |ri, row| {
            let y = (y0 + ri) as f64;
            for x in x0..x1 {
                let vx = x as f64 - cx;
                let vy = y - cy;
                let rad = (vx * vx + vy * vy).sqrt();
                let phi = vy.atan2(vx);
                let (mut sa, mut tot, mut nz) = (0u64, 0u64, 0u64);
                let mut sc = [0u64; 3];
                for &t in &ts {
                    for ang in [phi - t, t + phi] {
                        let px = (ang.cos() * rad + cx).trunc();
                        let py = (ang.sin() * rad + cy).trunc();
                        if !(px > 0.0 && px < w as f64 && py > 0.0 && py < h as f64) {
                            continue;
                        }
                        let p = sdata[py as usize * wu + px as usize];
                        tot += 1;
                        if p[3] != 0 {
                            nz += 1;
                            sa += p[3] as u64;
                            for k in 0..3 {
                                sc[k] += p[k] as u64;
                            }
                        }
                    }
                }
                if tot > 0 {
                    let a = (sa / tot).min(255) as u8;
                    let col = |k: usize| {
                        if nz > 0 {
                            (sc[k] / nz).min(255) as u8
                        } else {
                            0
                        }
                    };
                    row[x] = [col(0), col(1), col(2), a];
                }
            }
        });
        out
    }
}

/// Radiales Weichzeichnen anwenden (Ergebnis = Quadrat um die Mitte).
pub fn radial_blur(img: &RgbaImage, p: &RadialBlurParams, mask: Option<&Mask>) -> RgbaImage {
    p.apply(img, mask)
}

// ------------------------------------------------------------------ Zoom

/// Parameter von „Zentriertes Weichzeichnen…“ (`CZoomBlurFilter`, `@00b2cba0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ZoomBlurParams {
    /// `zoom_blur_amount` (Dialog 1…100), Standard 30.
    pub amount: i32,
    /// `zoom_blur_quality` (Dialog 1…100), Standard 20.
    pub quality: i32,
    /// `zoom_blur_center_x/y` in Dokumentkoordinaten (A); `None` (Normalfall in Dateien) =
    /// Mitte der Bildgrenzen `links + (B+1)/2`, `oben + (H+1)/2`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub center: Option<(i32, i32)>,
}

impl Default for ZoomBlurParams {
    fn default() -> Self {
        Self {
            amount: 30,
            quality: 20,
            center: None,
        }
    }
}

impl ZoomBlurParams {
    /// Dialogbereich Anzahl/Qualität.
    pub const RANGE: (i32, i32) = (1, 100);

    /// Werte auf die Dialogbereiche begrenzt.
    pub fn dialog_clamped(self) -> Self {
        Self {
            amount: self.amount.clamp(1, 100),
            quality: self.quality.clamp(1, 100),
            ..self
        }
    }
}

impl PixelFilter for ZoomBlurParams {
    /// `@00b2c9e0` (06 §4.13): Rechteck um `menge` erweitert, dann Quadrat um dessen Mitte mit
    /// Halbkante `⌊√(((H'+1)/2)² + ((B'+1)/2)²) + 0,5⌋ + menge`. Bestätigt durch `re\23` Gruppe 3
    /// (100/100 am Vektor 75,55–165,125 → sichtbar 32,21–207,158, weit über das Objekt hinaus).
    fn output_bounds(&self, input: IRect) -> IRect {
        if input.is_empty() {
            return input;
        }
        let m = self.amount.clamp(0, 100_000) as i64;
        let (l, t) = (input.left as i64 - m, input.top as i64 - m);
        let (r, b) = (input.right as i64 + m, input.bottom as i64 + m);
        let (w, h) = (r - l, b - t);
        let (hx, hy) = ((w + 1) / 2, (h + 1) / 2);
        let half = (((hx * hx + hy * hy) as f64).sqrt() + 0.5).floor() as i64 + m;
        let (cx, cy) = (l + w / 2, t + h / 2);
        let c = |v: i64| v.clamp(i32::MIN as i64 / 2, i32::MAX as i64 / 2) as i32;
        IRect::new(c(cx - half), c(cy - half), c(cx + half), c(cy + half))
    }

    /// Mittelpunkt ohne gespeicherte Schlüssel = Mitte der **Eingabe**grenzen (nicht des
    /// vergrößerten Puffers), daher hier vor dem Rahmen festgelegt.
    fn apply(&self, img: &RgbaImage, mask: Option<&Mask>) -> RgbaImage {
        let mut p = *self;
        if p.center.is_none() {
            let b = img.bounds();
            p.center = Some((
                b.left + (b.width() as i32 + 1) / 2,
                b.top + (b.height() as i32 + 1) / 2,
            ));
        }
        super::apply_framework(&p, img, mask)
    }

    fn process(&self, src: &RgbaImage, region: IRect) -> RgbaImage {
        let mut out = start(src);
        let b = src.bounds();
        let r = region.intersect(&b);
        if r.is_empty() {
            return out;
        }
        let (w, h) = (src.width() as i64, src.height() as i64);
        let wu = src.width();
        let (cx, cy) = match self.center {
            Some((x, y)) => (x as i64 - b.left as i64, y as i64 - b.top as i64),
            None => ((w + 1) / 2, (h + 1) / 2),
        };
        let amt = self.amount as f64 / 100.0;
        let qf = (self.quality as f64 * 0.75) / 100.0 + 0.25;
        let sdata = src.data();
        let (x0, x1) = ((r.left - b.left) as usize, (r.right - b.left) as usize);
        let (y0, y1) = ((r.top - b.top) as usize, (r.bottom - b.top) as usize);
        for_each_row(&mut out.data_mut()[y0 * wu..y1 * wu], wu, |ri, row| {
            let y = (y0 + ri) as i64;
            let ady = (cy - y).abs();
            let sy = if cy >= y { 1 } else { -1 };
            for x in x0..x1 {
                let xi = x as i64;
                let adx = (cx - xi).abs();
                let sx = if cx >= xi { 1 } else { -1 };
                let (dd, mut err, d, nn, maj, mnr) = if adx < ady {
                    (
                        ady,
                        (ady as f64 * 0.5) as i64,
                        adx,
                        (ady as f64 * 0.5 * amt) as i64,
                        (0, sy),
                        (sx, 0),
                    )
                } else {
                    (
                        adx,
                        (adx as f64 * 0.5) as i64,
                        ady,
                        (adx as f64 * 0.5 * amt) as i64,
                        (sx, 0),
                        (0, sy),
                    )
                };
                if nn < 1 {
                    continue;
                }
                let thr = nn as f64 / (qf * nn as f64);
                let (mut p1, mut p2) = ((xi, y), (xi, y));
                let mut ctr = 0.0f64;
                let (mut sa, mut tot, mut nz) = (0u64, 0u64, 0u64);
                let mut sc = [0u64; 3];
                let inside = |p: (i64, i64)| p.0 >= 0 && p.0 < w && p.1 >= 0 && p.1 < h;
                for _ in 0..nn {
                    ctr += 1.0;
                    if thr <= ctr {
                        for p in [p1, p2] {
                            if inside(p) {
                                let v = sdata[p.1 as usize * wu + p.0 as usize];
                                tot += 1;
                                if v[3] != 0 {
                                    sa += v[3] as u64;
                                    nz += 1;
                                    for k in 0..3 {
                                        sc[k] += v[k] as u64;
                                    }
                                }
                            }
                        }
                        ctr = 0.0;
                    }
                    err += d;
                    if dd <= err {
                        p1 = (p1.0 + mnr.0, p1.1 + mnr.1);
                        p2 = (p2.0 - mnr.0, p2.1 - mnr.1);
                        err -= dd;
                    }
                    p1 = (p1.0 + maj.0, p1.1 + maj.1);
                    p2 = (p2.0 - maj.0, p2.1 - maj.1);
                    // Beide Punkte haben den Puffer auf der Hauptachse verlassen: es kommt keine
                    // gültige Probe mehr (Hauptschritte sind monoton).
                    let out_major = |p: (i64, i64)| {
                        if maj.0 != 0 {
                            p.0 < 0 || p.0 >= w
                        } else {
                            p.1 < 0 || p.1 >= h
                        }
                    };
                    if out_major(p1) && out_major(p2) {
                        break;
                    }
                }
                if tot == 0 {
                    continue;
                }
                let a = (sa / tot).min(255) as u8;
                let col = |k: usize| {
                    if nz > 0 {
                        (sc[k] / nz).min(255) as u8
                    } else {
                        0
                    }
                };
                row[x] = [col(0), col(1), col(2), a];
            }
        });
        out
    }
}

/// Zentriertes Weichzeichnen anwenden.
pub fn zoom_blur(img: &RgbaImage, p: &ZoomBlurParams, mask: Option<&Mask>) -> RgbaImage {
    p.apply(img, mask)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radial_bounds() {
        let p = RadialBlurParams::default();
        // 10×6: Mitte (5,3), hw=5, hh=3 → R = floor(√34)+1 = 6
        assert_eq!(
            p.output_bounds(IRect::new(0, 0, 10, 6)),
            IRect::new(-1, -3, 11, 9)
        );
        let m = MotionBlurParams::default();
        assert_eq!(
            m.output_bounds(IRect::new(0, 0, 4, 4)),
            IRect::new(-10, -10, 14, 14)
        );
    }
}

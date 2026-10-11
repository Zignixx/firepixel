//! Gaußscher Weichzeichner (`CGaussianBlurFilter`, 06 §4.7) und Unscharf maskieren
//! (`CUnsharpMaskFilter`, §4.10).
//!
//! - Radius = σ, auf 0,1…250 begrenzt. **r > 200**: das Rechteck des Eingabebildes (bzw. der
//!   Rechenbereich) wird deckend mit der ungewichteten Mittelfarbe aller Pixel gefüllt (auch Alpha
//!   0 zählt; leer → Weiß; B Laufzeit T6). Sonst `σ = min(r, 70)`.
//! - Ausgabegrenzen: Eingabe ± `ceil(2,5·σ)` (bei Pfaden Gleitkomma-Grenzen ± m, dann `floor`,
//!   [`gaussian_output_bounds`]); die Eingabe wird transparent darauf erweitert, das Fenster erst am
//!   Rand dieses Puffers abgeschnitten und neu normiert.
//! - Kern `n = ceil(3σ)`, `w(x) = exp(−x²/((σ+σ)·σ)) · (1/(2πσ))^0,5`.
//! - Separierbar, erst waagerecht, dann senkrecht, Zwischenergebnis 8 Bit (abgeschnitten).
//!   Je Ausgabepixel Summen in Indexreihenfolge: `ΣW, ΣA` über alle Fensterpixel, `ΣWc, ΣC` nur
//!   über Pixel mit Alpha ≠ 0; `A' = ⌊ΣA/ΣW⌋`, `C' = ⌊ΣC/ΣWc⌋` (bzw. 0). Die Farbe ist also nicht
//!   alphagewichtet.
//! - **Rechengenauigkeit** ([`GaussPrecision`]): Das Original hängt vom FPU-Steuerwort des Rechners
//!   ab (06 §4.7, §11 Nr. 12). Die Korpus-Caches passen zu 24 Bit (`Single`, Vorgabe), das Original
//!   mit 53-Bit-Steuerwort zu `Double` (= Python-Referenz `gaussian_blur`).
//! - Die Mehrkern-Variante des Originals rechnet jedes Pixel gleich; hier wird zeilenweise mit
//!   rayon parallelisiert, Summationsreihenfolge je Pixel unverändert → bitgleich.

use fp_geom::{IRect, Rect};

use super::{start, PixelFilter};
use crate::image::{Px, RgbaImage};
use crate::mask::Mask;
use crate::par::for_each_row;

/// Rechengenauigkeit des Gauß-Weichzeichners.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
pub enum GaussPrecision {
    /// x87 mit 24-Bit-Mantisse (jede Operation als `f32`, Kern `gaussian_kernel_pc24`): passt zu
    /// den Caches im Korpus (voll deckend → Alpha 253).
    #[default]
    Single,
    /// IEEE double (53 Bit), wie `fwfilters.gaussian_blur_raw`.
    Double,
}

/// Normalisierter Radius: `None` = Mittelwertfüllung (r > 200), sonst σ (≤ 70).
pub fn effective_sigma(radius: f64) -> Option<f64> {
    let r = if radius.is_nan() { 0.1 } else { radius.clamp(0.1, 250.0) };
    if r > 200.0 { None } else { Some(r.min(70.0)) }
}

/// Randzugabe `ceil(2,5·σ)` (höchstens 175).
pub fn gaussian_margin(radius: f64) -> i32 {
    match effective_sigma(radius) {
        Some(s) => (s * 2.5).ceil() as i32,
        None => 0,
    }
}

/// Ausgabegrenzen für Gleitkomma-Objektgrenzen (Pfade): Grenzen ± `ceil(2,5·σ)`, dann auf allen
/// vier Seiten `floor` (06 §2.4).
pub fn gaussian_output_bounds(bounds: Rect, radius: f64) -> IRect {
    let m = gaussian_margin(radius) as f32;
    bounds.inflate(m, m).round_floor()
}

/// Gauß-Kern in double (`@006dfdf0`): `(n, Gewichte der Länge 2n+1)`.
pub fn gaussian_kernel(sigma: f64) -> (usize, Vec<f64>) {
    let n = (sigma * 3.0).ceil().max(0.0) as usize;
    let norm = (1.0 / (sigma * std::f64::consts::TAU)).powf(0.5);
    let two_s2 = (sigma + sigma) * sigma;
    let half: Vec<f64> = (0..=n).map(|i| {
        let x = i as f64 - n as f64;
        (-(x * x) / two_s2).exp() * norm
    }).collect();
    (n, mirror(half))
}

/// Gauß-Kern unter 24-Bit-Genauigkeit (`gaussian_kernel_pc24`): `σ·2π` und `1/(…)` als `f32`,
/// `pow`/`exp` in double, Produkt als `f32`.
pub fn gaussian_kernel_pc24(sigma: f64) -> (usize, Vec<f32>) {
    let f = |v: f64| v as f32 as f64;
    let n = (sigma * 3.0).ceil().max(0.0) as usize;
    let two = f(f(sigma + sigma) * sigma);
    let inv = f(1.0 / f(sigma * std::f64::consts::TAU));
    let norm = inv.powf(0.5);
    let half: Vec<f32> = (0..=n)
        .map(|i| {
            let x = i as i64 - n as i64;
            let e = (f(-((x * x) as f64)) / two).exp();
            (e * norm) as f32
        })
        .collect();
    (n, mirror(half))
}

fn mirror<T: Copy>(half: Vec<T>) -> Vec<T> {
    let mut k = half.clone();
    k.extend(half.iter().rev().skip(1));
    k
}

/// Eine Achse: `get(j)` liefert das Quellpixel an Index `j`; schreibt das Ergebnis für Ziel `t`.
/// Zeilenweise Pässe mit Summen je Ausgabepixel in Indexreihenfolge.
trait Acc: Copy + Send + Sync {
    fn pass_row(src: &[Px], dst: &mut [Px], lo_b: usize, hi_b: usize, x0: usize, kernel: &[Self], n: usize);
}

impl Acc for f64 {
    fn pass_row(src: &[Px], dst: &mut [Px], lo_b: usize, hi_b: usize, x0: usize, k: &[f64], n: usize) {
        for (o, d) in dst.iter_mut().enumerate() {
            let t = x0 + o;
            let lo = lo_b.max(t.saturating_sub(n));
            let hi = (hi_b - 1).min(t + n);
            let (mut sw, mut sa, mut swc) = (0.0f64, 0.0f64, 0.0f64);
            let mut sc = [0.0f64; 3];
            for (j, p) in src.iter().enumerate().take(hi + 1).skip(lo) {
                let w = k[j + n - t];
                sa += p[3] as f64 * w;
                sw += w;
                if p[3] != 0 {
                    swc += w;
                    for c in 0..3 {
                        sc[c] += p[c] as f64 * w;
                    }
                }
            }
            let tr = |v: f64| if v.is_finite() { ((v.trunc() as i64) & 0xFF) as u8 } else { 0 };
            let col = |c: usize| if swc != 0.0 { tr(sc[c] / swc) } else { 0 };
            *d = [col(0), col(1), col(2), if sw != 0.0 { tr(sa / sw) } else { 0 }];
        }
    }
}

impl Acc for f32 {
    fn pass_row(src: &[Px], dst: &mut [Px], lo_b: usize, hi_b: usize, x0: usize, k: &[f32], n: usize) {
        for (o, d) in dst.iter_mut().enumerate() {
            let t = x0 + o;
            let lo = lo_b.max(t.saturating_sub(n));
            let hi = (hi_b - 1).min(t + n);
            let (mut sw, mut sa, mut swc) = (0.0f32, 0.0f32, 0.0f32);
            let mut sc = [0.0f32; 3];
            for (j, p) in src.iter().enumerate().take(hi + 1).skip(lo) {
                let w = k[j + n - t];
                // Jede Operation einzeln als f32 (Addition kommutativ, also bitgleich zu `x + s`).
                if p[3] != 0 {
                    for c in 0..3 {
                        sc[c] += (p[c] as f32) * w;
                    }
                    swc += w;
                }
                sa += (p[3] as f32) * w;
                sw += w;
            }
            let fl = |v: f32| v.floor().clamp(0.0, 255.0) as u8;
            let col = |c: usize| if swc != 0.0 { fl(sc[c] / swc) } else { 0 };
            *d = [col(0), col(1), col(2), if sw != 0.0 { fl(sa / sw) } else { 0 }];
        }
    }
}

fn blur_generic<T: Acc>(src: &RgbaImage, region: IRect, kernel: &[T], n: usize) -> RgbaImage {
    let mut out = start(src);
    let b = src.bounds();
    let r = region.intersect(&b);
    if r.is_empty() {
        return out;
    }
    let (w, h) = (src.width(), src.height());
    let (x0, x1) = ((r.left - b.left) as usize, (r.right - b.left) as usize);
    let (y0, y1) = ((r.top - b.top) as usize, (r.bottom - b.top) as usize);
    // waagerechter Durchgang über die Zeilen region ± n (im Puffer), Spalten x0..x1
    let ty0 = y0.saturating_sub(n);
    let ty1 = (y1 + n).min(h);
    let cw = x1 - x0;
    let mut tmp = vec![[0u8; 4]; cw * (ty1 - ty0)];
    let sdata = src.data();
    for_each_row(&mut tmp, cw, |i, row| {
        let y = ty0 + i;
        T::pass_row(&sdata[y * w..(y + 1) * w], row, 0, w, x0, kernel, n);
    });
    // senkrechter Durchgang: Spalten von tmp (Zeilen ty0..ty1)
    let th = ty1 - ty0;
    let mut cols = vec![[0u8; 4]; cw * th];
    // transponieren, damit die Spalten zusammenhängend sind
    for yy in 0..th {
        for xx in 0..cw {
            cols[xx * th + yy] = tmp[yy * cw + xx];
        }
    }
    let oh = y1 - y0;
    let mut res = vec![[0u8; 4]; cw * oh];
    for_each_row(&mut res, oh, |xx, col_out| {
        // Ausgabe-Spalte xx, Ziele t = y0..y1 im Puffer ↔ Index t − ty0 in der Spalte
        T::pass_row(&cols[xx * th..(xx + 1) * th], col_out, 0, th, y0 - ty0, kernel, n);
    });
    let data = out.data_mut();
    for xx in 0..cw {
        for yy in 0..oh {
            data[(y0 + yy) * w + x0 + xx] = res[xx * oh + yy];
        }
    }
    out
}

/// Separierbarer Gauß auf `region` des Puffers `src` (ohne Grenzvergrößerung, ohne Sonderfall
/// r > 200): entspricht `gaussian_blur_raw` (Double) bzw. `gaussian_blur_raw_pc24` (Single).
pub fn gaussian_blur_raw(src: &RgbaImage, region: IRect, sigma: f64, precision: GaussPrecision) -> RgbaImage {
    match precision {
        GaussPrecision::Double => {
            let (n, k) = gaussian_kernel(sigma);
            blur_generic(src, region, &k, n)
        }
        GaussPrecision::Single => {
            let (n, k) = gaussian_kernel_pc24(sigma);
            blur_generic(src, region, &k, n)
        }
    }
}

/// Mittelwertfarbe für r > 200 (`StCalcAverageProcessor`, Gewichtungs-Flag 0): ungewichtet über
/// alle Pixel von `region`, Alpha 255; leer → Weiß.
pub fn average_color(src: &RgbaImage, region: IRect) -> Px {
    let r = region.intersect(&src.bounds());
    let n = r.area();
    if n == 0 {
        return [255, 255, 255, 255];
    }
    let mut s = [0u64; 3];
    for y in r.top..r.bottom {
        let row = src.row(y).unwrap_or(&[]);
        for p in &row[(r.left - src.bounds().left) as usize..(r.right - src.bounds().left) as usize] {
            for c in 0..3 {
                s[c] += p[c] as u64;
            }
        }
    }
    let m = |c: usize| (s[c] / n).min(255) as u8;
    [m(0), m(1), m(2), 255]
}

/// Parameter des Gaußschen Weichzeichners.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct GaussianBlurParams {
    /// `gaussian_blur_radius` (= σ), Dialog 0,1…250, Standard 4,1.
    pub radius: f64,
    /// Rechengenauigkeit (keine Dateieigenschaft, sondern Render-Option).
    #[serde(skip_serializing_if = "is_default_precision")]
    pub precision: GaussPrecision,
}

fn is_default_precision(p: &GaussPrecision) -> bool {
    *p == GaussPrecision::default()
}

impl Default for GaussianBlurParams {
    fn default() -> Self {
        Self { radius: 4.1, precision: GaussPrecision::default() }
    }
}

impl GaussianBlurParams {
    /// Dialogbereich des Radius (DDV).
    pub const RADIUS_RANGE: (f64, f64) = (0.1, 250.0);

    /// Radius auf den Dialogbereich begrenzt.
    pub fn dialog_clamped(self) -> Self {
        Self { radius: if self.radius.is_nan() { 4.1 } else { self.radius.clamp(0.1, 250.0) }, ..self }
    }
}

impl PixelFilter for GaussianBlurParams {
    fn output_bounds(&self, input: IRect) -> IRect {
        let m = gaussian_margin(self.radius);
        input.inflate(m, m)
    }

    fn process(&self, src: &RgbaImage, region: IRect) -> RgbaImage {
        match effective_sigma(self.radius) {
            None => {
                let mut out = start(src);
                let avg = average_color(src, region);
                out.fill_rect(region, fp_color::Rgba8::from_array(avg));
                out
            }
            Some(s) => gaussian_blur_raw(src, region, s, self.precision),
        }
    }
}

/// Gaußschen Weichzeichner anwenden (Ergebnis um `ceil(2,5·σ)` größer).
pub fn gaussian_blur(img: &RgbaImage, p: &GaussianBlurParams, mask: Option<&Mask>) -> RgbaImage {
    p.apply(img, mask)
}

// ------------------------------------------------------------------ Unscharf maskieren

/// Parameter von „Unscharf maskieren…“.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct UnsharpMaskParams {
    /// `unsharp_mask_radius`: erst ≤ 70, dann 0,1…250 (Dialog 0,1…250), Standard 4,1.
    pub radius: f64,
    /// `unsharp_mask_amount` in %, 1…500, Standard 50.
    pub amount: i32,
    /// `unsharp_mask_threshold`, 0…255, Standard 0.
    pub threshold: i32,
    /// Rechengenauigkeit des inneren Gauß.
    #[serde(skip_serializing_if = "is_default_precision")]
    pub precision: GaussPrecision,
}

impl Default for UnsharpMaskParams {
    fn default() -> Self {
        Self { radius: 4.1, amount: 50, threshold: 0, precision: GaussPrecision::default() }
    }
}

impl UnsharpMaskParams {
    /// Dialogbereich Radius.
    pub const RADIUS_RANGE: (f64, f64) = (0.1, 250.0);
    /// Dialogbereich Stärke.
    pub const AMOUNT_RANGE: (i32, i32) = (1, 500);
    /// Bereich Schwelle.
    pub const THRESHOLD_RANGE: (i32, i32) = (0, 255);

    /// Werte auf die Dialogbereiche begrenzt.
    pub fn dialog_clamped(self) -> Self {
        Self {
            radius: if self.radius.is_nan() { 4.1 } else { self.radius.clamp(0.1, 250.0) },
            amount: self.amount.clamp(1, 500),
            threshold: self.threshold.clamp(0, 255),
            ..self
        }
    }
}

impl PixelFilter for UnsharpMaskParams {
    fn process(&self, src: &RgbaImage, region: IRect) -> RgbaImage {
        let r = if self.radius.is_nan() { 0.1 } else { self.radius.min(70.0).clamp(0.1, 250.0) };
        let blurred = gaussian_blur_raw(src, region, r, self.precision);
        let amt = self.amount.clamp(1, 500) as f64;
        let k = (amt / 100.0 * 255.0 + 0.5).floor() as i32;
        let thr = self.threshold.clamp(0, 255);
        let mut out = start(src);
        let b = src.bounds();
        let rr = region.intersect(&b);
        let w = src.width();
        let bd = blurred.data();
        let data = out.data_mut();
        for y in (rr.top - b.top) as usize..(rr.bottom - b.top) as usize {
            for x in (rr.left - b.left) as usize..(rr.right - b.left) as usize {
                let i = y * w + x;
                let o = data[i];
                let mut n = o;
                for c in 0..4 {
                    let d = o[c] as i32 - bd[i][c] as i32;
                    if d > thr {
                        n[c] = (((k * d) >> 8) + o[c] as i32).min(255) as u8;
                    }
                }
                data[i] = n;
            }
        }
        out
    }
}

/// „Unscharf maskieren…“ anwenden (keine Grenzvergrößerung).
pub fn unsharp_mask(img: &RgbaImage, p: &UnsharpMaskParams, mask: Option<&Mask>) -> RgbaImage {
    p.apply(img, mask)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fp_color::Rgba8;

    fn sample_img() -> RgbaImage {
        let mut img = RgbaImage::new(IRect::new(-3, 5, 40, 33)).unwrap();
        for (i, p) in img.data_mut().iter_mut().enumerate() {
            let a = if i % 7 == 0 { 0 } else { (i * 37 % 256) as u8 };
            *p = [(i * 13) as u8, (i * 29) as u8, (i * 7) as u8, a];
        }
        img
    }

    #[test]
    fn kernel_symmetry_and_bounds() {
        let (n, k) = gaussian_kernel(2.0);
        assert_eq!(n, 6);
        assert_eq!(k.len(), 13);
        assert_eq!(k[0], k[12]);
        let p = GaussianBlurParams { radius: 2.0, ..Default::default() };
        assert_eq!(p.output_bounds(IRect::new(0, 0, 10, 10)), IRect::new(-5, -5, 15, 15));
        // Beispiel Location Pin (06 §2.4)
        assert_eq!(gaussian_output_bounds(Rect::new(-16.097, 1.0, 28.653, 35.0), 2.0), IRect::new(-22, -4, 33, 40));
    }

    #[test]
    fn single_precision_opaque_drops_to_253() {
        let img = RgbaImage::filled(IRect::new(0, 0, 21, 17), Rgba8::rgb(128, 128, 128)).unwrap();
        let p = GaussianBlurParams { radius: 2.0, precision: GaussPrecision::Single };
        let out = p.apply(&img, None);
        assert_eq!(out.pixel(10, 8).a, 253);
        let d = GaussianBlurParams { radius: 2.0, precision: GaussPrecision::Double }.apply(&img, None);
        assert_eq!(d.pixel(10, 8).a, 255);
    }

    #[test]
    fn region_equals_full_inside_and_parallel_is_deterministic() {
        let img = sample_img();
        for prec in [GaussPrecision::Single, GaussPrecision::Double] {
            let full = gaussian_blur_raw(&img, img.bounds(), 3.5, prec);
            let reg = IRect::new(5, 10, 20, 25);
            let part = gaussian_blur_raw(&img, reg, 3.5, prec);
            for y in reg.top..reg.bottom {
                for x in reg.left..reg.right {
                    assert_eq!(full.get(x, y), part.get(x, y));
                }
            }
            let pool = rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap();
            let seq = pool.install(|| gaussian_blur_raw(&img, img.bounds(), 3.5, prec));
            assert_eq!(seq, full);
        }
    }

    #[test]
    fn radius_over_200_fills_input_rect() {
        let img = RgbaImage::from_raw(IRect::new(0, 0, 2, 1), vec![[255, 0, 0, 255], [0, 0, 255, 0]]).unwrap();
        let out = GaussianBlurParams { radius: 210.0, ..Default::default() }.apply(&img, None);
        assert_eq!(out.bounds(), img.bounds());
        assert_eq!(out.data(), &[[127, 0, 127, 255], [127, 0, 127, 255]]);
    }
}

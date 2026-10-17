//! Resampling und Transformation von Bitmaps.
//!
//! Das Original rechnet je Zielpixel die Quellkoordinate der **Zielpixelmitte** zurück (affin bzw.
//! projektiv, 16.16-Festkomma, `@00801ad0`) und ruft einen von drei Abtastern
//! (13 §7.7, Tabelle `@017bb930`):
//!
//! | Abtaster | Original | Verfahren hier |
//! |---|---|---|
//! | [`Sampler::Nearest`] | `@007f3580` | `(u + 0x8000) >> 16` auf der Festkomma-Koordinate (B) |
//! | [`Sampler::Bilinear`] | `@007f8690` | 8-Bit-Bruchteil (Bits 8–15), Alpha linear, Farben alphagewichtet über die Divisionstabelle, Teilung `div255` (B für die Bausteine, A für die genaue Zusammensetzung) |
//! | [`Sampler::Bicubic`], [`Sampler::Soft`] | `@007f82d0` | 4×4-Kern mit `a = −1`, Farben alphagewichtet, Gleitkomma (B Laufzeit 13 T2: max. Δ 1–2 gegen das Original; die Festkomma-Gewichte sind offen) |
//!
//! Achsenparalleles Skalieren einer Bitmap (Skalieren-Werkzeug, Inspektor, `setSelectionBounds`)
//! läuft bikubisch und weich nicht über den Abtaster, sondern getrennt erst waagerecht, dann
//! senkrecht ([`scale_separable`], re 35 §3: Ecken-Abbildung, Vorfilter beim Verkleinern,
//! Gewichte in 1/1024).
//!
//! Die Voreinstellungen `EditTransformMode`/`AliasTransformMode` (14 §3.4) sind 1-basiert:
//! 1 bikubisch, 2 bilinear, 3 weich, 4 nächster Nachbar → [`Sampler::from_pref`]. Symbolinstanzen
//! und das 9-Slice-Mittelstück nutzen `AliasTransformMode` (Vorgabe bikubisch). Miniaturbilder
//! (`prVW`) tasten Bitmaps per nächstem Nachbarn ab (02 §14, Laufzeit T3).
//!
//! Koordinaten: Pixel `i` überdeckt `[i, i+1)`; eine Abtastposition `sx` liegt also bei
//! `u = sx − 0,5` in der Konvention „Pixelmitte = ganze Zahl“.

use fp_geom::{IRect, Matrix3};

use crate::image::{CLEAR, Px, RgbaImage};
use crate::par::for_each_row;

/// Abtastverfahren.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum Sampler {
    /// Nächster Nachbar.
    Nearest,
    /// Bilinear (Original-Festkomma, alphagewichtet).
    Bilinear,
    /// 4×4-Kern, `a = −1` (Vorgabe des Originals).
    #[default]
    Bicubic,
    /// „Weich“: landet im Original im selben 4×4-Abtaster wie bikubisch; die Unterscheidung
    /// (vermutlich Flags 0x400/0x200) ist offen (13 §7.7, ?). Hier gleich [`Sampler::Bicubic`].
    Soft,
}

impl Sampler {
    /// Aus dem Voreinstellungswert `*TransformMode` (1 bikubisch, 2 bilinear, 3 weich,
    /// 4 nächster Nachbar; andere Werte → bikubisch).
    pub fn from_pref(v: i64) -> Sampler {
        match v {
            2 => Sampler::Bilinear,
            3 => Sampler::Soft,
            4 => Sampler::Nearest,
            _ => Sampler::Bicubic,
        }
    }

    /// Voreinstellungswert (1…4).
    pub fn to_pref(self) -> i64 {
        match self {
            Sampler::Bicubic => 1,
            Sampler::Bilinear => 2,
            Sampler::Soft => 3,
            Sampler::Nearest => 4,
        }
    }

    /// Interne Flags von `@007f3960` (0x480 bikubisch, 0x140 bilinear, 0x280 weich, 0x120 nächster Nachbar).
    pub fn flags(self) -> u32 {
        match self {
            Sampler::Bicubic => 0x480,
            Sampler::Bilinear => 0x140,
            Sampler::Soft => 0x280,
            Sampler::Nearest => 0x120,
        }
    }
}

/// Verhalten außerhalb der Quelle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    /// Nachbarn außerhalb des Bildes sind `(0,0,0,0)` (Kanten laufen weich aus).
    Transparent,
    /// Nachbarn werden auf dieses Rechteck (Dokumentkoordinaten) begrenzt (9-Slice-Zonen).
    Clamp(IRect),
}

#[inline]
fn fetch(src: &RgbaImage, x: i64, y: i64, edge: Edge) -> Px {
    match edge {
        Edge::Transparent => {
            if x < i32::MIN as i64
                || x > i32::MAX as i64
                || y < i32::MIN as i64
                || y > i32::MAX as i64
            {
                return CLEAR;
            }
            src.get(x as i32, y as i32).unwrap_or(CLEAR)
        }
        Edge::Clamp(r) => {
            if r.is_empty() {
                return CLEAR;
            }
            let cx = x.clamp(r.left as i64, r.right as i64 - 1) as i32;
            let cy = y.clamp(r.top as i64, r.bottom as i64 - 1) as i32;
            src.get(cx, cy).unwrap_or(CLEAR)
        }
    }
}

/// Festkomma 16.16 der Koordinate `u` (Pixelmitte = ganze Zahl), abgerundet.
#[inline]
fn fixed(u: f64) -> i64 {
    let v = (u * 65536.0).floor();
    if v.is_finite() {
        v.clamp(-(1i64 << 46) as f64, (1i64 << 46) as f64) as i64
    } else {
        0
    }
}

/// Gewicht des 4×4-Kerns (Keys-Kubik mit Parameter `a`), berechnet wie die Laufzeitprüfung
/// `t2_9slice.cubic` (Potenzen über `pow`).
#[inline]
pub fn cubic_weight(t: f64, a: f64) -> f64 {
    let t = t.abs();
    if t < 1.0 {
        (a + 2.0) * t.powf(3.0) - (a + 3.0) * t.powf(2.0) + 1.0
    } else if t < 2.0 {
        a * t.powf(3.0) - 5.0 * a * t.powf(2.0) + 8.0 * a * t - 4.0 * a
    } else {
        0.0
    }
}

/// Parameter `a` des bikubischen Kerns des Originals (13 Laufzeit T2).
pub const BICUBIC_A: f64 = -1.0;

fn sample_bicubic(src: &RgbaImage, sx: f64, sy: f64, edge: Edge) -> Px {
    let u = sx - 0.5;
    let v = sy - 0.5;
    let i0 = u.floor();
    let j0 = v.floor();
    let fu = u - i0;
    let fv = v - j0;
    let (i0, j0) = (i0 as i64, j0 as i64);
    let mut a_sum = 0.0f64;
    let mut c_sum = [0.0f64; 3];
    for di in -1i64..=2 {
        let wx = cubic_weight(di as f64 - fu, BICUBIC_A);
        for dj in -1i64..=2 {
            let wt = wx * cubic_weight(dj as f64 - fv, BICUBIC_A);
            let p = fetch(src, i0 + di, j0 + dj, edge);
            let wa = wt * p[3] as f64;
            a_sum += wa;
            for k in 0..3 {
                c_sum[k] += wa * p[k] as f64;
            }
        }
    }
    if a_sum <= 0.0 {
        return CLEAR;
    }
    let r = |x: f64| (x + 0.5).floor().clamp(0.0, 255.0) as u8;
    [
        r(c_sum[0] / a_sum),
        r(c_sum[1] / a_sum),
        r(c_sum[2] / a_sum),
        r(a_sum),
    ]
}

/// Bilineare Mischung zweier Pixel mit 8-Bit-Gewicht `f` für `p1` (Alpha linear, Farbe
/// alphagewichtet über die Divisionstabelle; A für die genaue Zusammensetzung).
#[inline]
fn lerp_px(p0: Px, p1: Px, f: u32) -> Px {
    use fp_color::round::{DIV, div255, mul255};
    let a0 = mul255(p0[3] as u32, 255 - f);
    let a1 = mul255(p1[3] as u32, f);
    let a = (a0 + a1).min(255);
    if a == 0 {
        return CLEAR;
    }
    let w = DIV[a1.min(a) as usize][a as usize] as u32;
    let c = |k: usize| div255(p0[k] as u32 * (255 - w) + p1[k] as u32 * w) as u8;
    [c(0), c(1), c(2), a as u8]
}

fn sample_bilinear(src: &RgbaImage, sx: f64, sy: f64, edge: Edge) -> Px {
    let fu = fixed(sx - 0.5);
    let fv = fixed(sy - 0.5);
    let (i0, j0) = (fu >> 16, fv >> 16);
    let (wx, wy) = (((fu >> 8) & 0xFF) as u32, ((fv >> 8) & 0xFF) as u32);
    let top = lerp_px(fetch(src, i0, j0, edge), fetch(src, i0 + 1, j0, edge), wx);
    let bot = lerp_px(
        fetch(src, i0, j0 + 1, edge),
        fetch(src, i0 + 1, j0 + 1, edge),
        wx,
    );
    lerp_px(top, bot, wy)
}

/// Tastet `src` an der Position (`sx`, `sy`) ab (Dokumentkoordinaten, Pixel `i` überdeckt
/// `[i, i+1)`).
pub fn sample(src: &RgbaImage, sx: f64, sy: f64, sampler: Sampler, edge: Edge) -> Px {
    match sampler {
        Sampler::Nearest => {
            let i = (fixed(sx - 0.5) + 0x8000) >> 16;
            let j = (fixed(sy - 0.5) + 0x8000) >> 16;
            fetch(src, i, j, edge)
        }
        Sampler::Bilinear => sample_bilinear(src, sx, sy, edge),
        Sampler::Bicubic | Sampler::Soft => sample_bicubic(src, sx, sy, edge),
    }
}

/// Affine (oder projektive) Transformation eines Bildes mit `m` (Zeilenvektor-Konvention von
/// `MTXv`, siehe fp-geom). Ergebnisgrenzen = nach außen gerundete Hülle des transformierten
/// Rechtecks, beschnitten auf `clip`. Nicht invertierbare Matrix → leeres Bild.
pub fn transform(src: &RgbaImage, m: &Matrix3, sampler: Sampler, clip: Option<IRect>) -> RgbaImage {
    let Some(inv) = m.inverse() else {
        return RgbaImage::empty_at(0, 0);
    };
    let mut ob = m.transform_rect_bounds(&src.bounds().to_rect()).round_out();
    if let Some(c) = clip {
        ob = ob.intersect(&c);
    }
    let mut out = match RgbaImage::new(ob) {
        Ok(o) => o,
        Err(_) => return RgbaImage::empty_at(ob.left, ob.top),
    };
    let im = inv.m.map(|r| r.map(|v| v as f64));
    let w = out.width();
    let (l, t) = out.origin();
    for_each_row(out.data_mut(), w, |yi, row| {
        let y = t as f64 + yi as f64 + 0.5;
        for (xi, p) in row.iter_mut().enumerate() {
            let x = l as f64 + xi as f64 + 0.5;
            let mut sx = im[0][0] * x + im[1][0] * y + im[2][0];
            let mut sy = im[0][1] * x + im[1][1] * y + im[2][1];
            let wq = im[0][2] * x + im[1][2] * y + im[2][2];
            if wq != 0.0 && wq != 1.0 {
                sx /= wq;
                sy /= wq;
            }
            *p = sample(src, sx, sy, sampler, Edge::Transparent);
        }
    });
    out
}

/// Skaliert ein Bild auf `width × height` (gleiche linke obere Ecke), Abtastung je
/// Zielpixelmitte („Bild neu auflösen“, Miniaturbilder mit [`Sampler::Nearest`]).
pub fn resize(src: &RgbaImage, width: u32, height: u32, sampler: Sampler) -> RgbaImage {
    let (l, t) = src.origin();
    let ob = IRect::from_xywh(
        l,
        t,
        width.min(i32::MAX as u32) as i32,
        height.min(i32::MAX as u32) as i32,
    );
    let mut out = match RgbaImage::new(ob) {
        Ok(o) => o,
        Err(_) => return RgbaImage::empty_at(l, t),
    };
    if src.is_empty() {
        return out;
    }
    let sxf = src.width() as f64 / width.max(1) as f64;
    let syf = src.height() as f64 / height.max(1) as f64;
    let w = out.width();
    for_each_row(out.data_mut(), w, |yi, row| {
        let sy = t as f64 + (yi as f64 + 0.5) * syf;
        for (xi, p) in row.iter_mut().enumerate() {
            let sx = l as f64 + (xi as f64 + 0.5) * sxf;
            *p = sample(src, sx, sy, sampler, Edge::Clamp(src.bounds()));
        }
    });
    out
}

/// Abbildung einer 9-Slice-Achse (13 Laufzeit T2): Ränder 1:1, Mittelzone „an den Ecken
/// ausgerichtet“ (erstes/letztes Zielpixel trifft genau die Randpixel der Quell-Mittelzone).
/// Liefert je Zielpixel `k` die Abtastposition (Pixelmitte, bildlokal) und die Quellzone
/// (inklusive Grenzen).
pub fn nine_slice_axis(
    k: usize,
    n_dst: usize,
    n_src: usize,
    m1: usize,
    m2: usize,
) -> (f64, usize, usize) {
    let last = n_src.saturating_sub(1);
    let pos = if k < m1 {
        k as f64
    } else if k + m2 >= n_dst {
        n_src as f64 - (n_dst - k) as f64
    } else {
        let nd = n_dst.saturating_sub(m1 + m2);
        let ns = n_src.saturating_sub(m1 + m2);
        if nd > 1 {
            (((k - m1) * ns.saturating_sub(1)) as f64 / (nd - 1) as f64) + m1 as f64
        } else {
            m1 as f64
        }
    };
    let (lo, hi) = if k < m1 {
        (0, m1.saturating_sub(1))
    } else if k + m2 >= n_dst {
        (n_src.saturating_sub(m2), last)
    } else {
        (m1, n_src.saturating_sub(m2 + 1))
    };
    (pos, lo.min(last), hi.max(lo).min(last))
}

/// 9-Slice-Skalierung eines Bildes auf `width × height` mit den Rändern `insets` =
/// (links, oben, rechts, unten) in Quellpixeln. Ränder bleiben 1:1, die Mittelzonen werden „an
/// den Ecken ausgerichtet“ gestreckt, Nachbarn je Zone begrenzt (13 Laufzeit T2: 4×4-Kern
/// a = −1, 615/1.000 bzw. 843/3.600 exakt, max. Δ 1–2 gegen das Original). Ergebnis an derselben
/// linken oberen Ecke.
pub fn nine_slice_scale(
    src: &RgbaImage,
    insets: [u32; 4],
    width: u32,
    height: u32,
    sampler: Sampler,
) -> RgbaImage {
    let (l, t) = src.origin();
    let ob = IRect::from_xywh(
        l,
        t,
        width.min(i32::MAX as u32) as i32,
        height.min(i32::MAX as u32) as i32,
    );
    let mut out = match RgbaImage::new(ob) {
        Ok(o) => o,
        Err(_) => return RgbaImage::empty_at(l, t),
    };
    if src.is_empty() {
        return out;
    }
    let (sw, sh) = (src.width(), src.height());
    let xs: Vec<(f64, usize, usize)> = (0..width as usize)
        .map(|k| {
            nine_slice_axis(
                k,
                width as usize,
                sw,
                insets[0] as usize,
                insets[2] as usize,
            )
        })
        .collect();
    let ys: Vec<(f64, usize, usize)> = (0..height as usize)
        .map(|k| {
            nine_slice_axis(
                k,
                height as usize,
                sh,
                insets[1] as usize,
                insets[3] as usize,
            )
        })
        .collect();
    let w = out.width();
    for_each_row(out.data_mut(), w, |yi, row| {
        let (py, ylo, yhi) = ys[yi];
        for (xi, p) in row.iter_mut().enumerate() {
            let (px, xlo, xhi) = xs[xi];
            let zone = IRect::new(
                l + xlo as i32,
                t + ylo as i32,
                l + xhi as i32 + 1,
                t + yhi as i32 + 1,
            );
            let sx = (l as f64 + px) + 0.5;
            let sy = (t as f64 + py) + 0.5;
            *p = sample(src, sx, sy, sampler, Edge::Clamp(zone));
        }
    });
    out
}

// ---------------------------------------------------------------------------------------------
// Separable scaling of a bitmap (re 35 §3, Q-30 / TW-75): bicubic (default) and "soft"
// ---------------------------------------------------------------------------------------------

/// Bicubic kernel of the original, a = −1, evaluated in the order of re 35 §3.1.
#[inline]
pub fn kernel_bicubic(t: f64) -> f64 {
    let x = t.abs();
    if x < 1.0 {
        (x - 2.0) * t * t + 1.0
    } else if x < 2.0 {
        ((-x + 5.0) * x - 8.0) * x + 4.0
    } else {
        0.0
    }
}

/// "Soft": Mitchell–Netravali with B = C = 1/3 (re 35 §3.3).
#[inline]
pub fn kernel_mitchell(t: f64) -> f64 {
    let x = t.abs();
    if x < 1.0 {
        (7.0 * x * x * x - 12.0 * t * t + 16.0 / 3.0) / 6.0
    } else if x < 2.0 {
        (-7.0 / 3.0 * x * x * x + 12.0 * t * t - 20.0 * x + 32.0 / 3.0) / 6.0
    } else {
        0.0
    }
}

/// Weights of one axis (re 35 §3.2.2): for every target index the list of (source index,
/// weight in 1/1024). `n` source, `m` target size.
pub fn scale_contributions(n: usize, m: usize, kernel: fn(f64) -> f64) -> Vec<Vec<(usize, i32)>> {
    let q = m as f64 / n as f64;
    let (reach, f) = if q >= 1.0 {
        (2.0, 1.0)
    } else {
        (2.0 * n as f64 / m as f64, q)
    };
    let step = if m > 1 {
        (n as f64 - 1.0) / (m as f64 - 1.0)
    } else {
        0.0
    };
    let mut out = Vec::with_capacity(m);
    let mut c = 0.0f64;
    for _ in 0..m {
        let lo = (c - reach).ceil() as i64;
        let hi = (c + reach).floor() as i64;
        let k = |i: i64| kernel((c - i as f64) * f) * f;
        let sum: f64 = (lo..=hi)
            .filter(|&i| i >= 0 && (i as usize) < n)
            .map(k)
            .sum();
        // all candidates in order; `None` = discarded (outside or weight 0)
        let mut list: Vec<Option<(usize, i32)>> = (lo..=hi)
            .map(|i| {
                if i < 0 || i as usize >= n || sum == 0.0 {
                    return None;
                }
                let v = k(i) * 1024.0 / sum;
                let w = if v < 0.0 {
                    -((-v + 0.5).floor())
                } else {
                    (v + 0.5).floor()
                } as i32;
                (w != 0).then_some((i as usize, w))
            })
            .collect();
        let total: i32 = list.iter().flatten().map(|e| e.1).sum();
        if total != 1024 && !list.is_empty() {
            let mid = list.len() >> 1;
            let slot = if list[mid].is_some() {
                Some(mid)
            } else {
                list.iter().position(|e| e.is_some_and(|e| e.1 > 31))
            };
            if let Some(sl) = slot
                && let Some(e) = &mut list[sl]
            {
                e.1 += 1024 - total;
            }
        }
        out.push(list.into_iter().flatten().filter(|e| e.1 != 0).collect());
        c += step;
    }
    out
}

/// One target pixel from weighted source pixels (re 35 §3.2.3, integer arithmetic).
#[inline]
fn weigh_pixel<'a>(taps: impl Iterator<Item = (&'a Px, i32)>) -> Px {
    let (mut sa, mut sc) = (0i64, 0i64);
    let mut s = [0i64; 3];
    for (p, w) in taps {
        let aw = p[3] as i64 * w as i64;
        sa += aw;
        let t = aw + 128;
        let cw = ((t >> 8) + t) >> 8;
        for k in 0..3 {
            s[k] += p[k] as i64 * cw;
        }
        sc += cw;
    }
    if sc < 1 {
        return [255, 255, 255, 0];
    }
    let inv = ((0x20_0000i64 / sc) + 1) >> 1;
    let col = |v: i64| ((((inv * v) >> 19) + 1) >> 1).clamp(0, 255) as u8;
    [
        col(s[0]),
        col(s[1]),
        col(s[2]),
        (sa >> 10).clamp(0, 255) as u8,
    ]
}

/// Scales `src` separably to `width × height` at (`left`, `top`) – first horizontally, then
/// vertically, 8-bit intermediate (re 35 §3.2). `soft` = Mitchell kernel, else bicubic a = −1.
/// Same size = copy.
pub fn scale_separable(
    src: &RgbaImage,
    left: i32,
    top: i32,
    width: u32,
    height: u32,
    soft: bool,
) -> RgbaImage {
    let ob = IRect::from_xywh(
        left,
        top,
        width.min(i32::MAX as u32) as i32,
        height.min(i32::MAX as u32) as i32,
    );
    let (sw, sh) = (src.width(), src.height());
    if sw == width as usize && sh == height as usize {
        let mut out = src.clone();
        out.set_origin(left, top);
        return out;
    }
    let mut out = match RgbaImage::new(ob) {
        Ok(o) => o,
        Err(_) => return RgbaImage::empty_at(left, top),
    };
    if src.is_empty() || width == 0 || height == 0 {
        return out;
    }
    let kernel: fn(f64) -> f64 = if soft {
        kernel_mitchell
    } else {
        kernel_bicubic
    };
    let (dw, dh) = (width as usize, height as usize);
    let cx = scale_contributions(sw, dw, kernel);
    let cy = scale_contributions(sh, dh, kernel);
    let sdata = src.data();
    // horizontal pass: sh rows × dw columns
    let mut mid: Vec<Px> = vec![CLEAR; dw * sh];
    for_each_row(&mut mid, dw, |y, row| {
        let srow = &sdata[y * sw..(y + 1) * sw];
        for (x, p) in row.iter_mut().enumerate() {
            *p = weigh_pixel(cx[x].iter().map(|&(i, w)| (&srow[i], w)));
        }
    });
    let mid = &mid;
    for_each_row(out.data_mut(), dw, |y, row| {
        for (x, p) in row.iter_mut().enumerate() {
            *p = weigh_pixel(cy[y].iter().map(|&(j, w)| (&mid[j * dw + x], w)));
        }
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use fp_color::Rgba8;

    fn checker() -> RgbaImage {
        let mut img = RgbaImage::new(IRect::new(0, 0, 8, 8)).unwrap();
        for y in 0..8 {
            for x in 0..8 {
                let v = if (x + y) % 2 == 0 { 255 } else { 0 };
                img.set_pixel(x, y, Rgba8::rgb(v, v, v));
            }
        }
        img
    }

    #[test]
    fn identity_transform_is_exact() {
        let img = checker();
        for s in [Sampler::Nearest, Sampler::Bilinear, Sampler::Bicubic] {
            let out = transform(&img, &Matrix3::IDENTITY, s, None);
            assert_eq!(out.bounds(), img.bounds());
            assert_eq!(out, img, "{s:?}");
        }
        let moved = transform(
            &img,
            &Matrix3::translation(3.0, -2.0),
            Sampler::Nearest,
            None,
        );
        assert_eq!(moved.bounds(), IRect::new(3, -2, 11, 6));
        assert_eq!(moved.pixel(3, -2), img.pixel(0, 0));
    }

    #[test]
    fn nearest_half_scale() {
        let img = checker();
        let half = transform(&img, &Matrix3::scaling(0.5, 0.5), Sampler::Nearest, None);
        assert_eq!(half.bounds(), IRect::new(0, 0, 4, 4));
        let r = resize(&img, 16, 16, Sampler::Nearest);
        assert_eq!(r.pixel(1, 1), img.pixel(0, 0));
        assert_eq!(r.pixel(2, 0), img.pixel(1, 0));
    }

    #[test]
    fn bilinear_midpoint() {
        let mut img = RgbaImage::new(IRect::new(0, 0, 2, 1)).unwrap();
        img.set_pixel(0, 0, Rgba8::rgb(0, 0, 0));
        img.set_pixel(1, 0, Rgba8::rgb(255, 255, 255));
        let p = sample(&img, 1.0, 0.5, Sampler::Bilinear, Edge::Clamp(img.bounds()));
        assert!((126..=129).contains(&p[0]), "{p:?}");
        // transparenter Nachbar verfärbt nicht
        let mut t = RgbaImage::new(IRect::new(0, 0, 2, 1)).unwrap();
        t.set_pixel(0, 0, Rgba8::rgb(200, 10, 10));
        let q = sample(&t, 1.0, 0.5, Sampler::Bilinear, Edge::Clamp(t.bounds()));
        assert_eq!(&q[..3], &[200, 10, 10]);
        assert!((126..=129).contains(&q[3]));
    }

    #[test]
    fn nine_slice_axis_map() {
        // 20 → 60, Ränder 5: Ecken 1:1, Mittelzone 5..14 an den Ecken ausgerichtet
        assert_eq!(nine_slice_axis(0, 60, 20, 5, 5), (0.0, 0, 4));
        assert_eq!(nine_slice_axis(5, 60, 20, 5, 5), (5.0, 5, 14));
        assert_eq!(nine_slice_axis(54, 60, 20, 5, 5).0, 14.0);
        assert_eq!(nine_slice_axis(59, 60, 20, 5, 5), (19.0, 15, 19));
        let img = checker();
        let s = nine_slice_scale(&img, [2, 2, 2, 2], 8, 8, Sampler::Bicubic);
        assert_eq!(s, img);
    }

    #[test]
    fn pref_mapping() {
        assert_eq!(Sampler::from_pref(1), Sampler::Bicubic);
        assert_eq!(Sampler::from_pref(4), Sampler::Nearest);
        assert_eq!(Sampler::from_pref(2).flags(), 0x140);
        assert_eq!(Sampler::Soft.to_pref(), 3);
    }

    /// re 35 §3.4: weights of 480 → 240 (prefilter, R = 4) and 100 → 250 (enlarging).
    #[test]
    fn separable_weights_like_original() {
        let c = scale_contributions(480, 240, kernel_bicubic);
        assert_eq!(c[0], [(0, 682), (1, 427), (3, -85)]);
        assert_eq!(
            c[139],
            [
                (275, -18),
                (276, -75),
                (277, 125),
                (278, 438),
                (279, 472),
                (280, 180),
                (281, -67),
                (282, -31)
            ]
        );
        assert_eq!(
            c[140],
            [
                (277, -17),
                (278, -75),
                (279, 123),
                (280, 437),
                (281, 473),
                (282, 181),
                (283, -67),
                (284, -31)
            ]
        );
        assert_eq!(c[239], [(476, -85), (478, 427), (479, 682)]);
        let e = scale_contributions(100, 250, kernel_bicubic);
        assert_eq!(e[1], [(0, 668), (1, 441), (2, -85)]);
        // same size, bicubic: identity (re 35 §3.2.4)
        let id = scale_contributions(7, 7, kernel_bicubic);
        assert!(
            id.iter().enumerate().all(|(i, v)| v == &[(i, 1024)]),
            "{id:?}"
        );
    }

    /// Opaque pixels with a single full weight stay unchanged; empty sum → transparent white.
    #[test]
    fn separable_pixel_rule() {
        let p: Px = [20, 200, 7, 255];
        assert_eq!(weigh_pixel([(&p, 1024)].into_iter()), p);
        let q: Px = [9, 9, 9, 0];
        assert_eq!(weigh_pixel([(&q, 1024)].into_iter()), [255, 255, 255, 0]);
        let mut img = RgbaImage::new(IRect::new(0, 0, 4, 2)).unwrap();
        img.data_mut()
            .iter_mut()
            .for_each(|x| *x = [10, 20, 30, 255]);
        let out = scale_separable(&img, 3, 4, 2, 2, false);
        assert_eq!(out.origin(), (3, 4));
        assert!(out.data().iter().all(|x| *x == [10, 20, 30, 255]));
    }
}

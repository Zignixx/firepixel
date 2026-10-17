//! [`Mask`]: 8-Bit-Alphamaske mit Lage in Dokumentkoordinaten, dazu die Operationen der
//! Pixelauswahl (Menü „Auswählen“) und der Live-Effekte.
//!
//! Außerhalb ihrer Grenzen ist eine Maske 0. Grundbausteine (05 §2.2, 07 §5.6, 06 §11.2):
//!
//! - **Dreiecksfilter** (`@00834ba0`, horizontal `@0082f6e0`, vertikal `@00830070`): separierbar,
//!   Gewichte `1, 2, …, r+1, …, 2, 1`, Summe `(r+1)²`, nach jedem Durchgang Ganzzahldivision mit
//!   Abschneiden auf 8 Bit, Nullrand, das Ergebnis wächst um `r` je Seite → [`Mask::tent_blur`].
//!   Eigenheit des senkrechten Durchgangs: die **letzte Zeile** jeder Spalte wird als
//!   `screen(v, v) = 255 − mul(255−v, 255−v)` geschrieben (bei weicher Auswahl- und Füllkante
//!   immer, B Laufzeit 06 T7 / 07 T6).
//! - **Weiche Kante** („Auswählen › Weiche Kante…“) = Dreiecksfilter mit dieser Eigenheit →
//!   [`Mask::feather`]. Die gefilterte Bitmap wird dann durch diese Maske über das Original
//!   gemischt (06 T7: 10.000/10.000).
//! - **Morphologie** (`@00835190`): Dreiecksfilter und Schwelle mit der Voreinstellung
//!   `SelectionAdjustTolerance` (Standard 1) → [`Mask::expand`], [`Mask::contract`],
//!   [`Mask::border`], [`Mask::smooth`] („Auswahlrahmen erweitern/verkleinern/begrenzen/glätten“,
//!   Dialogbereich 1…100, 11 §877).

use fp_geom::IRect;

use crate::RasterError;
use crate::image::{checked_dims, norm_rect};

/// Standardwert der Voreinstellung `SelectionAdjustTolerance` (05 §2.2, 14).
pub const SELECTION_ADJUST_TOLERANCE: u8 = 1;

/// Dialogbereich für Erweitern/Verkleinern/Begrenzen/Glätten (11, OK klemmt auf 1…100).
pub const ADJUST_RANGE: (u32, u32) = (1, 100);

/// Größter Radius des Dreiecksfilters (05 §2.2).
pub const MAX_TENT_RADIUS: u32 = 1024;

/// Markierung „weichgezeichnet“ einer Maske (Maskenfeld `+0x20`, 05 §2.2). Sie steuert die
/// Nachbehandlung der Morphologie: weiche Eingaben werden nach der Schwelle erneut weichgezeichnet.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum SoftEdge {
    /// Harte Maske (frische Alphamasken, Konstruktor `@0082fbc0`).
    #[default]
    Hard,
    /// Markierung 1: Nachbehandlung mit Radius 1 (A: geglättete Kante).
    One,
    /// Markierung 2: mit [`Mask::tent_blur`] weichgezeichnet, die Radien sind gespeichert.
    Radii(u32, u32),
}

/// Morphologie-Modus von `@00835190` (05 §2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MorphMode {
    /// 0: Ausdehnen, `r = max(1, ⌊1,2·Betrag⌋)`, `v > t → 255`.
    Expand,
    /// 1: Zusammenziehen, `r = max(1, Betrag)`, `v < 255 − t → 0`, sonst 255.
    Contract,
    /// 2: Rand, `r = max(1, ⌊0,6·Betrag⌋)`, `t ≤ v ≤ 255 − t → 255`, danach erneut weich.
    Border,
    /// 3: Glätten, `r = max(1, ⌊1,6·Betrag⌋)`, `v > 127 → 255`.
    Smooth,
    /// 4: Rand ohne Nachbehandlung (Inneres Glühen).
    BorderRaw,
    /// 5: Fläche, kein Filter, `v > t → 255`.
    Area,
}

impl MorphMode {
    /// Radius des Dreiecksfilters für den Betrag.
    pub fn radius(self, amount: u32) -> u32 {
        let a = amount as f64;
        match self {
            MorphMode::Expand => ((a * 1.2) as u32).max(1),
            MorphMode::Contract => amount.max(1),
            MorphMode::Border | MorphMode::BorderRaw => ((a * 0.6) as u32).max(1),
            MorphMode::Smooth => ((a * 1.6) as u32).max(1),
            MorphMode::Area => 0,
        }
    }
}

/// 8-Bit-Maske (Abdeckung/Auswahl) mit Grenzen in Dokumentkoordinaten; außerhalb 0.
///
/// **Stabil**: `new`, `filled`, `from_rect`, `from_raw`, `empty_at`, `bounds`, `width`,
/// `height`, `data(_mut)`, `row_local`, `get`, `set` ändern sich nur noch rückwärtskompatibel.
/// Alias: [`AlphaMask`].
#[derive(Clone, PartialEq, Eq, Default)]
pub struct Mask {
    bounds: IRect,
    data: Vec<u8>,
    soft: SoftEdge,
}

impl std::fmt::Debug for Mask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mask")
            .field("bounds", &self.bounds)
            .field("soft", &self.soft)
            .finish_non_exhaustive()
    }
}

/// Anderer Name für [`Mask`].
pub type AlphaMask = Mask;

#[inline]
fn screen_self(v: u32) -> u8 {
    let t = (255 - v) * (255 - v) + 0x80;
    (255 - (((t >> 8) + t) >> 8)) as u8
}

impl Mask {
    /// Neue Maske, überall `value`.
    pub fn filled(bounds: IRect, value: u8) -> Result<Self, RasterError> {
        let bounds = norm_rect(bounds);
        let (w, h) = checked_dims(bounds)?;
        Ok(Self {
            bounds,
            data: vec![value; w * h],
            soft: SoftEdge::Hard,
        })
    }

    /// Neue Maske, überall 0.
    pub fn new(bounds: IRect) -> Result<Self, RasterError> {
        Self::filled(bounds, 0)
    }

    /// Rechteck-Auswahl: 255 in `rect`, Grenzen = `rect`.
    pub fn from_rect(rect: IRect) -> Result<Self, RasterError> {
        Self::filled(rect, 255)
    }

    /// Leere Maske an (`x`, `y`).
    pub fn empty_at(x: i32, y: i32) -> Self {
        Self {
            bounds: IRect::new(x, y, x, y),
            data: Vec::new(),
            soft: SoftEdge::Hard,
        }
    }

    /// Aus Rohdaten (Zeilen von oben nach unten).
    pub fn from_raw(bounds: IRect, data: Vec<u8>) -> Result<Self, RasterError> {
        let bounds = norm_rect(bounds);
        let (w, h) = checked_dims(bounds)?;
        if data.len() != w * h {
            return Err(RasterError::SizeMismatch {
                expected: w * h,
                got: data.len(),
            });
        }
        Ok(Self {
            bounds,
            data,
            soft: SoftEdge::Hard,
        })
    }

    /// Grenzen des Puffers.
    pub fn bounds(&self) -> IRect {
        self.bounds
    }

    /// Breite in Pixeln.
    pub fn width(&self) -> usize {
        self.bounds.width() as usize
    }

    /// Höhe in Pixeln.
    pub fn height(&self) -> usize {
        self.bounds.height() as usize
    }

    /// Rohdaten.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Rohdaten, veränderbar.
    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Markierung „weichgezeichnet“.
    pub fn soft_edge(&self) -> SoftEdge {
        self.soft
    }

    /// Setzt die Markierung „weichgezeichnet“.
    pub fn set_soft_edge(&mut self, s: SoftEdge) {
        self.soft = s;
    }

    /// Zeile mit lokalem Index `i`.
    pub fn row_local(&self, i: usize) -> &[u8] {
        let w = self.width();
        &self.data[i * w..(i + 1) * w]
    }

    /// Wert an Dokumentkoordinate (`x`, `y`); außerhalb 0.
    #[inline]
    pub fn get(&self, x: i32, y: i32) -> u8 {
        if self.bounds.contains(x, y) {
            self.data
                [(y - self.bounds.top) as usize * self.width() + (x - self.bounds.left) as usize]
        } else {
            0
        }
    }

    /// Setzt einen Wert; `false` außerhalb.
    pub fn set(&mut self, x: i32, y: i32, v: u8) -> bool {
        if self.bounds.contains(x, y) {
            let w = self.width();
            self.data[(y - self.bounds.top) as usize * w + (x - self.bounds.left) as usize] = v;
            true
        } else {
            false
        }
    }

    /// Kleinstes Rechteck der Werte > 0 (leer, wenn alles 0 ist).
    pub fn tight_bounds(&self) -> IRect {
        let w = self.width();
        let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0usize, 0usize);
        for y in 0..self.height() {
            let row = &self.data[y * w..(y + 1) * w];
            if let Some(f) = row.iter().position(|&v| v != 0) {
                let l = row.iter().rposition(|&v| v != 0).unwrap_or(f);
                x0 = x0.min(f);
                x1 = x1.max(l + 1);
                y0 = y0.min(y);
                y1 = y + 1;
            }
        }
        if x0 == usize::MAX {
            return IRect::new(
                self.bounds.left,
                self.bounds.top,
                self.bounds.left,
                self.bounds.top,
            );
        }
        let (l, t) = (self.bounds.left, self.bounds.top);
        IRect::new(l + x0 as i32, t + y0 as i32, l + x1 as i32, t + y1 as i32)
    }

    /// Kopie auf das Rechteck `rect` (außerhalb der alten Grenzen 0).
    pub fn copy_rect(&self, rect: IRect) -> Result<Self, RasterError> {
        let mut out = Self::new(rect)?;
        out.soft = self.soft;
        let r = out.bounds.intersect(&self.bounds);
        if !r.is_empty() {
            let n = r.width() as usize;
            for y in r.top..r.bottom {
                let si = (y - self.bounds.top) as usize * self.width()
                    + (r.left - self.bounds.left) as usize;
                let di = (y - out.bounds.top) as usize * out.width()
                    + (r.left - out.bounds.left) as usize;
                out.data[di..di + n].copy_from_slice(&self.data[si..si + n]);
            }
        }
        Ok(out)
    }

    /// Wendet `f` auf jeden Wert an.
    pub fn map(&mut self, f: impl Fn(u8) -> u8) {
        for v in &mut self.data {
            *v = f(*v);
        }
    }

    /// Auswahl umkehren innerhalb von `within` (z. B. Leinwand): `255 − v`.
    pub fn inverted(&self, within: IRect) -> Result<Self, RasterError> {
        let mut out = self.copy_rect(within)?;
        out.map(|v| 255 - v);
        Ok(out)
    }

    fn combine(
        &self,
        o: &Mask,
        f: impl Fn(u8, u8) -> u8,
        union_bounds: bool,
    ) -> Result<Self, RasterError> {
        let b = if union_bounds {
            self.bounds.union(&o.bounds)
        } else {
            self.bounds
        };
        let mut out = Self::new(b)?;
        let w = out.width();
        for y in 0..out.height() {
            for x in 0..w {
                let (dx, dy) = (b.left + x as i32, b.top + y as i32);
                out.data[y * w + x] = f(self.get(dx, dy), o.get(dx, dy));
            }
        }
        Ok(out)
    }

    /// Vereinigung (Maximum, „hinzufügen“).
    pub fn union(&self, o: &Mask) -> Result<Self, RasterError> {
        self.combine(o, |a, b| a.max(b), true)
    }

    /// Schnittmenge (Minimum).
    pub fn intersection(&self, o: &Mask) -> Result<Self, RasterError> {
        self.combine(o, |a, b| a.min(b), false)
    }

    /// Abziehen: `max(a − b, 0)` (Sättigungs-Subtraktion `@00830960`).
    pub fn subtract(&self, o: &Mask) -> Result<Self, RasterError> {
        self.combine(o, |a, b| a.saturating_sub(b), false)
    }

    /// Multiplikation `round(a·b/255)` (`@008306a0`).
    pub fn multiply(&self, o: &Mask) -> Result<Self, RasterError> {
        self.combine(
            o,
            |a, b| fp_color::round::mul255(a as u32, b as u32) as u8,
            false,
        )
    }

    /// Dreiecksfilter mit den Radien `rx` (waagerecht) und `ry` (senkrecht), siehe Moduldoku.
    /// Die Radien werden auf `0…1024` und auf die Maskengröße begrenzt (A: je Achse).
    /// `last_row_screen` schaltet die Eigenheit der letzten Zeile ein.
    pub fn tent_blur(&self, rx: u32, ry: u32, last_row_screen: bool) -> Result<Self, RasterError> {
        let rx = rx.min(MAX_TENT_RADIUS).min(self.width() as u32) as usize;
        let ry = ry.min(MAX_TENT_RADIUS).min(self.height() as u32) as usize;
        let (w, h) = (self.width(), self.height());
        let out_b = self.bounds.inflate(rx as i32, ry as i32);
        checked_dims(out_b)?;
        let (w2, h2) = (w + 2 * rx, h + 2 * ry);
        // waagerecht: h Zeilen × w2
        let mut tmp = vec![0u8; w2 * h];
        let denx = ((rx + 1) * (rx + 1)) as u32;
        let mut line = vec![0u32; w];
        for y in 0..h {
            for (d, &s) in line.iter_mut().zip(&self.data[y * w..(y + 1) * w]) {
                *d = s as u32;
            }
            tent_1d(&line, rx, |j, v| tmp[y * w2 + j] = (v / denx) as u8);
        }
        // senkrecht: Spalten von tmp
        let mut data = vec![0u8; w2 * h2];
        let deny = ((ry + 1) * (ry + 1)) as u32;
        let mut col = vec![0u32; h];
        for x in 0..w2 {
            for (y, c) in col.iter_mut().enumerate() {
                *c = tmp[y * w2 + x] as u32;
            }
            tent_1d(&col, ry, |j, v| data[j * w2 + x] = (v / deny) as u8);
        }
        if last_row_screen && h2 > 0 {
            for v in &mut data[(h2 - 1) * w2..] {
                *v = screen_self(*v as u32);
            }
        }
        let soft = if rx == 0 && ry == 0 {
            self.soft
        } else {
            SoftEdge::Radii(rx as u32, ry as u32)
        };
        Ok(Self {
            bounds: out_b,
            data,
            soft,
        })
    }

    /// „Weiche Kante“ mit Radius `r` (Dreiecksfilter mit Eigenheit der letzten Zeile).
    pub fn feather(&self, r: u32) -> Result<Self, RasterError> {
        self.tent_blur(r, r, true)
    }

    /// Morphologie `@00835190` mit Betrag `amount` und Schwelle `tol` (siehe [`MorphMode`]).
    /// Das Ergebnis ist um den Filterradius größer als die Eingabe.
    pub fn morph(&self, amount: u32, mode: MorphMode, tol: u8) -> Result<Self, RasterError> {
        let r = mode.radius(amount);
        let t = tol as u32;
        // re 27/28 L7: auch in der Morphologie (alle Modi, auch die Nachbehandlung) schreibt der
        // senkrechte Durchgang in der letzten Zeile screen(v, v) wie beim Weichzeichnen (Rand 5:
        // letzte Zeile 29 statt 15; erweitern 10 unten 1 px höher)
        let mut m = if r > 0 {
            self.tent_blur(r, r, true)?
        } else {
            self.clone()
        };
        match mode {
            MorphMode::Expand | MorphMode::Area => m.map(|v| if v as u32 > t { 255 } else { 0 }),
            MorphMode::Contract => m.map(|v| {
                if (v as u32) < 255u32.saturating_sub(t) {
                    0
                } else {
                    255
                }
            }),
            MorphMode::Border | MorphMode::BorderRaw => m.map(|v| {
                if (v as u32) > 255u32.saturating_sub(t) || (v as u32) < t {
                    0
                } else {
                    255
                }
            }),
            MorphMode::Smooth => m.map(|v| if v > 127 { 255 } else { 0 }),
        }
        m.soft = SoftEdge::Hard;
        match mode {
            MorphMode::Border => m = m.tent_blur(r.max(1), r.max(1), true)?,
            MorphMode::Expand | MorphMode::Contract | MorphMode::Smooth => match self.soft {
                SoftEdge::Hard => {}
                SoftEdge::One => m = m.tent_blur(1, 1, true)?,
                SoftEdge::Radii(a, b) => m = m.tent_blur(a, b, true)?,
            },
            MorphMode::BorderRaw | MorphMode::Area => {}
        }
        Ok(m)
    }

    /// „Auswahlrahmen erweitern“ um `px` Pixel (Modus 0).
    pub fn expand(&self, px: u32, tol: u8) -> Result<Self, RasterError> {
        self.morph(px, MorphMode::Expand, tol)
    }

    /// „Auswahlrahmen verkleinern“ um `px` Pixel (Modus 1).
    pub fn contract(&self, px: u32, tol: u8) -> Result<Self, RasterError> {
        self.morph(px, MorphMode::Contract, tol)
    }

    /// „Auswahlrahmen begrenzen“ (Rand) mit Breite `px` (Modus 2).
    pub fn border(&self, px: u32, tol: u8) -> Result<Self, RasterError> {
        self.morph(px, MorphMode::Border, tol)
    }

    /// „Auswahlrahmen glätten“ mit Stichprobenradius `px` (Modus 3).
    pub fn smooth(&self, px: u32, tol: u8) -> Result<Self, RasterError> {
        self.morph(px, MorphMode::Smooth, tol)
    }
}

/// Eindimensionaler Dreiecksfilter als zwei laufende Kastensummen der Breite `r+1`
/// (`min(k+1, 2r+1−k)` = Faltung zweier Kästen). Ruft `out(j, summe)` für `j = 0 … n+2r−1`
/// auf; Ausgabe `j` entspricht Eingabeposition `j − r`. Ganzzahlig exakt.
fn tent_1d(src: &[u32], r: usize, mut out: impl FnMut(usize, u32)) {
    let n = src.len();
    if r == 0 {
        for (j, &v) in src.iter().enumerate() {
            out(j, v);
        }
        return;
    }
    let get = |i: isize| -> u32 {
        if i >= 0 && (i as usize) < n {
            src[i as usize]
        } else {
            0
        }
    };
    // Kasten 1: b1[i] = Σ src[i−r ..= i], i = 0 … n+r−1
    let n1 = n + r;
    let mut b1 = vec![0u32; n1];
    let mut s = 0u32;
    for (i, b) in b1.iter_mut().enumerate() {
        s += get(i as isize);
        s -= get(i as isize - r as isize - 1);
        *b = s;
    }
    // Kasten 2 über b1: out[j] = Σ b1[j−r ..= j], j = 0 … n+2r−1
    let g1 = |i: isize| -> u32 {
        if i >= 0 && (i as usize) < n1 {
            b1[i as usize]
        } else {
            0
        }
    };
    let mut s2 = 0u32;
    for j in 0..n + 2 * r {
        s2 += g1(j as isize);
        s2 -= g1(j as isize - r as isize - 1);
        out(j, s2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tent_naive(src: &[u32], r: usize) -> Vec<u32> {
        let n = src.len();
        (0..n + 2 * r)
            .map(|j| {
                (0..=2 * r)
                    .map(|k| {
                        let w = (k + 1).min(2 * r + 1 - k) as u32;
                        let i = j as isize + k as isize - 2 * r as isize;
                        if i >= 0 && (i as usize) < n {
                            w * src[i as usize]
                        } else {
                            0
                        }
                    })
                    .sum()
            })
            .collect()
    }

    #[test]
    fn tent_running_sum_matches_naive() {
        let src: Vec<u32> = (0..23).map(|i| (i * 37 % 256) as u32).collect();
        for r in 0..8 {
            let mut got = vec![0; src.len() + 2 * r];
            tent_1d(&src, r, |j, v| got[j] = v);
            assert_eq!(got, tent_naive(&src, r), "r={r}");
        }
    }

    #[test]
    fn feather_rect() {
        let m = Mask::from_rect(IRect::new(10, 10, 20, 20)).unwrap();
        let f = m.feather(3).unwrap();
        assert_eq!(f.bounds(), IRect::new(7, 7, 23, 23));
        assert_eq!(f.get(15, 15), 255);
        assert_eq!(f.soft_edge(), SoftEdge::Radii(3, 3));
        // erste Zeile: a0/(r+1)² mehrfach abgeschnitten → 0
        assert_eq!(f.get(7, 7), 0);
        // letzte Zeile: screen(v, v) ≥ v
        let plain = m.tent_blur(3, 3, false).unwrap();
        for x in 7..23 {
            let (a, b) = (plain.get(x, 22), f.get(x, 22));
            assert_eq!(b, screen_self(a as u32));
            assert!(b >= a);
        }
    }

    #[test]
    fn morphology() {
        let m = Mask::from_rect(IRect::new(0, 0, 20, 20)).unwrap();
        let e = m.expand(5, 1).unwrap();
        let tb = e.tight_bounds();
        assert!(tb.left < 0 && tb.right > 20);
        let c = m.contract(5, 1).unwrap();
        let tc = c.tight_bounds();
        assert!(tc.left > 0 && tc.right < 20, "{tc:?}");
        let inv = m.inverted(IRect::new(-5, -5, 25, 25)).unwrap();
        assert_eq!(inv.get(-5, -5), 255);
        assert_eq!(inv.get(5, 5), 0);
        let b = m.border(5, 1).unwrap();
        assert_eq!(b.get(10, 10), 0);
        assert!(b.get(0, 10) > 0);
    }
}

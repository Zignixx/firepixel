//! [`RgbaImage`]: zusammenhängender RGBA8-Pixelpuffer mit Lage in Dokumentkoordinaten.
//!
//! Das Original rechnet Bitmaps und Effekt-Caches als nicht vormultipliziertes 32-Bit-Pixel mit
//! eigenem Rechteck in Dokument-Pixelkoordinaten (06 §2.3, §12). Genau so ist [`RgbaImage`]
//! aufgebaut: Kanalreihenfolge R, G, B, A je Pixel (`[u8; 4]`), Zeilen von oben nach unten,
//! dazu ein [`IRect`] (links/oben inklusiv, rechts/unten exklusiv), das auch negativ sein darf.
//!
//! Die Farbe voll transparenter Pixel bleibt erhalten; mehrere Filter rechnen damit (06 §12
//! „Fallstricke“).

use fp_color::Rgba8;
use fp_geom::IRect;

use crate::mask::Mask;
use crate::par::for_each_row;
use crate::{MAX_PIXELS, RasterError};

/// Ein Pixel als `[r, g, b, a]` (nicht vormultipliziert).
pub type Px = [u8; 4];

/// Voll transparentes Schwarz `(0, 0, 0, 0)`, die Füllung neuer Puffer und Erweiterungen.
pub const CLEAR: Px = [0, 0, 0, 0];

/// RGBA8-Bild mit Lage in Dokumentkoordinaten.
///
/// **Stabil**: Konstruktoren (`new`, `filled`, `from_raw`, `from_rgba_bytes`, `empty_at`),
/// Größe/Lage (`bounds`, `origin`, `width`, `height`, `set_origin`), Pixelzugriff (`get`,
/// `pixel`, `set_pixel`, `row`, `row_local(_mut)`, `data(_mut)`, `as_bytes(_mut)`) ändern sich nur
/// noch rückwärtskompatibel.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct RgbaImage {
    bounds: IRect,
    data: Vec<Px>,
}

impl std::fmt::Debug for RgbaImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RgbaImage")
            .field("bounds", &self.bounds)
            .finish_non_exhaustive()
    }
}

/// Prüft die Größe eines Rechtecks und liefert (Breite, Höhe) als `usize`.
pub(crate) fn checked_dims(bounds: IRect) -> Result<(usize, usize), RasterError> {
    let (w, h) = (bounds.width() as usize, bounds.height() as usize);
    match w.checked_mul(h) {
        Some(n) if n <= MAX_PIXELS => Ok((w, h)),
        _ => Err(RasterError::TooLarge {
            width: bounds.width(),
            height: bounds.height(),
        }),
    }
}

/// Normiert leere Rechtecke auf Breite/Höhe 0 an ihrer linken oberen Ecke.
pub(crate) fn norm_rect(r: IRect) -> IRect {
    if r.is_empty() {
        IRect::new(r.left, r.top, r.left, r.top)
    } else {
        r
    }
}

impl RgbaImage {
    /// Neues, voll transparentes Bild `(0,0,0,0)` mit diesen Grenzen.
    pub fn new(bounds: IRect) -> Result<Self, RasterError> {
        Self::filled(bounds, Rgba8::TRANSPARENT)
    }

    /// Neues Bild, gefüllt mit `px`.
    pub fn filled(bounds: IRect, px: Rgba8) -> Result<Self, RasterError> {
        let bounds = norm_rect(bounds);
        let (w, h) = checked_dims(bounds)?;
        Ok(Self {
            bounds,
            data: vec![px.to_array(); w * h],
        })
    }

    /// Leeres Bild (0×0) an der Position (`x`, `y`).
    pub fn empty_at(x: i32, y: i32) -> Self {
        Self {
            bounds: IRect::new(x, y, x, y),
            data: Vec::new(),
        }
    }

    /// Aus Rohdaten (Zeilen von oben nach unten, `[r, g, b, a]` je Pixel).
    pub fn from_raw(bounds: IRect, data: Vec<Px>) -> Result<Self, RasterError> {
        let bounds = norm_rect(bounds);
        let (w, h) = checked_dims(bounds)?;
        if data.len() != w * h {
            return Err(RasterError::SizeMismatch {
                expected: w * h,
                got: data.len(),
            });
        }
        Ok(Self { bounds, data })
    }

    /// Aus einem Byte-Puffer RGBA (4 Byte je Pixel).
    pub fn from_rgba_bytes(bounds: IRect, bytes: &[u8]) -> Result<Self, RasterError> {
        let bounds = norm_rect(bounds);
        let (w, h) = checked_dims(bounds)?;
        if bytes.len() != w * h * 4 {
            return Err(RasterError::SizeMismatch {
                expected: w * h * 4,
                got: bytes.len(),
            });
        }
        let data = bytes.as_chunks::<4>().0.to_vec();
        Ok(Self { bounds, data })
    }

    /// Aus Byte-Daten ARGB in der Speicherform des Originals (B, G, R, A je Pixel, `0xAARRGGBB`).
    pub fn from_bgra_bytes(bounds: IRect, bytes: &[u8]) -> Result<Self, RasterError> {
        let mut img = Self::from_rgba_bytes(bounds, bytes)?;
        for p in &mut img.data {
            p.swap(0, 2);
        }
        Ok(img)
    }

    /// Grenzen in Dokumentkoordinaten.
    #[inline]
    pub fn bounds(&self) -> IRect {
        self.bounds
    }

    /// Linke obere Ecke in Dokumentkoordinaten.
    #[inline]
    pub fn origin(&self) -> (i32, i32) {
        (self.bounds.left, self.bounds.top)
    }

    /// Breite in Pixeln.
    #[inline]
    pub fn width(&self) -> usize {
        self.bounds.width() as usize
    }

    /// Höhe in Pixeln.
    #[inline]
    pub fn height(&self) -> usize {
        self.bounds.height() as usize
    }

    /// Kein Pixel?
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Verschiebt das Bild an eine neue linke obere Ecke (Pixel bleiben).
    pub fn set_origin(&mut self, x: i32, y: i32) {
        let (w, h) = (self.bounds.width() as i32, self.bounds.height() as i32);
        self.bounds = IRect::from_xywh(x, y, w, h);
    }

    /// Kopie, um (`dx`, `dy`) verschoben.
    pub fn translated(&self, dx: i32, dy: i32) -> Self {
        let mut c = self.clone();
        c.bounds = self.bounds.translate(dx, dy);
        c
    }

    /// Alle Pixel, zeilenweise.
    #[inline]
    pub fn data(&self) -> &[Px] {
        &self.data
    }

    /// Alle Pixel, veränderbar.
    #[inline]
    pub fn data_mut(&mut self) -> &mut [Px] {
        &mut self.data
    }

    /// Pixel als Bytes RGBA (ohne Kopie, z. B. für Texturen oder PNG).
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.data.as_flattened()
    }

    /// Pixel als veränderbare Bytes RGBA.
    #[inline]
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        self.data.as_flattened_mut()
    }

    /// Zerlegt das Bild in Grenzen und Rohdaten.
    pub fn into_raw(self) -> (IRect, Vec<Px>) {
        (self.bounds, self.data)
    }

    /// Zeile mit lokalem Index `i` (0 = oberste Zeile).
    #[inline]
    pub fn row_local(&self, i: usize) -> &[Px] {
        let w = self.width();
        &self.data[i * w..(i + 1) * w]
    }

    /// Veränderbare Zeile mit lokalem Index `i`.
    #[inline]
    pub fn row_local_mut(&mut self, i: usize) -> &mut [Px] {
        let w = self.width();
        &mut self.data[i * w..(i + 1) * w]
    }

    /// Zeile in Dokumentkoordinate `y` (`None` außerhalb).
    pub fn row(&self, y: i32) -> Option<&[Px]> {
        if y < self.bounds.top || y >= self.bounds.bottom || self.is_empty() {
            return None;
        }
        Some(self.row_local((y - self.bounds.top) as usize))
    }

    #[inline]
    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if self.bounds.contains(x, y) {
            Some((y - self.bounds.top) as usize * self.width() + (x - self.bounds.left) as usize)
        } else {
            None
        }
    }

    /// Pixel an Dokumentkoordinate (`x`, `y`); außerhalb `None`.
    #[inline]
    pub fn get(&self, x: i32, y: i32) -> Option<Px> {
        self.index(x, y).map(|i| self.data[i])
    }

    /// Pixel an Dokumentkoordinate (`x`, `y`); außerhalb transparent.
    #[inline]
    pub fn pixel(&self, x: i32, y: i32) -> Rgba8 {
        Rgba8::from_array(self.get(x, y).unwrap_or(CLEAR))
    }

    /// Setzt ein Pixel; `false`, wenn (`x`, `y`) außerhalb liegt.
    #[inline]
    pub fn set_pixel(&mut self, x: i32, y: i32, px: Rgba8) -> bool {
        match self.index(x, y) {
            Some(i) => {
                self.data[i] = px.to_array();
                true
            }
            None => false,
        }
    }

    /// Kopie des Rechtecks `rect` (Dokumentkoordinaten); Pixel außerhalb des Bildes sind
    /// transparent `(0,0,0,0)`. Damit lassen sich Ausschnitte bilden **und** Bilder erweitern.
    pub fn copy_rect(&self, rect: IRect) -> Result<Self, RasterError> {
        let mut out = Self::new(rect)?;
        out.paste(self);
        Ok(out)
    }

    /// Erweitert das Bild transparent, sodass es `rect` enthält (Vereinigung der Grenzen).
    pub fn extended_to(&self, rect: IRect) -> Result<Self, RasterError> {
        if rect.is_empty() {
            return Ok(self.clone());
        }
        let target = if self.is_empty() {
            rect
        } else {
            self.bounds.union(&rect)
        };
        if target == self.bounds {
            return Ok(self.clone());
        }
        self.copy_rect(target)
    }

    /// Vergrößert (positiv) bzw. beschneidet (negativ) je Seite.
    pub fn expanded(
        &self,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    ) -> Result<Self, RasterError> {
        let b = self.bounds;
        self.copy_rect(IRect::new(
            b.left.saturating_sub(left),
            b.top.saturating_sub(top),
            b.right.saturating_add(right),
            b.bottom.saturating_add(bottom),
        ))
    }

    /// Beschneidet auf `rect ∩ Grenzen`.
    pub fn cropped(&self, rect: IRect) -> Self {
        let r = self.bounds.intersect(&rect);
        if r.is_empty() {
            return Self::empty_at(self.bounds.left, self.bounds.top);
        }
        // Größe ist durch das eigene Bild begrenzt, kann also nicht fehlschlagen.
        self.copy_rect(r)
            .unwrap_or_else(|_| Self::empty_at(r.left, r.top))
    }

    /// Kopiert die Pixel von `src` an deren Lage hinein (ersetzt, ohne Mischen); nur der
    /// überlappende Teil wird geschrieben.
    pub fn paste(&mut self, src: &RgbaImage) {
        let r = self.bounds.intersect(&src.bounds);
        if r.is_empty() {
            return;
        }
        let n = r.width() as usize;
        for y in r.top..r.bottom {
            let si =
                (y - src.bounds.top) as usize * src.width() + (r.left - src.bounds.left) as usize;
            let di = (y - self.bounds.top) as usize * self.width()
                + (r.left - self.bounds.left) as usize;
            self.data[di..di + n].copy_from_slice(&src.data[si..si + n]);
        }
    }

    /// Füllt `rect ∩ Grenzen` mit `px`.
    pub fn fill_rect(&mut self, rect: IRect, px: Rgba8) {
        let r = self.bounds.intersect(&rect);
        if r.is_empty() {
            return;
        }
        let p = px.to_array();
        for y in r.top..r.bottom {
            let di = (y - self.bounds.top) as usize * self.width()
                + (r.left - self.bounds.left) as usize;
            self.data[di..di + r.width() as usize].fill(p);
        }
    }

    /// Kleinstes Rechteck der Pixel mit Alpha > 0 (leer, wenn es keine gibt).
    ///
    /// Gespeicherte Render-Caches des Originals sind genau darauf zugeschnitten (06 §2.4, 05 §2.6).
    pub fn alpha_bounds(&self) -> IRect {
        self.alpha_bounds_min(1)
    }

    /// Kleinstes Rechteck der Pixel mit Alpha ≥ `min_alpha` (leer, wenn es keine gibt). Das
    /// Original beschneidet geöffnete und importierte Bilder mit `min_alpha = 2` (re\22 §1.2).
    pub fn alpha_bounds_min(&self, min_alpha: u8) -> IRect {
        let min_alpha = min_alpha.max(1);
        let (w, h) = (self.width(), self.height());
        let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0usize, 0usize);
        for y in 0..h {
            let row = self.row_local(y);
            if let Some(first) = row.iter().position(|p| p[3] >= min_alpha) {
                let last = row.iter().rposition(|p| p[3] >= min_alpha).unwrap_or(first);
                x0 = x0.min(first);
                x1 = x1.max(last + 1);
                y0 = y0.min(y);
                y1 = y + 1;
            }
        }
        if x0 == usize::MAX || w == 0 {
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

    /// Auf die Pixel mit Alpha > 0 zugeschnittene Kopie (wie gespeicherte Caches).
    pub fn trimmed(&self) -> Self {
        let r = self.alpha_bounds();
        if r.is_empty() {
            return Self::empty_at(r.left, r.top);
        }
        self.cropped(r)
    }

    /// Wendet `f` auf jedes Pixel an (bei großen Bildern parallel).
    pub fn map_pixels<F>(&mut self, f: F)
    where
        F: Fn(Px) -> Px + Sync + Send,
    {
        let w = self.width();
        for_each_row(&mut self.data, w, |_, row| {
            for p in row.iter_mut() {
                *p = f(*p);
            }
        });
    }

    /// Vormultiplizierte Kopie (Original-Rundung `mul255`, fp-color).
    pub fn premultiplied(&self) -> Self {
        let mut c = self.clone();
        c.map_pixels(|p| Rgba8::from_array(p).premultiply().to_array());
        c
    }

    /// Vormultiplikation rückgängig machen (gerundet; Alpha 0 → `(0,0,0,0)`).
    pub fn unpremultiplied(&self) -> Self {
        let mut c = self.clone();
        c.map_pixels(|p| Rgba8::from_array(p).unpremultiply().to_array());
        c
    }

    /// Alphakanal als 8-Bit-Maske mit denselben Grenzen.
    pub fn alpha_mask(&self) -> Mask {
        let data: Vec<u8> = self.data.iter().map(|p| p[3]).collect();
        // Gleiche Größe wie das Bild, also immer gültig.
        Mask::from_raw(self.bounds, data)
            .unwrap_or_else(|_| Mask::empty_at(self.bounds.left, self.bounds.top))
    }

    /// Setzt alle Pixel mit Alpha 0 auf deckendes Schwarz (`RemoveTransparencyForMask`,
    /// 06 §8: vor dem Filtern von Bitmap-Masken).
    pub fn remove_transparency(&mut self) {
        self.map_pixels(|p| if p[3] == 0 { [0, 0, 0, 255] } else { p });
    }

    /// Vergleich wie in den Prüfskripten von 06 §9: Anzahl gleicher Pixel (alle vier Kanäle
    /// gleich oder beide Alpha 0) und größte Kanalabweichung über `rect`. Pixel außerhalb eines
    /// Bildes gelten als `(0,0,0,0)`.
    pub fn compare(&self, other: &RgbaImage, rect: IRect) -> CompareResult {
        let mut res = CompareResult::default();
        for y in rect.top..rect.bottom {
            for x in rect.left..rect.right {
                let a = self.get(x, y).unwrap_or(CLEAR);
                let b = other.get(x, y).unwrap_or(CLEAR);
                res.total += 1;
                if a[3] == 0 && b[3] == 0 {
                    res.equal += 1;
                    res.strict += (a == b) as u64;
                    continue;
                }
                let d = (0..4).map(|i| a[i].abs_diff(b[i])).max().unwrap_or(0);
                if d == 0 {
                    res.equal += 1;
                    res.strict += 1;
                }
                res.max_diff = res.max_diff.max(d);
            }
        }
        res
    }
}

/// Ergebnis von [`RgbaImage::compare`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CompareResult {
    /// Pixel gleich (alle Kanäle gleich oder beide transparent).
    pub equal: u64,
    /// Pixel in allen vier Kanälen gleich (auch die Farbe transparenter Pixel).
    pub strict: u64,
    /// Verglichene Pixel.
    pub total: u64,
    /// Größte Kanalabweichung (transparente Paare zählen nicht).
    pub max_diff: u8,
}

impl std::fmt::Display for CompareResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}/{} pixelgenau, max. Abw. {}",
            self.equal, self.total, self.max_diff
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construct_and_access() {
        let mut img = RgbaImage::new(IRect::new(-3, 2, 2, 5)).unwrap();
        assert_eq!((img.width(), img.height()), (5, 3));
        assert!(img.set_pixel(-3, 2, Rgba8::rgb(1, 2, 3)));
        assert!(!img.set_pixel(2, 2, Rgba8::rgb(1, 2, 3)));
        assert_eq!(img.pixel(-3, 2), Rgba8::rgb(1, 2, 3));
        assert_eq!(img.pixel(100, 100), Rgba8::TRANSPARENT);
        assert_eq!(img.as_bytes().len(), 60);
        assert!(RgbaImage::from_raw(IRect::new(0, 0, 2, 2), vec![CLEAR; 3]).is_err());
        assert!(RgbaImage::new(IRect::new(0, 0, 1 << 20, 1 << 20)).is_err());
        let e = RgbaImage::new(IRect::new(5, 5, 1, 1)).unwrap();
        assert!(e.is_empty());
    }

    #[test]
    fn copy_extend_trim() {
        let mut img = RgbaImage::new(IRect::new(0, 0, 4, 4)).unwrap();
        img.fill_rect(IRect::new(1, 1, 3, 2), Rgba8::rgb(9, 9, 9));
        assert_eq!(img.alpha_bounds(), IRect::new(1, 1, 3, 2));
        let t = img.trimmed();
        assert_eq!(t.bounds(), IRect::new(1, 1, 3, 2));
        let ext = t.expanded(2, 1, 0, 3).unwrap();
        assert_eq!(ext.bounds(), IRect::new(-1, 0, 3, 5));
        assert_eq!(ext.pixel(1, 1), Rgba8::rgb(9, 9, 9));
        assert_eq!(ext.pixel(-1, 0), Rgba8::TRANSPARENT);
        let c = ext.cropped(IRect::new(2, 1, 10, 10));
        assert_eq!(c.bounds(), IRect::new(2, 1, 3, 5));
        let moved = c.translated(-2, -1);
        assert_eq!(moved.bounds(), IRect::new(0, 0, 1, 4));
    }

    #[test]
    fn premultiply_roundtrip() {
        let img = RgbaImage::filled(IRect::new(0, 0, 2, 1), Rgba8::new(255, 128, 0, 128)).unwrap();
        let p = img.premultiplied();
        assert_eq!(p.pixel(0, 0), Rgba8::new(128, 64, 0, 128));
        assert_eq!(
            p.unpremultiplied().pixel(1, 0),
            Rgba8::new(255, 128, 0, 128)
        );
    }
}

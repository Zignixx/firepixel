//! Pixeldaten: Kacheln (`mkBT`), Bitmaps (`IMGv`) und 8-Bit-Masken (`MSKv`).
//!
//! Eine Bitmap besteht aus einem Raster von Kacheln mit Kantenlänge `TSZi` (128 bei Bildern, 256 bei
//! Texturmasken), zeilenweise in `TILv` (02 §5.1). Jeder Eintrag ist entweder eine Kachel aus einem
//! `mkBT`-Chunk (`TIDi`) oder eine einfarbige Kachel (`TMCi` ARGB bzw. `TMMi` Maskenwert).
//! Bildkacheln sind 128×128 × ARGB (nicht vormultipliziert, Zeilenlänge immer 512 Byte), Maskenkacheln
//! 256×256 × 8 Bit; beide entpackt 65.536 Byte.

use crate::raw::Extra;
use crate::serde_ext;
use fp_color::{Argb, Rgba8};
use fp_geom::Point;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Art einer Kachel (Kopf des `mkBT`-Chunks, Offset 8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TileKind {
    /// 0: Bildkachel 128×128 ARGB.
    Image,
    /// 1: Texturmasken-Kachel 256×256 × 8 Bit.
    Mask,
    /// Andere Werte (im Korpus nicht vorhanden) bleiben erhalten.
    Other(u32),
}

impl TileKind {
    pub fn from_code(v: u32) -> Self {
        match v {
            0 => TileKind::Image,
            1 => TileKind::Mask,
            o => TileKind::Other(o),
        }
    }
    pub fn code(self) -> u32 {
        match self {
            TileKind::Image => 0,
            TileKind::Mask => 1,
            TileKind::Other(o) => o,
        }
    }
}

/// Eine Kachel aus einem `mkBT`-Chunk. Wird über `Arc` geteilt, weil eine Kachel mehrfach referenziert
/// sein kann (z. B. die gemeinsame „Grain“-Textur).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tile {
    /// Kachel-ID aus der Datei (`TIDi`). Ein Schreiber darf neu nummerieren.
    pub id: u32,
    pub kind: TileKind,
    /// Entpackte Daten (immer 65.536 Byte): ARGB-Zeilen bzw. Maskenbytes.
    #[serde(skip)]
    pub pixels: Arc<[u8]>,
    /// Originaler zlib-Strom aus der Datei (ohne Kopf). Gesetzt, solange `pixels` unverändert ist;
    /// ein Schreiber übernimmt ihn dann unverändert.
    #[serde(skip)]
    pub compressed: Option<Arc<[u8]>>,
    /// Bytes 12..76 des Kopfs, falls sie nicht alle 0 waren (Rundreise).
    #[serde(skip)]
    pub reserved: Option<Arc<[u8]>>,
}

/// Bytes einer entpackten Kachel.
pub const TILE_BYTES: usize = 65_536;

/// Obergrenze für zusammengesetzte Bitmaps (Schutz vor unsinnigen Größen aus beschädigten Dateien).
pub const MAX_PIXELS: usize = 1 << 28;

impl Tile {
    /// Neue Bildkachel aus 128×128 ARGB-Bytes.
    pub fn new_image(id: u32, argb: Vec<u8>) -> Tile {
        Tile {
            id,
            kind: TileKind::Image,
            pixels: argb.into(),
            compressed: None,
            reserved: None,
        }
    }
}

/// Ein Eintrag der Kachelliste `TILv`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TileEntry {
    /// `TIDi`: Kachel aus `mkBT`.
    Tile(Arc<Tile>),
    /// `TIDi` mit einer ID, zu der die Datei keine Kachel enthält (verlustfrei behalten).
    Missing(u32),
    /// `TMCi`: einfarbige Bildkachel.
    Solid(#[serde(with = "serde_ext::argb")] Argb),
    /// `TMMi`: einfarbige Maskenkachel (8-Bit-Wert, als 32-Bit-Zahl gespeichert).
    SolidMask(u32),
}

/// Kachelraster einer Bitmap bzw. Maske.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TileGrid {
    /// Breite in Pixeln (`WPXi`).
    pub width: u32,
    /// Höhe in Pixeln (`HPXi`).
    pub height: u32,
    /// Kachelkante (`TSZi`): 128 bei Bildern, 256 bei Texturmasken.
    pub tile_size: u32,
    /// `ceil(W/T)·ceil(H/T)` Einträge, zeilenweise.
    pub tiles: Vec<TileEntry>,
    /// Rundreise-Daten des `TILv`-Containers.
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub tiles_extra: Extra,
}

impl TileGrid {
    /// Anzahl Kachelspalten.
    pub fn columns(&self) -> u32 {
        if self.tile_size == 0 {
            0
        } else {
            self.width.div_ceil(self.tile_size)
        }
    }
    /// Anzahl Kachelzeilen.
    pub fn rows(&self) -> u32 {
        if self.tile_size == 0 {
            0
        } else {
            self.height.div_ceil(self.tile_size)
        }
    }

    /// Setzt eine Bildbitmap zu RGBA8 (nicht vormultipliziert) zusammen. Fehlende Kacheln bleiben
    /// transparent; das Ergebnis meldet in `.1`, ob Kacheln fehlten.
    pub fn to_rgba(&self) -> (Vec<u8>, bool) {
        let (w, h, ts) = (
            self.width as usize,
            self.height as usize,
            self.tile_size as usize,
        );
        if w.saturating_mul(h) > MAX_PIXELS {
            return (Vec::new(), true);
        }
        let mut out = vec![0u8; w * h * 4];
        let mut missing = false;
        if ts == 0 || w == 0 || h == 0 {
            return (out, false);
        }
        let cols = w.div_ceil(ts);
        for (k, e) in self.tiles.iter().enumerate() {
            let (ty, tx) = (k / cols, k % cols);
            let (x0, y0) = (tx * ts, ty * ts);
            if x0 >= w || y0 >= h {
                continue;
            }
            let cw = ts.min(w - x0);
            let ch = ts.min(h - y0);
            match e {
                TileEntry::Solid(c) => {
                    let px = [c.r(), c.g(), c.b(), c.a()];
                    for yy in 0..ch {
                        let o = ((y0 + yy) * w + x0) * 4;
                        for xx in 0..cw {
                            out[o + xx * 4..o + xx * 4 + 4].copy_from_slice(&px);
                        }
                    }
                }
                TileEntry::Tile(t)
                    if t.kind == TileKind::Image && t.pixels.len() >= ts * ts * 4 =>
                {
                    for yy in 0..ch {
                        let src = &t.pixels[yy * ts * 4..yy * ts * 4 + cw * 4];
                        let o = ((y0 + yy) * w + x0) * 4;
                        for xx in 0..cw {
                            let s = &src[xx * 4..xx * 4 + 4];
                            out[o + xx * 4] = s[1];
                            out[o + xx * 4 + 1] = s[2];
                            out[o + xx * 4 + 2] = s[3];
                            out[o + xx * 4 + 3] = s[0];
                        }
                    }
                }
                _ => missing = true,
            }
        }
        (out, missing)
    }

    /// Setzt eine 8-Bit-Maske zusammen (Maskenkacheln und `TMMi`).
    pub fn to_gray(&self) -> (Vec<u8>, bool) {
        let (w, h, ts) = (
            self.width as usize,
            self.height as usize,
            self.tile_size as usize,
        );
        if w.saturating_mul(h) > MAX_PIXELS {
            return (Vec::new(), true);
        }
        let mut out = vec![0u8; w * h];
        let mut missing = false;
        if ts == 0 || w == 0 || h == 0 {
            return (out, false);
        }
        let cols = w.div_ceil(ts);
        for (k, e) in self.tiles.iter().enumerate() {
            let (ty, tx) = (k / cols, k % cols);
            let (x0, y0) = (tx * ts, ty * ts);
            if x0 >= w || y0 >= h {
                continue;
            }
            let cw = ts.min(w - x0);
            let ch = ts.min(h - y0);
            match e {
                TileEntry::SolidMask(v) => {
                    for yy in 0..ch {
                        let o = (y0 + yy) * w + x0;
                        out[o..o + cw].fill(*v as u8);
                    }
                }
                TileEntry::Tile(t) if t.pixels.len() >= ts * ts => {
                    for yy in 0..ch {
                        let o = (y0 + yy) * w + x0;
                        out[o..o + cw].copy_from_slice(&t.pixels[yy * ts..yy * ts + cw]);
                    }
                }
                _ => missing = true,
            }
        }
        (out, missing)
    }

    /// Baut ein Kachelraster (128er Kacheln) aus RGBA8-Pixeln. Einfarbige Kacheln werden als `TMCi`
    /// abgelegt; neue Kacheln bekommen IDs aus `next_id` (ein Schreiber nummeriert ohnehin neu).
    pub fn from_rgba(width: u32, height: u32, rgba: &[u8], next_id: &mut u32) -> TileGrid {
        let ts = 128usize;
        let (w, h) = (width as usize, height as usize);
        let cols = w.div_ceil(ts);
        let rows = h.div_ceil(ts);
        let mut tiles = Vec::with_capacity(cols * rows);
        for ty in 0..rows {
            for tx in 0..cols {
                let (x0, y0) = (tx * ts, ty * ts);
                let cw = ts.min(w - x0);
                let ch = ts.min(h - y0);
                let mut buf = vec![0u8; TILE_BYTES];
                // Voll transparente Füllpixel wie das Original mit RGB = FF FF FF.
                for px in buf.as_chunks_mut::<4>().0 {
                    px.copy_from_slice(&[0, 255, 255, 255]);
                }
                let first = {
                    let o = (y0 * w + x0) * 4;
                    [
                        rgba.get(o + 3).copied().unwrap_or(0),
                        rgba.get(o).copied().unwrap_or(0),
                        rgba.get(o + 1).copied().unwrap_or(0),
                        rgba.get(o + 2).copied().unwrap_or(0),
                    ]
                };
                let mut uniform = cw == ts && ch == ts;
                for yy in 0..ch {
                    for xx in 0..cw {
                        let o = ((y0 + yy) * w + x0 + xx) * 4;
                        let s = rgba.get(o..o + 4).unwrap_or(&[0, 0, 0, 0]);
                        let d = (yy * ts + xx) * 4;
                        buf[d] = s[3];
                        buf[d + 1] = s[0];
                        buf[d + 2] = s[1];
                        buf[d + 3] = s[2];
                        if buf[d..d + 4] != first {
                            uniform = false;
                        }
                    }
                }
                if uniform {
                    let c = Argb(u32::from_be_bytes(first));
                    tiles.push(TileEntry::Solid(c));
                } else {
                    *next_id = next_id.wrapping_add(1);
                    tiles.push(TileEntry::Tile(Arc::new(Tile::new_image(*next_id, buf))));
                }
            }
        }
        TileGrid {
            width,
            height,
            tile_size: ts as u32,
            tiles,
            tiles_extra: Extra::default(),
        }
    }
}

/// Kleinste Deckkraft, ab der ein Bitmap-Pixel als sichtbar zählt (re\22 §1.2: Alpha 1 zählt wie 0).
pub const VISIBLE_ALPHA_MIN: u8 = 2;

/// Zwischenspeicher der sichtbaren Rechtecke je Bildkachel. Schlüssel: Adresse der Kachel und die
/// genutzte Breite/Höhe (Randkacheln). Der `Weak` hält die Adresse belegt, solange der Eintrag lebt
/// (keine Verwechslung mit einer neuen Kachel an derselben Stelle); Kachelpixel werden nie an Ort
/// und Stelle geändert, jede Bearbeitung erzeugt neue Kacheln.
type VisCache =
    std::collections::HashMap<(usize, u32, u32), (std::sync::Weak<Tile>, Option<[u32; 4]>)>;
static VIS_CACHE: std::sync::Mutex<Option<VisCache>> = std::sync::Mutex::new(None);
const VIS_CACHE_PRUNE: usize = 16_384;

/// Sichtbares Rechteck einer ARGB-Kachel (Alpha in Byte 0), lokal `[x0, y0, x1, y1)`.
fn tile_visible(px: &[u8], ts: usize, cw: usize, ch: usize) -> Option<[u32; 4]> {
    let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0usize, 0usize);
    for y in 0..ch {
        let row = &px[y * ts * 4..y * ts * 4 + cw * 4];
        let vis = |p: &[u8; 4]| p[0] >= VISIBLE_ALPHA_MIN;
        let (c, _) = row.as_chunks::<4>();
        let Some(first) = c.iter().position(vis) else {
            continue;
        };
        let last = c.iter().rposition(vis).unwrap_or(first);
        x0 = x0.min(first);
        x1 = x1.max(last + 1);
        y0 = y0.min(y);
        y1 = y + 1;
    }
    (x0 != usize::MAX).then_some([x0 as u32, y0 as u32, x1 as u32, y1 as u32])
}

impl TileGrid {
    /// Rechteck der Pixel mit Alpha ≥ [`VISIBLE_ALPHA_MIN`] in Bitmap-Pixeln `[x0, y0, x1, y1)`
    /// (`None`, wenn es keine gibt). Nur für Bildbitmaps; fehlende Kacheln zählen als transparent.
    pub fn visible_bounds(&self) -> Option<[u32; 4]> {
        let (w, h, ts) = (self.width, self.height, self.tile_size);
        if ts == 0 || w == 0 || h == 0 {
            return None;
        }
        let cols = w.div_ceil(ts);
        let mut acc: Option<[u32; 4]> = None;
        let mut add = |r: [u32; 4]| {
            acc = Some(match acc {
                None => r,
                Some(a) => [
                    a[0].min(r[0]),
                    a[1].min(r[1]),
                    a[2].max(r[2]),
                    a[3].max(r[3]),
                ],
            });
        };
        let mut guard = VIS_CACHE.lock().unwrap_or_else(|e| e.into_inner());
        let cache = guard.get_or_insert_with(Default::default);
        for (k, e) in self.tiles.iter().enumerate() {
            let k = k as u32;
            let (tx, ty) = (k % cols, k / cols);
            let (ox, oy) = (tx * ts, ty * ts);
            if ox >= w || oy >= h {
                continue;
            }
            let (cw, ch) = (ts.min(w - ox), ts.min(h - oy));
            let local = match e {
                TileEntry::Solid(c) => (c.a() >= VISIBLE_ALPHA_MIN).then_some([0, 0, cw, ch]),
                TileEntry::Tile(t)
                    if t.kind == TileKind::Image && t.pixels.len() >= (ts * ts * 4) as usize =>
                {
                    let key = (Arc::as_ptr(t) as usize, cw, ch);
                    match cache
                        .get(&key)
                        .filter(|(wk, _)| wk.upgrade().is_some_and(|a| Arc::ptr_eq(&a, t)))
                    {
                        Some((_, r)) => *r,
                        None => {
                            let r = tile_visible(&t.pixels, ts as usize, cw as usize, ch as usize);
                            if cache.len() >= VIS_CACHE_PRUNE {
                                cache.retain(|_, (wk, _)| wk.strong_count() > 0);
                                if cache.len() >= VIS_CACHE_PRUNE / 2 {
                                    cache.clear();
                                }
                            }
                            cache.insert(key, (Arc::downgrade(t), r));
                            r
                        }
                    }
                }
                _ => None,
            };
            if let Some([a, b, c, d]) = local {
                add([ox + a, oy + b, ox + c, oy + d]);
            }
        }
        acc
    }
}

/// Lage und Pixel einer Bitmap (`IMGv`-Inhalt): `XLCf YLCf XOFf YOFf WPXi HPXi TSZi TILv`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImageData {
    /// Linke obere Ecke in Dokumentkoordinaten (`XLCf`, `YLCf`).
    #[serde(with = "serde_ext::point")]
    pub origin: Point,
    /// Versatz `XOFf`/`YOFf`: Das Original addiert ihn beim Lesen zur Ecke; CS6 schreibt immer 0 (02 §12.10).
    #[serde(with = "serde_ext::point")]
    pub offset: Point,
    pub grid: TileGrid,
}

impl ImageData {
    /// Wirksame linke obere Ecke (Ursprung + Versatz).
    pub fn position(&self) -> Point {
        Point::new(self.origin.x + self.offset.x, self.origin.y + self.offset.y)
    }

    /// The same bitmap with every colour channel inverted (`255 − v`, alpha kept). A grayscale
    /// bitmap mask (`GRTi` 1) is stored inverted in the file (07 §7.5: value = α·(255 − grey));
    /// Firepixel keeps it in display form (white = visible) and converts with this at file I/O
    /// (RE spec 68 §9.2 M-6). Inflated image tiles become new tiles without the file's zlib
    /// stream; tiles that were never inflated (lazy reads) and missing tiles stay as they are.
    pub fn inverted_rgb(&self) -> ImageData {
        let mut out = self.clone();
        for e in &mut out.grid.tiles {
            match e {
                TileEntry::Tile(t) if t.kind == TileKind::Image && !t.pixels.is_empty() => {
                    let mut px = t.pixels.to_vec();
                    for c in px.as_chunks_mut::<4>().0 {
                        c[1] = 255 - c[1];
                        c[2] = 255 - c[2];
                        c[3] = 255 - c[3];
                    }
                    *t = Arc::new(Tile {
                        id: t.id,
                        kind: t.kind,
                        pixels: px.into(),
                        compressed: None,
                        reserved: t.reserved.clone(),
                    });
                }
                TileEntry::Solid(c) => *c = Argb(c.0 ^ 0x00ff_ffff),
                _ => {}
            }
        }
        out
    }

    /// Bitmap als RGBA8 zusammensetzen.
    pub fn to_rgba(&self) -> (Vec<u8>, bool) {
        self.grid.to_rgba()
    }

    /// Rechteck der sichtbaren Pixel (Alpha ≥ [`VISIBLE_ALPHA_MIN`]) in Dokumentkoordinaten, `None`
    /// bei einer leeren Bitmap. Das Original zeichnet daran den Auswahlrahmen und meldet es als
    /// `pixelRect`, unabhängig von den gespeicherten Objektgrenzen (re\22 §1.2 Regel 3).
    /// Je Kachel zwischengespeichert: Ändern sich Pixel, entsteht eine neue Kachel.
    pub fn visible_bounds(&self) -> Option<fp_geom::Rect> {
        let [x0, y0, x1, y1] = self.grid.visible_bounds()?;
        let p = self.position();
        Some(fp_geom::Rect::new(
            p.x + x0 as f32,
            p.y + y0 as f32,
            p.x + x1 as f32,
            p.y + y1 as f32,
        ))
    }

    /// Ein Pixel lesen (ohne Zusammensetzen der ganzen Bitmap).
    pub fn pixel(&self, x: u32, y: u32) -> Option<Rgba8> {
        let g = &self.grid;
        if x >= g.width || y >= g.height || g.tile_size == 0 {
            return None;
        }
        let ts = g.tile_size;
        let k = (y / ts) * g.columns() + x / ts;
        match g.tiles.get(k as usize)? {
            TileEntry::Solid(c) => Some(c.to_rgba8()),
            TileEntry::Tile(t) => {
                let o = (((y % ts) * ts + x % ts) * 4) as usize;
                let s = t.pixels.get(o..o + 4)?;
                Some(Rgba8::new(s[1], s[2], s[3], s[0]))
            }
            _ => None,
        }
    }
}

/// 8-Bit-Maske (`MSKv{WPXi HPXi TSZi TILv}`), z. B. Texturen und gespeicherte Auswahlen.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaskData {
    pub grid: TileGrid,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgba_roundtrip_via_tiles() {
        let (w, h) = (130u32, 5u32);
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for (i, px) in rgba.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            px.copy_from_slice(&[(i % 251) as u8, 7, 9, 255]);
        }
        let mut id = 100;
        let g = TileGrid::from_rgba(w, h, &rgba, &mut id);
        assert_eq!(g.tiles.len(), 2);
        let (back, missing) = g.to_rgba();
        assert!(!missing);
        assert_eq!(back, rgba);
    }

    #[test]
    fn visible_bounds_threshold_and_tiles() {
        // 300×200 (3×2 Kacheln), Alpha 1 bei (5,5) zählt nicht, Alpha 2 bei (250,150) und ein Block
        let (w, h) = (300u32, 200u32);
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        fn set(rgba: &mut [u8], w: u32, x: u32, y: u32, a: u8) {
            let o = ((y * w + x) * 4) as usize;
            rgba[o..o + 4].copy_from_slice(&[200, 10, 10, a]);
        }
        set(&mut rgba, w, 5, 5, 1);
        for y in 40..60 {
            for x in 100..140 {
                set(&mut rgba, w, x, y, 255);
            }
        }
        let mut id = 0;
        let g = TileGrid::from_rgba(w, h, &rgba, &mut id);
        assert_eq!(g.visible_bounds(), Some([100, 40, 140, 60]));
        // zweiter Aufruf aus dem Zwischenspeicher, gleiches Ergebnis
        assert_eq!(g.visible_bounds(), Some([100, 40, 140, 60]));
        set(&mut rgba, w, 250, 150, 2);
        let g2 = TileGrid::from_rgba(w, h, &rgba, &mut id);
        assert_eq!(g2.visible_bounds(), Some([100, 40, 251, 151]));
        let img = ImageData {
            origin: Point::new(10.0, -3.0),
            offset: Point::default(),
            grid: g2,
        };
        assert_eq!(
            img.visible_bounds(),
            Some(fp_geom::Rect::new(110.0, 37.0, 261.0, 148.0))
        );
        let empty = TileGrid::from_rgba(w, h, &vec![0u8; (w * h * 4) as usize], &mut id);
        assert_eq!(empty.visible_bounds(), None);
        let solid = TileGrid {
            width: 10,
            height: 10,
            tile_size: 128,
            tiles: vec![TileEntry::Solid(Argb(0x01ff0000))],
            tiles_extra: Extra::default(),
        };
        assert_eq!(solid.visible_bounds(), None);
        let solid = TileGrid {
            width: 10,
            height: 10,
            tile_size: 128,
            tiles: vec![TileEntry::Solid(Argb(0x02ff0000))],
            tiles_extra: Extra::default(),
        };
        assert_eq!(solid.visible_bounds(), Some([0, 0, 10, 10]));
    }

    #[test]
    fn solid_tiles() {
        let g = TileGrid {
            width: 200,
            height: 10,
            tile_size: 128,
            tiles: vec![TileEntry::Solid(Argb(0xff102030)), TileEntry::Missing(5)],
            tiles_extra: Extra::default(),
        };
        let (px, missing) = g.to_rgba();
        assert!(missing);
        assert_eq!(&px[0..4], &[0x10, 0x20, 0x30, 0xff]);
        let img = ImageData {
            origin: Point::new(1.0, 2.0),
            offset: Point::default(),
            grid: g,
        };
        assert_eq!(img.pixel(3, 3), Some(Rgba8::new(0x10, 0x20, 0x30, 0xff)));
        assert_eq!(img.pixel(150, 3), None);
    }
}

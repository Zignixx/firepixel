//! [`TiledImage`]: Bild aus 128×128-Kacheln mit Copy-on-Write für günstiges Rückgängig.
//!
//! Die Kachelgröße entspricht den Dateikacheln von `mkBT` (`TSZi` = 128, 02/04). Das Raster
//! beginnt an der linken oberen Ecke der Bildgrenzen. Jede Kachel liegt in einem [`Arc`]:
//! Ein [`Clone`] des Bildes kostet nur das Kopieren der Zeiger, und eine Änderung kopiert nur die
//! betroffene Kachel ([`Arc::make_mut`]). Ein Undo-Schritt hält einfach den alten
//! [`TiledImage`]; unveränderte Kacheln teilen sich beide Stände.
//!
//! Voll transparente Kacheln dürfen fehlen (`None`), sie belegen dann keinen Speicher.
//!
//! Achtung: Das interne Kachelmaß des Original-`CPixelBuffer` (Breite ≤ 499 → Bildbreite, sonst
//! 500; 06 §2.3) ist etwas anderes; es zählt nur für „Rauschen hinzufügen“ und steckt dort
//! ([`crate::filters::noise::pixelbuffer_tile`]).

use std::sync::Arc;

use fp_color::Rgba8;
use fp_geom::IRect;

use crate::RasterError;
use crate::image::{CLEAR, Px, RgbaImage, checked_dims, norm_rect};

/// Kantenlänge einer Kachel in Pixeln.
pub const TILE_SIZE: usize = 128;

/// Eine Kachel: `TILE_SIZE × TILE_SIZE` Pixel, Zeilen von oben nach unten. Randkacheln sind
/// ebenfalls voll groß; Pixel außerhalb der Bildgrenzen bleiben transparent.
#[derive(Clone, PartialEq, Eq)]
pub struct Tile {
    px: Box<[Px]>,
}

impl std::fmt::Debug for Tile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Tile")
    }
}

impl Tile {
    /// Neue, transparente Kachel.
    pub fn new() -> Self {
        Self {
            px: vec![CLEAR; TILE_SIZE * TILE_SIZE].into_boxed_slice(),
        }
    }

    /// Pixel an lokaler Position (0..128).
    #[inline]
    pub fn get(&self, x: usize, y: usize) -> Px {
        self.px[y * TILE_SIZE + x]
    }

    /// Setzt ein Pixel an lokaler Position (0..128).
    #[inline]
    pub fn set(&mut self, x: usize, y: usize, p: Px) {
        self.px[y * TILE_SIZE + x] = p;
    }

    /// Lokale Zeile `y`.
    #[inline]
    pub fn row(&self, y: usize) -> &[Px] {
        &self.px[y * TILE_SIZE..(y + 1) * TILE_SIZE]
    }

    /// Lokale Zeile `y`, veränderbar.
    #[inline]
    pub fn row_mut(&mut self, y: usize) -> &mut [Px] {
        &mut self.px[y * TILE_SIZE..(y + 1) * TILE_SIZE]
    }

    /// Alle Pixel transparent (Alpha 0, Farbe egal)?
    pub fn is_transparent(&self) -> bool {
        self.px.iter().all(|p| p[3] == 0)
    }
}

impl Default for Tile {
    fn default() -> Self {
        Self::new()
    }
}

/// Gekacheltes Bild mit Copy-on-Write-Kacheln.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TiledImage {
    bounds: IRect,
    cols: usize,
    rows: usize,
    tiles: Vec<Option<Arc<Tile>>>,
}

impl TiledImage {
    /// Neues, voll transparentes Bild (keine Kachel belegt).
    pub fn new(bounds: IRect) -> Result<Self, RasterError> {
        let bounds = norm_rect(bounds);
        let (w, h) = checked_dims(bounds)?;
        let cols = w.div_ceil(TILE_SIZE);
        let rows = h.div_ceil(TILE_SIZE);
        Ok(Self {
            bounds,
            cols,
            rows,
            tiles: vec![None; cols * rows],
        })
    }

    /// Aus einem zusammenhängenden Bild; transparente Kacheln werden nicht angelegt, wenn auch ihre
    /// Farbe 0 ist (sonst bleibt die Farbe transparenter Pixel erhalten).
    pub fn from_image(img: &RgbaImage) -> Self {
        let mut t = match Self::new(img.bounds()) {
            Ok(t) => t,
            Err(_) => return Self::default(),
        };
        t.write_image(img);
        t
    }

    /// Grenzen in Dokumentkoordinaten.
    pub fn bounds(&self) -> IRect {
        self.bounds
    }

    /// Kachelspalten und -zeilen.
    pub fn grid(&self) -> (usize, usize) {
        (self.cols, self.rows)
    }

    /// Rechteck der Kachel (`tx`, `ty`) in Dokumentkoordinaten (auf die Bildgrenzen beschnitten).
    pub fn tile_rect(&self, tx: usize, ty: usize) -> IRect {
        let l = self.bounds.left + (tx * TILE_SIZE) as i32;
        let t = self.bounds.top + (ty * TILE_SIZE) as i32;
        IRect::new(
            l,
            t,
            (l + TILE_SIZE as i32).min(self.bounds.right),
            (t + TILE_SIZE as i32).min(self.bounds.bottom),
        )
    }

    /// Kachel (`tx`, `ty`), `None` = transparent bzw. außerhalb.
    pub fn tile(&self, tx: usize, ty: usize) -> Option<&Arc<Tile>> {
        if tx >= self.cols || ty >= self.rows {
            return None;
        }
        self.tiles[ty * self.cols + tx].as_ref()
    }

    /// Setzt eine Kachel (z. B. beim Laden der Dateikacheln); `false` außerhalb des Rasters.
    pub fn set_tile(&mut self, tx: usize, ty: usize, tile: Option<Arc<Tile>>) -> bool {
        if tx >= self.cols || ty >= self.rows {
            return false;
        }
        self.tiles[ty * self.cols + tx] = tile;
        true
    }

    /// Veränderbare Kachel; legt sie an bzw. kopiert sie, wenn sie geteilt ist (Copy-on-Write).
    pub fn tile_mut(&mut self, tx: usize, ty: usize) -> Option<&mut Tile> {
        if tx >= self.cols || ty >= self.rows {
            return None;
        }
        let slot = &mut self.tiles[ty * self.cols + tx];
        let arc = slot.get_or_insert_with(|| Arc::new(Tile::new()));
        Some(Arc::make_mut(arc))
    }

    /// Teilen sich beide Bilder die Kachel (`tx`, `ty`) (gleicher Speicher oder beide leer)?
    pub fn shares_tile(&self, other: &TiledImage, tx: usize, ty: usize) -> bool {
        match (self.tile(tx, ty), other.tile(tx, ty)) {
            (None, None) => true,
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }

    /// Kacheln, die sich gegenüber `older` geändert haben (nach Zeiger), z. B. für Undo und
    /// gezieltes Neuzeichnen. Bei anderen Grenzen: alle Kacheln.
    pub fn changed_tiles(&self, older: &TiledImage) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        let same_grid = self.bounds == older.bounds;
        for ty in 0..self.rows {
            for tx in 0..self.cols {
                if !same_grid || !self.shares_tile(older, tx, ty) {
                    out.push((tx, ty));
                }
            }
        }
        out
    }

    /// Anzahl belegter Kacheln.
    pub fn allocated_tiles(&self) -> usize {
        self.tiles.iter().filter(|t| t.is_some()).count()
    }

    #[inline]
    fn locate(&self, x: i32, y: i32) -> Option<(usize, usize, usize, usize)> {
        if !self.bounds.contains(x, y) {
            return None;
        }
        let lx = (x - self.bounds.left) as usize;
        let ly = (y - self.bounds.top) as usize;
        Some((
            lx / TILE_SIZE,
            ly / TILE_SIZE,
            lx % TILE_SIZE,
            ly % TILE_SIZE,
        ))
    }

    /// Pixel an Dokumentkoordinate (`x`, `y`); außerhalb bzw. in leeren Kacheln transparent.
    pub fn pixel(&self, x: i32, y: i32) -> Rgba8 {
        match self.locate(x, y) {
            Some((tx, ty, lx, ly)) => match self.tile(tx, ty) {
                Some(t) => Rgba8::from_array(t.get(lx, ly)),
                None => Rgba8::TRANSPARENT,
            },
            None => Rgba8::TRANSPARENT,
        }
    }

    /// Setzt ein Pixel (kopiert die Kachel bei Bedarf); `false` außerhalb.
    pub fn set_pixel(&mut self, x: i32, y: i32, px: Rgba8) -> bool {
        match self.locate(x, y) {
            Some((tx, ty, lx, ly)) => match self.tile_mut(tx, ty) {
                Some(t) => {
                    t.set(lx, ly, px.to_array());
                    true
                }
                None => false,
            },
            None => false,
        }
    }

    /// Schreibt die Pixel von `img` (überlappender Teil) hinein. Nur berührte Kacheln werden
    /// kopiert; Kacheln, deren Inhalt sich nicht ändert, bleiben geteilt.
    pub fn write_image(&mut self, img: &RgbaImage) {
        let r = self.bounds.intersect(&img.bounds());
        if r.is_empty() {
            return;
        }
        for ty in 0..self.rows {
            for tx in 0..self.cols {
                let tr = self.tile_rect(tx, ty).intersect(&r);
                if tr.is_empty() {
                    continue;
                }
                let ox = self.bounds.left + (tx * TILE_SIZE) as i32;
                let oy = self.bounds.top + (ty * TILE_SIZE) as i32;
                // unverändert? Dann Kachel nicht anfassen.
                let unchanged = (tr.top..tr.bottom).all(|y| {
                    let src = &img.row(y).unwrap_or(&[])[(tr.left - img.bounds().left) as usize..]
                        [..tr.width() as usize];
                    match self.tile(tx, ty) {
                        Some(t) => {
                            let row = t.row((y - oy) as usize);
                            &row[(tr.left - ox) as usize..][..tr.width() as usize] == src
                        }
                        None => src.iter().all(|p| *p == CLEAR),
                    }
                });
                if unchanged {
                    continue;
                }
                if let Some(t) = self.tile_mut(tx, ty) {
                    for y in tr.top..tr.bottom {
                        let src = &img.row(y).unwrap_or(&[])
                            [(tr.left - img.bounds().left) as usize..]
                            [..tr.width() as usize];
                        let row = t.row_mut((y - oy) as usize);
                        row[(tr.left - ox) as usize..][..tr.width() as usize].copy_from_slice(src);
                    }
                }
            }
        }
    }

    /// Zusammenhängende Kopie des Rechtecks `rect` (außerhalb transparent).
    pub fn to_image_rect(&self, rect: IRect) -> Result<RgbaImage, RasterError> {
        let mut out = RgbaImage::new(rect)?;
        let r = self.bounds.intersect(&out.bounds());
        if r.is_empty() {
            return Ok(out);
        }
        let ob = out.bounds();
        let ow = out.width();
        let data = out.data_mut();
        for ty in 0..self.rows {
            for tx in 0..self.cols {
                let tr = self.tile_rect(tx, ty).intersect(&r);
                if tr.is_empty() {
                    continue;
                }
                let Some(t) = self.tile(tx, ty) else { continue };
                let ox = self.bounds.left + (tx * TILE_SIZE) as i32;
                let oy = self.bounds.top + (ty * TILE_SIZE) as i32;
                for y in tr.top..tr.bottom {
                    let src =
                        &t.row((y - oy) as usize)[(tr.left - ox) as usize..][..tr.width() as usize];
                    let di = (y - ob.top) as usize * ow + (tr.left - ob.left) as usize;
                    data[di..di + tr.width() as usize].copy_from_slice(src);
                }
            }
        }
        Ok(out)
    }

    /// Zusammenhängende Kopie des ganzen Bildes.
    pub fn to_image(&self) -> RgbaImage {
        self.to_image_rect(self.bounds)
            .unwrap_or_else(|_| RgbaImage::empty_at(self.bounds.left, self.bounds.top))
    }

    /// Gibt Kacheln frei, die vollständig `(0,0,0,0)` sind.
    pub fn compact(&mut self) {
        for t in &mut self.tiles {
            if t.as_ref().is_some_and(|k| k.px.iter().all(|p| *p == CLEAR)) {
                *t = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_on_write() {
        let mut img = RgbaImage::new(IRect::new(-10, -10, 290, 140)).unwrap();
        img.fill_rect(IRect::new(0, 0, 10, 10), Rgba8::rgb(1, 2, 3));
        let a = TiledImage::from_image(&img);
        assert_eq!(a.grid(), (3, 2));
        assert_eq!(a.allocated_tiles(), 1);
        let mut b = a.clone();
        assert!(b.shares_tile(&a, 0, 0));
        b.set_pixel(200, 5, Rgba8::rgb(7, 7, 7));
        assert!(b.shares_tile(&a, 0, 0));
        assert!(!b.shares_tile(&a, 1, 0));
        assert_eq!(b.changed_tiles(&a), vec![(1, 0)]);
        assert_eq!(a.pixel(200, 5), Rgba8::TRANSPARENT);
        assert_eq!(b.pixel(200, 5), Rgba8::rgb(7, 7, 7));
        // Rundreise
        assert_eq!(b.to_image().pixel(0, 0), Rgba8::rgb(1, 2, 3));
        assert_eq!(a.to_image(), img);
        // unveränderte Kacheln bleiben beim Schreiben geteilt
        let mut c = a.clone();
        c.write_image(&img);
        assert!(c.changed_tiles(&a).is_empty());
        let part = b.to_image_rect(IRect::new(195, 0, 205, 10)).unwrap();
        assert_eq!(part.pixel(200, 5), Rgba8::rgb(7, 7, 7));
    }
}

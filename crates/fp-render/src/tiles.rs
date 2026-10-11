//! Kachel-Cache der Seitenkomposition (Leistung bei großen Dokumenten).
//!
//! [`crate::Renderer::render_rect`] zeichnet die Seite zuerst als **Anzeigeliste** auf: je
//! Element der obersten Ebene ein Aussehen (`Arc<RgbaImage>` aus dem Element-Cache), Deckkraft und
//! Mischmodus. Gemischt wird dann in Kacheln von 256 × 256 Dokumentpixeln (an Vielfachen von 256
//! ausgerichtet). Eine Kachel wird wiederverwendet, wenn ihre **Signatur** gleich ist: Leinwandfarbe,
//! Unterlegfarbe und die Folge der Schritte, deren Bild die Kachel schneidet (Zeiger des Bildes,
//! Grenzen, Deckkraft, Mischmodus). Der Eintrag hält Klone der Bilder, so kann ein Zeiger nicht
//! für ein anderes Bild wiederverwendet werden. Weil das Mischen pixelweise geschieht, ist das
//! Ergebnis bitgleich zum direkten Mischen. Ausnahme „Auflösen“ (Zufallsfolge über die Zeilen):
//! dann wird die ganze Fläche direkt gemischt.
//!
//! Wirkung: Verschieben/Zoomen und erneutes Zeichnen kopieren nur Kacheln; nach einer Bearbeitung
//! werden nur die Kacheln neu gemischt, die das geänderte Element berührt (vorher und nachher).

use std::collections::HashMap;
use std::sync::Arc;

use fp_color::{BlendMode, Opacity, Rgba8};
use fp_color::blend::OverVariant;
use fp_geom::IRect;
use fp_raster::{composite, CompositeOptions, RgbaImage};
use rayon::prelude::*;

/// Kantenlänge einer Kachel in Dokumentpixeln.
pub const TILE: i32 = 256;

/// Ein aufgezeichneter Mischschritt.
#[derive(Clone, Debug)]
pub struct DrawOp {
    pub img: Arc<RgbaImage>,
    pub opacity: Opacity,
    pub blend: BlendMode,
}

fn opts(opacity: Opacity, blend: BlendMode) -> CompositeOptions {
    CompositeOptions { mode: blend, opacity, variant: OverVariant::Mmx, clip: None }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OpSig {
    ptr: usize,
    bounds: IRect,
    opacity: u8,
    blend: i32,
}

#[derive(Debug)]
struct Entry {
    fill: Rgba8,
    underlay: Option<Rgba8>,
    sig: Vec<OpSig>,
    _keep: Vec<Arc<RgbaImage>>,
    img: Arc<RgbaImage>,
    used: u64,
}

/// Kennzahlen des Kachel-Caches.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TileStats {
    pub tiles: usize,
    pub hits: u64,
    pub misses: u64,
}

/// Cache der gemischten Kacheln.
#[derive(Debug, Default)]
pub struct TileCache {
    map: HashMap<(i32, i32), Vec<Entry>>,
    count: usize,
    clock: u64,
    hits: u64,
    misses: u64,
}

fn intersects(a: IRect, b: IRect) -> bool {
    a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom
}

fn tile_rect(tx: i32, ty: i32) -> IRect {
    IRect::new(tx * TILE, ty * TILE, tx * TILE + TILE, ty * TILE + TILE)
}

/// Direkt mischen (ohne Kacheln).
pub fn mix_direct(area: IRect, fill: Rgba8, underlay: Option<Rgba8>, ops: &[DrawOp]) -> RgbaImage {
    mix(area, fill, underlay, &ops.iter().collect::<Vec<_>>())
}

fn mix(area: IRect, fill: Rgba8, underlay: Option<Rgba8>, ops: &[&DrawOp]) -> RgbaImage {
    let mut img = RgbaImage::filled(area, fill).unwrap_or_else(|_| RgbaImage::empty_at(area.left, area.top));
    for o in ops {
        composite(&mut img, &o.img, None, &opts(o.opacity, o.blend));
    }
    if let Some(bg) = underlay {
        let mut base = RgbaImage::filled(area, bg).unwrap_or_else(|_| RgbaImage::empty_at(area.left, area.top));
        composite(&mut base, &img, None, &opts(Opacity(255), BlendMode(0)));
        img = base;
    }
    img
}

/// Gemischte Kacheln eines Rechtecks (Zeilen ab `ty0`, Spalten ab `tx0`).
#[derive(Clone, Debug)]
pub struct TileGrid {
    pub tx0: i32,
    pub ty0: i32,
    pub cols: usize,
    pub tiles: Vec<Arc<RgbaImage>>,
}

impl TileGrid {
    /// Pixel an Dokumentposition `(x, y)` (außerhalb transparent).
    #[inline]
    pub fn get(&self, x: i32, y: i32) -> [u8; 4] {
        let (tx, ty) = (x.div_euclid(TILE) - self.tx0, y.div_euclid(TILE) - self.ty0);
        if tx < 0 || ty < 0 || tx as usize >= self.cols {
            return [0, 0, 0, 0];
        }
        self.tiles.get(ty as usize * self.cols + tx as usize).and_then(|t| t.get(x, y)).unwrap_or([0, 0, 0, 0])
    }

    /// Kacheln zu einem Bild mit den Grenzen `rect` zusammensetzen.
    pub fn assemble(&self, rect: IRect) -> RgbaImage {
        let mut out = RgbaImage::new(rect).unwrap_or_else(|_| RgbaImage::empty_at(rect.left, rect.top));
        if out.is_empty() {
            return out;
        }
        for (k, t) in self.tiles.iter().enumerate() {
            let (tx, ty) = (self.tx0 + (k % self.cols) as i32, self.ty0 + (k / self.cols) as i32);
            let tr = tile_rect(tx, ty);
            let (l, r) = (tr.left.max(rect.left), tr.right.min(rect.right));
            let (tp, b) = (tr.top.max(rect.top), tr.bottom.min(rect.bottom));
            if l >= r || tp >= b {
                continue;
            }
            let w = (r - l) as usize;
            for y in tp..b {
                let Some(src) = t.row(y) else { continue };
                let dst = out.row_local_mut((y - rect.top) as usize);
                let so = (l - tr.left) as usize;
                let d0 = (l - rect.left) as usize;
                dst[d0..d0 + w].copy_from_slice(&src[so..so + w]);
            }
        }
        out
    }
}

impl TileCache {
    /// Kennzahlen.
    pub fn stats(&self) -> TileStats {
        TileStats { tiles: self.count, hits: self.hits, misses: self.misses }
    }

    /// Leert den Cache.
    pub fn clear(&mut self) {
        self.map.clear();
        self.count = 0;
    }

    /// Mischt `ops` über `fill` (danach ggf. über `underlay`) im Rechteck `rect`.
    /// `budget_pixels` begrenzt den Cache (0 = ohne Cache mischen).
    pub fn compose(&mut self, rect: IRect, fill: Rgba8, underlay: Option<Rgba8>, ops: &[DrawOp], budget_pixels: usize) -> RgbaImage {
        if rect.right <= rect.left || rect.bottom <= rect.top {
            return RgbaImage::empty_at(rect.left, rect.top);
        }
        match self.compose_grid(rect, fill, underlay, ops, budget_pixels) {
            Some(g) => g.assemble(rect),
            None => mix(rect, fill, underlay, &ops.iter().collect::<Vec<_>>()),
        }
    }

    /// Wie [`TileCache::compose`], liefert aber die Kacheln selbst (ohne Zusammensetzen);
    /// `None`, wenn direkt gemischt werden muss (Auflösen, kein Budget).
    pub fn compose_grid(&mut self, rect: IRect, fill: Rgba8, underlay: Option<Rgba8>, ops: &[DrawOp], budget_pixels: usize) -> Option<TileGrid> {
        if rect.right <= rect.left || rect.bottom <= rect.top || budget_pixels < (TILE * TILE) as usize || ops.iter().any(|o| o.blend == BlendMode::DISSOLVE) {
            return None;
        }
        self.clock += 1;
        let clock = self.clock;
        let (tx0, ty0) = (rect.left.div_euclid(TILE), rect.top.div_euclid(TILE));
        let (tx1, ty1) = ((rect.right - 1).div_euclid(TILE), (rect.bottom - 1).div_euclid(TILE));
        let coords: Vec<(i32, i32)> = (ty0..=ty1).flat_map(|ty| (tx0..=tx1).map(move |tx| (tx, ty))).collect();
        // Signatur je Kachel
        let sigs: Vec<(Vec<OpSig>, Vec<&DrawOp>)> = coords
            .par_iter()
            .map(|&(tx, ty)| {
                let tr = tile_rect(tx, ty);
                let hit: Vec<&DrawOp> = ops.iter().filter(|o| intersects(o.img.bounds(), tr)).collect();
                let sig = hit.iter().map(|o| OpSig { ptr: Arc::as_ptr(&o.img) as usize, bounds: o.img.bounds(), opacity: o.opacity.0, blend: o.blend.0 as i32 }).collect();
                (sig, hit)
            })
            .collect();
        // vorhandene Kacheln suchen
        let mut found: Vec<Option<Arc<RgbaImage>>> = Vec::with_capacity(coords.len());
        for (k, c) in coords.iter().enumerate() {
            let e = self.map.get_mut(c).and_then(|v| v.iter_mut().find(|e| e.fill == fill && e.underlay == underlay && e.sig == sigs[k].0));
            match e {
                Some(e) => {
                    e.used = clock;
                    self.hits += 1;
                    found.push(Some(Arc::clone(&e.img)));
                }
                None => {
                    self.misses += 1;
                    found.push(None);
                }
            }
        }
        // fehlende Kacheln parallel mischen
        let fresh: Vec<(usize, Arc<RgbaImage>)> = found
            .iter()
            .enumerate()
            .filter(|(_, f)| f.is_none())
            .collect::<Vec<_>>()
            .par_iter()
            .map(|(k, _)| {
                let (tx, ty) = coords[*k];
                (*k, Arc::new(mix(tile_rect(tx, ty), fill, underlay, &sigs[*k].1)))
            })
            .collect();
        let max_tiles = (budget_pixels / (TILE * TILE) as usize).max(1);
        for (k, img) in fresh {
            let keep: Vec<Arc<RgbaImage>> = sigs[k].1.iter().map(|o| Arc::clone(&o.img)).collect();
            let v = self.map.entry(coords[k]).or_default();
            if v.len() >= 2 {
                // ältesten Eintrag dieser Stelle ersetzen
                if let Some(i) = v.iter().enumerate().min_by_key(|(_, e)| e.used).map(|(i, _)| i) {
                    v.remove(i);
                    self.count -= 1;
                }
            }
            v.push(Entry { fill, underlay, sig: sigs[k].0.clone(), _keep: keep, img: Arc::clone(&img), used: clock });
            self.count += 1;
            found[k] = Some(img);
        }
        if self.count > max_tiles {
            self.evict(max_tiles, clock);
        }
        let tiles = found.into_iter().map(|t| t.unwrap_or_else(|| Arc::new(RgbaImage::empty_at(0, 0)))).collect();
        Some(TileGrid { tx0, ty0, cols: (tx1 - tx0 + 1) as usize, tiles })
    }

    fn evict(&mut self, max_tiles: usize, clock: u64) {
        // zuerst alles, was im letzten Durchgang nicht gebraucht wurde, nach Alter
        let mut ages: Vec<u64> = self.map.values().flat_map(|v| v.iter().map(|e| e.used)).collect();
        ages.sort_unstable();
        let cut = ages.get(self.count.saturating_sub(max_tiles)).copied().unwrap_or(clock);
        let cut = cut.min(clock.saturating_sub(1));
        let mut n = 0;
        for v in self.map.values_mut() {
            v.retain(|e| e.used > cut);
            n += v.len();
        }
        self.map.retain(|_, v| !v.is_empty());
        self.count = n;
    }
}

//! Element-Cache: fertig gerechnetes Aussehen je Element (nach Effekten, vor Deckkraft/Mischmodus).
//!
//! **Invalidierung**: Ein Eintrag hält einen Klon des `Arc<Element>` (bzw. des `Arc<Symbol>` bei
//! Instanzen). Dadurch erzeugt jede spätere Bearbeitung über `Arc::make_mut` (Copy-on-Write im
//! Modell) zwingend eine neue Allokation; ein Eintrag ist gültig, solange der Zeiger gleich ist
//! (`Arc::ptr_eq`), die Abbildung und der Status gleich sind und die **Änderungs-Generation** des
//! Caches sich nicht geändert hat. Die Generation erhöht der Aufrufer bei Änderungen außerhalb
//! eines Elements, die dessen Aussehen beeinflussen (Symboldefinition in verschachtelten
//! Instanzen, Schriften, Voreinstellungen): [`RenderCache::bump_generation`].

use std::collections::HashMap;
use std::sync::Arc;

use fp_model::{Element, ElementId, Symbol};
use fp_raster::RgbaImage;

use crate::nine::PointMap;
use crate::options::RenderReport;

/// Fertiges Aussehen eines Elements.
#[derive(Debug, Clone)]
pub struct Look {
    /// Pixel in Dokumentkoordinaten.
    pub image: Arc<RgbaImage>,
    /// Zusätzliche Deckkraft (Animationsinstanzen), wird mit der Elementdeckkraft multipliziert.
    pub extra_opacity: Option<u8>,
}

#[derive(Debug, Clone)]
struct Entry {
    frame: usize,
    map: PointMap,
    elem: Arc<Element>,
    symbol: Option<Arc<Symbol>>,
    generation: u64,
    look: Option<Look>,
    report: RenderReport,
    pixels: usize,
}

/// Cache der Element-Ansichten (siehe Moduldoku).
#[derive(Debug, Default)]
pub struct RenderCache {
    entries: HashMap<ElementId, Vec<Entry>>,
    generation: u64,
    pixels: usize,
    hits: u64,
    misses: u64,
}

/// Kennzahlen des Caches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CacheStats {
    pub entries: usize,
    pub pixels: usize,
    pub generation: u64,
    pub hits: u64,
    pub misses: u64,
}

impl RenderCache {
    /// Erhöht die Änderungs-Generation: alle Einträge werden ungültig.
    pub fn bump_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.entries.clear();
        self.pixels = 0;
    }

    /// Verwirft alle Einträge eines Elements.
    pub fn invalidate(&mut self, id: ElementId) {
        if let Some(v) = self.entries.remove(&id) {
            self.pixels = self.pixels.saturating_sub(v.iter().map(|e| e.pixels).sum());
        }
    }

    /// Leert den Cache.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.pixels = 0;
    }

    /// Kennzahlen.
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entries: self.entries.values().map(|v| v.len()).sum(),
            pixels: self.pixels,
            generation: self.generation,
            hits: self.hits,
            misses: self.misses,
        }
    }

    pub(crate) fn get(
        &mut self,
        e: &Arc<Element>,
        frame: usize,
        map: &PointMap,
        symbol: Option<&Arc<Symbol>>,
    ) -> Option<(Option<Look>, RenderReport)> {
        let g = self.generation;
        let hit = self.entries.get(&e.id).and_then(|v| {
            v.iter().find(|x| {
                x.frame == frame
                    && x.generation == g
                    && Arc::ptr_eq(&x.elem, e)
                    && &x.map == map
                    && match (&x.symbol, symbol) {
                        (None, None) => true,
                        (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                        _ => false,
                    }
            })
        });
        match hit {
            Some(x) => {
                self.hits += 1;
                Some((x.look.clone(), x.report.clone()))
            }
            None => {
                self.misses += 1;
                None
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn put(
        &mut self,
        e: &Arc<Element>,
        frame: usize,
        map: &PointMap,
        symbol: Option<&Arc<Symbol>>,
        look: Option<Look>,
        report: RenderReport,
        budget: usize,
    ) {
        if budget == 0 {
            return;
        }
        let pixels = look
            .as_ref()
            .map(|l| l.image.width() * l.image.height())
            .unwrap_or(0)
            + 1;
        if pixels > budget / 2 {
            return;
        }
        if self.pixels + pixels > budget {
            self.clear();
        }
        let v = self.entries.entry(e.id).or_default();
        // Veraltete Einträge desselben Elements (anderer Zeiger) entfernen.
        let mut freed = 0;
        v.retain(|x| {
            let keep = Arc::ptr_eq(&x.elem, e) && x.generation == self.generation;
            if !keep {
                freed += x.pixels;
            }
            keep
        });
        if v.len() >= 8 {
            freed += v.remove(0).pixels;
        }
        v.push(Entry {
            frame,
            map: map.clone(),
            elem: Arc::clone(e),
            symbol: symbol.cloned(),
            generation: self.generation,
            look,
            report,
            pixels,
        });
        self.pixels = self.pixels.saturating_sub(freed) + pixels;
    }
}

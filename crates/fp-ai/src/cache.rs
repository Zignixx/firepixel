//! Zwischenspeicher: geladene Modelle, Bild-Einbettungen (SAM 2) und Ergebnisse (BiRefNet).
//!
//! Schlüssel eines Bildes ist ein schneller Inhalts-Hash der Eingabepixel plus Ausschnitt; ein
//! zweiter Klick auf dasselbe Bild braucht deshalb nur noch den Decoder.

use crate::birefnet::BiRefNet;
use crate::cancel::Progress;
use crate::error::Result;
use crate::models::{self, ModelId};
use crate::sam2::{ImageEmbedding, Sam2};
use crate::weights::Weights;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// Ein geladenes Modell.
pub enum Loaded {
    Sam2(Box<Sam2>),
    BiRefNet(Box<BiRefNet>),
}

fn models_map() -> &'static Mutex<HashMap<ModelId, Arc<Loaded>>> {
    static M: OnceLock<Mutex<HashMap<ModelId, Arc<Loaded>>>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(HashMap::new()))
}

// Ladevorgänge serialisieren (nicht zweimal dasselbe große Modell gleichzeitig lesen).
fn load_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

/// Ist das Modell schon geladen?
pub fn is_loaded(id: ModelId) -> bool {
    models_map().lock().map(|m| m.contains_key(&id)).unwrap_or(false)
}

/// Modell holen (lädt beim ersten Mal von der Platte).
pub fn model(id: ModelId) -> Result<Arc<Loaded>> {
    if let Ok(m) = models_map().lock()
        && let Some(x) = m.get(&id)
    {
        return Ok(Arc::clone(x));
    }
    let _g = load_lock().lock();
    if let Ok(m) = models_map().lock()
        && let Some(x) = m.get(&id)
    {
        return Ok(Arc::clone(x));
    }
    let path = models::find(id).ok_or_else(|| models::missing(id))?;
    let wt = Weights::load(&path)?;
    let bytes = wt.float_count() as u64 * 4;
    let loaded = crate::nn::pool().install(|| -> Result<Loaded> {
        Ok(match models::info(id).task {
            models::Task::Object => Loaded::Sam2(Box::new(Sam2::load(wt)?)),
            _ => Loaded::BiRefNet(Box::new(BiRefNet::load(wt)?)),
        })
    })?;
    let a = Arc::new(loaded);
    if let Ok(mut m) = models_map().lock() {
        m.insert(id, Arc::clone(&a));
    }
    if let Ok(mut s) = model_bytes().lock() {
        s.insert(id, bytes);
    }
    Ok(a)
}

/// Alle geladenen Modelle verwerfen (Speicher freigeben).
pub fn unload_all() {
    if let Ok(mut m) = models_map().lock() {
        m.clear();
    }
    if let Ok(mut s) = model_bytes().lock() {
        s.clear();
    }
}

/// Ein Modell aus dem Speicher nehmen (z. B. vor dem Entfernen der Datei).
pub fn unload(id: ModelId) {
    if let Ok(mut m) = models_map().lock() {
        m.remove(&id);
    }
    if let Ok(mut s) = model_bytes().lock() {
        s.remove(&id);
    }
}

fn model_bytes() -> &'static Mutex<HashMap<ModelId, u64>> {
    static M: OnceLock<Mutex<HashMap<ModelId, u64>>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Belegter Arbeitsspeicher der Zwischenspeicher (geladene Modelle, Einbettungen, Ergebnisse) in Byte.
pub fn memory_bytes() -> u64 {
    let m: u64 = model_bytes().lock().map(|s| s.values().sum()).unwrap_or(0);
    let e: u64 = emb_cache().lock().map(|c| c.items.iter().map(|(_, v)| ((v.emb.len() + v.s0.len() + v.s1.len()) * 4) as u64).sum()).unwrap_or(0);
    let a: u64 = alpha_cache().lock().map(|c| c.items.iter().map(|(_, v)| (v.len() * 4) as u64).sum()).unwrap_or(0);
    m + e + a
}

/// Schneller Inhalts-Hash (FNV-1a 64 über 8-Byte-Wörter, Länge eingerechnet).
pub fn hash_bytes(b: &[u8], extra: &[u64]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mix = |h: &mut u64, v: u64| {
        *h ^= v;
        *h = h.wrapping_mul(0x0000_0100_0000_01b3);
        *h ^= *h >> 29;
    };
    mix(&mut h, b.len() as u64);
    for e in extra {
        mix(&mut h, *e);
    }
    let (chunks, rest) = b.as_chunks::<8>();
    for c in chunks {
        mix(&mut h, u64::from_le_bytes(*c));
    }
    for (i, x) in rest.iter().enumerate() {
        mix(&mut h, (*x as u64) << (i * 8));
    }
    h
}

/// Kleiner LRU-Speicher.
struct Lru<V> {
    cap: usize,
    items: Vec<(u64, Arc<V>)>,
}

impl<V> Lru<V> {
    fn get(&mut self, k: u64) -> Option<Arc<V>> {
        let i = self.items.iter().position(|(kk, _)| *kk == k)?;
        let it = self.items.remove(i);
        let v = Arc::clone(&it.1);
        self.items.push(it);
        Some(v)
    }
    fn put(&mut self, k: u64, v: Arc<V>) {
        self.items.retain(|(kk, _)| *kk != k);
        self.items.push((k, v));
        while self.items.len() > self.cap {
            self.items.remove(0);
        }
    }
}

fn emb_cache() -> &'static Mutex<Lru<ImageEmbedding>> {
    static C: OnceLock<Mutex<Lru<ImageEmbedding>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(Lru { cap: 4, items: Vec::new() }))
}

fn alpha_cache() -> &'static Mutex<Lru<Vec<f32>>> {
    static C: OnceLock<Mutex<Lru<Vec<f32>>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(Lru { cap: 4, items: Vec::new() }))
}

/// Bild-Einbettung aus dem Speicher.
pub fn embedding_cached(key: u64) -> Option<Arc<ImageEmbedding>> {
    emb_cache().lock().ok()?.get(key)
}

/// Bild-Einbettung holen oder berechnen (`compute` läuft nur bei Fehltreffer).
pub fn embedding(key: u64, compute: impl FnOnce() -> Result<ImageEmbedding>) -> Result<Arc<ImageEmbedding>> {
    if let Some(e) = embedding_cached(key) {
        return Ok(e);
    }
    let e = Arc::new(compute()?);
    if let Ok(mut c) = emb_cache().lock() {
        c.put(key, Arc::clone(&e));
    }
    Ok(e)
}

/// Alpha-Ergebnis (Modellauflösung) aus dem Speicher.
pub fn alpha_cached(key: u64) -> Option<Arc<Vec<f32>>> {
    alpha_cache().lock().ok()?.get(key)
}

/// Alpha-Ergebnis holen oder berechnen.
pub fn alpha(key: u64, compute: impl FnOnce() -> Result<Vec<f32>>) -> Result<Arc<Vec<f32>>> {
    if let Some(e) = alpha_cached(key) {
        return Ok(e);
    }
    let e = Arc::new(compute()?);
    if let Ok(mut c) = alpha_cache().lock() {
        c.put(key, Arc::clone(&e));
    }
    Ok(e)
}

/// Zwischenspeicher leeren (Tests).
pub fn clear_results() {
    if let Ok(mut c) = emb_cache().lock() {
        c.items.clear();
    }
    if let Ok(mut c) = alpha_cache().lock() {
        c.items.clear();
    }
}

/// Lädt ein Modell im Hintergrund vor (z. B. beim Wählen des Werkzeugs), ohne zu blockieren.
pub fn preload(id: ModelId, _prog: &Progress) {
    if is_loaded(id) || !models::available(id) {
        return;
    }
    std::thread::spawn(move || {
        let _ = model(id);
    });
}

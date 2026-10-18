//! Miniaturen der Dateiliste (36 px), im Hintergrund gelesen (Muster wie `welcome\thumbs.rs`):
//! eine Warteschlange mit höchstens [`WORKERS`] Threads, damit 10 000 Dateien den Rechner nicht
//! fluten; angefragt werden nur die sichtbaren Zeilen. Schlüssel: Pfad + Änderungszeit.
//! Dekodiert wird über `fp-batch` (alle Bild- und Animationsformate, FW-PNG als Dokument).

use egui::{ColorImage, Id, TextureHandle, TextureOptions};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

/// Größte Kantenlänge der Miniatur (36 px bei bis zu 200 %).
const THUMB_PX: usize = 80;
/// Größere Dateien werden nicht gelesen.
const MAX_BYTES: u64 = 200 * 1024 * 1024;
/// Gleichzeitige Lese-Threads.
const WORKERS: usize = 2;

#[derive(Clone)]
enum State {
    Loading,
    Ready(TextureHandle),
    None,
}

type Key = (PathBuf, Option<SystemTime>);

#[derive(Default)]
struct Inner {
    map: HashMap<Key, State>,
    /// Has the file transparent pixels? Known once its thumbnail was read (BA-107 filter).
    alpha: HashMap<Key, bool>,
    queue: VecDeque<Key>,
    workers: usize,
}

#[derive(Clone, Default)]
struct Cache(Arc<Mutex<Inner>>);

fn cache(ctx: &egui::Context) -> Cache {
    ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<Cache>(Id::new("fp-batch-thumbs"))
            .clone()
    })
}

/// Miniatur, falls schon gelesen; reiht sie sonst ein.
pub fn get(
    ctx: &egui::Context,
    path: &Path,
    modified: Option<SystemTime>,
    bytes: u64,
) -> Option<TextureHandle> {
    if bytes > MAX_BYTES {
        return None;
    }
    let key: Key = (path.to_path_buf(), modified);
    let c = cache(ctx);
    let mut g = c.0.lock().unwrap_or_else(|e| e.into_inner());
    match g.map.get(&key) {
        Some(State::Ready(t)) => return Some(t.clone()),
        Some(_) => return None,
        None => {}
    }
    g.map.insert(key.clone(), State::Loading);
    g.queue.push_back(key);
    if g.workers < WORKERS {
        g.workers += 1;
        drop(g);
        let ctx2 = ctx.clone();
        let c2 = c.clone();
        let spawned = std::thread::Builder::new()
            .name("fp-batch-thumb".into())
            .spawn(move || worker(ctx2, c2));
        if spawned.is_err() {
            c.0.lock().unwrap_or_else(|e| e.into_inner()).workers -= 1;
            log::warn!("thumbnail thread could not be started");
        }
    }
    None
}

fn worker(ctx: egui::Context, c: Cache) {
    loop {
        let key = {
            let mut g = c.0.lock().unwrap_or_else(|e| e.into_inner());
            match g.queue.pop_front() {
                Some(k) => k,
                None => {
                    g.workers -= 1;
                    return;
                }
            }
        };
        let (state, alpha) = match decode(&key.0) {
            Some((img, alpha)) => (
                State::Ready(ctx.load_texture(
                    format!("fp-batch-thumb-{}", key.0.display()),
                    img,
                    TextureOptions::LINEAR,
                )),
                alpha,
            ),
            None => (State::None, false),
        };
        let mut g = c.0.lock().unwrap_or_else(|e| e.into_inner());
        g.alpha.insert(key.clone(), alpha);
        g.map.insert(key, state);
        drop(g);
        ctx.request_repaint();
    }
}

/// Has the file transparent pixels (first image)? `None` while unknown; asks for the thumbnail
/// (which reads it) like [`get`]. Files too large for a thumbnail stay unknown.
pub fn alpha(
    ctx: &egui::Context,
    path: &Path,
    modified: Option<SystemTime>,
    bytes: u64,
) -> Option<bool> {
    let key: Key = (path.to_path_buf(), modified);
    if let Some(a) = cache(ctx)
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .alpha
        .get(&key)
    {
        return Some(*a);
    }
    let _ = get(ctx, path, modified, bytes);
    None
}

/// Werden noch Miniaturen gelesen?
pub fn pending(ctx: &egui::Context) -> bool {
    cache(ctx)
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .map
        .values()
        .any(|s| matches!(s, State::Loading))
}

/// Liest die Datei (erstes Bild) und verkleinert auf höchstens [`THUMB_PX`]; dazu, ob das Bild
/// transparente Pixel hat (vor dem Verkleinern geprüft).
fn decode(path: &Path) -> Option<(ColorImage, bool)> {
    let info = fp_batch::probe(path).ok()?;
    if matches!(
        info.kind,
        fp_batch::ItemKind::Video | fp_batch::ItemKind::Other
    ) {
        return None;
    }
    let item = fp_batch::item::load(&info, false).ok()?;
    let img = item.preview_image().ok()?;
    let alpha = img.data().iter().any(|p| p[3] < 255);
    Some((to_color_image(&img, THUMB_PX), alpha))
}

/// Verkleinert ein Bild (Flächenmittel, vormultipliziert gemittelt) auf höchstens `max` px.
pub fn to_color_image(img: &fp_raster::RgbaImage, max: usize) -> ColorImage {
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return ColorImage::new([1, 1], vec![egui::Color32::TRANSPARENT]);
    }
    let px = img.data();
    let s = (max as f32 / w.max(h) as f32).min(1.0);
    let (tw, th) = (
        ((w as f32 * s).round() as usize).max(1),
        ((h as f32 * s).round() as usize).max(1),
    );
    if tw == w && th == h {
        let raw: Vec<u8> = px.iter().flat_map(|p| p.iter().copied()).collect();
        return ColorImage::from_rgba_unmultiplied([w, h], &raw);
    }
    let mut out = Vec::with_capacity(tw * th * 4);
    for ty in 0..th {
        let (y0, y1) = (ty * h / th, ((ty + 1) * h / th).max(ty * h / th + 1));
        for tx in 0..tw {
            let (x0, x1) = (tx * w / tw, ((tx + 1) * w / tw).max(tx * w / tw + 1));
            let (mut r, mut g, mut b, mut a, mut n) = (0u64, 0u64, 0u64, 0u64, 0u64);
            for y in y0..y1.min(h) {
                for x in x0..x1.min(w) {
                    let p = px[y * w + x];
                    let pa = p[3] as u64;
                    r += p[0] as u64 * pa;
                    g += p[1] as u64 * pa;
                    b += p[2] as u64 * pa;
                    a += pa;
                    n += 1;
                }
            }
            match (r.checked_div(a), g.checked_div(a), b.checked_div(a)) {
                (Some(r), Some(g), Some(b)) => {
                    out.extend_from_slice(&[r as u8, g as u8, b as u8, (a / n.max(1)) as u8])
                }
                _ => out.extend_from_slice(&[0, 0, 0, 0]),
            }
        }
    }
    ColorImage::from_rgba_unmultiplied([tw, th], &out)
}

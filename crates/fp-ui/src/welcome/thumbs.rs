//! Kleine Vorschaubilder für „Zuletzt geöffnete Dateien“ auf dem Startbildschirm (DES-2, N11).
//!
//! Gelesen wird im Hintergrund (ein Thread je Datei, höchstens [`super::MAX_RECENT`] gleichzeitig),
//! damit der erste Frame nicht hängt. Das Ergebnis liegt als Textur im egui-Speicher, Schlüssel ist
//! Pfad + Änderungszeit; ändert sich die Datei, wird neu gelesen. Unterstützt werden die Formate,
//! die sich ohne Dokumentaufbau schnell lesen lassen (PNG inkl. FW-PNG – dessen PNG-Bild ist die
//! flache Vorschau –, JPEG, BMP, QOI). Alles andere zeigt das Symbol der Dateiart.

use egui::{ColorImage, Id, TextureHandle, TextureOptions};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

/// Größte Kantenlänge der Vorschau in Bildpunkten (für 36-px-Kästchen bei bis zu 200 %).
const THUMB_PX: usize = 80;
/// Größere Dateien werden nicht gelesen.
const MAX_BYTES: u64 = 48 * 1024 * 1024;

#[derive(Clone)]
enum State {
    Loading,
    Ready(TextureHandle),
    None,
}

type Key = (PathBuf, Option<SystemTime>);

#[derive(Clone, Default)]
struct Cache(Arc<Mutex<HashMap<Key, State>>>);

fn cache(ctx: &egui::Context) -> Cache {
    ctx.data_mut(|d| d.get_temp_mut_or_default::<Cache>(Id::new("fp-welcome-thumbs")).clone())
}

/// Lässt sich die Datei als Vorschau lesen?
pub fn supported(path: &Path) -> bool {
    matches!(ext(path).as_str(), "png" | "jpg" | "jpeg" | "bmp" | "qoi")
}

fn ext(path: &Path) -> String {
    path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

/// Vorschau einer Datei, falls schon gelesen; stößt sonst das Lesen an.
pub fn get(ctx: &egui::Context, path: &Path) -> Option<TextureHandle> {
    if !supported(path) {
        return None;
    }
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_BYTES {
        return None;
    }
    let key: Key = (path.to_path_buf(), meta.modified().ok());
    let c = cache(ctx);
    let mut map = c.0.lock().unwrap_or_else(|e| e.into_inner());
    match map.get(&key) {
        Some(State::Ready(t)) => return Some(t.clone()),
        Some(_) => return None,
        None => {}
    }
    map.insert(key.clone(), State::Loading);
    drop(map);
    let ctx2 = ctx.clone();
    let spawned = std::thread::Builder::new().name("fp-welcome-thumb".into()).spawn(move || {
        let state = match decode(&key.0) {
            Some(img) => State::Ready(ctx2.load_texture(format!("fp-welcome-thumb-{}", key.0.display()), img, TextureOptions::LINEAR)),
            None => State::None,
        };
        c.0.lock().unwrap_or_else(|e| e.into_inner()).insert(key, state);
        ctx2.request_repaint();
    });
    if spawned.is_err() {
        log::warn!("Vorschau-Thread für den Startbildschirm ließ sich nicht starten");
    }
    None
}

/// Werden noch Vorschaubilder gelesen? (Für Tests und Bildserien.)
pub fn pending(ctx: &egui::Context) -> bool {
    cache(ctx).0.lock().unwrap_or_else(|e| e.into_inner()).values().any(|s| matches!(s, State::Loading))
}

/// Liest die Datei und verkleinert sie auf höchstens [`THUMB_PX`] Kantenlänge.
fn decode(path: &Path) -> Option<ColorImage> {
    let data = std::fs::read(path).ok()?;
    let r = match ext(path).as_str() {
        "png" => fp_io::png_in::decode_png(&data).ok()?,
        "jpg" | "jpeg" => fp_io::jpeg_in::decode(&data).ok()?,
        "bmp" => fp_io::bmp::decode(&data).ok()?,
        "qoi" => fp_io::modern::decode_qoi(&data).ok()?,
        _ => return None,
    };
    let (w, h) = (r.image.width(), r.image.height());
    if w == 0 || h == 0 {
        return None;
    }
    let px = r.image.data();
    let s = (THUMB_PX as f32 / w.max(h) as f32).min(1.0);
    let (tw, th) = (((w as f32 * s).round() as usize).max(1), ((h as f32 * s).round() as usize).max(1));
    // Flächenmittel je Zielpixel (vormultipliziert gemittelt, damit transparente Ränder nicht dunkeln)
    let mut out = Vec::with_capacity(tw * th * 4);
    for ty in 0..th {
        let (y0, y1) = (ty * h / th, ((ty + 1) * h / th).max(ty * h / th + 1));
        for tx in 0..tw {
            let (x0, x1) = (tx * w / tw, ((tx + 1) * w / tw).max(tx * w / tw + 1));
            let (mut r_, mut g, mut b, mut a, mut n) = (0u64, 0u64, 0u64, 0u64, 0u64);
            for y in y0..y1.min(h) {
                for x in x0..x1.min(w) {
                    let p = px[y * w + x];
                    let pa = p[3] as u64;
                    r_ += p[0] as u64 * pa;
                    g += p[1] as u64 * pa;
                    b += p[2] as u64 * pa;
                    a += pa;
                    n += 1;
                }
            }
            match (r_.checked_div(a), g.checked_div(a), b.checked_div(a)) {
                (Some(r), Some(g), Some(b)) => out.extend_from_slice(&[r as u8, g as u8, b as u8, (a / n.max(1)) as u8]),
                _ => out.extend_from_slice(&[0, 0, 0, 0]),
            }
        }
    }
    Some(ColorImage::from_rgba_unmultiplied([tw, th], &out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_types() {
        assert!(supported(Path::new(r"C:\a\b.fw.png")));
        assert!(supported(Path::new(r"C:\a\b.JPG")));
        assert!(!supported(Path::new(r"C:\a\b.jsf")));
        assert!(!supported(Path::new(r"C:\a\b.psd")));
    }
}

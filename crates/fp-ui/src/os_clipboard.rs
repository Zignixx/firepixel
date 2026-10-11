//! Systemzwischenablage für Bilder (HG): Kopieren/Ausschneiden legen zusätzlich zum internen
//! Inhalt (Vektoren, Attribute) ein flaches Bild in die Windows-Zwischenablage, damit andere
//! Programme es einfügen können. Einfügen nimmt ein fremdes Bild aus der Systemzwischenablage als
//! Bitmap (über `file.import`, ein Verlaufsschritt, mittig in der Ansicht); stammt das Bild von uns
//! selbst, gilt der interne Inhalt (Vektoren bleiben Vektoren) – wie im Original.
//!
//! Zugriff über arboard (reines Rust, `clipboard-win`). In Tests (`AppState::os_clipboard = false`)
//! wird die Systemzwischenablage nie berührt.

use crate::app_state::AppState;
use serde_json::json;
use std::borrow::Cow;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;

/// Prüfsumme des zuletzt von uns gesetzten Bildes.
static LAST_SET: Mutex<Option<u64>> = Mutex::new(None);

fn digest(w: usize, h: usize, bytes: &[u8]) -> u64 {
    let mut s = std::collections::hash_map::DefaultHasher::new();
    (w, h).hash(&mut s);
    bytes.hash(&mut s);
    s.finish()
}

/// Flaches Bild des internen Zwischenablage-Inhalts (Pixel oder gerenderte Elemente).
pub fn clipboard_image(st: &AppState) -> Option<fp_raster::RgbaImage> {
    if let Some(px) = &st.session.clipboard.pixels {
        return Some(px.clone());
    }
    let els = &st.session.clipboard.elements;
    if els.is_empty() {
        return None;
    }
    let d = st.session.active()?;
    crate::canvas::render::render_elements(&st.renderer, &d.doc, d.frame, els)
}

/// Nach erfolgreichem Kopieren/Ausschneiden: Bild in die Systemzwischenablage legen.
pub fn after_copy(st: &AppState) {
    if !st.os_clipboard {
        return;
    }
    let Some(img) = clipboard_image(st) else { return };
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return;
    }
    let bytes = img.as_bytes().to_vec();
    let sum = digest(w, h, &bytes);
    if let Ok(mut cb) = arboard::Clipboard::new()
        && cb.set_image(arboard::ImageData { width: w, height: h, bytes: Cow::Owned(bytes) }).is_ok()
        && let Ok(mut g) = LAST_SET.lock()
    {
        *g = Some(sum);
    }
}

/// Liest ein fremdes Bild aus der Systemzwischenablage (`None`: kein Bild oder unser eigenes).
pub fn foreign_image() -> Option<(usize, usize, Vec<u8>)> {
    let mut cb = arboard::Clipboard::new().ok()?;
    let img = cb.get_image().ok()?;
    let (w, h) = (img.width, img.height);
    let bytes = img.bytes.into_owned();
    if w == 0 || h == 0 || bytes.len() != w * h * 4 {
        return None;
    }
    let sum = digest(w, h, &bytes);
    if LAST_SET.lock().ok().and_then(|g| *g) == Some(sum) {
        return None;
    }
    Some((w, h, bytes))
}

/// Mitte des sichtbaren Teils der Leinwand in Dokumentkoordinaten.
pub fn visible_center(st: &AppState) -> Option<(f64, f64)> {
    let d = st.session.active()?;
    let v = &d.view;
    let z = v.zoom.max(1e-9);
    Some((v.origin.0 + v.viewport.0 / z / 2.0, v.origin.1 + v.viewport.1 / z / 2.0))
}

/// Fremden Text als Textobjekt einfügen: 12 pt, mittig im sichtbaren Teil, ausgewählt, ein
/// Verlaufsschritt (wie im Original).
pub fn paste_text(st: &mut AppState, text: &str) -> Result<(), String> {
    let (cx, cy) = visible_center(st).ok_or("kein Dokument geöffnet")?;
    let text = text.replace("\r\n", "\r").replace('\n', "\r");
    st.session
        .run_group("Einfügen", |s| {
            s.execute("draw.text", json!({ "text": text, "anchor": [cx, cy], "size": 12 }))?;
            // Grenzen des neuen Texts auf die Mitte schieben
            let b = s.active().and_then(|d| {
                let id = *d.selection.elements.first()?;
                let e = fp_engine::docops::element_on_page(&d.doc, d.page, id)?;
                fp_engine::docops::element_bounds(&d.doc, &e)
            });
            if let Some(b) = b {
                let dx = (cx - (b.left + b.right) as f64 / 2.0).round();
                let dy = (cy - (b.top + b.bottom) as f64 / 2.0).round();
                if dx != 0.0 || dy != 0.0 {
                    s.execute("edit.move", json!({ "dx": dx, "dy": dy }))?;
                }
            }
            Ok(())
        })
        .map_err(|e| e.to_string())
}

/// PNG schreiben (RGBA 8 Bit).
pub fn write_png(path: &std::path::Path, w: usize, h: usize, rgba: &[u8]) -> Result<(), String> {
    let f = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), w as u32, h as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().map_err(|e| e.to_string())?;
    wr.write_image_data(rgba).map_err(|e| e.to_string())
}

/// Vor `edit.paste`: fremdes Systembild als Bitmap einfügen. Liefert `true`, wenn eingefügt wurde.
pub fn paste_foreign(st: &mut AppState) -> bool {
    if !st.os_clipboard || st.session.active().is_none() {
        return false;
    }
    let Some((w, h, bytes)) = foreign_image() else { return false };
    let dir = std::env::temp_dir().join("Firepixel");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(format!("zwischenablage-{}.png", std::process::id()));
    if write_png(&path, w, h, &bytes).is_err() {
        return false;
    }
    // mittig im sichtbaren Teil der Leinwand (wie im Original)
    let at = visible_center(st).map(|(cx, cy)| [(cx - w as f64 / 2.0).round(), (cy - h as f64 / 2.0).round()]).unwrap_or([0.0, 0.0]);
    let ok = st.session.execute("file.import", json!({ "path": path.to_string_lossy(), "at": at })).is_ok();
    let _ = std::fs::remove_file(&path);
    ok
}

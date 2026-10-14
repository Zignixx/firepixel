//! Miniaturen für Ebenen-, Seiten- und Inspektor-Vorschauen über fp-render. Zwischenspeicher je
//! (Dokument, Element/Seite, Größe); neu gerechnet wird nur, wenn sich genau dieses Element bzw.
//! diese Seite geändert hat (Copy-on-Write: anderer `Arc`), höchstens ein paar Miniaturen pro Bild
//! (Zähler je egui-Durchlauf; fehlende folgen in den nächsten Bildern).

use crate::app_state::AppState;
use egui::{ColorImage, Context, Id, TextureHandle, TextureOptions};
use fp_engine::DocId;
use fp_model::{Document, Element};
use fp_raster::RgbaImage;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Cache {
    map: HashMap<(u64, u64, u32), (u64, TextureHandle)>,
    frame: u64,
    used: u32,
    /// Seitenminiaturen, die im Hintergrund neu gerechnet werden (Schlüssel → Stand, Ergebnis).
    pending: HashMap<(u64, u64, u32), (u64, std::sync::mpsc::Receiver<Option<RgbaImage>>)>,
}

const MAX_PER_FRAME: u32 = 6;

fn cache(ctx: &Context) -> Arc<Mutex<Cache>> {
    let id = Id::new("fp-thumb-cache");
    if let Some(c) = ctx.data(|d| d.get_temp::<Arc<Mutex<Cache>>>(id)) {
        return c;
    }
    let c = Arc::new(Mutex::new(Cache::default()));
    ctx.data_mut(|d| d.insert_temp(id, Arc::clone(&c)));
    c
}

fn to_texture(ctx: &Context, name: String, img: &RgbaImage, size: u32) -> Option<TextureHandle> {
    if img.width() == 0 || img.height() == 0 {
        return None;
    }
    let s = (size as f32 / img.width().max(img.height()) as f32).min(1.0);
    let (w, h) = (((img.width() as f32 * s).round() as u32).max(1), ((img.height() as f32 * s).round() as u32).max(1));
    let small = if s < 1.0 { fp_raster::resample::resize(img, w, h, fp_raster::resample::Sampler::Bilinear) } else { img.clone() };
    let ci = ColorImage::from_rgba_unmultiplied([small.width(), small.height()], small.as_bytes());
    Some(ctx.load_texture(name, ci, TextureOptions::LINEAR))
}

fn get(ctx: &Context, st: &AppState, key: (u64, u64, u32), generation: u64, render: impl FnOnce() -> Option<RgbaImage>) -> Option<TextureHandle> {
    let c = cache(ctx);
    let Ok(mut c) = c.lock() else { return None };
    let pass = ctx.cumulative_pass_nr();
    if c.frame != pass {
        c.frame = pass;
        c.used = 0;
    }
    let _ = st;
    let existing = c.map.get(&key).cloned();
    if let Some((g, t)) = &existing
        && *g == generation
    {
        return Some(t.clone());
    }
    if c.used >= MAX_PER_FRAME {
        ctx.request_repaint();
        return existing.map(|(_, t)| t);
    }
    c.used += 1;
    let img = render()?;
    let tex = to_texture(ctx, format!("thumb-{}-{}-{}", key.0, key.1, key.2), &img, key.2)?;
    c.map.insert(key, (generation, tex.clone()));
    if c.map.len() > 2000 {
        c.map.clear();
    }
    Some(tex)
}

/// Wie [`get`], aber eine **vorhandene** Miniatur wird im Hintergrund erneuert; bis dahin bleibt die
/// alte stehen (PERF-1: die Seitenminiatur rechnet die ganze Seite 1:1 – bei Seiten mit Tausenden
/// Objekten nach jeder Pfeiltaste Hunderte ms im UI-Thread). Erste Miniatur und Tests
/// (`sync_render`) rechnen wie bisher sofort.
fn get_refresh_async(ctx: &Context, st: &AppState, key: (u64, u64, u32), generation: u64, render: impl FnOnce() -> Option<RgbaImage> + Send + 'static) -> Option<TextureHandle> {
    let c = cache(ctx);
    let Ok(mut guard) = c.lock() else { return None };
    // fertige Hintergrundrechnung übernehmen
    if let Some((g, rx)) = guard.pending.get(&key) {
        match rx.try_recv() {
            Ok(img) => {
                let g = *g;
                guard.pending.remove(&key);
                if let Some(tex) = img.and_then(|img| to_texture(ctx, format!("thumb-{}-{}-{}", key.0, key.1, key.2), &img, key.2)) {
                    guard.map.insert(key, (g, tex));
                }
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                guard.pending.remove(&key);
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
    }
    let existing = guard.map.get(&key).cloned();
    match &existing {
        Some((g, t)) if *g == generation => return Some(t.clone()),
        Some(_) if !st.sync_render => {}
        _ => {
            drop(guard);
            return get(ctx, st, key, generation, render);
        }
    }
    if !guard.pending.contains_key(&key) {
        let (tx, rx) = std::sync::mpsc::channel();
        let ctx2 = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(render());
            ctx2.request_repaint();
        });
        guard.pending.insert(key, (generation, rx));
    }
    // läuft noch (ggf. für einen älteren Stand – danach startet die nächste Rechnung)
    ctx.request_repaint_after(std::time::Duration::from_millis(50));
    existing.map(|(_, t)| t)
}

/// Miniatur eines Elements.
pub fn element_thumb(ctx: &Context, st: &AppState, doc: DocId, generation: u64, d: &Arc<Document>, e: &Arc<Element>, size: u32) -> Option<TextureHandle> {
    let r = Arc::clone(&st.renderer);
    let frame = st.session.document(doc).map(|x| x.frame).unwrap_or(0);
    let _ = generation;
    let dep = if matches!(e.kind, fp_model::ElementKind::Instance(_) | fp_model::ElementKind::Group(_)) { (Arc::as_ptr(d) as usize as u64).rotate_left(29) } else { 0 };
    get(ctx, st, (doc.0, e.id.0, size), (Arc::as_ptr(e) as usize as u64) ^ (frame as u64) ^ dep, || {
        let out = r.render_element(d, e, &fp_render::PointMap::identity(), frame);
        Some(out.image)
    })
}

/// Miniatur der Maske eines maskierten Objekts (Ebenen-Bedienfeld): weiß = sichtbar wie im Original.
/// Bitmapmasken mit `MNAb` speichern invertiertes Grau (07 §7.5), daher hier zurückgedreht.
pub fn mask_thumb(ctx: &Context, st: &AppState, doc: DocId, d: &Arc<Document>, group: &fp_model::Element, mask: &Arc<Element>, size: u32) -> Option<TextureHandle> {
    let invert = matches!(&group.kind, fp_model::ElementKind::Group(g) if g.kind == fp_model::GroupKind::BitmapMask && g.mask_flags.hide_attrs)
        && matches!(mask.kind, fp_model::ElementKind::Bitmap(_));
    // Vektormaske (Pfadumriss): Form weiß auf Schwarz wie im Original (N35, Nutzerbild 15)
    let shape = matches!(&group.kind, fp_model::ElementKind::Group(g) if g.kind == fp_model::GroupKind::VectorMask);
    if !invert && !shape {
        return element_thumb(ctx, st, doc, 0, d, mask, size);
    }
    let r = Arc::clone(&st.renderer);
    let frame = st.session.document(doc).map(|x| x.frame).unwrap_or(0);
    let bit = if shape { MASK_SHAPE } else { MASK_INV };
    get(ctx, st, (doc.0, mask.id.0 | bit, size), (Arc::as_ptr(mask) as usize as u64) ^ (frame as u64), || {
        let mut img = r.render_element(d, mask, &fp_render::PointMap::identity(), frame).image;
        for p in img.data_mut() {
            if shape {
                p[0] = 255;
                p[1] = 255;
                p[2] = 255;
            } else {
                p[0] = 255 - p[0];
                p[1] = 255 - p[1];
                p[2] = 255 - p[2];
            }
        }
        Some(img)
    })
}

/// Schlüsselbit der Formminiaturen von Vektormasken.
const MASK_SHAPE: u64 = 1 << 60;

/// Schlüsselbit der invertierten Maskenminiaturen.
const MASK_INV: u64 = 1 << 61;

/// Schlüsselbit der Miniaturen „Element auf der Seite“ (getrennt von Element-, Seiten- und
/// Symbolminiaturen im selben Zwischenspeicher).
const ON_PAGE: u64 = 1 << 62;

/// Miniatur eines Elements an seiner Lage auf der verkleinerten Seite (Vorschaubild im Inspektor wie
/// im Original, PARITAET_A O2): transparentes Bild im Seitenformat (längste Seite `size`), darin nur
/// das Element an seinem Platz.
pub fn element_on_page_thumb(ctx: &Context, st: &AppState, doc: DocId, d: &Arc<Document>, page: usize, e: &Arc<Element>, size: u32) -> Option<TextureHandle> {
    let r = Arc::clone(&st.renderer);
    let frame = st.session.document(doc).map(|x| x.frame).unwrap_or(0);
    let cv = d.pages.get(page)?.canvas;
    let (pw, ph, ox, oy) = (cv.width, cv.height, cv.origin.x, cv.origin.y);
    let dep = if matches!(e.kind, fp_model::ElementKind::Instance(_) | fp_model::ElementKind::Group(_)) { (Arc::as_ptr(d) as usize as u64).rotate_left(29) } else { 0 };
    let geom = (u64::from(pw.to_bits()) << 32 | u64::from(ph.to_bits())) ^ (u64::from(ox.to_bits()) << 16) ^ u64::from(oy.to_bits()).rotate_left(48);
    get(ctx, st, (doc.0, e.id.0 | ON_PAGE, size), (Arc::as_ptr(e) as usize as u64) ^ (frame as u64) ^ dep ^ geom, || element_on_page_image(&r, d, page, e, frame, size))
}

/// Bild zu [`element_on_page_thumb`]: Seitenformat mit der längsten Seite `size` (höchstens 1:1),
/// transparent, das Element verkleinert an seiner Lage. `None` ohne Seite.
pub fn element_on_page_image(r: &fp_render::Renderer, d: &Document, page: usize, e: &Arc<Element>, frame: usize, size: u32) -> Option<RgbaImage> {
    let cv = &d.pages.get(page)?.canvas;
    let (pw, ph, ox, oy) = (cv.width.max(1.0), cv.height.max(1.0), cv.origin.x, cv.origin.y);
    let s = (size as f32 / pw.max(ph)).min(1.0);
    let (w, h) = (((pw * s).round() as i32).max(1), ((ph * s).round() as i32).max(1));
    let mut canvas = RgbaImage::new(fp_raster::IRect::from_xywh(0, 0, w, h)).ok()?;
    let out = r.render_element(d, e, &fp_render::PointMap::identity(), frame);
    if !out.image.is_empty() {
        let b = out.image.bounds();
        let (x0, y0) = (((b.left as f32 - ox) * s).floor() as i32, ((b.top as f32 - oy) * s).floor() as i32);
        let (x1, y1) = (((b.right as f32 - ox) * s).ceil() as i32, ((b.bottom as f32 - oy) * s).ceil() as i32);
        let mut small = fp_raster::resample::resize(&out.image, (x1 - x0).max(1) as u32, (y1 - y0).max(1) as u32, fp_raster::resample::Sampler::Bilinear);
        small.set_origin(x0, y0);
        canvas.paste(&small);
    }
    Some(canvas)
}

/// Miniatur einer Seite (Status 0).
pub fn page_thumb(ctx: &Context, st: &AppState, doc: DocId, generation: u64, d: &Arc<Document>, page: usize, size: u32) -> Option<TextureHandle> {
    let r = Arc::clone(&st.renderer);
    let _ = generation;
    let key_gen = d.pages.get(page).map(|p| Arc::as_ptr(p) as usize as u64).unwrap_or(0) ^ ((Arc::as_ptr(d) as usize as u64).rotate_left(17) * u64::from(!d.symbols.is_empty() || !d.shared_layers.is_empty()));
    let d = Arc::clone(d);
    get_refresh_async(ctx, st, (doc.0, u64::MAX - page as u64, size), key_gen, move || {
        let out = r.render_page(&d, page, 0);
        Some(out.image)
    })
}

/// Miniatur eines Symbols (Dokumentbibliothek).
pub fn symbol_thumb(ctx: &Context, st: &AppState, doc: DocId, generation: u64, d: &Arc<Document>, index: usize, size: u32) -> Option<TextureHandle> {
    let r = Arc::clone(&st.renderer);
    let sym = d.symbols.get(index).cloned()?;
    let _ = generation;
    get(ctx, st, (doc.0, (u64::MAX / 2) - index as u64, size), Arc::as_ptr(&sym) as usize as u64, || {
        let out = r.render_symbol(d, &sym, 0);
        Some(out.image)
    })
}

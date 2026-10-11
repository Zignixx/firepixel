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

/// Miniatur einer Seite (Status 0).
pub fn page_thumb(ctx: &Context, st: &AppState, doc: DocId, generation: u64, d: &Arc<Document>, page: usize, size: u32) -> Option<TextureHandle> {
    let r = Arc::clone(&st.renderer);
    let _ = generation;
    let key_gen = d.pages.get(page).map(|p| Arc::as_ptr(p) as usize as u64).unwrap_or(0) ^ ((Arc::as_ptr(d) as usize as u64).rotate_left(17) * u64::from(!d.symbols.is_empty() || !d.shared_layers.is_empty()));
    get(ctx, st, (doc.0, u64::MAX - page as u64, size), key_gen, || {
        let out = r.render_page(d, page, 0);
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

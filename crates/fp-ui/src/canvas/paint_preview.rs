//! Live-Vorschau der Malwerkzeuge (Pinsel, Bleistift, Radiergummi; PINSEL-1, N31): Die Engine
//! liefert die geänderten Pixel der Bitmap als Kacheln ([`fp_engine::paint::PaintPreview`]); hier
//! wird je Kachel eine Textur gehalten und nur neu hochgeladen, wenn sich die Kachel geändert hat.
//! Nach dem Loslassen bleibt die Vorschau stehen, bis das Hintergrundbild mit dem gemalten Strich
//! da ist (höchstens 2 s), damit der Strich nicht kurz verschwindet.

use egui::{Color32, Pos2, Rect, TextureHandle, TextureOptions};
use fp_engine::paint::PaintPreview;
use fp_geom::Point;
use std::collections::HashMap;

/// Texturen der Vorschau eines Dokuments.
#[derive(Clone, Default)]
pub struct PaintPreviewCache {
    /// Kennung des Zugs, zu dem die Texturen gehören.
    stroke: u64,
    tiles: HashMap<(i32, i32), (TextureHandle, u64, fp_geom::IRect)>,
    /// Zeit, zu der der Zug zuletzt lief (Nachlauf nach dem Loslassen).
    last_seen: f64,
    /// Zug ist vorbei (Loslassen oder Esc).
    ended: bool,
}

fn key(doc: fp_engine::DocId) -> egui::Id {
    egui::Id::new(("fp-paint-preview", doc.0))
}

fn color_image(img: &fp_raster::RgbaImage) -> egui::ColorImage {
    egui::ColorImage::from_rgba_unmultiplied([img.width(), img.height()], img.as_bytes())
}

fn draw(painter: &egui::Painter, c: &PaintPreviewCache, spp: &dyn Fn(Point) -> Pos2) {
    for (tex, _, r) in c.tiles.values() {
        let rect = Rect::from_min_max(spp(Point::new(r.left as f32, r.top as f32)), spp(Point::new(r.right as f32, r.bottom as f32)));
        painter.image(tex.id(), rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
    }
}

/// Zeichnet die Vorschau eines laufenden Zugs; lädt nur geänderte Kacheln hoch. Liefert die Zahl
/// der hochgeladenen Kacheln (Tests).
pub fn paint(ctx: &egui::Context, painter: &egui::Painter, doc: fp_engine::DocId, p: &PaintPreview, zoom: f64, time: f64, spp: &dyn Fn(Point) -> Pos2) -> usize {
    let mut uploads = 0;
    let filter = if zoom >= 1.0 { TextureOptions::NEAREST } else { TextureOptions::LINEAR };
    ctx.data_mut(|d| {
        let c = d.get_temp_mut_or_default::<PaintPreviewCache>(key(doc));
        if c.stroke != p.stroke {
            *c = PaintPreviewCache { stroke: p.stroke, ..Default::default() };
        }
        c.last_seen = time;
        c.ended = false;
        for t in p.tiles.iter() {
            match c.tiles.get_mut(&t.key) {
                Some((tex, ver, _)) if *ver == t.version => {
                    let _ = tex;
                }
                Some((tex, ver, _)) => {
                    tex.set(color_image(&t.image), filter);
                    *ver = t.version;
                    uploads += 1;
                }
                None => {
                    let tex = ctx.load_texture(format!("paint-{}-{}-{}", p.stroke, t.key.0, t.key.1), color_image(&t.image), filter);
                    c.tiles.insert(t.key, (tex, t.version, t.rect));
                    uploads += 1;
                }
            }
        }
        draw(painter, c, spp);
    });
    uploads
}

/// Nach dem Zug: Vorschau stehen lassen, bis das Hintergrundbild den Stand `generation` zeigt
/// (`shown_generation`), höchstens 2 s; sonst Texturen freigeben.
pub fn linger(ctx: &egui::Context, painter: &egui::Painter, doc: fp_engine::DocId, generation: u64, shown_generation: Option<u64>, time: f64, spp: &dyn Fn(Point) -> Pos2) {
    let keep = ctx.data_mut(|d| {
        let c = d.get_temp_mut_or_default::<PaintPreviewCache>(key(doc));
        if c.tiles.is_empty() {
            return false;
        }
        if !c.ended {
            c.ended = true;
            c.last_seen = time;
        }
        let waiting = time - c.last_seen < 2.0 && shown_generation.is_none_or(|g| g != generation);
        if waiting {
            draw(painter, c, spp);
        } else {
            *c = PaintPreviewCache::default();
        }
        waiting
    });
    if keep {
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}

/// Anzahl der gehaltenen Vorschau-Kacheln (Tests).
pub fn cached_tiles(ctx: &egui::Context, doc: fp_engine::DocId) -> usize {
    ctx.data_mut(|d| d.get_temp_mut_or_default::<PaintPreviewCache>(key(doc)).tiles.len())
}

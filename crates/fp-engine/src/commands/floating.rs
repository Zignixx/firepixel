//! FB-137 (RE spec 80): the floating pixel selection.
//!
//! Moving selected pixels lifts them out of their bitmap into a block that floats above it
//! ([`crate::selection::FloatingPixels`], held in the selection so that every History snapshot
//! carries it). The bitmap keeps the hole (a copy leaves it unchanged) and is not touched while the
//! block moves on: pixels the block passes over stay intact and show again when it moves away, and
//! a further drag or nudge picks up only the block. The block is merged (source-over, cut at the
//! canvas edge) only when the session runs a command that is not one of the few that keep it
//! (deselect, a new selection, leaving bitmap mode, save, painting, filters ...; see
//! [`keeps_floating`]); Delete discards it.

use super::{canvas_rect, sel_err};
use crate::command::CommandOutput;
use crate::docops;
use crate::error::{EngineError, Result};
use crate::selection::{FloatingPixels, Selection};
use crate::session::DocumentState;
use fp_geom::IRect;
use fp_model::{Document, ElementId, ElementKind};
use fp_raster::{Mask, RgbaImage};
use std::sync::Arc;

/// `block` composited over `base` at the offset `(dx, dy)` (source-over, straight alpha, the
/// measured rule of RE spec 80 section 6.2: transparent block pixels cover nothing). The result
/// grows to hold the moved block; `clip` (the canvas, section 6.4) limits the part of the block
/// that is placed, pixels outside it are dropped.
pub(crate) fn composite_block(
    base: &RgbaImage,
    block: &RgbaImage,
    (dx, dy): (i32, i32),
    clip: Option<IRect>,
) -> RgbaImage {
    let mut target = block.bounds().translate(dx, dy);
    if let Some(c) = clip {
        target = target.intersect(&c);
    }
    if target.is_empty() {
        return base.clone();
    }
    let nb = if base.bounds().is_empty() {
        target
    } else {
        base.bounds().union(&target)
    };
    let Ok(mut out) = base.extended_to(nb) else {
        return base.clone();
    };
    for y in target.top..target.bottom {
        for x in target.left..target.right {
            let Some(p) = block.get(x - dx, y - dy).filter(|p| p[3] > 0) else {
                continue;
            };
            let [r, g, b, a] = over(p, out.get(x, y).unwrap_or([0; 4]));
            out.set_pixel(x, y, fp_color::Rgba8::new(r, g, b, a));
        }
    }
    out
}

/// Source-over of the straight RGBA pixel `p` onto `q` (RE spec 80 section 6.2).
fn over(p: [u8; 4], q: [u8; 4]) -> [u8; 4] {
    let sa = p[3] as u32;
    if sa == 0 {
        return q;
    }
    let da = q[3] as u32 * (255 - sa) / 255;
    let oa = sa + da;
    let c = |i: usize| ((p[i] as u32 * sa + q[i] as u32 * da + oa / 2) / oa.max(1)) as u8;
    [c(0), c(1), c(2), oa.min(255) as u8]
}

/// Edits the pixels of a bitmap inside `rect` (document pixels) tile by tile: only the image
/// tiles that overlap `rect` are rebuilt, the others stay shared (a 12 Mpx bitmap otherwise costs
/// a full decode and encode per drag step). `f(x, y, rgba)` changes one straight RGBA pixel.
/// Returns `false` without a change when `rect` is not inside the bitmap, the bitmap does not sit
/// on whole pixels or a tile is not a plain image tile; the caller then edits the whole image.
fn edit_tiles(
    b: &mut fp_model::BitmapElem,
    rect: IRect,
    mut f: impl FnMut(i32, i32, &mut [u8; 4]),
) -> bool {
    use fp_model::image::{Tile, TileEntry, TileKind};
    let p = b.image.position();
    if p.x.fract() != 0.0 || p.y.fract() != 0.0 {
        return false;
    }
    let grid = &b.image.grid;
    let ts = grid.tile_size as i32;
    let (ox, oy) = (p.x as i32, p.y as i32);
    let bounds = IRect::from_xywh(ox, oy, grid.width as i32, grid.height as i32);
    if ts <= 0 || rect.intersect(&bounds) != rect {
        return false;
    }
    if rect.is_empty() {
        return true;
    }
    let cols = grid.columns() as i32;
    let tile_bytes = (ts * ts * 4) as usize;
    let (tx0, ty0) = ((rect.left - ox) / ts, (rect.top - oy) / ts);
    let (tx1, ty1) = ((rect.right - 1 - ox) / ts, (rect.bottom - 1 - oy) / ts);
    let mut fresh: Vec<(usize, Vec<u8>)> = Vec::new();
    for ty in ty0..=ty1 {
        for tx in tx0..=tx1 {
            let k = (ty * cols + tx) as usize;
            let mut buf = match grid.tiles.get(k) {
                Some(TileEntry::Solid(c)) => [c.a(), c.r(), c.g(), c.b()].repeat(tile_bytes / 4),
                Some(TileEntry::Tile(t))
                    if t.kind == TileKind::Image && t.pixels.len() >= tile_bytes =>
                {
                    t.pixels[..tile_bytes].to_vec()
                }
                _ => return false,
            };
            let tile = IRect::from_xywh(ox + tx * ts, oy + ty * ts, ts, ts).intersect(&rect);
            for y in tile.top..tile.bottom {
                for x in tile.left..tile.right {
                    let i = (((y - oy - ty * ts) * ts + (x - ox - tx * ts)) * 4) as usize;
                    let mut px = [buf[i + 1], buf[i + 2], buf[i + 3], buf[i]];
                    f(x, y, &mut px);
                    buf[i..i + 4].copy_from_slice(&[px[3], px[0], px[1], px[2]]);
                }
            }
            fresh.push((k, buf));
        }
    }
    let mut next = docops::tile_counter();
    for (k, buf) in fresh {
        next = next.wrapping_add(1);
        b.image.grid.tiles[k] = TileEntry::Tile(Arc::new(Tile::new_image(next, buf)));
    }
    b.effect_result = None;
    true
}

/// Places `block` at each offset of `at` in turn ([`composite_block`]) into the bitmap `id` on
/// `page` tile by tile ([`edit_tiles`]), all or nothing; `false` = not possible, nothing changed.
fn composite_in_tiles(
    doc: &mut Document,
    page: usize,
    id: ElementId,
    block: &RgbaImage,
    at: &[(i32, i32)],
) -> bool {
    docops::edit_by_id(doc, page, id, |e| {
        let ElementKind::Bitmap(b) = &mut e.kind else {
            return false;
        };
        let mut nb = b.clone();
        for &(dx, dy) in at {
            let rect = block.bounds().translate(dx, dy);
            let placed = edit_tiles(&mut nb, rect, |x, y, px| {
                if let Some(p) = block.get(x - dx, y - dy) {
                    *px = over(p, *px);
                }
            });
            if !placed {
                return false;
            }
        }
        *b = nb;
        e.cache = None;
        true
    })
    .unwrap_or(false)
}

/// The canvas of `page` in whole document pixels (the edge a merge cuts at, RE spec 80 §6.4).
fn canvas_clip(doc: &Document, page: usize) -> Option<IRect> {
    let r = canvas_rect(doc, page);
    (r.width() > 0.0 && r.height() > 0.0).then(|| r.round_out())
}

/// The pixels of the bitmap `id` on `page`.
fn bitmap_pixels(doc: &Document, page: usize, id: ElementId) -> Option<RgbaImage> {
    let e = docops::element_on_page(doc, page, id)?;
    match &e.kind {
        ElementKind::Bitmap(b) => docops::bitmap_image(b),
        _ => None,
    }
}

/// Bounds of the bitmap `id` on `page` in document pixels (whole pixels only).
fn bitmap_bounds(doc: &Document, page: usize, id: ElementId) -> Option<IRect> {
    let e = docops::element_on_page(doc, page, id)?;
    let ElementKind::Bitmap(b) = &e.kind else {
        return None;
    };
    let p = b.image.position();
    (p.x.fract() == 0.0 && p.y.fract() == 0.0).then(|| {
        IRect::from_xywh(
            p.x as i32,
            p.y as i32,
            b.image.grid.width as i32,
            b.image.grid.height as i32,
        )
    })
}

/// Replaces the pixels of the bitmap `id` on `page`; whether it was found.
fn set_bitmap_pixels(doc: &mut Document, page: usize, id: ElementId, img: &RgbaImage) -> bool {
    docops::edit_by_id(doc, page, id, |e| {
        if let ElementKind::Bitmap(b) = &mut e.kind {
            docops::set_bitmap_image(b, img);
            e.cache = None;
            true
        } else {
            false
        }
    })
    .unwrap_or(false)
}

/// Translates the marquee by `(dx, dy)`.
pub(crate) fn translate_mask(sel: &mut Selection, dx: i32, dy: i32) {
    if (dx, dy) == (0, 0) {
        return;
    }
    if let Some(m) = sel.pixels.as_mut()
        && let Ok(nm) = Mask::from_raw(m.bounds().translate(dx, dy), m.data().to_vec())
    {
        *m = nm;
    }
}

/// Lifts the selected pixels of the selected bitmap into a floating block (RE spec 80 §6.1): the
/// block takes `B * M` from where the marquee is now, the bitmap keeps `B * (1 - M)` (with `copy`
/// it stays unchanged). Returns whether a block floats now (`false`: the marquee lies beside the
/// bitmap's pixels, nothing to lift). Errors without a pixel selection or a selected bitmap.
pub(crate) fn lift(d: &mut DocumentState, copy: bool) -> Result<bool> {
    if d.selection.floating.is_some() {
        return Ok(true);
    }
    let Some(m) = d
        .selection
        .pixels
        .clone()
        .filter(|_| d.selection.has_pixels())
    else {
        return Err(sel_err("err-sel-empty-pixels"));
    };
    let geo = crate::geometry::BasicGeometry;
    let (_, e) =
        super::select::bitmap_for(d, None, &geo).ok_or_else(|| sel_err("err-sel-no-bitmap"))?;
    let ElementKind::Bitmap(b) = &e.kind else {
        return Err(sel_err("err-sel-no-bitmap"));
    };
    let img = docops::bitmap_image(b)
        .ok_or_else(|| EngineError::Failed(fp_i18n::t("err-bitmap-unreadable-short")))?;
    let (hole, block) = docops::cut_selected_pixels(&img, &m);
    if block.width() == 0 || block.height() == 0 {
        return Ok(false);
    }
    let page = d.page;
    if !copy {
        let doc = Arc::make_mut(&mut d.doc);
        set_bitmap_pixels(doc, page, e.id, &hole);
    }
    d.selection.floating = Some(FloatingPixels {
        page,
        element: e.id,
        pixels: Arc::new(block),
        dx: 0,
        dy: 0,
    });
    Ok(true)
}

/// Places the floating block into its bitmap at the current place (cut at the canvas edge) and
/// keeps it floating ("stamp", RE spec 80 §6.2: an Alt move of a floating block). Whether the
/// bitmap was found.
fn place(d: &mut DocumentState, f: &FloatingPixels) -> bool {
    let Some(img) = bitmap_pixels(&d.doc, f.page, f.element) else {
        return false;
    };
    let clip = canvas_clip(&d.doc, f.page);
    let out = composite_block(&img, &f.pixels, (f.dx, f.dy), clip);
    let doc = Arc::make_mut(&mut d.doc);
    set_bitmap_pixels(doc, f.page, f.element, &out)
}

/// Merges the floating block into its bitmap (RE spec 80 §1.3, §6.2, §6.4): source-over at the
/// current place, the bitmap grows up to the canvas edge, pixels beyond it are lost. The marquee
/// stays where the block was. Whether a block was there (a block whose bitmap is gone is dropped).
pub(crate) fn merge(d: &mut DocumentState) -> bool {
    let Some(f) = d.selection.floating.take() else {
        return false;
    };
    place(d, &f);
    true
}

/// Delete with a floating block (RE spec 80 §1.2 G2): the block is discarded, the hole stays, the
/// marquee stays at the block's place. Whether a block was there.
pub(crate) fn discard(d: &mut DocumentState) -> bool {
    d.selection.floating.take().is_some()
}

/// A move of the selected pixels by `(dx, dy)` that leaves them floating (RE spec 80 §1.2, §2,
/// §3): without a block the pixels are lifted first (`copy`: a copy, the bitmap stays); with a
/// block `copy` places it at its current place and the same pixels float on. The marquee moves
/// along. Row "Move" (a copy: "Duplicate").
pub(crate) fn move_floating(
    d: &mut DocumentState,
    dx: i32,
    dy: i32,
    copy: bool,
) -> Result<CommandOutput> {
    match d.selection.floating.clone() {
        Some(f) if copy => {
            place(d, &f);
        }
        Some(_) => {}
        None => {
            lift(d, copy)?;
        }
    }
    if let Some(f) = d.selection.floating.as_mut() {
        f.dx += dx;
        f.dy += dy;
    }
    translate_mask(&mut d.selection, dx, dy);
    Ok(CommandOutput::doc().labeled(fp_i18n::t(if copy {
        "msg-step-duplicate"
    } else {
        "msg-step-move"
    })))
}

/// Commands that keep a floating block (RE spec 80 §4): undo/redo and the History, the moves that
/// handle the block themselves (`select.floatPixels`, arrow-key nudges, Delete), saving (the
/// session merges before it with a row of its own) and everything that only changes the view.
/// Every other command merges the block first, inside its own History row; one that then changes
/// neither the document nor the selection gets the block back (copying, exporting, panels ...).
pub(crate) fn keeps_floating(name: &str, params: &serde_json::Value) -> bool {
    const KEEP: &[&str] = &[
        "edit.undo",
        "edit.redo",
        "history.goTo",
        "history.export",
        "select.floatPixels",
        "edit.clear",
        "file.save",
        "file.saveAs",
        "file.saveAll",
        "file.saveAsTemplate",
    ];
    if KEEP.contains(&name) || name.starts_with("edit.nudge.") {
        return true;
    }
    if name == "edit.move" {
        let nudge = params.get("nudge").and_then(|v| v.as_bool()) == Some(true);
        let ids = params
            .get("ids")
            .and_then(|v| v.as_array())
            .is_some_and(|a| !a.is_empty());
        return nudge && !ids;
    }
    ["view.", "window.", "help."]
        .iter()
        .any(|p| name.starts_with(p))
}

/// The floating block shown on the canvas, as a render-only edit of `doc` (a copy of the
/// session's document; no History): the block composited into its bitmap at its place, moved
/// further by `(dx, dy)` while it is dragged; `copy` (an Alt drag) also places it at its current
/// place. Without a block (`sel.floating` is `None`) the drag of a lift is shown instead: the
/// pixels of the bitmap `id` under the marquee cut out (a `copy` leaves them) and composited at
/// `(dx, dy)`. The block is not cut at the canvas here (the page view clips it); returns whether
/// anything changed.
pub fn preview_floating(
    doc: &mut Document,
    page: usize,
    id: ElementId,
    sel: &Selection,
    (dx, dy): (i32, i32),
    copy: bool,
) -> bool {
    match &sel.floating {
        Some(f) => {
            // fast path: the block (and a placed copy) inside the bitmap, tile by tile
            let at = (f.dx + dx, f.dy + dy);
            let stamp = copy && (dx, dy) != (0, 0);
            let offsets: Vec<(i32, i32)> = if stamp {
                vec![(f.dx, f.dy), at]
            } else {
                vec![at]
            };
            if composite_in_tiles(doc, f.page, f.element, &f.pixels, &offsets) {
                return true;
            }
            let Some(img) = bitmap_pixels(doc, f.page, f.element) else {
                return false;
            };
            let base = if copy && (dx, dy) != (0, 0) {
                composite_block(&img, &f.pixels, (f.dx, f.dy), None)
            } else {
                img
            };
            let out = composite_block(&base, &f.pixels, (f.dx + dx, f.dy + dy), None);
            set_bitmap_pixels(doc, f.page, f.element, &out)
        }
        None => {
            let Some(m) = sel.pixels.as_ref().filter(|_| sel.has_pixels()) else {
                return false;
            };
            if (dx, dy) == (0, 0) {
                return false;
            }
            // fast path: cut and place tile by tile when the moved pixels stay inside the bitmap
            if let Some(bb) = bitmap_bounds(doc, page, id) {
                let r = m.tight_bounds().intersect(&bb);
                let t = r.translate(dx, dy);
                if !r.is_empty()
                    && t.intersect(&bb) == t
                    && let Ok(mut block) = RgbaImage::new(r)
                {
                    let mut cut_doc = doc.clone();
                    let cut = docops::edit_by_id(&mut cut_doc, page, id, |e| {
                        let ElementKind::Bitmap(b) = &mut e.kind else {
                            return false;
                        };
                        edit_tiles(b, r, |x, y, px| {
                            let k = m.get(x, y) as u32;
                            if k == 0 {
                                return;
                            }
                            let a = (px[3] as u32 * k + 127) / 255;
                            block.set_pixel(
                                x,
                                y,
                                fp_color::Rgba8::new(px[0], px[1], px[2], a as u8),
                            );
                            if !copy {
                                px[3] = ((px[3] as u32 * (255 - k) + 127) / 255) as u8;
                            }
                        })
                    })
                    .unwrap_or(false);
                    if cut && composite_in_tiles(&mut cut_doc, page, id, &block, &[(dx, dy)]) {
                        *doc = cut_doc;
                        return true;
                    }
                }
            }
            let Some(img) = bitmap_pixels(doc, page, id) else {
                return false;
            };
            let (hole, block) = docops::cut_selected_pixels(&img, m);
            let base = if copy { img } else { hole };
            let out = composite_block(&base, &block, (dx, dy), None);
            set_bitmap_pixels(doc, page, id, &out)
        }
    }
}

/// A key for the floating block of `sel` (0 = none): changes when the block, its place or its
/// bitmap change, for render caches.
pub fn floating_key(sel: &Selection) -> u64 {
    let Some(f) = &sel.floating else {
        return 0;
    };
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let b = f.pixels.bounds();
    for v in [
        Arc::as_ptr(&f.pixels) as usize as u64,
        b.left as i64 as u64,
        b.top as i64 as u64,
        b.width() as u64,
        b.height() as u64,
        f.page as u64,
        f.element.0,
        f.dx as i64 as u64,
        f.dy as i64 as u64,
    ] {
        h ^= v;
        h = h.wrapping_mul(0x0100_0000_01b3).rotate_left(11);
    }
    h | 1
}

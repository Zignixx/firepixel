//! F153 (FB-139): a canvas render that redraws only the pixels whose looks changed
//! (`Renderer::render_device_patch` against the display list of the previous render). Zoomed in,
//! a live move step then costs the area of the moved object, not of the whole view; the patched
//! image equals a whole render pixel for pixel.

use std::sync::Arc;

use fp_color::Argb;
use fp_geom::{IRect, Rect};
use fp_model::{Document, Effect, EffectList, Element, LayerSlot, PathAttrs};
use fp_raster::RgbaImage;
use fp_render::Renderer;

/// Page 400 × 300: a blue rectangle below, M (with a drop shadow) moved by `d`, a grey
/// rectangle with blend mode `blend` above.
fn doc(d: (f32, f32), blend: u8) -> Document {
    let mut doc = Document::new(400.0, 300.0, Argb(0xffff_ffff));
    let below = Element::new_rectangle(
        &mut doc.ids,
        Rect::new(20.0, 20.0, 200.0, 200.0),
        0.0,
        PathAttrs::solid_fill(Argb(0xff33_66ff)),
    );
    let r = Rect::new(80.0 + d.0, 60.0 + d.1, 127.0 + d.0, 94.0 + d.1);
    let mut m = Element::new_rectangle(
        &mut doc.ids,
        r,
        0.0,
        PathAttrs::solid_fill(Argb(0xffcc_6633)),
    );
    let mut fx = EffectList::default();
    fx.effects
        .push(Effect::new(fp_effects::chain::moa::DROP_SHADOW, true));
    m.common.effects = Some(fx);
    let mut above = Element::new_rectangle(
        &mut doc.ids,
        Rect::new(100.0, 40.0, 330.0, 240.0),
        0.0,
        PathAttrs::solid_fill(Argb(0xff80_8080)),
    );
    above.common.blend = fp_color::BlendMode(blend);
    let page = Arc::make_mut(&mut doc.pages[0]);
    if let LayerSlot::Own(l) = &mut page.root.layers[0] {
        Arc::make_mut(&mut Arc::make_mut(l).cels[0]).elements =
            vec![Arc::new(above), Arc::new(m), Arc::new(below)];
    }
    doc
}

fn m_of(doc: &Document) -> Arc<Element> {
    match &doc.pages[0].root.layers[0] {
        LayerSlot::Own(l) => Arc::clone(&l.cels[0].elements[1]),
        _ => panic!("layer"),
    }
}

/// `base` with M replaced by the M of `moved` (the other elements keep their allocations).
fn with_m(base: &Document, moved: &Document) -> Document {
    let mut out = base.clone();
    let page = Arc::make_mut(&mut out.pages[0]);
    if let LayerSlot::Own(l) = &mut page.root.layers[0] {
        Arc::make_mut(&mut Arc::make_mut(l).cels[0]).elements[1] = m_of(moved);
    }
    out
}

/// `img` with `patch` copied in at `dirty` (device pixels relative to `dev`).
fn apply(img: &RgbaImage, dev: IRect, patch: &RgbaImage, dirty: IRect) -> RgbaImage {
    let mut out = img.clone();
    if dirty.is_empty() {
        return out;
    }
    let (ox, oy) = (
        (dirty.left - dev.left) as usize,
        (dirty.top - dev.top) as usize,
    );
    let (w, stride) = (dirty.width() as usize * 4, out.width() * 4);
    let src = patch.as_bytes();
    let dst = out.as_bytes_mut();
    for y in 0..dirty.height() as usize {
        let d = (oy + y) * stride + ox * 4;
        dst[d..d + w].copy_from_slice(&src[y * w..(y + 1) * w]);
    }
    out
}

const CLIP: IRect = IRect::new(0, 0, 400, 300);

#[test]
fn patched_steps_equal_whole_renders_at_every_zoom() {
    for blend in [0, 1, 18] {
        for zoom in [1.0, 2.0, 4.0, 8.0, 1.5] {
            let base = doc((0.0, 0.0), blend);
            let r = Renderer::default();
            // a view part in the middle of the page (device pixels of the grid at the page)
            let dev = IRect::new(
                (60.0 * zoom) as i32,
                (30.0 * zoom) as i32,
                (60.0 * zoom) as i32 + 700,
                (30.0 * zoom) as i32 + 500,
            );
            let (out, list) = r.render_device_listed(&base, 0, 0, CLIP, (0, 0), zoom, dev);
            let mut shown = out.image;
            let mut list = list.expect("display list");
            for d in [(1.0, 0.0), (2.0, 1.0), (7.0, 3.0), (7.0, 3.0), (30.0, -9.0)] {
                let moved = with_m(&base, &doc(d, blend));
                let p = r
                    .render_device_patch(&moved, 0, 0, CLIP, (0, 0), zoom, dev, &list)
                    .expect("patch");
                let whole = Renderer::default()
                    .render_device(&moved, 0, 0, CLIP, (0, 0), zoom, dev)
                    .image;
                shown = apply(&shown, dev, &p.output.image, p.dirty);
                assert!(
                    shown.as_bytes() == whole.as_bytes(),
                    "blend {blend}, zoom {zoom}, step {d:?}: patched image differs"
                );
                // only the moved object's area (with its shadow) was redrawn
                assert!(
                    zoom > 4.0 || p.dirty.area() * 2 < dev.area(),
                    "blend {blend}, zoom {zoom}, step {d:?}: dirty {:?}",
                    p.dirty
                );
                list = p.list;
            }
        }
    }
}

#[test]
fn unchanged_document_patches_nothing_and_other_regions_refuse() {
    let base = doc((0.0, 0.0), 0);
    let r = Renderer::default();
    let dev = IRect::new(0, 0, 800, 600);
    let (_, list) = r.render_device_listed(&base, 0, 0, CLIP, (0, 0), 2.0, dev);
    let list = list.expect("list");
    let p = r
        .render_device_patch(&base, 0, 0, CLIP, (0, 0), 2.0, dev, &list)
        .expect("patch");
    assert!(p.dirty.is_empty(), "nothing changed");
    // another part or zoom: no patch (the caller renders the whole part)
    assert!(
        r.render_device_patch(
            &base,
            0,
            0,
            CLIP,
            (0, 0),
            2.0,
            IRect::new(0, 0, 400, 600),
            &list
        )
        .is_none()
    );
    assert!(
        r.render_device_patch(&base, 0, 0, CLIP, (0, 0), 4.0, dev, &list)
            .is_none()
    );
}

#[test]
fn dissolve_has_no_display_list() {
    // "Dissolve" mixes over whole rows: no patch base
    let mut base = doc((0.0, 0.0), 0);
    let page = Arc::make_mut(&mut base.pages[0]);
    if let LayerSlot::Own(l) = &mut page.root.layers[0] {
        let cel = Arc::make_mut(&mut Arc::make_mut(l).cels[0]);
        Arc::make_mut(&mut cel.elements[0]).common.blend = fp_color::BlendMode::DISSOLVE;
    }
    let r = Renderer::default();
    let (_, list) =
        r.render_device_listed(&base, 0, 0, CLIP, (0, 0), 2.0, IRect::new(0, 0, 800, 600));
    assert!(list.is_none());
}

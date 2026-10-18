//! PSD layers of a page, one per object (BA-102, "Convert to PSD layers").
//!
//! Every layer of the document becomes a PSD group (`lsct`) with its objects as PSD layers
//! (bottom to top). Each object is rendered on its own: a copy of the document holds only that
//! object in its layer. With the export options:
//! - **Effects editable:** live effects that PSD has natively go into the layer effects block
//!   (`lfx2`) and are left out of the pixels; all others stay baked in.
//! - **Text editable:** text objects become type layers (`TySh`) with their rendered pixels.
//! - **Vector editable:** paths with a plain solid fill (no stroke, texture, feather or baked
//!   effect) become solid colour fill layers (`SoCo`) with a vector mask (`vmsk`).

use std::sync::Arc;

use fp_geom::IRect;
use fp_model::{Document, Element, ElementKind, LayerKind, LayerSlot};
use fp_raster::RgbaImage;

use crate::psd_blocks as pb;
use crate::psd_out::PsdLayer;
use crate::render::{Background, LayerFilter, PageRenderer, RenderRequest};
use crate::{ExportError, Result};

/// Which kinds of content stay editable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PsdEditable {
    pub effects: bool,
    pub vector: bool,
    pub text: bool,
}

/// Renders one element alone in its layer (layer opacity/blend neutral, element visible).
fn render_alone(
    doc: &Document,
    r: &dyn PageRenderer,
    page: usize,
    layer_ix: usize,
    elem: Element,
    rect: IRect,
) -> Result<RgbaImage> {
    let mut d = doc.clone();
    let mut lid = None;
    if let Some(p) = d.pages.get_mut(page)
        && let Some(LayerSlot::Own(l)) = Arc::make_mut(p).root.layers.get_mut(layer_ix)
    {
        let l = Arc::make_mut(l);
        l.opacity = fp_color::Opacity(255);
        l.blend = fp_color::BlendMode(0);
        lid = Some(l.id);
        if l.cels.is_empty() {
            l.cels.push(Arc::new(fp_model::Cel::empty()));
        }
        let mut e = elem;
        e.common.visible = true;
        e.common.opacity = fp_color::Opacity(255);
        e.common.blend = fp_color::BlendMode(0);
        for c in l.cels.iter_mut() {
            Arc::make_mut(c).elements = vec![Arc::new(e.clone())];
        }
    }
    let Some(lid) = lid else {
        return Ok(RgbaImage::empty_at(0, 0));
    };
    let img = r
        .render(
            &d,
            &RenderRequest {
                page,
                frame: 0,
                layers: LayerFilter::Only(vec![lid]),
                background: Background::Transparent,
                area: Some(rect),
            },
        )
        .map_err(ExportError::Render)?;
    let ab = img.alpha_bounds();
    let img = if ab.is_empty() {
        RgbaImage::empty_at(rect.left, rect.top)
    } else {
        img.cropped(ab)
    };
    Ok(img.translated(-rect.left, -rect.top))
}

fn plain_text(t: &fp_model::TextElem) -> String {
    let s: String = t.runs.iter().map(|r| r.text.as_str()).collect();
    s.replace('\u{3}', "\r").replace('\t', " ")
}

/// Type layer content of a text object (font, size and colour of the first run).
fn type_layer(
    doc: &Document,
    fonts: &fp_text::FontDb,
    t: &fp_model::TextElem,
    rect: IRect,
) -> pb::TypeLayer {
    let mut a = t.initial.clone();
    if let Some(r) = t.runs.first() {
        a.apply(&r.attrs);
    }
    let font = a
        .ps_font
        .clone()
        .or(t.initial.ps_font.clone())
        .or(a.font.clone())
        .unwrap_or_else(|| "ArialMT".into());
    let size = a.size.unwrap_or(12.0) as f64;
    let color = fp_color::Argb(a.color.unwrap_or(0xFF00_0000));
    let justification = match a.alignment.unwrap_or(0) {
        1 => 2,
        2 => 1,
        3 | 4 => 3,
        _ => 0,
    };
    // origin: first baseline at the left/centre/right end of the first line (text engine)
    let b = t.bounds;
    let block = fp_render::convert::text_block(doc, t);
    let lay = fp_text::layout(&block, fonts, &fp_text::LayoutOptions::default());
    let (ax, ay) = match lay.lines.first() {
        Some(l) => (
            match justification {
                2 => l.start + l.align_width / 2.0,
                1 => l.start + l.align_width,
                _ => l.start,
            },
            l.baseline,
        ),
        None => (b.left as f64, b.top as f64 + size * 0.9),
    };
    let (mut x, mut y) = (ax, ay);
    let mut lin = [1.0, 0.0, 0.0, 1.0];
    if let Some(m) = &t.transform {
        let m = &m.m;
        let (px, py) = (x, y);
        x = m[0][0] as f64 * px + m[1][0] as f64 * py + m[2][0] as f64;
        y = m[0][1] as f64 * px + m[1][1] as f64 * py + m[2][1] as f64;
        lin = [
            m[0][0] as f64,
            m[0][1] as f64,
            m[1][0] as f64,
            m[1][1] as f64,
        ];
    }
    pb::TypeLayer {
        text: plain_text(t),
        font,
        size,
        color,
        justification,
        transform: [
            lin[0],
            lin[1],
            lin[2],
            lin[3],
            x - rect.left as f64,
            y - rect.top as f64,
        ],
        bounds: [
            b.left as f64 - ax,
            b.top as f64 - ay,
            b.right as f64 - ax,
            b.bottom as f64 - ay,
        ],
    }
}

/// Solid fill colour of a path that PSD can show as a fill layer with vector mask.
fn shape_fill(p: &fp_model::PathElem) -> Option<fp_color::Argb> {
    let a = &p.attrs;
    let f = a.fill.as_ref()?;
    let plain = a.brush.is_none()
        && a.fill_texture.is_none()
        && f.shape == 0
        && f.feather == 0
        && f.category == "fc_Solid";
    plain.then_some(a.fill_color)
}

fn object_name(e: &Element, layer: &str, n: usize) -> String {
    if let Some(name) = e.common.name.as_ref().filter(|s| !s.is_empty()) {
        return name.clone();
    }
    if let ElementKind::Text(t) = &e.kind {
        let s: String = plain_text(t)
            .chars()
            .map(|c| if c == '\r' { ' ' } else { c })
            .take(40)
            .collect();
        if !s.trim().is_empty() {
            return s;
        }
    }
    format!("{layer} {n}")
}

/// PSD layers of page `page` (file order bottom to top), relative to `rect`.
pub fn psd_layers(
    doc: &Document,
    r: &dyn PageRenderer,
    page: usize,
    rect: IRect,
    ed: PsdEditable,
) -> Result<Vec<PsdLayer>> {
    let Some(p) = doc.pages.get(page) else {
        return Ok(Vec::new());
    };
    let fonts = fp_text::FontDb::shared();
    let mut out = Vec::new();
    for (li, slot) in p.root.layers.iter().enumerate() {
        let LayerSlot::Own(l) = slot else { continue };
        if l.kind == LayerKind::Web {
            continue;
        }
        let empty = |name: String, blocks| PsdLayer {
            name,
            image: RgbaImage::empty_at(0, 0),
            opacity: 255,
            blend: 0,
            visible: true,
            blocks,
        };
        out.push(empty(
            "</Layer group>".into(),
            vec![(*b"lsct", pb::section_block(3))],
        ));
        let elems: Vec<Arc<Element>> = l.cel(0).map(|c| c.elements.clone()).unwrap_or_default();
        for (n, e) in elems.iter().rev().enumerate() {
            let mut el: Element = (**e).clone();
            let mut blocks: Vec<([u8; 4], Vec<u8>)> = Vec::new();
            let mut baked_left = el
                .common
                .effects
                .as_ref()
                .is_some_and(|fx| fx.has_visible());
            if ed.effects
                && let Some(fx) = el.common.effects.as_mut()
                && let Some((data, used)) = pb::effects_block(&fx.effects)
            {
                blocks.push((*b"lfx2", data));
                let mut i = 0;
                fx.effects.retain(|_| {
                    let keep = !used.contains(&i);
                    i += 1;
                    keep
                });
                baked_left = fx.has_visible();
            }
            match &e.kind {
                ElementKind::Text(t) if ed.text => {
                    blocks.push((*b"TySh", pb::type_block(&type_layer(doc, &fonts, t, rect))));
                }
                ElementKind::Path(pe) if ed.vector && !baked_left => {
                    if let Some(c) = shape_fill(pe) {
                        blocks.push((*b"SoCo", pb::solid_color_block(c)));
                        blocks.push((
                            *b"vmsk",
                            pb::vector_mask_block(
                                &pe.contours,
                                (rect.left as f64, rect.top as f64),
                                rect.width() as f64,
                                rect.height() as f64,
                            ),
                        ));
                    }
                }
                _ => {}
            }
            let image = render_alone(doc, r, page, li, el, rect)?;
            out.push(PsdLayer {
                name: object_name(e, &l.name, n + 1),
                image,
                opacity: e.common.opacity.0,
                blend: e.common.blend.0,
                visible: e.common.visible,
                blocks,
            });
        }
        out.push(PsdLayer {
            name: l.name.clone(),
            image: RgbaImage::empty_at(0, 0),
            opacity: l.opacity.0,
            blend: l.blend.0,
            visible: l.is_visible(),
            blocks: vec![(*b"lsct", pb::section_block(1))],
        });
    }
    Ok(out)
}

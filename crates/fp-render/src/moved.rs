//! Looks of moved elements (F147, FB-131 to FB-135): the live move drag (F141, RE spec 76 §7.1)
//! renders a render-only copy of the document with the dragged elements moved by whole document
//! pixels. Each step makes new element allocations, so the element cache ([`crate::cache`],
//! keyed on the `Arc` pointer) would miss and the whole look including live effects (bevel, glow,
//! shadow) would be computed again per step. RE spec 76 §7.2: the effect rasters of the dragged
//! objects may be reused and only moved, the offset is whole document pixels (§7.4).
//!
//! **Rule.** A [`MovedLooks`] names pairs (moved element, source element): the moved element is
//! the source translated by the whole-pixel offset `(dx, dy)` (`edit.move` or its duplicate). While
//! a render runs with it ([`crate::Renderer::with_moved_looks`]), the look of a moved element is
//! the cached look of its source with the image origin shifted by `(dx, dy)`: no new effect
//! chain. A pair is only registered when the look of the source moves along unchanged
//! ([`look_moves_along`]): nothing in the element depends on its absolute position in the
//! document. Everything else (dithered fills and gradients, web dither, ordered-dither edges,
//! fills locked to the document, unlinked masks, Screentone, Photoshop effects, Add Noise and the
//! radial and zoom blurs, shared layers in symbols) is rendered normally, so the result always
//! equals the committed move. Children of groups and compound shapes are paired too (groups drawn
//! without isolation look up their children). Only looks drawn with the identity point map (page
//! elements) use a pair.

use std::sync::Arc;

use fp_effects::LiveEffect;
use fp_model::{Document, Element, ElementKind, Fill, LayerSlot, PathAttrs};

/// Pairs of moved elements and their sources with one whole-pixel offset (see the module doc).
#[derive(Debug, Clone, Default)]
pub struct MovedLooks {
    pub(crate) pairs: Vec<(Arc<Element>, Arc<Element>)>,
    pub(crate) offset: (i32, i32),
}

impl MovedLooks {
    /// Pairs for the top-level `(moved, source)` elements and their children; only those whose
    /// look moves along ([`look_moves_along`]). `doc` resolves the symbols of instances.
    pub fn new(
        doc: &Document,
        top: &[(Arc<Element>, Arc<Element>)],
        offset: (i32, i32),
    ) -> MovedLooks {
        let mut pairs = Vec::new();
        for (m, s) in top {
            collect(doc, m, s, &mut pairs, 0);
        }
        MovedLooks { pairs, offset }
    }

    /// Registered pairs.
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    /// No pair?
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    /// The offset in document pixels.
    pub fn offset(&self) -> (i32, i32) {
        self.offset
    }
}

fn collect(
    doc: &Document,
    moved: &Arc<Element>,
    src: &Arc<Element>,
    out: &mut Vec<(Arc<Element>, Arc<Element>)>,
    depth: u32,
) {
    if depth > 64 || Arc::ptr_eq(moved, src) {
        return;
    }
    if std::mem::discriminant(&moved.kind) != std::mem::discriminant(&src.kind) {
        return;
    }
    if look_moves_along(doc, src) {
        out.push((Arc::clone(moved), Arc::clone(src)));
    }
    let mut zip = |a: &[Arc<Element>], b: &[Arc<Element>]| {
        if a.len() == b.len() {
            for (x, y) in a.iter().zip(b) {
                collect(doc, x, y, out, depth + 1);
            }
        }
    };
    match (&moved.kind, &src.kind) {
        (ElementKind::Group(a), ElementKind::Group(b)) => {
            zip(&a.children, &b.children);
            if let (Some(x), Some(y)) = (&a.mask, &b.mask) {
                zip(std::slice::from_ref(x), std::slice::from_ref(y));
            }
        }
        (ElementKind::CompoundShape(a), ElementKind::CompoundShape(b)) => {
            zip(&a.operands, &b.operands);
            if let (Some(x), Some(y)) = (&a.result, &b.result) {
                zip(std::slice::from_ref(x), std::slice::from_ref(y));
            }
        }
        _ => {}
    }
}

/// Does the look of `e` stay the same, only shifted, when `e` is moved by whole document pixels
/// (`translate_element` without a fill locked to the document)? Conservative: `false` for
/// anything that depends on the absolute position (see the module doc).
pub fn look_moves_along(doc: &Document, e: &Element) -> bool {
    moves_along(doc, e, 0)
}

fn moves_along(doc: &Document, e: &Element, depth: u32) -> bool {
    if depth > 16 {
        return false;
    }
    if let Some(list) = &e.common.effects
        && !crate::convert::effect_entries(list)
            .iter()
            .all(|x| !x.visible || effect_moves_along(&x.effect))
    {
        return false;
    }
    let all = |xs: &[Arc<Element>]| xs.iter().all(|c| moves_along(doc, c, depth + 1));
    match &e.kind {
        ElementKind::Path(p) => attrs_move_along(&p.attrs),
        ElementKind::Text(t) => attrs_move_along(&t.attrs),
        ElementKind::Bitmap(b) => {
            let o = b.image.origin;
            o.x.fract() == 0.0 && o.y.fract() == 0.0
        }
        ElementKind::Group(g) => {
            // an unlinked mask stays where it is while the content moves (re 31 §5.4)
            let unlinked = g.kind.is_mask() && g.mask.is_some() && !g.mask_flags.linked;
            !unlinked
                && all(&g.children)
                && g.mask
                    .as_ref()
                    .is_none_or(|m| moves_along(doc, m, depth + 1))
                && g.replacement.as_ref().is_none_or(|r| all(r))
        }
        ElementKind::CompoundShape(c) => {
            all(&c.operands)
                && c.result
                    .as_ref()
                    .is_none_or(|r| moves_along(doc, r, depth + 1))
        }
        ElementKind::Instance(i) => doc.symbol_by_id(i.symbol_id).is_some_and(|(_, s)| {
            s.root.layers.iter().all(|slot| match slot {
                LayerSlot::Own(l) => l.cels.iter().all(|c| all(&c.elements)),
                LayerSlot::Raw(_) => true,
                _ => false,
            })
        }),
        ElementKind::WebObject(_) | ElementKind::Raw(_) => true,
        ElementKind::SubLayer(_) => false,
    }
}

/// Fills whose pixels depend on the document position: locked to the document (FB-44), ordered
/// dither edge or density (FB-23, FB-38), web dither (shape 16), dithered gradients.
fn attrs_move_along(a: &PathAttrs) -> bool {
    a.fill.as_ref().is_none_or(|f| {
        !f.locked_to_document()
            && f.edge != Fill::EDGE_DITHER
            && f.dither_density <= 0
            && f.shape != 16
            && f.transparent_dither != Some(true)
            && f.gradient
                .as_ref()
                .and_then(|g| g.stops.as_ref())
                .and_then(|s| s.dither)
                != Some(true)
    })
}

/// Live effects whose result is the same wherever the object lies.
fn effect_moves_along(e: &LiveEffect) -> bool {
    match e {
        LiveEffect::DropShadow(_)
        | LiveEffect::InnerShadow(_)
        | LiveEffect::Bevel(_)
        | LiveEffect::InnerGlow(_)
        | LiveEffect::SolidShadow(_)
        | LiveEffect::ColorFill(_)
        | LiveEffect::Unsupported { .. } => true,
        LiveEffect::Filter(f) => !matches!(
            f,
            fp_raster::Filter::AddNoise(_)
                | fp_raster::Filter::RadialBlur(_)
                | fp_raster::Filter::ZoomBlur(_)
        ),
        LiveEffect::Photoshop(_) | LiveEffect::Screentone(_) => false,
    }
}

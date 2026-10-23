//! Live preview of an inspector pop-up slider on the canvas (FB-09/FB-10, deliberate Firepixel
//! enhancement over the reference, which only updates the field while the thumb moves).
//!
//! While the thumb of an inspector pop-up slider is moved (drag from the arrow, or the pop-up
//! opened by a click), the inspector stores the pending value in
//! [`AppState::inspector_preview`]. The canvas renders a **copy** of the document with that value
//! applied to the selection ([`InspectorPreview::apply`]); the session's document, its history and
//! its dirty flag are never touched. The preview is reset at the start of every frame and set again
//! by the inspector only while the slider holds a pending value, so Esc, closing the pop-up, a
//! selection change or a hidden inspector discard it without residue. The single real apply
//! (one history step) happens through the normal command on release / close.
//!
//! Sliders with a preview (FB-09/FB-10, extended by FB-21/FB-22): opacity, stroke size, stroke edge
//! softness, fill edge feather, fill and stroke texture amount, and the text fields font size,
//! tracking, leading, indent, space before/after and horizontal scale (selected text objects; with
//! the text editor open only the field shows the pending value).
//!
//! Extended by FB-28/FB-29 (TW-205/TW-206): the Layers panel's opacity slider, the pending alpha
//! (and colour) of the fill and stroke colour pop-ups of the inspector, the Tools panel and the
//! Color Mixer, the Color Mixer's channel sliders and the rectangle roundness.

use crate::app_state::AppState;
use fp_color::Argb;
use fp_engine::DocId;
use fp_model::element::ElementKind;
use fp_model::{Document, Element, ElementId};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Which property the preview changes.
#[derive(Clone, Debug, PartialEq)]
pub enum PreviewValue {
    /// Object opacity in percent (0..100), like `element.setOpacity`.
    Opacity(f64),
    /// Stroke (brush tip) size in pixels, like `pi.stroke {width}`.
    StrokeWidth(f64),
    /// Stroke edge softness in percent (0..100), like `pi.stroke {softness}`.
    StrokeSoftness(f64),
    /// Fill edge feather / dither amount (0..100), like `pi.fill {feather, edge: 2}` (a dither
    /// edge keeps edge 3).
    Feather(f64),
    /// Fill texture amount in percent, like `pi.fill {textureAmount}`.
    FillTextureAmount(f64),
    /// Stroke texture amount in percent, like `pi.stroke {textureAmount}`.
    StrokeTextureAmount(f64),
    /// Text attributes, the parameters of `text.applyAttributes` (e.g. `{"size": 40}`).
    Text(serde_json::Value),
    /// Fill colour with alpha, like `pi.fill {color}` (no fill yet: a solid fill is created).
    FillColor(Argb),
    /// Stroke colour with alpha, like `pi.stroke {color}` on objects that have a stroke (an object
    /// without a stroke gets one only on commit).
    StrokeColor(Argb),
    /// Corner roundness of a rectangle primitive like `pi.roundness {value, pixels}`.
    Roundness { value: f64, pixels: bool },
}

/// A pending slider value shown on the canvas of one document.
#[derive(Clone, Debug, PartialEq)]
pub struct InspectorPreview {
    pub doc: DocId,
    /// Document generation the preview was made for (a stale preview is ignored).
    pub generation: u64,
    pub page: usize,
    /// Elements the value applies to (the selection when the slider was moved).
    pub ids: Vec<ElementId>,
    pub value: PreviewValue,
}

impl InspectorPreview {
    /// Preview for the active document's selection, `None` without a document or selection.
    pub fn for_selection(st: &AppState, value: PreviewValue) -> Option<InspectorPreview> {
        let d = st.session.active()?;
        if d.selection.elements.is_empty() {
            return None;
        }
        Some(InspectorPreview {
            doc: d.id,
            generation: d.generation,
            page: d.page,
            ids: d.selection.elements.clone(),
            value,
        })
    }

    /// Non-zero key of this preview for the render cache (a new value renders again).
    pub fn key(&self) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.generation.hash(&mut h);
        self.page.hash(&mut h);
        for id in &self.ids {
            id.0.hash(&mut h);
        }
        match &self.value {
            PreviewValue::Opacity(v) => (0u8, v.to_bits()).hash(&mut h),
            PreviewValue::StrokeWidth(v) => (1u8, v.to_bits()).hash(&mut h),
            PreviewValue::StrokeSoftness(v) => (2u8, v.to_bits()).hash(&mut h),
            PreviewValue::Feather(v) => (3u8, v.to_bits()).hash(&mut h),
            PreviewValue::FillTextureAmount(v) => (4u8, v.to_bits()).hash(&mut h),
            PreviewValue::StrokeTextureAmount(v) => (5u8, v.to_bits()).hash(&mut h),
            PreviewValue::Text(p) => (6u8, p.to_string()).hash(&mut h),
            PreviewValue::FillColor(c) => (7u8, c.0).hash(&mut h),
            PreviewValue::StrokeColor(c) => (8u8, c.0).hash(&mut h),
            PreviewValue::Roundness { value, pixels } => {
                (9u8, value.to_bits(), *pixels).hash(&mut h)
            }
        }
        h.finish() | 1
    }

    /// A copy of `doc` with the value applied to the preview's elements (the original document is
    /// not changed; unchanged parts stay shared).
    pub fn apply(&self, doc: &Document) -> Document {
        let mut out = doc.clone();
        if let PreviewValue::Text(p) = &self.value {
            // same attribute patch as `text.applyAttributes` on selected text objects
            let _ = fp_engine::commands::preview_text_attributes(&mut out, self.page, &self.ids, p);
            return out;
        }
        let pct = |v: f64| v.round().clamp(0.0, 100.0) as i32;
        for &id in &self.ids {
            let _ = fp_engine::docops::edit_by_id(&mut out, self.page, id, |e| match &self.value {
                PreviewValue::Opacity(v) => {
                    content_of(e).common.opacity = fp_color::Opacity::from_percent(*v);
                }
                PreviewValue::StrokeWidth(v) => {
                    fp_engine::docops::edit_path_attrs(e, &|a| {
                        if let Some(b) = a.brush.as_mut() {
                            b.diameter = v.round().max(1.0) as i32;
                        }
                    });
                }
                PreviewValue::StrokeSoftness(v) => {
                    fp_engine::docops::edit_path_attrs(e, &|a| {
                        if let Some(b) = a.brush.as_mut() {
                            b.softness = pct(*v) * 10;
                        }
                    });
                }
                PreviewValue::StrokeTextureAmount(v) => {
                    fp_engine::docops::edit_path_attrs(e, &|a| {
                        if let Some(b) = a.brush.as_mut() {
                            b.texture_blend = pct(*v) * 10;
                        }
                    });
                }
                PreviewValue::Feather(v) => {
                    fp_engine::docops::edit_path_attrs(e, &|a| {
                        if let Some(f) = a.fill.as_mut() {
                            // the dither edge (FB-23) keeps its kind, every other edge previews
                            // as feather
                            if f.edge != fp_model::Fill::EDGE_DITHER {
                                f.edge = fp_model::Fill::EDGE_FEATHER;
                            }
                            f.feather = pct(*v);
                        }
                    });
                }
                PreviewValue::FillTextureAmount(v) => {
                    fp_engine::docops::edit_path_attrs(e, &|a| {
                        if let Some(f) = a.fill.as_mut() {
                            f.texture_amount = pct(*v) * 10;
                        }
                    });
                }
                PreviewValue::FillColor(c) => {
                    fp_engine::docops::edit_path_attrs(e, &|a| {
                        a.fill_color = *c;
                        if a.fill.is_none() {
                            a.fill = Some(fp_model::Fill::solid());
                        }
                    });
                }
                PreviewValue::StrokeColor(c) => {
                    fp_engine::docops::edit_path_attrs(e, &|a| {
                        if a.brush.is_some() {
                            a.stroke_color = *c;
                        }
                    });
                }
                PreviewValue::Roundness { value, pixels } => {
                    fp_engine::commands::panel::apply_roundness(e, *value, *pixels);
                }
                PreviewValue::Text(_) => {}
            });
        }
        out
    }
}

/// Like the engine's opacity target: a single masked image gets the value on its content.
fn content_of(e: &mut Element) -> &mut Element {
    if e.masked_single().is_none() || !matches!(e.kind, ElementKind::Group(_)) {
        return e;
    }
    e.cache = None;
    let ElementKind::Group(g) = &mut e.kind else {
        unreachable!("masked_single is a group")
    };
    Element::edit(&mut g.children[0])
}

/// The document to render for `doc` (id `id`, generation `generation`): the preview copy while a
/// matching preview is active, otherwise `None` (render the document itself).
pub fn preview_doc(
    st: &AppState,
    id: DocId,
    generation: u64,
    page: usize,
    doc: &Arc<Document>,
) -> Option<Arc<Document>> {
    let p = st.inspector_preview.as_ref()?;
    if p.doc != id || p.generation != generation || p.page != page {
        return None;
    }
    Some(Arc::new(p.apply(doc)))
}

/// Key of the active preview for this document (0 = none), without building the copy.
pub fn preview_key(st: &AppState, id: DocId, generation: u64, page: usize) -> u64 {
    st.inspector_preview
        .as_ref()
        .filter(|p| p.doc == id && p.generation == generation && p.page == page)
        .map(InspectorPreview::key)
        .unwrap_or(0)
}

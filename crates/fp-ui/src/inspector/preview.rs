//! Live preview of an inspector pop-up slider on the canvas (FB-09/FB-10, deliberate Firepixel
//! enhancement over the reference, which only updates the field while the thumb moves).
//!
//! While the thumb of the opacity or stroke width slider is moved (drag from the arrow, or the
//! pop-up opened by a click), the inspector stores the pending value in
//! [`AppState::inspector_preview`]. The canvas renders a **copy** of the document with that value
//! applied to the selection ([`InspectorPreview::apply`]); the session's document, its history and
//! its dirty flag are never touched. The preview is reset at the start of every frame and set again
//! by the inspector only while the slider holds a pending value, so Esc, closing the pop-up, a
//! selection change or a hidden inspector discard it without residue. The single real apply
//! (one history step) happens through the normal command on release / close.

use crate::app_state::AppState;
use fp_engine::DocId;
use fp_model::element::ElementKind;
use fp_model::{Document, Element, ElementId};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Which property the preview changes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PreviewValue {
    /// Object opacity in percent (0..100), like `element.setOpacity`.
    Opacity(f64),
    /// Stroke (brush tip) size in pixels, like `pi.stroke {width}`.
    StrokeWidth(f64),
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
        match self.value {
            PreviewValue::Opacity(v) => (0u8, v.to_bits()).hash(&mut h),
            PreviewValue::StrokeWidth(v) => (1u8, v.to_bits()).hash(&mut h),
        }
        h.finish() | 1
    }

    /// A copy of `doc` with the value applied to the preview's elements (the original document is
    /// not changed; unchanged parts stay shared).
    pub fn apply(&self, doc: &Document) -> Document {
        let mut out = doc.clone();
        for &id in &self.ids {
            let _ = fp_engine::docops::edit_by_id(&mut out, self.page, id, |e| match self.value {
                PreviewValue::Opacity(v) => {
                    content_of(e).common.opacity = fp_color::Opacity::from_percent(v);
                }
                PreviewValue::StrokeWidth(v) => {
                    fp_engine::docops::edit_path_attrs(e, &|a| {
                        if let Some(b) = a.brush.as_mut() {
                            b.diameter = v.round().max(1.0) as i32;
                        }
                    });
                }
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

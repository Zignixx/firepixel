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
//!
//! FB-46: a pending value belongs to the objects it was started for. The pop-up slider and the
//! colour pop-up remember the selection when they open ([`PreviewTarget`]); a commit whose
//! selection changed in between is discarded, both when the pop-up decides to apply and when the
//! queued command runs ([`crate::app_state::Action::Commit`]). Undo, Redo and a jump in the History
//! panel first discard an open pop-up slider or colour pop-up like Esc ([`discard_pending`]) and
//! only then move in the history, so a pending value never lands on the objects the undo selects.

use crate::app_state::AppState;
use fp_color::Argb;
use fp_engine::DocId;
use fp_model::element::ElementKind;
use fp_model::{Document, Element, ElementId};
use std::cell::RefCell;
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

// ------------------------------------------------------------------ FB-46: preview targets

/// The objects a pending slider or colour pop-up value was started for (FB-46): document, page
/// and the selected element ids when the pop-up opened. `None` (as `Option<PreviewTarget>`)
/// stands for "no document".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewTarget {
    pub doc: DocId,
    pub page: usize,
    pub ids: Vec<ElementId>,
}

impl PreviewTarget {
    /// The active document's current selection (`None` without a document).
    pub fn of(st: &AppState) -> Option<PreviewTarget> {
        let d = st.session.active()?;
        Some(PreviewTarget {
            doc: d.id,
            page: d.page,
            ids: d.selection.elements.clone(),
        })
    }

    /// Does a commit started for `started` still apply to the selection `now`? Only when both are
    /// the same document, page and ids (FB-46: otherwise the value would land on other objects).
    pub fn still_applies(started: &Option<PreviewTarget>, now: &Option<PreviewTarget>) -> bool {
        started == now
    }
}

/// Memory key of the selection published for this frame ([`publish_target`]).
const CURRENT_TARGET: &str = "fp-preview-current-target";

/// Publishes the active selection for the widgets of this frame that have no access to the
/// application state (the pop-up sliders of [`super::fields`]). Called at the start of every frame
/// and again after a direct history jump.
pub fn publish_target(ctx: &egui::Context, st: &AppState) {
    let t = PreviewTarget::of(st);
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(CURRENT_TARGET), t));
}

/// The selection published by [`publish_target`] (`None` without a document or before the first
/// frame).
pub fn current_target(ctx: &egui::Context) -> Option<PreviewTarget> {
    ctx.data(|d| d.get_temp::<Option<PreviewTarget>>(egui::Id::new(CURRENT_TARGET)))
        .flatten()
}

thread_local! {
    /// Target of the pop-up commit made in this frame that has not reached a command yet: the next
    /// [`AppState::push`] of a command takes it and queues an [`crate::app_state::Action::Commit`].
    static COMMIT_STAMP: RefCell<Option<Option<PreviewTarget>>> = const { RefCell::new(None) };
}

/// A slider or colour pop-up applies its pending value now; the command the caller queues next
/// runs only while the selection is still `target` (FB-46).
pub fn stamp_commit(target: Option<PreviewTarget>) {
    COMMIT_STAMP.with(|c| *c.borrow_mut() = Some(target));
}

/// Takes the target of the pending pop-up commit ([`stamp_commit`]), if any.
pub fn take_commit_stamp() -> Option<Option<PreviewTarget>> {
    COMMIT_STAMP.with(|c| c.borrow_mut().take())
}

/// Forgets a pop-up commit that queued no command (start and end of every frame, an unchanged
/// value).
pub fn clear_commit_stamp() {
    COMMIT_STAMP.with(|c| *c.borrow_mut() = None);
}

/// Commands that move in the history: they first discard an open slider or colour pop-up
/// (FB-46).
pub fn moves_history(name: &str) -> bool {
    matches!(name, "edit.undo" | "edit.redo" | "history.goTo")
}

/// Closes an open pop-up slider and an open colour pop-up **without** applying their pending
/// value, like Esc, and ends the canvas preview (FB-46: before Undo, Redo or a History jump).
/// Returns whether a pop-up was open.
pub fn discard_pending(ctx: &egui::Context, st: &mut AppState) -> bool {
    let slider = super::fields::discard_open_slider(ctx);
    let colour = super::color_picker::discard_open(ctx);
    st.inspector_preview = None;
    slider || colour
}

/// Memory key of the History panel's area and the pass it was drawn in.
const HISTORY_RECT: &str = "fp-history-panel-rect";

/// The History panel was drawn at `r` in this pass (see [`discard_on_history_press`]).
pub fn note_history_rect(ctx: &egui::Context, r: egui::Rect) {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(HISTORY_RECT), (r, pass)));
}

/// At the start of a frame, before any widget closes a pop-up: a mouse press on the History panel
/// while a pop-up slider is open discards the pending value (FB-46). The click that follows then
/// jumps in the history with nothing pending; without this the click beside the pop-up would
/// apply the value in the same frame as the jump.
pub fn discard_on_history_press(ctx: &egui::Context, st: &mut AppState) {
    if !super::fields::slider_open(ctx) {
        return;
    }
    let press = ctx.input(|i| {
        if i.pointer.any_pressed() {
            i.pointer.press_origin()
        } else {
            None
        }
    });
    let Some(p) = press else {
        return;
    };
    let pass = ctx.cumulative_pass_nr();
    let on_history = ctx
        .data(|d| d.get_temp::<(egui::Rect, u64)>(egui::Id::new(HISTORY_RECT)))
        .is_some_and(|(r, drawn)| drawn + 1 >= pass && r.contains(p));
    if on_history {
        discard_pending(ctx, st);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(doc: u64, page: usize, ids: &[u64]) -> Option<PreviewTarget> {
        Some(PreviewTarget {
            doc: DocId(doc),
            page,
            ids: ids.iter().map(|i| ElementId(*i)).collect(),
        })
    }

    #[test]
    fn target_guard_applies_only_to_the_same_objects() {
        let a = target(1, 0, &[5, 7]);
        assert!(PreviewTarget::still_applies(&a, &target(1, 0, &[5, 7])));
        // undo selected another object, or the selection grew / shrank
        assert!(!PreviewTarget::still_applies(&a, &target(1, 0, &[9])));
        assert!(!PreviewTarget::still_applies(&a, &target(1, 0, &[5])));
        assert!(!PreviewTarget::still_applies(&a, &target(1, 0, &[])));
        // the same ids on another page or in another document are other objects
        assert!(!PreviewTarget::still_applies(&a, &target(1, 1, &[5, 7])));
        assert!(!PreviewTarget::still_applies(&a, &target(2, 0, &[5, 7])));
        assert!(!PreviewTarget::still_applies(&a, &None));
        assert!(PreviewTarget::still_applies(&None, &None));
    }

    #[test]
    fn commit_stamp_is_taken_once() {
        clear_commit_stamp();
        assert_eq!(take_commit_stamp(), None);
        stamp_commit(target(1, 0, &[3]));
        assert_eq!(take_commit_stamp(), Some(target(1, 0, &[3])));
        assert_eq!(take_commit_stamp(), None, "the next command is unguarded");
        stamp_commit(None);
        clear_commit_stamp();
        assert_eq!(take_commit_stamp(), None);
    }

    #[test]
    fn history_commands() {
        assert!(moves_history("edit.undo"));
        assert!(moves_history("edit.redo"));
        assert!(moves_history("history.goTo"));
        assert!(!moves_history("element.setOpacity"));
        assert!(!moves_history("history.replay"));
    }
}

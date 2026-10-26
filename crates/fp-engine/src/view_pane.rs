//! Split document view (FB-97 / TW-272): Window ▸ Duplicate Window shows the active document a
//! second time in a pane beside the first, each pane with its own view (zoom, scroll, preview
//! mode …), page, state, current layer and selection; both panes show the same document, so edits
//! appear in both.
//!
//! Only one pane is "live" at a time: its state sits in the document's usual fields
//! ([`DocumentState::view`], [`DocumentState::page`], [`DocumentState::frame`] …), so every
//! existing code path (commands, tools, panels, MCP, CLI, scripts, the parity runner) addresses the
//! focused pane unchanged. The other pane's state waits in [`SplitView::other`]. The UI swaps the
//! two while it draws the other pane ([`DocumentState::swap_pane`]) and swaps them back afterwards;
//! a real focus change goes through [`DocumentState::focus_pane`]. Nothing of the split is saved.

use crate::selection::Selection;
use crate::session::DocumentState;
use crate::view::ViewState;
use fp_model::LayerId;

/// The state of one pane that is not live (see the module documentation).
#[derive(Clone, Debug, PartialEq)]
pub struct ViewPane {
    pub view: ViewState,
    pub page: usize,
    pub page_selection: Vec<usize>,
    pub frame: usize,
    pub layer: Option<LayerId>,
    pub layer_choice: Option<(LayerId, LayerId)>,
    pub selection: Selection,
}

/// A document shown in two panes side by side (0 = left, 1 = right).
#[derive(Clone, Debug, PartialEq)]
pub struct SplitView {
    /// The pane whose state is in the document's live fields.
    pub live: usize,
    /// The state of the other pane.
    pub other: ViewPane,
}

impl DocumentState {
    /// Shown in two panes?
    pub fn is_split(&self) -> bool {
        self.split.is_some()
    }

    /// The pane whose state is in the live fields (0 without a split).
    pub fn live_pane(&self) -> usize {
        self.split.as_ref().map_or(0, |s| s.live)
    }

    /// The state of the pane that is not live (`None` without a split).
    pub fn other_pane(&self) -> Option<&ViewPane> {
        self.split.as_ref().map(|s| &s.other)
    }

    fn take_live_pane(&self) -> ViewPane {
        ViewPane {
            view: self.view.clone(),
            page: self.page,
            page_selection: self.page_selection.clone(),
            frame: self.frame,
            layer: self.layer,
            layer_choice: self.layer_choice,
            selection: self.selection.clone(),
        }
    }

    /// Splits the view: the right pane starts as a copy of the current one (same page, state,
    /// zoom and scroll) and gets the focus, like the new window of Duplicate Window. Returns
    /// `false` if the view was split already.
    pub fn split_view(&mut self) -> bool {
        if self.split.is_some() {
            return false;
        }
        self.split = Some(SplitView {
            live: 1,
            other: self.take_live_pane(),
        });
        true
    }

    /// Ends the split; the focused (live) pane stays. Returns `false` without a split.
    pub fn unsplit(&mut self) -> bool {
        self.split.take().is_some()
    }

    /// Exchanges the live state with the other pane's as it is (no clamping, no layer rules):
    /// the UI draws the other pane between two calls. Does nothing without a split.
    pub fn swap_pane(&mut self) {
        let Some(sp) = self.split.as_mut() else {
            return;
        };
        let o = &mut sp.other;
        std::mem::swap(&mut self.view, &mut o.view);
        std::mem::swap(&mut self.page, &mut o.page);
        std::mem::swap(&mut self.page_selection, &mut o.page_selection);
        std::mem::swap(&mut self.frame, &mut o.frame);
        std::mem::swap(&mut self.layer, &mut o.layer);
        std::mem::swap(&mut self.layer_choice, &mut o.layer_choice);
        std::mem::swap(&mut self.selection, &mut o.selection);
        sp.live = 1 - sp.live;
    }

    /// Gives `pane` the focus: its state becomes the live one. The pane's page, state, page
    /// selection, layer and selection are checked against the document, which may have changed
    /// in the other pane meanwhile (deleted pages, states, layers or objects). Returns `true` if
    /// the focus changed.
    pub fn focus_pane(&mut self, pane: usize) -> bool {
        if self.split.is_none() || pane > 1 || pane == self.live_pane() {
            return false;
        }
        self.remember_page_layer();
        self.swap_pane();
        let pages = self.doc.pages.len().max(1);
        let page_moved = self.page >= pages;
        self.page = self.page.min(pages - 1);
        self.frame = self.frame.min(self.frame_count() - 1);
        let (page, n) = (self.page, self.doc.pages.len());
        self.page_selection.retain(|&i| i < n && i != page);
        let doc = std::sync::Arc::clone(&self.doc);
        if page_moved
            || self
                .selection
                .elements
                .iter()
                .any(|&e| crate::docops::find_on_page(&doc, page, e).is_none())
        {
            self.selection.clear();
        }
        let layer_ok = self.layer.is_some_and(|l| {
            crate::docops::flat_layers(&doc, page, self.frame)
                .iter()
                .any(|f| f.id == l)
        });
        if page_moved || !layer_ok {
            self.layer_choice = None;
            self.enter_page_layer();
        }
        true
    }
}

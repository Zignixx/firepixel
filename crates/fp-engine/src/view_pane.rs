//! Split document view (FB-97 / TW-272): Window ▸ Duplicate Window shows the active document a
//! second time in a pane beside the first, each pane with its own view (zoom, scroll, preview
//! mode …), page, state, current layer and selection; both panes show the same document, so edits
//! appear in both.
//!
//! Only one pane is "live" at a time: its state sits in the document's usual fields
//! ([`DocumentState::view`], [`DocumentState::page`], [`DocumentState::frame`] …), so every
//! existing code path (commands, tools, panels, MCP, CLI, scripts, the parity runner) addresses the
//! focused pane unchanged. The other panes' states wait in [`SplitView::panes`]. The UI swaps a
//! parked pane in while it draws it ([`DocumentState::swap_with`]) and swaps back afterwards; a
//! real focus change goes through [`DocumentState::focus_pane`]. Nothing of the split is saved.
//!
//! F137: Duplicate Window on a pane of a split view splits that pane again, up to [`MAX_PANES`]
//! panes in one row (side by side) or one column (one above the other); Close Duplicate Window
//! closes the focused pane; the splitter positions are [`SplitLayout::bounds`].
//!
//! F132: the panes sit side by side (default) or one above the other
//! ([`SplitOrientation`], `window.splitOrientation`); orientation and splitter position live in
//! [`DocumentState::split_layout`], outside [`DocumentState::split`], so they are remembered for the
//! document for the rest of the session (also after the split was closed and opened again).

use crate::selection::Selection;
use crate::session::DocumentState;
use crate::view::ViewState;
use fp_model::LayerId;

/// How the two panes of a split view are arranged.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SplitOrientation {
    /// Side by side (pane 0 left, pane 1 right), the default of Window ▸ Duplicate Window.
    #[default]
    Vertical,
    /// One above the other (pane 0 top, pane 1 bottom).
    Horizontal,
}

impl SplitOrientation {
    /// Name used by `window.splitOrientation` (`vertical`, `horizontal`).
    pub fn name(self) -> &'static str {
        match self {
            SplitOrientation::Vertical => "vertical",
            SplitOrientation::Horizontal => "horizontal",
        }
    }

    /// From a `window.splitOrientation` name.
    pub fn from_name(s: &str) -> Option<SplitOrientation> {
        match s {
            "vertical" => Some(SplitOrientation::Vertical),
            "horizontal" => Some(SplitOrientation::Horizontal),
            _ => None,
        }
    }
}

/// Most panes a split view has (F137: Duplicate Window on a pane of a split adds a further pane).
pub const MAX_PANES: usize = 4;
/// Smallest share of a pane in the document area (along the orientation's axis).
pub const MIN_SHARE: f32 = 0.1;

/// Arrangement of a document's split view, remembered for the session (not saved).
#[derive(Clone, Debug, PartialEq)]
pub struct SplitLayout {
    pub orientation: SplitOrientation,
    /// Share of pane 0 (left or top) when two panes are shown: the splitter position a new split
    /// of the unsplit view starts with (mirrors `bounds[0]` while two panes are shown).
    pub ratio: f32,
    /// F137: positions of the splitters as shares of the document area (one per pane boundary,
    /// increasing, `pane count - 1` entries while split).
    pub bounds: Vec<f32>,
    /// F137: key of the pane shown while the view is not split (see [`SplitView::keys`]).
    pub solo_key: u64,
    /// F137: the next unused pane key.
    pub next_key: u64,
}

impl Default for SplitLayout {
    fn default() -> Self {
        SplitLayout {
            orientation: SplitOrientation::Vertical,
            ratio: 0.5,
            bounds: Vec::new(),
            solo_key: 0,
            next_key: 1,
        }
    }
}

impl SplitLayout {
    /// Spans (start, end as shares of the document area) of `n` panes, from
    /// [`SplitLayout::fitted_bounds`].
    pub fn spans(&self, n: usize) -> Vec<(f32, f32)> {
        let b = self.fitted_bounds(n);
        let mut out = Vec::with_capacity(n.max(1));
        let mut a = 0.0;
        for &x in &b {
            out.push((a, x));
            a = x;
        }
        out.push((a, 1.0));
        out
    }

    /// The splitter positions for `n` panes (`n - 1` entries): the stored ones if they fit,
    /// for two panes the remembered `ratio`, otherwise an even division.
    pub fn fitted_bounds(&self, n: usize) -> Vec<f32> {
        let n = n.max(1);
        let tol = MIN_SHARE - 1e-4;
        let ok = self.bounds.len() == n - 1
            && self.bounds.iter().all(|v| v.is_finite())
            && self
                .bounds
                .iter()
                .zip(std::iter::once(&0.0f32).chain(self.bounds.iter()))
                .all(|(b, a)| b - a >= tol)
            && self.bounds.last().is_none_or(|&l| 1.0 - l >= tol);
        if ok {
            return self.bounds.clone();
        }
        if n == 2 {
            return vec![self.ratio.clamp(MIN_SHARE, 1.0 - MIN_SHARE)];
        }
        (1..n).map(|i| i as f32 / n as f32).collect()
    }

    /// Moves splitter `i` of `n` panes to the share `v`, kept at least [`MIN_SHARE`] away from its
    /// neighbours and the edges. Returns `false` for an unknown splitter.
    pub fn set_bound(&mut self, n: usize, i: usize, v: f32) -> bool {
        let mut b = self.fitted_bounds(n);
        if i >= b.len() || !v.is_finite() {
            return false;
        }
        let lo = if i == 0 { 0.0 } else { b[i - 1] } + MIN_SHARE;
        let hi = b.get(i + 1).copied().unwrap_or(1.0) - MIN_SHARE;
        b[i] = v.clamp(lo, hi.max(lo));
        self.bounds = b;
        self.mirror();
        true
    }

    /// Pane `p` of `n` panes is split in two: a new splitter in the middle of its span (an even
    /// division when that span is too small for two panes).
    fn insert_after(&mut self, n: usize, p: usize) {
        if n <= 1 {
            self.bounds = vec![self.ratio.clamp(MIN_SHARE, 1.0 - MIN_SHARE)];
            return;
        }
        let spans = self.spans(n);
        let mut b = self.fitted_bounds(n);
        let (a, e) = spans.get(p).copied().unwrap_or((0.0, 1.0));
        b.insert(p.min(b.len()), (a + e) / 2.0);
        self.bounds = b;
        self.bounds = self.fitted_bounds(n + 1);
        self.mirror();
    }

    /// Pane `p` of `n` panes closes: its space goes to the pane before it (the first pane's to
    /// the pane after it).
    fn remove_pane(&mut self, n: usize, p: usize) {
        let mut b = self.fitted_bounds(n);
        if b.is_empty() {
            return;
        }
        let k = if p == 0 { 0 } else { (p - 1).min(b.len() - 1) };
        b.remove(k);
        self.bounds = b;
        self.mirror();
    }

    /// Keeps `ratio` equal to the splitter of a two-pane split.
    fn mirror(&mut self) {
        if self.bounds.len() == 1 {
            self.ratio = self.bounds[0];
        }
    }
}

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

/// A document shown in two to [`MAX_PANES`] panes (0 = left or top, counting right or down).
#[derive(Clone, Debug, PartialEq)]
pub struct SplitView {
    /// The pane whose state is in the document's live fields.
    pub live: usize,
    /// The parked state of every pane in order; `None` at `live`.
    pub panes: Vec<Option<ViewPane>>,
    /// F137: a stable key per pane (in order), kept while other panes open and close; the UI
    /// keeps a pane's canvas cache under it.
    pub keys: Vec<u64>,
}

impl DocumentState {
    /// Shown in two or more panes?
    pub fn is_split(&self) -> bool {
        self.split.is_some()
    }

    /// Number of panes (1 without a split).
    pub fn pane_count(&self) -> usize {
        self.split.as_ref().map_or(1, |s| s.panes.len())
    }

    /// The pane whose state is in the live fields (0 without a split).
    pub fn live_pane(&self) -> usize {
        self.split.as_ref().map_or(0, |s| s.live)
    }

    /// The parked state of the pane after the live one (cyclic; with two panes the other pane).
    /// `None` without a split.
    pub fn other_pane(&self) -> Option<&ViewPane> {
        let n = self.pane_count().max(1);
        self.parked_pane((self.live_pane() + 1) % n)
    }

    /// The parked state of pane `p` (`None` for the live pane, an unknown pane or no split).
    pub fn parked_pane(&self, p: usize) -> Option<&ViewPane> {
        self.split.as_ref()?.panes.get(p)?.as_ref()
    }

    /// F137: the stable key of pane `p` ([`SplitView::keys`]; the solo key without a split).
    pub fn pane_key(&self, p: usize) -> u64 {
        match &self.split {
            Some(s) => s.keys.get(p).copied().unwrap_or(u64::MAX),
            None => self.split_layout.solo_key,
        }
    }

    /// F137: page, frame and zoom shown in pane `p` (the live fields for the live pane).
    pub fn pane_summary(&self, p: usize) -> Option<(usize, usize, f64)> {
        if p == self.live_pane() {
            return Some((self.page, self.frame, self.view.zoom));
        }
        self.parked_pane(p).map(|v| (v.page, v.frame, v.view.zoom))
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

    fn put_live_pane(&mut self, v: ViewPane) -> ViewPane {
        ViewPane {
            view: std::mem::replace(&mut self.view, v.view),
            page: std::mem::replace(&mut self.page, v.page),
            page_selection: std::mem::replace(&mut self.page_selection, v.page_selection),
            frame: std::mem::replace(&mut self.frame, v.frame),
            layer: std::mem::replace(&mut self.layer, v.layer),
            layer_choice: std::mem::replace(&mut self.layer_choice, v.layer_choice),
            selection: std::mem::replace(&mut self.selection, v.selection),
        }
    }

    /// Splits the focused pane: a new pane after it starts as a copy of it (same page, state,
    /// zoom and scroll) and gets the focus, like the new window of Duplicate Window; the focused
    /// pane's space is shared between the two (F137). Returns `false` at [`MAX_PANES`] panes.
    pub fn split_view(&mut self) -> bool {
        let n = self.pane_count();
        if n >= MAX_PANES {
            return false;
        }
        let live = self.live_pane();
        let parked = self.take_live_pane();
        self.split_layout.insert_after(n, live);
        let key = self.split_layout.next_key;
        self.split_layout.next_key += 1;
        match self.split.as_mut() {
            Some(sp) => {
                sp.panes[live] = Some(parked);
                sp.panes.insert(live + 1, None);
                sp.keys.insert(live + 1, key);
                sp.live = live + 1;
            }
            None => {
                self.split = Some(SplitView {
                    live: 1,
                    panes: vec![Some(parked), None],
                    keys: vec![self.split_layout.solo_key, key],
                });
            }
        }
        true
    }

    /// Ends the split; the focused (live) pane stays. Returns `false` without a split.
    pub fn unsplit(&mut self) -> bool {
        let Some(sp) = self.split.take() else {
            return false;
        };
        if let Some(&k) = sp.keys.get(sp.live) {
            self.split_layout.solo_key = k;
        }
        true
    }

    /// F137: closes pane `p`. Closing the live pane gives the focus to the pane before it (the
    /// first pane's to the pane after it) first; with one pane left the split ends. Returns
    /// `false` without a split or for an unknown pane.
    pub fn close_pane(&mut self, p: usize) -> bool {
        let n = self.pane_count();
        if n < 2 || p >= n {
            return false;
        }
        if p == self.live_pane() {
            let to = if p == 0 { 1 } else { p - 1 };
            self.focus_pane(to);
        }
        self.split_layout.remove_pane(n, p);
        if n == 2 {
            return self.unsplit();
        }
        let Some(sp) = self.split.as_mut() else {
            return false;
        };
        sp.panes.remove(p);
        if p < sp.keys.len() {
            sp.keys.remove(p);
        }
        if sp.live > p {
            sp.live -= 1;
        }
        true
    }

    /// Exchanges the live state with parked pane `p` as it is (no clamping, no layer rules):
    /// afterwards `p` is live and the previous live pane is parked. The UI draws a parked pane
    /// between two calls. Does nothing without a split, for the live pane or an unknown pane.
    pub fn swap_with(&mut self, p: usize) {
        let Some(v) = self
            .split
            .as_mut()
            .filter(|s| p != s.live)
            .and_then(|s| s.panes.get_mut(p))
            .and_then(Option::take)
        else {
            return;
        };
        let old = self.put_live_pane(v);
        if let Some(sp) = self.split.as_mut() {
            let l = sp.live;
            sp.panes[l] = Some(old);
            sp.live = p;
        }
    }

    /// Exchanges the live state with the pane after it (with two panes: the other pane), see
    /// [`DocumentState::swap_with`].
    pub fn swap_pane(&mut self) {
        let n = self.pane_count();
        if n > 1 {
            self.swap_with((self.live_pane() + 1) % n);
        }
    }

    /// Gives `pane` the focus: its state becomes the live one. The pane's page, state, page
    /// selection, layer and selection are checked against the document, which may have changed
    /// in another pane meanwhile (deleted pages, states, layers or objects). Returns `true` if
    /// the focus changed.
    pub fn focus_pane(&mut self, pane: usize) -> bool {
        if self.split.is_none() || pane >= self.pane_count() || pane == self.live_pane() {
            return false;
        }
        self.remember_page_layer();
        self.swap_with(pane);
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

//! Object selection: optional mask preview while hovering (BA-037).
//!
//! With the preference `AiObjectHoverPreview` (default off) and the object-selection tool active,
//! the pointer resting on one pixel for [`DEBOUNCE`] seconds starts `select.aiObject` for that
//! point on a background thread against a copy of the pixels ([`eng::hover_plan`]). Only with a
//! warm model cache: the model must be loaded and the image embedding cached, otherwise nothing is
//! computed. The resulting object outline is drawn dashed; a click on the same pixel reuses the
//! computed decoder output and is instant.
//!
//! At most one job runs at a time. Moving the pointer, a document change, a tool change, pressing
//! the mouse button or leaving the canvas cancels the outstanding job; results that arrive for an
//! older pointer position are dropped. The UI thread never waits for a job.

use crate::app_state::AppState;
use crate::theme::Theme;
use egui::{Painter, Rect};
use fp_ai::job::Job;
use fp_engine::ToolId;
use fp_engine::commands::ai as eng;
use fp_engine::commands::ai::fp_ai;
use fp_engine::selection::SelectMode;
use fp_geom::Point;
use fp_raster::Mask;

/// Preference key (technical, English).
pub const PREF_KEY: &str = "AiObjectHoverPreview";

/// Rest time of the pointer before a preview job starts (seconds).
pub const DEBOUNCE: f64 = 0.12;

/// Everything a hover position depends on: document state, pointer pixel and visible rect (the
/// rect decides the automatic region of the model).
type HoverKey = (u64, u64, usize, usize, i64, i64, [u32; 4]);

/// Counters for tests and diagnostics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HoverStats {
    /// Jobs started.
    pub started: u64,
    /// Results dropped because the pointer (or document) had moved on.
    pub dropped: u64,
    /// Results shown.
    pub shown: u64,
}

/// Hover preview state (lives in [`crate::dialogs::models::AiDialogs`]).
#[derive(Default)]
pub struct ObjectHover {
    key: Option<HoverKey>,
    /// Input time when the pointer arrived at `key`.
    since: f64,
    /// Generation of the current position; a result of another generation is stale.
    generation: u64,
    /// Planned already for this generation (cold cache: do not retry every frame).
    tried: bool,
    job: Option<(u64, Job<Option<Mask>>)>,
    shown: Option<Mask>,
    ants: Option<super::super::overlay::AntsCache>,
    stats: HoverStats,
}

impl ObjectHover {
    /// Is an outline shown right now?
    pub fn is_shown(&self) -> bool {
        self.shown.is_some()
    }
    /// Mask of the shown outline.
    pub fn shown_mask(&self) -> Option<&Mask> {
        self.shown.as_ref()
    }
    /// Is a job running (or cancelled and not finished yet)?
    pub fn job_running(&self) -> bool {
        self.job.is_some()
    }
    /// Counters.
    pub fn stats(&self) -> HoverStats {
        self.stats
    }

    /// Cancel the outstanding job and hide the outline. The cancelled job stays in its slot until
    /// it has ended (cap of one concurrent job); its result is dropped.
    pub fn cancel(&mut self) {
        if let Some((_, j)) = &self.job {
            j.cancel();
        }
        if self.key.is_some() || self.shown.is_some() {
            self.generation += 1;
        }
        self.key = None;
        self.tried = false;
        self.shown = None;
        self.ants = None;
    }

    /// Collect a finished job; stale results are dropped.
    fn poll(&mut self) {
        let Some((g, job)) = self.job.as_mut() else {
            return;
        };
        let Some(r) = job.poll() else { return };
        let g = *g;
        self.job = None;
        if g != self.generation {
            self.stats.dropped += 1;
            return;
        }
        if let Ok(Some(m)) = r {
            self.shown = Some(m);
            self.ants = None;
            self.stats.shown += 1;
        }
    }
}

/// Per frame (after the canvas input): advance debounce, jobs and results. `pointer_here` = the
/// pointer is over the canvas area.
pub fn update(ctx: &egui::Context, st: &mut AppState, pointer_here: bool) {
    st.dialogs.ai.hover.poll();
    let Some(d) = st.session.active() else {
        st.dialogs.ai.hover.cancel();
        return;
    };
    let pointer_down = st.canvases.get(&d.id).is_some_and(|c| c.pointer_down);
    let enabled = st.session.prefs.get_bool(PREF_KEY, false)
        && st.session.tools.active == ToolId::AiObject
        && pointer_here
        && !pointer_down
        && !st.dialogs.any_open()
        && !st.dialogs.ai.busy();
    let pos = st.pointer_doc;
    let (Some(pos), true) = (pos, enabled) else {
        st.dialogs.ai.hover.cancel();
        if st.dialogs.ai.hover.job.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
        return;
    };
    let vis = d.view.visible_rect();
    let key: HoverKey = (
        d.id.0,
        d.generation,
        d.page,
        d.frame,
        pos.x.floor() as i64,
        pos.y.floor() as i64,
        [
            vis.left.to_bits(),
            vis.top.to_bits(),
            vis.right.to_bits(),
            vis.bottom.to_bits(),
        ],
    );
    let time = ctx.input(|i| i.time);
    let h = &mut st.dialogs.ai.hover;
    if h.key != Some(key) {
        let same_doc = h
            .key
            .is_some_and(|k| k.0 == key.0 && k.1 == key.1 && k.2 == key.2 && k.3 == key.3);
        if let Some((_, j)) = &h.job {
            j.cancel();
        }
        h.generation += 1;
        h.key = Some(key);
        h.since = time;
        h.tried = false;
        // still over the shown object: keep its outline until the new one is there
        let inside = h
            .shown
            .as_ref()
            .is_some_and(|m| m.get(key.4 as i32, key.5 as i32) >= 128);
        if !(same_doc && inside) {
            h.shown = None;
            h.ants = None;
        }
    }
    let rest = time - h.since;
    if !h.tried && h.job.is_none() && rest >= DEBOUNCE {
        h.tried = true;
        let params = fp_engine::tools::object_click_params(pos, SelectMode::Replace, vis);
        if let Ok(Some(plan)) = eng::hover_plan(&st.session, &params) {
            let h = &mut st.dialogs.ai.hover;
            let g = h.generation;
            let job = Job::spawn("hover", move |p| plan.compute(p));
            h.job = Some((g, job));
            h.stats.started += 1;
        }
    }
    let h = &st.dialogs.ai.hover;
    if h.job.is_some() {
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    } else if !h.tried {
        let wait = (DEBOUNCE - rest).max(0.0) + 0.001;
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(wait));
    }
}

/// Draw the preview outline dashed (after the tool overlays).
pub fn paint(
    ui: &egui::Ui,
    st: &mut AppState,
    painter: &Painter,
    area: Rect,
    view: &fp_engine::ViewState,
) {
    let h = &mut st.dialogs.ai.hover;
    let Some(m) = h.shown.as_ref() else { return };
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let time = ui.input(|i| i.time);
    let to_screen = |p: Point| super::super::to_screen(view, area.min, p.x as f64, p.y as f64);
    let outlines = super::super::overlay::ants_for(&mut h.ants, m);
    super::super::overlay::paint_ants(
        painter,
        outlines,
        &to_screen,
        time,
        c.marquee_a,
        c.marquee_b,
        area,
    );
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_millis(120));
}

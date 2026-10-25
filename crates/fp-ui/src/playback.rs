//! Animation playback on the canvas (FB-63, TW-239): while `ui.playing` (View ▸ Play/Stop, the
//! Play button of the status bar, its shortcut) the active document advances to the next state
//! after the shown state's own delay (1/100 s, default 7), looping per the document's loop
//! setting (0 = endless, -1 = once, k = k repeats after the first pass, like the APNG/GIF export).
//!
//! State switching is a view change ([`fp_engine::Session::show_state`]): no History rows. The
//! playback stops on toggle, when another document becomes active and when the state list
//! changes (states added, deleted, reordered, renamed).

use crate::app_state::AppState;
use fp_engine::DocId;

/// Running playback (kept in [`AppState::playback`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Playback {
    /// Document the playback was started on.
    doc: Option<DocId>,
    /// Signature of the state list at the start (count and names).
    states: Vec<Option<String>>,
    /// UI time when the next state is due.
    due: Option<f64>,
    /// Completed passes through the states.
    passes: u32,
}

/// Delay of state `i` of the active document in seconds (stored per state in 1/100 s).
fn delay_secs(st: &AppState, i: usize) -> f64 {
    let d = st
        .session
        .active()
        .and_then(|d| d.doc.pages.get(d.page))
        .and_then(|p| p.root.export.as_ref())
        .and_then(|e| e.frames.get(i).cloned())
        .unwrap_or_default()
        .delay;
    d.max(1) as f64 / 100.0
}

/// Count and names of the states of the active document's page.
fn state_list(st: &AppState) -> Vec<Option<String>> {
    let Some(d) = st.session.active() else {
        return Vec::new();
    };
    let frames = d
        .doc
        .pages
        .get(d.page)
        .and_then(|p| p.root.export.as_ref())
        .map(|e| e.frames.as_slice())
        .unwrap_or(&[]);
    (0..d.frame_count())
        .map(|i| frames.get(i).and_then(|f| f.name.clone()))
        .collect()
}

/// Stops the playback (the toggle shows "Play" again).
fn stop(st: &mut AppState) {
    st.session.ui.playing = false;
    st.playback = Playback::default();
}

/// One step of the playback driver; called once per UI frame.
pub fn frame(ctx: &egui::Context, st: &mut AppState) {
    if !st.session.ui.playing {
        st.playback = Playback::default();
        return;
    }
    let Some((id, cur, n, loop_count)) = st.session.active().map(|d| {
        (
            d.id,
            d.frame,
            d.frame_count(),
            d.doc.pages.get(d.page).map(|p| p.loop_count()).unwrap_or(0),
        )
    }) else {
        stop(st);
        return;
    };
    let now = ctx.input(|i| i.time);
    let states = state_list(st);
    match st.playback.doc {
        None => {
            if n < 2 {
                stop(st);
                return;
            }
            st.playback = Playback {
                doc: Some(id),
                states,
                due: Some(now + delay_secs(st, cur)),
                passes: 0,
            };
        }
        Some(doc) if doc != id || states != st.playback.states => {
            stop(st);
            return;
        }
        Some(_) => {}
    }
    let due = st.playback.due.unwrap_or(now);
    if now + 1e-6 >= due {
        let next = (cur + 1) % n;
        if next == 0 {
            st.playback.passes += 1;
            let passes = st.playback.passes;
            let total = match loop_count {
                0 => None,
                k if k < 0 => Some(1),
                k => Some(k as u32 + 1),
            };
            if total.is_some_and(|t| passes >= t) {
                // played to the end: stay on the last state
                stop(st);
                return;
            }
        }
        st.session.show_state(next);
        st.playback.due = Some(now + delay_secs(st, next));
    }
    let wait = st.playback.due.map(|d| (d - now).max(0.0)).unwrap_or(0.0);
    ctx.request_repaint_after(std::time::Duration::from_secs_f64(wait));
}

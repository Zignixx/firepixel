//! Animation playback on the canvas (FB-63, TW-239): while `ui.playing` (View ▸ Play/Stop, the
//! Play button of the status bar, its shortcut) the active document advances to the next state
//! after the shown state's own delay (1/100 s, default 7) and loops endlessly: the document's
//! loop setting applies to the GIF/APNG export only (RE spec 72 §3, FB-87). Starting jumps to the
//! next state at once.
//!
//! States excluded from export ("Include when exporting" off) are skipped, their delay does not
//! count (RE spec 72 §3).
//!
//! State switching is a view change ([`fp_engine::Session::show_state`]): no History rows. The
//! playback stops on toggle, when another document becomes active, when the document is closed
//! and when the state list changes (states added, deleted, reordered, renamed).

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

/// Which of the `n` states of the active document's page are part of the animation (shown when
/// exporting, [`fp_model::FrameInfo::visible`]); states without stored properties count as included.
fn included_states(st: &AppState, n: usize) -> Vec<bool> {
    let frames = st
        .session
        .active()
        .and_then(|d| d.doc.pages.get(d.page))
        .and_then(|p| p.root.export.as_ref())
        .map(|e| e.frames.clone())
        .unwrap_or_default();
    (0..n)
        .map(|i| frames.get(i).is_none_or(|f| f.visible))
        .collect()
}

/// The state after `cur` among the `included` ones (cyclic); `None` when fewer than two states are
/// included (nothing to play).
pub fn next_included_state(cur: usize, included: &[bool]) -> Option<usize> {
    let n = included.len();
    if included.iter().filter(|v| **v).count() < 2 {
        return None;
    }
    (1..=n).map(|k| (cur + k) % n).find(|i| included[*i])
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
    let Some((id, cur, n)) = st
        .session
        .active()
        .map(|d| (d.id, d.frame, d.frame_count()))
    else {
        stop(st);
        return;
    };
    let now = ctx.input(|i| i.time);
    let states = state_list(st);
    let included = included_states(st, n);
    match st.playback.doc {
        None => {
            if next_included_state(cur, &included).is_none() {
                stop(st);
                return;
            }
            // RE spec 72 §3.2: Play shows the next included state at once
            st.playback = Playback {
                doc: Some(id),
                states,
                due: Some(now),
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
        let Some(next) = next_included_state(cur, &included) else {
            stop(st);
            return;
        };
        st.session.show_state(next);
        st.playback.due = Some(now + delay_secs(st, next));
    }
    let wait = st.playback.due.map(|d| (d - now).max(0.0)).unwrap_or(0.0);
    ctx.request_repaint_after(std::time::Duration::from_secs_f64(wait));
}

#[cfg(test)]
mod tests {
    use super::next_included_state;

    #[test]
    fn excluded_states_are_skipped() {
        // states 1 and 3 excluded from export (FB-87, RE spec 72 §3)
        let inc = [true, false, true, false];
        assert_eq!(next_included_state(0, &inc), Some(2));
        assert_eq!(next_included_state(2, &inc), Some(0));
        // started on an excluded state: on to the next included one
        assert_eq!(next_included_state(1, &inc), Some(2));
        assert_eq!(next_included_state(3, &inc), Some(0));
        // fewer than two included states: nothing to play
        assert_eq!(next_included_state(0, &[true, false, false]), None);
        assert_eq!(next_included_state(0, &[true, true]), Some(1));
    }
}

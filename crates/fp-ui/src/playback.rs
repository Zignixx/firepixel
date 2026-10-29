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

/// Longest lag behind the schedule that playback catches up on (seconds); a longer hold-up (a slow
/// render, a hidden window) restarts the schedule instead.
const MAX_LAG: f64 = 0.25;

/// When the state shown at `now` (it was due at `due`) hands over to the next one, `delay` seconds
/// later: on the schedule (`due + delay`), so the lateness of the frame that showed it (timer
/// granularity, vsync, render time) does not add up over a pass (T144: each state lasted up to one
/// UI frame longer than its delay). States shorter than a UI frame are still all shown, one per
/// frame, and the time is made up on the longer states that follow. Lagging more than
/// [`MAX_LAG`] behind, the schedule restarts from `now`: no burst of states to catch up.
pub fn next_due(due: f64, now: f64, delay: f64) -> f64 {
    let on_schedule = due + delay;
    if now - on_schedule > MAX_LAG {
        now + delay
    } else {
        on_schedule
    }
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
        st.playback.due = Some(next_due(due, now, delay_secs(st, next)));
    }
    let wait = st.playback.due.map(|d| (d - now).max(0.0)).unwrap_or(0.0);
    ctx.request_repaint_after(std::time::Duration::from_secs_f64(wait));
}

#[cfg(test)]
mod tests {
    use super::{next_due, next_included_state};

    #[test]
    fn schedule_does_not_drift() {
        // frames every 1/60 s, delay 0.07 s: each switch fires up to a frame late, but the
        // schedule keeps the average (T144)
        let dt = 1.0 / 60.0;
        let delay = 0.07;
        let (mut now, mut due) = (0.0f64, 0.0f64);
        let mut switches = Vec::new();
        while switches.len() < 100 {
            if now + 1e-6 >= due {
                switches.push(now);
                due = next_due(due, now, delay);
            }
            now += dt;
        }
        let span = switches[99] - switches[0];
        assert!((span - 99.0 * delay).abs() <= dt + 1e-9, "span {span}");
        // a long hold-up restarts the schedule instead of catching up
        assert_eq!(next_due(1.0, 2.0, 0.1), 2.1);
        assert_eq!(next_due(1.0, 1.05, 0.1), 1.1);
        // a short lag stays on the schedule: the next frame shows the next state at once
        assert_eq!(next_due(1.0, 1.15, 0.1), 1.1);
    }

    #[test]
    fn states_shorter_than_a_frame_keep_the_pass_time() {
        // UI frames of 1/30 s (a remote session), delays 0.02 / 0.03 / 0.04 / 0.05 s (T144
        // corpus file 2): every state is shown, the 0.14 s cycle is kept on average
        let dt = 1.0 / 30.0;
        let delays = [0.02, 0.03, 0.04, 0.05];
        let (mut now, mut due, mut k) = (0.0f64, 0.0f64, 0usize);
        let mut switches = Vec::new();
        while switches.len() < 161 {
            if now + 1e-6 >= due {
                switches.push(now);
                due = next_due(due, now, delays[k % 4]);
                k += 1;
            }
            now += dt;
        }
        let span = switches[160] - switches[0];
        assert!((span - 40.0 * 0.14).abs() <= dt + 1e-9, "span {span}");
    }

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

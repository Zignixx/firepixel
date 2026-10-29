//! F146 (FB-129, TW-281): while the playback runs on the canvas, the coming states are rendered
//! ahead on the canvas render thread, so that a state is on screen in the frame it becomes due
//! (before: the old state stayed until the render job of the new one came back, one full render
//! after its due time). These tests use the real render thread (`sync_render` off) and wait for it
//! in wall time. Run with `FP_NO_SHELL_OPEN=1` like all test runs.

use egui::vec2;
use egui_kittest::Harness;
use fp_ui::FirepixelApp;
use fp_ui::app_state::{Action, AppState, UiSettings};
use fp_ui::canvas::render::{CanvasCache, RenderKey};
use fp_ui::theme::ThemeKind;
use serde_json::json;
use std::sync::Mutex;
use std::time::{Duration, Instant};

static LOCK: Mutex<()> = Mutex::new(());

type H = Harness<'static, FirepixelApp>;

/// 160 × 120 document with four states (a rectangle moved per state), delays 3 s, state 2
/// excluded from export.
fn session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute(
        "file.new",
        json!({"width": 160, "height": 120, "background": "#ffffff"}),
    )
    .expect("document");
    s.execute("state.add", json!({"count": 3})).expect("states");
    for i in 0..4 {
        s.execute("state.setCurrent", json!({"index": i}))
            .expect("state");
        s.execute(
            "draw.rectangle",
            json!({"rect": [10 + 30 * i, 20, 35 + 30 * i, 45]}),
        )
        .expect("rectangle");
        s.execute(
            "pi.state.props",
            json!({"indices": [i], "delay": 300, "visible": i != 2}),
        )
        .expect("props");
    }
    s.execute("state.setCurrent", json!({"index": 0}))
        .expect("state 1");
    s
}

fn harness() -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(
        &ctx,
        session(),
        UiSettings {
            theme: ThemeKind::Normal,
            ..UiSettings::default()
        },
        false,
    );
    app.state.sync_render = false;
    app.state.os_clipboard = false;
    let mut h = Harness::builder()
        .with_size(vec2(1200.0, 800.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(8)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    // the first image of the current state from the render thread
    wait_until(&mut h, |st| {
        shown(st).is_some_and(|k| Some(k.frame) == cur(st))
    });
    h
}

fn st(h: &mut H) -> &mut AppState {
    &mut h.state_mut().state
}

fn cur(st: &AppState) -> Option<usize> {
    st.session.active().map(|d| d.frame)
}

fn cache(st: &AppState) -> Option<&CanvasCache> {
    let id = st.session.active()?.id;
    st.canvases.get(&id)
}

fn shown(st: &AppState) -> Option<RenderKey> {
    cache(st)?.shown
}

fn prefetched(st: &AppState) -> Vec<usize> {
    cache(st).map_or(Vec::new(), |c| {
        c.prefetched.iter().map(|r| r.key.frame).collect()
    })
}

/// One UI frame at a time (1/60 s of UI time), waiting a little wall time for the render thread,
/// until `f` holds (at most 10 s wall time).
fn wait_until(h: &mut H, f: impl Fn(&AppState) -> bool) {
    let t = Instant::now();
    while !f(&h.state().state) {
        assert!(t.elapsed() < Duration::from_secs(10), "timed out");
        h.run_steps(1);
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn play(h: &mut H) {
    st(h).push(Action::Run("view.togglePlayback".into(), json!({})));
    h.run_steps(1);
}

#[test]
fn prefetch_of_the_next_states_skips_excluded_ones() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    fp_engine::shell::disable();
    let mut h = harness();
    // nothing is pre-rendered without playback
    h.run_steps(5);
    assert!(prefetched(&h.state().state).is_empty(), "no playback, no prefetch");
    play(&mut h);
    assert!(st(&mut h).session.ui.playing);
    // Play shows state 1 at once; once it is on the canvas, states 3 and 0 are rendered ahead
    // (state 2 is excluded from export and skipped)
    wait_until(&mut h, |st| shown(st).is_some_and(|k| k.frame == 1));
    wait_until(&mut h, |st| prefetched(st).len() == 2);
    let st0 = &h.state().state;
    assert_eq!(cur(st0), Some(1));
    assert_eq!(prefetched(st0), vec![3, 0], "next included states in play order");
    // the key of the shown image with another state
    let k = shown(st0).expect("shown");
    let c = cache(st0).expect("cache");
    for r in &c.prefetched {
        assert_eq!(RenderKey { frame: k.frame, ..r.key }, k, "same key except the state");
        assert_eq!(r.image.width() as u32, k.dev.width(), "image of the canvas size");
    }
    assert!(c.prefetching.is_empty(), "no request on the way");
}

#[test]
fn due_state_is_shown_from_the_prefetch_in_the_same_frame() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    fp_engine::shell::disable();
    let mut h = harness();
    play(&mut h);
    wait_until(&mut h, |st| prefetched(st).len() == 2);
    let want = cache(&h.state().state).expect("cache").prefetched[0].key;
    assert_eq!(want.frame, 3);
    // run to the due instant of state 3: the frame that makes it current shows it
    let t = Instant::now();
    while cur(&h.state().state) == Some(1) {
        assert!(t.elapsed() < Duration::from_secs(20), "state 3 due");
        h.run_steps(1);
    }
    let s = &h.state().state;
    assert_eq!(cur(s), Some(3));
    assert_eq!(shown(s), Some(want), "on the canvas in the due frame");
    let c = cache(s).expect("cache");
    assert_eq!(c.requested, Some(want), "no new render job for the shown state");
    // the next prefetch (state 0, done) stays; the one after (state 1) is requested
    assert_eq!(c.prefetched.iter().map(|r| r.key.frame).collect::<Vec<_>>(), [0]);
    assert_eq!(
        c.prefetching.iter().map(|(k, _)| k.frame).collect::<Vec<_>>(),
        [1]
    );
    // the state stays on the canvas in the following frames (no render of it again)
    h.run_steps(3);
    let s = &h.state().state;
    assert_eq!(shown(s), Some(want), "still the prefetched image");
}

#[test]
fn stale_prefetches_are_dropped() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    fp_engine::shell::disable();
    let mut h = harness();
    play(&mut h);
    wait_until(&mut h, |st| prefetched(st).len() == 2);
    // an edit during playback (new generation): the old images are never shown
    let old_gen = shown(&h.state().state).expect("shown").generation;
    st(&mut h)
        .session
        .execute(
            "draw.rectangle",
            json!({"rect": [5, 80, 25, 100]}),
        )
        .expect("edit");
    h.run_steps(1);
    let s = &h.state().state;
    assert!(s.session.ui.playing, "an edit does not stop the playback");
    let c = cache(s).expect("cache");
    assert!(
        c.prefetched.iter().all(|r| r.key.generation != old_gen),
        "stale prefetch dropped"
    );
    assert!(c.prefetching.iter().all(|(k, _)| k.generation != old_gen));
    // the new state images come again
    wait_until(&mut h, |st| prefetched(st).len() == 2);
    let gen_now = st(&mut h).session.active().expect("doc").generation;
    assert!(
        cache(&h.state().state)
            .expect("cache")
            .prefetched
            .iter()
            .all(|r| r.key.generation == gen_now)
    );
    // Stop (the action runs at the end of the frame): all prefetches go in the next frame, none
    // comes back
    play(&mut h);
    h.run_steps(1);
    let s = &h.state().state;
    assert!(!s.session.ui.playing, "stopped");
    let c = cache(s).expect("cache");
    assert!(c.prefetched.is_empty() && c.prefetching.is_empty(), "dropped on Stop");
    for _ in 0..20 {
        h.run_steps(1);
        std::thread::sleep(Duration::from_millis(2));
    }
    let c = cache(&h.state().state).expect("cache");
    assert!(c.prefetched.is_empty() && c.prefetching.is_empty());
}

#[test]
fn zoom_during_playback_drops_the_prefetches_and_keeps_playing() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    fp_engine::shell::disable();
    let mut h = harness();
    play(&mut h);
    wait_until(&mut h, |st| prefetched(st).len() == 2);
    let z0 = shown(&h.state().state).expect("shown").zoom;
    st(&mut h)
        .session
        .execute("view.zoomIn", json!({}))
        .expect("zoom");
    h.run_steps(1);
    let s = &h.state().state;
    assert!(s.session.ui.playing);
    assert!(
        cache(s)
            .expect("cache")
            .prefetched
            .iter()
            .all(|r| (r.key.zoom - z0).abs() > 1e-9),
        "prefetches of the old zoom dropped"
    );
    // the new zoom is shown, then prefetched again
    wait_until(&mut h, |st| {
        shown(st).is_some_and(|k| (k.zoom - z0).abs() > 1e-9) && prefetched(st).len() == 2
    });
}

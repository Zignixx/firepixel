//! FB-63 (TW-239): animation playback on the canvas (status-bar Play runs View ▸ Play/Stop, the
//! state advances after each state's own delay, loops per the loop setting, no History rows) and
//! the Image Preview playing documents whose states take longer to render than their delay (the
//! next state is shown only once rendered, no job is cancelled per state change). Run with
//! `FP_NO_SHELL_OPEN=1` like all test runs.

use egui::vec2;
use egui_kittest::Harness;
use egui_kittest::kittest::{By, Queryable};
use fp_ui::FirepixelApp;
use fp_ui::app_state::{Action, AppState, UiSettings};
use fp_ui::dialogs::image_preview::{ImagePreview, worker};
use fp_ui::theme::ThemeKind;
use serde_json::json;
use std::sync::Mutex;
use std::time::{Duration, Instant};

static LOCK: Mutex<()> = Mutex::new(());

struct Guard(#[allow(dead_code)] std::sync::MutexGuard<'static, ()>);

impl Drop for Guard {
    fn drop(&mut self) {
        worker::hold_jobs(false);
        fp_i18n::set_language("de");
    }
}

fn lock() -> Guard {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    fp_i18n::set_language("de");
    fp_engine::shell::disable();
    Guard(g)
}

type H = Harness<'static, FirepixelApp>;

/// 120 × 80 document with a rectangle and three states with delays 10, 30 and 20 (1/100 s).
fn session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute(
        "file.new",
        json!({"width": 120, "height": 80, "background": "#ffffff"}),
    )
    .expect("document");
    s.execute(
        "draw.rectangle",
        json!({"x": 10, "y": 10, "width": 40, "height": 30, "fill": "#ff0000"}),
    )
    .ok();
    s.execute("state.add", json!({"count": 2})).expect("states");
    for (i, d) in [(0, 10), (1, 30), (2, 20)] {
        s.execute("state.setCurrent", json!({"index": i}))
            .expect("state");
        s.execute("pi.state.props", json!({"indices": [i], "delay": d}))
            .expect("delay");
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
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder()
        .with_size(vec2(1400.0, 900.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(8)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h
}

fn st(h: &mut H) -> &mut AppState {
    &mut h.state_mut().state
}

fn frame(h: &mut H) -> usize {
    st(h)
        .session
        .active()
        .map(|d| d.frame)
        .unwrap_or(usize::MAX)
}

fn history_rows(h: &mut H) -> usize {
    st(h)
        .session
        .active()
        .map(|d| d.history.len() + d.history.dropped())
        .unwrap_or(0)
}

fn click(h: &mut H, label: &str) {
    h.query_all(By::new().label(label))
        .next()
        .unwrap_or_else(|| panic!("{label} not found"))
        .click();
    h.run_steps(1);
}

fn delays(s: &fp_engine::Session) -> Vec<i32> {
    let d = s.active().expect("doc");
    let e = d.doc.pages[d.page]
        .root
        .export
        .as_ref()
        .expect("export settings");
    (0..3).map(|i| e.frames[i].delay).collect()
}

/// Runs steps until `frame` changes `n` times; returns the frames seen and the switch times.
fn observe(h: &mut H, steps: usize) -> (Vec<usize>, Vec<f64>) {
    let mut seen = vec![frame(h)];
    let mut at = Vec::new();
    for _ in 0..steps {
        h.run_steps(1);
        let f = frame(h);
        if Some(&f) != seen.last() {
            seen.push(f);
            at.push(h.ctx.input(|i| i.time));
        }
    }
    (seen, at)
}

#[test]
fn canvas_play_button_plays_states_with_their_delays() {
    let _g = lock();
    let mut h = harness();
    assert_eq!(delays(&st(&mut h).session), vec![10, 30, 20]);
    // a selection that the state switches clear: still no History row
    st(&mut h).push(Action::Run("select.all".into(), json!({})));
    h.run_steps(2);
    let rows = history_rows(&mut h);
    assert!(!st(&mut h).session.ui.playing);
    click(&mut h, &fp_i18n::t("ui-state-play"));
    assert!(
        st(&mut h).session.ui.playing,
        "Play runs view.togglePlayback"
    );
    // ~2.4 s of UI time: two passes through 0.10 + 0.30 + 0.20 s
    let (seen, at) = observe(&mut h, 144);
    assert!(seen.len() >= 7, "frames {seen:?}");
    assert_eq!(&seen[..7], &[0, 1, 2, 0, 1, 2, 0], "loops endlessly");
    let dt: Vec<f64> = at.windows(2).map(|w| w[1] - w[0]).collect();
    // state 2 (index 1) shows 0.30 s, state 3 0.20 s, state 1 0.10 s (± one step)
    for (k, want) in [(0, 0.30), (1, 0.20), (2, 0.10), (3, 0.30)] {
        assert!(
            (dt[k] - want).abs() < 0.04,
            "switch {k}: {:.3} s, want {want}",
            dt[k]
        );
    }
    assert_eq!(
        history_rows(&mut h),
        rows,
        "playback writes no History rows"
    );
    // the button shows Stop while playing; Stop halts on the current state
    click(&mut h, &fp_i18n::t("ui-state-stop"));
    assert!(!st(&mut h).session.ui.playing, "stopped");
    let f = frame(&mut h);
    h.run_steps(60);
    assert_eq!(frame(&mut h), f, "stays after stopping");
    let play = fp_i18n::t("ui-state-play");
    assert!(h.query_all(By::new().label(&play)).next().is_some());
}

#[test]
fn canvas_playback_follows_loop_setting_and_stops_on_state_change() {
    let _g = lock();
    let mut h = harness();
    // play once: stops on the last state
    st(&mut h).push(Action::Run("pi.state.loop".into(), json!({"loop": -1})));
    h.run_steps(2);
    st(&mut h).push(Action::Run("view.togglePlayback".into(), json!({})));
    let (seen, _) = observe(&mut h, 90);
    assert_eq!(seen, vec![0, 1, 2], "one pass");
    assert!(!st(&mut h).session.ui.playing, "stopped at the end");
    // a state added while playing stops the playback
    st(&mut h).push(Action::Run("pi.state.loop".into(), json!({"loop": 0})));
    st(&mut h).push(Action::Run("view.togglePlayback".into(), json!({})));
    h.run_steps(5);
    assert!(st(&mut h).session.ui.playing);
    st(&mut h).push(Action::Run("state.add".into(), json!({})));
    h.run_steps(3);
    assert!(!st(&mut h).session.ui.playing, "state list changed → stop");
    // another document becomes active → stop
    st(&mut h).push(Action::Run("view.togglePlayback".into(), json!({})));
    h.run_steps(3);
    assert!(st(&mut h).session.ui.playing);
    st(&mut h).push(Action::Run(
        "file.new".into(),
        json!({"width": 50, "height": 50, "background": "#ffffff"}),
    ));
    h.run_steps(3);
    assert!(!st(&mut h).session.ui.playing, "document switch → stop");
}

fn dlg(h: &mut H) -> &mut ImagePreview {
    st(h).dialogs.preview.as_mut().expect("Image Preview open")
}

fn wait_idle(h: &mut H) {
    let t0 = Instant::now();
    loop {
        h.run_steps(1);
        if !dlg(h).is_busy() || t0.elapsed() > Duration::from_secs(60) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Key of the result pane 0 should show for the dialog's current state.
fn expected_key(h: &mut H) -> u64 {
    let (doc_key, page) = {
        let d = st(h).session.active().expect("doc");
        ((d.id.0, d.generation), d.page)
    };
    let p = dlg(h);
    let kind = worker::JobKind::Export(Box::new(p.panes[0].job_settings(false)));
    worker::settings_key(doc_key, page, p.frame, &kind)
}

#[test]
fn image_preview_plays_states_that_render_slower_than_their_delay() {
    let _g = lock();
    let mut h = harness();
    st(&mut h).push(Action::Run("file.imagePreview".into(), json!({})));
    h.run_steps(3);
    wait_idle(&mut h);
    assert_eq!(dlg(&mut h).frame, 0);
    // rendering is "slow": jobs wait until released (a large document takes seconds per state)
    worker::hold_jobs(true);
    let c0 = worker::counters();
    dlg(&mut h).playing = true;
    for _ in 0..60 {
        h.run_steps(1);
    }
    assert_eq!(
        dlg(&mut h).frame,
        0,
        "the shown state waits for the next one"
    );
    assert_eq!(dlg(&mut h).play_wait, Some(1), "busy cue names state 2");
    assert_eq!(
        worker::counters().cancelled,
        c0.cancelled,
        "no job is cancelled while waiting"
    );
    // rendering completes: every state is shown with its own picture
    worker::hold_jobs(false);
    let t0 = Instant::now();
    let mut seen = vec![0usize];
    let mut keys = vec![dlg(&mut h).panes[0].result.as_ref().map(|r| r.key)];
    while seen.len() < 5 && t0.elapsed() < Duration::from_secs(60) {
        h.run_steps(1);
        let f = dlg(&mut h).frame;
        if Some(&f) != seen.last() {
            let want = expected_key(&mut h);
            let got = dlg(&mut h).panes[0].result.as_ref().map(|r| r.key);
            assert_eq!(got, Some(want), "state {} shows its own picture", f + 1);
            seen.push(f);
            keys.push(got);
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(seen, vec![0, 1, 2, 0, 1], "plays all states in order");
    assert!(keys[0] != keys[1] && keys[1] != keys[2] && keys[2] != keys[0]);
    assert_eq!(
        worker::counters().cancelled,
        c0.cancelled,
        "playback cancels no per-state job"
    );
    dlg(&mut h).playing = false;
}

#[test]
fn image_preview_small_document_keeps_the_state_delays() {
    let _g = lock();
    let mut h = harness();
    st(&mut h).push(Action::Run("file.imagePreview".into(), json!({})));
    h.run_steps(3);
    wait_idle(&mut h);
    dlg(&mut h).playing = true;
    // first pass renders ahead; then the delays rule (0.10, 0.30, 0.20 s)
    let t0 = Instant::now();
    let mut seen = vec![dlg(&mut h).frame];
    let mut at = Vec::new();
    while seen.len() < 8 && t0.elapsed() < Duration::from_secs(60) {
        h.run_steps(1);
        let f = dlg(&mut h).frame;
        if Some(&f) != seen.last() {
            seen.push(f);
            at.push(h.ctx.input(|i| i.time));
        }
        if seen.len() < 4 {
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    assert_eq!(&seen[..7], &[0, 1, 2, 0, 1, 2, 0], "frames {seen:?}");
    // second pass: everything cached, the state delays hold exactly (± one step)
    let dt: Vec<f64> = at.windows(2).map(|w| w[1] - w[0]).collect();
    for (k, want) in [(3, 0.30), (4, 0.20), (5, 0.10)] {
        assert!(
            (dt[k] - want).abs() < 0.04,
            "switch {k}: {:.3} s, want {want}",
            dt[k]
        );
    }
    dlg(&mut h).playing = false;
}

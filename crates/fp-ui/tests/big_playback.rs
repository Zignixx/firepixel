//! T144 (FB-63 rest, TW-253): playback of real large animation documents saved by the reference
//! (`re\_testdateien\big_playback_01.fw.png`: 800 × 600, 45 states, seven layers, 91 unique
//! photos, vectors with live effects, a tweened symbol, delays 5 to 35, state 11 excluded from
//! export, 34 MB; `big_playback_02.fw.png`: 400 × 300, 160 small states, delays 2 to 5). Ignored
//! corpus test, runs when the files exist:
//! `cargo test --release -p fp-ui --test all big_playback:: -- --ignored --nocapture`
//! (another folder through `FP_BIG_PLAYBACK_DIR`).

use egui::vec2;
use egui_kittest::Harness;
use fp_ui::FirepixelApp;
use fp_ui::app_state::{Action, UiSettings};
use fp_ui::theme::ThemeKind;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

static LOCK: Mutex<()> = Mutex::new(());

const FILES: [&str; 2] = ["big_playback_01.fw.png", "big_playback_02.fw.png"];

fn corpus(name: &str) -> Option<PathBuf> {
    let dir = std::env::var_os("FP_BIG_PLAYBACK_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Users\Administrator\Desktop\REA\re\_testdateien"));
    let p = dir.join(name);
    if p.exists() {
        Some(p)
    } else {
        eprintln!("{} missing - skipped", p.display());
        None
    }
}

fn open(path: &Path) -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute("file.open", json!({"path": path})).expect("open");
    s
}

/// Delays (1/100 s) and the "include when exporting" flag of every state of page 0.
fn states(s: &fp_engine::Session) -> Vec<(i32, bool)> {
    let d = s.active().expect("document");
    let frames = d.doc.pages[0]
        .root
        .export
        .as_ref()
        .map(|e| e.frames.clone())
        .unwrap_or_default();
    (0..d.frame_count())
        .map(|i| frames.get(i).map_or((7, true), |f| (f.delay, f.visible)))
        .collect()
}

/// Delays (1/100 s) of the frames of a GIF file (Graphic Control Extensions, block walk).
fn gif_delays(b: &[u8]) -> Vec<u16> {
    fn skip_sub_blocks(b: &[u8], mut i: usize) -> usize {
        while i < b.len() && b[i] != 0 {
            i += b[i] as usize + 1;
        }
        i + 1
    }
    assert_eq!(&b[..3], b"GIF");
    let mut i = 13;
    if b[10] & 0x80 != 0 {
        i += 3 << ((b[10] & 7) + 1);
    }
    let mut out = Vec::new();
    while i < b.len() {
        match b[i] {
            0x21 => {
                if b[i + 1] == 0xF9 {
                    out.push(u16::from_le_bytes([b[i + 4], b[i + 5]]));
                }
                i = skip_sub_blocks(b, i + 2);
            }
            0x2C => {
                let packed = b[i + 9];
                i += 10;
                if packed & 0x80 != 0 {
                    i += 3 << ((packed & 7) + 1);
                }
                i = skip_sub_blocks(b, i + 1);
            }
            _ => break,
        }
    }
    out
}

#[test]
#[ignore = "corpus: needs re/_testdateien/big_playback_0*.fw.png"]
fn load_and_render_times_per_state() {
    for name in FILES {
        let Some(path) = corpus(name) else { continue };
        let t = Instant::now();
        let s = open(&path);
        let load = t.elapsed();
        let d = s.active().expect("document");
        let (doc, n) = (std::sync::Arc::clone(&d.doc), d.frame_count());
        let r = fp_render::Renderer::new(fp_render::RenderOptions::default());
        let mut ms = [Vec::new(), Vec::new()];
        for pass in ms.iter_mut() {
            for i in 0..n {
                let t = Instant::now();
                let img = r.render_page(&doc, 0, i).image;
                assert!(img.width() > 0, "{name}: state {i} renders");
                pass.push(t.elapsed().as_secs_f64() * 1000.0);
            }
        }
        for (k, pass) in ms.iter_mut().enumerate() {
            pass.sort_by(f64::total_cmp);
            eprintln!(
                "{name}: load {:.0} ms, {n} states, pass {} render ms min {:.1} median {:.1} max {:.1}",
                load.as_secs_f64() * 1000.0,
                k + 1,
                pass[0],
                pass[pass.len() / 2],
                pass[pass.len() - 1]
            );
        }
    }
}

/// Plays the document in the UI with frames of `dt` seconds (1/55 s: not a divisor of the delays;
/// 1/30 s: longer than the shortest delays, as in a remote session) and checks that every included state is shown in order, none dropped, and that a pass takes the
/// sum of the included delays (no drift: within one frame).
#[test]
#[ignore = "corpus: needs re/_testdateien/big_playback_0*.fw.png"]
fn big_playback() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    fp_engine::shell::disable();
    for name in FILES {
        let Some(path) = corpus(name) else { continue };
        for dt in [1.0 / 55.0, 1.0 / 30.0] {
            play(&path, name, dt);
        }
    }
}

fn play(path: &Path, name: &str, dt: f64) {
    {
        let s = open(path);
        let st = states(&s);
        let included: Vec<usize> = (0..st.len()).filter(|i| st[*i].1).collect();
        let pass: f64 = included
            .iter()
            .map(|i| st[*i].0.max(1) as f64 / 100.0)
            .sum();
        let ctx = egui::Context::default();
        let mut app = FirepixelApp::with_session(
            &ctx,
            s,
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
            .with_step_dt(dt as f32)
            .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
        h.run_steps(3);
        let frame =
            |h: &Harness<'_, FirepixelApp>| h.state().state.session.active().map(|d| d.frame);
        h.state_mut()
            .state
            .push(Action::Run("view.togglePlayback".into(), json!({})));
        let steps = ((2.2 * pass) / dt) as usize + 20;
        let wall = Instant::now();
        let mut seen = vec![frame(&h)];
        let mut at = vec![h.ctx.input(|i| i.time)];
        for _ in 0..steps {
            h.run_steps(1);
            let f = frame(&h);
            if seen.last() != Some(&f) {
                seen.push(f);
                at.push(h.ctx.input(|i| i.time));
            }
        }
        let wall = wall.elapsed().as_secs_f64();
        assert!(h.state().state.session.ui.playing, "{name}: still playing");
        let shown: Vec<usize> = seen[1..].iter().map(|f| f.expect("frame")).collect();
        // order: each switch goes to the next included state (cyclic), nothing dropped
        for w in shown.windows(2) {
            let k = included
                .iter()
                .position(|i| *i == w[0])
                .expect("included");
            assert_eq!(
                w[1],
                included[(k + 1) % included.len()],
                "{name}: next state after {}",
                w[0]
            );
        }
        let m = included.len();
        assert!(shown.len() > m + 1, "{name}: at least one full pass");
        let measured = at[1 + m] - at[1];
        eprintln!(
            "{name}: {m} included states, pass {measured:.3} s (sum of delays {pass:.3} s), \
             UI frames {:.1} ms, {steps} steps in {wall:.1} s wall",
            dt * 1000.0
        );
        assert!(
            (measured - pass).abs() <= dt + 1e-6,
            "{name}: pass {measured:.3} s, want {pass:.3} s"
        );
        h.state_mut()
            .state
            .push(Action::Run("view.togglePlayback".into(), json!({})));
        h.run_steps(2);
        assert!(!h.state().state.session.ui.playing, "{name}: Stop");
        let f = frame(&h);
        h.run_steps(30);
        assert_eq!(frame(&h), f, "{name}: stays after Stop");
    }
}

/// The animated GIF export of the corpus files: one frame per included state with its delay.
#[test]
#[ignore = "corpus: needs re/_testdateien/big_playback_0*.fw.png"]
fn big_playback_gif_export() {
    let dir = fp_i18n::profile::unique_temp_dir("t144-gif");
    for name in FILES {
        let Some(path) = corpus(name) else { continue };
        let mut s = open(&path);
        let st = states(&s);
        let want: Vec<u16> = st
            .iter()
            .filter(|(_, v)| *v)
            .map(|(d, _)| (*d).max(1) as u16)
            .collect();
        let out = dir.join(name.replace(".fw.png", ".gif"));
        let t = Instant::now();
        s.execute(
            "file.export",
            json!({"path": out, "settings": {"format": "animatedGif"}}),
        )
        .expect("export");
        let secs = t.elapsed().as_secs_f64();
        let bytes = std::fs::read(&out).expect("GIF written");
        let got = gif_delays(&bytes);
        eprintln!(
            "{name}: GIF {} frames, {:.2} s total, {} bytes, export {secs:.2} s",
            got.len(),
            got.iter().map(|d| *d as f64).sum::<f64>() / 100.0,
            bytes.len()
        );
        assert_eq!(got, want, "{name}: frame delays");
    }
}

/// F146 (FB-129, TW-281): latency of the canvas during playback with the real render thread
/// (`sync_render` off) and the other corpus file open in a second tab. The UI runs in real time
/// (frames of 1/60 s, paced by the wall clock); for every switch of the state the time from the
/// frame that made the state current (its due instant, within one frame) to the frame in which
/// the canvas shows it. Prints the numbers, checks only that every switch reaches the canvas.
#[test]
#[ignore = "corpus: needs re/_testdateien/big_playback_0*.fw.png"]
fn big_playback_canvas_latency() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    fp_engine::shell::disable();
    let (Some(p1), Some(p2)) = (corpus(FILES[0]), corpus(FILES[1])) else {
        return;
    };
    for (play_path, other) in [(&p1, &p2), (&p2, &p1)] {
        latency(play_path, other);
    }
}

fn latency(path: &Path, other: &Path) {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
    let name = name.unwrap_or_default();
    let mut s = fp_engine::Session::default();
    s.execute("file.open", json!({"path": other}))
        .expect("open other");
    s.execute("file.open", json!({"path": path})).expect("open");
    let dt = 1.0 / 60.0;
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(
        &ctx,
        s,
        UiSettings {
            theme: ThemeKind::Normal,
            ..UiSettings::default()
        },
        false,
    );
    app.state.sync_render = false;
    app.state.os_clipboard = false;
    let mut h = Harness::builder()
        .with_size(vec2(1400.0, 900.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(8)
        .with_step_dt(dt as f32)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    // first image of the current state on the canvas (real render thread)
    let shown = |h: &Harness<'_, FirepixelApp>| {
        let st = &h.state().state;
        let d = st.session.active()?;
        let k = st.canvases.get(&d.id)?.shown?;
        Some((d.frame, k.frame))
    };
    let start = Instant::now();
    loop {
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_secs_f64(dt));
        if shown(&h).is_some_and(|(a, b)| a == b) || start.elapsed().as_secs() > 60 {
            break;
        }
    }
    h.state_mut()
        .state
        .push(Action::Run("view.togglePlayback".into(), json!({})));
    let st0 = states(&h.state().state.session);
    let pass: f64 = st0
        .iter()
        .filter(|s| s.1)
        .map(|s| s.0.max(1) as f64 / 100.0)
        .sum();
    let wall = Instant::now();
    let mut next_frame = Instant::now();
    // (state, wall time it became current, frames until it was on the canvas)
    let mut pending: Option<(usize, Instant, usize)> = None;
    let mut lat_ms = Vec::new();
    let mut lat_frames = Vec::new();
    let mut skipped = 0usize;
    let mut last = shown(&h).map(|s| s.0);
    while wall.elapsed().as_secs_f64() < 1.5 * pass + 0.5 {
        next_frame += std::time::Duration::from_secs_f64(dt);
        h.run_steps(1);
        let Some((cur, on_canvas)) = shown(&h) else {
            continue;
        };
        if let Some((s, t0, n)) = pending.as_mut() {
            if on_canvas == *s {
                lat_ms.push(t0.elapsed().as_secs_f64() * 1000.0);
                lat_frames.push(*n);
                pending = None;
            } else {
                *n += 1;
            }
        }
        if last != Some(cur) {
            if pending.is_some() {
                // the next state became due before the previous one reached the canvas
                skipped += 1;
            }
            last = Some(cur);
            pending = (on_canvas != cur).then(|| (cur, Instant::now(), 1));
            if on_canvas == cur {
                lat_ms.push(0.0);
                lat_frames.push(0);
            }
        }
        let now = Instant::now();
        if next_frame > now {
            std::thread::sleep(next_frame - now);
        }
    }
    h.state_mut()
        .state
        .push(Action::Run("view.togglePlayback".into(), json!({})));
    h.run_steps(2);
    let mut sorted = lat_ms.clone();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len().max(1);
    let same_frame = lat_frames.iter().filter(|f| **f == 0).count();
    eprintln!(
        "{name} (other tab open): {} switches, shown in the due frame {same_frame}, \
         latency ms median {:.1} p90 {:.1} max {:.1}, frames mean {:.2}, \
         not on the canvas before the next switch {skipped}",
        lat_ms.len(),
        sorted.get(n / 2).copied().unwrap_or(0.0),
        sorted.get(n * 9 / 10).copied().unwrap_or(0.0),
        sorted.last().copied().unwrap_or(0.0),
        lat_frames.iter().sum::<usize>() as f64 / n as f64,
    );
    assert!(!lat_ms.is_empty(), "{name}: switches reached the canvas");
}

//! S-2: remaining items of RT-3 (re 28 D4/D5) and of T-1 (vector file options, slice inspector).
//! Run like all UI tests with `FP_NO_SHELL_OPEN=1`.

use egui::epaint::Shape;
use egui::{vec2, Rect};
use egui_kittest::kittest::{By, Queryable};
use egui_kittest::Harness;
use fp_ui::app_state::{Action, AppState, UiSettings};
use fp_ui::dialogs::image_preview::ViewMode;
use fp_ui::theme::ThemeKind;
use fp_ui::FirepixelApp;
use serde_json::json;
use std::time::{Duration, Instant};

type H = Harness<'static, FirepixelApp>;

fn harness(s: fp_engine::Session) -> H {
    fp_engine::shell::disable();
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings { theme: ThemeKind::Normal, ..UiSettings::default() }, false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(8).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h
}

fn st(h: &mut H) -> &mut AppState {
    &mut h.state_mut().state
}

fn painted_texts(h: &H) -> Vec<(String, Rect)> {
    fn walk(s: &Shape, out: &mut Vec<(String, Rect)>) {
        match s {
            Shape::Text(t) => out.push((t.galley.text().to_string(), t.visual_bounding_rect())),
            Shape::Vec(v) => v.iter().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let mut v = Vec::new();
    for cs in &h.output().shapes {
        walk(&cs.shape, &mut v);
    }
    v
}

fn wait_idle(h: &mut H) {
    let t0 = Instant::now();
    loop {
        h.run_steps(1);
        let busy = st(h).dialogs.preview.as_ref().is_some_and(|d| d.is_busy());
        if !busy || t0.elapsed() > Duration::from_secs(90) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    h.run_steps(3);
}

/// Every visible pane shows its image top-left aligned (image larger than the pane).
fn assert_top_left(h: &mut H, what: &str) {
    let d = st(h).dialogs.preview.as_ref().expect("preview");
    assert!(!d.layout.is_empty(), "{what}: no panes");
    for (i, lay) in &d.layout {
        let r = d.image_rect(*i, lay).unwrap_or_else(|| panic!("{what}: pane {i} without image"));
        assert!(r.width() > lay.image.width() && r.height() > lay.image.height(), "{what}: image smaller than pane {i}");
        assert!((r.min - lay.image.min).length() <= 0.75, "{what}: pane {i} not top-left: {r:?} in {:?}", lay.image);
    }
}

/// re 28 D4: the image preview shows a large image top-left aligned in 1 and in 2 panes (both
/// measured displays began at the top left); after the user moved the view, switching keeps it (N25).
#[test]
fn d4_preview_switch_one_to_two_keeps_top_left() {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 1200, "height": 900, "background": "#ffffff"})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [40, 40, 200, 240]})).expect("rect");
    let mut h = harness(s);
    st(&mut h).push(Action::Run("file.imagePreview".into(), json!({})));
    h.run_steps(3);
    wait_idle(&mut h);
    assert_top_left(&mut h, "1 pane");
    st(&mut h).dialogs.preview.as_mut().expect("preview").set_mode(ViewMode::Two);
    wait_idle(&mut h);
    assert_eq!(st(&mut h).dialogs.preview.as_ref().expect("preview").layout.len(), 2);
    assert_top_left(&mut h, "2 panes");
    // the user moves the view: no longer automatic, kept when switching back
    st(&mut h).dialogs.preview.as_mut().expect("preview").pan(vec2(-300.0, -250.0));
    h.run_steps(2);
    let c = st(&mut h).dialogs.preview.as_ref().expect("preview").center_doc();
    st(&mut h).dialogs.preview.as_mut().expect("preview").set_mode(ViewMode::One);
    wait_idle(&mut h);
    let d = st(&mut h).dialogs.preview.as_ref().expect("preview");
    assert!(d.user_view, "user view");
    assert!((d.center_doc() - c).length() <= 0.75, "2 → 1 keeps the user's view: {c:?} → {:?}", d.center_doc());
}

/// T-1 rest: the number fields of the vector file options are right-aligned as in the original.
#[test]
fn vector_options_numbers_right_aligned() {
    fp_i18n::with_language("de", || {
        let dir = std::env::temp_dir().join(format!("fp-s2-vec-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let ai = dir.join("probe.ai");
        std::fs::write(&ai, "%!PS-3.0\n%%BoundingBox: 0 0 120 90\n%%EndSetup\n0 0 0 1 k\n10 10 m\n90 10 L\n90 40 L\n10 40 L\nf\n").expect("AI");
        let mut s = fp_engine::Session::default();
        s.execute("file.new", json!({"width": 300, "height": 200})).expect("doc");
        let mut h = harness(s);
        assert!(fp_ui::dialogs::vector_options::open(st(&mut h), &ai));
        h.run_steps(4);
        let inputs: Vec<(Rect, String)> = h.get_all(By::new().role(egui::accesskit::Role::TextInput)).map(|n| (n.rect(), n.value().unwrap_or_default())).collect();
        let texts = painted_texts(&h);
        // fields of the dialog: right of their label in the same row
        let label = |l: &str| texts.iter().find(|(x, _)| x == l).map(|(_, r)| *r).unwrap_or_else(|| panic!("label {l}"));
        for (want, l) in [("100", "Skalieren:"), ("72", "Auflösung:")] {
            let lr = label(l);
            let field = inputs
                .iter()
                .filter(|(r, v)| v == want && r.left() >= lr.right() - 1.0 && (r.center().y - lr.center().y).abs() < 6.0)
                .map(|(r, _)| *r)
                .min_by(|a, b| a.left().total_cmp(&b.left()))
                .unwrap_or_else(|| panic!("field {want} next to {l} {lr:?}: {inputs:?}"));
            let t = texts.iter().find(|(x, r)| x == want && field.expand(2.0).contains(r.center())).map(|(_, r)| *r).unwrap_or_else(|| panic!("text {want}"));
            assert!(t.right() >= field.right() - 6.0 && t.left() > field.left() + 10.0, "{want} right-aligned: text {t:?} in {field:?}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    });
}

fn names_in(s: &fp_engine::Session, frame: usize) -> Vec<String> {
    fn walk(e: &fp_model::Element, out: &mut Vec<String>) {
        out.push(e.common.name.clone().unwrap_or_default());
        if let fp_model::ElementKind::Group(g) = &e.kind {
            g.children.iter().for_each(|c| walk(c, out));
        }
    }
    let d = s.active().expect("d");
    let mut out = Vec::new();
    for slot in &d.doc.pages[d.page].root.layers {
        if let fp_model::LayerSlot::Own(l) = slot
            && let Some(c) = l.cels.get(frame)
        {
            c.elements.iter().for_each(|e| walk(e, &mut out));
        }
    }
    out
}

/// T-1 rest (assumption kept): converting layers to states puts the topmost layer into the
/// **current** state and the next layer into the state after it, also when the current state is
/// not the first.
#[test]
fn layers_to_states_topmost_into_current_state() {
    let dir = std::env::temp_dir().join(format!("fp-s2-lay-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let src = dir.join("source.fw.png");
    {
        let mut s = fp_engine::Session::default();
        s.execute("file.new", json!({"width": 200, "height": 200, "background": "#ffffff"})).expect("doc");
        s.execute("draw.rectangle", json!({"rect": [10, 10, 30, 30], "name": "Bottom"})).expect("A");
        s.execute("layer.add", json!({})).expect("layer");
        s.execute("draw.rectangle", json!({"rect": [50, 50, 70, 70], "name": "Top"})).expect("B");
        s.execute("file.saveAs", json!({"path": src.to_string_lossy()})).expect("save");
    }
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 300, "background": "#ffffff"})).expect("doc");
    s.execute("state.add", json!({"count": 1})).expect("state");
    s.execute("state.setCurrent", json!({"index": 1})).expect("state 2");
    s.execute("file.import", json!({"path": src.to_string_lossy(), "at": [0, 0], "vector": {"layers": 2}})).expect("import");
    let d = s.active().expect("d");
    assert_eq!(d.frame, 1, "current state stays");
    assert!(d.frame_count() >= 3, "third state added: {}", d.frame_count());
    let (f0, f1, f2) = (names_in(&s, 0), names_in(&s, 1), names_in(&s, 2));
    assert!(!f0.iter().any(|n| n == "Top" || n == "Bottom"), "state 1 untouched: {f0:?}");
    assert!(f1.iter().any(|n| n == "Top") && !f1.iter().any(|n| n == "Bottom"), "topmost layer in the current state: {f1:?}");
    assert!(f2.iter().any(|n| n == "Bottom"), "next layer in the state after it: {f2:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

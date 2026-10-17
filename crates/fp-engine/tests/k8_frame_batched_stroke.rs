//! K-8 (FP1-06): fast brush stroke with a real mouse, several pointer events per frame
//! (8 points 6 ms apart, frame time 48 ms; charcoal "Soft", size 20, size from speed 100 %):
//! the last live preview shows the same pixels as the finished bitmap. Pointer speeds above
//! 1 px/ms used to be clamped to 1 only in the command (`PaintJob::from_params`), the preview kept
//! the real value.

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Overlay, ToolEvent};
use fp_model::ElementKind;
use serde_json::json;

/// Charcoal "Soft", size 20, only the sensitivity size from speed 100 % (as in the FP-3 test).
fn speed_brush() -> fp_model::Brush {
    let mut b = fp_engine::commands::panel::tool_brush_from(Some(&json!({"preset": "Kohlestift/Weich", "width": 20})), None);
    b.diameter = 20;
    b.sensitivities = vec![fp_model::Sensitivity { source: b'S', target: b'Z', value: 1000 }];
    b
}

fn at(mut e: ToolEvent, t: u64) -> ToolEvent {
    e.time_ms = t;
    e
}

/// Paints `frames` (each: list of (x, time_ms)) after a press at x 40; returns (differing, painted).
fn stroke(frames: &[Vec<(f32, u64)>], t0: u64) -> (usize, usize) {
    let mut s = doc_session(600.0, 300.0);
    s.set_colors(Some(fp_color::Argb(0xff00_0000)), None);
    s.set_tool(ToolId::Brush).unwrap();
    s.tools.options.brush_size = 20.0;
    s.tools.options.new_stroke = None;
    s.tools.options.new_brush = Some(serde_json::to_value(speed_brush()).unwrap());
    events(&mut s, &[at(ToolEvent::down(40.0, 150.0), t0)]);
    let mut last = None;
    let mut end = (40.0, t0);
    for f in frames {
        let evs: Vec<ToolEvent> = f.iter().map(|&(x, t)| at(ToolEvent::drag(x, 150.0), t)).collect();
        events(&mut s, &evs);
        if let Some(Overlay::Paint(p)) = s.tool_feedback().into_iter().find(|o| matches!(o, Overlay::Paint(_))) {
            last = Some(p);
        }
        end = *f.last().unwrap();
    }
    events(&mut s, &[at(ToolEvent::up(end.0, 150.0), end.1)]);
    let p = last.expect("preview");
    let el = top_elements(&s)[0].clone();
    let ElementKind::Bitmap(b) = &el.kind else { panic!("no bitmap") };
    let img = fp_engine::docops::bitmap_image(b).expect("image");
    let r = img.bounds();
    let (mut diff, mut painted) = (0, 0);
    for y in r.top..r.bottom {
        for x in r.left..r.right {
            let px = img.get(x, y).unwrap_or([0; 4]);
            let shown = p.tiles.iter().find(|t| t.rect.contains(x, y)).and_then(|t| t.image.get(x, y)).unwrap_or([0; 4]);
            painted += (px[3] > 0) as usize;
            diff += (shown != px) as usize;
        }
    }
    (diff, painted)
}

#[test]
fn several_points_per_frame_preview_equals_result() {
    let t0 = 1000;
    // 8 points 6 ms apart, 55 px each (≈ 9 px/ms, like 450 → 890 in 44 ms), one frame 48 ms later
    let xs: Vec<f32> = (1..=8).map(|k| 40.0 + 55.0 * k as f32).collect();
    // a) all events of the frame carry the frame time (old canvas behaviour)
    let same: Vec<(f32, u64)> = xs.iter().map(|&x| (x, t0 + 48)).collect();
    // b) frame time spread over the events (new canvas behaviour)
    let spread: Vec<(f32, u64)> = xs.iter().enumerate().map(|(k, &x)| (x, t0 + 6 * (k as u64 + 1))).collect();
    for (name, frame) in [("same", same), ("spread", spread)] {
        let (diff, painted) = stroke(&[frame], t0);
        assert!(painted > 1000, "{name}: {painted}");
        assert_eq!(diff, 0, "{name}: preview differs in {diff} of {painted} pixels");
    }
    // two frames of 4 points each
    let f1: Vec<(f32, u64)> = xs[..4].iter().enumerate().map(|(k, &x)| (x, t0 + 6 * (k as u64 + 1))).collect();
    let f2: Vec<(f32, u64)> = xs[4..].iter().enumerate().map(|(k, &x)| (x, t0 + 24 + 6 * (k as u64 + 1))).collect();
    let (diff, painted) = stroke(&[f1, f2], t0);
    assert_eq!(diff, 0, "two frames: preview differs in {diff} of {painted} pixels");
}

#[test]
fn slow_stroke_preview_equals_result() {
    let t0 = 1000;
    let frames: Vec<Vec<(f32, u64)>> = (1..=40).map(|k| vec![(40.0 + 11.0 * k as f32, t0 + 40 * k as u64)]).collect();
    let (diff, painted) = stroke(&frames, t0);
    assert!(painted > 1000, "{painted}");
    assert_eq!(diff, 0, "slow: preview differs in {diff} of {painted} pixels");
}

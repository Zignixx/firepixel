//! FB-67 (TW-251, Firepixel's own switch): View ▸ Live Preview While Dragging Points. Off by
//! default (outline preview only); on, a point or handle drag also reports `Overlay::PathLive`
//! with the dragged geometry so the canvas renders the object live. The document is untouched
//! until the release, which commits the same single step either way.

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Overlay, Session, ToolEvent};
use fp_model::{ElementId, ElementKind};
use serde_json::json;

fn tool(s: &mut Session, t: ToolId) {
    s.set_tool(t).unwrap_or_else(|e| panic!("{e}"));
}

fn triangle(s: &mut Session) -> ElementId {
    tool(s, ToolId::Pen);
    let mut evs = Vec::new();
    for (x, y) in [(10.0, 30.0), (30.0, 8.0), (50.0, 30.0), (10.0, 30.0)] {
        evs.push(ToolEvent::down(x, y));
        evs.push(ToolEvent::up(x, y));
    }
    events(s, &evs);
    top_elements(s)[0].id
}

fn first_anchor(s: &Session, id: ElementId) -> (f32, f32) {
    match &el(s, id).kind {
        ElementKind::Path(p) => {
            let a = p.contours[0].nodes[0].anchor;
            (a.x, a.y)
        }
        _ => panic!("not a path"),
    }
}

fn rows(s: &Session) -> usize {
    s.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn live(fb: &[Overlay]) -> Option<(u64, (f32, f32))> {
    fb.iter().find_map(|o| match o {
        Overlay::PathLive { target, contours } => {
            let a = contours[0].nodes[0].anchor;
            Some((*target, (a.x, a.y)))
        }
        _ => None,
    })
}

/// Drags the first anchor from 10/30 to 20/34; returns the feedback mid-drag, the document's
/// anchor mid-drag and the number of undo rows mid-drag.
fn drag(s: &mut Session) -> (Vec<Overlay>, (f32, f32), usize) {
    events(
        s,
        &[ToolEvent::down(10.0, 30.0), ToolEvent::drag(15.0, 32.0)],
    );
    let fb = s
        .tool_event(&ToolEvent::drag(20.0, 34.0))
        .unwrap_or_else(|e| panic!("{e}"))
        .overlay;
    let id = top_elements(s)[0].id;
    let mid = (first_anchor(s, id), rows(s));
    events(s, &[ToolEvent::up(20.0, 34.0)]);
    (fb, mid.0, mid.1)
}

#[test]
fn live_point_preview_toggle_is_off_by_default_and_flips_the_pref() {
    let mut s = doc_session(60.0, 40.0);
    assert_eq!(s.command_state("view.livePointPreview").1, Some(false));
    assert!(!fp_engine::tools::snap::live_point_preview(&s.prefs));
    ex(&mut s, "view.livePointPreview", json!({}));
    assert_eq!(s.command_state("view.livePointPreview").1, Some(true));
    assert!(s.prefs.get_bool("LivePointPreview", false));
    ex(&mut s, "view.livePointPreview", json!({}));
    assert_eq!(s.command_state("view.livePointPreview").1, Some(false));
    assert!(!s.prefs.get_bool("LivePointPreview", true));
}

#[test]
fn point_drag_reports_live_geometry_only_with_the_switch_and_commits_one_step() {
    for on in [false, true] {
        let mut s = doc_session(60.0, 40.0);
        let id = triangle(&mut s);
        if on {
            ex(&mut s, "view.livePointPreview", json!({}));
        }
        tool(&mut s, ToolId::Subselection);
        let before = rows(&s);
        let (fb, mid_anchor, mid_rows) = drag(&mut s);
        // the outline is there either way
        assert!(
            fb.iter().any(|o| matches!(o, Overlay::Path { .. })),
            "{fb:?}"
        );
        if on {
            assert_eq!(live(&fb), Some((id.0, (20.0, 34.0))), "{fb:?}");
        } else {
            assert_eq!(live(&fb), None, "{fb:?}");
        }
        // mid-drag: the document itself is unchanged, no row yet
        assert_eq!(mid_anchor, (10.0, 30.0), "on={on}");
        assert_eq!(mid_rows, before, "on={on}");
        // release: exactly one step, undo restores
        assert_eq!(first_anchor(&s, id), (20.0, 34.0), "on={on}");
        assert_eq!(rows(&s), before + 1, "on={on}");
        let label = s.undo_redo_labels().0;
        assert!(
            matches!(label.as_deref(), Some("Punkt verschieben" | "Move Point")),
            "{label:?}"
        );
        s.undo().expect("undo");
        assert_eq!(first_anchor(&s, id), (10.0, 30.0));
        // after the release no live geometry is reported any more
        assert_eq!(live(&s.tool_feedback()), None);
    }
}

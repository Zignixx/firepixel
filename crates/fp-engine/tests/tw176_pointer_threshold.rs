//! TW-176 (re 55 §7.4): the Pointer starts a selection move only after the mouse travelled more
//! than 3·√zoom screen px (no move at 3 / 4 / 6 / 12 px, a move at 4 / 5 / 7 / 13 px at
//! 100 / 200 / 400 / 1600 %), independent of the number of intermediate mouse events; the
//! delta is floor(end) − floor(start) in document px, and a drag past the threshold that ends
//! on the start pixel still writes a "Move" row with x:0, y:0.

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent};
use serde_json::json;

fn setup(zoom: f64) -> (Session, fp_model::ElementId) {
    let mut s = doc_session(300.0, 200.0);
    let a = rect(&mut s, 10.0, 10.0, 50.0, 50.0);
    select(&mut s, &[a]);
    s.set_tool(ToolId::Pointer)
        .unwrap_or_else(|e| panic!("{e}"));
    ex(&mut s, "view.setZoom", json!({"zoom": zoom}));
    (s, a)
}

fn rows(s: &Session) -> usize {
    s.active().map(|d| d.history.steps().len()).unwrap_or(0)
}

/// Horizontal drag from document x `x0` (y 30) to `x1` in `steps` intermediate events; returns
/// the new history row as (command, dx, dy), or `None` when no row was written.
fn drag_row(s: &mut Session, x0: f32, x1: f32, steps: usize) -> Option<(String, f64, f64)> {
    let n0 = rows(s);
    let mut evs = vec![ToolEvent::down(x0, 30.0)];
    for i in 1..=steps {
        evs.push(ToolEvent::drag(
            x0 + (x1 - x0) * i as f32 / steps as f32,
            30.0,
        ));
    }
    evs.push(ToolEvent::up(x1, 30.0));
    events(s, &evs);
    let d = s.active().expect("doc");
    let st = d.history.steps();
    (st.len() > n0).then(|| {
        assert_eq!(st.len(), n0 + 1, "one row per drag");
        let t = st.last().expect("row");
        (
            t.command.clone(),
            t.params["dx"].as_f64().unwrap_or(f64::NAN),
            t.params["dy"].as_f64().unwrap_or(f64::NAN),
        )
    })
}

#[test]
fn threshold_follows_the_measured_values_per_zoom() {
    // (zoom, largest travel without a move, smallest travel with a move) in screen px
    for (zoom, no, yes) in [
        (1.0, 3.0, 4.0),
        (2.0, 4.0, 5.0),
        (4.0, 6.0, 7.0),
        (16.0, 12.0, 13.0),
    ] {
        for steps in [1, 2, 20] {
            let (mut s, a) = setup(zoom);
            let z = zoom as f32;
            let x0 = 30.0;
            assert_eq!(
                drag_row(&mut s, x0, x0 + no / z, steps),
                None,
                "{zoom} {no} px {steps}"
            );
            assert!(rect_approx(
                bounds(&s, a),
                fp_geom::Rect::new(10.0, 10.0, 50.0, 50.0)
            ));
            let want = ((x0 + yes / z).floor() - x0.floor()) as f64;
            assert_eq!(
                drag_row(&mut s, x0, x0 + yes / z, steps),
                Some(("edit.move".to_string(), want, 0.0)),
                "{zoom} {yes} px {steps}"
            );
        }
    }
}

#[test]
fn zero_move_past_the_threshold_writes_a_move_row() {
    for lang in ["de", "en"] {
        fp_i18n::set_language(lang);
        // measured at 1600 %: 30.9375 → 30.0 (−15 px) and 19.0625 → 19.875 (+13 px) give x:0
        for (x0, x1) in [(30.9375, 30.0), (19.0625, 19.875)] {
            let (mut s, a) = setup(16.0);
            assert_eq!(
                drag_row(&mut s, x0, x1, 20),
                Some(("edit.move".to_string(), 0.0, 0.0))
            );
            let l = s.active().map(|d| d.history.labels()).unwrap_or_default();
            let want = if lang == "de" { "Verschieben" } else { "Move" };
            assert_eq!(l.last().map(String::as_str), Some(want), "{lang}");
            assert!(rect_approx(
                bounds(&s, a),
                fp_geom::Rect::new(10.0, 10.0, 50.0, 50.0)
            ));
        }
        // the other measured rows of the series: floor(end) − floor(start)
        for (x0, x1, dx) in [
            (30.0625, 31.5625, 1.0),
            (30.0625, 28.5625, -2.0),
            (30.5, 32.0, 2.0),
            (30.5, 29.0, -1.0),
            (30.9375, 32.0, 2.0),
        ] {
            let (mut s, _) = setup(16.0);
            assert_eq!(
                drag_row(&mut s, x0, x1, 20),
                Some(("edit.move".to_string(), dx, 0.0))
            );
        }
    }
    fp_i18n::set_language("de");
}

#[test]
fn a_click_without_travel_writes_no_row() {
    let (mut s, _) = setup(1.0);
    assert_eq!(drag_row(&mut s, 30.0, 30.0, 1), None);
}

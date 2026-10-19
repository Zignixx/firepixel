//! Arrow-key nudges: consecutive presses of an unchanged selection merge into one history row
//! with the summed offset (measured, TW-148); a selection change starts a new row.

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent, ToolKey};

fn press(s: &mut Session, e: ToolEvent) {
    events(s, &[e]);
}

fn labels(s: &Session) -> Vec<String> {
    s.active().map(|d| d.history.labels()).unwrap_or_default()
}

fn last_offset(s: &Session) -> (String, Option<f64>, Option<f64>) {
    let d = s.active().expect("doc");
    let steps = d.history.steps();
    let st = steps.last().expect("step");
    (st.command.clone(), st.params["dx"].as_f64(), st.params["dy"].as_f64())
}

#[test]
fn three_arrow_presses_are_one_move_row() {
    for lang in ["de", "en"] {
        fp_i18n::set_language(lang);
        let mut s = doc_session(300.0, 200.0);
        let a = rect(&mut s, 10.0, 10.0, 50.0, 50.0);
        select(&mut s, &[a]);
        s.set_tool(ToolId::Pointer).expect("pointer");
        let n0 = labels(&s).len();
        press(&mut s, ToolEvent::key(ToolKey::Right).shift());
        press(&mut s, ToolEvent::key(ToolKey::Right).shift());
        press(&mut s, ToolEvent::key(ToolKey::Down).shift());
        let l = labels(&s);
        let want = if lang == "de" { "Verschieben" } else { "Move" };
        assert_eq!(&l[n0..], [want], "{lang}");
        assert_eq!(last_offset(&s), ("edit.move".into(), Some(20.0), Some(10.0)));
        assert!(rect_approx(bounds(&s, a), fp_geom::Rect::new(30.0, 20.0, 70.0, 60.0)));
        s.undo().expect("undo");
        assert!(rect_approx(bounds(&s, a), fp_geom::Rect::new(10.0, 10.0, 50.0, 50.0)));
        s.redo().expect("redo");
        assert!(rect_approx(bounds(&s, a), fp_geom::Rect::new(30.0, 20.0, 70.0, 60.0)));
    }
    fp_i18n::set_language("de");
}

#[test]
fn selection_change_starts_a_new_row() {
    let mut s = doc_session(300.0, 200.0);
    let a = rect(&mut s, 10.0, 10.0, 50.0, 50.0);
    let b = rect(&mut s, 100.0, 100.0, 120.0, 120.0);
    select(&mut s, &[a]);
    s.set_tool(ToolId::Pointer).expect("pointer");
    let n0 = labels(&s).len();
    press(&mut s, ToolEvent::key(ToolKey::Right));
    press(&mut s, ToolEvent::key(ToolKey::Up));
    select(&mut s, &[b]);
    let n1 = labels(&s).len();
    press(&mut s, ToolEvent::key(ToolKey::Right).shift());
    let l = labels(&s);
    assert_eq!(l.len(), n1 + 1);
    assert_eq!(last_offset(&s), ("edit.move".into(), Some(10.0), Some(0.0)));
    assert!(rect_approx(bounds(&s, a), fp_geom::Rect::new(11.0, 9.0, 51.0, 49.0)));
    // the first row holds both presses on a
    let d = s.active().expect("doc");
    let steps = d.history.steps();
    assert_eq!(steps[n0].command, "edit.move");
    assert_eq!((steps[n0].params["dx"].as_f64(), steps[n0].params["dy"].as_f64()), (Some(1.0), Some(-1.0)));
}

#[test]
fn alt_arrow_presses_are_one_duplicate_row() {
    for lang in ["de", "en"] {
        fp_i18n::set_language(lang);
        let mut s = doc_session(300.0, 200.0);
        let a = rect(&mut s, 10.0, 10.0, 50.0, 50.0);
        select(&mut s, &[a]);
        s.set_tool(ToolId::Pointer).expect("pointer");
        let n0 = labels(&s).len();
        let n = top_elements(&s).len();
        press(&mut s, ToolEvent::key(ToolKey::Right).alt());
        press(&mut s, ToolEvent::key(ToolKey::Right).alt());
        press(&mut s, ToolEvent::key(ToolKey::Down).alt());
        let l = labels(&s);
        let want = if lang == "de" { "Duplizieren" } else { "Duplicate" };
        assert_eq!(&l[n0..], [want], "{lang}");
        // one copy, moved by the summed offset; the original stays
        assert_eq!(top_elements(&s).len(), n + 1);
        assert!(rect_approx(bounds(&s, a), fp_geom::Rect::new(10.0, 10.0, 50.0, 50.0)));
        assert_eq!(last_offset(&s), ("edit.move".into(), Some(2.0), Some(1.0)));
        s.undo().expect("undo");
        assert_eq!(top_elements(&s).len(), n);
    }
    fp_i18n::set_language("de");
}

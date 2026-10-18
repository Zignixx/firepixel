//! Arrow-key nudges: one history row per press, no merging (re 51 §2).

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

#[test]
fn three_arrow_presses_are_three_move_rows() {
    for lang in ["de", "en"] {
        fp_i18n::set_language(lang);
        let mut s = doc_session(300.0, 200.0);
        let a = rect(&mut s, 10.0, 10.0, 50.0, 50.0);
        select(&mut s, &[a]);
        s.set_tool(ToolId::Pointer).expect("pointer");
        let n0 = labels(&s).len();
        press(&mut s, ToolEvent::key(ToolKey::Right));
        press(&mut s, ToolEvent::key(ToolKey::Right));
        press(&mut s, ToolEvent::key(ToolKey::Down).shift());
        let l = labels(&s);
        let want = if lang == "de" { "Verschieben" } else { "Move" };
        assert_eq!(&l[n0..], [want, want, want], "{lang}");
        assert!(rect_approx(bounds(&s, a), fp_geom::Rect::new(12.0, 20.0, 52.0, 60.0)));
        s.undo().expect("undo");
        assert!(rect_approx(bounds(&s, a), fp_geom::Rect::new(12.0, 10.0, 52.0, 50.0)));
        s.undo().expect("undo");
        assert!(rect_approx(bounds(&s, a), fp_geom::Rect::new(11.0, 10.0, 51.0, 50.0)));
        s.undo().expect("undo");
        assert!(rect_approx(bounds(&s, a), fp_geom::Rect::new(10.0, 10.0, 50.0, 50.0)));
    }
    fp_i18n::set_language("de");
}

#[test]
fn alt_arrow_presses_are_duplicate_rows() {
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
        let l = labels(&s);
        let want = if lang == "de" { "Duplizieren" } else { "Duplicate" };
        assert_eq!(&l[n0..], [want, want], "{lang}");
        assert_eq!(top_elements(&s).len(), n + 2);
        s.undo().expect("undo");
        assert_eq!(top_elements(&s).len(), n + 1);
    }
    fp_i18n::set_language("de");
}

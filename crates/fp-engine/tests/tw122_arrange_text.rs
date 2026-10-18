//! TW-122 (re 46 §1): Ctrl+Shift+B/I/U belong to Arrange/Select/Insert in the default set, the
//! text styles sit on Ctrl+Alt+Shift+B/I/U and F5/F6/F7; arranging an object that cannot move is
//! a silent no-op; text style commands toggle over the whole pointer selection.

use crate::common::{doc_session, ex, rect, select, top_elements};
use fp_engine::Session;
use fp_engine::shortcuts::Shortcut;
use fp_model::ElementId;
use fp_model::element::ElementKind;
use serde_json::json;

fn sc(ctrl: bool, alt: bool, shift: bool, vk: u16) -> Shortcut {
    Shortcut {
        ctrl,
        alt,
        shift,
        vk,
    }
}

/// The set the original's measuring machine uses (re 10 T4: switched from "Web Standard" to the
/// program's own set); the fresh-install default stays "Web Standard" (BA-127).
#[test]
fn program_set_binds_ctrl_shift_biu_like_the_original() {
    let mut s = Session::default();
    s.prefs.set(
        "CurrentShortcutSet",
        fp_engine::prefs::PrefValue::Str("Firepixel".into()),
    );
    assert_eq!(s.shortcut_set().name, "Firepixel");
    let cases = [
        (sc(true, false, true, 0x42), "modify.arrange.sendBackward"),
        (sc(true, false, true, 0x49), "select.inverse"),
        (sc(true, false, true, 0x55), "edit.insert.hotspot"),
        (sc(true, true, true, 0x42), "text.style.fauxBold"),
        (sc(true, true, true, 0x49), "text.style.fauxItalic"),
        (sc(true, true, true, 0x55), "text.style.underline"),
        (sc(true, true, true, 0x50), "text.style.plain"),
        (sc(false, false, false, 0x74), "text.style.plain"),
        (sc(false, false, false, 0x75), "text.style.fauxBold"),
        (sc(false, false, false, 0x76), "text.style.fauxItalic"),
    ];
    for (k, want) in cases {
        assert_eq!(s.command_for_shortcut(&k).as_deref(), Some(want), "{k:?}");
    }
}

/// In "Web Standard" Ctrl+Shift+B is Break Apart and bold is Ctrl+B (re 46 §1.1, other sets).
#[test]
fn web_standard_set_differs_as_documented() {
    let s = Session::default();
    assert_eq!(s.shortcut_set().name, "Web Standard");
    let cases = [
        (sc(true, false, true, 0x42), "modify.symbol.breakApart"),
        (sc(true, false, true, 0x49), "select.inverse"),
        (sc(true, false, true, 0x55), "edit.insert.hotspot"),
        (sc(true, false, false, 0x42), "text.style.fauxBold"),
        (sc(true, false, false, 0x49), "text.style.fauxItalic"),
    ];
    for (k, want) in cases {
        assert_eq!(s.command_for_shortcut(&k).as_deref(), Some(want), "{k:?}");
    }
}

fn text(s: &mut Session, x: f32, y: f32, t: &str) -> ElementId {
    let v = ex(
        s,
        "draw.text",
        json!({"text": t, "anchor": [x, y], "size": 24, "color": "#000000"}),
    );
    ElementId(v["id"].as_u64().unwrap_or(0))
}

fn order(s: &Session) -> Vec<ElementId> {
    top_elements(s).iter().map(|e| e.id).collect()
}

/// Mark the document saved and return (history length, generation).
fn mark_saved(s: &mut Session) -> (usize, u64) {
    let d = s.active_mut().expect("doc");
    d.saved_generation = d.generation;
    (d.history.len(), d.generation)
}

fn assert_untouched(s: &Session, before: (usize, u64), what: &str) {
    let d = s.active().expect("doc");
    assert_eq!(d.history.len(), before.0, "{what}: no history step");
    assert_eq!(d.generation, before.1, "{what}: document unchanged");
    assert!(!d.is_dirty(), "{what}: document stays clean");
}

#[test]
fn arrange_without_possible_move_is_silent_noop() {
    let mut s = doc_session(400.0, 300.0);
    let tx1 = text(&mut s, 50.0, 80.0, "Abc 123");
    let r1 = rect(&mut s, 150.0, 50.0, 250.0, 120.0);
    let r2 = rect(&mut s, 260.0, 50.0, 350.0, 120.0);
    let stack = order(&s);
    assert_eq!(stack, vec![r2, r1, tx1]);

    select(&mut s, &[tx1]);
    for name in ["modify.arrange.sendBackward", "modify.arrange.sendToBack"] {
        let before = mark_saved(&mut s);
        assert!(s.target_warning(name).is_none(), "{name}: no warning");
        ex(&mut s, name, json!({}));
        assert_untouched(&s, before, name);
        assert_eq!(order(&s), stack);
    }
    select(&mut s, &[r2]);
    for name in ["modify.arrange.bringForward", "modify.arrange.bringToFront"] {
        let before = mark_saved(&mut s);
        ex(&mut s, name, json!({}));
        assert_untouched(&s, before, name);
        assert_eq!(order(&s), stack);
    }

    // a real move still makes one step
    let before = mark_saved(&mut s);
    ex(&mut s, "modify.arrange.sendBackward", json!({}));
    assert_eq!(order(&s), vec![r1, r2, tx1]);
    let d = s.active().expect("doc");
    assert_eq!(d.history.len(), before.0 + 1);
    assert!(d.is_dirty());
}

fn bold_runs(s: &Session, id: ElementId) -> Vec<bool> {
    let d = s.active().expect("doc");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("text");
    match &e.kind {
        ElementKind::Text(t) => t
            .resolved_runs()
            .iter()
            .map(|(a, _)| a.bold.unwrap_or(false))
            .collect(),
        _ => Vec::new(),
    }
}

#[test]
fn style_toggles_over_whole_pointer_selection() {
    let mut s = doc_session(400.0, 300.0);
    let a = text(&mut s, 20.0, 50.0, "One");
    let b = text(&mut s, 20.0, 150.0, "Two");
    // make only A bold
    select(&mut s, &[a]);
    ex(&mut s, "text.style.fauxBold", json!({}));
    assert!(bold_runs(&s, a).iter().all(|x| *x));
    // A bold, B not → both become bold (not a per-object flip)
    select(&mut s, &[a, b]);
    assert_eq!(
        s.registry().get("text.style.fauxBold").and_then(|c| c.checked(&s)),
        Some(false),
        "not all bold → unchecked"
    );
    ex(&mut s, "text.style.fauxBold", json!({}));
    assert!(bold_runs(&s, a).iter().all(|x| *x));
    assert!(bold_runs(&s, b).iter().all(|x| *x));
    // selection unchanged
    assert_eq!(s.active().expect("doc").selection.elements, vec![a, b]);
    // all bold → toggles off for all
    ex(&mut s, "text.style.fauxBold", json!({}));
    assert!(bold_runs(&s, a).iter().all(|x| !*x));
    assert!(bold_runs(&s, b).iter().all(|x| !*x));
}

//! FB-44 (Firepixel extension): "Lock pattern to document" (`pi.fill {lockPattern}`) keeps the
//! fill handles of a pattern or gradient in the document while the object is moved (script move,
//! arrow keys, X/Y fields, Pointer drag, align) or transformed (numeric transform and
//! `transform.matrix`, also with "Scale attributes" on), so the rendered tile phase stays; the flag
//! travels with duplicate, clone, Alt-drag copies and copy/paste; one History row. An unlocked
//! fill keeps moving along (F68, RE spec 61 §1). The Pointer drags all three handles when the
//! centre handle h1 is pressed (RE spec 61 §1.2).

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent};
use fp_model::{ElementId, ElementKind, FillHandles};
use serde_json::json;

/// Test pattern 16 × 12 (left half filled) in the test profile's pattern folder, written once per
/// test program.
fn write_pattern() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let _ = doc_session(10.0, 10.0); // test profile
        let user = fp_i18n::profile::roaming_dir().expect("test profile");
        let path = user.join("Patterns").join("Fb44Muster.png");
        let mut s = doc_session(16.0, 12.0);
        let _ = rect(&mut s, 0.0, 0.0, 8.0, 12.0);
        let _ = rect(&mut s, 8.0, 0.0, 16.0, 4.0);
        std::fs::create_dir_all(path.parent().expect("folder")).expect("create folder");
        ex(
            &mut s,
            "file.exportArea",
            json!({"rect": [0, 0, 16, 12], "path": path.to_string_lossy(), "settings": {"format": "png32"}}),
        );
        assert!(path.is_file(), "{path:?}");
    });
}

fn handles(s: &Session, id: ElementId) -> FillHandles {
    match &el(s, id).kind {
        ElementKind::Path(p) => p.fill_handles,
        ElementKind::Group(g) => match &g.children[0].kind {
            ElementKind::Path(p) => p.fill_handles,
            _ => panic!("path in group"),
        },
        _ => panic!("path"),
    }
}

fn hv(h: FillHandles) -> [(f32, f32); 3] {
    [
        (h.start.x, h.start.y),
        (h.end.x, h.end.y),
        (h.second.x, h.second.y),
    ]
}

fn shifted(h: FillHandles, dx: f32, dy: f32) -> [(f32, f32); 3] {
    hv(h).map(|(x, y)| (x + dx, y + dy))
}

fn locked(s: &Session, id: ElementId) -> bool {
    fp_engine::docops::path_attrs_of(&el(s, id))
        .and_then(|a| a.fill.clone())
        .is_some_and(|f| f.lock_to_document)
}

fn undo_count(s: &Session) -> usize {
    s.active().expect("doc").history.undo_count()
}

fn render(s: &Session) -> fp_raster::RgbaImage {
    let d = s.active().expect("doc");
    fp_engine::render::default_renderer()
        .expect("renderer")
        .render_page(&d.doc, d.page, d.frame)
        .expect("image")
}

fn px(img: &fp_raster::RgbaImage, x: u32, y: u32) -> [u8; 4] {
    let i = (y as usize * img.width() + x as usize) * 4;
    let b = img.as_bytes();
    [b[i], b[i + 1], b[i + 2], b[i + 3]]
}

/// Pattern rectangle 100,100–260,220 with the test pattern (handles (180,160) (196,160) (180,148)).
fn pattern_rect(s: &mut Session, lock: bool) -> ElementId {
    let r = rect(s, 100.0, 100.0, 260.0, 220.0);
    select(s, &[r]);
    ex(s, "pi.fill", json!({"pattern": "Fb44Muster"}));
    if lock {
        ex(s, "pi.fill", json!({"lockPattern": true}));
    }
    r
}

fn drag(s: &mut Session, a: (f32, f32), b: (f32, f32)) {
    events(s, &[ToolEvent::down(a.0, a.1)]);
    for i in 1..=10 {
        let t = i as f32 / 10.0;
        events(
            s,
            &[ToolEvent::drag(
                a.0 + (b.0 - a.0) * t,
                a.1 + (b.1 - a.1) * t,
            )],
        );
    }
    events(s, &[ToolEvent::up(b.0, b.1)]);
}

#[test]
fn lock_is_one_fill_row_and_undoable() {
    write_pattern();
    let mut s = doc_session(600.0, 400.0);
    let r = pattern_rect(&mut s, false);
    assert!(!locked(&s, r), "default off");
    let n0 = undo_count(&s);
    ex(&mut s, "pi.fill", json!({"lockPattern": true}));
    assert!(locked(&s, r));
    assert_eq!(undo_count(&s), n0 + 1, "one History row");
    // the same value again: no row
    ex(&mut s, "pi.fill", json!({"lockPattern": true}));
    assert_eq!(undo_count(&s), n0 + 1);
    ex(&mut s, "edit.undo", json!({}));
    assert!(!locked(&s, r));
    // choosing another pattern keeps the flag
    ex(&mut s, "edit.redo", json!({}));
    ex(&mut s, "pi.fill", json!({"category": "pattern"}));
    assert!(locked(&s, r));
}

#[test]
fn locked_pattern_stays_in_the_document_when_moved() {
    write_pattern();
    let mut s = doc_session(600.0, 400.0);
    let r = pattern_rect(&mut s, true);
    let h0 = handles(&s, r);
    assert_eq!(hv(h0), [(180.0, 160.0), (196.0, 160.0), (180.0, 148.0)]);
    let img0 = render(&s);
    ex(&mut s, "edit.move", json!({"dx": 37, "dy": 23}));
    assert_eq!(bounds(&s, r).left, 137.0, "the object moved");
    assert_eq!(hv(handles(&s, r)), hv(h0), "handles stay in the document");
    // the tile phase stays: every texel inside both the old and the new interior is unchanged
    let img1 = render(&s);
    let mut same = 0;
    for y in 126..218 {
        for x in 140..258 {
            assert_eq!(px(&img0, x, y), px(&img1, x, y), "texel {x},{y}");
            same += 1;
        }
    }
    assert!(same > 10_000);
    // and it really is a pattern (both colours present)
    let row: std::collections::HashSet<[u8; 4]> = (140..258).map(|x| px(&img1, x, 130)).collect();
    assert!(row.len() >= 2, "{row:?}");
    // undo puts the object back, handles unchanged
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(bounds(&s, r).left, 100.0);
    assert_eq!(hv(handles(&s, r)), hv(h0));
    // arrow keys, X/Y fields, script move with ids, align
    ex(&mut s, "edit.nudge.bigRight", json!({}));
    ex(&mut s, "edit.nudge.down", json!({}));
    assert_eq!(bounds(&s, r).left, 110.0);
    assert_eq!(hv(handles(&s, r)), hv(h0), "arrow keys");
    ex(&mut s, "element.setBounds", json!({"x": 200, "y": 150}));
    assert_eq!(bounds(&s, r).left, 200.0);
    assert_eq!(hv(handles(&s, r)), hv(h0), "X/Y fields");
    ex(
        &mut s,
        "edit.move",
        json!({"dx": -5, "dy": 0, "ids": [r.0]}),
    );
    assert_eq!(hv(handles(&s, r)), hv(h0), "move with ids");
    // the Pointer drag (away from the fill handles)
    s.set_tool(ToolId::Pointer).expect("pointer");
    let b = bounds(&s, r);
    drag(
        &mut s,
        (b.left + 5.0, b.bottom - 5.0),
        (b.left + 25.0, b.bottom + 5.0),
    );
    assert!(
        bounds(&s, r).left > b.left + 10.0,
        "dragged (with snapping)"
    );
    assert_eq!(hv(handles(&s, r)), hv(h0), "Pointer drag");
}

#[test]
fn unlocked_pattern_moves_along() {
    write_pattern();
    let mut s = doc_session(600.0, 400.0);
    let r = pattern_rect(&mut s, false);
    let h0 = handles(&s, r);
    let img0 = render(&s);
    ex(&mut s, "edit.move", json!({"dx": 37, "dy": 23}));
    assert_eq!(hv(handles(&s, r)), shifted(h0, 37.0, 23.0));
    let img1 = render(&s);
    for y in 104..216 {
        for x in 104..256 {
            assert_eq!(px(&img0, x, y), px(&img1, x + 37, y + 23), "texel {x},{y}");
        }
    }
}

#[test]
fn locked_fill_keeps_its_handles_in_transformations() {
    write_pattern();
    let mut s = doc_session(600.0, 400.0);
    let r = pattern_rect(&mut s, true);
    let h0 = handles(&s, r);
    for (mode, extra) in [
        ("scale", json!({"scaleX": 150, "scaleY": 150})),
        ("rotate", json!({"angle": 90})),
    ] {
        let mut p = json!({"mode": mode, "scaleAttributes": true});
        for (k, v) in extra.as_object().expect("object") {
            p[k] = v.clone();
        }
        ex(&mut s, "modify.transform.numericTransform", p);
        assert_eq!(
            hv(handles(&s, r)),
            hv(h0),
            "numeric {mode} with Scale attributes"
        );
        ex(&mut s, "edit.undo", json!({}));
    }
    // the transform tools' command with Scale attributes on, and a flip
    ex(
        &mut s,
        "transform.matrix",
        json!({"matrix": [1.25, 0, 0, 1.25, 10, 5], "scaleAttributes": true}),
    );
    assert!(bounds(&s, r).width() > 190.0, "scaled");
    assert_eq!(hv(handles(&s, r)), hv(h0), "transform.matrix");
    ex(&mut s, "modify.transform.flipHorizontal", json!({}));
    assert_eq!(hv(handles(&s, r)), hv(h0), "flip");
    // unlocked again: Scale attributes transforms the handles (RE spec 61 §1.5)
    ex(&mut s, "pi.fill", json!({"lockPattern": false}));
    ex(
        &mut s,
        "modify.transform.numericTransform",
        json!({"mode": "rotate", "angle": 90, "scaleAttributes": true}),
    );
    assert_ne!(hv(handles(&s, r)), hv(h0));
}

#[test]
fn locked_gradient_and_grouped_pattern_stay_too() {
    write_pattern();
    let mut s = doc_session(600.0, 400.0);
    let g = rect(&mut s, 300.0, 50.0, 400.0, 120.0);
    select(&mut s, &[g]);
    ex(
        &mut s,
        "element.setGradient",
        json!({"from": [310, 85], "to": [390, 85]}),
    );
    ex(&mut s, "pi.fill", json!({"lockPattern": true}));
    let hg = handles(&s, g);
    ex(&mut s, "edit.move", json!({"dx": 12, "dy": 7}));
    assert_eq!(hv(handles(&s, g)), hv(hg), "gradient");
    // a locked pattern inside a group stays when the group moves
    let r = pattern_rect(&mut s, true);
    let h0 = handles(&s, r);
    ex(&mut s, "modify.group", json!({}));
    let grp = s
        .active()
        .and_then(|d| d.selection.elements.first().copied())
        .expect("group");
    ex(&mut s, "edit.move", json!({"dx": 37, "dy": 23}));
    assert_eq!(hv(handles(&s, grp)), hv(h0), "grouped");
}

#[test]
fn lock_travels_with_duplicate_clone_and_paste() {
    write_pattern();
    let mut s = doc_session(600.0, 400.0);
    let r = pattern_rect(&mut s, true);
    let h0 = handles(&s, r);
    let sel = |s: &Session| {
        s.active()
            .and_then(|d| d.selection.elements.first().copied())
            .expect("selection")
    };
    // duplicate (+10/+10): the copy keeps the lock and therefore the same phase in the document
    ex(&mut s, "edit.duplicate", json!({}));
    let dup = sel(&s);
    assert_ne!(dup, r);
    assert!(locked(&s, dup));
    assert_eq!(bounds(&s, dup).left, 110.0);
    assert_eq!(hv(handles(&s, dup)), hv(h0), "duplicate: same phase");
    // clone
    select(&mut s, &[r]);
    ex(&mut s, "edit.clone", json!({}));
    let cl = sel(&s);
    assert!(locked(&s, cl));
    assert_eq!(hv(handles(&s, cl)), hv(h0));
    // Alt-drag copy (edit.move copy)
    select(&mut s, &[r]);
    ex(
        &mut s,
        "edit.move",
        json!({"dx": 50, "dy": 0, "copy": true}),
    );
    let mv = sel(&s);
    assert!(locked(&s, mv));
    assert_eq!(bounds(&s, mv).left, 150.0);
    assert_eq!(hv(handles(&s, mv)), hv(h0), "Alt-drag copy");
    // copy / paste
    select(&mut s, &[r]);
    ex(&mut s, "edit.copy", json!({}));
    ex(&mut s, "edit.paste", json!({}));
    let pasted = sel(&s);
    assert_ne!(pasted, r);
    assert!(locked(&s, pasted), "paste keeps the flag");
    assert_eq!(hv(handles(&s, pasted)), hv(h0));
}

/// RE spec 61 §1.2: a press on the centre handle h1 drags all three handles by the same delta
/// (the tile vectors stay); h2 and h3 move alone.
#[test]
fn pointer_centre_handle_drags_all_three_handles() {
    write_pattern();
    let mut s = doc_session(600.0, 400.0);
    let r = pattern_rect(&mut s, false);
    let h0 = handles(&s, r);
    s.set_tool(ToolId::Pointer).expect("pointer");
    let n0 = undo_count(&s);
    drag(&mut s, (180.0, 160.0), (217.0, 183.0));
    assert_eq!(undo_count(&s), n0 + 1, "one step");
    assert_eq!(
        hv(handles(&s, r)),
        shifted(h0, 37.0, 23.0),
        "whole set moved"
    );
    assert_eq!(bounds(&s, r).left, 100.0, "the object stays");
    // h2 alone
    drag(&mut s, (233.0, 183.0), (243.0, 193.0));
    let h = handles(&s, r);
    assert_eq!((h.end.x, h.end.y), (243.0, 193.0));
    assert_eq!(
        [(h.start.x, h.start.y), (h.second.x, h.second.y)],
        [(217.0, 183.0), (217.0, 171.0)]
    );
}

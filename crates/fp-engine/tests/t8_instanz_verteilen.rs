//! T-8 (re 35): TW-65 instance name only under the object layout of the inspector, TW-67
//! distribute / space evenly with ties like the original's script engine.

use crate::common::*;
use fp_engine::commands::panel::{distribute_targets, script_quicksort, script_round, space_targets, AlignBox};
use fp_engine::instance_name::apply_header_rule;
use fp_engine::ToolId;
use fp_model::ElementId;
use serde_json::json;

fn converted(s: &mut fp_engine::Session) -> ElementId {
    let a = rect(s, 50.0, 50.0, 150.0, 120.0);
    select(s, &[a]);
    let v = ex(s, "modify.symbol.convertToSymbol", json!({}));
    ElementId(v["id"].as_u64().expect("id"))
}

/// re 35 §1.3: pointer + visible inspector names the instance; eraser keeps the tool layout and
/// writes nothing; switching back to the pointer names it on the next refresh (§1.2.4).
#[test]
fn tw65_name_only_under_object_layout() {
    let mut s = doc_session(400.0, 300.0);
    let _ = s.set_tool(ToolId::Eraser);
    let id = converted(&mut s);
    assert!(!apply_header_rule(&mut s, true), "eraser layout: no header");
    assert_eq!(el(&s, id).common.name, None);
    for t in [ToolId::Brush, ToolId::Marquee, ToolId::OvalMarquee, ToolId::Lasso, ToolId::MagicWand, ToolId::Pencil, ToolId::Smudge, ToolId::ReplaceColor] {
        let _ = s.set_tool(t);
        assert!(!apply_header_rule(&mut s, true), "{t:?}");
    }
    let _ = s.set_tool(ToolId::Pointer);
    assert!(!apply_header_rule(&mut s, false), "hidden inspector names nothing");
    assert_eq!(el(&s, id).common.name, None);
    assert!(apply_header_rule(&mut s, true));
    assert_eq!(el(&s, id).common.name.as_deref(), Some("Symbol"));
}

/// re 35 §1.4: a scripted command (`run_command` without dialog) follows the same rule after the
/// command; without an inspector (headless) there is no name, and reading right after the
/// command gives an empty name.
#[test]
fn tw65_script_path_same_rule() {
    let mut s = doc_session(400.0, 300.0);
    let _ = s.set_tool(ToolId::Pointer);
    let id = converted(&mut s);
    assert_eq!(el(&s, id).common.name, None, "same execution: still empty");
    // headless: never refreshed → stays empty
    assert!(!apply_header_rule(&mut s, false));
    assert_eq!(el(&s, id).common.name, None);
    // with eraser after a scripted convert: nothing
    let mut s2 = doc_session(400.0, 300.0);
    let _ = s2.set_tool(ToolId::Eraser);
    let id2 = converted(&mut s2);
    apply_header_rule(&mut s2, true);
    assert_eq!(el(&s2, id2).common.name, None);
}

/// re 35 §1.2.5: "animate selection" leaves the instance unnamed even with the pointer.
#[test]
fn tw65_animate_selection_stays_unnamed() {
    let mut s = doc_session(400.0, 300.0);
    let _ = s.set_tool(ToolId::Pointer);
    let a = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
    select(&mut s, &[a]);
    let v = ex(&mut s, "modify.animation.animateSelection", json!({"frames": 3, "offsetX": 20, "addStates": false}));
    let id = ElementId(v["id"].as_u64().expect("id"));
    let before = el(&s, id).common.name.clone();
    assert!(!apply_header_rule(&mut s, true));
    assert_eq!(el(&s, id).common.name, before);
}

/// re 35 §2.3 example: keys 5, 1, 1, 3, 1 → 4, 1, 2, 3, 0 (a stable sort gives 1, 2, 4, 3, 0).
#[test]
fn tw67_script_quicksort_example() {
    let keys = [5.0, 1.0, 1.0, 3.0, 1.0];
    let mut v: Vec<usize> = (0..5).collect();
    script_quicksort(&mut v, &|a, b| keys[*a] - keys[*b]);
    assert_eq!(v, [4, 1, 2, 3, 0]);
    assert_eq!(script_round(-12.5), -12.0);
    assert_eq!(script_round(112.5), 113.0);
}

/// re 35 table 2.4 on plain numbers: `fw.selection` = PG, E1, R1 (stack from the top).
#[test]
fn tw67_table_2_4_numbers() {
    let b = |l: f64, t: f64, w: f64, h: f64| AlignBox { left: l, top: t, width: w, height: h };
    let pg = b(465.359, 165.0, 534.641 - 465.359, 80.0);
    let boxes = [pg, b(50.0, 170.0, 100.0, 70.0), b(50.0, 50.0, 100.0, 70.0)];
    let doc = b(0.0, 0.0, 800.0, 600.0);
    let get = |v: &[(usize, (f64, f64))], k: usize| v.iter().find(|t| t.0 == k).map(|t| t.1);
    // distribute left: R1, E1, PG → E1 258, PG 465 (other axis rounded too)
    let v = distribute_targets(&boxes, &doc, "distLeft", false);
    assert_eq!(v.iter().map(|t| t.0).collect::<Vec<_>>(), [2, 1, 0]);
    assert_eq!(get(&v, 1), Some((258.0, 170.0)));
    assert_eq!(get(&v, 0), Some((465.0, 165.0)));
    assert_eq!(get(&v, 2), Some((50.0, 50.0)));
    assert_eq!(get(&distribute_targets(&boxes, &doc, "distRight", false), 1), Some((242.0, 170.0)));
    assert_eq!(get(&distribute_targets(&boxes, &doc, "distVCenter", false), 1), Some((50.0, 110.0)));
    assert_eq!(get(&distribute_targets(&boxes, &doc, "distBottom", false), 1), Some((50.0, 113.0)));
    // space horizontally: E1, R1, PG (stable "<") → R1 258, E1 and PG stay
    let v = space_targets(&boxes, &doc, true, false, None);
    assert_eq!(v, [(2, (258.0, 50.0))]);
    // space vertically: dy = round(-12.5) = -12 → PG 108
    let v = space_targets(&boxes, &doc, false, false, None);
    assert_eq!(v, [(0, (465.0, 108.0))]);
}

fn open_vector() -> Option<fp_engine::Session> {
    let src = std::path::Path::new(r"C:\Users\Administrator\Desktop\REA\re\_testdateien\par\par_vector.fw.png");
    if !src.is_file() {
        eprintln!("skipped: {src:?} missing");
        return None;
    }
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(1);
    let p = std::env::temp_dir().join(format!("fp-t8-{}-{}-par_vector.fw.png", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
    std::fs::copy(src, &p).ok()?;
    let mut s = doc_session(10.0, 10.0);
    ex(&mut s, "file.open", json!({"path": p.to_string_lossy()}));
    let _ = std::fs::remove_file(&p);
    Some(s)
}

fn id_of(s: &fp_engine::Session, name: &str) -> ElementId {
    let d = s.active().expect("doc");
    for (_, l) in fp_engine::docops::own_layers(&d.doc, 0) {
        for cel in &l.cels {
            for e in &cel.elements {
                if e.common.name.as_deref() == Some(name) {
                    return e.id;
                }
            }
        }
    }
    panic!("{name} missing")
}

fn pixel_rects(s: &fp_engine::Session) -> String {
    let d = s.active().expect("doc");
    ["PG", "E1", "R1"]
        .iter()
        .map(|n| {
            let e = fp_engine::docops::element_on_page(&d.doc, d.page, id_of(s, n)).expect("element");
            let r = fp_engine::docops::element_pixel_rect(&d.doc, &e).expect("pixelRect");
            format!("{n}:{},{},{},{}", r.left, r.top, r.right, r.bottom)
        })
        .collect::<Vec<_>>()
        .join("|")
}

/// The 16 measured values (run 4 relative to the selection, run 7 to the canvas, saved option),
/// `pixelRect` of the original in the order PG|E1|R1.
#[test]
fn tw67_sixteen_measurements_runs_4_and_7() {
    let cases = [
        // run 4, relative to the selection
        ("distTop", false, "PG:465,110,535,190|E1:50,170,150,240|R1:50,50,150,120"),
        ("distVCenter", false, "PG:465,165,535,245|E1:50,110,150,180|R1:50,50,150,120"),
        ("distBottom", false, "PG:465,165,535,245|E1:50,113,150,183|R1:50,50,150,120"),
        ("distLeft", false, "PG:465,165,535,245|E1:258,170,358,240|R1:50,50,150,120"),
        ("distHCenter", false, "PG:465,165,535,245|E1:250,170,350,240|R1:50,50,150,120"),
        ("distRight", false, "PG:465,165,535,245|E1:242,170,342,240|R1:50,50,150,120"),
        ("spaceH", false, "PG:465,165,535,245|E1:50,170,150,240|R1:258,50,358,120"),
        ("spaceV", false, "PG:465,108,535,188|E1:50,170,150,240|R1:50,50,150,120"),
        // run 7, to the canvas
        ("distTop", true, "PG:465,265,535,345|E1:50,530,150,600|R1:50,0,150,70"),
        ("distVCenter", true, "PG:465,520,535,600|E1:50,263,150,333|R1:50,0,150,70"),
        ("distBottom", true, "PG:465,520,535,600|E1:50,265,150,335|R1:50,0,150,70"),
        ("distLeft", true, "PG:731,165,801,245|E1:365,170,465,240|R1:0,50,100,120"),
        ("distHCenter", true, "PG:731,165,801,245|E1:358,170,458,240|R1:0,50,100,120"),
        ("distRight", true, "PG:731,165,801,245|E1:350,170,450,240|R1:0,50,100,120"),
        ("spaceH", true, "PG:731,165,801,245|E1:0,170,100,240|R1:365,50,465,120"),
        ("spaceV", true, "PG:465,260,535,340|E1:50,530,150,600|R1:50,0,150,70"),
    ];
    let mut bad = Vec::new();
    for (mode, to_canvas, want) in cases {
        let Some(mut s) = open_vector() else { return };
        let ids = [id_of(&s, "R1"), id_of(&s, "E1"), id_of(&s, "PG")];
        select(&mut s, &ids);
        ex(&mut s, "pi.align", json!({"mode": mode, "toCanvas": to_canvas}));
        let got = pixel_rects(&s);
        if got != want {
            bad.push(format!("{mode} canvas={to_canvas}: {got} instead of {want}"));
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
}

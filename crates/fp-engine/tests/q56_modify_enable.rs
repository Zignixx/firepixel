//! Q-56 (re 46 §5): enable rules of the Modify menu per selection, from the measured table
//! (§5.2): Snap to Pixel, Convert Path to Marquee, Group, Ungroup, Lock Selection, Group as Mask,
//! Swap Symbol and the Arrange entries.

use crate::common::{bitmap, doc_session, ellipse, ex, rect, select};
use fp_engine::Session;
use fp_model::ElementId;
use serde_json::{Value, json};

fn id_of(v: &Value) -> ElementId {
    ElementId(
        v["id"]
            .as_u64()
            .or_else(|| v["ids"][0].as_u64())
            .unwrap_or(0),
    )
}

const COLS: [&str; 7] = [
    "modify.snapToPixel",
    "modify.convertPathToMarquee",
    "modify.group",
    "modify.ungroup",
    "modify.lockSelection",
    "modify.mask.groupAsMask",
    "modify.symbol.swapSymbol",
];

fn row(s: &Session) -> Vec<u8> {
    COLS.iter()
        .map(|c| u8::from(s.command_state(c).0))
        .collect()
}

#[test]
fn modify_menu_enable_rules_follow_the_measured_table() {
    let mut s = doc_session(400.0, 300.0);
    // R1 as a plain closed path: a rectangle primitive additionally enables Ungroup (it turns
    // into a free path there; menu parity table S2/S3, `paritaet_menue.rs`)
    let r1 = id_of(&ex(
        &mut s,
        "draw.path",
        json!({"contours": [{"closed": true, "nodes": [[10, 10], [60, 10], [60, 60], [10, 60]]}]}),
    ));
    let rp = rect(&mut s, 300.0, 200.0, 350.0, 250.0);
    let e1 = ellipse(&mut s, 70.0, 10.0, 120.0, 60.0);
    let p1 = id_of(&ex(
        &mut s,
        "draw.line",
        json!({"from": [10, 100], "to": [100, 120]}),
    ));
    let b1 = bitmap(&mut s, 150, 10, 30, 30, [255, 0, 0, 255]);
    let tx = id_of(&ex(
        &mut s,
        "draw.text",
        json!({"text": "Abc", "anchor": [200, 100], "size": 20}),
    ));
    // group G of a rectangle and an ellipse
    let gr = rect(&mut s, 10.0, 150.0, 60.0, 200.0);
    let ge = ellipse(&mut s, 70.0, 150.0, 120.0, 200.0);
    select(&mut s, &[gr, ge]);
    let g = id_of(&ex(&mut s, "modify.group", json!({})));
    // instance I1
    let ir = rect(&mut s, 200.0, 150.0, 250.0, 200.0);
    select(&mut s, &[ir]);
    let i1 = id_of(&ex(
        &mut s,
        "modify.symbol.convertToSymbol",
        json!({"name": "Sym"}),
    ));
    // slice S1
    select(&mut s, &[e1]);
    let s1 = id_of(&ex(&mut s, "edit.insert.slice", json!({})));

    //                     snap  marq  grp  ungrp lock  mask  swap
    let table: [(&str, Vec<ElementId>, [u8; 7]); 10] = [
        ("none", vec![], [0, 0, 0, 0, 0, 0, 0]),
        ("R1", vec![r1], [1, 1, 1, 0, 1, 0, 0]),
        ("rect primitive", vec![rp], [1, 1, 1, 1, 1, 0, 0]),
        ("P1", vec![p1], [1, 1, 1, 0, 1, 0, 0]),
        ("R1+E1", vec![r1, e1], [1, 1, 1, 0, 1, 1, 0]),
        ("G", vec![g], [1, 0, 1, 1, 1, 0, 0]),
        ("I1", vec![i1], [0, 0, 1, 0, 1, 0, 1]),
        ("B1", vec![b1], [0, 0, 1, 0, 1, 0, 0]),
        ("S1", vec![s1], [0, 0, 1, 0, 1, 0, 0]),
        ("TX", vec![tx], [0, 0, 1, 0, 1, 0, 0]),
    ];
    let mut bad = Vec::new();
    for (name, ids, want) in table {
        select(&mut s, &ids);
        let got = row(&s);
        if got != want {
            bad.push(format!("{name}: got {got:?} want {want:?}"));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

const ARRANGE: [&str; 4] = [
    "modify.arrange.bringToFront",
    "modify.arrange.bringForward",
    "modify.arrange.sendBackward",
    "modify.arrange.sendToBack",
];

/// Arrange: needs a selection and no bitmap edit mode.
#[test]
fn arrange_needs_selection_and_no_bitmap_mode() {
    let mut s = doc_session(400.0, 300.0);
    let r1 = rect(&mut s, 10.0, 10.0, 60.0, 60.0);
    let b1 = bitmap(&mut s, 150, 10, 30, 30, [255, 0, 0, 255]);
    select(&mut s, &[]);
    for c in ARRANGE {
        assert!(!s.command_state(c).0, "{c}: grey without selection");
    }
    select(&mut s, &[r1]);
    for c in ARRANGE {
        assert!(s.command_state(c).0, "{c}: active with an object");
    }
    select(&mut s, &[b1]);
    ex(&mut s, "modify.editBitmap", json!({}));
    assert!(s.active().is_some_and(|d| d.bitmap_mode), "bitmap mode");
    for c in ARRANGE {
        assert!(!s.command_state(c).0, "{c}: grey in bitmap mode");
    }
}

/// Snap to Pixel without a fitting selection: grey, and Ctrl+K is silent (the shortcut path only
/// runs available commands).
#[test]
fn snap_to_pixel_grey_without_path() {
    let mut s = doc_session(400.0, 300.0);
    let b1 = bitmap(&mut s, 150, 10, 30, 30, [255, 0, 0, 255]);
    select(&mut s, &[]);
    assert!(!s.is_available("modify.snapToPixel"));
    select(&mut s, &[b1]);
    assert!(!s.is_available("modify.snapToPixel"));
    let ctrl_k = fp_engine::shortcuts::Shortcut {
        ctrl: true,
        alt: false,
        shift: false,
        vk: 0x4B,
    };
    assert_eq!(
        s.command_for_shortcut(&ctrl_k).as_deref(),
        Some("modify.snapToPixel")
    );
}

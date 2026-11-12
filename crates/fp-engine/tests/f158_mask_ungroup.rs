//! F158 (RE spec 81 §4, deviations D3 and D4, FB-152): structure of mask groups with filters.
//!
//! - D3 (`u2`): Group as Mask with a single filtered content makes no wrapper; Ungroup gives the
//!   mask object and the content (its filters and those added to the masked element in one list).
//! - D4 (`u7`, `u8`): Paste Inside with a wrapper (filtered content or several contents) copies
//!   the target's filters to the wrapper, the mask keeps them; without a wrapper they move.
//! - FB-152 (Firepixel extension): "Ungroup Mask Completely" ends with the mask object and the
//!   loose contents in one step; enabled only where Ungroup would leave a group.

use crate::common;
use common::*;
use fp_engine::Session;
use fp_model::{Element, ElementId, ElementKind};
use serde_json::json;

const INVERT: &str = "{d2541291-70d6-11d1-8c8000a024cdc039}";

fn fx(e: &Element) -> usize {
    e.common.effects.as_ref().map(|l| l.effects.len()).unwrap_or(0)
}

fn ids(s: &Session) -> Vec<ElementId> {
    top_elements(s).iter().map(|e| e.id).collect()
}

fn hist(s: &Session) -> usize {
    s.active().map(|d| d.history.len()).unwrap_or(0)
}

fn add_fx(s: &mut Session, id: ElementId) {
    select(s, &[id]);
    ex(s, "element.addEffect", json!({"effect": INVERT}));
}

/// C 20..200 × 40..160, C2 120..280 × 60..140 and the mask object M 10..290 × 10..190 on top.
fn scene() -> (Session, ElementId, ElementId, ElementId) {
    let mut s = doc_session(300.0, 200.0);
    let c = rect(&mut s, 20.0, 40.0, 200.0, 160.0);
    let c2 = rect(&mut s, 120.0, 60.0, 280.0, 140.0);
    let m = rect(&mut s, 10.0, 10.0, 290.0, 190.0);
    (s, c, c2, m)
}

fn paste_inside(s: &mut Session, contents: &[ElementId], target: ElementId) -> ElementId {
    select(s, contents);
    ex(s, "edit.cut", json!({}));
    select(s, &[target]);
    ElementId(ex(s, "edit.pasteInside", json!({}))["id"].as_u64().unwrap_or(0))
}

#[test]
fn d3_group_as_mask_single_filtered_content_ungroups_without_a_group() {
    let (mut s, c, c2, m) = scene();
    select(&mut s, &[c2]);
    ex(&mut s, "edit.clear", json!({}));
    add_fx(&mut s, c);
    add_fx(&mut s, m);
    select(&mut s, &[m, c]);
    let g = ElementId(ex(&mut s, "modify.mask.groupAsMask", json!({}))["id"].as_u64().unwrap_or(0));
    // values set on the masked element afterwards
    select(&mut s, &[g]);
    ex(&mut s, "element.setOpacity", json!({"opacity": 60}));
    ex(&mut s, "element.addEffect", json!({"effect": INVERT}));
    assert!(
        !s.command_state("modify.ungroupMaskCompletely").0,
        "nothing more to dissolve"
    );
    ex(&mut s, "modify.ungroup", json!({}));
    assert_eq!(ids(&s), vec![m, c], "mask object + content, no group");
    let top = top_elements(&s);
    assert_eq!(fx(&top[0]), 1, "the mask keeps its filter");
    assert!(matches!(top[1].kind, ElementKind::Path(_)));
    assert_eq!(fx(&top[1]), 2, "content filter + the one added later, one list");
    assert_eq!(top[1].common.opacity.0, 153, "60 %");
}

#[test]
fn d4_paste_inside_filtered_content_copies_the_target_filters_to_the_wrapper() {
    let (mut s, c, c2, m) = scene();
    select(&mut s, &[c2]);
    ex(&mut s, "edit.clear", json!({}));
    add_fx(&mut s, c);
    add_fx(&mut s, m);
    let g = paste_inside(&mut s, &[c], m);
    let e = el(&s, g);
    assert_eq!(fx(&e), 1, "the wrapper gets a copy");
    let ElementKind::Group(gr) = &e.kind else {
        panic!("no group")
    };
    let mask = gr.mask.as_ref().expect("mask");
    assert_eq!(mask.id, m);
    assert_eq!(fx(mask), 1, "the mask keeps its filter (u7)");
    let (wrapper, _) = e.masked_single().expect("the wrapper is the masked element");
    let ElementKind::Group(wg) = &wrapper.kind else {
        panic!("wrapper is a group")
    };
    assert_eq!(wg.children.len(), 1);
    assert!(matches!(wg.children[0].kind, ElementKind::Path(_)));
    assert_eq!(fx(&wg.children[0]), 1);
    assert_eq!(s.active().map(|d| d.selection.elements.clone()), Some(vec![g]));
    // Ungroup leaves the wrapper (u7); Ungroup Mask Completely dissolves it as well
    assert!(s.command_state("modify.ungroupMaskCompletely").0);
    ex(&mut s, "modify.ungroup", json!({}));
    let top = top_elements(&s);
    assert_eq!(top.len(), 2);
    assert_eq!(top[0].id, m);
    assert_eq!(fx(&top[0]), 1);
    assert!(matches!(&top[1].kind, ElementKind::Group(g) if g.children.len() == 1));
    assert_eq!(fx(&top[1]), 1, "the wrapper keeps the copied filter");
    s.undo().expect("undo");
    ex(&mut s, "modify.ungroupMaskCompletely", json!({}));
    let top = top_elements(&s);
    assert_eq!(top.len(), 2);
    assert_eq!(top[0].id, m);
    assert!(matches!(top[1].kind, ElementKind::Path(_)), "the content alone");
    assert_eq!(fx(&top[1]), 1);
}

#[test]
fn d4_paste_inside_two_contents_copies_the_target_filters() {
    let (mut s, c, c2, m) = scene();
    add_fx(&mut s, m);
    let g = paste_inside(&mut s, &[c2, c], m);
    let e = el(&s, g);
    assert!(e.masked_single().is_none());
    assert_eq!(fx(&e), 1);
    let ElementKind::Group(gr) = &e.kind else {
        panic!("no group")
    };
    assert_eq!(fx(gr.mask.as_ref().expect("mask")), 1, "u8: the mask keeps it");
}

#[test]
fn d4_paste_inside_single_unfiltered_content_moves_the_target_filters() {
    let (mut s, c, c2, m) = scene();
    select(&mut s, &[c2]);
    ex(&mut s, "edit.clear", json!({}));
    add_fx(&mut s, m);
    let g = paste_inside(&mut s, &[c], m);
    let e = el(&s, g);
    let (child, mask) = e.masked_single().expect("masked single element");
    assert!(matches!(child.kind, ElementKind::Path(_)));
    assert_eq!(fx(mask), 0);
    assert_eq!(fx(&e), 1);
}

#[test]
fn fb152_ungroup_mask_completely_gives_loose_contents_in_one_step() {
    let (mut s, c, c2, m) = scene();
    select(&mut s, &[m, c2, c]);
    let g = ElementId(ex(&mut s, "modify.mask.groupAsMask", json!({}))["id"].as_u64().unwrap_or(0));
    select(&mut s, &[g]);
    ex(&mut s, "element.setOpacity", json!({"opacity": 60}));
    assert!(s.command_state("modify.ungroupMaskCompletely").0);
    let before = hist(&s);
    ex(&mut s, "modify.ungroupMaskCompletely", json!({}));
    assert_eq!(hist(&s), before + 1, "one history step");
    assert_eq!(ids(&s), vec![m, c2, c]);
    assert_eq!(
        s.active().map(|d| d.selection.elements.clone()),
        Some(vec![m, c2, c])
    );
    // the contents keep their own values (the dissolved group's 60 % goes, like a second Ungroup)
    assert_eq!(el(&s, c).common.opacity.0, 255);
    s.undo().expect("undo");
    assert_eq!(ids(&s), vec![g]);
    // plain Ungroup is unchanged: mask object + the group as a normal group
    ex(&mut s, "modify.ungroup", json!({}));
    let top = top_elements(&s);
    assert_eq!(top[0].id, m);
    assert!(matches!(&top[1].kind, ElementKind::Group(g) if !g.kind.is_mask() && g.children.len() == 2));
}

#[test]
fn fb152_ungroup_mask_completely_is_disabled_without_a_leftover_group() {
    let (mut s, c, c2, m) = scene();
    // plain group: no mask
    select(&mut s, &[c2, c]);
    ex(&mut s, "modify.group", json!({}));
    assert!(!s.command_state("modify.ungroupMaskCompletely").0);
    // masked single element: Ungroup leaves no group
    let (mut s2, c, c2, m2) = scene();
    select(&mut s2, &[c2]);
    ex(&mut s2, "edit.clear", json!({}));
    select(&mut s2, &[m2, c]);
    ex(&mut s2, "modify.mask.groupAsMask", json!({}));
    assert!(!s2.command_state("modify.ungroupMaskCompletely").0);
    let _ = m;
}

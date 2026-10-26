//! FB-35 / RE spec 68 section 6: the inverted mask (Firepixel extension) is stored in the mask
//! group `GRPv` as the Firepixel-own tag `fpIb` = 1, only when on. Firepixel reads it back; files
//! without the tag (all files of the reference format) keep a plain mask and are written unchanged.

use std::collections::HashMap;
use std::sync::Arc;

use fp_color::Argb;
use fp_fwpng::read::{ReadCtx, read_element};
use fp_fwpng::write::{CachePolicy, MASK_INVERT_EXT, WriteCtx, write_element};
use fp_geom::Rect;
use fp_model::{Document, Element, ElementKind, LayerSlot, PathAttrs, RawNode, RawValue};

fn masked(doc: &mut Document, invert: bool) -> Element {
    let content = Element::new_rectangle(
        &mut doc.ids,
        Rect::new(0.0, 0.0, 40.0, 40.0),
        0.0,
        PathAttrs::solid_fill(Argb(0xff00_ff00)),
    );
    let mask = Element::new_rectangle(
        &mut doc.ids,
        Rect::new(10.0, 10.0, 20.0, 20.0),
        0.0,
        PathAttrs::solid_fill(Argb(0xffff_0000)),
    );
    let mut g =
        Element::new_mask_group(&mut doc.ids, Arc::new(mask), vec![Arc::new(content)], true);
    if let ElementKind::Group(gr) = &mut g.kind {
        gr.mask_flags.invert = invert;
    }
    g
}

fn children(n: &RawNode) -> &[RawNode] {
    match &n.value {
        RawValue::List(v) => v,
        _ => &[],
    }
}

fn invert_of(e: &Element) -> bool {
    matches!(&e.kind, ElementKind::Group(g) if g.mask_flags.invert)
}

fn read_back(n: &RawNode) -> Element {
    let tiles = HashMap::new();
    let mut ctx = ReadCtx::new(&tiles);
    read_element(n, &mut ctx)
}

#[test]
fn inverted_mask_writes_the_own_tag_and_reads_it_back() {
    let mut doc = Document::new(40.0, 40.0, Argb(0xffff_ffff));
    let g = masked(&mut doc, true);
    let mut w = WriteCtx::new(CachePolicy::Keep, Vec::new());
    let node = write_element(&g, &mut w);
    assert!(
        children(&node).iter().any(|n| n.key == MASK_INVERT_EXT),
        "fpIb written"
    );
    let back = read_back(&node);
    assert!(invert_of(&back));
    assert!(!back.extra.has_raw(MASK_INVERT_EXT), "the tag is consumed");
    // a reader without the tag sees a plain mask
    let mut plain = node.clone();
    if let RawValue::List(v) = &mut plain.value {
        v.retain(|n| n.key != MASK_INVERT_EXT);
    }
    assert!(!invert_of(&read_back(&plain)));
}

#[test]
fn plain_mask_carries_no_tag() {
    let mut doc = Document::new(40.0, 40.0, Argb(0xffff_ffff));
    let g = masked(&mut doc, false);
    let mut w = WriteCtx::new(CachePolicy::Keep, Vec::new());
    let node = write_element(&g, &mut w);
    assert!(!children(&node).iter().any(|n| n.key == MASK_INVERT_EXT));
    assert!(!invert_of(&read_back(&node)));
}

#[test]
fn document_round_trip_keeps_the_inverted_mask() {
    let mut doc = Document::new(40.0, 40.0, Argb(0xffff_ffff));
    let g = masked(&mut doc, true);
    let p = Arc::make_mut(&mut doc.pages[0]);
    if let LayerSlot::Own(l) = &mut p.root.layers[0] {
        Arc::make_mut(&mut Arc::make_mut(l).cels[0]).elements = vec![Arc::new(g)];
    }
    let bytes = fp_fwpng::write(
        &doc,
        &fp_fwpng::WriteOptions::default(),
        &fp_fwpng::NoFlattener,
    )
    .expect("write");
    let back = fp_fwpng::read(&bytes).expect("read");
    let LayerSlot::Own(l) = &back.pages[0].root.layers[0] else {
        panic!("no layer")
    };
    assert!(invert_of(&l.cels[0].elements[0]));
}

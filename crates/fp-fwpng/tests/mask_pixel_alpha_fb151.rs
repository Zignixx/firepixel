//! FB-151 (TW-298): the mask mode "Pixel Alpha" (Firepixel extension) is stored in the mask group
//! `GRPv` as the Firepixel-own tag `fpAb` = 1 on a path-outline group (`GRTi` stays "mask to
//! path"), only when on. Firepixel reads it back; other readers see path outline / alpha channel.

use std::collections::HashMap;
use std::sync::Arc;

use fp_color::Argb;
use fp_fwpng::read::{ReadCtx, read_element};
use fp_fwpng::write::{CachePolicy, MASK_PIXEL_ALPHA_EXT, WriteCtx, write_element};
use fp_geom::Rect;
use fp_model::{
    Document, Element, ElementKind, GroupKind, LayerSlot, PathAttrs, RawNode, RawValue,
};

fn masked(doc: &mut Document, pixel: bool, kind: GroupKind) -> Element {
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
        PathAttrs::solid_fill(Argb(0x80ff_0000)),
    );
    let mut g =
        Element::new_mask_group(&mut doc.ids, Arc::new(mask), vec![Arc::new(content)], true);
    if let ElementKind::Group(gr) = &mut g.kind {
        gr.kind = kind;
        gr.mask_flags.pixel_alpha = pixel;
    }
    g
}

fn children(n: &RawNode) -> &[RawNode] {
    match &n.value {
        RawValue::List(v) => v,
        _ => &[],
    }
}

fn pixel_of(e: &Element) -> Option<(GroupKind, bool)> {
    match &e.kind {
        ElementKind::Group(g) => Some((g.kind, g.mask_flags.pixel_alpha)),
        _ => None,
    }
}

fn read_back(n: &RawNode) -> Element {
    let tiles = HashMap::new();
    let mut ctx = ReadCtx::new(&tiles);
    read_element(n, &mut ctx)
}

#[test]
fn pixel_alpha_writes_the_own_tag_and_reads_it_back() {
    let mut doc = Document::new(40.0, 40.0, Argb(0xffff_ffff));
    let g = masked(&mut doc, true, GroupKind::VectorMask);
    let mut w = WriteCtx::new(CachePolicy::Keep, Vec::new());
    let node = write_element(&g, &mut w);
    assert!(
        children(&node)
            .iter()
            .any(|n| n.key == MASK_PIXEL_ALPHA_EXT),
        "fpAb written"
    );
    let back = read_back(&node);
    assert_eq!(pixel_of(&back), Some((GroupKind::VectorMask, true)));
    assert!(
        !back.extra.has_raw(MASK_PIXEL_ALPHA_EXT),
        "the tag is consumed"
    );
    // a reader without the tag sees path outline / alpha channel
    let mut plain = node.clone();
    if let RawValue::List(v) = &mut plain.value {
        v.retain(|n| n.key != MASK_PIXEL_ALPHA_EXT);
    }
    assert_eq!(
        pixel_of(&read_back(&plain)),
        Some((GroupKind::VectorMask, false))
    );
}

#[test]
fn no_tag_when_off_or_in_grayscale() {
    let mut doc = Document::new(40.0, 40.0, Argb(0xffff_ffff));
    for (pixel, kind) in [
        (false, GroupKind::VectorMask),
        (true, GroupKind::BitmapMask),
    ] {
        let g = masked(&mut doc, pixel, kind);
        let mut w = WriteCtx::new(CachePolicy::Keep, Vec::new());
        let node = write_element(&g, &mut w);
        assert!(
            !children(&node)
                .iter()
                .any(|n| n.key == MASK_PIXEL_ALPHA_EXT),
            "{kind:?}"
        );
    }
}

#[test]
fn document_round_trip_keeps_pixel_alpha() {
    let mut doc = Document::new(40.0, 40.0, Argb(0xffff_ffff));
    let g = masked(&mut doc, true, GroupKind::VectorMask);
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
    assert_eq!(
        pixel_of(&l.cels[0].elements[0]),
        Some((GroupKind::VectorMask, true))
    );
}

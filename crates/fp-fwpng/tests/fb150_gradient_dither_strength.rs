//! FB-150 / TW-294: the gradient dither strength (Firepixel extension) is stored only in the
//! Firepixel-own marker `fpSi` of the fill, written when it is not the default 100; other readers
//! see the plain gradient dither. It round-trips through a whole document.

use std::collections::HashMap;
use std::sync::Arc;

use fp_color::Argb;
use fp_fwpng::read::{ReadCtx, read_fill};
use fp_fwpng::write::{CachePolicy, FILL_DITHER_STRENGTH_EXT, WriteCtx, write_fill};
use fp_geom::Rect;
use fp_model::{
    Document, Element, ElementKind, Fill, LayerSlot, PathAttrs, RawNode, RawValue, TagKey,
};

fn int_of(nodes: &[RawNode], key: &[u8; 4]) -> Option<i32> {
    nodes
        .iter()
        .find(|n| n.key == TagKey::new(key))
        .and_then(|n| match n.value {
            RawValue::Int(v) => Some(v as i32),
            _ => None,
        })
}

fn read_back(nodes: &[RawNode]) -> Fill {
    let tiles = HashMap::new();
    let mut ctx = ReadCtx::new(&tiles);
    read_fill(nodes, &mut ctx)
}

fn write(f: &Fill) -> Vec<RawNode> {
    let mut w = WriteCtx::new(CachePolicy::Keep, Vec::new());
    write_fill(f, &mut w)
}

#[test]
fn strength_lives_in_its_marker() {
    for v in [0, 1, 55, 99, 101, 250, 400] {
        let mut f = Fill::solid();
        f.gradient_dither_strength = v;
        let nodes = write(&f);
        assert_eq!(int_of(&nodes, b"fpSi"), Some(v));
        let back = read_back(&nodes);
        assert_eq!(back.gradient_dither_strength, v);
        assert!(
            !back.extra.has_raw(FILL_DITHER_STRENGTH_EXT),
            "the marker is consumed"
        );
    }
    // default 100: no marker, the nodes are exactly those of a plain fill
    let plain = write(&Fill::solid());
    assert_eq!(int_of(&plain, b"fpSi"), None);
    assert_eq!(
        read_back(&plain).gradient_dither_strength,
        Fill::GRADIENT_DITHER_STRENGTH_DEFAULT
    );
    // a reader without the marker sees the plain fill
    let mut f = Fill::solid();
    f.gradient_dither_strength = 250;
    let without: Vec<RawNode> = write(&f)
        .into_iter()
        .filter(|n| n.key != FILL_DITHER_STRENGTH_EXT)
        .collect();
    assert_eq!(without, plain);
}

#[test]
fn out_of_range_marker_is_clamped() {
    let mut f = Fill::solid();
    f.gradient_dither_strength = 50;
    let mut nodes = write(&f);
    let n = nodes
        .iter_mut()
        .find(|n| n.key == FILL_DITHER_STRENGTH_EXT)
        .expect("marker");
    n.value = RawValue::Int(5000);
    assert_eq!(
        read_back(&nodes).gradient_dither_strength,
        Fill::GRADIENT_DITHER_STRENGTH_MAX
    );
}

#[test]
fn document_round_trip_keeps_strength() {
    let mut doc = Document::new(100.0, 80.0, Argb(0xffff_ffff));
    let mut attrs = PathAttrs::solid_fill(Argb(0xff00_0000));
    if let Some(f) = attrs.fill.as_mut() {
        f.gradient_dither_strength = 300;
    }
    let e = Element::new_rectangle(&mut doc.ids, Rect::new(10.0, 10.0, 60.0, 50.0), 0.0, attrs);
    let p = Arc::make_mut(&mut doc.pages[0]);
    if let LayerSlot::Own(l) = &mut p.root.layers[0] {
        Arc::make_mut(&mut Arc::make_mut(l).cels[0]).elements = vec![Arc::new(e)];
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
    let ElementKind::Path(p) = &l.cels[0].elements[0].kind else {
        panic!("no path")
    };
    let f = p.attrs.fill.as_ref().expect("fill");
    assert_eq!(f.gradient_dither_strength, 300);
}

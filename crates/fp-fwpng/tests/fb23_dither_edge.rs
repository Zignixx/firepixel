//! FB-23 / TW-203: the "Dither" fill edge (Firepixel extension) is stored as a feather (`FETi` 2,
//! `FEFi` = width; `FETi` 0 at width 0) plus the Firepixel-own marker `fpEi` = 3. Firepixel reads
//! it back as dither; a reader that ignores the marker sees a plain feather (or hard) edge.

use std::collections::HashMap;
use std::sync::Arc;

use fp_color::Argb;
use fp_fwpng::read::{ReadCtx, read_fill};
use fp_fwpng::write::{CachePolicy, FILL_EDGE_EXT, WriteCtx, write_fill};
use fp_geom::Rect;
use fp_model::{
    Document, Element, ElementKind, Fill, LayerSlot, PathAttrs, RawNode, RawValue, TagKey,
};

fn dither_fill(width: i32) -> Fill {
    let mut f = Fill::solid();
    f.edge = Fill::EDGE_DITHER;
    f.feather = width;
    f
}

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

#[test]
fn dither_fill_is_written_as_feather_with_marker() {
    let mut w = WriteCtx::new(CachePolicy::Keep, Vec::new());
    let nodes = write_fill(&dither_fill(6), &mut w);
    assert_eq!(int_of(&nodes, b"FETi"), Some(Fill::EDGE_FEATHER));
    assert_eq!(int_of(&nodes, b"FEFi"), Some(6));
    assert_eq!(int_of(&nodes, b"fpEi"), Some(Fill::EDGE_DITHER));
    // Firepixel reads dither + width back
    let back = read_back(&nodes);
    assert_eq!((back.edge, back.feather), (Fill::EDGE_DITHER, 6));
    assert!(
        !back.extra.has_raw(FILL_EDGE_EXT),
        "the marker is consumed, not kept as an unknown tag"
    );
    // the plain path (a reader without the marker) shows a feather of the same width
    let plain: Vec<RawNode> = nodes
        .into_iter()
        .filter(|n| n.key != FILL_EDGE_EXT)
        .collect();
    let back = read_back(&plain);
    assert_eq!((back.edge, back.feather), (Fill::EDGE_FEATHER, 6));
}

#[test]
fn dither_width_zero_is_written_as_hard() {
    let mut w = WriteCtx::new(CachePolicy::Keep, Vec::new());
    let nodes = write_fill(&dither_fill(0), &mut w);
    assert_eq!(int_of(&nodes, b"FETi"), Some(Fill::EDGE_HARD));
    assert_eq!(int_of(&nodes, b"fpEi"), Some(Fill::EDGE_DITHER));
    assert_eq!(read_back(&nodes).edge, Fill::EDGE_DITHER);
    let plain: Vec<RawNode> = nodes
        .into_iter()
        .filter(|n| n.key != FILL_EDGE_EXT)
        .collect();
    assert_eq!(read_back(&plain).edge, Fill::EDGE_HARD);
}

#[test]
fn other_edges_carry_no_marker() {
    for (edge, feather) in [(0, 0), (1, 0), (2, 9)] {
        let mut f = Fill::solid();
        f.edge = edge;
        f.feather = feather;
        let mut w = WriteCtx::new(CachePolicy::Keep, Vec::new());
        let nodes = write_fill(&f, &mut w);
        assert_eq!(int_of(&nodes, b"fpEi"), None);
        assert_eq!(int_of(&nodes, b"FETi"), Some(edge));
        let back = read_back(&nodes);
        assert_eq!((back.edge, back.feather), (edge, feather));
    }
}

#[test]
fn document_round_trip_keeps_dither_and_amount() {
    let mut doc = Document::new(100.0, 80.0, Argb(0xffff_ffff));
    let mut attrs = PathAttrs::solid_fill(Argb(0xff00_0000));
    attrs.fill = Some(dither_fill(12));
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
    assert_eq!((f.edge, f.feather), (Fill::EDGE_DITHER, 12));
}

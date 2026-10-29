//! FB-44: "Lock pattern to document" (Firepixel extension) is stored only in the Firepixel-own fill
//! marker `fpEi` (bit 4, beside the dither edge's 3); at the default nothing is written, so the
//! reference-format output is unchanged.

use std::collections::HashMap;
use std::sync::Arc;

use fp_color::Argb;
use fp_fwpng::read::{ReadCtx, read_fill};
use fp_fwpng::write::{CachePolicy, FILL_EDGE_EXT, WriteCtx, write_fill};
use fp_geom::Rect;
use fp_model::attrs::Pattern;
use fp_model::{
    Document, Element, ElementKind, Fill, LayerSlot, PathAttrs, RawNode, RawValue, TagKey,
};

fn pattern_fill(lock: bool) -> Fill {
    let mut f = Fill::solid();
    f.category = "fc_Pattern".into();
    f.name = "Weave".into();
    f.user_name = Some("Weave".into());
    f.shape = 15;
    f.pattern = Some(Pattern {
        name: "Weave".into(),
        user_name: Some("Weave".into()),
        image: None,
        extra: Default::default(),
    });
    f.lock_to_document = lock;
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

fn write(f: &Fill) -> Vec<RawNode> {
    let mut w = WriteCtx::new(CachePolicy::Keep, Vec::new());
    write_fill(f, &mut w)
}

#[test]
fn lock_is_written_as_marker_bit_and_read_back() {
    let nodes = write(&pattern_fill(true));
    assert_eq!(int_of(&nodes, b"fpEi"), Some(Fill::EXT_LOCK_TO_DOCUMENT));
    let back = read_back(&nodes);
    assert!(back.lock_to_document);
    assert!(back.locked_to_document());
    assert_eq!(
        back.edge,
        Fill::EDGE_ANTIALIAS,
        "no dither from the lock bit"
    );
    assert!(
        !back.extra.has_raw(FILL_EDGE_EXT),
        "the marker is consumed, not kept as an unknown tag"
    );
    // a reader without the marker sees the plain pattern
    let plain: Vec<RawNode> = nodes
        .into_iter()
        .filter(|n| n.key != FILL_EDGE_EXT)
        .collect();
    let back = read_back(&plain);
    assert!(!back.lock_to_document);
    assert_eq!(back.shape, 15);
}

#[test]
fn lock_and_dither_share_the_marker() {
    let mut f = pattern_fill(true);
    f.edge = Fill::EDGE_DITHER;
    f.feather = 5;
    let nodes = write(&f);
    assert_eq!(
        int_of(&nodes, b"fpEi"),
        Some(Fill::EDGE_DITHER | Fill::EXT_LOCK_TO_DOCUMENT)
    );
    let back = read_back(&nodes);
    assert!(back.lock_to_document);
    assert_eq!((back.edge, back.feather), (Fill::EDGE_DITHER, 5));
    // an older Firepixel file (marker 3 only) still reads as dither without the lock
    let mut f = pattern_fill(false);
    f.edge = Fill::EDGE_DITHER;
    f.feather = 5;
    let nodes = write(&f);
    assert_eq!(int_of(&nodes, b"fpEi"), Some(Fill::EDGE_DITHER));
    let back = read_back(&nodes);
    assert!(!back.lock_to_document);
    assert_eq!(back.edge, Fill::EDGE_DITHER);
}

#[test]
fn default_writes_the_reference_format_unchanged() {
    let nodes = write(&pattern_fill(false));
    assert_eq!(
        int_of(&nodes, b"fpEi"),
        None,
        "no Firepixel tag at the default"
    );
    // a reference-format fill reads without the lock and writes back node for node
    let back = read_back(&nodes);
    assert!(!back.lock_to_document);
    assert_eq!(write(&back), nodes);
}

fn doc_with(fill: Fill) -> Document {
    let mut doc = Document::new(100.0, 80.0, Argb(0xffff_ffff));
    let mut attrs = PathAttrs::solid_fill(Argb(0xff00_0000));
    attrs.fill = Some(fill);
    let e = Element::new_rectangle(&mut doc.ids, Rect::new(10.0, 10.0, 60.0, 50.0), 0.0, attrs);
    let p = Arc::make_mut(&mut doc.pages[0]);
    if let LayerSlot::Own(l) = &mut p.root.layers[0] {
        Arc::make_mut(&mut Arc::make_mut(l).cels[0]).elements = vec![Arc::new(e)];
    }
    doc
}

fn first_fill(doc: &Document) -> Fill {
    let LayerSlot::Own(l) = &doc.pages[0].root.layers[0] else {
        panic!("no layer")
    };
    let ElementKind::Path(p) = &l.cels[0].elements[0].kind else {
        panic!("no path")
    };
    p.attrs.fill.clone().expect("fill")
}

#[test]
fn document_round_trip_keeps_the_lock() {
    let write_doc = |d: &Document| {
        fp_fwpng::write(
            d,
            &fp_fwpng::WriteOptions::default(),
            &fp_fwpng::NoFlattener,
        )
        .expect("write")
    };
    let locked = write_doc(&doc_with(pattern_fill(true)));
    let back = fp_fwpng::read(&locked).expect("read");
    assert!(first_fill(&back).lock_to_document);
    // default: the same bytes as a file without the feature, read back unlocked
    let plain = write_doc(&doc_with(pattern_fill(false)));
    let back = fp_fwpng::read(&plain).expect("read");
    assert!(!first_fill(&back).lock_to_document);
    assert_eq!(
        write_doc(&back),
        plain,
        "unlocked file round-trips byte for byte"
    );
}

#[test]
fn lock_and_dither_density_live_side_by_side() {
    // FB-44 and FB-38 together: the lock bit in `fpEi`, the density in its own `fpOi`
    let mut f = pattern_fill(true);
    f.dither_density = 40;
    let nodes = write(&f);
    assert_eq!(int_of(&nodes, b"fpEi"), Some(Fill::EXT_LOCK_TO_DOCUMENT));
    assert_eq!(int_of(&nodes, b"fpOi"), Some(40));
    let back = read_back(&nodes);
    assert!(back.lock_to_document);
    assert_eq!(back.dither_density, 40);
}

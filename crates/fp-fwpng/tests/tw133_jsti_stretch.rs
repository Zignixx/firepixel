//! TW-133 (re 50 section 1.2): the reference text engine never stores "stretched" alignment, so
//! a file with `JSTi 4` loads as left, and saving it again writes 0.

use std::sync::Arc;

use fp_color::Argb;
use fp_geom::Point;
use fp_model::{Document, Element, ElementKind, LayerSlot};

/// Document with one text whose initial attributes and runs carry `alignment`.
fn doc_with_alignment(alignment: i32) -> Document {
    let mut doc = Document::new(200.0, 100.0, Argb(0xffff_ffff));
    let mut t = Element::new_text(
        &mut doc.ids,
        "Abc 123",
        "ArialMT",
        12.0,
        Argb(0xff00_0000),
        Point::new(5.0, 40.0),
    );
    if let ElementKind::Text(tx) = &mut t.kind {
        tx.initial.alignment = Some(alignment);
        for r in &mut tx.runs {
            r.attrs.alignment = Some(alignment);
        }
    }
    let p = Arc::make_mut(&mut doc.pages[0]);
    if let LayerSlot::Own(l) = &mut p.root.layers[0] {
        Arc::make_mut(&mut Arc::make_mut(l).cels[0]).elements = vec![Arc::new(t)];
    }
    doc
}

/// Alignment of the initial attributes and of every run of the first text.
fn alignments(doc: &Document) -> Vec<Option<i32>> {
    let LayerSlot::Own(l) = &doc.pages[0].root.layers[0] else {
        panic!("no layer")
    };
    match &l.cels[0].elements[0].kind {
        ElementKind::Text(t) => std::iter::once(t.initial.alignment)
            .chain(t.runs.iter().map(|r| r.attrs.alignment))
            .collect(),
        _ => panic!("no text"),
    }
}

fn write(doc: &Document) -> Vec<u8> {
    fp_fwpng::write(
        doc,
        &fp_fwpng::WriteOptions::default(),
        &fp_fwpng::NoFlattener,
    )
    .expect("write")
}

#[test]
fn jsti_4_loads_as_left_and_saves_as_0() {
    // fixture: a file written with JSTi 4 (as older files carry it)
    let back = fp_fwpng::read(&write(&doc_with_alignment(4))).expect("read");
    let a = alignments(&back);
    assert!(!a.is_empty());
    assert!(
        a.iter().all(|v| *v == Some(0)),
        "stretched loads as left: {a:?}"
    );
    // saved again: left stays 0
    let again = fp_fwpng::read(&write(&back)).expect("read again");
    assert!(alignments(&again).iter().all(|v| *v == Some(0)));
}

#[test]
fn vertical_stretch_loads_as_vertical_left_and_others_are_kept() {
    let back = fp_fwpng::read(&write(&doc_with_alignment(0x14))).expect("read");
    assert!(alignments(&back).iter().all(|v| *v == Some(0x10)));
    for code in [1, 2, 3, 0x13] {
        let back = fp_fwpng::read(&write(&doc_with_alignment(code))).expect("read");
        assert!(
            alignments(&back).iter().all(|v| *v == Some(code)),
            "code {code} kept"
        );
    }
}

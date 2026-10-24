//! RE spec 68 M-6: a grayscale bitmap mask (`GRTi` 1) is stored with inverted colours in the file
//! (07 §7.5: value = α·(255 − grey)) but held in display form in memory (white = visible). The
//! conversion happens only at file I/O; an alpha-channel mask (`GRTi` 2) is stored as it is.

use std::sync::Arc;

use fp_color::Argb;
use fp_geom::{Point, Rect};
use fp_model::image::TileKind;
use fp_model::{Document, Element, ElementKind, LayerSlot, PathAttrs};

/// Document with a white square masked by a 100 × 20 grey ramp (display form: x·2).
fn doc(vector: bool) -> Document {
    let mut doc = Document::new(100.0, 100.0, Argb(0xffff_ffff));
    let content = Element::new_rectangle(
        &mut doc.ids,
        Rect::new(0.0, 0.0, 100.0, 100.0),
        0.0,
        PathAttrs::solid_fill(Argb(0xffff_ffff)),
    );
    let px: Vec<u8> = (0..100 * 20)
        .flat_map(|i| {
            let v = ((i % 100) * 2) as u8;
            [v, v, v, 255]
        })
        .collect();
    let mut tiles = 1;
    let mask = Element::new_bitmap(&mut doc.ids, Point::new(0.0, 0.0), 100, 20, &px, &mut tiles);
    let g = Element::new_mask_group(
        &mut doc.ids,
        Arc::new(mask),
        vec![Arc::new(content)],
        vector,
    );
    let p = Arc::make_mut(&mut doc.pages[0]);
    if let LayerSlot::Own(l) = &mut p.root.layers[0] {
        Arc::make_mut(&mut Arc::make_mut(l).cels[0]).elements = vec![Arc::new(g)];
    }
    doc
}

fn mask_rgba(doc: &Document) -> Vec<u8> {
    let LayerSlot::Own(l) = &doc.pages[0].root.layers[0] else {
        panic!("layer");
    };
    let ElementKind::Group(g) = &l.cels[0].elements[0].kind else {
        panic!("group");
    };
    let Some(m) = &g.mask else { panic!("mask") };
    let ElementKind::Bitmap(b) = &m.kind else {
        panic!("bitmap mask")
    };
    b.image.to_rgba().0
}

/// First stored image tile of the file, ARGB bytes.
fn file_tile(bytes: &[u8]) -> Vec<u8> {
    let raw = fp_fwpng::read_raw(bytes, true).expect("raw");
    let t = raw
        .tiles
        .values()
        .find(|t| t.kind == TileKind::Image)
        .expect("an image tile");
    t.pixels.to_vec()
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
fn grayscale_bitmap_mask_is_inverted_only_in_the_file() {
    let d = doc(false);
    let before = mask_rgba(&d);
    let bytes = write(&d);
    let t = file_tile(&bytes);
    // pixel x = 10: display grey 20 → file 235 (ARGB, alpha first)
    assert_eq!(&t[40..44], &[255, 235, 235, 235]);
    assert_eq!(
        &t[0..4],
        &[255, 255, 255, 255],
        "black (hidden) is white in the file"
    );
    let back = fp_fwpng::read(&bytes).expect("read");
    assert_eq!(mask_rgba(&back), before, "display form after reading");
    // and a second round trip keeps it
    let again = fp_fwpng::read(&write(&back)).expect("read");
    assert_eq!(mask_rgba(&again), before);
}

#[test]
fn alpha_channel_bitmap_mask_is_stored_as_it_is() {
    let d = doc(true);
    let before = mask_rgba(&d);
    let bytes = write(&d);
    let t = file_tile(&bytes);
    assert_eq!(&t[40..44], &[255, 20, 20, 20]);
    assert_eq!(mask_rgba(&fp_fwpng::read(&bytes).expect("read")), before);
}

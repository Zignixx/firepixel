//! TW-144 (re 50 section 3): `TRNs` stores paragraph ends as `\r` and forced line breaks as
//! U+0003, both written and read unchanged.

use std::sync::Arc;

use fp_color::Argb;
use fp_geom::Point;
use fp_model::{Document, Element, ElementKind, LayerSlot};

const TEXT: &str = "Zeile eins\rZeile zwei\u{3}weiter\rZeile drei";

fn doc_with_text() -> Document {
    let mut doc = Document::new(200.0, 100.0, Argb(0xffff_ffff));
    let t = Element::new_text(
        &mut doc.ids,
        TEXT,
        "ArialMT",
        12.0,
        Argb(0xff00_0000),
        Point::new(5.0, 40.0),
    );
    let p = Arc::make_mut(&mut doc.pages[0]);
    if let LayerSlot::Own(l) = &mut p.root.layers[0] {
        Arc::make_mut(&mut Arc::make_mut(l).cels[0]).elements = vec![Arc::new(t)];
    }
    doc
}

fn first_text(doc: &Document) -> String {
    let LayerSlot::Own(l) = &doc.pages[0].root.layers[0] else {
        panic!("no layer")
    };
    match &l.cels[0].elements[0].kind {
        ElementKind::Text(t) => t.plain_text(),
        _ => panic!("no text"),
    }
}

/// Does `hay` contain `needle` as UTF-8, UTF-16 BE or UTF-16 LE?
fn contains_any(hay: &[u8], needle: &str) -> bool {
    let utf8 = needle.as_bytes().to_vec();
    let be: Vec<u8> = needle
        .encode_utf16()
        .flat_map(|u| u.to_be_bytes())
        .collect();
    let le: Vec<u8> = needle
        .encode_utf16()
        .flat_map(|u| u.to_le_bytes())
        .collect();
    [utf8, be, le]
        .iter()
        .any(|n| hay.windows(n.len()).any(|w| w == n.as_slice()))
}

#[test]
fn trns_keeps_cr_and_u0003() {
    let bytes = fp_fwpng::write(
        &doc_with_text(),
        &fp_fwpng::WriteOptions::default(),
        &fp_fwpng::NoFlattener,
    )
    .expect("write");
    let raw = fp_fwpng::read_raw(&bytes, false).expect("raw");
    let ts = raw.ts_bytes.expect("mkTS");
    assert!(contains_any(&ts, "eins\rZeile"), "CR stored in TRNs");
    assert!(
        contains_any(&ts, "zwei\u{3}weiter"),
        "U+0003 stored in TRNs"
    );
    assert!(!contains_any(&ts, "eins\nZeile"), "no LF");
    assert!(!contains_any(&ts, "eins\r\nZeile"), "no CRLF");
    let back = fp_fwpng::read(&bytes).expect("read");
    assert_eq!(first_text(&back), TEXT);
}

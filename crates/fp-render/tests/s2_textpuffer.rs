//! S-2 (re 30 §4.2, re 28 D3): input buffer of the live effects for a text with a fixed frame
//! (`fx_text`, 40 px "Fx Ag" Arial Bold, frame 40–200): measured buffer 23,46–167,112.

mod common;

use fp_model::{ElementKind, LayerSlot};
use fp_render::compose::{fixed_frame_box, text_buffer_from_box, text_buffer_rect};

#[test]
fn fx_text_fixed_frame_buffer() {
    let p = common::rea()
        .join("re")
        .join("_testdateien")
        .join("par")
        .join("fx")
        .join("base")
        .join("fx_text.fw.png");
    let Some(doc) = common::load(&p) else {
        eprintln!("skipped: {p:?} missing");
        return;
    };
    let fonts = fp_text::FontDb::system();
    let mut found = None;
    for l in &doc.pages[0].root.layers {
        let LayerSlot::Own(l) = l else { continue };
        for c in &l.cels {
            for e in &c.elements {
                if let ElementKind::Text(t) = &e.kind {
                    found = text_buffer_rect(&fonts, &doc, t);
                }
            }
        }
    }
    let r = found.expect("text buffer");
    assert_eq!([r.left, r.top, r.right, r.bottom], [23, 46, 167, 112]);
}

/// Rule without the font file: point-text box of the bold 40 px line (40,52–150,102) moves onto
/// the frame bottom 104 → B' 40,54–150,104 → buffer 23,46–167,112 (re 30 §4.2).
#[test]
fn fixed_frame_rule() {
    let mut b = [40.0, 52.0, 150.0, 102.0];
    fixed_frame_box(&mut b, 104.0, 1);
    assert_eq!(b, [40.0, 54.0, 150.0, 104.0]);
    let r = text_buffer_from_box(b);
    assert_eq!([r.left, r.top, r.right, r.bottom], [23, 46, 167, 112]);
    // several lines: only the bottom grows; a frame above the content changes nothing
    let mut m = [40.0, 52.0, 150.0, 150.0];
    fixed_frame_box(&mut m, 160.0, 2);
    assert_eq!(m, [40.0, 52.0, 150.0, 160.0]);
    let mut n = [40.0, 52.0, 150.0, 102.0];
    fixed_frame_box(&mut n, 90.0, 1);
    assert_eq!(n, [40.0, 52.0, 150.0, 102.0]);
}

//! T-5 (re 30 §4): input buffer of the live effects of a text = content rectangle rounded outward,
//! ± 15 % of width and height, rounded outward again. Checked against the measured table of
//! re 28 D3 (Arial, advances = sum of rounded glyph advances).

use fp_geom::IRect;
use fp_render::compose::{
    TEXT_BUFFER_ARIAL, text_buffer_from_box, text_content_box, text_effect_buffer,
};

/// (size, text, raw-left, raw-top, advance, measured buffer l, t, r, b)
const TABLE: [(f64, &str, f64, f64, f64, [i32; 4]); 16] = [
    (12.0, "Fx Ag", 40.0, 60.0, 31.0, [35, 55, 76, 76]),
    (12.0, "ii", 40.0, 60.0, 6.0, [39, 55, 47, 76]),
    (24.0, "ii", 40.0, 60.0, 10.0, [38, 51, 52, 90]),
    (40.0, "Fx Ag", 40.0, 60.0, 104.0, [24, 46, 160, 110]),
    (40.0, "ii", 40.0, 60.0, 18.0, [37, 46, 61, 110]),
    (40.0, "WWWW", 80.0, 80.0, 152.0, [57, 66, 255, 130]),
    (40.0, "x", 80.0, 80.0, 20.0, [77, 66, 103, 130]),
    (72.0, "Fx Ag", 40.0, 60.0, 188.0, [11, 36, 257, 148]),
    (72.0, "ii", 40.0, 60.0, 32.0, [35, 36, 77, 148]),
    (72.0, "i", 80.0, 80.0, 16.0, [77, 56, 99, 168]),
    (72.0, "iiii", 80.0, 80.0, 64.0, [70, 56, 154, 168]),
    (72.0, "x", 80.0, 80.0, 36.0, [74, 56, 122, 168]),
    (72.0, "WWWW", 80.0, 80.0, 272.0, [39, 56, 393, 168]),
    (72.0, "Fx Ag", 123.0, 97.0, 188.0, [94, 73, 340, 185]),
    (100.0, "Fx Ag", 80.0, 80.0, 262.0, [39, 47, 381, 203]),
    (100.0, "ii", 80.0, 80.0, 44.0, [73, 47, 131, 203]),
];

#[test]
fn worked_example_72pt() {
    let (asc, ymax) = TEXT_BUFFER_ARIAL;
    let b = text_content_box(40.0, 60.0, 188.0, 72.0, asc, ymax);
    assert_eq!(b, [40.0, 49.0, 228.0, 135.0]);
    assert_eq!(text_buffer_from_box(b), IRect::new(11, 36, 257, 148));
    assert_eq!(
        text_effect_buffer(40.0, 60.0, 188.0, 72.0),
        IRect::new(11, 36, 257, 148)
    );
}

/// 15 of 16 rows exact; the 100 pt "Fx Ag" row (ambiguous noise solution, marked (*)) lies 1 px
/// to the right.
#[test]
fn measured_table_15_of_16() {
    let mut ok = 0;
    for (s, text, l, t, adv, want) in TABLE {
        let r = text_effect_buffer(l, t, adv, s);
        let got = [r.left, r.top, r.right, r.bottom];
        if got == want {
            ok += 1;
        } else {
            assert_eq!((s, text), (100.0, "Fx Ag"), "{got:?} instead of {want:?}");
            assert_eq!(got, [40, 47, 382, 203]);
        }
    }
    assert_eq!(ok, 15);
}

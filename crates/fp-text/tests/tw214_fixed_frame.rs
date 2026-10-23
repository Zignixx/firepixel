//! TW-214 (re 63 section 2.3, measured): horizontal fixed-width frames. The first baseline is
//! `top + round(d·s)` (d = top of the glyph "d"), every line sits on whole pixels
//! (`round(top + d·s + k·leading·s)`; the layout keeps the exact value, the hinted rasterizer
//! rounds it), and the frame ends `ceil` of the descent (`head.yMin`) below
//! the unrounded last baseline: one line Arial 12/15/30/60 pt -> 13/16/32/63, Myriad Pro 60 -> 58,
//! Times 60 -> 61; three lines Arial 12/30/60 -> 42/104/207. A frame of width 0 shows 10 px wide
//! and sets no glyph. Skipped when a font is missing.

mod common;

use common::*;
use fp_geom::Rect;
use fp_text::*;

fn fixed(font: &str, size: f64, text: &str, width: f32) -> Option<TextLayout> {
    if fonts()
        .resolve(font, false, false)
        .is_none_or(|r| r.substituted)
    {
        eprintln!("skipped: {font} not installed");
        return None;
    }
    let block = TextBlock::fixed(
        Rect::new(100.0, 100.0, 100.0 + width, 200.0),
        vec![TextRun::new(
            text,
            CharAttrs {
                font: font.into(),
                size,
                leading: 1.2,
                ..CharAttrs::default()
            },
        )],
    );
    Some(layout(&block, fonts(), &LayoutOptions::default()))
}

#[test]
fn first_line_height_and_baseline_as_measured() {
    for (font, size, height, baseline) in [
        ("ArialMT", 12.0, 13.0, 109.0),
        ("ArialMT", 15.0, 16.0, 111.0),
        ("ArialMT", 30.0, 32.0, 121.0),
        ("ArialMT", 60.0, 63.0, 143.0),
        ("MyriadPro-Regular", 60.0, 58.0, 143.0),
        ("TimesNewRomanPSMT", 60.0, 61.0, 142.0),
    ] {
        let Some(lay) = fixed(font, size, "Ag", 200.0) else {
            continue;
        };
        assert_eq!(
            lay.frame.bottom - lay.frame.top,
            height,
            "{font} {size} height"
        );
        assert_eq!(
            lay.lines[0].baseline.round(),
            baseline,
            "{font} {size} baseline"
        );
        assert!(
            lay.glyphs.iter().all(|g| g.y.round() == baseline),
            "{font} {size}"
        );
    }
}

#[test]
fn further_lines_add_the_leading_and_sit_on_whole_pixels() {
    for (size, height, baselines) in [
        (12.0, 42.0, [109.0, 123.0, 137.0]),
        (30.0, 104.0, [121.0, 157.0, 193.0]),
        (60.0, 207.0, [143.0, 215.0, 287.0]),
    ] {
        let Some(lay) = fixed("ArialMT", size, "Zeile 1\rZeile 2\rZeile 3", 200.0) else {
            return;
        };
        assert_eq!(
            lay.frame.bottom - lay.frame.top,
            height,
            "Arial {size} height"
        );
        let got: Vec<f64> = lay.lines.iter().map(|l| l.baseline.round()).collect();
        assert_eq!(got, baselines, "Arial {size} baselines");
    }
}

#[test]
fn zero_width_frame_shows_ten_pixels_without_glyphs() {
    let Some(lay) = fixed("ArialMT", 12.0, "Ag", 0.0) else {
        return;
    };
    assert_eq!((lay.frame.left, lay.frame.right), (100.0, 110.0));
    assert!(lay.glyphs.is_empty());
}

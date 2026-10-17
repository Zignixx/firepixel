//! G-B5 (B-2b BA-124): `pHYs` bei Einheit Pixel/cm wie im Original gemessen:
//! `trunc(px/cm) · 100` – 28,35 → 2800, 50 → 5000, 59,06 → 5900; in dpi weiter
//! `floor(trunc(dpi) / 0,0254)` (72 → 2834).

use std::sync::Arc;

use fp_export::doc_export::page_context;
use fp_export::png_out::{PngOptions, write_png_truecolor};

fn phys_x(bytes: &[u8]) -> u32 {
    let chunks = fp_fwpng::png::read_chunks(bytes).expect("PNG");
    let c = chunks.iter().find(|c| c.is(b"pHYs")).expect("pHYs");
    assert_eq!(c.data[8], 1, "Einheit Meter");
    u32::from_be_bytes([c.data[0], c.data[1], c.data[2], c.data[3]])
}

fn png_for(resolution: f32, unit: i32) -> Vec<u8> {
    let mut doc = fp_model::Document::new(4.0, 3.0, fp_model::Argb(0xffff_ffff));
    {
        let pg = Arc::make_mut(&mut doc.pages[0]);
        pg.canvas.resolution = resolution;
        pg.canvas.resolution_unit = unit;
    }
    let ctx = page_context(&doc, 0);
    write_png_truecolor(
        &[0xffff_0000; 12],
        4,
        3,
        true,
        &PngOptions::from_context(&ctx, false),
    )
    .expect("PNG")
}

#[test]
fn phys_per_cm_truncates_like_original() {
    for (r, want) in [(28.35f32, 2800u32), (50.0, 5000), (59.06, 5900)] {
        assert_eq!(phys_x(&png_for(r, 1)), want, "{r} px/cm");
        // FW-PNG (Speichern) nach derselben Regel
        let ch = fp_fwpng::png::make_phys(r, 1);
        assert_eq!(
            u32::from_be_bytes([ch.data[0], ch.data[1], ch.data[2], ch.data[3]]),
            want,
            "FW-PNG {r} px/cm"
        );
    }
    // dpi unverändert (BA-035)
    assert_eq!(phys_x(&png_for(72.0, 0)), 2834);
    assert_eq!(phys_x(&png_for(150.6, 0)), 5905);
}

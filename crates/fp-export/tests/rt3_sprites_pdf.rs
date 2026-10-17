//! RT-3 (re 28 E6): CSS-Sprites-Lagen und PDF-Qualitätstabellen wie im Original gemessen.

use fp_export::sprites::{SpriteLayout, sprite_layout};

/// Segmente in umgekehrter Anlegereihenfolge (oberstes Objekt der Web-Ebene zuerst):
/// r7_c9 160×180, r7_c3 150×60, r3_c8 40×20, r2_c5 100×80, r2_c2 60×40.
const SIZES: [(usize, usize); 5] = [(160, 180), (150, 60), (40, 20), (100, 80), (60, 40)];

#[test]
fn e6_grid_shelf_packing_p10_p11() {
    // M10 (re 43 §1): Vorgabe 10 px – 10 px Rand zum Blatt und 10 px zwischen zwei Kacheln (keine
    // Verdopplung): drei Kacheln 40 × 40 untereinander ergeben 60 × 160
    let (w, h, pos) = sprite_layout(
        &[(40, 40); 3],
        fp_export::sprites::SpriteOptions::default().padding,
        SpriteLayout::Vertical,
    );
    assert_eq!((w, h), (60, 160));
    assert_eq!(pos, vec![(10, 10), (10, 60), (10, 110)]);
    let (w, h, pos) = sprite_layout(&SIZES, 10, SpriteLayout::Grid);
    assert_eq!((w, h), (180, 410));
    // r7_c9 @10,10 · r2_c5 @10,200 · r7_c3 @10,290 · r2_c2 @10,360 · r3_c8 @120,200
    assert_eq!(
        pos,
        vec![(10, 10), (10, 290), (120, 200), (10, 200), (10, 360)]
    );
    let (w, h, pos) = sprite_layout(&SIZES, 11, SpriteLayout::Grid);
    assert_eq!((w, h), (182, 415));
    assert_eq!(
        pos,
        vec![(11, 11), (11, 293), (122, 202), (11, 202), (11, 364)]
    );
}

#[test]
fn e6_horizontal_vertical_p11() {
    let (w, h, pos) = sprite_layout(&SIZES, 11, SpriteLayout::Horizontal);
    assert_eq!((w, h), (576, 202));
    assert_eq!(
        pos.iter().map(|p| p.0).collect::<Vec<_>>(),
        vec![11, 182, 343, 394, 505]
    );
    assert!(pos.iter().all(|p| p.1 == 11));
    let (w, h, pos) = sprite_layout(&SIZES, 11, SpriteLayout::Vertical);
    assert_eq!((w, h), (182, 446));
    assert_eq!(
        pos.iter().map(|p| p.1).collect::<Vec<_>>(),
        vec![11, 202, 273, 304, 395]
    );
    assert!(pos.iter().all(|p| p.0 == 11));
}

/// PDF: ein JPEG je Seite mit den festen Tabellen der Stufe (erste Zeile Luminanz/Chrominanz).
#[test]
fn e6_pdf_quality_tables() {
    let t = &fp_export::pdf_jpeg_tables::PDF_JPEG_TABLES;
    assert!(
        t[0].0.iter().all(|&v| v == 1) && t[0].1.iter().all(|&v| v == 1),
        "Hoch: alle 1"
    );
    assert_eq!(&t[1].0[..8], &[4, 3, 3, 4, 5, 7, 8, 9]);
    assert_eq!(&t[1].1[..8], &[4, 3, 4, 5, 6, 7, 9, 9]);
    assert_eq!(&t[2].0[..8], &[9, 6, 6, 9, 12, 12, 17, 15]);
    assert_eq!(&t[3].0[..8], &[13, 9, 9, 13, 13, 16, 23, 22]);
    assert_eq!(&t[4].0[..8], &[20, 17, 26, 34, 34, 41, 52, 63]);
    assert_eq!(&t[4].1[..8], &[21, 21, 20, 27, 36, 44, 59, 71]);
    // Schreiber: die Tabelle der Stufe landet 1:1 im JPEG der Seite
    let img = fp_raster::RgbaImage::filled(
        fp_geom::IRect::new(0, 0, 32, 16),
        fp_color::Rgba8::new(200, 40, 40, 255),
    )
    .expect("Bild");
    let page = fp_export::vector_out::PdfPage {
        image: &img,
        width_pt: 32.0,
        height_pt: 16.0,
    };
    for level in 0..5u8 {
        let opt = fp_export::vector_out::PdfOptions {
            compression: fp_export::vector_out::PdfCompression::Jpeg,
            quality_level: Some(level),
            ..Default::default()
        };
        let pdf = fp_export::vector_out::write_pdf_with(std::slice::from_ref(&page), "t", &opt)
            .expect("PDF");
        let soi = pdf
            .windows(2)
            .position(|w| w == [0xFF, 0xD8])
            .expect("ein JPEG");
        assert_eq!(
            pdf.windows(2).filter(|w| *w == [0xFF, 0xD8]).count(),
            1,
            "ein JPEG je Seite"
        );
        let dqt = fp_export::jpeg::read_dqt(&pdf[soi..]);
        let zz = fp_export::jpeg_tables::ZIGZAG;
        let lum: Vec<u16> = (0..64).map(|k| t[level as usize].0[zz[k]]).collect();
        assert_eq!(dqt[0].1, lum, "Stufe {level}");
    }
}

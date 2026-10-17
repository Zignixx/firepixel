//! FX-T: Text-Rasterung wie das Original (Golden gegen die Messreihe `re\23-effekte.md`).
//!
//! Referenz `tests\data\fxt\fx_text_original.png`: gespeichertes Textbild der Basisdatei
//! `re\_testdateien\par\fx\base\fx_text.fw.png` (im Original per JSF angelegt; Leinwand 240 × 180,
//! „Fx Ag“, Arial-BoldMT 40 px, `#cc3300`, Rahmentext ab 40,60, Glättung „glatt“), ohne
//! Metadaten neu gespeichert. Ohne Arial Bold werden die Tests übersprungen.

mod common;

use common::*;
use fp_color::Rgba8;
use fp_geom::{IRect, Rect};
use fp_text::*;

fn need_arial_bold() -> bool {
    if fonts().find_ps("Arial-BoldMT").is_none() {
        eprintln!("übersprungen: Arial-BoldMT nicht installiert");
        return false;
    }
    true
}

/// Der Text der Basisdatei (Werte wie `TXTv` der Datei).
fn fx_block() -> TextBlock {
    let attrs = CharAttrs {
        font: "Arial-BoldMT".into(),
        size: 40.0,
        color: Rgba8::new(0xcc, 0x33, 0x00, 255),
        leading: 1.2,
        ..CharAttrs::default()
    };
    TextBlock {
        kind: FrameKind::FixedWidth,
        bounds: Rect::new(40.0, 60.0, 200.0, 120.0),
        runs: vec![TextRun::new("Fx Ag", attrs)],
        auto_kern: false,
        story_kerning: None,
        orientation: Orientation::HorizontalLtr,
        antialias: AntiAlias::default(),
        path: None,
    }
}

fn reference() -> Vec<[u8; 4]> {
    let p = data_dir().join("fxt").join("fx_text_original.png");
    let dec = png::Decoder::new(std::io::BufReader::new(
        std::fs::File::open(&p).expect("Referenz"),
    ));
    let mut r = dec.read_info().expect("png");
    let mut buf = vec![0; r.output_buffer_size().expect("Größe")];
    let info = r.next_frame(&mut buf).expect("Bild");
    assert_eq!(
        (info.width, info.height, info.color_type),
        (240, 180, png::ColorType::Rgba)
    );
    buf[..info.buffer_size()].as_chunks::<4>().0.to_vec()
}

/// (max. Abweichung Alpha, max. Abweichung Farbe vormultipliziert, Pixel mit Abweichung > 3)
fn compare(lay: &TextLayout) -> (u8, u8, usize) {
    let img = render::render(lay, None);
    let refi = reference();
    let (mut am, mut cm, mut n) = (0u8, 0u8, 0usize);
    for y in 0..180 {
        for x in 0..240 {
            let a = img.pixel(x, y);
            let b = refi[(y * 240 + x) as usize];
            let pm = |c: u8, al: u8| ((c as u32 * al as u32 + 127) / 255) as u8;
            let da = a.a.abs_diff(b[3]);
            let dc = pm(a.r, a.a)
                .abs_diff(pm(b[0], b[3]))
                .max(pm(a.g, a.a).abs_diff(pm(b[1], b[3])))
                .max(pm(a.b, a.a).abs_diff(pm(b[2], b[3])));
            am = am.max(da);
            cm = cm.max(dc);
            n += (da.max(dc) > 3) as usize;
        }
    }
    (am, cm, n)
}

#[test]
fn fx_text_like_original() {
    if !need_arial_bold() {
        return;
    }
    let lay = layout(&fx_block(), fonts(), &LayoutOptions::default());
    let (am, cm, n) = compare(&lay);
    println!("gehintet: Alpha max {am}, Farbe max {cm}, {n} Pixel > 3");
    // vorher (ungehintet, Grundlinie 88,63): Alpha max 159, 431 Pixel > 3
    assert!(
        am <= 16 && cm <= 16 && n <= 10,
        "Alpha max {am}, Farbe max {cm}, {n} Pixel > 3"
    );
}

#[test]
fn without_hinting_the_old_deviation_remains() {
    if !need_arial_bold() {
        return;
    }
    let lay = layout(
        &fx_block(),
        fonts(),
        &LayoutOptions {
            hinting: false,
            ..LayoutOptions::default()
        },
    );
    let (am, _, n) = compare(&lay);
    assert!(am > 100 && n > 300, "ungehintet Alpha max {am}, {n} Pixel");
}

#[test]
fn object_image_has_one_pixel_border() {
    if !need_arial_bold() {
        return;
    }
    let lay = layout(&fx_block(), fonts(), &LayoutOptions::default());
    let img = render::render(&lay, None);
    // pixelRect des Originals 43,60–148,98, Objektbild mit 1 px Rand ringsum
    assert_eq!(img.bounds(), IRect::new(42, 59, 149, 99));
    assert_eq!(img.alpha_bounds(), IRect::new(43, 60, 148, 98));
    assert_eq!(render::coverage(&lay, None).bounds(), img.bounds());
    // explizites Rechteck bleibt genau dieses Rechteck
    let r = IRect::new(0, 0, 240, 180);
    assert_eq!(render::render(&lay, Some(r)).bounds(), r);
}

#[test]
fn rerender_is_deterministic() {
    if !need_arial_bold() {
        return;
    }
    // §4b: 50× neu setzen und rastern ergibt dasselbe Bild (Zwischenspeicher der gehinteten Umrisse)
    let first = render::render(
        &layout(&fx_block(), fonts(), &LayoutOptions::default()),
        None,
    );
    for _ in 0..50 {
        let again = render::render(
            &layout(&fx_block(), fonts(), &LayoutOptions::default()),
            None,
        );
        assert_eq!(again.bounds(), first.bounds());
        assert!(again.data() == first.data());
    }
}

#[test]
fn hinting_only_for_axis_aligned_glyphs() {
    if !need_arial_bold() {
        return;
    }
    // Gedrehter Text (Abbildung mit Drehung) wird ungehintet gerastert und bleibt vollständig
    let mut lay = layout(&fx_block(), fonts(), &LayoutOptions::default());
    let rot = fp_text::raster::Affine {
        a: 0.0,
        b: 1.0,
        c: -1.0,
        d: 0.0,
        e: 200.0,
        f: 0.0,
    };
    for g in &mut lay.glyphs {
        g.transform = g.transform.then(&rot);
    }
    let img = render::render(&lay, None);
    let ab = img.alpha_bounds();
    assert!(ab.width() > 30 && ab.height() > 90, "{ab:?}");
    assert_eq!(img.bounds(), ab.inflate(1, 1));
}

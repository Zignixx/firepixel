//! Gemeinsame Hilfen der Integrationstests: PNG lesen, Vergleich.
#![allow(dead_code)]

use std::path::Path;

use fp_raster::{IRect, RgbaImage};

/// PNG (beliebiger Farbtyp, 8 Bit) als RGBA lesen; Lage (`x`, `y`).
pub fn load_png(path: &Path, x: i32, y: i32) -> RgbaImage {
    let file = std::fs::File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut dec = png::Decoder::new(std::io::BufReader::new(file));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = dec.read_info().expect("PNG-Kopf");
    let mut buf = vec![0; reader.output_buffer_size().expect("Puffergröße")];
    let info = reader.next_frame(&mut buf).expect("PNG-Daten");
    let (w, h) = (info.width as usize, info.height as usize);
    let bytes = &buf[..info.buffer_size()];
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => bytes.to_vec(),
        png::ColorType::Rgb => bytes.as_chunks::<3>().0.iter().flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => bytes.as_chunks::<2>().0.iter().flat_map(|c| [c[0], c[0], c[0], c[1]]).collect(),
        png::ColorType::Grayscale => bytes.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => panic!("Palette nach EXPAND unerwartet"),
    };
    RgbaImage::from_rgba_bytes(IRect::from_xywh(x, y, w as i32, h as i32), &rgba).expect("Größe")
}

/// Strenger Vergleich (alle vier Kanäle, auch Farbe transparenter Pixel): (gleich, gesamt, max).
pub fn strict_diff(a: &RgbaImage, b: &RgbaImage) -> (usize, usize, u8) {
    let r = a.bounds().union(&b.bounds());
    let mut eq = 0;
    let mut tot = 0;
    let mut mx = 0u8;
    for y in r.top..r.bottom {
        for x in r.left..r.right {
            let p = a.get(x, y).unwrap_or([0; 4]);
            let q = b.get(x, y).unwrap_or([0; 4]);
            tot += 1;
            let d = (0..4).map(|i| p[i].abs_diff(q[i])).max().unwrap_or(0);
            if d == 0 {
                eq += 1;
            }
            mx = mx.max(d);
        }
    }
    (eq, tot, mx)
}

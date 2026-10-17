//! T-2: BMP mit `BI_RLE8` (Bittiefe-Modus 1 des BMP-Plug-ins, re\27 E5): Kopf, Kodierung und
//! Rundreise schreiben (`fp-export`) → lesen (`fp-io`).

use fp_export::export::{EncodeExtras, export_image};
use fp_export::formats::{RasterData, write_bmp, write_bmp_ex};
use fp_export::options::{ColorMode, ExportFormat, OptimizeSettings, PaletteKind};
use fp_export::pipeline::{ExportContext, IndexedImage};
use fp_geom::IRect;
use fp_raster::RgbaImage;

fn u32at(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(d[o..o + 4].try_into().unwrap())
}

/// Indexbild mit langen Läufen, Einzelpixeln, Zweierfolgen, ungeraden Literalfolgen und Zeilen
/// länger als 255 Pixel.
fn mixed(w: usize, h: usize) -> IndexedImage {
    let palette: Vec<u32> = (0..7u32)
        .map(|i| 0xFF00_0000 | (i * 36) << 16 | (200 - i * 20) << 8 | (i * 11))
        .collect();
    let mut indices = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            let v = match y % 4 {
                0 => 3,                       // ein Lauf über die ganze Zeile (> 255 ⇒ geteilt)
                1 => ((x * 5 + y) % 7) as u8, // fast nur Literale
                2 => ((x / 3) % 7) as u8,     // Dreierläufe
                _ => {
                    if x % 9 < 2 {
                        1
                    } else {
                        ((x + y) % 7) as u8
                    }
                } // gemischt
            };
            indices.push(v);
        }
    }
    let mut counts = vec![0u64; palette.len()];
    for &i in &indices {
        counts[i as usize] += 1;
    }
    IndexedImage {
        width: w,
        height: h,
        palette,
        indices,
        transparent_index: None,
        counts,
        alpha_palette: false,
    }
}

fn assert_same(img: &IndexedImage, got: &RgbaImage, what: &str) {
    assert_eq!(
        (got.width(), got.height()),
        (img.width, img.height),
        "{what}: Größe"
    );
    for (i, (&ix, g)) in img.indices.iter().zip(got.data()).enumerate() {
        let c = img.palette[ix as usize];
        let e = [(c >> 16) as u8, (c >> 8) as u8, c as u8, 255];
        assert_eq!(
            e,
            *g,
            "{what}: Pixel ({}, {})",
            i % img.width,
            i / img.width
        );
    }
}

#[test]
fn bmp_rle8_header_and_roundtrip() {
    for (w, h) in [(1, 1), (2, 3), (5, 4), (37, 23), (300, 9)] {
        let img = mixed(w, h);
        let data = write_bmp_ex(&RasterData::Indexed(&img), 150.0, true).unwrap();
        // Kopf wie unkomprimiert, aber `biCompression` = 1 und `biSizeImage` = Datenlänge
        assert_eq!(&data[0..2], b"BM");
        assert_eq!(u32at(&data, 2) as usize, data.len(), "bfSize");
        assert_eq!(u32at(&data, 14), 40, "BITMAPINFOHEADER");
        assert_eq!(u16::from_le_bytes([data[28], data[29]]), 8, "8 Bit");
        assert_eq!(u32at(&data, 30), 1, "BI_RLE8");
        let off = u32at(&data, 10) as usize;
        assert_eq!(off, 14 + 40 + 1024);
        assert_eq!(u32at(&data, 34) as usize, data.len() - off, "biSizeImage");
        assert_eq!(u32at(&data, 38), 5904, "150 dpi");
        assert_eq!(
            (u32at(&data, 46), u32at(&data, 50)),
            (256, 7),
            "biClrUsed/biClrImportant"
        );
        // endet mit „Ende der Bitmap“, jede absolute Folge auf gerade Länge aufgefüllt
        assert_eq!(&data[data.len() - 2..], &[0, 1]);
        assert_eq!((data.len() - off) % 2, 0);
        let dec = fp_io::bmp::decode(&data).unwrap();
        assert_same(&img, &dec.image, &format!("RLE8 {w}×{h}"));
        assert_eq!(dec.dpi, Some(150.0));
    }
    // Läufe machen die Datei kleiner als unkomprimiert
    let img = mixed(300, 40);
    let rle = write_bmp_ex(&RasterData::Indexed(&img), 72.0, true).unwrap();
    let raw = write_bmp(&RasterData::Indexed(&img), 72.0).unwrap();
    assert!(rle.len() < raw.len(), "{} < {}", rle.len(), raw.len());
    // Truecolor bleibt unkomprimiert (RLE8 gibt es nur bei 8 Bit)
    let px = vec![0xFF12_3456u32; 6];
    let tc = write_bmp_ex(
        &RasterData::Truecolor {
            width: 3,
            height: 2,
            pixels: &px,
            alpha: false,
        },
        72.0,
        true,
    )
    .unwrap();
    assert_eq!(u32at(&tc, 30), 0);
}

#[test]
fn bmp_rle8_through_export_settings() {
    // Einstellung im Modell (Skript-/Exporteinstellung) → OptimizeSettings → Export
    let mut e = fp_model::ExportSettings {
        format: 3,
        color_mode: 0,
        ..Default::default()
    };
    e.bmp_rle8 = true;
    let mut s = OptimizeSettings::from_model(&e, None);
    s.format = ExportFormat::Bmp;
    s.color_mode = ColorMode::Indexed;
    s.palette_kind = PaletteKind::Adaptive;
    assert!(s.bmp_rle8);
    assert!(
        s.to_model(&fp_model::ExportSettings::default()).bmp_rle8,
        "Rundreise ins Modell"
    );
    let (w, h) = (40usize, 10usize);
    let mut bytes = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let c: [u8; 4] = if x < 20 {
                [255, 0, 0, 255]
            } else if y % 2 == 0 {
                [0, 0, 255, 255]
            } else {
                [0, 128, 0, 255]
            };
            bytes.extend_from_slice(&c);
        }
    }
    let img = RgbaImage::from_rgba_bytes(IRect::new(0, 0, w as i32, h as i32), &bytes).unwrap();
    let out = export_image(
        &img,
        &s,
        &ExportContext::default(),
        &EncodeExtras::standalone(),
    )
    .unwrap()
    .bytes;
    assert_eq!(u32at(&out, 30), 1, "BI_RLE8");
    let dec = fp_io::bmp::decode(&out).unwrap();
    assert_eq!(
        dec.image
            .data()
            .iter()
            .flatten()
            .copied()
            .collect::<Vec<u8>>(),
        bytes
    );
    // ohne die Einstellung unkomprimiert
    s.bmp_rle8 = false;
    let out = export_image(
        &img,
        &s,
        &ExportContext::default(),
        &EncodeExtras::standalone(),
    )
    .unwrap()
    .bytes;
    assert_eq!(u32at(&out, 30), 0);
}

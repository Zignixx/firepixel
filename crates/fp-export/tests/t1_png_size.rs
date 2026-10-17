//! T-1 TW-58: PNG-Größe gegen das Original. Befund (Bild der Bildvorschau, `par_export.fw.png`, vom
//! Original geschrieben): die Zeilenfilter von Firepixel sind Zeile für Zeile dieselben wie die des
//! Originals (Sub 3, Up 285, Paeth 12); der Unterschied (IDAT 56 125 gegen 55 403 Byte, +1,3 %) liegt
//! allein am Deflate-Kodierer (miniz Stufe 6 gegen zlib 1.2 Stufe 6). PNG8: Palettenbilder ohne Filter
//! wie libpng (adaptiv wären es 31 % mehr).

use fp_export::png_out::{PngOptions, write_png_truecolor};

/// IDAT-Daten (verkettet) und Filterbyte je Zeile einer PNG-Datei.
fn idat_and_filters(png: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let mut pos = 8;
    let mut idat = Vec::new();
    let (mut w, mut h, mut bpp) = (0usize, 0usize, 4usize);
    while pos + 8 <= png.len() {
        let n = u32::from_be_bytes([png[pos], png[pos + 1], png[pos + 2], png[pos + 3]]) as usize;
        let kind = &png[pos + 4..pos + 8];
        let body = &png[pos + 8..pos + 8 + n];
        if kind == b"IHDR" {
            w = u32::from_be_bytes([body[0], body[1], body[2], body[3]]) as usize;
            h = u32::from_be_bytes([body[4], body[5], body[6], body[7]]) as usize;
            bpp = match body[9] {
                6 => 4,
                2 => 3,
                _ => 1,
            };
        }
        if kind == b"IDAT" {
            idat.extend_from_slice(body);
        }
        pos += 12 + n;
    }
    let raw = miniz_inflate(&idat);
    let stride = w * bpp + 1;
    let filters = (0..h)
        .filter_map(|y| raw.get(y * stride).copied())
        .collect();
    (idat, filters)
}

fn miniz_inflate(z: &[u8]) -> Vec<u8> {
    use std::io::Read;
    let mut out = Vec::new();
    flate2::read::ZlibDecoder::new(z)
        .read_to_end(&mut out)
        .expect("zlib");
    out
}

/// Glatte Fläche mit Verlauf: wie libpng nur Sub/Up/Paeth, keine Zeile „None“ oder „Average“; der
/// zlib-Kopf ist der von zlib Stufe 6 (`78 9c`, bei kleinen Daten kleineres Fenster `58/68 …`).
#[test]
fn tw58_filters_like_libpng() {
    let (w, h) = (64usize, 48usize);
    let px: Vec<u32> = (0..w * h)
        .map(|i| {
            let (x, y) = ((i % w) as u32, (i / w) as u32);
            0xff00_0000 | ((x * 4) << 16) | ((y * 5) << 8) | ((x + y) & 0xff)
        })
        .collect();
    let png = write_png_truecolor(&px, w, h, true, &PngOptions::default()).expect("PNG");
    let (idat, filters) = idat_and_filters(&png);
    assert!(
        filters.iter().all(|f| matches!(f, 1 | 2 | 4)),
        "nur Sub/Up/Paeth: {filters:?}"
    );
    assert_eq!(idat[0] & 0x0f, 8, "Deflate");
    assert_eq!((idat[1] >> 6) & 3, 2, "Stufe „default“ (6)");
    assert_eq!(
        ((idat[0] as u16) << 8 | idat[1] as u16) % 31,
        0,
        "gültiger zlib-Kopf"
    );
}

/// Größe am Bild der Bildvorschau (Kompositbild von `par_export.fw.png`, vom Original geschrieben):
/// IDAT vorher/nachher gegen das Original. Nur mit den Testdateien (ignoriert).
#[test]
#[ignore]
fn tw58_par_sizes() {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../re/_testdateien/par/par_export.fw.png");
    let Ok(data) = std::fs::read(&p) else { return };
    let (orig_idat, orig_f) = idat_and_filters(&data);
    let dec = png::Decoder::new(std::io::Cursor::new(&data));
    let mut r = dec.read_info().expect("PNG");
    let mut buf = vec![0u8; r.output_buffer_size().expect("Größe")];
    let info = r.next_frame(&mut buf).expect("Bild");
    let (w, h) = (info.width as usize, info.height as usize);
    let px: Vec<u32> = buf[..w * h * 4]
        .chunks(4)
        .map(|c| (c[3] as u32) << 24 | (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32)
        .collect();
    let png32 = write_png_truecolor(&px, w, h, true, &PngOptions::default()).expect("PNG32");
    let (fp_idat, fp_f) = idat_and_filters(&png32);
    let count = |f: &[u8]| {
        (0..5u8)
            .map(|k| f.iter().filter(|x| **x == k).count())
            .collect::<Vec<_>>()
    };
    println!(
        "TW58 PNG32 IDAT Original {} Firepixel {} | Filter Original {:?} Firepixel {:?}",
        orig_idat.len(),
        fp_idat.len(),
        count(&orig_f),
        count(&fp_f)
    );
    let opaque: Vec<u32> = px.iter().map(|p| p | 0xff00_0000).collect();
    let png24 = write_png_truecolor(&opaque, w, h, false, &PngOptions::default()).expect("PNG24");
    println!(
        "TW58 PNG24 IDAT Firepixel {}",
        idat_and_filters(&png24).0.len()
    );
    let s8 = fp_export::options::builtin_presets()
        .into_iter()
        .find(|p| p.name == "PNG8")
        .expect("PNG8")
        .settings;
    let img = fp_raster::RgbaImage::from_rgba_bytes(
        fp_raster::IRect::new(0, 0, w as i32, h as i32),
        &buf[..w * h * 4],
    )
    .expect("Bild");
    let ctx = fp_export::pipeline::ExportContext::default();
    let e8 = fp_export::export::export_image(
        &img,
        &s8,
        &ctx,
        &fp_export::export::EncodeExtras::standalone(),
    )
    .expect("PNG8");
    let (i8, f8) = idat_and_filters(&e8.bytes);
    println!(
        "TW58 PNG8 Datei {} IDAT {} Filter {:?}",
        e8.bytes.len(),
        i8.len(),
        count(&f8)
    );
}

/// Messhilfe: miniz-Stufen auf den gefilterten Daten des Originals (ignoriert).
#[test]
#[ignore]
fn tw58_levels() {
    use std::io::Write;
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../re/_testdateien/par/par_export.fw.png");
    let Ok(data) = std::fs::read(&p) else { return };
    let (idat, _) = idat_and_filters(&data);
    let raw = miniz_inflate(&idat);
    for lvl in 1..=10u32 {
        let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(lvl));
        e.write_all(&raw).expect("w");
        println!("TW58 miniz Stufe {lvl}: {}", e.finish().expect("z").len());
    }
}

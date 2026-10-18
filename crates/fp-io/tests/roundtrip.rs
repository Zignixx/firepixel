//! Rundreisen: mit `fp-export` schreiben, mit `fp-io` lesen (verlustfreie Formate pixelgenau,
//! verlustbehaftete mit Fehlerschranke). Läuft ohne Installation und Testdateien.

use fp_export::animation::{AnimFrameSrc, animated_webp, apng};
use fp_export::formats::{
    IcoImage, RasterData, TiffCompression, write_bmp, write_ico, write_qoi, write_tiff, write_wbmp,
};
use fp_export::gif::{GifFrame, GifOptions, write_gif};
use fp_export::jpeg::{JpegParams, encode_jpeg};
use fp_export::jpeg_tables::FwJpegTables;
use fp_export::modern::{write_avif, write_webp};
use fp_export::pipeline::{ExportContext, IndexedImage};
use fp_export::png_out::{PngOptions, write_png_truecolor, write_png8};
use fp_export::psd_out::{PsdLayer, write_psd};
use fp_export::render::{PageRenderer, RenderRequest, SimpleRenderer};
use fp_geom::IRect;
use fp_io::ImportOptions;
use fp_io::build::ImportedContent;
use fp_io::detect::{FileFormat, detect_format};
use fp_model::FrameInfo;
use fp_raster::RgbaImage;

const W: usize = 37;
const H: usize = 23;

/// Testbild `0xAARRGGBB`: Verläufe, halbtransparenter Rand links, voll transparenter Streifen.
fn test_pixels(alpha: bool) -> Vec<u32> {
    let mut v = Vec::with_capacity(W * H);
    for y in 0..H {
        for x in 0..W {
            let (r, g, b) = (
                (x * 7) as u32 & 0xFF,
                (y * 11) as u32 & 0xFF,
                ((x * y) % 256) as u32,
            );
            let a = if !alpha {
                255
            } else if y == 5 {
                0
            } else if x < 8 {
                (x as u32 * 32 + 10).min(255)
            } else {
                255
            };
            v.push(a << 24 | r << 16 | g << 8 | b);
        }
    }
    v
}

fn to_rgba(p: u32) -> [u8; 4] {
    [(p >> 16) as u8, (p >> 8) as u8, p as u8, (p >> 24) as u8]
}

/// Gleich, wobei voll transparente Pixel nur über Alpha verglichen werden.
fn same(a: [u8; 4], b: [u8; 4]) -> bool {
    (a[3] == 0 && b[3] == 0) || a == b
}

fn assert_exact(expected: &[u32], got: &RgbaImage, what: &str) {
    assert_eq!((got.width(), got.height()), (W, H), "{what}: Größe");
    let diffs: Vec<String> = expected
        .iter()
        .zip(got.data())
        .enumerate()
        .filter(|(_, (e, g))| !same(to_rgba(**e), **g))
        .map(|(i, (e, g))| format!("({},{}) {:?}≠{:?}", i % W, i / W, to_rgba(*e), g))
        .collect();
    assert!(
        diffs.is_empty(),
        "{what}: {} abweichende Pixel, z. B. {:?}",
        diffs.len(),
        &diffs[..diffs.len().min(4)]
    );
}

fn mean_error(expected: &[u32], got: &RgbaImage) -> f64 {
    let sum: u64 = expected
        .iter()
        .zip(got.data())
        .map(|(e, g)| {
            (0..3)
                .map(|k| u64::from(to_rgba(*e)[k].abs_diff(g[k])))
                .sum::<u64>()
        })
        .sum();
    sum as f64 / (expected.len() * 3) as f64
}

/// Indexbild mit 16 Farben (4×4-Kacheln) und optional Transparenz an Index 3.
fn test_indexed() -> IndexedImage {
    let palette: Vec<u32> = (0..16u32)
        .map(|i| 0xFF00_0000 | (i * 16) << 16 | (255 - i * 16) << 8 | ((i * 37) & 0xFF))
        .collect();
    let indices: Vec<u8> = (0..W * H)
        .map(|i| (((i % W) / 4 + (i / W) / 4) % 16) as u8)
        .collect();
    let mut counts = vec![0u64; 16];
    for &i in &indices {
        counts[i as usize] += 1;
    }
    IndexedImage {
        width: W,
        height: H,
        palette,
        indices,
        transparent_index: None,
        counts,
        alpha_palette: false,
    }
}

fn frames3() -> Vec<AnimFrameSrc> {
    let base = test_pixels(true);
    (0..3)
        .map(|k| {
            let mut p = base.clone();
            for y in 2..10 {
                for x in (3 + k * 9)..(10 + k * 9) {
                    p[y * W + x] = 0xFF00_00FF | (k as u32 * 100) << 8;
                }
            }
            AnimFrameSrc {
                pixels: p,
                info: FrameInfo {
                    delay: 5 + k as i32 * 10,
                    ..FrameInfo::default()
                },
            }
        })
        .collect()
}

#[test]
fn png_truecolor_and_png8() {
    let px = test_pixels(true);
    for interlaced in [false, true] {
        let data = write_png_truecolor(
            &px,
            W,
            H,
            true,
            &PngOptions {
                interlaced,
                ..PngOptions::default()
            },
        )
        .unwrap();
        assert_eq!(detect_format(&data, ""), FileFormat::Png);
        assert_exact(
            &px,
            &fp_io::png_in::decode_png(&data).unwrap().image,
            "PNG32",
        );
        let r = fp_io::open_bytes("a.png", &data, &ImportOptions::default()).unwrap();
        assert_eq!(r.content, ImportedContent::Bitmap);
    }
    let opaque = test_pixels(false);
    let data = write_png_truecolor(&opaque, W, H, false, &PngOptions::default()).unwrap();
    assert_exact(
        &opaque,
        &fp_io::png_in::decode_png(&data).unwrap().image,
        "PNG24",
    );

    let idx = test_indexed();
    let data = write_png8(&idx, 0xFFFF_FFFF, &PngOptions::default()).unwrap();
    let img = fp_io::png_in::decode_png(&data).unwrap();
    let expected: Vec<u32> = idx
        .indices
        .iter()
        .map(|&i| idx.palette[i as usize])
        .collect();
    assert_exact(&expected, &img.image, "PNG8");
    assert!(img.palette.is_some(), "PNG8: Palette bleibt erhalten");
}

#[test]
fn gif_single_and_animated() {
    let idx = test_indexed();
    let pal: Vec<[u8; 3]> = idx
        .palette
        .iter()
        .map(|&c| [(c >> 16) as u8, (c >> 8) as u8, c as u8])
        .collect();
    let frame = GifFrame {
        indices: idx.indices.clone(),
        w: W as u16,
        h: H as u16,
        transparent: Some(3),
        ..GifFrame::default()
    };
    for interlaced in [false, true] {
        let data = write_gif(
            W as u16,
            H as u16,
            &pal,
            std::slice::from_ref(&frame),
            &GifOptions {
                interlaced,
                ..GifOptions::default()
            },
        );
        let g = fp_io::gif::decode(&data).unwrap();
        assert_eq!(g.frames.len(), 1);
        assert_eq!(
            g.frames[0].indices, idx.indices,
            "GIF-Indizes (Zeilensprung {interlaced})"
        );
        assert_eq!(g.frames[0].transparent, Some(3));
        assert_eq!(&g.global_palette.unwrap()[..16], &pal[..]);
    }
    // Animation: drei Bilder mit Versatz, Verzögerung und Löschmethode
    let frames: Vec<GifFrame> = (0..3u16)
        .map(|k| GifFrame {
            indices: vec![k as u8 + 1; 25],
            x: k * 5,
            y: k * 3,
            w: 5,
            h: 5,
            delay: 10 + k,
            disposal: 1 + (k as u8 % 2),
            ..GifFrame::default()
        })
        .collect();
    let mut all = vec![frame.clone()];
    all[0].transparent = None;
    all.extend(frames);
    let data = write_gif(
        W as u16,
        H as u16,
        &pal,
        &all,
        &GifOptions {
            loop_count: Some(0),
            ..GifOptions::default()
        },
    );
    let g = fp_io::gif::decode(&data).unwrap();
    assert_eq!(g.loop_count, Some(0));
    assert_eq!(g.frames.len(), 4);
    for (k, f) in g.frames[1..].iter().enumerate() {
        let k = k as u16;
        assert_eq!(
            (f.x, f.y, f.w, f.h, f.delay, f.disposal),
            (k * 5, k * 3, 5, 5, 10 + k, 1 + (k as u8 % 2))
        );
    }
    let anim = g.into_animation();
    let composed = anim.composed_frames();
    assert_eq!(composed.len(), 4);
    // Bild 3 (k = 1) liegt bei (5, 3) mit Index 2
    assert_eq!(composed[2].pixel(6, 4).to_array(), to_rgba(idx.palette[2]));
    let r = fp_io::open_bytes("a.gif", &data, &ImportOptions::default()).unwrap();
    assert_eq!(r.content, ImportedContent::Animation { frames: 4 });
}

#[test]
fn apng_roundtrip() {
    let src = frames3();
    let data = apng(&src, W, H, &ExportContext::default(), 0).unwrap();
    assert_eq!(detect_format(&data, ""), FileFormat::Apng);
    let anim = fp_io::png_in::decode_apng(&data).unwrap();
    assert_eq!(anim.loop_count, 0);
    let composed = anim.composed_frames();
    assert_eq!(composed.len(), 3);
    for (k, (s, c)) in src.iter().zip(&composed).enumerate() {
        assert_exact(&s.pixels, c, &format!("APNG Bild {k}"));
        assert_eq!(anim.frames[k].delay_cs, s.info.delay as u32);
    }
}

#[test]
fn webp_roundtrips() {
    let px = test_pixels(true);
    let data = write_webp(&px, W, H, true, 100).unwrap();
    assert_eq!(detect_format(&data, ""), FileFormat::WebP);
    let a = fp_io::modern::decode_webp(&data).unwrap();
    assert_exact(&px, &a.composed_frames()[0], "WebP verlustfrei");

    let opaque = test_pixels(false);
    let data = write_webp(&opaque, W, H, false, 90).unwrap();
    let a = fp_io::modern::decode_webp(&data).unwrap();
    let e = mean_error(&opaque, &a.composed_frames()[0]);
    assert!(e < 6.0, "WebP verlustbehaftet: mittlerer Fehler {e}");

    let src = frames3();
    let data = animated_webp(&src, W, H, true, 100, 0).unwrap();
    let a = fp_io::modern::decode_webp(&data).unwrap();
    let composed = a.composed_frames();
    assert_eq!(composed.len(), 3);
    for (k, (s, c)) in src.iter().zip(&composed).enumerate() {
        assert_exact(&s.pixels, c, &format!("animiertes WebP Bild {k}"));
    }
    let r = fp_io::open_bytes("a.webp", &data, &ImportOptions::default()).unwrap();
    assert_eq!(r.content, ImportedContent::Animation { frames: 3 });
}

#[test]
fn avif_roundtrip() {
    // deckend, hohe Qualität
    let opaque = test_pixels(false);
    let data = write_avif(&opaque, W, H, 95, 10).unwrap();
    assert_eq!(detect_format(&data, ""), FileFormat::Avif);
    let r = fp_io::open_bytes("a.avif", &data, &ImportOptions::default()).unwrap();
    assert_eq!(r.content, ImportedContent::Bitmap);
    let img = SimpleRenderer
        .render(&r.document, &RenderRequest::default())
        .unwrap();
    assert_eq!((img.width(), img.height()), (W, H));
    let e = mean_error(&opaque, &img);
    assert!(e < 3.0, "AVIF deckend: mittlerer Fehler {e}");
    assert!(img.data().iter().all(|p| p[3] == 255));
    // mit Alpha (eigenes Alpha-Item, `auxl`)
    let px = test_pixels(true);
    let data = write_avif(&px, W, H, 95, 10).unwrap();
    let a = fp_io::avif_in::decode_avif(&data).unwrap();
    let got = &a.frames[0].image;
    let ea: u64 = px
        .iter()
        .zip(got.data())
        .map(|(e, g)| u64::from(to_rgba(*e)[3].abs_diff(g[3])))
        .sum();
    assert!(
        (ea as f64 / px.len() as f64) < 2.0,
        "AVIF Alpha: mittlerer Fehler {}",
        ea as f64 / px.len() as f64
    );
    assert_eq!(got.pixel(10, 5).a, 0, "transparente Zeile");
    // beschädigt ⇒ Fehler, keine Panik
    for cut in [40, data.len() / 2, data.len() - 3] {
        let _ = fp_io::open_bytes("a.avif", &data[..cut], &ImportOptions::default());
    }
    let mut bad = data.clone();
    let n = bad.len();
    for b in &mut bad[n / 2..] {
        *b ^= 0x5A;
    }
    let _ = fp_io::open_bytes("a.avif", &bad, &ImportOptions::default());
}

/// HEIC-Testdateien (`re\_testdateien\phase5\heic`, erzeugt mit dem freien HEVC-Kodierer still265
/// aus `*_src.png`): Größe, Orientierung und mittlerer Fehler gegen die Quelle.
#[test]
fn heic_testfiles() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../re/_testdateien/phase5/heic");
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return;
    };
    let mut n = 0;
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) != Some("heic") {
            continue;
        }
        let stem = p.file_stem().unwrap().to_string_lossy().to_string();
        let data = std::fs::read(&p).unwrap();
        assert_eq!(detect_format(&data, ""), FileFormat::Heif, "{stem}");
        let t = std::time::Instant::now();
        let r = fp_io::open_bytes(&p.to_string_lossy(), &data, &ImportOptions::default())
            .unwrap_or_else(|e| panic!("{stem}: {e}"));
        let ms = t.elapsed().as_millis();
        let got = SimpleRenderer
            .render(&r.document, &RenderRequest::default())
            .unwrap();
        let src =
            fp_io::png_in::decode_png(&std::fs::read(dir.join(format!("{stem}_src.png"))).unwrap())
                .unwrap()
                .image;
        let (sw, sh) = (src.width(), src.height());
        // erwartetes Bild nach der Orientierung
        let expect = |x: usize, y: usize| -> [u8; 4] {
            match stem.as_str() {
                "synth_rot90" => src.data()[(sh - 1 - x) * sw + y],
                // still265 schreibt EXIF „horizontal spiegeln“ als `imir` 0 = oben ↔ unten
                "synth_mirror" => src.data()[(sh - 1 - y) * sw + x],
                _ => src.data()[y * sw + x],
            }
        };
        let (ew, eh) = if stem == "synth_rot90" {
            (sh, sw)
        } else {
            (sw, sh)
        };
        assert_eq!((got.width(), got.height()), (ew, eh), "{stem}: Größe");
        let mut sum = 0u64;
        for y in 0..eh {
            for x in 0..ew {
                let (a, b) = (expect(x, y), got.data()[y * ew + x]);
                sum += (0..3).map(|k| u64::from(a[k].abs_diff(b[k]))).sum::<u64>();
            }
        }
        let err = sum as f64 / (ew * eh * 3) as f64;
        eprintln!("HEIC {stem}: {ew}×{eh}, mittlerer Fehler {err:.2}, {ms} ms");
        dump(&r.document, &format!("{stem}.heic"));
        assert!(err < 4.0, "{stem}: mittlerer Fehler {err}");
        n += 1;
    }
    eprintln!("{n} HEIC-Dateien geprüft");
}

/// JPEG-Import gegen das Original (`re\_testdateien\phase5\jpeg`: mit Pillow/libjpeg-turbo erzeugte
/// Dateien, PNG-32-Export des Originals nach dem Öffnen): eigener IJG-Dekoder und zune-jpeg.
#[test]
fn jpeg_against_original() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../re/_testdateien/phase5/jpeg");
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return;
    };
    let mut names: Vec<String> = rd
        .flatten()
        .filter_map(|e| {
            e.file_name()
                .to_str()
                .and_then(|n| n.strip_suffix(".jpg"))
                .map(str::to_string)
        })
        .collect();
    names.sort();
    let (mut tot, mut ok_ijg, mut ok_zune) = (0usize, 0usize, 0usize);
    for n in names {
        let Ok(r) = std::fs::read(dir.join(format!("{n}_ref.png"))) else {
            continue;
        };
        let reference = fp_io::png_in::decode_png(&r).unwrap().image;
        let data = std::fs::read(dir.join(format!("{n}.jpg"))).unwrap();
        let a = fp_io::jpeg_in::decode(&data).unwrap().image;
        let b = fp_io::jpeg_in::decode_zune(&data).unwrap().image;
        let cnt = |img: &RgbaImage| {
            reference
                .data()
                .iter()
                .zip(img.data())
                .filter(|(x, y)| x == y)
                .count()
        };
        let maxd = |img: &RgbaImage| {
            reference
                .data()
                .iter()
                .zip(img.data())
                .flat_map(|(x, y)| (0..3).map(move |k| x[k].abs_diff(y[k])))
                .max()
                .unwrap_or(0)
        };
        let (ca, cb) = (cnt(&a), cnt(&b));
        eprintln!(
            "JPEG {n}: IJG {ca}/{} (max Δ {}), zune {cb} (max Δ {})",
            reference.data().len(),
            maxd(&a),
            maxd(&b)
        );
        tot += reference.data().len();
        ok_ijg += ca;
        ok_zune += cb;
    }
    eprintln!("JPEG gesamt: IJG {ok_ijg}/{tot}, zune {ok_zune}/{tot}");
    assert!(ok_ijg >= ok_zune);
}

/// PDF: eigener PDF-Export (Seitenbild) wieder einlesen; Vektor-PDF (PyMuPDF, `re\_testdateien\phase5\pdf`)
/// als Pfade einlesen und gegen das Rasterbild von MuPDF vergleichen.
#[test]
fn pdf_import() {
    use fp_export::vector_out::{PdfPage, write_pdf};
    let px = test_pixels(false);
    let img = RgbaImage::from_raw(
        IRect::from_xywh(0, 0, W as i32, H as i32),
        px.iter().map(|&p| to_rgba(p)).collect(),
    )
    .unwrap();
    let pdf = write_pdf(
        &[PdfPage {
            image: &img,
            width_pt: W as f64,
            height_pt: H as f64,
            text: None,
        }],
        "Test",
    )
    .unwrap();
    assert_eq!(detect_format(&pdf, ""), FileFormat::Pdf);
    let r = fp_io::open_bytes("a.pdf", &pdf, &ImportOptions::default()).unwrap();
    let out = fp_render::Renderer::default()
        .render_page(&r.document, 0, 0)
        .image;
    assert_eq!((out.width(), out.height()), (W, H));
    let e = mean_error(&px, &out);
    assert!(e < 2.0, "PDF-Seitenbild: mittlerer Fehler {e}");
    // beschädigt ⇒ Fehler ohne Panik
    assert!(
        fp_io::open_bytes("x.pdf", b"%PDF-1.4\n1 0 obj <<", &ImportOptions::default()).is_err()
    );

    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../re/_testdateien/phase5/pdf");
    let Ok(data) = std::fs::read(dir.join("vektor.pdf")) else {
        return;
    };
    let r = fp_io::open_bytes("vektor.pdf", &data, &ImportOptions::default()).unwrap();
    assert_eq!(r.document.pages.len(), 2, "zwei Seiten");
    let mut paths = 0;
    fp_model::traverse::walk_root(
        &r.document.pages[0].root,
        fp_model::traverse::RootRef::Page(0),
        &mut |_, _, e| {
            if matches!(e.kind, fp_model::ElementKind::Path(_)) {
                paths += 1;
            }
        },
    );
    assert!(paths >= 4, "Pfade: {paths}");
    for p in 0..2 {
        let reference = fp_io::png_in::decode_png(
            &std::fs::read(dir.join(format!("vektor_s{p}_ref.png"))).unwrap(),
        )
        .unwrap()
        .image;
        let out = fp_render::Renderer::default()
            .render_page(&r.document, p, 0)
            .image;
        assert_eq!(
            (out.width(), out.height()),
            (reference.width(), reference.height()),
            "Seite {p}: Größe"
        );
        let close = reference
            .data()
            .iter()
            .zip(out.data())
            .filter(|(a, b)| (0..3).all(|k| a[k].abs_diff(b[k]) <= 24))
            .count();
        let share = close as f64 / reference.data().len() as f64;
        eprintln!(
            "PDF Seite {p}: {close}/{} Pixel ±24 wie MuPDF ({:.1} %)",
            reference.data().len(),
            share * 100.0
        );
        if let Some(dir) = std::env::var_os("FP_DUMP_DIR") {
            let px: Vec<u32> = out
                .data()
                .iter()
                .map(|p| {
                    u32::from(p[3]) << 24
                        | u32::from(p[0]) << 16
                        | u32::from(p[1]) << 8
                        | u32::from(p[2])
                })
                .collect();
            if let Ok(png) =
                write_png_truecolor(&px, out.width(), out.height(), true, &PngOptions::default())
            {
                let _ = std::fs::write(
                    std::path::Path::new(&dir).join(format!("vektor_s{p}.png")),
                    png,
                );
            }
        }
        assert!(share > 0.97, "Seite {p}: nur {share}");
    }
}

/// SVG-Export (fp-export) → SVG-Import: Pfade bleiben Pfade, gleiches Bild.
#[test]
fn svg_export_roundtrip() {
    use fp_model::{Document, Element, LayerSlot, PathAttrs};
    let mut doc = Document::new(200.0, 120.0, fp_color::Argb(0xffff_ffff));
    let r = Element::new_rectangle(
        &mut doc.ids,
        fp_geom::Rect::new(10.0, 10.0, 90.0, 70.0),
        0.0,
        PathAttrs::solid_fill(fp_color::Argb(0xff33_66cc)),
    );
    let r2 = Element::new_rectangle(
        &mut doc.ids,
        fp_geom::Rect::new(100.0, 30.0, 190.0, 110.0),
        0.3,
        PathAttrs::solid_fill(fp_color::Argb(0xffcc_3333)),
    );
    {
        let p = std::sync::Arc::make_mut(&mut doc.pages[0]);
        if let LayerSlot::Own(l) = &mut p.root.layers[0] {
            std::sync::Arc::make_mut(&mut std::sync::Arc::make_mut(l).cels[0]).elements =
                vec![std::sync::Arc::new(r2), std::sync::Arc::new(r)];
        }
    }
    let svg = fp_export::vector_out::write_svg(&doc, 0, 0, None).unwrap();
    assert_eq!(detect_format(svg.as_bytes(), ""), FileFormat::Svg);
    let back = fp_io::open_bytes("a.svg", svg.as_bytes(), &ImportOptions::default()).unwrap();
    let mut n = 0;
    fp_model::traverse::walk_root(
        &back.document.pages[0].root,
        fp_model::traverse::RootRef::Page(0),
        &mut |_, _, e| {
            if matches!(e.kind, fp_model::ElementKind::Path(_)) {
                n += 1;
            }
        },
    );
    assert!(n >= 2, "Pfade zurück: {n}");
    let a = fp_render::Renderer::default().render_page(&doc, 0, 0).image;
    let b = fp_render::Renderer::default()
        .render_page(&back.document, 0, 0)
        .image;
    assert_eq!((a.width(), a.height()), (b.width(), b.height()));
    let close = a
        .data()
        .iter()
        .zip(b.data())
        .filter(|(x, y)| (0..3).all(|k| x[k].abs_diff(y[k]) <= 8))
        .count();
    assert!(
        close as f64 / a.data().len() as f64 > 0.99,
        "SVG-Rundreise: {close}/{}",
        a.data().len()
    );
}

/// TGA (selbst gebaut: unkomprimiert 32 Bit oben-links, RLE 24 Bit unten-links) und animiertes GIF aus
/// dem Export-Pfad des Originals (`animated_gif`).
#[test]
fn tga_and_animated_gif() {
    // 3×2, 32 Bit BGRA, Ursprung oben links (Deskriptor 0x28)
    let mut t = vec![0u8, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 2, 0, 32, 0x28];
    let px: [[u8; 4]; 6] = [
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 128],
        [10, 20, 30, 255],
        [40, 50, 60, 0],
        [70, 80, 90, 255],
    ];
    for p in &px {
        t.extend_from_slice(&[p[2], p[1], p[0], p[3]]);
    }
    let img = fp_io::tga::decode(&t).unwrap().image;
    for (i, p) in px.iter().enumerate() {
        assert_eq!(
            img.pixel((i % 3) as i32, (i / 3) as i32).to_array(),
            *p,
            "TGA32 Pixel {i}"
        );
    }
    // 4×2, RLE 24 Bit, Ursprung unten links: Zeile unten = 4× Rot (Lauf), Zeile oben = 2 Roh + Lauf
    let mut t = vec![0u8, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 2, 0, 24, 0x00];
    t.extend_from_slice(&[0x83, 0, 0, 255]);
    t.extend_from_slice(&[0x01, 255, 0, 0, 0, 255, 0, 0x81, 1, 2, 3]);
    let img = fp_io::tga::decode(&t).unwrap().image;
    assert_eq!(
        img.pixel(0, 1).to_array(),
        [255, 0, 0, 255],
        "unten links rot"
    );
    assert_eq!(img.pixel(0, 0).to_array(), [0, 0, 255, 255]);
    assert_eq!(img.pixel(1, 0).to_array(), [0, 255, 0, 255]);
    assert_eq!(img.pixel(3, 0).to_array(), [3, 2, 1, 255]);

    let src = frames3();
    let s = fp_export::options::OptimizeSettings {
        format: fp_export::options::ExportFormat::AnimatedGif,
        ..fp_export::options::OptimizeSettings::default()
    };
    let data =
        fp_export::animation::animated_gif(&src, W, H, &s, &ExportContext::default()).unwrap();
    assert_eq!(detect_format(&data, ""), FileFormat::Gif);
    let a = fp_io::gif::decode(&data).unwrap().into_animation();
    let composed = a.composed_frames();
    assert_eq!(composed.len(), 3);
    for (k, (s, c)) in src.iter().zip(&composed).enumerate() {
        // nur deckende Pixel vergleichen (GIF kennt keine Teiltransparenz)
        let (mut sum, mut cnt) = (0u64, 0u64);
        for (p, q) in s.pixels.iter().zip(c.data()) {
            if p >> 24 == 255 {
                sum += (0..3)
                    .map(|k| u64::from(to_rgba(*p)[k].abs_diff(q[k])))
                    .sum::<u64>();
                cnt += 3;
            }
        }
        let e = sum as f64 / cnt.max(1) as f64;
        assert!(e < 12.0, "animiertes GIF Bild {k}: mittlerer Fehler {e}");
        assert_eq!(a.frames[k].delay_cs, s.info.delay as u32, "Verzögerung {k}");
    }
}

/// SVG-Verläufe: linear, radial (kreisrund) und elliptisch (verzerrt) als Verlaufsfüllungen,
/// gerendert mit fp-render: Farben an Anfang/Mitte/Ende wie erwartet.
#[test]
fn svg_gradients() {
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="300" height="100">
      <defs>
        <linearGradient id="l" x1="0" y1="0" x2="100" y2="0" gradientUnits="userSpaceOnUse">
          <stop offset="0" stop-color="#ff0000"/><stop offset="1" stop-color="#0000ff"/></linearGradient>
        <radialGradient id="r" cx="150" cy="50" r="40" gradientUnits="userSpaceOnUse">
          <stop offset="0" stop-color="#ffffff"/><stop offset="1" stop-color="#000000" stop-opacity="0.5"/></radialGradient>
        <radialGradient id="e" cx="0.5" cy="0.5" r="0.5">
          <stop offset="0" stop-color="#00ff00"/><stop offset="1" stop-color="#004400"/></radialGradient>
      </defs>
      <rect x="0" y="0" width="100" height="100" fill="url(#l)"/>
      <rect x="110" y="10" width="80" height="80" fill="url(#r)"/>
      <rect x="200" y="20" width="100" height="60" fill="url(#e)"/>
    </svg>"##;
    let r = fp_io::open_bytes("v.svg", svg, &ImportOptions::default()).unwrap();
    assert!(
        r.warnings.iter().all(|w| !w.contains("erste Farbe")),
        "{:?}",
        r.warnings
    );
    let mut shapes = Vec::new();
    for s in &r.document.pages[0].root.layers {
        if let fp_model::LayerSlot::Own(l) = s {
            for e in &l.cels[0].elements {
                if let fp_model::ElementKind::Path(p) = &e.kind {
                    shapes.push(p.attrs.fill.as_ref().map(|f| f.shape).unwrap_or(-1));
                }
            }
        }
    }
    shapes.sort();
    assert_eq!(shapes, vec![2, 3, 9]);
    let out = fp_render::Renderer::new(fp_render::RenderOptions::default()).render_page(
        &r.document,
        0,
        0,
    );
    let px = |x: i32, y: i32| out.image.pixel(x, y);
    let (a, b) = (px(1, 50), px(98, 50));
    assert!(a.r > 240 && a.b < 15, "linear Anfang {a:?}");
    assert!(b.b > 240 && b.r < 15, "linear Ende {b:?}");
    let c = px(150, 50);
    assert!(c.r > 245 && c.g > 245, "radial Mitte {c:?}");
    let e = px(250, 50);
    assert!(e.g > 230 && e.r < 20, "elliptisch Mitte {e:?}");
    let edge = px(201, 50);
    assert!(edge.g < 110, "elliptisch Rand {edge:?}");
    dump(&r.document, "svg_gradients.svg");
}

/// Mit `FP_DUMP_DIR` das gelesene Bild als PNG ablegen (Sichtprüfung).
fn dump(doc: &fp_model::Document, name: &str) {
    let Some(dir) = std::env::var_os("FP_DUMP_DIR") else {
        return;
    };
    let img = fp_render::Renderer::default().render_page(doc, 0, 0).image;
    let px: Vec<u32> = img
        .data()
        .iter()
        .map(|p| {
            u32::from(p[3]) << 24 | u32::from(p[0]) << 16 | u32::from(p[1]) << 8 | u32::from(p[2])
        })
        .collect();
    if let Ok(png) =
        write_png_truecolor(&px, img.width(), img.height(), true, &PngOptions::default())
    {
        let file = name.rsplit('/').next().unwrap_or(name).replace('.', "_") + ".png";
        let _ = std::fs::write(std::path::Path::new(&dir).join(file), png);
    }
}

fn mp4_box(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut v = ((body.len() + 8) as u32).to_be_bytes().to_vec();
    v.extend_from_slice(kind);
    v.extend_from_slice(body);
    v
}

fn full_box(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut b = vec![0u8; 4];
    b.extend_from_slice(body);
    mp4_box(kind, &b)
}

/// Animiertes AVIF (`avis`) aus drei mit ravif kodierten Standbildern (AV1-Schlüsselbilder als
/// Samples einer Bildsequenz-Spur): Bildzahl, Dauer, Pixel.
#[test]
fn avif_sequence() {
    let src = frames3();
    let mut samples = Vec::new();
    for f in &src {
        let opaque: Vec<u32> = f.pixels.iter().map(|p| p | 0xFF00_0000).collect();
        let still = write_avif(&opaque, W, H, 95, 10).unwrap();
        let h = fp_io::isobmff::Heif::parse(&still).unwrap().unwrap();
        samples.push(h.data(h.primary).unwrap());
    }
    let ftyp = mp4_box(b"ftyp", b"avis\0\0\0\0avismsf1iso8mif1miaf");
    let mdat_body: Vec<u8> = samples.concat();
    let mdat = mp4_box(b"mdat", &mdat_body);
    let mut offs = Vec::new();
    let mut o = (ftyp.len() + 8) as u32;
    for s in &samples {
        offs.push(o);
        o += s.len() as u32;
    }
    let u32s = |v: &[u32]| v.iter().flat_map(|x| x.to_be_bytes()).collect::<Vec<u8>>();
    let mut entry = vec![0u8; 78];
    entry[24..26].copy_from_slice(&(W as u16).to_be_bytes());
    entry[26..28].copy_from_slice(&(H as u16).to_be_bytes());
    let stsd = full_box(b"stsd", &[u32s(&[1]), mp4_box(b"av01", &entry)].concat());
    let stts = full_box(b"stts", &u32s(&[2, 2, 50, 1, 150]));
    let stsc = full_box(b"stsc", &u32s(&[1, 1, 1, 1]));
    let mut stsz_body = u32s(&[0, 3]);
    stsz_body.extend(u32s(
        &samples.iter().map(|s| s.len() as u32).collect::<Vec<_>>(),
    ));
    let stsz = full_box(b"stsz", &stsz_body);
    let stco = full_box(b"stco", &[u32s(&[3]), u32s(&offs)].concat());
    let stbl = mp4_box(b"stbl", &[stsd, stts, stsc, stsz, stco].concat());
    let minf = mp4_box(b"minf", &stbl);
    let mdhd = full_box(b"mdhd", &u32s(&[0, 0, 1000, 250, 0]));
    let hdlr = full_box(
        b"hdlr",
        &[u32s(&[0]), b"pict".to_vec(), vec![0; 13]].concat(),
    );
    let mdia = mp4_box(b"mdia", &[mdhd, hdlr, minf].concat());
    let tkhd = full_box(b"tkhd", &[u32s(&[0, 0, 1, 0, 250]), vec![0; 60]].concat());
    let elst = mp4_box(b"edts", &{
        let mut b = vec![0, 0, 0, 1];
        b.extend(u32s(&[1, 250, 0, 0x0001_0000]));
        mp4_box(b"elst", &b)
    });
    let trak = mp4_box(b"trak", &[tkhd, elst, mdia].concat());
    let moov = mp4_box(b"moov", &trak);
    let file = [ftyp, mdat, moov].concat();
    assert_eq!(detect_format(&file, ""), FileFormat::Avif);
    let a = fp_io::avif_in::decode_avif(&file).unwrap();
    assert_eq!(a.frames.len(), 3);
    assert_eq!(
        a.frames.iter().map(|f| f.delay_cs).collect::<Vec<_>>(),
        vec![5, 5, 15]
    );
    assert_eq!(a.loop_count, 0, "elst-Flag 1 = endlos");
    for (k, (s, f)) in src.iter().zip(&a.frames).enumerate() {
        let opaque: Vec<u32> = s.pixels.iter().map(|p| p | 0xFF00_0000).collect();
        let e = mean_error(&opaque, &f.image);
        assert!(e < 3.0, "AVIF-Sequenz Bild {k}: mittlerer Fehler {e}");
    }
    let r = fp_io::open_bytes("a.avif", &file, &ImportOptions::default()).unwrap();
    assert_eq!(r.content, ImportedContent::Animation { frames: 3 });
}

/// Echte AVIF-Dateien anderer Kodierer (Testbilder aus Crate-Quellen der Cargo-Registry, falls vorhanden).
#[test]
fn avif_foreign_files() {
    let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) else {
        return;
    };
    let reg = std::path::Path::new(&home)
        .join(".cargo")
        .join("registry")
        .join("src");
    let Ok(idx) = std::fs::read_dir(&reg) else {
        return;
    };
    let mut n = 0;
    for d in idx.flatten() {
        for (c, f) in [
            (
                "avif-rust-0.0.8",
                "tests/plum-blossom-small.profile1.8bpc.yuv444.alpha-full.avif",
            ),
            ("avif-rust-0.0.8", "tests/WML2Viewer.avif"),
            ("gamut-avif-1.1.0", "tests/data/default_lossy50.avif"),
            ("avif-decode-3.0.0", "tests/data/tiny.avif"),
        ] {
            let p = d.path().join(c).join(f);
            let Ok(data) = std::fs::read(&p) else {
                continue;
            };
            let r = fp_io::open_bytes(&p.to_string_lossy(), &data, &ImportOptions::default())
                .unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            let pg = &r.document.pages[0];
            assert!(pg.canvas.width >= 1.0 && pg.canvas.height >= 1.0);
            dump(&r.document, f);
            n += 1;
        }
    }
    eprintln!("{n} fremde AVIF-Dateien gelesen");
}

#[test]
fn qoi_and_ico() {
    let px = test_pixels(true);
    let data = write_qoi(&px, W, H, true).unwrap();
    assert_eq!(detect_format(&data, ""), FileFormat::Qoi);
    assert_exact(&px, &fp_io::modern::decode_qoi(&data).unwrap().image, "QOI");

    let sized = |s: usize| -> Vec<u32> {
        (0..s * s)
            .map(|i| {
                0x8000_0000
                    | ((i % s) * 255 / s) as u32
                    | (((i / s) * 255 / s) as u32 * 0x100)
                    | if i % 7 == 0 { 0x7F00_0000 } else { 0 }
            })
            .collect()
    };
    let sizes = [16usize, 32, 256];
    let pix: Vec<Vec<u32>> = sizes.iter().map(|&s| sized(s)).collect();
    let imgs: Vec<IcoImage> = sizes
        .iter()
        .zip(&pix)
        .map(|(&s, p)| IcoImage {
            width: s,
            height: s,
            pixels: p,
        })
        .collect();
    let data = write_ico(&imgs, 256).unwrap();
    assert_eq!(detect_format(&data, ""), FileFormat::Ico);
    let dec = fp_io::modern::decode_ico(&data).unwrap();
    assert_eq!(dec.len(), 3);
    for (&s, p) in sizes.iter().zip(&pix) {
        let d = dec
            .iter()
            .find(|d| d.width() as usize == s)
            .expect("Größe fehlt");
        let diffs: Vec<String> = p
            .iter()
            .zip(d.image.data())
            .enumerate()
            .filter(|(_, (e, g))| !same(to_rgba(**e), **g))
            .map(|(i, (e, g))| format!("{i} {:?}≠{g:?}", to_rgba(*e)))
            .take(4)
            .collect();
        assert!(
            diffs.is_empty(),
            "ICO {}×{}: {diffs:?}",
            d.width(),
            d.height()
        );
    }
}

#[test]
fn tiff_bmp_wbmp() {
    let px = test_pixels(true);
    let idx = test_indexed();
    let expected_idx: Vec<u32> = idx
        .indices
        .iter()
        .map(|&i| idx.palette[i as usize])
        .collect();
    for c in [TiffCompression::None, TiffCompression::PackBits] {
        let data = write_tiff(
            &RasterData::Truecolor {
                width: W,
                height: H,
                pixels: &px,
                alpha: true,
            },
            72.0,
            c,
            Some("Firepixel"),
        )
        .unwrap();
        assert_eq!(detect_format(&data, ""), FileFormat::Tiff);
        assert_exact(
            &px,
            &fp_io::tiff_in::decode(&data, None).unwrap().image,
            &format!("TIFF32 {c:?}"),
        );
        let data = write_tiff(&RasterData::Indexed(&idx), 72.0, c, None).unwrap();
        assert_exact(
            &expected_idx,
            &fp_io::tiff_in::decode(&data, None).unwrap().image,
            &format!("TIFF8 {c:?}"),
        );
    }
    let data = write_bmp(
        &RasterData::Truecolor {
            width: W,
            height: H,
            pixels: &px,
            alpha: true,
        },
        72.0,
    )
    .unwrap();
    assert_eq!(detect_format(&data, ""), FileFormat::Bmp);
    // 32 Bit wie im Original mit 40-Byte-Kopf und `BI_RGB` (re\27 E5); das vierte Byte ist Alpha
    assert_eq!(
        u32::from_le_bytes(data[14..18].try_into().unwrap()),
        40,
        "BITMAPINFOHEADER"
    );
    assert_exact(&px, &fp_io::bmp::decode(&data).unwrap().image, "BMP32");
    let opaque = test_pixels(false);
    let data = write_bmp(
        &RasterData::Truecolor {
            width: W,
            height: H,
            pixels: &opaque,
            alpha: false,
        },
        72.0,
    )
    .unwrap();
    assert_exact(&opaque, &fp_io::bmp::decode(&data).unwrap().image, "BMP24");
    let data = write_bmp(&RasterData::Indexed(&idx), 96.0).unwrap();
    let dec = fp_io::bmp::decode(&data).unwrap();
    assert_exact(&expected_idx, &dec.image, "BMP8");
    // 96 dpi → 3779 px/m → 96 dpi (Formeln des Originals mit 2,5406, re\27 E5)
    assert_eq!(u32::from_le_bytes(data[38..42].try_into().unwrap()), 3779);
    assert_eq!(dec.dpi, Some(96.0));

    let bw = IndexedImage {
        palette: vec![0xFF00_0000, 0xFFFF_FFFF],
        indices: idx.indices.iter().map(|&i| i % 2).collect(),
        counts: vec![0, 0],
        ..idx.clone()
    };
    let data = write_wbmp(&bw).unwrap();
    let expected: Vec<u32> = bw.indices.iter().map(|&i| bw.palette[i as usize]).collect();
    assert_exact(
        &expected,
        &fp_io::wbmp::decode(&data).unwrap().image,
        "WBMP",
    );
}

#[test]
fn jpeg_roundtrip() {
    let px = test_pixels(false);
    let tables = FwJpegTables::ijg_fallback();
    for (progressive, subsampling) in [(false, 0), (true, 0), (false, 1)] {
        let params = JpegParams {
            quality: 95,
            progressive,
            subsampling,
            ..JpegParams::default()
        };
        let data = encode_jpeg(&px, W, H, &params, &tables).unwrap();
        assert_eq!(detect_format(&data, ""), FileFormat::Jpeg);
        let img = fp_io::jpeg_in::decode(&data).unwrap();
        // unabhängiger Dekoder (zune-jpeg) liest denselben Strom gleichwertig
        let z = fp_io::jpeg_in::decode_zune(&data).unwrap();
        let ez = mean_error(&px, &z.image);
        assert!(
            ez < 4.0,
            "JPEG über zune (progressiv {progressive}): mittlerer Fehler {ez}"
        );
        let e = mean_error(&px, &img.image);
        assert!(
            e < 4.0,
            "JPEG (progressiv {progressive}, Unterabtastung {subsampling}): mittlerer Fehler {e}"
        );
        assert_eq!(img.dpi, Some(72.0));
    }
}

/// Vollständige Export-Pipeline (`export_image`) für alle eingebauten Vorgaben und die modernen
/// Formate: Datei wird erkannt und mit gleicher Größe gelesen; verlustfreie Truecolor-Formate pixelgenau.
#[test]
fn export_pipeline_presets() {
    use fp_export::export::{EncodeExtras, export_image};
    use fp_export::options::{ColorMode, ExportFormat, OptimizeSettings, builtin_presets};
    let px = test_pixels(true);
    let img = RgbaImage::from_raw(
        IRect::from_xywh(0, 0, W as i32, H as i32),
        px.iter().map(|&p| to_rgba(p)).collect(),
    )
    .unwrap();
    let ctx = ExportContext {
        canvas: 0,
        ..ExportContext::default()
    };
    let mut cases: Vec<(String, OptimizeSettings)> = builtin_presets()
        .into_iter()
        .map(|p| (p.name.to_string(), p.settings))
        .collect();
    for f in [
        ExportFormat::WebP,
        ExportFormat::WebPLossless,
        ExportFormat::Avif,
        ExportFormat::Qoi,
        ExportFormat::Ico,
        ExportFormat::Tiff,
        ExportFormat::Bmp,
        ExportFormat::Wbmp,
    ] {
        let s = OptimizeSettings {
            format: f,
            color_mode: ColorMode::Rgba32,
            ..OptimizeSettings::default()
        };
        cases.push((format!("{f:?}"), s));
    }
    let mut n = 0;
    for (name, s) in cases {
        if matches!(s.format, ExportFormat::Pict | ExportFormat::Custom(_)) {
            continue;
        }
        let out = export_image(&img, &s, &ctx, &EncodeExtras::standalone())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let fname = format!("x.{}", s.format.extension());
        let r = fp_io::open_bytes(&fname, &out.bytes, &ImportOptions::default());
        let r = r.unwrap_or_else(|e| panic!("{name}: {e}"));
        let page = &r.document.pages[0];
        if s.format != ExportFormat::Ico {
            assert_eq!(
                (page.canvas.width, page.canvas.height),
                (W as f32, H as f32),
                "{name}: Größe"
            );
        }
        let lossless_tc = matches!(
            s.format,
            ExportFormat::WebPLossless | ExportFormat::Qoi | ExportFormat::Tiff | ExportFormat::Bmp
        ) || (s.format == ExportFormat::Png && s.color_mode == ColorMode::Rgba32);
        if lossless_tc {
            let got = SimpleRenderer
                .render(&r.document, &RenderRequest::default())
                .unwrap();
            assert_exact(&px, &got, &name);
        }
        n += 1;
    }
    assert!(n >= 20, "nur {n} Fälle");
}

#[test]
fn psd_roundtrip() {
    let bg = test_pixels(false);
    let background = RgbaImage::from_raw(
        IRect::from_xywh(0, 0, W as i32, H as i32),
        bg.iter().map(|&p| to_rgba(p)).collect(),
    )
    .unwrap();
    // Oberste Ebene: deckendes Rechteck 10×6 bei (20, 12)
    let top = RgbaImage::from_raw(
        IRect::from_xywh(20, 12, 10, 6),
        vec![[200, 30, 60, 255]; 60],
    )
    .unwrap();
    let mut composite = background.clone();
    for y in 12..18 {
        for x in 20..30 {
            composite.data_mut()[y * W + x] = [200, 30, 60, 255];
        }
    }
    let layers = [
        PsdLayer {
            name: "Hintergrund".into(),
            image: background,
            opacity: 255,
            blend: 0,
            visible: true,
            blocks: Vec::new(),
        },
        PsdLayer {
            name: "Rechteck".into(),
            image: top,
            opacity: 255,
            blend: 0,
            visible: true,
            blocks: Vec::new(),
        },
    ];
    let data = write_psd(W, H, &layers, &composite, 72.0, &[]).unwrap();
    assert_eq!(detect_format(&data, ""), FileFormat::Psd);
    let r = fp_io::open_bytes("a.psd", &data, &ImportOptions::default()).unwrap();
    assert_eq!(r.content, ImportedContent::Layered);
    let img = SimpleRenderer
        .render(&r.document, &RenderRequest::default())
        .unwrap();
    let expected: Vec<u32> = composite
        .data()
        .iter()
        .map(|p| {
            u32::from(p[3]) << 24 | u32::from(p[0]) << 16 | u32::from(p[1]) << 8 | u32::from(p[2])
        })
        .collect();
    assert_exact(&expected, &img, "PSD");
    let mut names = Vec::new();
    for slot in &r.document.pages[0].root.layers {
        if let fp_model::LayerSlot::Own(l) = slot
            && let Some(c) = l.cels.first()
        {
            names.extend(c.elements.iter().filter_map(|e| e.common.name.clone()));
        }
    }
    assert!(
        names.contains(&"Hintergrund".to_string()) && names.contains(&"Rechteck".to_string()),
        "Ebenennamen: {names:?}"
    );
}

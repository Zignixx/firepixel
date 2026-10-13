//! BATCH-5: Metadaten (EXIF/XMP/IPTC/ICC behalten, entfernen, setzen, Ausrichtung), Formate
//! (TIFF-Kompression, TGA, Metadaten-Übernahme), Optimieren (PNG verlustfrei, Farben reduzieren,
//! Zielgröße) – mit Golden-Fällen (`tools\golden\batch\…`) und Logikfällen nach AGENTEN_REGELN §4b.

mod support;

use fp_batch::item::{decode_bytes, still_from_rgba, Item};
use fp_batch::meta::{self, container, exif, pngopt, quantize, target};
use fp_batch::output::{encode, encode_plain, OutputFormat};
use fp_batch::recipe::{TargetKind, TargetSpec};
use fp_batch::{executor, Recipe, RunOptions};
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};
use support::*;

// ------------------------------------------------------------------ Testdaten

/// Bild w×h: linke Hälfte rot, rechte blau, mit leichtem Verlauf.
fn halves(w: u32, h: u32) -> Vec<u8> {
    let mut v = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let g = (y * 40 / h.max(1)) as u8;
            v.extend_from_slice(&if x < w / 2 { [230, g, 20, 255] } else { [20, g, 230, 255] });
        }
    }
    v
}

/// „Foto“ mit vielen Farben (für Zielgröße).
fn photo(w: u32, h: u32) -> Vec<u8> {
    let mut v = Vec::new();
    let mut s: u32 = 12345;
    for y in 0..h {
        for x in 0..w {
            s = s.wrapping_mul(1_103_515_245).wrapping_add(12345);
            let n = (s >> 24) as u8 / 6;
            v.extend_from_slice(&[((x * 255 / w) as u8).wrapping_add(n), ((y * 255 / h) as u8).wrapping_add(n), (((x + y) * 127 / (w + h)) as u8).wrapping_add(n), 255]);
        }
    }
    v
}

fn le16(v: u16) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}

/// EXIF mit Kamera, Software, Urheber, Datum, Ausrichtung, GPS, Herstellerdaten und Vorschaubild.
fn sample_exif(orientation: u16) -> Vec<u8> {
    let mut e = exif::Exif::new_empty();
    e.set_ascii(exif::TAG_MAKE, "TestCam");
    e.set_ascii(exif::TAG_MODEL, "Model X");
    e.set_ascii(exif::TAG_SOFTWARE, "Editor 1.0");
    e.set_orientation(orientation);
    let mut sub = exif::Ifd::default();
    let mut d = b"2024:05:06 07:08:09".to_vec();
    d.push(0);
    sub.set(exif::Entry { tag: exif::TAG_DATETIME_ORIGINAL, ty: 2, count: d.len() as u32, data: d });
    sub.set(exif::Entry { tag: exif::TAG_ISO, ty: 3, count: 1, data: le16(200) });
    sub.set(exif::Entry { tag: exif::TAG_MAKER_NOTE, ty: 7, count: 12, data: b"SERIAL-12345".to_vec() });
    let mut f = 28u32.to_le_bytes().to_vec();
    f.extend_from_slice(&10u32.to_le_bytes());
    sub.set(exif::Entry { tag: exif::TAG_FNUMBER, ty: 5, count: 1, data: f });
    e.exif = Some(sub);
    let mut gps = exif::Ifd::default();
    gps.set(exif::Entry { tag: 0, ty: 1, count: 4, data: vec![2, 3, 0, 0] });
    gps.set(exif::Entry { tag: 1, ty: 2, count: 2, data: b"N\0".to_vec() });
    let mut lat = Vec::new();
    for (n, d) in [(52u32, 1u32), (31, 1), (0, 1)] {
        lat.extend_from_slice(&n.to_le_bytes());
        lat.extend_from_slice(&d.to_le_bytes());
    }
    gps.set(exif::Entry { tag: 2, ty: 5, count: 3, data: lat });
    e.gps = Some(gps);
    let mut i1 = exif::Ifd::default();
    i1.set(exif::Entry { tag: 0x0103, ty: 3, count: 1, data: le16(6) });
    e.ifd1 = Some(i1);
    e.thumbnail = Some(vec![0xFF, 0xD8, 1, 2, 3, 4, 0xFF, 0xD9]);
    e.write()
}

const XMP_SAMPLE: &str = "<?xpacket begin=\"\"?><x:xmpmeta><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"><rdf:Description rdf:about=\"\" xmlns:tiff=\"t\" xmlns:exif=\"e\" tiff:Make=\"TestCam\" tiff:Orientation=\"6\" exif:GPSLatitude=\"52,31N\"><exif:GPSLongitude>13,24E</exif:GPSLongitude><dc:title>Bild</dc:title></rdf:Description></rdf:RDF></x:xmpmeta><?xpacket end=\"w\"?>";

/// Kleines (gültiges) ICC-Profil-Gerüst mit Beschreibung und Füllbytes.
fn sample_icc(len: usize) -> Vec<u8> {
    let mut v = vec![0u8; 132 + 12];
    v[128..132].copy_from_slice(&1u32.to_be_bytes());
    v[132..136].copy_from_slice(b"desc");
    let off = v.len() as u32;
    let text = b"Test RGB\0";
    let mut desc = b"desc\0\0\0\0".to_vec();
    desc.extend_from_slice(&(text.len() as u32).to_be_bytes());
    desc.extend_from_slice(text);
    v[136..140].copy_from_slice(&off.to_be_bytes());
    v[140..144].copy_from_slice(&(desc.len() as u32).to_be_bytes());
    v.extend_from_slice(&desc);
    while v.len() < len {
        v.push((v.len() % 251) as u8);
    }
    let l = v.len() as u32;
    v[0..4].copy_from_slice(&l.to_be_bytes());
    v
}

fn sample_meta(orientation: u16) -> container::Found {
    container::Found { exif: Some(sample_exif(orientation)), icc: Some(sample_icc(300)), xmp: Some(XMP_SAMPLE.as_bytes().to_vec()), iptc: Some(vec![0x1C, 2, 0x50, 0, 3, b'A', b'B', b'C']), comment: Some("hallo".into()) }
}

/// JPEG w×h (Hälften) mit Metadaten.
fn jpeg_with_meta(w: u32, h: u32, orientation: u16) -> Vec<u8> {
    let item = still_from_rgba(w, h, &halves(w, h)).expect("item");
    let (plain, _) = encode_plain(&item, OutputFormat::Jpeg, &Map::new()).expect("jpeg");
    container::write_jpeg(&plain, &sample_meta(orientation)).expect("write meta")
}

fn opts() -> RunOptions {
    RunOptions { threads: Some(2), ..RunOptions::default() }
}

fn recipe(out: &Path, format: &str) -> Recipe {
    let mut r = Recipe::named("meta");
    r.output.format = format.into();
    r.output.target = TargetSpec { kind: TargetKind::Folder, path: Some(out.to_path_buf()), ..TargetSpec::default() };
    r
}

fn run(r: &Recipe, src: &[PathBuf]) -> fp_batch::Report {
    let items = fp_batch::collect(src, false, false, &[]).items;
    let rep = executor::run(r, &items, &opts()).expect("run");
    assert_eq!(rep.totals.done, src.len(), "{:#?}", rep.items);
    rep
}

fn out_file(rep: &fp_batch::Report, k: usize) -> PathBuf {
    rep.items[k].target.clone().expect("target")
}

fn read_meta(path: &Path) -> container::Found {
    container::read(&std::fs::read(path).expect("read"))
}

fn pixel(img: &fp_raster::RgbaImage, x: i32, y: i32) -> [u8; 4] {
    img.get(x, y).expect("pixel")
}

// ------------------------------------------------------------------ EXIF-Struktur

#[test]
fn exif_roundtrip_summary_and_filter() {
    let raw = sample_exif(6);
    let e = exif::Exif::parse(&raw).expect("parse");
    let s = e.summary();
    assert_eq!(s.make.as_deref(), Some("TestCam"));
    assert_eq!(s.model.as_deref(), Some("Model X"));
    assert_eq!(s.iso, Some(200));
    assert_eq!(s.aperture.as_deref(), Some("f/2.8"));
    assert_eq!(s.orientation, Some(6));
    assert_eq!(s.date_taken.map(|d| d.to_string()).as_deref(), Some("2024-05-06 07:08:09"));
    assert!(e.has_gps());
    assert_eq!(e.thumbnail.as_deref(), Some(&[0xFF, 0xD8, 1, 2, 3, 4, 0xFF, 0xD9][..]));
    // neu schreiben → gleich lesbar (Offsets neu berechnet)
    let again = exif::Exif::parse(&e.write()).expect("reparse");
    assert_eq!(again, e);
    // Big Endian: neuer Block in MM-Reihenfolge bleibt MM und lesbar
    let mut be = exif::Exif { little_endian: false, ..exif::Exif::default() };
    be.set_ascii(exif::TAG_MAKE, "BigCam");
    be.set_orientation(3);
    let w = be.write();
    assert_eq!(&w[0..2], b"MM");
    let back = exif::Exif::parse(&w).expect("be");
    assert_eq!((back.summary().make.as_deref(), back.orientation()), (Some("BigCam"), Some(3)));
    let mut wm = w.clone();
    assert!(exif::set_orientation_in_place(&mut wm, 1));
    assert_eq!(exif::Exif::parse(&wm).and_then(|x| x.orientation()), Some(1));
    // Ausrichtung an Ort und Stelle
    let mut r2 = raw.clone();
    assert!(exif::set_orientation_in_place(&mut r2, 1));
    assert_eq!(r2.len(), raw.len());
    assert_eq!(exif::Exif::parse(&r2).and_then(|x| x.orientation()), Some(1));
    // Tag-Namen
    assert_eq!(exif::tag_from_value(&json!("Make")), Some(0x010F));
    assert_eq!(exif::tag_from_value(&json!("0x0112")), Some(0x0112));
    assert_eq!(exif::tag_from_value(&json!(305)), Some(305));
    assert_eq!(exif::tag_from_value(&json!("unsinn")), None);
    // kaputte Blöcke
    assert!(exif::Exif::parse(b"").is_none());
    assert!(exif::Exif::parse(b"II*\0\xff\xff\xff\xff").is_none());
    assert!(exif::Exif::parse(&raw[..20]).is_some_and(|e| e.ifd0.entries.len() <= 1) || exif::Exif::parse(&raw[..20]).is_none());
}

#[test]
fn orient_all_eight_cases() {
    // 3×2: Pixelwert = Index; Referenz über bekannte Abbildungen
    let rgba: Vec<u8> = (0..6u8).flat_map(|i| [i, 0, 0, 255]).collect();
    let img = fp_raster::RgbaImage::from_rgba_bytes(fp_raster::IRect::new(0, 0, 3, 2), &rgba).expect("img");
    let ids = |o: u16| -> (usize, usize, Vec<u8>) {
        let r = meta::orient(&img, o);
        (r.width(), r.height(), r.data().iter().map(|p| p[0]).collect())
    };
    assert_eq!(ids(1), (3, 2, vec![0, 1, 2, 3, 4, 5]));
    assert_eq!(ids(2), (3, 2, vec![2, 1, 0, 5, 4, 3]));
    assert_eq!(ids(3), (3, 2, vec![5, 4, 3, 2, 1, 0]));
    assert_eq!(ids(4), (3, 2, vec![3, 4, 5, 0, 1, 2]));
    assert_eq!(ids(5), (2, 3, vec![0, 3, 1, 4, 2, 5]));
    assert_eq!(ids(6), (2, 3, vec![3, 0, 4, 1, 5, 2]));
    assert_eq!(ids(7), (2, 3, vec![5, 2, 4, 1, 3, 0]));
    assert_eq!(ids(8), (2, 3, vec![2, 5, 1, 4, 0, 3]));
}

// ------------------------------------------------------------------ Container

#[test]
fn jpeg_png_webp_containers_roundtrip() {
    let mut m = sample_meta(1);
    m.icc = Some(sample_icc(150_000)); // > 64 KB → mehrere APP2-Teile
    let item = still_from_rgba(16, 8, &halves(16, 8)).expect("item");
    // JPEG
    let (j, _) = encode_plain(&item, OutputFormat::Jpeg, &Map::new()).expect("jpeg");
    let j2 = container::write_jpeg(&j, &m).expect("jpeg meta");
    assert_eq!(container::read(&j2), m);
    // zweimal schreiben = einmal (keine doppelten Segmente)
    assert_eq!(container::write_jpeg(&j2, &m).expect("again"), j2);
    assert!(decode_bytes("x.jpg", &j2).is_ok());
    // leer schreiben entfernt alles
    let j0 = container::write_jpeg(&j2, &container::Found::default()).expect("strip");
    assert!(container::read(&j0).is_empty());
    // PNG (IPTC kann PNG nicht tragen)
    let (p, _) = encode_plain(&item, OutputFormat::Png, &Map::new()).expect("png");
    let mp = container::Found { iptc: None, ..m.clone() };
    let p2 = container::write_png(&p, &mp).expect("png meta");
    assert_eq!(container::read(&p2), mp);
    assert_eq!(container::write_png(&p2, &mp).expect("again"), p2);
    assert_eq!(pngopt::decode_rgba(&p2).map(|x| x.2), pngopt::decode_rgba(&p).map(|x| x.2));
    // WebP verlustbehaftet (VP8) und verlustfrei mit Alpha (VP8L)
    for lossless in [false, true] {
        let mut o = Map::new();
        o.insert("lossless".into(), json!(lossless));
        let alpha = still_from_rgba(16, 8, &rgba(16, 8, true)).expect("alpha");
        let (w, _) = encode_plain(if lossless { &alpha } else { &item }, OutputFormat::WebP, &o).expect("webp");
        let mw = container::Found { iptc: None, comment: None, ..m.clone() };
        let w2 = container::write_webp(&w, &mw).expect("webp meta");
        assert_eq!(container::read(&w2), mw, "lossless {lossless}");
        let d = decode_bytes("x.webp", &w2).expect("decode webp with meta");
        assert_eq!(d.dimensions(), (16, 8));
        if lossless {
            assert_eq!(d.preview_image().expect("img").as_bytes(), decode_bytes("y.webp", &w).expect("plain").preview_image().expect("img").as_bytes());
        }
    }
    assert_eq!(meta::icc_description(&sample_icc(300)).as_deref(), Some("Test RGB"));
}

// ------------------------------------------------------------------ Lauf: Übernahme, Ausrichtung

#[test]
fn conversion_keeps_metadata_and_applies_orientation() {
    let dir = temp("meta-keep");
    let src = dir.join("foto.jpg");
    std::fs::write(&src, jpeg_with_meta(40, 20, 6)).expect("write");
    // EXIF-Leser für Namensbausteine ist eingehängt
    let _ = fp_batch::StepRegistry::global();
    assert_eq!(fp_batch::item::read_exif(&src).and_then(|s| s.model).as_deref(), Some("Model X"));
    for fmt in ["jpeg", "png", "webp"] {
        let rep = run(&recipe(&dir.join(format!("out-{fmt}")), fmt), std::slice::from_ref(&src));
        let t = out_file(&rep, 0);
        let m = read_meta(&t);
        let e = exif::Exif::parse(m.exif.as_deref().expect("exif kept")).expect("exif");
        assert_eq!(e.orientation(), Some(1), "{fmt}: Ausrichtung auf 1");
        assert!(e.has_gps(), "{fmt}: ohne Schritt bleibt alles");
        assert_eq!(e.summary().make.as_deref(), Some("TestCam"));
        assert!(m.icc.is_some(), "{fmt}: ICC");
        let x = String::from_utf8(m.xmp.expect("xmp")).expect("utf8");
        assert!(x.contains("tiff:Orientation=\"1\""), "{fmt}: XMP-Ausrichtung");
        // Pixel gedreht: 20×40, oben rot, unten blau
        let img = decode_bytes(&t.to_string_lossy(), &std::fs::read(&t).expect("read")).expect("decode").preview_image().expect("img");
        assert_eq!((img.width(), img.height()), (20, 40), "{fmt}");
        let top = pixel(&img, 10, 5);
        let bottom = pixel(&img, 10, 35);
        assert!(top[0] > 150 && top[2] < 100, "{fmt}: oben rot {top:?}");
        assert!(bottom[2] > 150 && bottom[0] < 100, "{fmt}: unten blau {bottom:?}");
        if fmt == "jpeg" {
            assert_eq!(m.comment.as_deref(), Some("hallo"));
            assert!(m.iptc.is_some());
        }
    }
    // Formate ohne Metadaten: Hinweis, Datei trotzdem fertig
    for fmt in ["gif", "bmp"] {
        let rep = run(&recipe(&dir.join(format!("out-{fmt}")), fmt), std::slice::from_ref(&src));
        assert!(rep.items[0].warnings.iter().any(|w| w.contains("EXIF") && w.contains(&fmt.to_uppercase())), "{fmt}: {:?}", rep.items[0].warnings);
        assert_eq!(dims(&out_file(&rep, 0)), (20, 40));
    }
}

// ------------------------------------------------------------------ metadata.edit / set

fn meta_after(steps: &[(&str, Value)], fmt: &str, name: &str) -> (container::Found, fp_batch::Report) {
    let dir = temp(name);
    let src = dir.join("foto.jpg");
    std::fs::write(&src, jpeg_with_meta(24, 16, 1)).expect("write");
    let mut r = recipe(&dir.join("out"), fmt);
    for (id, p) in steps {
        r.add_step(id, p.clone());
    }
    let rep = run(&r, &[src]);
    (read_meta(&out_file(&rep, 0)), rep)
}

#[test]
fn metadata_edit_removes_gps_and_is_idempotent() {
    let (m, _) = meta_after(&[("metadata.edit", json!({}))], "jpeg", "edit-gps");
    let e = exif::Exif::parse(m.exif.as_deref().expect("exif")).expect("parse");
    assert!(!e.has_gps(), "GPS weg");
    assert_eq!(e.summary().make.as_deref(), Some("TestCam"), "Rest bleibt");
    assert!(e.thumbnail.is_some());
    let x = String::from_utf8(m.xmp.clone().expect("xmp")).expect("utf8");
    assert!(!x.contains("GPS"), "XMP ohne GPS: {x}");
    assert!(x.contains("tiff:Make"));
    // Schritt zweimal = einmal (gleiche Bytes)
    let (m2, _) = meta_after(&[("metadata.edit", json!({})), ("metadata.edit", json!({}))], "jpeg", "edit-gps2");
    assert_eq!(m2, m);
    // Parameter 50× ändern → ein Schritt mit Endwert
    let mut r = Recipe::named("x");
    let i = r.add_step("metadata.edit", json!({}));
    for k in 0..50 {
        let list = if k % 2 == 0 { json!(["gps", "camera"]) } else { json!(["gps"]) };
        assert!(r.set_param(i, "remove", list));
    }
    assert_eq!(r.steps.len(), 1);
    assert_eq!(r.steps[0].params["remove"], json!(["gps"]));
}

#[test]
fn metadata_edit_categories_and_none() {
    let (m, _) = meta_after(&[("metadata.edit", json!({"remove": ["camera", "software", "thumbnails", "comments", "iptc"], "tags": ["DateTimeOriginal"]}))], "jpeg", "edit-cats");
    let e = exif::Exif::parse(m.exif.as_deref().expect("exif")).expect("parse");
    let s = e.summary();
    assert_eq!((s.make, s.model), (None, None));
    assert!(e.find(exif::TAG_MAKER_NOTE).is_none(), "Herstellerdaten (Seriennummer) weg");
    assert!(e.find(exif::TAG_SOFTWARE).is_none());
    assert!(e.thumbnail.is_none());
    assert!(s.date_taken.is_none(), "einzelnes Tag entfernt");
    assert_eq!(s.iso, Some(200));
    assert!(e.has_gps(), "GPS nicht gewählt → bleibt");
    assert!(m.comment.is_none() && m.iptc.is_none());
    assert!(!String::from_utf8_lossy(m.xmp.as_deref().expect("xmp")).contains("tiff:Make"));
    // keep none: alles weg außer Farbprofil
    let (n, _) = meta_after(&[("metadata.edit", json!({"keep": "none"}))], "jpeg", "edit-none");
    assert!(n.exif.is_none() && n.xmp.is_none() && n.iptc.is_none() && n.comment.is_none());
    assert!(n.icc.is_some());
    let (n2, _) = meta_after(&[("metadata.edit", json!({"keep": "none", "remove": ["icc"]}))], "png", "edit-none-icc");
    assert!(n2.is_empty());
    // ungültige Tags → Validierungsfehler
    let mut r = Recipe::named("x");
    r.add_step("metadata.edit", json!({"tags": ["kein-tag"]}));
    assert!(r.validate(&fp_batch::StepRegistry::global()).iter().any(|i| i.param.as_deref() == Some("tags")));
}

#[test]
fn metadata_set_writes_exif_and_xmp_once() {
    let p = json!({"artist": "Ada {name}", "copyright": "© {year} Ada", "keywords": "a, b; c", "rating": 4, "dateTaken": "+1h", "gps": "remove"});
    let (m, _) = meta_after(&[("metadata.set", p.clone()), ("metadata.set", p)], "jpeg", "set");
    let e = exif::Exif::parse(m.exif.as_deref().expect("exif")).expect("parse");
    assert_eq!(e.find(exif::TAG_ARTIST).and_then(exif::Exif::text).as_deref(), Some("Ada foto"));
    let year = chrono::Local::now().format("%Y").to_string();
    assert_eq!(e.find(exif::TAG_COPYRIGHT).and_then(exif::Exif::text), Some(format!("© {year} Ada")));
    // zweimal +1h → +2h (Verschieben ist eine Änderung je Anwendung); Datum lesbar
    assert_eq!(e.summary().date_taken.map(|d| d.to_string()).as_deref(), Some("2024-05-06 09:08:09"));
    assert!(!e.has_gps());
    let x = String::from_utf8(m.xmp.expect("xmp")).expect("utf8");
    assert_eq!(x.matches("<dc:creator>").count(), 1, "{x}");
    assert_eq!(x.matches("<dc:subject>").count(), 1);
    assert!(x.contains("<rdf:li>Ada foto</rdf:li>") && x.contains("<rdf:li>c</rdf:li>"));
    assert!(x.contains("<xmp:Rating>4</xmp:Rating>"));
    assert!(!x.contains("GPS"));
    // ohne Quell-Metadaten: neuer EXIF-Block und neues XMP-Paket
    let dir = temp("set-new");
    let src = dir.join("plain.png");
    write_png(&src, 8, 6);
    let mut r = recipe(&dir.join("out"), "png");
    r.add_step("metadata.set", json!({"artist": "Bob", "dateTaken": "2020-01-02 03:04"}));
    let rep = run(&r, std::slice::from_ref(&src));
    let m = read_meta(&out_file(&rep, 0));
    let e = exif::Exif::parse(m.exif.as_deref().expect("exif")).expect("parse");
    assert_eq!(e.find(exif::TAG_ARTIST).and_then(exif::Exif::text).as_deref(), Some("Bob"));
    assert_eq!(e.summary().date_taken.map(|d| d.to_string()).as_deref(), Some("2020-01-02 03:04:00"));
    assert!(String::from_utf8_lossy(m.xmp.as_deref().expect("xmp")).contains("Bob"));
    // ungültiges Datum → Validierung
    let mut bad = Recipe::named("x");
    bad.add_step("metadata.set", json!({"dateTaken": "gestern"}));
    assert!(bad.validate(&fp_batch::StepRegistry::global()).iter().any(|i| i.param.as_deref() == Some("dateTaken")));
}

// ------------------------------------------------------------------ Formate

#[test]
fn tiff_compressions_and_tga_roundtrip() {
    let px = photo(37, 23);
    let mut alpha = px.clone();
    for (i, p) in alpha.chunks_mut(4).enumerate() {
        p[3] = (i % 256) as u8;
    }
    for data in [&px, &alpha] {
        let mut item = still_from_rgba(37, 23, data).expect("item");
        if let Some(m) = item.meta_mut() {
            m.icc = Some(sample_icc(200));
            m.xmp = Some(XMP_SAMPLE.as_bytes().to_vec());
        }
        for c in ["default", "none", "lzw", "deflate", "packbits"] {
            let mut o = Map::new();
            o.insert("compression".into(), json!(c));
            let (b, _) = encode(&item, OutputFormat::Tiff, &o).expect("tiff");
            let d = decode_bytes("x.tif", &b).expect("decode tiff");
            assert_eq!(d.preview_image().expect("img").as_bytes(), &data[..], "TIFF {c}");
        }
        for c in ["default", "none"] {
            let mut o = Map::new();
            o.insert("compression".into(), json!(c));
            let (b, _) = encode(&item, OutputFormat::Tga, &o).expect("tga");
            let d = decode_bytes("x.tga", &b).expect("decode tga");
            let got = d.preview_image().expect("img");
            assert_eq!(got.as_bytes(), &data[..], "TGA {c}");
        }
    }
    // großes, gleichförmiges Bild: LZW mit Tabellen-Reset
    let big: Vec<u8> = (0..300 * 200).flat_map(|i| [(i % 7) as u8 * 30, (i / 300 % 5) as u8 * 40, 9, 255]).collect();
    let item = still_from_rgba(300, 200, &big).expect("big");
    let (b, _) = encode(&item, OutputFormat::Tiff, &Map::new()).expect("tiff");
    assert_eq!(decode_bytes("x.tif", &b).expect("decode").preview_image().expect("img").as_bytes(), &big[..]);
    // TGA ist als Ziel wählbar (Kodierer eingehängt)
    let _ = fp_batch::StepRegistry::global();
    assert!(OutputFormat::Tga.can_encode());
}

// ------------------------------------------------------------------ Optimieren

#[test]
fn png_optimizer_is_lossless_and_smaller() {
    // wenige Farben → Palette; Grau → Graustufen; Foto → RGB; nie größer
    let few: Vec<u8> = (0..64 * 64).flat_map(|i| if (i / 64 + i % 64) % 3 == 0 { [255, 0, 0, 255] } else { [0, 0, 255, 255] }).collect();
    let gray: Vec<u8> = photo(64, 64).chunks(4).flat_map(|p| [p[0], p[0], p[0], 255]).collect();
    let ph = photo(64, 64);
    let alpha = rgba(64, 64, true);
    for (name, data) in [("few", &few), ("gray", &gray), ("photo", &ph), ("alpha", &alpha)] {
        let item = still_from_rgba(64, 64, data).expect("item");
        let (plain, _) = encode_plain(&item, OutputFormat::Png, &Map::new()).expect("png");
        for level in 1..=6u8 {
            let o = pngopt::optimize(&plain, pngopt::PngOptOptions { level, alpha_clean: false });
            assert!(o.len() <= plain.len(), "{name} level {level}");
            assert_eq!(pngopt::decode_rgba(&o).expect("decode").2, pngopt::decode_rgba(&plain).expect("decode").2, "{name}: verlustfrei");
            assert!(decode_bytes("x.png", &o).is_ok());
        }
        if name == "few" {
            let o = pngopt::optimize(&plain, pngopt::PngOptOptions::default());
            let chunks = container::png_chunks(&o).expect("chunks");
            assert_eq!(chunks[0].1[9], 3, "Palette");
            assert_eq!(chunks[0].1[8], 1, "1 Bit");
            assert!(o.len() * 2 < plain.len(), "{} vs {}", o.len(), plain.len());
        }
        if name == "gray" {
            let o = pngopt::optimize(&plain, pngopt::PngOptOptions::default());
            let ct = container::png_chunks(&o).expect("chunks")[0].1[9];
            assert!(ct == 0 || ct == 3, "Graustufen bzw. Palette, nicht RGB: {ct}");
        }
    }
    // Schritt optimize.png im Lauf: kleiner, Pixel gleich, „safe“ entfernt EXIF, behält ICC
    let dir = temp("pngopt");
    let src = dir.join("foto.jpg");
    std::fs::write(&src, jpeg_with_meta(32, 32, 1)).expect("write");
    let rep0 = run(&recipe(&dir.join("a"), "png"), std::slice::from_ref(&src));
    let mut r = recipe(&dir.join("b"), "png");
    r.add_step("optimize.png", json!({}));
    let rep1 = run(&r, std::slice::from_ref(&src));
    let (a, b) = (out_file(&rep0, 0), out_file(&rep1, 0));
    assert!(std::fs::metadata(&b).expect("b").len() < std::fs::metadata(&a).expect("a").len());
    assert_eq!(pixels(&a), pixels(&b));
    let m = read_meta(&b);
    assert!(m.exif.is_none() && m.xmp.is_none() && m.icc.is_some());
    // bei anderem Zielformat: Hinweis, keine Wirkung
    let mut r = recipe(&dir.join("c"), "jpeg");
    r.add_step("optimize.png", json!({}));
    let rep = run(&r, std::slice::from_ref(&src));
    assert!(rep.items[0].warnings.iter().any(|w| w.contains("PNG")), "{:?}", rep.items[0].warnings);
    assert!(read_meta(&out_file(&rep, 0)).exif.is_some());
}

#[test]
fn quantize_reduces_colors() {
    let mut px: Vec<[u8; 4]> = photo(48, 48).as_chunks::<4>().0.to_vec();
    for (dither, name) in [(quantize::Dither::None, "none"), (quantize::Dither::Ordered, "ordered"), (quantize::Dither::FloydSteinberg, "fs")] {
        let mut p = px.clone();
        let pal = quantize::quantize(&mut p, 48, 16, dither, 0.8);
        assert!(pal.len() <= 16, "{name}");
        let uniq: std::collections::HashSet<[u8; 4]> = p.iter().copied().collect();
        assert!(uniq.len() <= 16, "{name}: {}", uniq.len());
        assert!(uniq.iter().all(|c| pal.contains(c)));
        // Mittelwert bleibt ungefähr erhalten
        let mean = |v: &[[u8; 4]]| v.iter().map(|c| c[0] as u64).sum::<u64>() / v.len() as u64;
        assert!(mean(&p).abs_diff(mean(&px)) < 12, "{name}");
    }
    // weniger Farben als erlaubt → unverändert (auch mit Dithering)
    px.truncate(100);
    let few: Vec<[u8; 4]> = px.iter().enumerate().map(|(i, _)| if i % 2 == 0 { [1, 2, 3, 255] } else { [9, 8, 7, 128] }).collect();
    let mut f2 = few.clone();
    quantize::quantize(&mut f2, 10, 256, quantize::Dither::FloydSteinberg, 1.0);
    assert_eq!(f2, few);
    // Schritt: PNG → Palette mit ≤ 8 Farben; GIF → keine zweite Farbreduktion über 8
    let dir = temp("quant");
    let src = dir.join("p.png");
    let item = still_from_rgba(40, 30, &photo(40, 30)).expect("item");
    std::fs::write(&src, encode_plain(&item, OutputFormat::Png, &Map::new()).expect("png").0).expect("write");
    for fmt in ["png", "gif"] {
        let mut r = recipe(&dir.join(fmt), fmt);
        r.add_step("optimize.quantize", json!({"colors": 8, "dither": "ordered"}));
        let rep = run(&r, std::slice::from_ref(&src));
        let got = pixels(&out_file(&rep, 0));
        let uniq: std::collections::HashSet<&[u8]> = got.chunks(4).collect();
        assert!(uniq.len() <= 8, "{fmt}: {}", uniq.len());
        if fmt == "png" {
            let b = std::fs::read(out_file(&rep, 0)).expect("read");
            assert_eq!(container::png_chunks(&b).expect("chunks")[0].1[9], 3, "Palette-PNG");
        }
    }
}

fn target_size_of(fmt: &str, params: Value, w: u32, h: u32, name: &str) -> (u64, Vec<String>, (u32, u32)) {
    let dir = temp(name);
    let src = dir.join("p.png");
    let item = still_from_rgba(w, h, &photo(w, h)).expect("item");
    std::fs::write(&src, encode_plain(&item, OutputFormat::Png, &Map::new()).expect("png").0).expect("write");
    let mut r = recipe(&dir.join("out"), fmt);
    r.add_step("optimize.targetSize", params);
    let rep = run(&r, std::slice::from_ref(&src));
    let t = out_file(&rep, 0);
    (std::fs::metadata(&t).expect("t").len(), rep.items[0].warnings.clone(), dims(&t))
}

#[test]
fn target_size_search() {
    // JPEG: erreichbar → ≤ Ziel, Maße unverändert
    let (b, w, d) = target_size_of("jpeg", json!({"maxKB": 20}), 320, 240, "t-jpeg");
    assert!(b <= 20 * 1024, "{b} {w:?}");
    assert_eq!(d, (320, 240));
    // großzügiges Ziel → beste Qualität (gleich groß wie ohne Schritt)
    let (big, _, _) = target_size_of("jpeg", json!({"maxKB": 100000}), 320, 240, "t-jpeg-big");
    let (none, _, _) = {
        let dir = temp("t-jpeg-none");
        let src = dir.join("p.png");
        let item = still_from_rgba(320, 240, &photo(320, 240)).expect("item");
        std::fs::write(&src, encode_plain(&item, OutputFormat::Png, &Map::new()).expect("png").0).expect("write");
        let rep = run(&recipe(&dir.join("out"), "jpeg"), std::slice::from_ref(&src));
        (std::fs::metadata(out_file(&rep, 0)).expect("m").len(), (), ())
    };
    assert_eq!(big, none);
    // unerreichbar ohne Verkleinern → Hinweis, kleinste Datei
    let (b2, w2, d2) = target_size_of("jpeg", json!({"maxKB": 1, "minQuality": 40}), 320, 240, "t-jpeg-miss");
    assert!(b2 > 1024);
    assert_eq!(d2, (320, 240));
    assert!(w2.iter().any(|x| x.contains("KB")), "{w2:?}");
    // mit Verkleinern erreichbar
    let (b3, w3, d3) = target_size_of("jpeg", json!({"maxKB": 3, "strategy": "qualityResize", "minScale": 10}), 320, 240, "t-jpeg-resize");
    assert!(b3 <= 3 * 1024, "{b3} {w3:?}");
    assert!(d3.0 < 320 && d3.1 < 240);
    // WebP und PNG (über Farben)
    let (b4, w4, _) = target_size_of("webp", json!({"maxKB": 30}), 320, 240, "t-webp");
    assert!(b4 <= 30 * 1024, "{b4} {w4:?}");
    let (b5, w5, _) = target_size_of("png", json!({"maxKB": 60}), 200, 150, "t-png");
    assert!(b5 <= 60 * 1024, "{b5} {w5:?}");
    // Spezifikation aus Parametern
    let s = target::TargetSpec::from_params(&json!({"maxKB": 100, "strategy": "qualityResize", "minScale": 50}));
    assert_eq!((s.max_bytes, s.resize, s.min_scale), (102_400, true, 0.5));
}

#[test]
fn preview_reports_encoded_size_with_target() {
    let dir = temp("prev");
    let src = dir.join("p.png");
    let item = still_from_rgba(300, 200, &photo(300, 200)).expect("item");
    std::fs::write(&src, encode_plain(&item, OutputFormat::Png, &Map::new()).expect("png").0).expect("write");
    let mut r = recipe(&dir.join("out"), "jpeg");
    r.add_step("optimize.targetSize", json!({"maxKB": 15}));
    let info = fp_batch::probe(&src).expect("probe");
    let reg = fp_batch::StepRegistry::global();
    let p = fp_batch::preview(&r, &info, &reg, &fp_batch::PreviewOptions { encode_full: true, ..Default::default() }).expect("preview");
    let rep = run(&r, std::slice::from_ref(&src));
    assert_eq!(p.bytes_after, Some(std::fs::metadata(out_file(&rep, 0)).expect("m").len()), "Vorschau = Lauf");
    assert!(p.bytes_after.unwrap_or(u64::MAX) <= 15 * 1024);
    // Vorschau zweimal → gleiches Ergebnis (nichts addiert sich)
    let p2 = fp_batch::preview(&r, &info, &reg, &fp_batch::PreviewOptions { encode_full: true, ..Default::default() }).expect("preview 2");
    assert_eq!(p2.bytes_after, p.bytes_after);
}

#[test]
fn catalog_lists_steps_with_texts() {
    let reg = fp_batch::StepRegistry::global();
    let cat = reg.catalog_json();
    for id in ["metadata.edit", "metadata.set", "metadata.orientation", "optimize.png", "optimize.quantize", "optimize.targetSize"] {
        let s = cat["steps"].as_array().expect("steps").iter().find(|s| s["id"] == id).unwrap_or_else(|| panic!("{id}"));
        assert_ne!(s["name"], json!(id), "{id}: Name übersetzt");
        assert!(s.get("description").is_some(), "{id}");
        assert!(s["params"].as_array().expect("params").iter().all(|p| p.get("label").is_some()), "{id}: Parameter beschriftet");
        let summary = reg.get(id).expect("step").summary(&json!({}));
        assert!(id == "metadata.set" || !summary.is_empty(), "{id}");
    }
    let info = meta::info(&fp_batch::item::Meta { exif: Some(sample_exif(1)), icc: Some(sample_icc(300)), ..Default::default() });
    assert_eq!(info["exif"]["gps"], json!(true));
    assert_eq!(info["exif"]["make"], json!("TestCam"));
    assert_eq!(info["icc"]["description"], json!("Test RGB"));
}

// ------------------------------------------------------------------ Golden-Fälle

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("tools").join("golden").join("batch")
}

/// Golden: `tools\golden\batch\<fall>\{in, recipe.json, expected.json}`. `FP_BLESS=1` schreibt
/// Eingaben (falls fehlend) und Erwartungen neu.
#[test]
fn golden_cases() {
    let bless = std::env::var("FP_BLESS").is_ok_and(|v| v == "1");
    for case in ["metadata-no-gps", "target-size-100kb"] {
        let dir = golden_dir().join(case);
        let input = dir.join("in").join("photo.jpg");
        if bless && !input.exists() {
            std::fs::create_dir_all(input.parent().expect("dir")).expect("mkdir");
            let bytes = if case == "metadata-no-gps" {
                // ohne XMP: dessen Pflicht-Kennung im JPEG enthält den Herstellernamen (fp-neutral)
                let item = still_from_rgba(64, 48, &halves(64, 48)).expect("item");
                let (plain, _) = encode_plain(&item, OutputFormat::Jpeg, &Map::new()).expect("jpeg");
                container::write_jpeg(&plain, &container::Found { xmp: None, ..sample_meta(6) }).expect("meta")
            } else {
                let item = still_from_rgba(640, 480, &photo(640, 480)).expect("item");
                let mut o = Map::new();
                o.insert("quality".into(), json!(95));
                encode_plain(&item, OutputFormat::Jpeg, &o).expect("jpeg").0
            };
            std::fs::write(&input, bytes).expect("write input");
        }
        let mut r = Recipe::load(&dir.join("recipe.json")).expect("recipe.json");
        let out = temp(&format!("golden-{case}"));
        r.output.target = TargetSpec { kind: TargetKind::Folder, path: Some(out.clone()), ..TargetSpec::default() };
        let rep = run(&r, std::slice::from_ref(&input));
        let t = out_file(&rep, 0);
        let bytes = std::fs::read(&t).expect("out");
        let item: Item = decode_bytes(&t.to_string_lossy(), &bytes).expect("decode");
        let got = json!({
            "file": t.file_name().map(|n| n.to_string_lossy().to_string()),
            "size": item.dimensions(),
            "meta": meta::info(item.meta().expect("meta")),
            "bytes": bytes.len(),
        });
        let exp_path = dir.join("expected.json");
        if bless {
            std::fs::write(&exp_path, serde_json::to_string_pretty(&got).expect("json") + "\n").expect("bless");
            continue;
        }
        let exp: Value = serde_json::from_str(&std::fs::read_to_string(&exp_path).expect("expected.json")).expect("json");
        assert_eq!(got["file"], exp["file"], "{case}");
        assert_eq!(got["size"], exp["size"], "{case}");
        assert_eq!(got["meta"], exp["meta"], "{case}");
        // verlustbehaftet: Größe ± 5 %
        let (g, e) = (got["bytes"].as_f64().unwrap_or(0.0), exp["bytes"].as_f64().unwrap_or(1.0));
        assert!((g - e).abs() <= e * 0.05, "{case}: {g} vs {e}");
        if case == "target-size-100kb" {
            assert!(g <= 100.0 * 1024.0, "{case}: {g}");
        }
    }
}

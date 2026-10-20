//! BA-108 remaining metadata (B-4): GPS and IPTC in `metadata.set`, TIFF read/write with EXIF
//! and IPTC, AVIF/HEIF and JPEG XL box reading and writing, collected `metadata.export`.

mod support;

use fp_batch::item::{Item, ItemInfo, ItemKind, still_from_rgba};
use fp_batch::meta::{self, container, exif::Exif, iptc};
use fp_batch::recipe::{TargetKind, TargetSpec};
use fp_batch::{
    CancelToken, OutputFormat, Recipe, RunOptions, StepContext, StepRegistry, executor,
};
use serde_json::{Value, json};
use std::path::PathBuf;
use support::*;

fn apply(id: &str, params: Value, item: &mut Item) {
    let reg = StepRegistry::global();
    let step = reg.get(id).expect("step");
    let p = fp_batch::step::with_defaults(&step.params(), &params);
    let (w, h) = item.dimensions();
    let info = ItemInfo {
        path: PathBuf::from("C:/in/a.png"),
        root: None,
        kind: ItemKind::Image,
        format: "png".into(),
        bytes: 0,
        width: w,
        height: h,
        frames: 1,
        modified: None,
        exif: None,
    };
    let cancel = CancelToken::new();
    let mut ctx = StepContext::new(0, 1, &info, &cancel, OutputFormat::Png);
    step.apply(item, &p, &mut ctx).expect("apply");
}

fn tagged() -> Item {
    let mut it = still_from_rgba(8, 6, &rgba(8, 6, false)).expect("item");
    apply(
        "metadata.set",
        json!({"artist": "Ann", "keywords": "sea, sky", "title": "Coast",
               "gps": "set", "latitude": -33.8568, "longitude": 151.2153, "altitude": "58"}),
        &mut it,
    );
    it
}

#[test]
fn set_writes_gps_and_iptc() {
    let it = tagged();
    let m = it.meta().expect("meta");
    let e = Exif::parse(m.exif.as_deref().expect("exif")).expect("parse");
    let (lat, lon) = e.gps_position().expect("gps");
    assert!((lat + 33.8568).abs() < 1e-5 && (lon - 151.2153).abs() < 1e-5);
    let irb = meta::iptc(m).expect("iptc");
    assert_eq!(iptc::record2(&irb, iptc::DS_KEYWORDS), vec!["sea", "sky"]);
    assert_eq!(iptc::record2(&irb, iptc::DS_BYLINE), vec!["Ann"]);
    assert_eq!(iptc::record2(&irb, iptc::DS_OBJECT_NAME), vec!["Coast"]);
    // invalid altitude is a validation error
    let mut r = Recipe::named("v");
    r.add_step("metadata.set", json!({"gps": "set", "altitude": "high"}));
    assert!(
        r.validate(&StepRegistry::global())
            .iter()
            .any(|i| i.param.as_deref() == Some("altitude"))
    );
}

#[test]
fn tiff_keeps_exif_gps_and_iptc() {
    let it = tagged();
    let (bytes, warnings) =
        fp_batch::output::encode(&it, OutputFormat::Tiff, &serde_json::Map::new()).expect("tiff");
    assert!(warnings.is_empty(), "{warnings:?}");
    let f = container::read(&bytes);
    let e = Exif::parse(f.exif.as_deref().expect("exif read back")).expect("parse");
    assert_eq!(artist(&e).as_deref(), Some("Ann"));
    assert!(e.gps_position().is_some());
    assert_eq!(
        iptc::record2(f.iptc.as_deref().expect("iptc"), iptc::DS_KEYWORDS),
        vec!["sea", "sky"]
    );
    assert!(f.xmp.is_some());
    // the file still decodes
    let back = fp_batch::item::decode_bytes("x.tif", &bytes).expect("decode");
    assert_eq!(back.dimensions(), (8, 6));
}

fn artist(e: &Exif) -> Option<String> {
    e.ifd0
        .get(fp_batch::meta::exif::TAG_ARTIST)
        .and_then(Exif::text)
}

fn bx(ty: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut v = ((payload.len() + 8) as u32).to_be_bytes().to_vec();
    v.extend_from_slice(ty);
    v.extend_from_slice(payload);
    v
}

fn exif_block() -> Vec<u8> {
    let mut e = Exif::new_empty();
    e.set_ascii(fp_batch::meta::exif::TAG_ARTIST, "Box");
    e.write()
}

const XMP: &[u8] = b"<x:xmpmeta xmlns:x=\"x\"><rdf:RDF/></x:xmpmeta>";

fn heif_file() -> Vec<u8> {
    let mut exif_item = 0u32.to_be_bytes().to_vec();
    exif_item.extend_from_slice(b"Exif\0\0");
    exif_item.extend_from_slice(&exif_block());
    let infe = |id: u16, ty: &[u8; 4], extra: &[u8]| {
        let mut p = vec![2, 0, 0, 0];
        p.extend_from_slice(&id.to_be_bytes());
        p.extend_from_slice(&0u16.to_be_bytes());
        p.extend_from_slice(ty);
        p.push(0); // name
        p.extend_from_slice(extra);
        bx(b"infe", &p)
    };
    let mut iinf = vec![0, 0, 0, 0, 0, 2];
    iinf.extend(infe(1, b"Exif", &[]));
    iinf.extend(infe(2, b"mime", b"application/rdf+xml\0"));
    let ftyp = bx(b"ftyp", b"avifmif1");
    // layout: ftyp, meta, mdat; offsets need the meta size → build twice
    let build = |exif_off: u32, xmp_off: u32| {
        let mut iloc = vec![0, 0, 0, 0, 0x44, 0x00];
        iloc.extend_from_slice(&2u16.to_be_bytes());
        for (id, off, len) in [
            (1u16, exif_off, exif_item.len() as u32),
            (2, xmp_off, XMP.len() as u32),
        ] {
            iloc.extend_from_slice(&id.to_be_bytes());
            iloc.extend_from_slice(&0u16.to_be_bytes());
            iloc.extend_from_slice(&1u16.to_be_bytes());
            iloc.extend_from_slice(&off.to_be_bytes());
            iloc.extend_from_slice(&len.to_be_bytes());
        }
        let mut meta = vec![0, 0, 0, 0];
        meta.extend(bx(b"iinf", &iinf));
        meta.extend(bx(b"iloc", &iloc));
        bx(b"meta", &meta)
    };
    let meta_len = build(0, 0).len();
    let mdat_start = (ftyp.len() + meta_len + 8) as u32;
    let meta = build(mdat_start, mdat_start + exif_item.len() as u32);
    let mut file = ftyp.clone();
    file.extend(meta);
    let mut mdat = exif_item.clone();
    mdat.extend_from_slice(XMP);
    file.extend(bx(b"mdat", &mdat));
    file
}

#[test]
fn heif_items_are_read() {
    let file = heif_file();
    let f = container::read(&file);
    let e = Exif::parse(f.exif.as_deref().expect("exif")).expect("parse");
    assert_eq!(artist(&e).as_deref(), Some("Box"));
    assert_eq!(f.xmp.as_deref(), Some(XMP));
}

#[test]
fn jxl_boxes_are_read() {
    let mut file = b"\0\0\0\x0CJXL \r\n\x87\n".to_vec();
    file.extend(bx(b"ftyp", b"jxl \0\0\0\0jxl "));
    let mut ex = 0u32.to_be_bytes().to_vec();
    ex.extend(exif_block());
    file.extend(bx(b"Exif", &ex));
    file.extend(bx(b"xml ", XMP));
    file.extend(bx(b"jxlc", &[0xFF, 0x0A]));
    let f = container::read(&file);
    assert!(f.exif.is_some());
    assert_eq!(f.xmp.as_deref(), Some(XMP));
}

#[test]
fn export_collects_one_csv() {
    let dir = temp("meta-collect");
    let srcs: Vec<PathBuf> = (0..3)
        .map(|k| {
            let p = dir.join(format!("m{k}.png"));
            write_png(&p, 10 + k, 5);
            p
        })
        .collect();
    let list = fp_batch::collect(&srcs, true, false, &[]).items;
    let out = dir.join("out");
    let mut r = Recipe::named("mc");
    r.output.format = "png".into();
    r.output.target = TargetSpec {
        kind: TargetKind::Folder,
        path: Some(out.clone()),
        ..TargetSpec::default()
    };
    r.add_step("metadata.export", json!({"format": "csv", "collect": true}));
    executor::run(&r, &list, &RunOptions::default()).expect("run");
    let csv = std::fs::read_to_string(out.join("metadata.csv")).expect("csv");
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 4, "{csv}");
    assert!(lines[1].starts_with("m0.png,") && lines[3].starts_with("m2.png,"));
    assert!(!out.join("m0.csv").exists(), "no per-item sidecars");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn avif_writes_exif_and_xmp_with_iptc_fields() {
    // BA-108: EXIF/XMP items in the encoded AVIF; IPTC fields travel in XMP
    let it = tagged();
    let (bytes, warnings) =
        fp_batch::output::encode(&it, OutputFormat::Avif, &serde_json::Map::new()).expect("avif");
    assert!(warnings.is_empty(), "{warnings:?}");
    let f = container::read(&bytes);
    let e = Exif::parse(f.exif.as_deref().expect("exif read back")).expect("parse");
    assert_eq!(artist(&e).as_deref(), Some("Ann"));
    assert!(e.gps_position().is_some());
    let x = String::from_utf8(f.xmp.expect("xmp")).expect("utf8");
    assert!(
        x.contains("Coast") && x.contains("sea") && x.contains("Ann"),
        "{x}"
    );
    // the file still decodes, also after a second write and after removing everything
    let back = fp_batch::item::decode_bytes("x.avif", &bytes).expect("decode");
    assert_eq!(back.dimensions(), (8, 6));
    let again = container::write_heif(&bytes, &f_with_artist("Bob")).expect("again");
    let e = Exif::parse(container::read(&again).exif.as_deref().expect("exif")).expect("parse");
    assert_eq!(artist(&e).as_deref(), Some("Bob"));
    let back = fp_batch::item::decode_bytes("x.avif", &again).expect("decode again");
    assert_eq!(back.dimensions(), (8, 6));
    let none = container::write_heif(&again, &container::Found::default()).expect("strip");
    assert!(container::read(&none).is_empty());
    assert!(fp_batch::item::decode_bytes("x.avif", &none).is_ok());
}

fn f_with_artist(a: &str) -> container::Found {
    let mut e = Exif::new_empty();
    e.set_ascii(fp_batch::meta::exif::TAG_ARTIST, a);
    container::Found {
        exif: Some(e.write()),
        xmp: Some(XMP.to_vec()),
        ..Default::default()
    }
}

/// 42-byte JPEG XL codestream from the documentation example of the `jxl-oxide` crate.
const TINY_JXL: [u8; 42] = [
    0xff, 0x0a, 0x30, 0x54, 0x10, 0x09, 0x08, 0x06, 0x01, 0x00, 0x78, 0x00, 0x4b, 0x38, 0x41, 0x3c,
    0xb6, 0x3a, 0x51, 0xfe, 0x00, 0x47, 0x1e, 0xa0, 0x85, 0xb8, 0x27, 0x1a, 0x48, 0x45, 0x84, 0x1b,
    0x71, 0x4f, 0xa8, 0x3e, 0x8e, 0x30, 0x03, 0x92, 0x84, 0x01,
];

#[test]
fn jxl_codestream_is_wrapped_and_still_decodes() {
    let plain = fp_batch::item::decode_bytes("x.jxl", &TINY_JXL).expect("decode codestream");
    let boxed = container::write_jxl(&TINY_JXL, &f_with_artist("Jo")).expect("wrap");
    assert!(boxed.starts_with(b"\0\0\0\x0CJXL \r\n\x87\n"));
    let f = container::read(&boxed);
    assert_eq!(
        artist(&Exif::parse(f.exif.as_deref().expect("exif")).expect("parse")).as_deref(),
        Some("Jo")
    );
    assert_eq!(f.xmp.as_deref(), Some(XMP));
    let back = fp_batch::item::decode_bytes("x.jxl", &boxed).expect("decode container");
    assert_eq!(back.dimensions(), plain.dimensions());
    // replacing inside the container keeps it decodable
    let again = container::write_jxl(&boxed, &f_with_artist("Kim")).expect("again");
    assert!(fp_batch::item::decode_bytes("x.jxl", &again).is_ok());
}

#[test]
fn heif_items_are_written_back() {
    let file = heif_file();
    let out = container::write(&file, &f_with_artist("Neu")).expect("write");
    let f = container::read(&out);
    let e = Exif::parse(f.exif.as_deref().expect("exif")).expect("parse");
    assert_eq!(artist(&e).as_deref(), Some("Neu"));
    assert_eq!(f.xmp.as_deref(), Some(XMP));
}

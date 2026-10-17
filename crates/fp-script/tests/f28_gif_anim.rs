//! F28 (TW-108, run 13 `export-results/07`): animated GIF of `par_export.fw.png` (T09) with
//! `setExportOptions({animAutoCrop: false})` against the reference file of run 12
//! (`07_gif_anim_result_orig_par.gif`): global palette of 256 entries with the last one free
//! (`#000000`), transparency index 255 in every image, disposal 1, delay 7, both images full size.
//! Only structure, palette size and transparency are compared (the reference quantizer is not
//! reproduced yet, see `docs\berichte\f28.md`).

use fp_engine::Session;
use fp_script::fileio::export_page;
use fp_script::{ScriptOptions, run_source};
use serde_json::{Value, json};
use std::path::Path;

const T09: &str = r"C:\Users\Administrator\Desktop\REA\re\_testdateien\par\par_export.fw.png";
const GOLDEN: &str = r"C:\Users\Administrator\Desktop\REA\re\_testdateien\par\run_lauf12_2026-10-17\export-results\files\07_gif_anim_result_orig_par.gif";

#[derive(Debug, PartialEq)]
struct Frame {
    rect: [u16; 4],
    disposal: u8,
    delay: u16,
    transparent: Option<u8>,
    local_palette: bool,
}

#[derive(Debug)]
struct Gif {
    size: [u16; 2],
    palette: Vec<[u8; 3]>,
    frames: Vec<Frame>,
}

fn parse(d: &[u8]) -> Gif {
    assert_eq!(&d[..6], b"GIF89a");
    let size = [
        u16::from_le_bytes([d[6], d[7]]),
        u16::from_le_bytes([d[8], d[9]]),
    ];
    let flags = d[10];
    let mut i = 13;
    let mut palette = Vec::new();
    if flags & 0x80 != 0 {
        let n = 2usize << (flags & 7);
        palette = (0..n)
            .map(|k| [d[i + 3 * k], d[i + 3 * k + 1], d[i + 3 * k + 2]])
            .collect();
        i += 3 * n;
    }
    let skip_blocks = |mut i: usize| {
        while d[i] != 0 {
            i += 1 + d[i] as usize;
        }
        i + 1
    };
    let mut frames = Vec::new();
    let mut gce: Option<(u8, u16, Option<u8>)> = None;
    while d[i] != 0x3B {
        match d[i] {
            0x21 => {
                if d[i + 1] == 0xF9 {
                    let pk = d[i + 3];
                    gce = Some((
                        (pk >> 2) & 7,
                        u16::from_le_bytes([d[i + 4], d[i + 5]]),
                        (pk & 1 == 1).then_some(d[i + 6]),
                    ));
                }
                i = skip_blocks(i + 2);
            }
            0x2C => {
                let r = |k: usize| u16::from_le_bytes([d[i + 1 + 2 * k], d[i + 2 + 2 * k]]);
                let rect = [r(0), r(1), r(2), r(3)];
                let pk = d[i + 9];
                i += 10;
                if pk & 0x80 != 0 {
                    i += 3 * (2usize << (pk & 7));
                }
                i = skip_blocks(i + 1);
                let (disposal, delay, transparent) = gce.take().unwrap_or((0, 0, None));
                frames.push(Frame {
                    rect,
                    disposal,
                    delay,
                    transparent,
                    local_palette: pk & 0x80 != 0,
                });
            }
            b => panic!("unexpected block {b:#x} at {i}"),
        }
    }
    Gif {
        size,
        palette,
        frames,
    }
}

fn run(s: &mut Session, src: &str) {
    let o = run_source(s, src, &ScriptOptions::default());
    assert!(o.ok, "error: {:?}", o.error);
}

#[test]
fn tw108_animated_gif_palette_and_transparency_like_reference() {
    let src = Path::new(T09);
    if !src.is_file() {
        eprintln!("skipped: {src:?} missing");
        return;
    }
    let dir = std::env::temp_dir().join(format!("fp-f28-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let copy = dir.join("par_export.fw.png");
    std::fs::copy(src, &copy).expect("copy");
    let mut s = Session::default();
    s.execute("file.open", json!({"path": copy.to_string_lossy()}))
        .expect("open");
    // run 13: the runner's JSF, then "Animated GIF" in the Optimize panel (format only)
    run(
        &mut s,
        "fw.getDocumentDOM().setExportOptions({exportFormat:'GIF', animAutoCrop:false}); 0",
    );
    run(
        &mut s,
        "fw.getDocumentDOM().setExportOptions({exportFormat:'animated gif'}); 0",
    );
    let out = dir.join("par.gif");
    let id = s.active().expect("doc").id;
    export_page(&s, id, None, None, &out, &Value::Null).expect("export");
    let fp = parse(&std::fs::read(&out).expect("read"));

    // expectations as read from the reference file
    let golden = Path::new(GOLDEN);
    let reference = golden
        .is_file()
        .then(|| parse(&std::fs::read(golden).expect("golden")));
    if let Some(r) = &reference {
        assert_eq!(r.palette.len(), 256);
        assert_eq!(r.palette[255], [0, 0, 0]);
        assert!(r.frames.iter().all(|f| f.transparent == Some(255)));
    }
    assert_eq!(fp.size, [400, 300]);
    assert_eq!(fp.palette.len(), 256, "global palette with 256 entries");
    assert_eq!(
        fp.palette[255],
        [0, 0, 0],
        "last entry free (#000000, no auto crop)"
    );
    assert_eq!(fp.frames.len(), 2);
    for (k, f) in fp.frames.iter().enumerate() {
        assert_eq!(
            f.rect,
            [0, 0, 400, 300],
            "image {k} full size (auto crop off)"
        );
        assert_eq!(
            f.transparent,
            Some(255),
            "image {k}: transparency index 255"
        );
        assert!(!f.local_palette);
    }
    if let Some(r) = &reference {
        assert_eq!(fp.size, r.size);
        assert_eq!(fp.frames, r.frames, "frame structure like the reference");
    }
    let _ = std::fs::remove_dir_all(dir);
}

/// `setExportOptions` takes `animAutoCrop` / `animAutoDifference` into the page settings.
#[test]
fn tw108_set_export_options_anim_flags() {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 40, "height": 30}))
        .expect("new");
    let page = |s: &Session| {
        let d = s.active().expect("doc");
        let p = &d.doc.pages[d.page];
        p.legacy_export
            .as_deref()
            .or(p.root.export.as_deref())
            .cloned()
            .expect("export settings")
    };
    run(
        &mut s,
        "fw.getDocumentDOM().setExportOptions({animAutoCrop:false, animAutoDifference:false}); 0",
    );
    let e = page(&s);
    assert!(!e.anim_auto_crop && !e.anim_auto_difference);
    run(
        &mut s,
        "fw.getDocumentDOM().setExportOptions({animAutoCrop:true, animAutoDifference:true}); 0",
    );
    let e = page(&s);
    assert!(e.anim_auto_crop && e.anim_auto_difference);
}

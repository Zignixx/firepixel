//! F27 (TW-102/103, run 12 `export-results/07,08`): exporting without options (`exportDocumentAs(doc,
//! url, null)`, bridge/MCP `export`) uses the page's export settings from the Optimize panel when they
//! fit the extension: PNG 8 → palette, PNG 24 → RGB without alpha, animated GIF → one image per state.

use fp_engine::Session;
use fp_export::{ColorMode, ExportFormat, OptimizeSettings};
use fp_script::fileio::{ExportSpec, encode, export_page};
use serde_json::{Value, json};
use std::path::Path;
use std::sync::Arc;

fn session(states: usize) -> Session {
    let mut s = Session::default();
    s.execute(
        "file.new",
        json!({"width": 40, "height": 30, "background": "#ffffff"}),
    )
    .expect("new");
    s.execute(
        "draw.rectangle",
        json!({"rect": [5, 5, 30, 20], "fillColor": "#3366cc"}),
    )
    .expect("rect");
    for k in 1..states {
        s.execute("state.add", json!({})).expect("state");
        s.execute(
            "draw.rectangle",
            json!({"rect": [5 + 3 * k, 8, 25, 28], "fillColor": "#cc3333"}),
        )
        .expect("rect");
    }
    s
}

fn set_format(s: &mut Session, f: ExportFormat, m: ColorMode) {
    let d = s.active_mut().expect("doc");
    let doc = Arc::make_mut(&mut d.doc);
    let page = Arc::make_mut(&mut doc.pages[0]);
    let mut o = OptimizeSettings::default();
    o.format = f;
    o.color_mode = m;
    let base = page.root.export.as_deref().cloned().unwrap_or_default();
    page.root.export = Some(Box::new(o.to_model(&base)));
    page.legacy_export = None;
}

/// PNG colour type byte in IHDR (3 = palette, 2 = RGB, 6 = RGBA).
fn png_color_type(b: &[u8]) -> u8 {
    assert_eq!(&b[1..4], b"PNG");
    b[25]
}

fn bytes(s: &Session, name: &str, opts: &Value) -> Vec<u8> {
    let d = s.active().expect("doc");
    encode(
        &d.doc,
        d.page,
        d.frame,
        &ExportSpec::from(Path::new(name), opts),
    )
    .expect("encode")
}

#[test]
fn png_bit_depth_from_optimize_settings_tw102() {
    let mut s = session(1);
    set_format(&mut s, ExportFormat::Png, ColorMode::Indexed);
    assert_eq!(
        png_color_type(&bytes(&s, "a.png", &Value::Null)),
        3,
        "PNG 8 → palette"
    );
    set_format(&mut s, ExportFormat::Png, ColorMode::Rgb24);
    assert_eq!(
        png_color_type(&bytes(&s, "a.png", &Value::Null)),
        2,
        "PNG 24 → RGB without alpha"
    );
    set_format(&mut s, ExportFormat::Png, ColorMode::Rgba32);
    assert_eq!(
        png_color_type(&bytes(&s, "a.png", &Value::Null)),
        6,
        "PNG 32 → RGBA"
    );
    // explicit options still win
    set_format(&mut s, ExportFormat::Png, ColorMode::Indexed);
    assert_eq!(
        png_color_type(&bytes(&s, "a.png", &json!({"colorMode": "32 bit"}))),
        6
    );
    // a format that does not fit the extension leaves the extension in charge
    set_format(&mut s, ExportFormat::Jpeg, ColorMode::Rgb24);
    assert_eq!(png_color_type(&bytes(&s, "a.png", &Value::Null)), 6);
}

#[test]
fn animated_gif_from_optimize_settings_tw103() {
    let mut s = session(2);
    let states = s.active().expect("doc").frame_count();
    set_format(&mut s, ExportFormat::AnimatedGif, ColorMode::Indexed);
    let dir = std::env::temp_dir().join(format!("fp-f27-{}", std::process::id()));
    let p = dir.join("par.gif");
    let id = s.active().expect("doc").id;
    export_page(&s, id, None, None, &p, &Value::Null).expect("export");
    let b = std::fs::read(&p).expect("read");
    // image descriptors (0x2C after a graphic control extension 0x21 0xF9 …) = number of images
    let images = b
        .windows(2)
        .filter(|w| w[0] == 0x00 && w[1] == 0x2C)
        .count();
    assert!(states >= 2, "test document has {states} states");
    assert!(
        images >= 2,
        "animated GIF with one image per state, got {images}"
    );
    // plain GIF setting keeps one image
    set_format(&mut s, ExportFormat::Gif, ColorMode::Indexed);
    export_page(&s, id, None, None, &p, &Value::Null).expect("export");
    let b = std::fs::read(&p).expect("read");
    assert!(
        b.windows(2)
            .filter(|w| w[0] == 0x00 && w[1] == 0x2C)
            .count()
            <= 1,
        "plain GIF: one image"
    );
    let _ = std::fs::remove_dir_all(dir);
}

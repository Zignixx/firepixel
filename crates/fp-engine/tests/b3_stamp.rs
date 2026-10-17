//! B3 (BA-041): Stempel PAR 05/03 – zwei Züge (330,260 → 390,260 und 330,300 → 390,300, Quelle per
//! Alt-Klick 120,268, Kreis 16, Rand 50, ausgerichtet) gegen das Ergebnisbild des Originals
//! `re\_testdateien\par\px\tool-stamp-replace-redeye\s03_aligned.png`. Ohne Testdateien übersprungen.

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent};
use serde_json::json;
use std::path::PathBuf;

fn par_dir() -> PathBuf {
    PathBuf::from(r"C:\Users\Administrator\Desktop\REA\re\_testdateien\par")
}

fn open_t02() -> Option<Session> {
    let src = par_dir().join("work").join("p57_t02.fw.png");
    if !src.is_file() {
        return None;
    }
    let mut s = doc_session(10.0, 10.0);
    let dir = std::env::temp_dir().join(format!("fp-b3-stamp-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let p = dir.join("t02.fw.png");
    std::fs::copy(&src, &p).ok()?;
    ex(&mut s, "file.open", json!({"path": p.to_string_lossy()}));
    Some(s)
}

fn render(s: &Session) -> fp_raster::RgbaImage {
    let d = s.active().expect("Dokument");
    fp_engine::render::default_renderer().expect("Renderer").render_page(&d.doc, d.page, d.frame).expect("Bild")
}

fn stroke(s: &mut Session, y: f32) {
    events(s, &[ToolEvent::down(330.0, y)]);
    for i in 1..=30 {
        events(s, &[ToolEvent::drag(330.0 + 2.0 * i as f32, y)]);
    }
    events(s, &[ToolEvent::up(390.0, y)]);
}

/// Abweichung zum Original: (Pixel mit Kanalabstand > 16, größter Abstand, geänderte Pixel).
fn diff(a: &fp_raster::RgbaImage, b: &fp_raster::RgbaImage, base: &fp_raster::RgbaImage) -> (usize, u8, usize) {
    let (mut n, mut max, mut changed) = (0, 0u8, 0);
    for ((p, q), o) in a.data().iter().zip(b.data()).zip(base.data()) {
        let d = (0..4).map(|c| p[c].abs_diff(q[c])).max().unwrap_or(0);
        if d > 16 {
            n += 1;
        }
        max = max.max(d);
        if p != o {
            changed += 1;
        }
    }
    (n, max, changed)
}

#[test]
fn stamp_two_strokes_close_to_original() {
    let Some(mut s) = open_t02() else { return };
    let Some(want) = std::fs::read(par_dir().join("px").join("tool-stamp-replace-redeye").join("s03_aligned.png")).ok().and_then(|d| fp_io::png_in::decode_png(&d).ok()).map(|r| r.image) else { return };
    let base = render(&s);
    s.set_tool(ToolId::RubberStamp).expect("Stempel");
    events(&mut s, &[ToolEvent::down(120.0, 268.0).alt(), ToolEvent::up(120.0, 268.0).alt()]);
    stroke(&mut s, 260.0);
    stroke(&mut s, 300.0);
    let img = render(&s);
    assert_eq!((img.width(), img.height()), (want.width(), want.height()));
    let (n, max, changed) = diff(&img, &want, &base);
    println!("Stempel 05/03: {changed} geänderte Pixel (Original 2 528), {n} Pixel > 16, max {max}");
    // Stand B3 (14.10.): 4 Pixel > 16, max 18 – vorher (PAR-F) 1 214 Pixel, max 136
    assert!(n <= 8 && max <= 24, "{n} Pixel weichen um mehr als 16 ab (max {max}, geändert {changed})");
    assert!(changed.abs_diff(2528) <= 120, "geänderte Pixel {changed}");
}

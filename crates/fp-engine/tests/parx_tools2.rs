//! PAR-X-TOOLS-2: Stempel/Farbe ersetzen/Rote Augen, Bitmap-Auswahl, Farbeimer/Verlauf/Pipette
//! nach den Messwerten des Originals (Einheiten 05–07, `docs\parity\tool-stamp-replace-redeye.md`,
//! `tool-bitmap-selection.md`, `tool-fill-gradient-eyedropper.md`). Sollwerte:
//! `re\_testdateien\par\px\<einheit>\pixels.csv` (Hash = sha256 der RGBA-Pixel, 16 Zeichen; Auswahlen
//! als Loch gemessen: Entf, Export). Ergebnisbilder ohne Oberfläche nach `tests\data\parx-tools2\`.
//! Fehlen die Testdateien (anderer Rechner), werden die Fälle übersprungen.

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent, ToolKey};
use fp_model::{ElementId, ElementKind};
use serde_json::{json, Value};
use sha2::Digest;
use std::path::PathBuf;

fn par_dir() -> PathBuf {
    PathBuf::from(r"C:\Users\Administrator\Desktop\REA\re\_testdateien\par")
}

fn out_dir() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("data").join("parx-tools2");
    let _ = std::fs::create_dir_all(&d);
    d
}

fn save_png(name: &str, img: &fp_raster::RgbaImage) {
    let px: Vec<u32> = img.data().iter().map(|p| (p[3] as u32) << 24 | (p[0] as u32) << 16 | (p[1] as u32) << 8 | p[2] as u32).collect();
    if let Ok(bytes) = fp_export::png_out::write_png_truecolor(&px, img.width(), img.height(), true, &fp_export::png_out::PngOptions::default()) {
        let _ = std::fs::write(out_dir().join(name), bytes);
    }
}

/// sha256 der RGBA-Pixel, 16 Hex-Zeichen (Verfahren von `pixels.csv`).
fn sha16(img: &fp_raster::RgbaImage) -> String {
    let h = sha2::Sha256::digest(img.as_bytes());
    h.iter().take(8).map(|b| format!("{b:02x}")).collect()
}

/// Geänderte Pixel gegenüber `base` und deren Grenzen (links, oben, rechts, unten; exklusiv).
fn changed(img: &fp_raster::RgbaImage, base: &fp_raster::RgbaImage) -> (usize, (i32, i32, i32, i32)) {
    let (mut n, mut l, mut t, mut r, mut b) = (0, i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    let w = img.width() as i32;
    for (k, (p, q)) in img.data().iter().zip(base.data()).enumerate() {
        if p != q {
            let (x, y) = (k as i32 % w, k as i32 / w);
            n += 1;
            l = l.min(x);
            t = t.min(y);
            r = r.max(x + 1);
            b = b.max(y + 1);
        }
    }
    (n, (l, t, r, b))
}

fn fastrand_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(1);
    N.fetch_add(1, Ordering::Relaxed)
}

/// Arbeitskopie einer PAR-Testdatei öffnen, B1 (erste Bitmap) gewählt (`None` = Datei fehlt).
fn open_par(name: &str) -> Option<Session> {
    let src = par_dir().join("work").join(name);
    if !src.is_file() {
        eprintln!("übersprungen: {src:?} fehlt");
        return None;
    }
    let mut s = doc_session(10.0, 10.0);
    let dir = std::env::temp_dir().join(format!("fp-parx-tools2-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let p = dir.join(format!("{}-{name}", fastrand_id()));
    std::fs::copy(&src, &p).ok()?;
    ex(&mut s, "file.open", json!({"path": p.to_string_lossy()}));
    s.set_colors(Some(fp_color::Argb(0xffc0_4040)), Some(fp_color::Argb(0xff80_8080)));
    if let Some(b) = first_of(&s, |k| matches!(k, ElementKind::Bitmap(_))) {
        select(&mut s, &[b]);
    }
    Some(s)
}

/// Erstes Element der Seite mit passender Art (oben → unten).
fn first_of(s: &Session, f: impl Fn(&ElementKind) -> bool) -> Option<ElementId> {
    top_elements(s).into_iter().find(|e| f(&e.kind)).map(|e| e.id)
}

fn render(s: &Session) -> fp_raster::RgbaImage {
    let d = s.active().expect("Dokument");
    fp_engine::render::default_renderer().expect("Renderer").render_page(&d.doc, d.page, d.frame).expect("Bild")
}

/// Echter Zug wie die Messung: Drücken, 30 Schritte, Loslassen.
fn drag_mods(s: &mut Session, a: (f32, f32), b: (f32, f32), m: impl Fn(ToolEvent) -> ToolEvent) {
    events(s, &[m(ToolEvent::down(a.0, a.1))]);
    for i in 1..=30 {
        let t = i as f32 / 30.0;
        events(s, &[m(ToolEvent::drag(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t))]);
    }
    events(s, &[m(ToolEvent::up(b.0, b.1))]);
}

fn drag(s: &mut Session, a: (f32, f32), b: (f32, f32)) {
    drag_mods(s, a, b, |e| e);
}

fn click(s: &mut Session, x: f32, y: f32) {
    events(s, &[ToolEvent::down(x, y), ToolEvent::up(x, y)]);
}

fn undo_label(s: &Session) -> String {
    s.active().expect("Dokument").history.undo_label().unwrap_or_default().to_string()
}

/// Auswahl als Loch messen wie im Original: Entf, Bild.
fn hole(s: &mut Session) -> fp_raster::RgbaImage {
    ex(s, "edit.clear", json!({}));
    render(s)
}

// ------------------------------------------------------------------ 06 Zauberstab, 07 Farbeimer

/// 06/10: Zauberstab-Toleranz 0/32/100 im Verlauf (Klick 160,120) – Hash wie das Original.
#[test]
fn wand_tolerance_matches_original() {
    for (tol, px, hash) in [(0, 6, "3e810c0f698cbe7c"), (32, 5323, "ba324474f6867728"), (100, 55009, "8434f316b1788c59")] {
        let Some(mut s) = open_par("p57_t02t.fw.png") else { return };
        let base = render(&s);
        s.set_tool(ToolId::MagicWand).expect("Zauberstab");
        s.tools.options.wand_tolerance = tol;
        click(&mut s, 160.5, 120.5);
        assert_eq!(undo_label(&s), "Zauberstab");
        let img = hole(&mut s);
        save_png(&format!("m10_wand{tol}.png"), &img);
        let (n, _) = changed(&img, &base);
        assert_eq!(n, px, "Toleranz {tol}");
        assert_eq!(sha16(&img), hash, "Toleranz {tol}");
    }
}

/// 07/02, 07/05: Farbeimer Rand „Hart“ – Klick ins Auge pixelgleich; Toleranz 32 wählt den
/// Bereich des Zauberstabs; Anti-Alias mischt am Rand (Original 547 px, Näherung ± 5 %),
/// Weiche Kante 10 macht den Rand weich (Original 1 595 px in 99,247–143,291).
#[test]
fn bucket_tolerance_and_edges() {
    let fill = |edge: i64, x: f32, y: f32, tol: u32| -> Option<(fp_raster::RgbaImage, fp_raster::RgbaImage)> {
        let mut s = open_par("p57_t02.fw.png")?;
        let base = render(&s);
        s.set_tool(ToolId::PaintBucket).expect("Farbeimer");
        s.tools.options.fill_tolerance = tol;
        s.tools.options.new_fill = Some(json!({"edge": edge, "feather": 10}));
        click(&mut s, x, y);
        assert_eq!(undo_label(&s), "Füllen");
        Some((render(&s), base))
    };
    let Some((hard, base)) = fill(0, 120.5, 268.5, 32) else { return };
    save_png("f05_edge_Hart.png", &hard);
    assert_eq!(sha16(&hard), "b13660af8d1ce213", "Hart");
    let (aa, _) = fill(1, 120.5, 268.5, 32).expect("AA");
    save_png("f05_edge_AntiAlias.png", &aa);
    let (n, bb) = changed(&aa, &base);
    assert!((520..=575).contains(&n), "Anti-Alias {n}");
    assert_eq!((bb.0, bb.1), (108, 256), "Anti-Alias {bb:?}");
    let (fe, _) = fill(2, 120.5, 268.5, 32).expect("weich");
    save_png("f05_edge_WeicheKte.png", &fe);
    let (n, bb) = changed(&fe, &base);
    assert!((1400..=1800).contains(&n), "weiche Kante {n}");
    assert!((bb.0 - 99).abs() <= 2 && (bb.2 - 143).abs() <= 2, "weiche Kante {bb:?}");
    for (tol, want) in [(0, 7), (32, 5553), (100, 55872)] {
        let (img, _) = fill(1, 160.5, 120.5, tol).expect("Verlauf");
        save_png(&format!("f02_tol{tol}.png"), &img);
        let (n, _) = changed(&img, &base);
        let dev = (n as f64 - want as f64).abs() / want as f64;
        assert!(dev <= 0.05 || (n as i64 - want as i64).abs() <= 2, "Toleranz {tol}: {n} statt {want}");
    }
    // Hart: genau der Bereich des Zauberstabs
    let (img, _) = fill(0, 160.5, 120.5, 32).expect("hart");
    assert_eq!(changed(&img, &base).0, 5323);
}

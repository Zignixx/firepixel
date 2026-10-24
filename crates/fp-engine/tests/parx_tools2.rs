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
use serde_json::json;
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

// ------------------------------------------------------------------ 05 Stempel, Farbe ersetzen, Rote Augen

fn set_pref(s: &mut Session, k: &str, v: fp_engine::prefs::PrefValue) {
    s.prefs.set(k, v);
}

fn replace_setup(s: &mut Session, from: &str, to: &str, tol: i64, colorize: bool, from_image: bool) {
    use fp_engine::prefs::PrefValue::{Bool, Int, Str};
    set_pref(s, "ReplaceColorToolSrcColor", Str(from.into()));
    set_pref(s, "ReplaceColorToolDestColor", Str(to.into()));
    set_pref(s, "ReplaceColorToolTolerance", Int(tol));
    set_pref(s, "ReplaceColorToolStrength", Int(200));
    set_pref(s, "ReplaceColorToolColorize", Bool(colorize));
    set_pref(s, "ReplaceColorToolMode", Bool(from_image));
    s.set_tool(ToolId::ReplaceColor).expect("Farbe ersetzen");
}

/// 05/07: Rot → Blau ersetzt nur den roten Streifen, 13 Zeilen, Ergebnis #0000FE (Kolorieren
/// #0000FF); „Von: Grafik“ ab Start 370 – Hash wie das Original. Schritt „Farbe ersetzen“, Rückgängig.
#[test]
fn replace_color_red_to_blue_matches_original() {
    for (colorize, from_image, start, px, hash) in [
        (false, false, 290.0, 520, "e6c50102d4dc11ad"),
        (true, false, 290.0, 520, "d9d4838d8c7653ad"),
        (false, true, 370.0, 452, "0f8fa34837df8c6e"),
    ] {
        let Some(mut s) = open_par("p57_t02.fw.png") else { return };
        let base = render(&s);
        replace_setup(&mut s, if from_image { "#00FF00" } else { "#FF0000" }, "#0000FF", 32, colorize, from_image);
        drag(&mut s, (start, 120.0), (510.0, 120.0));
        assert_eq!(undo_label(&s), "Farbe ersetzen");
        let img = render(&s);
        save_png(&format!("r07_{}.png", if colorize { "colorize" } else if from_image { "from_image" } else { "red_to_blue" }), &img);
        let (n, bb) = changed(&img, &base);
        assert_eq!(n, px, "Pixel {colorize}/{from_image}");
        assert_eq!(bb.1, 114);
        assert_eq!(bb.3, 127);
        assert_eq!(sha16(&img), hash, "Hash {colorize}/{from_image}");
        ex(&mut s, "edit.undo", json!({}));
        assert_eq!(sha16(&render(&s)), sha16(&base), "Rückgängig");
    }
}

/// 05/08: Toleranz wirkt abgestuft (Von #8E8881, Zug 60 → 270): 0 → wenige Pixel, 32 → ≈ 2 249,
/// 100 → ≈ 2 867 (Näherung ± 12 %).
#[test]
fn replace_color_tolerance_is_graded() {
    let mut last = 0;
    for (tol, want) in [(0, 20), (32, 2249), (100, 2867)] {
        let Some(mut s) = open_par("p57_t02.fw.png") else { return };
        let base = render(&s);
        replace_setup(&mut s, "#8E8881", "#0000FF", tol, false, false);
        drag(&mut s, (60.0, 120.0), (270.0, 120.0));
        let img = render(&s);
        save_png(&format!("r08_tol{tol}.png"), &img);
        let (n, _) = changed(&img, &base);
        if tol == 0 {
            assert!((1..=40).contains(&n), "Toleranz 0: {n}");
        } else {
            let dev = (n as f64 - want as f64).abs() / want as f64;
            assert!(dev <= 0.12, "Toleranz {tol}: {n} statt {want}");
        }
        assert!(n > last, "Toleranz {tol} wählt mehr als die kleinere");
        last = n;
    }
}

fn red_eye(s: &mut Session, tol: i64) {
    use fp_engine::prefs::PrefValue::Int;
    set_pref(s, "RedEyeRemovalToolTolerance", Int(tol));
    set_pref(s, "RedEyeRemovalToolStrength", Int(50));
    s.set_tool(ToolId::RedEyeRemoval).expect("Rote Augen");
}

/// 05/11–12, 05/15: Rote Augen mit aufgezogenem Rechteck über beide Augen: ≈ 1 116 px in
/// 107,255–214,282, Pupillen dunkel; Toleranz 10 = 50; 100 → mehr Pixel, Augen ≈ #1F1F1F.
/// Klick wirkt nicht (kein Schritt), waagrechter Zug: keine Änderung, Schritt „Bitmap bearbeiten“.
#[test]
fn red_eye_rectangle_and_tolerance() {
    let run = |tol: i64, a: (f32, f32), b: (f32, f32)| -> Option<(Session, fp_raster::RgbaImage, fp_raster::RgbaImage)> {
        let mut s = open_par("p57_t02.fw.png")?;
        let base = render(&s);
        red_eye(&mut s, tol);
        drag(&mut s, a, b);
        let img = render(&s);
        Some((s, img, base))
    };
    let Some((s, img50, base)) = run(50, (100.0, 250.0), (225.0, 287.0)) else { return };
    save_png("e11_rect.png", &img50);
    assert_eq!(undo_label(&s), "Rote Augen entfernen");
    let (n, bb) = changed(&img50, &base);
    assert!((1050..=1180).contains(&n), "Rechteck: {n}");
    assert!((bb.0 - 107).abs() <= 1 && (bb.1 - 255).abs() <= 1 && (bb.2 - 214).abs() <= 1 && (bb.3 - 282).abs() <= 1, "{bb:?}");
    let c = img50.get(120, 268).expect("Auge");
    assert!(c[0] < 140 && c[1] < 40, "Pupille dunkel: {c:?}");
    let (_, img10, _) = run(10, (100.0, 250.0), (225.0, 287.0)).expect("10");
    assert_eq!(sha16(&img10), sha16(&img50), "Toleranz 10 = 50");
    let (_, img100, _) = run(100, (100.0, 250.0), (225.0, 287.0)).expect("100");
    save_png("e12_rect_tol100.png", &img100);
    let (n100, _) = changed(&img100, &base);
    assert!(n100 > n && (1100..=1260).contains(&n100), "Toleranz 100: {n100}");
    let c = img100.get(120, 268).expect("Auge");
    assert!(c.iter().take(3).all(|v| (*v as i32 - 31).abs() <= 3), "dunkles Grau: {c:?}");
    // Klick: nichts, kein Schritt
    let mut s = open_par("p57_t02.fw.png").expect("Datei");
    red_eye(&mut s, 50);
    let n0 = s.active().expect("Dokument").history.undo_count();
    click(&mut s, 120.0, 268.0);
    assert_eq!(s.active().expect("Dokument").history.undo_count(), n0, "Klick ohne Schritt");
    assert_eq!(sha16(&render(&s)), sha16(&base));
    // waagrechter Zug (Höhe 0): keine Änderung, Schritt „Bitmap bearbeiten“
    drag(&mut s, (100.0, 268.0), (225.0, 268.0));
    assert_eq!(sha16(&render(&s)), sha16(&base), "Höhe 0 ohne Wirkung");
    assert_eq!(undo_label(&s), "Bitmap bearbeiten");
}

/// 05/03–04, 05/15: Stempel – Alt-Klick setzt die Quelle, beim Ziehen zeigt die Vorschau die echte
/// Kopie (pixelgleich mit dem Ergebnis nach dem Loslassen), Schritt „Stempel“.
#[test]
fn stamp_live_preview_equals_result() {
    let Some(mut s) = open_par("p57_t02.fw.png") else { return };
    let base = render(&s);
    s.set_tool(ToolId::RubberStamp).expect("Stempel");
    events(&mut s, &[ToolEvent::down(120.0, 268.0).alt(), ToolEvent::up(120.0, 268.0).alt()]);
    events(&mut s, &[ToolEvent::down(330.0, 260.0)]);
    for i in 1..=30 {
        events(&mut s, &[ToolEvent::drag(330.0 + 2.0 * i as f32, 260.0)]);
    }
    let preview = s
        .tool_overlay()
        .into_iter()
        .find_map(|o| match o {
            fp_engine::tools::Overlay::Paint(p) => Some(p),
            _ => None,
        })
        .expect("Live-Vorschau beim Ziehen");
    assert!(!preview.tiles.is_empty(), "Kacheln");
    events(&mut s, &[ToolEvent::up(390.0, 260.0)]);
    assert_eq!(undo_label(&s), "Stempel");
    let img = render(&s);
    save_png("s03_aligned.png", &img);
    let (n, bb) = changed(&img, &base);
    assert!((1150..=1400).contains(&n), "Kopie {n} (Original: zwei Züge 2 528)");
    assert!(bb.0 >= 320 && bb.2 <= 402, "{bb:?}");
    // 05/03: Zug 330 → 390 reicht wie im Original bis 322–399 (vorher endete er 2 px früher)
    assert_eq!((bb.0, bb.2), (322, 399), "Zuganfang/-ende {bb:?}");
    // Vorschau = Ergebnis (geänderte Pixel der Vorschau stehen so im Bild)
    let mut checked = 0;
    for t in preview.tiles.iter() {
        let r = t.rect;
        for y in r.top..r.bottom {
            for x in r.left..r.right {
                if let Some(p) = t.image.get(x, y).filter(|p| p[3] > 0) {
                    assert_eq!(img.get(x, y), Some(p), "Vorschau {x},{y}");
                    checked += 1;
                }
            }
        }
    }
    assert!(checked > 1000, "geprüfte Pixel {checked}");
}
// ------------------------------------------------------------------ 06 Auswahlrahmen, Lasso

fn marquee(s: &mut Session) {
    s.set_tool(ToolId::Marquee).expect("Auswahlrahmen");
    s.tools.options.marquee_edge = fp_engine::tools::EdgeMode::Hard;
    s.tools.options.marquee_style = fp_engine::tools::MarqueeStyle::Normal;
}

fn sel_bounds(s: &Session) -> Option<fp_geom::IRect> {
    s.active().and_then(|d| d.selection.pixels.as_ref().map(|m| m.tight_bounds()))
}

/// 06/02, 06/22: Rahmen 100,80–300,220 – Loch hashgleich, Schritt „Pixel auswählen“.
#[test]
fn marquee_draw_and_step_name() {
    let Some(mut s) = open_par("p57_t02t.fw.png") else { return };
    marquee(&mut s);
    drag(&mut s, (100.0, 80.0), (300.0, 220.0));
    assert_eq!(undo_label(&s), "Pixel auswählen");
    assert_eq!(sel_bounds(&s), Some(fp_geom::IRect::new(100, 80, 300, 220)));
    assert_eq!(sha16(&hole(&mut s)), "ab14a172833dcb51");
}

/// 06/03: ohne Rahmen Umschalt = Quadrat, Alt = von der Mitte; mit Rahmen Umschalt+Alt = Schnitt
/// in both orders (RE spec 69 section 4.2 measured Alt first, then Shift, as intersect too; the
/// older reading "Alt first = subtract" is replaced), hashes like the original.
#[test]
fn marquee_shift_alt_like_original() {
    let Some(mut s) = open_par("p57_t02t.fw.png") else { return };
    marquee(&mut s);
    drag_mods(&mut s, (100.0, 80.0), (300.0, 150.0), |e| e.shift());
    assert_eq!(sha16(&hole(&mut s)), "4dabede78d6d3fae", "Quadrat");
    let mut s = open_par("p57_t02t.fw.png").expect("Datei");
    marquee(&mut s);
    drag_mods(&mut s, (200.0, 150.0), (260.0, 190.0), |e| e.alt());
    assert_eq!(sha16(&hole(&mut s)), "7eb9136d3e660763", "von der Mitte");
    for (alt_first, hash) in [(false, "b1588fc512fd1bf1"), (true, "b1588fc512fd1bf1")] {
        let mut s = open_par("p57_t02t.fw.png").expect("Datei");
        marquee(&mut s);
        drag(&mut s, (100.0, 80.0), (200.0, 180.0));
        let first = if alt_first { ToolEvent::drag(150.0, 130.0).alt() } else { ToolEvent::drag(150.0, 130.0).shift() };
        events(&mut s, &[first]);
        drag_mods(&mut s, (150.0, 130.0), (300.0, 250.0), |e| e.shift().alt());
        assert_eq!(sha16(&hole(&mut s)), hash, "Alt zuerst: {alt_first}");
    }
}

/// 06/05, 06/16: Ziehen im Rahmen verschiebt nur den Rahmen; Pfeiltasten ebenso (1 px / 10 px);
/// Strg+Ziehen und Zeiger-Ziehen verschieben die Pixel (Hashes wie das Original), Rückgängig exakt.
#[test]
fn marquee_move_frame_and_pixels() {
    let Some(mut s) = open_par("p57_t02t.fw.png") else { return };
    marquee(&mut s);
    drag(&mut s, (100.0, 80.0), (200.0, 180.0));
    drag(&mut s, (120.0, 100.0), (170.0, 130.0));
    assert_eq!(sel_bounds(&s), Some(fp_geom::IRect::new(150, 110, 250, 210)));
    assert_eq!(sha16(&hole(&mut s)), "e164fd73729191c3", "Rahmen gezogen");
    let mut s = open_par("p57_t02t.fw.png").expect("Datei");
    marquee(&mut s);
    drag(&mut s, (100.0, 80.0), (200.0, 180.0));
    for _ in 0..10 {
        ex(&mut s, "edit.nudge.right", json!({}));
    }
    ex(&mut s, "edit.nudge.bigDown", json!({}));
    assert_eq!(sha16(&hole(&mut s)), "ddd9af986dfcbebe", "Pfeiltasten");
    for pointer in [false, true] {
        let mut s = open_par("p57_t02t.fw.png").expect("Datei");
        let base = render(&s);
        marquee(&mut s);
        drag(&mut s, (100.0, 80.0), (200.0, 180.0));
        if pointer {
            s.set_tool(ToolId::Pointer).expect("Zeiger");
            drag(&mut s, (120.0, 100.0), (170.0, 100.0));
        } else {
            drag_mods(&mut s, (120.0, 100.0), (170.0, 100.0), |e| e.ctrl());
        }
        let img = render(&s);
        save_png(&format!("m16_{}.png", if pointer { "drag_post" } else { "ctrldrag" }), &img);
        assert_eq!(sha16(&img), "5c6a628e1af58f6b", "Pixel verschoben (Zeiger: {pointer})");
        ex(&mut s, "edit.undo", json!({}));
        assert_eq!(sha16(&render(&s)), sha16(&base), "Rückgängig");
    }
}

/// 06/08, 06/18: Esc im offenen Polygon-Lasso beendet den Bitmapmodus, Werkzeug → Zeiger, B1 bleibt
/// gewählt; ein zweites Esc ändert nichts.
#[test]
fn poly_lasso_escape_and_second_escape() {
    let Some(mut s) = open_par("p57_t02t.fw.png") else { return };
    let b1 = first_of(&s, |k| matches!(k, ElementKind::Bitmap(_))).expect("B1");
    s.set_tool(ToolId::PolygonLasso).expect("Polygon-Lasso");
    click(&mut s, 100.0, 80.0);
    click(&mut s, 300.0, 80.0);
    events(&mut s, &[ToolEvent::key(ToolKey::Escape)]);
    assert_eq!(s.tools.active, ToolId::Pointer);
    assert_eq!(s.active().expect("Dokument").selection.elements, vec![b1]);
    events(&mut s, &[ToolEvent::key(ToolKey::Escape)]);
    assert_eq!(s.active().expect("Dokument").selection.elements, vec![b1], "zweites Esc");
}

/// 06/17: „Live-Auswahlrahmen“ – Weiche Kante 10 nachträglich wie beim Aufziehen (Hash wie 06/19).
#[test]
fn live_edge_feathers_existing_marquee() {
    let Some(mut s) = open_par("p57_t02t.fw.png") else { return };
    marquee(&mut s);
    drag(&mut s, (100.0, 80.0), (300.0, 220.0));
    ex(&mut s, "select.liveEdge", json!({"feather": 10}));
    let live = hole(&mut s);
    // gleich wie ein mit Weicher Kante 10 aufgezogener Rahmen (06/19: Zahlen wie das Original)
    let mut t = open_par("p57_t02t.fw.png").expect("Datei");
    marquee(&mut t);
    t.tools.options.marquee_edge = fp_engine::tools::EdgeMode::Feather(10);
    drag(&mut t, (100.0, 80.0), (300.0, 220.0));
    assert_eq!(sha16(&live), sha16(&hole(&mut t)));
    let (n, bb) = changed(&live, &render(&open_par("p57_t02t.fw.png").expect("Datei")));
    assert_eq!((n, bb), (35_088, (90, 70, 310, 230)));
}

// ------------------------------------------------------------------ 07 Verlauf, Pipette, Farbeimer auf Vektoren

/// 07/07, 07/16: Verlauf ohne Rahmen füllt die ganze Leinwand, B1 wächst auf 0,0–800,600 (Hash wie
/// das Original), Schritt „Füllen“.
#[test]
fn gradient_without_marquee_fills_canvas() {
    let Some(mut s) = open_par("p57_t02.fw.png") else { return };
    s.set_tool(ToolId::Gradient).expect("Verlauf");
    drag(&mut s, (100.0, 150.0), (300.0, 150.0));
    assert_eq!(undo_label(&s), "Füllen");
    let img = render(&s);
    save_png("g07_nomarquee.png", &img);
    let b1 = first_of(&s, |k| matches!(k, ElementKind::Bitmap(_))).expect("B1");
    let b = bounds(&s, b1);
    assert_eq!((b.left, b.top, b.right, b.bottom), (0.0, 0.0, 800.0, 600.0));
    let (n, _) = changed(&img, &render(&open_par("p57_t02.fw.png").expect("Datei")));
    assert!((243_000..=245_000).contains(&n), "{n}");
}

/// 07/08, 07/16: Verlauf auf einem Vektor: dritter Griff wie im Original (60,85 → 140,100 ⇒ 75,5),
/// Schritt „Füllung bearbeiten“; Farbeimer auf ein Objekt: Schritt „Füllen“.
#[test]
fn gradient_handles_and_vector_steps() {
    let mut s = doc_session(400.0, 300.0);
    let r1 = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
    select(&mut s, &[r1]);
    ex(&mut s, "element.setGradient", json!({"from": [60, 85], "to": [140, 100]}));
    assert_eq!(undo_label(&s), "Füllung bearbeiten");
    let ElementKind::Path(p) = &el(&s, r1).kind else { panic!("Pfad") };
    assert_eq!((p.fill_handles.second.x, p.fill_handles.second.y), (75.0, 5.0));
    select(&mut s, &[]);
    s.set_tool(ToolId::PaintBucket).expect("Farbeimer");
    click(&mut s, 100.0, 80.0);
    assert_eq!(undo_label(&s), "Füllen");
}

/// 07/11, 07/13: Pipette setzt die Farbe des aktiven Felds, gewählte Vektoren bekommen sie (Schritt
/// „Füllfarbe“/„Strichfarbe“); ein Klick neben die Leinwand ändert nichts.
#[test]
fn eyedropper_respects_active_well_and_selection() {
    let Some(mut s) = open_par("p57_t02.fw.png") else { return };
    select(&mut s, &[]);
    s.set_tool(ToolId::Eyedropper).expect("Pipette");
    click(&mut s, 380.0, 120.0);
    assert_eq!(s.tools.fill_color.to_rgba8().to_hex_rgba()[..7].to_uppercase(), "#FF0000");
    s.tools.options.stroke_well = true;
    click(&mut s, 420.0, 120.0);
    assert_eq!(s.tools.stroke_color.to_rgba8().to_hex_rgba()[..7].to_uppercase(), "#00A000");
    let before = (s.tools.fill_color, s.tools.stroke_color);
    click(&mut s, 1300.0, 400.0);
    assert_eq!((s.tools.fill_color, s.tools.stroke_color), before, "neben der Leinwand");
    s.tools.options.stroke_well = false;
    let r = rect(&mut s, 600.0, 400.0, 700.0, 500.0);
    select(&mut s, &[r]);
    s.set_tool(ToolId::Eyedropper).expect("Pipette");
    click(&mut s, 380.0, 120.0);
    assert_eq!(undo_label(&s), "Füllfarbe");
    let a = fp_engine::docops::path_attrs_of(&el(&s, r)).expect("Attribute");
    assert_eq!(a.fill_color.to_rgba8().to_hex_rgba()[..7].to_uppercase(), "#FF0000");
}
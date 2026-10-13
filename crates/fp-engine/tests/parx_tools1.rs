//! PAR-X-TOOLS-1: Radiergummi, Buntstift und Retusche-Werkzeuge gegen die gemessenen Ergebnisse
//! des Originals (Einheiten 03 `tool-pencil-eraser` und 04 `tool-retouch`, `docs\parity\*.md`).
//!
//! Testbild wie T02: B1 (480 × 320 bei 40,40, deckend) auf weißer Leinwand 800 × 600; die
//! Referenzbilder in `tests\data\parx-tools1\` sind die Exporte des Originals nach demselben Zug
//! (30 Mausschritte, ganzzahlige Punkte). Verglichen wird das Ergebnis über Weiß: geänderte Pixel,
//! Grenzen, Pixel mit mehr als 16 Stufen Abweichung zum Original und die größte Abweichung.
//! Dazu die Logikfälle nach AGENTEN_REGELN §4b (Wiederholung, Rückgängig, Auswahlwechsel, Vektor).

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent, ToolKey};
use fp_geom::IRect;
use fp_model::{ElementId, ElementKind};
use fp_raster::RgbaImage;
use serde_json::{json, Value};
use std::path::PathBuf;

fn data(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("data").join("parx-tools1").join(name)
}

fn load(name: &str) -> RgbaImage {
    let bytes = std::fs::read(data(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
    fp_io::png_in::decode_png(&bytes).expect("PNG").image
}

/// Sitzung mit B1 (aus dem Ausgangsexport geschnitten), ausgewählt.
fn t02() -> (Session, ElementId) {
    let mut s = doc_session(800.0, 600.0);
    let base = load("base_t02.png");
    let r = IRect::new(40, 40, 520, 360);
    let b1 = base.cropped(r);
    s.clipboard.clear();
    s.clipboard.pixels = Some(b1);
    let v = ex(&mut s, "edit.paste", json!({}));
    let id = ElementId(v["ids"][0].as_u64().expect("id"));
    select(&mut s, &[id]);
    (s, id)
}

fn image(s: &Session, id: ElementId) -> RgbaImage {
    match &el(s, id).kind {
        ElementKind::Bitmap(b) => fp_engine::docops::bitmap_image(b).expect("Bild"),
        _ => panic!("keine Bitmap"),
    }
}

/// Bitmap über weißer Leinwand, 800 × 600.
fn flat(img: &RgbaImage) -> Vec<[u8; 3]> {
    let mut out = vec![[255u8; 3]; 800 * 600];
    for y in 0..600 {
        for x in 0..800 {
            if let Some(p) = img.get(x, y) {
                let a = p[3] as u32;
                let m = |c: u8| ((c as u32 * a + 255 * (255 - a) + 127) / 255) as u8;
                out[(y * 800 + x) as usize] = [m(p[0]), m(p[1]), m(p[2])];
            }
        }
    }
    out
}

fn flat_ref(name: &str) -> Vec<[u8; 3]> {
    let r = load(name);
    (0..600).flat_map(|y| (0..800).map(move |x| (x, y))).map(|(x, y)| r.get(x, y).map(|p| [p[0], p[1], p[2]]).unwrap_or([255; 3])).collect()
}

#[derive(Debug)]
struct Cmp {
    changed: usize,
    bbox: (i32, i32, i32, i32),
    over16: usize,
    max: u8,
}

fn compare(res: &[[u8; 3]], refname: &str) -> Cmp {
    let base = flat_ref("base_t02.png");
    let rf = flat_ref(refname);
    let (mut changed, mut over16, mut max) = (0, 0, 0u8);
    let mut bb = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for (i, p) in res.iter().enumerate() {
        let (x, y) = ((i % 800) as i32, (i / 800) as i32);
        if *p != base[i] {
            changed += 1;
            bb = (bb.0.min(x), bb.1.min(y), bb.2.max(x + 1), bb.3.max(y + 1));
        }
        let d = (0..3).map(|k| p[k].abs_diff(rf[i][k])).max().unwrap_or(0);
        if d > 16 {
            over16 += 1;
        }
        max = max.max(d);
    }
    let c = Cmp { changed, bbox: bb, over16, max };
    eprintln!("{refname}: {c:?}");
    c
}

/// Zug wie im Original: 30 Mausschritte, ganzzahlige Punkte.
fn line(a: (f32, f32), b: (f32, f32)) -> Value {
    Value::Array((0..=30).map(|k| json!([(a.0 + (b.0 - a.0) * k as f32 / 30.0).round(), (a.1 + (b.1 - a.1) * k as f32 / 30.0).round()])).collect())
}

fn erase(edge: f64, round: bool, opacity: f64) -> Vec<[u8; 3]> {
    let (mut s, id) = t02();
    ex(&mut s, "bitmap.erase", json!({"points": line((60.0, 150.0), (500.0, 150.0)), "size": 13, "round": round, "edge": edge, "opacity": opacity}));
    flat(&image(&s, id))
}

fn retouch(mode: &str, extra: Value) -> (Session, ElementId) {
    let (mut s, id) = t02();
    let mut p = json!({"mode": mode, "points": line((50.0, 120.0), (510.0, 120.0)), "size": 13, "round": true, "soft": 1.0, "strength": 50, "range": 1});
    if let (Some(o), Some(e)) = (p.as_object_mut(), extra.as_object()) {
        for (k, v) in e {
            o.insert(k.clone(), v.clone());
        }
    }
    ex(&mut s, "bitmap.retouch", p);
    (s, id)
}

fn retouch_flat(mode: &str, extra: Value) -> Vec<[u8; 3]> {
    let (s, id) = retouch(mode, extra);
    flat(&image(&s, id))
}

// ------------------------------------------------------------------ 03 Radiergummi

#[test]
fn eraser_round_edge2_like_original() {
    let c = compare(&erase(2.0, true, 100.0), "e08_t02_round13.png");
    assert_eq!(c.bbox, (53, 143, 507, 157));
    assert!(c.over16 == 0 && c.max <= 16, "{c:?}");
}

#[test]
fn eraser_square_is_hard_with_13_rows() {
    // 03/09: eckig, Rand bleibt ohne Wirkung – pixelgleich mit dem Original
    let c = compare(&erase(2.0, false, 100.0), "e09_t02_square13.png");
    assert_eq!((c.changed, c.bbox, c.max), (5369, (54, 144, 507, 157), 0), "{c:?}");
}

#[test]
fn eraser_edge0_is_hard_and_edge50_soft() {
    // 03/10: Rand 0 hart, pixelgleich
    let c = compare(&erase(0.0, true, 100.0), "e10_t02_edge0.png");
    assert_eq!((c.changed, c.bbox, c.max), (5337, (54, 144, 507, 157), 0), "{c:?}");
    let c = compare(&erase(50.0, true, 100.0), "e10_t02_edge50.png");
    assert_eq!(c.bbox, (53, 143, 507, 157));
    assert!(c.over16 <= 40, "Rand 50 wie Original (vorher 843 Pixel > 16): {c:?}");
}

#[test]
fn eraser_opacity_50_erases_half_once() {
    // 03/11: Deckkraft 50 löscht im ganzen Zug höchstens die Hälfte
    let c = compare(&erase(2.0, true, 50.0), "e11_t02_op50.png");
    assert!(c.over16 == 0 && c.max <= 8, "{c:?}");
    // derselbe Zug zweimal halbiert den Rest noch einmal (zweiter Zug wirkt wie im Original weiter)
    let (mut s, id) = t02();
    for _ in 0..2 {
        ex(&mut s, "bitmap.erase", json!({"points": line((60.0, 150.0), (500.0, 150.0)), "size": 13, "edge": 2, "opacity": 50}));
    }
    let a = image(&s, id).get(200, 150).expect("px")[3];
    assert!((60..=68).contains(&a), "zwei Züge à 50 % → ≈ 25 % Deckung: {a}");
}

// ------------------------------------------------------------------ 03 Buntstift

#[test]
fn pencil_antialias_horizontal_stays_one_row() {
    let (mut s, id) = t02();
    s.set_colors(Some(fp_color::Argb(0xffc0_4040)), Some(fp_color::Argb(0xff80_8080)));
    let mut brush = fp_model::Brush::basic(1);
    brush.antialiased = true;
    let b = serde_json::to_value(&brush).expect("Pinsel");
    ex(&mut s, "bitmap.paint", json!({"tool": "pencil", "brush": b, "color": "#C04040", "trimEnd": false, "points": line((60.0, 150.0), (500.0, 150.0))}));
    ex(&mut s, "bitmap.paint", json!({"tool": "pencil", "brush": b, "color": "#C04040", "trimEnd": false, "points": line((60.0, 220.0), (260.0, 330.0))}));
    let img = image(&s, id);
    // 03/03: Waagrechte als eine volle Zeile #C04040, darüber/darunter unverändert
    for x in [61, 100, 300, 498] {
        assert_eq!(img.get(x, 150).map(|p| [p[0], p[1], p[2]]), Some([0xC0, 0x40, 0x40]), "x {x}");
    }
    let base = load("base_t02.png");
    for y in [149, 151] {
        assert_eq!(img.get(200, y), base.get(200, y), "Zeile {y} unverändert");
    }
    let c = compare(&flat(&img), "p03_t02_pencil_aa.png");
    assert!(c.over16 <= 260, "Diagonale ähnlich dem Original (vorher 1 435 Pixel abweichend): {c:?}");
}

#[test]
fn pencil_without_antialias_unchanged() {
    let (mut s, id) = t02();
    let brush = fp_model::Brush::basic(1);
    let b = serde_json::to_value(&brush).expect("Pinsel");
    for (a, z) in [((60.0, 150.0), (500.0, 150.0)), ((60.0, 220.0), (260.0, 330.0))] {
        ex(&mut s, "bitmap.paint", json!({"tool": "pencil", "brush": b, "color": "#C04040", "trimEnd": false, "points": line(a, z)}));
    }
    let c = compare(&flat(&image(&s, id)), "p02_t02_pencil_noaa.png");
    assert!(c.over16 <= 60, "{c:?}");
}

// ------------------------------------------------------------------ 04 Retusche

#[test]
fn blur_strength_edge_and_shape_like_original() {
    // 04/02: Vorgabe nur an harten Kanten, Verlauf bleibt
    let c = compare(&retouch_flat("blur", json!({})), "b02_blur_default.png");
    assert!(c.over16 == 0 && c.max <= 16 && c.bbox.1 == 113 && c.bbox.3 == 127, "{c:?}");
    // 04/03: Intensität 10 schwach, 100 deutlich stärker (Kanten ±16 zum Original)
    let c = compare(&retouch_flat("blur", json!({"strength": 10})), "b03_blur_int10.png");
    assert!(c.over16 == 0, "{c:?}");
    let c = compare(&retouch_flat("blur", json!({"strength": 100})), "b03_blur_int100.png");
    // das Original trifft eine Kante je nach Mauspunkten ein- oder zweimal (31 bzw. 52 Stufen),
    // Firepixel rechnet anderthalbmal; ±1-Änderungen im Verlauf (Rundung des Originals) fehlen
    assert!(c.over16 <= 5 && c.max <= 20, "{c:?}");
    // 04/13: eckig = hart, 13 Zeilen 114–126
    let c = compare(&retouch_flat("blur", json!({"round": false})), "b13_blur_square.png");
    assert!(c.over16 == 0 && (c.bbox.1, c.bbox.3) == (114, 127), "{c:?}");
    // 04/14: Rand 0 kaum anders als Rand 100 (vorher 348 Pixel, max 61)
    let c = compare(&retouch_flat("blur", json!({"soft": 0.0})), "b14_blur_edge0.png");
    assert!(c.over16 == 0 && (c.bbox.1, c.bbox.3) == (114, 127), "{c:?}");
    // 04/12: zweiter Zug verstärkt
    let (mut s, id) = retouch("blur", json!({}));
    ex(&mut s, "bitmap.retouch", json!({"mode": "blur", "points": line((50.0, 120.0), (510.0, 120.0)), "size": 13, "soft": 1.0, "strength": 50}));
    let c = compare(&flat(&image(&s, id)), "b12_blur_twice.png");
    assert!(c.over16 <= 5 && c.max <= 20, "{c:?}");
}

#[test]
fn sharpen_only_at_edges() {
    // 04/04: 40 Pixel nur an Kanten, max 12 (vorher 1 521 Pixel auch im Verlauf, max 95)
    let c = compare(&retouch_flat("sharpen", json!({})), "b04_sharpen_default.png");
    assert!(c.changed <= 80 && c.over16 == 0 && c.bbox.0 >= 270, "{c:?}");
}

#[test]
fn dodge_and_burn_ranges_like_original_without_color_shift() {
    // 04/06, 04/07: je Bereich wie gemessen; Mitteltöne lassen Schwarz/Weiß/Rot unverändert
    for (mode, k, file) in [
        ("dodge", 0, "b06_dodge_k0_shadows.png"),
        ("dodge", 1, "b06_dodge_k1_midtones.png"),
        ("dodge", 2, "b06_dodge_k2_highlights.png"),
        ("burn", 0, "b07_burn_k0_shadows.png"),
        ("burn", 1, "b07_burn_k1_midtones.png"),
        ("burn", 2, "b07_burn_k2_highlights.png"),
    ] {
        let res = retouch_flat(mode, json!({"range": k}));
        let c = compare(&res, file);
        assert!(c.over16 <= 3 && c.max <= 20, "{mode} {k}: {c:?}");
        let rf = flat_ref(file);
        for x in [150, 300, 340, 380] {
            let (a, b) = (res[120 * 800 + x], rf[120 * 800 + x]);
            assert!((0..3).all(|i| a[i].abs_diff(b[i]) <= 4), "{mode} {k} x {x}: {a:?} statt {b:?}");
        }
    }
}

#[test]
fn smudge_drags_edges_and_keeps_gradient() {
    // 04/09: Kanten wandern in Zugrichtung, der weiche Verlauf bleibt (vorher x 150 → #797996)
    let res = retouch_flat("smudge", json!({"size": 22, "soft": 0.5}));
    let c = compare(&res, "b09_smudge_default.png");
    let base = flat_ref("base_t02.png");
    for x in [100, 150, 200, 250] {
        let (a, b) = (res[120 * 800 + x], base[120 * 800 + x]);
        assert!((0..3).all(|i| a[i].abs_diff(b[i]) <= 1), "Verlauf x {x}: {a:?} statt {b:?}");
    }
    assert!(c.over16 <= 900 && c.bbox.1 == 109 && c.bbox.3 <= 132, "{c:?}");
    let p = |x: usize| res[120 * 800 + x];
    assert!(p(322)[0] < 40 && p(300)[0] > 10 && p(380)[1] > 10, "Kanten mitgezogen: {:?} {:?} {:?}", p(300), p(322), p(380));
}

#[test]
fn smudge_options_take_effect() {
    // 04/10: „Farbe verwischen“ mischt die Farbe am Zuganfang ein
    let plain = retouch_flat("smudge", json!({"size": 22, "soft": 0.5}));
    let col = retouch_flat("smudge", json!({"size": 22, "soft": 0.5, "fingerPaint": true, "color": "#00900D"}));
    let d = (40..120).filter(|x| plain[120 * 800 + x] != col[120 * 800 + x]).count();
    assert!(d > 30, "Farbe wirkt am Anfang: {d}");
    assert!(col[120 * 800 + 50][1] > plain[120 * 800 + 50][1] + 20, "grün eingemischt");
    // 04/11: „Gesamtes Dokument“ nimmt die weiße Leinwand am Bitmaprand auf
    let (mut s, id) = t02();
    let pts = json!([[30, 200], [45, 200], [60, 200], [75, 200], [90, 200]]);
    ex(&mut s, "bitmap.retouch", json!({"mode": "smudge", "points": pts, "size": 22, "soft": 0.5, "strength": 50}));
    let without = image(&s, id);
    let (mut s2, id2) = t02();
    ex(&mut s2, "bitmap.retouch", json!({"mode": "smudge", "points": pts, "size": 22, "soft": 0.5, "strength": 50, "wholeDocument": true}));
    let with = image(&s2, id2);
    assert_ne!(without.get(48, 200), with.get(48, 200), "Leinwand wird mitgezogen");
    let p = with.get(48, 200).expect("px");
    assert!(p[0] > 60 && p[1] > 80, "Weiß der Leinwand eingemischt: {p:?}");
}

#[test]
fn retouch_history_names_per_tool() {
    // 04/18: Name des Verlaufsschritts = Werkzeug
    for (mode, de) in [("blur", "Weichzeichner"), ("sharpen", "Scharfzeichner"), ("dodge", "Abwedler"), ("burn", "Nachbelichter"), ("smudge", "Verwischen")] {
        let (s, _) = retouch(mode, json!({}));
        assert_eq!(s.active().expect("d").history.undo_label(), Some(de), "{mode}");
    }
}

#[test]
fn retouch_on_vector_does_nothing_and_clears_selection() {
    // 04/15: Vektor gewählt → keine Wirkung, keine Meldung, Auswahl leer, kein Schritt
    let mut s = doc_session(400.0, 300.0);
    let r = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
    for t in [ToolId::Blur, ToolId::Sharpen, ToolId::Dodge, ToolId::Burn, ToolId::Smudge] {
        select(&mut s, &[r]);
        let h = s.active().expect("d").history.undo_count();
        let before = (*s.active().expect("d").doc).clone();
        s.set_tool(t).expect("Werkzeug");
        events(&mut s, &[ToolEvent::down(30.0, 85.0), ToolEvent::drag(100.0, 85.0), ToolEvent::up(200.0, 85.0)]);
        let d = s.active().expect("d");
        assert!(d.selection.elements.is_empty(), "{t:?}: Auswahl leer");
        assert_eq!(*d.doc, before, "{t:?}: keine Wirkung");
        assert_eq!(d.history.undo_count(), h, "{t:?}: kein Verlaufsschritt");
    }
}

#[test]
fn escape_after_stroke_ends_bitmap_mode_with_step() {
    // 03/16: Esc nach dem Zug → Schritt „Bitmapmodus beenden“, Zeiger, Bitmap bleibt gewählt
    for t in [ToolId::Eraser, ToolId::Pencil, ToolId::Blur] {
        let (mut s, id) = t02();
        s.set_tool(t).expect("Werkzeug");
        events(&mut s, &[ToolEvent::down(100.0, 150.0), ToolEvent::drag(150.0, 150.0), ToolEvent::up(200.0, 150.0)]);
        let n = s.active().expect("d").history.undo_count();
        events(&mut s, &[ToolEvent::key(ToolKey::Escape)]);
        let d = s.active().expect("d");
        assert_eq!(d.history.undo_count(), n + 1, "{t:?}: ein Schritt");
        assert_eq!(d.history.undo_label(), Some("Bitmapmodus beenden"), "{t:?}");
        assert_eq!(d.selection.elements, vec![id], "{t:?}: Bitmap bleibt gewählt");
        assert_eq!(s.tools.active, ToolId::Pointer);
    }
}

#[test]
fn repeated_and_undo_sequences_are_exact() {
    // §4b: Rückgängig stellt exakt her; A, B, Rückgängig, C, Wiederholen …
    let (mut s, id) = t02();
    let start = image(&s, id);
    ex(&mut s, "bitmap.retouch", json!({"mode": "dodge", "points": line((50.0, 120.0), (510.0, 120.0)), "range": 0}));
    let a = image(&s, id);
    ex(&mut s, "bitmap.erase", json!({"points": line((60.0, 150.0), (500.0, 150.0)), "size": 13, "edge": 0}));
    s.undo().expect("undo");
    assert_eq!(image(&s, id), a);
    ex(&mut s, "bitmap.retouch", json!({"mode": "smudge", "points": line((50.0, 120.0), (510.0, 120.0)), "size": 22}));
    let c = image(&s, id);
    s.undo().expect("undo");
    s.undo().expect("undo");
    assert_eq!(image(&s, id), start, "Ausgangsbild exakt");
    s.redo().expect("redo");
    s.redo().expect("redo");
    assert_eq!(image(&s, id), c);
    // derselbe Zug zweimal hintereinander ist deterministisch (gleicher Auftrag → gleiches Bild)
    let (mut s1, i1) = t02();
    let (mut s2, i2) = t02();
    for s in [&mut s1, &mut s2] {
        ex(s, "bitmap.retouch", json!({"mode": "burn", "points": line((50.0, 120.0), (510.0, 120.0)), "range": 2}));
    }
    assert_eq!(image(&s1, i1), image(&s2, i2));
}

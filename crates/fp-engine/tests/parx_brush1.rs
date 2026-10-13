//! PAR-X-BRUSH-1: Pinsel nach den Messwerten des Originals (Einheiten 01 `tool-brush` und 02
//! `tool-brush-stroke-options`, `docs\parity\tool-brush*.md`). Sollwerte: echte Züge
//! `re\_testdateien\par\px\tool-brush\real_strokes.txt` (Hash = sha256 der RGBA-Pixel, 16 Zeichen),
//! Probestriche und Vorlagen `re\_testdateien\par\px\tool-brush-stroke-options\` (`presets.csv`,
//! `03_*.png`). Ergebnisbilder ohne Oberfläche nach `tests\data\parx-brush1\`. Fehlen die
//! Testdateien (anderer Rechner), werden die Fälle übersprungen.

use crate::common;
use common::*;
use fp_engine::commands::panel::{tool_brush_from, BRUSH_PRESETS};
use fp_engine::paint::{paint_into, PaintJob, PaintTool};
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent, ToolKey};
use fp_model::ElementKind;
use serde_json::{json, Value};
use sha2::Digest;
use std::path::{Path, PathBuf};

fn par_dir() -> PathBuf {
    PathBuf::from(r"C:\Users\Administrator\Desktop\REA\re\_testdateien\par")
}

fn out_dir() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("data").join("parx-brush1");
    let _ = std::fs::create_dir_all(&d);
    d
}

fn load_png(p: &Path) -> Option<fp_raster::RgbaImage> {
    let data = std::fs::read(p).ok()?;
    fp_io::png_in::decode_png(&data).ok().map(|r| r.image)
}

fn save_png(name: &str, img: &fp_raster::RgbaImage) {
    let px: Vec<u32> = img.data().iter().map(|p| (p[3] as u32) << 24 | (p[0] as u32) << 16 | (p[1] as u32) << 8 | p[2] as u32).collect();
    if let Ok(bytes) = fp_export::png_out::write_png_truecolor(&px, img.width(), img.height(), true, &fp_export::png_out::PngOptions::default()) {
        let _ = std::fs::write(out_dir().join(name), bytes);
    }
}

/// sha256 der RGBA-Pixel, 16 Hex-Zeichen (Verfahren von `real_strokes.txt`).
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

/// Arbeitskopie einer PAR-Testdatei öffnen (`None` = Datei fehlt).
fn open_par(name: &str) -> Option<Session> {
    let src = par_dir().join(name);
    if !src.is_file() {
        eprintln!("übersprungen: {src:?} fehlt");
        return None;
    }
    let mut s = doc_session(10.0, 10.0);
    let dir = std::env::temp_dir().join(format!("fp-parx-brush1-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let p = dir.join(format!("{}-{name}", fastrand_id()));
    std::fs::copy(&src, &p).ok()?;
    ex(&mut s, "file.open", json!({"path": p.to_string_lossy()}));
    s.set_colors(Some(fp_color::Argb(0xffc0_4040)), Some(fp_color::Argb(0xff80_8080)));
    Some(s)
}

fn fastrand_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(1);
    N.fetch_add(1, Ordering::Relaxed)
}

fn render(s: &Session) -> fp_raster::RgbaImage {
    let d = s.active().expect("Dokument");
    fp_engine::render::default_renderer().expect("Renderer").render_page(&d.doc, d.page, d.frame).expect("Bild")
}

/// Pinsel-Werkzeugvorgabe wie im Inspektor (Strichart, Größe, Rand der Art).
fn brush(s: &mut Session, preset: &str, size: i32) {
    s.set_tool(ToolId::Brush).expect("Pinsel");
    s.tools.options.brush_size = size as f32;
    let soft = fp_engine::commands::panel::find_preset(preset).map(|p| p.softness / 10).unwrap_or(0);
    s.tools.options.new_stroke = Some(json!({"preset": preset, "width": size, "softness": soft, "placement": "center"}));
    s.tools.options.new_brush = None;
}

/// Echter Zug wie die Messung: Drücken, 30 Schritte, Loslassen (Umschalt optional).
fn drag(s: &mut Session, a: (f32, f32), b: (f32, f32), shift: bool) {
    let sh = |e: ToolEvent| if shift { e.shift() } else { e };
    events(s, &[sh(ToolEvent::down(a.0, a.1))]);
    for i in 1..=30 {
        let t = i as f32 / 30.0;
        events(s, &[sh(ToolEvent::drag(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t))]);
    }
    events(s, &[sh(ToolEvent::up(b.0, b.1))]);
}

fn steps(s: &Session) -> usize {
    s.active().expect("Dokument").history.undo_count()
}

/// 01/03, 01/12, 01/13: echte Züge auf T02 (nichts gewählt) – Hash, Pixelzahl und Grenzen wie
/// `real_strokes.txt` (r02, r03, r12, r13).
#[test]
fn real_strokes_match_original_hashes() {
    let Some(s0) = open_par("par_bitmap.fw.png") else { return };
    let base = render(&s0);
    save_png("t02_base.png", &base);
    assert_eq!(sha16(&base), "bf663a48136ea54c", "Grundbild T02");
    type Case = (&'static str, i32, (f32, f32), (f32, f32), bool, &'static str, usize, (i32, i32, i32, i32));
    let cases: [Case; 5] = [
        ("r03_t02_nosel_d1", 1, (60.0, 150.0), (500.0, 150.0), false, "69d6e5565fd13c78", 441, (60, 150, 501, 151)),
        ("r02_t02_nosel_d10", 10, (60.0, 150.0), (500.0, 150.0), false, "db9c8103846f1a9f", 4480, (56, 146, 506, 156)),
        ("r12_t02_shift_a2_d10", 10, (60.0, 150.0), (300.0, 260.0), true, "7af11207c043e86e", 3680, (56, 146, 306, 396)),
        ("r12_t02_shift_b_d10", 10, (60.0, 150.0), (300.0, 400.0), true, "0ba1ab1841c8c017", 3830, (56, 146, 316, 406)),
        ("r13_t02_click_d20", 20, (150.0, 300.0), (150.0, 300.0), false, "a4f5983df35f2b70", 316, (141, 291, 161, 311)),
    ];
    for (name, size, a, b, shift, sha, n, bbox) in cases {
        let mut s = open_par("par_bitmap.fw.png").expect("T02");
        ex(&mut s, "select.none", json!({}));
        brush(&mut s, "Einfach/Präzise abger.", size);
        if a == b {
            events(&mut s, &[ToolEvent::down(a.0, a.1), ToolEvent::up(b.0, b.1)]);
        } else {
            drag(&mut s, a, b, shift);
        }
        let img = render(&s);
        save_png(&format!("{name}.png"), &img);
        let (cn, cb) = changed(&img, &base);
        assert_eq!((cn, cb), (n, bbox), "{name}: Pixelzahl/Grenzen");
        assert_eq!(sha16(&img), sha, "{name}: Hash");
    }
}

/// 01/05 (r05): B1 gewählt – derselbe Zug malt in B1 (kein neues Objekt), Ergebnis wie r02.
/// Rückgängig stellt das Grundbild her, Wiederholen dasselbe Bild (§4b).
#[test]
fn stroke_into_selected_bitmap_and_undo_redo() {
    let Some(mut s) = open_par("par_bitmap.fw.png") else { return };
    let base = render(&s);
    let b1 = top_elements(&s)[0].id;
    select(&mut s, &[b1]);
    let n_obj = top_elements(&s).len();
    brush(&mut s, "Einfach/Präzise abger.", 10);
    let n0 = steps(&s);
    drag(&mut s, (60.0, 150.0), (500.0, 150.0), false);
    assert_eq!(top_elements(&s).len(), n_obj, "kein neues Objekt");
    assert_eq!(steps(&s), n0 + 1, "ein Schritt");
    let img = render(&s);
    assert_eq!(sha16(&img), "db9c8103846f1a9f", "r05");
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(sha16(&render(&s)), sha16(&base), "Rückgängig = Grundbild");
    ex(&mut s, "edit.redo", json!({}));
    assert_eq!(sha16(&render(&s)), "db9c8103846f1a9f", "Wiederholen");
}

/// 01/14: Esc mitten im Zug malt den Strich trotzdem ganz (r14_esc_mid); Esc danach → Zeiger und
/// Schritt „Bitmapmodus beenden“, B1 bleibt gewählt.
#[test]
fn escape_mid_stroke_and_after() {
    let Some(mut s) = open_par("par_bitmap.fw.png") else { return };
    let b1 = top_elements(&s)[0].id;
    select(&mut s, &[b1]);
    brush(&mut s, "Einfach/Präzise abger.", 10);
    let n0 = steps(&s);
    events(&mut s, &[ToolEvent::down(60.0, 150.0)]);
    for i in 1..=30 {
        events(&mut s, &[ToolEvent::drag(60.0 + 440.0 * i as f32 / 30.0, 150.0)]);
        if i == 15 {
            events(&mut s, &[ToolEvent::key(ToolKey::Escape)]);
        }
    }
    events(&mut s, &[ToolEvent::up(500.0, 150.0)]);
    assert_eq!(steps(&s), n0 + 1);
    assert_eq!(sha16(&render(&s)), "db9c8103846f1a9f", "r14_esc_mid");
    events(&mut s, &[ToolEvent::key(ToolKey::Escape)]);
    assert_eq!(s.tools.active, ToolId::Pointer, "Esc danach: Zeiger");
    assert_eq!(steps(&s), n0 + 2, "Schritt Bitmapmodus beenden");
    let label = s.active().and_then(|d| d.history.undo_label().map(|l| l.to_string())).unwrap_or_default();
    assert_eq!(label, fp_i18n::t("cmd-modify-exitBitmapMode"), "Verlaufsname");
    assert_eq!(s.active().map(|d| d.selection.elements.clone()), Some(vec![b1]), "B1 bleibt gewählt");
    assert_eq!(sha16(&render(&s)), "db9c8103846f1a9f", "Bild unverändert");
    // ohne vorherigen Zug: kein Schritt (FV-11)
    s.set_tool(ToolId::Brush).expect("Pinsel");
    events(&mut s, &[ToolEvent::key(ToolKey::Escape)]);
    assert_eq!(steps(&s), n0 + 2);
}

/// 01/11: T03, BM gewählt, Zug beginnt außerhalb der Bitmap (100,250 → 500,250, Größe 20):
/// „Transparenz beibehalten“ an → nichts gemalt, Auswahl aufgehoben; aus → neues Bitmap
/// 91,241–511,261, BM unverändert.
#[test]
fn stroke_starting_outside_selected_bitmap() {
    for keep in [true, false] {
        let Some(mut s) = open_par("par_bitmap_edge.fw.png") else { return };
        let bm = top_elements(&s)[0].id;
        select(&mut s, &[bm]);
        let before = el(&s, bm);
        brush(&mut s, "Einfach/Präzise abger.", 20);
        s.tools.options.keep_transparency = keep;
        let n_obj = top_elements(&s).len();
        drag(&mut s, (100.0, 250.0), (500.0, 250.0), false);
        assert_eq!(el(&s, bm), before, "BM unverändert (keep {keep})");
        if keep {
            assert_eq!(top_elements(&s).len(), n_obj, "nichts gemalt");
            assert!(s.active().expect("d").selection.elements.is_empty(), "Auswahl aufgehoben");
        } else {
            assert_eq!(top_elements(&s).len(), n_obj + 1, "neues Bitmap");
            let nb = top_elements(&s)[0].clone();
            let ElementKind::Bitmap(b) = &nb.kind else { panic!("Bitmap") };
            let r = fp_engine::docops::bitmap_image(b).expect("Bild").alpha_bounds();
            assert_eq!((r.left, r.top, r.right, r.bottom), (91, 241, 511, 261), "Grenzen wie Original");
            assert_eq!(s.active().expect("d").selection.elements, vec![nb.id], "neues Bitmap gewählt");
        }
    }
}

/// 01/06: T04, Vektor R1 (ohne Strich) gewählt: neues Bitmap mit 1-px-Strich (Inspektor zeigt den
/// Strich des Vektors), R1 unverändert.
#[test]
fn vector_selected_paints_with_its_stroke() {
    let Some(mut s) = open_par("par_vector.fw.png") else { return };
    let r1 = top_elements(&s).iter().rev().find(|e| matches!(e.kind, ElementKind::Path(_))).map(|e| e.id).expect("R1");
    select(&mut s, &[r1]);
    let before = el(&s, r1);
    brush(&mut s, "Einfach/Präzise abger.", 10);
    drag(&mut s, (60.0, 85.0), (140.0, 85.0), false);
    assert_eq!(el(&s, r1), before);
    let nb = top_elements(&s)[0].clone();
    let ElementKind::Bitmap(b) = &nb.kind else { panic!("neues Bitmap") };
    let r = fp_engine::docops::bitmap_image(b).expect("Bild").alpha_bounds();
    assert_eq!(r.height(), 1, "1-px-Strich ({r:?})");
}

/// 02/13: Spitzen 3 / Spitzenabstand 50 spreizt die Spuren (Original Ersatz 31,114–523,177),
/// vorher eine Spur (MINSTD mit Startwert 0).
#[test]
fn tip_spacing_spreads_tracks() {
    let Some(mut s) = open_par("par_bitmap.fw.png") else { return };
    let base = render(&s);
    ex(&mut s, "select.none", json!({}));
    brush(&mut s, "Einfach/Präzise abger.", 20);
    let mut b = tool_brush_from(s.tools.options.new_stroke.as_ref(), None);
    b.brush_type = 0;
    b.tip_count = 3;
    b.tip_spacing = 500;
    ex(&mut s, "pi.tool.brush", json!({"brush": serde_json::to_value(&b).expect("json")}));
    drag(&mut s, (60.0, 150.0), (500.0, 150.0), false);
    let img = render(&s);
    save_png("u02_13_tips3_spacing50.png", &img);
    let (_, (_, t, _, bt)) = changed(&img, &base);
    assert!(bt - t >= 35, "drei gespreizte Spuren: Höhe {}", bt - t);
}

/// Probestrich wie die Ersatzmessung des Originals: Linie 60,150 → 500,150 mit der Vorlage,
/// Größe 20, #C04040, Textur „Vein“ (Ader, Vorgabe des Inspektors), über T02.
fn probe(preset: usize, base: &fp_raster::RgbaImage, seed: u32) -> fp_raster::RgbaImage {
    let b = &BRUSH_PRESETS[preset];
    let brush = tool_brush_from(Some(&json!({"preset": format!("{}/{}", b.category_de, b.name_de), "width": 20})), None);
    let p = json!({"points": [[60, 150], [500, 150]], "brush": brush, "color": "#C04040", "texture": "Vein", "seed": seed});
    let job = PaintJob::from_params(PaintTool::Brush, &p, fp_color::Argb(0xffc0_4040), 20.0);
    paint_into(&job, Some(base), None).map(|i| i.cropped(base.bounds())).unwrap_or_else(|| base.clone())
}

fn csv_rows() -> Vec<std::collections::HashMap<String, String>> {
    let Ok(t) = std::fs::read_to_string(par_dir().join(r"px\tool-brush-stroke-options\presets.csv")) else { return Vec::new() };
    let mut lines = t.lines();
    let head: Vec<String> = lines.next().unwrap_or_default().split(';').map(|s| s.to_string()).collect();
    lines.map(|l| head.iter().cloned().zip(l.split(';').map(|s| s.to_string())).collect()).collect()
}

/// 02/03: alle 54 Stricharten gegen die Probestriche des Originals. Feste Arten: gleiche Pixelzahl
/// und Grenzen (Gestrichelt: Strichmuster gleich, die 1-px-Linie zwischen den Strichen im Bild des
/// Originals gehört nicht zum Pinsel); Zufallsarten statistisch über 8 Startwerte: Pixelzahl ±15 %,
/// Ausdehnung ±30 %, Mittelfarbe je Kanal ± 24.
#[test]
fn stroke_types_match_probe_strokes() {
    let rows = csv_rows();
    let Some(base) = load_png(&par_dir().join(r"px\tool-brush\00_base_none.png")) else { return };
    if rows.is_empty() {
        return;
    }
    let _ = doc_session(10.0, 10.0);
    let mut report = String::from("art;orig_n;fp_n;orig_bbox;fp_bbox;orig_mean;fp_mean\n");
    let mut bad = Vec::new();
    let random = ["Konfetti", "Pelz", "Punkte", "Quadrate", "Farbkleckser", "Garn", "Harke", "Kleckser", "Breiter Kleckser", "Flüssiger Kleckser", "SpaceAge-F.", "Kontur", "Borsten", "Txtur. Borsten", "Fasern", "Giftmüll", "Weich", "Dick"];
    for r in &rows {
        let (cat, name) = (r["category"].as_str(), r["name"].as_str());
        let Some(i) = BRUSH_PRESETS.iter().position(|p| p.category == cat && p.name == name) else {
            bad.push(format!("{name}: Vorlage fehlt"));
            continue;
        };
        let Some(orig) = load_png(&par_dir().join(r"px\tool-brush-stroke-options").join(&r["png_d20"])) else { continue };
        let (on, ob) = changed(&orig, &base);
        let mean = |img: &fp_raster::RgbaImage| -> [f64; 3] {
            let mut acc = [0f64; 3];
            let mut n = 0f64;
            for (p, q) in img.data().iter().zip(base.data()) {
                if p != q {
                    for c in 0..3 {
                        acc[c] += p[c] as f64;
                    }
                    n += 1.0;
                }
            }
            acc.map(|v| v / n.max(1.0))
        };
        let om = mean(&orig);
        let seeds = if random.contains(&r["art"].as_str()) { 8 } else { 1 };
        let (mut fnn, mut fh, mut fw, mut fm) = (0f64, 0f64, 0f64, [0f64; 3]);
        let mut last = (0, (0, 0, 0, 0));
        for k in 0..seeds {
            let img = probe(i, &base, 1234 + k * 7919);
            if k == 0 {
                save_png(&format!("03_{}.png", r["nr"]), &img);
            }
            let (n, b) = changed(&img, &base);
            last = (n, b);
            fnn += n as f64;
            fh += (b.3 - b.1) as f64;
            fw += (b.2 - b.0) as f64;
            let m = mean(&img);
            for c in 0..3 {
                fm[c] += m[c];
            }
        }
        let sf = seeds as f64;
        let (fnn, fh, fw) = (fnn / sf, fh / sf, fw / sf);
        let fm = fm.map(|v| v / sf);
        report.push_str(&format!("{};{};{:.0};{:?};{:?};{:?};{:?}\n", r["art"], on, fnn, ob, last.1, om.map(|v| v as u8), fm.map(|v| v as u8)));
        let dashed = cat == "bc_Dashed";
        if seeds == 1 && !dashed {
            if last != (on, ob) && (fnn - on as f64).abs() > on as f64 * 0.02 {
                bad.push(format!("{}: {:?} statt {:?}", r["art"], last, (on, ob)));
            }
        } else if !dashed {
            let (oh, ow) = ((ob.3 - ob.1) as f64, (ob.2 - ob.0) as f64);
            if (fnn - on as f64).abs() > on as f64 * 0.15 || (fh - oh).abs() > oh * 0.3 + 2.0 || (fw - ow).abs() > ow * 0.3 {
                bad.push(format!("{}: n {fnn:.0}/{on}, h {fh:.0}/{oh}, w {fw:.0}/{ow}", r["art"]));
            }
        }
        if (0..3).any(|c| (fm[c] - om[c]).abs() > 24.0) {
            bad.push(format!("{}: Farbe {:?} statt {:?}", r["art"], fm.map(|v| v as u8), om.map(|v| v as u8)));
        }
    }
    let _ = std::fs::write(out_dir().join("cmp03.csv"), &report);
    assert!(bad.is_empty(), "Abweichungen:\n{}", bad.join("\n"));
}

/// §4b: 50× Strichart/Größe wechseln, dann malen = einmal mit dem Endwert gemalt; Vorschau =
/// Ergebnis bei natürlicher Art (Startwert bleibt über den Zug gleich).
#[test]
fn repeated_changes_and_preview_equals_result() {
    let Some(mut s) = open_par("par_bitmap.fw.png") else { return };
    ex(&mut s, "select.none", json!({}));
    for k in 0..50 {
        brush(&mut s, if k % 2 == 0 { "Unregelmäßig/Punkte" } else { "Einfach/Weich abger." }, 3 + k % 30);
    }
    brush(&mut s, "Einfach/Präzise abger.", 10);
    drag(&mut s, (60.0, 150.0), (500.0, 150.0), false);
    assert_eq!(sha16(&render(&s)), "db9c8103846f1a9f", "wie einmal eingestellt");
    // natürliche Art: Vorschau während des Zugs = fertige Bitmap
    let Some(mut s) = open_par("par_bitmap.fw.png") else { return };
    ex(&mut s, "select.none", json!({}));
    brush(&mut s, "Unregelmäßig/Punkte", 20);
    events(&mut s, &[ToolEvent::down(60.0, 150.0)]);
    for i in 1..=30 {
        events(&mut s, &[ToolEvent::drag(60.0 + 440.0 * i as f32 / 30.0, 150.0 + (i % 3) as f32)]);
    }
    let shown: Vec<fp_engine::paint::PaintPreview> = s.tool_feedback().into_iter().filter_map(|o| if let fp_engine::Overlay::Paint(p) = o { Some(p) } else { None }).collect();
    assert!(!shown.is_empty(), "Vorschau");
    let p = shown[0].clone();
    events(&mut s, &[ToolEvent::up(500.0, 150.0)]);
    let nb = top_elements(&s)[0].clone();
    let ElementKind::Bitmap(b) = &nb.kind else { panic!("Bitmap") };
    let img = fp_engine::docops::bitmap_image(b).expect("Bild");
    let mut diff = 0;
    for t in p.tiles.iter() {
        for y in t.rect.top..t.rect.bottom {
            for x in t.rect.left..t.rect.right {
                let v = t.image.get(x, y).unwrap_or([0; 4]);
                if v[3] != 0 && img.get(x, y).unwrap_or([0; 4]) != v {
                    diff += 1;
                }
            }
        }
    }
    assert_eq!(diff, 0, "Vorschau = Ergebnis");
    let _: Value = json!({});
}

//! Schritte der Gruppe „Überlagerung“ (BATCH-6, SPEC-BATCH §3.6): Text/Wasserzeichen, Logo,
//! Rahmen – Ergebnis-Pixel, Lagen, Kachelung, Platzhalter, Logikfälle (§4b).

mod support;

use fp_batch::item::{still_from_rgba, Animation, Frame, Item, ItemInfo, ItemKind, Meta};
use fp_batch::steps::overlay::{border_image, expand_placeholders, stamp_positions, text_stamp};
use fp_batch::{CancelToken, OutputFormat, Recipe, Severity, StepContext, StepRegistry};
use serde_json::{json, Value};
use std::path::PathBuf;

fn info(w: u32, h: u32) -> ItemInfo {
    ItemInfo {
        path: PathBuf::from("C:/fotos/urlaub/strand.fw.png"),
        root: None,
        kind: ItemKind::Image,
        format: "png".into(),
        bytes: 0,
        width: w,
        height: h,
        frames: 1,
        modified: None,
        exif: None,
    }
}

fn apply_ctx(id: &str, params: Value, item: &mut Item, index: usize, preview: bool, info_size: Option<(u32, u32)>) -> fp_batch::Result<Vec<String>> {
    let reg = StepRegistry::global();
    let step = reg.get(id).expect("step registered");
    let p = fp_batch::step::with_defaults(&step.params(), &params);
    let (w, h) = info_size.unwrap_or_else(|| item.dimensions());
    let inf = info(w, h);
    let cancel = CancelToken::new();
    let mut ctx = StepContext::new(index, 12, &inf, &cancel, OutputFormat::Png);
    ctx.preview = preview;
    step.apply(item, &p, &mut ctx)?;
    Ok(ctx.warnings)
}

fn apply(id: &str, params: Value, item: &mut Item) -> fp_batch::Result<Vec<String>> {
    apply_ctx(id, params, item, 0, false, None)
}

fn gray(w: u32, h: u32) -> Item {
    still_from_rgba(w, h, &[128, 128, 128, 255].repeat((w * h) as usize)).expect("item")
}

fn px(item: &Item) -> (usize, Vec<[u8; 4]>) {
    let i = item.preview_image().expect("image");
    (i.width(), i.data().to_vec())
}

fn changed(item: &Item) -> Vec<(usize, usize)> {
    let (w, d) = px(item);
    d.iter().enumerate().filter(|(_, p)| **p != [128, 128, 128, 255]).map(|(i, _)| (i % w, i / w)).collect()
}

fn logo_b64(w: u32, h: u32, c: [u8; 4]) -> String {
    let it = still_from_rgba(w, h, &c.repeat((w * h) as usize)).expect("logo");
    fp_batch::host::base64(&fp_batch::output::encode(&it, OutputFormat::Png, &Default::default()).expect("png").0)
}

// ------------------------------------------------------------------ Platzhalter

#[test]
fn placeholders_are_expanded() {
    let inf = info(800, 600);
    let cancel = CancelToken::new();
    let ctx = StepContext::new(6, 12, &inf, &cancel, OutputFormat::Png);
    let s = expand_placeholders("(c) {name}.{ext} #{n:3}/{total} {w}x{h} {folder} {unknown}", &ctx, 800, 600);
    assert_eq!(s, "© strand.fw.png #007/12 800x600 urlaub {unknown}");
    let year = chrono::Local::now().format("%Y").to_string();
    assert_eq!(expand_placeholders("{year}", &ctx, 1, 1), year);
    assert_eq!(expand_placeholders("{exif-date}", &ctx, 1, 1).len(), 10, "Rückfall: heute");
    assert_eq!(expand_placeholders("offen {", &ctx, 1, 1), "offen {");
}

// ------------------------------------------------------------------ Lagen

#[test]
fn nine_positions_respect_margin() {
    let p = |pos: &str| json!({"position": pos, "margin": 10});
    // Bild 200×100, Stempel 20×10, Abstand 10 % der kürzeren Seite = 10 px
    let one = |pos: &str| stamp_positions(200, 100, 20, 10, &p(pos))[0];
    assert_eq!(one("topLeft"), (10, 10));
    assert_eq!(one("top"), (90, 10));
    assert_eq!(one("topRight"), (170, 10));
    assert_eq!(one("left"), (10, 45));
    assert_eq!(one("center"), (90, 45));
    assert_eq!(one("right"), (170, 45));
    assert_eq!(one("bottomLeft"), (10, 80));
    assert_eq!(one("bottom"), (90, 80));
    assert_eq!(one("bottomRight"), (170, 80));
    assert_eq!(stamp_positions(200, 100, 20, 10, &json!({"position": "free", "x": 25, "y": 50}))[0], (40, 45));
    // gekachelt: deckt das ganze Bild ab, Abstand wirkt
    let dense = stamp_positions(200, 100, 20, 10, &json!({"position": "tiled", "spacing": 0}));
    let sparse = stamp_positions(200, 100, 20, 10, &json!({"position": "tiled", "spacing": 300}));
    assert!(dense.len() > sparse.len() * 3, "{} {}", dense.len(), sparse.len());
    let mut cover = vec![false; 200 * 100];
    for (x, y) in &dense {
        for yy in *y..y + 10 {
            for xx in *x..x + 20 {
                if (0..200).contains(&xx) && (0..100).contains(&yy) {
                    cover[(yy * 200 + xx) as usize] = true;
                }
            }
        }
    }
    assert!(cover.iter().all(|c| *c), "Kacheln ohne Abstand decken alles");
    assert!(stamp_positions(200, 100, 0, 0, &json!({"position": "tiled"})).is_empty());
}

// ------------------------------------------------------------------ Logo

#[test]
fn logo_scaled_placed_with_opacity_and_blend() {
    let logo = logo_b64(10, 5, [255, 0, 0, 255]);
    let mut it = gray(200, 100);
    apply("overlay.image", json!({"imageData": logo, "scale": 10, "margin": 0, "position": "bottomRight"}), &mut it).expect("apply");
    let ch = changed(&it);
    // 10 % von 200 = 20 px breit, 10 px hoch, unten rechts ohne Abstand
    assert_eq!(ch.len(), 200);
    assert!(ch.iter().all(|(x, y)| *x >= 180 && *y >= 90));
    assert_eq!(px(&it).1[99 * 200 + 199], [255, 0, 0, 255]);
    let mut half = gray(200, 100);
    apply("overlay.image", json!({"imageData": logo, "opacity": 50, "margin": 0}), &mut half).expect("apply");
    let c = px(&half).1[99 * 200 + 199];
    assert!((c[0] as i32 - 191).abs() <= 1 && (c[1] as i32 - 64).abs() <= 1, "50 % Rot über Grau {c:?}");
    let mut mul = gray(200, 100);
    apply("overlay.image", json!({"imageData": logo, "blend": "multiply", "margin": 0}), &mut mul).expect("apply");
    assert_eq!(px(&mul).1[99 * 200 + 199], [128, 0, 0, 255], "multiplizieren");
    // gedreht 90°: 10×20 statt 20×10
    let mut rot = gray(200, 100);
    apply("overlay.image", json!({"imageData": logo, "scale": 10, "rotation": 90, "margin": 0, "position": "topLeft"}), &mut rot).expect("apply");
    let r = changed(&rot);
    let maxx = r.iter().map(|p| p.0).max().unwrap_or(0);
    let maxy = r.iter().map(|p| p.1).max().unwrap_or(0);
    assert!(maxx <= 10 && maxy >= 18, "{maxx} {maxy}");
    // kein Bild → Fehler und Validierungsfehler
    assert!(apply("overlay.image", json!({}), &mut gray(4, 4)).is_err());
    let mut rc = Recipe::named("t");
    rc.add_step("overlay.image", json!({}));
    assert!(rc.validate(&StepRegistry::global()).iter().any(|i| i.code == "missingImage" && i.severity == Severity::Error));
}

#[test]
fn tiled_logo_covers_animation_frames_identically() {
    let logo = logo_b64(4, 4, [0, 0, 255, 255]);
    let frame = || Frame { image: gray(60, 40).preview_image().expect("i"), delay_ms: 50 };
    let mut anim = Item::Animation(Animation { width: 60, height: 40, frames: vec![frame(), frame(), frame()], loop_count: 0, meta: Meta::default() });
    apply("overlay.image", json!({"imageData": logo, "position": "tiled", "scale": 10, "spacing": 100}), &mut anim).expect("apply");
    let Item::Animation(a) = &anim else { panic!("animation") };
    let f0 = a.frames[0].image.data().to_vec();
    assert!(a.frames.iter().all(|f| f.image.data() == &f0[..]), "jedes Frame gleich gestempelt");
    let blue = f0.iter().filter(|p| p[2] == 255 && p[0] == 0).count();
    assert!(blue > 6 * 36, "mehrere Kacheln: {blue}");
}

// ------------------------------------------------------------------ Text

#[test]
fn text_watermark_renders_in_corner_with_opacity() {
    let mut it = gray(400, 200);
    let w = apply("overlay.text", json!({"text": "Firepixel", "size": 8, "color": "#ffffff", "opacity": 100, "position": "bottomRight", "margin": 5}), &mut it).expect("apply");
    let ch = changed(&it);
    if ch.is_empty() {
        // Ohne Systemschriften (z. B. Server ohne Fonts) meldet der Schritt eine Warnung.
        assert!(!w.is_empty(), "weder Text noch Warnung");
        return;
    }
    // unten rechts mit 5 % von 200 = 10 px Abstand
    let maxx = ch.iter().map(|p| p.0).max().unwrap_or(0);
    let maxy = ch.iter().map(|p| p.1).max().unwrap_or(0);
    let minx = ch.iter().map(|p| p.0).min().unwrap_or(0);
    assert!((388..=390).contains(&maxx), "rechter Rand {maxx}");
    assert!((188..=190).contains(&maxy), "unterer Rand {maxy}");
    assert!(minx > 200, "Text sitzt rechts: {minx}");
    // Schriftgröße 8 % von 400 = 32 px → Text etwa 20…40 px hoch
    let miny = ch.iter().map(|p| p.1).min().unwrap_or(0);
    assert!((15..=45).contains(&(maxy - miny)), "Höhe {}", maxy - miny);
    assert!(px(&it).1.contains(&[255, 255, 255, 255]), "volle Deckkraft → reines Weiß");
    // 60 % Deckkraft: nie reines Weiß
    let mut soft = gray(400, 200);
    apply("overlay.text", json!({"text": "Firepixel", "size": 8, "opacity": 60}), &mut soft).expect("apply");
    let (_, d) = px(&soft);
    assert!(d.iter().all(|p| p[0] < 210), "60 %: höchstens 128 + 0,6·127");
    assert!(d.iter().any(|p| p[0] > 190));
}

#[test]
fn text_outline_shadow_rotation_and_tiling() {
    let base = json!({"text": "AB", "size": 10, "opacity": 100, "position": "center", "color": "#ffffff"});
    let Some(plain) = text_stamp("AB", 40.0, &base) else { return };
    let mut o = base.clone();
    o["outline"] = json!(true);
    o["outlineColor"] = json!("#000000");
    let outlined = text_stamp("AB", 40.0, &o).expect("stamp");
    assert!(outlined.width() > plain.width(), "Kontur macht den Stempel breiter");
    assert!(outlined.data().iter().any(|p| p[3] == 255 && p[0] < 30), "schwarze Kontur");
    let mut s = base.clone();
    s["shadow"] = json!(true);
    let shadowed = text_stamp("AB", 40.0, &s).expect("stamp");
    assert!(shadowed.height() > plain.height());
    // Drehung 90° tauscht die Seiten (über den ganzen Schritt geprüft)
    let mut it = gray(300, 300);
    let mut r = base.clone();
    r["rotation"] = json!(90);
    apply("overlay.text", r, &mut it).expect("apply");
    let ch = changed(&it);
    let wx = ch.iter().map(|p| p.0).max().unwrap_or(0) - ch.iter().map(|p| p.0).min().unwrap_or(0);
    let hy = ch.iter().map(|p| p.1).max().unwrap_or(0) - ch.iter().map(|p| p.1).min().unwrap_or(0);
    assert!(hy > wx, "gedreht: höher als breit ({wx}×{hy})");
    // gekachelt: Text überall
    let mut t = gray(300, 300);
    apply("overlay.text", json!({"text": "©", "size": 5, "opacity": 100, "position": "tiled", "spacing": 50}), &mut t).expect("apply");
    let ch = changed(&t);
    for (qx, qy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        assert!(ch.iter().any(|(x, y)| x / 150 == qx && y / 150 == qy), "Viertel {qx},{qy} hat Text");
    }
    // leerer Text: nichts, keine Warnung
    let mut e = gray(10, 10);
    assert!(apply("overlay.text", json!({"text": "  "}), &mut e).expect("apply").is_empty());
    assert!(changed(&e).is_empty());
}

#[test]
fn text_with_counter_differs_per_item_and_repeats_identically() {
    let mut a = gray(200, 80);
    let mut b = gray(200, 80);
    let mut a2 = gray(200, 80);
    let p = json!({"text": "{n:3}", "size": 10, "opacity": 100, "position": "center"});
    apply_ctx("overlay.text", p.clone(), &mut a, 0, false, None).expect("a");
    apply_ctx("overlay.text", p.clone(), &mut b, 7, false, None).expect("b");
    apply_ctx("overlay.text", p, &mut a2, 0, false, None).expect("a2");
    if changed(&a).is_empty() {
        return; // keine Schrift
    }
    assert_ne!(px(&a).1, px(&b).1, "001 ≠ 008");
    assert_eq!(px(&a).1, px(&a2).1, "gleiche Eingabe → gleiches Bild");
}

// ------------------------------------------------------------------ Rahmen

#[test]
fn border_outer_inner_percent_and_rounded() {
    let mut it = gray(20, 10);
    apply("overlay.border", json!({"width": 3, "color": "#ff0000"}), &mut it).expect("apply");
    let (w, d) = px(&it);
    assert_eq!((w, d.len() / w), (26, 16), "außen: Bild wächst um 2 × 3");
    assert_eq!(d[0], [255, 0, 0, 255]);
    assert_eq!(d[3 * 26 + 3], [128, 128, 128, 255]);
    assert_eq!(d[15 * 26 + 25], [255, 0, 0, 255]);
    let mut inner = gray(20, 10);
    apply("overlay.border", json!({"width": 2, "color": "#0000ff", "placement": "inner"}), &mut inner).expect("apply");
    let (w2, d2) = px(&inner);
    assert_eq!((w2, d2.len() / w2), (20, 10));
    assert_eq!(d2[20 + 1], [0, 0, 255, 255]);
    assert_eq!(d2[2 * 20 + 2], [128, 128, 128, 255]);
    // % der kürzeren Seite: 10 % von 10 = 1 px
    let mut pct = gray(20, 10);
    apply("overlay.border", json!({"width": 10, "unit": "percent"}), &mut pct).expect("apply");
    assert_eq!(px(&pct).0, 22);
    // abgerundet: Ecke transparent, Mitte der Kante deckend, Kante geglättet
    let src = gray(40, 40).preview_image().expect("img");
    let r = border_image(&src, &json!({"width": 0, "radius": 25}), 1.0);
    assert_eq!(r.data()[0][3], 0, "Ecke außen transparent");
    assert_eq!(r.data()[20][3], 255, "Kantenmitte deckend");
    assert!(r.data().iter().any(|p| p[3] > 0 && p[3] < 255), "geglättet");
    // Vorschau-Maßstab: Proxy halb so groß wie die Quelle → halbe Rahmenbreite
    let mut prev = gray(20, 10);
    apply_ctx("overlay.border", json!({"width": 4}), &mut prev, 0, true, Some((40, 20))).expect("apply");
    assert_eq!(px(&prev).0, 24);
}

#[test]
fn border_repeated_param_changes_apply_once() {
    let mut r = Recipe::named("t");
    let i = r.add_step("overlay.border", json!({}));
    for k in 0..50 {
        r.set_param(i, "width", json!(k));
    }
    assert_eq!(r.steps.len(), 1);
    let mut it = gray(10, 10);
    apply("overlay.border", r.steps[0].params.clone(), &mut it).expect("apply");
    assert_eq!(px(&it).0, 10 + 2 * 49, "nur der Endwert, einmal");
}

#[test]
fn catalog_lists_batch6_steps_with_labels() {
    let cat = StepRegistry::global().catalog_json();
    let ids: Vec<&str> = cat["steps"].as_array().expect("steps").iter().filter_map(|s| s["id"].as_str()).collect();
    for id in ["background.remove", "background.removeColor", "background.flatten", "background.replace", "background.shadow", "overlay.text", "overlay.image", "overlay.border"] {
        assert!(ids.contains(&id), "{id} fehlt");
    }
    let reg = StepRegistry::global();
    for s in reg.iter().filter(|s| s.id().starts_with("background.") || s.id().starts_with("overlay.")) {
        let key = fp_batch::step::step_key(s.id());
        for lang in ["de", "en"] {
            fp_i18n::set_language(lang);
            assert!(fp_i18n::has(&key), "{lang}: {key}");
            for p in s.params() {
                let k = fp_batch::step::param_key(s.id(), &p.name);
                assert!(fp_i18n::has(&k), "{lang}: {k}");
                if let fp_batch::ParamType::Enum { values } = &p.ty {
                    for v in values {
                        let ck = fp_batch::step::value_key(s.id(), &p.name, v); // BATCH-11a: ein Schema
                        assert!(fp_i18n::has(&ck), "{lang}: {ck}");
                    }
                }
            }
        }
    }
    fp_i18n::set_language("de");
}

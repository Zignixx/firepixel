//! Schritte der Gruppe „Hintergrund“ (BATCH-6, SPEC-BATCH §3.5): Ergebnis-Pixel geprüft, dazu
//! Logikfälle nach AGENTEN_REGELN §4b (Wiederholung, Vorschau = Ergebnis, Abbrechen, Animation,
//! Grenzfälle). Die KI-Fälle stehen in `steps_background_ai.rs` (eigener Prozess, Modellsuche
//! umgestellt).

mod support;

use fp_batch::item::{still_from_rgba, Animation, Frame, Item, ItemInfo, ItemKind, Meta};
use fp_batch::steps::background::{apply_subject_mask, base64_decode, color_key_mask, remove_color, shadow_image};
use fp_batch::{executor, CancelToken, OutputFormat, Recipe, Severity, StepContext, StepRegistry};
use serde_json::{json, Value};
use std::path::PathBuf;
use support::*;

fn info(w: u32, h: u32) -> ItemInfo {
    ItemInfo {
        path: PathBuf::from("C:/fotos/urlaub/bild.png"),
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

/// Wendet einen registrierten Schritt mit aufgefüllten Vorgaben an.
fn apply(id: &str, params: Value, item: &mut Item) -> fp_batch::Result<Vec<String>> {
    let reg = StepRegistry::global();
    let step = reg.get(id).expect("step registered");
    let p = fp_batch::step::with_defaults(&step.params(), &params);
    let (w, h) = item.dimensions();
    let inf = info(w, h);
    let cancel = CancelToken::new();
    let mut ctx = StepContext::new(0, 1, &inf, &cancel, OutputFormat::Png);
    step.apply(item, &p, &mut ctx)?;
    Ok(ctx.warnings)
}

fn img(item: &Item) -> Vec<[u8; 4]> {
    item.preview_image().expect("image").data().to_vec()
}

/// 20×20: weißer Hintergrund, rotes Quadrat 6..14, ein weißes Loch in der Mitte (9..11).
fn white_bg_square() -> Item {
    let mut v = Vec::new();
    for y in 0..20u32 {
        for x in 0..20u32 {
            let sq = (6..14).contains(&x) && (6..14).contains(&y);
            let hole = (9..11).contains(&x) && (9..11).contains(&y);
            v.extend_from_slice(if sq && !hole { &[200, 20, 20, 255] } else { &[255, 255, 255, 255] });
        }
    }
    still_from_rgba(20, 20, &v).expect("item")
}

/// 10×10 rot, mit halbtransparentem Rand (Alpha 128, Farbe mit Weiß vermischt) und
/// transparentem Außenring.
fn fringe_item() -> Item {
    let mut v = Vec::new();
    for y in 0..10u32 {
        for x in 0..10u32 {
            let edge = x == 0 || y == 0 || x == 9 || y == 9;
            let ring = x == 1 || y == 1 || x == 8 || y == 8;
            v.extend_from_slice(if edge {
                &[0, 0, 0, 0]
            } else if ring {
                // halb Rot, halb Weiß (wie auf weißem Grund geglättet), Alpha 50 %
                &[228, 128, 128, 128]
            } else {
                &[200, 0, 0, 255]
            });
        }
    }
    still_from_rgba(10, 10, &v).expect("item")
}

// ------------------------------------------------------------------ Farbe entfernen

#[test]
fn remove_color_contiguous_keeps_enclosed_hole() {
    let mut it = white_bg_square();
    apply("background.removeColor", json!({}), &mut it).expect("apply");
    let p = img(&it);
    assert_eq!(p[0][3], 0, "Ecke entfernt");
    assert_eq!(p[7 * 20 + 7], [200, 20, 20, 255], "Motiv bleibt");
    assert_eq!(p[10 * 20 + 10][3], 255, "eingeschlossenes Loch bleibt (nur von den Rändern aus)");
    let mut all = white_bg_square();
    apply("background.removeColor", json!({"contiguous": false}), &mut all).expect("apply");
    assert_eq!(img(&all)[10 * 20 + 10][3], 0, "ohne Zusammenhang wird auch das Loch entfernt");
}

#[test]
fn remove_color_tolerance_and_custom_color() {
    // Rauschen im Hintergrund: 250 statt 255
    let mut v = Vec::new();
    for i in 0..16u32 {
        v.extend_from_slice(if i == 5 { &[0, 0, 255, 255] } else if i % 2 == 0 { &[250, 250, 250, 255] } else { &[255, 255, 255, 255] });
    }
    let mut strict = still_from_rgba(4, 4, &v).expect("item");
    apply("background.removeColor", json!({"tolerance": 0, "contiguous": false}), &mut strict).expect("apply");
    let removed = img(&strict).iter().filter(|p| p[3] == 0).count();
    assert!(removed < 10, "Toleranz 0 entfernt nur genau die Eckfarbe: {removed}");
    let mut loose = still_from_rgba(4, 4, &v).expect("item");
    apply("background.removeColor", json!({"tolerance": 10, "contiguous": false}), &mut loose).expect("apply");
    assert_eq!(img(&loose).iter().filter(|p| p[3] == 0).count(), 15);
    let mut blue = still_from_rgba(4, 4, &v).expect("item");
    apply("background.removeColor", json!({"pick": "custom", "color": "#0000ff", "tolerance": 0, "contiguous": false}), &mut blue).expect("apply");
    let b = img(&blue);
    assert_eq!(b[5][3], 0);
    assert_eq!(b.iter().filter(|p| p[3] == 0).count(), 1);
}

#[test]
fn remove_color_feather_softens_only_the_kept_side_and_unmixes() {
    let base = white_bg_square();
    let mut it = base.clone();
    let Item::Still(s) = &mut it else { panic!("still") };
    remove_color(&mut s.image, [255, 255, 255, 255], 24, true, 2.0, true);
    let p = s.image.data();
    // Kante des Quadrats wird weich, Mitte bleibt deckend, Hintergrund bleibt weg
    assert!(p[6 * 20 + 10][3] > 0 && p[6 * 20 + 10][3] < 255, "{:?}", p[6 * 20 + 10]);
    assert_eq!(p[0][3], 0);
    // entmischt: weiche Kantenpixel tragen kein Weiß mehr (Rot bleibt dominant)
    let e = p[6 * 20 + 10];
    assert!(e[0] >= 190 && e[1] < 60, "entmischte Kantenfarbe {e:?}");
    // Maske allein
    let m = color_key_mask(&base.preview_image().expect("img"), [255, 255, 255, 255], 0, true);
    assert_eq!(m.iter().filter(|v| **v > 0.5).count(), 400 - 64, "Loch ist eingeschlossen");
}

// ------------------------------------------------------------------ Transparenz → Farbe

#[test]
fn flatten_white_is_exact_and_only_if_transparent() {
    let mut it = fringe_item();
    apply("background.flatten", json!({}), &mut it).expect("apply");
    let p = img(&it);
    assert!(p.iter().all(|q| q[3] == 255), "alles deckend");
    assert_eq!(p[0], [255, 255, 255, 255], "transparent → Weiß");
    assert_eq!(p[5 * 10 + 5], [200, 0, 0, 255], "Motiv unverändert");
    // halbtransparent über Weiß: 228·0,5 + 255·0,5 ≈ 241
    let e = p[10 + 4];
    assert!((e[0] as i32 - 242).abs() <= 1 && (e[1] as i32 - 192).abs() <= 1, "{e:?}");
    // deckendes Bild bleibt mit onlyIfTransparent unverändert (auch bei anderer Farbe)
    let opaque = still_from_rgba(2, 1, &[1, 2, 3, 255, 4, 5, 6, 255]).expect("item");
    let mut o2 = opaque.clone();
    apply("background.flatten", json!({"color": "#00ff00"}), &mut o2).expect("apply");
    assert_eq!(img(&o2), img(&opaque));
}

#[test]
fn flatten_unmix_removes_white_fringe_on_black() {
    let mut normal = fringe_item();
    apply("background.flatten", json!({"color": "#000000"}), &mut normal).expect("apply");
    let mut unmixed = fringe_item();
    apply("background.flatten", json!({"color": "#000000", "edge": "unmix", "matte": "#ffffff"}), &mut unmixed).expect("apply");
    let (n, u) = (img(&normal)[10 + 4], img(&unmixed)[10 + 4]);
    // normal: Grünanteil aus dem weißen Saum sichtbar; entmischt: reines Rot (halbe Helligkeit)
    assert!(n[1] > 50, "Saum sichtbar {n:?}");
    assert!(u[1] < 3 && (u[0] as i32 - 100).abs() <= 2, "Saum entfernt {u:?}");
    let mut hard = fringe_item();
    apply("background.flatten", json!({"color": "#000000", "edge": "hard", "threshold": 200}), &mut hard).expect("apply");
    assert_eq!(img(&hard)[10 + 4], [0, 0, 0, 255], "Alpha 128 < Schwelle 200 → Hintergrund");
}

#[test]
fn flatten_gradient_image_and_blur() {
    let clear = still_from_rgba(10, 10, &[0u8; 400]).expect("item");
    let mut g = clear.clone();
    apply("background.flatten", json!({"kind": "gradient", "color": "#000000", "color2": "#ffffff", "angle": 90}), &mut g).expect("apply");
    let p = img(&g);
    assert!(p[0][0] < 20 && p[90][0] > 235, "oben dunkel, unten hell: {:?} {:?}", p[0], p[90]);
    assert_eq!(p[0][0], p[9][0], "waagerecht gleich");
    let mut lr = clear.clone();
    apply("background.flatten", json!({"kind": "gradient", "color": "#000000", "color2": "#ffffff", "angle": 0}), &mut lr).expect("apply");
    let q = img(&lr);
    assert!(q[0][0] < q[9][0]);
    let mut rad = clear.clone();
    apply("background.flatten", json!({"kind": "gradient", "gradientShape": "radial", "color": "#ffffff", "color2": "#000000"}), &mut rad).expect("apply");
    let r = img(&rad);
    assert!(r[5 * 10 + 5][0] > r[0][0], "Mitte heller als Ecke");
    // eingebettetes Bild (2×1: rot, blau) „strecken“
    let png = {
        let it = still_from_rgba(2, 1, &[255, 0, 0, 255, 0, 0, 255, 255]).expect("item");
        fp_batch::output::encode(&it, OutputFormat::Png, &Default::default()).expect("png").0
    };
    let b64 = fp_batch::host::base64(&png);
    assert_eq!(base64_decode(&b64).as_deref(), Some(&png[..]));
    let mut im = clear.clone();
    apply("background.flatten", json!({"kind": "image", "imageData": b64, "fit": "stretch"}), &mut im).expect("apply");
    let m = img(&im);
    assert!(m[0][0] > 200 && m[0][2] < 50 && m[9][2] > 200 && m[9][0] < 50, "{:?} {:?}", m[0], m[9]);
    // fehlendes Bild → Fehler (kein Absturz), Validierung meldet es
    let mut bad = clear.clone();
    assert!(apply("background.flatten", json!({"kind": "image"}), &mut bad).is_err());
    let mut rc = Recipe::named("t");
    rc.add_step("background.flatten", json!({"kind": "image"}));
    assert!(rc.validate(&StepRegistry::global()).iter().any(|i| i.code == "missingImage" && i.severity == Severity::Error));
    // unscharfe Kopie: transparente Pixel bekommen Farbe aus der Umgebung
    let mut half = Vec::new();
    for i in 0..100 {
        half.extend_from_slice(if i % 10 < 5 { &[0, 0, 255, 255] } else { &[0, 0, 0, 0] });
    }
    let mut bl = still_from_rgba(10, 10, &half).expect("item");
    apply("background.flatten", json!({"kind": "blur", "blur": 6, "color": "#ffffff"}), &mut bl).expect("apply");
    let z = img(&bl);
    assert!(z[5][2] > z[5][0], "neben dem Blau bläulich: {:?}", z[5]);
    assert!(z.iter().all(|p| p[3] == 255));
}

#[test]
fn flatten_linear_light_differs_from_srgb() {
    let mut a = fringe_item();
    let mut b = fringe_item();
    apply("background.flatten", json!({"color": "#000000"}), &mut a).expect("apply");
    apply("background.flatten", json!({"color": "#000000", "linearLight": true}), &mut b).expect("apply");
    assert!(img(&b)[10 + 4][0] > img(&a)[10 + 4][0], "linear: hellere Mischung");
}

// ------------------------------------------------------------------ Ersetzen, Maske, Schatten

#[test]
fn replace_with_color_method_puts_new_background_behind_subject() {
    let mut it = white_bg_square();
    apply("background.replace", json!({"method": "color", "kind": "color", "color": "#00ff00"}), &mut it).expect("apply");
    let p = img(&it);
    assert_eq!(p[0], [0, 255, 0, 255]);
    assert_eq!(p[7 * 20 + 7], [200, 20, 20, 255]);
    assert_eq!(p[10 * 20 + 10], [255, 255, 255, 255], "eingeschlossenes Weiß bleibt");
    // unscharfe Kopie nutzt das Bild vor dem Entfernen
    let mut b = white_bg_square();
    apply("background.replace", json!({"method": "color", "kind": "blur", "blur": 4}), &mut b).expect("apply");
    assert!(img(&b).iter().all(|q| q[3] == 255));
}

#[test]
fn subject_mask_outputs() {
    let base = white_bg_square().preview_image().expect("img");
    let mask: Vec<f32> = (0..400).map(|i| if (6..14).contains(&(i % 20)) && (6..14).contains(&(i / 20)) { 1.0 } else if i % 20 == 5 { 0.5 } else { 0.0 }).collect();
    let mut t = base.clone();
    apply_subject_mask(&mut t, &mask, "transparent", false);
    assert_eq!(t.data()[0][3], 0);
    assert_eq!(t.data()[7 * 20 + 7], [200, 20, 20, 255]);
    assert_eq!(t.data()[7 * 20 + 5][3], 128);
    let mut m = base.clone();
    apply_subject_mask(&mut m, &mask, "mask", false);
    assert_eq!(m.data()[0], [0, 0, 0, 255]);
    assert_eq!(m.data()[7 * 20 + 7], [255, 255, 255, 255]);
    let mut a = base.clone();
    apply_subject_mask(&mut a, &mask, "alpha", true);
    assert_eq!(a.data()[0][3], 0, "Maske 0 → transparent");
    assert_eq!(a.data()[7 * 20 + 5], [255, 255, 255, 128], "Farbe bleibt, Alpha = Maske");
    // Entmischen mit geschätztem Hintergrund (Weiß): Kantenpixel 50 % Rot/Weiß → Rot
    let mut mixed = base.clone();
    mixed.data_mut()[7 * 20 + 5] = [228, 138, 138, 255];
    let mut d = mixed.clone();
    apply_subject_mask(&mut d, &mask, "transparent", true);
    let e = d.data()[7 * 20 + 5];
    assert!(e[0] >= 198 && e[1] <= 25 && e[3] == 128, "entmischt {e:?}");
}

#[test]
fn shadow_grows_canvas_and_falls_down_right() {
    let mut v = vec![0u8; 20 * 20 * 4];
    for y in 5..15 {
        for x in 5..15 {
            v[(y * 20 + x) * 4..(y * 20 + x) * 4 + 4].copy_from_slice(&[255, 255, 255, 255]);
        }
    }
    let src = still_from_rgba(20, 20, &v).expect("item").preview_image().expect("img");
    let p = json!({"distance": 4, "angle": 315, "blur": 0, "opacity": 100, "expandCanvas": true});
    let out = shadow_image(&src, &p, 1.0);
    // Versatz 4 px bei 315° = (3, 3); Reichweite 1 px (blur 0) → links/oben 0, rechts/unten 1 + 3
    assert_eq!((out.width(), out.height()), (24, 24));
    let at = |x: usize, y: usize| out.data()[y * out.width() + x];
    assert_eq!(at(10, 10), [255, 255, 255, 255], "Motiv oben drauf");
    assert_eq!(at(17, 17), [0, 0, 0, 255], "Schatten rechts unten");
    assert_eq!(at(3, 3)[3], 0, "links oben kein Schatten");
    let flat = shadow_image(&src, &json!({"expandCanvas": false}), 1.0);
    assert_eq!((flat.width(), flat.height()), (20, 20));
    let below = shadow_image(&src, &json!({"style": "below", "blur": 2}), 1.0);
    assert!(below.height() > 20);
    let under = below.data()[15 * below.width() + 10];
    assert!(under[3] > 0, "Bodenschatten unter dem Motiv {under:?}");
}

// ------------------------------------------------------------------ Logikfälle §4b

#[test]
fn repeated_param_changes_give_one_effect() {
    let mut r = Recipe::named("t");
    let i = r.add_step("background.flatten", json!({}));
    for k in 0..50 {
        r.set_param(i, "color", json!(format!("#{:02x}0000", k * 5)));
    }
    r.set_param(i, "color", json!("#0000ff"));
    assert_eq!(r.steps.len(), 1);
    let mut it = fringe_item();
    let p = r.steps[0].params.clone();
    apply("background.flatten", p, &mut it).expect("apply");
    assert_eq!(img(&it)[0], [0, 0, 255, 255], "Endwert gilt");
    // Schritt zweimal hintereinander auf dasselbe Bild = einmal (Füllen ist idempotent)
    let once = img(&it);
    apply("background.flatten", json!({"color": "#0000ff"}), &mut it).expect("apply");
    assert_eq!(img(&it), once);
    // Farbe entfernen zweimal = einmal
    let mut a = white_bg_square();
    apply("background.removeColor", json!({}), &mut a).expect("apply");
    let first = img(&a);
    apply("background.removeColor", json!({}), &mut a).expect("apply");
    assert_eq!(img(&a), first);
}

#[test]
fn animation_frames_all_processed_and_cancel_stops() {
    let frame = |c: u8| Frame { image: still_from_rgba(4, 4, &[c, 0, 0, 0].repeat(16)).expect("f").preview_image().expect("i"), delay_ms: 100 };
    let mut anim = Item::Animation(Animation { width: 4, height: 4, frames: vec![frame(1), frame(2), frame(3)], loop_count: 0, meta: Meta::default() });
    apply("background.flatten", json!({"color": "#ff00ff"}), &mut anim).expect("apply");
    let Item::Animation(a) = &anim else { panic!("animation") };
    assert!(a.frames.iter().all(|f| f.image.data().iter().all(|p| *p == [255, 0, 255, 255])));
    // Abbrechen: Fehler „abgebrochen“, kein Teilergebnis zählt
    let reg = StepRegistry::global();
    let step = reg.get("background.shadow").expect("step");
    let inf = info(4, 4);
    let cancel = CancelToken::new();
    cancel.cancel();
    let mut ctx = StepContext::new(0, 1, &inf, &cancel, OutputFormat::Png);
    let p = fp_batch::step::with_defaults(&step.params(), &json!({}));
    let err = step.apply(&mut anim, &p, &mut ctx).expect_err("cancelled");
    assert!(err.is_cancelled());
}

#[test]
fn edge_cases_one_pixel_and_fully_transparent() {
    let mut one = still_from_rgba(1, 1, &[255, 255, 255, 255]).expect("item");
    apply("background.removeColor", json!({}), &mut one).expect("apply");
    assert_eq!(img(&one)[0][3], 0);
    let mut clear = still_from_rgba(3, 3, &[0u8; 36]).expect("item");
    apply("background.shadow", json!({}), &mut clear).expect("apply");
    assert!(img(&clear).iter().all(|p| p[3] == 0), "kein Motiv → kein Schatten");
    for id in ["background.flatten", "background.replace"] {
        let mut c = still_from_rgba(3, 3, &[0u8; 36]).expect("item");
        apply(id, json!({"method": "color"}), &mut c).expect(id);
        assert!(img(&c).iter().all(|p| p[3] == 255), "{id}");
    }
}

#[test]
fn run_writes_flattened_jpeg_and_preview_matches() {
    let dir = temp("bg-run");
    let src = dir.join("in").join("logo.png");
    std::fs::create_dir_all(src.parent().expect("p")).expect("dir");
    let it = fringe_item();
    std::fs::write(&src, fp_batch::output::encode(&it, OutputFormat::Png, &Default::default()).expect("png").0).expect("write");
    let mut r = Recipe::named("t");
    r.output.format = "png".into();
    r.output.target = fp_batch::recipe::TargetSpec { kind: fp_batch::recipe::TargetKind::Folder, path: Some(dir.join("out")), ..Default::default() };
    r.add_step("background.flatten", json!({"color": "#336699"}));
    r.add_step("overlay.border", json!({"width": 2, "color": "#000000"}));
    let items = fp_batch::collect(std::slice::from_ref(&src), false, false, &[]).items;
    let rep = executor::run(&r, &items, &fp_batch::RunOptions::default()).expect("run");
    assert_eq!(rep.items.len(), 1);
    let out = dir.join("out").join("logo.png");
    assert_eq!(dims(&out), (14, 14));
    let px = pixels(&out);
    assert_eq!(&px[0..4], &[0, 0, 0, 255]);
    assert_eq!(&px[(2 * 14 + 2) * 4..(2 * 14 + 2) * 4 + 4], &[0x33, 0x66, 0x99, 255]);
    // Vorschau (Bild klein genug → kein Proxy) = Ergebnis
    let pv = fp_batch::preview(&r, &items[0], &StepRegistry::global(), &fp_batch::PreviewOptions::default()).expect("preview");
    assert_eq!(pv.after.expect("after").as_bytes(), &px[..]);
    let _ = std::fs::remove_dir_all(&dir);
}

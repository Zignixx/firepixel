//! Golden-Fälle der Gruppen „Hintergrund“ und „Überlagerung“ (BATCH-6, SPEC-BATCH §5.10):
//! `tools\golden\batch\<fall>\{in,expected}\` + `recipe.json`; der Lauf geht über
//! `executor::run` (wie CLI/Oberfläche) in einen Temp-Ordner und wird mit Toleranz verglichen
//! (größte Kanalabweichung 2). Text-Wasserzeichen hat keinen Golden-Fall (hängt von den
//! Systemschriften ab), sondern Logiktests in `steps_overlay.rs`.
//!
//! Neu erzeugen (Eingaben, Rezept, Erwartung): `$env:FP_GOLDEN_UPDATE='1'; cargo test -p fp-batch
//! --test golden_bg` – danach die Bilder ansehen und mitcommitten.

mod support;

use fp_batch::item::still_from_rgba;
use fp_batch::{executor, OutputFormat, Recipe, RunOptions};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use support::*;

fn golden_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("tools").join("golden").join("batch")
}

fn update() -> bool {
    std::env::var("FP_GOLDEN_UPDATE").is_ok_and(|v| v == "1")
}

fn png(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
    let mut v = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            v.extend_from_slice(&f(x, y));
        }
    }
    let item = still_from_rgba(w, h, &v).expect("item");
    fp_batch::output::encode(&item, OutputFormat::Png, &Default::default()).expect("png").0
}

/// Kreis mit geglättetem Rand, Farbe `c` auf Hintergrund `bg` (`None` = transparent, mit weißem
/// Saum wie ein auf Weiß freigestelltes Logo).
fn disc(w: u32, h: u32, c: [u8; 3], bg: Option<[u8; 3]>) -> Vec<u8> {
    png(w, h, |x, y| {
        let d = ((x as f32 + 0.5 - w as f32 / 2.0).powi(2) + (y as f32 + 0.5 - h as f32 / 2.0).powi(2)).sqrt();
        let a = (h as f32 * 0.35 + 0.5 - d).clamp(0.0, 1.0);
        match bg {
            Some(b) => {
                let m = |i: usize| (c[i] as f32 * a + b[i] as f32 * (1.0 - a)).round() as u8;
                [m(0), m(1), m(2), 255]
            }
            None => {
                // Saum: Farbe zur Hälfte mit Weiß vermischt, Alpha = Deckung
                let m = |i: usize| if a < 1.0 { ((c[i] as f32 + 255.0) / 2.0).round() as u8 } else { c[i] };
                [m(0), m(1), m(2), (a * 255.0).round() as u8]
            }
        }
    })
}

fn photo(w: u32, h: u32) -> Vec<u8> {
    png(w, h, |x, y| [(40 + x * 180 / w) as u8, (60 + y * 150 / h) as u8, 150, 255])
}

/// Ein Fall: Name, Eingabedateien (Name, PNG), Schritte.
type Case = (&'static str, Vec<(&'static str, Vec<u8>)>, Value);

fn cases() -> Vec<Case> {
    let logo = png(12, 6, |x, _| if x < 6 { [255, 255, 255, 255] } else { [220, 30, 30, 255] });
    vec![
        ("bg-flatten-unmix", vec![("logo.png", disc(48, 40, [30, 90, 200], None))], json!([{"id": "background.flatten", "params": {"color": "#000000", "edge": "unmix", "matte": "#ffffff"}}])),
        ("bg-remove-color", vec![("product.png", disc(48, 40, [200, 40, 40], Some([255, 255, 255])))], json!([{"id": "background.removeColor", "params": {"tolerance": 16, "feather": 1}}])),
        (
            "bg-replace-gradient",
            vec![("product.png", disc(48, 40, [200, 40, 40], Some([255, 255, 255])))],
            json!([{"id": "background.replace", "params": {"method": "color", "kind": "gradient", "gradientShape": "radial", "color": "#ffffff", "color2": "#3060a0"}}]),
        ),
        ("bg-shadow", vec![("logo.png", disc(48, 40, [30, 90, 200], None))], json!([{"id": "background.shadow", "params": {"distance": 4, "blur": 3, "opacity": 60}}])),
        ("overlay-border-round", vec![("photo.png", photo(60, 40))], json!([{"id": "overlay.border", "params": {"width": 5, "unit": "percent", "color": "#f0f0f0", "radius": 12}}])),
        (
            "overlay-image-tiled",
            vec![("photo.png", photo(60, 40))],
            json!([{"id": "overlay.image", "params": {"imageData": fp_batch::host::base64(&logo), "position": "tiled", "scale": 20, "opacity": 50, "rotation": -30, "spacing": 60}}]),
        ),
    ]
}

fn max_diff(a: &[u8], b: &[u8]) -> u8 {
    a.iter().zip(b).map(|(x, y)| x.abs_diff(*y)).max().unwrap_or(0)
}

#[test]
fn golden_background_and_overlay() {
    let root = golden_root();
    let tmp = temp("golden-bg");
    let mut checked = 0;
    for (name, inputs, steps) in cases() {
        let case = root.join(name);
        let (inp, exp) = (case.join("in"), case.join("expected"));
        if update() {
            std::fs::create_dir_all(&inp).expect("in");
            std::fs::create_dir_all(&exp).expect("expected");
            for (f, bytes) in &inputs {
                std::fs::write(inp.join(f), bytes).expect("write in");
            }
            let recipe = json!({"format": "firepixel-batch-recipe", "version": 1, "name": name, "steps": steps, "output": {"format": "png"}});
            std::fs::write(case.join("recipe.json"), serde_json::to_string_pretty(&recipe).expect("json") + "\n").expect("recipe");
        }
        let mut r = Recipe::from_json(&std::fs::read_to_string(case.join("recipe.json")).unwrap_or_else(|e| panic!("{name}: recipe.json fehlt ({e}); FP_GOLDEN_UPDATE=1"))).expect("recipe");
        let out = tmp.join(name);
        r.output.target = fp_batch::recipe::TargetSpec { kind: fp_batch::recipe::TargetKind::Folder, path: Some(out.clone()), ..Default::default() };
        let items = fp_batch::collect(std::slice::from_ref(&inp), false, false, &[]).items;
        assert_eq!(items.len(), inputs.len(), "{name}: Eingaben");
        let rep = executor::run(&r, &items, &RunOptions::default()).expect("run");
        assert!(rep.items.iter().all(|i| i.status == fp_batch::ItemStatus::Done), "{name}: {:?}", rep.items);
        for (f, _) in &inputs {
            let got = out.join(f);
            let want = exp.join(f);
            if update() {
                std::fs::copy(&got, &want).expect("update expected");
            }
            assert_eq!(dims(&got), dims(&want), "{name}/{f}: Größe");
            let d = max_diff(&pixels(&got), &pixels(&want));
            assert!(d <= 2, "{name}/{f}: Kanalabweichung {d} > 2");
            checked += 1;
        }
    }
    assert_eq!(checked, 6);
    let _ = std::fs::remove_dir_all(&tmp);
}

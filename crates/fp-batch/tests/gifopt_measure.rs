//! Messung des GIF-Optimierers (SPEC-BATCH §5.6 Punkt 7) gegen den bisherigen GIF-Export
//! (`fp-export`, Pfad der Ausgabe ohne eingehängten Kodierer) auf selbst erzeugten Testanimationen.
//! Eigener Testprozess: Der eingebaute Kodierer ist nur erreichbar, solange das Schrittregister (das
//! den Optimierer einhängt) in diesem Prozess noch nicht gebaut wurde – deshalb genau ein Test.
//!
//! Aufruf: `cargo test -p fp-batch --test gifopt_measure -- --ignored --nocapture`

mod gifopt_clips;

use fp_batch::steps::gifopt;
use gifopt_clips::*;
use serde_json::{json, Map, Value};

fn opts(v: Value) -> Map<String, Value> {
    v.as_object().cloned().unwrap_or_default()
}

#[test]
#[ignore = "Messung für den Bericht (langsam, gibt eine Tabelle aus)"]
fn measure_against_plain_export() {
    let clips = [screen(320, 240, 24), pan(320, 240, 24), cartoon(320, 240, 24)];
    // 1. bisheriger Export (vor dem ersten Zugriff auf das Register!)
    let baseline: Vec<Vec<u8>> = clips
        .iter()
        .map(|c| fp_batch::output::encode(&c.item(), fp_batch::OutputFormat::Gif, &Map::new()).expect("baseline").0)
        .collect();
    let variants: [(&str, Value); 5] = [
        ("simple", json!({"palette": "global", "dither": "floydSteinberg", "ditherAmount": 80, "lossy": 0, "diffTransparency": false, "cropChanged": false, "mergeSimilar": false, "diffThreshold": 0, "quality": 100})),
        ("best", json!({"preset": "best"})),
        ("balanced", json!({"preset": "balanced"})),
        ("smallest", json!({"preset": "smallest"})),
        ("maxKB=60%", Value::Null),
    ];
    println!("| Clip | Encoder | KB | vs. Export | SSIM | PSNR dB | ms |");
    println!("|---|---|---:|---:|---:|---:|---:|");
    for (clip, base) in clips.iter().zip(&baseline) {
        let row = |name: &str, bytes: &[u8], ms: u128| {
            println!(
                "| {} | {} | {:.1} | {:+.0} % | {:.4} | {:.2} | {} |",
                clip.name,
                name,
                bytes.len() as f64 / 1024.0,
                (bytes.len() as f64 / base.len() as f64 - 1.0) * 100.0,
                mean_ssim(clip, bytes),
                mean_psnr(clip, bytes),
                ms
            );
        };
        row("Export (bisher)", base, 0);
        if let Some(d) = std::env::var_os("FP_DUMP") {
            let _ = std::fs::write(std::path::Path::new(&d).join(format!("{}-export.gif", clip.name)), base);
        }
        for (name, v) in &variants {
            let v = if v.is_null() { json!({"maxKB": (base.len() * 6 / 10 / 1024).max(1)}) } else { v.clone() };
            let t = std::time::Instant::now();
            let r = gifopt::optimize(&clip.item(), &opts(v)).expect("optimize");
            row(name, &r.bytes, t.elapsed().as_millis());
            if let Some(d) = std::env::var_os("FP_DUMP") {
                let _ = std::fs::write(std::path::Path::new(&d).join(format!("{}-{}.gif", clip.name, name.replace(['=', '%'], ""))), &r.bytes);
            }
            if name.starts_with("maxKB") {
                println!("|  | ↳ Qualität {} · Faktor {:.2} · Schritt {} · {} Kodierungen · erreicht {} | | | | | |", r.quality, r.scale, r.frame_step, r.encodes, r.reached);
            }
        }
        // gleiche Qualität wie der Export: kleinste Qualitätsstufe mit SSIM ≥ SSIM des Exports
        let target = mean_ssim(clip, base);
        let (mut lo, mut hi) = (10u8, 100u8);
        let mut found: Option<(u8, Vec<u8>)> = None;
        while lo <= hi {
            let mid = lo + (hi - lo) / 2;
            let r = gifopt::optimize(&clip.item(), &opts(json!({"quality": mid}))).expect("optimize");
            if mean_ssim(clip, &r.bytes) >= target {
                found = Some((mid, r.bytes));
                hi = mid - 1;
            } else {
                lo = mid + 1;
            }
        }
        if let Some((q, bytes)) = found {
            row(&format!("gleiche SSIM (Qualität {q})"), &bytes, 0);
        }
    }
}
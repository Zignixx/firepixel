//! Messung des GIF-Optimierers (SPEC-BATCH §5.6 Punkt 7) gegen den bisherigen GIF-Export
//! (`fp-export`, Pfad der Ausgabe ohne eingehängten Kodierer) auf selbst erzeugten Testanimationen.
//! Eigener Testprozess: Der eingebaute Kodierer ist nur erreichbar, solange das Schrittregister (das
//! den Optimierer einhängt) in diesem Prozess noch nicht gebaut wurde – deshalb genau ein Test.
//!
//! Aufruf: `cargo test -p fp-batch --test gifopt_measure -- --ignored --nocapture`

mod gifopt_clips;

use fp_batch::steps::gifopt;
use gifopt_clips::*;
use serde_json::{Map, Value, json};

fn opts(v: Value) -> Map<String, Value> {
    v.as_object().cloned().unwrap_or_default()
}

#[test]
#[ignore = "Messung für den Bericht (langsam, gibt eine Tabelle aus)"]
fn measure_against_plain_export() {
    let clips = [
        screen(320, 240, 24),
        pan(320, 240, 24),
        cartoon(320, 240, 24),
    ];
    // 1. bisheriger Export (vor dem ersten Zugriff auf das Register!)
    let baseline: Vec<Vec<u8>> = clips
        .iter()
        .map(|c| {
            fp_batch::output::encode(&c.item(), fp_batch::OutputFormat::Gif, &Map::new())
                .expect("baseline")
                .0
        })
        .collect();
    let variants: [(&str, Value); 5] = [
        (
            "simple",
            json!({"palette": "global", "dither": "floydSteinberg", "ditherAmount": 80, "lossy": 0, "diffTransparency": false, "cropChanged": false, "mergeSimilar": false, "diffThreshold": 0, "quality": 100}),
        ),
        ("best", json!({"preset": "best"})),
        ("balanced", json!({"preset": "balanced"})),
        ("smallest", json!({"preset": "smallest"})),
        ("maxKB=60%", Value::Null),
    ];
    // SSIM: 12 Stichproben (wie bisher) und über alle Zeitpunkte (je 10 ms); Farbe: Farbtreue in der
    // Fläche (×100, kleiner = besser, siehe metrics::color_error)
    println!(
        "| Clip | Kodierer | Bytes | KB | vs. Export | SSIM | SSIM (alle 10 ms) | Farbe | PSNR dB | ms |"
    );
    println!("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|");
    for (clip, base) in clips.iter().zip(&baseline) {
        let row = |name: &str, bytes: &[u8], ms: u128| {
            println!(
                "| {} | {} | {} | {:.1} | {:+.1} % | {:.4} | {:.4} | {:.3} | {:.2} | {} |",
                clip.name,
                name,
                bytes.len(),
                bytes.len() as f64 / 1024.0,
                (bytes.len() as f64 / base.len() as f64 - 1.0) * 100.0,
                mean_ssim(clip, bytes),
                full_ssim(clip, bytes),
                mean_color_error(clip, bytes),
                mean_psnr(clip, bytes),
                ms
            );
        };
        row("Export (bisher)", base, 0);
        if let Some(d) = std::env::var_os("FP_DUMP") {
            let _ = std::fs::write(
                std::path::Path::new(&d).join(format!("{}-export.gif", clip.name)),
                base,
            );
        }
        for (name, v) in &variants {
            let v = if v.is_null() {
                json!({"maxKB": (base.len() * 6 / 10 / 1024).max(1)})
            } else {
                v.clone()
            };
            let t = std::time::Instant::now();
            let r = gifopt::optimize(&clip.item(), &opts(v)).expect("optimize");
            row(name, &r.bytes, t.elapsed().as_millis());
            if let Some(d) = std::env::var_os("FP_DUMP") {
                let _ = std::fs::write(
                    std::path::Path::new(&d).join(format!(
                        "{}-{}.gif",
                        clip.name,
                        name.replace(['=', '%'], "")
                    )),
                    &r.bytes,
                );
            }
            if name.starts_with("maxKB") {
                println!(
                    "|  | ↳ Qualität {} · Faktor {:.2} · Schritt {} · {} Kodierungen · erreicht {} | | | | | | | | |",
                    r.quality, r.scale, r.frame_step, r.encodes, r.reached
                );
            }
            if !name.starts_with("maxKB") && *name != "simple" {
                println!(
                    "|  | ↳ Foto-Anteil {:.2} · {} Farben · Verlust {} · {} Teilbilder | | | | | | | | |",
                    r.stats.photo,
                    r.stats.global_colors,
                    r.stats.lossy,
                    r.stats.frames.len()
                );
            }
        }
        // gleiche Qualität wie der Export: kleinste Qualitätsstufe mit SSIM ≥ SSIM des Exports
        let target = mean_ssim(clip, base);
        let (mut lo, mut hi) = (10u8, 100u8);
        let mut found: Option<(u8, Vec<u8>)> = None;
        while lo <= hi {
            let mid = lo + (hi - lo) / 2;
            let r =
                gifopt::optimize(&clip.item(), &opts(json!({"quality": mid}))).expect("optimize");
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

    // B-5 (BA-110): smallest file with SSIM ≥ 0.99 per mode (quality bisection), against the export
    println!();
    println!("| Clip | Mode | Quality | Bytes | vs. export | vs. default | SSIM | ms |");
    println!("|---|---|---:|---:|---:|---:|---:|---:|");
    let modes: [(&str, Value); 6] = [
        ("default", json!({})),
        ("lzw flexible", json!({"lzw": "flexible"})),
        ("perceptual palette", json!({"perceptualPalette": true})),
        (
            "both",
            json!({"lzw": "flexible", "perceptualPalette": true}),
        ),
        // BA-110: lossy LZW bounded by SSIM 0.99 per frame
        (
            "lossy minSsim 0.99",
            json!({"lzw": "flexible", "lossyMinSsim": 0.99}),
        ),
        (
            "lossy minSsim 0.99 + both",
            json!({"lzw": "flexible", "perceptualPalette": true, "lossyMinSsim": 0.99}),
        ),
    ];
    for (clip, base) in clips.iter().zip(&baseline) {
        let mut default_bytes = 0usize;
        for (name, extra) in &modes {
            let (mut lo, mut hi) = (10u8, 100u8);
            let mut found: Option<(u8, Vec<u8>, f64, u128)> = None;
            let mut top = 0f64;
            while lo <= hi {
                let mid = lo + (hi - lo) / 2;
                let mut o = opts(extra.clone());
                o.insert("quality".into(), json!(mid));
                let t = std::time::Instant::now();
                let r = gifopt::optimize(&clip.item(), &o).expect("optimize");
                let ms = t.elapsed().as_millis();
                let ssim = mean_ssim(clip, &r.bytes);
                top = top.max(ssim);
                if ssim >= 0.99 {
                    found = Some((mid, r.bytes, ssim, ms));
                    hi = mid - 1;
                } else {
                    lo = mid + 1;
                }
            }
            match found {
                Some((q, bytes, ssim, ms)) => {
                    if *name == "default" {
                        default_bytes = bytes.len();
                    }
                    println!(
                        "| {} | {} | {} | {} | {:+.1} % | {:+.1} % | {:.4} | {} |",
                        clip.name,
                        name,
                        q,
                        bytes.len(),
                        (bytes.len() as f64 / base.len() as f64 - 1.0) * 100.0,
                        (bytes.len() as f64 / default_bytes.max(1) as f64 - 1.0) * 100.0,
                        ssim,
                        ms
                    );
                }
                None => println!(
                    "| {} | {} | – | – | – | – | best {:.4} | – |",
                    clip.name, name, top
                ),
            }
        }
    }

    // BA-110: at fixed quality, flexible parsing vs. lossy LZW bounded by SSIM 0.99 per frame
    println!();
    println!("| Clip | Quality | Flexible B | SSIM | Bounded B | SSIM | vs. flexible | ms |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|");
    for clip in &clips {
        for q in [100u8, 90, 80] {
            let flex = gifopt::optimize(
                &clip.item(),
                &opts(json!({"quality": q, "lzw": "flexible"})),
            )
            .expect("optimize");
            let t = std::time::Instant::now();
            let bounded = gifopt::optimize(
                &clip.item(),
                &opts(json!({"quality": q, "lzw": "flexible", "lossyMinSsim": 0.99})),
            )
            .expect("optimize");
            let ms = t.elapsed().as_millis();
            println!(
                "| {} | {} | {} | {:.4} | {} | {:.4} | {:+.1} % | {} |",
                clip.name,
                q,
                flex.bytes.len(),
                mean_ssim(clip, &flex.bytes),
                bounded.bytes.len(),
                mean_ssim(clip, &bounded.bytes),
                (bounded.bytes.len() as f64 / flex.bytes.len() as f64 - 1.0) * 100.0,
                ms
            );
        }
    }
}

//! Vergleich mit der PyTorch-Referenz (offizieller Python-Code + offizielle Gewichte): Stichproben
//! der Ausgaben in `testdata\referenz.json` (erzeugt aus derselben Vorverarbeitung). Läuft nur, wenn
//! die Modelle im Modellordner liegen.

use fp_ai::birefnet::BiRefNet;
use fp_ai::models::{self, ModelId};
use fp_ai::sam2::{Prompt, Sam2};
use fp_ai::weights::Weights;
use fp_ai::{nn, pipeline, Progress};
use serde_json::Value;
use std::path::PathBuf;

fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

fn fixture() -> Value {
    let s = std::fs::read_to_string(data().join("referenz.json")).unwrap_or_default();
    serde_json::from_str(&s).unwrap_or(Value::Null)
}

fn image(name: &str) -> fp_raster::RgbaImage {
    let bytes = std::fs::read(data().join(name)).unwrap_or_default();
    fp_io::jpeg_in::decode(&bytes).map(|r| r.image).unwrap_or_else(|_| fp_raster::RgbaImage::empty_at(0, 0))
}

/// Größte Abweichung an den Stichproben.
fn max_diff(samples: &Value, got: &[f32]) -> f32 {
    samples.as_array().map(|a| a.iter().map(|s| (s[1].as_f64().unwrap_or(0.0) as f32 - got[s[0].as_u64().unwrap_or(0) as usize]).abs()).fold(0.0f32, f32::max)).unwrap_or(f32::MAX)
}

fn birefnet_case(key: &str, model: ModelId, tol: f32) {
    let Some(path) = models::find(model) else {
        eprintln!("{key}: Modell fehlt – übersprungen");
        return;
    };
    let f = fixture();
    let case = &f[key];
    let img = image(case["image"].as_str().unwrap_or_default());
    let s = case["size"].as_u64().unwrap_or(1024) as usize;
    let net = nn::pool().install(|| BiRefNet::load(Weights::load(&path).expect("Gewichte"))).expect("Modell");
    let x = pipeline::model_input(&img, img.bounds(), s);
    let y = nn::pool().install(|| net.forward(&x, s, &Progress::new())).expect("Vorwärts");
    let d = max_diff(&case["logits"], &y);
    eprintln!("{key}: größte Abweichung der Logits {d:.2e}");
    assert!(d < tol, "{key}: Abweichung {d}");
}

#[test]
fn birefnet_lite_matches_pytorch() {
    birefnet_case("birefnet_lite", ModelId::BiRefNetLite, 1e-3);
}

#[test]
fn birefnet_lite_matting_matches_pytorch() {
    birefnet_case("birefnet_lite_matting", ModelId::BiRefNetLiteMatting, 1e-3);
}

#[test]
fn sam2_tiny_matches_pytorch() {
    let Some(path) = models::find(ModelId::Sam2Tiny) else {
        eprintln!("SAM 2.1 fehlt – übersprungen");
        return;
    };
    let f = fixture();
    let case = &f["sam2_tiny"];
    let img = image(case["image"].as_str().unwrap_or_default());
    let sam = nn::pool().install(|| Sam2::load(Weights::load(&path).expect("Gewichte"))).expect("Modell");
    let x = pipeline::model_input(&img, img.bounds(), fp_ai::sam2::SIZE);
    let e = nn::pool().install(|| sam.encode(&x, &Progress::new())).expect("Encoder");
    let d_emb = max_diff(&case["emb"], &e.emb);
    let d_s0 = max_diff(&case["s0"], &e.s0);
    let points: Vec<(f32, f32, i32)> = case["points"].as_array().map(|a| a.iter().map(|p| (p[0].as_f64().unwrap_or(0.0) as f32, p[1].as_f64().unwrap_or(0.0) as f32, p[2].as_i64().unwrap_or(1) as i32)).collect()).unwrap_or_default();
    let p = Prompt { multimask: points.len() == 1, points, ..Prompt::default() };
    let d = nn::pool().install(|| sam.decode(&e, &p)).expect("Decoder");
    let d_logits = max_diff(&case["logits"], &d.logits);
    eprintln!("SAM 2.1: Einbettung {d_emb:.2e}, s0 {d_s0:.2e}, Logits {d_logits:.2e}, IoU {} / {}", d.iou, case["iou"]);
    assert!(d_emb < 1e-3 && d_s0 < 1e-3, "Einbettung");
    assert!(d_logits < 1e-2, "Logits");
    assert!((d.iou - case["iou"].as_f64().unwrap_or(0.0) as f32).abs() < 1e-3);
    assert!((d.object_score - case["object_score"].as_f64().unwrap_or(0.0) as f32).abs() < 1e-2);
}

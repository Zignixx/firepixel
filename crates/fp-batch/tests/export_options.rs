//! BA-108: WebP `alphaQuality`/`method` and AVIF `alphaQuality` from `output.options` reach the
//! encoders (two settings → different files).

use fp_batch::OutputFormat;
use fp_batch::output::encode;
use serde_json::{Map, Value, json};

/// Gradient with varying alpha, so the alpha plane matters.
fn item() -> fp_batch::Item {
    let (w, h) = (64u32, 64u32);
    let mut v = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let a = ((x * 4) ^ (y * 7)) & 0xff;
            v.extend_from_slice(&[(x * 3) as u8, (y * 5) as u8, 64, a.max(1) as u8]);
        }
    }
    fp_batch::item::still_from_rgba(w, h, &v).expect("item")
}

fn opts(v: Value) -> Map<String, Value> {
    v.as_object().cloned().unwrap_or_default()
}

fn bytes(fmt: OutputFormat, v: Value) -> Vec<u8> {
    encode(&item(), fmt, &opts(v)).expect("encode").0
}

#[test]
fn webp_alpha_quality_and_method_are_passed_through() {
    let full = bytes(
        OutputFormat::WebP,
        json!({"quality": 70, "alphaQuality": 100}),
    );
    let low = bytes(
        OutputFormat::WebP,
        json!({"quality": 70, "alphaQuality": 5}),
    );
    assert_ne!(full, low);
    assert!(low.len() < full.len(), "{} vs {}", low.len(), full.len());
    let fast = bytes(OutputFormat::WebP, json!({"quality": 70, "method": 0}));
    let slow = bytes(OutputFormat::WebP, json!({"quality": 70, "method": 6}));
    assert_ne!(fast, slow);
    // defaults unchanged: alpha 100, method 4
    assert_eq!(bytes(OutputFormat::WebP, json!({"quality": 70})), full);
}

#[test]
fn avif_alpha_quality_is_passed_through() {
    let high = bytes(
        OutputFormat::Avif,
        json!({"quality": 60, "speed": 10, "alphaQuality": 100}),
    );
    let low = bytes(
        OutputFormat::Avif,
        json!({"quality": 60, "speed": 10, "alphaQuality": 5}),
    );
    assert_ne!(high, low);
    assert!(low.len() < high.len(), "{} vs {}", low.len(), high.len());
}

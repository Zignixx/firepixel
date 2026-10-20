//! Real-ESRGAN with the real weights (`#[ignore]`, paths from the environment):
//!
//! * `FP_ESRGAN_X2` = `RealESRGAN_x2plus.safetensors`: network output on the 64 × 64 reference
//!   input against the PyTorch output of the official weights (`testdata\esrgan_x2_reference.json`,
//!   samples of `skillsafe-ai/realesrgan-x2plus` `reference/output-64x64.npy`).
//! * `FP_ESRGAN_X4` = `RealESRGAN_x4plus.safetensors`: upscale a 64 × 64 image, time on a pool of
//!   six threads.
//!
//! `cargo test -p fp-ai --test esrgan_reference -- --ignored --nocapture`

use fp_ai::esrgan::Esrgan;
use fp_ai::weights::Weights;
use fp_ai::{Cancel, Progress};
use serde_json::Value;
use std::path::PathBuf;

fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

fn env_path(var: &str) -> Option<PathBuf> {
    let p = PathBuf::from(std::env::var_os(var)?);
    p.is_file().then_some(p)
}

/// `.npy` with `<f4` values (header skipped).
fn npy_f32(bytes: &[u8]) -> Vec<f32> {
    let n = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
    bytes[10 + n..]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| f32::from_le_bytes(*c))
        .collect()
}

fn pool6() -> rayon::ThreadPool {
    rayon::ThreadPoolBuilder::new()
        .num_threads(6)
        .build()
        .expect("pool")
}

fn load(path: &std::path::Path) -> Esrgan {
    Esrgan::load(Weights::load(path).expect("weights")).expect("network")
}

#[test]
#[ignore = "needs FP_ESRGAN_X2 = path of RealESRGAN_x2plus.safetensors"]
fn x2plus_matches_pytorch_reference() {
    let Some(path) = env_path("FP_ESRGAN_X2") else {
        eprintln!("FP_ESRGAN_X2 not set – skipped");
        return;
    };
    let net = load(&path);
    assert_eq!((net.scale(), net.blocks()), (2, 23));
    let x = npy_f32(&std::fs::read(data().join("esrgan_x2_input_64.npy")).expect("input"));
    assert_eq!(x.len(), 3 * 64 * 64);
    let refj: Value = serde_json::from_str(
        &std::fs::read_to_string(data().join("esrgan_x2_reference.json")).expect("reference"),
    )
    .expect("json");
    let pool = pool6();
    let t = std::time::Instant::now();
    let y = pool
        .install(|| net.forward(&x, 64, 64, &Cancel::new()))
        .expect("forward");
    eprintln!("x2plus 64×64 → 128×128: {:.2} s", t.elapsed().as_secs_f64());
    assert_eq!(y.len(), 3 * 128 * 128);
    let mut d = 0.0f32;
    for s in refj["samples"].as_array().expect("samples") {
        let i = s[0].as_u64().unwrap_or(0) as usize;
        d = d.max((s[1].as_f64().unwrap_or(0.0) as f32 - y[i]).abs());
    }
    let mean = y.iter().map(|v| *v as f64).sum::<f64>() / y.len() as f64;
    eprintln!(
        "largest deviation {d:.2e}, mean {mean:.6} (reference {:.6})",
        refj["mean"].as_f64().unwrap_or(0.0)
    );
    assert!(d < 1e-4, "deviation {d}");
    assert!((mean - refj["mean"].as_f64().unwrap_or(0.0)).abs() < 1e-5);
    // tiled against one pass for an input larger than one tile (48 network px tiles, default
    // context): only the tile seams differ a little
    let big: Vec<f32> = (0..3 * 200 * 210).map(|i| x[i % x.len()]).collect();
    let full = pool
        .install(|| net.forward(&big, 200, 210, &Cancel::new()))
        .expect("full");
    let tiled = pool
        .install(|| {
            net.forward_tiled(
                &big,
                200,
                210,
                48,
                fp_ai::esrgan::TILE_PAD,
                &Progress::new(),
                0.0,
                1.0,
            )
        })
        .expect("tiled");
    let mut dmax = 0.0f32;
    let mut dsum = 0.0f64;
    for (a, b) in full.iter().zip(&tiled) {
        let e = (a.clamp(0.0, 1.0) - b.clamp(0.0, 1.0)).abs();
        dmax = dmax.max(e);
        dsum += e as f64;
    }
    eprintln!(
        "tiled vs one pass (48 px tiles): largest {:.2} of 255, mean {:.4} of 255",
        dmax * 255.0,
        dsum / full.len() as f64 * 255.0
    );
    assert!(dsum / full.len() as f64 * 255.0 < 1.0);
}

#[test]
#[ignore = "needs FP_ESRGAN_X4 = path of RealESRGAN_x4plus.safetensors"]
fn x4plus_upscales_64px_image() {
    let Some(path) = env_path("FP_ESRGAN_X4") else {
        eprintln!("FP_ESRGAN_X4 not set – skipped");
        return;
    };
    let t = std::time::Instant::now();
    let net = load(&path);
    eprintln!("x4plus loaded in {:.2} s", t.elapsed().as_secs_f64());
    assert_eq!((net.scale(), net.blocks()), (4, 23));
    // 64 × 64: soft diagonal gradient with a hard-edged square
    let (h, w) = (64usize, 64usize);
    let mut x = vec![0.0f32; 3 * h * w];
    for y in 0..h {
        for xx in 0..w {
            let inside = (20..44).contains(&y) && (20..44).contains(&xx);
            let g = (y + xx) as f32 / (h + w) as f32;
            let v = [
                if inside { 0.9 } else { g },
                if inside { 0.2 } else { 0.5 * g },
                if inside { 0.1 } else { 1.0 - g },
            ];
            for c in 0..3 {
                x[c * h * w + y * w + xx] = v[c];
            }
        }
    }
    let pool = pool6();
    for run in 0..2 {
        let t = std::time::Instant::now();
        let y = pool
            .install(|| net.forward(&x, h, w, &Cancel::new()))
            .expect("forward");
        eprintln!(
            "x4plus 64×64 → 256×256 (6 threads, run {}): {:.2} s",
            run + 1,
            t.elapsed().as_secs_f64()
        );
        assert_eq!(y.len(), 3 * 256 * 256);
        // inside the square the colour stays (0.9, 0.2, 0.1)
        let at = |c: usize, yy: usize, xx: usize| y[c * 256 * 256 + yy * 256 + xx];
        let centre = [at(0, 128, 128), at(1, 128, 128), at(2, 128, 128)];
        eprintln!("centre {centre:?}");
        assert!((centre[0] - 0.9).abs() < 0.05 && (centre[1] - 0.2).abs() < 0.05);
    }
}

/// Writes a 64 × 64 detail of `testdata\katze.jpg`, bicubic ×4 and Real-ESRGAN ×4 side by side to
/// `FP_ESRGAN_OUT` (PNG) for a visual check.
#[test]
#[ignore = "needs FP_ESRGAN_X4 and FP_ESRGAN_OUT"]
fn x4plus_writes_comparison_png() {
    let (Some(path), Some(out)) = (env_path("FP_ESRGAN_X4"), std::env::var_os("FP_ESRGAN_OUT"))
    else {
        eprintln!("FP_ESRGAN_X4 / FP_ESRGAN_OUT not set – skipped");
        return;
    };
    let net = load(&path);
    let bytes = std::fs::read(data().join("katze.jpg")).expect("jpg");
    let img = fp_io::jpeg_in::decode(&bytes).expect("decode").image;
    let (cx, cy) = (img.width() as i32 / 2, img.height() as i32 / 2);
    let crop = img.cropped(fp_geom::IRect::new(cx - 32, cy - 32, cx + 32, cy + 32));
    let t = std::time::Instant::now();
    let up = fp_ai::esrgan::upscale_with(
        &net,
        &crop,
        None,
        fp_ai::esrgan::AlphaMode::Network,
        &Progress::new(),
    )
    .expect("upscale");
    eprintln!(
        "64×64 photo detail ×4 (AI pool): {:.2} s",
        t.elapsed().as_secs_f64()
    );
    // left: bicubic, right: network
    let mut planes = vec![0.0f32; 3 * 64 * 64];
    for (i, p) in crop.data().iter().enumerate() {
        for c in 0..3 {
            planes[c * 4096 + i] = p[c] as f32;
        }
    }
    let bic = fp_ai::nn::resize_bicubic(&planes, 3, 64, 64, 256, 256);
    let mut rgb = vec![0u8; 512 * 256 * 3];
    for y in 0..256 {
        for x in 0..256 {
            for c in 0..3 {
                rgb[(y * 512 + x) * 3 + c] = bic[c * 65536 + y * 256 + x].clamp(0.0, 255.0) as u8;
                rgb[(y * 512 + 256 + x) * 3 + c] = up.data()[y * 256 + x][c];
            }
        }
    }
    let f = std::fs::File::create(&out).expect("create");
    let mut e = png::Encoder::new(std::io::BufWriter::new(f), 512, 256);
    e.set_color(png::ColorType::Rgb);
    e.set_depth(png::BitDepth::Eight);
    e.write_header()
        .and_then(|mut w| w.write_image_data(&rgb))
        .expect("png");
}

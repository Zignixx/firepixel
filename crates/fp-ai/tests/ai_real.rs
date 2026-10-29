//! F143: the AI features with the real model weights (`#[ignore]`; every test skips cleanly when
//! its model is not in the model folder). Plausibility is checked numerically on the photos in
//! `testdata\` and on generated images; outputs are written as PNG to
//! `<target>\tmp\ai_real\` (ignored) for a look by eye.
//!
//! `cargo test -p fp-ai --release --test ai_real -- --ignored --nocapture --test-threads 1`

use fp_ai::esrgan::{self, AlphaMode};
use fp_ai::models::{self, ModelId};
use fp_ai::pipeline::{self, ObjectPrompt, PromptPoint};
use fp_ai::{AiError, Progress, cache};
use fp_raster::{IRect, Mask, RgbaImage};
use std::path::PathBuf;
use std::time::Instant;

fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

fn out_dir() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ai_real");
    let _ = std::fs::create_dir_all(&d);
    d
}

fn photo(name: &str) -> RgbaImage {
    let bytes = std::fs::read(data().join(name)).expect("test photo");
    fp_io::jpeg_in::decode(&bytes).expect("jpeg").image
}

fn need(id: ModelId) -> bool {
    if models::available(id) {
        return true;
    }
    eprintln!("{id:?} missing in {:?} – skipped", models::search_folders());
    false
}

fn save_png(name: &str, w: usize, h: usize, rgba: &[u8]) {
    let p = out_dir().join(name);
    let Ok(f) = std::fs::File::create(&p) else {
        return;
    };
    let mut e = png::Encoder::new(std::io::BufWriter::new(f), w as u32, h as u32);
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    if let Ok(mut wr) = e.write_header() {
        let _ = wr.write_image_data(rgba);
    }
}

fn save_image(name: &str, img: &RgbaImage) {
    let flat: Vec<u8> = img.data().iter().flatten().copied().collect();
    save_png(name, img.width(), img.height(), &flat);
}

fn save_mask(name: &str, m: &Mask) {
    let flat: Vec<u8> = m.data().iter().flat_map(|v| [*v, *v, *v, 255]).collect();
    save_png(name, m.width(), m.height(), &flat);
}

/// Copy of `r` (origin 0, 0).
fn crop(img: &RgbaImage, r: IRect) -> RgbaImage {
    let b = img.bounds();
    let mut px = Vec::with_capacity((r.width() * r.height()) as usize);
    for y in r.top..r.bottom {
        for x in r.left..r.right {
            px.push(img.data()[(y - b.top) as usize * img.width() + (x - b.left) as usize]);
        }
    }
    RgbaImage::from_raw(IRect::new(0, 0, r.width() as i32, r.height() as i32), px).expect("crop")
}

/// Box downscale by `f` (origin 0, 0).
fn shrink(img: &RgbaImage, f: usize) -> RgbaImage {
    let (w, h) = (img.width() / f, img.height() / f);
    let mut px = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0u32; 4];
            for dy in 0..f {
                for dx in 0..f {
                    let p = img.data()[(y * f + dy) * img.width() + x * f + dx];
                    for c in 0..4 {
                        acc[c] += p[c] as u32;
                    }
                }
            }
            let n = (f * f) as u32;
            px.push([
                (acc[0] / n) as u8,
                (acc[1] / n) as u8,
                (acc[2] / n) as u8,
                (acc[3] / n) as u8,
            ]);
        }
    }
    RgbaImage::from_raw(IRect::new(0, 0, w as i32, h as i32), px).expect("shrink")
}

/// Bilinear upscale by `f` (the "no AI" baseline).
fn bilinear(img: &RgbaImage, f: usize) -> RgbaImage {
    let (w, h) = (img.width(), img.height());
    let (ow, oh) = (w * f, h * f);
    let at = |x: usize, y: usize| img.data()[y.min(h - 1) * w + x.min(w - 1)];
    let mut px = Vec::with_capacity(ow * oh);
    for y in 0..oh {
        let fy = ((y as f32 + 0.5) / f as f32 - 0.5).max(0.0);
        let (y0, ty) = (fy.floor() as usize, fy.fract());
        for x in 0..ow {
            let fx = ((x as f32 + 0.5) / f as f32 - 0.5).max(0.0);
            let (x0, tx) = (fx.floor() as usize, fx.fract());
            let (a, b, c, d) = (
                at(x0, y0),
                at(x0 + 1, y0),
                at(x0, y0 + 1),
                at(x0 + 1, y0 + 1),
            );
            let mut o = [0u8; 4];
            for k in 0..4 {
                let top = a[k] as f32 * (1.0 - tx) + b[k] as f32 * tx;
                let bot = c[k] as f32 * (1.0 - tx) + d[k] as f32 * tx;
                o[k] = (top * (1.0 - ty) + bot * ty).round() as u8;
            }
            px.push(o);
        }
    }
    RgbaImage::from_raw(IRect::new(0, 0, ow as i32, oh as i32), px).expect("bilinear")
}

fn luma(p: [u8; 4]) -> f32 {
    0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32
}

/// Mean absolute Laplacian of the luma (sharpness).
fn sharpness(img: &RgbaImage) -> f64 {
    let (w, h) = (img.width(), img.height());
    let l = |x: usize, y: usize| luma(img.data()[y * w + x]);
    let mut s = 0.0f64;
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let v = 4.0 * l(x, y) - l(x - 1, y) - l(x + 1, y) - l(x, y - 1) - l(x, y + 1);
            s += v.abs() as f64;
        }
    }
    s / ((w - 2) * (h - 2)) as f64
}

/// Mean absolute RGB difference (same size).
fn mean_diff(a: &RgbaImage, b: &RgbaImage) -> f64 {
    let n = a.data().len().min(b.data().len()).max(1);
    let s: f64 = a
        .data()
        .iter()
        .zip(b.data())
        .map(|(p, q)| {
            (0..3)
                .map(|c| (p[c] as f64 - q[c] as f64).abs())
                .sum::<f64>()
                / 3.0
        })
        .sum();
    s / n as f64
}

/// PSNR of the RGB channels (same size).
fn psnr(a: &RgbaImage, b: &RgbaImage) -> f64 {
    let n = a.data().len().min(b.data().len()).max(1);
    let se: f64 = a
        .data()
        .iter()
        .zip(b.data())
        .map(|(p, q)| {
            (0..3)
                .map(|c| (p[c] as f64 - q[c] as f64).powi(2))
                .sum::<f64>()
                / 3.0
        })
        .sum();
    10.0 * (255.0f64.powi(2) / (se / n as f64).max(1e-9)).log10()
}

fn coverage(m: &Mask) -> f64 {
    m.data().iter().map(|v| *v as f64 / 255.0).sum::<f64>() / m.data().len().max(1) as f64
}

/// Working set of the process in MB (Windows; elsewhere `None`).
fn rss_mb() -> Option<f64> {
    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!("(Get-Process -Id {}).PeakWorkingSet64", std::process::id()),
        ])
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse::<f64>()
        .ok()
        .map(|b| b / 1e6)
}

/// Real-ESRGAN on a real photo: the photo is shrunk by the factor, upscaled again, and compared
/// with the original (PSNR) and a bilinear upscale (sharpness, colour shift).
fn upscale_case(id: ModelId, f: usize) {
    if !need(id) {
        return;
    }
    let img = photo("produkt_tasse.jpg");
    let (w, h) = (img.width() as i32, img.height() as i32);
    let side = 256i32;
    let r = IRect::from_xywh((w - side) / 2, (h - side) / 2, side, side);
    let orig = crop(&img, r);
    let small = shrink(&orig, f);
    let t = Instant::now();
    let prog = Progress::new();
    let up = esrgan::upscale_image(&small, None, id, AlphaMode::Network, &prog).expect("upscale");
    let secs = t.elapsed().as_secs_f64();
    assert_eq!((up.width(), up.height()), (side as usize, side as usize));
    let base = bilinear(&small, f);
    let (s_ai, s_bl, s_or) = (sharpness(&up), sharpness(&base), sharpness(&orig));
    let (p_ai, p_bl) = (psnr(&up, &orig), psnr(&base, &orig));
    let shift = mean_diff(&up, &base);
    eprintln!(
        "{id:?} ×{f}: {}×{} → {}×{} in {secs:.2} s (incl. load), sharpness AI {s_ai:.2} / bilinear {s_bl:.2} / original {s_or:.2}, PSNR AI {p_ai:.2} dB / bilinear {p_bl:.2} dB, mean |AI − bilinear| {shift:.2}, cache {} MB, peak RSS {:?} MB",
        small.width(),
        small.height(),
        up.width(),
        up.height(),
        cache::memory_bytes() / 1_000_000,
        rss_mb()
    );
    save_image(&format!("upscale_{f}x_small.png"), &small);
    save_image(&format!("upscale_{f}x_ai.png"), &up);
    save_image(&format!("upscale_{f}x_bilinear.png"), &base);
    assert!(s_ai > s_bl * 1.3, "AI result sharper than bilinear");
    assert!(
        shift < 20.0,
        "colours stay (no shift, no channel swap): {shift}"
    );
    assert!(p_ai > 20.0, "close to the original: {p_ai} dB");
    // second run: model in the cache, only the network
    let t = Instant::now();
    let _ = esrgan::upscale_image(&small, None, id, AlphaMode::Network, &Progress::new())
        .expect("again");
    eprintln!("{id:?} warm: {:.2} s", t.elapsed().as_secs_f64());
}

#[test]
#[ignore = "needs RealESRGAN_x4plus in the model folder"]
fn ai_real_upscale_x4() {
    upscale_case(ModelId::RealEsrganX4, 4);
}

#[test]
#[ignore = "needs RealESRGAN_x2plus in the model folder"]
fn ai_real_upscale_x2() {
    upscale_case(ModelId::RealEsrganX2, 2);
}

/// Transparent image: the alpha goes through the network as a grey image and stays a clean disk.
#[test]
#[ignore = "needs RealESRGAN_x2plus in the model folder"]
fn ai_real_upscale_alpha() {
    if !need(ModelId::RealEsrganX2) {
        return;
    }
    let n = 64usize;
    let px: Vec<[u8; 4]> = (0..n * n)
        .map(|i| {
            let (x, y) = ((i % n) as f32 - 31.5, (i / n) as f32 - 31.5);
            let a = if x.hypot(y) < 20.0 { 255 } else { 0 };
            [200, 40, 30, a]
        })
        .collect();
    let img = RgbaImage::from_raw(IRect::new(0, 0, n as i32, n as i32), px).expect("img");
    let up = esrgan::upscale_image(
        &img,
        None,
        ModelId::RealEsrganX2,
        AlphaMode::Network,
        &Progress::new(),
    )
    .expect("upscale");
    let w = up.width();
    let a = |x: usize, y: usize| up.data()[y * w + x][3];
    let (inside, c0, c1) = (a(64, 64), a(2, 2), a(125, 125));
    eprintln!("alpha through the network: inside {inside}, corners {c0} / {c1}");
    assert!(
        inside == 255 && c0 <= 8 && c1 <= 8,
        "inside opaque, corners clear"
    );
    let c = up.data()[64 * w + 64];
    assert!(c[0] > 170 && c[1] < 80 && c[2] < 70, "colour kept: {c:?}");
    save_image("upscale_alpha_x2.png", &up);
}

/// Cancel during the network run: `Cancelled`, quickly.
#[test]
#[ignore = "needs RealESRGAN_x4plus in the model folder"]
fn ai_real_upscale_cancel() {
    if !need(ModelId::RealEsrganX4) {
        return;
    }
    let img = photo("katze.jpg");
    let prog = Progress::new();
    // load first, so the cancel lands in the tiles
    let _ = esrgan::load(ModelId::RealEsrganX4, &Progress::new()).expect("load");
    let p2 = prog.clone();
    let t = Instant::now();
    let h = std::thread::spawn(move || {
        esrgan::upscale_image(&img, None, ModelId::RealEsrganX4, AlphaMode::Bicubic, &p2)
    });
    std::thread::sleep(std::time::Duration::from_millis(1500));
    let at = prog.get();
    prog.cancel.cancel();
    let r = h.join().expect("thread");
    let secs = t.elapsed().as_secs_f64();
    eprintln!("cancel at {:.1} % → returned after {secs:.2} s", at * 100.0);
    assert!(matches!(r, Err(AiError::Cancelled)), "{r:?}");
    assert!(secs < 30.0, "cancel is honoured within a tile");
}

/// BiRefNet: the cup on the product photo is the subject, the corners are background.
fn subject_case(id: ModelId, image: &str) -> Option<Mask> {
    subject_case_with(id, image, false)
}

/// `touches_bottom`: the subject reaches the lower edge (portrait), only the top corners are
/// background.
fn subject_case_with(id: ModelId, image: &str, touches_bottom: bool) -> Option<Mask> {
    if !need(id) {
        return None;
    }
    let img = photo(image);
    let t = Instant::now();
    let m = pipeline::subject_mask(&img, None, id, &Progress::new()).expect("subject");
    let secs = t.elapsed().as_secs_f64();
    let b = m.bounds();
    let corners = [
        m.get(b.left + 2, b.top + 2),
        m.get(b.right - 3, b.top + 2),
        m.get(b.left + 2, b.bottom - 3),
        m.get(b.right - 3, b.bottom - 3),
    ];
    eprintln!(
        "{id:?} on {image} ({}×{}): {secs:.2} s (incl. load), coverage {:.1} %, tight box {:?}, corners {corners:?}, cache {} MB, peak RSS {:?} MB",
        img.width(),
        img.height(),
        coverage(&m) * 100.0,
        m.tight_bounds(),
        cache::memory_bytes() / 1_000_000,
        rss_mb()
    );
    save_mask(&format!("subject_{id:?}_{image}.png"), &m);
    let n = if touches_bottom { 2 } else { 4 };
    assert!(
        corners[..n].iter().all(|c| *c < 32),
        "corners are background"
    );
    let cov = coverage(&m);
    assert!((0.03..0.85).contains(&cov), "coverage {cov}");
    Some(m)
}

#[test]
#[ignore = "needs BiRefNet_lite in the model folder"]
fn ai_real_subject_lite() {
    for image in ["produkt_tasse.jpg", "hund_gras.jpg", "katze.jpg"] {
        subject_case(ModelId::BiRefNetLite, image);
    }
}

#[test]
#[ignore = "needs BiRefNet (Swin-L) in the model folder"]
fn ai_real_subject_full() {
    subject_case(ModelId::BiRefNet, "produkt_tasse.jpg");
    cache::unload(ModelId::BiRefNet);
}

/// SAM 2.1: a click on a clear object (red disk on a noisy green background) selects the disk.
fn object_case(id: ModelId) {
    if !need(id) {
        return;
    }
    let (w, h) = (480usize, 360usize);
    let (cx, cy, rad) = (180.0f32, 170.0f32, 70.0f32);
    let px: Vec<[u8; 4]> = (0..w * h)
        .map(|i| {
            let (x, y) = ((i % w) as i32, (i / w) as i32);
            let n = (((x * 7919 + y * 104_729) ^ (x * y)) & 63) as u8;
            if ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt() <= rad {
                [220, 30 + n / 4, 40, 255]
            } else {
                [60 + n / 2, 120 + n, 50, 255]
            }
        })
        .collect();
    let img = RgbaImage::from_raw(IRect::new(0, 0, w as i32, h as i32), px).expect("img");
    let prompt = ObjectPrompt {
        points: vec![PromptPoint {
            x: cx,
            y: cy,
            positive: true,
        }],
        boxx: None,
    };
    let t = Instant::now();
    let r = pipeline::object_mask(&img, None, &prompt, id, &Progress::new()).expect("object");
    let cold = t.elapsed().as_secs_f64();
    let t = Instant::now();
    let prompt2 = ObjectPrompt {
        points: vec![PromptPoint {
            x: cx + 20.0,
            y: cy - 10.0,
            positive: true,
        }],
        boxx: None,
    };
    let r2 = pipeline::object_mask(&img, None, &prompt2, id, &Progress::new()).expect("object 2");
    let warm = t.elapsed().as_secs_f64();
    // intersection over union with the true disk
    let (mut inter, mut uni) = (0usize, 0usize);
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let truth = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt() <= rad;
            let sel = r.mask.get(x, y) >= 128;
            inter += (truth && sel) as usize;
            uni += (truth || sel) as usize;
        }
    }
    let iou = inter as f64 / uni.max(1) as f64;
    eprintln!(
        "{id:?}: cold {cold:.2} s (load + encoder + decoder), warm click {warm:.3} s (cached {}), IoU with the disk {iou:.3}, predicted IoU {:.3}, object score {:.2}, cache {} MB, peak RSS {:?} MB",
        r2.cached,
        r.iou,
        r.object_score,
        cache::memory_bytes() / 1_000_000,
        rss_mb()
    );
    save_mask(&format!("object_{id:?}.png"), &r.mask);
    assert!(r2.cached, "second click reuses the embedding");
    assert!(iou > 0.85, "IoU {iou}");
    assert!(r.object_score > 0.0);
}

#[test]
#[ignore = "needs SAM 2.1 Hiera Tiny in the model folder"]
fn ai_real_object_tiny() {
    object_case(ModelId::Sam2Tiny);
}

#[test]
#[ignore = "needs SAM 2.1 Hiera Small in the model folder"]
fn ai_real_object_small() {
    object_case(ModelId::Sam2Small);
}

/// SAM 2.1 on a photo: a click on the cup gives a compact mask containing the click.
#[test]
#[ignore = "needs SAM 2.1 Hiera Tiny in the model folder"]
fn ai_real_object_photo() {
    if !need(ModelId::Sam2Tiny) {
        return;
    }
    let img = photo("produkt_tasse.jpg");
    // the body of the cup below the foam (photo 1280 × 854)
    let (cx, cy) = (700.0f32, 570.0f32);
    let prompt = ObjectPrompt {
        points: vec![PromptPoint {
            x: cx,
            y: cy,
            positive: true,
        }],
        boxx: None,
    };
    let r = pipeline::object_mask(&img, None, &prompt, ModelId::Sam2Tiny, &Progress::new())
        .expect("object");
    let cov = coverage(&r.mask);
    eprintln!(
        "SAM 2.1 tiny on the cup photo: coverage {:.1} %, box {:?}, IoU {:.3}",
        cov * 100.0,
        r.mask.tight_bounds(),
        r.iou
    );
    save_mask("object_photo.png", &r.mask);
    let b = r.mask.tight_bounds();
    assert!(r.mask.get(cx as i32, cy as i32) >= 128, "click inside");
    assert!((0.01..0.2).contains(&cov), "coverage {cov}");
    assert!(
        b.left > 500 && b.right < 1000 && b.top > 300 && b.bottom < 700,
        "cup only: {b:?}"
    );
}

/// Matting: refine the edge of the subject mask; the result stays close to the subject.
#[test]
#[ignore = "needs BiRefNet_lite and BiRefNet_lite-matting in the model folder"]
fn ai_real_refine_edge() {
    let Some(sel) = subject_case_with(ModelId::BiRefNetLite, "person_haare.jpg", true) else {
        return;
    };
    if !need(ModelId::BiRefNetLiteMatting) {
        return;
    }
    let img = photo("person_haare.jpg");
    let t = Instant::now();
    let m = pipeline::refine_edge(
        &img,
        &sel,
        16.0,
        ModelId::BiRefNetLiteMatting,
        &Progress::new(),
    )
    .expect("refine");
    let soft = m.data().iter().filter(|v| (16..240).contains(*v)).count();
    eprintln!(
        "refine edge (lite matting, 16 px): {:.2} s, coverage {:.1} % (subject {:.1} %), {soft} semi-transparent px",
        t.elapsed().as_secs_f64(),
        coverage(&m) * 100.0,
        coverage(&sel) * 100.0
    );
    save_mask("refine_person.png", &m);
    assert!((coverage(&m) - coverage(&sel)).abs() < 0.1);
    assert!(soft > 100, "soft edge exists");
}

/// Empty model folder: clear "model missing" error naming the model and the dialog.
#[test]
#[ignore = "changes the model search of the process"]
fn ai_real_missing_message() {
    fp_ai::download::disable();
    cache::unload_all();
    let dir = fp_i18n::profile::unique_temp_dir("ai-real-empty");
    models::set_search_override(vec![dir.clone()]);
    let img = photo("katze.jpg");
    let e = fp_i18n::with_language("en", || {
        esrgan::upscale_image(
            &img,
            None,
            ModelId::RealEsrganX4,
            AlphaMode::Network,
            &Progress::new(),
        )
        .map(|_| ())
        .unwrap_err()
        .to_string()
    });
    models::set_search_override(Vec::new());
    eprintln!("{e}");
    assert!(
        e.contains("Real-ESRGAN") && e.contains("Manage AI Models"),
        "{e}"
    );
}

/// Time of ×2 and ×4 on the same 320 × 213 photo (minimum of three warm runs each). The body
/// of ×2plus runs on the half resolution (pixel unshuffle), so ×2 should be clearly faster.
#[test]
#[ignore = "needs both Real-ESRGAN models in the model folder"]
fn ai_real_upscale_timing() {
    if !need(ModelId::RealEsrganX2) || !need(ModelId::RealEsrganX4) {
        return;
    }
    let img = shrink(&photo("katze.jpg"), 4);
    let mut line = format!("{}×{}:", img.width(), img.height());
    for id in [ModelId::RealEsrganX2, ModelId::RealEsrganX4] {
        let _ = esrgan::load(id, &Progress::new()).expect("load");
        let mut best = f64::MAX;
        for _ in 0..3 {
            let t = Instant::now();
            let _ = esrgan::upscale_image(&img, None, id, AlphaMode::Network, &Progress::new())
                .expect("upscale");
            best = best.min(t.elapsed().as_secs_f64());
        }
        line += &format!(" {id:?} {best:.2} s;");
    }
    eprintln!("{line} threads {}", fp_ai::nn::pool().current_num_threads());
}

/// A damaged model file of the right size: the checksum check names the file, loading reports a
/// readable error instead of a panic.
#[test]
#[ignore = "needs RealESRGAN_x2plus in the model folder; changes the model search of the process"]
fn ai_real_damaged_model_reports_error() {
    let Some(src) = models::find(ModelId::RealEsrganX2) else {
        eprintln!("RealEsrganX2 missing – skipped");
        return;
    };
    fp_ai::download::disable();
    cache::unload_all();
    let dir = fp_i18n::profile::unique_temp_dir("ai-real-damaged");
    let mut bytes = std::fs::read(&src).expect("read");
    bytes[..8].copy_from_slice(&u64::MAX.to_le_bytes());
    let file = models::info(ModelId::RealEsrganX2).file;
    std::fs::write(dir.join(file), &bytes).expect("write");
    drop(bytes);
    models::set_search_override(vec![dir.clone()]);
    let sum = fp_ai::download::verify(ModelId::RealEsrganX2, None);
    let img = photo("katze.jpg");
    let load = fp_i18n::with_language("en", || {
        esrgan::upscale_image(
            &img,
            None,
            ModelId::RealEsrganX2,
            AlphaMode::Network,
            &Progress::new(),
        )
        .map(|_| ())
    });
    models::set_search_override(Vec::new());
    let _ = std::fs::remove_dir_all(&dir);
    eprintln!("checksum: {sum:?}\nload: {load:?}");
    assert!(matches!(sum, Err(AiError::Checksum { .. })));
    assert!(matches!(load, Err(AiError::Weights(_))), "{load:?}");
}

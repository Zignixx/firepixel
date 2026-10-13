//! Qualitätsbilder der KI-Auswahl für `docs\screenshots\phase5_ki_*.png`.
//!
//! Je Testfoto ein Bild mit vier Zeilen (Spalten: Original | Maske | freigestellt auf Schachbrett):
//!
//! 1. Motiv auswählen (BiRefNet lite),
//! 2. Objektauswahl (SAM 2.1 Tiny, Klickpunkt als roter Ring),
//! 3. Kante verfeinern (BiRefNet lite Matting) auf dem Motiv aus Zeile 1,
//! 4. klassischer Zauberstab (Toleranz 32, gleicher Klickpunkt) zum Vergleich.
//!
//! Zeiten auf stdout.
//!
//! `cargo run --release -p fp-ai --example ai_images -- <testdata-ordner> <ausgabeordner>`

use fp_ai::models::ModelId;
use fp_ai::pipeline::{self, ObjectPrompt, PromptPoint};
use fp_ai::Progress;
use fp_raster::{Mask, RgbaImage};
use std::path::PathBuf;
use std::time::Instant;

const PANEL_W: usize = 420;

/// Zauberstab wie `select.magicWand`: 4er-Nachbarschaft, alle Kanäle innerhalb der Toleranz.
fn magic_wand(img: &RgbaImage, x: i32, y: i32, tol: i32) -> Mask {
    let b = img.bounds();
    let mut m = Mask::new(b).unwrap_or_else(|_| Mask::empty_at(0, 0));
    let Some(seed) = img.get(x, y) else { return m };
    let w = img.width();
    let mut seen = vec![false; img.width() * img.height()];
    let mut stack = vec![(x, y)];
    while let Some((cx, cy)) = stack.pop() {
        if !b.contains(cx, cy) {
            continue;
        }
        let i = (cy - b.top) as usize * w + (cx - b.left) as usize;
        if seen[i] {
            continue;
        }
        seen[i] = true;
        let Some(p) = img.get(cx, cy) else { continue };
        if (0..4).any(|k| (p[k] as i32 - seed[k] as i32).abs() > tol) {
            continue;
        }
        m.set(cx, cy, 255);
        stack.extend_from_slice(&[(cx + 1, cy), (cx - 1, cy), (cx, cy + 1), (cx, cy - 1)]);
    }
    m
}

struct Canvas {
    w: usize,
    h: usize,
    px: Vec<[u8; 3]>,
}

impl Canvas {
    fn put(&mut self, x0: usize, y0: usize, w: usize, h: usize, f: impl Fn(usize, usize) -> [u8; 3]) {
        for y in 0..h {
            for x in 0..w {
                if x0 + x < self.w && y0 + y < self.h {
                    self.px[(y0 + y) * self.w + x0 + x] = f(x, y);
                }
            }
        }
    }
}

fn sample(img: &RgbaImage, x: usize, y: usize, pw: usize, ph: usize) -> [u8; 4] {
    let sx = (x * img.width() / pw).min(img.width() - 1);
    let sy = (y * img.height() / ph).min(img.height() - 1);
    img.data()[sy * img.width() + sx]
}

fn mask_at(m: &Mask, img: &RgbaImage, x: usize, y: usize, pw: usize, ph: usize) -> u8 {
    let b = img.bounds();
    let sx = b.left + (x * img.width() / pw) as i32;
    let sy = b.top + (y * img.height() / ph) as i32;
    m.get(sx, sy)
}

fn row(c: &mut Canvas, y0: usize, img: &RgbaImage, m: &Mask, click: Option<(f32, f32)>) {
    let pw = PANEL_W;
    let ph = PANEL_W * img.height() / img.width();
    c.put(0, y0, pw, ph, |x, y| {
        let p = sample(img, x, y, pw, ph);
        if let Some((cx, cy)) = click {
            let (px, py) = (cx * pw as f32 / img.width() as f32, cy * ph as f32 / img.height() as f32);
            let d = ((x as f32 - px).powi(2) + (y as f32 - py).powi(2)).sqrt();
            if (5.0..8.0).contains(&d) {
                return [255, 0, 0];
            }
        }
        [p[0], p[1], p[2]]
    });
    c.put(pw + 4, y0, pw, ph, |x, y| {
        let v = mask_at(m, img, x, y, pw, ph);
        [v, v, v]
    });
    c.put(2 * pw + 8, y0, pw, ph, |x, y| {
        let p = sample(img, x, y, pw, ph);
        let a = mask_at(m, img, x, y, pw, ph) as u32;
        let chk = if ((x / 10) + (y / 10)) % 2 == 0 { 204u32 } else { 153 };
        let mix = |c: u8| ((c as u32 * a + chk * (255 - a)) / 255) as u8;
        [mix(p[0]), mix(p[1]), mix(p[2])]
    });
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let dir = PathBuf::from(a.get(1).cloned().unwrap_or_else(|| "crates/fp-ai/testdata".into()));
    let out = PathBuf::from(a.get(2).cloned().unwrap_or_else(|| "docs/screenshots".into()));
    let cases: &[(&str, &str, (f32, f32))] = &[
        ("person_haare.jpg", "phase5_ki_person.png", (700.0, 250.0)),
        ("hund_gras.jpg", "phase5_ki_hund.png", (230.0, 330.0)),
        ("katze.jpg", "phase5_ki_katze.png", (750.0, 500.0)),
        ("produkt_tasse.jpg", "phase5_ki_produkt.png", (720.0, 540.0)),
    ];
    let prog = Progress::new();
    for (file, name, click) in cases {
        let bytes = std::fs::read(dir.join(file)).expect("Testbild");
        let img = fp_io::jpeg_in::decode(&bytes).expect("JPEG").image;
        let t = Instant::now();
        let subject = pipeline::subject_mask(&img, None, ModelId::BiRefNetLite, &prog).expect("Motiv");
        let t_subject = t.elapsed().as_secs_f32();
        let prompt = ObjectPrompt { points: vec![PromptPoint { x: click.0, y: click.1, positive: true }], boxx: None };
        let t = Instant::now();
        let first = pipeline::object_mask(&img, None, &prompt, ModelId::Sam2Tiny, &prog).expect("Objekt");
        let t_first = t.elapsed().as_secs_f32();
        let t = Instant::now();
        let object = pipeline::object_mask(&img, None, &prompt, ModelId::Sam2Tiny, &prog).expect("Objekt");
        let t_click = t.elapsed().as_secs_f32();
        let t = Instant::now();
        let refined = pipeline::refine_edge(&img, &subject, 0.0, ModelId::BiRefNetLiteMatting, &prog).expect("Kante");
        let t_refine = t.elapsed().as_secs_f32();
        let t = Instant::now();
        let wand = magic_wand(&img, click.0 as i32, click.1 as i32, 32);
        let t_wand = t.elapsed().as_secs_f32();
        let _ = first;
        let ph = PANEL_W * img.height() / img.width();
        let mut c = Canvas { w: 3 * PANEL_W + 8, h: 4 * ph + 12, px: vec![[255, 255, 255]; (3 * PANEL_W + 8) * (4 * ph + 12)] };
        row(&mut c, 0, &img, &subject, None);
        row(&mut c, ph + 4, &img, &object.mask, Some(*click));
        row(&mut c, 2 * (ph + 4), &img, &refined, None);
        row(&mut c, 3 * (ph + 4), &img, &wand, Some(*click));
        let f = std::fs::File::create(out.join(name)).expect("PNG");
        let mut enc = png::Encoder::new(std::io::BufWriter::new(f), c.w as u32, c.h as u32);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        let mut wr = enc.write_header().expect("PNG-Kopf");
        let flat: Vec<u8> = c.px.iter().flat_map(|p| p.iter().copied()).collect();
        wr.write_image_data(&flat).expect("PNG-Daten");
        let soft = |m: &Mask| m.data().iter().filter(|v| **v > 0 && **v < 255).count();
        println!(
            "{file} ({}×{}): Motiv {t_subject:.2} s, Objekt erster Klick {t_first:.2} s / weiterer Klick {:.0} ms (Güte {:.2}), Kante {t_refine:.2} s, Zauberstab {:.0} ms; Halbtransparenz Motiv {} px, verfeinert {} px",
            img.width(),
            img.height(),
            t_click * 1000.0,
            object.iou,
            t_wand * 1000.0,
            soft(&subject),
            soft(&refined)
        );
        fp_ai::nn::trim();
    }
}

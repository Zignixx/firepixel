//! Selbst erzeugte Testanimationen für den GIF-Optimierer (deterministisch, ohne Dateien):
//! Bildschirmaufnahme, Foto-Kamerafahrt, Zeichentrick, Verlauf, Pixelart, Freisteller mit Alpha,
//! Szenenwechsel. Gemeinsam für `gifopt.rs` und `gifopt_measure.rs`.
#![allow(dead_code)]

use fp_batch::item::{Animation, Frame, Item, Meta};
use fp_raster::{IRect, RgbaImage};

pub type Px = [u8; 4];

pub struct Clip {
    pub name: &'static str,
    pub w: usize,
    pub h: usize,
    pub frames: Vec<Vec<Px>>,
    pub delay_ms: u32,
}

impl Clip {
    pub fn item(&self) -> Item {
        let frames = self
            .frames
            .iter()
            .map(|px| Frame { image: RgbaImage::from_raw(IRect::new(0, 0, self.w as i32, self.h as i32), px.clone()).expect("img"), delay_ms: self.delay_ms })
            .collect();
        let has_transparency = self.frames.iter().any(|f| f.iter().any(|p| p[3] < 255));
        Item::Animation(Animation { width: self.w as u32, height: self.h as u32, frames, loop_count: 0, meta: Meta { has_transparency, ..Meta::default() } })
    }
}

fn hash(x: u32, y: u32, s: u32) -> u32 {
    let mut h = x.wrapping_mul(374_761_393) ^ y.wrapping_mul(668_265_263) ^ s.wrapping_mul(2_246_822_519);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^ (h >> 16)
}

fn blend(a: Px, b: Px, t: f32) -> Px {
    let t = t.clamp(0.0, 1.0);
    [0, 1, 2, 3].map(|k| (a[k] as f32 * (1.0 - t) + b[k] as f32 * t).round() as u8)
}

fn fill_rect(px: &mut [Px], w: usize, x0: usize, y0: usize, x1: usize, y1: usize, c: Px) {
    let h = px.len() / w;
    for y in y0.min(h)..y1.min(h) {
        for x in x0.min(w)..x1.min(w) {
            px[y * w + x] = c;
        }
    }
}

/// Kreis mit Kantenglättung.
fn disc(px: &mut [Px], w: usize, cx: f32, cy: f32, r: f32, c: Px) {
    let h = px.len() / w;
    for y in 0..h {
        for x in 0..w {
            let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
            let cov = (r + 0.5 - d).clamp(0.0, 1.0);
            if cov > 0.0 {
                let i = y * w + x;
                px[i] = blend(px[i], c, cov);
            }
        }
    }
}

/// Bildschirmaufnahme: Fenster, Textzeilen, Zeiger, wachsende Eingabezeile, Fortschrittsbalken.
pub fn screen(w: usize, h: usize, n: usize) -> Clip {
    let frames = (0..n)
        .map(|f| {
            let mut px = vec![[236, 238, 242, 255]; w * h];
            fill_rect(&mut px, w, 0, 0, w, 14, [52, 101, 164, 255]);
            fill_rect(&mut px, w, 8, 22, w - 8, h - 8, [255, 255, 255, 255]);
            for line in 0..((h - 40) / 10) {
                let len = 20 + (hash(line as u32, 1, 3) as usize % (w - 60));
                for word in 0..len / 7 {
                    let x = 14 + word * 7;
                    fill_rect(&mut px, w, x, 28 + line * 10, x + 5, 33 + line * 10, [40, 40, 48, 255]);
                }
            }
            // Eingabezeile wächst
            let typed = (f * 4).min(w - 40);
            fill_rect(&mut px, w, 14, h - 20, 14 + typed, h - 14, [20, 20, 20, 255]);
            fill_rect(&mut px, w, 15 + typed, h - 21, 16 + typed, h - 13, [0, 0, 0, if f % 2 == 0 { 255 } else { 0 }]);
            // Fortschritt
            fill_rect(&mut px, w, w / 2, 4, w / 2 + (w / 3) * f / n.max(1), 10, [120, 200, 90, 255]);
            // Zeiger wandert
            let (cx, cy) = (20 + f * (w - 40) / n.max(1), 30 + (f * 3) % (h - 50));
            for k in 0..9 {
                fill_rect(&mut px, w, cx, cy + k, cx + k / 2 + 1, cy + k + 1, [0, 0, 0, 255]);
            }
            // transparentes Blinken der Schreibmarke nicht übernehmen (Bildschirm ist deckend)
            for p in px.iter_mut() {
                p[3] = 255;
            }
            px
        })
        .collect();
    Clip { name: "screen", w, h, frames, delay_ms: 100 }
}

/// Foto-Kamerafahrt: weiche Textur aus Wellen und Rauschen, je Frame 2 px verschoben.
pub fn pan(w: usize, h: usize, n: usize) -> Clip {
    let world = |x: f32, y: f32| -> Px {
        let v = (x * 0.05).sin() * 0.5 + (y * 0.07 + x * 0.013).cos() * 0.3 + ((x + y) * 0.21).sin() * 0.12;
        let noise = (hash(x as u32, y as u32, 9) % 17) as f32 - 8.0;
        let r = (120.0 + v * 80.0 + noise).clamp(0.0, 255.0);
        let g = (110.0 + (y * 0.6).min(90.0) + v * 40.0 + noise).clamp(0.0, 255.0);
        let b = (90.0 + (1.0 - v) * 60.0 + noise * 0.5).clamp(0.0, 255.0);
        [r as u8, g as u8, b as u8, 255]
    };
    let frames = (0..n).map(|f| (0..w * h).map(|i| world((i % w + 2 * f) as f32, (i / w) as f32)).collect()).collect();
    Clip { name: "pan", w, h, frames, delay_ms: 80 }
}

/// Zeichentrick: Himmel mit Verlauf, Hügel, springender Ball (geglättete Kanten).
pub fn cartoon(w: usize, h: usize, n: usize) -> Clip {
    let frames = (0..n)
        .map(|f| {
            let mut px: Vec<Px> = (0..w * h).map(|i| blend([90, 160, 230, 255], [200, 230, 255, 255], (i / w) as f32 / h as f32)).collect();
            disc(&mut px, w, w as f32 * 0.7, h as f32 * 1.6, h as f32 * 0.95, [70, 160, 60, 255]);
            disc(&mut px, w, w as f32 * 0.85, h as f32 * 0.2, 9.0, [255, 220, 60, 255]);
            let t = f as f32 / n as f32;
            let bx = 10.0 + t * (w as f32 - 20.0);
            let by = h as f32 * 0.75 - (t * std::f32::consts::PI * 3.0).sin().abs() * h as f32 * 0.5;
            disc(&mut px, w, bx, by, 8.0, [220, 50, 50, 255]);
            disc(&mut px, w, bx - 2.5, by - 2.5, 2.5, [255, 160, 160, 255]);
            px
        })
        .collect();
    Clip { name: "cartoon", w, h, frames, delay_ms: 60 }
}

/// Wenige flache Farben ohne Kantenglättung (verlustfrei kodierbar), mit Bewegung.
pub fn pixelart(w: usize, h: usize, n: usize) -> Clip {
    let pal: [Px; 6] = [[20, 20, 30, 255], [200, 60, 40, 255], [250, 200, 80, 255], [60, 140, 220, 255], [240, 240, 240, 255], [40, 160, 70, 255]];
    let frames = (0..n)
        .map(|f| {
            (0..w * h)
                .map(|i| {
                    let (x, y) = (i % w, i / w);
                    if (x + f * 2) % 16 < 3 && y > h / 2 {
                        pal[5]
                    } else if x.abs_diff(f * 3 % w) < 4 && y.abs_diff(h / 3) < 4 {
                        pal[1]
                    } else {
                        pal[(x / 8 + y / 8) % 4]
                    }
                })
                .collect()
        })
        .collect();
    Clip { name: "pixelart", w, h, frames, delay_ms: 100 }
}

/// Freisteller: Figur bewegt sich auf transparentem Grund, Alpha nur 0/255.
pub fn cutout(w: usize, h: usize, n: usize) -> Clip {
    let frames = (0..n)
        .map(|f| {
            let mut px = vec![[0, 0, 0, 0]; w * h];
            let x0 = (f * 5) % (w - 12);
            fill_rect(&mut px, w, x0, h / 3, x0 + 12, h / 3 + 12, [230, 80, 30, 255]);
            fill_rect(&mut px, w, 2, 2, 8, 8, [10, 10, 200, 255]); // statisch
            px
        })
        .collect();
    Clip { name: "cutout", w, h, frames, delay_ms: 100 }
}

/// Szenenwechsel: erst rötliche, dann bläuliche Verläufe mit vielen Farben.
pub fn scene_change(w: usize, h: usize, n: usize) -> Clip {
    let frames = (0..n)
        .map(|f| {
            let blue = f >= n / 2;
            (0..w * h)
                .map(|i| {
                    let (x, y) = ((i % w) as f32 / w as f32, (i / w) as f32 / h as f32);
                    let a = (x * 255.0) as u8;
                    let b = (y * 255.0) as u8;
                    if (i % w).abs_diff(f * 5 % w) < 3 {
                        [250, 250, 250, 255]
                    } else if blue {
                        [b / 4, a / 2, 160 + b / 3, 255]
                    } else {
                        [160 + a / 3, b / 3, a / 5, 255]
                    }
                })
                .collect()
        })
        .collect();
    Clip { name: "scene", w, h, frames, delay_ms: 100 }
}

/// Decodiert GIF-Bytes zu Vollbildern (RGBA) mit Dauer.
pub fn decode(bytes: &[u8]) -> (usize, usize, Vec<(Vec<Px>, u32)>) {
    match fp_batch::item::decode_bytes("x.gif", bytes).expect("decode") {
        Item::Animation(a) => (a.width as usize, a.height as usize, a.frames.iter().map(|f| (f.image.data().to_vec(), f.delay_ms)).collect()),
        Item::Still(s) => (s.image.width(), s.image.height(), vec![(s.image.data().to_vec(), 0)]),
        other => panic!("{:?}", other.kind()),
    }
}

/// Vollbilder in der Zeit ausrollen (je 10 ms ein Eintrag), damit zusammengelegte Frames vergleichbar sind.
pub fn timeline(frames: &[(Vec<Px>, u32)], fallback_ms: u32) -> Vec<&Vec<Px>> {
    let mut v = Vec::new();
    for (px, d) in frames {
        let d = if *d == 0 { fallback_ms } else { *d };
        for _ in 0..(d / 10).max(1) {
            v.push(px);
        }
    }
    v
}

/// Mittlere SSIM über die Zeit (Quelle vs. Ergebnis).
pub fn mean_ssim(clip: &Clip, bytes: &[u8]) -> f64 {
    use fp_batch::steps::gifopt::metrics::ssim;
    let (_, _, dec) = decode(bytes);
    let src: Vec<(Vec<Px>, u32)> = clip.frames.iter().map(|f| (f.clone(), clip.delay_ms)).collect();
    let a = timeline(&src, clip.delay_ms);
    let b = timeline(&dec, clip.delay_ms);
    let n = a.len().min(b.len());
    let step = (n / 12).max(1);
    let (mut s, mut c) = (0.0, 0.0);
    for i in (0..n).step_by(step) {
        s += ssim(a[i], b[i], clip.w, clip.h);
        c += 1.0;
    }
    s / c
}

/// Mittlere Farbtreue in der Fläche über die Zeit ([`fp_batch::steps::gifopt::metrics::color_error`]).
pub fn mean_color_error(clip: &Clip, bytes: &[u8]) -> f64 {
    use fp_batch::steps::gifopt::metrics::color_error;
    let (_, _, dec) = decode(bytes);
    let src: Vec<(Vec<Px>, u32)> = clip.frames.iter().map(|f| (f.clone(), clip.delay_ms)).collect();
    let a = timeline(&src, clip.delay_ms);
    let b = timeline(&dec, clip.delay_ms);
    let n = a.len().min(b.len());
    let step = (n / 12).max(1);
    let (mut s, mut c) = (0.0, 0.0);
    for i in (0..n).step_by(step) {
        s += color_error(a[i], b[i], clip.w, clip.h);
        c += 1.0;
    }
    s / c
}

/// SSIM über **alle** Zeitpunkte (je 10 ms), nicht nur 12 Stichproben – zeigt auch kurze
/// Zwischenzustände (z. B. aufgeteilte Frames).
pub fn full_ssim(clip: &Clip, bytes: &[u8]) -> f64 {
    use fp_batch::steps::gifopt::metrics::ssim;
    let (_, _, dec) = decode(bytes);
    let src: Vec<(Vec<Px>, u32)> = clip.frames.iter().map(|f| (f.clone(), clip.delay_ms)).collect();
    let a = timeline(&src, clip.delay_ms);
    let b = timeline(&dec, clip.delay_ms);
    let n = a.len().min(b.len());
    let (mut s, mut c) = (0.0, 0.0);
    // gleiche Bildpaare hintereinander nur einmal rechnen
    let mut last: Option<(&Vec<Px>, &Vec<Px>, f64)> = None;
    for i in 0..n {
        let v = match last {
            Some((x, y, v)) if std::ptr::eq(x, a[i]) && std::ptr::eq(y, b[i]) => v,
            _ => ssim(a[i], b[i], clip.w, clip.h),
        };
        last = Some((a[i], b[i], v));
        s += v;
        c += 1.0;
    }
    s / c
}

/// Mittlere PSNR über die Zeit.
pub fn mean_psnr(clip: &Clip, bytes: &[u8]) -> f64 {
    use fp_batch::steps::gifopt::metrics::psnr;
    let (_, _, dec) = decode(bytes);
    let src: Vec<(Vec<Px>, u32)> = clip.frames.iter().map(|f| (f.clone(), clip.delay_ms)).collect();
    let a = timeline(&src, clip.delay_ms);
    let b = timeline(&dec, clip.delay_ms);
    let n = a.len().min(b.len());
    let step = (n / 12).max(1);
    let (mut s, mut c) = (0.0, 0.0);
    for i in (0..n).step_by(step) {
        s += psnr(a[i], b[i]);
        c += 1.0;
    }
    s / c
}

//! „Farbe ersetzen“ und „Rote Augen entfernen“ nach den Messungen am Original (PAR-X-TOOLS-2,
//! `docs\parity\tool-stamp-replace-redeye.md` 07/08/11/12). Aufgerufen von `bitmap.retouch`
//! (Modi `replaceColor` und `redEye`).

use fp_color::Rgba8;
use fp_geom::{IRect, Point};
use fp_raster::{Mask, RgbaImage};

/// Werte von „Farbe ersetzen“.
#[derive(Clone, Debug)]
pub(crate) struct ReplaceParams {
    pub size: f32,
    pub round: bool,
    pub from: Rgba8,
    pub to: Rgba8,
    /// Toleranz in Prozent von 256 (Inspektor 0..255, Vorgabe 32).
    pub tolerance: f32,
    /// Stärke 0..255 (Vorgabe 200).
    pub strength: f32,
    pub colorize: bool,
}

/// Pixel, die eine harte Spitze entlang des Strichs überdeckt: Mitte des Pixels höchstens
/// Größe/2 von der Pixelmitte des Strichpunkts (gemessen: Größe 13 → 13 Zeilen, rundes Strichende ab
/// x 364 bei Start 370).
pub(crate) fn hard_stroke(pts: &[Point], size: f32, round: bool, bounds: IRect) -> Vec<(i32, i32)> {
    let r = (size / 2.0).max(0.5);
    let reach = r.ceil() as i32;
    let mut centers: Vec<(i32, i32)> = Vec::new();
    let mut push = |p: Point| {
        let c = (p.x.floor() as i32, p.y.floor() as i32);
        if centers.last() != Some(&c) {
            centers.push(c);
        }
    };
    if let Some(f) = pts.first() {
        push(*f);
    }
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let n = ((b.x - a.x).abs().max((b.y - a.y).abs()) * 2.0)
            .ceil()
            .max(1.0) as i32;
        for i in 1..=n {
            let t = i as f32 / n as f32;
            push(Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t));
        }
    }
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for (cx, cy) in centers {
        for dy in -reach..=reach {
            for dx in -reach..=reach {
                let d = if round {
                    (dx as f32).hypot(dy as f32)
                } else {
                    (dx.abs().max(dy.abs())) as f32
                };
                if d > r {
                    continue;
                }
                let (x, y) = (cx + dx, cy + dy);
                if x < bounds.left || y < bounds.top || x >= bounds.right || y >= bounds.bottom {
                    continue;
                }
                if seen.insert((x, y)) {
                    out.push((x, y));
                }
            }
        }
    }
    out
}

fn rgb_hsv(p: [f32; 3]) -> (f32, f32, f32) {
    let (r, g, b) = (p[0], p[1], p[2]);
    let mx = r.max(g).max(b);
    let mn = r.min(g).min(b);
    let d = mx - mn;
    let h = if d <= 0.0 {
        0.0
    } else if mx == r {
        60.0 * ((g - b) / d).rem_euclid(6.0)
    } else if mx == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    (h, if mx > 0.0 { d / mx } else { 0.0 }, mx)
}

fn hsv_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
    let c = v * s;
    let hp = h.rem_euclid(360.0) / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match hp as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    [r + m, g + m, b + m]
}

/// Farbe ersetzen: Jedes überdeckte Pixel bekommt den Farbton der In-Farbe (Sättigung und Helligkeit
/// bleiben; Kolorieren: Farbton und Sättigung der In-Farbe), gemischt mit
/// `Stärke/128 · (1 − Abstand/(Toleranz · 2,56))` (Abstand = größte Kanalabweichung von der
/// Von-Farbe). Gemessen: Rot → Blau ergibt #0000FE (Kolorieren #0000FF); Von #8E8881 bei Toleranz
/// 32 voll bis Abstand ≈ 29, null ab ≈ 82, bei 100 voll bis ≈ 90.
pub(crate) fn replace_color(
    img: &mut RgbaImage,
    pts: &[Point],
    prm: &ReplaceParams,
    sel: Option<&Mask>,
) -> bool {
    let cells = hard_stroke(pts, prm.size, prm.round, img.bounds());
    let from = [prm.from.r as f32, prm.from.g as f32, prm.from.b as f32];
    let (th, ts, _) = rgb_hsv([prm.to.r as f32, prm.to.g as f32, prm.to.b as f32]);
    let gain = prm.strength / 128.0;
    let span = prm.tolerance.max(0.0) * 2.56;
    let mut any = false;
    for (x, y) in cells {
        let Some(px) = img.get(x, y) else { continue };
        if px[3] == 0 {
            continue;
        }
        let c = [px[0] as f32, px[1] as f32, px[2] as f32];
        let m = (0..3)
            .map(|k| (c[k] - from[k]).abs())
            .fold(0.0f32, f32::max);
        let mut f = if span <= 0.0 {
            if m <= 0.0 { gain } else { 0.0 }
        } else {
            gain * (1.0 - m / span)
        };
        if let Some(s) = sel {
            f *= s.get(x, y) as f32 / 255.0;
        }
        let f = f.clamp(0.0, 1.0);
        if f <= 0.0 {
            continue;
        }
        let (_, s, v) = rgb_hsv(c);
        let full = if prm.colorize {
            hsv_rgb(th, ts, v)
        } else {
            hsv_rgb(th, s, v * 254.0 / 255.0)
        };
        let out = [0, 1, 2].map(|k| (c[k] + (full[k] - c[k]) * f).round().clamp(0.0, 255.0) as u8);
        if out != [px[0], px[1], px[2]] {
            img.set_pixel(x, y, Rgba8::new(out[0], out[1], out[2], px[3]));
            any = true;
        }
    }
    any
}

/// Werte von „Rote Augen entfernen“.
pub(crate) struct RedEyeParams {
    /// Aufgezogenes Rechteck (Dokumentpixel, rechts/unten exklusiv).
    pub rect: IRect,
    /// Toleranz 0..100 (Vorgabe 50).
    pub tolerance: f32,
    /// Stärke 0..100 (Vorgabe 50).
    pub strength: f32,
    /// Farbtonbereich, Mindestsättigung/-helligkeit (Voreinstellungen `RedEyeRemovalTool…`).
    pub min_hue: f32,
    pub max_hue: f32,
    pub min_sat: f32,
    pub min_lum: f32,
}

fn is_red(c: [f32; 3], p: &RedEyeParams) -> bool {
    let (h, s, v) = rgb_hsv(c);
    let hue_ok = if p.min_hue > p.max_hue {
        h >= p.min_hue || h <= p.max_hue
    } else {
        h >= p.min_hue && h <= p.max_hue
    };
    hue_ok && s >= p.min_sat && v / 255.0 >= p.min_lum
}

/// Rote Augen entfernen im aufgezogenen Rechteck (gemessen 05/11–12): rote Pixel (Farbton,
/// Sättigung, Helligkeit wie die Voreinstellungen) werden abgedunkelt – Rot Richtung Mittel aus Grün
/// und Blau, Grün leicht angehoben –, ihr Rand (1 px; ab Toleranz > 75 auch diagonal) halb.
/// Toleranz bis 50 ändert nichts (10 = 50), darüber geht es zum dunklen Grau (100 → #1F1F1F).
/// Rückgabe: ob sich ein Pixel geändert hat.
pub(crate) fn red_eye(img: &mut RgbaImage, prm: &RedEyeParams, sel: Option<&Mask>) -> bool {
    let r = prm.rect.intersect(&img.bounds());
    if r.is_empty() || prm.rect.width() < 1 || prm.rect.height() < 1 {
        return false;
    }
    let (w, h) = (r.width() as usize, r.height() as usize);
    let idx = |x: i32, y: i32| (y - r.top) as usize * w + (x - r.left) as usize;
    let mut core = vec![false; w * h];
    for y in r.top..r.bottom {
        for x in r.left..r.right {
            if let Some(px) = img.get(x, y)
                && px[3] > 0
                && is_red([px[0] as f32, px[1] as f32, px[2] as f32], prm)
            {
                core[idx(x, y)] = true;
            }
        }
    }
    let diag = prm.tolerance > 75.0;
    // Stärke wirkt stetig, aber nicht linear: gemessen macht Stärke 50 die Pupille #DC141E zu
    // #4E1C1E (Rot um 73 % Richtung Grün/Blau-Mittel); k = 1 − (1 − s)^1,88 trifft das (CT-B-05,
    // 05/11). 0 = keine Wirkung, 100 = Rot ganz auf das Grün/Blau-Mittel.
    let s = 1.0 - (1.0 - (prm.strength / 100.0).clamp(0.0, 1.0)).powf(1.88);
    let dark = ((prm.tolerance - 50.0) / 50.0).clamp(0.0, 1.0);
    let fix = |c: [f32; 3]| -> [f32; 3] {
        let gb = (c[1] + c[2]) / 2.0;
        let mut o = [
            c[0] - s * (c[0] - gb),
            c[1] + s * (c[2] - c[1]).max(0.0),
            c[2],
        ];
        if dark > 0.0 {
            let g = c[1].max(c[2]) + 1.0;
            o = o.map(|v| v + (g - v) * dark);
        }
        o
    };
    let mut any = false;
    let src = img.clone();
    for y in r.top..r.bottom {
        for x in r.left..r.right {
            let Some(px) = src.get(x, y) else { continue };
            let k = if core[idx(x, y)] {
                1.0
            } else {
                let at = |dx: i32, dy: i32| {
                    let (nx, ny) = (x + dx, y + dy);
                    nx >= r.left
                        && ny >= r.top
                        && nx < r.right
                        && ny < r.bottom
                        && core[idx(nx, ny)]
                };
                let n4 = at(1, 0) || at(-1, 0) || at(0, 1) || at(0, -1);
                let n8 = diag && (at(1, 1) || at(-1, -1) || at(1, -1) || at(-1, 1));
                if n4 || n8 { 0.5 } else { continue }
            };
            let k = k * sel.map(|m| m.get(x, y) as f32 / 255.0).unwrap_or(1.0);
            if k <= 0.0 {
                continue;
            }
            let c = [px[0] as f32, px[1] as f32, px[2] as f32];
            let f = fix(c);
            let out = [0, 1, 2].map(|i| (c[i] + (f[i] - c[i]) * k).round().clamp(0.0, 255.0) as u8);
            if out != [px[0], px[1], px[2]] {
                img.set_pixel(x, y, Rgba8::new(out[0], out[1], out[2], px[3]));
                any = true;
            }
        }
    }
    any
}

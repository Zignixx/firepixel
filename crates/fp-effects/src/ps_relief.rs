//! Satin und Abschrägung/Relief der Photoshop-Live-Effekte (05 §9.5). Das Original rechnet beide in
//! der PS-Bibliothek; die Verfahren hier sind an eigenen Laufzeitdateien vermessen
//! (`re\_testdateien\phase5\pseffekte\ps_bevel`, `ps_bevel2`, `ps_satin`, Erzeugung mit den
//! gleichnamigen JSF-Skripten; Prüfung `fp-render\tests\ps_relief_fit.rs`), Status A (empirisch):
//!
//! **Satin**: Objekt-Alpha um `o = (round(d·cos θ), round(−d·sin θ))` und `−o` verschoben, je mit dem
//! Dreiecksfilter `r = max(1, Größe − 1)` weichgezeichnet, **jede Kopie** durch die Kontur
//! (`mappingShape`, natürlicher kubischer Spline durch die Stützpunkte), Betrag der Differenz,
//! optional umgekehrt, mit dem Objekt-Alpha beschnitten; Deckkraft und Mischmodus wie die übrigen
//! inneren Effekte. 14 Fälle (Abstand, Größe 0–14, Winkel, Umkehren, Kontur, Modus) je höchstens ±1.
//!
//! **Abschrägung/Relief**: Höhenfeld aus dem Objekt-Alpha, Lambert-Beleuchtung mit der Normalen
//! `(gx, gy, 1)` (Zentraldifferenzen) und dem Licht `(cos h·cos θ, −cos h·sin θ, sin h)`
//! (θ auf ±180° begrenzt); Licht `max(0, (s − sin h)/(1 − sin h))`, Schatten `max(0, (sin h − s)/sin h)`.
//! - abrunden: Dreiecksfilter `r = max(1, Größe − 1)` des Alphas, Steigung · `max(Tiefe·(r+1), 1)`;
//! - meißeln hart/weich: Höhe = vorzeichenbehafteter Abstand / Größe, Steigung · `0,4·Tiefe·Größe`
//!   (weich: Außenkante steiler); Abstand als Schachbrettmaß;
//! - Relief/Kissenrelief: halbe Größe (aufgerundet), Kissen außen mit umgekehrter Höhe;
//!   Kontur-Relief braucht eine Kontur und bleibt ohne Wirkung (wie T4);
//! - Weichzeichnen: Licht- und Schattenfeld mit `r = max(1, Weichzeichnen − 1)`;
//! - innen: Schatten, dann Licht mit Modus/Farbe/Deckkraft ins Objekt; außen: Farbschicht hinter dem
//!   Objekt. Abrunden innen/außen, Relief, Kissen je höchstens ±1…±8, meißeln ±5…±10, meißeln außen
//!   gröber (Ecken). Glanzkontur, Struktur und Kontur der Abschrägung werden nicht ausgewertet.

use fp_geom::IRect;
use fp_raster::RgbaImage;

use crate::photoshop::{PsRecord, inner_mix};
use crate::util::{new_image, on_frame};

/// Dreiecksfilter mit Gewichten `(r + 1 − |j|)/(r + 1)²`, getrennt, außerhalb 0.
fn tri_blur(v: &[f64], w: usize, h: usize, r: usize) -> Vec<f64> {
    if r == 0 {
        return v.to_vec();
    }
    let n = ((r + 1) * (r + 1)) as f64;
    let k: Vec<f64> = (0..=2 * r)
        .map(|j| (r + 1 - j.abs_diff(r)) as f64 / n)
        .collect();
    let mut tmp = vec![0.0; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut s = 0.0;
            for (j, kj) in k.iter().enumerate() {
                let xi = x as isize + j as isize - r as isize;
                if xi >= 0 && (xi as usize) < w {
                    s += kj * v[y * w + xi as usize];
                }
            }
            tmp[y * w + x] = s;
        }
    }
    let mut out = vec![0.0; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut s = 0.0;
            for (j, kj) in k.iter().enumerate() {
                let yi = y as isize + j as isize - r as isize;
                if yi >= 0 && (yi as usize) < h {
                    s += kj * tmp[yi as usize * w + x];
                }
            }
            out[y * w + x] = s;
        }
    }
    out
}

/// Zentraldifferenzen (am Rand einseitig).
fn gradient(v: &[f64], w: usize, h: usize) -> (Vec<f64>, Vec<f64>) {
    let at = |x: usize, y: usize| v[y * w + x];
    let mut gx = vec![0.0; w * h];
    let mut gy = vec![0.0; w * h];
    for y in 0..h {
        for x in 0..w {
            gx[y * w + x] = if w < 2 {
                0.0
            } else if x == 0 {
                at(1, y) - at(0, y)
            } else if x == w - 1 {
                at(x, y) - at(x - 1, y)
            } else {
                (at(x + 1, y) - at(x - 1, y)) / 2.0
            };
            gy[y * w + x] = if h < 2 {
                0.0
            } else if y == 0 {
                at(x, 1) - at(x, 0)
            } else if y == h - 1 {
                at(x, y) - at(x, y - 1)
            } else {
                (at(x, y + 1) - at(x, y - 1)) / 2.0
            };
        }
    }
    (gx, gy)
}

/// Verschiebung um (dx, dy): `out[y][x] = v[y − dy][x − dx]`.
fn shifted(v: &[f64], w: usize, h: usize, dx: i32, dy: i32) -> Vec<f64> {
    let mut out = vec![0.0; w * h];
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let (sx, sy) = (x - dx, y - dy);
            if sx >= 0 && sy >= 0 && (sx as usize) < w && (sy as usize) < h {
                out[y as usize * w + x as usize] = v[sy as usize * w + sx as usize];
            }
        }
    }
    out
}

/// Schachbrett-Abstand der gesetzten Pixel zum nächsten ungesetzten (Randpixel = 1, ungesetzt = 0).
fn chamfer(set: &[bool], w: usize, h: usize) -> Vec<f64> {
    const INF: u32 = u32::MAX / 2;
    let mut d: Vec<u32> = set.iter().map(|&s| if s { INF } else { 0 }).collect();
    let idx = |x: usize, y: usize| y * w + x;
    for y in 0..h {
        for x in 0..w {
            if d[idx(x, y)] == 0 {
                continue;
            }
            let mut v = d[idx(x, y)];
            if y > 0 {
                v = v.min(d[idx(x, y - 1)] + 1);
                if x > 0 {
                    v = v.min(d[idx(x - 1, y - 1)] + 1);
                }
                if x + 1 < w {
                    v = v.min(d[idx(x + 1, y - 1)] + 1);
                }
            }
            if x > 0 {
                v = v.min(d[idx(x - 1, y)] + 1);
            }
            d[idx(x, y)] = v;
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            if d[idx(x, y)] == 0 {
                continue;
            }
            let mut v = d[idx(x, y)];
            if y + 1 < h {
                v = v.min(d[idx(x, y + 1)] + 1);
                if x + 1 < w {
                    v = v.min(d[idx(x + 1, y + 1)] + 1);
                }
                if x > 0 {
                    v = v.min(d[idx(x - 1, y + 1)] + 1);
                }
            }
            if x + 1 < w {
                v = v.min(d[idx(x + 1, y)] + 1);
            }
            d[idx(x, y)] = v;
        }
    }
    d.into_iter()
        .map(|v| if v >= INF { 1e6 } else { v as f64 })
        .collect()
}

/// Kontur (`N$name$h;v;c$…`, 05 §9.3) als natürlicher kubischer Spline über 0–255.
#[derive(Debug, Clone)]
pub(crate) struct Contour {
    xs: Vec<f64>,
    ys: Vec<f64>,
    c: Vec<f64>,
}

impl Contour {
    pub(crate) fn parse(spec: &str) -> Option<Contour> {
        let mut pts: Vec<(f64, f64)> = spec
            .split('$')
            .skip(2)
            .filter_map(|q| {
                let mut it = q.split(';');
                let x = it.next()?.trim().parse::<f64>().ok()?;
                let y = it.next()?.trim().parse::<f64>().ok()?;
                (x.is_finite() && y.is_finite()).then_some((x, y))
            })
            .collect();
        pts.sort_by(|a, b| a.0.total_cmp(&b.0));
        pts.dedup_by(|a, b| a.0 == b.0);
        if pts.len() < 2 {
            return None;
        }
        let xs: Vec<f64> = pts.iter().map(|p| p.0).collect();
        let ys: Vec<f64> = pts.iter().map(|p| p.1).collect();
        let n = xs.len();
        let h: Vec<f64> = xs.windows(2).map(|w| w[1] - w[0]).collect();
        // natürlicher Spline: tridiagonales System (Thomas-Verfahren) für die zweiten Koeffizienten
        let mut c = vec![0.0; n];
        if n > 2 {
            let m = n - 2;
            let mut diag = vec![0.0; m];
            let mut rhs = vec![0.0; m];
            for i in 1..n - 1 {
                diag[i - 1] = 2.0 * (h[i - 1] + h[i]);
                rhs[i - 1] = 3.0 * ((ys[i + 1] - ys[i]) / h[i] - (ys[i] - ys[i - 1]) / h[i - 1]);
            }
            for i in 1..m {
                let f = h[i] / diag[i - 1];
                diag[i] -= f * h[i];
                rhs[i] -= f * rhs[i - 1];
            }
            for i in (0..m).rev() {
                let next = if i + 1 < m { h[i + 1] * c[i + 2] } else { 0.0 };
                c[i + 1] = (rhs[i] - next) / diag[i];
            }
        }
        Some(Contour { xs, ys, c })
    }

    /// Wert für `v` in 0–1, Ergebnis 0–1.
    pub(crate) fn eval(&self, v: f64) -> f64 {
        let x = v * 255.0;
        let n = self.xs.len();
        let i = match self.xs.iter().position(|&xi| xi >= x) {
            Some(0) => 0,
            Some(p) => p - 1,
            None => n - 2,
        }
        .min(n - 2);
        let hh = self.xs[i + 1] - self.xs[i];
        let dx = x - self.xs[i];
        let (y0, y1, c0, c1) = (self.ys[i], self.ys[i + 1], self.c[i], self.c[i + 1]);
        let b = (y1 - y0) / hh - hh * (2.0 * c0 + c1) / 3.0;
        let d = (c1 - c0) / (3.0 * hh);
        ((y0 + b * dx + c0 * dx * dx + d * dx * dx * dx) / 255.0).clamp(0.0, 1.0)
    }
}

fn round_half_up(v: f64) -> i32 {
    if v.is_finite() {
        (v + 0.5).floor().clamp(-1e6, 1e6) as i32
    } else {
        0
    }
}

/// Objekt-Alpha (0–1) über `area`.
fn alpha01(obj: &RgbaImage, area: IRect) -> Vec<f64> {
    on_frame(obj, area)
        .data()
        .iter()
        .map(|p| p[3] as f64 / 255.0)
        .collect()
}

/// Satin ins Objekt (Alpha bleibt).
pub(crate) fn satin(img: &mut RgbaImage, obj: &RgbaImage, rec: &PsRecord) {
    let b = obj.bounds();
    let dist = rec.num("distance", 0.0).clamp(0.0, 1000.0);
    let th = rec.num("localLightingAngle", 19.0).to_radians();
    let (ox, oy) = (
        round_half_up(dist * th.cos()),
        round_half_up(-dist * th.sin()),
    );
    let size = rec.num("blur", 0.0).clamp(0.0, 1000.0);
    let r = if size > 0.0 {
        ((size as usize).saturating_sub(1)).max(1)
    } else {
        0
    };
    let m = r as i32 + ox.abs().max(oy.abs()) + 2;
    let area = IRect::new(b.left - m, b.top - m, b.right + m, b.bottom + m);
    let (w, h) = (area.width() as usize, area.height() as usize);
    let a = alpha01(obj, area);
    let b1 = tri_blur(&shifted(&a, w, h, ox, oy), w, h, r);
    let b2 = tri_blur(&shifted(&a, w, h, -ox, -oy), w, h, r);
    let contour = rec.get("mappingShape").and_then(Contour::parse);
    let map = |v: f64| {
        contour
            .as_ref()
            .map(|c| c.eval(v.clamp(0.0, 1.0)))
            .unwrap_or(v.clamp(0.0, 1.0))
    };
    let inv = rec.flag("invert");
    let op = rec.num("opacity", 50.0).clamp(0.0, 100.0) / 100.0;
    let c = rec.color("color").map(|c| c.rgb()).unwrap_or([0, 0, 0]);
    let mode = rec.get("mode").unwrap_or("multiply").to_string();
    let iw = img.width();
    let ib = img.bounds();
    for (i, p) in img.data_mut().iter_mut().enumerate() {
        if p[3] == 0 {
            continue;
        }
        let (x, y) = (ib.left + (i % iw) as i32, ib.top + (i / iw) as i32);
        if !area.contains(x, y) {
            continue;
        }
        let k = (y - area.top) as usize * w + (x - area.left) as usize;
        let mut s = (map(b1[k]) - map(b2[k])).abs();
        if inv {
            s = 1.0 - s;
        }
        let al = (s * a[k] * op * 255.0 + 0.5).floor().clamp(0.0, 255.0) as u8;
        if al > 0 {
            *p = inner_mix(*p, c, al, &mode);
        }
    }
}

/// Abschrägung/Relief: verändert das Objekt (innen) und liefert die Schicht außerhalb (hinter dem
/// Objekt), falls der Stil eine hat.
pub(crate) fn bevel(img: &mut RgbaImage, obj: &RgbaImage, rec: &PsRecord) -> Option<RgbaImage> {
    let style = rec.get("bevelStyle").unwrap_or("innerBevel");
    if style == "strokeEmboss" {
        return None;
    }
    let tech = rec.get("bevelTechnique").unwrap_or("softMatte");
    let size_raw = rec.num("blur", 5.0).clamp(0.0, 250.0);
    let half = matches!(style, "emboss" | "pillowEmboss");
    let size = if half {
        (size_raw / 2.0).ceil()
    } else {
        size_raw.floor()
    }
    .max(1.0);
    let r = ((size as usize).saturating_sub(1)).max(1);
    let depth = rec.num("strengthRatio", 100.0).clamp(1.0, 1000.0) / 100.0;
    let soft = rec.num("softness", 0.0).clamp(0.0, 16.0);
    let rs = if soft > 0.0 {
        ((soft as usize).saturating_sub(1)).max(1)
    } else {
        0
    };
    let b = obj.bounds();
    let m = (size as i32) + r as i32 + rs as i32 + 3;
    let area = IRect::new(b.left - m, b.top - m, b.right + m, b.bottom + m);
    let (w, h) = (area.width() as usize, area.height() as usize);
    let a = alpha01(obj, area);
    let (hmap, gain) = if tech == "softMatte" {
        (tri_blur(&a, w, h, r), (depth * (r + 1) as f64).max(1.0))
    } else {
        let inside: Vec<bool> = a.iter().map(|&v| v >= 0.5).collect();
        let outside: Vec<bool> = inside.iter().map(|&v| !v).collect();
        let din = chamfer(&inside, w, h);
        let dout = chamfer(&outside, w, h);
        let edge = if tech == "slopeLimitMatte" { 0.6 } else { 0.0 };
        let hv = (0..w * h)
            .map(|k| {
                let sd = if inside[k] {
                    din[k] - 1.0
                } else {
                    -dout[k] - edge
                };
                (sd / size).clamp(-1.0, 1.0)
            })
            .collect();
        (hv, 0.4 * depth * size)
    };
    let sign = if rec.get("bevelDirection") == Some("stampIn") {
        -1.0
    } else {
        1.0
    };
    let (gx, gy) = gradient(&hmap, w, h);
    let angle = if rec.flag("useGlobalAngle") {
        120.0
    } else {
        rec.num("localLightingAngle", 120.0)
    }
    .clamp(-180.0, 180.0);
    let (th, alt) = (
        angle.to_radians(),
        rec.num("localLightingAltitude", 30.0)
            .clamp(0.0, 90.0)
            .to_radians(),
    );
    let l = (alt.cos() * th.cos(), -alt.cos() * th.sin(), alt.sin());
    let flat = alt.sin();
    let light = |k: usize, s_sign: f64| -> (f64, f64) {
        let (x, y) = (gx[k] * gain * s_sign, gy[k] * gain * s_sign);
        let s = (x * l.0 + y * l.1 + l.2) / (x * x + y * y + 1.0).sqrt();
        let hi = if flat < 1.0 - 1e-9 {
            ((s - flat) / (1.0 - flat)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let sh = if flat > 1e-9 {
            ((flat - s) / flat).clamp(0.0, 1.0)
        } else {
            0.0
        };
        (hi, sh)
    };
    let mut hi = vec![0.0; w * h];
    let mut sh = vec![0.0; w * h];
    let mut hi_o = vec![0.0; w * h];
    let mut sh_o = vec![0.0; w * h];
    let pillow = style == "pillowEmboss";
    for k in 0..w * h {
        (hi[k], sh[k]) = light(k, sign);
        (hi_o[k], sh_o[k]) = if pillow {
            light(k, -sign)
        } else {
            (hi[k], sh[k])
        };
    }
    if rs > 0 {
        hi = tri_blur(&hi, w, h, rs);
        sh = tri_blur(&sh, w, h, rs);
        hi_o = tri_blur(&hi_o, w, h, rs);
        sh_o = tri_blur(&sh_o, w, h, rs);
    }
    let hop = rec.num("highlightOpacity", 75.0).clamp(0.0, 100.0) / 100.0;
    let sop = rec.num("shadowOpacity", 75.0).clamp(0.0, 100.0) / 100.0;
    let hc = rec
        .color("highlightColor")
        .map(|c| c.rgb())
        .unwrap_or([255, 255, 255]);
    let sc = rec
        .color("shadowColor")
        .map(|c| c.rgb())
        .unwrap_or([0, 0, 0]);
    let hmode = rec.get("highlightMode").unwrap_or("screen").to_string();
    let smode = rec.get("shadowMode").unwrap_or("multiply").to_string();
    let to8 = |v: f64| (v * 255.0 + 0.5).floor().clamp(0.0, 255.0) as u8;
    if matches!(style, "innerBevel" | "emboss" | "pillowEmboss") {
        let iw = img.width();
        let ib = img.bounds();
        for (i, p) in img.data_mut().iter_mut().enumerate() {
            if p[3] == 0 {
                continue;
            }
            let (x, y) = (ib.left + (i % iw) as i32, ib.top + (i / iw) as i32);
            if !area.contains(x, y) {
                continue;
            }
            let k = (y - area.top) as usize * w + (x - area.left) as usize;
            let (sa, ha) = (to8(sh[k] * sop * a[k]), to8(hi[k] * hop * a[k]));
            if sa > 0 {
                *p = inner_mix(*p, sc, sa, &smode);
            }
            if ha > 0 {
                *p = inner_mix(*p, hc, ha, &hmode);
            }
        }
    }
    if !matches!(style, "outerBevel" | "emboss" | "pillowEmboss") {
        return None;
    }
    let mut out = new_image(area);
    for (k, d) in out.data_mut().iter_mut().enumerate() {
        let outside = 1.0 - a[k];
        if outside <= 0.0 {
            continue;
        }
        let (s, hh) = (sh_o[k] * sop, hi_o[k] * hop);
        let a2 = s + hh * (1.0 - s);
        if a2 <= 0.0 {
            continue;
        }
        let ch = |sv: u8, hv: u8| {
            ((sv as f64 * s * (1.0 - hh) + hv as f64 * hh) / a2 + 0.5)
                .floor()
                .clamp(0.0, 255.0) as u8
        };
        *d = [
            ch(sc[0], hc[0]),
            ch(sc[1], hc[1]),
            ch(sc[2], hc[2]),
            to8(a2 * outside),
        ];
    }
    Some(out)
}

/// Eigenes Kachelmuster (64 × 64, RGB) zu einem PS-Musternamen. Die Muster des Originals stehen
/// nicht in der Datei (nur Name und ID der Bibliothek); Firepixel verwendet eigene, aus dem Namen
/// abgeleitete Muster: „Bubbles“ und Namen mit „bubble“/„dot“ als weiche Kreise, sonst je nach
/// Prüfsumme Karo, Streifen oder Körnung.
pub(crate) fn own_pattern(name: &str) -> (usize, Vec<[u8; 3]>) {
    const N: usize = 64;
    let lower = name.to_ascii_lowercase();
    let hash = name
        .bytes()
        .fold(2166136261u32, |h, c| (h ^ c as u32).wrapping_mul(16777619));
    let kind = if lower.contains("bubble") || lower.contains("dot") {
        0
    } else {
        1 + hash % 3
    };
    let mut px = vec![[0u8; 3]; N * N];
    let mut rnd = hash | 1;
    for y in 0..N {
        for x in 0..N {
            let v: f64 = match kind {
                0 => {
                    // drei weiche Kreise je Kachel
                    let mut best: f64 = 0.0;
                    for &(cx, cy, r) in &[(16.0, 18.0, 13.0), (46.0, 40.0, 15.0), (30.0, 54.0, 8.0)]
                    {
                        let dx = (x as f64 + 0.5 - cx + 32.0).rem_euclid(64.0) - 32.0;
                        let dy = (y as f64 + 0.5 - cy + 32.0).rem_euclid(64.0) - 32.0;
                        let d = (dx * dx + dy * dy).sqrt() / r;
                        if d < 1.0 {
                            best = best.max(0.35 + 0.65 * d * d);
                        }
                    }
                    if best > 0.0 { best } else { 0.15 }
                }
                1 => {
                    if ((x / 8) + (y / 8)) % 2 == 0 {
                        0.8
                    } else {
                        0.35
                    }
                }
                2 => {
                    if (x + y) % 16 < 8 {
                        0.75
                    } else {
                        0.3
                    }
                }
                _ => {
                    rnd ^= rnd << 13;
                    rnd ^= rnd >> 17;
                    rnd ^= rnd << 5;
                    0.3 + 0.5 * (rnd % 1000) as f64 / 1000.0
                }
            };
            let g = (v * 255.0).round().clamp(0.0, 255.0) as u8;
            px[y * N + x] = [g, (g as u16 * 9 / 10) as u8, (g as u16 * 8 / 10) as u8];
        }
    }
    (N, px)
}

/// Musterüberlagerung mit eigenem Muster (`own_pattern`): Skalierung, Phase, „an Ebene ausrichten“
/// (Ursprung oben links am Objekt, sonst am Dokument), Deckkraft und Mischmodus wie die übrigen
/// inneren Effekte.
pub(crate) fn pattern_fill(img: &mut RgbaImage, obj: &RgbaImage, rec: &PsRecord) {
    let (n, tile) = own_pattern(rec.get("patternName").unwrap_or(""));
    let scale = (rec.num("scale", 100.0) / 100.0).clamp(0.01, 10.0);
    let (phx, phy) = (rec.num("horizontal", 0.0), rec.num("vertical", 0.0));
    let origin = if rec.flag("align") {
        (obj.bounds().left as f64, obj.bounds().top as f64)
    } else {
        (0.0, 0.0)
    };
    let op = crate::photoshop::opacity255(rec.num("opacity", 100.0));
    let mode = rec.get("mode").unwrap_or("normal").to_string();
    let iw = img.width();
    let ib = img.bounds();
    for (i, p) in img.data_mut().iter_mut().enumerate() {
        if p[3] == 0 {
            continue;
        }
        let (x, y) = (
            (ib.left + (i % iw) as i32) as f64,
            (ib.top + (i / iw) as i32) as f64,
        );
        let u = (((x - origin.0 - phx) / scale).floor() as i64).rem_euclid(n as i64) as usize;
        let v = (((y - origin.1 - phy) / scale).floor() as i64).rem_euclid(n as i64) as usize;
        *p = inner_mix(*p, tile[v * n + u], op, &mode);
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contour_linear_and_curve() {
        let lin = Contour::parse("2$$0.000000;0.000000;1$255.000000;255.000000;1").unwrap();
        assert!((lin.eval(0.5) - 0.5).abs() < 1e-9);
        let g = Contour::parse("3$$0;0;1$128;200;1$255;255;1").unwrap();
        assert!((g.eval(128.0 / 255.0) - 200.0 / 255.0).abs() < 1e-9);
        assert!(g.eval(0.25) > 0.25);
    }

    #[test]
    fn tri_blur_keeps_sum() {
        let mut v = vec![0.0; 21 * 21];
        v[10 * 21 + 10] = 1.0;
        let b = tri_blur(&v, 21, 21, 3);
        let s: f64 = b.iter().sum();
        assert!((s - 1.0).abs() < 1e-9);
    }
}

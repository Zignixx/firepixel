//! Farbmodelle und Umrechnungen wie das Original (Spezifikation 14 §1.5–§1.9, Referenz
//! `re\farbe\farbmodelle.py`, bitgleich laut Golden-Tests).
//!
//! Zwei HSB-Varianten werden bewusst **getrennt** gehalten:
//! - Farbmischer: Farbton in ganzen Grad ([`hue_degrees`], reine Ganzzahlrechnung), S/B in Prozent;
//!   Rückweg [`hsb_deg_to_rgb`] mit 60°-Sextanten.
//! - Byte-HSB (Filter, Sortierung, Info-Bedienfeld): [`rgb_to_hsb8`] mit Sextantbreite **42**
//!   (nicht 42,5), Rückweg [`hsb8_to_rgb`]. Die Rundreise ist nur zu 22 % exakt; das ist so gewollt.

use crate::round::{cdiv, f32r, round_away, round_even, trunc};

// ------------------------------------------------------------------ Mischer: RGB / Hex / CMY

/// Mischer-Modell (Pref `ColorMixerColorModel`, Default 1 = Hex).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum MixerModel {
    Rgb = 0,
    #[default]
    Hex = 1,
    Cmy = 2,
    Hsb = 3,
    Gray = 4,
}

impl MixerModel {
    /// Aus dem Pref-Wert; unbekannte Werte → `None`.
    pub fn from_pref(v: i64) -> Option<MixerModel> {
        Some(match v {
            0 => MixerModel::Rgb,
            1 => MixerModel::Hex,
            2 => MixerModel::Cmy,
            3 => MixerModel::Hsb,
            4 => MixerModel::Gray,
            _ => return None,
        })
    }

    /// Feldbeschriftungen (`R:`, `G:`, `B:` usw.).
    pub fn labels(self) -> &'static [&'static str] {
        match self {
            MixerModel::Rgb | MixerModel::Hex => &["R:", "G:", "B:"],
            MixerModel::Cmy => &["C:", "M:", "Y:"],
            MixerModel::Hsb => &["H:", "S:", "B:"],
            MixerModel::Gray => &["K:"],
        }
    }

    /// Reglerbereiche je Feld (14 §1.7).
    pub fn slider_ranges(self) -> &'static [(i32, i32)] {
        match self {
            MixerModel::Rgb | MixerModel::Hex | MixerModel::Cmy => &[(0, 255), (0, 255), (0, 255)],
            MixerModel::Hsb => &[(0, 360), (0, 100), (0, 100)],
            MixerModel::Gray => &[(0, 100)],
        }
    }
}

/// Mischer-Modell 1 (Hex): je Kanal `%X`, bei < 16 mit führender `0` (`@00f59cc0`).
pub fn rgb_to_hex_text(r: u8, g: u8, b: u8) -> [String; 3] {
    [r, g, b].map(|v| {
        if v < 16 {
            format!("0{v:X}")
        } else {
            format!("{v:X}")
        }
    })
}

/// Mischer-Modell 2 (CMY): `255 − Kanal`, keine Prozent.
pub fn rgb_to_cmy(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
    (255 - r, 255 - g, 255 - b)
}

/// Rückweg CMY → RGB (`255 − Eingabe`, auf ein Byte gekürzt).
pub fn cmy_to_rgb(c: i64, m: i64, y: i64) -> (u8, u8, u8) {
    (
        ((255 - c) & 0xff) as u8,
        ((255 - m) & 0xff) as u8,
        ((255 - y) & 0xff) as u8,
    )
}

// ------------------------------------------------------------------ Mischer: HSB (Grad / Prozent)

/// max/min mit der Vergleichsreihenfolge des Originals (wirkt nur bei Gleichständen).
fn max_min(r: i64, g: i64, b: i64) -> (i64, i64) {
    let mut mx = if g <= r { r } else { g };
    if mx < b {
        mx = b;
    }
    let mut mn = if r <= g { r } else { g };
    if b < mn {
        mn = b;
    }
    (mx, mn)
}

/// Farbton in ganzen Grad 0..359 (`@006d0ec0`), reine Ganzzahlrechnung mit C-Division.
pub fn hue_degrees(r: u8, g: u8, b: u8) -> i32 {
    let (r, g, b) = (r as i64, g as i64, b as i64);
    let (mx, mn) = max_min(r, g, b);
    let mut h = 0;
    if mx > 0 {
        let d = mx - mn;
        if cdiv(d * 255, mx) > 0 {
            if r == mx {
                h = cdiv((g - b) * 60, d);
            } else if g == mx {
                h = cdiv((b - r) * 60, d) + 120;
            } else if b == mx {
                h = cdiv((r - g) * 60, d) + 240;
            } else {
                return 0;
            }
            if h < 0 {
                h += 360;
            }
        }
    }
    h as i32
}

/// Byte-HSB als Gleitkomma (`@006d0bf0`): H, S, B auf der Skala 0..255, Sextantbreite 42.
pub fn rgb_to_hsb8_f64(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let (ri, gi, bi) = (r as i64, g as i64, b as i64);
    let (mx, mn) = max_min(ri, gi, bi);
    let (mut h, mut s) = (0.0, 0.0);
    let v = mx as f64;
    if mx != 0 {
        let d = (mx - mn) as f64;
        s = d * 255.0 / mx as f64;
        if s != 0.0 {
            if mx == ri {
                h = (gi - bi) as f64 * 42.0 / d;
            } else if mx == gi {
                h = (bi - ri) as f64 * 42.0 / d + 85.0;
            } else if mx == bi {
                h = (ri - gi) as f64 * 42.0 / d + 170.0;
            }
            if h < 0.0 {
                h += 255.0;
            }
        }
    }
    (h, s, v)
}

/// Byte-HSB (`@006d0d20`): `(H8, S8, B8)` je mit `round_even` (Werte 0..=255).
pub fn rgb_to_hsb8(r: u8, g: u8, b: u8) -> (i32, i32, i32) {
    let (h, s, v) = rgb_to_hsb8_f64(r, g, b);
    (
        round_even(h) as i32,
        round_even(s) as i32,
        round_even(v) as i32,
    )
}

/// Gepackte Byte-HSB wie `@006d0d20`: `0xAA_HH_BB_SS` (Byte 2 = H, Byte 1 = B, Byte 0 = S).
pub fn hsb8_packed(r: u8, g: u8, b: u8, a: u8) -> u32 {
    let (h, s, v) = rgb_to_hsb8(r, g, b);
    ((a as u32) << 24)
        | (((h & 0xff) as u32) << 16)
        | (((v & 0xff) as u32) << 8)
        | (s & 0xff) as u32
}

/// Anzeige im Mischer (Modell 3): H in Grad, S und B in Prozent (`@00f59cc0`).
pub fn rgb_to_mixer_hsb(r: u8, g: u8, b: u8) -> (i32, i32, i32) {
    let (_, s8, v8) = rgb_to_hsb8(r, g, b);
    (
        hue_degrees(r, g, b),
        trunc(round_away(s8 as f64 * 100.0 / 255.0)) as i32,
        trunc(round_away(v8 as f64 * 100.0 / 255.0)) as i32,
    )
}

/// Ganzzahl-HSV mit 60°-Sextanten (`@006cf860`): H in Grad, S und V auf 0..255.
/// Rückgabe ungekürzt (Aufrufer kürzen auf ein Byte).
pub fn hsb_deg_to_rgb(h: i64, s: i64, v: i64) -> (i64, i64, i64) {
    if s == 0 {
        return (v, v, v);
    }
    let mut h = h;
    let mut sext = 0;
    if h > 59 {
        let mut found = false;
        for (k, lo) in [60i64, 120, 180, 240, 300].iter().enumerate() {
            if h < lo + 60 {
                h -= lo;
                sext = k + 1;
                found = true;
                break;
            }
        }
        if !found {
            h = 0;
            sext = 0;
        }
    }
    let p = cdiv((255 - s) * v, 255);
    let q = cdiv((255 - cdiv(h * s, 60)) * v, 255);
    let t = cdiv((255 - cdiv((60 - h) * s, 60)) * v, 255);
    match sext {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    }
}

/// Rückweg im Mischer (Modell 3, `@00f58f10`): H in Grad, S und B in Prozent.
pub fn mixer_hsb_to_rgb(h_deg: f64, s_pct: f64, b_pct: f64) -> (u8, u8, u8) {
    let h = trunc(round_away(h_deg));
    let s = trunc(round_away(s_pct * 255.0 / 100.0));
    let v = trunc(round_away(b_pct * 255.0 / 100.0));
    let (r, g, b) = hsb_deg_to_rgb(h, s, v);
    ((r & 0xff) as u8, (g & 0xff) as u8, (b & 0xff) as u8)
}

/// Byte-HSB → RGB als Gleitkomma (`@006cf650`): Sextantgrenzen 42/85/127/170/212/255,
/// Teiler immer 42; H = 255 zählt als Sextant 0 mit f = 0; H > 255 → Schwarz.
pub fn hsb8_to_rgb_f64(h: i32, s: i32, v: i32) -> (f64, f64, f64) {
    if s == 0 {
        return (v as f64, v as f64, v as f64);
    }
    let (sext, mut f) = if h == 255 {
        (0, 0.0)
    } else if h < 42 {
        (0, h as f64)
    } else if h < 85 {
        (1, h as f64 - 42.0)
    } else if h < 127 {
        (2, h as f64 - 85.0)
    } else if h < 170 {
        (3, h as f64 - 127.0)
    } else if h < 212 {
        (4, h as f64 - 170.0)
    } else if h < 255 {
        (5, h as f64 - 212.0)
    } else {
        return (0.0, 0.0, 0.0);
    };
    if f >= 42.0 {
        f = f.trunc();
    }
    let (sf, vf) = (s as f64, v as f64);
    let p = (255.0 - sf) * vf / 255.0;
    let q = (255.0 - f * sf / 42.0) * vf / 255.0;
    let t = (255.0 - sf * (42.0 - f) / 42.0) * vf / 255.0;
    match sext {
        0 => (vf, t, p),
        1 => (q, vf, p),
        2 => (p, vf, t),
        3 => (p, q, vf),
        4 => (t, p, vf),
        _ => (vf, p, q),
    }
}

/// Byte-HSB → RGB (`@006d0df0`): wie [`hsb8_to_rgb_f64`], Kanäle mit `round_even`, auf ein Byte gekürzt.
pub fn hsb8_to_rgb(h: i32, s: i32, v: i32) -> (u8, u8, u8) {
    let (r, g, b) = hsb8_to_rgb_f64(h, s, v);
    let c = |x: f64| (round_even(x) as i64 & 0xff) as u8;
    (c(r), c(g), c(b))
}

// ------------------------------------------------------------------ Mischer: Graustufen

/// Graustufen-Gewichte (`0x1b5f7b8`, Summe 512, ≈ Rec. 709).
pub const GRAY_WEIGHTS: [u32; 3] = [109, 366, 37];

/// Graustufe/Luminanz `(109·R + 366·G + 37·B) >> 9` (auch Sortierung beim Export, 08 §2.6).
#[inline]
pub fn gray_level(r: u8, g: u8, b: u8) -> u8 {
    (((r as u32 * 109 + g as u32 * 366 + b as u32 * 37) >> 9) & 0xff) as u8
}

/// Anzeige „K:“ in Prozent Schwarz (Mischer-Modell 4).
pub fn rgb_to_mixer_k(r: u8, g: u8, b: u8) -> i32 {
    trunc(100.0 - round_away(gray_level(r, g, b) as f64 * 100.0 / 255.0)) as i32
}

/// Reglerwert im Graustufen-Modell (Helligkeit in Prozent, `@00f58d60`).
pub fn rgb_to_gray_slider(r: u8, g: u8, b: u8) -> i32 {
    trunc(round_away(gray_level(r, g, b) as f64 * 100.0 / 255.0)) as i32
}

/// Rückweg K → Grau: `v = round_even(255 − round_away(K·255/100))`.
pub fn mixer_k_to_rgb(k_pct: f64) -> (u8, u8, u8) {
    let v = (round_even(255.0 - round_away(k_pct * 255.0 / 100.0)) as i64 & 0xff) as u8;
    (v, v, v)
}

// ------------------------------------------------------------------ Mischer: Anzeige und Eingabe

/// Textfelder des Farbmischers für eine Farbe (Anzeige `@00f59cc0`).
pub fn mixer_fields(model: MixerModel, r: u8, g: u8, b: u8) -> Vec<String> {
    match model {
        MixerModel::Rgb => vec![r.to_string(), g.to_string(), b.to_string()],
        MixerModel::Hex => rgb_to_hex_text(r, g, b).to_vec(),
        MixerModel::Cmy => {
            let (c, m, y) = rgb_to_cmy(r, g, b);
            vec![c.to_string(), m.to_string(), y.to_string()]
        }
        MixerModel::Hsb => {
            let (h, s, v) = rgb_to_mixer_hsb(r, g, b);
            vec![h.to_string(), format!("{s}%"), format!("{v}%")]
        }
        MixerModel::Gray => vec![format!("{}%", rgb_to_mixer_k(r, g, b))],
    }
}

/// Liest eine ganze Zahl wie `swscanf("%d")` bzw. `"%x"`: führende Leerzeichen, optionales
/// Vorzeichen, dann Ziffern; der Rest wird ignoriert. `None` ohne Ziffern.
fn scan_int(s: &str, radix: u32) -> Option<i64> {
    let s = s.trim_start();
    let (neg, rest) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let rest = if radix == 16 {
        rest.strip_prefix("0x")
            .or_else(|| rest.strip_prefix("0X"))
            .unwrap_or(rest)
    } else {
        rest
    };
    let digits: String = rest.chars().take_while(|c| c.is_digit(radix)).collect();
    if digits.is_empty() {
        return None;
    }
    let v = i64::from_str_radix(&digits, radix).ok()?;
    Some(if neg { -v } else { v })
}

/// Rückweg Textfelder → RGB (`@00f58f10`). RGB/Hex werden nicht begrenzt, sondern auf ein Byte
/// gekürzt (A); HSB und K wie [`mixer_hsb_to_rgb`] bzw. [`mixer_k_to_rgb`]. `None`, wenn ein Feld
/// keine Zahl enthält oder die Feldzahl nicht passt.
pub fn mixer_parse(model: MixerModel, fields: &[&str]) -> Option<(u8, u8, u8)> {
    let need = model.labels().len();
    if fields.len() != need {
        return None;
    }
    let radix = if model == MixerModel::Hex { 16 } else { 10 };
    let mut v = [0i64; 3];
    for (i, f) in fields.iter().enumerate() {
        v[i] = scan_int(f, radix)?;
    }
    let byte = |x: i64| (x & 0xff) as u8;
    Some(match model {
        MixerModel::Rgb | MixerModel::Hex => (byte(v[0]), byte(v[1]), byte(v[2])),
        MixerModel::Cmy => cmy_to_rgb(v[0], v[1], v[2]),
        MixerModel::Hsb => mixer_hsb_to_rgb(v[0] as f64, v[1] as f64, v[2] as f64),
        MixerModel::Gray => mixer_k_to_rgb(v[0] as f64),
    })
}

/// Wert eines Mischerfelds für den Popup-Schieber (▾ neben dem Feld wie im Original, R-7
/// `mixer.field.r.slider`): Zahl am Feldanfang (Hex sedezimal, `%` ignoriert), auf den
/// Reglerbereich begrenzt.
pub fn mixer_slider_value(model: MixerModel, index: usize, field: &str) -> f64 {
    let (lo, hi) = model
        .slider_ranges()
        .get(index)
        .copied()
        .unwrap_or((0, 255));
    let radix = if model == MixerModel::Hex { 16 } else { 10 };
    scan_int(field, radix)
        .unwrap_or(lo as i64)
        .clamp(lo as i64, hi as i64) as f64
}

/// Feldtext zu einem Schieberwert (Darstellung wie [`mixer_fields`]: Hex zweistellig groß,
/// S/B bei HSB und K mit `%`).
pub fn mixer_slider_text(model: MixerModel, index: usize, value: f64) -> String {
    let (lo, hi) = model
        .slider_ranges()
        .get(index)
        .copied()
        .unwrap_or((0, 255));
    let v = (value.round() as i64).clamp(lo as i64, hi as i64);
    match model {
        MixerModel::Hex => format!("{v:02X}"),
        MixerModel::Hsb if index > 0 => format!("{v}%"),
        MixerModel::Gray => format!("{v}%"),
        _ => v.to_string(),
    }
}

// ------------------------------------------------------------------ Info-Bedienfeld

/// Modell des Info-Bedienfelds (Pref `InfoPanelColorModel`, Default 1 = Hex).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum InfoPanelModel {
    Rgb = 0,
    #[default]
    Hex = 1,
    Cmy = 2,
    Hsb = 3,
}

/// Die vier Textfelder des Info-Bedienfelds (`@009a0a30`). `model` ist der Pref-Wert; Werte
/// außerhalb 0..=3 ergeben vier leere Felder (kein `default`-Zweig im Original).
/// HSB kommt aus der **Byte-HSB** (42er-Sextanten), nicht aus [`hue_degrees`]: z. B. Cyan 179°.
pub fn info_panel_values(a: u8, r: u8, g: u8, b: u8, model: i64) -> [String; 4] {
    match model {
        1 => [r, g, b, a].map(|v| format!("{v:02X}")),
        2 => [255 - r, 255 - g, 255 - b, a].map(|v| v.to_string()),
        3 => {
            let (h8, s8, v8) = rgb_to_hsb8(r, g, b);
            let f = |x: i32, k: f64| trunc(round_away((x & 0xff) as f64 * k / 255.0)).to_string();
            [f(h8, 360.0), f(s8, 100.0), f(v8, 100.0), a.to_string()]
        }
        0 => [r, g, b, a].map(|v| v.to_string()),
        _ => Default::default(),
    }
}

// ------------------------------------------------------------------ Pipette

/// Pipetten-Abtastung (`@006e2890`, 14 §1.11): Fenster `[x−n, x+n] × [y−n, y+n]`, an `bounds`
/// (links/oben inklusiv, rechts/unten exklusiv) beschnitten, Mittel je Kanal mit Ganzzahldivision.
/// `faithful_bug = true` bildet den Original-Fehler nach: Ergebnis ist das **linke obere** Pixel
/// des beschnittenen Fensters (auch im Komposit-Pfad, Laufzeit bestätigt). Punkt außerhalb → `None`.
/// `n` = `EyeDropperExtraSamples` (0 = 1 Pixel, 1 = 3×3, 2 = 5×5). Das Original setzt danach Alpha auf
/// 0xFF; das bleibt dem Aufrufer überlassen.
pub fn eyedropper_sample<F>(
    get_pixel: F,
    x: i32,
    y: i32,
    n: i32,
    bounds: (i32, i32, i32, i32),
    faithful_bug: bool,
) -> Option<crate::Rgba8>
where
    F: Fn(i32, i32) -> crate::Rgba8,
{
    let (l, t, r, b) = bounds;
    if !(l <= x && x < r && t <= y && y < b) {
        return None;
    }
    if n < 1 {
        return Some(get_pixel(x, y));
    }
    let (x0, y0) = ((x - n).max(l), (y - n).max(t));
    let (x1, y1) = ((x + n + 1).min(r), (y + n + 1).min(b));
    if faithful_bug {
        return Some(get_pixel(x0, y0));
    }
    let mut sum = [0u64; 4];
    let mut cnt = 0u64;
    for yy in y0..y1 {
        for xx in x0..x1 {
            let p = get_pixel(xx, yy);
            for (s, v) in sum.iter_mut().zip([p.r, p.g, p.b, p.a]) {
                *s += v as u64;
            }
            cnt += 1;
        }
    }
    let cnt = cnt.max(1);
    Some(crate::Rgba8::new(
        (sum[0] / cnt) as u8,
        (sum[1] / cnt) as u8,
        (sum[2] / cnt) as u8,
        (sum[3] / cnt) as u8,
    ))
}

// ------------------------------------------------------------------ Gamma-Vorschau

/// Tabelle der Ansicht „Gamma 1.8“ (`@00da1300`): `e = Other/Local` (bei Local ≤ 0,1: `e = Other`),
/// `T[i] = trunc(float(pow(float(i/255), e))·255 + 0,5)`. Windows-Default: Local 2,2, Other 1,8.
/// Angewandt auf R, G, B des fertigen Bildschirmbilds (inkl. Schachbrett), nie auf Dokumentdaten.
pub fn gamma_preview_table(local_gamma: f64, other_gamma: f64) -> [u8; 256] {
    let e = f32r(if local_gamma > 0.1 {
        other_gamma / local_gamma
    } else {
        other_gamma
    });
    let mut t = [0u8; 256];
    for (i, out) in t.iter_mut().enumerate() {
        let x = f32r(i as f64 / 255.0);
        let y = if x > 0.0 { f32r(x.powf(e)) } else { 0.0 };
        *out = trunc(y * 255.0 + 0.5).clamp(0, 255) as u8;
    }
    t
}

// ------------------------------------------------------------------ 8 ↔ 16 Bit

/// 8 → 16 Bit, ganzzahlig: `257·v` (gleich dem `float32`-Weg mit Abschneiden, 256 von 256).
#[inline]
pub const fn to16(v: u8) -> u16 {
    v as u16 * 257
}

/// 16 → 8 Bit, ganzzahlig: `v16·255/65535` abgeschnitten (gleich dem `float32`-Weg, 65.536 von 65.536).
#[inline]
pub const fn to8(v16: u16) -> u8 {
    (v16 as u32 * 255 / 65535) as u8
}

/// 8 → 16 Bit wie `@00da2200` (B): `q = float32(v/255)`, Produkt `q·65535` abgeschnitten; für
/// die Kanäle M, Y, K (`channel ≠ 0`) wird das Produkt vorher als `float32` gespeichert.
pub fn to16_float(v: u8, channel: usize) -> u32 {
    let q = f32r(v as f64 / 255.0);
    let mut p = q * 65535.0;
    if channel != 0 {
        p = f32r(p);
    }
    trunc(p) as u32
}

/// 16 → 8 Bit wie `@00da2130` (B): `trunc(float32(v/65535)·255)`.
pub fn to8_float(v16: u32) -> u8 {
    (trunc(f32r(v16 as f64 / 65535.0) * 255.0) & 0xff) as u8
}

#[cfg(test)]
mod tests {
    #[test]
    fn cffp4_mixer_slider_value_and_text() {
        use super::{MixerModel, mixer_fields, mixer_parse, mixer_slider_text, mixer_slider_value};
        assert_eq!(mixer_slider_value(MixerModel::Hex, 0, "99"), 153.0);
        assert_eq!(mixer_slider_text(MixerModel::Hex, 0, 10.0), "0A");
        assert_eq!(mixer_slider_value(MixerModel::Rgb, 1, "300"), 255.0);
        assert_eq!(mixer_slider_value(MixerModel::Hsb, 1, "40%"), 40.0);
        assert_eq!(mixer_slider_text(MixerModel::Hsb, 0, 200.0), "200");
        assert_eq!(mixer_slider_text(MixerModel::Hsb, 2, 50.4), "50%");
        assert_eq!(mixer_slider_text(MixerModel::Gray, 0, 120.0), "100%");
        // Rundreise: Schieber auf R = 0x40 im Hex-Modell ergibt wieder ein gültiges Feld
        let mut f = mixer_fields(MixerModel::Hex, 0x99, 0x99, 0x99);
        f[0] = mixer_slider_text(MixerModel::Hex, 0, 64.0);
        let refs: Vec<&str> = f.iter().map(|x| x.as_str()).collect();
        assert_eq!(
            mixer_parse(MixerModel::Hex, &refs),
            Some((0x40, 0x99, 0x99))
        );
    }

    use super::*;

    #[test]
    fn mixer_examples() {
        assert_eq!(hue_degrees(0, 255, 255), 180);
        assert_eq!(
            info_panel_values(255, 0, 255, 255, 3),
            ["179", "100", "100", "255"].map(String::from)
        );
        assert_eq!(
            info_panel_values(255, 10, 255, 0, 1),
            ["0A", "FF", "00", "FF"].map(String::from)
        );
        assert_eq!(info_panel_values(255, 1, 2, 3, 7), <[String; 4]>::default());
        assert_eq!(
            rgb_to_hex_text(10, 255, 0),
            ["0A".to_string(), "FF".into(), "00".into()]
        );
        assert_eq!(
            mixer_fields(MixerModel::Hsb, 255, 0, 0),
            vec!["0", "100%", "100%"]
        );
        assert_eq!(
            mixer_parse(MixerModel::Hex, &["ff", "0x80", "7"]),
            Some((255, 128, 7))
        );
        assert_eq!(
            mixer_parse(MixerModel::Cmy, &["0", "255", "55"]),
            Some((255, 0, 200))
        );
        assert_eq!(mixer_parse(MixerModel::Rgb, &["1", "x", "3"]), None);
        assert_eq!(mixer_parse(MixerModel::Gray, &["100%"]), Some((0, 0, 0)));
    }

    #[test]
    fn eyedropper() {
        use crate::Rgba8;
        // rot für x < 400, blau ab 400 (Laufzeittest 14): 5×5 an (401,200) → rot (Fehler), 1 Pixel → blau
        let px = |x: i32, _y: i32| {
            if x < 400 {
                Rgba8::rgb(255, 0, 0)
            } else {
                Rgba8::rgb(0, 0, 255)
            }
        };
        let b = (0, 0, 800, 600);
        assert_eq!(
            eyedropper_sample(px, 401, 200, 2, b, true),
            Some(Rgba8::rgb(255, 0, 0))
        );
        assert_eq!(
            eyedropper_sample(px, 401, 200, 0, b, true),
            Some(Rgba8::rgb(0, 0, 255))
        );
        assert_eq!(
            eyedropper_sample(px, 402, 200, 2, b, true),
            Some(Rgba8::rgb(0, 0, 255))
        );
        assert_eq!(
            eyedropper_sample(px, 401, 200, 2, b, false),
            Some(Rgba8::rgb(51, 0, 204))
        );
        assert_eq!(eyedropper_sample(px, 800, 0, 2, b, true), None);
    }

    #[test]
    fn gamma_examples() {
        let t = gamma_preview_table(2.2, 1.8);
        assert_eq!(
            (t[0], t[64], t[128], t[192], t[255]),
            (0, 82, 145, 202, 255)
        );
    }

    #[test]
    fn bit_depth() {
        for v in 0..=255u8 {
            for ch in 0..4 {
                assert_eq!(to16_float(v, ch), to16(v) as u32);
            }
        }
        for v in 0..=65535u16 {
            assert_eq!(to8_float(v as u32), to8(v), "{v}");
        }
    }
}

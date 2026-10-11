//! Mischmodi und Komposition wie das Original (Spezifikation 07 §7, 05 §2.5; Referenz
//! `re\vektor\blend.py`, im echten Original 46/46 Modi pixelgenau bestätigt, Laufzeittest T1).
//!
//! Pixel sind nicht vormultipliziertes RGBA ([`Rgba8`]). Ablauf je Pixel ([`blend_pixel`]):
//!
//! - **Schritt B** (Deckung): `a = mul(as, op)`, mit Maske `mul(as, m)`, mit beiden `mul(mul(m, op), as)`.
//! - **Schritt A** (Moduskern, nur wo das Ziel nicht transparent ist): je Kanal Mischfarbe
//!   `B(s, d)`, dann `Cs′ = s ± ((|B − s|·ad + 127) / 255)`.
//! - **Schritt C** (Source-over, nicht vormultipliziert): skalar ([`over_scalar`], `@006f53c0`)
//!   oder für Normal auf Seitenebene die MMX-Rundung ([`over_mmx`], `@006f5980`).
//!
//! Sonderzweige: Invert (17) = Quellfarbe `255 − Ziel`, dann over; Erase (25); Tint (29) mit
//! MMX-Lerp. Quelle `s` = Objekt, Ziel `d` = Untergrund, **nirgends vertauscht** (Laufzeit T1).
//! Overlay (4) prüft wie in der bestätigten Referenz das **Ziel** (`d < 128`).
//!
//! Für Effekte (05 §2.5) gibt es die vollständigen Bild-auf-Bild-Formen [`orig_over`],
//! [`orig_over_mmx`], [`behind`], [`over_exact`] (Photoshop-Effekte) und
//! [`color_over_keep_alpha`] (interner Modus 6). Eine einzige „over“-Funktion für alles führte zu
//! ±1-Fehlern an Kanten; deshalb bleiben die Varianten getrennt.

#![allow(clippy::needless_range_loop)] // Matrix-/Kanalindizes sind hier lesbarer

use crate::models::gray_level;
use crate::round::{div255, mul255, DIV};
use crate::{BlendMode, Opacity, Rgba8};

// ------------------------------------------------------------------ Modusnummern

impl BlendMode {
    pub const NORMAL: BlendMode = BlendMode(0);
    pub const MULTIPLY: BlendMode = BlendMode(2);
    pub const DIFFERENCE: BlendMode = BlendMode(3);
    pub const OVERLAY: BlendMode = BlendMode(4);
    pub const SCREEN: BlendMode = BlendMode(5);
    pub const DISSOLVE: BlendMode = BlendMode(7);
    pub const LINEAR_BURN: BlendMode = BlendMode(8);
    pub const DARKEN: BlendMode = BlendMode(9);
    pub const LIGHTEN: BlendMode = BlendMode(10);
    pub const HUE: BlendMode = BlendMode(11);
    pub const SATURATION: BlendMode = BlendMode(12);
    pub const COLOR: BlendMode = BlendMode(13);
    pub const LUMINOSITY: BlendMode = BlendMode(14);
    pub const INVERT: BlendMode = BlendMode(17);
    pub const LINEAR_DODGE: BlendMode = BlendMode(18);
    pub const VIVID_LIGHT: BlendMode = BlendMode(19);
    pub const LINEAR_LIGHT: BlendMode = BlendMode(20);
    pub const PIN_LIGHT: BlendMode = BlendMode(21);
    pub const HARD_MIX: BlendMode = BlendMode(22);
    pub const ERASE: BlendMode = BlendMode(25);
    pub const TINT: BlendMode = BlendMode(29);
    pub const AVERAGE: BlendMode = BlendMode(30);
    pub const NEGATION: BlendMode = BlendMode(31);
    pub const EXCLUSION: BlendMode = BlendMode(32);
    pub const HARD_LIGHT: BlendMode = BlendMode(33);
    pub const SOFT_LIGHT1: BlendMode = BlendMode(34);
    pub const SOFT_LIGHT2: BlendMode = BlendMode(35);
    pub const COLOR_DODGE: BlendMode = BlendMode(36);
    pub const COLOR_BURN: BlendMode = BlendMode(37);
    pub const INVERSE_COLOR_DODGE: BlendMode = BlendMode(38);
    pub const INVERSE_COLOR_BURN: BlendMode = BlendMode(39);
    pub const SOFT_DODGE: BlendMode = BlendMode(40);
    pub const SOFT_BURN: BlendMode = BlendMode(41);
    pub const REFLECT: BlendMode = BlendMode(42);
    pub const GLOW: BlendMode = BlendMode(43);
    pub const FREEZE: BlendMode = BlendMode(44);
    pub const HEAT: BlendMode = BlendMode(45);
    pub const ADDITIVE: BlendMode = BlendMode(46);
    pub const SUBTRACTIVE: BlendMode = BlendMode(47);
    pub const INTERPOLATION: BlendMode = BlendMode(48);
    pub const STAMP: BlendMode = BlendMode(49);
    pub const XOR: BlendMode = BlendMode(50);
    pub const RED: BlendMode = BlendMode(51);
    pub const GREEN: BlendMode = BlendMode(52);
    pub const BLUE: BlendMode = BlendMode(53);
    pub const SUBTRACT: BlendMode = BlendMode(54);

    /// Alle 46 Modi mit Namen (Nummer = `BLDi`; Namen wie in der Referenz `blend.py`).
    pub const ALL: [(BlendMode, &'static str); 46] = [
        (BlendMode(0), "normal"),
        (BlendMode(2), "multiply"),
        (BlendMode(3), "difference"),
        (BlendMode(4), "overlay"),
        (BlendMode(5), "screen"),
        (BlendMode(7), "dissolve"),
        (BlendMode(8), "linearburn"),
        (BlendMode(9), "darken"),
        (BlendMode(10), "lighten"),
        (BlendMode(11), "hue"),
        (BlendMode(12), "saturation"),
        (BlendMode(13), "color"),
        (BlendMode(14), "luminosity"),
        (BlendMode(17), "invert"),
        (BlendMode(18), "lineardodge"),
        (BlendMode(19), "vividlight"),
        (BlendMode(20), "linearlight"),
        (BlendMode(21), "pinlight"),
        (BlendMode(22), "hardmix"),
        (BlendMode(25), "erase"),
        (BlendMode(29), "tint"),
        (BlendMode(30), "average"),
        (BlendMode(31), "negation"),
        (BlendMode(32), "exclusion"),
        (BlendMode(33), "hardlight"),
        (BlendMode(34), "softlight1"),
        (BlendMode(35), "softlight2"),
        (BlendMode(36), "colordodge"),
        (BlendMode(37), "colorburn"),
        (BlendMode(38), "inversecolordodge"),
        (BlendMode(39), "inversecolorburn"),
        (BlendMode(40), "softdodge"),
        (BlendMode(41), "softburn"),
        (BlendMode(42), "reflect"),
        (BlendMode(43), "glow"),
        (BlendMode(44), "freeze"),
        (BlendMode(45), "heat"),
        (BlendMode(46), "additive"),
        (BlendMode(47), "subtractive"),
        (BlendMode(48), "interpolation"),
        (BlendMode(49), "stamp"),
        (BlendMode(50), "xor"),
        (BlendMode(51), "red"),
        (BlendMode(52), "green"),
        (BlendMode(53), "blue"),
        (BlendMode(54), "subtract"),
    ];

    /// Wert aus der Datei (`BLDi`): fehlt = 0; 55 wird beim Lesen zu 0 (07 §7.1). Werte außerhalb
    /// 0..=255 ergeben Normal. Unbekannte Nummern bleiben erhalten (Rundreise).
    pub fn from_file(raw: Option<i64>) -> BlendMode {
        match raw {
            None | Some(55) => BlendMode(0),
            Some(v) if (0..=255).contains(&v) => BlendMode(v as u8),
            Some(_) => BlendMode(0),
        }
    }

    /// Name des Modus, `None` für interne oder unbekannte Nummern.
    pub fn name(self) -> Option<&'static str> {
        BlendMode::ALL.iter().find(|(m, _)| *m == self).map(|(_, n)| *n)
    }

    /// Modus zum Namen (Groß-/Kleinschreibung egal).
    pub fn from_name(name: &str) -> Option<BlendMode> {
        BlendMode::ALL.iter().find(|(_, n)| n.eq_ignore_ascii_case(name)).map(|(m, _)| *m)
    }

    /// Einer der 46 dokumentierten Modi?
    pub fn is_known(self) -> bool {
        self.name().is_some()
    }
}

// ------------------------------------------------------------------ Grundformeln

/// `round(a·b/255)` (Original-Form).
#[inline]
pub fn mul(a: u8, b: u8) -> u8 {
    mul255(a as u32, b as u32) as u8
}

/// Divisionstabelle `⌊255·a/b⌋` (0 bei `b < a` oder `b = 0`).
#[inline]
pub fn div_table(a: u8, b: u8) -> u8 {
    DIV[a as usize][b as usize]
}

/// Kurve des Modus „interpolation“ (48): `C[i] = ⌊64 − 64·cos(i·π/255) + 0,5⌋`.
pub fn interpolation_curve() -> [i32; 256] {
    let mut c = [0i32; 256];
    for (i, v) in c.iter_mut().enumerate() {
        *v = (64.0 - 64.0 * (i as f64 * std::f64::consts::PI / 255.0).cos() + 0.5).floor() as i32;
    }
    c
}

fn interp_c(i: u8) -> i32 {
    use std::sync::OnceLock;
    static C: OnceLock<[i32; 256]> = OnceLock::new();
    C.get_or_init(interpolation_curve)[i as usize]
}

fn soft1(s: i32, d: i32) -> i32 {
    let s_ = s as f64 / 255.0;
    let d_ = d as f64 / 255.0;
    let x = if s_ < 0.5 {
        2.0 * s_ * d_ + d_ * d_ * (1.0 - 2.0 * s_)
    } else {
        d_.sqrt() * (2.0 * s_ - 1.0) + 2.0 * d_ * (1.0 - s_)
    };
    ((x * 255.0) as i64).clamp(0, 255) as i32
}

fn soft2(s: i32, d: i32) -> i32 {
    let s_ = s as f64 / 255.0;
    let d_ = d as f64 / 255.0;
    (((d_ * d_ + 2.0 * s_ * d_ - 2.0 * s_ * d_ * d_) * 255.0) as i64).clamp(0, 255) as i32
}

fn vivid(s: i32, d: i32) -> i32 {
    if s < 128 {
        return if s == 0 { 0 } else { (255 - (256 * (255 - d)) / (2 * s)).max(0) };
    }
    if s == 128 {
        return 128;
    }
    if s == 255 { 255 } else { ((256 * d) / (2 * (255 - s))).min(255) }
}

/// Kanalformel `B(s, d)` der kanalweisen Modi; `None` für Modi ohne Kanalformel
/// (HSL-Modi, Rot/Grün/Blau, Sonderzweige, unbekannte Nummern).
pub fn blend_channel(mode: BlendMode, s: u8, d: u8) -> Option<u8> {
    let (s, d) = (s as i32, d as i32);
    let mul_i = |a: i32, b: i32| mul255(a as u32, b as u32) as i32;
    let v = match mode.0 {
        0 => s,
        2 => mul_i(s, d),
        3 => (s - d).abs(),
        4 => {
            if d < 128 {
                2 * mul_i(s, d)
            } else {
                255 - 2 * mul_i(255 - s, 255 - d)
            }
        }
        5 => 255 - mul_i(255 - s, 255 - d),
        8 => (s + d - 255).max(0),
        9 => s.min(d),
        10 => s.max(d),
        18 => (s + d).min(255),
        19 => vivid(s, d),
        20 => (d + 2 * s - 255).clamp(0, 255),
        21 => {
            if s < 128 {
                d.min(2 * s)
            } else {
                d.max(2 * s - 256)
            }
        }
        22 => {
            if d < 255 - s {
                0
            } else {
                255
            }
        }
        30 => (s + d) >> 1,
        31 => 255 - (255 - s - d).abs(),
        32 => (s + d - ((s * d) >> 7)) & 255,
        33 => {
            if s < 128 {
                (s * d) >> 7
            } else {
                255 - (((255 - s) * (255 - d)) >> 7)
            }
        }
        34 => soft1(s, d),
        35 => soft2(s, d),
        36 => {
            if s == 255 {
                255
            } else {
                ((256 * d) / (255 - s)).min(255)
            }
        }
        37 => {
            if s == 0 {
                0
            } else {
                (255 - (256 * (255 - d)) / s).max(0)
            }
        }
        38 => {
            if d == 255 {
                255
            } else {
                ((256 * s) / (255 - d)).min(255)
            }
        }
        39 => {
            if d == 0 {
                0
            } else {
                (255 - (256 * (255 - s)) / d).max(0)
            }
        }
        40 => {
            if s + d < 256 {
                if s == 255 { 255 } else { ((128 * d) / (255 - s)).min(255) }
            } else if d == 0 {
                0
            } else {
                (255 - (128 * (255 - s)) / d).max(0)
            }
        }
        41 => {
            if s + d < 256 {
                if d == 255 { 255 } else { ((128 * s) / (255 - d)).min(255) }
            } else if s == 0 {
                0
            } else {
                (255 - (128 * (255 - d)) / s).max(0)
            }
        }
        42 => {
            if s == 255 {
                255
            } else {
                ((d * d) / (255 - s)).min(255)
            }
        }
        43 => {
            if d == 255 {
                255
            } else {
                ((s * s) / (255 - d)).min(255)
            }
        }
        44 => {
            if s == 0 {
                0
            } else {
                (255 - ((255 - d) * (255 - d)) / s).max(0)
            }
        }
        45 => {
            if d == 0 {
                0
            } else {
                (255 - ((255 - s) * (255 - s)) / d).max(0)
            }
        }
        46 => (s + d).min(255),
        47 => (s + d - 256).clamp(0, 255),
        48 => (interp_c(s as u8) + interp_c(d as u8)).min(255),
        49 => (d + 2 * s - 256).clamp(0, 255),
        50 => s ^ d,
        54 => (d - s).max(0),
        _ => return None,
    };
    Some(v as u8)
}

// ------------------------------------------------------------------ HSL-Modi (11..14)

fn lum(r: i64, g: i64, b: i64) -> i64 {
    (4915 * r + 9667 * g + 1802 * b + 8192) >> 14
}

fn cdiv(a: i64, b: i64) -> i64 {
    if b == 0 { 0 } else { a / b }
}

fn clip_color(r: i64, g: i64, b: i64) -> [i64; 3] {
    let l = lum(r, g, b);
    let n = r.min(g).min(b);
    let x = r.max(g).max(b);
    let mut c = [r, g, b];
    if n < 0 {
        c = c.map(|v| if l != n { l + cdiv((v - l) * l + (l - n).div_euclid(2), l - n) } else { l });
    }
    if x > 255 {
        c = c.map(|v| if x != l { l + cdiv((v - l) * (255 - l) + (x - l).div_euclid(2), x - l) } else { l });
    }
    c.map(|v| v.clamp(0, 255))
}

fn set_lum(c: [i64; 3], l: i64) -> [i64; 3] {
    let d = l - lum(c[0], c[1], c[2]);
    clip_color(c[0] + d, c[1] + d, c[2] + d)
}

fn sat(c: [i64; 3]) -> i64 {
    c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2])
}

fn set_sat(c: [i64; 3], s: i64) -> [i64; 3] {
    let mx = c[0].max(c[1]).max(c[2]);
    let mn = c[0].min(c[1]).min(c[2]);
    let rng = mx - mn;
    let mut out = [0; 3];
    if rng > 0 {
        for i in 0..3 {
            out[i] = ((c[i] - mn) * s + rng.div_euclid(2)).div_euclid(rng);
        }
    }
    out
}

/// Mischfarbe `B(Cs, Cd)` für RGB-Tripel (alle Modi mit Mischfarbe; sonst die Quellfarbe).
pub fn blend_rgb(mode: BlendMode, s: [u8; 3], d: [u8; 3]) -> [u8; 3] {
    if let (Some(r), Some(g), Some(b)) =
        (blend_channel(mode, s[0], d[0]), blend_channel(mode, s[1], d[1]), blend_channel(mode, s[2], d[2]))
    {
        return [r, g, b];
    }
    let si = s.map(|v| v as i64);
    let di = d.map(|v| v as i64);
    let out = match mode.0 {
        11 => set_lum(set_sat(si, sat(di)), lum(di[0], di[1], di[2])),
        12 => set_lum(set_sat(di, sat(si)), lum(di[0], di[1], di[2])),
        13 => set_lum(si, lum(di[0], di[1], di[2])),
        14 => set_lum(di, lum(si[0], si[1], si[2])),
        51 => return [s[0], d[1], d[2]],
        52 => return [d[0], s[1], d[2]],
        53 => return [d[0], d[1], s[2]],
        _ => return s,
    };
    out.map(|v| v as u8)
}

// ------------------------------------------------------------------ Source-over

/// Variante von Schritt C für den Modus Normal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OverVariant {
    /// Skalare Formel (`@006f53c0`); im Pfadpuffer (Strich über Füllung) und bei Stempel-Blits.
    Scalar,
    /// MMX-Rundung (`@006f5980`), aktiv bei MMX-Prozessoren; Seitenkomposition (316 statt 301
    /// von 372 Seiten exakt).
    #[default]
    Mmx,
}

/// Schritt C skalar (`@006f53c0`): Farbe `rgb` mit Deckung `a` über `dst`.
/// `a = 0` → `dst` unverändert; `ad = 0` oder `a = 255` → `(rgb, a)`.
#[inline]
pub fn over_scalar(dst: Rgba8, rgb: [u8; 3], a: u8) -> Rgba8 {
    if a == 0 {
        return dst;
    }
    let ad = dst.a;
    if ad == 0 || a == 255 {
        return Rgba8::new(rgb[0], rgb[1], rgb[2], a);
    }
    let (ar, w) = if ad == 255 {
        (255u32, a as u32)
    } else {
        let ar = a as u32 + ad as u32 - mul255(ad as u32, a as u32);
        (ar, DIV[a as usize][ar as usize] as u32)
    };
    let ch = |c: u8, d: u8| {
        let (c, d) = (c as i32, d as i32);
        let x = (c - d) * w as i32 + 128 + 255 * d;
        ((x + (x >> 8)) >> 8) as u8
    };
    Rgba8::new(ch(rgb[0], dst.r), ch(rgb[1], dst.g), ch(rgb[2], dst.b), ar as u8)
}

/// MMX-Lerp `(256·d + 127 − (d·w + (d·w>>8)) + s·w + (s·w>>8)) >> 8`, auf 0..255 begrenzt.
#[inline]
pub fn lerp_mmx(d: u8, s: u8, w: u32) -> u8 {
    let (d, s, w) = (d as i32, s as i32, w as i32);
    let dw = d * w;
    let sw = s * w;
    ((256 * d + 127 - (dw + (dw >> 8)) + sw + (sw >> 8)) >> 8).clamp(0, 255) as u8
}

/// Schritt C mit MMX-Rundung (`@006f5580`/`@006f5710`): Alpha und Gewicht wie skalar, Farbe per
/// [`lerp_mmx`] (bis ±1 anders).
#[inline]
pub fn over_mmx(dst: Rgba8, rgb: [u8; 3], a: u8) -> Rgba8 {
    if a == 0 {
        return dst;
    }
    let ad = dst.a;
    if ad == 0 || a == 255 {
        return Rgba8::new(rgb[0], rgb[1], rgb[2], a);
    }
    let (ar, w) = if ad == 255 {
        (255u32, a as u32)
    } else {
        let ar = a as u32 + ad as u32 - mul255(ad as u32, a as u32);
        (ar, DIV[a as usize][ar as usize] as u32)
    };
    Rgba8::new(lerp_mmx(dst.r, rgb[0], w), lerp_mmx(dst.g, rgb[1], w), lerp_mmx(dst.b, rgb[2], w), ar as u8)
}

/// Schritt B: wirksame Deckung aus Quellalpha, Deckkraft-Byte und Maske.
#[inline]
pub fn effective_alpha(a_src: u8, op: u8, mask: u8) -> u8 {
    match (mask != 255, op != 255) {
        (true, true) => mul(mul(mask, op), a_src),
        (true, false) => mul(a_src, mask),
        (false, true) => mul(a_src, op),
        (false, false) => a_src,
    }
}

#[inline]
fn step_a(s: u8, b: u8, ad: u8) -> u8 {
    let (s, b, ad) = (s as i32, b as i32, ad as i32);
    let v = if b >= s { s + ((b - s) * ad + 127) / 255 } else { s - ((s - b) * ad + 127) / 255 };
    v as u8
}

/// Ein Quellpixel mit Modus, Deckkraft-Byte `op` und Maske `mask` auf `dst` mischen
/// (vollständig wie `blend.blend_pixel`). `variant` wirkt nur auf den Modus Normal; alle anderen
/// Modi enden skalar.
pub fn blend_pixel(dst: Rgba8, src: Rgba8, mode: BlendMode, op: u8, mask: u8, variant: OverVariant) -> Rgba8 {
    let a = effective_alpha(src.a, op, mask);
    let ad = dst.a;
    match mode.0 {
        17 => {
            // Invert: Quellfarbe = 255 − Ziel, dann normales over (Laufzeittest 07 T1)
            if a == 0 {
                return dst;
            }
            return over_scalar(dst, [255 - dst.r, 255 - dst.g, 255 - dst.b], a);
        }
        25 => {
            // Erase
            if a == 0 {
                return dst;
            }
            if a == 255 {
                return Rgba8::TRANSPARENT_WHITE;
            }
            return Rgba8::new(dst.r, dst.g, dst.b, mul(ad, 255 - a));
        }
        29 => {
            // Tint: Mischung Ziel → Quelle mit Gewicht w = 255 − Grau(Ziel), MMX-Rundung
            let w = 255 - gray_level(dst.r, dst.g, dst.b) as u32;
            let cs = [lerp_mmx(dst.r, src.r, w), lerp_mmx(dst.g, src.g, w), lerp_mmx(dst.b, src.b, w)];
            return over_scalar(dst, cs, a);
        }
        _ => {}
    }
    let mut rgb = [src.r, src.g, src.b];
    if mode.0 != 0 && ad != 0 {
        let b = blend_rgb(mode, rgb, [dst.r, dst.g, dst.b]);
        for i in 0..3 {
            rgb[i] = step_a(rgb[i], b[i], ad);
        }
    }
    if mode.0 == 0 && variant == OverVariant::Mmx { over_mmx(dst, rgb, a) } else { over_scalar(dst, rgb, a) }
}

/// Kurzform für die Seitenkomposition: Quelle mit Modus und Deckkraft auf das Ziel
/// (Normal mit MMX-Rundung, ohne Maske).
#[inline]
pub fn blend(mode: BlendMode, src: Rgba8, dst: Rgba8, opacity: Opacity) -> Rgba8 {
    blend_pixel(dst, src, mode, opacity.0, 255, OverVariant::Mmx)
}

/// Zeilenvariante: `src` auf `dst` (gleiche Länge; überzählige Pixel werden ignoriert).
/// Quellpixel mit Alpha 0 werden übersprungen (wie die Referenz `draw_blend`).
pub fn blend_row(mode: BlendMode, src: &[Rgba8], dst: &mut [Rgba8], opacity: Opacity, variant: OverVariant) {
    let op = opacity.0;
    if mode.0 == 0 && op == 255 {
        for (d, s) in dst.iter_mut().zip(src) {
            if s.a == 255 {
                *d = *s;
            } else if s.a != 0 {
                *d = if variant == OverVariant::Mmx {
                    over_mmx(*d, [s.r, s.g, s.b], s.a)
                } else {
                    over_scalar(*d, [s.r, s.g, s.b], s.a)
                };
            }
        }
        return;
    }
    for (d, s) in dst.iter_mut().zip(src) {
        if s.a != 0 {
            *d = blend_pixel(*d, *s, mode, op, 255, variant);
        }
    }
}

/// Zeilenvariante mit Maske (je Pixel ein Byte).
pub fn blend_row_masked(
    mode: BlendMode,
    src: &[Rgba8],
    mask: &[u8],
    dst: &mut [Rgba8],
    opacity: Opacity,
    variant: OverVariant,
) {
    for ((d, s), m) in dst.iter_mut().zip(src).zip(mask) {
        if s.a != 0 && *m != 0 {
            *d = blend_pixel(*d, *s, mode, opacity.0, *m, variant);
        }
    }
}

// ------------------------------------------------------------------ Bild-auf-Bild (Effekte, 05 §2.5)

/// Original-„over“ für ganze Pixel (`@0077ad90` u. a.): `A = at + ab − div255(at·ab)`,
/// `r = ⌊255·at/A⌋`, `C = div255((Ct − Cb)·r + Cb·255)`. Sonderfälle: `at = 255` oder `ab = 0` →
/// oben; `at = 0` → unten; Ergebnis mit Alpha 0 → `(0, 0, 0, 0)`.
pub fn orig_over(top: Rgba8, bottom: Rgba8) -> Rgba8 {
    let (at, ab) = (top.a as u32, bottom.a as u32);
    let out = if at == 0 {
        bottom
    } else if at == 255 || ab == 0 {
        top
    } else {
        let a = at + ab - div255(at * ab);
        let r = (255 * at) / a.max(1);
        let c = |t: u8, b: u8| div255((t as u32) * r + (b as u32) * (255 - r)) as u8;
        Rgba8::new(c(top.r, bottom.r), c(top.g, bottom.g), c(top.b, bottom.b), a as u8)
    };
    if out.a == 0 { Rgba8::TRANSPARENT } else { out }
}

/// Wie [`orig_over`], Farbe mit MMX-Rundung (`@006f5980`; Objekt auf Effektebene in CS6).
pub fn orig_over_mmx(top: Rgba8, bottom: Rgba8) -> Rgba8 {
    let (at, ab) = (top.a as u32, bottom.a as u32);
    let out = if at == 0 {
        bottom
    } else if at == 255 || ab == 0 {
        top
    } else {
        let (a, w) = if ab == 255 {
            (255, at)
        } else {
            let a = at + ab - div255(at * ab);
            (a, (255 * at) / a.max(1))
        };
        Rgba8::new(lerp_mmx(bottom.r, top.r, w), lerp_mmx(bottom.g, top.g, w), lerp_mmx(bottom.b, top.b, w), a as u8)
    };
    if out.a == 0 { Rgba8::TRANSPARENT } else { out }
}

/// Untere Ebene **hinter** das Objekt mischen: `wb = ⌊255·(A − at)/A⌋`,
/// `C = div255((Cb − Ct)·wb + Ct·255)`; Sonderfälle wie [`orig_over`].
pub fn behind(top: Rgba8, bottom: Rgba8) -> Rgba8 {
    let (at, ab) = (top.a as u32, bottom.a as u32);
    let out = if at == 0 {
        bottom
    } else if at == 255 || ab == 0 {
        top
    } else {
        let a = at + ab - div255(at * ab);
        let wb = (255 * (a - at)) / a.max(1);
        let c = |t: u8, b: u8| div255((b as u32) * wb + (t as u32) * (255 - wb)) as u8;
        Rgba8::new(c(top.r, bottom.r), c(top.g, bottom.g), c(top.b, bottom.b), a as u8)
    };
    if out.a == 0 { Rgba8::TRANSPARENT } else { out }
}

/// Exakt gerundete Mischung der Photoshop-Effekte (PSL-Bibliothek): `A = at + round(ab·(255−at)/255)`,
/// `C = round((at·Ct + (A−at)·Cb)/A)`, ,5 aufgerundet.
pub fn over_exact(top: Rgba8, bottom: Rgba8) -> Rgba8 {
    let (at, ab) = (top.a as u32, bottom.a as u32);
    let a = at + (ab * (255 - at) + 127) / 255;
    if a == 0 {
        return Rgba8::TRANSPARENT;
    }
    let c = |t: u8, b: u8| ((2 * (at * t as u32 + (a - at) * b as u32) + a) / (2 * a)) as u8;
    Rgba8::new(c(top.r, bottom.r), c(top.g, bottom.g), c(top.b, bottom.b), a as u8)
}

/// Nur für CS5-Altdaten: exakt gerundet mit ,5 **abgerundet** (05 §2.5, `fx.over_place`).
pub fn over_place_cs5(top: Rgba8, bottom: Rgba8) -> Rgba8 {
    let (at, ab) = (top.a as u32, bottom.a as u32);
    let a = at + (ab * (255 - at) + 127) / 255;
    if a == 0 {
        return Rgba8::new(0, 0, 0, 0);
    }
    let c = |t: u8, b: u8| ((2 * (at * t as u32 + (a - at) * b as u32) + a - 1) / (2 * a)) as u8;
    Rgba8::new(c(top.r, bottom.r), c(top.g, bottom.g), c(top.b, bottom.b), a as u8)
}

/// Interner Modus 6 „Farbe über, Alpha bleibt“ (`@006f1370`): Farbe wie [`orig_over`],
/// Ergebnis-Alpha = Zielalpha (Zielalpha 0 → transparent).
pub fn color_over_keep_alpha(top: Rgba8, bottom: Rgba8) -> Rgba8 {
    let o = orig_over(top, bottom);
    if bottom.a == 0 { Rgba8::TRANSPARENT } else { Rgba8::new(o.r, o.g, o.b, bottom.a) }
}

/// Farbfüllung im Modus Normal (`StBlendConstSrcPreserveAlpha`): `C = div255((Cf − C)·a + C·255)`,
/// Alpha bleibt.
pub fn color_fill_normal(dst: Rgba8, color: [u8; 3], a: u8) -> Rgba8 {
    let a = a as u32;
    let c = |f: u8, d: u8| div255(f as u32 * a + d as u32 * (255 - a)) as u8;
    Rgba8::new(c(color[0], dst.r), c(color[1], dst.g), c(color[2], dst.b), dst.a)
}

// ------------------------------------------------------------------ Auflösen (dissolve)

/// Zufallsgenerator von MSVCRT (`rand()`): `x = x·214013 + 2531011`, Ergebnis `(x >> 16) & 0x7FFF`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MsvcRand {
    pub state: u32,
}

impl MsvcRand {
    /// Startwert wie `srand(seed)`.
    pub const fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    /// Nächster Wert 0..=32767 (`rand()`).
    pub fn rand(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(214_013).wrapping_add(2_531_011);
        (self.state >> 16) & 0x7FFF
    }
}

/// Auflösen (7, 07 §7.3): `as = 255` → Quelle; `as = 0` → Ziel; sonst `rand() % 255 ≤ as` ?
/// Quelle : Ziel; Ergebnis-Alpha 255. Liefert `true` für „Quelle“.
/// Hinweis: Bei voller Deckung verhält sich Dissolve wie Normal (Laufzeit T1); [`blend_pixel`]
/// behandelt 7 deshalb wie die Referenz ohne Zufall.
pub fn dissolve_pick(a_src: u8, rng: &mut MsvcRand) -> bool {
    match a_src {
        255 => true,
        0 => false,
        a => rng.rand() % 255 <= a as u32,
    }
}

// ------------------------------------------------------------------ Pinsel: Überlappung und Promille-Umweg

/// Überlappungstabelle `T[n][A]` (07 §6.5): kleinstes `a`, für das `n`-mal `u = u + a − mul(u,a)`
/// (ab `u = 0`) mindestens `A` ergibt; 255, wenn keines reicht.
pub fn overlap_alpha(n: u32, target: u8) -> u8 {
    for a in 0..=255u32 {
        let mut u = 0u32;
        for _ in 0..n {
            u = u + a - mul255(u, a);
        }
        if u >= target as u32 {
            return a as u8;
        }
    }
    255
}

/// Promille-Umweg der Stempeldeckkraft (`@00785750`/`@00786290`, B): `⌊⌊max(1,L)·1000/255⌋·255/1000⌋`,
/// also `L − 1` außer bei Vielfachen von 51. Nicht „optimieren“.
pub fn stamp_opacity_roundtrip(l: u8) -> u8 {
    let l = (l as u32).max(1);
    ((l * 1000 / 255) * 255 / 1000) as u8
}

/// Wirksame Stempeldeckkraft eines Pinsels (`stroke.py`): `L = ⌊BCNi·255/1000⌋`; bei `BFBi = 1`
/// `L = T[n][L]` mit `n = max(1, 1000/BSPi)`, wenn `BSPi·D′ > 1000`, sonst `D′` (höchstens 100);
/// danach der Promille-Umweg.
pub fn brush_stamp_opacity(bcni_permille: u32, bfbi: u32, bspi: u32, d_prime: f64) -> u8 {
    let l = (bcni_permille.min(1000) * 255 / 1000) as u8;
    let o = if bfbi == 1 {
        let nov = if bspi as f64 * d_prime > 1000.0 { (1000 / bspi.max(1)).max(1) as f64 } else { d_prime };
        let n = if nov.is_finite() { nov.clamp(0.0, 100.0) as u32 } else { 100 };
        overlap_alpha(n.min(100), l)
    } else {
        l
    };
    stamp_opacity_roundtrip(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolation_curve_ends() {
        let c = interpolation_curve();
        assert_eq!((c[0], c[255]), (0, 128));
        assert!(c.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn names_and_numbers() {
        assert_eq!(BlendMode::ALL.len(), 46);
        assert_eq!(BlendMode::from_name("Multiply"), Some(BlendMode::MULTIPLY));
        assert_eq!(BlendMode(54).name(), Some("subtract"));
        assert_eq!(BlendMode(1).name(), None);
        assert_eq!(BlendMode::from_file(Some(55)), BlendMode::NORMAL);
        assert_eq!(BlendMode::from_file(None), BlendMode::NORMAL);
        assert_eq!(BlendMode::from_file(Some(12)), BlendMode::SATURATION);
    }

    #[test]
    fn over_basics() {
        let d = Rgba8::new(0, 0, 255, 255);
        assert_eq!(over_scalar(d, [255, 0, 0], 0), d);
        assert_eq!(over_scalar(d, [255, 0, 0], 255), Rgba8::new(255, 0, 0, 255));
        let h = over_scalar(d, [255, 0, 0], 128);
        assert_eq!(h, Rgba8::new(128, 0, 127, 255));
        assert_eq!(blend(BlendMode::NORMAL, Rgba8::new(255, 0, 0, 128), d, Opacity(255)).a, 255);
        assert_eq!(orig_over(Rgba8::new(1, 2, 3, 0), Rgba8::new(9, 9, 9, 0)), Rgba8::TRANSPARENT);
        assert_eq!(stamp_opacity_roundtrip(255), 255);
        assert_eq!(stamp_opacity_roundtrip(200), 199);
        assert_eq!(stamp_opacity_roundtrip(0), 0);
        assert_eq!(overlap_alpha(1, 200), 200);
        let mut r = MsvcRand::new(1);
        assert_eq!(r.rand(), 41);
        assert_eq!(r.rand(), 18467);
    }

    #[test]
    fn rows_equal_pixels() {
        let src: Vec<Rgba8> = (0..64u32).map(|i| Rgba8::new((i * 4) as u8, (255 - i * 3) as u8, (i * 7) as u8, (i * 4) as u8)).collect();
        let base: Vec<Rgba8> = (0..64u32).map(|i| Rgba8::new((i * 3) as u8, 100, (200 - i) as u8, (i * 2 + 100) as u8)).collect();
        for mode in [BlendMode::NORMAL, BlendMode::MULTIPLY, BlendMode::HUE, BlendMode::ERASE] {
            for op in [255u8, 128] {
                let mut row = base.clone();
                blend_row(mode, &src, &mut row, Opacity(op), OverVariant::Mmx);
                for i in 0..64 {
                    let want = if src[i].a == 0 { base[i] } else { blend_pixel(base[i], src[i], mode, op, 255, OverVariant::Mmx) };
                    assert_eq!(row[i], want, "{mode:?} {op} {i}");
                }
            }
        }
    }
}

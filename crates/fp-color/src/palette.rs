//! Paletten und Farbabstand (Spezifikation 08 §2.4/§2.7, 14 §2.1/§2.4).
//!
//! - [`Palette`]: Farben (`Vec<Rgba8>`) und optionaler Transparenzindex.
//! - Farbabstand wie im Original: quadratischer RGB-Abstand ([`dist2_rgb`]), bei RGBA-Paletten
//!   gleich gewichtet über R, G, B **und** A ([`dist2_rgba`]); die LZW-Glättung nutzt den
//!   L1-Abstand ([`dist_l1_rgb`]).
//! - Nächste Farbe: erschöpfende Suche, erste minimale Distanz, strikt `<`, Startwert `0xFFFFF`,
//!   Abbruch bei 0 (`@00741770`, `@007416b0`). Die Zellen-Heuristik der Export-Quantisierung
//!   (8×8×8-Zellen) gehört zu `fp-export`.
//! - Sortieren nach Farbe (Farbfelder, 35103): Auswahlsortierung nach dem kleinsten Byte-Farbton,
//!   bei Gleichstand der **letzte** Kandidat ([`sort_by_hue`]).

use crate::Rgba8;
use crate::models::{gray_level, rgb_to_hsb8};

/// Palette: Farben in Reihenfolge und optionaler Transparenzindex.
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Palette {
    pub colors: Vec<Rgba8>,
    pub transparent_index: Option<usize>,
}

impl Palette {
    pub fn new(colors: Vec<Rgba8>) -> Self {
        Self {
            colors,
            transparent_index: None,
        }
    }

    /// Mit Transparenzindex (wird verworfen, wenn er außerhalb liegt).
    pub fn with_transparent(colors: Vec<Rgba8>, index: Option<usize>) -> Self {
        let transparent_index = index.filter(|&i| i < colors.len());
        Self {
            colors,
            transparent_index,
        }
    }

    pub fn len(&self) -> usize {
        self.colors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.colors.is_empty()
    }

    /// Index der nächsten Farbe nach quadratischem RGB-Abstand (erste minimale, strikt `<`).
    pub fn nearest_rgb(&self, c: Rgba8) -> Option<usize> {
        nearest_by(&self.colors, |p| dist2_rgb(*p, c))
    }

    /// Nächste Farbe für RGBA-Paletten (PNG8 mit Alpha, 08 §2.7): Alpha < 9 → Index 0, sonst
    /// quadratischer Abstand über R, G, B und A.
    pub fn nearest_rgba(&self, c: Rgba8) -> Option<usize> {
        if self.colors.is_empty() {
            return None;
        }
        if c.a < 9 {
            return Some(0);
        }
        nearest_by(&self.colors, |p| dist2_rgba(*p, c))
    }

    /// Index der ersten exakt gleichen Farbe (RGB und Alpha).
    pub fn find_exact(&self, c: Rgba8) -> Option<usize> {
        self.colors.iter().position(|p| *p == c)
    }
}

fn nearest_by<F: Fn(&Rgba8) -> u32>(colors: &[Rgba8], d: F) -> Option<usize> {
    let mut best = 0xFFFFFu32;
    let mut idx = None;
    for (i, p) in colors.iter().enumerate() {
        let v = d(p);
        if v < best {
            best = v;
            idx = Some(i);
            if v == 0 {
                break;
            }
        }
    }
    // Abstände ≥ 0xFFFFF kommen bei RGBA nicht vor (max. 4·255² = 260.100); zur Sicherheit erste Farbe
    idx.or(if colors.is_empty() { None } else { Some(0) })
}

/// Quadratischer RGB-Abstand.
#[inline]
pub fn dist2_rgb(a: Rgba8, b: Rgba8) -> u32 {
    let d = |x: u8, y: u8| (x as i32 - y as i32).pow(2) as u32;
    d(a.r, b.r) + d(a.g, b.g) + d(a.b, b.b)
}

/// Quadratischer RGBA-Abstand (alle vier Kanäle gleich gewichtet).
#[inline]
pub fn dist2_rgba(a: Rgba8, b: Rgba8) -> u32 {
    dist2_rgb(a, b) + (a.a as i32 - b.a as i32).pow(2) as u32
}

/// L1-Abstand über R, G, B (LZW-Glättung, 08 §2.6).
#[inline]
pub fn dist_l1_rgb(a: Rgba8, b: Rgba8) -> u32 {
    a.r.abs_diff(b.r) as u32 + a.g.abs_diff(b.g) as u32 + a.b.abs_diff(b.b) as u32
}

/// Luminanz für Palettensortierung: `(R·109 + G·366 + B·37) >> 9` (08 §2.6).
#[inline]
pub fn luminance(c: Rgba8) -> u8 {
    gray_level(c.r, c.g, c.b)
}

/// Sortieren nach Farbe wie das Farbfelder-Menü (`@006741a0`, 14 §2.4): wiederholt den Eintrag mit
/// dem kleinsten Byte-Farbton H8 wählen (Vergleich `≤`, bei Gleichstand also der letzte) und
/// anhängen. Graue Farben (H8 = 0) landen vorn. Der gewählte Eintrag wird aus der Quelle entfernt (A).
pub fn sort_by_hue(colors: &[Rgba8]) -> Vec<Rgba8> {
    let mut rest: Vec<(i32, Rgba8)> = colors
        .iter()
        .map(|c| (rgb_to_hsb8(c.r, c.g, c.b).0 & 0xff, *c))
        .collect();
    let mut out = Vec::with_capacity(rest.len());
    while !rest.is_empty() {
        let mut best = 0;
        for i in 1..rest.len() {
            if rest[i].0 <= rest[best].0 {
                best = i;
            }
        }
        out.push(rest.remove(best).1);
    }
    out
}

/// Kanal auf das nächste Vielfache von 51 einrasten („Websicher machen“, 14 §1.10, A).
#[inline]
pub fn snap_web_safe_channel(v: u8) -> u8 {
    (((v as u32 + 25) / 51) * 51) as u8
}

/// Farbe auf die nächste Web-216-Farbe einrasten (je Kanal, A).
pub fn snap_web_safe(c: Rgba8) -> Rgba8 {
    Rgba8::new(
        snap_web_safe_channel(c.r),
        snap_web_safe_channel(c.g),
        snap_web_safe_channel(c.b),
        c.a,
    )
}

/// Eingebauter Farbfeldsatz „Graustufen“ (Pref 2, `@00673700`): 256 Graustufen von 255 bis 0.
pub fn grayscale_swatches() -> Vec<Rgba8> {
    (0..=255u8).rev().map(|v| Rgba8::rgb(v, v, v)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest() {
        let p = Palette::new(vec![
            Rgba8::rgb(0, 0, 0),
            Rgba8::rgb(255, 255, 255),
            Rgba8::rgb(250, 250, 250),
        ]);
        assert_eq!(p.nearest_rgb(Rgba8::rgb(200, 200, 200)), Some(2));
        assert_eq!(p.nearest_rgb(Rgba8::rgb(10, 0, 0)), Some(0));
        let q = Palette::new(vec![Rgba8::rgb(1, 1, 1), Rgba8::rgb(1, 1, 1)]);
        assert_eq!(q.nearest_rgb(Rgba8::rgb(1, 1, 1)), Some(0));
        assert_eq!(p.nearest_rgba(Rgba8::new(255, 255, 255, 3)), Some(0));
        assert_eq!(Palette::default().nearest_rgb(Rgba8::BLACK), None);
        assert_eq!(
            Palette::with_transparent(vec![Rgba8::BLACK], Some(5)).transparent_index,
            None
        );
    }

    #[test]
    fn hue_sort() {
        let c = [
            Rgba8::rgb(0, 0, 255),
            Rgba8::rgb(255, 0, 0),
            Rgba8::rgb(128, 128, 128),
            Rgba8::rgb(0, 255, 0),
            Rgba8::rgb(1, 1, 1),
        ];
        let s = sort_by_hue(&c);
        // Grau (H8 = 0) und Rot (H8 = 0): bei Gleichstand gewinnt der letzte → (1,1,1) zuerst
        assert_eq!(s[0], Rgba8::rgb(1, 1, 1));
        assert_eq!(s[3], Rgba8::rgb(0, 255, 0));
        assert_eq!(s[4], Rgba8::rgb(0, 0, 255));
        assert_eq!(grayscale_swatches()[0], Rgba8::WHITE);
        assert_eq!(snap_web_safe_channel(25), 0);
        assert_eq!(snap_web_safe_channel(26), 51);
    }
}

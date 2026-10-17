//! Hilfen für [`Argb`] und [`Rgba8`]: Konstruktoren, Hex-Notation, Vormultiplikation.
//!
//! Hex-Notation (Spezifikation 14 §1.12): Voreinstellungen `#rrggbb` bzw. `#rrggbbaa` in
//! Kleinbuchstaben (Lesen unabhängig von der Schreibweise), Pipette/Zwischenablage `RRGGBB`
//! (Großbuchstaben, ohne `#`), HTML `#rrggbb`.

use crate::round::mul255;
use crate::{Argb, Rgba8};

impl Argb {
    /// Aus den Kanälen.
    pub const fn new(a: u8, r: u8, g: u8, b: u8) -> Argb {
        Argb(((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | b as u32)
    }

    /// Gleiche Farbe mit anderem Alpha.
    pub const fn with_alpha(self, a: u8) -> Argb {
        Argb((self.0 & 0x00FF_FFFF) | ((a as u32) << 24))
    }
}

impl From<Rgba8> for Argb {
    fn from(c: Rgba8) -> Argb {
        c.to_argb()
    }
}

impl From<Argb> for Rgba8 {
    fn from(c: Argb) -> Rgba8 {
        c.to_rgba8()
    }
}

impl Rgba8 {
    /// Deckendes Schwarz.
    pub const BLACK: Rgba8 = Rgba8::new(0, 0, 0, 255);
    /// Deckendes Weiß.
    pub const WHITE: Rgba8 = Rgba8::new(255, 255, 255, 255);
    /// „Keine Farbe“ in Original-Dateien: `0x00FFFFFF` (transparentes Weiß).
    pub const TRANSPARENT_WHITE: Rgba8 = Rgba8::new(255, 255, 255, 0);

    /// Deckende Farbe.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Rgba8 {
        Rgba8::new(r, g, b, 255)
    }

    /// Gleiche Farbe mit anderem Alpha.
    pub const fn with_alpha(self, a: u8) -> Rgba8 {
        Rgba8::new(self.r, self.g, self.b, a)
    }

    /// Kanäle als Feld `[r, g, b, a]`.
    pub const fn to_array(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Aus einem Feld `[r, g, b, a]`.
    pub const fn from_array(a: [u8; 4]) -> Rgba8 {
        Rgba8::new(a[0], a[1], a[2], a[3])
    }

    /// Vormultiplizieren mit `mul255` (gerundet).
    pub const fn premultiply(self) -> Rgba8 {
        let a = self.a as u32;
        Rgba8::new(
            mul255(self.r as u32, a) as u8,
            mul255(self.g as u32, a) as u8,
            mul255(self.b as u32, a) as u8,
            self.a,
        )
    }

    /// Vormultiplikation rückgängig machen (gerundet, auf 255 begrenzt; Alpha 0 → transparent).
    pub fn unpremultiply(self) -> Rgba8 {
        if self.a == 0 {
            return Rgba8::TRANSPARENT;
        }
        let a = self.a as u32;
        let f = |c: u8| ((c as u32 * 255 + a / 2) / a).min(255) as u8;
        Rgba8::new(f(self.r), f(self.g), f(self.b), self.a)
    }

    /// `#rrggbb` (Kleinbuchstaben, Voreinstellungen/HTML).
    pub fn to_hex_rgb(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// `#rrggbbaa` (Kleinbuchstaben, Voreinstellungen mit Alpha).
    pub fn to_hex_rgba(self) -> String {
        format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
    }

    /// `RRGGBB` (Großbuchstaben, ohne `#`; Pipette „Farbe kopieren“, `@00a1ab00`).
    pub fn to_clipboard_hex(self) -> String {
        format!("{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    /// Liest `#rgb`, `#rrggbb` oder `#rrggbbaa` (mit oder ohne `#`, Groß-/Kleinschreibung egal).
    /// Ohne Alpha-Angabe ist die Farbe deckend. `None` bei ungültigem Text.
    pub fn parse_hex(s: &str) -> Option<Rgba8> {
        let s = s.trim();
        let s = s.strip_prefix('#').unwrap_or(s);
        if !s.bytes().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let byte = |i: usize| u8::from_str_radix(s.get(i..i + 2)?, 16).ok();
        match s.len() {
            3 => {
                let n = |i: usize| {
                    u8::from_str_radix(s.get(i..i + 1)?, 16)
                        .ok()
                        .map(|v| v * 17)
                };
                Some(Rgba8::new(n(0)?, n(1)?, n(2)?, 255))
            }
            6 => Some(Rgba8::new(byte(0)?, byte(2)?, byte(4)?, 255)),
            8 => Some(Rgba8::new(byte(0)?, byte(2)?, byte(4)?, byte(6)?)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip() {
        let c = Rgba8::new(0x12, 0xAB, 0xFF, 0x80);
        assert_eq!(c.to_hex_rgb(), "#12abff");
        assert_eq!(c.to_hex_rgba(), "#12abff80");
        assert_eq!(c.to_clipboard_hex(), "12ABFF");
        assert_eq!(Rgba8::parse_hex("#12ABff80"), Some(c));
        assert_eq!(Rgba8::parse_hex("FFFF99"), Some(Rgba8::rgb(255, 255, 0x99)));
        assert_eq!(Rgba8::parse_hex("#f0a"), Some(Rgba8::rgb(255, 0, 170)));
        assert_eq!(
            Rgba8::parse_hex("#ffffff00"),
            Some(Rgba8::TRANSPARENT_WHITE)
        );
        assert_eq!(Rgba8::parse_hex("#12345"), None);
        assert_eq!(Rgba8::parse_hex("#gg0000"), None);
        assert_eq!(Rgba8::parse_hex("ä€"), None);
    }

    #[test]
    fn argb_helpers() {
        let a = Argb::new(1, 2, 3, 4);
        assert_eq!(a.0, 0x01020304);
        assert_eq!(a.with_alpha(0xFF).0, 0xFF020304);
        assert_eq!(Rgba8::from(a), Rgba8::new(2, 3, 4, 1));
        let p = Rgba8::new(255, 128, 0, 128).premultiply();
        assert_eq!(p, Rgba8::new(128, 64, 0, 128));
        assert_eq!(p.unpremultiply(), Rgba8::new(255, 128, 0, 128));
    }
}

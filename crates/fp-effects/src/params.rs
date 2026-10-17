//! Lesen der Effekt-Parameter aus der Datei-Form (`EPSv/DCEv`: Schlüssel → Text, Kurven als
//! Punktlisten; 05 §2.1).
//!
//! Die Parameter liegen als [`fp_raster::filters::EffectParams`] vor (dieselbe Form, die auch
//! [`fp_raster::Filter::from_effect`] liest). Die Lesehilfen bilden die Python-Referenz nach:
//! Zahlen wie `float(text)`, Ganzzahlen wie `int(float(text))` (abschneiden), Farben
//! `#RRGGBB[AA]` (ohne `AA` deckend). Fehlende oder nicht lesbare Werte ergeben die Vorgabe.

use fp_color::Rgba8;
pub use fp_raster::filters::{EffectParams, ParamValue};

/// Lesesicht auf die Parameter eines Effekts.
#[derive(Debug, Clone, Copy)]
pub struct Params<'a>(pub &'a EffectParams);

impl<'a> Params<'a> {
    /// Rohtext eines Parameters (getrimmt); `None`, wenn er fehlt oder eine Punktliste ist.
    pub fn text(&self, key: &str) -> Option<&'a str> {
        match self.0.get(key) {
            Some(ParamValue::Text(s)) => Some(s.trim()),
            _ => None,
        }
    }

    /// Zahl wie `float(text)`; Vorgabe, wenn der Wert fehlt oder nicht lesbar ist.
    pub fn num(&self, key: &str, default: f64) -> f64 {
        self.text(key)
            .and_then(parse_float)
            .filter(|v| v.is_finite())
            .unwrap_or(default)
    }

    /// Ganzzahl wie `int(float(text))` (abschneiden, gesättigt).
    pub fn int(&self, key: &str, default: i64) -> i64 {
        let v = self.num(key, default as f64);
        v.trunc().clamp(-1.0e15, 1.0e15) as i64
    }

    /// Wahrheitswert (`true`/`1`).
    pub fn flag(&self, key: &str, default: bool) -> bool {
        match self.text(key) {
            Some(s) => s.eq_ignore_ascii_case("true") || s == "1",
            None => default,
        }
    }

    /// Farbe `#RRGGBB[AA]`; Vorgabe bei fehlendem oder ungültigem Wert.
    pub fn color(&self, key: &str, default: Rgba8) -> Rgba8 {
        self.text(key).and_then(parse_color).unwrap_or(default)
    }
}

/// `float(text)` von Python: führende/folgende Leerzeichen erlaubt, sonst das Rust-Format.
pub fn parse_float(s: &str) -> Option<f64> {
    s.trim().parse::<f64>().ok()
}

/// Farbe `#RRGGBB` oder `#RRGGBBAA` (wie `fx.parse_color`; `#` optional).
pub fn parse_color(s: &str) -> Option<Rgba8> {
    let h = s.trim().trim_start_matches('#');
    let v = |i: usize| h.get(i..i + 2).and_then(|t| u8::from_str_radix(t, 16).ok());
    let (r, g, b) = (v(0)?, v(2)?, v(4)?);
    let a = if h.len() >= 8 { v(6)? } else { 255 };
    Some(Rgba8::new(r, g, b, a))
}

/// Farbe als Text `#rrggbb` bzw. `#rrggbbaa` (so schreibt das Original Effektfarben).
pub fn format_color(c: Rgba8) -> String {
    if c.a == 255 {
        format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
    } else {
        format!("#{:02x}{:02x}{:02x}{:02x}", c.r, c.g, c.b, c.a)
    }
}

/// Hilfe für Tests und Aufrufer: Parameter aus `(Schlüssel, Text)`-Paaren.
pub fn params_from_pairs<'s>(pairs: impl IntoIterator<Item = (&'s str, &'s str)>) -> EffectParams {
    pairs
        .into_iter()
        .map(|(k, v)| (k.to_string(), ParamValue::Text(v.to_string())))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reading() {
        let p = params_from_pairs([
            ("A", " 4.7 "),
            ("C", "#ff000080"),
            ("D", "#00ff00"),
            ("B", "true"),
        ]);
        let q = Params(&p);
        assert_eq!(q.int("A", 0), 4);
        assert_eq!(q.num("X", 2.5), 2.5);
        assert_eq!(q.color("C", Rgba8::BLACK), Rgba8::new(255, 0, 0, 128));
        assert_eq!(q.color("D", Rgba8::BLACK), Rgba8::new(0, 255, 0, 255));
        assert!(q.flag("B", false));
        assert_eq!(format_color(Rgba8::new(1, 2, 3, 4)), "#01020304");
        assert_eq!(parse_color("#zz0000"), None);
    }
}

//! Sprachabhängige Formate: Zugriffstasten, Tastenkürzel, Einheiten, Zahlen, Größen, Datum.

use crate::{cached, t, t_args, Value};

// ---------------------------------------------------------------------- Zugriffstasten

/// Text mit Zugriffstaste, zerlegt.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct AccessText {
    /// Anzeigetext ohne `&` (`&&` wird zu `&`).
    pub text: String,
    /// Zugriffstaste (klein geschrieben), z. B. `'d'` für „&Datei“.
    pub key: Option<char>,
    /// Lage der Zugriffstaste im Anzeigetext (Zeichenindex, nicht Byte) zum Unterstreichen.
    pub index: Option<usize>,
}

/// Zerlegt einen Text mit Zugriffstaste: `"&Datei"` → `("Datei", Some('d'))`.
/// `&&` ist ein wörtliches `&`; ein `&` am Ende wird verworfen. Nur das erste `&x` zählt.
pub fn strip_access(raw: &str) -> (String, Option<char>) {
    let a = access_info(raw);
    (a.text, a.key)
}

/// Wie [`strip_access`], zusätzlich mit der Lage der Zugriffstaste.
pub fn access_info(raw: &str) -> AccessText {
    let mut out = AccessText { text: String::with_capacity(raw.len()), key: None, index: None };
    let mut idx = 0usize;
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            match chars.peek().copied() {
                Some('&') => {
                    chars.next();
                    out.text.push('&');
                    idx += 1;
                }
                Some(n) if out.key.is_none() && !n.is_whitespace() => {
                    out.key = Some(n.to_lowercase().next().unwrap_or(n));
                    out.index = Some(idx);
                }
                _ => {}
            }
            continue;
        }
        out.text.push(c);
        idx += 1;
    }
    out
}

/// Übersetzter Text mit Zugriffstaste: `t_access("menu-file")` → „Datei“ + `'d'`.
pub fn t_access(key: &str) -> AccessText {
    access_info(&cached(key).0)
}

// ---------------------------------------------------------------------- Tastenkürzel

/// Tastennamen: (Kennung in `key-<kennung>`, Schreibweisen in deutsch/englisch, klein).
const KEY_ALIASES: &[(&str, &[&str])] = &[
    ("ctrl", &["ctrl", "strg", "control", "cmd", "command"]),
    ("alt", &["alt", "option"]),
    ("shift", &["shift", "umschalt", "umsch"]),
    ("enter", &["enter", "eingabe", "return", "eingabetaste"]),
    ("esc", &["esc", "escape"]),
    ("space", &["space", "leertaste", "leer"]),
    ("backspace", &["backspace", "rück", "rücktaste"]),
    ("tab", &["tab", "tabulator"]),
    ("del", &["del", "delete", "entf", "löschen-taste"]),
    ("ins", &["ins", "insert", "einfg"]),
    ("home", &["home", "pos1"]),
    ("end", &["end", "ende"]),
    ("pageup", &["pageup", "page up", "pgup", "bild auf", "bild↑"]),
    ("pagedown", &["pagedown", "page down", "pgdn", "bild ab", "bild↓"]),
    ("left", &["left", "links", "arrowleft", "nach-links", "nach-links-taste"]),
    ("right", &["right", "rechts", "arrowright", "nach-rechts", "nach-rechts-taste"]),
    ("up", &["up", "oben", "arrowup", "nach-oben", "nach-oben-taste"]),
    ("down", &["down", "unten", "arrowdown", "nach-unten", "nach-unten-taste"]),
];

/// Ziffernblock-Tasten in englischer Kurzform (`NumAdd` …) → Zeichen.
const NUM_KEYS: &[(&str, &str)] = &[("nummultiply", "*"), ("numadd", "+"), ("numsubtract", "-"), ("numdecimal", ","), ("numdivide", "/")];

/// Teilt an `+`; ein abschließendes `+` (Taste Plus) bleibt als eigener Teil erhalten.
fn split_keys(text: &str) -> Vec<&str> {
    let text = text.trim();
    let mut parts = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // „Num +“: ein Plus nach einem Leerzeichen gehört zum Tastennamen
        if bytes[i] == b'+' && i > start && bytes[i - 1] != b' ' {
            parts.push(&text[start..i]);
            start = i + 1;
        }
        i += 1;
    }
    if start < text.len() {
        parts.push(&text[start..]);
    }
    parts
}

fn key_label(token: &str) -> String {
    let lower = token.trim().to_lowercase();
    for (id, aliases) in KEY_ALIASES {
        if aliases.contains(&lower.as_str()) {
            let k = format!("key-{id}");
            let (text, found) = cached(&k);
            return if found { text.to_string() } else { token.trim().to_string() };
        }
    }
    let num = t("key-num");
    if let Some(rest) = lower.strip_prefix("num ") {
        return format!("{num} {}", rest.trim().to_uppercase());
    }
    if let Some((_, c)) = NUM_KEYS.iter().find(|(n, _)| *n == lower) {
        return format!("{num} {c}");
    }
    if let Some(d) = lower.strip_prefix("numpad").or_else(|| lower.strip_prefix("num")).filter(|d| d.len() == 1 && d.chars().all(|c| c.is_ascii_digit())) {
        return format!("{num} {d}");
    }
    token.trim().to_string()
}

/// Tastenkürzel in der aktuellen Sprache. Nimmt deutsche oder englische Schreibweise
/// (`Strg+Umschalt+S`, `Ctrl+Shift+S`, `Alt+F4`, `Strg++`, `Entf`) und mehrere Kürzel mit
/// Komma bzw. Semikolon getrennt (`Strg+Y, Strg+Umschalt+Z`). Unbekannte Teile bleiben.
pub fn shortcut_label(text: &str) -> String {
    let mut id = String::with_capacity(text.len() + 1);
    id.push('\u{2}');
    id.push_str(text);
    crate::derived(&id, || shortcut_label_uncached(text)).to_string()
}

fn shortcut_label_uncached(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 8);
    // mehrere Kürzel: „, “ bzw. „; “ (ein einzelnes „,“ ist die Kommataste)
    let mut rest = text;
    loop {
        let cut = [", ", "; "].iter().filter_map(|s| rest.find(s).map(|i| (i, s.len()))).min();
        let (part, next) = match cut {
            Some((i, l)) => (&rest[..i], Some((&rest[i..i + l], &rest[i + l..]))),
            None => (rest, None),
        };
        let keys: Vec<String> = split_keys(part).into_iter().map(key_label).collect();
        out.push_str(&keys.join("+"));
        match next {
            Some((sep, r)) => {
                out.push_str(sep);
                rest = r;
            }
            None => break,
        }
    }
    out
}

// ---------------------------------------------------------------------- Einheiten

/// Einheit in der aktuellen Sprache: `unit("px")` → `t("unit-px")`. Unbekannt → der Name selbst.
pub fn unit(name: &str) -> String {
    let (text, found) = cached(&format!("unit-{name}"));
    if found { text.to_string() } else { name.to_string() }
}

// ---------------------------------------------------------------------- Zahlen

/// Zahl als Text: höchstens `max` Nachkommastellen (gerundet, Nullen am Ende entfernt), mindestens
/// `min`, Dezimaltrennzeichen `sep`. Ohne `max` höchstens 3 Stellen (wie `Intl.NumberFormat`).
pub(crate) fn number_string(v: f64, min: Option<usize>, max: Option<usize>, sep: &str) -> String {
    if !v.is_finite() {
        return v.to_string();
    }
    let min = min.unwrap_or(0);
    let max = max.unwrap_or(3).max(min);
    let mut s = format!("{v:.max$}");
    if let Some(dot) = s.find('.') {
        let keep = dot + 1 + min;
        while s.len() > keep && s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    if s == "-0" {
        s = "0".to_string();
    }
    if sep != "." { s.replace('.', sep) } else { s }
}

/// Zahl mit `min`…`max` Nachkommastellen im Format der aktuellen Sprache
/// (`fmt_number(1.5, 0, 2)` → „1,5“ bzw. „1.5“).
pub fn fmt_number(v: f64, min_decimals: usize, max_decimals: usize) -> String {
    let (sep, found) = cached("fmt-decimal-separator");
    let sep: &str = if found { &sep } else { "," };
    number_string(v, Some(min_decimals), Some(max_decimals), sep)
}

/// Dateigröße: „512 Byte“, „1,5 KB“, „12,3 MB“, „1,2 GB“ (Faktor 1024, eine Nachkommastelle
/// ab KB, ganze Zahl ab 100).
pub fn fmt_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["kb", "mb", "gb", "tb"];
    if bytes < 1024 {
        return t_args("fmt-size-bytes", &[("n", Value::from(bytes as f64))]);
    }
    let mut v = bytes as f64 / 1024.0;
    let mut i = 0;
    while v >= 1024.0 && i + 1 < UNITS.len() {
        v /= 1024.0;
        i += 1;
    }
    let decimals = if v >= 100.0 { 0 } else { 1 };
    format!("{} {}", fmt_number(v, 0, decimals), unit(UNITS[i]))
}

// ---------------------------------------------------------------------- Datum und Uhrzeit

fn two(n: u32) -> String {
    format!("{n:02}")
}

/// Datum im Format der Sprache (`fmt-date`): de „12.10.2026“, en „10/12/2026“.
pub fn fmt_date(year: i32, month: u32, day: u32) -> String {
    t_args("fmt-date", &[("year", Value::from(year.to_string())), ("month", Value::from(two(month))), ("day", Value::from(two(day)))])
}

/// Uhrzeit im Format der Sprache (`fmt-time`): „14:05“.
pub fn fmt_time(hour: u32, minute: u32) -> String {
    t_args("fmt-time", &[("hour", Value::from(two(hour))), ("minute", Value::from(two(minute)))])
}

/// Datum und Uhrzeit (`fmt-datetime`): de „12.10.2026, 14:05“.
pub fn fmt_datetime(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> String {
    t_args("fmt-datetime", &[("date", Value::from(fmt_date(year, month, day))), ("time", Value::from(fmt_time(hour, minute)))])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_keys() {
        assert_eq!(strip_access("&Datei"), ("Datei".to_string(), Some('d')));
        assert_eq!(strip_access("Speichern &unter..."), ("Speichern unter...".to_string(), Some('u')));
        assert_eq!(strip_access("Drag && Drop"), ("Drag & Drop".to_string(), None));
        assert_eq!(strip_access("&&A&b"), ("&Ab".to_string(), Some('b')));
        assert_eq!(strip_access("Ende&"), ("Ende".to_string(), None));
        assert_eq!(strip_access("&Über"), ("Über".to_string(), Some('ü')));
        let a = access_info("Ö&ffnen");
        assert_eq!((a.key, a.index), (Some('f'), Some(1)));
        assert_eq!(strip_access("ohne"), ("ohne".to_string(), None));
    }

    #[test]
    fn numbers() {
        assert_eq!(number_string(1.5, None, None, ","), "1,5");
        assert_eq!(number_string(2.0, None, None, ","), "2");
        assert_eq!(number_string(0.1 + 0.2, None, None, "."), "0.3");
        assert_eq!(number_string(1.0, Some(2), None, "."), "1.00");
        assert_eq!(number_string(1.23456, None, Some(1), ","), "1,2");
        assert_eq!(number_string(-0.0001, None, Some(2), ","), "0");
        assert_eq!(number_string(1234567.0, None, None, ","), "1234567");
    }

    #[test]
    fn key_split() {
        assert_eq!(split_keys("Strg++"), vec!["Strg", "+"]);
        assert_eq!(split_keys("Ctrl+Shift+S"), vec!["Ctrl", "Shift", "S"]);
        assert_eq!(split_keys("+"), vec!["+"]);
        assert_eq!(split_keys("Num +"), vec!["Num +"]);
    }
}

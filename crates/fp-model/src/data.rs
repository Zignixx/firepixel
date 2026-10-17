//! Schlüssel/Wert-Wörterbücher: Custom Data (`DTAv`) und Effektparameter (`EPSv`).
//!
//! Aufbau (02 §5.9, 03 §10.8): `DTAv{DCEv{DCKs Schlüssel, Wert}…}`. Ein Wert ist
//! * ein Skalar als Text `DCVs` (auch Zahlen und Wahrheitswerte stehen als Text),
//! * eine Liste `GVRv{VVLs | GDTv | GVRv …}`,
//! * ein Wörterbuch `GDTv{DCEv…}`,
//! * eine Punktliste `GPLv{GPTv{XLCf YLCf}…}` (Kurven-Effekt).
//!
//! Bei Listen schreibt das Original zusätzlich ein `DCVs` mit der `toSource`-Textform; beim Lesen gewinnt
//! die Struktur (03 §10.12). Skalare werden erst beim Lesen über JS typisiert: `"true"/"false"` →
//! boolean, kanonische Zahlschreibweise → Zahl, sonst String ([`DataValue::typed`]).

use crate::raw::Extra;
use crate::serde_ext;
use fp_geom::Point;
use serde::{Deserialize, Serialize};

/// Variante des Eintrag-Containers: `DCEv` (üblich) oder `DCFv` (vom Leser akzeptierte Variante).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum EntryTag {
    #[default]
    Dce,
    Dcf,
}

/// Wert eines Eintrags.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DataValue {
    /// Kein Wert (Eintrag nur mit Schlüssel).
    None,
    /// Skalar als Text (`DCVs` bzw. `VVLs` in Listen).
    Scalar(String),
    /// Liste (`GVRv`).
    List(DataList),
    /// Wörterbuch (`GDTv`).
    Dict(DataDict),
    /// Punktliste (`GPLv`).
    Points(PointList),
}

/// Typisierter Skalar nach der Original-Leseregel (03 §10.12).
#[derive(Clone, Debug, PartialEq)]
pub enum TypedScalar {
    Bool(bool),
    Number(f64),
    String(String),
}

/// Ein Eintrag `DCEv{DCKs …}`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DataEntry {
    #[serde(default, skip_serializing_if = "is_default_tag")]
    pub tag: EntryTag,
    /// Schlüssel (`DCKs`).
    pub key: String,
    /// Wert. Bei Listen/Wörterbüchern/Punkten die Struktur.
    pub value: DataValue,
    /// Zusätzliche Textform (`DCVs` neben einer Struktur), z. B. `[ { name:"Color", … } ]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

fn is_default_tag(t: &EntryTag) -> bool {
    *t == EntryTag::Dce
}

/// Liste `GVRv`.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct DataList {
    pub items: Vec<DataValue>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

/// Wörterbuch `GDTv` bzw. `DTAv`: Einträge in Dateireihenfolge.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct DataDict {
    pub entries: Vec<DataEntry>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

/// Punktliste `GPLv{GPTv{XLCf YLCf}…}`.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct PointList {
    #[serde(with = "serde_ext::vec_point")]
    pub points: Vec<Point>,
    /// Rundreise-Daten je Punkt (gleiche Länge wie `points` oder leer).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub point_extra: Vec<Extra>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

/// Custom Data eines Elements (`DTAv`, JS `customData`). Gleicher Aufbau wie ein Wörterbuch.
pub type CustomData = DataDict;

impl DataEntry {
    /// Neuer Skalar-Eintrag.
    pub fn scalar(key: impl Into<String>, value: impl Into<String>) -> Self {
        DataEntry {
            tag: EntryTag::Dce,
            key: key.into(),
            value: DataValue::Scalar(value.into()),
            text: None,
            extra: Extra::default(),
        }
    }

    /// Wert als Text: Skalar oder (bei Strukturen) die mitgespeicherte Textform.
    pub fn text_value(&self) -> Option<&str> {
        match &self.value {
            DataValue::Scalar(s) => Some(s),
            _ => self.text.as_deref(),
        }
    }
}

impl DataDict {
    /// Erster Eintrag mit dem Schlüssel.
    pub fn get(&self, key: &str) -> Option<&DataEntry> {
        self.entries.iter().find(|e| e.key == key)
    }

    /// Veränderlicher Zugriff auf den ersten Eintrag mit dem Schlüssel.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut DataEntry> {
        self.entries.iter_mut().find(|e| e.key == key)
    }

    /// Skalarwert als Text (bei Strukturen die Textform).
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(|e| e.text_value())
    }

    /// Setzt einen Skalar (Text). Vorhandene Einträge werden ersetzt, neue angehängt.
    pub fn set_scalar(&mut self, key: &str, value: impl Into<String>) {
        let v = DataValue::Scalar(value.into());
        match self.get_mut(key) {
            Some(e) => {
                e.value = v;
                e.text = None;
            }
            None => self.entries.push(DataEntry {
                tag: EntryTag::Dce,
                key: key.to_string(),
                value: v,
                text: None,
                extra: Extra::default(),
            }),
        }
    }

    /// Setzt eine Zahl so, wie das Original sie speichert (`%.17g`, 03 §10.12).
    pub fn set_number(&mut self, key: &str, v: f64) {
        self.set_scalar(key, format_g17(v));
    }

    /// Setzt einen Wahrheitswert (`"true"`/`"false"`).
    pub fn set_bool(&mut self, key: &str, v: bool) {
        self.set_scalar(key, if v { "true" } else { "false" });
    }

    /// Setzt eine Struktur (Liste/Wörterbuch) und schreibt wie das Original die Textform dazu.
    pub fn set_structured(&mut self, key: &str, value: DataValue) {
        let text = Some(value.to_source());
        match self.get_mut(key) {
            Some(e) => {
                e.value = value;
                e.text = text;
            }
            None => self.entries.push(DataEntry {
                tag: EntryTag::Dce,
                key: key.to_string(),
                value,
                text,
                extra: Extra::default(),
            }),
        }
    }

    /// Entfernt alle Einträge mit dem Schlüssel; liefert, ob einer entfernt wurde.
    pub fn remove(&mut self, key: &str) -> bool {
        let n = self.entries.len();
        self.entries.retain(|e| e.key != key);
        n != self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl DataValue {
    /// Typisierung eines Skalars nach der Original-Regel (03 §10.12, A, 38/38 Laufzeitfälle):
    /// `"true"`/`"false"` → boolean; Text mit Punkt → Zahl, wenn `%.17g` den Text exakt wiedergibt;
    /// Text ohne Punkt → Zahl, wenn er die kanonische Dezimaldarstellung einer ganzen Zahl ist;
    /// sonst String. Nicht-Skalare liefern `None`.
    pub fn typed(&self) -> Option<TypedScalar> {
        match self {
            DataValue::Scalar(s) => Some(type_scalar(s)),
            _ => None,
        }
    }

    /// Textform wie JS `toSource` im Original: `[ 1, "zwei", { x:3 } ]` (03 §10.8).
    pub fn to_source(&self) -> String {
        match self {
            DataValue::None => "null".to_string(),
            DataValue::Scalar(s) => match type_scalar(s) {
                TypedScalar::Bool(b) => b.to_string(),
                TypedScalar::Number(_) => s.clone(),
                TypedScalar::String(s) => quote_js(&s),
            },
            DataValue::List(l) => {
                if l.items.is_empty() {
                    "[]".to_string()
                } else {
                    let parts: Vec<String> = l.items.iter().map(|v| v.to_source()).collect();
                    format!("[ {} ]", parts.join(", "))
                }
            }
            DataValue::Dict(d) => {
                if d.entries.is_empty() {
                    "{}".to_string()
                } else {
                    let parts: Vec<String> = d
                        .entries
                        .iter()
                        .map(|e| format!("{}:{}", e.key, e.value.to_source()))
                        .collect();
                    format!("{{ {} }}", parts.join(", "))
                }
            }
            DataValue::Points(p) => {
                let parts: Vec<String> = p
                    .points
                    .iter()
                    .map(|q| {
                        format!(
                            "{{ x:{}, y:{} }}",
                            format_g17(q.x as f64),
                            format_g17(q.y as f64)
                        )
                    })
                    .collect();
                format!("[ {} ]", parts.join(", "))
            }
        }
    }
}

fn quote_js(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// Skalar-Typisierung (siehe [`DataValue::typed`]).
pub fn type_scalar(s: &str) -> TypedScalar {
    if s == "true" {
        return TypedScalar::Bool(true);
    }
    if s == "false" {
        return TypedScalar::Bool(false);
    }
    let numeric_chars = !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_digit() || b == b'.' || b == b'-');
    if numeric_chars {
        if s.contains('.') {
            if let Ok(v) = s.parse::<f64>()
                && v.is_finite()
                && format_g17(v) == s
            {
                return TypedScalar::Number(v);
            }
        } else if let Ok(v) = s.parse::<i128>()
            && v.to_string() == s
        {
            return TypedScalar::Number(v as f64);
        }
    }
    TypedScalar::String(s.to_string())
}

/// Formatiert wie C `printf("%.17g", v)`: 17 signifikante Stellen, kürzeste Form ohne Nullen am Ende,
/// Exponentialform bei Exponent < −4 oder ≥ 17.
pub fn format_g17(v: f64) -> String {
    if v.is_nan() {
        return "nan".to_string();
    }
    if v.is_infinite() {
        return if v > 0.0 {
            "inf".to_string()
        } else {
            "-inf".to_string()
        };
    }
    if v == 0.0 {
        return if v.is_sign_negative() {
            "-0".to_string()
        } else {
            "0".to_string()
        };
    }
    // Wissenschaftliche Form mit 17 Stellen liefert Mantisse und Exponent (korrekt gerundet).
    let sci = format!("{:.16e}", v);
    let (mant, exp) = match sci.split_once('e') {
        Some(p) => p,
        None => return sci,
    };
    let exp: i32 = exp.parse().unwrap_or(0);
    let neg = mant.starts_with('-');
    let digits: String = mant.chars().filter(|c| c.is_ascii_digit()).collect();
    let sign = if neg { "-" } else { "" };
    if !(-4..17).contains(&exp) {
        let mut d = digits.trim_end_matches('0').to_string();
        if d.is_empty() {
            d.push('0');
        }
        let m = if d.len() > 1 {
            format!("{}.{}", &d[..1], &d[1..])
        } else {
            d
        };
        let es = if exp < 0 { '-' } else { '+' };
        return format!("{sign}{m}e{es}{:02}", exp.abs());
    }
    // Feste Form: 17 signifikante Stellen, Dezimalpunkt nach (exp+1) Stellen.
    let point = exp + 1;
    let s = if point <= 0 {
        format!("0.{}{}", "0".repeat((-point) as usize), digits)
    } else if (point as usize) >= digits.len() {
        format!("{}{}", digits, "0".repeat(point as usize - digits.len()))
    } else {
        format!(
            "{}.{}",
            &digits[..point as usize],
            &digits[point as usize..]
        )
    };
    let s = if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    };
    format!("{sign}{s}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn g17_matches_c() {
        assert_eq!(format_g17(0.1), "0.10000000000000001");
        assert_eq!(format_g17(12.0), "12");
        assert_eq!(format_g17(1.5), "1.5");
        assert_eq!(format_g17(-3.0), "-3");
        assert_eq!(format_g17(100.25), "100.25");
        assert_eq!(format_g17(123456789012.0), "123456789012");
        assert_eq!(format_g17(1e20), "1e+20");
        assert_eq!(format_g17(0.0001), "0.0001");
        assert_eq!(format_g17(0.00001), "1.0000000000000001e-05");
        assert_eq!(format_g17(11.25140380859375), "11.25140380859375");
    }

    #[test]
    fn scalar_typing_rules() {
        // 03 §10.12: Grenzfälle aus dem Laufzeittest.
        for s in [
            "[1,2]", "0x10", "1e3", " 7 ", "007", "00", "-007", "+1", "-0", ".5", "5.", "1.0",
            "1.50", "0.0", "Infinity", "NaN", "null", "{a:1}", "", "TRUE", "True", " true",
        ] {
            assert!(
                matches!(type_scalar(s), TypedScalar::String(_)),
                "{s:?} sollte String bleiben"
            );
        }
        for (s, v) in [
            ("12", 12.0),
            ("1.5", 1.5),
            ("-3", -3.0),
            ("0", 0.0),
            ("10", 10.0),
            ("0.5", 0.5),
            ("100.25", 100.25),
            ("123456789012", 123456789012.0),
            ("2147483648", 2147483648.0),
            ("-2147483649", -2147483649.0),
            ("0.10000000000000001", 0.1),
        ] {
            assert_eq!(type_scalar(s), TypedScalar::Number(v), "{s:?}");
        }
        assert_eq!(type_scalar("true"), TypedScalar::Bool(true));
        assert_eq!(type_scalar("false"), TypedScalar::Bool(false));
    }

    #[test]
    fn to_source_like_original() {
        let list = DataValue::List(DataList {
            items: vec![
                DataValue::Scalar("1".into()),
                DataValue::Scalar("zwei".into()),
                DataValue::Dict(DataDict {
                    entries: vec![DataEntry::scalar("x", "3")],
                    extra: Extra::default(),
                }),
            ],
            extra: Extra::default(),
        });
        assert_eq!(list.to_source(), "[ 1, \"zwei\", { x:3 } ]");
        let mut d = CustomData::default();
        d.set_number("n", 0.1);
        d.set_bool("b", false);
        assert_eq!(d.get_str("n"), Some("0.10000000000000001"));
        assert_eq!(d.get_str("b"), Some("false"));
        d.set_structured("arr", list);
        assert_eq!(
            d.get("arr").and_then(|e| e.text.clone()).as_deref(),
            Some("[ 1, \"zwei\", { x:3 } ]")
        );
        assert!(d.remove("n"));
    }
}

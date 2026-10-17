//! Rohknoten des Original-Tag-Baums und Rundreise-Daten.
//!
//! Das Dateiformat (`mkTS`) ist ein Baum aus Knoten `TAG TYP { … }` (Spezifikation 02 §3). Das Modell
//! bildet die bekannten Tags auf typisierte Felder ab. Alles, was nicht abgebildet ist, bleibt als
//! [`RawNode`] erhalten. Zusätzlich merkt sich jeder Container in [`Extra`] die Reihenfolge seiner
//! Kinder beim Lesen, damit ein Schreiber die Datei unverändert wieder erzeugen kann.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::sync::Arc;

/// Schlüssel eines Knotens: drei Zeichen Tag plus Typbuchstabe, z. B. `OPAi` (02 §3.1).
///
/// Derselbe 3-Buchstaben-Tag tritt nie mit zwei Typen auf; der Schlüssel identifiziert also das Feld.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct TagKey(pub [u8; 4]);

impl TagKey {
    /// Schlüssel aus vier Bytes, z. B. `TagKey::new(b"OPAi")`.
    pub const fn new(b: &[u8; 4]) -> Self {
        TagKey(*b)
    }

    /// Schlüssel aus Text; `None`, wenn der Text nicht genau 4 ASCII-Zeichen hat.
    pub fn parse(s: &str) -> Option<Self> {
        let b = s.as_bytes();
        if b.len() == 4 && b.iter().all(|c| c.is_ascii_graphic()) {
            Some(TagKey([b[0], b[1], b[2], b[3]]))
        } else {
            None
        }
    }

    /// Die drei Tag-Zeichen.
    pub fn tag(&self) -> [u8; 3] {
        [self.0[0], self.0[1], self.0[2]]
    }

    /// Typbuchstabe (`v`, `i`, `f`, `b`, `s`, `y`).
    pub fn kind(&self) -> u8 {
        self.0[3]
    }

    /// Schlüssel als Text (nicht druckbare Bytes werden zu `?`).
    pub fn as_string(&self) -> String {
        self.0
            .iter()
            .map(|&c| if c.is_ascii_graphic() { c as char } else { '?' })
            .collect()
    }
}

impl fmt::Debug for TagKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.as_string())
    }
}

impl fmt::Display for TagKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.as_string())
    }
}

impl Serialize for TagKey {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.as_string())
    }
}

impl<'de> Deserialize<'de> for TagKey {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        TagKey::parse(&s)
            .ok_or_else(|| serde::de::Error::custom(format!("ungültiger Tag-Schlüssel {s:?}")))
    }
}

/// Wert eines Rohknotens. Der Typ passt immer zum Typbuchstaben des Schlüssels.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RawValue {
    /// `v`: geordnete Liste von Kindknoten.
    List(Vec<RawNode>),
    /// `i`: 32-Bit-Wert (vorzeichenbehaftete Werte als Zweierkomplement, Farben als ARGB).
    Int(u32),
    /// `f`: Dezimalzahl. Geschrieben wie C-`%f` mit abgeschnittenen Nullen (02 §3.1).
    Num(f64),
    /// `b`: Wahrheitswert.
    Bool(bool),
    /// `s`: Zeichenkette (UTF-16 in der Datei).
    Str(String),
    /// `s` mit ungültigem UTF-16 (z. B. einzelne Surrogate); bleibt unverändert erhalten.
    Utf16(Vec<u16>),
    /// `y`: Binärblock (nur `PTRy`).
    Bytes(Vec<u8>),
}

/// Ein Knoten des Tag-Baums.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RawNode {
    pub key: TagKey,
    pub value: RawValue,
}

impl RawNode {
    pub fn new(key: TagKey, value: RawValue) -> Self {
        RawNode { key, value }
    }
    pub fn list(key: TagKey, children: Vec<RawNode>) -> Self {
        RawNode {
            key,
            value: RawValue::List(children),
        }
    }
    pub fn int(key: TagKey, v: u32) -> Self {
        RawNode {
            key,
            value: RawValue::Int(v),
        }
    }
    pub fn num(key: TagKey, v: f64) -> Self {
        RawNode {
            key,
            value: RawValue::Num(v),
        }
    }
    pub fn bool(key: TagKey, v: bool) -> Self {
        RawNode {
            key,
            value: RawValue::Bool(v),
        }
    }
    pub fn str(key: TagKey, v: impl Into<String>) -> Self {
        RawNode {
            key,
            value: RawValue::Str(v.into()),
        }
    }

    /// Kinder bei Listenknoten, sonst leer.
    pub fn children(&self) -> &[RawNode] {
        match &self.value {
            RawValue::List(v) => v,
            _ => &[],
        }
    }

    /// Erstes Kind mit dem Schlüssel.
    pub fn child(&self, key: TagKey) -> Option<&RawNode> {
        self.children().iter().find(|c| c.key == key)
    }

    pub fn as_int(&self) -> Option<u32> {
        match self.value {
            RawValue::Int(v) => Some(v),
            _ => None,
        }
    }
    pub fn as_num(&self) -> Option<f64> {
        match self.value {
            RawValue::Num(v) => Some(v),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self.value {
            RawValue::Bool(v) => Some(v),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match &self.value {
            RawValue::Str(v) => Some(v),
            _ => None,
        }
    }

    /// Tiefensuche über alle Knoten (Vorordnung), ohne Rekursion.
    pub fn walk(&self, mut f: impl FnMut(&RawNode)) {
        let mut stack = vec![self];
        while let Some(n) = stack.pop() {
            f(n);
            for c in n.children().iter().rev() {
                stack.push(c);
            }
        }
    }
}

/// Rundreise-Daten eines Containers.
///
/// * `nodes`: nicht abgebildete Kinder in Dateireihenfolge.
/// * `order`: Schlüssel **aller** Kinder in Dateireihenfolge, wie gelesen. `None` heißt „neu erzeugt“:
///   Ein Schreiber benutzt dann die Original-Reihenfolge und schreibt die Felder, die das Original immer
///   schreibt. `Some` (auch leer) heißt „aus einer Datei gelesen“: Felder mit Vorgabewert werden nur
///   geschrieben, wenn sie in `order` vorkamen.
/// * `shadow`: Originalknoten, deren Wert das Modell nur verlustbehaftet halten kann (z. B. ein Float,
///   der als `f32` anders formatiert würde). Solange der Modellwert gleich bleibt, schreibt der
///   Schreiber das Original zurück.
///
/// Wer ein Objekt inhaltlich neu aufbaut, kann `order` mit [`Extra::reset_layout`] verwerfen.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Extra {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<RawNode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<Arc<[TagKey]>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shadow: Vec<RawNode>,
}

impl Extra {
    /// Leere Rundreise-Daten für ein neu erzeugtes Objekt.
    pub fn new() -> Self {
        Extra::default()
    }

    /// Wahr, wenn keine Rohknoten, keine Reihenfolge und keine Schatten vorhanden sind.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty() && self.order.is_none() && self.shadow.is_empty()
    }

    /// Wahr, wenn das Objekt aus einer Datei gelesen wurde.
    pub fn is_from_file(&self) -> bool {
        self.order.is_some()
    }

    /// Verwirft Reihenfolge und Schatten (unbekannte Knoten bleiben). Danach schreibt ein Schreiber das
    /// Objekt wie ein neu erzeugtes.
    pub fn reset_layout(&mut self) {
        self.order = None;
        self.shadow.clear();
    }

    /// Kommt der Schlüssel unter den unbekannten Knoten vor?
    pub fn has_raw(&self, key: TagKey) -> bool {
        self.nodes.iter().any(|n| n.key == key)
    }
}

/// Schreibt einen Schlüssel als Konstante: `tag!(b"OPAi")`.
#[macro_export]
macro_rules! tag {
    ($b:literal) => {
        $crate::raw::TagKey::new($b)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tagkey_roundtrip() {
        let k = TagKey::new(b"OPAi");
        assert_eq!(k.as_string(), "OPAi");
        assert_eq!(k.kind(), b'i');
        assert_eq!(TagKey::parse("OPAi"), Some(k));
        assert_eq!(TagKey::parse("OPA"), None);
        let j = serde_json::to_string(&k).unwrap();
        assert_eq!(j, "\"OPAi\"");
        let back: TagKey = serde_json::from_str(&j).unwrap();
        assert_eq!(back, k);
    }

    #[test]
    fn node_access_and_walk() {
        let n = RawNode::list(
            TagKey::new(b"MTXv"),
            vec![
                RawNode::num(TagKey::new(b"M00f"), 1.0),
                RawNode::int(TagKey::new(b"OPAi"), 5),
            ],
        );
        assert_eq!(
            n.child(TagKey::new(b"OPAi")).and_then(|c| c.as_int()),
            Some(5)
        );
        let mut count = 0;
        n.walk(|_| count += 1);
        assert_eq!(count, 3);
    }
}

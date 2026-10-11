//! Identitäten: Laufzeit-Handles und persistente IDs aus dem Dateiformat.
//!
//! Das Format hat keine Element-IDs (04 §2.3). Für Auswahl, Undo, JS-Wrapper und Bedienfelder vergibt
//! das Modell stabile `u64`-Handles ([`ElementId`], [`LayerId`]), die nicht gespeichert werden.
//! Persistente IDs gibt es nur, wo das Format sie hat: Seiten-GUID (`PIDs`), geteilte Ebenen (`UIDs`),
//! Symbol-ID (`MSNi`), Stilname (`ZSNs`).

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

/// Laufzeit-ID eines Elements. Bleibt bei Copy-on-Write gleich, wird beim Duplizieren neu vergeben.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct ElementId(pub u64);

/// Laufzeit-ID einer Ebene oder Unterebene.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct LayerId(pub u64);

/// Vergibt Laufzeit-IDs, eindeutig je Dokument (Element- und Ebenen-IDs teilen einen Zähler).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdGen {
    next: u64,
}

impl IdGen {
    /// Neuer Zähler; die erste vergebene ID ist 1.
    pub fn new() -> Self {
        IdGen { next: 0 }
    }
    /// Setzt den Zähler so, dass alle IDs bis einschließlich `max` belegt gelten.
    pub fn reserve_up_to(&mut self, max: u64) {
        if self.next < max {
            self.next = max;
        }
    }
    pub fn element(&mut self) -> ElementId {
        self.next += 1;
        ElementId(self.next)
    }
    pub fn layer(&mut self) -> LayerId {
        self.next += 1;
        LayerId(self.next)
    }
    /// Zuletzt vergebener Wert.
    pub fn current(&self) -> u64 {
        self.next
    }
}

/// GUID im Textformat der Datei, z. B. `5a09c8d5-2f75-49c8-a13f-0ca3f7bed8a0` (`PIDs`, `UIDs`).
///
/// Der Text wird unverändert gehalten (Groß-/Kleinschreibung, Klammern), damit das Zurückschreiben
/// exakt bleibt. Die Null-GUID bedeutet „kein Verweis“ (beobachtet bei `MPLv/PIDs`).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct Guid(pub String);

impl Guid {
    /// Null-GUID wie das Original sie für „nicht verknüpft“ schreibt.
    pub fn nil() -> Guid {
        Guid("00000000-0000-0000-0000-000000000000".to_string())
    }

    /// Wahr, wenn alle Hex-Ziffern 0 sind (oder der Text leer ist).
    pub fn is_nil(&self) -> bool {
        self.0.chars().filter(|c| c.is_ascii_hexdigit()).all(|c| c == '0')
    }

    /// Vergleich ohne Rücksicht auf Groß-/Kleinschreibung und Klammern.
    pub fn same_as(&self, other: &Guid) -> bool {
        let norm = |s: &str| -> String {
            s.chars().filter(|c| c.is_ascii_hexdigit()).map(|c| c.to_ascii_lowercase()).collect()
        };
        norm(&self.0) == norm(&other.0)
    }

    /// Neue zufällige GUID (Version 4, Kleinschreibung wie in CS6-Dateien).
    ///
    /// Zufallsquelle: Systemzeit, Prozess-Zähler und Adresse eines Stack-Werts, gemischt mit SplitMix64.
    /// Für Eindeutigkeit innerhalb und zwischen Dokumenten genügt das; kryptographisch ist es nicht.
    pub fn new_random() -> Guid {
        static COUNTER: AtomicU64 = AtomicU64::new(0x9e37_79b9_7f4a_7c15);
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        let local = 0u8;
        let addr = &local as *const u8 as usize as u64;
        let c = COUNTER.fetch_add(0x9e37_79b9_7f4a_7c15, Ordering::Relaxed);
        let mut state = t ^ addr.rotate_left(17) ^ c;
        let mut next = || {
            state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        };
        let a = next();
        let b = next();
        let mut bytes = [0u8; 16];
        bytes[..8].copy_from_slice(&a.to_be_bytes());
        bytes[8..].copy_from_slice(&b.to_be_bytes());
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        let h: Vec<String> = bytes.iter().map(|b| format!("{b:02x}")).collect();
        Guid(format!(
            "{}-{}-{}-{}-{}",
            h[0..4].concat(),
            h[4..6].concat(),
            h[6..8].concat(),
            h[8..10].concat(),
            h[10..16].concat()
        ))
    }
}

/// Symbol-ID aus `MSTv/MSNi` bzw. `ALSv/MSNi`. Im Korpus nicht immer eindeutig (13 Dateien);
/// bei doppelter ID gilt die erste Definition (04 §1.10, 13 §7.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct SymbolId(pub u32);

/// Index in [`crate::Document::symbols`] (intern eindeutig, auch wenn die `SymbolId` doppelt vorkommt).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SymbolKey(pub usize);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guid_nil_and_compare() {
        assert!(Guid::nil().is_nil());
        assert!(!Guid("5a09c8d5-2f75-49c8-a13f-0ca3f7bed8a0".into()).is_nil());
        assert!(Guid("{ABCDEF00-0000-0000-0000-000000000001}".into())
            .same_as(&Guid("abcdef00-0000-0000-0000-000000000001".into())));
        let a = Guid::new_random();
        let b = Guid::new_random();
        assert_ne!(a, b);
        assert_eq!(a.0.len(), 36);
        assert_eq!(&a.0[14..15], "4");
    }

    #[test]
    fn idgen_counts() {
        let mut g = IdGen::new();
        assert_eq!(g.element(), ElementId(1));
        assert_eq!(g.layer(), LayerId(2));
        g.reserve_up_to(10);
        assert_eq!(g.element(), ElementId(11));
    }
}

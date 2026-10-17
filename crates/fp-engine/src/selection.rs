//! Auswahl (Elemente, Unterauswahl von Pfadpunkten, Pixelauswahl) und internes Zwischenablage-Modell.
//!
//! Elemente werden über ihre Laufzeit-ID ([`ElementId`]) gehalten; die Reihenfolge ist die
//! Auswahlreihenfolge. Die Pixelauswahl („Auswahlrahmen“) ist eine 8-Bit-Maske in
//! Dokumentkoordinaten ([`fp_raster::Mask`]). Das Systemformat der Zwischenablage ergänzt später die UI.

use fp_model::element::CommonAttrs;
use fp_model::{Element, ElementId, PathAttrs};
use fp_raster::{Mask, RgbaImage};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Verweis auf einen Pfadpunkt (Unterauswahl mit dem Teilauswahl-Werkzeug).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PointRef {
    pub element: ElementId,
    pub contour: usize,
    pub node: usize,
}

/// Art einer Auswahländerung.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SelectMode {
    /// Auswahl ersetzen.
    #[default]
    Replace,
    /// Hinzufügen (Umschalt bei Pixelauswahl).
    Add,
    /// Abziehen (Alt bei Pixelauswahl).
    Subtract,
    /// Schnittmenge (Umschalt+Alt).
    Intersect,
    /// Umschalten (Umschalt-Klick auf Objekte).
    Toggle,
}

impl SelectMode {
    /// Aus Namen (`replace`, `add`, `subtract`, `intersect`, `toggle`).
    pub fn from_name(s: &str) -> Option<SelectMode> {
        match s {
            "replace" => Some(SelectMode::Replace),
            "add" => Some(SelectMode::Add),
            "subtract" => Some(SelectMode::Subtract),
            "intersect" => Some(SelectMode::Intersect),
            "toggle" => Some(SelectMode::Toggle),
            _ => None,
        }
    }

    /// Name für Befehlsparameter.
    pub fn name(self) -> &'static str {
        match self {
            SelectMode::Replace => "replace",
            SelectMode::Add => "add",
            SelectMode::Subtract => "subtract",
            SelectMode::Intersect => "intersect",
            SelectMode::Toggle => "toggle",
        }
    }
}

/// Auswahl eines Dokuments.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Selection {
    /// Ausgewählte Elemente (Auswahlreihenfolge).
    pub elements: Vec<ElementId>,
    /// Unterausgewählte Pfadpunkte.
    pub points: Vec<PointRef>,
    /// Pixelauswahl (Auswahlrahmen), `None` = keine.
    pub pixels: Option<Mask>,
}

impl Selection {
    /// Keine Elemente und keine Pixelauswahl?
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty() && !self.has_pixels()
    }

    /// Gibt es eine nicht leere Pixelauswahl?
    pub fn has_pixels(&self) -> bool {
        self.pixels
            .as_ref()
            .is_some_and(|m| !m.tight_bounds().is_empty())
    }

    /// Ist das Element ausgewählt?
    pub fn contains(&self, id: ElementId) -> bool {
        self.elements.contains(&id)
    }

    /// Ersetzt die Elementauswahl (Punkte werden verworfen).
    pub fn set(&mut self, ids: Vec<ElementId>) {
        let mut v = Vec::with_capacity(ids.len());
        for id in ids {
            if !v.contains(&id) {
                v.push(id);
            }
        }
        self.elements = v;
        self.points.clear();
    }

    /// Wendet eine Elementauswahl mit Modus an.
    pub fn apply(&mut self, ids: &[ElementId], mode: SelectMode) {
        match mode {
            SelectMode::Replace => self.set(ids.to_vec()),
            SelectMode::Add => {
                for id in ids {
                    if !self.elements.contains(id) {
                        self.elements.push(*id);
                    }
                }
            }
            SelectMode::Subtract => self.elements.retain(|e| !ids.contains(e)),
            SelectMode::Intersect => self.elements.retain(|e| ids.contains(e)),
            SelectMode::Toggle => {
                for id in ids {
                    if let Some(i) = self.elements.iter().position(|e| e == id) {
                        self.elements.remove(i);
                    } else {
                        self.elements.push(*id);
                    }
                }
            }
        }
        let keep = self.elements.clone();
        self.points.retain(|p| keep.contains(&p.element));
    }

    /// Hebt die Elementauswahl und die Pixelauswahl auf.
    pub fn clear(&mut self) {
        self.elements.clear();
        self.points.clear();
        self.pixels = None;
    }

    /// Kombiniert eine neue Pixelmaske mit der bestehenden.
    pub fn apply_pixels(&mut self, mask: Mask, mode: SelectMode) {
        let new = match (&self.pixels, mode) {
            (_, SelectMode::Replace) | (None, SelectMode::Add) | (None, SelectMode::Toggle) => {
                Some(mask)
            }
            (None, SelectMode::Subtract) | (None, SelectMode::Intersect) => None,
            (Some(old), SelectMode::Add) | (Some(old), SelectMode::Toggle) => old.union(&mask).ok(),
            (Some(old), SelectMode::Subtract) => old.subtract(&mask).ok(),
            (Some(old), SelectMode::Intersect) => old.intersection(&mask).ok(),
        };
        self.pixels = new.filter(|m| !m.tight_bounds().is_empty());
    }

    /// Entfernt IDs, die es im Dokument nicht mehr gibt.
    pub fn retain_existing(&mut self, exists: impl Fn(ElementId) -> bool) {
        self.elements.retain(|id| exists(*id));
        self.points.retain(|p| exists(p.element));
    }
}

/// Attribute für „Attribute einfügen“.
#[derive(Clone, Debug, PartialEq)]
pub struct ClipAttributes {
    pub common: CommonAttrs,
    pub path: Option<Arc<PathAttrs>>,
}

/// Inhalt der internen Zwischenablage.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Clipboard {
    /// Kopierte Elemente (oben → unten), mit ihren Laufzeit-IDs; beim Einfügen werden neue IDs vergeben.
    pub elements: Vec<Arc<Element>>,
    /// Attribute des obersten kopierten Elements.
    pub attributes: Option<ClipAttributes>,
    /// Kopierte Pixel (Auswahlrahmen auf einer Bitmap), in Dokumentkoordinaten.
    pub pixels: Option<RgbaImage>,
    /// Auflösung des Quelldokuments (für die Warnung 12327 beim Einfügen).
    pub source_resolution: Option<f32>,
    /// Kopierter Text (z. B. „HTML-Code kopieren“); die UI überträgt ihn in die Systemzwischenablage.
    pub text: Option<String>,
}

impl Clipboard {
    /// Leer (keine Objekte/Pixel zum Einfügen)?
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty() && self.pixels.is_none()
    }

    /// Leeren.
    pub fn clear(&mut self) {
        *self = Clipboard::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fp_geom::IRect;

    #[test]
    fn modes() {
        let mut s = Selection::default();
        let (a, b, c) = (ElementId(1), ElementId(2), ElementId(3));
        s.apply(&[a, b], SelectMode::Replace);
        s.apply(&[b, c], SelectMode::Toggle);
        assert_eq!(s.elements, vec![a, c]);
        s.apply(&[a], SelectMode::Subtract);
        assert_eq!(s.elements, vec![c]);
        let m = Mask::from_rect(IRect::new(0, 0, 10, 10)).unwrap_or_default();
        s.apply_pixels(m.clone(), SelectMode::Replace);
        assert!(s.has_pixels());
        s.apply_pixels(m, SelectMode::Subtract);
        assert!(!s.has_pixels());
    }
}

/// Auswahlverlauf eines Dokuments für „Vorherige/Nächste Auswahl“ (Strg+Umschalt+Links/Rechts,
/// Q-21 aus M2): Jede neue, nicht leere Objektauswahl ist ein Eintrag. Zurück/Vor laufen nicht im
/// Kreis; wird nach einem Schritt zurück neu ausgewählt, fallen die späteren Einträge weg.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SelectionHistory {
    entries: Vec<Vec<ElementId>>,
    pos: usize,
}

impl SelectionHistory {
    /// Höchstzahl der gemerkten Auswahlen.
    pub const MAX: usize = 100;

    /// Merkt die aktuelle Auswahl (leere Auswahl und unveränderte Auswahl ergeben keinen Eintrag).
    pub fn note(&mut self, sel: &[ElementId]) {
        if sel.is_empty()
            || self
                .entries
                .get(self.pos)
                .is_some_and(|e| e.as_slice() == sel)
        {
            return;
        }
        if !self.entries.is_empty() {
            self.entries.truncate(self.pos + 1);
        }
        self.entries.push(sel.to_vec());
        if self.entries.len() > Self::MAX {
            self.entries.remove(0);
        }
        self.pos = self.entries.len() - 1;
    }

    /// Einen Eintrag zurück (`dir < 0`) bzw. vor (`dir > 0`) gehen. `current` ist die aktuelle
    /// Auswahl, `exists` filtert gelöschte Objekte heraus (leer gewordene Einträge werden
    /// übersprungen). `None`: Ende des Verlaufs, die Auswahl bleibt.
    pub fn step(
        &mut self,
        dir: i64,
        current: &[ElementId],
        exists: impl Fn(ElementId) -> bool,
    ) -> Option<Vec<ElementId>> {
        if self.entries.is_empty() {
            return None;
        }
        // weicht die Auswahl vom aktuellen Eintrag ab (z. B. aufgehoben), führt „zurück“ erst zu ihm
        let mut i = self.pos as i64;
        let at_entry = self
            .entries
            .get(self.pos)
            .is_some_and(|e| e.as_slice() == current);
        if dir > 0 || at_entry {
            i += dir.signum();
        }
        while i >= 0 && (i as usize) < self.entries.len() {
            let ids: Vec<ElementId> = self.entries[i as usize]
                .iter()
                .copied()
                .filter(|id| exists(*id))
                .collect();
            if !ids.is_empty() && ids.as_slice() != current {
                self.pos = i as usize;
                return Some(ids);
            }
            i += dir.signum();
        }
        None
    }

    /// Anzahl der Einträge.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Kein Eintrag?
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

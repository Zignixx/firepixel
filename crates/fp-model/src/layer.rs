//! Ebenen, Cels (Inhalt einer Ebene in einem Status) und Ebenenlisten (Spezifikationen 02 §4.2, 04 §1.6a).
//!
//! Ebenen stehen in `LYLv` **von unten nach oben** (die letzte ist die oberste). Ein Eintrag der Liste
//! ist eine eigene Ebene (`LAYv`), ein Verweis auf eine über Seiten geteilte Ebene (`LSAv`) oder der
//! Platzhalter der Masterseiten-Ebenen (`MPLv`).

use crate::element::Element;
use crate::ids::{Guid, IdGen, LayerId};
use crate::raw::{Extra, RawNode};
use crate::serde_ext;
use fp_color::{BlendMode, Opacity};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Art einer Ebene (`URLb`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayerKind {
    Normal,
    /// Web-Ebene mit Hotspots/Slices, immer über Status geteilt.
    Web,
}

/// Ebene bzw. Unterebene (`LAYv`/`LSLv`, im Original `CDbLayer`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub id: LayerId,
    /// Name (`LNMs`; lokalisiert, nie darauf verlassen).
    pub name: String,
    pub kind: LayerKind,
    /// `BKGb`: CS6 schreibt immer 0; beim Lesen erzwingt 1 eine geteilte Ebene (Altlast).
    pub background: bool,
    /// Verschachtelungstiefe (`LLLi`): 0 Ebene, 1 Unterebene, 2 Unter-Unterebene.
    pub level: i32,
    /// `MSLb`: Altflag, das Original schreibt es gleich `URLb`.
    pub legacy_shared: bool,
    /// Über Status geteilt (JS `sharing == "shared"`): `MCLb = 0`; fehlt `MCLb`, gilt `!MSLb`;
    /// `BKGb = 1` erzwingt geteilt (02 §4.2). Bei geteilten Ebenen gilt nur der erste Cel.
    pub shared_across_frames: bool,
    /// Aufgeklappt im Ebenen-Bedienfeld (`DISb`).
    pub expanded: bool,
    /// Sichtbarkeit je Status (`VIFv{VISb…}`); meist ein Eintrag.
    pub visibility: Vec<bool>,
    /// Ist `VIFv` vorhanden?
    pub has_visibility_node: bool,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub visibility_extra: Extra,
    /// Deckkraft (`OPAi`, im Korpus nie gesetzt).
    #[serde(with = "serde_ext::opacity", default)]
    pub opacity: Opacity,
    /// Mischmodus (`BLDi`, im Korpus nie gesetzt).
    #[serde(with = "serde_ext::blend", default)]
    pub blend: BlendMode,
    /// Ein Cel pro Status bzw. bei geteilten Ebenen Cel 0 (plus leere Hüllen).
    pub cels: Vec<Arc<Cel>>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub cels_extra: Extra,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

/// Inhalt einer Ebene in einem Status (`CELv`, JS `FrameNLayerIntersection`).
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Cel {
    /// Gesperrt (`LCKb`).
    pub locked: bool,
    /// Sichtbar (`VISb`, selten; fehlt = sichtbar).
    pub visible: bool,
    /// Elemente oben → unten (`ELMv`).
    pub elements: Vec<Arc<Element>>,
    /// Rundreise-Daten von `ELMv`.
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub elements_extra: Extra,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

impl Cel {
    /// Leerer, sichtbarer, nicht gesperrter Cel.
    pub fn empty() -> Cel {
        Cel { locked: false, visible: true, elements: Vec::new(), elements_extra: Extra::default(), extra: Extra::default() }
    }
}

/// Laufzeit-Zustand eines Elements (im Original CDbElement+0x08, nicht gespeichert, ergibt sich aus dem Ort).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ElementState {
    #[default]
    Normal,
    /// Inhalt der Masterseite auf einer anderen Seite (nicht treffbar, gilt als gesperrt).
    MasterPageContent,
    /// Inhalt einer über Seiten geteilten Ebene.
    SharedLayer,
}

impl Layer {
    /// Neue leere Ebene mit `frames` Cels (Original-Vorgaben: sichtbar, aufgeklappt, nicht geteilt).
    pub fn new(ids: &mut IdGen, name: &str, kind: LayerKind, frames: usize) -> Layer {
        let shared = kind == LayerKind::Web;
        let n = frames.max(1);
        Layer {
            id: ids.layer(),
            name: name.to_string(),
            kind,
            background: false,
            level: 0,
            legacy_shared: kind == LayerKind::Web,
            shared_across_frames: shared,
            expanded: true,
            visibility: vec![true],
            has_visibility_node: true,
            visibility_extra: Extra::default(),
            opacity: Opacity(255),
            blend: BlendMode(0),
            cels: (0..n).map(|_| Arc::new(Cel::empty())).collect(),
            cels_extra: Extra::default(),
            extra: Extra::default(),
        }
    }

    /// Ebene sichtbar (erster Eintrag von `VIFv`; fehlt er, sichtbar).
    pub fn is_visible(&self) -> bool {
        self.visibility.first().copied().unwrap_or(true)
    }

    /// Cel für einen Status nach der Regel von CDbLayer-Slot 119 (04 §1.5): Nur eine nicht geteilte
    /// Ebene der Tiefe 0 (bzw. Tiefe 1 mit Zustand „Masterseiten-Inhalt“) liefert `cels[frame]`; alle
    /// anderen liefern immer den ersten Cel.
    pub fn cel_at(&self, frame: usize, depth: u32, state: ElementState) -> Option<&Arc<Cel>> {
        let per_frame =
            !self.shared_across_frames && (depth == 0 || (depth == 1 && state == ElementState::MasterPageContent));
        if per_frame { self.cels.get(frame) } else { self.cels.first() }
    }

    /// Kurzform für oberste Ebenen im Normalzustand.
    pub fn cel(&self, frame: usize) -> Option<&Arc<Cel>> {
        self.cel_at(frame, 0, ElementState::Normal)
    }

    /// Vergibt rekursiv neue Laufzeit-IDs (Ebene und alle Elemente).
    pub fn reassign_ids(&mut self, ids: &mut IdGen) {
        self.id = ids.layer();
        for c in &mut self.cels {
            let c = Arc::make_mut(c);
            for e in &mut c.elements {
                Arc::make_mut(e).reassign_ids(ids);
            }
        }
    }
}

/// Eintrag einer Ebenenliste (`LYLv`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LayerSlot {
    /// Eigene Ebene (`LAYv`).
    Own(Arc<Layer>),
    /// Verweis auf eine über Seiten geteilte Ebene (`LSAv{UIDs ISPb}`).
    Shared(SharedLayerRef),
    /// Platzhalter für die Ebenen der Masterseite (`MPLv`); Position frei.
    MasterPage(MasterPageLayer),
    /// Unbekannter Eintrag, unverändert erhalten.
    Raw(RawNode),
}

/// Verweis auf eine geteilte Ebene (`LSAv`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SharedLayerRef {
    /// GUID der geteilten Ebene (`UIDs`).
    pub id: Guid,
    /// `ISPb` am Verweis: CS6 schreibt hier immer 0 (B, Laufzeit 04 T2); nur für die Rundreise.
    pub to_all_pages: bool,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

/// Definition einer über Seiten geteilten Ebene (`LSMv/LSEv{UIDs ISPb LAYv}`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SharedLayerDef {
    pub id: Guid,
    /// `ISPb`: auf alle Seiten verteilt (auch künftige; B, Laufzeit 04 T2).
    pub to_all_pages: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<Arc<Layer>>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

/// Platzhalter der Masterseiten-Ebenen (`MPLv{PIDs DISb LNMs VIFv}`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MasterPageLayer {
    /// GUID der Masterseite (`PIDs`); Null-GUID = nicht verknüpft.
    pub master: Guid,
    /// Aufgeklappt (`DISb`).
    pub expanded: bool,
    /// Name (`LNMs`).
    pub name: String,
    /// Sichtbarkeit (`VIFv`).
    pub visibility: Vec<bool>,
    pub has_visibility_node: bool,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub visibility_extra: Extra,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

impl MasterPageLayer {
    pub fn is_visible(&self) -> bool {
        self.visibility.first().copied().unwrap_or(true)
    }
    /// Verweis auf die Masterseite, `None` bei Null-GUID.
    pub fn master_id(&self) -> Option<&Guid> {
        if self.master.is_nil() { None } else { Some(&self.master) }
    }
}

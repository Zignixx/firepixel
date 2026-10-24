//! Elemente (`ELMv`-Einträge): Bitmap, Pfad (inkl. Rechteck-Primitive), Text, Gruppe (inkl. Maske und
//! Smart Shape), Compound Shape, Symbolinstanz, Web-Objekt, Unterebene.
//!
//! Elemente stehen in `ELMv` **von oben nach unten** (02 §4.3). Koordinaten sind Dokumentkoordinaten;
//! Matrizen nur bei Instanz, Text, Primitive und Smart Shape (Zeilenvektor-Konvention).

use crate::attrs::{FillHandles, PathAttrs};
use crate::data::CustomData;
use crate::effects::EffectList;
use crate::ids::{ElementId, IdGen, SymbolId};
use crate::image::ImageData;
use crate::layer::Layer;
use crate::raw::{Extra, RawNode};
use crate::serde_ext::{self, default_true, is_false, is_true};
use crate::text::TextElem;
use crate::web::{Behaviors, WebObjectElem};
use fp_color::{BlendMode, Opacity};
use fp_geom::{Matrix3, Point, Rect};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Ein Element mit gemeinsamen Attributen, Art, Render-Cache und Rundreise-Daten.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Element {
    /// Laufzeit-ID (nicht gespeichert).
    pub id: ElementId,
    pub common: CommonAttrs,
    pub kind: ElementKind,
    /// Render-Cache (Kind-`IMGv` von Pfad, Text, Gruppe, Instanz, Compound Shape; 02 §10, §12.9):
    /// das gerasterte Aussehen in Dokumentkoordinaten. Kein Dokumentinhalt; bei Inhaltsänderung
    /// verwerfen ([`Element::edit`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache: Option<Arc<BitmapNode>>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

/// Attribute aller Elemente (02 §6.1, 04 §3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommonAttrs {
    /// Objektname (`OBNs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Sichtbar (`VISb`, fehlt = sichtbar; nur `false` wird geschrieben).
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub visible: bool,
    /// Gesperrt (`LCKb`).
    #[serde(default, skip_serializing_if = "is_false")]
    pub locked: bool,
    /// Deckkraft als Byte; Datei `OPAi` in Promille = ⌊b·1000/255⌋ (fehlt = 1000).
    #[serde(with = "serde_ext::opacity", default)]
    pub opacity: Opacity,
    /// Mischmodus (`BLDi`, Nummern siehe 02 §6.3; fehlt = 0 Normal; Datei-Wert 0x37 gilt als Normal).
    #[serde(with = "serde_ext::blend", default)]
    pub blend: BlendMode,
    /// Live-Effekte (`EFLv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effects: Option<EffectList>,
    /// Verknüpfter Stil per Namen (`ZSNs`); oft ohne passenden Stil (04 §2.2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// Custom Data (`DTAv`, JS `customData`).
    #[serde(default)]
    pub custom_data: CustomData,
}

impl Default for CommonAttrs {
    fn default() -> Self {
        CommonAttrs {
            name: None,
            visible: true,
            locked: false,
            opacity: Opacity(255),
            blend: BlendMode(0),
            effects: None,
            style: None,
            custom_data: CustomData::default(),
        }
    }
}

/// Elementarten.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ElementKind {
    /// `IMGv`.
    Bitmap(BitmapElem),
    /// `PTHv` (mit optionaler Rechteck-Primitive `PRIv`).
    Path(PathElem),
    /// `TXTv`.
    Text(Box<TextElem>),
    /// `GRPv` (normal, Maskengruppe oder Smart Shape).
    Group(GroupElem),
    /// `CSGv`.
    CompoundShape(CompoundShapeElem),
    /// `ALSv`.
    Instance(InstanceElem),
    /// `URLv` (Hotspot bzw. Slice).
    WebObject(Box<WebObjectElem>),
    /// `LSLv`: Unterebene (eine Ebene ist im Original ein Element).
    SubLayer(Arc<Layer>),
    /// Unbekannter Elementknoten, unverändert erhalten.
    Raw(RawNode),
}

/// Ein `IMGv`-Knoten als Ganzes: Bitmap-Element, Render-Cache, Effekt-Ergebnis, Musterbild oder
/// Stilvorschau. Enthält die gemeinsamen Attribute, weil auch Caches `LCKb`, `DTAv` usw. tragen.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BitmapNode {
    pub common: CommonAttrs,
    pub bitmap: BitmapElem,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

/// Inhalt einer Bitmap.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BitmapElem {
    pub image: ImageData,
    /// Ergebnis der Live-Effekte (inneres `IMGv`), falls gespeichert. Zum Anzeigen das Ergebnis, zum
    /// Bearbeiten die Originalpixel benutzen (02 §10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect_result: Option<Arc<BitmapNode>>,
}

/// Pfad (`PTHv`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PathElem {
    /// Strich/Füllung/Textur (`PATv`).
    pub attrs: Arc<PathAttrs>,
    /// Strichposition (`BRPi`): 0 innen, 1 mittig, 2 außen.
    pub stroke_placement: i32,
    /// `PTTi` (geschrieben, vom Leser übersprungen).
    pub ptt: i32,
    /// Füllung über Strich (`FOTb`).
    pub fill_on_top: bool,
    /// Zufalls-Startwert (`RNDi`).
    pub random_seed: u32,
    /// Texturversatz (`TOXf TOYf`).
    #[serde(with = "serde_ext::point")]
    pub texture_offset: Point,
    /// Füll-Handles (`PCFb` + `PS*/PE*/PF*`).
    pub fill_handles: FillHandles,
    /// Konturen (`PBLv`).
    pub contours: Vec<Contour>,
    /// Rundreise-Daten von `PBLv`.
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub contours_extra: Extra,
    /// Abgeleitete Polylinie (`PCLv`), nur für alte Leser; CS6 ignoriert sie, wenn `PBLv` vorhanden ist.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polyline: Option<Vec<Polyline>>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub polyline_extra: Extra,
    /// Füllregel (`EOFb`): true = Even-Odd, false = Non-Zero.
    pub even_odd: bool,
    /// „AGM-Rendering verwenden“ (`AGMb`, fehlt = aus).
    pub agm: bool,
    /// Verknüpfungsart als Compound-Shape-Operand (`POPi`, nur ≠ 0 geschrieben).
    pub path_op: u32,
    /// `PRPb`: Stellvertreter-Pfad eines Compound Shapes, der nicht direkt hinter einem `CSGv` stand
    /// (sonst hängt er an [`CompoundShapeElem::proxy`]). Das Original verwirft solche Pfade beim Laden.
    #[serde(default, skip_serializing_if = "is_false")]
    pub compat_proxy: bool,
    /// Rechteck-Primitive (`PRIv`). Der Pfad enthält trotzdem die fertigen Konturen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primitive: Option<RectPrimitive>,
}

/// Kontur (`PBPv{ISCb [BSLf] PPCi PBTv…}`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Contour {
    /// Geschlossen (`ISCb`).
    pub closed: bool,
    /// `BSLf` (nur bei einer Programm-Einstellung geschrieben, immer 0; vom Leser übersprungen).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bsl: Option<f32>,
    pub nodes: Vec<ContourNode>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

/// Knoten einer Kontur (`PBTv`). Segment i→i+1 = kubische Bézierkurve
/// (Anker_i, Ausgang_i, Eingang_{i+1}, Anker_{i+1}).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContourNode {
    /// Anker (`XLCf YLCf`).
    #[serde(with = "serde_ext::point")]
    pub anchor: Point,
    /// Eingangs-Handle (`XPCf YPCf`; fehlende Achse = Anker).
    #[serde(with = "serde_ext::point")]
    pub pred: Point,
    /// Ausgangs-Handle (`XSCf YSCf`; fehlende Achse = Anker).
    #[serde(with = "serde_ext::point")]
    pub succ: Point,
    /// `CRVb` (nur als 1 vorhanden; A: Kurvenpunkt).
    #[serde(default, skip_serializing_if = "is_false")]
    pub curve: bool,
    /// Zufalls-Startwert (`RNDi`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub random: Option<u32>,
    /// Segmentlänge (`NSDf`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub segment_length: Option<f32>,
    /// Druck-/Geschwindigkeitsprofil (`BZLv{BZCi BAZv{PRSf VELf}…}`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pressure: Option<PressureProfile>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

impl ContourNode {
    /// Eckpunkt ohne Handles.
    pub fn corner(p: Point) -> ContourNode {
        ContourNode {
            anchor: p,
            pred: p,
            succ: p,
            curve: false,
            random: None,
            segment_length: None,
            pressure: None,
            extra: Extra::default(),
        }
    }
}

/// Druck- und Geschwindigkeitsprofil eines Segments (`BZLv`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PressureProfile {
    /// Anzahl Stützstellen (`BZCi`, 1 oder 64).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    /// Stützstellen (`BAZv{PRSf VELf}`): (Druck, Geschwindigkeit), fehlende Werte `None`.
    pub samples: Vec<PressureSample>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

/// Eine Stützstelle `BAZv`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PressureSample {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pressure: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub velocity: Option<f32>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

/// Abgeleitete Polylinie einer Kontur (`PPLv{ISCb PPCi PPTv…}`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Polyline {
    pub closed: bool,
    pub points: Vec<PolyPoint>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

/// Punkt der abgeleiteten Polylinie (`PPTv{XLCf YLCf PRSf VELf DURi RNDi [ONCb]}`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PolyPoint {
    #[serde(with = "serde_ext::point")]
    pub pos: Point,
    pub pressure: f32,
    pub velocity: f32,
    pub duration: u32,
    pub random: u32,
    /// `ONCb`: Punkt fällt auf einen Knoten.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_node: Option<bool>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

/// Rechteck-Primitive (`PRIv`, im Original `CDbPrimitiveRect`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RectPrimitive {
    /// Füll-Handles (`PCFb PS* PE* PF*`).
    pub fill_handles: FillHandles,
    /// Abbildung lokal → Dokument (`MTXv`).
    #[serde(with = "serde_ext::matrix")]
    pub transform: Matrix3,
    /// Live-Effekte der Primitive (`EFLv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effects: Option<EffectList>,
    /// Rechteck in lokalen Koordinaten (`RCTv{LFTf TOPf RITf BOTf}`).
    #[serde(with = "serde_ext::rect")]
    pub rect: Rect,
    /// Rundung 0..1 (`RCTv/RDSf`).
    pub roundness: f32,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub rect_extra: Extra,
    /// Rundungsmodus (`RDMb`, A: Pixel statt Prozent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roundness_pixels: Option<bool>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

/// Art einer Gruppe (`GRTi`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroupKind {
    /// 0: normale Gruppe.
    Normal,
    /// 1: Bitmap-Maske (erstes Kind ist die Maske).
    BitmapMask,
    /// 2: Vektor-Maske („mask to path“).
    VectorMask,
    /// Andere Werte bleiben erhalten.
    Other(i32),
}

impl GroupKind {
    pub fn from_code(v: i32) -> GroupKind {
        match v {
            0 => GroupKind::Normal,
            1 => GroupKind::BitmapMask,
            2 => GroupKind::VectorMask,
            o => GroupKind::Other(o),
        }
    }
    pub fn code(self) -> i32 {
        match self {
            GroupKind::Normal => 0,
            GroupKind::BitmapMask => 1,
            GroupKind::VectorMask => 2,
            GroupKind::Other(o) => o,
        }
    }
    /// Maskengruppe (`GRTi` 1 oder 2)?
    pub fn is_mask(self) -> bool {
        matches!(self, GroupKind::BitmapMask | GroupKind::VectorMask)
    }
}

/// Gruppe (`GRPv`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroupElem {
    pub kind: GroupKind,
    /// Bei Maskengruppen das Maskenelement (in der Datei das **erste** Kind).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mask: Option<Arc<Element>>,
    /// Kinder oben → unten (ohne Maske).
    pub children: Vec<Arc<Element>>,
    /// Rundreise-Daten von `ELMv`.
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub children_extra: Extra,
    /// Maskenflags (02 §5.5, 04 §1.6b).
    pub mask_flags: MaskFlags,
    /// `MRMv`: beim Umwandeln Vektor→Bitmap aufbewahrte Original-Vektormaske (B, Laufzeit 02 T4).
    /// Das Original nimmt dessen erstes Element als wirksame Maske.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replacement: Option<Vec<Arc<Element>>>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub replacement_extra: Extra,
    /// Smart Shape (`ISSv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smart_shape: Option<SmartShape>,
}

/// Sicht „Maske eines Elements“ (re 31 §5.2): `mask.element`, `mode`, `linked`, `enabled`,
/// `showAttrs`, `autoExpandImages`; `single` = Element ist ein maskiertes Einzelelement.
#[derive(Clone, Copy, Debug)]
pub struct ElementMask<'a> {
    pub element: &'a Arc<Element>,
    /// „mask to path“ (sonst „mask to image“).
    pub to_path: bool,
    pub linked: bool,
    pub enabled: bool,
    pub show_attrs: bool,
    pub auto_expand: bool,
    pub single: bool,
    /// Firepixel extension (FB-35): inverted mask (what lies inside the mask is hidden).
    pub invert: bool,
}

/// Maskenflags einer Maskengruppe.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaskFlags {
    /// `LNKb`: Maske mit Objekt verknüpft (Vorgabe true, nur `{0}` wird geschrieben).
    pub linked: bool,
    /// `MENb`: Maske aktiv (Vorgabe true).
    pub enabled: bool,
    /// `MNAb`: Maskenattribute ausgeblendet (JS `showAttrs = !MNAb`; nach `group` gesetzt).
    pub hide_attrs: bool,
    /// `MRXb`: A: `autoExpandImages` (nur 1 wird geschrieben).
    pub auto_expand: bool,
    /// Firepixel extension (FB-35, RE spec 68 §6): inverted mask. The final mask value per pixel
    /// becomes `255 − m` (scaled by the mask object's own opacity), so what lies inside the mask
    /// disappears and everything outside stays. FW-PNG: the Firepixel-own tag `fpIb` in `GRPv`;
    /// default off, files of the reference format are unchanged.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub invert: bool,
}

impl Default for MaskFlags {
    fn default() -> Self {
        MaskFlags {
            linked: true,
            enabled: true,
            hide_attrs: false,
            auto_expand: false,
            invert: false,
        }
    }
}

impl GroupElem {
    /// Flagbyte wie das Original es intern bildet (CDbElement+0x2F, Leser `@00a94fd0`): Bit 0 Pfadmaske,
    /// Bit 1 verknüpft, Bit 2 aktiv, Bit 4 Attribute zeigen, Bit 5 autoExpand. Nur bei Maskengruppen.
    pub fn mask_flag_byte(&self) -> Option<u8> {
        if !self.kind.is_mask() {
            return None;
        }
        let f = &self.mask_flags;
        let mut b = 0u8;
        if self.kind == GroupKind::VectorMask {
            b |= 0x01;
        }
        if f.linked {
            b |= 0x02;
        }
        if f.enabled {
            b |= 0x04;
        }
        if !f.hide_attrs {
            b |= 0x10;
        }
        if f.auto_expand {
            b |= 0x20;
        }
        Some(b)
    }

    /// Grayscale mask ("mask to image", `GRTi` 1) whose mask object is a bitmap: its pixels are
    /// stored colour-inverted in the file and kept in display form in memory (RE spec 68 M-6).
    pub fn has_grayscale_bitmap_mask(&self) -> bool {
        self.kind == GroupKind::BitmapMask
            && self
                .mask
                .as_ref()
                .is_some_and(|m| matches!(m.kind, ElementKind::Bitmap(_)))
    }

    /// Converts the mask bitmap of a grayscale bitmap mask between file form and display form
    /// (an involution: colour channels `255 − v`, alpha kept). Other groups stay unchanged.
    pub fn invert_grayscale_bitmap_mask(&mut self) {
        if !self.has_grayscale_bitmap_mask() {
            return;
        }
        if let Some(m) = &mut self.mask {
            let m = Arc::make_mut(m);
            if let ElementKind::Bitmap(b) = &mut m.kind {
                b.image = b.image.inverted_rgb();
                if let Some(r) = &mut b.effect_result {
                    let r = Arc::make_mut(r);
                    r.bitmap.image = r.bitmap.image.inverted_rgb();
                }
            }
        }
    }

    /// Wirksame Maske: bei `MRMv` dessen erstes Element, sonst das erste Kind.
    pub fn effective_mask(&self) -> Option<&Arc<Element>> {
        if let Some(r) = &self.replacement
            && let Some(e) = r.first()
        {
            return Some(e);
        }
        self.mask.as_ref()
    }
}

/// Smart Shape (Auto-Form, `ISSv`, 13 §4.4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SmartShape {
    /// Formname (`FILs`), z. B. „Rounded Rectangle“.
    pub name: String,
    /// Skript-Quelltext in Stücken zu höchstens 32.767 Zeichen (`JSVv{JSSs…}`).
    pub script_chunks: Vec<String>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub script_extra: Extra,
    /// Abbildung Form → Dokument (`MTXv`).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_ext::opt_matrix"
    )]
    pub transform: Option<Matrix3>,
    /// Steuerpunkte (`CPSv{CPTv…}`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_points: Option<Vec<ControlPoint>>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub control_points_extra: Extra,
    /// Ereignisnamen (`EVVv{EVTs…}`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub events_extra: Extra,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

impl SmartShape {
    /// Vollständiger Skripttext (Stücke hintereinander).
    pub fn script(&self) -> String {
        self.script_chunks.concat()
    }

    /// Setzt den Skripttext und teilt ihn in Stücke ≤ 32.767 UTF-16-Einheiten.
    pub fn set_script(&mut self, s: &str) {
        self.script_chunks = split_utf16_chunks(s, 32_767);
    }
}

/// Teilt einen Text in Stücke mit höchstens `max` UTF-16-Einheiten (ohne Surrogatpaare zu trennen).
pub fn split_utf16_chunks(s: &str, max: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut n = 0usize;
    for c in s.chars() {
        let l = c.len_utf16();
        if n + l > max && !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
            n = 0;
        }
        cur.push(c);
        n += l;
    }
    if !cur.is_empty() || out.is_empty() {
        out.push(cur);
    }
    out
}

/// Steuerpunkt einer Smart Shape (`CPTv`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlPoint {
    #[serde(with = "serde_ext::point")]
    pub position: Point,
    /// `CTYi` (immer 0 = default).
    pub kind: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,
    /// `TTDb` (toolTipTracksDrag).
    pub tooltip_tracks_drag: bool,
    pub visible: bool,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

/// Compound Shape (`CSGv`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompoundShapeElem {
    /// `GRTi` (immer 0 im Korpus).
    pub group_type: i32,
    /// Berechnetes Ergebnis (direktes Kind `PTHv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Arc<Element>>,
    /// Operanden (Pfade mit `POPi`), oben → unten.
    pub operands: Vec<Arc<Element>>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub operands_extra: Extra,
    /// Stellvertreter-Pfad mit `PRPb{1}`, der in der Datei direkt hinter dem `CSGv` steht
    /// (für alte Leser). Fehlt er beim Schreiben, erzeugt der Schreiber ihn aus `result`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy: Option<Arc<Element>>,
}

/// Symbolinstanz (`ALSv`, 02 §5.6, 13 §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InstanceElem {
    /// Verweis auf `MSTv/MSNi`.
    pub symbol_id: SymbolId,
    /// `ALNi`: eigene ID der Instanz (das Original vergibt sie beim Laden neu; bei Grafik-Instanzen fehlt sie).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<u32>,
    /// Symbol → Eltern (`MTXv`, Pflicht).
    #[serde(with = "serde_ext::matrix")]
    pub transform: Matrix3,
    /// Zweite Matrix (`ATMv`): Verlaufsstand, für die Darstellung bedeutungslos (13 §7.4).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_ext::opt_matrix"
    )]
    pub alt_transform: Option<Matrix3>,
    /// Transformationsmodus (`XFMi`; fehlt = 1).
    pub transform_mode: i32,
    /// Animationsparameter (`ANNi ANXf ANYf ANRi ASCf ASOi AEOi`).
    pub anim: AnimParams,
    /// Rich-Symbol-Parameter (`WPVs`): `Name&|&Wert&|&Typ&;&…`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub widget_params: Option<String>,
    /// Schaltflächen-Zeichenketten der Instanz (erben vom Symbol, wenn fehlend; 13 §7.5).
    #[serde(default)]
    pub button: ButtonStrings,
    /// „Down beim Laden zeigen“ (`DVLb`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_down_on_load: Option<bool>,
    /// Verhalten (`BEVv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub behaviors: Option<Behaviors>,
}

/// Animationsparameter einer Instanz (13 §7.3). Vorgaben des Lesers: 5 / 0 / 0 / 0 / 100 / 1000 / 1000.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnimParams {
    /// Anzahl Status (`ANNi`).
    pub frames: u32,
    /// Gesamtversatz bis zum letzten Status in px (`ANXf`, `ANYf`).
    #[serde(with = "serde_ext::point")]
    pub offset: Point,
    /// Gesamtdrehung in Grad (`ANRi`, im Uhrzeigersinn).
    pub rotation: i32,
    /// Endskalierung in % (`ASCf`).
    pub scale: f32,
    /// Start-/End-Deckkraft in Promille (`ASOi`, `AEOi`).
    pub start_opacity: u32,
    pub end_opacity: u32,
    /// Nur für die Grenzen (BA-020, F16): `true` = Weg waagerecht um seine Länge („Verschieben“,
    /// Richtung ohne Wirkung) wie nach dem Animieren-Dialog im Original (B-2b, PM-6); `false` = Weg
    /// als Vektor wie nach JSF `convertToAnimSymbol` (B-2) und beim Lesen einer Datei.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bounds_horizontal: bool,
}

impl Default for AnimParams {
    fn default() -> Self {
        AnimParams {
            frames: 5,
            offset: Point::default(),
            rotation: 0,
            scale: 100.0,
            start_opacity: 1000,
            end_opacity: 1000,
            bounds_horizontal: false,
        }
    }
}

/// Zustand einer Animationsinstanz in einem Status (13 §7.3, Laufzeit T6 bestätigt).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimFrame {
    /// Gezeigter Symbolstatus (`k mod FRC`).
    pub symbol_frame: usize,
    /// Fortschritt `t = min(k, n−1) / max(n−1, 1)`.
    pub t: f64,
    /// Skalierung um die Symbolmitte: `ASC·t/100 + (1 − t)`.
    pub scale: f64,
    /// Drehung um die Symbolmitte in Grad, im Uhrzeigersinn: `t·ANR`.
    pub rotation: f64,
    /// Versatz `t·(ANX, ANY)` (nach Skalierung/Drehung).
    pub offset: Point,
    /// Deckkraft-Byte `round_half_away((1−t)·ASO + t·AEO)` (Bytes aus Promille gerundet).
    pub opacity: Opacity,
}

impl AnimParams {
    /// Zustand im Instanzstatus `k` (= Seitenstatus − Startstatus) bei `symbol_frames` Status des
    /// Symbols. `None`, wenn die Kopie entfällt (Skalierung ≤ 0).
    pub fn frame(&self, k: usize, symbol_frames: u32) -> Option<AnimFrame> {
        let n = self.frames.max(1) as usize;
        let t = (k.min(n - 1)) as f64 / ((n - 1).max(1)) as f64;
        let scale = self.scale as f64 * t / 100.0 + (1.0 - t);
        if scale <= 0.0 {
            return None;
        }
        let to_byte = |p: u32| ((p.min(1000) as f64) / 1000.0 * 255.0).round();
        let t_op = (k % n) as f64 / ((n - 1).max(1)) as f64;
        let op = (1.0 - t_op) * to_byte(self.start_opacity) + t_op * to_byte(self.end_opacity);
        Some(AnimFrame {
            symbol_frame: k % symbol_frames.max(1) as usize,
            t,
            scale,
            rotation: t * self.rotation as f64,
            offset: Point::new(
                (t * self.offset.x as f64) as f32,
                (t * self.offset.y as f64) as f32,
            ),
            opacity: Opacity(op.round().clamp(0.0, 255.0) as u8),
        })
    }
}

/// Ein Rich-Symbol-Parameter aus `WPVs` (`Name&|&Wert&|&Typ`, Einträge durch `&;&` getrennt).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WidgetParam {
    pub name: String,
    pub value: String,
    pub kind: String,
}

impl InstanceElem {
    /// Zerlegt `WPVs` in Parameter (leere Einträge werden übersprungen).
    pub fn widget_values(&self) -> Vec<WidgetParam> {
        parse_widget_params(self.widget_params.as_deref().unwrap_or(""))
    }
}

/// Zerlegt einen `WPVs`-Text (13 §4.1).
pub fn parse_widget_params(s: &str) -> Vec<WidgetParam> {
    s.split("&;&")
        .filter(|e| !e.is_empty())
        .map(|e| {
            let mut it = e.split("&|&");
            WidgetParam {
                name: it.next().unwrap_or("").to_string(),
                value: it.next().unwrap_or("").to_string(),
                kind: it.next().unwrap_or("").to_string(),
            }
        })
        .collect()
}

/// Setzt einen `WPVs`-Text aus Parametern zusammen.
pub fn format_widget_params(p: &[WidgetParam]) -> String {
    let parts: Vec<String> = p
        .iter()
        .map(|w| format!("{}&|&{}&|&{}", w.name, w.value, w.kind))
        .collect();
    parts.join("&;&")
}

/// Zeichenketten von Schaltflächen (Symbol `MSTv` und Instanz `ALSv`).
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct ButtonStrings {
    /// Link-URL (`INMs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Alt-Text (`A2Ts`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
    /// Ziel (`ALTs`; bei Schaltflächen = Ziel-Frame, B Laufzeit 02 T7).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// Slice-/Dateiname (`FILs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    /// Schaltflächentext der Instanz (`BUTs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl Element {
    /// Neues Element mit Vorgabe-Attributen.
    pub fn new(id: ElementId, kind: ElementKind) -> Element {
        Element {
            id,
            common: CommonAttrs::default(),
            kind,
            cache: None,
            extra: Extra::default(),
        }
    }

    /// Copy-on-Write-Zugriff für Bearbeitungen; verwirft den Render-Cache (wie das Original bei
    /// Inhaltsänderung, 04 §1.6c).
    pub fn edit(this: &mut Arc<Element>) -> &mut Element {
        let e = Arc::make_mut(this);
        e.cache = None;
        e
    }

    /// Verschiebt einen gehaltenen Render-Cache mit (im Original Slot 2). Die Geometrie verschiebt der
    /// Aufrufer selbst.
    pub fn move_cache(&mut self, dx: f32, dy: f32) {
        if dx == 0.0 && dy == 0.0 {
            return;
        }
        if let Some(c) = &mut self.cache {
            let c = Arc::make_mut(c);
            c.bitmap.image.origin.x += dx;
            c.bitmap.image.origin.y += dy;
        }
    }

    /// Duplikat mit neuen IDs (auch für alle Kinder und Unterebenen).
    pub fn duplicate(&self, ids: &mut IdGen) -> Element {
        let mut e = self.clone();
        e.reassign_ids(ids);
        e
    }

    /// Vergibt rekursiv neue Laufzeit-IDs.
    pub fn reassign_ids(&mut self, ids: &mut IdGen) {
        self.id = ids.element();
        let redo = |v: &mut Vec<Arc<Element>>, ids: &mut IdGen| {
            for c in v.iter_mut() {
                Arc::make_mut(c).reassign_ids(ids);
            }
        };
        match &mut self.kind {
            ElementKind::Group(g) => {
                if let Some(m) = &mut g.mask {
                    Arc::make_mut(m).reassign_ids(ids);
                }
                redo(&mut g.children, ids);
                if let Some(r) = &mut g.replacement {
                    redo(r, ids);
                }
            }
            ElementKind::CompoundShape(c) => {
                if let Some(r) = &mut c.result {
                    Arc::make_mut(r).reassign_ids(ids);
                }
                redo(&mut c.operands, ids);
                if let Some(p) = &mut c.proxy {
                    Arc::make_mut(p).reassign_ids(ids);
                }
            }
            ElementKind::SubLayer(l) => Arc::make_mut(l).reassign_ids(ids),
            ElementKind::Text(t) => {
                if let Some(p) = &mut t.on_path
                    && let Some(e) = &mut p.path
                {
                    Arc::make_mut(e).reassign_ids(ids);
                }
            }
            _ => {}
        }
    }

    /// Direkte Kinder (Gruppe ohne Maske, Operanden eines Compound Shapes).
    pub fn children(&self) -> &[Arc<Element>] {
        match &self.kind {
            ElementKind::Group(g) => &g.children,
            ElementKind::CompoundShape(c) => &c.operands,
            _ => &[],
        }
    }

    /// Typcode wie Original-Slot 88 (04 §1.5): Text 0x3300, Gruppe 0x3301, Bitmap 0x3302,
    /// Smart-Shape-Gruppe 0x3303, Rechteck-Primitive 0x3304, Instanz 0x3305, Pfad 0x3306,
    /// Compound Shape 0x3307; Hotspot/Slice 0x1B21 (CDbURL-Slot 136); sonst 0.
    pub fn type_code(&self) -> u32 {
        match &self.kind {
            ElementKind::Text(_) => 0x3300,
            ElementKind::Group(g) if g.smart_shape.is_some() => 0x3303,
            ElementKind::Group(_) => 0x3301,
            ElementKind::Bitmap(_) => 0x3302,
            ElementKind::Path(p) if p.primitive.is_some() => 0x3304,
            ElementKind::Instance(_) => 0x3305,
            ElementKind::Path(_) => 0x3306,
            ElementKind::CompoundShape(_) => 0x3307,
            ElementKind::WebObject(_) => 0x1B21,
            ElementKind::SubLayer(_) | ElementKind::Raw(_) => 0,
        }
    }

    /// Maskengruppe mit genau einem Inhalt ohne eigene Maske, ohne eigene Effekte des Inhalts und
    /// ohne Smart Shape: im Original ein **maskiertes Einzelelement** (04 §1.6b; „Maske ▸ Alles
    /// einblenden“ auf einer Bitmap bleibt `[object Image]` mit `mask`, gemessen FIX-G5). Effekte der
    /// Gruppe sind die des Elements; maßgebend sind die Effekte des **Kindes** (Rundreise rt_I/rt_J,
    /// FIX-N35). Liefert (Inhalt, Maske).
    pub fn masked_single(&self) -> Option<(&Arc<Element>, &Arc<Element>)> {
        let ElementKind::Group(g) = &self.kind else {
            return None;
        };
        if !g.kind.is_mask()
            || g.children.len() != 1
            || g.smart_shape.is_some()
            || g.children[0]
                .common
                .effects
                .as_ref()
                .is_some_and(|l| !l.effects.is_empty())
        {
            return None;
        }
        let child = &g.children[0];
        if matches!(&child.kind, ElementKind::Group(c) if c.kind.is_mask()) {
            return None;
        }
        g.mask.as_ref().map(|m| (child, m))
    }

    /// Maske als **Attribut des Elements** (re 31 §5.1/§5.2; im Original Zeiger an `CDbElement`+0x18,
    /// Flagbyte +0x2F). Im Speicher hält Firepixel die Maske weiter an einer Maskengruppe (Datei-
    /// Darstellung `GRPv{GRTi}`); diese Sicht liefert für jedes maskierte Element – Einzelelement
    /// ([`Element::masked_single`]) wie Maskengruppe – Maskenelement und Schalter.
    pub fn element_mask(&self) -> Option<ElementMask<'_>> {
        let ElementKind::Group(g) = &self.kind else {
            return None;
        };
        if !g.kind.is_mask() {
            return None;
        }
        let element = g.mask.as_ref()?;
        Some(ElementMask {
            element,
            to_path: g.kind == GroupKind::VectorMask,
            linked: g.mask_flags.linked,
            enabled: g.mask_flags.enabled,
            show_attrs: !g.mask_flags.hide_attrs,
            auto_expand: g.mask_flags.auto_expand,
            single: self.masked_single().is_some(),
            invert: g.mask_flags.invert,
        })
    }

    /// Das Element, das der Nutzer sieht: bei einem maskierten Einzelelement der Inhalt (Klasse,
    /// Name, Grenzen, `pathAttributes`, re 31 §5.5), sonst das Element selbst.
    pub fn masked_content(&self) -> &Element {
        self.masked_single().map(|(c, _)| &**c).unwrap_or(self)
    }

    /// JS-Typname wie `"[object Path]"` ohne Klammern (A, 04 §1.4). Ein maskiertes Einzelelement
    /// ([`Element::masked_single`]) zeigt den Typ seines Inhalts wie im Original.
    pub fn js_type(&self) -> &'static str {
        if let Some((child, _)) = self.masked_single() {
            return child.js_type();
        }
        match &self.kind {
            ElementKind::Bitmap(_) => "Image",
            ElementKind::Path(p) if p.primitive.is_some() => "RectanglePrimitive",
            ElementKind::Path(_) => "Path",
            ElementKind::Text(_) => "Text",
            ElementKind::Group(_) => "Group",
            ElementKind::CompoundShape(_) => "CompoundShape",
            ElementKind::Instance(_) => "Instance",
            ElementKind::WebObject(w) if w.slice.is_some() => "SliceHotspot",
            ElementKind::WebObject(_) => "Hotspot",
            ElementKind::SubLayer(_) => "Layer",
            ElementKind::Raw(_) => "Element",
        }
    }

    /// Der Render-Cache darf zum Anzeigen benutzt werden (02 §15 Punkt 6, Laufzeit T5): Gruppen- und
    /// Instanz-Caches ja (Instanzen nur, wenn `format_version` ≥ 10 und kein Animationssymbol),
    /// Pfad-Caches nein (Pfad neu rastern). Für Texte entscheidet der Aufrufer (Schrift vorhanden?).
    pub fn cache_is_displayable(&self, format_version: u32, symbol_is_animation: bool) -> bool {
        if self.cache.is_none() {
            return false;
        }
        match &self.kind {
            ElementKind::Group(_) | ElementKind::CompoundShape(_) => true,
            ElementKind::Instance(_) => format_version > 9 && !symbol_is_animation,
            ElementKind::Text(_) => true,
            _ => false,
        }
    }
}

impl BitmapNode {
    /// Bitmap aus RGBA8-Pixeln an der Position `origin`.
    pub fn from_rgba(
        origin: Point,
        width: u32,
        height: u32,
        rgba: &[u8],
        next_tile_id: &mut u32,
    ) -> BitmapNode {
        BitmapNode {
            common: CommonAttrs::default(),
            bitmap: BitmapElem {
                image: ImageData {
                    origin,
                    offset: Point::default(),
                    grid: crate::image::TileGrid::from_rgba(width, height, rgba, next_tile_id),
                },
                effect_result: None,
            },
            extra: Extra::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_chunks() {
        let s = "a".repeat(70_000);
        let c = split_utf16_chunks(&s, 32_767);
        assert_eq!(c.len(), 3);
        assert_eq!(c.concat(), s);
        assert_eq!(split_utf16_chunks("", 10), vec![String::new()]);
        let e = "😀".repeat(3);
        let c = split_utf16_chunks(&e, 3);
        assert_eq!(
            c,
            vec!["😀".to_string(), "😀".to_string(), "😀".to_string()]
        );
    }

    #[test]
    fn widget_params_roundtrip() {
        let s = "Inbox&|&1&|&number&;&Outbox&|&10&|&number";
        let p = parse_widget_params(s);
        assert_eq!(p.len(), 2);
        assert_eq!(p[1].value, "10");
        assert_eq!(format_widget_params(&p), s);
    }

    #[test]
    fn anim_frames_like_t6() {
        // Laufzeit 13 T6: 5 Status, Versatz 100, Drehung 90, Skalierung 50 %, Deckkraft 1000 → 0.
        let a = AnimParams {
            frames: 5,
            offset: Point::new(100.0, 0.0),
            rotation: 90,
            scale: 50.0,
            start_opacity: 1000,
            end_opacity: 0,
            bounds_horizontal: false,
        };
        let f0 = a.frame(0, 1).unwrap();
        assert_eq!((f0.scale, f0.rotation, f0.opacity.0), (1.0, 0.0, 255));
        let f2 = a.frame(2, 1).unwrap();
        assert_eq!(f2.t, 0.5);
        assert_eq!(f2.scale, 0.75);
        assert_eq!(f2.offset, Point::new(50.0, 0.0));
        assert_eq!(f2.opacity.0, 128);
        assert_eq!(a.frame(4, 1).unwrap().opacity.0, 0);
    }

    #[test]
    fn mask_flags() {
        let g = GroupElem {
            kind: GroupKind::VectorMask,
            mask: None,
            children: vec![],
            children_extra: Extra::default(),
            mask_flags: MaskFlags {
                hide_attrs: true,
                ..MaskFlags::default()
            },
            replacement: None,
            replacement_extra: Extra::default(),
            smart_shape: None,
        };
        assert_eq!(g.mask_flag_byte(), Some(0x07));
    }
}

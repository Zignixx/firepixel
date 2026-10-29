//! Strich, Füllung, Pinsel, Verlauf, Muster, Textur (`PATv`, Spezifikationen 02 §6.2, 07 §5–6).

use crate::element::BitmapNode;
use crate::image::MaskData;
use crate::raw::Extra;
use crate::serde_ext;
use fp_color::Argb;
use fp_geom::Point;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Attribute eines Pfads bzw. Texts (`PATv`). Wird über `Arc` geteilt (Original: „Looks“).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PathAttrs {
    /// Strichfarbe ARGB (`BCLi`).
    #[serde(with = "serde_ext::argb")]
    pub stroke_color: Argb,
    /// Füllfarbe ARGB (`FCLi`), bei Vollfarbe.
    #[serde(with = "serde_ext::argb")]
    pub fill_color: Argb,
    /// Pinsel/Strich (`BPLv`); fehlt = kein Strich.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brush: Option<Brush>,
    /// Füllung (`FPLv`); fehlt = keine Füllung.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<Fill>,
    /// Textur des Strichs (`TXBv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_texture: Option<Texture>,
    /// Textur der Füllung (`TXFv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_texture: Option<Texture>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

impl Default for PathAttrs {
    fn default() -> Self {
        PathAttrs {
            stroke_color: Argb(0xff00_0000),
            fill_color: Argb(0xff00_00ff),
            brush: None,
            fill: None,
            stroke_texture: None,
            fill_texture: None,
            extra: Extra::default(),
        }
    }
}

/// Füll-Handles für Verläufe und Muster (`PCFb PSXf PSYf PEXf PEYf PFXf PFYf`).
#[derive(Clone, Copy, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct FillHandles {
    /// `PCFb`: Handles gültig (Leser übernimmt sie nur dann; Korpus immer 1).
    pub valid: bool,
    /// Handle 1 (Start): `PSXf PSYf`.
    #[serde(with = "serde_ext::point")]
    pub start: Point,
    /// Handle 2 (Ende): `PEXf PEYf`.
    #[serde(with = "serde_ext::point")]
    pub end: Point,
    /// Handle 3 (zweite Achse): `PFXf PFYf`.
    #[serde(with = "serde_ext::point")]
    pub second: Point,
}

/// Füllung (`FPLv`, im Original `CDbFillPathLook`, 07 §5.1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fill {
    /// Kategorie (`CATs`): `fc_Solid`, `fc_Linear`, `fc_Circular`, `fc_Pattern` …
    pub category: String,
    /// Interner Name (`INMs`).
    pub name: String,
    /// Anzeigename (`UNMs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
    /// `RDOb` (immer 0; A: schreibgeschützte Vorgabe).
    pub read_only: bool,
    /// Texturmenge in Promille (`FTBi`).
    pub texture_amount: i32,
    /// Weiche Kante in px (`FEFi`).
    pub feather: i32,
    /// Stempelmodus (`FSMi`): 1 = blend, 2 = blend opaque.
    pub stamping_mode: i32,
    /// Kante (`FETi`): 0 hart, 1 kantengeglättet, 2 weiche Kante; 3 = Dither
    /// ([`Fill::EDGE_DITHER`], Firepixel extension; `feather` holds its width).
    pub edge: i32,
    /// `FRDb` (Dither-Flag, nicht gerendert).
    pub dither: bool,
    /// `FRRi`.
    pub dither_amount: i32,
    /// Form (`FSHi`): 0 Vollfarbe, 2 linear, 3 radial, 4 konisch, 5 Satin, 7 Pinch, 8 Falten, 9 elliptisch,
    /// 10 rechteckig, 12 Balken, 13 Kräuseln, 14 Wellen, 15 Muster, 16 Web-Dither.
    pub shape: i32,
    /// Erweiterte Form (`FSXi`): 17 = Konturverlauf.
    pub shape_ext: i32,
    /// Web-Dither-Farben (`FD1i–FD4i`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dither_colors: Option<[u32; 4]>,
    /// Transparenz-Dither (`FDTb`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transparent_dither: Option<bool>,
    /// Verlauf (`FGLv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gradient: Option<Gradient>,
    /// Muster (`FPTv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<Pattern>,
    /// "Lock pattern to document" (Firepixel extension, FB-44): moving or transforming the object
    /// leaves the fill handles of a pattern or gradient where they are in the document, so the
    /// pattern stays put while the object moves. The reference has no such option; FW-PNG keeps it
    /// only in the Firepixel-own marker `fpEi` (bit [`Fill::EXT_LOCK_TO_DOCUMENT`]). Default off.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub lock_to_document: bool,
    /// Uniform dither density in per cent (Firepixel extension, FB-38, FB-109): the share of
    /// empty pixels of an ordered 8 × 8 on/off half tone of the whole fill; 0 = off,
    /// 50 = checkerboard, 100 = every pixel empty (monotonic). In FW-PNG it is stored only in the
    /// Firepixel-own marker `fpDe` of the fill (written when > 0), so other readers show the plain
    /// fill; the older marker `fpDi` (share of kept pixels, 100 = off) is still read and converted.
    #[serde(default, skip_serializing_if = "is_zero_i32")]
    pub dither_density: i32,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

fn is_zero_i32(v: &i32) -> bool {
    *v == 0
}

impl Fill {
    /// Bit of the Firepixel-own fill marker `fpEi` for [`Fill::lock_to_document`] (FB-44); the low
    /// bits `3` are the dither edge ([`Fill::EDGE_DITHER`]), so a marker reads `3`, `4` or `7`.
    pub const EXT_LOCK_TO_DOCUMENT: i32 = 4;

    /// Does the fill keep its handles in the document while the object moves or is transformed
    /// (FB-44)? Only patterns and gradients have handles that matter.
    pub fn locked_to_document(&self) -> bool {
        self.lock_to_document && (self.shape == 15 || (self.gradient.is_some() && self.shape != 0))
    }

    /// Edge kind "hard" (`FETi` 0).
    pub const EDGE_HARD: i32 = 0;
    /// Edge kind "anti-alias" (`FETi` 1).
    pub const EDGE_ANTIALIAS: i32 = 1;
    /// Edge kind "feather" (`FETi` 2, width in `feather`).
    pub const EDGE_FEATHER: i32 = 2;
    /// Edge kind "dither" (Firepixel extension, FB-23): the feather ramp of width `feather`
    /// quantised to fully on/off pixels by an ordered 8 × 8 threshold on document pixels. In
    /// FW-PNG it is stored as a feather (`FETi` 2, `FEFi` = width; `FETi` 0 at width 0) plus the
    /// Firepixel-own marker `fpEi` = 3, so other readers show a feathered (or hard) edge.
    pub const EDGE_DITHER: i32 = 3;

    /// Does the edge kind use the width in `feather` (feather and dither)?
    pub fn edge_uses_amount(edge: i32) -> bool {
        edge == Self::EDGE_FEATHER || edge == Self::EDGE_DITHER
    }

    /// Vollfarbige, kantengeglättete Füllung mit Original-Vorgaben (`fc_Solid`, Stempelmodus 1).
    pub fn solid() -> Fill {
        Fill {
            category: "fc_Solid".into(),
            name: "Solid".into(),
            user_name: Some("Solid".into()),
            read_only: false,
            texture_amount: 0,
            feather: 0,
            stamping_mode: 1,
            edge: 1,
            dither: false,
            dither_amount: 0,
            shape: 0,
            shape_ext: 0,
            dither_colors: None,
            transparent_dither: None,
            gradient: None,
            pattern: None,
            lock_to_document: false,
            dither_density: 0,
            extra: Extra::default(),
        }
    }

    /// Ist es ein Konturverlauf (`FSHi` 3 mit `FSXi` 17)?
    pub fn is_contour_gradient(&self) -> bool {
        self.shape_ext == 17
    }
}

/// Verlauf (`FGLv`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Gradient {
    pub category: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
    /// Farbstopps alte Form (`FGVv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy_stops: Option<StopList>,
    /// Neue Form (`FGYv{FG0v Farbstopps, FG1v Deckkraftstopps, FGDb, FGDi}`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stops: Option<GradientStops>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

impl Gradient {
    /// Wirksame Farbstopps: neue Form, sonst alte Form (07 §5.3).
    pub fn color_stops(&self) -> &[GradientStop] {
        if let Some(s) = &self.stops {
            return &s.colors.stops;
        }
        self.legacy_stops
            .as_ref()
            .map(|s| s.stops.as_slice())
            .unwrap_or(&[])
    }

    /// Deckkraftstopps (nur neue Form; Farbe = Graustufe der Deckkraft im Alpha-Kanal).
    pub fn opacity_stops(&self) -> &[GradientStop] {
        self.stops
            .as_ref()
            .map(|s| s.opacities.stops.as_slice())
            .unwrap_or(&[])
    }
}

/// `FGYv`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GradientStops {
    /// Farbstopps (`FG0v`).
    pub colors: StopList,
    /// Deckkraftstopps (`FG1v`).
    pub opacities: StopList,
    /// `FGDb` (A: Dither, ab CS5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dither: Option<bool>,
    /// `FGDi` (?).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dither_value: Option<i32>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

/// Stoppliste (`FGVv`, `FG0v`, `FG1v`): `FNCi` = Anzahl, je Stopp `FGIv{FGPf FGCi}`.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct StopList {
    pub stops: Vec<GradientStop>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

/// Ein Stopp `FGIv`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GradientStop {
    /// Position 0..1 (`FGPf`).
    pub position: f32,
    /// Farbe ARGB (`FGCi`).
    #[serde(with = "serde_ext::argb")]
    pub color: Argb,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

/// Muster (`FPTv{INMs UNMs IMGv}`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pattern {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
    /// Musterbild.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<Arc<BitmapNode>>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

/// Textur (`TXFv`/`TXBv{INMs [UNMs] MSKv}`): Name und 8-Bit-Maske.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Texture {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mask: Option<MaskData>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

/// Pinsel (`BPLv`, im Original `IDbStdBrushParms`, 07 §6.1). Prozentwerte in Promille.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Brush {
    /// Kategorie (`CATs`): `bc_Pencil`, `bc_Basic`, `bc_Air Brush`, …
    pub category: String,
    /// Name (`INMs`).
    pub name: String,
    /// Anzeigename (`UNMs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
    /// `RDOb`.
    pub read_only: bool,
    /// Winkel 0..359 (`BANi`).
    pub angle: i32,
    /// Seitenverhältnis ‰ (`BASi`).
    pub aspect: i32,
    /// Durchmesser in px (`BDIi`).
    pub diameter: i32,
    /// Anzahl Größen-/Winkelstufen 0..64 (`BMMi`, JS `maxCount`).
    pub max_count: i32,
    /// Mindestgröße ‰ (`BMSi`).
    pub min_size: i32,
    /// Weichheit ‰ (`BSEi`).
    pub softness: i32,
    /// Weichzeichnungsmodus (`BSFi`): 1 Glockenkurve, 0 linear.
    pub soften_mode: i32,
    /// Spitzenform (`BSHi`): 1 rund, 0 eckig.
    pub shape: i32,
    /// `BBLi` (immer 0, ohne Wirkung).
    pub bbl: i32,
    /// Schwärze ‰ (`BBKi`).
    pub blackness: i32,
    /// Konzentration ‰ (`BCNi`).
    pub concentration: i32,
    /// alphaRemap (`BEFi`): 0 none … 5 white neon edge.
    pub alpha_remap: i32,
    /// Typ (`BRTi`): 0 natural, 1 simple.
    pub brush_type: i32,
    /// Rückkopplung (`BFBi`): 0 none, 1 brush, 2 background.
    pub feedback: i32,
    /// Fluss ‰ (`BFRi`).
    pub flow_rate: i32,
    /// Anzahl Spitzen 1..32 (`BNTi`).
    pub tip_count: i32,
    /// Abstand ‰ (`BSPi`).
    pub spacing: i32,
    /// Texturmischung ‰ (`BTBi`).
    pub texture_blend: i32,
    /// Kantentextur ‰ (`BTEi`).
    pub texture_edge: i32,
    /// Spitzenabstand ‰ (`BTSi`).
    pub tip_spacing: i32,
    /// Spitzenabstandsmodus (`BSMi`): 0 random, 1 diagonal, 2 circular.
    pub tip_spacing_mode: i32,
    /// Spitzenfarbmodus (`BCMi`): 0 random, 1 uniform, 2 complementary, 3 hue, 4 shadow.
    pub tip_coloring_mode: i32,
    /// Kantenglättung (`BIAb`).
    pub antialiased: bool,
    /// Anzahl Strichmuster-Paare 0..3 (`NDIi`).
    pub dash_count: i32,
    /// Strichlängen (`DO1i–DO3i`).
    pub dash_on: [i32; 3],
    /// Lückenlängen (`DF1i–DF3i`).
    pub dash_off: [i32; 3],
    /// Empfindlichkeiten `S<Quelle><Ziel>i` in Dateireihenfolge.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sensitivities: Vec<Sensitivity>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

/// Empfindlichkeit eines Pinsels: `S<Quelle><Ziel>i` (07 §6.1).
/// Quelle: `P` Druck, `S` Geschwindigkeit, `H`/`V` Richtung, `R` Zufall.
/// Ziel: `Z` Größe, `A` Winkel, `O` Deckkraft, `B` Schwärze, `R` Streuung, `H` Farbton, `L` Helligkeit,
/// `S` Sättigung. Wert in Promille (intern Byte 0..128, Datei v·1000/128).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sensitivity {
    pub source: u8,
    pub target: u8,
    pub value: i32,
}

/// Gültige Quellen- und Zielbuchstaben der Empfindlichkeiten.
pub const SENSITIVITY_SOURCES: &[u8; 5] = b"PSHVR";
pub const SENSITIVITY_TARGETS: &[u8; 8] = b"ZAOBRHLS";

impl Brush {
    /// Einfacher harter Strich („Pencil 1-Pixel Hard“-ähnlich) mit Original-Vorgaben für `diameter`.
    pub fn basic(diameter: i32) -> Brush {
        Brush {
            category: "bc_Pencil".into(),
            name: "bn_1-Pixel Hard".into(),
            user_name: None,
            read_only: false,
            angle: 0,
            aspect: 1000,
            diameter,
            max_count: 0,
            min_size: 0,
            softness: 0,
            soften_mode: 1,
            shape: 1,
            bbl: 0,
            blackness: 0,
            concentration: 1000,
            alpha_remap: 0,
            brush_type: 1,
            feedback: 0,
            flow_rate: 0,
            tip_count: 1,
            spacing: 150,
            texture_blend: 0,
            texture_edge: 0,
            tip_spacing: 0,
            tip_spacing_mode: 0,
            tip_coloring_mode: 0,
            antialiased: false,
            dash_count: 0,
            dash_on: [0; 3],
            dash_off: [0; 3],
            sensitivities: Vec::new(),
            extra: Extra::default(),
        }
    }

    /// Wert einer Empfindlichkeit (0, wenn nicht gesetzt).
    pub fn sensitivity(&self, source: u8, target: u8) -> i32 {
        self.sensitivities
            .iter()
            .find(|s| s.source == source && s.target == target)
            .map(|s| s.value)
            .unwrap_or(0)
    }
}

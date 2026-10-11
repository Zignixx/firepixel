//! Symbole (`MSTv` in `MKBv/MSLv`, Spezifikation 13): Grafik, Schaltfläche, Animation, „widget“;
//! 9-Slice, Rich-Symbol-Skript, Bibliotheksverknüpfung, Schaltflächen-Optionen.

use crate::doc::LayerRoot;
use crate::element::ButtonStrings;
use crate::export::ExportSettings;
use crate::ids::SymbolId;
use crate::raw::Extra;
use crate::serde_ext;
use crate::web::Behaviors;
use fp_color::Argb;
use fp_geom::Rect;
use serde::{Deserialize, Serialize};

/// Art eines Symbols (`MSYi`): fehlt/0 Grafik, 1 Schaltfläche, 2 Animation, 3 „widget“.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolKind {
    Graphic,
    Button,
    Animation,
    /// Nur über `dom.addNewSymbol("widget", …)` (B, Laufzeit 13 T7).
    Widget,
    Other(i32),
}

impl SymbolKind {
    pub fn from_code(v: Option<i32>) -> SymbolKind {
        match v {
            None | Some(0) => SymbolKind::Graphic,
            Some(1) => SymbolKind::Button,
            Some(2) => SymbolKind::Animation,
            Some(3) => SymbolKind::Widget,
            Some(o) => SymbolKind::Other(o),
        }
    }
    pub fn code(self) -> i32 {
        match self {
            SymbolKind::Graphic => 0,
            SymbolKind::Button => 1,
            SymbolKind::Animation => 2,
            SymbolKind::Widget => 3,
            SymbolKind::Other(o) => o,
        }
    }
    /// JS-Name (`graphic`, `button`, `animation`, `widget`).
    pub fn js_name(self) -> &'static str {
        match self {
            SymbolKind::Graphic | SymbolKind::Other(_) => "graphic",
            SymbolKind::Button => "button",
            SymbolKind::Animation => "animation",
            SymbolKind::Widget => "widget",
        }
    }
}

/// Symboldefinition (im Original `CDbMaster : CDbRootElem`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Symbol {
    /// Name (`UNMs`).
    pub name: String,
    /// Symbol-ID (`MSNi`), Ziel der Instanz-Verweise.
    pub id: SymbolId,
    /// Art (`MSYi`; `None` = Tag fehlt = Grafik).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind_code: Option<i32>,
    /// Ursprung (`MOXf MOYf`, immer 0).
    pub origin_x: f32,
    pub origin_y: f32,
    /// Bibliotheksquelle (`LFNs`, `LMNi`, `LUTs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub library_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub library_symbol: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,
    /// Eigene Ebenen, Status, Raster, Hilfslinien, Zwiebelschicht, Leinwandfarbe.
    pub root: LayerRoot,
    /// 9-Slice (`NGSv`; vorhanden = eingeschaltet).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nine_slice: Option<NineSlice>,
    /// Rich-Symbol-Skript (`WJSs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub widget_script: Option<String>,
    /// Schaltflächen-Optionen (`ATSb IDSb DVLb IODb`).
    pub button_options: ButtonOptions,
    /// Schaltflächen-Zeichenketten (`INMs A2Ts ALTs FILs`; `BUTs` kommt nicht vor).
    #[serde(default)]
    pub button: ButtonStrings,
    /// Statusleistentext der Schaltfläche (`MSGs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_message: Option<String>,
    /// Verhalten (`BEVv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub behaviors: Option<Behaviors>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

impl Symbol {
    pub fn kind(&self) -> SymbolKind {
        SymbolKind::from_code(self.kind_code)
    }

    /// Ist es ein Rich Symbol (Grafiksymbol mit Skript)?
    pub fn is_rich(&self) -> bool {
        self.widget_script.is_some()
    }

    /// Exporteinstellungen des Symbols (selten).
    pub fn export(&self) -> Option<&ExportSettings> {
        self.root.export.as_deref()
    }

    /// Gezeigter Symbolstatus für einen Seitenstatus (13 §2.3): Animation `(f − start) mod FRC`,
    /// sonst 0.
    pub fn shown_frame(&self, page_frame: usize, start_frame: usize) -> usize {
        if self.kind() == SymbolKind::Animation {
            let frc = self.root.frame_count.max(1) as usize;
            page_frame.saturating_sub(start_frame) % frc
        } else {
            0
        }
    }
}

/// Schaltflächen-Optionen (13 §7.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ButtonOptions {
    /// Auto-Slice (`ATSb`, Vorgabe 1).
    pub auto_slice: bool,
    /// Down-Status einschließen (`IDSb`).
    pub include_down: bool,
    /// Down beim Laden zeigen (`DVLb`).
    pub show_down_on_load: bool,
    /// Over-While-Down einschließen (`IODb`).
    pub include_over_down: bool,
}

impl Default for ButtonOptions {
    fn default() -> Self {
        ButtonOptions { auto_slice: true, include_down: false, show_down_on_load: false, include_over_down: false }
    }
}

/// 9-Slice-Hilfslinien (`NGSv`, 13 §3.1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NineSlice {
    /// Gesperrt (`NGLb`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
    /// Farbe (`NGKi`).
    #[serde(default, skip_serializing_if = "Option::is_none", with = "serde_ext::opt_argb")]
    pub color: Option<Argb>,
    /// Hilfslinien (`NGCf` ×4): oben, unten, links, rechts.
    pub guides: Vec<f64>,
    /// Bezugsrechteck = Symbolgrenzen (`NGMv`).
    #[serde(default, skip_serializing_if = "Option::is_none", with = "serde_ext::opt_rect")]
    pub bounds: Option<Rect>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub bounds_extra: Extra,
    /// Zeichenbereich der Hilfslinien (`NGVv`).
    #[serde(default, skip_serializing_if = "Option::is_none", with = "serde_ext::opt_rect")]
    pub view: Option<Rect>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub view_extra: Extra,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

impl NineSlice {
    /// Hilfslinien (oben, unten, links, rechts), auf ganze Pixel gerundet (halb weg von null, 13 §7.6).
    pub fn rounded_guides(&self) -> Option<[f64; 4]> {
        if self.guides.len() < 4 {
            return None;
        }
        Some([self.guides[0].round(), self.guides[1].round(), self.guides[2].round(), self.guides[3].round()])
    }
}

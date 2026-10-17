//! Stile (`ZSTv` in `MKBv/ZSLv`, Spezifikationen 02 §5.8, 09 §3.6).

use crate::attrs::{FillHandles, PathAttrs};
use crate::effects::EffectList;
use crate::element::BitmapNode;
use crate::raw::Extra;
use crate::text::TextAttrs;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Ein Stil. Elemente verweisen per Namen (`ZSNs`) darauf.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Style {
    /// Name (`ZSNs`).
    pub name: String,
    /// Attributgruppen (`ZSFi`): Bit 0 Pinsel, 1 Pinselfarbe, 2 Füllung, 3 Füllfarbe, 4–8 ? (0x1FF = alle).
    pub flags: u32,
    /// Weitere Gruppen (`ZSOi`): Bit 0 Schrift, 1 Größe, 2 Schriftstil, 3 Effekte.
    pub other_flags: u32,
    /// Strich/Füllung (`PATv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attrs: Option<Arc<PathAttrs>>,
    /// Füll-Handles (`PSXf…PFYf`, ohne `PCFb`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_handles: Option<FillHandles>,
    /// Strichposition (`BRPi`), Füllung über Strich (`FOTb`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_placement: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_on_top: Option<bool>,
    /// Vorschaurahmen (`FALi FATi FARi FABi`, vorzeichenbehaftet).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_rect: Option<[i32; 4]>,
    /// Effekte (`EFLv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effects: Option<EffectList>,
    /// Textattribute (`FONs CFTs PTSf BOLb ITLb UNDb JSTi HSCf RKNf LEDf LDMi`).
    #[serde(default)]
    pub text: TextAttrs,
    /// `ATKb`, `TIAb`, `TAAi`, `TA1i`, `TA2i`, `TOSi`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_kern: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tia: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub antialias: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aa_sharpness: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aa_strength: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aa_oversample: Option<i32>,
    /// Vorschaubilder (`IMGv`, meist zwei: klein und groß).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub previews: Vec<Arc<BitmapNode>>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

impl Style {
    /// Leerer Stil mit Namen (alle Attributgruppen aktiv wie bei neuen Original-Stilen).
    pub fn new(name: &str) -> Style {
        Style {
            name: name.to_string(),
            flags: 0x1FF,
            other_flags: 0x1F,
            attrs: None,
            fill_handles: None,
            stroke_placement: None,
            fill_on_top: None,
            preview_rect: None,
            effects: None,
            text: TextAttrs::default(),
            auto_kern: None,
            tia: None,
            antialias: None,
            aa_sharpness: None,
            aa_strength: None,
            aa_oversample: None,
            previews: Vec::new(),
            extra: Extra::default(),
        }
    }

    pub fn use_brush(&self) -> bool {
        self.flags & 1 != 0
    }
    pub fn use_brush_color(&self) -> bool {
        self.flags & 2 != 0
    }
    pub fn use_fill(&self) -> bool {
        self.flags & 4 != 0
    }
    pub fn use_fill_color(&self) -> bool {
        self.flags & 8 != 0
    }
    pub fn use_text_font(&self) -> bool {
        self.other_flags & 1 != 0
    }
    pub fn use_text_size(&self) -> bool {
        self.other_flags & 2 != 0
    }
    pub fn use_text_styles(&self) -> bool {
        self.other_flags & 4 != 0
    }
    pub fn use_effects(&self) -> bool {
        self.other_flags & 8 != 0
    }
}

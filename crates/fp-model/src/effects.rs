//! Live-Effekte (`EFLv`, Spezifikationen 02 §7, 05, 03 §6).
//!
//! Aufbau: `EFLv{CATs INMs [UNMs] EPSv{DCEv{DCKs, DCVs | GPLv | GVRv}…}…}`. An Elementen ist
//! `CATs`/`INMs` = `"UNUSED"`; in Vorgabelisten (`EFDv`) bzw. Stilen tragen sie Kategorie und Namen.
//! Jedes `EPSv` ist ein Effekt; der Typ steht im Parameter `EffectMoaID`, die Sichtbarkeit in
//! `EffectIsVisible`. Alle Werte sind Strings (Farben `#RRGGBB[AA]`), Kurven sind Punktlisten.

use crate::data::{DataDict, DataEntry, DataValue};
use crate::raw::Extra;
use serde::{Deserialize, Serialize};

/// Variante des Effekt-Containers: `EPSv` (üblich) oder `EPDv` (vom Leser akzeptiert, 02 §7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum EffectTag {
    #[default]
    Eps,
    Epd,
}

/// Eine Effektliste.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EffectList {
    /// `CATs` (an Elementen `"UNUSED"`).
    pub category: String,
    /// `INMs` (an Elementen `"UNUSED"`).
    pub name: String,
    /// `UNMs` (Anzeigename, selten).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
    /// Effekte in Anwendungsreihenfolge.
    pub effects: Vec<Effect>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

impl Default for EffectList {
    fn default() -> Self {
        EffectList {
            category: "UNUSED".to_string(),
            name: "UNUSED".to_string(),
            user_name: None,
            effects: Vec::new(),
            extra: Extra::default(),
        }
    }
}

impl EffectList {
    /// Gibt es mindestens einen sichtbaren Effekt?
    pub fn has_visible(&self) -> bool {
        self.effects.iter().any(|e| e.is_visible())
    }
}

/// Ein Live-Effekt: geordnete Parameter.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Effect {
    #[serde(default, skip_serializing_if = "is_eps")]
    pub tag: EffectTag,
    pub params: DataDict,
}

fn is_eps(t: &EffectTag) -> bool {
    *t == EffectTag::Eps
}

/// Bekannte Effektarten (Tabelle `effect_moa_ids` aus `api/fw_api.json`, 05/06).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EffectKind {
    /// Abschrägung/Prägung/Glühen; Variante über `BevelType` (0 innere, 1 äußere Abschrägung,
    /// 2 erhabene, 3 vertiefte Prägung, 4 Glühen).
    BevelFamily,
    DropShadow,
    InnerShadow,
    InnerGlow,
    SolidShadow,
    ColorFill,
    /// Photoshop-Live-Effekte (Container für `dropShadow`, `innerShadow`, … als `&`-Datensätze).
    PhotoshopLive,
    GaussianBlur,
    Blur,
    BlurMore,
    MotionBlur,
    RadialBlur,
    ZoomBlur,
    Sharpen,
    SharpenMore,
    UnsharpMask,
    AddNoise,
    BrightnessContrast,
    HueSaturation,
    Curves,
    Levels,
    AutoLevels,
    Invert,
    FindEdges,
    ConvertToAlpha,
    /// Überbleibsel ohne Code in CS6 (`sky_color`/`cloud_color`).
    Clouds,
    DifferenceClouds,
    SimpleBlur,
    /// Photoshop-Plugin-Filter (`{d810e821-f86b-11d0-…}`), letzte 8 Byte = zwei ASCII-Vierercodes.
    PhotoshopPlugin,
    /// Screentone / halftone screen (Firepixel extension, FB-38; EffectMoaID
    /// [`SCREENTONE_MOA_ID`], stored in FW-PNG in the Firepixel marker `fpLv`).
    Screentone,
    /// Unbekannte ID.
    Unknown,
}

/// Tabelle MOA-ID → Effektart (Kleinschreibung wie in den Dateien).
pub const EFFECT_MOA_IDS: &[(&str, EffectKind)] = &[
    (
        "{7fe61102-6ce2-11d1-8c76000502701850}",
        EffectKind::BevelFamily,
    ),
    (
        "{a7944db8-6ce2-11d1-8c76000502701850}",
        EffectKind::DropShadow,
    ),
    (
        "{5600f702-774c-11d3-baad0000861f4d01}",
        EffectKind::InnerShadow,
    ),
    (
        "{2ba87123-8220-11d3-baad0000861f4d01}",
        EffectKind::InnerGlow,
    ),
    (
        "{b90c950e-64df-11d8-aaf2000a9582f7d4}",
        EffectKind::SolidShadow,
    ),
    (
        "{dd54adc0-a279-11d3-b92a000502f3fdbe}",
        EffectKind::ColorFill,
    ),
    (
        "{8eeadf50-9efe-11da-a7460800200c9a66}",
        EffectKind::PhotoshopLive,
    ),
    (
        "{d04ef8c0-71b3-11d1-8c8200a024cdc039}",
        EffectKind::GaussianBlur,
    ),
    ("{f1cfce41-718e-11d1-8c8200a024cdc039}", EffectKind::Blur),
    (
        "{f1cfce42-718e-11d1-8c8200a024cdc039}",
        EffectKind::BlurMore,
    ),
    (
        "{538016b6-ffd6-418f-a10c590e6ee841ae}",
        EffectKind::MotionBlur,
    ),
    (
        "{d1c33142-2ad8-4215-a39fc5d934c8fc0c}",
        EffectKind::RadialBlur,
    ),
    (
        "{94dbd663-9b2e-44bf-8064ab6efdd2d327}",
        EffectKind::ZoomBlur,
    ),
    ("{c20952b1-fc76-11d0-8be700a024cdc039}", EffectKind::Sharpen),
    (
        "{1f2f2591-9db7-11d1-8cad00a024cdc039}",
        EffectKind::SharpenMore,
    ),
    (
        "{f1cfce44-718e-11d1-8c8200a024cdc039}",
        EffectKind::UnsharpMask,
    ),
    (
        "{e4c0f4bc-c0a3-4cb3-b3513822027e4d9f}",
        EffectKind::AddNoise,
    ),
    (
        "{3439b08c-1921-11d3-9bde00e02910d580}",
        EffectKind::BrightnessContrast,
    ),
    (
        "{3439b08d-1922-11d3-9bde00e02910d580}",
        EffectKind::HueSaturation,
    ),
    ("{3439b08e-1923-11d3-9bde00e02910d580}", EffectKind::Curves),
    ("{d04ef8c2-71b5-11d1-8c8200a024cdc039}", EffectKind::Levels),
    (
        "{d04ef8c1-71b4-11d1-8c8200a024cdc039}",
        EffectKind::AutoLevels,
    ),
    ("{d2541291-70d6-11d1-8c8000a024cdc039}", EffectKind::Invert),
    (
        "{fc7093f1-f95c-11d0-8be200a024cdc039}",
        EffectKind::FindEdges,
    ),
    (
        "{2932d5a2-ca48-11d1-8561000502701850}",
        EffectKind::ConvertToAlpha,
    ),
    ("{ff84d00c-4494-11d8-a072000a9582f7d4}", EffectKind::Clouds),
    (
        "{d2e7769c-6349-11d8-b0e9000a9595f34e}",
        EffectKind::DifferenceClouds,
    ),
    (
        "{a2dea491-fd45-11d0-8be900a024cdc039}",
        EffectKind::SimpleBlur,
    ),
];

/// Präfix der Photoshop-Plugin-IDs.
pub const PLUGIN_MOA_PREFIX: &str = "{d810e821-f86b-11d0-";

/// Prefix of the EffectMoaIDs of Firepixel's own live filters (FB-38). Effects with this prefix
/// are Firepixel extensions: FW-PNG stores them in the Firepixel marker tag `fpLv` instead of an
/// `EPSv`, so files of the reference format never carry them.
pub const FIREPIXEL_MOA_PREFIX: &str = "{firepixel-";

/// EffectMoaID of the Screentone live filter (Firepixel extension, FB-38).
pub const SCREENTONE_MOA_ID: &str = "{firepixel-screentone}";

impl EffectKind {
    /// Effektart zu einer MOA-ID (Groß-/Kleinschreibung egal).
    pub fn from_moa_id(id: &str) -> EffectKind {
        let l = id.to_ascii_lowercase();
        if let Some((_, k)) = EFFECT_MOA_IDS.iter().find(|(m, _)| *m == l) {
            return *k;
        }
        if l.starts_with(PLUGIN_MOA_PREFIX) {
            return EffectKind::PhotoshopPlugin;
        }
        if l == SCREENTONE_MOA_ID {
            return EffectKind::Screentone;
        }
        EffectKind::Unknown
    }

    /// MOA-ID der Effektart (nicht für Plugin/Unbekannt).
    pub fn moa_id(self) -> Option<&'static str> {
        if self == EffectKind::Screentone {
            return Some(SCREENTONE_MOA_ID);
        }
        EFFECT_MOA_IDS
            .iter()
            .find(|(_, k)| *k == self)
            .map(|(m, _)| *m)
    }
}

impl Effect {
    /// Neuer Effekt mit MOA-ID und Sichtbarkeit (Reihenfolge wie das Original: Sichtbarkeit, ID, Parameter).
    pub fn new(moa_id: &str, visible: bool) -> Effect {
        let mut p = DataDict::default();
        p.entries.push(DataEntry::scalar(
            "EffectIsVisible",
            if visible { "true" } else { "false" },
        ));
        p.entries.push(DataEntry::scalar("EffectMoaID", moa_id));
        Effect {
            tag: EffectTag::Eps,
            params: p,
        }
    }

    /// `EffectMoaID`.
    pub fn moa_id(&self) -> Option<&str> {
        self.params.get_str("EffectMoaID")
    }

    /// Is it one of Firepixel's own live filters (EffectMoaID with [`FIREPIXEL_MOA_PREFIX`],
    /// FB-38)?
    pub fn is_firepixel(&self) -> bool {
        self.moa_id().is_some_and(|m| {
            m.trim()
                .to_ascii_lowercase()
                .starts_with(FIREPIXEL_MOA_PREFIX)
        })
    }

    /// Effektart aus der MOA-ID.
    pub fn kind(&self) -> EffectKind {
        self.moa_id()
            .map(EffectKind::from_moa_id)
            .unwrap_or(EffectKind::Unknown)
    }

    /// `EffectIsVisible` (fehlt = sichtbar, nur `"false"` schaltet aus).
    pub fn is_visible(&self) -> bool {
        self.params.get_str("EffectIsVisible") != Some("false")
    }

    /// Schaltet die Sichtbarkeit.
    pub fn set_visible(&mut self, v: bool) {
        self.params
            .set_scalar("EffectIsVisible", if v { "true" } else { "false" });
    }

    /// Schlüssel des Anzeigenamens in fp-i18n (`filter.ftl`, `effect-…` bzw. `filter-…`), z. B.
    /// `effect-drop-shadow`; `None` für Plugins und unbekannte Effekte (dann [`Effect::ui_name`]).
    pub fn label_key(&self) -> Option<&'static str> {
        Some(match self.kind() {
            EffectKind::BevelFamily => match self.get_f64("BevelType").unwrap_or(0.0) as i32 {
                1 => "effect-bevel-outer",
                2 => "effect-emboss-raised",
                3 => "effect-emboss-inset",
                4 => "effect-glow",
                _ => "effect-bevel-inner",
            },
            EffectKind::DropShadow => "effect-drop-shadow",
            EffectKind::InnerShadow => "effect-inner-shadow",
            EffectKind::InnerGlow => "effect-inner-glow",
            EffectKind::SolidShadow => "effect-solid-shadow",
            EffectKind::ColorFill => "effect-color-fill",
            EffectKind::PhotoshopLive => "effect-photoshop-live",
            EffectKind::GaussianBlur => "filter-blur-gaussian",
            EffectKind::Blur => "filter-blur-blur",
            EffectKind::BlurMore => "filter-blur-blurMore",
            EffectKind::MotionBlur => "filter-blur-motion",
            EffectKind::RadialBlur => "filter-blur-radial",
            EffectKind::ZoomBlur => "filter-blur-zoom",
            EffectKind::Sharpen => "filter-sharpen-sharpen",
            EffectKind::SharpenMore => "filter-sharpen-sharpenMore",
            EffectKind::UnsharpMask => "filter-sharpen-unsharpMask",
            EffectKind::AddNoise => "filter-noise-addNoise",
            EffectKind::BrightnessContrast => "filter-adjustColor-brightnessContrast",
            EffectKind::HueSaturation => "filter-adjustColor-hueSaturation",
            EffectKind::Curves => "filter-adjustColor-curves",
            EffectKind::Levels => "filter-adjustColor-levels",
            EffectKind::AutoLevels => "filter-adjustColor-autoLevels",
            EffectKind::Invert => "filter-adjustColor-invert",
            EffectKind::FindEdges => "filter-other-findEdges",
            EffectKind::ConvertToAlpha => "filter-other-convertToAlpha",
            EffectKind::Screentone => "effect-screentone",
            _ => return None,
        })
    }

    /// Anzeigename, den das Original nur schreibt (`mkbFile_WriteOnly_TemporaryEffectUiName`) bzw. `name`.
    pub fn ui_name(&self) -> Option<&str> {
        self.params
            .get_str("mkbFile_WriteOnly_TemporaryEffectUiName")
            .or_else(|| self.params.get_str("name"))
    }

    /// Parameter als Text.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.params.get_str(key)
    }

    /// Parameter als Zahl.
    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.get(key)?.trim().parse().ok()
    }

    /// Parameter als Wahrheitswert (`true`/`false`, `1`/`0`).
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        match self.get(key)?.trim() {
            "true" | "1" => Some(true),
            "false" | "0" => Some(false),
            _ => None,
        }
    }

    /// Farbparameter `#RRGGBB` oder `#RRGGBBAA` als ARGB.
    pub fn get_color(&self, key: &str) -> Option<fp_color::Argb> {
        parse_hex_color(self.get(key)?)
    }

    /// Punktliste (Kurven).
    pub fn get_points(&self, key: &str) -> Option<&[fp_geom::Point]> {
        match &self.params.get(key)?.value {
            DataValue::Points(p) => Some(&p.points),
            _ => None,
        }
    }

    /// Setzt einen Textparameter.
    pub fn set(&mut self, key: &str, value: impl Into<String>) {
        self.params.set_scalar(key, value);
    }
}

/// `#RRGGBB` bzw. `#RRGGBBAA` (Alpha zuletzt, 03 §6) → ARGB.
pub fn parse_hex_color(s: &str) -> Option<fp_color::Argb> {
    let h = s.trim().strip_prefix('#')?;
    let v = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
    match h.len() {
        6 => Some(fp_color::Argb(
            0xff00_0000 | (v(0)? as u32) << 16 | (v(2)? as u32) << 8 | v(4)? as u32,
        )),
        8 => Some(fp_color::Argb(
            (v(6)? as u32) << 24 | (v(0)? as u32) << 16 | (v(2)? as u32) << 8 | v(4)? as u32,
        )),
        _ => None,
    }
}

/// ARGB → `#rrggbb` (deckend) bzw. `#rrggbbaa`.
pub fn format_hex_color(c: fp_color::Argb) -> String {
    if c.a() == 255 {
        format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())
    } else {
        format!("#{:02x}{:02x}{:02x}{:02x}", c.r(), c.g(), c.b(), c.a())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_and_params() {
        let mut e = Effect::new("{A7944DB8-6CE2-11D1-8C76000502701850}", false);
        assert_eq!(e.kind(), EffectKind::DropShadow);
        assert!(!e.is_visible());
        e.set("ShadowColor", "#ffffffa6");
        e.set("ShadowBlur", "4");
        assert_eq!(e.get_color("ShadowColor"), Some(fp_color::Argb(0xa6ffffff)));
        assert_eq!(e.get_f64("ShadowBlur"), Some(4.0));
        e.set_visible(true);
        assert!(e.is_visible());
        assert_eq!(
            EffectKind::from_moa_id("{d810e821-f86b-11d0-466c747252646c42}"),
            EffectKind::PhotoshopPlugin
        );
        assert_eq!(
            EffectKind::Curves.moa_id(),
            Some("{3439b08e-1923-11d3-9bde00e02910d580}")
        );
        assert_eq!(format_hex_color(fp_color::Argb(0xff0000ff)), "#0000ff");
        assert_eq!(parse_hex_color("#0000ff"), Some(fp_color::Argb(0xff0000ff)));
    }
}

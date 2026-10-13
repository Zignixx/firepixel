//! Voreinstellungen (Spezifikation 14 §3, §4).
//!
//! Die 644 Voreinstellungen des Originals (`PREF/1001` + `PREF/1002`) liegen mit Typ und Vorgabe in
//! `data/prefs.json` (erzeugt aus `re\farbe\voreinstellungen.json`). [`Preferences`] speichert nur die
//! **Abweichungen** von den Vorgaben sowie Laufzeitschlüssel (`active tool for slot …`,
//! `DisableConfirm …`) und eigene Firepixel-Einstellungen (Design, zuletzt geöffnete Dateien).
//!
//! Gespeichert wird als JSON in einer eigenen Datei im Benutzerprofil
//! (`%APPDATA%\Firepixel\preferences.json`, siehe [`Preferences::default_path`]), nicht in der
//! Datei des Originals oder der Registry. Gelesen wird typtolerant wie im Original (14 §3.1 „Typtreue“):
//! `bool ↔ int ↔ real`, Farben als `#rrggbb[aa]`.

use crate::error::{EngineError, Result};
use fp_color::{Argb, Rgba8};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Wert einer Voreinstellung.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PrefValue {
    Bool(bool),
    Int(i64),
    Real(f64),
    Str(String),
    List(Vec<PrefValue>),
    Dict(BTreeMap<String, PrefValue>),
}

impl PrefValue {
    /// Als Wahrheitswert (Zahlen ≠ 0 sind wahr, `"true"`/`"1"`).
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            PrefValue::Bool(b) => Some(*b),
            PrefValue::Int(i) => Some(*i != 0),
            PrefValue::Real(r) => Some(*r != 0.0),
            PrefValue::Str(s) => match s.trim() {
                "true" | "1" => Some(true),
                "false" | "0" => Some(false),
                _ => None,
            },
            _ => None,
        }
    }

    /// Als Ganzzahl (Realzahlen werden abgeschnitten, `true` = 1).
    pub fn as_int(&self) -> Option<i64> {
        match self {
            PrefValue::Bool(b) => Some(*b as i64),
            PrefValue::Int(i) => Some(*i),
            PrefValue::Real(r) if r.is_finite() => Some(r.trunc() as i64),
            PrefValue::Str(s) => s.trim().parse::<f64>().ok().filter(|v| v.is_finite()).map(|v| v.trunc() as i64),
            _ => None,
        }
    }

    /// Als Realzahl.
    pub fn as_real(&self) -> Option<f64> {
        match self {
            PrefValue::Bool(b) => Some(*b as i64 as f64),
            PrefValue::Int(i) => Some(*i as f64),
            PrefValue::Real(r) => Some(*r),
            PrefValue::Str(s) => s.trim().parse().ok(),
            _ => None,
        }
    }

    /// Als Text (Zahlen werden formatiert).
    pub fn as_str(&self) -> Option<String> {
        match self {
            PrefValue::Str(s) => Some(s.clone()),
            PrefValue::Bool(b) => Some(b.to_string()),
            PrefValue::Int(i) => Some(i.to_string()),
            PrefValue::Real(r) => Some(r.to_string()),
            _ => None,
        }
    }

    /// Als Farbe (`#rrggbb` bzw. `#rrggbbaa`).
    pub fn as_color(&self) -> Option<Argb> {
        match self {
            PrefValue::Str(s) => Rgba8::parse_hex(s).map(|c| c.to_argb()),
            PrefValue::Int(i) => Some(Argb(*i as u32)),
            _ => None,
        }
    }

    /// Aus einem JSON-Wert (Objekte → Dict, Arrays → List, `null` → `None`).
    pub fn from_json(v: &serde_json::Value) -> Option<PrefValue> {
        serde_json::from_value(v.clone()).ok()
    }

    /// Als JSON-Wert.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

/// Typ einer Voreinstellung (wie in `PREF/1001`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrefType {
    Bool,
    Integer,
    Real,
    String,
    Dict,
}

/// Definition einer Voreinstellung mit Vorgabe des Originals.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PrefDef {
    /// Schlüssel (z. B. `MaxUndos`).
    pub key: String,
    /// Typ der Vorgabe.
    #[serde(rename = "type")]
    pub kind: PrefType,
    /// Vorgabewert.
    pub default: PrefValue,
    /// Kategorie (z. B. „Dialog: Allgemein“).
    #[serde(default)]
    pub category: Option<String>,
    /// Wirkung (Kurzbeschreibung aus 14).
    #[serde(default)]
    pub effect: Option<String>,
    /// Seite im Voreinstellungsdialog.
    #[serde(default)]
    pub dialog_page: Option<String>,
}

#[derive(Deserialize)]
struct PrefFile {
    prefs: Vec<PrefDef>,
}

static DEFS: OnceLock<Vec<PrefDef>> = OnceLock::new();

/// Alle 644 Voreinstellungen mit den Vorgaben des Originals.
pub fn definitions() -> &'static [PrefDef] {
    DEFS.get_or_init(|| {
        serde_json::from_str::<PrefFile>(include_str!("../data/prefs.json")).map(|f| f.prefs).unwrap_or_default()
    })
}

/// Definition zu einem Schlüssel.
pub fn definition(key: &str) -> Option<&'static PrefDef> {
    definitions().iter().find(|d| d.key == key)
}

/// Design der Oberfläche (ARCHITEKTUR §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Hellgrau wie das Original.
    #[default]
    Normal,
    /// Dunkles Design.
    Dark,
    /// Reines Schwarz, farbenfroh.
    Oled,
}

impl Theme {
    /// Deutsche Bezeichnung für Menü und Dialog.
    pub fn label_de(self) -> &'static str {
        match self {
            Theme::Normal => "Normal",
            Theme::Dark => "Dunkel",
            Theme::Oled => "OLED",
        }
    }

    /// Aus Namen (`normal`/`dark`/`oled` oder deutsch).
    pub fn from_name(s: &str) -> Option<Theme> {
        match s.trim().to_ascii_lowercase().as_str() {
            "normal" => Some(Theme::Normal),
            "dark" | "dunkel" => Some(Theme::Dark),
            "oled" => Some(Theme::Oled),
            _ => None,
        }
    }
}

/// Leinwand eines neuen Dokuments (`DefaultDocColorState`, 14 §3.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CanvasDefault {
    White,
    Transparent,
    Custom,
}

impl CanvasDefault {
    /// Aus dem Voreinstellungswert: 1 Weiß, 2 Transparent, 3 Benutzerdefiniert, sonst Weiß.
    pub fn from_code(v: i64) -> CanvasDefault {
        match v {
            2 => CanvasDefault::Transparent,
            3 => CanvasDefault::Custom,
            _ => CanvasDefault::White,
        }
    }

    /// Zahlenwert der Voreinstellung.
    pub fn code(self) -> i64 {
        match self {
            CanvasDefault::White => 1,
            CanvasDefault::Transparent => 2,
            CanvasDefault::Custom => 3,
        }
    }
}

/// Längeneinheit (14 §4, Codes `pixels` = 0, `inch` = 1, `cm` = 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Unit {
    Pixels,
    Inch,
    Cm,
}

impl Unit {
    /// Aus dem Textnamen (`pixels`, `inch`, `cm`).
    pub fn from_name(s: &str) -> Option<Unit> {
        match s.trim().to_ascii_lowercase().as_str() {
            "pixels" | "pixel" | "px" => Some(Unit::Pixels),
            "inch" | "in" | "zoll" => Some(Unit::Inch),
            "cm" => Some(Unit::Cm),
            _ => None,
        }
    }

    /// Textname wie in den Voreinstellungen.
    pub fn name(self) -> &'static str {
        match self {
            Unit::Pixels => "pixels",
            Unit::Inch => "inch",
            Unit::Cm => "cm",
        }
    }

    /// Wert in Pixel umrechnen (Auflösung in Pixel/Zoll).
    pub fn to_pixels(self, v: f64, ppi: f64) -> f64 {
        match self {
            Unit::Pixels => v,
            Unit::Inch => v * ppi,
            Unit::Cm => v / 2.54 * ppi,
        }
    }

    /// Pixel in diese Einheit umrechnen.
    pub fn from_pixels(self, px: f64, ppi: f64) -> f64 {
        if ppi == 0.0 {
            return px;
        }
        match self {
            Unit::Pixels => px,
            Unit::Inch => px / ppi,
            Unit::Cm => px / ppi * 2.54,
        }
    }
}

/// Auflösung `{wert; einheit}` (0 = pro Zoll, 1 = pro cm).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Resolution {
    pub value: f32,
    pub per_cm: bool,
}

impl Resolution {
    /// In Pixel/Zoll (ppcm × 2,54).
    pub fn ppi(&self) -> f64 {
        if self.per_cm { self.value as f64 * 2.54 } else { self.value as f64 }
    }
}

/// Höchstzahl gemerkter zuletzt geöffneter Dateien (MFC `LoadStdProfileSettings(10)`).
pub const MAX_RECENT_FILES: usize = 10;

/// Voreinstellungen von Firepixel.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Preferences {
    /// Formatversion der Datei.
    #[serde(default)]
    pub version: u32,
    /// Design der Oberfläche.
    #[serde(default)]
    pub theme: Theme,
    /// Zuletzt geöffnete Dateien (neueste zuerst, höchstens [`MAX_RECENT_FILES`]).
    #[serde(default)]
    pub recent_files: Vec<PathBuf>,
    /// Abweichungen von den Vorgaben des Originals und Laufzeitschlüssel.
    #[serde(default)]
    pub values: BTreeMap<String, PrefValue>,
}

/// Zuletzt per OK bestätigte Werte der Filterdialoge (Menü „Filter“): app-weit gemerkt, aber nur
/// für die Sitzung. Das Original legt sie nicht in seiner Voreinstellungsdatei ab und öffnet nach
/// einem Neustart wieder mit den Vorgaben (z. B. Rauschen 50, TW-09); darum schreibt
/// [`Preferences::save`] sie nicht mit.
pub const SESSION_ONLY_KEYS: &[&str] = &[
    "GaussianBlurRadius",
    "UnsharpMaskAmount",
    "UnsharpMaskRadius",
    "UnsharpMaskThreshold",
    "BrightnessContrastBrightness",
    "BrightnessContrastContrast",
    "MotionBlurAngle",
    "MotionBlurDistance",
    "HueSaturationHue",
    "HueSaturationSaturation",
    "HueSaturationLightness",
    "HueSaturationColorize",
    "AddNoiseAmount",
    "AddNoiseColor",
    "RadialBlurAmount",
    "RadialBlurQuality",
    "ZoomBlurAmount",
    "ZoomBlurQuality",
];

impl Preferences {
    /// Vorgaben (keine Abweichungen).
    pub fn new() -> Preferences {
        Preferences { version: 1, ..Preferences::default() }
    }

    /// Wirksamer Wert (gespeichert oder Vorgabe).
    pub fn get(&self, key: &str) -> Option<PrefValue> {
        self.values.get(key).cloned().or_else(|| definition(key).map(|d| d.default.clone()))
    }

    /// Wahrheitswert mit Rückfall.
    pub fn get_bool(&self, key: &str, fallback: bool) -> bool {
        self.get(key).and_then(|v| v.as_bool()).unwrap_or(fallback)
    }

    /// Ganzzahl mit Rückfall.
    pub fn get_int(&self, key: &str, fallback: i64) -> i64 {
        self.get(key).and_then(|v| v.as_int()).unwrap_or(fallback)
    }

    /// Realzahl mit Rückfall.
    pub fn get_real(&self, key: &str, fallback: f64) -> f64 {
        self.get(key).and_then(|v| v.as_real()).unwrap_or(fallback)
    }

    /// Text mit Rückfall.
    pub fn get_str(&self, key: &str, fallback: &str) -> String {
        self.get(key).and_then(|v| v.as_str()).unwrap_or_else(|| fallback.to_string())
    }

    /// Farbe mit Rückfall.
    pub fn get_color(&self, key: &str, fallback: Argb) -> Argb {
        self.get(key).and_then(|v| v.as_color()).unwrap_or(fallback)
    }

    /// Setzt einen Wert. Gleicht er der Vorgabe, wird die Abweichung entfernt.
    pub fn set(&mut self, key: &str, value: PrefValue) {
        if definition(key).is_some_and(|d| d.default == value) {
            self.values.remove(key);
        } else {
            self.values.insert(key.to_string(), value);
        }
    }

    /// Setzt einen Wert auf die Vorgabe zurück.
    pub fn reset(&mut self, key: &str) {
        self.values.remove(key);
    }

    /// Alle Warnungen wieder zeigen (Dialog „Alle Warnmeldungen zurücksetzen“, 14 §3.4).
    pub fn reset_warnings(&mut self) {
        self.values.retain(|k, _| !k.starts_with("DisableConfirm") && !k.starts_with("DisableNotice"));
    }

    // ---------------------------------------------------------------- typisierte Zugriffe

    /// Höchstzahl der Verlaufsschritte (`MaxUndos`, Vorgabe 20; Werte < 1 → 0 wie `@00510ee0`).
    pub fn max_undos(&self) -> usize {
        self.get_int("MaxUndos", 20).max(0) as usize
    }

    /// Leinwand neuer Dokumente.
    pub fn canvas_default(&self) -> CanvasDefault {
        CanvasDefault::from_code(self.get_int("DefaultDocColorState", 1))
    }

    /// Leinwandfarbe neuer Dokumente (weiß, transparent oder `CustomCanvasColor`).
    pub fn new_document_background(&self) -> Argb {
        match self.canvas_default() {
            CanvasDefault::White => Argb(0xffff_ffff),
            CanvasDefault::Transparent => Argb(0x00ff_ffff),
            CanvasDefault::Custom => self.get_color("CustomCanvasColor", Argb(0xff00_0000)),
        }
    }

    /// Größe neuer Dokumente (`DefaultDocSize` = „B H“, Vorgabe 660 × 440).
    pub fn new_document_size(&self) -> (f32, f32) {
        let s = self.get_str("DefaultDocSize", "660 440");
        let mut it = s.split_whitespace().filter_map(|v| v.parse::<f32>().ok());
        match (it.next(), it.next()) {
            (Some(w), Some(h)) if w > 0.0 && h > 0.0 => (w, h),
            _ => (660.0, 440.0),
        }
    }

    /// Raster neuer Dokumente und Seiten wie im Original: Farbe `GridColor` (Vorgabe #00ffff),
    /// Abstände `GridHorizontalDistance`/`GridVerticalDistance` (36 px), Ursprung 0,0.
    pub fn new_document_grid(&self) -> fp_model::Grid {
        let size = fp_geom::Point::new(self.get_int("GridHorizontalDistance", 36).max(1) as f32, self.get_int("GridVerticalDistance", 36).max(1) as f32);
        fp_model::Grid { size, color: self.get_color("GridColor", Argb(0xff00_ffff)), ..fp_model::Grid::default() }
    }

    /// Auflösung neuer Dokumente (`DefaultDocRes`, `DefaultDocResUnitName`).
    pub fn new_document_resolution(&self) -> Resolution {
        Resolution {
            value: self.get_real("DefaultDocRes", 72.0) as f32,
            per_cm: Unit::from_name(&self.get_str("DefaultDocResUnitName", "inch")) == Some(Unit::Cm),
        }
    }

    /// Standard-Strich- und Füllfarbe (`BrushDefaultColor`, `FillDefaultColor`).
    pub fn default_colors(&self) -> (Argb, Argb) {
        (self.get_color("BrushDefaultColor", Argb(0xff00_0000)), self.get_color("FillDefaultColor", Argb(0xffff_ffff)))
    }

    /// Aktuelle Strich- und Füllfarbe (`BrushCurrentColor`, `FillCurrentColor`).
    pub fn current_colors(&self) -> (Argb, Argb) {
        (self.get_color("BrushCurrentColor", Argb(0xff00_0000)), self.get_color("FillCurrentColor", Argb(0xff99_9999)))
    }

    /// Abtaster für Transformationen (`EditTransformMode` 1…4).
    pub fn edit_sampler(&self) -> fp_raster::Sampler {
        fp_raster::Sampler::from_pref(self.get_int("EditTransformMode", 1))
    }

    /// Abtaster für „Bildgröße“ (`ImageSizeTransformMode`).
    pub fn image_size_sampler(&self) -> fp_raster::Sampler {
        fp_raster::Sampler::from_pref(self.get_int("ImageSizeTransformMode", 1))
    }

    /// Aktives Tastenkürzel-Set (`CurrentShortcutSet`).
    pub fn shortcut_set(&self) -> String {
        self.get_str("CurrentShortcutSet", "Firepixel")
    }

    /// Fangabstand in Bildschirmpixeln (`SnapDistance`).
    pub fn snap_distance(&self) -> f64 {
        self.get_int("SnapDistance", 5) as f64
    }

    /// Markierungsabstand in Bildschirmpixeln (`PickDistance`).
    pub fn pick_distance(&self) -> f64 {
        self.get_int("PickDistance", 5) as f64
    }

    /// Nimmt eine Datei in die Liste der zuletzt geöffneten auf (vorne, ohne Doppelte, höchstens 10).
    pub fn add_recent_file(&mut self, path: &Path) {
        self.recent_files.retain(|p| p != path);
        self.recent_files.insert(0, path.to_path_buf());
        self.recent_files.truncate(MAX_RECENT_FILES);
    }

    // ---------------------------------------------------------------- Datei

    /// Standardpfad der Voreinstellungsdatei: `%APPDATA%\Firepixel\preferences.json`
    /// (sonst `$XDG_CONFIG_HOME/firepixel/…` bzw. `$HOME/.config/firepixel/…`); in Testläufen im
    /// Testprofil ([`fp_i18n::profile`], nie das echte Nutzerprofil).
    pub fn default_path() -> Option<PathBuf> {
        fp_i18n::profile::roaming_dir().map(|d| d.join("preferences.json"))
    }

    /// Liest eine Voreinstellungsdatei.
    pub fn load(path: &Path) -> Result<Preferences> {
        let text = std::fs::read_to_string(path)?;
        Preferences::from_json(&text)
    }

    /// Liest die Datei oder liefert die Vorgaben, wenn sie fehlt bzw. beschädigt ist.
    pub fn load_or_default(path: &Path) -> Preferences {
        Preferences::load(path).unwrap_or_else(|_| Preferences::new())
    }

    /// Aus JSON-Text.
    pub fn from_json(text: &str) -> Result<Preferences> {
        let mut p: Preferences =
            serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| EngineError::Failed(fp_i18n::tr!("err-prefs", detail = e.to_string())))?;
        p.recent_files.truncate(MAX_RECENT_FILES);
        Ok(p)
    }

    /// Als JSON-Text (formatiert); ohne die nur für die Sitzung gemerkten Werte
    /// ([`SESSION_ONLY_KEYS`]).
    pub fn to_json(&self) -> String {
        let out = if self.values.keys().any(|k| SESSION_ONLY_KEYS.contains(&k.as_str())) {
            let mut c = self.clone();
            c.values.retain(|k, _| !SESSION_ONLY_KEYS.contains(&k.as_str()));
            serde_json::to_string_pretty(&c)
        } else {
            serde_json::to_string_pretty(self)
        };
        out.unwrap_or_else(|_| "{}".to_string())
    }

    /// Schreibt die Datei (über eine temporäre Datei, damit ein Abbruch keine halbe Datei hinterlässt).
    pub fn save(&self, path: &Path) -> Result<()> {
        fp_i18n::profile::guard_write(path);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, self.to_json())?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_overrides() {
        assert_eq!(definitions().len(), 644);
        let mut p = Preferences::new();
        assert_eq!(p.max_undos(), 20);
        assert_eq!(p.canvas_default(), CanvasDefault::White);
        assert_eq!(p.new_document_size(), (660.0, 440.0));
        p.set("MaxUndos", PrefValue::Int(5));
        assert_eq!(p.max_undos(), 5);
        p.set("MaxUndos", PrefValue::Int(20));
        assert!(p.values.is_empty(), "Vorgabewert wird nicht gespeichert");
        // Typtreue: real gespeichert, als int gelesen
        p.set("MaxUndos", PrefValue::Real(7.9));
        assert_eq!(p.max_undos(), 7);
        p.set("DefaultDocColorState", PrefValue::Int(9));
        assert_eq!(p.canvas_default(), CanvasDefault::White);
        p.theme = Theme::Oled;
        let back = Preferences::from_json(&p.to_json()).unwrap_or_default();
        assert_eq!(back, p);
        assert_eq!(Unit::Cm.to_pixels(2.54, 72.0), 72.0);
    }

    #[test]
    fn filter_memory_is_session_only() {
        let mut p = Preferences::new();
        p.set("AddNoiseAmount", PrefValue::Int(20));
        p.set("MaxUndos", PrefValue::Int(5));
        assert_eq!(p.get_int("AddNoiseAmount", 50), 20, "in der Sitzung gemerkt");
        let back = Preferences::from_json(&p.to_json()).unwrap_or_default();
        assert_eq!(back.get_int("AddNoiseAmount", 50), 50, "nach Neustart wieder die Vorgabe");
        assert_eq!(back.max_undos(), 5);
    }
}

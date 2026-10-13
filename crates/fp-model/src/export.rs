//! Export-/Optimierungseinstellungen (`EXPv`, Spezifikation 08 §1) samt Status-Eigenschaften (02 §8.1).

use crate::raw::Extra;
use serde::{Deserialize, Serialize};

/// Exporteinstellungen einer Seite, eines Symbols oder einer Slice.
///
/// Die Exportoptionen (erster Teil) schreibt das Original immer; der HTML-Teil steht nur im Seiten-`EXPv`.
/// Felder mit `Option` werden genau dann geschrieben, wenn sie gesetzt sind.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExportSettings {
    /// `exportFormat` (`FMTi`): 0 GIF, 1 JPEG, 2 PNG, 3 custom, 4 GIF-Animation.
    pub format: i32,
    /// Mac-Typ/-Creator (`MCTi`, `MCCi`, nur bei `format = 3`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mac_type: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mac_creator: Option<u32>,
    /// `colorMode` (`CLRi`): 0 indiziert, 1 24 Bit, 2 32 Bit.
    pub color_mode: i32,
    /// `paletteMode` (`PLTi`): 0 custom, 1 adaptiv, 2 Raster, 3 Graustufen, 4 Mac, 5 Windows, 6 exakt, 7 Web 216.
    pub palette_mode: i32,
    /// Eigene Palette (`PALv{FCLi [CLLb] [CLTb] [CLMb BCLi] …}`).
    pub palette: Vec<PaletteEntry>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub palette_extra: Extra,
    /// `PLNi`: bei Raster `numGridEntries`, sonst Anzahl der Palettenfarben.
    pub palette_count: i32,
    /// `numEntriesRequested` (`PLRi`).
    pub entries_requested: i32,
    /// `paletteTransparency` (`PTNi`): 0 keine, 1 Index, 2 Index-Alpha, 3 RGBA.
    pub transparency: i32,
    /// `optimized` (`PLOb`), `localAdaptive` (`APLb`), `webSnapAdaptive` (`APSb`).
    pub optimized: bool,
    pub local_adaptive: bool,
    pub websnap_adaptive: bool,
    /// `animAutoCrop` (`AACb`), `animAutoDifference` (`AADb`).
    pub anim_auto_crop: bool,
    pub anim_auto_difference: bool,
    /// `webSnapTolerance` (`ASTi`).
    pub websnap_tolerance: i32,
    /// Dithering (`PLDi`): 0 aus, 2 „2×2“, ≥ 6 Diffusion in Prozent; beim Lesen 1 = Diffusion 100 %.
    pub dither: i32,
    /// `lossyGifAmount` (`GLTi`).
    pub lossy: i32,
    /// JPEG: Qualität (`JPQi`), Glättung (`JPSi`), Subsampling (`JPOi`), progressiv (`JPPb`).
    pub jpeg_quality: i32,
    pub jpeg_smoothness: i32,
    pub jpeg_subsampling: i32,
    pub jpeg_progressive: bool,
    /// GIF/PNG Interlace (`GFIb`).
    pub interlaced: bool,
    /// Selektives JPEG: Qualität (`JSQi`), aktiv (`JSEb`), Text (`JSTb`), Schaltflächen (`JSBb`).
    pub jpeg_sel_quality: i32,
    pub jpeg_sel_enabled: bool,
    pub jpeg_sel_preserve_text: bool,
    pub jpeg_sel_preserve_buttons: bool,
    /// Schleife (`FLPi`): 0 endlos, −1 keine, n > 0 Wiederholungen.
    pub loop_count: i32,
    /// Status-Eigenschaften (`FNLv{FNMv…}`); nur gesetzte Status haben einen Eintrag.
    pub frames: Vec<FrameInfo>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub frames_extra: Extra,
    /// Exportbereich (`LRLi LRTi LRRi LRBi`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop: Option<[i32; 4]>,
    /// Skalierung (`CLSb CNSb CLPi XSZi YSZi`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<ExportScale>,
    /// HTML-/Exporteinstellungen (nur im Seiten-`EXPv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub html: Option<Box<HtmlSettings>>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

/// Eintrag der eigenen Palette.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PaletteEntry {
    /// Farbe (`FCLi`).
    pub color: u32,
    /// Gesperrt (`CLLb`), transparent (`CLTb`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transparent: Option<bool>,
    /// Bearbeitet (`CLMb`) mit neuer Farbe (`BCLi`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_color: Option<u32>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

/// Status-Eigenschaften (`FNMv{FNPi FNDi VISb [OBNs]}`, JS `Frame`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrameInfo {
    /// Löschmethode (`FNPi`): 0 nicht angegeben, 1 ohne, 2 Hintergrund, 3 vorheriges.
    pub disposal: i32,
    /// Verzögerung in 1/100 s (`FNDi`, Vorgabe 7).
    pub delay: i32,
    /// Im Export sichtbar (`VISb`).
    pub visible: bool,
    /// Statusname (`OBNs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

impl Default for FrameInfo {
    fn default() -> Self {
        FrameInfo { disposal: 0, delay: 7, visible: true, name: None, extra: Extra::default() }
    }
}

/// Skalierung beim Export.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExportScale {
    pub use_scale: bool,
    pub constrain: bool,
    pub percent: i32,
    pub width: i32,
    pub height: i32,
}

/// HTML-/Exporteinstellungen (08 §1.5). Alle optional, weil Versionen unterschiedliche Teilmengen schreiben.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct HtmlSettings {
    /// `sliceAlongGuides` (`GSLb`), `sliceUsingUrls` (`LSUb`), `discardUnspecifiedSlices` (`DSUb`),
    /// `generateDemoHtml` (`LSDb`), `cssPopups` (`CSMb`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slice_along_guides: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slice_using_urls: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discard_unspecified_slices: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generate_demo_html: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub css_popups: Option<bool>,
    /// `externalCSS` (`ECSi`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_css: Option<i32>,
    /// `setByUser` invertiert (`NYSb`: 1 = noch nicht vom Benutzer gesetzt).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_yet_set: Option<bool>,
    /// `shimGeneration` (`LSSi`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shim_generation: Option<i32>,
    /// `htmlDestination` (`HTMi`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub html_destination: Option<i32>,
    /// `templateName` (`HTTs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_name: Option<String>,
    /// Bildverzeichnis (`IMDs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub images_dir: Option<String>,
    /// `exportFilesStyle` (`XFSi`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub export_files_style: Option<i32>,
    /// Letztes Exportverzeichnis (`LXDs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_export_dir: Option<String>,
    /// `multipleFilesMode` (`XMDi`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multiple_files_mode: Option<i32>,
    /// Bytes ohne JS-Namen (`TRMb`, `HTMb`, `XDAb`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trm: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub htm: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xda: Option<bool>,
    /// Basisname (`EFBs`), Erweiterung (`EFXs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_extension: Option<String>,
    /// Slice-Benennung (`NM4v{NM4i×6}`) und Status-Benennung (`NMFv{NMFi×2}`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slice_naming: Option<Vec<i32>>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub slice_naming_extra: Extra,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_naming: Option<Vec<i32>>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub frame_naming_extra: Extra,
    /// Statusnamen der Schaltflächen (`NS0s…NS3s`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_names: Option<[String; 4]>,
    /// `docHtmlEncoding` (`ENCi`), `tableAlignment` (`TALi`), `tablePctWidth` (`TPWi`), XHTML (`XTMi`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encoding: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_alignment: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_pct_width: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xhtml: Option<i32>,
    /// CSS: Hintergrund-URL (`CBUs`), Wiederholung (`CBRi`), Bindung (`CBAi`), Seitenausrichtung (`CDAi`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_repeat: Option<i32>,
    /// `backgroundAttachment` (`CBAi`): 0 fixed, 1 scroll.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_attachment: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_alignment: Option<i32>,
    /// Ränder (`CDMv{LFTf TOPf RITf BOTf}`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margins: Option<[f32; 4]>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub margins_extra: Extra,
    /// Externe CSS-Datei für das Dokument (`CXBb`), Dateiname (`CXFs`), CSS-Verzeichnis (`CBDs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_css_for_doc: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_css_filename: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub css_dir: Option<String>,
    /// Matte-Farbe (`FCLi` direkt im `EXPv`, nur wenn `useMatteColor`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matte_color: Option<u32>,
    /// Exportoptionen für CSS-Sprites (re\18 §4.2, §7.2: `spriteExportSelectorIsClass`,
    /// `…CSSPrefix`, `…CSSSuffix`, `…Padding`, `…Orientation` 1 waagerecht / 2 senkrecht /
    /// 3 Raster, `…PresetName`). Tags des Originals unbekannt; im FW-PNG als Firepixel-eigene
    /// Erweiterung `fpSv` im `EXPv` gespeichert (fp-fwpng `write::SPRITE_EXT`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sprite_selector_is_class: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sprite_css_prefix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sprite_css_suffix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sprite_padding: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sprite_orientation: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sprite_preset: Option<String>,
    /// „Standard-ALT-Tag“ der HTML-Einrichtung (re\18 §6.4). Tag des Originals
    /// unbekannt; im FW-PNG als `DALs` in der eigenen Erweiterung `fpSv` (fp-fwpng `write::SPRITE_EXT`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_alt: Option<String>,
}

impl Default for ExportSettings {
    /// Original-Vorgaben (`@007dba30`, 08 §1.2) mit HTML-Teil wie in neuen CS6-Dokumenten.
    fn default() -> Self {
        ExportSettings {
            format: 0,
            mac_type: None,
            mac_creator: None,
            color_mode: 0,
            palette_mode: 1,
            palette: Vec::new(),
            palette_extra: Extra::default(),
            palette_count: 0,
            entries_requested: 256,
            transparency: 0,
            optimized: true,
            local_adaptive: true,
            websnap_adaptive: false,
            anim_auto_crop: true,
            anim_auto_difference: true,
            websnap_tolerance: 14,
            dither: 0,
            lossy: 0,
            jpeg_quality: 80,
            jpeg_smoothness: 0,
            jpeg_subsampling: 0,
            jpeg_progressive: false,
            interlaced: false,
            jpeg_sel_quality: 90,
            jpeg_sel_enabled: false,
            jpeg_sel_preserve_text: true,
            jpeg_sel_preserve_buttons: false,
            loop_count: 0,
            frames: Vec::new(),
            frames_extra: Extra::default(),
            crop: None,
            scale: None,
            html: Some(Box::new(HtmlSettings::cs6_default())),
            extra: Extra::default(),
        }
    }
}

impl HtmlSettings {
    /// HTML-Einstellungen wie in neuen CS6-Dokumenten (Korpus, z. B. `Anchor.graphic.png`).
    pub fn cs6_default() -> HtmlSettings {
        HtmlSettings {
            slice_along_guides: Some(false),
            slice_using_urls: Some(false),
            discard_unspecified_slices: Some(false),
            generate_demo_html: Some(false),
            css_popups: Some(true),
            external_css: Some(1),
            not_yet_set: Some(true),
            shim_generation: Some(3),
            html_destination: Some(0),
            export_files_style: Some(1),
            multiple_files_mode: Some(0),
            trm: Some(true),
            htm: Some(true),
            xda: Some(true),
            slice_naming: Some(vec![6, 15, 1, 0, 0, 0]),
            frame_naming: Some(vec![5, 1]),
            state_names: Some([String::new(), "_o".into(), "_d".into(), "_od".into()]),
            encoding: Some(1),
            table_alignment: Some(0),
            table_pct_width: Some(0),
            xhtml: Some(1),
            background_url: Some(String::new()),
            background_repeat: Some(0),
            // `CBAi`: 0 fixed, 1 scroll (re 8 §1.5); neue Dokumente des Originals schreiben 1
            background_attachment: Some(1),
            page_alignment: Some(0),
            margins: Some([0.5, 0.5, 0.5, 0.5]),
            external_css_for_doc: Some(false),
            ..HtmlSettings::default()
        }
    }
}

impl ExportSettings {
    /// Exportoptionen ohne HTML-Teil (Slices, Symbole).
    pub fn options_only() -> ExportSettings {
        ExportSettings { html: None, ..ExportSettings::default() }
    }

    /// Eigenschaften eines Status (Vorgaben, wenn kein Eintrag existiert).
    pub fn frame(&self, index: usize) -> FrameInfo {
        self.frames.get(index).cloned().unwrap_or_default()
    }

    /// Dithering als (Modus, Prozent) nach der Leseregel (08 §1.3): Modus 0 aus, 1 Diffusion, 2 „2×2“.
    pub fn dither_mode(&self) -> (i32, i32) {
        match self.dither {
            v if v >= 6 => (1, v),
            1 => (1, 100),
            2 => (2, 100),
            _ => (0, 100),
        }
    }
}

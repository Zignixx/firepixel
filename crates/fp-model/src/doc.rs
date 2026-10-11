//! Dokument, Seiten und der gemeinsame Teil von Seite und Symbol (Spezifikationen 02 §4, 04 §2).
//!
//! Dateisicht: `MKBv` (Hauptwurzel: Symbolbibliothek, Stile, Vorgaben und eine Kompatibilitätskopie der
//! aktiven Seite) gefolgt von `PDCv` je Seite. Das Modell hält die Seiten aus den `PDCv` und die globalen
//! Listen aus `MKBv`. Die `MKBv`-Seitenkopie bleibt für die verlustfreie Rundreise in
//! [`Document::root_copy`] erhalten; nach Änderungen an der aktiven Seite verwirft man sie
//! ([`Document::invalidate_root_copy`]), dann erzeugt ein Schreiber sie neu.

use crate::attrs::{Brush, Fill, Gradient, PathAttrs};
use crate::effects::EffectList;
use crate::export::ExportSettings;
use crate::ids::{Guid, IdGen};
use crate::image::MaskData;
use crate::layer::{LayerSlot, SharedLayerDef};
use crate::raw::{Extra, RawNode};
use crate::serde_ext;
use crate::style::Style;
use crate::symbol::Symbol;
use crate::text::AteData;
use fp_color::Argb;
use fp_geom::Point;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Gemeinsamer Teil von Seite und Symbol (im Original `CDbRootElem`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LayerRoot {
    /// Anzahl Status (`FRCi`). Das Original schreibt den Tag zweimal mit demselben Wert.
    pub frame_count: u32,
    /// Wie oft `FRCi` in der Datei stand (Seiten 2, Symbole 1); `None` = Vorgabe des Originals.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_count_repeat: Option<u32>,
    /// Ebenenliste unten → oben (`LYLv`).
    pub layers: Vec<LayerSlot>,
    /// Ist `LYLv` vorhanden?
    pub has_layer_list: bool,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub layers_extra: Extra,
    /// Raster (`GRDv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid: Option<Grid>,
    /// Hilfslinien (`GDSv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guides: Option<Guides>,
    /// Zwiebelschicht: Status vor/nach dem aktuellen (`OSBi`, `OSAi`; negativ gilt als 0).
    pub onion_before: i32,
    pub onion_after: i32,
    /// Leinwandfarbe ARGB (`BGCi`; `00FFFFFF` = transparent).
    #[serde(with = "serde_ext::argb")]
    pub background: Argb,
    /// Exporteinstellungen (`EXPv`), inkl. Status-Eigenschaften und Schleife.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub export: Option<Box<ExportSettings>>,
}

impl LayerRoot {
    /// Leere Wurzel mit `frames` Status und weißer Leinwand.
    pub fn new(frames: u32) -> LayerRoot {
        LayerRoot {
            frame_count: frames.max(1),
            frame_count_repeat: None,
            layers: Vec::new(),
            has_layer_list: true,
            layers_extra: Extra::default(),
            grid: None,
            guides: None,
            onion_before: 0,
            onion_after: 0,
            background: Argb(0xffff_ffff),
            export: None,
        }
    }
}

/// Leinwand einer Seite.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Canvas {
    /// Ursprung (`XLCf`, `YLCf`; oft negativ, z. B. bei Symbol-Bibliotheksdateien).
    #[serde(with = "serde_ext::point")]
    pub origin: Point,
    /// Breite und Höhe in px (`WIDf`, `HITf`).
    pub width: f32,
    pub height: f32,
    /// Auflösung (`RESf`) und Einheit (`RSUi`: 0 Pixel/Zoll, 1 Pixel/cm).
    pub resolution: f32,
    pub resolution_unit: i32,
}

impl Default for Canvas {
    fn default() -> Self {
        Canvas { origin: Point::default(), width: 500.0, height: 500.0, resolution: 72.0, resolution_unit: 0 }
    }
}

/// Raster (`GRDv{GOXf GOYf GSXf GSYf GCLi}`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    #[serde(with = "serde_ext::point")]
    pub origin: Point,
    #[serde(with = "serde_ext::point")]
    pub size: Point,
    #[serde(with = "serde_ext::argb")]
    pub color: Argb,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

impl Default for Grid {
    fn default() -> Self {
        Grid { origin: Point::default(), size: Point::new(36.0, 36.0), color: Argb(0xff66_6666), extra: Extra::default() }
    }
}

/// Hilfslinien (`GDSv{GDLb GDKi GDHv{GDCf…} GDVv{GDCf…}}`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Guides {
    /// Gesperrt (`GDLb`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
    /// Farbe (`GDKi`).
    #[serde(default, skip_serializing_if = "Option::is_none", with = "serde_ext::opt_argb")]
    pub color: Option<Argb>,
    /// Horizontale Hilfslinien (y-Positionen, `GDHv`).
    pub horizontal: Vec<f32>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub horizontal_extra: Extra,
    /// Vertikale Hilfslinien (x-Positionen, `GDVv`).
    pub vertical: Vec<f32>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub vertical_extra: Extra,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

impl Default for Guides {
    fn default() -> Self {
        Guides {
            locked: Some(false),
            color: Some(Argb(0xff00_ff00)),
            horizontal: Vec::new(),
            horizontal_extra: Extra::default(),
            vertical: Vec::new(),
            vertical_extra: Extra::default(),
            extra: Extra::default(),
        }
    }
}

/// Vorgabelisten (`PLLv`, `FLDv`, `GLDv`, `EFDv`): Bibliothek der verfügbaren Striche, Füllungen,
/// Verläufe und Effekte mit Standardwerten. Keine Anwendung auf Objekte.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Presets {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brushes: Option<Vec<Brush>>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub brushes_extra: Extra,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fills: Option<Vec<Fill>>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub fills_extra: Extra,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gradients: Option<Vec<Gradient>>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub gradients_extra: Extra,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effects: Option<Vec<EffectList>>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub effects_extra: Extra,
}

/// Altform der Image-Map (`MNFv{MNDs}`).
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct ImageMap {
    /// `MNDs` (CS6 schreibt einen leeren String).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

/// Gespeicherte Pixelauswahl bzw. JPEG-Maske (`MSVv`, `OSSv`, `MJPv`: `{MSKv LFTf TOPf [NMSs]}`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedSelection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mask: Option<MaskData>,
    pub left: f32,
    pub top: f32,
    /// Name (`NMSs`, nur bei gespeicherten Auswahlen).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

/// Eine Seite (`PDCv`; auch die `MKBv`-Kopie der aktiven Seite).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Page {
    /// Formatversion (`VRSi`; fehlt → 8). CS6 schreibt 10.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format_version: Option<u32>,
    /// Unterversion (`VRMi`, CS6 schreibt 1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format_minor: Option<u32>,
    /// Zufallszahl bzw. Dokument-ID (`RQNi`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc_random: Option<u32>,
    pub canvas: Canvas,
    /// Ebenen, Status, Raster, Hilfslinien, Leinwandfarbe, Export.
    pub root: LayerRoot,
    pub presets: Presets,
    /// Werkzeug-Attribute für neue Objekte (`PATv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_attrs: Option<Arc<PathAttrs>>,
    /// Altform der Image-Map (`MNFv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_map: Option<ImageMap>,
    /// Seitenname (`PGNs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Masterseite (`IMPb`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_master: Option<bool>,
    /// Mit Masterseite verknüpft (`PILb`, gesetzt über `setLinkPage(…, true)`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_to_master: Option<bool>,
    /// Seiten-GUID (`PIDs`). In der `MKBv`-Kopie die GUID der Hauptwurzel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<Guid>,
    /// Geteilte Ebenen, die in dieser Wurzel definiert sind (`LSMv`). Im Korpus stehen alle Definitionen
    /// in `MKBv` ([`Document::shared_layers`]); in `PDCv` ist die Liste leer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_shared_layers: Option<Vec<SharedLayerDef>>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub local_shared_layers_extra: Extra,
    /// `ILLb`: Ebenenliste ist eine Kompatibilitätskopie (nur `MKBv`, immer 0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compat_layer_copy: Option<bool>,
    /// Letztes Exportverzeichnis und letzte Exportdatei (`LXDs`, `LXFs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_export_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_export_file: Option<String>,
    /// Aktuelle Pixelauswahl (`MSVv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<SavedSelection>,
    /// Gespeicherte Auswahlen (`MSSv{OSSv…}`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saved_selections: Option<Vec<SavedSelection>>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub saved_selections_extra: Extra,
    /// JPEG-Maske für selektives JPEG (`MJPv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jpeg_mask: Option<SavedSelection>,
    /// Binärblock `PTRy` (nur durchgereicht).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printer_data: Option<Vec<u8>>,
    /// Altform der Animationsschleife im Seitenkopf (`FLPi`), überschreibt die Exporteinstellung.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy_loop: Option<i32>,
    /// Zweiter Exportblock (`EXSv`, Altform mit Vorrang).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy_export: Option<Box<ExportSettings>>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub extra: Extra,
}

impl Page {
    /// Formatversion mit Lesevorgabe 8.
    pub fn version(&self) -> u32 {
        self.format_version.unwrap_or(8)
    }
    /// Masterseite?
    pub fn master(&self) -> bool {
        self.is_master.unwrap_or(false)
    }
    /// Name oder leer.
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or("")
    }
    /// Wirksame Schleifenzahl (Altform im Kopf hat Vorrang).
    pub fn loop_count(&self) -> i32 {
        self.legacy_loop.or_else(|| self.root.export.as_ref().map(|e| e.loop_count)).unwrap_or(0)
    }
}

/// Daten des PNG-Containers, die nicht zum Dokument gehören, aber beim Speichern erhalten bleiben sollen.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct FileMeta {
    /// Textchunks (`tEXt`) in Dateireihenfolge: (Schlüssel, Text Latin-1), z. B. „Software“, „Creation Time“.
    pub text_chunks: Vec<(String, String)>,
    /// XMP-Paket (`iTXt` bzw. `tEXt` mit dem XMP-Schlüsselwort, erkannt am Paket `<x:xmpmeta`): Rohdaten des Chunks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xmp_chunk: Option<RawChunk>,
    /// Auflösung aus `pHYs` (Pixel je Meter x/y, Einheit).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phys: Option<(u32, u32, u8)>,
    /// Weitere unbekannte Chunks mit ihrer Position relativ zu `IDAT` (vor/nach).
    pub other_chunks: Vec<RawChunk>,
    /// `mkBS`-Platzhalter der Originaldatei (entpackter Baum) und seine Kacheln, unverändert.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compat_tree: Option<Vec<RawNode>>,
    #[serde(skip)]
    pub compat_tiles: Vec<Arc<crate::image::Tile>>,
    /// Vorschau-Chunks der Originaldatei (`prVW`/`pRVW`/`mkPG`, unverändert, in Dateireihenfolge).
    #[serde(skip)]
    pub previews: Vec<RawChunk>,
    /// `mkBF`-Inhalt der Originaldatei.
    #[serde(skip)]
    pub format_block: Option<Vec<u8>>,
    /// Flaches Bild (IDAT) der Originaldatei, RGBA8; wird übernommen, solange kein Renderer bereitsteht.
    #[serde(skip)]
    pub flat_image: Option<Arc<FlatImage>>,
}

/// Ein unveränderter PNG-Chunk.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RawChunk {
    /// Chunk-Typ (4 Zeichen).
    pub kind: [u8; 4],
    #[serde(skip)]
    pub data: Vec<u8>,
    /// Stand der Chunk hinter `IDAT`?
    pub after_image: bool,
}

/// RGBA8-Bild (nicht vormultipliziert).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct FlatImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl FlatImage {
    /// Einfarbiges Bild.
    pub fn filled(width: u32, height: u32, rgba: [u8; 4]) -> FlatImage {
        let mut px = Vec::with_capacity(width as usize * height as usize * 4);
        for _ in 0..(width as usize * height as usize) {
            px.extend_from_slice(&rgba);
        }
        FlatImage { width, height, rgba: px }
    }
}

/// Ein Dokument (Modell der FW-PNG).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Document {
    /// Seiten in Anzeigereihenfolge (`PDCv`). Mindestens eine.
    pub pages: Vec<Arc<Page>>,
    /// Aktive Seite (die `MKBv` kopiert und deren Bild im `IDAT` steht).
    pub current_page: usize,
    /// Kompatibilitätskopie der aktiven Seite aus `MKBv` (mit deren Rundreise-Daten; die globalen Listen
    /// stehen nicht darin, sondern in diesem Dokument). `None` = beim Schreiben neu erzeugen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_copy: Option<Arc<Page>>,
    /// GUID der Hauptwurzel (`MKBv/PIDs`). Das Original erhält sie beim Speichern (B, Laufzeit 04 T3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_id: Option<Guid>,
    /// Altes Einseiten-Format ohne `PDCv` (FW 8/MX/frühe CS3): `pages[0]` ist der `MKBv`-Inhalt.
    pub single_root: bool,
    /// Symbolbibliothek (`MSLv`) in Dateireihenfolge; `SymbolId` kann doppelt vorkommen.
    pub symbols: Vec<Arc<Symbol>>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub symbols_extra: Extra,
    /// Stile (`ZSLv`); `None` = Liste fehlt (Dateien vor CS3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub styles: Option<Vec<Arc<Style>>>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub styles_extra: Extra,
    /// Über Seiten geteilte Ebenen (`MKBv/LSMv`), nach UID sortiert wie das Original sie schreibt.
    pub shared_layers: Vec<SharedLayerDef>,
    #[serde(default, skip_serializing_if = "Extra::is_empty")]
    pub shared_layers_extra: Extra,
    /// Ist `MKBv/LSMv` vorhanden?
    pub has_shared_layer_list: bool,
    /// Text-Engine-Daten (`aTXT`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ate: Option<AteData>,
    /// Weitere Wurzeln hinter `PDCv` (`ZSLv`, `ZEPv` in Bibliotheksdateien).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_roots: Vec<RawNode>,
    /// Vergibt Laufzeit-IDs.
    pub ids: IdGen,
    /// PNG-Container-Daten der Originaldatei.
    #[serde(default)]
    pub file_meta: FileMeta,
}

impl Document {
    /// Aktive Seite.
    pub fn active_page(&self) -> Option<&Arc<Page>> {
        self.pages.get(self.current_page)
    }

    /// Formatversion (aus der aktiven Seite bzw. der Hauptwurzel; Lesevorgabe 8).
    pub fn format_version(&self) -> u32 {
        self.root_copy
            .as_ref()
            .and_then(|p| p.format_version)
            .or_else(|| self.active_page().and_then(|p| p.format_version))
            .unwrap_or(8)
    }

    /// Verwirft die `MKBv`-Kompatibilitätskopie; ein Schreiber erzeugt sie dann aus der aktiven Seite.
    /// Nach jeder Änderung der aktiven Seite (oder Wechsel der aktiven Seite) aufrufen.
    pub fn invalidate_root_copy(&mut self) {
        self.root_copy = None;
    }

    /// Dekodierte ATE-Story eines Texts (`AIDi` ist 1-basiert), nur wenn `FOAb = 1`.
    pub fn text_story(&self, t: &crate::text::TextElem) -> Option<&crate::text::AteStory> {
        if t.from_story != Some(true) {
            return None;
        }
        let i = (t.story_index? as usize).checked_sub(1)?;
        self.ate.as_ref()?.stories.get(i)
    }

    /// Veränderlicher Zugriff auf eine Seite (Copy-on-Write); verwirft die `MKBv`-Kopie, wenn es die
    /// aktive Seite ist.
    pub fn page_mut(&mut self, index: usize) -> Option<&mut Page> {
        if index == self.current_page {
            self.root_copy = None;
        }
        self.pages.get_mut(index).map(Arc::make_mut)
    }
}

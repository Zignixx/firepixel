//! Dokumentmodell von Firepixel (reine Daten, serde-fähig, ohne UI- und Render-Abhängigkeit).
//!
//! Grundlage: Spezifikation 04 (Objektmodell, Rust-Entwurf `fw_model_entwurf.rs`), 02 (Dateiformat) und
//! die Fach-Spezifikationen 03/05/07/08/09/13/15 mit den Laufzeitergebnissen vom 11.10.2026.
//!
//! # Aufbau
//!
//! ```text
//! Document
//!  ├─ pages: Vec<Arc<Page>>          PDCv je Seite (Masterseite: Page::is_master)
//!  │   ├─ canvas, presets, tool_attrs, export (root.export) …
//!  │   └─ root: LayerRoot            Status (frame_count), Ebenenliste unten → oben, Raster, Hilfslinien
//!  │       └─ layers: Vec<LayerSlot> Own(Layer) | Shared(Verweis LSAv) | MasterPage(MPLv)
//!  │           └─ Layer.cels[Status] → Cel.elements (oben → unten) → Element
//!  ├─ root_copy                      MKBv-Kopie der aktiven Seite (Rundreise; None = neu erzeugen)
//!  ├─ symbols: Vec<Arc<Symbol>>      MSLv (SymbolId kann doppelt sein: erste Definition gewinnt)
//!  ├─ styles, shared_layers, ate (aTXT), file_meta (PNG-Container)
//! ```
//!
//! # Grundsätze
//!
//! * **Copy-on-Write**: Knoten liegen in `Arc`; Änderungen über `Arc::make_mut` (siehe
//!   [`Element::edit`], [`Document::page_mut`], [`Document::element_at_mut`]). Ein Undo-Schritt kann
//!   so einfach ein alter Klon des Dokuments sein (unveränderte Teilbäume werden geteilt).
//! * **IDs**: Laufzeit-Handles ([`ElementId`], [`LayerId`], u64) für Auswahl, Undo und Skripte;
//!   persistente IDs nur wie im Format ([`Guid`], [`SymbolId`], Stilnamen).
//! * **Verlustfreie Rundreise**: Unbekannte Tags bleiben als [`RawNode`] erhalten; jeder Container
//!   trägt [`Extra`] mit der Reihenfolge seiner Kinder beim Lesen. Ein Leser/Schreiber
//!   (`fp-fwpng`) bildet den Tag-Baum darauf ab.
//! * **Reihenfolgen wie Format und JS-API**: Ebenen unten → oben, Elemente oben → unten.
//! * **Render-Caches** sind kein Inhalt ([`Element::cache`]); bei Inhaltsänderung verwerfen.
//!
//! Die Grundtypen [`fp_geom::Point`], [`fp_geom::Rect`], [`fp_geom::Matrix3`], [`fp_color::Argb`],
//! [`fp_color::Opacity`], [`fp_color::BlendMode`] kommen aus `fp-geom`/`fp-color`; die serde-Anbindung
//! liegt in [`serde_ext`].

pub mod attrs;
pub mod data;
pub mod doc;
pub mod effects;
pub mod element;
pub mod export;
pub mod ids;
pub mod image;
pub mod layer;
pub mod new;
pub mod raw;
pub mod serde_ext;
pub mod style;
pub mod symbol;
pub mod text;
pub mod traverse;
pub mod web;

pub use attrs::{
    Brush, Fill, FillHandles, Gradient, GradientStop, GradientStops, PathAttrs, Pattern,
    Sensitivity, StopList, Texture,
};
pub use data::{
    CustomData, DataDict, DataEntry, DataList, DataValue, EntryTag, PointList, TypedScalar,
};
pub use doc::{
    Canvas, Document, FileMeta, FlatImage, Grid, Guides, ImageMap, LayerRoot, Page, Presets,
    RawChunk, SavedSelection,
};
pub use effects::{Effect, EffectKind, EffectList, EffectTag};
pub use element::{
    AnimParams, BitmapElem, BitmapNode, ButtonStrings, CommonAttrs, CompoundShapeElem, Contour,
    ContourNode, ControlPoint, Element, ElementKind, ElementMask, GroupElem, GroupKind,
    InstanceElem, MaskFlags, PathElem, PolyPoint, Polyline, PressureProfile, PressureSample,
    RectPrimitive, SmartShape,
};
pub use export::{ExportScale, ExportSettings, FrameInfo, HtmlSettings, PaletteEntry};
pub use ids::{ElementId, Guid, IdGen, LayerId, SymbolId, SymbolKey};
pub use image::{ImageData, MaskData, Tile, TileEntry, TileGrid, TileKind};
pub use layer::{
    Cel, ElementState, Layer, LayerKind, LayerSlot, MasterPageLayer, SharedLayerDef, SharedLayerRef,
};
pub use raw::{Extra, RawNode, RawValue, TagKey};
pub use style::Style;
pub use symbol::{ButtonOptions, NineSlice, Symbol, SymbolKind};
pub use text::{
    AteCharAttrs, AteData, AteParagraphAttrs, AteStory, CosDict, CosValue, TextAttrs, TextElem,
    TextOnPath, TextRun,
};
pub use traverse::{ElementPath, LayerOrigin, PathStep, ResolvedLayer, RootRef, WalkCtx};
pub use web::{
    Behavior, BehaviorAction, Behaviors, MenuItem, PopupMenu, SliceBackground, SliceProps,
    WebObjectElem, WebShape,
};

pub use fp_color::{Argb, BlendMode, Opacity, Rgba8};
pub use fp_geom::{Matrix3, Point, Rect};

//! Bedienfelder (11 §2): Kennungen, Titel, Fenster-Befehle und Inhalte.
//!
//! Jedes Bedienfeld ist ein [`PanelId`]. Der Inhalt wird pro Frame gezeichnet ([`show`]) und schickt
//! Änderungen als Befehle über die Aktions-Warteschlange an fp-engine (immediate mode statt
//! Beobachter, 11 §6). Phase 4: alle Bedienfelder mit Inhalt nach 11 §2.2 (Steuerelemente aus
//! `bedienfelder.json`/`dialoge.json`), Inhalte live aus der Sitzung.

pub mod align;
pub mod autoshapes;
pub mod color;
pub mod common;
pub mod drag_drop;
pub mod history;
pub mod layers;
pub mod library;
pub mod misc;
pub mod optimize;
pub mod pages;
pub mod states;
pub mod styles;
pub mod thumbs;

use crate::app_state::AppState;
use crate::theme::ToolGroup;
use serde::{Deserialize, Serialize};

/// Farben des Farbfelder-Bedienfelds (auch für das Farbfelder-Popup des Inspektors).
pub fn swatch_colors(ctx: &egui::Context, st: &AppState) -> Vec<fp_color::Argb> {
    color::current_swatches(ctx, st)
}

/// Name eines Status (Statusname aus den Exporteinstellungen oder „Status n“).
pub fn state_name(st: &AppState, i: usize) -> String {
    st.session
        .active()
        .and_then(|d| d.doc.pages.get(d.page))
        .and_then(|p| p.root.export.as_ref())
        .and_then(|e| e.frames.get(i))
        .and_then(|f| f.name.clone())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| fp_i18n::tr!("panel-state-default-name", n = i + 1))
}

/// Alle andockbaren Bedienfelder (ohne Werkzeuge und Eigenschaften-Inspektor).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PanelId {
    Optimize,
    History,
    Align,
    Pages,
    States,
    Layers,
    Styles,
    ColorPalette,
    Swatches,
    SpecialCharacters,
    ImageEditing,
    Path,
    AutoShapes,
    DocumentLibrary,
    CommonLibrary,
    Info,
    ColorMixer,
    Url,
    Behaviors,
    FindReplace,
    SymbolProperties,
    AutoShapeProperties,
    CssProperties,
}

impl PanelId {
    /// Alle Bedienfelder.
    pub const ALL: &'static [PanelId] = &[
        PanelId::Optimize,
        PanelId::History,
        PanelId::Align,
        PanelId::Pages,
        PanelId::States,
        PanelId::Layers,
        PanelId::Styles,
        PanelId::ColorPalette,
        PanelId::Swatches,
        PanelId::SpecialCharacters,
        PanelId::ImageEditing,
        PanelId::Path,
        PanelId::AutoShapes,
        PanelId::DocumentLibrary,
        PanelId::CommonLibrary,
        PanelId::Info,
        PanelId::ColorMixer,
        PanelId::Url,
        PanelId::Behaviors,
        PanelId::FindReplace,
        PanelId::SymbolProperties,
        PanelId::AutoShapeProperties,
        PanelId::CssProperties,
    ];

    /// Schlüssel des Reitertitels in `panels.ftl`.
    pub fn title_key(self) -> &'static str {
        match self {
            PanelId::Optimize => "panel-title-optimize",
            PanelId::History => "panel-title-history",
            PanelId::Align => "panel-title-align",
            PanelId::Pages => "panel-title-pages",
            PanelId::States => "panel-title-states",
            PanelId::Layers => "panel-title-layers",
            PanelId::Styles => "panel-title-styles",
            PanelId::ColorPalette => "panel-title-color-palette",
            PanelId::Swatches => "panel-title-swatches",
            PanelId::SpecialCharacters => "panel-title-special-characters",
            PanelId::ImageEditing => "panel-title-image-editing",
            PanelId::Path => "panel-title-path",
            PanelId::AutoShapes => "panel-title-auto-shapes",
            PanelId::DocumentLibrary => "panel-title-document-library",
            PanelId::CommonLibrary => "panel-title-common-library",
            PanelId::Info => "panel-title-info",
            PanelId::ColorMixer => "panel-title-color-mixer",
            PanelId::Url => "panel-title-url",
            PanelId::Behaviors => "panel-title-behaviors",
            PanelId::FindReplace => "panel-title-find-replace",
            PanelId::SymbolProperties => "panel-title-symbol-properties",
            PanelId::AutoShapeProperties => "panel-title-auto-shape-properties",
            PanelId::CssProperties => "panel-title-css-properties",
        }
    }

    /// Reitertitel (wie im Fenster-Menü, ohne Kürzel) in der aktuellen Sprache.
    pub fn title(self) -> &'static str {
        common::t_static(self.title_key())
    }

    /// Befehl in fp-engine (Fenster-Menü), falls es einen gibt. Die Flash-Bedienfelder von CS6
    /// (Ausrichten, Pfad, …) haben keinen Katalogbefehl; fp-ui führt sie selbst.
    pub fn engine_command(self) -> Option<&'static str> {
        Some(match self {
            PanelId::Optimize => "window.optimize",
            PanelId::History => "window.history",
            PanelId::Pages => "window.pages",
            PanelId::States => "window.states",
            PanelId::Layers => "window.layers",
            PanelId::Styles => "window.styles",
            PanelId::Swatches => "window.swatches",
            PanelId::AutoShapes => "window.autoShapes",
            PanelId::DocumentLibrary => "window.documentLibrary",
            PanelId::CommonLibrary => "window.commonLibrary",
            PanelId::Info => "window.info",
            PanelId::ColorMixer => "window.colorMixer",
            PanelId::Url => "window.url",
            PanelId::Behaviors => "window.behaviors",
            PanelId::FindReplace => "edit.findAndReplace",
            PanelId::CssProperties => "window.cssProperties",
            _ => return None,
        })
    }

    /// Bedienfeld zu einem Befehlsnamen.
    pub fn from_command(name: &str) -> Option<PanelId> {
        PanelId::ALL
            .iter()
            .copied()
            .find(|p| p.engine_command() == Some(name))
    }

    /// Name des Bedienfelds für die Brücke (`ui_action {"panel": …}`, `ui_state.visible_panels`):
    /// der Befehl des Fenster-Menüs, bei Bedienfeldern ohne Katalogbefehl ein Name in derselben
    /// Form (`window.align`, `window.path`, …), damit jedes Bedienfeld des Fenster-Menüs erreichbar ist.
    pub fn bridge_name(self) -> &'static str {
        if let Some(c) = self.engine_command() {
            return c;
        }
        match self {
            PanelId::Align => "window.align",
            PanelId::ColorPalette => "window.colorPalette",
            PanelId::SpecialCharacters => "window.specialCharacters",
            PanelId::ImageEditing => "window.imageEditing",
            PanelId::Path => "window.path",
            PanelId::SymbolProperties => "window.symbolProperties",
            PanelId::AutoShapeProperties => "window.autoShapeProperties",
            // alle übrigen haben einen Befehl (oben)
            _ => "window.unknown",
        }
    }

    /// Bedienfeld zu einem Brückennamen: [`PanelId::bridge_name`] mit oder ohne `window.`,
    /// ohne Rücksicht auf Groß-/Kleinschreibung, oder die Original-Befehls-ID ([`PanelId::fw_id`]).
    pub fn from_bridge_name(name: &str) -> Option<PanelId> {
        let n = name.trim();
        if let Ok(id) = n.parse::<u32>() {
            return PanelId::ALL.iter().copied().find(|p| p.fw_id() == id);
        }
        PanelId::ALL.iter().copied().find(|p| {
            let b = p.bridge_name();
            b.eq_ignore_ascii_case(n)
                || b.strip_prefix("window.")
                    .is_some_and(|s| s.eq_ignore_ascii_case(n))
        })
    }

    /// Original-Befehls-ID des Bedienfelds (Arbeitsbereich-XML `app-data`, 10 §4.5, 11 §4.2).
    pub fn fw_id(self) -> u32 {
        match self {
            PanelId::Optimize => 43409,
            PanelId::History => 43418,
            PanelId::Align => 48601,
            PanelId::Pages => 35798,
            PanelId::States => 43410,
            PanelId::Layers => 43412,
            PanelId::Styles => 43415,
            PanelId::ColorPalette => 48605,
            PanelId::Swatches => 43408,
            PanelId::SpecialCharacters => 48607,
            PanelId::ImageEditing => 48606,
            PanelId::Path => 48608,
            PanelId::AutoShapes => 43422,
            PanelId::DocumentLibrary => 43416,
            PanelId::CommonLibrary => 45816,
            PanelId::Info => 43404,
            PanelId::ColorMixer => 43405,
            PanelId::Url => 43414,
            PanelId::Behaviors => 43413,
            PanelId::FindReplace => 43402,
            PanelId::SymbolProperties => 48609,
            PanelId::AutoShapeProperties => 48602,
            PanelId::CssProperties => 49125,
        }
    }

    /// Kennfarbe des Reiters im OLED-Design (nach fachlicher Nähe zur Werkzeuggruppe).
    pub fn group(self) -> ToolGroup {
        match self {
            PanelId::Optimize | PanelId::Url | PanelId::Behaviors | PanelId::CssProperties => {
                ToolGroup::Web
            }
            PanelId::History | PanelId::FindReplace | PanelId::Info => ToolGroup::View,
            PanelId::Align | PanelId::Pages | PanelId::States | PanelId::Layers => {
                ToolGroup::Select
            }
            PanelId::Styles | PanelId::ColorPalette | PanelId::Swatches | PanelId::ColorMixer => {
                ToolGroup::Colors
            }
            PanelId::SpecialCharacters
            | PanelId::Path
            | PanelId::AutoShapes
            | PanelId::AutoShapeProperties => ToolGroup::Vector,
            PanelId::ImageEditing
            | PanelId::DocumentLibrary
            | PanelId::CommonLibrary
            | PanelId::SymbolProperties => ToolGroup::Bitmap,
        }
    }

    /// Kontextmenü-Ressource des Bedienfeldmenüs (≡), falls bekannt (11 §2.1).
    pub fn menu_resource(self) -> Option<&'static str> {
        Some(match self {
            PanelId::Layers => "6923",
            PanelId::States => "6924",
            PanelId::Pages => "7026",
            PanelId::History => "6939",
            PanelId::Styles => "6935",
            PanelId::Swatches => "6919",
            PanelId::ColorMixer => "6920",
            PanelId::Info => "6932",
            PanelId::Optimize => "6941",
            PanelId::DocumentLibrary => "6937",
            _ => return None,
        })
    }

    /// Bevorzugte Inhaltshöhe im Dock.
    pub fn preferred_height(self) -> f32 {
        match self {
            PanelId::Optimize => 300.0,
            PanelId::Layers | PanelId::Pages | PanelId::States => 300.0,
            PanelId::Styles | PanelId::Swatches | PanelId::ColorPalette => 280.0,
            _ => 240.0,
        }
    }
}

/// Zeichnet den Inhalt eines Bedienfelds.
pub fn show(ui: &mut egui::Ui, st: &mut AppState, id: PanelId) {
    match id {
        PanelId::Pages => pages::show(ui, st),
        PanelId::States => states::show(ui, st),
        PanelId::Layers => layers::show(ui, st),
        PanelId::History => history::show(ui, st),
        PanelId::Align => align::show(ui, st),
        PanelId::Path => misc::path(ui, st),
        PanelId::Swatches => color::swatches(ui, st),
        PanelId::ColorMixer => color::mixer(ui, st),
        PanelId::ColorPalette => color::palette(ui, st),
        PanelId::Info => misc::info(ui, st),
        PanelId::Optimize => optimize::show(ui, st),
        PanelId::Styles => styles::show(ui, st),
        PanelId::DocumentLibrary => library::document(ui, st),
        PanelId::CommonLibrary => library::common(ui, st),
        PanelId::AutoShapes => autoshapes::panel(ui, st),
        PanelId::SpecialCharacters => misc::special_characters(ui, st),
        PanelId::ImageEditing => misc::image_editing(ui, st),
        PanelId::Url => misc::url(ui, st),
        PanelId::Behaviors => misc::behaviors(ui, st),
        PanelId::FindReplace => misc::find_replace(ui, st),
        PanelId::SymbolProperties => misc::symbol_properties(ui, st),
        PanelId::AutoShapeProperties => autoshapes::properties(ui, st),
        PanelId::CssProperties => misc::css_properties(ui, st),
    }
    // Ziehen aus Bedienfeldern auf die Leinwand (Vorschau, Ablegen)
    let ctx = ui.ctx().clone();
    drag_drop::frame(&ctx, st);
}

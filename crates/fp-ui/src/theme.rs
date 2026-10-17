//! Zentrales Design-System: alle Farben, Maße und Schriftgrößen der Oberfläche.
//!
//! Drei Designs (ARCHITEKTUR §4):
//! - **Normal**: das Hellgrau des Originals. Die Werte sind aus `re\ui-referenz\01_hauptfenster.png`
//!   gemessen (Rahmen 214, Leinwandbereich 200, Trennlinien 100/127/100, Kopfzeilen-Verlauf 167→160,
//!   Listen 237, Auswahl 168/198/237 …).
//! - **Dunkel**: dieselbe Gliederung in dunklen Grautönen.
//! - **OLED**: reines Schwarz `#000000` und bewusst farbenfroh: jede Werkzeuggruppe hat eine kräftige
//!   Akzentfarbe, Icons sind farbig, Reiter tragen farbige Kennungen, Auswahl und Hover leuchten,
//!   Fokus-Ränder sind farbig.
//!
//! Widgets lesen Farben **nur** aus [`Theme`]; es gibt keine fest eingebauten Farbwerte in den
//! Zeichenfunktionen. Das aktive Design steht im egui-Kontext ([`Theme::current`]).

use egui::{Color32, CornerRadius, FontFamily, FontId, Stroke, Visuals};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Auswahl des Designs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThemeKind {
    /// Hellgrau wie das Original (Vorgabe).
    #[default]
    Normal,
    /// Dunkle Grautöne.
    Dark,
    /// Reines Schwarz, farbenfroh.
    Oled,
}

impl ThemeKind {
    /// Alle Designs in Menüreihenfolge.
    pub const ALL: [ThemeKind; 3] = [ThemeKind::Normal, ThemeKind::Dark, ThemeKind::Oled];

    /// Feste deutsche Kennung (Dateinamen von Bildschirmfotos, Testberichte). Für die Anzeige
    /// [`ThemeKind::name`] nehmen.
    pub fn label(self) -> &'static str {
        match self {
            ThemeKind::Normal => "Normal",
            ThemeKind::Dark => "Dunkel",
            ThemeKind::Oled => "OLED",
        }
    }

    /// Angezeigter Name in der aktuellen Sprache (Menü Ansicht ▸ Design, Voreinstellungen).
    pub fn name(self) -> String {
        fp_i18n::t(match self {
            ThemeKind::Normal => "ui-theme-normal",
            ThemeKind::Dark => "ui-theme-dark",
            ThemeKind::Oled => "ui-theme-oled",
        })
    }

    /// Umrechnung aus der Voreinstellung von fp-engine.
    pub fn from_engine(t: fp_engine::Theme) -> ThemeKind {
        match t {
            fp_engine::Theme::Normal => ThemeKind::Normal,
            fp_engine::Theme::Dark => ThemeKind::Dark,
            fp_engine::Theme::Oled => ThemeKind::Oled,
        }
    }

    /// Umrechnung in die Voreinstellung von fp-engine.
    pub fn to_engine(self) -> fp_engine::Theme {
        match self {
            ThemeKind::Normal => fp_engine::Theme::Normal,
            ThemeKind::Dark => fp_engine::Theme::Dark,
            ThemeKind::Oled => fp_engine::Theme::Oled,
        }
    }
}

/// Werkzeuggruppen des Werkzeugbedienfelds (Abschnitte, 11 §4.3). Im OLED-Design hat jede Gruppe
/// eine eigene Akzentfarbe, die auch Bedienfelder und Menüs der Gruppe kennzeichnet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolGroup {
    Select,
    Bitmap,
    Vector,
    Web,
    Colors,
    View,
}

impl ToolGroup {
    /// Alle Gruppen in Bedienfeld-Reihenfolge.
    pub const ALL: [ToolGroup; 6] = [
        ToolGroup::Select,
        ToolGroup::Bitmap,
        ToolGroup::Vector,
        ToolGroup::Web,
        ToolGroup::Colors,
        ToolGroup::View,
    ];

    /// Überschrift im Werkzeugbedienfeld in der aktuellen Sprache.
    pub fn name(self) -> String {
        fp_i18n::t(match self {
            ToolGroup::Select => "ui-toolgroup-select",
            ToolGroup::Bitmap => "ui-toolgroup-bitmap",
            ToolGroup::Vector => "ui-toolgroup-vector",
            ToolGroup::Web => "ui-toolgroup-web",
            ToolGroup::Colors => "ui-toolgroup-colors",
            ToolGroup::View => "ui-toolgroup-view",
        })
    }

    /// Feste deutsche Kennung (wie die Abschnittsnamen der Werkzeugtabelle). Für die Anzeige
    /// [`ToolGroup::name`] nehmen.
    pub fn label(self) -> &'static str {
        match self {
            ToolGroup::Select => "Auswählen",
            ToolGroup::Bitmap => "Bitmap",
            ToolGroup::Vector => "Vektor",
            ToolGroup::Web => "Web",
            ToolGroup::Colors => "Farben",
            ToolGroup::View => "Ansicht",
        }
    }

    /// Index 0..6.
    pub fn index(self) -> usize {
        match self {
            ToolGroup::Select => 0,
            ToolGroup::Bitmap => 1,
            ToolGroup::Vector => 2,
            ToolGroup::Web => 3,
            ToolGroup::Colors => 4,
            ToolGroup::View => 5,
        }
    }

    /// Gruppe zum Abschnittsnamen aus der Werkzeugtabelle von fp-engine.
    pub fn from_section(s: &str) -> ToolGroup {
        match s {
            "Bitmap" => ToolGroup::Bitmap,
            "Vektor" => ToolGroup::Vector,
            "Web" => ToolGroup::Web,
            "Farben" => ToolGroup::Colors,
            "Ansicht" => ToolGroup::View,
            _ => ToolGroup::Select,
        }
    }
}

/// Alle Farben eines Designs.
#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    // Anwendungsleiste (Titel + Menüs)
    pub app_bar_bg: Color32,
    pub app_bar_text: Color32,
    pub app_bar_logo: Color32,
    pub app_bar_logo_accent: Color32,
    pub app_bar_hover_bg: Color32,
    pub app_bar_open_bg: Color32,
    pub app_bar_open_border: Color32,
    pub window_button_hover: Color32,
    pub window_close_hover: Color32,
    // Menüs
    pub menu_bg: Color32,
    pub menu_border: Color32,
    pub menu_text: Color32,
    pub menu_text_disabled: Color32,
    pub menu_hover_bg: Color32,
    pub menu_hover_border: Color32,
    pub menu_hover_text: Color32,
    pub menu_separator: Color32,
    pub menu_check: Color32,
    // Rahmen, Leisten, Trenner
    pub frame_bg: Color32,
    pub line_dark: Color32,
    pub line_mid: Color32,
    pub line_light: Color32,
    pub grip_dark: Color32,
    pub grip_light: Color32,
    // Kopfzeilen (Bedienfeldgruppen, Inspektor, Dokumentreiter)
    pub header_top: Color32,
    pub header_bottom: Color32,
    pub header_highlight: Color32,
    pub header_line: Color32,
    pub dock_strip_top: Color32,
    pub dock_strip_bottom: Color32,
    pub dock_strip_arrow: Color32,
    pub tab_active_bg: Color32,
    pub tab_inactive_bg: Color32,
    pub tab_hover_bg: Color32,
    pub tab_separator: Color32,
    pub tab_text: Color32,
    pub tab_text_inactive: Color32,
    // Dokumentbereich
    pub doc_tab_bar_top: Color32,
    pub doc_tab_bar_bottom: Color32,
    pub doc_tab_active_bg: Color32,
    pub doc_tab_inactive_bg: Color32,
    pub doc_tab_text: Color32,
    pub doc_tab_border: Color32,
    pub canvas_bg: Color32,
    pub canvas_shadow: Color32,
    pub page_border: Color32,
    pub checker_a: Color32,
    pub checker_b: Color32,
    // Text
    pub text: Color32,
    pub text_weak: Color32,
    pub text_disabled: Color32,
    pub section_label: Color32,
    pub status_text: Color32,
    // Eingabefelder, Listen, Knöpfe
    pub field_bg: Color32,
    pub field_border: Color32,
    pub field_text: Color32,
    pub list_bg: Color32,
    pub list_border: Color32,
    pub list_selected_bg: Color32,
    pub list_selected_text: Color32,
    pub list_hover_bg: Color32,
    pub list_row_line: Color32,
    pub button_bg: Color32,
    pub button_border: Color32,
    pub button_hover_bg: Color32,
    pub button_hover_border: Color32,
    pub button_pressed_bg: Color32,
    pub tool_selected_bg: Color32,
    pub tool_selected_border: Color32,
    pub tool_hover_bg: Color32,
    pub tool_hover_border: Color32,
    pub focus: Color32,
    // Icons
    pub icon_fg: Color32,
    pub icon_light: Color32,
    pub icon_paper: Color32,
    pub icon_disabled: Color32,
    pub icon_accent: Color32,
    pub icon_accent2: Color32,
    pub icon_red: Color32,
    pub icon_green: Color32,
    pub icon_blue: Color32,
    pub icon_yellow: Color32,
    // Leinwand-Overlays
    pub selection: Color32,
    pub handle_fill: Color32,
    pub handle_border: Color32,
    pub marquee_a: Color32,
    pub marquee_b: Color32,
    pub tool_preview: Color32,
    pub ruler_bg: Color32,
    pub ruler_text: Color32,
    pub ruler_tick: Color32,
    pub ruler_marker: Color32,
    pub guide_default: Color32,
    pub grid_default: Color32,
    pub scrollbar_bg: Color32,
    pub scrollbar_thumb: Color32,
    /// Akzentfarbe je Werkzeuggruppe ([`ToolGroup::index`]).
    pub group_accent: [Color32; 6],
    // Leinwand-Werkzeuge (Phase 4)
    /// Hervorhebung beim Überfahren (`SelectionMouseOverPreview`, in CS6 rot).
    pub hover_highlight: Color32,
    /// Intelligente Hilfslinien (Rückfall, wenn `SmartGuideColor` fehlt).
    pub smart_guide: Color32,
    /// Mauszeiger: Füllung und Kontur.
    pub cursor_fill: Color32,
    pub cursor_outline: Color32,
    /// Beschriftungen auf der Leinwand (Größe, Winkel, Abstand).
    pub label_bg: Color32,
    pub label_text: Color32,
    /// Abdunkelung außerhalb des Zuschneiderahmens.
    pub crop_shade: Color32,
    /// Text-Editor: Auswahl und Caret.
    pub text_selection: Color32,
    pub text_caret: Color32,
}

/// Maße in logischen Pixeln (gemessen an `01_hauptfenster.png` bei 100 % Systemskalierung).
#[derive(Clone, Debug, PartialEq)]
pub struct Metrics {
    pub app_bar_height: f32,
    pub toolbar_height: f32,
    pub toolbar_button: f32,
    pub dock_strip_height: f32,
    pub tools_width: f32,
    pub tool_cell: egui::Vec2,
    pub tool_section_label_height: f32,
    pub dock_width: f32,
    pub dock_min_width: f32,
    pub panel_tab_height: f32,
    pub panel_tab_pad: f32,
    pub splitter: f32,
    pub doc_tab_height: f32,
    pub view_bar_height: f32,
    pub footer_height: f32,
    pub inspector_header_height: f32,
    pub inspector_row_height: f32,
    pub ruler_size: f32,
    pub menu_item_height: f32,
    pub menu_min_width: f32,
    pub icon_size: f32,
    pub handle_size: f32,
    pub list_row_height: f32,
    pub field_height: f32,
}

/// Schriftgrößen in Punkten der egui-Skala (≈ Pixel).
#[derive(Clone, Debug, PartialEq)]
pub struct FontSizes {
    /// Menüleiste und Menüs (Segoe UI 9 pt).
    pub menu: f32,
    /// Bedienfelder, Inspektor, Dialoge.
    pub ui: f32,
    /// Reiterbeschriftungen der Bedienfelder (fett).
    pub tab: f32,
    /// Abschnittsüberschriften im Werkzeugbedienfeld.
    pub section: f32,
    /// Fußleiste, Lineale.
    pub small: f32,
    /// Dialogtitel.
    pub heading: f32,
}

/// Ein vollständiges Design.
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub kind: ThemeKind,
    pub colors: Palette,
    pub metrics: Metrics,
    pub fonts: FontSizes,
    /// Icons farbig zeichnen (OLED).
    pub colorful_icons: bool,
    /// Leuchtende Auswahl/Hover mit Schein (OLED).
    pub glow: bool,
    /// Kopfzeilen mit Verlauf statt flach.
    pub gradients: bool,
}

const fn g(v: u8) -> Color32 {
    Color32::from_rgb(v, v, v)
}

const fn rgb(r: u8, gg: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, gg, b)
}

/// Name der fetten Schriftfamilie.
pub const BOLD_FAMILY: &str = "fp-bold";

impl Theme {
    /// Design zu einer Auswahl.
    pub fn new(kind: ThemeKind) -> Theme {
        match kind {
            ThemeKind::Normal => Theme::normal(),
            ThemeKind::Dark => Theme::dark(),
            ThemeKind::Oled => Theme::oled(),
        }
    }

    fn metrics() -> Metrics {
        Metrics {
            app_bar_height: 31.0,
            toolbar_height: 31.0,
            toolbar_button: 28.0,
            dock_strip_height: 12.0,
            tools_width: 62.0,
            tool_cell: egui::vec2(26.0, 24.0),
            tool_section_label_height: 20.0,
            dock_width: 268.0,
            dock_min_width: 180.0,
            panel_tab_height: 16.0,
            panel_tab_pad: 7.0,
            splitter: 3.0,
            doc_tab_height: 20.0,
            view_bar_height: 25.0,
            footer_height: 20.0,
            inspector_header_height: 18.0,
            inspector_row_height: 48.0,
            ruler_size: 16.0,
            menu_item_height: 22.0,
            menu_min_width: 230.0,
            icon_size: 16.0,
            handle_size: 5.0,
            list_row_height: 30.0,
            field_height: 20.0,
        }
    }

    fn font_sizes() -> FontSizes {
        FontSizes {
            menu: 12.0,
            ui: 11.5,
            tab: 11.0,
            section: 12.0,
            small: 11.0,
            heading: 13.0,
        }
    }

    /// Hellgrau wie das Original.
    pub fn normal() -> Theme {
        let c = Palette {
            app_bar_bg: g(214),
            app_bar_text: g(0),
            app_bar_logo: g(185),
            app_bar_logo_accent: g(160),
            app_bar_hover_bg: g(230),
            app_bar_open_bg: g(228),
            app_bar_open_border: g(124),
            window_button_hover: g(232),
            window_close_hover: rgb(232, 17, 35),
            menu_bg: g(249),
            menu_border: g(150),
            menu_text: g(0),
            menu_text_disabled: g(160),
            menu_hover_bg: rgb(204, 232, 255),
            menu_hover_border: rgb(153, 209, 255),
            menu_hover_text: g(0),
            menu_separator: g(215),
            menu_check: g(30),
            frame_bg: g(214),
            line_dark: g(100),
            line_mid: g(127),
            line_light: g(232),
            grip_dark: g(133),
            grip_light: g(232),
            header_top: g(167),
            header_bottom: g(160),
            header_highlight: g(199),
            header_line: g(115),
            dock_strip_top: g(159),
            dock_strip_bottom: g(142),
            dock_strip_arrow: g(62),
            tab_active_bg: g(214),
            tab_inactive_bg: g(163),
            tab_hover_bg: g(178),
            tab_separator: g(115),
            tab_text: g(43),
            tab_text_inactive: g(43),
            doc_tab_bar_top: g(167),
            doc_tab_bar_bottom: g(160),
            doc_tab_active_bg: g(214),
            doc_tab_inactive_bg: g(180),
            doc_tab_text: g(0),
            doc_tab_border: g(125),
            canvas_bg: g(200),
            canvas_shadow: g(187),
            page_border: g(150),
            checker_a: g(255),
            checker_b: g(204),
            text: g(0),
            text_weak: g(90),
            text_disabled: g(150),
            section_label: rgb(16, 16, 96),
            status_text: g(140),
            field_bg: g(255),
            field_border: rgb(130, 135, 144),
            field_text: g(0),
            list_bg: g(237),
            list_border: rgb(130, 135, 144),
            list_selected_bg: rgb(168, 198, 237),
            list_selected_text: g(0),
            list_hover_bg: rgb(212, 225, 243),
            list_row_line: g(205),
            button_bg: g(225),
            button_border: g(160),
            button_hover_bg: g(240),
            button_hover_border: g(120),
            button_pressed_bg: g(200),
            tool_selected_bg: g(255),
            tool_selected_border: g(160),
            tool_hover_bg: g(236),
            tool_hover_border: g(170),
            focus: rgb(0, 120, 215),
            icon_fg: g(40),
            icon_light: g(150),
            icon_paper: g(255),
            icon_disabled: g(170),
            icon_accent: rgb(0, 102, 204),
            icon_accent2: rgb(255, 102, 0),
            icon_red: rgb(220, 30, 30),
            icon_green: rgb(40, 150, 40),
            icon_blue: rgb(30, 110, 220),
            icon_yellow: rgb(240, 190, 30),
            selection: rgb(0, 153, 255),
            handle_fill: rgb(0, 153, 255),
            handle_border: g(255),
            marquee_a: g(0),
            marquee_b: g(255),
            tool_preview: rgb(0, 153, 255),
            ruler_bg: g(240),
            ruler_text: g(40),
            ruler_tick: g(90),
            ruler_marker: rgb(220, 30, 30),
            guide_default: rgb(0, 255, 0),
            grid_default: g(102),
            scrollbar_bg: g(220),
            scrollbar_thumb: g(180),
            group_accent: [
                rgb(0, 102, 204),
                rgb(0, 102, 204),
                rgb(0, 102, 204),
                rgb(0, 102, 204),
                rgb(0, 102, 204),
                rgb(0, 102, 204),
            ],
            hover_highlight: rgb(255, 0, 0),
            smart_guide: rgb(255, 74, 255),
            cursor_fill: g(0),
            cursor_outline: g(255),
            label_bg: Color32::from_rgba_unmultiplied(255, 255, 225, 235),
            label_text: g(0),
            crop_shade: Color32::from_black_alpha(110),
            text_selection: Color32::from_rgba_unmultiplied(51, 153, 255, 110),
            text_caret: g(0),
        };
        Theme {
            kind: ThemeKind::Normal,
            colors: c,
            metrics: Self::metrics(),
            fonts: Self::font_sizes(),
            colorful_icons: false,
            glow: false,
            gradients: true,
        }
    }

    /// Dunkle Grautöne.
    pub fn dark() -> Theme {
        let accent = rgb(70, 150, 230);
        let c = Palette {
            app_bar_bg: g(58),
            app_bar_text: g(220),
            app_bar_logo: g(120),
            app_bar_logo_accent: g(95),
            app_bar_hover_bg: g(78),
            app_bar_open_bg: g(40),
            app_bar_open_border: g(25),
            window_button_hover: g(80),
            window_close_hover: rgb(232, 17, 35),
            menu_bg: g(45),
            menu_border: g(20),
            menu_text: g(225),
            menu_text_disabled: g(110),
            menu_hover_bg: rgb(52, 92, 140),
            menu_hover_border: rgb(70, 120, 180),
            menu_hover_text: g(255),
            menu_separator: g(70),
            menu_check: g(230),
            frame_bg: g(66),
            line_dark: g(28),
            line_mid: g(45),
            line_light: g(88),
            grip_dark: g(30),
            grip_light: g(100),
            header_top: g(52),
            header_bottom: g(46),
            header_highlight: g(70),
            header_line: g(25),
            dock_strip_top: g(48),
            dock_strip_bottom: g(38),
            dock_strip_arrow: g(170),
            tab_active_bg: g(66),
            tab_inactive_bg: g(48),
            tab_hover_bg: g(58),
            tab_separator: g(30),
            tab_text: g(230),
            tab_text_inactive: g(165),
            doc_tab_bar_top: g(50),
            doc_tab_bar_bottom: g(44),
            doc_tab_active_bg: g(66),
            doc_tab_inactive_bg: g(52),
            doc_tab_text: g(230),
            doc_tab_border: g(25),
            canvas_bg: g(40),
            canvas_shadow: g(28),
            page_border: g(20),
            checker_a: g(255),
            checker_b: g(204),
            text: g(225),
            text_weak: g(160),
            text_disabled: g(105),
            section_label: rgb(150, 190, 240),
            status_text: g(130),
            field_bg: g(34),
            field_border: g(22),
            field_text: g(230),
            list_bg: g(50),
            list_border: g(25),
            list_selected_bg: rgb(52, 92, 140),
            list_selected_text: g(255),
            list_hover_bg: g(62),
            list_row_line: g(42),
            button_bg: g(78),
            button_border: g(32),
            button_hover_bg: g(92),
            button_hover_border: g(20),
            button_pressed_bg: g(46),
            tool_selected_bg: g(40),
            tool_selected_border: g(22),
            tool_hover_bg: g(80),
            tool_hover_border: g(40),
            focus: accent,
            icon_fg: g(215),
            icon_light: g(140),
            icon_paper: g(78),
            icon_disabled: g(100),
            icon_accent: accent,
            icon_accent2: rgb(255, 140, 50),
            icon_red: rgb(240, 80, 80),
            icon_green: rgb(90, 200, 90),
            icon_blue: rgb(90, 160, 240),
            icon_yellow: rgb(240, 200, 60),
            selection: rgb(0, 153, 255),
            handle_fill: rgb(0, 153, 255),
            handle_border: g(255),
            marquee_a: g(0),
            marquee_b: g(255),
            tool_preview: rgb(0, 170, 255),
            ruler_bg: g(52),
            ruler_text: g(200),
            ruler_tick: g(150),
            ruler_marker: rgb(255, 90, 90),
            guide_default: rgb(0, 255, 0),
            grid_default: g(102),
            scrollbar_bg: g(46),
            scrollbar_thumb: g(90),
            group_accent: [accent, accent, accent, accent, accent, accent],
            hover_highlight: rgb(255, 40, 40),
            smart_guide: rgb(255, 74, 255),
            cursor_fill: g(0),
            cursor_outline: g(255),
            label_bg: Color32::from_rgba_unmultiplied(40, 40, 40, 235),
            label_text: g(235),
            crop_shade: Color32::from_black_alpha(130),
            text_selection: Color32::from_rgba_unmultiplied(51, 153, 255, 110),
            text_caret: g(0),
        };
        Theme {
            kind: ThemeKind::Dark,
            colors: c,
            metrics: Self::metrics(),
            fonts: Self::font_sizes(),
            colorful_icons: false,
            glow: false,
            gradients: true,
        }
    }

    /// Reines Schwarz, farbenfroh (OLED).
    pub fn oled() -> Theme {
        // kräftige Akzente je Werkzeuggruppe
        let select = rgb(0, 200, 255);
        let bitmap = rgb(255, 64, 129);
        let vector = rgb(124, 255, 60);
        let web = rgb(255, 176, 0);
        let colors = rgb(190, 100, 255);
        let view = rgb(0, 235, 180);
        let black = g(0);
        let c = Palette {
            app_bar_bg: black,
            app_bar_text: g(240),
            app_bar_logo: select,
            app_bar_logo_accent: bitmap,
            app_bar_hover_bg: rgb(0, 40, 52),
            app_bar_open_bg: rgb(0, 30, 40),
            app_bar_open_border: select,
            window_button_hover: g(30),
            window_close_hover: rgb(255, 30, 60),
            menu_bg: black,
            menu_border: select,
            menu_text: g(240),
            menu_text_disabled: g(95),
            menu_hover_bg: rgb(0, 52, 66),
            menu_hover_border: select,
            menu_hover_text: g(255),
            menu_separator: g(40),
            menu_check: vector,
            frame_bg: black,
            line_dark: g(34),
            line_mid: g(18),
            line_light: g(0),
            grip_dark: g(70),
            grip_light: g(0),
            header_top: black,
            header_bottom: black,
            header_highlight: black,
            header_line: g(40),
            dock_strip_top: black,
            dock_strip_bottom: black,
            dock_strip_arrow: select,
            tab_active_bg: g(14),
            tab_inactive_bg: black,
            tab_hover_bg: g(22),
            tab_separator: g(34),
            tab_text: g(250),
            tab_text_inactive: g(150),
            doc_tab_bar_top: black,
            doc_tab_bar_bottom: black,
            doc_tab_active_bg: g(14),
            doc_tab_inactive_bg: black,
            doc_tab_text: g(245),
            doc_tab_border: g(40),
            canvas_bg: black,
            canvas_shadow: g(0),
            page_border: g(60),
            checker_a: g(255),
            checker_b: g(204),
            text: g(240),
            text_weak: g(165),
            text_disabled: g(90),
            section_label: select,
            status_text: g(120),
            field_bg: black,
            field_border: g(60),
            field_text: g(245),
            list_bg: black,
            list_border: g(40),
            list_selected_bg: rgb(0, 60, 80),
            list_selected_text: g(255),
            list_hover_bg: g(20),
            list_row_line: g(24),
            button_bg: g(10),
            button_border: g(60),
            button_hover_bg: rgb(0, 40, 52),
            button_hover_border: select,
            button_pressed_bg: rgb(0, 70, 90),
            tool_selected_bg: g(10),
            tool_selected_border: select,
            tool_hover_bg: g(16),
            tool_hover_border: g(80),
            focus: select,
            icon_fg: g(235),
            icon_light: g(120),
            icon_paper: g(30),
            icon_disabled: g(70),
            icon_accent: select,
            icon_accent2: bitmap,
            icon_red: rgb(255, 60, 90),
            icon_green: vector,
            icon_blue: rgb(60, 160, 255),
            icon_yellow: web,
            selection: select,
            handle_fill: select,
            handle_border: black,
            marquee_a: black,
            marquee_b: select,
            tool_preview: bitmap,
            ruler_bg: black,
            ruler_text: g(200),
            ruler_tick: g(110),
            ruler_marker: bitmap,
            guide_default: vector,
            grid_default: g(80),
            scrollbar_bg: black,
            scrollbar_thumb: g(60),
            group_accent: [select, bitmap, vector, web, colors, view],
            hover_highlight: bitmap,
            smart_guide: rgb(255, 74, 255),
            cursor_fill: black,
            cursor_outline: select,
            label_bg: Color32::from_rgba_unmultiplied(0, 0, 0, 235),
            label_text: select,
            crop_shade: Color32::from_black_alpha(160),
            text_selection: Color32::from_rgba_unmultiplied(0, 229, 255, 90),
            text_caret: vector,
        };
        Theme {
            kind: ThemeKind::Oled,
            colors: c,
            metrics: Self::metrics(),
            fonts: Self::font_sizes(),
            colorful_icons: true,
            glow: true,
            gradients: false,
        }
    }

    /// Akzentfarbe einer Werkzeuggruppe.
    pub fn accent(&self, group: ToolGroup) -> Color32 {
        self.colors.group_accent[group.index()]
    }

    // ------------------------------------------------------------------ Schriften

    /// Normale UI-Schrift in der Größe `size`.
    pub fn font(&self, size: f32) -> FontId {
        FontId::new(size, FontFamily::Proportional)
    }

    /// Fette Schrift (Reiter, Überschriften).
    pub fn bold(&self, size: f32) -> FontId {
        FontId::new(size, FontFamily::Name(BOLD_FAMILY.into()))
    }

    /// Schrift der Menüs.
    pub fn menu_font(&self) -> FontId {
        self.font(self.fonts.menu)
    }

    /// Schrift der Bedienfelder.
    pub fn ui_font(&self) -> FontId {
        self.font(self.fonts.ui)
    }

    /// Kleine Schrift.
    pub fn small_font(&self) -> FontId {
        self.font(self.fonts.small)
    }

    /// Schrift der Bedienfeld-Reiter.
    pub fn tab_font(&self) -> FontId {
        self.bold(self.fonts.tab)
    }

    // ------------------------------------------------------------------ egui

    /// egui-Visuals aus dem Design ableiten (Standard-Widgets wie Textfelder, Kombifelder, Fenster).
    pub fn visuals(&self) -> Visuals {
        let c = &self.colors;
        let mut v = if self.kind == ThemeKind::Normal {
            Visuals::light()
        } else {
            Visuals::dark()
        };
        let r0 = CornerRadius::ZERO;
        let r2 = CornerRadius::same(2);
        v.override_text_color = Some(c.text);
        v.panel_fill = c.frame_bg;
        // Popups (Menüs, Kombifelder) nutzen die Fensterfarbe; Dialoge setzen ihren Rahmen selbst.
        v.window_fill = c.menu_bg;
        v.window_stroke = Stroke::new(1.0, c.menu_border);
        v.window_corner_radius = r0;
        v.menu_corner_radius = r0;
        v.window_shadow = egui::Shadow {
            offset: [2, 2],
            blur: 6,
            spread: 0,
            color: Color32::from_black_alpha(if self.kind == ThemeKind::Normal {
                60
            } else {
                140
            }),
        };
        v.popup_shadow = egui::Shadow {
            offset: [2, 2],
            blur: 4,
            spread: 0,
            color: Color32::from_black_alpha(if self.kind == ThemeKind::Normal {
                50
            } else {
                120
            }),
        };
        v.extreme_bg_color = c.field_bg;
        v.text_edit_bg_color = Some(c.field_bg);
        v.faint_bg_color = c.list_bg;
        v.code_bg_color = c.list_bg;
        v.hyperlink_color = c.focus;
        v.selection.bg_fill = c.list_selected_bg;
        v.selection.stroke = Stroke::new(1.0, c.list_selected_text);
        v.weak_text_color = Some(c.text_weak);
        v.striped = false;
        v.slider_trailing_fill = true;
        v.handle_shape = egui::style::HandleShape::Rect { aspect_ratio: 0.6 };
        v.widgets.noninteractive.bg_fill = c.frame_bg;
        v.widgets.noninteractive.weak_bg_fill = c.frame_bg;
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, c.line_mid);
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, c.text);
        v.widgets.noninteractive.corner_radius = r0;
        v.widgets.inactive.bg_fill = c.field_bg;
        v.widgets.inactive.weak_bg_fill = c.button_bg;
        v.widgets.inactive.bg_stroke = Stroke::new(1.0, c.field_border);
        v.widgets.inactive.fg_stroke = Stroke::new(1.0, c.text);
        v.widgets.inactive.corner_radius = r2;
        v.widgets.hovered.bg_fill = c.button_hover_bg;
        v.widgets.hovered.weak_bg_fill = c.button_hover_bg;
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, c.button_hover_border);
        v.widgets.hovered.fg_stroke = Stroke::new(1.0, c.text);
        v.widgets.hovered.corner_radius = r2;
        v.widgets.hovered.expansion = 0.0;
        v.widgets.active.bg_fill = c.button_pressed_bg;
        v.widgets.active.weak_bg_fill = c.button_pressed_bg;
        v.widgets.active.bg_stroke = Stroke::new(1.0, c.focus);
        v.widgets.active.fg_stroke = Stroke::new(1.0, c.text);
        v.widgets.active.corner_radius = r2;
        v.widgets.active.expansion = 0.0;
        v.widgets.open.bg_fill = c.field_bg;
        v.widgets.open.weak_bg_fill = c.menu_hover_bg;
        v.widgets.open.bg_stroke = Stroke::new(1.0, c.menu_hover_border);
        v.widgets.open.fg_stroke = Stroke::new(1.0, c.menu_hover_text);
        v.widgets.open.corner_radius = r0;
        // Bildlaufleisten: Spur `scrollbar_bg`, Griff `scrollbar_thumb` (Textfelder behalten mit
        // `text_edit_bg_color` ihren eigenen Grund)
        scrollbar_widgets(&mut v, c);
        v
    }

    /// Wendet das Design auf den egui-Kontext an (Visuals, Textgrößen, Abstände) und legt es als
    /// aktives Design ab.
    pub fn apply(self: &Arc<Self>, ctx: &egui::Context) {
        let theme = Arc::clone(self);
        let visuals = theme.visuals();
        ctx.all_styles_mut(|style| {
            style.visuals = visuals.clone();
            use egui::TextStyle;
            style.text_styles = [
                (TextStyle::Small, theme.font(theme.fonts.small)),
                (TextStyle::Body, theme.font(theme.fonts.ui)),
                (TextStyle::Button, theme.font(theme.fonts.ui)),
                (TextStyle::Heading, theme.bold(theme.fonts.heading)),
                (
                    TextStyle::Monospace,
                    FontId::new(theme.fonts.ui, FontFamily::Monospace),
                ),
            ]
            .into();
            style.spacing.item_spacing = egui::vec2(6.0, 4.0);
            style.spacing.button_padding = egui::vec2(6.0, 2.0);
            style.spacing.interact_size = egui::vec2(20.0, theme.metrics.field_height);
            style.spacing.menu_margin = egui::Margin::same(2);
            style.spacing.window_margin = egui::Margin::same(8);
            style.spacing.combo_height = 300.0;
            style.spacing.scroll = egui::style::ScrollStyle::solid();
            style.interaction.tooltip_delay = 0.6;
        });
        ctx.data_mut(|d| d.insert_temp(egui::Id::new(THEME_KEY), theme));
    }

    /// Aktives Design aus dem egui-Kontext (Rückfall: Normal).
    pub fn current(ctx: &egui::Context) -> Arc<Theme> {
        ctx.data(|d| d.get_temp::<Arc<Theme>>(egui::Id::new(THEME_KEY)))
            .unwrap_or_else(|| Arc::new(Theme::normal()))
    }
}

const THEME_KEY: &str = "fp-ui-theme";

/// Griff der egui-Bildlaufleisten in allen Listen sichtbar machen (Befund NZ-2, N24): egui zeichnet
/// die feste Leiste mit `extreme_bg_color` als Spur und `widgets.{inactive,hovered,active}.bg_fill`
/// als Griff. Beides war `field_bg`, der Griff also erst unter der Maus zu sehen. Jetzt gelten
/// app-weit die Farben der Leinwand-Bildlaufleisten (`scrollbar_bg`/`scrollbar_thumb`), unter der
/// Maus bzw. beim Ziehen etwas kräftiger (heller in Dunkel/OLED).
///
/// `extreme_bg_color` nutzt sonst nur der egui-Fortschrittsbalken als Grund; `bg_fill` nur
/// egui-eigene Kästchen, Schieber und Farbknöpfe – diese wenigen Stellen zeichnen über
/// [`with_field_widgets`] weiter im Feldstil. Knöpfe und Kombifelder nutzen `weak_bg_fill`.
pub fn scrollbar_widgets(v: &mut Visuals, c: &Palette) {
    v.extreme_bg_color = c.scrollbar_bg;
    v.widgets.inactive.bg_fill = c.scrollbar_thumb;
    v.widgets.hovered.bg_fill = mix(c.scrollbar_thumb, c.text, 0.3);
    v.widgets.active.bg_fill = mix(c.scrollbar_thumb, c.text, 0.5);
}

/// Feldstil für egui-eigene Kästchen, Schieber und Farbknöpfe (`bg_fill` wie vor
/// [`scrollbar_widgets`]): Grund `field_bg`, unter der Maus `button_hover_bg`, gedrückt
/// `button_pressed_bg`. Gilt nur für `ui`.
pub fn field_widgets(ui: &mut egui::Ui) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let w = &mut ui.visuals_mut().widgets;
    w.inactive.bg_fill = c.field_bg;
    w.hovered.bg_fill = c.button_hover_bg;
    w.active.bg_fill = c.button_pressed_bg;
}

/// Zeichnet `add` im Feldstil ([`field_widgets`]), z. B. ein `egui::Checkbox`.
pub fn with_field_widgets<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    ui.scope(|ui| {
        field_widgets(ui);
        add(ui)
    })
    .inner
}

/// Farbe mit Deckkraft (0..255) für Schein-Effekte.
pub fn with_alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

/// Mischt zwei Farben (t = 0 → a, t = 1 → b).
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| {
        (x as f32 + (y as f32 - x as f32) * t)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color32::from_rgba_unmultiplied(
        l(a.r(), b.r()),
        l(a.g(), b.g()),
        l(a.b(), b.b()),
        l(a.a(), b.a()),
    )
}

// ---------------------------------------------------------------------- Schriften

/// Systemschriften für die Oberfläche (Name in egui, Datei in `%WINDIR%\Fonts`). Segoe UI deckt
/// Latein, Kyrillisch und Griechisch ab; die CJK-Schriften sind Rückfallschriften für Chinesisch,
/// Japanisch und Koreanisch (Oberflächentexte weiterer Sprachen, Ebenen-/Seiten-/Dateinamen).
#[cfg(not(target_os = "macos"))]
const SYSTEM_FONTS: &[(&str, &str)] = &[
    ("segoe", "segoeui.ttf"),
    ("segoe-bold", "segoeuib.ttf"),
    ("segoe-symbol", "seguisym.ttf"),
    ("cjk-zh", "msyh.ttc"),
    ("cjk-ja", "YuGothM.ttc"),
    ("cjk-ko", "malgun.ttf"),
];

/// macOS: dieselben Rollen mit den Systemschriften von macOS (absolute Pfade; fehlende Dateien
/// werden übersprungen, dann bleibt die egui-Standardschrift).
#[cfg(target_os = "macos")]
const SYSTEM_FONTS: &[(&str, &str)] = &[
    ("segoe", "/System/Library/Fonts/Supplemental/Arial.ttf"),
    (
        "segoe-bold",
        "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
    ),
    ("segoe-symbol", "/System/Library/Fonts/Apple Symbols.ttf"),
    ("cjk-zh", "/System/Library/Fonts/Hiragino Sans GB.ttc"),
    ("cjk-ja", "/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc"),
    ("cjk-ko", "/System/Library/Fonts/AppleSDGothicNeo.ttc"),
];

/// Einmal je Prozess gelesene Schriftdateien (gemeinsam für alle egui-Kontexte; die CJK-Dateien
/// sind zusammen etwa 47 MB groß).
fn system_fonts() -> &'static Vec<(&'static str, Arc<egui::FontData>)> {
    static FONTS: std::sync::OnceLock<Vec<(&'static str, Arc<egui::FontData>)>> =
        std::sync::OnceLock::new();
    FONTS.get_or_init(|| {
        let dir = std::env::var_os("WINDIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from(r"C:\Windows"))
            .join("Fonts");
        // als `'static` geliehen: epaint kopiert eigene (`Owned`) Schriftdaten in jeden egui-Kontext
        // (bei ≈ 50 MB je Kontext liefen die Gesamttests mit vielen Kontexten in Speichermangel);
        // die Dateien leben ohnehin bis zum Programmende
        SYSTEM_FONTS
            .iter()
            .filter_map(|(name, file)| {
                std::fs::read(dir.join(file)).ok().map(|b| {
                    (
                        *name,
                        Arc::new(egui::FontData::from_static(Box::leak(b.into_boxed_slice()))),
                    )
                })
            })
            .collect()
    })
}

/// Liest die Schriftdateien der Oberfläche schon vorab (Programmstart, im Hintergrund parallel
/// zur Fenster- und Grafik-Einrichtung; N30). [`install_fonts`] wartet sonst darauf.
pub fn prewarm_fonts() {
    let _ = system_fonts();
}

/// Reihenfolge der CJK-Rückfallschriften: die Schrift der aktuellen Sprache zuerst (gleiche
/// Unicode-Zeichen haben im Chinesischen, Japanischen und Koreanischen verschiedene Glyphenformen).
pub fn cjk_fallback_order(lang: &str) -> [&'static str; 3] {
    let base = lang.split(['-', '_']).next().unwrap_or("").to_lowercase();
    match base.as_str() {
        "ja" => ["cjk-ja", "cjk-zh", "cjk-ko"],
        "ko" => ["cjk-ko", "cjk-zh", "cjk-ja"],
        _ => ["cjk-zh", "cjk-ja", "cjk-ko"],
    }
}

/// Lädt Segoe UI (normal und fett) zur Laufzeit aus `%WINDIR%\Fonts`, dazu Segoe UI Symbol und die
/// CJK-Rückfallschriften (Microsoft YaHei, Yu Gothic, Malgun Gothic), soweit vorhanden. Fehlt Segoe
/// UI, bleibt die egui-Standardschrift; die fette Familie fällt dann auf die normale zurück. Die
/// Reihenfolge der CJK-Schriften richtet sich nach der aktuellen Sprache (beim Sprachwechsel neu
/// aufrufen).
///
/// Grenze: Schriften von rechts nach links (Arabisch, Hebräisch, Persisch) und Schriften, die
/// Glyphen-Formung brauchen (z. B. Devanagari, Thai), setzt egui 0.36 nicht richtig (kein Bidi,
/// kein Shaping); dafür werden bewusst keine Rückfallschriften geladen (siehe `language.rs`,
/// `crates\fp-i18n\README.md`).
pub fn install_fonts(ctx: &egui::Context) {
    let mut defs = egui::FontDefinitions::default();
    for (name, data) in system_fonts() {
        defs.font_data.insert((*name).to_string(), Arc::clone(data));
    }
    let mut prop: Vec<String> = Vec::new();
    if defs.font_data.contains_key("segoe") {
        prop.push("segoe".into());
    }
    let mut bold_list: Vec<String> = Vec::new();
    if defs.font_data.contains_key("segoe-bold") {
        bold_list.push("segoe-bold".into());
    }
    let base: Vec<String> = defs
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let mut proportional = prop.clone();
    proportional.extend(base.iter().cloned());
    if defs.font_data.contains_key("segoe-symbol") {
        proportional.push("segoe-symbol".into());
    }
    for f in cjk_fallback_order(&fp_i18n::current_language()) {
        if defs.font_data.contains_key(f) {
            proportional.push(f.into());
        }
    }
    let mut bold_family = bold_list;
    bold_family.extend(proportional.iter().cloned());
    defs.families.insert(FontFamily::Proportional, proportional);
    defs.families
        .insert(FontFamily::Name(BOLD_FAMILY.into()), bold_family);
    ctx.set_fonts(defs);
}

/// Fonts für Tests: nur die egui-Standardschriften, aber mit der fetten Familie (sonst panikt egui
/// bei unbekannter Familie).
pub fn install_fallback_fonts(ctx: &egui::Context) {
    let mut defs = egui::FontDefinitions::default();
    let base: Vec<String> = defs
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    defs.families
        .insert(FontFamily::Name(BOLD_FAMILY.into()), base);
    ctx.set_fonts(defs);
}

// ---------------------------------------------------------------------- Dialoge (DES-1)

/// Farben und Schatten des Dialograhmens (DES-1, `docs\DESIGN.md` §6/§11), aus der Palette
/// abgeleitet; keine eigenen Farbwerte.
impl Theme {
    /// Titelleiste der Dialoge als Verlauf (oben, unten). Normal: hell auf Rahmengrau, Dunkel:
    /// deutlich dunkler als die Dialogfläche (vorher kaum Kontrast), OLED: schwarz.
    pub fn dialog_title_gradient(&self) -> (Color32, Color32) {
        let c = &self.colors;
        match self.kind {
            ThemeKind::Normal => (
                mix(c.app_bar_bg, c.field_bg, 0.55),
                mix(c.app_bar_bg, c.field_bg, 0.18),
            ),
            ThemeKind::Dark => (
                mix(c.app_bar_bg, c.canvas_bg, 0.2),
                mix(c.app_bar_bg, c.canvas_bg, 0.6),
            ),
            ThemeKind::Oled => (c.app_bar_bg, c.app_bar_bg),
        }
    }

    /// Linie unter der Titelleiste der Dialoge.
    pub fn dialog_title_line(&self) -> Color32 {
        let c = &self.colors;
        match self.kind {
            ThemeKind::Normal => c.line_mid,
            ThemeKind::Dark => c.line_dark,
            ThemeKind::Oled => c.focus,
        }
    }

    /// Fläche der Fußleiste hinter der unteren Knopfzeile eines Dialogs.
    pub fn dialog_footer_bg(&self) -> Color32 {
        let c = &self.colors;
        match self.kind {
            ThemeKind::Normal => mix(c.frame_bg, c.header_bottom, 0.14),
            ThemeKind::Dark => mix(c.frame_bg, c.canvas_bg, 0.45),
            ThemeKind::Oled => mix(c.frame_bg, c.list_hover_bg, 0.5),
        }
    }

    /// Geätzte Linie über der Fußleiste: dunkle Linie und (außer OLED) Lichtkante darunter.
    pub fn dialog_footer_lines(&self) -> (Color32, Option<Color32>) {
        let c = &self.colors;
        match self.kind {
            ThemeKind::Normal => (mix(c.line_mid, c.frame_bg, 0.45), Some(c.line_light)),
            ThemeKind::Dark => (
                c.line_dark,
                Some(mix(c.line_light, self.dialog_footer_bg(), 0.5)),
            ),
            ThemeKind::Oled => (mix(c.line_dark, c.focus, 0.3), None),
        }
    }

    /// Schatten um den Dialog (OLED: Akzentschein statt Schatten).
    pub fn dialog_shadow(&self) -> egui::Shadow {
        match self.kind {
            ThemeKind::Normal => egui::Shadow {
                offset: [0, 6],
                blur: 20,
                spread: 0,
                color: Color32::from_black_alpha(70),
            },
            ThemeKind::Dark => egui::Shadow {
                offset: [0, 6],
                blur: 22,
                spread: 0,
                color: Color32::from_black_alpha(160),
            },
            ThemeKind::Oled => egui::Shadow {
                offset: [0, 0],
                blur: 18,
                spread: 1,
                color: with_alpha(self.colors.focus, 46),
            },
        }
    }

    /// Fläche des Standardknopfs (OK): leicht in der Fokusfarbe getönt, beim Überfahren stärker.
    pub fn default_button_fill(&self, hovered: bool) -> Color32 {
        let c = &self.colors;
        let base = if hovered && self.kind != ThemeKind::Oled {
            c.button_hover_bg
        } else {
            c.button_bg
        };
        let t = match (self.kind, hovered) {
            (ThemeKind::Normal, false) => 0.12,
            (ThemeKind::Normal, true) => 0.2,
            (ThemeKind::Dark, false) => 0.28,
            (ThemeKind::Dark, true) => 0.4,
            (ThemeKind::Oled, false) => 0.2,
            (ThemeKind::Oled, true) => 0.32,
        };
        mix(base, c.focus, t)
    }

    /// Gruppenrahmen in Dialogen: dunkle Linie und (außer OLED) Lichtkante.
    pub fn group_box_lines(&self) -> (Color32, Option<Color32>) {
        let c = &self.colors;
        match self.kind {
            ThemeKind::Normal => (mix(c.line_mid, c.frame_bg, 0.35), Some(c.line_light)),
            ThemeKind::Dark => (c.line_dark, Some(c.line_light)),
            ThemeKind::Oled => (c.line_dark, None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oled_is_pure_black_and_colorful() {
        let t = Theme::oled();
        assert_eq!(t.colors.frame_bg, Color32::from_rgb(0, 0, 0));
        assert_eq!(t.colors.canvas_bg, Color32::from_rgb(0, 0, 0));
        assert!(t.colorful_icons);
        // alle sechs Gruppenfarben verschieden
        for i in 0..6 {
            for j in (i + 1)..6 {
                assert_ne!(t.colors.group_accent[i], t.colors.group_accent[j]);
            }
        }
    }

    #[test]
    fn normal_matches_original_greys() {
        let t = Theme::normal();
        assert_eq!(t.colors.frame_bg, Color32::from_rgb(214, 214, 214));
        assert_eq!(t.colors.canvas_bg, Color32::from_rgb(200, 200, 200));
        assert_eq!(t.colors.list_selected_bg, Color32::from_rgb(168, 198, 237));
    }

    #[test]
    fn cjk_order_follows_language() {
        assert_eq!(cjk_fallback_order("ja")[0], "cjk-ja");
        assert_eq!(cjk_fallback_order("ko-KR")[0], "cjk-ko");
        assert_eq!(cjk_fallback_order("zh-TW")[0], "cjk-zh");
        assert_eq!(cjk_fallback_order("de")[0], "cjk-zh");
        // jede Schrift genau einmal
        for l in ["ja", "ko", "zh", "de"] {
            let o = cjk_fallback_order(l);
            assert!(o.contains(&"cjk-ja") && o.contains(&"cjk-ko") && o.contains(&"cjk-zh"));
        }
    }

    #[test]
    fn theme_kind_roundtrip_engine() {
        for k in ThemeKind::ALL {
            assert_eq!(ThemeKind::from_engine(k.to_engine()), k);
        }
    }
}

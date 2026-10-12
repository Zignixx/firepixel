//! Befehlskatalog aus der Spezifikation 10 (`re\werkzeuge\menuebefehle.json`).
//!
//! Die Tabelle `data/commands.json` erzeugt `tools\golden\fp-engine\gen_data.py`: je Hauptmenübefehl
//! (260) und je Tastenkürzel ohne Menüeintrag (20) der stabile Befehlsname, die Original-Befehls-ID,
//! der `cmd_`-Name (Schlüssel der Tastenkürzel-Sets), der deutsche Menüpfad, der Statuszeilentext und
//! die JS-Entsprechungen. Das Register ([`crate::command::CommandRegistry`]) ergänzt damit die
//! Beschreibung jedes Befehls und registriert für alle nicht umgesetzten Katalogeinträge Platzhalter.

use serde::Deserialize;
use std::sync::OnceLock;

/// Ein Eintrag des Befehlskatalogs.
#[derive(Clone, Debug, Deserialize)]
pub struct CatalogEntry {
    /// Original-Befehls-ID (`None` nur bei `cmd_ViewEditGuides`).
    pub fw_id: Option<u32>,
    /// `cmd_`-Name aus `MENU_DESCRIPTION_MAP` (Schlüssel der Tastenkürzel-Sets).
    pub cmd: Option<String>,
    /// Stabiler Befehlsname, z. B. `modify.arrange.bringToFront`.
    pub name: String,
    /// Deutscher Menüpfad (leer bei Kürzeln ohne Menüeintrag).
    pub path_de: Vec<String>,
    /// Englischer interner Pfad (`Modify::Arrange::Bring to Front`).
    pub path_en: Option<String>,
    /// Statuszeilentext (lange Form).
    pub status_de: Option<String>,
    /// Beschriftung ohne „…“.
    pub label_de: String,
    /// Menütext wie im Menü (mit „…“ bei Dialogen).
    pub menu_label_de: String,
    /// Steht im Hauptmenü.
    pub main_menu: bool,
    /// JS-API-Entsprechungen (03).
    #[serde(default)]
    pub js: Vec<String>,
    /// Undo-Klassen, die das Original im Aufrufpfad anlegt.
    #[serde(default)]
    pub undo_classes: Vec<String>,
    /// Tastenkürzel im deutschen Standardset „Firepixel“ (Anzeigetexte).
    #[serde(default)]
    pub shortcuts_de: Vec<String>,
    /// Menü-Platzhalter ohne eigenen Befehl (35343, 35848).
    #[serde(default)]
    pub placeholder: bool,
}

/// Eintrag der Bedienfeld-Tabelle (10 §4.5).
#[derive(Clone, Debug, Deserialize)]
pub struct PanelEntry {
    /// Befehls-ID des Bedienfelds (alter Schlüssel `befehl_id` wird weiter gelesen).
    #[serde(alias = "befehl_id")]
    pub command_id: u32,
    /// JS-Name für `fw.toggleFloater`.
    pub js_floater_name: Option<String>,
    /// `cmd_`-Name.
    pub cmd_name: Option<String>,
}

#[derive(Deserialize)]
struct CatalogFile {
    commands: Vec<CatalogEntry>,
    #[serde(default)]
    panels: Vec<PanelEntry>,
}

static CATALOG: OnceLock<CatalogFile> = OnceLock::new();

fn file() -> &'static CatalogFile {
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("../data/commands.json"))
            .unwrap_or(CatalogFile { commands: Vec::new(), panels: Vec::new() })
    })
}

/// Alle Katalogeinträge (260 Hauptmenübefehle, danach 20 Kürzel ohne Menüeintrag).
pub fn entries() -> &'static [CatalogEntry] {
    &file().commands
}

/// Bedienfelder mit Befehls-ID und JS-Namen.
pub fn panels() -> &'static [PanelEntry] {
    &file().panels
}

/// Eintrag zu einem Befehlsnamen.
pub fn by_name(name: &str) -> Option<&'static CatalogEntry> {
    entries().iter().find(|e| e.name == name)
}

/// Eintrag zu einer Original-Befehls-ID.
pub fn by_fw_id(id: u32) -> Option<&'static CatalogEntry> {
    entries().iter().find(|e| e.fw_id == Some(id))
}

/// Eintrag zu einem `cmd_`-Namen.
pub fn by_cmd(cmd: &str) -> Option<&'static CatalogEntry> {
    entries().iter().find(|e| e.cmd.as_deref() == Some(cmd))
}

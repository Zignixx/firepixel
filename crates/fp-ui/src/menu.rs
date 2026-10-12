//! Menüs: Hauptmenü (Ressource MENU 128 des Originals mit Trennern und Reihenfolge, Zustände
//! und Kürzel aus dem Befehlsregister von fp-engine), Laufzeit-Ergänzungen wie in CS6 (zuletzt
//! geöffnete Dateien, Fenster-Liste, Flash-Bedienfelder, Filter- und Befehle-Untermenüs) und die
//! Bedienfeldmenüs (Kontextmenü-Ressourcen).
//!
//! Die Menüdaten stehen in `data/menu_main.json` (aus `re\inventory\resources_de.json`, nur Texte,
//! IDs und Trenner). Ein Eintrag führt den Befehl aus, dessen Original-ID im Register steht
//! ([`fp_engine::CommandRegistry::by_fw_id`]); fehlt der Befehl, ist der Eintrag gesperrt.
//!
//! # Sprachen
//! Jeder Eintrag der Menüdaten trägt einen Schlüssel (`key`) aus `locales\<sprache>\menue.ftl`:
//! Befehle `cmd-…` ([`fp_i18n::command_key`], Wert = Menütext mit `&`), Untermenüs und Einträge
//! an anderer Stelle `menu-…`, Kontextmenüs `ctx-…`, Bedienfeldmenüs `pmenu-…`. Angezeigt wird
//! [`MenuEntry::local_text`]; `text` bleibt der deutsche Ausgangstext (Kürzel nach dem Tabulator).
//! Die Menüs werden in jedem Bild neu gebildet, ein Sprachwechsel wirkt sofort.

use crate::app_state::{Action, AppState};
use crate::panels::PanelId;
use crate::theme::{with_alpha, Theme, ThemeKind};
use egui::containers::menu::{MenuConfig, SubMenu};
use egui::{pos2, vec2, Align2, Popup, Rect, Response, Sense, Stroke, Ui, UiKind, UiStackInfo};
use fp_i18n::t;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::OnceLock;

/// Ein Eintrag der Menüressource.
#[derive(Clone, Debug, Deserialize)]
pub struct MenuEntry {
    /// Deutscher Ausgangstext (mit `&` und ggf. Kürzel nach `\t`).
    #[serde(default)]
    pub text: String,
    /// Schlüssel des Texts in `menue.ftl` (leer = `text` unverändert anzeigen).
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub id: Option<u32>,
    #[serde(default)]
    pub sep: bool,
    #[serde(default)]
    pub children: Option<Vec<MenuEntry>>,
}

impl MenuEntry {
    /// Text in der aktuellen Sprache, mit Zugriffstaste (`&`) und ggf. Kürzel nach dem Tabulator.
    pub fn local_text(&self) -> String {
        let (de, sc) = match self.text.split_once('\t') {
            Some((a, b)) => (a, Some(b)),
            None => (self.text.as_str(), None),
        };
        let base = if self.key.is_empty() { de.to_string() } else { t(&self.key) };
        match sc {
            Some(sc) => format!("{base}\t{}", fp_i18n::shortcut_label(sc)),
            None => base,
        }
    }
}

/// Tastenkürzel in der aktuellen Sprache (`Strg+Umschalt+S` bzw. `Ctrl+Shift+S`).
pub fn shortcut_text(sc: &fp_engine::shortcuts::Shortcut) -> String {
    // Umlaut-Tasten klein wie im Original („Strg+ü“), Buchstaben A–Z groß
    let de = sc.display_de();
    let de = match de.rsplit_once('+') {
        Some((mods, k)) if k.chars().count() == 1 && k.chars().all(|c| c.is_alphabetic() && !c.is_ascii()) => format!("{mods}+{}", k.to_lowercase()),
        _ => de,
    };
    fp_i18n::shortcut_label(&de)
}

/// Menütext ohne Zugriffstaste, Kürzel und abschließendes „…“/„...“ (Beschriftung).
pub fn plain_label(menu_text: &str) -> String {
    let s = clean_label(menu_text).0;
    let s = s.trim_end();
    let s = s.strip_suffix('…').or_else(|| s.strip_suffix("...")).unwrap_or(s);
    s.trim_end().to_string()
}

/// Text eines Schlüssels, `None` wenn es ihn in keiner Sprache gibt.
fn t_opt(key: &str) -> Option<String> {
    let s = t(key);
    (s != key).then_some(s)
}

/// Beschriftung eines Befehls in der aktuellen Sprache (ohne `&` und „…“), z. B. für
/// Werkzeugtipps, die Kürzelliste und Verlaufsschritte: `cmd-….label`, sonst der Menütext
/// `cmd-…` bereinigt, sonst die deutsche Beschriftung aus dem Register.
pub fn command_label(st: &AppState, name: &str) -> String {
    let key = fp_i18n::command_key(name);
    if let Some(s) = t_opt(&format!("{key}.label")) {
        return s;
    }
    if let Some(s) = t_opt(&key) {
        return plain_label(&s);
    }
    st.session.registry().get(name).map(|c| c.info().label.clone()).unwrap_or_else(|| name.to_string())
}

/// Statuszeilen-/Hilfetext eines Befehls in der aktuellen Sprache (`cmd-….description`).
pub fn command_description(st: &AppState, name: &str) -> Option<String> {
    t_opt(&format!("{}.description", fp_i18n::command_key(name))).or_else(|| st.session.registry().get(name).and_then(|c| c.info().description.clone()))
}

/// Menüpfad eines Befehls in der aktuellen Sprache („Modifizieren ▸ Ausrichten ▸ Links“), wie er
/// im Hauptmenü steht; Befehle außerhalb des Hauptmenüs → ihre Beschriftung.
pub fn command_path(st: &AppState, name: &str) -> String {
    fn walk(st: &AppState, entries: &[MenuEntry], name: &str, prefix: &mut Vec<String>) -> Option<String> {
        for e in entries {
            if e.sep {
                continue;
            }
            let label = clean_label(&e.local_text()).0;
            if let Some(c) = &e.children {
                prefix.push(label);
                if let Some(p) = walk(st, c, name, prefix) {
                    return Some(p);
                }
                prefix.pop();
            } else if e.id.and_then(|id| st.session.registry().by_fw_id(id)).is_some_and(|c| c.info().name == name) {
                let mut parts = prefix.clone();
                parts.push(label);
                return Some(parts.join(" ▸ "));
            }
        }
        None
    }
    walk(st, main_menu(), name, &mut Vec::new()).unwrap_or_else(|| command_label(st, name))
}

#[derive(Deserialize)]
struct MenuFile {
    menu: Vec<MenuEntry>,
    #[serde(default)]
    context: std::collections::BTreeMap<String, Vec<MenuEntry>>,
}

fn file() -> &'static MenuFile {
    static F: OnceLock<MenuFile> = OnceLock::new();
    F.get_or_init(|| {
        serde_json::from_str(include_str!("../data/menu_main.json")).unwrap_or(MenuFile { menu: Vec::new(), context: Default::default() })
    })
}

/// Hauptmenü wie in der Ressource.
pub fn main_menu() -> &'static [MenuEntry] {
    &file().menu
}

/// Kontextmenü-Ressource (Bedienfeldmenüs), z. B. `"6923"` für Ebenen.
pub fn context_menu(id: &str) -> Option<&'static [MenuEntry]> {
    file().context.get(id).and_then(|v| v.first()).and_then(|m| m.children.as_deref())
}

/// Menütext ohne `&`-Zugriffstaste und ohne Kürzel nach dem Tabulator.
pub fn clean_label(text: &str) -> (String, Option<String>) {
    let (label, sc) = match text.split_once('\t') {
        Some((a, b)) => (a, Some(b.to_string())),
        None => (text, None),
    };
    let mut out = String::with_capacity(label.len());
    let mut chars = label.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            if chars.peek() == Some(&'&') {
                out.push('&');
                chars.next();
            }
            continue;
        }
        out.push(c);
    }
    (out, sc)
}

// ---------------------------------------------------------------------- Aufbereiteter Menübaum

/// Was ein Menüeintrag tut.
#[derive(Clone, Debug, PartialEq)]
pub enum ItemAction {
    /// Befehl aus dem Register.
    Command(String),
    /// Aktion der Oberfläche.
    Ui(Action),
    /// Ohne Wirkung (gesperrt).
    None,
}

/// Aufbereiteter Menüknoten für die Anzeige.
#[derive(Clone, Debug)]
pub enum Node {
    Separator,
    Item { label: String, shortcut: Option<String>, action: ItemAction, enabled: bool, checked: Option<bool> },
    Sub { label: String, children: Vec<Node>, enabled: bool },
}

fn item(label: &str, action: ItemAction, enabled: bool, checked: Option<bool>) -> Node {
    Node::Item { label: label.to_string(), shortcut: None, action, enabled, checked }
}

/// Zustand eines Befehls für das Menü: (aktiv, Häkchen, Kürzel).
pub(crate) fn command_state(st: &AppState, name: &str, fallback_sc: Option<String>) -> (bool, Option<bool>, Option<String>) {
    let sc = menu_shortcut(st, name).map(|s| shortcut_text(&s)).or(fallback_sc);
    if let Some((en, ck)) = crate::actions::ui_command_state(st, name) {
        return (en, ck, sc);
    }
    let (en, ck) = st.session.command_state(name);
    (en, ck, sc)
}

/// Kürzel, das ein Menüeintrag zeigt: das erste des Satzes, aber wie im Original Entf vor der
/// Rücktaste (z. B. „Löschen“ mit Entf und Rück → Entf).
pub fn menu_shortcut(st: &AppState, name: &str) -> Option<fp_engine::shortcuts::Shortcut> {
    const VK_BACK: u16 = 8;
    let all = st.session.command_shortcuts(name);
    all.iter().find(|s| s.vk != VK_BACK).or(all.first()).copied()
}

/// „Rückgängig“/„Wiederholen“ mit dem Namen des Verlaufsschritts wie im Original
/// („Rückgängig: Auswahl aufheben“).
fn with_step_name(st: &AppState, name: &str, label: String) -> String {
    let doc = st.session.active();
    let step = match name {
        "edit.undo" => doc.and_then(|d| d.history.undo_label()).map(|s| s.to_string()),
        // ohne zurückgenommenen Schritt wiederholt das Original den letzten Befehl
        "edit.redo" => return with_redo_step(st, label),
        // „Filter wiederholen: <zuletzt benutzter Filter>“ (PM-7, MN-4)
        "filter.repeat" => st.session.last_filter.as_ref().map(|f| plain_label(&t(&fp_i18n::command_key(&f.command)))),
        _ => None,
    };
    match step {
        Some(s) if !s.is_empty() => fp_i18n::tr!("menu-edit-withStep", label = label, step = s),
        _ => label,
    }
}

fn with_redo_step(st: &AppState, label: String) -> String {
    match st.session.undo_redo_labels().1 {
        Some(s) if !s.is_empty() => fp_i18n::tr!("menu-edit-withStep", label = label, step = s),
        _ => label,
    }
}

/// Datei ▸ Vorschau im Browser wie im Original mit dem Programmnamen des Browsers
/// („Vorschau in iexplore.exe“); ohne Sekundärbrowser nur „Vorschau in“ (gesperrt).
fn browser_label(st: &AppState, name: &str, label: String, enabled: bool) -> (String, bool) {
    let (idx, key) = match name {
        "file.previewInBrowser.previewInPrimaryBrowser" => (0, "menu-file-previewIn"),
        "file.previewInBrowser.previewInSecondaryBrowser" => (1, "menu-file-previewIn"),
        "file.previewInBrowser.previewAllPagesInPrimaryBrowser" => (0, "menu-file-previewAllIn"),
        _ => return (label, enabled),
    };
    let prog = st.session.ui.browsers.get(idx).cloned().flatten();
    let browser = match &prog {
        Some(p) => std::path::Path::new(p).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| p.clone()),
        None if idx == 0 => t("menu-file-defaultBrowser"),
        None => String::new(),
    };
    let text = fp_i18n::tr!(key, browser = browser).trim_end().to_string();
    (text, enabled && (idx == 0 || prog.is_some()))
}

/// Schlüssel der Untermenüs, die zur Laufzeit gefüllt werden.
const KEY_RECENT: &str = "menu-file-openRecent";
const KEY_FONT: &str = "menu-text-font";
const KEY_EXTENSIONS: &str = "menu-window-extensions";

fn convert(st: &AppState, entries: &[MenuEntry]) -> Vec<Node> {
    let mut out = Vec::new();
    for e in entries {
        if e.sep {
            out.push(Node::Separator);
            continue;
        }
        let (label, sc) = clean_label(&e.local_text());
        if let Some(children) = &e.children {
            let mut kids = convert(st, children);
            // Laufzeit-Untermenüs
            match e.key.as_str() {
                KEY_RECENT => kids = recent_files(st),
                KEY_FONT => kids = font_menu(st),
                KEY_EXTENSIONS => kids = vec![item(&t("menu-window-extensions-none"), ItemAction::None, false, None)],
                _ => {}
            }
            // Untermenü-Köpfe sind im Original immer aktiv, auch wenn alles darin grau ist (PM-7)
            out.push(Node::Sub { label, children: kids, enabled: true });
            continue;
        }
        let Some(id) = e.id else {
            out.push(item(&label, ItemAction::None, false, None));
            continue;
        };
        let name = st.session.registry().by_fw_id(id).map(|c| c.info().name.clone());
        match name {
            Some(n) => {
                let (enabled, checked, shortcut) = command_state(st, &n, sc);
                let label = with_step_name(st, &n, label);
                let (label, enabled) = browser_label(st, &n, label, enabled);
                out.push(Node::Item { label, shortcut, action: ItemAction::Command(n), enabled, checked });
            }
            None => out.push(Node::Item { label, shortcut: sc, action: ItemAction::None, enabled: false, checked: None }),
        }
    }
    out
}

/// Zuletzt benutzte Schriften (Voreinstellung `RecentFontList`, Form „(A)(B)…“).
fn recent_fonts(st: &AppState) -> Vec<String> {
    if !st.session.prefs.get_bool("ShowRecentFonts", true) {
        return Vec::new();
    }
    let limit = st.session.prefs.get_int("RecentFontLimit", 5).clamp(0, 50) as usize;
    let list = st.session.prefs.get_str("RecentFontList", "");
    list.split(')').filter_map(|p| p.trim().strip_prefix('(')).filter(|s| !s.is_empty()).map(str::to_string).take(limit).collect()
}

/// Merkt eine im Menü gewählte Schrift als zuletzt benutzt (vorn in `RecentFontList`).
pub fn note_font_choice(st: &mut AppState, a: &Action) {
    let Action::Run(name, p) = a else { return };
    if name != "text.font" {
        return;
    }
    let Some(ps) = p.get("font").and_then(|v| v.as_str()) else { return };
    let fams = crate::inspector::fonts::families();
    let Some(fam) = fams.iter().find(|f| f.styles.iter().any(|s| s.1 == ps) || f.name == ps).map(|f| f.name.clone()) else { return };
    let mut list = recent_fonts(st);
    list.retain(|n| *n != fam);
    list.insert(0, fam);
    list.truncate(st.session.prefs.get_int("RecentFontLimit", 5).clamp(1, 50) as usize);
    let s: String = list.iter().map(|n| format!("({n})")).collect();
    st.session.prefs.set("RecentFontList", fp_engine::prefs::PrefValue::Str(s));
}

/// Aktuelle Schrift für das Häkchen in Text ▸ Schrift: die des ersten ausgewählten Texts, sonst
/// die Vorgabe des Textwerkzeugs (PostScript- oder Familienname).
pub fn current_font(st: &AppState) -> String {
    let from_text = st.session.active().and_then(|d| {
        d.selection.elements.iter().find_map(|id| {
            let e = fp_engine::docops::element_on_page(&d.doc, d.page, *id)?;
            match &e.kind {
                fp_model::element::ElementKind::Text(t) => t.resolved_runs().first().and_then(|(a, _)| a.font.clone()).or_else(|| t.initial.font.clone()),
                _ => None,
            }
        })
    });
    from_text.unwrap_or_else(|| st.session.tools.options.text_font.clone())
}

/// Text ▸ Schrift wie im Original: oben die zuletzt benutzten Schriften (Voreinstellung
/// `RecentFontList`), Trennlinie, dann alle Familien **flach** (lange Liste mit Bildlauf, siehe
/// [`show_nodes`]); Familien mit mehreren Schnitten zeigen die Schnitte als Untermenü. Die aktuelle
/// Schrift trägt ein Häkchen. Die Einträge sind auch ohne Text aktiv: dann setzt `text.font` die
/// Vorgabe des Textwerkzeugs.
fn font_menu(st: &AppState) -> Vec<Node> {
    let enabled = st.session.is_available("text.font");
    let fams = crate::inspector::fonts::families();
    let cur = current_font(st);
    let pick = |ps: &str| ItemAction::Ui(Action::Run("text.font".into(), json!({ "font": ps })));
    let entry = |f: &crate::inspector::fonts::Family| {
        let is_family = f.name.eq_ignore_ascii_case(&cur);
        if f.styles.len() > 1 {
            let base = crate::inspector::fonts::base_style(f);
            let children = f
                .styles
                .iter()
                .map(|(style, ps)| {
                    let on = ps.eq_ignore_ascii_case(&cur) || (is_family && *style == base);
                    item(style, pick(ps), enabled, Some(on))
                })
                .collect();
            return Node::Sub { label: f.name.clone(), children, enabled: true };
        }
        let ps = f.styles.first().map(|s| s.1.clone()).unwrap_or_else(|| f.name.clone());
        let on = is_family || ps.eq_ignore_ascii_case(&cur);
        item(&f.name, pick(&ps), enabled, Some(on))
    };
    let mut out: Vec<Node> = Vec::new();
    let recent: Vec<Node> = recent_fonts(st).iter().filter_map(|n| fams.iter().find(|f| f.name.eq_ignore_ascii_case(n))).map(entry).collect();
    if !recent.is_empty() {
        out.extend(recent);
        out.push(Node::Separator);
    }
    out.extend(fams.iter().map(entry));
    out
}

fn recent_files(st: &AppState) -> Vec<Node> {
    let files = &st.session.prefs.recent_files;
    if files.is_empty() {
        return vec![item(&t("menu-file-openRecent-none"), ItemAction::None, false, None)];
    }
    files
        .iter()
        .take(10)
        .enumerate()
        .map(|(i, p)| item(&format!("{} {}", i + 1, p.display()), ItemAction::Ui(Action::OpenPath(p.clone())), true, None))
        .collect()
}

/// Filter-Untermenüs wie im Original: (Schlüssel des Untermenüs, Befehle). Angezeigt wird der
/// Menütext `cmd-…` des Befehls; fehlt ein Befehl im Register, ist der Eintrag gesperrt.
const FILTER_GROUPS: [(&str, &[&str]); 5] = [
    ("menu-filter-other", &["filter.other.convertToAlpha", "filter.other.findEdges"]),
    (
        "menu-filter-adjustColor",
        &[
            "filter.adjustColor.autoLevels",
            "filter.adjustColor.colorFill",
            "filter.adjustColor.hueSaturation",
            "filter.adjustColor.brightnessContrast",
            "filter.adjustColor.curves",
            "filter.adjustColor.levels",
            "filter.adjustColor.invert",
        ],
    ),
    ("menu-filter-noise", &["filter.noise.addNoise"]),
    ("menu-filter-sharpen", &["filter.sharpen.sharpen", "filter.sharpen.sharpenMore", "filter.sharpen.unsharpMask"]),
    ("menu-filter-blur", &["filter.blur.motion", "filter.blur.gaussian", "filter.blur.radial", "filter.blur.blurMore", "filter.blur.blur", "filter.blur.zoom"]),
];

/// Untermenü-Inhalte, die das Original zur Laufzeit aus Skripten bzw. Filtern füllt.
fn filter_submenus(st: &AppState) -> Vec<Node> {
    FILTER_GROUPS
        .iter()
        .map(|(key, items)| {
            let children: Vec<Node> = items
                .iter()
                .map(|name| {
                    let label = clean_label(&t(&fp_i18n::command_key(name))).0;
                    if st.session.registry().get(name).is_some() {
                        let (en, ck, sc) = command_state(st, name, None);
                        Node::Item { label, shortcut: sc, action: ItemAction::Command(name.to_string()), enabled: en, checked: ck }
                    } else {
                        item(&label, ItemAction::None, false, None)
                    }
                })
                .collect();
            // Kopf immer aktiv wie im Original (PM-7)
            Node::Sub { label: clean_label(&t(key)).0, children, enabled: true }
        })
        .collect()
}

/// Befehle-Menü zur Laufzeit wie im Original: der Befehlsordner (`commands_dir`, darin gespeicherte
/// und mitgelieferte `.jsf`-Befehle) wird gelesen, Unterordner werden Untermenüs; ein Klick führt das
/// Skript über fp-script aus (`commands.runScript`). Der Ordner wird höchstens alle 2 s neu gelesen.
fn command_submenus() -> Vec<Node> {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    static CACHE: Mutex<Option<(Instant, Vec<Node>)>> = Mutex::new(None);
    if let Ok(mut c) = CACHE.lock() {
        if let Some((t, nodes)) = c.as_ref()
            && t.elapsed() < Duration::from_secs(2)
        {
            return nodes.clone();
        }
        let nodes = scan_commands(&fp_engine::commands::misc::commands_dir(), 0);
        *c = Some((Instant::now(), nodes.clone()));
        return nodes;
    }
    Vec::new()
}

fn scan_commands(dir: &std::path::Path, depth: usize) -> Vec<Node> {
    let mut entries: Vec<std::path::PathBuf> = std::fs::read_dir(dir).map(|rd| rd.flatten().map(|e| e.path()).collect()).unwrap_or_default();
    entries.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default());
    let mut out = Vec::new();
    for p in entries {
        // „∕“ (U+2215) im Dateinamen steht für „/“, das Dateinamen nicht enthalten dürfen
        // („Feste Breite ein/aus“ wie im Original, MN-4)
        let name = p.file_stem().map(|n| n.to_string_lossy().replace('\u{2215}', "/")).unwrap_or_default();
        if p.is_dir() && depth < 4 {
            let children = scan_commands(&p, depth + 1);
            if !children.is_empty() {
                let fname = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or(name);
                out.push(Node::Sub { label: fname, children, enabled: true });
            }
        } else if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("jsf")) {
            out.push(item(&name, ItemAction::Ui(Action::Run("commands.runScript".into(), json!({ "path": p.to_string_lossy() }))), true, None));
        }
    }
    out
}

/// Feste Einträge am Ende des Befehle-Menüs (nicht zwischengespeichert: Sprache und Zustand).
/// „Warnmeldungen zurücksetzen“ steht wie im Original nur einmal, als mitgelieferter Befehl im
/// Befehlsordner (MN-4; vorher zusätzlich hier).
fn command_tail() -> Vec<Node> {
    let on = crate::bridge::running_url().is_some();
    vec![
        Node::Separator,
        item(&clean_label(&t("menu-commands-mcp")).0, ItemAction::Ui(Action::Run("ui.mcpToggle".into(), json!({}))), true, Some(on)),
    ]
}

fn panel_item(st: &AppState, p: PanelId) -> Node {
    // Titel kommt aus `bedienfelder.ftl` (PanelId::title)
    // wie im Original: Häkchen nur, wenn das Bedienfeld vorn in seiner sichtbaren Gruppe liegt
    let checked = st.settings.workspace.is_visible(p) && st.settings.workspace.is_front(p);
    item(p.title(), ItemAction::Ui(Action::TogglePanel(p)), true, Some(checked))
}

/// Der aufbereitete Hauptmenübaum (je Frame neu, damit Zustände stimmen).
pub fn build_main_menu(st: &AppState) -> Vec<(String, Vec<Node>)> {
    let mut out = Vec::new();
    // Menüeintrag eines Befehls, der zur Laufzeit dazukommt (Text = `cmd-…`)
    let runtime_item = |cmd: &str| {
        let (en, ck, sc) = command_state(st, cmd, None);
        Node::Item { label: clean_label(&t(&fp_i18n::command_key(cmd))).0, shortcut: sc, action: ItemAction::Command(cmd.into()), enabled: en, checked: ck }
    };
    for top in main_menu() {
        let (label, _) = clean_label(&top.local_text());
        let mut nodes = convert(st, top.children.as_deref().unwrap_or(&[]));
        match top.key.as_str() {
            "menu-view" => {
                nodes.push(Node::Separator);
                let children = ThemeKind::ALL.iter().map(|k| item(&k.name(), ItemAction::Ui(Action::SetTheme(*k)), true, Some(st.settings.theme == *k))).collect();
                nodes.push(Node::Sub { label: clean_label(&t("menu-view-theme")).0, children, enabled: true });
                nodes.push(crate::language::menu_node(st));
            }
            "menu-commands" => {
                // KI-Modelle (Agent KI) vor den Befehlen aus dem Befehlsordner
                if st.session.registry().get("ai.manageModels").is_some() {
                    nodes.push(Node::Separator);
                    nodes.push(runtime_item("ai.manageModels"));
                }
                nodes.extend(command_submenus());
                nodes.extend(command_tail());
            }
            "menu-select" => {
                // KI-Auswahl nach „Ähnliches auswählen“
                let pos = nodes.iter().position(|n| matches!(n, Node::Item { action: ItemAction::Command(c), .. } if c == "select.similar")).map(|i| i + 1).unwrap_or(nodes.len());
                let mut extra = Vec::new();
                for cmd in ["select.aiSubject", "select.aiRefineEdge"] {
                    if st.session.registry().get(cmd).is_some() {
                        extra.push(runtime_item(cmd));
                    }
                }
                for (k, n) in extra.into_iter().enumerate() {
                    nodes.insert(pos + k, n);
                }
            }
            "menu-modify" => {
                // Bitmap nachzeichnen (VT-2, ohne Original-ID) nach „Pfad ändern“; öffnet den Dialog
                let alter = clean_label(&t("menu-modify-alterPath")).0;
                let pos = nodes.iter().position(|n| matches!(n, Node::Sub { label, .. } if *label == alter)).map(|i| i + 1).unwrap_or(nodes.len());
                if st.session.registry().get("modify.traceBitmap").is_some() {
                    nodes.insert(pos, runtime_item("modify.traceBitmap"));
                }
            }
            "menu-filter" => nodes.extend(filter_submenus(st)),
            "menu-window" => {
                // Flash-Bedienfelder (Laufzeit) nach „Suchen und ersetzen“, Dokumentliste am Ende
                let pos = nodes.iter().position(|n| matches!(n, Node::Item { action: ItemAction::Command(c), .. } if c == "edit.findAndReplace")).map(|i| i + 1).unwrap_or(nodes.len());
                let extra = vec![
                    Node::Separator,
                    panel_item(st, PanelId::Align),
                    panel_item(st, PanelId::AutoShapeProperties),
                    Node::Sub { label: clean_label(&t("menu-window-other")).0, children: vec![panel_item(st, PanelId::ImageEditing), panel_item(st, PanelId::ColorPalette), panel_item(st, PanelId::SpecialCharacters)], enabled: true },
                    panel_item(st, PanelId::Path),
                    panel_item(st, PanelId::SymbolProperties),
                ];
                for (k, n) in extra.into_iter().enumerate() {
                    nodes.insert(pos + k, n);
                }
                // Arbeitsbereichlayouts (Ordner der Layouts, 11 §4.2) vor „Erweiterungen“
                let ext = clean_label(&t(KEY_EXTENSIONS)).0;
                let at = nodes.iter().position(|n| matches!(n, Node::Sub { label, .. } if *label == ext)).unwrap_or(0);
                nodes.insert(at, Node::Sub { label: clean_label(&t("menu-window-workspaceLayouts")).0, children: crate::appbar::workspace_menu(st), enabled: true });
                let docs = st.session.documents();
                if !docs.is_empty() {
                    nodes.push(Node::Separator);
                    let active = st.session.active_id();
                    for (i, d) in docs.iter().enumerate() {
                        let star = if d.is_dirty() { "*" } else { "" };
                        nodes.push(item(&format!("{} {}{}", i + 1, star, d.title), ItemAction::Ui(Action::Activate(d.id)), true, Some(active == Some(d.id))));
                    }
                }
            }
            _ => {}
        }
        out.push((label, nodes));
    }
    out
}

/// Bedienfeldmenü (≡) eines Bedienfelds: Einträge der Ressource, am Ende „Hilfe“ wie im Original.
/// „Schließen“/„Registerkartengruppe schließen“ stehen im Kontextmenü des Reiters
/// ([`build_tab_menu`], MN-3).
pub fn build_panel_menu(st: &AppState, panel: PanelId) -> Vec<Node> {
    let mut nodes = match panel.menu_resource().and_then(context_menu) {
        Some(entries) => convert(st, entries),
        None => Vec::new(),
    };
    crate::dialogs::panel_menus::adjust_menu(st, &mut nodes);
    if !nodes.is_empty() {
        nodes.push(Node::Separator);
    }
    nodes.push(help_item());
    nodes
}

fn help_item() -> Node {
    item(&t("pmenu-help"), ItemAction::Ui(Action::Run("help.firepixelHelp".into(), json!({ "topic": "panels" }))), true, None)
}

/// Kontextmenü (Rechtsklick) auf den Reiter eines Bedienfelds: Schließen, Registerkartengruppe schließen.
pub fn build_tab_menu(_st: &AppState, panel: PanelId) -> Vec<Node> {
    vec![
        item(&t("pmenu-close"), ItemAction::Ui(Action::TogglePanel(panel)), true, None),
        item(&t("pmenu-closeGroup"), ItemAction::Ui(Action::Run("ui.closeGroup".into(), json!({ "panel": format!("{panel:?}") }))), true, None),
    ]
}

/// Bedienfeldmenü (≡) des Eigenschaften-Inspektors: im Original nur „Hilfe“. Die Effektoptionen
/// (Ressource 6921) stehen im Effekt-Menü „+ ▸ Optionen“ des Inspektors.
pub fn build_inspector_menu(_st: &AppState) -> Vec<Node> {
    vec![help_item()]
}

/// Kontextmenü auf den Reitern des Inspektors: Volle/Halbe Höhe und Schließen.
pub fn build_inspector_tab_menu(st: &AppState) -> Vec<Node> {
    let half = st.settings.workspace.inspector_half;
    vec![
        item(&t("pi-menu-full-height"), ItemAction::Ui(Action::Run("ui.inspector.half".into(), json!({ "half": false }))), true, Some(!half)),
        item(&t("pi-menu-half-height"), ItemAction::Ui(Action::Run("ui.inspector.half".into(), json!({ "half": true }))), true, Some(half)),
        Node::Separator,
        item(&t("pmenu-close"), ItemAction::Command("window.properties".into()), true, None),
    ]
}

// ---------------------------------------------------------------------- Anzeige

/// Rahmen der Menü-Popups aus dem Design.
pub fn menu_frame(theme: &Theme) -> egui::Frame {
    egui::Frame::new()
        .fill(theme.colors.menu_bg)
        .stroke(Stroke::new(1.0, theme.colors.menu_border))
        .inner_margin(egui::Margin::same(2))
        .shadow(egui::Shadow { offset: [2, 2], blur: 4, spread: 0, color: egui::Color32::from_black_alpha(if theme.glow { 0 } else { 70 }) })
}

const CHECK_COL: f32 = 28.0;
/// Ab so vielen Einträgen bekommt ein Untermenü einen Bildlauf.
pub const LONG_SUBMENU: usize = 40;
const RIGHT_PAD: f32 = 22.0;

/// Zeichnet einen Menüeintrag und liefert die Antwort.
fn paint_item(ui: &mut Ui, label: &str, shortcut: Option<&str>, checked: Option<bool>, enabled: bool, submenu: bool, open: bool) -> Response {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let font = theme.menu_font();
    let lw = crate::widgets::text_width(ui, label, &font);
    let sw = shortcut.map(|s| crate::widgets::text_width(ui, s, &font) + 30.0).unwrap_or(0.0);
    let want = (CHECK_COL + lw + sw + RIGHT_PAD + 10.0).max(theme.metrics.menu_min_width - 8.0);
    let resp = ui.allocate_response(vec2(want, theme.metrics.menu_item_height), if enabled { Sense::click() } else { Sense::hover() });
    let rect = resp.rect;
    let hot = enabled && (resp.hovered() || open);
    if hot {
        let r = rect.shrink2(vec2(1.0, 0.0));
        ui.painter().rect_filled(r, 0.0, c.menu_hover_bg);
        ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0, c.menu_hover_border), egui::StrokeKind::Inside);
        if theme.glow {
            ui.painter().rect_filled(Rect::from_min_size(r.min, vec2(3.0, r.height())), 0.0, c.menu_hover_border);
        }
    }
    let col = if !enabled { c.menu_text_disabled } else if hot { c.menu_hover_text } else { c.menu_text };
    if checked == Some(true) {
        let cr = Rect::from_center_size(pos2(rect.left() + 14.0, rect.center().y), vec2(12.0, 12.0));
        let st = crate::icons::IconStyle::mono(if enabled { c.menu_check } else { c.menu_text_disabled }, c.menu_bg);
        crate::icons::paint(ui.painter(), cr, crate::icons::Icon::Check, &st);
    }
    ui.painter().text(pos2(rect.left() + CHECK_COL + 8.0, rect.center().y), Align2::LEFT_CENTER, label, font.clone(), col);
    if let Some(s) = shortcut {
        ui.painter().text(pos2(rect.right() - RIGHT_PAD, rect.center().y), Align2::RIGHT_CENTER, s, font.clone(), col);
    }
    if submenu {
        let cx = rect.right() - 12.0;
        let cy = rect.center().y;
        ui.painter().line(vec![pos2(cx - 2.0, cy - 4.0), pos2(cx + 2.0, cy), pos2(cx - 2.0, cy + 4.0)], Stroke::new(1.0, col));
    }
    resp
}

fn paint_separator(ui: &mut Ui) {
    let theme = Theme::current(ui.ctx());
    let resp = ui.allocate_response(vec2(10.0, 9.0), Sense::hover());
    let r = resp.rect;
    ui.painter().hline((r.left() + CHECK_COL)..=(r.right() - 8.0), r.center().y.floor() + 0.5, Stroke::new(1.0, theme.colors.menu_separator));
}

/// Zeigt die Knoten eines Menüs; ausgewählte Aktionen landen in `out`.
pub fn show_nodes(ui: &mut Ui, nodes: &[Node], out: &mut Vec<Action>) {
    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
    for n in nodes {
        match n {
            Node::Separator => paint_separator(ui),
            Node::Item { label, shortcut, action, enabled, checked } => {
                let resp = paint_item(ui, label, shortcut.as_deref(), *checked, *enabled, false, false);
                if resp.clicked() {
                    match action {
                        ItemAction::Command(c) => out.push(Action::Run(c.clone(), Value::Object(Default::default()))),
                        ItemAction::Ui(a) => out.push(a.clone()),
                        ItemAction::None => {}
                    }
                    ui.close();
                }
            }
            Node::Sub { label, children, enabled } => {
                let id = ui.next_auto_id();
                let open = egui::containers::menu::MenuState::from_ui(ui, |s, _| s.open_item == Some(SubMenu::id_from_widget_id(id)));
                let resp = paint_item(ui, label, None, sub_head_checked(nodes, children), *enabled, true, open);
                if *enabled {
                    SubMenu::new().show(ui, &resp, |ui| show_submenu_body(ui, children, out));
                }
            }
        }
    }
}

/// Häkchen am Kopf eines Untermenüs: In langen Listen (Text ▸ Schrift, ab [`LONG_SUBMENU`]
/// Einträgen) trägt eine Familie mit mehreren Schnitten das ✓ am Familienkopf, wenn einer ihrer
/// Schnitte gewählt ist – sonst sähe man es erst nach dem Aufklappen (Befund MN-2). Kurze Menüs
/// (z. B. Ansicht ▸ Design) zeigen am Kopf wie im Original kein Häkchen.
pub fn sub_head_checked(siblings: &[Node], children: &[Node]) -> Option<bool> {
    (siblings.len() > LONG_SUBMENU && children.iter().any(|c| matches!(c, Node::Item { checked: Some(true), .. }))).then_some(true)
}

/// Inhalt eines Untermenüs; lange Untermenüs (Text ▸ Schrift: alle Familien flach, ab
/// [`LONG_SUBMENU`] Einträgen) bekommen wie im Original einen Bildlauf und bleiben im Fenster.
pub fn show_submenu_body(ui: &mut Ui, children: &[Node], out: &mut Vec<Action>) {
    if children.len() > LONG_SUBMENU {
        let max_h = (ui.ctx().content_rect().height() - 40.0).max(120.0);
        egui::ScrollArea::vertical().max_height(max_h).auto_shrink([true, true]).show(ui, |ui| show_nodes(ui, children, out));
    } else {
        show_nodes(ui, children, out);
    }
}

/// Öffnet ein Menü-Popup an `resp` (Bedienfeldmenü, Flyout-ähnliche Menüs).
pub fn popup_menu(ui: &Ui, resp: &Response, nodes: &[Node], out: &mut Vec<Action>) {
    let theme = Theme::current(ui.ctx());
    Popup::menu(resp)
        .frame(menu_frame(&theme))
        .info(UiStackInfo::new(UiKind::Menu).with_tag_value(MenuConfig::MENU_CONFIG_TAG, MenuConfig::new()))
        .show(|ui| show_nodes(ui, nodes, out));
}

/// Kennung im egui-Speicher: (Zahl der Menütitel, die in die Leiste passen, Zahl aller Titel).
pub const MENU_FIT_ID: &str = "fp-menu-bar-fit";

/// Menüleiste in der Anwendungsleiste. Fährt man bei offenem Menü über einen anderen Titel,
/// öffnet sich dessen Menü (Windows-Verhalten).
pub fn show_menu_bar(ui: &mut Ui, st: &mut AppState) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let mut menus = build_main_menu(st);
    let all_menus = menus.clone();
    let kbd = crate::menu_kbd::state(ui.ctx());
    let show_access = crate::menu_kbd::show_access_keys(ui.ctx());
    let mut anchors: Vec<Rect> = Vec::new();
    let mut out = Vec::new();
    // Schmales Fenster: Menüs, die nicht passen, wandern in ein Überlaufmenü „»“ (nichts verschwindet)
    let font = theme.menu_font();
    let widths: Vec<f32> = menus.iter().map(|(l, _)| crate::widgets::text_width(ui, l, &font) + 18.0).collect();
    let avail = ui.available_width();
    let overflow_w = 26.0;
    let mut fit = menus.len();
    if widths.iter().sum::<f32>() > avail {
        let mut used = 0.0;
        fit = 0;
        for w in &widths {
            if used + w + overflow_w > avail {
                break;
            }
            used += w;
            fit += 1;
        }
    }
    // für Tests: wie viele Menütitel passen (Überlauf bei langen Übersetzungen erkennen)
    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(MENU_FIT_ID), (fit, menus.len())));
    let overflow: Vec<(String, Vec<Node>)> = menus.split_off(fit);
    if !overflow.is_empty() {
        let children = overflow.into_iter().map(|(label, nodes)| Node::Sub { label, children: nodes, enabled: true }).collect();
        menus.push(("»".to_string(), children));
    }
    let ids: Vec<egui::Id> = (0..menus.len()).map(|i| egui::Id::new(("fp-main-menu", i))).collect();
    let any_open = ids.iter().any(|id| Popup::is_id_open(ui.ctx(), *id));
    for (i, (label, nodes)) in menus.iter().enumerate() {
        let font = theme.menu_font();
        let w = if label == "»" { overflow_w } else { crate::widgets::text_width(ui, label, &font) + 18.0 };
        let (rect, resp) = ui.allocate_exact_size(vec2(w, theme.metrics.app_bar_height - 8.0), Sense::click());
        let open = Popup::is_id_open(ui.ctx(), ids[i]);
        if any_open && !open && resp.hovered() {
            Popup::open_id(ui.ctx(), ids[i]);
        }
        let r = rect.shrink2(vec2(1.0, 1.0));
        // Titel dieses Eintrags bzw. der übergelaufenen Menüs (Anker der Tastaturmenüs)
        if label == "»" {
            while anchors.len() < all_menus.len() {
                anchors.push(rect);
            }
        } else {
            anchors.push(rect);
        }
        let kbd_here = kbd.active && (kbd.top == i || (label == "»" && kbd.top >= i));
        if kbd_here && !kbd.path.is_empty() {
            ui.painter().rect_filled(r, 0.0, c.app_bar_open_bg);
            ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0, c.app_bar_open_border), egui::StrokeKind::Inside);
        } else if kbd_here {
            ui.painter().rect_filled(r, 0.0, c.app_bar_hover_bg);
            ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0, c.focus), egui::StrokeKind::Inside);
        } else if open {
            ui.painter().rect_filled(r, 0.0, c.app_bar_open_bg);
            ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0, c.app_bar_open_border), egui::StrokeKind::Inside);
        } else if resp.hovered() {
            ui.painter().rect_filled(r, 0.0, c.app_bar_hover_bg);
            if theme.glow {
                ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0, with_alpha(c.focus, 140)), egui::StrokeKind::Inside);
            }
        }
        let tr = ui.painter().text(rect.center(), Align2::CENTER_CENTER, label, font.clone(), c.app_bar_text);
        if show_access {
            crate::menu_kbd::underline(ui.painter(), ui, pos2(tr.left(), rect.center().y), label, crate::menu_kbd::access_of(&format!("/{label}")), &font, c.app_bar_text);
        }
        Popup::menu(&resp)
            .id(ids[i])
            .frame(menu_frame(&theme))
            .info(UiStackInfo::new(UiKind::Menu).with_tag_value(MenuConfig::MENU_CONFIG_TAG, MenuConfig::new()))
            .show(|ui| show_nodes(ui, nodes, &mut out));
    }
    for a in out {
        note_font_choice(st, &a);
        st.push(a);
    }
    crate::menu_kbd::show(ui.ctx(), st, &all_menus, &anchors);
}

/// Linker Rand des Beschriftungstexts in Menüeinträgen.
pub const LABEL_X: f32 = CHECK_COL + 8.0;

/// Menüeintrag zeichnen (für die Tastaturmenüs, `open` = hervorgehoben).
pub fn paint_item_pub(ui: &mut Ui, label: &str, shortcut: Option<&str>, checked: Option<bool>, enabled: bool, submenu: bool, open: bool) -> Response {
    paint_item(ui, label, shortcut, checked, enabled, submenu, open)
}

/// Trenner zeichnen (für die Tastaturmenüs).
pub fn paint_separator_pub(ui: &mut Ui) {
    paint_separator(ui);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_resource_has_ten_menus_in_order() {
        let names: Vec<String> = main_menu().iter().map(|m| clean_label(&m.text).0).collect();
        assert_eq!(names, ["Datei", "Bearbeiten", "Ansicht", "Auswählen", "Modifizieren", "Text", "Befehle", "Filter", "Fenster", "Hilfe"]);
    }

    #[test]
    fn labels_are_cleaned() {
        assert_eq!(clean_label("&Schließen\tStrg+F4"), ("Schließen".to_string(), Some("Strg+F4".to_string())));
        assert_eq!(clean_label("A&&B"), ("A&B".to_string(), None));
    }

    #[test]
    fn panel_menus_exist() {
        assert!(context_menu("6923").is_some_and(|m| !m.is_empty()));
        assert!(context_menu("6939").is_some());
    }
}

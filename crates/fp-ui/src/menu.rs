//! Menüs: Hauptmenü (Ressource MENU 128 des Originals mit Trennern und Reihenfolge, Zustände
//! und Kürzel aus dem Befehlsregister von fp-engine), Laufzeit-Ergänzungen wie in CS6 (zuletzt
//! geöffnete Dateien, Fenster-Liste, Flash-Bedienfelder, Filter- und Befehle-Untermenüs) und die
//! Bedienfeldmenüs (Kontextmenü-Ressourcen).
//!
//! Die Menüdaten stehen in `data/menu_main.json` (aus `re\inventory\resources_de.json`, nur Texte,
//! IDs und Trenner). Ein Eintrag führt den Befehl aus, dessen Original-ID im Register steht
//! ([`fp_engine::CommandRegistry::by_fw_id`]); fehlt der Befehl, ist der Eintrag gesperrt.

use crate::app_state::{Action, AppState};
use crate::panels::PanelId;
use crate::theme::{with_alpha, Theme, ThemeKind};
use egui::containers::menu::{MenuConfig, SubMenu};
use egui::{pos2, vec2, Align2, Popup, Rect, Response, Sense, Stroke, Ui, UiKind, UiStackInfo};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::OnceLock;

/// Ein Eintrag der Menüressource.
#[derive(Clone, Debug, Deserialize)]
pub struct MenuEntry {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub id: Option<u32>,
    #[serde(default)]
    pub sep: bool,
    #[serde(default)]
    pub children: Option<Vec<MenuEntry>>,
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
fn command_state(st: &AppState, name: &str, fallback_sc: Option<String>) -> (bool, Option<bool>, Option<String>) {
    let sc = st.session.command_shortcuts(name).first().map(|s| s.display_de()).or(fallback_sc);
    if let Some((en, ck)) = crate::actions::ui_command_state(st, name) {
        return (en, ck, sc);
    }
    let (en, ck) = st.session.command_state(name);
    (en, ck, sc)
}

fn convert(st: &AppState, entries: &[MenuEntry], top: &str) -> Vec<Node> {
    let mut out = Vec::new();
    for e in entries {
        if e.sep {
            out.push(Node::Separator);
            continue;
        }
        let (label, sc) = clean_label(&e.text);
        if let Some(children) = &e.children {
            let mut kids = convert(st, children, top);
            // Laufzeit-Untermenüs
            if label == "Zuletzt geöffnete Dateien" {
                kids = recent_files(st);
            } else if label == "Schrift" && top == "Text" {
                kids = font_menu(st);
            } else if label == "Erweiterungen" {
                kids = vec![item("Keine Erweiterungen", ItemAction::None, false, None)];
            }
            let enabled = kids.iter().any(|k| matches!(k, Node::Item { enabled: true, .. } | Node::Sub { enabled: true, .. }));
            out.push(Node::Sub { label, children: kids, enabled });
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
                out.push(Node::Item { label, shortcut, action: ItemAction::Command(n), enabled, checked });
            }
            None => out.push(Node::Item { label, shortcut: sc, action: ItemAction::None, enabled: false, checked: None }),
        }
    }
    out
}

/// Text ▸ Schrift: Systemschriften (fp-text, im Hintergrund indiziert), bei vielen Familien nach
/// Anfangsbuchstaben in Untermenüs; Klick setzt `text.font` für die ausgewählten Texte.
fn font_menu(st: &AppState) -> Vec<Node> {
    let enabled = st.session.is_available("text.font");
    let fams = crate::inspector::fonts::families();
    let entry = |f: &crate::inspector::fonts::Family| {
        let ps = f.styles.first().map(|s| s.1.clone()).unwrap_or_else(|| f.name.clone());
        item(&f.name, ItemAction::Ui(Action::Run("text.font".into(), json!({ "font": ps }))), enabled, None)
    };
    if fams.len() <= 40 {
        return fams.iter().map(entry).collect();
    }
    let mut out: Vec<Node> = Vec::new();
    let mut cur: Option<(String, Vec<Node>)> = None;
    for f in fams.iter() {
        let first = f.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default();
        let key = if first.chars().all(|c| c.is_alphabetic()) { first } else { "#".to_string() };
        if cur.as_ref().is_none_or(|(k, _)| *k != key) {
            if let Some((k, v)) = cur.take() {
                out.push(Node::Sub { label: k, children: v, enabled });
            }
            cur = Some((key, Vec::new()));
        }
        if let Some((_, v)) = cur.as_mut() {
            v.push(entry(f));
        }
    }
    if let Some((k, v)) = cur {
        out.push(Node::Sub { label: k, children: v, enabled });
    }
    out
}

fn recent_files(st: &AppState) -> Vec<Node> {
    let files = &st.session.prefs.recent_files;
    if files.is_empty() {
        return vec![item("Keine", ItemAction::None, false, None)];
    }
    files
        .iter()
        .take(10)
        .enumerate()
        .map(|(i, p)| item(&format!("{} {}", i + 1, p.display()), ItemAction::Ui(Action::OpenPath(p.clone())), true, None))
        .collect()
}

/// Untermenü-Inhalte, die das Original zur Laufzeit aus Skripten bzw. Filtern füllt.
fn filter_submenus(st: &AppState) -> Vec<Node> {
    let groups: [(&str, &[(&str, &str)]); 5] = [
        ("Andere", &[("Kanten finden", "filter.other.findEdges"), ("In Alpha konvertieren", "filter.other.convertToAlpha")]),
        (
            "Farbe anpassen",
            &[
                ("Auto-Tonwertkorrektur", "filter.adjustColor.autoLevels"),
                ("Farbfüllung...", "filter.adjustColor.colorFill"),
                ("Farbton/Sättigung...", "filter.adjustColor.hueSaturation"),
                ("Helligkeit/Kontrast...", "filter.adjustColor.brightnessContrast"),
                ("Kurven...", "filter.adjustColor.curves"),
                ("Tonwertkorrektur...", "filter.adjustColor.levels"),
                ("Umkehren", "filter.adjustColor.invert"),
            ],
        ),
        ("Rauschen", &[("Rauschen hinzufügen...", "filter.noise.addNoise")]),
        ("Scharfzeichnen", &[("Scharfzeichnen", "filter.sharpen.sharpen"), ("Stark scharfzeichnen", "filter.sharpen.sharpenMore"), ("Unscharf maskieren...", "filter.sharpen.unsharpMask")]),
        (
            "Weichzeichnen",
            &[
                ("Bewegungsunschärfe...", "filter.blur.motion"),
                ("Gaußscher Weichzeichner...", "filter.blur.gaussian"),
                ("Radiales Weichzeichnen...", "filter.blur.radial"),
                ("Stark weichzeichnen", "filter.blur.blurMore"),
                ("Weichzeichnen", "filter.blur.blur"),
                ("Zentriertes Weichzeichnen...", "filter.blur.zoom"),
            ],
        ),
    ];
    groups
        .iter()
        .map(|(label, items)| {
            let children: Vec<Node> = items
                .iter()
                .map(|(l, name)| {
                    let resolved = resolve_filter(st, name, l);
                    match resolved {
                        Some(n) => {
                            let (en, ck, sc) = command_state(st, &n, None);
                            Node::Item { label: l.to_string(), shortcut: sc, action: ItemAction::Command(n), enabled: en, checked: ck }
                        }
                        None => item(l, ItemAction::None, false, None),
                    }
                })
                .collect();
            let enabled = children.iter().any(|k| matches!(k, Node::Item { enabled: true, .. }));
            Node::Sub { label: label.to_string(), children, enabled }
        })
        .collect()
}

/// Filterbefehl im Register: erst der erwartete Name, sonst ein `filter.*`-Befehl mit gleicher
/// Beschriftung (der Name kann sich in fp-engine noch ändern).
fn resolve_filter(st: &AppState, name: &str, label: &str) -> Option<String> {
    if st.session.registry().get(name).is_some() {
        return Some(name.to_string());
    }
    let plain = label.trim_end_matches("...");
    st.session.registry().iter().find(|c| c.info().name.starts_with("filter.") && c.info().label.trim_end_matches("...") == plain).map(|c| c.info().name.clone())
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
        let name = p.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
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
    if depth == 0 {
        out.push(item("Warnmeldungen zurücksetzen", ItemAction::Ui(Action::Run("ui.resetWarnings".into(), json!({}))), true, None));
        out.push(Node::Separator);
        let on = crate::bridge::running_url().is_some();
        out.push(item("KI-Steuerkanal (MCP)", ItemAction::Ui(Action::Run("ui.mcpToggle".into(), json!({}))), true, Some(on)));
    }
    out
}

fn panel_item(st: &AppState, p: PanelId) -> Node {
    let checked = st.settings.workspace.is_visible(p);
    item(p.title(), ItemAction::Ui(Action::TogglePanel(p)), true, Some(checked))
}

/// Der aufbereitete Hauptmenübaum (je Frame neu, damit Zustände stimmen).
pub fn build_main_menu(st: &AppState) -> Vec<(String, Vec<Node>)> {
    let mut out = Vec::new();
    for top in main_menu() {
        let (label, _) = clean_label(&top.text);
        let mut nodes = convert(st, top.children.as_deref().unwrap_or(&[]), &label);
        match label.as_str() {
            "Ansicht" => {
                nodes.push(Node::Separator);
                let children = ThemeKind::ALL.iter().map(|k| item(k.label(), ItemAction::Ui(Action::SetTheme(*k)), true, Some(st.settings.theme == *k))).collect();
                nodes.push(Node::Sub { label: "Design".into(), children, enabled: true });
            }
            "Befehle" => {
                // KI-Modelle (Agent KI) vor den Befehlen aus dem Befehlsordner
                if st.session.registry().get("ai.manageModels").is_some() {
                    let (en, ck, sc) = command_state(st, "ai.manageModels", None);
                    nodes.push(Node::Separator);
                    nodes.push(Node::Item { label: "KI-Modelle verwalten...".into(), shortcut: sc, action: ItemAction::Command("ai.manageModels".into()), enabled: en, checked: ck });
                }
                nodes.extend(command_submenus())
            }
            "Auswählen" => {
                // KI-Auswahl nach „Ähnliches auswählen“
                let pos = nodes.iter().position(|n| matches!(n, Node::Item { action: ItemAction::Command(c), .. } if c == "select.similar")).map(|i| i + 1).unwrap_or(nodes.len());
                let mut extra = Vec::new();
                for (label, cmd) in [("Motiv auswählen", "select.aiSubject"), ("Kante verfeinern...", "select.aiRefineEdge")] {
                    if st.session.registry().get(cmd).is_some() {
                        let (en, ck, sc) = command_state(st, cmd, None);
                        extra.push(Node::Item { label: label.into(), shortcut: sc, action: ItemAction::Command(cmd.into()), enabled: en, checked: ck });
                    }
                }
                for (k, n) in extra.into_iter().enumerate() {
                    nodes.insert(pos + k, n);
                }
            }
            "Filter" => nodes.extend(filter_submenus(st)),
            "Fenster" => {
                // Flash-Bedienfelder (Laufzeit) nach „Suchen und ersetzen“, Dokumentliste am Ende
                let pos = nodes.iter().position(|n| matches!(n, Node::Item { action: ItemAction::Command(c), .. } if c == "edit.findAndReplace")).map(|i| i + 1).unwrap_or(nodes.len());
                let extra = vec![
                    Node::Separator,
                    panel_item(st, PanelId::Align),
                    panel_item(st, PanelId::AutoShapeProperties),
                    Node::Sub { label: "Sonstige".into(), children: vec![panel_item(st, PanelId::ImageEditing), panel_item(st, PanelId::ColorPalette), panel_item(st, PanelId::SpecialCharacters)], enabled: true },
                    panel_item(st, PanelId::Path),
                    panel_item(st, PanelId::SymbolProperties),
                ];
                for (k, n) in extra.into_iter().enumerate() {
                    nodes.insert(pos + k, n);
                }
                // Arbeitsbereichlayouts (Ordner der Layouts, 11 §4.2) vor „Erweiterungen“
                let at = nodes.iter().position(|n| matches!(n, Node::Sub { label, .. } if label == "Erweiterungen")).unwrap_or(0);
                nodes.insert(at, Node::Sub { label: "Arbeitsbereichlayouts".into(), children: crate::appbar::workspace_menu(st), enabled: true });
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

/// Bedienfeldmenü (≡) eines Bedienfelds: Einträge der Ressource plus Schließen.
pub fn build_panel_menu(st: &AppState, panel: PanelId) -> Vec<Node> {
    let mut nodes = match panel.menu_resource().and_then(context_menu) {
        Some(entries) => convert(st, entries, ""),
        None => Vec::new(),
    };
    if !nodes.is_empty() {
        nodes.push(Node::Separator);
    }
    nodes.push(item("Hilfe", ItemAction::Ui(Action::Run("help.firepixelHelp".into(), json!({ "topic": "panels" }))), true, None));
    nodes.push(Node::Separator);
    nodes.push(item("Schließen", ItemAction::Ui(Action::TogglePanel(panel)), true, None));
    nodes.push(item("Registerkartengruppe schließen", ItemAction::Ui(Action::Run("ui.closeGroup".into(), json!({ "panel": format!("{panel:?}") }))), true, None));
    nodes
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
                let resp = paint_item(ui, label, None, None, *enabled, true, open);
                if *enabled {
                    SubMenu::new().show(ui, &resp, |ui| show_nodes(ui, children, out));
                }
            }
        }
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

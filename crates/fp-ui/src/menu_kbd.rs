//! Hauptmenü per Tastatur wie unter Windows (und im Original):
//!
//! * Alt+Buchstabe öffnet das Menü mit dieser Zugriffstaste (unterstrichener Buchstabe, `&` in
//!   `data\menu_main.json`), z. B. Alt+D = „Datei“,
//! * Alt allein (drücken und loslassen) bzw. F10 aktiviert die Menüleiste (erster Titel markiert),
//! * ←/→ wechseln das Menü (bei offenem Menü öffnet das Nachbarmenü), → öffnet ein Untermenü,
//!   ← schließt es,
//! * ↑/↓ wandern über die Einträge (Trenner und graue Einträge werden übersprungen), Pos1/Ende,
//! * Eingabe bzw. Leertaste führt aus bzw. öffnet, Esc schließt stufenweise,
//! * Buchstaben wirken in offenen Menüs als Zugriffstasten.
//!
//! [`keyboard`] läuft vor der übrigen Tastaturverarbeitung und nimmt die Tasten, die es verwendet,
//! aus der Ereignisliste (Werkzeuge und Kürzel sehen sie dann nicht). [`show`] zeichnet die per
//! Tastatur geöffneten Menüs; die Maus bedient sie weiterhin.

use crate::app_state::{Action, AppState};
use crate::menu::{self, ItemAction, MenuEntry, Node};
use crate::theme::Theme;
use egui::{pos2, vec2, Event, Id, Key, Rect};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Zustand der Tastaturbedienung (egui-Speicher).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct KbdMenu {
    /// Menüleiste aktiv (Titel `top` markiert).
    pub active: bool,
    pub top: usize,
    /// Ausgewählter Eintrag je offener Ebene (leer = nur die Leiste ist aktiv).
    pub path: Vec<usize>,
    /// Alt gedrückt, ohne dass eine andere Taste kam (Alt allein aktiviert die Leiste).
    alt_alone: bool,
    alt_prev: bool,
}

fn state_id() -> Id {
    Id::new("fp-menu-kbd")
}

/// Aktueller Zustand.
pub fn state(ctx: &egui::Context) -> KbdMenu {
    ctx.data(|d| d.get_temp(state_id())).unwrap_or_default()
}

fn store(ctx: &egui::Context, s: KbdMenu) {
    ctx.data_mut(|d| d.insert_temp(state_id(), s));
}

/// Zugriffstaste eines Ressourcentexts (Buchstabe nach `&`, nicht `&&`) und seine Lage im
/// bereinigten Text (Zeichenindex).
pub fn access_key(raw: &str) -> Option<(usize, char)> {
    let label = raw.split('\t').next().unwrap_or(raw);
    let mut idx = 0;
    let mut chars = label.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            match chars.peek() {
                Some('&') => {
                    chars.next();
                    idx += 1;
                }
                Some(n) => return Some((idx, n.to_lowercase().next().unwrap_or(*n))),
                None => {}
            }
            continue;
        }
        idx += 1;
    }
    None
}

/// Zugriffstasten aller Menüeinträge, Schlüssel = Pfad der bereinigten Beschriftungen.
fn access_map() -> &'static HashMap<String, (usize, char)> {
    static M: OnceLock<HashMap<String, (usize, char)>> = OnceLock::new();
    M.get_or_init(|| {
        fn walk(entries: &[MenuEntry], prefix: &str, out: &mut HashMap<String, (usize, char)>) {
            for e in entries {
                if e.sep {
                    continue;
                }
                let (label, _) = menu::clean_label(&e.text);
                let key = format!("{prefix}/{label}");
                if let Some(a) = access_key(&e.text) {
                    out.insert(key.clone(), a);
                }
                if let Some(c) = &e.children {
                    walk(c, &key, out);
                }
            }
        }
        let mut out = HashMap::new();
        walk(menu::main_menu(), "", &mut out);
        out
    })
}

/// Zugriffstaste eines Eintrags im Menübaum (Pfad wie `/Datei/Öffnen...`).
pub fn access_of(path: &str) -> Option<(usize, char)> {
    access_map().get(path).copied()
}

fn selectable(n: &Node) -> bool {
    matches!(n, Node::Item { enabled: true, .. } | Node::Sub { enabled: true, .. })
}

fn first_selectable(nodes: &[Node]) -> usize {
    nodes.iter().position(selectable).unwrap_or(0)
}

fn step(nodes: &[Node], from: usize, down: bool) -> usize {
    let n = nodes.len();
    if n == 0 {
        return 0;
    }
    let mut i = from;
    for _ in 0..n {
        i = if down { (i + 1) % n } else { (i + n - 1) % n };
        if selectable(&nodes[i]) {
            return i;
        }
    }
    from
}

fn label_of(n: &Node) -> &str {
    match n {
        Node::Item { label, .. } | Node::Sub { label, .. } => label,
        Node::Separator => "",
    }
}

/// Knoten der Ebene `level` (0 = Menü `top`) entlang `path`.
fn level_nodes<'a>(menus: &'a [(String, Vec<Node>)], top: usize, path: &[usize], level: usize) -> Option<(&'a [Node], String)> {
    let (title, nodes) = menus.get(top)?;
    let mut nodes: &[Node] = nodes;
    let mut key = format!("/{title}");
    for &i in path.iter().take(level) {
        match nodes.get(i)? {
            Node::Sub { label, children, .. } => {
                key = format!("{key}/{label}");
                nodes = children;
            }
            _ => return None,
        }
    }
    Some((nodes, key))
}

fn activate(n: &Node, out: &mut Vec<Action>) {
    if let Node::Item { action, enabled: true, .. } = n {
        match action {
            ItemAction::Command(c) => out.push(Action::Run(c.clone(), Value::Object(Default::default()))),
            ItemAction::Ui(a) => out.push(a.clone()),
            ItemAction::None => {}
        }
    }
}

/// Tastaturbedienung auswerten (vor den Kürzeln aufrufen).
pub fn keyboard(ctx: &egui::Context, st: &mut AppState) {
    let mut s = state(ctx);
    let (events, alt_now, pointer_pressed) = ctx.input(|i| (i.events.clone(), i.modifiers.alt, i.pointer.any_pressed()));
    let menus = menu::build_main_menu(st);
    let n_top = menus.len().max(1);
    let mut out: Vec<Action> = Vec::new();
    let mut consumed: Vec<usize> = Vec::new();
    // Maus beendet die Tastaturbedienung
    if pointer_pressed && s.active {
        s = KbdMenu::default();
    }
    // Alt allein: drücken und ohne andere Taste loslassen
    let any_key = events.iter().any(|e| matches!(e, Event::Key { pressed: true, .. }) || matches!(e, Event::PointerButton { .. }));
    if alt_now && !s.alt_prev {
        s.alt_alone = true;
    }
    if any_key {
        s.alt_alone = false;
    }
    if !alt_now && s.alt_prev && s.alt_alone {
        if s.active {
            s = KbdMenu { alt_prev: false, ..KbdMenu::default() };
            egui::Popup::close_all(ctx);
        } else {
            s.active = true;
            s.top = 0;
            s.path.clear();
            egui::Popup::close_all(ctx);
        }
        s.alt_alone = false;
    }
    s.alt_prev = alt_now;
    for (ei, e) in events.iter().enumerate() {
        let Event::Key { key, pressed: true, modifiers, .. } = e else { continue };
        let key = *key;
        // Alt+Buchstabe: Menü über die Zugriffstaste öffnen
        if modifiers.alt && !modifiers.ctrl && !modifiers.command && !(s.active && !s.path.is_empty()) {
            let ch = key.name().chars().next().map(|c| c.to_ascii_lowercase());
            if let Some(ch) = ch
                && key.name().chars().count() == 1
                && let Some(t) = menus.iter().position(|(title, _)| access_of(&format!("/{title}")).map(|a| a.1) == Some(ch))
            {
                s.active = true;
                s.top = t;
                s.path = vec![first_selectable(&menus[t].1)];
                egui::Popup::close_all(ctx);
                consumed.push(ei);
                continue;
            }
        }
        if key == Key::F10 && !modifiers.ctrl && !modifiers.shift {
            if s.active {
                s = KbdMenu::default();
            } else {
                s.active = true;
                s.top = 0;
                s.path.clear();
                egui::Popup::close_all(ctx);
            }
            consumed.push(ei);
            continue;
        }
        if !s.active {
            continue;
        }
        consumed.push(ei);
        let open = !s.path.is_empty();
        match key {
            Key::Escape => {
                if s.path.len() > 1 {
                    s.path.pop();
                } else if open {
                    s.path.clear();
                } else {
                    s = KbdMenu::default();
                }
            }
            Key::ArrowLeft => {
                if s.path.len() > 1 {
                    s.path.pop();
                } else {
                    s.top = (s.top + n_top - 1) % n_top;
                    if open {
                        s.path = vec![first_selectable(&menus[s.top].1)];
                    }
                }
            }
            Key::ArrowRight => {
                let mut opened = false;
                if open
                    && let Some((nodes, _)) = level_nodes(&menus, s.top, &s.path, s.path.len() - 1)
                    && let Some(Node::Sub { children, enabled: true, .. }) = nodes.get(*s.path.last().unwrap_or(&0))
                {
                    s.path.push(first_selectable(children));
                    opened = true;
                }
                if !opened {
                    s.top = (s.top + 1) % n_top;
                    if open {
                        s.path = vec![first_selectable(&menus[s.top].1)];
                    }
                }
            }
            Key::ArrowDown | Key::ArrowUp => {
                if !open {
                    s.path = vec![first_selectable(&menus[s.top].1)];
                } else if let Some((nodes, _)) = level_nodes(&menus, s.top, &s.path, s.path.len() - 1)
                    && let Some(last) = s.path.last_mut()
                {
                    *last = step(nodes, *last, key == Key::ArrowDown);
                }
            }
            Key::Home | Key::End => {
                if open
                    && let Some((nodes, _)) = level_nodes(&menus, s.top, &s.path, s.path.len() - 1)
                    && let Some(last) = s.path.last_mut()
                {
                    *last = if key == Key::Home { first_selectable(nodes) } else { step(nodes, 0, false) };
                }
            }
            Key::Enter | Key::Space => {
                if !open {
                    s.path = vec![first_selectable(&menus[s.top].1)];
                } else if let Some((nodes, _)) = level_nodes(&menus, s.top, &s.path, s.path.len() - 1)
                    && let Some(node) = nodes.get(*s.path.last().unwrap_or(&0))
                {
                    match node {
                        Node::Sub { children, enabled: true, .. } => s.path.push(first_selectable(children)),
                        Node::Item { enabled: true, .. } => {
                            activate(node, &mut out);
                            s = KbdMenu::default();
                        }
                        _ => {}
                    }
                }
            }
            k => {
                // Zugriffstaste im offenen Menü bzw. in der Leiste
                let name = k.name();
                if name.chars().count() != 1 {
                    continue;
                }
                let ch = name.chars().next().map(|c| c.to_ascii_lowercase()).unwrap_or(' ');
                if !open {
                    if let Some(t) = menus.iter().position(|(title, _)| access_of(&format!("/{title}")).map(|a| a.1) == Some(ch)) {
                        s.top = t;
                        s.path = vec![first_selectable(&menus[t].1)];
                    }
                } else if let Some((nodes, key_path)) = level_nodes(&menus, s.top, &s.path, s.path.len() - 1) {
                    let hit = nodes.iter().position(|nd| selectable(nd) && access_of(&format!("{key_path}/{}", label_of(nd))).map(|a| a.1) == Some(ch));
                    if let Some(i) = hit {
                        match &nodes[i] {
                            Node::Sub { children, .. } => {
                                if let Some(last) = s.path.last_mut() {
                                    *last = i;
                                }
                                s.path.push(first_selectable(children));
                            }
                            node => {
                                activate(node, &mut out);
                                s = KbdMenu::default();
                            }
                        }
                    }
                }
            }
        }
    }
    // verwendete Tasten (und bei aktiver Leiste die Texteingabe) nicht weitergeben
    if !consumed.is_empty() || s.active {
        let active = s.active;
        ctx.input_mut(|i| {
            let mut k = 0;
            i.events.retain(|e| {
                let keep = !consumed.contains(&k) && !(active && matches!(e, Event::Text(_) | Event::Key { .. }));
                k += 1;
                keep
            });
        });
    }
    store(ctx, s);
    for a in out {
        st.push(a);
    }
}

/// Unterstreicht die Zugriffstaste in einer Beschriftung (Text links an `pos`).
pub fn underline(painter: &egui::Painter, ui: &egui::Ui, pos: egui::Pos2, label: &str, access: Option<(usize, char)>, font: &egui::FontId, color: egui::Color32) {
    let Some((idx, _)) = access else { return };
    let prefix: String = label.chars().take(idx).collect();
    let ch: String = label.chars().skip(idx).take(1).collect();
    if ch.is_empty() {
        return;
    }
    let x0 = pos.x + crate::widgets::text_width(ui, &prefix, font);
    let w = crate::widgets::text_width(ui, &ch, font);
    let y = pos.y + font.size * 0.62;
    painter.hline(x0..=(x0 + w), y.round() + 0.5, egui::Stroke::new(1.0, color));
}

/// Sollen Zugriffstasten unterstrichen werden (Alt gedrückt oder Tastaturbedienung aktiv)?
pub fn show_access_keys(ctx: &egui::Context) -> bool {
    ctx.input(|i| i.modifiers.alt) || state(ctx).active
}

/// Zeichnet die per Tastatur geöffneten Menüs unter dem Titel `anchor` (Rechteck des Titels).
pub fn show(ctx: &egui::Context, st: &mut AppState, menus: &[(String, Vec<Node>)], anchors: &[Rect]) {
    let mut s = state(ctx);
    if !s.active || s.path.is_empty() {
        return;
    }
    let theme = Theme::current(ctx);
    let mut out = Vec::new();
    let mut origin = anchors.get(s.top).map(|r| r.left_bottom()).unwrap_or(pos2(40.0, 30.0));
    let mut close = false;
    for level in 0..s.path.len() {
        let Some((nodes, key_path)) = level_nodes(menus, s.top, &s.path, level) else { break };
        let sel = s.path[level];
        let mut sel_rect = None;
        egui::Area::new(Id::new(("fp-menu-kbd-level", level))).order(egui::Order::Foreground).fixed_pos(origin).show(ctx, |ui| {
            menu::menu_frame(&theme).show(ui, |ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                for (i, n) in nodes.iter().enumerate() {
                    if matches!(n, Node::Separator) {
                        menu::paint_separator_pub(ui);
                        continue;
                    }
                    let (label, shortcut, enabled, checked, sub) = match n {
                        Node::Item { label, shortcut, enabled, checked, .. } => (label.as_str(), shortcut.as_deref(), *enabled, *checked, false),
                        Node::Sub { label, enabled, .. } => (label.as_str(), None, *enabled, None, true),
                        Node::Separator => continue,
                    };
                    let resp = menu::paint_item_pub(ui, label, shortcut, checked, enabled, sub, i == sel);
                    let font = theme.menu_font();
                    let col = if enabled { theme.colors.menu_text } else { theme.colors.menu_text_disabled };
                    underline(ui.painter(), ui, pos2(resp.rect.left() + menu::LABEL_X, resp.rect.center().y), label, access_of(&format!("{key_path}/{label}")), &font, col);
                    if i == sel {
                        sel_rect = Some(resp.rect);
                    }
                    if resp.clicked() && enabled {
                        match n {
                            Node::Sub { .. } => {
                                s.path.truncate(level + 1);
                                s.path[level] = i;
                            }
                            _ => {
                                activate(n, &mut out);
                                close = true;
                            }
                        }
                    }
                }
            });
        });
        origin = sel_rect.map(|r| r.right_top() + vec2(2.0, -4.0)).unwrap_or(origin + vec2(200.0, 0.0));
    }
    if close {
        s = KbdMenu::default();
    }
    store(ctx, s);
    for a in out {
        st.push(a);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_keys_from_resource() {
        assert_eq!(access_key("&Datei"), Some((0, 'd')));
        assert_eq!(access_key("Ö&ffnen...\tStrg+O"), Some((1, 'f')));
        assert_eq!(access_key("A && B"), None);
        assert_eq!(access_of("/Datei").map(|a| a.1), Some('d'));
    }
}

//! Gemeinsame Bausteine der Bedienfelder: Liste mit Rahmen, Listenzeile, Fußleiste mit Knöpfen,
//! Inline-Umbenennen. Alle Farben aus dem [`Theme`].

use crate::app_state::Action;
use crate::inspector::glyphs::{self, Glyph};
use crate::theme::Theme;
use egui::{pos2, vec2, Align2, Id, Rect, Sense, Stroke, StrokeKind, Ui};

/// Höhe der Fußleiste (Knöpfe 13 × 11 DLU).
pub const FOOTER_H: f32 = 22.0;

/// Liste mit Rahmen über die verfügbare Höhe minus `footer`; `add` zeichnet die Zeilen.
pub fn list_frame(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, footer: f32, add: impl FnOnce(&mut Ui)) -> Rect {
    let theme = Theme::current(ui.ctx());
    let avail = ui.available_rect_before_wrap();
    // RAND-2b (NZ-2): nie über die Fußleiste hinaus; ist das Bedienfeld zu niedrig, wird die
    // Liste kleiner (früher mindestens 24 px, sie lag dann über den Fußleistenknöpfen)
    let rect = Rect::from_min_max(avail.min + vec2(1.0, 1.0), pos2(avail.right() - 1.0, (avail.bottom() - footer).max(avail.top() + 1.0)));
    if rect.height() >= 4.0 {
        ui.painter().rect_filled(rect, 0.0, theme.colors.list_bg);
        ui.painter().rect_stroke(rect, 0.0, Stroke::new(1.0, theme.colors.list_border), StrokeKind::Inside);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(1.0)).layout(egui::Layout::top_down(egui::Align::Min)));
        // ohne Mindesthöhe (egui: 64 px), sonst reicht der Bildlaufbereich über die Fußleiste
        egui::ScrollArea::vertical().id_salt(Id::new(id)).auto_shrink([false, false]).min_scrolled_height(0.0).max_height(rect.height() - 2.0).show(&mut child, |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            add(ui);
        });
    }
    ui.allocate_rect(rect, Sense::hover());
    rect
}

/// Zeile einer Liste (Auswahlfarbe, Hover, Trennlinie).
pub fn row(ui: &mut Ui, height: f32, selected: bool) -> (Rect, egui::Response) {
    let theme = Theme::current(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click_and_drag());
    let c = &theme.colors;
    if selected {
        ui.painter().rect_filled(rect, 0.0, c.list_selected_bg);
        if theme.glow {
            ui.painter().rect_stroke(rect, 0.0, Stroke::new(1.0, c.focus), StrokeKind::Inside);
        }
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, 0.0, c.list_hover_bg);
    }
    ui.painter().hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, c.list_row_line));
    (rect, resp)
}

/// Ein Knopf der Fußleiste.
pub struct FooterButton<'a> {
    pub glyph: Glyph,
    pub tip: &'a str,
    pub enabled: bool,
    pub action: Option<Action>,
}

/// Fußleiste mit Text links und Knöpfen rechts; liefert die Aktion des geklickten Knopfs.
pub fn footer(ui: &mut Ui, left_x: f32, text: &str, buttons: &[FooterButton<'_>]) -> Option<Action> {
    let theme = Theme::current(ui.ctx());
    let mut out = None;
    let rect = ui.available_rect_before_wrap();
    let bar = Rect::from_min_max(pos2(rect.left(), rect.bottom() - FOOTER_H), rect.right_bottom());
    // Text nie unter die Knöpfe laufen lassen (schmale Bedienfelder: gekürzt mit …)
    let max_w = (bar.width() - left_x - 8.0 - 20.0 * buttons.len() as f32).max(0.0);
    crate::widgets::text_clipped(ui.painter(), pos2(bar.left() + left_x, bar.center().y), Align2::LEFT_CENTER, text, theme.small_font(), theme.colors.text, max_w);
    let mut x = bar.right() - 4.0;
    for (i, b) in buttons.iter().enumerate().rev() {
        x -= 20.0;
        let r = Rect::from_min_size(pos2(x, bar.top() + 3.0), vec2(19.0, 16.0));
        let resp = ui.interact(r, Id::new(("footer-btn", bar.min.x as i32, bar.min.y as i32, i)), if b.enabled { Sense::click() } else { Sense::hover() });
        if b.enabled && resp.hovered() {
            ui.painter().rect_filled(r, 2.0, theme.colors.button_hover_bg);
            ui.painter().rect_stroke(r, 2.0, Stroke::new(1.0, theme.colors.button_hover_border), StrokeKind::Inside);
        }
        glyphs::paint(ui.painter(), r.shrink(1.0), b.glyph, &theme, b.enabled);
        crate::ui_kit::icon_button_info(&resp, b.enabled, &b.tip, None);
        let resp = resp.on_hover_text(b.tip);
        if b.enabled && resp.clicked() {
            out = b.action.clone();
        }
    }
    out
}

/// Inline-Umbenennen (Windoids 5988/19135/6487): Textfeld über der Zeile; liefert den neuen Namen
/// bei Eingabe, `Some(None)` bei Abbruch.
pub fn inline_rename(ui: &mut Ui, id: Id, r: Rect, current: &str) -> Option<Option<String>> {
    let key = id.with("buf");
    let stored: Option<String> = ui.data(|d| d.get_temp(key));
    let first = stored.is_none();
    let mut buf: String = stored.unwrap_or_else(|| current.to_string());
    let theme = Theme::current(ui.ctx());
    ui.painter().rect_filled(r, 0.0, theme.colors.field_bg);
    let resp = ui.put(r, egui::TextEdit::singleline(&mut buf).id(id).font(theme.ui_font()).desired_width(r.width()).event_filter(crate::inspector::fields::TEXT_FILTER));
    let esc = ui.input(|i| i.key_pressed(egui::Key::Escape));
    // Esc behält das Feld selbst (sonst erreicht die Taste zusätzlich die Leinwand): verwerfen
    if esc && resp.has_focus() {
        ui.memory_mut(|m| m.surrender_focus(id));
        ui.data_mut(|d| d.remove::<String>(key));
        return Some(None);
    }
    if !resp.has_focus() && !resp.lost_focus() {
        resp.request_focus();
    }
    // wie die Edit-Zelle des Originals: beim Öffnen ist der ganze Name markiert
    if (first || resp.gained_focus()) && let Some(mut s) = egui::TextEdit::load_state(ui.ctx(), id) {
        let n = buf.chars().count();
        s.cursor.set_char_range(Some(egui::text::CCursorRange::two(egui::text::CCursor::new(0), egui::text::CCursor::new(n))));
        s.store(ui.ctx(), id);
    }
    if resp.lost_focus() {
        ui.data_mut(|d| d.remove::<String>(key));
        return Some(if esc || buf.trim().is_empty() { None } else { Some(buf.trim().to_string()) });
    }
    ui.data_mut(|d| d.insert_temp(key, buf));
    None
}

/// Menüknoten aus einer Kontextmenü-Ressource (z. B. `"6940"`); die Einträge führen die Befehle
/// mit derselben Befehls-ID aus fp-engine aus (Zustand/Häkchen aus dem Register).
pub fn resource_menu_nodes(st: &crate::app_state::AppState, id: &str) -> Vec<crate::menu::Node> {
    fn conv(st: &crate::app_state::AppState, entries: &[crate::menu::MenuEntry]) -> Vec<crate::menu::Node> {
        let mut out = Vec::new();
        for e in entries {
            if e.sep {
                out.push(crate::menu::Node::Separator);
                continue;
            }
            // Text der aktuellen Sprache (Schlüssel aus den Menüdaten, I18N-MENUE)
            let (label, _) = crate::menu::clean_label(&e.local_text());
            if let Some(ch) = &e.children {
                let children = conv(st, ch);
                out.push(crate::menu::Node::Sub { label, children, enabled: true });
                continue;
            }
            let name = e.id.and_then(|i| st.session.registry().by_fw_id(i).map(|c| c.info().name.clone()));
            match name {
                Some(n) => {
                    let (enabled, checked) = st.session.command_state(&n);
                    out.push(crate::menu::Node::Item { label, shortcut: None, action: crate::menu::ItemAction::Command(n), enabled, checked });
                }
                None => out.push(crate::menu::Node::Item { label, shortcut: None, action: crate::menu::ItemAction::None, enabled: false, checked: None }),
            }
        }
        out
    }
    crate::menu::context_menu(id).map(|e| conv(st, e)).unwrap_or_default()
}

/// Öffnet das Menü einer Kontextmenü-Ressource an `resp` und reiht die gewählte Aktion ein.
/// Kontextmenü (Rechtsklick) aus einer Menüressource, z. B. 6923 auf den Zeilen des Ebenen-Bedienfelds.
pub fn resource_context_menu(resp: &egui::Response, st: &mut crate::app_state::AppState, id: &str) {
    let theme = Theme::current(&resp.ctx);
    let mut acts = Vec::new();
    let view: &crate::app_state::AppState = st;
    egui::Popup::context_menu(resp)
        .frame(crate::menu::menu_frame(&theme))
        .info(egui::UiStackInfo::new(egui::UiKind::Menu).with_tag_value(egui::containers::menu::MenuConfig::MENU_CONFIG_TAG, egui::containers::menu::MenuConfig::new()))
        .show(|ui| {
            // Einträge erst beim Öffnen bilden (Zustände/Häkchen aus dem Register)
            let nodes = resource_menu_nodes(view, id);
            crate::menu::show_nodes(ui, &nodes, &mut acts)
        });
    for a in acts {
        st.push(a);
    }
}

pub fn resource_menu(ui: &Ui, resp: &egui::Response, st: &mut crate::app_state::AppState, id: &str) {
    let nodes = resource_menu_nodes(st, id);
    let mut acts = Vec::new();
    crate::menu::popup_menu(ui, resp, &nodes, &mut acts);
    for a in acts {
        st.push(a);
    }
}

// ---------------------------------------------------------------------- Texte (Phase 6)

/// Text eines Schlüssels aus `bedienfelder.ftl` (bzw. jeder anderen Bereichsdatei).
pub fn t(key: &str) -> String {
    fp_i18n::t(key)
}

/// Übersetzter Text als `&'static str` (für APIs, die feste Texte erwarten, z. B. Reitertitel).
/// Jeder unterschiedliche Text wird genau einmal abgelegt; die Menge ist durch die Zahl der
/// Schlüssel × Sprachen begrenzt.
pub fn t_static(key: &str) -> &'static str {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static POOL: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();
    let text = fp_i18n::t(key);
    let pool = POOL.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut m) = pool.lock() else {
        return "";
    };
    if let Some(s) = m.get(&text) {
        return s;
    }
    let s: &'static str = Box::leak(text.clone().into_boxed_str());
    m.insert(text, s);
    s
}

/// Text in `r` (links, rechts oder mittig ausgerichtet), bei Platzmangel mit „…“ gekürzt; der
/// volle Text steht dann als Tooltip über `r`. Liefert `true`, wenn gekürzt wurde.
pub fn fit_text(ui: &Ui, r: Rect, align: Align2, text: &str, font: egui::FontId, color: egui::Color32) -> bool {
    let full = crate::widgets::text_width(ui, text, &font);
    let x = match align.x() {
        egui::Align::Min => r.left(),
        egui::Align::Center => r.center().x,
        egui::Align::Max => r.right(),
    };
    let y = match align.y() {
        egui::Align::Min => r.top(),
        egui::Align::Center => r.center().y,
        egui::Align::Max => r.bottom(),
    };
    crate::widgets::text_clipped(ui.painter(), pos2(x, y), align, text, font, color, r.width().max(0.0));
    let cut = full > r.width();
    if cut {
        let id = Id::new(("fit-text", r.min.x as i32, r.min.y as i32, text));
        let _ = ui.interact(r, id, Sense::hover()).on_hover_text(text);
    }
    cut
}

/// Beschriftung wie [`crate::inspector::fields::label`], aber auf `r` gekürzt (mit „…“ und Tooltip).
pub fn label_fit(ui: &Ui, r: Rect, text: &str, enabled: bool) {
    let t = Theme::current(ui.ctx());
    let col = if enabled { t.colors.text } else { t.colors.text_disabled };
    fit_text(ui, r, Align2::LEFT_CENTER, text, t.ui_font(), col);
}

/// Rechtsbündige Beschriftung (wie [`crate::inspector::fields::label_right`]), auf `r` gekürzt.
pub fn label_right_fit(ui: &Ui, r: Rect, text: &str, enabled: bool) {
    let t = Theme::current(ui.ctx());
    let col = if enabled { t.colors.text } else { t.colors.text_disabled };
    fit_text(ui, r, Align2::RIGHT_CENTER, text, t.ui_font(), col);
}

/// Kontrollkästchen wie [`crate::inspector::fields::checkbox`], Beschriftung auf `r` gekürzt.
pub fn checkbox_fit(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, r: Rect, text: &str, checked: bool, enabled: bool) -> Option<bool> {
    let out = crate::inspector::fields::checkbox(ui, id, r, "", checked, enabled);
    label_fit(ui, Rect::from_min_max(pos2(r.left() + 18.0, r.top()), r.max), text, enabled);
    out
}

/// Anzeigename eines Werkzeugs: `tool-<name ohne „tool.“>`, sonst `cmd-tool-…`, sonst die
/// Beschriftung aus fp-engine (gleiche Reihenfolge wie im Inspektor).
pub fn tool_label(t: fp_engine::ToolId) -> String {
    let Some(info) = fp_engine::tools::info(t) else { return String::new() };
    let short = info.name.strip_prefix("tool.").unwrap_or(info.name);
    for key in [format!("tool-{short}"), fp_i18n::command_key(info.name)] {
        if fp_i18n::has(&key) {
            return fp_i18n::t(&key);
        }
    }
    info.label.to_string()
}

/// Hinweistext für Funktionen, die noch nicht verfügbar sind.
pub fn not_available(ui: &mut Ui, text: &str) {
    let theme = Theme::current(ui.ctx());
    ui.add_space(6.0);
    ui.label(egui::RichText::new(text).color(theme.colors.text_disabled).font(theme.small_font()));
}

/// Kleine Überschrift (wie „Ausrichten:“ im Flash-Bedienfeld).
pub fn caption(ui: &mut Ui, text: &str) {
    let theme = Theme::current(ui.ctx());
    ui.label(egui::RichText::new(text).font(theme.ui_font()).color(theme.colors.text));
}

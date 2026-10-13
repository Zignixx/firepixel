//! Eigenschaften-Inspektor (11 §2.3): Rahmen mit Reitern „Eigenschaften“/„Symboleigenschaften“,
//! sechs Quadranten (Q1 links oben, Q2 links unten, Q3/Q4 Mitte, Q5/Q6 rechts), Umschaltung nach der
//! Layout-Tabelle ([`layouts`]) und Trennlinien nach der 9-Bit-Maske.
//!
//! **Erweiterung in Phase 4:** Für jede Vorlage eine Funktion registrieren
//! ([`QuadrantRegistry::register`]); sie bekommt die Fläche des Quadranten und den Zustand und
//! schickt Änderungen als Befehle über die Aktions-Warteschlange. Nicht registrierte Vorlagen
//! bleiben leer.

pub mod ai_quads;
pub mod color_picker;
pub mod fields;
pub mod fonts;
pub mod glyphs;
pub mod layouts;
pub mod obj_quads;
pub mod popups;
pub mod scale;
mod quads;
pub use quads::tool_header_lines;
pub use quads::selection_thumb_rect;
pub use quads::export_preset_order;
pub mod texts;
mod tool_quads;

use crate::app_state::{Action, AppState};
use crate::docking;
use crate::panels::PanelId;
use crate::theme::{Theme, ToolGroup};
use crate::widgets;
use egui::{pos2, vec2, Id, Rect, Sense, Stroke, Ui};
use std::collections::HashMap;
use std::sync::OnceLock;

/// Zeichenfunktion eines Quadranten.
pub type QuadrantFn = fn(&mut Ui, &mut AppState, Rect);

/// Register der Quadranten-Inhalte je Vorlagen-ID.
#[derive(Default)]
pub struct QuadrantRegistry {
    map: HashMap<u16, QuadrantFn>,
}

impl QuadrantRegistry {
    /// Register mit den Inhalten aus Phase 3.
    pub fn with_defaults() -> QuadrantRegistry {
        let mut r = QuadrantRegistry::default();
        quads::register(&mut r);
        obj_quads::register(&mut r);
        tool_quads::register(&mut r);
        ai_quads::register(&mut r);
        // Bitmap: Knopf „Nachzeichnen…“ (VT-2)
        r.register(crate::dialogs::trace::INSPECTOR_Q5, crate::dialogs::trace::inspector_q5);
        r
    }

    /// Inhalt für eine Vorlage registrieren (ersetzt einen vorhandenen).
    pub fn register(&mut self, template: u16, f: QuadrantFn) {
        self.map.insert(template, f);
    }

    /// Inhalt zu einer Vorlage.
    pub fn get(&self, template: u16) -> Option<QuadrantFn> {
        self.map.get(&template).copied()
    }

    /// Ist die Vorlage registriert?
    pub fn contains(&self, template: u16) -> bool {
        self.map.contains_key(&template)
    }
}

fn registry() -> &'static QuadrantRegistry {
    static R: OnceLock<QuadrantRegistry> = OnceLock::new();
    R.get_or_init(QuadrantRegistry::with_defaults)
}

/// Laufzeitzustand: zuletzt eingesetzte Vorlagen (Vorlage 0 lässt den Quadranten stehen).
#[derive(Clone, Debug, PartialEq)]
pub struct InspectorState {
    pub quads: [u16; 6],
    pub mask: u16,
    pub number: u16,
}

impl Default for InspectorState {
    fn default() -> Self {
        InspectorState { quads: [14079, 14080, 14081, 14082, 14083, 14084], mask: 73, number: 1 }
    }
}

impl InspectorState {
    /// Layout nach der aktuellen Sitzung aktualisieren.
    pub fn update(&mut self, s: &fp_engine::Session) {
        if let Some(l) = layouts::select_layout(s) {
            for (i, q) in l.quads.iter().enumerate() {
                if *q != 0 {
                    self.quads[i] = *q;
                }
            }
            self.mask = l.mask;
            self.number = l.number;
        }
    }
}

/// Höhe des Inspektors (Kopf + zwei Zeilen bzw. eine Zeile im Halbhöhenmodus).
pub fn height(theme: &Theme, st: &AppState) -> f32 {
    let ws = &st.settings.workspace;
    let header = theme.metrics.inspector_header_height;
    if ws.inspector_collapsed {
        header + 2.0
    } else if ws.inspector_half {
        header + theme.metrics.inspector_row_height + 6.0
    } else {
        header + 2.0 * theme.metrics.inspector_row_height + 8.0
    }
}

/// Nächste Höhe des Inspektors beim Doppelklick auf den Reiter: voll → halb → nur Reiter → voll.
pub fn cycle_height(ws: &mut crate::workspace::Workspace) {
    if ws.inspector_collapsed {
        ws.inspector_collapsed = false;
        ws.inspector_half = false;
    } else if ws.inspector_half {
        ws.inspector_collapsed = true;
        ws.inspector_half = false;
    } else {
        ws.inspector_half = true;
    }
}

/// Zeichnet den Inspektor in die verfügbare Fläche (unterer Bereich).
pub fn show(ui: &mut Ui, st: &mut AppState) {
    // Werkzeugvorgaben, die beim Werkzeugwechsel bzw. beim Beginn eines neuen Texts gelten
    tool_quads::sync_fill_selection(ui.ctx(), st);
    ui.ctx().data_mut(|d| d.remove::<Rect>(Id::new(HSCROLL_ID)));
    obj_quads::apply_text_tool_defaults(ui.ctx(), st);
    let theme = Theme::current(ui.ctx());
    let c = theme.colors.clone();
    let full = ui.max_rect();
    ui.painter().rect_filled(full, 0.0, c.frame_bg);
    let header = Rect::from_min_size(full.min, vec2(full.width(), theme.metrics.inspector_header_height));
    let sym_visible = st.settings.workspace.is_visible(PanelId::SymbolProperties);
    // führende Leerzeichen lassen Platz für das ◊-Einklappsymbol
    let tab_props = format!("  {}", texts::t("pi-tab-properties"));
    let tab_sym = texts::t("pi-tab-symbol-properties");
    let mut tabs: Vec<(&str, ToolGroup)> = vec![(tab_props.as_str(), ToolGroup::Select)];
    if sym_visible {
        tabs.push((tab_sym.as_str(), ToolGroup::Bitmap));
    }
    let active = st.settings.workspace.bottom_active.min(tabs.len() - 1);
    // Platz für das Einklapp-Symbol vor dem ersten Reiter
    let strip_rect = Rect::from_min_max(pos2(header.left() + 2.0, header.top()), header.max);
    widgets::header_bar(ui.painter(), header, &theme);
    let out = docking::tab_strip(ui, strip_rect, Id::new("inspector-tabs"), &tabs, Some(active), true, st.settings.workspace.inspector_collapsed);
    if let Some(r0) = out.tab_rects.first() {
        // ◊-Kennzeichen (Einklappen) im ersten Reiter
        let cx = r0.left() + 5.0;
        let cy = r0.center().y;
        let col = c.tab_text;
        ui.painter().line_segment([pos2(cx - 2.5, cy - 1.0), pos2(cx, cy - 3.5)], Stroke::new(1.0, col));
        ui.painter().line_segment([pos2(cx, cy - 3.5), pos2(cx + 2.5, cy - 1.0)], Stroke::new(1.0, col));
        ui.painter().line_segment([pos2(cx - 2.5, cy + 1.0), pos2(cx, cy + 3.5)], Stroke::new(1.0, col));
        ui.painter().line_segment([pos2(cx, cy + 3.5), pos2(cx + 2.5, cy + 1.0)], Stroke::new(1.0, col));
    }
    if let Some(i) = out.clicked {
        let ws = &mut st.settings.workspace;
        if i == ws.bottom_active {
            ws.inspector_collapsed = !ws.inspector_collapsed;
        } else {
            ws.bottom_active = i;
            ws.inspector_collapsed = false;
        }
    }
    // Doppelklick auf den Reiter bzw. die freie Reiterleiste schaltet wie im Original reihum: volle
    // Höhe → halbe Höhe (nur Zeile 1) → nur Reiter → volle Höhe (PAR 16/11). Die beiden Einzelklicks
    // des Doppelklicks heben sich oben auf.
    let bg_double = out.background.as_ref().is_some_and(|b| b.double_clicked());
    if out.double_clicked.is_some() || bg_double {
        cycle_height(&mut st.settings.workspace);
    }
    // ≡ wie im Original nur „Hilfe“ (die Effektoptionen 6921 stehen unter „+ ▸ Optionen“);
    // Volle/Halbe Höhe und Schließen im Kontextmenü der Reiter (MN-3)
    let mut acts = Vec::new();
    if let Some(resp) = &out.menu_clicked {
        let nodes = crate::menu::build_inspector_menu(st);
        crate::menu::popup_menu(ui, resp, &nodes, &mut acts);
    }
    if !out.tab_resps.is_empty() {
        let nodes = crate::menu::build_inspector_tab_menu(st);
        for r in &out.tab_resps {
            r.context_menu(|ui| crate::menu::show_nodes(ui, &nodes, &mut acts));
        }
    }
    for a in acts {
        if let Action::Run(n, p) = &a
            && n == "ui.inspector.half"
        {
            st.settings.workspace.inspector_half = p.get("half").and_then(|v| v.as_bool()).unwrap_or(false);
            continue;
        }
        st.push(a);
    }
    if st.settings.workspace.inspector_collapsed {
        return;
    }
    let body = Rect::from_min_max(pos2(full.left(), header.bottom() + 1.0), full.max);
    if active == 1 && sym_visible {
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(body.shrink(6.0)));
        crate::panels::show(&mut child, st, PanelId::SymbolProperties);
        return;
    }
    let mut state: InspectorState = ui.ctx().data(|d| d.get_temp(Id::new("inspector-state"))).unwrap_or_default();
    state.update(&st.session);
    ui.ctx().data_mut(|d| d.insert_temp(Id::new("inspector-state"), state.clone()));
    let half = st.settings.workspace.inspector_half;
    if body.width() + 0.5 >= BODY_WIDTH {
        quadrants(ui, st, body, &state, half, &theme);
    } else {
        // schmales Fenster: die Quadranten behalten ihre Breite nach dem Original, der Inspektor
        // bekommt einen waagerechten Bildlauf (statt Quadrant 3/4 abzuschneiden)
        let mut child = ui.new_child(egui::UiBuilder::new().id_salt("inspector-scroll").max_rect(body).layout(egui::Layout::left_to_right(egui::Align::Min)));
        // RAND-2a (N23): schwebende, schmale Leiste über dem 5-px-Rand unter Zeile 2. Die feste
        // Leiste des Designs nahm 10 px von der Höhe und schnitt die untere Zeile ab (Kästchen
        // „Autom. Unterschneidung“). Die Leiste bleibt dünn sichtbar, damit man den Bildlauf
        // erkennt; das Mausrad blättert ohne Umschalttaste waagerecht.
        child.spacing_mut().scroll = hscroll_style();
        child.style_mut().always_scroll_the_only_direction = true;
        let out = egui::ScrollArea::horizontal()
            .id_salt("inspector-hscroll")
            .auto_shrink([false, false])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .show(&mut child, |ui| {
                let (r, _) = ui.allocate_exact_size(vec2(BODY_WIDTH, body.height()), Sense::hover());
                quadrants(ui, st, r, &state, half, &theme);
            });
        ui.ctx().data_mut(|d| d.insert_temp(Id::new(HSCROLL_ID), out.inner_rect));
    }
    // Übernahme durch einen Klick daneben (z. B. auf die Leinwand) sofort ausführen, damit der
    // Wert noch das gezeigte Objekt trifft (die Leinwand verarbeitet den Klick erst danach)
    if fields::take_commit_now(ui.ctx()) {
        st.process_actions(ui.ctx());
    }
    ui.allocate_rect(full, Sense::hover());
}

const HSCROLL_ID: &str = "inspector-hscroll-viewport";

/// Sichtbarer Ausschnitt des waagerechten Bildlaufs im schmalen Fenster (`None`, wenn der
/// Inspektor ganz hineinpasst). Texte, die genau an dieser Kante enden, sind nicht abgeschnitten,
/// sondern liegen nur außerhalb des Ausschnitts (Prüfer `clipping_audit`, RAND-2a).
pub fn hscroll_viewport(ctx: &egui::Context) -> Option<Rect> {
    ctx.data(|d| d.get_temp::<Rect>(Id::new(HSCROLL_ID)))
}

/// Bildlaufleiste des schmalen Inspektors: schwebend (belegt keine Höhe), ruhend 3 px dünn und
/// sichtbar, beim Darüberfahren 6 px.
fn hscroll_style() -> egui::style::ScrollStyle {
    egui::style::ScrollStyle {
        floating: true,
        bar_width: 6.0,
        floating_width: 3.0,
        floating_allocated_width: 0.0,
        bar_inner_margin: 0.0,
        bar_outer_margin: 1.0,
        dormant_background_opacity: 0.0,
        dormant_handle_opacity: 0.7,
        active_background_opacity: 0.4,
        active_handle_opacity: 1.0,
        interact_background_opacity: 0.6,
        interact_handle_opacity: 1.0,
        ..egui::style::ScrollStyle::solid()
    }
}

/// Breite der Quadranten Q1/Q2, Q3/Q4 und Q5/Q6 (Pixel) und die nötige Gesamtbreite.
const W1: f32 = 126.0;
const W3: f32 = 590.0;
const W5: f32 = 180.0;
/// Breite, die der Inspektor ohne Bildlauf braucht.
pub const BODY_WIDTH: f32 = 2.0 + W1 + W3 + W5 + 4.0;

/// Trennlinien und Quadranten in `body` zeichnen.
fn quadrants(ui: &mut Ui, st: &mut AppState, body: Rect, state: &InspectorState, half: bool, theme: &Theme) {
    let c = &theme.colors;
    let rh = theme.metrics.inspector_row_height;
    let top = body.top() + 2.0;
    let w1 = W1;
    let w5 = W5;
    let x0 = body.left() + 2.0;
    let x1 = x0 + w1;
    let x3 = (body.right() - 4.0 - w5).max(x1 + 200.0);
    let x2 = (x1 + W3).min(x3);
    let rects = [
        Rect::from_min_max(pos2(x0, top), pos2(x1, top + rh)),
        Rect::from_min_max(pos2(x0, top + rh), pos2(x1, top + 2.0 * rh)),
        Rect::from_min_max(pos2(x1, top), pos2(x2, top + rh)),
        Rect::from_min_max(pos2(x1, top + rh), pos2(x2, top + 2.0 * rh)),
        Rect::from_min_max(pos2(x2, top), pos2(x2 + w5, top + rh)),
        Rect::from_min_max(pos2(x2, top + rh), pos2(x2 + w5, top + 2.0 * rh)),
    ];
    // Trennlinien (Bit 0..8)
    let line = Stroke::new(1.0, if theme.glow { c.line_dark } else { c.line_mid });
    let light = Stroke::new(1.0, c.line_light);
    let mid_y = top + rh;
    let bits = state.mask;
    let p = ui.painter();
    let hl = |x_a: f32, x_b: f32| {
        p.hline((x_a + 4.0)..=(x_b - 4.0), mid_y - 0.5, line);
        p.hline((x_a + 4.0)..=(x_b - 4.0), mid_y + 0.5, light);
    };
    let vl = |x: f32, y_a: f32, y_b: f32| {
        p.vline(x - 0.5, (y_a + 3.0)..=(y_b - 3.0), line);
        p.vline(x + 0.5, (y_a + 3.0)..=(y_b - 3.0), light);
    };
    if !half {
        if bits & 1 != 0 {
            hl(x0, x1);
        }
        if bits & 8 != 0 {
            hl(x1, x2);
        }
        if bits & 64 != 0 {
            hl(x2, x2 + w5);
        }
    }
    if bits & 2 != 0 {
        vl(x1, top, mid_y);
    }
    if bits & 4 != 0 && !half {
        vl(x1, mid_y, top + 2.0 * rh);
    }
    if bits & 16 != 0 {
        vl(x2, top, mid_y);
    }
    if bits & 32 != 0 && !half {
        vl(x2, mid_y, top + 2.0 * rh);
    }
    if bits & 128 != 0 {
        vl(x2 + w5, top, mid_y);
    }
    if bits & 256 != 0 && !half {
        vl(x2 + w5, mid_y, top + 2.0 * rh);
    }
    for (i, r) in rects.iter().enumerate() {
        if half && i % 2 == 1 {
            continue;
        }
        let t = state.quads[i];
        let reg = registry();
        if let Some(f) = reg.get(t) {
            let mut child = ui.new_child(egui::UiBuilder::new().id_salt(("quad", i, t)).max_rect(*r).layout(egui::Layout::top_down(egui::Align::Min)));
            child.set_clip_rect(r.intersect(ui.clip_rect()));
            // lange (übersetzte) Beschriftungen in die festen Felder einpassen
            fields::set_fit(ui.ctx(), true);
            f(&mut child, st, *r);
            fields::set_fit(ui.ctx(), false);
        } else if cfg!(debug_assertions) {
            let name = layouts::template_name(t);
            if !name.is_empty() {
                ui.painter().text(r.left_top() + vec2(8.0, 6.0), egui::Align2::LEFT_TOP, format!("{name} ({t})"), theme.font(9.5), c.text_disabled);
            }
        }
    }
}

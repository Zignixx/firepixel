//! Kontextmenüs der Leinwand (Rechtsklick) wie das Original: Einträge, Trenner und Untermenüs aus
//! den Menüressourcen (`data\canvas_context.json` aus `re\inventory\resources_de.json`), Zustand
//! und Kürzel aus dem Befehlsregister. Welches Menü erscheint, hängt wie in CS6 vom Werkzeug und
//! von der Auswahl ab:
//!
//! | Menü | Wann |
//! |---|---|
//! | 6914 „Nichts auswählen“ | keine Auswahl |
//! | 6925 „Werkzeug auswählen“ | Objekte ausgewählt |
//! | 6915 „Text auswählen“ | Text ausgewählt bzw. Text-Editor offen |
//! | 6913 „Pixel auswählen“ | Pixelauswahl bzw. Bitmapmodus |
//! | 6916 „Transformationswerkzeug“ | Skalieren/Neigen/Verzerren |
//! | 6910 „Zuschneidewerkzeug“ | Zuschneiden/Exportbereich |
//! | 6906 „Zoom“ | Zoom-Werkzeug |
//! | 49120 „Pipette“ | Pipette |
//!
//! Ein Rechtsklick auf ein nicht ausgewähltes Objekt wählt es zuerst aus (wie CS6).

use crate::app_state::{Action, AppState};
use crate::menu::{clean_label, ItemAction, MenuEntry, Node};
use fp_engine::ToolId;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::OnceLock;

#[derive(Deserialize)]
struct ContextFile {
    context: BTreeMap<String, Vec<MenuEntry>>,
}

fn file() -> &'static ContextFile {
    static F: OnceLock<ContextFile> = OnceLock::new();
    F.get_or_init(|| serde_json::from_str(include_str!("../../data/canvas_context.json")).unwrap_or(ContextFile { context: BTreeMap::new() }))
}

/// Einträge eines Kontextmenüs (`"6914"` …).
pub fn entries(id: &str) -> &'static [MenuEntry] {
    file().context.get(id).and_then(|v| v.first()).and_then(|m| m.children.as_deref()).unwrap_or(&[])
}

/// Offenes Kontextmenü.
#[derive(Clone, Debug, PartialEq)]
pub struct ContextMenu {
    /// Ressourcen-ID des Menüs.
    pub id: &'static str,
    /// Rechtsklick-Punkt in Dokumentkoordinaten (Pipette: Farbe an dieser Stelle).
    pub at: Option<fp_geom::Point>,
}

/// Menü für den aktuellen Zustand.
pub fn menu_for(st: &AppState) -> &'static str {
    let tool = st.session.tools.active;
    let bitmap_mode = st.session.active().is_some_and(|d| d.bitmap_mode);
    match tool {
        // Zuschneiden im Bitmapmodus: „Bitmap-Zuschneidevorgang abbrechen“
        ToolId::Crop if bitmap_mode => return "6912",
        ToolId::Crop | ToolId::ExportArea => return "6910",
        ToolId::Scale | ToolId::Skew | ToolId::Distort | ToolId::NineSliceScale => return "6916",
        ToolId::Zoom => return "6906",
        ToolId::Eyedropper => return "49120",
        _ => {}
    }
    if st.session.tool_is_editing_text() {
        return "6915";
    }
    let Some(d) = st.session.active() else { return "6914" };
    if d.bitmap_mode || d.selection.has_pixels() {
        return "6913";
    }
    if d.selection.elements.is_empty() {
        return "6914";
    }
    let all_text = d
        .selection
        .elements
        .iter()
        .filter_map(|id| fp_engine::docops::element_on_page(&d.doc, d.page, *id))
        .all(|e| matches!(e.kind, fp_model::ElementKind::Text(_)));
    if all_text { "6915" } else { "6925" }
}

fn state(st: &AppState, name: &str, fallback_sc: Option<String>) -> (bool, Option<bool>, Option<String>) {
    let sc = st.session.command_shortcuts(name).first().map(|s| s.display_de()).or(fallback_sc);
    if let Some((en, ck)) = crate::actions::ui_command_state(st, name) {
        return (en, ck, sc);
    }
    let (en, ck) = st.session.command_state(name);
    (en, ck, sc)
}

/// Aufbereiteter Menübaum (Zustand je Frame).
pub fn build(st: &AppState, entries: &[MenuEntry]) -> Vec<Node> {
    let mut out = Vec::new();
    for e in entries {
        if e.sep {
            if !out.is_empty() && !matches!(out.last(), Some(Node::Separator)) {
                out.push(Node::Separator);
            }
            continue;
        }
        let (label, sc) = clean_label(&e.text);
        if let Some(children) = &e.children {
            let kids = build(st, children);
            let enabled = kids.iter().any(|k| matches!(k, Node::Item { enabled: true, .. } | Node::Sub { enabled: true, .. }));
            out.push(Node::Sub { label, children: kids, enabled });
            continue;
        }
        // Zuschneidewerkzeug: „Dokument zuschneiden“ übernimmt den Rahmen (Eingabe), „Zuschneiden
        // abbrechen“ verwirft ihn (Esc) – beides an das laufende Werkzeug
        let crop_tool = matches!(st.session.tools.active, ToolId::Crop | ToolId::ExportArea);
        let tool_key = match e.id {
            Some(35145) if crop_tool => Some("enter"),
            Some(35344) | Some(35345) => Some("escape"),
            _ => None,
        };
        if let Some(k) = tool_key {
            out.push(Node::Item { label, shortcut: sc, action: ItemAction::Ui(Action::Run("ui.toolKey".into(), serde_json::json!({"key": k}))), enabled: true, checked: None });
            continue;
        }
        let name = e.id.and_then(|id| st.session.registry().by_fw_id(id).map(|c| c.info().name.clone()));
        match name {
            Some(n) => {
                let (enabled, checked, shortcut) = state(st, &n, sc);
                out.push(Node::Item { label, shortcut, action: ItemAction::Command(n), enabled, checked });
            }
            None => {
                // Pipette: Farbe kopieren (ohne Registerbefehl, Laufzeit über die Oberfläche)
                let action = match e.id {
                    Some(49121) => ItemAction::Ui(Action::Run("ui.copyColor".into(), serde_json::json!({"which": "sample"}))),
                    Some(49122) => ItemAction::Ui(Action::Run("ui.copyColor".into(), serde_json::json!({"which": "fill"}))),
                    Some(49123) => ItemAction::Ui(Action::Run("ui.copyColor".into(), serde_json::json!({"which": "stroke"}))),
                    // „Ausgewähltes Segment exportieren…“ / „CSS-Sprites exportieren…“: Export-Dialog
                    Some(35413) | Some(49124) => ItemAction::Ui(Action::Run("file.export".into(), serde_json::json!({}))),
                    _ => ItemAction::None,
                };
                let enabled = action != ItemAction::None;
                out.push(Node::Item { label, shortcut: sc, action, enabled, checked: None });
            }
        }
    }
    while matches!(out.last(), Some(Node::Separator)) {
        out.pop();
    }
    out
}

/// Rechtsklick an `at` (Dokumentkoordinaten): ein nicht ausgewähltes Objekt unter dem Zeiger wird
/// zuerst ausgewählt (wie CS6), danach das passende Menü bestimmt.
pub fn open_at(st: &mut AppState, at: fp_geom::Point) -> ContextMenu {
    let tool = st.session.tools.active;
    let selects = matches!(tool, ToolId::Pointer | ToolId::SelectBehind | ToolId::Subselection) && !st.session.tool_is_editing_text();
    if selects && let Some(d) = st.session.active() {
        let opt = fp_engine::docops::HitOptions { tolerance: st.session.prefs.pick_distance() / d.view.zoom.max(1e-6), ..Default::default() };
        let hit = fp_engine::docops::hits_at(&d.doc, d.page, d.frame, at, st.session.geometry(), &opt).first().and_then(|p| d.doc.element_at(p).map(|e| e.id));
        if let Some(id) = hit
            && !d.selection.contains(id)
        {
            // sofort ausführen: das Menü hängt von der neuen Auswahl ab
            if let Err(e) = st.session.execute("select.set", serde_json::json!({"ids": [id.0], "mode": "replace"})) {
                st.push(Action::Status(e.to_string()));
            }
        }
    }
    ContextMenu { id: menu_for(st), at: Some(at) }
}

/// Farbe als `#RRGGBB`.
fn hex(c: fp_color::Argb) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b())
}

/// Pipetten-Kontextmenü: Farbe an der Klickstelle, Füll- bzw. Strichfarbe in die Zwischenablage.
fn copy_color(ctx: &egui::Context, st: &mut AppState, which: &str, at: Option<fp_geom::Point>) {
    let c = match which {
        "fill" => Some(st.session.tools.fill_color),
        "stroke" => Some(st.session.tools.stroke_color),
        _ => at.and_then(|p| {
            let d = st.session.active()?;
            let r = fp_geom::IRect::new(p.x.floor() as i32, p.y.floor() as i32, p.x.floor() as i32 + 1, p.y.floor() as i32 + 1);
            let img = st.renderer.render_region(&d.doc, d.page, d.frame, r, 1.0).image;
            img.get(r.left, r.top).map(|px| fp_color::Argb(((px[3] as u32) << 24) | ((px[0] as u32) << 16) | ((px[1] as u32) << 8) | px[2] as u32))
        }),
    };
    match c {
        Some(c) => {
            ctx.copy_text(hex(c));
            st.push(Action::Status(format!("Farbe {} kopiert.", hex(c))));
        }
        None => st.push(Action::Status("Keine Farbe an dieser Stelle.".into())),
    }
}

/// Zeigt das Kontextmenü am Antwortbereich der Leinwand (öffnet sich per Rechtsklick).
pub fn show(ui: &egui::Ui, resp: &egui::Response, st: &mut AppState, menu: Option<&ContextMenu>) {
    use egui::containers::menu::MenuConfig;
    use egui::{Popup, UiKind, UiStackInfo};
    let theme = crate::theme::Theme::current(ui.ctx());
    let Some(menu) = menu else { return };
    let nodes = build(st, entries(menu.id));
    let mut acts: Vec<Action> = Vec::new();
    Popup::context_menu(resp)
        .frame(crate::menu::menu_frame(&theme))
        .info(UiStackInfo::new(UiKind::Menu).with_tag_value(MenuConfig::MENU_CONFIG_TAG, MenuConfig::new()))
        .show(|ui| crate::menu::show_nodes(ui, &nodes, &mut acts));
    for a in acts {
        apply(ui.ctx(), st, a, menu.at);
    }
}

/// Führt einen gewählten Kontextmenü-Eintrag aus (Oberflächen-Einträge direkt, Befehle über die
/// Aktions-Warteschlange).
pub fn apply(ctx: &egui::Context, st: &mut AppState, a: Action, at: Option<fp_geom::Point>) {
    match a {
        Action::Run(name, p) if name == "ui.toolKey" => {
            let k = if p.get("key").and_then(|v| v.as_str()) == Some("enter") { fp_engine::tools::ToolKey::Enter } else { fp_engine::tools::ToolKey::Escape };
            let r = st.session.tool_event(&fp_engine::ToolEvent::key(k));
            super::tool_result(ctx, st, r);
        }
        Action::Run(name, p) if name == "ui.copyColor" => {
            let which = p.get("which").and_then(|v| v.as_str()).unwrap_or("sample").to_string();
            copy_color(ctx, st, &which, at);
        }
        a => st.push(a),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Jeder Eintrag mit ID in den Leinwand-Kontextmenüs hat einen Befehl im Register oder wird von
    /// der Oberfläche selbst ausgeführt (Pipette, Zuschneiden, Export-Dialog).
    #[test]
    fn every_context_entry_has_an_action() {
        fn walk(es: &[MenuEntry], out: &mut Vec<(u32, String)>) {
            for e in es {
                if let Some(c) = &e.children {
                    walk(c, out);
                } else if let Some(id) = e.id {
                    out.push((id, e.text.clone()));
                }
            }
        }
        let s = fp_engine::Session::default();
        let reg = s.registry();
        let ui_handled = [49121, 49122, 49123, 35145, 35344, 35345, 35413, 49124];
        let mut missing = Vec::new();
        for id in ["6906", "6910", "6912", "6913", "6914", "6915", "6916", "6925", "49120"] {
            let mut v = Vec::new();
            walk(entries(id), &mut v);
            for (fw, text) in v {
                if reg.by_fw_id(fw).is_none() && !ui_handled.contains(&fw) {
                    missing.push(format!("{id}: {fw} {text}"));
                }
            }
        }
        assert!(missing.is_empty(), "ohne Befehl: {missing:?}");
    }

    #[test]
    fn resources_have_all_canvas_menus() {
        for id in ["6906", "6910", "6912", "6913", "6914", "6915", "6916", "6925", "49120"] {
            assert!(!entries(id).is_empty(), "Menü {id} fehlt");
        }
        // „Nichts auswählen“ beginnt mit Einfügen, „Werkzeug auswählen“ mit Bearbeiten ▸
        assert_eq!(clean_label(&entries("6914")[0].text).0, "Einfügen");
        assert!(entries("6925")[0].children.is_some());
    }
}

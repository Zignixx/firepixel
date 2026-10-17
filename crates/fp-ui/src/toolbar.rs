//! Hauptsymbolleiste „Standard“ (Fenster ▸ Werkzeugleisten ▸ Standard, `MainToolBar`): Datei,
//! Bearbeiten | Gruppieren, Pfade, Anordnen | Symbol, Ausrichten ▾ | Drehen, Spiegeln – in der
//! Reihenfolge von CS6 (`re\ui-referenz\01_hauptfenster.png`).

use crate::app_state::{Action, AppState};
use crate::icons::Icon;
use crate::menu::{self, ItemAction, Node};
use crate::theme::{Theme, ToolGroup};
use crate::widgets::{self, ButtonLook};
use egui::{Rect, Sense, Stroke, Ui, pos2, vec2};
use serde_json::json;

/// Eintrag der Symbolleiste.
enum Entry {
    Button(Icon, &'static str, ToolGroup),
    AlignMenu,
    Separator,
}

const ENTRIES: &[Entry] = &[
    Entry::Button(Icon::New, "file.new", ToolGroup::Web),
    Entry::Button(Icon::Save, "file.save", ToolGroup::Web),
    Entry::Button(Icon::Open, "file.open", ToolGroup::Web),
    Entry::Button(Icon::Import, "file.import", ToolGroup::Web),
    Entry::Button(Icon::Export, "file.export", ToolGroup::Web),
    Entry::Button(Icon::Print, "file.print", ToolGroup::Web),
    Entry::Button(Icon::Undo, "edit.undo", ToolGroup::View),
    Entry::Button(Icon::Redo, "edit.redo", ToolGroup::View),
    Entry::Button(Icon::Cut, "edit.cut", ToolGroup::Bitmap),
    Entry::Button(Icon::Copy, "edit.copy", ToolGroup::Bitmap),
    Entry::Button(Icon::Paste, "edit.paste", ToolGroup::Bitmap),
    Entry::Separator,
    Entry::Button(Icon::Group, "modify.group", ToolGroup::Select),
    Entry::Button(Icon::Ungroup, "modify.ungroup", ToolGroup::Select),
    Entry::Button(Icon::Join, "modify.combinePaths.join", ToolGroup::Vector),
    Entry::Button(Icon::Split, "modify.combinePaths.split", ToolGroup::Vector),
    Entry::Button(
        Icon::BringToFront,
        "modify.arrange.bringToFront",
        ToolGroup::Colors,
    ),
    Entry::Button(
        Icon::BringForward,
        "modify.arrange.bringForward",
        ToolGroup::Colors,
    ),
    Entry::Button(
        Icon::SendBackward,
        "modify.arrange.sendBackward",
        ToolGroup::Colors,
    ),
    Entry::Button(
        Icon::SendToBack,
        "modify.arrange.sendToBack",
        ToolGroup::Colors,
    ),
    Entry::Separator,
    Entry::Button(
        Icon::EditSymbol,
        "modify.symbol.editSymbol",
        ToolGroup::Select,
    ),
    Entry::AlignMenu,
    Entry::Separator,
    Entry::Button(
        Icon::RotateCcw,
        "modify.transform.rotate90DegreeCcw",
        ToolGroup::Select,
    ),
    Entry::Button(
        Icon::RotateCw,
        "modify.transform.rotate90DegreeCw",
        ToolGroup::Select,
    ),
    Entry::Button(
        Icon::FlipHorizontal,
        "modify.transform.flipHorizontal",
        ToolGroup::Select,
    ),
    Entry::Button(
        Icon::FlipVertical,
        "modify.transform.flipVertical",
        ToolGroup::Select,
    ),
];

/// Befehl im Register (Namen aus dem Katalog; `Ccw`/`CCW`-Schreibweisen werden beide akzeptiert).
fn resolve(st: &AppState, name: &str) -> Option<String> {
    if st.session.registry().get(name).is_some() {
        return Some(name.to_string());
    }
    let lower = name.to_lowercase();
    st.session
        .registry()
        .iter()
        .find(|c| c.info().name.to_lowercase() == lower)
        .map(|c| c.info().name.clone())
}

/// Werkzeugtipp „Beschriftung (Kürzel)“ in der aktuellen Sprache (`cmd-…` aus `menu.ftl`).
fn tooltip(st: &AppState, name: &str) -> String {
    if st.session.registry().get(name).is_none() {
        return String::new();
    }
    let mut t = menu::command_label(st, name);
    if let Some(sc) = st.session.command_shortcuts(name).first() {
        t.push_str(&format!(" ({})", menu::shortcut_text(sc)));
    }
    t
}

/// Einträge des Ausrichten-Menüs (Trenner = leerer Name).
const ALIGN_ITEMS: [&str; 10] = [
    "modify.align.left",
    "modify.align.centerVertical",
    "modify.align.right",
    "",
    "modify.align.top",
    "modify.align.centerHorizontal",
    "modify.align.bottom",
    "",
    "modify.align.distributeWidths",
    "modify.align.distributeHeights",
];

/// Zeichnet die Symbolleiste in die verfügbare Fläche.
pub fn show(ui: &mut Ui, st: &mut AppState) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, 0.0, c.frame_bg);
    ui.painter().hline(
        rect.x_range(),
        rect.top() + 0.5,
        Stroke::new(1.0, c.line_dark),
    );
    ui.painter().hline(
        rect.x_range(),
        rect.top() + 1.5,
        Stroke::new(1.0, c.line_light),
    );
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0, c.line_dark),
    );
    widgets::grip(
        ui.painter(),
        Rect::from_min_max(
            pos2(rect.left() + 4.0, rect.top() + 5.0),
            pos2(rect.left() + 8.0, rect.bottom() - 4.0),
        ),
        true,
        &theme,
    );
    let size = vec2(
        theme.metrics.toolbar_button + 2.0,
        theme.metrics.toolbar_button - 4.0,
    );
    let mut x = rect.left() + 13.0;
    let y = rect.center().y - size.y / 2.0 + 1.0;
    let studio = crate::batch::is_front(st);
    for e in ENTRIES {
        match e {
            Entry::Separator => {
                let sx = x + 4.0;
                ui.painter().vline(
                    sx + 0.5,
                    (rect.top() + 6.0)..=(rect.bottom() - 5.0),
                    Stroke::new(1.0, c.line_mid),
                );
                ui.painter().vline(
                    sx + 1.5,
                    (rect.top() + 6.0)..=(rect.bottom() - 5.0),
                    Stroke::new(1.0, c.line_light),
                );
                x += 10.0;
            }
            Entry::Button(icon, name, group) => {
                let r = Rect::from_min_size(pos2(x, y), size);
                let resolved = resolve(st, name);
                // Batch-Studio vorn: nur „Neu“ und „Öffnen“ (holen ein Dokument nach vorn); alles andere
                // würde unsichtbar auf das verdeckte Dokument wirken (BATCH-T)
                let studio_blocks = studio && !matches!(*name, "file.new" | "file.open");
                let enabled = !studio_blocks
                    && resolved.as_deref().is_some_and(|n| {
                        st.session.is_available(n)
                            || crate::actions::ui_command_state(st, n).is_some_and(|s| s.0)
                    });
                let tip = resolved
                    .as_deref()
                    .map(|n| tooltip(st, n))
                    .unwrap_or_default();
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r));
                let resp = widgets::icon_button(
                    &mut child,
                    *icon,
                    size,
                    theme.metrics.icon_size,
                    ButtonLook {
                        selected: false,
                        enabled,
                        group: Some(*group),
                    },
                    &tip,
                );
                if resp.clicked()
                    && let Some(n) = resolved
                {
                    st.push(Action::Run(n, json!({})));
                }
                x += size.x;
            }
            Entry::AlignMenu => {
                let r = Rect::from_min_size(pos2(x, y), vec2(size.x + 12.0, size.y));
                let resp = ui.interact(r, egui::Id::new("tb-align"), Sense::click());
                let any = !studio
                    && ["modify.align.left", "modify.align.top"]
                        .iter()
                        .any(|n| st.session.is_available(n));
                widgets::button_frame(
                    ui.painter(),
                    r,
                    &resp,
                    ButtonLook {
                        selected: false,
                        enabled: true,
                        group: Some(ToolGroup::Select),
                    },
                    &theme,
                );
                let ir =
                    Rect::from_center_size(pos2(r.left() + 14.0, r.center().y), vec2(16.0, 16.0));
                crate::icons::paint(
                    ui.painter(),
                    ir,
                    Icon::AlignLeft,
                    &crate::icons::IconStyle::new(&theme, Some(ToolGroup::Select), any),
                );
                widgets::drop_arrow(
                    ui.painter(),
                    Rect::from_center_size(pos2(r.right() - 8.0, r.center().y), vec2(8.0, 8.0)),
                    c.icon_fg,
                );
                crate::ui_kit::icon_button_info(
                    &resp,
                    true,
                    &menu::clean_label(&fp_i18n::t("menu-modify-align")).0,
                    None,
                );
                let resp =
                    resp.on_hover_text(menu::clean_label(&fp_i18n::t("menu-modify-align")).0);
                let nodes: Vec<Node> = ALIGN_ITEMS
                    .iter()
                    .map(|n| {
                        if n.is_empty() {
                            Node::Separator
                        } else {
                            Node::Item {
                                label: menu::command_label(st, n),
                                shortcut: st
                                    .session
                                    .command_shortcuts(n)
                                    .first()
                                    .map(menu::shortcut_text),
                                action: ItemAction::Command(n.to_string()),
                                enabled: !studio && st.session.is_available(n),
                                checked: None,
                            }
                        }
                    })
                    .collect();
                let mut acts = Vec::new();
                menu::popup_menu(ui, &resp, &nodes, &mut acts);
                for a in acts {
                    st.push(a);
                }
                x += r.width();
            }
        }
    }
    ui.allocate_rect(rect, Sense::hover());
}

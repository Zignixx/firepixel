//! Anwendungsleiste wie CS6 (Titel und Menüs in einer Zeile, eigene Fensterknöpfe): Logo, Menüs,
//! Schnellknöpfe (Durchsuchen, Hand, Zoom, Vergrößerung), Arbeitsbereich-Umschalter und
//! Minimieren/Maximieren/Schließen. Leere Flächen verschieben das Fenster, Doppelklick maximiert.

use crate::app_state::{Action, AppState};
use crate::icons::{self, Icon, IconStyle};
use crate::menu::{self, ItemAction, Node};
use crate::theme::{Theme, ToolGroup};
use crate::widgets::{self, ButtonLook};
use crate::workspace::Workspace;
use egui::{Align2, Id, Rect, Sense, Stroke, StrokeKind, Ui, ViewportCommand, pos2, vec2};
use fp_engine::ToolId;
use serde_json::json;

/// Zeichnet die Anwendungsleiste. Platzvergabe bei schmalen Fenstern (nichts überlappt): Logo,
/// Menüs und Fensterknöpfe immer; danach der Arbeitsbereich-Umschalter (sonst nur ▾), dann das
/// Vergrößerungsfeld, zuletzt die Schnellknöpfe.
pub fn show(ui: &mut Ui, st: &mut AppState) {
    let theme = Theme::current(ui.ctx());
    let c = theme.colors.clone();
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, 0.0, c.app_bar_bg);
    // Hintergrund zum Verschieben (vor allen Knöpfen anlegen, damit diese gewinnen)
    let drag = ui.interact(rect, Id::new("appbar-drag"), Sense::click_and_drag());
    if drag.drag_started() {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
    if drag.double_clicked() {
        let max = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(!max));
    }
    // eigenes App-Icon (24 px pixelgenau), in allen Designs gleich
    let logo = Rect::from_min_size(
        pos2(rect.left() + 5.0, (rect.center().y - 12.0).round()),
        vec2(24.0, 24.0),
    );
    icons::paint_app_logo(ui.painter(), logo);
    // Fensterknöpfe rechts oben (wie CS6 klein am oberen Rand)
    let wb = vec2(30.0, 19.0);
    let close = Rect::from_min_size(
        pos2(rect.right() - wb.x * 1.5 - 4.0, rect.top()),
        vec2(wb.x * 1.5, wb.y),
    );
    let maxr = Rect::from_min_size(pos2(close.left() - wb.x, rect.top()), wb);
    let minr = Rect::from_min_size(pos2(maxr.left() - wb.x, rect.top()), wb);
    // Menüs (höchstens bis vor einen kompakten Umschalter)
    let compact_w = 24.0;
    let menu_rect = Rect::from_min_max(
        pos2(logo.right() + 8.0, rect.top() + 4.0),
        pos2(
            (minr.left() - compact_w - 16.0).max(logo.right() + 8.0),
            rect.bottom() - 4.0,
        ),
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(menu_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    child.spacing_mut().item_spacing = vec2(0.0, 0.0);
    child.set_clip_rect(menu_rect);
    menu::show_menu_bar(&mut child, st);
    let menus_end = child.min_rect().right().min(menu_rect.right());
    // Platz zwischen Menüs und Fensterknöpfen verteilen
    let font = theme.menu_font();
    let name = workspace_display_name(&st.settings.workspace.name);
    let tw = widgets::text_width(ui, &name, &font);
    let space = minr.left() - 14.0 - menus_end;
    let switcher_full = tw + 26.0 + 16.0;
    let full_switcher = space >= switcher_full;
    let switcher_w = if full_switcher { tw + 26.0 } else { compact_w };
    let rest = space - switcher_w - 16.0;
    let quick_w = 12.0 + 3.0 * 34.0 + 96.0;
    let zoom_w = 12.0 + 96.0;
    let show_quick = rest >= quick_w;
    let show_zoom = rest >= zoom_w;
    let mut x = menus_end + 10.0;
    if show_zoom {
        ui.painter().vline(
            x + 0.5,
            (rect.top() + 7.0)..=(rect.bottom() - 7.0),
            Stroke::new(1.0, c.line_mid),
        );
        ui.painter().vline(
            x + 1.5,
            (rect.top() + 7.0)..=(rect.bottom() - 7.0),
            Stroke::new(1.0, c.line_light),
        );
        x += 12.0;
    }
    // Schnellknöpfe
    if show_quick {
        let bsize = vec2(26.0, 22.0);
        let y = rect.center().y - bsize.y / 2.0;
        let quick: [(Icon, String, Action, bool); 3] = [
            (
                Icon::Browse,
                fp_i18n::t("ui-appbar-browse"),
                Action::Run("file.open".into(), json!({})),
                false,
            ),
            (
                Icon::Hand,
                fp_i18n::t("ui-appbar-hand-tool"),
                Action::SetTool(ToolId::Hand),
                st.session.tools.active == ToolId::Hand,
            ),
            (
                Icon::Zoom,
                fp_i18n::t("ui-appbar-zoom-tool"),
                Action::SetTool(ToolId::Zoom),
                st.session.tools.active == ToolId::Zoom,
            ),
        ];
        for (icon, tip, act, selected) in quick {
            let r = Rect::from_min_size(pos2(x, y), bsize);
            let mut b = ui.new_child(egui::UiBuilder::new().max_rect(r));
            if widgets::icon_button(
                &mut b,
                icon,
                bsize,
                16.0,
                ButtonLook {
                    selected,
                    enabled: true,
                    group: Some(ToolGroup::View),
                },
                &tip,
            )
            .clicked()
            {
                st.push(act);
            }
            x += bsize.x + 8.0;
        }
    }
    // Vergrößerung
    if show_zoom {
        let zr = Rect::from_min_size(pos2(x + 2.0, rect.top() + 6.0), vec2(92.0, 19.0));
        let zoom = st.session.active().map(|d| d.view.zoom);
        ui.painter().rect_filled(zr, 0.0, c.field_bg);
        ui.painter().rect_stroke(
            zr,
            0.0,
            Stroke::new(1.0, c.field_border),
            StrokeKind::Inside,
        );
        let btn = Rect::from_min_max(
            pos2(zr.right() - 16.0, zr.top() + 1.0),
            zr.max - vec2(1.0, 1.0),
        );
        ui.painter().rect_filled(btn, 0.0, c.button_bg);
        ui.painter().vline(
            btn.left() + 0.5,
            btn.y_range(),
            Stroke::new(1.0, c.field_border),
        );
        widgets::drop_arrow(ui.painter(), btn, c.text);
        let ztext = zoom
            .map(|z| format!("{:.0}%", z * 100.0))
            .unwrap_or_default();
        ui.painter().text(
            pos2(zr.left() + 6.0, zr.center().y),
            Align2::LEFT_CENTER,
            ztext,
            theme.small_font(),
            if zoom.is_some() {
                c.field_text
            } else {
                c.text_disabled
            },
        );
        let zresp = ui
            .interact(zr, Id::new("appbar-zoom"), Sense::click())
            .on_hover_text(fp_i18n::t("ui-zoom"));
        if zoom.is_some() {
            let nodes: Vec<Node> = [
                6.0f64, 12.0, 25.0, 50.0, 66.0, 100.0, 150.0, 200.0, 300.0, 400.0, 800.0, 1600.0,
                3200.0, 6400.0,
            ]
            .iter()
            .map(|p| {
                let z = match *p as i64 {
                    6 => 0.0625,
                    12 => 0.125,
                    66 => 0.66,
                    _ => p / 100.0,
                };
                Node::Item {
                    label: format!("{p:.0}%"),
                    shortcut: None,
                    action: ItemAction::Ui(Action::SetZoom(z)),
                    enabled: true,
                    checked: Some(zoom == Some(z)),
                }
            })
            .collect();
            let mut acts = Vec::new();
            menu::popup_menu(ui, &zresp, &nodes, &mut acts);
            for a in acts {
                st.push(a);
            }
        }
    }
    let frame = Rect::from_min_max(minr.min, close.max);
    ui.painter().rect_stroke(
        frame,
        egui::CornerRadius {
            nw: 0,
            ne: 0,
            sw: 3,
            se: 3,
        },
        Stroke::new(1.0, c.line_mid),
        StrokeKind::Inside,
    );
    let maximized = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
    for (r, icon, cmd, tip) in [
        (minr, Icon::Minimize, 0, fp_i18n::t("ui-window-minimize")),
        (
            maxr,
            if maximized {
                Icon::Restore
            } else {
                Icon::Maximize
            },
            1,
            fp_i18n::t(if maximized {
                "ui-window-restore"
            } else {
                "ui-window-maximize"
            }),
        ),
        (close, Icon::WindowClose, 2, fp_i18n::t("ui-window-close")),
    ] {
        let resp = ui.interact(r, Id::new(("winbtn", cmd)), Sense::click());
        if resp.hovered() {
            ui.painter().rect_filled(
                r.shrink(1.0),
                0.0,
                if cmd == 2 {
                    c.window_close_hover
                } else {
                    c.window_button_hover
                },
            );
        }
        if cmd > 0 {
            ui.painter()
                .vline(r.left() + 0.5, r.y_range(), Stroke::new(1.0, c.line_mid));
        }
        let col = if cmd == 2 && resp.hovered() {
            egui::Color32::WHITE
        } else {
            c.app_bar_text
        };
        crate::ui_kit::icon_button_info(&resp, true, &tip, None);
        icons::paint(
            ui.painter(),
            Rect::from_center_size(r.center(), vec2(12.0, 12.0)),
            icon,
            &IconStyle::mono(col, c.app_bar_bg),
        );
        if resp.on_hover_text(tip).clicked() {
            match cmd {
                0 => ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true)),
                1 => ui
                    .ctx()
                    .send_viewport_cmd(ViewportCommand::Maximized(!maximized)),
                _ => st.push(Action::Quit),
            }
        }
    }
    // Arbeitsbereich-Umschalter („Einen Arbeitsbereich auswählen“), bei Platzmangel nur ▾
    let wr = Rect::from_min_size(
        pos2(minr.left() - 14.0 - switcher_w, rect.top() + 5.0),
        vec2(switcher_w, 21.0),
    );
    ui.painter().vline(
        wr.left() - 8.5,
        (rect.top() + 7.0)..=(rect.bottom() - 7.0),
        Stroke::new(1.0, c.line_mid),
    );
    let wresp = ui.interact(wr, Id::new("workspace-switcher"), Sense::click());
    if wresp.hovered() {
        ui.painter().rect_filled(wr, 0.0, c.app_bar_hover_bg);
    }
    if full_switcher {
        ui.painter().text(
            pos2(wr.left() + 4.0, wr.center().y),
            Align2::LEFT_CENTER,
            &name,
            font,
            c.app_bar_text,
        );
    }
    widgets::drop_arrow(
        ui.painter(),
        Rect::from_center_size(pos2(wr.right() - 8.0, wr.center().y + 2.0), vec2(8.0, 8.0)),
        c.text_weak,
    );
    let wresp = wresp.on_hover_text(fp_i18n::tr!("ui-workspace-switcher", name = name.as_str()));
    let nodes = workspace_menu(st);
    let mut acts = Vec::new();
    menu::popup_menu(ui, &wresp, &nodes, &mut acts);
    for a in acts {
        st.push(a);
    }
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0, c.line_dark),
    );
}

/// Einträge „Arbeitsbereichlayouts“ (Umschalter oben rechts und Fenster-Menü): mitgelieferte Layouts,
/// gespeicherte Layouts (eframe-Speicher und XML-Dateien im Layoutordner), Aktuelles speichern,
/// Standardarbeitsbereich.
pub fn workspace_menu(st: &AppState) -> Vec<Node> {
    let name = st.settings.workspace.name.clone();
    let entry = |n: &str| Node::Item {
        label: workspace_display_name(n),
        shortcut: None,
        action: ItemAction::Ui(Action::LoadWorkspace(n.to_string())),
        enabled: true,
        checked: Some(n == name),
    };
    let mut nodes: Vec<Node> = Workspace::presets()
        .iter()
        .map(|w| entry(&w.name))
        .collect();
    let mut saved: Vec<String> = st
        .settings
        .named_workspaces
        .iter()
        .map(|(n, _)| n.clone())
        .collect();
    if let Some(dir) = crate::workspace::layouts_dir()
        && let Ok(list) = std::fs::read_dir(dir)
    {
        for e in list.flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("xml"))
                && let Some(stem) = p.file_stem().map(|s| s.to_string_lossy().to_string())
                && !saved.contains(&stem)
            {
                saved.push(stem);
            }
        }
    }
    if !saved.is_empty() {
        nodes.push(Node::Separator);
        for n in &saved {
            nodes.push(entry(n));
        }
    }
    nodes.push(Node::Separator);
    nodes.push(Node::Item {
        label: fp_i18n::t("ui-workspace-save-current"),
        shortcut: None,
        action: ItemAction::Ui(Action::Run("ui.saveWorkspace".into(), json!({}))),
        enabled: true,
        checked: None,
    });
    nodes.push(Node::Item {
        label: fp_i18n::t("ui-workspace-default"),
        shortcut: None,
        action: ItemAction::Ui(Action::ResetWorkspace),
        enabled: true,
        checked: None,
    });
    nodes
}

/// Angezeigter Name eines Arbeitsbereichs: die mitgelieferten Layouts in der aktuellen Sprache
/// (gespeichert wird ihre feste Kennung, z. B. „Erweiterter Modus“), eigene Layouts unverändert.
pub fn workspace_display_name(name: &str) -> String {
    match crate::workspace::preset_key(name) {
        Some(key) => fp_i18n::t(key),
        None => name.to_string(),
    }
}

/// Größenänderung am Fensterrand (ohne Systemrahmen): Mauszeiger setzen und Ziehen starten.
pub fn resize_edges(ctx: &egui::Context) {
    let maximized = ctx.input(|i| {
        i.viewport().maximized.unwrap_or(false) || i.viewport().fullscreen.unwrap_or(false)
    });
    if maximized {
        return;
    }
    let screen = ctx.content_rect();
    let Some(p) = ctx.pointer_hover_pos() else {
        return;
    };
    let m = 5.0;
    let (l, r, t, b) = (
        p.x - screen.left() < m,
        screen.right() - p.x < m,
        p.y - screen.top() < m,
        screen.bottom() - p.y < m,
    );
    use egui::viewport::ResizeDirection as D;
    let dir = match (l, r, t, b) {
        (true, _, true, _) => Some(D::NorthWest),
        (_, true, true, _) => Some(D::NorthEast),
        (true, _, _, true) => Some(D::SouthWest),
        (_, true, _, true) => Some(D::SouthEast),
        (true, _, _, _) => Some(D::West),
        (_, true, _, _) => Some(D::East),
        (_, _, true, _) => Some(D::North),
        (_, _, _, true) => Some(D::South),
        _ => None,
    };
    if let Some(d) = dir {
        let icon = match d {
            D::NorthWest | D::SouthEast => egui::CursorIcon::ResizeNwSe,
            D::NorthEast | D::SouthWest => egui::CursorIcon::ResizeNeSw,
            D::West | D::East => egui::CursorIcon::ResizeHorizontal,
            _ => egui::CursorIcon::ResizeVertical,
        };
        ctx.set_cursor_icon(icon);
        if ctx.input(|i| i.pointer.primary_pressed()) {
            ctx.send_viewport_cmd(ViewportCommand::BeginResize(d));
        }
    }
}

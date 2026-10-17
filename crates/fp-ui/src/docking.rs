//! Eigenes Docking im Stil des Originals (Owl-Docks, 11 §4.1): rechts ein Dock mit gestapelten
//! Bedienfeldgruppen (Reiterleiste, Bedienfeldmenü ≡, Einklappen per Klick auf den aktiven Reiter),
//! schwebende Gruppen als eigene Fensterflächen, Reiter per Ziehen umgruppieren, andocken und lösen.
//! Symbolmodus (Iconic Mode) mit aufklappenden Bedienfeldern.

use crate::app_state::{Action, AppState};
use crate::icons::{self, Icon, IconStyle};
use crate::menu;
use crate::panels::{self, PanelId};
use crate::theme::{Theme, ToolGroup, with_alpha};
use crate::widgets;
use crate::workspace::{DockMode, PanelGroup};
use egui::{Align2, Color32, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

/// Wohin ein gezogener Reiter fällt.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DropTarget {
    /// In Dock-Gruppe `group` an Reiterposition `index`.
    DockTabs { group: usize, index: usize },
    /// Neue Dock-Gruppe an Position `at`.
    DockGap { at: usize },
    /// In schwebende Gruppe.
    FloatTabs { group: usize, index: usize },
}

/// Laufzeitzustand des Dockings (nicht gespeichert).
#[derive(Clone, Debug, Default)]
pub struct DockUi {
    /// Gezogener Reiter.
    pub drag: Option<PanelId>,
    /// Ablagezonen dieses Frames.
    pub zones: Vec<(Rect, DropTarget)>,
    /// Ablagezonen des letzten Frames (für das Loslassen).
    pub last_zones: Vec<(Rect, DropTarget)>,
    /// Gezeichnete Bedienfelder dieses Frames: vorderer Reiter einer Gruppe und das Rechteck der
    /// Gruppe (Reiterleiste + Inhalt, eingeklappt nur die Leiste). Für die Brücke (`panel_rects`, TW-59).
    pub panel_rects: Vec<(PanelId, Rect)>,
    /// `panel_rects` des letzten Frames (die Brücke läuft vor dem Zeichnen).
    pub last_panel_rects: Vec<(PanelId, Rect)>,
    /// Im Symbolmodus aufgeklapptes Bedienfeld.
    pub iconic_open: Option<PanelId>,
    /// Gemessene Inhaltsbreite von Bedienfeldern, deren Inhalt breiter war als ihr Platz
    /// (gilt für den Sprachstand `measured_generation`; beim Sprachwechsel neu gemessen).
    pub measured: std::collections::BTreeMap<PanelId, f32>,
    pub measured_generation: u64,
    /// Zuletzt gezielt geöffnetes Bedienfeld (Reiterklick, Menü *Fenster*): reicht die Höhe des
    /// Docks nicht für alle Gruppen, bleibt seine Gruppe offen und andere klappen zu (CT-3).
    pub prefer: Option<PanelId>,
    /// Dock-Gruppen, die im letzten Bild nur wegen der Höhe eingeklappt angezeigt wurden
    /// (gespeichert offen); ein Reiterklick klappt sie auf statt zu.
    pub forced: Vec<usize>,
}

/// Innenabstand des Bedienfeldinhalts links und rechts (Gruppe → Inhalt).
pub const PANEL_INSET: f32 = 4.0;

/// Breite, die der Inhalt eines Bedienfelds mindestens braucht (ohne Innenabstand): die feste
/// Anordnung nach Vorlage (DLU, schon auf Deutsch breiter als die alte Dock-Mindestbreite von
/// 172 px) bzw. die gemessene Breite, wenn der Inhalt in der aktuellen Sprache mehr brauchte.
pub fn panel_min_width(st: &AppState, p: PanelId) -> f32 {
    let fixed: f32 = match p {
        PanelId::Align | PanelId::ColorPalette => 196.0,
        PanelId::Info | PanelId::Url => 216.0,
        PanelId::FindReplace => 218.0,
        PanelId::Optimize => 228.0,
        PanelId::Layers => 232.0,
        PanelId::ColorMixer => 246.0,
        _ => 172.0,
    };
    let measured = if st.dock.measured_generation == fp_i18n::generation() {
        st.dock.measured.get(&p).copied().unwrap_or(0.0)
    } else {
        0.0
    };
    fixed.max(measured)
}

/// Mindestbreite des Docks wie im Original: so breit wie das breiteste angedockte (offene)
/// Bedienfeld, mindestens `metrics.dock_min_width`.
pub fn dock_min_width(st: &AppState, theme: &Theme) -> f32 {
    let ws = &st.settings.workspace;
    ws.dock
        .iter()
        .flat_map(|g| ws.visible_panels(g))
        .map(|p| panel_min_width(st, p) + 2.0 * PANEL_INSET)
        .fold(theme.metrics.dock_min_width, f32::max)
}

/// Mindestbreite einer schwebenden Gruppe (breitestes Bedienfeld, mindestens [`FLOAT_MIN`]).
pub fn floating_min_width(st: &AppState, g: &PanelGroup) -> f32 {
    st.settings
        .workspace
        .visible_panels(g)
        .into_iter()
        .map(|p| panel_min_width(st, p) + 2.0 * PANEL_INSET + 2.0)
        .fold(FLOAT_MIN[0], f32::max)
}

/// Merkt sich, dass der Inhalt von `p` `w` Punkte breit war (nur wenn er nicht passte).
fn record_width(st: &mut AppState, p: PanelId, w: f32) {
    let g = fp_i18n::generation();
    if st.dock.measured_generation != g {
        st.dock.measured.clear();
        st.dock.measured_generation = g;
    }
    // begrenzt, damit ein Inhalt, der sich an der verfügbaren Breite orientiert, das Dock nicht aufschaukelt
    let w = w.min(600.0);
    let e = st.dock.measured.entry(p).or_insert(0.0);
    if w > *e {
        *e = w;
    }
}

/// Höhe einer Reiterleiste.
pub fn tab_strip_height(theme: &Theme) -> f32 {
    theme.metrics.panel_tab_height + 2.0
}

/// Ergebnis der Reiterleiste.
#[derive(Default)]
pub struct TabStripOut {
    pub clicked: Option<usize>,
    /// Doppelklick auf einen Reiter.
    pub double_clicked: Option<usize>,
    pub drag_started: Option<usize>,
    pub menu_clicked: Option<egui::Response>,
    pub tab_rects: Vec<Rect>,
    /// Antworten der Reiter (für das Kontextmenü per Rechtsklick).
    pub tab_resps: Vec<egui::Response>,
    pub background: Option<egui::Response>,
}

/// Breiten der Reiter in `avail` (CT-C-21): passen die natürlichen Breiten `nat`, bleiben sie; sonst
/// behält der aktive Reiter seine Breite (höchstens die Hälfte), die übrigen teilen sich den Rest
/// anteilig, jeder mindestens 28 px.
pub fn fit_tab_widths(nat: &[f32], active: Option<usize>, avail: f32) -> Vec<f32> {
    let total: f32 = nat.iter().sum();
    if total <= avail || nat.is_empty() {
        return nat.to_vec();
    }
    let act = active.filter(|a| *a < nat.len() && nat.len() > 1);
    let act_w = act.map(|a| nat[a].min(avail * 0.5)).unwrap_or(0.0);
    let others: f32 = total - act.map(|a| nat[a]).unwrap_or(0.0);
    let rest = (avail - act_w).max(0.0);
    nat.iter()
        .enumerate()
        .map(|(i, w)| {
            if Some(i) == act {
                act_w
            } else {
                (w * rest / others.max(1.0)).floor().max(28.0)
            }
        })
        .collect()
}

/// Zeichnet eine Reiterleiste wie in CS6 (Verlauf, aktiver Reiter in Inhaltsfarbe, Trenner, ≡).
/// `accent` je Reiter für die farbigen Kennungen im OLED-Design.
pub fn tab_strip(
    ui: &mut Ui,
    rect: Rect,
    id: Id,
    tabs: &[(&str, ToolGroup)],
    active: Option<usize>,
    show_menu: bool,
    collapsed: bool,
) -> TabStripOut {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let mut out = TabStripOut::default();
    let bg = ui.interact(rect, id.with("bg"), Sense::click_and_drag());
    widgets::header_bar(ui.painter(), rect, &theme);
    let font = theme.tab_font();
    let mut x = rect.left() + 1.0;
    let menu_w = if show_menu { 18.0 } else { 0.0 };
    let max_right = rect.right() - menu_w - 2.0;
    // Reichen die Reiter nicht, werden alle anteilig gekürzt (Text mit „…“), der aktive behält
    // möglichst seine Breite – vorher bekam nur der letzte den Rest und war bloß „…“ (CT-C-21)
    let nat: Vec<f32> = tabs
        .iter()
        .map(|(t, _)| widgets::text_width(ui, t, &font) + 2.0 * theme.metrics.panel_tab_pad)
        .collect();
    let widths = fit_tab_widths(&nat, active, (max_right - x).max(0.0));
    for (i, (title, group)) in tabs.iter().enumerate() {
        let w = widths[i].min((max_right - x).max(24.0));
        let tr = Rect::from_min_max(
            pos2(x, rect.top() + 1.0),
            pos2((x + w).min(max_right), rect.bottom()),
        );
        if tr.width() < 8.0 {
            break;
        }
        let resp = ui.interact(tr, id.with(("tab", i)), Sense::click_and_drag());
        // voller Name für Barrierefreiheit und `ui_find`, auch wenn der Reiter gekürzt ist
        resp.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::SelectableLabel,
                true,
                active == Some(i),
                *title,
            )
        });
        let is_active = active == Some(i) && !collapsed;
        let accent = theme.accent(*group);
        if active == Some(i) {
            ui.painter().rect_filled(tr, 0.0, c.tab_active_bg);
            ui.painter().vline(
                tr.left() + 0.5,
                tr.y_range(),
                Stroke::new(1.0, c.tab_separator),
            );
            ui.painter().vline(
                tr.right() - 0.5,
                tr.y_range(),
                Stroke::new(1.0, c.tab_separator),
            );
            if theme.glow {
                ui.painter().rect_filled(
                    Rect::from_min_size(tr.min, vec2(tr.width(), 2.0)),
                    0.0,
                    accent,
                );
                ui.painter().rect_filled(
                    Rect::from_min_size(tr.min + vec2(0.0, 2.0), vec2(tr.width(), 4.0)),
                    0.0,
                    with_alpha(accent, 40),
                );
            }
            if is_active {
                // aktiver Reiter verschmilzt mit dem Inhalt
                ui.painter().hline(
                    tr.x_range().shrink(1.0),
                    tr.bottom() - 0.5,
                    Stroke::new(1.0, c.tab_active_bg),
                );
            }
        } else {
            if resp.hovered() {
                ui.painter()
                    .rect_filled(tr.shrink2(vec2(0.0, 1.0)), 0.0, c.tab_hover_bg);
            }
            ui.painter().vline(
                tr.right() - 0.5,
                tr.y_range().shrink(1.0),
                Stroke::new(1.0, c.tab_separator),
            );
            if theme.glow {
                ui.painter().hline(
                    tr.x_range().shrink(4.0),
                    tr.bottom() - 2.5,
                    Stroke::new(1.0, with_alpha(accent, 150)),
                );
            }
        }
        let tc = if active == Some(i) {
            c.tab_text
        } else {
            c.tab_text_inactive
        };
        widgets::text_clipped(
            ui.painter(),
            pos2(tr.left() + theme.metrics.panel_tab_pad, tr.center().y),
            Align2::LEFT_CENTER,
            title,
            font.clone(),
            tc,
            tr.width() - 2.0 * theme.metrics.panel_tab_pad + 4.0,
        );
        if resp.clicked() {
            out.clicked = Some(i);
        }
        if resp.double_clicked() {
            out.double_clicked = Some(i);
        }
        if resp.drag_started() {
            out.drag_started = Some(i);
        }
        out.tab_rects.push(tr);
        out.tab_resps.push(resp);
        x += w;
    }
    if show_menu {
        let mr =
            Rect::from_center_size(pos2(rect.right() - 10.0, rect.center().y), vec2(16.0, 12.0));
        let resp = ui.interact(mr, id.with("menu"), Sense::click());
        let st = IconStyle::mono(
            if resp.hovered() { c.focus } else { c.tab_text },
            c.frame_bg,
        );
        icons::paint(ui.painter(), mr, Icon::PanelMenu, &st);
        crate::ui_kit::icon_button_info(&resp, true, &fp_i18n::t("ui-dock-panel-menu"), None);
        out.menu_clicked = Some(resp.on_hover_text(fp_i18n::t("ui-dock-panel-menu")));
    }
    out.background = Some(bg);
    out
}

fn panel_icon(p: PanelId) -> Icon {
    match p {
        PanelId::Optimize => Icon::Export,
        PanelId::History => Icon::Undo,
        PanelId::Align => Icon::AlignLeft,
        PanelId::Pages => Icon::PageSmall,
        PanelId::States => Icon::DistributeStates,
        PanelId::Layers => Icon::Folder,
        PanelId::Styles => Icon::Star,
        PanelId::ColorPalette | PanelId::Swatches => Icon::Swatches,
        PanelId::SpecialCharacters => Icon::Text,
        PanelId::ImageEditing => Icon::Brush,
        PanelId::Path => Icon::Pen,
        PanelId::AutoShapes | PanelId::AutoShapeProperties => Icon::AutoShape,
        PanelId::DocumentLibrary | PanelId::CommonLibrary | PanelId::SymbolProperties => {
            Icon::EditSymbol
        }
        PanelId::Info => Icon::Info,
        PanelId::ColorMixer => Icon::DefaultColors,
        PanelId::Url => Icon::HotspotRect,
        PanelId::Behaviors => Icon::Slice,
        PanelId::FindReplace => Icon::Zoom,
        PanelId::CssProperties => Icon::Document,
    }
}

/// Zeichnet eine Gruppe (Reiterleiste + Inhalt) in `rect`. `where_` bestimmt die Ablagezone.
fn group_ui(
    ui: &mut Ui,
    st: &mut AppState,
    rect: Rect,
    group: &PanelGroup,
    gid: Id,
    floating: Option<usize>,
    dock_index: Option<usize>,
) -> Option<egui::Response> {
    let theme = Theme::current(ui.ctx());
    let vis = st.settings.workspace.visible_panels(group);
    if vis.is_empty() {
        return None;
    }
    let active_panel = st.settings.workspace.active_panel(group);
    let active_idx = active_panel.and_then(|p| vis.iter().position(|q| *q == p));
    let strip = Rect::from_min_size(rect.min, vec2(rect.width(), tab_strip_height(&theme)));
    let tabs: Vec<(&str, ToolGroup)> = vis.iter().map(|p| (p.title(), p.group())).collect();
    let out = tab_strip(ui, strip, gid, &tabs, active_idx, true, group.collapsed);
    if let Some(p) = active_panel {
        st.dock
            .panel_rects
            .push((p, if group.collapsed { strip } else { rect }));
    }
    // Ablagezone: Reiterleiste
    let target_index = |rects: &[Rect], x: f32| {
        rects
            .iter()
            .position(|r| x < r.center().x)
            .unwrap_or(rects.len())
    };
    if st.dock.drag.is_some()
        && let Some(p) = ui.ctx().pointer_hover_pos()
        && strip.contains(p)
    {
        let idx = target_index(&out.tab_rects, p.x);
        let x = out
            .tab_rects
            .get(idx)
            .map(|r| r.left())
            .or_else(|| out.tab_rects.last().map(|r| r.right()))
            .unwrap_or(strip.left() + 2.0);
        ui.painter()
            .vline(x, strip.y_range(), Stroke::new(2.0, theme.colors.focus));
    }
    let tab_rects = out.tab_rects.clone();
    let real_index = |vis_idx: usize| -> usize {
        // sichtbaren Index in Gruppenindex umrechnen
        vis.get(vis_idx)
            .and_then(|p| group.panels.iter().position(|q| q == p))
            .unwrap_or(group.panels.len())
    };
    let zone_target = match (floating, dock_index) {
        (Some(f), _) => DropTarget::FloatTabs {
            group: f,
            index: group.panels.len(),
        },
        (None, Some(g)) => DropTarget::DockTabs {
            group: g,
            index: group.panels.len(),
        },
        _ => DropTarget::DockGap { at: 0 },
    };
    // Zonen je Reiterhälfte für die genaue Position
    let mut x0 = strip.left();
    for (i, r) in tab_rects.iter().enumerate() {
        let zr = Rect::from_min_max(pos2(x0, strip.top()), pos2(r.center().x, strip.bottom()));
        let idx = real_index(i);
        let t = match zone_target {
            DropTarget::FloatTabs { group: f, .. } => DropTarget::FloatTabs {
                group: f,
                index: idx,
            },
            DropTarget::DockTabs { group: g, .. } => DropTarget::DockTabs {
                group: g,
                index: idx,
            },
            o => o,
        };
        st.dock.zones.push((zr, t));
        x0 = r.center().x;
    }
    st.dock.zones.push((
        Rect::from_min_max(pos2(x0, strip.top()), strip.max),
        zone_target,
    ));

    if let Some(i) = out.clicked {
        let gi = real_index(i);
        st.push(Action::Run(
            "ui.dock.tabClicked".into(),
            serde_json::json!({ "floating": floating, "group": dock_index, "index": gi }),
        ));
    }
    if let Some(i) = out.drag_started
        && let Some(p) = vis.get(i)
    {
        st.dock.drag = Some(*p);
    }
    if let (Some(resp), Some(p)) = (&out.menu_clicked, active_panel) {
        let nodes = menu::build_panel_menu(st, p);
        let mut acts = Vec::new();
        menu::popup_menu(ui, resp, &nodes, &mut acts);
        for a in acts {
            st.push(a);
        }
    }
    // Kontextmenü des Reiters: Schließen, Registerkartengruppe schließen (wie im Original, MN-3)
    let mut acts = Vec::new();
    for (i, r) in out.tab_resps.iter().enumerate() {
        if let Some(p) = vis.get(i) {
            r.context_menu(|ui| menu::show_nodes(ui, &menu::build_tab_menu(st, *p), &mut acts));
        }
    }
    for a in acts {
        st.push(a);
    }
    if !group.collapsed
        && let Some(p) = active_panel
    {
        let content = Rect::from_min_max(pos2(rect.left(), strip.bottom()), rect.max);
        if content.height() > 4.0 {
            ui.painter()
                .rect_filled(content, 0.0, theme.colors.frame_bg);
            let inner = content.shrink2(vec2(PANEL_INSET, 3.0));
            let need = panel_min_width(st, p);
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt(("panel", p))
                    .max_rect(inner)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            child.set_clip_rect(inner.intersect(ui.clip_rect()));
            if inner.width() + 0.5 >= need {
                panels::show(&mut child, st, p);
                // Inhalt breiter als sein Platz (z. B. längere Texte nach Sprachwechsel): merken
                let used = child.min_rect().width();
                if used > inner.width() + 1.0 {
                    record_width(st, p, used);
                }
            } else {
                // zu wenig Platz (Fenster schmaler als das Bedienfeld): waagerechter Bildlauf
                egui::ScrollArea::horizontal()
                    .id_salt(("panel-hscroll", p))
                    .auto_shrink([false, false])
                    .show(&mut child, |ui| {
                        let r =
                            Rect::from_min_size(ui.cursor().min, vec2(need, ui.available_height()));
                        let mut c = ui.new_child(
                            egui::UiBuilder::new()
                                .id_salt(("panel", p))
                                .max_rect(r)
                                .layout(egui::Layout::top_down(egui::Align::Min)),
                        );
                        panels::show(&mut c, st, p);
                        ui.allocate_rect(r, Sense::hover());
                    });
            }
        }
    }
    out.background
}

/// Rechtes Dock (im rechten Seitenbereich).
pub fn show_dock(ui: &mut Ui, st: &mut AppState) {
    let theme = Theme::current(ui.ctx());
    let full = ui.max_rect();
    let strip = Rect::from_min_size(
        full.min,
        vec2(full.width(), theme.metrics.dock_strip_height),
    );
    widgets::dock_strip(ui.painter(), strip, &theme);
    let iconic = st.settings.workspace.dock_mode != DockMode::Expanded;
    let br = Rect::from_center_size(
        pos2(strip.right() - 9.0, strip.center().y),
        vec2(12.0, 10.0),
    );
    let bresp = ui.interact(br, Id::new("dock-collapse"), Sense::click());
    let st_icon = IconStyle::mono(theme.colors.dock_strip_arrow, theme.colors.frame_bg);
    icons::paint(
        ui.painter(),
        br,
        if iconic {
            Icon::CollapseLeft
        } else {
            Icon::CollapseRight
        },
        &st_icon,
    );
    crate::ui_kit::icon_button_info(
        &bresp,
        true,
        &fp_i18n::t(if iconic {
            "ui-dock-expand"
        } else {
            "ui-dock-collapse-icons"
        }),
        None,
    );
    if bresp
        .on_hover_text(fp_i18n::t(if iconic {
            "ui-dock-expand"
        } else {
            "ui-dock-collapse-icons"
        }))
        .clicked()
    {
        st.push(Action::Run(
            "ui.dock.toggleIconic".into(),
            serde_json::json!({}),
        ));
    }
    let area = Rect::from_min_max(pos2(full.left(), strip.bottom()), full.max);
    ui.painter().rect_filled(area, 0.0, theme.colors.frame_bg);
    if iconic {
        iconic_dock(ui, st, area);
        return;
    }
    let mut groups: Vec<(usize, PanelGroup)> = st
        .settings
        .workspace
        .dock
        .iter()
        .enumerate()
        .filter(|(_, g)| !st.settings.workspace.visible_panels(g).is_empty())
        .map(|(i, g)| (i, g.clone()))
        .collect();
    let split = theme.metrics.splitter;
    let header = tab_strip_height(&theme);
    let n = groups.len();
    let fixed: f32 = n as f32 * header + n as f32 * split;
    let rem = (area.height() - fixed).max(0.0);
    // Zu wenig Höhe: untere Gruppen nur für die Anzeige einklappen, damit jede offene Gruppe eine
    // Mindesthöhe behält (sonst überlappen Listen und Knopfleisten); gespeichert bleibt der Zustand.
    const MIN_GROUP_CONTENT: f32 = 120.0;
    // Die Gruppe des zuletzt gezielt geöffneten Bedienfelds bleibt offen (sonst ließe sich die
    // unterste Gruppe bei kleinem Fenster nie aufklappen, CT-3)
    let prefer = st.dock.prefer.and_then(|p| {
        groups
            .iter()
            .find(|(_, g)| g.panels.contains(&p))
            .map(|(gi, _)| *gi)
    });
    let mut forced = Vec::new();
    loop {
        let open = groups.iter().filter(|(_, g)| !g.collapsed).count();
        if open <= 1 || rem >= open as f32 * MIN_GROUP_CONTENT {
            break;
        }
        match groups
            .iter_mut()
            .rev()
            .find(|(gi, g)| !g.collapsed && Some(*gi) != prefer)
        {
            Some((gi, g)) => {
                g.collapsed = true;
                forced.push(*gi);
            }
            None => break,
        }
    }
    st.dock.forced = forced;
    let wsum: f32 = groups
        .iter()
        .filter(|(_, g)| !g.collapsed)
        .map(|(_, g)| g.weight)
        .sum::<f32>()
        .max(1e-3);
    let mut y = area.top();
    let mut prev_expanded: Option<(usize, f32)> = None;
    for (k, (gi, g)) in groups.iter().enumerate() {
        // Trenner oberhalb jeder Gruppe (wie CS6: dunkel/mittel/dunkel)
        let sp = Rect::from_min_size(pos2(area.left(), y), vec2(area.width(), split));
        widgets::hsplitter(ui.painter(), y, area.x_range(), &theme);
        st.dock
            .zones
            .push((sp.expand2(vec2(0.0, 3.0)), DropTarget::DockGap { at: *gi }));
        if st.dock.drag.is_some()
            && let Some(p) = ui.ctx().pointer_hover_pos()
            && sp.expand2(vec2(0.0, 3.0)).contains(p)
        {
            ui.painter().hline(
                sp.x_range(),
                sp.center().y,
                Stroke::new(3.0, theme.colors.focus),
            );
        }
        // Höhe verändern zwischen zwei aufgeklappten Gruppen
        if !g.collapsed
            && let Some((pgi, ph)) = prev_expanded
        {
            let resp = ui.interact(sp, Id::new(("dock-split", k)), Sense::drag());
            if resp.hovered() || resp.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
            }
            if resp.dragged() {
                let dy = resp.drag_delta().y;
                let h = (rem * g.weight / wsum).max(1.0);
                let new_prev = (ph + dy).max(40.0);
                let new_this = (h - dy).max(40.0);
                let total_w = st
                    .settings
                    .workspace
                    .dock
                    .get(pgi)
                    .map(|x| x.weight)
                    .unwrap_or(1.0)
                    + g.weight;
                let wp = total_w * new_prev / (new_prev + new_this);
                if let Some(x) = st.settings.workspace.dock.get_mut(pgi) {
                    x.weight = wp.max(0.1);
                }
                if let Some(x) = st.settings.workspace.dock.get_mut(*gi) {
                    x.weight = (total_w - wp).max(0.1);
                }
            }
        }
        y += split;
        let h = if g.collapsed {
            header
        } else {
            header + rem * g.weight / wsum
        };
        let r = Rect::from_min_size(pos2(area.left(), y), vec2(area.width(), h));
        group_ui(ui, st, r, g, Id::new(("dock-group", *gi)), None, Some(*gi));
        if !g.collapsed {
            prev_expanded = Some((*gi, h - header));
        }
        y += h;
    }
    // freier Raum unter den Gruppen: neue Gruppe am Ende
    let rest = Rect::from_min_max(pos2(area.left(), y), area.max);
    if rest.height() > 2.0 {
        widgets::hsplitter(ui.painter(), y, area.x_range(), &theme);
        st.dock.zones.push((
            rest,
            DropTarget::DockGap {
                at: st.settings.workspace.dock.len(),
            },
        ));
    }
}

fn iconic_dock(ui: &mut Ui, st: &mut AppState, area: Rect) {
    let theme = Theme::current(ui.ctx());
    let with_names = st.settings.workspace.dock_mode == DockMode::IconicWithNames;
    let mut y = area.top() + 4.0;
    let groups: Vec<PanelGroup> = st.settings.workspace.dock.clone();
    for (gi, g) in groups.iter().enumerate() {
        let vis = st.settings.workspace.visible_panels(g);
        if vis.is_empty() {
            continue;
        }
        if gi > 0 {
            widgets::hsplitter(ui.painter(), y, area.x_range(), &theme);
            y += 6.0;
        }
        for p in vis {
            let r = Rect::from_min_size(pos2(area.left() + 3.0, y), vec2(area.width() - 6.0, 26.0));
            let resp = ui.interact(r, Id::new(("iconic", p)), Sense::click());
            let open = st.dock.iconic_open == Some(p);
            widgets::button_frame(
                ui.painter(),
                r,
                &resp,
                widgets::ButtonLook {
                    selected: open,
                    enabled: true,
                    group: Some(p.group()),
                },
                &theme,
            );
            let ir = Rect::from_center_size(pos2(r.left() + 13.0, r.center().y), vec2(16.0, 16.0));
            icons::paint(
                ui.painter(),
                ir,
                panel_icon(p),
                &IconStyle::new(&theme, Some(p.group()), true),
            );
            if with_names {
                widgets::text_clipped(
                    ui.painter(),
                    pos2(r.left() + 28.0, r.center().y),
                    Align2::LEFT_CENTER,
                    p.title(),
                    theme.ui_font(),
                    theme.colors.text,
                    r.width() - 30.0,
                );
            }
            crate::ui_kit::icon_button_info(&resp, true, p.title(), Some(open));
            if resp.on_hover_text(p.title()).clicked() {
                st.dock.iconic_open = if open { None } else { Some(p) };
            }
            if open {
                // aufgeklapptes Bedienfeld links neben dem Dock
                let size = vec2(
                    250.0f32.max(panel_min_width(st, p) + 2.0 * PANEL_INSET),
                    p.preferred_height() + tab_strip_height(&theme),
                );
                let pos = pos2(
                    area.left() - size.x - 2.0,
                    r.top().min(area.bottom() - size.y).max(area.top()),
                );
                let mut gcopy = g.clone();
                gcopy.collapsed = false;
                if let Some(ix) = gcopy.panels.iter().position(|q| *q == p) {
                    gcopy.active = ix;
                }
                egui::Area::new(Id::new("iconic-flyout"))
                    .fixed_pos(pos)
                    .order(egui::Order::Foreground)
                    .show(ui.ctx(), |ui| {
                        let rect = Rect::from_min_size(pos, size);
                        ui.painter().rect_filled(rect, 0.0, theme.colors.frame_bg);
                        let (_r, _resp) = ui.allocate_exact_size(size, Sense::hover());
                        group_ui(
                            ui,
                            st,
                            rect,
                            &gcopy,
                            Id::new(("iconic-group", gi)),
                            None,
                            Some(gi),
                        );
                        ui.painter().rect_stroke(
                            rect,
                            0.0,
                            Stroke::new(
                                1.0,
                                if theme.glow {
                                    theme.accent(p.group())
                                } else {
                                    theme.colors.line_dark
                                },
                            ),
                            StrokeKind::Outside,
                        );
                    });
            }
            y += 28.0;
        }
    }
}

/// Schwebende Gruppen (über allem außer Menüs).
pub fn show_floating(ctx: &egui::Context, st: &mut AppState) {
    let theme = Theme::current(ctx);
    if st.settings.workspace.panels_hidden {
        return;
    }
    let groups = st.settings.workspace.floating.clone();
    for (fi, f) in groups.iter().enumerate() {
        if st.settings.workspace.visible_panels(&f.group).is_empty() {
            continue;
        }
        let id = Id::new(("float", fi, f.group.panels.first().copied()));
        // Mindestgröße und Lage im Fenster (Kopfleiste bleibt greifbar), wie das Original
        let screen = ctx.content_rect();
        let min_w = floating_min_width(st, &f.group);
        // Titelleiste (Leiste wie über dem Dock, mit Einklappen und Schließen; CT-C-19)
        let title_h = theme.metrics.dock_strip_height;
        let size = vec2(
            f.size[0].clamp(min_w, screen.width().max(min_w)),
            if f.group.collapsed {
                title_h + tab_strip_height(&theme) + 2.0
            } else {
                f.size[1].clamp(FLOAT_MIN[1], (screen.height() - 40.0).max(FLOAT_MIN[1]))
            },
        );
        let pos = pos2(
            f.pos[0].clamp(
                screen.left() - size.x + 80.0,
                (screen.right() - 80.0).max(screen.left()),
            ),
            f.pos[1].clamp(
                screen.top() + 31.0,
                (screen.bottom() - 30.0).max(screen.top() + 31.0),
            ),
        );
        egui::Area::new(id)
            .fixed_pos(pos)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                let rect = Rect::from_min_size(pos, size);
                let (_r, _resp) = ui.allocate_exact_size(size, Sense::hover());
                let shadow = egui::Shadow {
                    offset: [3, 3],
                    blur: 8,
                    spread: 0,
                    color: Color32::from_black_alpha(if theme.glow { 0 } else { 70 }),
                };
                ui.painter().add(shadow.as_shape(rect, 0.0));
                ui.painter().rect_filled(rect, 0.0, theme.colors.frame_bg);
                let inner = rect.shrink(1.0);
                let title = Rect::from_min_size(inner.min, vec2(inner.width(), title_h));
                let title_resp = float_title(ui, st, title, id, fi, &f.group, &theme);
                let inner = Rect::from_min_max(pos2(inner.left(), title.bottom()), inner.max);
                let bg = group_ui(ui, st, inner, &f.group, id.with("g"), Some(fi), None);
                let bg = match (bg, title_resp) {
                    (Some(b), Some(t)) => Some(b.union(t)),
                    (b, t) => b.or(t),
                };
                // Verschieben über den freien Teil der Reiterleiste
                if let Some(bg) = bg {
                    if bg.dragged() {
                        let d = bg.drag_delta();
                        if let Some(x) = st.settings.workspace.floating.get_mut(fi) {
                            x.pos[0] += d.x;
                            x.pos[1] += d.y;
                        }
                    }
                    if bg.drag_stopped()
                        && let Some(p) = ctx.pointer_interact_pos()
                    {
                        // über dem Dock losgelassen → andocken
                        if let Some((_, DropTarget::DockGap { at })) =
                            st.dock.last_zones.iter().find(|(r, t)| {
                                r.contains(p) && matches!(t, DropTarget::DockGap { .. })
                            })
                        {
                            st.push(Action::Run(
                                "ui.dock.dockFloating".into(),
                                serde_json::json!({ "floating": fi, "at": at }),
                            ));
                        }
                    }
                    if bg.double_clicked()
                        && let Some(x) = st.settings.workspace.floating.get_mut(fi)
                    {
                        x.group.collapsed = !x.group.collapsed;
                    }
                }
                ui.painter().rect_stroke(
                    rect,
                    0.0,
                    Stroke::new(
                        1.0,
                        if theme.glow {
                            theme.colors.focus
                        } else {
                            theme.colors.line_dark
                        },
                    ),
                    StrokeKind::Inside,
                );
                // Größe ändern (Ecke unten rechts)
                if !f.group.collapsed {
                    let gr = Rect::from_min_size(
                        rect.right_bottom() - vec2(12.0, 12.0),
                        vec2(12.0, 12.0),
                    );
                    let resp = ui.interact(gr, id.with("resize"), Sense::drag());
                    for k in 0..3 {
                        let o = 3.0 + k as f32 * 3.0;
                        ui.painter().line_segment(
                            [
                                gr.right_bottom() - vec2(o, 1.0),
                                gr.right_bottom() - vec2(1.0, o),
                            ],
                            Stroke::new(1.0, theme.colors.grip_dark),
                        );
                    }
                    if resp.hovered() || resp.dragged() {
                        ctx.set_cursor_icon(egui::CursorIcon::ResizeNwSe);
                    }
                    if resp.dragged() {
                        let d = resp.drag_delta();
                        if let Some(x) = st.settings.workspace.floating.get_mut(fi) {
                            x.size[0] = (x.size[0] + d.x).max(min_w);
                            x.size[1] = (x.size[1] + d.y).max(FLOAT_MIN[1]);
                        }
                    }
                }
            });
    }
}

/// Titelleiste einer schwebenden Gruppe (CT-C-19): Pfeil links klappt ein/aus, „×“ rechts schließt
/// die Gruppe (wie „Registerkartengruppe schließen“); der Rest verschiebt das Fenster.
fn float_title(
    ui: &mut Ui,
    st: &mut AppState,
    r: Rect,
    id: Id,
    fi: usize,
    g: &PanelGroup,
    theme: &Theme,
) -> Option<egui::Response> {
    let c = &theme.colors;
    widgets::dock_strip(ui.painter(), r, theme);
    let arrow = Rect::from_min_size(r.min, vec2(r.height() + 6.0, r.height()));
    let close = Rect::from_min_max(pos2(r.right() - r.height() - 6.0, r.top()), r.max);
    let a = ui
        .interact(arrow, id.with("collapse"), Sense::click())
        .on_hover_text(fp_i18n::t(if g.collapsed {
            "ui-panel-expand"
        } else {
            "ui-panel-collapse"
        }));
    let x = ui
        .interact(close, id.with("close"), Sense::click())
        .on_hover_text(fp_i18n::t("pmenu-closeGroup"));
    let ac = if a.hovered() {
        c.text
    } else {
        c.dock_strip_arrow
    };
    let p = arrow.center();
    let tri = if g.collapsed {
        vec![
            p + vec2(-2.0, -4.0),
            p + vec2(2.0, 0.0),
            p + vec2(-2.0, 4.0),
        ]
    } else {
        vec![
            p + vec2(-4.0, -2.0),
            p + vec2(4.0, -2.0),
            p + vec2(0.0, 2.0),
        ]
    };
    ui.painter()
        .add(egui::Shape::convex_polygon(tri, ac, Stroke::NONE));
    let xc = if x.hovered() {
        c.text
    } else {
        c.dock_strip_arrow
    };
    let q = close.center();
    for d in [vec2(3.0, 3.0), vec2(3.0, -3.0)] {
        ui.painter()
            .line_segment([q - d, q + d], Stroke::new(1.3, xc));
    }
    if a.clicked()
        && let Some(f) = st.settings.workspace.floating.get_mut(fi)
    {
        f.group.collapsed = !f.group.collapsed;
    }
    if x.clicked()
        && let Some(p) = g.panels.first().copied()
    {
        st.settings.workspace.close_group_of(p);
        st.sync_panels_to_engine();
    }
    let rest = Rect::from_min_max(pos2(arrow.right(), r.top()), pos2(close.left(), r.bottom()));
    Some(ui.interact(rest, id.with("title"), Sense::click_and_drag()))
}

/// Mindestgröße schwebender Bedienfeldgruppen (Breite, Höhe) – darunter überlappen Bedienfeldinhalte.
pub const FLOAT_MIN: [f32; 2] = [220.0, 160.0];

/// Reiter-Ziehen: Geisterbild zeichnen und beim Loslassen ablegen bzw. lösen.
pub fn finish_drag(ctx: &egui::Context, st: &mut AppState) {
    let theme = Theme::current(ctx);
    st.dock.last_zones = std::mem::take(&mut st.dock.zones);
    st.dock.last_panel_rects = std::mem::take(&mut st.dock.panel_rects);
    let Some(p) = st.dock.drag else { return };
    let pointer = ctx.pointer_interact_pos();
    if let Some(pos) = ctx.pointer_hover_pos() {
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            Id::new("dock-ghost"),
        ));
        let font = theme.tab_font();
        let w = ctx.fonts_mut(|f| {
            f.layout_no_wrap(p.title().to_string(), font.clone(), Color32::WHITE)
                .size()
                .x
        }) + 16.0;
        let r = Rect::from_min_size(pos + vec2(8.0, 6.0), vec2(w, 18.0));
        painter.rect_filled(r, 0.0, with_alpha(theme.colors.tab_active_bg, 220));
        painter.rect_stroke(
            r,
            0.0,
            Stroke::new(1.0, theme.colors.focus),
            StrokeKind::Inside,
        );
        painter.text(
            pos2(r.left() + 8.0, r.center().y),
            Align2::LEFT_CENTER,
            p.title(),
            font,
            theme.colors.tab_text,
        );
        ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
    }
    let released = ctx.input(|i| i.pointer.any_released() || !i.pointer.any_down());
    if released {
        st.dock.drag = None;
        let Some(pos) = pointer else { return };
        let target = st
            .dock
            .last_zones
            .iter()
            .find(|(r, _)| r.contains(pos))
            .map(|(_, t)| *t);
        let a = match target {
            Some(DropTarget::DockTabs { group, index }) => {
                serde_json::json!({ "panel": format!("{p:?}"), "kind": "dockTabs", "group": group, "index": index })
            }
            Some(DropTarget::DockGap { at }) => {
                serde_json::json!({ "panel": format!("{p:?}"), "kind": "dockGap", "at": at })
            }
            Some(DropTarget::FloatTabs { group, index }) => {
                serde_json::json!({ "panel": format!("{p:?}"), "kind": "floatTabs", "group": group, "index": index })
            }
            None => {
                serde_json::json!({ "panel": format!("{p:?}"), "kind": "float", "pos": [pos.x - 20.0, pos.y - 8.0] })
            }
        };
        st.push(Action::Run("ui.dock.drop".into(), a));
    }
}

/// Bedienfeld-Kennung aus dem Debug-Namen (für die Aktionsparameter).
pub fn panel_from_name(s: &str) -> Option<PanelId> {
    PanelId::ALL.iter().copied().find(|p| format!("{p:?}") == s)
}

/// Ziel-Punkt eines Pos2 als Array.
pub fn pos_arr(p: Pos2) -> [f32; 2] {
    [p.x, p.y]
}

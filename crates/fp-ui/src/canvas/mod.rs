//! Dokumentbereich (11 §4.4–§4.8): Dokument-Reiter, Ansichtsreiter Original/Vorschau/2-fach/4-fach
//! mit Seitenauswahl, Leinwand (Rendern über fp-render, Zoom, Bildlauf, Lineale, Raster,
//! Hilfslinien, Schachbrett, Auswahlrahmen und Werkzeug-Vorschau) und die Fußleiste
//! (Auswahlinfo, Status-Steuerung, Dokumentgröße, Vergrößerung). Mausereignisse gehen als
//! [`fp_engine::ToolEvent`] an die Sitzung.

pub mod anim;
pub mod context;
pub mod cursors;
pub mod ghost;
pub mod overlay;
pub mod paint_preview;
pub mod render;
pub mod stack;
#[cfg(test)]
mod tests;
pub mod textedit;
pub mod tools;
pub mod web;

use crate::app_state::{Action, AppState};
use crate::icons::{self, Icon, IconStyle};
use crate::theme::{Theme, ToolGroup, with_alpha};
use crate::widgets::{self, ButtonLook};
use egui::{Align2, Color32, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use fp_engine::tools::{Cursor, Modifiers, PointerKind, ToolInput};
use fp_engine::view::{PreviewMode, ZOOM_STEPS};
use fp_engine::{Overlay, ToolEvent};
use fp_geom::{IRect, Point};
use render::{RenderJob, RenderKey};
use serde_json::json;

/// Zeichnet den ganzen Dokumentbereich in die verfügbare Fläche.
pub fn show_document_area(ui: &mut Ui, st: &mut AppState) {
    let theme = Theme::current(ui.ctx());
    let full = ui.max_rect();
    let m = &theme.metrics;
    // TW-158: the full-screen modes hide the document tabs (and in full screen every bar)
    let chrome = crate::screen_mode::current(st);
    let tab_h = if chrome.doc_tabs {
        m.doc_tab_height
    } else {
        0.0
    };
    let tabs = Rect::from_min_size(full.min, vec2(full.width(), tab_h));
    if chrome.doc_tabs {
        doc_tabs(ui, st, tabs);
    }
    // geschlossene Dokumente: Leinwand-Cache und Kachel-Cache des Renderers freigeben
    let open: Vec<fp_engine::DocId> = st.session.documents().iter().map(|d| d.id).collect();
    let before = st.canvases.len();
    st.canvases.retain(|id, _| open.contains(id));
    if st.canvases.len() < before {
        st.renderer.clear_tiles();
    }
    if crate::batch::is_front(st) {
        // BATCH-2: Batch-Studio statt der Leinwand (Reiter „Batch“)
        crate::batch::show(
            ui,
            st,
            Rect::from_min_max(pos2(full.left(), tabs.bottom()), full.max),
        );
        return;
    }
    if st.session.active().is_none() {
        let rest = Rect::from_min_max(pos2(full.left(), tabs.bottom()), full.max);
        // Startbildschirm (DESIGN, `welcome.rs`): zuletzt geöffnet, Neu erstellen, Erweitern
        crate::welcome::show(ui, st, rest);
        return;
    }
    let view_bar_h = if chrome.view_bar {
        m.view_bar_height
    } else {
        0.0
    };
    let view_bar = Rect::from_min_size(
        pos2(full.left(), tabs.bottom()),
        vec2(full.width(), view_bar_h),
    );
    if chrome.view_bar {
        view_bar_ui(ui, st, view_bar);
    }
    let footer_h = if chrome.footer { m.footer_height } else { 0.0 };
    let footer = Rect::from_min_max(pos2(full.left(), full.bottom() - footer_h), full.max);
    // TW-36: Symbolbearbeitung im Fenster des Dokuments mit Pfadleiste (Zurück · Seite ▸ Symbol)
    let mut top = view_bar.bottom();
    if let Some(path) = symbol_path(st) {
        let bar = Rect::from_min_size(pos2(full.left(), top), vec2(full.width(), SYMBOL_BAR_H));
        symbol_bar_ui(ui, st, bar, &path);
        top = bar.bottom();
    }
    let canvas = Rect::from_min_max(pos2(full.left(), top), pos2(full.right(), footer.top()));
    let preview = st
        .session
        .active()
        .is_some_and(|d| d.view.preview_mode != PreviewMode::Original);
    if preview {
        // Vorschau/2-fach/4-fach: Exportbild (nicht bearbeitbar); die Originalkachel ist die Leinwand
        crate::preview::show(ui, st, canvas, &mut |ui, st, r| canvas_ui(ui, st, r));
    } else {
        canvas_ui(ui, st, canvas);
    }
    if chrome.footer {
        footer_ui(ui, st, footer);
    }
}

// ---------------------------------------------------------------------- Symbolbearbeitung

/// Höhe der Pfadleiste im Symbol-Bearbeitungsmodus.
pub const SYMBOL_BAR_H: f32 = 24.0;

/// Pfad der aktiven Symbol-Bearbeitungsansicht: Seitenname des Dokuments und Symbolname
/// (`None`, wenn das aktive Dokument keine Bearbeitungsansicht ist).
pub fn symbol_path(st: &AppState) -> Option<(String, String)> {
    let d = st.session.active()?;
    let link = d.symbol_edit.as_ref()?;
    let parent = st.session.document(link.parent)?;
    let page = parent
        .doc
        .pages
        .get(link.page)
        .map(|p| p.display_name().to_string())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| fp_i18n::tr!("ui-page-default-name", n = link.page + 1));
    let symbol = parent
        .doc
        .symbol_by_id(link.symbol)
        .map(|(_, s)| s.name.clone())
        .unwrap_or_default();
    Some((page, symbol))
}

/// Abgeblendete Umgebung beim Bearbeiten an Position (re 28 S2): Hintergrund der Seite und alle
/// übrigen Objekte an ihrer Lage mit 50 % Deckkraft (Alpha 128; Rot über Weiß erscheint als
/// 255,127,127), die bearbeitete Instanz selbst nicht. Die Ansicht liegt so, dass Symbolkoordinaten
/// `p` auf der Seite bei `p + Instanzversatz` erscheinen (`begin_edit`).
pub fn symbol_backdrop_image(
    st: &AppState,
) -> Option<(fp_raster::RgbaImage, fp_color::Argb, (f32, f32))> {
    let d = st.session.active()?;
    let link = d.symbol_edit.as_ref().filter(|l| l.in_place)?;
    let parent = st.session.document(link.parent)?;
    let o = link
        .transform
        .map(|m| m.apply(Point::new(0.0, 0.0)))
        .unwrap_or(Point::new(0.0, 0.0));
    let bg = parent
        .doc
        .pages
        .get(link.page)
        .map(|p| p.root.background)
        .unwrap_or(fp_color::Argb(0));
    let mut els = Vec::new();
    for (_, l) in fp_engine::docops::own_layers(&parent.doc, link.page)
        .into_iter()
        .rev()
    {
        if !l.is_visible() || l.kind == fp_model::LayerKind::Web {
            continue;
        }
        let ci = if l.shared_across_frames {
            0
        } else {
            parent.frame
        };
        if let Some(c) = l.cels.get(ci) {
            els.extend(
                c.elements
                    .iter()
                    .filter(|e| Some(e.id) != link.instance && e.common.visible)
                    .cloned(),
            );
        }
    }
    let img = render::render_elements(&st.renderer, &parent.doc, parent.frame, &els)
        .unwrap_or_else(|| fp_raster::RgbaImage::empty_at(0, 0));
    Some((img, bg, (o.x, o.y)))
}

fn paint_symbol_backdrop(
    ctx: &egui::Context,
    st: &AppState,
    painter: &egui::Painter,
    page_screen: Rect,
    area: Rect,
    view: &fp_engine::ViewState,
) {
    let Some(d) = st.session.active() else { return };
    let Some(link) = d.symbol_edit.as_ref().filter(|l| l.in_place) else {
        return;
    };
    let Some(parent) = st.session.document(link.parent) else {
        return;
    };
    let key = egui::Id::new(("rt3-symbol-backdrop", d.id.0, parent.generation));
    type Cached = Option<(
        egui::TextureHandle,
        fp_geom::IRect,
        fp_color::Argb,
        (f32, f32),
    )>;
    let cached: Cached = ctx.data(|m| m.get_temp(key)).flatten();
    let cached = match cached {
        Some(c) => Some(c),
        None => symbol_backdrop_image(st).map(|(img, bg, o)| {
            let tex = ctx.load_texture(
                "symbol-backdrop",
                render::to_color_image(&img),
                egui::TextureOptions::NEAREST,
            );
            let c = (tex, img.bounds(), bg, o);
            ctx.data_mut(|m| m.insert_temp::<Cached>(key, Some(c.clone())));
            c
        }),
    };
    let Some((tex, b, bg, o)) = cached else {
        return;
    };
    if bg.a() > 0 {
        painter.rect_filled(
            page_screen,
            0.0,
            Color32::from_rgba_unmultiplied(bg.r(), bg.g(), bg.b(), bg.a()),
        );
    }
    if b.is_empty() {
        return;
    }
    let (ox, oy) = (o.0 as f64, o.1 as f64);
    let a = to_screen(view, area.min, b.left as f64 - ox, b.top as f64 - oy);
    let z = to_screen(view, area.min, b.right as f64 - ox, b.bottom as f64 - oy);
    painter.image(
        tex.id(),
        Rect::from_min_max(a, z),
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
        Color32::from_white_alpha(128),
    );
}

/// Dokumente, deren Reiter gerade eine Symbol-Bearbeitungsansicht vertritt (Original: dasselbe
/// Fenster, Titel „<Dokument> (Symbol)“).
pub fn hidden_by_symbol_edit(st: &AppState) -> Vec<fp_engine::DocId> {
    st.session
        .documents()
        .iter()
        .filter_map(|d| d.symbol_edit.as_ref().map(|l| l.parent))
        .collect()
}

/// Pfadleiste: Knopf „Zurück“ (beendet die Bearbeitung) und „Seite ▸ Symbol“.
fn symbol_bar_ui(ui: &mut Ui, st: &mut AppState, rect: Rect, path: &(String, String)) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    ui.painter().rect_filled(rect, 0.0, c.frame_bg);
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0, c.line_mid),
    );
    let font = theme.ui_font();
    let back = fp_i18n::t("ui-symbol-edit-back");
    let bw = widgets::text_width(ui, &back, &font) + 26.0;
    let br = Rect::from_min_size(
        pos2(rect.left() + 4.0, rect.top() + 2.0),
        vec2(bw, rect.height() - 4.0),
    );
    let resp = ui.interact(br, Id::new("symbol-edit-back"), Sense::click());
    widgets::button_frame(
        ui.painter(),
        br,
        &resp,
        ButtonLook {
            selected: false,
            enabled: true,
            group: Some(ToolGroup::View),
        },
        &theme,
    );
    ui.painter().text(
        pos2(br.left() + 10.0, br.center().y),
        Align2::CENTER_CENTER,
        "←",
        font.clone(),
        c.text,
    );
    ui.painter().text(
        pos2(br.left() + 20.0, br.center().y),
        Align2::LEFT_CENTER,
        &back,
        font.clone(),
        c.text,
    );
    crate::ui_kit::icon_button_info(&resp, true, &back, None);
    if resp
        .on_hover_text(fp_i18n::t("ui-symbol-edit-back.tooltip"))
        .clicked()
    {
        st.push(Action::Run("modify.symbol.exitEdit".into(), json!({})));
    }
    let label = format!("{}  ▸  {}", path.0, path.1);
    widgets::text_clipped(
        ui.painter(),
        pos2(br.right() + 10.0, rect.center().y),
        Align2::LEFT_CENTER,
        &label,
        font,
        c.text,
        (rect.right() - br.right() - 16.0).max(0.0),
    );
}

// ---------------------------------------------------------------------- Reiter

/// Kontextmenü eines Dokumentreiters wie im Original: Alles hier zusammenführen · Schließen ·
/// Alle schließen · In neues Fenster verschieben │ Neues Dokument · Dokument öffnen. Firepixel hat
/// nur ein Dokumentfenster: „Zusammenführen“ und „In neues Fenster“ sind daher (wie im Original
/// bei einem Fenster) gesperrt.
pub fn tab_menu(id: fp_engine::DocId, _docs: usize) -> Vec<crate::menu::Node> {
    use crate::menu::{ItemAction, Node};
    let item = |key: &str, action: ItemAction, enabled: bool| Node::Item {
        label: fp_i18n::t(key),
        shortcut: None,
        action,
        enabled,
        checked: None,
    };
    vec![
        item("ui-tab-merge-all", ItemAction::None, false),
        item(
            "ui-tab-close",
            ItemAction::Ui(Action::CloseDocument(id)),
            true,
        ),
        item(
            "ui-tab-close-all",
            ItemAction::Command("file.closeAll".into()),
            true,
        ),
        item("ui-tab-move-to-new-window", ItemAction::None, false),
        Node::Separator,
        item(
            "ui-tab-new-document",
            ItemAction::Command("file.new".into()),
            true,
        ),
        item(
            "ui-tab-open-document",
            ItemAction::Command("file.open".into()),
            true,
        ),
    ]
}

fn doc_tabs(ui: &mut Ui, st: &mut AppState, rect: Rect) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    if theme.gradients {
        widgets::vgradient(ui.painter(), rect, c.doc_tab_bar_top, c.doc_tab_bar_bottom);
        ui.painter().hline(
            rect.x_range(),
            rect.top() + 0.5,
            Stroke::new(1.0, c.header_highlight),
        );
    } else {
        ui.painter().rect_filled(rect, 0.0, c.doc_tab_bar_top);
    }
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0, c.doc_tab_border),
    );
    // BATCH-2: Reiter „Batch“ links; ist das Studio vorn, ist kein Dokumentreiter aktiv
    let rect = crate::batch::doc_tab(ui, st, rect);
    let active = if crate::batch::is_front(st) {
        None
    } else {
        st.session.active_id()
    };
    // TW-36: Ein Dokument in Symbolbearbeitung hat keinen eigenen Reiter; der Reiter der Ansicht
    // zeigt „<Dokument> (Symbol)“ und den Änderungsstern des Dokuments.
    let hidden = hidden_by_symbol_edit(st);
    let docs: Vec<(fp_engine::DocId, String, bool)> = st
        .session
        .documents()
        .iter()
        .filter(|d| !hidden.contains(&d.id))
        .map(|d| {
            (
                d.id,
                d.title.clone(),
                d.is_dirty()
                    || d.symbol_edit
                        .as_ref()
                        .and_then(|l| st.session.document(l.parent))
                        .is_some_and(|p| p.is_dirty()),
            )
        })
        .collect();
    let font = theme.bold(theme.fonts.ui);
    // Überlauf wie im Original: passen nicht alle Reiter, steht rechts „»“ mit „Weitere Dokumente“;
    // der aktive Reiter bleibt immer sichtbar.
    let labels: Vec<String> = docs
        .iter()
        .map(|(_, t, dirty)| format!("{}{}", if *dirty { "*" } else { "" }, t))
        .collect();
    let widths: Vec<f32> = labels
        .iter()
        .map(|l| (widgets::text_width(ui, l, &font) + 34.0).min(260.0))
        .collect();
    let more_w = 24.0;
    let total: f32 = widths.iter().map(|w| w + 1.0).sum();
    let avail = rect.width() - 8.0;
    let overflow = total > avail;
    let room = if overflow { avail - more_w } else { avail };
    let active_i = docs
        .iter()
        .position(|(id, _, _)| active == Some(*id))
        .unwrap_or(0);
    let mut start = 0;
    while start < active_i
        && widths[start..=active_i]
            .iter()
            .map(|w| w + 1.0)
            .sum::<f32>()
            > room
    {
        start += 1;
    }
    let mut visible = vec![false; docs.len()];
    let mut used = 0.0;
    for i in start..docs.len() {
        if used + widths[i] + 1.0 > room && i != start {
            break;
        }
        used += widths[i] + 1.0;
        visible[i] = true;
    }
    let mut x = rect.left() + 8.0;
    let mut tab_rects: Vec<(fp_engine::DocId, Rect)> = Vec::new();
    let mut tab_actions: Vec<Action> = Vec::new();
    for (i, (id, _title, _dirty)) in docs.iter().enumerate() {
        if !visible[i] {
            continue;
        }
        let label = labels[i].clone();
        let w = widths[i];
        let tr = Rect::from_min_size(pos2(x, rect.top() + 2.0), vec2(w, rect.height() - 2.0));
        tab_rects.push((*id, tr));
        let resp = ui.interact(tr, Id::new(("doc-tab", id.0)), Sense::click());
        let is_active = active == Some(*id);
        if is_active {
            ui.painter().rect_filled(tr, 0.0, c.doc_tab_active_bg);
            ui.painter().rect_stroke(
                tr,
                0.0,
                Stroke::new(1.0, c.doc_tab_border),
                StrokeKind::Inside,
            );
            ui.painter().hline(
                tr.x_range().shrink(1.0),
                tr.bottom() - 0.5,
                Stroke::new(1.0, c.doc_tab_active_bg),
            );
            if theme.glow {
                let accent = theme.colors.group_accent[i % 6];
                ui.painter().rect_filled(
                    Rect::from_min_size(tr.min, vec2(tr.width(), 2.0)),
                    0.0,
                    accent,
                );
            }
        } else if resp.hovered() {
            ui.painter()
                .rect_filled(tr.shrink(1.0), 0.0, c.tab_hover_bg);
        }
        if theme.glow && !is_active {
            ui.painter().hline(
                tr.x_range().shrink(6.0),
                tr.bottom() - 2.5,
                Stroke::new(1.0, with_alpha(theme.colors.group_accent[i % 6], 150)),
            );
        }
        widgets::text_clipped(
            ui.painter(),
            pos2(tr.left() + 8.0, tr.center().y),
            Align2::LEFT_CENTER,
            &label,
            font.clone(),
            c.doc_tab_text,
            w - 30.0,
        );
        let xr = Rect::from_center_size(pos2(tr.right() - 11.0, tr.center().y), vec2(12.0, 12.0));
        let xresp = ui.interact(xr, Id::new(("doc-tab-x", id.0)), Sense::click());
        let xc = if xresp.hovered() {
            c.icon_red
        } else {
            c.text_weak
        };
        icons::paint(
            ui.painter(),
            xr.shrink(1.0),
            Icon::Close,
            &IconStyle::mono(xc, c.frame_bg),
        );
        crate::ui_kit::icon_button_info(&xresp, true, &fp_i18n::t("ui-tab-close"), None);
        if xresp.on_hover_text(fp_i18n::t("ui-tab-close")).clicked() {
            st.push(Action::CloseDocument(*id));
        } else if resp.clicked() {
            st.push(Action::Activate(*id));
        }
        if resp.middle_clicked() {
            st.push(Action::CloseDocument(*id));
        }
        // Kontextmenü des Reiters wie im Original (Strings 12287–12297)
        let nodes = tab_menu(*id, docs.len());
        resp.context_menu(|ui| crate::menu::show_nodes(ui, &nodes, &mut tab_actions));
        x += w + 1.0;
    }
    if overflow {
        let mr = Rect::from_min_size(
            pos2(rect.right() - more_w - 2.0, rect.top() + 3.0),
            vec2(more_w, rect.height() - 5.0),
        );
        let mresp = ui
            .interact(mr, Id::new("doc-tabs-more"), Sense::click())
            .on_hover_text(fp_i18n::t("ui-tab-more"));
        crate::ui_kit::icon_button_info(&mresp, true, &fp_i18n::t("ui-tab-more"), None);
        if mresp.hovered() {
            ui.painter().rect_filled(mr, 0.0, c.tab_hover_bg);
        }
        ui.painter().text(
            mr.center(),
            Align2::CENTER_CENTER,
            "»",
            font.clone(),
            c.doc_tab_text,
        );
        let nodes: Vec<crate::menu::Node> = docs
            .iter()
            .enumerate()
            .filter(|(i, _)| !visible[*i])
            .map(|(i, (id, _, _))| crate::menu::Node::Item {
                label: labels[i].clone(),
                shortcut: None,
                action: crate::menu::ItemAction::Ui(Action::Activate(*id)),
                enabled: true,
                checked: None,
            })
            .collect();
        crate::menu::popup_menu(ui, &mresp, &nodes, &mut tab_actions);
    }
    for a in tab_actions {
        st.push(a);
    }
    ui.ctx()
        .data_mut(|d| d.insert_temp(Id::new("lw-doc-tabs"), tab_rects));
}

/// Objekte, die gerade auf den Reiter eines anderen Dokuments gezogen wurden (Ablegen kopiert sie
/// dort hinein, wie im Original).
#[derive(Clone)]
struct Carry {
    from: fp_engine::DocId,
    elements: Vec<std::sync::Arc<fp_model::Element>>,
    /// Greifpunkt im Quelldokument.
    grab: Point,
}

const CARRY_ID: &str = "lw-carry";

/// Ablegen der getragenen Objekte im aktiven Dokument an `at` (ein Verlaufsschritt).
fn drop_carry(st: &mut AppState, c: &Carry, at: Point) {
    let old = std::mem::take(&mut st.session.clipboard);
    st.session.clipboard.elements = c.elements.clone();
    let (dx, dy) = (at.x - c.grab.x, at.y - c.grab.y);
    let r = st
        .session
        .run_group(&fp_i18n::t("ui-history-paste-objects"), |s| {
            s.execute("edit.paste", json!({}))?;
            if dx != 0.0 || dy != 0.0 {
                s.execute("edit.move", json!({"dx": dx, "dy": dy}))?;
            }
            Ok(())
        });
    st.session.clipboard = old;
    if let Err(e) = r {
        st.push(Action::Status(e.to_string()));
    }
}

// ---------------------------------------------------------------------- Ansichtsleiste

fn view_bar_ui(ui: &mut Ui, st: &mut AppState, rect: Rect) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    ui.painter().rect_filled(rect, 0.0, c.frame_bg);
    let Some(d) = st.session.active() else { return };
    let mode = d.view.preview_mode;
    let modes = [
        (
            PreviewMode::Original,
            Icon::ViewOriginal,
            "ui-view-original",
        ),
        (PreviewMode::Preview, Icon::ViewPreview, "ui-view-preview"),
        (PreviewMode::TwoUp, Icon::View2Up, "ui-view-2up"),
        (PreviewMode::FourUp, Icon::View4Up, "ui-view-4up"),
    ];
    let mut x = rect.left() + 4.0;
    let font = theme.ui_font();
    let mut chosen = None;
    // rechter Rand für die Ansichtsreiter: Platz für den Seitenknopf lassen (schmale Leinwand)
    let modes_right = rect.right() - 36.0;
    for (i, (m, icon, key)) in modes.iter().enumerate() {
        let label = fp_i18n::t(key);
        let tip = fp_i18n::t(&format!("{key}.tooltip"));
        let w = widgets::text_width(ui, &label, &font) + 26.0;
        if x + w > modes_right {
            break;
        }
        let r = Rect::from_min_size(pos2(x, rect.top() + 3.0), vec2(w, rect.height() - 6.0));
        let resp = ui.interact(r, Id::new(("view-mode", i)), Sense::click());
        let selected = mode == *m;
        widgets::button_frame(
            ui.painter(),
            r,
            &resp,
            ButtonLook {
                selected,
                enabled: true,
                group: Some(ToolGroup::View),
            },
            &theme,
        );
        icons::paint(
            ui.painter(),
            Rect::from_center_size(pos2(r.left() + 11.0, r.center().y), vec2(14.0, 14.0)),
            *icon,
            &IconStyle::new(&theme, Some(ToolGroup::View), true),
        );
        ui.painter().text(
            pos2(r.left() + 20.0, r.center().y),
            Align2::LEFT_CENTER,
            &label,
            font.clone(),
            c.text,
        );
        crate::ui_kit::icon_button_info(&resp, true, &label, Some(selected));
        if resp.on_hover_text(tip).clicked() {
            chosen = Some(*m);
        }
        x += w + 2.0;
        if i == 0 {
            ui.painter().vline(
                x + 0.5,
                (rect.top() + 5.0)..=(rect.bottom() - 5.0),
                Stroke::new(1.0, c.line_mid),
            );
            ui.painter().vline(
                x + 1.5,
                (rect.top() + 5.0)..=(rect.bottom() - 5.0),
                Stroke::new(1.0, c.line_light),
            );
            x += 4.0;
        }
    }
    ui.painter().vline(
        x + 2.5,
        (rect.top() + 5.0)..=(rect.bottom() - 5.0),
        Stroke::new(1.0, c.line_mid),
    );
    if let Some(m) = chosen
        && let Some(d) = st.session.active_mut()
    {
        d.view.preview_mode = m;
    }
    // Seitenauswahl rechts
    let Some(d) = st.session.active() else { return };
    let names: Vec<String> = d
        .doc
        .pages
        .iter()
        .enumerate()
        .map(|(i, p)| {
            if p.display_name().is_empty() {
                fp_i18n::tr!("ui-page-default-name", n = i + 1)
            } else {
                p.display_name().to_string()
            }
        })
        .collect();
    let page = d.page;
    let nav = Rect::from_center_size(pos2(rect.right() - 20.0, rect.center().y), vec2(18.0, 18.0));
    // Seitenauswahl rechts, bei schmaler Leinwand nach links begrenzt bzw. ausgeblendet (nichts überlappt)
    let combo = Rect::from_min_max(
        pos2((rect.right() - 236.0).max(x + 14.0), rect.top() + 3.0),
        pos2(nav.left() - 8.0, rect.bottom() - 3.0),
    );
    ui.painter().vline(
        nav.left() - 4.5,
        (rect.top() + 5.0)..=(rect.bottom() - 5.0),
        Stroke::new(1.0, c.line_mid),
    );
    if combo.width() >= 90.0 {
        ui.painter().vline(
            combo.left() - 6.5,
            (rect.top() + 5.0)..=(rect.bottom() - 5.0),
            Stroke::new(1.0, c.line_mid),
        );
        icons::paint(
            ui.painter(),
            Rect::from_center_size(pos2(combo.left() + 8.0, combo.center().y), vec2(13.0, 13.0)),
            Icon::PageSmall,
            &IconStyle::new(&theme, None, false),
        );
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(
            pos2(combo.left() + 18.0, combo.top()),
            combo.max,
        )));
        child.set_clip_rect(combo);
        let mut sel = page;
        egui::ComboBox::from_id_salt("page-select")
            .width(combo.width() - 26.0)
            .truncate()
            .selected_text(names.get(page).cloned().unwrap_or_default())
            .show_ui(&mut child, |ui| {
                for (i, n) in names.iter().enumerate() {
                    ui.selectable_value(&mut sel, i, n.as_str());
                }
            });
        if sel != page {
            st.push(Action::SetPage(sel));
        }
    }
    let resp = ui.interact(nav, Id::new("page-nav"), Sense::click());
    widgets::button_frame(
        ui.painter(),
        nav,
        &resp,
        ButtonLook {
            selected: false,
            enabled: names.len() > 1,
            group: Some(ToolGroup::View),
        },
        &theme,
    );
    icons::paint(
        ui.painter(),
        nav.shrink(2.0),
        Icon::PageNavigate,
        &IconStyle::new(&theme, Some(ToolGroup::View), names.len() > 1),
    );
    crate::ui_kit::icon_button_info(&resp, names.len() > 1, &fp_i18n::t("ui-page-nav"), None);
    if resp.on_hover_text(fp_i18n::t("ui-page-nav")).clicked() && names.len() > 1 {
        st.push(Action::SetPage((page + 1) % names.len()));
    }
}

// ---------------------------------------------------------------------- Leinwand

/// Größe des Sichtbereichs (Bildschirmpunkte) des aktiven Dokuments melden. Hat sich seit der
/// letzten Meldung die Ansicht (Original/Vorschau/2-fach/4-fach) geändert, bleibt der Dokumentpunkt
/// in der Mitte stehen: alle Ansichten teilen eine Lage (Zoom und Mittelpunkt, N25), und Felder
/// verschiedener Größe zentrieren auf denselben Punkt.
pub(crate) fn report_viewport(st: &mut AppState, size: egui::Vec2) {
    let Some(d) = st.session.active() else { return };
    let (id, mode) = (d.id, d.view.preview_mode);
    let cc = st.canvases.entry(id).or_default();
    let switched = cc.view_mode.is_some_and(|m| m != mode);
    cc.view_mode = Some(mode);
    let prev = cc.switch_center;
    let Some(d) = st.session.active_mut() else {
        return;
    };
    let new = (size.x.max(1.0) as f64, size.y.max(1.0) as f64);
    let old = d.view.viewport;
    let mut keep = None;
    if switched && old != new {
        let z = d.view.zoom.max(1e-9);
        let now = (
            d.view.origin.0 + old.0 / 2.0 / z,
            d.view.origin.1 + old.1 / 2.0 / z,
        );
        // seit dem letzten Wechsel nicht bewegt (nur ausgerichtet): dessen genauen Punkt nehmen
        let c = prev
            .filter(|p| ((p.0 - now.0) * z).abs() < 1.0 && ((p.1 - now.1) * z).abs() < 1.0)
            .unwrap_or(now);
        d.view.origin = (c.0 - new.0 / 2.0 / z, c.1 - new.1 / 2.0 / z);
        keep = Some(c);
    }
    d.view.viewport = new;
    if keep.is_some()
        && let Some(cc) = st.canvases.get_mut(&id)
    {
        cc.switch_center = keep;
    }
}

/// Seitenrechteck in Dokumentkoordinaten.
pub(crate) fn page_rect(doc: &fp_model::Document, page: usize) -> Option<IRect> {
    fp_render::Renderer::page_rect(doc, page)
}

/// Ansicht so begrenzen, dass die Seite im Bild bleibt; kleinere Seiten werden zentriert.
/// Der Bildlauf endet genau am Dokumentrand wie im Original (re\22-ansicht-bitmap-farbe.md §3.1,
/// BV-5; vorher 60 px darüber hinaus).
#[cfg(test)]
pub(crate) fn clamp_view(view: &mut fp_engine::ViewState, pr: IRect) {
    clamp_view_bars(view, pr, 0.0);
}

/// Breite der Bildlaufleisten über der Leinwand (Punkte).
pub(crate) const SCROLLBAR_T: f64 = 8.0;

/// Wie [`clamp_view`]; `bar` = Breite der Bildlaufleisten, die rechts bzw. unten über der Fläche
/// liegen, sobald die Seite in der anderen Richtung nicht hineinpasst. Der rechte bzw. untere
/// Dokumentrand endet dann genau an der Leiste statt darunter.
pub(crate) fn clamp_view_bars(view: &mut fp_engine::ViewState, pr: IRect, bar: f64) {
    let z = view.zoom.max(1e-6);
    let (vw, vh) = view.viewport;
    let (pw, ph) = (pr.width() as f64, pr.height() as f64);
    let (bar_r, bar_b) = scrollbar_insets(view, pr, bar);
    if pw * z <= vw {
        view.origin.0 = pr.left as f64 + pw / 2.0 - vw / 2.0 / z;
    } else {
        let hi = (pr.right as f64 - (vw - bar_r) / z).max(pr.left as f64);
        view.origin.0 = view.origin.0.clamp(pr.left as f64, hi);
    }
    if ph * z <= vh {
        view.origin.1 = pr.top as f64 + ph / 2.0 - vh / 2.0 / z;
    } else {
        let hi = (pr.bottom as f64 - (vh - bar_b) / z).max(pr.top as f64);
        view.origin.1 = view.origin.1.clamp(pr.top as f64, hi);
    }
}

/// (rechts, unten): Platz der senkrechten bzw. waagerechten Bildlaufleiste, wenn sie gezeigt wird.
fn scrollbar_insets(view: &fp_engine::ViewState, pr: IRect, bar: f64) -> (f64, f64) {
    let z = view.zoom;
    let (vw, vh) = view.viewport;
    let v_shown = pr.height() as f64 * z > vh + 1.0;
    let h_shown = pr.width() as f64 * z > vw + 1.0;
    (
        if v_shown { bar } else { 0.0 },
        if h_shown { bar } else { 0.0 },
    )
}

/// Ansichtsursprung so verschieben (um weniger als einen halben Gerätepixel), dass die linke obere
/// Seitenecke genau auf einem Gerätepixel liegt. Die Seite wird ohnehin auf ganze Gerätepixel
/// gezeichnet; ohne diesen Abgleich lag der Zeiger bei ungerader Leinwandgröße einen halben Pixel
/// neben dem gezeichneten Dokumentpunkt (Bildschirmpixel l + x ergab x + 0,5 – PM-1, L16:
/// Gleichstand beim Fang). Bei 100 % entspricht damit Zeiger (l + x, t + y) genau Punkt (x, y).
fn align_view_to_pixels(view: &mut fp_engine::ViewState, pr: IRect, area_min: Pos2, ppp: f64) {
    let z = view.zoom.max(1e-6);
    let ppp = ppp.max(1e-6);
    for (axis, a) in [(0, area_min.x as f64), (1, area_min.y as f64)] {
        let (o, edge) = if axis == 0 {
            (view.origin.0, pr.left as f64)
        } else {
            (view.origin.1, pr.top as f64)
        };
        let dev = (a + (edge - o) * z) * ppp;
        let fix = (dev - dev.round()) / (ppp * z);
        if fix.abs() > 1e-9 {
            if axis == 0 {
                view.origin.0 += fix;
            } else {
                view.origin.1 += fix;
            }
        }
    }
}

fn to_screen(view: &fp_engine::ViewState, origin: Pos2, x: f64, y: f64) -> Pos2 {
    let (sx, sy) = view.doc_to_screen(Point::new(x as f32, y as f32));
    pos2(origin.x + sx as f32, origin.y + sy as f32)
}

/// Rohes Zeigerereignis eines Bildes (in Reihenfolge).
#[derive(Clone, Copy, Debug)]
enum RawPtr {
    Mods(egui::Modifiers),
    Move(Pos2),
    Button(Pos2, bool, egui::Modifiers),
    /// Leertaste gedrückt bzw. losgelassen (Tastenzustand je Rohereignis, PARITAET_A L7).
    Space(bool),
}

fn mods(i: &egui::InputState, space: bool) -> Modifiers {
    Modifiers {
        shift: i.modifiers.shift,
        alt: i.modifiers.alt,
        ctrl: i.modifiers.ctrl || i.modifiers.command,
        space,
        caps: false,
    }
}

fn canvas_ui(ui: &mut Ui, st: &mut AppState, rect: Rect) {
    let theme = Theme::current(ui.ctx());
    let c = theme.colors.clone();
    let Some(d) = st.session.active() else { return };
    let id = d.id;
    let chrome = crate::screen_mode::current(st);
    let show_rulers = d.view.show_rulers && chrome.rulers;
    let ruler = if show_rulers {
        theme.metrics.ruler_size
    } else {
        0.0
    };
    let area = Rect::from_min_max(rect.min + vec2(ruler, ruler), rect.max);
    // Hintergrund mit Schatten unter der Ansichtsleiste wie CS6
    if chrome.black {
        // TW-158: full screen = the canvas alone on black
        ui.painter()
            .rect_filled(rect, 0.0, crate::screen_mode::FULL_PASTEBOARD);
    } else {
        ui.painter().rect_filled(rect, 0.0, c.canvas_bg);
    }
    if theme.gradients && !chrome.black {
        for k in 0..5 {
            let col = crate::theme::mix(c.frame_bg, c.canvas_bg, k as f32 / 4.0);
            ui.painter().hline(
                rect.x_range(),
                rect.top() + k as f32 + 0.5,
                Stroke::new(1.0, col),
            );
        }
    }
    let ppp = ui.ctx().pixels_per_point() as f64;
    // Sichtbereich melden, Erst-Einpassung, Begrenzung
    let needs_fit = st.canvases.entry(id).or_default().needs_initial_fit;
    report_viewport(st, area.size());
    if let Some(d) = st.session.active_mut()
        && let Some(pr) = page_rect(&d.doc, d.page)
    {
        if needs_fit {
            if pr.width() as f64 > d.view.viewport.0 || pr.height() as f64 > d.view.viewport.1 {
                d.view.fit_rect(fp_geom::Rect::new(
                    pr.left as f32,
                    pr.top as f32,
                    pr.right as f32,
                    pr.bottom as f32,
                ));
            } else {
                d.view.zoom = 1.0;
            }
        }
        clamp_view_bars(&mut d.view, pr, SCROLLBAR_T);
        align_view_to_pixels(&mut d.view, pr, area.min, ppp);
    }
    if needs_fit && let Some(cc) = st.canvases.get_mut(&id) {
        cc.needs_initial_fit = false;
    }
    st.canvases.entry(id).or_default().area = Some(area);
    overlay::sync_tool_attrs(ui.ctx(), st);
    let resp = ui.interact(area, Id::new(("canvas", id.0)), Sense::click_and_drag());
    input(ui, st, area, &resp);
    // Zoom/Bildlauf aus diesem Bild sofort zentrieren bzw. begrenzen – sonst würde ein Bild lang mit
    // dem neuen Zoom, aber dem alten Versatz gezeichnet
    if let Some(d) = st.session.active_mut()
        && let Some(pr) = page_rect(&d.doc, d.page)
    {
        d.view.viewport = (area.width().max(1.0) as f64, area.height().max(1.0) as f64);
        clamp_view_bars(&mut d.view, pr, SCROLLBAR_T);
        align_view_to_pixels(&mut d.view, pr, area.min, ppp);
    }
    // Rückmeldung des Werkzeugs (auch ohne Ereignis) und im Hintergrund auszublendende Elemente
    let feedback = st.session.tool_feedback();
    // Rote Vorauswahl-Hervorhebung nur, solange der Zeiger über der Leinwand ist: Verlässt er das
    // Fenster (bzw. liegt ein Bedienfeld darüber), kommt kein Bewegungsereignis mehr, das sie
    // löscht; die alte Hervorhebung blieb stehen (CT-3: rote Linie ab dem Seitenursprung)
    let pointer_here = resp.hovered()
        && ui
            .ctx()
            .pointer_hover_pos()
            .is_some_and(|p| area.contains(p));
    // the brush tip outline is drawn only while the pointer is over the canvas (RE spec 57 §2:
    // it disappears as soon as the pointer leaves the canvas, also while a stroke runs)
    let pressed = st.canvases.get(&id).is_some_and(|c| c.pointer_down);
    let feedback: Vec<Overlay> = if pointer_here {
        feedback
    } else {
        feedback
            .into_iter()
            .filter(|o| match o {
                Overlay::Highlight { .. } => pressed,
                Overlay::BrushTip { .. } => false,
                _ => true,
            })
            .collect()
    };
    // object selection: mask preview while hovering (BA-037)
    tools::ai_object::update(ui.ctx(), st, pointer_here);
    let preview_drag = st.session.prefs.get_bool("PreviewDrag", true);
    // Stapelfolge der Vorschau (N34, BA-039): gezogene bzw. bemalte Objekte bleiben zwischen den
    // Objekten darunter (Hintergrundbild) und darüber (durchsichtige Schichten)
    let mut text_hidden: Vec<u64> = Vec::new();
    let mut source: Option<(Vec<u64>, bool, bool)> = None;
    // FB-87 / TW-241: a pixel move of a bitmap with opacity, blend mode or live effects renders the
    // moved bitmap as the element itself (render-only document copy), not as ghost sprites
    let live_px = preview_drag
        .then(|| {
            st.session
                .active()
                .and_then(|d| ghost::live_pixels(&feedback, &d.doc, d.page))
        })
        .flatten();
    for o in &feedback {
        match o {
            Overlay::Ghost { ids, copy, .. }
                if preview_drag && source.is_none() && live_px.is_none() =>
            {
                source = Some((ids.clone(), *copy, true))
            }
            // BA-039: a stroke on a bitmap the page draws directly is composited in place (full
            // page composite per tile); only other strokes use the layered overlay
            Overlay::Paint(p) if source.is_none() => {
                let in_place = st
                    .session
                    .active()
                    .is_some_and(|d| paint_preview::composite_possible(&d.doc, d.page, p));
                if !in_place {
                    source = p.target.map(|t| (vec![t], true, false))
                }
            }
            Overlay::TextEdit(v) => text_hidden.extend(v.target),
            // Zeiger zieht einen Füllgriff: das Objekt zeichnet die Vorschau live (re 40 §1)
            Overlay::FillLive { target, .. } => text_hidden.push(*target),
            // the open path the pen is drawing: drawn without its fill (RE spec 60 section 1.2)
            Overlay::PenPath { target }
                if st
                    .session
                    .active()
                    .and_then(|d| pen_path_without_fill(&d.doc, d.page, *target))
                    .is_some() =>
            {
                text_hidden.push(*target)
            }
            _ => {}
        }
    }
    let plan = source.and_then(|(ids, copy, ghost)| {
        let d = st.session.active()?;
        let src_key = stack::source_key(d.generation, d.page, d.frame, &ids, copy);
        let cc = st.canvases.entry(id).or_default();
        if cc.stack_src != src_key || cc.stack.is_none() {
            // gezogen: notfalls wie bisher (Vorschau oben); gemalt: ohne Schichtung nichts ausblenden
            cc.stack = stack::plan(&d.doc, d.page, d.frame, &ids, copy)
                .or_else(|| ghost.then(|| stack::StackPlan::flat(&ids, copy)));
            cc.stack_src = src_key;
        }
        cc.stack.clone()
    });
    let mut hidden: Vec<stack::Atom> = plan.as_ref().map(|p| p.hidden.clone()).unwrap_or_default();
    hidden.extend(text_hidden.iter().map(|t| stack::Atom::Elem(*t)));
    let layers = plan.as_ref().map(|p| p.layers()).unwrap_or_default();
    let variant = stack::variant(
        plan.as_ref().unwrap_or(&stack::StackPlan::default()),
        &text_hidden,
    );
    {
        let cc = st.canvases.entry(id).or_default();
        if plan.is_none() {
            cc.stack = None;
        }
        cc.stack_variant = variant;
    }
    // Zeichnen
    let Some(d) = st.session.active() else { return };
    let view = d.view.clone();
    let Some(pr) = page_rect(&d.doc, d.page) else {
        return;
    };
    let painter = ui.painter_at(area);
    let bg = d
        .doc
        .pages
        .get(d.page)
        .map(|p| p.root.background)
        .unwrap_or(fp_color::Argb(0xffffffff));
    // Rendern: Gerätepixel im Gitter der Seite (Ursprung: linke obere Seitenecke, auf ganze
    // Gerätepixel gerundet), als Kacheln ([`render::TILE`]); sichtbarer Teil plus Rand
    let zoom_dev = view.zoom * ppp;
    let ppf = ppp as f32;
    let origin = to_screen(&view, area.min, pr.left as f64, pr.top as f64);
    let (gx0, gy0) = (
        (origin.x * ppf).round() as f64,
        (origin.y * ppf).round() as f64,
    );
    let extent = IRect::new(
        0,
        0,
        render::device_edge(pr.left as i64, pr.right as i64, zoom_dev) as i32,
        render::device_edge(pr.top as i64, pr.bottom as i64, zoom_dev) as i32,
    );
    // sichtbare Gerätepixel (relativ zum Gitterursprung)
    let visible_dev = IRect::new(
        ((area.min.x as f64 * ppp) - gx0).floor() as i32,
        ((area.min.y as f64 * ppp) - gy0).floor() as i32,
        ((area.max.x as f64 * ppp) - gx0).ceil() as i32,
        ((area.max.y as f64 * ppp) - gy0).ceil() as i32,
    )
    .intersect(&extent);
    let around = |f: f64| {
        let (mx, my) = (
            ((view.viewport.0 * ppp * f) as i32).min(4096),
            ((view.viewport.1 * ppp * f) as i32).min(4096),
        );
        render::snap_to_tiles(
            IRect::new(
                visible_dev.left - mx,
                visible_dev.top - my,
                visible_dev.right + mx,
                visible_dev.bottom + my,
            ),
            extent,
        )
    };
    let (generation, page, frame, docarc) =
        (d.generation, d.page, d.frame, std::sync::Arc::clone(&d.doc));
    // bitmap mask in bitmap editing: no fill/stroke underlay (RE spec 68 §4.2 rule 5, TW-225)
    let mask_edit = d.bitmap_mask_edit().map_or(0, |m| m.0);
    // pending inspector slider value (FB-09/FB-10): rendered from a preview copy of the document
    let preview = crate::inspector::preview::preview_key(st, id, generation, page);
    // FB-67: live point drag (View ▸ Live Preview While Dragging Points), from a copy as well
    let live_path = crate::inspector::preview::live_path(&feedback);
    let preview = crate::inspector::preview::combined_key(
        preview,
        live_path.map_or(0, |(t, cs)| {
            crate::inspector::preview::live_path_key(generation, page, t, cs)
        }),
    );
    let live_px_mask = live_px.and_then(|lp| {
        d.selection
            .pixels
            .as_ref()
            .filter(|_| d.selection.has_pixels())
            .map(|m| (lp, ghost::live_pixels_key(generation, page, &lp, m)))
    });
    let preview =
        crate::inspector::preview::combined_key(preview, live_px_mask.map_or(0, |(_, k)| k));
    let cc = st.canvases.entry(id).or_default();
    // Ergebnisse des Threads übernehmen
    let gamma = view.gamma_preview;
    let fresh = |k: &RenderKey| {
        k.doc == id
            && k.preview == preview
            && k.mask_edit == mask_edit
            && k.generation == generation
            && k.epoch == cc.epoch
            && k.page == page
            && k.frame == frame
            && k.variant == variant
            && k.gamma == gamma
            && k.page_rect == pr
            && (k.zoom - zoom_dev).abs() < 1e-9
    };
    let holds = |k: &RenderKey, r: IRect| fresh(k) && k.dev.intersect(&r) == r;
    let covered = cc.shown.as_ref().is_some_and(|k| holds(k, visible_dev));
    let pending_ok = cc.requested.as_ref().is_some_and(|k| holds(k, visible_dev));
    // Rand erst, wenn der sichtbare Teil steht (BA-098: nach dem Zoomen zuerst nur die sichtbaren
    // Kacheln hochladen); nachgefordert, sobald der Rand auf ein Viertel der Ansicht schrumpft
    let same_zoom = cc
        .shown
        .as_ref()
        .is_some_and(|k| k.doc == id && k.page == page && (k.zoom - zoom_dev).abs() < 1e-9);
    // höchstens die größte Texturseite des Grafikadapters (die Schichten über der Vorschau sind
    // eine Textur je Bild): sonst nur der sichtbare Teil, notfalls um seine Mitte begrenzt
    let max_tex = (ui.ctx().input(|i| i.max_texture_side) as i32).min(8000);
    let fits = |r: &IRect| r.width() as i32 <= max_tex && r.height() as i32 <= max_tex;
    let cap = |dev: IRect| {
        let vis = render::snap_to_tiles(visible_dev, extent);
        if fits(&dev) {
            dev
        } else if fits(&vis) {
            vis
        } else {
            let (w, h) = (
                (visible_dev.width() as i32).min(max_tex),
                (visible_dev.height() as i32).min(max_tex),
            );
            let cx = (visible_dev.left + visible_dev.right) / 2;
            let cy = (visible_dev.top + visible_dev.bottom) / 2;
            IRect::new(cx - w / 2, cy - h / 2, cx - w / 2 + w, cy - h / 2 + h)
        }
    };
    let margin_due = covered
        && cc.requested == cc.shown
        && cc
            .shown
            .as_ref()
            .is_some_and(|k| !holds(k, around(0.25)) && k.dev != cap(around(0.5)));
    if !visible_dev.is_empty() && ((!covered && !pending_ok) || margin_due) {
        let dev = cap(if same_zoom || margin_due {
            around(0.5)
        } else {
            render::snap_to_tiles(visible_dev, extent)
        });
        let key = RenderKey {
            doc: id,
            generation,
            epoch: cc.epoch,
            page,
            frame,
            region: render::covered_region(pr, dev, zoom_dev),
            page_rect: pr,
            dev,
            zoom: zoom_dev,
            variant,
            gamma,
            preview,
            mask_edit,
        };
        cc.requested = Some(key);
        let docarc = if preview != 0 {
            let base = crate::inspector::preview::preview_doc(st, id, generation, page, &docarc)
                .unwrap_or(docarc);
            let base = match live_path {
                Some((t, cs)) => crate::inspector::preview::apply_live_path(&base, page, t, cs),
                None => base,
            };
            match (
                live_px_mask,
                st.session
                    .active()
                    .and_then(|d| d.selection.pixels.as_ref()),
            ) {
                (Some((lp, _)), Some(m)) => ghost::apply_live_pixels(&base, page, &lp, m),
                _ => base,
            }
        } else {
            docarc
        };
        if let Some(res) = st.worker.request(
            RenderJob {
                key,
                doc: docarc,
                hidden: hidden.clone(),
                layers: layers.clone(),
            },
            st.sync_render,
        ) {
            render::show_result(ui.ctx(), st.canvases.entry(id).or_default(), res);
        }
    }
    // Seitenlage in ganzen Gerätepixeln, abgeleitet aus derselben Abtastregel wie die Textur
    // (fp-render: Gerätepixel i zeigt Dokumentpixel `links + ⌊(i + ½) / Zoom⌋`): Gerätepixel p liegt
    // in der Seite, wenn sein Abtastpunkt in der Seite liegt. Hintergrund, Schatten, Schachbrett,
    // Ausschnitt der Textur und Vorschauen nutzen alle dieses eine Rechteck – keine Lücke und kein
    // Überstand an den Kanten, auch bei Bruchteil-Zoom und 125/150 % DPI.
    let cc = st.canvases.entry(id).or_default();
    // bis das Ergebnis des Hintergrund-Threads abgeholt ist, weiter Bilder anfordern (auf den
    // Weckruf des Threads allein ist kein Verlass, PM-1 N13)
    if cc.requested.is_some() && cc.requested != cc.shown {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(16));
    }
    let tex_now = match &cc.shown {
        Some(k) if k.doc == id && k.page == page && !cc.tiles.is_empty() => Some(*k),
        _ => None,
    };
    let fresh_tex = tex_now.filter(|k| (k.zoom - zoom_dev).abs() < 1e-9);
    // Ursprung des Abtastgitters (Gerätepixel, ganzzahlig) und Bezugs-Dokumentpunkt
    let (gx, gy, rl, rt) = match fresh_tex {
        Some(k) => {
            let a = to_screen(
                &view,
                area.min,
                k.page_rect.left as f64,
                k.page_rect.top as f64,
            );
            (
                (a.x * ppf).round() as f64,
                (a.y * ppf).round() as f64,
                k.page_rect.left as f64,
                k.page_rect.top as f64,
            )
        }
        None => (gx0, gy0, pr.left as f64, pr.top as f64),
    };
    // erster Gerätepixel i (ab dem Gitterursprung), dessen Abtastpunkt bei oder hinter der Kante `e`
    // liegt – mit genau den Gleitkomma-Schritten von fp-render (sonst kippt die letzte Zeile)
    let edge =
        |g: f64, r: f64, e: f64| g + render::device_edge(r as i64, e as i64, zoom_dev) as f64;
    let page_screen = Rect::from_min_max(
        pos2(
            (edge(gx, rl, pr.left as f64) / ppp) as f32,
            (edge(gy, rt, pr.top as f64) / ppp) as f32,
        ),
        pos2(
            (edge(gx, rl, pr.right as f64) / ppp) as f32,
            (edge(gy, rt, pr.bottom as f64) / ppp) as f32,
        ),
    );
    cc.page_screen = Some(page_screen);
    if theme.kind != crate::theme::ThemeKind::Oled && !chrome.black {
        painter.rect_filled(
            page_screen.translate(vec2(2.0, 2.0)),
            0.0,
            with_alpha(Color32::BLACK, 30),
        );
    }
    if (bg.0 >> 24) < 255 {
        widgets::checker_view(
            &painter.with_clip_rect(page_screen.intersect(area)),
            page_screen.intersect(area),
            8.0,
            &theme,
            gamma,
        );
    }
    // An Position bearbeiten: übrige Objekte der Seite abgeblendet dahinter (re 28 S2)
    paint_symbol_backdrop(
        ui.ctx(),
        st,
        &painter.with_clip_rect(page_screen.intersect(area)),
        page_screen,
        area,
        &view,
    );
    if let Some(k) = &tex_now {
        let page_painter = painter.with_clip_rect(page_screen.intersect(area));
        let (a, scale) = if fresh_tex.is_some() {
            // aktuelle Kacheln: Pixel genau auf dem Gerätepixelgitter
            (pos2((gx / ppp) as f32, (gy / ppp) as f32), 1.0)
        } else {
            // alte Kacheln mit ihrem eigenen Zoom auf die neue Ansicht abbilden, bis die neuen da sind
            let a = to_screen(
                &view,
                area.min,
                k.page_rect.left as f64,
                k.page_rect.top as f64,
            );
            (a, (view.zoom * ppp / k.zoom.max(1e-9)) as f32)
        };
        let place = |r: IRect| {
            Rect::from_min_size(
                a + vec2(r.left as f32, r.top as f32) / ppf * scale,
                vec2(r.width() as f32, r.height() as f32) / ppf * scale,
            )
        };
        if let Some(cc) = st.canvases.get(&id) {
            for t in &cc.tiles {
                page_painter.image(
                    t.texture.id(),
                    place(t.dev),
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
        }
        st.canvases.entry(id).or_default().tex_rect = Some(place(k.dev));
    }
    preview_stack(ui, st, page_screen.intersect(area), area, &view, &feedback);
    if theme.glow {
        painter.rect_stroke(
            page_screen,
            0.0,
            Stroke::new(1.0, with_alpha(c.focus, 90)),
            StrokeKind::Outside,
        );
    }
    let spp = |p: Point| to_screen(&view, area.min, p.x as f64, p.y as f64);
    overlay::swatch_feedback(
        ui.ctx(),
        st,
        &painter.with_clip_rect(page_screen.intersect(area)),
        &spp,
    );
    overlays(ui, st, &painter, area, &view, &feedback);
    tools::ai_object::paint(ui, st, &painter, area, &view);
    if let Some(cc) = st.canvases.get_mut(&id) {
        cc.info = feedback.iter().rev().find_map(|o| match o {
            Overlay::Label { text, .. } => Some(text.clone()),
            _ => None,
        });
        cc.feedback = feedback;
    }
    // View ▸ Coordinate Tag: tag next to the pointer, per tool (re 58 §4, FB-41)
    let tag = pointer_tag(ui, st, id, area, &view, pointer_here);
    if let Some((at, text)) = &tag {
        pointer_tag_box(&painter, &theme, *at, text);
    }
    if let Some(cc) = st.canvases.get_mut(&id) {
        cc.pointer_tag = tag.map(|(_, text)| text);
    }
    if chrome.black {
        // page edge 1 px at the top and left, no shadow (full screen, TW-158)
        let edge = Stroke::new(1.0, crate::screen_mode::PAGE_EDGE);
        let e = page_screen.expand(0.5);
        painter.hline(e.left()..=e.right(), e.top(), edge);
        painter.vline(e.left(), e.top()..=e.bottom(), edge);
    }
    if show_rulers {
        rulers(ui, st, rect, area, &view);
    }
    if chrome.scrollbars {
        scrollbars(ui, st, area, &view, pr);
    }
    // Kontextmenü (Rechtsklick)
    if resp.secondary_clicked()
        && !st.dialogs.any_open()
        && let Some(p) = resp.interact_pointer_pos()
    {
        let at = view.screen_to_doc((p.x - area.left()) as f64, (p.y - area.top()) as f64);
        let m = context::open_at(st, at);
        if let Some(cc) = st.canvases.get_mut(&id) {
            cc.context = Some(m);
        }
    }
    let menu = st.canvases.get(&id).and_then(|c| c.context.clone());
    context::show(ui, &resp, st, menu.as_ref());
    // eigener Mauszeiger
    paint_cursor(ui, st, area, &resp);
}

/// Coordinate tag next to the pointer (re 58 §4, measured: the reference ties it to *View ▸
/// Tooltips*; Firepixel gives it the own switch *View ▸ Coordinate Tag*, FB-41), per tool:
/// - shape tools (Rectangle, Ellipse, Line, Polygon, Auto Shape): on hover the pointer position
///   "x:53 y:37"; while a shape is drawn its size "w:80 h:50";
/// - Pointer: nothing on hover or on a press without movement; while objects are dragged the
///   dragged objects' new top-left corner "x:130 y:120";
/// - every other tool (Pen, Text …): no tag.
///
/// Document pixels rounded; no delay; the tag's top-left is at pointer x + 0, y + 14 and it goes
/// away on release. Hidden while the view pans, a guide is dragged, the context menu or a dialog is
/// open. Returns the screen position of the tag and its text.
fn pointer_tag(
    ui: &Ui,
    st: &AppState,
    id: fp_engine::DocId,
    area: Rect,
    view: &fp_engine::ViewState,
    pointer_here: bool,
) -> Option<(Pos2, String)> {
    use fp_engine::tools::ToolId;
    if !fp_engine::session::coordinate_tag(&st.session.prefs) || st.dialogs.any_open() {
        return None;
    }
    let cc = st.canvases.get(&id);
    let pressed = cc.is_some_and(|c| c.pointer_down);
    // the tool's feedback of this frame
    let feedback = cc.map(|c| c.feedback.as_slice()).unwrap_or(&[]);
    // a drag that started on the canvas keeps the tag even if egui stops reporting hover
    if !(pointer_here || pressed) {
        return None;
    }
    let busy = cc.is_some_and(|c| {
        c.panning || c.guide_move.is_some() || c.guide_drag.is_some() || c.context.is_some()
    });
    if busy {
        return None;
    }
    let p = ui.ctx().pointer_hover_pos().filter(|p| area.contains(*p))?;
    let shape_tool = matches!(
        st.session.tools.active,
        ToolId::Rectangle | ToolId::Ellipse | ToolId::Line | ToolId::Polygon | ToolId::AutoShape
    );
    let text = match st.session.tools.active {
        ToolId::Pointer if pressed => {
            let quad = feedback.iter().find_map(|o| match o {
                Overlay::Ghost { quad, .. } => Some(*quad),
                _ => None,
            })?;
            let x = quad.iter().map(|q| q.x).fold(f32::INFINITY, f32::min);
            let y = quad.iter().map(|q| q.y).fold(f32::INFINITY, f32::min);
            fp_i18n::tr!(
                "ui-canvas-pointer-tag",
                x = x.round() as f64,
                y = y.round() as f64
            )
        }
        _ if shape_tool && pressed => {
            let r = shape_preview_bounds(feedback)?;
            fp_i18n::tr!(
                "ui-canvas-size-tag",
                w = r.width().round() as f64,
                h = r.height().round() as f64
            )
        }
        _ if shape_tool => {
            let d = view.screen_to_doc((p.x - area.left()) as f64, (p.y - area.top()) as f64);
            fp_i18n::tr!(
                "ui-canvas-pointer-tag",
                x = d.x.round() as f64,
                y = d.y.round() as f64
            )
        }
        _ => return None,
    };
    Some((p + vec2(0.0, 14.0), text))
}

/// Bounds of the shape a shape tool is drawing (its first outline overlay); `None` before the
/// pointer has moved.
fn shape_preview_bounds(feedback: &[Overlay]) -> Option<fp_geom::Rect> {
    let pts: Vec<Point> = feedback.iter().find_map(|o| match o {
        Overlay::Rect { rect, .. } | Overlay::Ellipse { rect, .. } => {
            let r = rect.normalized();
            Some(vec![
                Point::new(r.left, r.top),
                Point::new(r.right, r.bottom),
            ])
        }
        Overlay::Line { from, to } => Some(vec![*from, *to]),
        Overlay::Polyline { points, .. } if !points.is_empty() => Some(points.clone()),
        Overlay::Path { contours } if !contours.is_empty() => Some(
            contours
                .iter()
                .flat_map(|c| c.nodes.iter().map(|n| n.anchor))
                .collect(),
        ),
        _ => None,
    })?;
    let (mut l, mut t, mut r, mut b) = (
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    );
    for q in &pts {
        l = l.min(q.x);
        t = t.min(q.y);
        r = r.max(q.x);
        b = b.max(q.y);
    }
    (l.is_finite() && (r > l || b > t)).then(|| fp_geom::Rect::new(l, t, r, b))
}

/// The coordinate tag (*View ▸ Coordinate Tag*): light grey box with rounded corners and dark
/// text, its top-left corner at `at` (re 58 §4).
fn pointer_tag_box(painter: &egui::Painter, theme: &Theme, at: Pos2, text: &str) {
    let ink = Color32::from_gray(0x20);
    let galley = painter.layout_no_wrap(text.to_string(), theme.small_font(), ink);
    let r = Rect::from_min_size(at, galley.size() + vec2(6.0, 2.0));
    painter.rect_filled(r, 3.0, Color32::from_gray(0xd4));
    painter.galley(at + vec2(3.0, 1.0), galley, ink);
}

/// Eigener Mauszeiger über der Leinwand (Systemzeiger ausgeblendet).
fn paint_cursor(ui: &Ui, st: &mut AppState, area: Rect, resp: &egui::Response) {
    let Some(p) = ui.ctx().pointer_hover_pos().filter(|p| area.contains(*p)) else {
        return;
    };
    // inactive window: the system cursor only (FB-18, TW-199)
    if !cursors::may_paint(ui.ctx()) {
        return;
    }
    let busy = st
        .session
        .active_id()
        .and_then(|id| st.canvases.get(&id))
        .is_some_and(|c| c.pointer_down || c.panning || c.guide_move.is_some());
    if !(resp.hovered() || busy) {
        return;
    }
    let cursor = match st.session.active_id().and_then(|id| st.canvases.get(&id)) {
        Some(c) if c.panning => Cursor::Grabbing,
        Some(c) if c.guide_move.is_some() => {
            if c.guide_move.is_some_and(|g| g.0) {
                Cursor::GuideH
            } else {
                Cursor::GuideV
            }
        }
        // Strg+Leertaste = vorübergehend Zoom (mit Alt: verkleinern)
        _ if st.space_down && ui.input(|i| i.modifiers.command) => {
            if ui.input(|i| i.modifiers.alt) {
                Cursor::ZoomOut
            } else {
                Cursor::ZoomIn
            }
        }
        _ if st.space_down => Cursor::Hand,
        _ => st.tool_cursor,
    };
    let theme = Theme::current(ui.ctx());
    let precise = st.session.prefs.get_bool("UsePrecisionCursors", false);
    cursors::hide_system_cursor(ui.ctx());
    let painter = ui.ctx().layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        Id::new("fp-canvas-cursor"),
    ));
    cursors::paint(&painter, p, cursor, &theme.colors, precise);
    st.painted_cursor = Some(p);
}

/// Punkte (logische Pixel) eines Rad-Ereignisses, die einer Rastung entsprechen (Präzisions-Touchpads
/// und hochauflösende Räder liefern Punkte statt Zeilen; Windows: 120 Einheiten je Rastung).
const POINTS_PER_NOTCH: f32 = 100.0;

/// Rastungen eines Rad-Ereignisses.
pub(crate) fn wheel_notches(unit: egui::MouseWheelUnit, dy: f32) -> f32 {
    match unit {
        // eine Rastung je Zeilen-/Seiten-Ereignis, egal wie viele Zeilen Windows meldet
        egui::MouseWheelUnit::Line | egui::MouseWheelUnit::Page => {
            if dy == 0.0 {
                0.0
            } else {
                dy.signum()
            }
        }
        egui::MouseWheelUnit::Point => dy / POINTS_PER_NOTCH,
    }
}

/// Höchstens eine Stufe je Ereignis aus dem Akkumulator nehmen (Richtungswechsel verwirft den Rest).
pub(crate) fn take_step(acc: &mut f32) -> i32 {
    if *acc >= 1.0 {
        *acc = 0.0;
        1
    } else if *acc <= -1.0 {
        *acc = 0.0;
        -1
    } else {
        0
    }
}

/// Hilfslinie unter dem Bildschirmpunkt (waagerecht?, Index) – höchstens 3 px entfernt.
/// Lage einer Hilfslinie der aktuellen Seite (Dokumentpixel).
fn guide_position(d: &fp_engine::DocumentState, horizontal: bool, index: usize) -> Option<f32> {
    let g = d.doc.pages.get(d.page)?.root.guides.as_ref()?;
    if horizontal {
        g.horizontal.get(index).copied()
    } else {
        g.vertical.get(index).copied()
    }
}

/// Dialog „Hilfslinie verschieben“ (Original-Dialog 5995: „Position:“, OK, Abbrechen) für den
/// Doppelklick auf eine Hilfslinie; OK ohne Änderung legt keinen Schritt an (CT-B-04).
pub fn move_guide_dialog(
    horizontal: bool,
    index: usize,
    position: f32,
) -> crate::dialogs::form::FormDialog {
    use crate::dialogs::form::{Field, FormDialog};
    let t = |k: &str| fp_i18n::t(k);
    FormDialog::new(
        "5995",
        &t("cmd-guide-move"),
        "guide.move",
        vec![Field::number(
            "position",
            &t("dlg-lbl-guide-position"),
            position.round() as f64,
            -100_000.0,
            100_000.0,
        )],
    )
    .with_base(
        json!({"orientation": if horizontal { "horizontal" } else { "vertical" }, "index": index}),
    )
    .with_width(220.0)
    .with_skip_unchanged()
}

fn guide_at(d: &fp_engine::DocumentState, area: Rect, p: Pos2) -> Option<(bool, usize)> {
    if !d.view.show_guides || d.view.lock_guides {
        return None;
    }
    let g = d.doc.pages.get(d.page)?.root.guides.as_ref()?;
    let sp = |x: f64, y: f64| to_screen(&d.view, area.min, x, y);
    let mut best: Option<(f32, (bool, usize))> = None;
    for (i, &x) in g.vertical.iter().enumerate() {
        let dx = (sp(x as f64, 0.0).x - p.x).abs();
        if dx <= 3.0 && best.is_none_or(|(b, _)| dx < b) {
            best = Some((dx, (false, i)));
        }
    }
    for (i, &y) in g.horizontal.iter().enumerate() {
        let dy = (sp(0.0, y as f64).y - p.y).abs();
        if dy <= 3.0 && best.is_none_or(|(b, _)| dy < b) {
            best = Some((dy, (true, i)));
        }
    }
    best.map(|(_, g)| g)
}

/// Antwort des Werkzeugs auswerten: Cursor, Statuszeile, Zwischenablage.
fn tool_result(
    ctx: &egui::Context,
    st: &mut AppState,
    r: fp_engine::Result<fp_engine::tools::ToolResponse>,
) {
    match r {
        Ok(r) => {
            st.tool_cursor = r.cursor;
            if let Some(s) = r.status {
                st.push(Action::Status(s));
            }
            if let Some(c) = r.clipboard {
                ctx.copy_text(c);
            }
            if let Some(req) = r.ui_request {
                ui_request(st, &req);
                // die Eingabetaste, die den Vorgang übernommen hat, darf im eben geöffneten Dialog
                // nicht gleich die Vorgabeschaltfläche auslösen
                ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
            }
        }
        Err(e) => st.push(Action::Status(e.to_string())),
    }
}

/// Anfragen des Werkzeugs an die Oberfläche (Dialoge).
pub fn ui_request(st: &mut AppState, req: &fp_engine::tools::Invocation) {
    // KI-Auswahl (Agent KI): Rechnung im Hintergrund, danach der Befehl
    if crate::dialogs::models::handles(&req.command) {
        crate::dialogs::models::request(st, req);
        return;
    }
    if req.command == fp_engine::tools::pathedit::PRIMITIVE_WARNING {
        crate::dialogs::warning::path_tools_on_primitive(st);
        return;
    }
    if req.command == "file.exportArea" {
        let rect = crate::canvas::parse_rect(&req.params["rect"]);
        // Wie im Original öffnet der Doppelklick die **Bildvorschau** mit Register „Datei“ und
        // gesetztem „Exportbereich“ (X/Y/B/H des Rahmens, PAR 08/20)
        if let (Some(r), Some(mut p)) = (rect, crate::dialogs::image_preview::ImagePreview::new(st))
        {
            let origin = st
                .session
                .active()
                .and_then(|d| d.doc.pages.get(d.page).map(|pg| pg.canvas.origin))
                .unwrap_or_default();
            let r = r.normalized();
            let area = [
                (r.left - origin.x).round() as i32,
                (r.top - origin.y).round() as i32,
                (r.right - origin.x).round() as i32,
                (r.bottom - origin.y).round() as i32,
            ];
            for pane in p.panes.iter_mut() {
                pane.settings.crop = Some(area);
            }
            p.tab = 1;
            st.dialogs.preview = Some(p);
            return;
        }
        if let (Some(id), Some(r)) = (st.session.active_id(), rect) {
            st.canvases.entry(id).or_default().export_area = Some(r);
        }
        if !crate::dialogs::open_for_command(st, "file.export") {
            st.push(Action::Status(fp_i18n::t("ui-export-area-set")));
        }
    }
}

/// Rechteck aus `{left, top, right, bottom}`.
pub(crate) fn parse_rect(v: &serde_json::Value) -> Option<fp_geom::Rect> {
    let f = |k: &str| v.get(k).and_then(|x| x.as_f64()).map(|x| x as f32);
    Some(fp_geom::Rect::new(
        f("left")?,
        f("top")?,
        f("right")?,
        f("bottom")?,
    ))
}

/// Merker: der laufende Klick begann, während ein Popup offen war ([`note_popup_press`]).
const POPUP_PRESS: &str = "fp-popup-press";

/// Zu Beginn jedes Bildes (vor dem Zeichnen, also vor dem Schließen der Popups): Drückt der
/// Anwender eine Maustaste, während ein Popup offen ist, gehört der ganze Klick dem Popup (es
/// schließt) und nicht der Leinwand – wie im Original (TW-52: Klick neben den Popup-Schieber auf
/// leere Leinwand hebt die Auswahl nicht auf).
pub fn note_popup_press(ctx: &egui::Context) {
    let (pressed, down) = ctx.input(|i| (i.pointer.any_pressed(), i.pointer.any_down()));
    let id = Id::new(POPUP_PRESS);
    if pressed
        && (egui::Popup::is_any_open(ctx)
            || crate::inspector::popups::fill_popup_open(ctx)
            || crate::inspector::color_picker::any_open(ctx))
    {
        ctx.data_mut(|d| d.insert_temp(id, true));
    } else if pressed || !down {
        ctx.data_mut(|d| d.remove::<bool>(id));
    }
}

/// Gehört der laufende Klick einem (gerade geschlossenen) Popup?
pub fn popup_press(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(Id::new(POPUP_PRESS)))
        .unwrap_or(false)
}

/// Importieren mit Platzierungszeiger (CT-C-05, re\12 §1.5): nach der Dateiwahl wartet die Datei,
/// bis ein Klick ihre linke obere Ecke setzt (Originalgröße) oder ein Aufziehen das Zielrechteck
/// bestimmt (K-7, re8: unter 10 Bildschirmpixeln Bewegung = Klick; Rechteck seitentreu zu den
/// sichtbaren Pixeln, rechnet `file.import {placeFrom, placeTo}`). Esc bricht ab, ohne etwas einzufügen.
#[derive(Clone, Debug, PartialEq)]
pub struct PlaceImport {
    pub path: std::path::PathBuf,
    /// Vektordatei-Optionen (TW-43), gehen als `vector` an `file.import`.
    pub vector: Option<serde_json::Value>,
    /// Druckpunkt (Bildschirm, Dokument), solange die Maustaste gehalten wird.
    pub start: Option<(Pos2, Point)>,
    /// Zug erkannt (|dx| oder |dy| ≥ 10 Bildschirmpixel); bleibt gesetzt bis Maus-auf.
    pub dragged: bool,
    /// Sichtbare Grenzen der Quelle (K-9), beim Start des Platzierungszeigers einmal gelesen; die
    /// Vorschau zeigt damit das seitentreue Rechteck des Imports (sonst das rohe Zugrechteck).
    pub visible: Option<fp_geom::Rect>,
    /// Zuletzt gezeigtes Vorschaurechteck (Dokument), solange gezogen wird.
    pub preview: Option<fp_geom::Rect>,
}

/// Datei zum Platzieren vormerken (Datei ▸ Importieren nach der Dateiwahl).
pub fn begin_place_import(st: &mut AppState, path: std::path::PathBuf) {
    begin_place_import_with(st, path, None);
}

/// Wie [`begin_place_import`], mit den Vektordatei-Optionen aus dem Dialog (TW-43).
pub fn begin_place_import_with(
    st: &mut AppState,
    path: std::path::PathBuf,
    vector: Option<serde_json::Value>,
) {
    // K-9 (re 38 Modus 1): Quelle einmal laden, sichtbare Grenzen für die Vorschau merken
    let visible = fp_engine::commands::import_visible_bounds(&path, vector.as_ref());
    st.place_import = Some(PlaceImport {
        path,
        vector,
        start: None,
        dragged: false,
        visible,
        preview: None,
    });
    st.tool_cursor = Cursor::Crosshair;
    st.push(Action::Status(fp_i18n::t("ui-import-place-hint")));
}

fn place_import_input(
    ui: &Ui,
    st: &mut AppState,
    area: Rect,
    view: &fp_engine::ViewState,
    pointer: Option<Pos2>,
    buttons: (bool, bool, bool),
    free: bool,
) {
    let (pressed, down, released) = buttons;
    let to_doc =
        |p: Pos2| view.screen_to_doc((p.x - area.left()) as f64, (p.y - area.top()) as f64);
    st.tool_cursor = Cursor::Crosshair;
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) || st.session.active().is_none() {
        st.place_import = None;
        st.status = None;
        return;
    }
    let Some(pl) = st.place_import.as_mut() else {
        return;
    };
    if pl.start.is_none() {
        if pressed
            && free
            && let Some(p) = pointer.filter(|p| area.contains(*p))
        {
            pl.start = Some((p, to_doc(p)));
        }
        return;
    }
    let Some((s0, d0)) = pl.start else { return };
    let cur = pointer.unwrap_or(s0);
    // Klickschwelle 10 Bildschirmpixel je Achse (re8 §2.1); Umschalt/Alt wirken nicht
    if (cur.x - s0.x).abs() >= 10.0 || (cur.y - s0.y).abs() >= 10.0 {
        pl.dragged = true;
    }
    let dragged = pl.dragged;
    if down && !released {
        // Vorschau: das Rechteck, das der Import belegen wird (K-9: seitentreu zu den sichtbaren
        // Pixeln, umschließt den Zeiger; ohne bekannte Grenzen das rohe Zugrechteck)
        if dragged {
            let r = fp_engine::commands::place_preview_rect(d0, to_doc(cur), pl.visible.as_ref());
            pl.preview = Some(r);
            let to_screen = |x: f32, y: f32| {
                let (sx, sy) = view.doc_to_screen(Point::new(x, y));
                Pos2::new(area.left() + sx as f32, area.top() + sy as f32)
            };
            let painter = ui.painter().with_clip_rect(area);
            painter.rect_stroke(
                Rect::from_two_pos(to_screen(r.left, r.top), to_screen(r.right, r.bottom)),
                0.0,
                Stroke::new(1.0, st.theme.colors.focus),
                StrokeKind::Inside,
            );
        }
        return;
    }
    let path = pl.path.to_string_lossy().to_string();
    let vector = pl.vector.take();
    st.place_import = None;
    st.status = None;
    let d1 = to_doc(cur);
    let params = if !dragged {
        json!({"path": path, "placeFrom": [d0.x, d0.y]})
    } else {
        json!({"path": path, "placeFrom": [d0.x, d0.y], "placeTo": [d1.x, d1.y]})
    };
    let mut params = params;
    if let Some(v) = vector {
        params["vector"] = v;
    }
    st.run("file.import", params);
}

fn input(ui: &mut Ui, st: &mut AppState, area: Rect, resp: &egui::Response) {
    let Some(d) = st.session.active() else { return };
    let id = d.id;
    let view = d.view.clone();
    let space = st.space_down;
    let (
        pointer,
        wheel,
        zoom_delta,
        mods_now,
        primary_pressed,
        primary_released,
        middle_down,
        primary_down,
        time,
    ) = ui.input(|i| {
        (
            i.pointer.interact_pos(),
            i.smooth_scroll_delta,
            i.zoom_delta(),
            mods(i, space),
            i.pointer.primary_pressed(),
            i.pointer.primary_released(),
            i.pointer.middle_down(),
            i.pointer.primary_down(),
            i.time,
        )
    });
    // Leertaste wird wie `st.space_down` nur außerhalb von Texteingaben gezählt
    let space_keys = !ui.ctx().text_edit_focused() && !st.session.tool_is_editing_text();
    let (raw_seq, end_mods) = ui.input(|i| {
        let mut v = Vec::new();
        for e in &i.events {
            match e {
                egui::Event::ModifiersChanged(m) => v.push(RawPtr::Mods(*m)),
                egui::Event::Key {
                    key: egui::Key::Space,
                    pressed,
                    repeat: false,
                    ..
                } if space_keys => v.push(RawPtr::Space(*pressed)),
                egui::Event::WindowFocused(false) => v.push(RawPtr::Mods(egui::Modifiers::NONE)),
                egui::Event::PointerMoved(p) => v.push(RawPtr::Move(*p)),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers,
                } => v.push(RawPtr::Button(*pos, *pressed, *modifiers)),
                _ => {}
            }
        }
        (v, i.modifiers)
    });
    let hovered = resp.hovered() || resp.dragged();
    let to_doc =
        |p: Pos2| view.screen_to_doc((p.x - area.left()) as f64, (p.y - area.top()) as f64);
    if let Some(p) = pointer.filter(|p| area.contains(*p)) {
        st.pointer_doc = Some(to_doc(p));
    }
    // Zoom (Strg+Mausrad, Zwei-Finger-Zoom): je volle Rastung genau eine der 17 Stufen um den
    // Mauszeiger. Ausgewertet werden die rohen Ereignisse dieses Frames (nicht `zoom_delta`, das
    // egui über mehrere Frames glättet – jede geglättete Teilmenge war früher eine ganze Stufe).
    let _ = zoom_delta;
    type Wheel = (egui::MouseWheelUnit, egui::Vec2, bool, bool);
    let (wheel_events, pinch, line_px): (Vec<Wheel>, f32, f32) = ui.input(|i| {
        let mut w = Vec::new();
        let mut z = 1.0f32;
        for e in &i.events {
            match e {
                egui::Event::MouseWheel {
                    unit,
                    delta,
                    modifiers,
                    ..
                } => w.push((
                    *unit,
                    *delta,
                    modifiers.ctrl || modifiers.command,
                    modifiers.shift,
                )),
                egui::Event::Zoom(f) => z *= *f,
                _ => {}
            }
        }
        // Rastung = 52 Bildschirmpixel wie im Original (UX-A L9/L11)
        (w, z, 52.0)
    });
    let zoom_wheel = wheel_events.iter().any(|w| w.2) || mods_now.ctrl;
    if hovered {
        let anchor = pointer.map(|p| ((p.x - area.left()) as f64, (p.y - area.top()) as f64));
        let mut steps: i32 = 0;
        {
            let cc = st.canvases.entry(id).or_default();
            for (unit, delta, ctrl, _) in &wheel_events {
                if !*ctrl {
                    continue;
                }
                let n = wheel_notches(*unit, delta.y);
                if n * cc.wheel_acc < 0.0 {
                    cc.wheel_acc = 0.0;
                }
                cc.wheel_acc += n;
                steps += take_step(&mut cc.wheel_acc);
            }
            if pinch != 1.0 {
                // Zwei-Finger-Zoom: eine Stufe je Faktor 1,25 (Abstand der Zoomstufen)
                cc.pinch_acc += pinch.ln() / 1.25f32.ln();
                steps += take_step(&mut cc.pinch_acc);
            }
        }
        if let Some(d) = st.session.active_mut() {
            for _ in 0..steps.abs() {
                if steps > 0 {
                    d.view.zoom_in(anchor)
                } else {
                    d.view.zoom_out(anchor)
                }
            }
        }
        // Bildlauf aus den Rohereignissen (sofort, ohne die Glättung von egui; `smooth_scroll_delta`
        // kam in der App nicht zuverlässig an): Zeile = Zeilenhöhe von egui, Seite = Sichthöhe,
        // Punkte direkt; Umschalt = waagerecht
        let _ = wheel;
        let page_h = area.height();
        let (mut dx, mut dy) = (0.0f32, 0.0f32);
        for (unit, delta, ctrl, shift) in &wheel_events {
            if *ctrl {
                continue;
            }
            let k = match unit {
                egui::MouseWheelUnit::Point => 1.0,
                egui::MouseWheelUnit::Line => line_px,
                egui::MouseWheelUnit::Page => page_h,
            };
            // Umschalt+Rad tut im Original nichts (UX-A L10); Alt+Rad rollt wie ohne Taste
            if *shift {
                continue;
            }
            let v = *delta * k;
            dx += v.x;
            dy += v.y;
        }
        if (dx != 0.0 || dy != 0.0)
            && let Some(d) = st.session.active_mut()
        {
            d.view.pan(dx as f64, dy as f64);
        }
        let _ = zoom_wheel;
    }
    let editing = st.session.tool_is_editing_text();
    // Neue Zeigerereignisse nur, wenn die Leinwand selbst unter dem Zeiger liegt (kein Menü, Popup
    // oder Dialog darüber); ein laufendes Ziehen darf immer zu Ende gehen.
    // Ein Klick, der ein offenes Popup (Popup-Schieber, Kombiliste, Menü) schließt, erreicht die
    // Leinwand nicht: das Original verbraucht ihn, die Auswahl bleibt (TW-52)
    let free = resp.hovered() && !st.dialogs.any_open() && !popup_press(ui.ctx());
    let pointer_tool = matches!(
        st.session.tools.active,
        fp_engine::ToolId::Pointer
            | fp_engine::ToolId::Subselection
            | fp_engine::ToolId::MaskElementSelect
            | fp_engine::ToolId::SelectBehind
    );
    let guide_hit = match (pointer, st.session.active()) {
        (Some(p), Some(d)) if free && pointer_tool && area.contains(p) => guide_at(d, area, p),
        _ => None,
    };
    // Leertaste am Anfang dieses Bildes: vor dem ersten Leertasten-Ereignis galt dessen Gegenteil
    let space_start = raw_seq
        .iter()
        .find_map(|r| {
            if let RawPtr::Space(d) = r {
                Some(!*d)
            } else {
                None
            }
        })
        .unwrap_or(space);
    // Vorübergehender Zoom wie im Original (UX-A L7): Strg+Leertaste+Klick vergrößert um den
    // Klickpunkt, Strg+Alt+Leertaste+Klick verkleinert. Ausgewertet je Rohereignis: Strg,
    // Leertaste und Maustaste können im selben Bild ankommen (und schon wieder losgelassen sein),
    // das Original zoomt auch dann (PARITAET_A L7)
    let pressed_down = st.canvases.get(&id).is_some_and(|c| c.pointer_down);
    if free && !pressed_down {
        let mut sp = space_start;
        let mut zooms: Vec<(Pos2, bool)> = Vec::new();
        for r in &raw_seq {
            match *r {
                RawPtr::Space(d) => sp = d,
                RawPtr::Button(q, true, m) if sp && (m.ctrl || m.command) && area.contains(q) => {
                    zooms.push((q, m.alt))
                }
                _ => {}
            }
        }
        if let Some(d) = st.session.active_mut() {
            for (q, out) in zooms {
                let anchor = Some(((q.x - area.left()) as f64, (q.y - area.top()) as f64));
                if out {
                    d.view.zoom_out(anchor)
                } else {
                    d.view.zoom_in(anchor)
                }
            }
        }
    }
    if space && mods_now.ctrl && free {
        st.tool_cursor = if mods_now.alt {
            Cursor::ZoomOut
        } else {
            Cursor::ZoomIn
        };
        return;
    }
    let cc = st.canvases.entry(id).or_default();
    // Hand: Leertaste oder mittlere Taste
    // Leertaste schwenkt nur, wenn sie vor dem Mausdruck gehalten wurde; während eines laufenden
    // Werkzeugvorgangs (Aufziehen) geht sie als Modifikator an das Werkzeug (Form verschieben)
    if (space && !cc.pointer_down && (resp.dragged_by(egui::PointerButton::Primary) || cc.panning))
        || (middle_down && hovered)
        || resp.dragged_by(egui::PointerButton::Middle)
    {
        cc.panning = primary_down || middle_down;
        let delta = resp.drag_delta();
        if let Some(d) = st.session.active_mut() {
            d.view.pan(delta.x as f64, delta.y as f64);
        }
        return;
    }
    cc.panning = false;
    // Objekte auf den Reiter eines anderen Dokuments ziehen: Dokument wird aktiv, Ablegen auf der
    // Leinwand kopiert sie hinein (wie im Original)
    let ctx = ui.ctx().clone();
    let carry: Option<Carry> = ctx.data(|d| d.get_temp(Id::new(CARRY_ID)));
    if let Some(c) = carry {
        if primary_released || !primary_down {
            ctx.data_mut(|d| d.remove::<Carry>(Id::new(CARRY_ID)));
            if c.from != id
                && let Some(p) = pointer.filter(|p| area.contains(*p))
            {
                drop_carry(st, &c, to_doc(p));
            }
        }
        return;
    }
    // Importieren: Platzierungszeiger statt Werkzeug (CT-C-05)
    if st.place_import.is_some() {
        place_import_input(
            ui,
            st,
            area,
            &view,
            pointer,
            (primary_pressed, primary_down, primary_released),
            free,
        );
        return;
    }
    if cc.pointer_down
        && primary_down
        && let Some(p) = pointer.filter(|p| !area.contains(*p))
        && cc
            .feedback
            .iter()
            .any(|o| matches!(o, Overlay::Ghost { .. }))
    {
        let tabs: Vec<(fp_engine::DocId, Rect)> = ctx
            .data(|d| d.get_temp(Id::new("lw-doc-tabs")))
            .unwrap_or_default();
        if let Some((other, _)) = tabs.iter().find(|(d, r)| *d != id && r.contains(p)) {
            let grab = cc.press_pos.unwrap_or_default();
            cc.pointer_down = false;
            let elements: Vec<std::sync::Arc<fp_model::Element>> = st
                .session
                .active()
                .map(|d| {
                    d.selection
                        .elements
                        .iter()
                        .filter_map(|e| fp_engine::docops::element_on_page(&d.doc, d.page, *e))
                        .collect()
                })
                .unwrap_or_default();
            // Ziehen im Quelldokument abbrechen (nichts verschieben)
            let r = st
                .session
                .tool_event(&ToolEvent::key(fp_engine::tools::ToolKey::Escape));
            tool_result(&ctx, st, r);
            if !elements.is_empty() {
                ctx.data_mut(|d| {
                    d.insert_temp(
                        Id::new(CARRY_ID),
                        Carry {
                            from: id,
                            elements,
                            grab,
                        },
                    )
                });
                st.push(Action::Activate(*other));
            }
            return;
        }
    }
    let Some(cc) = st.canvases.get_mut(&id) else {
        return;
    };
    // Hilfslinie mit dem Zeiger verschieben; ins Lineal (aus der Leinwand) ziehen = löschen
    if let Some((horizontal, index)) = cc.guide_move {
        if primary_released || !primary_down {
            cc.guide_move = None;
            let press = cc.guide_press.take();
            let o = if horizontal { "horizontal" } else { "vertical" };
            match pointer.filter(|p| area.contains(*p)) {
                Some(p) => {
                    // Klick ohne Ziehen: kein Verlaufsschritt; zweiter Klick binnen 500 ms und 4 px auf
                    // dieselbe Hilfslinie öffnet „Hilfslinie verschieben“, auch ohne sichtbare Lineale
                    // (CT-B-04, re 33 §8 b)
                    let clicked = press.is_some_and(|q| q.distance(p) <= 4.0);
                    let now = ctx.input(|i| i.time);
                    let double = clicked
                        && cc.guide_click.is_some_and(|(t, q, g)| {
                            g == (horizontal, index) && now - t <= 0.5 && q.distance(p) <= 4.0
                        });
                    cc.guide_click = if clicked && !double {
                        Some((now, p, (horizontal, index)))
                    } else {
                        None
                    };
                    let current = st
                        .session
                        .active()
                        .and_then(|d| guide_position(d, horizontal, index));
                    if double {
                        if let Some(cur) = current {
                            st.dialogs.form = Some(move_guide_dialog(horizontal, index, cur));
                        }
                    } else if !clicked {
                        let dp = to_doc(p);
                        let pos = if horizontal {
                            dp.y.round()
                        } else {
                            dp.x.round()
                        };
                        // unveränderte Lage: kein leerer Schritt „Hilfslinie verschieben“
                        if current != Some(pos) {
                            st.run(
                                "guide.move",
                                json!({"orientation": o, "index": index, "position": pos}),
                            );
                        }
                    }
                }
                None => st.run("guide.move", json!({"orientation": o, "index": index})),
            }
        }
        return;
    }
    if let (Some(g), Some(p)) = (guide_hit, pointer)
        && primary_pressed
        && area.contains(p)
        && !space
    {
        cc.guide_move = Some(g);
        cc.guide_press = Some(p);
        return;
    }
    if guide_hit.is_some() && !cc.pointer_down {
        st.tool_cursor = if guide_hit.is_some_and(|g| g.0) {
            Cursor::GuideH
        } else {
            Cursor::GuideV
        };
        return;
    }
    // Werkzeug-Ereignisse
    let t_ms = (time * 1000.0) as u64;
    let mut events: Vec<ToolEvent> = Vec::new();
    let mk = |kind: PointerKind, p: Point| ToolEvent {
        input: ToolInput::Pointer { kind, pos: p },
        mods: mods_now,
        pressure: 1.0,
        time_ms: t_ms,
    };
    let caret = |index: usize, extend: bool| ToolEvent {
        input: ToolInput::TextCaret { index, extend },
        mods: mods_now,
        pressure: 1.0,
        time_ms: t_ms,
    };
    if let Some(p) = pointer {
        let dp = to_doc(p);
        // Klick/Ziehen im bearbeiteten Text: Caret setzen bzw. markieren (Layout von fp-text)
        let tol = (4.0 / view.zoom.max(1e-6)) as f32;
        let in_text = editing && cc.text.as_ref().is_some_and(|lc| lc.contains(dp, tol));
        if primary_pressed && free && area.contains(p) && !space && in_text {
            // click count inside the open editor (re 47 §2.2): 2nd click selects the word, 3rd the
            // paragraph; the click that opened the editor counts as the first one
            let near = |t: f64, q: Pos2| time - t <= 0.5 && q.distance(p) <= 4.0;
            let clicks = match (cc.text_clicks, cc.last_click) {
                (Some((t, q, n)), _) if near(t, q) => n.saturating_add(1),
                (_, Some((t, q))) if near(t, q) => 2,
                _ => 1,
            };
            cc.text_clicks = Some((time, p, clicks));
            cc.last_click = None;
            if let Some(lc) = cc.text.as_ref() {
                events.push(caret(lc.hit(dp), mods_now.shift && clicks == 1));
            }
            if clicks == 2 {
                events.push(mk(PointerKind::DoubleClick, dp));
            } else if clicks >= 3 {
                events.push(ToolEvent {
                    input: ToolInput::Edit(fp_engine::tools::EditOp::SelectParagraph),
                    mods: mods_now,
                    pressure: 1.0,
                    time_ms: t_ms,
                });
            }
            cc.text_drag = true;
            cc.last_pos = Some(dp);
        } else if cc.text_drag {
            if primary_down && cc.last_pos != Some(dp) {
                if let Some(lc) = cc.text.as_ref() {
                    events.push(caret(lc.hit(dp), true));
                }
                cc.last_pos = Some(dp);
            }
            if !primary_down || primary_released {
                cc.text_drag = false;
            }
        } else {
            // Rohe Ereignisse dieses Bildes der Reihe nach (UX-A L3/L15/L16/L24): Bei echter,
            // schneller Eingabe kommen Umschalttasten, Drücken, Bewegen und Loslassen (auch ein
            // ganzer Doppelklick) im selben Bild an; der Bildzustand von egui (`primary_pressed`,
            // `modifiers` am Bildende, `interact_pos`) verliert dann Lage und Umschalttasten.
            let mut cur = cc.frame_mods.unwrap_or(end_mods);
            let mut cur_space = space_start;
            let tm = |m: egui::Modifiers, space: bool| Modifiers {
                shift: m.shift,
                alt: m.alt,
                ctrl: m.ctrl || m.command,
                space,
                caps: false,
            };
            // Zeitstempel je Ereignis: echte Mausbewegungen kommen gesammelt (mehrere je Bild);
            // mit derselben Bildzeit hätten sie keinen Zeitabstand (Pinsel: Geschwindigkeit)
            let ptr_count = raw_seq
                .iter()
                .filter(|r| matches!(r, RawPtr::Move(_) | RawPtr::Button(..)))
                .count();
            let times = event_times_ms(cc.last_input_time, time, ptr_count);
            let mut ptr_idx = 0usize;
            let mut had_button = false;
            for r in &raw_seq {
                let t_ev = if matches!(r, RawPtr::Move(_) | RawPtr::Button(..)) {
                    ptr_idx += 1;
                    times.get(ptr_idx - 1).copied().unwrap_or(t_ms)
                } else {
                    t_ms
                };
                let at = |kind: PointerKind, q: Point, m: Modifiers| ToolEvent {
                    input: ToolInput::Pointer { kind, pos: q },
                    mods: m,
                    pressure: 1.0,
                    time_ms: t_ev,
                };
                match *r {
                    RawPtr::Mods(m) => cur = m,
                    RawPtr::Space(d) => cur_space = d,
                    RawPtr::Move(q) => {
                        let dq = to_doc(q);
                        if (cc.pointer_down || (free && area.contains(q)))
                            && cc.last_pos != Some(dq)
                        {
                            cc.last_pos = Some(dq);
                            events.push(at(PointerKind::Move, dq, tm(cur, cur_space)));
                        }
                    }
                    RawPtr::Button(q, true, m) => {
                        cur = m;
                        had_button = true;
                        // gedrückte Leertaste: Hand bzw. (mit Strg) vorübergehender Zoom, kein Werkzeugklick
                        if !cc.pointer_down && free && area.contains(q) && !cur_space {
                            let dq = to_doc(q);
                            cc.pointer_down = true;
                            cc.press_pos = Some(dq);
                            cc.last_pos = Some(dq);
                            cc.click_press = Some(q);
                            events.push(at(PointerKind::Down, dq, tm(cur, cur_space)));
                        }
                    }
                    RawPtr::Button(q, false, m) => {
                        cur = m;
                        had_button = true;
                        if cc.pointer_down {
                            let dq = to_doc(q);
                            cc.pointer_down = false;
                            cc.last_pos = Some(dq);
                            events.push(at(PointerKind::Up, dq, tm(cur, cur_space)));
                            // Doppelklick selbst erkennen (Windows: zweiter Klick binnen 500 ms,
                            // höchstens 4 px entfernt), je Klick – auch zwei Klicks in einem Bild
                            if cc.click_press.take().is_some_and(|c| c.distance(q) <= 4.0) {
                                let double = cc
                                    .last_click
                                    .is_some_and(|(t, l)| time - t <= 0.5 && l.distance(q) <= 4.0);
                                cc.last_click = if double { None } else { Some((time, q)) };
                                if double {
                                    events.push(at(
                                        PointerKind::DoubleClick,
                                        dq,
                                        tm(cur, cur_space),
                                    ));
                                }
                            } else {
                                cc.last_click = None;
                            }
                            // danach Überfahren an derselben Stelle: Werkzeug meldet wieder seinen Zeiger
                            if free && area.contains(q) {
                                events.push(at(PointerKind::Move, dq, tm(cur, cur_space)));
                            }
                        }
                    }
                }
            }
            // Loslassen ohne Ereignis (außerhalb des Fensters, Fokusverlust)
            if cc.pointer_down && !had_button && (primary_released || !primary_down) {
                cc.pointer_down = false;
                cc.click_press = None;
                cc.last_pos = Some(dp);
                events.push(mk(PointerKind::Up, dp));
            }
            // FB-72: Shift/Alt/Ctrl pressed or released over the canvas without mouse motion:
            // one move at the same place re-evaluates the tool's hover cursor (selection badges,
            // Alt-duplicate, Pen variants, ...); only on a change, never while a press is held
            let now_mods = tm(cur, cur_space);
            let keys = |m: Modifiers| (m.shift, m.alt, m.ctrl);
            if events.is_empty()
                && !cc.pointer_down
                && free
                && area.contains(p)
                && cc.tool_mods.is_some_and(|m| keys(m) != keys(now_mods))
            {
                events.push(ToolEvent {
                    input: ToolInput::Pointer {
                        kind: PointerKind::Move,
                        pos: dp,
                    },
                    mods: now_mods,
                    pressure: 1.0,
                    time_ms: t_ms,
                });
            }
            if let Some(last) = events.last() {
                cc.tool_mods = Some(last.mods);
            }
        }
    }
    if let Some(cc) = st.canvases.get_mut(&id) {
        cc.frame_mods = Some(end_mods);
        cc.last_input_time = Some(time);
    }
    let ctx = ui.ctx().clone();
    for ev in events {
        let kind = ev.kind();
        let hit_before = st.session.tool_press_hit();
        let r = st.session.tool_event(&ev);
        if matches!(kind, Some(PointerKind::Down | PointerKind::Up)) {
            log_pointer(st, &ev, hit_before);
        }
        // Double-click on a text opens the editor within this event (on the second press, so
        // keys typed afterwards already reach it); the engine places the caret at the click point
        // (TW-135). No caret is reported later: it would move the caret away from characters
        // typed in between. Only the layout is rebuilt.
        if kind == Some(PointerKind::DoubleClick)
            && r.as_ref()
                .is_ok_and(|r| r.switch_tool == Some(fp_engine::ToolId::Text))
            && let Some(cc) = st.canvases.get_mut(&id)
        {
            cc.text = None;
        }
        tool_result(&ctx, st, r);
    }
}

/// Most entries kept in `AppState::pointer_log`.
pub const POINTER_LOG_MAX: usize = 16;

/// F28 (Q-50): records a canvas press/release for the bridge: active tool, what the press hit
/// (on release: what the press had hit, i.e. the tool state before the release), modifiers and the
/// selection count afterwards.
fn log_pointer(st: &mut AppState, ev: &fp_engine::tools::ToolEvent, hit_before: Option<&str>) {
    let down = ev.kind() == Some(PointerKind::Down);
    let hit = if down {
        st.session.tool_press_hit()
    } else {
        hit_before
    };
    let pos = ev.pos().map(|p| [p.x, p.y]);
    let tool = fp_engine::tools::info(st.session.tools.active).map(|i| i.name);
    let selection = st
        .session
        .active()
        .map(|d| d.selection.elements.len())
        .unwrap_or(0);
    let entry = serde_json::json!({
        "event": if down { "down" } else { "up" },
        "tool": tool,
        "hit": hit,
        "pos": pos,
        "mods": {"alt": ev.mods.alt, "shift": ev.mods.shift, "ctrl": ev.mods.ctrl, "space": ev.mods.space},
        "selection": selection,
    });
    st.pointer_log.push(entry);
    if st.pointer_log.len() > POINTER_LOG_MAX {
        let n = st.pointer_log.len() - POINTER_LOG_MAX;
        st.pointer_log.drain(..n);
    }
}

/// Längste Zeit (s), auf die die Zeigerereignisse eines Bildes verteilt werden (nach einer Pause
/// ohne Bild nicht über die ganze Pause).
const EVENT_SPREAD_MAX: f64 = 0.1;

/// Zeitstempel (ms) für `n` Zeigerereignisse eines Bildes zur Zeit `now` (s): gleichmäßig zwischen
/// dem letzten Bild `prev` (höchstens [`EVENT_SPREAD_MAX`] zurück) und `now`; das letzte Ereignis
/// trägt `now`. Ohne früheres Bild alle `now`.
pub(crate) fn event_times_ms(prev: Option<f64>, now: f64, n: usize) -> Vec<u64> {
    let start = prev
        .filter(|p| *p < now)
        .map_or(now, |p| p.max(now - EVENT_SPREAD_MAX));
    (1..=n)
        .map(|k| ((start + (now - start) * k as f64 / n as f64) * 1000.0).round() as u64)
        .collect()
}

/// 2 × 2-Schachbrett (weiß/transparent, wiederholt, ohne Glättung) für das gepunktete Raster (TW-55).
fn grid_dot_texture(ctx: &egui::Context) -> egui::TextureHandle {
    let key = Id::new("canvas-grid-dot-texture");
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(key)) {
        return t;
    }
    let (w, c) = (Color32::WHITE, Color32::TRANSPARENT);
    let img = egui::ColorImage::new([2, 2], vec![w, c, c, w]);
    let opt = egui::TextureOptions {
        magnification: egui::TextureFilter::Nearest,
        minification: egui::TextureFilter::Nearest,
        wrap_mode: egui::TextureWrapMode::Repeat,
        mipmap_mode: None,
    };
    let t = ctx.load_texture("canvas-grid-dot", img, opt);
    ctx.data_mut(|d| d.insert_temp(key, t.clone()));
    t
}

fn overlays(
    ui: &Ui,
    st: &mut AppState,
    painter: &egui::Painter,
    area: Rect,
    view: &fp_engine::ViewState,
    feedback: &[Overlay],
) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let ctx = ui.ctx().clone();
    let time = ui.input(|i| i.time);
    let Some(d) = st.session.active() else { return };
    let id = d.id;
    let (generation, frame_i) = (d.generation, d.frame);
    let page = d.doc.pages.get(d.page);
    let sp = |x: f64, y: f64| to_screen(view, area.min, x, y);
    let spp = |p: Point| to_screen(view, area.min, p.x as f64, p.y as f64);
    // Raster
    if view.show_grid
        && let Some(pg) = page
        && let Some(pr) = page_rect(&d.doc, d.page)
    {
        // TW-55: ohne Raster in der Datei gilt das Raster der Voreinstellungen (Original: Cyan)
        let g = pg
            .root
            .grid
            .clone()
            .unwrap_or_else(|| st.session.prefs.new_document_grid());
        let (gx, gy) = (g.size.x.max(1.0) as f64, g.size.y.max(1.0) as f64);
        let col = argb(g.color);
        // gepunktet wie im Original: jedes zweite Bildschirmpixel (x + y gerade)
        let ppp = ui.ctx().pixels_per_point();
        let px = 1.0 / ppp;
        let mut mesh = egui::Mesh::with_texture(grid_dot_texture(ui.ctx()).id());
        let mut dotted = |r: Rect| {
            let uv = Rect::from_min_max(
                pos2(r.left() * ppp / 2.0, r.top() * ppp / 2.0),
                pos2(r.right() * ppp / 2.0, r.bottom() * ppp / 2.0),
            );
            mesh.add_rect_with_uv(r, uv, col);
        };
        if gx * view.zoom >= 4.0 {
            let mut x = g.origin.x as f64 + ((pr.left as f64 - g.origin.x as f64) / gx).ceil() * gx;
            let (y0, y1) = (
                sp(0.0, pr.top as f64).y.round(),
                sp(0.0, pr.bottom as f64).y.round(),
            );
            while x <= pr.right as f64 {
                let sx = sp(x, 0.0).x.round();
                dotted(Rect::from_min_max(pos2(sx, y0), pos2(sx + px, y1 + px)));
                x += gx;
            }
        }
        if gy * view.zoom >= 4.0 {
            let mut y = g.origin.y as f64 + ((pr.top as f64 - g.origin.y as f64) / gy).ceil() * gy;
            let (x0, x1) = (
                sp(pr.left as f64, 0.0).x.round(),
                sp(pr.right as f64, 0.0).x.round(),
            );
            while y <= pr.bottom as f64 {
                let sy = sp(0.0, y).y.round();
                dotted(Rect::from_min_max(pos2(x0, sy), pos2(x1 + px, sy + px)));
                y += gy;
            }
        }
        painter.add(egui::Shape::mesh(mesh));
    }
    // Hilfslinien (die gerade verschobene folgt dem Zeiger)
    let moving_guide = st.canvases.get(&id).and_then(|cc| cc.guide_move);
    if view.show_guides
        && let Some(pg) = page
        && let Some(gd) = &pg.root.guides
    {
        let col = gd.color.map(argb).unwrap_or(c.guide_default);
        for (i, &x) in gd.vertical.iter().enumerate() {
            if moving_guide != Some((false, i)) {
                painter.vline(
                    sp(x as f64, 0.0).x.round() + 0.5,
                    area.y_range(),
                    Stroke::new(1.0, col),
                );
            }
        }
        for (i, &y) in gd.horizontal.iter().enumerate() {
            if moving_guide != Some((true, i)) {
                painter.hline(
                    area.x_range(),
                    sp(0.0, y as f64).y.round() + 0.5,
                    Stroke::new(1.0, col),
                );
            }
        }
        if let (Some((horizontal, _)), Some(pp)) = (moving_guide, ui.ctx().pointer_interact_pos()) {
            let dp = view.screen_to_doc((pp.x - area.left()) as f64, (pp.y - area.top()) as f64);
            let (line, label) = if horizontal {
                (sp(0.0, dp.y.round() as f64).y, dp.y.round())
            } else {
                (sp(dp.x.round() as f64, 0.0).x, dp.x.round())
            };
            if horizontal {
                painter.hline(area.x_range(), line.round() + 0.5, Stroke::new(1.0, col));
            } else {
                painter.vline(line.round() + 0.5, area.y_range(), Stroke::new(1.0, col));
            }
            if area.contains(pp) {
                label_at(
                    painter,
                    &theme,
                    pp + vec2(10.0, 10.0),
                    &format!("{label:.0}"),
                );
            }
        }
    }
    // Web-Ebene: Segmente/Hotspots und Segment-Hilfslinien (UX-A O17)
    if !view.hide_slices
        && let Some(pr) = page_rect(&d.doc, d.page)
    {
        let items = web::items(&d.doc, d.page, d.frame);
        if !items.is_empty() {
            let pg = fp_geom::Rect::new(
                pr.left as f32,
                pr.top as f32,
                pr.right as f32,
                pr.bottom as f32,
            );
            let gc = st
                .session
                .prefs
                .get_color("SliceGuideColor", fp_color::Argb(0xffff_4a4a));
            web::paint(painter, &items, pg, view.show_slice_guides, argb(gc), &spp);
        }
    }
    // Auswahl: Rahmen mit Eckgriffen
    // Während des Ziehens wandert der Auswahlrahmen mit der Vorschau (Rechteck → Viereck der Vorschau)
    // (a pixel move keeps the object frame in place, FB-73)
    let moving: Option<(Vec<u64>, fp_geom::Matrix3)> = feedback.iter().find_map(|o| match o {
        Overlay::Ghost {
            ids,
            rect,
            quad,
            pixels: false,
            ..
        } => fp_geom::Matrix3::rect_to_quad(rect, quad).map(|m| (ids.clone(), m)),
        _ => None,
    });
    let mut frames: Vec<[Point; 4]> = Vec::new();
    // Text im Editor: dessen Rahmen zeichnet der Editor aus dem aktuellen Satz (wächst live mit)
    let editing_target = feedback.iter().find_map(|o| match o {
        Overlay::TextEdit(v) => v.target,
        _ => None,
    });
    // Malwerkzeuge (Pinsel, Radiergummi, Weichzeichner & Co., Stempel …): das Original zeigt um die
    // Bitmap, in die gemalt wird, weder Rahmen noch Griffe (FV-11, gemessen)
    let paint_tool = matches!(
        st.session.tools.active,
        fp_engine::ToolId::Brush
            | fp_engine::ToolId::Pencil
            | fp_engine::ToolId::Eraser
            | fp_engine::ToolId::Blur
            | fp_engine::ToolId::Sharpen
            | fp_engine::ToolId::Dodge
            | fp_engine::ToolId::Burn
            | fp_engine::ToolId::Smudge
            | fp_engine::ToolId::RubberStamp
            | fp_engine::ToolId::ReplaceColor
            | fp_engine::ToolId::RedEyeRemoval
    );
    let subsel_outline = matches!(
        st.session.tools.active,
        fp_engine::ToolId::Subselection | fp_engine::ToolId::MaskElementSelect
    );
    // FB-19 (RE spec 60 section 1.3): the Pointer and the Pen show a single selected vector path
    // (free or rectangle primitive) as its outline plus anchor squares (from `Overlay::PathPoints`),
    // not as a bounding box; the path the pen is drawing is shown by the pen's own overlay
    let path_outline = matches!(
        st.session.tools.active,
        fp_engine::ToolId::Pointer | fp_engine::ToolId::Pen
    ) && d.selection.elements.len() == 1;
    let pen_target = feedback.iter().find_map(|o| match o {
        Overlay::PenPath { target } => Some(*target),
        _ => None,
    });
    if view.show_edges {
        // alle gewählten Elemente in einem Durchlauf (PERF-1: eine Suche je ID kostete bei
        // Tausenden gewählten Objekten O(Objekte × Auswahl) je Bild)
        let found = fp_engine::docops::find_many_on_page(&d.doc, d.page, &d.selection.elements);
        let moving_ids: std::collections::HashSet<u64> = moving
            .as_ref()
            .map(|(ids, _)| ids.iter().copied().collect())
            .unwrap_or_default();
        // Rahmen/Griffe ganz außerhalb der Leinwand zeichnet der Painter ohnehin nicht (Clip = `area`):
        // gar nicht erst anlegen (gleiches Bild, weniger Formen bei Tausenden gewählten Objekten)
        let cull = painter.clip_rect().expand(theme.metrics.handle_size + 2.0);
        // viele Objekte: Rahmen und Griffe als ein Mesh (PERF-1, pixelgleich bei ganzzahliger Skalierung)
        let ppp = ui.ctx().pixels_per_point();
        let mut batch = (!theme.glow
            && ppp.fract() == 0.0
            && d.selection.elements.len()
                >= overlay::FRAME_BATCH_MIN.load(std::sync::atomic::Ordering::Relaxed))
        .then(overlay::FrameBatch::default);
        for (eid, path) in d.selection.elements.iter().zip(&found) {
            if editing_target == Some(eid.0) {
                continue;
            }
            let Some(e) = path.as_ref().and_then(|p| d.doc.element_at(p)).cloned() else {
                continue;
            };
            // bitmap mode hides the object handles of the bitmap and draws no frame (RE spec 71
            // §1.2)
            if (paint_tool || d.bitmap_mode) && matches!(e.kind, fp_model::ElementKind::Bitmap(_)) {
                continue;
            }
            // Animationsinstanz: Rahmen nur um das Bild im aktuellen Status (TW-20)
            let Some(b) = anim::frame_bounds(&d.doc, &e, d.frame)
                .or_else(|| fp_engine::docops::element_frame_bounds(&d.doc, &e))
            else {
                continue;
            };
            let is_moving = moving_ids.contains(&eid.0);
            let corners = match moving.as_ref().filter(|_| is_moving) {
                Some((_, m)) => b.corners().map(|p| m.apply(p)),
                None => b.corners(),
            };
            frames.push(corners);
            // FB-06 (re 54 §1, §6): the Subselection tool shows a vector object as its path outline
            // (1 px, real curves through the anchors) instead of a bounding box with corner squares;
            // the anchors and handles come from `Overlay::PathPoints` on top of it
            if (subsel_outline || path_outline)
                && let fp_model::ElementKind::Path(pe) = &e.kind
            {
                if pen_target == Some(eid.0) {
                    continue;
                }
                if let Some(bt) = batch.as_mut() {
                    bt.flush(painter);
                }
                let line = Stroke::new(1.0, c.selection);
                match moving.as_ref().filter(|_| is_moving) {
                    None => paint_path_outline(painter, &pe.contours, &spp, line),
                    // Pointer drag: the outline follows the preview
                    Some((_, m)) if path_outline => {
                        paint_path_outline(painter, &pe.contours, &|p| spp(m.apply(p)), line)
                    }
                    Some(_) => {}
                }
                continue;
            }
            if is_moving {
                let q: Vec<Pos2> = corners.iter().map(|p| spp(*p)).collect();
                if !cull.intersects(Rect::from_points(&q)) {
                    continue;
                }
                if let Some(bt) = batch.as_mut() {
                    bt.flush(painter);
                }
                painter.add(egui::Shape::closed_line(
                    q.clone(),
                    Stroke::new(1.0, c.selection),
                ));
                for p in q {
                    painter.rect_filled(
                        Rect::from_center_size(
                            p,
                            vec2(theme.metrics.handle_size, theme.metrics.handle_size),
                        ),
                        0.0,
                        c.handle_fill,
                    );
                }
                continue;
            }
            let r = Rect::from_min_max(
                sp(b.left as f64, b.top as f64),
                sp(b.right as f64, b.bottom as f64),
            );
            let r = Rect::from_min_max(
                pos2(r.left().round() + 0.5, r.top().round() + 0.5),
                pos2(r.right().round() - 0.5, r.bottom().round() - 0.5),
            );
            if !cull.intersects(Rect::from_two_pos(r.min, r.max)) {
                continue;
            }
            let hs = theme.metrics.handle_size;
            if let Some(bt) = batch.as_mut() {
                if bt.frame(r, c.selection, hs, c.handle_fill) {
                    continue;
                }
                bt.flush(painter);
            }
            painter.rect_stroke(r, 0.0, Stroke::new(1.0, c.selection), StrokeKind::Middle);
            for p in [
                r.left_top(),
                r.right_top(),
                r.left_bottom(),
                r.right_bottom(),
            ] {
                let h = Rect::from_center_size(p, vec2(hs, hs));
                painter.rect_filled(h, 0.0, c.handle_fill);
                if theme.glow {
                    painter.rect_stroke(
                        h.expand(1.0),
                        0.0,
                        Stroke::new(1.0, with_alpha(c.handle_fill, 90)),
                        StrokeKind::Outside,
                    );
                }
            }
        }
        if let Some(bt) = batch.as_mut() {
            bt.flush(painter);
        }
        // Verlaufs-/Mustergriffe des einzigen gewählten Objekts beim Zeiger: Linie vom Start (runder
        // Punkt) zum Ende (Quadrat) wie im Original (PAR 18/07); beim Ziehen eines Griffs bleiben
        // die alten Griffe bis zum Loslassen stehen, dazu die Vorschau des Werkzeugs (re 37 §3)
        if show_stored_fill_handles(st.session.tools.active, moving.is_some())
            && let [only] = found.as_slice()
            && let Some(e) = only.as_ref().and_then(|p| d.doc.element_at(p))
        {
            // re 40 §1: beim Zeiger folgt die Griffslinie dem Zeiger (live), keine Vorschaulinie;
            // die live gezogenen Griffe zeichnet `Overlay::FillLive` über dem Live-Bild (FP4-02:
            // hier gezeichnet lagen sie unter dem Bild des Objekts und waren nicht zu sehen)
            if live_fill_element(feedback, e).is_none() {
                fill_handles(painter, e, &spp, c);
            }
        }
        // Bewegungspfad der einzigen gewählten Animationsinstanz (TW-20)
        if moving.is_none()
            && let [only] = found.as_slice()
            && let Some(e) = only.as_ref().and_then(|p| d.doc.element_at(p))
            && let Some(pts) = anim::motion_path(&d.doc, e)
        {
            anim::paint_path(painter, &pts, &spp, c.selection);
        }
    }
    st.canvases.entry(id).or_default().sel_frames = frames;
    // Pixelauswahl: laufende Ameisen am echten Umriss (auch bei ausgeblendeten Kanten wie CS6)
    if let Some(m) = d
        .selection
        .pixels
        .as_ref()
        .filter(|_| d.selection.has_pixels())
    {
        let cc = st.canvases.entry(id).or_default();
        let outlines = overlay::ants_for(&mut cc.ants, m);
        // the marching ants move with the dragged pixels (FB-73)
        let (ox, oy) = feedback
            .iter()
            .find_map(|o| match o {
                Overlay::Ghost {
                    rect,
                    quad,
                    pixels: true,
                    ..
                } => Some((quad[0].x - rect.left, quad[0].y - rect.top)),
                _ => None,
            })
            .unwrap_or((0.0, 0.0));
        let ants_spp = |p: Point| spp(Point::new(p.x + ox, p.y + oy));
        overlay::paint_ants(
            painter,
            outlines,
            &ants_spp,
            time,
            c.marquee_a,
            c.marquee_b,
            area,
        );
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(120));
    } else if let Some(cc) = st.canvases.get_mut(&id) {
        cc.ants = None;
    }
    // Werkzeug-Rückmeldung
    let Some(d) = st.session.active() else { return };
    let stroke = Stroke::new(1.0, c.tool_preview);
    // the brush outline is red while a mask is being edited (RE spec 68 §4.1 e5)
    let tip_color = if st.session.mask_selected() {
        Color32::from_rgb(255, 0, 0)
    } else {
        c.tool_preview
    };
    // Gerenderte Vorschauen (Verschieben, Kopie, Transformieren, Text) nur innerhalb der Leinwand
    // wie im fertigen Bild; Rahmen, Griffe und Umrisse dürfen darüber hinaus
    let page_clip = st
        .canvases
        .get(&id)
        .and_then(|c| c.page_screen)
        .or_else(|| {
            page_rect(&d.doc, d.page).map(|pr| {
                Rect::from_min_max(
                    sp(pr.left as f64, pr.top as f64),
                    sp(pr.right as f64, pr.bottom as f64),
                )
            })
        })
        .map(|r| r.intersect(area))
        .unwrap_or(area);
    let page_painter = painter.with_clip_rect(page_clip);
    // the transform tools draw the frame line and its handles 1 screen pixel left of and above the
    // frame, at every zoom (RE spec 67 §1.1); other handles sit on their points
    let xform_shift = if matches!(
        st.session.tools.active,
        fp_engine::ToolId::Scale | fp_engine::ToolId::Skew | fp_engine::ToolId::Distort
    ) {
        vec2(-1.0, -1.0)
    } else {
        vec2(0.0, 0.0)
    };
    // Subselection tool: point display per re 54 (Some(ShowSolidPoints))
    let subsel_points = matches!(
        st.session.tools.active,
        fp_engine::ToolId::Subselection | fp_engine::ToolId::MaskElementSelect
    )
    .then(|| st.session.prefs.get_bool("ShowSolidPoints", false));
    let cc = st.canvases.entry(id).or_default();
    let mut had_text = false;
    for o in feedback.iter().cloned() {
        match o {
            Overlay::Rect { rect, dashed } => {
                let r = Rect::from_min_max(
                    sp(rect.left as f64, rect.top as f64),
                    sp(rect.right as f64, rect.bottom as f64),
                );
                if dashed {
                    dashed_rect(painter, r, c.marquee_a, c.marquee_b, time);
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_millis(120));
                } else {
                    painter.rect_stroke(r, 0.0, stroke, StrokeKind::Middle);
                }
            }
            Overlay::Ellipse { rect, dashed } => {
                let r = Rect::from_min_max(
                    sp(rect.left as f64, rect.top as f64),
                    sp(rect.right as f64, rect.bottom as f64),
                );
                let pts: Vec<Pos2> = (0..64)
                    .map(|i| {
                        let a = i as f32 / 64.0 * std::f32::consts::TAU;
                        pos2(
                            r.center().x + r.width() / 2.0 * a.cos(),
                            r.center().y + r.height() / 2.0 * a.sin(),
                        )
                    })
                    .collect();
                if dashed {
                    let pts: Vec<Point> = pts.iter().map(|p| Point::new(p.x, p.y)).collect();
                    overlay::paint_ants(
                        painter,
                        &[pts],
                        &|p| pos2(p.x, p.y),
                        time,
                        c.marquee_a,
                        c.marquee_b,
                        area,
                    );
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_millis(120));
                } else {
                    painter.add(egui::Shape::closed_line(pts, stroke));
                }
            }
            Overlay::Line { from, to } => {
                painter.line_segment([spp(from), spp(to)], stroke);
            }
            Overlay::Polyline { points, closed } => {
                let v: Vec<Pos2> = points.iter().map(|p| spp(*p)).collect();
                if closed {
                    painter.add(egui::Shape::closed_line(v, stroke));
                } else {
                    painter.add(egui::Shape::line(v, stroke));
                }
            }
            Overlay::Path { contours } => {
                for ct in &contours {
                    let v: Vec<Pos2> = fp_engine::geometry::flatten(ct)
                        .iter()
                        .map(|p| spp(*p))
                        .collect();
                    if ct.closed {
                        painter.add(egui::Shape::closed_line(v, stroke));
                    } else {
                        painter.add(egui::Shape::line(v, stroke));
                    }
                }
            }
            Overlay::Handles { points } => {
                for p in points {
                    let h = Rect::from_center_size(spp(p) + xform_shift, vec2(5.0, 5.0));
                    painter.rect_filled(h, 0.0, c.handle_fill);
                    painter.rect_stroke(
                        h,
                        0.0,
                        Stroke::new(1.0, c.handle_border),
                        StrokeKind::Outside,
                    );
                }
            }
            Overlay::TransformBox { quad, center } => {
                let v: Vec<Pos2> = quad.iter().map(|p| spp(*p) + xform_shift).collect();
                painter.add(egui::Shape::closed_line(
                    v.clone(),
                    Stroke::new(1.0, c.selection),
                ));
                for p in &v {
                    let h = Rect::from_center_size(*p, vec2(5.0, 5.0));
                    painter.rect_filled(h, 0.0, c.handle_fill);
                    painter.rect_stroke(
                        h,
                        0.0,
                        Stroke::new(1.0, c.handle_border),
                        StrokeKind::Outside,
                    );
                }
                let _ = center;
            }
            Overlay::Text {
                anchor,
                width,
                text,
            } => {
                let a = spp(anchor);
                let r = painter.text(
                    a,
                    Align2::LEFT_TOP,
                    text,
                    theme.font(12.0 * view.zoom as f32),
                    c.text,
                );
                let w = width
                    .map(|w| w * view.zoom as f32)
                    .unwrap_or(r.width().max(4.0));
                painter.rect_stroke(
                    Rect::from_min_size(a, vec2(w, r.height().max(14.0))),
                    0.0,
                    stroke,
                    StrokeKind::Outside,
                );
            }
            Overlay::Stroke { points, diameter } => {
                for p in points {
                    painter.circle_stroke(
                        spp(p),
                        (diameter as f64 * view.zoom / 2.0).max(1.0) as f32,
                        Stroke::new(1.0, with_alpha(c.tool_preview, 160)),
                    );
                }
            }
            Overlay::SmartGuide { from, to } => {
                painter.line_segment([spp(from), spp(to)], Stroke::new(1.0, c.smart_guide));
            }
            Overlay::Label { at, text } => {
                label_at(painter, &theme, spp(at) + vec2(8.0, 8.0), &text);
            }
            Overlay::Highlight { contours, rect } => {
                let hs = Stroke::new(1.0, c.hover_highlight);
                // crisp 1 px line on the pixels inside the path coordinates (TW-157)
                for ct in &contours {
                    let v: Vec<Pos2> = crisp_outline(
                        &fp_engine::geometry::flatten(ct)
                            .iter()
                            .map(|p| spp(*p))
                            .collect::<Vec<_>>(),
                    );
                    if ct.closed {
                        painter.add(egui::Shape::closed_line(v, hs));
                    } else {
                        painter.add(egui::Shape::line(v, hs));
                    }
                }
                if let Some(r) = rect {
                    let r = Rect::from_min_max(
                        sp(r.left as f64, r.top as f64),
                        sp(r.right as f64, r.bottom as f64),
                    );
                    painter.rect_stroke(
                        Rect::from_min_max(
                            pos2(r.left().round() + 0.5, r.top().round() + 0.5),
                            pos2(r.right().round() - 0.5, r.bottom().round() - 0.5),
                        ),
                        0.0,
                        hs,
                        StrokeKind::Middle,
                    );
                }
                // filled 5 x 5 px square on every corner point (re 52 §2b)
                for q in highlight_corners(&contours, rect) {
                    painter.rect_filled(
                        Rect::from_center_size(spp(q), vec2(5.0, 5.0)),
                        0.0,
                        c.hover_highlight,
                    );
                }
            }
            Overlay::PathPoints {
                contours,
                selected,
                handles,
            } if subsel_points.is_some() && handles => {
                // FB-06 (re 54 §1.3/§1.4): nothing with hidden edges; otherwise both handles of every
                // segment with a selected end, then the anchor squares on top (selected filled,
                // the others hollow; `ShowSolidPoints` reverses the fill)
                let Some(solid) = subsel_points.filter(|_| view.show_edges) else {
                    continue;
                };
                subselection_points(painter, &contours, &selected, solid, &spp, c);
            }
            Overlay::PathPoints {
                contours,
                selected,
                handles,
            } => {
                // CS6: Anker als kleine Quadrate (Auswahlfarbe gefüllt, unterausgewählt hohl), Henkel als Kreise
                for (ci, ct) in contours.iter().enumerate() {
                    for (ni, n) in ct.nodes.iter().enumerate() {
                        let a = spp(n.anchor);
                        let sel = selected.contains(&(ci, ni));
                        if handles && sel {
                            for q in [n.pred, n.succ] {
                                if q != n.anchor {
                                    let qs = spp(q);
                                    painter.line_segment([a, qs], Stroke::new(1.0, c.selection));
                                    painter.circle_filled(qs, 2.5, c.selection);
                                }
                            }
                        }
                        let h = Rect::from_center_size(a, vec2(5.0, 5.0));
                        if sel || !handles {
                            painter.rect_filled(h, 0.0, c.selection);
                        } else {
                            painter.rect_filled(h, 0.0, c.handle_border);
                            painter.rect_stroke(
                                h,
                                0.0,
                                Stroke::new(1.0, c.selection),
                                StrokeKind::Inside,
                            );
                        }
                    }
                }
            }
            Overlay::Ghost { quad, .. } => {
                // die Vorschau selbst zeichnet `preview_stack` (in der Stapelfolge, unter Raster,
                // Hilfslinien und Auswahlrahmen); hier nur der Umriss
                let v: Vec<Pos2> = quad.iter().map(|p| spp(*p)).collect();
                painter.add(egui::Shape::closed_line(v, Stroke::new(1.0, c.selection)));
            }
            Overlay::Shade { rect } => {
                let r = Rect::from_min_max(
                    sp(rect.left as f64, rect.top as f64),
                    sp(rect.right as f64, rect.bottom as f64),
                );
                for s in [
                    Rect::from_min_max(area.min, pos2(area.right(), r.top())),
                    Rect::from_min_max(pos2(area.left(), r.bottom()), area.max),
                    Rect::from_min_max(pos2(area.left(), r.top()), pos2(r.left(), r.bottom())),
                    Rect::from_min_max(pos2(r.right(), r.top()), pos2(area.right(), r.bottom())),
                ] {
                    painter.rect_filled(s.intersect(area), 0.0, c.crop_shade);
                }
            }
            Overlay::Pivot { at } => {
                let p = spp(at);
                painter.circle_stroke(p, 4.0, Stroke::new(1.0, c.selection));
                painter.circle_filled(p, 1.5, c.selection);
            }
            Overlay::GradientHandles { start, end } => {
                // re 37 §3: schwarze 1-px-Linie vom festen Griff zum Zeiger, Griffsymbol am Zeiger
                // (Quadrat, innen hell); über der ganzen Leinwand, nicht am Objekt beschnitten
                let (a, b) = (spp(start), spp(end));
                let line = Stroke::new(1.0, GRADIENT_PREVIEW_LINE);
                painter.with_clip_rect(area).line_segment([a, b], line);
                painter.circle_filled(a, 3.0, c.handle_fill);
                painter.circle_stroke(a, 3.0, line);
                let r = Rect::from_center_size(b, vec2(7.0, 7.0));
                painter.rect_filled(r, 0.0, c.handle_fill);
                painter.rect_stroke(r, 0.0, line, StrokeKind::Inside);
            }
            // FB-67: the page itself renders the live point drag (preview copy), nothing on top
            Overlay::PathLive { .. } => {}
            Overlay::PenPath { target } => {
                // the open path being drawn without its fill (RE spec 60 section 1.2); the page
                // render hides the element itself
                if let Some(e) = pen_path_without_fill(&d.doc, d.page, target) {
                    let key = generation.wrapping_mul(0x9e37_79b9_7f4a_7c15)
                        ^ target
                        ^ 0x7065_6e5f_7061_7468;
                    if let Some(s) = cc.ghosts.element_sprite(
                        &ctx,
                        &st.renderer,
                        &d.doc,
                        frame_i,
                        &e,
                        key,
                        view.zoom,
                    ) {
                        ghost::paint_at(&page_painter, s, &spp);
                    }
                }
            }
            Overlay::FillLive { target, handles } => {
                // Füllung live mit den gezogenen Griffen (Zeiger, re 40 §1)
                if let Some(e) =
                    fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(target))
                {
                    let mut e = (*e).clone();
                    fp_engine::commands::panel::replace_handles(&mut e, handles);
                    let key = fill_live_key(generation, target, &handles);
                    if let Some(s) = cc.ghosts.fill_sprite(
                        &ctx,
                        &st.renderer,
                        &d.doc,
                        frame_i,
                        &e,
                        key,
                        view.zoom,
                    ) {
                        ghost::paint_at(&page_painter, s, &spp);
                    }
                    // Griffslinie am Zeiger über dem Live-Bild (re 40 §1, FP4-02)
                    if view.show_edges && fill_live_shows_handles(st.session.tools.active) {
                        fill_handles(painter, &e, &spp, c);
                    }
                }
            }
            Overlay::TextEdit(v) => {
                had_text = true;
                if let Some(lc) =
                    textedit::ensure_layout(&mut cc.text, &st.renderer, &d.doc, &v, generation)
                {
                    // Vorschau mit fp-render (gleicher Satz wie das fertige Textobjekt)
                    if let Some(e) = v.preview_element(&d.doc) {
                        let key = textedit::view_key(&v, generation);
                        if let Some(s) = cc.ghosts.element_sprite(
                            &ctx,
                            &st.renderer,
                            &d.doc,
                            frame_i,
                            &e,
                            key,
                            view.zoom,
                        ) {
                            ghost::paint_at(&page_painter, s, &spp);
                        }
                    }
                    textedit::paint(painter, lc, &v, time, c, &spp);
                    let cursor_rect = textedit::caret_rect(lc, &v, &spp);
                    ctx.output_mut(|out| {
                        out.ime = Some(egui::output::IMEOutput {
                            purpose: egui::IMEPurpose::Normal,
                            rect: area,
                            cursor_rect,
                            should_interrupt_composition: false,
                        });
                    });
                    ctx.request_repaint_after(std::time::Duration::from_millis(250));
                }
            }
            Overlay::BrushTip {
                at,
                diameter,
                round,
            } => {
                let p = spp(at);
                let rr = (diameter as f64 * view.zoom / 2.0).max(1.0) as f32;
                if round {
                    painter.circle_stroke(p, rr, Stroke::new(1.0, tip_color));
                } else {
                    painter.rect_stroke(
                        Rect::from_center_size(p, vec2(rr * 2.0, rr * 2.0)),
                        0.0,
                        Stroke::new(1.0, tip_color),
                        StrokeKind::Middle,
                    );
                }
            }
            Overlay::ControlPoint { at, tooltip, hot } => {
                // gelbe Raute mit dunklem Rand (Steuerpunkt einer automatischen Form)
                let p = spp(at);
                let r = if hot { 5.5 } else { 4.5 };
                let pts = vec![
                    p + vec2(0.0, -r),
                    p + vec2(r, 0.0),
                    p + vec2(0.0, r),
                    p + vec2(-r, 0.0),
                ];
                painter.add(egui::Shape::convex_polygon(
                    pts,
                    Color32::from_rgb(255, 214, 0),
                    Stroke::new(1.0, Color32::from_rgb(90, 70, 0)),
                ));
                if hot && let Some(t) = tooltip {
                    handle_tip_at(painter, &theme, p, &t);
                }
            }
            Overlay::Marker { at } => {
                let p = spp(at);
                painter.line_segment([p - vec2(5.0, 0.0), p + vec2(5.0, 0.0)], stroke);
                painter.line_segment([p - vec2(0.0, 5.0), p + vec2(0.0, 5.0)], stroke);
            }
            // Malwerkzeuge: der echte Strich live (PINSEL-1) – zeichnet `preview_stack`
            Overlay::Paint(_) => {}
        }
    }
    if had_text {
        cc.text_linger = Some(time);
    } else {
        // Nach dem Ende der Bearbeitung (Esc, Klick daneben): die Textvorschau stehen lassen, bis
        // das Hintergrundbild mit dem übernommenen Text da ist – sonst war der Text bis zum
        // nächsten Neuzeichnen unsichtbar (PM-1, N13). Höchstens 2 s.
        let waiting = cc.text_linger.is_some_and(|t0| time - t0 < 2.0)
            && cc
                .shown
                .is_none_or(|k| k.generation != generation || k.variant != 0);
        match cc.ghosts.text.as_ref() {
            Some(s) if waiting => {
                ghost::paint_at(&page_painter, s, &spp);
                ctx.request_repaint_after(std::time::Duration::from_millis(16));
            }
            _ => {
                cc.text = None;
                cc.ghosts.text = None;
                cc.text_linger = None;
            }
        }
    }
}

/// Eine Schicht über den Vorschauen ([`render::CanvasCache::layers`]) an der Stelle der Textur.
fn paint_layer(
    painter: &egui::Painter,
    layers: &[Option<egui::TextureHandle>],
    i: usize,
    rect: Option<Rect>,
) {
    if let (Some(Some(t)), Some(r)) = (layers.get(i), rect) {
        painter.image(
            t.id(),
            r,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }
}

/// Vorschauen in der Stapelfolge (N34, BA-039), direkt über dem Hintergrundbild und unter Raster,
/// Hilfslinien, Auswahlrahmen und Werkzeug-Rückmeldung: je Lauf gezogener Objekte die Vorschau
/// (bzw. die Malkacheln), darüber die durchsichtige Schicht der Objekte darüber. Nach dem Loslassen
/// bleibt das stehen, bis das neue Hintergrundbild da ist.
fn preview_stack(
    ui: &Ui,
    st: &mut AppState,
    page_clip: Rect,
    area: Rect,
    view: &fp_engine::ViewState,
    feedback: &[Overlay],
) {
    let ctx = ui.ctx().clone();
    let time = ui.input(|i| i.time);
    let preview_drag = st.session.prefs.get_bool("PreviewDrag", true);
    let Some(d) = st.session.active() else { return };
    let (id, generation, page_i, frame_i) = (d.id, d.generation, d.page, d.frame);
    let spp = |p: Point| to_screen(view, area.min, p.x as f64, p.y as f64);
    let page_painter = ui.painter().with_clip_rect(page_clip);
    let cc = st.canvases.entry(id).or_default();
    let ready = cc.shown.is_some_and(|k| k.variant == cc.stack_variant);
    let tr = cc.tex_rect;
    let mut layers_drawn = false;
    let mut had_ghost = false;
    let mut had_paint = false;
    for o in feedback {
        match o {
            Overlay::Ghost {
                ids,
                rect,
                quad,
                copy,
                pixels,
            } => {
                had_ghost = true;
                // FB-87: drawn by the canvas image itself (moved bitmap in a render-only copy)
                if *pixels && preview_drag && ghost::live_pixels(feedback, &d.doc, page_i).is_some()
                {
                    cc.ghosts.linger = None;
                    continue;
                }
                cc.ghosts.linger = Some(ghost::Linger {
                    generation,
                    rect: *rect,
                    quad: *quad,
                    // a pixel copy keeps the bitmap id: the preview lingers like a move
                    ids: if *copy && !*pixels {
                        Vec::new()
                    } else {
                        ids.clone()
                    },
                    pixels: *pixels,
                });
                if !preview_drag || layers_drawn {
                    continue;
                }
                // Bis das passende Hintergrundbild da ist, bleibt das alte Bild stehen
                let plan = cc
                    .stack
                    .clone()
                    .unwrap_or_else(|| stack::StackPlan::flat(ids, *copy));
                if !ready && plan.has_layers() {
                    continue;
                }
                let runs: Vec<Vec<u64>> = plan.runs.iter().map(|r| r.ids.clone()).collect();
                let layers = if ready { cc.layers.clone() } else { Vec::new() };
                // FB-73: a pixel move shows the bitmap with the hole in place and only the
                // selected pixels at the offset (each limited to the view, BA-097)
                if *pixels && let (Some(bid), Some(m)) = (ids.first(), d.selection.pixels.as_ref())
                {
                    let visible = view.visible_rect();
                    let need_hole = ghost::needed_part(*rect, &rect.corners(), visible);
                    let need_cluster = ghost::needed_part(*rect, quad, visible);
                    let (hole, cluster) = cc.ghosts.pixel_sprites(
                        &ctx,
                        &st.renderer,
                        &d.doc,
                        (generation, page_i, frame_i),
                        (*bid, *copy),
                        m,
                        view.zoom,
                        (need_hole, need_cluster),
                    );
                    for i in 0..runs.len().max(layers.len()).max(1) {
                        if i == 0 {
                            if let Some(s) = hole {
                                ghost::paint_at(&page_painter, s, &spp);
                            }
                            if let Some(s) = cluster {
                                ghost::paint_mapped(
                                    &page_painter,
                                    s,
                                    *rect,
                                    quad,
                                    Color32::WHITE,
                                    &spp,
                                );
                            }
                        }
                        paint_layer(&page_painter, &layers, i, tr);
                    }
                    layers_drawn = true;
                    continue;
                }
                // BA-097: only the part in view (plus a margin) goes into the texture
                let need = ghost::needed_part(*rect, quad, view.visible_rect());
                let sprites = cc.ghosts.drag_sprites(
                    &ctx,
                    &st.renderer,
                    &d.doc,
                    generation,
                    page_i,
                    frame_i,
                    &runs,
                    view.zoom,
                    need,
                );
                for i in 0..sprites.len().max(layers.len()) {
                    if let Some(Some(s)) = sprites.get(i) {
                        ghost::paint_mapped(&page_painter, s, *rect, quad, Color32::WHITE, &spp);
                    }
                    paint_layer(&page_painter, &layers, i, tr);
                }
                layers_drawn = true;
            }
            Overlay::Paint(p) => {
                had_paint = true;
                let input = paint_preview::CompositeInput {
                    renderer: &st.renderer,
                    doc: &d.doc,
                    generation,
                    page: page_i,
                    frame: frame_i,
                    gamma: view.gamma_preview,
                };
                if paint_preview::paint_composite(&ctx, &page_painter, id, p, &input, time, &spp)
                    .is_none()
                {
                    paint_preview::paint(&ctx, &page_painter, id, p, view.zoom, time, &spp);
                }
            }
            _ => {}
        }
    }
    if !had_paint {
        paint_preview::linger(
            &ctx,
            &page_painter,
            id,
            generation,
            cc.shown.map(|k| k.generation),
            time,
            &spp,
        );
    }
    // Nach dem Loslassen: Vorschau stehen lassen, bis das Hintergrundbild ohne Ausblendung da ist
    let mut lingering = false;
    if !had_ghost && let Some(l) = &cc.ghosts.linger {
        let waiting = cc.shown.is_some_and(|k| k.variant != 0) && !l.ids.is_empty();
        if waiting && preview_drag && l.pixels {
            // FB-73: the pixel move stays until the new background is there
            for i in 0..cc.layers.len().max(1) {
                if i == 0
                    && let Some(p) = &cc.ghosts.pixels
                {
                    if let Some(s) = &p.hole.sprite {
                        ghost::paint_at(&page_painter, s, &spp);
                    }
                    if let Some(s) = &p.cluster.sprite {
                        ghost::paint_mapped(
                            &page_painter,
                            s,
                            l.rect,
                            &l.quad,
                            Color32::WHITE,
                            &spp,
                        );
                    }
                }
                paint_layer(&page_painter, &cc.layers, i, tr);
            }
            lingering = true;
        } else if waiting && preview_drag {
            for i in 0..cc.ghosts.drag.len().max(cc.layers.len()) {
                if let Some(Some(s)) = cc.ghosts.drag.get(i) {
                    ghost::paint_mapped(&page_painter, s, l.rect, &l.quad, Color32::WHITE, &spp);
                }
                paint_layer(&page_painter, &cc.layers, i, tr);
            }
            lingering = true;
        } else {
            cc.ghosts.linger = None;
            cc.ghosts.drag.clear();
            cc.ghosts.pixels = None;
        }
    }
    // Schichten gehören zum angezeigten Hintergrundbild: ohne laufende Vorschau einfach darüber
    if !layers_drawn && !lingering {
        for i in 0..cc.layers.len() {
            paint_layer(&page_painter, &cc.layers, i, tr);
        }
    }
}

/// Element mit den live gezogenen Füllgriffen, wenn der Zeiger gerade einen Griff von `e` zieht.
fn live_fill_element(feedback: &[Overlay], e: &fp_model::Element) -> Option<fp_model::Element> {
    feedback.iter().find_map(|o| match o {
        Overlay::FillLive { target, handles } if *target == e.id.0 => {
            let mut n = e.clone();
            fp_engine::commands::panel::replace_handles(&mut n, *handles);
            Some(n)
        }
        _ => None,
    })
}

/// Schlüssel der Live-Füllvorschau (Generation, Objekt, Griffe).
fn fill_live_key(generation: u64, target: u64, h: &fp_model::FillHandles) -> u64 {
    let mut k = generation.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ target;
    for v in [
        h.start.x, h.start.y, h.end.x, h.end.y, h.second.x, h.second.y,
    ] {
        k = (k ^ v.to_bits() as u64).wrapping_mul(0x0100_0000_01b3);
    }
    k
}

/// The open path element `target` the pen is drawing, without its fill (RE spec 60 section 1.2:
/// the fill of an open path appears only once it is closed or finished); `None` when it has no
/// fill or no open contour (then the page shows it unchanged).
fn pen_path_without_fill(
    doc: &fp_model::Document,
    page: usize,
    target: u64,
) -> Option<fp_model::Element> {
    let e = fp_engine::docops::element_on_page(doc, page, fp_model::ElementId(target))?;
    let fp_model::ElementKind::Path(pe) = &e.kind else {
        return None;
    };
    if pe.attrs.fill.is_none() || pe.contours.iter().all(|c| c.closed) {
        return None;
    }
    let mut n = (*e).clone();
    if let fp_model::ElementKind::Path(pe) = &mut n.kind {
        let a = std::sync::Arc::make_mut(&mut pe.attrs);
        a.fill = None;
        a.fill_texture = None;
    }
    Some(n)
}

/// Path outline as crisp 1 px lines along the real curves (Subselection display, re 54 §6).
fn paint_path_outline(
    painter: &egui::Painter,
    contours: &[fp_model::Contour],
    spp: &dyn Fn(Point) -> Pos2,
    stroke: Stroke,
) {
    for ct in contours {
        let v: Vec<Pos2> = crisp_outline(
            &fp_engine::geometry::flatten(ct)
                .iter()
                .map(|p| spp(*p))
                .collect::<Vec<_>>(),
        );
        if v.len() < 2 {
            continue;
        }
        if ct.closed {
            painter.add(egui::Shape::closed_line(v, stroke));
        } else {
            painter.add(egui::Shape::line(v, stroke));
        }
    }
}

/// Handles drawn for the Subselection display (re 54 §1.3): both handles of every segment with at
/// least one selected end, plus the outer handle of a selected first/last point of an open path.
/// Pairs of (anchor, handle) as (contour, node, which) with `which` = false for pred, true for succ.
pub fn subselection_handles(
    contours: &[fp_model::Contour],
    selected: &[(usize, usize)],
) -> Vec<(usize, usize, bool)> {
    let mut out = Vec::new();
    for (ci, ct) in contours.iter().enumerate() {
        let n = ct.nodes.len();
        if n < 2 {
            continue;
        }
        let sel = |k: usize| selected.contains(&(ci, k));
        let segs = if ct.closed { n } else { n - 1 };
        for i in 0..segs {
            let j = (i + 1) % n;
            if sel(i) || sel(j) {
                out.push((ci, i, true));
                out.push((ci, j, false));
            }
        }
        if !ct.closed {
            if sel(0) {
                out.push((ci, 0, false));
            }
            if sel(n - 1) {
                out.push((ci, n - 1, true));
            }
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// Anchors and handles of the Subselection display (re 54 §1.3, §1.4, §6): handle lines first,
/// then a 5 x 5 px square on every anchor, filled when `selected XOR solid`, else hollow (1 px
/// border in the selection colour, white inside).
fn subselection_points(
    painter: &egui::Painter,
    contours: &[fp_model::Contour],
    selected: &[(usize, usize)],
    solid: bool,
    spp: &dyn Fn(Point) -> Pos2,
    c: &crate::theme::Palette,
) {
    let line = Stroke::new(1.0, c.selection);
    for (ci, ni, succ) in subselection_handles(contours, selected) {
        let n = &contours[ci].nodes[ni];
        let q = if succ { n.succ } else { n.pred };
        if q != n.anchor {
            let (a, qs) = (spp(n.anchor), spp(q));
            painter.line_segment([a, qs], line);
            painter.circle_filled(qs, 2.5, c.selection);
        }
    }
    for (ci, ct) in contours.iter().enumerate() {
        for (ni, n) in ct.nodes.iter().enumerate() {
            let a = spp(n.anchor);
            // pixel-aligned: the square covers whole pixels centred on the anchor pixel
            let a = pos2(a.x.floor() + 0.5, a.y.floor() + 0.5);
            let h = Rect::from_center_size(a, vec2(5.0, 5.0));
            if selected.contains(&(ci, ni)) != solid {
                painter.rect_filled(h, 0.0, c.selection);
            } else {
                painter.rect_filled(h, 0.0, c.handle_border);
                painter.rect_stroke(h, 0.0, line, StrokeKind::Inside);
            }
        }
    }
}

/// Screen points of a hover outline snapped to pixel centres, like the selection frame: points on
/// the left/top half of the outline go to the pixel right/below the coordinate, points on the
/// right/bottom half to the pixel left/above it, so a 1 px stroke covers exactly one pixel row or
/// column inside the path coordinates instead of two anti-aliased ones (TW-157).
pub fn crisp_outline(v: &[Pos2]) -> Vec<Pos2> {
    let r = Rect::from_points(v);
    let c = r.center();
    let snap = |x: f32, mid: f32, size: f32| {
        if size >= 1.0 && x > mid {
            x.round() - 0.5
        } else {
            x.round() + 0.5
        }
    };
    v.iter()
        .map(|p| pos2(snap(p.x, c.x, r.width()), snap(p.y, c.y, r.height())))
        .collect()
}

/// Zeichnet die Leinwand die gespeicherten Verlaufsgriffe des gewählten Objekts? Beim Zeiger bzw.
/// Corner points of a hover frame, each marked with a filled 5 x 5 px square (re 52 §2b): the
/// anchors of the outlined paths, or the four corners of the bounds rectangle.
pub fn highlight_corners(
    contours: &[fp_model::Contour],
    rect: Option<fp_geom::Rect>,
) -> Vec<Point> {
    let mut v: Vec<Point> = contours
        .iter()
        .flat_map(|c| c.nodes.iter().map(|n| n.anchor))
        .collect();
    if let Some(r) = rect {
        v.extend([
            Point::new(r.left, r.top),
            Point::new(r.right, r.top),
            Point::new(r.right, r.bottom),
            Point::new(r.left, r.bottom),
        ]);
    }
    v
}

/// Hintere auswählen, solange kein Objekt verschoben wird; auch während ein Griff gezogen wird
/// (re 37 §3: die alte Linie samt Griffen bleibt bis zum Loslassen, BA-120 zurückgenommen). Die
/// Füllwerkzeuge zeichnen sie über ihre eigene Rückmeldung.
fn show_stored_fill_handles(tool: fp_engine::ToolId, moving: bool) -> bool {
    matches!(
        tool,
        fp_engine::ToolId::Pointer | fp_engine::ToolId::SelectBehind
    ) && !moving
}

/// Zeichnet `Overlay::FillLive` die gezogenen Griffe über das Live-Bild (Zeiger bzw. Hintere
/// auswählen, re 40 §1: die Griffslinie folgt dem Zeiger)?
fn fill_live_shows_handles(tool: fp_engine::ToolId) -> bool {
    show_stored_fill_handles(tool, false)
}

/// Linie der Verlaufsgriff-Vorschau: schwarz, 1 px, nicht am Objekt beschnitten (re 37 §3).
const GRADIENT_PREVIEW_LINE: Color32 = Color32::BLACK;

/// Beschriftung mit Hintergrund (Größe, Winkel, Abstand, Position).
/// Füllgriffe eines Objekts mit Verlaufs- oder Musterfüllung: Linie Start → Ende, Start als runder
/// Punkt, Ende als Quadrat; bei Formen mit zweiter Achse (Ellipse, Rechteck, Muster) auch deren
/// Griff (PAR 18/07).
fn fill_handles(
    painter: &egui::Painter,
    e: &fp_model::Element,
    spp: &dyn Fn(Point) -> Pos2,
    c: &crate::theme::Palette,
) {
    let Some(a) = fp_engine::docops::path_attrs_of(e) else {
        return;
    };
    let Some(f) = a.fill.as_ref() else { return };
    let kind = fp_engine::commands::panel::fill_kind(Some(f));
    if kind != 2 && kind != 3 {
        return;
    }
    let Some(h) = fp_engine::commands::panel::handles_of(e).filter(|h| h.valid) else {
        return;
    };
    let (s, en, se) = (spp(h.start), spp(h.end), spp(h.second));
    let line = Stroke::new(1.0, c.selection);
    let second = kind == 3 || matches!(f.shape, 9 | 10);
    painter.line_segment([s, en], line);
    if second {
        painter.line_segment([s, se], line);
    }
    let knob = |p: Pos2, round: bool| {
        if round {
            painter.circle_filled(p, 3.5, c.handle_fill);
            painter.circle_stroke(p, 3.5, line);
        } else {
            let r = Rect::from_center_size(p, vec2(7.0, 7.0));
            painter.rect_filled(r, 0.0, c.handle_fill);
            painter.rect_stroke(r, 0.0, line, StrokeKind::Inside);
        }
    };
    knob(s, true);
    knob(en, false);
    if second {
        knob(se, false);
    }
}

/// Hintergrund der Griff-Tooltips automatischer Formen im Original (Lauf 10, TW-96).
pub const HANDLE_TIP_BG: Color32 = Color32::from_rgb(214, 214, 214);

/// Griff-Tooltip einer automatischen Form wie im Original (`tool-autoshapes/04`, TW-96): graues
/// Kästchen ohne Rand mit schwarzer Schrift, linke Kante am Griff, 14 px darunter (also links
/// unter dem Zeiger und über der Form). Liefert das Kästchen.
pub fn handle_tip_at(painter: &egui::Painter, theme: &Theme, handle: Pos2, text: &str) -> Rect {
    let galley = painter.layout_no_wrap(text.to_string(), theme.small_font(), Color32::BLACK);
    let r = Rect::from_min_size(
        pos2(handle.x.round(), (handle.y + 14.0).round()),
        galley.size() + vec2(6.0, 2.0),
    );
    painter.rect_filled(r, 3.0, HANDLE_TIP_BG);
    painter.galley(r.min + vec2(3.0, 1.0), galley, Color32::BLACK);
    r
}

fn label_at(painter: &egui::Painter, theme: &Theme, at: Pos2, text: &str) {
    let c = &theme.colors;
    let galley = painter.layout_no_wrap(text.to_string(), theme.small_font(), c.label_text);
    let r = Rect::from_min_size(at, galley.size()).expand2(vec2(3.0, 1.0));
    painter.rect_filled(r, 2.0, c.label_bg);
    painter.galley(at, galley, c.label_text);
}

fn argb(a: fp_color::Argb) -> Color32 {
    Color32::from_rgb(
        (a.0 >> 16 & 0xff) as u8,
        (a.0 >> 8 & 0xff) as u8,
        (a.0 & 0xff) as u8,
    )
}

/// „Laufende Ameisen“: gestrichelter Rahmen, dessen Muster mit der Zeit wandert.
pub(crate) fn dashed_rect(painter: &egui::Painter, r: Rect, a: Color32, b: Color32, time: f64) {
    painter.rect_stroke(r, 0.0, Stroke::new(1.0, b), StrokeKind::Middle);
    let phase = ((time * 8.0) as i64 % 8) as f32;
    let corners = [
        r.left_top(),
        r.right_top(),
        r.right_bottom(),
        r.left_bottom(),
        r.left_top(),
    ];
    for w in corners.windows(2) {
        let (p, q) = (w[0], w[1]);
        let len = p.distance(q);
        let dir = (q - p) / len.max(1e-3);
        let mut t = phase - 8.0;
        while t < len {
            let t0 = t.max(0.0);
            let t1 = (t + 4.0).min(len);
            if t1 > t0 {
                painter.line_segment([p + dir * t0, p + dir * t1], Stroke::new(1.0, a));
            }
            t += 8.0;
        }
    }
}

fn rulers(ui: &mut Ui, st: &mut AppState, rect: Rect, area: Rect, view: &fp_engine::ViewState) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let rs = theme.metrics.ruler_size;
    let top = Rect::from_min_max(
        pos2(area.left(), rect.top()),
        pos2(rect.right(), area.top()),
    );
    let left = Rect::from_min_max(
        pos2(rect.left(), area.top()),
        pos2(area.left(), rect.bottom()),
    );
    let corner = Rect::from_min_size(rect.min, vec2(rs, rs));
    let p = ui.painter();
    p.rect_filled(top, 0.0, c.ruler_bg);
    p.rect_filled(left, 0.0, c.ruler_bg);
    p.rect_filled(corner, 0.0, c.ruler_bg);
    p.hline(
        top.x_range(),
        top.bottom() - 0.5,
        Stroke::new(1.0, c.ruler_tick),
    );
    p.vline(
        left.right() - 0.5,
        left.y_range(),
        Stroke::new(1.0, c.ruler_tick),
    );
    let z = view.zoom;
    let step = [
        1.0, 2.0, 5.0, 10.0, 20.0, 25.0, 50.0, 100.0, 200.0, 250.0, 500.0, 1000.0, 2000.0, 5000.0,
    ]
    .into_iter()
    .find(|s| s * z >= 50.0)
    .unwrap_or(10000.0);
    let minor = step / 5.0;
    let font = theme.font(9.0);
    let vis = view.visible_rect();
    let mut x = (vis.left as f64 / minor).floor() * minor;
    while x <= vis.right as f64 {
        let sx = area.left() + ((x - view.origin.0) * z) as f32;
        let major = (x / step).round() * step == x || ((x / step).fract()).abs() < 1e-9;
        let h = if major { rs } else { rs * 0.3 };
        p.vline(
            sx.round() + 0.5,
            (top.bottom() - h)..=top.bottom(),
            Stroke::new(1.0, c.ruler_tick),
        );
        if major {
            p.text(
                pos2(sx + 2.0, top.top() + 1.0),
                Align2::LEFT_TOP,
                format!("{x:.0}"),
                font.clone(),
                c.ruler_text,
            );
        }
        x += minor;
    }
    let mut y = (vis.top as f64 / minor).floor() * minor;
    while y <= vis.bottom as f64 {
        let sy = area.top() + ((y - view.origin.1) * z) as f32;
        let major = ((y / step).fract()).abs() < 1e-9;
        let w = if major { rs } else { rs * 0.3 };
        p.hline(
            (left.right() - w)..=left.right(),
            sy.round() + 0.5,
            Stroke::new(1.0, c.ruler_tick),
        );
        if major {
            p.text(
                pos2(left.left() + 1.0, sy + 2.0),
                Align2::LEFT_TOP,
                format!("{y:.0}"),
                font.clone(),
                c.ruler_text,
            );
        }
        y += minor;
    }
    if let Some(pd) = st.pointer_doc {
        let sx = area.left() + ((pd.x as f64 - view.origin.0) * z) as f32;
        let sy = area.top() + ((pd.y as f64 - view.origin.1) * z) as f32;
        p.vline(sx, top.y_range(), Stroke::new(1.0, c.ruler_marker));
        p.hline(left.x_range(), sy, Stroke::new(1.0, c.ruler_marker));
    }
    // Hilfslinien aus dem Lineal ziehen
    let Some(id) = st.session.active_id() else {
        return;
    };
    let rt = ui.interact(top, Id::new("ruler-top"), Sense::drag());
    let rl = ui.interact(left, Id::new("ruler-left"), Sense::drag());
    let cc = st.canvases.entry(id).or_default();
    if rt.drag_started() {
        cc.guide_drag = Some(true);
    }
    if rl.drag_started() {
        cc.guide_drag = Some(false);
    }
    if let Some(horizontal) = cc.guide_drag {
        if let Some(pp) = ui.ctx().pointer_interact_pos() {
            let dpt = view.screen_to_doc((pp.x - area.left()) as f64, (pp.y - area.top()) as f64);
            if horizontal {
                ui.painter()
                    .hline(area.x_range(), pp.y, Stroke::new(1.0, c.guide_default));
            } else {
                ui.painter()
                    .vline(pp.x, area.y_range(), Stroke::new(1.0, c.guide_default));
            }
            if area.contains(pp) {
                let v = if horizontal {
                    dpt.y.round()
                } else {
                    dpt.x.round()
                };
                label_at(
                    ui.painter(),
                    &theme,
                    pp + vec2(10.0, 10.0),
                    &format!("{v:.0}"),
                );
            }
        }
        if ui.input(|i| i.pointer.any_released()) {
            cc.guide_drag = None;
            if let Some(pp) = ui.ctx().pointer_interact_pos()
                && area.contains(pp)
            {
                let dpt =
                    view.screen_to_doc((pp.x - area.left()) as f64, (pp.y - area.top()) as f64);
                let (o, pos) = if horizontal {
                    ("horizontal", dpt.y.round())
                } else {
                    ("vertical", dpt.x.round())
                };
                st.run("guide.add", json!({ "orientation": o, "position": pos }));
            }
        }
    }
}

fn scrollbars(ui: &mut Ui, st: &mut AppState, area: Rect, view: &fp_engine::ViewState, pr: IRect) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let z = view.zoom;
    let (vw, vh) = view.viewport;
    let content_w = pr.width() as f64 * z;
    let content_h = pr.height() as f64 * z;
    let t = SCROLLBAR_T as f32;
    // sichtbarer Bereich ohne die jeweils andere Leiste (Bildlauf endet am Dokumentrand, BV-5)
    let (bar_r, bar_b) = scrollbar_insets(view, pr, SCROLLBAR_T);
    let (vw_vis, vh_vis) = (vw - bar_r, vh - bar_b);
    if content_w > vw + 1.0 {
        let track = Rect::from_min_max(
            pos2(area.left(), area.bottom() - t),
            pos2(area.right() - t, area.bottom()),
        );
        let frac = (vw_vis / content_w).clamp(0.05, 1.0) as f32;
        let pos = (((view.origin.0 - pr.left as f64) * z) / (content_w - vw_vis).max(1.0))
            .clamp(0.0, 1.0) as f32;
        let tw = track.width() * frac;
        let thumb = Rect::from_min_size(
            pos2(track.left() + (track.width() - tw) * pos, track.top() + 1.0),
            vec2(tw, t - 2.0),
        );
        ui.painter()
            .rect_filled(track, 0.0, with_alpha(c.scrollbar_bg, 200));
        ui.painter().rect_filled(thumb, 3.0, c.scrollbar_thumb);
        let resp = ui.interact(track, Id::new("hscroll"), Sense::drag());
        if resp.dragged()
            && let Some(d) = st.session.active_mut()
        {
            let dx = resp.drag_delta().x as f64 / (track.width() - tw).max(1.0) as f64
                * (content_w - vw_vis);
            d.view.pan(-dx, 0.0);
        }
    }
    if content_h > vh + 1.0 {
        let track = Rect::from_min_max(
            pos2(area.right() - t, area.top()),
            pos2(area.right(), area.bottom() - t),
        );
        let frac = (vh_vis / content_h).clamp(0.05, 1.0) as f32;
        let pos = (((view.origin.1 - pr.top as f64) * z) / (content_h - vh_vis).max(1.0))
            .clamp(0.0, 1.0) as f32;
        let th = track.height() * frac;
        let thumb = Rect::from_min_size(
            pos2(
                track.left() + 1.0,
                track.top() + (track.height() - th) * pos,
            ),
            vec2(t - 2.0, th),
        );
        ui.painter()
            .rect_filled(track, 0.0, with_alpha(c.scrollbar_bg, 200));
        ui.painter().rect_filled(thumb, 3.0, c.scrollbar_thumb);
        let resp = ui.interact(track, Id::new("vscroll"), Sense::drag());
        if resp.dragged()
            && let Some(d) = st.session.active_mut()
        {
            let dy = resp.drag_delta().y as f64 / (track.height() - th).max(1.0) as f64
                * (content_h - vh_vis);
            d.view.pan(0.0, -dy);
        }
    }
}

/// Pixel image of the "Exit Bitmap Mode" symbol, 10 x 10 px without anti-aliasing (RE spec 71
/// §1.1): `O` circle, `x` cross.
const EXIT_BITMAP_MODE_PIXELS: [&str; 10] = [
    "...OOOO...",
    "..OOOOOO..",
    ".OxxOOxxO.",
    "OOxxxxxxOO",
    "OOOxxxxOOO",
    "OOOxxxxOOO",
    "OOxxxxxxOO",
    ".OxxOOxxO.",
    "..OOOOOO..",
    "...OOOO...",
];

/// Circle and cross colour of the "Exit Bitmap Mode" symbol (RE spec 71 §1.1): red #CC0000 with a
/// white cross in bitmap mode, otherwise grey #BFBFBF with #F4F4F4.
pub(crate) fn exit_bitmap_mode_colors(on: bool) -> (Color32, Color32) {
    if on {
        (Color32::from_rgb(0xcc, 0, 0), Color32::WHITE)
    } else {
        (
            Color32::from_rgb(0xbf, 0xbf, 0xbf),
            Color32::from_rgb(0xf4, 0xf4, 0xf4),
        )
    }
}

/// The status-bar button "Exit Bitmap Mode": no frame, no hover change, the 10 x 10 px symbol in
/// whole pixels; disabled (a click does nothing) outside bitmap mode.
fn exit_bitmap_mode_button(ui: &mut Ui, size: egui::Vec2, on: bool, tip: &str) -> egui::Response {
    let sense = if on { Sense::click() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(size, sense);
    if ui.is_rect_visible(rect) {
        let ppp = ui.ctx().pixels_per_point();
        let px = 1.0 / ppp;
        let side = 10.0 * px;
        let o = pos2(
            ((rect.center().x - side / 2.0) * ppp).round() / ppp,
            ((rect.center().y - side / 2.0) * ppp).round() / ppp,
        );
        let (circle, cross) = exit_bitmap_mode_colors(on);
        let painter = ui.painter();
        for (y, row) in EXIT_BITMAP_MODE_PIXELS.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                let c = match ch {
                    'O' => circle,
                    'x' => cross,
                    _ => continue,
                };
                let p = o + vec2(x as f32 * px, y as f32 * px);
                painter.rect_filled(Rect::from_min_size(p, vec2(px, px)), 0.0, c);
            }
        }
    }
    crate::ui_kit::icon_button_info(&resp, on, tip, None);
    if tip.is_empty() {
        resp
    } else {
        resp.on_hover_text(tip)
    }
}

// ---------------------------------------------------------------------- Fußleiste

/// Auswahlinfo links in der Fußleiste (Typtexte 10922–10929 bzw. Exportformat + „(Dokument)“).
fn selection_info(st: &AppState) -> String {
    let Some(d) = st.session.active() else {
        return String::new();
    };
    if d.bitmap_mode {
        return fp_i18n::t("ui-sel-bitmap-mode");
    }
    let n = d.selection.elements.len();
    if n == 0 {
        // wie im Original: Exportformat der Optimieren-Einstellung der Seite („GIF (Dokument)“)
        let s = fp_export::doc_export::page_settings(&d.doc, d.page);
        return fp_i18n::tr!(
            "ui-sel-document",
            format = s.format.display_name().to_string()
        );
    }
    if n > 1 {
        return fp_i18n::tr!("ui-sel-objects", n = n);
    }
    let Some(e) = d
        .selection
        .elements
        .first()
        .and_then(|id| fp_engine::docops::element_on_page(&d.doc, d.page, *id))
    else {
        return fp_i18n::t("ui-sel-object");
    };
    use fp_model::ElementKind as K;
    fp_i18n::t(match &e.kind {
        K::Bitmap(_) => "ui-sel-bitmap",
        K::Path(p) if p.primitive.is_some() => "ui-sel-rectangle",
        K::Path(_) => "ui-sel-path",
        K::Group(_) => "ui-sel-group",
        K::CompoundShape(_) => "ui-sel-compound",
        K::Text(_) => "ui-sel-text",
        K::Instance(_) => "ui-sel-symbol",
        K::WebObject(_) => "ui-sel-slice",
        _ => "ui-sel-object",
    })
}

/// FB-31 (Firepixel extension): status text of a pausable bitmap stroke – the hint "press P"
/// while a stroke is painted, the instructions while it is paused; `None` without a stroke. With
/// `alt` held a Brush or Pencil stroke announces its subtraction mode (FB-69).
pub fn stroke_pause_status(st: &AppState, alt: bool) -> Option<String> {
    let subtract = (alt || st.session.tool_stroke_subtracting())
        && matches!(
            st.session.tools.active,
            fp_engine::tools::ToolId::Brush | fp_engine::tools::ToolId::Pencil
        );
    match st.session.tool_stroke_pause() {
        fp_engine::StrokePause::Painting if subtract => Some(fp_i18n::t("ui-stroke-subtract")),
        fp_engine::StrokePause::Paused if subtract => Some(fp_i18n::t("ui-stroke-subtract-paused")),
        fp_engine::StrokePause::Painting => Some(fp_i18n::t("ui-stroke-pause-hint")),
        fp_engine::StrokePause::Paused => Some(fp_i18n::t("ui-stroke-paused")),
        fp_engine::StrokePause::Idle => None,
    }
}

/// Text on the left of the status line at time `now` (`alt` = Alt held): the stroke pause text
/// (FB-31, FB-69), otherwise a recent message, the canvas info or the selection info.
pub fn footer_status(st: &AppState, now: f64, alt: bool) -> String {
    if let Some(s) = stroke_pause_status(st, alt) {
        return s;
    }
    let info = st
        .session
        .active_id()
        .and_then(|id| st.canvases.get(&id))
        .and_then(|c| c.info.clone());
    match (&st.status, info) {
        (Some((s, t)), _) if now - t < AppState::STATUS_SECONDS => s.clone(),
        (_, Some(i)) => i,
        _ => selection_info(st),
    }
}

fn footer_ui(ui: &mut Ui, st: &mut AppState, rect: Rect) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    ui.painter().rect_filled(rect, 0.0, c.frame_bg);
    let now = ui.input(|i| i.time);
    let alt = ui.input(|i| i.modifiers.alt);
    let status = footer_status(st, now, alt);
    let Some(d) = st.session.active() else { return };
    let (frames, frame) = (d.frame_count(), d.frame);
    let (w, h) = d
        .doc
        .pages
        .get(d.page)
        .map(|p| (p.canvas.width, p.canvas.height))
        .unwrap_or((0.0, 0.0));
    let zoom = d.view.zoom;
    let bitmap_mode = d.bitmap_mode;
    // rechts: Zoom, Größe, Status-Steuerung
    let zr = Rect::from_min_max(
        pos2(rect.right() - 58.0, rect.top() + 1.0),
        pos2(rect.right() - 2.0, rect.bottom() - 1.0),
    );
    let sr = Rect::from_min_max(
        pos2(zr.left() - 75.0, zr.top()),
        pos2(zr.left() - 2.0, zr.bottom()),
    );
    let p = ui.painter();
    for r in [zr, sr] {
        p.rect_filled(r, 0.0, c.frame_bg);
        p.rect_stroke(r, 0.0, Stroke::new(1.0, c.line_light), StrokeKind::Inside);
    }
    p.text(
        sr.center(),
        Align2::CENTER_CENTER,
        format!("{w:.0} x {h:.0}"),
        theme.small_font(),
        c.text,
    );
    // Anzeige wie im Original mit `%4.f%%` (rechtsbündig vierstellig, z. B. „ 100%“)
    p.text(
        pos2(zr.left() + 6.0, zr.center().y),
        Align2::LEFT_CENTER,
        format!("{:4.0}%", zoom * 100.0),
        theme.small_font(),
        c.text,
    );
    widgets::drop_arrow(
        p,
        Rect::from_center_size(pos2(zr.right() - 9.0, zr.center().y + 2.0), vec2(8.0, 8.0)),
        c.text,
    );
    let zresp = ui
        .interact(zr, Id::new("footer-zoom"), Sense::click())
        .on_hover_text(fp_i18n::t("ui-zoom"));
    let nodes: Vec<crate::menu::Node> = ZOOM_STEPS
        .iter()
        .filter(|z| ![0.16, 0.33, 6.0].contains(*z))
        .map(|z| crate::menu::Node::Item {
            label: format!(
                "{:.0}%",
                if *z == 0.0625 {
                    6.0
                } else if *z == 0.125 {
                    12.0
                } else {
                    z * 100.0
                }
            ),
            shortcut: None,
            action: crate::menu::ItemAction::Ui(Action::SetZoom(*z)),
            enabled: true,
            checked: Some(zoom == *z),
        })
        .chain([
            crate::menu::Node::Separator,
            crate::menu::Node::Item {
                label: fp_i18n::t("ui-zoom-fit-selection"),
                shortcut: None,
                action: crate::menu::ItemAction::Command("view.fitSelection".into()),
                enabled: st.session.is_available("view.fitSelection"),
                checked: None,
            },
            crate::menu::Node::Item {
                label: fp_i18n::t("ui-zoom-fit-all"),
                shortcut: None,
                action: crate::menu::ItemAction::Command("view.fitAll".into()),
                enabled: true,
                checked: None,
            },
        ])
        .collect();
    let mut acts = Vec::new();
    crate::menu::popup_menu(ui, &zresp, &nodes, &mut acts);
    for a in acts {
        st.push(a);
    }
    // Status-Steuerung (Reihenfolge Laufzeit T3): Erster, Wiedergeben, Letzter, Nummer, Vorheriger, Nächster, Bitmapmodus beenden
    let bsize = vec2(20.0, 16.0);
    let mut x = sr.left() - 6.0 - 7.0 * 22.0 - 16.0;
    let y = rect.center().y - bsize.y / 2.0;
    let multi = frames > 1;
    let playing = st.session.ui.playing;
    let items: [(Icon, String, bool, Option<Action>); 3] = [
        (
            Icon::FirstFrame,
            fp_i18n::t("ui-state-first"),
            multi && frame > 0,
            Some(Action::SetFrame(0)),
        ),
        // FB-63: Play runs View ▸ Play/Stop; while the animation plays the button shows Stop
        if playing {
            (
                Icon::Stop,
                fp_i18n::t("ui-state-stop"),
                true,
                Some(Action::Run(
                    "view.togglePlayback".into(),
                    serde_json::json!({}),
                )),
            )
        } else {
            (
                Icon::Play,
                fp_i18n::t("ui-state-play"),
                multi,
                Some(Action::Run(
                    "view.togglePlayback".into(),
                    serde_json::json!({}),
                )),
            )
        },
        (
            Icon::LastFrame,
            fp_i18n::t("ui-state-last"),
            multi && frame + 1 < frames,
            Some(Action::SetFrame(frames.saturating_sub(1))),
        ),
    ];
    for (icon, tip, en, act) in items {
        let r = Rect::from_min_size(pos2(x, y), bsize);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r));
        if widgets::icon_button(
            &mut child,
            icon,
            bsize,
            11.0,
            ButtonLook {
                selected: icon == Icon::Stop,
                enabled: en,
                group: Some(ToolGroup::View),
            },
            &tip,
        )
        .clicked()
            && let Some(a) = act
        {
            st.push(a);
        }
        x += 22.0;
    }
    ui.painter().text(
        pos2(x + 14.0, rect.center().y),
        Align2::CENTER_CENTER,
        format!("{}", frame + 1),
        theme.small_font(),
        c.text,
    );
    x += 32.0;
    let items2: [(Icon, String, bool, Option<Action>); 3] = [
        (
            Icon::PrevFrame,
            fp_i18n::t("ui-state-prev"),
            multi && frame > 0,
            Some(Action::SetFrame(frame.saturating_sub(1))),
        ),
        (
            Icon::NextFrame,
            fp_i18n::t("ui-state-next"),
            multi && frame + 1 < frames,
            Some(Action::SetFrame(frame + 1)),
        ),
        (
            Icon::ExitBitmapMode,
            fp_i18n::t("ui-exit-bitmap-mode"),
            bitmap_mode,
            // one row "Exit Bitmap Mode" also without work in the mode; the tool stays (RE spec
            // 71 §1.4)
            Some(Action::Run(
                "modify.exitBitmapMode".into(),
                json!({"row": true}),
            )),
        ),
    ];
    for (icon, tip, en, act) in items2 {
        let r = Rect::from_min_size(pos2(x, y), bsize);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r));
        let resp = if icon == Icon::ExitBitmapMode {
            exit_bitmap_mode_button(&mut child, bsize, en, &tip)
        } else {
            widgets::icon_button(
                &mut child,
                icon,
                bsize,
                11.0,
                ButtonLook {
                    selected: false,
                    enabled: en,
                    group: Some(ToolGroup::View),
                },
                &tip,
            )
        };
        if resp.clicked()
            && let Some(a) = act
        {
            st.push(a);
        }
        x += 22.0;
    }
    // links: Auswahlinfo bzw. Meldung
    let left_w =
        (rect.width() - (rect.right() - (sr.left() - 6.0 - 7.0 * 22.0 - 16.0)) - 12.0).max(20.0);
    widgets::text_clipped(
        ui.painter(),
        pos2(rect.left() + 6.0, rect.center().y),
        Align2::LEFT_CENTER,
        &status,
        theme.small_font(),
        c.status_text,
        left_w,
    );
}

#[cfg(test)]
mod event_time_tests {
    use super::event_times_ms;

    #[test]
    fn events_of_a_frame_are_spread_over_the_frame_time() {
        // 8 Bewegungen in einem Bild 48 ms nach dem letzten: je 6 ms
        assert_eq!(
            event_times_ms(Some(1.0), 1.048, 8),
            vec![1006, 1012, 1018, 1024, 1030, 1036, 1042, 1048]
        );
        // ein Ereignis: Bildzeit; kein früheres Bild: alle Bildzeit
        assert_eq!(event_times_ms(Some(1.0), 1.016, 1), vec![1016]);
        assert_eq!(event_times_ms(None, 2.0, 3), vec![2000, 2000, 2000]);
        // nach langer Pause höchstens 100 ms zurück
        assert_eq!(event_times_ms(Some(0.0), 5.0, 2), vec![4950, 5000]);
    }
}

//! Dokumentbereich (11 §4.4–§4.8): Dokument-Reiter, Ansichtsreiter Original/Vorschau/2-fach/4-fach
//! mit Seitenauswahl, Leinwand (Rendern über fp-render, Zoom, Bildlauf, Lineale, Raster,
//! Hilfslinien, Schachbrett, Auswahlrahmen und Werkzeug-Vorschau) und die Fußleiste
//! (Auswahlinfo, Status-Steuerung, Dokumentgröße, Vergrößerung). Mausereignisse gehen als
//! [`fp_engine::ToolEvent`] an die Sitzung.

pub mod context;
pub mod cursors;
pub mod ghost;
pub mod overlay;
pub mod render;
pub mod textedit;
pub mod web;
#[cfg(test)]
mod tests;

use crate::app_state::{Action, AppState};
use crate::icons::{self, Icon, IconStyle};
use crate::theme::{with_alpha, Theme, ToolGroup};
use crate::widgets::{self, ButtonLook};
use egui::{pos2, vec2, Align2, Color32, CursorIcon, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Ui};
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
    let tabs = Rect::from_min_size(full.min, vec2(full.width(), m.doc_tab_height));
    doc_tabs(ui, st, tabs);
    // geschlossene Dokumente: Leinwand-Cache und Kachel-Cache des Renderers freigeben
    let open: Vec<fp_engine::DocId> = st.session.documents().iter().map(|d| d.id).collect();
    let before = st.canvases.len();
    st.canvases.retain(|id, _| open.contains(id));
    if st.canvases.len() < before {
        st.renderer.clear_tiles();
    }
    if st.session.active().is_none() {
        let rest = Rect::from_min_max(pos2(full.left(), tabs.bottom()), full.max);
        // Startbildschirm (DESIGN, `welcome.rs`): zuletzt geöffnet, Neu erstellen, Erweitern
        crate::welcome::show(ui, st, rest);
        return;
    }
    let view_bar = Rect::from_min_size(pos2(full.left(), tabs.bottom()), vec2(full.width(), m.view_bar_height));
    view_bar_ui(ui, st, view_bar);
    let footer = Rect::from_min_max(pos2(full.left(), full.bottom() - m.footer_height), full.max);
    let canvas = Rect::from_min_max(pos2(full.left(), view_bar.bottom()), pos2(full.right(), footer.top()));
    let preview = st.session.active().is_some_and(|d| d.view.preview_mode != PreviewMode::Original);
    if preview {
        // Vorschau/2-fach/4-fach: Exportbild (nicht bearbeitbar); die Originalkachel ist die Leinwand
        crate::preview::show(ui, st, canvas, &mut |ui, st, r| canvas_ui(ui, st, r));
    } else {
        canvas_ui(ui, st, canvas);
    }
    footer_ui(ui, st, footer);
}

// ---------------------------------------------------------------------- Reiter

/// Kontextmenü eines Dokumentreiters wie im Original: Alles hier zusammenführen · Schließen ·
/// Alle schließen · In neues Fenster verschieben │ Neues Dokument · Dokument öffnen. Firepixel hat
/// nur ein Dokumentfenster: „Zusammenführen“ und „In neues Fenster“ sind daher (wie im Original
/// bei einem Fenster) gesperrt.
pub fn tab_menu(id: fp_engine::DocId, _docs: usize) -> Vec<crate::menu::Node> {
    use crate::menu::{ItemAction, Node};
    let item = |key: &str, action: ItemAction, enabled: bool| Node::Item { label: fp_i18n::t(key), shortcut: None, action, enabled, checked: None };
    vec![
        item("ui-tab-merge-all", ItemAction::None, false),
        item("ui-tab-close", ItemAction::Ui(Action::CloseDocument(id)), true),
        item("ui-tab-close-all", ItemAction::Command("file.closeAll".into()), true),
        item("ui-tab-move-to-new-window", ItemAction::None, false),
        Node::Separator,
        item("ui-tab-new-document", ItemAction::Command("file.new".into()), true),
        item("ui-tab-open-document", ItemAction::Command("file.open".into()), true),
    ]
}

fn doc_tabs(ui: &mut Ui, st: &mut AppState, rect: Rect) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    if theme.gradients {
        widgets::vgradient(ui.painter(), rect, c.doc_tab_bar_top, c.doc_tab_bar_bottom);
        ui.painter().hline(rect.x_range(), rect.top() + 0.5, Stroke::new(1.0, c.header_highlight));
    } else {
        ui.painter().rect_filled(rect, 0.0, c.doc_tab_bar_top);
    }
    ui.painter().hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, c.doc_tab_border));
    let active = st.session.active_id();
    let docs: Vec<(fp_engine::DocId, String, bool)> = st.session.documents().iter().map(|d| (d.id, d.title.clone(), d.is_dirty())).collect();
    let font = theme.bold(theme.fonts.ui);
    // Überlauf wie im Original: passen nicht alle Reiter, steht rechts „»“ mit „Weitere Dokumente“;
    // der aktive Reiter bleibt immer sichtbar.
    let labels: Vec<String> = docs.iter().map(|(_, t, dirty)| format!("{}{}", if *dirty { "*" } else { "" }, t)).collect();
    let widths: Vec<f32> = labels.iter().map(|l| (widgets::text_width(ui, l, &font) + 34.0).min(260.0)).collect();
    let more_w = 24.0;
    let total: f32 = widths.iter().map(|w| w + 1.0).sum();
    let avail = rect.width() - 8.0;
    let overflow = total > avail;
    let room = if overflow { avail - more_w } else { avail };
    let active_i = docs.iter().position(|(id, _, _)| active == Some(*id)).unwrap_or(0);
    let mut start = 0;
    while start < active_i && widths[start..=active_i].iter().map(|w| w + 1.0).sum::<f32>() > room {
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
            ui.painter().rect_stroke(tr, 0.0, Stroke::new(1.0, c.doc_tab_border), StrokeKind::Inside);
            ui.painter().hline(tr.x_range().shrink(1.0), tr.bottom() - 0.5, Stroke::new(1.0, c.doc_tab_active_bg));
            if theme.glow {
                let accent = theme.colors.group_accent[i % 6];
                ui.painter().rect_filled(Rect::from_min_size(tr.min, vec2(tr.width(), 2.0)), 0.0, accent);
            }
        } else if resp.hovered() {
            ui.painter().rect_filled(tr.shrink(1.0), 0.0, c.tab_hover_bg);
        }
        if theme.glow && !is_active {
            ui.painter().hline(tr.x_range().shrink(6.0), tr.bottom() - 2.5, Stroke::new(1.0, with_alpha(theme.colors.group_accent[i % 6], 150)));
        }
        widgets::text_clipped(ui.painter(), pos2(tr.left() + 8.0, tr.center().y), Align2::LEFT_CENTER, &label, font.clone(), c.doc_tab_text, w - 30.0);
        let xr = Rect::from_center_size(pos2(tr.right() - 11.0, tr.center().y), vec2(12.0, 12.0));
        let xresp = ui.interact(xr, Id::new(("doc-tab-x", id.0)), Sense::click());
        let xc = if xresp.hovered() { c.icon_red } else { c.text_weak };
        icons::paint(ui.painter(), xr.shrink(1.0), Icon::Close, &IconStyle::mono(xc, c.frame_bg));
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
        let mr = Rect::from_min_size(pos2(rect.right() - more_w - 2.0, rect.top() + 3.0), vec2(more_w, rect.height() - 5.0));
        let mresp = ui.interact(mr, Id::new("doc-tabs-more"), Sense::click()).on_hover_text(fp_i18n::t("ui-tab-more"));
        if mresp.hovered() {
            ui.painter().rect_filled(mr, 0.0, c.tab_hover_bg);
        }
        ui.painter().text(mr.center(), Align2::CENTER_CENTER, "»", font.clone(), c.doc_tab_text);
        let nodes: Vec<crate::menu::Node> = docs
            .iter()
            .enumerate()
            .filter(|(i, _)| !visible[*i])
            .map(|(i, (id, _, _))| crate::menu::Node::Item { label: labels[i].clone(), shortcut: None, action: crate::menu::ItemAction::Ui(Action::Activate(*id)), enabled: true, checked: None })
            .collect();
        crate::menu::popup_menu(ui, &mresp, &nodes, &mut tab_actions);
    }
    for a in tab_actions {
        st.push(a);
    }
    ui.ctx().data_mut(|d| d.insert_temp(Id::new("lw-doc-tabs"), tab_rects));
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
    let r = st.session.run_group(&fp_i18n::t("ui-history-paste-objects"), |s| {
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
        (PreviewMode::Original, Icon::ViewOriginal, "ui-view-original"),
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
        widgets::button_frame(ui.painter(), r, &resp, ButtonLook { selected, enabled: true, group: Some(ToolGroup::View) }, &theme);
        icons::paint(ui.painter(), Rect::from_center_size(pos2(r.left() + 11.0, r.center().y), vec2(14.0, 14.0)), *icon, &IconStyle::new(&theme, Some(ToolGroup::View), true));
        ui.painter().text(pos2(r.left() + 20.0, r.center().y), Align2::LEFT_CENTER, label, font.clone(), c.text);
        if resp.on_hover_text(tip).clicked() {
            chosen = Some(*m);
        }
        x += w + 2.0;
        if i == 0 {
            ui.painter().vline(x + 0.5, (rect.top() + 5.0)..=(rect.bottom() - 5.0), Stroke::new(1.0, c.line_mid));
            ui.painter().vline(x + 1.5, (rect.top() + 5.0)..=(rect.bottom() - 5.0), Stroke::new(1.0, c.line_light));
            x += 4.0;
        }
    }
    ui.painter().vline(x + 2.5, (rect.top() + 5.0)..=(rect.bottom() - 5.0), Stroke::new(1.0, c.line_mid));
    if let Some(m) = chosen
        && let Some(d) = st.session.active_mut()
    {
        d.view.preview_mode = m;
    }
    // Seitenauswahl rechts
    let Some(d) = st.session.active() else { return };
    let names: Vec<String> = d.doc.pages.iter().enumerate().map(|(i, p)| if p.display_name().is_empty() { fp_i18n::tr!("ui-page-default-name", n = i + 1) } else { p.display_name().to_string() }).collect();
    let page = d.page;
    let nav = Rect::from_center_size(pos2(rect.right() - 20.0, rect.center().y), vec2(18.0, 18.0));
    // Seitenauswahl rechts, bei schmaler Leinwand nach links begrenzt bzw. ausgeblendet (nichts überlappt)
    let combo = Rect::from_min_max(pos2((rect.right() - 236.0).max(x + 14.0), rect.top() + 3.0), pos2(nav.left() - 8.0, rect.bottom() - 3.0));
    ui.painter().vline(nav.left() - 4.5, (rect.top() + 5.0)..=(rect.bottom() - 5.0), Stroke::new(1.0, c.line_mid));
    if combo.width() >= 90.0 {
        ui.painter().vline(combo.left() - 6.5, (rect.top() + 5.0)..=(rect.bottom() - 5.0), Stroke::new(1.0, c.line_mid));
        icons::paint(ui.painter(), Rect::from_center_size(pos2(combo.left() + 8.0, combo.center().y), vec2(13.0, 13.0)), Icon::PageSmall, &IconStyle::new(&theme, None, false));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(pos2(combo.left() + 18.0, combo.top()), combo.max)));
        child.set_clip_rect(combo);
        let mut sel = page;
        egui::ComboBox::from_id_salt("page-select").width(combo.width() - 26.0).truncate().selected_text(names.get(page).cloned().unwrap_or_default()).show_ui(&mut child, |ui| {
            for (i, n) in names.iter().enumerate() {
                ui.selectable_value(&mut sel, i, n.as_str());
            }
        });
        if sel != page {
            st.push(Action::SetPage(sel));
        }
    }
    let resp = ui.interact(nav, Id::new("page-nav"), Sense::click());
    widgets::button_frame(ui.painter(), nav, &resp, ButtonLook { selected: false, enabled: names.len() > 1, group: Some(ToolGroup::View) }, &theme);
    icons::paint(ui.painter(), nav.shrink(2.0), Icon::PageNavigate, &IconStyle::new(&theme, Some(ToolGroup::View), names.len() > 1));
    if resp.on_hover_text(fp_i18n::t("ui-page-nav")).clicked() && names.len() > 1 {
        st.push(Action::SetPage((page + 1) % names.len()));
    }
}

// ---------------------------------------------------------------------- Leinwand

/// Seitenrechteck in Dokumentkoordinaten.
fn page_rect(doc: &fp_model::Document, page: usize) -> Option<IRect> {
    fp_render::Renderer::page_rect(doc, page)
}

/// Ansicht so begrenzen, dass die Seite im Bild bleibt; kleinere Seiten werden zentriert.
fn clamp_view(view: &mut fp_engine::ViewState, pr: IRect) {
    let z = view.zoom.max(1e-6);
    let (vw, vh) = view.viewport;
    let (pw, ph) = (pr.width() as f64, pr.height() as f64);
    let margin = 60.0 / z;
    if pw * z <= vw {
        view.origin.0 = pr.left as f64 + pw / 2.0 - vw / 2.0 / z;
    } else {
        view.origin.0 = view.origin.0.clamp(pr.left as f64 - margin, pr.right as f64 - vw / z + margin);
    }
    if ph * z <= vh {
        view.origin.1 = pr.top as f64 + ph / 2.0 - vh / 2.0 / z;
    } else {
        view.origin.1 = view.origin.1.clamp(pr.top as f64 - margin, pr.bottom as f64 - vh / z + margin);
    }
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
        let (o, edge) = if axis == 0 { (view.origin.0, pr.left as f64) } else { (view.origin.1, pr.top as f64) };
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
    Modifiers { shift: i.modifiers.shift, alt: i.modifiers.alt, ctrl: i.modifiers.ctrl || i.modifiers.command, space, caps: false }
}

fn canvas_ui(ui: &mut Ui, st: &mut AppState, rect: Rect) {
    let theme = Theme::current(ui.ctx());
    let c = theme.colors.clone();
    let Some(d) = st.session.active() else { return };
    let id = d.id;
    let show_rulers = d.view.show_rulers;
    let ruler = if show_rulers { theme.metrics.ruler_size } else { 0.0 };
    let area = Rect::from_min_max(rect.min + vec2(ruler, ruler), rect.max);
    // Hintergrund mit Schatten unter der Ansichtsleiste wie CS6
    ui.painter().rect_filled(rect, 0.0, c.canvas_bg);
    if theme.gradients {
        for k in 0..5 {
            let col = crate::theme::mix(c.frame_bg, c.canvas_bg, k as f32 / 4.0);
            ui.painter().hline(rect.x_range(), rect.top() + k as f32 + 0.5, Stroke::new(1.0, col));
        }
    }
    let ppp = ui.ctx().pixels_per_point() as f64;
    // Sichtbereich melden, Erst-Einpassung, Begrenzung
    let needs_fit = st.canvases.entry(id).or_default().needs_initial_fit;
    if let Some(d) = st.session.active_mut() {
        d.view.viewport = (area.width().max(1.0) as f64, area.height().max(1.0) as f64);
        if let Some(pr) = page_rect(&d.doc, d.page) {
            if needs_fit {
                if pr.width() as f64 > d.view.viewport.0 || pr.height() as f64 > d.view.viewport.1 {
                    d.view.fit_rect(fp_geom::Rect::new(pr.left as f32, pr.top as f32, pr.right as f32, pr.bottom as f32));
                } else {
                    d.view.zoom = 1.0;
                }
            }
            clamp_view(&mut d.view, pr);
            align_view_to_pixels(&mut d.view, pr, area.min, ppp);
        }
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
        clamp_view(&mut d.view, pr);
        align_view_to_pixels(&mut d.view, pr, area.min, ppp);
    }
    // Rückmeldung des Werkzeugs (auch ohne Ereignis) und im Hintergrund auszublendende Elemente
    let feedback = st.session.tool_feedback();
    let preview_drag = st.session.prefs.get_bool("PreviewDrag", true);
    let mut hidden: Vec<u64> = Vec::new();
    for o in &feedback {
        match o {
            Overlay::Ghost { ids, copy: false, .. } if preview_drag => hidden.extend(ids.iter().copied()),
            Overlay::TextEdit(v) => hidden.extend(v.target),
            _ => {}
        }
    }
    hidden.sort_unstable();
    hidden.dedup();
    let variant = render::hidden_variant(&hidden);
    // Zeichnen
    let Some(d) = st.session.active() else { return };
    let view = d.view.clone();
    let Some(pr) = page_rect(&d.doc, d.page) else { return };
    let painter = ui.painter_at(area);
    let bg = d.doc.pages.get(d.page).map(|p| p.root.background).unwrap_or(fp_color::Argb(0xffffffff));
    // Rendern: sichtbarer Ausschnitt plus Rand
    let vis = view.visible_rect();
    let want = IRect::new(vis.left.floor() as i32, vis.top.floor() as i32, vis.right.ceil() as i32, vis.bottom.ceil() as i32);
    let visible = want.intersect(&pr);
    let zoom_dev = view.zoom * ppp;
    let (generation, page, frame, docarc) = (d.generation, d.page, d.frame, std::sync::Arc::clone(&d.doc));
    let cc = st.canvases.entry(id).or_default();
    // Ergebnisse des Threads übernehmen
    let fresh = |k: &RenderKey| k.doc == id && k.generation == generation && k.epoch == cc.epoch && k.page == page && k.frame == frame && k.variant == variant && (k.zoom - zoom_dev).abs() < 1e-9;
    let covered = cc.shown.as_ref().is_some_and(|k| fresh(k) && k.region.intersect(&visible) == visible);
    let pending_ok = cc.requested.as_ref().is_some_and(|k| fresh(k) && k.region.intersect(&visible) == visible);
    if !visible.is_empty() && !covered && !pending_ok {
        // Rand: halber Sichtbereich je Seite, Textur höchstens 8000 Gerätepixel
        let mx = ((view.viewport.0 / view.zoom) * 0.5) as i32;
        let my = ((view.viewport.1 / view.zoom) * 0.5) as i32;
        let mut region = IRect::new(visible.left - mx, visible.top - my, visible.right + mx, visible.bottom + my).intersect(&pr);
        // höchstens die größte Texturseite des Grafikadapters (sonst Absturz beim Hochladen)
        let max_tex = (ui.ctx().input(|i| i.max_texture_side) as f64).min(8000.0);
        let max_side = ((max_tex / zoom_dev).floor() as i32).max(1);
        if region.width() as i32 > max_side || region.height() as i32 > max_side {
            region = visible;
        }
        if region.width() as i32 > max_side || region.height() as i32 > max_side {
            // sehr großes Fenster bei hoher Vergrößerung: um die Mitte des Sichtbereichs begrenzen
            let cx = (visible.left + visible.right) / 2;
            let cy = (visible.top + visible.bottom) / 2;
            let hw = (region.width() as i32).min(max_side) / 2;
            let hh = (region.height() as i32).min(max_side) / 2;
            region = IRect::new(cx - hw, cy - hh, cx - hw + (region.width() as i32).min(max_side), cy - hh + (region.height() as i32).min(max_side));
        }
        let key = RenderKey { doc: id, generation, epoch: cc.epoch, page, frame, region, zoom: zoom_dev, variant };
        cc.requested = Some(key);
        if let Some(res) = st.worker.request(RenderJob { key, doc: docarc, hidden: hidden.clone() }, st.sync_render) {
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
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
    }
    let tex_now = match (&cc.texture, &cc.shown) {
        (Some(tex), Some(k)) if k.doc == id && k.page == page => Some((tex.clone(), *k)),
        _ => None,
    };
    let ppf = ppp as f32;
    let fresh_tex = tex_now.as_ref().filter(|(_, k)| (k.zoom - zoom_dev).abs() < 1e-9);
    // Ursprung des Abtastgitters (Gerätepixel, ganzzahlig) und Bezugs-Dokumentpunkt
    let (gx, gy, rl, rt) = match fresh_tex {
        Some((_, k)) => {
            let a = to_screen(&view, area.min, k.region.left as f64, k.region.top as f64);
            ((a.x * ppf).round() as f64, (a.y * ppf).round() as f64, k.region.left as f64, k.region.top as f64)
        }
        None => {
            let a = to_screen(&view, area.min, pr.left as f64, pr.top as f64);
            ((a.x * ppf).round() as f64, (a.y * ppf).round() as f64, pr.left as f64, pr.top as f64)
        }
    };
    // erster Gerätepixel i (ab dem Gitterursprung), dessen Abtastpunkt bei oder hinter der Kante `e`
    // liegt – mit genau den Gleitkomma-Schritten von fp-render (sonst kippt die letzte Zeile)
    let edge = |g: f64, r: f64, e: f64| {
        let (r, e) = (r as i64, e as i64);
        let sample = |i: i64| r + ((i as f64 + 0.5) / zoom_dev).floor() as i64;
        let mut i = ((e - r) as f64 * zoom_dev - 0.5).ceil() as i64;
        while sample(i - 1) >= e {
            i -= 1;
        }
        while sample(i) < e {
            i += 1;
        }
        g + i as f64
    };
    let page_screen = Rect::from_min_max(
        pos2((edge(gx, rl, pr.left as f64) / ppp) as f32, (edge(gy, rt, pr.top as f64) / ppp) as f32),
        pos2((edge(gx, rl, pr.right as f64) / ppp) as f32, (edge(gy, rt, pr.bottom as f64) / ppp) as f32),
    );
    cc.page_screen = Some(page_screen);
    if theme.kind != crate::theme::ThemeKind::Oled {
        painter.rect_filled(page_screen.translate(vec2(2.0, 2.0)), 0.0, with_alpha(Color32::BLACK, 30));
    }
    if (bg.0 >> 24) < 255 {
        widgets::checker(&painter.with_clip_rect(page_screen.intersect(area)), page_screen.intersect(area), 8.0, &theme);
    }
    if let Some((tex, k)) = &tex_now {
        let page_painter = painter.with_clip_rect(page_screen.intersect(area));
        let rect = if fresh_tex.is_some() {
            // aktuelle Textur: Pixel genau auf dem Gerätepixelgitter
            Rect::from_min_size(pos2((gx / ppp) as f32, (gy / ppp) as f32), vec2(tex.size()[0] as f32 / ppf, tex.size()[1] as f32 / ppf))
        } else {
            // alte Textur mit ihrem eigenen Zoom auf die neue Ansicht abbilden, bis die neue da ist
            let a = to_screen(&view, area.min, k.region.left as f64, k.region.top as f64);
            let size = vec2(tex.size()[0] as f32 / ppf, tex.size()[1] as f32 / ppf) * (view.zoom * ppp / k.zoom.max(1e-9)) as f32;
            Rect::from_min_size(a, size)
        };
        page_painter.image(tex.id(), rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
    }
    if theme.glow {
        painter.rect_stroke(page_screen, 0.0, Stroke::new(1.0, with_alpha(c.focus, 90)), StrokeKind::Outside);
    }
    let spp = |p: Point| to_screen(&view, area.min, p.x as f64, p.y as f64);
    overlay::swatch_feedback(ui.ctx(), st, &painter.with_clip_rect(page_screen.intersect(area)), &spp);
    overlays(ui, st, &painter, area, &view, &feedback);
    if let Some(cc) = st.canvases.get_mut(&id) {
        cc.info = feedback.iter().rev().find_map(|o| match o {
            Overlay::Label { text, .. } => Some(text.clone()),
            _ => None,
        });
        cc.feedback = feedback;
    }
    if show_rulers {
        rulers(ui, st, rect, area, &view);
    }
    scrollbars(ui, st, area, &view, pr);
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

/// Eigener Mauszeiger über der Leinwand (Systemzeiger ausgeblendet).
fn paint_cursor(ui: &Ui, st: &AppState, area: Rect, resp: &egui::Response) {
    let Some(p) = ui.ctx().pointer_hover_pos().filter(|p| area.contains(*p)) else { return };
    let busy = st.session.active_id().and_then(|id| st.canvases.get(&id)).is_some_and(|c| c.pointer_down || c.panning || c.guide_move.is_some());
    if !(resp.hovered() || busy) {
        return;
    }
    let cursor = match st.session.active_id().and_then(|id| st.canvases.get(&id)) {
        Some(c) if c.panning => Cursor::Grabbing,
        Some(c) if c.guide_move.is_some() => if c.guide_move.is_some_and(|g| g.0) { Cursor::GuideH } else { Cursor::GuideV },
        // Strg+Leertaste = vorübergehend Zoom (mit Alt: verkleinern)
        _ if st.space_down && ui.input(|i| i.modifiers.command) => {
            if ui.input(|i| i.modifiers.alt) { Cursor::ZoomOut } else { Cursor::ZoomIn }
        }
        _ if st.space_down => Cursor::Hand,
        _ => st.tool_cursor,
    };
    let theme = Theme::current(ui.ctx());
    let precise = st.session.prefs.get_bool("UsePrecisionCursors", false);
    ui.ctx().set_cursor_icon(CursorIcon::None);
    let painter = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, Id::new("fp-canvas-cursor")));
    cursors::paint(&painter, p, cursor, &theme.colors, precise);
}

/// Punkte (logische Pixel) eines Rad-Ereignisses, die einer Rastung entsprechen (Präzisions-Touchpads
/// und hochauflösende Räder liefern Punkte statt Zeilen; Windows: 120 Einheiten je Rastung).
const POINTS_PER_NOTCH: f32 = 100.0;

/// Rastungen eines Rad-Ereignisses.
fn wheel_notches(unit: egui::MouseWheelUnit, dy: f32) -> f32 {
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
fn take_step(acc: &mut f32) -> i32 {
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
fn tool_result(ctx: &egui::Context, st: &mut AppState, r: fp_engine::Result<fp_engine::tools::ToolResponse>) {
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
    if req.command == "file.exportArea" {
        let rect = crate::canvas::parse_rect(&req.params["rect"]);
        if let (Some(id), Some(r)) = (st.session.active_id(), rect) {
            st.canvases.entry(id).or_default().export_area = Some(r);
        }
        // Export-Dialog wie im Original (der Dialog übernimmt den Bereich aus `export_area`)
        if !crate::dialogs::open_for_command(st, "file.export") {
            st.push(Action::Status(fp_i18n::t("ui-export-area-set")));
        }
    }
}

/// Rechteck aus `{left, top, right, bottom}`.
pub(crate) fn parse_rect(v: &serde_json::Value) -> Option<fp_geom::Rect> {
    let f = |k: &str| v.get(k).and_then(|x| x.as_f64()).map(|x| x as f32);
    Some(fp_geom::Rect::new(f("left")?, f("top")?, f("right")?, f("bottom")?))
}

fn input(ui: &mut Ui, st: &mut AppState, area: Rect, resp: &egui::Response) {
    let Some(d) = st.session.active() else { return };
    let id = d.id;
    let view = d.view.clone();
    let space = st.space_down;
    let (pointer, wheel, zoom_delta, mods_now, primary_pressed, primary_released, middle_down, primary_down, time) = ui.input(|i| {
        (i.pointer.interact_pos(), i.smooth_scroll_delta, i.zoom_delta(), mods(i, space), i.pointer.primary_pressed(), i.pointer.primary_released(), i.pointer.middle_down(), i.pointer.primary_down(), i.time)
    });
    // Leertaste wird wie `st.space_down` nur außerhalb von Texteingaben gezählt
    let space_keys = !ui.ctx().text_edit_focused() && !st.session.tool_is_editing_text();
    let (raw_seq, end_mods) = ui.input(|i| {
        let mut v = Vec::new();
        for e in &i.events {
            match e {
                egui::Event::ModifiersChanged(m) => v.push(RawPtr::Mods(*m)),
                egui::Event::Key { key: egui::Key::Space, pressed, repeat: false, .. } if space_keys => v.push(RawPtr::Space(*pressed)),
                egui::Event::WindowFocused(false) => v.push(RawPtr::Mods(egui::Modifiers::NONE)),
                egui::Event::PointerMoved(p) => v.push(RawPtr::Move(*p)),
                egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers } => v.push(RawPtr::Button(*pos, *pressed, *modifiers)),
                _ => {}
            }
        }
        (v, i.modifiers)
    });
    let hovered = resp.hovered() || resp.dragged();
    let to_doc = |p: Pos2| view.screen_to_doc((p.x - area.left()) as f64, (p.y - area.top()) as f64);
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
                egui::Event::MouseWheel { unit, delta, modifiers, .. } => w.push((*unit, *delta, modifiers.ctrl || modifiers.command, modifiers.shift)),
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
                if steps > 0 { d.view.zoom_in(anchor) } else { d.view.zoom_out(anchor) }
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
        if (dx != 0.0 || dy != 0.0) && let Some(d) = st.session.active_mut() {
            d.view.pan(dx as f64, dy as f64);
        }
        let _ = zoom_wheel;
    }
    let editing = st.session.tool_is_editing_text();
    // Neue Zeigerereignisse nur, wenn die Leinwand selbst unter dem Zeiger liegt (kein Menü, Popup
    // oder Dialog darüber); ein laufendes Ziehen darf immer zu Ende gehen.
    let free = resp.hovered() && !st.dialogs.any_open();
    let pointer_tool = matches!(st.session.tools.active, fp_engine::ToolId::Pointer | fp_engine::ToolId::Subselection | fp_engine::ToolId::SelectBehind);
    let guide_hit = match (pointer, st.session.active()) {
        (Some(p), Some(d)) if free && pointer_tool && area.contains(p) => guide_at(d, area, p),
        _ => None,
    };
    // Leertaste am Anfang dieses Bildes: vor dem ersten Leertasten-Ereignis galt dessen Gegenteil
    let space_start = raw_seq.iter().find_map(|r| if let RawPtr::Space(d) = r { Some(!*d) } else { None }).unwrap_or(space);
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
                RawPtr::Button(q, true, m) if sp && (m.ctrl || m.command) && area.contains(q) => zooms.push((q, m.alt)),
                _ => {}
            }
        }
        if let Some(d) = st.session.active_mut() {
            for (q, out) in zooms {
                let anchor = Some(((q.x - area.left()) as f64, (q.y - area.top()) as f64));
                if out { d.view.zoom_out(anchor) } else { d.view.zoom_in(anchor) }
            }
        }
    }
    if space && mods_now.ctrl && free {
        st.tool_cursor = if mods_now.alt { Cursor::ZoomOut } else { Cursor::ZoomIn };
        return;
    }
    let cc = st.canvases.entry(id).or_default();
    // Hand: Leertaste oder mittlere Taste
    // Leertaste schwenkt nur, wenn sie vor dem Mausdruck gehalten wurde; während eines laufenden
    // Werkzeugvorgangs (Aufziehen) geht sie als Modifikator an das Werkzeug (Form verschieben)
    if (space && !cc.pointer_down && (resp.dragged_by(egui::PointerButton::Primary) || cc.panning)) || (middle_down && hovered) || resp.dragged_by(egui::PointerButton::Middle) {
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
    if cc.pointer_down
        && primary_down
        && let Some(p) = pointer.filter(|p| !area.contains(*p))
        && cc.feedback.iter().any(|o| matches!(o, Overlay::Ghost { .. }))
    {
        let tabs: Vec<(fp_engine::DocId, Rect)> = ctx.data(|d| d.get_temp(Id::new("lw-doc-tabs"))).unwrap_or_default();
        if let Some((other, _)) = tabs.iter().find(|(d, r)| *d != id && r.contains(p)) {
            let grab = cc.press_pos.unwrap_or_default();
            cc.pointer_down = false;
            let elements: Vec<std::sync::Arc<fp_model::Element>> = st
                .session
                .active()
                .map(|d| d.selection.elements.iter().filter_map(|e| fp_engine::docops::element_on_page(&d.doc, d.page, *e)).collect())
                .unwrap_or_default();
            // Ziehen im Quelldokument abbrechen (nichts verschieben)
            let r = st.session.tool_event(&ToolEvent::key(fp_engine::tools::ToolKey::Escape));
            tool_result(&ctx, st, r);
            if !elements.is_empty() {
                ctx.data_mut(|d| d.insert_temp(Id::new(CARRY_ID), Carry { from: id, elements, grab }));
                st.push(Action::Activate(*other));
            }
            return;
        }
    }
    let Some(cc) = st.canvases.get_mut(&id) else { return };
    // Hilfslinie mit dem Zeiger verschieben; ins Lineal (aus der Leinwand) ziehen = löschen
    if let Some((horizontal, index)) = cc.guide_move {
        if primary_released || !primary_down {
            cc.guide_move = None;
            let o = if horizontal { "horizontal" } else { "vertical" };
            match pointer.filter(|p| area.contains(*p)) {
                Some(p) => {
                    let dp = to_doc(p);
                    let pos = if horizontal { dp.y.round() } else { dp.x.round() };
                    st.run("guide.move", json!({"orientation": o, "index": index, "position": pos}));
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
        return;
    }
    if guide_hit.is_some() && !cc.pointer_down {
        st.tool_cursor = if guide_hit.is_some_and(|g| g.0) { Cursor::GuideH } else { Cursor::GuideV };
        return;
    }
    // Werkzeug-Ereignisse
    let t_ms = (time * 1000.0) as u64;
    let mut events: Vec<ToolEvent> = Vec::new();
    let mk = |kind: PointerKind, p: Point| ToolEvent { input: ToolInput::Pointer { kind, pos: p }, mods: mods_now, pressure: 1.0, time_ms: t_ms };
    let caret = |index: usize, extend: bool| ToolEvent { input: ToolInput::TextCaret { index, extend }, mods: mods_now, pressure: 1.0, time_ms: t_ms };
    if let Some(pc) = cc.pending_caret
        && let Some(lc) = cc.text.as_ref()
    {
        events.push(caret(lc.hit(pc), false));
        cc.pending_caret = None;
    }
    if !editing {
        cc.pending_caret = None;
    }
    if let Some(p) = pointer {
        let dp = to_doc(p);
        // Klick/Ziehen im bearbeiteten Text: Caret setzen bzw. markieren (Layout von fp-text)
        let tol = (4.0 / view.zoom.max(1e-6)) as f32;
        let in_text = editing && cc.text.as_ref().is_some_and(|lc| lc.contains(dp, tol));
        if primary_pressed && free && area.contains(p) && !space && in_text {
            if let Some(lc) = cc.text.as_ref() {
                events.push(caret(lc.hit(dp), mods_now.shift));
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
            let tm = |m: egui::Modifiers, space: bool| Modifiers { shift: m.shift, alt: m.alt, ctrl: m.ctrl || m.command, space, caps: false };
            let at = |kind: PointerKind, q: Point, m: Modifiers| ToolEvent { input: ToolInput::Pointer { kind, pos: q }, mods: m, pressure: 1.0, time_ms: t_ms };
            let mut had_button = false;
            for r in &raw_seq {
                match *r {
                    RawPtr::Mods(m) => cur = m,
                    RawPtr::Space(d) => cur_space = d,
                    RawPtr::Move(q) => {
                        let dq = to_doc(q);
                        if (cc.pointer_down || (free && area.contains(q))) && cc.last_pos != Some(dq) {
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
                                let double = cc.last_click.is_some_and(|(t, l)| time - t <= 0.5 && l.distance(q) <= 4.0);
                                cc.last_click = if double { None } else { Some((time, q)) };
                                if double {
                                    events.push(at(PointerKind::DoubleClick, dq, tm(cur, cur_space)));
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
        }
    }
    if let Some(cc) = st.canvases.get_mut(&id) {
        cc.frame_mods = Some(end_mods);
    }
    let ctx = ui.ctx().clone();
    for ev in events {
        let r = st.session.tool_event(&ev);
        // Doppelklick auf Text öffnet den Editor: Einfügemarke an die Klickstelle, sobald das
        // Textlayout da ist (nächstes Bild)
        if ev.kind() == Some(PointerKind::DoubleClick)
            && r.as_ref().is_ok_and(|r| r.switch_tool == Some(fp_engine::ToolId::Text))
            && let Some(cc) = st.canvases.get_mut(&id)
        {
            cc.pending_caret = ev.pos();
            cc.text = None;
        }
        tool_result(&ctx, st, r);
    }
}


fn overlays(ui: &Ui, st: &mut AppState, painter: &egui::Painter, area: Rect, view: &fp_engine::ViewState, feedback: &[Overlay]) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let ctx = ui.ctx().clone();
    let time = ui.input(|i| i.time);
    let Some(d) = st.session.active() else { return };
    let id = d.id;
    let (generation, page_i, frame_i) = (d.generation, d.page, d.frame);
    let page = d.doc.pages.get(d.page);
    let sp = |x: f64, y: f64| to_screen(view, area.min, x, y);
    let spp = |p: Point| to_screen(view, area.min, p.x as f64, p.y as f64);
    // Raster
    if view.show_grid
        && let Some(pg) = page
        && let Some(pr) = page_rect(&d.doc, d.page)
    {
        let g = pg.root.grid.clone().unwrap_or_default();
        let (gx, gy) = (g.size.x.max(1.0) as f64, g.size.y.max(1.0) as f64);
        let col = argb(g.color);
        if gx * view.zoom >= 4.0 {
            let mut x = g.origin.x as f64 + ((pr.left as f64 - g.origin.x as f64) / gx).ceil() * gx;
            while x <= pr.right as f64 {
                painter.vline(sp(x, 0.0).x.round() + 0.5, sp(0.0, pr.top as f64).y..=sp(0.0, pr.bottom as f64).y, Stroke::new(1.0, col));
                x += gx;
            }
        }
        if gy * view.zoom >= 4.0 {
            let mut y = g.origin.y as f64 + ((pr.top as f64 - g.origin.y as f64) / gy).ceil() * gy;
            while y <= pr.bottom as f64 {
                painter.hline(sp(pr.left as f64, 0.0).x..=sp(pr.right as f64, 0.0).x, sp(0.0, y).y.round() + 0.5, Stroke::new(1.0, col));
                y += gy;
            }
        }
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
                painter.vline(sp(x as f64, 0.0).x.round() + 0.5, area.y_range(), Stroke::new(1.0, col));
            }
        }
        for (i, &y) in gd.horizontal.iter().enumerate() {
            if moving_guide != Some((true, i)) {
                painter.hline(area.x_range(), sp(0.0, y as f64).y.round() + 0.5, Stroke::new(1.0, col));
            }
        }
        if let (Some((horizontal, _)), Some(pp)) = (moving_guide, ui.ctx().pointer_interact_pos()) {
            let dp = view.screen_to_doc((pp.x - area.left()) as f64, (pp.y - area.top()) as f64);
            let (line, label) = if horizontal { (sp(0.0, dp.y.round() as f64).y, dp.y.round()) } else { (sp(dp.x.round() as f64, 0.0).x, dp.x.round()) };
            if horizontal {
                painter.hline(area.x_range(), line.round() + 0.5, Stroke::new(1.0, col));
            } else {
                painter.vline(line.round() + 0.5, area.y_range(), Stroke::new(1.0, col));
            }
            if area.contains(pp) {
                label_at(painter, &theme, pp + vec2(10.0, 10.0), &format!("{label:.0}"));
            }
        }
    }
    // Web-Ebene: Segmente/Hotspots und Segment-Hilfslinien (UX-A O17)
    if !view.hide_slices
        && let Some(pr) = page_rect(&d.doc, d.page)
    {
        let items = web::items(&d.doc, d.page, d.frame);
        if !items.is_empty() {
            let pg = fp_geom::Rect::new(pr.left as f32, pr.top as f32, pr.right as f32, pr.bottom as f32);
            let gc = st.session.prefs.get_color("SliceGuideColor", fp_color::Argb(0xffff_4a4a));
            web::paint(painter, &items, pg, view.show_slice_guides, argb(gc), &spp);
        }
    }
    // Auswahl: Rahmen mit Eckgriffen
    // Während des Ziehens wandert der Auswahlrahmen mit der Vorschau (Rechteck → Viereck der Vorschau)
    let moving: Option<(Vec<u64>, fp_geom::Matrix3)> = feedback.iter().find_map(|o| match o {
        Overlay::Ghost { ids, rect, quad, .. } => fp_geom::Matrix3::rect_to_quad(rect, quad).map(|m| (ids.clone(), m)),
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
    if view.show_edges {
        for eid in &d.selection.elements {
            if editing_target == Some(eid.0) {
                continue;
            }
            let Some(e) = fp_engine::docops::element_on_page(&d.doc, d.page, *eid) else { continue };
            if paint_tool && matches!(e.kind, fp_model::ElementKind::Bitmap(_)) {
                continue;
            }
            let Some(b) = fp_engine::docops::element_bounds(&d.doc, &e) else { continue };
            let corners = match moving.as_ref().filter(|(ids, _)| ids.contains(&eid.0)) {
                Some((_, m)) => b.corners().map(|p| m.apply(p)),
                None => b.corners(),
            };
            frames.push(corners);
            if moving.as_ref().is_some_and(|(ids, _)| ids.contains(&eid.0)) {
                let q: Vec<Pos2> = corners.iter().map(|p| spp(*p)).collect();
                painter.add(egui::Shape::closed_line(q.clone(), Stroke::new(1.0, c.selection)));
                for p in q {
                    painter.rect_filled(Rect::from_center_size(p, vec2(theme.metrics.handle_size, theme.metrics.handle_size)), 0.0, c.handle_fill);
                }
                continue;
            }
            let r = Rect::from_min_max(sp(b.left as f64, b.top as f64), sp(b.right as f64, b.bottom as f64));
            let r = Rect::from_min_max(pos2(r.left().round() + 0.5, r.top().round() + 0.5), pos2(r.right().round() - 0.5, r.bottom().round() - 0.5));
            painter.rect_stroke(r, 0.0, Stroke::new(1.0, c.selection), StrokeKind::Middle);
            let hs = theme.metrics.handle_size;
            for p in [r.left_top(), r.right_top(), r.left_bottom(), r.right_bottom()] {
                let h = Rect::from_center_size(p, vec2(hs, hs));
                painter.rect_filled(h, 0.0, c.handle_fill);
                if theme.glow {
                    painter.rect_stroke(h.expand(1.0), 0.0, Stroke::new(1.0, with_alpha(c.handle_fill, 90)), StrokeKind::Outside);
                }
            }
        }
    }
    st.canvases.entry(id).or_default().sel_frames = frames;
    // Pixelauswahl: laufende Ameisen am echten Umriss (auch bei ausgeblendeten Kanten wie CS6)
    if let Some(m) = d.selection.pixels.as_ref().filter(|_| d.selection.has_pixels()) {
        let cc = st.canvases.entry(id).or_default();
        let outlines = overlay::ants_for(&mut cc.ants, m);
        overlay::paint_ants(painter, outlines, &spp, time, c.marquee_a, c.marquee_b, area);
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(120));
    } else if let Some(cc) = st.canvases.get_mut(&id) {
        cc.ants = None;
    }
    // Werkzeug-Rückmeldung
    let Some(d) = st.session.active() else { return };
    let preview_drag = st.session.prefs.get_bool("PreviewDrag", true);
    let stroke = Stroke::new(1.0, c.tool_preview);
    // Gerenderte Vorschauen (Verschieben, Kopie, Transformieren, Text) nur innerhalb der Leinwand
    // wie im fertigen Bild; Rahmen, Griffe und Umrisse dürfen darüber hinaus
    let page_clip = st
        .canvases
        .get(&id)
        .and_then(|c| c.page_screen)
        .or_else(|| page_rect(&d.doc, d.page).map(|pr| Rect::from_min_max(sp(pr.left as f64, pr.top as f64), sp(pr.right as f64, pr.bottom as f64))))
        .map(|r| r.intersect(area))
        .unwrap_or(area);
    let page_painter = painter.with_clip_rect(page_clip);
    let cc = st.canvases.entry(id).or_default();
    let mut had_ghost = false;
    let mut had_text = false;
    for o in feedback.iter().cloned() {
        match o {
            Overlay::Rect { rect, dashed } => {
                let r = Rect::from_min_max(sp(rect.left as f64, rect.top as f64), sp(rect.right as f64, rect.bottom as f64));
                if dashed {
                    dashed_rect(painter, r, c.marquee_a, c.marquee_b, time);
                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(120));
                } else {
                    painter.rect_stroke(r, 0.0, stroke, StrokeKind::Middle);
                }
            }
            Overlay::Ellipse { rect, dashed } => {
                let r = Rect::from_min_max(sp(rect.left as f64, rect.top as f64), sp(rect.right as f64, rect.bottom as f64));
                let pts: Vec<Pos2> = (0..64)
                    .map(|i| {
                        let a = i as f32 / 64.0 * std::f32::consts::TAU;
                        pos2(r.center().x + r.width() / 2.0 * a.cos(), r.center().y + r.height() / 2.0 * a.sin())
                    })
                    .collect();
                if dashed {
                    let pts: Vec<Point> = pts.iter().map(|p| Point::new(p.x, p.y)).collect();
                    overlay::paint_ants(painter, &[pts], &|p| pos2(p.x, p.y), time, c.marquee_a, c.marquee_b, area);
                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(120));
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
                    let v: Vec<Pos2> = fp_engine::geometry::flatten(ct).iter().map(|p| spp(*p)).collect();
                    if ct.closed {
                        painter.add(egui::Shape::closed_line(v, stroke));
                    } else {
                        painter.add(egui::Shape::line(v, stroke));
                    }
                }
            }
            Overlay::Handles { points } => {
                for p in points {
                    let h = Rect::from_center_size(spp(p), vec2(5.0, 5.0));
                    painter.rect_filled(h, 0.0, c.handle_fill);
                    painter.rect_stroke(h, 0.0, Stroke::new(1.0, c.handle_border), StrokeKind::Outside);
                }
            }
            Overlay::TransformBox { quad, center } => {
                let v: Vec<Pos2> = quad.iter().map(|p| spp(*p)).collect();
                painter.add(egui::Shape::closed_line(v.clone(), Stroke::new(1.0, c.selection)));
                for p in &v {
                    let h = Rect::from_center_size(*p, vec2(5.0, 5.0));
                    painter.rect_filled(h, 0.0, c.handle_fill);
                    painter.rect_stroke(h, 0.0, Stroke::new(1.0, c.handle_border), StrokeKind::Outside);
                }
                let _ = center;
            }
            Overlay::Text { anchor, width, text } => {
                let a = spp(anchor);
                let r = painter.text(a, Align2::LEFT_TOP, text, theme.font(12.0 * view.zoom as f32), c.text);
                let w = width.map(|w| w * view.zoom as f32).unwrap_or(r.width().max(4.0));
                painter.rect_stroke(Rect::from_min_size(a, vec2(w, r.height().max(14.0))), 0.0, stroke, StrokeKind::Outside);
            }
            Overlay::Stroke { points, diameter } => {
                for p in points {
                    painter.circle_stroke(spp(p), (diameter as f64 * view.zoom / 2.0).max(1.0) as f32, Stroke::new(1.0, with_alpha(c.tool_preview, 160)));
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
                for ct in &contours {
                    let v: Vec<Pos2> = fp_engine::geometry::flatten(ct).iter().map(|p| spp(*p)).collect();
                    if ct.closed {
                        painter.add(egui::Shape::closed_line(v, hs));
                    } else {
                        painter.add(egui::Shape::line(v, hs));
                    }
                }
                if let Some(r) = rect {
                    painter.rect_stroke(Rect::from_min_max(sp(r.left as f64, r.top as f64), sp(r.right as f64, r.bottom as f64)), 0.0, hs, StrokeKind::Middle);
                }
            }
            Overlay::PathPoints { contours, selected, handles } => {
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
                            painter.rect_stroke(h, 0.0, Stroke::new(1.0, c.selection), StrokeKind::Inside);
                        }
                    }
                }
            }
            Overlay::Ghost { ids, rect, quad, copy } => {
                had_ghost = true;
                if preview_drag && let Some(s) = cc.ghosts.drag_sprite(&ctx, &st.renderer, &d.doc, generation, page_i, frame_i, &ids, view.zoom) {
                    ghost::paint_mapped(&page_painter, s, rect, &quad, Color32::WHITE, &spp);
                }
                let v: Vec<Pos2> = quad.iter().map(|p| spp(*p)).collect();
                painter.add(egui::Shape::closed_line(v, Stroke::new(1.0, c.selection)));
                cc.ghosts.linger = Some(ghost::Linger { generation, rect, quad, ids: if copy { Vec::new() } else { ids } });
            }
            Overlay::Shade { rect } => {
                let r = Rect::from_min_max(sp(rect.left as f64, rect.top as f64), sp(rect.right as f64, rect.bottom as f64));
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
                let (a, b) = (spp(start), spp(end));
                painter.line_segment([a, b], Stroke::new(1.0, c.handle_border));
                painter.circle_filled(a, 4.0, c.handle_fill);
                painter.rect_filled(Rect::from_center_size(b, vec2(7.0, 7.0)), 0.0, c.handle_fill);
            }
            Overlay::TextEdit(v) => {
                had_text = true;
                if let Some(lc) = textedit::ensure_layout(&mut cc.text, &st.renderer, &d.doc, &v, generation) {
                    // Vorschau mit fp-render (gleicher Satz wie das fertige Textobjekt)
                    if let Some(e) = v.preview_element(&d.doc) {
                        let key = textedit::view_key(&v, generation);
                        if let Some(s) = cc.ghosts.element_sprite(&ctx, &st.renderer, &d.doc, frame_i, &e, key, view.zoom) {
                            ghost::paint_at(&page_painter, s, &spp);
                        }
                    }
                    textedit::paint(painter, lc, &v, time, c, &spp);
                    let cursor_rect = textedit::caret_rect(lc, &v, &spp);
                    ctx.output_mut(|out| {
                        out.ime = Some(egui::output::IMEOutput { purpose: egui::IMEPurpose::Normal, rect: area, cursor_rect, should_interrupt_composition: false });
                    });
                    ctx.request_repaint_after(std::time::Duration::from_millis(250));
                }
            }
            Overlay::BrushTip { at, diameter, round } => {
                let p = spp(at);
                let rr = (diameter as f64 * view.zoom / 2.0).max(1.0) as f32;
                if round {
                    painter.circle_stroke(p, rr, Stroke::new(1.0, c.tool_preview));
                } else {
                    painter.rect_stroke(Rect::from_center_size(p, vec2(rr * 2.0, rr * 2.0)), 0.0, Stroke::new(1.0, c.tool_preview), StrokeKind::Middle);
                }
            }
            Overlay::ControlPoint { at, tooltip, hot } => {
                // gelbe Raute mit dunklem Rand (Steuerpunkt einer automatischen Form)
                let p = spp(at);
                let r = if hot { 5.5 } else { 4.5 };
                let pts = vec![p + vec2(0.0, -r), p + vec2(r, 0.0), p + vec2(0.0, r), p + vec2(-r, 0.0)];
                painter.add(egui::Shape::convex_polygon(pts, Color32::from_rgb(255, 214, 0), Stroke::new(1.0, Color32::from_rgb(90, 70, 0))));
                if hot && let Some(t) = tooltip {
                    label_at(painter, &theme, p + vec2(10.0, 10.0), &t);
                }
            }
            Overlay::Marker { at } => {
                let p = spp(at);
                painter.line_segment([p - vec2(5.0, 0.0), p + vec2(5.0, 0.0)], stroke);
                painter.line_segment([p - vec2(0.0, 5.0), p + vec2(0.0, 5.0)], stroke);
            }
        }
    }
    if had_text {
        cc.text_linger = Some(time);
    } else {
        // Nach dem Ende der Bearbeitung (Esc, Klick daneben): die Textvorschau stehen lassen, bis
        // das Hintergrundbild mit dem übernommenen Text da ist – sonst war der Text bis zum
        // nächsten Neuzeichnen unsichtbar (PM-1, N13). Höchstens 2 s.
        let waiting = cc.text_linger.is_some_and(|t0| time - t0 < 2.0) && cc.shown.is_none_or(|k| k.generation != generation || k.variant != 0);
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
    // Nach dem Loslassen: Vorschau stehen lassen, bis das Hintergrundbild ohne Ausblendung da ist
    if !had_ghost && let Some(l) = &cc.ghosts.linger {
        let waiting = cc.shown.is_some_and(|k| k.variant != 0) && !l.ids.is_empty();
        if waiting && preview_drag {
            if let Some(s) = cc.ghosts.drag.as_ref() {
                ghost::paint_mapped(&page_painter, s, l.rect, &l.quad, Color32::WHITE, &spp);
            }
        } else {
            cc.ghosts.linger = None;
            cc.ghosts.drag = None;
        }
    }
}

/// Beschriftung mit Hintergrund (Größe, Winkel, Abstand, Position).
fn label_at(painter: &egui::Painter, theme: &Theme, at: Pos2, text: &str) {
    let c = &theme.colors;
    let galley = painter.layout_no_wrap(text.to_string(), theme.small_font(), c.label_text);
    let r = Rect::from_min_size(at, galley.size()).expand2(vec2(3.0, 1.0));
    painter.rect_filled(r, 2.0, c.label_bg);
    painter.galley(at, galley, c.label_text);
}

fn argb(a: fp_color::Argb) -> Color32 {
    Color32::from_rgb((a.0 >> 16 & 0xff) as u8, (a.0 >> 8 & 0xff) as u8, (a.0 & 0xff) as u8)
}

/// „Laufende Ameisen“: gestrichelter Rahmen, dessen Muster mit der Zeit wandert.
fn dashed_rect(painter: &egui::Painter, r: Rect, a: Color32, b: Color32, time: f64) {
    painter.rect_stroke(r, 0.0, Stroke::new(1.0, b), StrokeKind::Middle);
    let phase = ((time * 8.0) as i64 % 8) as f32;
    let corners = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
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
    let top = Rect::from_min_max(pos2(area.left(), rect.top()), pos2(rect.right(), area.top()));
    let left = Rect::from_min_max(pos2(rect.left(), area.top()), pos2(area.left(), rect.bottom()));
    let corner = Rect::from_min_size(rect.min, vec2(rs, rs));
    let p = ui.painter();
    p.rect_filled(top, 0.0, c.ruler_bg);
    p.rect_filled(left, 0.0, c.ruler_bg);
    p.rect_filled(corner, 0.0, c.ruler_bg);
    p.hline(top.x_range(), top.bottom() - 0.5, Stroke::new(1.0, c.ruler_tick));
    p.vline(left.right() - 0.5, left.y_range(), Stroke::new(1.0, c.ruler_tick));
    let z = view.zoom;
    let step = [1.0, 2.0, 5.0, 10.0, 20.0, 25.0, 50.0, 100.0, 200.0, 250.0, 500.0, 1000.0, 2000.0, 5000.0].into_iter().find(|s| s * z >= 50.0).unwrap_or(10000.0);
    let minor = step / 5.0;
    let font = theme.font(9.0);
    let vis = view.visible_rect();
    let mut x = (vis.left as f64 / minor).floor() * minor;
    while x <= vis.right as f64 {
        let sx = area.left() + ((x - view.origin.0) * z) as f32;
        let major = (x / step).round() * step == x || ((x / step).fract()).abs() < 1e-9;
        let h = if major { rs } else { rs * 0.3 };
        p.vline(sx.round() + 0.5, (top.bottom() - h)..=top.bottom(), Stroke::new(1.0, c.ruler_tick));
        if major {
            p.text(pos2(sx + 2.0, top.top() + 1.0), Align2::LEFT_TOP, format!("{x:.0}"), font.clone(), c.ruler_text);
        }
        x += minor;
    }
    let mut y = (vis.top as f64 / minor).floor() * minor;
    while y <= vis.bottom as f64 {
        let sy = area.top() + ((y - view.origin.1) * z) as f32;
        let major = ((y / step).fract()).abs() < 1e-9;
        let w = if major { rs } else { rs * 0.3 };
        p.hline((left.right() - w)..=left.right(), sy.round() + 0.5, Stroke::new(1.0, c.ruler_tick));
        if major {
            p.text(pos2(left.left() + 1.0, sy + 2.0), Align2::LEFT_TOP, format!("{y:.0}"), font.clone(), c.ruler_text);
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
    let Some(id) = st.session.active_id() else { return };
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
                ui.painter().hline(area.x_range(), pp.y, Stroke::new(1.0, c.guide_default));
            } else {
                ui.painter().vline(pp.x, area.y_range(), Stroke::new(1.0, c.guide_default));
            }
            if area.contains(pp) {
                let v = if horizontal { dpt.y.round() } else { dpt.x.round() };
                label_at(ui.painter(), &theme, pp + vec2(10.0, 10.0), &format!("{v:.0}"));
            }
        }
        if ui.input(|i| i.pointer.any_released()) {
            cc.guide_drag = None;
            if let Some(pp) = ui.ctx().pointer_interact_pos()
                && area.contains(pp)
            {
                let dpt = view.screen_to_doc((pp.x - area.left()) as f64, (pp.y - area.top()) as f64);
                let (o, pos) = if horizontal { ("horizontal", dpt.y.round()) } else { ("vertical", dpt.x.round()) };
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
    let t = 8.0;
    if content_w > vw + 1.0 {
        let track = Rect::from_min_max(pos2(area.left(), area.bottom() - t), pos2(area.right() - t, area.bottom()));
        let frac = (vw / content_w).clamp(0.05, 1.0) as f32;
        let pos = (((view.origin.0 - pr.left as f64) * z) / (content_w - vw).max(1.0)).clamp(0.0, 1.0) as f32;
        let tw = track.width() * frac;
        let thumb = Rect::from_min_size(pos2(track.left() + (track.width() - tw) * pos, track.top() + 1.0), vec2(tw, t - 2.0));
        ui.painter().rect_filled(track, 0.0, with_alpha(c.scrollbar_bg, 200));
        ui.painter().rect_filled(thumb, 3.0, c.scrollbar_thumb);
        let resp = ui.interact(track, Id::new("hscroll"), Sense::drag());
        if resp.dragged()
            && let Some(d) = st.session.active_mut()
        {
            let dx = resp.drag_delta().x as f64 / (track.width() - tw).max(1.0) as f64 * (content_w - vw);
            d.view.pan(-dx, 0.0);
        }
    }
    if content_h > vh + 1.0 {
        let track = Rect::from_min_max(pos2(area.right() - t, area.top()), pos2(area.right(), area.bottom() - t));
        let frac = (vh / content_h).clamp(0.05, 1.0) as f32;
        let pos = (((view.origin.1 - pr.top as f64) * z) / (content_h - vh).max(1.0)).clamp(0.0, 1.0) as f32;
        let th = track.height() * frac;
        let thumb = Rect::from_min_size(pos2(track.left() + 1.0, track.top() + (track.height() - th) * pos), vec2(t - 2.0, th));
        ui.painter().rect_filled(track, 0.0, with_alpha(c.scrollbar_bg, 200));
        ui.painter().rect_filled(thumb, 3.0, c.scrollbar_thumb);
        let resp = ui.interact(track, Id::new("vscroll"), Sense::drag());
        if resp.dragged()
            && let Some(d) = st.session.active_mut()
        {
            let dy = resp.drag_delta().y as f64 / (track.height() - th).max(1.0) as f64 * (content_h - vh);
            d.view.pan(0.0, -dy);
        }
    }
}

// ---------------------------------------------------------------------- Fußleiste

/// Auswahlinfo links in der Fußleiste (Typtexte 10922–10929 bzw. Exportformat + „(Dokument)“).
fn selection_info(st: &AppState) -> String {
    let Some(d) = st.session.active() else { return String::new() };
    if d.bitmap_mode {
        return fp_i18n::t("ui-sel-bitmap-mode");
    }
    let n = d.selection.elements.len();
    if n == 0 {
        // wie im Original: Exportformat der Optimieren-Einstellung der Seite („GIF (Dokument)“)
        let s = fp_export::doc_export::page_settings(&d.doc, d.page);
        return fp_i18n::tr!("ui-sel-document", format = s.format.display_name().to_string());
    }
    if n > 1 {
        return fp_i18n::tr!("ui-sel-objects", n = n);
    }
    let Some(e) = d.selection.elements.first().and_then(|id| fp_engine::docops::element_on_page(&d.doc, d.page, *id)) else { return fp_i18n::t("ui-sel-object") };
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

fn footer_ui(ui: &mut Ui, st: &mut AppState, rect: Rect) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    ui.painter().rect_filled(rect, 0.0, c.frame_bg);
    let now = ui.input(|i| i.time);
    let info = st.session.active_id().and_then(|id| st.canvases.get(&id)).and_then(|c| c.info.clone());
    let status = match (&st.status, info) {
        (Some((s, t)), _) if now - t < 6.0 => s.clone(),
        (_, Some(i)) => i,
        _ => selection_info(st),
    };
    let Some(d) = st.session.active() else { return };
    let (frames, frame) = (d.frame_count(), d.frame);
    let (w, h) = d.doc.pages.get(d.page).map(|p| (p.canvas.width, p.canvas.height)).unwrap_or((0.0, 0.0));
    let zoom = d.view.zoom;
    let bitmap_mode = d.bitmap_mode;
    // rechts: Zoom, Größe, Status-Steuerung
    let zr = Rect::from_min_max(pos2(rect.right() - 58.0, rect.top() + 1.0), pos2(rect.right() - 2.0, rect.bottom() - 1.0));
    let sr = Rect::from_min_max(pos2(zr.left() - 75.0, zr.top()), pos2(zr.left() - 2.0, zr.bottom()));
    let p = ui.painter();
    for r in [zr, sr] {
        p.rect_filled(r, 0.0, c.frame_bg);
        p.rect_stroke(r, 0.0, Stroke::new(1.0, c.line_light), StrokeKind::Inside);
    }
    p.text(sr.center(), Align2::CENTER_CENTER, format!("{w:.0} x {h:.0}"), theme.small_font(), c.text);
    // Anzeige wie im Original mit `%4.f%%` (rechtsbündig vierstellig, z. B. „ 100%“)
    p.text(pos2(zr.left() + 6.0, zr.center().y), Align2::LEFT_CENTER, format!("{:4.0}%", zoom * 100.0), theme.small_font(), c.text);
    widgets::drop_arrow(p, Rect::from_center_size(pos2(zr.right() - 9.0, zr.center().y + 2.0), vec2(8.0, 8.0)), c.text);
    let zresp = ui.interact(zr, Id::new("footer-zoom"), Sense::click()).on_hover_text(fp_i18n::t("ui-zoom"));
    let nodes: Vec<crate::menu::Node> = ZOOM_STEPS
        .iter()
        .filter(|z| ![0.16, 0.33, 6.0].contains(*z))
        .map(|z| crate::menu::Node::Item { label: format!("{:.0}%", if *z == 0.0625 { 6.0 } else if *z == 0.125 { 12.0 } else { z * 100.0 }), shortcut: None, action: crate::menu::ItemAction::Ui(Action::SetZoom(*z)), enabled: true, checked: Some(zoom == *z) })
        .chain([
            crate::menu::Node::Separator,
            crate::menu::Node::Item { label: fp_i18n::t("ui-zoom-fit-selection"), shortcut: None, action: crate::menu::ItemAction::Command("view.fitSelection".into()), enabled: st.session.is_available("view.fitSelection"), checked: None },
            crate::menu::Node::Item { label: fp_i18n::t("ui-zoom-fit-all"), shortcut: None, action: crate::menu::ItemAction::Command("view.fitAll".into()), enabled: true, checked: None },
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
    let items: [(Icon, String, bool, Option<Action>); 3] = [
        (Icon::FirstFrame, fp_i18n::t("ui-state-first"), multi && frame > 0, Some(Action::SetFrame(0))),
        (Icon::Play, fp_i18n::t("ui-state-play"), multi, None),
        (Icon::LastFrame, fp_i18n::t("ui-state-last"), multi && frame + 1 < frames, Some(Action::SetFrame(frames.saturating_sub(1)))),
    ];
    for (icon, tip, en, act) in items {
        let r = Rect::from_min_size(pos2(x, y), bsize);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r));
        if widgets::icon_button(&mut child, icon, bsize, 11.0, ButtonLook { selected: false, enabled: en, group: Some(ToolGroup::View) }, &tip).clicked()
            && let Some(a) = act
        {
            st.push(a);
        }
        x += 22.0;
    }
    ui.painter().text(pos2(x + 14.0, rect.center().y), Align2::CENTER_CENTER, format!("{}", frame + 1), theme.small_font(), c.text);
    x += 32.0;
    let items2: [(Icon, String, bool, Option<Action>); 3] = [
        (Icon::PrevFrame, fp_i18n::t("ui-state-prev"), multi && frame > 0, Some(Action::SetFrame(frame.saturating_sub(1)))),
        (Icon::NextFrame, fp_i18n::t("ui-state-next"), multi && frame + 1 < frames, Some(Action::SetFrame(frame + 1))),
        (Icon::ExitBitmapMode, fp_i18n::t("ui-exit-bitmap-mode"), bitmap_mode, Some(Action::Run("modify.exitBitmapMode".into(), json!({})))),
    ];
    for (icon, tip, en, act) in items2 {
        let r = Rect::from_min_size(pos2(x, y), bsize);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r));
        if widgets::icon_button(&mut child, icon, bsize, 11.0, ButtonLook { selected: false, enabled: en, group: Some(ToolGroup::View) }, &tip).clicked()
            && let Some(a) = act
        {
            st.push(a);
        }
        x += 22.0;
    }
    // links: Auswahlinfo bzw. Meldung
    let left_w = (rect.width() - (rect.right() - (sr.left() - 6.0 - 7.0 * 22.0 - 16.0)) - 12.0).max(20.0);
    widgets::text_clipped(ui.painter(), pos2(rect.left() + 6.0, rect.center().y), Align2::LEFT_CENTER, &status, theme.small_font(), c.status_text, left_w);
}

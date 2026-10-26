//! FB-97 / TW-272: split document view. Window ▸ Duplicate Window shows the active document in two
//! panes side by side with a draggable splitter; each pane has its own page, state, zoom and
//! scroll (`fp_engine::view_pane`) and a header with its page combo. The pane last clicked has the
//! focus: tools, panels, rulers, the status bar and keyboard shortcuts address it. The other pane
//! is drawn from its parked state (swapped in for the drawing only) and takes no input; a press or
//! a wheel turn over it gives it the focus first, so the tool acts in the pane under the pointer.

use crate::app_state::{Action, AppState};
use crate::theme::Theme;
use crate::tips::HoverTip;
use egui::{CursorIcon, Id, Rect, Sense, Stroke, Ui, pos2, vec2};
use serde_json::json;

/// Height of a pane's header (page combo, close button).
pub const HEADER_H: f32 = 22.0;
/// Width of the splitter between the panes.
pub const SPLITTER_W: f32 = 5.0;
/// Smallest and largest share of the left pane.
pub const MIN_RATIO: f32 = 0.1;
pub const MAX_RATIO: f32 = 0.9;

/// Left pane, splitter and right pane of `rect` for the share `ratio` of the left pane.
pub fn pane_rects(rect: Rect, ratio: f32) -> (Rect, Rect, Rect) {
    let lw = ((rect.width() - SPLITTER_W).max(0.0) * ratio.clamp(MIN_RATIO, MAX_RATIO)).round();
    let left = Rect::from_min_size(rect.min, vec2(lw, rect.height()));
    let splitter = Rect::from_min_size(
        pos2(left.right(), rect.top()),
        vec2(SPLITTER_W, rect.height()),
    );
    let right = Rect::from_min_max(pos2(splitter.right(), rect.top()), rect.max);
    (left, splitter, right)
}

/// Body of a pane (below its header).
pub fn pane_body(pane: Rect) -> Rect {
    Rect::from_min_max(pos2(pane.left(), pane.top() + HEADER_H), pane.max)
}

/// Display names of the pages of a document ("Page n" for unnamed pages).
pub fn page_names(d: &fp_engine::DocumentState) -> Vec<String> {
    d.doc
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
        .collect()
}

/// What a pane header asks for (applied after both panes are drawn).
enum Request {
    Page(usize, usize),
    Close(usize),
}

/// Swaps the live state with the parked pane (engine state and canvas cache).
fn swap(st: &mut AppState) {
    let Some(d) = st.session.active_mut() else {
        return;
    };
    d.swap_pane();
    let (id, pane) = (d.id, d.live_pane());
    st.use_pane_cache(id, pane);
}

/// Draws the split document area into `rect`; `body` draws one pane's canvas (or preview).
pub fn show(
    ui: &mut Ui,
    st: &mut AppState,
    rect: Rect,
    body: &mut dyn FnMut(&mut Ui, &mut AppState, Rect),
) {
    let Some(d) = st.session.active() else { return };
    let id = d.id;
    let theme = Theme::current(ui.ctx());
    let c = theme.colors.clone();
    let (left, splitter, right) = pane_rects(rect, st.split_ratio);
    let panes = [left, right];
    // splitter
    let sresp = ui.interact(splitter, Id::new("fp-split-handle"), Sense::drag());
    if sresp.dragged()
        && let Some(p) = sresp.interact_pointer_pos()
        && rect.width() > SPLITTER_W
    {
        st.split_ratio = ((p.x - rect.left() - SPLITTER_W / 2.0) / (rect.width() - SPLITTER_W))
            .clamp(MIN_RATIO, MAX_RATIO);
    }
    if sresp.hovered() || sresp.dragged() {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
    }
    ui.painter().rect_filled(splitter, 0.0, c.frame_bg);
    ui.painter().vline(
        splitter.center().x,
        splitter.y_range(),
        Stroke::new(1.0, c.line_mid),
    );
    // a press or a wheel turn over the pane without the focus gives it the focus first
    let live = d.live_pane();
    let other = 1 - live;
    let hit = ui.ctx().input(|i| {
        let press = if i.pointer.any_pressed() {
            i.pointer.press_origin()
        } else {
            None
        };
        let turned = i
            .events
            .iter()
            .any(|e| matches!(e, egui::Event::MouseWheel { .. } | egui::Event::Zoom(_)));
        let wheel = if turned { i.pointer.hover_pos() } else { None };
        press.or(wheel)
    });
    if let Some(p) = hit
        && panes[other].contains(p)
        && ui.ctx().layer_id_at(p) == Some(ui.layer_id())
        && !st.dialogs.any_open()
    {
        let _ = st.session.focus_pane(other);
    }
    let live = st.session.active().map_or(0, |d| d.live_pane());
    st.use_pane_cache(id, live);
    let mut request = None;
    for (p, &r) in panes.iter().enumerate() {
        let passive = p != live;
        if passive {
            swap(st);
        }
        st.pane_passive = passive;
        header_ui(ui, st, r, p, !passive, &mut request);
        let b = pane_body(r);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(b));
        child.set_clip_rect(b.intersect(ui.clip_rect()));
        body(&mut child, st, b);
        st.pane_passive = false;
        if passive {
            swap(st);
        }
    }
    match request {
        Some(Request::Page(p, i)) => {
            let _ = st.session.focus_pane(p);
            st.push(Action::SetPage(i));
        }
        Some(Request::Close(p)) => {
            // the other pane gets the focus and stays; this one closes
            let _ = st.session.focus_pane(1 - p);
            st.push(Action::Run("window.closeDuplicate".into(), json!({})));
        }
        None => {}
    }
}

/// Header of pane `p`: focus mark, page combo and close button.
fn header_ui(
    ui: &mut Ui,
    st: &mut AppState,
    pane: Rect,
    p: usize,
    focused: bool,
    request: &mut Option<Request>,
) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let header = Rect::from_min_size(pane.min, vec2(pane.width(), HEADER_H));
    ui.painter().rect_filled(header, 0.0, c.frame_bg);
    if focused {
        ui.painter().rect_filled(
            Rect::from_min_size(header.min, vec2(header.width(), 2.0)),
            0.0,
            c.focus,
        );
    }
    ui.painter().hline(
        header.x_range(),
        header.bottom() - 0.5,
        Stroke::new(1.0, c.line_mid),
    );
    let Some(d) = st.session.active() else { return };
    let names = page_names(d);
    let page = d.page;
    // close button at the right
    let xr = Rect::from_center_size(
        pos2(header.right() - 11.0, header.center().y + 1.0),
        vec2(16.0, 16.0),
    );
    let xresp = ui
        .interact(xr, Id::new(("fp-split-close", p)), Sense::click())
        .hover_tip(fp_i18n::t("ui-split-close-pane"));
    let xcol = if xresp.hovered() { c.text } else { c.text_weak };
    let k = 3.5;
    for (a, b) in [
        (xr.center() + vec2(-k, -k), xr.center() + vec2(k, k)),
        (xr.center() + vec2(-k, k), xr.center() + vec2(k, -k)),
    ] {
        ui.painter().line_segment([a, b], Stroke::new(1.2, xcol));
    }
    if xresp.clicked() {
        *request = Some(Request::Close(p));
    }
    let combo = Rect::from_min_max(
        pos2(header.left() + 6.0, header.top() + 3.0),
        pos2(
            (xr.left() - 6.0).min(header.left() + 240.0),
            header.bottom() - 2.0,
        ),
    );
    if combo.width() < 40.0 {
        return;
    }
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(combo));
    child.set_clip_rect(combo.intersect(ui.clip_rect()));
    let mut sel = page;
    let text = names.get(page).cloned().unwrap_or_default();
    let text = if focused {
        egui::RichText::new(text)
    } else {
        egui::RichText::new(text).color(c.text_weak)
    };
    egui::ComboBox::from_id_salt(("fp-split-page", p))
        .width(combo.width() - 4.0)
        .truncate()
        .selected_text(text)
        .show_ui(&mut child, |ui| {
            for (i, n) in names.iter().enumerate() {
                ui.selectable_value(&mut sel, i, n.as_str());
            }
        })
        .response
        .hover_tip(fp_i18n::t("ui-split-pane-page"));
    if sel != page {
        *request = Some(Request::Page(p, sel));
    }
}

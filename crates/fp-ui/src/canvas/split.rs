//! FB-97 / TW-272: split document view. Window ▸ Duplicate Window shows the active document in two
//! panes with a draggable splitter, side by side by default or one above the other (F132,
//! `window.splitOrientation`); each pane has its own page, state, zoom and scroll
//! (`fp_engine::view_pane`) and a header with its page combo. F137 / TW-275: Duplicate Window on a
//! split view splits the focused pane again, up to four panes in one row or column, each splitter
//! draggable; Close Duplicate Window or a pane's × closes that pane.
//!
//! The pane last clicked has the focus: tools, panels, rulers and keyboard shortcuts address it
//! (Shift+F6 or Window ▸ Next Pane moves the focus through all panes). The other panes are drawn
//! from their parked state (swapped in for the drawing only) and take no input, but show the hover
//! feedback of the active tool (cursor, hover highlight, brush tip, coordinate tag) while the
//! pointer is over them (F132); a press or a wheel turn over one gives it the focus first, so the
//! tool acts in the pane under the pointer. The status bar and the Info panel follow the pane
//! under the pointer, else the focused pane (F137, [`AppState::hover_pane`]). Orientation and
//! splitter positions are remembered per document for the session
//! (`DocumentState::split_layout`).

use crate::app_state::{Action, AppState};
use crate::theme::Theme;
use crate::tips::HoverTip;
use egui::{CursorIcon, Id, Pos2, Rect, Sense, Stroke, Ui, pos2, vec2};
use serde_json::json;

/// Height of a pane's header (page combo, close button).
pub const HEADER_H: f32 = 22.0;
/// Width of the splitter between the panes.
pub const SPLITTER_W: f32 = 5.0;
/// Smallest and largest share of the first (left or top) pane of two.
pub const MIN_RATIO: f32 = fp_engine::view_pane::MIN_SHARE;
pub const MAX_RATIO: f32 = 1.0 - fp_engine::view_pane::MIN_SHARE;

/// Left pane, splitter and right pane of `rect` for the share `ratio` of the left pane.
pub fn pane_rects(rect: Rect, ratio: f32) -> (Rect, Rect, Rect) {
    pane_rects_in(rect, ratio, false)
}

/// First pane, splitter and second pane of `rect` for the share `ratio` of the first pane: side
/// by side, or one above the other with `horizontal` (F132).
pub fn pane_rects_in(rect: Rect, ratio: f32, horizontal: bool) -> (Rect, Rect, Rect) {
    let r = ratio.clamp(MIN_RATIO, MAX_RATIO);
    let (p, s) = layout_rects(rect, &[r], horizontal);
    (p[0], s[0], p[1])
}

/// F137: the panes and splitters of `rect` for the splitter positions `bounds` (shares of the
/// area, increasing; `bounds.len() + 1` panes) in a row, or a column with `horizontal`. The
/// splitters take [`SPLITTER_W`] each; pane `i` ends at `round(available × bounds[i])`.
pub fn layout_rects(rect: Rect, bounds: &[f32], horizontal: bool) -> (Vec<Rect>, Vec<Rect>) {
    let n = bounds.len() + 1;
    let (start, len) = if horizontal {
        (rect.top(), rect.height())
    } else {
        (rect.left(), rect.width())
    };
    let avail = (len - SPLITTER_W * (n - 1) as f32).max(0.0);
    let span = |a: f32, b: f32| {
        if horizontal {
            Rect::from_min_max(pos2(rect.left(), a), pos2(rect.right(), b))
        } else {
            Rect::from_min_max(pos2(a, rect.top()), pos2(b, rect.bottom()))
        }
    };
    let (mut panes, mut splitters) = (Vec::with_capacity(n), Vec::with_capacity(n - 1));
    let mut a = start;
    for (i, &b) in bounds.iter().enumerate() {
        let e = start + i as f32 * SPLITTER_W + (avail * b.clamp(0.0, 1.0)).round();
        let e = e.max(a);
        panes.push(span(a, e));
        splitters.push(span(e, e + SPLITTER_W));
        a = e + SPLITTER_W;
    }
    panes.push(span(a.min(start + len), start + len));
    (panes, splitters)
}

/// Share of the first pane for the splitter dragged to `p` (F132: along the orientation's axis).
pub fn ratio_at(rect: Rect, p: Pos2, horizontal: bool) -> Option<f32> {
    bound_at(rect, p, horizontal, 0, 2).map(|v| v.clamp(MIN_RATIO, MAX_RATIO))
}

/// F137: position (share of the area) of splitter `i` of `n` panes dragged to `p`, the inverse of
/// [`layout_rects`] (the engine keeps the panes apart, `SplitLayout::set_bound`).
pub fn bound_at(rect: Rect, p: Pos2, horizontal: bool, i: usize, n: usize) -> Option<f32> {
    let (len, at) = if horizontal {
        (rect.height(), p.y - rect.top())
    } else {
        (rect.width(), p.x - rect.left())
    };
    let avail = len - SPLITTER_W * n.saturating_sub(1) as f32;
    (avail > 0.0).then(|| (at - SPLITTER_W / 2.0 - i as f32 * SPLITTER_W) / avail)
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

/// What a pane header asks for (applied after all panes are drawn).
enum Request {
    Page(usize, usize),
    Close(usize),
}

/// Makes pane `p` live for the drawing (engine state and canvas cache).
fn swap_to(st: &mut AppState, p: usize) {
    let Some(d) = st.session.active_mut() else {
        return;
    };
    d.swap_with(p);
    let (id, key) = (d.id, d.pane_key(d.live_pane()));
    st.use_pane_cache(id, key);
}

/// F137: the pane of `panes` under the screen point `p`.
pub fn pane_at(panes: &[Rect], p: Pos2) -> Option<usize> {
    panes.iter().position(|r| r.contains(p))
}

/// Draws the split document area into `rect`; `body` draws one pane's canvas (or preview).
pub fn show(
    ui: &mut Ui,
    st: &mut AppState,
    rect: Rect,
    body: &mut dyn FnMut(&mut Ui, &mut AppState, Rect),
) {
    let Some(d) = st.session.active() else { return };
    let n = d.pane_count();
    let horizontal =
        d.split_layout.orientation == fp_engine::view_pane::SplitOrientation::Horizontal;
    let bounds = d.split_layout.fitted_bounds(n);
    let theme = Theme::current(ui.ctx());
    let c = theme.colors.clone();
    let (panes, splitters) = layout_rects(rect, &bounds, horizontal);
    // splitters (their positions are remembered per document for the session, F132, F137)
    let line = Stroke::new(1.0, c.line_mid);
    for (i, &sr) in splitters.iter().enumerate() {
        let sid = if i == 0 {
            Id::new("fp-split-handle")
        } else {
            Id::new(("fp-split-handle", i))
        };
        let sresp = ui.interact(sr, sid, Sense::drag());
        if sresp.dragged()
            && let Some(p) = sresp.interact_pointer_pos()
            && let Some(v) = bound_at(rect, p, horizontal, i, n)
            && let Some(d) = st.session.active_mut()
        {
            d.split_layout.set_bound(n, i, v);
        }
        if sresp.hovered() || sresp.dragged() {
            ui.ctx().set_cursor_icon(if horizontal {
                CursorIcon::ResizeVertical
            } else {
                CursorIcon::ResizeHorizontal
            });
        }
        ui.painter().rect_filled(sr, 0.0, c.frame_bg);
        if horizontal {
            ui.painter().hline(sr.x_range(), sr.center().y, line);
        } else {
            ui.painter().vline(sr.center().x, sr.y_range(), line);
        }
    }
    let Some(d) = st.session.active() else { return };
    let id = d.id;
    // a press or a wheel turn over a pane without the focus gives it the focus first
    let live = d.live_pane();
    let (hit, hover) = ui.ctx().input(|i| {
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
        (press.or(wheel), i.pointer.hover_pos())
    });
    let ours = |p: Pos2| ui.ctx().layer_id_at(p) == Some(ui.layer_id());
    // (only the pane bodies: the header's × closes its pane without moving the focus, the page
    // combo focuses its pane itself)
    let bodies: Vec<Rect> = panes.iter().map(|&r| pane_body(r)).collect();
    if let Some(p) = hit
        && let Some(target) = pane_at(&bodies, p)
        && target != live
        && ours(p)
        && !st.dialogs.any_open()
    {
        let _ = st.session.focus_pane(target);
    }
    // F137 / TW-275: the pane under the pointer (status bar and Info panel follow it)
    st.hover_pane = hover.filter(|&p| ours(p)).and_then(|p| pane_at(&panes, p));
    let Some(d) = st.session.active() else { return };
    let live = d.live_pane();
    st.pane_focus_key = d.pane_key(live);
    st.use_pane_cache(id, st.pane_focus_key);
    let mut request = None;
    for (p, &r) in panes.iter().enumerate() {
        let passive = p != live;
        if passive {
            swap_to(st, p);
        }
        st.pane_passive = passive;
        header_ui(ui, st, r, p, !passive, &mut request);
        let b = pane_body(r);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(b));
        child.set_clip_rect(b.intersect(ui.clip_rect()));
        body(&mut child, st, b);
        st.pane_passive = false;
        if passive {
            swap_to(st, live);
        }
    }
    match request {
        Some(Request::Page(p, i)) => {
            let _ = st.session.focus_pane(p);
            st.push(Action::SetPage(i));
        }
        Some(Request::Close(p)) => {
            // F137: this pane closes; the focus stays (or goes to the neighbour of a closed
            // focused pane)
            st.push(Action::Run(
                "window.closeDuplicate".into(),
                json!({ "pane": p }),
            ));
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

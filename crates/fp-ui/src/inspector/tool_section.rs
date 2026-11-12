//! Tool options section of the Properties panel (Firepixel extension FB-149, TW-295): with an
//! object selected and a tool with options active, the panel shows the object properties as
//! before and, below a thin separator line, a second section with the tool icon, "Current tool"
//! and the tool name, then the options of that tool. The swap button at the left end of the
//! separator puts the tool section on top or below (persisted in the workspace,
//! `inspector_tool_section_on_top`). The section lies below the regular two rows, so a panel of
//! the usual height reaches it with its vertical scroll bar; nothing changes without a
//! selection or at half height.
//!
//! - Pointer, Subselection (and the Mask Element Selection that shares their layout) and the
//!   selection tools Marquee, Oval Marquee, Lasso, Polygon Lasso, Magic Wand: the existing
//!   Firepixel preferences of the Edit page as check boxes, bound to the same preference keys
//!   (a change here is a change of the preference, saved the same way); the Pointer group also
//!   shows its existing "Marquee:" popup (Rectangle / Lasso, [`super::layouts::POINTER_Q5`]).
//! - Tools whose own tool row stands after the object rows of the layout table (Pen, Line,
//!   Rectangle, Auto Shape, Polygon, Ellipse, hotspot tools): that same tool row (Q3 to Q6),
//!   drawn by the existing quadrant builders. Tools whose row stands before the object rows
//!   (Brush, Eraser, Text, …) already show their options with a selection.

use super::layouts::{self, PiLayout, Pred};
use super::{InspectorState, texts};
use crate::app_state::AppState;
use crate::icons::{self, IconStyle};
use crate::theme::{Theme, ToolGroup};
use crate::tips::HoverTip;
use crate::widgets;
use egui::{Align2, Id, Rect, Sense, Stroke, Ui, pos2, vec2};
use fp_engine::prefs::PrefValue;
use fp_engine::{Session, ToolId};

/// Template id of the section header (icon box, "Current tool", tool name); Firepixel's own id
/// outside the original's template range.
pub const HEADER_Q1: u16 = 61020;

/// Height of the separator strip with the swap button.
pub const SEPARATOR_H: f32 = 14.0;

/// Pitch of the check box lines.
const LINE_H: f32 = 17.0;

/// Width of the header (like Q1).
const HEADER_W: f32 = super::W1;

/// The preferences shown as check boxes (key, label key of the Preferences dialog), in the order
/// of the Edit page: first column, then second column.
pub const PREF_CHECKS: [(&str, &str); 5] = [
    (
        crate::canvas::tools::ai_object::PREF_KEY,
        "dlg-pref-aiobjecthoverpreview",
    ),
    (
        fp_engine::tools::ALT_MARQUEE_DESELECT_PREF,
        "dlg-pref-altmarqueedeselects",
    ),
    (
        fp_engine::tools::ARROW_KEYS_MOVE_PIXELS_PREF,
        "dlg-pref-arrowkeysmovepixels",
    ),
    (
        fp_engine::tools::DUPLICATE_WITH_CTRL_DRAG_PREF,
        "dlg-pref-duplicatewithctrldrag",
    ),
    (
        fp_engine::tools::SHIFT_MARQUEE_ADDS_ONLY_PREF,
        "dlg-pref-shiftmarqueeaddsonly",
    ),
];

/// Lines of the first column (the rest go into the second).
const COL1: usize = 3;

/// What the section shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Section {
    /// Preference check boxes; `marquee`: with the Pointer's "Marquee:" popup.
    Prefs { marquee: bool },
    /// The tool's own tool row (Q3 to Q6 and the line mask of the layout table).
    ToolRow { quads: [u16; 4], mask: u16 },
}

const POINTER_GROUP: [ToolId; 3] = [
    ToolId::Pointer,
    ToolId::Subselection,
    ToolId::MaskElementSelect,
];
const SELECTION_TOOLS: [ToolId; 5] = [
    ToolId::Marquee,
    ToolId::OvalMarquee,
    ToolId::Lasso,
    ToolId::PolygonLasso,
    ToolId::MagicWand,
];

/// Q5 with the path buttons (Combine, …) of the vector rows.
const PATH_OPS_Q5: u16 = 14132;

/// Is `q` an object header (selection 14101 / multi-selection 14102)?
fn object_header(q: u16) -> bool {
    q == 14101 || q == 14102
}

/// Empty templates and the document quads: no tool options.
fn is_option_quad(q: u16) -> bool {
    !matches!(q, 0 | 14079..=14084 | 14031 | 6564 | 14007 | 14100)
}

/// Tools whose own tool row stands after the object rows of the layout table (so a selection
/// shows the object panel) and has options for new objects. The Text tool is left out: while a
/// text is edited its row shows the attributes of that text, the object panel already does.
const ROW_TOOLS: [ToolId; 9] = [
    ToolId::Pen,
    ToolId::Line,
    ToolId::Rectangle,
    ToolId::AutoShape,
    ToolId::Polygon,
    ToolId::Ellipse,
    ToolId::HotspotRect,
    ToolId::HotspotPolygon,
    ToolId::HotspotCircle,
];

/// The tool's own row of the layout table, if it has options in Q3 to Q6.
fn tool_row(tool: ToolId) -> Option<&'static PiLayout> {
    if !ROW_TOOLS.contains(&tool) {
        return None;
    }
    layouts::LAYOUTS
        .iter()
        .find(|l| l.pred == Pred::Tool(tool))
        .filter(|l| l.quads[2..].iter().any(|q| is_option_quad(*q)))
}

/// The section for the current session (`None` = panel as before).
pub fn section_for(s: &Session) -> Option<Section> {
    let d = s.active()?;
    let tool = s.tools.active;
    let shown = layouts::select_layout(s)?;
    let object = object_header(shown.quads[0]);
    if POINTER_GROUP.contains(&tool) {
        return object.then_some(Section::Prefs { marquee: true });
    }
    if SELECTION_TOOLS.contains(&tool) {
        let any = !d.selection.elements.is_empty() || d.selection.has_pixels();
        return any.then_some(Section::Prefs { marquee: false });
    }
    if !object {
        return None;
    }
    let row = tool_row(tool)?;
    // the path buttons of Q5 (14132, Combine etc.) act on the selected objects and stand in the
    // object panel already (same widgets, same ids): left out of the section
    let q5 = if row.quads[4] == PATH_OPS_Q5 {
        14083
    } else {
        row.quads[4]
    };
    Some(Section::ToolRow {
        quads: [row.quads[2], row.quads[3], q5, row.quads[5]],
        mask: row.mask,
    })
}

/// Column layout of the check boxes: x offsets (relative to the options' left edge) of the two
/// columns, or one column when the second does not fit into `width`.
fn columns(ui: &Ui, theme: &Theme, width: f32) -> (f32, bool) {
    let font = theme.small_font();
    let w = |k: &str| widgets::text_width(ui, &texts::t(k), &font) + 22.0;
    let w1 = PREF_CHECKS[..COL1]
        .iter()
        .map(|(_, k)| w(k))
        .fold(0.0, f32::max);
    let w2 = PREF_CHECKS[COL1..]
        .iter()
        .map(|(_, k)| w(k))
        .fold(0.0, f32::max);
    let x2 = w1 + 16.0;
    (x2, x2 + w2 <= width)
}

/// x offset of the check boxes behind the header (and the Pointer's popup).
fn checks_left(marquee: bool) -> f32 {
    HEADER_W + if marquee { super::W5 + 8.0 } else { 8.0 }
}

/// Height of the section content (without the separator) for a body of `width` pixels.
pub fn content_height(ui: &Ui, theme: &Theme, sec: &Section, width: f32) -> f32 {
    match sec {
        Section::Prefs { marquee } => {
            let (_, two) = columns(ui, theme, width - checks_left(*marquee) - 4.0);
            let lines = if two { COL1 } else { PREF_CHECKS.len() };
            (lines as f32 * LINE_H + 8.0).max(theme.metrics.inspector_row_height)
        }
        Section::ToolRow { .. } => super::content_height(theme),
    }
}

/// Is the tool section on top (swap button)?
pub fn on_top(st: &AppState) -> bool {
    st.settings.workspace.inspector_tool_section_on_top
}

/// Separator strip: swap button at the left, thin line to the right edge.
pub fn separator(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let cy = r.center().y;
    let br = Rect::from_min_size(pos2(r.left() + 4.0, cy - 7.0), vec2(22.0, 14.0));
    let resp = ui.interact(br, Id::new("pi-tool-section-swap"), Sense::click());
    resp.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            true,
            texts::t("pi-tool-section-swap"),
        )
    });
    let clicked = resp.clicked();
    let resp = resp.hover_tip(texts::t("pi-tip-tool-section-swap"));
    let col = if resp.hovered() { c.focus } else { c.text };
    let p = ui.painter();
    // two stacked boxes in the middle
    let bx = Rect::from_min_size(pos2(br.left() + 7.0, br.top() + 1.0), vec2(8.0, 5.5));
    let by = bx.translate(vec2(0.0, 6.5));
    let box_stroke = Stroke::new(1.0, c.line_mid);
    p.rect_stroke(bx, 0.0, box_stroke, egui::StrokeKind::Inside);
    p.rect_stroke(by, 0.0, box_stroke, egui::StrokeKind::Inside);
    let s = Stroke::new(1.0, col);
    // left bracket with an arrow up, right bracket with an arrow down
    let (l, rt) = (br.left() + 2.5, br.right() - 2.5);
    let (t, b) = (br.top() + 2.0, br.bottom() - 2.0);
    p.line_segment([pos2(l + 2.5, t), pos2(l, t)], s);
    p.line_segment([pos2(l, t), pos2(l, b)], s);
    p.line_segment([pos2(l, b), pos2(l + 2.0, b)], s);
    p.line_segment([pos2(l + 2.5, t), pos2(l + 1.0, t - 1.5)], s);
    p.line_segment([pos2(l + 2.5, t), pos2(l + 1.0, t + 1.5)], s);
    p.line_segment([pos2(rt - 2.0, t), pos2(rt, t)], s);
    p.line_segment([pos2(rt, t), pos2(rt, b)], s);
    p.line_segment([pos2(rt, b), pos2(rt - 2.5, b)], s);
    p.line_segment([pos2(rt - 2.5, b), pos2(rt - 1.0, b - 1.5)], s);
    p.line_segment([pos2(rt - 2.5, b), pos2(rt - 1.0, b + 1.5)], s);
    // the separator line
    let line = Stroke::new(1.0, if theme.glow { c.line_dark } else { c.line_mid });
    p.hline((br.right() + 4.0)..=(r.right() - 4.0), cy - 0.5, line);
    p.hline(
        (br.right() + 4.0)..=(r.right() - 4.0),
        cy + 0.5,
        Stroke::new(1.0, c.line_light),
    );
    if clicked {
        let ws = &mut st.settings.workspace;
        ws.inspector_tool_section_on_top = !ws.inspector_tool_section_on_top;
    }
}

/// Header: icon box, "Current tool" and the tool name (also the Q1 of a tool row section).
pub fn header(ui: &mut Ui, st: &mut AppState, r: Rect) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let id = st.session.tools.active;
    let group = fp_engine::tools::info(id).map(|i| ToolGroup::from_section(i.section));
    let bx = Rect::from_min_size(pos2(r.left() + 8.0, r.top() + 4.0), vec2(26.0, 26.0));
    let p = ui.painter();
    p.rect_filled(bx, 0.0, c.field_bg);
    p.rect_stroke(
        bx,
        0.0,
        Stroke::new(1.0, c.field_border),
        egui::StrokeKind::Inside,
    );
    icons::paint(
        p,
        bx.shrink(5.0),
        icons::tool_icon(id),
        &IconStyle::new(&theme, group, true),
    );
    let tx = bx.right() + 6.0;
    let w = r.right() - 2.0 - tx;
    for (text, y) in [
        (texts::t("pi-current-tool"), 6.0),
        (super::quads::tool_q1_label(st), 20.0),
    ] {
        // a long (translated) text gets a slightly smaller font before it is cut off
        let mut font = theme.small_font();
        while widgets::text_width(ui, &text, &font) > w && font.size > 7.0 {
            font.size -= 0.5;
        }
        widgets::text_clipped(
            ui.painter(),
            pos2(tx, bx.top() + y),
            Align2::LEFT_CENTER,
            &text,
            font,
            c.text,
            w,
        );
    }
}

/// Draws the section content into `r` (below or above the separator).
pub fn content(ui: &mut Ui, st: &mut AppState, r: Rect, sec: &Section, theme: &Theme) {
    match *sec {
        Section::ToolRow { quads, mask } => {
            let state = InspectorState {
                quads: [HEADER_Q1, 14080, quads[0], quads[1], quads[2], quads[3]],
                mask,
                number: 0,
            };
            super::quadrants(ui, st, r, &state, false, theme, "tool-section-quad");
        }
        Section::Prefs { marquee } => {
            let hr = Rect::from_min_size(r.min, vec2(HEADER_W, r.height()));
            header(ui, st, hr);
            if marquee && let Some(f) = super::registry().get(layouts::POINTER_Q5) {
                let qr = Rect::from_min_size(
                    pos2(r.left() + HEADER_W, r.top() + 6.0),
                    vec2(super::W5, theme.metrics.inspector_row_height),
                );
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt("tool-section-marquee")
                        .max_rect(qr)
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                f(&mut child, st, qr);
            }
            let left = r.left() + checks_left(marquee);
            let (x2, two) = columns(ui, theme, r.right() - 4.0 - left);
            for (i, (key, label)) in PREF_CHECKS.iter().enumerate() {
                let (x, line) = if two && i >= COL1 {
                    (left + x2, i - COL1)
                } else {
                    (left, i)
                };
                let text = texts::t(label);
                let w = widgets::text_width(ui, &text, &theme.small_font()) + 24.0;
                let cr = Rect::from_min_size(
                    pos2(x, r.top() + 4.0 + line as f32 * LINE_H),
                    vec2(w, LINE_H - 1.0),
                );
                let on = st.session.prefs.get_bool(key, false);
                if let Some(v) =
                    super::fields::checkbox(ui, ("pi-tool-section-pref", *key), cr, &text, on, true)
                    && v != on
                {
                    set_pref(st, key, v);
                }
            }
        }
    }
}

/// Sets a preference like the Preferences dialog does on OK (stored at once).
pub fn set_pref(st: &mut AppState, key: &str, on: bool) {
    st.session.prefs.set(key, PrefValue::Bool(on));
    st.session.prefs_changed();
}

//! Tool attributes that follow the selection (Q-58, TW-192, RE spec 58 §2).
//!
//! Measured in the reference application: the tool attributes (fill, stroke colour, tip, size and
//! placement; `dom.pathAttributes` in scripts) take on the attributes of the selected object once
//! the user interface is idle, and keep them after deselecting - but only when exactly one path
//! (also a rectangle primitive) or one text is selected (RE spec 59 section 2.4): groups, compound
//! shapes, auto shapes, several selected objects, bitmaps and web objects change nothing. A path
//! with the fill "none" gives the tool its own stored fill colour. Inside one script nothing
//! follows; the next script sees the change.
//!
//! In Firepixel the tool attributes are the colours of the tools panel
//! (`ToolState::stroke_color/fill_color`) plus the inspector's tool defaults
//! (`ToolOptions::new_fill/new_stroke/new_brush`, the parameters of `pi.fill`/`pi.stroke` and the full
//! brush). [`Session::follow_selection`] runs after every top-level command, group, undo and redo
//! and on every frame of the user interface; a running script holds it
//! ([`Session::hold_tool_follow`]).

use crate::commands::panel::{
    BRUSH_PRESETS, DEFAULT_TOOL_PRESET, fill_kind, preset_index_of, shape_id,
};
use crate::session::Session;
use fp_color::Argb;
use fp_model::PathAttrs;
use fp_model::element::{Element, ElementKind};
use serde_json::{Value, json};

/// The tool attributes: colours of the tools panel and the inspector's defaults for new objects.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolStyle {
    pub stroke_color: Argb,
    pub fill_color: Argb,
    /// Parameters of `pi.fill` for new objects (`None` = solid with the fill colour).
    pub new_fill: Option<Value>,
    /// Parameters of `pi.stroke` for new objects (`None` = the factory stroke, 1 px).
    pub new_stroke: Option<Value>,
    /// Full brush of new objects (`fp_model::Brush` as JSON), `None` = from `new_stroke`.
    pub new_brush: Option<Value>,
}

impl ToolStyle {
    /// Is the stroke "none" (transparent stroke colour or `{"none": true}`)?
    pub fn stroke_is_none(&self) -> bool {
        self.stroke_color.a() == 0
            || self
                .new_stroke
                .as_ref()
                .and_then(|v| v.get("none"))
                .and_then(|v| v.as_bool())
                == Some(true)
    }

    /// Is the fill "none" (transparent fill colour or the fill kind `{"category": "none"}`)?
    pub fn fill_is_none(&self) -> bool {
        self.fill_color.a() == 0
            || self
                .new_fill
                .as_ref()
                .and_then(|v| v.get("category"))
                .and_then(|v| v.as_str())
                == Some("none")
    }

    /// Stroke placement of new objects: "inside", "center" or "outside".
    pub fn placement(&self) -> &str {
        self.new_stroke
            .as_ref()
            .and_then(|v| v.get("placement"))
            .and_then(|v| v.as_str())
            .unwrap_or("center")
    }
}

/// Tool attributes taken from an element: a path (also a rectangle primitive) or a text; `None` for
/// groups, compound shapes, auto shapes, bitmaps, symbols, web objects and the like (RE spec 59
/// section 2.4: the tool does not follow them).
pub fn style_of_element(e: &Element) -> Option<ToolStyle> {
    match &e.kind {
        ElementKind::Path(p) => Some(style_from(&p.attrs, p.stroke_placement)),
        ElementKind::Text(t) => Some(style_from(&t.attrs, t.stroke_placement)),
        _ => None,
    }
}

/// Tool attributes from path attributes and a stroke placement (0 inside, 1 centre, 2 outside),
/// in the canonical form the inspector uses (see [`style_of_element`]).
pub fn style_of_attrs(a: &PathAttrs, placement: i32) -> ToolStyle {
    style_from(a, placement)
}

/// Colours an element shows in the toolbox wells (stroke and fill, alpha 0 for "none"): paths and
/// texts their own, an auto shape those of its first path; `None` for anything else.
fn well_colors_of(e: &Element) -> Option<(Argb, Argb)> {
    let of = |a: &PathAttrs| {
        let none = |c: Argb| Argb(c.0 & 0x00ff_ffff);
        (
            if a.brush.is_some() {
                a.stroke_color
            } else {
                none(a.stroke_color)
            },
            if a.fill.is_some() {
                a.fill_color
            } else {
                none(a.fill_color)
            },
        )
    };
    match &e.kind {
        ElementKind::Path(p) => Some(of(&p.attrs)),
        ElementKind::Text(t) => Some(of(&t.attrs)),
        ElementKind::Group(g) if g.smart_shape.is_some() => {
            g.children.iter().find_map(|c| match &c.kind {
                ElementKind::Path(p) => Some(of(&p.attrs)),
                _ => None,
            })
        }
        _ => None,
    }
}

/// Stroke placement id (0 inside, 1 centre, 2 outside) of a placement name.
pub fn placement_id(name: &str) -> i32 {
    match name {
        "inside" => 0,
        "outside" => 2,
        _ => 1,
    }
}

/// Stroke parameters of the tip `bn_1-Pixel Anti-Aliased` (`bc_Pencil`) in the size `width`.
pub fn one_pixel_aa_stroke(width: i32, placement: &str) -> Option<Value> {
    stroke_params(
        preset_index_of("bc_Pencil", "bn_1-Pixel Anti-Aliased"),
        width,
        0,
        placement,
        None,
    )
}

/// Stroke parameters of the kind "none" with a placement (left out when centred).
pub fn none_stroke(placement: &str) -> Value {
    if placement == "center" {
        json!({"none": true})
    } else {
        json!({"none": true, "placement": placement})
    }
}

fn placement_name(p: i32) -> &'static str {
    match p {
        0 => "inside",
        2 => "outside",
        _ => "center",
    }
}

/// `pi.stroke` parameters of a tool stroke in the inspector's canonical form (`attrs_params` in
/// fp-ui): `None` for the factory stroke (default preset, 1 px, hard, centred, no texture). A preset
/// that is not in the list (`None`) is written as the default preset; the full brush then comes
/// from `new_brush`.
pub fn stroke_params(
    preset: Option<usize>,
    width: i32,
    softness: i32,
    placement: &str,
    texture: Option<(String, i32)>,
) -> Option<Value> {
    let default = preset_index_of(DEFAULT_TOOL_PRESET.0, DEFAULT_TOOL_PRESET.1);
    let preset = preset.or(default);
    let softness = softness.clamp(0, 100);
    let width = width.clamp(1, 1000);
    if preset == default
        && width == 1
        && softness == 0
        && placement == "center"
        && texture.is_none()
    {
        return None;
    }
    let p = preset.and_then(|i| BRUSH_PRESETS.get(i))?;
    let mut v = json!({
        "preset": format!("{}/{}", p.category_de, p.name_de),
        "width": width,
        "softness": softness,
        "placement": placement,
    });
    if let Some((t, amount)) = texture {
        v["texture"] = json!(t);
        v["textureAmount"] = json!(amount.clamp(0, 100));
    }
    Some(v)
}

/// Tool attributes from path attributes, written in the canonical form the inspector uses
/// (`attrs_params` in fp-ui: defaults are `None`), so that the user interface takes them over
/// without a difference.
fn style_from(a: &PathAttrs, placement: i32) -> ToolStyle {
    let place = placement_name(placement);
    let (stroke_color, new_stroke, new_brush) = match &a.brush {
        // no stroke: the stroke kind "none", the colour stays (measured: brush null keeps its
        // brushColor, which the 1-pixel line of `addNewLine` takes); the placement stays apart
        // (measured: a text gives "outside", RE spec 59 section 2.2)
        None => (a.stroke_color, Some(none_stroke(place)), None),
        Some(b) => {
            let preset = preset_index_of(&b.category, &b.name);
            let texture = a
                .stroke_texture
                .as_ref()
                .map(|t| t.name.clone())
                .filter(|n| !n.is_empty() && b.texture_blend > 0)
                .map(|n| (n, b.texture_blend / 10));
            let stroke = stroke_params(preset, b.diameter, b.softness / 10, place, texture);
            // the full brush only when the stroke parameters do not rebuild it (a custom tip):
            // otherwise the paint tools keep building theirs from the stroke parameters
            let full = serde_json::to_value(b).ok();
            let rebuilt = serde_json::to_value(crate::commands::panel::tool_brush_from(
                stroke.as_ref(),
                None,
            ))
            .ok();
            let brush = if full == rebuilt { None } else { full };
            (a.stroke_color, stroke, brush)
        }
    };
    let (fill_color, new_fill) = match a.fill.as_ref() {
        // no fill: the tool fill becomes "none" with the object's own stored fill colour (measured,
        // RE spec 59 section 2.2: a path with fill none and fillColor #808080 gives #808080)
        None => (a.fill_color, Some(json!({"category": "none"}))),
        Some(f) => {
            let kind = match fill_kind(Some(f)) {
                4 => 1,
                k => k,
            };
            let texture = a
                .fill_texture
                .as_ref()
                .map(|t| t.name.clone())
                .filter(|n| !n.is_empty() && f.texture_amount > 0);
            let fill = if kind == 1 && f.edge == 1 && f.feather == 0 && texture.is_none() {
                None
            } else {
                let mut v = match kind {
                    2 => json!({"category": "gradient", "shape": shape_id(f)}),
                    3 => match f
                        .pattern
                        .as_ref()
                        .map(|p| p.name.clone())
                        .filter(|n| !n.is_empty())
                    {
                        Some(n) => json!({"category": "pattern", "pattern": n}),
                        None => json!({"category": "pattern"}),
                    },
                    _ => json!({"category": "solid"}),
                };
                v["edge"] = json!(f.edge.clamp(0, 3));
                if fp_model::Fill::edge_uses_amount(f.edge) {
                    v["feather"] = json!(f.feather.clamp(0, 100));
                }
                if let Some(t) = texture {
                    v["texture"] = json!(t);
                    v["textureAmount"] = json!((f.texture_amount / 10).clamp(0, 100));
                    v["textureTransparent"] = json!(f.transparent_dither.unwrap_or(false));
                }
                Some(v)
            };
            (a.fill_color, fill)
        }
    };
    ToolStyle {
        stroke_color,
        fill_color,
        new_fill,
        new_stroke,
        new_brush,
    }
}

impl Session {
    /// The current tool attributes.
    pub fn tool_style(&self) -> ToolStyle {
        let o = &self.tools.options;
        ToolStyle {
            stroke_color: self.tools.stroke_color,
            fill_color: self.tools.fill_color,
            new_fill: o.new_fill.clone(),
            new_stroke: o.new_stroke.clone(),
            new_brush: o.new_brush.clone(),
        }
    }

    /// Sets the tool attributes (colours with [`SessionEvent::ColorsChanged`]); `true` when something
    /// changed. Raises [`Session::tool_attrs_generation`] so that the user interface mirrors them.
    ///
    /// [`SessionEvent::ColorsChanged`]: crate::session::SessionEvent::ColorsChanged
    pub fn set_tool_style(&mut self, st: ToolStyle) -> bool {
        let cols =
            self.tools.stroke_color != st.stroke_color || self.tools.fill_color != st.fill_color;
        let o = &mut self.tools.options;
        let opts = o.new_fill != st.new_fill
            || o.new_stroke != st.new_stroke
            || o.new_brush != st.new_brush;
        if opts {
            o.new_fill = st.new_fill;
            o.new_stroke = st.new_stroke;
            o.new_brush = st.new_brush;
        }
        if cols {
            self.set_colors(Some(st.stroke_color), Some(st.fill_color));
        }
        if cols || opts {
            self.tool_attrs_gen += 1;
        }
        cols || opts
    }

    /// Path attributes and stroke placement (0 inside, 1 centre, 2 outside) of a new object drawn
    /// with the tool attributes (`dom.pathAttributes`): fill (none included), stroke colour, tip,
    /// size and placement. A gradient or pattern fill of the tool gives a solid fill here.
    pub fn tool_path_attrs(&self) -> (PathAttrs, i32) {
        let st = self.tool_style();
        let opaque = |c: Argb| Argb(c.0 | 0xff00_0000);
        let mut a = PathAttrs::solid_fill(opaque(st.fill_color));
        if st.fill_is_none() {
            a.fill = None;
        }
        a.stroke_color = opaque(st.stroke_color);
        a.brush = (!st.stroke_is_none()).then(|| {
            crate::commands::panel::tool_brush_from(st.new_stroke.as_ref(), st.new_brush.as_ref())
        });
        (a, placement_id(st.placement()))
    }

    /// Colours the toolbox wells show (stroke, fill; alpha 0 = "none", drawn as a white box with a
    /// red diagonal). Measured (RE spec 59 sections 2.2 and 4): while several objects are selected
    /// the wells show the topmost one's colours, while an auto shape is selected those of its path;
    /// this is only a display, the tool attributes stay. Otherwise the tool attributes, with "none"
    /// for a stroke or fill of the kind none.
    pub fn tool_well_colors(&self) -> (Argb, Argb) {
        let (stroke, fill) = self.tool_well_colors_raw();
        if self.mask_selected() {
            (mask_gray(stroke), mask_gray(fill))
        } else {
            (stroke, fill)
        }
    }

    /// Is exactly one mask selected (mask editing)? Then the colour wells show grey (RE spec 68
    /// §2.1, §2.4 rule 4).
    pub fn mask_selected(&self) -> bool {
        self.active().is_some_and(|d| {
            matches!(d.selection.elements.as_slice(), [id] if crate::docops::mask_owner(&d.doc, d.page, *id).is_some())
        })
    }

    fn tool_well_colors_raw(&self) -> (Argb, Argb) {
        if let Some(d) = self.active() {
            let shown = match d.selection.elements.as_slice() {
                [] => None,
                [id] => crate::docops::element_on_page(&d.doc, d.page, *id)
                    .filter(|e| matches!(&e.kind, ElementKind::Group(g) if g.smart_shape.is_some()))
                    .and_then(|e| well_colors_of(&e)),
                ids => crate::docops::paths_sorted(&d.doc, d.page, ids)
                    .first()
                    .and_then(|p| d.doc.element_at(p))
                    .and_then(|e| well_colors_of(e)),
            };
            if let Some(c) = shown {
                return c;
            }
        }
        let st = self.tool_style();
        let none = |c: Argb| Argb(c.0 & 0x00ff_ffff);
        (
            if st.stroke_is_none() {
                none(st.stroke_color)
            } else {
                st.stroke_color
            },
            if st.fill_is_none() {
                none(st.fill_color)
            } else {
                st.fill_color
            },
        )
    }

    /// "Default Colours" (button and D, `color.defaults`, RE spec 59 section 3.3): stroke and fill
    /// colour from the preferences (black and white); tip, size and placement stay. A stroke
    /// "none" becomes `bn_1-Pixel Anti-Aliased` 1 px, a fill "none" a solid fill.
    pub fn default_tool_colors(&mut self) {
        let (stroke, fill) = self.prefs.default_colors();
        let mut st = self.tool_style();
        if st.stroke_is_none() {
            let place = st.placement().to_string();
            st.new_stroke = one_pixel_aa_stroke(1, &place);
            st.new_brush = None;
        }
        if st.fill_is_none() {
            st.new_fill = None;
        }
        st.stroke_color = stroke;
        st.fill_color = fill;
        self.set_tool_style(st);
    }

    /// The fill kind "none" belongs to the document (RE spec 59 section 2.3): the document being
    /// left keeps whether the tool fill is none.
    pub(crate) fn note_document_tool_fill(&mut self) {
        let none = self.tool_style().fill_is_none();
        if let Some(d) = self.active_mut() {
            d.tool_fill_none = none;
        }
    }

    /// Entering a document: a new or opened one starts with a solid fill in the session's fill
    /// colour (never "none"); one that had the fill none gets it back. Stroke (none included),
    /// tip, size, placement and the colours stay session-wide.
    pub(crate) fn enter_document_tool_fill(&mut self) {
        let Some(want_none) = self.active().map(|d| d.tool_fill_none) else {
            return;
        };
        let mut st = self.tool_style();
        if st.fill_is_none() == want_none {
            return;
        }
        if want_none {
            st.new_fill = Some(json!({"category": "none"}));
        } else {
            st.fill_color = Argb(st.fill_color.0 | 0xff00_0000);
            if st
                .new_fill
                .as_ref()
                .and_then(|v| v.get("category"))
                .and_then(|v| v.as_str())
                == Some("none")
            {
                st.new_fill = None;
            }
        }
        self.set_tool_style(st);
    }

    /// Counts the changes of the tool attributes made by the engine (follow, scripts).
    pub fn tool_attrs_generation(&self) -> u64 {
        self.tool_attrs_gen
    }

    /// A running script holds the follow: selection changes inside it do not change the tool
    /// attributes (measured, RE spec 58 §2.2). Pair with [`Session::release_tool_follow`].
    pub fn hold_tool_follow(&mut self) {
        self.follow_hold += 1;
    }

    /// Ends a [`Session::hold_tool_follow`].
    pub fn release_tool_follow(&mut self) {
        self.follow_hold = self.follow_hold.saturating_sub(1);
    }

    /// The tool attributes take on those of the selected object when the active document, its
    /// content or its selection changed since the last look and exactly one path or text is
    /// selected; anything else (nothing, several objects, groups, auto shapes …) keeps them.
    /// Does nothing while a script or a command group runs. `true` when the attributes changed.
    pub fn follow_selection(&mut self) -> bool {
        self.follow_selection_or(None)
    }

    /// [`Session::follow_selection`] after Undo, Redo and History jumps (TW-186 with Q-58, RE spec
    /// 58 §1 and §2): when a selected object supplies tool attributes, the toolbox follows it (the
    /// restored colours of a Swap Colours row equal that object's colours in the reference, which
    /// follows the selection before the user can swap); otherwise (nothing selected, only bitmaps
    /// and the like, or the follow is held) the toolbox colours `fallback` stored with the crossed
    /// row come back. Exactly one of the two writes the toolbox, so it never shows both in turn.
    pub(crate) fn follow_selection_or(&mut self, fallback: Option<(Argb, Argb)>) -> bool {
        if let Some(st) = self.selection_style_to_follow() {
            return self.set_tool_style(st);
        }
        match fallback {
            Some((stroke, fill))
                if (self.tools.stroke_color, self.tools.fill_color) != (stroke, fill) =>
            {
                self.set_colors(Some(stroke), Some(fill));
                true
            }
            _ => false,
        }
    }

    /// The tool attributes of the one selected path or text, when the active document, its content
    /// or its selection changed since the last look (noted here); `None` while a script or a
    /// command group holds the follow, when nothing changed, or when not exactly one path or text
    /// is selected.
    fn selection_style_to_follow(&mut self) -> Option<ToolStyle> {
        if self.follow_hold > 0 || self.group_depth > 0 {
            return None;
        }
        let d = self.active()?;
        if let Some((id, generation, sel)) = &self.follow_seen
            && *id == d.id
            && *generation == d.generation
            && *sel == d.selection.elements
        {
            return None;
        }
        let key = (d.id, d.generation, d.selection.elements.clone());
        let style = match d.selection.elements.as_slice() {
            [id] => crate::docops::element_on_page(&d.doc, d.page, *id)
                .and_then(|e| style_of_element(&e)),
            _ => None,
        };
        self.follow_seen = Some(key);
        style
    }
}

/// The grey a colour well shows while a mask is selected: Rec. 709 luma of the colour, alpha kept
/// (red `#ff0000` → `#363636`, RE spec 68 §2.1).
pub fn mask_gray(c: Argb) -> Argb {
    let l = fp_color::models::luma709(c.r(), c.g(), c.b()) as u32;
    Argb((c.0 & 0xff00_0000) | (l << 16) | (l << 8) | l)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A path with the factory tool stroke and a solid fill gives back the factory tool attributes:
    /// no stroke parameters, no full brush, no fill parameters.
    #[test]
    fn factory_stroke_round_trips() {
        let mut a = PathAttrs::solid_fill(Argb(0xffff_ffff));
        a.stroke_color = Argb(0xff00_0000);
        a.brush = Some(crate::commands::panel::default_tool_brush(1));
        let st = style_from(&a, 1);
        assert_eq!(st.new_stroke, None);
        assert_eq!(st.new_brush, None, "{:?}", a.brush);
        assert_eq!(st.new_fill, None);
    }
}

//! Tool attributes that follow the selection (Q-58, TW-192, RE spec 58 §2).
//!
//! Measured in the reference application: the tool attributes (fill, stroke colour, tip, size and
//! placement; `dom.pathAttributes` in scripts) take on the attributes of the last selected object
//! once the user interface is idle, and keep them after deselecting. With several objects selected
//! the topmost one counts (like the inspector, TW-120). Inside one script nothing follows; the next
//! script sees the change.
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

    /// Stroke placement of new objects: "inside", "center" or "outside".
    pub fn placement(&self) -> &str {
        self.new_stroke
            .as_ref()
            .and_then(|v| v.get("placement"))
            .and_then(|v| v.as_str())
            .unwrap_or("center")
    }
}

/// Tool attributes taken from an element: paths and texts directly, groups and compound shapes from
/// the first child that has path attributes (like the inspector); `None` for bitmaps, symbols, web
/// objects and the like.
pub fn style_of_element(e: &Element, current: &ToolStyle) -> Option<ToolStyle> {
    match &e.kind {
        ElementKind::Path(p) => Some(style_from(&p.attrs, p.stroke_placement, current)),
        ElementKind::Text(t) => Some(style_from(&t.attrs, t.stroke_placement, current)),
        ElementKind::Group(g) => g.children.iter().find_map(|c| style_of_element(c, current)),
        ElementKind::CompoundShape(c) => c
            .result
            .as_ref()
            .and_then(|r| style_of_element(r, current))
            .or_else(|| c.operands.iter().find_map(|o| style_of_element(o, current))),
        _ => None,
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
fn style_from(a: &PathAttrs, placement: i32, current: &ToolStyle) -> ToolStyle {
    let place = placement_name(placement);
    let (stroke_color, new_stroke, new_brush) = match &a.brush {
        // no stroke: the stroke kind "none", the colour stays (measured: brush null keeps its
        // brushColor, which the 1-pixel line of `addNewLine` takes)
        None => (a.stroke_color, Some(json!({"none": true})), None),
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
        // no fill: the colour of the tools panel stays (as the inspector's "none")
        None => (current.fill_color, Some(json!({"category": "none"}))),
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
                v["edge"] = json!(f.edge.clamp(0, 2));
                if f.edge == 2 {
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

    /// The tool attributes take on those of the topmost selected object when the active document,
    /// its content or its selection changed since the last look; an empty selection keeps them.
    /// Does nothing while a script or a command group runs. `true` when the attributes changed.
    pub fn follow_selection(&mut self) -> bool {
        if self.follow_hold > 0 || self.group_depth > 0 {
            return false;
        }
        let Some(d) = self.active() else {
            return false;
        };
        if let Some((id, generation, sel)) = &self.follow_seen
            && *id == d.id
            && *generation == d.generation
            && *sel == d.selection.elements
        {
            return false;
        }
        let key = (d.id, d.generation, d.selection.elements.clone());
        let current = self.tool_style();
        let style = if d.selection.elements.is_empty() {
            None
        } else {
            crate::docops::paths_sorted(&d.doc, d.page, &d.selection.elements)
                .iter()
                .filter_map(|p| d.doc.element_at(p))
                .find_map(|e| style_of_element(e, &current))
        };
        self.follow_seen = Some(key);
        match style {
            Some(st) => self.set_tool_style(st),
            None => false,
        }
    }
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
        let cur = Session::default().tool_style();
        let st = style_from(&a, 1, &cur);
        assert_eq!(st.new_stroke, None);
        assert_eq!(st.new_brush, None, "{:?}", a.brush);
        assert_eq!(st.new_fill, None);
    }
}

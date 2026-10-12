//! Befehle der Zeichenwerkzeuge und Bedienfelder: Formen, Pfade, Text, Hotspots/Segmente,
//! Objektattribute (Eigenschaften-Inspektor), Ebenen, Seiten, Status, Farben, Werkzeugwahl, Verlauf.

use super::{argb_hex, insert_new, shape_attrs, sel_err, take_ids, target_ids, with_doc, with_targets, DOC};
use crate::command::{cmd, CommandOutput, CommandRegistry, ParamKind, Params, Requirement};
use crate::docops;
use crate::error::{EngineError, Result};
use crate::geometry;
use crate::history::RecordedCommand;
use crate::session::{DocumentState, Session};
use crate::tools;
use fp_color::{Argb, BlendMode, Opacity};
use fp_geom::{Point, Rect};
use fp_model::effects::{Effect, EffectKind, EffectList};
use fp_model::element::{Element, ElementKind};
use fp_model::layer::{LayerKind, LayerSlot};
use fp_model::{Brush, Contour, Fill, Gradient, GradientStop, GradientStops, LayerId, StopList, WebShape};
use serde_json::{json, Value};
use std::sync::Arc;

/// Liest Konturen `[{closed, nodes: [{anchor, in, out}]}]` (Henkel optional).
pub(crate) fn parse_contours(p: &Params) -> Result<Vec<Contour>> {
    let err = |key: &str| EngineError::InvalidParams { name: p.command.clone(), message: fp_i18n::t(key) };
    let arr = p.get("contours").and_then(|v| v.as_array()).ok_or_else(|| err("err-param-contours"))?;
    let mut out = Vec::new();
    for c in arr {
        let closed = c.get("closed").and_then(|v| v.as_bool()).unwrap_or(false);
        let nodes = c.get("nodes").and_then(|v| v.as_array()).ok_or_else(|| err("err-param-nodes-missing"))?;
        let mut ns = Vec::new();
        for n in nodes {
            let (a, i, o) = if let Some(a) = n.get("anchor") {
                let a = crate::command::parse_point(a).ok_or_else(|| err("anchor"))?;
                let i = n.get("in").and_then(crate::command::parse_point).unwrap_or(a);
                let o = n.get("out").and_then(crate::command::parse_point).unwrap_or(a);
                (a, i, o)
            } else {
                let a = crate::command::parse_point(n).ok_or_else(|| err("err-param-node"))?;
                (a, a, a)
            };
            ns.push((a, i, o));
        }
        if !ns.is_empty() {
            out.push(geometry::contour_from_nodes(&ns, closed));
        }
    }
    if out.is_empty() {
        return Err(err("err-param-no-contours"));
    }
    Ok(out)
}

fn finish_shape(s: &mut Session, mut e: Element, p: &Params) -> Result<CommandOutput> {
    if let Some(n) = p.opt_str("name")? {
        e.common.name = Some(n);
    }
    with_doc(s, |d| {
        let id = insert_new(d, e)?;
        Ok(CommandOutput::doc_with(json!({"id": id.0})))
    })
}

fn attrs_from(s: &Session, p: &Params, default_fill: bool, default_stroke: f32) -> Result<fp_model::PathAttrs> {
    let fill = p.bool_or("fill", default_fill)?;
    let sw = p.f64_or("strokeWidth", default_stroke as f64)? as f32;
    let mut a = shape_attrs(s, fill, sw);
    if let Some(c) = p.opt_color("fillColor")? {
        a.fill_color = c;
        if fill && a.fill.is_none() && c.a() > 0 {
            a.fill = Some(Fill::solid());
        }
    }
    if let Some(c) = p.opt_color("strokeColor")? {
        a.stroke_color = c;
        if sw > 0.0 && a.brush.is_none() && c.a() > 0 {
            a.brush = Some(Brush::basic(sw.round().max(1.0) as i32));
        }
    }
    Ok(a)
}

fn shape_opts(b: crate::command::CommandBuilder) -> crate::command::CommandBuilder {
    b.opt("fill", ParamKind::Bool, "Füllung (Vorgabe: ja, Füllfarbe des Werkzeugbedienfelds)")
        .opt("fillColor", ParamKind::Color, "Füllfarbe")
        .opt("strokeWidth", ParamKind::Number { min: Some(0.0), max: Some(1000.0) }, "Strichbreite (0 = kein Strich)")
        .opt("strokeColor", ParamKind::Color, "Strichfarbe")
        .opt("name", ParamKind::String, "Objektname")
}

fn layer_param(d: &DocumentState, p: &Params) -> Result<LayerId> {
    match p.opt_i64("layer")? {
        Some(v) => Ok(LayerId(v as u64)),
        None => d.layer.ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-active-layer"))),
    }
}

fn edit_layer(s: &mut Session, p: &Params, f: impl FnOnce(&mut fp_model::Layer)) -> Result<CommandOutput> {
    with_doc(s, |d| {
        let id = layer_param(d, p)?;
        let page = d.page;
        let slot = docops::layer_slot(&d.doc, page, id).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-layer")))?;
        let l = docops::layer_mut(Arc::make_mut(&mut d.doc), page, slot).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-layer")))?;
        f(l);
        Ok(CommandOutput::doc())
    })
}

/// Setzt die Effektliste eines Elements; bei Rechteck-Grundformen auch die Kopie in `PRIv/EFLv`,
/// aus der das Original die Effekte liest.
pub(crate) fn set_effects(e: &mut Element, list: Option<EffectList>) {
    if let ElementKind::Path(p) = &mut e.kind
        && let Some(pr) = &mut p.primitive
    {
        pr.effects = list.clone();
    }
    e.common.effects = list;
}

fn edit_effects(s: &mut Session, p: &Params, f: impl Fn(&mut EffectList) -> Result<()>) -> Result<CommandOutput> {
    with_doc(s, |d| {
        let ids = target_ids(d, p)?;
        let page = d.page;
        let doc = Arc::make_mut(&mut d.doc);
        for id in ids {
            let mut err = None;
            docops::edit_by_id(doc, page, id, |e| {
                let mut l = e.common.effects.clone().unwrap_or_default();
                match f(&mut l) {
                    Ok(()) => set_effects(e, if l.effects.is_empty() { None } else { Some(l) }),
                    Err(x) => err = Some(x),
                }
            })
            .ok_or(EngineError::ElementNotFound(id.0))?;
            if let Some(e) = err {
                return Err(e);
            }
        }
        Ok(CommandOutput::doc())
    })
}

/// MOA-ID zu einem Effektnamen (z. B. `dropShadow`, `innerGlow`, `gaussianBlur`) oder direkt einer ID.
pub fn effect_moa_id(name: &str) -> Option<String> {
    if name.starts_with('{') {
        return Some(name.to_ascii_lowercase());
    }
    let k = match name.to_ascii_lowercase().as_str() {
        "bevel" | "innerbevel" | "outerbevel" | "emboss" | "glow" => EffectKind::BevelFamily,
        "dropshadow" => EffectKind::DropShadow,
        "innershadow" => EffectKind::InnerShadow,
        "innerglow" => EffectKind::InnerGlow,
        "solidshadow" => EffectKind::SolidShadow,
        "colorfill" => EffectKind::ColorFill,
        "gaussianblur" => EffectKind::GaussianBlur,
        "blur" => EffectKind::Blur,
        "blurmore" => EffectKind::BlurMore,
        "motionblur" => EffectKind::MotionBlur,
        "radialblur" => EffectKind::RadialBlur,
        "zoomblur" => EffectKind::ZoomBlur,
        "sharpen" => EffectKind::Sharpen,
        "sharpenmore" => EffectKind::SharpenMore,
        "unsharpmask" => EffectKind::UnsharpMask,
        "addnoise" => EffectKind::AddNoise,
        "brightnesscontrast" => EffectKind::BrightnessContrast,
        "huesaturation" => EffectKind::HueSaturation,
        "curves" => EffectKind::Curves,
        "levels" => EffectKind::Levels,
        "autolevels" => EffectKind::AutoLevels,
        "invert" => EffectKind::Invert,
        "findedges" => EffectKind::FindEdges,
        "converttoalpha" => EffectKind::ConvertToAlpha,
        _ => return None,
    };
    k.moa_id().map(|s| s.to_string())
}

fn web_object(s: &mut Session, p: &Params, slice: bool) -> Result<CommandOutput> {
    let shape = p.str("shape")?;
    let (bounds, polygon) = match shape.as_str() {
        "polygon" => {
            let pts = p.points("points")?;
            if pts.len() < 3 {
                return Err(EngineError::InvalidParams { name: p.command.clone(), message: fp_i18n::t("err-param-three-points") });
            }
            let c = geometry::polygon_contour(&pts, true);
            (geometry::contours_bounds(std::slice::from_ref(&c)).unwrap_or_default(), Some(c))
        }
        _ => (p.rect("rect")?.normalized(), None),
    };
    let url = p.opt_str("url")?;
    with_doc(s, |d| {
        let doc = Arc::make_mut(&mut d.doc);
        let mut idg = take_ids(doc);
        let mut e = if slice { Element::new_slice(&mut idg, bounds) } else { Element::new_hotspot(&mut idg, bounds) };
        doc.ids = idg;
        if let ElementKind::WebObject(w) = &mut e.kind {
            w.shape = match shape.as_str() {
                "circle" => WebShape::Circle,
                "polygon" => WebShape::Polygon,
                _ => WebShape::Rect,
            };
            w.polygon = polygon;
            if let Some(u) = url {
                w.url = Some(u);
            }
        }
        let id = insert_new(d, e)?;
        Ok(CommandOutput::doc_with(json!({"id": id.0})))
    })
}

fn gradient_fill(colors: &[Argb]) -> Fill {
    let n = colors.len().max(2);
    let stops: Vec<GradientStop> = (0..n)
        .map(|i| GradientStop { position: i as f32 / (n - 1) as f32, color: colors.get(i).copied().unwrap_or(Argb(0xff00_0000)), extra: Default::default() })
        .collect();
    let opac: Vec<GradientStop> = stops.iter().map(|s| GradientStop { position: s.position, color: Argb(((s.color.a() as u32) << 24) | 0x00ff_ffff), extra: Default::default() }).collect();
    let mut f = Fill::solid();
    f.category = "fc_Linear".into();
    f.name = "Linear".into();
    f.user_name = Some("Linear".into());
    f.shape = 2;
    f.gradient = Some(Gradient {
        category: "fc_Linear".into(),
        name: "Linear".into(),
        user_name: None,
        legacy_stops: None,
        stops: Some(GradientStops { colors: StopList { stops, extra: Default::default() }, opacities: StopList { stops: opac, extra: Default::default() }, dither: None, dither_value: None, extra: Default::default() }),
        extra: Default::default(),
    });
    f
}

/// Farbparameter-Liste `colors`.
pub(crate) fn colors_param(p: &Params, fallback: &[Argb]) -> Result<Vec<Argb>> {
    match p.get("colors").and_then(|v| v.as_array()) {
        Some(a) => a
            .iter()
            .map(|v| v.as_str().and_then(fp_color::Rgba8::parse_hex).map(|c| c.to_argb()).ok_or_else(|| EngineError::InvalidParams { name: p.command.clone(), message: fp_i18n::t("err-param-colors") }))
            .collect(),
        None => Ok(fallback.to_vec()),
    }
}

pub(super) fn register(r: &mut CommandRegistry) {
    // Formen
    r.add(
        shape_opts(cmd("draw.rectangle").label("Rechteck").requires(DOC))
            .param("rect", ParamKind::Rect, "Rechteck")
            .opt_default("roundness", ParamKind::Number { min: Some(0.0), max: Some(1.0) }, json!(0), "Eckenrundung 0..1")
            .run(|s, p| {
                let rr = p.rect("rect")?.normalized();
                let rd = p.f64_or("roundness", 0.0)?.clamp(0.0, 1.0);
                let a = attrs_from(s, p, true, 0.0)?;
                let e = {
                    let d = s.doc_mut()?;
                    let doc = Arc::make_mut(&mut d.doc);
                    let mut idg = take_ids(doc);
                    let mut e = Element::new_rectangle(&mut idg, rr, rd as f32, a);
                    doc.ids = idg;
                    if let ElementKind::Path(pe) = &mut e.kind {
                        pe.contours = vec![geometry::rect_contour(rr, rd)];
                    }
                    e
                };
                finish_shape(s, e, p)
            }),
    );
    r.add(shape_opts(cmd("draw.ellipse").label("Ellipse").requires(DOC)).param("rect", ParamKind::Rect, "umschließendes Rechteck").run(|s, p| {
        let rr = p.rect("rect")?.normalized();
        let a = attrs_from(s, p, true, 0.0)?;
        let e = {
            let d = s.doc_mut()?;
            let doc = Arc::make_mut(&mut d.doc);
            let mut idg = take_ids(doc);
            let e = Element::new_path(&mut idg, vec![geometry::ellipse_contour(rr)], a);
            doc.ids = idg;
            e
        };
        finish_shape(s, e, p)
    }));
    r.add(
        shape_opts(cmd("draw.polygon").label("Polygon").requires(DOC))
            .param("center", ParamKind::Point, "Mittelpunkt")
            .param("corner", ParamKind::Point, "erste Ecke")
            .opt_default("sides", ParamKind::Integer { min: Some(3), max: Some(360) }, json!(5), "Seiten bzw. Zacken")
            .opt_default("star", ParamKind::Bool, json!(false), "Stern")
            .opt("spikiness", ParamKind::Number { min: Some(0.0), max: Some(1.0) }, "Spitzigkeit 0..1 (fehlt = automatisch)")
            .run(|s, p| {
                let c = geometry::polygon_tool_contour(p.point("center")?, p.point("corner")?, p.i64_or("sides", 5)?.clamp(3, 360) as usize, p.bool_or("star", false)?, p.opt_f64("spikiness")?)
                    .ok_or_else(|| EngineError::InvalidParams { name: p.command.clone(), message: fp_i18n::t("err-param-radius-zero") })?;
                let a = attrs_from(s, p, true, 0.0)?;
                let e = {
                    let d = s.doc_mut()?;
                    let doc = Arc::make_mut(&mut d.doc);
                    let mut idg = take_ids(doc);
                    let e = Element::new_path(&mut idg, vec![c], a);
                    doc.ids = idg;
                    e
                };
                finish_shape(s, e, p)
            }),
    );
    r.add(shape_opts(cmd("draw.line").label("Linie").requires(DOC)).param("from", ParamKind::Point, "Anfang").param("to", ParamKind::Point, "Ende").run(|s, p| {
        let c = geometry::polygon_contour(&[p.point("from")?, p.point("to")?], false);
        let a = attrs_from(s, p, false, 1.0)?;
        let e = {
            let d = s.doc_mut()?;
            let doc = Arc::make_mut(&mut d.doc);
            let mut idg = take_ids(doc);
            let e = Element::new_path(&mut idg, vec![c], a);
            doc.ids = idg;
            e
        };
        finish_shape(s, e, p)
    }));
    r.add(
        shape_opts(cmd("draw.path").label("Pfad").requires(DOC))
            .param("contours", ParamKind::Json, "[{closed, nodes: [{anchor: [x, y], in: [x, y], out: [x, y]}]}]")
            .run(|s, p| {
                let contours = parse_contours(p)?;
                let closed = contours.iter().any(|c| c.closed);
                let a = attrs_from(s, p, closed, 1.0)?;
                let e = {
                    let d = s.doc_mut()?;
                    let doc = Arc::make_mut(&mut d.doc);
                    let mut idg = take_ids(doc);
                    let e = Element::new_path(&mut idg, contours, a);
                    doc.ids = idg;
                    e
                };
                finish_shape(s, e, p)
            }),
    );
    r.add(
        cmd("draw.text")
            .label("Text")
            .requires(DOC)
            .param("text", ParamKind::String, "Text (Absätze mit \\r)")
            .opt("anchor", ParamKind::Point, "Punkttext: Anker (x, erste Grundlinie)")
            .opt("rect", ParamKind::Rect, "Rahmentext fester Breite")
            .opt("font", ParamKind::String, "Schrift")
            .opt("size", ParamKind::Number { min: Some(1.0), max: Some(5000.0) }, "Schriftgröße in px")
            .opt("color", ParamKind::Color, "Textfarbe (Vorgabe: Füllfarbe)")
            .opt("runs", ParamKind::Json, "formatierte Läufe [{attrs, text}] (Text-Editor)")
            .run(|s, p| {
                let text = p.str("text")?;
                let runs = super::text::parse_runs(p, &text)?;
                let font = p.opt_str("font")?.unwrap_or_else(|| s.tools.options.text_font.clone());
                let size = p.f64_or("size", s.tools.options.text_size as f64)? as f32;
                let color = p.opt_color("color")?.unwrap_or(s.tools.fill_color);
                let rect = p.opt_rect("rect")?;
                let anchor = p.opt_point("anchor")?.or_else(|| rect.map(|r| Point::new(r.left, r.top + size))).ok_or_else(|| EngineError::InvalidParams { name: p.command.clone(), message: fp_i18n::t("err-param-anchor-or-rect") })?;
                let e = {
                    let d = s.doc_mut()?;
                    let doc = Arc::make_mut(&mut d.doc);
                    let mut idg = take_ids(doc);
                    let mut e = Element::new_text(&mut idg, &text, &font, size, color, anchor);
                    doc.ids = idg;
                    if let (Some(r), ElementKind::Text(t)) = (rect, &mut e.kind) {
                        t.frame_type = Some(1);
                        t.bounds = Rect::new(r.left, r.top, r.right, r.bottom.max(r.top + size * 1.2));
                        t.anchor_rect = Some(t.bounds);
                    }
                    if let (Some(runs), ElementKind::Text(t)) = (runs, &mut e.kind) {
                        crate::tools::text::apply_runs(t, runs);
                    }
                    e
                };
                finish_shape(s, e, p)
            }),
    );
    r.add(
        cmd("web.addHotspot")
            .label("Hotspot")
            .requires(DOC)
            .param("shape", ParamKind::Enum { values: vec!["rect".into(), "circle".into(), "polygon".into()] }, "Form")
            .opt("rect", ParamKind::Rect, "Rechteck (rect/circle)")
            .opt("points", ParamKind::Json, "Punkte (polygon)")
            .opt("url", ParamKind::String, "Link")
            .run(|s, p| web_object(s, p, false)),
    );
    r.add(
        cmd("web.addSlice")
            .label("Segment")
            .requires(DOC)
            .param("shape", ParamKind::Enum { values: vec!["rect".into(), "polygon".into()] }, "Form")
            .opt("rect", ParamKind::Rect, "Rechteck")
            .opt("points", ParamKind::Json, "Punkte (polygon)")
            .opt("url", ParamKind::String, "Link")
            .run(|s, p| web_object(s, p, true)),
    );

    // Objektattribute (Eigenschaften-Inspektor)
    let ids_opt = |b: crate::command::CommandBuilder| b.opt("ids", ParamKind::Ids, "Elemente (Vorgabe: Auswahl)");
    r.add(
        ids_opt(cmd("element.setFill").label("Füllung ändern").requires(DOC))
            .opt("color", ParamKind::Color, "Füllfarbe")
            .opt_default("kind", ParamKind::Enum { values: vec!["solid".into(), "none".into()] }, json!("solid"), "Füllart")
            .run(|s, p| {
                let c = p.opt_color("color")?;
                let none = p.opt_str("kind")?.as_deref() == Some("none");
                with_doc(s, |d| {
                    let (page, ids) = (d.page, target_ids(d, p)?);
                    let doc = Arc::make_mut(&mut d.doc);
                    for id in ids {
                        docops::edit_by_id(doc, page, id, |e| {
                            docops::edit_path_attrs(e, &|a| {
                                if none {
                                    a.fill = None;
                                    return;
                                }
                                if let Some(c) = c {
                                    a.fill_color = c;
                                }
                                if a.fill.as_ref().is_none_or(|f| f.shape != 0) {
                                    a.fill = Some(Fill::solid());
                                }
                            });
                        })
                        .ok_or(EngineError::ElementNotFound(id.0))?;
                    }
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(
        ids_opt(cmd("element.setStroke").label("Strich ändern").requires(DOC))
            .opt("color", ParamKind::Color, "Strichfarbe")
            .opt("width", ParamKind::Number { min: Some(0.0), max: Some(1000.0) }, "Strichbreite (0 = kein Strich)")
            .opt("placement", ParamKind::Enum { values: vec!["inside".into(), "center".into(), "outside".into()] }, "Strichposition")
            .run(|s, p| {
                let c = p.opt_color("color")?;
                let w = p.opt_f64("width")?;
                let place = p.opt_str("placement")?.map(|v| match v.as_str() {
                    "inside" => 0,
                    "outside" => 2,
                    _ => 1,
                });
                with_doc(s, |d| {
                    let (page, ids) = (d.page, target_ids(d, p)?);
                    let doc = Arc::make_mut(&mut d.doc);
                    for id in ids {
                        docops::edit_by_id(doc, page, id, |e| {
                            if let (Some(pl), ElementKind::Path(pe)) = (place, &mut e.kind) {
                                pe.stroke_placement = pl;
                            }
                            docops::edit_path_attrs(e, &|a| {
                                if let Some(c) = c {
                                    a.stroke_color = c;
                                }
                                match w {
                                    Some(w) if w <= 0.0 => a.brush = None,
                                    Some(w) => match &mut a.brush {
                                        Some(b) => b.diameter = w.round().max(1.0) as i32,
                                        None => a.brush = Some(Brush::basic(w.round().max(1.0) as i32)),
                                    },
                                    None => {
                                        if a.brush.is_none() && c.is_some() {
                                            a.brush = Some(Brush::basic(1));
                                        }
                                    }
                                }
                            });
                        })
                        .ok_or(EngineError::ElementNotFound(id.0))?;
                    }
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(ids_opt(cmd("element.setOpacity").label("Deckkraft").requires(DOC)).param("opacity", ParamKind::Number { min: Some(0.0), max: Some(100.0) }, "Deckkraft in %").run(|s, p| {
        let o = Opacity::from_permille((p.f64("opacity")?.clamp(0.0, 100.0) * 10.0).round() as u32);
        common_edit(s, p, move |e| e.common.opacity = o)
    }));
    r.add(ids_opt(cmd("element.setBlendMode").label("Mischmodus").requires(DOC)).param("mode", ParamKind::String, "Name (normal, multiply, …) oder BLDi-Nummer").run(|s, p| {
        let m = p.str("mode")?;
        let mode = BlendMode::from_name(&m).or_else(|| m.parse::<u8>().ok().map(BlendMode)).ok_or_else(|| EngineError::InvalidParams { name: p.command.clone(), message: fp_i18n::tr!("err-unknown-blend", name = m.as_str()) })?;
        common_edit(s, p, move |e| e.common.blend = mode)
    }));
    r.add(ids_opt(cmd("element.setName").label("Objektname").requires(DOC)).param("name", ParamKind::String, "Name").run(|s, p| {
        let n = p.str("name")?;
        common_edit(s, p, move |e| e.common.name = if n.is_empty() { None } else { Some(n.clone()) })
    }));
    r.add(ids_opt(cmd("element.setVisible").label("Sichtbarkeit").requires(DOC)).param("visible", ParamKind::Bool, "sichtbar").run(|s, p| {
        let v = p.bool_or("visible", true)?;
        common_edit(s, p, move |e| e.common.visible = v)
    }));
    r.add(ids_opt(cmd("element.setLocked").label("Sperren").requires(DOC)).param("locked", ParamKind::Bool, "gesperrt").run(|s, p| {
        let v = p.bool_or("locked", true)?;
        common_edit(s, p, move |e| e.common.locked = v)
    }));
    r.add(
        cmd("element.setBounds")
            .label("Position und Größe")
            .requires(DOC)
            .opt("ids", ParamKind::Ids, "Elemente (Vorgabe: Auswahl; die Auswahl bleibt unverändert)")
            .opt("x", ParamKind::Number { min: None, max: None }, "X")
            .opt("y", ParamKind::Number { min: None, max: None }, "Y")
            .opt("width", ParamKind::Number { min: Some(0.0), max: None }, "B")
            .opt("height", ParamKind::Number { min: Some(0.0), max: None }, "H")
            .run(|s, p| {
                with_targets(s, p, |s, p| {
                    let d = s.doc()?;
                    let b = docops::ids_bounds(&d.doc, d.page, &d.selection.elements).ok_or_else(|| sel_err("err-sel-no-bounds-short"))?;
                    let x = p.f64_or("x", b.left as f64)? as f32;
                    let y = p.f64_or("y", b.top as f64)? as f32;
                    let w = p.f64_or("width", b.width() as f64)? as f32;
                    let h = p.f64_or("height", b.height() as f64)? as f32;
                    super::modify::transform_selection(s, docops::rect_to_rect(b, Rect::new(x, y, x + w, y + h)))
                })
            }),
    );
    r.add(
        ids_opt(cmd("element.addEffect").label("Effekt hinzufügen").requires(DOC))
            .param("effect", ParamKind::String, "Effekt (dropShadow, innerGlow, bevel, gaussianBlur, … oder MOA-ID)")
            .opt("params", ParamKind::Json, "Parameter {Schlüssel: Wert} wie im Dateiformat")
            .opt_default("visible", ParamKind::Bool, json!(true), "sichtbar")
            .run(|s, p| {
                let name = p.str("effect")?;
                let moa = effect_moa_id(&name).ok_or_else(|| EngineError::InvalidParams { name: p.command.clone(), message: fp_i18n::tr!("err-unknown-effect", name = name.as_str()) })?;
                let vis = p.bool_or("visible", true)?;
                let params: Vec<(String, String)> = match p.get("params") {
                    Some(Value::Object(m)) => m.iter().map(|(k, v)| (k.clone(), v.as_str().map(|s| s.to_string()).unwrap_or_else(|| v.to_string()))).collect(),
                    _ => Vec::new(),
                };
                edit_effects(s, p, move |l| {
                    let mut e = Effect::new(&moa, vis);
                    for (k, v) in &params {
                        e.set(k, v.clone());
                    }
                    l.effects.push(e);
                    Ok(())
                })
            }),
    );
    r.add(
        ids_opt(cmd("element.setEffect").label("Effekt einstellen").requires(DOC))
            .opt("effect", ParamKind::String, "Effekt (dropShadow, glow, … oder MOA-ID): ersetzt den ersten Effekt dieser Art, fehlt er, wird er angelegt")
            .opt("index", ParamKind::Integer { min: Some(0), max: None }, "statt effect: Index in der Effektliste")
            .opt("params", ParamKind::Json, "Parameter {Schlüssel: Wert} (ersetzen die vorhandenen Werte)")
            .opt("visible", ParamKind::Bool, "sichtbar")
            .run(|s, p| {
                let moa = match p.opt_str("effect")? {
                    Some(n) => Some(effect_moa_id(&n).ok_or_else(|| EngineError::InvalidParams { name: p.command.clone(), message: fp_i18n::tr!("err-unknown-effect", name = n.as_str()) })?),
                    None => None,
                };
                let index = p.opt_i64("index")?.map(|v| v.max(0) as usize);
                if moa.is_none() && index.is_none() {
                    return Err(EngineError::InvalidParams { name: p.command.clone(), message: fp_i18n::t("err-param-effect-or-index") });
                }
                let vis = p.opt_bool("visible")?;
                let params: Vec<(String, String)> = match p.get("params") {
                    Some(Value::Object(m)) => m.iter().map(|(k, v)| (k.clone(), v.as_str().map(|s| s.to_string()).unwrap_or_else(|| v.to_string()))).collect(),
                    _ => Vec::new(),
                };
                edit_effects(s, p, move |l| {
                    let pos = match (&moa, index) {
                        (_, Some(i)) if i < l.effects.len() => Some(i),
                        (_, Some(_)) => return Err(EngineError::NotFound(fp_i18n::t("err-nf-effect"))),
                        (Some(m), None) => l.effects.iter().position(|e| e.moa_id() == Some(m.as_str())),
                        (None, None) => None,
                    };
                    let i = match pos {
                        Some(i) => i,
                        None => {
                            // noch nicht vorhanden: genau einmal anlegen
                            l.effects.push(Effect::new(moa.as_deref().unwrap_or_default(), true));
                            l.effects.len() - 1
                        }
                    };
                    let e = &mut l.effects[i];
                    for (k, v) in &params {
                        e.set(k, v.clone());
                    }
                    if let Some(v) = vis {
                        e.set("EffectIsVisible", if v { "true" } else { "false" });
                    }
                    Ok(())
                })
            }),
    );
    r.add(ids_opt(cmd("element.removeEffect").label("Effekt entfernen").requires(DOC)).param("index", ParamKind::Integer { min: Some(0), max: None }, "Index in der Effektliste").run(|s, p| {
        let i = p.usize("index")?;
        edit_effects(s, p, move |l| {
            if i < l.effects.len() {
                l.effects.remove(i);
                Ok(())
            } else {
                Err(EngineError::NotFound(fp_i18n::t("err-nf-effect")))
            }
        })
    }));
    r.add(
        ids_opt(cmd("element.setEffectVisible").label("Effekt ein-/ausblenden").requires(DOC))
            .param("index", ParamKind::Integer { min: Some(0), max: None }, "Index")
            .param("visible", ParamKind::Bool, "sichtbar")
            .run(|s, p| {
                let i = p.usize("index")?;
                let v = p.bool_or("visible", true)?;
                edit_effects(s, p, move |l| match l.effects.get_mut(i) {
                    Some(e) => {
                        e.set_visible(v);
                        Ok(())
                    }
                    None => Err(EngineError::NotFound(fp_i18n::t("err-nf-effect"))),
                })
            }),
    );
    r.add(
        ids_opt(cmd("element.setGradient").label("Verlauf").requires(DOC))
            .param("from", ParamKind::Point, "Verlaufsanfang")
            .param("to", ParamKind::Point, "Verlaufsende")
            .opt("colors", ParamKind::Json, "Farben [#rrggbb, …] (Vorgabe: Füll- → Strichfarbe)")
            .run(|s, p| {
                let (a, b) = (p.point("from")?, p.point("to")?);
                let colors = colors_param(p, &[s.tools.fill_color, s.tools.stroke_color])?;
                let fill = gradient_fill(&colors);
                with_doc(s, |d| {
                    let (page, ids) = (d.page, target_ids(d, p)?);
                    let doc = Arc::make_mut(&mut d.doc);
                    for id in ids {
                        docops::edit_by_id(doc, page, id, |e| {
                            docops::edit_path_attrs(e, &|x| x.fill = Some(fill.clone()));
                            let h = fp_model::FillHandles { valid: true, start: a, end: b, second: Point::new(a.x - (b.y - a.y), a.y + (b.x - a.x)) };
                            match &mut e.kind {
                                ElementKind::Path(pe) => pe.fill_handles = h,
                                ElementKind::Text(t) => t.fill_handles = h,
                                _ => {}
                            }
                        });
                    }
                    Ok(CommandOutput::doc())
                })
            }),
    );

    // Ebenen
    r.add(
        cmd("layer.add")
            .label("Ebene hinzufügen")
            .requires(DOC)
            .opt("name", ParamKind::String, "Name")
            .run(|s, p| {
                let n = p.opt_str("name")?;
                with_doc(s, |d| {
                    let id = super::edit::add_layer(d, n.as_deref(), LayerKind::Normal)?;
                    Ok(CommandOutput::doc_with(json!({"layer": id.0})))
                })
            }),
    );
    r.add(cmd("layer.delete").label("Ebene löschen").requires(DOC).opt("layer", ParamKind::Integer { min: Some(0), max: None }, "Ebenen-ID (Vorgabe: aktive)").run(|s, p| {
        with_doc(s, |d| {
            let id = layer_param(d, p)?;
            let page = d.page;
            let normals = docops::own_layers(&d.doc, page).iter().filter(|(_, l)| l.kind == LayerKind::Normal).count();
            let slot = docops::layer_slot(&d.doc, page, id).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-layer")))?;
            let is_web = docops::own_layers(&d.doc, page).iter().any(|(s, l)| *s == slot && l.kind == LayerKind::Web);
            if is_web || normals <= 1 {
                return Err(EngineError::Failed(fp_i18n::t("err-layer-no-delete")));
            }
            let pg = Arc::make_mut(&mut d.doc).page_mut(page).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
            pg.root.layers.remove(slot);
            d.layer = docops::top_normal_layer(&d.doc, page);
            d.selection.clear();
            Ok(CommandOutput::doc())
        })
    }));
    r.add(cmd("layer.rename").label("Ebene umbenennen").requires(DOC).opt("layer", ParamKind::Integer { min: Some(0), max: None }, "Ebenen-ID").param("name", ParamKind::String, "Name").run(|s, p| {
        let n = p.str("name")?;
        edit_layer(s, p, move |l| l.name = n)
    }));
    r.add(cmd("layer.setVisible").label("Ebene ein-/ausblenden").requires(DOC).opt("layer", ParamKind::Integer { min: Some(0), max: None }, "Ebenen-ID").param("visible", ParamKind::Bool, "sichtbar").run(|s, p| {
        let v = p.bool_or("visible", true)?;
        edit_layer(s, p, move |l| {
            if l.visibility.is_empty() {
                l.visibility.push(v);
            } else {
                l.visibility[0] = v;
            }
        })
    }));
    r.add(cmd("layer.setLocked").label("Ebene sperren").requires(DOC).opt("layer", ParamKind::Integer { min: Some(0), max: None }, "Ebenen-ID").param("locked", ParamKind::Bool, "gesperrt").run(|s, p| {
        let v = p.bool_or("locked", true)?;
        edit_layer(s, p, move |l| {
            for c in &mut l.cels {
                Arc::make_mut(c).locked = v;
            }
        })
    }));
    r.add(cmd("layer.setShared").label("Ebene über Status teilen").requires(DOC).opt("layer", ParamKind::Integer { min: Some(0), max: None }, "Ebenen-ID").param("shared", ParamKind::Bool, "geteilt").run(|s, p| {
        let v = p.bool_or("shared", true)?;
        let frames = s.doc()?.frame_count();
        edit_layer(s, p, move |l| {
            l.shared_across_frames = v;
            if !v {
                let first = l.cels.first().cloned().unwrap_or_else(|| Arc::new(fp_model::Cel::empty()));
                while l.cels.len() < frames {
                    l.cels.push(Arc::new((*first).clone()));
                }
            }
        })
    }));
    r.add(cmd("layer.setActive").label("Aktive Ebene").requires(DOC).param("layer", ParamKind::Integer { min: Some(0), max: None }, "Ebenen-ID").run(|s, p| {
        with_doc(s, |d| {
            let id = layer_param(d, p)?;
            // eigene Ebene oder über Seiten geteilte Ebene dieser Seite
            let shared = d.doc.pages.get(d.page).is_some_and(|pg| {
                pg.root.layers.iter().any(|sl| matches!(sl, LayerSlot::Shared(r) if d.doc.shared_layer(&r.id).and_then(|x| x.layer.as_ref()).is_some_and(|l| l.id == id)))
            });
            if !shared {
                docops::layer_slot(&d.doc, d.page, id).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-layer")))?;
            }
            d.layer = Some(id);
            Ok(CommandOutput::doc())
        })
    }));
    r.add(
        cmd("layer.move")
            .label("Ebene verschieben")
            .requires(DOC)
            .opt("layer", ParamKind::Integer { min: Some(0), max: None }, "Ebenen-ID")
            .param("index", ParamKind::Integer { min: Some(0), max: None }, "neue Position in der Ebenenliste (0 = unten)")
            .run(|s, p| {
                let to = p.usize("index")?;
                with_doc(s, |d| {
                    let id = layer_param(d, p)?;
                    let page = d.page;
                    let slot = docops::layer_slot(&d.doc, page, id).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-layer")))?;
                    let pg = Arc::make_mut(&mut d.doc).page_mut(page).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
                    let l = pg.root.layers.remove(slot);
                    let to = to.min(pg.root.layers.len());
                    pg.root.layers.insert(to, l);
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(cmd("layer.duplicate").label("Ebene duplizieren").requires(DOC).opt("layer", ParamKind::Integer { min: Some(0), max: None }, "Ebenen-ID").run(|s, p| {
        with_doc(s, |d| {
            let id = layer_param(d, p)?;
            let page = d.page;
            let slot = docops::layer_slot(&d.doc, page, id).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-layer")))?;
            let doc = Arc::make_mut(&mut d.doc);
            let Some(LayerSlot::Own(l)) = doc.pages.get(page).and_then(|pg| pg.root.layers.get(slot)).cloned() else { return Err(EngineError::NotFound(fp_i18n::t("err-nf-layer"))) };
            let mut nl = (*l).clone();
            let mut idg = take_ids(doc);
            nl.reassign_ids(&mut idg);
            doc.ids = idg;
            nl.name = fp_i18n::tr!("msg-default-copy", name = nl.name.as_str());
            let nid = nl.id;
            doc.page_mut(page).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?.root.layers.insert(slot + 1, LayerSlot::Own(Arc::new(nl)));
            d.layer = Some(nid);
            Ok(CommandOutput::doc_with(json!({"layer": nid.0})))
        })
    }));

    // Seiten
    r.add(cmd("page.add").label("Seite hinzufügen").requires(DOC).opt("name", ParamKind::String, "Name").run(|s, p| {
        let n = p.opt_str("name")?;
        s.execute("edit.insert.page", json!({"name": n})).map(|o| CommandOutput { effect: crate::command::Effect::None, ..o })
    }));
    r.add(cmd("page.delete").label("Seite löschen").requires(&[Requirement::Document, Requirement::MultiplePages]).opt("index", ParamKind::Integer { min: Some(0), max: None }, "Seitenindex (Vorgabe: aktuelle)").run(|s, p| {
        with_doc(s, |d| {
            let i = p.opt_i64("index")?.map(|v| v.max(0) as usize).unwrap_or(d.page);
            let doc = Arc::make_mut(&mut d.doc);
            if i >= doc.pages.len() || doc.pages.len() <= 1 {
                return Err(EngineError::NotFound(fp_i18n::t("err-nf-page")));
            }
            doc.pages.remove(i);
            if doc.current_page >= doc.pages.len() {
                doc.current_page = doc.pages.len() - 1;
            }
            doc.invalidate_root_copy();
            d.page = d.page.min(d.doc.pages.len() - 1);
            d.frame = 0;
            d.layer = docops::top_normal_layer(&d.doc, d.page);
            d.selection.clear();
            Ok(CommandOutput::doc())
        })
    }));
    r.add(cmd("page.duplicate").label("Seite duplizieren").requires(DOC).opt("index", ParamKind::Integer { min: Some(0), max: None }, "Seitenindex").run(|s, p| {
        with_doc(s, |d| {
            let i = p.opt_i64("index")?.map(|v| v.max(0) as usize).unwrap_or(d.page);
            let doc = Arc::make_mut(&mut d.doc);
            let mut pg = (**doc.pages.get(i).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?).clone();
            let mut idg = take_ids(doc);
            for slot in &mut pg.root.layers {
                if let LayerSlot::Own(l) = slot {
                    Arc::make_mut(l).reassign_ids(&mut idg);
                }
            }
            doc.ids = idg;
            pg.id = Some(fp_model::Guid::new_random());
            pg.name = Some(fp_i18n::tr!("msg-default-copy", name = pg.display_name()));
            pg.is_master = Some(false);
            doc.pages.insert(i + 1, Arc::new(pg));
            d.page = i + 1;
            d.frame = 0;
            d.layer = docops::top_normal_layer(&d.doc, d.page);
            d.selection.clear();
            Ok(CommandOutput::doc_with(json!({"page": i + 1})))
        })
    }));
    r.add(cmd("page.rename").label("Seite umbenennen").requires(DOC).opt("index", ParamKind::Integer { min: Some(0), max: None }, "Seitenindex").param("name", ParamKind::String, "Name").run(|s, p| {
        let n = p.str("name")?;
        with_doc(s, |d| {
            let i = p.opt_i64("index")?.map(|v| v.max(0) as usize).unwrap_or(d.page);
            Arc::make_mut(&mut d.doc).page_mut(i).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?.name = Some(n);
            Ok(CommandOutput::doc())
        })
    }));
    r.add(cmd("page.setCurrent").label("Seite wechseln").requires(DOC).param("index", ParamKind::Integer { min: Some(0), max: None }, "Seitenindex").run(|s, p| {
        let i = p.usize("index")?;
        with_doc(s, |d| {
            if i >= d.doc.pages.len() {
                return Err(EngineError::NotFound(fp_i18n::t("err-nf-page")));
            }
            d.page = i;
            d.frame = 0;
            d.layer = docops::layer_on_page(&d.doc, i, d.layer);
            d.selection.clear();
            Ok(CommandOutput::doc())
        })
    }));

    // Status
    r.add(
        cmd("state.add")
            .label("Status hinzufügen")
            .requires(DOC)
            .opt_default("count", ParamKind::Integer { min: Some(1), max: Some(1000) }, json!(1), "Anzahl")
            .opt("at", ParamKind::Integer { min: Some(0), max: None }, "Position (Vorgabe: hinter dem aktuellen)")
            .run(|s, p| {
                let n = p.i64_or("count", 1)?.max(1) as usize;
                let at = p.opt_i64("at")?;
                with_doc(s, |d| {
                    let at = at.map(|v| v.max(0) as usize).unwrap_or(d.frame + 1);
                    let page = d.page;
                    docops::insert_frames(Arc::make_mut(&mut d.doc), page, at, n).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
                    d.frame = at.min(d.frame_count() - 1);
                    d.selection.clear();
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(cmd("state.delete").label("Status löschen").requires(&[Requirement::Document, Requirement::MultipleFrames]).opt("index", ParamKind::Integer { min: Some(0), max: None }, "Status").run(|s, p| {
        with_doc(s, |d| {
            let i = p.opt_i64("index")?.map(|v| v.max(0) as usize).unwrap_or(d.frame);
            let page = d.page;
            docops::delete_frame(Arc::make_mut(&mut d.doc), page, i).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-state")))?;
            d.frame = d.frame.min(d.frame_count() - 1);
            d.selection.clear();
            Ok(CommandOutput::doc())
        })
    }));
    r.add(cmd("state.duplicate").label("Status duplizieren").requires(DOC).opt("index", ParamKind::Integer { min: Some(0), max: None }, "Status").run(|s, p| {
        with_doc(s, |d| {
            let i = p.opt_i64("index")?.map(|v| v.max(0) as usize).unwrap_or(d.frame);
            let page = d.page;
            let doc = Arc::make_mut(&mut d.doc);
            docops::insert_frames(doc, page, i + 1, 1).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
            let mut idg = take_ids(doc);
            if let Some(pg) = doc.page_mut(page) {
                for slot in &mut pg.root.layers {
                    if let LayerSlot::Own(l) = slot {
                        let l = Arc::make_mut(l);
                        if l.shared_across_frames {
                            continue;
                        }
                        if let Some(src) = l.cels.get(i).cloned() {
                            let mut c = (*src).clone();
                            for e in &mut c.elements {
                                Arc::make_mut(e).reassign_ids(&mut idg);
                            }
                            if let Some(dst) = l.cels.get_mut(i + 1) {
                                *dst = Arc::new(c);
                            }
                        }
                    }
                }
            }
            doc.ids = idg;
            d.frame = i + 1;
            d.selection.clear();
            Ok(CommandOutput::doc())
        })
    }));
    r.add(cmd("state.setCurrent").label("Status wechseln").requires(DOC).param("index", ParamKind::Integer { min: Some(0), max: None }, "Status").run(|s, p| {
        let i = p.usize("index")?;
        with_doc(s, |d| {
            if i >= d.frame_count() {
                return Err(EngineError::NotFound(fp_i18n::t("err-nf-state")));
            }
            d.frame = i;
            d.selection.clear();
            Ok(CommandOutput::doc())
        })
    }));

    // Farben des Werkzeugbedienfelds (Pseudo-Werkzeuge D/X, 10 §2.1)
    r.add(cmd("color.set").label("Farben setzen").opt("stroke", ParamKind::Color, "Strichfarbe").opt("fill", ParamKind::Color, "Füllfarbe").run(|s, p| {
        let (st, fi) = (p.opt_color("stroke")?, p.opt_color("fill")?);
        s.set_colors(st, fi);
        Ok(CommandOutput::session_with(json!({"stroke": argb_hex(s.tools.stroke_color), "fill": argb_hex(s.tools.fill_color)})))
    }));
    r.add(cmd("color.swap").label("Pinsel- und Füllungsfarben austauschen").describe("Pseudo-Werkzeug 35278 (Kürzel X)").run(|s, _| {
        let (st, fi) = (s.tools.stroke_color, s.tools.fill_color);
        s.set_colors(Some(fi), Some(st));
        Ok(CommandOutput::session())
    }));
    r.add(cmd("color.defaults").label("Standardfarben aktivieren").describe("Pseudo-Werkzeug 35279 (Kürzel D)").run(|s, _| {
        let (st, fi) = s.prefs.default_colors();
        s.set_colors(Some(st), Some(fi));
        Ok(CommandOutput::session())
    }));
    r.add(cmd("color.none").label("Keine Pinselstriche oder Füllungen").describe("Pseudo-Werkzeug 35082").run(|s, _| {
        s.set_colors(Some(Argb(0x00ff_ffff)), Some(Argb(0x00ff_ffff)));
        Ok(CommandOutput::session())
    }));
    r.add(
        cmd("color.sample")
            .label("Pipette")
            .requires(DOC)
            .param("point", ParamKind::Point, "Punkt")
            .opt_default("target", ParamKind::Enum { values: vec!["fill".into(), "stroke".into(), "none".into()] }, json!("fill"), "Ziel")
            .opt_default("samples", ParamKind::Integer { min: Some(1), max: Some(5) }, json!(1), "1, 3 oder 5 Pixel (Mittelwert)")
            .run(|s, p| {
                let pt = p.point("point")?;
                let n = match p.i64_or("samples", 1)? {
                    3 => 3,
                    5 => 5,
                    _ => 1,
                };
                let c = super::bitmap::sample_color(s, pt, n)?;
                match p.opt_str("target")?.as_deref() {
                    Some("stroke") => s.set_colors(Some(c), None),
                    Some("none") => {}
                    _ => s.set_colors(None, Some(c)),
                }
                Ok(CommandOutput::session_with(json!({"color": argb_hex(c)})))
            }),
    );

    // Werkzeuge und Verlauf
    r.add(cmd("tool.select").label("Werkzeug wählen").param("tool", ParamKind::String, "Werkzeugname (tool.pen, Stift, pen) oder GUID").run(|s, p| {
        let n = p.str("tool")?;
        let t = tools::by_name(&n).or_else(|| tools::by_guid(&n)).ok_or_else(|| EngineError::NotFound(fp_i18n::tr!("err-nf-tool", name = n.as_str())))?;
        s.set_tool(t)?;
        Ok(CommandOutput::session_with(json!({"tool": tools::info(t).map(|i| i.name)})))
    }));
    r.add(
        cmd("history.replay")
            .label("Schritte wiederholen")
            .requires(DOC)
            .param("commands", ParamKind::Json, "[{command, params}, …]")
            .opt("label", ParamKind::String, "Verlaufsbeschriftung")
            .run(|s, p| {
                let cmds: Vec<RecordedCommand> = p.parse("commands")?.unwrap_or_default();
                let label = p.opt_str("label")?.unwrap_or_else(|| "Schritte wiederholen".into());
                s.replay(&cmds, &label)?;
                Ok(CommandOutput::session())
            }),
    );
    r.add(cmd("history.goTo").label("Verlauf: zu Schritt").requires(DOC).param("position", ParamKind::Integer { min: Some(0), max: None }, "Anzahl ausgeführter Schritte").run(|s, p| {
        s.history_go_to(p.usize("position")?)?;
        Ok(CommandOutput::session())
    }));
    r.add(cmd("history.export").label("Schritte als Befehle").requires(DOC).opt("from", ParamKind::Integer { min: Some(0), max: None }, "erster Schritt").opt("to", ParamKind::Integer { min: Some(0), max: None }, "Ende (exklusiv)").run(|s, p| {
        let d = s.doc()?;
        let n = d.history.undo_count();
        let from = p.i64_or("from", 0)?.max(0) as usize;
        let to = p.opt_i64("to")?.map(|v| v.max(0) as usize).unwrap_or(n);
        let cmds = d.history.export(from..to);
        Ok(CommandOutput::value(json!({"commands": cmds, "script": crate::history::to_script(&cmds), "labels": d.history.labels()})))
    }));
}

fn common_edit(s: &mut Session, p: &Params, f: impl Fn(&mut Element)) -> Result<CommandOutput> {
    with_doc(s, |d| {
        let (page, ids) = (d.page, target_ids(d, p)?);
        let doc = Arc::make_mut(&mut d.doc);
        for id in ids {
            docops::edit_by_id(doc, page, id, &f).ok_or(EngineError::ElementNotFound(id.0))?;
        }
        Ok(CommandOutput::doc())
    })
}

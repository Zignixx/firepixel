//! Befehle der Zeichenwerkzeuge und Bedienfelder: Formen, Pfade, Text, Hotspots/Segmente,
//! Objektattribute (Eigenschaften-Inspektor), Ebenen, Seiten, Status, Farben, Werkzeugwahl, Verlauf.

use super::{
    DOC, argb_hex, insert_new, sel_err, shape_attrs, take_ids, target_ids, with_doc, with_targets,
};
use crate::command::{CommandOutput, CommandRegistry, ParamKind, Params, Requirement, cmd};
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
use fp_model::{
    Brush, Contour, Fill, Gradient, GradientStop, GradientStops, LayerId, StopList, WebShape,
};
use serde_json::{Value, json};
use std::sync::Arc;

/// Swap colours (toolbox button, X, `swapBrushAndFillColors`; TW-186, re 58 §1, measured).
///
/// Without selected objects only the toolbox colours swap, without a History row. With a
/// selection the toolbox colours are those of the topmost selected path or text (the toolbox
/// follows the selection in the reference application; a bitmap-only selection keeps the toolbox
/// colours); they swap — "none" included — and are applied to every selected element: fill =
/// new toolbox fill, stroke = new toolbox stroke. An existing stroke keeps its tip, width and
/// placement; a new stroke gets the 1-pixel anti-aliased tip, 1 px. Bitmaps keep their pixels but
/// the row is still written. One row "Stroke Color", or "Brush and Fill" when one side was none.
fn swap_colors(s: &mut Session, _p: &Params) -> Result<CommandOutput> {
    let has_selection = s
        .active()
        .is_some_and(|d| !super::selected_paths(d).is_empty());
    if !has_selection {
        let (st, fi) = (s.tools.stroke_color, s.tools.fill_color);
        s.set_colors(Some(fi), Some(st));
        return Ok(CommandOutput::session());
    }
    let none = |c: Argb| Argb(c.0 & 0x00ff_ffff);
    let (st, fi) = s
        .doc()
        .ok()
        .and_then(|d| {
            super::selected_elements(d)
                .iter()
                .find_map(|(_, e)| docops::path_attrs_of(e))
        })
        .map(|a| {
            let st = if a.brush.is_some() {
                a.stroke_color
            } else {
                none(a.stroke_color)
            };
            let fi = if a.fill.is_some() {
                a.fill_color
            } else {
                none(a.fill_color)
            };
            (st, fi)
        })
        .unwrap_or((s.tools.stroke_color, s.tools.fill_color));
    let with_none = st.a() == 0 || fi.a() == 0;
    // swapped: the old fill becomes the stroke and the old stroke the fill
    let (new_stroke, new_fill) = (fi, st);
    s.set_colors(Some(new_stroke), Some(new_fill));
    let new_tip = super::panel::preset_brush("bn_1-Pixel Anti-Aliased", 1);
    with_doc(s, |d| {
        let page = d.page;
        let ids: Vec<_> = super::selected_elements(d)
            .iter()
            .filter(|(_, e)| docops::path_attrs_of(e).is_some())
            .map(|(_, e)| e.id)
            .collect();
        if ids.is_empty() {
            return Ok(());
        }
        let doc = Arc::make_mut(&mut d.doc);
        for id in ids {
            docops::edit_by_id(doc, page, id, |e| {
                docops::edit_path_attrs(e, &|a| {
                    if new_fill.a() == 0 {
                        a.fill = None;
                    } else {
                        a.fill_color = new_fill;
                        if a.fill.is_none() {
                            a.fill = Some(Fill::solid());
                        }
                    }
                    if new_stroke.a() == 0 {
                        a.brush = None;
                    } else {
                        a.stroke_color = new_stroke;
                        if a.brush.is_none() {
                            a.brush = Some(new_tip.clone());
                        }
                    }
                });
            });
        }
        Ok(())
    })?;
    // a bitmap gets the row without a pixel change (measured)
    s.force_history_step();
    let key = if with_none {
        "msg-step-brush-and-fill"
    } else {
        "msg-step-stroke-color"
    };
    Ok(CommandOutput::doc().labeled(fp_i18n::t(key)))
}

/// Liest Konturen `[{closed, nodes: [{anchor, in, out}]}]` (Henkel optional).
pub(crate) fn parse_contours(p: &Params) -> Result<Vec<Contour>> {
    let err = |key: &str| EngineError::InvalidParams {
        name: p.command.clone(),
        message: fp_i18n::t(key),
    };
    let arr = p
        .get("contours")
        .and_then(|v| v.as_array())
        .ok_or_else(|| err("err-param-contours"))?;
    let mut out = Vec::new();
    for c in arr {
        let closed = c.get("closed").and_then(|v| v.as_bool()).unwrap_or(false);
        let nodes = c
            .get("nodes")
            .and_then(|v| v.as_array())
            .ok_or_else(|| err("err-param-nodes-missing"))?;
        let mut ns = Vec::new();
        for n in nodes {
            let (a, i, o) = if let Some(a) = n.get("anchor") {
                let a = crate::command::parse_point(a).ok_or_else(|| err("anchor"))?;
                let i = n
                    .get("in")
                    .and_then(crate::command::parse_point)
                    .unwrap_or(a);
                let o = n
                    .get("out")
                    .and_then(crate::command::parse_point)
                    .unwrap_or(a);
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

fn attrs_from(
    s: &Session,
    p: &Params,
    default_fill: bool,
    default_stroke: f32,
) -> Result<fp_model::PathAttrs> {
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
    b.opt(
        "fill",
        ParamKind::Bool,
        "Füllung (Vorgabe: ja, Füllfarbe des Werkzeugbedienfelds)",
    )
    .opt("fillColor", ParamKind::Color, "Füllfarbe")
    .opt(
        "strokeWidth",
        ParamKind::Number {
            min: Some(0.0),
            max: Some(1000.0),
        },
        "Strichbreite (0 = kein Strich)",
    )
    .opt("strokeColor", ParamKind::Color, "Strichfarbe")
    .opt("name", ParamKind::String, "Objektname")
}

fn layer_param(d: &DocumentState, p: &Params) -> Result<LayerId> {
    match p.opt_i64("layer")? {
        Some(v) => Ok(LayerId(v as u64)),
        None => d
            .layer
            .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-active-layer"))),
    }
}

/// RE spec 65 rule 1: the selected objects (a subselected object alone, out of its group; a
/// selected group whole) go as one block, in their stacking order, to the top of the layer or
/// sublayer `target` (above its own objects and sublayers); with `copy` copies go there and the
/// originals stay. The moved objects or the copies stay selected and `target` becomes the chosen
/// and current layer. Nothing happens (no History row) with nothing selected, when `target`
/// already holds a selected object (its circle is filled), is locked (itself or through a parent)
/// or is the web layer, when the selection holds a slice or hotspot, or a selected sublayer
/// contains `target`.
fn move_selection_to_layer(
    d: &mut DocumentState,
    target: LayerId,
    copy: bool,
) -> Result<CommandOutput> {
    let (page, frame) = (d.page, d.frame);
    let flat = docops::flat_layers(&d.doc, page, frame);
    let t = flat
        .iter()
        .find(|l| l.id == target)
        .copied()
        .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-layer")))?;
    let mut paths = super::selected_paths(d);
    // an object inside a selected group goes with the group
    let all = paths.clone();
    paths.retain(|p| {
        !all.iter().any(|q| {
            q.root == p.root
                && q.slot == p.slot
                && q.cel == p.cel
                && q.steps.len() < p.steps.len()
                && p.steps.starts_with(&q.steps)
        })
    });
    let unchanged = Ok(CommandOutput::doc());
    if paths.is_empty() || t.locked || t.web {
        return unchanged;
    }
    if paths
        .iter()
        .any(|p| docops::owning_layer(&d.doc, page, p) == Some(target))
    {
        return unchanged;
    }
    let els: Vec<Arc<Element>> = paths
        .iter()
        .filter_map(|p| d.doc.element_at(p).cloned())
        .collect();
    if els.iter().any(|e| match &e.kind {
        ElementKind::WebObject(_) => true,
        ElementKind::SubLayer(sl) => docops::layer_tree_contains(sl, target),
        _ => false,
    }) {
        return unchanged;
    }
    let doc = Arc::make_mut(&mut d.doc);
    let moved: Vec<Arc<Element>> = if copy {
        els.iter()
            .map(|e| Arc::new(docops::reassign(doc, e)))
            .collect()
    } else {
        // bottom first, so the addresses of the objects above stay valid
        let mut out = Vec::new();
        for p in paths.iter().rev() {
            let Some(e) = docops::remove_at(doc, p) else {
                continue;
            };
            out.push(e);
            // a group left without children goes as well
            if p.steps.len() > 1 {
                let parent = fp_model::traverse::ElementPath {
                    steps: p.steps[..p.steps.len() - 1].to_vec(),
                    ..p.clone()
                };
                if doc.element_at(&parent).is_some_and(|g| {
                    matches!(&g.kind, ElementKind::Group(g) if g.children.is_empty() && g.mask.is_none())
                }) {
                    docops::remove_at(doc, &parent);
                }
            }
        }
        out.reverse();
        out
    };
    let ids: Vec<fp_model::ElementId> = moved.iter().map(|e| e.id).collect();
    for e in moved.into_iter().rev() {
        docops::insert_element(doc, page, frame, Some(target), 0, e)
            .ok_or_else(|| EngineError::Failed(fp_i18n::t("err-object-move-failed")))?;
    }
    d.selection.set(ids);
    d.choose_layer(Some(target));
    Ok(CommandOutput::doc())
}

/// Eigene Ebene `id` als oberste Unterebene in die Ebene `target` (Ziehen auf die Mitte der
/// Zielzeile, re 40 §2a); die verschobene Ebene wird aktiv.
fn move_layer_into(
    d: &mut crate::session::DocumentState,
    id: fp_model::LayerId,
    target: fp_model::LayerId,
) -> Result<()> {
    let nf = || EngineError::NotFound(fp_i18n::t("err-nf-layer"));
    if id == target {
        return Err(nf());
    }
    let (page, frame) = (d.page, d.frame);
    let slot = docops::layer_slot(&d.doc, page, id).ok_or_else(nf)?;
    docops::layer_by_id(&d.doc, page, target).ok_or_else(nf)?;
    let doc = Arc::make_mut(&mut d.doc);
    let pg = doc
        .page_mut(page)
        .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
    let LayerSlot::Own(layer) = pg.root.layers.remove(slot) else {
        return Err(nf());
    };
    let mut idg = std::mem::take(&mut doc.ids);
    let e = Element::new(idg.element(), ElementKind::SubLayer(layer));
    doc.ids = idg;
    docops::insert_element(doc, page, frame, Some(target), 0, Arc::new(e)).ok_or_else(nf)?;
    d.layer = Some(id);
    Ok(())
}

/// The Layers panel's New Layer button with a current sublayer (RE spec 64 §1.3): a new sublayer
/// directly above the current one in the same parent, which becomes current. `None` when the
/// current layer is not a sublayer.
fn add_sibling_sublayer(
    d: &mut crate::session::DocumentState,
    name: Option<&str>,
) -> Result<Option<fp_model::LayerId>> {
    let page = d.page;
    let Some(el) = d.layer.and_then(|l| docops::as_sublayer(&d.doc, page, l)) else {
        return Ok(None);
    };
    let path = docops::find_on_page(&d.doc, page, el)
        .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-layer")))?;
    let count = docops::flat_layers(&d.doc, page, d.frame).len();
    let doc = Arc::make_mut(&mut d.doc);
    let frames = doc
        .pages
        .get(page)
        .map(|pg| pg.root.frame_count as usize)
        .unwrap_or(1)
        .max(1);
    let label = name
        .map(str::to_string)
        .unwrap_or_else(|| fp_i18n::tr!("msg-default-layer", n = count.to_string()));
    let mut idg = std::mem::take(&mut doc.ids);
    let sub = fp_model::Layer::new(&mut idg, &label, LayerKind::Normal, frames);
    let id = sub.id;
    let e = Element::new(idg.element(), ElementKind::SubLayer(Arc::new(sub)));
    doc.ids = idg;
    let (list, at) = docops::parent_list_mut(doc, &path)
        .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-layer")))?;
    list.insert(at, Arc::new(e));
    d.layer = Some(id);
    Ok(Some(id))
}

fn edit_layer(
    s: &mut Session,
    p: &Params,
    f: impl FnOnce(&mut fp_model::Layer),
) -> Result<CommandOutput> {
    with_doc(s, |d| {
        let id = layer_param(d, p)?;
        let page = d.page;
        // auch Unterebenen (Skript-Ebenennummern zählen sie flach mit, Befund F6-1)
        docops::edit_layer_by_id(Arc::make_mut(&mut d.doc), page, id, f)
            .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-layer")))?;
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

fn edit_effects(
    s: &mut Session,
    p: &Params,
    f: impl Fn(&mut EffectList) -> Result<()>,
) -> Result<CommandOutput> {
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
        // Firepixel extension (FB-38)
        "screentone" => EffectKind::Screentone,
        _ => return None,
    };
    k.moa_id().map(|s| s.to_string())
}

/// Vorgaben, die das Effekt-Menü des Originals einfügt (`EFDv`, re\23 Gruppe 1), für Effekte, die per
/// Name hinzugefügt werden; übergebene Parameter überschreiben sie (FX-1). Ohne diese Liste wurde
/// `glow` ohne `BevelType` zur Abschrägung innen, und fehlende Schlüssel (`EdgeThreshold`,
/// `BevelContrast`) ergaben ein anderes Glühen als im Original.
fn menu_preset(name: &str) -> &'static [(&'static str, &'static str)] {
    match name.to_ascii_lowercase().as_str() {
        "dropshadow" => &[
            ("ShadowColor", "#000000a2"),
            ("ShadowType", "0"),
            ("ShadowBlur", "4"),
            ("ShadowDistance", "7"),
            ("ShadowAngle", "315"),
        ],
        "innershadow" => &[
            ("ShadowColor", "#000000a5"),
            ("ShadowType", "0"),
            ("ShadowBlur", "4"),
            ("ShadowDistance", "7"),
            ("ShadowAngle", "315"),
        ],
        "innerglow" => &[
            ("BevelContrast", "65"),
            ("MaskSoftness", "4"),
            ("OuterBevelColor", "#000000"),
            ("GlowStartDistance", "0"),
            ("GlowWidth", "4"),
        ],
        "glow" => &[
            ("BevelType", "4"),
            ("BevelWidth", "10"),
            ("BevelContrast", "65"),
            ("EdgeThreshold", "1"),
            ("AngleSoftness", "25"),
            ("MaskSoftness", "12"),
            ("LightDistance", "100"),
            ("LightAngle", "135"),
            ("SlopeType", "0"),
            ("SlopeMultiplier", "1"),
            ("OuterBevelColor", "#df0000"),
            ("DownBlendColor", "#0000003f"),
            ("HitBlendColor", "#ffffff3f"),
            ("HiliteColor", "#ffffff"),
            ("ShadowColor", "#000000"),
            ("EmbossFaceColor", "#00000000"),
            ("ShowObject", "false"),
            ("ButtonState", "0"),
            ("GlowStartDistance", "0"),
            ("GlowWidth", "5"),
        ],
        // Screentone (Firepixel extension, FB-38): dots, 8 px, the classic 45° screen, tone from
        // the luminance, black ink
        "screentone" => &[
            ("screentone_type", "0"),
            ("screentone_size", "8"),
            ("screentone_angle", "45"),
            ("screentone_density", "50"),
            ("screentone_from_luminance", "true"),
            ("screentone_color", "#000000"),
            ("screentone_keep_color", "false"),
        ],
        _ => &[],
    }
}

fn web_object(s: &mut Session, p: &Params, slice: bool) -> Result<CommandOutput> {
    let shape = p.str("shape")?;
    let (bounds, polygon) = match shape.as_str() {
        "polygon" => {
            let pts = p.points("points")?;
            if pts.len() < 3 {
                return Err(EngineError::InvalidParams {
                    name: p.command.clone(),
                    message: fp_i18n::t("err-param-three-points"),
                });
            }
            let c = geometry::polygon_contour(&pts, true);
            (
                geometry::contours_bounds(std::slice::from_ref(&c)).unwrap_or_default(),
                Some(c),
            )
        }
        _ => (p.rect("rect")?.normalized(), None),
    };
    let url = p.opt_str("url")?;
    with_doc(s, |d| {
        let doc = Arc::make_mut(&mut d.doc);
        let mut idg = take_ids(doc);
        let mut e = if slice {
            Element::new_slice(&mut idg, bounds)
        } else {
            Element::new_hotspot(&mut idg, bounds)
        };
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
        .map(|i| GradientStop {
            position: i as f32 / (n - 1) as f32,
            color: colors.get(i).copied().unwrap_or(Argb(0xff00_0000)),
            extra: Default::default(),
        })
        .collect();
    let opac: Vec<GradientStop> = stops
        .iter()
        .map(|s| GradientStop {
            position: s.position,
            color: Argb(((s.color.a() as u32) << 24) | 0x00ff_ffff),
            extra: Default::default(),
        })
        .collect();
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
        stops: Some(GradientStops {
            colors: StopList {
                stops,
                extra: Default::default(),
            },
            opacities: StopList {
                stops: opac,
                extra: Default::default(),
            },
            dither: None,
            dither_value: None,
            extra: Default::default(),
        }),
        extra: Default::default(),
    });
    f
}

/// Farbparameter-Liste `colors`.
pub(crate) fn colors_param(p: &Params, fallback: &[Argb]) -> Result<Vec<Argb>> {
    match p.get("colors").and_then(|v| v.as_array()) {
        Some(a) => a
            .iter()
            .map(|v| {
                v.as_str()
                    .and_then(fp_color::Rgba8::parse_hex)
                    .map(|c| c.to_argb())
                    .ok_or_else(|| EngineError::InvalidParams {
                        name: p.command.clone(),
                        message: fp_i18n::t("err-param-colors"),
                    })
            })
            .collect(),
        None => Ok(fallback.to_vec()),
    }
}

pub(super) fn register(r: &mut CommandRegistry) {
    // Formen
    r.add(
        shape_opts(cmd("draw.rectangle").label("Rechteck").requires(DOC))
            .param("rect", ParamKind::Rect, "Rechteck")
            .opt_default(
                "roundness",
                ParamKind::Number {
                    min: Some(0.0),
                    max: Some(1.0),
                },
                json!(0),
                "Eckenrundung 0..1",
            )
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
    r.add(
        shape_opts(cmd("draw.ellipse").label("Ellipse").requires(DOC))
            .param("rect", ParamKind::Rect, "umschließendes Rechteck")
            .run(|s, p| {
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
            }),
    );
    r.add(
        shape_opts(cmd("draw.polygon").label("Polygon").requires(DOC))
            .param("center", ParamKind::Point, "Mittelpunkt")
            .param("corner", ParamKind::Point, "erste Ecke")
            .opt_default(
                "sides",
                ParamKind::Integer {
                    min: Some(3),
                    max: Some(360),
                },
                json!(5),
                "Seiten bzw. Zacken",
            )
            .opt_default("star", ParamKind::Bool, json!(false), "Stern")
            .opt(
                "spikiness",
                ParamKind::Number {
                    min: Some(0.0),
                    max: Some(1.0),
                },
                "Spitzigkeit 0..1 (fehlt = automatisch)",
            )
            .run(|s, p| {
                let c = geometry::polygon_tool_contour(
                    p.point("center")?,
                    p.point("corner")?,
                    p.i64_or("sides", 5)?.clamp(3, 360) as usize,
                    p.bool_or("star", false)?,
                    p.opt_f64("spikiness")?,
                )
                .ok_or_else(|| EngineError::InvalidParams {
                    name: p.command.clone(),
                    message: fp_i18n::t("err-param-radius-zero"),
                })?;
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
    r.add(
        shape_opts(cmd("draw.line").label("Linie").requires(DOC))
            .param("from", ParamKind::Point, "Anfang")
            .param("to", ParamKind::Point, "Ende")
            .run(|s, p| {
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
            }),
    );
    r.add(
        shape_opts(cmd("draw.path").label("Pfad").requires(DOC))
            .param(
                "contours",
                ParamKind::Json,
                "[{closed, nodes: [{anchor: [x, y], in: [x, y], out: [x, y]}]}]",
            )
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
            .opt(
                "anchor",
                ParamKind::Point,
                "Punkttext: Anker (x, erste Grundlinie)",
            )
            .opt("rect", ParamKind::Rect, "Rahmentext fester Breite")
            .opt("font", ParamKind::String, "Schrift")
            .opt(
                "size",
                ParamKind::Number {
                    min: Some(1.0),
                    max: Some(5000.0),
                },
                "Schriftgröße in px",
            )
            .opt(
                "color",
                ParamKind::Color,
                "Textfarbe (Vorgabe: Textfarbe des Textwerkzeugs)",
            )
            .opt(
                "runs",
                ParamKind::Json,
                "formatierte Läufe [{attrs, text}] (Text-Editor)",
            )
            .opt(
                "antialias",
                ParamKind::Enum {
                    values: super::text::antialias_names(),
                },
                "Glättung (Vorgabe: Textwerkzeug, Text_AntiAliasLevel)",
            )
            .opt(
                "autoKern",
                ParamKind::Bool,
                "automatische Unterschneidung (Vorgabe: Textwerkzeug, Text_AutoKern)",
            )
            .opt(
                "orientation",
                ParamKind::Enum {
                    values: super::text::orientation_names(),
                },
                "Textrichtung (Vorgabe: Textwerkzeug, Text_Orientation)",
            )
            .opt(
                "initFromPrefs",
                ParamKind::Bool,
                "Text_*-Vorgaben übernehmen (Vorgabe: ja)",
            )
            .run(|s, p| {
                let text = super::text::normalize_breaks(&p.str("text")?);
                // Glättung, Unterschneidung und Richtung sind Objektattribute; neue Texte bekommen die
                // Vorgaben des Textwerkzeugs (Original `a_133`, O6), Parameter überschreiben sie.
                let (d_aa, d_kern, d_ori) = super::text::tool_text_defaults(&s.prefs);
                let aa = p
                    .opt_str("antialias")?
                    .map(|n| super::text::antialias_code(&n))
                    .unwrap_or(d_aa);
                let auto_kern = p.opt_bool("autoKern")?.unwrap_or(d_kern);
                let ori = p
                    .opt_str("orientation")?
                    .map(|n| super::text::orientation_code(&n))
                    .unwrap_or(d_ori);
                let runs = super::text::parse_runs(p, &text)?;
                // `initFromPrefs: false` (script `addNewText(rect, false)`): factory font and size
                // (MyriadPro-Regular 12 pt, re 63 section 2.2), alignment, style and leading
                // instead of the text tool's defaults (re 50 section 2.3)
                let from_prefs = p.opt_bool("initFromPrefs")?.unwrap_or(true);
                let (tool_font, tool_size) = if from_prefs {
                    (
                        s.tools.options.text_font.clone(),
                        s.tools.options.text_size as f64,
                    )
                } else {
                    let f = crate::tools::factory_text_font();
                    (f.unwrap_or_else(|| "Arial".into()), 12.0)
                };
                let font = p.opt_str("font")?.unwrap_or(tool_font);
                let size = p.f64_or("size", tool_size)? as f32;
                let color = p.opt_color("color")?.unwrap_or(s.tools.options.text_color);
                let rect = p.opt_rect("rect")?;
                let (lead, pref_attrs) = if from_prefs {
                    (
                        s.prefs.get_real("Text_Leading", 1.2),
                        super::text::new_text_pref_attrs(&s.prefs),
                    )
                } else {
                    (1.2, fp_model::text::TextAttrs::default())
                };
                let anchor = p
                    .opt_point("anchor")?
                    .or_else(|| rect.map(|r| Point::new(r.left, r.top + size)))
                    .ok_or_else(|| EngineError::InvalidParams {
                        name: p.command.clone(),
                        message: fp_i18n::t("err-param-anchor-or-rect"),
                    })?;
                let e = {
                    let d = s.doc_mut()?;
                    let doc = Arc::make_mut(&mut d.doc);
                    let mut idg = take_ids(doc);
                    let mut e = Element::new_text(&mut idg, &text, &font, size, color, anchor);
                    doc.ids = idg;
                    if let ElementKind::Text(t) = &mut e.kind {
                        super::text::apply_antialias(t, aa);
                        t.auto_kern = auto_kern;
                        t.orientation = ori;
                    }
                    if let (Some(r), ElementKind::Text(t)) = (rect, &mut e.kind) {
                        t.frame_type = Some(1);
                        t.bounds =
                            Rect::new(r.left, r.top, r.right, r.bottom.max(r.top + size * 1.2));
                        t.anchor_rect = Some(t.bounds);
                    }
                    match (runs, &mut e.kind) {
                        (Some(runs), ElementKind::Text(t)) => {
                            crate::tools::text::apply_runs(t, runs)
                        }
                        (None, ElementKind::Text(t)) => {
                            // Zeilenabstand der Werksvorgabe (`Text_Leading` 120 %, UX-A O4)
                            let l = Some(lead as f32);
                            t.initial.leading = l;
                            for r in &mut t.runs {
                                r.attrs.leading = l;
                            }
                            // the other `Text_*` defaults (not the colour, re 50 section 2.3)
                            t.initial.apply(&pref_attrs);
                            for r in &mut t.runs {
                                r.attrs.apply(&pref_attrs);
                            }
                        }
                        _ => {}
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
            .param(
                "shape",
                ParamKind::Enum {
                    values: vec!["rect".into(), "circle".into(), "polygon".into()],
                },
                "Form",
            )
            .opt("rect", ParamKind::Rect, "Rechteck (rect/circle)")
            .opt("points", ParamKind::Json, "Punkte (polygon)")
            .opt("url", ParamKind::String, "Link")
            .run(|s, p| web_object(s, p, false)),
    );
    r.add(
        cmd("web.addSlice")
            .label("Segment")
            .requires(DOC)
            .param(
                "shape",
                ParamKind::Enum {
                    values: vec!["rect".into(), "polygon".into()],
                },
                "Form",
            )
            .opt("rect", ParamKind::Rect, "Rechteck")
            .opt("points", ParamKind::Json, "Punkte (polygon)")
            .opt("url", ParamKind::String, "Link")
            .run(|s, p| web_object(s, p, true)),
    );

    // Objektattribute (Eigenschaften-Inspektor)
    let ids_opt = |b: crate::command::CommandBuilder| {
        b.opt("ids", ParamKind::Ids, "Elemente (Vorgabe: Auswahl)")
    };
    r.add(
        ids_opt(cmd("element.setFill").label("Füllung ändern").requires(DOC))
            .opt("color", ParamKind::Color, "Füllfarbe")
            .opt_default(
                "kind",
                ParamKind::Enum {
                    values: vec!["solid".into(), "none".into()],
                },
                json!("solid"),
                "Füllart",
            )
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
        ids_opt(
            cmd("element.setStroke")
                .label("Strich ändern")
                .requires(DOC),
        )
        .opt("color", ParamKind::Color, "Strichfarbe")
        .opt(
            "width",
            ParamKind::Number {
                min: Some(0.0),
                max: Some(1000.0),
            },
            "Strichbreite (0 = kein Strich)",
        )
        .opt(
            "placement",
            ParamKind::Enum {
                values: vec!["inside".into(), "center".into(), "outside".into()],
            },
            "Strichposition",
        )
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
    r.add(
        ids_opt(cmd("element.setOpacity").label("Deckkraft").requires(DOC))
            .param(
                "opacity",
                ParamKind::Number {
                    min: Some(0.0),
                    max: Some(100.0),
                },
                "Deckkraft in %",
            )
            .run(|s, p| {
                let o = Opacity::from_percent(p.f64("opacity")?);
                common_edit(s, p, move |e| content_of(e).common.opacity = o)
            }),
    );
    r.add(
        ids_opt(
            cmd("element.setBlendMode")
                .label("Mischmodus")
                .requires(DOC),
        )
        .param(
            "mode",
            ParamKind::String,
            "Name (normal, multiply, …) oder BLDi-Nummer",
        )
        .run(|s, p| {
            let m = p.str("mode")?;
            let mode = BlendMode::from_name(&m)
                .or_else(|| m.parse::<u8>().ok().map(BlendMode))
                .ok_or_else(|| EngineError::InvalidParams {
                    name: p.command.clone(),
                    message: fp_i18n::tr!("err-unknown-blend", name = m.as_str()),
                })?;
            common_edit(s, p, move |e| content_of(e).common.blend = mode)
        }),
    );
    r.add(
        ids_opt(cmd("element.setName").label("Objektname").requires(DOC))
            .param("name", ParamKind::String, "Name")
            .run(|s, p| {
                let n = p.str("name")?;
                common_edit(s, p, move |e| {
                    e.common.name = if n.is_empty() { None } else { Some(n.clone()) };
                    // Unterebene: der angezeigte Name ist der Ebenenname (fixg5 Q-05, Umbenennen im Ebenen-Bedienfeld)
                    if let fp_model::ElementKind::SubLayer(sl) = &mut e.kind
                        && !n.is_empty()
                    {
                        Arc::make_mut(sl).name = n.clone();
                    }
                })
            }),
    );
    r.add(
        ids_opt(
            cmd("element.setVisible")
                .label("Sichtbarkeit")
                .requires(DOC),
        )
        .param("visible", ParamKind::Bool, "sichtbar")
        .run(|s, p| {
            let v = p.bool_or("visible", true)?;
            common_edit(s, p, move |e| e.common.visible = v)
        }),
    );
    r.add(
        ids_opt(cmd("element.setLocked").label("Sperren").requires(DOC))
            .param("locked", ParamKind::Bool, "gesperrt")
            .run(|s, p| {
                let v = p.bool_or("locked", true)?;
                let locks = |s: &Session| {
                    s.doc()
                        .map(|d| docops::flat_layers(&d.doc, d.page, d.frame))
                        .unwrap_or_default()
                };
                let before = locks(s);
                let out = common_edit(s, p, move |e| e.common.locked = v)?;
                // a sublayer locked or unlocked in the Layers panel: RE spec 65 rule 3 as for
                // `layer.setLocked`
                if locks(s) != before {
                    with_doc(s, |d| {
                        d.resolve_current_layer();
                        Ok(())
                    })?;
                }
                Ok(out)
            }),
    );
    r.add(
        cmd("element.setBounds")
            .label("Position und Größe")
            .requires(DOC)
            .opt(
                "ids",
                ParamKind::Ids,
                "Elemente (Vorgabe: Auswahl; die Auswahl bleibt unverändert)",
            )
            .opt(
                "x",
                ParamKind::Number {
                    min: None,
                    max: None,
                },
                "X",
            )
            .opt(
                "y",
                ParamKind::Number {
                    min: None,
                    max: None,
                },
                "Y",
            )
            .opt(
                "width",
                ParamKind::Number {
                    min: Some(0.0),
                    max: None,
                },
                "B",
            )
            .opt(
                "height",
                ParamKind::Number {
                    min: Some(0.0),
                    max: None,
                },
                "H",
            )
            .run(|s, p| {
                with_targets(s, p, |s, p| {
                    let d = s.doc()?;
                    // Bezug wie im Inspektor: maskiertes Einzelbild mit seinen Bildgrenzen (re 29b §2)
                    let b = docops::ids_info_bounds(&d.doc, d.page, &d.selection.elements)
                        .ok_or_else(|| sel_err("err-sel-no-bounds-short"))?;
                    let x = p.f64_or("x", b.left as f64)? as f32;
                    let y = p.f64_or("y", b.top as f64)? as f32;
                    let w = p.f64_or("width", b.width() as f64)? as f32;
                    let h = p.f64_or("height", b.height() as f64)? as f32;
                    let m = docops::rect_to_rect(b, Rect::new(x, y, x + w, y + h));
                    let scale_attrs = s.prefs.get_bool("Firepixel_InfoScaleAttributes", true);
                    // "Scale attributes" off: a resize leaves the fill handles in the document; a
                    // pure move always takes them along (RE spec 61 §1.3/§1.5)
                    let before = (!scale_attrs && !m.is_translation(1e-6))
                        .then(|| super::modify::selection_snapshot(s))
                        .flatten();
                    let out = super::modify::transform_selection(s, m)?;
                    if let Some(before) = before {
                        super::modify::restore_selected_fill_handles(s, &before)?;
                    }
                    // Strich mitskalieren wie im Original („Attribute skalieren“, Vorgabe ein; Faktor
                    // √(sx·sy), z. B. 8 px bei doppelter Breite → 11 px, PAR 17/16)
                    let (sx, sy) = (
                        if b.width() > 0.0 { w / b.width() } else { 1.0 },
                        if b.height() > 0.0 {
                            h / b.height()
                        } else {
                            1.0
                        },
                    );
                    let f = (sx * sy).abs().sqrt();
                    if (f - 1.0).abs() > 1e-6 && scale_attrs {
                        let d = s.doc_mut()?;
                        let ids = d.selection.elements.clone();
                        let page = d.page;
                        let doc = Arc::make_mut(&mut d.doc);
                        for id in ids {
                            docops::edit_by_id(doc, page, id, |e| scale_stroke_width(e, f));
                        }
                    }
                    Ok(out)
                })
            }),
    );
    r.add(
        ids_opt(
            cmd("element.addEffect")
                .label("Effekt hinzufügen")
                .requires(DOC),
        )
        .param(
            "effect",
            ParamKind::String,
            "Effekt (dropShadow, innerGlow, bevel, gaussianBlur, … oder MOA-ID)",
        )
        .opt(
            "params",
            ParamKind::Json,
            "Parameter {Schlüssel: Wert} wie im Dateiformat",
        )
        .opt_default("visible", ParamKind::Bool, json!(true), "sichtbar")
        .run(|s, p| {
            let name = p.str("effect")?;
            let moa = effect_moa_id(&name).ok_or_else(|| EngineError::InvalidParams {
                name: p.command.clone(),
                message: fp_i18n::tr!("err-unknown-effect", name = name.as_str()),
            })?;
            let vis = p.bool_or("visible", true)?;
            // Menü-Vorgaben (FX-1), dann die Parameter: Text wie im Dateiformat, Punktlisten (Kurven:
            // `[{x, y}, …]` bzw. `[[x, y], …]`) als `GPLv` (FX-4)
            let mut params: Vec<(String, fp_model::DataValue)> = menu_preset(&name)
                .iter()
                .map(|(k, v)| (k.to_string(), fp_model::DataValue::Scalar(v.to_string())))
                .collect();
            if let Some(Value::Object(m)) = p.get("params") {
                params.extend(
                    m.iter()
                        .map(|(k, v)| (k.clone(), super::panel::effect_param_value(v))),
                );
            }
            edit_effects(s, p, move |l| {
                let mut e = Effect::new(&moa, vis);
                for (k, v) in &params {
                    match v {
                        fp_model::DataValue::Scalar(t) => e.set(k, t.clone()),
                        other => e.params.set_structured(k, other.clone()),
                    }
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
    r.add(
        ids_opt(
            cmd("element.removeEffect")
                .label("Effekt entfernen")
                .requires(DOC),
        )
        .param(
            "index",
            ParamKind::Integer {
                min: Some(0),
                max: None,
            },
            "Index in der Effektliste",
        )
        .run(|s, p| {
            let i = p.usize("index")?;
            edit_effects(s, p, move |l| {
                if i < l.effects.len() {
                    l.effects.remove(i);
                    Ok(())
                } else {
                    Err(EngineError::NotFound(fp_i18n::t("err-nf-effect")))
                }
            })
        }),
    );
    r.add(
        ids_opt(
            cmd("element.setEffectVisible")
                .label("Effekt ein-/ausblenden")
                .requires(DOC),
        )
        .param(
            "index",
            ParamKind::Integer {
                min: Some(0),
                max: None,
            },
            "Index",
        )
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
            .opt(
                "colors",
                ParamKind::Json,
                "Farben [#rrggbb, …] (Vorgabe: Füll- → Strichfarbe)",
            )
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
                            // dritter Griff (Breite) wie im Original senkrecht zur anderen Seite: 60,85 → 140,100 ergibt 75,5 (PAR 07/08)
                            let h = fp_model::FillHandles {
                                valid: true,
                                start: a,
                                end: b,
                                second: Point::new(a.x + (b.y - a.y), a.y - (b.x - a.x)),
                            };
                            match &mut e.kind {
                                ElementKind::Path(pe) => pe.fill_handles = h,
                                ElementKind::Text(t) => t.fill_handles = h,
                                _ => {}
                            }
                        });
                    }
                    Ok(CommandOutput::doc().labeled(fp_i18n::t("msg-step-edit-fill")))
                })
            }),
    );

    // Ebenen
    r.add(
        cmd("layer.add")
            .label("Ebene hinzufügen")
            .requires(DOC)
            .opt("name", ParamKind::String, "Name")
            .opt(
                "sibling",
                ParamKind::Bool,
                "Knopf „Neue Ebene“ im Ebenen-Bedienfeld: bei aktueller Unterebene wird die neue Ebene ihr Geschwister darüber",
            )
            .run(|s, p| {
                let n = p.opt_str("name")?;
                let sibling = p.bool_or("sibling", false)?;
                with_doc(s, |d| {
                    let id = match sibling {
                        true => add_sibling_sublayer(d, n.as_deref())?,
                        false => None,
                    };
                    let id = match id {
                        Some(id) => id,
                        None => super::edit::add_layer(d, n.as_deref(), LayerKind::Normal)?,
                    };
                    Ok(CommandOutput::doc_with(json!({"layer": id.0})))
                })
            }),
    );
    r.add(
        cmd("layer.delete")
            .label("Ebene löschen")
            .requires(DOC)
            .opt(
                "layer",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "Ebenen-ID (Vorgabe: aktive)",
            )
            .run(|s, p| {
                with_doc(s, |d| {
                    let id = layer_param(d, p)?;
                    let page = d.page;
                    let normals = docops::own_layers(&d.doc, page)
                        .iter()
                        .filter(|(_, l)| l.kind == LayerKind::Normal)
                        .count();
                    // Unterebene (kann seit B-5 aktiv sein): ihr Element samt Inhalt entfernen, aktiv wird
                    // die Ebene der obersten Stufe, in der sie lag
                    if docops::layer_slot(&d.doc, page, id).is_none()
                        && let Some(el) = docops::sublayer_element(&d.doc, page, id)
                    {
                        let removed = docops::remove_ids(Arc::make_mut(&mut d.doc), page, &[el]);
                        let parent = removed.first().and_then(|(path, _)| {
                            docops::own_layers(&d.doc, page)
                                .into_iter()
                                .find(|(s, _)| *s == path.slot)
                                .map(|(_, l)| l.id)
                        });
                        d.layer = parent.or_else(|| docops::top_normal_layer(&d.doc, page));
                        d.selection.clear();
                        return Ok(CommandOutput::doc());
                    }
                    let slot = docops::layer_slot(&d.doc, page, id)
                        .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-layer")))?;
                    let is_web = docops::own_layers(&d.doc, page)
                        .iter()
                        .any(|(s, l)| *s == slot && l.kind == LayerKind::Web);
                    if is_web || normals <= 1 {
                        return Err(EngineError::Failed(fp_i18n::t("err-layer-no-delete")));
                    }
                    let pg = Arc::make_mut(&mut d.doc)
                        .page_mut(page)
                        .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
                    pg.root.layers.remove(slot);
                    d.layer = docops::top_normal_layer(&d.doc, page);
                    d.selection.clear();
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(
        cmd("layer.rename")
            .label("Ebene umbenennen")
            .requires(DOC)
            .opt(
                "layer",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "Ebenen-ID",
            )
            .param("name", ParamKind::String, "Name")
            .run(|s, p| {
                let n = p.str("name")?;
                edit_layer(s, p, move |l| l.name = n)
            }),
    );
    r.add(
        cmd("layer.setVisible")
            .label("Ebene ein-/ausblenden")
            .requires(DOC)
            .opt(
                "layer",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "Ebenen-ID",
            )
            .param("visible", ParamKind::Bool, "sichtbar")
            .opt(
                "dragContinue",
                ParamKind::Bool,
                "Fortsetzung eines Zugs über die Augen (ein Verlaufsschritt, TW-69)",
            )
            .run(|s, p| {
                let v = p.bool_or("visible", true)?;
                // TW-54: alle Status und Cels, bei Unterebenen auch das Element (wie das Auge im Bedienfeld)
                with_doc(s, |d| {
                    let id = layer_param(d, p)?;
                    let page = d.page;
                    docops::set_layer_shown(Arc::make_mut(&mut d.doc), page, id, v)
                        .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-layer")))?;
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(
        cmd("layer.setLocked")
            .label("Ebene sperren")
            .requires(DOC)
            .opt(
                "layer",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "Ebenen-ID",
            )
            .param("locked", ParamKind::Bool, "gesperrt")
            .run(|s, p| {
                let v = p.bool_or("locked", true)?;
                let out = edit_layer(s, p, move |l| {
                    for c in &mut l.cels {
                        Arc::make_mut(c).locked = v;
                    }
                })?;
                // RE spec 65 rule 3 (64 §3 rule 7): locking and unlocking any layer resolve the
                // current layer again from the chosen one: the first unlocked layer from it
                // upward, the web layer included, wrapping from flat index 0
                with_doc(s, |d| {
                    d.resolve_current_layer();
                    Ok(())
                })?;
                Ok(out)
            }),
    );
    r.add(
        cmd("layer.setShared")
            .label("Ebene über Status teilen")
            .requires(DOC)
            .opt("layer", ParamKind::Integer { min: Some(0), max: None }, "Ebenen-ID")
            .param("shared", ParamKind::Bool, "geteilt")
            .opt("copyTo", ParamKind::String, "Aufheben: freigegebene Daten in \"current\" (aktuellen Status) oder \"all\" (alle Status, Vorgabe) kopieren")
            .run(|s, p| {
                let v = p.bool_or("shared", true)?;
                let current_only = p.opt_str("copyTo")?.is_some_and(|c| c.eq_ignore_ascii_case("current"));
                let (frames, frame) = (s.doc()?.frame_count(), s.doc()?.frame);
                edit_layer(s, p, move |l| {
                    let was = l.shared_across_frames;
                    l.shared_across_frames = v;
                    if v && !was {
                        // re 40 §2b: Freigeben übernimmt den Inhalt des aktuellen Status, die Objekte
                        // der anderen Status werden gelöscht
                        let cur = l.cels.get(frame).or(l.cels.first()).cloned().unwrap_or_else(|| Arc::new(fp_model::Cel::empty()));
                        l.cels = vec![cur];
                    } else if !v {
                        // Aufheben: freigegebene Daten in den aktuellen Status oder in alle kopieren
                        let first = l.cels.first().cloned().unwrap_or_else(|| Arc::new(fp_model::Cel::empty()));
                        if was && current_only {
                            l.cels = (0..frames.max(1)).map(|i| if i == frame { first.clone() } else { Arc::new(fp_model::Cel::empty()) }).collect();
                        }
                        while l.cels.len() < frames {
                            l.cels.push(Arc::new((*first).clone()));
                        }
                    }
                })
            }),
    );
    // The plain click on a layer row passes `select` (the row's own objects): one History row
    // "Layer Change" whose undo restores the current layer and the selection (RE spec 64 §2.4);
    // without `select` only the current layer changes and no row is written.
    r.add(cmd("layer.setActive").label("Ebenenänderung").requires(DOC).param("layer", ParamKind::Integer { min: Some(0), max: None }, "Ebenen-ID").opt("select", ParamKind::Ids, "Klick auf eine Ebenenzeile: diese Objekte mit auswählen (ein Protokollschritt „Ebenenänderung“)").run(|s, p| {
        let select = p.opt_ids("select")?;
        let row = select.is_some();
        let changed = with_doc(s, |d| {
            let before = (d.layer, d.selection.elements.clone());
            let id = layer_param(d, p)?;
            // eigene Ebene oder über Seiten geteilte Ebene dieser Seite
            let shared = d.doc.pages.get(d.page).is_some_and(|pg| {
                pg.root.layers.iter().any(|sl| matches!(sl, LayerSlot::Shared(r) if d.doc.shared_layer(&r.id).and_then(|x| x.layer.as_ref()).is_some_and(|l| l.id == id)))
            });
            // auch eine Unterebene kann aktiv sein (B-2 BA-030)
            if !shared && docops::sublayer_element(&d.doc, d.page, id).is_none() {
                docops::layer_slot(&d.doc, d.page, id).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-layer")))?;
            }
            d.choose_layer(Some(id));
            if let Some(ids) = select {
                let mut usable = Vec::new();
                for e in ids {
                    let path = docops::find_on_page(&d.doc, d.page, e)
                        .ok_or(EngineError::ElementNotFound(e.0))?;
                    if !docops::is_locked(&d.doc, d.page, &path) {
                        usable.push(e);
                    }
                }
                d.selection.set(usable);
            }
            Ok(before != (d.layer, d.selection.elements.clone()))
        })?;
        if row && changed {
            s.force_history_step();
        }
        Ok(CommandOutput::doc())
    }));
    // RE spec 65 rule 1 (TW-217): the click on a layer's selection circle in the Layers panel
    // (Alt copies) and the script's `moveSelectionToLayer`: one History row "Layer Change"
    r.add(
        cmd(super::MOVE_SELECTION_TO_LAYER)
            .label("Ebenenänderung")
            .requires(DOC)
            .param(
                "layer",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "Ziel-Ebenen-ID (Ebene oder Unterebene)",
            )
            .opt_default(
                "copy",
                ParamKind::Bool,
                json!(false),
                "Kopien statt Verschieben (Alt)",
            )
            .run(|s, p| {
                let copy = p.bool_or("copy", false)?;
                with_doc(s, |d| {
                    let target = layer_param(d, p)?;
                    move_selection_to_layer(d, target, copy)
                })
            }),
    );
    r.add(
        cmd("layer.move")
            .label("Ebene verschieben")
            .requires(DOC)
            .opt(
                "layer",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "Ebenen-ID",
            )
            .param(
                "index",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "neue Position in der Ebenenliste (0 = unten)",
            )
            .opt(
                "into",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "Ebenen-ID: als oberste Unterebene in diese Ebene verschieben (TW-86, re 40 §2a)",
            )
            .run(|s, p| {
                let to = p.usize("index")?;
                let into = p.opt_i64("into")?.map(|v| fp_model::LayerId(v as u64));
                with_doc(s, |d| {
                    let id = layer_param(d, p)?;
                    let page = d.page;
                    if let Some(target) = into {
                        move_layer_into(d, id, target)?;
                        return Ok(CommandOutput::doc());
                    }
                    // Unterebene (kann aktiv sein): Position in ihrer Elternebene, 0 = unten (B-9)
                    if let Some(el) = docops::as_sublayer(&d.doc, page, id) {
                        docops::move_sublayer(Arc::make_mut(&mut d.doc), page, el, to)
                            .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-layer")))?;
                        return Ok(CommandOutput::doc());
                    }
                    let slot = docops::layer_slot(&d.doc, page, id)
                        .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-layer")))?;
                    let pg = Arc::make_mut(&mut d.doc)
                        .page_mut(page)
                        .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
                    let l = pg.root.layers.remove(slot);
                    let to = to.min(pg.root.layers.len());
                    pg.root.layers.insert(to, l);
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(
        cmd("layer.duplicate")
            .label("Ebene duplizieren")
            .requires(DOC)
            .opt(
                "layer",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "Ebenen-ID",
            )
            .run(|s, p| {
                with_doc(s, |d| {
                    let id = layer_param(d, p)?;
                    let page = d.page;
                    // Unterebene: Kopie direkt darüber in derselben Elternebene, wird aktiv (B-9)
                    if let Some(el) = docops::as_sublayer(&d.doc, page, id) {
                        let nid = docops::duplicate_sublayer(
                            Arc::make_mut(&mut d.doc),
                            page,
                            el,
                            1,
                            |i, _| i,
                        )
                        .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-layer")))?;
                        d.layer = Some(nid);
                        return Ok(CommandOutput::doc_with(json!({"layer": nid.0})));
                    }
                    let slot = docops::layer_slot(&d.doc, page, id)
                        .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-layer")))?;
                    let doc = Arc::make_mut(&mut d.doc);
                    let Some(LayerSlot::Own(l)) = doc
                        .pages
                        .get(page)
                        .and_then(|pg| pg.root.layers.get(slot))
                        .cloned()
                    else {
                        return Err(EngineError::NotFound(fp_i18n::t("err-nf-layer")));
                    };
                    let mut nl = (*l).clone();
                    let mut idg = take_ids(doc);
                    nl.reassign_ids(&mut idg);
                    doc.ids = idg;
                    nl.name = fp_i18n::tr!("msg-default-copy", name = nl.name.as_str());
                    let nid = nl.id;
                    doc.page_mut(page)
                        .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?
                        .root
                        .layers
                        .insert(slot + 1, LayerSlot::Own(Arc::new(nl)));
                    d.layer = Some(nid);
                    Ok(CommandOutput::doc_with(json!({"layer": nid.0})))
                })
            }),
    );

    // Seiten
    r.add(
        cmd("page.add")
            .label("Seite hinzufügen")
            .requires(DOC)
            .opt("name", ParamKind::String, "Name")
            .run(|s, p| {
                let n = p.opt_str("name")?;
                s.execute("edit.insert.page", json!({"name": n}))
                    .map(|o| CommandOutput {
                        effect: crate::command::Effect::None,
                        ..o
                    })
            }),
    );
    r.add(
        cmd("page.delete")
            .label("Seite löschen")
            .requires(&[Requirement::Document, Requirement::MultiplePages])
            .opt(
                "index",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "Seitenindex (Vorgabe: aktuelle)",
            )
            .run(|s, p| {
                with_doc(s, |d| {
                    let i = p
                        .opt_i64("index")?
                        .map(|v| v.max(0) as usize)
                        .unwrap_or(d.page);
                    if i >= d.doc.pages.len() || d.doc.pages.len() <= 1 {
                        return Err(EngineError::NotFound(fp_i18n::t("err-nf-page")));
                    }
                    d.remember_page_layer();
                    let doc = Arc::make_mut(&mut d.doc);
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
            }),
    );
    r.add(
        cmd("page.duplicate")
            .label("Seite duplizieren")
            .requires(DOC)
            .opt(
                "index",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "Seitenindex",
            )
            .run(|s, p| {
                with_doc(s, |d| {
                    let i = p
                        .opt_i64("index")?
                        .map(|v| v.max(0) as usize)
                        .unwrap_or(d.page);
                    d.remember_page_layer();
                    let doc = Arc::make_mut(&mut d.doc);
                    let mut pg = (**doc
                        .pages
                        .get(i)
                        .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?)
                    .clone();
                    let mut idg = take_ids(doc);
                    for slot in &mut pg.root.layers {
                        if let LayerSlot::Own(l) = slot {
                            Arc::make_mut(l).reassign_ids(&mut idg);
                        }
                    }
                    doc.ids = idg;
                    pg.id = Some(fp_model::Guid::new_random());
                    // RE spec 70 §2.2: "<name> Copy", then "Copy 1", "Copy 2" … (FB-50)
                    pg.name = Some(docops::page_copy_name(doc, pg.display_name()));
                    pg.is_master = Some(false);
                    doc.pages.insert(i + 1, Arc::new(pg));
                    // the model's current page (owner of the stored root copy) keeps its page
                    if i < doc.current_page {
                        doc.current_page += 1;
                    }
                    d.page = i + 1;
                    d.frame = 0;
                    d.layer = docops::top_normal_layer(&d.doc, d.page);
                    d.selection.clear();
                    Ok(CommandOutput::doc_with(json!({"page": i + 1})))
                })
            }),
    );
    r.add(
        cmd("page.rename")
            .label("Seite umbenennen")
            .requires(DOC)
            .opt(
                "index",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "Seitenindex",
            )
            .param("name", ParamKind::String, "Name")
            .run(|s, p| {
                let n = p.str("name")?;
                with_doc(s, |d| {
                    let i = p
                        .opt_i64("index")?
                        .map(|v| v.max(0) as usize)
                        .unwrap_or(d.page);
                    Arc::make_mut(&mut d.doc)
                        .page_mut(i)
                        .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?
                        .name = Some(n);
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(
        cmd("page.setCurrent")
            .label("Seite wechseln")
            .requires(DOC)
            .param(
                "index",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "Seitenindex",
            )
            .run(|s, p| {
                let i = p.usize("index")?;
                // RE spec 70 §2.1: a page change is a History row of its own in the reference
                // (also without a selection); inside a script it joins the script's row
                if s.active().is_some_and(|d| d.page != i) {
                    s.force_history_step();
                }
                with_doc(s, |d| {
                    if i >= d.doc.pages.len() {
                        return Err(EngineError::NotFound(fp_i18n::t("err-nf-page")));
                    }
                    // RE spec 65 rule 3: every page keeps its chosen layer; the first visit
                    // starts on the bottom-most unlocked layer
                    d.remember_page_layer();
                    d.page = i;
                    d.frame = 0;
                    d.enter_page_layer();
                    d.selection.clear();
                    // History row "Change Page Selection" like the reference (RE spec 70 §2.1)
                    Ok(CommandOutput::doc().labeled(fp_i18n::t("msg-step-change-page-selection")))
                })
            }),
    );

    // Status
    r.add(
        cmd("state.add")
            .label("Status hinzufügen")
            .requires(DOC)
            .opt_default(
                "count",
                ParamKind::Integer {
                    min: Some(1),
                    max: Some(1000),
                },
                json!(1),
                "Anzahl",
            )
            .opt(
                "at",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "Position (Vorgabe: hinter dem aktuellen)",
            )
            .run(|s, p| {
                let n = p.i64_or("count", 1)?.max(1) as usize;
                let at = p.opt_i64("at")?;
                with_doc(s, |d| {
                    let at = at.map(|v| v.max(0) as usize).unwrap_or(d.frame + 1);
                    let page = d.page;
                    docops::insert_frames(Arc::make_mut(&mut d.doc), page, at, n)
                        .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
                    d.frame = at.min(d.frame_count() - 1);
                    d.selection.clear();
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(
        cmd("state.delete")
            .label("Status löschen")
            .requires(&[Requirement::Document, Requirement::MultipleFrames])
            .opt(
                "index",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "Status",
            )
            .run(|s, p| {
                with_doc(s, |d| {
                    let i = p
                        .opt_i64("index")?
                        .map(|v| v.max(0) as usize)
                        .unwrap_or(d.frame);
                    let page = d.page;
                    docops::delete_frame(Arc::make_mut(&mut d.doc), page, i)
                        .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-state")))?;
                    d.frame = d.frame.min(d.frame_count() - 1);
                    d.selection.clear();
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(
        cmd("state.duplicate")
            .label("Status duplizieren")
            .requires(DOC)
            .opt(
                "index",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "Status",
            )
            .run(|s, p| {
                with_doc(s, |d| {
                    let i = p
                        .opt_i64("index")?
                        .map(|v| v.max(0) as usize)
                        .unwrap_or(d.frame);
                    let page = d.page;
                    let doc = Arc::make_mut(&mut d.doc);
                    docops::insert_frames(doc, page, i + 1, 1)
                        .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
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
            }),
    );
    r.add(
        cmd("state.setCurrent")
            .label("Status wechseln")
            .requires(DOC)
            .param(
                "index",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "Status",
            )
            .run(|s, p| {
                let i = p.usize("index")?;
                with_doc(s, |d| {
                    if i >= d.frame_count() {
                        return Err(EngineError::NotFound(fp_i18n::t("err-nf-state")));
                    }
                    d.frame = i;
                    // RE spec 65 rule 3: a layer locked in the new state hands over to the next
                    // unlocked one above (wrapping), which becomes the chosen layer
                    d.adopt_resolved_layer();
                    d.selection.clear();
                    Ok(CommandOutput::doc())
                })
            }),
    );
    // Umordnen per Ziehen im Status- bzw. Seiten-Bedienfeld (CT-C-02/03), Aufbau wie `layer.move`:
    // `index` = neue Position (nach dem Herausnehmen), Quelle Vorgabe = aktueller Status/aktuelle Seite.
    r.add(
        cmd("state.move")
            .label("Status verschieben")
            .requires(DOC)
            .opt(
                "state",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "Status",
            )
            .param(
                "index",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "neue Position (0 = erste)",
            )
            .run(|s, p| {
                let to = p.usize("index")?;
                let from = p.opt_i64("state")?.map(|v| v.max(0) as usize);
                with_doc(s, |d| {
                    let from = from.unwrap_or(d.frame);
                    let n = d.frame_count();
                    if from >= n {
                        return Err(EngineError::NotFound(fp_i18n::t("err-nf-state")));
                    }
                    let to = to.min(n - 1);
                    let page = d.page;
                    docops::move_frame(Arc::make_mut(&mut d.doc), page, from, to)
                        .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-state")))?;
                    d.frame = docops::moved_index(d.frame, from, to);
                    d.selection.clear();
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(
        cmd("page.move")
            .label("Seite verschieben")
            .requires(DOC)
            .opt(
                "page",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "Seitenindex",
            )
            .param(
                "index",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "neue Position (0 = erste)",
            )
            .run(|s, p| {
                let to = p.usize("index")?;
                let from = p.opt_i64("page")?.map(|v| v.max(0) as usize);
                with_doc(s, |d| {
                    let from = from.unwrap_or(d.page);
                    let n = d.doc.pages.len();
                    if from >= n {
                        return Err(EngineError::NotFound(fp_i18n::t("err-nf-page")));
                    }
                    let to = to.min(n - 1);
                    docops::move_page(Arc::make_mut(&mut d.doc), from, to)
                        .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
                    d.page = docops::moved_index(d.page, from, to);
                    Ok(CommandOutput::doc())
                })
            }),
    );

    // Farben des Werkzeugbedienfelds (Pseudo-Werkzeuge D/X, 10 §2.1)
    r.add(cmd("color.set").label("Farben setzen").opt("stroke", ParamKind::Color, "Strichfarbe").opt("fill", ParamKind::Color, "Füllfarbe").run(|s, p| {
        let (st, fi) = (p.opt_color("stroke")?, p.opt_color("fill")?);
        s.set_colors(st, fi);
        Ok(CommandOutput::session_with(json!({"stroke": argb_hex(s.tools.stroke_color), "fill": argb_hex(s.tools.fill_color)})))
    }));
    r.add(
        cmd("color.swap")
            .label("Pinsel- und Füllungsfarben austauschen")
            .describe("Pseudo-Werkzeug 35278 (Kürzel X)")
            .run(swap_colors),
    );
    r.add(
        cmd("color.defaults")
            .label("Standardfarben aktivieren")
            .describe("Pseudo-Werkzeug 35279 (Kürzel D): nur die Farben; Spitze, Größe und Lage bleiben, ein Strich „keine“ wird 1-Pixel weich 1 px, eine Füllung „keine“ einfarbig")
            .run(|s, _| {
                s.default_tool_colors();
                Ok(CommandOutput::session())
            }),
    );
    r.add(
        cmd("color.none")
            .label("Keine Pinselstriche oder Füllungen")
            .describe("Pseudo-Werkzeug 35082")
            .run(|s, _| {
                s.set_colors(Some(Argb(0x00ff_ffff)), Some(Argb(0x00ff_ffff)));
                Ok(CommandOutput::session())
            }),
    );
    r.add(
        cmd("color.sample")
            .label("Pipette")
            .requires(DOC)
            .param("point", ParamKind::Point, "Punkt")
            .opt_default(
                "target",
                ParamKind::Enum {
                    values: vec!["fill".into(), "stroke".into(), "none".into()],
                },
                json!("fill"),
                "Ziel",
            )
            .opt_default(
                "samples",
                ParamKind::Integer {
                    min: Some(1),
                    max: Some(5),
                },
                json!(1),
                "1, 3 oder 5 Pixel (Mittelwert)",
            )
            .opt_default(
                "applyToSelection",
                ParamKind::Bool,
                json!(false),
                "gewählte Vektoren bekommen die Farbe (Pipette)",
            )
            .run(|s, p| {
                let pt = p.point("point")?;
                let n = match p.i64_or("samples", 1)? {
                    3 => 3,
                    5 => 5,
                    _ => 1,
                };
                let c = super::bitmap::sample_color(s, pt, n)?;
                let target = p.opt_str("target")?;
                match target.as_deref() {
                    Some("stroke") => s.set_colors(Some(c), None),
                    Some("none") => {}
                    _ => s.set_colors(None, Some(c)),
                }
                // Pipette mit gewählten Vektoren: sie bekommen die Farbe, Schritt „Füllfarbe“ bzw.
                // „Strichfarbe“ (PAR 07/11)
                let vectors = s.doc().map(|d| {
                    super::any_selected(d, |e| {
                        !matches!(e.kind, fp_model::element::ElementKind::Bitmap(_))
                    })
                })?;
                if p.bool_or("applyToSelection", false)?
                    && vectors
                    && target.as_deref() != Some("none")
                {
                    let stroke = target.as_deref() == Some("stroke");
                    let (name, key) = if stroke {
                        ("element.setStroke", "msg-step-stroke-color")
                    } else {
                        ("element.setFill", "msg-step-fill-color")
                    };
                    let reg = std::sync::Arc::clone(s.registry());
                    let cmd = reg
                        .get(name)
                        .ok_or_else(|| EngineError::UnknownCommand(name.into()))?;
                    let ip = crate::command::Params::new(name, json!({"color": argb_hex(c)}));
                    crate::command::validate(cmd.info(), &ip)?;
                    let out = cmd.apply(s, &ip)?;
                    return Ok(CommandOutput {
                        value: json!({"color": argb_hex(c)}),
                        ..out
                    }
                    .labeled(fp_i18n::t(key)));
                }
                Ok(CommandOutput::session_with(json!({"color": argb_hex(c)})))
            }),
    );

    // Werkzeuge und Verlauf
    r.add(
        cmd("tool.select")
            .label("Werkzeug wählen")
            .param(
                "tool",
                ParamKind::String,
                "Werkzeugname (tool.pen, Stift, pen) oder GUID",
            )
            .run(|s, p| {
                let n = p.str("tool")?;
                let t = tools::by_name(&n)
                    .or_else(|| tools::by_guid(&n))
                    .ok_or_else(|| {
                        EngineError::NotFound(fp_i18n::tr!("err-nf-tool", name = n.as_str()))
                    })?;
                s.set_tool(t)?;
                Ok(CommandOutput::session_with(
                    json!({"tool": tools::info(t).map(|i| i.name)}),
                ))
            }),
    );
    r.add(
        cmd("history.replay")
            .label("Schritte wiederholen")
            .requires(DOC)
            .param("commands", ParamKind::Json, "[{command, params}, …]")
            .opt("label", ParamKind::String, "Verlaufsbeschriftung")
            .run(|s, p| {
                let cmds: Vec<RecordedCommand> = p.parse("commands")?.unwrap_or_default();
                let label = p
                    .opt_str("label")?
                    .unwrap_or_else(|| "Schritte wiederholen".into());
                s.replay(&cmds, &label)?;
                Ok(CommandOutput::session())
            }),
    );
    r.add(
        cmd("history.goTo")
            .label("Verlauf: zu Schritt")
            .requires(DOC)
            .param(
                "position",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "Anzahl ausgeführter Schritte",
            )
            .run(|s, p| {
                s.history_go_to(p.usize("position")?)?;
                Ok(CommandOutput::session())
            }),
    );
    r.add(cmd("history.export").label("Schritte als Befehle").requires(DOC).opt("from", ParamKind::Integer { min: Some(0), max: None }, "erster Schritt").opt("to", ParamKind::Integer { min: Some(0), max: None }, "Ende (exklusiv)").run(|s, p| {
        let d = s.doc()?;
        let n = d.history.undo_count();
        let from = p.i64_or("from", 0)?.max(0) as usize;
        let to = p.opt_i64("to")?.map(|v| v.max(0) as usize).unwrap_or(n);
        let cmds = d.history.export(from..to);
        Ok(CommandOutput::value(json!({"commands": cmds, "script": crate::history::to_script(&cmds), "labels": d.history.labels()})))
    }));
}

/// Ziel von Deckkraft/Mischmodus: bei einem maskierten Einzelbild ([`Element::masked_single`]) der
/// **Inhalt**, nicht die umhüllende Maskengruppe (re 29b §2, BA-006: das Original zeigt und ändert
/// die Werte des Bildes), sonst das Element selbst.
fn content_of(e: &mut Element) -> &mut Element {
    if e.masked_single().is_none() {
        return e;
    }
    e.cache = None;
    match &mut e.kind {
        ElementKind::Group(g) => {
            let c = Arc::make_mut(&mut g.children[0]);
            c.cache = None;
            c
        }
        _ => unreachable!("masked_single ist eine Gruppe"),
    }
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

/// Pinselstärke eines Elements (und der Kinder einer Gruppe) mit dem Faktor `f` skalieren
/// (gerundet, mindestens 1 px; „Attribute skalieren“ beim Ändern von B/H, PAR 17/16).
pub(crate) fn scale_stroke_width(e: &mut Element, f: f32) {
    docops::edit_path_attrs(e, &|a| {
        if let Some(b) = a.brush.as_mut() {
            b.diameter = ((b.diameter as f32) * f).round().max(1.0) as i32;
        }
    });
}

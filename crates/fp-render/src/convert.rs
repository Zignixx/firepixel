//! Schmale Adapterschicht: Dokumentmodell (`fp-model`) → Eingabe-Strukturen von `fp-vector`,
//! `fp-text`, `fp-effects` und Pixelpuffer von `fp-raster`.
//!
//! Die Zuordnung folgt den Tabellen in den Moduldokus von `fp_vector::attrs` und `fp_text::model`
//! (Vorlagen: deren `tests\common\mod.rs`).

use std::sync::Arc;

use fp_color::Argb;
use fp_effects::params::{EffectParams, ParamValue};
use fp_effects::{EffectEntry, LiveEffect};
use fp_geom::{BezierContour, BezierNode, IRect, Point, Rect, Vec2};
use fp_model::{
    BitmapNode, Contour, DataValue, Document, EffectList, ImageData, PathAttrs, PathElem, TextElem,
};
use fp_raster::{Mask, RgbaImage};
use fp_vector as fv;

use crate::nine::PointMap;

/// Bitmap eines `IMGv`-Inhalts als [`RgbaImage`] an seiner (gerundeten) Lage. `None`, wenn Kacheln
/// fehlen oder die Größe unsinnig ist.
pub fn image_of(img: &ImageData) -> Option<RgbaImage> {
    let (px, missing) = img.to_rgba();
    if missing {
        return None;
    }
    let p = img.position();
    let (x, y) = (round_coord(p.x as f64), round_coord(p.y as f64));
    let b = IRect::from_xywh(
        x,
        y,
        img.grid.width.min(i32::MAX as u32) as i32,
        img.grid.height.min(i32::MAX as u32) as i32,
    );
    RgbaImage::from_rgba_bytes(b, &px).ok()
}

/// Bitmap eines Knotens (`BitmapNode`).
pub fn image_of_node(n: &BitmapNode) -> Option<RgbaImage> {
    image_of(&n.bitmap.image)
}

/// Koordinate auf ganze Pixel (wie `int(round(x))` der Referenz: Gleichstand gerade).
pub fn round_coord(v: f64) -> i32 {
    if v.is_finite() {
        v.round_ties_even().clamp(-1.0e9, 1.0e9) as i32
    } else {
        0
    }
}

/// 8-Bit-Maske (Textur) aus `MSKv`.
pub fn mask_of(m: &fp_model::MaskData) -> Option<Arc<Mask>> {
    let (data, missing) = m.grid.to_gray();
    if missing && data.is_empty() {
        return None;
    }
    Mask::from_raw(
        IRect::new(0, 0, m.grid.width as i32, m.grid.height as i32),
        data,
    )
    .ok()
    .map(Arc::new)
}

/// Effektliste des Modells → Einträge für [`fp_effects::apply_chain`].
pub fn effect_entries(list: &EffectList) -> Vec<EffectEntry> {
    list.effects
        .iter()
        .map(|e| {
            let mut p = EffectParams::new();
            for d in &e.params.entries {
                match &d.value {
                    DataValue::Scalar(s) => {
                        p.insert(d.key.clone(), ParamValue::Text(s.clone()));
                    }
                    DataValue::Points(pl) => {
                        p.insert(
                            d.key.clone(),
                            ParamValue::Points(pl.points.iter().map(|q| (q.x, q.y)).collect()),
                        );
                    }
                    _ => {
                        if let Some(t) = &d.text {
                            p.insert(d.key.clone(), ParamValue::Text(t.clone()));
                        }
                    }
                }
            }
            EffectEntry {
                effect: LiveEffect::parse(e.moa_id().unwrap_or(""), &p),
                visible: e.is_visible(),
            }
        })
        .collect()
}

/// Bézier-Konturen eines Pfads (`PBLv`) mit `BZCi` je Knoten.
pub fn bezier_contours(contours: &[Contour]) -> Vec<(BezierContour, Vec<u32>)> {
    contours
        .iter()
        .filter(|c| !c.nodes.is_empty())
        .map(|c| {
            let nodes = c
                .nodes
                .iter()
                .map(|n| BezierNode::new(n.anchor, n.pred, n.succ))
                .collect();
            let counts = c
                .nodes
                .iter()
                .map(|n| n.pressure.as_ref().and_then(|p| p.count).unwrap_or(1))
                .collect();
            (BezierContour::new(nodes, c.closed), counts)
        })
        .collect()
}

fn map_point(map: &PointMap, p: Point) -> Point {
    let v = map.apply(Vec2::new(p.x as f64, p.y as f64));
    Point::new(v.x as f32, v.y as f32)
}

/// Konturen als Polylinien in Dokumentkoordinaten. Affine Abbildungen transformieren die
/// gespeicherte Polylinie (`PCLv`, wie die Referenz); 9-Slice bildet Anker und Griffe ab und flacht
/// danach neu ab (13 §3.3).
pub fn path_contours(p: &PathElem, map: &PointMap) -> Vec<fv::PathContour> {
    if let (Some(pl), false) = (&p.polyline, map.is_nine_slice()) {
        let out: Vec<fv::PathContour> = pl
            .iter()
            .filter(|l| !l.points.is_empty())
            .map(|l| {
                let mut c = fv::PathContour::new(Vec::with_capacity(l.points.len()), l.closed);
                for q in &l.points {
                    let pos = Vec2::new(q.pos.x as f64, q.pos.y as f64);
                    c.points.push(map.apply(pos));
                    c.dynamics.push(fv::PointDynamics {
                        pressure: q.pressure,
                        velocity: q.velocity,
                        seed: q.random,
                    });
                }
                c
            })
            .collect();
        if !out.is_empty() {
            return out;
        }
    }
    bezier_contours(&p.contours)
        .into_iter()
        .map(|(c, n)| {
            let mapped = if map.is_identity() {
                c
            } else {
                BezierContour::new(
                    c.nodes
                        .iter()
                        .map(|nd| {
                            BezierNode::new(
                                map_point(map, nd.pt),
                                map_point(map, nd.ctrl_in),
                                map_point(map, nd.ctrl_out),
                            )
                        })
                        .collect(),
                    c.closed,
                )
            };
            fv::PathContour::from_bezier(&mapped, Some(&n))
        })
        .collect()
}

const SRC: &[u8] = b"PSHVR";
const TGT: &[u8] = b"ZAOBRHLS";

/// Pinsel (`BPLv`) → [`fv::BrushParams`].
pub fn brush_params(b: &fp_model::Brush) -> fv::BrushParams {
    let u = |v: i32| v.max(0) as u32;
    let mut p = fv::BrushParams {
        diameter: u(b.diameter),
        angle: u(b.angle),
        min_size: u(b.min_size),
        aspect: u(b.aspect),
        max_count: u(b.max_count),
        softness: u(b.softness),
        shape: if b.shape == 1 {
            fv::TipShape::Circle
        } else {
            fv::TipShape::Square
        },
        bell_curve: b.soften_mode == 1,
        kind: if b.brush_type == 0 {
            fv::BrushType::Natural
        } else {
            fv::BrushType::Simple
        },
        feedback: fv::Feedback::from_u32(u(b.feedback)),
        tip_spacing_mode: match b.tip_spacing_mode {
            1 => fv::TipSpacingMode::Diagonal,
            2 => fv::TipSpacingMode::Circular,
            _ => fv::TipSpacingMode::Random,
        },
        tip_coloring_mode: match b.tip_coloring_mode {
            1 => fv::TipColoringMode::Uniform,
            2 => fv::TipColoringMode::Complementary,
            3 => fv::TipColoringMode::Hue,
            4 => fv::TipColoringMode::Shadow,
            _ => fv::TipColoringMode::Random,
        },
        alpha_remap: u(b.alpha_remap),
        spacing: u(b.spacing),
        flow_rate: u(b.flow_rate),
        tip_count: u(b.tip_count),
        tip_spacing: u(b.tip_spacing),
        blackness: u(b.blackness),
        concentration: u(b.concentration),
        texture_blend: u(b.texture_blend),
        texture_edge: u(b.texture_edge),
        antialiased: b.antialiased,
        dashes: Vec::new(),
        sense: [[0; 5]; 8],
    };
    for k in 0..(b.dash_count.clamp(0, 3) as usize) {
        p.dashes.push((u(b.dash_on[k]), u(b.dash_off[k])));
    }
    for s in &b.sensitivities {
        if let (Some(si), Some(ti)) = (
            SRC.iter().position(|c| *c == s.source),
            TGT.iter().position(|c| *c == s.target),
        ) && s.value != 0
        {
            p.sense[ti][si] = fv::BrushParams::sense_from_permille(s.value.max(0) as u32);
        }
    }
    p
}

fn stops(v: &[fp_model::GradientStop]) -> Vec<fv::GradientStop> {
    v.iter()
        .map(|s| fv::GradientStop {
            pos: s.position as f64,
            color: s.color,
        })
        .collect()
}

/// Füllung und Strich eines `PATv` mit Griffen (bereits abgebildet).
fn paint(
    attrs: &PathAttrs,
    handles: fv::GradientHandles,
) -> (Option<fv::Fill>, Option<fv::Stroke>) {
    let fill = attrs.fill.as_ref().map(|f| {
        let (sh, sx) = (f.shape.max(0) as u32, f.shape_ext.max(0) as u32);
        let kind = if sx == 17 || fv::GradientShape::from_tags(sh, sx).is_some() {
            let (cst, ost) = f
                .gradient
                .as_ref()
                .map(|g| (stops(g.color_stops()), stops(g.opacity_stops())))
                .unwrap_or_default();
            let dither = f
                .gradient
                .as_ref()
                .and_then(|g| g.stops.as_ref())
                .and_then(|s| s.dither)
                .unwrap_or(false);
            fv::FillKind::Gradient(fv::Gradient {
                shape: fv::GradientShape::from_tags(sh, sx).unwrap_or(fv::GradientShape::Linear),
                color_stops: cst,
                opacity_stops: ost,
                dither,
            })
        } else if sh == 16 {
            let d = f.dither_colors.unwrap_or([0xff00_0000; 4]);
            fv::FillKind::WebDither([Argb(d[0]), Argb(d[1]), Argb(d[2]), Argb(d[3])])
        } else if sh == 15 {
            let img = f
                .pattern
                .as_ref()
                .and_then(|p| p.image.as_ref())
                .and_then(|n| image_of_node(n))
                .map(|i| i.translated(-i.origin().0, -i.origin().1))
                .unwrap_or_else(|| RgbaImage::empty_at(0, 0));
            fv::FillKind::Pattern(Arc::new(img))
        } else {
            fv::FillKind::Solid(attrs.fill_color)
        };
        let amount = f.texture_amount.max(0) as u32;
        let texture = if amount > 0 {
            attrs
                .fill_texture
                .as_ref()
                .and_then(|t| t.mask.as_ref())
                .and_then(mask_of)
                .map(|m| fv::FillTexture {
                    mask: m,
                    amount_permille: amount,
                })
        } else {
            None
        };
        fv::Fill {
            kind,
            edge: fv::FillEdge::from_tags(f.edge.max(0) as u32, f.feather.max(0) as u32),
            handles,
            texture,
            stamping: fv::StampingMode::from_u32(f.stamping_mode.max(0) as u32),
            transparency_dither: f.transparent_dither.unwrap_or(false),
            dither_density: f.dither_density.clamp(0, 100) as u8,
        }
    });
    let stroke = attrs.brush.as_ref().map(|b| {
        let mut s = fv::Stroke::new(brush_params(b), attrs.stroke_color);
        if b.texture_blend > 0 || b.texture_edge > 0 {
            s.texture = attrs
                .stroke_texture
                .as_ref()
                .and_then(|t| t.mask.as_ref())
                .and_then(mask_of)
                .map(|m| fv::StrokeTexture { mask: m });
        }
        s
    });
    (fill, stroke)
}

fn map_handles(h: &fp_model::FillHandles, map: &PointMap) -> fv::GradientHandles {
    let m = |p: Point| map.apply(Vec2::new(p.x as f64, p.y as f64));
    fv::GradientHandles {
        p1: m(h.start),
        p2: m(h.end),
        p3: m(h.second),
    }
}

/// Pfad (`PTHv`) → [`fv::VectorPath`] in Dokumentkoordinaten.
pub fn vector_path(p: &PathElem, map: &PointMap) -> fv::VectorPath {
    let mut v = fv::VectorPath::new(path_contours(p, map));
    v.even_odd = p.even_odd;
    v.agm = p.agm;
    v.fill_over_stroke = p.fill_on_top;
    v.placement = fv::StrokePlacement::from_u32(p.stroke_placement.max(0) as u32);
    v.ptt = p.ptt == 1;
    v.random_seed = p.random_seed;
    v.texture_offset = map.apply(Vec2::new(
        p.texture_offset.x as f64,
        p.texture_offset.y as f64,
    ));
    let (fill, stroke) = paint(&p.attrs, map_handles(&p.fill_handles, map));
    v.fill = fill;
    v.stroke = stroke;
    v
}

/// Füllung eines Pfads ohne Strich (Deckung einer Vektormaske ohne Maskenattribute, 07 §7.5).
pub fn fill_only(mut v: fv::VectorPath) -> fv::VectorPath {
    v.stroke = None;
    if v.fill.is_none() {
        v.fill = Some(fv::Fill::solid(Argb(0xff00_0000)));
    }
    if let Some(f) = &mut v.fill {
        f.kind = fv::FillKind::Solid(Argb(0xff00_0000));
        f.texture = None;
        f.transparency_dither = false;
        f.dither_density = 0;
    }
    v
}

fn char_patch(a: &fp_model::TextAttrs, header: bool) -> fp_text::CharAttrsPatch {
    let font = if header {
        a.ps_font.clone().or_else(|| a.font.clone())
    } else {
        a.font.clone()
    };
    fp_text::CharAttrsPatch {
        font,
        size: a.size.map(|v| v as f64),
        color: a.color.map(|c| Argb(c).to_rgba8()),
        bold: a.bold,
        italic: a.italic,
        underline: a.underline,
        h_scale: a.horizontal_scale.map(|v| v as f64),
        kerning: a.kerning.map(|v| v as f64),
        tracking: a.tracking.map(|v| v as f64),
        baseline_shift: a.baseline_shift.map(|v| v as f64),
        leading: a.leading.map(|v| v as f64),
        leading_mode: a
            .leading_mode
            .map(|v| fp_text::LeadingMode::from_ldmi(v as i64)),
        alignment: a.alignment.map(|v| fp_text::Alignment::from_jsti(v as i64)),
        indent: a.indent.map(|v| v as f64),
        space_before: a.space_before.map(|v| v as f64),
        space_after: a.space_after.map(|v| v as f64),
    }
}

/// Fill edge of a text object (TW-203): edge 0 Hard, 1 Anti-Alias (the text's anti-aliasing level
/// applies), 2 Feather and 3 Dither (Firepixel extension) with the width in `feather`, like the
/// fill edge of a path. Unlike [`fv::FillEdge::from_tags`], edge 1 stays anti-aliased whatever
/// `feather` holds (the anti-aliasing level menu sets edge 1 and keeps the width); feather width
/// 0 is anti-aliased and dither width 0 is Hard, as for paths. A text without fill counts as
/// anti-aliased.
pub fn text_fill_edge(t: &TextElem) -> fv::FillEdge {
    let Some(f) = t.attrs.fill.as_ref() else {
        return fv::FillEdge::AntiAliased;
    };
    let w = f.feather.clamp(0, fv::feather::MAX_FEATHER as i32) as u32;
    match f.edge {
        fp_model::Fill::EDGE_HARD => fv::FillEdge::Hard,
        fp_model::Fill::EDGE_FEATHER if w > 0 => fv::FillEdge::Feather(w),
        fp_model::Fill::EDGE_DITHER if w > 0 => fv::FillEdge::Dither(w),
        fp_model::Fill::EDGE_DITHER => fv::FillEdge::Hard,
        _ => fv::FillEdge::AntiAliased,
    }
}

/// Paint of a text whose fill is a gradient, pattern or web dither (Q-64, RE spec 62 §2.6): a path
/// without contours that carries the fill a path with the same fill attributes and the text's
/// `fill_handles` would get. The handles (and the texture offset) are in document coordinates and
/// go through `map` only, not through the text's own matrix. `None` for a solid fill, no fill and
/// the contour gradient (which needs the outline; the text then keeps its run colours).
pub fn text_paint(t: &TextElem, map: &PointMap) -> Option<fv::VectorPath> {
    let (fill, _) = paint(&t.attrs, map_handles(&t.fill_handles, map));
    let fill = fill?;
    match &fill.kind {
        fv::FillKind::Solid(_) => return None,
        fv::FillKind::Gradient(g) if g.shape == fv::GradientShape::Contour => return None,
        _ => {}
    }
    let mut v = fv::VectorPath::new(Vec::new());
    v.texture_offset = map.apply(Vec2::new(
        t.texture_offset.x as f64,
        t.texture_offset.y as f64,
    ));
    v.fill = Some(fill);
    Some(v)
}

/// Text (`TXTv`) → [`fp_text::TextBlock`] (ohne `MTXv`; die Abbildung wendet der Renderer auf das
/// Layout an).
pub fn text_block(_doc: &Document, t: &TextElem) -> fp_text::TextBlock {
    let mut initial = fp_text::CharAttrs {
        leading: 1.2,
        ..fp_text::CharAttrs::default()
    };
    char_patch(&t.initial, true).apply(&mut initial);
    let blocks: Vec<(fp_text::CharAttrsPatch, String)> = t
        .runs
        .iter()
        .map(|r| (char_patch(&r.attrs, false), r.text.clone()))
        .collect();
    let runs = fp_text::resolve_runs(&initial, &blocks);
    let r = t.anchor_rect.unwrap_or(t.bounds);
    let path = t.on_path.as_ref().and_then(|op| {
        let pe = op.path.as_ref()?;
        let fp_model::ElementKind::Path(p) = &pe.kind else {
            return None;
        };
        Some(fp_text::TextPath {
            mode: fp_text::PathTextMode::from_tami(op.mode.unwrap_or(0) as i64),
            offset: op.offset.unwrap_or(0.0) as f64,
            contours: bezier_contours(&p.contours)
                .into_iter()
                .map(|(c, _)| c)
                .collect(),
            reversed: false,
        })
    });
    // anti-aliasing (tone curve, oversampling) is on for every edge kind except Hard; feather and
    // dither post-process that coverage in the renderer (TW-203)
    let feti = t
        .attrs
        .fill
        .as_ref()
        .map(|_| i64::from(text_fill_edge(t) != fv::FillEdge::Hard));
    fp_text::TextBlock {
        kind: fp_text::FrameKind::from_tfti(t.frame_type.unwrap_or(0) as i64),
        bounds: Rect::new(r.left, r.top, r.right, r.bottom),
        runs,
        auto_kern: t.auto_kern,
        story_kerning: None,
        orientation: fp_text::Orientation::from_orii(t.orientation as i64),
        antialias: fp_text::AntiAlias::from_tags(
            t.antialias as i64,
            t.aa_sharpness as i64,
            t.aa_strength as i64,
            t.aa_oversample as i64,
            feti,
        ),
        path,
    }
}

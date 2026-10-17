//! Automatische Formen (Smart Shapes): Gruppen mit eingebettetem Skript, Steuerpunkten und
//! `customData`, die sich beim Ziehen der Steuerpunkte neu berechnen (13 §4.4, 03 §5 Stufe 4).
//!
//! Aufbau:
//! * [`shapes`]: die eingebauten Formen – **eigene** Skripte (clean room, nach Beobachtung des
//!   Verhaltens im Original geschrieben), in die exe eingebettet. Gespeicherte Formen tragen ihr
//!   Skript im Dokument (`ISSv/JSVv`); beim Bearbeiten läuft immer das eingebettete Skript, auch das
//!   von Formen aus fremden Dateien.
//! * Laufzeit (`prelude.js`): `smartShape`, `Path`/`Contour`/`ContourNode`, `ControlPoint`,
//!   `Register*Move` in einem eigenen, kleinen JS-Kontext je Vorgang. Rust gibt den Zustand als JSON
//!   hinein und liest ihn nach jedem Ereignis zurück.
//! * [`op`]: Vorgänge `list`, `insert`, `dragControlPoint`, `setParams`, `redraw`, `describe` –
//!   dieselben, die fp-engine als Befehle `autoShape.*` anbietet ([`crate::JsfHost`]).

pub mod shapes;

use boa_engine::{Context, Source};
use fp_engine::docops;
use fp_engine::{EngineError, Session};
use fp_geom::{Matrix3, Point};
use fp_model::element::{ControlPoint, SmartShape};
use fp_model::{Element, ElementId, ElementKind};
use serde_json::{Value, json};
use std::sync::Arc;

pub use shapes::{SHAPES, ShapeDef, find_shape};

const PRELUDE: &str = include_str!("prelude.js");
const HELPERS: &str = include_str!("helpers.js");

type R<T> = fp_engine::Result<T>;

fn fail(m: impl Into<String>) -> EngineError {
    EngineError::Failed(m.into())
}

// ------------------------------------------------------------------ JS-Kontext

/// Ein JS-Kontext für die Ereignisse eines Vorgangs (ein Skript, mehrere Ereignisse).
struct Engine {
    ctx: Context,
    script: String,
    /// Einmal übersetztes Skript (bei Übersetzungsfehler wird je Ereignis neu ausgewertet).
    compiled: Option<boa_engine::Script>,
}

impl Engine {
    fn new(script: &str) -> R<Engine> {
        let mut ctx = Context::default();
        ctx.runtime_limits_mut()
            .set_loop_iteration_limit(20_000_000);
        ctx.runtime_limits_mut().set_recursion_limit(1024);
        for (what, src) in [("prelude", PRELUDE), ("helpers", HELPERS)] {
            ctx.eval(Source::from_bytes(src)).map_err(|e| {
                fail(fp_i18n::tr!(
                    "err-script-autoshape-runtime",
                    part = what,
                    error = e.to_string()
                ))
            })?;
        }
        // Kompatibilität mit eingebetteten Skripten des Originals: dessen Programmname als Alias
        if let Some(n) = fp_color::install::original_app_name() {
            let _ = ctx.eval(Source::from_bytes(&format!("var {n} = fw;")));
        }
        let compiled = boa_engine::Script::parse(Source::from_bytes(script), None, &mut ctx).ok();
        Ok(Engine {
            ctx,
            script: script.to_string(),
            compiled,
        })
    }

    /// Führt ein Ereignis aus und liefert den neuen Zustand (`__dump()`).
    fn event(&mut self, state: &Value) -> R<Value> {
        let begin = format!("__begin({});", state);
        self.ctx
            .eval(Source::from_bytes(&begin))
            .map_err(|e| fail(fp_i18n::tr!("err-script-autoshape", error = e.to_string())))?;
        let r = match &self.compiled {
            Some(s) => s.evaluate(&mut self.ctx),
            None => self.ctx.eval(Source::from_bytes(&self.script)),
        };
        if let Err(e) = r {
            return Err(fail(fp_i18n::tr!(
                "err-script-autoshape-script",
                operation = state["operation"].as_str().unwrap_or("").to_string(),
                error = e.to_string()
            )));
        }
        let out = self
            .ctx
            .eval(Source::from_bytes("__dump()"))
            .map_err(|e| fail(fp_i18n::tr!("err-script-autoshape", error = e.to_string())))?;
        let text = out
            .to_string(&mut self.ctx)
            .map(|s| s.to_std_string_escaped())
            .map_err(|e| fail(e.to_string()))?;
        serde_json::from_str(&text)
            .map_err(|e| fail(fp_i18n::tr!("err-script-autoshape", error = e.to_string())))
    }
}

// ------------------------------------------------------------------ Modell ↔ JSON

fn pt(p: Point) -> Value {
    json!({"x": p.x, "y": p.y})
}

fn point_of(v: &Value) -> Option<Point> {
    fp_engine::command::parse_point(v)
}

fn mat_json(m: &Matrix3) -> Value {
    json!([
        m.m[0][0], m.m[0][1], m.m[0][2], m.m[1][0], m.m[1][1], m.m[1][2], m.m[2][0], m.m[2][1],
        m.m[2][2]
    ])
}

fn custom_json(cd: &fp_model::CustomData) -> Value {
    Value::Object(
        cd.entries
            .iter()
            .map(|e| (e.key.clone(), crate::imp::elem::data_to_json(&e.value)))
            .collect(),
    )
}

fn set_custom(cd: &mut fp_model::CustomData, v: &Value) {
    cd.entries.clear();
    if let Some(m) = v.as_object() {
        for (k, x) in m {
            let d = crate::imp::elem::json_to_data(x);
            match d {
                fp_model::DataValue::List(_) | fp_model::DataValue::Dict(_) => {
                    cd.set_structured(k, d)
                }
                fp_model::DataValue::Scalar(s) => cd.set_scalar(k, s),
                _ => {}
            }
        }
    }
}

fn path_json(e: &Element) -> Value {
    if let ElementKind::Text(t) = &e.kind {
        let color = t
            .runs
            .first()
            .and_then(|r| r.attrs.color)
            .or(t.initial.color)
            .map(fp_color::Argb)
            .unwrap_or(t.attrs.fill_color);
        return json!({
            "id": e.id.0, "type": "text", "name": e.common.name, "text": t.plain_text(),
            "font": t.initial.font.clone().unwrap_or_else(|| "Arial".into()), "size": t.initial.size.unwrap_or(12.0),
            "color": crate::imp::elem::hex(color), "x": t.bounds.left, "y": t.bounds.top + t.initial.size.unwrap_or(12.0),
            "visible": e.common.visible, "opacity": (e.common.opacity.0 as f64 * 100.0 / 255.0).round(),
        });
    }
    let ElementKind::Path(p) = &e.kind else {
        return json!({"id": e.id.0, "keep": true});
    };
    let contours: Vec<Value> = p
        .contours
        .iter()
        .map(|c| {
            json!({"isClosed": c.closed, "nodes": c.nodes.iter().map(|n| json!({
                "x": n.anchor.x, "y": n.anchor.y, "predX": n.pred.x, "predY": n.pred.y,
                "succX": n.succ.x, "succY": n.succ.y, "isCurvePoint": n.curve,
            })).collect::<Vec<_>>()})
        })
        .collect();
    json!({
        "id": e.id.0, "name": e.common.name, "pathAttributes": crate::imp::elem::path_attrs_json(e),
        "isEvenOddFill": p.even_odd, "opacity": (e.common.opacity.0 as f64 * 100.0 / 255.0).round(),
        "visible": e.common.visible, "customData": custom_json(&e.common.custom_data), "contours": contours,
    })
}

/// Zustand einer automatischen Form als JSON für die Laufzeit.
pub fn elem_json(e: &Element) -> Option<Value> {
    let ElementKind::Group(g) = &e.kind else {
        return None;
    };
    let s = g.smart_shape.as_ref()?;
    Some(json!({
        "name": e.common.name,
        "customData": custom_json(&e.common.custom_data),
        "transform": mat_json(&s.transform.unwrap_or(Matrix3::IDENTITY)),
        "elements": g.children.iter().map(|c| path_json(c)).collect::<Vec<_>>(),
        "controlPoints": s.control_points.as_ref().map(|cps| cps.iter().map(|c| json!({
            "x": c.position.x, "y": c.position.y, "name": c.name, "toolTip": c.tooltip,
            "toolTipTracksDrag": c.tooltip_tracks_drag, "visible": c.visible, "type": "default",
        })).collect::<Vec<_>>()).unwrap_or_default(),
        "events": s.events.clone().unwrap_or_default(),
    }))
}

fn f32_of(v: &Value, k: &str, d: f32) -> f32 {
    v.get(k)
        .and_then(Value::as_f64)
        .map(|x| x as f32)
        .filter(|x| x.is_finite())
        .unwrap_or(d)
}

/// #rrggbb[aa] → ARGB.
fn hex_argb(c: &str) -> Option<fp_color::Argb> {
    let h = c.trim_start_matches('#');
    let v = u32::from_str_radix(h, 16).ok()?;
    match h.len() {
        6 => Some(fp_color::Argb(0xff00_0000 | v)),
        8 => Some(fp_color::Argb(((v & 0xff) << 24) | (v >> 8))),
        _ => None,
    }
}

fn color_of(v: &Value, k: &str) -> Option<fp_color::Argb> {
    v.get(k)
        .and_then(Value::as_str)
        .and_then(crate::conv::norm_color)
        .and_then(|c| hex_argb(&c))
}

/// Pfadattribute aus dem JSON (`pathAttributes`) auf vorhandene Attribute anwenden.
fn apply_attrs(a: &mut fp_model::PathAttrs, pa: &Value) {
    if !pa.is_object() {
        return;
    }
    match pa.get("fill") {
        Some(Value::Null) => a.fill = None,
        Some(f) if f.is_object() && a.fill.is_none() => a.fill = Some(fp_model::Fill::solid()),
        _ => {}
    }
    if let Some(c) = color_of(pa, "fillColor") {
        a.fill_color = c;
    }
    match pa.get("brush") {
        Some(Value::Null) => a.brush = None,
        Some(b) if b.is_object() => {
            let d = b
                .get("diameter")
                .and_then(Value::as_f64)
                .unwrap_or(1.0)
                .round()
                .max(1.0) as i32;
            match &mut a.brush {
                Some(br) => br.diameter = d,
                None => a.brush = Some(fp_model::Brush::basic(d)),
            }
        }
        _ => {}
    }
    if let Some(c) = color_of(pa, "brushColor") {
        a.stroke_color = c;
    }
}

fn contours_from(v: &Value) -> Vec<fp_model::element::Contour> {
    v.get("contours")
        .and_then(Value::as_array)
        .map(|cs| {
            cs.iter()
                .map(|c| fp_model::element::Contour {
                    closed: c.get("isClosed").and_then(Value::as_bool).unwrap_or(false),
                    bsl: None,
                    nodes: c
                        .get("nodes")
                        .and_then(Value::as_array)
                        .map(|ns| {
                            ns.iter()
                                .map(|n| {
                                    let x = f32_of(n, "x", 0.0);
                                    let y = f32_of(n, "y", 0.0);
                                    fp_model::element::ContourNode {
                                        anchor: Point::new(x, y),
                                        pred: Point::new(
                                            f32_of(n, "predX", x),
                                            f32_of(n, "predY", y),
                                        ),
                                        succ: Point::new(
                                            f32_of(n, "succX", x),
                                            f32_of(n, "succY", y),
                                        ),
                                        curve: n
                                            .get("isCurvePoint")
                                            .and_then(Value::as_bool)
                                            .unwrap_or(false),
                                        random: None,
                                        segment_length: None,
                                        pressure: None,
                                        extra: Default::default(),
                                    }
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                    extra: Default::default(),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Vorgabe-Attribute neuer Pfade: wie der erste vorhandene Pfad, sonst Füll-/Strichfarbe der Sitzung.
fn template_attrs(session: &Session, base: Option<&Element>) -> fp_model::PathAttrs {
    if let Some(ElementKind::Group(g)) = base.map(|e| &e.kind) {
        for c in &g.children {
            if let ElementKind::Path(p) = &c.kind {
                return (*p.attrs).clone();
            }
        }
    }
    let mut a = fp_model::PathAttrs::solid_fill(session.tools.fill_color);
    a.stroke_color = session.tools.stroke_color;
    a.brush = Some(fp_model::Brush::basic(1));
    a
}

fn default_attrs_json(session: &Session, base: Option<&Element>) -> Value {
    let a = template_attrs(session, base);
    let hex = crate::imp::elem::hex;
    json!({
        "fill": a.fill.as_ref().map(|f| json!({"category": f.category, "name": f.name, "shape": "solid"})),
        "fillColor": hex(a.fill_color),
        "brush": a.brush.as_ref().map(|b| json!({"category": b.category, "name": b.name, "diameter": b.diameter})),
        "brushColor": hex(a.stroke_color),
    })
}

/// Baut das Gruppenelement aus dem Zustand nach dem letzten Ereignis.
fn build_group(
    doc: &mut fp_model::Document,
    session_tpl: fp_model::PathAttrs,
    base: Option<&Element>,
    shape_name: &str,
    script: &str,
    st: &Value,
    transform: Matrix3,
) -> Element {
    let old_children: Vec<Arc<Element>> = match base.map(|e| &e.kind) {
        Some(ElementKind::Group(g)) => g.children.clone(),
        _ => Vec::new(),
    };
    let mut children = Vec::new();
    for pj in st
        .get("elements")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let old = pj
            .get("id")
            .and_then(Value::as_u64)
            .and_then(|id| old_children.iter().find(|c| c.id.0 == id));
        if pj.get("keep").and_then(Value::as_bool) == Some(true) {
            if let Some(o) = old {
                children.push(Arc::clone(o));
            }
            continue;
        }
        if pj.get("type").and_then(Value::as_str) == Some("text") {
            let text = pj.get("text").and_then(Value::as_str).unwrap_or("");
            let font = pj.get("font").and_then(Value::as_str).unwrap_or("Arial");
            let size = f32_of(&pj, "size", 12.0).max(1.0);
            let color = color_of(&pj, "color").unwrap_or(fp_color::Argb(0xff00_0000));
            let anchor = Point::new(f32_of(&pj, "x", 0.0), f32_of(&pj, "y", 0.0));
            let mut t = Element::new_text(&mut doc.ids, text, font, size, color, anchor);
            if let Some(o) = old {
                t.id = o.id;
                t.common = o.common.clone();
            }
            if let Some(n) = pj.get("name") {
                t.common.name = n.as_str().map(String::from);
            }
            if let Some(v) = pj.get("visible").and_then(Value::as_bool) {
                t.common.visible = v;
            }
            children.push(Arc::new(t));
            continue;
        }
        let contours = contours_from(&pj);
        let mut e = match old {
            Some(o) if matches!(o.kind, ElementKind::Path(_)) => (**o).clone(),
            _ => Element::new_path(&mut doc.ids, Vec::new(), session_tpl.clone()),
        };
        e.cache = None;
        if let ElementKind::Path(p) = &mut e.kind {
            let mut a = (*p.attrs).clone();
            apply_attrs(&mut a, pj.get("pathAttributes").unwrap_or(&Value::Null));
            p.attrs = Arc::new(a);
            p.contours = contours;
            p.polyline = None;
            p.primitive = None;
            p.even_odd = pj
                .get("isEvenOddFill")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let b = fp_model::new::contour_bounds(&p.contours);
            p.fill_handles = fp_model::new::default_fill_handles(b);
            p.texture_offset = Point::new(b.left, b.top);
        }
        if let Some(n) = pj.get("name") {
            e.common.name = n.as_str().map(String::from);
        }
        if let Some(o) = pj.get("opacity").and_then(Value::as_f64) {
            e.common.opacity =
                fp_model::Opacity((o.clamp(0.0, 100.0) * 255.0 / 100.0).round() as u8);
        }
        if let Some(v) = pj.get("visible").and_then(Value::as_bool) {
            e.common.visible = v;
        }
        if let Some(cd) = pj
            .get("customData")
            .filter(|c| c.as_object().is_some_and(|m| !m.is_empty()))
        {
            set_custom(&mut e.common.custom_data, cd);
        }
        children.push(Arc::new(e));
    }
    let mut g = match base {
        Some(b) => {
            let mut g = b.clone();
            g.cache = None;
            if let ElementKind::Group(gr) = &mut g.kind {
                gr.children = children;
            }
            g
        }
        None => Element::new_group(&mut doc.ids, children),
    };
    set_custom(
        &mut g.common.custom_data,
        st.get("customData").unwrap_or(&Value::Null),
    );
    let cps: Vec<ControlPoint> = st
        .get("controlPoints")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .map(|c| ControlPoint {
                    position: Point::new(f32_of(c, "x", 0.0), f32_of(c, "y", 0.0)),
                    kind: 0,
                    name: c
                        .get("name")
                        .and_then(Value::as_str)
                        .filter(|s| !s.is_empty())
                        .map(String::from),
                    tooltip: c
                        .get("toolTip")
                        .and_then(Value::as_str)
                        .filter(|s| !s.is_empty())
                        .map(String::from),
                    tooltip_tracks_drag: c
                        .get("toolTipTracksDrag")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    visible: c.get("visible").and_then(Value::as_bool).unwrap_or(true),
                    extra: Default::default(),
                })
                .collect()
        })
        .unwrap_or_default();
    let events: Vec<String> = st
        .get("events")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    if let ElementKind::Group(gr) = &mut g.kind {
        let mut s = gr.smart_shape.take().unwrap_or_else(|| SmartShape {
            name: shape_name.to_string(),
            script_chunks: Vec::new(),
            script_extra: Default::default(),
            transform: Some(transform),
            control_points: None,
            control_points_extra: Default::default(),
            events: None,
            events_extra: Default::default(),
            extra: Default::default(),
        });
        if s.script_chunks.is_empty() {
            s.set_script(script);
        }
        s.control_points = Some(cps);
        s.events = if events.is_empty() {
            None
        } else {
            Some(events)
        };
        gr.smart_shape = Some(s);
    }
    g
}

// ------------------------------------------------------------------ Register*Move

fn parm(p: &Value, k: &str, d: f64) -> f64 {
    p.get(k)
        .and_then(Value::as_f64)
        .filter(|x| x.is_finite())
        .unwrap_or(d)
}

fn snap(v: f64, inc: f64) -> f64 {
    if inc > 0.0 {
        (v / inc).round() * inc
    } else {
        v
    }
}

/// Verschiebung eines registrierten Punkts für die Mausbewegung `d` (relativ zum Druckpunkt).
fn moved(
    kind: &str,
    base: (f64, f64),
    other: Option<(f64, f64)>,
    p: &Value,
    d: (f64, f64),
    mouse: (f64, f64),
    down: (f64, f64),
) -> (f64, f64) {
    match kind {
        "linear" => {
            let o = other.unwrap_or((0.0, 0.0));
            let (vx, vy) = (base.0 - o.0, base.1 - o.1);
            let len = (vx * vx + vy * vy).sqrt();
            if len < 1e-9 {
                return base;
            }
            let (ux, uy) = (vx / len, vy / len);
            let t = (d.0 * ux + d.1 * uy) * parm(p, "deltaLinearToLinear", 1.0);
            let nl = (len + t).clamp(parm(p, "minLinear", -1e9), parm(p, "maxLinear", 1e9));
            (o.0 + ux * nl, o.1 + uy * nl)
        }
        "circular" | "polygon" => {
            let c = other.unwrap_or((0.0, 0.0));
            let r0 = ((base.0 - c.0).powi(2) + (base.1 - c.1).powi(2)).sqrt();
            let rm = ((mouse.0 - c.0).powi(2) + (mouse.1 - c.1).powi(2)).sqrt();
            let rd = ((down.0 - c.0).powi(2) + (down.1 - c.1).powi(2)).sqrt();
            let r = snap(
                r0 + (rm - rd) * parm(p, "deltaRtoR", 1.0),
                parm(p, "incrementRadius", 0.0),
            )
            .clamp(parm(p, "minRadius", 0.0), parm(p, "maxRadius", 1e9));
            let mut a = (base.1 - c.1).atan2(base.0 - c.0);
            if p.get("rotate").and_then(Value::as_bool) == Some(true) {
                a += (mouse.1 - c.1).atan2(mouse.0 - c.0) - (down.1 - c.1).atan2(down.0 - c.0);
            }
            (c.0 + r * a.cos(), c.1 + r * a.sin())
        }
        _ => {
            let nx = snap(
                parm(p, "deltaXtoX", 1.0) * d.0 + parm(p, "deltaYtoX", 0.0) * d.1,
                parm(p, "incrementX", 0.0),
            );
            let ny = snap(
                parm(p, "deltaXtoY", 0.0) * d.0 + parm(p, "deltaYtoY", 1.0) * d.1,
                parm(p, "incrementY", 0.0),
            );
            let rel = p.get("minMaxRelative").and_then(Value::as_bool) == Some(true);
            let (bx, by) = if rel { base } else { (0.0, 0.0) };
            let x = (base.0 + nx).clamp(bx + parm(p, "minX", -1e9), bx + parm(p, "maxX", 1e9));
            let y = (base.1 + ny).clamp(by + parm(p, "minY", -1e9), by + parm(p, "maxY", 1e9));
            (x, y)
        }
    }
}

/// Wendet die registrierten Bewegungen auf den Zustand `base` an (für Skripte ohne Ziehereignisse).
fn apply_moves(base: &Value, mouse: Point, down: Point) -> Value {
    let mut st = base.clone();
    let d = ((mouse.x - down.x) as f64, (mouse.y - down.y) as f64);
    let (m, dn) = (
        (mouse.x as f64, mouse.y as f64),
        (down.x as f64, down.y as f64),
    );
    for mv in base
        .get("moves")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let kind = mv["kind"].as_str().unwrap_or("move").to_string();
        if kind == "bbox" {
            continue;
        }
        let p = &mv["parms"];
        let other = mv
            .get("other")
            .and_then(|o| Some((o.get("x")?.as_f64()?, o.get("y")?.as_f64()?)));
        let t = &mv["target"];
        if let Some(i) = t.get("cp").and_then(Value::as_u64) {
            let b = &base["controlPoints"][i as usize];
            let (bx, by) = (
                b["x"].as_f64().unwrap_or(0.0),
                b["y"].as_f64().unwrap_or(0.0),
            );
            let (x, y) = moved(&kind, (bx, by), other, p, d, m, dn);
            st["controlPoints"][i as usize]["x"] = json!(x);
            st["controlPoints"][i as usize]["y"] = json!(y);
        } else if let (Some(pi), Some(ci), Some(ni)) = (
            t.get("path").and_then(Value::as_u64),
            t.get("contour").and_then(Value::as_u64),
            t.get("node").and_then(Value::as_u64),
        ) {
            let b = &base["elements"][pi as usize]["contours"][ci as usize]["nodes"][ni as usize];
            let (bx, by) = (
                b["x"].as_f64().unwrap_or(0.0),
                b["y"].as_f64().unwrap_or(0.0),
            );
            let (x, y) = moved(&kind, (bx, by), other, p, d, m, dn);
            let (dx, dy) = (x - bx, y - by);
            let n = &mut st["elements"][pi as usize]["contours"][ci as usize]["nodes"][ni as usize];
            if p.get("movePt").and_then(Value::as_bool) != Some(false) {
                n["x"] = json!(x);
                n["y"] = json!(y);
            }
            for (on, kx, ky) in [
                ("movePred", "predX", "predY"),
                ("moveSucc", "succX", "succY"),
            ] {
                if p.get(on).and_then(Value::as_bool) != Some(false) {
                    n[kx] = json!(b[kx].as_f64().unwrap_or(bx) + dx);
                    n[ky] = json!(b[ky].as_f64().unwrap_or(by) + dy);
                }
            }
        }
    }
    st
}

// ------------------------------------------------------------------ Vorgänge

thread_local! {
    static DOC_INFO: std::cell::RefCell<Value> = const { std::cell::RefCell::new(Value::Null) };
}

/// Ereignis-Zustand für die Laufzeit.
#[allow(clippy::too_many_arguments)]
fn state(
    operation: &str,
    elem: &Value,
    down: Point,
    prev: Point,
    cur: Point,
    cp: i64,
    mods: (bool, bool, bool),
    gets_drag: bool,
    defaults: &Value,
    doc_size: (f32, f32),
) -> Value {
    json!({
        "operation": operation, "elem": elem, "mouseDownPos": pt(down), "prevMousePos": pt(prev), "currentMousePos": pt(cur),
        "cpIndex": cp, "shift": mods.0, "alt": mods.1, "ctrl": mods.2, "getsDragEvents": gets_drag,
        "defaultAttrs": defaults, "docWidth": doc_size.0, "docHeight": doc_size.1, "doc": DOC_INFO.with(|d| d.borrow().clone()),
    })
}

/// Übernimmt den Zustand nach einem Ereignis als neues `elem` (Laufzeitfelder entfernt).
fn as_elem(dump: &Value, prev_elem: &Value) -> Value {
    let mut e = prev_elem.clone();
    for k in [
        "customData",
        "elements",
        "controlPoints",
        "events",
        "transformMode",
    ] {
        if let Some(v) = dump.get(k) {
            e[k] = v.clone();
        }
    }
    e
}

fn mods_of(p: &Value) -> (bool, bool, bool) {
    let b = |k: &str| p.get(k).and_then(Value::as_bool).unwrap_or(false);
    (b("shift"), b("alt"), b("ctrl"))
}

fn doc_size(session: &Session) -> (f32, f32) {
    session
        .active()
        .and_then(|d| {
            d.doc
                .pages
                .get(d.page)
                .map(|p| (p.canvas.width, p.canvas.height))
        })
        .unwrap_or((0.0, 0.0))
}

/// Angaben zum Dokument für Formen wie „Dateiinfo“.
fn doc_info(session: &Session) -> Value {
    match session.active() {
        Some(d) => {
            let p = d.doc.pages.get(d.page);
            json!({
                "title": d.title, "pages": d.doc.pages.len(), "page": d.page,
                "layers": p.map(|p| p.root.layers.iter().filter(|l| matches!(l, fp_model::LayerSlot::Own(l) if l.kind == fp_model::LayerKind::Normal)).count()).unwrap_or(0),
                "frames": p.map(|p| p.root.frame_count.max(1)).unwrap_or(1), "resolution": p.map(|p| p.canvas.resolution).unwrap_or(72.0),
            })
        }
        None => json!({}),
    }
}

/// Liste aller eingebauten Formen.
pub fn list() -> Value {
    Value::Array(
        SHAPES
            .iter()
            .map(|s| json!({"name": s.en, "label": s.de, "group": s.group, "params": s.params.iter().map(|p| json!({"key": p.key, "label": p.label, "labelKey": p.label_key(), "type": p.kind})).collect::<Vec<_>>()}))
            .collect(),
    )
}

fn shape_of(session: &Session, id: ElementId) -> R<(Element, usize)> {
    let d = session.doc()?;
    let e =
        docops::element_on_page(&d.doc, d.page, id).ok_or(EngineError::ElementNotFound(id.0))?;
    match &e.kind {
        ElementKind::Group(g) if g.smart_shape.is_some() => Ok(((*e).clone(), d.page)),
        _ => Err(EngineError::Selection(fp_i18n::tr!(
            "err-script-element-not-autoshape",
            id = id.0.to_string()
        ))),
    }
}

fn replace_shape(session: &mut Session, page: usize, new: Element) -> R<()> {
    let id = new.id;
    let d = session.doc_mut()?;
    let doc = d.doc_mut();
    docops::edit_by_id(doc, page, id, |e| *e = new).ok_or(EngineError::ElementNotFound(id.0))?;
    Ok(())
}

/// Start- und Endpunkt des Zugs zu einem aufgezogenen Rahmen: `from` (Startpunkt des Zugs) wählt
/// die Startecke, damit die Formskripte die Zugrichtung kennen (runde Formen liegen im Original in
/// Zugrichtung ab dem Startpunkt). Ohne `from` oder bei einem Start im Rahmen (Alt = von der Mitte)
/// links oben → rechts unten.
fn drag_corners(r: fp_geom::Rect, from: Option<Point>) -> (Point, Point) {
    let pick = |f: f32, lo: f32, hi: f32| {
        if (f - hi).abs() < (f - lo).abs() {
            (hi, lo)
        } else {
            (lo, hi)
        }
    };
    match from {
        Some(f)
            if !(f.x > r.left + 0.5 && f.x < r.right - 0.5)
                && !(f.y > r.top + 0.5 && f.y < r.bottom - 0.5) =>
        {
            let (x0, x1) = pick(f.x, r.left, r.right);
            let (y0, y1) = pick(f.y, r.top, r.bottom);
            (Point::new(x0, y0), Point::new(x1, y1))
        }
        _ => (Point::new(r.left, r.top), Point::new(r.right, r.bottom)),
    }
}

/// Fügt eine Form ein (`rect` = aufgezogener Rahmen, sonst `at`). Liefert `{id}` und wählt sie aus.
pub fn insert(session: &mut Session, p: &Value) -> R<Value> {
    let name = p
        .get("shape")
        .or_else(|| p.get("name"))
        .and_then(Value::as_str)
        .ok_or_else(|| fail(fp_i18n::t("err-script-shape-missing")))?;
    let def = find_shape(name)
        .ok_or_else(|| fail(fp_i18n::tr!("err-script-shape-unknown", name = name)))?;
    let rect = p
        .get("rect")
        .and_then(fp_engine::command::parse_rect)
        .filter(|r| r.width() >= 2.0 || r.height() >= 2.0);
    let at = p
        .get("at")
        .and_then(point_of)
        .or_else(|| rect.map(|r| Point::new(r.left, r.top)))
        .unwrap_or(Point::new(0.0, 0.0));
    let mods = mods_of(p);
    let defaults = default_attrs_json(session, None);
    let size = doc_size(session);
    let mut eng = Engine::new(def.source)?;
    let mut elem = json!({"elements": [], "controlPoints": [], "customData": {}, "transform": mat_json(&Matrix3::IDENTITY)});
    match rect {
        Some(r) => {
            let (a, b) = drag_corners(r, p.get("from").and_then(point_of));
            let mut prev = a;
            for (op, cur) in [
                ("BeginDragInsert", a),
                ("DragInsert", b),
                ("EndDragInsert", b),
            ] {
                let out = eng.event(&state(
                    op, &elem, a, prev, cur, -1, mods, false, &defaults, size,
                ))?;
                elem = as_elem(&out, &elem);
                prev = cur;
            }
        }
        None => {
            let out = eng.event(&state(
                "InsertSmartShapeAt",
                &elem,
                at,
                at,
                at,
                -1,
                mods,
                false,
                &defaults,
                size,
            ))?;
            elem = as_elem(&out, &elem);
        }
    }
    if std::env::var_os("FP_AUTOSHAPE_DEBUG").is_some() {
        eprintln!("autoshape insert: {elem}");
    }
    let tpl = template_attrs(session, None);
    let d = session.doc_mut()?;
    let (page, frame, layer) = (d.page, d.frame, d.layer);
    let doc = d.doc_mut();
    let g = build_group(doc, tpl, None, def.en, def.source, &elem, Matrix3::IDENTITY);
    let id = g.id;
    docops::insert_element(doc, page, frame, layer, 0, Arc::new(g))
        .ok_or_else(|| fail(fp_i18n::t("err-script-no-layer")))?;
    d.selection.elements = vec![id];
    Ok(json!({"id": id.0, "shape": def.en}))
}

/// Steuerpunkt `index` von `from` nach `to` ziehen (optional über Zwischenpunkte `points`).
pub fn drag_control_point(session: &mut Session, p: &Value) -> R<Value> {
    let id = ElementId(
        p.get("id")
            .and_then(Value::as_u64)
            .ok_or_else(|| fail(fp_i18n::tr!("err-script-param-missing", param = "id")))?,
    );
    let index = p
        .get("index")
        .and_then(Value::as_i64)
        .ok_or_else(|| fail(fp_i18n::tr!("err-script-param-missing", param = "index")))?;
    let (e, page) = shape_of(session, id)?;
    let mut elem = elem_json(&e).ok_or_else(|| fail(fp_i18n::t("err-script-not-autoshape")))?;
    let ncp = elem["controlPoints"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0) as i64;
    if index < 0 || index >= ncp {
        return Err(fail(fp_i18n::tr!(
            "err-script-control-point",
            index = index,
            count = ncp
        )));
    }
    let cp = &elem["controlPoints"][index as usize];
    let cp_pos = Point::new(
        cp["x"].as_f64().unwrap_or(0.0) as f32,
        cp["y"].as_f64().unwrap_or(0.0) as f32,
    );
    let from = p.get("from").and_then(point_of).unwrap_or(cp_pos);
    let to = p
        .get("to")
        .and_then(point_of)
        .ok_or_else(|| fail(fp_i18n::tr!("err-script-param-missing", param = "to")))?;
    let mut path: Vec<Point> = p
        .get("points")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(point_of).collect())
        .unwrap_or_default();
    // höchstens 24 Zwischenschritte (das Ergebnis hängt nur vom letzten Punkt ab, die Zwischen-
    // schritte sind für Skripte da, die Mausbewegungen mitzählen)
    if path.len() > 24 {
        let step = path.len().div_ceil(24);
        path = path.iter().step_by(step).copied().collect();
    }
    path.push(to);
    let mods = mods_of(p);
    let defaults = default_attrs_json(session, Some(&e));
    let size = doc_size(session);
    let script = match &e.kind {
        ElementKind::Group(g) => g
            .smart_shape
            .as_ref()
            .map(|s| s.script())
            .unwrap_or_default(),
        _ => String::new(),
    };
    let mut eng = Engine::new(&script)?;
    let begin = eng.event(&state(
        "BeginDragControlPoint",
        &elem,
        from,
        from,
        from,
        index,
        mods,
        false,
        &defaults,
        size,
    ))?;
    elem = as_elem(&begin, &elem);
    let gets = begin
        .get("getsDragEvents")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut prev = from;
    for q in &path {
        if gets {
            let out = eng.event(&state(
                "DragControlPoint",
                &elem,
                from,
                prev,
                *q,
                index,
                mods,
                true,
                &defaults,
                size,
            ))?;
            elem = as_elem(&out, &elem);
        } else {
            let moved = apply_moves(&begin, *q, from);
            elem = as_elem(&moved, &elem);
        }
        prev = *q;
    }
    let end = eng.event(&state(
        "EndDragControlPoint",
        &elem,
        from,
        prev,
        to,
        index,
        mods,
        gets,
        &defaults,
        size,
    ))?;
    elem = as_elem(&end, &elem);
    if std::env::var_os("FP_AUTOSHAPE_DEBUG").is_some() {
        eprintln!("autoshape drag end elements: {}", elem["elements"]);
    }
    finish(session, &e, page, &elem)?;
    Ok(json!({"id": id.0}))
}

fn finish(session: &mut Session, e: &Element, page: usize, elem: &Value) -> R<()> {
    let tpl = template_attrs(session, Some(e));
    let (name, script, xf) = match &e.kind {
        ElementKind::Group(g) => g
            .smart_shape
            .as_ref()
            .map(|s| {
                (
                    s.name.clone(),
                    s.script(),
                    s.transform.unwrap_or(Matrix3::IDENTITY),
                )
            })
            .unwrap_or_default(),
        _ => Default::default(),
    };
    let d = session.doc_mut()?;
    let doc = d.doc_mut();
    let g = build_group(doc, tpl, Some(e), &name, &script, elem, xf);
    replace_shape(session, page, g)
}

/// Ereignis ohne Maus (Neuzeichnen nach Parameteränderung).
fn redraw_with(session: &mut Session, id: ElementId, change: impl FnOnce(&mut Value)) -> R<Value> {
    let (e, page) = shape_of(session, id)?;
    let mut elem = elem_json(&e).ok_or_else(|| fail(fp_i18n::t("err-script-not-autoshape")))?;
    change(&mut elem);
    let defaults = default_attrs_json(session, Some(&e));
    let size = doc_size(session);
    let script = match &e.kind {
        ElementKind::Group(g) => g
            .smart_shape
            .as_ref()
            .map(|s| s.script())
            .unwrap_or_default(),
        _ => String::new(),
    };
    let mut eng = Engine::new(&script)?;
    let z = Point::new(0.0, 0.0);
    let out = eng.event(&state(
        "RedrawSmartShape",
        &elem,
        z,
        z,
        z,
        -1,
        (false, false, false),
        false,
        &defaults,
        size,
    ))?;
    elem = as_elem(&out, &elem);
    finish(session, &e, page, &elem)?;
    Ok(json!({"id": id.0}))
}

/// Parameter (customData) setzen und neu zeichnen.
pub fn set_params(session: &mut Session, p: &Value) -> R<Value> {
    let id = ElementId(
        p.get("id")
            .and_then(Value::as_u64)
            .ok_or_else(|| fail(fp_i18n::tr!("err-script-param-missing", param = "id")))?,
    );
    let params = p
        .get("params")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| fail(fp_i18n::t("err-script-params-missing")))?;
    redraw_with(session, id, |elem| {
        for (k, v) in params {
            elem["customData"][k] = v;
        }
    })
}

/// Beschreibung für Inspektor/Bedienfeld: Parameter mit Werten und Steuerpunkte.
pub fn describe(session: &Session, p: &Value) -> R<Value> {
    let id = ElementId(
        p.get("id")
            .and_then(Value::as_u64)
            .ok_or_else(|| fail(fp_i18n::tr!("err-script-param-missing", param = "id")))?,
    );
    let (e, _) = shape_of(session, id)?;
    let elem = elem_json(&e).unwrap_or(Value::Null);
    let name = match &e.kind {
        ElementKind::Group(g) => g
            .smart_shape
            .as_ref()
            .map(|s| s.name.clone())
            .unwrap_or_default(),
        _ => String::new(),
    };
    let cd = elem["customData"].clone();
    let params: Vec<Value> = match find_shape(&name) {
        Some(def) => def
            .params
            .iter()
            .map(|pd| {
                let mut v = json!({"key": pd.key, "label": pd.label, "labelKey": pd.label_key(), "type": pd.kind, "value": cd.get(pd.key).cloned().unwrap_or(Value::Null)});
                if let Some(m) = pd.min {
                    v["min"] = json!(m);
                }
                if let Some(m) = pd.max {
                    v["max"] = json!(m);
                }
                if !pd.options.is_empty() {
                    v["options"] = json!(pd.options);
                    v["optionKeys"] = json!(pd.options.iter().map(|o| shapes::option_key(o)).collect::<Vec<_>>());
                }
                v
            })
            .collect(),
        None => cd
            .as_object()
            .map(|m| {
                m.iter()
                    .filter(|(k, v)| !k.starts_with('_') && *k != "shapeName" && (v.is_number() || v.is_boolean() || v.is_string()))
                    .map(|(k, v)| json!({"key": k, "label": k, "type": if v.is_number() { "number" } else if v.is_boolean() { "bool" } else { "text" }, "value": v}))
                    .collect()
            })
            .unwrap_or_default(),
    };
    let label = find_shape(&name)
        .map(|d| d.de)
        .unwrap_or(name.as_str())
        .to_string();
    Ok(
        json!({"id": id.0, "shape": name, "label": label, "params": params, "controlPoints": elem["controlPoints"], "customData": cd}),
    )
}

/// Führt einen Vorgang aus (`op`: list, insert, dragControlPoint, setParams, redraw, describe).
pub fn op(session: &mut Session, p: &Value) -> R<Value> {
    let info = doc_info(session);
    DOC_INFO.with(|d| *d.borrow_mut() = info);
    match p.get("op").and_then(Value::as_str).unwrap_or("") {
        "list" => Ok(list()),
        "insert" => insert(session, p),
        "dragControlPoint" => drag_control_point(session, p),
        "setParams" => set_params(session, p),
        "redraw" => {
            let id = ElementId(
                p.get("id")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| fail(fp_i18n::tr!("err-script-param-missing", param = "id")))?,
            );
            redraw_with(session, id, |_| {})
        }
        "describe" => describe(session, p),
        other => Err(fail(fp_i18n::tr!(
            "err-script-op-unknown",
            op = other,
            ops = "list, insert, dragControlPoint, setParams, redraw, describe"
        ))),
    }
}

/// Wie [`op`], aber als **ein** Verlaufsschritt (für Aufrufer außerhalb von `Session::execute`).
pub fn op_with_history(session: &mut Session, p: &Value) -> R<Value> {
    let kind = p.get("op").and_then(Value::as_str).unwrap_or("");
    if matches!(kind, "list" | "describe") {
        return op(session, p);
    }
    let label = match kind {
        "insert" => "Automatische Form",
        "dragControlPoint" => "Steuerpunkt ziehen",
        _ => "Automatische Form bearbeiten",
    };
    let mut out = Value::Null;
    session.run_group(label, |s| {
        out = op(s, p)?;
        Ok(())
    })?;
    Ok(out)
}

//! Elemente (Path, Text, Image, Group, Instance, RectanglePrimitive, Hotspot, SliceHotspot,
//! CompoundShape), Ebenen, Status, Ebeneninhalte, Konturen, Knoten und lebende Listen.

use super::{element, exec, ni};
use crate::bindings::hr;
use crate::conv::{self, Arg};
use crate::runtime;
use crate::state::{st, with_state, FwErr, FwResult, Handle};
use boa_engine::{Context, JsValue};
use fp_engine::DocId;
use fp_model::element::ElementKind;
use fp_model::{DataValue, Element, ElementId, LayerSlot, TypedScalar};
use serde_json::{json, Map, Value};
use std::sync::Arc;

fn jserr(e: boa_engine::JsError) -> FwErr {
    FwErr::Js(e.to_string())
}

/// Wrapper eines Elements (Klasse nach Elementart).
pub(crate) fn wrap_element(ctx: &mut Context, doc: DocId, e: &Element) -> FwResult<JsValue> {
    runtime::wrap(ctx, e.js_type(), Handle::Element { doc, id: e.id })
}

fn r6(v: f64) -> f64 {
    (v * 1e6).round() / 1e6
}

/// Element mit dem Zustand seiner Seite (für Befehle mit `ids`).
fn elem_handle(h: &Handle) -> FwResult<(DocId, ElementId)> {
    match h {
        Handle::Element { doc, id } => Ok((*doc, *id)),
        _ => Err(FwErr::bad("kein Element")),
    }
}

pub(crate) fn hex(c: fp_color::Argb) -> String {
    fp_engine::commands::argb_hex(c)
}

fn hex_rgb(c: fp_color::Argb) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())
}

/// Grenzen eines Elements.
fn bounds(doc: DocId, e: &Element) -> FwResult<[f64; 4]> {
    st(|s| {
        let d = s.session.document(doc).ok_or_else(|| FwErr::bad("Dokument"))?;
        Ok(fp_engine::docops::element_bounds(&d.doc, e).map(|b| [b.left as f64, b.top as f64, b.right as f64, b.bottom as f64]).unwrap_or([0.0; 4]))
    })
}

/// `RectanglePrimitive.originalSides` wie im Original: Seiten in absoluten Dokumentkoordinaten
/// (R1 → {50, 50, 150, 120}, nach dem Ziehen eines Eckgriffs die neuen Seiten). Gespeichert ist das
/// Rechteck lokal um die Mitte samt Abbildung (`RCTv` + `MTXv`): Ohne Drehung/Scherung sind die Seiten
/// das abgebildete Rechteck; mit Drehung/Scherung (Frei transformieren) bleibt das ungedrehte Rechteck
/// in seiner Größe um den abgebildeten Mittelpunkt (die Transformation steht separat in `transform`).
pub(super) fn original_sides(r: &fp_model::element::RectPrimitive) -> fp_geom::Rect {
    let m = r.transform;
    let axis_aligned = m.m[0][1].abs() < 1e-6 && m.m[1][0].abs() < 1e-6;
    if axis_aligned {
        return m.transform_rect_bounds(&r.rect);
    }
    let c = m.apply(fp_geom::Point::new((r.rect.left + r.rect.right) / 2.0, (r.rect.top + r.rect.bottom) / 2.0));
    let (hw, hh) = ((r.rect.right - r.rect.left) / 2.0, (r.rect.bottom - r.rect.top) / 2.0);
    fp_geom::Rect::new(c.x - hw, c.y - hh, c.x + hw, c.y + hh)
}

/// Element vorübergehend auswählen und einen Auswahl-Befehl ausführen (Auswahl danach zurück).
fn with_selected(doc: DocId, ids: &[ElementId], name: &str, params: Value) -> FwResult<()> {
    st(|s| {
        let prev = s.session.document(doc).map(|d| d.selection.elements.clone()).unwrap_or_default();
        let idv: Vec<u64> = ids.iter().map(|i| i.0).collect();
        s.exec_on(Some(doc), "select.set", json!({"ids": idv}))?;
        let r = s.exec_on(Some(doc), name, params);
        let pv: Vec<u64> = prev.iter().map(|i| i.0).collect();
        let _ = if pv.is_empty() { s.exec_on(Some(doc), "select.none", json!({})) } else { s.exec_on(Some(doc), "select.set", json!({"ids": pv})) };
        r.map(|_| ())
    })
}

// ------------------------------------------------------------------ Snapshots

fn fill_json(f: &fp_model::Fill) -> Value {
    let shape = match f.category.as_str() {
        "fc_Solid" => "solid",
        "fc_Linear" => "linear",
        "fc_Radial" => "radial",
        "fc_Conical" => "conical",
        "fc_Satin" => "satin",
        "fc_Starburst" | "fc_Pinch" => "starburst",
        "fc_Bars" => "bars",
        "fc_Ripples" | "fc_Ripple" => "ripples",
        "fc_Waves" => "waves",
        "fc_Elliptical" => "elliptical",
        "fc_Rectangular" => "rectangular",
        "fc_Folds" => "folds",
        "fc_ContourGradient" | "fc_ContourGrad" => "contour",
        "fc_Pattern" => "pattern",
        "fc_WebDither" => "web dither",
        _ => "solid",
    };
    // wie das Original: eine weiche Kante meldet „antialiased“ mit `feather` > 0 (PAR 18/11)
    let edge = match f.edge {
        0 => "hard",
        _ => "antialiased",
    };
    let gradient = f.gradient.as_ref().map(|g| {
        json!({
            "name": g.name, "dither": g.stops.as_ref().and_then(|s| s.dither).unwrap_or(false),
            "nodes": g.color_stops().iter().map(|s| json!({"color": hex(s.color), "position": r6(s.position as f64), "isOpacityNode": false})).collect::<Vec<_>>(),
            "opacityNodes": g.opacity_stops().iter().map(|s| json!({"color": hex(s.color), "position": r6(s.position as f64), "isOpacityNode": true})).collect::<Vec<_>>(),
        })
    });
    json!({
        "category": f.category, "name": f.name, "shape": shape, "edgeType": edge, "feather": f.feather,
        "textureBlend": f.texture_amount, "stampingMode": if f.stamping_mode == 0 { "blend" } else { "blend opaque" },
        "webDitherTransparent": f.transparent_dither.unwrap_or(false),
        "ditherColors": f.dither_colors.map(|c| c.iter().map(|x| hex(fp_color::Argb(*x))).collect::<Vec<_>>()).unwrap_or_else(|| vec!["#000000".into(), "#000000".into()]),
        "gradient": gradient, "pattern": f.pattern.as_ref().map(|p| json!({"name": p.name, "image": p.user_name.clone().unwrap_or_else(|| p.name.clone())})),
    })
}

pub(crate) fn brush_json(b: &fp_model::Brush) -> Value {
    let pick = |list: &[&'static str], i: i32| list.get(i.max(0) as usize).copied().unwrap_or(list[0]);
    let alpha = pick(&["none", "white neon", "black neon", "edge", "neon edge", "white neon edge"], b.alpha_remap);
    let feedback = pick(&["none", "brush", "background"], b.feedback);
    let tsm = pick(&["random", "diagonal", "circular"], b.tip_spacing_mode);
    let tcm = pick(&["random", "uniform", "complementary", "hue", "shadow"], b.tip_coloring_mode);
    let mut v = json!({
        "category": b.category, "name": b.name, "diameter": b.diameter, "angle": b.angle, "aspect": b.aspect,
        "softness": b.softness, "softenMode": if b.soften_mode == 1 { "bell curve" } else { "linear" }, "spacing": b.spacing,
        "tipCount": b.tip_count, "flowRate": b.flow_rate, "blackness": b.blackness, "concentration": b.concentration,
        "maxCount": b.max_count, "minSize": b.min_size, "alphaRemap": alpha,
        "shape": if b.shape == 0 { "square" } else { "circle" }, "type": if b.brush_type == 0 { "natural" } else { "simple" },
        "feedback": feedback,
        "antiAliased": b.antialiased, "textureBlend": b.texture_blend, "textureEdge": b.texture_edge,
        "tipSpacing": b.tip_spacing, "tipSpacingMode": tsm,
        "tipColoringMode": tcm,
        "numDashes": b.dash_count,
        "dashOnSize1": b.dash_on[0], "dashOffSize1": b.dash_off[0], "dashOnSize2": b.dash_on[1], "dashOffSize2": b.dash_off[1],
        "dashOnSize3": b.dash_on[2], "dashOffSize3": b.dash_off[2],
    });
    // Empfindlichkeiten sense_<Quelle>_<Ziel> (Quelle P/S/H/V/R, Ziel Z/A/O/B/R/H/L/S)
    let src = [(b'P', "pressure"), (b'S', "speed"), (b'H', "hdir"), (b'V', "vdir"), (b'R', "random")];
    let tgt = [(b'Z', "size"), (b'A', "angle"), (b'O', "opacity"), (b'B', "blackness"), (b'R', "scatter"), (b'H', "hue"), (b'L', "lightness"), (b'S', "saturation")];
    for (sc, sn) in src {
        for (tc, tn) in tgt {
            let val = b.sensitivities.iter().find(|s| s.source == sc && s.target == tc).map(|s| s.value).unwrap_or(0);
            v[format!("sense_{sn}_{tn}")] = json!(val);
        }
    }
    v
}

/// Stil wie das Style-Objekt des Originals.
pub(crate) fn style_json(s: &fp_model::Style) -> Value {
    let size = s.text.size.map(|z| format!("{}pt", conv::fmt_num(z as f64)));
    json!({
        "name": s.name,
        "pathAttributes": s.attrs.as_ref().map(|a| json!({
            "brush": a.brush.as_ref().map(brush_json), "fill": a.fill.as_ref().map(fill_json),
            "brushColor": hex(a.stroke_color), "fillColor": hex(a.fill_color),
        })),
        "effectList": s.effects.as_ref().map(|l| json!({"category": l.category, "name": l.name, "effects": l.effects.iter().map(|f| json!({"EffectMoaID": f.moa_id().unwrap_or(""), "EffectIsVisible": f.is_visible()})).collect::<Vec<_>>()})),
        "textSize": size, "textFont": s.text.font, "textBold": s.text.bold.unwrap_or(false), "textItalic": s.text.italic.unwrap_or(false),
        "textUnderline": s.text.underline.unwrap_or(false),
        "use_brush": s.use_brush(), "use_fill": s.use_fill(), "use_brushColor": s.use_brush_color(), "use_fillColor": s.use_fill_color(),
        "use_effectList": s.use_effects(), "use_textSize": s.use_text_size(), "use_textFont": s.use_text_font(), "use_textStyles": s.use_text_styles(),
    })
}

pub(crate) fn path_attrs_json(e: &Element) -> Value {
    let Some(a) = fp_engine::docops::path_attrs_of(e) else { return Value::Null };
    let (placement, fill_on_top, handles) = match &e.kind {
        ElementKind::Path(p) => (p.stroke_placement, p.fill_on_top, Some(p.fill_handles)),
        ElementKind::Text(t) => (t.stroke_placement, t.fill_on_top, Some(t.fill_handles)),
        _ => (0, false, None),
    };
    let hp = |p: fp_geom::Point| json!({"x": r6(p.x as f64), "y": r6(p.y as f64)});
    let mut v = json!({
        "brush": a.brush.as_ref().map(brush_json),
        "brushColor": hex(a.stroke_color),
        "fill": a.fill.as_ref().map(fill_json),
        "fillColor": hex(a.fill_color),
        // BRPi: 0 innen, 1 mittig, 2 außen (PAR 17/08b)
        "brushPlacement": match placement { 0 => "inside", 2 => "outside", _ => "center" },
        "fillOnTop": fill_on_top,
        "brushTexture": a.stroke_texture.as_ref().map(|t| json!({"name": t.name})),
        "fillTexture": a.fill_texture.as_ref().map(|t| json!({"name": t.name})),
    });
    if let Some(h) = handles {
        v["fillHandle1"] = hp(h.start);
        v["fillHandle2"] = hp(h.end);
        v["fillHandle3"] = hp(h.second);
    }
    v
}

/// Effektliste wie das Original (`{category, name, effects:[{EffectMoaID, EffectIsVisible, …}]}`).
fn effects_json(e: &Element) -> Value {
    let Some(list) = &e.common.effects else { return Value::Null };
    let effects: Vec<Value> = list
        .effects
        .iter()
        .map(|f| {
            let mut m = Map::new();
            m.insert("EffectMoaID".into(), json!(f.moa_id().unwrap_or("")));
            m.insert("EffectIsVisible".into(), json!(f.is_visible()));
            for en in &f.params.entries {
                if en.key == "EffectMoaID" || en.key == "EffectIsVisible" {
                    continue;
                }
                let v = match en.text_value() {
                    Some(t) => typed_scalar(t),
                    None => Value::Null,
                };
                m.insert(en.key.clone(), v);
            }
            if let Some(n) = f.ui_name() {
                m.entry("name").or_insert(json!(n));
            }
            Value::Object(m)
        })
        .collect();
    json!({"category": list.category, "name": list.name, "effects": effects})
}

/// customData-Skalar nach der Leseregel 03 §10.12.
pub(crate) fn typed_scalar(t: &str) -> Value {
    match fp_model::data::type_scalar(t) {
        TypedScalar::Bool(b) => json!(b),
        TypedScalar::Number(x) => serde_json::Number::from_f64(x).map(Value::Number).unwrap_or(Value::Null),
        TypedScalar::String(s) => json!(s),
    }
}

/// Namen der Ausrichtung (`JSTi`) wie im Original.
fn alignment_name(al: i32) -> &'static str {
    match al {
        1 => "center",
        2 => "right",
        3 => "justify",
        4 => "stretch",
        _ => "left",
    }
}

/// Vergleichsschlüssel für Schriftnamen (ohne Leerzeichen/Bindestriche, klein).
fn font_key(s: &str) -> String {
    s.chars().filter(|c| !matches!(c, ' ' | '-' | '_')).flat_map(char::to_lowercase).collect()
}

/// PostScript-Name der Anfangsschrift wie `initialAttrs.face` im Original (`MyriadPro-Regular`).
/// Der Kopf trägt meist den Familiennamen (`Myriad Pro`); dann gilt `CFTs`, sonst der Name des ersten
/// Laufs derselben Familie, sonst Familie ohne Leerzeichen + `-Regular`.
pub(crate) fn ps_face(t: &fp_model::TextElem) -> Option<String> {
    let fam = t.initial.font.clone().or_else(|| t.runs.iter().find_map(|r| r.attrs.font.clone()))?;
    if let Some(ps) = t.initial.ps_font.as_ref().filter(|p| !p.is_empty() && !p.contains(' ')) {
        return Some(ps.clone());
    }
    if !fam.contains(' ') {
        return Some(fam);
    }
    let fk = font_key(&fam);
    if let Some(f) = t.runs.iter().filter_map(|r| r.attrs.font.as_ref()).find(|f| !f.contains(' ') && font_key(f.split('-').next().unwrap_or(f)).starts_with(&fk)) {
        return Some(f.clone());
    }
    Some(format!("{}-Regular", fam.replace(' ', "")))
}

/// Textattribute als Original-Objekt. `full`: alle 15 Laufeigenschaften mit Vorgaben (sonst nur die
/// gesetzten); die Glättungswerte (`antiAliasSharpness`, `antiAliasStrength`, `overSample`) stehen nur in
/// `initialAttrs`.
fn text_attrs_json(a: &fp_model::TextAttrs, full: bool, face: Option<String>) -> Map<String, Value> {
    let mut m = Map::new();
    let mut put = |k: &str, v: Option<Value>, d: Value| {
        if let Some(v) = v {
            m.insert(k.into(), v);
        } else if full {
            m.insert(k.into(), d);
        }
    };
    // f32 über die kürzeste Dezimalform nach f64 (1.2 statt 1.2000000476837158)
    let num = |v: Option<f32>| v.map(|x| json!(x.to_string().parse::<f64>().unwrap_or(x as f64)));
    put("alignment", a.alignment.map(|al| json!(alignment_name(al))), json!("left"));
    put("baselineShift", num(a.baseline_shift), json!(0));
    put("bold", a.bold.map(|b| json!(b)), json!(false));
    put("face", face.map(|f| json!(f)), json!(""));
    put("fillColor", a.color.map(|c| json!(hex(fp_color::Argb(c)))), json!("#000000"));
    put("horizontalScale", num(a.horizontal_scale), json!(1));
    put("italic", a.italic.map(|b| json!(b)), json!(false));
    put("leading", num(a.leading), json!(1));
    put("leadingMode", a.leading_mode.map(|l| json!(if l == 1 { "exact" } else { "percentage" })), json!("percentage"));
    put("paragraphIndent", num(a.indent), json!(0));
    put("paragraphSpacingAfter", num(a.space_after), json!(0));
    put("paragraphSpacingBefore", num(a.space_before), json!(0));
    put("rangeKerning", num(a.tracking), json!(0));
    put("size", a.size.map(|s| json!(format!("{}pt", conv::fmt_num(s as f64)))), json!("12pt"));
    put("underline", a.underline.map(|b| json!(b)), json!(false));
    m
}

/// `textRuns` wie im Original (TW-02): `initialAttrs` mit allen 18 Eigenschaften (15 Laufeigenschaften
/// und Glättung), der erste Lauf mit dem vollständigen aufgelösten Satz, weitere Läufe nur mit den
/// geänderten Werten.
pub(crate) fn text_runs_json(t: &fp_model::TextElem) -> Value {
    let face = ps_face(t);
    let mut init = t.initial.clone();
    if init.color.is_none() {
        init.color = Some(t.attrs.fill_color.0);
    }
    let mut ia = text_attrs_json(&init, true, Some(face.clone().unwrap_or_default()));
    ia.insert("antiAliasSharpness".into(), json!(t.aa_sharpness));
    ia.insert("antiAliasStrength".into(), json!(t.aa_strength));
    ia.insert("overSample".into(), json!(t.aa_oversample));
    let mut runs: Vec<Value> = Vec::with_capacity(t.runs.len());
    let mut cur = init.clone();
    for (i, r) in t.runs.iter().enumerate() {
        cur.apply(&r.attrs);
        let changed = if i == 0 {
            let f = cur.font.clone().filter(|f| !f.contains(' ')).or_else(|| face.clone());
            text_attrs_json(&cur, true, f)
        } else {
            text_attrs_json(&r.attrs, false, r.attrs.font.clone())
        };
        runs.push(json!({"changedAttrs": changed, "characters": r.text}));
    }
    if runs.is_empty() {
        runs.push(json!({"changedAttrs": {}, "characters": t.plain_text()}));
    }
    json!({"initialAttrs": ia, "textRuns": runs})
}

/// Original-Textattribute (`face`, `size` „24pt“, `fillColor`, `bold` …) → Parameter für
/// `text.setAttributes`.
pub(super) fn text_attrs_params(a: Option<&Value>) -> Value {
    let mut m = Map::new();
    let Some(a) = a else { return Value::Object(m) };
    if let Some(f) = a.get("face").and_then(|v| v.as_str()) {
        m.insert("font".into(), json!(f));
    }
    let size = a.get("size").and_then(|v| v.as_str().and_then(|s| s.trim().trim_end_matches("pt").trim_end_matches("px").trim().parse::<f64>().ok()).or_else(|| v.as_f64()));
    if let Some(sz) = size.filter(|s| *s >= 1.0) {
        m.insert("size".into(), json!(sz));
    }
    if let Some(c) = a.get("fillColor").and_then(|v| v.as_str()).and_then(conv::norm_color) {
        m.insert("color".into(), json!(c));
    }
    for k in ["bold", "italic", "underline"] {
        if let Some(b) = a.get(k).and_then(|v| v.as_bool()) {
            m.insert(k.into(), json!(b));
        }
    }
    let al = match a.get("alignment").and_then(|v| v.as_str()) {
        Some("left") => Some(0),
        Some("center") => Some(1),
        Some("right") => Some(2),
        Some("justify") => Some(3),
        Some("stretch") => Some(4),
        _ => None,
    };
    if let Some(al) = al {
        m.insert("alignment".into(), json!(al));
    }
    // Zeilenabstand: Faktor (Modus „percentage“) bzw. px (Modus „exact“), wie `LEDf`
    if let Some(l) = a.get("leading").and_then(|v| v.as_f64()).filter(|l| *l >= 0.0) {
        m.insert("leading".into(), json!(l));
    }
    let mode = match a.get("leadingMode") {
        Some(Value::String(s)) if s == "percentage" => Some(0),
        Some(Value::String(s)) if s == "exact" => Some(1),
        Some(v) => v.as_i64().filter(|i| (0..=1).contains(i)),
        None => None,
    };
    if let Some(mode) = mode {
        m.insert("leadingMode".into(), json!(mode));
    }
    if let Some(h) = a.get("horizontalScale").and_then(|v| v.as_f64()).filter(|h| *h >= 0.01) {
        m.insert("horizontalScale".into(), json!(h));
    }
    for (from, to) in [
        ("rangeKerning", "tracking"),
        ("baselineShift", "baselineShift"),
        ("paragraphIndent", "indent"),
        ("paragraphSpacingBefore", "spaceBefore"),
        ("paragraphSpacingAfter", "spaceAfter"),
    ] {
        if let Some(v) = a.get(from).and_then(|v| v.as_f64()) {
            m.insert(to.into(), json!(v));
        }
    }
    Value::Object(m)
}

/// Glättungswerte aus `initialAttrs` (`antiAliasSharpness`, `antiAliasStrength`, `overSample`) →
/// Parameter für `text.setProperties` (leer, wenn keiner gesetzt ist).
pub(super) fn text_aa_params(a: Option<&Value>) -> Map<String, Value> {
    let mut m = Map::new();
    let Some(a) = a else { return m };
    for (from, to, lo, hi) in [("antiAliasSharpness", "sharpness", 0, 255), ("antiAliasStrength", "strength", 0, 255), ("overSample", "oversample", 1, 64)] {
        if let Some(v) = a.get(from).and_then(|v| v.as_f64()) {
            m.insert(to.into(), json!((v.round() as i64).clamp(lo, hi)));
        }
    }
    m
}

fn contours_of(e: &Element) -> Vec<fp_model::element::Contour> {
    match &e.kind {
        ElementKind::Path(p) => p.contours.clone(),
        _ => Vec::new(),
    }
}

/// Konturen als JSON für `path.setContours`.
fn contours_json(cs: &[fp_model::element::Contour]) -> Value {
    Value::Array(
        cs.iter()
            .map(|c| {
                json!({"closed": c.closed, "nodes": c.nodes.iter().map(|n| json!({
                    "anchor": [n.anchor.x, n.anchor.y], "in": [n.pred.x, n.pred.y], "out": [n.succ.x, n.succ.y]
                })).collect::<Vec<_>>()})
            })
            .collect(),
    )
}

/// Konturen eines Pfads ändern (über `path.setContours`).
fn edit_contours(doc: DocId, id: ElementId, f: impl FnOnce(&mut Vec<fp_model::element::Contour>) -> FwResult<()>) -> FwResult<()> {
    let e = element(doc, id)?;
    let mut cs = contours_of(&e);
    if cs.is_empty() {
        return Err(FwErr::bad("kein Pfad"));
    }
    f(&mut cs)?;
    // leere Konturen behalten einen Knoten (das Original erlaubt length = 0 nur vorübergehend)
    for c in &mut cs {
        if c.nodes.is_empty() {
            c.nodes.push(fp_model::element::ContourNode::corner(fp_geom::Point::new(0.0, 0.0)));
        }
    }
    exec(Some(doc), "path.setContours", json!({"id": id.0, "contours": contours_json(&cs)})).map(|_| ())
}

// ------------------------------------------------------------------ Methoden

pub(super) fn method(cls: &str, name: &str, h: &Handle, args: &[Arg], this: &JsValue, ctx: &mut Context) -> FwResult<JsValue> {
    let _ = this;
    match (cls, name) {
        ("Group", "removeTransformation") => {
            let (doc, id) = elem_handle(h)?;
            with_selected(doc, &[id], "modify.transform.removeTransformations", json!({}))?;
            Ok(JsValue::undefined())
        }
        ("Group", "smartShapeToGlobalCoords") | ("Group", "globalToSmartShapeCoords") => {
            let (doc, id) = elem_handle(h)?;
            let e = element(doc, id)?;
            let m = match &e.kind {
                ElementKind::Group(g) => g.smart_shape.as_ref().and_then(|s| s.transform).unwrap_or(fp_geom::Matrix3::IDENTITY),
                _ => fp_geom::Matrix3::IDENTITY,
            };
            let m = if name == "globalToSmartShapeCoords" { m.inverse().unwrap_or(fp_geom::Matrix3::IDENTITY) } else { m };
            let (x, y) = args.first().and_then(|a| a.point()).unwrap_or((0.0, 0.0));
            let p = m.apply(fp_geom::Point::new(x as f32, y as f32));
            conv::point_js(p.x as f64, p.y as f64, ctx).map_err(jserr)
        }
        ("Group", "RegisterForEvent") | ("Group", "UnRegisterForEvent") | ("Group", "UnRegisterAllEvents") => {
            let (doc, id) = elem_handle(h)?;
            let ev = args.first().and_then(|a| a.str()).unwrap_or_default();
            let mut count = 0usize;
            edit_direct(doc, id, |e| {
                if let ElementKind::Group(g) = &mut e.kind
                    && let Some(s) = &mut g.smart_shape
                {
                    let list = s.events.get_or_insert_with(Vec::new);
                    match name {
                        "RegisterForEvent" => {
                            if !list.contains(&ev) {
                                list.push(ev.clone());
                            }
                        }
                        "UnRegisterForEvent" => list.retain(|x| *x != ev),
                        _ => list.clear(),
                    }
                    count = list.len();
                }
            })?;
            Ok(if name == "RegisterForEvent" { JsValue::from(count as i32) } else { JsValue::undefined() })
        }
        (_, "generateSmartShapeCode") => {
            // JS-Code, der die Pfade des Elements als automatische Form nachbaut (Wurzel = 1. Argument)
            let (doc, id) = elem_handle(h)?;
            let root = args.first().and_then(|a| a.str()).filter(|s| !s.is_empty()).unwrap_or_else(|| "smartShape.elem".into());
            let e = element(doc, id)?;
            let mut paths: Vec<Element> = Vec::new();
            fn collect(e: &Element, out: &mut Vec<Element>) {
                match &e.kind {
                    ElementKind::Path(_) => out.push(e.clone()),
                    ElementKind::Group(g) => g.children.iter().for_each(|c| collect(c, out)),
                    _ => {}
                }
            }
            collect(&e, &mut paths);
            let mut code = String::new();
            for (i, p) in paths.iter().enumerate() {
                code.push_str(&format!("{root}.elements[{i}] = new Path();\n"));
                for (ci, c) in contours_of(p).iter().enumerate() {
                    code.push_str(&format!("{root}.elements[{i}].contours[{ci}] = new Contour();\n{root}.elements[{i}].contours[{ci}].isClosed = {};\n", c.closed));
                    code.push_str(&format!("{root}.elements[{i}].contours[{ci}].nodes.length = {};\n", c.nodes.len()));
                    for (ni_, n) in c.nodes.iter().enumerate() {
                        code.push_str(&format!(
                            "var n = {root}.elements[{i}].contours[{ci}].nodes[{ni_}]; n.predX = {}; n.predY = {}; n.x = {}; n.y = {}; n.succX = {}; n.succY = {};\n",
                            conv::fmt_num(n.pred.x as f64), conv::fmt_num(n.pred.y as f64), conv::fmt_num(n.anchor.x as f64), conv::fmt_num(n.anchor.y as f64), conv::fmt_num(n.succ.x as f64), conv::fmt_num(n.succ.y as f64)
                        ));
                    }
                }
            }
            Ok(conv::s(&code))
        }
        ("ContourNode", _) | ("ControlPoint", _) => {
            // Register*Move wirkt nur während eines Steuerpunkt-Ziehens in der Formen-Laufzeit
            // (crate::autoshape); außerhalb ohne Wirkung.
            Ok(JsValue::undefined())
        }
        ("SliceHotspot", "exportHtmlAndImages") => {
            let (doc, _) = elem_handle(h)?;
            let url = args.first().and_then(|a| a.str()).unwrap_or_default();
            exec(Some(doc), "file.export", json!({"path": crate::urls::url_to_path(&url).to_string_lossy(), "style": "htmlAndImages", "useSlices": true}))?;
            Ok(JsValue::from(true))
        }
        _ => ni(cls, name),
    }
}

// ------------------------------------------------------------------ Properties lesen

pub(super) fn get(cls: &str, name: &str, h: &Handle, this: &JsValue, ctx: &mut Context) -> FwResult<JsValue> {
    let _ = this;
    match h {
        Handle::Element { doc, id } => element_get(cls, name, *doc, *id, ctx),
        Handle::Layer { doc, page, slot, sub } => layer_get(name, *doc, *page, *slot, *sub, ctx),
        Handle::Frame { doc, page, frame } => frame_get(name, *doc, *page, *frame, ctx),
        Handle::Cel { doc, page, slot, sub, frame } => cel_get(name, *doc, *page, *slot, *sub, *frame, ctx),
        Handle::Contour { doc, id, c } => {
            let e = element(*doc, *id)?;
            let cs = contours_of(&e);
            let con = cs.get(*c).ok_or_else(|| FwErr::Hr(hr::E_ARRAY_INDEX, "Kontur".into()))?;
            match name {
                "isClosed" => Ok(JsValue::from(con.closed)),
                "nodes" => runtime::wrap_proxy(ctx, Handle::Nodes { doc: *doc, id: *id, c: *c }, true),
                _ => ni(cls, name),
            }
        }
        Handle::Node { doc, id, c, n } => node_get(cls, name, *doc, *id, *c, *n, ctx),
        _ => ni(cls, name),
    }
}

fn element_get(cls: &str, name: &str, doc: DocId, id: ElementId, ctx: &mut Context) -> FwResult<JsValue> {
    let e = element(doc, id)?;
    let j = |v: Value, ctx: &mut Context| conv::json_to_js(&v, ctx).map_err(jserr);
    Ok(match name {
        "left" | "top" | "width" | "height" => {
            let b = bounds(doc, &e)?;
            conv::n(r6(match name {
                "left" => b[0],
                "top" => b[1],
                "width" => b[2] - b[0],
                _ => b[3] - b[1],
            }))
        }
        "pixelRect" => {
            // Bitmaps: Rechteck der sichtbaren Pixel (Alpha ≥ 2), unabhängig von den Objektgrenzen;
            // leere Bitmap wie im Original ±2³¹ (re\22 §1.2 Regel 3)
            // Maskiertes Einzelelement (Maske ▸ Alles einblenden/ausblenden, fixg5 Q-02): die nach der
            // Maske sichtbaren Pixel; ganz ausgeblendet wie im Original das leere Rechteck ±2³¹.
            let masked = e.masked_single().is_some();
            let vmask = matches!(&e.kind, ElementKind::Group(g) if g.kind == fp_model::element::GroupKind::VectorMask);
            // TW-16: Text meldet wie im Original die Tintengrenzen (gerenderte Alpha), nicht den Rahmen.
            let text = matches!(e.kind, ElementKind::Text(_));
            let r = st(|s| {
                let d = s.session.document(doc).ok_or_else(|| FwErr::bad("Dokument"))?;
                // Q-14 (M1): Vektormaske (Einfügen innen) meldet im Original immer den Maskenbereich,
                // auch ohne Überlappung mit dem Inhalt
                if let ElementKind::Group(g) = &e.kind
                    && matches!(g.kind, fp_model::element::GroupKind::VectorMask)
                    && let Some(m) = &g.mask
                {
                    // TW-35: sonst Inhalt ∩ aktive Maske (Als Maske gruppieren); deaktiviert der Inhalt
                    // allein. Zeigt die Maske Füllung und Strich (Innen einfügen), zählt ihr Bereich mit.
                    let content = g.children.iter().filter_map(|c| fp_engine::docops::element_pixel_rect(&d.doc, c)).reduce(|a, b| a.union(&b));
                    if !g.mask_flags.enabled {
                        return Ok(content);
                    }
                    let mr = fp_engine::docops::element_pixel_rect(&d.doc, m);
                    let inter = content.zip(mr).and_then(|(c, m)| {
                        let r = fp_geom::Rect::new(c.left.max(m.left), c.top.max(m.top), c.right.min(m.right), c.bottom.min(m.bottom));
                        (r.right > r.left && r.bottom > r.top).then_some(r)
                    });
                    return Ok(if g.mask_flags.hide_attrs { inter } else { mr.map(|m| inter.map_or(m, |i| i.union(&m))) });
                }
                if masked || text {
                    let r = fp_render::Renderer::new(fp_render::RenderOptions { background: fp_render::BackgroundMode::Transparent, ..fp_render::RenderOptions::default() });
                    let img = r.render_element(&d.doc, &e, &fp_render::PointMap::identity(), d.frame).image;
                    let b = img.alpha_bounds_min(2);
                    return Ok((!b.is_empty()).then(|| fp_geom::Rect::new(b.left as f32, b.top as f32, b.right as f32, b.bottom as f32)));
                }
                Ok(fp_engine::docops::element_pixel_rect(&d.doc, &e))
            })?;
            let b = match (r, &e.kind) {
                (Some(r), _) => [r.left as f64, r.top as f64, r.right as f64, r.bottom as f64],
                (None, _) if masked || text || vmask => {
                    let m = 2f64.powi(31);
                    [-m, -m, m, m]
                }
                (None, ElementKind::Bitmap(_)) => {
                    let m = 2f64.powi(31);
                    [-m, -m, m, m]
                }
                (None, _) => [0.0; 4],
            };
            conv::rect_js(b, ctx).map_err(jserr)?
        }
        "visible" => JsValue::from(e.common.visible),
        "locked" => JsValue::from(e.common.locked),
        "opacity" => conv::n((e.common.opacity.to_percent() * 100.0).round() / 100.0),
        "blendMode" => conv::s(e.common.blend.name().unwrap_or("normal")),
        // maskiertes Einzelelement: Name des Inhalts, wie im Original (FIX-G5, Q-02)
        "name" => e.common.name.as_ref().or_else(|| e.masked_single().and_then(|(c, _)| c.common.name.as_ref())).map(|n| conv::s(n)).unwrap_or_else(JsValue::null),
        "isLayer" => JsValue::from(matches!(e.kind, ElementKind::SubLayer(_))),
        "isSmartShape" => JsValue::from(matches!(&e.kind, ElementKind::Group(g) if g.smart_shape.is_some())),
        "effectList" => j(effects_json(&e), ctx)?,
        "customData" => runtime::wrap_proxy(ctx, Handle::CustomData { doc, id }, false)?,
        "mask" => match &e.kind {
            ElementKind::Group(g) if g.kind.is_mask() => {
                let el = match &g.mask {
                    Some(m) => wrap_element(ctx, doc, m)?,
                    None => JsValue::null(),
                };
                let mode = if matches!(g.kind, fp_model::element::GroupKind::VectorMask) { "mask to path" } else { "mask to image" };
                conv::object(
                    vec![("element", el), ("owner", wrap_element(ctx, doc, &e)?), ("linked", JsValue::from(g.mask_flags.linked)), ("enabled", JsValue::from(g.mask_flags.enabled)), ("mode", conv::s(mode)), ("showAttrs", JsValue::from(!g.mask_flags.hide_attrs)), ("autoExpandImages", JsValue::from(g.mask_flags.auto_expand))],
                    ctx,
                )
                .map_err(jserr)?
            }
            _ => JsValue::null(),
        },
        "pathAttributes" => {
            let v = j(path_attrs_json(&e), ctx)?;
            if let Some(o) = v.as_object() {
                // PathAttrs.setDefaultFillHandles(): Füll-Handles des Elements auf die Vorgabe setzen
                let f = boa_engine::NativeFunction::from_copy_closure_with_captures(
                    |_, _, caps: &(u64, u64), _| {
                        let (d, i) = *caps;
                        default_fill_handles(DocId(d), ElementId(i)).map(|_| JsValue::undefined()).map_err(|e| runtime::throw(e, "PathAttrs.setDefaultFillHandles"))
                    },
                    (doc.0, id.0),
                );
                let func = boa_engine::object::FunctionObjectBuilder::new(ctx.realm(), f).name(boa_engine::js_string!("setDefaultFillHandles")).build();
                o.set(boa_engine::js_string!("setDefaultFillHandles"), func, false, ctx).map_err(jserr)?;
            }
            v
        }
        "contours" => match &e.kind {
            ElementKind::Path(_) => runtime::wrap_proxy(ctx, Handle::Contours { doc, id }, true)?,
            _ => JsValue::null(),
        },
        "isEvenOddFill" => JsValue::from(matches!(&e.kind, ElementKind::Path(p) if p.even_odd)),
        "pathOperation" => conv::s("none"),
        "randSeed" => conv::n(match &e.kind {
            ElementKind::Path(p) => p.random_seed as f64,
            ElementKind::Text(t) => t.random_seed as f64,
            _ => 0.0,
        }),
        "textureOffset" => conv::point_js(0.0, 0.0, ctx).map_err(jserr)?,
        // Einheit px: Rundung in Pixeln der aktuellen Größe, `mode` „exact“ (wie das Original, PAR 17/05)
        "roundness" => conv::n(match &e.kind {
            ElementKind::Path(p) => p
                .primitive
                .as_ref()
                .map(|r| if r.roundness_pixels == Some(true) { r6(fp_engine::commands::panel::primitive_roundness_px(r) as f64) } else { r6(r.roundness as f64) })
                .unwrap_or(0.0),
            _ => 0.0,
        }),
        "originalSides" => match &e.kind {
            ElementKind::Path(p) => match &p.primitive {
                Some(r) => {
                    let b = original_sides(r);
                    conv::rect_js([r6(b.left as f64), r6(b.top as f64), r6(b.right as f64), r6(b.bottom as f64)], ctx).map_err(jserr)?
                }
                None => JsValue::null(),
            },
            _ => JsValue::null(),
        },
        "mode" => conv::s(match &e.kind {
            ElementKind::Path(p) if p.primitive.as_ref().is_some_and(|r| r.roundness_pixels == Some(true)) => "exact",
            _ => "percentage",
        }),
        "transform" => {
            let m = match &e.kind {
                ElementKind::Instance(i) => Some(i.transform),
                ElementKind::Text(t) => t.transform,
                ElementKind::Path(p) => p.primitive.as_ref().map(|r| r.transform),
                _ => None,
            }
            .unwrap_or_default();
            let a: Vec<f64> = m.m.iter().flat_map(|row| row.iter().map(|v| r6(*v as f64))).collect();
            j(json!({"matrix": a}), ctx)?
        }
        // Text
        "textChars" => match &e.kind {
            ElementKind::Text(t) => conv::s(&t.plain_text()),
            _ => JsValue::undefined(),
        },
        "textRuns" => match &e.kind {
            ElementKind::Text(t) => j(text_runs_json(t), ctx)?,
            _ => JsValue::null(),
        },
        "rawLeft" | "rawTop" | "rawWidth" | "rawHeight" => match &e.kind {
            ElementKind::Text(t) => {
                let b = t.bounds;
                conv::n(r6(match name {
                    "rawLeft" => b.left as f64,
                    "rawTop" => b.top as f64,
                    "rawWidth" => (b.right - b.left) as f64,
                    _ => (b.bottom - b.top) as f64,
                }))
            }
            _ => JsValue::undefined(),
        },
        "font" | "fontsize" | "bold" | "italic" | "underline" | "alignment" | "justify" => match &e.kind {
            ElementKind::Text(t) => {
                let a = t.resolved_runs().first().map(|(a, _)| a.clone()).unwrap_or_else(|| t.initial.clone());
                match name {
                    "font" => a.font.map(|f| conv::s(&f)).unwrap_or_else(JsValue::null),
                    "fontsize" => conv::n(a.size.unwrap_or(12.0).round() as f64),
                    "bold" => JsValue::from(a.bold.unwrap_or(false)),
                    "italic" => JsValue::from(a.italic.unwrap_or(false)),
                    "underline" => JsValue::from(a.underline.unwrap_or(false)),
                    _ => conv::s(match a.alignment.unwrap_or(0) {
                        1 => "center",
                        2 => "right",
                        3 => "justify",
                        4 => "stretch",
                        _ => "left",
                    }),
                }
            }
            _ => JsValue::undefined(),
        },
        // „Keine Kantenglättung“ ist kein `TAAi`-Wert, sondern die harte Füllkante; `TAAi` 0 = glatt
        // (re\09 §3.5, BA-066)
        "antiAliased" => JsValue::from(matches!(&e.kind, ElementKind::Text(t) if t.attrs.fill.as_ref().is_none_or(|f| f.edge != 0))),
        "antiAliasMode" => conv::s(match &e.kind {
            ElementKind::Text(t) => super::aa_name(t.antialias),
            _ => "smooth",
        }),
        "autoKern" => JsValue::from(matches!(&e.kind, ElementKind::Text(t) if t.auto_kern)),
        "autoExpand" => JsValue::from(matches!(&e.kind, ElementKind::Text(t) if t.frame_type.unwrap_or(0) == 0)),
        "orientation" => conv::s(match &e.kind {
            ElementKind::Text(t) => super::orientation_jsf_name(t.orientation as i32),
            _ => "horizontal left to right",
        }),
        "transformMode" => conv::s("paths"),
        // Gruppe
        "elements" => match &e.kind {
            ElementKind::Group(g) => {
                let mut list: Vec<Arc<Element>> = Vec::new();
                if let Some(m) = &g.mask {
                    list.push(Arc::clone(m));
                }
                list.extend(g.children.iter().cloned());
                super::wrap_elements(ctx, doc, &list)?
            }
            ElementKind::CompoundShape(c) => super::wrap_elements(ctx, doc, &c.operands)?,
            _ => conv::array(Vec::new(), ctx),
        },
        "groupType" => conv::s(match &e.kind {
            ElementKind::Group(g) if g.kind.is_mask() => {
                if matches!(g.kind, fp_model::element::GroupKind::VectorMask) {
                    "mask to path"
                } else {
                    "mask to image"
                }
            }
            _ => "normal",
        }),
        "controlPoints" => conv::array(Vec::new(), ctx),
        "smartShapeCode" => conv::s(&match &e.kind {
            ElementKind::Group(g) => g.smart_shape.as_ref().map(|s| s.script()).unwrap_or_default(),
            _ => String::new(),
        }),
        // Hotspot/Slice: Kontur (Polygon), Verhalten (Behavior {call, event}), Exportoptionen
        "contour" => match &e.kind {
            ElementKind::WebObject(w) => match &w.polygon {
                Some(c) => j(json!({"isClosed": c.closed, "nodes": c.nodes.iter().map(|n| json!({"x": n.anchor.x, "y": n.anchor.y, "predX": n.pred.x, "predY": n.pred.y, "succX": n.succ.x, "succY": n.succ.y})).collect::<Vec<_>>()}), ctx)?,
                None => JsValue::null(),
            },
            _ => JsValue::null(),
        },
        "behaviors" => match &e.kind {
            ElementKind::WebObject(w) => {
                let list: Vec<Value> = w
                    .behaviors
                    .as_ref()
                    .map(|b| b.items.iter().map(|x| json!({"call": format!("behavior{}{}", x.action, x.status_text.as_deref().map(|t| format!(":{t}")).unwrap_or_default()), "event": x.event})).collect())
                    .unwrap_or_default();
                j(Value::Array(list), ctx)?
            }
            _ => JsValue::null(),
        },
        "exportOptions" => match &e.kind {
            ElementKind::WebObject(w) => match w.slice.as_ref().and_then(|s| s.export.as_ref()) {
                Some(x) => j(serde_json::to_value(&**x).unwrap_or(Value::Null), ctx)?,
                None => JsValue::null(),
            },
            _ => JsValue::null(),
        },
        // Instanz
        "symbolID" | "symbolName" | "instanceType" => match &e.kind {
            ElementKind::Instance(i) => {
                let sym = with_state(|s| s.session.document(doc).and_then(|d| d.doc.symbol_by_id(i.symbol_id).map(|(_, s)| (s.name.clone(), s.kind().js_name()))))?;
                match name {
                    "symbolID" => conv::s(&i.symbol_id.0.to_string()),
                    "symbolName" => sym.as_ref().map(|s| conv::s(&s.0)).unwrap_or_else(JsValue::null),
                    _ => sym.as_ref().map(|s| conv::s(s.1)).unwrap_or_else(JsValue::null),
                }
            }
            _ => JsValue::null(),
        },
        "altText" | "urlText" | "targetText" => {
            let v = match &e.kind {
                ElementKind::WebObject(w) => match name {
                    "altText" => w.alt_text.clone(),
                    "urlText" => w.url.clone(),
                    _ => w.target.clone(),
                },
                ElementKind::Instance(i) => match name {
                    "altText" => i.button.alt.clone(),
                    "urlText" => i.button.url.clone(),
                    _ => i.button.target.clone(),
                },
                _ => None,
            };
            v.map(|t| conv::s(&t)).unwrap_or_else(|| conv::s(""))
        }
        "shape" => conv::s(match &e.kind {
            ElementKind::WebObject(w) => match w.shape {
                fp_model::WebShape::Circle => "circle",
                fp_model::WebShape::Polygon => "polygon",
                _ => "rectangle",
            },
            _ => "rectangle",
        }),
        "color" => match &e.kind {
            ElementKind::WebObject(w) => conv::s(&hex_rgb(w.color)),
            _ => JsValue::null(),
        },
        "sliceID" | "baseName" => match &e.kind {
            ElementKind::WebObject(w) => conv::s(&w.file_name.clone().unwrap_or_else(|| format!("slice_{}", id.0))),
            _ => JsValue::null(),
        },
        "sliceKind" => conv::s("image"),
        "htmlText" | "tdTagText" => conv::s(""),
        "resultantPath" => match &e.kind {
            ElementKind::CompoundShape(c) => match &c.result {
                Some(r) => wrap_element(ctx, doc, r)?,
                None => JsValue::null(),
            },
            _ => JsValue::null(),
        },
        _ => return ni(cls, name),
    })
}

fn node_get(cls: &str, name: &str, doc: DocId, id: ElementId, c: usize, n: usize, ctx: &mut Context) -> FwResult<JsValue> {
    let e = element(doc, id)?;
    let cs = contours_of(&e);
    let node = cs.get(c).and_then(|k| k.nodes.get(n)).ok_or_else(|| FwErr::Hr(hr::E_ARRAY_INDEX, "Knoten".into()))?;
    Ok(match name {
        "x" => conv::n(r6(node.anchor.x as f64)),
        "y" => conv::n(r6(node.anchor.y as f64)),
        "predX" => conv::n(r6(node.pred.x as f64)),
        "predY" => conv::n(r6(node.pred.y as f64)),
        "succX" => conv::n(r6(node.succ.x as f64)),
        "succY" => conv::n(r6(node.succ.y as f64)),
        "isCurvePoint" => JsValue::from(node.curve),
        "isSelectedPoint" => {
            let sel = with_state(|s| s.session.document(doc).map(|d| d.selection.points.iter().any(|p| p.element == id && p.contour == c && p.node == n)).unwrap_or(false))?;
            JsValue::from(sel)
        }
        "randomSeed" => conv::n(node.random.unwrap_or(0) as f64),
        "name" => conv::s(""),
        "dynamicInfo" => match &node.pressure {
            // ContourNodeDynamicInfo: Druck/Geschwindigkeit der Stützstellen (Mittel) und deren Anzahl
            Some(p) if !p.samples.is_empty() => {
                let n = p.samples.len() as f64;
                let pr = p.samples.iter().map(|s| s.pressure.unwrap_or(1.0) as f64).sum::<f64>() / n;
                let ve = p.samples.iter().map(|s| s.velocity.unwrap_or(0.0) as f64).sum::<f64>() / n;
                conv::object(vec![("velocity", conv::n(r6(ve))), ("duration", conv::n(p.count.unwrap_or(p.samples.len() as u32) as f64)), ("pressure", conv::n(r6(pr)))], ctx).map_err(jserr)?
            }
            _ => JsValue::null(),
        }
        _ => return ni(cls, name),
    })
}

/// Flache Ebenenliste wie `dom.layers` im Original (Befund F2-1): jede Ebene der obersten Stufe
/// (unten → oben), direkt danach ihre Unterebenen (unten → oben, rekursiv). Unterebenen sind in
/// Firepixel Elemente (`ElementKind::SubLayer`) im Cel des aktuellen Status.
pub(super) fn flat_layers(doc: DocId, page: usize) -> FwResult<Vec<(usize, Option<fp_model::ElementId>)>> {
    Ok(flat_layer_entries(doc, page)?.into_iter().map(|e| (e.slot, e.sub)).collect())
}

/// Ein Eintrag der flachen Ebenenliste (Index = Ebenennummer im Skript-API, wie `dom.layers[i]`).
#[derive(Clone, Debug)]
pub(crate) struct FlatLayer {
    /// Platz der Ebene der obersten Stufe, zu der der Eintrag gehört.
    pub slot: usize,
    /// Unterebene (Element-ID), `None` für die Ebene der obersten Stufe selbst.
    pub sub: Option<fp_model::ElementId>,
    /// Ebenen-ID (bei Unterebenen die ID der inneren Ebene); `None` z. B. für die Masterseitenebene.
    pub id: Option<u64>,
    /// Flacher Index der Elternebene, `None` auf der obersten Stufe.
    pub parent: Option<usize>,
    /// Platz der Masterseitenebene.
    pub master: bool,
}

/// Flache Ebenenliste der Seite mit IDs und Eltern (gleiche Reihenfolge wie [`flat_layers`]).
pub(crate) fn flat_layer_entries(doc: DocId, page: usize) -> FwResult<Vec<FlatLayer>> {
    fn subs(l: &fp_model::Layer, frame: usize, slot: usize, parent: usize, out: &mut Vec<FlatLayer>) {
        let Some(cel) = l.cel(frame).or_else(|| l.cel(0)) else { return };
        for e in cel.elements.iter().rev() {
            if let ElementKind::SubLayer(sl) = &e.kind {
                let me = out.len();
                out.push(FlatLayer { slot, sub: Some(e.id), id: Some(sl.id.0), parent: Some(parent), master: false });
                subs(sl, frame, slot, me, out);
            }
        }
    }
    st(|s| {
        let d = s.session.document(doc).ok_or_else(|| FwErr::bad("Dokument"))?;
        let p = d.doc.pages.get(page).ok_or_else(|| FwErr::bad("Seite"))?;
        let mut out = Vec::new();
        for (slot, ls) in p.root.layers.iter().enumerate() {
            let l = match ls {
                LayerSlot::Own(l) => Some(Arc::clone(l)),
                LayerSlot::Shared(r) => d.doc.shared_layer(&r.id).and_then(|x| x.layer.clone()),
                _ => None,
            };
            let me = out.len();
            out.push(FlatLayer { slot, sub: None, id: l.as_ref().map(|l| l.id.0), parent: None, master: matches!(ls, LayerSlot::MasterPage(_)) });
            if let Some(l) = l {
                subs(&l, d.frame, slot, me, &mut out);
            }
        }
        Ok(out)
    })
}

fn layer_snapshot(doc: DocId, page: usize, slot: usize, sub: Option<fp_model::ElementId>) -> FwResult<(Option<Arc<fp_model::Layer>>, bool)> {
    st(|s| {
        let d = s.session.document(doc).ok_or_else(|| FwErr::bad("Dokument"))?;
        if let Some(id) = sub {
            let e = d.doc.find_element(id).and_then(|p| d.doc.element_at(&p).cloned());
            return Ok(match e.as_deref().map(|e| &e.kind) {
                Some(ElementKind::SubLayer(l)) => (Some(Arc::clone(l)), false),
                _ => (None, false),
            });
        }
        let p = d.doc.pages.get(page).ok_or_else(|| FwErr::bad("Seite"))?;
        Ok(match p.root.layers.get(slot) {
            Some(LayerSlot::Own(l)) => (Some(Arc::clone(l)), false),
            Some(LayerSlot::Shared(r)) => (d.doc.shared_layer(&r.id).and_then(|x| x.layer.clone()), true),
            _ => (None, false),
        })
    })
}

fn layer_get(name: &str, doc: DocId, page: usize, slot: usize, sub: Option<fp_model::ElementId>, ctx: &mut Context) -> FwResult<JsValue> {
    let (l, _shared_pages) = layer_snapshot(doc, page, slot, sub)?;
    let frames = st(|s| Ok(s.session.document(doc).and_then(|d| d.doc.pages.get(page).map(|p| p.root.frame_count.max(1) as usize)).unwrap_or(1)))?;
    Ok(match name {
        "isLayer" => JsValue::from(true),
        "name" => conv::s(l.as_ref().map(|l| l.name.as_str()).unwrap_or("Masterseite")),
        "layerType" => conv::s(match l.as_ref().map(|l| l.kind) {
            Some(fp_model::LayerKind::Web) => "web",
            _ => "normal",
        }),
        "sharing" => conv::s(if l.as_ref().is_some_and(|l| l.shared_across_frames) { "shared" } else { "not shared" }),
        "disclosure" => JsValue::from(l.as_ref().is_some_and(|l| l.expanded)),
        "frames" => {
            let mut v = Vec::new();
            for f in 0..frames {
                v.push(runtime::wrap(ctx, "FrameNLayerIntersection", Handle::Cel { doc, page, slot, sub, frame: f })?);
            }
            conv::array(v, ctx)
        }
        "elems" => {
            let els: Vec<Arc<Element>> = l.as_ref().and_then(|l| l.cel(0)).map(|c| c.elements.clone()).unwrap_or_default();
            super::wrap_elements(ctx, doc, &els)?
        }
        _ => return ni("Layer", name),
    })
}

fn frame_get(name: &str, doc: DocId, page: usize, frame: usize, ctx: &mut Context) -> FwResult<JsValue> {
    let n_layers = st(|s| Ok(s.session.document(doc).and_then(|d| d.doc.pages.get(page).map(|p| p.root.layers.len())).unwrap_or(0)))?;
    Ok(match name {
        "layers" | "topLayers" => {
            let list = if name == "layers" { flat_layers(doc, page)? } else { (0..n_layers).map(|s| (s, None)).collect() };
            let mut v = Vec::new();
            for (slot, sub) in list {
                v.push(runtime::wrap(ctx, "FrameNLayerIntersection", Handle::Cel { doc, page, slot, sub, frame })?);
            }
            conv::array(v, ctx)
        }
        "visible" => JsValue::from(true),
        "delay" => conv::n(7.0),
        "disposal" => conv::s("unspecified"),
        "name" => conv::s(&format!("Status {}", frame + 1)),
        _ => return ni("Frame", name),
    })
}

fn cel_get(name: &str, doc: DocId, page: usize, slot: usize, sub: Option<fp_model::ElementId>, frame: usize, ctx: &mut Context) -> FwResult<JsValue> {
    let (l, _) = layer_snapshot(doc, page, slot, sub)?;
    let cel = l.as_ref().and_then(|l| l.cel(frame)).cloned();
    Ok(match name {
        "elements" | "elemsandsublayers" => {
            // wie im Original: `elements` ohne Unterebenen (die stehen flach in `dom.layers`)
            let mut els: Vec<Arc<Element>> = cel.map(|c| c.elements.clone()).unwrap_or_default();
            if name == "elements" {
                els.retain(|e| !matches!(e.kind, ElementKind::SubLayer(_)));
            }
            super::wrap_elements(ctx, doc, &els)?
        }
        "visible" => JsValue::from(l.as_ref().is_none_or(|l| l.is_visible()) && cel.as_ref().is_none_or(|c| c.visible)),
        "locked" => JsValue::from(cel.as_ref().is_some_and(|c| c.locked)),
        "name" => conv::s(l.as_ref().map(|l| l.name.as_str()).unwrap_or("")),
        _ => return ni("FrameNLayerIntersection", name),
    })
}

// ------------------------------------------------------------------ Properties schreiben

pub(super) fn set(cls: &str, name: &str, h: &Handle, a: Arg, raw: &JsValue, this: &JsValue, ctx: &mut Context) -> FwResult<()> {
    let _ = this;
    match h {
        Handle::Element { doc, id } => element_set(cls, name, *doc, *id, a, raw, ctx),
        Handle::Layer { doc, page, slot, sub } => {
            let (l, _) = layer_snapshot(*doc, *page, *slot, *sub)?;
            let Some(l) = l else { return Ok(()) };
            match name {
                "name" => exec(Some(*doc), "layer.rename", json!({"layer": l.id.0, "name": a.str().unwrap_or_default()})).map(|_| ()),
                "sharing" => exec(Some(*doc), "layer.setShared", json!({"layer": l.id.0, "shared": a.str().as_deref() == Some("shared")})).map(|_| ()),
                "disclosure" => Ok(()),
                _ => ni("Layer", &format!("{name} =")),
            }
        }
        Handle::Cel { doc, page, slot, sub, .. } => {
            let (l, _) = layer_snapshot(*doc, *page, *slot, *sub)?;
            let Some(l) = l else { return Ok(()) };
            match name {
                "visible" => exec(Some(*doc), "layer.setVisible", json!({"layer": l.id.0, "visible": a.bool()})).map(|_| ()),
                "locked" => exec(Some(*doc), "layer.setLocked", json!({"layer": l.id.0, "locked": a.bool()})).map(|_| ()),
                _ => ni("FrameNLayerIntersection", &format!("{name} =")),
            }
        }
        Handle::Frame { .. } => match name {
            "delay" | "disposal" | "visible" | "name" => Ok(()),
            _ => ni("Frame", &format!("{name} =")),
        },
        Handle::Contour { doc, id, c } => match name {
            "isClosed" => {
                let closed = a.bool();
                let c = *c;
                edit_contours(*doc, *id, |cs| {
                    if let Some(k) = cs.get_mut(c) {
                        k.closed = closed;
                    }
                    Ok(())
                })
            }
            _ => ni("Contour", &format!("{name} =")),
        },
        Handle::Node { doc, id, c, n } => node_set(name, *doc, *id, *c, *n, a),
        _ => ni(cls, &format!("{name} =")),
    }
}

fn element_set(cls: &str, name: &str, doc: DocId, id: ElementId, a: Arg, raw: &JsValue, ctx: &mut Context) -> FwResult<()> {
    let _ = (raw, ctx);
    match name {
        "left" | "top" => {
            let e = element(doc, id)?;
            let b = bounds(doc, &e)?;
            let v = a.f64();
            let (dx, dy) = if name == "left" { (v - b[0], 0.0) } else { (0.0, v - b[1]) };
            if dx != 0.0 || dy != 0.0 {
                exec(Some(doc), "edit.move", json!({"dx": dx, "dy": dy, "ids": [id.0]}))?;
            }
            Ok(())
        }
        "visible" => exec(Some(doc), "element.setVisible", json!({"ids": [id.0], "visible": a.bool()})).map(|_| ()),
        "opacity" => exec(Some(doc), "element.setOpacity", json!({"ids": [id.0], "opacity": a.f64().clamp(0.0, 100.0)})).map(|_| ()),
        "blendMode" => exec(Some(doc), "element.setBlendMode", json!({"ids": [id.0], "mode": a.str().unwrap_or_else(|| "normal".into())})).map(|_| ()),
        "name" => exec(Some(doc), "element.setName", json!({"ids": [id.0], "name": a.str().unwrap_or_default()})).map(|_| ()),
        "effectList" => super::doc::apply_effects(doc, &[id], &a.json()),
        "customData" => {
            let v = a.json();
            set_custom_data_all(doc, id, &v)
        }
        "textChars" => exec(Some(doc), "text.setText", json!({"id": id.0, "text": a.str().unwrap_or_default()})).map(|_| ()),
        "font" => exec(Some(doc), "text.setAttributes", json!({"ids": [id.0], "font": a.str().unwrap_or_default()})).map(|_| ()),
        "fontsize" => exec(Some(doc), "text.setAttributes", json!({"ids": [id.0], "size": a.f64().max(1.0)})).map(|_| ()),
        "bold" | "italic" | "underline" => exec(Some(doc), "text.setAttributes", json!({"ids": [id.0], name: a.bool()})).map(|_| ()),
        "textRuns" => {
            let v = a.json();
            let text: String = v
                .get("textRuns")
                .and_then(|r| r.as_array())
                .map(|x| x.iter().filter_map(|r| r.get("characters").and_then(|c| c.as_str())).collect::<Vec<_>>().join(""))
                .unwrap_or_default();
            if !text.is_empty() {
                exec(Some(doc), "text.setText", json!({"id": id.0, "text": text}))?;
            }
            let mut attrs = text_attrs_params(v.get("initialAttrs"));
            if let (Value::Object(m), Some(Value::Object(first))) =
                (&mut attrs, v.get("textRuns").and_then(|r| r.as_array()).and_then(|x| x.first()).and_then(|r| r.get("changedAttrs")).map(|c| text_attrs_params(Some(c))))
            {
                m.extend(first);
            }
            if let Value::Object(m) = &mut attrs
                && !m.is_empty()
            {
                m.insert("ids".into(), json!([id.0]));
                exec(Some(doc), "text.setAttributes", attrs)?;
            }
            let mut aa = text_aa_params(v.get("initialAttrs"));
            if !aa.is_empty() {
                aa.insert("ids".into(), json!([id.0]));
                exec(Some(doc), "text.setProperties", Value::Object(aa))?;
            }
            Ok(())
        }
        "autoExpand" => {
            let auto = a.bool();
            text_prop(doc, id, cls, name, json!({"frameType": if auto { "auto" } else { "fixed" }}))
        }
        "autoKern" => {
            let v = a.bool();
            text_prop(doc, id, cls, name, json!({"autoKern": v}))
        }
        "antiAliased" => {
            // an: bisheriger Glättungsmodus (`TAAi`) mit weicher Kante, aus: harte Füllkante
            let cur = match &element(doc, id)?.kind {
                ElementKind::Text(t) => t.antialias,
                _ => 0,
            };
            let v = a.bool();
            text_prop(doc, id, cls, name, json!({"antialias": if v { super::aa_name(cur) } else { "none" }}))
        }
        "antiAliasMode" => text_prop(doc, id, cls, name, json!({"antialias": super::aa_mode_from_jsf(&a.str().unwrap_or_default())})),
        "orientation" => {
            let code = super::orientation_code_from_jsf(&a.str().unwrap_or_default());
            text_prop(doc, id, cls, name, json!({"orientation": super::orientation_name(code)}))
        }
        "alignment" | "justify" => {
            let code = match a.str().unwrap_or_default().as_str() {
                "center" => 1,
                "right" => 2,
                "justify" => 3,
                "stretch" => 4,
                _ => 0,
            };
            exec(Some(doc), "text.setAttributes", json!({"ids": [id.0], "alignment": code})).map(|_| ())
        }
        "pathAttributes" => {
            let v = a.json();
            if let Some(c) = v.get("fillColor").and_then(|c| c.as_str()).and_then(conv::norm_color) {
                exec(Some(doc), "element.setFill", json!({"ids": [id.0], "color": c}))?;
            }
            if let Some(c) = v.get("brushColor").and_then(|c| c.as_str()).and_then(conv::norm_color) {
                exec(Some(doc), "element.setStroke", json!({"ids": [id.0], "color": c}))?;
            }
            Ok(())
        }
        "altText" | "urlText" | "targetText" => {
            let v = a.str().unwrap_or_default();
            let e = element(doc, id)?;
            match &e.kind {
                ElementKind::WebObject(_) => {
                    let key = match name {
                        "altText" => "alt",
                        "urlText" => "url",
                        _ => "target",
                    };
                    exec(Some(doc), "pi.web", json!({"ids": [id.0], key: v})).map(|_| ())
                }
                _ => edit_direct(doc, id, |e| {
                    if let ElementKind::Instance(i) = &mut e.kind {
                        let slot = match name {
                            "altText" => &mut i.button.alt,
                            "urlText" => &mut i.button.url,
                            _ => &mut i.button.target,
                        };
                        *slot = Some(v);
                    }
                }),
            }
        }
        "rawWidth" | "rawHeight" | "rawLeft" | "rawTop" => {
            // Textrahmen: Breite (0 = automatisch), Höhe, Lage der Rahmenecke
            let v = a.f64();
            match name {
                "rawWidth" => {
                    let p = if v <= 0.0 { json!({"ids": [id.0], "frameType": "auto"}) } else { json!({"ids": [id.0], "frameType": "fixed", "width": v}) };
                    exec(Some(doc), "text.setProperties", p).map(|_| ())
                }
                "rawHeight" => edit_text(doc, id, cls, name, |t| {
                    let r = t.anchor_rect.unwrap_or(t.bounds);
                    t.anchor_rect = Some(fp_geom::Rect::new(r.left, r.top, r.right, r.top + v.max(1.0) as f32));
                }),
                _ => {
                    let e = element(doc, id)?;
                    // raw* = gespeicherter Textrahmen ohne den angezeigten 2-px-Rand (EB-1)
                    let b = match &e.kind {
                        ElementKind::Text(t) => [t.bounds.left as f64, t.bounds.top as f64],
                        _ => {
                            let b = bounds(doc, &e)?;
                            [b[0], b[1]]
                        }
                    };
                    let (dx, dy) = if name == "rawLeft" { (v - b[0], 0.0) } else { (0.0, v - b[1]) };
                    exec(Some(doc), "edit.move", json!({"dx": dx, "dy": dy, "ids": [id.0]})).map(|_| ())
                }
            }
        }
        "mask" => {
            // ElementMask-Objekt zurückschreiben: linked, enabled, mode, showAttrs, autoExpandImages
            let m = a.json();
            if m.is_null() {
                return with_selected(doc, &[id], "modify.mask.deleteMask", json!({}));
            }
            let b = |k: &str| m.get(k).and_then(|v| v.as_bool());
            let mode = m.get("mode").and_then(|v| v.as_str()).map(String::from);
            edit_direct(doc, id, |e| {
                if let ElementKind::Group(g) = &mut e.kind
                    && g.kind.is_mask()
                {
                    if let Some(v) = b("linked") {
                        g.mask_flags.linked = v;
                    }
                    if let Some(v) = b("enabled") {
                        g.mask_flags.enabled = v;
                    }
                    if let Some(v) = b("showAttrs") {
                        g.mask_flags.hide_attrs = !v;
                    }
                    if let Some(v) = b("autoExpandImages") {
                        g.mask_flags.auto_expand = v;
                    }
                    match mode.as_deref() {
                        Some("mask to path") => g.kind = fp_model::element::GroupKind::VectorMask,
                        Some("mask to image") | Some("alpha") | Some("grayscale") => g.kind = fp_model::element::GroupKind::BitmapMask,
                        _ => {}
                    }
                    e.cache = None;
                }
            })
        }
        "baseName" => exec(Some(doc), "pi.web", json!({"ids": [id.0], "fileName": a.str().unwrap_or_default()})).map(|_| ()),
        "htmlText" => exec(Some(doc), "pi.web", json!({"ids": [id.0], "html": a.str().unwrap_or_default()})).map(|_| ()),
        "sliceKind" => exec(Some(doc), "pi.web", json!({"ids": [id.0], "sliceType": if a.str().unwrap_or_default().to_lowercase().contains("html") { 1 } else { 0 }})).map(|_| ()),
        "tdTagText" => {
            let v = a.str().unwrap_or_default();
            edit_direct(doc, id, |e| {
                if let ElementKind::WebObject(w) = &mut e.kind
                    && let Some(s) = &mut w.slice
                {
                    s.td_text = Some(v);
                }
            })
        }
        "shape" => {
            let s = match a.str().unwrap_or_default().to_lowercase().as_str() {
                "circle" => 1,
                "polygon" | "poly" => 2,
                _ => 0,
            };
            exec(Some(doc), "pi.web", json!({"ids": [id.0], "shape": s})).map(|_| ())
        }
        "color" => {
            let c = a.str().and_then(|c| conv::norm_color(&c)).unwrap_or_else(|| "#00ff00".into());
            exec(Some(doc), "pi.web", json!({"ids": [id.0], "color": c})).map(|_| ())
        }
        "isEvenOddFill" | "randSeed" | "textureOffset" | "pathOperation" => {
            let (b, n, p, s) = (a.bool(), a.i64(), a.point(), a.str().unwrap_or_default().to_lowercase());
            edit_direct(doc, id, |e| {
                if let ElementKind::Path(pe) = &mut e.kind {
                    match name {
                        "isEvenOddFill" => pe.even_odd = b,
                        "randSeed" => pe.random_seed = n.max(0) as u32,
                        "textureOffset" => {
                            if let Some((x, y)) = p {
                                pe.texture_offset = fp_geom::Point::new(x as f32, y as f32);
                            }
                        }
                        _ => {
                            pe.path_op = match s.as_str() {
                                "union" => 1,
                                "intersect" => 2,
                                "punch" | "subtract" => 3,
                                "crop" => 4,
                                _ => 0,
                            }
                        }
                    }
                    e.cache = None;
                }
            })
        }
        "transformMode" => {
            let pixels = a.str().unwrap_or_default().to_lowercase().contains("pixel");
            edit_direct(doc, id, |e| match &mut e.kind {
                ElementKind::Instance(i) => i.transform_mode = i32::from(pixels),
                ElementKind::Text(t) => t.transform_mode = i32::from(pixels),
                _ => {}
            })
        }
        "groupType" => {
            let t = a.str().unwrap_or_default().to_lowercase();
            edit_direct(doc, id, |e| {
                if let ElementKind::Group(g) = &mut e.kind
                    && g.mask.is_some()
                {
                    g.kind = if t.contains("image") { fp_model::element::GroupKind::BitmapMask } else { fp_model::element::GroupKind::VectorMask };
                }
            })
        }
        "smartShapeCode" => {
            let code = a.str().unwrap_or_default();
            edit_direct(doc, id, |e| {
                if let ElementKind::Group(g) = &mut e.kind
                    && let Some(s) = &mut g.smart_shape
                {
                    s.set_script(&code);
                }
            })
        }
        // wie das Original: Anteil 0…1 (Einheit %) bzw. Pixel (Einheit px, `mode` „exact“)
        "roundness" => {
            let e = element(doc, id)?;
            let px = matches!(&e.kind, ElementKind::Path(p) if p.primitive.as_ref().is_some_and(|r| r.roundness_pixels == Some(true)));
            let v = a.f64().max(0.0);
            exec(Some(doc), "pi.roundness", json!({"ids": [id.0], "value": if px { v } else { v * 100.0 }, "pixels": px})).map(|_| ())
        }
        "mode" => {
            let px = a.str().unwrap_or_default().to_lowercase().contains("pixel");
            let e = element(doc, id)?;
            // gleiche Rundung, nur die Einheit wechselt
            let r = match &e.kind {
                ElementKind::Path(p) => p.primitive.as_ref().map(|pr| if px { fp_engine::commands::panel::primitive_roundness_px(pr) } else { pr.roundness * 100.0 }),
                _ => None,
            };
            match r {
                Some(r) => exec(Some(doc), "pi.roundness", json!({"ids": [id.0], "value": r, "pixels": px})).map(|_| ()),
                None => Ok(()),
            }
        }
        _ => ni(cls, &format!("{name} =")),
    }
}

fn node_set(name: &str, doc: DocId, id: ElementId, c: usize, n: usize, a: Arg) -> FwResult<()> {
    if name == "isSelectedPoint" {
        let mode = if a.bool() { "add" } else { "subtract" };
        return exec(Some(doc), "select.points", json!({"points": [{"element": id.0, "contour": c, "node": n}], "mode": mode})).map(|_| ());
    }
    let v = a.f64() as f32;
    edit_contours(doc, id, |cs| {
        let k = cs.get_mut(c).and_then(|k| k.nodes.get_mut(n)).ok_or_else(|| FwErr::Hr(hr::E_ARRAY_INDEX, "Knoten".into()))?;
        match name {
            "x" => k.anchor.x = v,
            "y" => k.anchor.y = v,
            "predX" => k.pred.x = v,
            "predY" => k.pred.y = v,
            "succX" => k.succ.x = v,
            "succY" => k.succ.y = v,
            "isCurvePoint" => k.curve = a.bool(),
            "randomSeed" | "name" | "dynamicInfo" => {}
            _ => return Err(FwErr::NotImpl(format!("ContourNode.{name} ="))),
        }
        Ok(())
    })
}

// ------------------------------------------------------------------ customData

/// Ändert ein Element direkt im Modell (Adapter für Eigenschaften ohne fp-engine-Befehl:
/// customData, Text-Rahmenart/Glättung/Kerning/Richtung) – als Verlaufsschritt.
fn edit_direct(doc: DocId, id: ElementId, f: impl FnOnce(&mut Element)) -> FwResult<()> {
    let path = st(|s| {
        let d = s.session.document(doc).ok_or_else(|| FwErr::bad("Dokument"))?;
        d.doc.find_element(id).ok_or_else(|| FwErr::bad("Element"))
    })?;
    // Verlaufsschritt über edit_doc (run_group)
    super::edit_doc(doc, |d| d.doc_mut().element_at_mut(&path).map(f).ok_or_else(|| FwErr::bad("Element")))?
}

fn edit_custom_data(doc: DocId, id: ElementId, f: impl FnOnce(&mut fp_model::DataDict)) -> FwResult<()> {
    edit_direct(doc, id, |e| f(&mut e.common.custom_data))
}

/// Texteigenschaft eines einzelnen Textobjekts über `text.setProperties` (Auswahl bleibt).
fn text_prop(doc: DocId, id: ElementId, cls: &str, name: &str, props: Value) -> FwResult<()> {
    if !matches!(element(doc, id)?.kind, ElementKind::Text(_)) {
        return Err(FwErr::NotImpl(format!("{cls}.{name} =")));
    }
    super::set_text_props(doc, Some(&[id]), props)
}

fn edit_text(doc: DocId, id: ElementId, cls: &str, name: &str, f: impl FnOnce(&mut fp_model::TextElem)) -> FwResult<()> {
    let e = element(doc, id)?;
    if !matches!(e.kind, ElementKind::Text(_)) {
        return Err(FwErr::NotImpl(format!("{cls}.{name} =")));
    }
    edit_direct(doc, id, |e| {
        if let ElementKind::Text(t) = &mut e.kind {
            f(t);
        }
    })
}

/// Füll-Handles eines Elements auf die Vorgabe (links Mitte → rechts Mitte, zweite Achse oben links).
fn default_fill_handles(doc: DocId, id: ElementId) -> FwResult<()> {
    let e = super::element(doc, id)?;
    let b = st(|s| {
        let d = s.session.document(doc).ok_or_else(|| FwErr::bad("Dokument"))?;
        Ok(fp_engine::docops::element_bounds(&d.doc, &e))
    })?;
    let Some(b) = b else { return Ok(()) };
    let h = fp_model::new::default_fill_handles(b);
    exec(Some(doc), "element.setFillHandles", json!({"ids": [id.0], "start": [h.start.x, h.start.y], "end": [h.end.x, h.end.y], "second": [h.second.x, h.second.y]})).map(|_| ())
}

pub(crate) fn json_to_data(v: &Value) -> DataValue {
    match v {
        Value::Null => DataValue::None,
        Value::Bool(b) => DataValue::Scalar(b.to_string()),
        Value::Number(x) => DataValue::Scalar(fp_model::data::format_g17(x.as_f64().unwrap_or(0.0))),
        Value::String(t) => DataValue::Scalar(t.clone()),
        Value::Array(a) => DataValue::List(fp_model::DataList { items: a.iter().map(json_to_data).collect(), ..Default::default() }),
        Value::Object(m) => DataValue::Dict(fp_model::DataDict {
            entries: m.iter().map(|(k, v)| fp_model::DataEntry { tag: Default::default(), key: k.clone(), value: json_to_data(v), text: None, extra: Default::default() }).collect(),
            ..Default::default()
        }),
    }
}

pub(crate) fn data_to_json(v: &DataValue) -> Value {
    match v {
        DataValue::None => Value::Null,
        DataValue::Scalar(t) => typed_scalar(t),
        DataValue::List(l) => Value::Array(l.items.iter().map(data_to_json).collect()),
        DataValue::Dict(d) => Value::Object(d.entries.iter().map(|e| (e.key.clone(), data_to_json(&e.value))).collect()),
        DataValue::Points(p) => Value::Array(p.points.iter().map(|q| json!({"x": q.x, "y": q.y})).collect()),
    }
}

fn set_custom_data_all(doc: DocId, id: ElementId, v: &Value) -> FwResult<()> {
    // Engine-Befehl; nur `null`-Werte (bleiben im Original als leerer Eintrag stehen, der Befehl
    // entfernt sie) gehen weiter direkt ins Modell
    if let Some(m) = v.as_object()
        && !m.values().any(Value::is_null)
    {
        // Elemente außerhalb der aktuellen Seite/des aktuellen Status erreicht der Befehl nicht
        if exec(Some(doc), "element.setCustomData", json!({"ids": [id.0], "data": v, "replace": true})).is_ok() {
            return Ok(());
        }
    }
    if !v.is_object() && !v.is_null() {
        return exec(Some(doc), "element.setCustomData", json!({"ids": [id.0], "data": {}, "replace": true})).map(|_| ());
    }
    let entries: Vec<(String, DataValue)> = v.as_object().map(|m| m.iter().map(|(k, x)| (k.clone(), json_to_data(x))).collect()).unwrap_or_default();
    edit_custom_data(doc, id, |cd| {
        cd.entries.clear();
        for (k, val) in entries {
            cd.set_structured(&k, val);
        }
    })
}

/// FwArray aus JSON-Liste.
fn fwarray(items: &[Value], ctx: &mut Context) -> FwResult<JsValue> {
    let mut vals = Vec::with_capacity(items.len());
    for it in items {
        vals.push(data_json_to_js(it, ctx)?);
    }
    let ctor = ctx.global_object().get(boa_engine::js_string!("FwArray"), ctx).map_err(jserr)?;
    let o = boa_engine::JsObject::with_object_proto(ctx.intrinsics());
    if let Some(c) = ctor.as_object()
        && let Some(p) = c.get(boa_engine::js_string!("prototype"), ctx).map_err(jserr)?.as_object()
    {
        o.set_prototype(Some(p));
    }
    for (i, v) in vals.into_iter().enumerate() {
        o.create_data_property_or_throw(i as u32, v, ctx).map_err(jserr)?;
    }
    o.define_property_or_throw(
        boa_engine::js_string!("length"),
        boa_engine::property::PropertyDescriptor::builder().value(items.len() as u32).writable(true).enumerable(false).configurable(false),
        ctx,
    )
    .map_err(jserr)?;
    Ok(o.into())
}

/// customData-Wert → JS (Listen als FwArray).
fn data_json_to_js(v: &Value, ctx: &mut Context) -> FwResult<JsValue> {
    match v {
        Value::Array(a) => fwarray(a, ctx),
        Value::Object(m) => {
            let o = boa_engine::JsObject::with_object_proto(ctx.intrinsics());
            for (k, x) in m {
                let jv = data_json_to_js(x, ctx)?;
                o.create_data_property_or_throw(boa_engine::JsString::from(k.as_str()), jv, ctx).map_err(jserr)?;
            }
            Ok(o.into())
        }
        other => conv::json_to_js(other, ctx).map_err(jserr),
    }
}

// ------------------------------------------------------------------ Proxys

/// Proxy-Lesezugriff; `None` = an das Ziel weiterreichen.
pub(super) fn proxy_get(h: &Handle, key: &str, ctx: &mut Context) -> FwResult<Option<JsValue>> {
    match h {
        Handle::Nodes { doc, id, c } => {
            let e = element(*doc, *id)?;
            let n = contours_of(&e).get(*c).map(|k| k.nodes.len()).unwrap_or(0);
            if key == "length" {
                return Ok(Some(JsValue::from(n as u32)));
            }
            if let Ok(i) = key.parse::<usize>() {
                if i < n {
                    return runtime::wrap(ctx, "ContourNode", Handle::Node { doc: *doc, id: *id, c: *c, n: i }).map(Some);
                }
                return Ok(Some(JsValue::undefined()));
            }
            Ok(None)
        }
        Handle::Contours { doc, id } => {
            let e = element(*doc, *id)?;
            let n = contours_of(&e).len();
            if key == "length" {
                return Ok(Some(JsValue::from(n as u32)));
            }
            if let Ok(i) = key.parse::<usize>() {
                if i < n {
                    return runtime::wrap(ctx, "Contour", Handle::Contour { doc: *doc, id: *id, c: i }).map(Some);
                }
                return Ok(Some(JsValue::undefined()));
            }
            Ok(None)
        }
        Handle::CustomData { doc, id } => {
            let e = element(*doc, *id)?;
            match e.common.custom_data.get(key) {
                Some(en) => {
                    let v = match &en.value {
                        DataValue::Scalar(t) => typed_scalar(t),
                        DataValue::None => match &en.text {
                            Some(t) => Value::String(t.clone()),
                            None => Value::Null,
                        },
                        other => data_to_json(other),
                    };
                    data_json_to_js(&v, ctx).map(Some)
                }
                None => Ok(None),
            }
        }
        _ => Ok(None),
    }
}

/// Proxy-Schreibzugriff; `false` = an das Ziel weiterreichen.
pub(super) fn proxy_set(h: &Handle, key: &str, value: &JsValue, ctx: &mut Context) -> FwResult<bool> {
    match h {
        Handle::Nodes { doc, id, c } => {
            if key == "length" {
                let n = value.as_number().unwrap_or(0.0).max(0.0) as usize;
                let c = *c;
                edit_contours(*doc, *id, |cs| {
                    if let Some(k) = cs.get_mut(c) {
                        let last = k.nodes.last().cloned().unwrap_or_else(|| fp_model::element::ContourNode::corner(fp_geom::Point::new(0.0, 0.0)));
                        k.nodes.resize(n.max(1), last);
                    }
                    Ok(())
                })?;
                return Ok(true);
            }
            Ok(false)
        }
        Handle::Contours { doc, id } => {
            if key == "length" {
                let n = value.as_number().unwrap_or(0.0).max(1.0) as usize;
                edit_contours(*doc, *id, |cs| {
                    let last = cs.last().cloned();
                    if let Some(l) = last {
                        cs.resize(n, l);
                    }
                    Ok(())
                })?;
                return Ok(true);
            }
            Ok(false)
        }
        Handle::CustomData { doc, id } => {
            let v = conv::js_to_json(value, ctx, 0).map_err(jserr)?;
            if v.is_null() {
                // `customData.x = null`: Eintrag bleibt (leer) stehen wie im Original
                let k = key.to_string();
                edit_custom_data(*doc, *id, move |cd| cd.set_structured(&k, DataValue::None))?;
            } else if exec(Some(*doc), "element.setCustomData", json!({"ids": [id.0], "data": {key: v}})).is_err() {
                // Element außerhalb der aktuellen Seite: direkt (ebenfalls ein Verlaufsschritt)
                let (k, dv) = (key.to_string(), json_to_data(&v));
                edit_custom_data(*doc, *id, move |cd| cd.set_structured(&k, dv))?;
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// Schlüssel eines Proxys (für `for…in`, `in`).
pub(super) fn proxy_keys(h: &Handle) -> Vec<String> {
    match h {
        Handle::CustomData { doc, id } => element(*doc, *id).map(|e| e.common.custom_data.entries.iter().map(|x| x.key.clone()).collect()).unwrap_or_default(),
        Handle::Nodes { doc, id, c } => {
            let n = element(*doc, *id).map(|e| contours_of(&e).get(*c).map(|k| k.nodes.len()).unwrap_or(0)).unwrap_or(0);
            (0..n).map(|i| i.to_string()).collect()
        }
        Handle::Contours { doc, id } => {
            let n = element(*doc, *id).map(|e| contours_of(&e).len()).unwrap_or(0);
            (0..n).map(|i| i.to_string()).collect()
        }
        _ => Vec::new(),
    }
}

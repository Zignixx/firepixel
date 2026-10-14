//! Document-DOM (`fw.getDocumentDOM()`): Methoden und Properties.

use super::{element, exec, wrap_element, wrap_ids};
use crate::bindings::hr;
use crate::conv::{self, Arg};
use crate::runtime;
use crate::state::{st, with_state, DefaultAttrs, FwErr, FwResult, Handle};
use crate::urls::{path_to_url, url_to_path};
use boa_engine::{Context, JsValue};
use fp_engine::DocId;
use fp_model::{ElementId, LayerSlot};
use serde_json::{json, Map, Value};
use std::path::Path;
use std::sync::Arc;

const CLS: &str = "Document";

pub(super) fn jserr(e: boa_engine::JsError) -> FwErr {
    FwErr::Js(e.to_string())
}

/// Aktuelle Seite, Status, Auswahl eines Dokuments.
pub(super) fn info(doc: DocId) -> FwResult<(usize, usize, Vec<ElementId>)> {
    st(|s| {
        let d = s.session.document(doc).ok_or_else(|| FwErr::bad(fp_i18n::t("err-script-doc-closed")))?;
        Ok((d.page, d.frame, d.selection.elements.clone()))
    })
}

pub(super) fn selection(doc: DocId) -> FwResult<Vec<ElementId>> {
    info(doc).map(|i| i.2)
}

/// Befehl auf der Auswahl; leere Auswahl → nichts tun (wie das Original bei Attribut-Methoden).
pub(super) fn on_selection(doc: DocId, name: &str, params: Value) -> FwResult<JsValue> {
    if selection(doc)?.is_empty() {
        return Ok(JsValue::undefined());
    }
    exec(Some(doc), name, params).map(|_| JsValue::undefined())
}

pub(super) fn defaults(doc: DocId) -> DefaultAttrs {
    with_state(|s| s.defaults.get(&doc.0).cloned().unwrap_or_default()).unwrap_or_default()
}

pub(super) fn set_defaults(doc: DocId, f: impl FnOnce(&mut DefaultAttrs)) -> FwResult<()> {
    with_state(|s| f(s.defaults.entry(doc.0).or_default()))
}

/// Füll- bzw. Strichtextur nach Namen (`""` = keine) auf die Auswahl, ohne Auswahl in die
/// Vorgaben für neue Objekte. `pi.fill`/`pi.stroke` laden die Maske mit
/// `fp_engine::resources::load_texture` aus den Texturordnern (ohne Datei bleibt nur der Name).
fn set_texture(doc: DocId, stroke: bool, name: &str) -> FwResult<()> {
    let name = name.trim().to_string();
    if selection(doc)?.is_empty() {
        return set_defaults(doc, |d| if stroke { d.stroke_texture = Some(name) } else { d.fill_texture = Some(name) });
    }
    exec(Some(doc), if stroke { "pi.stroke" } else { "pi.fill" }, json!({"texture": name}))?;
    Ok(())
}

/// Neue Form zeichnen und die Texturvorgaben (setFillTexture/setBrushTexture ohne Auswahl) anwenden.
fn draw_shape(doc: DocId, cmd: &str, p: Map<String, Value>, closed: bool) -> FwResult<()> {
    exec(Some(doc), cmd, shape_params(doc, p, closed))?;
    let d = defaults(doc);
    if let Some(t) = d.fill_texture.filter(|t| !t.is_empty() && closed) {
        exec(Some(doc), "pi.fill", json!({"texture": t}))?;
    }
    if let Some(t) = d.stroke_texture.filter(|t| !t.is_empty()) {
        exec(Some(doc), "pi.stroke", json!({"texture": t}))?;
    }
    Ok(())
}

/// Parameter für neue Formen aus den Vorgaben (setBrush/setFill ohne Auswahl).
pub(super) fn shape_params(doc: DocId, mut p: Map<String, Value>, closed: bool) -> Value {
    let d = defaults(doc);
    match d.fill {
        Some(None) => {
            p.insert("fill".into(), json!(false));
        }
        Some(Some(c)) if closed => {
            p.insert("fillColor".into(), json!(c));
        }
        _ => {}
    }
    match d.stroke {
        Some(None) => {
            p.insert("strokeWidth".into(), json!(0));
        }
        Some(Some((w, c))) => {
            p.insert("strokeWidth".into(), json!(w));
            if let Some(c) = c {
                p.insert("strokeColor".into(), json!(c));
            }
        }
        None => {}
    }
    Value::Object(p)
}

pub(super) fn rect_arr(r: [f64; 4]) -> Value {
    json!([r[0].min(r[2]), r[1].min(r[3]), r[0].max(r[2]), r[1].max(r[3])])
}

/// Fill-Objekt → Füllfarbe/Verlauf auf der Auswahl.
fn apply_fill(doc: DocId, fill: &Value, color: Option<String>) -> FwResult<()> {
    let sel = selection(doc)?;
    if fill.is_null() {
        if sel.is_empty() {
            return set_defaults(doc, |d| d.fill = Some(None));
        }
        exec(Some(doc), "element.setFill", json!({"kind": "none"}))?;
        return Ok(());
    }
    let grad_colors: Vec<String> = fill
        .get("gradient")
        .and_then(|g| g.get("nodes"))
        .and_then(|n| n.as_array())
        .map(|a| a.iter().filter_map(|n| n.get("color").and_then(|c| c.as_str()).and_then(conv::norm_color)).collect())
        .unwrap_or_default();
    if sel.is_empty() {
        let c = color.clone().or_else(|| grad_colors.first().cloned());
        return set_defaults(doc, |d| d.fill = Some(c));
    }
    if grad_colors.len() >= 2 {
        // Verlauf: waagerecht über die Auswahlgrenzen (Griffe setzt setFillVector)
        let b = bounds(doc, &sel)?.unwrap_or([0.0, 0.0, 1.0, 1.0]);
        let cy = (b[1] + b[3]) / 2.0;
        exec(Some(doc), "element.setGradient", json!({"from": [b[0], cy], "to": [b[2], cy], "colors": grad_colors}))?;
        with_state(|s| s.export_options.insert(gradient_key(doc), json!(grad_colors)))?;
        return Ok(());
    }
    let mut p = json!({"kind": "solid"});
    if let Some(c) = color {
        p["color"] = json!(c);
    }
    exec(Some(doc), "element.setFill", p)?;
    Ok(())
}

fn gradient_key(doc: DocId) -> u64 {
    doc.0 | (1u64 << 62)
}

/// Brush-Objekt → Strich auf der Auswahl.
fn apply_brush(doc: DocId, brush: &Value, color: Option<String>) -> FwResult<()> {
    // Prüfung wie im Original: unvollständige Pinsel-Objekte (ohne Durchmesser) werden abgelehnt
    // (EBadParam, Protokolle 07 T0/T2)
    if brush.is_object() && brush.get("diameter").is_none() {
        return Err(FwErr::Hr(hr::E_BAD_PARAM, "Pinsel ohne diameter".into()));
    }
    let sel = selection(doc)?;
    if brush.is_null() {
        if sel.is_empty() {
            return set_defaults(doc, |d| d.stroke = Some(None));
        }
        exec(Some(doc), "element.setStroke", json!({"width": 0}))?;
        return Ok(());
    }
    let w = brush.get("diameter").and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))).unwrap_or(1.0);
    if sel.is_empty() {
        return set_defaults(doc, |d| d.stroke = Some(Some((w, color))));
    }
    let mut p = json!({"width": w.max(0.0)});
    if let Some(c) = color {
        p["color"] = json!(c);
    }
    exec(Some(doc), "element.setStroke", p)?;
    Ok(())
}

/// Grenzen mehrerer Elemente.
pub(super) fn bounds(doc: DocId, ids: &[ElementId]) -> FwResult<Option<[f64; 4]>> {
    st(|s| {
        let d = s.session.document(doc).ok_or_else(|| FwErr::bad(fp_i18n::t("err-script-doc-closed")))?;
        Ok(fp_engine::docops::ids_bounds(&d.doc, d.page, ids).map(|r| [r.left as f64, r.top as f64, r.right as f64, r.bottom as f64]))
    })
}

/// Effektliste (Original-Objekt) → Effekte der Auswahl ersetzen.
pub(super) fn apply_effects(doc: DocId, ids: &[ElementId], list: &Value) -> FwResult<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let idv: Vec<u64> = ids.iter().map(|i| i.0).collect();
    let effects: Vec<Value> = list.get("effects").and_then(|e| e.as_array()).cloned().unwrap_or_default();
    st(|s| {
        // bestehende Effekte entfernen
        for id in ids {
            let n = s
                .session
                .document(doc)
                .and_then(|d| d.doc.find_element(*id).and_then(|p| d.doc.element_at(&p).cloned()))
                .and_then(|e| e.common.effects.as_ref().map(|l| l.effects.len()))
                .unwrap_or(0);
            for _ in 0..n {
                s.exec_on(Some(doc), "element.removeEffect", json!({"ids": [id.0], "index": 0}))?;
            }
        }
        for e in &effects {
            let Some(moa) = e.get("EffectMoaID").and_then(|v| v.as_str()) else { continue };
            let visible = e.get("EffectIsVisible").map(|v| v.as_bool().unwrap_or(v.as_str() != Some("false"))).unwrap_or(true);
            let mut params = Map::new();
            if let Some(o) = e.as_object() {
                for (k, v) in o {
                    if matches!(k.as_str(), "EffectMoaID" | "EffectIsVisible" | "category" | "name") {
                        continue;
                    }
                    let text = match v {
                        Value::String(t) => t.clone(),
                        Value::Bool(b) => b.to_string(),
                        Value::Number(x) => conv::fmt_num(x.as_f64().unwrap_or(0.0)),
                        Value::Null => continue,
                        // Punktlisten (Kurven `rgb_points` …) bleiben Listen → `GPLv` (FX-4)
                        Value::Array(_) => {
                            params.insert(k.clone(), v.clone());
                            continue;
                        }
                        other => other.to_string(),
                    };
                    params.insert(k.clone(), json!(text));
                }
            }
            s.exec_on(Some(doc), "element.addEffect", json!({"ids": idv, "effect": moa, "params": params, "visible": visible}))?;
        }
        Ok(())
    })
}

/// Bitmap-Filter aus einem Original-Effekt-Objekt (`filterSelection`).
fn filter_command(e: &Value) -> Option<(&'static str, Value)> {
    let num = |k: &str| e.get(k).and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.trim().parse().ok())));
    if let Some(r) = num("gaussian_blur_radius") {
        return Some(("filter.blur.gaussian", json!({"radius": r})));
    }
    if let (Some(h), Some(s), Some(l)) = (num("hue_amount"), num("saturation_amount"), num("lightness_amount")) {
        return Some(("filter.adjustColor.hueSaturation", json!({"hue": h, "saturation": s, "lightness": l})));
    }
    if let (Some(b), Some(c)) = (num("brightness_amount"), num("contrast_amount")) {
        return Some(("filter.adjustColor.brightnessContrast", json!({"brightness": b, "contrast": c})));
    }
    if num("noise_amount").is_some() {
        return Some(("filter.noise.addNoise", json!({"amount": num("noise_amount"), "use_color": e.get("noise_color").cloned()})));
    }
    use fp_model::EffectKind as K;
    let moa = e.get("EffectMoaID").and_then(|v| v.as_str()).unwrap_or("");
    let p = |pairs: &[(&str, &str)]| {
        let mut m = Map::new();
        for (from, to) in pairs {
            if let Some(v) = num(from) {
                m.insert((*to).into(), json!(v));
            }
        }
        Value::Object(m)
    };
    Some(match K::from_moa_id(moa) {
        K::Invert => ("filter.adjustColor.invert", json!({})),
        K::AutoLevels => ("filter.adjustColor.autoLevels", json!({})),
        K::Blur => ("filter.blur.blur", json!({})),
        K::BlurMore => ("filter.blur.blurMore", json!({})),
        K::GaussianBlur => ("filter.blur.gaussian", p(&[("gaussian_blur_radius", "radius")])),
        K::MotionBlur => ("filter.blur.motion", p(&[("motion_blur_angle", "angle"), ("motion_blur_distance", "distance")])),
        K::RadialBlur => ("filter.blur.radial", p(&[("radial_blur_amount", "amount"), ("radial_blur_quality", "quality")])),
        K::ZoomBlur => ("filter.blur.zoom", p(&[("zoom_blur_amount", "amount"), ("zoom_blur_quality", "quality")])),
        K::Sharpen => ("filter.sharpen.sharpen", json!({})),
        K::SharpenMore => ("filter.sharpen.sharpenMore", json!({})),
        K::UnsharpMask => ("filter.sharpen.unsharpMask", p(&[("unsharp_mask_radius", "radius"), ("unsharp_mask_amount", "amount"), ("unsharp_mask_threshold", "threshold")])),
        K::AddNoise => ("filter.noise.addNoise", p(&[("noise_amount", "amount")])),
        K::FindEdges => ("filter.other.findEdges", json!({})),
        K::ConvertToAlpha => ("filter.other.convertToAlpha", json!({})),
        K::BrightnessContrast => ("filter.adjustColor.brightnessContrast", p(&[("brightness_amount", "brightness"), ("contrast_amount", "contrast")])),
        K::HueSaturation => ("filter.adjustColor.hueSaturation", p(&[("hue_amount", "hue"), ("saturation_amount", "saturation"), ("lightness_amount", "lightness")])),
        _ => return None,
    })
}

/// FW-PNG-Kopie (ohne Pfadwechsel).
pub(super) fn save_copy(doc: DocId, path: &Path) -> FwResult<()> {
    st(|s| {
        let d = s.session.document(doc).ok_or_else(|| FwErr::bad(fp_i18n::t("err-script-doc-closed")))?;
        let (model, page) = (Arc::clone(&d.doc), d.page);
        let mut m = (*model).clone();
        if m.current_page != page && page < m.pages.len() {
            m.current_page = page;
            m.invalidate_root_copy();
        }
        let opts = fp_fwpng::WriteOptions { software: "Firepixel".into(), ..fp_fwpng::WriteOptions::default() };
        let bytes = match s.session.renderer() {
            Some(r) => match r.flattener() {
                Some(f) => fp_fwpng::write(&m, &opts, f),
                None => fp_fwpng::write(&m, &opts, &fp_engine::render::RendererFlattener(r.as_ref())),
            },
            None => fp_fwpng::write(&m, &opts, &fp_fwpng::NoFlattener),
        }
        .map_err(|e| FwErr::generic(e.to_string()))?;
        if let Some(dir) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir).map_err(|e| FwErr::generic(e.to_string()))?;
        }
        std::fs::write(path, bytes).map_err(|e| FwErr::generic(format!("{}: {e}", path.display())))
    })
}

/// Flacher Export der aktuellen Seite (PNG32 direkt, GIF/JPEG/WebP/… über fp-export).
pub(super) fn export_flat(doc: DocId, path: &Path, opts: &Value) -> FwResult<()> {
    st(|s| {
        crate::fileio::export_page(&s.session, doc, None, None, path, opts).map(|_| ()).map_err(|e| {
            if e.contains("nicht unterstützt") || e.contains("braucht fp-export") {
                FwErr::NotImpl(format!("Export: {e}"))
            } else {
                FwErr::generic(e)
            }
        })
    })
}

/// Bild-/Original-Datei importieren (Adapter: Pixel bzw. Objekte über die Zwischenablage einfügen).
fn import_file(doc: DocId, path: &Path, bounds: Option<[f64; 4]>, keep_aspect: bool) -> FwResult<()> {
    let bytes = std::fs::read(path).map_err(|e| FwErr::Hr(hr::E_FILE_NOT_FOUND, format!("{}: {e}", path.display())))?;
    if fp_fwpng::is_fwpng(&bytes) {
        let model = fp_fwpng::read(&bytes).map_err(|e| FwErr::Hr(0x8004_1421, e.to_string()))?;
        let els: Vec<Arc<fp_model::Element>> = model
            .pages
            .get(model.current_page.min(model.pages.len().saturating_sub(1)))
            .map(|p| {
                p.root
                    .layers
                    .iter()
                    .rev()
                    .filter_map(|l| match l {
                        LayerSlot::Own(l) if l.kind == fp_model::LayerKind::Normal => l.cel(0).cloned(),
                        _ => None,
                    })
                    .flat_map(|c| c.elements.clone())
                    .collect()
            })
            .unwrap_or_default();
        st(|s| {
            s.session.clipboard.clear();
            s.session.clipboard.elements = els;
            s.exec_on(Some(doc), "edit.paste", json!({}))?;
            Ok(())
        })?;
    } else {
        let (w, h, px) = crate::fileio::read_pixels(path).map_err(|e| FwErr::Hr(0x8004_1416, e))?;
        let (x, y) = bounds.map(|b| (b[0], b[1])).unwrap_or((0.0, 0.0));
        let rgba = fp_raster::RgbaImage::from_rgba_bytes(fp_geom::IRect::new(x.round() as i32, y.round() as i32, x.round() as i32 + w as i32, y.round() as i32 + h as i32), &px)
            .map_err(|e| FwErr::generic(e.to_string()))?;
        // wie im Original auf die sichtbaren Pixel (Alpha ≥ 2) beschneiden, Lage bleibt; ganz
        // transparentes Bild: kein Objekt (re §1.2 Regel 1)
        let full = rgba.bounds();
        let vis = rgba.alpha_bounds_min(fp_model::image::VISIBLE_ALPHA_MIN);
        if vis.is_empty() {
            return Ok(());
        }
        let rgba = if vis == full { rgba } else { rgba.cropped(vis) };
        st(|s| {
            s.session.clipboard.clear();
            s.session.clipboard.pixels = Some(rgba);
            s.exec_on(Some(doc), "edit.paste", json!({}))?;
            Ok(())
        })?;
        // Zielrechteck: das ganze Bild hineinskalieren, der beschnittene Inhalt behält seinen
        // (mitskalierten) Versatz; ohne Größenänderung steht er schon richtig
        if let Some(b) = bounds {
            let (tw, th) = (b[2] - b[0], b[3] - b[1]);
            let (fw, fh) = (w as f64, h as f64);
            if tw > 0.0 && th > 0.0 && ((fw - tw).abs() > 0.5 || (fh - th).abs() > 0.5) {
                let (kx, ky) = if keep_aspect {
                    let k = (tw / fw).min(th / fh);
                    (k, k)
                } else {
                    (tw / fw, th / fh)
                };
                let (ox, oy) = ((vis.left - full.left) as f64, (vis.top - full.top) as f64);
                let p = json!({"x": b[0] + ox * kx, "y": b[1] + oy * ky, "width": vis.width() as f64 * kx, "height": vis.height() as f64 * ky});
                exec(Some(doc), "element.setBounds", p)?;
            }
        }
        return Ok(());
    }
    // auf die Zielgrenzen bringen (das Original skaliert in das Rechteck)
    if let Some(b) = bounds {
        let sel = selection(doc)?;
        if let Some(cur) = super::doc::bounds(doc, &sel)? {
            let (cw, ch) = (cur[2] - cur[0], cur[3] - cur[1]);
            let (tw, th) = (b[2] - b[0], b[3] - b[1]);
            let mut p = json!({"x": b[0], "y": b[1]});
            if tw > 0.0 && th > 0.0 && cw > 0.0 && ch > 0.0 && ((cw - tw).abs() > 0.5 || (ch - th).abs() > 0.5) {
                let (w, h) = if keep_aspect {
                    let k = (tw / cw).min(th / ch);
                    (cw * k, ch * k)
                } else {
                    (tw, th)
                };
                p["width"] = json!(w);
                p["height"] = json!(h);
            }
            exec(Some(doc), "element.setBounds", p)?;
        }
    }
    Ok(())
}

/// Ebenen-ID zum Index der Ebenenliste (unten → oben); -1 = aktive.
pub(super) fn layer_id(doc: DocId, idx: i64) -> FwResult<Option<u64>> {
    st(|s| {
        let d = s.session.document(doc).ok_or_else(|| FwErr::bad(fp_i18n::t("err-script-doc-closed")))?;
        if idx < 0 {
            return Ok(d.layer.map(|l| l.0));
        }
        let p = d.doc.pages.get(d.page).ok_or_else(|| FwErr::bad("Seite"))?;
        match p.root.layers.get(idx as usize) {
            Some(LayerSlot::Own(l)) => Ok(Some(l.id.0)),
            Some(LayerSlot::Shared(r)) => Ok(d.doc.shared_layer(&r.id).and_then(|x| x.layer.as_ref()).map(|l| l.id.0)),
            Some(_) => Ok(None),
            None => Err(FwErr::Hr(hr::E_ARRAY_INDEX, format!("Ebene {idx} gibt es nicht"))),
        }
    })
}

/// IDs aller Ebenen der aktuellen Seite (eigene und freigegebene).
fn all_layer_ids(doc: DocId) -> FwResult<Vec<u64>> {
    let n = st(|s| Ok(s.session.document(doc).and_then(|d| d.doc.pages.get(d.page)).map(|p| p.root.layers.len()).unwrap_or(0)))?;
    let mut v = Vec::new();
    for i in 0..n {
        if let Some(id) = layer_id(doc, i as i64)? {
            v.push(id);
        }
    }
    Ok(v)
}

/// Index der aktiven Ebene.
pub(super) fn current_layer_index(doc: DocId) -> FwResult<i64> {
    st(|s| {
        let d = s.session.document(doc).ok_or_else(|| FwErr::bad(fp_i18n::t("err-script-doc-closed")))?;
        let Some(id) = d.layer else { return Ok(-1) };
        Ok(fp_engine::docops::layer_slot(&d.doc, d.page, id).map(|i| i as i64).unwrap_or(-1))
    })
}

pub(super) fn text_align_code(a: &str) -> Option<i64> {
    match a {
        "left" => Some(0),
        "center" | "centre" => Some(1),
        "right" => Some(2),
        "justify" => Some(3),
        "stretch" => Some(4),
        _ => None,
    }
}

/// `setTextRuns`: Text und Anfangsattribute.
fn set_text_runs(doc: DocId, runs: &Value) -> FwResult<()> {
    let text: String = runs
        .get("textRuns")
        .and_then(|r| r.as_array())
        .map(|a| a.iter().filter_map(|r| r.get("characters").and_then(|c| c.as_str())).collect::<Vec<_>>().join(""))
        .unwrap_or_default();
    if selection(doc)?.is_empty() {
        return Ok(());
    }
    if !text.is_empty() {
        exec(Some(doc), "text.setText", json!({"text": text}))?;
    }
    let mut attrs = super::elem::text_attrs_params(runs.get("initialAttrs"));
    if let (Value::Object(m), Some(Value::Object(first))) =
        (&mut attrs, runs.get("textRuns").and_then(|r| r.as_array()).and_then(|a| a.first()).map(|r| super::elem::text_attrs_params(r.get("changedAttrs"))))
    {
        m.extend(first);
    }
    if attrs.as_object().is_some_and(|m| !m.is_empty()) {
        exec(Some(doc), "text.setAttributes", attrs)?;
    }
    let aa = super::elem::text_aa_params(runs.get("initialAttrs"));
    if !aa.is_empty() {
        exec(Some(doc), "text.setProperties", Value::Object(aa))?;
    }
    Ok(())
}

pub(super) fn arg_str(args: &[Arg], i: usize) -> String {
    args.get(i).and_then(|a| a.str()).unwrap_or_default()
}

pub(super) fn method(doc: DocId, name: &str, args: &[Arg], ctx: &mut Context) -> FwResult<JsValue> {
    let a = |i: usize| args.get(i).cloned().unwrap_or(Arg::Null);
    let u = || Ok(JsValue::undefined());
    match name {
        // ---------------------------------------------------------- Erzeugen
        "addNewRectangle" | "addNewRectanglePrimitive" => {
            let r = a(0).rect().ok_or_else(|| FwErr::Hr(hr::E_BAD_PARAM_TYPE, "Rechteck".into()))?;
            let mut p = Map::new();
            p.insert("rect".into(), rect_arr(r));
            let round = a(1).f64();
            if round > 0.0 {
                p.insert("roundness".into(), json!(round.clamp(0.0, 1.0)));
            }
            draw_shape(doc, "draw.rectangle", p, true)?;
            u()
        }
        "addNewOval" => {
            let r = a(0).rect().ok_or_else(|| FwErr::Hr(hr::E_BAD_PARAM_TYPE, "Rechteck".into()))?;
            let mut p = Map::new();
            p.insert("rect".into(), rect_arr(r));
            draw_shape(doc, "draw.ellipse", p, true)?;
            u()
        }
        "addNewLine" => {
            let (x0, y0) = a(0).point().ok_or_else(|| FwErr::Hr(hr::E_BAD_PARAM_TYPE, "Punkt".into()))?;
            let (x1, y1) = a(1).point().ok_or_else(|| FwErr::Hr(hr::E_BAD_PARAM_TYPE, "Punkt".into()))?;
            let mut p = Map::new();
            p.insert("from".into(), json!([x0, y0]));
            p.insert("to".into(), json!([x1, y1]));
            draw_shape(doc, "draw.line", p, false)?;
            u()
        }
        "addNewStar" | "addNewPolygon" => {
            // addNewStar(numPoints, radius1, radius2, angle, center) bzw. Polygon
            let n = a(0).i64().clamp(3, 360);
            let r1 = a(1).f64();
            let c = args.iter().rev().find_map(|x| x.point()).unwrap_or((0.0, 0.0));
            let mut p = Map::new();
            p.insert("center".into(), json!([c.0, c.1]));
            p.insert("corner".into(), json!([c.0, c.1 - r1.abs().max(1.0)]));
            p.insert("sides".into(), json!(n));
            p.insert("star".into(), json!(name == "addNewStar"));
            draw_shape(doc, "draw.polygon", p, true)?;
            u()
        }
        "addNewText" => {
            let r = a(0).rect().ok_or_else(|| FwErr::Hr(hr::E_BAD_PARAM_TYPE, "Rechteck".into()))?;
            let mut p = json!({"text": " "});
            if (r[2] - r[0]).abs() > 2.0 {
                p["rect"] = rect_arr(r);
            } else {
                p["anchor"] = json!([r[0], r[1] + 12.0]);
            }
            exec(Some(doc), "draw.text", p)?;
            u()
        }
        "addNewImage" => {
            let r = a(0).rect().unwrap_or([0.0, 0.0, 100.0, 100.0]);
            let _ = exec(Some(doc), "select.none", json!({}));
            st(|s| {
                let img = fp_raster::RgbaImage::new(fp_geom::IRect::new(r[0] as i32, r[1] as i32, r[2].max(r[0] + 1.0) as i32, r[3].max(r[1] + 1.0) as i32)).map_err(|e| FwErr::generic(e.to_string()))?;
                s.session.clipboard.clear();
                s.session.clipboard.pixels = Some(img);
                s.exec_on(Some(doc), "edit.paste", json!({}))?;
                Ok(())
            })?;
            u()
        }
        "importFile" => {
            let path = super::fw::resolve_read(&url_to_path(&arg_str(args, 0)));
            import_file(doc, &path, a(1).rect(), a(2).bool())?;
            u()
        }
        "addNewHotspot" => {
            let r = args.iter().find_map(|x| x.rect()).unwrap_or([0.0, 0.0, 10.0, 10.0]);
            exec(Some(doc), "web.addHotspot", json!({"shape": "rect", "rect": rect_arr(r)}))?;
            u()
        }
        "addNewSlice" => {
            let r = args.iter().find_map(|x| x.rect()).unwrap_or([0.0, 0.0, 10.0, 10.0]);
            exec(Some(doc), "web.addSlice", json!({"shape": "rect", "rect": rect_arr(r)}))?;
            u()
        }
        "addNewLayer" => {
            let n = a(0).str();
            let mut p = json!({});
            if let Some(n) = n {
                p["name"] = json!(n);
            }
            exec(Some(doc), "layer.add", p)?;
            if a(1).bool()
                && let Some(id) = layer_id(doc, -1)?
            {
                exec(Some(doc), "layer.setShared", json!({"layer": id, "shared": true}))?;
            }
            Ok(conv::s(&with_state(|s| s.session.document(doc).and_then(|d| d.layer).map(|l| l.0.to_string()))?.unwrap_or_default()))
        }
        "addNewPage" => {
            let n = a(0).str();
            exec(Some(doc), "page.add", n.map(|n| json!({"name": n})).unwrap_or_else(|| json!({})))?;
            u()
        }
        "addFrames" | "duplicateFrame" => {
            let n = a(0).i64().clamp(1, 1000);
            exec(Some(doc), "state.add", json!({"count": n}))?;
            u()
        }
        // ---------------------------------------------------------- Auswahl
        "selectAll" => exec(Some(doc), "select.all", json!({})).map(|_| JsValue::undefined()),
        "selectNone" => exec(Some(doc), "select.none", json!({})).map(|_| JsValue::undefined()),
        "selectAllOnLayer" => {
            let idx = a(0).i64();
            let toggle = a(2).bool();
            let ids: Vec<u64> = st(|s| {
                let d = s.session.document(doc).ok_or_else(|| FwErr::bad("Dokument"))?;
                let page = d.doc.pages.get(d.page).ok_or_else(|| FwErr::bad("Seite"))?;
                let slot = if idx < 0 { d.layer.and_then(|l| fp_engine::docops::layer_slot(&d.doc, d.page, l)).unwrap_or(0) } else { idx as usize };
                Ok(match page.root.layers.get(slot) {
                    Some(LayerSlot::Own(l)) => l.cel(d.frame).map(|c| c.elements.iter().map(|e| e.id.0).collect()).unwrap_or_default(),
                    _ => Vec::new(),
                })
            })?;
            let mode = if toggle { "toggle" } else if a(1).bool() { "add" } else { "replace" };
            exec(Some(doc), "select.set", json!({"ids": ids, "mode": mode}))?;
            u()
        }
        "elementsAt" => {
            let r = a(0).rect().unwrap_or([0.0, 0.0, 0.0, 0.0]);
            let ids: Vec<ElementId> = st(|s| {
                let d = s.session.document(doc).ok_or_else(|| FwErr::bad("Dokument"))?;
                let all = fp_engine::docops::selectable_ids(&d.doc, d.page, d.frame);
                Ok(all
                    .into_iter()
                    .filter(|id| {
                        fp_engine::docops::element_on_page(&d.doc, d.page, *id)
                            .and_then(|e| fp_engine::docops::element_bounds(&d.doc, &e))
                            .is_some_and(|b| (b.left as f64) < r[2] && (b.right as f64) > r[0] && (b.top as f64) < r[3] && (b.bottom as f64) > r[1])
                    })
                    .collect())
            })?;
            wrap_ids(ctx, doc, &ids)
        }
        "deleteSelection" => on_selection(doc, "edit.clear", json!({})),
        "cloneSelection" => on_selection(doc, "edit.clone", json!({})),
        "duplicateSelection" => on_selection(doc, "edit.duplicate", json!({})),
        "clipCopy" => on_selection(doc, "edit.copy", json!({})),
        "clipCut" => on_selection(doc, "edit.cut", json!({})),
        "clipPaste" => exec(Some(doc), "edit.paste", json!({})).map(|_| JsValue::undefined()),
        "clipPasteAsMask" => on_selection(doc, "edit.pasteAsMask", json!({})),
        "clipPasteInside" => on_selection(doc, "edit.pasteInside", json!({})),
        "group" => {
            // gültige Gruppenarten wie im Original: "normal", "mask to path", "mask to image"
            let t = a(0).str().unwrap_or_else(|| "normal".into());
            match t.as_str() {
                "normal" => on_selection(doc, "modify.group", json!({})),
                "mask to path" | "mask to image" => {
                    on_selection(doc, "modify.mask.groupAsMask", json!({}))?;
                    let vector = t == "mask to path";
                    let ids = selection(doc)?;
                    super::edit_doc(doc, |d| {
                        let page = d.page;
                        for id in ids {
                            fp_engine::docops::edit_by_id(d.doc_mut(), page, id, |e| {
                                if let fp_model::ElementKind::Group(g) = &mut e.kind
                                    && g.kind.is_mask()
                                {
                                    g.kind = if vector { fp_model::element::GroupKind::VectorMask } else { fp_model::element::GroupKind::BitmapMask };
                                    e.cache = None;
                                }
                            });
                        }
                    })?;
                    Ok(JsValue::undefined())
                }
                other => Err(FwErr::Hr(hr::E_BAD_PARAM, format!("group: unbekannte Gruppenart „{other}“ (normal, mask to path, mask to image)"))),
            }
        }
        "ungroup" => on_selection(doc, "modify.ungroup", json!({})),
        "arrange" => {
            let c = match arg_str(args, 0).as_str() {
                "front" => "modify.arrange.bringToFront",
                "forward" => "modify.arrange.bringForward",
                "backward" => "modify.arrange.sendBackward",
                "back" => "modify.arrange.sendToBack",
                other => return Err(FwErr::bad(format!("arrange: {other}"))),
            };
            on_selection(doc, c, json!({}))
        }
        "align" => {
            let c = match arg_str(args, 0).as_str() {
                "left" => "modify.align.left",
                "right" => "modify.align.right",
                "top" => "modify.align.top",
                "bottom" => "modify.align.bottom",
                "center vertical" | "middle" => "modify.align.centerVertical",
                "center horizontal" | "center" => "modify.align.centerHorizontal",
                "space evenly vertical" | "distribute heights" => "modify.align.distributeHeights",
                "space evenly horizontal" | "distribute widths" => "modify.align.distributeWidths",
                other => return Err(FwErr::bad(format!("align: {other}"))),
            };
            on_selection(doc, c, json!({}))
        }
        // ---------------------------------------------------------- Geometrie
        "moveSelectionBy" => {
            let (dx, dy) = a(0).point().ok_or_else(|| FwErr::Hr(hr::E_BAD_PARAM_TYPE, "Punkt".into()))?;
            on_selection(doc, "edit.move", json!({"dx": dx, "dy": dy, "copy": a(1).bool()}))
        }
        "moveSelectionTo" => {
            let (x, y) = a(0).point().ok_or_else(|| FwErr::Hr(hr::E_BAD_PARAM_TYPE, "Punkt".into()))?;
            let sel = selection(doc)?;
            if let Some(b) = bounds(doc, &sel)? {
                exec(Some(doc), "edit.move", json!({"dx": x - b[0], "dy": y - b[1], "copy": a(1).bool()}))?;
            }
            u()
        }
        "setSelectionBounds" => {
            let r = a(0).rect().ok_or_else(|| FwErr::Hr(hr::E_BAD_PARAM_TYPE, "Rechteck".into()))?;
            on_selection(doc, "element.setBounds", json!({"x": r[0], "y": r[1], "width": (r[2] - r[0]).max(0.0), "height": (r[3] - r[1]).max(0.0)}))
        }
        "getSelectionBounds" => {
            let sel = selection(doc)?;
            match bounds(doc, &sel)? {
                Some(b) => conv::rect_js(b, ctx).map_err(jserr),
                None => Ok(JsValue::null()),
            }
        }
        "resizeSelection" => on_selection(doc, "element.setBounds", json!({"width": a(0).f64().max(0.0), "height": a(1).f64().max(0.0)})),
        "scaleSelection" => on_selection(doc, "modify.transform.numericTransform", json!({"mode": "scale", "scaleX": a(0).f64() * 100.0, "scaleY": a(1).f64() * 100.0, "constrain": false})),
        "rotateSelection" => on_selection(doc, "modify.transform.numericTransform", json!({"mode": "rotate", "angle": a(0).f64()})),
        "reflectSelection" => {
            let (h, v) = (a(0).bool(), a(1).bool());
            if h {
                on_selection(doc, "modify.transform.flipHorizontal", json!({}))?;
            }
            if v {
                on_selection(doc, "modify.transform.flipVertical", json!({}))?;
            }
            u()
        }
        "transformSelection" => {
            let m = a(0).json();
            let mat = m.get("matrix").cloned().unwrap_or(m);
            on_selection(doc, "transform.matrix", json!({"matrix": mat}))
        }
        "removeTransformation" => on_selection(doc, "modify.transform.removeTransformations", json!({})),
        "snapToPixel" => on_selection(doc, "modify.snapToPixel", json!({})),
        // ---------------------------------------------------------- Attribute
        "setFill" => {
            apply_fill(doc, &a(0).json(), None)?;
            u()
        }
        "setFillNColor" | "setFillNColorNTexture" => {
            let c = a(1).str().and_then(|c| conv::norm_color(&c));
            let fill = a(0).json();
            apply_fill(doc, &fill, c)?;
            if name == "setFillNColorNTexture"
                && !fill.is_null()
                && let Some(t) = a(2).str()
            {
                set_texture(doc, false, &t)?;
            }
            u()
        }
        "setFillColor" => {
            let c = a(0).str().and_then(|c| conv::norm_color(&c));
            if selection(doc)?.is_empty() {
                return set_defaults(doc, |d| d.fill = Some(c)).map(|_| JsValue::undefined());
            }
            exec(Some(doc), "element.setFill", json!({"color": c, "kind": "solid"}))?;
            u()
        }
        "setFillVector" | "setGradientFillHandles" => {
            let (p1, p2) = (a(0).point(), a(1).point());
            let colors = with_state(|s| s.export_options.get(&gradient_key(doc)).cloned())?;
            if let (Some(p1), Some(p2)) = (p1, p2)
                && !selection(doc)?.is_empty()
            {
                let mut p = json!({"from": [p1.0, p1.1], "to": [p2.0, p2.1]});
                if let Some(c) = colors {
                    p["colors"] = c;
                }
                exec(Some(doc), "element.setGradient", p)?;
            }
            u()
        }
        "setFillTexture" | "setBrushTexture" => {
            if let Some(t) = a(0).str() {
                set_texture(doc, name == "setBrushTexture", &t)?;
            }
            u()
        }
        "setFillEdgeMode" | "setFillPlacement" | "setDefaultFillVector" => u(),
        "setBrush" => {
            apply_brush(doc, &a(0).json(), None)?;
            u()
        }
        "setBrushNColor" | "setBrushNColorNTexture" => {
            let c = a(1).str().and_then(|c| conv::norm_color(&c));
            let brush = a(0).json();
            apply_brush(doc, &brush, c)?;
            if name == "setBrushNColorNTexture"
                && !brush.is_null()
                && let Some(t) = a(2).str()
            {
                set_texture(doc, true, &t)?;
            }
            u()
        }
        "setBrushColor" => {
            let c = a(0).str().and_then(|c| conv::norm_color(&c));
            on_selection(doc, "element.setStroke", json!({"color": c}))
        }
        "setBrushSize" => on_selection(doc, "element.setStroke", json!({"width": a(0).f64().max(0.0)})),
        "setBrushPlacement" => {
            let p = arg_str(args, 0);
            let pl = match p.as_str() {
                "inside" => "inside",
                "outside" => "outside",
                _ => "center",
            };
            on_selection(doc, "element.setStroke", json!({"placement": pl}))
        }
        "setOpacity" => on_selection(doc, "element.setOpacity", json!({"opacity": a(0).f64().clamp(0.0, 100.0)})),
        "setBlendMode" => on_selection(doc, "element.setBlendMode", json!({"mode": arg_str(args, 0)})),
        "applyEffects" => {
            let sel = selection(doc)?;
            apply_effects(doc, &sel, &a(0).json())?;
            u()
        }
        "filterSelectionByName" if a(1).str().is_some() => {
            // (Kategorie, Filtername) als Text, englisch oder deutsch wie im Filter-Menü
            let n = arg_str(args, 1).to_lowercase();
            let (c, p) = match n.trim_end_matches("...").trim_end_matches('…') {
                "sharpen" | "scharfzeichnen" => ("filter.sharpen.sharpen", json!({})),
                "sharpen more" | "stark scharfzeichnen" => ("filter.sharpen.sharpenMore", json!({})),
                "unsharp mask" | "unscharf maskieren" => ("filter.sharpen.unsharpMask", json!({"amount": 50, "radius": 1.0, "threshold": 0})),
                "blur" | "weichzeichnen" => ("filter.blur.blur", json!({})),
                "blur more" | "stark weichzeichnen" => ("filter.blur.blurMore", json!({})),
                "gaussian blur" | "gaußscher weichzeichner" | "gaussscher weichzeichner" => ("filter.blur.gaussian", json!({"radius": 1.0})),
                "invert" | "umkehren" => ("filter.adjustColor.invert", json!({})),
                "auto levels" | "auto-tonwertkorrektur" => ("filter.adjustColor.autoLevels", json!({})),
                "find edges" | "kanten suchen" => ("filter.other.findEdges", json!({})),
                "convert to alpha" | "in alpha konvertieren" => ("filter.other.convertToAlpha", json!({})),
                "add noise" | "rauschen hinzufügen" => ("filter.noise.addNoise", json!({"amount": 10})),
                other => return Err(FwErr::Hr(hr::E_BAD_PARAM, format!("filterSelectionByName: unbekannter Filter „{other}“"))),
            };
            on_selection(doc, c, p)
        }
        "filterSelection" | "filterSelectionByName" => {
            let e = if name == "filterSelection" { a(0).json() } else { a(1).json() };
            match filter_command(&e) {
                Some((c, p)) => on_selection(doc, c, p),
                None => Err(FwErr::NotImpl(format!("{CLS}.{name} für diesen Filter"))),
            }
        }
        "setElementName" => on_selection(doc, "element.setName", json!({"name": arg_str(args, 0)})),
        "setElementVisible" | "setElementLocked" => {
            // (frameNum, layerNum, objectIndex, flag)
            let (f, l, o, flag) = (a(0).i64(), a(1).i64(), a(2).i64(), a(3).bool());
            let id = st(|s| {
                let d = s.session.document(doc).ok_or_else(|| FwErr::bad("Dokument"))?;
                let page = d.doc.pages.get(d.page).ok_or_else(|| FwErr::bad("Seite"))?;
                let slot = if l < 0 { d.layer.and_then(|x| fp_engine::docops::layer_slot(&d.doc, d.page, x)).unwrap_or(0) } else { l as usize };
                let frame = if f < 0 { d.frame } else { f as usize };
                match page.root.layers.get(slot) {
                    Some(LayerSlot::Own(ly)) => ly.cel(frame).and_then(|c| c.elements.get(o.max(0) as usize)).map(|e| e.id.0).ok_or_else(|| FwErr::Hr(hr::E_ARRAY_INDEX, "Objektindex".into())),
                    _ => Err(FwErr::Hr(hr::E_ARRAY_INDEX, "Ebene".into())),
                }
            })?;
            let c = if name == "setElementVisible" { "element.setVisible" } else { "element.setLocked" };
            let key = if name == "setElementVisible" { "visible" } else { "locked" };
            exec(Some(doc), c, json!({"ids": [id], key: flag}))?;
            u()
        }
        "hideSelection" => on_selection(doc, "element.setVisible", json!({"visible": false})),
        "showAllHidden" => {
            let ids: Vec<u64> = st(|s| {
                let d = s.session.document(doc).ok_or_else(|| FwErr::bad("Dokument"))?;
                Ok(fp_engine::docops::selectable_ids(&d.doc, d.page, d.frame).into_iter().map(|i| i.0).collect())
            })?;
            if !ids.is_empty() {
                exec(Some(doc), "element.setVisible", json!({"ids": ids, "visible": true}))?;
            }
            u()
        }
        "lockSelection" => on_selection(doc, "modify.lockSelection", json!({})),
        "selectFeather" => {
            // ohne Pixelauswahl wirkungslos (wie das Original)
            let has = with_state(|s| s.session.document(doc).is_some_and(|d| d.selection.has_pixels()))?;
            if has {
                exec(Some(doc), "select.feather", json!({"radius": a(0).i64().clamp(1, 1000)}))?;
            }
            u()
        }
        "selectInverse" => exec(Some(doc), "select.inverse", json!({})).map(|_| JsValue::undefined()),
        "setSelectionMask" => {
            if a(0).is_null() {
                exec(Some(doc), "select.deleteBitmapSelection", json!({})).map(|_| JsValue::undefined())
            } else {
                Err(FwErr::NotImpl(format!("{CLS}.{name} mit Maske")))
            }
        }
        "setAllLayersDisclosure" => {
            super::set_layers_expanded(doc, None, a(0).bool())?;
            u()
        }
        "setTextAntiAliasing" => {
            let code = match arg_str(args, 0).as_str() {
                "none" => 0,
                "crisp" => 1,
                "strong" => 2,
                "custom" => 4,
                _ => 3,
            };
            super::set_text_props(doc, None, json!({"antialias": super::aa_name(code)}))?;
            u()
        }
        "setTextOnPathMode" => {
            let mode = match arg_str(args, 0).as_str() {
                "rotate" => 0,
                "vertical" => 1,
                "skew" => 2,
                "skewvert" | "skew vertical" => 3,
                _ => 0,
            };
            super::edit_selected_texts(doc, |t| {
                if let Some(op) = &mut t.on_path {
                    op.mode = Some(mode);
                }
            })?;
            u()
        }
        "setTextOnPathOffset" => {
            let off = a(0).f64() as f32;
            super::edit_selected_texts(doc, |t| {
                if let Some(op) = &mut t.on_path {
                    op.offset = Some(off);
                }
            })?;
            u()
        }
        // ---------------------------------------------------------- Text
        "setTextRuns" => {
            set_text_runs(doc, &a(0).json())?;
            u()
        }
        "setTextAlignment" => match text_align_code(&arg_str(args, 0)) {
            Some(al) => on_selection(doc, "text.setAttributes", json!({"alignment": al})),
            None => u(),
        },
        "setTextAutoKern" => {
            let v = a(0).bool();
            super::set_text_props(doc, None, json!({"autoKern": v}))?;
            u()
        }
        "setTextAutoExpand" => {
            let v = a(0).bool();
            super::set_text_props(doc, None, json!({"frameType": if v { "auto" } else { "fixed" }}))?;

            u()
        }
        "setTextOrientation" => {
            let code = super::orientation_code_from_jsf(&arg_str(args, 0));
            super::set_text_props(doc, None, json!({"orientation": super::orientation_name(code)}))?;
            u()
        }
        "setTextCustomAntiAliasSharpness" | "setTextCustomAntiAliasStrength" | "setTextCustomAntiAliasOverSample" => {
            let v = a(0).i64();
            let key = match name {
                "setTextCustomAntiAliasSharpness" => "sharpness",
                "setTextCustomAntiAliasStrength" => "strength",
                _ => "oversample",
            };
            let v = if key == "oversample" { v.clamp(1, 64) } else { v.clamp(0, 255) };
            super::set_text_props(doc, None, json!({key: v}))?;
            u()
        }
        "attachTextToPath" => on_selection(doc, "text.attachToPath", json!({})),
        "placeTextInPath" | "attachTextInPath" => on_selection(doc, "text.attachInPath", json!({})),
        "detachTextFromPath" => on_selection(doc, "text.detachFromPath", json!({})),
        "reversePathTextDirection" => on_selection(doc, "text.reverseDirection", json!({})),
        // ---------------------------------------------------------- Pfade
        "pathUnion" => on_selection(doc, "modify.combinePaths.union", json!({})),
        "pathPunch" => on_selection(doc, "modify.combinePaths.punch", json!({})),
        "pathIntersect" => on_selection(doc, "modify.combinePaths.intersect", json!({})),
        "pathCrop" => on_selection(doc, "modify.combinePaths.crop", json!({})),
        "joinPaths" => on_selection(doc, "modify.combinePaths.join", json!({})),
        "splitPaths" => on_selection(doc, "modify.combinePaths.split", json!({})),
        "pathSimplify" => on_selection(doc, "modify.alterPath.simplify", json!({"amount": a(0).f64().max(0.0)})),
        "pathExpand" => {
            let mut p = json!({"width": a(0).f64().max(0.1)});
            let miter = a(1).f64();
            if miter >= 1.0 {
                p["miter"] = json!(miter.min(10.0));
            }
            let cap = arg_str(args, 2);
            if ["butt", "square", "round"].contains(&cap.as_str()) {
                p["cap"] = json!(cap);
            }
            let join = arg_str(args, 3);
            if ["bevel", "round", "miter"].contains(&join.as_str()) {
                p["join"] = json!(join);
            }
            on_selection(doc, "modify.alterPath.expandStroke", p)
        }
        "pathInset" => on_selection(doc, "modify.alterPath.insetPath", json!({"width": a(0).f64()})),
        // mode: 0 hart, 1 Anti-Alias, 2 weiche Kante (mit featherAmount), wie der Dialog D32
        "convertPathToMarquee" => {
            let edge = match a(0).i64() {
                0 if !args.is_empty() => "hard",
                2 => "feather",
                _ => "antiAlias",
            };
            on_selection(doc, "modify.convertPathToMarquee", json!({"edge": edge, "feather": a(1).i64().clamp(0, 1000)}))
        }
        "flattenSelection" => on_selection(doc, "modify.flattenSelection", json!({})),
        "mergeDown" => exec(Some(doc), "modify.mergeDown", json!({})).map(|_| JsValue::undefined()),
        "flattenDocument" | "flattenLayers" => exec(Some(doc), "modify.flattenLayers", json!({})).map(|_| JsValue::undefined()),
        "convertToSymbol" => {
            let kind = arg_str(args, 0).to_lowercase();
            if !["graphic", "button", "animation"].contains(&kind.as_str()) {
                return Ok(JsValue::null());
            }
            on_selection(doc, "modify.symbol.convertToSymbol", json!({"kind": kind, "name": arg_str(args, 1)}))
        }
        // ---------------------------------------------------------- Ebenen, Seiten, Status
        "setLayerName" => {
            let id = layer_id(doc, a(0).i64())?.ok_or_else(|| FwErr::bad("Ebene"))?;
            exec(Some(doc), "layer.rename", json!({"layer": id, "name": arg_str(args, 1)}))?;
            u()
        }
        // (layerIndex, frameIndex, value, allLayers): mit allLayers = true jede Ebene der Seite,
        // layerIndex wird dann wie im Original (re\03, fw_api) ignoriert
        "setLayerVisible" | "setLayerLocked" => {
            let ids = if a(3).bool() { all_layer_ids(doc)? } else { vec![layer_id(doc, a(0).i64())?.ok_or_else(|| FwErr::bad("Ebene"))?] };
            for id in ids {
                if name == "setLayerVisible" {
                    exec(Some(doc), "layer.setVisible", json!({"layer": id, "visible": a(2).bool()}))?;
                } else {
                    exec(Some(doc), "layer.setLocked", json!({"layer": id, "locked": a(2).bool()}))?;
                }
            }
            u()
        }
        "setLayerSharing" => {
            let id = layer_id(doc, a(0).i64())?.ok_or_else(|| FwErr::bad("Ebene"))?;
            exec(Some(doc), "layer.setShared", json!({"layer": id, "shared": arg_str(args, 1) == "shared"}))?;
            u()
        }
        "deleteLayer" => {
            let id = layer_id(doc, a(0).i64())?.ok_or_else(|| FwErr::bad("Ebene"))?;
            exec(Some(doc), "layer.delete", json!({"layer": id}))?;
            u()
        }
        "changeCurrentPage" => exec(Some(doc), "page.setCurrent", json!({"index": a(0).i64().max(0)})).map(|_| JsValue::undefined()),
        "setPageName" => exec(Some(doc), "page.rename", json!({"index": a(0).i64().max(0), "name": arg_str(args, 1)})).map(|_| JsValue::undefined()),
        "deletePageAt" => exec(Some(doc), "page.delete", json!({"index": a(0).i64().max(0)})).map(|_| JsValue::undefined()),
        "duplicatePage" => exec(Some(doc), "page.duplicate", json!({"index": a(0).i64().max(0)})).map(|_| JsValue::undefined()),
        "deleteFrames" => exec(Some(doc), "state.delete", json!({"index": a(0).i64().max(0)})).map(|_| JsValue::undefined()),
        "setDocumentCanvasSize" => {
            // Rechteck relativ zur bisherigen Leinwand: Verankerung aus den unveränderten Kanten
            let r = a(0).rect().ok_or_else(|| FwErr::Hr(hr::E_BAD_PARAM_TYPE, "Rechteck".into()))?;
            let (w0, h0) = with_state(|s| s.session.document(doc).and_then(|d| d.doc.pages.get(d.page).map(|p| (p.canvas.width as f64, p.canvas.height as f64))))?.unwrap_or((0.0, 0.0));
            let hx = if r[0].abs() < 0.5 { "left" } else if (r[2] - w0).abs() < 0.5 { "right" } else { "" };
            let vy = if r[1].abs() < 0.5 { "top" } else if (r[3] - h0).abs() < 0.5 { "bottom" } else { "" };
            let anchor = match (vy, hx) {
                ("top", "left") => "topLeft",
                ("top", "right") => "topRight",
                ("top", _) => "top",
                ("bottom", "left") => "bottomLeft",
                ("bottom", "right") => "bottomRight",
                ("bottom", _) => "bottom",
                (_, "left") => "left",
                (_, "right") => "right",
                _ => "center",
            };
            exec(Some(doc), "modify.canvas.canvasSize", json!({"width": (r[2] - r[0]).max(1.0), "height": (r[3] - r[1]).max(1.0), "anchor": anchor}))?;
            u()
        }
        "setDocumentCanvasColor" => {
            let c = a(0).str().and_then(|c| conv::norm_color(&c)).unwrap_or_else(|| "#ffffff".into());
            exec(Some(doc), "modify.canvas.canvasColor", json!({"color": c}))?;
            u()
        }
        "setDocumentImageSize" => {
            let r = a(0).rect().ok_or_else(|| FwErr::Hr(hr::E_BAD_PARAM_TYPE, "Rechteck".into()))?;
            exec(Some(doc), "modify.canvas.imageSize", json!({"width": (r[2] - r[0]).max(1.0), "height": (r[3] - r[1]).max(1.0), "constrain": false}))?;
            u()
        }
        "setDocumentResolution" => u(),
        "fitCanvas" => exec(Some(doc), "modify.canvas.fitCanvas", json!({})).map(|_| JsValue::undefined()),
        "trimCanvas" => exec(Some(doc), "modify.canvas.trimCanvas", json!({})).map(|_| JsValue::undefined()),
        "rotateDocument" => {
            let deg = a(0).i64().rem_euclid(360);
            let c = match deg {
                90 => "modify.canvas.rotate90DegreeCw",
                180 => "modify.canvas.rotate180Degree",
                270 => "modify.canvas.rotate90DegreeCcw",
                _ => return u(),
            };
            exec(Some(doc), c, json!({})).map(|_| JsValue::undefined())
        }
        // ---------------------------------------------------------- Datei
        "close" => super::close_doc(doc).map(|_| JsValue::undefined()),
        "save" => super::fw::save_doc(doc, None).map(|_| JsValue::undefined()),
        "makeActive" => {
            st(|s| s.session.activate(doc).map_err(FwErr::from))?;
            u()
        }
        "exportTo" => {
            let path = url_to_path(&arg_str(args, 0));
            let mut opts = a(1).json();
            if opts.is_null() {
                opts = with_state(|s| s.export_options.get(&doc.0).cloned())?.unwrap_or(Value::Null);
            }
            export_flat(doc, &path, &opts)?;
            Ok(JsValue::from(true))
        }
        "setExportOptions" => {
            let v = a(0).json();
            with_state(|s| s.export_options.insert(doc.0, v))?;
            u()
        }
        "undo" | "redo" => {
            st(|s| {
                s.session.activate(doc).map_err(FwErr::from)?;
                if name == "undo" { s.session.undo() } else { s.session.redo() }.map_err(|e| FwErr::generic(e.to_string()))
            })?;
            u()
        }
        _ => super::doc2::method(doc, name, args, ctx),
    }
}

pub(super) fn get(doc: DocId, name: &str, ctx: &mut Context) -> FwResult<JsValue> {
    let snap = st(|s| {
        let d = s.session.document(doc).ok_or_else(|| FwErr::bad(fp_i18n::t("err-script-doc-closed")))?;
        Ok((Arc::clone(&d.doc), d.page, d.frame, d.path.clone(), d.title.clone(), d.is_dirty(), d.layer))
    });
    let (model, page, frame, path, title, dirty, _layer) = match snap {
        Ok(v) => v,
        Err(e) if name == "isValid" => {
            let _ = e;
            return Ok(JsValue::from(false));
        }
        Err(e) => return Err(e),
    };
    let pg = model.pages.get(page).cloned().ok_or_else(|| FwErr::bad("Seite"))?;
    Ok(match name {
        "isValid" => JsValue::from(true),
        "width" => conv::n(pg.canvas.width as f64),
        "height" => conv::n(pg.canvas.height as f64),
        "left" => conv::n(pg.canvas.origin.x as f64),
        "top" => conv::n(pg.canvas.origin.y as f64),
        "resolution" => conv::n(pg.canvas.resolution as f64),
        "resolutionUnits" => conv::s(if pg.canvas.resolution_unit == 1 { "cm" } else { "inch" }),
        "backgroundColor" => conv::s(&fp_engine::commands::argb_hex(pg.root.background)),
        "filePathForSave" | "filePathForRevert" => path.map(|p| conv::s(&path_to_url(&p))).unwrap_or_else(JsValue::null),
        "docTitleWithoutExtension" => conv::s(title.trim_end_matches(".png").trim_end_matches(".fw").trim_end_matches(".jpg").trim_end_matches(".gif")),
        "isDirty" => JsValue::from(dirty),
        "pagesCount" => conv::n(model.pages.len() as f64),
        "currentPageNum" => conv::n(page as f64),
        "pageName" => conv::s(pg.display_name()),
        "currentFrameNum" => conv::n(frame as f64),
        "frameCount" => conv::n(pg.root.frame_count.max(1) as f64),
        "currentLayerNum" => conv::n(current_layer_index(doc)? as f64),
        "isSymbolDocument" | "inLaunchAndEdit" | "isPaintMode" | "gammaPreview" => JsValue::from(false),
        "useMatteColor" => JsValue::from(false),
        "matteColor" => conv::s("#ffffff"),
        "onionSkinBefore" => conv::n(pg.root.onion_before as f64),
        "onionSkinAfter" => conv::n(pg.root.onion_after as f64),
        "frameLoopingCount" => conv::n(pg.loop_count() as f64),
        "layers" | "topLayers" => {
            // `layers` flach mit Unterebenen wie im Original (Befund F2-1), `topLayers` nur die oberste Stufe
            let list: Vec<(usize, Option<fp_model::ElementId>)> = if name == "layers" { super::elem::flat_layers(doc, page)? } else { (0..pg.root.layers.len()).map(|s| (s, None)).collect() };
            let mut v = Vec::new();
            for (slot, sub) in list {
                v.push(runtime::wrap(ctx, "Layer", Handle::Layer { doc, page, slot, sub })?);
            }
            conv::array(v, ctx)
        }
        "frames" => {
            let n = pg.root.frame_count.max(1) as usize;
            let mut v = Vec::new();
            for f in 0..n {
                v.push(runtime::wrap(ctx, "Frame", Handle::Frame { doc, page, frame: f })?);
            }
            conv::array(v, ctx)
        }
        "pathAttributes" => {
            let d = defaults(doc);
            let fill = match d.fill {
                Some(None) => JsValue::null(),
                _ => conv::json_to_js(&json!({"category": "fc_Solid", "name": "fn_Normal", "shape": "solid"}), ctx).map_err(jserr)?,
            };
            let fc = match &d.fill {
                Some(Some(c)) => c.clone(),
                _ => with_state(|s| fp_engine::commands::argb_hex(s.session.tools.fill_color))?,
            };
            let sc = with_state(|s| fp_engine::commands::argb_hex(s.session.tools.stroke_color))?;
            conv::object(vec![("fill", fill), ("fillColor", conv::s(&fc)), ("brushColor", conv::s(&sc)), ("brush", JsValue::null())], ctx).map_err(jserr)?
        }
        "exportOptions" | "exportSettings" | "exportFormatOptions" => {
            // folgt den Exporteinstellungen der Seite (Exportformat im Dokument-Inspektor,
            // Optimieren-Bedienfeld; PAR 16/08); per Skript gesetzte Werte gelten, solange das
            // Format dazu passt
            // wie `fp_export::doc_export::page_settings`: Exporteinstellungen der Seite (ältere Dateien:
            // `legacy_export` zuerst)
            let page = with_state(|s| s.session.document(doc).and_then(|d| d.doc.pages.get(d.page).map(|p| p.legacy_export.as_ref().or(p.root.export.as_ref()).map(|e| (**e).clone()))))?.flatten();
            let mut v = export_options_json(page.as_ref());
            if let Some(Value::Object(set)) = with_state(|s| s.export_options.get(&doc.0).cloned())?
                && set.get("exportFormat").is_none_or(|f| Some(f) == v.get("exportFormat"))
                && let Value::Object(m) = &mut v
            {
                for (k, x) in set {
                    m.insert(k, x);
                }
            }
            let o = conv::json_to_js(&v, ctx).map_err(jserr)?;
            // ExportOptions.loadColorPalette(url) / saveColorPalette(url): Palette des Optimieren-Bedienfelds
            if let Some(obj) = o.as_object() {
                for (fname, cmd) in [("loadColorPalette", "panel.optimize.loadPalette"), ("saveColorPalette", "panel.optimize.savePalette")] {
                    let f = boa_engine::NativeFunction::from_copy_closure_with_captures(
                        |_, args, caps: &(u64, &'static str), ctx| {
                            let url = args.first().map(|a| a.to_string(ctx).map(|s| s.to_std_string_escaped())).transpose()?.unwrap_or_default();
                            let path = super::fw::resolve_read(&crate::urls::url_to_path(&url));
                            let r = exec(Some(DocId(caps.0)), caps.1, json!({"path": path.to_string_lossy()}));
                            match r {
                                Ok(_) => Ok(JsValue::from(true)),
                                Err(e) => Err(runtime::throw(e, caps.1)),
                            }
                        },
                        (doc.0, cmd),
                    );
                    let func = boa_engine::object::FunctionObjectBuilder::new(ctx.realm(), f).name(boa_engine::JsString::from(fname)).build();
                    obj.set(boa_engine::JsString::from(fname), func, false, ctx).map_err(jserr)?;
                }
            }
            o
        }
        "pngText" => {
            let mut m = Map::new();
            for (k, v) in &model.file_meta.text_chunks {
                m.insert(k.clone(), json!(v));
            }
            conv::json_to_js(&Value::Object(m), ctx).map_err(jserr)?
        }
        "lastExportDirectory" | "lastExportFile" | "defaultAltText" | "mapType" | "backgroundUrl" | "VectorOperation" => conv::s(""),
        _ => return super::doc2::get(doc, name, ctx),
    })
}

pub(super) fn set(doc: DocId, name: &str, a: Arg, raw: &JsValue, ctx: &mut Context) -> FwResult<()> {
    let _ = raw;
    match name {
        "currentLayerNum" => {
            if let Some(id) = layer_id(doc, a.i64())? {
                exec(Some(doc), "layer.setActive", json!({"layer": id}))?;
            }
            Ok(())
        }
        "currentFrameNum" => exec(Some(doc), "state.setCurrent", json!({"index": a.i64().max(0)})).map(|_| ()),
        "currentPageNum" => exec(Some(doc), "page.setCurrent", json!({"index": a.i64().max(0)})).map(|_| ()),
        "pageName" => {
            let i = with_state(|s| s.session.document(doc).map(|d| d.page).unwrap_or(0))?;
            exec(Some(doc), "page.rename", json!({"index": i, "name": a.str().unwrap_or_default()})).map(|_| ())
        }
        "backgroundColor" => {
            let c = a.str().and_then(|c| conv::norm_color(&c)).unwrap_or_else(|| "#ffffff".into());
            exec(Some(doc), "modify.canvas.canvasColor", json!({"color": c})).map(|_| ())
        }
        "width" | "height" => {
            let (w, h) = with_state(|s| s.session.document(doc).and_then(|d| d.doc.pages.get(d.page).map(|p| (p.canvas.width as f64, p.canvas.height as f64))))?.unwrap_or((1.0, 1.0));
            let (w, h) = if name == "width" { (a.f64(), h) } else { (w, a.f64()) };
            exec(Some(doc), "modify.canvas.canvasSize", json!({"width": w.max(1.0), "height": h.max(1.0)})).map(|_| ())
        }
        "exportOptions" | "exportSettings" => {
            let v = a.json();
            with_state(|s| {
                s.export_options.insert(doc.0, v);
            })
        }
        "isDirty" | "filePathForSave" | "resolution" | "resolutionUnits" | "useMatteColor" | "matteColor" | "gammaPreview" | "defaultAltText" | "mapType" | "lastExportFile"
        | "lastExportDirectory" | "onionSkinBefore" | "onionSkinAfter" | "frameLoopingCount" | "backgroundUrl" | "VectorOperation" | "pngText" => Ok(()),
        _ => super::doc2::set(doc, name, a, ctx),
    }
}

/// `dom.exportOptions` aus den Exporteinstellungen der Seite (ohne Einstellungen: PNG 32 Bit wie
/// ein neues Dokument). `name` ist der Name der eingebauten Vorgabe, wenn eine passt.
pub(super) fn export_options_json(e: Option<&fp_model::ExportSettings>) -> Value {
    let Some(e) = e else {
        return json!({"exportFormat": "PNG", "colorMode": "32 bit", "jpegQuality": 80, "numEntriesRequested": 0, "webSnapAdaptive": false, "name": "PNG32"});
    };
    let format = match e.format {
        0 => "GIF",
        1 => "JPEG",
        2 => "PNG",
        4 => "Animated GIF",
        _ => "Custom",
    };
    let mode = match e.color_mode {
        0 => "indexed",
        1 => "24 bit",
        _ => "32 bit",
    };
    let palette = match e.palette_mode {
        1 => "adaptive",
        2 => "uniform",
        3 => "grayscale",
        4 => "macintosh",
        5 => "windows",
        6 => "exact",
        7 => "web216",
        _ => "custom",
    };
    let gif = |n: i32| e.color_mode == 0 && e.entries_requested == n;
    let name = match (e.format, e.color_mode) {
        (2, 2) => "PNG32",
        (1, _) if e.jpeg_quality >= 80 => "JPEG - Höhere Qualität",
        (1, _) => "JPEG - Kleinere Datei",
        (0, _) if e.palette_mode == 7 => "GIF Web 216",
        (0, _) if e.palette_mode == 1 && e.websnap_adaptive && gif(256) => "GIF WebSnap 256",
        (0, _) if e.palette_mode == 1 && e.websnap_adaptive && gif(128) => "GIF WebSnap 128",
        (0, _) if e.palette_mode == 1 && !e.websnap_adaptive && gif(256) => "GIF Adaptiv 256",
        (4, _) if e.palette_mode == 1 && e.websnap_adaptive && gif(128) => "Animiertes GIF WebSnap 128",
        _ => "",
    };
    json!({
        "exportFormat": format, "colorMode": mode, "paletteMode": palette, "numEntriesRequested": e.entries_requested,
        "webSnapAdaptive": e.websnap_adaptive, "webSnapTolerance": e.websnap_tolerance, "jpegQuality": e.jpeg_quality,
        "lossyGifAmount": e.lossy, "interlacedGIF": e.interlaced, "name": name,
    })
}

/// Element-Wrapper zur ID (für Rückgaben).
#[allow(dead_code)]
pub(super) fn wrap_id(ctx: &mut Context, doc: DocId, id: ElementId) -> FwResult<JsValue> {
    let e = element(doc, id)?;
    wrap_element(ctx, doc, &e)
}

//! Modifizieren ▸ Bitmap nachzeichnen… (`modify.traceBitmap`): ausgewählte Bitmaps über
//! [`fp_trace`] in gefüllte Vektorpfade umwandeln. Ergebnis je Bitmap eine Gruppe gefüllter
//! Pfade (ohne Strich) auf einer neuen Ebene über der aktuellen; ein Verlaufsschritt.
//! Der Dialog mit Live-Vorschau (fp-ui) ruft diesen Befehl mit den gewählten Optionen auf und
//! bietet vorher sein fertiges Vorschauergebnis an ([`offer_prepared`]): Stimmen Bild und Optionen
//! überein, wird es übernommen statt neu gerechnet (VT-4).

use super::edit::add_layer;
use super::{ids_json, sel_err, selected_elements, take_ids, with_doc};
use crate::command::{cmd, CommandOutput, CommandRegistry, ObjectKind, ParamKind, Params, Requirement};
use crate::docops;
use crate::error::{EngineError, Result};
use fp_model::element::ElementKind;
use fp_model::layer::LayerKind;
use fp_model::{Element, ElementId, Fill, PathAttrs};
use fp_trace::{ColorMode, CurveMode, Engine, Hierarchy, Preset, TraceOptions, TurnPolicy};
use serde_json::{json, Map, Value};
use std::cell::{Cell, RefCell};
use std::sync::Arc;

/// Fertiges Ergebnis der Dialog-Vorschau für genau ein Bild und genau diese Optionen.
struct Prepared {
    image: Arc<fp_raster::RgbaImage>,
    opts: TraceOptions,
    result: Arc<fp_trace::TraceResult>,
}

thread_local! {
    // Befehle laufen im Thread der Oberfläche (bzw. des Aufrufers): je Thread ein Angebot.
    static PREPARED: RefCell<Option<Prepared>> = const { RefCell::new(None) };
    static COMPUTED: Cell<u64> = const { Cell::new(0) };
}

/// Bietet dem nächsten `modify.traceBitmap` in diesem Thread ein fertiges Ergebnis an (Dialog:
/// letzte Vorschau). Der Befehl übernimmt es für eine Bitmap mit genau diesem Bild, wenn seine
/// Optionen (validiert) gleich sind; sonst rechnet er wie immer. Das Angebot gilt für einen Aufruf.
pub fn offer_prepared(image: Arc<fp_raster::RgbaImage>, opts: &TraceOptions, result: Arc<fp_trace::TraceResult>) {
    PREPARED.with(|p| *p.borrow_mut() = Some(Prepared { image, opts: opts.validated(), result }));
}

/// Verwirft ein nicht benutztes Angebot.
pub fn clear_prepared() {
    PREPARED.with(|p| p.borrow_mut().take());
}

/// Zahl der Bilder, die `modify.traceBitmap` in diesem Thread tatsächlich nachgezeichnet hat
/// (übernommene Vorschauergebnisse zählen nicht) – für Tests und Diagnose.
pub fn computed_traces() -> u64 {
    COMPUTED.with(Cell::get)
}

/// Ergebnis für ein Bild: das angebotene, wenn Bild und Optionen passen, sonst neu gerechnet.
fn trace_or_prepared(img: &fp_raster::RgbaImage, opts: &TraceOptions) -> Result<fp_trace::TraceResult> {
    let hit = PREPARED.with(|p| {
        let mut p = p.borrow_mut();
        let ok = p.as_ref().is_some_and(|q| q.opts == *opts && *q.image == *img);
        if ok { p.take().map(|q| q.result) } else { None }
    });
    if let Some(r) = hit {
        return Ok((*r).clone());
    }
    COMPUTED.with(|c| c.set(c.get() + 1));
    fp_trace::trace(img, opts).map_err(|e| EngineError::Failed(e.to_string()))
}

fn enum_kind(values: &[&str]) -> ParamKind {
    ParamKind::Enum { values: values.iter().map(|s| s.to_string()).collect() }
}

fn int(min: i64, max: i64) -> ParamKind {
    ParamKind::Integer { min: Some(min), max: Some(max) }
}

fn num(min: f64, max: f64) -> ParamKind {
    ParamKind::Number { min: Some(min), max: Some(max) }
}

/// `snake_case`-Schlüssel (z. B. `keep_original`) zusätzlich als `camelCase` annehmen.
fn normalized(p: &Params) -> Params {
    let Some(obj) = p.value.as_object() else { return p.clone() };
    let mut out = Map::new();
    for (k, v) in obj {
        let mut camel = String::with_capacity(k.len());
        let mut up = false;
        for c in k.chars() {
            if c == '_' {
                up = true;
            } else if up {
                camel.extend(c.to_uppercase());
                up = false;
            } else {
                camel.push(c);
            }
        }
        out.entry(camel).or_insert_with(|| v.clone());
    }
    Params::new(&p.command, Value::Object(out))
}

/// Optionen aus den Parametern: Vorgabe `preset`, darüber die einzeln angegebenen Werte.
pub(crate) fn options_from_params(p: &Params) -> Result<TraceOptions> {
    let preset = match p.opt_str("preset")? {
        Some(name) => Preset::from_name(&name).ok_or_else(|| EngineError::Failed(fp_i18n::tr!("trace-err-unknown-preset", name = name.as_str())))?,
        None => Preset::Logo,
    };
    let mut o = TraceOptions::from_preset(preset);
    let bad = |key: &str| EngineError::Failed(fp_i18n::tr!("trace-err-bad-options", detail = key));
    if let Some(s) = p.opt_str("colorMode")? {
        o.color_mode = match s.as_str() {
            "color" => ColorMode::Color,
            "blackWhite" => ColorMode::BlackWhite,
            _ => return Err(bad("colorMode")),
        };
    }
    if let Some(s) = p.opt_str("mode")? {
        o.mode = match s.as_str() {
            "polygon" => CurveMode::Polygon,
            "spline" => CurveMode::Spline,
            _ => return Err(bad("mode")),
        };
    }
    if let Some(s) = p.opt_str("hierarchical")? {
        o.hierarchical = match s.as_str() {
            "stacked" => Hierarchy::Stacked,
            "cutout" => Hierarchy::Cutout,
            _ => return Err(bad("hierarchical")),
        };
    }
    let u = |v: f64, max: f64| v.clamp(0.0, max).round();
    if let Some(v) = p.opt_f64("colors")? {
        o.colors = u(v, 256.0) as u32;
    }
    if let Some(v) = p.opt_f64("colorPrecision")? {
        o.color_precision = u(v, 8.0) as u8;
    }
    if let Some(v) = p.opt_f64("layerDifference")? {
        o.layer_difference = u(v, 255.0) as u8;
    }
    if let Some(v) = p.opt_f64("filterSpeckle")? {
        o.filter_speckle = u(v, 128.0) as u32;
    }
    if let Some(v) = p.opt_f64("cornerThreshold")? {
        o.corner_threshold = v;
    }
    if let Some(v) = p.opt_f64("lengthThreshold")? {
        o.length_threshold = v;
    }
    if let Some(v) = p.opt_f64("maxIterations")? {
        o.max_iterations = u(v, 50.0) as usize;
    }
    if let Some(v) = p.opt_f64("spliceThreshold")? {
        o.splice_threshold = v;
    }
    if let Some(v) = p.opt_f64("pathPrecision")? {
        o.path_precision = u(v, 8.0) as u32;
    }
    if let Some(v) = p.opt_bool("ignoreTransparent")? {
        o.ignore_transparent = v;
    }
    if let Some(v) = p.opt_f64("alphaThreshold")? {
        o.alpha_threshold = u(v, 255.0) as u8;
    }
    if let Some(v) = p.opt_f64("threshold")? {
        o.threshold = u(v, 255.0) as u8;
    }
    if let Some(v) = p.opt_f64("maxPixels")? {
        o.max_pixels = v.clamp(0.0, fp_trace::HARD_PIXEL_LIMIT as f64) as u64;
    }
    if let Some(v) = p.opt_bool("downscale")? {
        o.downscale = v;
    }
    // Logo-Nachbearbeitung: `true`/`false` schaltet sie mit den Werten der Vorgabe, ein Objekt
    // überschreibt einzelne Schalter (camelCase wie in `RefineOptions`).
    match p.get("refine") {
        None | Some(Value::Null) => {}
        Some(Value::Bool(b)) => o.refine.enabled = *b,
        Some(Value::Object(m)) => {
            let mut base = serde_json::to_value(&o.refine).map_err(|_| bad("refine"))?;
            for (k, v) in m {
                base[k.as_str()] = v.clone();
            }
            o.refine = serde_json::from_value(base).map_err(|_| bad("refine"))?;
        }
        Some(_) => return Err(bad("refine")),
    }
    // Verfahren (VT-6) und Einstellungen des Konturverfahrens: Objekt `potrace` (wie gespeichert)
    // und/oder einzelne Schlüssel `turdSize`, `turnPolicy`, `alphaMax`, `optCurve`, `optTolerance`.
    if let Some(s) = p.opt_str("engine")? {
        o.engine = Engine::from_name(&s).ok_or_else(|| bad("engine"))?;
    }
    match p.get("potrace") {
        None | Some(Value::Null) => {}
        Some(Value::Object(m)) => {
            let mut base = serde_json::to_value(&o.potrace).map_err(|_| bad("potrace"))?;
            for (k, v) in m {
                base[k.as_str()] = v.clone();
            }
            o.potrace = serde_json::from_value(base).map_err(|_| bad("potrace"))?;
        }
        Some(_) => return Err(bad("potrace")),
    }
    if let Some(v) = p.opt_f64("turdSize")? {
        o.potrace.turd_size = v.clamp(0.0, 10_000.0).round() as u32;
    }
    if let Some(s) = p.opt_str("turnPolicy")? {
        o.potrace.turn_policy = TurnPolicy::from_name(&s).ok_or_else(|| bad("turnPolicy"))?;
    }
    if let Some(v) = p.opt_f64("alphaMax")? {
        o.potrace.alpha_max = v;
    }
    if let Some(v) = p.opt_bool("optCurve")? {
        o.potrace.opt_curve = v;
    }
    if let Some(v) = p.opt_f64("optTolerance")? {
        o.potrace.opt_tolerance = v;
    }
    Ok(o.validated())
}

/// Ergebnis eines Bildes → Gruppe gefüllter Pfade (Kinder oben → unten).
fn result_group(ids: &mut fp_model::IdGen, r: &fp_trace::TraceResult, name: Option<String>) -> Option<Element> {
    let mut children = Vec::with_capacity(r.paths.len());
    for tp in r.top_to_bottom() {
        let contours: Vec<_> = tp.contours.iter().map(crate::geometry::from_bezier).collect();
        let attrs = PathAttrs { fill_color: tp.color.to_argb(), fill: Some(Fill::solid()), brush: None, ..PathAttrs::default() };
        let mut e = Element::new_path(ids, contours, attrs);
        if let ElementKind::Path(pe) = &mut e.kind {
            // Löcher sind eigene Konturen.
            pe.even_odd = true;
        }
        children.push(Arc::new(e));
    }
    if children.is_empty() {
        return None;
    }
    let mut g = Element::new_group(ids, children);
    g.common.name = name;
    Some(g)
}

pub(crate) fn register(r: &mut CommandRegistry) {
    r.add(
        cmd("modify.traceBitmap")
            .label("Bitmap nachzeichnen")
            .describe("Zeichnet die ausgewählte Bitmap als gefüllte, bearbeitbare Vektorpfade auf einer neuen Ebene nach")
            .requires(&[Requirement::Document, Requirement::AnyOf(ObjectKind::Bitmap)])
            .opt_default("preset", enum_kind(&["logo", "blackWhite", "illustration", "photo"]), json!("logo"), "Vorgabe; die übrigen Optionen überschreiben einzelne Werte")
            .opt("colorMode", enum_kind(&["color", "blackWhite"]), "Farbe oder Schwarzweiß")
            .opt("colors", int(0, 256), "Höchstzahl der Farben (0 = keine Farbreduktion)")
            .opt("colorPrecision", int(1, 8), "signifikante Bits je Farbkanal")
            .opt("layerDifference", int(0, 255), "Farbabstand zwischen Stapelebenen")
            .opt("filterSpeckle", int(0, 128), "Flecken kleiner als n × n Pixel entfernen")
            .opt("cornerThreshold", num(0.0, 180.0), "Eckenwinkel in Grad")
            .opt("lengthThreshold", num(3.5, 10.0), "Mindestlänge der Segmente in Pixeln")
            .opt("maxIterations", int(1, 50), "Höchstzahl der Glättungsdurchläufe")
            .opt("spliceThreshold", num(0.0, 180.0), "Spleißwinkel der Splines in Grad")
            .opt("pathPrecision", int(0, 8), "Nachkommastellen der Koordinaten")
            .opt("mode", enum_kind(&["polygon", "spline"]), "gerade Segmente oder Bézierkurven")
            .opt("hierarchical", enum_kind(&["stacked", "cutout"]), "Flächen stapeln oder ausschneiden")
            .opt("ignoreTransparent", ParamKind::Bool, "transparente Pixel ergeben keine Flächen")
            .opt("alphaThreshold", int(1, 255), "Alpha unter diesem Wert gilt als transparent")
            .opt("threshold", int(0, 255), "Schwarzweiß: Helligkeitsschwelle")
            .opt("maxPixels", ParamKind::Integer { min: Some(0), max: Some(fp_trace::HARD_PIXEL_LIMIT as i64) }, "größere Bilder herunterskalieren (0 = nur feste Obergrenze)")
            .opt("downscale", ParamKind::Bool, "zu große Bilder herunterskalieren statt abzulehnen")
            .opt("refine", ParamKind::Json, "Logo-Nachbearbeitung: true/false oder Objekt (enabled, snapColors, colorTolerance, straighten, snapAngle, shapes, shapeTolerance, reduceNodes, fitTolerance, closeGaps, symmetry, fast); fehlt: wie die Vorgabe")
            .opt_default("engine", enum_kind(&["clusters", "potrace"]), json!("clusters"), "Verfahren: Farbcluster oder Konturen (Potrace-Verfahren)")
            .opt("turdSize", int(0, 10_000), "Konturen: Flecken bis zu dieser Fläche in Pixeln entfernen")
            .opt("turnPolicy", enum_kind(&["minority", "majority", "black", "white", "left", "right"]), "Konturen: Abbiegeregel an diagonal berührenden Pixeln")
            .opt("alphaMax", num(0.0, fp_trace::ALPHA_MAX_LIMIT), "Konturen: Eckenschwelle (0 = nur Ecken, ab 4/3 keine Ecken)")
            .opt("optCurve", ParamKind::Bool, "Konturen: Kurven zusammenfassen")
            .opt("optTolerance", num(0.0, 5.0), "Konturen: Toleranz beim Zusammenfassen in Pixeln")
            .opt("potrace", ParamKind::Json, "Konturen: alle Einstellungen als Objekt (turdSize, turnPolicy, alphaMax, optCurve, optTolerance)")
            .opt_default("keepOriginal", ParamKind::Bool, json!(true), "Original-Bitmap behalten (false = löschen)")
            .opt("layerName", ParamKind::String, "Name der neuen Ebene")
            .run(|s, p| {
                let p = normalized(p);
                let opts = options_from_params(&p)?;
                let keep = p.bool_or("keepOriginal", true)?;
                let layer_name = p.opt_str("layerName")?.filter(|n| !n.trim().is_empty()).unwrap_or_else(|| fp_i18n::t("trace-layer-name"));
                with_doc(s, |d| {
                    let bitmaps: Vec<(ElementId, fp_raster::RgbaImage, Option<String>)> = selected_elements(d)
                        .into_iter()
                        .filter_map(|(_, e)| match &e.kind {
                            ElementKind::Bitmap(b) => docops::bitmap_image(b).map(|img| (e.id, img, e.common.name.clone())),
                            _ => None,
                        })
                        .collect();
                    if bitmaps.is_empty() {
                        return Err(sel_err("err-sel-no-bitmap"));
                    }
                    // Erst alles nachzeichnen (ohne Dokumentänderung), dann einfügen.
                    let mut results = Vec::with_capacity(bitmaps.len());
                    for (id, img, name) in &bitmaps {
                        let r = trace_or_prepared(img, &opts);
                        let r = match r {
                            Ok(r) => r,
                            Err(e) => {
                                clear_prepared();
                                return Err(e);
                            }
                        };
                        results.push((*id, r, name.clone()));
                    }
                    clear_prepared();
                    let (mut paths, mut nodes, mut downscaled) = (0usize, 0usize, false);
                    let mut groups = Vec::new();
                    {
                        let doc = Arc::make_mut(&mut d.doc);
                        let mut idg = take_ids(doc);
                        for (_, r, name) in &results {
                            paths += r.paths.len();
                            nodes += r.node_count();
                            downscaled |= r.downscaled();
                            let gname = name.as_ref().map(|n| fp_i18n::tr!("trace-group-name", name = n.as_str()));
                            if let Some(g) = result_group(&mut idg, r, gname) {
                                groups.push(g);
                            }
                        }
                        doc.ids = idg;
                    }
                    if groups.is_empty() {
                        return Err(EngineError::Failed(fp_i18n::t("trace-err-no-paths")));
                    }
                    if !keep {
                        let ids: Vec<ElementId> = bitmaps.iter().map(|b| b.0).collect();
                        let page = d.page;
                        docops::remove_ids(Arc::make_mut(&mut d.doc), page, &ids);
                    }
                    let layer = add_layer(d, Some(&layer_name), LayerKind::Normal)?;
                    let (page, frame) = (d.page, d.frame);
                    let doc = Arc::make_mut(&mut d.doc);
                    let mut new_ids = Vec::with_capacity(groups.len());
                    // Auswahl ist oben → unten sortiert: von unten einfügen, damit die Reihenfolge bleibt.
                    for g in groups.into_iter().rev() {
                        new_ids.insert(0, g.id);
                        docops::insert_element(doc, page, frame, Some(layer), 0, Arc::new(g)).ok_or_else(|| EngineError::Failed(fp_i18n::t("err-element-insert-failed")))?;
                    }
                    d.selection.set(new_ids.clone());
                    Ok(CommandOutput::doc_with(json!({
                        "ids": ids_json(&new_ids),
                        "layer": layer.0,
                        "paths": paths,
                        "nodes": nodes,
                        "downscaled": downscaled,
                        "keptOriginal": keep,
                    })))
                })
            }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(v: Value) -> TraceOptions {
        options_from_params(&Params::new("modify.traceBitmap", v)).expect("Optionen")
    }

    #[test]
    fn refine_parameter_and_dialog_round_trip() {
        // fehlt: wie die Vorgabe
        assert!(opts(json!({})).refine.enabled);
        assert!(!opts(json!({"preset": "photo"})).refine.enabled);
        // true/false und Objekt über der Vorgabe
        assert!(!opts(json!({"refine": false})).refine.enabled);
        assert!(opts(json!({"preset": "illustration", "refine": true})).refine.enabled);
        let o = opts(json!({"refine": {"shapes": false, "snapAngle": 9.0}}));
        assert!(o.refine.enabled && !o.refine.shapes && o.refine.snap_angle == 9.0 && o.refine.straighten);
        assert!(options_from_params(&Params::new("modify.traceBitmap", json!({"refine": 3}))).is_err());
        // alle Optionen des Dialogs (serialisiert) kommen unverändert an – Voraussetzung dafür,
        // dass OK das Vorschauergebnis übernehmen darf
        for p in Preset::ALL {
            let mut d = TraceOptions::from_preset(p);
            d.colors = 23;
            d.corner_threshold = 77.5;
            d.max_pixels = 1_234_567;
            d.refine.enabled = !d.refine.enabled;
            d.refine.fit_tolerance = 1.7;
            d.refine.symmetry = true;
            assert_eq!(opts(serde_json::to_value(&d).expect("JSON")), d.validated(), "{p:?}");
            // zweites Verfahren (VT-6) mit eigenen Werten
            d.engine = Engine::Potrace;
            d.potrace = fp_trace::PotraceOptions { turd_size: 9, turn_policy: TurnPolicy::White, alpha_max: 0.7, opt_curve: false, opt_tolerance: 0.45 };
            assert_eq!(opts(serde_json::to_value(&d).expect("JSON")), d.validated(), "{p:?} Konturen");
        }
    }

    #[test]
    fn engine_parameters() {
        // Vorgabe: bisheriges Verfahren
        let o = opts(json!({}));
        assert_eq!(o.engine, Engine::Clusters);
        assert_eq!(o.potrace, fp_trace::PotraceOptions::default());
        // einzelne Schlüssel (auch snake_case) über dem Objekt
        let p = Params::new("modify.traceBitmap", json!({"engine": "potrace", "potrace": {"turdSize": 5, "alphaMax": 0.5}, "alpha_max": 0.9, "turnPolicy": "black", "opt_curve": false, "optTolerance": 0.3}));
        let o = options_from_params(&normalized(&p)).expect("Optionen");
        assert_eq!(o.engine, Engine::Potrace);
        assert_eq!((o.potrace.turd_size, o.potrace.alpha_max, o.potrace.turn_policy, o.potrace.opt_curve, o.potrace.opt_tolerance), (5, 0.9, TurnPolicy::Black, false, 0.3));
        // Bereiche und Fehler
        assert_eq!(opts(json!({"alphaMax": 7.0, "turdSize": 1e9})).potrace.alpha_max, fp_trace::ALPHA_MAX_LIMIT);
        assert_eq!(opts(json!({"turdSize": 1e9})).potrace.turd_size, 10_000);
        for bad in [json!({"engine": "magic"}), json!({"turnPolicy": "up"}), json!({"potrace": 3}), json!({"potrace": {"turnPolicy": 1}})] {
            assert!(options_from_params(&Params::new("modify.traceBitmap", bad.clone())).is_err(), "{bad}");
        }
    }

    #[test]
    fn prepared_result_is_used_once_and_only_for_the_same_image_and_options() {
        let img = fp_raster::RgbaImage::filled(fp_geom::IRect::from_xywh(0, 0, 8, 8), fp_color::Rgba8::new(0, 0, 0, 255)).expect("Bild");
        let o = TraceOptions::from_preset(Preset::Logo).validated();
        let fake = Arc::new(fp_trace::TraceResult { paths: Vec::new(), bounds: img.bounds(), traced_width: 8, traced_height: 8 });
        let n0 = computed_traces();
        offer_prepared(Arc::new(img.clone()), &o, Arc::clone(&fake));
        // andere Optionen: neu gerechnet, Angebot bleibt
        let mut other = o.clone();
        other.colors = 5;
        assert!(trace_or_prepared(&img, &other.validated()).expect("Ergebnis").node_count() > 0);
        assert_eq!(computed_traces(), n0 + 1);
        // gleiche Optionen und gleiches Bild: übernommen (das leere Kunstergebnis), dann verbraucht
        assert!(trace_or_prepared(&img, &o).expect("Ergebnis").paths.is_empty());
        assert_eq!(computed_traces(), n0 + 1);
        assert!(!trace_or_prepared(&img, &o).expect("Ergebnis").paths.is_empty());
        assert_eq!(computed_traces(), n0 + 2);
        // anderes Bild
        offer_prepared(Arc::new(img.clone()), &o, fake);
        let img2 = fp_raster::RgbaImage::filled(fp_geom::IRect::from_xywh(0, 0, 8, 8), fp_color::Rgba8::new(9, 0, 0, 255)).expect("Bild");
        assert!(!trace_or_prepared(&img2, &o).expect("Ergebnis").paths.is_empty());
        clear_prepared();
    }
}

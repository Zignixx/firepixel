//! Gruppe „Erweitert“, Schritt `vector.trace` „Nachzeichnen → SVG/FW-PNG“ (SPEC-BATCH §3.13,
//! Paket BATCH-9) und der SVG-Kodierer der Endkarte „Speichern als“.
//!
//! Der Schritt ruft **denselben Befehl** wie *Modifizieren ▸ Bitmap nachzeichnen…*
//! (`modify.traceBitmap`, beide Verfahren `clusters`/`potrace`, Vorgaben, Aufräumen) in einer eigenen
//! headless Sitzung je Element auf. Ergebnis ist ein **Dokument** (Element bleibt bearbeitbar):
//! - Ebene mit der nachgezeichneten Gruppe gefüllter Pfade (Löcher gerade/ungerade),
//! - auf Wunsch (`keepOriginal`) darunter die Original-Bitmap auf einer **ausgeblendeten** Ebene
//!   (nur im FW-PNG sichtbar/einblendbar; SVG und flache Formate zeigen nur die Pfade),
//! - Leinwand transparent (`transparentCanvas`, Vorgabe), sonst wie die Quelle.
//!
//! Ausgabe über die Endkarte: **SVG** (Pfade, Kodierer [`encode_svg`] über `fp-export`), **FW-PNG**
//! (bearbeitbar, eingebauter Weg) oder jedes Pixelformat (das Ergebnis wird flach gerechnet).
//!
//! Parameter mit Vorgabe `null` gelten „wie die Vorgabe (`preset`)“ und werden nur weitergereicht,
//! wenn sie gesetzt sind – so ändert die Wahl der Vorgabe wirklich alle ihre Werte.

use crate::error::{BatchError, Result};
use crate::item::{DocItem, Item, ItemKind, Meta, Still};
use crate::output::{self, OutputFormat};
use crate::step::{Issue, ParamDef, ParamType, Step, StepContext, StepGroup, StepRegistry};
use crate::steps::script::new_session;
use fp_model::{LayerKind, LayerSlot};
use serde_json::{json, Map, Value};
use std::sync::Arc;

/// ID „Nachzeichnen → SVG/FW-PNG“.
pub const TRACE: &str = "vector.trace";

/// Vorgaben (wie `modify.traceBitmap`).
pub const PRESETS: [&str; 4] = ["logo", "blackWhite", "illustration", "photo"];
/// Verfahren: Farbflächen (Cluster) oder glatte Kurven (Potrace-Verfahren, eigenständig umgesetzt).
pub const ENGINES: [&str; 2] = ["clusters", "potrace"];
/// Obergrenze der Eckenschwelle `alphaMax` (wie `fp_trace::ALPHA_MAX_LIMIT`).
pub const ALPHA_MAX_LIMIT: f64 = 4.0 / 3.0;

/// Parameter, die unverändert an `modify.traceBitmap` gehen (wenn gesetzt).
const PASS_THROUGH: [&str; 17] = [
    "preset",
    "engine",
    "colorMode",
    "colors",
    "filterSpeckle",
    "cornerThreshold",
    "mode",
    "hierarchical",
    "threshold",
    "ignoreTransparent",
    "pathPrecision",
    "refine",
    "turdSize",
    "turnPolicy",
    "alphaMax",
    "optCurve",
    "optTolerance",
];

/// Parameter ohne feste Vorgabe („wie die Vorgabe“).
fn unset(name: &str, ty: ParamType) -> ParamDef {
    ParamDef { name: name.to_string(), ty, default: Value::Null, beginner: false, unit: None }
}

fn choices(v: &[&str]) -> ParamType {
    ParamType::Enum { values: v.iter().map(|s| s.to_string()).collect() }
}

struct TraceStep;

impl Step for TraceStep {
    fn id(&self) -> &str {
        TRACE
    }
    fn group(&self) -> StepGroup {
        StepGroup::Advanced
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::choice("preset", &PRESETS, "logo").beginner(),
            ParamDef::choice("engine", &ENGINES, "clusters").beginner(),
            unset("colorMode", choices(&["color", "blackWhite"])),
            unset("colors", ParamType::Int { min: 0, max: 256 }),
            unset("filterSpeckle", ParamType::Int { min: 0, max: 128 }).unit("px"),
            unset("cornerThreshold", ParamType::Number { min: 0.0, max: 180.0 }).unit("°"),
            unset("mode", choices(&["spline", "polygon"])),
            unset("hierarchical", choices(&["stacked", "cutout"])),
            unset("threshold", ParamType::Int { min: 0, max: 255 }),
            unset("ignoreTransparent", ParamType::Bool),
            unset("pathPrecision", ParamType::Int { min: 0, max: 8 }),
            unset("refine", ParamType::Bool),
            unset("turdSize", ParamType::Int { min: 0, max: 10_000 }).unit("px²"),
            unset("turnPolicy", choices(&["minority", "majority", "black", "white", "left", "right"])),
            unset("alphaMax", ParamType::Number { min: 0.0, max: ALPHA_MAX_LIMIT }),
            unset("optCurve", ParamType::Bool),
            unset("optTolerance", ParamType::Number { min: 0.0, max: 5.0 }).unit("px"),
            ParamDef::bool("keepOriginal", false),
            ParamDef::bool("transparentCanvas", true),
        ]
    }
    fn validate(&self, params: &Value) -> Vec<Issue> {
        crate::step::validate_params(&self.params(), params)
    }
    fn applies_to(&self) -> &[ItemKind] {
        &[ItemKind::Image, ItemKind::Animation, ItemKind::Document]
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        ctx.check_cancel()?;
        let (image, meta) = match &*item {
            Item::FileOnly(_) => return Ok(()),
            Item::Still(s) => (s.image.clone(), s.meta.clone()),
            Item::Animation(a) => {
                let first = a.frames.first().map(|f| f.image.clone()).ok_or_else(|| BatchError::Decode(fp_i18n::t("batch-err-no-frames")))?;
                if a.frames.len() > 1 {
                    ctx.warn(fp_i18n::t("batch-warn-trace-first-frame"));
                }
                (first, a.meta.clone())
            }
            Item::Document(d) => (crate::item::render_document(&d.doc)?, d.meta.clone()),
        };
        let doc = trace_image(&image, &meta, params)?;
        ctx.check_cancel()?;
        *item = Item::Document(DocItem { doc: Arc::new(doc), meta: Meta { has_transparency: true, ..meta } });
        Ok(())
    }
    fn summary(&self, params: &Value) -> String {
        let preset = params.get("preset").and_then(Value::as_str).unwrap_or("logo");
        let engine = params.get("engine").and_then(Value::as_str).unwrap_or("clusters");
        let pk = format!("batch-value-vector-trace-preset-{preset}");
        let ek = format!("batch-value-vector-trace-engine-{engine}");
        let label = |k: &str, raw: &str| if fp_i18n::has(k) { fp_i18n::t(k) } else { raw.to_string() };
        format!("{} · {}", label(&pk, preset), label(&ek, engine))
    }
}

/// Parameter für `modify.traceBitmap` aus den Schritt-Parametern (nur gesetzte Werte).
pub fn command_params(params: &Value) -> Value {
    let mut m = Map::new();
    for k in PASS_THROUGH {
        if let Some(v) = params.get(k).filter(|v| !v.is_null()) {
            m.insert(k.to_string(), v.clone());
        }
    }
    m.insert("keepOriginal".into(), json!(params.get("keepOriginal").and_then(Value::as_bool).unwrap_or(false)));
    Value::Object(m)
}

/// Zeichnet ein Bild nach: Dokument mit Leinwand in Bildgröße, darin die Ebene mit der
/// nachgezeichneten Gruppe (und ggf. die ausgeblendete Original-Ebene).
pub fn trace_image(image: &fp_raster::RgbaImage, meta: &Meta, params: &Value) -> Result<fp_model::Document> {
    let doc = output::item_document(&Item::Still(Still { image: image.clone(), meta: meta.clone() }))?;
    let mut s = new_session(false);
    s.add_document(doc, None);
    let err = |e: fp_engine::EngineError| BatchError::Invalid(e.to_string());
    s.execute("select.all", json!({})).map_err(err)?;
    let out = s.execute("modify.traceBitmap", command_params(params)).map_err(err)?;
    let layer = out.value.get("layer").and_then(Value::as_u64);
    let mut doc = s.doc().map_err(err)?.doc.as_ref().clone();
    let keep = params.get("keepOriginal").and_then(Value::as_bool).unwrap_or(false);
    let transparent = params.get("transparentCanvas").and_then(Value::as_bool).unwrap_or(true);
    for page in &mut doc.pages {
        let page = Arc::make_mut(page);
        if transparent {
            page.root.background = fp_color::Argb(0x00FF_FFFF);
        }
        // Original-Bitmap behalten, aber ausblenden (SVG/flache Formate zeigen nur die Pfade);
        // ohne `keepOriginal` sind ihre Ebenen leer und werden entfernt.
        page.root.layers.retain(|slot| match slot {
            LayerSlot::Own(l) => keep || l.kind == LayerKind::Web || Some(l.id.0) == layer || l.cels.iter().any(|c| !c.elements.is_empty()),
            _ => true,
        });
        for slot in &mut page.root.layers {
            if let LayerSlot::Own(l) = slot
                && l.kind != LayerKind::Web
                && Some(l.id.0) != layer
            {
                let l = Arc::make_mut(l);
                let n = l.visibility.len().max(1);
                l.visibility = vec![false; n];
            }
        }
    }
    Ok(doc)
}

/// Kodierer „SVG“: Dokumente (z. B. nach `vector.trace`) als Vektorpfade, Einzelbilder als
/// eingebettetes Bild (erste Seite, erstes Bild).
pub fn encode_svg(item: &Item, _options: &Map<String, Value>) -> Result<Vec<u8>> {
    let doc = output::item_document(item)?;
    let svg = fp_export::vector_out::write_svg(&doc, 0, 0, None).map_err(|e| BatchError::Encode(e.to_string()))?;
    Ok(svg.into_bytes())
}

/// Hängt die Kodierer dieses Pakets ein (SVG). Mehrfach aufrufbar.
pub fn install_encoders() {
    output::register_encoder(OutputFormat::Svg, encode_svg);
}

/// Registriert die Schritte (und den SVG-Kodierer).
pub fn register(r: &mut StepRegistry) {
    install_encoders();
    r.register(Arc::new(TraceStep));
}

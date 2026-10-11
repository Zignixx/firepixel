//! KI-Auswahl (fp-ai): Objektauswahl (SAM 2.1), Motiv auswählen und Kante verfeinern (BiRefNet).
//!
//! * `select.aiObject {points | point, box?, mode?, session?, refine?, source?, region?, model?, feather?}`
//! * `select.aiSubject {mode?, source?, point?, region?, model?, feather?}`
//! * `select.aiRefineEdge {radius?, output: "selection" | "mask", source?, model?}`
//! * `ai.models {}` (Modelle, Ordner, Verfügbarkeit), `ai.setModelFolder {path}`, `ai.clearCache {}`
//!
//! Ergebnisse sind normale Pixelauswahlen bzw. (bei `output: "mask"`) eine Bitmapmaske. Wie die
//! anderen Klick-Auswahlen erzeugt eine Auswahländerung **keinen** Verlaufsschritt
//! ([`crate::command::Effect::Session`] + `SelectionChanged`); die Maskenausgabe ist genau ein
//! Schritt.
//!
//! Die Rechnung selbst ist synchron (CLI, MCP, JSF). Die Oberfläche wärmt die Zwischenspeicher von
//! fp-ai vorher im Hintergrund an ([`prepare`] + [`run_prepared`]) und führt den Befehl erst danach
//! aus – dann braucht er nur noch den schnellen Teil (Decoder bzw. Zuschnitt).

/// fp-ai für die Oberfläche (Hintergrundaufträge, Modellkatalog, Download), ohne eigene Abhängigkeit.
pub use fp_ai;

use super::{sel_err, DOC};
use crate::command::{cmd, CommandOutput, CommandRegistry, Effect, Params, ParamKind, Requirement};
use crate::docops;
use crate::error::{EngineError, Result};
use crate::selection::SelectMode;
use crate::session::{SessionEvent, Session};
use fp_ai::models::{self, ModelId, Task};
use fp_ai::pipeline::{self, ObjectPrompt, PromptPoint};
use fp_ai::{AiError, Progress};
use fp_geom::{IRect, Point};
use fp_model::element::ElementKind;
use fp_model::ElementId;
use fp_raster::{Mask, RgbaImage, SoftEdge};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

// ------------------------------------------------------------------ Quelle

/// Bildquelle einer KI-Auswahl.
#[derive(Clone, Debug)]
pub struct AiSource {
    /// Pixel in Dokumentkoordinaten.
    pub image: Arc<RgbaImage>,
    /// Bitmap, aus der die Pixel stammen (`None` = gerenderte Seite).
    pub target: Option<ElementId>,
    /// `"bitmap"` oder `"page"`.
    pub kind: &'static str,
}

/// Letzte gerenderte Seite (Schlüssel: Dokument-Zeiger, Generation, Seite, Status).
type PageCache = Mutex<Option<((usize, u64, usize, usize), Arc<RgbaImage>)>>;

fn page_cache() -> &'static PageCache {
    static C: OnceLock<PageCache> = OnceLock::new();
    C.get_or_init(|| Mutex::new(None))
}

/// Modellordner aus den Voreinstellungen (`AiModelFolder`) an fp-ai weitergeben.
fn sync_folder(s: &Session) {
    let f = s.prefs.get_str("AiModelFolder", "");
    models::set_user_folder(if f.trim().is_empty() { None } else { Some(std::path::PathBuf::from(f.trim())) });
}

/// Bildquelle bestimmen: `source` = `auto` (ausgewählte Bitmap, sonst Bitmap unter `at`, sonst die
/// sichtbare Seite), `bitmap` (nur Bitmap) oder `page` (gerenderte Seite mit allen sichtbaren Ebenen).
pub fn resolve_source(s: &Session, source: &str, at: Option<Point>) -> Result<AiSource> {
    let d = s.doc()?;
    if source != "page" {
        let geo = crate::geometry::BasicGeometry;
        if let Some((_, e)) = super::select::bitmap_for(d, at, &geo)
            && let ElementKind::Bitmap(b) = &e.kind
            && e.common.visible
        {
            let img = docops::bitmap_image(b).ok_or_else(|| EngineError::Failed(fp_i18n::t("err-bitmap-unreadable")))?;
            return Ok(AiSource { image: Arc::new(img), target: Some(e.id), kind: "bitmap" });
        }
        if source == "bitmap" {
            return Err(sel_err("err-sel-no-bitmap-at-point"));
        }
    }
    let key = (Arc::as_ptr(&d.doc) as usize, d.generation, d.page, d.frame);
    if let Ok(c) = page_cache().lock()
        && let Some((k, img)) = c.as_ref()
        && *k == key
    {
        return Ok(AiSource { image: Arc::clone(img), target: None, kind: "page" });
    }
    let r = s.renderer().ok_or(EngineError::BackendMissing("err-backend-renderer-page"))?;
    let img = Arc::new(r.render_page(&d.doc, d.page, d.frame).ok_or(EngineError::BackendMissing("err-backend-renderer"))?);
    if let Ok(mut c) = page_cache().lock() {
        *c = Some((key, Arc::clone(&img)));
    }
    Ok(AiSource { image: img, target: None, kind: "page" })
}

fn source_param(s: &Session, p: &Params) -> Result<String> {
    Ok(p.opt_str("source")?.unwrap_or_else(|| s.prefs.get_str("AiSource", "auto")))
}

/// Modell einer Aufgabe: Parameter `model`, sonst Voreinstellung, sonst Vorgabe.
pub fn model_for(s: &Session, p: Option<&Params>, task: Task) -> Result<ModelId> {
    let key = match task {
        Task::Object => "AiObjectModel",
        Task::Subject => "AiSubjectModel",
        Task::Matting => "AiMattingModel",
    };
    let name = match p {
        Some(p) => p.opt_str("model")?,
        None => None,
    }
    .unwrap_or_else(|| s.prefs.get_str(key, ""));
    let id = if name.is_empty() { models::default_for(task) } else { models::by_name(&name).ok_or_else(|| EngineError::InvalidParams { name: "model".into(), message: fp_i18n::tr!("err-unknown-model", name = name.as_str()) })? };
    if models::info(id).task != task {
        return Err(EngineError::InvalidParams { name: "model".into(), message: fp_i18n::tr!("err-model-wrong-task", name = models::info(id).name) });
    }
    Ok(id)
}

/// Fehler von fp-ai als Engine-Fehler (deutsch).
pub fn ai_err(e: AiError) -> EngineError {
    match e {
        AiError::Input(m) => EngineError::Selection(m),
        other => EngineError::Failed(other.to_string()),
    }
}

fn ensure_model(id: ModelId) -> Result<()> {
    if models::available(id) { Ok(()) } else { Err(ai_err(models::missing(id))) }
}

fn region_param(p: &Params) -> Result<Option<IRect>> {
    Ok(p.opt_rect("region")?.map(|r| r.normalized().round_out()))
}

// ------------------------------------------------------------------ Auswahl anwenden

fn mode_of(p: &Params) -> Result<SelectMode> {
    Ok(p.opt_str("mode")?.and_then(|m| SelectMode::from_name(&m)).unwrap_or_default())
}

fn mode_param() -> ParamKind {
    ParamKind::Enum { values: ["replace", "add", "subtract", "intersect"].iter().map(|s| s.to_string()).collect() }
}

fn combine(base: Option<Mask>, m: Mask, mode: SelectMode) -> Option<Mask> {
    let mut sel = crate::selection::Selection { pixels: base, ..Default::default() };
    sel.apply_pixels(m, mode);
    sel.pixels
}

fn feathered(mut m: Mask, feather: u32) -> Mask {
    if feather > 0
        && let Ok(f) = m.feather(feather)
    {
        m = f;
    }
    m.set_soft_edge(SoftEdge::One);
    m
}

fn mask_hash(m: Option<&Mask>) -> u64 {
    match m {
        None => 0,
        Some(m) => {
            let b = m.bounds();
            fp_ai::cache::hash_bytes(m.data(), &[b.left as u64, b.top as u64, b.right as u64, b.bottom as u64])
        }
    }
}

/// Setzt die Pixelauswahl ohne Verlaufsschritt (wie Klick-Auswahlen) und meldet die Änderung.
fn set_selection(s: &mut Session, pixels: Option<Mask>, value: Value) -> Result<CommandOutput> {
    let d = s.doc_mut()?;
    let id = d.id;
    let changed = d.selection.pixels != pixels;
    d.selection.pixels = pixels.filter(|m| !m.tight_bounds().is_empty());
    if changed {
        s.emit(SessionEvent::SelectionChanged(id));
    }
    Ok(CommandOutput { effect: Effect::Session, label: None, value })
}

fn bounds_json(m: Option<&Mask>) -> Value {
    match m.map(|m| m.tight_bounds()).filter(|b| !b.is_empty()) {
        Some(b) => json!([b.left, b.top, b.right, b.bottom]),
        None => Value::Null,
    }
}

// ------------------------------------------------------------------ Objektauswahl: Sitzungen

/// Laufende Objektauswahl (Klicks zum Verfeinern).
#[derive(Clone, Debug)]
struct ObjSession {
    prompt: ObjectPrompt,
    base: Option<Mask>,
    mode: SelectMode,
    source: String,
    region: Option<IRect>,
    model: ModelId,
    result_hash: u64,
}

fn sessions() -> &'static Mutex<HashMap<u64, ObjSession>> {
    static S: OnceLock<Mutex<HashMap<u64, ObjSession>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Neue Kennung für eine Objektauswahl-Sitzung (das Werkzeug gibt sie bei weiteren Klicks mit).
pub fn new_session_token() -> u64 {
    static N: AtomicU64 = AtomicU64::new(1);
    N.fetch_add(1, Ordering::Relaxed)
}

fn parse_points(p: &Params) -> Result<Vec<PromptPoint>> {
    let mut out = Vec::new();
    let label_of = |v: Option<&Value>| -> bool {
        match v {
            Some(Value::Bool(b)) => *b,
            Some(Value::Number(n)) => n.as_f64().unwrap_or(1.0) > 0.0,
            Some(Value::String(s)) => !matches!(s.as_str(), "negative" | "background" | "0" | "false" | "minus"),
            _ => true,
        }
    };
    if let Some(v) = p.get("points") {
        let arr = v.as_array().ok_or_else(|| EngineError::InvalidParams { name: p.command.clone(), message: fp_i18n::t("err-param-points-list") })?;
        for e in arr {
            let pp = match e {
                Value::Array(a) if a.len() >= 2 => {
                    let x = a[0].as_f64().unwrap_or(f64::NAN) as f32;
                    let y = a[1].as_f64().unwrap_or(f64::NAN) as f32;
                    PromptPoint { x, y, positive: label_of(a.get(2)) }
                }
                Value::Object(o) => {
                    let x = o.get("x").and_then(|v| v.as_f64()).unwrap_or(f64::NAN) as f32;
                    let y = o.get("y").and_then(|v| v.as_f64()).unwrap_or(f64::NAN) as f32;
                    PromptPoint { x, y, positive: label_of(o.get("label").or_else(|| o.get("positive"))) }
                }
                _ => return Err(EngineError::InvalidParams { name: p.command.clone(), message: fp_i18n::t("err-param-points-triples") }),
            };
            if !pp.x.is_finite() || !pp.y.is_finite() {
                return Err(EngineError::InvalidParams { name: p.command.clone(), message: fp_i18n::t("err-param-points-numbers") });
            }
            out.push(pp);
        }
    }
    if let Some(pt) = p.opt_point("point")? {
        out.push(PromptPoint { x: pt.x, y: pt.y, positive: label_of(p.get("label")) });
    }
    Ok(out)
}

/// Alles, was ein Objektauswahl-Aufruf braucht (auch für das Vorwärmen in der Oberfläche).
struct ObjectPlan {
    src: AiSource,
    region: Option<IRect>,
    model: ModelId,
    prompt: ObjectPrompt,
    base: Option<Mask>,
    mode: SelectMode,
    token: u64,
    source: String,
}

fn object_plan(s: &Session, p: &Params) -> Result<ObjectPlan> {
    let new_points = parse_points(p)?;
    let boxx = p.opt_rect("box")?.map(|r| {
        let r = r.normalized();
        [r.left, r.top, r.right, r.bottom]
    });
    let token = p.opt_i64("session")?.map(|v| v.max(0) as u64);
    let refine = p.bool_or("refine", token.is_some())?;
    let cur = s.doc()?.selection.pixels.clone();
    // Fortsetzen nur, wenn die Auswahl noch das letzte Ergebnis dieser Sitzung ist
    let cont = match (token, refine) {
        (Some(t), true) => sessions().lock().ok().and_then(|m| m.get(&t).cloned()).filter(|ss| ss.result_hash == mask_hash(cur.as_ref())),
        _ => None,
    };
    let is_new = cont.is_none();
    let (mut prompt, base, mode, source, mut region, model, token) = match cont {
        Some(ss) => (ss.prompt, ss.base, ss.mode, ss.source, ss.region, ss.model, token.unwrap_or(0)),
        None => {
            let mode = if p.has("mode") {
                mode_of(p)?
            } else if refine && new_points.first().is_some_and(|q| !q.positive) {
                // Alt-Klick ohne laufende Sitzung: angeklicktes Objekt abziehen
                SelectMode::Subtract
            } else if refine {
                SelectMode::Add
            } else {
                SelectMode::Replace
            };
            let base = if mode == SelectMode::Replace { None } else { cur };
            (ObjectPrompt::default(), base, mode, source_param(s, p)?, region_param(p)?, model_for(s, Some(p), Task::Object)?, token.unwrap_or_else(new_session_token))
        }
    };
    let fresh = prompt.points.is_empty() && prompt.boxx.is_none();
    for q in &new_points {
        // Erster Punkt einer neuen Sitzung zum Abziehen: das Objekt selbst ist „positiv“
        let q = if fresh && mode == SelectMode::Subtract { PromptPoint { positive: true, ..*q } } else { *q };
        prompt.add_point(q);
    }
    if boxx.is_some() {
        prompt.boxx = boxx;
    }
    if prompt.points.is_empty() && prompt.boxx.is_none() {
        return Err(EngineError::InvalidParams { name: p.command.clone(), message: fp_i18n::t("err-param-ai-prompt") });
    }
    let at = prompt.points.first().map(|q| Point::new(q.x, q.y)).or_else(|| prompt.boxx.map(|b| Point::new((b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0)));
    let src = resolve_source(s, &source, at)?;
    if is_new
        && region.is_none()
        && let Some(v) = p.opt_rect("view")?
    {
        region = auto_region(src.image.bounds(), v);
    }
    Ok(ObjectPlan { src, region, model, prompt, base, mode, token, source })
}

/// Ausschnitt für stark vergrößerte Ansichten: Ist das Bild in beiden Richtungen mehr als doppelt
/// so groß wie der sichtbare Bereich, wird nur dieser (mit 25 % Rand) codiert – feinere Details.
fn auto_region(img: IRect, view: fp_geom::Rect) -> Option<IRect> {
    let v = view.normalized().round_out().intersect(&img);
    if v.is_empty() || (img.width() as f32) <= 2.0 * v.width() as f32 || (img.height() as f32) <= 2.0 * v.height() as f32 {
        return None;
    }
    let m = (v.width().max(v.height()) / 4).max(32) as i32;
    Some(v.inflate(m, m).intersect(&img))
}

fn run_object(s: &mut Session, p: &Params) -> Result<CommandOutput> {
    sync_folder(s);
    let plan = object_plan(s, p)?;
    ensure_model(plan.model)?;
    let fe = p.i64_or("feather", 0)?.clamp(0, 1000) as u32;
    let r = pipeline::object_mask(&plan.src.image, plan.region, &plan.prompt, plan.model, &Progress::new()).map_err(ai_err)?;
    fp_ai::nn::trim();
    let m = feathered(r.mask, fe);
    let result = combine(plan.base.clone(), m, plan.mode);
    let ss = ObjSession {
        prompt: plan.prompt.clone(),
        base: plan.base,
        mode: plan.mode,
        source: plan.source,
        region: plan.region,
        model: plan.model,
        result_hash: mask_hash(result.as_ref()),
    };
    if let Ok(mut m) = sessions().lock() {
        if m.len() > 64 {
            m.clear();
        }
        m.insert(plan.token, ss);
    }
    let value = json!({
        "session": plan.token,
        "iou": r.iou,
        "objectScore": r.object_score,
        "cached": r.cached,
        "source": plan.src.kind,
        "points": plan.prompt.points.len(),
        "bounds": bounds_json(result.as_ref()),
    });
    set_selection(s, result, value)
}

// ------------------------------------------------------------------ Motiv, Kante

fn run_subject(s: &mut Session, p: &Params) -> Result<CommandOutput> {
    sync_folder(s);
    let model = model_for(s, Some(p), Task::Subject)?;
    ensure_model(model)?;
    let src = resolve_source(s, &source_param(s, p)?, p.opt_point("point")?)?;
    let mode = mode_of(p)?;
    let fe = p.i64_or("feather", 0)?.clamp(0, 1000) as u32;
    let m = pipeline::subject_mask(&src.image, region_param(p)?, model, &Progress::new()).map_err(ai_err)?;
    fp_ai::nn::trim();
    if m.tight_bounds().is_empty() {
        return Err(sel_err("err-sel-no-subject"));
    }
    let m = feathered(m, fe);
    let base = if mode == SelectMode::Replace { None } else { s.doc()?.selection.pixels.clone() };
    let result = combine(base, m, mode);
    let value = json!({"source": src.kind, "bounds": bounds_json(result.as_ref())});
    set_selection(s, result, value)
}

fn refine_inputs(s: &Session, p: &Params) -> Result<(AiSource, Mask, f32, ModelId)> {
    let model = model_for(s, Some(p), Task::Matting)?;
    let sel = s.doc()?.selection.pixels.clone().filter(|m| !m.tight_bounds().is_empty()).ok_or_else(|| sel_err("err-req-pixel-selection"))?;
    let tb = sel.tight_bounds();
    let at = Point::new((tb.left + tb.right) as f32 / 2.0, (tb.top + tb.bottom) as f32 / 2.0);
    let src = resolve_source(s, &source_param(s, p)?, Some(at))?;
    let radius = p.opt_f64("radius")?.unwrap_or_else(|| s.prefs.get_int("AiRefineRadius", 0) as f64).clamp(0.0, 500.0) as f32;
    Ok((src, sel, radius, model))
}

fn run_refine(s: &mut Session, p: &Params) -> Result<CommandOutput> {
    sync_folder(s);
    let output = p.opt_str("output")?.unwrap_or_else(|| s.prefs.get_str("AiRefineOutput", "selection"));
    if output != "selection" && output != "mask" {
        return Err(EngineError::InvalidParams { name: p.command.clone(), message: fp_i18n::t("err-param-output") });
    }
    let (src, sel, radius, model) = refine_inputs(s, p)?;
    ensure_model(model)?;
    let refined = pipeline::refine_edge(&src.image, &sel, radius, model, &Progress::new()).map_err(ai_err)?;
    fp_ai::nn::trim();
    let mut refined = refined;
    refined.set_soft_edge(SoftEdge::One);
    if output == "selection" {
        let value = json!({"source": src.kind, "bounds": bounds_json(Some(&refined)), "output": "selection"});
        return set_selection(s, Some(refined), value);
    }
    // Bitmapmaske auf der Quell-Bitmap (genau ein Verlaufsschritt)
    let target = src.target.ok_or_else(|| sel_err("err-sel-mask-needs-bitmap"))?;
    {
        let d = s.doc()?;
        let path = docops::find_on_page(&d.doc, d.page, target).ok_or(EngineError::ElementNotFound(target.0))?;
        if docops::is_locked(&d.doc, d.page, &path) {
            return Err(sel_err("err-sel-bitmap-locked"));
        }
    }
    let d = s.doc_mut()?;
    d.selection.set(vec![target]);
    d.selection.pixels = Some(refined);
    let out = super::modify::mask_from_pixels(s, true)?;
    Ok(out.labeled(fp_i18n::t("msg-step-refine-edge")))
}

// ------------------------------------------------------------------ Vorwärmen für die Oberfläche

/// Vorbereitete Hintergrundrechnung (nur Pixel und Parameter, kein Zugriff auf die Sitzung).
pub struct Prepared {
    kind: PrepKind,
    /// Modell (für den Fortschrittstext).
    pub model: ModelId,
    /// Kennung der Rechnung (Bildinhalt, Ausschnitt, Modell, Art): gleiche Kennung = dieselbe
    /// Vorbereitung, ein zweiter Klick braucht sie nicht neu zu starten.
    pub key: u64,
}

fn prep_key(img: &RgbaImage, region: Option<IRect>, model: ModelId, kind: u64) -> u64 {
    pipeline::image_key(img, region.unwrap_or_else(|| img.bounds()), model) ^ kind.wrapping_mul(0x9e37_79b9_7f4a_7c15)
}

enum PrepKind {
    Object { image: Arc<RgbaImage>, region: Option<IRect> },
    Subject { image: Arc<RgbaImage>, region: Option<IRect> },
    Refine { image: Arc<RgbaImage>, sel: Mask, radius: f32 },
}

/// Ist das Modell eines KI-Befehls vorhanden? `Some(id)` = fehlt.
pub fn missing_model(s: &Session, name: &str, params: &Value) -> Result<Option<ModelId>> {
    sync_folder(s);
    let p = Params::new(name, params.clone());
    let task = match name {
        "select.aiObject" => Task::Object,
        "select.aiSubject" => Task::Subject,
        "select.aiRefineEdge" => Task::Matting,
        _ => return Ok(None),
    };
    let id = model_for(s, Some(&p), task)?;
    Ok(if models::available(id) { None } else { Some(id) })
}

/// Was muss vor dem Befehl im Hintergrund gerechnet werden? `None` = nichts (Zwischenspeicher
/// warm, der Befehl ist sofort fertig).
pub fn prepare(s: &Session, name: &str, params: &Value) -> Result<Option<Prepared>> {
    sync_folder(s);
    let p = Params::new(name, params.clone());
    match name {
        "select.aiObject" => {
            let plan = object_plan(s, &p)?;
            ensure_model(plan.model)?;
            if pipeline::object_ready(&plan.src.image, plan.region, plan.model) {
                return Ok(None);
            }
            let key = prep_key(&plan.src.image, plan.region, plan.model, 1);
            Ok(Some(Prepared { kind: PrepKind::Object { image: plan.src.image, region: plan.region }, model: plan.model, key }))
        }
        "select.aiSubject" => {
            let model = model_for(s, Some(&p), Task::Subject)?;
            ensure_model(model)?;
            let src = resolve_source(s, &source_param(s, &p)?, p.opt_point("point")?)?;
            let region = region_param(&p)?;
            if pipeline::subject_ready(&src.image, region, model) {
                return Ok(None);
            }
            let key = prep_key(&src.image, region, model, 2);
            Ok(Some(Prepared { kind: PrepKind::Subject { image: src.image, region }, model, key }))
        }
        "select.aiRefineEdge" => {
            let (src, sel, radius, model) = refine_inputs(s, &p)?;
            ensure_model(model)?;
            if pipeline::refine_ready(&src.image, &sel, radius, model) {
                return Ok(None);
            }
            let key = prep_key(&src.image, None, model, 3) ^ mask_hash(Some(&sel)) ^ radius.to_bits() as u64;
            Ok(Some(Prepared { kind: PrepKind::Refine { image: src.image, sel, radius }, model, key }))
        }
        _ => Ok(None),
    }
}

/// Rechnet eine Vorbereitung (im Hintergrund-Thread aufrufen); abbrechbar über `prog`.
pub fn run_prepared(pr: &Prepared, prog: &Progress) -> fp_ai::Result<()> {
    let r = match &pr.kind {
        PrepKind::Object { image, region } => pipeline::object_prepare(image, *region, pr.model, prog).map(|_| ()),
        PrepKind::Subject { image, region } => pipeline::subject_mask(image, *region, pr.model, prog).map(|_| ()),
        PrepKind::Refine { image, sel, radius } => pipeline::refine_edge(image, sel, *radius, pr.model, prog).map(|_| ()),
    };
    fp_ai::nn::trim();
    r
}

/// Ist das ein KI-Befehl, den die Oberfläche im Hintergrund vorbereitet?
pub fn is_ai_command(name: &str) -> bool {
    matches!(name, "select.aiObject" | "select.aiSubject" | "select.aiRefineEdge")
}

// ------------------------------------------------------------------ Registrierung

fn source_kind() -> ParamKind {
    ParamKind::Enum { values: ["auto", "bitmap", "page"].iter().map(|s| s.to_string()).collect() }
}

pub(super) fn register(r: &mut CommandRegistry) {
    r.add(
        cmd("select.aiObject")
            .label("Objektauswahl")
            .describe("Wählt das angeklickte Objekt per KI (SAM 2.1) als Pixelauswahl aus")
            .requires(DOC)
            .opt("points", ParamKind::Json, "Klickpunkte [[x, y, label], …] (label 1 = Objekt, 0 = Hintergrund)")
            .opt("point", ParamKind::Point, "einzelner Klickpunkt")
            .opt("label", ParamKind::Json, "Label des einzelnen Punktes (1/0)")
            .opt("box", ParamKind::Rect, "Rahmen um das Objekt")
            .opt_default("mode", mode_param(), json!("replace"), "Verknüpfung mit der bisherigen Auswahl")
            .opt("session", ParamKind::Integer { min: Some(0), max: None }, "Sitzung einer laufenden Objektauswahl (aus dem Ergebnis)")
            .opt("refine", ParamKind::Bool, "Punkte zur laufenden Sitzung hinzufügen (Umschalt/Alt-Klick)")
            .opt_default("source", source_kind(), json!("auto"), "Bildquelle: Bitmap oder sichtbare Seite")
            .opt("region", ParamKind::Rect, "nur dieser Ausschnitt (z. B. sichtbarer Bereich)")
            .opt("view", ParamKind::Rect, "sichtbarer Bereich: bei starker Vergrößerung wird nur er codiert")
            .opt("model", ParamKind::String, "sam2Tiny (Vorgabe) oder sam2Small")
            .opt_default("feather", ParamKind::Integer { min: Some(0), max: Some(1000) }, json!(0), "weiche Kante")
            .run(run_object),
    );
    r.add(
        cmd("select.aiSubject")
            .label("Motiv auswählen")
            .describe("Wählt das Hauptmotiv der Bitmap bzw. der sichtbaren Seite per KI (BiRefNet) als weiche Pixelauswahl aus")
            .requires(DOC)
            .opt_default("mode", mode_param(), json!("replace"), "Verknüpfung mit der bisherigen Auswahl")
            .opt_default("source", source_kind(), json!("auto"), "Bildquelle")
            .opt("point", ParamKind::Point, "Punkt (wählt die Bitmap unter dem Punkt)")
            .opt("region", ParamKind::Rect, "nur dieser Ausschnitt")
            .opt("model", ParamKind::String, "birefnetLite (Vorgabe) oder birefnet")
            .opt_default("feather", ParamKind::Integer { min: Some(0), max: Some(1000) }, json!(0), "weiche Kante")
            .run(run_subject),
    );
    r.add(
        cmd("select.aiRefineEdge")
            .label("Kante verfeinern")
            .describe("Verfeinert die Kante der Pixelauswahl per KI (Haare, Fell) mit echter Halbtransparenz")
            .requires(&[Requirement::Document, Requirement::PixelSelection])
            .opt("radius", ParamKind::Number { min: Some(0.0), max: Some(500.0) }, "Breite des Kantenbereichs in Pixeln (0 = automatisch)")
            .opt_default("output", ParamKind::Enum { values: vec!["selection".into(), "mask".into()] }, json!("selection"), "Ergebnis als weiche Pixelauswahl oder als Bitmapmaske")
            .opt_default("source", source_kind(), json!("auto"), "Bildquelle")
            .opt("model", ParamKind::String, "birefnetLiteMatting (Vorgabe) oder birefnetHrMatting")
            .run(run_refine),
    );
    r.add(cmd("ai.models").label("KI-Modelle").describe("Liste der KI-Modelle mit Quelle, Größe, Lizenz und Verfügbarkeit").run(|s, _| {
        sync_folder(s);
        let list: Vec<Value> = models::MODELS
            .iter()
            .map(|m| {
                json!({
                    "id": m.id, "name": m.name, "task": m.task, "file": m.file, "url": m.url, "page": m.page,
                    "size": m.size, "sha256": m.sha256, "license": m.license, "note": m.note, "default": m.default,
                    "available": models::available(m.id),
                    "path": models::find(m.id).map(|p| p.display().to_string()),
                })
            })
            .collect();
        let folders: Vec<String> = models::search_folders().iter().map(|p| p.display().to_string()).collect();
        Ok(CommandOutput::value(json!({"models": list, "folders": folders, "downloadFolder": models::download_folder().display().to_string()})))
    }));
    r.add(
        cmd("ai.setModelFolder")
            .label("Modellordner festlegen")
            .param("path", ParamKind::Path, "Ordner mit den KI-Modellen (leer = Standard)")
            .run(|s, p| {
                let path = p.str("path")?;
                s.prefs.set("AiModelFolder", crate::prefs::PrefValue::Str(path));
                sync_folder(s);
                s.emit(SessionEvent::PreferencesChanged);
                Ok(CommandOutput::session())
            }),
    );
    r.add(cmd("ai.clearCache").label("KI-Zwischenspeicher leeren").run(|_, _| {
        fp_ai::cache::clear_results();
        fp_ai::cache::unload_all();
        fp_ai::nn::trim();
        Ok(CommandOutput::value(Value::Null))
    }));
    r.add(cmd("ai.manageModels").label("KI-Modelle verwalten").describe("Zeigt die KI-Modelle und lädt fehlende nach Zustimmung herunter").run(|s, _| {
        // Ohne Oberfläche: Liste als Ergebnis; die Oberfläche öffnet stattdessen den Dialog.
        let reg = Arc::clone(s.registry());
        let c = reg.get("ai.models").ok_or_else(|| EngineError::UnknownCommand("ai.models".into()))?;
        c.apply(s, &Params::new("ai.models", Value::Null))
    }));
}

//! Schritt-Schnittstelle (SPEC-BATCH §5.1) und Register. Spätere Pakete (BATCH-4 … BATCH-9) legen
//! ihre Schritte in `src\steps\<gruppe>.rs` an und hängen in `steps\mod.rs` **eine** Zeile
//! `<gruppe>::register(r);` an.
//!
//! Texte: Schrittname `batch-step-<id>` (Punkte als `-`, z. B. `batch-step-file-copyOnly`),
//! Beschreibung `batch-step-<id>.description`, Parameter `batch-param-<id>-<param>` (Tooltip als
//! `.tooltip`), Auswahlwerte `batch-value-<id>-<param>-<wert>` (siehe [`step_key`], [`param_key`],
//! [`value_key`]); Schlüssel in `locales\{de,en}\batch*.ftl`.

use crate::error::{BatchError, Result};
use crate::executor::CancelToken;
use crate::item::{Item, ItemInfo, ItemKind};
use crate::output::OutputFormat;
use serde::Serialize;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

/// Gruppe (Farbe der Schrittkarte, Gliederung der Galerie).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum StepGroup {
    Files,
    Format,
    Geometry,
    Color,
    Background,
    Overlay,
    Metadata,
    Animation,
    Optimize,
    Document,
    Advanced,
}

impl StepGroup {
    /// Technischer Name (`files`, `geometry` …), auch Teil des Textschlüssels `batch-group-<name>`.
    pub fn name(self) -> &'static str {
        match self {
            StepGroup::Files => "files",
            StepGroup::Format => "format",
            StepGroup::Geometry => "geometry",
            StepGroup::Color => "color",
            StepGroup::Background => "background",
            StepGroup::Overlay => "overlay",
            StepGroup::Metadata => "metadata",
            StepGroup::Animation => "animation",
            StepGroup::Optimize => "optimize",
            StepGroup::Document => "document",
            StepGroup::Advanced => "advanced",
        }
    }

    /// Name in der aktuellen Sprache.
    pub fn label(self) -> String {
        fp_i18n::t(&format!("batch-group-{}", self.name()))
    }
}

/// Art des Schritts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum StepKind {
    /// Wirkt auf jedes Element einzeln (parallel).
    PerItem,
    /// Sammel-Schritt: alle Elemente → neue Elemente (z. B. `anim.fromFrames`), zweite Phase.
    Collect,
    /// Feste Endkarte (`rename`, `output`): steht nicht in `steps`, sondern in `rename`/`output`.
    Final,
}

/// Typ eines Parameters (für das generische Formular der Oberfläche und die Prüfung).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ParamType {
    Bool,
    Int {
        min: i64,
        max: i64,
    },
    Number {
        min: f64,
        max: f64,
    },
    /// Einer der Werte.
    Enum {
        values: Vec<String>,
    },
    /// Mehrere der Werte (Liste).
    Multi {
        values: Vec<String>,
    },
    /// `#rrggbb[aa]` oder `transparent`.
    Color,
    Text,
    Path,
    /// Zeit in ms oder `"m:ss.mmm"`.
    Time,
    /// Beliebiges JSON (Bausteine, Punkte …).
    Json,
}

/// Bedingung für die Sichtbarkeit eines Parameters im Formular (BATCH-11a): der Parameter `param`
/// hat (mit Vorgabe) einen der Werte `values` (Texte; Wahrheitswerte als `true`/`false`, Zahlen
/// als Text). Versteckte Parameter behalten ihren Wert, sie wirken nur nicht im Formular und in der
/// Zusammenfassung.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParamCond {
    pub param: String,
    pub values: Vec<String>,
}

/// Beschreibung eines Parameters.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParamDef {
    pub name: String,
    #[serde(flatten)]
    pub ty: ParamType,
    pub default: Value,
    /// Im Anfänger-Modus sichtbar (Markierung **A** in SPEC-BATCH §3).
    pub beginner: bool,
    /// Einheit (`px`, `%`, `ms` …), gedämpft hinter dem Feld.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// Nur sichtbar, wenn ein anderer Parameter einen bestimmten Wert hat ([`ParamDef::when`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visible_when: Option<ParamCond>,
}

/// Wert eines Parameters als Vergleichstext für [`ParamCond`].
pub fn cond_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

impl ParamDef {
    /// Parameter ohne Vorgabe, Bedingung und Einheit (für eigene Schema-Bausteine der Schritte).
    pub fn new(name: &str, ty: ParamType, default: Value) -> ParamDef {
        ParamDef {
            name: name.to_string(),
            ty,
            default,
            beginner: false,
            unit: None,
            visible_when: None,
        }
    }
    pub fn bool(name: &str, default: bool) -> ParamDef {
        ParamDef::new(name, ParamType::Bool, json!(default))
    }
    pub fn int(name: &str, min: i64, max: i64, default: i64) -> ParamDef {
        ParamDef::new(name, ParamType::Int { min, max }, json!(default))
    }
    pub fn number(name: &str, min: f64, max: f64, default: f64) -> ParamDef {
        ParamDef::new(name, ParamType::Number { min, max }, json!(default))
    }
    pub fn choice(name: &str, values: &[&str], default: &str) -> ParamDef {
        ParamDef::new(
            name,
            ParamType::Enum {
                values: values.iter().map(|s| s.to_string()).collect(),
            },
            json!(default),
        )
    }
    pub fn multi(name: &str, values: &[&str], default: &[&str]) -> ParamDef {
        ParamDef::new(
            name,
            ParamType::Multi {
                values: values.iter().map(|s| s.to_string()).collect(),
            },
            json!(default),
        )
    }
    pub fn color(name: &str, default: &str) -> ParamDef {
        ParamDef::new(name, ParamType::Color, json!(default))
    }
    pub fn text(name: &str, default: &str) -> ParamDef {
        ParamDef::new(name, ParamType::Text, json!(default))
    }
    pub fn path(name: &str) -> ParamDef {
        ParamDef::new(name, ParamType::Path, Value::Null)
    }
    pub fn time(name: &str, default: Value) -> ParamDef {
        ParamDef::new(name, ParamType::Time, default)
    }
    pub fn json(name: &str, default: Value) -> ParamDef {
        ParamDef::new(name, ParamType::Json, default)
    }
    /// Im Anfänger-Modus sichtbar.
    pub fn beginner(mut self) -> ParamDef {
        self.beginner = true;
        self
    }
    /// Einheit.
    pub fn unit(mut self, u: &str) -> ParamDef {
        self.unit = Some(u.to_string());
        self
    }
    /// Nur sichtbar, wenn `param` einen der Werte `values` hat (z. B. `size` nur bei
    /// `mode` = `longEdge`/`shortEdge`).
    pub fn when(mut self, param: &str, values: &[&str]) -> ParamDef {
        self.visible_when = Some(ParamCond {
            param: param.to_string(),
            values: values.iter().map(|s| s.to_string()).collect(),
        });
        self
    }
    /// Sichtbar bei diesen Parametern? `defs` liefert die Vorgaben der anderen Parameter.
    pub fn is_visible(&self, defs: &[ParamDef], params: &Value) -> bool {
        let Some(c) = &self.visible_when else {
            return true;
        };
        let v = params
            .get(&c.param)
            .filter(|v| !v.is_null())
            .cloned()
            .or_else(|| {
                defs.iter()
                    .find(|d| d.name == c.param)
                    .map(|d| d.default.clone())
            })
            .unwrap_or(Value::Null);
        // die Bedingung selbst muss sichtbar sein (Ketten wie engine → turnPolicy)
        let parent_visible = defs
            .iter()
            .find(|d| d.name == c.param)
            .is_none_or(|d| d.name == self.name || d.is_visible(defs, params));
        parent_visible && c.values.iter().any(|x| *x == cond_text(&v))
    }

    /// Prüft einen gesetzten Wert. `None` = in Ordnung.
    pub fn check(&self, v: &Value) -> Option<String> {
        let bad = || {
            Some(fp_i18n::tr!(
                "batch-issue-param-type",
                param = self.name.clone()
            ))
        };
        match &self.ty {
            ParamType::Bool => (!v.is_boolean()).then(bad).flatten(),
            ParamType::Int { min, max } => match v
                .as_i64()
                .or_else(|| v.as_f64().filter(|f| f.fract() == 0.0).map(|f| f as i64))
            {
                None => bad(),
                Some(x) if x < *min || x > *max => Some(fp_i18n::tr!(
                    "batch-issue-param-range",
                    param = self.name.clone(),
                    min = *min,
                    max = *max
                )),
                _ => None,
            },
            ParamType::Number { min, max } => match v.as_f64() {
                None => bad(),
                Some(x) if !x.is_finite() || x < *min || x > *max => Some(fp_i18n::tr!(
                    "batch-issue-param-range",
                    param = self.name.clone(),
                    min = *min,
                    max = *max
                )),
                _ => None,
            },
            ParamType::Enum { values } => match v.as_str() {
                Some(s) if values.iter().any(|x| x == s) => None,
                _ => Some(fp_i18n::tr!(
                    "batch-issue-param-enum",
                    param = self.name.clone(),
                    values = values.join(", ")
                )),
            },
            ParamType::Multi { values } => match v.as_array() {
                Some(a)
                    if a.iter()
                        .all(|x| x.as_str().is_some_and(|s| values.iter().any(|y| y == s))) =>
                {
                    None
                }
                _ => Some(fp_i18n::tr!(
                    "batch-issue-param-enum",
                    param = self.name.clone(),
                    values = values.join(", ")
                )),
            },
            ParamType::Color => match v.as_str() {
                Some(s) if parse_color(s).is_some() => None,
                _ => bad(),
            },
            ParamType::Text | ParamType::Path => {
                (!(v.is_string() || v.is_null())).then(bad).flatten()
            }
            ParamType::Time => (!(v.is_number() || v.is_string() || v.is_null()))
                .then(bad)
                .flatten(),
            ParamType::Json => None,
        }
    }
}

/// `#rgb`, `#rrggbb`, `#rrggbbaa` oder `transparent` → RGBA.
pub fn parse_color(s: &str) -> Option<[u8; 4]> {
    let s = s.trim();
    if s.eq_ignore_ascii_case("transparent") || s.eq_ignore_ascii_case("none") {
        return Some([0, 0, 0, 0]);
    }
    let h = s.strip_prefix('#').unwrap_or(s);
    let byte = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
    match h.len() {
        3 => {
            let d = |i: usize| {
                u8::from_str_radix(h.get(i..i + 1)?, 16)
                    .ok()
                    .map(|x| x * 17)
            };
            Some([d(0)?, d(1)?, d(2)?, 255])
        }
        6 => Some([byte(0)?, byte(2)?, byte(4)?, 255]),
        8 => Some([byte(0)?, byte(2)?, byte(4)?, byte(6)?]),
        _ => None,
    }
}

/// Schwere eines Hinweises.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// Prüfergebnis (Validierung von Rezept und Schritten).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    /// Index des Schritts in `recipe.steps` (None = Rezept/Endkarten).
    pub step: Option<usize>,
    pub step_id: Option<String>,
    /// Betroffener Parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<String>,
    pub severity: Severity,
    /// Technischer Code (englisch, stabil).
    pub code: String,
    /// Meldung in der aktuellen Sprache.
    pub message: String,
}

impl Issue {
    /// Hinweis zu einem Schritt.
    pub fn step(index: usize, id: &str, severity: Severity, code: &str, message: String) -> Issue {
        Issue {
            step: Some(index),
            step_id: Some(id.to_string()),
            param: None,
            severity,
            code: code.to_string(),
            message,
        }
    }
    /// Hinweis zum ganzen Rezept.
    pub fn global(severity: Severity, code: &str, message: String) -> Issue {
        Issue {
            step: None,
            step_id: None,
            param: None,
            severity,
            code: code.to_string(),
            message,
        }
    }
    /// Fehler eines Parameters (Schritt-Index setzt [`crate::Recipe::validate`]).
    pub fn param(param: &str, message: String) -> Issue {
        Issue {
            step: None,
            step_id: None,
            param: Some(param.to_string()),
            severity: Severity::Error,
            code: "invalidParam".into(),
            message,
        }
    }
}

/// Prüft Parameter gegen ihr Schema (unbekannte Parameter werden ignoriert, fehlende haben ihre Vorgabe).
pub fn validate_params(defs: &[ParamDef], params: &Value) -> Vec<Issue> {
    let mut out = Vec::new();
    if !(params.is_object() || params.is_null()) {
        out.push(Issue::param(
            "",
            fp_i18n::t("batch-issue-params-not-object"),
        ));
        return out;
    }
    for d in defs {
        if let Some(v) = params.get(&d.name).filter(|v| !v.is_null())
            && let Some(m) = d.check(v)
        {
            out.push(Issue::param(&d.name, m));
        }
    }
    out
}

/// Parameter mit Vorgaben aufgefüllt.
pub fn with_defaults(defs: &[ParamDef], params: &Value) -> Value {
    let mut o = params.as_object().cloned().unwrap_or_default();
    for d in defs {
        if !o.contains_key(&d.name) && !d.default.is_null() {
            o.insert(d.name.clone(), d.default.clone());
        }
    }
    Value::Object(o)
}

/// Kontext eines Schritts für ein Element.
pub struct StepContext<'a> {
    /// Position des Elements in der (gefilterten, sortierten) Liste.
    pub index: usize,
    pub total: usize,
    pub info: &'a ItemInfo,
    pub cancel: &'a CancelToken,
    /// Vorschau (Proxy-Bild) statt Lauf.
    pub preview: bool,
    /// Zielformat der Endkarte „Speichern als“ (aufgelöst, nie `Same`).
    pub output_format: OutputFormat,
    /// Index des laufenden Schritts in `recipe.steps`.
    pub step_index: usize,
    /// Warnungen (übersetzt), landen im Bericht.
    pub warnings: Vec<String>,
    /// Maßstab des Vorschau-Proxys gegenüber dem geladenen Original (≤ 1; `None` = unbekannt bzw.
    /// kein Proxy). Gesetzt von [`crate::pipeline::preview`]; Schritte rechnen Pixelangaben
    /// (Rahmenbreite, Schattenabstand, Radius …) damit um, damit die Vorschau wie das Ergebnis
    /// aussieht – unabhängig von Größenschritten davor. Abfrage über [`StepContext::preview_scale`].
    pub proxy_scale: Option<f64>,
}

impl<'a> StepContext<'a> {
    /// Neuer Kontext.
    pub fn new(
        index: usize,
        total: usize,
        info: &'a ItemInfo,
        cancel: &'a CancelToken,
        output_format: OutputFormat,
    ) -> StepContext<'a> {
        StepContext {
            index,
            total,
            info,
            cancel,
            preview: false,
            output_format,
            step_index: 0,
            warnings: Vec::new(),
            proxy_scale: None,
        }
    }
    /// Maßstab der Vorschau für Pixelangaben: 1,0 außerhalb der Vorschau, sonst der Proxy-Maßstab
    /// (`None`, wenn ihn niemand gesetzt hat – dann schätzen die Schritte wie bisher).
    pub fn preview_scale(&self) -> Option<f64> {
        if !self.preview {
            return Some(1.0);
        }
        self.proxy_scale
            .filter(|s| s.is_finite() && *s > 0.0)
            .map(|s| s.min(1.0))
    }
    /// Warnung merken.
    pub fn warn(&mut self, msg: impl Into<String>) {
        self.warnings.push(msg.into());
    }
    /// Fehler, wenn abgebrochen (zwischen Kachelzeilen/Frames aufrufen, spätestens alle 100 ms).
    pub fn check_cancel(&self) -> Result<()> {
        if self.cancel.is_cancelled() {
            Err(BatchError::cancelled())
        } else {
            Ok(())
        }
    }
}

/// Ein Schritt.
pub trait Step: Send + Sync {
    /// Technische ID im Rezept (`resize`, `file.copyOnly` …).
    fn id(&self) -> &str;
    fn group(&self) -> StepGroup;
    fn kind(&self) -> StepKind {
        StepKind::PerItem
    }
    /// Parameter-Schema (Formular der Oberfläche, Prüfung, Katalog).
    fn params(&self) -> Vec<ParamDef> {
        Vec::new()
    }
    /// Prüfung der Parameter. Vorgabe: gegen [`Step::params`].
    fn validate(&self, params: &Value) -> Vec<Issue> {
        validate_params(&self.params(), params)
    }
    /// Element-Arten, auf die der Schritt wirkt (sonst grau mit Hinweis, Element unverändert).
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::ALL
    }
    /// In der Live-Vorschau rechnen (sonst übersprungen und als Hinweis genannt).
    fn preview_ok(&self) -> bool {
        true
    }
    /// Reiner Dateischritt (kein Dekodieren nötig, §5.3).
    fn file_only(&self) -> bool {
        false
    }
    /// Höchstzahl gleichzeitig laufender Elemente mit diesem Schritt (`None` = frei). KI-Schritte
    /// rechnen selbst mehrfädig und brauchen viel Speicher: dort wenige Elemente zugleich (BATCH-T).
    fn max_parallel(&self, _params: &Value) -> Option<usize> {
        None
    }
    /// Wendet den Schritt an. `params` sind mit den Vorgaben aufgefüllt.
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()>;
    /// Sammel-Schritt (nur [`StepKind::Collect`]): alle Elemente → neue Elemente.
    fn collect(
        &self,
        _items: Vec<Item>,
        _params: &Value,
        _ctx: &mut StepContext,
    ) -> Result<Vec<Item>> {
        Err(BatchError::Unsupported(fp_i18n::tr!(
            "batch-err-not-collect",
            id = self.id().to_string()
        )))
    }
    /// Zusammenfassung in Klartext für die zugeklappte Karte (aktuelle Sprache). Vorgabe: leer.
    fn summary(&self, _params: &Value) -> String {
        String::new()
    }
}

/// Textschlüssel des Schrittnamens: `batch-step-` + ID mit `-` statt `.`.
pub fn step_key(id: &str) -> String {
    format!("batch-step-{}", id.replace('.', "-"))
}

/// Textschlüssel eines Parameters: `batch-param-<id>-<param>`.
pub fn param_key(id: &str, param: &str) -> String {
    format!("batch-param-{}-{}", id.replace('.', "-"), param)
}

/// Textschlüssel eines Auswahlwerts (einheitliches Schema seit BATCH-11a):
/// `batch-value-<id>-<param>-<wert>` (Punkte der ID als `-`), z. B.
/// `batch-value-resize-mode-longEdge`. Gilt für `choice()`- und `multi()`-Parameter aller Schritte
/// und der Endkarten.
pub fn value_key(id: &str, param: &str, value: &str) -> String {
    format!("batch-value-{}-{}-{}", id.replace('.', "-"), param, value)
}

/// Name eines Schritts in der aktuellen Sprache (Rückfall: ID).
pub fn step_label(id: &str) -> String {
    let k = step_key(id);
    if fp_i18n::has(&k) {
        fp_i18n::t(&k)
    } else {
        id.to_string()
    }
}

/// Register aller Schritte.
#[derive(Default, Clone)]
pub struct StepRegistry {
    steps: BTreeMap<String, Arc<dyn Step>>,
}

impl std::fmt::Debug for StepRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.steps.keys()).finish()
    }
}

impl StepRegistry {
    /// Leeres Register.
    pub fn new() -> StepRegistry {
        StepRegistry::default()
    }

    /// Register mit allen eingebauten Schritten (`steps::register_all`).
    pub fn with_defaults() -> StepRegistry {
        let mut r = StepRegistry::new();
        crate::steps::register_all(&mut r);
        r
    }

    /// Gemeinsames Register mit den eingebauten Schritten.
    pub fn global() -> Arc<StepRegistry> {
        static GLOBAL: OnceLock<Arc<StepRegistry>> = OnceLock::new();
        GLOBAL
            .get_or_init(|| Arc::new(StepRegistry::with_defaults()))
            .clone()
    }

    /// Schritt hinzufügen (gleiche ID ersetzt).
    pub fn register(&mut self, step: Arc<dyn Step>) {
        self.steps.insert(step.id().to_string(), step);
    }

    pub fn get(&self, id: &str) -> Option<&Arc<dyn Step>> {
        self.steps.get(id)
    }

    pub fn contains(&self, id: &str) -> bool {
        self.steps.contains_key(id)
    }

    /// Alle Schritte, nach ID sortiert.
    pub fn iter(&self) -> impl Iterator<Item = &Arc<dyn Step>> {
        self.steps.values()
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Katalog (JSON) für Agenten, CLI `batch --steps` und `batch.steps`: Schritte nach Gruppe, mit
    /// Namen/Beschreibung in der aktuellen Sprache und Parameter-Schema.
    pub fn catalog_json(&self) -> Value {
        let mut list: Vec<&Arc<dyn Step>> = self.steps.values().collect();
        list.sort_by_key(|s| (s.group(), s.id().to_string()));
        let steps: Vec<Value> = list
            .iter()
            .map(|s| {
                let id = s.id();
                let desc_key = format!("{}.description", step_key(id));
                let params: Vec<Value> = s
                    .params()
                    .iter()
                    .map(|p| {
                        let mut v = serde_json::to_value(p).unwrap_or(Value::Null);
                        let k = param_key(id, &p.name);
                        if fp_i18n::has(&k) {
                            v["label"] = json!(fp_i18n::t(&k));
                        }
                        if let ParamType::Enum { values } | ParamType::Multi { values } = &p.ty {
                            let labels: Map<String, Value> = values
                                .iter()
                                .filter_map(|x| {
                                    let vk = value_key(id, &p.name, x);
                                    fp_i18n::has(&vk).then(|| (x.clone(), json!(fp_i18n::t(&vk))))
                                })
                                .collect();
                            if !labels.is_empty() {
                                v["valueLabels"] = Value::Object(labels);
                            }
                        }
                        v
                    })
                    .collect();
                let mut o = Map::new();
                o.insert("id".into(), json!(id));
                o.insert("group".into(), json!(s.group().name()));
                o.insert(
                    "kind".into(),
                    serde_json::to_value(s.kind()).unwrap_or(Value::Null),
                );
                o.insert("name".into(), json!(step_label(id)));
                if fp_i18n::has(&desc_key) {
                    o.insert("description".into(), json!(fp_i18n::t(&desc_key)));
                }
                o.insert("previewOk".into(), json!(s.preview_ok()));
                o.insert("fileOnly".into(), json!(s.file_only()));
                o.insert(
                    "appliesTo".into(),
                    serde_json::to_value(s.applies_to()).unwrap_or(Value::Null),
                );
                o.insert("params".into(), Value::Array(params));
                Value::Object(o)
            })
            .collect();
        json!({"format": "firepixel-batch-steps", "version": crate::recipe::RECIPE_VERSION, "steps": steps})
    }
}

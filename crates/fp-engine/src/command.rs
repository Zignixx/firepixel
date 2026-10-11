//! Befehlsregister (ARCHITEKTUR §3): ein Register für Menü, Werkzeuge, Dialoge, Bedienfelder, CLI,
//! MCP und JSF.
//!
//! Jeder Befehl hat einen stabilen Namen (`modify.group`, `filter.blur.gaussian`, …), eine
//! JSON-Parameterbeschreibung (für MCP als JSON-Schema, [`CommandInfo::params_schema`]), eine
//! deutsche Beschriftung, den Menüpfad, die Original-Befehls-ID mit `cmd_`-Namen (Schlüssel der
//! Tastenkürzel-Sets), ein Verfügbarkeits-Prädikat ([`Requirement`]) und die Ausführung
//! ([`Command::apply`]). Ausgeführt wird immer über [`crate::Session::execute`]; die Sitzung prüft
//! die Verfügbarkeit, legt den Verlaufsschritt an und meldet Ereignisse.
//!
//! Alle Katalogeinträge aus `menuebefehle.json`, die noch keine Wirkung haben, registriert
//! [`CommandRegistry::with_defaults`] als Platzhalter mit [`CommandInfo::implemented`] = `false`,
//! damit Menüs vollständig sind.

use crate::catalog::{self, CatalogEntry};
use crate::error::{EngineError, Result};
use crate::session::Session;
use crate::shortcuts::{Shortcut, ShortcutSet};
use fp_model::symbol::SymbolKind;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Typ eines Parameters.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ParamKind {
    Bool,
    Integer { min: Option<i64>, max: Option<i64> },
    Number { min: Option<f64>, max: Option<f64> },
    String,
    /// Einer der Werte.
    Enum { values: Vec<String> },
    /// Farbe `#rrggbb[aa]`.
    Color,
    /// Punkt `{x, y}` oder `[x, y]`.
    Point,
    /// Rechteck `{left, top, right, bottom}` oder `[l, t, r, b]`.
    Rect,
    /// Liste von Element-IDs.
    Ids,
    /// Dateipfad.
    Path,
    /// Beliebiger JSON-Wert (Objekt/Liste).
    Json,
}

/// Beschreibung eines Parameters.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParamSpec {
    pub name: String,
    #[serde(flatten)]
    pub kind: ParamKind,
    pub required: bool,
    /// Kurzbeschreibung (deutsch).
    pub description: String,
    /// Vorgabewert (Dokumentation).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
}

impl ParamSpec {
    fn new(name: &str, kind: ParamKind, required: bool, desc: &str) -> ParamSpec {
        ParamSpec { name: name.into(), kind, required, description: desc.into(), default: None }
    }

    /// JSON-Schema-Eigenschaft.
    pub fn schema(&self) -> Value {
        let mut s = match &self.kind {
            ParamKind::Bool => json!({"type": "boolean"}),
            ParamKind::Integer { min, max } => {
                let mut o = json!({"type": "integer"});
                if let Some(m) = min {
                    o["minimum"] = json!(m);
                }
                if let Some(m) = max {
                    o["maximum"] = json!(m);
                }
                o
            }
            ParamKind::Number { min, max } => {
                let mut o = json!({"type": "number"});
                if let Some(m) = min {
                    o["minimum"] = json!(m);
                }
                if let Some(m) = max {
                    o["maximum"] = json!(m);
                }
                o
            }
            ParamKind::String | ParamKind::Path => json!({"type": "string"}),
            ParamKind::Enum { values } => json!({"type": "string", "enum": values}),
            ParamKind::Color => json!({"type": "string", "pattern": "^#?[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$"}),
            ParamKind::Point => json!({"type": ["object", "array"], "description": "{x, y} oder [x, y]"}),
            ParamKind::Rect => json!({"type": ["object", "array"], "description": "{left, top, right, bottom} oder [l, t, r, b]"}),
            ParamKind::Ids => json!({"type": "array", "items": {"type": "integer"}}),
            ParamKind::Json => json!({}),
        };
        if let Value::Object(o) = &mut s {
            o.insert("description".into(), json!(self.description));
            if let Some(d) = &self.default {
                o.insert("default".into(), d.clone());
            }
        }
        s
    }
}

/// Objektart für Objekt-Prädikate (11 §2.3.2: „jedes Element der Auswahl ist X“).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ObjectKind {
    Bitmap,
    Slice,
    Hotspot,
    /// Hotspot oder Segment.
    WebObject,
    RectPrimitive,
    Text,
    /// Element mit Pfadattributen (Pfad, Rechteck, Text, Compound Shape, Gruppen daraus).
    PathAttrs,
    /// Pfad (auch Rechteck-Grundform).
    Path,
    CompoundShape,
    Group,
    MaskGroup,
    Instance,
    GraphicInstance,
    ButtonInstance,
    AnimationInstance,
}

/// Verfügbarkeitsbedingung (alle Bedingungen eines Befehls müssen gelten).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "arg")]
pub enum Requirement {
    /// Ein Dokument ist geöffnet.
    Document,
    /// Mindestens ein Element ausgewählt.
    Selection,
    /// Mindestens `n` Elemente ausgewählt (Mehrfachauswahl `@00dfefe0`: n = 2).
    MinSelected(usize),
    /// Eine Pixelauswahl existiert.
    PixelSelection,
    /// Elemente oder Pixel ausgewählt.
    SelectionOrPixels,
    /// Jedes ausgewählte Element hat diese Art (und es gibt mindestens eines).
    AllOf(ObjectKind),
    /// Mindestens ein ausgewähltes Element hat diese Art.
    AnyOf(ObjectKind),
    /// Die Zwischenablage ist nicht leer.
    Clipboard,
    /// Es gibt einen Rückgängig-Schritt.
    CanUndo,
    /// Es gibt einen Wiederholen-Schritt.
    CanRedo,
    /// Das Dokument hat eine gespeicherte Datei.
    HasPath,
    /// Mehr als eine Seite bzw. ein Status.
    MultiplePages,
    MultipleFrames,
}

impl Requirement {
    /// Schlüssel der Begründung (`meldungen.ftl`), falls nicht erfüllt.
    pub fn reason_key(&self) -> &'static str {
        match self {
            Requirement::Document => "err-req-document",
            Requirement::Selection | Requirement::SelectionOrPixels => "err-req-selection",
            Requirement::MinSelected(_) => "err-req-min-selected",
            Requirement::PixelSelection => "err-req-pixel-selection",
            Requirement::AllOf(_) => "err-req-all-of",
            Requirement::AnyOf(_) => "err-req-any-of",
            Requirement::Clipboard => "err-req-clipboard",
            Requirement::CanUndo => "err-req-can-undo",
            Requirement::CanRedo => "err-req-can-redo",
            Requirement::HasPath => "err-req-has-path",
            Requirement::MultiplePages => "err-req-multiple-pages",
            Requirement::MultipleFrames => "err-req-multiple-frames",
        }
    }

    /// Begründung in der aktuellen Sprache, falls nicht erfüllt.
    pub fn reason(&self) -> String {
        match self {
            Requirement::MinSelected(n) => fp_i18n::tr!(self.reason_key(), n = n.to_string()),
            Requirement::AllOf(k) | Requirement::AnyOf(k) => fp_i18n::tr!(self.reason_key(), kind = format!("{k:?}")),
            _ => fp_i18n::t(self.reason_key()),
        }
    }
}

/// Beschreibung eines Befehls.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandInfo {
    /// Stabiler Name.
    pub name: String,
    /// Deutsche Beschriftung (ohne „…“).
    pub label: String,
    /// Menütext (mit „…“ bei Dialogen).
    pub menu_label: String,
    /// Deutscher Menüpfad (leer = nicht im Hauptmenü).
    pub menu_path: Vec<String>,
    /// Statuszeilentext.
    pub description: Option<String>,
    /// Original-Befehls-ID.
    pub fw_id: Option<u32>,
    /// `cmd_`-Name (Tastenkürzel-Sets).
    pub fw_cmd: Option<String>,
    /// JS-API-Entsprechungen.
    pub js: Vec<String>,
    /// Parameter.
    pub params: Vec<ParamSpec>,
    /// Verfügbarkeits-Prädikat.
    pub requires: Vec<Requirement>,
    /// Hat der Befehl Wirkung? `false` = Platzhalter („nicht umgesetzt“).
    pub implemented: bool,
    /// Menüpunkt mit Häkchen.
    pub checkable: bool,
    /// Hinweis (z. B. „braucht Renderer“).
    pub note: Option<String>,
}

impl CommandInfo {
    /// Name des Verlaufsschritts in der aktuellen Sprache. In der Ausgangssprache (Deutsch) genau
    /// [`CommandInfo::label`]; sonst das Attribut `cmd-….step` (falls vorhanden), der Menütext
    /// `cmd-…` ohne Zugriffstaste und „…“, bei Filtern `filter-…`, zuletzt `label`.
    pub fn step_label(&self) -> String {
        if fp_i18n::current_language() == fp_i18n::SOURCE_LANG {
            return self.label.clone();
        }
        let key = fp_i18n::command_key(&self.name);
        let step = format!("{key}.step");
        if fp_i18n::has(&step) {
            return fp_i18n::t(&step);
        }
        if self.name.starts_with("filter.") {
            let fk = self.name.replace('.', "-");
            if fp_i18n::has(&fk) {
                return fp_i18n::t(&fk);
            }
        }
        if fp_i18n::has(&key) {
            return plain_label(&fp_i18n::t(&key));
        }
        self.label.clone()
    }

    /// JSON-Schema der Parameter (für MCP `describe_command`).
    pub fn params_schema(&self) -> Value {
        let mut props = Map::new();
        let mut req = Vec::new();
        for p in &self.params {
            props.insert(p.name.clone(), p.schema());
            if p.required {
                req.push(Value::String(p.name.clone()));
            }
        }
        json!({"type": "object", "properties": props, "required": req, "additionalProperties": true})
    }

    /// Kürzel im angegebenen Set.
    pub fn shortcuts<'a>(&self, set: &'a ShortcutSet) -> &'a [Shortcut] {
        self.fw_cmd.as_deref().map(|c| set.command(c)).unwrap_or(&[])
    }
}

/// Menütext ohne Zugriffstaste (`&`, `&&` = `&`) und ohne abschließendes „…“/„...“.
pub fn plain_label(menu_text: &str) -> String {
    let s = menu_text.replace("&&", "\u{1}").replace('&', "").replace('\u{1}', "&");
    let s = s.trim_end();
    let s = s.strip_suffix('…').or_else(|| s.strip_suffix("...")).unwrap_or(s);
    s.trim_end().to_string()
}

/// Art der Wirkung eines Befehls (steuert Verlauf und Ereignisse).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect {
    /// Dokument und/oder Auswahl geändert → Verlaufsschritt (wenn sich etwas geändert hat).
    Document,
    /// Nur Ansicht (Zoom, Raster …), kein Verlaufsschritt.
    View,
    /// Sitzung (Dokument geöffnet/geschlossen, aktives Werkzeug, Voreinstellungen, Bedienfelder).
    Session,
    /// Keine Änderung (Abfrage, Kopieren).
    None,
}

/// Ergebnis eines Befehls.
#[derive(Clone, Debug, PartialEq)]
pub struct CommandOutput {
    pub effect: Effect,
    /// Abweichende Verlaufsbeschriftung.
    pub label: Option<String>,
    /// Rückgabewert (z. B. neue Element-IDs, Pfad, Farbe).
    pub value: Value,
}

impl CommandOutput {
    /// Dokumentänderung ohne Rückgabewert.
    pub fn doc() -> CommandOutput {
        CommandOutput { effect: Effect::Document, label: None, value: Value::Null }
    }
    /// Dokumentänderung mit Rückgabewert.
    pub fn doc_with(value: Value) -> CommandOutput {
        CommandOutput { effect: Effect::Document, label: None, value }
    }
    /// Ansichtsänderung.
    pub fn view() -> CommandOutput {
        CommandOutput { effect: Effect::View, label: None, value: Value::Null }
    }
    /// Sitzungsänderung.
    pub fn session() -> CommandOutput {
        CommandOutput { effect: Effect::Session, label: None, value: Value::Null }
    }
    /// Sitzungsänderung mit Rückgabewert.
    pub fn session_with(value: Value) -> CommandOutput {
        CommandOutput { effect: Effect::Session, label: None, value }
    }
    /// Keine Änderung, nur Rückgabewert.
    pub fn value(value: Value) -> CommandOutput {
        CommandOutput { effect: Effect::None, label: None, value }
    }
    /// Beschriftung des Verlaufsschritts setzen.
    pub fn labeled(mut self, label: impl Into<String>) -> CommandOutput {
        self.label = Some(label.into());
        self
    }
}

/// Ein Befehl.
pub trait Command: Send + Sync {
    /// Beschreibung.
    fn info(&self) -> &CommandInfo;
    /// Ausführen (über [`Session::execute`] aufrufen, nicht direkt).
    fn apply(&self, session: &mut Session, params: &Params) -> Result<CommandOutput>;
    /// Häkchen-Zustand für abhakbare Menüpunkte.
    fn checked(&self, _session: &Session) -> Option<bool> {
        None
    }
    /// Zusätzliche Verfügbarkeitsprüfung (nach [`CommandInfo::requires`]).
    fn extra_available(&self, _session: &Session) -> bool {
        true
    }
}

type ApplyFn = dyn Fn(&mut Session, &Params) -> Result<CommandOutput> + Send + Sync;
type CheckFn = dyn Fn(&Session) -> bool + Send + Sync;

/// Befehl aus Closures (so sind fast alle eingebauten Befehle umgesetzt).
pub struct FnCommand {
    info: CommandInfo,
    apply: Box<ApplyFn>,
    checked: Option<Box<CheckFn>>,
    avail: Option<Box<CheckFn>>,
}

impl Command for FnCommand {
    fn info(&self) -> &CommandInfo {
        &self.info
    }
    fn apply(&self, session: &mut Session, params: &Params) -> Result<CommandOutput> {
        (self.apply)(session, params)
    }
    fn checked(&self, session: &Session) -> Option<bool> {
        self.checked.as_ref().map(|f| f(session))
    }
    fn extra_available(&self, session: &Session) -> bool {
        self.avail.as_ref().is_none_or(|f| f(session))
    }
}

/// Baukasten für [`FnCommand`].
pub struct CommandBuilder {
    info: CommandInfo,
    checked: Option<Box<CheckFn>>,
    avail: Option<Box<CheckFn>>,
}

/// Neuer Befehl mit Namen; Menüdaten ergänzt das Register aus dem Katalog.
pub fn cmd(name: &str) -> CommandBuilder {
    CommandBuilder {
        info: CommandInfo {
            name: name.into(),
            label: String::new(),
            menu_label: String::new(),
            menu_path: Vec::new(),
            description: None,
            fw_id: None,
            fw_cmd: None,
            js: Vec::new(),
            params: Vec::new(),
            requires: Vec::new(),
            implemented: true,
            checkable: false,
            note: None,
        },
        checked: None,
        avail: None,
    }
}

impl CommandBuilder {
    /// Deutsche Beschriftung (für Befehle außerhalb des Katalogs).
    pub fn label(mut self, l: &str) -> Self {
        self.info.label = l.into();
        self
    }
    /// Statuszeilentext.
    pub fn describe(mut self, d: &str) -> Self {
        self.info.description = Some(d.into());
        self
    }
    /// Verfügbarkeitsbedingungen.
    pub fn requires(mut self, r: &[Requirement]) -> Self {
        self.info.requires.extend_from_slice(r);
        self
    }
    /// Pflichtparameter.
    pub fn param(mut self, name: &str, kind: ParamKind, desc: &str) -> Self {
        self.info.params.push(ParamSpec::new(name, kind, true, desc));
        self
    }
    /// Optionaler Parameter.
    pub fn opt(mut self, name: &str, kind: ParamKind, desc: &str) -> Self {
        self.info.params.push(ParamSpec::new(name, kind, false, desc));
        self
    }
    /// Optionaler Parameter mit Vorgabe.
    pub fn opt_default(mut self, name: &str, kind: ParamKind, default: Value, desc: &str) -> Self {
        let mut p = ParamSpec::new(name, kind, false, desc);
        p.default = Some(default);
        self.info.params.push(p);
        self
    }
    /// Menüpfad (für Befehle außerhalb des Katalogs, z. B. Filter).
    pub fn menu(mut self, path: &[&str]) -> Self {
        self.info.menu_path = path.iter().map(|s| s.to_string()).collect();
        self.info.menu_label = path.last().map(|s| s.to_string()).unwrap_or_default();
        self
    }
    /// Hinweis.
    pub fn note(mut self, n: &str) -> Self {
        self.info.note = Some(n.into());
        self
    }
    /// Original-Befehls-ID (für Befehle außerhalb des Katalogs, z. B. Bedienfeldmenüs).
    pub fn fw_id(mut self, id: u32) -> Self {
        self.info.fw_id = Some(id);
        self
    }
    /// `cmd_`-Name (Tastenkürzel-Sets) für Befehle außerhalb des Katalogs.
    pub fn fw_cmd(mut self, c: &str) -> Self {
        self.info.fw_cmd = Some(c.into());
        self
    }
    /// JS-Entsprechung.
    pub fn js(mut self, j: &str) -> Self {
        self.info.js.push(j.into());
        self
    }
    /// Häkchen-Zustand.
    pub fn checked(mut self, f: impl Fn(&Session) -> bool + Send + Sync + 'static) -> Self {
        self.info.checkable = true;
        self.checked = Some(Box::new(f));
        self
    }
    /// Zusätzliche Verfügbarkeitsprüfung.
    pub fn available(mut self, f: impl Fn(&Session) -> bool + Send + Sync + 'static) -> Self {
        self.avail = Some(Box::new(f));
        self
    }
    /// Befehl ohne Wirkung (Platzhalter mit Beschreibung).
    pub fn not_implemented(mut self) -> FnCommand {
        self.info.implemented = false;
        let n = self.info.name.clone();
        self.run(move |_, _| Err(EngineError::NotImplemented(n.clone())))
    }

    /// Ausführung festlegen und Befehl bauen.
    pub fn run(self, f: impl Fn(&mut Session, &Params) -> Result<CommandOutput> + Send + Sync + 'static) -> FnCommand {
        FnCommand { info: self.info, apply: Box::new(f), checked: self.checked, avail: self.avail }
    }
}

struct Placeholder {
    info: CommandInfo,
}

impl Command for Placeholder {
    fn info(&self) -> &CommandInfo {
        &self.info
    }
    fn apply(&self, _s: &mut Session, _p: &Params) -> Result<CommandOutput> {
        Err(EngineError::NotImplemented(self.info.name.clone()))
    }
}

fn merge_catalog(info: &mut CommandInfo, e: &CatalogEntry) {
    if info.label.is_empty() {
        info.label = e.label_de.clone();
    }
    if info.menu_label.is_empty() {
        info.menu_label = e.menu_label_de.clone();
    }
    if info.menu_path.is_empty() {
        info.menu_path = e.path_de.clone();
    }
    if info.description.is_none() {
        info.description = e.status_de.clone();
    }
    if e.fw_id.is_some() || info.fw_id.is_none() {
        info.fw_id = e.fw_id;
    }
    if e.cmd.is_some() || info.fw_cmd.is_none() {
        info.fw_cmd = e.cmd.clone();
    }
    if info.js.is_empty() {
        info.js = e.js.clone();
    }
}

/// Das Befehlsregister.
#[derive(Default)]
pub struct CommandRegistry {
    commands: Vec<Arc<dyn Command>>,
    index: BTreeMap<String, usize>,
}

impl CommandRegistry {
    /// Leeres Register.
    pub fn new() -> CommandRegistry {
        CommandRegistry::default()
    }

    /// Register mit allen eingebauten Befehlen und Platzhaltern für den Rest des Katalogs.
    pub fn with_defaults() -> CommandRegistry {
        let mut r = CommandRegistry::new();
        crate::commands::register_all(&mut r);
        r.fill_placeholders();
        r
    }

    /// Registriert einen Befehl aus dem Baukasten (Menüdaten aus dem Katalog ergänzt).
    pub fn add(&mut self, c: FnCommand) {
        let mut c = c;
        if let Some(e) = catalog::by_name(&c.info.name) {
            merge_catalog(&mut c.info, e);
        }
        if c.info.label.is_empty() {
            c.info.label = c.info.name.clone();
        }
        if c.info.menu_label.is_empty() {
            c.info.menu_label = c.info.label.clone();
        }
        self.register(Arc::new(c));
    }

    /// Registriert einen beliebigen Befehl (ersetzt einen gleichnamigen).
    pub fn register(&mut self, c: Arc<dyn Command>) {
        let name = c.info().name.clone();
        if let Some(&i) = self.index.get(&name) {
            self.commands[i] = c;
        } else {
            self.index.insert(name, self.commands.len());
            self.commands.push(c);
        }
    }

    /// Registriert für jeden Katalogeintrag ohne Befehl einen Platzhalter („nicht umgesetzt“).
    pub fn fill_placeholders(&mut self) {
        for e in catalog::entries() {
            if self.index.contains_key(&e.name) {
                continue;
            }
            let mut info = cmd(&e.name).info;
            merge_catalog(&mut info, e);
            info.implemented = false;
            info.requires = vec![];
            if e.placeholder {
                info.note = Some("Menü-Platzhalter, zur Laufzeit ersetzt".into());
            }
            self.register(Arc::new(Placeholder { info }));
        }
    }

    /// Befehl zum Namen.
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Command>> {
        self.index.get(name).map(|&i| &self.commands[i])
    }

    /// Befehl zur Original-Befehls-ID.
    pub fn by_fw_id(&self, id: u32) -> Option<&Arc<dyn Command>> {
        self.commands.iter().find(|c| c.info().fw_id == Some(id))
    }

    /// Befehl zum `cmd_`-Namen.
    pub fn by_fw_cmd(&self, cmd: &str) -> Option<&Arc<dyn Command>> {
        self.commands.iter().find(|c| c.info().fw_cmd.as_deref() == Some(cmd))
    }

    /// Alle Befehle in Registrierungsreihenfolge.
    pub fn iter(&self) -> impl Iterator<Item = &Arc<dyn Command>> {
        self.commands.iter()
    }

    /// Zahl der Befehle.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Leer?
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Katalog als JSON (CLI `commands`, MCP `list_commands`).
    pub fn catalog_json(&self) -> Value {
        Value::Array(
            self.commands
                .iter()
                .map(|c| {
                    let i = c.info();
                    json!({
                        "name": i.name, "label": i.label, "menu": i.menu_path, "description": i.description,
                        "fwId": i.fw_id, "fwCmd": i.fw_cmd, "implemented": i.implemented, "checkable": i.checkable,
                        "params": i.params_schema(), "requires": i.requires, "note": i.note, "js": i.js,
                    })
                })
                .collect(),
        )
    }
}

/// Hauptmenü als Baum (Menütexte in Katalogreihenfolge) für die UI.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MenuNode {
    /// Menütext.
    pub label: String,
    /// Befehlsname bei Blättern.
    pub command: Option<String>,
    pub children: Vec<MenuNode>,
}

/// Baut den Hauptmenübaum aus dem Katalog.
pub fn main_menu() -> Vec<MenuNode> {
    let mut roots: Vec<MenuNode> = Vec::new();
    for e in catalog::entries().iter().filter(|e| e.main_menu) {
        let mut level = &mut roots;
        let n = e.path_de.len();
        for (i, seg) in e.path_de.iter().enumerate() {
            let pos = match level.iter().position(|m| &m.label == seg && m.command.is_none()) {
                Some(p) if i + 1 < n => p,
                _ => {
                    level.push(MenuNode { label: seg.clone(), command: if i + 1 == n { Some(e.name.clone()) } else { None }, children: Vec::new() });
                    level.len() - 1
                }
            };
            level = &mut level[pos].children;
        }
    }
    roots
}

// ------------------------------------------------------------------ Parameter

/// Parameter eines Aufrufs mit typisierten Zugriffen.
#[derive(Clone, Debug, PartialEq)]
pub struct Params {
    /// Befehlsname (für Fehlermeldungen).
    pub command: String,
    /// JSON-Objekt (oder `null`).
    pub value: Value,
}

impl Params {
    /// Parameter für einen Befehl.
    pub fn new(command: &str, value: Value) -> Params {
        Params { command: command.into(), value }
    }

    /// Parameterfehler mit Schlüssel `err-param-…` (Platzhalter `$key` = Parametername).
    fn err(&self, msg_key: &str, key: &str) -> EngineError {
        EngineError::InvalidParams { name: self.command.clone(), message: fp_i18n::tr!(msg_key, key = key) }
    }

    /// Roher Wert.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.value.get(key).filter(|v| !v.is_null())
    }

    /// Ist der Parameter gesetzt?
    pub fn has(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// Zahl (optional).
    pub fn opt_f64(&self, key: &str) -> Result<Option<f64>> {
        match self.get(key) {
            None => Ok(None),
            Some(Value::Number(n)) => n.as_f64().filter(|v| v.is_finite()).map(Some).ok_or_else(|| self.err("err-param-bad-number", key)),
            Some(Value::String(s)) => s.trim().parse::<f64>().ok().filter(|v| v.is_finite()).map(Some).ok_or_else(|| self.err("err-param-not-number", key)),
            Some(Value::Bool(b)) => Ok(Some(*b as i32 as f64)),
            Some(_) => Err(self.err("err-param-number-expected", key)),
        }
    }

    /// Zahl (Pflicht).
    pub fn f64(&self, key: &str) -> Result<f64> {
        self.opt_f64(key)?.ok_or_else(|| self.err("err-param-absent", key))
    }

    /// Zahl mit Vorgabe.
    pub fn f64_or(&self, key: &str, d: f64) -> Result<f64> {
        Ok(self.opt_f64(key)?.unwrap_or(d))
    }

    /// Ganzzahl (optional).
    pub fn opt_i64(&self, key: &str) -> Result<Option<i64>> {
        Ok(self.opt_f64(key)?.map(|v| v.round() as i64))
    }

    /// Ganzzahl mit Vorgabe.
    pub fn i64_or(&self, key: &str, d: i64) -> Result<i64> {
        Ok(self.opt_i64(key)?.unwrap_or(d))
    }

    /// Ganzzahl (Pflicht).
    pub fn i64(&self, key: &str) -> Result<i64> {
        self.opt_i64(key)?.ok_or_else(|| self.err("err-param-absent", key))
    }

    /// Index ≥ 0 (Pflicht).
    pub fn usize(&self, key: &str) -> Result<usize> {
        let v = self.i64(key)?;
        usize::try_from(v).map_err(|_| self.err("err-param-negative", key))
    }

    /// Wahrheitswert mit Vorgabe.
    pub fn bool_or(&self, key: &str, d: bool) -> Result<bool> {
        match self.get(key) {
            None => Ok(d),
            Some(Value::Bool(b)) => Ok(*b),
            Some(Value::Number(n)) => Ok(n.as_f64().unwrap_or(0.0) != 0.0),
            Some(Value::String(s)) => match s.as_str() {
                "true" | "1" => Ok(true),
                "false" | "0" => Ok(false),
                _ => Err(self.err("err-param-bool-expected", key)),
            },
            Some(_) => Err(self.err("err-param-bool-expected", key)),
        }
    }

    /// Optionaler Wahrheitswert.
    pub fn opt_bool(&self, key: &str) -> Result<Option<bool>> {
        if self.has(key) { self.bool_or(key, false).map(Some) } else { Ok(None) }
    }

    /// Text (optional).
    pub fn opt_str(&self, key: &str) -> Result<Option<String>> {
        match self.get(key) {
            None => Ok(None),
            Some(Value::String(s)) => Ok(Some(s.clone())),
            Some(Value::Number(n)) => Ok(Some(n.to_string())),
            Some(Value::Bool(b)) => Ok(Some(b.to_string())),
            Some(_) => Err(self.err("err-param-text-expected", key)),
        }
    }

    /// Text (Pflicht).
    pub fn str(&self, key: &str) -> Result<String> {
        self.opt_str(key)?.ok_or_else(|| self.err("err-param-absent", key))
    }

    /// Farbe (optional): `#rrggbb[aa]`, `rrggbb` oder ARGB-Zahl.
    pub fn opt_color(&self, key: &str) -> Result<Option<fp_color::Argb>> {
        match self.get(key) {
            None => Ok(None),
            Some(Value::String(s)) => {
                if s.eq_ignore_ascii_case("none") || s.eq_ignore_ascii_case("transparent") {
                    return Ok(Some(fp_color::Argb(0x00ff_ffff)));
                }
                fp_color::Rgba8::parse_hex(s).map(|c| Some(c.to_argb())).ok_or_else(|| self.err("err-param-hex-color-expected", key))
            }
            Some(Value::Number(n)) => n.as_u64().map(|v| Some(fp_color::Argb(v as u32))).ok_or_else(|| self.err("err-param-color-expected", key)),
            Some(_) => Err(self.err("err-param-color-expected", key)),
        }
    }

    /// Farbe (Pflicht).
    pub fn color(&self, key: &str) -> Result<fp_color::Argb> {
        self.opt_color(key)?.ok_or_else(|| self.err("err-param-absent", key))
    }

    /// Punkt (optional): `{x, y}` oder `[x, y]`.
    pub fn opt_point(&self, key: &str) -> Result<Option<fp_geom::Point>> {
        match self.get(key) {
            None => Ok(None),
            Some(v) => parse_point(v).map(Some).ok_or_else(|| self.err("err-param-point-expected", key)),
        }
    }

    /// Punkt (Pflicht).
    pub fn point(&self, key: &str) -> Result<fp_geom::Point> {
        self.opt_point(key)?.ok_or_else(|| self.err("err-param-absent", key))
    }

    /// Rechteck (optional): `{left, top, right, bottom}`, `{x, y, width, height}` oder `[l, t, r, b]`.
    pub fn opt_rect(&self, key: &str) -> Result<Option<fp_geom::Rect>> {
        match self.get(key) {
            None => Ok(None),
            Some(v) => parse_rect(v).map(Some).ok_or_else(|| self.err("err-param-rect-expected", key)),
        }
    }

    /// Rechteck (Pflicht).
    pub fn rect(&self, key: &str) -> Result<fp_geom::Rect> {
        self.opt_rect(key)?.ok_or_else(|| self.err("err-param-absent", key))
    }

    /// Element-IDs (optional).
    pub fn opt_ids(&self, key: &str) -> Result<Option<Vec<fp_model::ElementId>>> {
        match self.get(key) {
            None => Ok(None),
            Some(Value::Array(a)) => a
                .iter()
                .map(|v| v.as_u64().map(fp_model::ElementId).ok_or_else(|| self.err("err-param-ints-expected", key)))
                .collect::<Result<Vec<_>>>()
                .map(Some),
            Some(Value::Number(n)) => n.as_u64().map(|v| Some(vec![fp_model::ElementId(v)])).ok_or_else(|| self.err("err-param-id-expected", key)),
            Some(_) => Err(self.err("err-param-ids-expected", key)),
        }
    }

    /// Liste von Punkten (`[[x, y], …]` oder `[{x, y}, …]`).
    pub fn points(&self, key: &str) -> Result<Vec<fp_geom::Point>> {
        match self.get(key) {
            Some(Value::Array(a)) => a.iter().map(|v| parse_point(v).ok_or_else(|| self.err("err-param-points-expected", key))).collect(),
            None => Err(self.err("err-param-absent", key)),
            Some(_) => Err(self.err("err-param-points-expected", key)),
        }
    }

    /// Wert als serde-Typ.
    pub fn parse<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        match self.get(key) {
            None => Ok(None),
            Some(v) => match serde_json::from_value(v.clone()) {
                Ok(x) => Ok(Some(x)),
                // Dialoge schicken ganzzahlige Werte auch als `26.0`: dann als Ganzzahlen erneut versuchen
                Err(e) => {
                    let mut w = v.clone();
                    crate::commands::whole_numbers_as_ints(&mut w);
                    serde_json::from_value(w).map(Some).map_err(|_| EngineError::InvalidParams { name: self.command.clone(), message: fp_i18n::tr!("err-param-detail", key = key, detail = e.to_string()) })
                }
            },
        }
    }
}

/// Liest einen Punkt aus JSON.
pub fn parse_point(v: &Value) -> Option<fp_geom::Point> {
    match v {
        Value::Array(a) if a.len() == 2 => Some(fp_geom::Point::new(a[0].as_f64()? as f32, a[1].as_f64()? as f32)),
        Value::Object(o) => Some(fp_geom::Point::new(o.get("x")?.as_f64()? as f32, o.get("y")?.as_f64()? as f32)),
        _ => None,
    }
}

/// Liest ein Rechteck aus JSON.
pub fn parse_rect(v: &Value) -> Option<fp_geom::Rect> {
    let f = |x: &Value| x.as_f64().map(|v| v as f32);
    match v {
        Value::Array(a) if a.len() == 4 => Some(fp_geom::Rect::new(f(&a[0])?, f(&a[1])?, f(&a[2])?, f(&a[3])?)),
        Value::Object(o) => {
            if let (Some(l), Some(t), Some(r), Some(b)) = (o.get("left"), o.get("top"), o.get("right"), o.get("bottom")) {
                Some(fp_geom::Rect::new(f(l)?, f(t)?, f(r)?, f(b)?))
            } else {
                let (x, y, w, h) = (f(o.get("x")?)?, f(o.get("y")?)?, f(o.get("width")?)?, f(o.get("height")?)?);
                Some(fp_geom::Rect::new(x, y, x + w, y + h))
            }
        }
        _ => None,
    }
}

/// Prüft die Pflichtparameter und Grundtypen gegen die Beschreibung.
pub fn validate(info: &CommandInfo, p: &Params) -> Result<()> {
    if !(p.value.is_null() || p.value.is_object()) {
        return Err(EngineError::InvalidParams { name: info.name.clone(), message: fp_i18n::t("err-param-not-object") });
    }
    for spec in &info.params {
        let v = p.get(&spec.name);
        let Some(v) = v else {
            if spec.required {
                return Err(EngineError::InvalidParams { name: info.name.clone(), message: fp_i18n::tr!("err-param-missing", key = spec.name.as_str()) });
            }
            continue;
        };
        let ok = match &spec.kind {
            ParamKind::Bool => v.is_boolean() || v.is_number(),
            ParamKind::Integer { min, max } => {
                v.is_string() || v.as_f64().is_some_and(|x| min.is_none_or(|m| x >= m as f64) && max.is_none_or(|m| x <= m as f64))
            }
            ParamKind::Number { min, max } => {
                v.is_string() || v.as_f64().is_some_and(|x| min.is_none_or(|m| x >= m) && max.is_none_or(|m| x <= m))
            }
            ParamKind::Enum { values } => v.as_str().is_some_and(|s| values.iter().any(|x| x == s)),
            ParamKind::String | ParamKind::Path => v.is_string(),
            ParamKind::Color => v.is_string() || v.is_number(),
            ParamKind::Point => parse_point(v).is_some(),
            ParamKind::Rect => parse_rect(v).is_some(),
            ParamKind::Ids => v.is_array() || v.is_number(),
            ParamKind::Json => true,
        };
        if !ok {
            return Err(EngineError::InvalidParams { name: info.name.clone(), message: fp_i18n::tr!("err-param-invalid", key = spec.name.as_str()) });
        }
    }
    Ok(())
}

/// Prüft ein Objekt-Prädikat gegen ein Element.
pub fn element_is(e: &fp_model::Element, k: ObjectKind, doc: &fp_model::Document) -> bool {
    use fp_model::ElementKind as K;
    match (k, &e.kind) {
        (ObjectKind::Bitmap, K::Bitmap(_)) => true,
        (ObjectKind::Slice, K::WebObject(w)) => w.slice.is_some(),
        (ObjectKind::Hotspot, K::WebObject(w)) => w.slice.is_none(),
        (ObjectKind::WebObject, K::WebObject(_)) => true,
        (ObjectKind::RectPrimitive, K::Path(p)) => p.primitive.is_some(),
        (ObjectKind::Text, K::Text(_)) => true,
        (ObjectKind::PathAttrs, K::Path(_) | K::Text(_) | K::CompoundShape(_)) => true,
        (ObjectKind::PathAttrs, K::Group(g)) => !g.children.is_empty() && g.children.iter().all(|c| element_is(c, ObjectKind::PathAttrs, doc)),
        (ObjectKind::Path, K::Path(_)) => true,
        (ObjectKind::CompoundShape, K::CompoundShape(_)) => true,
        (ObjectKind::Group, K::Group(_)) => true,
        (ObjectKind::MaskGroup, K::Group(g)) => g.kind.is_mask(),
        (ObjectKind::Instance, K::Instance(_)) => true,
        (ObjectKind::GraphicInstance, K::Instance(i)) => instance_kind(doc, i) == Some(SymbolKind::Graphic),
        (ObjectKind::ButtonInstance, K::Instance(i)) => instance_kind(doc, i) == Some(SymbolKind::Button),
        (ObjectKind::AnimationInstance, K::Instance(i)) => instance_kind(doc, i) == Some(SymbolKind::Animation),
        _ => false,
    }
}

fn instance_kind(doc: &fp_model::Document, i: &fp_model::InstanceElem) -> Option<SymbolKind> {
    doc.symbol_by_id(i.symbol_id).map(|(_, s)| s.kind())
}

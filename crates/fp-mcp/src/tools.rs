//! Die MCP-Werkzeuge: Beschreibungen (für Agenten optimiert, mit Beispielen), JSON-Schemas und die
//! Ausführung auf einer [`Session`]. Transportunabhängig und synchron, damit dieselbe Logik in
//! Tests, im Brückenmodus und in der CLI läuft.

use crate::docjson::{self, DocJsonOptions};
use crate::render::{self, RenderRequest};
use fp_engine::tools::{Modifiers, PointerKind, ToolInput};
use fp_engine::{CommandInfo, DocId, EngineError, Session, ToolEvent, ToolKey};
use fp_geom::Point;
use fp_render::Renderer;
use serde_json::{json, Map, Value};
use std::path::PathBuf;
use std::sync::Arc;

/// Beschreibung eines Werkzeugs.
#[derive(Clone, Debug)]
pub struct ToolDef {
    pub name: &'static str,
    /// Titel in der Sprache des Aufrufs (`mcp-tool-<name>`).
    pub title: String,
    /// Beschreibung in der Sprache des Aufrufs (`mcp-tool-<name>.description`).
    pub description: String,
    pub schema: Value,
    /// Ändert nichts (nur lesen).
    pub read_only: bool,
    /// Kann Daten überschreiben (Dateien).
    pub destructive: bool,
}

/// Ein Inhaltsblock der Antwort.
#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    Text(String),
    /// PNG-Bild.
    Image(Vec<u8>),
}

/// Ergebnis eines Werkzeugaufrufs.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolOutput {
    pub blocks: Vec<Block>,
    /// Strukturiertes Ergebnis (dasselbe wie der erste Textblock, als JSON).
    pub structured: Option<Value>,
    pub is_error: bool,
}

impl ToolOutput {
    fn json(v: Value) -> ToolOutput {
        let text = serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string());
        ToolOutput { blocks: vec![Block::Text(text)], structured: Some(v), is_error: false }
    }
    fn error(msg: impl Into<String>) -> ToolOutput {
        let m = msg.into();
        ToolOutput { blocks: vec![Block::Text(m.clone())], structured: Some(json!({"error": m})), is_error: true }
    }
    /// Erster Textblock.
    pub fn text(&self) -> String {
        self.blocks.iter().find_map(|b| if let Block::Text(t) = b { Some(t.clone()) } else { None }).unwrap_or_default()
    }
}

/// Umgebung der Werkzeuge (geteilt über Aufrufe).
#[derive(Clone)]
pub struct ToolEnv {
    /// Offscreen-Renderer mit Element-Cache.
    pub renderer: Arc<Renderer>,
    /// JSF-Skripte über `run_script` erlauben.
    pub allow_scripts: bool,
    /// Sprache der Antworten und Meldungen (nur dieser Thread, die Oberfläche bleibt unverändert);
    /// Vorgabe `en` für KI-Agenten, je Aufruf überschreibbar mit dem Argument `language`.
    pub language: String,
}

impl Default for ToolEnv {
    fn default() -> Self {
        ToolEnv { renderer: Arc::new(Renderer::default()), allow_scripts: true, language: default_language() }
    }
}

/// Antwortsprache für KI-Agenten: `FIREPIXEL_MCP_LANG`, sonst Englisch (auch für die Antworten
/// der Oberfläche im Brückenmodus).
pub fn default_language() -> String {
    std::env::var("FIREPIXEL_MCP_LANG").ok().filter(|l| !l.trim().is_empty()).unwrap_or_else(|| "en".into())
}

// ------------------------------------------------------------------ Beschreibungen

/// Titel, Beschreibung und Parameterbeschreibungen aus `mcp.ftl` in der aktuellen Sprache
/// (`mcp-tool-<werkzeug>`, `.description`, `mcp-tool-<werkzeug>-<parameter>`, `doc` = `mcp-doc-arg`).
fn localize_tool(mut d: ToolDef) -> ToolDef {
    use fp_i18n::{has, t};
    let key = format!("mcp-tool-{}", d.name);
    d.title = t(&key);
    d.description = t(&format!("{key}.description"));
    if let Some(Value::Object(props)) = d.schema.get_mut("properties") {
        for (p, v) in props.iter_mut() {
            let pk = if p == "doc" { "mcp-doc-arg".to_string() } else { format!("{key}-{p}") };
            if has(&pk)
                && let Value::Object(o) = v
            {
                o.insert("description".into(), json!(t(&pk)));
            }
        }
    }
    d
}

fn obj(props: Value, required: &[&str]) -> Value {
    json!({"type": "object", "properties": props, "required": required, "additionalProperties": false})
}

/// Alle Werkzeuge in fester Reihenfolge.
pub fn tool_defs() -> Vec<ToolDef> {
    let defs = vec![
        ToolDef {
            name: "list_commands",
            title: String::new(),
            description: String::new(),
            schema: obj(
                json!({
                    "category": {"type": "string"},
                    "filter": {"type": "string"},
                    "include_unimplemented": {"type": "boolean", "default": false},
                    "available_only": {"type": "boolean", "default": false},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 1000, "default": 200}
                }),
                &[],
            ),
            read_only: true,
            destructive: false,
        },
        ToolDef {
            name: "describe_command",
            title: String::new(),
            description: String::new(),
            schema: obj(json!({"name": {"type": "string"}}), &["name"]),
            read_only: true,
            destructive: false,
        },
        ToolDef {
            name: "run_command",
            title: String::new(),
            description: String::new(),
            schema: obj(
                json!({
                    "name": {"type": "string"},
                    "params": {"type": "object", "additionalProperties": true},
                    "steps": {"type": "array", "items": {"type": "object", "properties": {"name": {"type": "string"}, "params": {"type": "object"}}, "required": ["name"]}},
                    "label": {"type": "string"},
                    "doc": {"type": "integer"}
                }),
                &[],
            ),
            read_only: false,
            destructive: false,
        },
        ToolDef {
            name: "get_document",
            title: String::new(),
            description: String::new(),
            schema: obj(
                json!({
                    "doc": {"type": "integer"},
                    "page": {"type": "integer", "minimum": 0},
                    "all_pages": {"type": "boolean", "default": false},
                    "state": {"type": "integer", "minimum": 0},
                    "depth": {"type": "integer", "minimum": 0, "maximum": 32, "default": 8},
                    "types": {"type": "array", "items": {"type": "string"}},
                    "name": {"type": "string"},
                    "selected_only": {"type": "boolean", "default": false},
                    "attributes": {"type": "boolean", "default": true},
                    "custom_data": {"type": "boolean", "default": false},
                    "max_elements": {"type": "integer", "minimum": 1, "maximum": 100000, "default": 500}
                }),
                &[],
            ),
            read_only: true,
            destructive: false,
        },
        ToolDef {
            name: "select",
            title: String::new(),
            description: String::new(),
            schema: obj(
                json!({
                    "ids": {"type": "array", "items": {"type": "integer"}},
                    "all": {"type": "boolean"},
                    "none": {"type": "boolean"},
                    "point": {},
                    "rect": {},
                    "deep": {"type": "boolean", "default": false},
                    "mode": {"type": "string", "enum": ["replace", "add", "subtract", "intersect", "toggle"], "default": "replace"},
                    "doc": {"type": "integer"}
                }),
                &[],
            ),
            read_only: false,
            destructive: false,
        },
        ToolDef {
            name: "render",
            title: String::new(),
            description: String::new(),
            schema: obj(
                json!({
                    "doc": {"type": "integer"},
                    "page": {"type": "integer", "minimum": 0},
                    "state": {"type": "integer", "minimum": 0},
                    "zoom": {"type": "number", "exclusiveMinimum": 0, "maximum": 64, "default": 1},
                    "rect": {},
                    "element": {"type": "integer"},
                    "max_pixels": {"type": "integer", "minimum": 1, "default": 4000000}
                }),
                &[],
            ),
            read_only: true,
            destructive: false,
        },
        ToolDef {
            name: "open",
            title: String::new(),
            description: String::new(),
            schema: obj(json!({"path": {"type": "string"}}), &["path"]),
            read_only: false,
            destructive: false,
        },
        ToolDef {
            name: "save",
            title: String::new(),
            description: String::new(),
            schema: obj(json!({"path": {"type": "string"}, "doc": {"type": "integer"}}), &[]),
            read_only: false,
            destructive: true,
        },
        ToolDef {
            name: "export",
            title: String::new(),
            description: String::new(),
            schema: obj(
                json!({
                    "path": {"type": "string"},
                    "format": {"type": "string"},
                    "quality": {"type": "integer", "minimum": 1, "maximum": 100},
                    "colors": {"type": "integer", "minimum": 2, "maximum": 256},
                    "page": {"type": "integer", "minimum": 0},
                    "state": {"type": "integer", "minimum": 0},
                    "zoom": {"type": "number", "exclusiveMinimum": 0, "default": 1},
                    "rect": {},
                    "element": {"type": "integer"},
                    "doc": {"type": "integer"}
                }),
                &["path"],
            ),
            read_only: false,
            destructive: true,
        },
        ToolDef {
            name: "undo",
            title: String::new(),
            description: String::new(),
            schema: obj(json!({"steps": {"type": "integer", "minimum": 1, "default": 1}, "doc": {"type": "integer"}}), &[]),
            read_only: false,
            destructive: false,
        },
        ToolDef {
            name: "redo",
            title: String::new(),
            description: String::new(),
            schema: obj(json!({"steps": {"type": "integer", "minimum": 1, "default": 1}, "doc": {"type": "integer"}}), &[]),
            read_only: false,
            destructive: false,
        },
        ToolDef {
            name: "history",
            title: String::new(),
            description: String::new(),
            schema: obj(json!({"goto": {"type": "integer", "minimum": 0}, "doc": {"type": "integer"}}), &[]),
            read_only: false,
            destructive: false,
        },
        ToolDef {
            name: "new_document",
            title: String::new(),
            description: String::new(),
            schema: obj(
                json!({
                    "width": {"type": "number", "minimum": 1, "maximum": 30000},
                    "height": {"type": "number", "minimum": 1, "maximum": 30000},
                    "background": {"type": "string"},
                    "resolution": {"type": "number", "minimum": 1}
                }),
                &["width", "height"],
            ),
            read_only: false,
            destructive: false,
        },
        ToolDef {
            name: "tool_event",
            title: String::new(),
            description: String::new(),
            schema: obj(
                json!({
                    "tool": {"type": "string"},
                    "events": {"type": "array", "items": {"type": "object", "properties": {
                        "type": {"type": "string", "enum": ["down", "move", "up", "double", "key", "text"]},
                        "x": {"type": "number"}, "y": {"type": "number"},
                        "key": {"type": "string", "enum": ["escape", "enter", "up", "down", "left", "right", "backspace", "delete", "tab"]},
                        "text": {"type": "string"},
                        "shift": {"type": "boolean"}, "alt": {"type": "boolean"}, "ctrl": {"type": "boolean"}, "space": {"type": "boolean"},
                        "pressure": {"type": "number", "minimum": 0, "maximum": 1}
                    }, "required": ["type"]}},
                    "drag": {"type": "object", "properties": {"from": {}, "to": {}, "steps": {"type": "integer", "minimum": 1, "maximum": 200, "default": 8}, "shift": {"type": "boolean"}, "alt": {"type": "boolean"}}, "required": ["from", "to"]},
                    "doc": {"type": "integer"}
                }),
                &[],
            ),
            read_only: false,
            destructive: false,
        },
        ToolDef {
            name: "documents",
            title: String::new(),
            description: String::new(),
            schema: obj(json!({"activate": {"type": "integer"}, "close": {"type": "integer"}, "discard": {"type": "boolean", "default": false}}), &[]),
            read_only: false,
            destructive: false,
        },
        ToolDef {
            name: "run_script",
            title: String::new(),
            description: String::new(),
            schema: obj(json!({"code": {"type": "string"}, "path": {"type": "string"}, "single_undo_step": {"type": "boolean", "default": false}}), &[]),
            read_only: false,
            destructive: false,
        },
        ToolDef {
            name: "screenshot",
            title: String::new(),
            description: String::new(),
            schema: obj(
                json!({
                    "rect": {"type": "array", "items": {"type": "number"}, "minItems": 4, "maxItems": 4},
                    "max_size": {"type": "integer", "minimum": 64, "maximum": 8192, "default": 1600}
                }),
                &[],
            ),
            read_only: true,
            destructive: false,
        },
        ToolDef {
            name: "ui_state",
            title: String::new(),
            description: String::new(),
            schema: obj(json!({}), &[]),
            read_only: true,
            destructive: false,
        },
        ToolDef {
            name: "ui_action",
            title: String::new(),
            description: String::new(),
            schema: obj(
                json!({
                    "run": {"type": "string"}, "params": {"type": "object"}, "zoom": {"type": "number"}, "fit": {"type": "boolean"},
                    "panel": {"type": "string"}, "tool": {"type": "string"}, "theme": {"type": "string", "enum": ["normal", "dark", "oled"]},
                    "close_dialogs": {"type": "boolean"}, "status": {"type": "string"}
                }),
                &[],
            ),
            read_only: false,
            destructive: false,
        },
        ToolDef {
            name: "ui_input",
            title: String::new(),
            description: String::new(),
            schema: obj(
                json!({
                    "events": {"type": "array", "items": {"type": "object"}},
                    "screenshot": {},
                    "screenshot_each": {"type": "boolean", "default": false},
                    "settle_frames": {"type": "integer", "minimum": 0, "maximum": 60, "default": 2},
                    "max_size": {"type": "integer", "minimum": 64, "maximum": 8192, "default": 1600}
                }),
                &["events"],
            ),
            read_only: false,
            destructive: false,
        },
        ToolDef {
            name: "ui_find",
            title: String::new(),
            description: String::new(),
            schema: obj(
                json!({
                    "text": {"type": "string"}, "role": {"type": "string"},
                    "exact": {"type": "boolean", "default": false}, "region": {"type": "array", "items": {"type": "number"}, "minItems": 4, "maxItems": 4},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 500, "default": 50}
                }),
                &[],
            ),
            read_only: true,
            destructive: false,
        },
        ToolDef {
            name: "ui_file_dialog",
            title: String::new(),
            description: String::new(),
            schema: obj(
                json!({
                    "action": {"type": "string", "enum": ["status", "set", "cancel", "choose"], "default": "status"},
                    "path": {"type": "string"},
                    "paths": {"type": "array", "items": {"type": "string"}},
                    "filter": {"type": "integer", "minimum": 0},
                    "save_as_copy": {"type": "boolean"},
                    "append_fw_png": {"type": "boolean"},
                    "open_untitled": {"type": "boolean"},
                    "open_as_animation": {"type": "boolean"}
                }),
                &[],
            ),
            read_only: false,
            destructive: false,
        },
    ];
    defs.into_iter().chain([crate::batch::tool_def()]).map(localize_tool).collect() // BATCH-9: + `batch`
}

/// Werkzeuge, die die laufende Oberfläche brauchen (Brückenmodus; asynchron im Server beantwortet).
pub const UI_TOOLS: &[&str] = &["screenshot", "ui_state", "ui_action", "ui_input", "ui_find", "ui_file_dialog"];

/// Zeitlimit für die Antwort der Oberfläche: `ui_input` bekommt zusätzlich die angeforderten
/// Wartezeiten (höchstens 5 Minuten).
pub fn ui_timeout(name: &str, args: &Value) -> std::time::Duration {
    let mut ms: u64 = 15_000;
    if name == "ui_input"
        && let Some(ev) = args.get("events").and_then(Value::as_array)
    {
        for e in ev {
            ms += arg_u64(e, "ms").unwrap_or(0).saturating_mul(if e.get("type").and_then(Value::as_str) == Some("drag") { arg_u64(e, "steps").unwrap_or(12).max(1) } else { 1 });
            ms += 250; // Frames je Schritt
        }
    }
    std::time::Duration::from_millis(ms.min(300_000))
}

/// Übersetzt den Aufruf eines Oberflächen-Werkzeugs in eine [`crate::UiRequest`].
pub fn ui_request_for(name: &str, args: &Value) -> Option<crate::UiRequest> {
    match name {
        "screenshot" => {
            let rect = args.get("rect").and_then(|r| r.as_array()).filter(|a| a.len() == 4).map(|a| {
                let f = |i: usize| a[i].as_f64().unwrap_or(0.0) as f32;
                [f(0), f(1), f(2), f(3)]
            });
            Some(crate::UiRequest::Screenshot { rect })
        }
        "ui_state" => Some(crate::UiRequest::State),
        "ui_action" => Some(crate::UiRequest::Action(if args.is_null() { json!({}) } else { args.clone() })),
        "ui_input" => Some(crate::UiRequest::Input(if args.is_null() { json!({}) } else { args.clone() })),
        "ui_find" => Some(crate::UiRequest::Find(if args.is_null() { json!({}) } else { args.clone() })),
        "ui_file_dialog" => Some(crate::UiRequest::FileDialog(if args.is_null() { json!({}) } else { args.clone() })),
        _ => None,
    }
}

/// Verkleinert RGBA8 per Flächenmittel, sodass die größere Kante höchstens `max` ist.
pub fn downscale_rgba(width: u32, height: u32, rgba: &[u8], max: u32) -> (u32, u32, Vec<u8>) {
    let big = width.max(height);
    if big <= max || max == 0 || rgba.len() != (width * height * 4) as usize {
        return (width, height, rgba.to_vec());
    }
    let s = f64::from(big) / f64::from(max);
    let nw = ((f64::from(width) / s).round() as u32).max(1);
    let nh = ((f64::from(height) / s).round() as u32).max(1);
    let mut out = vec![0u8; (nw * nh * 4) as usize];
    for y in 0..nh {
        let y0 = (f64::from(y) * s) as u32;
        let y1 = ((f64::from(y + 1) * s) as u32).clamp(y0 + 1, height);
        for x in 0..nw {
            let x0 = (f64::from(x) * s) as u32;
            let x1 = ((f64::from(x + 1) * s) as u32).clamp(x0 + 1, width);
            let mut acc = [0u64; 4];
            let mut n = 0u64;
            for yy in y0..y1 {
                for xx in x0..x1 {
                    let i = ((yy * width + xx) * 4) as usize;
                    for c in 0..4 {
                        acc[c] += u64::from(rgba[i + c]);
                    }
                    n += 1;
                }
            }
            let o = ((y * nw + x) * 4) as usize;
            for c in 0..4 {
                out[o + c] = (acc[c] / n.max(1)) as u8;
            }
        }
    }
    (nw, nh, out)
}

/// Baut aus der Antwort der Oberfläche die Werkzeugausgabe.
pub fn ui_output(name: &str, args: &Value, reply: crate::UiReply) -> ToolOutput {
    match reply {
        crate::UiReply::Error(m) => ToolOutput::error(m),
        crate::UiReply::Json(v) => ToolOutput::json(v),
        crate::UiReply::Image { width, height, rgba } => {
            let max = arg_u64(args, "max_size").unwrap_or(1600).clamp(64, 8192) as u32;
            let (w, h, px) = downscale_rgba(width, height, &rgba, max);
            match crate::image::encode_png_raw(w, h, &px) {
                Ok(png) => {
                    let info = json!({"tool": name, "width": w, "height": h, "window_pixels": [width, height]});
                    ToolOutput { blocks: vec![Block::Image(png), Block::Text(info.to_string())], structured: Some(info), is_error: false }
                }
                Err(e) => ToolOutput::error(format!("PNG: {e}")),
            }
        }
        crate::UiReply::JsonImages(v, images) => {
            let max = arg_u64(args, "max_size").unwrap_or(1600).clamp(64, 8192) as u32;
            let mut blocks = Vec::new();
            for img in &images {
                let (w, h, px) = downscale_rgba(img.width, img.height, &img.rgba, max);
                match crate::image::encode_png_raw(w, h, &px) {
                    Ok(png) => blocks.push(Block::Image(png)),
                    Err(e) => return ToolOutput::error(format!("PNG: {e}")),
                }
            }
            blocks.push(Block::Text(v.to_string()));
            ToolOutput { blocks, structured: Some(v), is_error: false }
        }
    }
}

/// Fehlerausgabe (für Transporte).
pub fn error_output(msg: impl Into<String>) -> ToolOutput {
    ToolOutput::error(msg)
}

// ------------------------------------------------------------------ Hilfen

fn arg_u64(a: &Value, k: &str) -> Option<u64> {
    match a.get(k)? {
        Value::Number(n) => n.as_u64().or_else(|| n.as_f64().filter(|v| *v >= 0.0).map(|v| v.round() as u64)),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn arg_f64(a: &Value, k: &str) -> Option<f64> {
    match a.get(k)? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn arg_bool(a: &Value, k: &str) -> bool {
    match a.get(k) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0) != 0.0,
        Some(Value::String(s)) => s == "true" || s == "1",
        _ => false,
    }
}

fn arg_str<'a>(a: &'a Value, k: &str) -> Option<&'a str> {
    a.get(k).and_then(|v| v.as_str())
}

fn engine_err(e: EngineError) -> ToolOutput {
    let hint = match &e {
        EngineError::UnknownCommand(_) => fp_i18n::t("err-mcp-hint-unknown-command"),
        EngineError::InvalidParams { .. } => fp_i18n::t("err-mcp-hint-invalid-params"),
        EngineError::Unavailable { .. } => fp_i18n::t("err-mcp-hint-unavailable"),
        EngineError::NoDocument => fp_i18n::t("err-mcp-hint-no-document"),
        _ => String::new(),
    };
    ToolOutput::error(format!("{e}{hint}"))
}

fn activate(s: &mut Session, args: &Value) -> Result<(), ToolOutput> {
    if let Some(id) = arg_u64(args, "doc") {
        s.activate(DocId(id)).map_err(|_| ToolOutput::error(fp_i18n::tr!("err-mcp-doc-not-open-hint", id = id.to_string())))?;
    }
    Ok(())
}

fn selection_summary(s: &Session) -> Value {
    let Some(d) = s.active() else { return json!([]) };
    Value::Array(
        d.selection
            .elements
            .iter()
            .map(|id| {
                let e = fp_engine::docops::element_on_page(&d.doc, d.page, *id);
                match e {
                    Some(e) => {
                        let b = fp_engine::docops::element_bounds(&d.doc, &e);
                        json!({"id": id.0, "type": docjson::element_type(&e), "name": e.common.name,
                               "bounds": b.map(|b| json!([b.left, b.top, b.right, b.bottom]))})
                    }
                    None => json!({"id": id.0}),
                }
            })
            .collect(),
    )
}

fn state_summary(s: &Session) -> Value {
    match s.active() {
        Some(d) => json!({
            "doc": d.id.0, "generation": d.generation, "dirty": d.is_dirty(),
            "undo": d.history.undo_count(), "redo": d.history.redo_count(),
            "page": d.page, "state": d.frame, "activeLayer": d.layer.map(|l| l.0),
            "selection": selection_summary(s),
        }),
        None => json!({"doc": null}),
    }
}

fn parse_point(v: &Value) -> Option<Point> {
    fp_engine::command::parse_point(v)
}

/// Kategorie eines Befehlsnamens (erstes Segment).
fn category(name: &str) -> &str {
    name.split('.').next().unwrap_or(name)
}

/// Sucht einen Befehl: Name, JS-Name, `cmd_`-Name (Groß-/Kleinschreibung egal).
pub fn resolve_command<'a>(s: &'a Session, name: &str) -> Option<&'a CommandInfo> {
    let reg = s.registry();
    if let Some(c) = reg.get(name) {
        return Some(c.info());
    }
    let lower = name.to_lowercase();
    reg.iter().map(|c| c.info()).find(|i| {
        i.name.to_lowercase() == lower
            || i.js.iter().any(|j| j.to_lowercase() == lower || j.to_lowercase().ends_with(&format!(".{lower}")))
            || i.fw_cmd.as_deref().is_some_and(|c| c.eq_ignore_ascii_case(name))
    })
}

fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        prev = cur;
    }
    prev[b.len()]
}

/// Ähnliche Befehlsnamen (für Fehlermeldungen).
pub fn suggestions(s: &Session, name: &str) -> Vec<String> {
    let lower = name.to_lowercase();
    let last = lower.rsplit('.').next().unwrap_or(&lower).to_string();
    let mut scored: Vec<(usize, String)> = s
        .registry()
        .iter()
        .map(|c| c.info())
        .filter(|i| i.implemented)
        .map(|i| {
            let n = i.name.to_lowercase();
            let d = if n.contains(&last) || i.label.to_lowercase().contains(&last) { 0 } else { edit_distance(&n, &lower).min(edit_distance(n.rsplit('.').next().unwrap_or(&n), &last) + 2) };
            (d, i.name.clone())
        })
        .collect();
    scored.sort();
    scored.into_iter().take(6).filter(|(d, _)| *d <= 6).map(|(_, n)| n).collect()
}

/// Beispielwert für einen Parameter (für describe_command).
fn example_value(p: &fp_engine::ParamSpec) -> Value {
    use fp_engine::ParamKind as K;
    if let Some(d) = &p.default {
        return d.clone();
    }
    match &p.kind {
        K::Bool => json!(true),
        K::Integer { min, .. } => json!(min.unwrap_or(0).max(0)),
        K::Number { min, max } => {
            let lo = min.unwrap_or(0.0);
            let hi = max.unwrap_or(lo + 10.0);
            json!(((lo + hi) / 2.0).min(lo.max(0.0) + 10.0))
        }
        K::String => json!("…"),
        K::Enum { values } => json!(values.first().cloned().unwrap_or_default()),
        K::Color => json!("#ff0000"),
        K::Point => json!([10, 10]),
        K::Rect => json!([10, 10, 110, 60]),
        K::Ids => json!([1]),
        K::Path => json!("C:\\path\\file.fw.png"),
        K::Json => json!({}),
    }
}

// ------------------------------------------------------------------ Ausführung

/// Führt ein Werkzeug aus.
pub fn call(s: &mut Session, env: &ToolEnv, name: &str, args: &Value) -> ToolOutput {
    let lang = arg_str(args, "language").map(str::to_string).unwrap_or_else(|| env.language.clone());
    fp_i18n::with_language(&lang, || call_inner(s, env, name, args))
}

fn call_inner(s: &mut Session, env: &ToolEnv, name: &str, args: &Value) -> ToolOutput {
    let empty = json!({});
    let args = if args.is_null() { &empty } else { args };
    match name {
        "list_commands" => list_commands(s, args),
        "describe_command" => describe_command(s, args),
        "run_command" => run_command(s, args),
        "get_document" => get_document(s, args),
        "select" => select(s, args),
        "render" => render_tool(s, env, args),
        "open" => open(s, args),
        "save" => save(s, args),
        "export" => export(s, env, args),
        "undo" | "redo" => undo_redo(s, args, name == "undo"),
        "history" => history(s, args),
        "new_document" => new_document(s, args),
        "tool_event" => tool_event(s, args),
        "documents" => documents(s, args),
        "run_script" => run_script(s, env, args),
        "batch" => crate::batch::call(s, args), // BATCH-9
        "screenshot" | "ui_state" | "ui_action" | "ui_input" | "ui_find" | "ui_file_dialog" => ToolOutput::error(crate::McpError::NoUi.to_string()),
        other => ToolOutput::error(fp_i18n::tr!("err-mcp-unknown-tool", name = other)),
    }
}

fn list_commands(s: &Session, a: &Value) -> ToolOutput {
    let cat = arg_str(a, "category").map(|c| c.to_lowercase());
    let filter = arg_str(a, "filter").map(|c| c.to_lowercase());
    let with_unimpl = arg_bool(a, "include_unimplemented");
    let avail_only = arg_bool(a, "available_only");
    let limit = arg_u64(a, "limit").unwrap_or(200).clamp(1, 1000) as usize;
    let reg = s.registry();
    if cat.is_none() && filter.is_none() {
        let mut cats: std::collections::BTreeMap<String, (usize, usize)> = Default::default();
        for c in reg.iter() {
            let i = c.info();
            let e = cats.entry(category(&i.name).to_string()).or_default();
            e.0 += 1;
            if i.implemented {
                e.1 += 1;
            }
        }
        let list: Vec<Value> = cats.into_iter().map(|(k, (n, imp))| json!({"category": k, "commands": n, "implemented": imp})).collect();
        return ToolOutput::json(json!({
            "total": reg.len(),
            "categories": list,
            "hint": fp_i18n::t("msg-mcp-categories-hint")
        }));
    }
    let mut out = Vec::new();
    let mut total = 0usize;
    for c in reg.iter() {
        let i = c.info();
        if !with_unimpl && !i.implemented {
            continue;
        }
        if let Some(cat) = &cat {
            let menu0 = i.menu_path.first().map(|m| m.to_lowercase()).unwrap_or_default();
            let local0 = command_menu_path(i).first().map(|m| m.to_lowercase()).unwrap_or_default();
            let n = i.name.to_lowercase();
            if !(n == *cat || n.starts_with(&format!("{cat}.")) || menu0 == *cat || local0 == *cat) {
                continue;
            }
        }
        if let Some(f) = &filter {
            let hay = format!(
                "{} {} {} {} {} {} {} {}",
                i.name.to_lowercase(),
                i.label.to_lowercase(),
                command_label(i).to_lowercase(),
                i.description.clone().unwrap_or_default().to_lowercase(),
                command_description(i).unwrap_or_default().to_lowercase(),
                i.js.join(" ").to_lowercase(),
                i.menu_path.join(" ").to_lowercase(),
                command_menu_path(i).join(" ").to_lowercase()
            );
            if !hay.contains(f.as_str()) {
                continue;
            }
        }
        let available = s.is_available(&i.name);
        if avail_only && !available {
            continue;
        }
        total += 1;
        if out.len() >= limit {
            continue;
        }
        let params: Vec<String> = i.params.iter().map(|p| if p.required { format!("{}*", p.name) } else { p.name.clone() }).collect();
        let mut o = json!({"name": i.name, "label": command_label(i), "params": params, "available": available});
        if !i.implemented {
            o["implemented"] = json!(false);
        }
        if !i.menu_path.is_empty() {
            o["menu"] = json!(command_menu_path(i).join(" › "));
        }
        out.push(o);
    }
    ToolOutput::json(json!({"count": total, "shown": out.len(), "commands": out, "legend": fp_i18n::t("msg-mcp-legend")}))
}

fn describe_command(s: &Session, a: &Value) -> ToolOutput {
    let Some(name) = arg_str(a, "name") else { return ToolOutput::error(fp_i18n::tr!("err-mcp-param-missing", param = "name")) };
    let Some(i) = resolve_command(s, name) else {
        return ToolOutput::error(fp_i18n::tr!("err-mcp-unknown-command", name = name, similar = suggestions(s, name).join(", ")));
    };
    let (enabled, checked) = s.command_state(&i.name);
    let reqs: Vec<Value> = i.requires.iter().map(|r| json!({"requirement": r, "met": s.requirement_met(r), "reason": r.reason()})).collect();
    let shortcuts: Vec<String> = s.command_shortcuts(&i.name).iter().map(|k| k.to_string()).collect();
    let mut ex = Map::new();
    for p in i.params.iter().filter(|p| p.required) {
        ex.insert(p.name.clone(), example_value(p));
    }
    let example = if ex.is_empty() { json!({"name": i.name}) } else { json!({"name": i.name, "params": ex}) };
    ToolOutput::json(json!({
        "name": i.name, "label": command_label(i), "menu": command_menu_path(i), "description": command_description(i),
        "implemented": i.implemented, "availableNow": enabled, "checked": checked,
        "requires": reqs, "params": i.params_schema(), "shortcuts": shortcuts,
        "js": i.js, "fwId": i.fw_id, "fwCmd": i.fw_cmd, "note": i.note.as_deref().map(fp_engine::command::source_text),
        "example": example,
    }))
}

fn run_command(s: &mut Session, a: &Value) -> ToolOutput {
    if let Err(e) = activate(s, a) {
        return e;
    }
    if let Some(Value::Array(steps)) = a.get("steps") {
        let label = arg_str(a, "label").map(str::to_string).unwrap_or_else(|| fp_i18n::t("msg-mcp-agent-steps"));
        let mut results = Vec::new();
        let mut names = Vec::new();
        for st in steps {
            let Some(n) = st.get("name").and_then(|v| v.as_str()) else { return ToolOutput::error(fp_i18n::t("err-mcp-step-needs-name")) };
            match resolve_command(s, n) {
                Some(i) => names.push((i.name.clone(), st.get("params").cloned().unwrap_or(Value::Null))),
                None => return ToolOutput::error(fp_i18n::tr!("err-mcp-unknown-command", name = n, similar = suggestions(s, n).join(", "))),
            }
        }
        let r = s.run_group(&label, |s| {
            for (n, p) in &names {
                let o = s.execute(n, p.clone()).map_err(|e| EngineError::Failed(format!("{n}: {e}")))?;
                results.push(o.value);
            }
            Ok(())
        });
        return match r {
            Ok(()) => ToolOutput::json(json!({"ok": true, "results": results, "state": state_summary(s)})),
            Err(e) => engine_err(e),
        };
    }
    let Some(name) = arg_str(a, "name") else { return ToolOutput::error(fp_i18n::t("err-mcp-name-or-steps")) };
    let Some(info) = resolve_command(s, name) else {
        return ToolOutput::error(fp_i18n::tr!("err-mcp-unknown-command", name = name, similar = suggestions(s, name).join(", ")));
    };
    let real = info.name.clone();
    let params = a.get("params").cloned().unwrap_or(Value::Null);
    match s.execute(&real, params) {
        Ok(o) => ToolOutput::json(json!({
            "ok": true, "command": real, "result": o.value, "effect": format!("{:?}", o.effect),
            "historyLabel": s.active().and_then(|d| d.history.undo_label().map(|l| l.to_string())),
            "state": state_summary(s),
        })),
        Err(e) => engine_err(e),
    }
}

fn get_document(s: &mut Session, a: &Value) -> ToolOutput {
    if let Err(e) = activate(s, a) {
        return e;
    }
    let Some(d) = s.active() else {
        return ToolOutput::json(json!({"doc": null, "documents": docjson::documents_json(s), "hint": fp_i18n::t("err-mcp-no-document-hint")}));
    };
    let opts = DocJsonOptions {
        page: arg_u64(a, "page").map(|v| v as usize),
        all_pages: arg_bool(a, "all_pages"),
        state: arg_u64(a, "state").map(|v| v as usize),
        depth: arg_u64(a, "depth").map(|v| v.min(32) as usize).unwrap_or(8),
        max_elements: arg_u64(a, "max_elements").map(|v| v.clamp(1, 100_000) as usize).unwrap_or(500),
        types: a.get("types").and_then(|v| v.as_array()).map(|v| v.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default(),
        name_contains: arg_str(a, "name").map(String::from),
        selected_only: arg_bool(a, "selected_only"),
        attributes: a.get("attributes").map(|v| v.as_bool().unwrap_or(true)).unwrap_or(true),
        custom_data: arg_bool(a, "custom_data"),
    };
    let mut v = docjson::document_json(d, &opts);
    v["documents"] = docjson::documents_json(s);
    ToolOutput::json(v)
}

fn select(s: &mut Session, a: &Value) -> ToolOutput {
    if let Err(e) = activate(s, a) {
        return e;
    }
    let mode = arg_str(a, "mode").unwrap_or("replace").to_string();
    let r = if arg_bool(a, "none") {
        s.execute("select.none", json!({}))
    } else if arg_bool(a, "all") {
        s.execute("select.all", json!({}))
    } else if let Some(ids) = a.get("ids") {
        s.execute("select.set", json!({"ids": ids, "mode": mode}))
    } else if let Some(p) = a.get("point") {
        s.execute("select.pointAt", json!({"point": p, "mode": mode, "deep": arg_bool(a, "deep")}))
    } else if let Some(r) = a.get("rect") {
        s.execute("select.inRect", json!({"rect": r, "mode": mode}))
    } else {
        return ToolOutput::error(fp_i18n::t("err-mcp-select-kind"));
    };
    match r {
        Ok(_) => ToolOutput::json(json!({"ok": true, "selection": selection_summary(s), "count": s.active().map_or(0, |d| d.selection.elements.len())})),
        Err(e) => engine_err(e),
    }
}

fn render_request(a: &Value, default_max: u64) -> Result<RenderRequest, ToolOutput> {
    let rect = match a.get("rect") {
        None | Some(Value::Null) => None,
        Some(v) => Some(fp_engine::command::parse_rect(v).ok_or_else(|| ToolOutput::error(fp_i18n::t("err-mcp-rect-expected")))?),
    };
    Ok(RenderRequest {
        page: arg_u64(a, "page").map(|v| v as usize),
        state: arg_u64(a, "state").map(|v| v as usize),
        zoom: arg_f64(a, "zoom").unwrap_or(1.0),
        rect,
        element: arg_u64(a, "element"),
        max_pixels: arg_u64(a, "max_pixels").unwrap_or(default_max).max(1),
    })
}

fn render_tool(s: &mut Session, env: &ToolEnv, a: &Value) -> ToolOutput {
    if let Err(e) = activate(s, a) {
        return e;
    }
    let req = match render_request(a, 4_000_000) {
        Ok(r) => r,
        Err(e) => return e,
    };
    let Some(d) = s.active() else { return ToolOutput::error(fp_i18n::t("err-mcp-no-document-hint")) };
    match render::render(&env.renderer, d, &req) {
        Ok(r) => {
            let meta = r.meta();
            ToolOutput { blocks: vec![Block::Image(r.png), Block::Text(meta.to_string())], structured: Some(meta), is_error: false }
        }
        Err(e) => ToolOutput::error(e),
    }
}

fn doc_brief(s: &Session) -> Value {
    match s.active() {
        Some(d) => {
            let p = d.doc.pages.get(d.page);
            json!({"doc": d.id.0, "title": d.title, "path": d.path.as_ref().map(|p| p.to_string_lossy().to_string()),
                   "pages": d.doc.pages.len(), "width": p.map(|p| p.canvas.width), "height": p.map(|p| p.canvas.height),
                   "states": p.map(|p| p.root.frame_count.max(1))})
        }
        None => Value::Null,
    }
}

fn open(s: &mut Session, a: &Value) -> ToolOutput {
    let Some(path) = arg_str(a, "path") else { return ToolOutput::error(fp_i18n::tr!("err-mcp-param-missing", param = "path")) };
    match fp_script::fileio::open_document(s, std::path::Path::new(path)) {
        Ok(_) => ToolOutput::json(json!({"ok": true, "document": doc_brief(s)})),
        Err(e) => ToolOutput::error(fp_i18n::tr!("err-mcp-open-failed", error = e.to_string(), formats = fp_script::fileio::import_extensions().join(", "))),
    }
}

fn save(s: &mut Session, a: &Value) -> ToolOutput {
    if let Err(e) = activate(s, a) {
        return e;
    }
    let r = match arg_str(a, "path") {
        Some(p) => s.execute("file.saveAs", json!({"path": p})),
        None => s.execute("file.save", json!({})),
    };
    match r {
        Ok(o) => ToolOutput::json(json!({"ok": true, "saved": o.value, "document": doc_brief(s)})),
        Err(e) => engine_err(e),
    }
}

/// Exportformat aus Parameter bzw. Endung (`fwpng`, `png` oder eine andere Bildendung).
fn export_format(a: &Value, path: &str) -> String {
    let l = path.to_lowercase();
    match arg_str(a, "format").map(|f| f.to_lowercase()) {
        Some(f) if f == "fwpng" || f == "fw.png" => "fwpng".into(),
        Some(f) => f.trim_start_matches('.').to_string(),
        None if l.ends_with(".fw.png") => "fwpng".into(),
        None => std::path::Path::new(&l).extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_else(|| "png".into()),
    }
}

/// Schreibt eine FW-PNG-Kopie, ohne Pfad und Speicherstand des Dokuments zu ändern
/// (Adapter: `fp_engine::commands::file::save_document` ist nicht öffentlich).
pub fn save_fwpng_copy(s: &Session, doc: &fp_model::Document, page: usize, path: &std::path::Path) -> Result<(), String> {
    let mut d = doc.clone();
    if d.current_page != page && page < d.pages.len() {
        d.current_page = page;
        d.invalidate_root_copy();
    }
    let opts = fp_fwpng::WriteOptions { software: "Firepixel".into(), ..fp_fwpng::WriteOptions::default() };
    let bytes = match s.renderer() {
        Some(r) => match r.flattener() {
            Some(f) => fp_fwpng::write(&d, &opts, f),
            None => fp_fwpng::write(&d, &opts, &fp_engine::render::RendererFlattener(r.as_ref())),
        },
        None => fp_fwpng::write(&d, &opts, &fp_fwpng::NoFlattener),
    }
    .map_err(|e| e.to_string())?;
    if let Some(dir) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

fn export(s: &mut Session, env: &ToolEnv, a: &Value) -> ToolOutput {
    if let Err(e) = activate(s, a) {
        return e;
    }
    let Some(path) = arg_str(a, "path").map(PathBuf::from) else { return ToolOutput::error(fp_i18n::tr!("err-mcp-param-missing", param = "path")) };
    let fmt = export_format(a, &path.to_string_lossy());
    let Some(d) = s.active() else { return ToolOutput::error(fp_i18n::t("err-mcp-no-document")) };
    if fmt == "fwpng" {
        let (doc, page) = (Arc::clone(&d.doc), d.page);
        return match save_fwpng_copy(s, &doc, page, &path) {
            Ok(()) => ToolOutput::json(json!({"ok": true, "path": path.to_string_lossy(), "format": "fwpng"})),
            Err(e) => ToolOutput::error(e),
        };
    }
    if fmt != "png" {
        // GIF, JPEG, WebP, AVIF, BMP, TIFF, QOI, ICO, APNG über fp-export (ganze Seite)
        let opts = json!({"jpegQuality": a.get("quality"), "numPaletteColors": a.get("colors")});
        let target = if path.extension().is_some_and(|e| e.to_string_lossy().eq_ignore_ascii_case(&fmt)) { path.clone() } else { path.with_extension(&fmt) };
        let (id, page, state) = (d.id, arg_u64(a, "page").map(|v| v as usize), arg_u64(a, "state").map(|v| v as usize));
        return match fp_script::fileio::export_page(s, id, page, state, &target, &opts) {
            Ok(n) => ToolOutput::json(json!({"ok": true, "path": target.to_string_lossy(), "format": fmt, "bytes": n})),
            Err(e) => ToolOutput::error(fp_i18n::tr!("err-mcp-export-failed", error = e.to_string(), formats = fp_script::fileio::export_extensions().join(", "))),
        };
    }
    let req = match render_request(a, 400_000_000) {
        Ok(r) => r,
        Err(e) => return e,
    };
    match render::render(&env.renderer, d, &req) {
        Ok(r) => {
            if let Some(dir) = path.parent().filter(|p| !p.as_os_str().is_empty())
                && let Err(e) = std::fs::create_dir_all(dir)
            {
                return ToolOutput::error(fp_i18n::tr!("err-mcp-folder", path = dir.display().to_string(), error = e.to_string()));
            }
            match std::fs::write(&path, &r.png) {
                Ok(()) => ToolOutput::json(json!({"ok": true, "path": path.to_string_lossy(), "format": "png", "image": r.meta()})),
                Err(e) => ToolOutput::error(format!("{}: {e}", path.display())),
            }
        }
        Err(e) => ToolOutput::error(e),
    }
}

fn undo_redo(s: &mut Session, a: &Value, undo: bool) -> ToolOutput {
    if let Err(e) = activate(s, a) {
        return e;
    }
    let n = arg_u64(a, "steps").unwrap_or(1).clamp(1, 10_000);
    let mut done = 0;
    for _ in 0..n {
        let r = if undo { s.undo() } else { s.redo() };
        if r.is_err() {
            break;
        }
        done += 1;
    }
    if done == 0 {
        return ToolOutput::error(fp_i18n::t(if undo { "err-mcp-nothing-to-undo" } else { "err-mcp-nothing-to-redo" }));
    }
    ToolOutput::json(json!({"ok": true, "steps": done, "state": state_summary(s)}))
}

fn history(s: &mut Session, a: &Value) -> ToolOutput {
    if let Err(e) = activate(s, a) {
        return e;
    }
    if let Some(p) = arg_u64(a, "goto")
        && let Err(e) = s.history_go_to(p as usize)
    {
        return engine_err(e);
    }
    let Some(d) = s.active() else { return ToolOutput::error(fp_i18n::t("err-mcp-no-document")) };
    let steps: Vec<Value> = d.history.steps().iter().map(|st| json!({"label": st.label, "command": st.command, "params": st.params})).collect();
    ToolOutput::json(json!({
        "position": d.history.undo_count(), "redoAvailable": d.history.redo_count(), "maxUndos": d.history.max_undos(),
        "dropped": d.history.dropped(), "steps": steps,
        "hint": fp_i18n::t("msg-mcp-history-hint")
    }))
}

fn new_document(s: &mut Session, a: &Value) -> ToolOutput {
    let mut p = Map::new();
    for k in ["width", "height", "background", "resolution"] {
        if let Some(v) = a.get(k) {
            p.insert(k.into(), v.clone());
        }
    }
    match s.execute("file.new", Value::Object(p)) {
        Ok(_) => ToolOutput::json(json!({"ok": true, "document": doc_brief(s)})),
        Err(e) => engine_err(e),
    }
}

fn mods_of(e: &Value) -> Modifiers {
    Modifiers { shift: arg_bool(e, "shift"), alt: arg_bool(e, "alt"), ctrl: arg_bool(e, "ctrl"), space: arg_bool(e, "space"), caps: false }
}

fn parse_event(e: &Value) -> Result<ToolEvent, String> {
    let t = arg_str(e, "type").ok_or_else(|| fp_i18n::tr!("err-mcp-param-missing", param = "type"))?;
    let pos = || -> Result<Point, String> {
        let x = arg_f64(e, "x").ok_or_else(|| fp_i18n::tr!("err-mcp-param-missing", param = "x"))?;
        let y = arg_f64(e, "y").ok_or_else(|| fp_i18n::tr!("err-mcp-param-missing", param = "y"))?;
        Ok(Point::new(x as f32, y as f32))
    };
    let input = match t {
        "down" => ToolInput::Pointer { kind: PointerKind::Down, pos: pos()? },
        "move" | "drag" => ToolInput::Pointer { kind: PointerKind::Move, pos: pos()? },
        "up" => ToolInput::Pointer { kind: PointerKind::Up, pos: pos()? },
        "double" | "doubleclick" => ToolInput::Pointer { kind: PointerKind::DoubleClick, pos: pos()? },
        "text" => ToolInput::Text(arg_str(e, "text").unwrap_or("").to_string()),
        "key" => {
            let k = match arg_str(e, "key").unwrap_or("").to_lowercase().as_str() {
                "escape" | "esc" => ToolKey::Escape,
                "enter" | "return" => ToolKey::Enter,
                "up" => ToolKey::Up,
                "down" => ToolKey::Down,
                "left" => ToolKey::Left,
                "right" => ToolKey::Right,
                "backspace" => ToolKey::Backspace,
                "delete" | "del" => ToolKey::Delete,
                "tab" => ToolKey::Tab,
                other => return Err(fp_i18n::tr!("err-mcp-unknown-key", key = other)),
            };
            ToolInput::Key(k)
        }
        other => return Err(fp_i18n::tr!("err-mcp-unknown-event", kind = other)),
    };
    let pressure = arg_f64(e, "pressure").unwrap_or(1.0).clamp(0.0, 1.0) as f32;
    Ok(ToolEvent { input, mods: mods_of(e), pressure, time_ms: 0 })
}

fn tool_event(s: &mut Session, a: &Value) -> ToolOutput {
    if let Err(e) = activate(s, a) {
        return e;
    }
    if let Some(t) = arg_str(a, "tool") {
        let Some(id) = fp_engine::tools::by_name(t).or_else(|| fp_engine::tools::by_name(&format!("tool.{t}"))) else {
            let names: Vec<&str> = fp_engine::tools::TOOLS.iter().filter(|t| t.implemented).map(|t| t.name).collect();
            return ToolOutput::error(fp_i18n::tr!("err-mcp-unknown-canvas-tool", name = t, names = names.join(", ")));
        };
        if let Err(e) = s.set_tool(id) {
            return engine_err(e);
        }
    }
    let mut events = Vec::new();
    if let Some(d) = a.get("drag") {
        let (Some(from), Some(to)) = (d.get("from").and_then(parse_point), d.get("to").and_then(parse_point)) else {
            return ToolOutput::error(fp_i18n::t("err-mcp-drag-from-to"));
        };
        let steps = arg_u64(d, "steps").unwrap_or(8).clamp(1, 200) as usize;
        let mods = mods_of(d);
        let ev = |kind, p: Point| ToolEvent { input: ToolInput::Pointer { kind, pos: p }, mods, pressure: 1.0, time_ms: 0 };
        events.push(ev(PointerKind::Down, from));
        for i in 1..=steps {
            let t = i as f32 / steps as f32;
            events.push(ev(PointerKind::Move, Point::new(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t)));
        }
        events.push(ev(PointerKind::Up, to));
    }
    if let Some(Value::Array(list)) = a.get("events") {
        for e in list {
            match parse_event(e) {
                Ok(ev) => events.push(ev),
                Err(m) => return ToolOutput::error(m),
            }
        }
    }
    let mut executed = Vec::new();
    let mut last_overlay = Vec::new();
    let mut cursor = String::new();
    for (i, ev) in events.iter().enumerate() {
        let mut ev = ev.clone();
        ev.time_ms = i as u64 * 16;
        match s.tool_event(&ev) {
            Ok(r) => {
                for (c, v) in r.commands.iter().zip(r.results.iter()) {
                    executed.push(json!({"command": c.command, "params": c.params, "result": v}));
                }
                last_overlay = r.overlay.iter().map(|o| serde_json::to_value(o).unwrap_or(Value::Null)).collect();
                cursor = format!("{:?}", r.cursor);
            }
            Err(e) => return ToolOutput::error(fp_i18n::tr!("err-mcp-event", index = i.to_string(), error = e.to_string())),
        }
    }
    ToolOutput::json(json!({
        "ok": true, "tool": fp_engine::tools::info(s.tools.active).map(|i| i.name), "events": events.len(),
        "executed": executed, "overlay": last_overlay, "cursor": cursor, "state": state_summary(s),
        "hint": if executed.is_empty() { fp_i18n::t("msg-mcp-tool-nothing-yet") } else { String::new() }
    }))
}

fn documents(s: &mut Session, a: &Value) -> ToolOutput {
    if let Some(id) = arg_u64(a, "activate")
        && s.activate(DocId(id)).is_err()
    {
        return ToolOutput::error(fp_i18n::tr!("err-mcp-doc-not-open", id = id.to_string()));
    }
    if let Some(id) = arg_u64(a, "close") {
        let Some(d) = s.document(DocId(id)) else { return ToolOutput::error(fp_i18n::tr!("err-mcp-doc-not-open", id = id.to_string())) };
        if d.is_dirty() && !arg_bool(a, "discard") {
            return ToolOutput::error(fp_i18n::tr!("err-mcp-unsaved", title = d.title.clone()));
        }
        if let Err(e) = s.close_document(DocId(id)) {
            return engine_err(e);
        }
    }
    ToolOutput::json(json!({"documents": docjson::documents_json(s), "active": s.active_id().map(|d| d.0)}))
}

fn run_script(s: &mut Session, env: &ToolEnv, a: &Value) -> ToolOutput {
    if !env.allow_scripts {
        return ToolOutput::error(fp_i18n::t("err-mcp-scripts-off"));
    }
    let opts = fp_script::ScriptOptions { single_undo_step: arg_bool(a, "single_undo_step"), ..fp_script::ScriptOptions::default() };
    let outcome = match (arg_str(a, "code"), arg_str(a, "path")) {
        (Some(code), _) => fp_script::run_source(s, code, &opts),
        (None, Some(p)) => fp_script::run_file(s, std::path::Path::new(p), &opts),
        (None, None) => return ToolOutput::error(fp_i18n::t("err-mcp-code-or-path")),
    };
    let v = outcome.to_json();
    let mut out = ToolOutput::json(json!({"script": v, "state": state_summary(s)}));
    out.is_error = !outcome.ok;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(s: &mut Session, name: &str, a: Value) -> ToolOutput {
        call(s, &ToolEnv::default(), name, &a)
    }

    #[test]
    fn defs_have_object_schemas_and_unique_names() {
        let defs = tool_defs();
        let mut names: Vec<&str> = defs.iter().map(|d| d.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), defs.len());
        assert!(defs.iter().all(|d| d.schema["type"] == "object" && d.description.len() > 40));
    }

    #[test]
    fn draw_select_render_flow() {
        let mut s = Session::default();
        assert!(!run(&mut s, "new_document", json!({"width": 120, "height": 80})).is_error);
        let r = run(&mut s, "run_command", json!({"name": "draw.rectangle", "params": {"rect": [10, 10, 60, 40], "fillColor": "#ff0000"}}));
        assert!(!r.is_error, "{}", r.text());
        let id = r.structured.as_ref().and_then(|v| v["result"]["id"].as_u64());
        assert!(id.is_some());
        assert!(!run(&mut s, "select", json!({"none": true})).is_error);
        let sel = run(&mut s, "select", json!({"point": [20, 20]}));
        assert_eq!(sel.structured.as_ref().map(|v| v["count"].clone()), Some(json!(1)));
        let img = run(&mut s, "render", json!({"zoom": 0.5}));
        assert!(matches!(img.blocks.first(), Some(Block::Image(p)) if p.starts_with(&[137, 80, 78, 71])));
        let u = run(&mut s, "undo", json!({}));
        assert!(!u.is_error);
        let h = run(&mut s, "history", json!({}));
        assert!(h.text().contains("position"));
    }

    #[test]
    fn unknown_command_suggests() {
        let mut s = Session::default();
        let r = run(&mut s, "describe_command", json!({"name": "draw.rectangel"}));
        assert!(r.is_error && r.text().contains("draw.rectangle"), "{}", r.text());
        let c = run(&mut s, "list_commands", json!({}));
        assert!(c.text().contains("categories"));
        let d = run(&mut s, "list_commands", json!({"category": "draw"}));
        assert!(d.text().contains("draw.rectangle"));
    }

    #[test]
    fn tool_event_draws_rectangle() {
        let mut s = Session::default();
        run(&mut s, "new_document", json!({"width": 200, "height": 100}));
        let r = run(&mut s, "tool_event", json!({"tool": "rectangle", "drag": {"from": [10, 10], "to": [110, 60]}}));
        assert!(!r.is_error, "{}", r.text());
        let n = s.active().map(|d| d.history.undo_count()).unwrap_or(0);
        assert_eq!(n, 1);
        let steps = run(&mut s, "run_command", json!({"steps": [{"name": "select.all"}, {"name": "modify.transform.flipHorizontal"}]}));
        assert!(!steps.is_error, "{}", steps.text());
    }
}

/// Beschriftung eines Befehls in der Sprache der Antwort: `cmd-….label` (eindeutige Beschriftung,
/// z. B. „State Properties“ statt des Bedienfeldmenütexts „Properties...“), sonst der Menütext
/// `cmd-…` ohne Zugriffstaste und „...“; sonst der Text aus dem Register.
pub fn command_label(i: &fp_engine::CommandInfo) -> String {
    let k = fp_i18n::command_key(&i.name);
    let label = format!("{k}.label");
    if fp_i18n::has(&label) {
        strip_amp(&fp_i18n::t(&label))
    } else if fp_i18n::has(&k) {
        strip_amp(&fp_i18n::t(&k))
    } else {
        i.label.clone()
    }
}

/// Menüpfad eines Befehls in der Sprache der Antwort: je Stufe `menu-<präfix des Befehlsnamens>`
/// (`modify.align.left` → `menu-modify`, `menu-modify-align`), wenn dessen deutscher Text der
/// Stufe entspricht; sonst bleibt die Stufe wie im Register.
pub fn command_menu_path(i: &fp_engine::CommandInfo) -> Vec<String> {
    let segs: Vec<&str> = i.name.split('.').collect();
    let source = fp_i18n::current_language() == fp_i18n::SOURCE_LANG;
    i.menu_path
        .iter()
        .enumerate()
        .map(|(k, de)| {
            let plain = |t: String| strip_amp(&t).trim_end_matches("...").trim_end_matches('…').to_string();
            let de_plain = plain(de.clone());
            if k < segs.len() {
                let key = format!("menu-{}", segs[..=k].join("-"));
                if fp_i18n::has(&key) && fp_i18n::with_language(fp_i18n::SOURCE_LANG, || plain(fp_i18n::t(&key))) == de_plain {
                    return if source { de.clone() } else { strip_amp(&fp_i18n::t(&key)) };
                }
            }
            if k + 1 == i.menu_path.len() && fp_i18n::with_language(fp_i18n::SOURCE_LANG, || plain(command_label(i))) == de_plain {
                return if source { de.clone() } else { command_label(i) };
            }
            // sonst: Menüschlüssel desselben Hauptmenüs mit diesem deutschen Text (z. B. „Scannen“)
            let prefix = format!("menu-{}", segs[0]);
            let found = fp_i18n::keys(fp_i18n::SOURCE_LANG)
                .into_iter()
                .filter(|k| k.starts_with(&prefix) && !k.contains('.'))
                .find(|k| fp_i18n::with_language(fp_i18n::SOURCE_LANG, || plain(fp_i18n::t(k))) == de_plain);
            match found {
                Some(k) if !source => strip_amp(&fp_i18n::t(&k)),
                _ => de.clone(),
            }
        })
        .collect()
}

/// Beschreibung eines Befehls in der Sprache der Antwort (`cmd-….description`, sonst `.tooltip`).
pub fn command_description(i: &fp_engine::CommandInfo) -> Option<String> {
    let k = fp_i18n::command_key(&i.name);
    for attr in ["description", "tooltip"] {
        let key = format!("{k}.{attr}");
        if fp_i18n::has(&key) {
            return Some(fp_i18n::t(&key));
        }
    }
    i.description.clone()
}

/// `&Datei` → `Datei`, `&&` → `&`.
fn strip_amp(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '&' {
            if it.peek() == Some(&'&') {
                out.push('&');
                it.next();
            }
            continue;
        }
        out.push(c);
    }
    out.trim_end_matches("...").trim_end_matches('…').trim().to_string()
}

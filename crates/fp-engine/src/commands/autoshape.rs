//! Automatische Formen (Werkzeug „Automatische Form“, Bedienfeld „Automatische Formen“).
//!
//! Die Formen sind Skripte; ihre Laufzeit liefert fp-script über
//! [`crate::session::ScriptHost::auto_shape`]. Die Befehle hier reichen nur weiter, damit jede
//! Aktion im Register steht und als Verlaufsschritt landet.

use crate::command::{CommandOutput, CommandRegistry, ParamKind, cmd};
use crate::error::{EngineError, Result};
use crate::session::Session;
use serde_json::{Value, json};

fn call(s: &mut Session, value: &Value, op: &str) -> Result<Value> {
    let host = s
        .script_host()
        .ok_or(EngineError::BackendMissing("err-backend-autoshape-runtime"))?;
    let mut v = if value.is_object() {
        value.clone()
    } else {
        json!({})
    };
    v["op"] = json!(op);
    host.auto_shape(s, &v)
}

pub(super) fn register(r: &mut CommandRegistry) {
    r.add(
        cmd("autoShape.insert")
            .label("Automatische Form einfügen")
            .requires(super::DOC)
            .param(
                "shape",
                ParamKind::String,
                "Form (deutscher oder englischer Name)",
            )
            .opt("rect", ParamKind::Rect, "Rahmen")
            .opt("at", ParamKind::Point, "Mittelpunkt (statt rect)")
            .run(|s, p| Ok(CommandOutput::doc_with(call(s, &p.value, "insert")?))),
    );
    r.add(
        cmd("autoShape.dragControlPoint")
            .label("Steuerpunkt ziehen")
            .requires(super::DOC)
            .param(
                "id",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "Form",
            )
            .param(
                "index",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "Steuerpunkt",
            )
            .param("from", ParamKind::Point, "Start")
            .param("to", ParamKind::Point, "Ziel")
            .opt("points", ParamKind::Json, "Zwischenpunkte")
            .opt("shift", ParamKind::Bool, "Umschalt")
            .opt("alt", ParamKind::Bool, "Alt")
            .opt("ctrl", ParamKind::Bool, "Strg")
            .run(|s, p| {
                Ok(CommandOutput::doc_with(call(
                    s,
                    &p.value,
                    "dragControlPoint",
                )?))
            }),
    );
    r.add(
        cmd("autoShape.setParams")
            .label("Formparameter")
            .requires(super::DOC)
            .param(
                "id",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "Form",
            )
            .param("params", ParamKind::Json, "Parameter {Name: Wert}")
            .run(|s, p| Ok(CommandOutput::doc_with(call(s, &p.value, "setParams")?))),
    );
    r.add(
        cmd("autoShape.redraw")
            .label("Form neu zeichnen")
            .requires(super::DOC)
            .param(
                "id",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "Form",
            )
            .run(|s, p| Ok(CommandOutput::doc_with(call(s, &p.value, "redraw")?))),
    );
    r.add(
        cmd("autoShape.describe")
            .label("Form beschreiben")
            .requires(super::DOC)
            .param(
                "id",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "Form",
            )
            .run(|s, p| Ok(CommandOutput::value(call(s, &p.value, "describe")?))),
    );
    r.add(
        cmd("autoShape.list")
            .label("Automatische Formen")
            .run(|s, p| Ok(CommandOutput::value(call(s, &p.value, "list")?))),
    );
}

//! Umsetzung der API-Member: Abbildung auf das Befehlsregister von fp-engine und Lesezugriffe auf
//! das Dokumentmodell. Nicht umgesetzte Member liefern [`FwErr::NotImpl`].

mod doc;
mod doc2;
pub(crate) mod elem;
mod files;
mod fw;

use crate::conv::{self, Arg};
use crate::runtime;
use crate::state::{FwErr, FwResult, Handle, st, with_state};
use boa_engine::property::PropertyKey;
use boa_engine::{Context, JsResult, JsValue, js_string};
use fp_engine::DocId;
use fp_model::{Element, ElementId};
use serde_json::Value;
use std::sync::Arc;

pub(crate) use elem::wrap_element;

/// Nicht umgesetzt.
pub(crate) fn ni<T>(cls: &str, name: &str) -> FwResult<T> {
    Err(FwErr::NotImpl(format!("{cls}.{name}")))
}

/// Methodenaufruf.
pub fn call_method(
    cls: &str,
    name: &str,
    h: &Handle,
    args: &[Arg],
    this: &JsValue,
    ctx: &mut Context,
) -> FwResult<JsValue> {
    match cls {
        "Firepixel" => fw::method(name, args, ctx),
        "HistoryPalette" => fw::history_method(name, args, ctx),
        "$" => fw::dollar_method(name, args, ctx),
        "fw.dialogs" => Ok(JsValue::undefined()),
        "Find" => doc2::find_method(name, h),
        "SetCurrentPage" => doc2::page_setter_method(name, h),
        "DocumentStatic" => doc2::static_method(name, args, ctx),
        "Widget" => doc2::widget_method(name, args, ctx),
        "Files" => files::method(name, args, ctx),
        "FileReference" => files::fileref_method(name, h, args, ctx),
        "Document" => match h {
            Handle::Document(d) => doc::method(*d, name, args, ctx),
            _ => Err(FwErr::bad("Document-Methode ohne Dokument")),
        },
        _ => elem::method(cls, name, h, args, this, ctx),
    }
}

/// Property lesen.
pub fn get_prop(
    cls: &str,
    name: &str,
    h: &Handle,
    this: &JsValue,
    ctx: &mut Context,
) -> FwResult<JsValue> {
    match cls {
        "Firepixel" => fw::get(name, ctx),
        "Errors" => fw::errors_get(name),
        "Tools" => Ok(conv::s(name)),
        "System" => fw::system_get(name),
        "$" => fw::dollar_get(name),
        "(DomObject-Basis)" => Ok(conv::s("")),
        "FwArray" => Ok(JsValue::undefined()),
        "Find" => doc2::find_get(name, h),
        "SetCurrentPage" => doc2::page_setter_get(name, h),
        "Widget" => doc2::widget_get(name, ctx),
        "Document" => match h {
            Handle::Document(d) => doc::get(*d, name, ctx),
            _ => Err(FwErr::bad("kein Dokument")),
        },
        _ => elem::get(cls, name, h, this, ctx),
    }
}

/// Property schreiben (read-only-Properties haben keinen Setter und kommen hier nicht an).
pub fn set_prop(
    cls: &str,
    name: &str,
    h: &Handle,
    a: Arg,
    raw: &JsValue,
    this: &JsValue,
    ctx: &mut Context,
) -> FwResult<()> {
    match cls {
        "Firepixel" => fw::set(name, a, raw, ctx),
        "Document" => match h {
            Handle::Document(d) => doc::set(*d, name, a, raw, ctx),
            _ => Err(FwErr::bad("kein Dokument")),
        },
        _ => elem::set(cls, name, h, a, raw, this, ctx),
    }
}

// ------------------------------------------------------------------ gemeinsame Hilfen

/// Dokument-Wrapper.
pub(crate) fn wrap_doc(ctx: &mut Context, d: DocId) -> FwResult<JsValue> {
    runtime::wrap(ctx, "Document", Handle::Document(d))
}

/// Element suchen.
pub(crate) fn element(doc: DocId, id: ElementId) -> FwResult<Arc<Element>> {
    st(|s| {
        // geschlossene Dokumente: Wrapper bleiben lesbar (das Original hält das Objekt am Leben)
        let model = match s.session.document(doc) {
            Some(d) => Arc::clone(&d.doc),
            None => s
                .closed
                .get(&doc.0)
                .cloned()
                .ok_or_else(|| FwErr::bad(format!("Dokument {} ist geschlossen", doc.0)))?,
        };
        let p = model
            .find_element(id)
            .ok_or_else(|| FwErr::bad(format!("Element {} gibt es nicht mehr", id.0)))?;
        model
            .element_at(&p)
            .cloned()
            .ok_or_else(|| FwErr::bad("Element nicht gefunden"))
    })
}

/// Dokument schließen (ohne Rückfrage); das Modell bleibt für Lesezugriffe der Wrapper erhalten.
pub(crate) fn close_doc(doc: DocId) -> FwResult<()> {
    st(|s| {
        if let Some(d) = s.session.document(doc) {
            let m = Arc::clone(&d.doc);
            s.closed.insert(doc.0, m);
        }
        s.exec_on(
            Some(doc),
            "file.close",
            serde_json::json!({"discard": true}),
        )
        .map(|_| ())
    })
}

/// Befehl ausführen (im Dokument `doc`, sonst im aktiven).
pub(crate) fn exec(doc: Option<DocId>, name: &str, params: Value) -> FwResult<Value> {
    st(|s| s.exec_on(doc, name, params).map(|o| o.value))
}

/// Ändert den Dokumentzustand direkt (Adapter für Eigenschaften ohne fp-engine-Befehl) als
/// Verlaufsschritt (`Session::run_group`; innerhalb eines Skript-Schritts Teil davon). Das
/// Dokument wird dafür kurz aktiviert.
pub(crate) fn edit_doc<R>(
    doc: DocId,
    f: impl FnOnce(&mut fp_engine::DocumentState) -> R,
) -> FwResult<R> {
    st(|s| {
        let prev = s.session.active_id();
        if prev != Some(doc) {
            s.session.activate(doc).map_err(FwErr::from)?;
        }
        // als Verlaufsschritt (bzw. Teil des Skript-Schritts): run_group vergleicht vorher/nachher
        let mut out = None;
        let g = s
            .session
            .run_group("Skript", |sess| match sess.active_mut() {
                Some(d) => {
                    out = Some(f(d));
                    Ok(())
                }
                None => Err(fp_engine::EngineError::NoDocument),
            });
        let r = match (g, out) {
            (Ok(()), Some(r)) => Ok(r),
            (Err(e), _) => Err(FwErr::from(e)),
            (Ok(()), None) => Err(FwErr::bad("Dokument")),
        };
        if let Some(p) = prev
            && p != doc
        {
            let _ = s.session.activate(p);
        }
        r
    })
}

/// Mehrere Engine-Befehle im Dokument `doc` als **ein** Verlaufsschritt (bzw. Teil des
/// Skript-Schritts); das Dokument wird dafür kurz aktiviert, die Auswahl bleibt unberührt,
/// solange die Befehle `ids` bekommen.
pub(crate) fn exec_group(doc: DocId, cmds: Vec<(&'static str, Value)>) -> FwResult<()> {
    if cmds.is_empty() {
        return Ok(());
    }
    st(|s| {
        let prev = s.session.active_id();
        if prev != Some(doc) {
            s.session.activate(doc).map_err(FwErr::from)?;
        }
        let r = s
            .session
            .run_group("Skript", |sess| {
                for (n, p) in cmds {
                    sess.execute(n, p)?;
                }
                Ok(())
            })
            .map_err(FwErr::from);
        if let Some(p) = prev
            && p != doc
        {
            let _ = s.session.activate(p);
        }
        r
    })
}

/// Textobjekte unter `ids` (ohne: Auswahl) mit der Breite ihres Textrahmens.
fn text_targets(doc: DocId, ids: Option<&[ElementId]>) -> FwResult<Vec<(ElementId, f32)>> {
    st(|s| {
        let d = s
            .session
            .document(doc)
            .ok_or_else(|| FwErr::bad("Dokument"))?;
        let list: Vec<ElementId> = ids
            .map(<[ElementId]>::to_vec)
            .unwrap_or_else(|| d.selection.elements.clone());
        Ok(list
            .into_iter()
            .filter_map(|id| {
                let p = d.doc.find_element(id)?;
                match &d.doc.element_at(&p)?.kind {
                    fp_model::element::ElementKind::Text(t) => {
                        let r = t.anchor_rect.unwrap_or(t.bounds);
                        Some((id, (r.right - r.left).max(1.0)))
                    }
                    _ => None,
                }
            })
            .collect())
    })
}

/// Texteigenschaften über den Engine-Befehl `text.setProperties` setzen (ein Verlaufsschritt).
/// Ohne Textobjekte geschieht nichts (wie im Original). Bei `frameType: "fixed"` ohne Breite
/// behält jeder Text seine bisherige Rahmenbreite.
pub(crate) fn set_text_props(doc: DocId, ids: Option<&[ElementId]>, props: Value) -> FwResult<()> {
    let targets = text_targets(doc, ids)?;
    if targets.is_empty() {
        return Ok(());
    }
    let per_text = props.get("frameType").and_then(Value::as_str) == Some("fixed")
        && props.get("width").is_none();
    let cmds: Vec<(&'static str, Value)> = if per_text {
        targets
            .iter()
            .map(|(id, w)| {
                let mut p = props.clone();
                p["ids"] = serde_json::json!([id.0]);
                p["width"] = serde_json::json!(w);
                ("text.setProperties", p)
            })
            .collect()
    } else {
        let mut p = props;
        p["ids"] = serde_json::json!(targets.iter().map(|(i, _)| i.0).collect::<Vec<_>>());
        vec![("text.setProperties", p)]
    };
    exec_group(doc, cmds)
}

/// Glättungs-Code (TAAi) → Name in `text.setProperties`.
pub(crate) fn aa_name(code: i32) -> &'static str {
    match code {
        1 => "crisp",
        2 => "strong",
        3 => "system",
        4 => "custom",
        _ => "smooth",
    }
}

/// JSF-Glättungsname (`antiAliasMode`, `setTextAntiAliasing`) → Name in `text.setProperties`;
/// „none“ = harte Füllkante, Unbekanntes = glatt (BA-066).
pub(crate) fn aa_mode_from_jsf(name: &str) -> &'static str {
    match name {
        "none" => "none",
        "crisp" => "crisp",
        "strong" => "strong",
        "system" => "system",
        "custom" => "custom",
        _ => "smooth",
    }
}

/// Richtungs-Code (ORIi) → Name in `text.setProperties`.
pub(crate) fn orientation_name(code: i32) -> &'static str {
    match code {
        1 => "verticalRtl",
        2 => "horizontalRtl",
        3 => "verticalLtr",
        _ => "horizontalLtr",
    }
}

/// Richtungs-Code (ORIi) → JSF-Name, gemessen in `re\09-text.md` T5:
/// 1 = „vertical right to left“, 3 = „vertical left to right“.
pub(crate) fn orientation_jsf_name(code: i32) -> &'static str {
    match code {
        1 => "vertical right to left",
        2 => "horizontal right to left",
        3 => "vertical left to right",
        _ => "horizontal left to right",
    }
}

/// JSF-Name → Richtungs-Code (ORIi). Vertikal ohne ausdrückliches
/// „left to right“ gilt als rechts→links (1), horizontal ohne „right to left“
/// als links→rechts (0). Nicht nach einzelnen Wörtern suchen: „right to left“
/// enthält auch „left“.
pub(crate) fn orientation_code_from_jsf(name: &str) -> i32 {
    let v = name.trim().to_ascii_lowercase();
    if v.starts_with("vert") {
        if v.contains("left to right") || v.ends_with("ltr") {
            3
        } else {
            1
        }
    } else if v.contains("right to left") || v.ends_with("rtl") {
        2
    } else {
        0
    }
}

/// Ebene (Platz auf der aktuellen Seite) auf- oder zuklappen über `layer.setExpanded`.
pub(crate) fn set_layers_expanded(doc: DocId, slots: Option<usize>, open: bool) -> FwResult<()> {
    let ids: Vec<u64> = st(|s| {
        let d = s
            .session
            .document(doc)
            .ok_or_else(|| FwErr::bad("Dokument"))?;
        Ok(fp_engine::docops::own_layers(&d.doc, d.page)
            .into_iter()
            .filter(|(slot, _)| slots.is_none_or(|x| x == *slot))
            .map(|(_, l)| l.id.0)
            .collect())
    })?;
    exec_group(
        doc,
        ids.into_iter()
            .map(|id| {
                (
                    "layer.setExpanded",
                    serde_json::json!({"layer": id, "expanded": open}),
                )
            })
            .collect(),
    )
}

/// Ändert alle ausgewählten Textobjekte direkt.
pub(crate) fn edit_selected_texts(doc: DocId, f: impl Fn(&mut fp_model::TextElem)) -> FwResult<()> {
    edit_doc(doc, |d| {
        let (page, ids) = (d.page, d.selection.elements.clone());
        let m = d.doc_mut();
        for id in ids {
            fp_engine::docops::edit_by_id(m, page, id, |e| {
                if let fp_model::element::ElementKind::Text(t) = &mut e.kind {
                    f(t);
                }
            });
        }
    })
}

/// Liste von Elementen als Wrapper-Array.
pub(crate) fn wrap_elements(
    ctx: &mut Context,
    doc: DocId,
    els: &[Arc<Element>],
) -> FwResult<JsValue> {
    let mut v = Vec::with_capacity(els.len());
    for e in els {
        v.push(wrap_element(ctx, doc, e)?);
    }
    Ok(conv::array(v, ctx))
}

/// Liste von IDs als Wrapper-Array.
pub(crate) fn wrap_ids(ctx: &mut Context, doc: DocId, ids: &[ElementId]) -> FwResult<JsValue> {
    let els: Vec<Arc<Element>> = ids.iter().filter_map(|i| element(doc, *i).ok()).collect();
    wrap_elements(ctx, doc, &els)
}

/// Element-Handles aus einem Array-Argument (Wrapper).
pub(crate) fn ids_from_arg(a: &Arg) -> FwResult<Vec<(DocId, ElementId)>> {
    let list = match a.json() {
        Value::Array(v) => v,
        Value::Null => Vec::new(),
        v @ Value::Object(_) => vec![v],
        _ => {
            return Err(FwErr::Hr(
                crate::bindings::hr::E_BAD_PARAM_TYPE,
                "Liste von Elementen erwartet".into(),
            ));
        }
    };
    let idx: Vec<usize> = list
        .iter()
        .filter_map(|v| v.get("__h").and_then(|h| h.as_u64()).map(|h| h as usize))
        .collect();
    with_state(|s| {
        idx.iter()
            .filter_map(|i| match s.handles.get(*i) {
                Some(Handle::Element { doc, id }) => Some((*doc, *id)),
                _ => None,
            })
            .collect()
    })
}

/// JS-Wert eines Handles (für `{"__h": i}` in JSON-Argumenten).
pub(crate) fn handle_by_index(i: usize) -> Option<Handle> {
    with_state(|s| s.handles.get(i).cloned()).ok().flatten()
}

/// Dokument eines Arguments (Wrapper).
pub(crate) fn doc_from_arg(a: &Arg) -> Option<DocId> {
    match a.handle().and_then(handle_by_index) {
        Some(Handle::Document(d)) => Some(d),
        _ => None,
    }
}

// ------------------------------------------------------------------ globale Funktionen

fn arg_text(args: &[JsValue], ctx: &mut Context) -> String {
    args.iter()
        .map(|a| {
            a.to_string(ctx)
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn global_alert(
    _this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let t = arg_text(args, ctx);
    let _ = with_state(|s| s.alerts.push(t));
    Ok(JsValue::undefined())
}

pub(crate) fn global_confirm(
    _this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let t = arg_text(args, ctx);
    let ans = with_state(|s| {
        s.alerts.push(format!("confirm: {t}"));
        s.opts.confirm_answer
    })
    .unwrap_or(false);
    Ok(JsValue::from(ans))
}

pub(crate) fn global_prompt(
    _this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let t = arg_text(&args[..args.len().min(1)], ctx);
    let _ = with_state(|s| s.alerts.push(format!("prompt: {t}")));
    let default = match args.get(1) {
        Some(v) if !v.is_undefined() && !v.is_null() => {
            Some(v.to_string(ctx)?.to_std_string_escaped())
        }
        _ => None,
    };
    match fp_engine::script_prompt::next(&t, default.as_deref()) {
        // ohne Oberfläche: Vorgabewert (2. Argument) bzw. null
        fp_engine::script_prompt::Answer::Headless => {
            Ok(args.get(1).cloned().unwrap_or_else(JsValue::null))
        }
        fp_engine::script_prompt::Answer::Value(Some(a)) => {
            Ok(JsValue::from(boa_engine::JsString::from(a.as_str())))
        }
        fp_engine::script_prompt::Answer::Value(None) => Ok(JsValue::null()),
        // die Oberfläche fragt zuerst: Lauf abbrechen (das Dokument bleibt unverändert)
        fp_engine::script_prompt::Answer::Pending => Err(boa_engine::JsNativeError::error()
            .with_message("prompt: input required")
            .into()),
    }
}

pub(crate) fn global_write(
    _this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let t = arg_text(args, ctx);
    let _ = with_state(|s| s.log.push(t));
    Ok(JsValue::undefined())
}

pub(crate) fn global_localize(
    _this: &JsValue,
    args: &[JsValue],
    _ctx: &mut Context,
) -> JsResult<JsValue> {
    Ok(args.first().cloned().unwrap_or_default())
}

/// `fp.run(name, params)`: Befehl aus dem Befehlsregister ausführen (Format von „Schritte kopieren“
/// und „Befehl speichern“). Liefert den Rückgabewert des Befehls (z. B. `{id: 17}`).
pub(crate) fn global_fp_run(
    _this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let name = match args.first() {
        Some(v) => v.to_string(ctx)?.to_std_string_escaped(),
        None => {
            return Err(runtime::throw(
                FwErr::Hr(
                    crate::bindings::hr::E_BAD_PARAM_COUNT,
                    "fp.run(name, params)".into(),
                ),
                "fp.run",
            ));
        }
    };
    let params = match args.get(1) {
        Some(v) if !v.is_undefined() && !v.is_null() => conv::js_to_json(v, ctx, 0)?,
        _ => Value::Object(Default::default()),
    };
    let _ = with_state(|s| {
        s.used.insert(format!("fp.run({name})"));
    });
    let v = exec(None, &name, params).map_err(|e| runtime::throw(e, "fp.run"))?;
    conv::json_to_js(&v, ctx)
}

/// `fp.t(key, args)`: übersetzter Text aus den Sprachdateien in der aktuellen Sprache der Oberfläche
/// (z. B. für die mitgelieferten Befehlsskripte, MN-5). `args` ist ein Objekt mit Fluent-Variablen
/// (`{ name: "Knopf" }`); ein unbekannter Schlüssel liefert den Schlüssel selbst.
pub(crate) fn global_fp_t(
    _this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let key = match args.first() {
        Some(v) if !v.is_undefined() => v.to_string(ctx)?.to_std_string_escaped(),
        _ => {
            return Err(runtime::throw(
                FwErr::Hr(
                    crate::bindings::hr::E_BAD_PARAM_COUNT,
                    "fp.t(key, args)".into(),
                ),
                "fp.t",
            ));
        }
    };
    let vars = match args.get(1) {
        Some(v) if v.is_object() => conv::js_to_json(v, ctx, 0)?,
        _ => Value::Null,
    };
    let owned: Vec<(String, fp_i18n::Value<'static>)> = vars
        .as_object()
        .map(|m| {
            m.iter()
                .map(|(k, v)| {
                    let fv = match v {
                        Value::Number(n) => fp_i18n::Value::from(n.as_f64().unwrap_or(0.0)),
                        Value::String(s) => fp_i18n::Value::from(s.clone()),
                        other => fp_i18n::Value::from(other.to_string()),
                    };
                    (k.clone(), fv)
                })
                .collect()
        })
        .unwrap_or_default();
    let refs: Vec<(&str, fp_i18n::Value<'_>)> =
        owned.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
    let text = if refs.is_empty() {
        fp_i18n::t(&key)
    } else {
        fp_i18n::t_args(&key, &refs)
    };
    Ok(conv::s(&text))
}

/// `fp.available(name)`: ist der Befehl im aktuellen Zustand ausführbar?
pub(crate) fn global_fp_available(
    _this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let name = args
        .first()
        .map(|v| v.to_string(ctx).map(|s| s.to_std_string_escaped()))
        .transpose()?
        .unwrap_or_default();
    Ok(JsValue::from(
        st(|s| Ok(s.session.is_available(&name))).unwrap_or(false),
    ))
}

pub(crate) fn global_noop(
    _this: &JsValue,
    _args: &[JsValue],
    _ctx: &mut Context,
) -> JsResult<JsValue> {
    Ok(JsValue::undefined())
}

pub(crate) fn global_uneval(
    _this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let v = args.first().cloned().unwrap_or_default();
    Ok(conv::s(&to_source_js(&v, ctx)?))
}

/// `toSource` wie im Original. Zwei Formen (FIX-R2, Belege Paarlauf 13.10.2026):
/// - **Wirtwerte** (von Firepixel geliefert: `guides`, `gridSize`, `textRuns`, `customData`, …) wie
///   der Serialisierer des Originals: `{ x:36, y:36 }` ohne Klammern, `[ 300 ]` mit Innenabstand,
///   leer `[  ]`/`{  }`, Steuerzeichen in Zeichenketten als `\u000D` (TW-08).
/// - **Reine JS-Werte** wie SpiderMonkey 1.5: `[800, 600, , 2, 1]` (kein Innenabstand, `undefined`
///   und `null` als Lücke), Objektliterale außen mit Klammern `({a:1, b:[1, 2]})`, verschachtelt ohne,
///   Zeichenketten mit `\n`/`\r`/`\t`.
pub(crate) fn to_source(this: &JsValue, _args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    Ok(conv::s(&to_source_js(this, ctx)?))
}

/// Quelltextform eines JS-Werts (siehe [`to_source`]).
pub fn to_source_js(v: &JsValue, ctx: &mut Context) -> JsResult<String> {
    src_value(v, ctx, 0, false, true, false)
}

/// `host`: innerhalb eines Wirtwerts (vererbt sich nach innen); `top`: äußerste Ebene;
/// `in_js_array`: Element eines reinen JS-Arrays (dort wird `undefined` zur Lücke).
fn src_value(
    v: &JsValue,
    ctx: &mut Context,
    depth: u32,
    host: bool,
    top: bool,
    in_js_array: bool,
) -> JsResult<String> {
    if depth > 24 {
        return Ok("null".into());
    }
    // `undefined` und `null` sind im JS-Array des Originals beide eine Lücke (BA-073, Beleg M2 q18:
    // `[…, a.brush ? … : null].toSource()` → `[…, "fc_Solid/fn_Normal", ]`; im ganzen Paarlauf gibt das
    // Original nie `null` in einem JS-Array aus)
    if v.is_null_or_undefined() {
        return Ok(if in_js_array {
            String::new()
        } else {
            "null".into()
        });
    }
    if let Some(b) = v.as_boolean() {
        return Ok(b.to_string());
    }
    if let Some(x) = v.as_number() {
        return Ok(conv::fmt_num(x));
    }
    if let Some(st) = v.as_string() {
        let t = st.to_std_string_escaped();
        return Ok(if host { quote_host(&t) } else { quote_js(&t) });
    }
    let Some(o) = v.as_object() else {
        return Ok("null".into());
    };
    if o.is_callable() {
        return Ok("null".into());
    }
    if conv::handle_of(v, ctx).is_some() {
        let j = conv::js_to_json(v, ctx, depth)?;
        return Ok(to_source_text(&j));
    }
    let host = host || conv::is_host(v, ctx);
    let len_v = o.get(js_string!("length"), ctx)?;
    if o.is_array() || (len_v.as_number().is_some() && conv::is_fwarray(v, ctx)) {
        let len = len_v.as_number().unwrap_or(0.0).clamp(0.0, 1.0e6) as u32;
        let mut parts = Vec::with_capacity(len as usize);
        for i in 0..len {
            let e = o.get(i, ctx)?;
            parts.push(src_value(&e, ctx, depth + 1, host, false, !host)?);
        }
        return Ok(if host {
            format!("[ {} ]", parts.join(", "))
        } else {
            format!("[{}]", parts.join(", "))
        });
    }
    let keys = o.own_property_keys(ctx)?;
    let mut parts = Vec::new();
    for k in keys {
        let name = match &k {
            PropertyKey::String(s) => s.to_std_string_escaped(),
            PropertyKey::Index(i) => i.get().to_string(),
            PropertyKey::Symbol(_) => continue,
        };
        if name == conv::HANDLE_KEY || name == conv::HOST_KEY {
            continue;
        }
        let e = o.get(k, ctx)?;
        if e.is_undefined() || e.as_object().is_some_and(|x| x.is_callable()) {
            continue;
        }
        parts.push(format!(
            "{}:{}",
            source_key(&name, host),
            src_value(&e, ctx, depth + 1, host, false, false)?
        ));
    }
    Ok(if host {
        format!("{{ {} }}", parts.join(", "))
    } else if top {
        format!("({{{}}})", parts.join(", "))
    } else {
        format!("{{{}}}", parts.join(", "))
    })
}

/// Eigenschaftsname: Bezeichner (und bei JS-Objekten Indizes) ohne Anführungszeichen.
fn source_key(k: &str, host: bool) -> String {
    let ident = !k.is_empty()
        && k.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
        && !k.starts_with(|c: char| c.is_ascii_digit());
    let index = !host
        && !k.is_empty()
        && k.chars().all(|c| c.is_ascii_digit())
        && (k == "0" || !k.starts_with('0'));
    if ident || index {
        k.to_string()
    } else if host {
        quote_host(k)
    } else {
        quote_js(k)
    }
}

/// Zeichenkette im Wirt-Serialisierer des Originals: Steuerzeichen als `\u000D` (Hex in Großbuchstaben).
fn quote_host(t: &str) -> String {
    let mut out = String::with_capacity(t.len() + 2);
    out.push('"');
    for c in t.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\u{:04X}", c as u32))
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Zeichenkette wie SpiderMonkey 1.5 (`js_QuoteString`): `\b \f \n \r \t \v`, übrige
/// Steuerzeichen als `\xHH`.
fn quote_js(t: &str) -> String {
    let mut out = String::with_capacity(t.len() + 2);
    out.push('"');
    for c in t.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{b}' => out.push_str("\\v"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\x{:02X}", c as u32))
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Quelltextform eines JSON-Werts in Wirtform (für Wrapper-Handles und Rust-Aufrufer).
pub fn to_source_text(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(x) => conv::fmt_num(x.as_f64().unwrap_or(0.0)),
        Value::String(t) => quote_host(t),
        Value::Array(a) => format!(
            "[ {} ]",
            a.iter().map(to_source_text).collect::<Vec<_>>().join(", ")
        ),
        Value::Object(m) => {
            let parts: Vec<String> = m
                .iter()
                .map(|(k, v)| format!("{}:{}", source_key(k, true), to_source_text(v)))
                .collect();
            format!("{{ {} }}", parts.join(", "))
        }
    }
}

// ------------------------------------------------------------------ Proxy-Fallen (lebende Listen, customData)

fn trap_handle(args: &[JsValue], ctx: &mut Context) -> Option<Handle> {
    let target = args.first()?;
    let i = target
        .as_object()?
        .get(js_string!("__fp"), ctx)
        .ok()?
        .as_number()? as usize;
    handle_by_index(i)
}

fn key_string(v: &JsValue) -> Option<String> {
    v.as_string().map(|s| s.to_std_string_escaped())
}

fn to_js_err(e: FwErr, member: &str) -> boa_engine::JsError {
    runtime::throw(e, member)
}

pub(crate) fn proxy_get(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let key = args.get(1).cloned().unwrap_or_default();
    if let (Some(h), Some(k)) = (trap_handle(args, ctx), key_string(&key))
        && let Some(v) = elem::proxy_get(&h, &k, ctx).map_err(|e| to_js_err(e, &k))?
    {
        return Ok(v);
    }
    match args.first().and_then(|t| t.as_object()) {
        Some(t) => {
            let pk = key.to_property_key(ctx)?;
            t.get(pk, ctx)
        }
        None => Ok(JsValue::undefined()),
    }
}

pub(crate) fn proxy_set(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let key = args.get(1).cloned().unwrap_or_default();
    let value = args.get(2).cloned().unwrap_or_default();
    if let (Some(h), Some(k)) = (trap_handle(args, ctx), key_string(&key))
        && elem::proxy_set(&h, &k, &value, ctx).map_err(|e| to_js_err(e, &k))?
    {
        return Ok(JsValue::from(true));
    }
    match args.first().and_then(|t| t.as_object()) {
        Some(t) => {
            let pk = key.to_property_key(ctx)?;
            t.set(pk, value, false, ctx).map(JsValue::from)
        }
        None => Ok(JsValue::from(false)),
    }
}

pub(crate) fn proxy_has(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let key = args.get(1).cloned().unwrap_or_default();
    if let (Some(h), Some(k)) = (trap_handle(args, ctx), key_string(&key))
        && elem::proxy_keys(&h).contains(&k)
    {
        return Ok(JsValue::from(true));
    }
    match args.first().and_then(|t| t.as_object()) {
        Some(t) => {
            let pk = key.to_property_key(ctx)?;
            t.has_property(pk, ctx).map(JsValue::from)
        }
        None => Ok(JsValue::from(false)),
    }
}

pub(crate) fn proxy_own_keys(
    _this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let keys = trap_handle(args, ctx)
        .map(|h| elem::proxy_keys(&h))
        .unwrap_or_default();
    let mut v: Vec<JsValue> = keys.iter().map(|k| conv::s(k)).collect();
    // nicht konfigurierbare Ziel-Eigenschaften müssen enthalten sein (es gibt keine)
    if v.is_empty() {
        v = Vec::new();
    }
    Ok(conv::array(v, ctx))
}

pub(crate) fn proxy_get_own_property_descriptor(
    _this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let key = args.get(1).cloned().unwrap_or_default();
    if let (Some(h), Some(k)) = (trap_handle(args, ctx), key_string(&key))
        && elem::proxy_keys(&h).contains(&k)
    {
        let v = elem::proxy_get(&h, &k, ctx)
            .map_err(|e| to_js_err(e, &k))?
            .unwrap_or_default();
        let d = conv::object(
            vec![
                ("value", v),
                ("writable", JsValue::from(true)),
                ("enumerable", JsValue::from(true)),
                ("configurable", JsValue::from(true)),
            ],
            ctx,
        )?;
        return Ok(d);
    }
    match args.first().and_then(|t| t.as_object()) {
        Some(t) => {
            let pk = key.to_property_key(ctx)?;
            let has = t.has_own_property(pk.clone(), ctx)?;
            if !has {
                return Ok(JsValue::undefined());
            }
            let v = t.get(pk, ctx)?;
            conv::object(
                vec![
                    ("value", v),
                    ("writable", JsValue::from(false)),
                    ("enumerable", JsValue::from(false)),
                    ("configurable", JsValue::from(true)),
                ],
                ctx,
            )
        }
        None => Ok(JsValue::undefined()),
    }
}

/// `fw.runScript(url)`: Datei im selben Kontext ausführen.
pub(crate) fn run_script_file(path: &std::path::Path, ctx: &mut Context) -> FwResult<JsValue> {
    let bytes = std::fs::read(path).map_err(|e| {
        FwErr::Hr(
            crate::bindings::hr::E_FILE_NOT_FOUND,
            format!("{}: {e}", path.display()),
        )
    })?;
    let src = crate::decode_text(&bytes)
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    with_state(|s| s.script_stack.push(path.to_path_buf()))?;
    let r = ctx.eval(boa_engine::Source::from_bytes(&src));
    let _ = with_state(|s| s.script_stack.pop());
    match r {
        Ok(v) => Ok(v),
        Err(e) => {
            // verschachtelte Ausnahme unverändert weiterreichen (0x80041427)
            let msg = e.to_string();
            Err(FwErr::Js(msg))
        }
    }
}

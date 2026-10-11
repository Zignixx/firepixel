//! Modifizieren ▸ Pop-up-Menü ▸ Hinzufügen / Bearbeiten (Spezifikation 15 §3).
//!
//! Das Menü hängt als Verhalten (Art 9) am ausgewählten Web-Objekt (Slice oder Hotspot). Einträge
//! kommen als Liste `[{text, url, target, submenu: […]}]`, die Darstellung als Felder von
//! `fp_model::PopupMenu` (snake_case, z. B. `font`, `size`, `vertical`, `text_up_color`).
//! Ohne Parameter liefert „Bearbeiten“ das vorhandene Menü.

use super::{selected_elements, with_doc};
use crate::command::{cmd, CommandOutput, CommandRegistry, ObjectKind, ParamKind, Params, Requirement};
use crate::docops;
use crate::error::{EngineError, Result};
use fp_model::element::ElementKind;
use fp_model::web::{format_menu_items, parse_menu_items, Behavior, BehaviorAction, Behaviors, MenuItem, PopupMenu};
use serde_json::{json, Value};
use std::sync::Arc;

const WEB_SEL: &[Requirement] = &[Requirement::Document, Requirement::AnyOf(ObjectKind::WebObject)];

fn bad(p: &Params, m: impl Into<String>) -> EngineError {
    EngineError::InvalidParams { name: p.command.clone(), message: m.into() }
}

fn items_from_json(v: &Value) -> Option<Vec<MenuItem>> {
    v.as_array()?
        .iter()
        .map(|x| {
            Some(MenuItem {
                text: x.get("text")?.as_str()?.to_string(),
                url: x.get("url").and_then(Value::as_str).unwrap_or("").to_string(),
                target: x.get("target").and_then(Value::as_str).unwrap_or("").to_string(),
                submenu: match x.get("submenu") {
                    Some(s) => items_from_json(s)?,
                    None => Vec::new(),
                },
            })
        })
        .collect()
}

fn items_to_json(items: &[MenuItem]) -> Value {
    Value::Array(items.iter().map(|i| json!({"text": i.text, "url": i.url, "target": i.target, "submenu": items_to_json(&i.submenu)})).collect())
}

fn now_stamp() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}{m:02}{d:02}{:02}{:02}{:02}", rem / 3600, rem % 3600 / 60, rem % 60)
}

/// Menü aus Parametern auf `base` anwenden.
fn apply_params(base: PopupMenu, p: &Params) -> Result<PopupMenu> {
    let mut m = base;
    if let Some(v) = p.get("menu") {
        let mut cur = serde_json::to_value(&m).map_err(|e| EngineError::Failed(e.to_string()))?;
        let Value::Object(upd) = v else { return Err(bad(p, fp_i18n::t("err-param-menu"))) };
        if let Value::Object(o) = &mut cur {
            for (k, x) in upd {
                o.insert(k.clone(), x.clone());
            }
        }
        m = serde_json::from_value(cur).map_err(|e| bad(p, format!("menu: {e}")))?;
    }
    if let Some(v) = p.get("items") {
        let items = items_from_json(v).ok_or_else(|| bad(p, fp_i18n::t("err-param-items")))?;
        if items.is_empty() {
            return Err(bad(p, fp_i18n::t("err-param-items-empty")));
        }
        m.items_text = Some(format_menu_items(&items));
    }
    Ok(m)
}

fn menu_json(m: &PopupMenu) -> Value {
    let items = m.items_text.as_deref().and_then(parse_menu_items).unwrap_or_default();
    json!({"items": items_to_json(&items), "menu": serde_json::to_value(m).unwrap_or(Value::Null)})
}

pub(super) fn register(r: &mut CommandRegistry) {
    r.add(
        cmd("modify.popUpMenu.addPopUpMenu")
            .requires(WEB_SEL)
            .param("items", ParamKind::Json, "Einträge [{text, url, target, submenu}]")
            .opt("menu", ParamKind::Json, "Darstellung (Felder von PopupMenu, z. B. {\"font\": \"Arial\", \"size\": 12, \"vertical\": true})")
            .opt_default("event", ParamKind::Integer { min: Some(0), max: Some(3) }, json!(0), "Ereignis (0 onMouseOver, 1 onClick)")
            .run(|s, p| {
                let event = p.i64_or("event", 0)? as i32;
                let mut m = apply_params(PopupMenu::default(), p)?;
                m.creation_date = Some(now_stamp());
                with_doc(s, |d| {
                    let page = d.page;
                    let target = selected_elements(d).into_iter().find(|(_, e)| matches!(e.kind, ElementKind::WebObject(_))).map(|(_, e)| e.id).ok_or_else(|| EngineError::Selection(fp_i18n::t("err-sel-no-web-object")))?;
                    // eindeutige Menü-ID je Dokument
                    let mut max_id = 0;
                    d.doc.walk_elements(|_, _, e: &Arc<fp_model::Element>| {
                        if let ElementKind::WebObject(w) = &e.kind {
                            for b in w.behaviors.iter().flat_map(|b| b.items.iter()) {
                                if let Some(pm) = &b.popup {
                                    max_id = max_id.max(pm.menu_id.unwrap_or(0));
                                }
                            }
                        }
                    });
                    m.menu_id = Some(max_id + 1);
                    let out = menu_json(&m);
                    let doc = Arc::make_mut(&mut d.doc);
                    docops::edit_by_id(doc, page, target, |e| {
                        if let ElementKind::WebObject(w) = &mut e.kind {
                            let mut b = Behavior::new(BehaviorAction::PopupMenu, event);
                            b.popup = Some(Box::new(m.clone()));
                            w.behaviors.get_or_insert_with(Behaviors::default).items.push(b);
                        }
                    })
                    .ok_or(EngineError::ElementNotFound(target.0))?;
                    Ok(CommandOutput::doc_with(out).labeled(fp_i18n::t("msg-step-add-popup")))
                })
            }),
    );
    r.add(
        cmd("modify.popUpMenu.editPopUpMenu")
            .requires(WEB_SEL)
            .opt("items", ParamKind::Json, "neue Einträge")
            .opt("menu", ParamKind::Json, "geänderte Darstellungsfelder")
            .opt_default("index", ParamKind::Integer { min: Some(0), max: None }, json!(0), "welches Pop-up-Menü des Objekts")
            .run(|s, p| {
                let idx = p.i64_or("index", 0)?.max(0) as usize;
                let change = p.has("items") || p.has("menu");
                with_doc(s, |d| {
                    let page = d.page;
                    let (id, cur) = selected_elements(d)
                        .into_iter()
                        .find_map(|(_, e)| match &e.kind {
                            ElementKind::WebObject(w) => w.behaviors.as_ref().and_then(|b| b.items.iter().filter_map(|x| x.popup.as_deref()).nth(idx).cloned()).map(|m| (e.id, m)),
                            _ => None,
                        })
                        .ok_or_else(|| EngineError::Selection(fp_i18n::t("err-sel-no-popup")))?;
                    if !change {
                        return Ok(CommandOutput::value(menu_json(&cur)));
                    }
                    let m = apply_params(cur, p)?;
                    let out = menu_json(&m);
                    let doc = Arc::make_mut(&mut d.doc);
                    docops::edit_by_id(doc, page, id, |e| {
                        if let ElementKind::WebObject(w) = &mut e.kind
                            && let Some(b) = w.behaviors.as_mut()
                            && let Some(x) = b.items.iter_mut().filter(|x| x.popup.is_some()).nth(idx)
                        {
                            x.popup = Some(Box::new(m.clone()));
                        }
                    });
                    Ok(CommandOutput::doc_with(out).labeled(fp_i18n::t("msg-step-edit-popup")))
                })
            }),
    );
}

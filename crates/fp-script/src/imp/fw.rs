//! `fw` (Klasse `Firepixel`), `fw.historyPalette`, `Errors`, `System`, `$`.

use super::{doc_from_arg, exec, ids_from_arg, ni, wrap_doc, wrap_ids};
use crate::bindings::{self, hr};
use crate::conv::{self, Arg};
use crate::runtime;
use crate::state::{FwErr, FwResult, Handle, st, with_state};
use crate::urls::{path_to_url, url_to_path};
use boa_engine::{Context, JsValue};
use fp_engine::{DocId, PrefValue};
use serde_json::{Value, json};
use std::path::PathBuf;

const CLS: &str = "Firepixel";

fn script_dir() -> Option<PathBuf> {
    with_state(|s| s.script_stack.last().cloned())
        .ok()
        .flatten()
}

/// Verzeichnisse der Anwendung (Firepixel hat keine Installation wie das Original): Benutzerordner.
fn app_dir(sub: &str) -> String {
    // %APPDATA%\Firepixel, in Testläufen das Testprofil (fp_i18n::profile)
    let base =
        fp_i18n::profile::roaming_dir().unwrap_or_else(|| std::env::temp_dir().join("Firepixel"));
    let p = if sub.is_empty() { base } else { base.join(sub) };
    path_to_url(&p) + "/"
}

fn color_of(c: fp_color::Argb) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())
}

pub(super) fn get(name: &str, ctx: &mut Context) -> FwResult<JsValue> {
    Ok(match name {
        "documents" | "documentList" => {
            let ids: Vec<DocId> =
                with_state(|s| s.session.documents().iter().map(|d| d.id).collect())?;
            let mut v = Vec::new();
            for id in ids {
                v.push(wrap_doc(ctx, id)?);
            }
            conv::array(v, ctx)
        }
        "selection" => {
            let r = with_state(|s| {
                s.session
                    .active()
                    .map(|d| (d.id, d.selection.elements.clone()))
            })?;
            match r {
                Some((d, ids)) => wrap_ids(ctx, d, &ids)?,
                None => conv::array(Vec::new(), ctx),
            }
        }
        "historyPalette" => runtime::wrap(ctx, "HistoryPalette", Handle::Singleton)?,
        "dialogs" => runtime::wrap(ctx, "fw.dialogs", Handle::Singleton)?,
        "platform" => conv::s(if cfg!(windows) { "win" } else { "mac" }),
        "appName" => conv::s("Firepixel"),
        "appMacCreator" | "appMacJsfFileType" => conv::s(""),
        "ellipseBCPConst" => JsValue::from(0.552_284_749_830_793_4_f64),
        "currentScriptDir" => {
            match script_dir().and_then(|p| p.parent().map(|d| d.to_path_buf())) {
                Some(d) => conv::s(&path_to_url(&d)),
                None => JsValue::null(),
            }
        }
        "currentScriptFileName" => match script_dir()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        {
            Some(n) => conv::s(&n),
            None => JsValue::null(),
        },
        "appDir" => conv::s(&app_dir("")),
        "appJsCommandsDir" | "userJsCommandsDir" => conv::s(&app_dir("Commands")),
        "appSmartShapesDir" | "userSmartShapesDir" => conv::s(&app_dir("Auto Shapes")),
        "appSmartShapeToolsDir" | "userSmartShapeToolsDir" => conv::s(&app_dir("Auto Shape Tools")),
        "appSettingsDir" | "appPrefsDir" => conv::s(&app_dir("Settings")),
        "appTemplatesDir" | "userTemplatesDir" => conv::s(&app_dir("Templates")),
        "appBatchCodeDir"
        | "appExportSettingsDir"
        | "appFavoritesDir"
        | "appHelpDir"
        | "appHtmlCodeDir"
        | "appSwfCommandsDir"
        | "userSwfCommandsDir"
        | "appJsExtensionsDir"
        | "appPatternsDir"
        | "appPresetsDir"
        | "appStylesDir"
        | "appSymbolLibrariesDir"
        | "userSymbolLibrariesDir"
        | "appTexturesDir"
        | "appXtrasDir"
        | "commonLibraryDir" => conv::s(&app_dir(name)),
        "currentWorkspaceName" => conv::s("Firepixel"),
        "appDebugMode" | "xhtmlFormat" | "dismissBatchDialogWhenDone" => JsValue::from(false),
        "uiOK" | "errorReportingOK" => JsValue::from(true),
        "textOutputEncoding" => conv::s(&with_state(|s| s.text_encoding.clone())?),
        "activeTool" => {
            let t = with_state(|s| {
                fp_engine::tools::info(s.session.tools.active)
                    .map(|i| i.name.trim_start_matches("tool.").to_string())
            })?;
            conv::s(&t.unwrap_or_default())
        }
        "activeViewScale" => JsValue::from(with_state(|s| {
            s.session.active().map(|d| d.view.zoom).unwrap_or(1.0)
        })?),
        "screenRect" => {
            conv::rect_js([0.0, 0.0, 1920.0, 1080.0], ctx).map_err(|e| FwErr::Js(e.to_string()))?
        }
        "textInsertionIndex"
        | "textInsertionLength"
        | "progressCountCurrent"
        | "progressCountTotal"
        | "isConnectedToInternet" => JsValue::from(0),
        "batchStatusString" | "getDynamicSWFURL" => conv::s(""),
        "getStartBackgroundColor" | "getEndBackgroundColor" => {
            let c = with_state(|s| s.session.prefs.new_document_background())?;
            conv::s(&color_of(c))
        }
        "mruRecentFileList" | "mruRecentFileNames" => {
            let files: Vec<PathBuf> = with_state(|s| s.session.prefs.recent_files.clone())?;
            let v: Vec<JsValue> = files
                .iter()
                .map(|p| {
                    if name == "mruRecentFileList" {
                        conv::s(&path_to_url(p))
                    } else {
                        conv::s(
                            &p.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default(),
                        )
                    }
                })
                .collect();
            conv::array(v, ctx)
        }
        "styles" => {
            // Stile des aktiven Dokuments (Namen und Attribute wie die Style-Objekte des Originals)
            let list: Vec<Value> = with_state(|s| {
                s.session
                    .active()
                    .and_then(|d| {
                        d.doc
                            .styles
                            .as_ref()
                            .map(|v| v.iter().map(|st| super::elem::style_json(st)).collect())
                    })
                    .unwrap_or_default()
            })?;
            conv::json_to_js(&Value::Array(list), ctx).map_err(|e| FwErr::Js(e.to_string()))?
        }
        "files" => {
            let files: Vec<JsValue> = with_state(|s| {
                s.session
                    .documents()
                    .iter()
                    .filter_map(|d| d.path.as_ref().map(|p| path_to_url(p)))
                    .collect::<Vec<_>>()
            })?
            .into_iter()
            .map(|u| conv::s(&u))
            .collect();
            conv::array(files, ctx)
        }
        "selectedMask" => JsValue::null(),
        _ => return ni(CLS, name),
    })
}

pub(super) fn set(name: &str, a: Arg, raw: &JsValue, ctx: &mut Context) -> FwResult<()> {
    match name {
        "selection" => {
            let _ = raw;
            let ids = ids_from_arg(&a)?;
            let Some(doc) = ids
                .first()
                .map(|x| x.0)
                .or_else(|| with_state(|s| s.session.active_id()).ok().flatten())
            else {
                return Ok(());
            };
            let list: Vec<u64> = ids.iter().filter(|x| x.0 == doc).map(|x| x.1.0).collect();
            st(|s| {
                if s.session.active_id() != Some(doc) {
                    s.session.activate(doc).map_err(FwErr::from)?;
                }
                if list.is_empty() {
                    s.exec_on(Some(doc), "select.none", json!({}))?;
                } else {
                    s.exec_on(Some(doc), "select.set", json!({"ids": list}))?;
                }
                Ok(())
            })?;
            let _ = ctx;
            Ok(())
        }
        "textOutputEncoding" => {
            let v = a.str().unwrap_or_default();
            with_state(|s| s.text_encoding = v)?;
            Ok(())
        }
        "activeTool" => {
            let t = a.str().unwrap_or_default();
            exec(None, "tool.select", json!({"tool": t})).map(|_| ())
        }
        "activeViewScale" => {
            let z = a.f64();
            exec(None, "view.setZoom", json!({"zoom": z})).map(|_| ())
        }
        "uiOK"
        | "errorReportingOK"
        | "dismissBatchDialogWhenDone"
        | "xhtmlFormat"
        | "batchStatusString"
        | "progressCountCurrent"
        | "progressCountTotal" => Ok(()),
        _ => ni(CLS, &format!("{name} =")),
    }
}

/// Neues Dokument wie `createFirepixelDocument(size, res, bgColor)`.
fn create_document(args: &[Arg], ctx: &mut Context) -> FwResult<JsValue> {
    let (w, h) = args
        .first()
        .and_then(|a| a.point())
        .ok_or_else(|| FwErr::Hr(hr::E_BAD_PARAM_TYPE, "size {x, y}".into()))?;
    let res = args.get(1).map(|a| a.json()).unwrap_or(Value::Null);
    let ppi = res
        .get("pixelsPerUnit")
        .and_then(|v| v.as_f64())
        .unwrap_or(72.0);
    // Einheit Pixel/cm bleibt im Dokument erhalten (B-2b BA-124: `resolution` 28,35, `resolutionUnits` cm).
    // Namen wie im Original nur „inch“, „cm“, „pixels“ (re 27 K1); jeder andere (z. B. „centimeter“)
    // ist ein Skriptfehler 0x80040001, die Zahl wird nie umgerechnet. „pixels“ zählt wie „inch“.
    let per_cm = match res.get("units").and_then(|v| v.as_str()) {
        None | Some("inch") | Some("pixels") => false,
        Some("cm") => true,
        Some(other) => {
            return Err(FwErr::Hr(
                hr::E_BAD_PARAM,
                format!("createFirepixelDocument: unbekannte Einheit „{other}“ (inch, cm, pixels)"),
            ));
        }
    };
    let bg = args
        .get(2)
        .and_then(|a| a.str())
        .and_then(|c| conv::norm_color(&c))
        .unwrap_or_else(|| "#ffffff".into());
    let v = exec(
        None,
        "file.new",
        json!({"width": w.max(1.0), "height": h.max(1.0), "background": bg, "resolution": ppi.clamp(1.0, 5000.0), "perCm": per_cm}),
    )?;
    let id = v
        .get("doc")
        .and_then(|d| d.as_u64())
        .ok_or_else(|| FwErr::generic("kein Dokument angelegt"))?;
    wrap_doc(ctx, DocId(id))
}

fn doc_arg_or_active(a: Option<&Arg>) -> FwResult<DocId> {
    if let Some(d) = a.and_then(doc_from_arg) {
        return Ok(d);
    }
    with_state(|s| s.session.active_id())?
        .ok_or_else(|| FwErr::Hr(hr::E_NO_ACTIVE_DOCUMENT, "kein Dokument".into()))
}

/// Speichert `doc` unter `path` als FW-PNG (`saveAs`) und macht den Pfad zum Dokumentpfad.
pub(super) fn save_doc(doc: DocId, path: Option<PathBuf>) -> FwResult<Option<PathBuf>> {
    let r = match &path {
        Some(p) => exec(
            Some(doc),
            "file.saveAs",
            json!({"path": p.to_string_lossy()}),
        )?,
        None => exec(Some(doc), "file.save", json!({}))?,
    };
    Ok(r.get("path").and_then(|p| p.as_str()).map(PathBuf::from))
}

pub(super) fn method(name: &str, args: &[Arg], ctx: &mut Context) -> FwResult<JsValue> {
    let a0 = args.first();
    match name {
        "getDocumentDOM" => {
            let which = a0.and_then(|a| a.str());
            match which.as_deref() {
                None | Some("document") => match with_state(|s| s.session.active_id())? {
                    Some(d) => wrap_doc(ctx, d),
                    None => Ok(JsValue::null()),
                },
                Some("master") => Ok(JsValue::null()),
                Some(_) => Err(FwErr::bad(
                    "getDocumentDOM: nur \"document\" oder \"master\"",
                )),
            }
        }
        "createFirepixelDocument" => create_document(args, ctx),
        "createDocument" => {
            let id = exec(None, "file.new", json!({}))?
                .get("doc")
                .and_then(|d| d.as_u64())
                .unwrap_or(0);
            wrap_doc(ctx, DocId(id))
        }
        "openDocument" | "openMultiDocument" => {
            let url = a0
                .and_then(|a| a.str())
                .ok_or_else(|| FwErr::bad("Datei fehlt"))?;
            let path = resolve_read(&url_to_path(&url));
            if !path.exists() {
                return Err(FwErr::Hr(
                    hr::E_FILE_NOT_FOUND,
                    format!("{} gibt es nicht", path.display()),
                ));
            }
            let id = st(|s| {
                crate::fileio::open_document(&mut s.session, &path)
                    .map_err(|e| FwErr::Hr(0x8004_1416, e))
            })?;
            wrap_doc(ctx, id)
        }
        "saveDocument" | "saveDocumentAs" | "saveDocumentAsync" | "saveDocumentAsAsync" => {
            let doc = doc_arg_or_active(a0)?;
            let url = if name.contains("As") && !name.contains("Async") {
                None
            } else {
                args.get(1).and_then(|a| a.str())
            };
            let path = url.map(|u| url_to_path(&u));
            let has_path = with_state(|s| {
                s.session
                    .document(doc)
                    .map(|d| d.path.is_some())
                    .unwrap_or(false)
            })?;
            if path.is_none() && !has_path {
                // Das Original würde einen Dialog zeigen; ohne Oberfläche: Abbruch
                return Err(FwErr::Hr(
                    hr::E_USER_CANCELED,
                    "kein Dateiname (Dialog nicht verfügbar)".into(),
                ));
            }
            let saved = save_doc(doc, path)?;
            Ok(saved
                .map(|p| conv::s(&path_to_url(&p)))
                .unwrap_or_else(JsValue::null))
        }
        "saveDocumentCopyAs" | "saveDocumentAsTemplate" | "saveDocumentAsTemplateAsync" => {
            let doc = doc_arg_or_active(a0)?;
            let url = args
                .get(1)
                .and_then(|a| a.str())
                .ok_or_else(|| FwErr::Hr(hr::E_USER_CANCELED, "kein Dateiname".into()))?;
            let path = url_to_path(&url);
            super::doc::save_copy(doc, &path)?;
            Ok(conv::s(&path_to_url(&path)))
        }
        "saveAll" | "saveAllAsync" => {
            exec(None, "file.saveAll", json!({})).map(|_| JsValue::undefined())
        }
        "saveAs" => {
            let doc = doc_arg_or_active(None)?;
            let url = a0
                .and_then(|a| a.str())
                .ok_or_else(|| FwErr::Hr(hr::E_USER_CANCELED, "kein Dateiname".into()))?;
            save_doc(doc, Some(url_to_path(&url))).map(|_| JsValue::undefined())
        }
        "closeDocument" => {
            let doc = doc_arg_or_active(a0)?;
            super::close_doc(doc).map(|_| JsValue::undefined())
        }
        "revertDocument" => {
            let doc = doc_arg_or_active(a0)?;
            exec(Some(doc), "file.revert", json!({})).map(|_| JsValue::undefined())
        }
        "setActiveWindow" | "moveFocusToDoc" => {
            let doc = doc_arg_or_active(a0)?;
            st(|s| s.session.activate(doc).map_err(FwErr::from))?;
            Ok(JsValue::undefined())
        }
        "findOpenDocument" => {
            let url = a0.and_then(|a| a.str()).unwrap_or_default();
            let p = url_to_path(&url);
            let found = with_state(|s| {
                s.session
                    .documents()
                    .iter()
                    .find(|d| d.path.as_ref().is_some_and(|x| x == &p))
                    .map(|d| d.id)
            })?;
            match found {
                Some(d) => wrap_doc(ctx, d),
                None => Ok(JsValue::null()),
            }
        }
        "getDocumentPath" => {
            let doc = doc_arg_or_active(a0)?;
            let p = with_state(|s| s.session.document(doc).and_then(|d| d.path.clone()))?;
            Ok(p.map(|p| conv::s(&path_to_url(&p)))
                .unwrap_or_else(JsValue::null))
        }
        "getPref" => {
            let key = a0.and_then(|a| a.str()).unwrap_or_default();
            let v = with_state(|s| {
                s.session
                    .prefs
                    .get(&key)
                    .map(|v| v.to_json())
                    .or_else(|| s.extra_prefs.get(&key).cloned())
            })?;
            match v {
                Some(j) => conv::json_to_js(&j, ctx).map_err(|e| FwErr::Js(e.to_string())),
                None => Ok(JsValue::undefined()),
            }
        }
        "setPref" => {
            let key = a0.and_then(|a| a.str()).unwrap_or_default();
            let val = args.get(1).map(|a| a.json()).unwrap_or(Value::Null);
            with_state(|s| {
                let known = fp_engine::prefs::definition(&key).is_some()
                    || s.session.prefs.get(&key).is_some();
                match (known, PrefValue::from_json(&val)) {
                    (true, Some(pv)) => s.session.prefs.set(&key, pv),
                    _ => {
                        s.extra_prefs.insert(key, val);
                    }
                }
            })?;
            Ok(JsValue::undefined())
        }
        "undo" | "redo" => {
            st(|s| {
                if name == "undo" {
                    s.session.undo()
                } else {
                    s.session.redo()
                }
                .map_err(|e| FwErr::generic(e.to_string()))
            })?;
            Ok(JsValue::undefined())
        }
        "clipCopy" => exec(None, "edit.copy", json!({})).map(|_| JsValue::undefined()),
        "clipCut" => exec(None, "edit.cut", json!({})).map(|_| JsValue::undefined()),
        "clipPaste" => exec(None, "edit.paste", json!({})).map(|_| JsValue::undefined()),
        "deleteSelection" => exec(None, "edit.clear", json!({})).map(|_| JsValue::undefined()),
        "runScript" => {
            let url = a0
                .and_then(|a| a.str())
                .ok_or_else(|| FwErr::bad("Skript fehlt"))?;
            super::run_script_file(&resolve_read(&url_to_path(&url)), ctx)
        }
        "yesNoDialog" => {
            let t = a0.and_then(|a| a.str()).unwrap_or_default();
            let ans = with_state(|s| {
                s.alerts.push(format!("yesNoDialog: {t}"));
                s.opts.confirm_answer
            })?;
            Ok(JsValue::from(ans))
        }
        "browseForFileURL"
        | "browseForFolderURL"
        | "browseDocument"
        | "locateDocDialog"
        | "chooseScriptTargetDialog"
        | "popupColorPickerOverMouse"
        | "popupColorPicker" => Ok(JsValue::null()),
        "getRed" | "getGreen" | "getBlue" => {
            let c = a0
                .and_then(|a| a.str())
                .and_then(|c| conv::norm_color(&c))
                .unwrap_or_else(|| "#000000".into());
            let off = match name {
                "getRed" => 1,
                "getGreen" => 3,
                _ => 5,
            };
            Ok(JsValue::from(
                i32::from_str_radix(&c[off..off + 2], 16).unwrap_or(0),
            ))
        }
        "makeRGBColor" => {
            let c: Vec<i64> = args.iter().take(3).map(|a| a.i64().clamp(0, 255)).collect();
            Ok(conv::s(&format!(
                "#{:02x}{:02x}{:02x}",
                c.first().copied().unwrap_or(0),
                c.get(1).copied().unwrap_or(0),
                c.get(2).copied().unwrap_or(0)
            )))
        }
        "shiftKeyDown" | "ctrlCmdKeyDown" | "altOptKeyDown" | "isPIExpanded"
        | "getHideAllFloaters" | "associatePngwithFw" => Ok(JsValue::from(false)),
        // primary screen into a new document (not under automation: FP_NO_SHELL_OPEN)
        "takeScreenshot" => Ok(JsValue::from(
            crate::shell_open_allowed() && exec(None, "commands.screenCapture", json!({})).is_ok(),
        )),
        "isPIVisible" | "getFloaterVisibility" | "checkFwJsVersion" => {
            Ok(JsValue::from(name != "getFloaterVisibility"))
        }
        "setFloaterVisibility"
        | "toggleFloater"
        | "setHideAllFloaters"
        | "growPIWindow"
        | "shrinkPIWindow"
        | "setPIPosition"
        | "showPIWindow"
        | "hidePIWindow"
        | "reloadPatterns"
        | "reloadTextures"
        | "reloadCommonLibrary"
        | "logPipEvent"
        | "enableFlashDebugging"
        | "disableFlashDebugging"
        | "quit"
        | "quitApplication"
        | "addProjectEntry"
        | "logChangeInProject"
        | "removeAllProjectEntries" => Ok(JsValue::undefined()),
        // setActiveViewScale(scale, center): Zoom der aktiven Ansicht und Mittelpunkt (BA-068)
        "setActiveViewScale" => {
            let z = a0.map(|a| a.f64()).unwrap_or(1.0);
            if !(z.is_finite() && z > 0.0) {
                return Ok(JsValue::undefined());
            }
            let mut p = json!({"zoom": z.clamp(0.0625, 64.0)});
            if let Some((x, y)) = args.get(1).and_then(|a| a.point()) {
                p["center"] = json!([x, y]);
            }
            exec(None, "view.setZoom", p).map(|_| JsValue::undefined())
        }
        // „Warndialoge zurücksetzen“ (Befehle-Menü): abgeschaltete Warnungen wieder zeigen (FV-5)
        "resetWarnings" => {
            with_state(|s| {
                s.session.prefs.reset_warnings();
                s.session.prefs_changed();
            })?;
            Ok(JsValue::undefined())
        }
        "getProjectEntries" => Ok(conv::array(Vec::new(), ctx)),
        "getPIPosition" => conv::point_js(0.0, 0.0, ctx).map_err(|e| FwErr::Js(e.to_string())),
        "getFWLaunchLanguage" => Ok(conv::s("de_DE")),
        "getRootDirectory" => Ok(conv::s(&app_dir(""))),
        "internalNameToUIName" => Ok(a0
            .map(|a| conv::s(&a.str().unwrap_or_default()))
            .unwrap_or_default()),
        "writePanelStateToFile" | "readPanelStateFromFile" => {
            // Bedienfeldzustand in einer Datei (Schlüssel = Dateiname) – ohne Bedienfelder wirkungslos
            Ok(if name.starts_with("read") {
                JsValue::null()
            } else {
                JsValue::undefined()
            })
        }
        "exportDocumentAs" => {
            let doc = doc_arg_or_active(a0)?;
            let url = args
                .get(1)
                .and_then(|a| a.str())
                .ok_or_else(|| FwErr::bad("Ziel fehlt"))?;
            let opts = args.get(2).map(|a| a.json()).unwrap_or(Value::Null);
            super::doc::export_flat(doc, &url_to_path(&url), &opts)?;
            Ok(JsValue::from(true))
        }
        "eval" => {
            let code = a0.and_then(|a| a.str()).unwrap_or_default();
            ctx.eval(boa_engine::Source::from_bytes(&code))
                .map_err(|e| FwErr::Js(e.to_string()))
        }
        "throwInternalError" => Err(FwErr::Hr(hr::E_INTERNAL, "throwInternalError".into())),
        "createDocumentWithDialog" | "newMobileDocument" => {
            let p = if name == "newMobileDocument" {
                json!({"width": 320, "height": 480})
            } else {
                json!({})
            };
            let id = exec(None, "file.new", p)?
                .get("doc")
                .and_then(|d| d.as_u64())
                .unwrap_or(0);
            wrap_doc(ctx, DocId(id))
        }
        "newDocumentFromTemplate" => {
            let url = a0
                .and_then(|a| a.str())
                .ok_or_else(|| FwErr::bad("Vorlage fehlt"))?;
            let v = exec(
                None,
                "file.newFromTemplate",
                json!({"path": resolve_read(&url_to_path(&url)).to_string_lossy()}),
            )?;
            match v.get("doc").and_then(|d| d.as_u64()).or_else(|| {
                with_state(|s| s.session.active_id().map(|d| d.0))
                    .ok()
                    .flatten()
            }) {
                Some(id) => wrap_doc(ctx, DocId(id)),
                None => Ok(JsValue::null()),
            }
        }
        "exportPages"
        | "exportFrames"
        | "exportLayers"
        | "exportHtmlAndImages"
        | "exportCSSLayers"
        | "exportCSSSpriteForJQuery"
        | "exportPSD"
        | "exportPDF" => {
            // (doc, url[, settings]) bzw. (doc, settings, url) – URL = erstes Argument, das wie ein Pfad aussieht
            let doc = doc_arg_or_active(a0)?;
            let url = args
                .iter()
                .skip(1)
                .filter_map(|a| a.str())
                .find(|s| s.contains('/') || s.contains('\\') || s.contains(':'));
            let Some(url) = url else {
                return Err(FwErr::Hr(
                    hr::E_USER_CANCELED,
                    "kein Ziel (Dialog nicht verfügbar)".into(),
                ));
            };
            let style = match name {
                "exportPages" => "pagesToFiles",
                "exportFrames" => "framesToFiles",
                "exportLayers" => "layersToFiles",
                "exportHtmlAndImages" => "htmlAndImages",
                "exportCSSLayers" => "cssAndImages",
                "exportCSSSpriteForJQuery" => "cssSprites",
                "exportPSD" => "psd",
                _ => "pdf",
            };
            exec(
                Some(doc),
                "file.export",
                json!({"path": url_to_path(&url).to_string_lossy(), "style": style}),
            )?;
            Ok(JsValue::from(true))
        }
        "exportAndCopyHTMLCode" => {
            exec(None, "edit.copyHtmlCode", json!({"style": "htmlAndImages"}))
                .map(|_| JsValue::from(true))
        }
        "updateHTML" => {
            let url = args
                .iter()
                .filter_map(|a| a.str())
                .next()
                .unwrap_or_default();
            exec(
                None,
                "file.updateHtml",
                json!({"path": url_to_path(&url).to_string_lossy()}),
            )
            .map(|_| JsValue::from(true))
        }
        "getHTMLFileForScript" => Ok(JsValue::null()),
        "getNumberOfTables" | "readNthTable" => {
            // Tabellen einer HTML-Datei (für „HTML aktualisieren“-Skripte): einfache Zählung von <table>
            let url = a0.and_then(|a| a.str()).unwrap_or_default();
            let text = std::fs::read(url_to_path(&url))
                .map(|b| crate::decode_text(&b))
                .unwrap_or_default();
            let lower = text.to_lowercase();
            if name == "getNumberOfTables" {
                return Ok(JsValue::from(lower.matches("<table").count() as i32));
            }
            let n = args.get(1).map(|a| a.i64()).unwrap_or(0).max(0) as usize;
            let starts: Vec<usize> = lower.match_indices("<table").map(|(i, _)| i).collect();
            match starts.get(n) {
                Some(&s) => {
                    let e = lower[s..]
                        .find("</table>")
                        .map(|e| s + e + 8)
                        .unwrap_or(text.len());
                    Ok(conv::s(&text[s..e]))
                }
                None => Ok(JsValue::null()),
            }
        }
        "doBatchProcess" => {
            // (files, commands|null, outDir, …)
            let files = args.first().map(|a| a.json()).unwrap_or(Value::Null);
            let files: Vec<String> = files
                .as_array()
                .map(|v| {
                    v.iter()
                        .filter_map(|x| {
                            x.as_str()
                                .map(|u| url_to_path(u).to_string_lossy().to_string())
                        })
                        .collect()
                })
                .unwrap_or_default();
            let out = args
                .iter()
                .skip(1)
                .filter_map(|a| a.str())
                .find(|s| s.contains('/') || s.contains('\\'))
                .map(|u| url_to_path(&u).to_string_lossy().to_string());
            let Some(out) = out else {
                return Err(FwErr::bad("Zielordner fehlt"));
            };
            exec(
                None,
                "file.batchProcess",
                json!({"files": files, "outDir": out}),
            )
            .map(|_| JsValue::from(true))
        }
        "browseHelp" => exec(
            None,
            "help.firepixelHelp",
            json!({"open": crate::shell_open_allowed()}),
        )
        .map(|_| JsValue::undefined()),
        "launchBrowserTo" => {
            let url = a0.and_then(|a| a.str()).unwrap_or_default();
            if !crate::shell_open_allowed() {
                return Ok(JsValue::from(false));
            }
            let target = if url.starts_with("file:") {
                url_to_path(&url)
            } else {
                PathBuf::from(&url)
            };
            Ok(JsValue::from(
                fp_engine::shell::shell_open(&target, None).unwrap_or(false),
            ))
        }
        "launchApp" => {
            let app = a0.and_then(|a| a.str()).unwrap_or_default();
            if !crate::shell_open_allowed() || app.is_empty() {
                return Ok(JsValue::from(false));
            }
            let mut c = std::process::Command::new(url_to_path(&app));
            if let Some(list) = args
                .get(1)
                .map(|a| a.json())
                .and_then(|v| v.as_array().cloned())
            {
                for x in list {
                    if let Some(s) = x.as_str() {
                        c.arg(if s.starts_with("file:") {
                            url_to_path(s).to_string_lossy().to_string()
                        } else {
                            s.to_string()
                        });
                    }
                }
            }
            Ok(JsValue::from(fp_engine::shell::spawn(c).unwrap_or(false)))
        }
        "findApp" | "chooseBrowser" => Ok(JsValue::null()),
        "getPlatformNameForPSFont" | "getFamilyNameForPSFont" | "getStyleNameForPSFont" => {
            let ps = a0.and_then(|a| a.str()).unwrap_or_default();
            let (fam, style) = ps
                .split_once('-')
                .map(|(f, s)| (f.to_string(), s.trim_end_matches("MT").to_string()))
                .unwrap_or((ps.clone(), "Regular".into()));
            let fam = fam
                .trim_end_matches("MT")
                .trim_end_matches("PS")
                .to_string();
            // Binnenmajuskeln trennen: „TimesNewRoman“ → „Times New Roman“ (nur vor Kleinbuchstaben)
            let chars: Vec<char> = fam.chars().collect();
            let mut fam_spaced = String::new();
            for (i, ch) in chars.iter().enumerate() {
                let next_lower = chars.get(i + 1).is_some_and(|c| c.is_lowercase());
                if i > 0 && ch.is_uppercase() && next_lower && !fam_spaced.ends_with(' ') {
                    fam_spaced.push(' ');
                }
                fam_spaced.push(*ch);
            }
            let style = if style.is_empty() {
                "Regular".to_string()
            } else {
                style
            };
            Ok(conv::s(&match name {
                "getFamilyNameForPSFont" => fam_spaced,
                "getStyleNameForPSFont" => style,
                _ => {
                    if style == "Regular" {
                        fam_spaced
                    } else {
                        format!("{fam_spaced} {style}")
                    }
                }
            }))
        }
        "findNext" | "replace" | "replaceAll" | "setUpFindReplace" => find_replace(name, args),
        "addSwatches" => {
            let list = a0.map(|a| a.json()).unwrap_or(Value::Null);
            let colors: Vec<String> = match list {
                Value::Array(v) => v
                    .iter()
                    .filter_map(|x| x.as_str().and_then(conv::norm_color))
                    .collect(),
                Value::String(s) => conv::norm_color(&s).into_iter().collect(),
                _ => Vec::new(),
            };
            for c in colors {
                exec(None, "panel.swatches.addColor", json!({"color": c}))?;
            }
            Ok(JsValue::undefined())
        }
        "setActiveColor" => {
            let c = a0
                .and_then(|a| a.str())
                .and_then(|c| conv::norm_color(&c))
                .unwrap_or_else(|| "#000000".into());
            let stroke = args.get(1).and_then(|a| a.str()).is_some_and(|w| {
                w.to_lowercase().contains("stroke") || w.to_lowercase().contains("brush")
            });
            exec(
                None,
                "color.set",
                if stroke {
                    json!({"stroke": c})
                } else {
                    json!({"fill": c})
                },
            )
            .map(|_| JsValue::undefined())
        }
        "getRotationMatrix" | "getScaleMatrix" => {
            let m = if name == "getRotationMatrix" {
                let r = args.first().map(|a| a.f64()).unwrap_or(0.0).to_radians();
                [r.cos(), r.sin(), 0.0, -r.sin(), r.cos(), 0.0, 0.0, 0.0, 1.0]
            } else {
                let sx = args.first().map(|a| a.f64()).unwrap_or(1.0);
                let sy = args
                    .get(1)
                    .map(|a| if a.is_null() { sx } else { a.f64() })
                    .unwrap_or(sx);
                [sx, 0.0, 0.0, 0.0, sy, 0.0, 0.0, 0.0, 1.0]
            };
            conv::json_to_js(&json!({"matrix": m}), ctx).map_err(|e| FwErr::Js(e.to_string()))
        }
        "saveJsCommand" => {
            // (code, name)
            let code = a0.and_then(|a| a.str()).unwrap_or_default();
            let n = args
                .get(1)
                .and_then(|a| a.str())
                .unwrap_or_else(|| "Befehl".into());
            exec(
                None,
                "commands.manageSavedCommands",
                json!({"action": "save", "name": n, "code": code}),
            )
            .map(|_| JsValue::undefined())
        }
        _ => ni(CLS, name),
    }
}

/// Such-/Ersetz-Parameter aus einem Original-Objekt: (suchen, ersetzen, Groß/klein beachten, ganzes Wort).
fn find_params(v: &Value) -> (String, String, bool, bool) {
    let s = |keys: &[&str]| {
        keys.iter()
            .find_map(|k| v.get(*k).and_then(|x| x.as_str()).map(String::from))
            .unwrap_or_default()
    };
    let b = |keys: &[&str]| {
        keys.iter().any(|k| {
            v.get(*k)
                .is_some_and(|x| x.as_bool() == Some(true) || x.as_str() == Some("true"))
        })
    };
    (
        s(&["find", "findString", "searchString", "what"]),
        s(&["replace", "replaceString", "replaceWith"]),
        b(&["findMatchCase", "matchCase"]),
        b(&["findWholeWord", "wholeWord"]),
    )
}

/// Ersetzt in einem Text; liefert (neuer Text, Anzahl).
fn replace_text(t: &str, find: &str, rep: &str, case: bool, whole: bool) -> (String, usize) {
    if find.is_empty() {
        return (t.to_string(), 0);
    }
    let chars: Vec<char> = t.chars().collect();
    let pat: Vec<char> = find.chars().collect();
    let eq = |a: char, b: char| {
        if case {
            a == b
        } else {
            a.to_lowercase().eq(b.to_lowercase())
        }
    };
    let (mut out, mut n, mut i) = (String::new(), 0usize, 0usize);
    while i < chars.len() {
        let hit = i + pat.len() <= chars.len()
            && pat.iter().enumerate().all(|(k, c)| eq(chars[i + k], *c))
            && (!whole
                || ((i == 0 || !chars[i - 1].is_alphanumeric())
                    && (i + pat.len() == chars.len() || !chars[i + pat.len()].is_alphanumeric())));
        if hit {
            out.push_str(rep);
            n += 1;
            i += pat.len();
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    (out, n)
}

/// Ersetzt in allen (bzw. den angegebenen) Textobjekten eines Dokuments; liefert die Anzahl.
pub(super) fn replace_in_doc(
    doc: DocId,
    params: &Value,
    only: Option<&[fp_model::ElementId]>,
) -> FwResult<usize> {
    let (find, rep, case, whole) = find_params(params);
    let texts: Vec<(fp_model::ElementId, String)> = st(|s| {
        let d = s
            .session
            .document(doc)
            .ok_or_else(|| FwErr::bad("Dokument"))?;
        let ids = fp_engine::docops::selectable_ids(&d.doc, d.page, d.frame);
        Ok(ids
            .into_iter()
            .filter(|id| only.is_none_or(|o| o.contains(id)))
            .filter_map(|id| {
                fp_engine::docops::element_on_page(&d.doc, d.page, id).and_then(|e| {
                    if let fp_model::ElementKind::Text(t) = &e.kind {
                        Some((id, t.plain_text()))
                    } else {
                        None
                    }
                })
            })
            .collect())
    })?;
    let mut total = 0;
    for (id, t) in texts {
        let (next, n) = replace_text(&t, &find, &rep, case, whole);
        if n > 0 {
            exec(Some(doc), "text.setText", json!({"id": id.0, "text": next}))?;
            total += n;
        }
    }
    Ok(total)
}

fn find_replace(name: &str, args: &[Arg]) -> FwResult<JsValue> {
    if name == "setUpFindReplace" {
        let v = args.first().map(|a| a.json()).unwrap_or(Value::Null);
        with_state(|s| s.find = v)?;
        return Ok(JsValue::undefined());
    }
    let params = with_state(|s| s.find.clone())?;
    let Some(doc) = with_state(|s| s.session.active_id())? else {
        return Ok(JsValue::from(false));
    };
    match name {
        "replaceAll" => Ok(JsValue::from(replace_in_doc(doc, &params, None)? as i32)),
        "replace" => {
            let sel = with_state(|s| {
                s.session
                    .document(doc)
                    .map(|d| d.selection.elements.clone())
                    .unwrap_or_default()
            })?;
            Ok(JsValue::from(replace_in_doc(doc, &params, Some(&sel))? > 0))
        }
        _ => {
            // findNext: nächstes Textobjekt mit Treffer nach der aktuellen Auswahl auswählen
            let (find, _, case, whole) = find_params(&params);
            let hit = st(|s| {
                let d = s
                    .session
                    .document(doc)
                    .ok_or_else(|| FwErr::bad("Dokument"))?;
                let ids = fp_engine::docops::selectable_ids(&d.doc, d.page, d.frame);
                let start = d
                    .selection
                    .elements
                    .first()
                    .and_then(|cur| ids.iter().position(|x| x == cur))
                    .map(|p| p + 1)
                    .unwrap_or(0);
                let order: Vec<_> = ids[start..]
                    .iter()
                    .chain(ids[..start].iter())
                    .copied()
                    .collect();
                Ok(order.into_iter().find(|id| {
                    fp_engine::docops::element_on_page(&d.doc, d.page, *id)
                        .is_some_and(|e| matches!(&e.kind, fp_model::ElementKind::Text(t) if replace_text(&t.plain_text(), &find, "", case, whole).1 > 0))
                }))
            })?;
            match hit {
                Some(id) => {
                    exec(Some(doc), "select.set", json!({"ids": [id.0]}))?;
                    Ok(JsValue::from(true))
                }
                None => Ok(JsValue::from(false)),
            }
        }
    }
}

/// Lese-Umleitung (ScriptOptions::read_fallback).
pub(crate) fn resolve_read(p: &std::path::Path) -> PathBuf {
    if p.exists() {
        return p.to_path_buf();
    }
    let fb = with_state(|s| s.opts.read_fallback.clone()).unwrap_or_default();
    for (from, to) in fb {
        if let Ok(rest) = p.strip_prefix(&from) {
            let alt = to.join(rest);
            if alt.exists() {
                return alt;
            }
        }
    }
    p.to_path_buf()
}

pub(super) fn history_method(name: &str, args: &[Arg], ctx: &mut Context) -> FwResult<JsValue> {
    match name {
        "getStepCount" => Ok(JsValue::from(with_state(|s| {
            s.session
                .active()
                .map(|d| d.history.undo_count() + d.history.redo_count())
                .unwrap_or(0)
        })? as i32)),
        "getStepsAsJavaScript" => {
            let idx: Vec<usize> = match args.first().map(|a| a.json()) {
                Some(Value::Array(v)) => v
                    .iter()
                    .filter_map(|x| x.as_u64().map(|i| i as usize))
                    .collect(),
                _ => Vec::new(),
            };
            let text = with_state(|s| {
                let Some(d) = s.session.active() else {
                    return String::new();
                };
                let steps = d.history.steps();
                idx.iter()
                    .filter_map(|i| steps.get(*i))
                    .map(|st| {
                        format!(
                            "fp.run({}, {});",
                            serde_json::to_string(&st.command).unwrap_or_default(),
                            st.params
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })?;
            Ok(conv::s(&text))
        }
        "clearSteps" => {
            st(|s| {
                if let Some(d) = s.session.active_mut() {
                    d.history.clear();
                }
                Ok(())
            })?;
            Ok(JsValue::undefined())
        }
        "replaySteps" => {
            let idx: Vec<usize> = match args.first().map(|a| a.json()) {
                Some(Value::Array(v)) => v
                    .iter()
                    .filter_map(|x| x.as_u64().map(|i| i as usize))
                    .collect(),
                _ => Vec::new(),
            };
            st(|s| {
                let cmds: Vec<fp_engine::RecordedCommand> = match s.session.active() {
                    Some(d) => {
                        let steps = d.history.steps();
                        idx.iter()
                            .filter_map(|i| steps.get(*i))
                            .map(|st| fp_engine::RecordedCommand {
                                command: st.command.clone(),
                                params: st.params.clone(),
                            })
                            .collect()
                    }
                    None => Vec::new(),
                };
                s.session
                    .replay(&cmds, "Schritte wiederholen")
                    .map_err(FwErr::from)
            })?;
            Ok(JsValue::undefined())
        }
        "saveAsCommand" => {
            // (indices, url): Schritte als Skript; mit url zusätzlich als Datei speichern
            let idx: Vec<usize> = match args.first().map(|a| a.json()) {
                Some(Value::Array(v)) => v
                    .iter()
                    .filter_map(|x| x.as_u64().map(|i| i as usize))
                    .collect(),
                _ => Vec::new(),
            };
            let text = with_state(|s| {
                let Some(d) = s.session.active() else {
                    return String::new();
                };
                let steps = d.history.steps();
                idx.iter()
                    .filter_map(|i| steps.get(*i))
                    .map(|st| {
                        format!(
                            "fp.run({}, {});",
                            serde_json::to_string(&st.command).unwrap_or_default(),
                            st.params
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })?;
            if let Some(url) = args.get(1).and_then(|a| a.str()) {
                let p = url_to_path(&url);
                fp_i18n::profile::guard_write(&p);
                if let Some(dir) = p.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                std::fs::write(&p, &text)
                    .map_err(|e| FwErr::generic(format!("{}: {e}", p.display())))?;
            }
            Ok(conv::s(&text))
        }
        "getSelection" => Ok(conv::array(Vec::new(), ctx)),
        "setSelection" | "copySteps" | "setUndoState" => Ok(JsValue::undefined()),
        "getUndoState" => Ok(JsValue::from(true)),
        _ => ni("HistoryPalette", name),
    }
}

pub(super) fn errors_get(name: &str) -> FwResult<JsValue> {
    if name == "EUserCanceled" {
        return Ok(JsValue::from(f64::from(hr::E_USER_CANCELED as i32)));
    }
    let _ = bindings::error_hr(name);
    Ok(conv::s(&bindings::error_text(name)))
}

pub(super) fn system_get(name: &str) -> FwResult<JsValue> {
    Ok(match name {
        "osName" => conv::s(if cfg!(windows) { "Windows" } else { "Mac OS" }),
        "controlFaceColor" | "menuColor" => conv::s("#f0f0f0"),
        "controlHilightColor" => conv::s("#ffffff"),
        "controlShadowColor" => conv::s("#a0a0a0"),
        "controlDarkShadowColor" => conv::s("#696969"),
        "hilightItemColor" => conv::s("#0078d7"),
        "hilightTextColor" => conv::s("#ffffff"),
        _ => conv::s("#000000"),
    })
}

pub(super) fn dollar_get(name: &str) -> FwResult<JsValue> {
    Ok(match name {
        "version" => conv::s(env!("CARGO_PKG_VERSION")),
        "build" | "buildDate" => conv::s("Firepixel"),
        "locale" => conv::s("de_DE"),
        "os" => conv::s(if cfg!(windows) { "Windows" } else { "Mac OS" }),
        "fileName" => match script_dir() {
            Some(p) => conv::s(&path_to_url(&p)),
            None => conv::s(""),
        },
        _ => return ni("$", name),
    })
}

pub(super) fn dollar_method(name: &str, args: &[Arg], ctx: &mut Context) -> FwResult<JsValue> {
    match name {
        "getenv" => {
            let k = args.first().and_then(|a| a.str()).unwrap_or_default();
            Ok(std::env::var(&k)
                .map(|v| conv::s(&v))
                .unwrap_or_else(|_| JsValue::null()))
        }
        "sleep" => {
            let ms = args.first().map(|a| a.i64()).unwrap_or(0).clamp(0, 5_000) as u64;
            std::thread::sleep(std::time::Duration::from_millis(ms));
            Ok(JsValue::undefined())
        }
        "evalFile" => {
            let url = args.first().and_then(|a| a.str()).unwrap_or_default();
            super::run_script_file(&resolve_read(&url_to_path(&url)), ctx)
        }
        "gc" => Ok(JsValue::undefined()),
        _ => ni("$", name),
    }
}

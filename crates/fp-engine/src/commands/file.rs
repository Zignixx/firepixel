//! Menü „Datei“ sowie Voreinstellungen, Tastenkürzel-Sets und Bedienfelder (Menü „Fenster“).

use super::DOC;
use crate::command::{cmd, CommandOutput, CommandRegistry, ParamKind, Requirement};
use crate::error::{EngineError, Result};
use crate::prefs::PrefValue;
use crate::render::RendererFlattener;
use crate::session::{Session, SessionEvent};
use fp_color::Argb;
use fp_geom::Point;
use fp_model::{Document, Element};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Liest eine Datei als Dokument: FW-PNG über fp-fwpng, sonst ein normales PNG als Bitmap.
pub fn load_document(path: &Path) -> Result<Document> {
    let bytes = std::fs::read(path)?;
    if fp_fwpng::is_fwpng(&bytes) {
        return Ok(fp_fwpng::read(&bytes)?);
    }
    // andere Formate (PSD, GIF, JPEG, TIFF, SVG, WebP …) über fp-io
    if let Ok(r) = fp_io::open_bytes(&path.to_string_lossy(), &bytes, &fp_io::ImportOptions::with_installation()) {
        return Ok(r.document);
    }
    let chunks = fp_fwpng::png::read_chunks(&bytes)?;
    let img = fp_fwpng::png::decode_image(&chunks)?;
    let mut doc = Document::new(img.width.max(1) as f32, img.height.max(1) as f32, Argb(0x00ff_ffff));
    let mut ids = std::mem::take(&mut doc.ids);
    let mut next = crate::docops::tile_counter();
    let e = Element::new_bitmap(&mut ids, Point::new(0.0, 0.0), img.width, img.height, &img.rgba, &mut next);
    doc.ids = ids;
    if let Some(l) = crate::docops::top_normal_layer(&doc, 0)
        && crate::docops::insert_element(&mut doc, 0, 0, Some(l), 0, Arc::new(e)).is_none()
    {
        return Err(EngineError::Failed(fp_i18n::t("err-image-insert-failed")));
    }
    Ok(doc)
}

/// Schreibt das Dokument als FW-PNG (mit Renderer für das flache Bild, falls vorhanden).
pub fn save_document(s: &Session, doc: &Document, page: usize, path: &Path) -> Result<()> {
    let mut d = doc.clone();
    if d.current_page != page && page < d.pages.len() {
        d.current_page = page;
        d.invalidate_root_copy();
    }
    let opts = fp_fwpng::WriteOptions { software: "Firepixel".into(), ..fp_fwpng::WriteOptions::default() };
    let bytes = match s.renderer() {
        Some(r) => match r.flattener() {
            Some(f) => fp_fwpng::write(&d, &opts, f)?,
            None => fp_fwpng::write(&d, &opts, &RendererFlattener(r.as_ref()))?,
        },
        None => fp_fwpng::write(&d, &opts, &fp_fwpng::NoFlattener)?,
    };
    write_atomic(path, &bytes)
}

fn open_path(s: &mut Session, path: PathBuf, as_template: bool) -> Result<CommandOutput> {
    let doc = load_document(&path)?;
    let id = s.add_document(doc, if as_template { None } else { Some(path.clone()) });
    if !as_template {
        s.prefs.add_recent_file(&path);
        s.prefs_changed();
    }
    Ok(CommandOutput::session_with(json!({"doc": id.0})))
}

/// Hängt die erste Seite von `other` als neuen letzten Status an `doc` an (Elemente aller eigenen
/// Ebenen, oberste zuerst, mit neuen IDs; „Als Animation öffnen“).
fn append_as_frame(doc: &mut Document, other: Document) {
    let Some(page) = other.pages.first() else { return };
    let mut elements: Vec<Element> = Vec::new();
    for slot in &page.root.layers {
        if let fp_model::layer::LayerSlot::Own(l) = slot
            && l.kind == fp_model::layer::LayerKind::Normal
            && let Some(c) = l.cel(0)
        {
            elements.extend(c.elements.iter().map(|e| (**e).clone()));
        }
    }
    let at = doc.pages.first().map(|p| p.root.frame_count as usize).unwrap_or(1);
    if crate::docops::insert_frames(doc, 0, at, 1).is_none() {
        return;
    }
    let mut ids = std::mem::take(&mut doc.ids);
    for e in &mut elements {
        e.reassign_ids(&mut ids);
    }
    doc.ids = ids;
    for (i, e) in elements.into_iter().enumerate() {
        crate::docops::insert_element(doc, 0, at, None, i, Arc::new(e));
    }
}

/// Schreibt Bytes über eine Zwischendatei (kein halb geschriebenes Ziel bei Fehlern).
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    fp_i18n::profile::guard_write(path);
    if let Some(dir) = path.parent()
        && !dir.as_os_str().is_empty()
    {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("fpsave.tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Speichert das aktive Dokument. `flattened`: `Some(true)` = „Reduziertes PNG“ (re\20 §5),
/// `Some(false)` = FW-PNG, `None` = Art des Dokuments (nach einem Speichern als reduziertes PNG
/// wieder flach). `adopt` = das Dokument gehört danach zur Datei (Pfad, Titel, Art, gespeichert);
/// sonst bleibt es unverändert („Als Kopie speichern“).
fn save_active(s: &mut Session, path: Option<PathBuf>, adopt: bool, flattened: Option<bool>) -> Result<CommandOutput> {
    let d = s.doc()?;
    let target = path.or_else(|| d.path.clone()).ok_or_else(|| EngineError::InvalidParams {
        name: "file.save".into(),
        message: fp_i18n::t("err-param-path-needed"),
    })?;
    let flat = flattened.unwrap_or(d.flattened);
    let (doc, page, id) = (Arc::clone(&d.doc), d.page, d.id);
    if flat {
        let bytes = super::export::flattened_png_bytes(s)?;
        write_atomic(&target, &bytes)?;
    } else {
        save_document(s, &doc, page, &target)?;
    }
    if adopt {
        let d = s.doc_mut()?;
        d.path = Some(target.clone());
        d.title = target.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        d.saved_generation = d.generation;
        d.flattened = flat;
        s.prefs.add_recent_file(&target);
    }
    s.emit(SessionEvent::Saved { doc: id, path: target.clone() });
    Ok(CommandOutput::session_with(json!({"path": target.to_string_lossy(), "flattened": flat, "copy": !adopt})))
}

fn toggle_panel(s: &mut Session, name: &str, default: bool) -> Result<CommandOutput> {
    let v = !s.ui.panels.get(name).copied().unwrap_or(default);
    s.ui.panels.insert(name.to_string(), v);
    s.emit(SessionEvent::PanelsChanged);
    Ok(CommandOutput::session_with(json!({"visible": v})))
}

/// Bedienfelder (Fenster-Menü) mit Vorgabe-Sichtbarkeit.
pub const PANELS: &[(&str, bool)] = &[
    ("window.toolbars.main", true),
    ("window.tools", true),
    ("window.properties", true),
    ("window.optimize", true),
    ("window.layers", true),
    ("window.commonLibrary", false),
    ("window.cssProperties", false),
    ("window.pages", true),
    ("window.states", true),
    ("window.history", false),
    ("window.autoShapes", false),
    ("window.styles", false),
    ("window.documentLibrary", false),
    ("window.url", false),
    ("window.colorMixer", false),
    ("window.swatches", false),
    ("window.info", false),
    ("window.behaviors", false),
    ("edit.findAndReplace", false),
    ("window.jquery", false),
];

pub(super) fn register(r: &mut CommandRegistry) {
    r.add(
        cmd("file.new")
            .opt("width", ParamKind::Number { min: Some(1.0), max: Some(30000.0) }, "Breite in Pixel (Vorgabe DefaultDocSize)")
            .opt("height", ParamKind::Number { min: Some(1.0), max: Some(30000.0) }, "Höhe in Pixel")
            .opt("background", ParamKind::Color, "Leinwandfarbe (Vorgabe nach DefaultDocColorState)")
            .opt("resolution", ParamKind::Number { min: Some(1.0), max: Some(5000.0) }, "Auflösung (Pixel/Zoll)")
            .run(|s, p| {
                let (dw, dh) = s.prefs.new_document_size();
                let w = p.f64_or("width", dw as f64)? as f32;
                let h = p.f64_or("height", dh as f64)? as f32;
                let bg = p.opt_color("background")?.unwrap_or_else(|| s.prefs.new_document_background());
                let res = s.prefs.new_document_resolution();
                let mut doc = Document::new(w, h, bg);
                if let Some(pg) = doc.pages.first_mut() {
                    let pg = Arc::make_mut(pg);
                    pg.canvas.resolution = p.f64_or("resolution", res.value as f64)? as f32;
                    pg.canvas.resolution_unit = if res.per_cm && !p.has("resolution") { 1 } else { 0 };
                    pg.root.grid = Some(s.prefs.new_document_grid());
                }
                let id = s.add_document(doc, None);
                Ok(CommandOutput::session_with(json!({"doc": id.0})))
            }),
    );
    r.add(
        cmd("file.newFromTemplate")
            .param("path", ParamKind::Path, "Vorlagendatei (.fw.png)")
            .run(|s, p| open_path(s, PathBuf::from(p.str("path")?), true)),
    );
    r.add(
        cmd("file.open")
            .param("path", ParamKind::Path, "Datei (.fw.png oder .png)")
            .opt_default("untitled", ParamKind::Bool, json!(false), "als „Unbenannt“ öffnen (ohne Dateinamen; Speichern fragt nach einem Namen)")
            .opt("frames", ParamKind::Json, "weitere Dateien [Pfad, …]: „Als Animation öffnen“, je Datei ein Status nach dem ersten")
            .run(|s, p| {
                let path = PathBuf::from(p.str("path")?);
                let untitled = p.bool_or("untitled", false)?;
                let frames: Vec<PathBuf> = match p.get("frames") {
                    Some(Value::Array(a)) => a.iter().filter_map(|v| v.as_str()).map(PathBuf::from).collect(),
                    Some(Value::String(one)) => vec![PathBuf::from(one)],
                    _ => Vec::new(),
                };
                if !untitled && frames.is_empty() {
                    return open_path(s, path, false);
                }
                // Original: „Als Animation öffnen“ legt die gewählten Dateien als Status eines
                // unbenannten Dokuments an; „Als ‚Unbenannt‘ öffnen“ öffnet ohne Dateinamen
                let mut doc = load_document(&path)?;
                for f in &frames {
                    append_as_frame(&mut doc, load_document(f)?);
                }
                let id = s.add_document(doc, None);
                s.prefs.add_recent_file(&path);
                s.prefs_changed();
                Ok(CommandOutput::session_with(json!({"doc": id.0})))
            }),
    );
    r.add(
        cmd("file.openRecent")
            .opt_default("index", ParamKind::Integer { min: Some(0), max: Some(9) }, json!(0), "Index in der Liste zuletzt geöffneter Dateien")
            .available(|s| !s.prefs.recent_files.is_empty())
            .run(|s, p| {
                let i = p.i64_or("index", 0)?.max(0) as usize;
                let path = s.prefs.recent_files.get(i).cloned().ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-recent-file")))?;
                open_path(s, path, false)
            }),
    );
    r.add(
        cmd("file.close")
            .requires(DOC)
            .opt_default("discard", ParamKind::Bool, json!(false), "ungespeicherte Änderungen verwerfen")
            .run(|s, p| {
                let d = s.doc()?;
                if d.symbol_edit.is_some() {
                    // Symbol-Bearbeitungsansicht: Schließen = „Fertig“ (bzw. verwerfen)
                    let id = d.id;
                    super::symbol_edit::finish_edit(s, id, p.bool_or("discard", false)?)?;
                    return Ok(CommandOutput::session());
                }
                if d.is_dirty() && !p.bool_or("discard", false)? {
                    return Err(EngineError::Failed(fp_i18n::tr!("err-unsaved-discard", name = d.title.as_str())));
                }
                let id = d.id;
                s.close_document(id)?;
                Ok(CommandOutput::session())
            }),
    );
    r.add(
        cmd("file.closeAll")
            .requires(DOC)
            .opt_default("discard", ParamKind::Bool, json!(false), "ungespeicherte Änderungen verwerfen")
            .run(|s, p| {
                let discard = p.bool_or("discard", false)?;
                if !discard && let Some(d) = s.documents().iter().find(|d| d.is_dirty()) {
                    return Err(EngineError::Failed(fp_i18n::tr!("err-unsaved", name = d.title.as_str())));
                }
                let ids: Vec<_> = s.documents().iter().map(|d| d.id).collect();
                for id in ids {
                    s.close_document(id)?;
                }
                Ok(CommandOutput::session())
            }),
    );
    r.add(
        cmd("file.save")
            .requires(DOC)
            .opt("path", ParamKind::Path, "Zieldatei (nötig, wenn das Dokument noch keinen Namen hat)")
            .run(|s, p| save_active(s, p.opt_str("path")?.map(PathBuf::from), true, None)),
    );
    r.add(
        cmd("file.saveAs")
            .requires(DOC)
            .param("path", ParamKind::Path, "Zieldatei")
            .opt_default("flattened", ParamKind::Bool, json!(false), "reduziertes PNG speichern")
            .opt_default("copy", ParamKind::Bool, json!(false), "als Kopie speichern")
            .run(|s, p| {
                let copy = p.bool_or("copy", false)?;
                save_active(s, Some(PathBuf::from(p.str("path")?)), !copy, Some(p.bool_or("flattened", false)?))
            }),
    );
    r.add(cmd("file.saveAll").requires(DOC).run(|s, _| {
        let ids: Vec<_> = s.documents().iter().filter(|d| d.path.is_some()).map(|d| d.id).collect();
        let active = s.active_id();
        let mut saved = Vec::new();
        for id in ids {
            s.activate(id)?;
            if let Ok(o) = save_active(s, None, true, None) {
                saved.push(o.value);
            }
        }
        if let Some(a) = active {
            s.activate(a)?;
        }
        Ok(CommandOutput::session_with(Value::Array(saved)))
    }));
    r.add(
        cmd("file.saveAsTemplate")
            .requires(DOC)
            .param("path", ParamKind::Path, "Vorlagendatei")
            .run(|s, p| save_active(s, Some(PathBuf::from(p.str("path")?)), false, Some(false))),
    );
    r.add(cmd("file.revert").requires(&[Requirement::Document, Requirement::HasPath]).run(|s, _| {
        let path = s.doc()?.path.clone().ok_or_else(|| EngineError::Failed(fp_i18n::t("err-no-filename")))?;
        let doc = load_document(&path)?;
        let prefs_max = s.prefs.max_undos();
        let d = s.doc_mut()?;
        d.doc = Arc::new(doc);
        d.history = crate::history::History::new(prefs_max);
        d.selection.clear();
        d.page = d.doc.current_page.min(d.doc.pages.len().saturating_sub(1));
        d.frame = 0;
        d.layer = crate::docops::top_normal_layer(&d.doc, d.page);
        d.generation += 1;
        d.saved_generation = d.generation;
        let (id, g) = (d.id, d.generation);
        s.emit(SessionEvent::DocumentChanged { doc: id, generation: g });
        s.emit(SessionEvent::HistoryChanged(id));
        Ok(CommandOutput::session())
    }));
    // „Datei-Info“ (file.fileInfo) steht in `file_info.rs`.
    for (name, i) in [("file.previewInBrowser.setPrimaryBrowser", 0usize), ("file.previewInBrowser.setSecondaryBrowser", 1)] {
        r.add(cmd(name).param("path", ParamKind::Path, "Programmdatei des Browsers").run(move |s, p| {
            s.ui.browsers[i] = Some(p.str("path")?);
            Ok(CommandOutput::session())
        }));
    }
    r.add(cmd("file.exit").run(|s, _| {
        s.ui.quit_requested = true;
        s.emit(SessionEvent::QuitRequested);
        Ok(CommandOutput::session())
    }));
    r.add(cmd("help.about").run(|_, _| {
        Ok(CommandOutput::value(json!({"name": "Firepixel", "version": env!("CARGO_PKG_VERSION"), "engine": "fp-engine"})))
    }));

    // Voreinstellungen und Tastenkürzel
    r.add(
        cmd("edit.preferences")
            .opt("key", ParamKind::String, "Schlüssel (z. B. MaxUndos); ohne Wert wird gelesen")
            .opt("value", ParamKind::Json, "neuer Wert")
            .opt("values", ParamKind::Json, "mehrere Werte {Schlüssel: Wert}")
            .opt("theme", ParamKind::Enum { values: vec!["normal".into(), "dark".into(), "oled".into()] }, "Design der Oberfläche")
            .run(|s, p| {
                let mut changed = false;
                if let Some(t) = p.opt_str("theme")? {
                    s.prefs.theme = crate::prefs::Theme::from_name(&t).unwrap_or_default();
                    changed = true;
                }
                if let Some(Value::Object(m)) = p.get("values") {
                    for (k, v) in m {
                        if let Some(pv) = PrefValue::from_json(v) {
                            s.prefs.set(k, pv);
                            changed = true;
                        }
                    }
                }
                let key = p.opt_str("key")?;
                if let (Some(k), Some(v)) = (&key, p.get("value")) {
                    let pv = PrefValue::from_json(v).ok_or_else(|| EngineError::InvalidParams { name: "edit.preferences".into(), message: fp_i18n::t("err-param-value") })?;
                    s.prefs.set(k, pv);
                    changed = true;
                }
                if changed {
                    // MaxUndos wirkt (wie das Original) erst für neu angelegte Dokumente
                    s.prefs_changed();
                }
                let value = key.as_deref().and_then(|k| s.prefs.get(k)).map(|v| v.to_json()).unwrap_or(Value::Null);
                Ok(CommandOutput::session_with(json!({"value": value, "theme": s.prefs.theme})))
            }),
    );
    r.add(
        cmd("edit.keyboardShortcuts")
            .opt("set", ParamKind::String, "aktives Tastenkürzel-Set (mitgeliefert oder eigenes)")
            .opt("duplicate", ParamKind::String, "aktives Set als eigenes Set mit diesem Namen speichern und aktivieren")
            .opt("command", ParamKind::String, "Befehlsname (z. B. file.save) für neue Kürzel im aktiven eigenen Set")
            .opt("tool", ParamKind::String, "Werkzeug (z. B. tool.pen) für neue Kürzel im aktiven eigenen Set")
            .opt("shortcuts", ParamKind::Json, "Kürzel [\"Strg+Umschalt+K\", …] (leer = entfernen)")
            .opt("deleteSet", ParamKind::String, "eigenes Set löschen")
            .run(|s, p| {
                let bad = |m: String| EngineError::InvalidParams { name: "edit.keyboardShortcuts".into(), message: m };
                if let Some(name) = p.opt_str("set")? {
                    let set = crate::shortcuts::any_set(&name).ok_or_else(|| EngineError::NotFound(fp_i18n::tr!("err-nf-shortcut-set", name = name.as_str())))?;
                    s.prefs.set("CurrentShortcutSet", PrefValue::Str(set.name.clone()));
                    s.prefs_changed();
                }
                if let Some(name) = p.opt_str("duplicate")? {
                    let mut set = s.shortcut_set().clone();
                    set.name = name.trim().to_string();
                    let saved = crate::shortcuts::save_custom_set(&set).map_err(bad)?;
                    s.prefs.set("CurrentShortcutSet", PrefValue::Str(saved.name.clone()));
                    s.prefs_changed();
                }
                if p.has("command") || p.has("tool") {
                    let list: Vec<String> = p.parse("shortcuts")?.unwrap_or_default();
                    let keys: Vec<crate::shortcuts::Shortcut> = list.iter().map(|k| crate::shortcuts::Shortcut::parse(k).ok_or_else(|| bad(fp_i18n::tr!("err-unknown-shortcut", name = k.as_str())))).collect::<Result<_>>()?;
                    let cur = s.shortcut_set();
                    if crate::shortcuts::builtin_set(&cur.name).is_some() {
                        return Err(bad(fp_i18n::t("err-builtin-set-readonly")));
                    }
                    let mut set = cur.clone();
                    if let Some(c) = p.opt_str("command")? {
                        let reg = std::sync::Arc::clone(s.registry());
                        let info = reg.get(&c).map(|x| x.info().clone()).ok_or_else(|| EngineError::UnknownCommand(c.clone()))?;
                        let key = info.fw_cmd.clone().unwrap_or_else(|| info.name.clone());
                        set.set_command(&key, keys.clone()).map_err(bad)?;
                    }
                    if let Some(t) = p.opt_str("tool")? {
                        let id = crate::tools::by_name(&t).ok_or_else(|| bad(fp_i18n::tr!("err-unknown-tool", name = t.as_str())))?;
                        let guid = crate::tools::info(id).map(|i| i.guid).unwrap_or_default();
                        set.set_tool(guid, keys).map_err(bad)?;
                    }
                    crate::shortcuts::save_custom_set(&set).map_err(bad)?;
                    s.prefs_changed();
                }
                if let Some(name) = p.opt_str("deleteSet")? {
                    crate::shortcuts::delete_custom_set(&name).map_err(bad)?;
                    if s.prefs.shortcut_set().eq_ignore_ascii_case(&name) {
                        s.prefs.set("CurrentShortcutSet", PrefValue::Str(crate::shortcuts::builtin_set_names().first().map(|n| n.to_string()).unwrap_or_default()));
                    }
                    s.prefs_changed();
                }
                Ok(CommandOutput::session_with(json!({"set": s.prefs.shortcut_set(), "available": crate::shortcuts::builtin_set_names(), "custom": crate::shortcuts::custom_set_names()})))
            }),
    );

    // Bedienfelder (Fenster-Menü, Weg 4) – Zustand für die UI
    for &(name, default) in PANELS {
        r.add(cmd(name).checked(move |s| s.ui.panels.get(name).copied().unwrap_or(default)).run(move |s, _| toggle_panel(s, name, default)));
    }
    r.add(cmd("window.hidePanels").checked(|s| s.ui.panels_hidden).run(|s, _| {
        s.ui.panels_hidden = !s.ui.panels_hidden;
        s.emit(SessionEvent::PanelsChanged);
        Ok(CommandOutput::session())
    }));
    r.add(cmd("view.toggleScreenMode").label("Bildschirmmodus umschalten").describe("Pseudo-Werkzeug 33015 (Kürzel F)").run(|s, _| {
        s.ui.screen_mode = (s.ui.screen_mode + 1) % 3;
        s.emit(SessionEvent::PanelsChanged);
        Ok(CommandOutput::session_with(json!({"mode": s.ui.screen_mode})))
    }));
}

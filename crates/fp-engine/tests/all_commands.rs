//! Funktionstest **aller** Befehle des Registers.
//!
//! Für jeden Befehl: passendes Testdokument und Auswahl (aus den Verfügbarkeitsbedingungen
//! abgeleitet bzw. aus der Tabelle [`case_for`]), sinnvolle Parameter, dann
//!
//! 1. **Wirkung**: Dokument, Auswahl, Ansicht, Sitzung (Voreinstellungen, Werkzeuge, Bedienfelder,
//!    Zwischenablage, Dokumentliste) oder ein Rückgabewert müssen sich ändern; Befehle mit
//!    Verlaufsschritt müssen das Dokument bzw. die Auswahl ändern. Für viele Befehle prüft eine
//!    eigene Prüfung das **Ergebnis** am Dokument ([`check_for`]).
//! 2. **Verlauf**: höchstens ein neuer Schritt; Rückgängig stellt Dokument und Auswahl exakt
//!    (gleicher Schnappschuss) wieder her, Wiederholen ebenso den Zustand danach.
//! 3. **Datei**: Das geänderte Dokument wird als FW-PNG gespeichert, mit fp-fwpng gelesen und
//!    strukturell verglichen (Seiten, Ebenen, Status, Elementarten, Symbole).
//!
//! Ergebnis als Zahl `geprüft/gesamt` (mit `--nocapture`).

use crate::common;

use common::*;
use fp_engine::{CommandRegistry, ObjectKind, Requirement, Session};
use fp_model::{Document, ElementId, LayerSlot};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

type Setup = Box<dyn Fn(&mut Session) -> Value>;
type Check = Box<dyn Fn(&Session, &Value) -> Result<(), String>>;

struct Case {
    params: Value,
    setup: Setup,
    check: Option<Check>,
    /// Befehl ändert absichtlich nichts Beobachtbares im Testzustand (nur dokumentierte Ausnahmen).
    no_effect_ok: bool,
}

/// Einfache Skript-Laufzeit für den Test (die echte JSF-Laufzeit steckt fp-script ein): je Zeile
/// `fp.run("befehl", {json});` oder nur ein Befehlsname.
struct LineHost;

impl fp_engine::ScriptHost for LineHost {
    fn run_script(&self, s: &mut Session, code: &str, _path: Option<&std::path::Path>) -> fp_engine::Result<Value> {
        let mut last = Value::Null;
        for line in code.lines().map(str::trim).filter(|l| !l.is_empty()) {
            let (name, params) = match line.strip_prefix("fp.run(\"") {
                Some(rest) => {
                    let (n, p) = rest.split_once("\",").unwrap_or((rest.trim_end_matches("\");"), "{});"));
                    (n.to_string(), serde_json::from_str(p.trim().trim_end_matches(';').trim_end_matches(')')).unwrap_or(json!({})))
                }
                None => (line.to_string(), json!({})),
            };
            last = s.execute(&name, params)?.value;
        }
        Ok(last)
    }

    /// Ersatz der Formen-Laufzeit: legt ein Rechteck an bzw. vermerkt die Aktion in customData
    /// (prüft die Weitergabe der Befehle; die Formen selbst testet fp-script).
    fn auto_shape(&self, s: &mut Session, op: &Value) -> fp_engine::Result<Value> {
        let kind = op["op"].as_str().unwrap_or("");
        match kind {
            "list" => return Ok(json!([{"name": "Pfeil", "group": "tool"}])),
            "describe" => return Ok(json!({"params": [], "controlPoints": []})),
            _ => {}
        }
        let d = s.doc_mut()?;
        let (page, frame, layer) = (d.page, d.frame, d.layer);
        let doc = Arc::make_mut(&mut d.doc);
        if kind == "insert" {
            let mut ids = std::mem::take(&mut doc.ids);
            let e = fp_model::Element::new_rectangle(&mut ids, fp_geom::Rect::new(10.0, 10.0, 50.0, 50.0), 0.0, fp_model::PathAttrs::default());
            doc.ids = ids;
            let id = e.id;
            fp_engine::docops::insert_element(doc, page, frame, layer, 0, Arc::new(e));
            d.selection.set(vec![id]);
            return Ok(json!({"id": id.0}));
        }
        let id = ElementId(op["id"].as_u64().unwrap_or(0));
        fp_engine::docops::edit_by_id(doc, page, id, |e| e.common.custom_data.set_scalar("autoShape", kind)).ok_or(fp_engine::EngineError::ElementNotFound(id.0))?;
        Ok(json!({"id": id.0}))
    }
}

/// Die Tests dieser Datei teilen Ordner und Dateinamen: nacheinander ausführen.
fn serial() -> std::sync::MutexGuard<'static, ()> {
    static M: std::sync::Mutex<()> = std::sync::Mutex::new(());
    M.lock().unwrap_or_else(|p| p.into_inner())
}

fn tmp() -> PathBuf {
    let d = std::env::temp_dir().join("fp_all_commands");
    let _ = std::fs::create_dir_all(&d);
    d
}

fn renderer() -> Option<Arc<dyn fp_engine::render::PageRenderer>> {
    static R: OnceLock<Option<Arc<dyn fp_engine::render::PageRenderer>>> = OnceLock::new();
    R.get_or_init(fp_engine::render::default_renderer).clone()
}

/// Testdokument: Rechteck-Grundform, Ellipse, Text, Bitmap, Hotspot, Slice (oben → unten: Slice, Hotspot, Bitmap, Text, Ellipse, Rechteck).
struct Ids {
    rect: ElementId,
    ellipse: ElementId,
    text: ElementId,
    bitmap: ElementId,
    hotspot: ElementId,
    slice: ElementId,
    layer: u64,
}

fn fixture() -> (Session, Ids) {
    // Batch-Laufzeit einstecken wie App, CLI und MCP (sonst melden batch.* „fehlt fp-batch“)
    fp_batch::install();
    let mut s = Session::default();
    s.set_renderer(renderer());
    s.set_script_host(Some(Arc::new(LineHost)));
    // genug Verlaufsschritte für die Wiederholungstests (50× + Vorbereitung)
    s.prefs.set("MaxUndos", fp_engine::PrefValue::Int(200));
    ex(&mut s, "file.new", json!({"width": 400, "height": 300, "background": "#ffffff"}));
    // neue Dokumente exportieren als PNG 32 (BA-033); die Paletten- und Größenbefehle brauchen GIF
    ex(&mut s, "pi.export.settings", json!({"settings": serde_json::to_value(fp_model::ExportSettings::default()).unwrap_or_default()}));
    let rect = ElementId(ex(&mut s, "draw.rectangle", json!({"rect": [10, 10, 60, 40], "fillColor": "#3366cc", "strokeWidth": 2, "strokeColor": "#000000"}))["id"].as_u64().unwrap_or(0));
    let ellipse = ElementId(ex(&mut s, "draw.ellipse", json!({"rect": [100, 20, 160, 80], "fillColor": "#cc3333"}))["id"].as_u64().unwrap_or(0));
    let text = ElementId(ex(&mut s, "draw.text", json!({"text": "Hallo Welt", "anchor": [20, 150], "size": 20, "font": "Arial", "color": "#000000"}))["id"].as_u64().unwrap_or(0));
    let bitmap = common::bitmap(&mut s, 200, 100, 20, 20, [200, 100, 50, 255]);
    let hotspot = ElementId(ex(&mut s, "web.addHotspot", json!({"shape": "rect", "rect": [300, 10, 340, 40]}))["id"].as_u64().unwrap_or(0));
    let slice = ElementId(ex(&mut s, "web.addSlice", json!({"shape": "rect", "rect": [300, 60, 360, 100]}))["id"].as_u64().unwrap_or(0));
    ex(&mut s, "select.none", json!({}));
    // Verlauf der Vorbereitung verwerfen, damit Rückgängig nur den geprüften Befehl betrifft
    if let Ok(d) = s.doc_mut() {
        d.history.clear();
    }
    let layer = s.active().and_then(|d| d.layer).map(|l| l.0).unwrap_or(0);
    (s, Ids { rect, ellipse, text, bitmap, hotspot, slice, layer })
}

fn first_id(v: &Value) -> ElementId {
    ElementId(v["id"].as_u64().or_else(|| v["ids"][0].as_u64()).unwrap_or(0))
}

/// Auswahl passend zu den Verfügbarkeitsbedingungen.
fn auto_setup(reqs: &[Requirement], ids: &Ids) -> Setup {
    let (rect, ellipse, text, bitmap, hotspot, slice) = (ids.rect, ids.ellipse, ids.text, ids.bitmap, ids.hotspot, ids.slice);
    let mut kind = None;
    let mut min = 0usize;
    let mut pixels = false;
    let mut clip = false;
    let mut undo = false;
    let mut redo = false;
    let mut has_path = false;
    let mut pages = false;
    let mut frames = false;
    let mut sel = false;
    for r in reqs {
        match r {
            Requirement::AllOf(k) | Requirement::AnyOf(k) => kind = Some(*k),
            Requirement::MinSelected(n) => min = *n,
            Requirement::PixelSelection => pixels = true,
            Requirement::Selection | Requirement::SelectionOrPixels => sel = true,
            Requirement::Clipboard => clip = true,
            Requirement::CanUndo => undo = true,
            Requirement::CanRedo => redo = true,
            Requirement::HasPath => has_path = true,
            Requirement::MultiplePages => pages = true,
            Requirement::MultipleFrames => frames = true,
            Requirement::Document => {}
        }
    }
    Box::new(move |s: &mut Session| {
        if pages {
            ex(s, "page.add", json!({}));
            ex(s, "page.setCurrent", json!({"index": 0}));
        }
        if frames {
            ex(s, "state.add", json!({}));
        }
        if has_path {
            let p = tmp().join(format!("haspath_{}.fw.png", std::process::id()));
            ex(s, "file.saveAs", json!({"path": p.to_string_lossy()}));
        }
        if undo || redo {
            rect_quick(s);
            if redo {
                ex(s, "edit.undo", json!({}));
            }
        }
        if clip {
            select(s, &[ellipse]);
            ex(s, "edit.copy", json!({}));
        }
        let mut target: Vec<ElementId> = match kind {
            Some(ObjectKind::Bitmap) => vec![bitmap],
            Some(ObjectKind::Slice) => vec![slice],
            Some(ObjectKind::Hotspot) | Some(ObjectKind::WebObject) => vec![hotspot],
            Some(ObjectKind::RectPrimitive) => vec![rect],
            Some(ObjectKind::Text) => vec![text],
            Some(ObjectKind::Path) | Some(ObjectKind::PathAttrs) => vec![ellipse],
            Some(ObjectKind::CompoundShape) => {
                select(s, &[rect, ellipse]);
                vec![first_id(&ex(s, "pi.compound.combine", json!({"op": "union"})))]
            }
            Some(ObjectKind::Group) => {
                select(s, &[rect, ellipse]);
                vec![first_id(&ex(s, "modify.group", json!({})))]
            }
            Some(ObjectKind::MaskGroup) => {
                select(s, &[rect, ellipse]);
                ex(s, "modify.mask.groupAsMask", json!({}));
                s.active().map(|d| d.selection.elements.clone()).unwrap_or_default()
            }
            Some(ObjectKind::Instance) | Some(ObjectKind::GraphicInstance) => {
                select(s, &[ellipse]);
                ex(s, "modify.symbol.convertToSymbol", json!({"name": "Sym"}));
                s.active().map(|d| d.selection.elements.clone()).unwrap_or_default()
            }
            Some(ObjectKind::ButtonInstance) => {
                select(s, &[ellipse]);
                ex(s, "modify.symbol.convertToSymbol", json!({"name": "Knopf", "kind": "button"}));
                s.active().map(|d| d.selection.elements.clone()).unwrap_or_default()
            }
            Some(ObjectKind::AnimationInstance) => {
                select(s, &[ellipse]);
                ex(s, "modify.symbol.convertToSymbol", json!({"name": "Anim", "kind": "animation"}));
                s.active().map(|d| d.selection.elements.clone()).unwrap_or_default()
            }
            None if min >= 3 => vec![rect, ellipse, rect_quick(s)],
            None if min == 2 => vec![rect, ellipse],
            None if sel || clip => vec![if clip { rect } else { ellipse }],
            None => vec![],
        };
        if min > target.len() && kind.is_some() {
            target.push(ellipse);
        }
        if pixels {
            select(s, &[bitmap]);
            ex(s, "select.pixels", json!({"shape": "rect", "rect": [202, 102, 212, 112]}));
        } else if !target.is_empty() {
            select(s, &target);
        }
        Value::Null
    })
}

fn rect_quick(s: &mut Session) -> ElementId {
    ElementId(ex(s, "draw.rectangle", json!({"rect": [250, 200, 290, 240], "fillColor": "#00aa00"}))["id"].as_u64().unwrap_or(0))
}

/// Vorgabewert für einen Pflichtparameter nach Typ.
fn default_param(spec: &fp_engine::ParamSpec, ids: &Ids) -> Value {
    use fp_engine::ParamKind as K;
    match &spec.kind {
        K::Bool => json!(true),
        K::Integer { min, max } => json!(min.unwrap_or(1).max(1).min(max.unwrap_or(i64::MAX))),
        K::Number { min, max } => json!(min.unwrap_or(1.0).max(1.0).min(max.unwrap_or(f64::MAX))),
        K::String => json!("Test"),
        K::Enum { values } => json!(values.first().cloned().unwrap_or_default()),
        K::Color => json!("#336699"),
        K::Point => json!([30, 30]),
        K::Rect => json!([20, 20, 80, 60]),
        K::Ids => json!([ids.ellipse.0]),
        K::Path => json!(tmp().join("datei.fw.png").to_string_lossy()),
        K::Json => json!({}),
    }
}

/// Zustand für den Wirkungsvergleich.
#[derive(Clone)]
struct Snap {
    docs: Vec<(u64, usize, String)>,
    active: Option<u64>,
    doc: Option<Arc<Document>>,
    sel: Option<fp_engine::Selection>,
    loc: Option<(usize, usize, Option<fp_model::LayerId>, bool)>,
    view: Option<fp_engine::ViewState>,
    undo: usize,
    session: String,
}

fn snap(s: &Session) -> Snap {
    let d = s.active();
    Snap {
        docs: s.documents().iter().map(|d| (d.id.0, Arc::as_ptr(&d.doc) as usize, d.title.clone())).collect(),
        active: s.active_id().map(|i| i.0),
        doc: d.map(|d| Arc::clone(&d.doc)),
        sel: d.map(|d| d.selection.clone()),
        loc: d.map(|d| (d.page, d.frame, d.layer, d.bitmap_mode)),
        view: d.map(|d| d.view.clone()),
        undo: d.map(|d| d.history.undo_count()).unwrap_or(0),
        session: format!("{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}", s.prefs, s.tools.active, s.tools.stroke_color, s.tools.fill_color, s.tools.options, s.ui, s.clipboard, s.last_filter.as_ref().map(|c| &c.command)),
    }
}

/// Inhalt eines Dokuments für den Vergleich: Modell als JSON (ohne Kachelpixel) plus Prüfsumme
/// aller Bitmap-Pixel.
fn fingerprint(doc: &Document) -> (String, u64) {
    use std::hash::{Hash, Hasher};
    let json = serde_json::to_string(doc).unwrap_or_default();
    let mut h = std::collections::hash_map::DefaultHasher::new();
    doc.walk_elements(|_, _, e| {
        if let fp_model::ElementKind::Bitmap(b) = &e.kind {
            b.image.to_rgba().0.hash(&mut h);
            if let Some(r) = &b.effect_result {
                r.bitmap.image.to_rgba().0.hash(&mut h);
            }
        }
    });
    (json, h.finish())
}

/// Hat sich der **Inhalt** geändert (nicht nur der Schnappschuss)?
fn doc_changed(a: &Snap, b: &Snap) -> bool {
    match (&a.doc, &b.doc) {
        (Some(x), Some(y)) => !Arc::ptr_eq(x, y) && fingerprint(x) != fingerprint(y),
        (None, None) => false,
        _ => true,
    }
}

/// Ganz leere Bitmap (0×0)? Solche verwirft das Original beim Speichern (re\22 §1, FIX-A `78d91bc`).
fn is_empty_bitmap(e: &fp_model::Element) -> bool {
    matches!(&e.kind, fp_model::ElementKind::Bitmap(b) if b.image.grid.width == 0 || b.image.grid.height == 0)
}

/// Struktur eines Dokuments für den Dateivergleich.
fn summary(doc: &Document) -> Vec<String> {
    summary_with(doc, false)
}

/// Struktur wie [`summary`]; mit `as_saved` ohne leere Bitmaps (so, wie die Datei sie enthalten muss).
fn summary_with(doc: &Document, as_saved: bool) -> Vec<String> {
    let mut v = vec![format!("pages {}", doc.pages.len()), format!("symbols {}", doc.symbols.len())];
    for (i, p) in doc.pages.iter().enumerate() {
        v.push(format!("page {i} frames {} canvas {}x{}", p.root.frame_count, p.canvas.width, p.canvas.height));
        for slot in &p.root.layers {
            if let LayerSlot::Own(l) = slot {
                for (ci, c) in l.cels.iter().enumerate() {
                    let kinds: Vec<&str> = c.elements.iter().filter(|e| !(as_saved && is_empty_bitmap(e))).map(|e| e.js_type()).collect();
                    if !kinds.is_empty() {
                        v.push(format!("  layer {} cel {ci}: {}", l.name, kinds.join(",")));
                    }
                }
            }
        }
    }
    v
}

fn file_roundtrip(s: &Session, name: &str) -> Result<(), String> {
    let Some(d) = s.active() else { return Ok(()) };
    let path = tmp().join(format!("{}.fw.png", name.replace('.', "_")));
    fp_engine::commands::save_document(s, &d.doc, d.page, &path).map_err(|e| format!("Speichern: {e}"))?;
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let back = fp_fwpng::read(&bytes).map_err(|e| format!("Lesen: {e}"))?;
    let _ = std::fs::remove_file(&path);
    // Erwartet: das Dokument ohne leere Bitmaps – die entfallen beim Speichern wie im Original,
    // das Dokument im Speicher behält sie.
    let (a, b) = (summary_with(&d.doc, true), summary(&back));
    if a != b {
        return Err(format!("Datei weicht ab:\n  vorher {a:?}\n  gelesen {b:?}"));
    }
    let mut empty_in_file = 0;
    back.walk_elements(|_, _, e| empty_in_file += usize::from(is_empty_bitmap(e)));
    if empty_in_file > 0 {
        return Err(format!("{empty_in_file} leere Bitmap(s) in der Datei, erwartet: beim Speichern entfallen"));
    }
    Ok(())
}

/// Prüft einen Befehl vollständig; liefert eine Fehlermeldung.
fn run_case(reg: &CommandRegistry, name: &str) -> Result<(), String> {
    let (mut s, ids) = fixture();
    let info = reg.get(name).map(|c| c.info().clone()).ok_or("nicht registriert")?;
    if !info.implemented {
        return Err("nicht umgesetzt".into());
    }
    let case = case_for(name, &info, &ids);
    let _ = (case.setup)(&mut s);
    let before = snap(&s);
    let out = s.execute(name, case.params.clone()).map_err(|e| format!("Fehler: {e} (Parameter {})", case.params))?;
    let after = snap(&s);
    // 1. Wirkung
    let changed_doc = doc_changed(&before, &after);
    let changed_sel = before.sel != after.sel;
    let changed_any = changed_doc
        || changed_sel
        || before.loc != after.loc
        || before.view != after.view
        || before.docs != after.docs
        || before.active != after.active
        || before.session != after.session
        || before.undo != after.undo;
    if !changed_any && out.value.is_null() && !case.no_effect_ok {
        return Err("keine Wirkung".into());
    }
    if out.effect == fp_engine::Effect::Document && !(changed_doc || changed_sel || before.loc != after.loc || before.view != after.view) && !case.no_effect_ok {
        return Err("Dokumentbefehl ohne Änderung".into());
    }
    if let Some(c) = &case.check {
        c(&s, &out.value).map_err(|e| format!("Ergebnis: {e}"))?;
    }
    // 2. Verlauf
    let same_doc = before.active == after.active && before.docs.len() == after.docs.len();
    if same_doc && after.undo > before.undo && !name.starts_with("edit.undo") && !name.starts_with("edit.redo") && !name.starts_with("history.") {
        if after.undo != before.undo + 1 {
            return Err(format!("{} Verlaufsschritte statt 1", after.undo - before.undo));
        }
        s.undo().map_err(|e| format!("Rückgängig: {e}"))?;
        let u = snap(&s);
        // Einfügen zurücknehmen hebt im Original die Objektauswahl auf (PM-1, `UNDO_CLEARS_SELECTION`).
        let mut want_sel = before.sel.clone();
        if fp_engine::history::UNDO_CLEARS_SELECTION.contains(&name)
            && let Some(w) = &mut want_sel
        {
            w.elements.clear();
            w.points.clear();
        }
        if !(u.doc.as_ref().zip(before.doc.as_ref()).is_some_and(|(x, y)| Arc::ptr_eq(x, y)) && u.sel == want_sel) {
            return Err("Rückgängig stellt den Zustand nicht exakt wieder her".into());
        }
        s.redo().map_err(|e| format!("Wiederholen: {e}"))?;
        let r = snap(&s);
        if !(r.doc.as_ref().zip(after.doc.as_ref()).is_some_and(|(x, y)| Arc::ptr_eq(x, y)) && r.sel == after.sel) {
            return Err("Wiederholen stellt den Zustand nicht exakt her".into());
        }
    } else if same_doc && changed_doc && !matches!(out.effect, fp_engine::Effect::Session | fp_engine::Effect::View | fp_engine::Effect::None) && !name.starts_with("edit.undo") && !name.starts_with("edit.redo") && !name.starts_with("history.") {
        return Err("Dokument geändert ohne Verlaufsschritt".into());
    }
    // 3. Datei
    if changed_doc {
        file_roundtrip(&s, name)?;
    }
    Ok(())
}

fn r(params: Value) -> Option<(Value, Option<Setup>)> {
    Some((params, None))
}

fn rs(params: Value, setup: Setup) -> Option<(Value, Option<Setup>)> {
    Some((params, Some(setup)))
}

fn sel_ids(v: Vec<ElementId>) -> Setup {
    Box::new(move |s: &mut Session| {
        select(s, &v);
        Value::Null
    })
}

/// Parameter und Vorbereitung je Befehl (wo die automatische Ableitung nicht reicht).
fn overrides(name: &str, ids: &Ids) -> Option<(Value, Option<Setup>)> {
    let (rect, ellipse, text, bitmap, slice) = (ids.rect, ids.ellipse, ids.text, ids.bitmap, ids.slice);
    let t = |n: &str| tmp().join(n).to_string_lossy().to_string();
    let saved = |s: &mut Session| {
        let p = tmp().join(format!("gespeichert_{}.fw.png", std::process::id()));
        ex(s, "file.saveAs", json!({"path": p.to_string_lossy()}));
        p
    };
    let _ = saved;
    match name {
        // ---------------------------------------------------------------- Datei
        "file.open" | "file.newFromTemplate" => rs(json!({"path": t("vorlage.fw.png")}), Box::new(move |s: &mut Session| {
            ex(s, "file.saveAs", json!({"path": t("vorlage.fw.png")}));
            Value::Null
        })),
        "file.save" => rs(json!({"path": t("save.fw.png")}), Box::new(|_| Value::Null)),
        "file.saveAs" | "file.saveAsTemplate" => r(json!({"path": t(&format!("{name}.fw.png"))})),
        "file.close" | "file.closeAll" => r(json!({"discard": true})),
        "file.import" => rs(json!({"path": t("import.fw.png"), "at": [5, 5]}), Box::new(move |s: &mut Session| {
            let d = s.doc().map(|d| Arc::clone(&d.doc)).ok();
            if let Some(d) = d {
                let _ = fp_engine::commands::save_document(s, &d, 0, &tmp().join("import.fw.png"));
            }
            Value::Null
        })),
        "file.export" => r(json!({"path": t("export_out"), "style": "imagesOnly", "settings": {"format": "png32"}})),
        "file.exportArea" => r(json!({"rect": [0, 0, 50, 50], "path": t("bereich.png"), "settings": {"format": "png24"}})),
        "file.updateHtml" => rs(json!({"path": t("upd/seite.htm")}), Box::new(move |_s: &mut Session| {
            let _ = std::fs::create_dir_all(tmp().join("upd"));
            let _ = std::fs::write(tmp().join("upd").join("seite.htm"), "<html></html>");
            Value::Null
        })),
        "file.batchProcess" => rs(json!({"files": [t("batch_in.fw.png")], "outDir": t("batch_out"), "commands": [{"name": "select.all"}, {"name": "modify.transform.flipHorizontal"}], "settings": {"format": "jpeg"}}), Box::new(|s: &mut Session| {
            let d = s.doc().map(|d| Arc::clone(&d.doc)).ok();
            if let Some(d) = d {
                let _ = fp_engine::commands::save_document(s, &d, 0, &tmp().join("batch_in.fw.png"));
            }
            Value::Null
        })),
        // Batch-Studio (fp-batch über den Haken BatchHost): Quelldatei = das Testdokument
        "batch.run" | "batch.plan" | "batch.preview" => {
            let src = t("batch_src.fw.png");
            let out = t(&format!("{}_out", name.replace('.', "_")));
            let _ = std::fs::remove_dir_all(&out);
            let recipe = json!({"output": {"format": "png"}});
            let params = match name {
                "batch.preview" => json!({"recipe": recipe, "file": src, "maxSize": 64}),
                _ => json!({"recipe": recipe, "files": [src], "out": out}),
            };
            rs(params, Box::new(|s: &mut Session| {
                let d = s.doc().map(|d| Arc::clone(&d.doc)).ok();
                if let Some(d) = d {
                    let _ = fp_engine::commands::save_document(s, &d, 0, &tmp().join("batch_src.fw.png"));
                }
                Value::Null
            }))
        }
        "file.print" => rs(json!({"open": false, "outDir": t("druck")}), Box::new(|s: &mut Session| {
            ex(s, "file.pageSetup", json!({"orientation": "landscape", "fit": true}));
            Value::Null
        })),
        "file.pageSetup" => r(json!({"orientation": "landscape", "fit": true})),
        "file.twainAcquire" => rs(json!({"file": t("scan_in.png")}), Box::new(|s: &mut Session| {
            ex(s, "file.exportArea", json!({"rect": [0, 0, 40, 40], "path": tmp().join("scan_in.png").to_string_lossy(), "settings": {"format": "png32"}}));
            Value::Null
        })),
        "file.twainSelect" => r(json!({"device": "{6BDD1FC6-810F-11D0-BEC7-08002BE2092F}\\0000"})),
        "file.browseInBridge" => r(json!({"folder": tmp().to_string_lossy(), "open": false})),
        "file.openRecent" => rs(json!({"index": 0}), Box::new(move |s: &mut Session| {
            ex(s, "file.saveAs", json!({"path": t("recent.fw.png")}));
            Value::Null
        })),
        "file.previewInBrowser.setPrimaryBrowser" | "file.previewInBrowser.setSecondaryBrowser" => r(json!({"path": "browser.exe"})),
        n if n.starts_with("file.previewInBrowser.preview") || n.starts_with("file.sendToEMail.") => r(json!({"open": false})),
        "file.imagePreview" => r(json!({"settings": {"format": "jpeg", "quality": 60}, "apply": true})),
        "file.exportWizard" => r(json!({"apply": true})),
        "file.htmlSetup" => r(json!({"settings": {"images_dir": "bilder/"}})),
        "edit.preferences" => r(json!({"key": "MaxUndos", "value": 30})),
        "edit.keyboardShortcuts" => r(json!({"set": "Photoshop"})),
        // ---------------------------------------------------------------- Befehle/Hilfe/Fenster
        "commands.manageSavedCommands" => r(json!({"action": "save", "name": "Testbefehl", "code": "fp.run(\"select.all\", {});"})),
        "commands.run" => rs(json!({"name": "Testbefehl2"}), Box::new(|_| {
            let _ = std::fs::create_dir_all(fp_engine::commands::misc::commands_dir());
            let _ = std::fs::write(fp_engine::commands::misc::commands_dir().join("Testbefehl2.jsf"), "select.all");
            Value::Null
        })),
        "commands.runScript" => r(json!({"code": "select.all"})),
        n if n.starts_with("help.") && n != "help.about" => r(json!({"open": false})),
        n if n.contains("anageExtensions") => r(json!({"action": "list"})),
        // ---------------------------------------------------------------- Text
        "text.font" => rs(json!({"font": "Times New Roman"}), sel_ids(vec![text])),
        "text.setText" => rs(json!({"text": "Neu"}), sel_ids(vec![text])),
        "text.editor" => rs(json!({"text": "Geändert", "size": 30}), sel_ids(vec![text])),
        "text.setProperties" => rs(json!({"frameType": "fixed", "width": 200, "antialias": "crisp", "autoKern": false, "kerning": 10, "orientation": "verticalRtl"}), sel_ids(vec![text])),
        "text.attachToPath" | "text.attachInPath" => rs(json!({}), sel_ids(vec![text, ellipse])),
        "text.detachFromPath" | "text.reverseDirection" => rs(json!({}), Box::new(move |s: &mut Session| {
            select(s, &[text, ellipse]);
            ex(s, "text.attachToPath", json!({}));
            Value::Null
        })),
        n if n.starts_with("text.orientation") => {
            let rot = n.ends_with("rotateAroundPath");
            rs(json!({}), Box::new(move |s: &mut Session| {
            select(s, &[text, ellipse]);
            ex(s, "text.attachToPath", json!({}));
            if rot {
                ex(s, "text.orientation.vertical", json!({}));
            }
            Value::Null
        }))
        }
        "text.checkSpelling" => rs(json!({"replace": [{"word": "Welt", "with": "Wald"}]}), sel_ids(vec![text])),
        "text.spellingSetup" => r(json!({"addWords": ["Firepixel"], "language": "de_DE"})),
        // ---------------------------------------------------------------- Auswahl
        "select.set" => r(json!({"ids": [ellipse.0]})),
        "select.inRect" => r(json!({"rect": [0, 0, 100, 100]})),
        "select.pointAt" => r(json!({"point": [120, 50]})),
        "select.points" => r(json!({"points": [{"element": ellipse.0, "contour": 0, "node": 0}]})),
        "select.pixels" => r(json!({"shape": "rect", "rect": [0, 0, 10, 10]})),
        "select.magicWand" => rs(json!({"point": [205, 105]}), sel_ids(vec![bitmap])),
        // PAR-X-TOOLS-2: Rahmen weich machen bzw. verschieben, Auswahlpixel verschieben (B1 mit Rahmen)
        "select.liveEdge" | "select.moveMarquee" | "select.movePixels" => {
            let p = if name == "select.liveEdge" { json!({"feather": 3}) } else { json!({"dx": 2, "dy": 1}) };
            rs(p, Box::new(move |s: &mut Session| {
                select(s, &[bitmap]);
                ex(s, "select.pixels", json!({"shape": "rect", "rect": [202, 102, 212, 112]}));
                Value::Null
            }))
        }
        // KI-Auswahl (Agent KI): Klickpunkt auf der Bitmap; ohne Modelle meldet der Befehl das fehlende Modell
        "select.aiObject" => rs(json!({"point": [205, 105]}), sel_ids(vec![bitmap])),
        "select.restoreBitmapSelection" | "select.deleteBitmapSelection" => rs(json!({"name": "A"}), Box::new(move |s: &mut Session| {
            select(s, &[bitmap]);
            ex(s, "select.pixels", json!({"shape": "rect", "rect": [200, 100, 205, 105]}));
            ex(s, "select.saveBitmapSelection", json!({"name": "A"}));
            ex(s, "select.none", json!({}));
            Value::Null
        })),
        "select.subselect" | "select.superselect" => {
            let sup = name == "select.superselect";
            rs(json!({}), Box::new(move |s: &mut Session| {
            select(s, &[rect, ellipse]);
            let g = first_id(&ex(s, "modify.group", json!({})));
            if sup {
                select(s, &[ellipse]);
            } else {
                select(s, &[g]);
            }
            Value::Null
        }))
        }
        // ---------------------------------------------------------------- Bearbeiten
        "edit.move" => rs(json!({"dx": 5, "dy": 7}), sel_ids(vec![ellipse])),
        "edit.paste" | "edit.pasteAsMask" | "edit.pasteInside" | "edit.pasteAttributes" => rs(json!({}), Box::new(move |s: &mut Session| {
            select(s, &[ellipse]);
            ex(s, "edit.copy", json!({}));
            select(s, &[rect]);
            Value::Null
        })),
        "edit.insert.bitmapViaCopy" | "edit.insert.bitmapViaCut" | "edit.cropSelectedBitmap" => rs(json!({}), Box::new(move |s: &mut Session| {
            select(s, &[bitmap]);
            ex(s, "select.pixels", json!({"shape": "rect", "rect": [202, 102, 212, 112]}));
            Value::Null
        })),
        "edit.cropDocument" => r(json!({"rect": [0, 0, 200, 150]})),
        "edit.insert.hotspot" | "edit.insert.slice" | "edit.insert.polygonSlice" => rs(json!({}), sel_ids(vec![ellipse])),
        // ---------------------------------------------------------------- Ansicht
        "view.setZoom" => r(json!({"zoom": 2})),
        "view.zoomToRect" => r(json!({"rect": [0, 0, 50, 50]})),
        "view.pan" => r(json!({"dx": 10, "dy": 10})),
        "view.setViewport" => r(json!({"width": 640, "height": 480})),
        "guide.add" => r(json!({"orientation": "horizontal", "position": 5})),
        "guide.move" | "guide.remove" | "guide.delete" => rs(json!({"orientation": "horizontal", "index": 0, "position": 9}), Box::new(|s: &mut Session| {
            ex(s, "guide.add", json!({"orientation": "horizontal", "position": 5}));
            Value::Null
        })),
        // ---------------------------------------------------------------- Modifizieren
        "modify.canvas.canvasSize" => r(json!({"width": 500, "height": 350})),
        "modify.canvas.canvasColor" => r(json!({"color": "#00ff00"})),
        "modify.canvas.imageSize" => r(json!({"percent": 150})),
        "modify.animation.settings" | "modify.animation.removeAnimation" => rs(json!({"frames": 3}), Box::new(move |s: &mut Session| {
            select(s, &[ellipse]);
            ex(s, "modify.symbol.convertToSymbol", json!({"name": "A", "kind": "animation"}));
            Value::Null
        })),
        "modify.symbol.tweenInstances" => rs(json!({"steps": 2}), Box::new(move |s: &mut Session| {
            select(s, &[ellipse]);
            ex(s, "modify.symbol.convertToSymbol", json!({"name": "S"}));
            ex(s, "edit.duplicate", json!({}));
            ex(s, "edit.move", json!({"dx": 100, "dy": 0}));
            let insts: Vec<ElementId> = top_elements(s).iter().filter(|e| e.js_type() == "Instance").map(|e| e.id).collect();
            select(s, &insts);
            Value::Null
        })),
        "modify.symbol.swapSymbol" => rs(json!({"symbol": "S2"}), Box::new(move |s: &mut Session| {
            select(s, &[rect]);
            ex(s, "modify.symbol.convertToSymbol", json!({"name": "S2"}));
            select(s, &[ellipse]);
            ex(s, "modify.symbol.convertToSymbol", json!({"name": "S"}));
            Value::Null
        })),
        "modify.symbol.editSymbol" | "modify.symbol.editInPlace" => rs(json!({}), Box::new(move |s: &mut Session| {
            select(s, &[ellipse]);
            ex(s, "modify.symbol.convertToSymbol", json!({"name": "S"}));
            Value::Null
        })),
        "modify.symbol.exitEdit" => rs(json!({}), Box::new(move |s: &mut Session| {
            select(s, &[ellipse]);
            ex(s, "modify.symbol.convertToSymbol", json!({"name": "S"}));
            ex(s, "modify.symbol.editSymbol", json!({}));
            ex(s, "select.all", json!({}));
            ex(s, "modify.transform.flipHorizontal", json!({}));
            Value::Null
        })),
        "modify.popUpMenu.addPopUpMenu" => rs(json!({"items": [{"text": "Start", "url": "index.htm"}, {"text": "Mehr", "url": "", "submenu": [{"text": "A", "url": "a.htm"}]}], "menu": {"font": "Arial", "size": 12.0}}), sel_ids(vec![slice])),
        "modify.popUpMenu.editPopUpMenu" | "modify.popUpMenu.deletePopUpMenu" => rs(json!({"items": [{"text": "Neu", "url": "neu.htm"}]}), Box::new(move |s: &mut Session| {
            select(s, &[slice]);
            ex(s, "modify.popUpMenu.addPopUpMenu", json!({"items": [{"text": "Start", "url": "index.htm"}]}));
            Value::Null
        })),
        "modify.selectiveJpeg.settings" => r(json!({"enabled": true, "quality": 30, "preserveText": true})),
        "modify.selectiveJpeg.saveSelectionAsJpegMask" => rs(json!({}), Box::new(move |s: &mut Session| {
            ex(s, "select.pixels", json!({"shape": "rect", "rect": [0, 0, 5, 5]}));
            Value::Null
        })),
        "modify.selectiveJpeg.restoreJpegMaskAsSelection" | "modify.selectiveJpeg.removeJpegMask" => rs(json!({}), Box::new(|s: &mut Session| {
            ex(s, "select.pixels", json!({"shape": "rect", "rect": [0, 0, 5, 5]}));
            ex(s, "modify.selectiveJpeg.saveSelectionAsJpegMask", json!({}));
            ex(s, "select.none", json!({}));
            Value::Null
        })),
        "modify.mask.revealSelection" | "modify.mask.hideSelection" => rs(json!({}), Box::new(move |s: &mut Session| {
            select(s, &[bitmap]);
            ex(s, "select.pixels", json!({"shape": "rect", "rect": [202, 102, 212, 112]}));
            Value::Null
        })),
        "modify.mask.revealAll" | "modify.mask.hideAll" => rs(json!({}), sel_ids(vec![bitmap])),
        "modify.mask.disableMask" | "modify.mask.deleteMask" | "modify.mask.applyMask" | "modify.mask.link" => rs(json!({}), Box::new(move |s: &mut Session| {
            select(s, &[rect, ellipse]);
            ex(s, "modify.mask.groupAsMask", json!({}));
            Value::Null
        })),
        // T-6: Gruppe als Maske zeigt Füllung und Strich nicht (0x07) – einschalten ändert das Dokument
        "modify.mask.options" => rs(json!({"showAttrs": true}), Box::new(move |s: &mut Session| {
            select(s, &[rect, ellipse]);
            ex(s, "modify.mask.groupAsMask", json!({}));
            Value::Null
        })),
        "modify.flattenSelection" => rs(json!({}), Box::new(move |s: &mut Session| {
            select(s, &[bitmap]);
            Value::Null
        })),
        "modify.exitBitmapMode" => rs(json!({}), Box::new(move |s: &mut Session| {
            select(s, &[bitmap]);
            ex(s, "modify.editBitmap", json!({}));
            Value::Null
        })),
        "modify.editBitmap" => rs(json!({}), sel_ids(vec![bitmap])),
        "modify.ungroup" => rs(json!({}), Box::new(move |s: &mut Session| {
            select(s, &[rect, ellipse]);
            ex(s, "modify.group", json!({}));
            Value::Null
        })),
        "modify.align.distributeWidths" | "modify.align.distributeHeights" => rs(json!({}), Box::new(move |s: &mut Session| {
            let x = rect_quick(s);
            select(s, &[rect, ellipse, x]);
            Value::Null
        })),
        "modify.combinePaths.split" => rs(json!({}), Box::new(move |s: &mut Session| {
            select(s, &[rect, ellipse]);
            ex(s, "modify.combinePaths.join", json!({}));
            Value::Null
        })),
        "transform.matrix" => rs(json!({"matrix": [1, 0, 0, 1, 3, 3]}), sel_ids(vec![ellipse])),
        "transform.distort" => rs(json!({"rect": [100, 20, 160, 80], "quad": [[100, 20], [160, 20], [170, 90], [100, 80]]}), sel_ids(vec![ellipse])),
        "transform.nineSlice" => rs(json!({"rect": [100, 20, 160, 80], "to": [90, 10, 200, 120]}), sel_ids(vec![ellipse])),
        // ---------------------------------------------------------------- Pfade und Bitmaps
        "path.movePoints" | "path.deletePoints" => rs(json!({"dx": 1, "dy": 1}), Box::new(move |s: &mut Session| {
            select(s, &[ellipse]);
            ex(s, "select.points", json!({"points": [{"element": ellipse.0, "contour": 0, "node": 0}]}));
            Value::Null
        })),
        "path.setContours" => rs(json!({"contours": [{"closed": false, "nodes": [[0, 0], [5, 5]]}]}), sel_ids(vec![ellipse])),
        "path.insertNode" => r(json!({"id": ellipse.0, "contour": 0, "segment": 0, "t": 0.5})),
        "path.removeNode" => r(json!({"id": ellipse.0, "contour": 0, "node": 0})),
        "path.knife" => rs(json!({"points": [[130, 0], [130, 100]]}), sel_ids(vec![ellipse])),
        "path.scrub" => rs(json!({"points": [[100, 50], [160, 50]], "amount": -0.5}), sel_ids(vec![ellipse])),
        "bitmap.retouch" => rs(json!({"mode": "blur", "points": [[201, 101], [210, 110]]}), sel_ids(vec![bitmap])),
        "bitmap.paint" | "bitmap.erase" => rs(json!({"points": [[201, 101], [210, 110]], "size": 3}), sel_ids(vec![bitmap])),
        "bitmap.floodFill" => rs(json!({"point": [205, 105], "color": "#00ff00"}), sel_ids(vec![bitmap])),
        "bitmap.gradient" => rs(json!({"from": [200, 100], "to": [220, 100]}), sel_ids(vec![bitmap])),
        // ---------------------------------------------------------------- Zeichnen und Attribute
        "draw.rectangle" | "draw.ellipse" => r(json!({"rect": [5, 5, 50, 50]})),
        "draw.polygon" => r(json!({"center": [50, 50], "corner": [60, 50], "sides": 6})),
        "draw.line" => r(json!({"from": [0, 0], "to": [50, 50]})),
        "draw.path" => r(json!({"contours": [{"closed": true, "nodes": [[0, 0], [50, 0], [50, 50]]}]})),
        "draw.text" => r(json!({"text": "x", "anchor": [5, 5]})),
        "web.addHotspot" | "web.addSlice" => r(json!({"shape": "rect", "rect": [0, 0, 50, 50]})),
        "element.setOpacity" => rs(json!({"opacity": 40}), sel_ids(vec![ellipse])),
        "element.setBlendMode" => rs(json!({"mode": "screen"}), sel_ids(vec![ellipse])),
        "element.setName" => rs(json!({"name": "neu"}), sel_ids(vec![ellipse])),
        "element.setVisible" => rs(json!({"visible": false}), sel_ids(vec![ellipse])),
        "element.setLocked" => rs(json!({"locked": true}), sel_ids(vec![ellipse])),
        "element.addEffect" => rs(json!({"effect": "dropShadow"}), sel_ids(vec![rect])),
        "element.setEffect" => rs(json!({"effect": "glow", "params": {"GlowWidth": "8"}}), sel_ids(vec![rect])),
        "element.removeEffect" | "element.setEffectVisible" => rs(json!({"index": 0, "visible": false}), Box::new(move |s: &mut Session| {
            select(s, &[rect]);
            ex(s, "element.addEffect", json!({"effect": "dropShadow"}));
            Value::Null
        })),
        "element.setGradient" => rs(json!({"from": [100, 20], "to": [160, 80]}), sel_ids(vec![ellipse])),
        "element.setFill" => rs(json!({"color": "#010203"}), sel_ids(vec![ellipse])),
        "element.setStroke" => rs(json!({"color": "#010203", "width": 4}), sel_ids(vec![ellipse])),
        "element.setBounds" => r(json!({"ids": [ellipse.0], "x": 10, "y": 200, "width": 30})),
        "element.setFillHandles" => rs(json!({"start": [100, 20], "end": [160, 20]}), Box::new(move |s: &mut Session| {
            select(s, &[ellipse]);
            ex(s, "element.setGradient", json!({"from": [100, 20], "to": [160, 80]}));
            Value::Null
        })),
        "element.setCustomData" => r(json!({"ids": [ellipse.0], "data": {"a": 1, "b": "x", "c": [1, 2]}})),
        "element.setPattern" => rs(json!({"ids": [ellipse.0], "image": t("muster.png")}), Box::new(|s: &mut Session| {
            ex(s, "file.exportArea", json!({"rect": [0, 0, 16, 16], "path": tmp().join("muster.png").to_string_lossy(), "settings": {"format": "png32"}}));
            Value::Null
        })),
        "element.setTexture" => rs(json!({"ids": [ellipse.0], "image": t("textur.png"), "amount": 50}), Box::new(|s: &mut Session| {
            ex(s, "file.exportArea", json!({"rect": [0, 0, 16, 16], "path": tmp().join("textur.png").to_string_lossy(), "settings": {"format": "png32"}}));
            Value::Null
        })),
        "element.setBrushSensitivity" => r(json!({"ids": [rect.0], "values": {"PZ": 50, "SO": 20}})),
        "layer.rename" => r(json!({"name": "x"})),
        "layer.setVisible" => r(json!({"visible": false})),
        "layer.setLocked" => r(json!({"locked": true})),
        "layer.setShared" => r(json!({"shared": true})),
        "layer.setExpanded" => r(json!({"expanded": false})),
        "layer.move" => rs(json!({"index": 0}), Box::new(|s: &mut Session| {
            ex(s, "layer.add", json!({}));
            Value::Null
        })),
        "layer.delete" => rs(json!({}), Box::new(|s: &mut Session| {
            ex(s, "layer.add", json!({}));
            Value::Null
        })),
        "page.delete" => rs(json!({}), Box::new(|s: &mut Session| {
            ex(s, "page.add", json!({}));
            Value::Null
        })),
        "page.rename" => r(json!({"name": "p"})),
        // CT-C-02/03 (CF-C): Umordnen per Ziehen, zweite Seite bzw. zweiter Status nach vorn
        "page.move" => rs(json!({"page": 1, "index": 0}), Box::new(|s: &mut Session| {
            ex(s, "page.add", json!({}));
            Value::Null
        })),
        "state.move" => rs(json!({"state": 1, "index": 0}), Box::new(|s: &mut Session| {
            ex(s, "state.add", json!({}));
            Value::Null
        })),
        "page.setCurrent" => rs(json!({"index": 0}), Box::new(|s: &mut Session| {
            ex(s, "page.add", json!({}));
            Value::Null
        })),
        "state.setCurrent" => rs(json!({"index": 0}), Box::new(|s: &mut Session| {
            ex(s, "state.add", json!({}));
            Value::Null
        })),
        "color.sample" => r(json!({"point": [120, 50]})),
        "color.set" => r(json!({"fill": "#123456", "stroke": "#654321"})),
        "tool.select" => r(json!({"tool": "tool.pen"})),
        "document.setOrigin" => r(json!({"left": 10, "top": 20})),
        "filter.repeat" => rs(json!({}), Box::new(move |s: &mut Session| {
            select(s, &[bitmap]);
            ex(s, "filter.adjustColor.invert", json!({}));
            Value::Null
        })),
        "history.goTo" => rs(json!({"position": 0}), Box::new(|s: &mut Session| {
            rect_quick(s);
            Value::Null
        })),
        "history.replay" => r(json!({"commands": [{"command": "select.all", "params": {}}, {"command": "modify.transform.flipHorizontal", "params": {}}]})),
        n if n.starts_with("filter.") => rs(json!({}), sel_ids(vec![bitmap])),
        "autoShape.insert" => r(json!({"shape": "Pfeil", "rect": [10, 10, 50, 50]})),
        "file.saveCopyAs" => r(json!({"path": t("kopie.fw.png")})),
        "edit.redo" => rs(json!({}), Box::new(|s: &mut Session| {
            rect_quick(s);
            ex(s, "edit.undo", json!({}));
            Value::Null
        })),
        "select.feather" => rs(json!({"radius": 3}), Box::new(move |s: &mut Session| {
            select(s, &[bitmap]);
            ex(s, "select.pixels", json!({"shape": "rect", "rect": [202, 102, 212, 112]}));
            Value::Null
        })),
        "commands.symbolScript" => rs(json!({}), Box::new(move |s: &mut Session| {
            select(s, &[ellipse]);
            ex(s, "modify.symbol.convertToSymbol", json!({"name": "S"}));
            Value::Null
        })),
        "modify.symbol.breakApart" => rs(json!({}), Box::new(move |s: &mut Session| {
            select(s, &[ellipse]);
            ex(s, "modify.symbol.convertToSymbol", json!({"name": "S"}));
            Value::Null
        })),
        "modify.mergeDown" => rs(json!({}), Box::new(|s: &mut Session| {
            // Rechteck direkt über der Bitmap
            let id = rect_quick(s);
            select(s, &[id]);
            Value::Null
        })),
        "modify.alterPath.simplify" | "modify.alterPath.expandStroke" | "modify.alterPath.insetPath" => rs(json!({}), sel_ids(vec![ellipse])),
        "text.size.larger" | "text.size.smaller" => rs(json!({}), sel_ids(vec![text])),
        // only pixel selections are inverted (TW-133)
        "select.inverse" => rs(json!({}), Box::new(|s: &mut Session| {
            ex(s, "select.pixels", json!({"shape": "rect", "rect": [0, 0, 5, 5]}));
            Value::Null
        })),
        // "stretched" needs a fixed-width frame; point text keeps its alignment (TW-133)
        "text.align.stretched" => rs(json!({}), Box::new(move |s: &mut Session| {
            select(s, &[text]);
            ex(s, "text.setProperties", json!({"frameType": "fixed", "width": 200}));
            Value::Null
        })),
        n if n.starts_with("text.size.") || n.starts_with("text.style.") && n != "text.style.plain" || n.starts_with("text.align.") && n != "text.align.left" => {
            rs(json!({"size": 30}), sel_ids(vec![text]))
        }
        "pi.layer.setActiveShared" => rs(json!({"layer": ids.layer}), Box::new(|s: &mut Session| {
            ex(s, "page.add", json!({}));
            ex(s, "page.setCurrent", json!({"index": 0}));
            ex(s, "panel.layers.shareToAllPages", json!({}));
            ex(s, "page.setCurrent", json!({"index": 1}));
            ex(s, "layer.add", json!({}));
            Value::Null
        })),
        "window.answers" => r(json!({"open": false})),
        // ---------------------------------------------------------------- Zustand vorher so, dass der Befehl etwas ändert
        "view.showAll" => rs(json!({}), Box::new(move |s: &mut Session| {
            select(s, &[ellipse]);
            ex(s, "element.setVisible", json!({"visible": false}));
            ex(s, "select.none", json!({}));
            Value::Null
        })),
        "view.guides.editGuides" => r(json!({"color": "#ff0000", "locked": true})),
        "view.guides.clearGuides" => rs(json!({}), Box::new(|s: &mut Session| {
            ex(s, "guide.add", json!({"orientation": "vertical", "position": 40}));
            Value::Null
        })),
        "modify.canvas.fitCanvas" => rs(json!({}), Box::new(|s: &mut Session| {
            ex(s, "modify.canvas.canvasSize", json!({"width": 600, "height": 500}));
            Value::Null
        })),
        "modify.flattenLayers" => rs(json!({}), Box::new(|s: &mut Session| {
            ex(s, "layer.add", json!({}));
            rect_quick(s);
            Value::Null
        })),
        "modify.transform.numericTransform" => rs(json!({"mode": "scale", "scaleX": 50, "scaleY": 50}), sel_ids(vec![ellipse])),
        "modify.transform.removeTransformations" => rs(json!({}), Box::new(move |s: &mut Session| {
            select(s, &[ellipse]);
            ex(s, "modify.symbol.convertToSymbol", json!({"name": "S"}));
            ex(s, "modify.transform.rotate90DegreeCw", json!({}));
            Value::Null
        })),
        "modify.alterPath.antiAliasFill" => rs(json!({}), Box::new(move |s: &mut Session| {
            select(s, &[ellipse]);
            ex(s, "pi.fill", json!({"edge": 0}));
            Value::Null
        })),
        "modify.snapToPixel" => rs(json!({}), Box::new(|s: &mut Session| {
            let id = first_id(&ex(s, "draw.rectangle", json!({"rect": [10.3, 10.6, 50.2, 40.7]})));
            select(s, &[id]);
            Value::Null
        })),
        "text.style.plain" => rs(json!({}), Box::new(move |s: &mut Session| {
            select(s, &[text]);
            ex(s, "text.style.fauxBold", json!({}));
            Value::Null
        })),
        "text.align.left" => rs(json!({}), Box::new(move |s: &mut Session| {
            select(s, &[text]);
            ex(s, "text.align.right", json!({}));
            Value::Null
        })),
        "pi.compound.combine" => rs(json!({}), Box::new(move |s: &mut Session| {
            select(s, &[rect, ellipse]);
            ex(s, "pi.compound.setOp", json!({"op": 1}));
            Value::Null
        })),
        "pi.state.copyTo" => rs(json!({"from": 1, "to": 1}), Box::new(move |s: &mut Session| {
            ex(s, "state.add", json!({}));
            ex(s, "state.setCurrent", json!({"index": 0}));
            select(s, &[ellipse]);
            Value::Null
        })),
        "pi.guides" => r(json!({"color": "#00ff00", "show": false, "snap": true})),
        "panel.layers.showAll" => rs(json!({}), Box::new(|s: &mut Session| {
            ex(s, "layer.setVisible", json!({"visible": false}));
            Value::Null
        })),
        "panel.layers.unlockAll" => rs(json!({}), Box::new(|s: &mut Session| {
            ex(s, "layer.setLocked", json!({"locked": true}));
            Value::Null
        })),
        "panel.optimize.exportWizard" => r(json!({"kb": 2})),
        "panel.optimize.editColor" => rs(json!({"color": "#123456", "newColor": "#654321"}), Box::new(|s: &mut Session| {
            ex(s, "panel.optimize.addColor", json!({"color": "#123456"}));
            Value::Null
        })),
        "pi.brush.save" => rs(json!({"name": "Test"}), Box::new(move |s: &mut Session| {
            ex(s, "select.set", json!({"ids": [rect.0]}));
            ex(s, "pi.stroke", json!({"width": 3, "color": "#000000"}));
            Value::Null
        })),
        "pi.brush.delete" => rs(json!({"name": "Test"}), Box::new(move |s: &mut Session| {
            ex(s, "select.set", json!({"ids": [rect.0]}));
            ex(s, "pi.stroke", json!({"width": 3, "color": "#000000"}));
            ex(s, "pi.brush.save", json!({"name": "Test"}));
            Value::Null
        })),
        "panel.optimize.rebuildPalette" => rs(json!({}), Box::new(|s: &mut Session| {
            ex(s, "panel.optimize.addColor", json!({"color": "#123456"}));
            Value::Null
        })),
        "panel.optimize.unlockAll" | "panel.optimize.removeAllEdits" => {
            let lock = name == "panel.optimize.unlockAll";
            rs(json!({}), Box::new(move |s: &mut Session| {
                ex(s, "panel.optimize.addColor", json!({"color": "#123456"}));
                if lock {
                    ex(s, "panel.optimize.lockColors", json!({"colors": ["#123456"], "on": true}));
                } else {
                    ex(s, "panel.optimize.editColor", json!({"color": "#123456", "newColor": "#654321"}));
                }
                Value::Null
            }))
        }
        "panel.effects.locatePlugins" => r(json!({"path": tmp().to_string_lossy()})),
        n if n.starts_with("autoShape.") => r(json!({"id": ellipse.0, "index": 0, "from": [100, 20], "to": [110, 30], "params": {"a": 1}})),
        // ---------------------------------------------------------------- Ansicht/Auswahl (Zustand vorher anders)
        "view.magnification.100" => rs(json!({}), Box::new(|s: &mut Session| {
            ex(s, "view.setZoom", json!({"zoom": 2}));
            Value::Null
        })),
        "view.grid.editGrid" => r(json!({"width": 25, "height": 30, "color": "#ff0000"})),
        "view.state.next" | "view.state.previous" => rs(json!({}), Box::new(|s: &mut Session| {
            ex(s, "state.add", json!({}));
            Value::Null
        })),
        "select.none" => rs(json!({}), sel_ids(vec![ellipse])),
        // Auswahlverlauf (Q-21): zwei Auswahlen, für „Nächste“ vorher einen Schritt zurück
        "select.previous" | "select.next" => {
            let next = name == "select.next";
            rs(json!({}), Box::new(move |s: &mut Session| {
                select(s, &[ellipse]);
                select(s, &[bitmap]);
                if next {
                    ex(s, "select.previous", json!({}));
                }
                Value::Null
            }))
        }
        "select.similar" => rs(json!({}), Box::new(move |s: &mut Session| {
            select(s, &[bitmap]);
            ex(s, "select.pixels", json!({"shape": "rect", "rect": [202, 102, 204, 104]}));
            Value::Null
        })),
        "window.closeDuplicate" => rs(json!({}), Box::new(|s: &mut Session| {
            ex(s, "window.duplicateWindow", json!({}));
            Value::Null
        })),
        "layer.setActive" => rs(json!({"layer": ids.layer}), Box::new(|s: &mut Session| {
            ex(s, "layer.add", json!({}));
            Value::Null
        })),
        "modify.combinePaths.join" => rs(json!({}), Box::new(move |s: &mut Session| {
            let a = first_id(&ex(s, "draw.line", json!({"from": [10, 250], "to": [60, 250]})));
            let b = first_id(&ex(s, "draw.line", json!({"from": [60, 250], "to": [60, 290]})));
            select(s, &[a, b]);
            Value::Null
        })),
        "modify.combinePaths.union" | "modify.combinePaths.intersect" | "modify.combinePaths.punch" | "modify.combinePaths.crop" => rs(json!({}), Box::new(move |s: &mut Session| {
            let a = first_id(&ex(s, "draw.ellipse", json!({"rect": [120, 40, 200, 100]})));
            select(s, &[ellipse, a]);
            Value::Null
        })),
        "text.setAttributes" => rs(json!({"size": 30, "bold": true, "color": "#ff0000"}), sel_ids(vec![text])),
        "tool.pick" => r(json!({"command": "select.set", "params": {"ids": [rect.0]}})),
        "tool.shape" => r(json!({"draw": "draw.rectangle", "params": {"rect": [0, 0, 10, 10]}, "fill": {"category": "gradient"}, "stroke": {"width": 3}})),
        // ---------------------------------------------------------------- Inspektor (pi.*)
        "pi.fill" => rs(json!({"category": "gradient", "shape": 3}), sel_ids(vec![rect])),
        "pi.stroke" => rs(json!({"width": 5, "color": "#ff00ff", "placement": "outside"}), sel_ids(vec![rect])),
        "pi.brush" => rs(json!({"diameter": 8, "softness": 20}), sel_ids(vec![rect])),
        "pi.roundness" => rs(json!({"value": 30}), sel_ids(vec![rect])),
        "pi.effect.set" | "pi.effects.visible" | "pi.effects.replace" => rs(json!({"index": 0, "params": {"ShadowAngle": "45"}, "visible": false, "effects": []}), Box::new(move |s: &mut Session| {
            select(s, &[rect]);
            ex(s, "element.addEffect", json!({"effect": "dropShadow"}));
            Value::Null
        })),
        "pi.effect.move" => rs(json!({"index": 0, "to": 1}), Box::new(move |s: &mut Session| {
            select(s, &[rect]);
            ex(s, "element.addEffect", json!({"effect": "dropShadow"}));
            ex(s, "element.addEffect", json!({"effect": "innerGlow"}));
            Value::Null
        })),
        "pi.compound.setOp" => rs(json!({"op": 2}), sel_ids(vec![rect, ellipse])),
        // Kontextmenü „Pfad kombinieren“ (LW)
        "path.combine.union" | "path.combine.intersect" | "path.combine.punch" | "path.combine.crop" | "path.combine.normal" => rs(json!({}), sel_ids(vec![rect, ellipse])),
        "pi.text.antialias" => rs(json!({"mode": 2}), sel_ids(vec![text])),
        // neue Texte haben wie im Original keine Autom. Unterschneidung (Text_AutoKern, EB-1)
        "pi.text.autoKern" => rs(json!({"on": true}), sel_ids(vec![text])),
        "pi.text.pathOffset" => rs(json!({"offset": 12}), Box::new(move |s: &mut Session| {
            select(s, &[text, ellipse]);
            ex(s, "text.attachToPath", json!({}));
            Value::Null
        })),
        "pi.web" => rs(json!({"url": "https://example.org", "alt": "Alt"}), sel_ids(vec![slice])),
        "pi.behavior.add" => rs(json!({"action": 1, "statusText": "Hallo"}), sel_ids(vec![slice])),
        "pi.behavior.remove" | "pi.behavior.event" | "pi.behavior.clear" => rs(json!({"index": 0, "event": 1}), Box::new(move |s: &mut Session| {
            select(s, &[slice]);
            ex(s, "pi.behavior.add", json!({"action": 1, "statusText": "Hallo"}));
            Value::Null
        })),
        "pi.button" => rs(json!({"text": "Los", "showDownOnLoad": true}), Box::new(move |s: &mut Session| {
            select(s, &[ellipse]);
            ex(s, "modify.symbol.convertToSymbol", json!({"name": "Knopf", "kind": "button"}));
            Value::Null
        })),
        "pi.symbol.params" => rs(json!({"values": {"label": "Neu"}}), Box::new(move |s: &mut Session| {
            select(s, &[ellipse]);
            ex(s, "modify.symbol.convertToSymbol", json!({"name": "S"}));
            Value::Null
        })),
        "pi.export.settings" | "panel.optimize.settings" => r(json!({"settings": serde_json::to_value(fp_model::ExportSettings { format: 1, jpeg_quality: 55, ..Default::default() }).unwrap_or_default()})),
        "pi.element.moveTo" => r(json!({"id": ellipse.0, "layer": ids.layer, "index": 0})),
        "pi.style.new" | "panel.styles.new" => rs(json!({"name": "Stil1"}), sel_ids(vec![rect])),
        "pi.style.apply" | "panel.styles.breakLink" => rs(json!({"name": "Stil1"}), Box::new(move |s: &mut Session| {
            select(s, &[rect]);
            ex(s, "pi.style.new", json!({"name": "Stil1"}));
            select(s, &[ellipse]);
            Value::Null
        })),
        "pi.style.delete" | "pi.style.rename" | "panel.styles.duplicate" | "panel.styles.delete" | "panel.styles.rename" | "panel.styles.redefine" | "panel.styles.options" | "panel.styles.clearOverrides" => {
            rs(json!({"name": "Stil1", "newName": "Stil2", "names": ["Stil1"]}), Box::new(move |s: &mut Session| {
                select(s, &[rect]);
                ex(s, "pi.style.new", json!({"name": "Stil1"}));
                ex(s, "pi.stroke", json!({"width": 7}));
                Value::Null
            }))
        }
        "panel.styles.load" | "panel.styles.importLibrary" => rs(json!({"path": t("stile.json")}), Box::new(move |s: &mut Session| {
            select(s, &[rect]);
            ex(s, "pi.style.new", json!({"name": "Exportiert"}));
            ex(s, "panel.styles.saveLibrary", json!({"path": tmp().join("stile.json").to_string_lossy()}));
            ex(s, "panel.styles.delete", json!({"name": "Exportiert"}));
            Value::Null
        })),
        "panel.styles.saveLibrary" => rs(json!({"path": t("stile_save.json")}), Box::new(move |s: &mut Session| {
            select(s, &[rect]);
            ex(s, "pi.style.new", json!({"name": "S"}));
            Value::Null
        })),
        n if n.starts_with("pi.symbol.") || n.starts_with("panel.library.") && !matches!(n, "panel.library.newSymbol" | "panel.library.play" | "panel.library.selectUnused") => {
            let file = n == "pi.symbol.import" || n == "panel.library.import" || n == "panel.library.update";
            let update = n == "panel.library.update";
            let params = if update { json!({}) } else { json!({"index": 0, "name": "Umbenannt", "path": t("bibliothek.png"), "on": true, "place": true}) };
            rs(params, Box::new(move |s: &mut Session| {
                select(s, &[ellipse]);
                ex(s, "modify.symbol.convertToSymbol", json!({"name": "S"}));
                if file {
                    ex(s, "panel.library.export", json!({"path": tmp().join("bibliothek.png").to_string_lossy()}));
                }
                if update {
                    ex(s, "pi.symbol.import", json!({"path": tmp().join("bibliothek.png").to_string_lossy()}));
                }
                Value::Null
            }))
        }
        "panel.layers.addMasterPageLayer" | "panel.pages.linkToMaster" | "panel.pages.resetMaster" => rs(json!({}), Box::new(|s: &mut Session| {
            ex(s, "page.add", json!({}));
            ex(s, "pi.page.setMaster", json!({"index": 0, "master": true}));
            ex(s, "page.setCurrent", json!({"index": 1}));
            Value::Null
        })),
        "panel.layers.shareToPages" => rs(json!({"pages": [1]}), Box::new(|s: &mut Session| {
            ex(s, "page.add", json!({}));
            ex(s, "page.setCurrent", json!({"index": 0}));
            Value::Null
        })),
        "panel.layers.excludeShared" | "panel.layers.detachShared" => {
            let layer = ids.layer;
            rs(json!({}), Box::new(move |s: &mut Session| {
                ex(s, "page.add", json!({}));
                ex(s, "page.setCurrent", json!({"index": 0}));
                ex(s, "panel.layers.shareToAllPages", json!({}));
                ex(s, "page.setCurrent", json!({"index": 1}));
                ex(s, "layer.setActive", json!({"layer": layer}));
                Value::Null
            }))
        }
        "panel.layers.editMask" => rs(json!({}), Box::new(move |s: &mut Session| {
            select(s, &[rect, ellipse]);
            ex(s, "modify.mask.groupAsMask", json!({}));
            Value::Null
        })),
        "panel.history.clear" => rs(json!({}), Box::new(|s: &mut Session| {
            rect_quick(s);
            Value::Null
        })),
        "panel.swatches.add" | "panel.swatches.replace" => rs(json!({"path": t("farben.act")}), Box::new(|s: &mut Session| {
            ex(s, "panel.swatches.save", json!({"path": tmp().join("farben.act").to_string_lossy()}));
            Value::Null
        })),
        "panel.swatches.colorCubes" | "panel.mixer.hex" | "panel.info.hex" | "panel.info.pixels" => {
            let pre = match name {
                "panel.swatches.colorCubes" => "panel.swatches.grayscale",
                "panel.mixer.hex" => "panel.mixer.hsb",
                "panel.info.hex" => "panel.info.hsb",
                _ => "panel.info.inches",
            };
            rs(json!({}), Box::new(move |s: &mut Session| {
                ex(s, pre, json!({}));
                Value::Null
            }))
        }
        "panel.optimize.deleteSettings" | "panel.optimize.applySettings" => rs(json!({"name": "Meine"}), Box::new(|s: &mut Session| {
            ex(s, "file.imagePreview", json!({"settings": {"format": "gif", "colors": 32}, "apply": true}));
            ex(s, "panel.optimize.saveSettings", json!({"name": "Meine"}));
            ex(s, "file.imagePreview", json!({"settings": {"format": "jpeg"}, "apply": true}));
            Value::Null
        })),
        "panel.optimize.makeTransparent" | "panel.optimize.removeEdit" | "panel.optimize.lockColors" | "panel.optimize.snapToWeb" | "panel.optimize.deleteColors" => {
            let edit = name == "panel.optimize.removeEdit";
            rs(json!({"colors": ["#123456"]}), Box::new(move |s: &mut Session| {
                ex(s, "panel.optimize.addColor", json!({"color": "#123456"}));
                if edit {
                    ex(s, "panel.optimize.editColor", json!({"color": "#123456", "newColor": "#654321"}));
                }
                Value::Null
            }))
        }
        _ => None,
    }
}

/// Fachliche Prüfungen des Ergebnisses (Auswahl wichtiger Befehle).
fn check_for(name: &str, ids: &Ids) -> Option<Check> {
    let (rect, ellipse, text) = (ids.rect, ids.ellipse, ids.text);
    let _ = rect;
    Some(match name {
        "edit.move" => Box::new(move |s, _| {
            let b = bounds(s, ellipse);
            if (b.left - 105.0).abs() < 0.01 && (b.top - 27.0).abs() < 0.01 { Ok(()) } else { Err(format!("Grenzen {b:?}")) }
        }),
        "element.setBounds" => Box::new(move |s, _| {
            let b = bounds(s, ellipse);
            let sel = s.active().map(|d| d.selection.elements.clone()).unwrap_or_default();
            if (b.left - 10.0).abs() < 0.01 && (b.top - 200.0).abs() < 0.01 && (b.width() - 30.0).abs() < 0.01 && sel.is_empty() { Ok(()) } else { Err(format!("Grenzen {b:?}, Auswahl {sel:?}")) }
        }),
        "text.convertToPaths" => Box::new(move |s, _| {
            let d = s.active().ok_or("kein Dokument")?;
            if fp_engine::docops::find_on_page(&d.doc, d.page, text).is_some() {
                return Err("Text noch vorhanden".into());
            }
            let g = top_elements(s).into_iter().find(|e| e.js_type() == "Group").ok_or("keine Gruppe")?;
            match &g.kind {
                fp_model::ElementKind::Group(gr) if gr.children.len() >= 9 => Ok(()),
                _ => Err("zu wenige Glyphenpfade".into()),
            }
        }),
        "text.checkSpelling" => Box::new(move |s, v| {
            let t = el(s, text);
            match &t.kind {
                fp_model::ElementKind::Text(tx) if tx.plain_text() == "Hallo Wald" && v["replaced"] == 1 => Ok(()),
                _ => Err(format!("{v}")),
            }
        }),
        "element.setCustomData" => Box::new(move |s, _| {
            let e = el(s, ellipse);
            let c = &e.common.custom_data;
            if c.get_str("a") == Some("1") && c.get_str("b") == Some("x") && c.get_str("c") == Some("[ 1, 2 ]") { Ok(()) } else { Err(format!("{c:?}")) }
        }),
        "element.setPattern" => Box::new(move |s, _| {
            let e = el(s, ellipse);
            let a = fp_engine::docops::path_attrs_of(&e).ok_or("keine Attribute")?;
            let f = a.fill.as_ref().ok_or("keine Füllung")?;
            if f.shape == 15 && f.pattern.as_ref().and_then(|p| p.image.as_ref()).is_some() { Ok(()) } else { Err("kein Muster".into()) }
        }),
        "element.setTexture" => Box::new(move |s, _| {
            let e = el(s, ellipse);
            let a = fp_engine::docops::path_attrs_of(&e).ok_or("keine Attribute")?;
            let m = a.fill_texture.as_ref().and_then(|t| t.mask.as_ref()).ok_or("keine Textur")?;
            if m.grid.width == 16 && a.fill.as_ref().is_some_and(|f| f.texture_amount == 500) { Ok(()) } else { Err("Textur falsch".into()) }
        }),
        "element.setBrushSensitivity" => Box::new(move |s, _| {
            let e = el(s, rect);
            let a = fp_engine::docops::path_attrs_of(&e).ok_or("keine Attribute")?;
            let b = a.brush.as_ref().ok_or("kein Pinsel")?;
            if b.sensitivities.iter().any(|x| x.source == b'P' && x.target == b'Z' && x.value == 500) { Ok(()) } else { Err(format!("{:?}", b.sensitivities)) }
        }),
        "file.export" => Box::new(|_, v| {
            let f = v["files"][0].as_str().ok_or("keine Datei")?;
            let b = std::fs::read(f).map_err(|e| e.to_string())?;
            if b.starts_with(&[0x89, b'P', b'N', b'G']) { Ok(()) } else { Err("kein PNG".into()) }
        }),
        "file.exportArea" => Box::new(|_, v| if v["width"] == 50 && v["height"] == 50 { Ok(()) } else { Err(format!("{v}")) }),
        "file.imagePreview" => Box::new(|s, v| {
            let d = s.active().ok_or("kein Dokument")?;
            let st = fp_export::doc_export::page_settings(&d.doc, d.page);
            if st.format == fp_export::ExportFormat::Jpeg && st.jpeg.quality == 60 && v["bytes"].as_u64().unwrap_or(0) > 100 { Ok(()) } else { Err(format!("{v}")) }
        }),
        "file.print" => Box::new(|_, v| {
            let f = v["printed"][0]["file"].as_str().ok_or("kein Druckbild")?;
            if std::path::Path::new(f).is_file() && v["printed"][0]["width"].as_u64() > v["printed"][0]["height"].as_u64() { Ok(()) } else { Err(format!("{v}")) }
        }),
        "file.batchProcess" => Box::new(|_, v| {
            let f = v["processed"][0]["output"].as_str().ok_or_else(|| format!("{v}"))?;
            let b = std::fs::read(f).map_err(|e| format!("{f}: {e}"))?;
            if b.starts_with(&[0xFF, 0xD8]) { Ok(()) } else { Err("kein JPEG".into()) }
        }),
        // Batch: Lauf schreibt genau eine PNG-Datei, Plan schreibt nichts, Vorschau liefert ein Bild
        "batch.run" => Box::new(|_, v| {
            let out = tmp().join("batch_run_out");
            let pngs: Vec<_> = std::fs::read_dir(&out)
                .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| std::fs::read(p).is_ok_and(|b| b.starts_with(&[0x89, b'P', b'N', b'G']))).collect())
                .unwrap_or_default();
            if v["ok"] == true && v["totals"]["done"] == 1 && pngs.len() == 1 { Ok(()) } else { Err(format!("{v} – Ausgabe {pngs:?}")) }
        }),
        "batch.plan" => Box::new(|_, v| {
            let n = v["entries"].as_array().map(Vec::len).unwrap_or(0);
            if n == 1 && !tmp().join("batch_plan_out").exists() { Ok(()) } else { Err(format!("{v}")) }
        }),
        "batch.preview" => Box::new(|_, v| {
            // Größe des Testdokuments (400 × 300), Bild als PNG
            let size_ok = v["before"]["width"] == 400 && v["before"]["height"] == 300 && v["after"]["width"] == 400;
            if size_ok && v["after"]["png"].as_str().is_some_and(|p| p.starts_with("iVBOR")) { Ok(()) } else { Err(format!("{v}")) }
        }),
        "file.import" => Box::new(|s, v| {
            let n = v["ids"].as_array().map(|a| a.len()).unwrap_or(0);
            if n == 4 && top_elements(s).len() == 10 { Ok(()) } else { Err(format!("{v}")) }
        }),
        "modify.symbol.exitEdit" => Box::new(|s, _| {
            if s.documents().len() != 1 {
                return Err("Ansicht nicht geschlossen".into());
            }
            let d = s.active().ok_or("kein Dokument")?;
            if d.history.undo_label() == Some("Symbol bearbeiten") { Ok(()) } else { Err(format!("{:?}", d.history.undo_label())) }
        }),
        "modify.symbol.editSymbol" | "modify.symbol.editInPlace" => Box::new(|s, _| {
            let d = s.active().ok_or("kein Dokument")?;
            if d.symbol_edit.is_some() && s.documents().len() == 2 && !top_elements(s).is_empty() { Ok(()) } else { Err("keine Bearbeitungsansicht".into()) }
        }),
        "modify.popUpMenu.addPopUpMenu" => Box::new(|_, v| if v["items"][1]["submenu"][0]["text"] == "A" { Ok(()) } else { Err(format!("{v}")) }),
        "modify.popUpMenu.editPopUpMenu" => Box::new(|_, v| if v["items"][0]["text"] == "Neu" { Ok(()) } else { Err(format!("{v}")) }),
        "document.setOrigin" => Box::new(|s, _| {
            let d = s.active().ok_or("kein Dokument")?;
            let o = d.doc.pages[d.page].canvas.origin;
            if o.x == 10.0 && o.y == 20.0 { Ok(()) } else { Err(format!("{o:?}")) }
        }),
        "edit.copyHtmlCode" => Box::new(|s, _| if s.clipboard.text.as_deref().is_some_and(|t| t.contains("<table")) { Ok(()) } else { Err("kein HTML".into()) }),
        "modify.group" => Box::new(|s, _| if top_elements(s).iter().any(|e| e.js_type() == "Group") { Ok(()) } else { Err("keine Gruppe".into()) }),
        "modify.transform.flipHorizontal" => Box::new(move |s, _| {
            let b = bounds(s, ellipse);
            if (b.left - 100.0).abs() < 0.01 { Ok(()) } else { Err(format!("{b:?}")) }
        }),
        _ => return None,
    })
}

/// Zielbefehl eines Bedienfeldmenü-Alias (Statustext „Bedienfeldmenü: wie „x““).
fn alias_target(info: &fp_engine::CommandInfo) -> Option<String> {
    let d = info.description.as_deref()?;
    let rest = d.strip_prefix("Bedienfeldmenü: wie „")?;
    Some(rest.split('“').next()?.to_string())
}

fn case_for(name: &str, info: &fp_engine::CommandInfo, ids: &Ids) -> Case {
    if overrides(name, ids).is_none()
        && let Some(target) = alias_target(info)
        && let Some(t) = CommandRegistry::with_defaults().get(&target).map(|c| c.info().clone())
    {
        let mut c = case_for(&target, &t, ids);
        c.check = check_for(name, ids);
        return c;
    }
    let (params, setup) = match overrides(name, ids) {
        Some((p, s)) => (p, s.unwrap_or_else(|| auto_setup(&info.requires, ids))),
        None => {
            let mut o = serde_json::Map::new();
            for p in info.params.iter().filter(|p| p.required) {
                o.insert(p.name.clone(), default_param(p, ids));
            }
            (Value::Object(o), auto_setup(&info.requires, ids))
        }
    };
    // Copy is a history row of its own that leaves the document untouched (TW-137)
    let no_effect_ok = matches!(name, "file.saveAll" | "view.fitSelection" | "view.hideSelection" | "ai.clearCache" | "edit.copy" | "edit.copyAsVectors");
    Case { params, setup, check: check_for(name, ids), no_effect_ok }
}

#[test]
fn all_commands_work() {
    let _g = serial();
    fp_i18n::profile::use_test_profile(); // nie ins echte Nutzerprofil (EB-3)
    // nie Fremdprogramme (Browser, E-Mail, Druck, Explorer) starten
    fp_engine::shell::disable();

    let reg = CommandRegistry::with_defaults();
    let names: Vec<String> = reg.iter().map(|c| c.info().name.clone()).collect();
    let filter = std::env::var("FP_ONLY").ok();
    let mut ok = 0;
    let mut failures = Vec::new();
    for n in &names {
        if filter.as_deref().is_some_and(|f| !n.contains(f)) {
            continue;
        }
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_case(&reg, n)));
        match res {
            Ok(Ok(())) => ok += 1,
            Ok(Err(e)) => failures.push(format!("{n}: {e}")),
            Err(p) => failures.push(format!("{n}: Panik {:?}", p.downcast_ref::<String>().cloned().or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string())))),
        }
    }
    let total = if filter.is_some() { ok + failures.len() } else { names.len() };
    println!("Funktionstest aller Befehle: {ok}/{total}");
    for f in &failures {
        println!("FEHLT {f}");
    }
    assert!(failures.is_empty(), "{} von {total} Befehlen scheitern", failures.len());
}


/// Korpus: Befehle mit Wirkung auf die ganze Seite (Auswahl = alles) auf echten FW-PNG-Dateien:
/// kein Fehler außer „nicht verfügbar“, Rückgängig exakt, Datei speichern und wieder lesen.
/// `cargo test -p fp-engine --test all_commands -- --ignored --nocapture` (Dateizahl `FP_CORPUS_N`, Vorgabe 25).
#[test]
#[ignore]
fn commands_on_corpus_files() {
    let _g = serial();
    fp_i18n::profile::use_test_profile(); // nie ins echte Nutzerprofil (EB-3)
    fp_engine::shell::disable();
    let m = std::fs::canonicalize(env!("CARGO_MANIFEST_DIR")).unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    let root = std::env::var_os("FP_CORPUS_DIR").map(PathBuf::from).unwrap_or_else(|| m.join("../../../korpus"));
    let mut files = Vec::new();
    let mut stack = vec![root];
    while let Some(d) = stack.pop() {
        let Ok(it) = std::fs::read_dir(&d) else { continue };
        for e in it.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("png")) && std::fs::read(&p).is_ok_and(|b| fp_fwpng::is_fwpng(&b)) {
                files.push(p);
            }
        }
    }
    files.sort();
    let n: usize = std::env::var("FP_CORPUS_N").ok().and_then(|v| v.parse().ok()).unwrap_or(25);
    let step = (files.len() / n.max(1)).max(1);
    let picked: Vec<PathBuf> = files.into_iter().step_by(step).take(n).collect();
    let cmds: &[(&str, Value)] = &[
        ("modify.group", json!({})),
        ("modify.transform.flipHorizontal", json!({})),
        ("modify.transform.flipVertical", json!({})),
        ("modify.transform.rotate90DegreeCw", json!({})),
        ("modify.transform.rotate180Degree", json!({})),
        ("modify.align.left", json!({})),
        ("modify.align.centerVertical", json!({})),
        ("modify.arrange.bringToFront", json!({})),
        ("modify.arrange.sendToBack", json!({})),
        ("edit.duplicate", json!({})),
        ("edit.clone", json!({})),
        ("edit.clear", json!({})),
        ("edit.move", json!({"dx": 7, "dy": -3})),
        ("element.setOpacity", json!({"opacity": 50})),
        ("element.addEffect", json!({"effect": "dropShadow"})),
        ("modify.canvas.imageSize", json!({"percent": 50})),
        ("modify.canvas.canvasSize", json!({"width": 640, "height": 480})),
        ("modify.canvas.canvasColor", json!({"color": "#123456"})),
        ("modify.canvas.rotate90DegreeCw", json!({})),
        ("modify.canvas.fitCanvas", json!({})),
        ("modify.symbol.convertToSymbol", json!({"name": "Korpus"})),
        ("modify.flattenSelection", json!({})),
        ("select.inverse", json!({})),
        ("file.imagePreview", json!({"settings": {"format": "png32"}})),
        ("file.exportWizard", json!({"apply": true})),
        ("text.checkSpelling", json!({})),
    ];
    let out = tmp().join("korpus");
    let _ = std::fs::create_dir_all(&out);
    let (mut ok, mut skipped, mut failures) = (0, 0, Vec::new());
    for f in &picked {
        for (name, params) in cmds {
            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<bool, String> {
                let mut s = Session::default();
                s.set_renderer(renderer());
                s.execute("file.open", json!({"path": f.to_string_lossy()})).map_err(|e| format!("öffnen: {e}"))?;
                let _ = s.execute("select.all", json!({}));
                let before = snap(&s);
                match s.execute(name, params.clone()) {
                    Ok(_) => {}
                    Err(fp_engine::EngineError::Unavailable { .. }) | Err(fp_engine::EngineError::Selection(_)) => return Ok(false),
                    Err(e) => return Err(e.to_string()),
                }
                let after = snap(&s);
                if after.undo == before.undo + 1 {
                    s.undo().map_err(|e| e.to_string())?;
                    let u = snap(&s);
                    if !u.doc.as_ref().zip(before.doc.as_ref()).is_some_and(|(x, y)| Arc::ptr_eq(x, y)) {
                        return Err("Rückgängig nicht exakt".into());
                    }
                    s.redo().map_err(|e| e.to_string())?;
                }
                if doc_changed(&before, &after) {
                    file_roundtrip(&s, &format!("korpus_{name}"))?;
                }
                Ok(true)
            }));
            match res {
                Ok(Ok(true)) => ok += 1,
                Ok(Ok(false)) => skipped += 1,
                Ok(Err(e)) => failures.push(format!("{} {name}: {e}", f.display())),
                Err(_) => failures.push(format!("{} {name}: Panik", f.display())),
            }
        }
    }
    println!("Korpus: {} Dateien, {ok} Läufe ok, {skipped} nicht verfügbar, {} Fehler", picked.len(), failures.len());
    for f in &failures {
        println!("FEHLT {f}");
    }
    assert!(failures.is_empty());
}

// ---------------------------------------------------------------------- Wiederholung (AGENTEN_REGELN §4b)

/// Befehle, die bei jeder Ausführung bewusst etwas **hinzufügen** oder **relativ** ändern
/// (Ebene/Seite/Status/Objekt hinzufügen, verschieben, drehen, skalieren …). Für sie gilt: N
/// Ausführungen = N Verlaufsschritte, Rückgängig/Wiederholen der ganzen Folge exakt. Alle anderen
/// Dokumentbefehle mit Parametern müssen **setzen**: 50× mit wechselnden Werten und danach der
/// Endwert ergeben exakt denselben Zustand wie einmal der Endwert (keine Duplikate).
const ADDITIVE: &[&str] = &[
    "draw.", "web.addHotspot", "web.addSlice", "layer.add", "layer.duplicate", "page.add", "page.duplicate", "state.add", "state.duplicate", "guide.add", "edit.move", "edit.insert.",
    "edit.duplicate", "edit.clone", "edit.paste", "transform.", "path.", "bitmap.", "element.addEffect", "file.import", "modify.canvas.imageSize",
    "modify.canvas.canvasSize", "modify.transform.numericTransform", "pi.symbol.place", "pi.symbol.import", "panel.layers.newLayer", "panel.layers.duplicate",
    "panel.states.add", "panel.states.duplicate", "panel.library.newSymbol", "panel.library.duplicate", "panel.styles.duplicate", "panel.optimize.addColor",
    "panel.swatches.addColor", "panel.swatches.add", "filter.", "select.", "pi.layer.addSub", "pi.style.new", "autoShape.insert", "tool.shape",
    "modify.popUpMenu.addPopUpMenu", "pi.behavior.add", "text.checkSpelling", "edit.cropDocument", "modify.symbol.tweenInstances", "pi.state.copyTo",
    "history.replay", "layer.move", "page.move", "state.move", "pi.element.moveTo", "panel.history.replaySelected", "modify.animation.animateSelection", "edit.insert",
    "modify.symbol.convertToSymbol", "pi.effect.move", "file.batchProcess",
    // Farbe löschen verbraucht: jede Ausführung entfernt eine (weitere) Farbe aus der Palette
    "panel.optimize.deleteColors", "panel.optimize.deleteColorMenu",
];

fn additive(name: &str) -> bool {
    ADDITIVE.iter().any(|p| if p.ends_with('.') { name.starts_with(p) } else { name == *p || name.starts_with(&format!("{p}.")) })
}

/// Zwischenwert für Wiederholung `k`: Zahlen, Wahrheitswerte und Farben wechseln, Bezüge bleiben.
fn vary(name: &str, v: &Value, k: usize) -> Value {
    let Value::Object(m) = v else { return v.clone() };
    // Paletteneinträge werden über ihre Farbe angesprochen (Bezug, kein Wert)
    let palette = name.starts_with("panel.optimize.");
    let fixed = ["index", "id", "ids", "layer", "page", "from", "to", "points", "rect", "path", "name", "newName", "names", "count", "steps", "open", "apply", "discard", "replace", "effect", "shape", "mode", "draw", "params", "symbol", "tool", "set", "orientation"];
    let mut o = serde_json::Map::new();
    for (key, x) in m {
        let nx = if fixed.contains(&key.as_str()) || (palette && key == "color") {
            x.clone()
        } else {
            match x {
                Value::Number(n) => {
                    let f = n.as_f64().unwrap_or(0.0);
                    let d = (k % 5) as f64 + 1.0;
                    if n.is_i64() || n.is_u64() { json!((f as i64 - d as i64).max(0)) } else { json!((f - d).max(0.0)) }
                }
                Value::Bool(b) => json!(if k.is_multiple_of(2) { !b } else { *b }),
                Value::String(s) if s.starts_with('#') && s.len() == 7 => json!(format!("#{:02x}{}", (k * 37) % 256, &s[3..])),
                other => other.clone(),
            }
        };
        o.insert(key.clone(), nx);
    }
    Value::Object(o)
}

fn repeat_case(reg: &CommandRegistry, name: &str) -> Result<&'static str, String> {
    let info = reg.get(name).map(|c| c.info().clone()).ok_or("nicht registriert")?;
    let (mut b, ids) = fixture();
    let case = case_for(name, &info, &ids);
    if !case.params.as_object().is_some_and(|m| !m.is_empty()) {
        return Ok("ohne Parameter");
    }
    let _ = (case.setup)(&mut b);
    let start = snap(&b);
    // einmal mit dem Endwert
    let out = b.execute(name, case.params.clone()).map_err(|e| format!("einmal: {e}"))?;
    if out.effect != fp_engine::Effect::Document || b.documents().len() != 1 {
        return Ok("kein Dokumentbefehl");
    }
    let once = b.active().map(|d| fingerprint(&d.doc)).unwrap_or_default();
    let add = additive(name);
    // weitere 49× (wechselnde Werte bzw. gleiche bei additiven), dann noch einmal der Endwert
    for k in 0..49 {
        let p = if add { case.params.clone() } else { vary(name, &case.params, k) };
        match b.execute(name, p.clone()) {
            Ok(_) => {}
            // Befehle, die ihr Ziel verbrauchen (löschen, umbenennen …), dürfen danach „nicht gefunden“ melden
            Err(fp_engine::EngineError::NotFound(_) | fp_engine::EngineError::Unavailable { .. } | fp_engine::EngineError::Selection(_) | fp_engine::EngineError::ElementNotFound(_)) => {
                return Ok("einmalig");
            }
            Err(fp_engine::EngineError::InvalidParams { .. }) if !add => {}
            Err(e) => return Err(format!("Wiederholung {k} mit {p}: {e}")),
        }
    }
    b.execute(name, case.params.clone()).map_err(|e| format!("Endwert: {e}"))?;
    let end = snap(&b);
    if !add {
        let again = b.active().map(|d| fingerprint(&d.doc)).unwrap_or_default();
        if again != once {
            return Err("50× setzen ergibt einen anderen Zustand als einmal mit dem Endwert (Duplikate?)".into());
        }
    }
    // Rückgängig der ganzen Folge, dann Wiederholen
    let steps = end.undo - start.undo;
    for _ in 0..steps {
        b.undo().map_err(|e| format!("Rückgängig: {e}"))?;
    }
    let u = snap(&b);
    if !u.doc.as_ref().zip(start.doc.as_ref()).is_some_and(|(x, y)| Arc::ptr_eq(x, y)) {
        return Err(format!("{steps}× Rückgängig stellt den Ausgangszustand nicht her"));
    }
    for _ in 0..steps {
        b.redo().map_err(|e| format!("Wiederholen: {e}"))?;
    }
    let r = snap(&b);
    if !r.doc.as_ref().zip(end.doc.as_ref()).is_some_and(|(x, y)| Arc::ptr_eq(x, y)) {
        return Err(format!("{steps}× Wiederholen stellt den Endzustand nicht her"));
    }
    // gemischte Folge: Rückgängig, Ausführen, Rückgängig, Wiederholen
    if steps > 0 {
        b.undo().map_err(|e| e.to_string())?;
        b.execute(name, case.params.clone()).map_err(|e| format!("nach Rückgängig: {e}"))?;
        let c = snap(&b);
        b.undo().map_err(|e| e.to_string())?;
        b.redo().map_err(|e| e.to_string())?;
        if !snap(&b).doc.as_ref().zip(c.doc.as_ref()).is_some_and(|(x, y)| Arc::ptr_eq(x, y)) {
            return Err("Folge Rückgängig/Ausführen/Rückgängig/Wiederholen falsch".into());
        }
    }
    Ok(if add { "additiv" } else { "setzend" })
}
#[test]
fn repeated_commands_replace_instead_of_append() {
    let _g = serial();
    fp_i18n::profile::use_test_profile(); // nie ins echte Nutzerprofil (EB-3)
    fp_engine::shell::disable();
    let reg = CommandRegistry::with_defaults();
    let filter = std::env::var("FP_ONLY").ok();
    let (mut setting, mut adding, mut other, mut consuming) = (0, 0, 0, 0);
    let mut failures = Vec::new();
    for c in reg.iter() {
        let n = c.info().name.clone();
        if filter.as_deref().is_some_and(|f| !n.contains(f)) {
            continue;
        }
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| repeat_case(&reg, &n))) {
            Ok(Ok("setzend")) => setting += 1,
            Ok(Ok("additiv")) => adding += 1,
            Ok(Ok("einmalig")) => consuming += 1,
            Ok(Ok(_)) => other += 1,
            Ok(Err(e)) => failures.push(format!("{n}: {e}")),
            Err(_) => failures.push(format!("{n}: Panik")),
        }
    }
    println!("Wiederholungstest: {setting} setzende, {adding} additive und {consuming} verbrauchende Dokumentbefehle geprüft, {other} ohne Parameter/kein Dokumentbefehl, {} Fehler", failures.len());
    for f in &failures {
        println!("FEHLT {f}");
    }
    assert!(failures.is_empty());
}
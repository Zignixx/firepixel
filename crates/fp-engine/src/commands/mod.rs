//! Eingebaute Befehle. Je Menü ein Modul; gemeinsame Hilfen hier.
//!
//! Konvention: Befehle auf dem aktiven Dokument ändern es Copy-on-Write
//! (`Arc::make_mut(&mut state.doc)`) und liefern [`crate::command::CommandOutput::doc`]; die Sitzung legt dann den
//! Verlaufsschritt an. Parameter sind in [`crate::command::ParamSpec`] beschrieben (deutsch).

pub mod ai;
pub mod batch; // BATCH-1: batch.* (Haken BatchHost, fp-batch)
pub(crate) mod bitmap;
mod draw;
mod edit;
pub mod export;
mod import;
pub mod symbol_edit;
mod popup;
pub mod spelling;
pub mod attrs;
mod autoshape;
mod file;
pub mod file_info;
mod filters;
pub mod misc;
mod modify;
pub mod panel;
mod select;
mod text;
mod retouch_px;
pub(crate) use tooling::stamp_image;
mod tooling;
pub mod trace;
mod view;

pub use export::PageSetup;
pub use file::{load_document, save_document};
pub use filters::{whole_numbers_as_ints, FILTERS};
pub(crate) use text::editor_patch;
pub use text::tool_text_defaults;

use crate::command::{CommandRegistry, Requirement};
use crate::docops;
use crate::error::{EngineError, Result};
use crate::session::{DocumentState, Session};
use fp_color::Argb;
use fp_geom::Rect;
use fp_model::traverse::ElementPath;
use fp_model::{Brush, Document, Element, ElementId, Fill, PathAttrs};
use std::sync::Arc;

/// Registriert alle eingebauten Befehle.
pub fn register_all(r: &mut CommandRegistry) {
    file::register(r);
    file_info::register(r);
    export::register(r);
    misc::register(r);
    import::register(r);
    symbol_edit::register(r);
    popup::register(r);
    spelling::register(r);
    attrs::register(r);
    autoshape::register(r);
    edit::register(r);
    view::register(r);
    select::register(r);
    modify::register(r);
    text::register(r);
    filters::register(r);
    bitmap::register(r);
    draw::register(r);
    tooling::register(r);
    panel::register(r);
    ai::register(r);
    trace::register(r);
    batch::register(r);
}

pub(crate) const DOC: &[Requirement] = &[Requirement::Document];
pub(crate) const SEL: &[Requirement] = &[Requirement::Document, Requirement::Selection];

/// ARGB als `#rrggbb` bzw. `#rrggbbaa`.
pub fn argb_hex(c: Argb) -> String {
    fp_model::effects::format_hex_color(c)
}

/// Aktives Dokument mit Copy-on-Write-Zugriff: `f(state, doc)`.
pub(crate) fn with_doc<R>(s: &mut Session, f: impl FnOnce(&mut DocumentState) -> Result<R>) -> Result<R> {
    let d = s.doc_mut()?;
    f(d)
}

/// Adressen der ausgewählten Elemente (oben zuerst).
/// Gesperrte Objekte (Objekt oder Ebene gesperrt) werden wie im Original nicht bearbeitet.
pub(crate) fn selected_paths(d: &DocumentState) -> Vec<ElementPath> {
    docops::paths_sorted(&d.doc, d.page, &d.selection.elements).into_iter().filter(|p| !docops::is_locked(&d.doc, d.page, p)).collect()
}

/// Ausgewählte Elemente (oben zuerst).
pub(crate) fn selected_elements(d: &DocumentState) -> Vec<(ElementPath, Arc<Element>)> {
    selected_paths(d).into_iter().filter_map(|p| d.doc.element_at(&p).cloned().map(|e| (p, e))).collect()
}

/// Fehler „keine passende Auswahl“; `key` = Schlüssel `err-sel-…` bzw. `err-req-…`.
pub(crate) fn sel_err(key: &str) -> EngineError {
    EngineError::Selection(fp_i18n::t(key))
}

/// Menüfreigabe wie im Original (PM-7, MN-2): Bei einer Pixelauswahl sperrt das Original die
/// Objektbefehle (Duplizieren, Klonen, Als Vektoren kopieren, Gesamtauswahl treffen, Auswahl
/// animieren, In Symbol konvertieren, Auswahl auf Hintergrund reduzieren, Anordnen, Gruppieren).
/// Ausschneiden, Kopieren und Löschen bleiben aktiv.
pub(crate) fn no_pixel_selection(s: &Session) -> bool {
    s.active().is_none_or(|d| !d.selection.has_pixels())
}

/// Nicht nur Web-Objekte (Segmente, Hotspots) ausgewählt: Befehle, die das Original bei einem
/// Segment sperrt (Als Vektoren kopieren, In Symbol konvertieren, Auswahl auf Hintergrund
/// reduzieren, Maske ▸ Alles ein-/ausblenden, Transformationen entfernen).
pub(crate) fn not_only_web(s: &Session) -> bool {
    s.active().is_none_or(|d| {
        let els = selected_elements(d);
        els.is_empty() || !els.iter().all(|(_, e)| matches!(e.kind, fp_model::element::ElementKind::WebObject(_)))
    })
}

/// Attribute für neue Formen aus den Werkzeugfarben: Füllung (wenn `fill`), Strich (wenn
/// `stroke_width > 0` und die Strichfarbe nicht transparent ist).
pub(crate) fn shape_attrs(s: &Session, fill: bool, stroke_width: f32) -> PathAttrs {
    let mut a = PathAttrs { stroke_color: s.tools.stroke_color, fill_color: s.tools.fill_color, ..PathAttrs::default() };
    if fill && s.tools.fill_color.a() > 0 {
        a.fill = Some(Fill::solid());
    }
    if stroke_width > 0.0 && s.tools.stroke_color.a() > 0 {
        a.brush = Some(Brush::basic(stroke_width.round().max(1.0) as i32));
    }
    a
}

/// Fügt ein neues Element oben in die aktive Ebene ein und wählt es aus.
pub(crate) fn insert_new(d: &mut DocumentState, e: Element) -> Result<ElementId> {
    let id = e.id;
    let (page, frame, layer) = (d.page, d.frame, d.layer);
    let doc = Arc::make_mut(&mut d.doc);
    docops::insert_element(doc, page, frame, layer, 0, Arc::new(e)).ok_or_else(|| EngineError::Failed(fp_i18n::t("err-element-insert-failed")))?;
    d.selection.set(vec![id]);
    Ok(id)
}

/// Neue Laufzeit-ID-Quelle des Dokuments (vorübergehend herausgenommen).
pub(crate) fn take_ids(doc: &mut Document) -> fp_model::IdGen {
    std::mem::take(&mut doc.ids)
}

/// Leinwand-Rechteck der Seite.
pub(crate) fn canvas_rect(doc: &Document, page: usize) -> Rect {
    doc.pages
        .get(page)
        .map(|p| Rect::new(p.canvas.origin.x, p.canvas.origin.y, p.canvas.origin.x + p.canvas.width, p.canvas.origin.y + p.canvas.height))
        .unwrap_or_default()
}

/// IDs als JSON.
pub(crate) fn ids_json(ids: &[ElementId]) -> serde_json::Value {
    serde_json::Value::Array(ids.iter().map(|i| serde_json::json!(i.0)).collect())
}

/// Führt einen Auswahlbefehl auf den Elementen des Parameters `ids` aus (falls angegeben) und stellt
/// danach die vorherige Auswahl wieder her; ohne `ids` wirkt er auf die Auswahl.
pub(crate) fn with_targets(s: &mut Session, p: &crate::command::Params, f: impl FnOnce(&mut Session, &crate::command::Params) -> Result<crate::command::CommandOutput>) -> Result<crate::command::CommandOutput> {
    let Some(ids) = p.opt_ids("ids")?.filter(|v| !v.is_empty()) else {
        return f(s, p);
    };
    let saved = s.doc()?.selection.clone();
    s.doc_mut()?.selection.set(ids);
    let r = f(s, p);
    if let Ok(d) = s.doc_mut() {
        d.selection = saved;
    }
    r
}

/// Ziel-IDs: Parameter `ids` oder die Auswahl.
pub(crate) fn target_ids(d: &DocumentState, p: &crate::command::Params) -> Result<Vec<ElementId>> {
    match p.opt_ids("ids")? {
        Some(v) if !v.is_empty() => Ok(v),
        _ => {
            if d.selection.elements.is_empty() {
                Err(sel_err("err-sel-no-objects"))
            } else {
                Ok(d.selection.elements.clone())
            }
        }
    }
}

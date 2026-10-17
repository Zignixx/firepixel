//! Symbol-Bearbeitungsmodus (Modifizieren ▸ Symbol ▸ Symbol bearbeiten / An Position bearbeiten).
//!
//! Das Symbol wird in einer eigenen Bearbeitungsansicht geöffnet: ein Dokument der Sitzung, dessen
//! einzige Seite die Ebenen, Status und Hilfslinien des Symbols trägt (Leinwand = Symbolinhalt in
//! Symbolkoordinaten, transparent). Alle Befehle und Werkzeuge wirken dort wie auf einer Seite und
//! haben einen eigenen Verlauf. „Fertig“ (`modify.symbol.exitEdit`, auch beim Schließen der Ansicht)
//! schreibt das Ergebnis als **ein** Verlaufsschritt in das Dokument zurück; alle Instanzen zeigen
//! danach den neuen Inhalt. Bei „An Position“ merkt sich die Ansicht die Abbildung der Instanz,
//! damit die Oberfläche die Seite abgeblendet dahinter zeigen kann.

use super::selected_elements;
use crate::command::{CommandOutput, CommandRegistry, ParamKind, Requirement, cmd};
use crate::docops;
use crate::error::{EngineError, Result};
use crate::session::{DocId, Session, SessionEvent, SymbolEdit};
use fp_model::element::ElementKind;
use fp_model::{Document, LayerSlot, SymbolId};
use serde_json::json;
use std::sync::Arc;

/// Inhaltsgrenzen eines Symbols (alle Status) in Symbolkoordinaten.
fn symbol_bounds(doc: &Document, sym: &fp_model::Symbol) -> Option<fp_geom::Rect> {
    let mut b: Option<fp_geom::Rect> = None;
    for slot in &sym.root.layers {
        if let LayerSlot::Own(l) = slot {
            for cel in &l.cels {
                for e in &cel.elements {
                    if let Some(r) = docops::element_bounds(doc, e) {
                        b = Some(b.map_or(r, |x| x.union(&r)));
                    }
                }
            }
        }
    }
    b
}

/// Öffnet die Bearbeitungsansicht eines Symbols.
pub fn begin_edit(
    s: &mut Session,
    symbol: SymbolId,
    in_place: bool,
    instance: Option<(fp_model::ElementId, fp_geom::Matrix3)>,
) -> Result<DocId> {
    let d = s.doc()?;
    let parent = d.id;
    let page = d.page;
    let (_, sym) = d.doc.symbol_by_id(symbol).ok_or_else(|| {
        EngineError::NotFound(fp_i18n::tr!("err-nf-symbol-id", id = symbol.0.to_string()))
    })?;
    let sym = Arc::clone(sym);
    // TW-36: Die Ansicht hat die Größe der Dokumentseite (Original: `dom` 800 × 600 auch für ein
    // leeres Symbol), mittig um den Inhalt bzw. den Bezugspunkt 0,0; größerer Inhalt vergrößert sie.
    let (pw, ph) = d
        .doc
        .pages
        .get(page)
        .map(|p| (p.canvas.width, p.canvas.height))
        .unwrap_or((100.0, 100.0));
    let b = symbol_bounds(&d.doc, &sym).unwrap_or(fp_geom::Rect::new(0.0, 0.0, 0.0, 0.0));
    // re 28 S2: Die Koordinaten des Inhalts bleiben symbolbezogen, die Leinwand hat die Größe des
    // Dokuments. An Position: die Leinwand liegt so, dass der Inhalt an der Instanzlage erscheint
    // (Ursprung = Instanzabbildung, die Seite liegt abgeblendet dahinter); sonst liegt der
    // Symbolursprung 0,0 in der Leinwandmitte. Größerer Inhalt vergrößert die Ansicht.
    let at = instance
        .filter(|_| in_place)
        .map(|(_, m)| m.apply(fp_geom::Point::new(0.0, 0.0)));
    let (w, h, l, t) = match at {
        Some(o) => {
            let (pl, pt) = d
                .doc
                .pages
                .get(page)
                .map(|p| (p.canvas.origin.x, p.canvas.origin.y))
                .unwrap_or((0.0, 0.0));
            (
                pw.max(1.0).ceil(),
                ph.max(1.0).ceil(),
                (pl - o.x).round(),
                (pt - o.y).round(),
            )
        }
        None => {
            let hw = b.left.abs().max(b.right.abs());
            let hh = b.top.abs().max(b.bottom.abs());
            let (w, h) = (
                pw.max((2.0 * hw).ceil()).max(1.0).ceil(),
                ph.max((2.0 * hh).ceil()).max(1.0).ceil(),
            );
            (w, h, (-w / 2.0).floor(), (-h / 2.0).floor())
        }
    };
    let parent_title = d.title.clone();
    let mut doc = Document::new(w, h, fp_color::Argb(0x00FF_FFFF));
    doc.symbols = d.doc.symbols.clone();
    doc.styles = d.doc.styles.clone();
    doc.ids = d.doc.ids.clone();
    if let Some(pg) = doc.page_mut(0) {
        pg.root = sym.root.clone();
        pg.root.background = fp_color::Argb(0x00FF_FFFF);
        pg.canvas.origin = fp_geom::Point::new(l, t);
        pg.canvas.width = w;
        pg.canvas.height = h;
        pg.name = Some(sym.name.clone());
    }
    // Symbol braucht mindestens eine normale Ebene
    if crate::docops::top_normal_layer(&doc, 0).is_none() {
        let mut ids = std::mem::take(&mut doc.ids);
        let layer = fp_model::Layer::new(
            &mut ids,
            &fp_i18n::tr!("msg-default-layer", n = "1"),
            fp_model::LayerKind::Normal,
            doc.pages[0].root.frame_count.max(1) as usize,
        );
        doc.ids = ids;
        if let Some(pg) = doc.page_mut(0) {
            pg.root.layers.push(LayerSlot::Own(Arc::new(layer)));
        }
    }
    let id = s.add_document(doc, None);
    let st = s.doc_mut()?;
    // TW-36: Titel wie im Original „<Dokument> (Symbol)“ (Bearbeitung im Fenster des Dokuments)
    st.title = format!("{parent_title} (Symbol)");
    st.symbol_edit = Some(SymbolEdit {
        parent,
        symbol,
        in_place,
        page,
        instance: instance.map(|x| x.0),
        transform: instance.map(|x| x.1),
    });
    Ok(id)
}

/// Schreibt die Bearbeitungsansicht `edit` in ihr Dokument zurück (ein Verlaufsschritt), schließt
/// sie und aktiviert das Dokument.
pub fn finish_edit(s: &mut Session, edit: DocId, discard: bool) -> Result<DocId> {
    let st = s
        .document(edit)
        .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-edit-view")))?;
    let link = st
        .symbol_edit
        .clone()
        .ok_or_else(|| EngineError::Failed(fp_i18n::t("err-no-symbol-edit")))?;
    let changed = st.history.undo_count() > 0;
    let edited = Arc::clone(&st.doc);
    if !discard && changed && s.document(link.parent).is_some() {
        s.modify_document(
            link.parent,
            "Symbol bearbeiten",
            "modify.symbol.exitEdit",
            |p| {
                let doc = Arc::make_mut(&mut p.doc);
                let root = edited
                    .pages
                    .first()
                    .map(|pg| pg.root.clone())
                    .ok_or_else(|| EngineError::Failed(fp_i18n::t("err-view-empty")))?;
                // neue Symbole aus der Ansicht übernehmen (z. B. verschachtelt angelegte)
                for sy in &edited.symbols {
                    if doc.symbol_by_id(sy.id).is_none() {
                        doc.symbols.push(Arc::clone(sy));
                    }
                }
                let pos = doc
                    .symbols
                    .iter()
                    .position(|x| x.id == link.symbol)
                    .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-symbol")))?;
                let sym = Arc::make_mut(&mut doc.symbols[pos]);
                let bg = sym.root.background;
                sym.root = root;
                sym.root.background = bg;
                doc.ids.reserve_up_to(edited.ids.current());
                Ok(())
            },
        )?;
    }
    s.close_document(edit)?;
    if s.document(link.parent).is_some() {
        s.activate(link.parent)?;
        if let Some(i) = link.instance
            && let Ok(d) = s.doc_mut()
            && docops::find_on_page(&d.doc, d.page, i).is_some()
        {
            d.selection.set(vec![i]);
        }
        let id = link.parent;
        s.emit(SessionEvent::SelectionChanged(id));
    }
    Ok(link.parent)
}

fn selected_instance(s: &Session) -> Option<(fp_model::ElementId, SymbolId, fp_geom::Matrix3)> {
    let d = s.active()?;
    selected_elements(d)
        .into_iter()
        .find_map(|(_, e)| match &e.kind {
            ElementKind::Instance(i) => Some((e.id, i.symbol_id, i.transform)),
            _ => None,
        })
}

pub(super) fn register(r: &mut CommandRegistry) {
    for (name, in_place) in [
        ("modify.symbol.editSymbol", false),
        ("modify.symbol.editInPlace", true),
    ] {
        r.add(
            cmd(name)
                .requires(&[Requirement::Document])
                .opt(
                    "symbol",
                    ParamKind::Integer {
                        min: Some(0),
                        max: None,
                    },
                    "Symbol-ID (Vorgabe: Symbol der ausgewählten Instanz)",
                )
                .available(|s| selected_instance(s).is_some())
                .run(move |s, p| {
                    let (symbol, inst) = match p.opt_i64("symbol")? {
                        Some(v) => (SymbolId(v.max(0) as u32), None),
                        None => {
                            let (id, sym, m) = selected_instance(s).ok_or_else(|| {
                                EngineError::Selection(fp_i18n::t("err-sel-no-instance"))
                            })?;
                            (sym, Some((id, m)))
                        }
                    };
                    let id = begin_edit(s, symbol, in_place, inst)?;
                    Ok(CommandOutput::session_with(
                        json!({"doc": id.0, "symbol": symbol.0, "inPlace": in_place}),
                    ))
                }),
        );
    }
    r.add(
        cmd("modify.symbol.exitEdit")
            .label("Symbolbearbeitung beenden")
            .requires(&[Requirement::Document])
            .opt_default(
                "discard",
                ParamKind::Bool,
                json!(false),
                "Änderungen verwerfen",
            )
            .available(|s| s.active().is_some_and(|d| d.symbol_edit.is_some()))
            .run(|s, p| {
                let edit = s.doc()?.id;
                let parent = finish_edit(s, edit, p.bool_or("discard", false)?)?;
                Ok(CommandOutput::session_with(json!({"doc": parent.0})))
            }),
    );
}

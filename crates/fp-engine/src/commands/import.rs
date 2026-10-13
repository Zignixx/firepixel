//! Datei ▸ Importieren (fp-io): Inhalt einer Datei (FW-PNG, PSD, SVG, GIF, JPEG, PNG, TIFF, WebP …)
//! in das aktive Dokument übernehmen – als ein Verlaufsschritt, ausgewählt.
//!
//! Regeln: Elemente der ersten Seite (Status 1) des importierten Dokuments; mehrere Ebenen werden je
//! Ebene zu einer Gruppe mit dem Ebenennamen (Struktur bleibt erhalten), eine Ebene wird direkt
//! eingefügt. Symbole des Imports kommen in die Dokumentbibliothek; kollidierende Symbol-IDs werden
//! neu vergeben und die Instanzen umgehängt. Lage: Parameter `at` (linke obere Ecke des Inhalts)
//! bzw. die Lage in der Quelldatei.

use super::{insert_new, with_doc, DOC};
use crate::command::{cmd, CommandOutput, CommandRegistry, ParamKind};
use crate::docops;
use crate::error::EngineError;
use fp_model::element::{Element, ElementKind};
use fp_model::{Document, LayerKind, LayerSlot, SymbolId};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

/// Hängt Instanzen (rekursiv) auf neue Symbol-IDs um.
pub fn remap_instances(e: &mut Element, map: &HashMap<SymbolId, SymbolId>) {
    match &mut e.kind {
        ElementKind::Instance(i) => {
            if let Some(n) = map.get(&i.symbol_id) {
                i.symbol_id = *n;
            }
        }
        ElementKind::Group(g) => {
            if let Some(m) = g.mask.as_mut() {
                remap_instances(Element::edit(m), map);
            }
            for c in g.children.iter_mut() {
                remap_instances(Element::edit(c), map);
            }
        }
        ElementKind::CompoundShape(c) => {
            for o in c.operands.iter_mut() {
                remap_instances(Element::edit(o), map);
            }
        }
        ElementKind::SubLayer(l) => {
            let l = Arc::make_mut(l);
            for cel in l.cels.iter_mut() {
                for c in Arc::make_mut(cel).elements.iter_mut() {
                    remap_instances(Element::edit(c), map);
                }
            }
        }
        _ => {}
    }
}

fn remap_root(root: &mut fp_model::LayerRoot, map: &HashMap<SymbolId, SymbolId>) {
    for slot in root.layers.iter_mut() {
        if let LayerSlot::Own(l) = slot {
            let l = Arc::make_mut(l);
            for cel in l.cels.iter_mut() {
                for c in Arc::make_mut(cel).elements.iter_mut() {
                    remap_instances(Element::edit(c), map);
                }
            }
        }
    }
}

/// Übernimmt die Symbole von `src` in `dst`; liefert die Umbenennung kollidierender IDs.
pub fn merge_symbols(dst: &mut Document, src: &Document) -> HashMap<SymbolId, SymbolId> {
    let mut map = HashMap::new();
    let mut next = dst.new_symbol_id().0;
    for s in &src.symbols {
        if dst.symbol_by_id(s.id).is_some() || map.values().any(|v: &SymbolId| *v == s.id) {
            map.insert(s.id, SymbolId(next));
            next += 1;
        }
    }
    for s in &src.symbols {
        let mut sym = (**s).clone();
        if let Some(n) = map.get(&s.id) {
            sym.id = *n;
        }
        remap_root(&mut sym.root, &map);
        dst.symbols.push(Arc::new(sym));
    }
    map
}

/// Elemente zum Einfügen aus einem importierten Dokument (oben → unten).
pub fn import_elements(dst: &mut Document, src: &Document) -> Vec<Element> {
    let map = merge_symbols(dst, src);
    let Some(page) = src.pages.first() else { return Vec::new() };
    // Ebenen oben → unten (Liste ist unten → oben)
    let layers: Vec<_> = page
        .root
        .layers
        .iter()
        .rev()
        .filter_map(|s| match s {
            LayerSlot::Own(l) if l.kind != LayerKind::Web => Some(Arc::clone(l)),
            _ => None,
        })
        .filter(|l| l.cels.first().is_some_and(|c| !c.elements.is_empty()))
        .collect();
    let mut out = Vec::new();
    let mut ids = std::mem::take(&mut dst.ids);
    for l in &layers {
        let Some(cel) = l.cels.first() else { continue };
        let mut els: Vec<Arc<Element>> = Vec::new();
        for e in &cel.elements {
            let mut d = e.duplicate(&mut ids);
            remap_instances(&mut d, &map);
            els.push(Arc::new(d));
        }
        if layers.len() == 1 {
            out.extend(els.into_iter().map(|e| (*e).clone()));
        } else {
            let mut g = Element::new_group(&mut ids, els);
            g.common.name = Some(l.name.clone());
            out.push(g);
        }
    }
    dst.ids = ids;
    out
}

pub(super) fn register(r: &mut CommandRegistry) {
    r.add(
        cmd("file.import")
            .requires(DOC)
            .param("path", ParamKind::Path, "Datei (FW-PNG, PSD, SVG, AI/EPS, GIF, JPEG, PNG, BMP, TIFF, TGA, WebP, JXL, QOI, ICO …)")
            .opt("at", ParamKind::Point, "linke obere Ecke des importierten Inhalts (Vorgabe: Lage in der Datei)")
            .js("importFile")
            .run(|s, p| {
                let path = std::path::PathBuf::from(p.str("path")?);
                let data = std::fs::read(&path)?;
                let res = fp_io::import_bytes(&path.to_string_lossy(), &data, &fp_io::ImportOptions::with_installation()).map_err(|e| EngineError::Format(e.to_string()))?;
                let at = p.opt_point("at")?;
                with_doc(s, |d| {
                    let doc = Arc::make_mut(&mut d.doc);
                    let mut els = import_elements(doc, &res.document);
                    if els.is_empty() {
                        return Err(EngineError::Failed(fp_i18n::t("err-file-no-objects")));
                    }
                    if let Some(at) = at {
                        // Einzelbild: `at` ist die linke obere Ecke des Bildes; das auf die sichtbaren
                        // Pixel beschnittene Objekt behält seinen Versatz darin (re\22 §1.2 Regel 1)
                        let b = if res.content == fp_io::ImportedContent::Bitmap {
                            Some(fp_geom::Rect::new(0.0, 0.0, 0.0, 0.0))
                        } else {
                            els.iter().filter_map(|e| docops::element_bounds(doc, e)).reduce(|a, b| a.union(&b))
                        };
                        if let Some(b) = b {
                            let (dx, dy) = (at.x - b.left, at.y - b.top);
                            for e in els.iter_mut() {
                                docops::translate_element(e, dx, dy);
                            }
                        }
                    }
                    let mut new_ids = Vec::new();
                    // unterstes zuerst einfügen, damit die Reihenfolge erhalten bleibt
                    for e in els.into_iter().rev() {
                        new_ids.push(insert_new(d, e)?);
                    }
                    new_ids.reverse();
                    d.selection.set(new_ids.clone());
                    Ok(CommandOutput::doc_with(json!({"ids": super::ids_json(&new_ids), "format": format!("{:?}", res.format), "warnings": res.warnings})).labeled(fp_i18n::t("msg-step-import")))
                })
            }),
    );
}

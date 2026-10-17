//! Symbolursprung (L-3, re 28 S2, re 13 T1): Beim Umwandeln liegt der Ursprung eines Symbols in der
//! Mitte seiner Inhaltsgrenzen, die Instanz bekommt `MTXv = ATMv = T(Mitte)`. Firepixel-Dateien vor
//! L-3 legten den Ursprung in die Dokumentkoordinaten (Inhalt unverschoben, Instanz mit
//! Einheitsmatrix). Solche Dateien erkennt [`is_legacy`] am `tEXt Software` (genau
//! [`fp_fwpng::SOFTWARE_LEGACY`]); [`upgrade_legacy`] rechnet ihre Symbole auf den mittigen Ursprung
//! um, ohne dass sich auf der Leinwand etwas ändert. Dateien des Originals (anderer `Software`-Text)
//! bleiben unberührt und werden unverändert geschrieben.

use crate::docops;
use fp_geom::{Matrix3, Rect};
use fp_model::element::ElementKind;
use fp_model::layer::{LayerKind, LayerSlot};
use fp_model::{Document, SymbolId};
use std::collections::HashMap;
use std::sync::Arc;

/// Symbolursprung beim Umwandeln (in Elternkoordinaten): die Mitte der Objektgrenzen `b`
/// (re 13 T1: Rechteck 100 × 50 bei 100,100 → `T(150,125)`, Symbolgrenzen −50…50/−25…25;
/// re 28 S2: 0,0–50,50 → −25,−25, 50 × 50). M10 (re 43 §5): `b` sind die Pfadgrenzen ohne Strich
/// und Effekte (Fließkomma, [`docops::element_bounds`]), die Mitte wird je Achse halb weg von null
/// auf ganze Pixel gerundet (125,5 → 126, −25,5 → −26, −26,5 → −27; 325,3 → 325).
pub fn origin_for(b: Rect) -> (f32, f32) {
    let (cx, cy) = (
        (b.left as f64 + b.right as f64) / 2.0,
        (b.top as f64 + b.bottom as f64) / 2.0,
    );
    // `f64::round` rundet halb weg von null
    (cx.round() as f32, cy.round() as f32)
}

/// Liest eine FW-PNG und rechnet Symbole älterer Firepixel-Dateien um ([`upgrade_legacy`]).
pub fn read_fwpng(bytes: &[u8]) -> fp_fwpng::Result<Document> {
    let mut doc = fp_fwpng::read(bytes)?;
    upgrade_legacy(&mut doc);
    Ok(doc)
}

/// Stammt das Dokument aus einer Firepixel-Datei mit Symbolursprung in Dokumentkoordinaten?
pub fn is_legacy(doc: &Document) -> bool {
    doc.file_meta
        .text_chunks
        .iter()
        .any(|(k, v)| k == "Software" && v == fp_fwpng::SOFTWARE_LEGACY)
}

/// Objektgrenzen des Inhalts eines Symbols (alle Status, normale Ebenen), in Symbolkoordinaten.
pub fn symbol_content_bounds(doc: &Document, sym: &fp_model::Symbol) -> Option<Rect> {
    let mut r: Option<Rect> = None;
    for slot in &sym.root.layers {
        let LayerSlot::Own(l) = slot else { continue };
        if l.kind == LayerKind::Web {
            continue;
        }
        for cel in &l.cels {
            for e in &cel.elements {
                if let Some(b) = docops::element_bounds(doc, e) {
                    r = Some(match r {
                        Some(a) => a.union(&b),
                        None => b,
                    });
                }
            }
        }
    }
    r
}

/// Rechnet die Symbole einer älteren Firepixel-Datei auf den mittigen Ursprung um und setzt die
/// Kennung auf [`fp_fwpng::SOFTWARE`]. Jede Symbolmitte `c` (gerundet wie beim Umwandeln) wird
/// vorab aus dem unveränderten Dokument bestimmt; dann wird der Inhalt um −c verschoben und jede
/// Instanz (Seiten, Symbole, geteilte Ebenen) bekommt `T(c)·MTXv` (ebenso `ATMv`), 9-Slice-
/// Hilfslinien und Bezugsrechteck wandern mit. Die Darstellung bleibt gleich. Liefert, ob die
/// Datei als alt erkannt wurde.
pub fn upgrade_legacy(doc: &mut Document) -> bool {
    if !is_legacy(doc) {
        return false;
    }
    let mut shift: HashMap<SymbolId, (usize, f32, f32)> = HashMap::new();
    for (k, sym) in doc.symbols.iter().enumerate() {
        if let Some(b) = symbol_content_bounds(doc, sym) {
            let (cx, cy) = origin_for(b);
            if cx != 0.0 || cy != 0.0 {
                shift.insert(sym.id, (k, cx, cy));
            }
        }
    }
    if !shift.is_empty() {
        // Instanzmatrizen (links mit T(c) verknüpfen); Adressen vor dem Verschieben sammeln, die
        // Baumstruktur ändert sich dabei nicht
        let mut paths = Vec::new();
        doc.walk_elements(|_, path, e| {
            if let ElementKind::Instance(i) = &e.kind
                && let Some(&(_, cx, cy)) = shift.get(&i.symbol_id)
            {
                paths.push((path.clone(), cx, cy));
            }
        });
        for (path, cx, cy) in paths {
            if let Some(e) = doc.element_at_mut(&path)
                && let ElementKind::Instance(i) = &mut e.kind
            {
                let t = Matrix3::translation(cx, cy);
                i.transform = t.then(&i.transform);
                if let Some(a) = &mut i.alt_transform {
                    *a = t.then(a);
                }
            }
        }
        // Inhalt der Symbole um −c verschieben (verschachtelte Instanzen: rechts mit T(−c))
        for &(k, cx, cy) in shift.values() {
            let Some(s) = doc.symbols.get_mut(k) else {
                continue;
            };
            let sym = Arc::make_mut(s);
            for slot in &mut sym.root.layers {
                let LayerSlot::Own(l) = slot else { continue };
                let l = Arc::make_mut(l);
                for cel in &mut l.cels {
                    let cel = Arc::make_mut(cel);
                    for e in &mut cel.elements {
                        docops::translate_element(Arc::make_mut(e), -cx, -cy);
                    }
                }
            }
            if let Some(n) = &mut sym.nine_slice {
                for (j, g) in n.guides.iter_mut().enumerate() {
                    // Reihenfolge oben, unten, links, rechts
                    *g -= if j < 2 { cy as f64 } else { cx as f64 };
                }
                n.bounds = n.bounds.map(|r| r.translate(-cx, -cy));
                n.view = n.view.map(|r| r.translate(-cx, -cy));
            }
        }
        doc.invalidate_root_copy();
    }
    for (k, v) in &mut doc.file_meta.text_chunks {
        if k == "Software" && v == fp_fwpng::SOFTWARE_LEGACY {
            *v = fp_fwpng::SOFTWARE.to_string();
        }
    }
    true
}

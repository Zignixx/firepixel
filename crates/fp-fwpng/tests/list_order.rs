//! Reihenfolge bearbeiteter Listen (Befund EB): eine nach dem Lesen eingefügte Kopie bleibt an ihrer
//! Stelle der Stapelfolge, auch wenn die gelesene Schlüsselfolge (`Extra.order`) etwas anderes sagt.

use std::sync::Arc;

use fp_color::Argb;
use fp_geom::{Point, Rect};
use fp_model::{Document, Element, ElementKind, LayerSlot, PathAttrs};

fn kinds(doc: &Document) -> Vec<&'static str> {
    let LayerSlot::Own(l) = &doc.pages[0].root.layers[0] else { return vec![] };
    l.cels[0]
        .elements
        .iter()
        .map(|e| match e.kind {
            ElementKind::Path(_) => "pfad",
            ElementKind::Text(_) => "text",
            _ => "anderes",
        })
        .collect()
}

#[test]
fn inserted_element_keeps_its_place() {
    let mut doc = Document::new(100.0, 80.0, Argb(0xffff_ffff));
    let r1 = Element::new_rectangle(&mut doc.ids, Rect::new(0.0, 0.0, 10.0, 10.0), 0.0, PathAttrs::solid_fill(Argb(0xff00_00ff)));
    let r2 = Element::new_rectangle(&mut doc.ids, Rect::new(20.0, 0.0, 30.0, 10.0), 0.0, PathAttrs::solid_fill(Argb(0xff00_ff00)));
    let t = Element::new_text(&mut doc.ids, "A", "ArialMT", 12.0, Argb(0xff00_0000), Point::new(5.0, 40.0));
    {
        let p = Arc::make_mut(&mut doc.pages[0]);
        if let LayerSlot::Own(l) = &mut p.root.layers[0] {
            Arc::make_mut(&mut Arc::make_mut(l).cels[0]).elements = vec![Arc::new(t), Arc::new(r1), Arc::new(r2)];
        }
    }
    let bytes = fp_fwpng::write(&doc, &fp_fwpng::WriteOptions::default(), &fp_fwpng::NoFlattener).unwrap();
    let mut back = fp_fwpng::read(&bytes).unwrap();
    assert_eq!(kinds(&back), ["text", "pfad", "pfad"]);
    // oben eine Pfadkopie einfügen und den Text nach unten schieben (Bearbeitung nach dem Lesen)
    {
        let p = Arc::make_mut(&mut back.pages[0]);
        if let LayerSlot::Own(l) = &mut p.root.layers[0] {
            let cel = Arc::make_mut(&mut Arc::make_mut(l).cels[0]);
            let t = cel.elements.remove(0);
            let copy = cel.elements[0].clone();
            cel.elements.insert(0, copy);
            cel.elements.push(t);
        }
    }
    let bytes2 = fp_fwpng::write(&back, &fp_fwpng::WriteOptions::default(), &fp_fwpng::NoFlattener).unwrap();
    let again = fp_fwpng::read(&bytes2).unwrap();
    assert_eq!(kinds(&again), ["pfad", "pfad", "pfad", "text"]);
}

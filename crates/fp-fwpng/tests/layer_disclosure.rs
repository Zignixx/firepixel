//! R-32: the layer disclosure (`DISb`, the Layers panel's expanded/collapsed state) is document
//! data. A collapsed layer and a collapsed sublayer come back collapsed, expanded ones expanded.

use std::sync::Arc;

use fp_color::Argb;
use fp_fwpng::{NoFlattener, WriteOptions, read, write};
use fp_geom::Rect;
use fp_model::{Document, Element, ElementKind, Layer, LayerKind, LayerSlot, PathAttrs};

/// Document with "Ebene 1" (a rectangle and the sublayer "Unterebene" with a rectangle) and
/// "Ebene 2" (a rectangle); `open1`, `open_sub`, `open2` are their disclosure flags.
fn doc(open1: bool, open_sub: bool, open2: bool) -> Document {
    let mut doc = Document::new(200.0, 100.0, Argb(0xffff_ffff));
    let mut ids = std::mem::take(&mut doc.ids);
    let rect = |ids: &mut fp_model::IdGen, x: f32| {
        Arc::new(Element::new_rectangle(
            ids,
            Rect::new(x, 10.0, x + 20.0, 30.0),
            0.0,
            PathAttrs::solid_fill(Argb(0xffff_0000)),
        ))
    };
    let mut sub = Layer::new(&mut ids, "Unterebene", LayerKind::Normal, 1);
    sub.level = 1;
    sub.expanded = open_sub;
    let inner = rect(&mut ids, 50.0);
    Arc::make_mut(&mut sub.cels[0]).elements.push(inner);
    let sub_el = Arc::new(Element::new(
        ids.element(),
        ElementKind::SubLayer(Arc::new(sub)),
    ));
    let r1 = rect(&mut ids, 10.0);
    let mut l2 = Layer::new(&mut ids, "Ebene 2", LayerKind::Normal, 1);
    l2.expanded = open2;
    let r2 = rect(&mut ids, 100.0);
    Arc::make_mut(&mut l2.cels[0]).elements.push(r2);
    doc.ids = ids;
    let page = doc.page_mut(0).expect("page");
    if let LayerSlot::Own(l) = &mut page.root.layers[0] {
        let l = Arc::make_mut(l);
        l.expanded = open1;
        let cel = Arc::make_mut(&mut l.cels[0]);
        cel.elements.push(r1);
        cel.elements.push(sub_el);
    }
    page.root.layers.push(LayerSlot::Own(Arc::new(l2)));
    doc
}

/// Disclosure of (layer 1, its sublayer, layer 2) in a read-back document.
fn flags(d: &Document) -> (bool, bool, bool) {
    let own: Vec<&Arc<Layer>> = d.pages[0]
        .root
        .layers
        .iter()
        .filter_map(|s| match s {
            LayerSlot::Own(l) if l.kind == LayerKind::Normal => Some(l),
            _ => None,
        })
        .collect();
    let l1 = own.iter().find(|l| l.name == "Ebene 1").expect("Ebene 1");
    let l2 = own.iter().find(|l| l.name == "Ebene 2").expect("Ebene 2");
    let sub = l1.cels[0]
        .elements
        .iter()
        .find_map(|e| match &e.kind {
            ElementKind::SubLayer(s) => Some(s.expanded),
            _ => None,
        })
        .expect("sublayer");
    (l1.expanded, sub, l2.expanded)
}

#[test]
fn layer_disclosure_round_trips() {
    for want in [
        (false, true, true),
        (true, false, true),
        (true, true, false),
        (false, false, false),
        (true, true, true),
    ] {
        let d = doc(want.0, want.1, want.2);
        let bytes = write(&d, &WriteOptions::default(), &NoFlattener).expect("write");
        let back = read(&bytes).expect("read");
        assert_eq!(
            flags(&back),
            want,
            "disclosure (Ebene 1, Unterebene, Ebene 2)"
        );
        // and once more through the written file
        let bytes2 = write(&back, &WriteOptions::default(), &NoFlattener).expect("write again");
        assert_eq!(flags(&read(&bytes2).expect("read again")), want);
    }
}

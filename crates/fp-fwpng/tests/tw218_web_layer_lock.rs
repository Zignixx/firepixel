//! TW-218 (RE spec 65 rule 4): the file keeps the web layer's lock (`LAYv` with `URLb 1` →
//! `CLLv/CELv/LCKb 1`), but the reference application drops it on open. A locked normal layer and
//! a locked sublayer keep their lock; written again, the web layer's cel says `LCKb 0`.

use std::sync::Arc;

use fp_color::Argb;
use fp_fwpng::{NoFlattener, WriteOptions, read, read_raw, write};
use fp_geom::Rect;
use fp_model::{
    Document, Element, ElementKind, Layer, LayerKind, LayerSlot, PathAttrs, RawNode, RawValue,
};

/// "Ebene 1" with a rectangle and the locked sublayer "S1" (with a rectangle), the locked
/// "Ebene 2" and the locked web layer.
fn doc() -> Document {
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
    let mut sub = Layer::new(&mut ids, "S1", LayerKind::Normal, 1);
    sub.level = 1;
    let inner = rect(&mut ids, 50.0);
    {
        let c = Arc::make_mut(&mut sub.cels[0]);
        c.elements.push(inner);
        c.locked = true;
    }
    let sub_el = Arc::new(Element::new(
        ids.element(),
        ElementKind::SubLayer(Arc::new(sub)),
    ));
    let r1 = rect(&mut ids, 10.0);
    let mut l2 = Layer::new(&mut ids, "Ebene 2", LayerKind::Normal, 1);
    Arc::make_mut(&mut l2.cels[0]).locked = true;
    doc.ids = ids;
    let page = doc.page_mut(0).expect("page");
    for slot in &mut page.root.layers {
        let LayerSlot::Own(l) = slot else { continue };
        let l = Arc::make_mut(l);
        if l.kind == LayerKind::Web {
            for c in &mut l.cels {
                Arc::make_mut(c).locked = true;
            }
        } else {
            let cel = Arc::make_mut(&mut l.cels[0]);
            cel.elements.push(r1.clone());
            cel.elements.push(sub_el.clone());
        }
    }
    let web_at = page
        .root
        .layers
        .iter()
        .position(|s| matches!(s, LayerSlot::Own(l) if l.kind == LayerKind::Web))
        .unwrap_or(page.root.layers.len());
    page.root
        .layers
        .insert(web_at, LayerSlot::Own(Arc::new(l2)));
    doc
}

/// Locks of (web layer, "Ebene 2", sublayer "S1") in a read document.
fn locks(d: &Document) -> (bool, bool, bool) {
    let mut out = (None, None, None);
    for slot in &d.pages[0].root.layers {
        let LayerSlot::Own(l) = slot else { continue };
        let locked = l.cels.iter().any(|c| c.locked);
        if l.kind == LayerKind::Web {
            out.0 = Some(locked);
        } else if l.name == "Ebene 2" {
            out.1 = Some(locked);
        } else {
            for e in &l.cels[0].elements {
                if let ElementKind::SubLayer(s) = &e.kind {
                    out.2 = Some(s.cels.iter().any(|c| c.locked));
                }
            }
        }
    }
    (
        out.0.expect("web layer"),
        out.1.expect("Ebene 2"),
        out.2.expect("S1"),
    )
}

fn child<'a>(n: &'a [RawNode], key: &str) -> Option<&'a RawValue> {
    n.iter()
        .find(|c| c.key.as_string() == key)
        .map(|c| &c.value)
}

/// `LCKb` of every cel of every web layer (`LAYv` with `URLb 1`) in the tag tree, root and page
/// copies alike.
fn web_cel_locks(nodes: &[RawNode], out: &mut Vec<bool>) {
    for n in nodes {
        let RawValue::List(kids) = &n.value else {
            continue;
        };
        if n.key.as_string() == "LAYv"
            && matches!(child(kids, "URLb"), Some(RawValue::Bool(true)))
            && let Some(RawValue::List(cl)) = child(kids, "CLLv")
        {
            for cel in cl {
                if let RawValue::List(c) = &cel.value {
                    out.push(matches!(child(c, "LCKb"), Some(RawValue::Bool(true))));
                }
            }
        }
        web_cel_locks(kids, out);
    }
}

fn written_web_locks(bytes: &[u8]) -> Vec<bool> {
    let raw = read_raw(bytes, false).expect("raw");
    let mut v = Vec::new();
    web_cel_locks(raw.ts.as_deref().unwrap_or(&[]), &mut v);
    v
}

#[test]
fn web_layer_lock_is_written_but_dropped_on_open() {
    let d = doc();
    assert_eq!(locks(&d), (true, true, true));
    let bytes = write(&d, &WriteOptions::default(), &NoFlattener).expect("write");
    let w = written_web_locks(&bytes);
    assert!(
        !w.is_empty() && w.iter().all(|l| *l),
        "the file keeps LCKb 1 on the web layer: {w:?}"
    );
    let back = read(&bytes).expect("read");
    assert_eq!(
        locks(&back),
        (false, true, true),
        "web layer unlocked, Ebene 2 and S1 locked"
    );
    // saved again (k1_resave): the web layer's cel says LCKb 0, the others keep their lock
    let bytes2 = write(&back, &WriteOptions::default(), &NoFlattener).expect("write again");
    let w2 = written_web_locks(&bytes2);
    assert!(
        !w2.is_empty() && w2.iter().all(|l| !*l),
        "LCKb 0 after the round trip: {w2:?}"
    );
    assert_eq!(
        locks(&read(&bytes2).expect("read again")),
        (false, true, true)
    );
}

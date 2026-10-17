//! K-9 (re\37 §6, rest of U-1): the SVG reader keeps the layers of the file. Top-level `<g>` with an
//! `id` below `<svg>` are layers, named by `inkscape:label` or else the `id`; "consider layers" then
//! creates one new, named layer per file layer, filled only in the current state (like the AI
//! reader). A file with a single layer creates no new layer (assumption, like AI). PDF: optional
//! content groups do not survive the hayro → SVG path, a PDF stays one layer (limit, report k9.md).

use crate::common::*;
use serde_json::json;
use std::path::PathBuf;

const TWO_GROUPS: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" width="120" height="90" viewBox="0 0 120 90">
<g id="layer1" inkscape:groupmode="layer" inkscape:label="Unten"><rect x="10" y="10" width="50" height="50" fill="#ff0000"/></g>
<g id="Oben"><rect x="40" y="20" width="50" height="60" fill="#0000ff"/><rect x="0" y="0" width="5" height="5" fill="#00ff00"/></g>
</svg>"##;

const ONE_GROUP: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="90">
<g id="only"><rect x="10" y="10" width="50" height="50" fill="#ff0000"/><rect x="40" y="20" width="50" height="60" fill="#0000ff"/></g>
</svg>"##;

/// Own file per call (tests run in parallel, see u1_vector_layers).
fn svg_file(svg: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("fp-k9-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    let p = d.join(format!("layers_{n}.svg"));
    std::fs::write(&p, svg).expect("write");
    p
}

/// Document 300 × 200 with 3 states, current state 2 (index 1).
fn session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    ex(&mut s, "file.new", json!({"width": 300, "height": 200, "background": "#ffffff"}));
    ex(&mut s, "state.add", json!({"count": 2}));
    ex(&mut s, "state.setCurrent", json!({"index": 1}));
    s
}

/// Normal layers bottom → top: (name, element count per state).
fn layers(s: &fp_engine::Session) -> Vec<(String, Vec<usize>)> {
    let d = s.active().expect("doc");
    fp_engine::docops::own_layers(&d.doc, d.page)
        .into_iter()
        .filter(|(_, l)| l.kind == fp_model::LayerKind::Normal)
        .map(|(_, l)| (l.name.clone(), l.cels.iter().map(|c| c.elements.len()).collect()))
        .collect()
}

fn reader_layers(svg: &str) -> Vec<(String, usize)> {
    let res = fp_io::import_bytes("t.svg", svg.as_bytes(), &fp_io::ImportOptions::default()).expect("SVG");
    res.document.pages[0]
        .root
        .layers
        .iter()
        .filter_map(|s| match s {
            fp_model::LayerSlot::Own(l) if l.kind == fp_model::LayerKind::Normal => Some((l.name.clone(), l.cels[0].elements.len())),
            _ => None,
        })
        .collect()
}

#[test]
fn reader_keeps_top_level_groups_as_layers() {
    // inkscape:label wins over the id; the viewBox wrapper of usvg is looked through
    assert_eq!(reader_layers(TWO_GROUPS), vec![("Unten".to_string(), 1), ("Oben".to_string(), 2)]);
    // a single group stays one layer with the default name
    let one = reader_layers(ONE_GROUP);
    assert_eq!(one.len(), 1, "{one:?}");
    assert_ne!(one[0].0, "only", "single file layer keeps the default layer name");
}

#[test]
fn reader_puts_loose_objects_between_layers_into_own_layers() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
<rect x="0" y="0" width="5" height="5"/>
<g id="A"><rect x="10" y="10" width="5" height="5"/></g>
<rect x="20" y="20" width="5" height="5"/><rect x="30" y="30" width="5" height="5"/>
<g id="B"><rect x="40" y="40" width="5" height="5"/></g>
</svg>"##;
    let l = reader_layers(svg);
    let counts: Vec<usize> = l.iter().map(|(_, c)| *c).collect();
    assert_eq!(counts, vec![1, 1, 2, 1], "{l:?}");
    assert_eq!((l[1].0.as_str(), l[3].0.as_str()), ("A", "B"));
}

#[test]
fn consider_layers_creates_named_layers_in_current_state() {
    let p = svg_file(TWO_GROUPS);
    for v in [None, Some(json!({"layers": 1}))] {
        let mut s = session();
        let before = layers(&s);
        let mut args = json!({"path": p.to_string_lossy(), "at": [20, 20]});
        if let Some(v) = &v {
            args["vector"] = v.clone();
        }
        let out = ex(&mut s, "file.import", args);
        assert_eq!(out["ids"].as_array().map(|a| a.len()), Some(3), "{v:?}: three new objects");
        let after = layers(&s);
        assert_eq!(after.len(), before.len() + 2, "{v:?}: two new layers {after:?}");
        assert_eq!(&after[..before.len()], &before[..], "{v:?}: existing layers untouched");
        let new = &after[before.len()..];
        assert_eq!(new[0], ("Unten".to_string(), vec![0, 1, 0]), "{v:?}");
        assert_eq!(new[1], ("Oben".to_string(), vec![0, 2, 0]), "{v:?}");
        let d = s.active().expect("doc");
        assert_eq!((d.frame_count(), d.frame), (3, 1), "{v:?}: states stay");
        ex(&mut s, "edit.undo", json!({}));
        assert_eq!(layers(&s), before, "{v:?}: one undo step");
    }
}

#[test]
fn single_layer_and_ignore_layers_add_no_layer() {
    // single file layer: no new layer (assumption like AI)
    let p = svg_file(ONE_GROUP);
    let mut s = session();
    let before = layers(&s);
    ex(&mut s, "file.import", json!({"path": p.to_string_lossy(), "at": [20, 20]}));
    let after = layers(&s);
    assert_eq!(after.len(), before.len(), "{after:?}");
    // two layers, "ignore layers": everything into the current layer
    let p = svg_file(TWO_GROUPS);
    let mut s = session();
    let before = layers(&s);
    ex(&mut s, "file.import", json!({"path": p.to_string_lossy(), "at": [20, 20], "vector": {"layers": 0}}));
    let after = layers(&s);
    assert_eq!(after.len(), before.len(), "{after:?}");
    assert_eq!(after.iter().map(|(_, c)| c[1]).sum::<usize>(), 3, "{after:?}");
}

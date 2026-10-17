//! U-1 (re\37 §6): importing a vector file with "consider layers" creates one new, named layer per
//! layer of the file, filled only in the current state; the state count and the current state stay.
//! "Layers to states" keeps the assumption top layer → current state, further layers → next states.

use crate::common::*;
use serde_json::json;
use std::path::PathBuf;

/// AI file with two layers "Unten" (red, 50 × 50) and "Oben" (blue, 50 × 70), like `m7_2ebenen.ai`.
/// Each call writes its own file: tests run in parallel, and a shared path could be read while
/// another test truncates and rewrites it (GT-0p: "AI/EPS ohne %%BoundingBox").
fn two_layer_ai() -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("fp-u1-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    let p = d.join(format!("two_layers_{n}.ai"));
    let ai = "%!PS-3.0\n%%BoundingBox:0 0 120 90\n%AI5_FileFormat 3\n%%EndComments\n%%EndSetup\n\
1 1 1 1 0 0 0 79 128 255 Lb\n(Unten) Ln\n0 1 1 0 k\n10 10 m\n60 10 l\n60 60 l\n10 60 l\n10 10 l\nf\nLB\n\
1 1 1 1 0 0 0 79 128 255 Lb\n(Oben) Ln\n1 1 0 0 k\n40 -40 m\n90 -40 l\n90 30 l\n40 30 l\n40 -40 l\nf\nLB\n%%PageTrailer\n%%Trailer\n";
    std::fs::write(&p, ai).expect("write");
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

fn state(s: &fp_engine::Session) -> (usize, usize) {
    let d = s.active().expect("doc");
    (d.frame_count(), d.frame)
}

#[test]
fn reader_keeps_layers_of_ai() {
    let p = two_layer_ai();
    let data = std::fs::read(&p).expect("read");
    let res = fp_io::import_bytes(&p.to_string_lossy(), &data, &fp_io::ImportOptions::default()).expect("import");
    let names: Vec<String> = res.document.pages[0]
        .root
        .layers
        .iter()
        .filter_map(|s| match s {
            fp_model::LayerSlot::Own(l) if l.kind == fp_model::LayerKind::Normal => Some(format!("{}:{}", l.name, l.cels[0].elements.len())),
            _ => None,
        })
        .collect();
    assert_eq!(names, vec!["Unten:1", "Oben:1"]);
}

#[test]
fn consider_layers_creates_named_layers_in_current_state() {
    let p = two_layer_ai();
    // script import without options (no dialog) and the explicit dialog option behave alike
    for v in [None, Some(json!({"layers": 1}))] {
        let mut s = session();
        let before = layers(&s);
        let mut args = json!({"path": p.to_string_lossy(), "at": [20, 20]});
        if let Some(v) = &v {
            args["vector"] = v.clone();
        }
        let out = ex(&mut s, "file.import", args);
        assert_eq!(out["ids"].as_array().map(|a| a.len()), Some(2), "{v:?}: two new objects");
        let after = layers(&s);
        assert_eq!(after.len(), before.len() + 2, "{v:?}: two new layers {after:?}");
        assert_eq!(&after[..before.len()], &before[..], "{v:?}: existing layers untouched");
        let new = &after[before.len()..];
        assert_eq!(new[0], ("Unten".to_string(), vec![0, 1, 0]), "{v:?}: lower layer only in state 2");
        assert_eq!(new[1], ("Oben".to_string(), vec![0, 1, 0]), "{v:?}: upper layer on top, only in state 2");
        assert_eq!(state(&s), (3, 1), "{v:?}: state count and current state stay");
        assert_eq!(s.active().expect("doc").selection.elements.len(), 2, "{v:?}: imported objects selected");
        // one undo step removes everything again
        ex(&mut s, "edit.undo", json!({}));
        assert_eq!(layers(&s), before, "{v:?}: undo");
    }
}

#[test]
fn ignore_layers_uses_current_layer() {
    let p = two_layer_ai();
    let mut s = session();
    let before = layers(&s);
    ex(&mut s, "file.import", json!({"path": p.to_string_lossy(), "at": [20, 20], "vector": {"layers": 0}}));
    let after = layers(&s);
    assert_eq!(after.len(), before.len(), "no new layer");
    assert_eq!(after.iter().map(|(_, c)| c[1]).sum::<usize>(), 2, "both objects in state 2: {after:?}");
    assert_eq!(state(&s), (3, 1));
}

#[test]
fn layers_to_states_keeps_assumption() {
    let p = two_layer_ai();
    let mut s = session();
    let before = layers(&s);
    ex(&mut s, "file.import", json!({"path": p.to_string_lossy(), "at": [20, 20], "vector": {"layers": 2}}));
    let after = layers(&s);
    assert_eq!(after.len(), before.len(), "no new layer");
    let per_state: Vec<usize> = (0..3).map(|f| after.iter().map(|(_, c)| c[f]).sum()).collect();
    // assumption: top layer → current state (2), next layer → state 3
    assert_eq!(per_state, vec![0, 1, 1], "{after:?}");
    assert_eq!(state(&s), (3, 1));
}

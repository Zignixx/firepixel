//! FB-152 (F158, Firepixel extension): `dom.ungroupMaskCompletely()` (a Firepixel-own method beside
//! the reference API) runs *Modify ▸ Ungroup Mask Completely*: the mask object and the contents of
//! a mask group come out as loose objects in one step.

use fp_script::{ScriptOptions, run_source};
use serde_json::json;

#[test]
fn ungroup_mask_completely_from_a_script() {
    let mut s = fp_engine::Session::default();
    s.execute(
        "file.new",
        json!({"width": 300, "height": 200, "background": "#ffffff"}),
    )
    .expect("document");
    let mut ids = Vec::new();
    for r in [[20, 40, 200, 160], [120, 60, 280, 140], [10, 10, 290, 190]] {
        ids.push(
            s.execute("draw.rectangle", json!({"rect": r}))
                .expect("rectangle")
                .value["id"]
                .clone(),
        );
    }
    s.execute("select.set", json!({"ids": [ids[2], ids[1], ids[0]]}))
        .expect("select");
    s.execute("modify.mask.groupAsMask", json!({}))
        .expect("mask");
    let o = run_source(
        &mut s,
        "var d = fw.getDocumentDOM(); d.ungroupMaskCompletely(); fw.selection.length;",
        &ScriptOptions::default(),
    );
    assert!(
        o.ok,
        "error: {:?}\ndiagnostics: {:?}",
        o.error, o.diagnostics
    );
    assert_eq!(o.result.as_deref(), Some("3"));
    let d = s.active().expect("doc");
    let top: Vec<u64> = d.doc.pages[d.page]
        .root
        .layers
        .iter()
        .filter_map(|l| match l {
            fp_model::LayerSlot::Own(l) => Some(l),
            _ => None,
        })
        .flat_map(|l| l.cels[0].elements.iter().map(|e| e.id.0))
        .collect();
    let want: Vec<u64> = ids.iter().rev().filter_map(|v| v.as_u64()).collect();
    assert_eq!(top, want, "mask object, then the two contents, no group");
}

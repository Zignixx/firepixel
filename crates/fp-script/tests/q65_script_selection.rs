//! RE spec 64 §3 rule 4 from scripts (S23, S23b): `fw.selection = [e]` and `selectAllOnLayer`
//! with one object make that object's layer current (`currentLayerNum`); two objects keep it; a
//! layer set by the script after selecting stays.

use fp_script::{ScriptOptions, run_source};
use serde_json::json;

/// L1 (bottom) with r1, L2 with r2, L3 with r3; L3 current (RE spec 64 `mkn`).
fn session() -> fp_engine::Session {
    fp_engine::shell::disable();
    fp_i18n::profile::use_test_profile();
    let mut s = fp_engine::Session::default();
    let mut ex = |n: &str, p: serde_json::Value| {
        s.execute(n, p).unwrap_or_else(|e| panic!("{n}: {e}"));
    };
    ex(
        "file.new",
        json!({"width": 600, "height": 400, "background": "#ffffff"}),
    );
    ex("draw.rectangle", json!({"rect": [50, 50, 130, 110]}));
    ex("layer.add", json!({"name": "L2"}));
    ex("draw.rectangle", json!({"rect": [200, 100, 280, 160]}));
    ex("layer.add", json!({"name": "L3"}));
    ex("draw.rectangle", json!({"rect": [350, 150, 430, 210]}));
    ex("select.none", json!({}));
    s
}

fn result(s: &mut fp_engine::Session, src: &str) -> String {
    let o = run_source(s, src, &ScriptOptions::default());
    assert!(o.ok, "error: {:?}\nscript: {src}", o.error);
    o.result.unwrap_or_default()
}

const D: &str =
    "var d=fw.getDocumentDOM();function obj(i){return d.layers[i].frames[0].elements[0];}";

#[test]
fn s23_fw_selection_of_one_object() {
    let mut s = session();
    assert_eq!(result(&mut s, &format!("{D}d.currentLayerNum")), "2");
    assert_eq!(
        result(
            &mut s,
            &format!("{D}fw.selection=[obj(0)];d.currentLayerNum")
        ),
        "0"
    );
    assert_eq!(
        result(
            &mut s,
            &format!("{D}fw.selection=[obj(1)];d.currentLayerNum")
        ),
        "1"
    );
    // two objects keep the current layer, inside one script and afterwards
    assert_eq!(
        result(
            &mut s,
            &format!("{D}fw.selection=[obj(2),obj(0)];d.currentLayerNum")
        ),
        "1"
    );
    assert_eq!(result(&mut s, &format!("{D}d.currentLayerNum")), "1");
    // the script's own layer choice after selecting stays
    result(
        &mut s,
        &format!("{D}fw.selection=[obj(0)];d.currentLayerNum=2"),
    );
    assert_eq!(result(&mut s, &format!("{D}d.currentLayerNum")), "2");
}

#[test]
fn s23b_select_all_on_layer() {
    let mut s = session();
    assert_eq!(
        result(
            &mut s,
            &format!("{D}d.selectAllOnLayer(0);d.currentLayerNum")
        ),
        "0"
    );
    assert_eq!(result(&mut s, &format!("{D}d.currentLayerNum")), "0");
}

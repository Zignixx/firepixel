//! RE spec 65 rule 1 (TW-217): `dom.moveSelectionToLayer(layerIndex, copy, "none", -1)` moves the
//! whole selection as one block to the top of the layer (flat index), keeps it selected and makes
//! the layer current; with `copy` the copies go there unshifted and are selected.

use fp_script::{ScriptOptions, run_source};
use serde_json::json;

/// L1 {r1}, L2 {r2}, L3 {r3}; flat 0 L1, 1 L2, 2 L3, 3 web layer; nothing selected.
fn session() -> fp_engine::Session {
    fp_engine::shell::disable();
    fp_i18n::profile::use_test_profile();
    fp_i18n::set_language("de");
    let mut s = fp_engine::Session::default();
    let ex = |s: &mut fp_engine::Session, c: &str, p: serde_json::Value| {
        s.execute(c, p).unwrap_or_else(|e| panic!("{c}: {e}"));
    };
    ex(
        &mut s,
        "file.new",
        json!({"width": 300, "height": 200, "background": "#ffffff"}),
    );
    ex(
        &mut s,
        "draw.rectangle",
        json!({"rect": [10, 10, 40, 40], "name": "r1"}),
    );
    ex(&mut s, "layer.add", json!({"name": "L2"}));
    ex(
        &mut s,
        "draw.rectangle",
        json!({"rect": [60, 10, 90, 40], "name": "r2"}),
    );
    ex(&mut s, "layer.add", json!({"name": "L3"}));
    ex(
        &mut s,
        "draw.rectangle",
        json!({"rect": [110, 10, 140, 40], "name": "r3"}),
    );
    ex(&mut s, "select.none", json!({}));
    s
}

fn result(s: &mut fp_engine::Session, src: &str) -> String {
    let o = run_source(s, src, &ScriptOptions::default());
    assert!(
        o.ok,
        "error: {:?}\ndiagnostics: {:?}\nscript: {src}",
        o.error, o.diagnostics
    );
    o.result.unwrap_or_default()
}

const D: &str = "var d=fw.getDocumentDOM();function el(i,j){return d.layers[i].frames[0].elements[j];}\
function names(i){var r=[],e=d.layers[i].frames[0].elements;for(var k=0;k<e.length;k++)r.push(e[k].name);return r.join('/');}";

#[test]
fn moves_the_selection_as_a_block_and_keeps_it_selected() {
    let mut s = session();
    let r = result(
        &mut s,
        &format!(
            "{D}fw.selection=[el(1,0),el(0,0)];d.moveSelectionToLayer(2,false,'none',-1);\
             [names(0),names(1),names(2),fw.selection.length,d.currentLayerNum].join(',')"
        ),
    );
    assert_eq!(
        r, ",,r2/r1/r3,2,2",
        "r2 over r1 on top of L3, both selected, L3 current"
    );
}

#[test]
fn copy_flag_puts_selected_copies_on_top() {
    let mut s = session();
    let r = result(
        &mut s,
        &format!(
            "{D}fw.selection=[el(0,0)];d.moveSelectionToLayer(1,true,'none',-1);\
             var c=fw.selection[0],n=fw.selection.length,x=c.left,y=c.top;\
             d.moveSelectionBy({{x:100,y:0}},false,false);\
             [names(0),names(1),n,x,y,el(1,0).left,el(0,0).left,d.currentLayerNum].join(',')"
        ),
    );
    assert_eq!(
        r, "r1,r1/r2,1,10,10,110,10,1",
        "the original stays, the unshifted copy on top of L2 is the selection"
    );
}

#[test]
fn a_filled_or_locked_target_changes_nothing() {
    let mut s = session();
    // the layer already holds a selected object
    let r = result(
        &mut s,
        &format!(
            "{D}fw.selection=[el(2,0),el(0,0)];d.moveSelectionToLayer(2,false,'none',-1);\
             [names(0),names(2)].join(',')"
        ),
    );
    assert_eq!(r, "r1,r3");
    // a locked target
    let r = result(
        &mut s,
        &format!(
            "{D}d.setLayerLocked(1,0,true,false);fw.selection=[el(0,0)];d.moveSelectionToLayer(1,false,'none',-1);\
             [names(0),names(1)].join(',')"
        ),
    );
    assert_eq!(r, "r1,r2");
}

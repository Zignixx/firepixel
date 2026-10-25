//! FB-64 (TW-240): the layer a brush or pencil stroke puts a new bitmap into. The current layer when
//! it is a normal layer that is shown and unlocked; otherwise the nearest such layer below, else
//! above; never the web layer; without any the stroke is refused with a status message, no change
//! and no History row. The brush's live preview uses the same rule; locking a layer no longer makes
//! the web layer current while another layer is unlocked.

use crate::common::*;
use fp_engine::tools::{Overlay, ToolId};
use fp_engine::{Session, ToolEvent, docops};
use fp_model::{ElementId, LayerId};
use serde_json::json;

/// L1 (bottom) with r1, L2 with r2, L3 with r3; L3 current, nothing selected.
fn mk() -> (Session, [LayerId; 3]) {
    fp_i18n::set_language("en");
    let mut s = doc_session(600.0, 400.0);
    let l1 = cur(&s);
    rect(&mut s, 50.0, 50.0, 130.0, 110.0);
    let l2 = LayerId(
        ex(&mut s, "layer.add", json!({"name": "L2"}))["layer"]
            .as_u64()
            .unwrap_or(0),
    );
    rect(&mut s, 200.0, 100.0, 280.0, 160.0);
    let l3 = LayerId(
        ex(&mut s, "layer.add", json!({"name": "L3"}))["layer"]
            .as_u64()
            .unwrap_or(0),
    );
    rect(&mut s, 350.0, 150.0, 430.0, 210.0);
    ex(&mut s, "select.none", json!({}));
    assert_eq!(cur(&s), l3);
    (s, [l1, l2, l3])
}

fn cur(s: &Session) -> LayerId {
    s.active().and_then(|d| d.layer).expect("current layer")
}

fn rows(s: &Session) -> usize {
    s.active().map(|d| d.history.len()).unwrap_or(0)
}

fn web(s: &Session) -> LayerId {
    let d = s.active().expect("doc");
    docops::web_layer(&d.doc, d.page).expect("web layer")
}

fn lock(s: &mut Session, l: LayerId) {
    ex(s, "layer.setLocked", json!({"layer": l.0, "locked": true}));
}

fn hide(s: &mut Session, l: LayerId) {
    ex(
        s,
        "layer.setVisible",
        json!({"layer": l.0, "visible": false}),
    );
}

fn set_cur(s: &mut Session, l: LayerId) {
    ex(s, "layer.setActive", json!({"layer": l.0}));
}

fn target(s: &Session) -> Result<Option<LayerId>, docops::NoPaintLayer> {
    let d = s.active().expect("doc");
    docops::paint_target_layer(&d.doc, d.page, d.frame, d.layer)
}

fn stroke(s: &mut Session) -> fp_engine::Result<fp_engine::CommandOutput> {
    let b = serde_json::to_value(fp_model::Brush::basic(4)).expect("brush");
    s.execute(
        "bitmap.paint",
        json!({"tool": "pencil", "brush": b, "color": "#C04040", "trimEnd": false,
               "points": [[100.0, 300.0], [300.0, 300.0]]}),
    )
}

/// Paints a stroke and returns the layer of the bitmap it painted (selected afterwards).
fn paint(s: &mut Session) -> (ElementId, LayerId) {
    stroke(s).expect("stroke");
    let d = s.active().expect("doc");
    let id = *d
        .selection
        .elements
        .first()
        .expect("painted bitmap selected");
    let path = docops::find_on_page(&d.doc, d.page, id).expect("path");
    (
        id,
        docops::owning_layer(&d.doc, d.page, &path).expect("layer"),
    )
}

fn count(s: &Session) -> usize {
    let d = s.active().expect("doc");
    docops::own_layers(&d.doc, d.page)
        .iter()
        .map(|(_, l)| l.cels.iter().map(|c| c.elements.len()).sum::<usize>())
        .sum()
}

#[test]
fn ok_current_normal_layer_takes_the_bitmap() {
    let (mut s, l) = mk();
    assert_eq!(paint(&mut s).1, l[2]);
    let (mut s, l) = mk();
    set_cur(&mut s, l[1]);
    assert_eq!(paint(&mut s).1, l[1]);
    // an empty layer as well
    let (mut s, _) = mk();
    let l4 = LayerId(
        ex(&mut s, "layer.add", json!({"name": "L4"}))["layer"]
            .as_u64()
            .unwrap_or(0),
    );
    assert_eq!(paint(&mut s).1, l4);
}

#[test]
fn ok_selected_bitmap_on_unlocked_layer_is_painted_into() {
    let (mut s, _) = mk();
    let b = bitmap(&mut s, 50, 250, 300, 100, [0, 0, 255, 255]);
    select(&mut s, &[b]);
    let n = count(&s);
    stroke(&mut s).expect("stroke");
    assert_eq!(count(&s), n, "no new element");
    let d = s.active().expect("doc");
    assert_eq!(d.selection.elements, vec![b]);
}

#[test]
fn hidden_current_layer_paints_into_the_visible_layer_below_or_above() {
    let (mut s, l) = mk();
    hide(&mut s, l[2]);
    assert_eq!(target(&s), Ok(Some(l[1])));
    assert_eq!(paint(&mut s).1, l[1]);
    assert_eq!(cur(&s), l[1], "the target becomes current");
    // the bottom layer hidden → the layer above
    let (mut s, l) = mk();
    set_cur(&mut s, l[0]);
    hide(&mut s, l[0]);
    assert_eq!(paint(&mut s).1, l[1]);
}

#[test]
fn locked_current_layer_paints_into_an_unlocked_neighbour() {
    // helper: a locked current layer → the nearest unlocked one below
    let (mut s, l) = mk();
    set_cur(&mut s, l[1]);
    {
        let d = s.active_mut().expect("doc");
        let page = d.page;
        docops::edit_layer_by_id(d.doc_mut(), page, l[1], |x| {
            for c in &mut x.cels {
                std::sync::Arc::make_mut(c).locked = true;
            }
        })
        .expect("layer");
    }
    assert_eq!(target(&s), Ok(Some(l[0])));
    // through the command: L3 locked while current → L2 current, the stroke lands there
    let (mut s, l) = mk();
    lock(&mut s, l[2]);
    assert_ne!(cur(&s), web(&s), "locking keeps the web layer out");
    let (_, on) = paint(&mut s);
    assert!(on == l[0] || on == l[1], "an unlocked normal layer");
}

#[test]
fn locking_the_top_layers_does_not_make_the_web_layer_current() {
    let (mut s, l) = mk();
    lock(&mut s, l[1]);
    assert_eq!(cur(&s), l[2]);
    lock(&mut s, l[2]);
    assert_eq!(
        cur(&s),
        l[0],
        "wraps to the unlocked L1 instead of the web layer"
    );
    assert_eq!(paint(&mut s).1, l[0]);
    // every normal layer locked → still the web layer (nothing else is unlocked)
    lock(&mut s, l[0]);
    assert_eq!(cur(&s), web(&s));
}

#[test]
fn web_layer_current_paints_into_the_nearest_unlocked_visible_normal_layer() {
    let (mut s, l) = mk();
    let w = web(&s);
    set_cur(&mut s, w);
    assert_eq!(target(&s), Ok(Some(l[2])));
    assert_eq!(paint(&mut s).1, l[2]);
    // L3 hidden, L2 locked → L1
    let (mut s, l) = mk();
    hide(&mut s, l[2]);
    lock(&mut s, l[1]);
    let w = web(&s);
    set_cur(&mut s, w);
    assert_eq!(paint(&mut s).1, l[0]);
}

#[test]
fn no_paintable_layer_refuses_without_change_or_history_row() {
    let (mut s, l) = mk();
    lock(&mut s, l[0]);
    lock(&mut s, l[1]);
    hide(&mut s, l[2]);
    assert_eq!(target(&s), Err(docops::NoPaintLayer));
    let (n, h) = (count(&s), rows(&s));
    let err = stroke(&mut s).expect_err("refused");
    let msg = fp_i18n::t("err-paint-no-layer");
    assert!(err.to_string().contains(&msg), "{err}");
    assert_eq!((count(&s), rows(&s)), (n, h));
    // the brush shows no live stroke and the release changes nothing either
    s.set_tool(ToolId::Brush).expect("brush");
    // choosing the Brush writes its own row (RE spec 72 §1); the refused stroke writes none
    let h = rows(&s);
    s.tools.options.brush_size = 10.0;
    let _ = s.tool_event(&ToolEvent::down(100.0, 300.0));
    let _ = s.tool_event(&ToolEvent::drag(200.0, 300.0));
    let live = s.tool_feedback().into_iter().any(|o| match o {
        Overlay::Paint(p) => p
            .tiles
            .iter()
            .any(|t| t.image.data().iter().any(|q| q[3] > 0)),
        _ => false,
    });
    assert!(!live, "no live stroke");
    let _ = s.tool_event(&ToolEvent::up(200.0, 300.0));
    assert_eq!((count(&s), rows(&s)), (n, h));
}

#[test]
fn selected_bitmap_on_locked_layer_gives_a_new_bitmap_on_an_unlocked_layer() {
    let (mut s, l) = mk();
    set_cur(&mut s, l[0]);
    let b = bitmap(&mut s, 50, 250, 300, 100, [0, 0, 255, 255]);
    select(&mut s, &[b]);
    lock(&mut s, l[0]);
    let old = el(&s, b);
    let (id, on) = paint(&mut s);
    assert_ne!(id, b);
    assert_ne!(on, l[0], "not into the locked layer");
    assert_ne!(on, web(&s));
    assert!(*el(&s, b) == *old, "the locked bitmap is unchanged");
}

#[test]
fn brush_live_preview_shows_the_stroke_on_a_paintable_layer() {
    let (mut s, l) = mk();
    hide(&mut s, l[2]);
    s.set_tool(ToolId::Brush).expect("brush");
    s.tools.options.brush_size = 10.0;
    events(
        &mut s,
        &[ToolEvent::down(100.0, 300.0), ToolEvent::drag(200.0, 300.0)],
    );
    let live = s
        .tool_feedback()
        .into_iter()
        .any(|o| matches!(o, Overlay::Paint(_)));
    assert!(live, "the stroke is previewed (it lands in L2)");
    events(&mut s, &[ToolEvent::up(200.0, 300.0)]);
    let d = s.active().expect("doc");
    let id = d.selection.elements[0];
    let path = docops::find_on_page(&d.doc, d.page, id).expect("path");
    assert_eq!(docops::owning_layer(&d.doc, d.page, &path), Some(l[1]));
}

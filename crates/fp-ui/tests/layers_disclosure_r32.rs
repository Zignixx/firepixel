//! R-32: the Layers panel shows each layer's disclosure from the document (`DISb`, the flag that
//! `setLayerDisclosure` / `layer.setExpanded` writes), not panel state keyed by layer id that would
//! carry over to another document or to the reopened file. The disclosure triangle writes the flag
//! through `layer.setExpanded` (one History step, like the reference's disclosure undoable).

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_engine::tools::ToolId;
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use serde_json::{Value, json};

type H = Harness<'static, FirepixelApp>;

fn ex(s: &mut fp_engine::Session, c: &str, p: Value) -> Value {
    s.execute(c, p).unwrap_or_else(|e| panic!("{c}: {e}")).value
}

/// New document with one rectangle named `name` on "Ebene 1"; nothing selected.
fn new_doc(s: &mut fp_engine::Session, name: &str) {
    ex(s, "file.new", json!({"width": 400, "height": 300, "background": "#ffffff"}));
    let v = ex(s, "draw.rectangle", json!({"rect": [20, 20, 120, 120]}));
    let id = v["id"].as_u64().expect("id");
    ex(s, "element.setName", json!({"ids": [id], "name": name}));
    ex(s, "select.none", json!({}));
}

fn harness(s: fp_engine::Session) -> H {
    fp_i18n::set_language("de");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.os_clipboard = false;
    app.state.sync_render = true;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1500.0, 950.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(8)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    for _ in 0..2 {
        if find(&h, "Ebene 1").is_some() {
            break;
        }
        let tab = find(&h, "Ebenen").expect("Layers tab");
        click_at(&mut h, tab);
        h.run_steps(30);
    }
    assert!(find(&h, "Ebene 1").is_some(), "Layers panel shows Ebene 1");
    h
}

fn ev(h: &mut H, e: Event) {
    h.input_mut().events.push(e);
    h.step();
}

fn click_at(h: &mut H, p: Pos2) {
    h.run_steps(30);
    ev(h, Event::PointerMoved(p));
    let e = |down| Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: down, modifiers: Modifiers::NONE };
    ev(h, e(true));
    ev(h, e(false));
    ev(h, Event::PointerMoved(Pos2::new(5.0, 940.0)));
    h.run_steps(5);
}

fn texts(shape: &egui::epaint::Shape, out: &mut Vec<(String, egui::Rect)>) {
    match shape {
        egui::epaint::Shape::Text(t) => out.push((t.galley.job.text.clone(), egui::Rect::from_min_size(t.pos, t.galley.size()))),
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
        _ => {}
    }
}

/// Rect of a text in the right part of the window (the Layers panel), rightmost match.
fn find_rect(h: &H, label: &str) -> Option<egui::Rect> {
    let mut all = Vec::new();
    for cs in &h.output().shapes {
        texts(&cs.shape, &mut all);
    }
    all.into_iter()
        .filter(|(t, r)| t == label && r.center().x > 900.0)
        .map(|(_, r)| r)
        .max_by(|a, b| a.center().x.total_cmp(&b.center().x))
}

fn find(h: &H, label: &str) -> Option<Pos2> {
    find_rect(h, label).map(|r| r.center())
}

/// Whether the panel shows a row with the text `label` (a few frames for the list to settle).
fn shows(h: &mut H, label: &str) -> bool {
    h.run_steps(10);
    find(h, label).is_some()
}

/// Click on the disclosure triangle of the layer row `layer` (the triangle sits 28 px left of the
/// name: row + 50 against row + 78).
fn click_triangle(h: &mut H, layer: &str) {
    let r = find_rect(h, layer).unwrap_or_else(|| panic!("layer row {layer}"));
    click_at(h, Pos2::new(r.left() - 28.0, r.center().y));
}

fn active(h: &H) -> &fp_engine::DocumentState {
    h.state().state.session.active().expect("document")
}

fn layer_of(h: &H, name: &str) -> std::sync::Arc<fp_model::Layer> {
    let d = active(h);
    d.doc
        .resolved_layers(d.page)
        .into_iter()
        .find(|rl| rl.layer.name == name)
        .map(|rl| rl.layer)
        .unwrap_or_else(|| panic!("layer {name}"))
}

/// Disclosure flag of the layer `name` of the active document.
fn expanded(h: &H, name: &str) -> bool {
    layer_of(h, name).expanded
}

fn layer_id(h: &H, name: &str) -> u64 {
    layer_of(h, name).id.0
}

fn tmp(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("fp_r32_disclosure_{}", std::process::id()));
    std::fs::create_dir_all(&d).expect("temp dir");
    d.join(name)
}

fn run(h: &mut H, c: &str, p: Value) {
    h.state_mut().state.session.execute(c, p).unwrap_or_else(|e| panic!("{c}: {e}"));
}

/// The triangle writes the document's flag in one History step (undoable); `layer.setExpanded`
/// (the command behind `setLayerDisclosure`) is reflected by the panel rows.
#[test]
fn panel_layers_triangle_writes_the_document_flag() {
    let mut s = fp_engine::Session::default();
    new_doc(&mut s, "O1");
    s.set_tool(ToolId::Pointer).expect("pointer");
    let mut h = harness(s);
    assert!(expanded(&h, "Ebene 1") && shows(&mut h, "O1"), "new layer expanded, object row shown");
    let n0 = active(&h).history.undo_count();
    click_triangle(&mut h, "Ebene 1");
    assert!(!expanded(&h, "Ebene 1"), "triangle collapses the layer in the document");
    assert!(!shows(&mut h, "O1"), "object row gone");
    assert_eq!(active(&h).history.undo_count(), n0 + 1, "one History step");
    let last = active(&h).history.steps().last().map(|st| st.command.clone());
    assert_eq!(last.as_deref(), Some("layer.setExpanded"));
    h.state_mut().state.session.undo().expect("undo");
    assert!(expanded(&h, "Ebene 1") && shows(&mut h, "O1"), "undo expands it again");
    click_triangle(&mut h, "Ebene 1");
    assert!(!shows(&mut h, "O1"), "collapsed again");
    click_triangle(&mut h, "Ebene 1");
    assert!(expanded(&h, "Ebene 1") && shows(&mut h, "O1"), "a second click expands");
    // the script / command side (setLayerDisclosure) drives the rows
    let id = layer_id(&h, "Ebene 1");
    run(&mut h, "layer.setExpanded", json!({"layer": id, "expanded": false}));
    assert!(!shows(&mut h, "O1"), "layer.setExpanded false hides the object row");
    run(&mut h, "layer.setExpanded", json!({"layer": id, "expanded": true}));
    assert!(shows(&mut h, "O1"), "layer.setExpanded true shows it");
}

/// The case of R-32: collapse "Ebene 1" in one document; a second document whose layer has the same
/// id shows it expanded, and the reopened files show what they stored.
#[test]
fn panel_layers_disclosure_does_not_carry_over_documents() {
    let mut s = fp_engine::Session::default();
    new_doc(&mut s, "A1");
    s.set_tool(ToolId::Pointer).expect("pointer");
    let doc_a = s.active_id().expect("A");
    let mut h = harness(s);
    let id_a = layer_id(&h, "Ebene 1");
    click_triangle(&mut h, "Ebene 1");
    assert!(!shows(&mut h, "A1"), "collapsed in A");
    // second document, same layer id
    new_doc(&mut h.state_mut().state.session, "B1");
    assert_eq!(layer_id(&h, "Ebene 1"), id_a, "same layer id in B");
    assert!(expanded(&h, "Ebene 1"), "B's layer is expanded in the document");
    assert!(shows(&mut h, "B1"), "and in the panel: no state carried over from A");
    let doc_b = h.state().state.session.active_id().expect("B");
    // back to A: still collapsed (its own flag)
    h.state_mut().state.session.activate(doc_a).expect("activate A");
    assert!(!shows(&mut h, "A1"), "A stays collapsed");
    // save both, close them and reopen: each shows what its file stores
    let (pa, pb) = (tmp("a.fw.png"), tmp("b.fw.png"));
    run(&mut h, "file.saveAs", json!({"path": pa.display().to_string()}));
    run(&mut h, "file.close", json!({}));
    h.state_mut().state.session.activate(doc_b).expect("activate B");
    run(&mut h, "file.saveAs", json!({"path": pb.display().to_string()}));
    run(&mut h, "file.close", json!({}));
    run(&mut h, "file.open", json!({"path": pb.display().to_string()}));
    assert!(expanded(&h, "Ebene 1") && shows(&mut h, "B1"), "reopened B: expanded as stored");
    run(&mut h, "file.open", json!({"path": pa.display().to_string()}));
    assert!(!expanded(&h, "Ebene 1"), "reopened A: collapsed as stored");
    assert!(!shows(&mut h, "A1"), "reopened A: object row hidden");
    // the reopened file B again, with A's collapsed layer of the same id open beside it
    run(&mut h, "file.close", json!({}));
    run(&mut h, "file.close", json!({}));
    run(&mut h, "file.open", json!({"path": pa.display().to_string()}));
    run(&mut h, "file.open", json!({"path": pb.display().to_string()}));
    assert!(expanded(&h, "Ebene 1") && shows(&mut h, "B1"), "B expanded beside the collapsed A");
    let _ = std::fs::remove_file(pa);
    let _ = std::fs::remove_file(pb);
}

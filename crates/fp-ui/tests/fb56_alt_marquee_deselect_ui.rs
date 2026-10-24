//! FB-56 (Firepixel extension, off by default): the Preferences check box "Alt-drag on empty canvas
//! deselects" (page Edit) writes `Firepixel_AltMarqueeDeselects`; with it on, the Pointer's
//! Alt-marquee on the real canvas removes the objects inside from the selection and the canvas
//! cursor shows the arrow with "−" (`Cursor::Deselect`) instead of the "+" copy badge.

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fp_engine::Session;
use fp_engine::prefs::PrefValue;
use fp_engine::tools::{ALT_MARQUEE_DESELECT_PREF, Cursor, ToolId};
use fp_model::ElementId;
use fp_ui::FirepixelApp;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::PreferencesDialog;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

#[test]
fn fb56_preferences_check_box_writes_the_key() {
    fp_i18n::set_language("en");
    let st = AppState::new(Session::default(), UiSettings::default(), None);
    assert!(!st.session.prefs.get_bool(ALT_MARQUEE_DESELECT_PREF, false), "default off");
    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1100.0, 800.0))
        .build_ui_state(|ui, st: &mut AppState| fp_ui::dialogs::show_all(ui.ctx(), st), st);
    fp_ui::theme::install_fonts(&harness.ctx);
    harness.run_steps(2);
    let mut d = PreferencesDialog::new(harness.state());
    d.category = 1; // Edit
    harness.state_mut().dialogs.preferences = Some(d);
    harness.run_steps(3);
    let label = fp_i18n::t("dlg-pref-altmarqueedeselects");
    assert_eq!(label, "Alt-drag on empty canvas deselects");
    harness.get_by_label(&label).click();
    harness.run_steps(3);
    harness.get_by_label(&fp_i18n::t("btn-ok")).click();
    harness.run_steps(3);
    assert!(harness.state().dialogs.preferences.is_none(), "OK closes");
    assert!(harness.state().session.prefs.get_bool(ALT_MARQUEE_DESELECT_PREF, false), "checking stores the key");
    // the German label
    let de = fp_i18n::with_language("de", || fp_i18n::t("dlg-pref-altmarqueedeselects"));
    assert_eq!(de, "Alt-Ziehen auf leerer Fläche hebt Auswahl auf");
}

fn harness(s: Session) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(12)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn at(h: &H, x: f32, y: f32) -> Pos2 {
    let st = &h.state().state;
    let d = st.session.active().expect("document");
    let area = st.canvases.get(&d.id).and_then(|c| c.area).expect("canvas");
    let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(x, y));
    Pos2::new(area.min.x + sx as f32, area.min.y + sy as f32)
}

fn button(h: &mut H, p: Pos2, pressed: bool, m: Modifiers) {
    h.input_mut().events.push(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed, modifiers: m });
    h.step();
}

/// Drag from `a` to `b` with `m` held; returns the canvas cursor just before the release.
fn drag(h: &mut H, a: (f32, f32), b: (f32, f32), m: Modifiers) -> Cursor {
    let first = at(h, a.0, a.1);
    h.input_mut().events.push(Event::ModifiersChanged(m));
    h.input_mut().events.push(Event::PointerMoved(first));
    h.step();
    button(h, first, true, m);
    let mut last = first;
    for t in [0.3f32, 0.7, 1.0] {
        last = at(h, a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
        h.input_mut().events.push(Event::PointerMoved(last));
        h.step();
    }
    let c = h.state().state.tool_cursor;
    button(h, last, false, m);
    h.input_mut().events.push(Event::ModifiersChanged(Modifiers::NONE));
    h.run_steps(30);
    c
}

fn sel(h: &H) -> Vec<ElementId> {
    let mut v = h.state().state.session.active().map(|d| d.selection.elements.clone()).unwrap_or_default();
    v.sort_by_key(|i| i.0);
    v
}

fn id_of(v: &serde_json::Value) -> ElementId {
    ElementId(v["id"].as_u64().or_else(|| v["ids"][0].as_u64()).expect("id"))
}

/// Objects on the active page (all own layers, current frame).
fn count(h: &H) -> usize {
    let d = h.state().state.session.active().expect("document");
    fp_engine::docops::own_layers(&d.doc, d.page)
        .into_iter()
        .map(|(_, l)| l.cels.get(if l.shared_across_frames { 0 } else { d.frame }).map_or(0, |c| c.elements.len()))
        .sum()
}

fn rows(h: &H) -> usize {
    h.state().state.session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

#[test]
fn fb56_pointer_alt_marquee_cursor_badge_and_deselect_on_the_canvas() {
    fp_engine::shell::disable();
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 600, "height": 400, "background": "#ffffff"})).expect("doc");
    let a = id_of(&s.execute("draw.rectangle", json!({"rect": [100, 80, 200, 160]})).expect("a").value);
    let b = id_of(&s.execute("draw.rectangle", json!({"rect": [250, 80, 350, 160]})).expect("b").value);
    let c = id_of(&s.execute("draw.rectangle", json!({"rect": [400, 200, 500, 300]})).expect("c").value);
    let mut h = harness(s);
    h.state_mut().state.session.set_tool(ToolId::Pointer).expect("tool");
    // off (reference): the Alt-marquee replaces, the cursor shows "+"
    h.state_mut().state.session.execute("select.set", json!({"ids": [a.0, b.0, c.0], "mode": "replace"})).expect("sel");
    h.run_steps(3);
    let cur = drag(&mut h, (80.0, 60.0), (220.0, 180.0), Modifiers::ALT);
    assert_eq!(cur, Cursor::Copy, "off: arrow with +");
    assert_eq!(sel(&h), vec![a], "off: Alt-marquee replaces");
    // on: the Alt-marquee removes A and B, C stays, cursor with "−", no History row
    h.state_mut().state.session.prefs.set(ALT_MARQUEE_DESELECT_PREF, PrefValue::Bool(true));
    h.state_mut().state.session.execute("select.set", json!({"ids": [a.0, b.0, c.0], "mode": "replace"})).expect("sel");
    h.run_steps(3);
    let r0 = rows(&h);
    let cur = drag(&mut h, (80.0, 60.0), (370.0, 180.0), Modifiers::ALT);
    assert_eq!(cur, Cursor::Deselect, "on: arrow with -");
    assert_eq!(sel(&h), vec![c], "A and B removed, C stays");
    assert_eq!(rows(&h), r0, "no History row");
    // Alt-drag on C still clones it (cursor with "+")
    let n = count(&h);
    let cur = drag(&mut h, (450.0, 250.0), (470.0, 270.0), Modifiers::ALT);
    assert_eq!(cur, Cursor::Copy, "clone cursor");
    let n2 = count(&h);
    assert_eq!(n2, n + 1, "Alt-drag on an object clones");
}

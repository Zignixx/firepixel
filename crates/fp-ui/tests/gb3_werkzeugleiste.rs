//! G-B3 (B-2b BA-128): Farbwähler der Werkzeugleiste wie im Original gemessen.
//! - Füllung: „Verlauf“ bzw. „Muster“ sind immer bedienbar (auch ohne Auswahl) und öffnen das
//!   Verlaufs- bzw. Muster-Popup; erst eine Wahl darin ändert etwas (Abbrechen: R1 bleibt einfarbig).
//! - Strich: das Popup der Werkzeugleiste hat keine Strichlage-Knöpfe (nur Ohne + Farbkreis).

use egui::{Event, Id, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::Harness;
use fp_engine::Session;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::inspector::color_picker as cp;
use fp_ui::inspector::{obj_quads, popups};
use fp_ui::FirepixelApp;
use serde_json::json;

fn app(with_rect: bool) -> FirepixelApp {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200})).expect("doc");
    if with_rect {
        s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80], "fillColor": "#ff0000", "strokeWidth": 2, "strokeColor": "#000000"})).expect("R1");
    } else {
        s.execute("select.none", json!({})).expect("nichts");
    }
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.sync_render = true;
    a
}

fn harness(a: FirepixelApp) -> Harness<'static, FirepixelApp> {
    Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(16).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a)
}

fn st<'a>(h: &'a Harness<'_, FirepixelApp>) -> &'a AppState {
    &h.state().state
}

fn rect_of(h: &Harness<'_, FirepixelApp>, id: Id) -> Rect {
    h.ctx.read_response(id).unwrap_or_else(|| panic!("Widget {id:?} nicht gezeichnet")).rect
}

fn click_at(h: &mut Harness<'_, FirepixelApp>, p: Pos2) {
    h.event(Event::PointerMoved(p));
    h.run_steps(1);
    h.event(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run_steps(1);
    h.event(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

fn key(h: &mut Harness<'_, FirepixelApp>, k: Key) {
    h.event(Event::Key { key: k, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
    h.event(Event::Key { key: k, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

fn undo_count(st: &AppState) -> usize {
    st.session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

/// Öffnet den Farbwähler am Farbfeld der Werkzeugleiste, liefert dessen Rechteck.
fn open_well(h: &mut Harness<'_, FirepixelApp>, stroke: bool) -> Rect {
    let sw = Id::new(("well", stroke));
    let well = rect_of(h, sw);
    click_at(h, egui::pos2(well.right() - well.width() / 4.0, well.center().y));
    cp::open_rect(&h.ctx, sw).expect("Farbwähler offen")
}

/// Knopf der Fußleiste (Mitte bei x = `cx`, wie im Farbwähler).
fn footer(p: Rect, cx: f32) -> Pos2 {
    p.min + egui::vec2(cx, 260.5)
}

fn r1_fill_kind(st: &AppState) -> u8 {
    let d = st.session.active().expect("doc");
    let id = *fp_engine::docops::selectable_ids(&d.doc, d.page, d.frame).first().expect("R1");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("R1");
    fp_ui::ui_commands::fill_kind(fp_engine::docops::path_attrs_of(&e).and_then(|a| a.fill.clone()).as_ref())
}

#[test]
fn gradient_and_pattern_open_their_popup_without_selection() {
    let mut h = harness(app(false));
    h.run_steps(3);
    let (gp, pp) = obj_quads::toolbar_fill_popup_ids();
    let kind0 = obj_quads::tool_attrs(&h.ctx).fill_kind;
    let p = open_well(&mut h, false);
    click_at(&mut h, footer(p, 64.5));
    assert!(popups::anchored_open(&h.ctx, gp), "Verlauf ohne Auswahl öffnet das Verlaufs-Popup");
    assert_eq!(obj_quads::tool_attrs(&h.ctx).fill_kind, kind0, "Werkzeugvorgabe erst mit einer Wahl");
    key(&mut h, Key::Escape);
    assert!(!popups::anchored_open(&h.ctx, gp), "Esc schließt");
    let p = open_well(&mut h, false);
    click_at(&mut h, footer(p, 88.5));
    assert!(popups::anchored_open(&h.ctx, pp), "Muster öffnet das Muster-Popup");
    assert!(!popups::anchored_open(&h.ctx, gp));
    key(&mut h, Key::Escape);
    assert_eq!(obj_quads::tool_attrs(&h.ctx).fill_kind, kind0);
}

#[test]
fn gradient_with_selection_cancel_keeps_solid_fill() {
    let mut h = harness(app(true));
    h.run_steps(3);
    let (gp, _) = obj_quads::toolbar_fill_popup_ids();
    let u0 = undo_count(st(&h));
    let p = open_well(&mut h, false);
    click_at(&mut h, footer(p, 64.5));
    assert!(popups::anchored_open(&h.ctx, gp), "Verlaufs-Popup offen");
    assert_eq!(r1_fill_kind(st(&h)), 1, "R1 noch einfarbig");
    key(&mut h, Key::Escape);
    assert!(!popups::anchored_open(&h.ctx, gp));
    assert_eq!(r1_fill_kind(st(&h)), 1, "nach Abbrechen einfarbig");
    assert_eq!(undo_count(st(&h)), u0, "kein Verlaufsschritt");
}

#[test]
fn toolbar_stroke_popup_has_no_placement_buttons() {
    let mut h = harness(app(true));
    h.run_steps(3);
    let u0 = undo_count(st(&h));
    // wo im Inspektor-Popup „Strich innen“ liegt: hier nichts
    let p = open_well(&mut h, true);
    click_at(&mut h, footer(p, 40.5));
    assert_eq!(undo_count(st(&h)), u0, "keine Strichlage geändert");
    key(&mut h, Key::Escape);
}

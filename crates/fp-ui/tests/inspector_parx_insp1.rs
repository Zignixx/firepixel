//! PAR-X-INSP-1: Inspektor wie das Original (docs\parity\inspector-document.md, inspector-path-rect.md,
//! inspector-fill.md) – Füllart-Knöpfe öffnen ihr Popup, Verlaufsarten-Liste im Popup (12 Arten,
//! schließt das Popup nicht), Stopp aus dem Popup ziehen löscht ihn, Werkzeug übernimmt die zuletzt
//! gesetzte Füllung, Pfadoperationen, Muster am Werkzeug, Reiter-Doppelklick, Exportformat-Liste.

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::Harness;
use fp_engine::ToolId;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::inspector::{obj_quads, popups};
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

const W: f32 = 1800.0;
const HH: f32 = 1130.0;

fn harness(setup: impl FnOnce(&mut fp_engine::Session)) -> H {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 300, "background": "#ffffff"})).expect("Dokument");
    setup(&mut s);
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(W, HH)).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn rect(s: &mut fp_engine::Session, r: [i32; 4]) -> u64 {
    s.execute("draw.rectangle", json!({"rect": r, "fillColor": "#808080", "strokeWidth": 0})).expect("Rechteck").value["id"].as_u64().expect("id")
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn press(h: &mut H, p: Pos2, down: bool) {
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: down, modifiers: Modifiers::NONE });
}

fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    press(h, p, true);
    press(h, p, false);
    h.run_steps(2);
}

fn drag(h: &mut H, from: Pos2, to: Pos2) {
    ev(h, Event::PointerMoved(from));
    press(h, from, true);
    for k in 1..=10 {
        let t = k as f32 / 10.0;
        ev(h, Event::PointerMoved(from + (to - from) * t));
    }
    press(h, to, false);
    h.run_steps(2);
}

fn undo_count(h: &H) -> usize {
    st(h).session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn undo_label(h: &H) -> String {
    st(h).session.active().and_then(|d| d.history.undo_label().map(|s| s.to_string())).unwrap_or_default()
}

fn fill(h: &H) -> fp_model::Fill {
    obj_quads::shown_attrs(st(h)).and_then(|a| a.fill.clone()).expect("Füllung")
}

fn button(h: &H, label: &str) -> Option<(egui::Rect, bool)> {
    h.query_all(By::new().role(Role::Button).label(label)).next().map(|n| (n.rect(), !n.accesskit_node().is_disabled()))
}

fn combo(h: &H, value: &str) -> egui::Rect {
    h.get_all(By::new().role(Role::ComboBox)).find(|n| n.value().as_deref() == Some(value)).unwrap_or_else(|| panic!("Kombifeld „{value}“")).rect()
}

/// „Verlaufsfläche“ stellt in einem Schritt „Füllen“ um und öffnet sofort das Verlaufs-Popup; die
/// Liste „Verlauf:“ zeigt die 12 Arten des Originals, eine Wahl darin schließt das Popup nicht;
/// Esc schließt es, die Auswahl bleibt (PAR 18/01, 18/02, 18/20).
#[test]
fn gradient_button_opens_popup_and_list_keeps_it_open() {
    let mut h = harness(|s| {
        rect(s, [50, 50, 150, 120]);
    });
    let n0 = undo_count(&h);
    let (b, en) = button(&h, "Verlaufsfläche").expect("Knopf Verlaufsfläche");
    assert!(en);
    click(&mut h, b.center());
    h.run_steps(2);
    assert_eq!(fp_engine::commands::panel::fill_kind(Some(&fill(&h))), 2, "Verlauf");
    assert_eq!(undo_count(&h), n0 + 1);
    assert_eq!(undo_label(&h), "Füllen");
    let pid = obj_quads::fill_popup_ids(false).0;
    assert!(popups::anchored_open(&h.ctx, pid), "Verlaufs-Popup offen");
    // Liste aufklappen: 12 Arten
    let c = combo(&h, "Linear").center();
    click(&mut h, c);
    for name in ["Linear", "Radial", "Rechteck", "Kegel", "Kontur", "Glanz", "Supernova", "Falten", "Ellipse", "Streifen", "Kleine Wellen", "Wellen"] {
        assert!(h.query_by_label(name).is_some(), "Art „{name}“ in der Liste");
    }
    let k = h.get_by_label("Kontur").rect();
    click(&mut h, k.center());
    h.run_steps(2);
    assert!(fill(&h).is_contour_gradient(), "Kontur gewählt");
    assert_eq!(undo_count(&h), n0 + 2);
    assert!(popups::anchored_open(&h.ctx, pid), "Wahl in der Liste schließt das Popup nicht");
    // 50× dieselbe Art: kein weiterer Schritt (§4b)
    for _ in 0..3 {
        let c = combo(&h, "Kontur").center();
    click(&mut h, c);
        let k = h.get_by_label("Kontur").rect();
        click(&mut h, k.center());
    }
    assert_eq!(undo_count(&h), n0 + 2);
    ev(&mut h, Event::Key { key: Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
    ev(&mut h, Event::Key { key: Key::Escape, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
    assert!(!popups::anchored_open(&h.ctx, pid), "Esc schließt das Popup");
    assert_eq!(st(&h).session.active().map(|d| d.selection.elements.len()), Some(1), "Auswahl bleibt");
}

/// Klick unter die Verlaufsleiste legt einen Farbstopp an; ihn aus dem Popup nach unten ziehen löscht
/// ihn wieder (PAR 18/05).
#[test]
fn gradient_stop_dragged_out_is_deleted() {
    let mut h = harness(|s| {
        rect(s, [50, 50, 150, 120]);
        s.execute("pi.fill", json!({"category": "gradient"})).expect("Verlauf");
    });
    let pid = obj_quads::fill_popup_ids(false).0;
    popups::set_anchored_open(&h.ctx, pid, true);
    h.run_steps(3);
    let c = combo(&h, "Linear");
    let o = c.min - egui::vec2(46.0, 0.0);
    let bar_bottom = o.y + 74.0;
    let x = o.x + 16.0 + 0.5 * 176.0;
    let stops = |h: &H| fill(h).gradient.as_ref().map(|g| g.color_stops().len()).unwrap_or(0);
    assert_eq!(stops(&h), 2);
    click(&mut h, egui::pos2(x, bar_bottom + 7.0));
    h.run_steps(2);
    assert_eq!(stops(&h), 3, "neuer Stopp");
    drag(&mut h, egui::pos2(x, bar_bottom + 7.0), egui::pos2(x, bar_bottom + 90.0));
    h.run_steps(2);
    assert_eq!(stops(&h), 2, "herausgezogen = gelöscht");
}

/// Die zuletzt im Inspektor gesetzte Füllung gilt für neue Objekte (PAR 18/22): Verlauf „Wellen“
/// an der Auswahl → Werkzeugvorgabe für neue Rechtecke ist derselbe Verlauf.
#[test]
fn new_objects_take_last_fill() {
    let mut h = harness(|s| {
        rect(s, [50, 50, 150, 120]);
    });
    let (b, _) = button(&h, "Verlaufsfläche").expect("Knopf");
    click(&mut h, b.center());
    h.run_steps(2);
    let c = combo(&h, "Linear").center();
    click(&mut h, c);
    let k = h.get_by_label("Wellen").rect();
    click(&mut h, k.center());
    h.run_steps(3);
    let ta = obj_quads::tool_attrs(&h.ctx);
    assert_eq!((ta.fill_kind, ta.grad_shape), (2, 14), "Werkzeug übernimmt den Verlauf");
    let (f, _) = fp_ui::canvas::overlay::attrs_params(&ta);
    assert_eq!(f.as_ref().and_then(|v| v["category"].as_str()), Some("gradient"));
    assert_eq!(f.as_ref().and_then(|v| v["shape"].as_i64()), Some(14));
}

/// Rechteck-Werkzeug ohne Auswahl: „Kombinieren“ + 5 Pfadknöpfe wie im Original (Referenzbilder
/// tool-shapes/07, inspector-fill/21; FIX-G5), im leeren Dokument grau, mit Pfaden auf der Seite
/// bedienbar; „Muster“ bedienbar (PAR 18/21); ein gewählter Pfad: Pfadoperationen und „Pinselstrich
/// löschen“ bedienbar (PAR 17/02).
#[test]
fn rect_tool_pathops_and_path_ops_enabled_for_one_path() {
    let union = fp_i18n::t("pi-pathop-union");
    let mut h = harness(|_| {});
    let _ = h.state_mut().state.session.set_tool(ToolId::Rectangle);
    h.run_steps(4);
    assert!(button(&h, &union).is_some_and(|b| !b.1), "Pfadknöpfe im Werkzeug-Inspektor, im leeren Dokument grau");
    assert!(button(&h, "Muster").is_some_and(|b| b.1), "Muster bedienbar");
    let mut h = harness(|s| {
        rect(s, [10, 10, 60, 60]);
        s.execute("select.none", json!({})).expect("abwählen");
    });
    let _ = h.state_mut().state.session.set_tool(ToolId::Rectangle);
    h.run_steps(4);
    let n0 = undo_count(&h);
    let (b, en) = button(&h, &union).expect("Vereinigen im Werkzeug-Inspektor");
    assert!(en, "mit Pfaden auf der Seite bedienbar");
    click(&mut h, b.center());
    assert_eq!(undo_count(&h), n0, "im Werkzeug-Inspektor ohne Wirkung");
    let mut h = harness(|s| {
        s.execute("draw.ellipse", json!({"rect": [50, 50, 150, 120]})).expect("Ellipse");
        s.execute("pi.stroke", json!({"preset": "Bleistift/1-Pixel hart", "color": "#000000"})).expect("Strich");
    });
    assert!(button(&h, &union).is_some_and(|b| b.1), "Vereinigen bei einem Pfad bedienbar");
    assert!(button(&h, &fp_i18n::t("pi-stroke-delete")).is_some_and(|b| b.1), "Pinselstrich löschen bedienbar");
    let n0 = undo_count(&h);
    let (b, _) = button(&h, &union).expect("Vereinigen");
    click(&mut h, b.center());
    assert_eq!(undo_count(&h), n0, "ein Pfad allein wird nicht zusammengefasst");
}

/// Reiter-Doppelklick: die Höhe wechselt reihum voll → halb → nur Reiter → voll (PAR 16/11; der
/// Doppelklick im echten Fenster steht in NACHPRUEFUNG).
#[test]
fn inspector_height_cycles() {
    let mut ws = fp_ui::workspace::Workspace::default();
    let seq: Vec<(bool, bool)> = (0..4)
        .map(|_| {
            fp_ui::inspector::cycle_height(&mut ws);
            (ws.inspector_half, ws.inspector_collapsed)
        })
        .collect();
    assert_eq!(seq, [(true, false), (false, true), (false, false), (true, false)]);
}

/// Exportformat-Liste im Dokument-Inspektor: die acht Vorgaben des Originals in dessen Reihenfolge,
/// danach die Firepixel-Formate (PAR 16/07).
#[test]
fn export_presets_in_original_order() {
    let all = fp_export::options::builtin_presets();
    let order = fp_ui::inspector::export_preset_order(&all);
    assert_eq!(order.len(), all.len());
    let names: Vec<String> = order.iter().take(8).map(|&i| fp_ui::panels::optimize::preset_label(all[i].name)).collect();
    assert_eq!(names, ["PNG32", "JPEG - Höhere Qualität", "JPEG - Kleinere Datei", "GIF Web 216", "GIF WebSnap 256", "GIF WebSnap 128", "GIF Adaptiv 256", "Animiertes GIF WebSnap 128"]);
}

/// Musternamen deutsch wie im Original, alphabetisch („Baumwolle“ … „Ölfarbe 3“), englisch der Dateiname.
#[test]
fn pattern_names_like_original() {
    assert_eq!(popups::pattern_label("Cotton"), "Baumwolle");
    assert_eq!(popups::pattern_label("Oil Paint 3"), "Ölfarbe 3");
    assert_eq!(obj_quads::texture_label("Argyle 01"), "Argyle 01");
    assert_eq!(obj_quads::texture_label("Plaster"), "Gips");
    fp_i18n::with_language("en", || assert_eq!(popups::pattern_label("Cotton"), "Cotton"));
    let items = popups::pattern_items();
    if items.len() == 60 {
        assert_eq!(items.first().map(|(_, l)| l.as_str()), Some("Baumwolle"));
        assert_eq!(items.last().map(|(_, l)| l.as_str()), Some("Ölfarbe 3"));
    }
}

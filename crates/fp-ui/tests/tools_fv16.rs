//! FV-16 (PLAN P-LW2): Werkzeug-Inspektor von Verlauf und Farbe ersetzen wie im Original.
//! Verlauf: nur das Farbfeld ohne die Füllart-Knöpfe (PARITAET_A W25/W26), „Auswahl füllen“ und
//! Toleranz getrennt vom Farbeimer gemerkt; Farbe ersetzen: Werksvorgaben Toleranz 32, Stärke 200,
//! „Von“ #00FF00, „in“ #FF0000 (W8), Änderungen landen in den Voreinstellungen des Werkzeugs.
//! Realistische Eingaben wie egui-winit unter Windows (je Ereignis ein Bild).

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::Harness;
use fp_engine::ToolId;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

const W: f32 = 1800.0;
const HH: f32 = 1130.0;

fn harness() -> H {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 300, "background": "#ffffff"})).expect("Dokument");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(W, HH)).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn key(h: &mut H, k: Key, m: Modifiers) {
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: m });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: m });
}

fn text(h: &mut H, t: &str) {
    for c in t.chars() {
        ev(h, Event::Text(c.to_string()));
    }
}

fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

fn tool(h: &mut H, t: ToolId) {
    let _ = h.state_mut().state.session.set_tool(t);
    h.run_steps(3);
}

/// Eingabefelder im Inspektor (unterer Fensterteil): Rechteck, Text, bedienbar.
fn inputs(h: &H) -> Vec<(egui::Rect, String, bool)> {
    let mut v: Vec<(egui::Rect, String, bool)> = h
        .get_all(By::new().role(egui::accesskit::Role::TextInput))
        .map(|n| (n.rect(), n.value().unwrap_or_default(), !n.accesskit_node().is_disabled()))
        .filter(|(r, _, _)| r.top() > HH - 160.0)
        .collect();
    v.sort_by(|a, b| a.0.top().total_cmp(&b.0.top()).then(a.0.left().total_cmp(&b.0.left())));
    v
}

fn input_with(h: &H, value: &str) -> egui::Rect {
    let all = inputs(h);
    let m: Vec<_> = all.iter().filter(|(_, v, _)| v == value).collect();
    assert_eq!(m.len(), 1, "genau ein Feld mit „{value}“: {all:?}");
    m[0].0
}

/// Feld anklicken, Inhalt markieren, Text tippen (je Zeichen ein Ereignis), Eingabe.
fn type_into(h: &mut H, field: egui::Rect, s: &str) {
    click(h, field.center());
    key(h, Key::End, Modifiers::NONE);
    key(h, Key::Home, Modifiers::SHIFT);
    text(h, s);
    key(h, Key::Enter, Modifiers::NONE);
    h.run_steps(2);
}

/// Wurden im letzten Bild die Füllart-Knöpfe Keine/Einfarbig/Verlauf des Werkzeug-Inspektors
/// gezeichnet (Widget-Antwort mit der Id aus `obj_quads::fill_panel`)?
fn kind_buttons_shown(h: &H) -> usize {
    let base = egui::Id::new(("pi-fill", true));
    (0u8..3).filter(|k| h.ctx.read_response(egui::Id::new(base.with(("kind", *k)))).is_some()).count()
}

// ------------------------------------------------------------------ W25/W26 Verlauf

#[test]
fn w25_gradient_tool_shows_swatch_without_kind_buttons() {
    let mut h = harness();
    tool(&mut h, ToolId::PaintBucket);
    assert_eq!(kind_buttons_shown(&h), 3, "Füllwerkzeug: Knöpfe wie im Original");
    tool(&mut h, ToolId::Gradient);
    assert_eq!(kind_buttons_shown(&h), 0, "Verlauf: nur das Farbfeld");
    assert!(fp_ui::inspector::obj_quads::tool_swatch_gradient(&h.ctx).is_some(), "Farbfeld zeigt den Verlauf");
    // hin und her: kein Rest, immer gleich
    for _ in 0..5 {
        tool(&mut h, ToolId::PaintBucket);
        assert_eq!(kind_buttons_shown(&h), 3);
        tool(&mut h, ToolId::Gradient);
        assert_eq!(kind_buttons_shown(&h), 0);
    }
    // Rechteck-Werkzeug behält die Knöpfe
    tool(&mut h, ToolId::Rectangle);
    assert_eq!(kind_buttons_shown(&h), 3);
}

#[test]
fn w25_fill_selection_and_tolerance_per_tool() {
    let mut h = harness();
    tool(&mut h, ToolId::Gradient);
    let n = h.get_by_role_and_label(egui::accesskit::Role::CheckBox, "Auswahl füllen");
    assert_eq!(n.accesskit_node().toggled(), Some(egui::accesskit::Toggled::True), "Verlauf: Vorgabe ☑");
    n.click();
    h.run_steps(3);
    assert!(!st(&h).session.tools.options.gradient_fill_selection);
    assert!(!st(&h).session.prefs.get_bool("GradientToolFillsSelection", true), "in der Voreinstellung gemerkt");
    // jetzt ist die Toleranz des Verlaufs bedienbar und eigenständig
    let f = input_with(&h, "32");
    type_into(&mut h, f, "12");
    assert_eq!(st(&h).session.tools.options.gradient_tolerance, 12);
    assert_eq!(st(&h).session.tools.options.fill_tolerance, 32, "Farbeimer unberührt");
    tool(&mut h, ToolId::PaintBucket);
    let n = h.get_by_role_and_label(egui::accesskit::Role::CheckBox, "Auswahl füllen");
    assert_eq!(n.accesskit_node().toggled(), Some(egui::accesskit::Toggled::False), "Farbeimer: Vorgabe ☐");
    assert!(inputs(&h).iter().any(|(_, v, en)| v == "32" && *en), "Farbeimer-Toleranz 32 bedienbar");
    tool(&mut h, ToolId::Gradient);
    assert!(inputs(&h).iter().any(|(_, v, _)| v == "12"), "Verlauf behält 12");
    assert!(!st(&h).session.tools.options.gradient_fill_selection);
}

// ------------------------------------------------------------------ W8 Farbe ersetzen

#[test]
fn w8_replace_color_factory_defaults() {
    let mut h = harness();
    tool(&mut h, ToolId::ReplaceColor);
    let ta = fp_ui::inspector::obj_quads::tool_attrs(&h.ctx);
    assert_eq!((ta.retouch_size, ta.tolerance, ta.strength), (13, 32, 200));
    assert!(inputs(&h).iter().any(|(_, v, _)| v == "200"), "Stärke 200: {:?}", inputs(&h));
    // die Farbfelder zeigen die Voreinstellungen des Werkzeugs (Werksvorgabe grün → rot)
    let p = &st(&h).session.prefs;
    let (src, dst) = (p.get_color("ReplaceColorToolSrcColor", fp_color::Argb(0)), p.get_color("ReplaceColorToolDestColor", fp_color::Argb(0)));
    assert_eq!((src.0 | 0xff00_0000, dst.0 | 0xff00_0000), (0xff00_ff00, 0xffff_0000));
    // Strich-/Füllfarbe ändern sich nicht durch das Werkzeug
    let (sc, fc) = (st(&h).session.tools.stroke_color, st(&h).session.tools.fill_color);
    // Stärke tippen → Voreinstellung `ReplaceColorToolStrength`
    let f = input_with(&h, "200");
    type_into(&mut h, f, "150");
    assert_eq!(st(&h).session.prefs.get_int("ReplaceColorToolStrength", 0), 150);
    // 50× derselbe Wert: kein Unterschied
    for _ in 0..3 {
        let f = input_with(&h, "150");
        type_into(&mut h, f, "150");
    }
    assert_eq!(st(&h).session.prefs.get_int("ReplaceColorToolStrength", 0), 150);
    // Werkzeugwechsel und zurück: Wert bleibt
    tool(&mut h, ToolId::RubberStamp);
    tool(&mut h, ToolId::ReplaceColor);
    assert_eq!(fp_ui::inspector::obj_quads::tool_attrs(&h.ctx).strength, 150);
    assert_eq!((st(&h).session.tools.stroke_color, st(&h).session.tools.fill_color), (sc, fc));
}

/// Verlauf-Werkzeug: das Farbfeld zeigt die eigenen Werkzeugfarben (Werksvorgabe Schwarz → Weiß),
/// Füll- und Strichfarbe ändern es nicht (Original: Füllfarbe #808080, Strichfarbe #C04040 →
/// Farbfeld Schwarz → Weiß); geänderte Werkzeugfarben erscheinen sofort.
#[test]
fn w26_gradient_tool_swatch_uses_own_colors() {
    let mut h = harness();
    tool(&mut h, ToolId::Gradient);
    let sw = || fp_ui::inspector::obj_quads::tool_swatch_gradient;
    assert_eq!(sw()(&h.ctx), Some(vec![0xff00_0000, 0xffff_ffff]));
    h.state_mut().state.session.tools.fill_color = fp_color::Argb(0xff80_8080);
    h.state_mut().state.session.tools.stroke_color = fp_color::Argb(0xffc0_4040);
    h.run_steps(2);
    assert_eq!(sw()(&h.ctx), Some(vec![0xff00_0000, 0xffff_ffff]), "Füll-/Strichfarbe ohne Einfluss");
    h.state_mut().state.session.tools.options.gradient_colors = vec![fp_color::Argb(0xffff_0000), fp_color::Argb(0xff00_00ff), fp_color::Argb(0xff00_ff00)];
    h.run_steps(2);
    assert_eq!(sw()(&h.ctx), Some(vec![0xffff_0000, 0xff00_00ff, 0xff00_ff00]));
    tool(&mut h, ToolId::PaintBucket);
    assert_eq!(sw()(&h.ctx), None, "Füllwerkzeug einfarbig");
}

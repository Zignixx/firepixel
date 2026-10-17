//! T-2 (BEFUNDE Z. 256, re\24 Nr. 7/9): Werkzeug-Inspektor des Pinsels im deutschen Layout –
//! „Pinselstrich bearbeiten“ ganz lesbar mit 3 px Rand je Seite (wie CF-X „Bearbeiten…“), Popups
//! von Größe/Rand/Menge/Deckkraft als senkrechte Schieber 24 × 222, Mischmodus-Liste wie das Original.

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::Harness;
use fp_engine::ToolId;
use fp_ui::app_state::UiSettings;
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

const W: f32 = 1800.0;
const HH: f32 = 1130.0;

fn harness() -> H {
    fp_i18n::set_language("de");
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 300, "background": "#ffffff"})).expect("Dokument");
    s.set_tool(ToolId::Brush).expect("Pinsel");
    s.tools.options.brush_size = 35.0;
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(W, HH)).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

fn escape(h: &mut H) {
    ev(h, Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

/// Zahlenfelder im Inspektor (unterer Fensterteil).
fn inputs(h: &H) -> Vec<(egui::Rect, String)> {
    let mut v: Vec<(egui::Rect, String)> = h.get_all(By::new().role(egui::accesskit::Role::TextInput)).map(|n| (n.rect(), n.value().unwrap_or_default())).filter(|(r, _)| r.top() > HH - 160.0).collect();
    v.sort_by(|a, b| a.0.top().total_cmp(&b.0.top()).then(a.0.left().total_cmp(&b.0.left())));
    v
}

fn sliders(h: &H) -> Vec<egui::Rect> {
    h.query_all(By::new().role(egui::accesskit::Role::Slider)).map(|n| n.rect()).collect()
}

/// Pfeil ▾ rechts neben einem Zahlenfeld (1 DLU Abstand, 9 DLU breit).
fn arrow_of(field: egui::Rect) -> Pos2 {
    Pos2::new(field.right() + 3.0 + 1.5 + 6.75, field.center().y)
}

#[test]
fn stroke_edit_button_fully_readable_with_three_px_margin() {
    let mut h = harness();
    fp_ui::inspector::fields::set_audit(&h.ctx, true);
    h.run_steps(2);
    let audit = fp_ui::inspector::fields::take_audit(&h.ctx);
    let hits: Vec<_> = audit.iter().filter(|a| a.text == "Pinselstrich bearbeiten").collect();
    assert!(!hits.is_empty(), "Knopf im Werkzeug-Inspektor des Pinsels");
    for a in hits {
        assert_eq!(a.kind, "button");
        assert!(a.fit < 2, "nicht gekürzt: {a:?}");
        // 3 px Rand je Seite: der Text liegt innerhalb des um 3 px verkleinerten Knopfs
        assert!(a.drawn.left() >= a.avail.left() - 0.5 && a.drawn.right() <= a.avail.right() + 0.5, "3 px Rand: {a:?}");
    }
    assert!(h.query_all(By::new().role(egui::accesskit::Role::Button)).any(|n| n.accesskit_node().label().as_deref() == Some("Pinselstrich bearbeiten")));
}

#[test]
fn popup_sliders_are_vertical_24_by_222() {
    let mut h = harness();
    let fields = inputs(&h);
    assert!(!fields.is_empty(), "Zahlenfelder im Pinsel-Inspektor: {fields:?}");
    let mut seen = 0;
    for (f, val) in fields {
        click(&mut h, arrow_of(f));
        let s = sliders(&h);
        if s.len() != 1 {
            escape(&mut h);
            continue;
        }
        seen += 1;
        let r = s[0];
        // Schieberfläche 20 × 218 + 1 px Innenrand + 1 px Rahmen je Seite = 24 × 222 (re\24 Nr. 9;
        // Spur 212 nach re 29 §8 + 2 × 3)
        assert_eq!((r.width(), r.height()), (20.0, 218.0), "Feld „{val}“: {r:?}");
        let outer = (r.width() + 2.0 * 2.0, r.height() + 2.0 * 2.0);
        assert_eq!(outer, (24.0, 222.0));
        let area = h.ctx.memory(|m| m.areas().visible_layer_ids().into_iter().filter_map(|l| m.area_rect(l.id)).find(|a| a.contains_rect(r) && a.width() < 40.0));
        if let Some(a) = area {
            assert_eq!((a.width().round(), a.height().round()), (24.0, 222.0), "Popup-Fläche „{val}“: {a:?}");
        }
        escape(&mut h);
    }
    assert!(seen >= 3, "Größe, Rand, Menge/Deckkraft mit Popup-Schieber (gesehen: {seen})");
}

#[test]
fn brush_blend_list_like_original() {
    let names = fp_ui::inspector::texts::blend_labels();
    let want = "Normal, Durchschnitt, Auflösen, -----, Abdunkeln, Multiplizieren, Farbig nachbelichten, Umgek. farb. nachb., Weich nachbelichten, Linear nachbelichten, -----, Aufhellen, Negativ multiplizieren, Farbig abwedeln, Umgek. farb. abw., Weich abwedeln, Linear abwedeln, -----, Überlagern, Weiches Licht, Unscharfes Licht, Hartes Licht, Strahlendes Licht, Lineares Licht, Lichtpunkt, Hart mischen, -----, Differenz, Ausschluss, Negation, -----, Farbton, Sättigung, Farbe, Luminanz, Rot, Grün, Blau, -----, Spiegeln, Glühen, Abkühlen, Erwärmen, -----, Additiv, Subtraktiv, Subtrahieren, Interpolation, Stempel, XOR, -----, Umkehren, Farbton, -----, Löschen";
    let norm: Vec<String> = names.iter().map(|n| if n.chars().all(|c| c == '-' || c == '─' || c == '—') { "-----".to_string() } else { n.clone() }).collect();
    assert_eq!(norm, want.split(", ").map(String::from).collect::<Vec<_>>());
}

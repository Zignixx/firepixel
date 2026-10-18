//! F19 (Tweak-Liste 16.10., A3-6): Bitmap-Auswahl wiederherstellen (TW-66), Esc-Kette (TW-71),
//! Bedienfeldmenüs (TW-73/74), Segment-Miniatur (TW-76). TW-69 steht in `f18.rs`.

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{By, Queryable};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
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

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn key(h: &mut H, k: Key) {
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: Modifiers::NONE });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: Modifiers::NONE });
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

fn sel(h: &H) -> usize {
    st(h).session.active().map(|d| d.selection.elements.len()).unwrap_or(0)
}

fn key_mod(h: &mut H, k: Key, m: Modifiers) {
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: m });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: m });
}

/// R1 gewählt, dazu R2.
fn setup() -> H {
    harness(|s| {
        let v = s.execute("draw.rectangle", json!({"rect": [20, 20, 140, 90], "fillColor": "#ff0000", "strokeWidth": 0})).expect("R1");
        let r1 = v.value["id"].as_u64().expect("id");
        s.execute("draw.rectangle", json!({"rect": [200, 40, 290, 110], "fillColor": "#0000ff", "strokeWidth": 0})).expect("R2");
        s.execute("select.set", json!({"ids": [r1]})).expect("Auswahl");
    })
}

fn field_with(h: &H, v: &str) -> egui::Rect {
    let r: Vec<egui::Rect> = h
        .get_all(By::new().role(egui::accesskit::Role::TextInput))
        .filter(|n| n.rect().top() > HH - 200.0 && n.value().as_deref() == Some(v))
        .map(|n| n.rect())
        .collect();
    let all: Vec<(Option<String>, egui::Rect)> = h.get_all(By::new().role(egui::accesskit::Role::TextInput)).map(|n| (n.value(), n.rect())).collect();
    assert!(!r.is_empty(), "Feld mit {v}: {all:?}");
    r[0]
}

/// TW-71: Esc im Zahlenfeld des Inspektors bleibt im Feld (wie ein Win32-Eingabefeld, FV-1/F7) und
/// hebt die Auswahl nicht auf (Original `xs-shortcuts/16`: Auswahl bleibt).
#[test]
fn tw71_escape_in_field_keeps_selection() {
    let mut h = setup();
    let w = field_with(&h, "120");
    click(&mut h, w.center());
    assert!(h.ctx.text_edit_focused(), "Feld hat den Fokus");
    // Strg+A markiert den Feldtext (TW-78, Original Lauf 8 `xs-shortcuts/16`)
    key_mod(&mut h, Key::A, Modifiers::COMMAND);
    ev(&mut h, Event::Text("123".into()));
    field_with(&h, "123");
    for _ in 0..2 {
        key(&mut h, Key::Escape);
        h.run_steps(2);
        assert_eq!(sel(&h), 1, "Esc im Feld hebt die Auswahl nicht auf");
    }
}

fn menu_rows(h: &H, p: fp_ui::panels::PanelId) -> Vec<String> {
    fp_ui::menu::build_panel_menu(st(h), p)
        .iter()
        .map(|n| match n {
            fp_ui::menu::Node::Separator => "---".to_string(),
            fp_ui::menu::Node::Item { label, enabled, .. } => format!("{label}{}", if *enabled { "" } else { " (grau)" }),
            fp_ui::menu::Node::Sub { label, .. } => format!("{label} ▸"),
        })
        .collect()
}

/// TW-73/74: Bedienfeldmenüs Allgemeine Bibliothek und Automatische Formen mit den Einträgen des
/// Originals (ohne Wirkung grau), danach „Hilfe“; ellipses as in the reference (run 17, TW-125).
#[test]
fn tw73_tw74_library_and_auto_shapes_menus() {
    let h = setup();
    assert_eq!(
        menu_rows(&h, fp_ui::panels::PanelId::CommonLibrary),
        ["Neuer Ordner... (grau)", "Löschen (grau)", "Duplizieren... (grau)", "Umbenennen (grau)", "Neu laden (grau)", "---", "Hilfe"]
    );
    assert_eq!(menu_rows(&h, fp_ui::panels::PanelId::AutoShapes), ["Zusätzliche automatische Formen... (grau)", "---", "Hilfe"]);
}

/// TW-76: Miniatur eines gewählten Segments zeigt eine Fläche in Segmentfarbe an seiner Lage im
/// verkleinerten Dokument (Original `inspector-web/01`), kein leeres Schachbrett.
#[test]
fn tw76_slice_thumbnail_shows_slice_area() {
    let mut h = harness(|s| {
        s.execute("draw.rectangle", json!({"rect": [20, 200, 180, 280], "fillColor": "#ff0000", "strokeWidth": 0})).expect("R");
        s.execute("edit.insert.slice", json!({})).expect("slice");
    });
    h.run_steps(3);
    let r = fp_ui::inspector::selection_thumb_rect(&h.ctx).expect("Segmentfläche gezeichnet");
    // 500 × 300 in 25 × 21: Segment 160 × 80 → etwa 8 × 4
    // Seitenverhältnis des Segments (2 : 1), deutlich kleiner als die Miniatur
    assert!((r.width() / r.height() - 2.0).abs() < 0.3 && r.width() < 20.0, "{r:?}");
}

/// TW-66: Auswahl speichern, Strg+D, Auswählen ▸ Bitmap-Auswahl wiederherstellen… öffnet den Dialog
/// wie im Original (Lauf 4/7), auch ohne aktiven Auswahlrahmen.
#[test]
fn tw66_restore_dialog_opens_after_deselect() {
    let mut h = harness(|s| {
        s.execute("select.pixels", json!({"shape": "rect", "rect": [100, 100, 300, 250]})).expect("Rahmen");
        s.execute("select.saveBitmapSelection", json!({"name": "Standard"})).expect("speichern");
        s.execute("select.none", json!({})).expect("aufheben");
    });
    h.run_steps(2);
    let (on, _) = st(&h).session.command_state("select.restoreBitmapSelection");
    assert!(on, "Menüeintrag aktiv");
    assert!(fp_ui::dialogs::open_for_command(&mut h.state_mut().state, "select.restoreBitmapSelection"), "Dialog öffnet");
}

fn texts(shape: &egui::epaint::Shape, out: &mut Vec<(String, egui::Rect)>) {
    match shape {
        egui::epaint::Shape::Text(t) => out.push((t.galley.job.text.clone(), egui::Rect::from_min_size(t.pos, t.galley.size()))),
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
        _ => {}
    }
}

fn all_texts(h: &H) -> Vec<(String, egui::Rect)> {
    let mut all = Vec::new();
    for cs in &h.output().shapes {
        texts(&cs.shape, &mut all);
    }
    all
}

/// TW-77: Symboleigenschaften mit gewählter Instanz ohne Parameter zeigen wie im Original die
/// (leere) Tabelle „Name | Wert“ statt des Hinweises „Kein Rich Symbol ausgewählt“.
#[test]
fn tw77_symbol_properties_table_for_plain_instance() {
    let mut h = harness(|s| {
        s.execute("draw.rectangle", json!({"rect": [20, 20, 140, 90], "fillColor": "#ff0000", "strokeWidth": 0})).expect("R");
        s.execute("modify.symbol.convertToSymbol", json!({"name": "Sym"})).expect("Symbol");
    });
    let tab = all_texts(&h).into_iter().find(|(t, _)| t == "Symboleigenschaften").expect("Reiter").1;
    click(&mut h, tab.center());
    h.run_steps(4);
    let all: Vec<String> = all_texts(&h).into_iter().map(|t| t.0).collect();
    assert_eq!(sel(&h), 1, "Instanz gewählt");
    assert!(all.iter().any(|t| t == "Wert"), "Tabellenkopf sichtbar: {all:?}");
    assert!(!all.iter().any(|t| t == "Kein Rich Symbol ausgewählt"), "kein Hinweis bei gewählter Instanz");
}

/// TW-48/63: ≡ ▸ Neuer Stil… öffnet mit und ohne Auswahl den Dialog (Lauf 7 ohne Dialog: Läufer).
#[test]
fn tw48_new_style_opens_dialog_with_and_without_selection() {
    for selected in [true, false] {
        let mut h = setup();
        if !selected {
            h.state_mut().state.session.execute("select.none", json!({})).expect("none");
        }
        let nodes = fp_ui::menu::build_panel_menu(st(&h), fp_ui::panels::PanelId::Styles);
        let cmd = nodes
            .iter()
            .find_map(|n| match n {
                fp_ui::menu::Node::Item { label, action: fp_ui::menu::ItemAction::Command(c), enabled: true, .. } if label == "Neuer Stil..." => Some(c.clone()),
                _ => None,
            })
            .expect("Neuer Stil... aktiv");
        h.state_mut().state.push(fp_ui::app_state::Action::Run(cmd, json!({})));
        h.run_steps(4);
        assert!(st(&h).dialogs.any_open(), "Dialog offen (Auswahl: {selected})");
    }
}

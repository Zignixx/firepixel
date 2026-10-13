//! FIX-B: Befunde aus CT-1b (Menüs im echten Fenster, 13.10.2026).
//! - MN-5: `prompt()` in Befehlsskripten fragt in der Oberfläche (Dialog), headless Vorgabewert
//! - MN-6: Klick auf eine Trennlinie (bzw. knapp darüber/darunter) in einem Menü tut nichts
//! - MN-4: Vorschau im Browser / „Demo – aktuelles Dokument“ meldet sich in der Statuszeile,
//!   Vorschauordner je Prozess eindeutig
//! - N1: Verlaufsnamen in der Sprache der Oberfläche, auch wenn der Befehl in der Antwortsprache
//!   der MCP-Brücke lief

use fp_ui::app_state::{Action, AppState, UiSettings};
use fp_ui::menu::{ItemAction, Node};
use serde_json::{json, Value};

fn state() -> (AppState, egui::Context) {
    fp_engine::shell::disable();
    let mut session = fp_ui::ui_commands::new_session(fp_engine::Preferences::new());
    session.execute("file.new", json!({"width": 300, "height": 200})).expect("Dokument");
    (AppState::new(session, UiSettings::default(), None), egui::Context::default())
}

fn elements(st: &AppState) -> usize {
    let v = serde_json::to_value(&*st.session.active().expect("Dokument").doc).unwrap_or(Value::Null);
    v.to_string().matches("\"common\"").count()
}

fn rect_widths(st: &AppState) -> Vec<f32> {
    let d = st.session.active().expect("Dokument");
    let mut v = Vec::new();
    d.doc.walk_elements(|_, _, e| {
        if let Some(b) = fp_engine::docops::element_bounds(&d.doc, e) {
            v.push(b.right - b.left);
        }
    });
    v
}

const CODE: &str = "var w = prompt('Neue Breite?', '40'); var h = prompt('Neue Höhe?', '30');\n\
if (w != null && h != null) fw.getDocumentDOM().addNewRectanglePrimitive({left: 0, top: 0, right: Number(w), bottom: Number(h)}, 0);";

#[test]
fn script_prompt_asks_in_a_dialog_and_reruns_with_answers() {
    let (mut st, ctx) = state();
    let n = elements(&st);
    let undo = st.session.active().map(|d| d.history.undo_count()).unwrap_or(0);
    st.push(Action::Run("commands.runScript".into(), json!({"code": CODE})));
    st.process_actions(&ctx);
    let d = st.dialogs.script_prompt.clone().expect("Eingabedialog statt still den Vorgabewert");
    assert_eq!((d.text.as_str(), d.value.as_str()), ("Neue Breite?", "40"));
    assert!(st.dialogs.any_open());
    assert_eq!(elements(&st), n, "vor der Antwort keine Änderung");
    assert_eq!(st.session.active().map(|d| d.history.undo_count()).unwrap_or(0), undo, "kein Verlaufsschritt");
    // OK mit 120 → zweite Frage
    st.dialogs.script_prompt = None;
    st.push(d.rerun(Some("120")));
    st.process_actions(&ctx);
    let d = st.dialogs.script_prompt.clone().expect("zweite Frage");
    assert_eq!((d.text.as_str(), d.value.as_str()), ("Neue Höhe?", "30"));
    // OK mit dem Vorgabewert → Rechteck 120 breit, genau ein Schritt, kein Dialog mehr
    st.dialogs.script_prompt = None;
    st.push(d.rerun(Some(&d.value)));
    st.process_actions(&ctx);
    assert!(st.dialogs.script_prompt.is_none());
    assert_eq!(elements(&st), n + 1);
    assert_eq!(rect_widths(&st), vec![120.0]);
    assert_eq!(st.session.active().map(|d| d.history.undo_count()).unwrap_or(0), undo + 1);
    assert!(st.dialogs.message.is_none(), "„prompt:“ erscheint nicht als Meldung");
    // Abbrechen bei der ersten Frage → prompt() liefert null, das Skript tut nichts
    let (mut st, ctx) = state();
    st.push(Action::Run("commands.runScript".into(), json!({"code": CODE})));
    st.process_actions(&ctx);
    let d = st.dialogs.script_prompt.take().expect("Frage");
    st.push(d.rerun(None));
    st.process_actions(&ctx);
    let d = st.dialogs.script_prompt.take().expect("zweite Frage nach Abbrechen");
    st.push(d.rerun(None));
    st.process_actions(&ctx);
    assert!(st.dialogs.script_prompt.is_none());
    assert_eq!(elements(&st), n);
    // ohne Oberfläche (Brücke/CLI: direkt über die Sitzung) weiter der Vorgabewert
    let (mut st, _ctx) = state();
    st.session.execute("commands.runScript", json!({"code": CODE})).expect("Skript");
    assert_eq!(rect_widths(&st), vec![40.0]);
}

#[test]
fn script_prompt_dialog_ok_and_escape() {
    let (mut st, ctx) = state();
    st.push(Action::Run("commands.runScript".into(), json!({"code": CODE})));
    st.process_actions(&ctx);
    assert!(st.dialogs.script_prompt.is_some());
    // Dialog zeichnen: Feld mit Vorgabe, Escape bricht ab (→ null, zweite Frage folgt)
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(900.0, 650.0)).build_ui_state(|ui, st: &mut AppState| fp_ui::dialogs::show_all(ui.ctx(), st), st);
    h.run_steps(3);
    assert!(h.state().dialogs.script_prompt.is_some(), "Dialog bleibt offen");
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    let ctx = h.ctx.clone();
    h.state_mut().process_actions(&ctx);
    // Escape = Abbrechen → erneuter Lauf mit null → die zweite Frage erscheint
    let d = h.state().dialogs.script_prompt.clone().expect("zweite Frage nach Escape");
    assert_eq!(d.text, "Neue Höhe?");
    assert_eq!(d.answers, vec![Value::Null]);
}

/// Bedienfeld-≡-Menü (Popup wie in `docking.rs`) mit Eintrag, Trennlinie, Eintrag: Klicks auf die
/// Linie und knapp darüber/darunter lösen nichts aus und lassen das Menü offen, ein Klick in den
/// zweiten Eintrag schon.
#[test]
fn separator_clicks_do_nothing() {
    use std::cell::Cell;
    let nodes = vec![
        Node::Item { label: "Erster".into(), shortcut: None, action: ItemAction::Command("a.first".into()), enabled: true, checked: None },
        Node::Separator,
        Node::Item { label: "Zweiter".into(), shortcut: None, action: ItemAction::Command("a.second".into()), enabled: true, checked: None },
    ];
    let ctx = egui::Context::default();
    // wie im echten Fenster unter Windows mit 125 % Skalierung (CT-1b lief so)
    ctx.set_zoom_factor(1.25);
    let button = Cell::new(egui::Rect::NOTHING);
    let frame = |ctx: &egui::Context, events: Vec<egui::Event>| -> (Vec<Action>, Vec<egui::Shape>) {
        let mut out = Vec::new();
        let raw = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0))), events, ..Default::default() };
        let mut o = ctx.run_ui(raw, |ui| {
            egui::Area::new(egui::Id::new("panel")).fixed_pos(egui::pos2(40.0, 40.0)).show(ui.ctx(), |ui| {
                let resp = ui.button("≡");
                button.set(resp.rect);
                fp_ui::menu::popup_menu(ui, &resp, &nodes, &mut out);
            });
        });
        o.textures_delta.clear();
        (out, o.shapes.into_iter().map(|s| s.shape).collect())
    };
    let click = |ctx: &egui::Context, p: egui::Pos2| -> Vec<Action> {
        let mut acts = Vec::new();
        // wie die Brücke (`ui_input`): Bewegen, Drücken und Loslassen ohne Bild dazwischen
        for ev in [
            vec![
                egui::Event::PointerMoved(p),
                egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() },
                egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() },
            ],
            vec![],
            vec![],
        ] {
            acts.extend(frame(ctx, ev).0);
        }
        acts
    };
    let _ = frame(&ctx, Vec::new());
    let _ = frame(&ctx, Vec::new());
    // ≡ anklicken: das Menü geht auf
    let _ = click(&ctx, button.get().center());
    // Lage der Trennlinie aus den gezeichneten Formen
    fn walk(s: &egui::Shape, out: &mut Vec<(f32, f32, f32)>) {
        match s {
            egui::Shape::LineSegment { points, .. } if (points[0].y - points[1].y).abs() < 0.01 && (points[1].x - points[0].x).abs() > 4.0 => {
                out.push((points[0].y, points[0].x.min(points[1].x), points[0].x.max(points[1].x)))
            }
            egui::Shape::Vec(v) => v.iter().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let line = || {
        let (_, shapes) = frame(&ctx, Vec::new());
        let mut v = Vec::new();
        shapes.iter().for_each(|s| walk(s, &mut v));
        v.into_iter().find(|l| l.0 > button.get().bottom())
    };
    let (y, x0, x1) = line().expect("Menü offen, Trennlinie gezeichnet");
    // mitten auf der Linie und rechts davon (die ganze Zeile gehört zum Trenner)
    for x in [(x0 + x1) / 2.0, x1 + 4.0] {
        for dy in [-3.0, -1.0, 0.0, 1.0, 3.0] {
            let a = click(&ctx, egui::pos2(x, y + dy));
            assert!(a.is_empty(), "Klick ({x}, {dy:+} px) an der Trennlinie löst {a:?} aus");
            assert!(line().is_some(), "Menü bleibt nach dem Klick auf den Trenner offen");
        }
    }
    let a = click(&ctx, egui::pos2((x0 + x1) / 2.0, y + 4.5 + 11.0));
    assert!(matches!(a.as_slice(), [Action::Run(c, _)] if c == "a.second"), "zweiter Eintrag: {a:?}");
}

#[test]
fn preview_in_browser_reports_status_with_unique_folder() {
    let (mut st, ctx) = state();
    st.session.execute("draw.rectangle", json!({"rect": [10, 10, 60, 40], "fillColor": "#ff0000"})).expect("Rechteck");
    // „Demo – aktuelles Dokument“ ruft die Vorschau aus einem Befehlsskript
    st.push(Action::Run("commands.runScript".into(), json!({"code": "fp.run(\"file.previewInBrowser.previewAllPagesInPrimaryBrowser\", {});"})));
    st.process_actions(&ctx);
    let id = st.session.active_id().expect("Dokument").0;
    let dir = fp_engine::commands::export::preview_dir(id);
    let status = st.status.as_ref().map(|s| s.0.clone()).unwrap_or_default();
    assert!(status.contains("Vorschau erstellt") && status.contains(&dir.to_string_lossy().to_string()), "Statuszeile: {status:?}");
    assert!(dir.to_string_lossy().contains(&format!("{}-doc{id}", std::process::id())), "je Prozess eindeutig: {dir:?}");
    assert!(std::fs::read_dir(&dir).map(|r| r.count()).unwrap_or(0) > 0, "Vorschau geschrieben");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn history_labels_in_ui_language_when_run_in_agent_language() {
    fp_i18n::set_language("de");
    let steps = |st: &mut AppState| {
        st.session.execute("draw.rectangle", json!({"rect": [10, 10, 60, 40]})).expect("Rechteck");
        let a = st.session.active().and_then(|d| d.history.undo_label().map(str::to_string)).unwrap_or_default();
        st.session.execute("edit.nudge.right", json!({})).expect("Verschieben");
        let b = st.session.active().and_then(|d| d.history.undo_label().map(str::to_string)).unwrap_or_default();
        st.session.execute("select.none", json!({})).ok();
        (a, b)
    };
    // Bezug: dieselben Schritte direkt in der deutschen Oberfläche
    let (mut st, _ctx) = state();
    let de = steps(&mut st);
    // über die Brücke: Antwortsprache Englisch, Verlauf trotzdem deutsch
    let (mut st, _ctx) = state();
    let bridge = fp_i18n::with_language("en", || steps(&mut st));
    assert_eq!(bridge, de, "Verlaufsnamen wie in der deutschen Oberfläche");
    let (mut st, _ctx) = state();
    let en = {
        fp_i18n::set_language("en");
        let r = steps(&mut st);
        fp_i18n::set_language("de");
        r
    };
    assert_ne!(en.1, de.1, "Englisch wäre anders ({en:?})");
}

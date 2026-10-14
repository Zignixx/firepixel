//! UX-B-Abweichungen zum Original (BEFUNDE unter IBD, Katalog `docs\UX_ABGLEICH_B.md`), mit
//! realistischen Ereignissen (1/60 s je Bild, Strg = ctrl + command, Text als `Event::Text`):
//! Fokus beim Öffnen, „800 ⇥ 600 ↵“, Eingabe ohne Fokus, letzte Werte in „Neues Dokument“,
//! Gruppentitel und „Vorlagen…“, Wortlaut und Zugriffstasten der Ungespeichert-Abfrage,
//! Ziehen über die Augenspalte, „Als Befehl speichern“ mit Namensabfrage, B/H-Grenze 6000.

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness_with(s: fp_engine::Session) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let tmp = std::env::temp_dir().join("fp-ibd-dialogs-ux");
    let _ = std::fs::create_dir_all(&tmp);
    app.state.file_override = Some(fp_ui::files::FilePickOverride { open: vec![tmp.join("nicht-vorhanden.fw.png")], save: Some(tmp.join("ausgabe.png")) });
    let mut h = Harness::builder().with_size(egui::vec2(1600.0, 1000.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h
}

fn doc_session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80]})).expect("rect");
    s
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn ev(h: &mut H, e: Event) {
    let m = match &e {
        Event::Key { modifiers, .. } | Event::PointerButton { modifiers, .. } => Some(*modifiers),
        _ => None,
    };
    if let Some(m) = m {
        h.event(Event::ModifiersChanged(m));
    }
    h.event(e);
    h.run_steps(1);
}

fn key(h: &mut H, k: Key, m: Modifiers) {
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: m });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: m });
    ev(h, Event::ModifiersChanged(Modifiers::NONE));
}

fn typ(h: &mut H, s: &str) {
    ev(h, Event::Text(s.into()));
}

fn press(h: &mut H, p: Pos2, down: bool) {
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: down, modifiers: Modifiers::NONE });
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

fn doc_count(h: &H) -> usize {
    st(h).session.documents().len()
}

fn open_new_document(h: &mut H) {
    h.state_mut().state.run("file.new", json!({}));
    h.run_steps(3);
    assert!(st(h).dialogs.new_document.is_some(), "Neues Dokument offen");
}

/// D02/D03/D05/D07/D08: Fokus in „Breite“ mit markiertem Inhalt, „800 ⇥ 600 ↵“, letzte Werte,
/// Gruppentitel „…K/…M“, „Vorlagen…“ aktiv.
#[test]
fn new_document_focus_tab_enter_and_last_values() {
    let mut h = harness_with(fp_engine::Session::default());
    open_new_document(&mut h);
    assert!(h.ctx.text_edit_focused(), "Fokus im ersten Feld beim Öffnen");
    assert!(all_texts(&h).iter().any(|(t, _)| t == "Leinwandgröße: 1.1M"), "Gruppentitel wie im Original (660 × 440)");
    let n0 = doc_count(&h);
    typ(&mut h, "800");
    key(&mut h, Key::Tab, Modifiers::NONE);
    typ(&mut h, "600");
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(4);
    assert!(st(&h).dialogs.new_document.is_none(), "Eingabe bestätigt");
    assert_eq!(doc_count(&h), n0 + 1);
    let d = st(&h).session.active().expect("neues Dokument");
    let c = d.doc.pages[d.page].canvas;
    assert_eq!((c.width, c.height), (800.0, 600.0), "800 ⇥ 600 ↵");
    // nächstes Öffnen zeigt die zuletzt bestätigten Werte
    open_new_document(&mut h);
    let nd = st(&h).dialogs.new_document.clone().expect("offen");
    assert_eq!((nd.width.as_str(), nd.height.as_str()), ("800", "600"), "letzte Werte gemerkt");
    // kleiner Wert → „K“
    typ(&mut h, "50");
    key(&mut h, Key::Tab, Modifiers::NONE);
    typ(&mut h, "30");
    h.run_steps(2);
    assert!(all_texts(&h).iter().any(|(t, _)| t == "Leinwandgröße: 5.9K"), "K-Einheit wie im Original");
    // Eingabe ohne Fokus (Rückschritt UX-B): Fokus abgeben, dann Eingabe
    h.ctx.memory_mut(|m| {
        if let Some(f) = m.focused() {
            m.surrender_focus(f);
        }
    });
    h.run_steps(2);
    assert!(!h.ctx.text_edit_focused());
    let n1 = doc_count(&h);
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(4);
    assert!(st(&h).dialogs.new_document.is_none(), "Eingabe ohne Feldfokus bestätigt");
    assert_eq!(doc_count(&h), n1 + 1);
    // „Vorlagen…“ ist aktiv und schließt den Dialog zugunsten der Vorlagenwahl
    open_new_document(&mut h);
    let b: Vec<_> = egui_kittest::kittest::Queryable::get_all(&h, egui_kittest::kittest::By::new().label("Vorlagen...")).collect();
    b.last().expect("Vorlagen…").click();
    drop(b);
    h.run_steps(4);
    assert!(st(&h).dialogs.new_document.is_none(), "„Vorlagen…“ aktiv");
}

/// M02/M03/M06: Wortlaut „Änderungen an <Name> speichern?“, Zugriffstaste N = Nein.
#[test]
fn unsaved_prompt_wording_and_access_keys() {
    let mut h = harness_with(doc_session());
    let id = st(&h).session.active_id().expect("d");
    h.state_mut().state.dialogs.unsaved = Some(fp_ui::dialogs::UnsavedFlow::new(vec![id], fp_ui::dialogs::PendingClose::CloseDocs));
    h.run_steps(3);
    let title = st(&h).session.document(id).map(|d| d.title.clone()).expect("Titel");
    let expect = format!("Änderungen an {} speichern?", fp_ui::dialogs::without_last_extension(&title));
    assert!(all_texts(&h).iter().any(|(t, _)| *t == expect), "Wortlaut {expect}");
    assert!(!expect.contains('„'), "ohne Anführungszeichen");
    key(&mut h, Key::N, Modifiers::NONE);
    h.run_steps(4);
    assert!(st(&h).dialogs.unsaved.is_none(), "N = Nein schließt");
    assert!(st(&h).session.document(id).is_none(), "Dokument ohne Speichern geschlossen");
}

/// E11: Ziehen über die Augenspalte schaltet alle überstrichenen Ebenen.
#[test]
fn eye_column_drag_toggles_all_crossed_layers() {
    let mut s = doc_session();
    s.execute("layer.add", json!({})).expect("l2");
    s.execute("layer.add", json!({})).expect("l3");
    let mut h = harness_with(s);
    // Ebenen-Bedienfeld nach vorn (Klick auf die schon aktive Karte klappt ein: dann noch einmal)
    for _ in 0..2 {
        if all_texts(&h).iter().any(|(t, _)| t == "Ebene 1") {
            break;
        }
        let tab = all_texts(&h).into_iter().filter(|(t, _)| t == "Ebenen").map(|(_, r)| r.center()).max_by(|a, b| a.x.total_cmp(&b.x)).expect("Reiter");
        ev(&mut h, Event::PointerMoved(tab));
        press(&mut h, tab, true);
        press(&mut h, tab, false);
        h.run_steps(30);
    }
    let names: Vec<String> = {
        let d = st(&h).session.active().expect("d");
        fp_engine::docops::own_layers(&d.doc, d.page).into_iter().map(|(_, l)| l.name.clone()).collect()
    };
    let rows: Vec<egui::Rect> = names.iter().filter_map(|n| all_texts(&h).into_iter().filter(|(t, _)| t == n).map(|(_, r)| r).max_by(|a, b| a.left().total_cmp(&b.left()))).collect();
    assert!(rows.len() >= 3, "Ebenenzeilen gezeichnet: {names:?} in {:?}", all_texts(&h).iter().map(|x| &x.0).collect::<Vec<_>>());
    let top = rows.iter().map(|r| r.center().y).fold(f32::MAX, f32::min);
    let bottom = rows.iter().map(|r| r.center().y).fold(f32::MIN, f32::max);
    // Augenspalte: 78 px links vom Namen beginnt die Zeile, das Auge sitzt in den ersten 20 px
    let x = rows[0].left() - 78.0 + 10.0;
    let a = Pos2::new(x, top);
    let b = Pos2::new(x, bottom);
    ev(&mut h, Event::PointerMoved(a));
    press(&mut h, a, true);
    for i in 1..=30 {
        ev(&mut h, Event::PointerMoved(a + (b - a) * (i as f32 / 30.0)));
    }
    press(&mut h, b, false);
    h.run_steps(3);
    let d = st(&h).session.active().expect("d");
    let hidden: Vec<bool> = fp_engine::docops::own_layers(&d.doc, d.page).into_iter().filter(|(_, l)| names.contains(&l.name) && l.kind != fp_model::LayerKind::Web).map(|(_, l)| !l.is_visible()).collect();
    assert!(hidden.iter().filter(|x| **x).count() >= 3, "alle überstrichenen Ebenen ausgeblendet: {hidden:?}");
}

/// F19: „Als Befehl speichern“ fragt nach einem Namen und legt den Befehl im Befehlsordner ab
/// (in Tests: Ordner „Befehle“ neben der Speicher-Vorgabe).
#[test]
fn save_as_command_asks_for_name() {
    let mut h = harness_with(doc_session());
    let dir = st(&h).file_override.as_ref().and_then(|o| o.save.as_ref()).and_then(|p| p.parent()).expect("Vorgabe").join("Befehle");
    let _ = std::fs::remove_file(dir.join("Mein Rechteck.jsf"));
    h.state_mut().state.dialogs.panel_sel.history = [0usize].into_iter().collect();
    h.state_mut().state.run("panel.history.saveAsCommand", json!({}));
    h.run_steps(3);
    let f = st(&h).dialogs.form.as_ref().expect("Namensabfrage statt Dateidialog");
    assert_eq!(f.title, "Befehl speichern");
    assert!(h.ctx.text_edit_focused(), "Fokus im Namensfeld");
    typ(&mut h, "Mein Rechteck");
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(4);
    assert!(dir.join("Mein Rechteck.jsf").exists(), "Befehl im Befehlsordner");
}
/// I01: B-Feld „100150“ → 6000 wie im Original, mit Meldung.
#[test]
fn width_field_limited_to_6000_with_message() {
    let mut s = doc_session();
    s.execute("select.all", json!({})).expect("sel");
    let mut h = harness_with(s);
    let n: Vec<_> = egui_kittest::kittest::Queryable::get_all(&h, egui_kittest::kittest::By::new().role(egui::accesskit::Role::TextInput)).collect();
    // B-Feld des Inspektors: rechts neben der Beschriftung „B:“
    let bl = all_texts(&h).into_iter().find(|(t, _)| t == "B:").map(|(_, r)| r).expect("B:");
    let w = n.iter().filter(|x| (x.rect().center().y - bl.center().y).abs() < 6.0 && x.rect().left() >= bl.right() - 2.0).min_by(|a, b| a.rect().left().total_cmp(&b.rect().left())).expect("B-Feld");
    assert_eq!(w.value().as_deref(), Some("100"));
    w.click();
    drop(n);
    h.run_steps(3);
    key(&mut h, Key::End, Modifiers::NONE);
    typ(&mut h, "150");
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(4);
    let d = st(&h).session.active().expect("d");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, d.selection.elements[0]).expect("e");
    let b = fp_engine::docops::element_bounds(&d.doc, &e).expect("b");
    assert_eq!(b.right - b.left, 6000.0, "auf 6000 begrenzt");
    assert!(st(&h).status.as_ref().is_some_and(|(m, _)| m.contains("6000")), "Meldung statt stillem Klemmen");
}

/// Ursache des Wacklers von [`width_field_limited_to_6000_with_message`] im Gesamtlauf (B-6): der
/// Sprachwechsel eines anderen Test-Threads erhöhte den globalen Zähler `fp_i18n::generation`, die
/// App verwarf daraufhin die Meldung der Statuszeile. Im gemeinsamen Testprogramm gilt die Sprache
/// je Thread, also auch der Zähler.
#[test]
fn language_switch_in_other_thread_keeps_this_threads_generation() {
    let g = fp_i18n::generation();
    std::thread::spawn(|| assert!(fp_i18n::set_language("en"))).join().expect("Thread");
    assert_eq!(fp_i18n::generation(), g, "fremder Sprachwechsel verwirft hier nichts");
    assert!(fp_i18n::set_language("en"));
    assert!(fp_i18n::generation() > g, "eigener Sprachwechsel zählt");
    assert!(fp_i18n::set_language("de"));
}

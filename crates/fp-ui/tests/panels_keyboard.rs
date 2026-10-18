//! Tastatur im Ebenen-Bedienfeld und in Inspektor-Textfeldern mit den Ereignisfolgen, die
//! egui-winit unter Windows liefert (Strg+C/X = `Event::Copy`/`Cut`, Strg+V = `Event::Paste(text)`
//! nur bei Text in der Systemablage, sonst nur das Loslassen von V).
//!
//! Verhalten des Originals (Kürzelsatz-XML und Bedienfeld-Analyse): Die Bedienfelder behalten keinen
//! Tastaturfokus, die Kürzel gehen an das Dokument. Kopieren/Ausschneiden/Einfügen,
//! Löschen (Entf **und** Rücktaste, `cmd_Clear` = 8/46) und Pfeile wirken also auf die im Bedienfeld
//! gewählten Objekte. Ein Klick auf die Ebenenzeile wählt alle Objekte der Ebene (UX-B-Beleg aus
//! den Fensternachrichten des Originals), Strg+C kopiert also deren Objekte. Ohne Auswahl ändern
//! Entf/Rücktaste nichts.
//! Für „Ebene duplizieren“ gibt es kein Kürzel (Bedienfeldmenü bzw. Alt-Klick auf „Neu“), zum
//! Umbenennen kein F2 (nur Doppelklick; F2 blendet das Ebenen-Bedienfeld aus und ein). Im Umbenennen-Feld und in Inspektor-Textfeldern wirken
//! die Kürzel nur auf den Text.

use egui::{Event, Key, Modifiers, Pos2};
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use fp_engine::tools::ToolId;
use fp_ui::app_state::UiSettings;
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

const LAYER: &str = "Ebene Eins";

fn harness() -> H {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 400, "height": 300, "background": "#ffffff"})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80]})).expect("a");
    s.execute("element.setName", json!({"name": "Alpha"})).expect("na");
    s.execute("draw.rectangle", json!({"rect": [200, 150, 300, 220]})).expect("b");
    s.execute("element.setName", json!({"name": "Beta"})).expect("nb");
    let layer = s.active().and_then(|d| d.layer).expect("Ebene");
    s.execute("layer.rename", json!({"layer": layer.0, "name": "Ebene Eins"})).expect("ln");
    s.execute("select.none", json!({})).expect("none");
    s.set_tool(ToolId::Pointer).expect("Zeiger");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.os_clipboard = false;
    app.state.sync_render = true;
    let mut h = Harness::builder().with_size(egui::vec2(1500.0, 950.0)).with_pixels_per_point(1.0).with_max_steps(8).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    // Registerkarte „Ebenen“ der Gruppe nach vorn holen (Klick auf die aktive Karte klappt die
    // Gruppe ein: dann noch einmal)
    for _ in 0..2 {
        let mut all = Vec::new();
        for cs in &h.output().shapes {
            texts(&cs.shape, &mut all);
        }
        if all.iter().any(|(t, _)| t == LAYER) {
            break;
        }
        click_row(&mut h, "Ebenen", false);
        h.run_steps(30);
    }
    h
}

fn ev(h: &mut H, e: Event) {
    h.input_mut().events.push(e);
    h.step();
}

fn key(h: &mut H, k: Key, m: Modifiers) {
    ev(h, Event::Key { key: k, physical_key: None, pressed: true, repeat: false, modifiers: m });
    ev(h, Event::Key { key: k, physical_key: None, pressed: false, repeat: false, modifiers: m });
}

fn release(h: &mut H, k: Key, m: Modifiers) {
    ev(h, Event::Key { key: k, physical_key: None, pressed: false, repeat: false, modifiers: m });
}

fn ctrl_c(h: &mut H) {
    ev(h, Event::Copy);
    release(h, Key::C, Modifiers::CTRL);
    h.step();
}

fn ctrl_x(h: &mut H) {
    ev(h, Event::Cut);
    release(h, Key::X, Modifiers::CTRL);
    h.step();
}

fn ctrl_v(h: &mut H, system_text: Option<&str>) {
    if let Some(t) = system_text {
        ev(h, Event::Paste(t.into()));
    }
    release(h, Key::V, Modifiers::CTRL);
    h.step();
}

fn click(h: &mut H, p: Pos2, double: bool) {
    // kurze Pause wie zwischen zwei Handgriffen (0,5 s); der Doppelklick selbst dauert 4 Bilder (< 0,1 s)
    h.run_steps(30);
    ev(h, Event::PointerMoved(p));
    let press = |down| Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: down, modifiers: Modifiers::NONE };
    ev(h, press(true));
    ev(h, press(false));
    if double {
        ev(h, press(true));
        ev(h, press(false));
    }
    h.run_steps(3);
}

fn texts(shape: &egui::epaint::Shape, out: &mut Vec<(String, egui::Rect)>) {
    match shape {
        egui::epaint::Shape::Text(t) => out.push((t.galley.job.text.clone(), egui::Rect::from_min_size(t.pos, t.galley.size()))),
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
        _ => {}
    }
}

/// Mitte der gezeichneten Beschriftung `label` im Ebenen-Bedienfeld (rechte Bedienfeldspalte).
fn row_of(h: &H, label: &str) -> Pos2 {
    let mut all = Vec::new();
    for cs in &h.output().shapes {
        texts(&cs.shape, &mut all);
    }
    // nur Zeilen der Ebenenliste: Spalte ab dem Ebenennamen (Objektnamen stehen rechts der Miniatur)
    let col = all.iter().find(|(t, _)| t.as_str() == LAYER || t.as_str() == "Vordergrund").map(|(_, r)| r.left());
    all.iter()
        .filter(|(t, r)| t == label && (label == "Ebenen" || col.is_none_or(|x| r.left() >= x - 1.0 && r.left() <= x + 60.0)))
        .map(|(_, r)| r.center())
        .min_by(|a, b| a.x.total_cmp(&b.x)).unwrap_or_else(|| panic!("Zeile „{label}“ nicht gezeichnet: {:?}", all.iter().map(|x| &x.0).collect::<Vec<_>>()))
}

fn click_row(h: &mut H, label: &str, double: bool) {
    let p = row_of(h, label);
    click(h, p, double);
}

fn names(h: &H) -> Vec<String> {
    let d = h.state().state.session.active().expect("d");
    fp_engine::docops::own_layers(&d.doc, d.page).into_iter().flat_map(|(_, l)| l.cels.first().map(|c| c.elements.clone()).unwrap_or_default()).map(|e| e.common.name.clone().unwrap_or_default()).collect()
}

fn sel(h: &H) -> Vec<u64> {
    h.state().state.session.active().map(|d| d.selection.elements.iter().map(|e| e.0).collect()).unwrap_or_default()
}

fn undo_count(h: &H) -> usize {
    h.state().state.session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn doc(h: &H) -> fp_model::Document {
    (*h.state().state.session.active().expect("d").doc).clone()
}

fn left_of(h: &H, name: &str) -> f64 {
    let d = h.state().state.session.active().expect("d");
    let e = fp_engine::docops::own_layers(&d.doc, d.page).into_iter().flat_map(|(_, l)| l.cels.first().map(|c| c.elements.clone()).unwrap_or_default()).find(|e| e.common.name.as_deref() == Some(name)).expect("Element");
    fp_engine::docops::element_bounds(&d.doc, &e).expect("b").left as f64
}

#[test]
fn layers_panel_clipboard_delete_arrows() {
    let mut h = harness();
    // Objektzeile anklicken wählt aus; Strg+C/Strg+V (ohne Paste-Ereignis) fügt genau einmal ein
    click_row(&mut h, "Alpha", false);
    assert_eq!(sel(&h).len(), 1, "Zeile wählt das Objekt");
    let n0 = undo_count(&h);
    ctrl_c(&mut h);
    assert_eq!(h.state().state.session.clipboard.elements.len(), 1, "Strg+C kopiert das gewählte Objekt");
    ctrl_v(&mut h, None);
    assert_eq!(names(&h).len(), 3, "Strg+V fügt ein");
    // Copy and Paste are one history row each (TW-137, RE spec 49 section 1)
    assert_eq!(undo_count(&h), n0 + 2, "Copy row + Paste row");
    // Paste mit Text in der Systemablage: wieder genau einmal
    ctrl_v(&mut h, Some("Text"));
    assert_eq!(names(&h).len(), 4, "kein doppeltes Einfügen");
    // Strg+X schneidet das gewählte Objekt aus
    ctrl_x(&mut h);
    assert_eq!(names(&h).len(), 3, "Strg+X");
    key(&mut h, Key::Z, Modifiers::CTRL);
    key(&mut h, Key::Z, Modifiers::CTRL);
    key(&mut h, Key::Z, Modifiers::CTRL);
    h.step();
    assert_eq!(names(&h), vec!["Beta".to_string(), "Alpha".to_string()], "Rückgängig");
    // Klick auf die Ebenenzeile wählt wie im Original alle Objekte der Ebene (UX-B-Beleg): Strg+C
    // kopiert sie, Strg+V fügt beide ein
    h.state_mut().state.session.clipboard = Default::default();
    click_row(&mut h, LAYER, false);
    assert_eq!(sel(&h).len(), 2, "Ebenenzeile wählt alle Objekte der Ebene");
    ctrl_c(&mut h);
    assert_eq!(h.state().state.session.clipboard.elements.len(), 2, "Strg+C kopiert die Objekte der Ebene");
    ctrl_v(&mut h, None);
    assert_eq!(names(&h).len(), 4);
    key(&mut h, Key::Z, Modifiers::CTRL);
    h.step();
    // ohne Auswahl: Entf/Rücktaste ändern nichts (die Ebene wird nicht gelöscht)
    h.state_mut().state.session.execute("select.none", json!({})).expect("none");
    h.step();
    let base = doc(&h);
    key(&mut h, Key::Delete, Modifiers::NONE);
    key(&mut h, Key::Backspace, Modifiers::NONE);
    h.step();
    assert_eq!(doc(&h), base, "Entf/Rücktaste ohne Objektauswahl ändern nichts");
    // Pfeile verschieben das im Bedienfeld gewählte Objekt; Entf und Rücktaste löschen es
    click_row(&mut h, "Beta", false);
    let l0 = left_of(&h, "Beta");
    key(&mut h, Key::ArrowRight, Modifiers::NONE);
    h.step();
    assert_eq!(left_of(&h, "Beta"), l0 + 1.0, "Pfeil rechts = 1 px");
    key(&mut h, Key::ArrowLeft, Modifiers::SHIFT);
    h.step();
    assert_eq!(left_of(&h, "Beta"), l0 - 9.0, "Umschalt+Pfeil = 10 px");
    key(&mut h, Key::Delete, Modifiers::NONE);
    h.step();
    assert_eq!(names(&h), vec!["Alpha".to_string()], "Entf löscht das gewählte Objekt");
    click_row(&mut h, "Alpha", false);
    key(&mut h, Key::Backspace, Modifiers::NONE);
    h.step();
    assert!(names(&h).is_empty(), "Rücktaste löscht wie im Original (cmd_Clear 8/46)");
}

#[test]
fn layers_panel_rename_keys() {
    let mut h = harness();
    // Doppelklick → Umbenennen; Strg+C/V/X wirken nur auf den Text
    click_row(&mut h, "Alpha", true);
    assert!(h.ctx.text_edit_focused(), "Doppelklick öffnet das Umbenennen-Feld");
    let (n0, clip0) = (undo_count(&h), h.state().state.session.clipboard.elements.len());
    ctrl_c(&mut h);
    ctrl_x(&mut h);
    ctrl_v(&mut h, Some("Neu"));
    ctrl_v(&mut h, None);
    assert_eq!(names(&h).len(), 2, "im Umbenennen-Feld kein Einfügen von Objekten");
    assert_eq!((undo_count(&h), h.state().state.session.clipboard.elements.len()), (n0, clip0), "Dokument und Objektablage unberührt");
    // Pfeile, Entf und Rücktaste bewegen nur die Schreibmarke bzw. löschen Zeichen
    let l0 = left_of(&h, "Alpha");
    key(&mut h, Key::ArrowLeft, Modifiers::NONE);
    key(&mut h, Key::Delete, Modifiers::NONE);
    key(&mut h, Key::Backspace, Modifiers::NONE);
    h.step();
    assert_eq!((names(&h).len(), left_of(&h, "Alpha")), (2, l0), "Tasten wirken im Feld, nicht auf das Objekt");
    // Esc bricht ab: Name unverändert, Auswahl bleibt, kein Verlaufsschritt
    let s0 = sel(&h);
    key(&mut h, Key::Escape, Modifiers::NONE);
    h.run_steps(3);
    assert!(!h.ctx.text_edit_focused());
    assert_eq!(names(&h), vec!["Beta".to_string(), "Alpha".to_string()], "Esc verwirft");
    assert_eq!((sel(&h), undo_count(&h)), (s0.clone(), n0), "Esc wirkt nicht zusätzlich auf die Leinwand");
    // Doppelklick markiert den ganzen Namen (wie die Edit-Zelle des Originals), Tippen ersetzt, Eingabe übernimmt in genau einem Schritt
    click_row(&mut h, "Alpha", true);
    assert!(h.ctx.text_edit_focused());
    ev(&mut h, Event::Text("Gamma".into()));
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(3);
    assert_eq!(names(&h), vec!["Beta".to_string(), "Gamma".to_string()], "Eingabe übernimmt");
    assert_eq!(undo_count(&h), n0 + 1, "genau ein Schritt (Eingabe löst sonst nichts aus)");
    assert_eq!(sel(&h), s0);
    // Ebene umbenennen per Doppelklick
    click_row(&mut h, "Ebene Eins", true);
    assert!(h.ctx.text_edit_focused());
    ev(&mut h, Event::Text("Vordergrund".into()));
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(3);
    let d = h.state().state.session.active().expect("d");
    assert_eq!(fp_engine::docops::own_layers(&d.doc, d.page)[0].1.name, "Vordergrund");
    // F2 benennt nicht um (im Original kein Kürzel)
    click_row(&mut h, "Beta", false);
    let base = doc(&h);
    key(&mut h, Key::F2, Modifiers::NONE);
    h.step();
    assert!(!h.ctx.text_edit_focused(), "F2 öffnet kein Umbenennen");
    assert_eq!(doc(&h), base);
    // F2 ist im Original „Fenster > Ebenen“ (Bedienfeld aus-/einblenden): zweimal = wieder sichtbar
    key(&mut h, Key::F2, Modifiers::NONE);
    h.run_steps(3);
    let mut all = Vec::new();
    for cs in &h.output().shapes {
        texts(&cs.shape, &mut all);
    }
    assert!(all.iter().any(|(t, _)| t == "Vordergrund"), "Ebenen-Bedienfeld nach zweitem F2 wieder sichtbar");
}

#[test]
fn inspector_text_field_clipboard_only_text() {
    let mut h = harness();
    click_row(&mut h, "Alpha", false);
    // Namensfeld des Inspektors fokussieren. Nicht einfach das erste Textfeld: seit BA-033 (PNG 32 als
    // Exportvorgabe) zeigt das Optimieren-Bedienfeld davor ein Zahlenfeld (B-8)
    let fields: Vec<_> = h.get_all(egui_kittest::kittest::By::new().role(egui::accesskit::Role::TextInput)).filter(|n| n.value().as_deref() == Some("Alpha")).collect();
    assert!(!fields.is_empty(), "Inspektor zeigt das Namensfeld");
    fields[0].click();
    drop(fields);
    h.run_steps(3);
    assert!(h.ctx.text_edit_focused(), "Inspektorfeld fokussiert");
    let (base, n0) = (doc(&h), undo_count(&h));
    h.state_mut().state.session.clipboard = Default::default();
    ctrl_c(&mut h);
    ctrl_x(&mut h);
    ctrl_v(&mut h, Some("12"));
    ctrl_v(&mut h, None);
    assert!(h.state().state.session.clipboard.elements.is_empty(), "Strg+C/X im Feld kopiert keine Objekte");
    assert_eq!(names(&h).len(), 2, "Strg+V im Feld fügt keine Objekte ein");
    assert_eq!(doc(&h), base, "Dokument unverändert, solange das Feld nicht übernommen ist");
    assert_eq!(undo_count(&h), n0);
    // Esc verwirft die Eingabe
    key(&mut h, Key::Escape, Modifiers::NONE);
    h.run_steps(3);
    assert_eq!(doc(&h), base, "Esc im Feld verwirft");
    assert!(!h.ctx.text_edit_focused(), "Esc verlässt das Feld");
    assert_eq!(sel(&h).len(), 1, "Esc im Feld hebt die Auswahl nicht zusätzlich auf");
    assert_eq!(undo_count(&h), n0);
}

/// UX-B-Belege: Rechtsklick auf Ebenen-/Objektzeile öffnet das Ebenenmenü 6923; Doppelklick auf ein
/// unbenanntes Objekt öffnet ein leeres Feld (kein Typname vorbelegt).
#[test]
fn layers_panel_context_menu_and_unnamed_rename() {
    let mut h = harness();
    let p = row_of(&h, "Alpha");
    h.run_steps(45);
    ev(&mut h, Event::PointerMoved(p));
    ev(&mut h, Event::PointerButton { pos: p, button: egui::PointerButton::Secondary, pressed: true, modifiers: Modifiers::NONE });
    ev(&mut h, Event::PointerButton { pos: p, button: egui::PointerButton::Secondary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(3);
    let mut all = Vec::new();
    for cs in &h.output().shapes {
        texts(&cs.shape, &mut all);
    }
    assert!(all.iter().any(|(t, _)| t.starts_with("Neue Ebene")), "Kontextmenü 6923 offen: {:?}", all.iter().map(|x| &x.0).collect::<Vec<_>>());
    key(&mut h, Key::Escape, Modifiers::NONE);
    h.run_steps(3);
    // unbenanntes Objekt
    h.state_mut().state.session.execute("draw.rectangle", json!({"rect": [300, 20, 360, 60]})).expect("r");
    let id = sel(&h)[0];
    h.state_mut().state.session.execute("element.setName", json!({"ids": [id], "name": ""})).ok();
    h.state_mut().state.session.execute("select.none", json!({})).expect("none");
    h.run_steps(3);
    click_row(&mut h, "Rechteck", true);
    assert!(h.ctx.text_edit_focused(), "Umbenennen offen");
    let n: Vec<_> = h.get_all(egui_kittest::kittest::By::new().role(egui::accesskit::Role::TextInput)).filter(|n| n.is_focused()).collect();
    assert_eq!(n.first().and_then(|n| n.value()).unwrap_or_default(), "", "unbenanntes Objekt: leeres Feld");
}
/// UX-B E01: Doppelklick kurz (< 0,6 s) nach einem Klick auf eine andere Zeile – egui zählt ihn als
/// Dreifachklick; das Umbenennen muss trotzdem aufgehen (Bildtakt 1/60 s, keine Pause).
#[test]
fn double_click_right_after_other_click_opens_rename() {
    let mut h = harness();
    let (b, a) = (row_of(&h, "Beta"), row_of(&h, "Alpha"));
    let btn = |p: Pos2, down: bool| Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: down, modifiers: Modifiers::NONE };
    ev(&mut h, Event::PointerMoved(b));
    ev(&mut h, btn(b, true));
    ev(&mut h, btn(b, false));
    ev(&mut h, Event::PointerMoved(a));
    ev(&mut h, btn(a, true));
    ev(&mut h, btn(a, false));
    ev(&mut h, btn(a, true));
    ev(&mut h, btn(a, false));
    h.run_steps(3);
    assert!(h.ctx.text_edit_focused(), "Umbenennen offen");
    ev(&mut h, Event::Text("Neu".into()));
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(3);
    assert!(names(&h).contains(&"Neu".to_string()), "ganz markiert, Tippen ersetzt: {:?}", names(&h));
}
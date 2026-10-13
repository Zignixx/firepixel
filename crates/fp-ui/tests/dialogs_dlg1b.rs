//! DLG-1b (Rest P-DLG1, Messwerte PM-5 in PARITAET_B §2): Bildgröße rechnet beim Tippen nach (D19),
//! Leinwandgröße mit Live-Anzeige, Anker per Tastatur und Auswahl aller Objekte nach OK (D20),
//! Zugriffstasten in „Neues Dokument“ (D01), Kategorieliste der Voreinstellungen per Tab (D16) und
//! keine Verlaufsschritte, die das Original nicht anlegt (D18, D19, D11, D13).
//! Ereignisse wie egui-winit unter Windows: je Bild 1/60 s, Taste + Text je Zeichen.

use egui::{Event, Key, Modifiers};
use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::{dialog_rect, last_dialog_id};
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

/// Testdokument wie PM-5: 500 × 400 (72 ppi), R1, R2, T1 „Hallo“, nichts ausgewählt.
fn session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 400, "background": "#ffffff"})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80], "name": "R1"})).expect("R1");
    s.execute("draw.rectangle", json!({"rect": [200, 40, 300, 140], "name": "R2"})).expect("R2");
    s.execute("draw.text", json!({"text": "Hallo", "anchor": [40, 300], "size": 24})).expect("T1");
    s.execute("select.none", json!({})).expect("none");
    s
}

fn harness() -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, session(), UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    // ohne Systemfokus wie die Brücken-Instanz (DLG-1)
    h.input_mut().focused = false;
    h.run_steps(3);
    h
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn key_m(h: &mut H, k: Key, m: Modifiers) {
    if m.any() {
        h.event(Event::ModifiersChanged(m));
    }
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: m });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: m });
    if m.any() {
        ev(h, Event::ModifiersChanged(Modifiers::NONE));
    }
}

fn key(h: &mut H, k: Key) {
    key_m(h, k, Modifiers::NONE);
}

fn alt() -> Modifiers {
    Modifiers { alt: true, ..Default::default() }
}

fn shift() -> Modifiers {
    Modifiers { shift: true, ..Default::default() }
}

/// Tippen wie die Brücke (`type`) bzw. egui-winit: Taste drücken + Text, Taste loslassen.
fn typ(h: &mut H, s: &str) {
    for c in s.chars() {
        let k = Key::from_name(&c.to_string());
        if let Some(k) = k {
            h.event(Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: Modifiers::NONE });
        }
        ev(h, Event::Text(c.to_string()));
        if let Some(k) = k {
            ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: Modifiers::NONE });
        }
    }
}

/// Werte der Textfelder im obersten Dialog (in Zeichenreihenfolge).
fn field_values(h: &H) -> Vec<String> {
    let Some(dr) = last_dialog_id(&h.ctx).and_then(|id| dialog_rect(&h.ctx, &id)) else { return vec![] };
    h.get_all(By::new().role(egui::accesskit::Role::TextInput)).filter(|n| dr.contains(n.rect().center())).map(|n| n.accesskit_node().value().unwrap_or_default().to_string()).collect()
}

fn open(h: &mut H, name: &str) {
    h.state_mut().state.run(name, json!({}));
    h.run_steps(4);
    assert!(st(h).dialogs.any_open(), "{name}: Dialog offen");
}

fn focused(h: &H) -> Option<egui::Id> {
    h.ctx.memory(|m| m.focused())
}

fn undo_count(h: &H) -> usize {
    st(h).session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn page_size(h: &H) -> (f32, f32) {
    let d = st(h).session.active().expect("doc");
    let c = &d.doc.pages[d.page].canvas;
    (c.width, c.height)
}

/// Gezeichnete Beschriftungen (Textprüfung der Felder, `fields::set_audit`).
fn drawn_texts(h: &mut H) -> Vec<String> {
    fp_ui::inspector::fields::set_audit(&h.ctx, true);
    h.run_steps(1);
    let out = fp_ui::inspector::fields::take_audit(&h.ctx).into_iter().map(|a| a.text).collect();
    fp_ui::inspector::fields::set_audit(&h.ctx, false);
    out
}

#[test]
fn image_size_recalculates_while_typing() {
    let mut h = harness();
    for _ in 0..3 {
        open(&mut h, "modify.canvas.imageSize");
        // Startfokus Breite, ganz markiert: Tippen ersetzt; ohne Tab/Fokusverlust nachgeführt
        typ(&mut h, "250");
        let v = field_values(&h);
        assert_eq!(&v[..4], &["250", "200", "3.472", "2.778"], "B 250: Höhe und Druckgröße beim Tippen (Original)");
        key(&mut h, Key::Escape);
        h.run_steps(3);
        assert!(st(&h).dialogs.image_size.is_none());
        assert_eq!(page_size(&h), (500.0, 400.0), "Abbrechen ohne Wirkung");
    }
    open(&mut h, "modify.canvas.imageSize");
    assert_eq!(&field_values(&h)[..4], &["500", "400", "6.944", "5.556"], "Druckgröße mit 3 Stellen und Punkt");
    typ(&mut h, "333");
    // Höhe ungerundet weitergeführt: 266,4 px → Pixelfeld 266, Druckhöhe 3.7 wie im Original
    assert_eq!(&field_values(&h)[..4], &["333", "266", "4.625", "3.7"]);
    let u0 = undo_count(&h);
    key(&mut h, Key::Enter);
    h.run_steps(4);
    assert!(st(&h).dialogs.image_size.is_none());
    assert_eq!(page_size(&h), (333.0, 266.0));
    assert_eq!(undo_count(&h), u0 + 1, "OK = ein Verlaufsschritt");
}

#[test]
fn image_size_ok_without_change_adds_no_step() {
    let mut h = harness();
    let (u0, doc0) = (undo_count(&h), std::sync::Arc::clone(&st(&h).session.active().expect("d").doc));
    for _ in 0..3 {
        open(&mut h, "modify.canvas.imageSize");
        key(&mut h, Key::Enter);
        h.run_steps(4);
        assert!(st(&h).dialogs.image_size.is_none());
    }
    // getippt, aber auf den Ausgangswert zurück: ebenfalls kein Schritt
    open(&mut h, "modify.canvas.imageSize");
    typ(&mut h, "250");
    key(&mut h, Key::Backspace);
    key(&mut h, Key::Backspace);
    key(&mut h, Key::Backspace);
    typ(&mut h, "500");
    key(&mut h, Key::Enter);
    h.run_steps(4);
    assert_eq!(undo_count(&h), u0, "OK ohne Änderung: kein Verlaufsschritt (Original)");
    assert!(std::sync::Arc::ptr_eq(&doc0, &st(&h).session.active().expect("d").doc), "Dokument unverändert");
}

#[test]
fn canvas_size_live_anchor_keys_and_select_all() {
    let mut h = harness();
    open(&mut h, "modify.canvas.canvasSize");
    let texts = drawn_texts(&mut h);
    assert!(texts.iter().any(|t| t == "B: 500") && texts.iter().any(|t| t == "H: 400"), "Live-Anzeige vorhanden: {texts:?}");
    // ↔/↕ sind gezeichnete Symbole (TW-12); „Aktuelle Größe“: Symbol, Zahl, Einheit getrennt
    assert!(texts.iter().any(|t| t == "500") && texts.iter().any(|t| t == "400"), "Aktuelle Größe als Zahl: {texts:?}");
    assert!(!texts.iter().any(|t| t.starts_with("B: 500 ")), "keine Zeile „B: 500 Pixel“ mehr: {texts:?}");
    typ(&mut h, "600");
    assert_eq!(st(&h).dialogs.canvas_size.as_ref().expect("offen").width, 600.0, "Breite beim Tippen übernommen");
    let texts = drawn_texts(&mut h);
    assert!(texts.iter().any(|t| t == "B: 600"), "Live-Anzeige beim Tippen: {texts:?}");
    // Tab: B › H › Einheit › Einheit › Anker (ein Tab-Stopp)
    for _ in 0..4 {
        key(&mut h, Key::Tab);
    }
    assert_eq!(st(&h).dialogs.canvas_size.as_ref().expect("offen").anchor, 4, "Mitte");
    key(&mut h, Key::ArrowLeft);
    key(&mut h, Key::ArrowUp);
    assert_eq!(st(&h).dialogs.canvas_size.as_ref().expect("offen").anchor, 0, "Pfeiltasten wählen im Raster (oben links)");
    key(&mut h, Key::ArrowUp);
    assert_eq!(st(&h).dialogs.canvas_size.as_ref().expect("offen").anchor, 0, "am Rand bleibt der Anker");
    key(&mut h, Key::ArrowRight);
    key(&mut h, Key::ArrowRight);
    key(&mut h, Key::ArrowDown);
    assert_eq!(st(&h).dialogs.canvas_size.as_ref().expect("offen").anchor, 5, "rechts Mitte");
    // genau ein Stopp: nächster Tab verlässt das Raster (Kästchen), Umschalt+Tab kommt zurück
    let anchor5 = egui::Id::new("cs-anchor").with(5usize);
    assert_eq!(focused(&h), Some(anchor5));
    key(&mut h, Key::Tab);
    assert!(focused(&h).is_some_and(|f| (0..9usize).all(|k| f != egui::Id::new("cs-anchor").with(k))), "Raster hat den Fokus nicht mehr");
    key_m(&mut h, Key::Tab, shift());
    assert_eq!(focused(&h), Some(anchor5), "Umschalt+Tab zurück auf den gewählten Anker");
    key(&mut h, Key::ArrowLeft);
    assert_eq!(st(&h).dialogs.canvas_size.as_ref().expect("offen").anchor, 4);
    let radios = h.get_all(By::new().role(egui::accesskit::Role::RadioButton)).count();
    assert!(radios >= 9, "Anker in der Barrierefreiheit: {radios}");
    let u0 = undo_count(&h);
    key(&mut h, Key::Enter);
    h.run_steps(4);
    assert!(st(&h).dialogs.canvas_size.is_none());
    assert_eq!(page_size(&h), (600.0, 400.0));
    let d = st(&h).session.active().expect("doc");
    assert_eq!(d.selection.elements.len(), 3, "nach OK alle Objekte ausgewählt (Original)");
    assert_eq!(undo_count(&h), u0 + 1);
    h.state_mut().state.session.execute("edit.undo", json!({})).expect("undo");
    h.run_steps(2);
    assert_eq!(page_size(&h), (500.0, 400.0));
    assert!(st(&h).session.active().expect("doc").selection.elements.is_empty(), "Rückgängig stellt die leere Auswahl her");
}

#[test]
fn new_document_access_keys() {
    let mut h = harness();
    open(&mut h, "file.new");
    key_m(&mut h, Key::H, alt());
    typ(&mut h, "77");
    key_m(&mut h, Key::B, alt());
    typ(&mut h, "123");
    key_m(&mut h, Key::T, alt());
    {
        let d = st(&h).dialogs.new_document.as_ref().expect("offen");
        assert_eq!((d.width.as_str(), d.height.as_str()), ("123", "77"), "Alt+B/Alt+H wählen das Feld, Inhalt markiert");
        assert_eq!(format!("{:?}", d.canvas), "Transparent", "Alt+T");
    }
    key_m(&mut h, Key::I, alt());
    assert_eq!(format!("{:?}", st(&h).dialogs.new_document.as_ref().expect("offen").canvas), "White", "Alt+I (We&iß)");
    // Optionsgruppe hat jetzt den Fokus: Pfeil wählt weiter
    key(&mut h, Key::ArrowDown);
    assert_eq!(format!("{:?}", st(&h).dialogs.new_document.as_ref().expect("offen").canvas), "Transparent");
    key_m(&mut h, Key::N, alt());
    assert_eq!(format!("{:?}", st(&h).dialogs.new_document.as_ref().expect("offen").canvas), "Custom", "Alt+N (Be&nutzerdefiniert)");
    assert!(st(&h).dialogs.new_document.is_some(), "Zugriffstasten schließen den Dialog nicht und öffnen kein Menü");
    key(&mut h, Key::Escape);
    h.run_steps(3);
    assert!(st(&h).dialogs.new_document.is_none());
}

#[test]
fn preferences_category_list_is_a_tab_stop() {
    let mut h = harness();
    open(&mut h, "edit.preferences");
    let cat = |h: &H| st(h).dialogs.preferences.as_ref().expect("offen").category;
    // Startfokus auf der Liste (Original); Pfeiltasten wählen
    key(&mut h, Key::ArrowDown);
    key(&mut h, Key::ArrowDown);
    assert_eq!(cat(&h), 2, "Liste hat beim Öffnen den Fokus");
    key(&mut h, Key::ArrowUp);
    assert_eq!(cat(&h), 1);
    key(&mut h, Key::End);
    let last = cat(&h);
    assert!(last > 2);
    key(&mut h, Key::Home);
    assert_eq!(cat(&h), 0);
    // Tab verlässt die Liste, Umschalt+Tab kommt zurück
    let item0 = egui::Id::new(("prefcat", 0usize));
    assert_eq!(focused(&h), Some(item0));
    key(&mut h, Key::Tab);
    assert!(focused(&h).is_some_and(|f| f != item0), "Tab verlässt die Liste");
    key_m(&mut h, Key::Tab, shift());
    assert_eq!(focused(&h), Some(item0), "per Umschalt+Tab wieder erreichbar");
    key(&mut h, Key::ArrowDown);
    assert_eq!(cat(&h), 1, "per Tab wieder erreichbar");
    key(&mut h, Key::Escape);
    h.run_steps(3);
    assert!(st(&h).dialogs.preferences.is_none());
}

#[test]
fn grid_export_wizard_html_setup_add_no_history_step() {
    let mut h = harness();
    let u0 = undo_count(&h);
    // Raster bearbeiten: OK ohne Änderung führt nichts aus
    let doc0 = std::sync::Arc::clone(&st(&h).session.active().expect("d").doc);
    open(&mut h, "view.grid.editGrid");
    key(&mut h, Key::Enter);
    h.run_steps(4);
    assert!(!st(&h).dialogs.any_open());
    assert!(std::sync::Arc::ptr_eq(&doc0, &st(&h).session.active().expect("d").doc), "OK ohne Änderung ändert nichts");
    // mit Änderung: wirkt, aber kein Verlaufsschritt (Original: Strg+Z ändert das Raster nicht)
    for w in [20, 25, 20] {
        h.state_mut().state.session.execute("view.grid.editGrid", json!({"width": w, "height": 30})).expect("grid");
    }
    h.run_steps(2);
    let d = st(&h).session.active().expect("d");
    assert_eq!(d.doc.pages[0].root.grid.as_ref().map(|g| (g.size.x, g.size.y)), Some((20.0, 30.0)));
    assert!(d.is_dirty(), "Dokument gilt als geändert");
    assert_eq!(undo_count(&h), u0, "Raster bearbeiten: kein Verlaufsschritt");
    let s = &mut h.state_mut().state.session;
    let out = s.execute("file.exportWizard", json!({"apply": true})).expect("wizard");
    assert_eq!(out.effect, fp_engine::Effect::View);
    s.execute("file.htmlSetup", json!({"settings": {"images_dir": "bilder/"}})).expect("html");
    s.execute("file.htmlSetup", json!({"settings": {"images_dir": "bilder/"}})).expect("html 2");
    let d = s.active().expect("d");
    assert_eq!(d.doc.pages[0].root.export.as_ref().and_then(|e| e.html.as_ref()).and_then(|x| x.images_dir.clone()), Some("bilder/".to_string()));
    assert_eq!(d.history.undo_count(), u0, "Exportassistent und HTML-Einrichtung: kein Verlaufsschritt");
}

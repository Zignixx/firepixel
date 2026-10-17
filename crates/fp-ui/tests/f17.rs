//! F17 (Tweak-Liste 15.10.): Dialoge, Dateien, Inspektor.
//! - TW-52: Esc im Popup-Schieber schließt nur das Popup (Wert verworfen, Auswahl bleibt); ein
//!   Klick neben das Popup auf leere Leinwand übernimmt den Wert und hebt die Auswahl nicht auf;
//!   Pfeil drücken und ziehen übernimmt beim Loslassen. TW-32 bleibt: Esc ohne Popup hebt die
//!   Auswahl auf.

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{By, NodeT, Queryable};
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

fn drag(h: &mut H, a: Pos2, b: Pos2, frames: usize) {
    ev(h, Event::PointerMoved(a));
    press(h, a, true);
    for i in 1..=frames {
        ev(h, Event::PointerMoved(a + (b - a) * (i as f32 / frames as f32)));
    }
    press(h, b, false);
    h.run_steps(2);
}

fn screen(h: &H, x: f32, y: f32) -> Pos2 {
    let s = st(h);
    let d = s.session.active().expect("Dokument");
    let area = s.canvases.get(&d.id).and_then(|c| c.area).expect("Leinwand");
    let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(x, y));
    Pos2::new(area.min.x + sx as f32, area.min.y + sy as f32)
}

fn sel(h: &H) -> usize {
    st(h).session.active().map(|d| d.selection.elements.len()).unwrap_or(0)
}

fn undo_count(h: &H) -> usize {
    st(h).session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn opacity(h: &H, id: u64) -> f64 {
    let d = st(h).session.active().expect("Dokument");
    fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id)).expect("Element").common.opacity.to_percent().round()
}

/// Deckkraftfeld im Inspektor (einziges Feld mit „100“ unten im Fenster).
fn opacity_field(h: &H) -> egui::Rect {
    let v: Vec<egui::Rect> = h
        .get_all(By::new().role(egui::accesskit::Role::TextInput))
        .filter(|n| n.rect().top() > HH - 160.0 && n.value().as_deref() == Some("100"))
        .map(|n| n.rect())
        .collect();
    assert_eq!(v.len(), 1, "ein Feld mit 100: {v:?}");
    v[0]
}

/// Pfeil ▾ rechts neben dem Zahlenfeld.
fn arrow_of(field: egui::Rect) -> Pos2 {
    Pos2::new(field.right() + 3.0 + 1.5 + 6.75, field.center().y)
}

fn sliders(h: &H) -> Vec<egui::Rect> {
    h.query_all(By::new().role(egui::accesskit::Role::Slider)).map(|n| n.rect()).collect()
}

/// R1 rot gewählt, dazu R2.
fn setup() -> (H, u64) {
    let mut r1 = 0;
    let h = harness(|s| {
        let v = s.execute("draw.rectangle", json!({"rect": [20, 20, 140, 90], "fillColor": "#ff0000", "strokeWidth": 0})).expect("R1");
        r1 = v.value["id"].as_u64().expect("id");
        s.execute("draw.rectangle", json!({"rect": [200, 40, 290, 110], "fillColor": "#0000ff", "strokeWidth": 0})).expect("R2");
        s.execute("select.set", json!({"ids": [r1]})).expect("Auswahl");
    });
    (h, r1)
}

/// Popup per Klick öffnen und den Knopf von 100 auf etwa 30 ziehen (wie `drag_popup_slider`).
fn open_and_drag(h: &mut H) -> egui::Rect {
    let f = opacity_field(h);
    click(h, arrow_of(f));
    let s = sliders(h);
    assert_eq!(s.len(), 1, "Popup-Schieber offen");
    let r = s[0];
    let top = r.top() + fp_ui::inspector::fields::VSLIDER_PAD;
    let track = fp_ui::inspector::fields::VSLIDER_TRACK;
    drag(h, Pos2::new(r.center().x, top + 1.0), Pos2::new(r.center().x, top + track * 0.7), 12);
    r
}

#[test]
fn tw52_escape_in_popup_slider_after_drag_keeps_selection() {
    let (mut h, r1) = setup();
    let n0 = undo_count(&h);
    open_and_drag(&mut h);
    assert_eq!(sliders(&h).len(), 1, "nach dem Ziehen bleibt das per Klick geöffnete Popup offen");
    assert_eq!(opacity(&h, r1), 100.0, "noch nicht übernommen");
    key(&mut h, Key::Escape);
    h.run_steps(2);
    assert!(sliders(&h).is_empty(), "Esc schließt das Popup");
    assert_eq!(sel(&h), 1, "Esc im Popup hebt die Auswahl nicht auf");
    assert_eq!(opacity(&h, r1), 100.0, "Esc verwirft den gezogenen Wert");
    assert_eq!(undo_count(&h), n0, "kein Verlaufsschritt");
    // TW-32 bleibt: Esc ohne Popup hebt die Auswahl auf
    key(&mut h, Key::Escape);
    h.run_steps(2);
    assert_eq!(sel(&h), 0, "zweites Esc (ohne Popup) hebt die Auswahl auf");
}

#[test]
fn tw52_click_beside_popup_slider_commits_and_keeps_selection() {
    let (mut h, r1) = setup();
    let n0 = undo_count(&h);
    open_and_drag(&mut h);
    // leere Leinwand (wie `canvas_click [700, 550]` im Lauf: kein Objekt darunter)
    let p = screen(&h, 450.0, 250.0);
    click(&mut h, p);
    assert!(sliders(&h).is_empty(), "Klick daneben schließt das Popup");
    assert_eq!(sel(&h), 1, "der Klick gehört dem Popup, nicht der Leinwand");
    assert_eq!(opacity(&h, r1), 30.0, "Klick daneben übernimmt den gezogenen Wert");
    assert_eq!(undo_count(&h), n0 + 1, "ein Verlaufsschritt");
    // danach wirkt ein Klick auf leere Leinwand wieder normal
    click(&mut h, p);
    assert_eq!(sel(&h), 0, "Klick ins Leere ohne Popup hebt die Auswahl auf");
}

#[test]
fn tw52_press_on_arrow_and_drag_commits_on_release() {
    let (mut h, r1) = setup();
    let n0 = undo_count(&h);
    let a = arrow_of(opacity_field(&h));
    // drücken und ein Stück ziehen öffnet den Schieber; dann zur Mitte der Spur (= 50)
    ev(&mut h, Event::PointerMoved(a));
    press(&mut h, a, true);
    for i in 1..=4 {
        ev(&mut h, Event::PointerMoved(a - egui::vec2(0.0, 3.0 * i as f32)));
    }
    let s = sliders(&h);
    assert_eq!(s.len(), 1, "Drücken und Ziehen am Pfeil öffnet den Schieber");
    let to = Pos2::new(a.x, s[0].top() + fp_ui::inspector::fields::VSLIDER_PAD + fp_ui::inspector::fields::VSLIDER_TRACK * 0.5);
    let from = a - egui::vec2(0.0, 12.0);
    for i in 1..=12 {
        ev(&mut h, Event::PointerMoved(from + (to - from) * (i as f32 / 12.0)));
    }
    assert_eq!(opacity(&h, r1), 100.0, "beim Ziehen noch nicht übernommen");
    press(&mut h, to, false);
    h.run_steps(2);
    assert!(sliders(&h).is_empty(), "Loslassen schließt");
    assert_eq!(opacity(&h, r1), 50.0, "Loslassen übernimmt");
    assert_eq!(undo_count(&h), n0 + 1);
    assert_eq!(sel(&h), 1);
}

// ------------------------------------------------------------------ TW-53: Auswahl animieren

fn frames(h: &H) -> usize {
    st(h).session.active().map(|d| d.frame_count()).unwrap_or(0)
}

/// Auswahl animieren + Enter: Die Animation steht sofort (Instanz, Dokument geändert, ein Schritt,
/// 1 Status), während die Rückfrage „Automatisch neue Status hinzufügen?“ noch offen ist. Esc
/// lässt es dabei, OK hängt die Status an.
#[test]
fn tw53_animate_selection_applies_before_the_question() {
    for ok in [false, true] {
        let (mut h, r1) = setup();
        let n0 = undo_count(&h);
        h.state_mut().state.run("modify.animation.animateSelection", json!({}));
        h.run_steps(3);
        assert!(st(&h).dialogs.form.is_some(), "Dialog „Animieren“ offen");
        key(&mut h, Key::Enter);
        h.run_steps(4);
        assert!(st(&h).dialogs.form.is_none(), "Enter schließt den Dialog");
        assert!(st(&h).dialogs.warning.is_some(), "Rückfrage offen");
        {
            let d = st(&h).session.active().expect("doc");
            let id = d.selection.elements.first().copied().expect("Instanz gewählt");
            assert_ne!(id.0, r1, "R1 ist jetzt eine Instanz");
            let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("Instanz");
            assert!(matches!(e.kind, fp_model::ElementKind::Instance(_)), "Animationsinstanz angelegt");
            assert!(d.is_dirty(), "Dokument geändert");
        }
        assert_eq!(undo_count(&h), n0 + 1, "ein Verlaufsschritt");
        assert_eq!(frames(&h), 1, "Status bleibt 1, solange die Frage offen ist");
        key(&mut h, if ok { Key::Enter } else { Key::Escape });
        h.run_steps(4);
        assert!(st(&h).dialogs.warning.is_none());
        assert_eq!(frames(&h), if ok { 5 } else { 1 }, "OK hängt die Status an, Esc nicht");
        assert_eq!(st(&h).session.active().map(|d| d.doc.symbols.len()), Some(1), "ein Symbol");
    }
}

// ------------------------------------------------------------------ TW-45…49: Wortlaut wie im Original

fn texts(h: &H) -> Vec<String> {
    h.get_all(By::new()).filter_map(|n| n.accesskit_node().label().map(|s| s.to_string()).or_else(|| n.value())).collect()
}

fn buttons(h: &H) -> Vec<String> {
    h.query_all(By::new().role(egui::accesskit::Role::Button)).filter_map(|n| n.accesskit_node().label().map(|s| s.to_string())).collect()
}

/// TW-45: Effekt-Beschriftungen „Aussparung“, „Versatz:“, „Einfarbig“; „Einfarbig“ aus = Farben des
/// Objekts (transparentes Weiß), an = gewählte Farbe.
#[test]
fn tw45_effect_labels_and_solid_checkbox() {
    fp_i18n::with_language("de", || {
        assert_eq!(fp_i18n::t("dlg-fx-knockout"), "Aussparung");
        assert_eq!(fp_i18n::t("dlg-fx-glow-offset"), "Versatz:");
        let (mut h, _) = setup();
        let i = fp_ui::dialogs::effects::FX.iter().position(|f| f.menu_text() == fp_i18n::t("effect-solid-shadow")).expect("Einfarbiger Schatten");
        fp_ui::dialogs::effects::begin_add(&mut h.state_mut().state, i);
        let f = st(&h).dialogs.form.clone().expect("Dialog");
        let solid = f.fields.iter().find(|x| x.key == "solid").expect("Feld „solid“");
        assert_eq!(solid.label, "Einfarbig");
        // TW-62: neuer Effekt wie im Original ohne Farbe, „Einfarbig“ aus (fx-shadow-glow/10)
        assert_eq!(solid.value, json!(false), "Vorgabe: Einfarbig aus");
        assert_eq!(f.params()["params"]["color"], json!("#ffffff00"), "Einfarbig aus = Originalfarben");
        let mut f2 = f.clone();
        f2.field_mut("solid").expect("solid").value = json!(true);
        assert_ne!(f2.params()["params"]["color"], json!("#ffffff00"), "Einfarbig an = gewählte Farbe");
    });
}

/// TW-46: Tastaturkurzbefehle wie im Original.
#[test]
fn tw46_shortcut_dialog_wording() {
    fp_i18n::with_language("de", || {
        assert_eq!(fp_i18n::t("dlg-sc-current-set"), "Aktuelle Auswahl:");
        assert_eq!(fp_i18n::t("dlg-sc-shortcut"), "Kurzbefehle:");
        assert_eq!(fp_i18n::t("dlg-sc-press-key"), "Taste drücken:");
        assert_eq!(fp_i18n::t("dlg-sc-assign"), "Ändern");
    });
}

/// TW-47: Voreinstellungen ▸ Text mit dem Wortlaut des Originals, Zeilenabstand in Prozent.
#[test]
fn tw47_preferences_type_page_like_original() {
    fp_i18n::with_language("de", || {
        let (mut h, _) = setup();
        let mut d = fp_ui::dialogs::PreferencesDialog::new(st(&h));
        d.category = 3;
        h.state_mut().state.dialogs.preferences = Some(d);
        h.run_steps(4);
        let tx = texts(&h);
        for want in ["Größe der Schriftvorschau", "Anzahl der zuletzt verwendeten Schriften", "Änderungen werden erst nach Neustart aktiv"] {
            assert!(tx.iter().any(|s| s == want), "„{want}“ fehlt: {tx:?}");
        }
        // gemalte Beschriftungen links der Felder
        for (k, want) in [("dlg-pref-defaultleading", "Zeilenabstand:"), ("dlg-pref-defaultbaselineshift", "Grundlinienverschiebung:"), ("dlg-pref-defaultmissingfont", "Standardschrift:"), ("dlg-pref-cap-options", "Optionen:"), ("dlg-pref-cap-missingfont", "Vorgehensweise bei fehlenden Schriften:")] {
            assert_eq!(fp_i18n::t(k), want);
        }
        assert!(tx.iter().any(|s| s == "120"), "Zeilenabstand 1,2 als 120 (%): {tx:?}");
        assert!(!tx.iter().any(|s| s.contains("Faktor") || s.contains("(px)")), "keine eigenen Zusätze: {tx:?}");
        assert_eq!(fp_i18n::t("dlg-pref-category"), "Kategorie");
        assert_eq!(fp_i18n::t("dlg-pref-leglookforpng"), "Beim Bearbeiten mit externer Anwendung:");
        // Zusatzmodule: je Ordner ein Knopf „Durchsuchen...“
        h.state_mut().state.dialogs.preferences.as_mut().expect("offen").category = 6;
        h.run_steps(4);
        let n = buttons(&h).iter().filter(|b| *b == "Durchsuchen...").count();
        assert_eq!(n, 3, "drei Knöpfe „Durchsuchen...“: {:?}", buttons(&h));
    });
}

/// TW-48, ab Lauf 10 TW-92 (F24): „Neuer Stil“ mit dem Namen „Stil“ vorbelegt; leer gemacht bleibt
/// es „Stil n“.
#[test]
fn tw48_new_style_name_empty_by_default() {
    fp_i18n::with_language("de", || {
        let (h, _) = setup();
        let mut f = fp_ui::dialogs::misc::new_style_form(st(&h));
        let name = f.fields.iter().find(|x| x.key == "name").expect("Name");
        assert_eq!(name.value, json!("Stil"), "Name vorbelegt (TW-92)");
        f.field_mut("name").expect("Name").value = json!("");
        assert_eq!(fp_ui::dialogs::misc::style_params(&f)["name"], json!("Stil 1"), "leer → „Stil 1“");
        let mut f2 = f.clone();
        f2.field_mut("name").expect("Name").value = json!("Mein Stil");
        assert_eq!(fp_ui::dialogs::misc::style_params(&f2)["name"], json!("Mein Stil"));
        assert_eq!(fp_i18n::t("dlg-style-other"), "Text - Sonsti&ges");
        assert_eq!(fp_i18n::t("dlg-style-cap-props"), "Eigenschaften:");
    });
}

/// TW-49: Exportassistent Seite 1 ohne Aufzählungszeichen und ohne „Zurück“; ab Seite 2 mit.
#[test]
fn tw49_export_wizard_first_page_without_back() {
    fp_i18n::with_language("de", || {
        let (mut h, _) = setup();
        assert!(fp_ui::dialogs::export_wizard::open(&mut h.state_mut().state), "Assistent offen");
        h.run_steps(4);
        let b = buttons(&h);
        assert!(!b.iter().any(|x| x == "Zurück"), "Seite 1 ohne „Zurück“: {b:?}");
        assert!(b.iter().any(|x| x == "Weiter"), "{b:?}");
        let tx = texts(&h);
        assert!(tx.iter().any(|s| s == "Exportformat auswählen."), "Satz ohne Punkt davor: {tx:?}");
        assert!(!tx.iter().any(|s| s.starts_with('•')), "keine Aufzählungszeichen: {tx:?}");
        assert_eq!(fp_i18n::t("dlg-wiz-unit-k"), "k");
        let w = h.state_mut().state.dialogs.wizard.as_mut().expect("Assistent");
        w.advance();
        h.run_steps(4);
        assert!(buttons(&h).iter().any(|x| x == "Zurück"), "ab Seite 2 mit „Zurück“");
    });
}

// ------------------------------------------------------------------ TW-42: Einzeln/Mehrere

fn web_bounds(h: &H) -> Vec<[i32; 4]> {
    let d = st(h).session.active().expect("doc");
    let mut v: Vec<[i32; 4]> = d
        .selection
        .elements
        .iter()
        .filter_map(|id| fp_engine::docops::element_on_page(&d.doc, d.page, *id))
        .filter(|e| matches!(e.kind, fp_model::ElementKind::WebObject(_)))
        .filter_map(|e| fp_engine::docops::element_bounds(&d.doc, &e))
        .map(|b| [b.left.round() as i32, b.top.round() as i32, b.right.round() as i32, b.bottom.round() as i32])
        .collect();
    v.sort();
    v
}

/// Objekt einfügen ▸ Segment/Hotspot mit zwei gewählten Objekten: Rückfrage mit „Einzeln“ und
/// „Mehrere“, ohne „Abbrechen“; Esc = Einzeln (ein Objekt über beide), Mehrere = eines je Objekt.
#[test]
fn tw42_insert_slice_with_two_objects_asks_single_or_multiple() {
    fp_i18n::with_language("de", || {
        for (cmd, answer) in [("edit.insert.slice", Key::Escape), ("edit.insert.slice", Key::Enter), ("edit.insert.hotspot", Key::Escape)] {
            let (mut h, r1) = setup();
            let r2 = {
                let d = st(&h).session.active().expect("doc");
                fp_engine::docops::selectable_ids(&d.doc, d.page, d.frame).into_iter().map(|i| i.0).find(|i| *i != r1).expect("R2")
            };
            h.state_mut().state.session.execute("select.set", json!({"ids": [r1, r2]})).expect("Auswahl");
            h.state_mut().state.run(cmd, json!({}));
            h.run_steps(3);
            let w = st(&h).dialogs.warning.clone().expect("Rückfrage");
            let want = if cmd == "edit.insert.hotspot" { "Es sind mehrere Elemente ausgewählt. Einen Hotspot oder mehrere Hotspots erstellen?" } else { "Es sind mehrere Elemente ausgewählt. Ein Segment oder mehrere Segmente erstellen?" };
            assert_eq!(w.warning.text, want);
            let b = buttons(&h);
            assert!(b.iter().any(|x| x == "Einzeln") && b.iter().any(|x| x == "Mehrere"), "{b:?}");
            assert!(!b.iter().any(|x| x == "Abbrechen"), "kein Abbrechen: {b:?}");
            if answer == Key::Enter {
                // „Mehrere“ anklicken
                let r = h.get_by_label("Mehrere").rect();
                click(&mut h, r.center());
            } else {
                key(&mut h, answer);
            }
            h.run_steps(3);
            assert!(st(&h).dialogs.warning.is_none());
            let got = web_bounds(&h);
            if answer == Key::Enter {
                assert_eq!(got, vec![[20, 20, 140, 90], [200, 40, 290, 110]], "Mehrere: eines je Objekt");
            } else {
                assert_eq!(got, vec![[20, 20, 290, 110]], "{cmd} Esc = Einzeln: eines über beide");
            }
        }
    });
}

/// TW-42 über `run_command`: Parameter `multiple` ohne Rückfrage.
#[test]
fn tw42_multiple_parameter_without_question() {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 300})).expect("Dokument");
    let a = s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 100]})).expect("A").value["id"].as_u64().expect("id");
    let b = s.execute("draw.rectangle", json!({"rect": [160, 20, 220, 80]})).expect("B").value["id"].as_u64().expect("id");
    s.execute("select.set", json!({"ids": [a, b]})).expect("Auswahl");
    let v = s.execute("edit.insert.slice", json!({"multiple": true})).expect("Mehrere");
    assert_eq!(v.value["ids"].as_array().map(|x| x.len()), Some(2));
    s.execute("select.set", json!({"ids": [a, b]})).expect("Auswahl");
    let v = s.execute("edit.insert.hotspot", json!({})).expect("Einzeln");
    assert_eq!(v.value["ids"].as_array().map(|x| x.len()), Some(1), "Vorgabe: Einzeln");
}

// ------------------------------------------------------------------ Q-26: Feld-Kontextmenü

fn right_click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Secondary, pressed: true, modifiers: Modifiers::NONE });
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Secondary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

/// Rechtsklick in ein Inspektorfeld: kleines übersetztes Menü; „Alles auswählen“ markiert den
/// Feldtext, Tippen ersetzt ihn, Eingabe übernimmt; die Objektauswahl bleibt.
#[test]
fn q26_field_context_menu() {
    fp_i18n::with_language("de", || {
        let (mut h, r1) = setup();
        let f = opacity_field(&h);
        right_click(&mut h, f.center());
        let b = buttons(&h);
        for want in ["Rückgängig", "Ausschneiden", "Kopieren", "Einfügen", "Löschen", "Alles auswählen"] {
            assert!(b.iter().any(|x| x == want), "„{want}“ fehlt: {b:?}");
        }
        let r = h.get_by_label("Alles auswählen").rect();
        click(&mut h, r.center());
        assert!(h.query_by_label("Alles auswählen").is_none(), "Menü zu");
        for c in "40".chars() {
            ev(&mut h, Event::Text(c.to_string()));
        }
        key(&mut h, Key::Enter);
        h.run_steps(2);
        assert_eq!(opacity(&h, r1), 40.0, "markierter Text ersetzt");
        assert_eq!(sel(&h), 1, "Auswahl bleibt");
    });
}

// ------------------------------------------------------------------ TW-44 / Q-27: Segment-Inspektor

fn slice_of(h: &H, id: u64) -> fp_model::WebObjectElem {
    let d = st(h).session.active().expect("doc");
    match &fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id)).expect("Segment").kind {
        fp_model::ElementKind::WebObject(w) => (**w).clone(),
        _ => panic!("kein Web-Objekt"),
    }
}

/// Segment mit Objektnamen „S1“: Das Namensfeld zeigt den automatischen Exportnamen, ein Eintrag
/// setzt den Exportnamen (Q-27). Das Exportformat ist bedienbar: Liste der Vorgaben, Wahl setzt die
/// Exporteinstellung des Segments; daneben das Bearbeiten-Symbol (TW-44).
#[test]
fn tw44_q27_slice_inspector_export_name_and_format() {
    fp_i18n::with_language("de", || {
        let mut sid = 0;
        let mut h = harness(|s| {
            let v = s.execute("draw.rectangle", json!({"rect": [20, 20, 140, 90]})).expect("R");
            let r = v.value["id"].as_u64().expect("id");
            s.execute("select.set", json!({"ids": [r]})).expect("Auswahl");
            sid = s.execute("edit.insert.slice", json!({})).expect("Segment").value["id"].as_u64().expect("id");
            s.execute("select.set", json!({"ids": [sid]})).expect("Auswahl");
            s.execute("element.setName", json!({"name": "S1"})).expect("Name");
        });
        h.run_steps(3);
        let vals: Vec<(egui::Rect, String)> = h.get_all(By::new().role(egui::accesskit::Role::TextInput)).filter(|n| n.rect().top() > HH - 160.0).map(|n| (n.rect(), n.value().unwrap_or_default())).collect();
        let (nr, nv) = vals.iter().find(|(_, v)| v.starts_with("_r")).cloned().unwrap_or_else(|| panic!("automatischer Exportname im Namensfeld: {vals:?}"));
        assert!(!vals.iter().any(|(_, v)| v == "S1"), "nicht der Objektname: {vals:?}");
        assert!(nv.contains("_c"), "{nv}");
        // Eintrag setzt den Exportnamen
        click(&mut h, nr.center());
        key(&mut h, Key::End);
        ev(&mut h, Event::Key { key: Key::Home, physical_key: Some(Key::Home), pressed: true, repeat: false, modifiers: Modifiers::SHIFT });
        for c in "kopf".chars() {
            ev(&mut h, Event::Text(c.to_string()));
        }
        key(&mut h, Key::Enter);
        h.run_steps(2);
        assert_eq!(slice_of(&h, sid).file_name.as_deref(), Some("kopf"), "Exportname gesetzt");
        assert_eq!(st(&h).session.active().and_then(|d| fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(sid))).and_then(|e| e.common.name.clone()).as_deref(), Some("S1"), "Objektname bleibt");
        // Exportformat: bedienbar, Liste mit den Vorgaben
        let combo = h.get_all(By::new().role(egui::accesskit::Role::ComboBox)).filter(|n| n.rect().top() > HH - 160.0).find(|n| n.value().as_deref() == Some("PNG32")).map(|n| n.rect()).expect("Kombifeld „PNG32“");
        click(&mut h, combo.center());
        let jpeg = fp_ui::panels::optimize::preset_label(fp_export::options::builtin_presets().iter().find(|p| p.settings.format == fp_export::ExportFormat::Jpeg).expect("JPEG").name);
        let item = h.get_by_label(&jpeg).rect();
        click(&mut h, item.center());
        let e = slice_of(&h, sid).slice.and_then(|s| s.export).expect("Exporteinstellung des Segments");
        assert_eq!(fp_export::OptimizeSettings::from_model(&e, None).format, fp_export::ExportFormat::Jpeg);
        assert!(h.query_by_label("Exportoptionen bearbeiten").is_some(), "Bearbeiten-Symbol");
    });
}

// ------------------------------------------------------------------ TW-43: Vektordatei-Optionen

/// Importieren einer Vektordatei zeigt „Vektordatei-Optionen“ mit den Vorgaben des Originals; OK
/// platziert mit den Optionen, Skalieren 200 % verdoppelt die Größe, eine geänderte Breite
/// bestimmt die Skalierung. Bilddateien kommen ohne den Dialog.
#[test]
fn tw43_vector_file_options_dialog() {
    fp_i18n::with_language("de", || {
        let dir = std::env::temp_dir().join(format!("fp-f17-tw43-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("Ordner");
        let ai = dir.join("probe.ai");
        std::fs::write(&ai, "%!PS-3.0\n%%Creator: Probe\n%%BoundingBox: 0 0 100 50\n%%EndSetup\n0 0 0 1 k\n10 10 m\n90 10 L\n90 40 L\n10 40 L\nf\n").expect("AI");
        let (mut h, _) = setup();
        assert!(fp_ui::dialogs::vector_options::open(&mut h.state_mut().state, &ai), "Dialog für AI");
        let f = st(&h).dialogs.form.clone().expect("Formular");
        assert_eq!(f.title, "Vektordatei-Optionen");
        let v = |k: &str| f.get(k);
        let n = |k: &str| v(k).as_f64();
        assert_eq!((n("scale"), n("width"), n("height"), n("resolution")), (Some(100.0), Some(100.0), Some(50.0), Some(72.0)), "Vorgaben");
        assert_eq!((v("antialiasPaths"), v("antialiasText"), v("antialiasKind"), v("layers"), v("includeInvisible")), (json!(true), json!(true), json!(3), json!(1), json!(false)));
        // Skalieren 200 → doppelte Größe
        let mut f2 = f.clone();
        f2.field_mut("scale").expect("scale").value = json!(200.0);
        let p = fp_ui::dialogs::vector_options::params(&f2);
        assert_eq!(p["vector"]["scale"].as_f64(), Some(200.0));
        h.state_mut().state.run("file.import", p.clone());
        h.run_steps(2);
        assert!(st(&h).place_import.as_ref().is_some_and(|pl| pl.vector.is_some()), "danach Platzierungszeiger mit den Optionen");
        let mut q = p.clone();
        q["at"] = json!([0, 0]);
        let out = h.state_mut().state.session.execute("file.import", q).expect("Import");
        let id = out.value["ids"][0].as_u64().expect("id");
        let d = st(&h).session.active().expect("doc");
        let e = fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id)).expect("Element");
        let b = fp_engine::docops::element_bounds(&d.doc, &e).expect("Grenzen");
        assert_eq!(((b.right - b.left).round(), (b.bottom - b.top).round()), (160.0, 60.0), "200 %: {b:?}");
        // geänderte Breite bestimmt die Skalierung
        let mut f3 = f.clone();
        f3.field_mut("width").expect("width").value = json!(50.0);
        assert_eq!(fp_ui::dialogs::vector_options::params(&f3)["vector"]["scale"].as_f64(), Some(50.0));
        // keine Vektordatei (z. B. Bild): kein Dialog
        let png = dir.join("probe.png");
        std::fs::write(&png, [0x89u8, b'P', b'N', b'G', 13, 10, 26, 10]).expect("PNG");
        assert!(!fp_ui::dialogs::vector_options::open(&mut h.state_mut().state, &png));
        let _ = std::fs::remove_dir_all(&dir);
    });
}

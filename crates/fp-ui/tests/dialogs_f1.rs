//! F1 (TWEAKS 14.10.: TW-14, TW-22, TW-23, TW-29, Q-12) mit realistischen Eingaben: Klick ins Feld,
//! Strg+A, Tippen wie egui-winit (Taste + Text), Enter/Esc als Taste, Knöpfe per Mausklick.
//! - TW-14: getippter Wert + Enter gilt (Numerisch transformieren, Textgröße ▸ Andere, Neues
//!   Dokument, Leinwandgröße mit zwei Feldern)
//! - Q-12: Esc im fokussierten Feld eines Filterdialogs bricht sofort ab, die Vorschau ist weg
//! - TW-22: Assistent „HTML-Code kopieren“ (Stil, Basisname, Bilderordner, Fertig stellen)
//! - TW-23: Popupmenü-Editor mit Abbrechen / < Zurück / Weiter > / Fertig
//! - TW-29: HTML-Einrichtung, „Durchsuchen...“ neben dem Hintergrundbild

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::{dialog_rect, last_dialog_id};
use fp_ui::FirepixelApp;
use serde_json::json;
use std::sync::Arc;

type H = Harness<'static, FirepixelApp>;

fn harness_with(s: fp_engine::Session) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h
}

/// 400 × 300 mit R1 (50,50–150,120) und einem Text (24 px); gibt die IDs zurück.
fn vector_session() -> (fp_engine::Session, u64, u64) {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 400, "height": 300, "background": "#ffffff"})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [50, 50, 150, 120], "name": "R1"})).expect("rect");
    let r1 = s.active().expect("d").selection.elements[0].0;
    s.execute("draw.text", json!({"text": "Hallo", "anchor": [60, 220], "size": 24})).expect("text");
    let tx = s.active().expect("d").selection.elements[0].0;
    (s, r1, tx)
}

/// 300 × 200 mit einer ausgewählten Bitmap B1.
fn bitmap_session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [40, 40, 140, 120], "fillColor": "#3366cc", "name": "B1"})).expect("rect");
    s.execute("modify.flattenSelection", json!({})).expect("reduzieren");
    s
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

fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
}

fn open(h: &mut H, cmd: &str) {
    h.state_mut().state.run(cmd, json!({}));
    h.run_steps(4);
    assert!(st(h).dialogs.any_open(), "{cmd}: Dialog offen");
}

fn dlg_rect(h: &H) -> Rect {
    dialog_rect(&h.ctx, &last_dialog_id(&h.ctx).expect("Dialog")).expect("Rechteck")
}

/// Textfelder des obersten Dialogs von oben nach unten (Rechteck, Text).
fn text_fields(h: &H) -> Vec<(Rect, String)> {
    let dr = dlg_rect(h);
    let mut v: Vec<(Rect, String)> = h.get_all(By::new().role(Role::TextInput)).map(|n| (n.rect(), n.value().unwrap_or_default())).filter(|(r, _)| dr.contains(r.center())).collect();
    v.sort_by(|a, b| a.0.top().total_cmp(&b.0.top()));
    v
}

/// Knopf im obersten Dialog (Beschriftung ohne „&“).
fn button(h: &H, label: &str) -> Option<(Rect, bool)> {
    let dr = dlg_rect(h);
    h.query_all(By::new().role(Role::Button).label(label)).map(|n| (n.rect(), !n.accesskit_node().is_disabled())).find(|(r, _)| dr.contains(r.center()))
}

fn click_button(h: &mut H, label: &str) {
    let (r, on) = button(h, label).unwrap_or_else(|| panic!("Knopf {label}"));
    assert!(on, "Knopf {label} gesperrt");
    click(h, r.center());
    h.run_steps(3);
}

/// Feld `i` anklicken, Inhalt markieren und `text` tippen (ohne Abschluss).
fn type_into(h: &mut H, i: usize, text: &str) {
    let r = text_fields(h)[i].0;
    click(h, r.center());
    key_m(h, Key::A, Modifiers { ctrl: true, command: true, ..Default::default() });
    typ(h, text);
}

fn bounds(h: &H, id: u64) -> [f32; 4] {
    let d = st(h).session.active().expect("doc");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id)).expect("Element");
    let b = fp_engine::docops::element_bounds(&d.doc, &e).expect("Grenzen");
    [b.left, b.top, b.right, b.bottom]
}

fn text_size(h: &H, id: u64) -> f32 {
    let d = st(h).session.active().expect("doc");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id)).expect("Text");
    let fp_model::ElementKind::Text(t) = &e.kind else { panic!("Text") };
    t.resolved_runs().first().and_then(|(a, _)| a.size).unwrap_or(0.0)
}

fn select(h: &mut H, id: u64) {
    h.state_mut().state.session.execute("select.set", json!({"ids": [id]})).expect("Auswahl");
    h.run_steps(2);
}

// ---------------------------------------------------------------------------------------- TW-14

/// Numerisch transformieren: 150 tippen, Enter → R1 auf 150 % (wie das Original 25,32–175,138).
#[test]
fn tw14_numeric_transform_typed_value_and_enter() {
    let (s, r1, _) = vector_session();
    let mut h = harness_with(s);
    select(&mut h, r1);
    open(&mut h, "modify.transform.numericTransform");
    type_into(&mut h, 0, "150");
    key(&mut h, Key::Enter);
    h.run_steps(4);
    assert!(!st(&h).dialogs.any_open(), "Enter schließt");
    assert_eq!(bounds(&h, r1), [25.0, 32.5, 175.0, 137.5], "R1 auf 150 %");
}

/// Textgröße ▸ Andere: 48 tippen, Enter → Text 48.
#[test]
fn tw14_text_size_other_typed_value_and_enter() {
    let (s, _, tx) = vector_session();
    let mut h = harness_with(s);
    select(&mut h, tx);
    open(&mut h, "text.size.other");
    type_into(&mut h, 0, "48");
    key(&mut h, Key::Enter);
    h.run_steps(4);
    assert!(!st(&h).dialogs.any_open(), "Enter schließt");
    assert_eq!(text_size(&h, tx), 48.0);
}

/// Neues Dokument: Breite 123, Klick in Höhe, 77, Enter → 123 × 77.
#[test]
fn tw14_new_document_two_fields_and_enter() {
    let (s, _, _) = vector_session();
    let mut h = harness_with(s);
    open(&mut h, "file.new");
    type_into(&mut h, 0, "123");
    type_into(&mut h, 1, "77");
    key(&mut h, Key::Enter);
    h.run_steps(4);
    assert!(!st(&h).dialogs.any_open(), "Enter schließt");
    let d = st(&h).session.active().expect("neues Dokument");
    let c = &d.doc.pages[d.page].canvas;
    assert_eq!((c.width, c.height), (123.0, 77.0));
}

/// Leinwandgröße: Breite 600, Klick in Höhe, 500, Enter → beide gelten, Objekte mittig (+100/+100).
#[test]
fn tw14_canvas_size_second_field_and_enter() {
    let (s, r1, _) = vector_session();
    let mut h = harness_with(s);
    open(&mut h, "modify.canvas.canvasSize");
    type_into(&mut h, 0, "600");
    type_into(&mut h, 1, "500");
    key(&mut h, Key::Enter);
    h.run_steps(4);
    assert!(!st(&h).dialogs.any_open(), "Enter schließt");
    let d = st(&h).session.active().expect("doc");
    let c = &d.doc.pages[d.page].canvas;
    assert_eq!((c.width, c.height), (600.0, 500.0), "beide Felder übernommen");
    assert_eq!(bounds(&h, r1), [150.0, 150.0, 250.0, 220.0], "Objekte mittig verschoben");
}

// ---------------------------------------------------------------------------------------- Q-12

/// Esc im fokussierten Feld (Vorschau an, Wert getippt): Dialog sofort zu, Dokument wie vorher.
#[test]
fn q12_esc_in_focused_filter_field_cancels_at_once() {
    for cmd in ["filter.blur.gaussian", "filter.sharpen.unsharpMask", "filter.adjustColor.brightnessContrast", "filter.adjustColor.hueSaturation"] {
        let mut h = harness_with(bitmap_session());
        let d0 = Arc::clone(&st(&h).session.active().expect("doc").doc);
        open(&mut h, cmd);
        type_into(&mut h, 0, "8");
        h.run_steps(6);
        assert!(h.ctx.text_edit_focused(), "{cmd}: Feld hat den Fokus");
        key(&mut h, Key::Escape);
        h.run_steps(4);
        assert!(!st(&h).dialogs.any_open(), "{cmd}: ein Esc schließt den Dialog");
        let d = st(&h).session.active().expect("doc");
        assert!(*d.doc == *d0, "{cmd}: Vorschau zurückgenommen, Dokument unverändert");
    }
}

// ---------------------------------------------------------------------------------------- TW-22

fn export_session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 240, "height": 160})).expect("file.new");
    s.execute("draw.rectangle", json!({"rect": [10, 10, 100, 100], "fillColor": "#ff0000"})).expect("rect");
    s.execute("edit.insert.slice", json!({})).expect("slice");
    s
}

/// Assistent: Seite 1 Stil (XHTML), Weiter, Seite 2 Basisname, Weiter, Seite 3 Ordner; Fertig
/// stellen erst auf der letzten Seite; Zurück geht zurück; Ergebnis in Zwischenablage und Ordner.
#[test]
fn tw22_copy_html_wizard_pages_and_finish() {
    let dir = fp_i18n::profile::unique_temp_dir("f1-copy-html");
    let mut h = harness_with(export_session());
    h.state_mut().state.file_override = Some(fp_ui::files::FilePickOverride { open: vec![], save: Some(dir.join("x.htm")) });
    open(&mut h, "edit.copyHtmlCode");
    let w = st(&h).dialogs.copy_html.clone().expect("Assistent offen");
    assert_eq!(w.page, 0);
    assert!(button(&h, "Abbrechen").is_some());
    assert_eq!(button(&h, "< Zurück").map(|b| b.1), Some(false), "Zurück gesperrt auf Seite 1");
    assert_eq!(button(&h, "Fertig stellen").map(|b| b.1), Some(false), "Fertig stellen gesperrt auf Seite 1");
    // Enter = Weiter (Standardknopf)
    key(&mut h, Key::Enter);
    h.run_steps(3);
    assert_eq!(st(&h).dialogs.copy_html.as_ref().map(|w| w.page), Some(1), "Enter blättert weiter");
    type_into(&mut h, 0, "seite");
    key(&mut h, Key::Tab);
    h.run_steps(2);
    assert_eq!(st(&h).dialogs.copy_html.as_ref().map(|w| w.base_name.clone()), Some("seite".into()));
    assert!(button(&h, "HTML-Einrichtung...").is_some(), "Knopf HTML-Einrichtung");
    click_button(&mut h, "< Zurück");
    assert_eq!(st(&h).dialogs.copy_html.as_ref().map(|w| w.page), Some(0));
    click_button(&mut h, "Weiter >");
    click_button(&mut h, "Weiter >");
    assert_eq!(st(&h).dialogs.copy_html.as_ref().map(|w| w.page), Some(2));
    assert_eq!(button(&h, "Weiter >").map(|b| b.1), Some(false), "Weiter gesperrt auf der letzten Seite");
    click_button(&mut h, "Durchsuchen...");
    assert_eq!(st(&h).dialogs.copy_html.as_ref().map(|w| std::path::PathBuf::from(&w.folder)), Some(dir.clone()), "Ordner gewählt");
    click_button(&mut h, "Fertig stellen");
    h.run_steps(3);
    assert!(st(&h).dialogs.copy_html.is_none(), "Assistent zu");
    let html = st(&h).session.clipboard.text.clone().expect("HTML in der Zwischenablage");
    assert!(html.contains("seite"), "Basisname im Code");
    let files: Vec<String> = std::fs::read_dir(&dir).expect("Ordner").flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
    assert!(files.iter().any(|f| f.starts_with("seite")), "Bilder mit Basisname geschrieben: {files:?}");
}

/// Esc und Abbrechen schließen ohne Kopieren; „HTML-Einrichtung...“ öffnet die Einrichtung darüber.
#[test]
fn tw22_cancel_and_html_setup_on_top() {
    let mut h = harness_with(export_session());
    open(&mut h, "edit.copyHtmlCode");
    click_button(&mut h, "Weiter >");
    click_button(&mut h, "HTML-Einrichtung...");
    h.run_steps(3);
    assert!(st(&h).dialogs.html_setup.is_some(), "HTML-Einrichtung offen");
    assert!(st(&h).dialogs.copy_html.is_some(), "Assistent bleibt darunter offen");
    key(&mut h, Key::Escape);
    h.run_steps(3);
    assert!(st(&h).dialogs.html_setup.is_none(), "Esc schließt nur die Einrichtung");
    assert!(st(&h).dialogs.copy_html.is_some());
    key(&mut h, Key::Escape);
    h.run_steps(3);
    assert!(st(&h).dialogs.copy_html.is_none(), "Esc schließt den Assistenten");
    assert!(st(&h).session.clipboard.text.is_none(), "nichts kopiert");
}

/// XHTML-Wahl gilt nur für diesen Export (Dokument unverändert).
#[test]
fn tw22_style_choice_does_not_change_document() {
    let mut s = export_session();
    let d0 = Arc::clone(&s.active().expect("d").doc);
    let o = s.execute("edit.copyHtmlCode", json!({"xhtml": false, "baseName": "abc"})).expect("kopieren");
    let html = o.value["html"].as_str().expect("html").to_string();
    assert!(html.contains("abc"));
    let o2 = s.execute("edit.copyHtmlCode", json!({"xhtml": true})).expect("kopieren");
    assert_ne!(o2.value["html"].as_str(), Some(html.as_str()), "XHTML anders als HTML");
    assert!(*s.active().expect("d").doc == *d0, "Dokument unverändert");
}

// ---------------------------------------------------------------------------------------- TW-23

/// Popupmenü-Editor: Titel, Knöpfe Abbrechen / < Zurück / Weiter > / Fertig; Weiter blättert die
/// Karten, auf der letzten ist Weiter gesperrt; Fertig legt das Menü an.
#[test]
fn tw23_popup_editor_wizard_navigation() {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [10, 10, 100, 60]})).expect("rect");
    s.execute("edit.insert.slice", json!({})).expect("slice");
    let mut h = harness_with(s);
    open(&mut h, "modify.popUpMenu.addPopUpMenu");
    assert!(st(&h).dialogs.popup_editor.is_some(), "Editor offen");
    assert!(h.query_all(By::new().label_contains("Popupmenü-Editor")).next().is_some(), "Titel „Popupmenü-Editor“");
    assert!(button(&h, "OK").is_none(), "kein OK mehr");
    for l in ["Abbrechen", "< Zurück", "Weiter >", "Fertig"] {
        assert!(button(&h, l).is_some(), "Knopf {l}");
    }
    assert_eq!(button(&h, "< Zurück").map(|b| b.1), Some(false));
    for k in 1..4 {
        click_button(&mut h, "Weiter >");
        assert_eq!(st(&h).dialogs.popup_editor.as_ref().map(|d| d.tab), Some(k));
    }
    assert_eq!(button(&h, "Weiter >").map(|b| b.1), Some(false), "letzte Karte");
    click_button(&mut h, "< Zurück");
    assert_eq!(st(&h).dialogs.popup_editor.as_ref().map(|d| d.tab), Some(2));
    let n0 = st(&h).session.active().map(|d| d.history.undo_count()).unwrap_or(0);
    click_button(&mut h, "Fertig");
    assert!(st(&h).dialogs.popup_editor.is_none(), "Fertig schließt");
    assert_eq!(st(&h).session.active().map(|d| d.history.undo_count()), Some(n0 + 1), "Menü angelegt");
}

// ---------------------------------------------------------------------------------------- TW-29

/// HTML-Einrichtung: „Durchsuchen...“ rechts neben dem Hintergrundbild trägt die Datei ein
/// (relativ zum Dokumentordner, sonst voller Pfad mit „/“).
#[test]
fn tw29_html_setup_browse_background() {
    let img = fp_i18n::profile::unique_temp_dir("f1-bg").join("hg.png");
    let mut h = harness_with(export_session());
    h.state_mut().state.file_override = Some(fp_ui::files::FilePickOverride { open: vec![img.clone()], save: None });
    open(&mut h, "file.htmlSetup");
    let field = text_fields(&h).into_iter().find(|(r, _)| r.width() > 150.0).expect("Feld Hintergrundbild").0;
    let (b, on) = button(&h, "Durchsuchen...").expect("Knopf Durchsuchen");
    assert!(on && b.left() > field.right() && (b.center().y - field.center().y).abs() < 3.0, "rechts neben dem Feld: {b:?} {field:?}");
    click_button(&mut h, "Durchsuchen...");
    let want = img.to_string_lossy().replace('\\', "/");
    assert_eq!(st(&h).dialogs.html_setup.as_ref().map(|d| d.background_url.clone()), Some(want));
    assert_eq!(fp_ui::dialogs::html_setup::background_url_for(std::path::Path::new("C:/web/img/a.png"), Some(std::path::Path::new("C:/web"))), "img/a.png");
}

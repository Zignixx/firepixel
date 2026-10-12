//! Offscreen-Tests der Leinwand mit egui_kittest: echte Maus-/Tastaturereignisse über das ganze
//! Hauptfenster, Prüfung von Dokument, Werkzeug-Rückmeldung (Ghost, Text-Editor, Ameisen,
//! Beschriftungen), Hilfslinien und Kontextmenü.

use crate::app_state::UiSettings;
use crate::theme::Theme;
use crate::FirepixelApp;
use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_engine::{Overlay, ToolId};
use fp_model::ElementKind;
use serde_json::json;

fn harness() -> Harness<'static, FirepixelApp> {
    let mut session = fp_engine::Session::default();
    let _ = session.execute("file.new", json!({"width": 400, "height": 300, "background": "#ffffff"}));
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, session, UiSettings::default(), false);
    // Tests fassen die Systemzwischenablage nie an
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1500.0, 950.0)).with_pixels_per_point(1.0).with_max_steps(8).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

/// Dokumentpunkt → Bildschirmpunkt der Leinwand.
fn at(h: &Harness<'static, FirepixelApp>, x: f32, y: f32) -> Pos2 {
    let st = &h.state().state;
    let d = st.session.active().expect("Dokument");
    let area = st.canvases.get(&d.id).and_then(|c| c.area).expect("Leinwand");
    super::to_screen(&d.view, area.min, x as f64, y as f64)
}

fn ev(h: &mut Harness<'static, FirepixelApp>, e: Event) {
    h.input_mut().events.push(e);
    h.step();
}

fn mv(h: &mut Harness<'static, FirepixelApp>, p: Pos2) {
    ev(h, Event::PointerMoved(p));
}

fn button(h: &mut Harness<'static, FirepixelApp>, p: Pos2, b: PointerButton, pressed: bool, m: Modifiers) {
    ev(h, Event::PointerButton { pos: p, button: b, pressed, modifiers: m });
}

fn mouse_drag(h: &mut Harness<'static, FirepixelApp>, a: Pos2, b: Pos2, m: Modifiers) {
    mv(h, a);
    button(h, a, PointerButton::Primary, true, m);
    mv(h, a + (b - a) * 0.5);
    mv(h, b);
    button(h, b, PointerButton::Primary, false, m);
    h.step();
}

fn key(h: &mut Harness<'static, FirepixelApp>, k: Key, m: Modifiers) {
    ev(h, Event::Key { key: k, physical_key: None, pressed: true, repeat: false, modifiers: m });
    ev(h, Event::Key { key: k, physical_key: None, pressed: false, repeat: false, modifiers: m });
}

fn top(h: &Harness<'static, FirepixelApp>) -> Vec<std::sync::Arc<fp_model::Element>> {
    let d = h.state().state.session.active().expect("Dokument");
    fp_engine::docops::own_layers(&d.doc, d.page).into_iter().rev().flat_map(|(_, l)| l.cels.first().map(|c| c.elements.clone()).unwrap_or_default()).collect()
}

fn set_tool(h: &mut Harness<'static, FirepixelApp>, t: ToolId) {
    let _ = h.state_mut().state.session.set_tool(t);
    h.step();
}

#[test]
fn rectangle_drag_shows_size_label_and_creates_shape() {
    let mut h = harness();
    set_tool(&mut h, ToolId::Rectangle);
    let (a, b) = (at(&h, 50.0, 40.0), at(&h, 150.0, 100.0));
    mv(&mut h, a);
    button(&mut h, a, PointerButton::Primary, true, Modifiers::NONE);
    mv(&mut h, b);
    // Rückmeldung: Pfad-Vorschau und „B × H“ in Beschriftung und Statuszeile
    let st = &h.state().state;
    let cc = st.canvases.values().next().expect("Leinwand");
    assert!(cc.feedback.iter().any(|o| matches!(o, Overlay::Label { text, .. } if text == "100 × 60")), "{:?}", cc.feedback);
    assert_eq!(cc.info.as_deref(), Some("100 × 60"));
    button(&mut h, b, PointerButton::Primary, false, Modifiers::NONE);
    h.step();
    let t = top(&h);
    assert_eq!(t.len(), 1);
    let bnd = fp_engine::docops::element_bounds(&h.state().state.session.active().expect("d").doc, &t[0]).expect("Grenzen");
    assert_eq!((bnd.left, bnd.top, bnd.right, bnd.bottom), (50.0, 40.0, 150.0, 100.0));
    // eigener Mauszeiger: Werkzeug meldet Fadenkreuz
    assert_eq!(h.state().state.tool_cursor, fp_engine::tools::Cursor::Crosshair);
}

#[test]
fn moving_hides_original_and_shows_ghost() {
    let mut h = harness();
    let _ = h.state_mut().state.session.execute("draw.rectangle", json!({"rect": [50, 50, 150, 120]}));
    set_tool(&mut h, ToolId::Pointer);
    let (a, b) = (at(&h, 100.0, 80.0), at(&h, 160.0, 90.0));
    mv(&mut h, a);
    button(&mut h, a, PointerButton::Primary, true, Modifiers::NONE);
    mv(&mut h, a + egui::vec2(10.0, 0.0));
    mv(&mut h, b);
    {
        let cc = h.state().state.canvases.values().next().expect("Leinwand");
        // der Auswahlrahmen wandert mit (Zeiger 60/10 Dokumentpixel nach rechts/unten)
        let f = cc.sel_frames.first().expect("Auswahlrahmen");
        assert_eq!((f[0].x, f[0].y), (110.0, 60.0), "{f:?}");
        assert!(cc.feedback.iter().any(|o| matches!(o, Overlay::Ghost { .. })));
        assert!(cc.shown.is_some_and(|k| k.variant != 0), "Original im Hintergrundbild ausgeblendet");
        assert!(cc.ghosts.drag.is_some(), "gerenderte Vorschau");
    }
    button(&mut h, b, PointerButton::Primary, false, Modifiers::NONE);
    h.run_steps(2);
    let cc = h.state().state.canvases.values().next().expect("Leinwand");
    assert!(cc.shown.is_some_and(|k| k.variant == 0), "nach dem Loslassen wieder vollständig");
    let f = cc.sel_frames.first().expect("Auswahlrahmen");
    assert_eq!((f[0].x, f[0].y), (110.0, 60.0), "Rahmen steht am neuen Ort");
    let d = h.state().state.session.active().expect("d");
    let bnd = fp_engine::docops::element_bounds(&d.doc, &top(&h)[0]).expect("Grenzen");
    assert_eq!((bnd.left, bnd.top), (110.0, 60.0));
}

#[test]
fn shared_layer_element_moves_with_mouse() {
    let mut h = harness();
    {
        let s = &mut h.state_mut().state.session;
        let _ = s.execute("draw.rectangle", json!({"rect": [50, 50, 150, 120]}));
        let _ = s.execute("page.add", json!({}));
        let _ = s.execute("page.setCurrent", json!({"index": 0}));
        s.execute("panel.layers.shareToAllPages", json!({})).expect("teilen");
        let _ = s.execute("select.none", json!({}));
    }
    set_tool(&mut h, ToolId::Pointer);
    h.run_steps(2);
    let (a, b) = (at(&h, 100.0, 80.0), at(&h, 130.0, 100.0));
    mv(&mut h, a);
    button(&mut h, a, PointerButton::Primary, true, Modifiers::NONE);
    mv(&mut h, a + egui::vec2(10.0, 5.0));
    mv(&mut h, b);
    {
        let cc = h.state().state.canvases.values().next().expect("Leinwand");
        assert!(cc.shown.is_some_and(|k| k.variant != 0), "Original der geteilten Ebene ausgeblendet");
        assert!(cc.ghosts.drag.is_some(), "Vorschau gerendert");
    }
    button(&mut h, b, PointerButton::Primary, false, Modifiers::NONE);
    h.run_steps(2);
    let d = h.state().state.session.active().expect("d");
    let id = d.selection.elements.first().copied().expect("Auswahl");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("Element");
    let bnd = fp_engine::docops::element_bounds(&d.doc, &e).expect("Grenzen");
    assert_eq!((bnd.left, bnd.top), (80.0, 70.0));
}

#[test]
fn swatch_feedback_highlights_palette_pixels() {
    let mut h = harness();
    {
        let s = &mut h.state_mut().state.session;
        let img = fp_raster::RgbaImage::filled(fp_geom::IRect::from_xywh(10, 10, 20, 10), fp_color::Rgba8::new(255, 0, 0, 255)).expect("Bild");
        s.clipboard.clear();
        s.clipboard.pixels = Some(img);
        let _ = s.execute("edit.paste", json!({}));
        let _ = s.execute("select.none", json!({}));
    }
    h.step();
    let count = |h: &Harness<'static, FirepixelApp>| h.state().state.canvases.values().next().and_then(|c| c.swatch.as_ref().map(|s| s.count));
    assert_eq!(count(&h), None, "aus ohne Schalter");
    h.state_mut().state.session.prefs.set("Firepixel_OptimizeSwatchFeedback", fp_engine::PrefValue::Bool(true));
    h.state_mut().state.dialogs.panel_sel.palette = vec![0xffff_0000];
    h.step();
    assert_eq!(count(&h), Some(200), "die 20×10 roten Pixel");
    // Auswahl wechseln: Rückmeldung folgt; Schalter aus: verschwindet
    h.state_mut().state.dialogs.panel_sel.palette = vec![0xffff_ffff];
    h.step();
    assert_eq!(count(&h), Some(400 * 300 - 200), "weiße Leinwand ohne das Rot");
    h.state_mut().state.session.prefs.set("Firepixel_OptimizeSwatchFeedback", fp_engine::PrefValue::Bool(false));
    h.step();
    assert_eq!(count(&h), None);
}

fn release(h: &mut Harness<'static, FirepixelApp>, k: Key, m: Modifiers) {
    ev(h, Event::Key { key: k, physical_key: None, pressed: false, repeat: false, modifiers: m });
}

/// Strg+C/X/V so, wie egui-winit sie unter Windows liefert: Drücken wird zu `Event::Copy`/`Cut`
/// bzw. `Paste(text)` (nur wenn die Systemablage Text hat!), das Loslassen kommt als Taste.
fn ctrl_c(h: &mut Harness<'static, FirepixelApp>) {
    ev(h, Event::Copy);
    release(h, Key::C, Modifiers::CTRL);
}
fn ctrl_x(h: &mut Harness<'static, FirepixelApp>) {
    ev(h, Event::Cut);
    release(h, Key::X, Modifiers::CTRL);
}
fn ctrl_v(h: &mut Harness<'static, FirepixelApp>, system_text: Option<&str>) {
    if let Some(t) = system_text {
        ev(h, Event::Paste(t.into()));
    }
    release(h, Key::V, Modifiers::CTRL);
    h.step();
}

fn undo_count(h: &Harness<'static, FirepixelApp>) -> usize {
    h.state().state.session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn doc_clone(h: &Harness<'static, FirepixelApp>) -> fp_model::Document {
    (*h.state().state.session.active().expect("d").doc).clone()
}

/// Kürzel mit realistischen Ereignisfolgen auf der Leinwand: Kopieren/Ausschneiden/Einfügen
/// (mit und ohne Text in der Systemablage), Duplizieren, Gruppieren, Alles auswählen, Rückgängig,
/// Wiederholen, Entf, Pfeile – jeweils Ergebnis am Dokument, Verlauf +1, Rückgängig exakt.
#[test]
fn clipboard_and_menu_shortcuts_with_real_event_sequences() {
    let mut h = harness();
    let _ = h.state_mut().state.session.execute("draw.rectangle", json!({"rect": [50, 50, 150, 120]}));
    set_tool(&mut h, ToolId::Pointer);
    let base = doc_clone(&h);
    let n0 = undo_count(&h);
    // Strg+C, dann Strg+V (Systemablage enthält nur unser Bild → kein Paste-Ereignis)
    ctrl_c(&mut h);
    h.step();
    assert!(!h.state().state.session.clipboard.elements.is_empty(), "Strg+C kopiert das Objekt");
    ctrl_v(&mut h, None);
    assert_eq!(top(&h).len(), 2, "Strg+V fügt ein");
    assert_eq!(undo_count(&h), n0 + 1, "ein Verlaufsschritt");
    let d = h.state().state.session.active().expect("d");
    let b0 = fp_engine::docops::element_bounds(&d.doc, &top(&h)[0]).expect("b");
    assert_eq!((b0.left, b0.top), (50.0, 50.0), "an gleicher Stelle eingefügt wie im Original");
    // Strg+V mit Text in der Systemablage: genau EIN Einfügen (Paste-Ereignis + Loslassen)
    ctrl_v(&mut h, Some("irgendein Text"));
    assert_eq!(top(&h).len(), 3, "kein doppeltes Einfügen");
    // Rückgängig über Strg+Z (Tastendruck kommt als Key) zweimal = Ausgangszustand
    key(&mut h, Key::Z, Modifiers::CTRL);
    key(&mut h, Key::Z, Modifiers::CTRL);
    h.step();
    assert_eq!(doc_clone(&h), base, "Rückgängig exakt");
    key(&mut h, Key::Y, Modifiers::CTRL);
    h.step();
    assert_eq!(top(&h).len(), 2, "Strg+Y wiederholt");
    // Strg+X schneidet aus, Strg+V fügt zurück
    let _ = h.state_mut().state.session.execute("select.all", json!({}));
    ctrl_x(&mut h);
    h.step();
    assert_eq!(top(&h).len(), 0, "Strg+X");
    ctrl_v(&mut h, None);
    assert_eq!(top(&h).len(), 2);
    // Strg+A, Strg+G, Umschalt+Strg+G, Strg+D
    key(&mut h, Key::A, Modifiers::CTRL);
    h.step();
    assert_eq!(h.state().state.session.active().map(|d| d.selection.elements.len()), Some(2));
    key(&mut h, Key::G, Modifiers::CTRL);
    h.step();
    assert_eq!(top(&h).len(), 1, "Strg+G gruppiert");
    key(&mut h, Key::G, Modifiers::CTRL | Modifiers::SHIFT);
    h.step();
    assert_eq!(top(&h).len(), 2, "Umschalt+Strg+G löst");
    let _ = h.state_mut().state.session.execute("select.none", json!({}));
    let first = top(&h)[0].id;
    let _ = h.state_mut().state.session.execute("select.set", json!({"ids": [first.0]}));
    // Satz „Firepixel“ wie im Original: Strg+Alt+D dupliziert, Strg+D hebt die Auswahl auf
    key(&mut h, Key::D, Modifiers::CTRL | Modifiers::ALT);
    h.step();
    assert_eq!(top(&h).len(), 3, "Strg+Alt+D dupliziert");
    let dup = h.state().state.session.active().and_then(|d| d.selection.elements.first().copied()).expect("Duplikat ausgewählt");
    key(&mut h, Key::D, Modifiers::CTRL);
    h.step();
    assert!(h.state().state.session.active().is_some_and(|d| d.selection.elements.is_empty()), "Strg+D = Auswahl aufheben");
    let _ = h.state_mut().state.session.execute("select.set", json!({"ids": [dup.0]}));
    // Umschalt+Pfeil verschiebt um 10, Entf löscht
    let id = h.state().state.session.active().and_then(|d| d.selection.elements.first().copied()).expect("Auswahl");
    let bx = |h: &Harness<'static, FirepixelApp>| {
        let d = h.state().state.session.active().expect("d");
        fp_engine::docops::element_on_page(&d.doc, d.page, id).and_then(|e| fp_engine::docops::element_bounds(&d.doc, &e)).map(|b| b.left)
    };
    let l0 = bx(&h);
    key(&mut h, Key::ArrowRight, Modifiers::SHIFT);
    h.step();
    assert_eq!(bx(&h), l0.map(|v| v + 10.0));
    key(&mut h, Key::Delete, Modifiers::NONE);
    h.step();
    assert_eq!(top(&h).len(), 2, "Entf löscht");
    key(&mut h, Key::Backspace, Modifiers::NONE);
    h.step();
    assert_eq!(top(&h).len(), 2, "Rücktaste ohne Auswahl tut nichts");
}

/// Fokuszustände: Text-Editor auf der Leinwand (Strg+C/V betreffen nur den Text), offener Dialog
/// (Kürzel wirken nicht auf das Dokument), AltGr im Text-Editor ist Texteingabe.
#[test]
fn shortcuts_respect_focus_text_editor_and_dialogs() {
    let mut h = harness();
    let _ = h.state_mut().state.session.execute("draw.rectangle", json!({"rect": [200, 200, 260, 260]}));
    let base_count = top(&h).len();
    set_tool(&mut h, ToolId::Text);
    let p = at(&h, 40.0, 60.0);
    mv(&mut h, p);
    button(&mut h, p, PointerButton::Primary, true, Modifiers::NONE);
    button(&mut h, p, PointerButton::Primary, false, Modifiers::NONE);
    ev(&mut h, Event::Text("abc".into()));
    key(&mut h, Key::A, Modifiers::CTRL);
    ctrl_c(&mut h);
    let copied = h.output().platform_output.commands.iter().any(|c| matches!(c, egui::OutputCommand::CopyText(t) if t == "abc"));
    let _ = copied;
    assert!(h.state().state.session.clipboard.elements.is_empty(), "Strg+C im Text kopiert keine Objekte");
    ctrl_v(&mut h, Some("XY"));
    // AltGr+Q (Strg+Alt) liefert '@' als Text, kein Kürzel
    ev(&mut h, Event::Key { key: Key::Q, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::CTRL | Modifiers::ALT });
    ev(&mut h, Event::Text("@".into()));
    key(&mut h, Key::Escape, Modifiers::NONE);
    h.step();
    assert_eq!(top(&h).len(), base_count + 1);
    assert!(matches!(&top(&h)[0].kind, ElementKind::Text(x) if x.plain_text() == "XY@"), "{:?}", top(&h)[0].kind);
    // Dialog offen: Strg+C/V/D wirken nicht auf das Dokument
    let _ = crate::dialogs::open_for_command(&mut h.state_mut().state, "file.export");
    h.step();
    assert!(h.state().state.dialogs.any_open());
    let before = doc_clone(&h);
    ctrl_c(&mut h);
    ctrl_v(&mut h, None);
    key(&mut h, Key::D, Modifiers::CTRL);
    h.step();
    assert_eq!(doc_clone(&h), before, "Dialog offen: Dokument unverändert");
}

/// Vorschau beim Ziehen über den Leinwandrand: außerhalb bleibt der Arbeitsbereich grau (wie im
/// Original und im fertigen Bild), innerhalb ist das Objekt zu sehen (offscreen mit wgpu).
#[test]
fn drag_preview_is_clipped_to_the_canvas() {
    let mut session = fp_engine::Session::default();
    let _ = session.execute("file.new", json!({"width": 400, "height": 300, "background": "#ffffff"}));
    let img = fp_raster::RgbaImage::filled(fp_geom::IRect::from_xywh(300, 100, 80, 60), fp_color::Rgba8::new(255, 0, 0, 255)).expect("Bild");
    session.clipboard.clear();
    session.clipboard.pixels = Some(img);
    let _ = session.execute("edit.paste", json!({}));
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, session, UiSettings::default(), false);
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1500.0, 950.0)).with_pixels_per_point(1.0).with_max_steps(8).wgpu().build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    set_tool(&mut h, ToolId::Pointer);
    let (a, b) = (at(&h, 340.0, 130.0), at(&h, 400.0, 130.0));
    mv(&mut h, a);
    button(&mut h, a, PointerButton::Primary, true, Modifiers::NONE);
    mv(&mut h, a + egui::vec2(10.0, 0.0));
    mv(&mut h, b);
    h.step();
    let shot = match h.render() {
        Ok(i) => i,
        Err(e) => {
            eprintln!("kein Grafikadapter, Bildprüfung übersprungen: {e}");
            return;
        }
    };
    let bg = Theme::current(&h.ctx).colors.canvas_bg;
    let px = |p: Pos2| shot.get_pixel(p.x as u32, p.y as u32).0;
    let outside = px(at(&h, 425.0, 112.0));
    let inside = px(at(&h, 385.0, 112.0));
    assert_eq!((outside[0], outside[1], outside[2]), (bg.r(), bg.g(), bg.b()), "außerhalb der Leinwand Arbeitsbereich");
    assert_eq!((inside[0], inside[1], inside[2]), (255, 0, 0), "innerhalb das gezogene Objekt");
    button(&mut h, b, PointerButton::Primary, false, Modifiers::NONE);
    h.run_steps(3);
    if let Ok(after) = h.render() {
        let o = after.get_pixel(at(&h, 425.0, 112.0).x as u32, at(&h, 425.0, 112.0).y as u32).0;
        assert_eq!((o[0], o[1], o[2]), (bg.r(), bg.g(), bg.b()), "nach dem Loslassen identisch");
    }
}

/// Ereignis wie in HGs `gesamt_alltag.rs`: Modifikatorzustand über `ModifiersChanged`, ein Bild je
/// Ereignis.
fn hg_ev(h: &mut Harness<'static, FirepixelApp>, e: Event) {
    let m = match &e {
        Event::Key { modifiers, .. } | Event::PointerButton { modifiers, .. } | Event::MouseWheel { modifiers, .. } => Some(*modifiers),
        _ => None,
    };
    if let Some(m) = m {
        h.event(Event::ModifiersChanged(m));
    }
    h.event(e);
    h.run_steps(1);
}

fn hg_click(h: &mut Harness<'static, FirepixelApp>, p: Pos2) {
    hg_ev(h, Event::PointerMoved(p));
    hg_ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    hg_ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    hg_ev(h, Event::ModifiersChanged(Modifiers::NONE));
}

/// UX-A C15–C19: Strg+Umschalt/Alt+V/X/C kommen von egui-winit nur als Paste/Cut/Copy bzw. als
/// Loslassen von V an – sie müssen trotzdem die Kürzel des Satzes auslösen (Innen einfügen, Als
/// Maske einfügen …), nicht das einfache Einfügen. Umschalt+Einfg (ohne Strg) bleibt Einfügen.
#[test]
fn clipboard_shortcuts_with_shift_alt_use_their_commands() {
    let mut h = harness();
    let _ = h.state_mut().state.session.execute("draw.rectangle", json!({"rect": [50, 50, 150, 120]}));
    let _ = h.state_mut().state.session.execute("edit.copy", json!({}));
    let _ = h.state_mut().state.session.execute("draw.ellipse", json!({"rect": [60, 60, 140, 110]}));
    let st = &h.state().state;
    let cs = |m: Modifiers| fp_engine::Shortcut { ctrl: true, alt: m.alt, shift: m.shift, vk: 0x56 };
    for m in [Modifiers::CTRL | Modifiers::SHIFT, Modifiers::CTRL | Modifiers::ALT] {
        let expected = crate::input::command_for_key(st, Key::V, &cs(m));
        let Some(cmd) = expected else { continue };
        assert_ne!(cmd, "edit.paste");
    }
    // Strg+Umschalt+V ohne Text in der Systemablage: nur das Loslassen kommt an
    let expected = crate::input::command_for_key(&h.state().state, Key::V, &cs(Modifiers::CTRL | Modifiers::SHIFT));
    let label = expected.as_ref().and_then(|c| h.state().state.session.registry().get(c).map(|x| x.info().label.clone()));
    hg_ev(&mut h, Event::Key { key: Key::V, physical_key: Some(Key::V), pressed: false, repeat: false, modifiers: Modifiers::CTRL | Modifiers::SHIFT });
    h.run_steps(2);
    if let Some(l) = label {
        let got = h.state().state.session.active().and_then(|d| d.history.undo_label().map(|s| s.to_string()));
        assert_eq!(got.as_deref(), Some(l.as_str()), "Strg+Umschalt+V = {expected:?}");
    }
    // Umschalt+Einfg (Paste-Ereignis mit Umschalt, ohne Strg) = einfaches Einfügen
    let n = top(&h).len();
    hg_ev(&mut h, Event::ModifiersChanged(Modifiers::SHIFT));
    hg_ev(&mut h, Event::Paste("x".into()));
    hg_ev(&mut h, Event::ModifiersChanged(Modifiers::NONE));
    h.run_steps(2);
    assert!(top(&h).len() > n, "Umschalt+Einfg fügt ein");
}

/// UX-A N07: Pfeiltaste ohne Auswahl tut stumm nichts (keine Meldung in der Statuszeile).
#[test]
fn arrow_without_selection_is_silent() {
    let mut h = harness();
    set_tool(&mut h, ToolId::Pointer);
    h.state_mut().state.status = None;
    key(&mut h, Key::ArrowRight, Modifiers::NONE);
    h.step();
    assert!(h.state().state.status.is_none(), "{:?}", h.state().state.status);
}

/// UX-A Z22 über die Oberfläche: Leertaste während des Aufziehens verschiebt das Rechteck.
#[test]
fn space_during_drag_moves_the_shape_in_ui() {
    let mut h = harness();
    set_tool(&mut h, ToolId::Rectangle);
    let a = at(&h, 200.0, 200.0);
    hg_ev(&mut h, Event::PointerMoved(a));
    hg_ev(&mut h, Event::PointerButton { pos: a, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    let q = at(&h, 230.0, 220.0);
    hg_ev(&mut h, Event::PointerMoved(q));
    let q = at(&h, 260.0, 240.0);
    hg_ev(&mut h, Event::PointerMoved(q));
    hg_ev(&mut h, Event::Key { key: Key::Space, physical_key: Some(Key::Space), pressed: true, repeat: false, modifiers: Modifiers::NONE });
    let q = at(&h, 285.0, 240.0);
    hg_ev(&mut h, Event::PointerMoved(q));
    let q = at(&h, 310.0, 240.0);
    hg_ev(&mut h, Event::PointerMoved(q));
    hg_ev(&mut h, Event::Key { key: Key::Space, physical_key: Some(Key::Space), pressed: false, repeat: false, modifiers: Modifiers::NONE });
    let b = at(&h, 310.0, 240.0);
    hg_ev(&mut h, Event::PointerButton { pos: b, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
    let d = h.state().state.session.active().expect("d");
    let r = fp_engine::docops::element_bounds(&d.doc, &top(&h)[0]).expect("Rechteck");
    assert_eq!((r.left, r.top, r.right, r.bottom), (250.0, 200.0, 310.0, 240.0));
}

/// UX-A L15/L16 mit egui-winit-genauer Folge: Umschalt/Alt nur als `ModifiersChanged` (eigenes
/// Bild, vor dem Mausdruck), Zeigerereignisse mit denselben Modifikatoren, Loslassen der Taste erst
/// nach dem Loslassen der Maus. Umschalt = Quadrat ab dem Startpunkt (ohne Fang), Alt = von der
/// Mitte aus; die Mitte y = 145 rastet mit Abstand 5 an der Seitenmitte 150 ein (PM-1, L16).
#[test]
fn rectangle_shift_alt_held_before_press_winit_sequence() {
    for (m, expect) in [(Modifiers::SHIFT, (240.0, 145.0, 300.0, 205.0)), (Modifiers::ALT, (180.0, 125.0, 300.0, 175.0))] {
        let mut h = harness();
        let _ = h.state_mut().state.session.execute("view.setZoom", json!({"zoom": 1.0}));
        set_tool(&mut h, ToolId::Rectangle);
        h.run_steps(2);
        let a = at(&h, 240.0, 145.0);
        ev(&mut h, Event::PointerMoved(a));
        ev(&mut h, Event::ModifiersChanged(m));
        ev(&mut h, Event::PointerButton { pos: a, button: PointerButton::Primary, pressed: true, modifiers: m });
        for i in 1..=10 {
            let q = at(&h, 240.0 + 6.0 * i as f32, 145.0 + 3.0 * i as f32);
            ev(&mut h, Event::PointerMoved(q));
        }
        let b = at(&h, 300.0, 175.0);
        ev(&mut h, Event::PointerButton { pos: b, button: PointerButton::Primary, pressed: false, modifiers: m });
        ev(&mut h, Event::ModifiersChanged(Modifiers::NONE));
        h.run_steps(2);
        let d = h.state().state.session.active().expect("d");
        let r = fp_engine::docops::element_bounds(&d.doc, &top(&h)[0]).expect("Rechteck");
        assert_eq!((r.left, r.top, r.right, r.bottom), expect, "{m:?}");
    }
}

/// PM-1 (L16, halber Pixel): Ein Zeiger auf Bildschirmpunkt (l + x, t + y) ist bei 100 % genau der
/// Dokumentpunkt (x, y) – auch bei ungerader Leinwandgröße, bei der die zentrierte Seite sonst
/// einen halben Pixel neben dem Zeiger lag. Dazu der Gleichstand-Fall aus der Messung: Alt-Rechteck
/// 240,143 → 300,173 (Ende 2 px von R2-Mitte 175, 3 px von T1-Oberkante 170) ergibt wie im Original
/// 180,111–300,175 (mit dem halben Pixel kam 180,116–300,170 heraus).
#[test]
fn pointer_maps_to_whole_document_pixels_and_tie_snaps_like_original() {
    for (w, hh) in [(1500.0, 950.0), (1501.0, 951.0), (1500.0, 951.0), (1499.0, 948.0)] {
        let mut session = fp_engine::Session::default();
        let _ = session.execute("file.new", json!({"width": 600, "height": 400, "background": "#ffffff"}));
        for (r, c) in [([50, 50, 150, 120], "#ff0000"), ([450, 150, 520, 200], "#0000ff"), ([50, 170, 110, 230], "#000000")] {
            let _ = session.execute("draw.rectangle", json!({"rect": r, "fillColor": c}));
        }
        let _ = session.execute("draw.ellipse", json!({"rect": [200, 50, 300, 120]}));
        let _ = session.execute("select.set", json!({"ids": []}));
        let ctx = egui::Context::default();
        let mut app = FirepixelApp::with_session(&ctx, session, UiSettings::default(), false);
        app.state.os_clipboard = false;
        let mut h = Harness::builder().with_size(egui::vec2(w, hh)).with_pixels_per_point(1.0).with_max_steps(8).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
        h.run_steps(4);
        let _ = h.state_mut().state.session.execute("view.setZoom", json!({"zoom": 1.0}));
        h.run_steps(3);
        let o = at(&h, 0.0, 0.0);
        assert_eq!((o.x.fract(), o.y.fract()), (0.0, 0.0), "{w}×{hh}: Seitenecke {o:?} nicht auf ganzen Pixeln");
        // gezeichnete Seite liegt genau dort
        let page = h.state().state.canvases.values().next().and_then(|c| c.page_screen).expect("Seite gezeichnet");
        assert_eq!(page.min, o, "{w}×{hh}: gezeichnete Seite");
        mv(&mut h, o + egui::vec2(123.0, 77.0));
        assert_eq!(h.state().state.pointer_doc, Some(fp_geom::Point::new(123.0, 77.0)), "{w}×{hh}");
        // Gleichstand-Fall mit Alt, ganzzahlige Zeigerlagen wie SendInput
        set_tool(&mut h, ToolId::Rectangle);
        let p = |x: f32, y: f32| o + egui::vec2(x, y);
        let m = Modifiers::ALT;
        ev(&mut h, Event::PointerMoved(p(240.0, 143.0)));
        ev(&mut h, Event::ModifiersChanged(m));
        ev(&mut h, Event::PointerButton { pos: p(240.0, 143.0), button: PointerButton::Primary, pressed: true, modifiers: m });
        for i in 1..=20 {
            ev(&mut h, Event::PointerMoved(p((240.0 + 3.0 * i as f32).round(), (143.0 + 1.5 * i as f32).round())));
        }
        ev(&mut h, Event::PointerButton { pos: p(300.0, 173.0), button: PointerButton::Primary, pressed: false, modifiers: m });
        ev(&mut h, Event::ModifiersChanged(Modifiers::NONE));
        h.run_steps(2);
        let d = h.state().state.session.active().expect("d");
        let r = fp_engine::docops::element_bounds(&d.doc, &top(&h)[0]).expect("Rechteck");
        assert_eq!((r.left, r.top, r.right, r.bottom), (180.0, 111.0, 300.0, 175.0), "{w}×{hh}");
    }
}

/// Mehrere Ereignisse in **einem** Bild (echte, schnelle Eingabe per SendInput).
fn burst(h: &mut Harness<'static, FirepixelApp>, evs: Vec<Event>) {
    h.input_mut().events.extend(evs);
    h.step();
    h.run_steps(2);
}

/// UX-A L15/L16/L24/L3: Umschalttaste, Drücken, Ziehen und Loslassen im selben Bild – die Lage des
/// Drückens und die Umschalttasten je Ereignis gelten (nicht der Zustand am Bildende).
#[test]
fn burst_input_keeps_press_position_and_modifiers() {
    let mut h = harness();
    let _ = h.state_mut().state.session.execute("view.setZoom", json!({"zoom": 1.0}));
    set_tool(&mut h, ToolId::Rectangle);
    h.run_steps(2);
    let (a, m1, b) = (at(&h, 240.0, 145.0), at(&h, 270.0, 160.0), at(&h, 300.0, 175.0));
    let s = Modifiers::SHIFT;
    burst(&mut h, vec![
        Event::PointerMoved(a),
        Event::ModifiersChanged(s),
        Event::PointerButton { pos: a, button: PointerButton::Primary, pressed: true, modifiers: s },
        Event::PointerMoved(m1),
        Event::PointerMoved(b),
        Event::PointerButton { pos: b, button: PointerButton::Primary, pressed: false, modifiers: s },
        Event::ModifiersChanged(Modifiers::NONE),
    ]);
    let d = h.state().state.session.active().expect("d");
    let r = fp_engine::docops::element_bounds(&d.doc, &top(&h)[0]).expect("Rechteck");
    assert_eq!((r.left, r.top, r.right, r.bottom), (240.0, 145.0, 300.0, 205.0), "Quadrat ab dem Drückpunkt");
    // Stift: Klick, Klick, Doppelklick (der Doppelklick in einem Bild) → offener Pfad mit 3 Punkten
    let _ = h.state_mut().state.session.execute("select.none", json!({}));
    set_tool(&mut h, ToolId::Pen);
    let n0 = top(&h).len();
    let click = |p: Pos2| vec![Event::PointerMoved(p), Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE }];
    let (p1, p2, p3) = (at(&h, 250.0, 50.0), at(&h, 330.0, 50.0), at(&h, 330.0, 120.0));
    burst(&mut h, click(p1));
    burst(&mut h, click(p2));
    let mut dbl = click(p3);
    dbl.extend(click(p3).into_iter().skip(1));
    burst(&mut h, dbl);
    assert_eq!(top(&h).len(), n0 + 1, "Pfad übernommen");
    let e = top(&h)[0].clone();
    let fp_model::ElementKind::Path(pe) = &e.kind else { panic!("kein Pfad") };
    assert_eq!(pe.contours[0].nodes.len(), 3);
    assert!(!pe.contours[0].closed);
    // Zeiger: Umschalt erst während des Ziehens (L3) → waagerecht eingerastet
    set_tool(&mut h, ToolId::Pointer);
    let _ = h.state_mut().state.session.execute("select.none", json!({}));
    let rect = top(&h).into_iter().find(|x| matches!(x.kind, fp_model::ElementKind::Path(ref p) if p.primitive.is_some() || p.contours[0].closed)).expect("Rechteck");
    let id = rect.id;
    let before = fp_engine::docops::element_bounds(&h.state().state.session.active().expect("d").doc, &rect).expect("b");
    let g = at(&h, 270.0, 175.0);
    burst(&mut h, vec![Event::PointerMoved(g), Event::PointerButton { pos: g, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }, Event::PointerMoved(g + egui::vec2(10.0, 2.0))]);
    burst(&mut h, vec![Event::ModifiersChanged(s), Event::PointerMoved(g + egui::vec2(40.0, 10.0)), Event::PointerButton { pos: g + egui::vec2(40.0, 10.0), button: PointerButton::Primary, pressed: false, modifiers: s }, Event::ModifiersChanged(Modifiers::NONE)]);
    let d = h.state().state.session.active().expect("d");
    let el = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("Pfad");
    let after = fp_engine::docops::element_bounds(&d.doc, &el).expect("b");
    assert_eq!((after.left - before.left, after.top - before.top), (40.0, 0.0), "Umschalt während des Ziehens rastet waagerecht");
}

/// Gezeichnete Lage der Leinwand-Textur im letzten Bild (Rechteck des Bildes mit dieser Textur).
fn drawn_canvas_rect(h: &Harness<'static, FirepixelApp>) -> Option<egui::Rect> {
    let st = &h.state().state;
    let id = st.session.active_id()?;
    let tex = st.canvases.get(&id)?.texture.as_ref()?.id();
    let mut r: Option<egui::Rect> = None;
    for cs in &h.output().shapes {
        if let egui::epaint::Shape::Mesh(m) = &cs.shape
            && m.texture_id == tex
        {
            let b = m.calc_bounds();
            r = Some(r.map_or(b, |x| x.union(b)));
        }
    }
    r
}

/// Erwartete Lage der ganzen Seite im aktuellen Ansichtszustand.
fn expected_page_rect(h: &Harness<'static, FirepixelApp>) -> egui::Rect {
    let st = &h.state().state;
    let d = st.session.active().expect("d");
    let area = st.canvases.get(&d.id).and_then(|c| c.area).expect("area");
    let pr = fp_render::Renderer::page_rect(&d.doc, d.page).expect("Seite");
    egui::Rect::from_min_max(super::to_screen(&d.view, area.min, pr.left as f64, pr.top as f64), super::to_screen(&d.view, area.min, pr.right as f64, pr.bottom as f64))
}

/// Nutzerbefund: Beim Zoomen darf die Leinwand nie ein Bild lang mit neuem Zoom an der alten Lage
/// stehen. Nach JEDEM Bild (1/60 s) passt die gezeichnete Textur zur Ansicht, kleine Seiten sind
/// zentriert – auch während das Hintergrund-Rendering noch läuft (alte Textur skaliert).
#[test]
fn zoom_never_draws_canvas_at_stale_offset() {
    let mut h = harness();
    let _ = h.state_mut().state.session.execute("file.new", json!({"width": 1254, "height": 1254}));
    let _ = h.state_mut().state.session.execute("draw.rectangle", json!({"rect": [100, 100, 1100, 1100]}));
    h.run_steps(3);
    let _ = h.state_mut().state.session.execute("view.setZoom", json!({"zoom": 0.5}));
    h.run_steps(4);
    let check = |h: &Harness<'static, FirepixelApp>, what: &str| {
        let Some(drawn) = drawn_canvas_rect(h) else { return };
        let st = &h.state().state;
        // Seitenlage, mit der dieses Bild gezeichnet wurde
        let want = st.canvases.get(&st.session.active_id().expect("id")).and_then(|c| c.page_screen).unwrap_or_else(|| expected_page_rect(h));
        let area = st.canvases.get(&st.session.active_id().expect("id")).and_then(|c| c.area).expect("area");
        // die gezeichnete Textur deckt einen Ausschnitt der Seite: sie muss innerhalb der erwarteten
        // Seitenlage liegen (keine Verschiebung) und kleine Seiten sind waagerecht zentriert
        assert!(drawn.min.x >= want.min.x - 1.0 && drawn.min.y >= want.min.y - 1.0 && drawn.max.x <= want.max.x + 1.0 && drawn.max.y <= want.max.y + 1.0, "{what}: gezeichnet {drawn:?}, Seite {want:?}");
        if want.width() <= area.width() {
            assert!((want.center().x - area.center().x).abs() <= 1.0, "{what}: nicht zentriert {want:?} in {area:?}");
        }
    };
    check(&h, "Ausgang 50 %");
    // Strg+Mausrad an verschiedenen Stellen, mit verzögertem Hintergrund-Rendering
    for (sync, pos) in [(true, 0.2f32), (false, 0.8), (false, 0.3)] {
        h.state_mut().state.sync_render = sync;
        let area = h.state().state.canvases.get(&h.state().state.session.active_id().expect("id")).and_then(|c| c.area).expect("area");
        let p = egui::pos2(area.left() + area.width() * pos, area.top() + area.height() * 0.4);
        hg_ev(&mut h, Event::PointerMoved(p));
        h.input_mut().events.push(Event::MouseWheel { unit: egui::MouseWheelUnit::Line, delta: egui::vec2(0.0, 1.0), phase: egui::TouchPhase::Move, modifiers: Modifiers::CTRL });
        for f in 0..6 {
            h.step();
            check(&h, &format!("Rad hinein, Bild {f}, sync {sync}"));
        }
        h.input_mut().events.push(Event::MouseWheel { unit: egui::MouseWheelUnit::Line, delta: egui::vec2(0.0, -1.0), phase: egui::TouchPhase::Move, modifiers: Modifiers::CTRL });
        for f in 0..6 {
            h.step();
            check(&h, &format!("Rad heraus, Bild {f}, sync {sync}"));
        }
    }
    // Befehlswege: Strg+Plus/Minus, 100 %, Alles anpassen
    h.state_mut().state.sync_render = false;
    for cmd in ["view.zoomIn", "view.zoomOut", "view.zoomIn", "view.fitAll"] {
        h.state_mut().state.run(cmd, json!({}));
        for f in 0..4 {
            h.step();
            check(&h, &format!("{cmd}, Bild {f}"));
        }
    }
}

/// Nutzerbefund: heller 1-px-Streifen am Seitenrand. Dunkles Bild bis zum Rand; bei vielen
/// Zoomstufen, geraden/ungeraden Fensterbreiten und DPI 100/125/150 % darf an keiner der vier
/// Kanten innerhalb der Seite ein Pixel in Hintergrundfarbe liegen und außerhalb kein Bildpixel.
#[test]
fn page_edges_are_pixel_exact() {
    let mut failures = Vec::new();
    let mut checked = 0;
    'outer: for ppp in [1.0f32, 1.25, 1.5] {
        for width in [1500.0f32, 1501.0] {
            let mut session = fp_engine::Session::default();
            let _ = session.execute("file.new", json!({"width": 1920, "height": 1080, "background": "#ffffff"}));
            let img = fp_raster::RgbaImage::filled(fp_geom::IRect::from_xywh(0, 0, 1920, 1080), fp_color::Rgba8::new(20, 20, 20, 255)).expect("Bild");
            session.clipboard.clear();
            session.clipboard.pixels = Some(img);
            let _ = session.execute("edit.paste", json!({}));
            let _ = session.execute("select.none", json!({}));
            let ctx = egui::Context::default();
            let mut app = FirepixelApp::with_session(&ctx, session, UiSettings::default(), false);
            app.state.os_clipboard = false;
            let mut h = Harness::builder().with_size(egui::vec2(width, 950.0)).with_pixels_per_point(ppp).with_max_steps(8).with_step_dt(1.0 / 60.0).wgpu().build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
            h.run_steps(4);
            // Zoomstufen, dazu bei großen Zoomstufen an den rechten unteren Rand gerollt (Teil-Rendering:
            // der Ausschnitt beginnt mitten in der Seite und endet an ihrer Kante)
            for (z, scrolled) in [(0.25f64, false), (0.33, false), (0.5, false), (0.66, false), (1.0, false), (1.0, true), (1.5, true), (2.0, false), (2.0, true), (4.0, true)] {
                let _ = h.state_mut().state.session.execute("view.setZoom", json!({"zoom": z}));
                if scrolled && let Some(d) = h.state_mut().state.session.active_mut() {
                    d.view.pan(-100_000.0, -100_000.0);
                }
                h.run_steps(3);
                let shot = match h.render() {
                    Ok(i) => i,
                    Err(e) => {
                        eprintln!("kein Grafikadapter: {e}");
                        break 'outer;
                    }
                };
                let st = &h.state().state;
                let cc = st.canvases.get(&st.session.active_id().expect("id")).expect("cc");
                let (Some(page), Some(area)) = (cc.page_screen, cc.area) else { continue };
                let vis = page.intersect(area);
                // Gerätepixel: Pixel (x, y) deckt [x, x+1) / ppp
                let to_px = |v: f32| v * ppp;
                let dark = |x: i64, y: i64| {
                    if x < 0 || y < 0 || x >= shot.width() as i64 || y >= shot.height() as i64 {
                        return None;
                    }
                    let p = shot.get_pixel(x as u32, y as u32).0;
                    Some(p[0] < 60 && p[1] < 60 && p[2] < 60)
                };
                let (l, r, t, b) = (to_px(page.left()), to_px(page.right()), to_px(page.top()), to_px(page.bottom()));
                let (vl, vr, vt, vb) = (to_px(vis.left()), to_px(vis.right()), to_px(vis.top()), to_px(vis.bottom()));
                let ymid = ((vt + vb) / 2.0) as i64;
                let xmid = ((vl + vr) / 2.0) as i64;
                let mut probe = |name: &str, inside: (i64, i64), outside: (i64, i64), edge_visible: bool| {
                    if !edge_visible {
                        return;
                    }
                    checked += 1;
                    if dark(inside.0, inside.1) == Some(false) {
                        let around: Vec<[u8; 4]> = (-2..=2)
                            .map(|d| {
                                let (x, y) = if name == "links" || name == "rechts" { (inside.0 + d, inside.1) } else { (inside.0, inside.1 + d) };
                                shot.get_pixel(x.max(0) as u32, y.max(0) as u32).0
                            })
                            .collect();
                        failures.push(format!("ppp {ppp} Breite {width} Zoom {z}: {name} innen hell bei {inside:?} (Seite {page:?}) Umgebung {around:?}"));
                    }
                    if dark(outside.0, outside.1) == Some(true) {
                        failures.push(format!("ppp {ppp} Breite {width} Zoom {z}: {name} außen dunkel bei {outside:?} (Seite {page:?})"));
                    }
                };
                // erste/letzte Pixelspalte bzw. -zeile mit Mittelpunkt in der Seite
                let first = |a: f32| (a - 0.5).ceil() as i64;
                let last = |a: f32| (a - 0.5).floor() as i64;
                probe("links", (first(l), ymid), (first(l) - 1, ymid), l >= to_px(area.left()) + 1.0);
                probe("rechts", (last(r), ymid), (last(r) + 1, ymid), r <= to_px(area.right()) - 1.0);
                probe("oben", (xmid, first(t)), (xmid, first(t) - 1), t >= to_px(area.top()) + 1.0);
                probe("unten", (xmid, last(b)), (xmid, last(b) + 1), b <= to_px(area.bottom()) - 1.0);
            }
        }
    }
    eprintln!("Kanten geprüft: {checked}");
    assert!(failures.is_empty(), "{failures:#?}");
}

/// Mausrad ohne Strg mit HGs Ereignisfolge (Zoom 400 %, Zeile −1, danach 15 Bilder).
#[test]
fn plain_wheel_scrolls_with_hg_sequence() {
    let mut h = harness();
    // wie HGs Testdokument: 800 × 600
    let _ = h.state_mut().state.session.execute("file.new", json!({"width": 800, "height": 600}));
    h.run_steps(3);
    let _ = h.state_mut().state.session.execute("view.setZoom", json!({"zoom": 4.0}));
    h.run_steps(3);
    // HGs Punkt (Dokument 300,200) liegt bei 400 % außerhalb der sichtbaren Leinwand (über der
    // Ansichtsleiste) – dort darf das Rad die Leinwand nicht rollen; daher Mitte der Leinwand
    let area = h.state().state.canvases.get(&h.state().state.session.active_id().expect("id")).and_then(|c| c.area).expect("area");
    assert!(!area.contains(at(&h, 300.0, 200.0)));
    let p = area.center();
    let o0 = h.state().state.session.active().map(|d| d.view.origin).unwrap_or_default();
    hg_ev(&mut h, Event::PointerMoved(p));
    hg_ev(&mut h, Event::MouseWheel { unit: egui::MouseWheelUnit::Line, delta: egui::vec2(0.0, -1.0), phase: egui::TouchPhase::Move, modifiers: Modifiers::NONE });
    h.run_steps(15);
    let o1 = h.state().state.session.active().map(|d| d.view.origin).unwrap_or_default();
    let dy = (o1.1 - o0.1) * 4.0;
    // eine Rastung = 52 Bildschirmpixel wie im Original (UX-A L9)
    assert!((dy - 52.0).abs() < 0.01, "Bildlauf {dy}");
    // Umschalt+Rad tut im Original nichts (L10), Alt+Rad rollt senkrecht wie ohne Taste (L11)
    hg_ev(&mut h, Event::MouseWheel { unit: egui::MouseWheelUnit::Line, delta: egui::vec2(0.0, -1.0), phase: egui::TouchPhase::Move, modifiers: Modifiers::SHIFT });
    h.run_steps(5);
    let o2 = h.state().state.session.active().map(|d| d.view.origin).unwrap_or_default();
    assert_eq!(o2, o1, "Umschalt+Rad: nichts");
    hg_ev(&mut h, Event::MouseWheel { unit: egui::MouseWheelUnit::Line, delta: egui::vec2(0.0, -1.0), phase: egui::TouchPhase::Move, modifiers: Modifiers::ALT });
    h.run_steps(5);
    let o3 = h.state().state.session.active().map(|d| d.view.origin).unwrap_or_default();
    assert!(((o3.1 - o2.1) * 4.0 - 52.0).abs() < 0.01 && o3.0 == o2.0, "Alt+Rad senkrecht {o2:?} → {o3:?}");
}

/// Doppelklick mit dem Zeiger auf Text öffnet den Text-Editor an der Klickstelle (HGs Folge).
#[test]
fn double_click_text_starts_editing_hg_sequence() {
    let mut h = harness();
    let _ = h.state_mut().state.session.execute("draw.text", json!({"text": "Firepixel Test", "anchor": [40, 330], "size": 24}));
    let _ = h.state_mut().state.session.execute("select.none", json!({}));
    set_tool(&mut h, ToolId::Pointer);
    let d = h.state().state.session.active().expect("d");
    let e = top(&h)[0].clone();
    let b = fp_engine::docops::element_bounds(&d.doc, &e).expect("Grenzen");
    let p = at(&h, b.left + 30.0, (b.top + b.bottom) / 2.0);
    hg_click(&mut h, p);
    assert_eq!(h.state().state.session.active().map(|d| d.selection.elements.len()), Some(1), "erster Klick wählt den Text");
    hg_click(&mut h, p);
    h.run_steps(2);
    assert!(h.state().state.session.tool_is_editing_text(), "Text-Editor offen (Werkzeug {:?})", h.state().state.session.tools.active);
    assert_eq!(h.state().state.session.tools.active, ToolId::Text);
    // Einfügemarke an der Klickstelle, nicht am Ende
    let caret = h.state().state.canvases.values().next().and_then(|c| c.feedback.iter().find_map(|o| match o {
        Overlay::TextEdit(v) => Some(v.caret),
        _ => None,
    }));
    assert!(caret.is_some_and(|c| c > 0 && c < "Firepixel Test".chars().count()), "Caret {caret:?}");
}

/// Nutzerbefund: Text beginnen, „Ha“ tippen, Strg+A, im Inspektor 200 pt wählen → der markierte
/// Text ändert sich live (N15), der Editor bleibt offen, der Editorrahmen wächst mit; nach Esc ist
/// der Auswahlrahmen so groß wie der gesetzte Text (nicht die alte Schätzung).
#[test]
fn inspector_size_while_typing_grows_text_live() {
    let mut h = harness();
    set_tool(&mut h, ToolId::Text);
    let p = at(&h, 40.0, 40.0);
    hg_click(&mut h, p);
    h.run_steps(2);
    assert!(h.state().state.session.tool_is_editing_text());
    hg_ev(&mut h, Event::Text("Ha".into()));
    h.run_steps(2);
    let frame = |h: &Harness<'static, FirepixelApp>| h.state().state.canvases.values().next().and_then(|c| c.text.as_ref().map(|t| t.frame)).expect("Editor-Satz");
    let small = frame(&h);
    key(&mut h, Key::A, Modifiers::CTRL);
    // genau das, was das Größenfeld des Inspektors schickt
    h.state_mut().state.run("text.setAttributes", json!({"size": 200}));
    h.run_steps(3);
    assert!(h.state().state.session.tool_is_editing_text(), "Editor bleibt offen");
    let big = frame(&h);
    assert!(big.height() > small.height() * 8.0, "Editorrahmen wächst live: {small:?} → {big:?}");
    let n0 = undo_count(&h);
    hg_ev(&mut h, Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
    hg_ev(&mut h, Event::Key { key: egui::Key::Escape, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::NONE });
    h.run_steps(3);
    assert!(!h.state().state.session.tool_is_editing_text());
    assert_eq!(undo_count(&h), n0 + 1, "ein Verlaufsschritt");
    let sel = h.state().state.canvases.values().next().and_then(|c| c.sel_frames.first().copied()).expect("Auswahlrahmen");
    let (w, hgt) = (sel[1].x - sel[0].x, sel[2].y - sel[1].y);
    assert!(hgt > 180.0 && w > big.width() - 2.0, "Auswahlrahmen {w}×{hgt}, Satz {big:?}");
}

/// PARITAET_A T6 (PM-1): ohne Markierung gilt die im Inspektor gewählte Größe nur für die danach
/// getippten Zeichen – „Abc“, 48, „D“, Esc ergibt [Abc][D 48]; der Rahmen wächst erst mit dem „D“.
#[test]
fn inspector_size_at_caret_applies_to_next_characters() {
    let mut h = harness();
    set_tool(&mut h, ToolId::Text);
    let p = at(&h, 40.0, 60.0);
    hg_click(&mut h, p);
    h.run_steps(2);
    for c in ["A", "b", "c"] {
        hg_ev(&mut h, Event::Text(c.into()));
    }
    h.run_steps(2);
    let frame = |h: &Harness<'static, FirepixelApp>| h.state().state.canvases.values().next().and_then(|c| c.text.as_ref().map(|t| t.frame)).expect("Editor-Satz");
    let small = frame(&h);
    let size0 = h.state().state.session.text_edit_attrs().and_then(|a| a.size).expect("Größe");
    h.state_mut().state.run("text.setAttributes", json!({"size": 48}));
    h.run_steps(3);
    assert_eq!(frame(&h), small, "vorhandene Zeichen bleiben");
    assert_eq!(h.state().state.session.text_edit_attrs().and_then(|a| a.size), Some(48.0), "Inspektor zeigt 48");
    hg_ev(&mut h, Event::Text("D".into()));
    h.run_steps(3);
    assert!(frame(&h).height() > small.height() * 1.5, "Rahmen wächst mit dem großen D");
    key(&mut h, Key::Escape, Modifiers::NONE);
    h.run_steps(3);
    let d = h.state().state.session.active().expect("d");
    let ElementKind::Text(t) = &top(&h)[0].kind else { panic!("kein Text") };
    let runs: Vec<(String, Option<f32>)> = t.resolved_runs().into_iter().map(|(a, s)| (s.to_string(), a.size)).collect();
    assert_eq!(runs, vec![("Abc".to_string(), Some(size0)), ("D".to_string(), Some(48.0))]);
    assert_eq!(d.selection.elements.len(), 1);
}

/// Ausgewähltes Objekt auf den Reiter eines anderen Dokuments ziehen: Dokument wird aktiv, Ablegen
/// auf der Leinwand kopiert das Objekt hinein (ein Verlaufsschritt), das Quelldokument bleibt.
#[test]
fn drag_object_onto_other_document_tab() {
    let mut h = harness();
    let src = h.state().state.session.active_id().expect("Quelle");
    let _ = h.state_mut().state.session.execute("draw.rectangle", json!({"rect": [50, 50, 150, 120]}));
    let _ = h.state_mut().state.session.execute("file.new", json!({"width": 400, "height": 300, "background": "#ffffff"}));
    let dst = h.state().state.session.active_id().expect("Ziel");
    h.state_mut().state.push(crate::app_state::Action::Activate(src));
    h.run_steps(3);
    set_tool(&mut h, ToolId::Pointer);
    let src_before = doc_clone(&h);
    let tabs: Vec<(fp_engine::DocId, egui::Rect)> = h.ctx.data(|d| d.get_temp(egui::Id::new("lw-doc-tabs"))).unwrap_or_default();
    let tab = tabs.iter().find(|(d, _)| *d == dst).map(|(_, r)| r.center()).expect("Reiter");
    let a = at(&h, 100.0, 80.0);
    hg_ev(&mut h, Event::PointerMoved(a));
    hg_ev(&mut h, Event::PointerButton { pos: a, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    for i in 1..=12 {
        let t = i as f32 / 12.0;
        hg_ev(&mut h, Event::PointerMoved(a + (tab - a) * t));
    }
    h.run_steps(3);
    assert_eq!(h.state().state.session.active_id(), Some(dst), "Reiter wird aktiv");
    let drop = at(&h, 200.0, 150.0);
    for i in 1..=8 {
        let t = i as f32 / 8.0;
        hg_ev(&mut h, Event::PointerMoved(tab + (drop - tab) * t));
    }
    let n0 = undo_count(&h);
    hg_ev(&mut h, Event::PointerButton { pos: drop, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
    assert_eq!(h.state().state.session.active_id(), Some(dst));
    assert_eq!(top(&h).len(), 1, "Objekt hineinkopiert");
    assert_eq!(undo_count(&h), n0 + 1, "ein Verlaufsschritt");
    let d = h.state().state.session.active().expect("d");
    let b = fp_engine::docops::element_bounds(&d.doc, &top(&h)[0]).expect("Grenzen");
    assert_eq!((b.left, b.top), (150.0, 120.0), "an der Ablagestelle (Greifpunkt bleibt unter dem Zeiger)");
    // Quelldokument unverändert
    h.state_mut().state.push(crate::app_state::Action::Activate(src));
    h.run_steps(2);
    assert_eq!(doc_clone(&h), src_before);
}

fn wheel(h: &mut Harness<'static, FirepixelApp>, unit: egui::MouseWheelUnit, dy: f32, ctrl: bool) {
    let m = if ctrl { Modifiers::CTRL } else { Modifiers::NONE };
    h.input_mut().events.push(Event::MouseWheel { unit, delta: egui::vec2(0.0, dy), phase: egui::TouchPhase::Move, modifiers: m });
    h.step();
}

fn zoom(h: &Harness<'static, FirepixelApp>) -> f64 {
    h.state().state.session.active().map(|d| d.view.zoom).unwrap_or(0.0)
}

/// Zoomstufe nach `n` Schritten hinein (+) bzw. heraus (−) von `z` aus.
fn steps_from(h: &Harness<'static, FirepixelApp>, n: i32) -> f64 {
    let mut v = h.state().state.session.active().expect("d").view.clone();
    for _ in 0..n.abs() {
        if n > 0 { v.zoom_in(None) } else { v.zoom_out(None) }
    }
    v.zoom
}

/// Strg+Mausrad: genau eine Zoomstufe je Rastung – mit Zeilen-Ereignissen (Mausrad), großen
/// Punkt-Deltas (hochauflösendes Rad, 120 je Rastung), vielen kleinen Punkt-Deltas (Touchpad)
/// und mit den Frames danach (egui glättet das Scrollen über mehrere Frames).
#[test]
fn ctrl_wheel_zooms_one_step_per_notch() {
    let mut h = harness();
    set_tool(&mut h, ToolId::Pointer);
    // so weit vergrößert, dass die Seite größer als die Ansicht ist (sonst wird sie zentriert)
    for _ in 0..4 {
        let _ = h.state_mut().state.session.execute("view.zoomIn", json!({}));
    }
    h.run_steps(2);
    let p = at(&h, 100.0, 100.0);
    mv(&mut h, p);
    let z0 = zoom(&h);
    let one_in = steps_from(&h, 1);
    wheel(&mut h, egui::MouseWheelUnit::Line, 1.0, true);
    h.run_steps(10);
    assert_eq!(zoom(&h), one_in, "eine Rastung (Zeile) = eine Stufe, auch nach den Glättungs-Frames");
    // Zoom um den Mauszeiger: der Dokumentpunkt unter dem Zeiger bleibt stehen
    let q = at(&h, 100.0, 100.0);
    assert!((q.x - p.x).abs() <= 1.0 && (q.y - p.y).abs() <= 1.0, "{p:?} → {q:?}");
    wheel(&mut h, egui::MouseWheelUnit::Line, -1.0, true);
    h.run_steps(10);
    assert_eq!(zoom(&h), z0);
    // Windows meldet je Rastung u. U. 3 Zeilen: trotzdem eine Stufe
    wheel(&mut h, egui::MouseWheelUnit::Line, 3.0, true);
    h.run_steps(5);
    assert_eq!(zoom(&h), one_in);
    wheel(&mut h, egui::MouseWheelUnit::Line, -3.0, true);
    // hochauflösendes Rad: 120 Punkte = eine Rastung
    wheel(&mut h, egui::MouseWheelUnit::Point, 120.0, true);
    h.run_steps(10);
    assert_eq!(zoom(&h), one_in, "120 Punkte = eine Stufe");
    wheel(&mut h, egui::MouseWheelUnit::Point, -120.0, true);
    assert_eq!(zoom(&h), z0);
    // Touchpad: 30 kleine Deltas zu je 4 Punkten = 120 Punkte = eine Stufe
    for _ in 0..30 {
        wheel(&mut h, egui::MouseWheelUnit::Point, 4.0, true);
    }
    h.run_steps(10);
    assert_eq!(zoom(&h), one_in, "viele kleine Deltas ergeben genau eine Stufe");
    // fünf Rastungen = fünf Stufen, nicht bis zum Anschlag
    let five = steps_from(&h, 5);
    for _ in 0..5 {
        wheel(&mut h, egui::MouseWheelUnit::Line, 1.0, true);
    }
    h.run_steps(10);
    assert_eq!(zoom(&h), five);
}

/// Mausrad ohne Strg: Bildlauf in normaler Geschwindigkeit, kein Zoom.
#[test]
fn plain_wheel_scrolls_normally() {
    let mut h = harness();
    let _ = h.state_mut().state.session.execute("view.setViewport", json!({"width": 400, "height": 400}));
    // vergrößern, damit es etwas zu rollen gibt
    for _ in 0..4 {
        let _ = h.state_mut().state.session.execute("view.zoomIn", json!({}));
    }
    h.run_steps(2);
    let p = at(&h, 200.0, 150.0);
    mv(&mut h, p);
    let z = zoom(&h);
    let o0 = h.state().state.session.active().map(|d| d.view.origin).unwrap_or_default();
    wheel(&mut h, egui::MouseWheelUnit::Line, -1.0, false);
    h.run_steps(20);
    let o1 = h.state().state.session.active().map(|d| d.view.origin).unwrap_or_default();
    assert_eq!(zoom(&h), z, "kein Zoom ohne Strg");
    let moved_px = ((o1.1 - o0.1) * z).abs();
    assert!((10.0..=200.0).contains(&moved_px), "eine Rastung rollt {moved_px} px");
}

#[test]
fn text_tool_typing_home_key_and_ime() {
    let mut h = harness();
    set_tool(&mut h, ToolId::Text);
    let p = at(&h, 40.0, 60.0);
    mv(&mut h, p);
    button(&mut h, p, PointerButton::Primary, true, Modifiers::NONE);
    button(&mut h, p, PointerButton::Primary, false, Modifiers::NONE);
    assert!(h.state().state.session.tool_is_editing_text());
    ev(&mut h, Event::Text("Hallo".into()));
    h.step();
    assert!(h.state().state.canvases.values().next().is_some_and(|c| c.text.is_some()), "Satz über fp-text");
    // Pos1 über das Layout, Buchstaben gehen nicht an die Werkzeugkürzel
    key(&mut h, Key::Home, Modifiers::NONE);
    key(&mut h, Key::V, Modifiers::NONE);
    ev(&mut h, Event::Text("V".into()));
    assert_eq!(h.state().state.session.tools.active, ToolId::Text, "V ist Text, kein Werkzeugwechsel");
    // IME: Vorschau, dann Übernahme
    ev(&mut h, Event::Ime(egui::ImeEvent::Preedit { text: "あ".into(), active_range_chars: None }));
    ev(&mut h, Event::Ime(egui::ImeEvent::Commit("あ".into())));
    key(&mut h, Key::Escape, Modifiers::NONE);
    h.step();
    let t = top(&h);
    assert!(matches!(&t[0].kind, ElementKind::Text(x) if x.plain_text() == "VあHallo"), "{:?}", t[0].kind);
}

#[test]
fn ctrl_z_undoes_typing_inside_the_editor() {
    let mut h = harness();
    let _ = h.state_mut().state.session.execute("draw.rectangle", json!({"rect": [200, 200, 260, 260]}));
    set_tool(&mut h, ToolId::Text);
    let p = at(&h, 40.0, 60.0);
    mv(&mut h, p);
    button(&mut h, p, PointerButton::Primary, true, Modifiers::NONE);
    button(&mut h, p, PointerButton::Primary, false, Modifiers::NONE);
    ev(&mut h, Event::Text("Hallo".into()));
    ev(&mut h, Event::Text(" Welt".into()));
    key(&mut h, Key::Backspace, Modifiers::NONE);
    let text = |h: &Harness<'static, FirepixelApp>| {
        h.state().state.canvases.values().next().and_then(|c| c.feedback.iter().find_map(|o| match o {
            Overlay::TextEdit(v) => Some(v.text.clone()),
            _ => None,
        }))
    };
    assert_eq!(text(&h).as_deref(), Some("Hallo Wel"));
    let hist = h.state().state.session.active().map(|d| d.history.undo_count()).unwrap_or(0);
    key(&mut h, Key::Z, Modifiers::CTRL);
    assert_eq!(text(&h).as_deref(), Some("Hallo Welt"), "Rücktaste zurückgenommen");
    key(&mut h, Key::Z, Modifiers::CTRL);
    assert_eq!(text(&h).as_deref(), Some(""), "Tippen (ein Schritt) zurückgenommen");
    key(&mut h, Key::Y, Modifiers::CTRL);
    assert_eq!(text(&h).as_deref(), Some("Hallo Welt"));
    key(&mut h, Key::Z, Modifiers::CTRL | Modifiers::SHIFT);
    assert_eq!(text(&h).as_deref(), Some("Hallo Wel"), "Umschalt+Strg+Z = Wiederholen");
    // der Dokumentverlauf blieb unberührt (das Rechteck ist noch da)
    assert_eq!(h.state().state.session.active().map(|d| d.history.undo_count()).unwrap_or(0), hist);
    assert_eq!(top(&h).len(), 1);
    key(&mut h, Key::Escape, Modifiers::NONE);
    h.step();
    assert!(matches!(&top(&h)[0].kind, ElementKind::Text(x) if x.plain_text() == "Hallo Wel"));
}

#[test]
fn guides_move_and_delete_by_dragging_out() {
    let mut h = harness();
    let _ = h.state_mut().state.session.execute("guide.add", json!({"orientation": "vertical", "position": 100}));
    set_tool(&mut h, ToolId::Pointer);
    h.step();
    let (a, b) = (at(&h, 100.0, 150.0), at(&h, 140.0, 150.0));
    mouse_drag(&mut h, a, b, Modifiers::NONE);
    let g = |h: &Harness<'static, FirepixelApp>| h.state().state.session.active().and_then(|d| d.doc.pages[0].root.guides.as_ref().map(|g| g.vertical.clone())).unwrap_or_default();
    assert_eq!(g(&h), vec![140.0]);
    // ins Lineal ziehen = löschen
    let area = h.state().state.canvases.values().next().and_then(|c| c.area).expect("area");
    mouse_drag(&mut h, b, egui::pos2(area.left() - 5.0, b.y), Modifiers::NONE);
    assert!(g(&h).is_empty());
}

#[test]
fn marquee_ants_and_context_menu() {
    let mut h = harness();
    set_tool(&mut h, ToolId::Marquee);
    let (a, b) = (at(&h, 20.0, 20.0), at(&h, 80.0, 60.0));
    mouse_drag(&mut h, a, b, Modifiers::NONE);
    h.step();
    let cc = h.state().state.canvases.values().next().expect("Leinwand");
    let ants = cc.ants.as_ref().expect("Ameisen");
    assert_eq!(ants.outlines.len(), 1);
    // Rechtsklick: Kontextmenü „Pixel auswählen“ (6913)
    let p = at(&h, 40.0, 40.0);
    mv(&mut h, p);
    button(&mut h, p, PointerButton::Secondary, true, Modifiers::NONE);
    button(&mut h, p, PointerButton::Secondary, false, Modifiers::NONE);
    h.step();
    let cc = h.state().state.canvases.values().next().expect("Leinwand");
    assert_eq!(cc.context.as_ref().map(|c| c.id), Some("6913"));
}

#[test]
fn right_click_selects_object_under_pointer() {
    let mut h = harness();
    let _ = h.state_mut().state.session.execute("draw.rectangle", json!({"rect": [50, 50, 150, 120]}));
    let _ = h.state_mut().state.session.execute("select.none", json!({}));
    set_tool(&mut h, ToolId::Pointer);
    let p = at(&h, 100.0, 80.0);
    mv(&mut h, p);
    button(&mut h, p, PointerButton::Secondary, true, Modifiers::NONE);
    button(&mut h, p, PointerButton::Secondary, false, Modifiers::NONE);
    h.run_steps(2);
    let st = &h.state().state;
    assert_eq!(st.session.active().map(|d| d.selection.elements.len()), Some(1));
    assert_eq!(st.canvases.values().next().and_then(|c| c.context.as_ref().map(|c| c.id)), Some("6925"));
}

/// Bildschirmfotos der Werkzeuge (offscreen mit wgpu) nach `docs\screenshots\phase5_lw_*.png`:
/// `cargo test -p fp-ui --lib canvas::tests::screenshots -- --ignored`.
#[test]
#[ignore]
fn screenshots() {
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("docs").join("screenshots");
    let _ = std::fs::create_dir_all(&out);
    let mut session = fp_engine::Session::default();
    let _ = session.execute("file.new", json!({"width": 640, "height": 420, "background": "#ffffff"}));
    // Bitmap mit Farbflächen für Zauberstab und Pinsel
    let mut img = fp_raster::RgbaImage::filled(fp_geom::IRect::from_xywh(380, 220, 220, 160), fp_color::Rgba8::new(70, 130, 200, 255)).expect("Bild");
    for y in 220..380 {
        for x in 490..600 {
            img.set_pixel(x, y, fp_color::Rgba8::new(240, 190, 60, 255));
        }
    }
    session.clipboard.clear();
    session.clipboard.pixels = Some(img);
    let _ = session.execute("edit.paste", json!({}));
    let _ = session.execute("select.none", json!({}));
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, session, UiSettings::default(), false);
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1600.0, 1000.0)).with_pixels_per_point(1.0).with_max_steps(8).wgpu().build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(6);
    let shot = |h: &mut Harness<'static, FirepixelApp>, name: &str| {
        h.step();
        match h.render() {
            Ok(i) => {
                let _ = i.save(out.join(format!("phase5_lw_{name}.png")));
            }
            Err(e) => eprintln!("kein Grafikadapter: {e}"),
        }
    };
    // Rechteck und Ellipse (Ellipse noch beim Aufziehen mit Größenbeschriftung)
    set_tool(&mut h, ToolId::Rectangle);
    let (a, b) = (at(&h, 40.0, 40.0), at(&h, 200.0, 140.0));
    mouse_drag(&mut h, a, b, Modifiers::NONE);
    set_tool(&mut h, ToolId::Ellipse);
    let (a, b) = (at(&h, 240.0, 40.0), at(&h, 360.0, 150.0));
    mv(&mut h, a);
    button(&mut h, a, PointerButton::Primary, true, Modifiers::NONE);
    mv(&mut h, b);
    shot(&mut h, "rechteck_ellipse");
    button(&mut h, b, PointerButton::Primary, false, Modifiers::NONE);
    // Stift: drei Punkte, der letzte gezogen, Vorschau des nächsten Segments
    set_tool(&mut h, ToolId::Pen);
    for (x, y, d) in [(40.0, 220.0, false), (120.0, 180.0, true), (200.0, 260.0, false)] {
        let p = at(&h, x, y);
        mv(&mut h, p);
        button(&mut h, p, PointerButton::Primary, true, Modifiers::NONE);
        let q = if d { at(&h, x + 40.0, y) } else { p };
        mv(&mut h, q);
        button(&mut h, q, PointerButton::Primary, false, Modifiers::NONE);
    }
    let q = at(&h, 280.0, 300.0);
    mv(&mut h, q);
    shot(&mut h, "stift");
    key(&mut h, Key::Enter, Modifiers::NONE);
    // Verschieben mit Vorschau (Rechteck)
    set_tool(&mut h, ToolId::Pointer);
    let (a, b) = (at(&h, 120.0, 90.0), at(&h, 160.0, 120.0));
    mv(&mut h, a);
    button(&mut h, a, PointerButton::Primary, true, Modifiers::NONE);
    mv(&mut h, a + egui::vec2(8.0, 4.0));
    mv(&mut h, b);
    shot(&mut h, "verschieben");
    button(&mut h, b, PointerButton::Primary, false, Modifiers::NONE);
    // Skalieren: Rahmen mit Griffen, Ecke gezogen
    set_tool(&mut h, ToolId::Scale);
    let (a, b) = (at(&h, 240.0, 170.0), at(&h, 290.0, 200.0));
    mv(&mut h, a);
    button(&mut h, a, PointerButton::Primary, true, Modifiers::NONE);
    mv(&mut h, b);
    shot(&mut h, "skalieren");
    button(&mut h, b, PointerButton::Primary, false, Modifiers::NONE);
    // Drehen: außerhalb des Rahmens ziehen
    let (a, b) = (at(&h, 330.0, 60.0), at(&h, 360.0, 160.0));
    mv(&mut h, a);
    button(&mut h, a, PointerButton::Primary, true, Modifiers::NONE);
    mv(&mut h, b);
    shot(&mut h, "drehen");
    button(&mut h, b, PointerButton::Primary, false, Modifiers::NONE);
    // Text tippen: Caret und Markierung
    set_tool(&mut h, ToolId::Text);
    let p = at(&h, 40.0, 330.0);
    mv(&mut h, p);
    button(&mut h, p, PointerButton::Primary, true, Modifiers::NONE);
    button(&mut h, p, PointerButton::Primary, false, Modifiers::NONE);
    ev(&mut h, Event::Text("Firepixel Text".into()));
    key(&mut h, Key::ArrowLeft, Modifiers::SHIFT);
    key(&mut h, Key::ArrowLeft, Modifiers::SHIFT);
    shot(&mut h, "text");
    key(&mut h, Key::Escape, Modifiers::NONE);
    // Zauberstab: laufende Ameisen am Umriss
    set_tool(&mut h, ToolId::MagicWand);
    let p = at(&h, 420.0, 300.0);
    mv(&mut h, p);
    button(&mut h, p, PointerButton::Primary, true, Modifiers::NONE);
    button(&mut h, p, PointerButton::Primary, false, Modifiers::NONE);
    shot(&mut h, "zauberstab");
    // Pinsel auf der Bitmap mit Spitzen-Vorschau
    let _ = h.state_mut().state.session.execute("select.none", json!({}));
    let _ = h.state_mut().state.session.execute("select.pointAt", json!({"point": [420, 300]}));
    set_tool(&mut h, ToolId::Brush);
    h.state_mut().state.session.tools.options.brush_size = 8.0;
    let (a, b) = (at(&h, 400.0, 250.0), at(&h, 580.0, 340.0));
    mouse_drag(&mut h, a, b, Modifiers::NONE);
    let q = at(&h, 560.0, 260.0);
    mv(&mut h, q);
    shot(&mut h, "pinsel");
    // 9-Slice-Skalierung mit Hilfslinien
    let _ = h.state_mut().state.session.execute("select.pointAt", json!({"point": [100, 60]}));
    set_tool(&mut h, ToolId::NineSliceScale);
    let q = at(&h, 100.0, 100.0);
    mv(&mut h, q);
    shot(&mut h, "9slice");
}

#[test]
fn open_dialog_blocks_canvas_and_export_area_opens_dialog() {
    let mut h = harness();
    // Exportbereich: Rahmen + Eingabe → Export-Dialog mit Bereich
    set_tool(&mut h, ToolId::ExportArea);
    let (a, b) = (at(&h, 20.0, 20.0), at(&h, 120.0, 90.0));
    mouse_drag(&mut h, a, b, Modifiers::NONE);
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.step();
    let st = &h.state().state;
    // der Export-Dialog hat den Bereich übernommen
    let area = st.dialogs.export.as_ref().and_then(|d| d.area);
    assert!(area.is_some_and(|r| r.left == 20.0 && r.bottom == 90.0), "{area:?}");
    assert!(st.canvases.values().next().is_some_and(|c| c.export_area.is_none()), "Bereich nur einmal übernommen");
    assert!(st.dialogs.any_open(), "Export-Dialog offen");
    // solange ein Dialog offen ist, erreicht kein Klick die Leinwand
    set_tool(&mut h, ToolId::Rectangle);
    let (a, b) = (at(&h, 200.0, 150.0), at(&h, 260.0, 200.0));
    mouse_drag(&mut h, a, b, Modifiers::NONE);
    assert!(top(&h).is_empty(), "Klick durch den Dialog auf die Leinwand");
}

#[test]
fn pseudo_tool_keys_and_photoshop_set() {
    let mut h = harness();
    set_tool(&mut h, ToolId::Pointer);
    h.state_mut().state.session.set_colors(Some(fp_color::Argb(0xff12_3456)), Some(fp_color::Argb(0xff65_4321)));
    key(&mut h, Key::X, Modifiers::NONE);
    h.step();
    let t = &h.state().state.session.tools;
    assert_eq!((t.stroke_color.0, t.fill_color.0), (0xff65_4321, 0xff12_3456), "X tauscht");
    key(&mut h, Key::D, Modifiers::NONE);
    h.step();
    let (s0, f0) = (h.state().state.session.tools.stroke_color, h.state().state.session.tools.fill_color);
    assert_ne!((s0.0, f0.0), (0xff65_4321, 0xff12_3456), "D setzt Standardfarben");
    key(&mut h, Key::F, Modifiers::NONE);
    h.step();
    assert_eq!(h.state().state.session.ui.screen_mode, 1, "F schaltet den Bildschirmmodus");
    // zurück zum Standard über den Kontextmenü-Befehl „Vollbildmodus beenden“
    let _ = h.state_mut().state.session.execute("view.exitFullScreen", json!({}));
    assert_eq!(h.state().state.session.ui.screen_mode, 0);
    // Satz „Photoshop“: 2 = Segmente ein/aus, U = Rechteck-Flyout
    let _ = h.state_mut().state.session.execute("edit.keyboardShortcuts", json!({"set": "Photoshop"}));
    let hidden = |h: &Harness<'static, FirepixelApp>| h.state().state.session.active().map(|d| d.view.hide_slices).unwrap_or(false);
    let before = hidden(&h);
    key(&mut h, Key::Num2, Modifiers::NONE);
    h.step();
    assert_ne!(hidden(&h), before, "2 blendet Segmente um");
    key(&mut h, Key::U, Modifiers::NONE);
    h.step();
    assert_eq!(h.state().state.session.tools.active, ToolId::Rectangle);
}

#[test]
fn numpad_shortcuts_web_standard_set() {
    let mut h = harness();
    let _ = h.state_mut().state.session.execute("edit.keyboardShortcuts", json!({"set": "Web Standard"}));
    let st = &h.state().state;
    // Strg+Alt+Num 1–9 = Ausrichten, Strg+Num +/− = Zoom (egui meldet Ziffernblock wie Ziffernreihe)
    let ca = |vk| fp_engine::Shortcut { ctrl: true, alt: true, shift: false, vk };
    let c = |vk| fp_engine::Shortcut { ctrl: true, alt: false, shift: false, vk };
    let found: Vec<Option<String>> = vec![
        crate::input::command_for_key(st, Key::Num1, &ca(0x31)),
        crate::input::command_for_key(st, Key::Plus, &c(0xBB)),
        crate::input::command_for_key(st, Key::Minus, &c(0xBD)),
    ];
    assert!(found.iter().all(|f| f.is_some()), "{found:?}");
    assert!(found[1].as_deref().is_some_and(|n| n.contains("zoom")) && found[2].as_deref().is_some_and(|n| n.contains("zoom")), "{found:?}");
}

fn find_item(nodes: &[crate::menu::Node], label: &str) -> Option<crate::menu::ItemAction> {
    for n in nodes {
        match n {
            crate::menu::Node::Item { label: l, action, .. } if l == label => return Some(action.clone()),
            crate::menu::Node::Sub { children, .. } => {
                if let Some(a) = find_item(children, label) {
                    return Some(a);
                }
            }
            _ => {}
        }
    }
    None
}

#[test]
fn crop_context_menu_commits_and_cancels() {
    let mut h = harness();
    set_tool(&mut h, ToolId::Crop);
    let before = h.state().state.session.active().map(|d| (*d.doc).clone()).expect("doc");
    let (a, b) = (at(&h, 10.0, 10.0), at(&h, 210.0, 160.0));
    mouse_drag(&mut h, a, b, Modifiers::NONE);
    // Rechtsklick im Rahmen: Menü 6910
    let p = at(&h, 100.0, 100.0);
    mv(&mut h, p);
    button(&mut h, p, PointerButton::Secondary, true, Modifiers::NONE);
    button(&mut h, p, PointerButton::Secondary, false, Modifiers::NONE);
    assert_eq!(h.state().state.canvases.values().next().and_then(|c| c.context.as_ref().map(|c| c.id)), Some("6910"));
    let nodes = super::context::build(&h.state().state, super::context::entries("6910"));
    // „Zuschneiden abbrechen“ verwirft den Rahmen, Dokument unverändert
    let Some(crate::menu::ItemAction::Ui(act)) = find_item(&nodes, "Zuschneiden abbrechen") else { panic!("Eintrag fehlt") };
    let ctx = h.ctx.clone();
    super::context::apply(&ctx, &mut h.state_mut().state, act, None);
    assert!(!h.state_mut().state.session.tool_feedback().iter().any(|o| matches!(o, Overlay::Shade { .. })), "Rahmen verworfen");
    assert_eq!(h.state().state.session.active().map(|d| (*d.doc).clone()), Some(before.clone()));
    // wie Esc: das Werkzeug wechselt zum Zeiger (PARITAET_A L28)
    assert_eq!(h.state().state.session.tools.active, ToolId::Pointer);
    // „Dokument zuschneiden“ übernimmt den Rahmen
    set_tool(&mut h, ToolId::Crop);
    mouse_drag(&mut h, a, b, Modifiers::NONE);
    let nodes = super::context::build(&h.state().state, super::context::entries("6910"));
    let Some(crate::menu::ItemAction::Ui(act)) = find_item(&nodes, "Dokument zuschneiden") else { panic!("Eintrag fehlt") };
    super::context::apply(&ctx, &mut h.state_mut().state, act, None);
    let c = h.state().state.session.active().map(|d| (d.doc.pages[0].canvas.width, d.doc.pages[0].canvas.height));
    assert_eq!(c, Some((200.0, 150.0)));
    let _ = h.state_mut().state.session.undo();
    assert_eq!(h.state().state.session.active().map(|d| (*d.doc).clone()), Some(before));
}

#[test]
fn arrow_keys_nudge_through_the_tool() {
    let mut h = harness();
    let _ = h.state_mut().state.session.execute("draw.rectangle", json!({"rect": [50, 50, 150, 120]}));
    set_tool(&mut h, ToolId::Pointer);
    key(&mut h, Key::ArrowRight, Modifiers::SHIFT);
    h.step();
    let d = h.state().state.session.active().expect("d");
    let bnd = fp_engine::docops::element_bounds(&d.doc, &top(&h)[0]).expect("Grenzen");
    assert_eq!(bnd.left, 60.0);
}

/// UX-A L26 mit Mausereignissen über viele Bilder: R1 50,50–150,120 gewählt, Eckgriff rechts
/// unten mit dem Zeiger um +40/+30 ziehen → größer (Original live), Rundung bleibt 0.
#[test]
fn pointer_corner_handle_scales_rectangle_ui() {
    let mut h = harness();
    let _ = h.state_mut().state.session.execute("view.setZoom", json!({"zoom": 1.0}));
    let _ = h.state_mut().state.session.execute("draw.rectangle", json!({"rect": [50, 50, 150, 120]}));
    set_tool(&mut h, ToolId::Pointer);
    h.run_steps(2);
    let a = at(&h, 150.0, 120.0);
    hg_ev(&mut h, Event::PointerMoved(a));
    hg_ev(&mut h, Event::PointerButton { pos: a, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    for i in 1..=10 {
        hg_ev(&mut h, Event::PointerMoved(a + egui::vec2(4.0 * i as f32, 3.0 * i as f32)));
    }
    let b = a + egui::vec2(40.0, 30.0);
    hg_ev(&mut h, Event::PointerButton { pos: b, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
    let d = h.state().state.session.active().expect("d");
    let e = top(&h)[0].clone();
    let r = fp_engine::docops::element_bounds(&d.doc, &e).expect("Grenzen");
    assert_eq!((r.left, r.top, r.right, r.bottom), (50.0, 50.0, 190.0, 150.0));
    let ElementKind::Path(p) = &e.kind else { panic!("kein Pfad") };
    assert_eq!(p.primitive.as_ref().map(|x| x.roundness), Some(0.0));
}

/// UX-A L24 mit getrennten Bildern und echtem Abstand (Klicks ~0,3 s auseinander, Doppelklick
/// ~0,1 s): Stift Klick, Klick, Doppelklick → offener Pfad mit drei Punkten.
#[test]
fn pen_double_click_in_separate_frames_keeps_path() {
    let mut h = harness();
    let _ = h.state_mut().state.session.execute("view.setZoom", json!({"zoom": 1.0}));
    set_tool(&mut h, ToolId::Pen);
    h.run_steps(2);
    let n0 = top(&h).len();
    let (p1, p2, p3) = (at(&h, 250.0, 150.0), at(&h, 330.0, 150.0), at(&h, 330.0, 220.0));
    for p in [p1, p2, p3] {
        hg_click(&mut h, p);
        h.run_steps(15);
    }
    hg_click(&mut h, p3);
    h.run_steps(3);
    assert_eq!(top(&h).len(), n0 + 1, "Pfad übernommen");
    let e = top(&h)[0].clone();
    let ElementKind::Path(pe) = &e.kind else { panic!("kein Pfad") };
    let pts: Vec<(f32, f32)> = pe.contours[0].nodes.iter().map(|n| (n.anchor.x, n.anchor.y)).collect();
    assert_eq!(pts, vec![(250.0, 150.0), (330.0, 150.0), (330.0, 220.0)]);
    assert!(!pe.contours[0].closed);
}

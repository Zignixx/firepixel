//! UX-B: Abgleich Bedienfelder, Dialoge und Fenster mit dem Original (docs\UX_ABGLEICH_B.md).
//!
//! Sonden mit realistischen Eingaben (Tastenereignisse wie egui-winit, Klicks über mehrere Bilder).
//! Jede Sonde druckt `UXB <Nr>: <Beobachtung>`; geprüft wird nur, was im Katalog als „Original“
//! belegt ist. Aufruf: `cargo test -p fp-ui --test ux_b_abgleich -- --nocapture` (die Sonden laufen über
//! [`shell_off`] nacheinander).

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use fp_engine::Session;
use fp_ui::app_state::{Action, UiSettings};
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn session() -> Session {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 500, "height": 400})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 100]})).expect("rect");
    s.execute("draw.ellipse", json!({"rect": [200, 50, 300, 150]})).expect("ellipse");
    s.execute("layer.add", json!({"name": "Ebene Zwei"})).expect("layer");
    s.execute("draw.rectangle", json!({"rect": [50, 200, 250, 300]})).expect("rect2");
    s
}

fn harness_with(s: Session) -> H {
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.sync_render = true;
    let tmp = std::env::temp_dir().join("fp-uxb");
    let _ = std::fs::create_dir_all(&tmp);
    a.state.file_override = Some(fp_ui::files::FilePickOverride { open: vec![], save: Some(tmp.join("uxb.fw.png")) });
    let mut h = Harness::builder().with_size(egui::vec2(1600.0, 1000.0)).with_pixels_per_point(1.0).with_step_dt(1.0 / 60.0).with_max_steps(20).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a);
    h.run_steps(4);
    h
}

fn harness() -> H {
    harness_with(session())
}

fn ev(h: &mut H, e: Event) {
    let m = match &e {
        Event::Key { modifiers, .. } | Event::PointerButton { modifiers, .. } | Event::MouseWheel { modifiers, .. } => *modifiers,
        _ => Modifiers::NONE,
    };
    if m == Modifiers::NONE {
        h.event(e);
    } else {
        h.event_modifiers(e, m);
    }
    h.run_steps(1);
}

fn key(h: &mut H, k: Key, m: Modifiers) {
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: m });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: m });
}

fn typ(h: &mut H, s: &str) {
    for ch in s.chars() {
        ev(h, Event::Text(ch.to_string()));
    }
}

fn ctrl() -> Modifiers {
    Modifiers { ctrl: true, command: true, ..Default::default() }
}

fn click(h: &mut H, p: Pos2, m: Modifiers) {
    ev(h, Event::PointerMoved(p));
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: m });
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: m });
    h.run_steps(2);
}

fn dclick(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    for _ in 0..2 {
        ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
        ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    }
    h.run_steps(2);
}

fn shot(h: &mut H, name: &str) {
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/screenshots");
    let _ = std::fs::create_dir_all(&out);
    match h.render() {
        Ok(img) => {
            let _ = img.save(out.join(format!("phase5_uxb_{name}.png")));
        }
        Err(e) => println!("UXB Bild {name}: kein Rendern ({e})"),
    }
}

fn focused(h: &H) -> bool {
    h.ctx.memory(|m| m.focused().is_some())
}

/// Die Tests dieser Datei laufen nacheinander (NZ-2). Grund: Jede Sonde macht Bilder
/// (`shot` → `Harness::render`), und dabei legt egui_kittest für jeden Harness einen eigenen
/// wgpu-Renderer mit eigener Instanz und eigenem Gerät an (beim ersten Bild, abgebaut mit dem
/// Harness). Starten neun Sonden das gleichzeitig, stürzt der Prozess gelegentlich mit einer
/// Zugriffsverletzung ab: 6 von 30 Läufen, ohne Bilder 0 von 30 (gemessen 12.10.2026). Der
/// Wächter wird zuerst angelegt und zuletzt freigegeben, er umschließt also auch den Abbau des
/// Renderers. Damit braucht der Aufruf kein `--test-threads 1` mehr.
static GPU: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[must_use]
fn shell_off() -> std::sync::MutexGuard<'static, ()> {
    let g = GPU.lock().unwrap_or_else(|e| e.into_inner());
    fp_engine::shell::disable();
    g
}

#[test]
fn uxb_d_neues_dokument() {
    let _gpu = shell_off();
    let mut h = harness();
    h.state_mut().state.run("file.new", json!({}));
    h.run_steps(4);
    println!("UXB D01 Neues Dokument offen: {}", h.state().state.dialogs.new_document.is_some());
    println!("UXB D02 Fokus beim Öffnen gesetzt: {}", focused(&h));
    shot(&mut h, "neues_dokument");
    // sofort tippen wie im Original (Breite markiert): 800 Tab 600 Enter
    typ(&mut h, "800");
    key(&mut h, Key::Tab, Modifiers::NONE);
    typ(&mut h, "600");
    let d = h.state().state.dialogs.new_document.clone();
    println!("UXB D03 nach Tippen 800⇥600: Breite='{}' Höhe='{}'", d.as_ref().map(|d| d.width.clone()).unwrap_or_default(), d.as_ref().map(|d| d.height.clone()).unwrap_or_default());
    let n0 = h.state().state.session.documents().len();
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(3);
    let n1 = h.state().state.session.documents().len();
    let size = h.state().state.session.active().and_then(|d| d.doc.pages.get(d.page).map(|p| (p.canvas.width, p.canvas.height)));
    println!("UXB D04 Enter = OK: Dokumente {n0}→{n1}, Größe {size:?}, Dialog zu: {}", h.state().state.dialogs.new_document.is_none());
    // gemerkte Werte
    h.state_mut().state.run("file.new", json!({}));
    h.run_steps(3);
    let d = h.state().state.dialogs.new_document.clone();
    println!("UXB D05 erneut geöffnet: Breite='{}' Höhe='{}'", d.as_ref().map(|d| d.width.clone()).unwrap_or_default(), d.as_ref().map(|d| d.height.clone()).unwrap_or_default());
    key(&mut h, Key::Escape, Modifiers::NONE);
    h.run_steps(2);
    println!("UXB D06 Esc schließt: {}", h.state().state.dialogs.new_document.is_none());
    println!("UXB D07 Dokumente nach Esc unverändert: {}", h.state().state.session.documents().len() == n1);
}

#[test]
fn uxb_m_ungespeichert() {
    let _gpu = shell_off();
    let mut h = harness();
    let id = h.state().state.session.active_id().expect("doc");
    h.state_mut().state.push(Action::CloseDocument(id));
    h.run_steps(4);
    println!("UXB M01 Abfrage offen: {}", h.state().state.dialogs.unsaved.is_some());
    let labels: Vec<String> = h.query_all_by_label_contains("speichern").map(|n| format!("{n:?}")).collect();
    println!("UXB M02 Text: {labels:?}");
    shot(&mut h, "ungespeichert");
    key(&mut h, Key::Escape, Modifiers::NONE);
    h.run_steps(2);
    println!("UXB M03 Esc = Abbrechen: offen={} Dokument da={}", h.state().state.dialogs.unsaved.is_some(), h.state().state.session.documents().len());
}

#[test]
fn uxb_w_reiter_und_tasten() {
    let _gpu = shell_off();
    let mut s = session();
    s.execute("file.new", json!({"width": 300, "height": 200})).expect("doc2");
    let mut h = harness_with(s);
    let ids: Vec<_> = h.state().state.session.documents().iter().map(|d| d.id).collect();
    let a0 = h.state().state.session.active_id();
    key(&mut h, Key::Tab, ctrl());
    h.run_steps(2);
    let a1 = h.state().state.session.active_id();
    println!("UXB W01 Strg+Tab wechselt Dokument: {} ({a0:?}→{a1:?}, {} Dokumente)", a0 != a1, ids.len());
    println!("UXB W02 Fenstertitel: '{}'", h.state().state.window_title());
    shot(&mut h, "zwei_dokumente");
    // Alt+D (Menü Datei) und F10
    key(&mut h, Key::D, Modifiers { alt: true, ..Default::default() });
    h.run_steps(2);
    let open = egui::Popup::is_id_open(&h.ctx, egui::Id::new(("fp-main-menu", 0usize)));
    println!("UXB ME01 Alt+D öffnet Datei-Menü: {open}");
    shot(&mut h, "alt_d");
    key(&mut h, Key::Escape, Modifiers::NONE);
    key(&mut h, Key::F10, Modifiers::NONE);
    h.run_steps(2);
    let any = (0..12usize).any(|i| egui::Popup::is_id_open(&h.ctx, egui::Id::new(("fp-main-menu", i))));
    println!("UXB ME02 F10 aktiviert Menüleiste (Menü offen): {any}");
}

/// Dialoge: Fokus beim Öffnen, Lage (zentriert?), Enter = Standardknopf, Esc = Abbrechen.
#[test]
fn uxb_d_dialoge_grundverhalten() {
    let _gpu = shell_off();
    let cmds = [
        "modify.canvas.imageSize",
        "modify.canvas.canvasSize",
        "modify.transform.numericTransform",
        "filter.blur.gaussian",
        "view.grid.editGrid",
        "file.export",
        "file.imagePreview",
        "edit.preferences",
        "edit.keyboardShortcuts",
        "filter.adjustColor.hueSaturation",
        "select.feather",
        "panel.layers.newLayer",
    ];
    for c in cmds {
        let mut h = harness();
        h.state_mut().state.run("select.all", json!({}));
        h.run_steps(2);
        h.state_mut().state.run(c, json!({}));
        h.run_steps(4);
        let open = h.state().state.dialogs.any_open();
        let id = fp_ui::dialogs::last_dialog_id(&h.ctx).unwrap_or_default();
        let r = fp_ui::dialogs::dialog_rect(&h.ctx, &id);
        let screen = h.ctx.content_rect();
        let centered = r.map(|r| (r.center() - screen.center()).length() < 3.0);
        let inputs: Vec<String> = h.query_all_by_role(egui::accesskit::Role::TextInput).map(|n| format!("{:?}", n.is_focused())).collect();
        println!("UXB DLG {c}: offen={open} id={id} zentriert={centered:?} Fokus={} Textfelder={} fokussiert={:?}", focused(&h), inputs.len(), inputs.iter().position(|s| s == "true"));
        let hist0 = h.state().state.session.active().map(|d| d.history.undo_count()).unwrap_or(0);
        key(&mut h, Key::Escape, Modifiers::NONE);
        h.run_steps(3);
        let hist1 = h.state().state.session.active().map(|d| d.history.undo_count()).unwrap_or(0);
        println!("UXB DLG {c}: Esc schließt={} Verlauf {hist0}→{hist1}", !h.state().state.dialogs.any_open());
    }
}

/// Bestätigungsdialog (Rückfrage) mit Enter.
#[test]
fn uxb_m_rueckfrage_enter() {
    let _gpu = shell_off();
    let mut h = harness();
    h.state_mut().state.dialogs.confirm = Some(fp_ui::dialogs::Confirm { title: "Firepixel".into(), text: "Testfrage".into(), command: "select.all".into(), params: json!({}) });
    h.run_steps(3);
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(3);
    println!("UXB M10 Rückfrage: Enter wählt Standardknopf (Dialog zu): {}", h.state().state.dialogs.confirm.is_none());
    shot(&mut h, "rueckfrage");
}

/// Neues Dokument merkt die zuletzt benutzte Größe.
#[test]
fn uxb_d_neues_dokument_merken() {
    let _gpu = shell_off();
    let mut h = harness();
    h.state_mut().state.run("file.new", json!({}));
    h.run_steps(3);
    if let Some(d) = h.state_mut().state.dialogs.new_document.as_mut() {
        d.width = "321".into();
        d.height = "123".into();
    }
    h.run_steps(1);
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(3);
    h.state_mut().state.run("file.new", json!({}));
    h.run_steps(3);
    let d = h.state().state.dialogs.new_document.clone();
    println!("UXB D05b nach 321×123 erneut: Breite='{}' Höhe='{}'", d.as_ref().map(|d| d.width.clone()).unwrap_or_default(), d.as_ref().map(|d| d.height.clone()).unwrap_or_default());
}

/// Ebenen-Bedienfeld sichtbar machen und fotografieren (Koordinaten für weitere Sonden).
#[test]
fn uxb_e_ebenen_bild() {
    let _gpu = shell_off();
    let mut h = harness();
    click(&mut h, Pos2::new(1442.0, 384.0), Modifiers::NONE);
    h.state_mut().state.run("select.set", json!({"ids": []}));
    h.run_steps(3);
    shot(&mut h, "ebenen_start");
    h.state_mut().state.run("select.all", json!({}));
    h.run_steps(3);
    shot(&mut h, "auswahl_alle");
    println!("UXB E00 Bilder gespeichert");
}

fn layer_names(h: &H) -> Vec<String> {
    let s = &h.state().state;
    let d = s.session.active().expect("doc");
    d.doc.resolved_layers(d.page).iter().map(|l| l.layer.name.clone()).collect()
}

fn layers_harness() -> H {
    let mut h = harness();
    click(&mut h, Pos2::new(1442.0, 384.0), Modifiers::NONE);
    h.run_steps(3);
    h
}

fn focused_value(h: &H) -> Option<String> {
    h.query_all_by_role(egui::accesskit::Role::TextInput).find(|n| n.is_focused()).and_then(|n| n.value())
}

#[test]
fn uxb_e_ebenen_umbenennen() {
    let _gpu = shell_off();
    // E01: Doppelklick auf den Namen, sofort tippen, Enter
    let mut h = layers_harness();
    dclick(&mut h, Pos2::new(1430.0, 516.0));
    println!("UXB E01a Umbenennen-Feld offen, Fokuswert: {:?}", focused_value(&h));
    shot(&mut h, "ebene_umbenennen");
    typ(&mut h, "Neu");
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(3);
    println!("UXB E01 Doppelklick+„Neu“+Enter → Ebenen: {:?}", layer_names(&h));
    // E02: Esc bricht ab
    let mut h = layers_harness();
    dclick(&mut h, Pos2::new(1430.0, 516.0));
    typ(&mut h, "XYZ");
    key(&mut h, Key::Escape, Modifiers::NONE);
    h.run_steps(3);
    println!("UXB E02 Esc → Ebenen: {:?}", layer_names(&h));
    // E03: Klick daneben übernimmt
    let mut h = layers_harness();
    dclick(&mut h, Pos2::new(1430.0, 516.0));
    typ(&mut h, "Q");
    click(&mut h, Pos2::new(700.0, 750.0), Modifiers::NONE);
    h.run_steps(3);
    println!("UXB E03 Klick daneben → Ebenen: {:?}", layer_names(&h));
    // E04: Objekt umbenennen
    let mut h = layers_harness();
    dclick(&mut h, Pos2::new(1462.0, 573.0));
    typ(&mut h, "Kasten");
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(3);
    let names: Vec<String> = {
        let s = &h.state().state;
        let d = s.session.active().expect("doc");
        d.doc.resolved_layers(d.page).iter().flat_map(|l| l.layer.cel(d.frame).map(|c| c.elements.iter().map(|e| e.common.name.clone().unwrap_or_default()).collect::<Vec<_>>()).unwrap_or_default()).collect()
    };
    println!("UXB E04 Objekt-Doppelklick+„Kasten“ → Namen: {names:?}");
}

#[test]
fn uxb_e_ebenen_klicks() {
    let _gpu = shell_off();
    let vis = |h: &H| -> Vec<(String, bool)> {
        let s = &h.state().state;
        let d = s.session.active().expect("doc");
        d.doc.resolved_layers(d.page).iter().map(|l| (l.layer.name.clone(), l.layer.is_visible())).collect()
    };
    // E10: Alt+Klick auf das Auge von „Ebene 1“
    let mut h = layers_harness();
    click(&mut h, Pos2::new(1340.0, 516.0), Modifiers { alt: true, ..Default::default() });
    println!("UXB E10 Alt+Klick Auge Ebene 1 → {:?}", vis(&h));
    // E11: Ziehen über die Augenspalte (Ebene Zwei → Ebene 1)
    let mut h = layers_harness();
    let a = Pos2::new(1340.0, 463.0);
    let b = Pos2::new(1340.0, 516.0);
    ev(&mut h, Event::PointerMoved(a));
    ev(&mut h, Event::PointerButton { pos: a, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    for i in 1..=10 {
        ev(&mut h, Event::PointerMoved(a + (b - a) * (i as f32 / 10.0)));
    }
    ev(&mut h, Event::PointerButton { pos: b, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(3);
    println!("UXB E11 Ziehen über Augen → {:?}", vis(&h));
    // E12: Rechtsklick auf eine Ebene → Kontextmenü?
    let mut h = layers_harness();
    let p = Pos2::new(1430.0, 516.0);
    ev(&mut h, Event::PointerMoved(p));
    ev(&mut h, Event::PointerButton { pos: p, button: PointerButton::Secondary, pressed: true, modifiers: Modifiers::NONE });
    ev(&mut h, Event::PointerButton { pos: p, button: PointerButton::Secondary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(3);
    shot(&mut h, "ebene_rechtsklick");
    let n = h.query_all_by_role(egui::accesskit::Role::MenuItem).count() + h.query_all_by_label_contains("Ebene löschen").count();
    println!("UXB E12 Rechtsklick Ebene → Menüeinträge sichtbar: {n}");
    // E13: Klick auf Ebene „Ebene 1“: Auswahl / aktive Ebene
    let mut h = layers_harness();
    h.state_mut().state.run("select.all", json!({}));
    h.run_steps(3);
    click(&mut h, Pos2::new(1430.0, 516.0), Modifiers::NONE);
    let (sel, act) = {
        let d = h.state().state.session.active().expect("doc");
        (d.selection.elements.len(), d.layer.and_then(|l| d.doc.resolved_layers(d.page).iter().find(|x| x.layer.id == l).map(|x| x.layer.name.clone())))
    };
    println!("UXB E13 Klick auf Ebene 1 → Auswahl {sel} Objekte, aktiv {act:?}");
    // E14: Umschalt+Klick auf zwei Objekte
    let mut h = layers_harness();
    click(&mut h, Pos2::new(1462.0, 543.0), Modifiers::NONE);
    click(&mut h, Pos2::new(1462.0, 490.0), Modifiers { shift: true, ..Default::default() });
    let sel = h.state().state.session.active().map(|d| d.selection.elements.len()).unwrap_or(0);
    println!("UXB E14 Klick + Umschalt+Klick → {sel} Objekte");
    // E15: Ebene 1 über Ebene Zwei ziehen
    let mut h = layers_harness();
    let a = Pos2::new(1430.0, 516.0);
    let b = Pos2::new(1430.0, 458.0);
    ev(&mut h, Event::PointerMoved(a));
    ev(&mut h, Event::PointerButton { pos: a, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    for i in 1..=12 {
        ev(&mut h, Event::PointerMoved(a + (b - a) * (i as f32 / 12.0)));
    }
    ev(&mut h, Event::PointerButton { pos: b, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(3);
    println!("UXB E15 Ebene 1 nach oben ziehen → {:?}", layer_names(&h));
    // E16: Entf bei Fokus im Bedienfeld (Objekt gewählt) löscht Objekt
    let mut h = layers_harness();
    click(&mut h, Pos2::new(1462.0, 543.0), Modifiers::NONE);
    let n0 = h.state().state.session.active().map(|d| d.doc.resolved_layers(d.page).iter().map(|l| l.layer.cel(d.frame).map(|c| c.elements.len()).unwrap_or(0)).sum::<usize>()).unwrap_or(0);
    key(&mut h, Key::Delete, Modifiers::NONE);
    h.run_steps(3);
    let n1 = h.state().state.session.active().map(|d| d.doc.resolved_layers(d.page).iter().map(|l| l.layer.cel(d.frame).map(|c| c.elements.len()).unwrap_or(0)).sum::<usize>()).unwrap_or(0);
    println!("UXB E16 Entf nach Klick im Bedienfeld → Objekte {n0}→{n1}");
}

#[test]
fn uxb_i_zahlenfelder() {
    let _gpu = shell_off();
    let w = Pos2::new(124.0, 953.0);
    let bounds = |h: &H| {
        let d = h.state().state.session.active().expect("doc");
        let id = d.selection.elements[0];
        fp_engine::docops::ids_bounds(&d.doc, d.page, &[id])
    };
    let sel_rect = |h: &mut H| {
        let id = {
            let d = h.state().state.session.active().expect("doc");
            d.doc.resolved_layers(d.page).iter().find_map(|l| l.layer.cel(d.frame).and_then(|c| c.elements.first().map(|e| e.id)))
        };
        h.state_mut().state.run("select.set", json!({"ids": [id.map(|i| i.0)]}));
        h.run_steps(3);
    };
    // I01: Klick ins B-Feld, tippen, Enter
    let mut h = harness();
    sel_rect(&mut h);
    shot(&mut h, "inspektor_rechteck");
    let b0 = bounds(&h);
    click(&mut h, w, Modifiers::NONE);
    println!("UXB I01a nach Klick ins B-Feld: Fokuswert {:?}", focused_value(&h));
    typ(&mut h, "150");
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(3);
    println!("UXB I01 Klick+„150“+Enter: {b0:?} → {:?}; Fokus danach {:?}", bounds(&h), focused_value(&h));
    // I02: Doppelklick markiert alles?
    let mut h = harness();
    sel_rect(&mut h);
    dclick(&mut h, w);
    typ(&mut h, "150");
    key(&mut h, Key::Tab, Modifiers::NONE);
    h.run_steps(2);
    println!("UXB I02 Doppelklick+„150“+Tab: → {:?}; Tab-Ziel-Feldwert {:?}", bounds(&h), focused_value(&h));
    // I03: Esc verwirft
    let mut h = harness();
    sel_rect(&mut h);
    dclick(&mut h, w);
    typ(&mut h, "33");
    key(&mut h, Key::Escape, Modifiers::NONE);
    h.run_steps(2);
    println!("UXB I03 Esc: {:?}", bounds(&h));
    // I04: Mausrad über dem Feld
    let mut h = harness();
    sel_rect(&mut h);
    let b0 = bounds(&h);
    ev(&mut h, Event::PointerMoved(w));
    ev(&mut h, Event::MouseWheel { unit: egui::MouseWheelUnit::Line, delta: egui::vec2(0.0, 1.0), phase: egui::TouchPhase::Move, modifiers: Modifiers::NONE });
    h.run_steps(2);
    println!("UXB I04 Mausrad über B-Feld: {b0:?} → {:?}", bounds(&h));
    // I05: Pfeil hoch im Feld
    let mut h = harness();
    sel_rect(&mut h);
    dclick(&mut h, w);
    key(&mut h, Key::ArrowUp, Modifiers::NONE);
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(2);
    println!("UXB I05 Pfeil hoch + Enter: → {:?}", bounds(&h));
    // I06: Deckkraft 150 eintippen (Klemmen)
    let mut h = harness();
    sel_rect(&mut h);
    dclick(&mut h, Pos2::new(621.0, 901.0));
    typ(&mut h, "150");
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(2);
    let op = {
        let d = h.state().state.session.active().expect("doc");
        let id = d.selection.elements[0];
        fp_engine::docops::element_on_page(&d.doc, d.page, id).map(|e| e.common.opacity.to_percent())
    };
    println!("UXB I06 Deckkraft „150“+Enter → {op:?}");
}

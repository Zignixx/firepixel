//! DLG-7 (P-DLG7 „Pfad und Transformieren“, Messung PM-6, PARITAET_B D26/D29–D32):
//! - D26 Numerisch transformieren: ↔/↕-Symbole statt „Breite:/Höhe:“, Objektgröße „B: … / H: …“,
//!   Kette; das zweite Feld rechnet beim Tippen mit; Pixelmaße bei „Größe ändern“ gemerkt
//! - D29–D31 Beschriftungen und Anordnung („Menge“, „Winkelgrenze“ 10.00, „Endkappen“, Richtung
//!   untereinander, Symbolknöpfe als ein Tab-Stopp)
//! - D32 Pfad in Auswahlrahmen konvertieren: neuer Dialog (Rand Hart/Anti-Alias/Weiche Kante +
//!   Breite), Wirkung, Abbrechen
//!
//! Realistische Eingaben: Klick ins Feld, Strg+A, Tippen wie egui-winit (Taste + Text), Tab; OK und
//! Abbrechen per Mausklick, Esc als Taste.

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::{dialog_rect, last_dialog_id};
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

/// Dokument 300 × 200 mit dem Rechteck R1 (20,20,120,100) und dem Pfad P1 (Dreieck), R1 ausgewählt.
fn session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    let node = |x: f64, y: f64| json!({"anchor": [x, y], "in": [x, y], "out": [x, y]});
    s.execute("draw.path", json!({"contours": [{"closed": true, "nodes": [node(160.0, 40.0), node(260.0, 40.0), node(210.0, 140.0)]}], "fillColor": "#cc3333", "name": "P1"})).expect("Pfad");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 100], "fillColor": "#3366cc", "name": "R1"})).expect("rect");
    s
}

fn harness() -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, session(), UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
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

fn ctrl() -> Modifiers {
    Modifiers { ctrl: true, command: true, ..Default::default() }
}

/// Tippen wie egui-winit: Taste drücken + Text, Taste loslassen.
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
    h.run_steps(2);
}

fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
}

fn select_named(h: &mut H, name: &str) -> fp_model::ElementId {
    let id = {
        let d = st(h).session.active().expect("doc");
        fp_engine::docops::selectable_ids(&d.doc, d.page, d.frame).into_iter().find(|id| fp_engine::docops::element_on_page(&d.doc, d.page, *id).is_some_and(|e| e.common.name.as_deref() == Some(name))).expect(name)
    };
    h.state_mut().state.session.execute("select.set", json!({"ids": [id.0]})).expect("Auswahl");
    h.run_steps(2);
    id
}

fn open(h: &mut H, cmd: &str) {
    h.state_mut().state.run(cmd, json!({}));
    h.run_steps(4);
    assert!(st(h).dialogs.any_open(), "{cmd}: Dialog offen");
}

fn dlg_rect(h: &H) -> Rect {
    dialog_rect(&h.ctx, &last_dialog_id(&h.ctx).expect("Dialog")).expect("Rechteck")
}

/// Textfelder des Dialogs von oben nach unten (Rechteck, Text, gesperrt).
fn text_fields(h: &H) -> Vec<(Rect, String, bool)> {
    let dr = dlg_rect(h);
    let mut v: Vec<(Rect, String, bool)> = h.get_all(By::new().role(Role::TextInput)).map(|n| (n.rect(), n.value().unwrap_or_default(), n.accesskit_node().is_disabled())).filter(|(r, _, _)| dr.contains(r.center())).collect();
    v.sort_by(|a, b| a.0.top().total_cmp(&b.0.top()));
    v
}

fn button(h: &H, label: &str) -> Rect {
    let dr = dlg_rect(h);
    h.query_all(By::new().role(Role::Button).label(label)).map(|n| n.rect()).find(|r| dr.contains(r.center())).unwrap_or_else(|| panic!("Knopf {label}"))
}

/// Gezeichnete Beschriftungen im Dialog (Text, Rechteck).
fn labels(h: &mut H) -> Vec<(String, Rect)> {
    let dr = dlg_rect(h);
    fp_ui::inspector::fields::set_audit(&h.ctx, true);
    h.run_steps(1);
    let v = fp_ui::inspector::fields::take_audit(&h.ctx).into_iter().filter(|a| dr.contains(a.drawn.center())).map(|a| (a.text, a.drawn)).collect();
    fp_ui::inspector::fields::set_audit(&h.ctx, false);
    v
}

fn label_texts(h: &mut H) -> Vec<String> {
    labels(h).into_iter().map(|(t, _)| t).collect()
}

/// Feld `i` (von oben) anklicken, alles markieren und tippen (ohne Tab).
fn type_into(h: &mut H, i: usize, text: &str) {
    let r = text_fields(h)[i].0;
    click(h, r.center());
    key_m(h, Key::A, ctrl());
    typ(h, text);
}

fn bounds_of(h: &H, id: fp_model::ElementId) -> (f64, f64, f64, f64) {
    let d = st(h).session.active().expect("doc");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("Element");
    let b = fp_engine::docops::element_bounds(&d.doc, &e).expect("Grenzen");
    (b.left.round() as f64, b.top.round() as f64, b.right.round() as f64, b.bottom.round() as f64)
}

fn undo_count(h: &H) -> usize {
    st(h).session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

/// D26: Symbole statt „Breite:/Höhe:“, Objektgröße B/H rechts, beim Tippen rechnet das zweite Feld
/// und die Objektgröße sofort mit (ohne Tab); OK skaliert R1 auf 45,40,95,80, Strg+Z stellt her.
#[test]
fn numeric_transform_object_size_and_live_proportions() {
    let mut h = harness();
    let r1 = select_named(&mut h, "R1");
    open(&mut h, "modify.transform.numericTransform");
    let l = label_texts(&mut h);
    assert!(l.iter().any(|x| x == "B: 100") && l.iter().any(|x| x == "H: 80"), "Objektgröße: {l:?}");
    assert!(!l.iter().any(|x| x == "Breite:" || x == "Höhe:"), "Symbole statt Beschriftung: {l:?}");
    // Name und Tooltip der Symbole bleiben „Breite:“/„Höhe:“
    let dr = dlg_rect(&h);
    assert!(h.query_all(By::new().label("Breite:")).any(|n| dr.contains(n.rect().center())), "Symbol ↔ benannt");
    type_into(&mut h, 0, "50");
    {
        let d = st(&h).dialogs.numeric_transform.as_ref().expect("Dialog");
        assert_eq!((d.sx, d.sy), (50.0, 50.0), "H folgt beim Tippen");
    }
    assert_eq!(text_fields(&h)[1].1, "50", "zweites Feld zeigt 50 sofort");
    let l = label_texts(&mut h);
    assert!(l.iter().any(|x| x == "B: 50") && l.iter().any(|x| x == "H: 40"), "Objektgröße folgt: {l:?}");
    let n0 = undo_count(&h);
    key(&mut h, Key::Enter);
    h.run_steps(4);
    assert!(st(&h).dialogs.numeric_transform.is_none(), "Enter = OK");
    assert_eq!(bounds_of(&h, r1), (45.0, 40.0, 95.0, 80.0), "Skalieren 50 %");
    assert_eq!(undo_count(&h), n0 + 1, "ein Verlaufsschritt");
    h.state_mut().state.session.undo().expect("Rückgängig");
    h.run_steps(2);
    assert_eq!(bounds_of(&h, r1), (20.0, 20.0, 120.0, 100.0), "Strg+Z stellt her");
}

/// D26 „Größe ändern“: Breite 200 tippen → Höhe 160 sofort; OK wirkt; die Pixelmaße sind beim
/// nächsten Öffnen gemerkt, Abbrechen merkt nichts. Ohne Proportionen folgt nichts.
#[test]
fn numeric_transform_resize_follows_and_remembers_pixels() {
    let mut h = harness();
    let r1 = select_named(&mut h, "R1");
    open(&mut h, "modify.transform.numericTransform");
    h.state_mut().state.dialogs.numeric_transform.as_mut().expect("Dialog").mode = 1;
    h.run_steps(3);
    assert_eq!(text_fields(&h)[0].1, "100");
    type_into(&mut h, 0, "200");
    {
        let d = st(&h).dialogs.numeric_transform.as_ref().expect("Dialog");
        assert_eq!((d.w, d.h), (200.0, 160.0), "Höhe folgt beim Tippen");
    }
    assert_eq!(text_fields(&h)[1].1, "160");
    assert!(label_texts(&mut h).iter().any(|x| x == "B: 200"), "Objektgröße folgt");
    let b = button(&h, "OK").center();
    click(&mut h, b);
    h.run_steps(4);
    assert!(st(&h).dialogs.numeric_transform.is_none(), "OK schließt");
    assert_eq!(bounds_of(&h, r1), (-30.0, -20.0, 170.0, 140.0), "Größe 200 × 160 um die Mitte");
    open(&mut h, "modify.transform.numericTransform");
    {
        let d = st(&h).dialogs.numeric_transform.as_ref().expect("Dialog");
        assert_eq!((d.mode, d.w, d.h), (1, 200.0, 160.0), "Pixelmaße gemerkt");
    }
    // ohne Proportionen bleibt das andere Feld stehen; Abbrechen merkt nichts
    h.state_mut().state.dialogs.numeric_transform.as_mut().expect("Dialog").constrain = false;
    h.run_steps(2);
    type_into(&mut h, 1, "50");
    {
        let d = st(&h).dialogs.numeric_transform.as_ref().expect("Dialog");
        assert_eq!((d.w, d.h), (200.0, 50.0), "ohne Kette folgt nichts");
    }
    let b = button(&h, "Abbrechen").center();
    click(&mut h, b);
    h.run_steps(4);
    assert!(st(&h).dialogs.numeric_transform.is_none());
    open(&mut h, "modify.transform.numericTransform");
    {
        let d = st(&h).dialogs.numeric_transform.as_ref().expect("Dialog");
        assert_eq!((d.w, d.h), (200.0, 160.0), "Abbrechen merkt nichts");
    }
    key(&mut h, Key::Escape);
    h.run_steps(4);
}

/// D29–D31: Beschriftungen wie im Original, Winkelgrenze „10.00“, Richtung untereinander,
/// Beschriftungen rechtsbündig vor den Feldern; englische Texte.
#[test]
fn path_dialog_labels_like_original() {
    let mut h = harness();
    select_named(&mut h, "R1");
    for (cmd, want, not) in [
        ("modify.alterPath.simplify", &["Menge:"][..], &["Betrag:"][..]),
        ("modify.alterPath.expandStroke", &["Breite:", "Ecken:", "Winkelgrenze:", "Endkappen:"][..], &["Gehrungsgrenze:", "Enden:"][..]),
        ("modify.alterPath.insetPath", &["Richtung:", "Innen", "Außen", "Breite:", "Ecken:", "Winkelgrenze:"][..], &["Gehrungsgrenze:"][..]),
    ] {
        open(&mut h, cmd);
        let l = label_texts(&mut h);
        for w in want {
            assert!(l.iter().any(|x| x == w), "{cmd}: „{w}“ fehlt in {l:?}");
        }
        for n in not {
            assert!(!l.iter().any(|x| x == n), "{cmd}: „{n}“ noch da: {l:?}");
        }
        if cmd != "modify.alterPath.simplify" {
            assert!(text_fields(&h).iter().any(|f| f.1 == "10.00"), "{cmd}: Winkelgrenze 10.00");
        }
        if cmd == "modify.alterPath.insetPath" {
            let lr = labels(&mut h);
            let y = |t: &str| lr.iter().find(|(x, _)| x == t).map(|(_, r)| r.center()).expect(t);
            assert!(y("Außen").y > y("Innen").y + 10.0, "Richtung untereinander");
            let left = |t: &str| lr.iter().find(|(x, _)| x == t).map(|(_, r)| r.left()).expect(t);
            assert!((left("Außen") - left("Innen")).abs() < 2.0, "Innen/Außen bündig");
            // Beschriftungen rechtsbündig: rechte Kanten gleich
            let right = |t: &str| lr.iter().find(|(x, _)| x == t).map(|(_, r)| r.right()).expect(t);
            assert!((right("Breite:") - right("Winkelgrenze:")).abs() < 2.0, "rechtsbündig");
        }
        key(&mut h, Key::Escape);
        h.run_steps(4);
    }
    fp_i18n::set_language("en");
    assert_eq!(fp_i18n::t("dlg-lbl-caps"), "End caps:");
    assert_eq!(fp_i18n::t("dlg-title-path-to-marquee"), "Convert Path to Marquee");
    fp_i18n::set_language("de");
}

/// D30: Tab-Folge Breite › Ecken (ein Stopp) › Winkelgrenze; Enter mit Breite 5 erweitert den
/// Strich (ein Verlaufsschritt), Winkelgrenze bleibt 10.
#[test]
fn expand_stroke_tab_order_and_effect() {
    let mut h = harness();
    select_named(&mut h, "R1");
    open(&mut h, "modify.alterPath.expandStroke");
    assert!(h.ctx.text_edit_focused(), "Startfokus Breite");
    typ(&mut h, "5");
    // bis zur Winkelgrenze: ▾ des Breitenfelds (heute ein eigener Stopp, siehe Bericht) und Ecken
    // als **ein** Stopp; einzeln wären es fünf Tabs
    let mut tabs = 0;
    loop {
        key(&mut h, Key::Tab);
        h.run_steps(2);
        tabs += 1;
        if h.ctx.text_edit_focused() || tabs > 6 {
            break;
        }
    }
    assert!(tabs <= 3, "Ecken nur ein Tab-Stopp ({tabs} Tabs bis zur Winkelgrenze)");
    let f = st(&h).dialogs.form.as_ref().expect("Formular");
    assert_eq!(f.get("width").as_f64(), Some(5.0));
    assert_eq!(f.get("miter").as_f64(), Some(10.0));
    let n0 = undo_count(&h);
    key(&mut h, Key::Enter);
    h.run_steps(4);
    assert!(!st(&h).dialogs.any_open(), "Enter = OK");
    assert_eq!(undo_count(&h), n0 + 1, "ein Verlaufsschritt");
}

/// D32: Der Befehl öffnet jetzt einen Dialog („Rand:“ Anti-Alias vorgewählt, Breite gesperrt,
/// Startfokus OK). Abbrechen ändert nichts; „Weiche Kante“ 5 + OK → weiche Pixelauswahl, Pfad
/// gelöscht, ein Verlaufsschritt; Strg+Z stellt her; Werte gemerkt.
#[test]
fn path_to_marquee_dialog_opens_applies_and_cancels() {
    let mut h = harness();
    let p1 = select_named(&mut h, "P1");
    let n0 = undo_count(&h);
    open(&mut h, "modify.convertPathToMarquee");
    {
        let f = st(&h).dialogs.form.as_ref().expect("Formular D32");
        assert_eq!(f.command, "modify.convertPathToMarquee");
        assert_eq!(f.title, "Pfad in Auswahlrahmen konvertieren");
        assert_eq!(f.get("edge"), json!("antiAlias"), "Vorgabe Anti-Alias");
    }
    let l = label_texts(&mut h);
    assert!(l.iter().any(|x| x == "Rand:"), "Beschriftung „Rand:“: {l:?}");
    let tf = text_fields(&h);
    assert_eq!(tf.len(), 1, "ein Feld (Breite)");
    assert!(tf[0].2, "Breite gesperrt außer bei weicher Kante");
    assert!(!h.ctx.text_edit_focused(), "Startfokus OK");
    // Abbrechen: keine Wirkung
    let b = button(&h, "Abbrechen").center();
    click(&mut h, b);
    h.run_steps(4);
    assert!(!st(&h).dialogs.any_open(), "Abbrechen schließt");
    assert_eq!(undo_count(&h), n0, "kein Verlaufsschritt");
    assert!(st(&h).session.active().expect("doc").selection.pixels.is_none(), "keine Auswahl");
    assert!(fp_engine::docops::element_on_page(&st(&h).session.active().expect("doc").doc, 0, p1).is_some(), "Pfad bleibt");
    // Weiche Kante 5
    open(&mut h, "modify.convertPathToMarquee");
    h.state_mut().state.dialogs.form.as_mut().expect("Formular").field_mut("edge").expect("edge").value = json!(2);
    h.run_steps(3);
    assert!(!text_fields(&h)[0].2, "Breite bei weicher Kante frei");
    type_into(&mut h, 0, "5");
    key(&mut h, Key::Tab);
    h.run_steps(2);
    let b = button(&h, "OK").center();
    click(&mut h, b);
    h.run_steps(4);
    assert!(!st(&h).dialogs.any_open(), "OK schließt");
    assert_eq!(undo_count(&h), n0 + 1, "ein Verlaufsschritt");
    {
        let d = st(&h).session.active().expect("doc");
        let m = d.selection.pixels.as_ref().expect("Pixelauswahl");
        let b = m.bounds();
        assert!(b.left < 160 && b.right > 260, "weiche Kante verbreitert die Auswahl: {b:?}");
        assert!(m.data().iter().any(|v| *v > 0 && *v < 255), "weiche Werte");
        assert!(fp_engine::docops::element_on_page(&d.doc, d.page, p1).is_none(), "Pfad gelöscht");
    }
    h.state_mut().state.session.undo().expect("Rückgängig");
    h.run_steps(2);
    assert!(fp_engine::docops::element_on_page(&st(&h).session.active().expect("doc").doc, 0, p1).is_some(), "Strg+Z stellt den Pfad her");
    // gemerkt
    select_named(&mut h, "P1");
    open(&mut h, "modify.convertPathToMarquee");
    {
        let f = st(&h).dialogs.form.as_ref().expect("Formular");
        assert_eq!((f.get("edge"), f.get("feather")), (json!("feather"), json!(5)), "Werte gemerkt");
    }
    key(&mut h, Key::Escape);
    h.run_steps(4);
}

/// D32 Befehl: hart = nur 0/255, Anti-Alias mit Zwischenwerten an der Schräge, weiche Kante breiter.
#[test]
fn path_to_marquee_edges_in_engine() {
    let mask = |edge: &str, feather: i64| {
        let mut s = session();
        let d = s.active().expect("doc");
        let p1 = fp_engine::docops::selectable_ids(&d.doc, d.page, d.frame).into_iter().find(|id| fp_engine::docops::element_on_page(&d.doc, d.page, *id).is_some_and(|e| e.common.name.as_deref() == Some("P1"))).expect("P1");
        s.execute("select.set", json!({"ids": [p1.0]})).expect("Auswahl");
        s.execute("modify.convertPathToMarquee", json!({"edge": edge, "feather": feather})).expect("konvertieren");
        s.active().expect("doc").selection.pixels.clone().expect("Auswahl")
    };
    let hard = mask("hard", 0);
    assert!(hard.data().iter().all(|v| *v == 0 || *v == 255), "hart: nur 0/255");
    let aa = mask("antiAlias", 0);
    assert!(aa.data().iter().any(|v| *v > 0 && *v < 255), "Anti-Alias: Zwischenwerte");
    let soft = mask("feather", 6);
    assert!(soft.bounds().right - soft.bounds().left > aa.bounds().right - aa.bounds().left, "weiche Kante breiter");
    // Vorgabe ohne Parameter wie bisher: Anti-Alias
    assert_eq!(mask("antiAlias", 9).data(), aa.data(), "Breite nur bei weicher Kante");
}

//! DLG-1 (P-DLG1, Nachmessung PM-5): Zahlenfelder der Dialoge bleiben über viele Öffnungen
//! bedienbar, auch wenn das Hauptfenster nicht den Systemfokus hat (RDP ohne Vordergrund, Brücke,
//! Alt+Tab). Ereignisse wie egui-winit unter Windows: je Bild 1/60 s, Taste + Text je Zeichen.

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::{dialog_rect, last_dialog_id};
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

/// Testdokument wie PM-5: 500 × 400, R1, R2, T1 „Hallo“ 24 pt, dazu Ellipse, Symbol und Stil für
/// die Dialoge der Bedienfelder. Liefert die Sitzung und die ID des Texts.
fn session() -> (fp_engine::Session, u64) {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 400, "background": "#ffffff"})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80], "name": "R1"})).expect("R1");
    s.execute("draw.rectangle", json!({"rect": [200, 40, 300, 140], "name": "R2"})).expect("R2");
    let t = s.execute("draw.text", json!({"text": "Hallo", "anchor": [40, 300], "size": 24})).expect("T1");
    let tid = t.value.get("id").and_then(|v| v.as_u64()).expect("Text-ID");
    s.execute("draw.ellipse", json!({"rect": [320, 200, 420, 300]})).expect("ellipse");
    s.execute("modify.symbol.convertToSymbol", json!({"name": "Sym"})).expect("symbol");
    s.execute("pi.style.new", json!({"name": "Stil 1"})).expect("style");
    s.execute("select.none", json!({})).expect("none");
    (s, tid)
}

fn harness(focused: bool) -> (H, u64) {
    let ctx = egui::Context::default();
    let (s, tid) = session();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let tmp = std::env::temp_dir().join("fp-dlg1-dialogs");
    let _ = std::fs::create_dir_all(&tmp);
    app.state.file_override = Some(fp_ui::files::FilePickOverride { open: vec![tmp.join("nicht-vorhanden.png")], save: Some(tmp.join("ausgabe.png")) });
    app.state.dialogs.panel_sel.styles = vec!["Stil 1".into()];
    app.state.dialogs.panel_sel.symbol = Some(0);
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    // Fenster ohne Systemfokus (wie die Brücken-Instanz bzw. RDP ohne Vordergrundfenster)
    h.input_mut().focused = focused;
    h.run_steps(3);
    (h, tid)
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

fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
}

fn dialog_area(h: &H) -> Option<egui::Rect> {
    let id = last_dialog_id(&h.ctx)?;
    dialog_rect(&h.ctx, &id)
}

/// Werte der Textfelder im obersten Dialog (in Zeichenreihenfolge).
/// Bedienbare Eingabefelder im Dialog (gesperrte wie Selektives JPEG ohne Haken zählen nicht).
fn field_values(h: &H) -> Vec<String> {
    let Some(dr) = dialog_area(h) else { return vec![] };
    h.get_all(By::new().role(egui::accesskit::Role::TextInput)).filter(|n| dr.contains(n.rect().center()) && !n.accesskit_node().is_disabled()).map(|n| n.accesskit_node().value().unwrap_or_default().to_string()).collect()
}

fn field_center(h: &H, k: usize) -> Option<Pos2> {
    let dr = dialog_area(h)?;
    h.get_all(By::new().role(egui::accesskit::Role::TextInput)).filter(|n| dr.contains(n.rect().center()) && !n.accesskit_node().is_disabled()).nth(k).map(|n| {
        let c = n.rect().center();
        egui::pos2(c.x as f32, c.y as f32)
    })
}

fn button_center(h: &H, label: &str) -> Pos2 {
    let dr = dialog_area(h).expect("Dialog");
    let c = h.get_all(By::new().role(egui::accesskit::Role::Button)).find(|n| dr.contains(n.rect().center()) && n.accesskit_node().label().is_some_and(|l| l == label)).map(|n| n.rect().center()).expect("Knopf");
    egui::pos2(c.x as f32, c.y as f32)
}

fn open(h: &mut H, name: &str) {
    h.state_mut().state.run(name, json!({}));
    h.run_steps(4);
    assert!(st(h).dialogs.any_open(), "{name}: Dialog offen");
}

fn page_size(h: &H) -> (f32, f32) {
    let d = st(h).session.active().expect("doc");
    let c = &d.doc.pages[d.page].canvas;
    (c.width, c.height)
}

fn text_size(h: &H, id: u64) -> f32 {
    let d = st(h).session.active().expect("doc");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id)).expect("Text");
    let fp_model::ElementKind::Text(t) = &e.kind else { panic!("Text") };
    t.resolved_runs().first().and_then(|(a, _)| a.size).unwrap_or(0.0)
}

fn grid_size(h: &H) -> (f32, f32) {
    let d = st(h).session.active().expect("doc");
    d.doc.pages[d.page].root.grid.as_ref().map(|g| (g.size.x, g.size.y)).unwrap_or((36.0, 36.0))
}

/// Punkt 1 (hoch): Leinwandgröße, Bildgröße, Raster bearbeiten und Textgröße je 10× im Wechsel
/// öffnen, jedes Mal einen neuen Wert tippen, OK bzw. Abbrechen; der getippte Wert steht im Feld
/// und wirkt bei OK. Mit und ohne Systemfokus des Fensters.
#[test]
fn size_dialogs_take_typed_values_on_every_opening() {
    for focused in [false, true] {
        let (mut h, tid) = harness(focused);
        for round in 0..10usize {
            let ok = round % 2 == 0;
            // Leinwandgröße: Breite
            let (w0, _) = page_size(&h);
            let v = 520 + round * 10;
            open(&mut h, "modify.canvas.canvasSize");
            typ(&mut h, &v.to_string());
            h.run_steps(3);
            assert_eq!(field_values(&h).first().map(String::as_str), Some(v.to_string().as_str()), "fokus={focused} Runde {round}: Leinwandgröße zeigt die getippte Breite");
            if ok {
                key(&mut h, Key::Enter);
            } else {
                let c = button_center(&h, &fp_i18n::t("btn-cancel"));
                click(&mut h, c);
            }
            h.run_steps(4);
            assert!(!st(&h).dialogs.any_open(), "Runde {round}: Leinwandgröße geschlossen");
            assert_eq!(page_size(&h).0, if ok { v as f32 } else { w0 }, "fokus={focused} Runde {round}: Leinwandbreite");

            // Bildgröße: Breite (Proportionen beibehalten)
            let (w1, _) = page_size(&h);
            let v = 300 + round * 7;
            open(&mut h, "modify.canvas.imageSize");
            typ(&mut h, &v.to_string());
            h.run_steps(3);
            assert_eq!(field_values(&h).first().map(String::as_str), Some(v.to_string().as_str()), "fokus={focused} Runde {round}: Bildgröße zeigt die getippte Breite");
            if ok {
                key(&mut h, Key::Enter);
            } else {
                key(&mut h, Key::Escape);
            }
            h.run_steps(4);
            assert!(!st(&h).dialogs.any_open(), "Runde {round}: Bildgröße geschlossen");
            assert_eq!(page_size(&h).0, if ok { v as f32 } else { w1 }, "fokus={focused} Runde {round}: Bildbreite");

            // Raster bearbeiten: waagerechter Abstand (erstes Feld)
            let g0 = grid_size(&h);
            let v = 10 + round;
            open(&mut h, "view.grid.editGrid");
            let c = field_center(&h, 0).expect("Rasterfeld");
            click(&mut h, c);
            key_m(&mut h, Key::A, ctrl());
            typ(&mut h, &v.to_string());
            h.run_steps(3);
            assert_eq!(field_values(&h).first().map(String::as_str), Some(v.to_string().as_str()), "fokus={focused} Runde {round}: Raster zeigt den getippten Abstand");
            if ok {
                key(&mut h, Key::Enter);
            } else {
                key(&mut h, Key::Escape);
            }
            h.run_steps(4);
            assert_eq!(grid_size(&h).0, if ok { v as f32 } else { g0.0 }, "fokus={focused} Runde {round}: Rasterabstand");

            // Textgröße (Text gewählt)
            h.state_mut().state.session.execute("select.set", json!({"ids": [tid]})).expect("Auswahl");
            h.run_steps(2);
            let s0 = text_size(&h, tid);
            let v = 30 + round;
            open(&mut h, "text.size.other");
            let c = field_center(&h, 0).expect("Größenfeld");
            click(&mut h, c);
            key_m(&mut h, Key::A, ctrl());
            typ(&mut h, &v.to_string());
            h.run_steps(3);
            assert_eq!(field_values(&h).first().map(String::as_str), Some(v.to_string().as_str()), "fokus={focused} Runde {round}: Textgröße zeigt den getippten Wert");
            if ok {
                key(&mut h, Key::Enter);
            } else {
                key(&mut h, Key::Escape);
            }
            h.run_steps(4);
            assert_eq!(text_size(&h, tid), if ok { v as f32 } else { s0 }, "fokus={focused} Runde {round}: Textgröße");
            h.state_mut().state.session.execute("select.none", json!({})).expect("none");
        }
    }
}

/// Dialoge mit Eingabefeldern (Liste wie `dialogs_open.rs`, ohne Datei-/Druckdialoge).
const DIALOG_COMMANDS: &[&str] = &[
    "modify.canvas.canvasSize",
    "modify.canvas.imageSize",
    "modify.transform.numericTransform",
    "filter.blur.gaussian",
    "filter.blur.motion",
    "filter.blur.radial",
    "filter.blur.zoom",
    "filter.sharpen.unsharpMask",
    "filter.noise.addNoise",
    "filter.adjustColor.brightnessContrast",
    "filter.adjustColor.hueSaturation",
    "select.feather",
    "select.expandMarquee",
    "select.contractMarquee",
    "select.borderMarquee",
    "select.smoothMarquee",
    "select.saveBitmapSelection",
    "modify.alterPath.simplify",
    "modify.alterPath.insetPath",
    "modify.alterPath.expandStroke",
    "text.size.other",
    "modify.symbol.tweenInstances",
    "modify.symbol.convertToSymbol",
    "modify.animation.animateSelection",
    "modify.animation.settings",
    "view.grid.editGrid",
    "view.guides.editGuides",
    "edit.insert.layer",
    "edit.insert.state",
    "file.exportWizard",
    "file.htmlSetup",
    "modify.selectiveJpeg.settings",
    "edit.insert.newSymbol",
    "edit.insert.newButton",
    "text.spellingSetup",
    "panel.layers.newLayer",
    "panel.layers.newSubLayer",
    "panel.layers.duplicate",
    "panel.layers.shareToPages",
    "panel.states.add",
    "panel.states.duplicate",
    "panel.states.properties",
    "panel.states.onionCustom",
    "panel.pages.rename",
    "panel.styles.rename",
    "panel.library.newSymbol",
    "panel.library.properties",
    "panel.optimize.saveSettings",
    "panel.optimize.optimizeToSize",
    "panel.optimize.exportWizard",
    "panel.effects.saveAsStyle",
];

/// Punkt 1, alle Dialoge mit Eingabefeldern: je Dialog 10× öffnen, ins erste Feld klicken, Inhalt
/// markieren, einen neuen Wert tippen; er muss stehen bleiben (kein Rücksprung auf den alten Wert).
/// Fenster ohne Systemfokus, alle Dialoge in derselben Instanz nacheinander.
#[test]
fn every_dialog_field_keeps_typed_text_over_ten_openings() {
    let (mut h, _) = harness(false);
    h.state_mut().state.session.execute("select.all", json!({})).expect("alles");
    let mut checked = Vec::new();
    let mut skipped = Vec::new();
    for name in DIALOG_COMMANDS {
        for round in 0..10usize {
            h.state_mut().state.run(name, json!({}));
            h.run_steps(4);
            if !st(&h).dialogs.any_open() {
                skipped.push(format!("{name} (kein Dialog)"));
                break;
            }
            let Some(c) = field_center(&h, 0) else {
                skipped.push(name.to_string());
                key(&mut h, Key::Escape);
                h.run_steps(3);
                break;
            };
            click(&mut h, c);
            key_m(&mut h, Key::A, ctrl());
            let v = (11 + round).to_string();
            typ(&mut h, &v);
            h.run_steps(3);
            assert_eq!(field_values(&h).first().map(String::as_str), Some(v.as_str()), "{name} Runde {round}: getippter Wert bleibt stehen");
            key(&mut h, Key::Escape);
            h.run_steps(4);
            assert!(!st(&h).dialogs.any_open(), "{name} Runde {round}: Esc schließt");
            if round == 9 {
                checked.push(*name);
            }
        }
    }
    eprintln!("geprüft {}: {checked:?}\nohne Eingabefeld/Dialog: {skipped:?}", checked.len());
    assert!(checked.len() >= 25, "zu wenige Dialoge geprüft ({}): ohne Feld {skipped:?}", checked.len());
}

// ---------------------------------------------------------------------- Tab-Folge (PM-5)

/// Rolle und Name des fokussierten Bedienelements (Barrierefreiheits-Baum).
fn focused(h: &H) -> Option<(egui::accesskit::Role, String)> {
    h.ctx.memory(|m| m.focused())?;
    h.get_all(By::new()).find(|n| n.accesskit_node().is_focused()).map(|n| {
        let node = n.accesskit_node();
        let name = node.label().map(|s| s.to_string()).or_else(|| node.value().map(|s| s.to_string())).unwrap_or_default();
        (node.role(), name)
    })
}

/// Tab-Folge eines Dialogs: `n` × Tab, je Schritt das fokussierte Element.
fn tab_cycle(h: &mut H, n: usize) -> Vec<(egui::accesskit::Role, String)> {
    let mut v = Vec::new();
    for _ in 0..n {
        key(h, Key::Tab);
        h.run_steps(1);
        v.push(focused(h).unwrap_or((egui::accesskit::Role::Unknown, String::new())));
    }
    v
}

/// Die Titelleiste (und ihr Schließen-Kreuz) ist in keinem Dialog ein Tab-Stopp.
#[test]
fn title_bar_is_never_a_tab_stop() {
    let (mut h, _) = harness(true);
    h.state_mut().state.session.execute("select.all", json!({})).expect("alles");
    let title = fp_i18n::tr!("dlg-titlebar", title = "");
    let mut seen = 0;
    for name in DIALOG_COMMANDS {
        h.state_mut().state.run(name, json!({}));
        h.run_steps(4);
        if !st(&h).dialogs.any_open() {
            continue;
        }
        let id = last_dialog_id(&h.ctx).expect("Dialog");
        for _ in 0..20 {
            key(&mut h, Key::Tab);
            let f = h.ctx.memory(|m| m.focused());
            assert!(f != Some(egui::Id::new((id.as_str(), "title-drag"))) && f != Some(egui::Id::new((id.as_str(), "x"))), "{name}: Titelleiste bzw. Kreuz als Tab-Stopp");
            if let Some((_, n)) = focused(&h) {
                assert!(!(n.starts_with(title.trim()) && !title.trim().is_empty()), "{name}: Tab erreicht „{n}“");
            }
        }
        seen += 1;
        key(&mut h, Key::Escape);
        h.run_steps(4);
        if st(&h).dialogs.any_open() {
            h.state_mut().state.dialogs = Default::default();
            h.run_steps(2);
        }
    }
    assert!(seen >= 40, "{seen} Dialoge geprüft");
}

/// Startfokus wie im Original gemessen: Exportassistent, HTML-Einrichtung, Selektives JPEG und
/// Rechtschreibung einrichten auf OK; Leinwandfarbe auf dem gewählten Optionsfeld „Weiß“; sonst das
/// erste Eingabefeld.
#[test]
fn start_focus_like_the_original() {
    let ok = fp_i18n::t("btn-ok");
    for name in ["file.exportWizard", "file.htmlSetup", "modify.selectiveJpeg.settings", "text.spellingSetup"] {
        let (mut h, _) = harness(true);
        open(&mut h, name);
        h.run_steps(3);
        assert_eq!(focused(&h), Some((egui::accesskit::Role::Button, ok.clone())), "{name}: Startfokus OK");
        // Eingabe = OK (fokussierter Standardknopf)
        key(&mut h, Key::Enter);
        h.run_steps(4);
        assert!(!st(&h).dialogs.any_open(), "{name}: Eingabe schließt");
    }
    let (mut h, _) = harness(true);
    open(&mut h, "modify.canvas.canvasColor");
    h.run_steps(3);
    let f = focused(&h).expect("Fokus");
    assert_eq!(f.0, egui::accesskit::Role::RadioButton, "Leinwandfarbe: Startfokus auf der Optionsgruppe ({f:?})");
    for name in ["modify.canvas.canvasSize", "modify.canvas.imageSize", "view.grid.editGrid", "text.size.other", "file.new"] {
        let (mut h, _) = harness(true);
        open(&mut h, name);
        h.run_steps(3);
        assert_eq!(focused(&h).map(|f| f.0), Some(egui::accesskit::Role::TextInput), "{name}: Startfokus erstes Feld");
    }
}

/// Optionsfeld-Gruppen sind ein Tab-Stopp, Pfeiltasten wählen innerhalb der Gruppe.
#[test]
fn radio_group_is_one_tab_stop_with_arrow_keys() {
    let (mut h, _) = harness(true);
    open(&mut h, "modify.canvas.canvasColor");
    h.run_steps(3);
    let cycle = tab_cycle(&mut h, 12);
    let radios = cycle.iter().filter(|(r, _)| *r == egui::accesskit::Role::RadioButton).count();
    assert!(radios <= 12 / 3 + 1 && radios >= 1, "Leinwandfarbe: Optionsgruppe einmal je Umlauf: {cycle:?}");
    // Stopps je Umlauf: Gruppe, (Farbfeld), OK, Abbrechen → kein Optionsfeld zweimal hintereinander
    assert!(cycle.windows(2).all(|w| !(w[0].0 == egui::accesskit::Role::RadioButton && w[1].0 == egui::accesskit::Role::RadioButton)), "zwei Optionsfelder nacheinander: {cycle:?}");
    // zur Gruppe zurück, Pfeil ab → „Transparent“ gewählt und fokussiert
    for _ in 0..12 {
        if focused(&h).is_some_and(|f| f.0 == egui::accesskit::Role::RadioButton) {
            break;
        }
        key(&mut h, Key::Tab);
    }
    let before = focused(&h).expect("Gruppe");
    key(&mut h, Key::ArrowDown);
    h.run_steps(2);
    let after = focused(&h).expect("Fokus nach Pfeil");
    assert_eq!(after.0, egui::accesskit::Role::RadioButton, "Pfeil bleibt in der Gruppe");
    assert_ne!(after.1, before.1, "Pfeil wählt das nächste Optionsfeld");
    let f = st(&h).dialogs.form.as_ref().expect("Formular");
    assert_eq!(f.fields.iter().find(|x| matches!(x.kind, fp_ui::dialogs::form::FieldKind::Radio(_))).and_then(|x| x.value.as_u64()), Some(1), "Transparent gewählt");
    key(&mut h, Key::ArrowUp);
    h.run_steps(2);
    let f = st(&h).dialogs.form.as_ref().expect("Formular");
    assert_eq!(f.fields.iter().find(|x| matches!(x.kind, fp_ui::dialogs::form::FieldKind::Radio(_))).and_then(|x| x.value.as_u64()), Some(0), "Pfeil auf → wieder Weiß");
}

/// „Neues Dokument“: Breite › Höhe › Auflösung › 3 Einheiten › Optionsgruppe › OK › Abbrechen ›
/// Vorlagen… (Farbfeld kein Tab-Stopp, Original PM-5).
#[test]
fn new_document_tab_order_like_the_original() {
    let (mut h, _) = harness(true);
    open(&mut h, "file.new");
    h.run_steps(3);
    let start = focused(&h).expect("Startfokus");
    assert_eq!(start.0, egui::accesskit::Role::TextInput);
    let cycle = tab_cycle(&mut h, 10);
    use egui::accesskit::Role as R;
    let roles: Vec<R> = cycle.iter().map(|c| c.0).collect();
    let (ok, cancel, tmpl) = (fp_i18n::t("btn-ok"), fp_i18n::t("btn-cancel"), fp_i18n::t("dlg-new-doc-templates"));
    assert_eq!(&roles[0..2], &[R::TextInput, R::TextInput], "Höhe, Auflösung: {cycle:?}");
    assert_eq!(roles.iter().filter(|r| **r == R::RadioButton).count(), 1, "eine Optionsgruppe: {cycle:?}");
    let pos = |l: &str| cycle.iter().position(|c| c.0 == R::Button && c.1.replace('&', "") == l.replace('&', ""));
    let (po, pc, pt) = (pos(&ok), pos(&cancel), pos(&tmpl));
    assert!(po.is_some() && pc.is_some() && pt.is_some(), "Knöpfe im Umlauf: {cycle:?}");
    assert!(po < pc && pc < pt, "OK › Abbrechen › Vorlagen…: {cycle:?}");
    let radio = roles.iter().position(|r| *r == R::RadioButton).expect("Gruppe");
    assert_eq!(Some(radio + 1), po, "nach der Optionsgruppe direkt OK (Farbfeld kein Tab-Stopp): {cycle:?}");
    // Umlauf zurück zur Breite
    assert_eq!(cycle[9].0, R::TextInput, "nach Vorlagen… wieder Breite: {cycle:?}");
}

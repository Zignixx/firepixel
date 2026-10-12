//! P-DLG2: Raster bearbeiten (5984), Leinwandfarbe (5977), Selektive JPEG-Einstellungen (19147)
//! und Textgröße (6073) wie im Original (Befunde PM-5, PARITAET_B §2 D18/D21/D22/D23): Felder,
//! Anordnung nach der Vorlage, gesperrte Felder, Vorgabewerte. Realistische Eingaben (Klicks über
//! mehrere Bilder, Tippen als `Event::Text`).

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::kittest::{By, NodeT, Queryable};
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
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h
}

fn harness() -> H {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80]})).expect("rect");
    harness_with(s)
}

/// Dokument mit einem 24-pt-Text (gewählt) und einem Rechteck.
fn harness_text() -> (H, u64) {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [20, 120, 120, 180]})).expect("rect");
    let id = s.execute("draw.text", json!({"text": "Hallo", "anchor": [30, 60], "size": 24})).expect("text").value["id"].as_u64().expect("id");
    (harness_with(s), id)
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

fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

fn open(h: &mut H, cmd: &str) {
    h.state_mut().state.run(cmd, json!({}));
    h.run_steps(3);
    assert!(st(h).dialogs.form.is_some(), "{cmd}: Formular offen");
}

fn form(h: &H) -> &fp_ui::dialogs::form::FormDialog {
    st(h).dialogs.form.as_ref().expect("Formular")
}

fn dialog_rect(h: &H) -> Rect {
    fp_ui::dialogs::dialog_rect(&h.ctx, &fp_ui::dialogs::last_dialog_id(&h.ctx).expect("Dialog")).expect("Rechteck")
}

fn client(h: &H) -> egui::Vec2 {
    fp_ui::dialogs::dialog_geom(&h.ctx, &fp_ui::dialogs::last_dialog_id(&h.ctx).expect("Dialog")).expect("Lage").client
}

/// Knoten einer Rolle im Dialog, von oben nach unten (Rechteck, Name, gesperrt).
fn nodes(h: &H, role: Role) -> Vec<(Rect, String, bool)> {
    let dr = dialog_rect(h);
    let mut v: Vec<(Rect, String, bool)> = h.query_all(By::new().role(role)).map(|n| (n.rect(), n.accesskit_node().label().or_else(|| n.value()).unwrap_or_default(), n.accesskit_node().is_disabled())).filter(|(r, _, _)| dr.contains(r.center())).collect();
    v.sort_by(|a, b| a.0.top().total_cmp(&b.0.top()).then(a.0.left().total_cmp(&b.0.left())));
    v
}

fn text_fields(h: &H) -> Vec<(Rect, String, bool)> {
    let dr = dialog_rect(h);
    let mut v: Vec<(Rect, String, bool)> = h.query_all(By::new().role(Role::TextInput)).map(|n| (n.rect(), n.value().unwrap_or_default(), n.accesskit_node().is_disabled())).filter(|(r, _, _)| dr.contains(r.center())).collect();
    v.sort_by(|a, b| a.0.top().total_cmp(&b.0.top()));
    v
}

fn button(h: &H, label: &str) -> Rect {
    nodes(h, Role::Button).into_iter().find(|(_, l, _)| l == label).unwrap_or_else(|| panic!("Knopf {label}")).0
}

fn ok(h: &mut H) {
    let r = button(h, &fp_i18n::t("btn-ok"));
    click(h, r.center());
    h.run_steps(3);
}

/// In ein Feld klicken, Inhalt ersetzen, Tab übernimmt.
fn type_into(h: &mut H, r: Rect, text: &str) {
    click(h, r.center());
    ev(h, Event::Key { key: Key::A, physical_key: Some(Key::A), pressed: true, repeat: false, modifiers: Modifiers::COMMAND });
    for ch in text.chars() {
        ev(h, Event::Text(ch.to_string()));
    }
    key(h, Key::Tab);
    h.run_steps(2);
}

fn focused(h: &H) -> Option<(Role, String)> {
    h.ctx.memory(|m| m.focused())?;
    h.get_all(By::new()).find(|n| n.accesskit_node().is_focused()).map(|n| {
        let node = n.accesskit_node();
        (node.role(), node.label().map(|s| s.to_string()).or_else(|| node.value().map(|s| s.to_string())).unwrap_or_default())
    })
}

fn grid(h: &H) -> Option<fp_model::Grid> {
    let d = st(h).session.active().expect("doc");
    d.doc.pages.get(d.page).and_then(|p| p.root.grid.clone())
}

fn text_size(h: &H, id: u64) -> f32 {
    let d = st(h).session.active().expect("doc");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id)).expect("Text");
    let fp_model::ElementKind::Text(t) = &e.kind else { panic!("Text") };
    t.resolved_runs().first().and_then(|(a, _)| a.size).unwrap_or(0.0)
}

/// D18: nur Farbe und zwei Abstände (keine Kästchen), ↔/↕ als Symbol, Größe nach der Vorlage
/// (151 × 89 DLU ≈ 227 × 145 px); OK mit 20/30 wirkt und lässt „anzeigen“/„ausrichten“ unberührt.
#[test]
fn edit_grid_like_the_original() {
    let mut h = harness();
    h.state_mut().state.run("view.grid.showGrid", json!({}));
    h.run_steps(2);
    let shown = st(&h).session.active().expect("doc").view.show_grid;
    let snapped = st(&h).session.active().expect("doc").view.snap_to_grid;
    open(&mut h, "view.grid.editGrid");
    let keys: Vec<String> = form(&h).fields.iter().map(|f| f.key.clone()).collect();
    assert_eq!(keys, ["color", "width", "height"], "keine Kästchen „Raster anzeigen“/„Am Raster ausrichten“");
    assert!(nodes(&h, Role::CheckBox).is_empty(), "kein Kästchen im Dialog: {:?}", nodes(&h, Role::CheckBox));
    let c = client(&h);
    assert!((c.x - 227.0).abs() <= 8.0 && (c.y - 145.0).abs() <= 4.0, "Größe wie die Vorlage: {c:?}");
    // Symbole statt „Abstand ↔:“-Beschriftung: der Text ist nur noch Name/Tooltip des Symbols
    let fields = text_fields(&h);
    assert_eq!(fields.len(), 2, "zwei Abstände: {fields:?}");
    assert_eq!(fields[0].1, "36");
    let labels = nodes(&h, Role::Label);
    for (k, f) in [("dlg-lbl-grid-h", &fields[0]), ("dlg-lbl-grid-v", &fields[1])] {
        let sym = labels.iter().find(|(_, l, _)| *l == fp_i18n::t(k)).unwrap_or_else(|| panic!("Symbol {k}: {labels:?}"));
        assert!(sym.0.right() <= f.0.left() && sym.0.width() < 30.0, "Symbol links vom Feld, klein: {sym:?} {f:?}");
    }
    // Feld links von der Einheit, zweites Feld darunter; OK links von Abbrechen, unter den Feldern
    assert!(fields[1].0.top() > fields[0].0.bottom());
    let (okr, cancel) = (button(&h, &fp_i18n::t("btn-ok")), button(&h, &fp_i18n::t("btn-cancel")));
    assert!(okr.right() < cancel.left() && okr.top() > fields[1].0.bottom());
    // Tab wie im Original: Feld › Feld › OK › Abbrechen (Farbfeld kein Tab-Stopp)
    h.run_steps(2);
    assert_eq!(focused(&h).map(|f| f.0), Some(Role::TextInput), "Startfokus erstes Feld");
    let mut cycle = Vec::new();
    for _ in 0..4 {
        key(&mut h, Key::Tab);
        h.run_steps(2);
        cycle.push(focused(&h).expect("Fokus"));
    }
    let (okl, cancell) = (fp_i18n::t("btn-ok"), fp_i18n::t("btn-cancel"));
    assert_eq!(cycle[0].0, Role::TextInput, "{cycle:?}");
    assert_eq!(cycle[1], (Role::Button, okl), "{cycle:?}");
    assert_eq!(cycle[2], (Role::Button, cancell), "{cycle:?}");
    assert_eq!(cycle[3].0, Role::TextInput, "Umlauf ohne Farbfeld: {cycle:?}");
    type_into(&mut h, fields[0].0, "20");
    let r = text_fields(&h)[1].0;
    type_into(&mut h, r, "30");
    assert_eq!(form(&h).get("width").as_f64(), Some(20.0));
    ok(&mut h);
    assert!(st(&h).dialogs.form.is_none(), "zu");
    let g = grid(&h).expect("Raster");
    assert_eq!((g.size.x, g.size.y), (20.0, 30.0));
    let d = st(&h).session.active().expect("doc");
    assert_eq!((d.view.show_grid, d.view.snap_to_grid), (shown, snapped), "Anzeigen/Ausrichten unverändert");
    // zweites Öffnen zeigt 20/30
    open(&mut h, "view.grid.editGrid");
    let f = text_fields(&h);
    assert_eq!((f[0].1.as_str(), f[1].1.as_str()), ("20", "30"));
    key(&mut h, Key::Escape);
}

/// D21: Optionen untereinander, Farbfeld eingerückt unter „Benutzerdefiniert:“, schmal wie die
/// Vorlage (153 DLU ≈ 230 px); Wirkung unverändert.
#[test]
fn canvas_color_stacked_like_the_original() {
    let mut h = harness();
    open(&mut h, "modify.canvas.canvasColor");
    let radios = nodes(&h, Role::RadioButton);
    assert_eq!(radios.len(), 3, "{radios:?}");
    let names: Vec<&str> = radios.iter().map(|r| r.1.as_str()).collect();
    assert_eq!(names, [fp_i18n::t("dlg-canvas-white"), fp_i18n::t("dlg-canvas-transparent"), fp_i18n::t("dlg-canvas-custom")]);
    assert_eq!(fp_i18n::t("dlg-canvas-custom"), "Benutzerdefiniert:");
    for w in radios.windows(2) {
        assert!((w[0].0.left() - w[1].0.left()).abs() < 0.5 && w[1].0.top() >= w[0].0.bottom(), "untereinander: {radios:?}");
    }
    // Farbfeld: kein „Farbe:“, eingerückt unter dem dritten Optionsfeld
    assert!(nodes(&h, Role::Label).iter().all(|(_, l, _)| *l != fp_i18n::t("dlg-lbl-color")), "keine Beschriftung „Farbe:“");
    fp_ui::inspector::fields::set_audit(&h.ctx, true);
    h.run_steps(1);
    let audit = fp_ui::inspector::fields::take_audit(&h.ctx);
    fp_ui::inspector::fields::set_audit(&h.ctx, false);
    let custom = radios[2].0;
    let sw = audit.iter().filter(|a| a.kind == "field" && a.drawn.top() >= custom.bottom() - 0.5 && (a.drawn.width() - 26.0).abs() < 0.5).map(|a| a.drawn).next().expect("Farbfeld unter „Benutzerdefiniert:“");
    assert!(sw.left() > custom.left() + 10.0 && sw.left() < custom.left() + 30.0, "eingerückt: {sw:?} unter {custom:?}");
    let ok_r = button(&h, &fp_i18n::t("btn-ok"));
    assert!(ok_r.top() > sw.bottom(), "Knöpfe darunter");
    let c = client(&h);
    assert!(c.x <= 240.0, "schmal wie die Vorlage: {c:?}");
    // Transparent + OK wirkt wie bisher
    click(&mut h, radios[1].0.center());
    ok(&mut h);
    let d = st(&h).session.active().expect("doc");
    assert_eq!(d.doc.pages[d.page].root.background.a(), 0, "transparent");
}

/// D22: ohne Haken sind Qualität, Überlagerungsfarbe und die beiden Kästchen gesperrt (Tippen und
/// Klicken wirken nicht); mit Haken bedienbar. Überlagerungsfarbe = `SelectiveJPEGColor` (#ff0080),
/// bei OK gemerkt; Qualität rechts neben dem Haken.
#[test]
fn selective_jpeg_greyed_until_enabled() {
    let mut h = harness();
    open(&mut h, "modify.selectiveJpeg.settings");
    assert_eq!(form(&h).get("_overlay").as_str().map(|s| s[..7].to_ascii_lowercase()), Some("#ff0080".into()), "Vorgabe der Überlagerungsfarbe");
    let enable = fp_i18n::t("dlg-md-enable-selective");
    let checks = nodes(&h, Role::CheckBox);
    assert_eq!(checks.len(), 3, "{checks:?}");
    assert_eq!(checks[0].1, enable);
    assert!(!checks[0].2 && checks[1].2 && checks[2].2, "ohne Haken gesperrt: {checks:?}");
    let q = text_fields(&h);
    assert_eq!(q.len(), 1);
    assert!(q[0].2, "Qualität gesperrt");
    assert!(q[0].0.left() > checks[0].0.left() + 100.0 && (q[0].0.center().y - checks[0].0.center().y).abs() < 4.0, "Qualität rechts neben dem Haken: {q:?} {checks:?}");
    // gesperrt: Tippen und Klicken ändern nichts
    type_into(&mut h, q[0].0, "50");
    assert_eq!(form(&h).get("quality").as_f64(), Some(90.0));
    let text_check = checks[1].0;
    click(&mut h, text_check.center());
    assert_eq!(form(&h).get("preserveText").as_bool(), Some(true), "gesperrtes Kästchen bleibt");
    // Haken setzen: alles bedienbar
    click(&mut h, checks[0].0.center());
    assert_eq!(form(&h).get("enabled").as_bool(), Some(true));
    let checks = nodes(&h, Role::CheckBox);
    assert!(checks.iter().all(|c| !c.2), "mit Haken bedienbar: {checks:?}");
    assert!(!text_fields(&h)[0].2, "Qualität bedienbar");
    let r = text_fields(&h)[0].0;
    type_into(&mut h, r, "70");
    assert_eq!(form(&h).get("quality").as_f64(), Some(70.0));
    // andere Überlagerungsfarbe (wie aus der Farbwahl), OK: gemerkt, Einstellungen am Dokument
    h.state_mut().state.dialogs.form.as_mut().expect("Formular").field_mut("_overlay").expect("Farbfeld").value = json!("#00ff00ff");
    ok(&mut h);
    assert!(st(&h).dialogs.form.is_none());
    assert_eq!(st(&h).session.prefs.get_str("SelectiveJPEGColor", ""), "#00ff00");
    let v = h.state_mut().state.session.execute("modify.selectiveJpeg.settings", json!({})).expect("Abfrage").value;
    assert_eq!(v["enabled"], json!(true));
    assert_eq!(v["quality"], json!(70));
    open(&mut h, "modify.selectiveJpeg.settings");
    assert_eq!(form(&h).get("_overlay").as_str().map(|s| s[..7].to_string()), Some("#00ff00".into()), "gemerkt");
    assert!(!nodes(&h, Role::CheckBox)[1].2, "Haken gemerkt, Kästchen bedienbar");
    key(&mut h, Key::Escape);
}

/// D23: Vorgabe = Größe des gewählten Texts (24), ohne Textauswahl der zuletzt bestätigte Wert
/// (48), sonst die Vorgabe des Textwerkzeugs; Feld mit Popup-Schieber, ohne Einheit, Knöpfe rechts
/// untereinander.
#[test]
fn text_size_shows_selection_or_last_value() {
    let (mut h, id) = harness_text();
    // ohne Auswahl, noch nichts bestätigt: Vorgabe des Werkzeugs (12)
    h.state_mut().state.run("select.none", json!({}));
    h.run_steps(2);
    open(&mut h, "text.size.other");
    assert_eq!(form(&h).get("size").as_f64(), Some(12.0));
    key(&mut h, Key::Escape);
    h.run_steps(2);
    h.state_mut().state.run("select.all", json!({}));
    h.run_steps(2);
    open(&mut h, "text.size.other");
    assert_eq!(text_fields(&h).first().map(|f| f.1.clone()), Some("24".into()), "Größe des gewählten Texts");
    let f = &form(&h).fields[0];
    assert!(f.popup, "Popup-Schieber ▾");
    assert!(matches!(f.kind, fp_ui::dialogs::form::FieldKind::Number { unit: "", min: 8.0, max: 96.0, .. }), "ohne Einheit, Bereich 8..96: {:?}", f.kind);
    let fr = text_fields(&h)[0].0;
    let (okr, cancel) = (button(&h, &fp_i18n::t("btn-ok")), button(&h, &fp_i18n::t("btn-cancel")));
    assert!(okr.left() > fr.right() + 15.0 && (okr.left() - cancel.left()).abs() < 0.5 && cancel.top() > okr.bottom(), "Knöpfe rechts untereinander: {okr:?} {cancel:?}");
    let label = nodes(&h, Role::Label);
    assert!(label.iter().all(|(_, l, _)| *l != fp_i18n::t("dlg-unit-points")), "keine Einheit „Punkt“");
    let c = client(&h);
    assert!((c.x - 274.5).abs() <= 8.0 && (c.y - 81.0).abs() <= 4.0, "Größe wie die Vorlage (183 × 50 DLU): {c:?}");
    type_into(&mut h, fr, "48");
    ok(&mut h);
    assert!(st(&h).dialogs.form.is_none());
    assert_eq!(text_size(&h, id), 48.0);
    // ohne Textauswahl: der zuletzt bestätigte Wert
    h.state_mut().state.run("select.none", json!({}));
    h.run_steps(2);
    open(&mut h, "text.size.other");
    assert_eq!(form(&h).get("size").as_f64(), Some(48.0), "zuletzt benutzt");
    key(&mut h, Key::Escape);
    h.run_steps(2);
    // gewählter Text gewinnt immer
    h.state_mut().state.run("select.all", json!({}));
    h.state_mut().state.run("text.size.other", json!({"size": 30}));
    h.run_steps(2);
    open(&mut h, "text.size.other");
    assert_eq!(form(&h).get("size").as_f64(), Some(30.0));
    key(&mut h, Key::Escape);
    // Vorgabe des Werkzeugs inzwischen anders (Inspektor): die zählt
    h.run_steps(2);
    h.state_mut().state.run("select.none", json!({}));
    h.state_mut().state.session.tools.options.text_size = 18.0;
    h.run_steps(2);
    open(&mut h, "text.size.other");
    assert_eq!(form(&h).get("size").as_f64(), Some(18.0), "geänderte Werkzeugvorgabe");
    key(&mut h, Key::Escape);
}

/// Englisch: längere Texte passen (keine Überläufe), Anordnung bleibt.
#[test]
fn dlg2_dialogs_fit_in_english() {
    fp_i18n::with_language("en", dlg2_dialogs_fit);
}

fn dlg2_dialogs_fit() {
    let (mut h, _) = harness_text();
    for cmd in ["view.grid.editGrid", "modify.canvas.canvasColor", "modify.selectiveJpeg.settings", "text.size.other"] {
        let _ = fp_ui::dialogs::take_overflows(&h.ctx);
        open(&mut h, cmd);
        h.run_steps(2);
        let o = fp_ui::dialogs::take_overflows(&h.ctx);
        assert!(o.is_empty(), "{cmd}: {o:?}");
        key(&mut h, Key::Escape);
        h.run_steps(2);
    }
}

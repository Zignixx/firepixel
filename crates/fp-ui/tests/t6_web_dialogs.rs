//! T-6 (re 31 §1–§3): Dialoge „Bild austauschen“ (6013) und „Navigationsleistenbild festlegen“ (6015):
//! Vorgaben, Grau-Regeln, OK-Übernahme, Einstiege aus Kontextmenü und Verhalten-Bedienfeld.

use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use fp_ui::app_state::{Action, AppState, UiSettings};
use fp_ui::dialogs::web_behavior::{NavBarDialog, SwapImageDialog, WebBehaviorDialog};
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness_with(s: fp_engine::Session) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1600.0, 1000.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

/// Dokument mit 3 Status, Segmente s_a (gewählt) und s_b.
fn session() -> (fp_engine::Session, u64, u64) {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    let mut ids = Vec::new();
    for (r, n) in [([20, 20, 120, 80], "s_a"), ([150, 20, 250, 80], "s_b")] {
        s.execute("select.none", json!({})).expect("none");
        s.execute("draw.rectangle", json!({"rect": r})).expect("rect");
        s.execute("edit.insert.slice", json!({})).expect("slice");
        let id = s.active().expect("d").selection.elements[0];
        ids.push(id.0);
        s.execute("pi.web", json!({"ids": [id.0], "fileName": n})).expect("name");
    }
    s.execute("state.add", json!({})).expect("state 2");
    s.execute("state.add", json!({})).expect("state 3");
    s.active_mut().expect("d").selection.elements = vec![fp_model::ElementId(ids[0])];
    (s, ids[0], ids[1])
}

fn behaviors(st: &AppState, id: u64) -> Vec<fp_model::Behavior> {
    let d = st.session.active().expect("d");
    match fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id)).map(|e| e.kind.clone()) {
        Some(fp_model::ElementKind::WebObject(w)) => w.behaviors.map(|b| b.items).unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn swap(h: &H) -> SwapImageDialog {
    match &st(h).dialogs.web_behavior {
        Some(WebBehaviorDialog::Swap(d)) => d.clone(),
        o => panic!("Bild austauschen erwartet: {o:?}"),
    }
}

fn nav(h: &H) -> NavBarDialog {
    match &st(h).dialogs.web_behavior {
        Some(WebBehaviorDialog::NavBar(d)) => d.clone(),
        o => panic!("Navigationsleistenbild erwartet: {o:?}"),
    }
}

fn ok(h: &mut H, p: serde_json::Value) {
    h.state_mut().state.dialogs.web_behavior = None;
    h.state_mut().state.run("pi.behavior.add", p);
    h.run_steps(2);
}

fn ctx_action(h: &mut H, name: &str) {
    let ctx = h.ctx.clone();
    fp_ui::canvas::context::apply(&ctx, &mut h.state_mut().state, Action::Run(name.into(), json!({})), None);
    h.run_steps(2);
}

fn swap_new(h: &mut H) -> SwapImageDialog {
    ctx_action(h, "ui.behaviorSwapImage");
    let d = swap(h);
    h.state_mut().state.dialogs.web_behavior = None;
    d
}

/// §1.3/§1.7: neues Verhalten aus dem Kontextmenü: Liste ohne Auswahl, Status „2“, Status-Nr.,
/// beide Kästchen an; Statusliste = Anzahl Status (3).
#[test]
fn swap_defaults_for_new_behavior() {
    let (s, _, _) = session();
    let mut h = harness_with(s);
    let d = swap_new(&mut h);
    assert_eq!(d.targets.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(), ["s_a", "s_b"]);
    assert_eq!(d.target, None);
    assert_eq!(d.frame, 2);
    assert_eq!(d.frame_count, 3);
    assert!(!d.use_file && d.preload && d.restore);
    assert!(d.frame_list_enabled() && !d.file_enabled());
}

/// §1.4 Grau-Regeln, „Durchsuchen“ wandelt `\` in `/`; §1.5 OK mit Datei: Status −1, Pfad mit `/`,
/// Ziel = Slice-ID des gewählten Segments.
#[test]
fn swap_source_rules_and_file_ok() {
    let (s, a, b) = session();
    let mut h = harness_with(s);
    let mut d = swap_new(&mut h);
    d.choose_file();
    assert!(d.file_enabled() && !d.frame_list_enabled());
    d.browsed("C:\\bilder\\x.gif");
    assert_eq!(d.file, "C:/bilder/x.gif");
    assert_eq!(d.browse_start(), "C:\\bilder\\x.gif");
    d.target = Some(1);
    ok(&mut h, d.params());
    let l = behaviors(st(&h), a);
    assert_eq!(l.len(), 1);
    let x = &l[0];
    assert_eq!((x.action, x.event, x.frame, x.file.as_deref(), x.preload, x.restore), (2, 0, Some(-1), Some("C:/bilder/x.gif"), Some(true), Some(true)));
    let dd = st(&h).session.active().expect("d");
    let sid = match fp_engine::docops::element_on_page(&dd.doc, dd.page, fp_model::ElementId(b)).map(|e| e.kind.clone()) {
        Some(fp_model::ElementKind::WebObject(w)) => w.slice.and_then(|s| s.slice_id),
        _ => None,
    };
    assert!(sid.is_some() && x.target_slice == sid, "Ziel = Slice-ID von s_b");
    // zurück auf Status-Nr.: Datei verworfen, Eingabe grau
    let mut d2 = swap_new(&mut h);
    d2.choose_file();
    d2.file = "a/b.png".into();
    d2.choose_frame();
    assert!(d2.file.is_empty() && !d2.file_enabled());
}

/// §1.7: Ziel s_b, Status 3 → `frame` 2 (0-basiert); OK ohne Ziel legt trotzdem an; Bearbeiten
/// (Doppelklick) zeigt die Werte und ersetzt an der Position.
#[test]
fn swap_ok_frame_without_target_and_edit() {
    let (s, a, _) = session();
    let mut h = harness_with(s);
    let mut d = swap_new(&mut h);
    d.target = Some(1);
    d.frame = 3;
    d.preload = false;
    ok(&mut h, d.params());
    let d = swap_new(&mut h);
    ok(&mut h, d.params());
    let l = behaviors(st(&h), a);
    assert_eq!(l.len(), 2, "angehängt, nie ersetzt");
    assert_eq!((l[0].frame, l[0].preload, l[0].file.as_deref()), (Some(2), Some(false), None));
    assert!(l[0].target_slice.is_some());
    assert_eq!(l[1].target_slice, None, "OK ohne Auswahl: kein Ziel");
    fp_ui::dialogs::web_behavior::open_swap(&mut h.state_mut().state, Some(0));
    let mut e = swap(&h);
    assert_eq!((e.target, e.frame, e.preload), (Some(1), 3, false));
    e.frame = 1;
    ok(&mut h, e.params());
    let l = behaviors(st(&h), a);
    assert_eq!(l.len(), 2);
    assert_eq!(l[0].frame, Some(0));
}

/// Klick in den Lageplan wählt das Segment unter dem Punkt.
#[test]
fn swap_map_pick() {
    let (s, _, _) = session();
    let mut h = harness_with(s);
    let d = swap_new(&mut h);
    assert_eq!(d.pick_at(fp_geom::Point::new(200.0, 50.0)), Some(1));
    assert_eq!(d.pick_at(fp_geom::Point::new(50.0, 50.0)), Some(0));
    assert_eq!(d.pick_at(fp_geom::Point::new(50.0, 150.0)), None);
}

/// §2.3: Kontextmenü: beide aus und anhängen; zweites Mal Werte übernehmen und an der Position
/// ersetzen. Bedienfeld „+“: beide aus, immer anhängen.
#[test]
fn nav_bar_context_menu_replaces() {
    let (s, a, _) = session();
    let mut h = harness_with(s);
    h.state_mut().state.run("pi.behavior.add", json!({"action": 1, "event": 0, "statusText": "x"}));
    h.run_steps(2);
    ctx_action(&mut h, "ui.behaviorNavBar");
    let mut d = nav(&h);
    assert!(!d.over_down && !d.down_on_load && d.replace);
    d.over_down = true;
    ok(&mut h, d.params());
    let l = behaviors(st(&h), a);
    assert_eq!(l.len(), 2);
    assert_eq!((l[1].action, l[1].event, l[1].flags, l[1].checked), (0, 0, Some(3), Some(false)));
    ctx_action(&mut h, "ui.behaviorNavBar");
    let mut d = nav(&h);
    assert!(d.over_down && !d.down_on_load, "Werte des vorhandenen Verhaltens");
    d.down_on_load = true;
    ok(&mut h, d.params());
    let l = behaviors(st(&h), a);
    assert_eq!(l.len(), 2, "ersetzt an seiner Position");
    assert_eq!((l[1].flags, l[1].checked), (Some(3), Some(true)));
    fp_ui::dialogs::web_behavior::open_nav_bar(&mut h.state_mut().state, false, None);
    let d = nav(&h);
    assert!(!d.over_down && !d.down_on_load && !d.replace);
    ok(&mut h, d.params());
    assert_eq!(behaviors(st(&h), a).len(), 3);
}

/// Die Dialoge zeichnen sich mit den Beschriftungen der Vorlage; Esc bricht ohne Änderung ab.
#[test]
fn dialogs_render_and_cancel() {
    let (s, a, _) = session();
    let mut h = harness_with(s);
    ctx_action(&mut h, "ui.behaviorSwapImage");
    h.run_steps(3);
    assert!(h.query_by_label("Bilder vorab laden").is_some(), "Kästchen Vorab laden");
    assert!(h.query_by_label("Grafikdatei:").is_some(), "Optionsfeld Grafikdatei");
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    assert!(st(&h).dialogs.web_behavior.is_none());
    assert!(behaviors(st(&h), a).is_empty());
    ctx_action(&mut h, "ui.behaviorNavBar");
    h.run_steps(3);
    assert!(h.query_by_label("OverDown-Status einschließen (Status 4)").is_some());
}

/// Export (15 §2.5/§2.6, re 31 §4): Austausch aus Datei nimmt den Pfad unverändert; ohne Ziel still verworfen.
#[test]
fn export_uses_file_path_unchanged() {
    let b = fp_model::Behavior { target_slice: Some(7), file: Some("bilder/x.gif".into()), frame: Some(-1), preload: Some(true), restore: Some(true), ..fp_model::Behavior::new(fp_model::BehaviorAction::SwapImage, 0) };
    let v = fp_export::behavior::expand_for_export(&b, false, 3, &|id| id == 7);
    assert_eq!(v.len(), 2, "Austausch + Wiederherstellen: {v:?}");
    assert_eq!(v[0].file, "bilder/x.gif");
    let none = fp_model::Behavior { target_slice: None, ..b };
    assert!(fp_export::behavior::expand_for_export(&none, false, 3, &|_| true).is_empty());
}

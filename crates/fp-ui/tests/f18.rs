//! F18 (Tweak-Liste 15.10.): Ebenen-Bedienfeld TW-54 (Alt+Klick aufs Auge, Ziehen ab dem Auge einer
//! Unterebene), Kontextmenü TW-50, Filtermenü TW-51, Raster TW-55, Abdunkeln Q-25.

use egui::{Event, Modifiers, PointerButton, Pos2};
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
    let mut h = Harness::builder().with_size(egui::vec2(1600.0, 1000.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn ev(h: &mut H, e: Event) {
    if let Event::PointerButton { modifiers, .. } = &e {
        h.event(Event::ModifiersChanged(*modifiers));
    }
    h.event(e);
    h.run_steps(1);
}

fn press(h: &mut H, p: Pos2, down: bool, m: Modifiers) {
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: down, modifiers: m });
}

fn texts(shape: &egui::epaint::Shape, out: &mut Vec<(String, egui::Rect)>) {
    match shape {
        egui::epaint::Shape::Text(t) => out.push((t.galley.job.text.clone(), egui::Rect::from_min_size(t.pos, t.galley.size()))),
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
        _ => {}
    }
}

fn all_texts(h: &H) -> Vec<(String, egui::Rect)> {
    let mut all = Vec::new();
    for cs in &h.output().shapes {
        texts(&cs.shape, &mut all);
    }
    all
}

/// Ebene 1…3, in Ebene 3 eine Unterebene; liefert die Sitzung und den Namen der Unterebene.
fn layered() -> (fp_engine::Session, String) {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80]})).expect("rect");
    s.execute("layer.add", json!({"name": "Ebene 2"})).expect("l2");
    s.execute("layer.add", json!({"name": "Ebene 3"})).expect("l3");
    s.execute("pi.layer.addSub", json!({})).expect("sub");
    let d = s.active().expect("d");
    let mut name = String::new();
    for (_, l) in fp_engine::docops::own_layers(&d.doc, d.page) {
        for c in &l.cels {
            for e in &c.elements {
                if let fp_model::ElementKind::SubLayer(sl) = &e.kind {
                    name = sl.name.clone();
                }
            }
        }
    }
    assert!(!name.is_empty(), "Unterebene angelegt");
    (s, name)
}

fn show_layers_panel(h: &mut H) {
    for _ in 0..2 {
        if all_texts(h).iter().any(|(t, _)| t == "Ebene 1") {
            return;
        }
        let tab = all_texts(h).into_iter().filter(|(t, _)| t == "Ebenen").map(|(_, r)| r.center()).max_by(|a, b| a.x.total_cmp(&b.x)).expect("Reiter");
        ev(h, Event::PointerMoved(tab));
        press(h, tab, true, Modifiers::NONE);
        press(h, tab, false, Modifiers::NONE);
        h.run_steps(30);
    }
}

/// Mitte des Auges der Zeile mit dem Namen `name` (Ebenen- und Unterebenenzeilen beginnen gleich links).
fn eye_of(h: &H, name: &str) -> Pos2 {
    let layer = all_texts(h).into_iter().filter(|(t, _)| t == "Ebene 1").map(|(_, r)| r).max_by(|a, b| a.left().total_cmp(&b.left())).expect("Ebene 1");
    let row = all_texts(h).into_iter().filter(|(t, _)| t == name).map(|(_, r)| r).max_by(|a, b| a.left().total_cmp(&b.left())).unwrap_or_else(|| panic!("Zeile {name}"));
    Pos2::new(layer.left() - 78.0 + 10.0, row.center().y)
}

fn shown(h: &H) -> Vec<(String, bool)> {
    let d = st(h).session.active().expect("d");
    let mut out = Vec::new();
    for (_, l) in fp_engine::docops::own_layers(&d.doc, d.page) {
        if l.kind == fp_model::LayerKind::Web {
            continue;
        }
        out.push((l.name.clone(), l.is_shown(d.frame)));
        for c in &l.cels {
            for e in &c.elements {
                if let fp_model::ElementKind::SubLayer(sl) = &e.kind {
                    out.push((sl.name.clone(), e.common.visible && sl.is_shown(d.frame)));
                }
            }
        }
    }
    out
}

/// TW-54: Alt+Klick aufs Auge von Ebene 1 blendet wie im Original alle Ebenen samt Unterebene aus.
#[test]
fn alt_click_on_eye_hides_all_layers() {
    let (s, _) = layered();
    let mut h = harness_with(s);
    show_layers_panel(&mut h);
    let p = eye_of(&h, "Ebene 1");
    ev(&mut h, Event::PointerMoved(p));
    press(&mut h, p, true, Modifiers::ALT);
    press(&mut h, p, false, Modifiers::ALT);
    h.run_steps(3);
    let v = shown(&h);
    assert!(v.len() >= 4 && v.iter().all(|(_, x)| !x), "alle ausgeblendet: {v:?}");
}

/// TW-54, ab Lauf 10 TW-94 (F24): Ziehen ab dem Auge einer Unterebene bis zum Auge von Ebene 1: die
/// Unterebene (Startzeile) und die überstrichenen Ebenen 2 und 1 werden ausgeblendet
/// (`panel-layers/04_eye_drag` im Original, Lauf 10).
#[test]
fn eye_drag_from_sublayer_hides_crossed_layers() {
    let (s, sub) = layered();
    let mut h = harness_with(s);
    show_layers_panel(&mut h);
    let a = eye_of(&h, &sub);
    let b = eye_of(&h, "Ebene 1");
    ev(&mut h, Event::PointerMoved(a));
    press(&mut h, a, true, Modifiers::NONE);
    for i in 1..=30 {
        ev(&mut h, Event::PointerMoved(a + (b - a) * (i as f32 / 30.0)));
    }
    press(&mut h, b, false, Modifiers::NONE);
    h.run_steps(3);
    let v: std::collections::BTreeMap<String, bool> = shown(&h).into_iter().collect();
    assert_eq!(v.get(&sub), Some(&false), "Startzeile umgeschaltet (TW-94): {v:?}");
    assert_eq!(v.get("Ebene 3"), Some(&true), "Ebene 3 nicht überstrichen: {v:?}");
    assert_eq!(v.get("Ebene 2"), Some(&false), "Ebene 2 ausgeblendet: {v:?}");
    assert_eq!(v.get("Ebene 1"), Some(&false), "Ebene 1 ausgeblendet: {v:?}");
}

/// TW-69 (F19): schnelles Ziehen über die Augen (wenige Mausbewegungen) schaltet jede überstrichene
/// Zeile, auch die Unterebene dazwischen (`panel-layers/04_eye_drag`: Ebene 1 und Unterebene aus).
#[test]
fn tw69_fast_eye_drag_hides_every_crossed_row_with_sublayer() {
    let (s, sub) = layered();
    let mut h = harness_with(s);
    show_layers_panel(&mut h);
    let a = eye_of(&h, "Ebene 3");
    let b = eye_of(&h, "Ebene 1");
    let undo = |h: &H| st(h).session.active().map(|d| d.history.undo_count()).unwrap_or(0);
    let n0 = undo(&h);
    ev(&mut h, Event::PointerMoved(a));
    press(&mut h, a, true, Modifiers::NONE);
    for i in 1..=3 {
        ev(&mut h, Event::PointerMoved(a + (b - a) * (i as f32 / 3.0)));
    }
    press(&mut h, b, false, Modifiers::NONE);
    h.run_steps(3);
    let v: std::collections::BTreeMap<String, bool> = shown(&h).into_iter().collect();
    for n in ["Ebene 3", sub.as_str(), "Ebene 2", "Ebene 1"] {
        assert_eq!(v.get(n), Some(&false), "{n} ausgeblendet: {v:?}");
    }
    // TW-69 Rest (F21): ein Verlaufsschritt für den ganzen Zug; Rückgängig blendet alle wieder ein
    assert_eq!(undo(&h), n0 + 1, "ein Schritt für den Zug");
    h.state_mut().state.session.undo().expect("undo");
    h.run_steps(3);
    let v: std::collections::BTreeMap<String, bool> = shown(&h).into_iter().collect();
    for n in ["Ebene 3", sub.as_str(), "Ebene 2", "Ebene 1"] {
        assert_eq!(v.get(n), Some(&true), "{n} wieder sichtbar: {v:?}");
    }
}

fn labels(nodes: &[fp_ui::menu::Node]) -> Vec<(String, bool)> {
    nodes
        .iter()
        .map(|n| match n {
            fp_ui::menu::Node::Item { label, enabled, .. } | fp_ui::menu::Node::Sub { label, enabled, .. } => (label.clone(), *enabled),
            fp_ui::menu::Node::Separator => ("-".into(), true),
        })
        .collect()
}

fn web_session(slice: bool) -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80]})).expect("rect");
    s.execute("select.all", json!({})).expect("sel");
    s.execute(if slice { "edit.insert.slice" } else { "edit.insert.hotspot" }, json!({})).expect("web");
    s
}

fn selected_behaviors(st: &AppState) -> usize {
    let d = st.session.active().expect("d");
    d.selection
        .elements
        .iter()
        .filter_map(|id| fp_engine::docops::element_on_page(&d.doc, d.page, *id))
        .map(|e| match &e.kind {
            fp_model::ElementKind::WebObject(w) => w.behaviors.as_ref().map(|b| b.items.len()).unwrap_or(0),
            _ => 0,
        })
        .sum()
}

/// TW-50: Kontextmenü eines Segments: Export, Anordnen, Verhalten-Block, Vollbildmodus beenden.
#[test]
fn slice_context_menu_has_behavior_block() {
    let h = harness_with(web_session(true));
    let st = st(&h);
    let id = fp_ui::canvas::context::menu_for(st);
    assert_eq!(id, "web-slice");
    let l = labels(&fp_ui::canvas::context::build(st, fp_ui::canvas::context::entries(id)));
    let names: Vec<&str> = l.iter().map(|x| x.0.as_str()).filter(|x| *x != "-").collect();
    assert_eq!(
        names,
        [
            "Ausgewähltes Segment exportieren...",
            "CSS-Sprites exportieren...",
            "Anordnen",
            "Verhalten \"Einfaches Rollover\" hinzufügen",
            "Verhalten \"Bild austauschen\" hinzufügen...",
            "Statusleistenmeldung hinzufügen...",
            "Navigationsleiste hinzufügen...",
            "Popupmenü hinzufügen...",
            "Popupmenü bearbeiten...",
            "Alle Verhalten löschen",
            "Vollbildmodus beenden",
        ]
    );
    let edit_popup = l.iter().find(|x| x.0 == "Popupmenü bearbeiten...").expect("bearbeiten");
    assert!(!edit_popup.1, "Popupmenü bearbeiten grau ohne Popupmenü");
    assert!(l.iter().find(|x| x.0 == "Alle Verhalten löschen").is_some_and(|x| x.1), "Alle Verhalten löschen aktiv");
}

/// TW-50: Hotspot ohne Einfaches Rollover und Navigationsleiste.
#[test]
fn hotspot_context_menu_without_rollover_and_nav_bar() {
    let h = harness_with(web_session(false));
    let st = st(&h);
    let id = fp_ui::canvas::context::menu_for(st);
    assert_eq!(id, "web-hotspot");
    let l = labels(&fp_ui::canvas::context::build(st, fp_ui::canvas::context::entries(id)));
    let names: Vec<&str> = l.iter().map(|x| x.0.as_str()).filter(|x| *x != "-").collect();
    assert_eq!(names, ["Verhalten \"Bild austauschen\" hinzufügen...", "Statusleistenmeldung hinzufügen...", "Popupmenü hinzufügen...", "Popupmenü bearbeiten...", "Alle Verhalten löschen", "Vollbildmodus beenden"]);
}

/// TW-50: Einfaches Rollover legt das Verhalten an, Alle Verhalten löschen entfernt alle,
/// Statusleistenmeldung öffnet den Dialog.
#[test]
fn behavior_entries_act_on_the_slice() {
    let mut h = harness_with(web_session(true));
    let nodes = fp_ui::canvas::context::build(st(&h), fp_ui::canvas::context::entries("web-slice"));
    let act = |label: &str| {
        nodes
            .iter()
            .find_map(|n| match n {
                fp_ui::menu::Node::Item { label: l, action, .. } if l == label => Some(action.clone()),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{label}"))
    };
    let run = |h: &mut H, a: fp_ui::menu::ItemAction| {
        match a {
            fp_ui::menu::ItemAction::Ui(a) => {
                let ctx = h.ctx.clone();
                fp_ui::canvas::context::apply(&ctx, &mut h.state_mut().state, a, None);
            }
            fp_ui::menu::ItemAction::Command(n) => h.state_mut().state.run(&n, json!({})),
            _ => panic!("ohne Aktion"),
        }
        h.run_steps(3);
    };
    run(&mut h, act("Verhalten \"Einfaches Rollover\" hinzufügen"));
    assert_eq!(selected_behaviors(st(&h)), 1);
    // T-6 (re 31 §3): „Bild austauschen“ öffnet jetzt den Dialog 6013 statt direkt anzuhängen; OK hängt an
    run(&mut h, act("Verhalten \"Bild austauschen\" hinzufügen..."));
    assert_eq!(selected_behaviors(st(&h)), 1);
    let p = match &st(&h).dialogs.web_behavior {
        Some(fp_ui::dialogs::web_behavior::WebBehaviorDialog::Swap(d)) => d.params(),
        o => panic!("Dialog Bild austauschen erwartet: {o:?}"),
    };
    h.state_mut().state.dialogs.web_behavior = None;
    h.state_mut().state.run("pi.behavior.add", p);
    h.run_steps(2);
    assert_eq!(selected_behaviors(st(&h)), 2);
    run(&mut h, act("Alle Verhalten löschen"));
    assert_eq!(selected_behaviors(st(&h)), 0);
    run(&mut h, act("Statusleistenmeldung hinzufügen..."));
    assert!(st(&h).dialogs.form.is_some(), "Dialog Statusleistentext offen");
}

/// TW-51: Filter ▸ Farbe anpassen hat wie im Original 6 Einträge, ohne „Farbfüllung…“ (die gibt es nur
/// als Live-Filter).
#[test]
fn adjust_color_menu_without_color_fill() {
    let h = harness_with(web_session(true));
    let menus = fp_ui::menu::build_main_menu(st(&h));
    fn find<'a>(nodes: &'a [fp_ui::menu::Node], name: &str) -> Option<&'a [fp_ui::menu::Node]> {
        for n in nodes {
            if let fp_ui::menu::Node::Sub { label, children, .. } = n {
                if label == name {
                    return Some(children);
                }
                if let Some(c) = find(children, name) {
                    return Some(c);
                }
            }
        }
        None
    }
    let filter = menus.iter().find(|(n, _)| n.replace('&', "") == "Filter").map(|(_, c)| c.as_slice()).expect("Filter");
    let adj = labels(find(filter, "Farbe anpassen").expect("Farbe anpassen"));
    let names: Vec<&str> = adj.iter().map(|x| x.0.as_str()).filter(|x| *x != "-").collect();
    assert_eq!(names.len(), 6, "{names:?}");
    assert!(!names.iter().any(|n| n.starts_with("Farbfüllung")), "{names:?}");
}

fn meshes(shape: &egui::epaint::Shape, out: &mut Vec<egui::epaint::Mesh>) {
    match shape {
        egui::epaint::Shape::Mesh(m) => out.push((**m).clone()),
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| meshes(s, out)),
        _ => {}
    }
}

/// TW-55: Datei ohne Raster: Raster der Voreinstellungen (Cyan) gepunktet (Muster-Textur, jedes
/// zweite Pixel), nicht durchgezogen grau.
#[test]
fn grid_without_file_grid_is_dotted_cyan() {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    {
        let d = s.doc_mut().expect("d");
        let page = d.page;
        std::sync::Arc::make_mut(&mut d.doc).page_mut(page).expect("p").root.grid = None;
    }
    s.execute("view.grid.showGrid", json!({})).expect("grid");
    let mut h = harness_with(s);
    h.run_steps(3);
    let mut all = Vec::new();
    for cs in &h.output().shapes {
        meshes(&cs.shape, &mut all);
    }
    let cyan = egui::Color32::from_rgb(0, 255, 255);
    let grid = all.iter().find(|m| m.texture_id != egui::TextureId::default() && !m.vertices.is_empty() && m.vertices.iter().all(|v| v.color == cyan)).expect("Raster-Netz in Cyan");
    // 300 / 36 → 8 senkrechte, 200 / 36 → 5 waagrechte Linien (je 4 Ecken), UV halbe Pixel: Punktmuster
    assert!(grid.vertices.len() >= 4 * 13, "{}", grid.vertices.len());
    let v = &grid.vertices[..4];
    let (du, dv) = ((v[1].uv.x - v[0].uv.x).abs().max((v[2].uv.x - v[0].uv.x).abs()), (v[2].uv.y - v[0].uv.y).abs());
    let (dx, dy) = ((v[1].pos.x - v[0].pos.x).abs().max((v[2].pos.x - v[0].pos.x).abs()), (v[2].pos.y - v[0].pos.y).abs());
    assert!((du - dx / 2.0).abs() < 0.01 && (dv - dy / 2.0).abs() < 0.01, "Textur wiederholt sich alle 2 Pixel");
}

/// Q-25: Filterdialog mit Vorschau dunkelt die Leinwand nicht ab, ein Dialog ohne Vorschau weiter.
#[test]
fn preview_dialogs_do_not_dim_the_canvas() {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    let img = fp_raster::RgbaImage::filled(fp_geom::IRect::from_xywh(40, 40, 100, 80), fp_color::Rgba8::new(200, 30, 30, 255)).expect("img");
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    s.execute("edit.paste", json!({})).expect("paste");
    let mut h = harness_with(s);
    h.state_mut().state.run("filter.blur.gaussian", json!({}));
    h.run_steps(4);
    let f = st(&h).dialogs.form.as_ref().expect("Filterdialog offen");
    assert!(f.preview, "Dialog mit Vorschau");
    let id = format!("fp-form-{}", f.id);
    assert_eq!(fp_ui::dialogs::dialog_backdrop_alpha(&h.ctx, &id), 0, "keine Abdunklung");
    // ohne Vorschau (Statusleistentext) bleibt die Abdunklung
    h.state_mut().state.dialogs.form = None;
    h.run_steps(2);
    h.state_mut().state.dialogs.open_status_text();
    h.run_steps(4);
    let f = st(&h).dialogs.form.as_ref().expect("Dialog offen");
    assert!(!f.preview);
    let id = format!("fp-form-{}", f.id);
    assert!(fp_ui::dialogs::dialog_backdrop_alpha(&h.ctx, &id) > 0, "abgedunkelt");
}

/// TW-91 (F23): Hinweis- und Meldungsfenster dunkeln die Leinwand nicht ab (Original: eigenes
/// Meldungsfenster, Leinwand bleibt hell), z. B. der Hinweis der Pfadwerkzeuge auf einem Rechteck.
#[test]
fn tw91_hint_dialogs_do_not_dim_the_canvas() {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    let mut h = harness_with(s);
    fp_ui::dialogs::warning::path_tools_on_primitive(&mut h.state_mut().state);
    h.run_steps(4);
    assert!(st(&h).dialogs.warning.is_some(), "Hinweis offen");
    assert_eq!(fp_ui::dialogs::dialog_backdrop_alpha(&h.ctx, "fp-warning"), 0, "keine Abdunklung");
}

/// TW-94 (F24): Zug über die Augen ab der Unterebene „Gemeinsam“ (T07) bis „Ebene 1“: das Original
/// blendet die Startzeile, die zweite Unterebene und Ebene 1 aus (`panel-layers/04_eye_drag`, Lauf 10:
/// `Ebene 1:false|Ebene 2:false|Ebene 3:true|Unterebene:false|Gemeinsam:false|Webebene:true`).
#[test]
fn tw94_eye_drag_toggles_start_row_t07() {
    let src = std::path::PathBuf::from(r"C:\Users\Administrator\Desktop\REA\re\_testdateien\par\par_layers.fw.png");
    if !src.is_file() {
        return;
    }
    let dir = std::env::temp_dir().join(format!("fp-f24-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let p = dir.join("t07.fw.png");
    std::fs::copy(&src, &p).expect("copy");
    let mut s = fp_engine::Session::default();
    s.execute("file.open", json!({"path": p.to_string_lossy()})).expect("open");
    let mut h = harness_with(s);
    show_layers_panel(&mut h);
    let before: std::collections::BTreeMap<String, bool> = shown(&h).into_iter().collect();
    assert_eq!(before.get("Gemeinsam"), Some(&true), "{before:?}");
    let a = eye_of(&h, "Gemeinsam");
    let b = eye_of(&h, "Ebene 1");
    ev(&mut h, Event::PointerMoved(a));
    press(&mut h, a, true, Modifiers::NONE);
    for i in 1..=12 {
        ev(&mut h, Event::PointerMoved(a + (b - a) * (i as f32 / 12.0)));
    }
    press(&mut h, b, false, Modifiers::NONE);
    h.run_steps(3);
    let v: std::collections::BTreeMap<String, bool> = shown(&h).into_iter().collect();
    assert_eq!(v.get("Gemeinsam"), Some(&false), "Startzeile umgeschaltet: {v:?}");
    assert_eq!(v.get("Unterebene"), Some(&false), "Unterebene aus: {v:?}");
    assert_eq!(v.get("Ebene 1"), Some(&false), "Ebene 1 aus: {v:?}");
    assert_eq!(v.get("Ebene 2"), Some(&false), "Ebene 2 bleibt aus: {v:?}");
    assert_eq!(v.get("Ebene 3"), Some(&true), "Ebene 3 (über der Startzeile) nicht überstrichen: {v:?}");
    // ein Verlaufsschritt; Rückgängig stellt alle wieder her
    h.state_mut().state.session.undo().expect("undo");
    h.run_steps(3);
    let u: std::collections::BTreeMap<String, bool> = shown(&h).into_iter().collect();
    assert_eq!(u, before, "Rückgängig in einem Schritt");
}

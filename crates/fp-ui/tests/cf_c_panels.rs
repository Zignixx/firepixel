//! CF-C: Befunde des Klicktests CT-C in Bedienfeldern und Abläufen der Oberfläche. Echte Eingaben
//! (Zeiger drücken, ziehen, loslassen) über egui, je Ereignis ein Bild.

use egui::{Event, Modifiers, PointerButton, Pos2, Rect, Shape};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::panels::PanelId;
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness_with(s: fp_engine::Session, show: &[PanelId]) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    for p in show {
        app.state.settings.workspace.show(*p);
    }
    let mut h = Harness::builder().with_size(egui::vec2(1600.0, 1000.0)).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
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

fn drag(h: &mut H, a: Pos2, b: Pos2, frames: usize) {
    ev(h, Event::PointerMoved(a));
    ev(h, Event::PointerButton { pos: a, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    for i in 1..=frames {
        ev(h, Event::PointerMoved(a + (b - a) * (i as f32 / frames as f32)));
    }
    ev(h, Event::PointerButton { pos: b, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(3);
}

/// Alle gezeichneten Texte mit Lage.
fn painted_texts(h: &H) -> Vec<(String, Rect)> {
    fn walk(s: &Shape, out: &mut Vec<(String, Rect)>) {
        match s {
            Shape::Text(t) => out.push((t.galley.text().to_string(), t.visual_bounding_rect())),
            Shape::Vec(v) => v.iter().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let mut v = Vec::new();
    for cs in &h.output().shapes {
        walk(&cs.shape, &mut v);
    }
    v
}

fn text_rect(h: &H, pred: impl Fn(&str) -> bool) -> Rect {
    let all = painted_texts(h);
    all.iter().find(|(s, _)| pred(s)).map(|(_, r)| *r).unwrap_or_else(|| panic!("Text nicht gezeichnet: {:?}", all.iter().map(|(s, _)| s).collect::<Vec<_>>()))
}

fn state_names(h: &H) -> Vec<String> {
    let d = st(h).session.active().expect("doc");
    let n = d.frame_count();
    (0..n).map(|i| d.doc.pages[d.page].root.export.as_ref().and_then(|e| e.frames.get(i)).and_then(|f| f.name.clone()).unwrap_or_default()).collect()
}

/// CT-C-02: „Status 3“ auf „Status 1“ ziehen → Status 3 steht vorn (mit Inhalt und Namen), ein
/// Verlaufsschritt; Rückgängig stellt die alte Reihenfolge her.
#[test]
fn ct_c_02_states_reorder_by_drag() {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 200, "height": 100, "background": "#ffffff"})).expect("doc");
    s.execute("state.add", json!({"count": 2})).expect("status");
    for (i, name) in ["Eins", "Zwei", "Drei"].iter().enumerate() {
        s.execute("pi.state.props", json!({"indices": [i], "name": name})).expect("Name");
    }
    s.execute("state.setCurrent", json!({"index": 2})).expect("Status 3");
    s.execute("draw.rectangle", json!({"rect": [10, 10, 50, 50], "fillColor": "#ff0000"})).expect("Rechteck");
    s.execute("state.setCurrent", json!({"index": 0})).expect("Status 1");
    let undo_before = s.active().expect("doc").history.undo_count();
    let mut h = harness_with(s, &[PanelId::States]);
    assert_eq!(state_names(&h), ["Eins", "Zwei", "Drei"]);
    let from = text_rect(&h, |t| t == "Drei").center();
    let one = text_rect(&h, |t| t == "Eins");
    drag(&mut h, from, Pos2::new(from.x, one.top() + 2.0), 10);
    assert_eq!(state_names(&h), ["Drei", "Eins", "Zwei"], "umgeordnet");
    let d = st(&h).session.active().expect("doc");
    assert_eq!(d.frame, 1, "aktueller Status („Eins“) bleibt gewählt");
    let has_rect = |h: &H, f: usize| {
        let d = st(h).session.active().expect("doc");
        d.doc.pages[d.page].root.layers.iter().any(|l| matches!(l, fp_model::LayerSlot::Own(l) if l.cels.get(f).is_some_and(|c| !c.elements.is_empty())))
    };
    assert!(has_rect(&h, 0), "Inhalt von Status 3 wandert mit");
    assert_eq!(d.history.undo_count(), undo_before + 1, "ein Verlaufsschritt");
    h.state_mut().state.session.execute("edit.undo", json!({})).expect("undo");
    h.run_steps(2);
    assert_eq!(state_names(&h), ["Eins", "Zwei", "Drei"], "Rückgängig");
    assert!(has_rect(&h, 2));
}

/// CT-C-03: „02 Seite 2“ über „01 Seite 1“ ziehen → Seite 2 steht vorn, ein Verlaufsschritt.
#[test]
fn ct_c_03_pages_reorder_by_drag() {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 200, "height": 100, "background": "#ffffff"})).expect("doc");
    s.execute("page.add", json!({})).expect("Seite");
    s.execute("page.rename", json!({"index": 0, "name": "Alpha"})).expect("Name");
    s.execute("page.rename", json!({"index": 1, "name": "Beta"})).expect("Name");
    s.execute("page.setCurrent", json!({"index": 0})).expect("Seite 1");
    let mut h = harness_with(s, &[PanelId::Pages]);
    let names = |h: &H| st(h).session.active().expect("doc").doc.pages.iter().map(|p| p.display_name().to_string()).collect::<Vec<_>>();
    assert_eq!(names(&h), ["Alpha", "Beta"]);
    let from = text_rect(&h, |t| t.ends_with("Beta")).center();
    let first = text_rect(&h, |t| t.ends_with("Alpha"));
    drag(&mut h, from, Pos2::new(from.x, first.top() - 4.0), 10);
    assert_eq!(names(&h), ["Beta", "Alpha"], "umgeordnet");
    assert_eq!(st(&h).session.active().expect("doc").page, 1, "aktuelle Seite (Alpha) bleibt aktuell");
    h.state_mut().state.session.execute("edit.undo", json!({})).expect("undo");
    h.run_steps(2);
    assert_eq!(names(&h), ["Alpha", "Beta"], "Rückgängig");
}

/// Befehle direkt (MCP/Skript): Quelle als Parameter, Ziel begrenzt, falscher Index = Fehler.
#[test]
fn ct_c_02_03_move_commands() {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 50, "height": 50})).expect("doc");
    s.execute("state.add", json!({"count": 3})).expect("status");
    s.execute("pi.state.props", json!({"indices": [0], "name": "A"})).expect("Name");
    s.execute("state.move", json!({"state": 0, "index": 99})).expect("move");
    let d = s.active().expect("doc");
    assert_eq!(d.doc.pages[0].root.export.as_ref().and_then(|e| e.frames.get(3)).and_then(|f| f.name.clone()).as_deref(), Some("A"));
    assert!(s.execute("state.move", json!({"state": 9, "index": 0})).is_err());
    s.execute("page.add", json!({})).expect("Seite");
    s.execute("page.add", json!({})).expect("Seite");
    s.execute("page.rename", json!({"index": 2, "name": "C"})).expect("Name");
    s.execute("page.move", json!({"page": 2, "index": 0})).expect("move");
    assert_eq!(s.active().expect("doc").doc.pages[0].display_name(), "C");
    assert!(s.execute("page.move", json!({"page": 5, "index": 0})).is_err());
    fp_i18n::set_language("en");
    let lbl = |n: &str| fp_i18n::t(&fp_i18n::command_key(n));
    assert_eq!(lbl("state.move"), "Move State");
    assert_eq!(lbl("page.move"), "Move Page");
    fp_i18n::set_language("de");
    assert_eq!(lbl("state.move"), "Status verschieben");
}

fn press(h: &mut H, p: Pos2, down: bool) {
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: down, modifiers: Modifiers::NONE });
}

/// Bildschirmstelle eines Dokumentpunkts auf der Leinwand.
fn screen_of(h: &H, x: f32, y: f32) -> Pos2 {
    let d = st(h).session.active().expect("doc");
    let area = st(h).canvases.get(&d.id).and_then(|c| c.area).expect("Leinwand");
    let v = &d.view;
    area.min + egui::vec2(((x as f64 - v.origin.0) * v.zoom) as f32, ((y as f64 - v.origin.1) * v.zoom) as f32)
}

fn selected_bounds(h: &H) -> Vec<fp_geom::Rect> {
    let d = st(h).session.active().expect("doc");
    d.selection.elements.iter().filter_map(|id| fp_engine::docops::element_on_page(&d.doc, d.page, *id)).filter_map(|e| fp_engine::docops::element_bounds(&d.doc, &e)).collect()
}

/// Grenzen des obersten Objekts der aktuellen Ebene (das Platzieren lässt nichts ausgewählt, re8 §2.4).
fn top_bounds(h: &H) -> Vec<fp_geom::Rect> {
    let d = st(h).session.active().expect("doc");
    let top = fp_engine::docops::own_layers(&d.doc, d.page).into_iter().rev().find_map(|(_, l)| l.cels.get(d.frame).and_then(|c| c.elements.first().cloned()));
    top.and_then(|e| fp_engine::docops::element_bounds(&d.doc, &e)).into_iter().collect()
}

fn element_count(h: &H) -> usize {
    let d = st(h).session.active().expect("doc");
    d.doc.pages[d.page].root.layers.iter().map(|l| match l {
        fp_model::LayerSlot::Own(l) => l.cels.iter().map(|c| c.elements.len()).sum(),
        _ => 0,
    }).sum()
}

fn import_harness(name: &str) -> (H, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("fp_cfc_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("Ordner");
    let png = dir.join("bild.png");
    fp_ui::os_clipboard::write_png(&png, 40, 20, &[90u8; 40 * 20 * 4]).expect("PNG");
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 400, "height": 300, "background": "#ffffff"})).expect("doc");
    let mut h = harness_with(s, &[]);
    h.state_mut().state.settings.workspace.show(PanelId::Layers);
    (h, png)
}

fn start_import(h: &mut H, png: &std::path::Path) {
    h.state_mut().state.file_override = Some(fp_ui::files::FilePickOverride { open: vec![png.to_path_buf()], save: None });
    h.state_mut().state.push(fp_ui::app_state::Action::Run("file.import".into(), json!({})));
    h.run_steps(3);
}

/// CT-C-05: Datei ▸ Importieren wartet nach der Dateiwahl auf den Platzierungszeiger: Klick setzt die
/// linke obere Ecke in Originalgröße, Aufziehen skaliert proportional ins Rechteck, Esc bricht ab.
/// K-7 (re8): das Rechteck umschließt den Zeiger (größeres Verhältnis führt, statt einpassen),
/// danach ist nichts ausgewählt; geprüft wird deshalb das oberste Objekt der Ebene.
#[test]
fn ct_c_05_import_uses_placement_pointer() {
    let (mut h, png) = import_harness("import");
    let undo0 = st(&h).session.active().expect("doc").history.undo_count();
    start_import(&mut h, &png);
    assert!(st(&h).place_import.is_some(), "Platzierungszeiger aktiv");
    assert_eq!(st(&h).session.active().expect("doc").history.undo_count(), undo0, "noch nichts eingefügt");
    // Klick bei 100,50
    let p = screen_of(&h, 100.0, 50.0);
    ev(&mut h, Event::PointerMoved(p));
    press(&mut h, p, true);
    press(&mut h, p, false);
    h.run_steps(3);
    assert!(st(&h).place_import.is_none());
    assert!(selected_bounds(&h).is_empty(), "nichts ausgewählt");
    let b = top_bounds(&h);
    assert_eq!(b.len(), 1, "ein Objekt");
    assert_eq!((b[0].left.round(), b[0].top.round(), b[0].width().round(), b[0].height().round()), (100.0, 50.0, 40.0, 20.0));
    assert_eq!(st(&h).session.active().expect("doc").history.undo_count(), undo0 + 1, "ein Verlaufsschritt");
    // Aufziehen 10,10 → 90,90: Zugende inklusiv (F27, re 41 §2.3) → 10,10 → 91,91; y führt
    // (81/20 > 81/40), Rechteck umschließt den Zeiger → 162 × 81
    start_import(&mut h, &png);
    let (a, b) = (screen_of(&h, 10.0, 10.0), screen_of(&h, 90.0, 90.0));
    drag(&mut h, a, b, 8);
    let b = top_bounds(&h);
    assert_eq!(b.len(), 1);
    assert_eq!((b[0].left.round(), b[0].top.round(), b[0].width().round(), b[0].height().round()), (10.0, 10.0, 162.0, 81.0));
    // Esc: abgebrochen, nichts eingefügt
    let n = element_count(&h);
    start_import(&mut h, &png);
    let p = screen_of(&h, 200.0, 200.0);
    ev(&mut h, Event::PointerMoved(p));
    ev(&mut h, Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
    ev(&mut h, Event::Key { key: egui::Key::Escape, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::NONE });
    assert!(st(&h).place_import.is_none(), "Esc bricht ab");
    let p = screen_of(&h, 200.0, 200.0);
    press(&mut h, p, true);
    press(&mut h, p, false);
    h.run_steps(2);
    assert_eq!(element_count(&h), n, "nichts eingefügt");
    let _ = std::fs::remove_dir_all(png.parent().expect("Ordner"));
}
/// K-9 (re 38 Modus 1): beim Aufziehen zeigt die Vorschau das seitentreue Rechteck des Imports
/// (sichtbare Pixel der Quelle, umschließt den Zeiger, größeres Verhältnis führt) statt des rohen
/// Zugrechtecks; der Import belegt danach genau dieses Rechteck.
#[test]
fn k9_import_drag_preview_is_aspect_true() {
    let (mut h, _) = import_harness("k9preview");
    // wie `par.png`: 160 × 120, sichtbare Pixel 10,10–151,111 (141 × 101)
    let png = std::env::temp_dir().join(format!("fp_cfc_k9_{}.png", std::process::id()));
    let mut px = vec![0u8; 160 * 120 * 4];
    for y in 10..111 {
        for x in 10..151 {
            px[(y * 160 + x) * 4..][..4].copy_from_slice(&[55, 155, 255, 255]);
        }
    }
    fp_ui::os_clipboard::write_png(&png, 160, 120, &px).expect("PNG");
    start_import(&mut h, &png);
    let vis = st(&h).place_import.as_ref().and_then(|pl| pl.visible).expect("sichtbare Grenzen beim Start gelesen");
    assert_eq!((vis.width(), vis.height()), (141.0, 101.0), "{vis:?}");
    let (a, b) = (screen_of(&h, 20.0, 20.0), screen_of(&h, 180.0, 120.0));
    ev(&mut h, Event::PointerMoved(a));
    press(&mut h, a, true);
    for i in 1..=6 {
        ev(&mut h, Event::PointerMoved(a + (b - a) * (i as f32 / 6.0)));
    }
    h.run_steps(1);
    let r = st(&h).place_import.as_ref().and_then(|pl| pl.preview).expect("Vorschau beim Ziehen");
    // x führt (160/141 > 100/101): Höhe 101 · 160/141 = 114,6 → unten 135 statt 120 (rohes Zugrechteck)
    assert!((r.left - 20.0).abs() <= 1.0 && (r.top - 20.0).abs() <= 1.0 && (r.right - 180.0).abs() <= 1.0, "{r:?}");
    assert!((r.bottom - 135.0).abs() <= 1.0, "seitentreu, nicht roh: {r:?}");
    // gezeichnet: ein Rechteck in Bildschirmgröße der Vorschau
    let (s0, s1) = (screen_of(&h, r.left, r.top), screen_of(&h, r.right, r.bottom));
    let col = st(&h).theme.colors.focus;
    fn walk(s: &Shape, col: egui::Color32, out: &mut Vec<Rect>) {
        match s {
            Shape::Rect(r) if (r.stroke.width - 1.0).abs() < 0.01 && r.stroke.color == col => out.push(r.rect),
            Shape::Vec(v) => v.iter().for_each(|x| walk(x, col, out)),
            _ => {}
        }
    }
    let mut rects = Vec::new();
    for cs in &h.output().shapes {
        walk(&cs.shape, col, &mut rects);
    }
    assert!(rects.iter().any(|q| (q.min - s0).length() < 1.5 && (q.max - s1).length() < 1.5), "Vorschau gezeichnet: {rects:?} / {s0:?}–{s1:?}");
    press(&mut h, b, false);
    h.run_steps(3);
    assert!(st(&h).place_import.is_none());
    // file.import bildet die ganze Bitmap auf genau dieses Rechteck ab (K-7): die sichtbaren
    // Pixel liegen darin, um den transparenten Rand eingerückt
    let t = top_bounds(&h);
    assert_eq!(t.len(), 1);
    assert!(t[0].left > r.left && t[0].top > r.top && t[0].right < r.right && t[0].bottom < r.bottom, "{t:?} in {r:?}");
    let _ = std::fs::remove_file(&png);
}

/// Fokusrahmen des Startbildschirms: Rechtecke mit 1,5 px Strich in der Fokusfarbe.
fn focus_rings(h: &H) -> Vec<Rect> {
    let col = st(h).theme.colors.focus;
    fn walk(s: &Shape, col: egui::Color32, out: &mut Vec<Rect>) {
        match s {
            Shape::Rect(r) if (r.stroke.width - 1.5).abs() < 0.01 && r.stroke.color == col => out.push(r.rect),
            Shape::Vec(v) => v.iter().for_each(|x| walk(x, col, out)),
            _ => {}
        }
    }
    let mut v = Vec::new();
    for cs in &h.output().shapes {
        walk(&cs.shape, col, &mut v);
    }
    v
}

fn key(h: &mut H, k: egui::Key) {
    for pressed in [true, false] {
        ev(h, Event::Key { key: k, physical_key: Some(k), pressed, repeat: false, modifiers: Modifiers::NONE });
    }
    h.run_steps(1);
}

/// CT-C-06: Startseite mit zuletzt geöffneten Dateien, Zeiger außerhalb, 5 × Tab: der fokussierte
/// Eintrag hat einen sichtbaren Rahmen; Pfeil ab bewegt Fokus und Rahmen zum nächsten Eintrag.
#[test]
fn ct_c_06_start_page_focus_ring_and_arrows() {
    fp_engine::shell::disable();
    let dir = std::env::temp_dir().join(format!("fp_cfc_welcome_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let mut s = fp_engine::Session::default();
    s.prefs.recent_files = (0..8).map(|i| dir.join(format!("datei{i}.svg"))).collect();
    let mut h = harness_with(s, &[]);
    ev(&mut h, Event::PointerGone);
    for _ in 0..5 {
        key(&mut h, egui::Key::Tab);
    }
    let focused = h.ctx.memory(|m| m.focused());
    assert!(focused.is_some(), "Fokus im Startbildschirm");
    let rings = focus_rings(&h);
    assert_eq!(rings.len(), 1, "genau ein sichtbarer Fokusrahmen: {rings:?}");
    let before = rings[0];
    key(&mut h, egui::Key::ArrowDown);
    assert_ne!(h.ctx.memory(|m| m.focused()), focused, "Pfeil ab bewegt den Fokus");
    let rings = focus_rings(&h);
    assert_eq!(rings.len(), 1, "{rings:?}");
    assert!(rings[0].top() > before.top(), "Rahmen wandert nach unten: {before:?} → {:?}", rings[0]);
}
/// CT-C-07 (kein Produktfehler, Original-Verhalten): Die Speichern-Rückfrage nennt den Namen wie das
/// Original ohne die letzte Endung („par_layers.fw“ statt „par_layers.fw.png“).
#[test]
fn ct_c_07_unsaved_prompt_drops_last_extension_like_original() {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 40, "height": 30})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [2, 2, 20, 20]})).expect("geändert");
    let mut h = harness_with(s, &[]);
    let id = st(&h).session.active_id().expect("doc");
    let title = st(&h).session.document(id).map(|d| d.title.clone()).expect("Titel");
    assert!(title.ends_with(".fw.png"), "{title}");
    h.state_mut().state.dialogs.unsaved = Some(fp_ui::dialogs::UnsavedFlow::new(vec![id], fp_ui::dialogs::PendingClose::CloseDocs));
    h.run_steps(3);
    let want = format!("Änderungen an {} speichern?", title.strip_suffix(".png").expect(".png"));
    let texts = painted_texts(&h);
    assert!(texts.iter().any(|(t, _)| *t == want), "{want} nicht gefunden: {:?}", texts.iter().filter(|(t, _)| t.contains("speichern")).collect::<Vec<_>>());
}
/// CT-C-09: Strg+A im Batch-Studio (ohne Textfeld-Fokus) schaltet alle Dateien ein, nie aus.
#[test]
fn ct_c_09_batch_ctrl_a_never_disables_all_files() {
    let dir = std::env::temp_dir().join(format!("fp_cfc_batch_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let files: Vec<std::path::PathBuf> = (0..3).map(|i| {
        let p = dir.join(format!("b{i}.png"));
        fp_ui::os_clipboard::write_png(&p, 8, 6, &[120u8; 8 * 6 * 4]).expect("PNG");
        p
    }).collect();
    let mut h = harness_with(fp_engine::Session::default(), &[]);
    fp_ui::batch::open(&mut h.state_mut().state);
    fp_ui::batch::studio_mut(&mut h.state_mut().state).expect("Studio").add_paths(&files, false);
    h.run_steps(4);
    let enabled = |h: &mut H| fp_ui::batch::studio_mut(&mut h.state_mut().state).expect("Studio").files.iter().filter(|f| f.enabled).count();
    assert_eq!(enabled(&mut h), 3);
    let ctrl_a = |h: &mut H| {
        for pressed in [true, false] {
            ev(h, Event::Key { key: egui::Key::A, physical_key: Some(egui::Key::A), pressed, repeat: false, modifiers: Modifiers::COMMAND });
        }
    };
    ctrl_a(&mut h);
    ctrl_a(&mut h);
    assert_eq!(enabled(&mut h), 3, "Strg+A schaltet nichts aus");
    fp_ui::batch::studio_mut(&mut h.state_mut().state).expect("Studio").files[1].enabled = false;
    ctrl_a(&mut h);
    assert_eq!(enabled(&mut h), 3, "Strg+A schaltet alle ein");
    let _ = std::fs::remove_dir_all(&dir);
}
/// CT-C-11: Fenster-Menü – kein ✓ an Untermenüs (Arbeitsbereichlayouts, Werkzeugleisten), und
/// „Symboleigenschaften“ nur mit ✓, wenn ihr Reiter vorn liegt (wie alle Bedienfelder).
#[test]
fn ct_c_11_window_menu_checks_follow_one_rule() {
    use fp_ui::menu::Node;
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 40, "height": 30})).expect("doc");
    let mut h = harness_with(s, &[]);
    let window = |h: &H| {
        let menus = fp_ui::menu::build_main_menu(st(h));
        menus.into_iter().find(|(l, _)| l == "Fenster").map(|(_, n)| n).expect("Fenster-Menü")
    };
    let nodes = window(&h);
    for n in &nodes {
        if let Node::Sub { label, children, .. } = n {
            assert_eq!(fp_ui::menu::sub_head_checked(&nodes, children), None, "kein ✓ an „{label}“");
        }
    }
    let checked = |nodes: &[Node], name: &str| nodes.iter().find_map(|n| match n {
        Node::Item { label, checked, .. } if label == name => Some(*checked == Some(true)),
        _ => None,
    });
    let sym = PanelId::SymbolProperties.title();
    assert_eq!(checked(&nodes, &sym), Some(false), "Symboleigenschaften: hinterer Reiter ohne ✓");
    h.state_mut().state.settings.workspace.show(PanelId::SymbolProperties);
    h.run_steps(2);
    assert_eq!(checked(&window(&h), &sym), Some(true), "vorn: ✓");
}

/// CT-C-13: Brücke – `ui_action {"panel": "window.properties"}` wirkt, `visible_panels` nennt nur
/// gezeichnete Reiter (nicht „Verlauf“ hinter „Optimieren“).
#[test]
fn ct_c_13_bridge_panels() {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 40, "height": 30})).expect("doc");
    let mut h = harness_with(s, &[PanelId::Optimize]);
    let ctx = h.ctx.clone();
    let vis = |h: &H| -> Vec<String> {
        let v = fp_ui::bridge::ui_state(&ctx, st(h));
        v["visible_panels"].as_array().expect("Liste").iter().filter_map(|x| x.as_str().map(str::to_string)).collect()
    };
    let ws = &st(&h).settings.workspace;
    assert!(ws.is_visible(PanelId::History) && !ws.is_front(PanelId::History), "Verlauf liegt hinter einem anderen Reiter");
    let v = vis(&h);
    assert!(v.iter().any(|p| p == "window.optimize"), "{v:?}");
    assert!(!v.iter().any(|p| p == "window.history"), "verdeckter Reiter nicht gemeldet: {v:?}");
    h.state_mut().state.settings.workspace.show_inspector = false;
    h.state_mut().state.sync_panels_to_engine();
    let r = fp_ui::bridge::ui_action(&ctx, &mut h.state_mut().state, &json!({"panel": "window.properties"}));
    assert!(!matches!(r, fp_mcp::UiReply::Error(_)), "window.properties bekannt");
    h.run_steps(2);
    assert!(st(&h).settings.workspace.show_inspector, "Eigenschaften eingeblendet");
    assert!(vis(&h).iter().any(|p| p == "window.properties"));
}
/// CT-C-17: „Vorlagen…“ im Dialog „Neues Dokument“ geht wie Datei ▸ Neu von Vorlage über den
/// Vorlagenordner (angelegt, mit den mitgelieferten Vorlagen) statt ohne Startordner.
#[test]
fn ct_c_17_templates_button_uses_template_folder() {
    let mut h = harness_with(fp_engine::Session::default(), &[]);
    h.state_mut().state.file_override = Some(fp_ui::files::FilePickOverride { open: vec![], save: None });
    h.state_mut().state.push(fp_ui::app_state::Action::Run("file.new".into(), json!({})));
    h.run_steps(3);
    assert!(st(&h).dialogs.new_document.is_some(), "Neues Dokument offen");
    let label = fp_i18n::t("dlg-new-doc-templates").replace('&', "");
    let r = text_rect(&h, |t| t == label);
    let p = r.center();
    ev(&mut h, Event::PointerMoved(p));
    for down in [true, false] {
        ev(&mut h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: down, modifiers: Modifiers::NONE });
    }
    h.run_steps(3);
    assert!(st(&h).dialogs.new_document.is_none(), "Dialog geschlossen");
    let dir = fp_ui::files::templates_dir().expect("Vorlagenordner");
    assert!(dir.is_dir(), "Vorlagenordner angelegt: {}", dir.display());
    let start = fp_ui::files::start_dir(&st(&h).session.prefs, fp_ui::files::DialogKind::NewFromTemplate, None);
    assert_eq!(start.as_deref(), Some(dir.as_path()), "Startordner = Vorlagenordner");
}
/// CT-C-08: Kartenkopf „Speichern als“ zeigt dieselbe Qualität, die geschrieben wird (WebP 82).
#[test]
fn ct_c_08_output_summary_quality_matches_encoder() {
    let mut r = fp_batch::Recipe::named("t");
    r.output.format = "webp".into();
    let s = fp_ui::batch::recipe::output_summary(&r);
    assert!(s.contains("82"), "{s}");
    r.output.format = "jpg".into();
    assert!(fp_ui::batch::recipe::output_summary(&r).contains("85"));
    assert_eq!(fp_batch::output::default_quality(fp_batch::OutputFormat::WebP), Some(82));
}

/// CT-C-12: Die Brücke meldet nur die gerade sichtbare Meldung; „nicht verfügbar“ nennt den
/// Menütext statt der Befehlskennung.
#[test]
fn ct_c_12_status_expires_and_names_menu_label() {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 40, "height": 30})).expect("doc");
    let mut h = harness_with(s, &[]);
    h.state_mut().state.status = Some(("alt".into(), -100.0));
    let now = h.ctx.input(|i| i.time);
    assert_eq!(st(&h).shown_status(now), None, "abgelaufene Meldung");
    h.state_mut().state.push(fp_ui::app_state::Action::Run("modify.mask.revealAll".into(), json!({})));
    h.run_steps(3);
    let now = h.ctx.input(|i| i.time);
    let msg = st(&h).shown_status(now).map(str::to_string).unwrap_or_default();
    assert!(!msg.contains("modify.mask.revealAll"), "keine Befehlskennung: {msg}");
    assert!(msg.is_empty() || msg.contains("nicht verfügbar"), "{msg}");
}

/// CT-C-19/21: Reiterbreiten werden anteilig gekürzt (kein Reiter nur „…“); neue schwebende
/// Bedienfelder liegen gestaffelt.
#[test]
fn ct_c_19_21_tabs_shrink_and_floating_staggered() {
    let nat = [80.0, 90.0, 40.0, 130.0];
    let w = fp_ui::docking::fit_tab_widths(&nat, Some(0), 250.0);
    assert_eq!(w[0], 80.0, "aktiver Reiter behält seine Breite");
    assert!(w.iter().all(|x| *x >= 28.0), "{w:?}");
    assert!(w.iter().sum::<f32>() <= 250.0 + 1.0, "passt: {w:?}");
    assert_eq!(fp_ui::docking::fit_tab_widths(&nat, Some(1), 400.0), nat.to_vec(), "genug Platz: unverändert");
    let mut ws = fp_ui::workspace::Workspace::expanded();
    for p in [PanelId::Info, PanelId::ColorMixer, PanelId::Url] {
        ws.closed.insert(p);
        ws.floating.retain(|f| !f.group.panels.contains(&p));
        ws.dock.iter_mut().for_each(|g| g.panels.retain(|q| *q != p));
    }
    ws.floating.clear();
    for p in [PanelId::Info, PanelId::ColorMixer, PanelId::Url] {
        ws.show(p);
    }
    let pos: Vec<[f32; 2]> = ws.floating.iter().map(|f| f.pos).collect();
    assert_eq!(pos.len(), 3);
    assert!(pos[0] != pos[1] && pos[1] != pos[2], "gestaffelt: {pos:?}");
}

/// CT-C-20: Ordner der Allgemeinen Bibliothek mit verständlichem Namen.
#[test]
fn ct_c_20_common_library_folder_name() {
    assert_eq!(fp_i18n::t("panel-commonlib-original"), "Standardbibliothek");
    fp_i18n::set_language("en");
    assert_eq!(fp_i18n::t("panel-commonlib-original"), "Standard library");
    fp_i18n::set_language("de");
}
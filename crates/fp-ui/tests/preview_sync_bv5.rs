//! N25 (BV-5): In der 2-/4-fach-Ansicht laufen alle Felder gleich mit – im Dokumentfenster
//! (Reiter „Vorschau/2-fach/4-fach“ über der Leinwand) und im Dialog „Bildvorschau“. Eine Lage
//! (Zoom + Dokumentpunkt in der Feldmitte) für alle Felder; jede Bewegung in einem Feld (Mausrad,
//! Strg+Rad, Leertaste+Ziehen, Hand, mittlere Taste, Zoomwerkzeug, Zoomliste) setzt sie für alle;
//! Umschalten 1/2/4 behält sie. Realistische Eingaben (Rad-Rastungen, Ziehen über viele Bilder,
//! Leertaste halten) nach AGENTEN_REGELN §4b/§4c.
//! Wie alle Testläufe mit `FP_NO_SHELL_OPEN=1` starten (AGENTEN_REGELN §4a).

use egui::epaint::Shape;
use egui::{vec2, Event, Modifiers, Pos2, Rect, Vec2};
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use fp_engine::view::PreviewMode;
use fp_engine::ToolId;
use fp_ui::app_state::{Action, AppState, UiSettings};
use fp_ui::dialogs::image_preview::{crop, toolbar, ImagePreview, Tool};
use fp_ui::theme::ThemeKind;
use fp_ui::FirepixelApp;
use serde_json::json;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Tests dieser Datei laufen nacheinander.
static LOCK: Mutex<()> = Mutex::new(());

struct Guard(#[allow(dead_code)] std::sync::MutexGuard<'static, ()>);

fn lock() -> Guard {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    fp_engine::shell::disable();
    Guard(g)
}

type H = Harness<'static, FirepixelApp>;

const DOC_W: i32 = 800;
const DOC_H: i32 = 600;

/// Bild 800 × 600 mit Verläufen: größer als jedes Feld, damit sich der Ausschnitt verschieben lässt.
fn session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": DOC_W, "height": DOC_H, "background": "#ffffff"})).expect("Dokument");
    let mut img = fp_raster::RgbaImage::filled(fp_geom::IRect::from_xywh(0, 0, DOC_W, DOC_H), fp_color::Rgba8::new(255, 255, 255, 255)).expect("Bild");
    let w = img.width();
    for (i, px) in img.data_mut().iter_mut().enumerate() {
        let (x, y) = ((i % w) as f64, (i / w) as f64);
        *px = [(x * 255.0 / DOC_W as f64) as u8, (y * 255.0 / DOC_H as f64) as u8, ((x + y) as u32 % 256) as u8, 255];
    }
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    s.execute("edit.paste", json!({})).expect("Bitmap einfügen");
    s.execute("select.none", json!({})).ok();
    s
}

fn harness() -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, session(), UiSettings { theme: ThemeKind::Normal, ..UiSettings::default() }, false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(8).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h
}

fn st(h: &mut H) -> &mut AppState {
    &mut h.state_mut().state
}

fn ev(h: &mut H, e: Event) {
    h.input_mut().events.push(e);
}

fn button(h: &mut H, p: Pos2, b: egui::PointerButton, pressed: bool, modifiers: Modifiers) {
    ev(h, Event::PointerButton { pos: p, button: b, pressed, modifiers });
}

fn click(h: &mut H, p: Pos2, modifiers: Modifiers) {
    ev(h, Event::PointerMoved(p));
    h.run_steps(1);
    button(h, p, egui::PointerButton::Primary, true, modifiers);
    h.run_steps(1);
    button(h, p, egui::PointerButton::Primary, false, modifiers);
    h.run_steps(2);
}

/// Ziehen wie ein Mensch: drücken, über 12 Bilder bewegen, loslassen.
fn drag_with(h: &mut H, from: Pos2, to: Pos2, b: egui::PointerButton) {
    ev(h, Event::PointerMoved(from));
    h.run_steps(1);
    button(h, from, b, true, Modifiers::NONE);
    h.run_steps(1);
    for i in 1..=12 {
        ev(h, Event::PointerMoved(from + (to - from) * (i as f32 / 12.0)));
        h.run_steps(1);
    }
    button(h, to, b, false, Modifiers::NONE);
    h.run_steps(2);
}

fn drag(h: &mut H, from: Pos2, to: Pos2) {
    drag_with(h, from, to, egui::PointerButton::Primary);
}

fn space(h: &mut H, pressed: bool) {
    ev(h, Event::Key { key: egui::Key::Space, physical_key: Some(egui::Key::Space), pressed, repeat: false, modifiers: Modifiers::NONE });
    h.run_steps(1);
}

/// Mausrad über `p`: `notches` Zeilen-Rastungen, je ein Ereignis pro Bild (Windows: 1 Zeile/Rastung
/// meldet egui als Line mit 1.0).
fn wheel(h: &mut H, p: Pos2, notches: i32, modifiers: Modifiers) {
    ev(h, Event::PointerMoved(p));
    h.run_steps(1);
    for _ in 0..notches.abs() {
        ev(h, Event::MouseWheel { unit: egui::MouseWheelUnit::Line, delta: vec2(0.0, notches.signum() as f32), modifiers, phase: egui::TouchPhase::Move });
        h.run_steps(1);
    }
    h.run_steps(1);
}

fn painted_images(h: &H, tex: egui::TextureId) -> Vec<Rect> {
    fn walk(s: &Shape, tex: egui::TextureId, out: &mut Vec<Rect>) {
        match s {
            Shape::Mesh(m) if m.texture_id == tex => out.push(m.calc_bounds()),
            Shape::Vec(v) => v.iter().for_each(|x| walk(x, tex, out)),
            _ => {}
        }
    }
    let mut v = Vec::new();
    for cs in &h.output().shapes {
        walk(&cs.shape, tex, &mut v);
    }
    v
}

// ------------------------------------------------------------------ Dokumentfenster

fn set_mode(h: &mut H, m: PreviewMode) {
    let key = match m {
        PreviewMode::Original => "ui-view-original",
        PreviewMode::Preview => "ui-view-preview",
        PreviewMode::TwoUp => "ui-view-2up",
        PreviewMode::FourUp => "ui-view-4up",
    };
    h.get_by_label(&fp_i18n::t(key)).click();
    h.run_steps(4);
    assert_eq!(view(h).preview_mode, m);
}

fn view(h: &mut H) -> fp_engine::ViewState {
    st(h).session.active().expect("Dokument").view.clone()
}

/// Dokumentpunkt in der Mitte des gemeinsamen Sichtbereichs.
fn center_doc(h: &mut H) -> Vec2 {
    let v = view(h);
    vec2((v.origin.0 + v.viewport.0 / 2.0 / v.zoom) as f32, (v.origin.1 + v.viewport.1 / 2.0 / v.zoom) as f32)
}

/// Felder (Rahmen mit Beschriftung) des Dokumentfensters, hergeleitet aus der Fläche der
/// Originalkachel (Feld 0 links oben, Lineale aus).
fn doc_panes(h: &mut H) -> Vec<Rect> {
    let id = st(h).session.active_id().expect("Dokument");
    let area = st(h).canvases.get(&id).and_then(|c| c.area).expect("Leinwandfläche");
    let mode = view(h).preview_mode;
    let pane = vec2(area.width(), area.height() + 20.0);
    let rect = match mode {
        PreviewMode::TwoUp => Rect::from_min_size(area.min, vec2(pane.x * 2.0 + 2.0, pane.y)),
        PreviewMode::FourUp => Rect::from_min_size(area.min, pane * 2.0 + vec2(2.0, 2.0)),
        _ => Rect::from_min_size(area.min, pane),
    };
    fp_ui::preview::pane_rects(rect, mode)
}

/// Lage der Seite relativ zur Feldmitte in jedem Feld (gezeichnet: Originalkachel = Leinwand,
/// sonst das Exportbild der Kachel) und der Zoom, mit dem gezeichnet wurde.
fn doc_offsets(h: &mut H) -> Vec<(Vec2, Vec2)> {
    h.run_steps(2);
    let id = st(h).session.active_id().expect("Dokument");
    let mode = view(h).preview_mode;
    let panes = doc_panes(h);
    let (d_doc, page) = {
        let d = st(h).session.active().expect("Dokument");
        (std::sync::Arc::clone(&d.doc), d.page)
    };
    let settings = fp_ui::preview::tile_settings(&fp_export::doc_export::page_settings(&d_doc, page), mode);
    // Die Exportbilder der Kacheln rechnet ein Hintergrund-Thread; unter Last (Gesamtlauf) sind sie
    // nach zwei Bildern evtl. noch nicht da. Darum weiterzeichnen, bis jedes Feld sein aktuelles
    // Exportbild zeigt (höchstens ≈ 20 s), und erst dann die Lage vergleichen.
    let mut missing = Vec::new();
    for attempt in 0..400 {
        missing.clear();
        let mut out = Vec::new();
        for (i, p) in panes.iter().enumerate() {
            let body = fp_ui::preview::pane_body(*p);
            let r = if i == 0 {
                st(h).canvases.get(&id).and_then(|c| c.page_screen)
            } else {
                let ctx = h.ctx.clone();
                fp_ui::preview::tile(&ctx, st(h), &settings[i - 1]).and_then(|t| t.texture).and_then(|tex| painted_images(h, tex.id()).first().copied())
            };
            match r {
                Some(r) => out.push((r.min - body.center(), r.size())),
                None => missing.push(i),
            }
        }
        if missing.is_empty() {
            return out;
        }
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        h.run_steps(1);
    }
    panic!("Feld {missing:?}: Exportbild nach 20 s nicht gezeichnet")
}

fn assert_doc_synced(h: &mut H, what: &str) -> Vec2 {
    let o = doc_offsets(h);
    for (i, (off, size)) in o.iter().enumerate().skip(1) {
        assert!((*off - o[0].0).length() <= 1.01, "{what}: Feld {i} zeigt einen anderen Ausschnitt als Feld 0: {o:?}");
        assert!((*size - o[0].1).length() <= 1.01, "{what}: Feld {i} hat einen anderen Zoom als Feld 0: {o:?}");
    }
    let z = view(h).zoom as f32;
    assert!((o[0].1.x - DOC_W as f32 * z).abs() <= 1.01, "{what}: Seitenbreite = Zoom × 800: {o:?} bei {z}");
    o[0].0
}

#[test]
fn doc_window_four_up_all_panes_follow_every_navigation() {
    let _g = lock();
    let mut h = harness();
    set_mode(&mut h, PreviewMode::FourUp);
    let panes = doc_panes(&mut h);
    assert_eq!(panes.len(), 4);
    let start = assert_doc_synced(&mut h, "4-fach geöffnet");
    let body = |k: usize| fp_ui::preview::pane_body(panes[k]);

    // Mausrad in Feld 1 (oben rechts): 3 Rastungen nach unten → alle Felder laufen mit
    wheel(&mut h, body(1).center(), -3, Modifiers::NONE);
    let after_wheel = assert_doc_synced(&mut h, "Mausrad in Feld 1");
    assert!((after_wheel.y - (start.y - 3.0 * 52.0)).abs() <= 1.01, "3 Rastungen = 156 px: {start:?} → {after_wheel:?}");

    // Leertaste halten + Ziehen in Feld 1 über viele Bilder
    space(&mut h, true);
    drag(&mut h, body(1).center(), body(1).center() + vec2(-60.0, 40.0));
    space(&mut h, false);
    let after_space = assert_doc_synced(&mut h, "Leertaste+Ziehen in Feld 1");
    assert!((after_space - (after_wheel + vec2(-60.0, 40.0))).length() <= 1.01, "gegriffener Punkt folgt dem Zeiger: {after_wheel:?} → {after_space:?}");

    // mittlere Taste in Feld 2 (unten links)
    drag_with(&mut h, body(2).center(), body(2).center() + vec2(30.0, -20.0), egui::PointerButton::Middle);
    let after_middle = assert_doc_synced(&mut h, "mittlere Taste in Feld 2");
    assert!((after_middle - (after_space + vec2(30.0, -20.0))).length() <= 1.01, "{after_space:?} → {after_middle:?}");

    // Hand-Werkzeug in Feld 3 (unten rechts)
    st(&mut h).push(Action::SetTool(ToolId::Hand));
    h.run_steps(2);
    drag(&mut h, body(3).center(), body(3).center() + vec2(25.0, 25.0));
    let after_hand = assert_doc_synced(&mut h, "Hand in Feld 3");
    assert!((after_hand - (after_middle + vec2(25.0, 25.0))).length() <= 1.01, "{after_middle:?} → {after_hand:?}");

    // Hand-Werkzeug in der Originalkachel (Feld 0) – die Vorschauen laufen mit
    drag(&mut h, body(0).center(), body(0).center() + vec2(-20.0, -30.0));
    let after_hand0 = assert_doc_synced(&mut h, "Hand in Feld 0");
    assert!((after_hand0 - (after_hand + vec2(-20.0, -30.0))).length() <= 1.01, "{after_hand:?} → {after_hand0:?}");

    // Strg+Rad in Feld 3: eine Stufe größer um den Zeiger; Dokumentpunkt unter dem Zeiger bleibt
    let z0 = view(&mut h).zoom;
    let p = body(3).center() + vec2(40.0, 30.0);
    let v0 = view(&mut h);
    let vmin = fp_ui::preview::virtual_min(&v0, body(3));
    let doc_before = v0.screen_to_doc((p.x - vmin.x) as f64, (p.y - vmin.y) as f64);
    wheel(&mut h, p, 1, Modifiers::CTRL);
    let z1 = view(&mut h).zoom;
    assert_eq!(z1, fp_engine::view::zoom_step_in(z0), "eine Rastung = eine Stufe");
    assert_doc_synced(&mut h, "Strg+Rad in Feld 3");
    let v1 = view(&mut h);
    let vmin = fp_ui::preview::virtual_min(&v1, body(3));
    let doc_after = v1.screen_to_doc((p.x - vmin.x) as f64, (p.y - vmin.y) as f64);
    assert!((doc_after.x - doc_before.x).abs() < 0.6 && (doc_after.y - doc_before.y).abs() < 0.6, "Punkt unter dem Zeiger fest: {doc_before:?} → {doc_after:?}");

    // Zoomwerkzeug in Feld 2: Klick größer, Alt+Klick kleiner
    st(&mut h).push(Action::SetTool(ToolId::Zoom));
    h.run_steps(2);
    click(&mut h, body(2).center(), Modifiers::NONE);
    assert_eq!(view(&mut h).zoom, fp_engine::view::zoom_step_in(z1), "Zoomklick in Feld 2");
    assert_doc_synced(&mut h, "Zoomwerkzeug in Feld 2");
    click(&mut h, body(2).center(), Modifiers::ALT);
    assert_eq!(view(&mut h).zoom, z1, "Alt+Zoomklick in Feld 2");
    assert_doc_synced(&mut h, "Alt+Zoomwerkzeug in Feld 2");

    // Zoomliste (Fußleiste): 200 % – Mitte bleibt, alle Felder gleich
    let c = center_doc(&mut h);
    st(&mut h).push(Action::SetZoom(2.0));
    h.run_steps(3);
    assert_eq!(view(&mut h).zoom, 2.0);
    assert_doc_synced(&mut h, "Zoomliste 200 %");
    assert!((center_doc(&mut h) - c).length() < 1.0, "Zoomliste hält die Mitte: {c:?}");
}

#[test]
fn doc_window_mode_switch_keeps_the_shared_position() {
    let _g = lock();
    let mut h = harness();
    // Original: weit hineinzoomen und verschieben
    st(&mut h).push(Action::SetZoom(2.0));
    h.run_steps(3);
    let id = st(&mut h).session.active_id().expect("Dokument");
    let area = st(&mut h).canvases.get(&id).and_then(|c| c.area).expect("Fläche");
    wheel(&mut h, area.center(), -2, Modifiers::NONE);
    let c0 = center_doc(&mut h);
    let z0 = view(&mut h).zoom;
    // Original → 4-fach → 2-fach → 4-fach → Vorschau → Original: Zoom und Mittelpunkt bleiben
    for m in [PreviewMode::FourUp, PreviewMode::TwoUp, PreviewMode::FourUp, PreviewMode::Preview, PreviewMode::TwoUp, PreviewMode::Original] {
        set_mode(&mut h, m);
        h.run_steps(2);
        assert_eq!(view(&mut h).zoom, z0, "{m:?}: Zoom bleibt");
        let c = center_doc(&mut h);
        assert!((c - c0).length() <= 1.0, "{m:?}: Mittelpunkt bleibt {c0:?}, ist {c:?}");
        if matches!(m, PreviewMode::TwoUp | PreviewMode::FourUp) {
            assert_doc_synced(&mut h, &format!("{m:?} nach Umschalten"));
        }
    }
    // im 2-fach bewegen, dann auf 4-fach: Lage bleibt und alle vier Felder zeigen sie
    set_mode(&mut h, PreviewMode::TwoUp);
    let panes = doc_panes(&mut h);
    space(&mut h, true);
    drag(&mut h, fp_ui::preview::pane_body(panes[1]).center(), fp_ui::preview::pane_body(panes[1]).center() + vec2(50.0, 30.0));
    space(&mut h, false);
    assert_doc_synced(&mut h, "Leertaste+Ziehen im 2-fach");
    let c1 = center_doc(&mut h);
    assert!((c1 - c0).length() > 10.0, "verschoben: {c0:?} → {c1:?}");
    set_mode(&mut h, PreviewMode::FourUp);
    assert!((center_doc(&mut h) - c1).length() <= 1.0, "2 → 4: Lage bleibt");
    assert_doc_synced(&mut h, "2 → 4");
    // 50 Rastungen hin und her: kein Drift, keine doppelten Schritte (§4b)
    let panes = doc_panes(&mut h);
    let p = fp_ui::preview::pane_body(panes[3]).center();
    let before = center_doc(&mut h);
    for _ in 0..25 {
        wheel(&mut h, p, 1, Modifiers::NONE);
        wheel(&mut h, p, -1, Modifiers::NONE);
    }
    assert!((center_doc(&mut h) - before).length() <= 1.0, "hin und her ohne Drift");
    assert_doc_synced(&mut h, "nach 50 Rastungen");
}

// ------------------------------------------------------------------ Dialog „Bildvorschau“

fn dlg(h: &mut H) -> &mut ImagePreview {
    st(h).dialogs.preview.as_mut().expect("Bildvorschau offen")
}

fn open_dialog(h: &mut H) {
    st(h).push(Action::Run("file.imagePreview".into(), json!({})));
    h.run_steps(3);
    assert!(st(h).dialogs.preview.is_some(), "Dialog offen");
}

fn wait_idle(h: &mut H) {
    let t0 = Instant::now();
    loop {
        h.run_steps(1);
        let busy = st(h).dialogs.preview.as_ref().is_some_and(|d| d.is_busy());
        if !busy || t0.elapsed() > Duration::from_secs(90) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    h.run_steps(2);
}

/// Dokumentpunkt in der Mitte jedes sichtbaren Feldes, gemessen am gezeichneten Bild.
fn dialog_centers(h: &mut H) -> Vec<(usize, Vec2)> {
    h.run_steps(2);
    let d = dlg(h);
    let z = d.zoom();
    let lays = d.layout.clone();
    let mut out = Vec::new();
    for (i, lay) in lays {
        let d = dlg(h);
        let (o, f, _) = d.pane_map(i).unwrap_or_else(|| panic!("Feld {i}: kein Bild"));
        let tex = if d.panes[i].show_preview { d.panes[i].texture_id() } else { d.original_texture_id() }.expect("Textur");
        let r = *painted_images(h, tex).iter().find(|r| lay.image.intersects(**r)).unwrap_or_else(|| panic!("Feld {i}: Bild nicht gezeichnet"));
        out.push((i, o + (lay.image.center() - r.min) / z / f));
    }
    out
}

fn assert_dialog_synced(h: &mut H, what: &str) -> Vec2 {
    let c = dialog_centers(h);
    for (i, p) in &c[1..] {
        assert!((*p - c[0].1).length() <= 0.75, "{what}: Feld {i} zeigt einen anderen Dokumentpunkt als Feld {}: {c:?}", c[0].0);
    }
    c[0].1
}

#[test]
fn image_preview_dialog_panes_stay_in_sync_with_crop_scale_space_and_wheel() {
    let _g = lock();
    let mut h = harness();
    open_dialog(&mut h);
    h.get_by_label(&fp_i18n::t("image-preview-view-4")).click();
    h.run_steps(3);
    wait_idle(&mut h);
    let doc = (DOC_W as usize, DOC_H as usize);
    // Feld 1 mit Exportbereich, Feld 2 mit 50 % Skalierung, Feld 3 „Vorschau“ aus (Original)
    {
        let d = dlg(&mut h);
        d.panes[1].settings.crop = Some([200, 100, 700, 500]);
        crop::set_percent(&mut d.panes[2].settings, doc, 50.0);
        d.panes[3].show_preview = false;
    }
    h.run_steps(2);
    wait_idle(&mut h);
    let c0 = assert_dialog_synced(&mut h, "verschiedene Bilder");

    // Mausrad in Feld 1: 2 Rastungen nach unten → alle Felder
    let l1 = dlg(&mut h).layout.iter().find(|(k, _)| *k == 1).map(|(_, l)| *l).expect("Feld 1");
    wheel(&mut h, l1.image.center(), -2, Modifiers::NONE);
    let c1 = assert_dialog_synced(&mut h, "Mausrad in Feld 1");
    assert!(c1.y > c0.y + 50.0, "nach unten gerollt: {c0:?} → {c1:?}");

    // Zoomwerkzeug aktiv, Leertaste halten + Ziehen in Feld 1 verschiebt trotzdem (alle Felder)
    h.get_by_label(&fp_i18n::t("image-preview-tool-zoom")).click();
    h.run_steps(2);
    assert_eq!(dlg(&mut h).tool, Tool::Zoom);
    let zoom_before = dlg(&mut h).zoom_idx;
    space(&mut h, true);
    drag(&mut h, l1.image.center(), l1.image.center() + vec2(40.0, 30.0));
    space(&mut h, false);
    assert_eq!(dlg(&mut h).zoom_idx, zoom_before, "Leertaste+Ziehen zoomt nicht");
    let c2 = assert_dialog_synced(&mut h, "Leertaste+Ziehen in Feld 1");
    assert!((c2 - (c1 - vec2(40.0, 30.0))).length() <= 0.75, "Feld 1 (Maßstab 1): {c1:?} → {c2:?}");

    // Zoomklick in Feld 3: alle Felder 200 %, gleicher Punkt
    let l3 = dlg(&mut h).layout.iter().find(|(k, _)| *k == 3).map(|(_, l)| *l).expect("Feld 3");
    click(&mut h, l3.image.center() + vec2(10.0, 5.0), Modifiers::NONE);
    assert_eq!(dlg(&mut h).zoom_idx, toolbar::ZOOM_100 + 1);
    assert_dialog_synced(&mut h, "Zoomklick in Feld 3");

    // Strg+Rad in Feld 2 (Maßstab 50 %): eine Stufe größer, alle gleich
    let l2 = dlg(&mut h).layout.iter().find(|(k, _)| *k == 2).map(|(_, l)| *l).expect("Feld 2");
    wheel(&mut h, l2.image.center() + vec2(-20.0, 10.0), 1, Modifiers::CTRL);
    assert_eq!(dlg(&mut h).zoom_idx, toolbar::ZOOM_100 + 2, "eine Rastung = eine Stufe");
    assert_dialog_synced(&mut h, "Strg+Rad in Feld 2");

    // mittlere Taste in Feld 0
    let l0 = dlg(&mut h).layout.iter().find(|(k, _)| *k == 0).map(|(_, l)| *l).expect("Feld 0");
    drag_with(&mut h, l0.image.center(), l0.image.center() + vec2(-30.0, -10.0), egui::PointerButton::Middle);
    let c3 = assert_dialog_synced(&mut h, "mittlere Taste in Feld 0");

    // Umschalten 4 → 2 → 4: Lage bleibt
    let z = dlg(&mut h).zoom_idx;
    h.get_by_label(&fp_i18n::t("image-preview-view-2")).click();
    h.run_steps(3);
    wait_idle(&mut h);
    let c4 = assert_dialog_synced(&mut h, "2-fach");
    assert!((c4 - c3).length() <= 0.75, "4 → 2: {c3:?} → {c4:?}");
    h.get_by_label(&fp_i18n::t("image-preview-view-4")).click();
    h.run_steps(3);
    wait_idle(&mut h);
    let c5 = assert_dialog_synced(&mut h, "wieder 4-fach");
    assert!((c5 - c3).length() <= 0.75, "2 → 4: {c3:?} → {c5:?}");
    assert_eq!(dlg(&mut h).zoom_idx, z, "Zoom bleibt beim Umschalten");
}

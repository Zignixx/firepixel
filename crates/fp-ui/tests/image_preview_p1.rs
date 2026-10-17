//! Bildvorschau, Paket P1 (re\17-bildvorschau.md §5, §6, §10 P1, BV-1): 1/2/4 Felder mit eigener
//! Exporteinstellung, aktives Feld bindet die linke Spalte, gemeinsamer Zoom und Bildlauf,
//! „Vorschau“ aus = Original, Kopfzeile nach der Formel aus §5.4, Abbruch der Hintergrundrechnung,
//! dieselbe Byte-Zahl wie der echte Export, alle drei Designs in Deutsch und Englisch.
//! Wie alle Testläufe mit `FP_NO_SHELL_OPEN=1` starten (AGENTEN_REGELN §4a).

use egui::accesskit::Role;
use egui::epaint::Shape;
use egui::{pos2, vec2, Pos2, Rect};
use egui_kittest::kittest::{By, Queryable};
use egui_kittest::Harness;
use fp_export::options::{builtin_presets, ColorMode, ExportFormat, OptimizeSettings, PaletteKind};
use fp_ui::app_state::{Action, AppState, UiSettings};
use fp_ui::dialogs::image_preview::{self, pane, toolbar, worker, ImagePreview, Tool, ViewMode};
use fp_ui::dialogs::{self};
use fp_ui::theme::ThemeKind;
use fp_ui::FirepixelApp;
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Sprache, Design und die Zähler des Workers sind global: Tests dieser Datei laufen nacheinander.
static LOCK: Mutex<()> = Mutex::new(());

struct Guard(#[allow(dead_code)] std::sync::MutexGuard<'static, ()>);

impl Drop for Guard {
    fn drop(&mut self) {
        worker::hold_jobs(false);
        fp_i18n::set_language("de");
    }
}

fn lock() -> Guard {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    fp_i18n::set_language("de");
    fp_engine::shell::disable();
    Guard(g)
}

type H = Harness<'static, FirepixelApp>;

/// Foto-artiges Bild 240 × 160 (Verläufe, Kreis) auf weißer Leinwand: GIF, JPEG und PNG haben
/// deutlich verschiedene Größen.
fn session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 240, "height": 160, "background": "#ffffff"})).expect("Dokument");
    // neue Dokumente exportieren als PNG 32 (BA-033); diese Tests prüfen die GIF-Ansicht
    s.execute("pi.export.settings", json!({"settings": serde_json::to_value(fp_model::ExportSettings::default()).expect("json")})).expect("GIF");
    let mut img = fp_raster::RgbaImage::filled(fp_geom::IRect::from_xywh(0, 0, 240, 160), fp_color::Rgba8::new(255, 255, 255, 255)).expect("Bild");
    let w = img.width();
    for (i, px) in img.data_mut().iter_mut().enumerate() {
        let (x, y) = ((i % w) as f64, (i / w) as f64);
        let d = (x - 150.0).hypot(y - 70.0);
        *px = [(x * 255.0 / 240.0) as u8, (y * 255.0 / 160.0) as u8, (255.0 - d.min(255.0)) as u8, 255];
    }
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    s.execute("edit.paste", json!({})).expect("Bitmap einfügen");
    s.execute("select.none", json!({})).ok();
    s
}

fn harness_with(w: f32, h: f32, theme: ThemeKind, wgpu: bool) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, session(), UiSettings { theme, ..UiSettings::default() }, false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let b = Harness::builder().renderer(crate::gpu_common::renderer()).with_size(vec2(w, h)).with_pixels_per_point(1.0).with_max_steps(8).with_step_dt(1.0 / 60.0);
    let b = if wgpu { b.renderer(crate::gpu_common::renderer()) } else { b };
    let mut h = b.build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h
}

fn harness() -> H {
    harness_with(1400.0, 900.0, ThemeKind::Normal, false)
}

fn st(h: &mut H) -> &mut AppState {
    &mut h.state_mut().state
}

fn dlg(h: &mut H) -> &mut ImagePreview {
    st(h).dialogs.preview.as_mut().expect("Bildvorschau offen")
}

fn open(h: &mut H) {
    st(h).push(Action::Run("file.imagePreview".into(), json!({})));
    h.run_steps(3);
    assert!(st(h).dialogs.preview.is_some(), "Dialog offen");
}

/// Bilder zeichnen, bis alle sichtbaren Felder fertig gerechnet sind.
fn wait_idle(h: &mut H) {
    let t0 = Instant::now();
    loop {
        h.run_steps(1);
        let busy = st(h).dialogs.preview.as_ref().is_some_and(|d| d.is_busy());
        if !busy || t0.elapsed() > Duration::from_secs(60) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    h.run_steps(2);
}

fn click_at(h: &mut H, p: Pos2, modifiers: egui::Modifiers) {
    h.event(egui::Event::PointerMoved(p));
    h.run_steps(1);
    h.event(egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: true, modifiers });
    h.run_steps(1);
    h.event(egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: false, modifiers });
    h.run_steps(2);
}

/// Ziehen wie ein Mensch: drücken, über mehrere Bilder bewegen, loslassen.
fn drag(h: &mut H, from: Pos2, to: Pos2) {
    h.event(egui::Event::PointerMoved(from));
    h.run_steps(1);
    h.event(egui::Event::PointerButton { pos: from, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() });
    h.run_steps(1);
    for i in 1..=8 {
        h.event(egui::Event::PointerMoved(from + (to - from) * (i as f32 / 8.0)));
        h.run_steps(1);
    }
    h.event(egui::Event::PointerButton { pos: to, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
    h.run_steps(2);
}

fn layout(h: &mut H, i: usize) -> pane::PaneLayout {
    dlg(h).layout.iter().find(|(k, _)| *k == i).map(|(_, l)| *l).unwrap_or_else(|| panic!("Feld {i} nicht gezeichnet"))
}

/// Alle gezeichneten Texte (mit Lage).
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

/// Texte des Dialogs: alles ab dem Titel in der Titelleiste (der modale Dialog liegt über dem
/// Hauptfenster und wird nach ihm gezeichnet).
fn dialog_texts(h: &H, rect: Rect) -> Vec<(String, Rect)> {
    let all = painted_texts(h);
    let title = fp_i18n::t("dlg-preview-title");
    let start = all.iter().position(|(s, r)| *s == title && rect.contains(r.center()) && r.center().y < rect.top() + dialogs::DIALOG_TITLE_H).unwrap_or(all.len());
    all[start..].to_vec()
}

/// Gezeichnete Bilder je Textur (Bildschirmrechteck).
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

/// Kombifeld der linken Spalte (Format) – das erste links der Vorschaufläche.
fn left_format_combo(h: &mut H) -> (Rect, String) {
    let area_left = layout(h, 0).frame.left();
    let mut v: Vec<(Rect, String)> = h.query_all(By::new().role(Role::ComboBox)).map(|n| (n.rect(), n.value().unwrap_or_default())).filter(|(r, _)| r.right() < area_left).collect();
    v.sort_by(|a, b| a.0.top().total_cmp(&b.0.top()));
    v.into_iter().next().expect("Formatliste der linken Spalte")
}

fn choose_left_format(h: &mut H, name: &str) {
    let (r, _) = left_format_combo(h);
    click_at(h, r.center(), Default::default());
    h.get_by_label(name).click();
    h.run_steps(3);
}

fn renderer(st: &AppState) -> fp_ui::panels::optimize::ExportRenderer {
    fp_ui::panels::optimize::ExportRenderer { normal: Arc::clone(&st.renderer) }
}

// ---------------------------------------------------------------------- Tests

#[test]
fn modes_one_two_four_layout_and_new_panes_copy_settings() {
    let _g = lock();
    let mut h = harness();
    open(&mut h);
    assert_eq!((dlg(&mut h).mode, dlg(&mut h).active, dlg(&mut h).zoom_idx, dlg(&mut h).tool), (ViewMode::One, 0, toolbar::ZOOM_100, Tool::Pointer), "Start: 1-fach, Feld 0, 100 %, Zeiger");
    // Start- = Mindestgröße (§1.1: Client 698 × 582)
    let r = dialogs::dialog_rect(&h.ctx, image_preview::DIALOG_ID).expect("gezeichnet");
    assert!((r.width() - 700.0).abs() < 1.5 && (r.height() - (582.0 + dialogs::DIALOG_TITLE_H + 2.0)).abs() < 1.5, "Startgröße {r:?}");
    let doc_settings = dlg(&mut h).panes[0].settings.clone();
    // Feld 0 auf JPEG, dann 2-fach: das neue untere Feld übernimmt die Einstellung und wird aktiv
    dlg(&mut h).panes[0].settings = builtin_presets()[5].settings.clone();
    h.get_by_label(&fp_i18n::t("image-preview-view-2")).click();
    h.run_steps(3);
    assert_eq!((dlg(&mut h).mode, dlg(&mut h).active), (ViewMode::Two, 1));
    let p0 = dlg(&mut h).panes[0].settings.clone();
    assert_eq!(dlg(&mut h).panes[1].settings, p0);
    let (a, b) = (layout(&mut h, 0), layout(&mut h, 1));
    assert!((a.frame.left() - b.frame.left()).abs() < 0.5 && b.frame.top() > a.frame.bottom() && (b.frame.top() - a.frame.bottom() - 2.0).abs() < 0.5, "2-fach übereinander: {a:?} {b:?}");
    assert!(!a.header_below && b.header_below, "Kopfzeile oben bzw. unter dem Bild");
    // Feld 1 ändern, 4-fach: neue Felder 2/3 übernehmen die Einstellung des aktiven Feldes (1)
    dlg(&mut h).panes[1].settings = builtin_presets()[9].settings.clone();
    h.get_by_label(&fp_i18n::t("image-preview-view-4")).click();
    h.run_steps(3);
    assert_eq!((dlg(&mut h).mode, dlg(&mut h).active), (ViewMode::Four, 3), "4-fach: unten rechts aktiv");
    assert_eq!(dlg(&mut h).panes[2].settings.color_mode, ColorMode::Rgba32);
    assert_eq!(dlg(&mut h).panes[0].settings.format, ExportFormat::Jpeg, "Feld 0 behält seine Einstellung");
    let l: Vec<Rect> = (0..4).map(|i| layout(&mut h, i).frame).collect();
    assert!(l[0].left() == l[1].left() && l[2].left() == l[3].left() && l[0].top() == l[2].top() && l[1].top() == l[3].top(), "2 × 2: {l:?}");
    assert!(l[2].left() > l[0].right() && l[1].top() > l[0].bottom(), "Nummern 0 oben links, 1 unten links, 2 oben rechts, 3 unten rechts");
    for i in 0..4 {
        assert!(layout(&mut h, i).compact);
        for j in 0..i {
            assert!(!l[i].intersects(l[j]), "Felder {i}/{j} überlappen");
        }
    }
    // zurück auf 1-fach: Feld 0 aktiv; ausgeblendete Felder behalten ihre Einstellung
    h.get_by_label(&fp_i18n::t("image-preview-view-1")).click();
    h.run_steps(3);
    assert_eq!((dlg(&mut h).mode, dlg(&mut h).active), (ViewMode::One, 0));
    assert_eq!(dlg(&mut h).panes[3].settings.color_mode, ColorMode::Rgba32);
    // Abbrechen: nichts ins Dokument; erneutes Öffnen beginnt wieder 1-fach mit der Dokumenteinstellung
    let doc0 = Arc::clone(&st(&mut h).session.active().expect("doc").doc);
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    assert!(st(&mut h).dialogs.preview.is_none(), "Esc schließt");
    assert!(*st(&mut h).session.active().expect("doc").doc == *doc0, "Abbrechen ändert nichts");
    open(&mut h);
    assert_eq!(dlg(&mut h).mode, ViewMode::One);
    assert_eq!(dlg(&mut h).panes[0].settings, doc_settings, "nichts vom letzten Dialog gemerkt");
}

#[test]
fn every_pane_has_own_settings_and_active_pane_binds_left_column() {
    let _g = lock();
    let mut h = harness();
    open(&mut h);
    h.get_by_label(&fp_i18n::t("image-preview-view-4")).click();
    h.run_steps(3);
    wait_idle(&mut h);
    // Klick in Feld 0 (echte Maus) aktiviert es
    let c0 = layout(&mut h, 0).image.center();
    click_at(&mut h, c0, Default::default());
    assert_eq!(dlg(&mut h).active, 0);
    // Format über die linke Spalte ändern: wirkt nur auf Feld 0
    choose_left_format(&mut h, "JPEG");
    assert_eq!(dlg(&mut h).panes[0].settings.format, ExportFormat::Jpeg);
    for i in 1..4 {
        assert_eq!(dlg(&mut h).panes[i].settings.format, ExportFormat::Gif, "Feld {i} unverändert");
    }
    wait_idle(&mut h);
    // Feld 3 aktivieren: die linke Spalte zeigt dessen Format (GIF), Feld 2 auf PNG 8 stellen
    let c3 = layout(&mut h, 3).image.center();
    click_at(&mut h, c3, Default::default());
    assert_eq!(dlg(&mut h).active, 3);
    assert_eq!(left_format_combo(&mut h).1, "GIF");
    let c2 = layout(&mut h, 2).image.center();
    click_at(&mut h, c2, Default::default());
    choose_left_format(&mut h, "PNG 8");
    assert_eq!((dlg(&mut h).panes[2].settings.format, dlg(&mut h).panes[2].settings.color_mode), (ExportFormat::Png, ColorMode::Indexed));
    click_at(&mut h, c0, Default::default());
    assert_eq!(left_format_combo(&mut h).1, "JPEG", "Feld 0 behält JPEG");
    wait_idle(&mut h);
    // jede Kopfzeile zeigt ihr eigenes Format; die Größen sind echte Kodierungen
    let texts: Vec<String> = painted_texts(&h).into_iter().map(|t| t.0).collect();
    for (i, f) in [(0usize, "JPEG"), (1, "GIF"), (2, "PNG"), (3, "GIF")] {
        let p = &dlg(&mut h).panes[i];
        let res = p.result.clone().expect("Ergebnis");
        assert_eq!(pane::format_label(&p.settings), f);
        assert!(res.bytes > 0 && res.error.is_none(), "Feld {i}: {res:?}");
        assert!(texts.contains(&pane::size_text(res.bytes)), "Feld {i}: Größentext {} nicht gezeichnet", pane::size_text(res.bytes));
    }
    let sizes: Vec<usize> = (0..3).map(|i| dlg(&mut h).panes[i].result.as_ref().map(|r| r.bytes).unwrap_or(0)).collect();
    assert!(sizes[0] != sizes[1] && sizes[1] != sizes[2], "verschiedene Formate, verschiedene Größen: {sizes:?}");
    // Vorgabenliste eines Feldes: Auswahl lädt die Vorgabe in genau dieses Feld und aktiviert es
    let entries = pane::preset_entries(&[]);
    let k = entries.iter().position(|e| e.label == fp_i18n::t("panel-opt-preset-gif-web216")).expect("GIF Web 216");
    let f1 = layout(&mut h, 1).frame;
    let combos: Vec<Rect> = h.query_all(By::new().role(Role::ComboBox)).map(|n| n.rect()).filter(|r| f1.contains(r.center())).collect();
    let combos = if combos.is_empty() { panic!("Vorgabenliste in Feld 1") } else { combos };
    click_at(&mut h, combos[0].center(), Default::default());
    h.get_by_label(&entries[k].label).click();
    h.run_steps(3);
    assert_eq!(dlg(&mut h).active, 1, "Vorgabenliste macht das Feld aktiv");
    assert_eq!(dlg(&mut h).panes[1].settings.palette_kind, PaletteKind::Web216);
    assert_eq!(dlg(&mut h).panes[3].settings.palette_kind, PaletteKind::Adaptive);
    assert_eq!(pane::matching_preset(&entries, &dlg(&mut h).panes[1].settings), Some(k), "Listenname bei genauer Übereinstimmung");
    dlg(&mut h).panes[1].settings.lossy = 30;
    assert_eq!(pane::matching_preset(&entries, &dlg(&mut h).panes[1].settings), None, "nach Änderung leer");
    // OK übernimmt die Einstellung des aktiven Feldes (Feld 1 mit Verlust 30) in die Seite, ein Schritt
    let n0 = st(&mut h).session.active().map(|d| d.history.undo_count()).unwrap_or(0);
    wait_idle(&mut h);
    h.get_by_label(&fp_i18n::t("btn-ok")).click();
    h.run_steps(4);
    assert!(st(&mut h).dialogs.preview.is_none());
    let d = st(&mut h).session.active().expect("doc");
    let page = fp_export::doc_export::page_settings(&d.doc, d.page);
    assert_eq!((page.palette_kind, page.lossy), (PaletteKind::Web216, 30));
    assert_eq!(d.history.undo_count(), n0 + 1, "genau ein Verlaufsschritt");
}

#[test]
fn zoom_and_pan_act_on_all_panes_at_once() {
    let _g = lock();
    let mut h = harness();
    open(&mut h);
    h.get_by_label(&fp_i18n::t("image-preview-view-4")).click();
    h.run_steps(3);
    wait_idle(&mut h);
    // Zoomwerkzeug: Klick = eine Stufe größer um den Klickpunkt, Alt+Klick = kleiner
    h.get_by_label(&fp_i18n::t("image-preview-tool-zoom")).click();
    h.run_steps(2);
    assert_eq!(dlg(&mut h).tool, Tool::Zoom);
    let l2 = layout(&mut h, 2);
    let c_before = dlg(&mut h).center.expect("Mitte gesetzt");
    let p = l2.image.center() + vec2(20.0, 10.0);
    click_at(&mut h, p, Default::default());
    assert_eq!(dlg(&mut h).zoom_idx, toolbar::ZOOM_100 + 1, "200 %");
    assert_eq!(dlg(&mut h).active, 2, "Zoomklick aktiviert das Feld");
    // der Bildpunkt unter dem Zeiger bleibt stehen: alter Punkt = c + (20,10)/1, neu = c' + (20,10)/2
    let c_after = dlg(&mut h).center.expect("Mitte");
    let want = c_before + vec2(20.0, 10.0) - vec2(20.0, 10.0) / 2.0;
    assert!((c_after - want).length() < 0.6, "Klickpunkt fest: {c_after:?} statt {want:?}");
    click_at(&mut h, p, egui::Modifiers::ALT);
    assert_eq!(dlg(&mut h).zoom_idx, toolbar::ZOOM_100, "Alt+Klick: zurück auf 100 %");
    // Zoomliste: 400 %
    let zr = h.query_all(By::new().role(Role::ComboBox)).find(|n| n.value().as_deref() == Some("100%")).map(|n| n.rect()).expect("Zoomliste");
    click_at(&mut h, zr.center(), Default::default());
    h.get_by_label("400%").click();
    h.run_steps(3);
    assert_eq!(toolbar::ZOOMS[dlg(&mut h).zoom_idx].0, "400%");
    // Zeiger: Ziehen in Feld 1 verschiebt alle Felder gleich
    h.get_by_label(&fp_i18n::t("image-preview-tool-pointer")).click();
    h.run_steps(2);
    let c0 = dlg(&mut h).center.expect("Mitte");
    let l1 = layout(&mut h, 1);
    drag(&mut h, l1.image.center(), l1.image.center() + vec2(-40.0, -24.0));
    let c1 = dlg(&mut h).center.expect("Mitte");
    assert!((c1 - (c0 + vec2(40.0, 24.0) / 4.0)).length() < 0.6, "Ziehen um (-40,-24) bei 400 %: {c0:?} → {c1:?}");
    assert_eq!(dlg(&mut h).active, 1);
    // gezeichnet: jedes Feld zeigt denselben Bildpunkt in seiner Mitte, alle mit 400 %
    h.run_steps(2);
    let mut offsets = Vec::new();
    for i in 0..4 {
        let tex = dlg(&mut h).panes[i].texture_id().expect("Textur");
        let lay = layout(&mut h, i);
        let rects = painted_images(&h, tex);
        let r = rects.first().copied().unwrap_or_else(|| panic!("Feld {i}: kein Bild gezeichnet"));
        assert!((r.width() - 240.0 * 4.0).abs() < 1.0 && (r.height() - 160.0 * 4.0).abs() < 1.0, "Feld {i}: {r:?}");
        offsets.push(lay.image.center() - r.min);
    }
    for o in &offsets[1..] {
        assert!((*o - offsets[0]).length() < 0.6, "gleicher Ausschnitt in allen Feldern: {offsets:?}");
    }
    assert!((offsets[0] / 4.0 - c1).length() < 0.6, "Feldmitte = gemeinsamer Bildpunkt");
    // weit hinaus ziehen: der Ausschnitt bleibt im Bild (kein Bildlauf ins Leere)
    drag(&mut h, l1.image.center(), l1.image.center() + vec2(900.0, 900.0));
    let c2 = dlg(&mut h).center.expect("Mitte");
    let half = l1.image.size() / 2.0 / 4.0;
    assert!(c2.x >= half.x - 0.5 && c2.y >= half.y - 0.5, "begrenzt: {c2:?}");
    // Zoom und Ziehen ändern die Einstellungen nicht und starten keine neue Rechnung
    assert!(!dlg(&mut h).is_busy());
}

#[test]
fn preview_off_shows_the_original() {
    let _g = lock();
    let mut h = harness();
    open(&mut h);
    h.get_by_label(&fp_i18n::t("image-preview-view-2")).click();
    h.run_steps(3);
    // Feld 0 auf GIF Web 216 (sichtbares Raster), dann dessen „Vorschau“ abhaken
    dlg(&mut h).panes[0].settings = builtin_presets()[0].settings.clone();
    wait_idle(&mut h);
    let label = fp_i18n::t("image-preview-preview");
    let boxes: Vec<Rect> = h.query_all(By::new().role(Role::CheckBox).label(label.as_str())).map(|n| n.rect()).collect();
    assert_eq!(boxes.len(), 2, "ein Kästchen je Feld");
    let top = boxes.iter().copied().min_by(|a, b| a.top().total_cmp(&b.top())).expect("oben");
    click_at(&mut h, top.center(), Default::default());
    assert!(!dlg(&mut h).panes[0].show_preview && dlg(&mut h).panes[1].show_preview);
    assert_eq!(dlg(&mut h).active, 0, "Kästchen macht das Feld aktiv");
    wait_idle(&mut h);
    let orig = dlg(&mut h).original.clone().expect("Original gerechnet");
    let orig_img = orig.image.clone().expect("Bild");
    // das Original ist das gerenderte Dokument ohne Exportoptimierung
    let stt = st(&mut h);
    let d = stt.session.active().expect("doc");
    let plain = stt.renderer.render_rect(&d.doc, d.page, d.frame, fp_export::render::page_rect(&d.doc, d.page).expect("Seite")).image;
    assert_eq!(*orig_img, plain, "Original = gerendertes Dokument");
    let gif = dlg(&mut h).panes[0].result.clone().and_then(|r| r.image.clone()).expect("GIF-Ergebnis");
    assert_ne!(*gif, *orig_img, "Web 216 mit Raster weicht vom Original ab");
    // gezeichnet: Feld 0 zeigt die Textur des Originals; Format-, Paletten- und Größenzeile weg, Farben bleiben
    let ot = dlg(&mut h).original_texture_id().expect("Textur des Originals");
    let l0 = layout(&mut h, 0);
    assert!(painted_images(&h, ot).iter().any(|r| l0.image.contains(r.center())), "Original in Feld 0");
    let header: Vec<String> = painted_texts(&h).into_iter().filter(|(_, r)| l0.header.contains(r.center())).map(|t| t.0).collect();
    let p0 = &dlg(&mut h).panes[0];
    let ht = pane::header_text(&p0.settings, p0.result.as_deref());
    assert!(!header.contains(&ht.palette) && !header.contains(&ht.size) && !header.contains(&"GIF".to_string()), "ausgeblendet: {header:?}");
    assert!(header.contains(&ht.colors), "Farbzeile bleibt: {header:?}");
    // Vorgabenliste bleibt bedienbar; wieder anhaken zeigt die Vorschau
    click_at(&mut h, top.center(), Default::default());
    assert!(dlg(&mut h).panes[0].show_preview);
}

#[test]
fn header_texts_follow_spec_and_sizes_equal_real_export() {
    let _g = lock();
    // Formel §5.4 mit den Beispielen des Originals, Deutsch und Englisch
    assert_eq!(pane::size_text(460_854), "460.85K  74 s bei 56 KBit/s");
    assert_eq!(pane::size_text(48), "58 Byte  0 s bei 56 KBit/s");
    assert_eq!(pane::size_text(612_080), "612.08K  98 s bei 56 KBit/s");
    assert_eq!(pane::size_text(1_459_040), "1459.04K  234 s bei 56 KBit/s");
    fp_i18n::set_language("en");
    assert_eq!(pane::size_text(612_080), "612.08K  98 sec @56kbps");
    assert_eq!(pane::size_text(48), "58 bytes  0 sec @56kbps");
    fp_i18n::set_language("de");
    // Kopfzeilen der Vorgaben (Laufzeitbelege §8.1)
    let p = builtin_presets();
    let h = |s: &OptimizeSettings| {
        let t = pane::header_text(s, None);
        (t.format, t.palette, t.colors)
    };
    assert_eq!(h(&p[0].settings), ("GIF".into(), "Web 216".into(), "216 Farben".into()));
    assert_eq!(h(&p[5].settings), ("JPEG".into(), "Qualität 60 (2)".into(), "Millionen".into()));
    assert_eq!(h(&p[9].settings), ("PNG".into(), String::new(), "Millionen+Alpha".into()));
    assert_eq!(h(&p[6].settings), ("Animiertes GIF".into(), "Adaptiv".into(), "128 Farben".into()), "WebSnap-Adaptiv heißt in der Kopfzeile Adaptiv");
    let wbmp = OptimizeSettings { format: ExportFormat::Wbmp, color_mode: ColorMode::Indexed, palette_kind: PaletteKind::Grayscale, max_colors: 2, ..OptimizeSettings::default() };
    assert_eq!(h(&wbmp), ("WBMP".into(), "Schwarzweiß".into(), "2 Farben".into()), "Graustufen mit 2 Stufen = Schwarzweiß (Fehler des Originals korrigiert)");
    let bmp = OptimizeSettings { format: ExportFormat::Bmp, color_mode: ColorMode::Rgb24, ..OptimizeSettings::default() };
    assert_eq!(h(&bmp), ("BMP".into(), String::new(), "Millionen".into()));
    fp_i18n::set_language("en");
    assert_eq!(h(&p[5].settings), ("JPEG".into(), "Quality 60 (2)".into(), "Millions".into()));
    assert_eq!(h(&p[0].settings).2, "216 colors");
    fp_i18n::set_language("de");
    // Reihenfolge der Vorgabenliste wie im Original (§8.1), dann Trennstrich und Firepixel-Vorgaben
    let e = pane::preset_entries(&[]);
    let keys = ["panel-opt-preset-png32", "panel-opt-preset-jpeg-better", "panel-opt-preset-jpeg-smaller", "panel-opt-preset-gif-web216", "panel-opt-preset-gif-websnap256", "panel-opt-preset-gif-websnap128", "panel-opt-preset-gif-adaptive256", "panel-opt-preset-anim-gif-websnap128"];
    let labels: Vec<String> = e.iter().take(8).map(|x| x.label.clone()).collect();
    assert_eq!(labels, keys.iter().map(|k| fp_i18n::t(k)).collect::<Vec<_>>(), "preset_list_order_follows_spec");
    assert!(e[8].settings.is_none(), "Trennstrich nach den 8 Vorgaben des Originals");
    // im Dialog: BMP 24 und JPEG haben genau die Byte-Zahl des echten Exports (gleiche Kodierer)
    let mut hh = harness();
    open(&mut hh);
    for s in [bmp.clone(), p[5].settings.clone(), p[4].settings.clone(), p[0].settings.clone()] {
        dlg(&mut hh).panes[0].settings = s.clone();
        wait_idle(&mut hh);
        let res = dlg(&mut hh).panes[0].result.clone().expect("Ergebnis");
        let st = st(&mut hh);
        let d = st.session.active().expect("doc");
        let real = fp_export::doc_export::export_whole(&renderer(st), &d.doc, d.page, &s, &fp_export::export::EncodeExtras::default()).expect("Export");
        assert_eq!(res.bytes, real.len(), "{:?}: Vorschau = echter Export", s.format);
        if s.format == ExportFormat::Bmp {
            assert_eq!(res.bytes, 240 * 160 * 3 + 54);
        }
        let texts: Vec<String> = painted_texts(&hh).into_iter().map(|t| t.0).collect();
        let ht = pane::header_text(&s, Some(&res));
        for t in [&ht.format, &ht.colors, &ht.size] {
            assert!(texts.contains(t), "{:?}: „{t}“ nicht gezeichnet", s.format);
        }
    }
}

/// „+“ im Feld: Dialog „Vorgabenname“ vorbelegt mit der zuletzt gewählten Vorgabe; OK speichert
/// die Einstellung **dieses Feldes**, Esc schließt nur den kleinen Dialog.
#[test]
fn plus_saves_the_pane_settings_as_user_preset() {
    let _g = lock();
    let mut h = harness();
    open(&mut h);
    h.get_by_label(&fp_i18n::t("image-preview-view-2")).click();
    h.run_steps(3);
    let mut s = builtin_presets()[1].settings.clone();
    s.max_colors = 64;
    dlg(&mut h).panes[1].settings = s.clone();
    dlg(&mut h).panes[1].last_preset = Some(fp_i18n::t("panel-opt-preset-gif-websnap256"));
    let tip = fp_i18n::t("image-preview-save-current-tip");
    let f1 = layout(&mut h, 1).frame;
    let plus = h.query_all(By::new().label(tip.as_str())).map(|n| n.rect()).find(|r| f1.contains(r.center())).expect("„+“ in Feld 1");
    click_at(&mut h, plus.center(), Default::default());
    assert_eq!(dlg(&mut h).active, 1);
    assert_eq!(dlg(&mut h).preset_name.as_ref().map(|p| p.1.clone()), Some(fp_i18n::t("panel-opt-preset-gif-websnap256")), "vorbelegt");
    assert_eq!(dialogs::last_dialog_id(&h.ctx).as_deref(), Some(image_preview::PRESET_DIALOG_ID));
    // Esc schließt nur „Vorgabenname“
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    assert!(st(&mut h).dialogs.preview.is_some() && dlg(&mut h).preset_name.is_none(), "Bildvorschau bleibt offen");
    // erneut, Namen tippen, OK
    click_at(&mut h, plus.center(), Default::default());
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text("Meine 64".into()));
    h.run_steps(2);
    h.key_press(egui::Key::Enter);
    h.run_steps(4);
    assert!(dlg(&mut h).preset_name.is_none(), "geschlossen");
    assert!(st(&mut h).dialogs.preview.is_some(), "Enter im kleinen Dialog schließt die Bildvorschau nicht");
    let own = fp_ui::ui_commands::export_presets(&st(&mut h).session.prefs);
    let (_, e) = own.iter().find(|(n, _)| n == "Meine 64").expect("Vorgabe gespeichert");
    let back = OptimizeSettings::from_model(e, None);
    assert_eq!((back.max_colors, back.web_adaptive), (64, true), "Einstellung von Feld 1");
    // die Liste zeigt jetzt den eigenen Namen (genaue Übereinstimmung)
    let entries = pane::preset_entries(&own);
    let k = pane::matching_preset(&entries, &dlg(&mut h).panes[1].settings).expect("passt");
    assert_eq!(entries[k].label, "Meine 64");
    assert_eq!(dlg(&mut h).panes[1].last_preset.as_deref(), Some("Meine 64"));
}

#[test]
fn worker_cancels_outdated_jobs_and_ui_stays_fluid() {
    let _g = lock();
    let mut h = harness();
    worker::hold_jobs(true);
    let c0 = worker::counters();
    open(&mut h);
    h.run_steps(2);
    assert!(dlg(&mut h).panes[0].is_busy(), "Auftrag läuft (angehalten)");
    let k1 = dlg(&mut h).panes[0].job_key().expect("Auftrag");
    // neue Einstellung: der alte Auftrag wird abgebrochen, nach der Entprellung startet der neue
    dlg(&mut h).panes[0].settings = builtin_presets()[5].settings.clone();
    h.run_steps(1);
    let t0 = Instant::now();
    let mut k2 = None;
    while t0.elapsed() < Duration::from_secs(5) {
        let f0 = Instant::now();
        h.run_steps(1);
        assert!(f0.elapsed() < Duration::from_millis(1500), "Oberfläche blockiert nicht ({:?})", f0.elapsed());
        k2 = dlg(&mut h).panes[0].job_key().filter(|k| *k != k1);
        if k2.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(k2.is_some(), "neuer Auftrag nach Entprellung");
    // viele Änderungen schnell hintereinander (wie Tippen): nur der letzte Stand wird gerechnet
    for q in 10..40 {
        dlg(&mut h).panes[0].settings.jpeg.quality = q;
        h.run_steps(1);
    }
    worker::hold_jobs(false);
    wait_idle(&mut h);
    let c1 = worker::counters();
    assert!(c1.cancelled >= c0.cancelled + 2, "alte Aufträge abgebrochen: {c0:?} → {c1:?}");
    let p = &dlg(&mut h).panes[0];
    let res = p.result.clone().expect("Ergebnis");
    let st0 = st(&mut h);
    let d = st0.session.active().expect("doc");
    let want = worker::settings_key((d.id.0, d.generation), d.page, d.frame, &worker::JobKind::Export(Box::new(dlg(&mut h).panes[0].settings.clone())));
    assert_eq!(res.key, want, "angezeigt wird das Ergebnis des letzten Stands");
    assert_eq!(dlg(&mut h).panes[0].settings.jpeg.quality, 39);
    assert!(c1.started - c0.started < 30, "Entprellung: nicht jede Taste ein Auftrag ({} gestartet)", c1.started - c0.started);
    // Dialog schließen bricht laufende Aufträge ab
    worker::hold_jobs(true);
    dlg(&mut h).panes[0].settings.jpeg.quality = 77;
    h.run_steps(1);
    std::thread::sleep(Duration::from_millis(200));
    h.run_steps(2);
    let c2 = worker::counters();
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    worker::hold_jobs(false);
    let t0 = Instant::now();
    while worker::counters().cancelled == c2.cancelled && t0.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(worker::counters().cancelled > c2.cancelled, "Schließen bricht ab");
}

/// Alle Texte im Dialog, keine Rohschlüssel, keine Überläufe – 1/2/4-fach, drei Designs, de/en,
/// auch auf 800 × 600 (Dialog passt in das Fenster).
#[test]
fn layout_fits_in_all_modes_themes_and_languages() {
    let _g = lock();
    for (w, hh) in [(1400.0, 900.0), (800.0, 600.0)] {
        for lang in ["de", "en"] {
            for theme in [ThemeKind::Normal, ThemeKind::Dark, ThemeKind::Oled] {
                fp_i18n::set_language(lang);
                let mut h = harness_with(w, hh, theme, false);
                open(&mut h);
                for mode in [ViewMode::One, ViewMode::Two, ViewMode::Four] {
                    dlg(&mut h).set_mode(mode);
                    wait_idle(&mut h);
                    let _ = dialogs::take_overflows(&h.ctx);
                    h.run_steps(2);
                    let over: Vec<String> = dialogs::take_overflows(&h.ctx);
                    assert!(over.is_empty(), "{lang} {theme:?} {mode:?} {w}×{hh}: Überlauf {over:?}");
                    let rect = dialogs::dialog_rect(&h.ctx, image_preview::DIALOG_ID).expect("Dialog");
                    let screen = h.ctx.content_rect();
                    assert!(screen.expand(0.5).contains_rect(rect), "{lang} {theme:?} {w}×{hh}: Dialog {rect:?} größer als {screen:?}");
                    let texts = dialog_texts(&h, rect);
                    assert!(texts.len() > 10, "{lang} {theme:?} {mode:?}: Dialogtexte gefunden");
                    for (s, r) in texts {
                        assert!(!s.starts_with("image-preview-") && !s.starts_with("dlg-preview-"), "{lang}: Rohschlüssel {s}");
                        if r.is_positive() {
                            assert!(rect.expand(1.0).contains_rect(r), "{lang} {theme:?} {mode:?}: Text „{s}“ {r:?} ragt aus {rect:?}");
                        }
                    }
                    // Kopfzeilentexte bleiben in ihrem Feld
                    let lays = dlg(&mut h).layout.clone();
                    for (i, l) in &lays {
                        for (s, r) in dialog_texts(&h, rect).into_iter().filter(|(_, r)| l.header.contains(r.center())) {
                            assert!(l.frame.expand(0.5).contains_rect(r), "{lang} {theme:?} {mode:?} Feld {i}: „{s}“ {r:?} außerhalb {:?}", l.frame);
                        }
                    }
                }
            }
        }
    }
}

/// Bilder für die Doku: `docs\screenshots\bildvorschau\bv1_<modus>_<design>_<sprache>.png`
/// (braucht einen Grafikadapter: `cargo test -p fp-ui --test image_preview_p1 -- --ignored`).
#[test]
#[ignore]
fn screenshots() {
    let _g = lock();
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/screenshots/bildvorschau");
    let _ = std::fs::create_dir_all(&out);
    let cases: Vec<(ViewMode, ThemeKind, &str)> = vec![
        (ViewMode::One, ThemeKind::Normal, "de"),
        (ViewMode::Two, ThemeKind::Normal, "de"),
        (ViewMode::Four, ThemeKind::Normal, "de"),
        (ViewMode::One, ThemeKind::Normal, "en"),
        (ViewMode::Two, ThemeKind::Normal, "en"),
        (ViewMode::Four, ThemeKind::Normal, "en"),
        (ViewMode::Four, ThemeKind::Dark, "de"),
        (ViewMode::Four, ThemeKind::Oled, "de"),
        (ViewMode::Four, ThemeKind::Dark, "en"),
        (ViewMode::Four, ThemeKind::Oled, "en"),
        (ViewMode::Two, ThemeKind::Dark, "de"),
        (ViewMode::One, ThemeKind::Oled, "en"),
    ];
    for (mode, theme, lang) in cases {
        fp_i18n::set_language(lang);
        let mut h = harness_with(1100.0, 760.0, theme, true);
        open(&mut h);
        dlg(&mut h).set_mode(mode);
        // verschiedene Einstellungen je Feld wie im Original-Beispiel (GIF Web 216, PNG32, JPEG klein, Anim. GIF)
        let p = builtin_presets();
        let picks = [0usize, 5, 9, 6];
        for (k, &i) in mode.panes().iter().enumerate() {
            dlg(&mut h).panes[i].settings = p[picks[k % 4]].settings.clone();
        }
        dlg(&mut h).active = *mode.panes().first().unwrap_or(&0);
        wait_idle(&mut h);
        h.run_steps(3);
        let name = format!("bv1_{}_{}_{lang}.png", match mode { ViewMode::One => "1fach", ViewMode::Two => "2fach", ViewMode::Four => "4fach" }, match theme { ThemeKind::Normal => "normal", ThemeKind::Dark => "dunkel", _ => "oled" });
        match h.render() {
            Ok(img) => {
                let r = dialogs::dialog_rect(&h.ctx, image_preview::DIALOG_ID).expect("Dialog");
                let r = r.intersect(Rect::from_min_size(pos2(0.0, 0.0), vec2(img.width() as f32, img.height() as f32)));
                let crop = image::imageops::crop_imm(&img, r.left() as u32, r.top() as u32, r.width() as u32, r.height() as u32).to_image();
                crop.save(out.join(&name)).expect("Bild speichern");
            }
            Err(e) => {
                eprintln!("kein Grafikadapter: {e}");
                return;
            }
        }
    }
}

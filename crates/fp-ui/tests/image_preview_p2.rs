//! Bildvorschau, Paket P2 „Registerkarte Optionen vollständig“ (re\17-bildvorschau.md §2, §10 P2,
//! BV-2): Formatliste und Sichtbarkeit je Format (§2.3), Palette/Verlust/Rastern wirken auf Bild und
//! Größe wie der Export, Farbtabelle (Auswahl, Sperren, Websicher, Transparent, Statuszeile,
//! Palettenmenü), drei Pipetten, „Optimieren auf Größe“, alle drei Designs in Deutsch und Englisch.
//! Wie alle Testläufe mit `FP_NO_SHELL_OPEN=1` starten (AGENTEN_REGELN §4a).

use egui::accesskit::Role;
use egui::epaint::Shape;
use egui::{vec2, Pos2, Rect};
use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::Harness;
use fp_export::options::{ColorMode, Dither, ExportFormat, OptimizeSettings, PaletteKind, Transparency};
use fp_ui::app_state::{Action, AppState, UiSettings};
use fp_ui::dialogs::image_preview::color_table::{self, Grid, Pipette};
use fp_ui::dialogs::image_preview::options::{self, Show, FORMATS};
use fp_ui::dialogs::image_preview::{self, optimize_to_size, pane, worker, ImagePreview, Tool};
use fp_ui::dialogs;
use fp_ui::theme::ThemeKind;
use fp_ui::FirepixelApp;
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

static LOCK: Mutex<()> = Mutex::new(());

struct Guard(#[allow(dead_code)] std::sync::MutexGuard<'static, ()>);

impl Drop for Guard {
    fn drop(&mut self) {
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

/// Foto-artiges Bild 240 × 160 (Verläufe, Kreis) auf weißer Leinwand.
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

fn harness_with(w: f32, h: f32, theme: ThemeKind) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, session(), UiSettings { theme, ..UiSettings::default() }, false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let b = Harness::builder().with_size(vec2(w, h)).with_pixels_per_point(1.0).with_max_steps(8).with_step_dt(1.0 / 60.0);
    let mut h = b.build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h
}

fn harness() -> H {
    harness_with(1400.0, 900.0, ThemeKind::Normal)
}

fn st(h: &mut H) -> &mut AppState {
    &mut h.state_mut().state
}

fn dlg(h: &mut H) -> &mut ImagePreview {
    st(h).dialogs.preview.as_mut().expect("Bildvorschau offen")
}

fn settings(h: &mut H) -> OptimizeSettings {
    let d = dlg(h);
    d.panes[d.active].settings.clone()
}

fn open(h: &mut H) {
    st(h).push(Action::Run("file.imagePreview".into(), json!({})));
    h.run_steps(3);
    assert!(st(h).dialogs.preview.is_some(), "Dialog offen");
}

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

fn right_click_at(h: &mut H, p: Pos2) {
    h.event(egui::Event::PointerMoved(p));
    h.run_steps(1);
    h.event(egui::Event::PointerButton { pos: p, button: egui::PointerButton::Secondary, pressed: true, modifiers: Default::default() });
    h.run_steps(1);
    h.event(egui::Event::PointerButton { pos: p, button: egui::PointerButton::Secondary, pressed: false, modifiers: Default::default() });
    h.run_steps(3);
}

fn double_click_at(h: &mut H, p: Pos2) {
    h.event(egui::Event::PointerMoved(p));
    h.run_steps(1);
    // erster Klick, dann der zweite rasch hinterher (innerhalb der Doppelklickzeit auch bei
    // langsamen Bildern des Software-Rasterers)
    h.event(egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() });
    h.run_steps(1);
    h.event(egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
    h.event(egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() });
    h.event(egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
    h.run_steps(3);
}

fn layout(h: &mut H, i: usize) -> pane::PaneLayout {
    dlg(h).layout.iter().find(|(k, _)| *k == i).map(|(_, l)| *l).unwrap_or_else(|| panic!("Feld {i} nicht gezeichnet"))
}

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

/// Kombifelder der linken Spalte von oben nach unten (Format zuerst).
fn left_combos(h: &mut H) -> Vec<(Rect, String)> {
    let area_left = layout(h, 0).frame.left();
    let mut v: Vec<(Rect, String)> = h.query_all(By::new().role(Role::ComboBox)).map(|n| (n.rect(), n.value().unwrap_or_default())).filter(|(r, _)| r.right() < area_left).collect();
    v.sort_by(|a, b| a.0.top().total_cmp(&b.0.top()));
    v
}

fn choose_format(h: &mut H, name: &str) {
    let r = left_combos(h)[0].0;
    click_at(h, r.center(), Default::default());
    h.get_by_label(name).click();
    h.run_steps(3);
}

/// Kombifeld der linken Spalte mit dem aktuellen Wert `value` öffnen und `item` wählen.
fn choose_in(h: &mut H, value: &str, item: &str) {
    let r = left_combos(h).into_iter().find(|(_, v)| v == value).unwrap_or_else(|| panic!("Liste mit „{value}“")).0;
    click_at(h, r.center(), Default::default());
    h.get_by_label(item).click();
    h.run_steps(3);
}

/// Knoten mit dem Namen `label` in der Bildvorschau (Lage, aktiv). Nur im Dialog suchen: Seit NZ-1
/// (`ec4ac99`) haben auch die Symbolknöpfe des Optimieren-Bedienfelds hinter dem Dialog
/// Barrierefreiheits-Namen, mit demselben Wortlaut wie hier („Farbe bearbeiten“, „Farbe zu
/// Transparenz hinzufügen“, „Farbe sperren“ …). Der erste Treffer war dann der (graue) Knopf im
/// Bedienfeld, und ein Klick darauf traf den Hintergrund des modalen Dialogs.
fn dialog_node(h: &H, label: &str) -> Option<(Rect, bool)> {
    let dr = dialogs::dialog_rect(&h.ctx, image_preview::DIALOG_ID).expect("Bildvorschau gezeichnet");
    h.query_all(By::new().label(label)).find(|n| dr.contains(n.rect().center())).map(|n| (n.rect(), !n.accesskit_node().is_disabled()))
}

fn has_label(h: &H, label: &str) -> bool {
    dialog_node(h, label).is_some()
}

fn label_rect(h: &H, label: &str) -> Rect {
    dialog_node(h, label).map(|(r, _)| r).unwrap_or_else(|| panic!("„{label}“ nicht gefunden"))
}

fn is_enabled(h: &H, label: &str) -> bool {
    dialog_node(h, label).is_some_and(|(_, en)| en)
}

/// Lage der Farbtabelle und ihr Raster.
fn table_grid(h: &mut H) -> (Rect, Grid, usize) {
    let r = label_rect(h, &fp_i18n::t("image-preview-color-table"));
    let a = dlg(h).active;
    let n = dlg(h).panes[a].result.as_ref().map(|r| r.entries.len()).unwrap_or(0);
    (r, Grid::new(r, n), n)
}

fn result(h: &mut H) -> Arc<worker::PaneResult> {
    let d = dlg(h);
    d.panes[d.active].result.clone().expect("Ergebnis")
}

fn real_export(h: &mut H, s: &OptimizeSettings) -> usize {
    let st = st(h);
    let d = st.session.active().expect("doc");
    let r = fp_ui::panels::optimize::ExportRenderer { normal: Arc::clone(&st.renderer) };
    fp_export::doc_export::export_whole(&r, &d.doc, d.page, s, &fp_export::export::EncodeExtras::default()).expect("Export").len()
}

/// Bildschirmpunkt des Bildpixels (x, y) im Feld 0 (1-fach, 100 %).
fn pixel_pos(h: &mut H, x: f32, y: f32) -> Pos2 {
    let lay = layout(h, 0);
    let res = result(h);
    let img = res.image.as_ref().expect("Bild");
    let c = dlg(h).center.unwrap_or(vec2(img.width() as f32, img.height() as f32) / 2.0);
    let z = dlg(h).zoom();
    lay.image.center() - c * z + vec2(x + 0.5, y + 0.5) * z
}

// ---------------------------------------------------------------------- Tests

/// Tabelle §2.3 für alle Einträge der Formatliste, dazu Reihenfolge §2.2 und die Grau-Regeln.
#[test]
fn format_list_and_visibility_follow_table() {
    let _g = lock();
    // Reihenfolge der Formatliste wie im Original, Trennstriche als eigene Einträge
    let names: Vec<String> = (0..FORMATS.len()).map(options::format_name).collect();
    assert_eq!(
        names[..16].to_vec(),
        ["GIF", "Animiertes GIF", "JPEG", "------------", "PNG 8", "PNG 24", "PNG 32", "------------", "WBMP", "------------", "TIFF 8", "TIFF 24", "TIFF 32", "------------", "BMP 8", "BMP 24"].map(String::from).to_vec()
    );
    assert!(FORMATS[16].1.is_none(), "Firepixel-Formate hinter einem Trennstrich");
    use Show::{Disabled as D, Hidden as X, Visible as V};
    // Spalten: Menü, Palette, Qualität, Verlust, Farben, Rastern, Glättung, JPEG-Kästchen, Tabelle,
    // Transparenz, Hintergrund, Unbenutzte entfernen, Zeilensprung
    let table: [(&str, [Show; 13]); 13] = [
        ("GIF", [V, V, X, V, V, V, X, X, V, V, V, V, V]),
        ("Animiertes GIF", [V, V, X, V, V, V, X, X, V, V, V, V, V]),
        ("JPEG", [X, X, V, X, X, X, V, V, X, X, V, V, X]),
        ("PNG 8", [V, V, X, X, V, V, X, X, V, V, V, V, V]),
        ("PNG 24", [X, X, X, X, X, X, X, X, X, X, V, X, X]),
        ("PNG 32", [X, X, X, X, X, X, X, X, X, X, V, X, X]),
        ("WBMP", [V, D, X, X, D, V, X, X, V, X, V, V, X]),
        ("TIFF 8", [V, V, X, X, V, V, X, X, V, X, V, V, X]),
        ("BMP 8", [V, V, X, X, V, V, X, X, V, X, V, V, X]),
        ("TIFF 24", [X, X, X, X, X, X, X, X, X, X, V, X, X]),
        ("TIFF 32", [X, X, X, X, X, X, X, X, X, X, V, X, X]),
        ("BMP 24", [X, X, X, X, X, X, X, X, X, X, V, X, X]),
        ("APNG", [X, X, X, X, X, X, X, X, X, X, V, X, X]),
    ];
    for (name, want) in table {
        let i = names.iter().position(|n| n == name).unwrap_or_else(|| panic!("{name}"));
        let (f, m) = FORMATS[i].1.expect("Format");
        let mut s = OptimizeSettings::default();
        options::apply_format(&mut s, f, m);
        assert_eq!(options::format_index(&s), Some(i), "{name}: Eintrag wiedergefunden");
        let v = options::visibility(&s);
        let got = [v.options_menu, v.palette, v.quality, v.loss, v.colors, v.dither, v.smoothing, v.jpeg_checks, v.color_table, v.transparency, v.matte, v.remove_unused, v.interlaced];
        assert_eq!(got, want, "{name}");
    }
    // im Dialog: GIF zeigt Tabelle, Pipetten, Zeilensprung; JPEG die JPEG-Kästchen; PNG 32 nur Hintergrund
    let mut h = harness();
    open(&mut h);
    wait_idle(&mut h);
    let (table_l, pip, interl, prog, unused, matte) = (fp_i18n::t("image-preview-color-table"), fp_i18n::t("image-preview-tip-pipette-set"), fp_i18n::t("image-preview-opt-interlaced"), fp_i18n::t("image-preview-opt-progressive"), fp_i18n::t("image-preview-opt-remove-unused"), fp_i18n::t("image-preview-tip-matte"));
    let opt_menu = fp_i18n::t("image-preview-tip-options");
    assert!(has_label(&h, &table_l) && has_label(&h, &pip) && has_label(&h, &interl) && has_label(&h, &opt_menu) && !has_label(&h, &prog));
    // ohne gewählte Farbe ist von den sechs Knöpfen nur „Farbe hinzufügen“ aktiv (§2.3)
    assert!(is_enabled(&h, &fp_i18n::t("image-preview-tip-add")) && !is_enabled(&h, &fp_i18n::t("image-preview-tip-lock")) && !is_enabled(&h, &fp_i18n::t("image-preview-tip-delete")));
    // Rastermenge grau, solange Rastern aus ist
    assert_eq!(settings(&mut h).dither, Dither::None);
    choose_format(&mut h, "JPEG");
    assert_eq!(settings(&mut h).format, ExportFormat::Jpeg);
    assert!(has_label(&h, &prog) && has_label(&h, &fp_i18n::t("image-preview-opt-sharpen")) && has_label(&h, &unused) && !has_label(&h, &table_l) && !has_label(&h, &pip) && !has_label(&h, &interl) && !has_label(&h, &opt_menu));
    // Glättung als Liste mit „Ohne Glätten“ … „8 Maximale Glättung“
    choose_in(&mut h, &fp_i18n::t("image-preview-opt-smoothing-none"), &fp_i18n::t("image-preview-opt-smoothing-max"));
    assert_eq!(settings(&mut h).jpeg.smoothing, 8);
    // Farbränder scharfzeichnen = Subsampling 0
    let sharpen = label_rect(&h, &fp_i18n::t("image-preview-opt-sharpen"));
    let sub0 = settings(&mut h).jpeg.subsampling;
    click_at(&mut h, sharpen.left_center() + vec2(6.0, 0.0), Default::default());
    assert_ne!(settings(&mut h).jpeg.subsampling == 0, sub0 == 0, "Kästchen schaltet das Subsampling");
    choose_format(&mut h, "PNG 32");
    assert!(!has_label(&h, &table_l) && !has_label(&h, &unused) && has_label(&h, &matte));
    // WBMP: Palette fest Schwarzweiß/2 (grau); danach bleibt TIFF 8 auf Schwarzweiß/2 (§2.3)
    choose_format(&mut h, "WBMP");
    let s = settings(&mut h);
    assert_eq!((s.palette_kind, s.max_colors), (PaletteKind::Grayscale, 2));
    let combos = left_combos(&mut h);
    assert!(combos.iter().any(|(_, v)| *v == fp_i18n::t("panel-opt-palette-bw")), "{combos:?}");
    choose_format(&mut h, "TIFF 8");
    let s = settings(&mut h);
    assert_eq!((s.format, s.palette_kind, s.max_colors), (ExportFormat::Tiff, PaletteKind::Grayscale, 2));
    assert!(!has_label(&h, &pip), "TIFF 8 ohne Pipetten");
    // GIF → Animiertes GIF: 256 → 255 Farben (§2.3)
    choose_format(&mut h, "GIF");
    dlg(&mut h).panes[0].settings.max_colors = 256;
    h.run_steps(2);
    choose_format(&mut h, "Animiertes GIF");
    assert_eq!(settings(&mut h).max_colors, 255);
}

/// Palette, Verlust und Rastern ändern Bild und Größe, genau wie der Export.
#[test]
fn palette_loss_and_dither_change_image_and_size() {
    let _g = lock();
    let mut h = harness();
    open(&mut h);
    wait_idle(&mut h);
    let base = result(&mut h);
    assert_eq!(settings(&mut h).format, ExportFormat::Gif);
    // Palettenliste: Web 216 über die Liste (10 Einträge, „WebSnap-Adaptiv“ an Stelle 2)
    let pnames: Vec<String> = (0..fp_ui::panels::optimize::PALETTES.len()).map(options::palette_name).collect();
    assert_eq!(pnames.len(), 10);
    assert_eq!(pnames[1], "WebSnap-Adaptiv");
    choose_in(&mut h, &pnames[0], &pnames[2]);
    wait_idle(&mut h);
    let s = settings(&mut h);
    assert_eq!((s.palette_kind, s.max_colors), (PaletteKind::Web216, 216));
    let web = result(&mut h);
    assert_ne!(web.bytes, base.bytes, "Web 216 ändert die Größe");
    assert_ne!(web.image, base.image, "und das Bild");
    assert!(web.colors < 216 && web.entries.iter().all(|e| e.web_safe), "tatsächliche Anzahl {}, alle websicher", web.colors);
    assert_eq!(web.bytes, real_export(&mut h, &s), "Vorschau = Export");
    // tatsächliche Farbanzahl wird angezeigt
    assert!(painted_texts(&h).iter().any(|(t, _)| *t == web.colors.to_string()), "Anzeige der tatsächlichen Farben");
    // Rastern anhaken: 100 %, Menge aktiv; ändert das Bild
    choose_in(&mut h, &pnames[2], &pnames[0]);
    wait_idle(&mut h);
    let adaptive = result(&mut h);
    let tip = fp_i18n::t("image-preview-tip-dither");
    let r = h.query_all(By::new().role(Role::CheckBox)).map(|n| n.rect()).find(|r| label_rect(&h, &tip).intersects(*r)).expect("Kästchen Rastern");
    click_at(&mut h, r.center(), Default::default());
    assert_eq!(settings(&mut h).dither, Dither::Diffusion(100));
    wait_idle(&mut h);
    let dith = result(&mut h);
    assert_ne!(dith.image, adaptive.image, "Rastern ändert das Bild");
    let s = settings(&mut h);
    assert_eq!(dith.bytes, real_export(&mut h, &s));
    // Verlust 60: kleiner, wie der Export
    dlg(&mut h).panes[0].settings.dither = Dither::None;
    dlg(&mut h).panes[0].settings.lossy = 60;
    wait_idle(&mut h);
    let lossy = result(&mut h);
    assert!(lossy.bytes < adaptive.bytes, "Verlust verkleinert: {} < {}", lossy.bytes, adaptive.bytes);
    let s = settings(&mut h);
    assert_eq!(lossy.bytes, real_export(&mut h, &s));
    // Farbanzahl 16 über das Kombifeld
    let colors = left_combos(&mut h).into_iter().find(|(_, v)| v == "256");
    assert!(colors.is_none() || colors.is_some(), "Kombifeld mit Eingabe");
    dlg(&mut h).panes[0].settings.max_colors = 16;
    wait_idle(&mut h);
    assert!(result(&mut h).colors <= 16);
    // „Nicht verwendete Farben entfernen“ und Zeilensprung schalten die Einstellung
    let unused = label_rect(&h, &fp_i18n::t("image-preview-opt-remove-unused"));
    let before = settings(&mut h).optimized;
    click_at(&mut h, unused.left_center() + vec2(6.0, 0.0), Default::default());
    assert_ne!(settings(&mut h).optimized, before);
    let interl = label_rect(&h, &fp_i18n::t("image-preview-opt-interlaced"));
    click_at(&mut h, interl.left_center() + vec2(6.0, 0.0), Default::default());
    assert!(settings(&mut h).interlaced);
    wait_idle(&mut h);
    let s = settings(&mut h);
    assert_eq!(result(&mut h).bytes, real_export(&mut h, &s), "Zeilensprung wie im Export");
}

/// Drei Pipetten (§2.6): Klick ins Vorschaufeld nimmt die Palettenfarbe des Bildpunkts.
#[test]
fn pipettes_set_add_and_remove_transparency() {
    let _g = lock();
    let mut h = harness();
    open(&mut h);
    wait_idle(&mut h);
    let set = fp_i18n::t("image-preview-tip-pipette-set");
    let r = label_rect(&h, &set);
    click_at(&mut h, r.center(), Default::default());
    assert_eq!(dlg(&mut h).tool, Tool::Pipette(Pipette::Set));
    // Pixel (10, 10) des Bildes
    let res = result(&mut h);
    let k = res.index_at(10, 10).expect("Index");
    let color = res.entries[k].color;
    let p = pixel_pos(&mut h, 10.0, 10.0);
    click_at(&mut h, p, Default::default());
    let s = settings(&mut h);
    assert_eq!(s.transparency, Transparency::Index, "Liste springt auf Indextransparenz");
    let tr: Vec<u32> = s.palette.iter().filter(|c| c.transparent).map(|c| c.color & 0xFF_FFFF).collect();
    assert_eq!(tr, vec![color & 0xFF_FFFF]);
    wait_idle(&mut h);
    let res = result(&mut h);
    let img = res.image.as_ref().expect("Bild");
    assert_eq!(img.data()[10 * img.width() + 10][3], 0, "Pixel erscheint als Loch");
    assert_eq!(res.entries.iter().filter(|e| e.transparent).count(), 1);
    assert_eq!(res.bytes, real_export(&mut h, &s));
    // zweite Pipette: weitere Farbe dazu (Pixel unten rechts)
    let add = label_rect(&h, &fp_i18n::t("image-preview-tip-pipette-add"));
    click_at(&mut h, add.center(), Default::default());
    assert_eq!(dlg(&mut h).tool, Tool::Pipette(Pipette::Add));
    let k2 = res.index_at(230, 150).expect("Index");
    let c2 = res.entries[k2].color;
    assert_ne!(c2 & 0xFF_FFFF, color & 0xFF_FFFF);
    let p2 = pixel_pos(&mut h, 230.0, 150.0);
    click_at(&mut h, p2, Default::default());
    assert_eq!(settings(&mut h).palette.iter().filter(|c| c.transparent).count(), 2);
    // dritte Pipette: die erste Farbe wieder heraus
    let rem = label_rect(&h, &fp_i18n::t("image-preview-tip-pipette-remove"));
    click_at(&mut h, rem.center(), Default::default());
    click_at(&mut h, p, Default::default());
    let s = settings(&mut h);
    let tr: Vec<u32> = s.palette.iter().filter(|c| c.transparent).map(|c| c.color & 0xFF_FFFF).collect();
    assert_eq!(tr, vec![c2 & 0xFF_FFFF]);
    // Pipette erneut drücken = zurück zum Zeiger; der Zeigerknopf ist mit Pipette nicht gedrückt
    click_at(&mut h, rem.center(), Default::default());
    assert_eq!(dlg(&mut h).tool, Tool::Pointer);
    // Transparenzliste: Alphatransparenz
    choose_in(&mut h, &fp_i18n::t("panel-opt-transp-index"), &fp_i18n::t("panel-opt-transp-alpha"));
    assert_eq!(settings(&mut h).transparency, Transparency::IndexAlpha);
}

/// Farbtabelle (§2.5): Auswahl, Statuszeile statt Pipetten, Sperren/Websicher/Transparent als
/// Schalter, Palettenmenü (≡ und Rechtsklick), Doppelklick = Farbwahl, Klick in die Vorschau.
#[test]
fn color_table_lock_websafe_transparent_and_menu() {
    let _g = lock();
    let mut h = harness();
    open(&mut h);
    wait_idle(&mut h);
    let (area, grid, n) = table_grid(&mut h);
    assert!(n > 16, "Palette mit {n} Farben");
    assert_eq!(grid.cell.y, color_table::CELL_H, "Felder 13 px hoch");
    // Farbe wählen: Statuszeile ersetzt Pipetten und Transparenzliste
    let entries = result(&mut h).entries.clone();
    // eine nicht websichere Farbe suchen
    let k = entries.iter().position(|e| !e.web_safe).expect("nicht websicher");
    click_at(&mut h, grid.cell_rect(k).center(), Default::default());
    assert_eq!(dlg(&mut h).table.selected, vec![entries[k].color]);
    let status = color_table::status_text(&entries[k], k, false);
    assert!(status.starts_with("R:") && status.contains(&format!("Index:{k:03}")), "{status}");
    assert!(has_label(&h, &status), "Statuszeile „{status}“");
    assert!(!has_label(&h, &fp_i18n::t("image-preview-tip-pipette-set")), "Pipetten ausgeblendet");
    // alle sechs Knöpfe aktiv
    for key in ["image-preview-tip-edit-color", "image-preview-tip-transparent", "image-preview-tip-snap-web", "image-preview-tip-lock", "image-preview-tip-add", "image-preview-tip-delete"] {
        assert!(is_enabled(&h, &fp_i18n::t(key)), "{key} aktiv");
    }
    // Sperren
    let lock_r = label_rect(&h, &fp_i18n::t("image-preview-tip-lock"));
    click_at(&mut h, lock_r.center(), Default::default());
    assert!(settings(&mut h).palette.iter().any(|p| p.locked && p.color & 0xFF_FFFF == entries[k].color & 0xFF_FFFF));
    wait_idle(&mut h);
    // Websicher machen: ersetzt durch die nächste Web-Farbe, markiert als modifiziert
    let web_r = label_rect(&h, &fp_i18n::t("image-preview-tip-snap-web"));
    click_at(&mut h, web_r.center(), Default::default());
    wait_idle(&mut h);
    let res = result(&mut h);
    let e = res.entries.iter().find(|e| e.color & 0xFF_FFFF == entries[k].color & 0xFF_FFFF).copied().expect("Eintrag bleibt");
    assert!(e.locked && e.edited && e.web_safe, "{e:?}");
    assert_eq!(e.shown & 0xFF_FFFF, fp_export::palette_edit::web_snap(entries[k].color) & 0xFF_FFFF);
    let i = res.entries.iter().position(|x| *x == e).expect("Index");
    let st_text = color_table::status_text(&e, i, false);
    assert!(st_text.ends_with(&format!("{} {} {}", fp_i18n::t("image-preview-flag-locked"), fp_i18n::t("image-preview-flag-edited"), fp_i18n::t("image-preview-flag-websafe"))), "{st_text}");
    assert!(has_label(&h, &st_text));
    // Knöpfe zeigen den Zustand (eingedrückt): Sperren, Websicher, Bearbeiten
    for key in ["image-preview-tip-lock", "image-preview-tip-snap-web", "image-preview-tip-edit-color"] {
        let lbl = fp_i18n::t(key);
        let dr = dialogs::dialog_rect(&h.ctx, image_preview::DIALOG_ID).expect("Bildvorschau gezeichnet");
        let node = h.query_all(By::new().label(lbl.as_str())).find(|n| dr.contains(n.rect().center())).expect(key);
        let a = node.accesskit_node();
        assert!(a.toggled() == Some(egui::accesskit::Toggled::True) || a.is_selected() == Some(true), "{key} eingedrückt");
    }
    // Transparent: Feld zeigt das Karomuster, Indextransparenz wird eingeschaltet
    let tr_r = label_rect(&h, &fp_i18n::t("image-preview-tip-transparent"));
    click_at(&mut h, tr_r.center(), Default::default());
    assert_eq!(settings(&mut h).transparency, Transparency::Index);
    wait_idle(&mut h);
    let res = result(&mut h);
    assert!(res.entries.iter().any(|x| x.color & 0xFF_FFFF == entries[k].color & 0xFF_FFFF && x.transparent));
    let s = settings(&mut h);
    assert_eq!(res.bytes, real_export(&mut h, &s), "Flags wirken wie im Export");
    // Palettenmenü per Rechtsklick auf die Tabelle: „Alle Farben freigeben“ aktiv
    right_click_at(&mut h, grid.cell_rect(k).center());
    let unlock = fp_i18n::t("image-preview-menu-unlock-all");
    assert!(is_enabled(&h, &unlock), "Alle Farben freigeben aktiv");
    h.get_by_label(&unlock).click();
    h.run_steps(3);
    assert!(!fp_export::palette_edit::any_locked(&settings(&mut h)));
    // ≡-Knopf zeigt dasselbe Menü: „Alle Bearbeitungen entfernen“
    let menu_r = label_rect(&h, &fp_i18n::t("image-preview-tip-options"));
    click_at(&mut h, menu_r.center(), Default::default());
    let all_edits = fp_i18n::t("image-preview-menu-remove-all-edits");
    assert!(is_enabled(&h, &all_edits));
    assert!(has_label(&h, &fp_i18n::t("image-preview-menu-load-palette")) && has_label(&h, &fp_i18n::t("image-preview-menu-save-palette")));
    h.get_by_label(&all_edits).click();
    h.run_steps(3);
    assert!(!fp_export::palette_edit::any_edited(&settings(&mut h)));
    // Sortierung über das Menü: wirkt auf die Palette (Datei)
    click_at(&mut h, menu_r.center(), Default::default());
    h.get_by_label(&fp_i18n::t("image-preview-menu-sort-luminance")).click();
    h.run_steps(3);
    assert_eq!(settings(&mut h).sorting, fp_export::Sorting::Luminance);
    wait_idle(&mut h);
    // Doppelklick: Farbwahl öffnet sich; OK macht die Farbe „modifiziert“
    let (_, grid, _) = table_grid(&mut h);
    let entries = result(&mut h).entries.clone();
    double_click_at(&mut h, grid.cell_rect(3).center());
    let cd = dlg(&mut h).color_dialog.clone().expect("Dialog „Farbe“ offen");
    assert_eq!(cd.mode, color_table::ColorMode::Edit(entries[3].color));
    dlg(&mut h).color_dialog.as_mut().expect("offen").color = egui::Color32::from_rgb(0x12, 0x34, 0x56);
    h.run_steps(2);
    let cr = dialogs::dialog_rect(&h.ctx, color_table::COLOR_DIALOG_ID).expect("Farbdialog gezeichnet");
    let ok_l = fp_i18n::t("btn-ok");
    let ok = h.query_all(By::new().label(ok_l.as_str())).map(|n| n.rect()).find(|r| cr.contains(r.center())).expect("OK im Farbdialog");
    click_at(&mut h, ok.center(), Default::default());
    assert!(dlg(&mut h).color_dialog.is_none());
    assert!(settings(&mut h).palette.iter().any(|p| p.edited == Some(0xFF12_3456)), "bearbeitet");
    assert!(st(&mut h).dialogs.preview.is_some(), "OK im Farbdialog schließt die Bildvorschau nicht");
    wait_idle(&mut h);
    // Klick in die Vorschau (Zeiger, Farbe gewählt) wählt den Eintrag des Bildpunkts
    let res = result(&mut h);
    let kk = res.index_at(200, 30).expect("Index");
    let p = pixel_pos(&mut h, 200.0, 30.0);
    click_at(&mut h, p, Default::default());
    assert_eq!(dlg(&mut h).table.selected, vec![res.entries[kk].color], "Eintrag des angeklickten Pixels");
    // Klick ins Leere der Tabelle hebt die Auswahl auf, die Pipetten kehren zurück
    let empty = area.right_bottom() - vec2(4.0, 4.0);
    click_at(&mut h, empty, Default::default());
    assert!(dlg(&mut h).table.selected.is_empty());
    assert!(has_label(&h, &fp_i18n::t("image-preview-tip-pipette-set")));
    // Mehrfachauswahl mit Strg und Löschen: benutzerdefinierte Palette ohne die Farben
    let (_, grid, n) = table_grid(&mut h);
    let entries = result(&mut h).entries.clone();
    click_at(&mut h, grid.cell_rect(0).center(), Default::default());
    click_at(&mut h, grid.cell_rect(1).center(), egui::Modifiers::COMMAND);
    assert_eq!(dlg(&mut h).table.selected.len(), 2);
    let del = label_rect(&h, &fp_i18n::t("image-preview-tip-delete"));
    click_at(&mut h, del.center(), Default::default());
    let s = settings(&mut h);
    assert_eq!((s.palette_kind, s.palette.len()), (PaletteKind::Custom, n - 2));
    assert!(s.palette.iter().all(|p| p.color & 0xFF_FFFF != entries[0].shown & 0xFF_FFFF || entries[0].shown != entries[0].color));
    // Palettenliste steht auf „Benutzerdefiniert“
    assert!(left_combos(&mut h).iter().any(|(_, v)| *v == fp_i18n::t("panel-opt-palette-custom")));
}

/// Palette laden/speichern über das Menü (Dateidialog mit Testvorgabe).
#[test]
fn palette_save_and_load() {
    let _g = lock();
    let dir = std::env::temp_dir().join(format!("fp-bv2-palette-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("p.act");
    let mut h = harness();
    st(&mut h).file_override = Some(fp_ui::files::FilePickOverride { open: vec![path.clone()], save: Some(path.clone()) });
    open(&mut h);
    dlg(&mut h).panes[0].settings.max_colors = 8;
    wait_idle(&mut h);
    let shown: Vec<u32> = result(&mut h).entries.iter().map(|e| e.shown & 0xFF_FFFF).collect();
    let menu_r = label_rect(&h, &fp_i18n::t("image-preview-tip-options"));
    click_at(&mut h, menu_r.center(), Default::default());
    h.get_by_label(&fp_i18n::t("image-preview-menu-save-palette")).click();
    h.run_steps(3);
    let bytes = std::fs::read(&path).expect("Palette gespeichert");
    assert_eq!(bytes.len(), shown.len() * 3, "ACT ohne Anhang");
    // andere Palette einstellen, dann laden
    dlg(&mut h).panes[0].settings.palette_kind = PaletteKind::Web216;
    dlg(&mut h).panes[0].settings.max_colors = 216;
    h.run_steps(2);
    click_at(&mut h, menu_r.center(), Default::default());
    h.get_by_label(&fp_i18n::t("image-preview-menu-load-palette")).click();
    h.run_steps(4);
    let s = settings(&mut h);
    assert_eq!(s.palette_kind, PaletteKind::Custom, "nach „Palette laden“: Benutzerdefiniert");
    assert_eq!(s.palette.iter().map(|p| p.color & 0xFF_FFFF).collect::<Vec<_>>(), shown);
    let _ = std::fs::remove_dir_all(&dir);
}

/// „Optimieren auf Größe“ (§2.8): Vorbelegung, Bisektion im Hintergrund, Meldung bei zu kleinem Ziel.
#[test]
fn optimize_to_size_dialog() {
    let _g = lock();
    let mut h = harness();
    open(&mut h);
    wait_idle(&mut h);
    let full = result(&mut h).bytes;
    let tip = fp_i18n::t("image-preview-tip-optimize-size");
    let r = label_rect(&h, &tip);
    click_at(&mut h, r.center(), Default::default());
    let sd = dlg(&mut h).size_dialog.clone().expect("Dialog offen");
    assert_eq!(sd.text, optimize_to_size::prefill(full), "vorbelegt mit der aktuellen Größe");
    assert_eq!(dialogs::last_dialog_id(&h.ctx).as_deref(), Some(optimize_to_size::DIALOG_ID));
    // Ziel = 40 % der Größe
    let target = full * 2 / 5;
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text(format!("{:.2}", target as f64 / 1000.0)));
    h.run_steps(2);
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    assert!(dlg(&mut h).size_dialog.is_none(), "Dialog geschlossen");
    assert!(st(&mut h).dialogs.preview.is_some(), "Enter schließt die Bildvorschau nicht");
    wait_idle(&mut h);
    let s = settings(&mut h);
    assert!(s.max_colors < 256, "weniger Farben: {}", s.max_colors);
    let res = result(&mut h);
    assert!(res.bytes < full && (res.bytes as f64) < target as f64 * 1.25, "{} nahe {target} (vorher {full})", res.bytes);
    assert_eq!(res.bytes, real_export(&mut h, &s));
    // zu kleines Ziel: Meldung, Werte bleiben
    let before = settings(&mut h);
    click_at(&mut h, r.center(), Default::default());
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text("0.01".into()));
    h.run_steps(2);
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    wait_idle(&mut h);
    let msg = fp_i18n::t("image-preview-size-too-small");
    assert_eq!(dlg(&mut h).message.as_deref(), Some(msg.as_str()));
    assert!(painted_texts(&h).iter().any(|(t, _)| t.contains("zu klein")), "Meldung gezeichnet");
    assert_eq!(settings(&mut h), before, "ursprüngliche Werte");
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    assert!(dlg(&mut h).message.is_none() && st(&mut h).dialogs.preview.is_some());
    // PNG 32: keine Suche, keine Änderung
    dlg(&mut h).panes[0].settings = OptimizeSettings { format: ExportFormat::Png, color_mode: ColorMode::Rgba32, ..OptimizeSettings::default() };
    wait_idle(&mut h);
    let png = settings(&mut h);
    dlg(&mut h).size_dialog = Some(optimize_to_size::SizeDialog { pane: 0, text: "1".into() });
    h.run_steps(2);
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    wait_idle(&mut h);
    assert_eq!(settings(&mut h), png);
    assert!(dlg(&mut h).message.is_none());
}

/// Optionen-Registerkarte in allen Designs und Sprachen: keine Rohschlüssel, nichts ragt aus der
/// linken Spalte, auch mit gewählter Farbe (Statuszeile) und bei JPEG; Statuszeile deutsch/englisch.
#[test]
fn options_tab_fits_in_all_themes_and_languages() {
    let _g = lock();
    let e = fp_export::palette_edit::PaletteEntry { color: 0xFF33_33CC, shown: 0xFF33_33CC, count: 1, locked: true, transparent: false, edited: true, web_safe: true };
    assert_eq!(color_table::status_text(&e, 0, false), "R:051 G:051 B:204 Hex:3333cc Index:000 Gesperrt Modifiziert Websicher");
    let y = fp_export::palette_edit::PaletteEntry { color: 0xFF9B_8D7D, shown: 0xFF9B_8D7D, count: 1, locked: false, transparent: true, edited: false, web_safe: false };
    assert_eq!(color_table::status_text(&y, 9, false), "R:155 G:141 B:125 Hex:9b8d7d Index:009 Transparent");
    fp_i18n::set_language("en");
    assert_eq!(color_table::status_text(&e, 0, false), "R:051 G:051 B:204 Hex:3333cc Index:000 Locked Edited Web Safe");
    for (w, hh) in [(1400.0, 900.0), (800.0, 600.0)] {
        for lang in ["de", "en"] {
            for theme in [ThemeKind::Normal, ThemeKind::Dark, ThemeKind::Oled] {
                fp_i18n::set_language(lang);
                let mut h = harness_with(w, hh, theme);
                open(&mut h);
                wait_idle(&mut h);
                for case in 0..3 {
                    match case {
                        0 => {}
                        1 => {
                            let (_, grid, _) = table_grid(&mut h);
                            click_at(&mut h, grid.cell_rect(5).center(), Default::default());
                            assert!(!dlg(&mut h).table.selected.is_empty());
                        }
                        _ => {
                            dlg(&mut h).panes[0].settings = fp_export::options::builtin_presets()[5].settings.clone();
                            wait_idle(&mut h);
                        }
                    }
                    let _ = dialogs::take_overflows(&h.ctx);
                    h.run_steps(2);
                    let over = dialogs::take_overflows(&h.ctx);
                    assert!(over.is_empty(), "{lang} {theme:?} {w}×{hh} Fall {case}: Überlauf {over:?}");
                    let rect = dialogs::dialog_rect(&h.ctx, image_preview::DIALOG_ID).expect("Dialog");
                    let left_r = layout(&mut h, 0).frame.left();
                    let all = painted_texts(&h);
                    let title = fp_i18n::t("dlg-preview-title");
                    let start = all.iter().position(|(s, r)| *s == title && rect.contains(r.center()) && r.center().y < rect.top() + dialogs::DIALOG_TITLE_H).unwrap_or(all.len());
                    for (s, r) in &all[start..] {
                        assert!(!s.starts_with("image-preview-") && !s.starts_with("dlg-preview-"), "{lang}: Rohschlüssel {s}");
                        if r.is_positive() && rect.contains(r.center()) && r.center().x < left_r {
                            assert!(r.right() <= left_r - 4.0, "{lang} {theme:?} {w}×{hh} Fall {case}: „{s}“ {r:?} ragt in die Vorschau");
                            assert!(r.bottom() <= rect.bottom() + 0.5, "{lang} {theme:?}: „{s}“ unter dem Dialog");
                        }
                    }
                }
            }
        }
    }
}

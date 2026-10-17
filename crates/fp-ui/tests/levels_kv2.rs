//! KV-2: Tonwertkorrektur nach re\19-kurven-tonwert.md §3 – Gamma ↔ Mittelgriff (Binärsuchen),
//! Histogramm (RGB-Summe, Maßstab), Schieber-Regeln, Auto, Pipetten (mit der Eigenheit der
//! Mittelton-Pipette), Vorschau während des Ziehens, Abbrechen ohne Verlaufsschritt, OK = ein
//! Schritt, nichts gemerkt außer dem gemeinsamen Vorschau-Schalter.

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{By, Queryable};
use egui_kittest::Harness;
use fp_engine::Session;
use fp_raster::filters::levels_math as lm;
use fp_raster::filters::{LevelsChannel, LevelsParams};
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::levels_common::{self, Pipette};
use fp_ui::dialogs::levels::{self, LevelsDialog, Slider};
use fp_ui::FirepixelApp;
use serde_json::json;
use std::sync::Arc;

type H = Harness<'static, FirepixelApp>;

// ------------------------------------------------------------------ Rechenregeln

#[test]
fn gamma_handle_examples_from_spec() {
    // Griff genau in der Mitte → 0,99609375, nicht 1
    assert_eq!(lm::gamma_from_fraction(0.5), 0.99609375);
    assert_eq!(lm::gamma_from_handle(127.5, 0.0, 255.0), 0.99609375);
    // Laufzeitbeispiel: Griff 99,23 bei Schwarz 63, Weiß 200 → 2,265625 („2.27“)
    assert_eq!(lm::gamma_from_handle(99.23, 63.0, 200.0), 2.265625);
    // Gamma 2 liegt bei f ≈ 0,703 (Anteil von rechts), also links der Mitte
    let f = lm::fraction_from_gamma(2.0);
    assert!((f - 0.703).abs() < 0.01, "f(2) = {f}");
    assert!(lm::handle_from_gamma(2.0, 0.0, 255.0) < 127.5);
    // Gamma 1 → Griff in der Mitte; kleine Gammas rechts der Mitte
    assert_eq!(lm::handle_from_gamma(1.0, 0.0, 255.0), 127.5);
    assert!(lm::handle_from_gamma(0.5, 0.0, 255.0) > 127.5);
    // Richtung: Griff nach links = heller (Gamma > 1), nach rechts = dunkler (Gamma < 1)
    assert!(lm::gamma_from_handle(60.0, 0.0, 255.0) > 1.5);
    assert!(lm::gamma_from_handle(200.0, 0.0, 255.0) < 0.7);
    // Hin und zurück trifft das Gamma auf ±0,03 (Abbruchschwelle der Suche)
    for g in [0.3, 0.5, 0.8, 1.0, 1.5, 2.265625, 4.0] {
        let pos = lm::handle_from_gamma(g, 0.0, 255.0);
        assert!((lm::gamma_from_handle(pos, 0.0, 255.0) - g).abs() < 0.06, "Gamma {g}");
    }
}

#[test]
fn preview_tables_follow_the_measured_sequence() {
    // Sitzung s3: Mittelton-Pipette auf Grau 64 → Gamma je Kanal 0,5029; Grau 64 → 16, 192 → 145
    let mut p = LevelsParams::default();
    lm::apply_pick(&mut p, Pipette::Midtone, [64, 64, 64]);
    for c in [&p.red, &p.green, &p.blue] {
        assert_eq!(c.gamma, 0.5029296875);
    }
    assert_eq!(p.rgb, LevelsChannel::default(), "RGB-Satz bleibt");
    let t = p.tables();
    assert_eq!((t.red[64], t.red[192]), (16, 145));
    // Lichter auf das angezeigte Grau 145 → Weißpunkt 145
    lm::apply_pick(&mut p, Pipette::Highlight, [145, 145, 145]);
    assert_eq!((p.red.source_high, p.green.source_high, p.blue.source_high), (145, 145, 145));
    // (200, 100, 50) wird jetzt als (255, 121, 30) angezeigt
    let t = p.tables();
    let shown = [t.rgb[t.red[200] as usize], t.rgb[t.green[100] as usize], t.rgb[t.blue[50] as usize]];
    assert_eq!(shown, [255, 121, 30]);
    // Schatten auf die angezeigte Farbe → Schwarzpunkt Rot 143 (= 145 − 2), Grün 121, Blau 30
    lm::apply_pick(&mut p, Pipette::Shadow, shown);
    assert_eq!((p.red.source_low, p.green.source_low, p.blue.source_low), (143, 121, 30));
    // Schatten auf 0 → 0, Lichter höchstens 255, mindestens Schwarz + 2
    let mut q = LevelsParams::default();
    q.red.source_low = 100;
    lm::apply_pick(&mut q, Pipette::Highlight, [50, 255, 0]);
    assert_eq!((q.red.source_high, q.green.source_high, q.blue.source_high), (102, 255, 2));
    lm::apply_pick(&mut q, Pipette::Shadow, [0, 254, 7]);
    assert_eq!((q.red.source_low, q.green.source_low, q.blue.source_low), (0, 253, 0));
    // Tabellen der Laufzeitmessung d2: levels_table(63, 200, 0, 255, 2,265625)
    let c = LevelsChannel { source_low: 63, source_high: 200, dest_low: 0, dest_high: 255, gamma: 2.265625 };
    let t = c.table();
    assert_eq!((t[0], t[63], t[200], t[255]), (0, 0, 255, 255));
    // gekreuzte Ausgabe (unten 49, oben 30) kehrt um
    let x = LevelsChannel { dest_low: 49, dest_high: 30, ..Default::default() }.table();
    assert_eq!((x[0], x[255]), (49, 30));
}

#[test]
fn auto_sets_channel_sets_and_keeps_rgb() {
    // Verlauf 20…230, je ein Pixel pro Wert → 20/1/230 in Rot, Grün, Blau, RGB unverändert
    let mut h = [[0u64; 256]; 3];
    for c in &mut h {
        c[20..=230].fill(1);
    }
    let mut p = LevelsParams::default();
    p.rgb.gamma = 1.7;
    p.red.dest_low = 9;
    lm::auto_levels_from(&mut p, &h);
    for c in [&p.red, &p.green, &p.blue] {
        assert_eq!(*c, LevelsChannel { source_low: 20, source_high: 230, dest_low: 0, dest_high: 255, gamma: 1.0 });
    }
    assert_eq!(p.rgb.gamma, 1.7, "RGB-Satz bleibt");
    // die Tabelle schneidet ab: 32 → 14 (das Kurven-Auto rundet auf 15)
    assert_eq!(p.red.table()[32], 14);
}

#[test]
fn histogram_rgb_sum_scale_and_heights() {
    let bounds = fp_geom::IRect::from_xywh(0, 0, 4, 1);
    let img = fp_raster::RgbaImage::from_raw(bounds, vec![[10, 20, 30, 255], [10, 200, 30, 255], [250, 250, 250, 0], [10, 20, 30, 1]]).expect("Bild");
    let h = lm::channel_histograms(&img);
    assert_eq!((h[0][10], h[1][20], h[1][200], h[2][30]), (3, 2, 1, 3));
    assert_eq!(h[0][250], 0, "Alpha 0 zählt nicht");
    // RGB = Summe der Kanalhistogramme, jeder Wert durch die Tabelle seines Kanals
    let mut p = LevelsParams::default();
    let sum = lm::display_histogram(&h, &p, 0);
    assert_eq!((sum[10], sum[20], sum[30], sum[200]), (3, 2, 3, 1));
    p.red = LevelsChannel { dest_low: 100, dest_high: 100, ..Default::default() };
    p.rgb.gamma = 3.0; // die RGB-Einstellung geht nicht ein
    let sum = lm::display_histogram(&h, &p, 0);
    assert_eq!((sum[10], sum[100], sum[20]), (0, 3, 2));
    assert_eq!(lm::display_histogram(&h, &p, 2), h[1], "Kanal Grün = Originalhistogramm");
    // Maßstab = Mittel der 5 größten Klassen (ganzzahlig), höhere Säulen abgeschnitten
    let mut g = [0u64; 256];
    g[0] = 1000;
    for v in 1..5 {
        g[v * 50] = 100;
    }
    g[255] = 10;
    assert_eq!(lm::histogram_scale(&g), (1000 + 400) / 5);
    let hs = lm::histogram_heights(&g, 239, 109);
    assert_eq!(hs.len(), 239);
    assert_eq!(hs[0], 109, "abgeschnitten");
    assert_eq!(hs[238], (10.0f64 / 280.0 * 109.0).round() as u32, "letzte Spalte = Wert 255");
    assert!(hs.iter().all(|&x| x <= 109));
    assert!(lm::histogram_heights(&[0; 256], 239, 109).iter().all(|&x| x == 0), "leeres Bild");
}

#[test]
fn slider_rules_input_ordered_output_crossing() {
    // Feldwert aus dem Schieber kaufmännisch gerundet: 62,95 → 63
    let mut c = LevelsChannel::default();
    lm::apply_input_drag(&mut c, 0, 62.95);
    assert_eq!(c.source_low, 63);
    // Mittelgriff setzt Gamma (Anteil von rechts)
    c.source_high = 200;
    lm::apply_input_drag(&mut c, 1, 99.23);
    assert_eq!(c.gamma, 2.265625);
    // Schwarz Richtung 250 ziehen (viele Mausbewegungen): der Mittelgriff wandert mit, Schwarz hält
    // mindestens 8,5 vor ihm und bleibt deutlich unter Weiß
    for k in 0..200 {
        lm::apply_input_drag(&mut c, 0, 64.0 + k as f64);
        let h = lm::input_handles(&c);
        assert!(h[0] < h[1] && h[1] < h[2], "geordnet: {h:?}");
    }
    let h = lm::input_handles(&c);
    assert!(c.source_low > 150 && c.source_low < 175, "Halt vor Weiß: {}", c.source_low);
    assert!(h[1] - h[0] >= lm::INPUT_HANDLE_GAP - 0.5, "Abstand {h:?}");
    assert_eq!(c.source_high, 200);
    assert_eq!(c.gamma, 2.265625, "Gamma bleibt beim Ziehen von Schwarz");
    // Weiß kann nicht unter den Mittelgriff
    lm::apply_input_drag(&mut c, 2, 0.0);
    assert!(c.source_high as f64 >= lm::input_handles(&c)[1]);
    // Mittelgriff nicht über Weiß hinaus
    let mut m = LevelsChannel::default();
    lm::apply_input_drag(&mut m, 1, 255.0);
    assert!(lm::input_handles(&m)[1] <= 255.0 - lm::INPUT_HANDLE_GAP + 0.6);
    // Ausgabe: Griffe kreuzen sich, nur 0…255
    let mut o = LevelsChannel::default();
    lm::apply_output_drag(&mut o, 0, 200.4);
    lm::apply_output_drag(&mut o, 1, 30.0);
    assert_eq!((o.dest_low, o.dest_high), (200, 30));
    lm::apply_output_drag(&mut o, 1, 400.0);
    assert_eq!(o.dest_high, 255);
    // Wiederholung: 50× derselbe Wert = einmal
    let mut a = LevelsChannel::default();
    for _ in 0..50 {
        lm::apply_input_drag(&mut a, 1, 90.0);
    }
    let mut b = LevelsChannel::default();
    lm::apply_input_drag(&mut b, 1, 90.0);
    assert_eq!(a, b);
}

#[test]
fn slider_geometry_and_handle_hits() {
    let track = egui::Rect::from_min_size(egui::pos2(17.0, 174.0), egui::vec2(249.0, 13.0));
    assert_eq!(levels::slider_x(track, 0.0), 22.0, "Spur 5 px eingerückt");
    assert_eq!(levels::slider_x(track, 255.0), 261.0);
    assert_eq!(levels::slider_value(track, 10.0), 0.0);
    assert_eq!(levels::slider_value(track, 300.0), 255.0);
    let vals = [0.0, 127.5, 255.0];
    assert_eq!(levels::handle_at(track, &vals, 24.0), Some(0));
    assert_eq!(levels::handle_at(track, &vals, 141.5), Some(1));
    assert_eq!(levels::handle_at(track, &vals, 80.0), None, "Spur neben den Griffen tut nichts");
}

// ------------------------------------------------------------------ Dialog (kittest)

fn harness() -> H {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 400, "height": 300, "background": "#ffffff"})).expect("Dokument");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 220, 160], "fillColor": "#404040", "strokeWidth": 0})).expect("Rechteck");
    s.execute("modify.flattenSelection", json!({})).expect("Bitmap");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn doc(h: &H) -> Arc<fp_model::Document> {
    Arc::clone(&st(h).session.active().expect("Dokument").doc)
}

fn history(h: &H) -> (usize, usize) {
    let d = st(h).session.active().expect("Dokument");
    (d.history.undo_count(), d.history.redo_count())
}

fn dlg(h: &H) -> &LevelsDialog {
    st(h).dialogs.levels.as_ref().expect("Tonwertdialog offen")
}

fn open(h: &mut H) {
    assert!(fp_ui::dialogs::open_for_command(&mut h.state_mut().state, "filter.adjustColor.levels"));
    h.run_steps(3);
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

fn press(h: &mut H, label: &str) {
    h.run_steps(1);
    let r = h.get_all(By::new().label(label)).last().unwrap_or_else(|| panic!("Knopf {label}")).rect();
    click(h, r.center());
}

/// Pixel des Bitmaps an (x, y) im Dokument.
fn bitmap_px(h: &H, x: i32, y: i32) -> [u8; 4] {
    let d = st(h).session.active().expect("Dokument");
    let id = d.selection.elements[0];
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("Element");
    let fp_model::ElementKind::Bitmap(b) = &e.kind else { panic!("kein Bitmap") };
    fp_engine::docops::bitmap_image(b).expect("Bild").get(x, y).expect("Pixel")
}

#[test]
fn mid_handle_drag_previews_live_and_cancel_leaves_no_step() {
    let mut h = harness();
    let (d0, h0) = (doc(&h), history(&h));
    let px0 = bitmap_px(&h, 50, 50);
    open(&mut h);
    assert!(dlg(&h).preview.preview_on, "Vorschau an (Vorgabe)");
    let track = levels::track_rect(&h.ctx, Slider::Input).expect("Eingabeschieber");
    let y = track.center().y;
    let x0 = levels::slider_x(track, 127.5);
    // Mittelgriff über viele Bilder nach links ziehen; die Vorschau rechnet schon während des Ziehens
    ev(&mut h, Event::PointerMoved(egui::pos2(x0, y)));
    ev(&mut h, Event::PointerButton { pos: egui::pos2(x0, y), button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    for k in 1..=20 {
        ev(&mut h, Event::PointerMoved(egui::pos2(x0 - 3.0 * k as f32, y)));
    }
    let g = dlg(&h).params.rgb.gamma;
    assert!(g > 1.5, "Griff links → Gamma > 1: {g}");
    assert!(bitmap_px(&h, 50, 50)[0] > px0[0], "Vorschau während des Ziehens (heller)");
    assert_eq!(history(&h).0, h0.0 + 1, "Vorschau ersetzt, statt zu addieren");
    ev(&mut h, Event::PointerButton { pos: egui::pos2(x0 - 60.0, y), button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
    // Klick auf die Spur neben den Griffen ändert nichts
    let before = dlg(&h).params;
    click(&mut h, egui::pos2(levels::slider_x(track, 230.0), y));
    assert_eq!(dlg(&h).params, before, "nur Griffe reagieren");
    press(&mut h, "Abbrechen");
    assert!(st(&h).dialogs.levels.is_none());
    assert!(*doc(&h) == *d0, "Dokument wie vorher");
    assert_eq!(history(&h), h0, "kein Verlaufsschritt nach Abbrechen");
}

#[test]
fn ok_is_one_step_and_nothing_is_remembered() {
    let mut h = harness();
    let h0 = history(&h);
    open(&mut h);
    h.state_mut().state.dialogs.levels.as_mut().expect("Dialog").drag_handle(Slider::Input, 0, 30.0);
    h.run_steps(2);
    h.state_mut().state.dialogs.levels.as_mut().expect("Dialog").set_channel(1);
    h.state_mut().state.dialogs.levels.as_mut().expect("Dialog").drag_handle(Slider::Output, 1, 120.0);
    h.run_steps(2);
    press(&mut h, "OK");
    assert!(st(&h).dialogs.levels.is_none());
    assert_eq!(history(&h), (h0.0 + 1, 0), "OK = genau ein Verlaufsschritt");
    // erneut öffnen: Grundwerte, Kanal RGB
    open(&mut h);
    assert_eq!(dlg(&h).params, LevelsParams::default(), "nichts gemerkt");
    assert_eq!(dlg(&h).channel, 0);
    press(&mut h, "Abbrechen");
    assert_eq!(history(&h), (h0.0 + 1, 0));
}

#[test]
fn preview_switch_is_shared_and_kept_after_cancel() {
    let mut h = harness();
    let (d0, h0) = (doc(&h), history(&h));
    open(&mut h);
    h.state_mut().state.dialogs.levels.as_mut().expect("Dialog").set_field(1, 2.0);
    h.run_steps(2);
    assert_eq!(history(&h).0, h0.0 + 1, "Vorschau angewandt");
    press(&mut h, "Vorschau");
    assert!(!dlg(&h).preview.preview_on);
    assert!(!levels_common::preview_pref(st(&h)), "sofort gemerkt");
    assert_eq!(history(&h), h0, "Vorschau aus → Dokument original");
    press(&mut h, "Abbrechen");
    assert!(*doc(&h) == *d0);
    // gemerkt auch nach Abbrechen, gemeinsam mit dem Kurvendialog
    open(&mut h);
    assert!(!dlg(&h).preview.preview_on, "Tonwert startet ohne Vorschau");
    press(&mut h, "Vorschau");
    assert!(levels_common::preview_pref(st(&h)));
    press(&mut h, "Abbrechen");
    assert_eq!(history(&h), h0);
}

#[test]
fn auto_button_stays_open_and_pipette_picks_displayed_color() {
    let mut h = harness();
    let h0 = history(&h);
    open(&mut h);
    // Auto: Kanalsätze auf die Grenzen des Bitmap-Histogramms, der Dialog bleibt offen
    press(&mut h, "Auto");
    let d = dlg(&h);
    assert_eq!(d.params.rgb, LevelsChannel::default(), "RGB bleibt");
    assert_eq!((d.params.red.source_low, d.params.red.source_high), fp_raster::filters::table::histogram_bounds(&d.hist[0]));
    // Pipette Lichter, dann Klick auf das Bitmap in der Leinwand → angezeigte Farbe
    let (canvas_pt, expected) = {
        let s = st(&h);
        let doc = s.session.active().expect("Dokument");
        let area = s.canvases.get(&doc.id).and_then(|c| c.area).expect("Leinwand");
        let (sx, sy) = doc.view.doc_to_screen(fp_geom::Point::new(100.5, 100.5));
        let c = levels_common::displayed_color_at(s, egui::pos2(area.left() + sx as f32, area.top() + sy as f32)).expect("Farbe");
        (egui::pos2(area.left() + sx as f32, area.top() + sy as f32), c)
    };
    let before = dlg(&h).params;
    press(&mut h, "Lichterfarbe auswählen");
    assert_eq!(dlg(&h).picking, Some(Pipette::Highlight));
    click(&mut h, canvas_pt);
    assert!(st(&h).dialogs.levels.is_some(), "Klick auf die Leinwand schließt den Dialog nicht");
    let d = dlg(&h);
    assert_eq!(d.picking, None, "Pipette nur einmal");
    let mut want = before;
    lm::apply_pick(&mut want, Pipette::Highlight, expected);
    assert_eq!(d.params, want, "Lichter mit der angezeigten Farbe {expected:?}");
    // Esc im Aufnahmemodus beendet nur die Aufnahme
    press(&mut h, "Schattenfarbe auswählen");
    ev(&mut h, Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
    ev(&mut h, Event::Key { key: egui::Key::Escape, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::NONE });
    assert!(st(&h).dialogs.levels.is_some());
    assert_eq!(dlg(&h).picking, None);
    press(&mut h, "Abbrechen");
    assert_eq!(history(&h), h0, "Abbrechen ohne Verlaufsschritt");
}

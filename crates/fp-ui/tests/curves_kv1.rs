//! Kurvendialog originalgetreu (KV-1, re\19-kurven-tonwert.md §2): Mausregeln im Kurvenfeld,
//! Felder Eingabe/Ausgabe, Umkehren, Auto und Pipetten gegen die Zahlenbeispiele der
//! Spezifikation, gemeinsamer Vorschau-Schalter, Abbrechen ohne Verlaufsschritt, OK genau ein
//! Schritt, nichts gemerkt nach OK.

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use fp_geom::IRect;
use fp_raster::filters::{CurvePoint, CurvesParams};
use fp_raster::RgbaImage;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::curves::{CurvesDialog, FIELD};
use fp_ui::dialogs::levels_common::{Pipette, PREVIEW_PREF};
use fp_ui::FirepixelApp;
use serde_json::json;
use std::sync::Arc;

type H = Harness<'static, FirepixelApp>;

fn pts(v: &[(f32, f32)]) -> Vec<CurvePoint> {
    v.iter().map(|&(x, y)| CurvePoint { x, y }).collect()
}

fn dlg() -> CurvesDialog {
    CurvesDialog::with_state([[0; 256]; 3], false, true)
}

/// Pixelstelle in der Fläche für den Kurvenwert (x, y) bei nicht umgekehrter Darstellung.
fn px_of(x: f32, y: f32) -> (f32, f32) {
    ((x / 255.0 * FIELD).round(), (FIELD - y / 255.0 * FIELD).round())
}

/// Klick (Drücken + Loslassen) an Kurvenwert-Stelle.
fn click_at(d: &mut CurvesDialog, x: f32, y: f32) {
    let (px, py) = px_of(x, y);
    d.press(px, py);
    d.release();
}

// ------------------------------------------------------------------ Mausregeln (Modell)

#[test]
fn rundung_und_koordinaten() {
    let d = dlg();
    // §2.5: (63,14; 128,71) → (63, 129)
    let (px, py) = (63.14 / 255.0 * FIELD, FIELD - 128.71 / 255.0 * FIELD);
    assert_eq!(d.to_curve(px, py), (63.0, 129.0));
    assert_eq!(d.to_curve(0.0, FIELD), (0.0, 0.0));
    assert_eq!(d.to_curve(FIELD, 0.0), (255.0, 255.0));
}

#[test]
fn neuer_punkt_nur_nahe_der_kurve() {
    let mut d = dlg();
    // weit weg von der Diagonale: nichts
    click_at(&mut d, 128.0, 40.0);
    assert_eq!(d.points().len(), 2);
    assert_eq!(d.selected, Some(0), "Auswahl unverändert");
    // ±4 an der Kurve: neuer Punkt, gewählt
    click_at(&mut d, 128.0, 131.0);
    assert_eq!(d.points().len(), 3);
    let p = d.points()[1];
    assert_eq!((p.x, p.y), (p.x.round(), p.y.round()), "ganzzahlig");
    assert!((p.x - 128.0).abs() <= 1.0 && (p.y - 131.0).abs() <= 1.0, "{p:?}");
    assert_eq!(d.selected, Some(1));
    assert_eq!(d.shown, (p.x, p.y), "Felder zeigen den neuen Punkt");
}

#[test]
fn mindestabstand_beim_einfuegen() {
    // §2.6 Laufzeit `s2/j_dicht`: Klicks bei 18 und 25 setzen Punkte, 29 wird abgewiesen
    let mut d = dlg();
    for x in [15.0f32, 20.0, 24.0] {
        d.press(x, FIELD - x);
        d.release();
    }
    let xs: Vec<f32> = d.points().iter().map(|p| p.x).collect();
    assert_eq!(xs, vec![0.0, 18.0, 25.0, 255.0]);
    // dicht an einem Endpunkt ebenfalls nicht
    click_at(&mut d, 253.0, 253.0);
    assert_eq!(d.points().len(), 4);
}

#[test]
fn startschwelle_drei_einheiten() {
    let mut d = dlg();
    click_at(&mut d, 128.0, 128.0);
    let before = d.points().clone();
    let (px, py) = d.to_px(before[1]);
    d.press(px, py);
    // 2 px ≈ 2,5 Einheiten: noch nicht verschoben
    d.drag_to(px + 2.0, py);
    assert_eq!(*d.points(), before, "unter der Schwelle");
    d.drag_to(px + 2.0, py - 20.0);
    d.release();
    let p = d.points()[1];
    assert_eq!(p, CurvePoint { x: d.to_curve(px + 2.0, py - 20.0).0, y: d.to_curve(px + 2.0, py - 20.0).1 });
    assert_eq!(d.points().len(), 3);
}

#[test]
fn herausziehen_loescht_rueckkehr_fuegt_wieder_ein() {
    let mut d = dlg();
    click_at(&mut d, 128.0, 128.0);
    let (px, py) = d.to_px(d.points()[1]);
    // nach unten aus dem Feld: vorläufig entfernt
    d.press(px, py);
    d.drag_to(px, FIELD + 30.0);
    assert_eq!(d.points().len(), 2, "vorläufig entfernt");
    // zurück ins Feld: wieder eingefügt
    d.drag_to(px, py - 10.0);
    assert_eq!(d.points().len(), 3, "wieder eingefügt");
    // wieder hinaus und loslassen: gelöscht
    d.drag_to(px, FIELD + 30.0);
    d.release();
    assert_eq!(*d.points(), pts(&[(0.0, 0.0), (255.0, 255.0)]));
    assert_eq!(d.selected, None);
}

#[test]
fn ueberholen_loescht_felder_behalten_letzten_wert() {
    let mut d = dlg();
    click_at(&mut d, 128.0, 128.0);
    click_at(&mut d, 192.0, 192.0);
    assert_eq!(d.points().len(), 4);
    let (px, py) = d.to_px(d.points()[1]);
    d.press(px, py);
    d.drag_to(px + 20.0, py + 40.0);
    let last = d.shown;
    assert_eq!(last, (d.points()[1].x, d.points()[1].y));
    // über den Nachbarn 192 hinaus nach rechts → gelöscht
    d.drag_to(d.to_px(CurvePoint { x: 200.0, y: 100.0 }).0, py + 40.0);
    d.release();
    assert_eq!(d.points().len(), 3);
    assert_eq!(d.points()[1].x, 192.0);
    assert_eq!(d.shown, last, "Felder zeigen den letzten gültigen Wert");
}

#[test]
fn endpunkte_fest_und_begrenzt() {
    let mut d = dlg();
    let (px, py) = d.to_px(d.points()[0]);
    d.press(px, py);
    d.drag_to(px - 50.0, py + 50.0);
    d.release();
    assert_eq!(d.points()[0], CurvePoint { x: 0.0, y: 0.0 }, "auf 0…255 begrenzt, nicht gelöscht");
    // nach rechts bis an den anderen Endpunkt: Abstand 4 bleibt
    d.press(px, py);
    d.drag_to(FIELD + 40.0, py - 60.0);
    d.release();
    assert_eq!(d.points().len(), 2);
    assert_eq!(d.points()[0].x, 251.0);
}

#[test]
fn felder_eingabe_ausgabe() {
    let mut d = dlg();
    click_at(&mut d, 128.0, 128.0);
    click_at(&mut d, 192.0, 192.0);
    d.selected = Some(1);
    // Ausgabe 300 → 255
    d.commit_fields(100.0, 300.0);
    assert_eq!(d.points()[1], CurvePoint { x: 100.0, y: 255.0 });
    // Eingabe jenseits des Nachbarn → auf den Nachbarn begrenzt → verletzt den Abstand → nichts
    let before = d.points().clone();
    d.commit_fields(250.0, 10.0);
    assert_eq!(*d.points(), before);
    // Endpunkt: Ausgabe begrenzt
    d.selected = Some(0);
    d.commit_fields(0.0, 999.0);
    assert_eq!(d.points()[0], CurvePoint { x: 0.0, y: 255.0 });
}

#[test]
fn umkehren_ist_nur_anzeige() {
    let mut d = dlg();
    click_at(&mut d, 64.0, 64.0);
    let p0 = d.points().clone();
    d.reversed = true;
    assert_eq!(*d.points(), p0);
    // gespiegelte Mausumrechnung: links oben = Eingabe 255, Ausgabe 0
    assert_eq!(d.to_curve(0.0, 0.0), (255.0, 0.0));
    assert_eq!(d.to_px(CurvePoint { x: 255.0, y: 0.0 }), (0.0, 0.0));
}

#[test]
fn kanalwechsel_waehlt_punkt_null() {
    let mut d = dlg();
    click_at(&mut d, 128.0, 128.0);
    d.set_channel(1);
    assert_eq!((d.selected, d.shown), (Some(0), (0.0, 0.0)));
    click_at(&mut d, 64.0, 64.0);
    assert_eq!(d.params.red_points.len(), 3);
    assert_eq!(d.params.rgb_points.len(), 3, "jeder Kanal hat seine eigene Liste");
    assert_eq!(d.params.green_points.len(), 2);
}

// ------------------------------------------------------------------ Auto und Pipetten (Modell)

#[test]
fn auto_im_dialog() {
    let mut h = [[0u64; 256]; 3];
    for c in &mut h {
        for v in c.iter_mut().take(231).skip(20) {
            *v = 10;
        }
        c[20] = 500;
        c[230] = 500;
    }
    let mut d = CurvesDialog::with_state(h, false, true);
    click_at(&mut d, 128.0, 128.0);
    d.auto();
    assert_eq!(d.params.rgb_points, pts(&[(0.0, 0.0), (255.0, 255.0)]));
    for l in [&d.params.red_points, &d.params.green_points, &d.params.blue_points] {
        assert_eq!(*l, pts(&[(20.0, 0.0), (230.0, 255.0)]));
    }
    // Kurven-Auto rundet: 32 → 15 (Tonwert-Auto schneidet ab: 14)
    assert_eq!(d.params.tables().red[32], 15);
    assert_eq!(d.selected, Some(0));
}

#[test]
fn pipetten_nach_spezifikation() {
    let mut d = dlg();
    d.apply_pipette(Pipette::Shadow, [200, 100, 50]);
    assert_eq!(d.params.red_points, pts(&[(200.0, 0.0), (255.0, 255.0)]));
    assert_eq!(d.params.green_points, pts(&[(100.0, 0.0), (255.0, 255.0)]));
    assert_eq!(d.params.blue_points, pts(&[(50.0, 0.0), (255.0, 255.0)]));
    // angezeigte Farbe von (40,120,220) nach der Schatten-Korrektur
    let t = d.params.tables();
    let shown = [t.rgb[t.red[40] as usize], t.rgb[t.green[120] as usize], t.rgb[t.blue[220] as usize]];
    assert_eq!(shown, [0, 33, 211]);
    d.apply_pipette(Pipette::Midtone, shown);
    assert_eq!(d.params.green_points, pts(&[(100.0, 0.0), (102.0, 127.5), (255.0, 255.0)]));
    assert_eq!(d.params.blue_points, pts(&[(50.0, 0.0), (211.0, 127.5), (255.0, 255.0)]));
    d.apply_pipette(Pipette::Highlight, [250, 30, 3]);
    assert_eq!(d.params.red_points, pts(&[(200.0, 0.0), (250.0, 255.0)]));
    assert_eq!(d.params.green_points, pts(&[(26.0, 0.0), (30.0, 255.0)]));
    assert_eq!(d.params.blue_points, pts(&[(0.0, 0.0), (4.0, 255.0)]));
    assert_eq!(d.picking, None, "Pipette bleibt nicht aktiv");
}

// ------------------------------------------------------------------ im Fenster (kittest)

/// Dokument mit Testbitmap (Grauverlauf oben, darunter Grau 64, Grau 192, (200,100,50),
/// (40,120,220)), gewählt.
fn harness() -> H {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    let mut img = RgbaImage::filled(IRect::from_xywh(10, 10, 264, 120), fp_color::Rgba8::new(255, 255, 255, 255)).expect("Bild");
    let w = img.width();
    for (i, px) in img.data_mut().iter_mut().enumerate() {
        let (x, y) = (i % w, i / w);
        *px = if y < 60 {
            let g = (x * 255 / (w - 1)) as u8;
            [g, g, g, 255]
        } else {
            match x / 66 {
                0 => [64, 64, 64, 255],
                1 => [192, 192, 192, 255],
                2 => [200, 100, 50, 255],
                _ => [40, 120, 220, 255],
            }
        };
    }
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    s.execute("edit.paste", json!({})).expect("einfügen");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn undo_count(h: &H) -> usize {
    st(h).session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn doc(h: &H) -> Arc<fp_model::Document> {
    Arc::clone(&st(h).session.active().expect("doc").doc)
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn button(h: &mut H, p: Pos2, down: bool) {
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: down, modifiers: Modifiers::NONE });
}

fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    button(h, p, true);
    button(h, p, false);
    h.run_steps(3);
}

fn drag(h: &mut H, a: Pos2, b: Pos2, frames: usize) {
    ev(h, Event::PointerMoved(a));
    button(h, a, true);
    for i in 1..=frames {
        ev(h, Event::PointerMoved(a + (b - a) * (i as f32 / frames as f32)));
    }
    button(h, b, false);
    h.run_steps(3);
}

fn open(h: &mut H) {
    h.state_mut().state.run("filter.adjustColor.curves", json!({}));
    h.run_steps(3);
    assert!(st(h).dialogs.curves.is_some(), "Kurven offen");
}

fn curves(h: &H) -> &CurvesDialog {
    st(h).dialogs.curves.as_ref().expect("Kurven")
}

/// Linke obere Ecke des Client-Bereichs des Dialogs auf dem Bildschirm.
fn client_origin(h: &H) -> Pos2 {
    let r = fp_ui::dialogs::dialog_rect(&h.ctx, "fp-curves").expect("Dialog");
    r.min + egui::vec2(1.0, 1.0 + fp_ui::dialogs::DIALOG_TITLE_H)
}

/// Linke obere Ecke der 208er-Fläche auf dem Bildschirm.
fn field_origin(h: &H) -> Pos2 {
    client_origin(h) + egui::vec2(34.0, 29.0)
}

/// Bildschirmstelle eines Dokumentpunkts auf der Leinwand.
fn screen_of(h: &H, x: f32, y: f32) -> Pos2 {
    let d = st(h).session.active().expect("doc");
    let area = st(h).canvases.get(&d.id).and_then(|c| c.area).expect("Leinwand");
    let v = &d.view;
    area.min + egui::vec2(((x as f64 - v.origin.0) * v.zoom) as f32, ((y as f64 - v.origin.1) * v.zoom) as f32)
}

/// Punkt in der Mitte der Diagonale per Maus nach oben ziehen (Kurve aufhellen).
fn bend_curve(h: &mut H) {
    let a = field_origin(h) + egui::vec2(104.0, 104.0);
    drag(h, a, a + egui::vec2(0.0, -40.0), 6);
}

#[test]
fn maus_im_fenster_setzt_und_loescht_punkte() {
    let mut h = harness();
    let n0 = undo_count(&h);
    open(&mut h);
    bend_curve(&mut h);
    let p = curves(&h).points().clone();
    assert_eq!(p.len(), 3, "Punkt gesetzt: {p:?}");
    assert!(p[1].y > p[1].x + 30.0, "nach oben gezogen: {p:?}");
    assert_eq!(undo_count(&h), n0 + 1, "Vorschau nach dem Loslassen");
    // Klick weit neben der Kurve: nichts
    let o = field_origin(&h);
    click(&mut h, o + egui::vec2(180.0, 190.0));
    assert_eq!(curves(&h).points().len(), 3);
    // Punkt aus dem Feld nach unten ziehen: gelöscht
    let (px, py) = curves(&h).to_px(p[1]);
    drag(&mut h, o + egui::vec2(px, py), o + egui::vec2(px, FIELD + 40.0), 8);
    assert_eq!(curves(&h).points().len(), 2, "herausgezogen = gelöscht");
    // Rampe kehrt die Darstellung um, Punkte bleiben
    click(&mut h, o + egui::vec2(60.0, FIELD + 10.0));
    assert!(curves(&h).reversed);
    assert_eq!(curves(&h).points().len(), 2);
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
}

#[test]
fn abbrechen_ohne_verlaufsschritt_ok_genau_einer_nichts_gemerkt() {
    let mut h = harness();
    let d0 = doc(&h);
    let n0 = undo_count(&h);
    open(&mut h);
    bend_curve(&mut h);
    assert!(*doc(&h) != *d0, "Vorschau wirkt im Dokument");
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    assert!(st(&h).dialogs.curves.is_none());
    assert!(*doc(&h) == *d0, "Abbrechen stellt das Original her");
    assert_eq!(undo_count(&h), n0, "Abbrechen ohne Verlaufsschritt");
    // OK: genau ein Schritt
    open(&mut h);
    bend_curve(&mut h);
    let okp = client_origin(&h) + egui::vec2(273.0 + 49.0, 11.0 + 11.0);
    click(&mut h, okp);
    assert!(st(&h).dialogs.curves.is_none(), "OK schließt");
    assert_eq!(undo_count(&h), n0 + 1, "OK = ein Verlaufsschritt");
    assert!(*doc(&h) != *d0);
    // nichts gemerkt: nächster Aufruf startet mit Identität, RGB, Punkt 0
    open(&mut h);
    let c = curves(&h);
    assert_eq!(c.params, CurvesParams::default());
    assert_eq!((c.channel, c.selected, c.reversed), (0, Some(0), false));
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    assert_eq!(undo_count(&h), n0 + 1);
}

#[test]
fn vorschau_schalter_app_weit_auch_bei_abbrechen() {
    let mut h = harness();
    let d0 = doc(&h);
    open(&mut h);
    assert!(curves(&h).preview.preview_on, "Vorgabe an");
    bend_curve(&mut h);
    assert!(*doc(&h) != *d0);
    // Vorschau aus: Dokument zeigt das Original, Schalter sofort gemerkt
    let pv = client_origin(&h) + egui::vec2(271.0 + 7.0, 136.0 + 8.0);
    click(&mut h, pv);
    assert!(!curves(&h).preview.preview_on);
    assert!(*doc(&h) == *d0, "Vorschau aus = Original");
    assert!(!st(&h).session.prefs.get_bool(PREVIEW_PREF, true), "sofort gemerkt");
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    assert!(!st(&h).session.prefs.get_bool(PREVIEW_PREF, true), "auch nach Abbrechen");
    open(&mut h);
    assert!(!curves(&h).preview.preview_on, "nächster Aufruf ohne Vorschau");
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    assert!(*doc(&h) == *d0);
}

#[test]
fn pipetten_nehmen_die_angezeigte_farbe() {
    let mut h = harness();
    open(&mut h);
    // Dialog an der Titelleiste zur Seite ziehen, damit die Leinwand frei ist
    let r = fp_ui::dialogs::dialog_rect(&h.ctx, "fp-curves").expect("Dialog");
    let t = r.min + egui::vec2(60.0, 12.0);
    drag(&mut h, t, t + egui::vec2(480.0, 0.0), 8);
    assert!(fp_ui::dialogs::dialog_rect(&h.ctx, "fp-curves").expect("Dialog").left() > r.left() + 400.0, "Dialog verschoben");
    // Schatten auf (200,100,50) (drittes Feld: x = 10 + 132…198, y = 10 + 60…120)
    h.get_by_label(&fp_i18n::t("dlg-pipette-shadow")).click();
    h.run_steps(2);
    assert_eq!(curves(&h).picking, Some(Pipette::Shadow), "Aufnahmemodus");
    let p = screen_of(&h, 170.0, 115.0);
    click(&mut h, p);
    assert!(st(&h).dialogs.curves.is_some(), "Klick auf die Leinwand schließt den Dialog nicht");
    assert_eq!(curves(&h).picking, None);
    assert_eq!(curves(&h).params.red_points, pts(&[(200.0, 0.0), (255.0, 255.0)]));
    assert_eq!(curves(&h).params.green_points, pts(&[(100.0, 0.0), (255.0, 255.0)]));
    assert_eq!(curves(&h).params.blue_points, pts(&[(50.0, 0.0), (255.0, 255.0)]));
    h.run_steps(3);
    // Mittelton auf (40,120,220): angezeigt wird mit Vorschau schon (0,33,211)
    h.get_by_label(&fp_i18n::t("dlg-pipette-midtone")).click();
    h.run_steps(2);
    let p = screen_of(&h, 240.0, 115.0);
    click(&mut h, p);
    assert_eq!(curves(&h).params.red_points, pts(&[(200.0, 0.0), (202.0, 127.5), (255.0, 255.0)]));
    assert_eq!(curves(&h).params.green_points, pts(&[(100.0, 0.0), (102.0, 127.5), (255.0, 255.0)]));
    assert_eq!(curves(&h).params.blue_points, pts(&[(50.0, 0.0), (211.0, 127.5), (255.0, 255.0)]));
    // Esc im Aufnahmemodus beendet nur die Aufnahme
    h.get_by_label(&fp_i18n::t("dlg-pipette-highlight")).click();
    h.run_steps(2);
    assert_eq!(curves(&h).picking, Some(Pipette::Highlight));
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    assert!(st(&h).dialogs.curves.is_some(), "Dialog bleibt offen");
    assert_eq!(curves(&h).picking, None);
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    assert!(st(&h).dialogs.curves.is_none());
}

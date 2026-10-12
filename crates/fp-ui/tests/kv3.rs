//! KV-3: Kurven und Tonwert als Live-Effekt („+“ öffnet den Dialog, ⓘ bearbeitet mit den
//! gespeicherten Werten, Abbrechen ändert nichts) und die Dialoge „Bitmap-Auswahl speichern /
//! wiederherstellen / löschen“ nach re\19-kurven-tonwert.md §1.1, §5 und §6.

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{By, Queryable};
use egui_kittest::Harness;
use fp_engine::{DocId, Session};
use fp_model::effects::{Effect, EffectKind};
use fp_raster::filters::CurvePoint;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::bitmap_selection::{BitmapSelectionDialog, Kind};
use fp_ui::dialogs::effects::{self, FxKind, FX};
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness_with(s: Session) -> H {
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

fn stm(h: &mut H) -> &mut AppState {
    &mut h.state_mut().state
}

fn undo_count(h: &H) -> usize {
    st(h).session.active().expect("Dokument").history.undo_count()
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

/// Knopf mit der Beschriftung drücken (bei mehreren der oberste, z. B. in der Rückfrage).
fn press(h: &mut H, label: &str) {
    h.run_steps(1);
    let r = h.get_all(By::new().label(label)).last().unwrap_or_else(|| panic!("Knopf {label}")).rect();
    click(h, egui::pos2(r.center().x as f32, r.center().y as f32));
}

// ------------------------------------------------------------------ Live-Effekt

fn effect_harness() -> H {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("Dokument");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 180, 140], "fillColor": "#808080", "strokeWidth": 0})).expect("Rechteck");
    harness_with(s)
}

fn effects_of(h: &H) -> Vec<Effect> {
    let d = st(h).session.active().expect("Dokument");
    let id = d.selection.elements[0];
    fp_engine::docops::element_on_page(&d.doc, d.page, id).and_then(|e| e.common.effects.clone()).map(|l| l.effects).unwrap_or_default()
}

fn fx_index(cmd: &str) -> usize {
    FX.iter().position(|f| f.kind == FxKind::Filter(match cmd {
        "curves" => "filter.adjustColor.curves",
        _ => "filter.adjustColor.levels",
    })).expect("Menüeintrag")
}

fn pts(v: &[(f32, f32)]) -> Vec<CurvePoint> {
    v.iter().map(|&(x, y)| CurvePoint { x, y }).collect()
}

fn stored_points(e: &Effect, key: &str) -> Vec<(f32, f32)> {
    e.get_points(key).expect("Punktliste").iter().map(|p| (p.x, p.y)).collect()
}

#[test]
fn curves_plus_opens_dialog_cancel_adds_nothing_ok_appends() {
    let mut h = effect_harness();
    let u0 = undo_count(&h);
    effects::begin_add(stm(&mut h), fx_index("curves"));
    h.run_steps(3);
    let d = st(&h).dialogs.curves.as_ref().expect("„+“ öffnet den Kurvendialog");
    assert!(d.live && d.edit.is_none());
    assert_eq!(d.params, fp_raster::filters::CurvesParams::default(), "Grundwerte (Identität)");
    // Werte ändern → Vorschau zeigt den Effekt vorläufig
    stm(&mut h).dialogs.curves.as_mut().unwrap().params.rgb_points = pts(&[(0.0, 0.0), (64.0, 124.0), (255.0, 255.0)]);
    h.run_steps(3);
    assert_eq!(effects_of(&h).len(), 1, "Vorschau als Effekt");
    press(&mut h, "Abbrechen");
    assert!(st(&h).dialogs.curves.is_none());
    assert!(effects_of(&h).is_empty(), "Abbrechen beim Anlegen: kein Effekt");
    assert_eq!(undo_count(&h), u0, "kein Verlaufsschritt");

    // Anlegen mit OK: Effekt am Ende der Liste, alle vier Listen geschrieben
    effects::begin_add(stm(&mut h), fx_index("curves"));
    h.run_steps(3);
    stm(&mut h).dialogs.curves.as_mut().unwrap().params.rgb_points = pts(&[(0.0, 0.0), (64.0, 124.0), (255.0, 255.0)]);
    h.run_steps(3);
    press(&mut h, "OK");
    let fx = effects_of(&h);
    assert_eq!(fx.len(), 1);
    assert_eq!(fx[0].kind(), EffectKind::Curves);
    assert_eq!(stored_points(&fx[0], "rgb_points"), vec![(0.0, 0.0), (64.0, 124.0), (255.0, 255.0)]);
    for k in ["red_points", "green_points", "blue_points"] {
        assert_eq!(stored_points(&fx[0], k), vec![(0.0, 0.0), (255.0, 255.0)], "{k}");
    }
    assert!(effects::editable(&fx[0]), "Kurven-Effekt hat ein ⓘ");
    assert_eq!(undo_count(&h), u0 + 1, "OK = ein Schritt");
}

#[test]
fn curves_info_edits_stored_values_cancel_keeps_ok_replaces() {
    let mut h = effect_harness();
    effects::begin_add(stm(&mut h), fx_index("curves"));
    h.run_steps(3);
    stm(&mut h).dialogs.curves.as_mut().unwrap().params.rgb_points = pts(&[(0.0, 0.0), (64.0, 129.0), (255.0, 255.0)]);
    h.run_steps(3);
    press(&mut h, "OK");
    let u1 = undo_count(&h);

    // ⓘ: Dialog startet mit den gespeicherten Werten; Abbrechen lässt sie unverändert
    let e = effects_of(&h)[0].clone();
    effects::begin_edit(stm(&mut h), 0, &e);
    h.run_steps(3);
    let d = st(&h).dialogs.curves.as_ref().expect("ⓘ öffnet den Kurvendialog");
    assert_eq!(d.params.rgb_points, pts(&[(0.0, 0.0), (64.0, 129.0), (255.0, 255.0)]));
    assert_eq!(d.edit.as_ref().map(|x| x.index), Some(0));
    stm(&mut h).dialogs.curves.as_mut().unwrap().params.rgb_points = pts(&[(0.0, 0.0), (64.0, 201.0), (255.0, 255.0)]);
    h.run_steps(3);
    assert_eq!(stored_points(&effects_of(&h)[0], "rgb_points")[1], (64.0, 201.0), "Vorschau mit den Dialogwerten");
    press(&mut h, "Abbrechen");
    let fx = effects_of(&h);
    assert_eq!(fx.len(), 1);
    assert_eq!(stored_points(&fx[0], "rgb_points")[1], (64.0, 129.0), "Abbrechen: Werte unverändert");
    assert_eq!(undo_count(&h), u1);

    // OK ersetzt die Werte, kein zweiter Effekt, ein Schritt
    effects::begin_edit(stm(&mut h), 0, &fx[0]);
    h.run_steps(3);
    stm(&mut h).dialogs.curves.as_mut().unwrap().params.rgb_points = pts(&[(0.0, 0.0), (64.0, 201.0), (255.0, 255.0)]);
    h.run_steps(3);
    press(&mut h, "OK");
    let fx = effects_of(&h);
    assert_eq!(fx.len(), 1, "ersetzt, nicht angehängt");
    assert_eq!(stored_points(&fx[0], "rgb_points"), vec![(0.0, 0.0), (64.0, 201.0), (255.0, 255.0)]);
    assert_eq!(undo_count(&h), u1 + 1);
    // Rückgängig stellt die alten Werte wieder her
    stm(&mut h).session.undo().expect("Rückgängig");
    assert_eq!(stored_points(&effects_of(&h)[0], "rgb_points")[1], (64.0, 129.0));
}

#[test]
fn levels_live_add_edit_cancel_and_ok() {
    let mut h = effect_harness();
    let u0 = undo_count(&h);
    effects::begin_add(stm(&mut h), fx_index("levels"));
    h.run_steps(3);
    assert!(st(&h).dialogs.levels.as_ref().expect("„+“ öffnet die Tonwertkorrektur").live);
    press(&mut h, "Abbrechen");
    assert!(effects_of(&h).is_empty(), "Abbrechen beim Anlegen: kein Effekt");
    assert_eq!(undo_count(&h), u0);

    effects::begin_add(stm(&mut h), fx_index("levels"));
    h.run_steps(3);
    {
        let d = stm(&mut h).dialogs.levels.as_mut().unwrap();
        d.params.rgb.source_low = 30;
        d.params.red.gamma = 2.0;
    }
    h.run_steps(3);
    press(&mut h, "OK");
    let fx = effects_of(&h);
    assert_eq!(fx.len(), 1);
    assert_eq!(fx[0].kind(), EffectKind::Levels);
    assert_eq!(fx[0].get("source_low_rgb"), Some("30"));
    assert_eq!(fx[0].get_f64("gamma_red"), Some(2.0));
    assert_eq!(fx[0].get("source_high_blue"), Some("255"));
    assert!(effects::editable(&fx[0]));
    let u1 = undo_count(&h);

    effects::begin_edit(stm(&mut h), 0, &fx[0]);
    h.run_steps(3);
    let d = st(&h).dialogs.levels.as_ref().expect("ⓘ öffnet die Tonwertkorrektur");
    assert_eq!((d.params.rgb.source_low, d.params.red.gamma), (30, 2.0), "gespeicherte Werte");
    stm(&mut h).dialogs.levels.as_mut().unwrap().params.rgb.source_low = 80;
    h.run_steps(3);
    press(&mut h, "Abbrechen");
    assert_eq!(effects_of(&h)[0].get("source_low_rgb"), Some("30"), "Abbrechen: unverändert");
    assert_eq!(undo_count(&h), u1);

    let e = effects_of(&h)[0].clone();
    effects::begin_edit(stm(&mut h), 0, &e);
    h.run_steps(3);
    stm(&mut h).dialogs.levels.as_mut().unwrap().params.rgb.source_low = 80;
    h.run_steps(3);
    press(&mut h, "OK");
    let fx = effects_of(&h);
    assert_eq!(fx.len(), 1);
    assert_eq!(fx[0].get("source_low_rgb"), Some("80"));
    assert_eq!(fx[0].get_f64("gamma_red"), Some(2.0), "andere Werte bleiben");
    assert_eq!(undo_count(&h), u1 + 1);
}

// ------------------------------------------------------------------ Bitmap-Auswahl

/// Drei Dokumente: A und B 400 × 300, C 100 × 100; aktiv ist A mit einer Pixelauswahl.
fn selection_harness() -> (H, [DocId; 3]) {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 400, "height": 300, "background": "#ffffff"})).expect("A");
    s.execute("file.new", json!({"width": 400, "height": 300, "background": "#ffffff"})).expect("B");
    s.execute("file.new", json!({"width": 100, "height": 100, "background": "#ffffff"})).expect("C");
    let ids: Vec<DocId> = s.documents().iter().map(|d| d.id).collect();
    s.activate(ids[0]).expect("A aktiv");
    s.execute("select.pixels", json!({"shape": "rect", "rect": [10, 10, 50, 50]})).expect("Auswahlrahmen");
    (harness_with(s), [ids[0], ids[1], ids[2]])
}

fn saved(h: &H, id: DocId) -> Vec<String> {
    fp_ui::dialogs::bitmap_selection::saved_names(st(h), id)
}

fn pixel(h: &H, x: i32, y: i32) -> bool {
    st(h).session.active().unwrap().selection.pixels.as_ref().is_some_and(|m| m.get(x, y) > 127)
}

fn bsel(h: &H) -> &BitmapSelectionDialog {
    st(h).dialogs.bitmap_selection.as_ref().expect("Dialog offen")
}

#[test]
fn save_dialog_lists_same_size_documents_and_new_entry() {
    let (mut h, [a, b, c]) = selection_harness();
    assert!(fp_ui::dialogs::open_for_command(stm(&mut h), "select.saveBitmapSelection"));
    h.run_steps(3);
    let d = bsel(&h);
    let docs: Vec<DocId> = d.docs.iter().map(|x| x.0).collect();
    assert_eq!(docs, vec![a, b], "nur Dokumente gleicher Größe, C fehlt ({c:?})");
    assert_eq!(d.doc_id(), Some(a), "vorgewählt das aktuelle");
    assert_eq!(d.selection_items(), vec!["Neu".to_string()]);
    assert_eq!(d.name, "Standard");
    assert!(d.name_enabled());
    assert_eq!(d.op, 0);
    assert_eq!(d.op_labels()[0], "Neue Auswahl");
    assert!((1..4).all(|k| !d.op_enabled(k)), "Verknüpfungen grau bei „Neu“");
    let u0 = undo_count(&h);
    press(&mut h, "OK");
    assert!(st(&h).dialogs.bitmap_selection.is_none());
    assert_eq!(saved(&h, a), vec!["Standard".to_string()]);
    assert_eq!(undo_count(&h), u0 + 1, "eigener Verlaufsschritt");
    assert_eq!(st(&h).session.undo_redo_labels().0.as_deref(), Some("Auswahl speichern"));
    // Rückgängig nimmt das Speichern zurück
    stm(&mut h).session.undo().expect("Rückgängig");
    assert!(saved(&h, a).is_empty());
    stm(&mut h).session.redo().expect("Wiederholen");
    assert_eq!(saved(&h, a).len(), 1);

    // Zweiter Aufruf: wieder „Neu“, „Standard“; Liste mit dem vorhandenen Namen; nichts gemerkt
    assert!(fp_ui::dialogs::open_for_command(stm(&mut h), "select.saveBitmapSelection"));
    h.run_steps(2);
    let d = bsel(&h).clone();
    assert_eq!(d.selection_items(), vec!["Neu".to_string(), "Standard".to_string()]);
    assert_eq!((d.sel, d.name.as_str(), d.op), (0, "Standard", 0));
    // vorhandene Auswahl gewählt: Name grau, Verknüpfungen aktiv, erstes Feld „Auswahl ersetzen“
    let mut d2 = d.clone();
    d2.set_selection(1);
    assert!(!d2.name_enabled());
    assert!((0..4).all(|k| d2.op_enabled(k)));
    assert_eq!(d2.op_labels()[0], "Auswahl ersetzen");
    d2.op = 2;
    d2.set_selection(0);
    assert_eq!(d2.op, 0, "zurück auf „Neu“: Vorgang wieder „Neue Auswahl“");
    // Dokument wechseln: Liste neu gefüllt (B hat keine Auswahl)
    let mut d3 = d.clone();
    d3.set_doc(st(&h), 1);
    assert_eq!(d3.selection_items(), vec!["Neu".to_string()]);
    press(&mut h, "Abbrechen");
}

#[test]
fn same_name_asks_cancel_returns_ok_replaces() {
    let (mut h, [a, ..]) = selection_harness();
    stm(&mut h).session.execute("select.saveBitmapSelection", json!({})).expect("speichern");
    // andere Pixelauswahl, wieder unter „Standard“ speichern
    stm(&mut h).session.execute("select.pixels", json!({"shape": "rect", "rect": [200, 100, 260, 160]})).expect("Rahmen");
    assert!(fp_ui::dialogs::open_for_command(stm(&mut h), "select.saveBitmapSelection"));
    h.run_steps(3);
    assert!(bsel(&h).needs_confirm());
    let u0 = undo_count(&h);
    press(&mut h, "OK");
    assert!(bsel(&h).confirm, "Rückfrage offen");
    // Abbrechen → zurück in den offenen Dialog, nichts gespeichert
    press(&mut h, "Abbrechen");
    assert!(!bsel(&h).confirm, "Dialog bleibt offen");
    assert_eq!(undo_count(&h), u0);
    // OK → Rückfrage → OK ersetzt (kein zweiter Eintrag)
    press(&mut h, "OK");
    assert!(bsel(&h).confirm);
    press(&mut h, "OK");
    assert!(st(&h).dialogs.bitmap_selection.is_none());
    assert_eq!(saved(&h, a), vec!["Standard".to_string()], "ersetzt, nicht angehängt");
    assert_eq!(undo_count(&h), u0 + 1);
    // geladen wird jetzt der neue Rahmen
    stm(&mut h).session.execute("select.none", json!({})).expect("aufheben");
    stm(&mut h).session.execute("select.restoreBitmapSelection", json!({"name": "Standard"})).expect("laden");
    assert!(pixel(&h, 220, 120) && !pixel(&h, 20, 20));
}

#[test]
fn save_into_existing_with_operations_and_into_other_document() {
    let (mut h, [a, b, _]) = selection_harness();
    let s = &mut stm(&mut h).session;
    s.execute("select.saveBitmapSelection", json!({"name": "X"})).expect("X");
    s.execute("select.pixels", json!({"shape": "rect", "rect": [40, 10, 90, 50]})).expect("Rahmen 2");
    // hinzufügen
    s.execute("select.saveBitmapSelection", json!({"name": "X", "mode": "add"})).expect("add");
    s.execute("select.none", json!({})).unwrap();
    s.execute("select.restoreBitmapSelection", json!({"name": "X"})).unwrap();
    assert!(pixel(&h, 15, 20) && pixel(&h, 85, 20), "Vereinigung");
    let s = &mut stm(&mut h).session;
    s.execute("select.pixels", json!({"shape": "rect", "rect": [40, 10, 90, 50]})).unwrap();
    s.execute("select.saveBitmapSelection", json!({"name": "X", "mode": "intersect"})).expect("intersect");
    s.execute("select.none", json!({})).unwrap();
    s.execute("select.restoreBitmapSelection", json!({"name": "X"})).unwrap();
    assert!(!pixel(&h, 15, 20) && pixel(&h, 45, 20), "Schnittmenge");
    let s = &mut stm(&mut h).session;
    s.execute("select.pixels", json!({"shape": "rect", "rect": [40, 10, 70, 50]})).unwrap();
    s.execute("select.saveBitmapSelection", json!({"name": "X", "mode": "subtract"})).expect("subtract");
    s.execute("select.none", json!({})).unwrap();
    s.execute("select.restoreBitmapSelection", json!({"name": "X"})).unwrap();
    assert!(!pixel(&h, 45, 20) && pixel(&h, 75, 20), "abgezogen");

    // in ein anderes Dokument gleicher Größe speichern: Schritt im Zieldokument
    let s = &mut stm(&mut h).session;
    let ub = s.document(b).unwrap().history.undo_count();
    s.execute("select.saveBitmapSelection", json!({"name": "Fremd", "document": b.0})).expect("nach B");
    assert_eq!(saved(&h, b), vec!["Fremd".to_string()]);
    assert!(!saved(&h, a).contains(&"Fremd".to_string()));
    assert_eq!(st(&h).session.document(b).unwrap().history.undo_count(), ub + 1);
    // falsche Größe wird abgelehnt
    let c = st(&h).session.documents()[2].id;
    assert!(stm(&mut h).session.execute("select.saveBitmapSelection", json!({"document": c.0})).is_err());
}

#[test]
fn restore_dialog_rules_invert_and_operations() {
    let (mut h, [a, b, _]) = selection_harness();
    stm(&mut h).session.execute("select.saveBitmapSelection", json!({"name": "Eins"})).unwrap();
    stm(&mut h).session.execute("select.pixels", json!({"shape": "rect", "rect": [100, 100, 150, 150]})).unwrap();
    stm(&mut h).session.execute("select.saveBitmapSelection", json!({"name": "Zwei", "document": b.0})).unwrap();

    // mit Pixelauswahl: alle Vorgänge aktiv; Quelle = Dokumente gleicher Größe mit Auswahlen
    assert!(fp_ui::dialogs::open_for_command(stm(&mut h), "select.restoreBitmapSelection"));
    h.run_steps(3);
    let d = bsel(&h).clone();
    assert_eq!(d.kind, Kind::Restore);
    assert_eq!(d.docs.iter().map(|x| x.0).collect::<Vec<_>>(), vec![a, b]);
    assert_eq!(d.names, vec!["Eins".to_string()]);
    assert_eq!((d.sel, d.op, d.invert), (0, 0, false), "erste Auswahl, Neue Auswahl, Umkehren aus");
    assert!((0..4).all(|k| d.op_enabled(k)));
    press(&mut h, "Abbrechen");

    // ohne Pixelauswahl: Hinzufügen/Abziehen/Schnittmenge grau
    stm(&mut h).session.execute("select.none", json!({})).unwrap();
    assert!(fp_ui::dialogs::open_for_command(stm(&mut h), "select.restoreBitmapSelection"));
    h.run_steps(3);
    assert!((1..4).all(|k| !bsel(&h).op_enabled(k)));
    // Umkehren ankreuzen und OK: alles außer dem gespeicherten Rechteck, Schritt „Auswahl laden“
    stm(&mut h).dialogs.bitmap_selection.as_mut().unwrap().invert = true;
    let u0 = undo_count(&h);
    press(&mut h, "OK");
    assert!(!pixel(&h, 20, 20) && pixel(&h, 200, 200), "umgekehrt geladen");
    assert_eq!(undo_count(&h), u0 + 1);
    assert_eq!(st(&h).session.undo_redo_labels().0.as_deref(), Some("Auswahl laden"));

    // aus Dokument B in A laden, mit Hinzufügen zur bestehenden Auswahl
    stm(&mut h).session.execute("select.pixels", json!({"shape": "rect", "rect": [10, 10, 50, 50]})).unwrap();
    assert!(fp_ui::dialogs::open_for_command(stm(&mut h), "select.restoreBitmapSelection"));
    h.run_steps(2);
    {
        let st2 = stm(&mut h);
        let mut d = st2.dialogs.bitmap_selection.take().unwrap();
        d.set_doc(st2, 1);
        assert_eq!(d.names, vec!["Zwei".to_string()]);
        d.op = 1;
        st2.dialogs.bitmap_selection = Some(d);
    }
    h.run_steps(2);
    press(&mut h, "OK");
    assert!(pixel(&h, 20, 20) && pixel(&h, 120, 120), "hinzugefügt");
}

#[test]
fn delete_dialog_removes_chosen_selection() {
    let (mut h, [a, ..]) = selection_harness();
    stm(&mut h).session.execute("select.saveBitmapSelection", json!({"name": "Eins"})).unwrap();
    stm(&mut h).session.execute("select.saveBitmapSelection", json!({"name": "Zwei"})).unwrap();
    assert!(fp_ui::dialogs::open_for_command(stm(&mut h), "select.deleteBitmapSelection"));
    h.run_steps(3);
    assert_eq!(bsel(&h).names, vec!["Eins".to_string(), "Zwei".to_string()]);
    stm(&mut h).dialogs.bitmap_selection.as_mut().unwrap().set_selection(1);
    press(&mut h, "Abbrechen");
    assert_eq!(saved(&h, a).len(), 2, "Abbrechen löscht nichts");
    assert!(fp_ui::dialogs::open_for_command(stm(&mut h), "select.deleteBitmapSelection"));
    h.run_steps(2);
    stm(&mut h).dialogs.bitmap_selection.as_mut().unwrap().set_selection(1);
    press(&mut h, "OK");
    assert_eq!(saved(&h, a), vec!["Eins".to_string()], "ohne Rückfrage gelöscht");
    stm(&mut h).session.undo().unwrap();
    assert_eq!(saved(&h, a).len(), 2);
}

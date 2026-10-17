//! DLG-5 (P-DLG5, Messung PM-6, PARITAET_B D41/D42/D45/D46/D48 und gemerkte Werte D26/D35/D41–D50):
//! - gemerkte Werte: OK merkt die bestätigten Werte für das nächste Öffnen, Abbrechen und Esc nicht
//! - Kennlinie der Sättigung linear −100…100, 0 in der Mitte
//! - Anordnung der Filterdialoge: Feld rechts neben dem Schieber, Knöpfe rechts, „Vorschau“ und
//!   „Kolorieren“ in der Knopfspalte; Beschriftungen, Startfokus, Dezimalpunkt „4.1“
//! - Verlaufsname „Bild filtern“ beim Filtern von Pixeln
//!
//! Realistische Eingaben: Klick ins Feld, Strg+A, Tippen wie egui-winit (Taste + Text), Tab; OK und
//! Abbrechen per Mausklick, Esc als Taste.

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::{dialog_rect, last_dialog_id};
use fp_ui::FirepixelApp;
use serde_json::{json, Value};

type H = Harness<'static, FirepixelApp>;

/// Dokument 300 × 200 mit einer Bitmap B1 (aus einem Rechteck reduziert), ausgewählt.
fn session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [40, 40, 140, 120], "fillColor": "#3366cc", "name": "B1"})).expect("rect");
    s.execute("modify.flattenSelection", json!({})).expect("reduzieren");
    let d = s.active().expect("doc");
    assert_eq!(d.selection.elements.len(), 1, "B1 ausgewählt");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, d.selection.elements[0]).expect("B1");
    assert!(matches!(e.kind, fp_model::ElementKind::Bitmap(_)), "B1 ist eine Bitmap");
    s
}

fn harness() -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, session(), UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn key_m(h: &mut H, k: Key, m: Modifiers) {
    if m.any() {
        h.event(Event::ModifiersChanged(m));
    }
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: m });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: m });
    if m.any() {
        ev(h, Event::ModifiersChanged(Modifiers::NONE));
    }
}

fn key(h: &mut H, k: Key) {
    key_m(h, k, Modifiers::NONE);
}

/// Tippen wie egui-winit: Taste drücken + Text, Taste loslassen.
fn typ(h: &mut H, s: &str) {
    for c in s.chars() {
        let k = Key::from_name(&c.to_string());
        if let Some(k) = k {
            h.event(Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: Modifiers::NONE });
        }
        ev(h, Event::Text(c.to_string()));
        if let Some(k) = k {
            ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: Modifiers::NONE });
        }
    }
}

fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
}

fn open(h: &mut H, cmd: &str) {
    h.state_mut().state.run(cmd, json!({}));
    h.run_steps(4);
    assert!(st(h).dialogs.any_open(), "{cmd}: Dialog offen");
}

fn dlg_rect(h: &H) -> Rect {
    dialog_rect(&h.ctx, &last_dialog_id(&h.ctx).expect("Dialog")).expect("Rechteck")
}

/// Zahlenfelder des Dialogs von oben nach unten (Rechteck, Text).
fn text_fields(h: &H) -> Vec<(Rect, String)> {
    let dr = dlg_rect(h);
    let mut v: Vec<(Rect, String)> = h.get_all(By::new().role(Role::TextInput)).map(|n| (n.rect(), n.value().unwrap_or_default())).filter(|(r, _)| dr.contains(r.center())).collect();
    v.sort_by(|a, b| a.0.top().total_cmp(&b.0.top()));
    v
}

/// Schieberleisten des Dialogs von oben nach unten (Rechteck, Position).
fn sliders(h: &H) -> Vec<(Rect, f64)> {
    let dr = dlg_rect(h);
    let mut v: Vec<(Rect, f64)> = h.query_all(By::new().role(Role::Slider)).map(|n| (n.rect(), n.accesskit_node().numeric_value().unwrap_or(f64::NAN))).filter(|(r, _)| dr.contains(r.center())).collect();
    v.sort_by(|a, b| a.0.top().total_cmp(&b.0.top()));
    v
}

fn button(h: &H, label: &str) -> Rect {
    let dr = dlg_rect(h);
    h.query_all(By::new().role(Role::Button).label(label)).map(|n| n.rect()).find(|r| dr.contains(r.center())).unwrap_or_else(|| panic!("Knopf {label}"))
}

/// Gezeichnete Kästchen (Beschriftung → Rechteck) im Dialog (Layout-Protokoll der Felder).
fn checks(h: &mut H) -> Vec<(String, Rect)> {
    let dr = dlg_rect(h);
    fp_ui::inspector::fields::set_audit(&h.ctx, true);
    h.run_steps(1);
    let v = fp_ui::inspector::fields::take_audit(&h.ctx).into_iter().filter(|a| a.kind == "check" && dr.contains(a.drawn.center())).map(|a| (a.text, a.drawn)).collect();
    fp_ui::inspector::fields::set_audit(&h.ctx, false);
    v
}

/// Gezeichnete Beschriftungen im Dialog.
fn labels(h: &mut H) -> Vec<String> {
    let dr = dlg_rect(h);
    fp_ui::inspector::fields::set_audit(&h.ctx, true);
    h.run_steps(1);
    let v = fp_ui::inspector::fields::take_audit(&h.ctx).into_iter().filter(|a| dr.contains(a.drawn.center())).map(|a| a.text).collect();
    fp_ui::inspector::fields::set_audit(&h.ctx, false);
    v
}

/// Zahl `i` (von oben) eintippen: Klick, Strg+A, Text, Tab.
fn type_field(h: &mut H, i: usize, text: &str) {
    let r = text_fields(h)[i].0;
    click(h, r.center());
    key_m(h, Key::A, Modifiers { ctrl: true, command: true, ..Default::default() });
    typ(h, text);
    key(h, Key::Tab);
    h.run_steps(2);
}

fn form_value(h: &H, key: &str) -> Value {
    st(h).dialogs.form.as_ref().expect("Formular").get(key)
}

fn set_check(h: &mut H, key: &str, on: bool) {
    h.state_mut().state.dialogs.form.as_mut().expect("Formular").field_mut(key).expect("Kästchen").value = json!(on);
    h.run_steps(2);
}

fn press_ok(h: &mut H) {
    let r = button(h, "OK");
    click(h, r.center());
    h.run_steps(4);
    assert!(!st(h).dialogs.any_open(), "OK schließt");
}

fn press_cancel(h: &mut H) {
    let r = button(h, "Abbrechen");
    click(h, r.center());
    h.run_steps(4);
    assert!(!st(h).dialogs.any_open(), "Abbrechen schließt");
}

fn undo_label(h: &H) -> Option<String> {
    st(h).session.active().and_then(|d| d.history.undo_label().map(str::to_string))
}

/// Je Filterdialog: Zahlenfelder (in der Reihenfolge von oben) und Kästchen mit Werten fürs OK
/// (PM-6: Farbton/Sättigung 30/50, Helligkeit/Kontrast 20/−30, Rauschen 20, Unscharf maskieren
/// 100/2, Bewegungsunschärfe 90/20, Gauß 2, Radial/Zentriert 10/20) und für das Abbrechen danach.
struct Case {
    cmd: &'static str,
    numbers: &'static [(&'static str, &'static str, f64)],
    other: &'static [&'static str],
    check: Option<(&'static str, bool)>,
}

const CASES: &[Case] = &[
    Case { cmd: "filter.adjustColor.hueSaturation", numbers: &[("hue", "30", 30.0), ("saturation", "50", 50.0), ("lightness", "-10", -10.0)], other: &["-40", "-60", "5"], check: Some(("colorize", true)) },
    Case { cmd: "filter.adjustColor.brightnessContrast", numbers: &[("brightness", "20", 20.0), ("contrast", "-30", -30.0)], other: &["7", "8"], check: None },
    Case { cmd: "filter.noise.addNoise", numbers: &[("amount", "20", 20.0)], other: &["70"], check: Some(("use_color", false)) },
    Case { cmd: "filter.sharpen.unsharpMask", numbers: &[("amount", "100", 100.0), ("radius", "2", 2.0), ("threshold", "5", 5.0)], other: &["60", "3.5", "9"], check: None },
    Case { cmd: "filter.blur.motion", numbers: &[("angle", "45", 45.0), ("distance", "20", 20.0)], other: &["10", "5"], check: None },
    Case { cmd: "filter.blur.gaussian", numbers: &[("radius", "2", 2.0)], other: &["7.5"], check: None },
    Case { cmd: "filter.blur.radial", numbers: &[("amount", "10", 10.0), ("quality", "30", 30.0)], other: &["50", "60"], check: None },
    Case { cmd: "filter.blur.zoom", numbers: &[("amount", "10", 10.0), ("quality", "30", 30.0)], other: &["50", "60"], check: None },
];

/// Alle Filterdialoge (D41, D42, D45–D50): OK merkt die Werte (zweites und drittes Öffnen zeigen
/// sie), Abbrechen per Knopf und Esc merken die geänderten Werte nicht; OK legt genau einen
/// Verlaufsschritt „Bild filtern“ an.
#[test]
fn filter_dialogs_remember_ok_values_not_cancel() {
    let mut h = harness();
    for c in CASES {
        // vor dem Öffnen zählen: die Vorschau ist schon beim Öffnen angewandt
        let n0 = st(&h).session.active().map(|d| d.history.undo_count()).unwrap_or(0);
        open(&mut h, c.cmd);
        for (i, (_, text, _)) in c.numbers.iter().enumerate() {
            type_field(&mut h, i, text);
        }
        if let Some((k, v)) = c.check {
            set_check(&mut h, k, v);
        }
        for (k, _, v) in c.numbers {
            assert_eq!(form_value(&h, k).as_f64(), Some(*v), "{}: {k} getippt", c.cmd);
        }
        press_ok(&mut h);
        assert_eq!(st(&h).session.active().map(|d| d.history.undo_count()), Some(n0 + 1), "{}: ein Verlaufsschritt", c.cmd);
        assert_eq!(undo_label(&h).as_deref(), Some("Bild filtern"), "{}: Verlaufsname", c.cmd);
        // zweimal: Abbrechen per Knopf, dann per Esc – beide Male bleiben die OK-Werte
        for round in 0..2 {
            open(&mut h, c.cmd);
            for (k, _, v) in c.numbers {
                assert_eq!(form_value(&h, k).as_f64(), Some(*v), "{} Runde {round}: {k} gemerkt", c.cmd);
            }
            if let Some((k, v)) = c.check {
                assert_eq!(form_value(&h, k).as_bool(), Some(v), "{} Runde {round}: {k} gemerkt", c.cmd);
                set_check(&mut h, k, !v);
            }
            for (i, text) in c.other.iter().enumerate() {
                type_field(&mut h, i, text);
            }
            assert_ne!(form_value(&h, c.numbers[0].0).as_f64(), Some(c.numbers[0].2), "{}: geändert", c.cmd);
            if round == 0 {
                press_cancel(&mut h);
            } else {
                key(&mut h, Key::Escape);
                h.run_steps(4);
                assert!(!st(&h).dialogs.any_open(), "{}: Esc schließt", c.cmd);
            }
        }
        open(&mut h, c.cmd);
        for (k, _, v) in c.numbers {
            assert_eq!(form_value(&h, k).as_f64(), Some(*v), "{}: {k} nach Abbrechen weiter gemerkt", c.cmd);
        }
        key(&mut h, Key::Escape);
        h.run_steps(4);
    }
}

/// Ohne vorheriges OK öffnen die Dialoge mit den Vorgaben (frische Voreinstellungen).
#[test]
fn filter_dialogs_open_with_defaults_first() {
    let mut h = harness();
    for (cmd, key_, v) in [("filter.blur.gaussian", "radius", 4.1), ("filter.sharpen.unsharpMask", "amount", 50.0), ("filter.adjustColor.hueSaturation", "saturation", 0.0), ("filter.noise.addNoise", "amount", 50.0)] {
        open(&mut h, cmd);
        assert_eq!(form_value(&h, key_).as_f64(), Some(v), "{cmd}: Vorgabe");
        key(&mut h, Key::Escape);
        h.run_steps(4);
    }
}

/// Weiche Kante (D35): OK mit 5 gemerkt, Abbrechen mit 7 nicht.
#[test]
fn feather_remembers_ok_value() {
    let mut h = harness();
    h.state_mut().state.session.execute("select.pixels", json!({"shape": "rect", "rect": [50, 50, 120, 110]})).expect("Pixelauswahl");
    h.run_steps(2);
    open(&mut h, "select.feather");
    assert_eq!(form_value(&h, "radius").as_f64(), Some(10.0), "Vorgabe 10");
    type_field(&mut h, 0, "5");
    press_ok(&mut h);
    open(&mut h, "select.feather");
    assert_eq!(form_value(&h, "radius").as_f64(), Some(5.0), "gemerkt");
    type_field(&mut h, 0, "7");
    press_cancel(&mut h);
    open(&mut h, "select.feather");
    assert_eq!(form_value(&h, "radius").as_f64(), Some(5.0), "Abbrechen merkt nichts");
    key(&mut h, Key::Escape);
    h.run_steps(4);
}

/// Numerisch transformieren (D26): Skalieren 50/50 per OK gemerkt, Abbrechen mit 70 nicht.
#[test]
fn numeric_transform_remembers_ok_values() {
    let mut h = harness();
    open(&mut h, "modify.transform.numericTransform");
    {
        let d = st(&h).dialogs.numeric_transform.as_ref().expect("Dialog");
        assert_eq!((d.mode, d.sx, d.sy), (0, 100.0, 100.0), "Vorgabe");
    }
    type_field(&mut h, 0, "50");
    {
        let d = st(&h).dialogs.numeric_transform.as_ref().expect("Dialog");
        assert_eq!((d.sx, d.sy), (50.0, 50.0), "Proportionen: beide 50");
    }
    key(&mut h, Key::Enter);
    h.run_steps(4);
    assert!(st(&h).dialogs.numeric_transform.is_none(), "Enter = OK");
    open(&mut h, "modify.transform.numericTransform");
    {
        let d = st(&h).dialogs.numeric_transform.as_ref().expect("Dialog");
        assert_eq!((d.mode, d.sx, d.sy), (0, 50.0, 50.0), "gemerkt wie im Original");
    }
    type_field(&mut h, 0, "70");
    key(&mut h, Key::Escape);
    h.run_steps(4);
    assert!(st(&h).dialogs.numeric_transform.is_none(), "Esc schließt");
    open(&mut h, "modify.transform.numericTransform");
    {
        let d = st(&h).dialogs.numeric_transform.as_ref().expect("Dialog");
        assert_eq!((d.sx, d.sy), (50.0, 50.0), "Esc merkt nichts");
    }
    key(&mut h, Key::Escape);
    h.run_steps(4);
}

/// Sättigung: Kennlinie linear −100…100, 0 in der Mitte, 50 → ¾ (wie Farbton und Helligkeit).
#[test]
fn saturation_slider_is_linear_with_zero_in_the_middle() {
    let mut h = harness();
    open(&mut h, "filter.adjustColor.hueSaturation");
    let s = sliders(&h);
    assert_eq!(s.len(), 3, "Farbton, Sättigung, Helligkeit");
    assert_eq!(s[0].1, 180.0, "Farbton 0 = Mitte von 0…360");
    assert_eq!(s[1].1, 100.0, "Sättigung 0 = Mitte von 0…200");
    assert_eq!(s[2].1, 100.0, "Helligkeit 0 = Mitte von 0…200");
    type_field(&mut h, 1, "50");
    assert_eq!(sliders(&h)[1].1, 150.0, "50 → ¾");
    type_field(&mut h, 1, "-100");
    assert_eq!(sliders(&h)[1].1, 0.0, "−100 ganz links");
    // Ziehen: Mitte der Leiste ≈ 0, rechter Rand 100
    let (r, _) = sliders(&h)[1];
    let start = Pos2::new(r.left() + 5.0, r.center().y);
    ev(&mut h, Event::PointerMoved(start));
    ev(&mut h, Event::PointerButton { pos: start, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    for i in 1..=20 {
        let p = Pos2::new(start.x + (r.center().x - start.x) * i as f32 / 20.0, r.center().y);
        ev(&mut h, Event::PointerMoved(p));
    }
    ev(&mut h, Event::PointerButton { pos: Pos2::new(r.center().x, r.center().y), button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
    let v = form_value(&h, "saturation").as_f64().expect("Wert");
    assert!(v.abs() <= 6.0, "Mitte der Leiste ≈ 0, war {v}");
    assert_eq!(fp_ui::inspector::scale::SATURATION.to_pos(100.0, -100.0, 100.0), 200.0);
    key(&mut h, Key::Escape);
    h.run_steps(4);
}

/// Anordnung D41/D42/D46/D48: jedes Feld rechts neben seiner Schieberleiste (gleiche Höhe), OK und
/// Abbrechen rechts oben neben den Feldern, „Vorschau“ (und bei D41 „Kolorieren“) in der Knopfspalte
/// unter Abbrechen. D45: Feld mit ▾, „Farbe“ links, Knöpfe rechts.
#[test]
fn filter_layout_fields_beside_sliders_buttons_right() {
    let mut h = harness();
    for (cmd, n) in [("filter.adjustColor.hueSaturation", 3), ("filter.adjustColor.brightnessContrast", 2), ("filter.sharpen.unsharpMask", 3), ("filter.blur.gaussian", 1)] {
        open(&mut h, cmd);
        let (f, s) = (text_fields(&h), sliders(&h));
        assert_eq!((f.len(), s.len()), (n, n), "{cmd}: Felder und Leisten");
        for (i, ((fr, _), (sr, _))) in f.iter().zip(&s).enumerate() {
            assert!(fr.left() >= sr.right(), "{cmd} #{i}: Feld {fr:?} rechts der Leiste {sr:?}");
            assert!((fr.center().y - sr.center().y).abs() <= 6.0, "{cmd} #{i}: Feld auf Höhe der Leiste");
        }
        let (ok, cancel) = (button(&h, "OK"), button(&h, "Abbrechen"));
        let fields_right = f.iter().map(|x| x.0.right()).fold(0.0, f32::max);
        assert!(ok.left() > fields_right && cancel.left() > fields_right, "{cmd}: Knöpfe rechts der Felder");
        assert!(ok.top() < s[0].0.top(), "{cmd}: OK oben");
        assert!(cancel.top() > ok.bottom() && (cancel.left() - ok.left()).abs() < 1.0, "{cmd}: Abbrechen unter OK");
        let ch = checks(&mut h);
        let preview = ch.iter().find(|c| c.0 == "Vorschau").unwrap_or_else(|| panic!("{cmd}: Vorschau {ch:?}")).1;
        assert!(preview.left() >= ok.left() - 1.0 && preview.top() > cancel.bottom(), "{cmd}: Vorschau in der Knopfspalte");
        if cmd == "filter.adjustColor.hueSaturation" {
            let col = ch.iter().find(|c| c.0 == "Kolorieren").expect("Kolorieren").1;
            assert!(col.left() >= ok.left() - 1.0 && col.top() > cancel.bottom() && col.bottom() <= preview.top() + 1.0, "Kolorieren zwischen Abbrechen und Vorschau");
        }
        let dr = dlg_rect(&h);
        assert!(dr.width() > dr.height(), "{cmd}: quer wie im Original ({dr:?})");
        key(&mut h, Key::Escape);
        h.run_steps(4);
    }
    open(&mut h, "filter.noise.addNoise");
    let f = text_fields(&h);
    assert_eq!(f.len(), 1);
    let ok = button(&h, "OK");
    assert!(ok.left() > f[0].0.right() + 14.0, "Rauschen: Knöpfe rechts neben Feld und ▾");
    let ch = checks(&mut h);
    let farbe = ch.iter().find(|c| c.0 == "Farbe").expect("Farbe").1;
    assert!(farbe.right() < ok.left() && farbe.top() > f[0].0.bottom() - 1.0, "Farbe links unter dem Feld");
    key(&mut h, Key::Escape);
    h.run_steps(4);
}

/// Beschriftungen wie im Original (Deutsch und Englisch), „Scharfzeichnungsmenge“ ohne „%“.
#[test]
fn filter_labels_like_original() {
    let mut h = harness();
    for (cmd, want, not) in [
        ("filter.adjustColor.hueSaturation", &["Farbton:", "Sättigung:", "Helligkeit:", "Kolorieren"][..], &["Färben"][..]),
        ("filter.sharpen.unsharpMask", &["Scharfzeichnungsmenge:", "Pixelradius:", "Schwellenwert:"][..], &["Schärfe:", "%"][..]),
        ("filter.blur.gaussian", &["Weichzeichnungsradius:"][..], &["Radius:"][..]),
        ("filter.noise.addNoise", &["Menge:", "Farbe"][..], &["Stärke:"][..]),
        ("filter.adjustColor.brightnessContrast", &["Helligkeit:", "Kontrast:"][..], &[][..]),
    ] {
        open(&mut h, cmd);
        let l = labels(&mut h);
        for w in want {
            assert!(l.iter().any(|x| x == w), "{cmd}: „{w}“ fehlt in {l:?}");
        }
        for n in not {
            assert!(!l.iter().any(|x| x == n), "{cmd}: „{n}“ noch da: {l:?}");
        }
        key(&mut h, Key::Escape);
        h.run_steps(4);
    }
    fp_i18n::set_language("en");
    assert_eq!(fp_i18n::t("dlg-lbl-blur-radius"), "Blur radius:");
    assert_eq!(fp_i18n::t("dlg-lbl-sharpen-amount"), "Sharpen amount:");
    assert_eq!(fp_i18n::t("dlg-lbl-lightness"), "Lightness:");
    assert_eq!(fp_i18n::t("dlg-lbl-colorize"), "Colorize");
    fp_i18n::set_language("de");
}

/// Dezimalpunkt wie im Original auch in Deutsch: Gauß und Pixelradius zeigen „4.1“; Eingabe mit
/// Komma geht weiter.
#[test]
fn radius_fields_show_decimal_point() {
    let mut h = harness();
    open(&mut h, "filter.blur.gaussian");
    assert_eq!(text_fields(&h)[0].1, "4.1", "Gauß: 4.1");
    type_field(&mut h, 0, "2,5");
    assert_eq!(form_value(&h, "radius").as_f64(), Some(2.5), "Komma wird verstanden");
    assert_eq!(text_fields(&h)[0].1, "2.5");
    key(&mut h, Key::Escape);
    h.run_steps(4);
    open(&mut h, "filter.sharpen.unsharpMask");
    let f = text_fields(&h);
    assert_eq!((f[0].1.as_str(), f[1].1.as_str(), f[2].1.as_str()), ("50", "4.1", "0"));
    key(&mut h, Key::Escape);
    h.run_steps(4);
}

/// Startfokus: D41/D46/D48 erstes Feld (markiert, Tippen ersetzt), D45 Rauschen OK (Enter = OK).
#[test]
fn start_focus_like_original() {
    let mut h = harness();
    for (cmd, key_, text, v) in [("filter.adjustColor.hueSaturation", "hue", "30", 30.0), ("filter.sharpen.unsharpMask", "amount", "120", 120.0), ("filter.blur.gaussian", "radius", "3", 3.0)] {
        open(&mut h, cmd);
        assert!(h.ctx.text_edit_focused(), "{cmd}: Feld fokussiert");
        typ(&mut h, text);
        key(&mut h, Key::Tab);
        h.run_steps(2);
        assert_eq!(form_value(&h, key_).as_f64(), Some(v), "{cmd}: erstes Feld ganz markiert");
        key(&mut h, Key::Escape);
        h.run_steps(4);
    }
    let n0 = st(&h).session.active().map(|d| d.history.undo_count()).unwrap_or(0);
    open(&mut h, "filter.noise.addNoise");
    assert!(!h.ctx.text_edit_focused(), "Rauschen: Startfokus OK, kein Feld");
    key(&mut h, Key::Enter);
    h.run_steps(4);
    assert!(!st(&h).dialogs.any_open(), "Enter = OK");
    assert_eq!(st(&h).session.active().map(|d| d.history.undo_count()), Some(n0 + 1));
}

/// Verlaufsname: Filter auf Pixel = „Bild filtern“ (Englisch „Filter Image“), als Live-Filter auf
/// einem Vektorobjekt bleibt der Filtername; Rückgängig stellt die Bitmap her.
#[test]
fn history_name_filter_image() {
    let mut s = session();
    let before = std::sync::Arc::clone(&s.active().expect("doc").doc);
    for (cmd, p) in [("filter.blur.gaussian", json!({"radius": 2.0})), ("filter.adjustColor.invert", json!({})), ("filter.adjustColor.hueSaturation", json!({"hue": 30, "saturation": 50}))] {
        s.execute(cmd, p).unwrap_or_else(|e| panic!("{cmd}: {e}"));
        assert_eq!(s.active().unwrap().history.undo_label(), Some("Bild filtern"), "{cmd}");
    }
    for _ in 0..3 {
        s.undo().expect("undo");
    }
    assert!(*s.active().unwrap().doc == *before, "Rückgängig stellt B1 her");
    fp_i18n::set_language("en");
    s.execute("filter.blur.gaussian", json!({"radius": 2.0})).expect("Gauß");
    assert_eq!(s.active().unwrap().history.undo_label(), Some("Filter Image"));
    fp_i18n::set_language("de");
    s.execute("draw.rectangle", json!({"rect": [160, 40, 220, 100]})).expect("R");
    s.execute("filter.blur.gaussian", json!({"radius": 2.0, "live": true})).expect("live");
    assert_eq!(s.active().unwrap().history.undo_label(), Some("Gaußscher Weichzeichner"), "Live-Filter: Filtername");
}

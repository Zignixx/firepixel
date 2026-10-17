//! NZ-3: Kleinkram an der Oberfläche – Bildlaufleisten app-weit sichtbar (Befund NZ-2),
//! Schrift-Menü (✓ am Familienkopf, `Text_Font` auch für die Werksvorgabe gemerkt, Befund MN-2),
//! Zugriffstasten und Feldfokus in Kurven/Tonwertkorrektur (KV-1/KV-2) und Bildgröße (DLG-1b),
//! Meldung mit Info-Symbol (DLG-4, D25).

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_engine::Session;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::access;
use fp_ui::dialogs::levels::{self, Slider};
use fp_ui::menu::{self, ItemAction, Node};
use fp_ui::theme::{mix, Theme};
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

// ------------------------------------------------------------------ Bildlaufleisten

fn contrast(a: egui::Color32, b: egui::Color32) -> i32 {
    (a.r() as i32 - b.r() as i32).abs().max((a.g() as i32 - b.g() as i32).abs()).max((a.b() as i32 - b.b() as i32).abs())
}

#[test]
fn bildlaufleiste_in_allen_designs_sichtbar() {
    for theme in [Theme::normal(), Theme::dark(), Theme::oled()] {
        let c = &theme.colors;
        let v = theme.visuals();
        // egui: Spur = extreme_bg_color, Griff = widgets.{inactive,hovered,active}.bg_fill
        assert_eq!(v.extreme_bg_color, c.scrollbar_bg, "{:?}: Spur", theme.kind);
        assert_eq!(v.widgets.inactive.bg_fill, c.scrollbar_thumb, "{:?}: Griff ruhend", theme.kind);
        assert_eq!(v.widgets.hovered.bg_fill, mix(c.scrollbar_thumb, c.text, 0.3), "{:?}: Griff unter der Maus", theme.kind);
        assert_eq!(v.widgets.active.bg_fill, mix(c.scrollbar_thumb, c.text, 0.5), "{:?}: Griff beim Ziehen", theme.kind);
        assert!(contrast(v.widgets.inactive.bg_fill, v.extreme_bg_color) >= 40, "{:?}: Griff hebt sich von der Spur ab", theme.kind);
        // Textfelder behalten ihren eigenen Grund
        assert_eq!(v.text_edit_bg_color, Some(c.field_bg), "{:?}: Textfeldgrund", theme.kind);
    }
}

#[test]
fn bildlaufleiste_gezeichnet_mit_sichtbarem_griff() {
    // Eine beliebige lange Liste (nicht die Schriftliste) zeigt Spur und Griff in den Designfarben.
    for theme in [Theme::normal(), Theme::dark(), Theme::oled()] {
        let theme = std::sync::Arc::new(theme);
        let t2 = std::sync::Arc::clone(&theme);
        let mut h = Harness::builder().with_size(egui::vec2(300.0, 300.0)).build_ui(move |ui| {
            t2.apply(ui.ctx());
            egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                for i in 0..100 {
                    ui.label(format!("Zeile {i}"));
                }
            });
        });
        h.run_steps(3);
        let out = h.output();
        let c = &theme.colors;
        let mut track = false;
        let mut thumb = false;
        for s in out.shapes.iter() {
            if let egui::epaint::Shape::Rect(r) = &s.shape
                && r.rect.width() <= 16.0
                && r.rect.height() > 10.0
            {
                track |= r.fill == c.scrollbar_bg;
                thumb |= r.fill == c.scrollbar_thumb;
            }
        }
        assert!(track && thumb, "{:?}: Spur {track}, Griff {thumb}", theme.kind);
    }
}

#[test]
fn feldstil_fuer_egui_kaestchen() {
    let theme = std::sync::Arc::new(Theme::normal());
    let t2 = std::sync::Arc::clone(&theme);
    let seen = std::sync::Arc::new(std::sync::Mutex::new(None));
    let s2 = std::sync::Arc::clone(&seen);
    let mut h = Harness::builder().with_size(egui::vec2(200.0, 100.0)).build_ui(move |ui| {
        t2.apply(ui.ctx());
        let inner = fp_ui::theme::with_field_widgets(ui, |ui| ui.visuals().widgets.inactive.bg_fill);
        *s2.lock().unwrap() = Some((inner, ui.visuals().widgets.inactive.bg_fill));
    });
    h.run_steps(2);
    let (inner, outer) = seen.lock().unwrap().expect("gezeichnet");
    assert_eq!(inner, theme.colors.field_bg, "Kästchen im Feldstil");
    assert_eq!(outer, theme.colors.scrollbar_thumb, "außerhalb unverändert");
}

// ------------------------------------------------------------------ Schrift-Menü

fn item(label: &str, checked: bool) -> Node {
    Node::Item { label: label.into(), shortcut: None, action: ItemAction::None, enabled: true, checked: Some(checked) }
}

#[test]
fn haekchen_am_familienkopf_in_langer_liste() {
    let fam = Node::Sub { label: "Familie".into(), children: vec![item("Regular", false), item("Bold", true)], enabled: true };
    let other = Node::Sub { label: "Andere".into(), children: vec![item("Regular", false), item("Bold", false)], enabled: true };
    let mut long: Vec<Node> = (0..60).map(|i| item(&format!("F{i}"), false)).collect();
    long.push(fam.clone());
    long.push(other.clone());
    let kids = |n: &Node| match n {
        Node::Sub { children, .. } => children.clone(),
        _ => unreachable!(),
    };
    assert_eq!(menu::sub_head_checked(&long, &kids(&fam)), Some(true), "✓ am Kopf der gewählten Familie");
    assert_eq!(menu::sub_head_checked(&long, &kids(&other)), None, "kein ✓ an anderen Familien");
    // kurze Menüs (Ansicht ▸ Design …): kein Häkchen am Kopf
    let short = vec![fam.clone(), other.clone()];
    assert_eq!(menu::sub_head_checked(&short, &kids(&fam)), None);
}

fn harness_with(s: Session) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn font_nodes(st: &AppState) -> Vec<Node> {
    let menus = menu::build_main_menu(st);
    let (_, text) = menus.iter().find(|(l, _)| l == "Text").expect("Menü Text");
    text.iter()
        .find_map(|n| match n {
            Node::Sub { label, children, .. } if label == "Schrift" => Some(children.clone()),
            _ => None,
        })
        .expect("Text ▸ Schrift")
}

#[test]
fn schrift_menue_haekchen_am_kopf_der_echten_familie() {
    let fams = fp_ui::inspector::fonts::families();
    let Some(fam) = fams.iter().find(|f| f.styles.len() > 1) else { return };
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 200, "height": 100})).expect("Dokument");
    s.execute("text.font", json!({ "font": fam.name })).expect("Schrift als Vorgabe");
    let h = harness_with(s);
    let nodes = font_nodes(st(&h));
    if nodes.len() <= menu::LONG_SUBMENU {
        return; // zu wenige Schriften installiert: kein langes Menü
    }
    let heads: Vec<&str> = nodes
        .iter()
        .filter_map(|n| match n {
            Node::Sub { label, children, .. } if menu::sub_head_checked(&nodes, children) == Some(true) => Some(label.as_str()),
            _ => None,
        })
        .collect();
    assert!(!heads.is_empty() && heads.iter().all(|l| *l == fam.name), "✓ nur am Kopf von {}: {heads:?}", fam.name);
}

#[test]
fn werksvorgabe_arial_wird_gemerkt() {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 200, "height": 100})).expect("Dokument");
    s.execute("text.font", json!({ "font": "Arial" })).expect("Arial als Vorgabe");
    assert_eq!(s.prefs.values.get("Text_Font").and_then(|v| v.as_str()), Some("Arial".into()), "Text_Font ausdrücklich gespeichert");
    // nach einem „Neustart“ (Voreinstellungen aus JSON) gilt Arial weiter
    let back: fp_engine::prefs::Preferences = serde_json::from_str(&s.prefs.to_json()).expect("JSON");
    assert_eq!(back.values.get("Text_Font").and_then(|v| v.as_str()), Some("Arial".into()));
}

// ------------------------------------------------------------------ Zugriffstasten

#[test]
fn zugriffstaste_zerlegen() {
    assert_eq!(access::split_access("E&ingabetonwerte:"), Some(("E".to_string(), 'i')));
    assert_eq!(access::split_access("A && B &Cee"), Some(("A & B ".to_string(), 'C')));
    assert_eq!(access::split_access("Vorschau"), None);
    assert_eq!(access::plain("&Bild neu auflösen"), "Bild neu auflösen");
}

#[test]
fn texte_mit_zugriffstasten() {
    for (k, de, en) in [
        ("dlg-levels-input", "E&ingabetonwerte:", "&Input Levels:"),
        ("dlg-levels-output", "A&usgabetonwerte:", "&Output Levels:"),
        ("dlg-levels-preview", "&Vorschau", "&Preview"),
        ("dlg-image-size-constrain", "&Proportionen beibehalten", "&Constrain proportions"),
        ("dlg-image-size-resample", "&Bild neu auflösen", "&Resample image"),
        ("btn-help", "&Hilfe", "&Help"),
    ] {
        fp_i18n::set_language("de");
        assert_eq!(fp_i18n::t(k), de);
        fp_i18n::set_language("en");
        assert_eq!(fp_i18n::t(k), en);
    }
    fp_i18n::set_language("de");
}

fn bitmap_harness() -> H {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 400, "height": 300, "background": "#ffffff"})).expect("Dokument");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 220, 160], "fillColor": "#404040", "strokeWidth": 0})).expect("Rechteck");
    s.execute("modify.flattenSelection", json!({})).expect("Bitmap");
    harness_with(s)
}

fn alt(h: &mut H, key: Key) {
    h.event(Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::ALT });
    h.run_steps(1);
    h.event(Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::ALT });
    h.run_steps(2);
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn open(h: &mut H, cmd: &str) {
    assert!(fp_ui::dialogs::open_for_command(&mut h.state_mut().state, cmd), "{cmd}");
    h.run_steps(3);
}

#[test]
fn kurven_zugriffstasten_und_fokus() {
    let mut h = bitmap_harness();
    open(&mut h, "filter.adjustColor.curves");
    let on = st(&h).dialogs.curves.as_ref().expect("Kurven").preview.preview_on;
    alt(&mut h, Key::V);
    assert_eq!(st(&h).dialogs.curves.as_ref().expect("Kurven").preview.preview_on, !on, "Alt+V schaltet die Vorschau um");
    alt(&mut h, Key::A);
    assert!(access::field_has_focus(&h.ctx, "cv-out"), "Alt+A: Feld Ausgabe");
    alt(&mut h, Key::E);
    assert!(access::field_has_focus(&h.ctx, "cv-in"), "Alt+E: Feld Eingabe");
    // Punkt greifen: Feld Eingabe bekommt den Fokus (vorher Fokus woanders hin)
    alt(&mut h, Key::A);
    let r = fp_ui::dialogs::dialog_rect(&h.ctx, "fp-curves").expect("Dialog");
    let field = r.min + egui::vec2(1.0 + 34.0, 1.0 + fp_ui::dialogs::DIALOG_TITLE_H + 29.0);
    let p = field + egui::vec2(fp_ui::dialogs::curves::FIELD, 0.0); // Endpunkt (255, 255) oben rechts
    let p = Pos2::new(p.x - 1.0, p.y + 1.0);
    ev(&mut h, Event::PointerMoved(p));
    ev(&mut h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run_steps(1);
    assert!(access::field_has_focus(&h.ctx, "cv-in"), "Punkt gegriffen → Feld Eingabe");
    ev(&mut h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
    assert!(access::field_has_focus(&h.ctx, "cv-in"), "Fokus bleibt nach dem Loslassen");
    let d = st(&h).dialogs.curves.as_ref().expect("Kurven");
    assert_eq!(d.selected, Some(d.points().len() - 1), "Endpunkt gewählt");
}

#[test]
fn tonwert_zugriffstasten_und_griff_fokus() {
    let mut h = bitmap_harness();
    open(&mut h, "filter.adjustColor.levels");
    alt(&mut h, Key::I);
    assert!(access::field_has_focus(&h.ctx, ("lv-in", 0usize)), "Alt+I: erstes Feld der Eingabetonwerte");
    alt(&mut h, Key::U);
    assert!(access::field_has_focus(&h.ctx, ("lv-out", 0usize)), "Alt+U: erstes Feld der Ausgabetonwerte");
    let on = st(&h).dialogs.levels.as_ref().expect("Tonwert").preview.preview_on;
    alt(&mut h, Key::V);
    assert_eq!(st(&h).dialogs.levels.as_ref().expect("Tonwert").preview.preview_on, !on, "Alt+V schaltet die Vorschau um");
    // Mittelgriff (Gamma) drücken: Feld Gamma bekommt den Fokus und folgt beim Ziehen
    let track = levels::track_rect(&h.ctx, Slider::Input).expect("Eingabeschieber");
    let (x0, y) = (levels::slider_x(track, 127.5), track.center().y);
    ev(&mut h, Event::PointerMoved(egui::pos2(x0, y)));
    ev(&mut h, Event::PointerButton { pos: egui::pos2(x0, y), button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run_steps(1);
    assert!(access::field_has_focus(&h.ctx, ("lv-in", 1usize)), "Gammagriff → Feld Gamma");
    for k in 1..=10 {
        ev(&mut h, Event::PointerMoved(egui::pos2(x0 - 3.0 * k as f32, y)));
    }
    ev(&mut h, Event::PointerButton { pos: egui::pos2(x0 - 30.0, y), button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
    assert!(access::field_has_focus(&h.ctx, ("lv-in", 1usize)), "Fokus bleibt");
    assert!(st(&h).dialogs.levels.as_ref().expect("Tonwert").params.rgb.gamma > 1.2, "Ziehen wirkt trotz Feldfokus");
    // Ausgabe: rechter Griff → zweites Ausgabefeld
    let ot = levels::track_rect(&h.ctx, Slider::Output).expect("Ausgabeschieber");
    let (x1, y1) = (levels::slider_x(ot, 255.0), ot.center().y);
    ev(&mut h, Event::PointerMoved(egui::pos2(x1, y1)));
    ev(&mut h, Event::PointerButton { pos: egui::pos2(x1, y1), button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run_steps(1);
    assert!(access::field_has_focus(&h.ctx, ("lv-out", 1usize)), "Weißgriff Ausgabe → Feld Ausgabe max");
    ev(&mut h, Event::PointerButton { pos: egui::pos2(x1, y1), button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
    assert!(access::field_has_focus(&h.ctx, ("lv-out", 1usize)), "Klick ohne Bewegung: Fokus bleibt im Feld");
}

#[test]
fn bildgroesse_zugriffstasten() {
    let mut h = bitmap_harness();
    open(&mut h, "modify.canvas.imageSize");
    let d = st(&h).dialogs.image_size.as_ref().expect("Bildgröße");
    let (c0, r0) = (d.constrain, d.resample);
    alt(&mut h, Key::P);
    assert_eq!(st(&h).dialogs.image_size.as_ref().expect("Bildgröße").constrain, !c0, "Alt+P: Proportionen");
    alt(&mut h, Key::B);
    assert_eq!(st(&h).dialogs.image_size.as_ref().expect("Bildgröße").resample, !r0, "Alt+B: Bild neu auflösen");
}

// ------------------------------------------------------------------ Meldung

#[test]
fn meldung_mit_info_symbol() {
    let mut h = bitmap_harness();
    h.state_mut().state.dialogs.message = Some(("Firepixel".into(), "Kurz.".into()));
    h.run_steps(3);
    let r = fp_ui::dialogs::dialog_rect(&h.ctx, "fp-message").expect("Meldung offen");
    // Text steht rechts neben dem 32-px-Symbol
    let out = h.output();
    let text_left = out
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::epaint::Shape::Text(t) if t.galley.job.text == "Kurz." => Some(t.pos.x),
            _ => None,
        })
        .next()
        .expect("Text gezeichnet");
    assert!(text_left >= r.left() + 16.0 + 32.0, "Text rechts vom Symbol ({text_left} / {:?})", r);
}

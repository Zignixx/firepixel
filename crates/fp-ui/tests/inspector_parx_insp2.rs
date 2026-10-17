//! PAR-X-INSP-2: Mischmodus-Namen je Liste wie im Original (Objekt-Liste „Sprenkeln“ … „Radieren“,
//! Pinsel-Liste „Auflösen“ … „Löschen“) und ganz lesbar (PAR 17/07b), Farbrad im Verlaufs-Popup
//! (18/04b), Winkel ohne „-0“ (18/08b).

use egui::accesskit::Role;
use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{By, Queryable};
use egui_kittest::Harness;
use fp_ui::app_state::UiSettings;
use fp_ui::inspector::{color_picker, fields, obj_quads, popups, texts};
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness(setup: impl FnOnce(&mut fp_engine::Session)) -> H {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 300, "background": "#ffffff"})).expect("Dokument");
    setup(&mut s);
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1800.0, 1130.0)).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn rect(s: &mut fp_engine::Session) {
    s.execute("draw.rectangle", json!({"rect": [50, 50, 150, 120], "fillColor": "#808080", "strokeWidth": 0})).expect("Rechteck");
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

fn text_w(h: &H, s: &str) -> f32 {
    let font = fp_ui::theme::Theme::current(&h.ctx).ui_font();
    h.ctx.fonts_mut(|f| f.layout_no_wrap(s.to_string(), font, egui::Color32::WHITE).size().x)
}

/// Liste des Inspektors laut `inspector-path-rect.orig.md` Nr. 07.
const OBJECT_LIST: &str = "Normal, Durchschnitt, Sprenkeln, -----, Abdunkeln, Multiplizieren, Farbig nachbelichten, Umgek. farb. nachb., Weich nachbelichten, Linear nachbelichten, -----, Aufhellen, Negativ multiplizieren, Farbig abwedeln, Umgek. farb. abw., Weich abwedeln, Linear abwedeln, -----, Ineinanderkopieren, Weiches Licht, Unscharfes Licht, Hartes Licht, Strahlendes Licht, Lineares Licht, Lichtpunkt, Hart mischen, -----, Differenz, Ausschluss, Negation, -----, Farbton, Sättigung, Farbe, Helligkeit, Rot, Grün, Blau, -----, Spiegeln, Glühen, Abkühlen, Erwärmen, -----, Additiv, Subtraktiv, Subtrahieren, Interpolation, Stempel, XOR, -----, Umkehren, Tönung, -----, Radieren";

fn norm(v: &[String]) -> Vec<&str> {
    v.iter().map(|s| if s.starts_with("---") { "-----" } else { s.as_str() }).collect()
}

/// Objekt-Liste (Inspektor, Ebenen, Farbfüllung) und Pinsel-Liste haben getrennte Namen wie im
/// Original; Englisch hat für beide die englischen Namen.
#[test]
fn blend_names_per_list_like_original() {
    let want: Vec<&str> = OBJECT_LIST.split(", ").collect();
    let obj = texts::blend_labels_obj();
    assert_eq!(norm(&obj), want);
    let layers: Vec<String> = obj_quads::blend_names().iter().map(|s| s.to_string()).collect();
    assert_eq!(norm(&layers), want, "Ebenen-Bedienfeld wie der Inspektor");
    let brush = texts::blend_labels();
    for (i, name) in [(2, "Auflösen"), (18, "Überlagern"), (34, "Luminanz"), (52, "Farbton"), (54, "Löschen")] {
        assert_eq!(brush[i], name, "Pinsel-Liste bleibt");
    }
    fp_i18n::with_language("en", || {
        let obj = texts::blend_labels_obj();
        for (i, name) in [(2, "Dissolve"), (12, "Screen"), (18, "Overlay"), (34, "Luminosity"), (52, "Tint"), (54, "Erase")] {
            assert_eq!(obj[i], name);
        }
    });
}

/// Mischmodus-Liste des Inspektors: Namen des Originals, „Negativ multiplizieren“ und die übrigen
/// langen Einträge ganz lesbar (Eintrag mindestens Text + Innenabstand breit).
#[test]
fn object_blend_list_fully_readable() {
    let mut h = harness(rect);
    let combos: Vec<egui::Rect> = h.get_all(By::new().role(Role::ComboBox)).filter(|n| n.value().as_deref() == Some("Normal")).map(|n| n.rect()).collect();
    assert!(!combos.is_empty(), "Mischmodus-Feld");
    for c in combos {
        click(&mut h, c.center());
        assert!(h.query_by_label("Sprenkeln").is_some(), "Objekt-Liste mit „Sprenkeln“");
        for name in ["Negativ multiplizieren", "Ineinanderkopieren", "Farbig nachbelichten", "Linear nachbelichten"] {
            let r = h.get_by_label(name).rect();
            let need = text_w(&h, name) + 10.0;
            assert!(r.width() >= need - 0.5, "„{name}“ ganz lesbar: Eintrag {} px, Text braucht {need} px", r.width());
        }
        ev(&mut h, Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
        h.run_steps(2);
    }
}

/// Zahlenfelder zeigen nie „-0“ (Winkel im Muster-Popup, PAR 18/08b).
#[test]
fn numbers_never_show_minus_zero() {
    assert_eq!(fields::fmt_num(-0.0, 0), "0");
    assert_eq!(fields::fmt_num(-0.3, 0), "0");
    assert_eq!(fields::fmt_num(-0.001, 2), "0");
    assert_eq!(fields::fmt_num(-1.0, 0), "-1");
    assert_eq!(fields::fmt_num(-0.5, 1), "-0,5");
    if popups::pattern_files().is_empty() {
        return;
    }
    // Muster-Popup direkt nach dem Umstellen: Winkel „0“
    let mut h = harness(|s| {
        rect(s);
        s.execute("pi.fill", json!({"category": "pattern"})).expect("Muster");
    });
    let pid = obj_quads::fill_popup_ids(false).1;
    popups::set_anchored_open(&h.ctx, pid, true);
    h.run_steps(3);
    let vals: Vec<String> = h.get_all(By::new().role(Role::TextInput)).filter_map(|n| n.value()).collect();
    assert!(!vals.iter().any(|v| v == "-0"), "kein „-0“: {vals:?}");
    assert!(vals.iter().any(|v| v == "0"), "Winkel 0: {vals:?}");
}

/// Verlaufs-Popup: Farbrad-Knopf links neben den Farbstopps öffnet den Farbwähler des ersten
/// Farbstopps gleich mit dem Farbkreis; Esc schließt erst den Farbkreis.
#[test]
fn gradient_popup_color_wheel() {
    let mut h = harness(|s| {
        rect(s);
        s.execute("pi.fill", json!({"category": "gradient"})).expect("Verlauf");
    });
    let pid = obj_quads::fill_popup_ids(false).0;
    popups::set_anchored_open(&h.ctx, pid, true);
    h.run_steps(3);
    let tip = fp_i18n::t("pi-color-system");
    let wheel: Vec<egui::Rect> = h.query_all(By::new().role(Role::Button).label(tip.as_str())).map(|n| n.rect()).collect();
    assert_eq!(wheel.len(), 1, "ein Farbrad-Knopf im Verlaufs-Popup");
    let lin = h.get_all(By::new().role(Role::ComboBox)).find(|n| n.value().as_deref() == Some("Linear")).expect("Verlauf:").rect();
    assert!(wheel[0].center().x < lin.left() && wheel[0].top() > lin.bottom() + 40.0, "links unten neben den Farbstopps: {:?}", wheel[0]);
    assert!(!color_picker::any_open(&h.ctx));
    click(&mut h, wheel[0].center());
    h.run_steps(3);
    assert!(color_picker::any_open(&h.ctx), "Farbwähler offen");
    assert!(color_picker::wheel_open(&h.ctx), "mit Farbkreis");
    assert!(popups::anchored_open(&h.ctx, pid), "Verlaufs-Popup bleibt offen");
    ev(&mut h, Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
    assert!(!color_picker::wheel_open(&h.ctx), "Esc schließt den Farbkreis");
}

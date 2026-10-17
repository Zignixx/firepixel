//! F24 (Tweak-Liste 17.10., A3-9): Dialog „Neuer Stil“ mit den Vorgaben und dem Aufbau des
//! Originals (TW-92/93) und die Griff-Tooltips automatischer Formen (TW-96). Der Augen-Zug (TW-94)
//! steht bei den übrigen Augen-Tests in `f18`.

use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness_with(s: fp_engine::Session) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1600.0, 1000.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn texts(shape: &egui::epaint::Shape, out: &mut Vec<(String, egui::Rect)>) {
    match shape {
        egui::epaint::Shape::Text(t) => out.push((t.galley.job.text.clone(), egui::Rect::from_min_size(t.pos, t.galley.size()))),
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
        _ => {}
    }
}

fn all_texts(h: &H) -> Vec<(String, egui::Rect)> {
    let mut all = Vec::new();
    for cs in &h.output().shapes {
        texts(&cs.shape, &mut all);
    }
    all
}

/// Rechteck (bzw. Textblock) ausgewählt.
fn session(text: bool) -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    if text {
        s.execute("draw.text", json!({"text": "Hallo", "anchor": [20, 40], "size": 24})).expect("text");
    } else {
        s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80]})).expect("rect");
    }
    s.execute("select.all", json!({})).expect("sel");
    s
}

fn checks(f: &fp_ui::dialogs::form::FormDialog) -> Vec<(String, bool)> {
    f.fields.iter().filter(|x| x.key.starts_with('_') && x.key != "_caption").map(|x| (x.key.clone(), x.value.as_bool().unwrap_or(false))).collect()
}

/// TW-92: Name „Stil“ vorbelegt; bei einem Rechteck die vier Text-Kästchen aus, die übrigen an
/// (Lauf 10, `panel-library-styles/11`; re 39 §3.3: Maske B 0x08 ohne Text, 0x1F mit Text).
#[test]
fn tw92_new_style_defaults_by_selection() {
    fp_i18n::with_language("de", || {
        let h = harness_with(session(false));
        let f = fp_ui::dialogs::misc::new_style_form(st(&h));
        assert_eq!(f.fields[0].value, json!("Stil"), "Name vorbelegt");
        let c = checks(&f);
        let on: Vec<&str> = c.iter().filter(|x| x.1).map(|x| x.0.as_str()).collect();
        assert_eq!(on, ["_fillType", "_fillColor", "_effect", "_strokeType", "_strokeColor"], "{c:?}");
        // leer gemacht: wie bisher „Stil n“
        let mut f2 = f.clone();
        f2.field_mut("name").expect("Name").value = json!("");
        assert_eq!(fp_ui::dialogs::misc::style_params(&f2)["name"], json!("Stil 1"));
        assert_eq!(fp_ui::dialogs::misc::style_params(&f)["name"], json!("Stil"));
        let h = harness_with(session(true));
        let f = fp_ui::dialogs::misc::new_style_form(st(&h));
        assert!(checks(&f).iter().all(|x| x.1), "Text: alle an {:?}", checks(&f));
        // re 39 §3.3: „Füllungstyp“ schaltet zwei Bits (0x44)
        assert_eq!(fp_ui::dialogs::misc::style_params(&f)["flags"].as_u64().map(|v| v & 0x44), Some(0x44));
        let mut f3 = f.clone();
        f3.field_mut("_fillType").expect("Füllungstyp").value = json!(false);
        assert_eq!(fp_ui::dialogs::misc::style_params(&f3)["flags"].as_u64().map(|v| v & 0x44), Some(0));
    });
    fp_i18n::with_language("en", || {
        let h = harness_with(session(false));
        let f = fp_ui::dialogs::misc::new_style_form(st(&h));
        assert_eq!(f.fields[0].value, json!("Style"));
    });
}

/// TW-93: Aufbau wie Vorlage 19123: Name oben rechts der Miniatur, „Eigenschaften:“ links neben
/// zwei Kästchen-Spalten (Füllung/Effekt links, Strich/Text rechts), OK und Abbrechen unten rechts.
#[test]
fn tw93_new_style_two_columns() {
    fp_i18n::with_language("de", || {
        let mut h = harness_with(session(false));
        h.state_mut().state.dialogs.open_new_style();
        h.run_steps(6);
        let tx = all_texts(&h);
        let at = |s: &str| tx.iter().filter(|x| x.0 == s).map(|x| x.1).next_back().unwrap_or_else(|| panic!("{s} fehlt: {:?}", tx.iter().map(|x| &x.0).collect::<Vec<_>>()));
        let (fill, fillc, fx) = (at("Füllungstyp"), at("Füllfarbe"), at("Effekt"));
        let (stroke, strokec, font, other) = (at("Strichtyp"), at("Strichfarbe"), at("Textschrift"), at("Text - Sonstiges"));
        let props = at("Eigenschaften:");
        let name = at("Name:");
        assert!((fill.center().y - stroke.center().y).abs() < 1.0, "Füllungstyp und Strichtyp in einer Zeile");
        assert!((fillc.center().y - strokec.center().y).abs() < 1.0);
        assert!((fx.center().y - font.center().y).abs() < 1.0, "Effekt und Textschrift in einer Zeile");
        assert!(stroke.left() > fill.right(), "zweite Spalte rechts");
        assert!((fill.left() - fx.left()).abs() < 1.0 && (stroke.left() - other.left()).abs() < 1.0, "Spalten bündig");
        assert!(other.top() > font.bottom(), "Text - Sonstiges unten rechts");
        assert!(props.right() < fill.left() && (props.center().y - fill.center().y).abs() < 3.0, "Eigenschaften: links neben der ersten Zeile");
        assert!(name.bottom() < fill.top() && (name.right() - props.right()).abs() < 2.0, "Name: über Eigenschaften:, rechtsbündig");
        // Dialoggröße etwa wie im Original (432 × 246 mit Rahmen)
        let r = fp_ui::dialogs::dialog_rect(&h.ctx, &fp_ui::dialogs::last_dialog_id(&h.ctx).expect("Dialog")).expect("Rechteck");
        assert!((380.0..500.0).contains(&r.width()) && (200.0..300.0).contains(&r.height()), "{r:?}");
        // Miniatur links der Beschriftungen
        assert!(name.left() > r.left() + 80.0, "Platz für die Miniatur: {name:?} in {r:?}");
    });
}

/// TW-96: Griff-Tooltip wie im Original: graues Kästchen (#D6D6D6) ohne Rand mit schwarzer
/// Schrift, linke Kante am Griff, darunter (`tool-autoshapes/04`: Griff 125,100 → Kästchen 125,114).
#[test]
fn tw96_handle_tooltip_left_below_handle_grey() {
    let ctx = egui::Context::default();
    let mut got = None;
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        let ctx = ui.ctx();
        let theme = fp_ui::theme::Theme::current(ctx);
        let painter = ctx.layer_painter(egui::LayerId::background());
        got = Some(fp_ui::canvas::handle_tip_at(&painter, &theme, egui::pos2(125.0, 100.0), "Klicken, um zwischen den Ecken umzuschalten"));
    });
    out.textures_delta.clear();
    let r = got.expect("Kästchen");
    assert_eq!(r.left(), 125.0, "linke Kante am Griff: {r:?}");
    assert_eq!(r.top(), 114.0, "14 px unter dem Griff: {r:?}");
    assert!((13.0..18.0).contains(&r.height()), "flach wie im Original (15 px): {r:?}");
    assert_eq!(fp_ui::canvas::HANDLE_TIP_BG, egui::Color32::from_rgb(214, 214, 214));
}

/// M10 (re 43 §3): Die Kästchen richten sich nach dem obersten gewählten Element; ohne Auswahl
/// sind alle 9 gesetzt; die Miniatur zeigt ein Quadrat bzw. „ABC“; Zugriffstasten wie im Original.
#[test]
fn t11_new_style_top_selected_and_no_selection() {
    fp_i18n::with_language("de", || {
        let mut s = fp_engine::Session::default();
        s.execute("file.new", json!({"width": 600, "height": 400, "background": "#ffffff"})).expect("doc");
        s.execute("draw.rectangle", json!({"rect": [50, 50, 150, 150]})).expect("rect");
        s.execute("draw.text", json!({"text": "Hallo", "anchor": [200, 40], "size": 24})).expect("text");
        s.execute("select.all", json!({})).expect("sel");
        let mut h = harness_with(s);
        let on = |h: &H| checks(&fp_ui::dialogs::misc::new_style_form(st(h))).iter().filter(|x| x.1).count();
        // R + T, T oben: alle 9, Miniatur „ABC“
        assert_eq!(on(&h), 9);
        assert!(fp_ui::dialogs::misc::style_preview(st(&h)).text);
        // R nach vorn: nur die 5, Miniatur Quadrat
        let sess = &mut h.state_mut().state.session;
        let rect_id = {
            let d = sess.active().expect("doc");
            let ids = d.selection.elements.clone();
            ids.into_iter().find(|id| !matches!(fp_engine::docops::element_on_page(&d.doc, d.page, *id).map(|e| e.kind.clone()), Some(fp_model::ElementKind::Text(_)))).expect("R")
        };
        sess.execute("select.set", json!({"ids": [rect_id.0]})).expect("R wählen");
        sess.execute("modify.arrange.bringToFront", json!({})).expect("nach vorn");
        sess.execute("select.all", json!({})).expect("sel");
        assert_eq!(on(&h), 5);
        let pv = fp_ui::dialogs::misc::style_preview(st(&h));
        assert!(!pv.text && pv.fill.is_some(), "{pv:?}");
        // ohne Auswahl: alle 9, „ABC“ aus den Vorgaben, Befehl aktiv
        h.state_mut().state.session.execute("select.none", json!({})).expect("none");
        let f = fp_ui::dialogs::misc::new_style_form(st(&h));
        assert_eq!(checks(&f).iter().filter(|x| x.1).count(), 9);
        assert_eq!(f.fields[0].value, json!("Stil"));
        assert!(fp_ui::dialogs::misc::style_preview(st(&h)).text);
        let out = h.state_mut().state.session.execute("pi.style.new", fp_ui::dialogs::misc::style_params(&f));
        assert!(out.is_ok(), "{out:?}");
        // Zugriffstasten (re 43 §3)
        let labels: Vec<String> = f.fields.iter().filter(|x| x.key != "_caption").map(|x| x.label.clone()).collect();
        assert_eq!(
            labels,
            ["&Name:", "Füllungst&yp", "Füll&farbe", "&Effekt", "&Strichtyp", "St&richfarbe", "Te&xtschrift", "&Textgröße", "Textst&il", "Text - Sonsti&ges"]
        );
    });
}

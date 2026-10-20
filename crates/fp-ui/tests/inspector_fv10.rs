//! Funktionsvergleich mit dem Original (FV-10, Befunde PM-2 im Abschnitt IBD): Polygon-Winkel und
//! „Autom.“ bedienbar (W19), Texturname der Vorgabe statt „Ohne“ (W21), B/H/X/Y ohne Auswahl weiß
//! und leer (W22), feste Auswahlrahmen-Werte mit ↔/↕ und 0/0 (W24), Verlaufsvorschau im
//! Füllfarbfeld des Verlauf-Werkzeugs (W26), kein „Bearbeiten…“ neben der Exportliste (W29),
//! Vorschaubild an der Lage im Dokument und keins bei Mehrfachauswahl (O2), Horizontalskala mit
//! „%“ (O8), leere Stil-Liste bei Mehrfachauswahl (O16). Realistische Eingaben wie egui-winit unter
//! Windows (je Ereignis ein Bild, Tippen als `Event::Text`, Tastendruck und Loslassen getrennt).

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::Harness;
use fp_engine::tools::MarqueeStyle;
use fp_engine::ToolId;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::FirepixelApp;
use serde_json::json;
use std::sync::Arc;

type H = Harness<'static, FirepixelApp>;

const W: f32 = 1800.0;
const HH: f32 = 1130.0;

fn harness(setup: impl FnOnce(&mut fp_engine::Session)) -> H {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 300, "background": "#ffffff"})).expect("Dokument");
    setup(&mut s);
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().renderer(crate::gpu_common::renderer()).with_size(egui::vec2(W, HH)).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn rect(s: &mut fp_engine::Session, r: [i32; 4], c: &str) -> u64 {
    let v = s.execute("draw.rectangle", json!({"rect": r, "fillColor": c, "strokeWidth": 0})).expect("Rechteck");
    v.value.get("id").and_then(|x| x.as_u64()).expect("id")
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn key(h: &mut H, k: Key, m: Modifiers) {
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: m });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: m });
}

fn text(h: &mut H, t: &str) {
    for c in t.chars() {
        ev(h, Event::Text(c.to_string()));
    }
}

fn press(h: &mut H, p: Pos2, down: bool) {
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: down, modifiers: Modifiers::NONE });
}

fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    press(h, p, true);
    press(h, p, false);
    h.run_steps(2);
}

fn tool(h: &mut H, t: ToolId) {
    let _ = h.state_mut().state.session.set_tool(t);
    h.run_steps(3);
}

fn undo_count(h: &H) -> usize {
    st(h).session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn doc(h: &H) -> Arc<fp_model::Document> {
    Arc::clone(&st(h).session.active().expect("Dokument").doc)
}

fn element(h: &H, id: u64) -> Arc<fp_model::Element> {
    let d = st(h).session.active().expect("Dokument");
    fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id)).expect("Element")
}

/// Eingabefelder im Inspektor (unterer Fensterteil): Rechteck, Text, bedienbar.
fn inputs(h: &H) -> Vec<(egui::Rect, String, bool)> {
    let mut v: Vec<(egui::Rect, String, bool)> = h
        .get_all(By::new().role(egui::accesskit::Role::TextInput))
        .map(|n| (n.rect(), n.value().unwrap_or_default(), !n.accesskit_node().is_disabled()))
        .filter(|(r, _, _)| r.top() > HH - 160.0)
        .collect();
    v.sort_by(|a, b| a.0.top().total_cmp(&b.0.top()).then(a.0.left().total_cmp(&b.0.left())));
    v
}

fn input_with(h: &H, value: &str) -> (egui::Rect, bool) {
    let all = inputs(h);
    let m: Vec<_> = all.iter().filter(|(_, v, _)| v == value).collect();
    assert_eq!(m.len(), 1, "genau ein Feld mit „{value}“: {all:?}");
    (m[0].0, m[0].2)
}

/// Kombifelder im Inspektor: Rechteck, Wert, bedienbar.
fn combos(h: &H) -> Vec<(egui::Rect, String, bool)> {
    let mut v: Vec<(egui::Rect, String, bool)> = h
        .get_all(By::new().role(egui::accesskit::Role::ComboBox))
        .map(|n| (n.rect(), n.value().unwrap_or_default(), !n.accesskit_node().is_disabled()))
        .filter(|(r, _, _)| r.top() > HH - 160.0)
        .collect();
    v.sort_by(|a, b| a.0.left().total_cmp(&b.0.left()).then(a.0.top().total_cmp(&b.0.top())));
    v
}

fn combos_with(h: &H, value: &str) -> Vec<(egui::Rect, bool)> {
    combos(h).into_iter().filter(|(_, v, _)| v == value).map(|(r, _, e)| (r, e)).collect()
}

/// Feld anklicken, Inhalt ersetzen (Ende, Umschalt+Pos1, Tippen) und mit Eingabe übernehmen.
fn type_into(h: &mut H, field: egui::Rect, s: &str) {
    click(h, field.center());
    key(h, Key::End, Modifiers::NONE);
    key(h, Key::Home, Modifiers::SHIFT);
    text(h, s);
    key(h, Key::Enter, Modifiers::NONE);
    h.run_steps(2);
}

/// Kombifeld öffnen und Eintrag wählen.
fn choose(h: &mut H, combo: egui::Rect, item: &str) {
    click(h, combo.center());
    let r = h.get_by_label(item).rect();
    click(h, r.center());
}

fn checkbox(h: &H, label: &str) -> (egui::Rect, bool, bool) {
    let n = h.get_by_label(label);
    let checked = matches!(n.accesskit_node().toggled(), Some(egui::accesskit::Toggled::True));
    (n.rect(), checked, !n.accesskit_node().is_disabled())
}

/// Gezeichnete Beschriftungen im Inspektor (Text und Rechteck) über das Layout-Protokoll.
fn drawn_labels(h: &mut H) -> Vec<(String, egui::Rect)> {
    fp_ui::inspector::fields::set_audit(&h.ctx, true);
    h.run_steps(1);
    let log = fp_ui::inspector::fields::take_audit(&h.ctx);
    fp_ui::inspector::fields::set_audit(&h.ctx, false);
    log.into_iter().filter(|e| e.kind == "label" || e.kind == "name").map(|e| (e.text, e.drawn)).collect()
}

// ------------------------------------------------------------------ W19 Polygon

/// Form = Polygon: Winkel 38 und „Autom.“ ☑ sind wie im Original bedienbar (nicht grau). Ein
/// getippter Winkel gilt und hebt „Autom.“ auf, „Autom.“ wieder an stellt die Automatik her; bei
/// „Stern“ genauso. §4b: 20× tippen → Endwert.
#[test]
fn w19_polygon_angle_and_auto_enabled() {
    let mut h = harness(|_| {});
    tool(&mut h, ToolId::Polygon);
    assert!(!st(&h).session.tools.options.polygon_star, "Werksvorgabe Form = Polygon");
    let auto = fp_i18n::t("pi-poly-auto");
    let (field, en) = input_with(&h, "38");
    assert!(en, "Winkel bedienbar bei Form = Polygon");
    let (cb, checked, cen) = checkbox(&h, &auto);
    assert!(checked && cen, "Autom. ☑ und bedienbar");
    assert!(st(&h).session.tools.options.polygon_spikiness.is_none());
    type_into(&mut h, field, "50");
    assert_eq!(st(&h).session.tools.options.polygon_spikiness, Some(0.5));
    assert!(!checkbox(&h, &auto).1, "getippter Winkel hebt Autom. auf");
    assert_eq!(input_with(&h, "50").0, field);
    click(&mut h, egui::pos2(cb.left() + 6.0, cb.center().y));
    assert!(st(&h).session.tools.options.polygon_spikiness.is_none(), "Autom. an = Automatik");
    assert!(checkbox(&h, &auto).1);
    // Stern: ebenfalls bedienbar
    h.state_mut().state.session.tools.options.polygon_star = true;
    h.run_steps(2);
    assert!(input_with(&h, "38").1);
    assert!(checkbox(&h, &auto).2);
    // §4b: viele Eingaben → nur der Endwert, kein Verlaufsschritt (Werkzeugvorgabe)
    let n0 = undo_count(&h);
    for k in 0..20 {
        let cur = inputs(&h).into_iter().find(|(r, _, _)| *r == field).expect("Winkelfeld").0;
        type_into(&mut h, cur, &format!("{}", 20 + k));
    }
    assert_eq!(st(&h).session.tools.options.polygon_spikiness, Some(0.39));
    assert_eq!(undo_count(&h), n0, "Werkzeugoptionen ohne Verlaufsschritt");
}

// ------------------------------------------------------------------ W21 Textur

fn vein() -> String {
    fp_ui::inspector::obj_quads::texture_label(fp_ui::inspector::obj_quads::DEFAULT_TEXTURE)
}

/// Ohne Textur zeigt die Texturliste wie im Original den Namen der Vorgabe („Ader“, Datei „Vein“)
/// mit Menge 0 – bei Vektorwerkzeugen (Füllung und Strich), Füllwerkzeug, Verlauf und am gewählten
/// Rechteck. Einen Eintrag „Ohne“ gibt es nicht.
#[test]
fn w21_texture_shows_default_name_with_zero() {
    assert_eq!(vein(), "Ader", "deutscher Name der Vorgabetextur");
    fp_i18n::with_language("en", || assert_eq!(vein(), "Vein"));
    let mut h = harness(|_| {});
    for (t, n) in [(ToolId::Rectangle, 2), (ToolId::Ellipse, 2), (ToolId::Polygon, 2), (ToolId::Pen, 2), (ToolId::PaintBucket, 1), (ToolId::Gradient, 1)] {
        tool(&mut h, t);
        let c = combos_with(&h, &vein());
        assert_eq!(c.len(), n, "{t:?}: Texturlisten mit „Ader“: {:?}", combos(&h));
    }
    // Liste ohne „Ohne“
    tool(&mut h, ToolId::Rectangle);
    let (c, _) = combos_with(&h, &vein())[0];
    click(&mut h, c.center());
    let none = fp_i18n::t("pi-none");
    let items: Vec<String> = h.query_all(By::new().role(egui::accesskit::Role::Button)).filter_map(|n| n.accesskit_node().label()).collect();
    assert!(!items.contains(&none), "kein Eintrag „Ohne“ in der Texturliste");
    key(&mut h, Key::Escape, Modifiers::NONE);
    h.run_steps(2);
    // gewähltes Rechteck: „Ader“ mit 0, Wählen einer anderen Textur ist ein Schritt, Rückgängig stellt her
    let mut id = 0;
    let mut h = harness(|s| id = rect(s, [50, 50, 150, 120], "#ff0000"));
    let c = combos_with(&h, &vein());
    assert_eq!(c.len(), 2, "Füll- und Strichtextur zeigen „Ader“: {:?}", combos(&h));
    assert!(c[0].1, "Fülltextur bedienbar");
    let d0 = doc(&h);
    // erster Eintrag der Liste (oben, ohne Bildlauf erreichbar), sofern es nicht die Vorgabe ist
    let (items, _) = fp_ui::inspector::obj_quads::texture_items("");
    if let Some(other) = items.first().map(|(n, _)| n.clone()).filter(|n| n.as_str() != fp_ui::inspector::obj_quads::DEFAULT_TEXTURE) {
        let n0 = undo_count(&h);
        choose(&mut h, c[0].0, &fp_ui::inspector::obj_quads::texture_label(&other));
        let attrs = fp_ui::inspector::obj_quads::shown_attrs(st(&h)).expect("Attribute");
        assert_eq!(attrs.fill_texture.as_ref().map(|t| t.name.clone()).as_deref(), Some(other.as_str()));
        assert_eq!(undo_count(&h), n0 + 1, "ein Verlaufsschritt");
        h.state_mut().state.session.undo().expect("undo");
        h.run_steps(2);
        assert_eq!(format!("{:?}", doc(&h)), format!("{d0:?}"), "Rückgängig stellt her");
        assert_eq!(combos_with(&h, &vein()).len(), 2);
    }
    let _ = element(&h, id);
}

// ------------------------------------------------------------------ W22 B/H/X/Y

/// B/H/X/Y-Felder im linken Inspektorteil.
fn box_fields(h: &H) -> Vec<(egui::Rect, String, bool)> {
    // die unteren vier Felder links (darüber steht bei Auswahl das Namensfeld)
    let v: Vec<_> = inputs(h).into_iter().filter(|(r, _, _)| r.right() < 200.0).collect();
    v[v.len().saturating_sub(4)..].to_vec()
}

/// Rechteck, Ellipse, Auto-Form, Auswahlrahmen, Lasso, Zuschneiden, Exportbereich ohne Auswahl:
/// B/H/X/Y weiß, leer und bedienbar wie im Original; eine Eingabe ohne Ziel ändert nichts (kein
/// Verlaufsschritt). Mit Auswahl stehen dort die Werte.
#[test]
fn w22_box_fields_white_and_empty_without_selection() {
    let mut h = harness(|_| {});
    for t in [ToolId::Rectangle, ToolId::Ellipse, ToolId::AutoShape, ToolId::Marquee, ToolId::Lasso, ToolId::Crop, ToolId::ExportArea] {
        tool(&mut h, t);
        let b = box_fields(&h);
        assert_eq!(b.len(), 4, "{t:?}: B/H/X/Y: {:?}", inputs(&h));
        assert!(b.iter().all(|(_, v, en)| v.is_empty() && *en), "{t:?}: weiß, leer, bedienbar: {b:?}");
    }
    tool(&mut h, ToolId::Rectangle);
    let d0 = doc(&h);
    let n0 = undo_count(&h);
    let w = box_fields(&h)[0].0;
    type_into(&mut h, w, "50");
    assert_eq!(undo_count(&h), n0, "ohne Auswahl kein Schritt");
    assert_eq!(format!("{:?}", doc(&h)), format!("{d0:?}"));
    assert!(box_fields(&h).iter().all(|(_, v, _)| v.is_empty()), "Feld wieder leer");
    // mit Auswahl: Werte
    let h = harness(|s| {
        rect(s, [50, 50, 150, 120], "#ff0000");
    });
    let v: Vec<String> = box_fields(&h).into_iter().map(|(_, v, _)| v).collect();
    assert_eq!(v, ["100", "50", "70", "50"]);
}

// ------------------------------------------------------------------ W24 Auswahlrahmen

/// Auswahlrahmen: feste Breite/Höhe mit den Symbolen ↔/↕ (keine Buchstaben B/H), bei „Normal“
/// 0/0 grau; „Festes Verhältnis“ 1/2, „Feste Größe“ 64/64 aus den Werksvorgaben (im Original
/// gemessen), bedienbar; getippte Werte bleiben beim Hin- und Herwechseln erhalten.
#[test]
fn w24_marquee_fixed_values_symbols_and_zero() {
    let mut h = harness(|_| {});
    tool(&mut h, ToolId::Marquee);
    let labels = drawn_labels(&mut h);
    let texts: Vec<&str> = labels.iter().map(|(t, _)| t.as_str()).collect();
    assert!(texts.contains(&"\u{2194}") && texts.contains(&"\u{2195}"), "Symbole ↔/↕: {texts:?}");
    let q3: Vec<(egui::Rect, String, bool)> = inputs(&h).into_iter().filter(|(r, _, _)| r.left() > 200.0 && r.left() < 400.0).collect();
    assert_eq!(q3.iter().map(|(_, v, e)| (v.as_str(), *e)).collect::<Vec<_>>(), [("0", false), ("0", false)], "Normal: 0/0 grau: {q3:?}");
    let (wr, hr) = (q3[0].0, q3[1].0);
    let field = |h: &H, r: egui::Rect| inputs(h).into_iter().find(|(x, _, _)| *x == r).map(|(_, v, e)| (v, e)).expect("Feld");
    let style = combos_with(&h, &fp_i18n::t("pi-marquee-normal"))[0].0;
    choose(&mut h, style, &fp_i18n::t("pi-marquee-fixed-ratio"));
    assert_eq!(st(&h).session.tools.options.marquee_style, MarqueeStyle::FixedRatio(1.0, 2.0));
    assert_eq!((field(&h, wr), field(&h, hr)), (("1".to_string(), true), ("2".to_string(), true)));
    choose(&mut h, style, &fp_i18n::t("pi-marquee-fixed-size"));
    assert_eq!(st(&h).session.tools.options.marquee_style, MarqueeStyle::FixedSize(64.0, 64.0));
    assert_eq!((field(&h, wr), field(&h, hr)), (("64".to_string(), true), ("64".to_string(), true)));
    type_into(&mut h, wr, "100");
    assert_eq!(st(&h).session.tools.options.marquee_style, MarqueeStyle::FixedSize(100.0, 64.0));
    // hin und zurück: Werte bleiben
    choose(&mut h, style, &fp_i18n::t("pi-marquee-fixed-ratio"));
    assert_eq!(st(&h).session.tools.options.marquee_style, MarqueeStyle::FixedRatio(1.0, 2.0));
    choose(&mut h, style, &fp_i18n::t("pi-marquee-fixed-size"));
    assert_eq!(st(&h).session.tools.options.marquee_style, MarqueeStyle::FixedSize(100.0, 64.0));
    choose(&mut h, style, &fp_i18n::t("pi-marquee-normal"));
    assert_eq!(st(&h).session.tools.options.marquee_style, MarqueeStyle::Normal);
    assert_eq!((field(&h, wr), field(&h, hr)), (("0".to_string(), false), ("0".to_string(), false)));
}

// ------------------------------------------------------------------ W26 Verlauf

/// Verlauf-Werkzeug: das Füllfarbfeld zeigt den Verlauf, den das Werkzeug malt – seit FV-16 die
/// eigenen Farben des Werkzeugs (Werksvorgabe Schwarz → Weiß, im Original gemessen), unabhängig
/// von der Füllfarbe; Füllwerkzeug und Rechteck bleiben einfarbig.
#[test]
fn w26_gradient_tool_swatch_shows_gradient() {
    let mut h = harness(|_| {});
    tool(&mut h, ToolId::Gradient);
    assert_eq!(fp_ui::inspector::obj_quads::tool_swatch_gradient(&h.ctx), Some(vec![0xff00_0000, 0xffff_ffff]));
    h.state_mut().state.session.tools.fill_color = fp_color::Argb(0xff00_00ff);
    h.run_steps(2);
    assert_eq!(fp_ui::inspector::obj_quads::tool_swatch_gradient(&h.ctx), Some(vec![0xff00_0000, 0xffff_ffff]));
    for t in [ToolId::PaintBucket, ToolId::Rectangle] {
        tool(&mut h, t);
        assert_eq!(fp_ui::inspector::obj_quads::tool_swatch_gradient(&h.ctx), None, "{t:?} einfarbig");
    }
}

// ------------------------------------------------------------------ W29 Dokument-Inspektor

/// Dokument-Inspektor (Zeiger ohne Auswahl): neben der Exportformat-Liste kein Knopf „Bearbeiten…“.
#[test]
fn w29_document_inspector_without_edit_button() {
    let mut h = harness(|_| {});
    tool(&mut h, ToolId::Pointer);
    let edit = fp_i18n::t("pi-edit");
    let n = h.query_all(By::new().role(egui::accesskit::Role::Button)).filter(|n| n.rect().top() > HH - 160.0 && n.accesskit_node().label().as_deref() == Some(edit.as_str())).count();
    assert_eq!(n, 0, "kein „Bearbeiten…“ im Dokument-Inspektor");
    assert!(!combos(&h).is_empty(), "Exportliste vorhanden");
}

// ------------------------------------------------------------------ O2 Vorschaubild

/// Vorschaubild: das Objekt an seiner Lage im verkleinerten Dokument (transparent drumherum), das
/// Bild hat das Seitenformat; bei Mehrfachauswahl kein Vorschaubild.
#[test]
fn o2_thumbnail_object_at_its_place() {
    let (mut a, mut b) = (0, 0);
    let mut h = harness(|s| {
        a = rect(s, [50, 50, 150, 120], "#ff0000");
        b = rect(s, [300, 150, 450, 280], "#0000ff");
        s.execute("select.set", json!({"ids": [a]})).expect("Auswahl");
    });
    h.run_steps(3);
    let r = fp_ui::inspector::selection_thumb_rect(&h.ctx).expect("Vorschaubild bei einem Objekt");
    assert!((r.width() / r.height() - 500.0 / 300.0).abs() < 0.05, "Seitenformat 5:3: {r:?}");
    // Bildinhalt: R1 (50..150, 50..120) bei 100 px Breite → 10..30, 10..24
    let d = doc(&h);
    let e = element(&h, a);
    let img = fp_ui::panels::thumbs::element_on_page_image(&st(&h).renderer, &d, 0, &e, 0, 100).expect("Bild");
    assert_eq!((img.width(), img.height()), (100, 60));
    let red = img.pixel(20, 17);
    assert!(red.r > 200 && red.g < 40 && red.a > 200, "R1 an seiner Lage: {red:?}");
    for (x, y) in [(5, 5), (60, 40), (90, 55), (35, 17)] {
        assert_eq!(img.pixel(x, y).a, 0, "außerhalb von R1 transparent ({x},{y})");
    }
    // Mehrfachauswahl: kein Bild
    h.state_mut().state.session.execute("select.set", json!({"ids": [a, b]})).expect("Auswahl");
    h.run_steps(3);
    assert_eq!(fp_ui::inspector::selection_thumb_rect(&h.ctx), None, "kein Vorschaubild bei Mehrfachauswahl");
    // wieder ein Objekt: Bild
    h.state_mut().state.session.execute("select.set", json!({"ids": [b]})).expect("Auswahl");
    h.run_steps(3);
    assert!(fp_ui::inspector::selection_thumb_rect(&h.ctx).is_some());
}

// ------------------------------------------------------------------ O8 Horizontalskala

fn hscale(h: &H, id: u64) -> f32 {
    let e = element(h, id);
    let fp_model::ElementKind::Text(t) = &e.kind else { panic!("Text") };
    t.resolved_runs().first().and_then(|(a, _)| a.horizontal_scale).unwrap_or(1.0)
}

/// Text-Inspektor (Objekt und Werkzeug): die Horizontalskala zeigt „100%“; getippt „120“ bzw.
/// „80%“ gilt (ein Schritt je Übernahme), das Feld zeigt den Wert wieder mit „%“; Rückgängig stellt
/// her. §4b: 10× tippen → Endwert.
#[test]
fn o8_horizontal_scale_with_percent() {
    let mut id = 0;
    let mut h = harness(|s| {
        let v = s.execute("draw.text", json!({"text": "Hallo", "anchor": [30, 60], "size": 24})).expect("Text");
        id = v.value.get("id").and_then(|x| x.as_u64()).unwrap_or(0);
    });
    h.run_steps(2);
    let (f, en) = input_with(&h, "100%");
    assert!(en);
    let n0 = undo_count(&h);
    type_into(&mut h, f, "120");
    assert!((hscale(&h, id) - 1.2).abs() < 1e-4, "120 % übernommen");
    assert_eq!(input_with(&h, "120%").0, f);
    assert_eq!(undo_count(&h), n0 + 1);
    type_into(&mut h, f, "80%");
    assert!((hscale(&h, id) - 0.8).abs() < 1e-4, "„80%“ übernommen");
    assert_eq!(input_with(&h, "80%").0, f);
    for k in 0..10 {
        type_into(&mut h, f, &format!("{}", 100 + k));
    }
    assert!((hscale(&h, id) - 1.09).abs() < 1e-4);
    while undo_count(&h) > n0 {
        h.state_mut().state.session.undo().expect("undo");
    }
    h.run_steps(2);
    assert!((hscale(&h, id) - 1.0).abs() < 1e-4, "Rückgängig stellt 100 % her");
    input_with(&h, "100%");
    // Textwerkzeug ohne Auswahl
    h.state_mut().state.session.execute("select.none", json!({})).expect("nichts");
    tool(&mut h, ToolId::Text);
    // Text tool without a selection shows the tool defaults. Values set on text objects selected
    // with the pointer are stored as defaults (TW-138), but activating the text tool resets the
    // paragraph defaults (R5, RE spec 56 section 3): the horizontal scale is 100 % again.
    assert!(input_with(&h, "100%").1, "tool: reset on activating the text tool");
    assert!(
        (st(&h).session.prefs.get_real("Text_HorizScale", 0.0) - 1.0).abs() < 1e-9,
        "Text_HorizScale reset"
    );
}

// ------------------------------------------------------------------ O16 Stil-Liste

/// Stil-Liste: ein Rechteck ohne Stil zeigt „Kein Stil“, zwei Rechtecke (beide ohne Stil) eine
/// leere Liste wie im Original (bedienbar).
#[test]
fn o16_style_list_empty_for_multiple_objects() {
    let (mut a, mut b) = (0, 0);
    let mut h = harness(|s| {
        a = rect(s, [50, 50, 150, 120], "#ff0000");
        b = rect(s, [300, 150, 450, 280], "#0000ff");
        s.execute("select.set", json!({"ids": [a]})).expect("Auswahl");
    });
    h.run_steps(3);
    let none = fp_i18n::t("pi-style-none");
    assert_eq!(combos_with(&h, &none).len(), 1, "ein Objekt: „Kein Stil“: {:?}", combos(&h));
    h.state_mut().state.session.execute("select.set", json!({"ids": [a, b]})).expect("Auswahl");
    h.run_steps(3);
    assert!(combos_with(&h, &none).is_empty(), "zwei Objekte: kein „Kein Stil“: {:?}", combos(&h));
    let right: Vec<(egui::Rect, String, bool)> = combos(&h).into_iter().filter(|(r, _, _)| r.left() > W * 0.4).collect();
    assert!(right.iter().any(|(_, v, e)| v.is_empty() && *e), "leere, bedienbare Stil-Liste: {right:?}");
}

// ------------------------------------------------------------------ Bilder (Nachweis)

/// Nachher-Bilder des Inspektors (offscreen gerendert) nach `target-fv10\fv10_bilder\` für die
/// Vorher/Nachher-Montagen in `docs\screenshots\paritaet\fv10_*.png`. Aufruf:
/// `cargo test -p fp-ui --test inspector_fv10 -- --ignored bilder`
#[test]
#[ignore]
fn bilder_nachher() {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target-fv10/fv10_bilder");
    let _ = std::fs::create_dir_all(&dir);
    let save = |h: &mut H, name: &str| {
        h.run_steps(8);
        match h.render() {
            Ok(img) => {
                let _ = img.save(dir.join(format!("{name}.png")));
            }
            Err(e) => eprintln!("kein Grafikadapter: {e}"),
        }
    };
    let mut h = harness(|_| {});
    for (t, n) in [(ToolId::Polygon, "w19_polygon"), (ToolId::Rectangle, "w21_w22_rechteck"), (ToolId::Marquee, "w24_rahmen"), (ToolId::Gradient, "w26_verlauf"), (ToolId::Pointer, "w29_dokument"), (ToolId::Text, "o8_textwerkzeug")] {
        tool(&mut h, t);
        save(&mut h, n);
    }
    let (mut a, mut b, mut t) = (0, 0, 0);
    let mut h = harness(|s| {
        a = rect(s, [50, 50, 150, 120], "#ff0000");
        b = rect(s, [300, 150, 450, 280], "#0000ff");
        let v = s.execute("draw.text", json!({"text": "Hallo", "anchor": [200, 60], "size": 24})).expect("Text");
        t = v.value.get("id").and_then(|x| x.as_u64()).unwrap_or(0);
        s.execute("select.set", json!({"ids": [a]})).expect("Auswahl");
    });
    save(&mut h, "o2_r1");
    h.state_mut().state.session.execute("select.set", json!({"ids": [a, b]})).expect("Auswahl");
    save(&mut h, "o2_o16_mehrfach");
    h.state_mut().state.session.execute("select.set", json!({"ids": [t]})).expect("Auswahl");
    save(&mut h, "o8_text");
}

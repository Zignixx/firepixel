//! FIX-C (Befunde von CT-1a, Dialoge):
//! - D43 Kurven: gepunktetes Raster und Kurvenlinie werden sichtbar gezeichnet
//! - D42 Helligkeit/Kontrast: Pfeiltasten bewegen den fokussierten Schieber
//!
//! Realistische Eingaben: Tasten wie egui-winit (Drücken/Loslassen), Klicks über mehrere Bilder.

use egui::accesskit::Role;
use egui::{Color32, Event, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::FirepixelApp;
use serde_json::{json, Value};

type H = Harness<'static, FirepixelApp>;

fn harness_with(s: fp_engine::Session) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h
}

/// Dokument mit der Bitmap B1, ausgewählt (Filterdialoge).
fn bitmap_session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [40, 40, 140, 120], "fillColor": "#3366cc", "name": "B1"})).expect("rect");
    s.execute("modify.flattenSelection", json!({})).expect("reduzieren");
    s
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn key(h: &mut H, k: Key) {
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: Modifiers::NONE });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

#[allow(dead_code)]
fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

fn open(h: &mut H, cmd: &str) {
    h.state_mut().state.run(cmd, json!({}));
    h.run_steps(4);
    assert!(st(h).dialogs.any_open(), "{cmd}: Dialog offen");
}

#[allow(dead_code)]
fn dialog_rect(h: &H) -> Rect {
    fp_ui::dialogs::dialog_rect(&h.ctx, &fp_ui::dialogs::last_dialog_id(&h.ctx).expect("Dialog")).expect("Rechteck")
}

fn focused(h: &H) -> Option<(Role, String)> {
    h.ctx.memory(|m| m.focused())?;
    h.get_all(By::new()).find(|n| n.accesskit_node().is_focused()).map(|n| {
        let node = n.accesskit_node();
        (node.role(), node.label().map(|s| s.to_string()).or_else(|| node.value().map(|s| s.to_string())).unwrap_or_default())
    })
}

fn form_value(h: &H, key: &str) -> Value {
    st(h).dialogs.form.as_ref().expect("Formular").get(key)
}

/// Einzelpixel (1 × 1) einer Farbe in den Netzen des letzten Bildes innerhalb von `r`.
fn mesh_pixels(h: &H, r: Rect, c: Color32) -> usize {
    let mut n = 0;
    for cs in &h.output().shapes {
        let egui::epaint::Shape::Mesh(m) = &cs.shape else { continue };
        for q in m.vertices.chunks(4) {
            if q.len() == 4 && q[0].color == c && r.contains(q[0].pos) {
                let b = Rect::from_points(&q.iter().map(|v| v.pos).collect::<Vec<_>>());
                if (b.width() - 1.0).abs() < 0.01 && (b.height() - 1.0).abs() < 0.01 {
                    n += 1;
                }
            }
        }
    }
    n
}

/// D43: Raster (3 + 3 Linien, jedes zweite Pixel) und Kurve (ein Pixel je Spalte) werden als
/// ungeglättete Einzelpixel gezeichnet – vorher als 1 × 1-Rechtecke, die der Tessellator zu
/// kaum sichtbaren Linienstücken macht.
#[test]
fn d43_curves_grid_and_line_visible() {
    let mut h = harness_with(bitmap_session());
    open(&mut h, "filter.adjustColor.curves");
    let r = fp_ui::dialogs::dialog_rect(&h.ctx, "fp-curves").expect("Kurven");
    let w = fp_ui::dialogs::curves::FIELD as usize;
    assert_eq!(mesh_pixels(&h, r, Color32::from_gray(0x80)), 3 * 2 * w / 2, "gepunktetes Raster 4 × 4");
    assert_eq!(mesh_pixels(&h, r, Color32::BLACK), w, "Kurve: ein Pixel je Spalte");
    key(&mut h, Key::Escape);
}

/// D42: Pfeiltasten verschieben den fokussierten Schieber; Wert und Feld folgen.
#[test]
fn d42_arrow_keys_move_focused_slider() {
    let mut h = harness_with(bitmap_session());
    open(&mut h, "filter.adjustColor.brightnessContrast");
    assert_eq!(focused(&h).map(|f| f.0), Some(Role::Slider), "Startfokus Schieber");
    for _ in 0..5 {
        key(&mut h, Key::ArrowRight);
    }
    assert_eq!(form_value(&h, "brightness").as_f64(), Some(5.0), "→ ×5");
    key(&mut h, Key::ArrowLeft);
    assert_eq!(form_value(&h, "brightness").as_f64(), Some(4.0), "←");
    assert_eq!(focused(&h).map(|f| f.0), Some(Role::Slider), "Fokus bleibt am Schieber");
    // per Tab erreichter zweiter Schieber
    key(&mut h, Key::Tab);
    key(&mut h, Key::Tab);
    assert_eq!(focused(&h).map(|f| f.0), Some(Role::Slider), "Kontrast-Schieber");
    key(&mut h, Key::ArrowLeft);
    key(&mut h, Key::ArrowLeft);
    assert_eq!(form_value(&h, "contrast").as_f64(), Some(-2.0), "← ×2 am Kontrast");
    // Klick auf den Schieber, dann Pfeil
    let sl: Vec<Rect> = h.get_all(By::new().role(Role::Slider)).map(|n| n.rect()).collect();
    click(&mut h, sl[0].center());
    let v = form_value(&h, "brightness").as_f64().expect("Wert");
    key(&mut h, Key::ArrowRight);
    assert_eq!(form_value(&h, "brightness").as_f64(), Some(v + 1.0), "→ nach Klick");
    key(&mut h, Key::Escape);
}

/// D21 Leinwandfarbe: Startfokus „Weiß“, ↓ → Transparent, Tab → OK (das eingerückte Farbfeld ist
/// kein Tab-Stopp), Umschalt+Tab zurück zur Gruppe.
#[test]
fn d21_canvas_color_no_hidden_tab_stop() {
    let mut h = harness_with(bitmap_session());
    open(&mut h, "modify.canvas.canvasColor");
    assert_eq!(focused(&h), Some((Role::RadioButton, fp_i18n::t("dlg-canvas-white"))), "Startfokus Weiß");
    key(&mut h, Key::ArrowDown);
    assert_eq!(focused(&h), Some((Role::RadioButton, fp_i18n::t("dlg-canvas-transparent"))), "↓ Transparent");
    key(&mut h, Key::Tab);
    assert_eq!(focused(&h), Some((Role::Button, fp_i18n::t("btn-ok"))), "Tab → OK");
    key(&mut h, Key::Tab);
    assert_eq!(focused(&h), Some((Role::Button, fp_i18n::t("btn-cancel"))), "Tab → Abbrechen");
    ev(&mut h, Event::ModifiersChanged(Modifiers::SHIFT));
    ev(&mut h, Event::Key { key: Key::Tab, physical_key: Some(Key::Tab), pressed: true, repeat: false, modifiers: Modifiers::SHIFT });
    ev(&mut h, Event::Key { key: Key::Tab, physical_key: Some(Key::Tab), pressed: false, repeat: false, modifiers: Modifiers::SHIFT });
    ev(&mut h, Event::ModifiersChanged(Modifiers::NONE));
    h.run_steps(2);
    assert_eq!(focused(&h), Some((Role::Button, fp_i18n::t("btn-ok"))), "Umschalt+Tab → OK");
    key(&mut h, Key::Escape);
}
/// Rechte Kante eines gezeichneten Texts im letzten Bild.
fn text_right(h: &H, text: &str) -> Option<f32> {
    h.output().shapes.iter().find_map(|cs| match &cs.shape {
        egui::epaint::Shape::Text(ts) if ts.galley.text() == text => Some(ts.pos.x + ts.galley.size().x),
        _ => None,
    })
}

/// D16 Voreinstellungen: außen 480–520 px hoch (vorher ≈ 585), „Sprache:“ rechtsbündig vor dem Feld
/// wie „Design:“; in Englisch ebenso.
#[test]
fn d16_preferences_height_and_language_label() {
    for lang in ["de", "en"] {
        fp_i18n::set_language(lang);
        let mut h = harness_with(bitmap_session());
        open(&mut h, "edit.preferences");
        let r = fp_ui::dialogs::dialog_rect(&h.ctx, "fp-prefs").expect("Voreinstellungen");
        assert!((480.0..=520.0).contains(&r.height()), "{lang}: Höhe {}", r.height());
        let lang_r = text_right(&h, &fp_i18n::t("lang-label")).expect("Sprache:");
        let theme_r = text_right(&h, &fp_i18n::t("dlg-pref-theme")).expect("Design:");
        assert!((lang_r - theme_r).abs() <= 1.5, "{lang}: Sprache rechts {lang_r}, Design rechts {theme_r}");
        key(&mut h, Key::Escape);
    }
    fp_i18n::set_language("de");
}
/// Linke Kante eines gezeichneten Texts im letzten Bild (Rechteck).
fn text_rect(h: &H, text: &str) -> Option<Rect> {
    h.output().shapes.iter().find_map(|cs| match &cs.shape {
        egui::epaint::Shape::Text(ts) if ts.galley.text() == text => Some(Rect::from_min_size(ts.pos, ts.galley.size())),
        _ => None,
    })
}

/// KV-3: Live-Effekt Kurven heißt in der Filterliste „Kurven...“ (wie der Menüeintrag) und hat das
/// ⓘ direkt nach dem Kästchen; ein Klick darauf öffnet den Kurvendialog mit den Effektwerten.
#[test]
fn kv3_live_curves_label_and_info_button() {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 180, 140], "fillColor": "#808080", "strokeWidth": 0})).expect("rect");
    s.execute("element.addEffect", json!({"effect": "curves"})).expect("Kurven-Effekt");
    let mut h = harness_with(s);
    h.run_steps(3);
    let label = fp_ui::menu::clean_label(&fp_i18n::t("cmd-filter-adjustColor-curves")).0;
    assert_eq!(label, "Kurven...");
    let lr = text_rect(&h, &label).expect("Eintrag „Kurven...“ in der Filterliste");
    assert!(text_rect(&h, "Gradationskurven").is_none(), "alter Name");
    // ⓘ links vor dem Namen (13 px breit, 5 px Abstand)
    click(&mut h, egui::pos2(lr.left() - 5.0 - 6.5, lr.center().y));
    h.run_steps(3);
    assert!(st(&h).dialogs.curves.is_some(), "ⓘ öffnet den Kurvendialog");
    key(&mut h, Key::Escape);
}
/// E2 CSS-Sprites: Präfix tippen und sofort OK klicken übernimmt den Wert (Beispielzeile live);
/// ein ungültiger Präfix wie „1 x“ führt beim Export zur Meldung „Einige der CSS-Selektornamen …“.
#[test]
fn e2_sprite_prefix_typed_then_ok_is_kept() {
    use fp_export::doc_export::FilesStyle;
    use fp_ui::dialogs::export::{style_index, ExportDialog};
    use fp_ui::dialogs::export_options::OptionsDialog;
    let mut h = harness_with(bitmap_session());
    let mut d = ExportDialog::new(st(&h)).expect("Exportdialog");
    d.set_style(style_index(FilesStyle::CssSprites));
    d.open_options();
    assert!(matches!(d.options, Some(OptionsDialog::Sprites(_))));
    h.state_mut().state.dialogs.export = Some(d);
    h.run_steps(4);
    let r = fp_ui::dialogs::dialog_rect(&h.ctx, "fp-export-sprite-options").expect("Sprite-Optionen");
    // Feld in der Zeile der Beschriftung „Präfix:“ (der Exportdialog liegt darunter)
    let lab = text_rect(&h, &fp_i18n::t("dlg-expopt-prefix")).expect("Präfix:");
    let field = h.get_all(By::new().role(Role::TextInput)).map(|n| n.rect()).find(|x| r.contains(x.center()) && (x.center().y - lab.center().y).abs() < 6.0 && x.left() > lab.right()).expect("Präfixfeld");
    click(&mut h, field.center());
    for c in ["1", " ", "x"] {
        ev(&mut h, Event::Text(c.into()));
    }
    let prefix = |h: &H| match &st(h).dialogs.export.as_ref().expect("Export").options {
        Some(OptionsDialog::Sprites(o)) => (o.prefix.clone(), o.example()),
        _ => Default::default(),
    };
    assert_eq!(prefix(&h).0, "1 x", "Wert sofort im Dialog (Beispiel {})", prefix(&h).1);
    let ok = h.get_all(By::new().role(Role::Button).label(fp_i18n::t("btn-ok").as_str())).map(|n| n.rect()).find(|x| r.contains(x.center())).expect("OK");
    click(&mut h, ok.center());
    h.run_steps(3);
    let d = st(&h).dialogs.export.as_ref().expect("Exportdialog bleibt");
    assert!(d.options.is_none(), "Optionen geschlossen");
    assert_eq!(d.sprite.prefix, "1 x", "getippter Präfix übernommen");
    h.state_mut().state.dialogs.export = None;
}
/// E3: Der Exportdialog (nicht nur der Engine-Befehl) wendet die HTML-Einrichtung an:
/// „Dateiname in Kleinbuchstaben“ und leere Zellen (Farbe #123456, geschütztes Leerzeichen).
#[test]
fn e3_export_dialog_uses_html_setup() {
    use fp_export::doc_export::FilesStyle;
    use fp_ui::dialogs::export::{begin_export, style_index, ExportDialog, ExportStep};
    use fp_ui::dialogs::html_setup::{self, HtmlSetupDialog};
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 240, "height": 160})).expect("file.new");
    s.execute("draw.rectangle", json!({"rect": [10, 10, 100, 100], "fillColor": "#ff0000"})).expect("rect");
    s.execute("edit.insert.slice", json!({})).expect("slice");
    s.execute("select.set", json!({"ids": []})).ok();
    let mut st = AppState::new(s, UiSettings::default(), None);
    let mut hs = HtmlSetupDialog::new(&st).expect("HTML-Einrichtung");
    hs.lowercase = true;
    hs.shims = 1; // Einzelne Tabelle ohne Platzhalter
    hs.use_canvas = false;
    hs.cell_color = "#123456".into();
    hs.contents = 2; // Geschütztes Leerzeichen
    hs.include_unsliced = false; // Bereiche ohne Segmente nicht einschließen ⇒ leere Zellen
    html_setup::apply(&mut st, &hs);
    let dir = std::env::temp_dir().join(format!("fp-fixc-e3-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("Ordner");
    let mut d = ExportDialog::new(&st).expect("Exportdialog");
    d.set_style(style_index(FilesStyle::HtmlAndImages));
    d.folder = dir.display().to_string();
    d.file_name = "MyPage.htm".into();
    match begin_export(&mut st, &mut d) {
        ExportStep::Done(_) => {}
        o => panic!("{o:?}"),
    }
    let mut files: Vec<String> = std::fs::read_dir(&dir).expect("Ordner").flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
    files.sort();
    assert!(files.iter().any(|f| f == "mypage.htm"), "HTML klein: {files:?}");
    assert!(files.iter().all(|f| *f == f.to_lowercase()), "alle Namen klein: {files:?}");
    assert!(!files.iter().any(|f| f == "spacer.gif"), "ohne Abstandhalter: {files:?}");
    let html = std::fs::read_to_string(dir.join("mypage.htm")).expect("HTML");
    assert!(html.contains("bgcolor=\"#123456\">&nbsp;</td>"), "leere Zellen: {html}");
    let _ = std::fs::remove_dir_all(&dir);
}
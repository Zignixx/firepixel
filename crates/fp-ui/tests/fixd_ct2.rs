//! FIX-D: Befunde aus dem Klicktest CT-2 (`docs\funktionstest\ct2_2026-10-13.md`) in der Oberfläche
//! mit echten Zeigerereignissen.

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_engine::tools::ToolId;
use fp_engine::Session;
use fp_geom::IRect;
use fp_model::{ElementId, ElementKind};
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn app_with_bitmap(rgba: [u8; 4]) -> (FirepixelApp, ElementId) {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 600, "height": 400, "background": "#ffffff"})).expect("Dokument");
    let img = fp_raster::RgbaImage::filled(IRect::new(50, 50, 550, 350), fp_color::Rgba8::new(rgba[0], rgba[1], rgba[2], rgba[3])).expect("Bild");
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    let v = s.execute("edit.paste", json!({})).expect("Bitmap");
    let id = ElementId(v.value["ids"][0].as_u64().expect("id"));
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.sync_render = true;
    (a, id)
}

fn harness(a: FirepixelApp) -> H {
    Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(16).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a)
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn image(h: &H, id: ElementId) -> fp_raster::RgbaImage {
    let d = st(h).session.active().expect("Dokument");
    match &fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("Element").kind {
        ElementKind::Bitmap(b) => fp_engine::docops::bitmap_image(b).expect("Bild"),
        _ => panic!("keine Bitmap"),
    }
}

/// Dokumentpunkt → Bildschirm (Seitenlage aus dem letzten Bild).
fn screen(h: &H, x: f32, y: f32) -> Pos2 {
    let s = st(h);
    let d = s.session.active().expect("Dokument");
    let pr = s.canvases.get(&d.id).and_then(|c| c.page_screen).expect("Seitenlage");
    let r = fp_render::Renderer::page_rect(&d.doc, d.page).expect("Seite");
    let (w, ht) = ((r.right - r.left) as f32, (r.bottom - r.top) as f32);
    egui::pos2(pr.left() + (x - r.left as f32) / w * pr.width(), pr.top() + (y - r.top as f32) / ht * pr.height())
}

/// Zieht mit echten Zeigerereignissen über viele Bilder (je Bild ≈ 4 px).
fn mouse_drag(h: &mut H, a: (f32, f32), b: (f32, f32)) {
    let p0 = screen(h, a.0, a.1);
    h.event(Event::PointerMoved(p0));
    h.run_steps(1);
    h.event(Event::PointerButton { pos: p0, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run_steps(1);
    let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
    let k = (len / 25.0).ceil().max(1.0) as usize;
    let mut last = p0;
    for i in 1..=k {
        let t = i as f32 / k as f32;
        last = screen(h, a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
        h.event(Event::PointerMoved(last));
        h.run_steps(1);
    }
    h.event(Event::PointerButton { pos: last, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

/// CT-2 W8: Farbe ersetzen (Größe 13, Toleranz 32, Stärke 200, Grün → Blau) auf grüner Bitmap –
/// beim Ziehen mit der Maus wird der ganze Strich blau, nicht nur der erste Stempel.
#[test]
fn replace_color_mouse_drag_replaces_whole_stroke() {
    use fp_engine::prefs::PrefValue::{Bool, Int, Str};
    let (a, id) = app_with_bitmap([0, 255, 0, 255]);
    let mut h = harness(a);
    {
        let s = &mut h.state_mut().state.session;
        for (k, v) in [
            ("ReplaceColorToolSrcColor", Str("#00FF00".into())),
            ("ReplaceColorToolDestColor", Str("#0000FF".into())),
            ("ReplaceColorToolTolerance", Int(32)),
            ("ReplaceColorToolStrength", Int(200)),
            ("ReplaceColorToolDiameter", Int(13)),
            ("ReplaceColorToolColorize", Bool(false)),
            ("ReplaceColorToolMode", Bool(false)),
        ] {
            s.prefs.set(k, v);
        }
        s.set_tool(ToolId::ReplaceColor).expect("Werkzeug");
    }
    h.run_steps(3);
    mouse_drag(&mut h, (100.0, 200.0), (400.0, 200.0));
    let img = image(&h, id);
    for x in [100, 160, 250, 330, 398] {
        let p = img.get(x, 200).expect("Pixel");
        assert!(p[1] < 10 && p[2] > 240, "x {x}: {p:?} statt blau");
    }
}

fn small_harness(a: FirepixelApp) -> H {
    Harness::builder().with_size(egui::vec2(900.0, 650.0)).with_pixels_per_point(1.0).with_max_steps(16).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a)
}

fn click_at(h: &mut H, p: Pos2) {
    h.event(Event::PointerMoved(p));
    h.run_steps(1);
    h.event(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run_steps(1);
    h.event(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(3);
}


/// CT-2: Das Untermenü des Strichart-Menüs öffnet im 900 × 650-Fenster neben dem Hauptmenü (rechts,
/// ohne Platz dort links), nicht darüber; Haupt- und Untermenü sind so breit wie ihre Einträge,
/// auch nach mehrmaligem Öffnen und Wählen.
#[test]
fn stroke_category_submenu_beside_main_menu() {
    use egui_kittest::kittest::Queryable;
    let (a, _) = app_with_bitmap([200, 200, 200, 255]);
    let mut h = small_harness(a);
    h.state_mut().state.session.execute("select.none", json!({})).expect("nichts");
    h.state_mut().state.session.set_tool(ToolId::Brush).expect("Pinsel");
    h.run_steps(4);
    let combos: Vec<(String, egui::Rect)> = h
        .query_all(egui_kittest::kittest::By::new().role(egui::accesskit::Role::ComboBox))
        .map(|n| (n.value().unwrap_or_default().to_string(), n.rect()))
        .collect();
    let cat = combos.iter().find(|(v, _)| v == "Präzise abger.").map(|(_, r)| *r).expect("Strichart-Knopf"); // Werksvorgabe seit FIX-G5
    click_at(&mut h, cat.center());
    let popups = |h: &H| -> Vec<egui::Rect> {
        h.ctx.memory(|m| {
            m.areas()
                .visible_layer_ids()
                .into_iter()
                .filter(|l| l.order == egui::Order::Foreground)
                .filter_map(|l| m.area_rect(l.id))
                .filter(|r| r.width() > 40.0 && r.height() > 40.0)
                .collect()
        })
    };
    let main = popups(&h);
    assert_eq!(main.len(), 1, "ein Menü offen: {main:?}");
    let m = main[0];
    assert!(m.width() < 300.0, "Hauptmenü so breit wie seine Einträge, nicht {:.0} px", m.width());
    // über die Einträge fahren, bis das Untermenü „Einfach“ aufgeht
    let mut sub = None;
    let mut y = m.top() + 4.0;
    while y < m.top() + 90.0 && sub.is_none() {
        h.event(Event::PointerMoved(egui::pos2(m.left() + 40.0, y)));
        h.run_steps(4);
        sub = popups(&h).into_iter().find(|r| *r != m);
        y += 4.0;
    }
    let sub = sub.expect("Untermenü geht auf");
    eprintln!("Hauptmenü {m:?} Untermenü {sub:?}");
    // Art wählen und erneut öffnen (CT-2: mehrmals geöffnet, Artwechsel)
    for round in 0..3 {
        let first = egui::pos2(sub.center().x, sub.top() + 14.0);
        h.event(Event::PointerMoved(egui::pos2(sub.left() + 10.0, y)));
        h.run_steps(2);
        click_at(&mut h, first);
        h.run_steps(4);
        let cats: Vec<(String, egui::Rect)> = h
            .query_all(egui_kittest::kittest::By::new().role(egui::accesskit::Role::ComboBox))
            .map(|n| (n.value().unwrap_or_default().to_string(), n.rect()))
            .collect();
        let cat2 = cats.iter().find(|(_, r)| (r.center().y - cat.center().y).abs() < 3.0 && (r.left() - cat.left()).abs() < 3.0).map(|(_, r)| *r).expect("Strichart-Knopf");
        click_at(&mut h, cat2.center());
        let again = popups(&h);
        assert!(again.len() == 1 && (again[0].width() - m.width()).abs() < 1.0, "Runde {round}: {again:?}");
        h.event(Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
        h.run_steps(3);
    }
    assert!(sub.left() >= m.right() - 4.0 || sub.right() <= m.left() + 4.0, "Untermenü neben dem Hauptmenü: {m:?} / {sub:?}");
    let room_right = m.right() + sub.width() <= 900.0;
    assert_eq!(sub.left() >= m.right() - 4.0, room_right, "rechts daneben, wenn Platz ist, sonst links: {m:?} / {sub:?}");
    assert!(sub.right() <= 900.0 && sub.bottom() <= 650.0, "im Fenster: {sub:?}");
}

fn open_popups(h: &H) -> Vec<egui::Rect> {
    h.ctx.memory(|m| {
        m.areas()
            .visible_layer_ids()
            .into_iter()
            .filter(|l| l.order == egui::Order::Foreground)
            .filter_map(|l| m.area_rect(l.id))
            .filter(|r| r.width() > 40.0 && r.height() > 40.0)
            .collect()
    })
}

fn combo_rect(h: &H, value: &str) -> Option<egui::Rect> {
    use egui_kittest::kittest::Queryable;
    h.query_all(egui_kittest::kittest::By::new().role(egui::accesskit::Role::ComboBox)).find(|n| n.value().as_deref() == Some(value)).map(|n| n.rect())
}

/// CT-2 / NZ-1 W21: Die Texturliste des Rechteck-Werkzeugs ist so breit wie der längste Name
/// („Linie waagerecht 1…4“ ganz lesbar), in Deutsch und Englisch, im 900 × 650-Fenster.
#[test]
fn texture_list_as_wide_as_longest_name() {
    for lang in ["de", "en"] {
        fp_i18n::set_language(lang);
        let (a, _) = app_with_bitmap([200, 200, 200, 255]);
        let mut h = small_harness(a);
        h.state_mut().state.session.execute("select.none", json!({})).expect("nichts");
        h.state_mut().state.session.set_tool(ToolId::Rectangle).expect("Rechteck");
        h.run_steps(4);
        let label = fp_ui::inspector::obj_quads::texture_label(fp_ui::inspector::obj_quads::DEFAULT_TEXTURE);
        let r = combo_rect(&h, &label).unwrap_or_else(|| panic!("Texturfeld „{label}“"));
        click_at(&mut h, r.center());
        let p = open_popups(&h);
        assert_eq!(p.len(), 1, "{lang}: Liste offen {p:?}");
        let (items, labels, _) = fp_ui::inspector::obj_quads::texture_list_labels("");
        let font = fp_ui::theme::Theme::current(&h.ctx).ui_font();
        let longest = labels.iter().map(|l| h.ctx.fonts_mut(|f| f.layout_no_wrap(l.clone(), font.clone(), egui::Color32::WHITE).size().x)).fold(0.0f32, f32::max);
        assert!(items.len() > 1);
        assert!(p[0].width() >= longest + 10.0, "{lang}: Liste {:.0} px, längster Name {:.0} px", p[0].width(), longest);
    }
    fp_i18n::set_language("de");
}

/// CT-2 KI-2: Esc während „Bild analysieren“ bricht nur die KI-Rechnung ab; die vorhandene
/// Pixelauswahl und der Bitmapmodus bleiben (wie der Knopf „Abbrechen“), kein Verlaufsschritt.
#[test]
fn escape_in_ai_progress_keeps_pixel_selection() {
    use fp_engine::commands::ai::fp_ai::{self, models::ModelId, Stage};
    let (a, id) = app_with_bitmap([90, 140, 200, 255]);
    let mut h = harness(a);
    {
        let s = &mut h.state_mut().state.session;
        s.execute("select.set", json!({"ids": [id.0]})).expect("Bitmap");
        s.execute("select.pixels", json!({"shape": "rect", "rect": [219, 29, 371, 141]})).expect("Pixelauswahl");
        s.set_tool(ToolId::AiObject).expect("KI-Objektauswahl");
    }
    h.run_steps(3);
    let before = st(&h).session.active().map(|d| (d.selection.has_pixels(), d.selection.pixels.clone(), d.history.undo_count())).expect("Dokument");
    assert!(before.0, "Pixelauswahl vorhanden");
    fp_ui::dialogs::models::start_demo(&mut h.state_mut().state, "select.aiObject", ModelId::Sam2Tiny, |pr| {
        pr.set_stage(Stage::Analyze);
        pr.set(0.45);
        while !pr.cancel.is_cancelled() {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        Err(fp_ai::AiError::Cancelled)
    });
    let t = std::time::Instant::now();
    while t.elapsed() < std::time::Duration::from_millis(400) {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
    assert!(st(&h).dialogs.ai.busy(), "Rechnung läuft");
    h.event(Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
    h.step();
    h.event(Event::Key { key: egui::Key::Escape, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::NONE });
    let t = std::time::Instant::now();
    while st(&h).dialogs.ai.busy() && t.elapsed() < std::time::Duration::from_secs(3) {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
    assert!(!st(&h).dialogs.ai.busy(), "Esc bricht ab");
    let after = st(&h).session.active().map(|d| (d.selection.has_pixels(), d.selection.pixels.clone(), d.history.undo_count())).expect("Dokument");
    assert!(after.0, "Pixelauswahl bleibt");
    assert!(after.1 == before.1, "Pixelauswahl unverändert");
    assert_eq!(after.2, before.2, "kein Verlaufsschritt");
    assert_eq!(st(&h).session.tools.active, ToolId::AiObject, "Werkzeug bleibt");
}

fn fill_of(h: &H, id: ElementId) -> fp_color::Argb {
    let d = st(h).session.active().expect("Dokument");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("Element");
    fp_engine::docops::path_attrs_of(&e).map(|a| a.fill_color).expect("Pfad")
}

fn stroke_of(h: &H, id: ElementId) -> (fp_color::Argb, bool) {
    let d = st(h).session.active().expect("Dokument");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("Element");
    fp_engine::docops::path_attrs_of(&e).map(|a| (a.stroke_color, a.brush.is_some())).expect("Pfad")
}

fn key_tap(h: &mut H, k: egui::Key, m: Modifiers) {
    h.event(Event::Key { key: k, physical_key: None, pressed: true, repeat: false, modifiers: m });
    h.event(Event::Key { key: k, physical_key: None, pressed: false, repeat: false, modifiers: m });
    h.run_steps(2);
}

/// HEX im Farb-Popup der Werkzeugleiste tippen und mit Enter übernehmen (Feld wie im FARB-1b-Test).
fn well_hex(h: &mut H, stroke: bool, hex: &str) {
    use fp_ui::inspector::color_picker as cp;
    let sw = egui::Id::new(("well", stroke));
    let well = h.ctx.read_response(sw).expect("Farbfeld der Werkzeugleiste").rect;
    click_at(h, egui::pos2(well.right() - well.width() / 4.0, well.center().y));
    let p = cp::open_rect(&h.ctx, sw).expect("Popup offen");
    click_at(h, p.min + egui::vec2(180.0, 201.0));
    let ctrl = Modifiers { alt: false, ctrl: true, shift: false, mac_cmd: false, command: true };
    key_tap(h, egui::Key::A, ctrl);
    for ch in hex.chars() {
        h.event(Event::Text(ch.to_string()));
        h.run_steps(1);
    }
    key_tap(h, egui::Key::Enter, Modifiers::NONE);
    assert!(cp::open_rect(&h.ctx, sw).is_none(), "Enter schließt");
}

/// CT-2 FARB-1b: Farbe aus dem Popup der Werkzeugleiste wirkt wie im Original auf das gewählte
/// Objekt (Füllung bzw. Strich, je ein Verlaufsschritt, Rückgängig exakt); ohne Auswahl bzw. mit
/// gewählter Bitmap nur die Werkzeugfarbe.
#[test]
fn toolbar_color_applies_to_selection() {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("Dokument");
    let v = s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80], "fillColor": "#3a7bd5", "strokeWidth": 0})).expect("Rechteck");
    let r = ElementId(v.value["id"].as_u64().expect("id"));
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.sync_render = true;
    let mut h = harness(a);
    h.state_mut().state.session.execute("select.set", json!({"ids": [r.0]})).expect("Auswahl");
    h.run_steps(3);
    let n0 = st(&h).session.active().map(|d| d.history.undo_count()).unwrap_or(0);
    well_hex(&mut h, false, "123456");
    assert_eq!(st(&h).session.tools.fill_color, fp_color::Argb(0xff12_3456), "Werkzeugfarbe");
    assert_eq!(fill_of(&h, r), fp_color::Argb(0xff12_3456), "Füllung des gewählten Rechtecks");
    assert_eq!(st(&h).session.active().map(|d| d.history.undo_count()).unwrap_or(0), n0 + 1, "ein Verlaufsschritt");
    // Strich: Rechteck ohne Strich bekommt einen
    well_hex(&mut h, true, "ff00ff");
    assert_eq!(stroke_of(&h, r), (fp_color::Argb(0xffff_00ff), true), "Strich des Rechtecks");
    assert_eq!(st(&h).session.active().map(|d| d.history.undo_count()).unwrap_or(0), n0 + 2);
    h.state_mut().state.session.undo().expect("Rückgängig");
    h.state_mut().state.session.undo().expect("Rückgängig");
    assert_eq!(fill_of(&h, r), fp_color::Argb(0xff3a_7bd5), "Rückgängig stellt die Füllung her");
    assert!(!stroke_of(&h, r).1, "Rückgängig: wieder ohne Strich");
    // ohne Auswahl: nur die Werkzeugfarbe
    h.state_mut().state.session.execute("select.none", json!({})).expect("nichts");
    h.run_steps(2);
    let n1 = st(&h).session.active().map(|d| d.history.undo_count()).unwrap_or(0);
    well_hex(&mut h, false, "00ff00");
    assert_eq!(st(&h).session.tools.fill_color, fp_color::Argb(0xff00_ff00));
    assert_eq!(fill_of(&h, r), fp_color::Argb(0xff3a_7bd5), "ohne Auswahl bleibt das Rechteck");
    assert_eq!(st(&h).session.active().map(|d| d.history.undo_count()).unwrap_or(0), n1, "kein Verlaufsschritt");
}

/// CT-2 FARB-1b (6): Der Klappschieber der Deckkraft bleibt im Fenster – reicht der Platz unter dem
/// Knopf nicht (900 × 650, Popup unten), wird er nach oben geschoben; sonst unverändert darunter.
#[test]
fn opacity_slider_stays_in_window() {
    use fp_ui::inspector::color_picker::{slider_rect, POPUP_H};
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900.0, 650.0));
    // Popup weit oben: Schieber direkt unter dem Knopf (Lage wie bisher)
    let r = slider_rect(egui::pos2(100.0, 40.0), screen);
    assert_eq!((r.left(), r.top(), r.width(), r.height()), (147.0, 275.0, 24.0, 222.0));
    // Popup unten im 900 × 650-Fenster (wie im Klicktest, Schieber ab y ≈ 480): ganz im Fenster
    let o = egui::pos2(60.0, 650.0 - POPUP_H - 2.0);
    let r = slider_rect(o, screen);
    assert!(r.bottom() <= 650.0 && r.top() >= 0.0, "{r:?}");
    assert_eq!(r.height(), 222.0, "volle Höhe, alle Werte erreichbar");
}

/// CT-2: Nach „Pinselstrich bearbeiten“ (Werkzeugvorgabe, Größe 30) zeigt das Größenfeld des
/// Inspektors 30 statt der alten 35; Abbrechen lässt 35 stehen.
#[test]
fn brush_edit_dialog_updates_inspector_size() {
    use egui_kittest::kittest::Queryable;
    let (a, _) = app_with_bitmap([200, 200, 200, 255]);
    let mut h = harness(a);
    h.state_mut().state.session.execute("select.none", json!({})).expect("nichts");
    h.state_mut().state.session.set_tool(ToolId::Brush).expect("Pinsel");
    h.run_steps(3);
    let ctx = h.ctx.clone();
    fp_ui::inspector::obj_quads::edit_tool_attrs(&ctx, |t| t.stroke_width = 35);
    h.run_steps(2);
    for (ok, want) in [(false, 35), (true, 30)] {
        fp_ui::dialogs::brush_edit::open(&mut h.state_mut().state, true);
        if let Some(d) = h.state_mut().state.dialogs.brush_edit.as_mut() {
            d.brush.diameter = 30;
        }
        h.run_steps(3);
        let label = if ok { "OK" } else { "Abbrechen" };
        h.get_all(egui_kittest::kittest::By::new().label(label)).last().unwrap_or_else(|| panic!("{label}")).click();
        h.run_steps(4);
        assert!(h.state().state.dialogs.brush_edit.is_none(), "Dialog zu");
        assert_eq!(fp_ui::inspector::obj_quads::tool_attrs(&ctx).stroke_width, want, "Größenfeld nach {label}");
    }
}

/// CT-2: Die Zeile eines Live-Filters im Inspektor und ihr ⓘ (Einstellungen) haben Namen für
/// die Barrierefreiheit; ein Klick auf ⓘ öffnet weiter die Einstellungen.
#[test]
fn live_filter_row_info_has_accessible_name() {
    use egui_kittest::kittest::Queryable;
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("Dokument");
    let v = s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80], "fillColor": "#3a7bd5"})).expect("Rechteck");
    let r = v.value["id"].as_u64().expect("id");
    s.execute("select.set", json!({"ids": [r]})).expect("Auswahl");
    s.execute("element.addEffect", json!({"effect": "glow"})).expect("Glühen");
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.sync_render = true;
    let mut h = harness(a);
    h.run_steps(4);
    let name = fp_i18n::t("pi-fx-edit");
    let info = h.get_by_label(&name).rect();
    assert!(h.query_all(egui_kittest::kittest::By::new().role(egui::accesskit::Role::Button).label(&name)).next().is_some(), "ⓘ als Knopf mit Namen");
    assert!(h.state().state.dialogs.form.is_none());
    click_at(&mut h, info.center());
    h.run_steps(3);
    assert!(h.state().state.dialogs.form.is_some(), "Klick auf ⓘ öffnet die Einstellungen");
}

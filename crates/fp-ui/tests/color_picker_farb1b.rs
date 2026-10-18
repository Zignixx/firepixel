//! FARB-1b, Nutzerbefund N27b: überall der Farbwähler des Originals (re\22-ansicht-bitmap-farbe.md §2).
//! Raster gegen die Messung, Rasterklick, HEX/RGB tippen, Pipette über der Leinwand, Esc/Enter,
//! 50× wählen ohne Effekt-Stapel, genau ein Verlaufsschritt, Lage am oberen Rand und dasselbe Popup
//! an allen Aufrufstellen (AGENTEN_REGELN §4b, echte Ereignisse über mehrere Frames).

use egui::{Event, Id, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use fp_color::Argb;
use fp_engine::Session;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::effects;
use fp_ui::inspector::color_picker::{self as cp, POPUP_H, POPUP_W};
use fp_ui::FirepixelApp;
use serde_json::json;

/// Rasterfarben der Messung (`re\_testdateien\par\ref\color-popup\grid_21x15.csv`).
const GRID_CSV: &str = "\
r0;FFBFBF;FFCFBF;FFDFBF;FFEFBF;FFFFBF;EFFFBF;DFFFBF;CFFFBF;BFFFBF;BFFFCF;BFFFDF;BFFFEF;BFFFFF;BFEFFF;BFDFFF;BFCFFF;BFBFFF;CFBFFF;DFBFFF;EFBFFF;FFBFFF
r1;FF9999;FFB399;FFCC99;FFE599;FFFF99;E5FF99;CCFF99;B3FF99;99FF99;99FFB3;99FFCC;99FFE5;99FFFF;99E5FF;99CCFF;99B3FF;9999FF;B399FF;CC99FF;E599FF;FF99FF
r2;FF7373;FF9673;FFB973;FFDC73;FFFF73;DCFF73;B9FF73;96FF73;73FF73;73FF96;73FFB9;73FFDC;73FFFF;73DCFF;73B9FF;7396FF;7373FF;9673FF;B973FF;DC73FF;FF73FF
r3;FF4D4D;FF7A4D;FFA64D;FFD24D;FFFF4D;D2FF4D;A6FF4D;7AFF4D;4DFF4D;4DFF7A;4DFFA6;4DFFD2;4DFFFF;4DD2FF;4DA6FF;4D7AFF;4D4DFF;7A4DFF;A64DFF;D24DFF;FF4DFF
r4;FF2626;FF5C26;FF9326;FFC926;FFFF26;C9FF26;93FF26;5CFF26;26FF26;26FF5C;26FF93;26FFC9;26FFFF;26C9FF;2693FF;265CFF;2626FF;5C26FF;9326FF;C926FF;FF26FF
r5;FF0000;FF4000;FF8000;FFBF00;FFFF00;BFFF00;80FF00;40FF00;00FF00;00FF40;00FF80;00FFBF;00FFFF;00BFFF;0080FF;0040FF;0000FF;4000FF;8000FF;BF00FF;FF00FF
r6;D90000;D93600;D96D00;D9A300;D9D900;A3D900;6DD900;36D900;00D900;00D936;00D96D;00D9A3;00D9D9;00A3D9;006DD9;0036D9;0000D9;3600D9;6D00D9;A300D9;D900D9
r7;B20000;B22D00;B25900;B28500;B2B200;85B200;59B200;2DB200;00B200;00B22D;00B259;00B285;00B2B2;0085B2;0059B2;002DB2;0000B2;2D00B2;5900B2;8500B2;B200B2
r8;8C0000;8C2300;8C4600;8C6900;8C8C00;698C00;468C00;238C00;008C00;008C23;008C46;008C69;008C8C;00698C;00468C;00238C;00008C;23008C;46008C;69008C;8C008C
r9;660000;661A00;663300;664C00;666600;4C6600;336600;1A6600;006600;00661A;006633;00664C;006666;004C66;003366;001A66;000066;1A0066;330066;4C0066;660066
r10;400000;401000;402000;403000;404000;304000;204000;104000;004000;004010;004020;004030;004040;003040;002040;001040;000040;100040;200040;300040;400040
r11;000000;000000;000000;000000;000000;000000;000000;000000;000000;000000;000000;000000;000000;000000;000000;000000;000000;000000;000000;000000;000000
r12;FFFFFF;EEEEEE;DDDDDD;CCCCCC;BBBBBB;AAAAAA;999999;888888;777777;666666;555555;444444;333333;222222;111111;000000;000000;FF0000;FF7F00;FFFF00;7FFF00
r13;FFFFFF;F0F0E1;DFDFD0;CECEBF;BDBDAE;ACAC9D;9B9B8C;8A8A7B;79796A;686859;575748;464637;353526;242415;131304;000000;000000;007FFF;00FFFF;00FF7F;00FF00
r14;FFFFFF;ECECFB;DBDBEA;CACAD9;B9B9C8;A8A8B7;9797A6;868695;757584;646473;535362;424251;313140;20202F;0F0F1E;000000;000000;0000FF;7F00FF;FF00FF;FF007F
";

fn app(rects: &[(&str, [f64; 4])]) -> FirepixelApp {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200})).expect("doc");
    for (col, r) in rects {
        s.execute("draw.rectangle", json!({"rect": r, "fillColor": col, "strokeWidth": 0})).expect("rect");
    }
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.sync_render = true;
    a
}

fn harness_sized(a: FirepixelApp, w: f32, h: f32) -> Harness<'static, FirepixelApp> {
    Harness::builder().with_size(egui::vec2(w, h)).with_pixels_per_point(1.0).with_max_steps(16).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a)
}

fn harness(a: FirepixelApp) -> Harness<'static, FirepixelApp> {
    harness_sized(a, 1400.0, 900.0)
}

fn st<'a>(h: &'a Harness<'_, FirepixelApp>) -> &'a AppState {
    &h.state().state
}

fn effects_of(st: &AppState) -> Vec<fp_model::effects::Effect> {
    let d = st.session.active().expect("doc");
    let id = *fp_engine::docops::selectable_ids(&d.doc, d.page, d.frame).last().expect("Element");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("element");
    e.common.effects.clone().map(|l| l.effects).unwrap_or_default()
}

fn undo_count(st: &AppState) -> usize {
    st.session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn open_glow(h: &mut Harness<'_, FirepixelApp>) {
    h.run_steps(2);
    let i = effects::FX.iter().position(|f| f.menu_text() == "Glühen").expect("Glühen");
    effects::begin_add(&mut h.state_mut().state, i);
    h.run_steps(2);
    assert!(st(h).dialogs.form.is_some(), "Glühen-Dialog offen");
}

/// Id des Farbfelds `color` im offenen Formular (wie `form::draw_fields` → `color_button`).
fn glow_swatch(h: &Harness<'_, FirepixelApp>) -> Id {
    let f = st(h).dialogs.form.as_ref().expect("Dialog");
    let i = f.fields.iter().position(|x| x.key == "color").expect("Farbfeld");
    Id::new(Id::new((f.id.as_str(), i)))
}

fn form_color(h: &Harness<'_, FirepixelApp>) -> String {
    let f = st(h).dialogs.form.as_ref().expect("Dialog");
    f.fields.iter().find(|x| x.key == "color").and_then(|x| x.value.as_str()).expect("Farbwert").to_lowercase()
}

fn rect_of(h: &Harness<'_, FirepixelApp>, id: Id) -> Rect {
    h.ctx.read_response(id).unwrap_or_else(|| panic!("Widget {id:?} nicht gezeichnet")).rect
}

fn click_at(h: &mut Harness<'_, FirepixelApp>, p: Pos2) {
    h.event(Event::PointerMoved(p));
    h.run_steps(1);
    h.event(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run_steps(1);
    h.event(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

fn key(h: &mut Harness<'_, FirepixelApp>, k: Key, m: Modifiers) {
    h.event(Event::Key { key: k, physical_key: None, pressed: true, repeat: false, modifiers: m });
    h.event(Event::Key { key: k, physical_key: None, pressed: false, repeat: false, modifiers: m });
    h.run_steps(2);
}

const CTRL: Modifiers = Modifiers { alt: false, ctrl: true, shift: false, mac_cmd: false, command: true };

/// Tippt `text` Zeichen für Zeichen (jede Taste ein Ereignis) in das fokussierte Feld (vorher alles markieren).
fn type_into(h: &mut Harness<'_, FirepixelApp>, text: &str) {
    key(h, Key::A, CTRL);
    for ch in text.chars() {
        h.event(Event::Text(ch.to_string()));
        h.run_steps(1);
    }
}

/// Öffnet das Popup am Farbfeld `swatch` (Klick in die Mitte) und liefert sein Rechteck.
fn open_popup(h: &mut Harness<'_, FirepixelApp>, swatch: Id) -> Rect {
    let r = rect_of(h, swatch);
    click_at(h, r.center());
    h.run_steps(1);
    cp::open_rect(&h.ctx, swatch).unwrap_or_else(|| panic!("Popup an {swatch:?} offen"))
}

fn cell_center(popup: Rect, c: usize, r: usize) -> Pos2 {
    cp::cell_rect(popup.min, c, r).center()
}

/// Bildschirmpunkt eines Dokumentpunkts der aktiven Seite.
fn doc_to_screen(h: &Harness<'_, FirepixelApp>, x: f64, y: f64) -> Pos2 {
    let s = st(h);
    let d = s.session.active().expect("doc");
    let area = s.canvases.get(&d.id).and_then(|c| c.area).expect("Leinwand gezeichnet");
    let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(x as f32, y as f32));
    egui::pos2(area.min.x + sx as f32, area.min.y + sy as f32)
}

#[test]
fn grid_matches_measurement() {
    for line in GRID_CSV.lines() {
        let mut parts = line.split(';');
        let r: usize = parts.next().expect("Zeile")[1..].parse().expect("Zeilennummer");
        for (c, hex) in parts.enumerate() {
            let want = u32::from_str_radix(hex, 16).expect("hex");
            assert_eq!(cp::grid_color(c, r).0 & 0xff_ffff, want, "Zelle r{r} c{c}");
        }
    }
}

#[test]
fn popup_size_and_place_above_swatch() {
    let mut h = harness(app(&[("#ff0000", [20.0, 20.0, 120.0, 80.0])]));
    open_glow(&mut h);
    let sw = glow_swatch(&h);
    let swr = rect_of(&h, sw);
    let p = open_popup(&mut h, sw);
    assert_eq!((p.width(), p.height()), (POPUP_W, POPUP_H), "259 × 277 wie im Original");
    if swr.top() >= POPUP_H {
        assert_eq!(p.bottom(), swr.top(), "Unterkante auf der Oberkante des Farbfelds");
    } else {
        assert_eq!(p.top(), 0.0, "oben zu wenig Platz: am oberen Rand");
    }
    assert!((p.left() - swr.center().x).abs() <= 1.0, "linke Kante beim Klickpunkt: {} / {}", p.left(), swr.center().x);
}

#[test]
fn popup_at_top_edge_when_no_room_above() {
    // niedriges Fenster: der Dialog und sein Farbfeld liegen weit oben
    let mut h = harness_sized(app(&[("#ff0000", [20.0, 20.0, 120.0, 80.0])]), 1400.0, 340.0);
    open_glow(&mut h);
    let sw = glow_swatch(&h);
    let swr = rect_of(&h, sw);
    assert!(swr.top() < POPUP_H, "Testaufbau: Farbfeld bei y = {}", swr.top());
    let p = open_popup(&mut h, sw);
    assert_eq!(p.top(), 0.0, "Popup am oberen Rand, verdeckt das Farbfeld");
}

#[test]
fn grid_click_sets_exact_color_and_dialog_stays() {
    let mut h = harness(app(&[("#ff0000", [20.0, 20.0, 120.0, 80.0])]));
    open_glow(&mut h);
    let sw = glow_swatch(&h);
    for (c, r) in [(8usize, 5usize), (3, 1), (18, 13), (5, 12)] {
        let p = open_popup(&mut h, sw);
        let want = cp::grid_color(c, r);
        h.event(Event::PointerMoved(cell_center(p, c, r)));
        h.run_steps(2);
        assert_eq!(cp::live_color(&h.ctx, sw), Some(Some(want)), "Vorschau zeigt die überfahrene Zelle live");
        click_at(&mut h, cell_center(p, c, r));
        assert!(cp::open_rect(&h.ctx, sw).is_none(), "Klick auf Zelle schließt");
        assert!(st(&h).dialogs.form.is_some(), "Effekt-Dialog bleibt offen");
        assert_eq!(&form_color(&h)[..7], format!("#{:06x}", want.0 & 0xff_ffff), "genaue Rasterfarbe r{r} c{c}");
    }
}

fn button(h: &mut Harness<'_, FirepixelApp>, p: Pos2, pressed: bool) {
    h.event(Event::PointerMoved(p));
    h.event(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

#[test]
fn grid_drag_takes_cell_under_pointer_on_release() {
    let mut h = harness(app(&[("#ff0000", [20.0, 20.0, 120.0, 80.0])]));
    open_glow(&mut h);
    let sw = glow_swatch(&h);
    let start = form_color(&h);
    // press on one cell, drag to another, release: the cell at release wins (TW-112)
    let p = open_popup(&mut h, sw);
    button(&mut h, cell_center(p, 3, 1), true);
    assert!(cp::open_rect(&h.ctx, sw).is_some(), "press alone keeps the popup open");
    assert_eq!(form_color(&h), start, "press alone takes nothing");
    h.event(Event::PointerMoved(cell_center(p, 10, 6)));
    h.run_steps(2);
    h.event(Event::PointerMoved(cell_center(p, 18, 13)));
    h.run_steps(2);
    let want = cp::grid_color(18, 13);
    assert_eq!(cp::live_color(&h.ctx, sw), Some(Some(want)), "preview follows the drag");
    button(&mut h, cell_center(p, 18, 13), false);
    assert!(cp::open_rect(&h.ctx, sw).is_none(), "release closes");
    assert_eq!(&form_color(&h)[..7], format!("#{:06x}", want.0 & 0xff_ffff), "cell under the pointer at release");
    // Esc while the button is held cancels
    let before = form_color(&h);
    let p = open_popup(&mut h, sw);
    button(&mut h, cell_center(p, 5, 12), true);
    key(&mut h, Key::Escape, Modifiers::NONE);
    assert!(cp::open_rect(&h.ctx, sw).is_none(), "Esc closes");
    button(&mut h, cell_center(p, 5, 12), false);
    assert_eq!(form_color(&h), before, "Esc while held takes nothing");
    assert!(st(&h).dialogs.form.is_some(), "effect dialog stays open");
}

#[test]
fn hex_typed_live_then_enter_applies_and_esc_discards() {
    let mut h = harness(app(&[("#ff0000", [20.0, 20.0, 120.0, 80.0])]));
    open_glow(&mut h);
    let sw = glow_swatch(&h);
    let start = form_color(&h);
    // Esc nach getippter HEX: Rücknahme, Dialog bleibt offen
    let p = open_popup(&mut h, sw);
    click_at(&mut h, p.min + egui::vec2(180.0, 201.0));
    type_into(&mut h, "123456");
    h.event(Event::PointerMoved(p.min + egui::vec2(180.0, 201.0)));
    h.run_steps(2);
    assert_eq!(cp::live_color(&h.ctx, sw), Some(Some(Argb(0xff12_3456))), "Vorschau folgt dem Getippten live");
    assert_eq!(form_color(&h), start, "vor Enter ändert sich nichts");
    key(&mut h, Key::Escape, Modifiers::NONE);
    assert!(cp::open_rect(&h.ctx, sw).is_none(), "Esc schließt");
    assert!(st(&h).dialogs.form.is_some(), "Esc schließt nur das Popup, nicht den Dialog");
    assert_eq!(form_color(&h), start, "Esc verwirft");
    // Enter übernimmt (ohne „#“ getippt)
    let p = open_popup(&mut h, sw);
    click_at(&mut h, p.min + egui::vec2(180.0, 201.0));
    type_into(&mut h, "123456");
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert!(cp::open_rect(&h.ctx, sw).is_none(), "Enter schließt");
    assert!(st(&h).dialogs.form.is_some(), "Enter bestätigt nicht den Dialog");
    assert_eq!(&form_color(&h)[..7], "#123456");
}

#[test]
fn rgb_typed_with_tab_then_enter() {
    let mut h = harness(app(&[("#ff0000", [20.0, 20.0, 120.0, 80.0])]));
    open_glow(&mut h);
    let sw = glow_swatch(&h);
    let p = open_popup(&mut h, sw);
    click_at(&mut h, p.min + egui::vec2(152.0, 224.0));
    type_into(&mut h, "200");
    key(&mut h, Key::Tab, Modifiers::NONE);
    type_into(&mut h, "100");
    key(&mut h, Key::Tab, Modifiers::NONE);
    type_into(&mut h, "50");
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert!(cp::open_rect(&h.ctx, sw).is_none());
    assert_eq!(&form_color(&h)[..7], "#c86432", "R 200, G 100, B 50");
}

#[test]
fn typing_without_field_click_is_ignored_enter_closes_without_change() {
    let mut h = harness(app(&[("#ff0000", [20.0, 20.0, 120.0, 80.0])]));
    h.run_steps(2);
    let tool0 = st(&h).session.tools.active;
    // Werkzeugleiste Füllung (kein Dialog: Tastenkürzel der Werkzeuge wären sonst aktiv)
    let well = rect_of(&h, Id::new(("well", false)));
    let at = egui::pos2(well.right() - well.width() / 4.0, well.center().y);
    click_at(&mut h, at);
    let sw = Id::new(("well", false));
    assert!(cp::open_rect(&h.ctx, sw).is_some(), "Popup der Werkzeugleiste offen");
    let fill0 = st(&h).session.tools.fill_color;
    for ch in ["v", "b", "1"] {
        h.event(Event::Text(ch.into()));
        h.event(Event::Key { key: Key::V, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
        h.run_steps(1);
    }
    assert_eq!(st(&h).session.tools.active, tool0, "Tippen ohne Feldklick wählt kein Werkzeug");
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert!(cp::open_rect(&h.ctx, sw).is_none(), "Enter schließt");
    assert_eq!(st(&h).session.tools.fill_color, fill0, "ohne Änderung");
}

#[test]
fn pipette_over_canvas_live_and_click_picks_pixel() {
    let mut h = harness(app(&[("#00ff00", [150.0, 20.0, 290.0, 180.0])]));
    h.run_steps(3);
    let _ = h.state_mut().state.session.execute("select.none", json!({}));
    h.run_steps(2);
    let sw = Id::new(("well", false));
    let well = rect_of(&h, sw);
    click_at(&mut h, egui::pos2(well.right() - well.width() / 4.0, well.center().y));
    assert!(cp::open_rect(&h.ctx, sw).is_some(), "Popup offen");
    let green = doc_to_screen(&h, 220.0, 100.0);
    let white = doc_to_screen(&h, 60.0, 100.0);
    let sel0 = st(&h).session.active().map(|d| d.selection.elements.clone());
    h.event(Event::PointerMoved(white));
    h.run_steps(2);
    assert_eq!(cp::live_color(&h.ctx, sw), Some(Some(Argb(0xffff_ffff))), "live: weiße Leinwand");
    h.event(Event::PointerMoved(green));
    h.run_steps(2);
    assert_eq!(cp::live_color(&h.ctx, sw), Some(Some(Argb(0xff00_ff00))), "live: grünes Rechteck");
    assert_ne!(st(&h).session.tools.fill_color.0 & 0xff_ffff, 0x00ff00, "Überfahren ändert noch nichts");
    let u0 = undo_count(st(&h));
    click_at(&mut h, green);
    assert!(cp::open_rect(&h.ctx, sw).is_none(), "Klick außerhalb schließt");
    assert_eq!(st(&h).session.tools.fill_color.0 & 0xff_ffff, 0x00ff00, "Klick übernimmt das Leinwandpixel");
    assert_eq!(st(&h).session.active().map(|d| d.selection.elements.clone()), sel0, "der Klick erreicht die Leinwand nicht (keine Auswahländerung)");
    assert_eq!(undo_count(st(&h)), u0, "Werkzeugfarbe: kein Dokumentschritt");
}

#[test]
fn inspector_fill_pipette_is_one_history_step() {
    // rotes Rechteck gewählt, Farbe vom grünen aufnehmen
    let mut h = harness(app(&[("#00ff00", [150.0, 20.0, 290.0, 180.0]), ("#ff0000", [10.0, 20.0, 100.0, 180.0])]));
    h.run_steps(3);
    let sw = Id::new(Id::new(Id::new(("pi-fill", false)).with("sw")));
    let u0 = undo_count(st(&h));
    let p = open_popup(&mut h, sw);
    assert_eq!((p.width(), p.height()), (POPUP_W, POPUP_H));
    let green = doc_to_screen(&h, 220.0, 100.0);
    click_at(&mut h, green);
    assert!(cp::open_rect(&h.ctx, sw).is_none());
    let d = st(&h).session.active().expect("doc");
    let id = *d.selection.elements.first().expect("Auswahl bleibt");
    let _ = id;
    let img = st(&h).session.renderer().and_then(|r| r.render_page(&d.doc, d.page, d.frame)).expect("Seite gerendert");
    assert_eq!(img.get(50, 100).map(|p| [p[0], p[1], p[2]]), Some([0, 255, 0]), "gewähltes Rechteck jetzt grün");
    assert_eq!(undo_count(st(&h)), u0 + 1, "genau ein Verlaufsschritt");
}

#[test]
fn fifty_picks_no_effect_stack_and_ok_is_one_step() {
    let mut h = harness(app(&[("#ff0000", [20.0, 20.0, 120.0, 80.0])]));
    h.run_steps(2);
    let u0 = undo_count(st(&h));
    open_glow(&mut h);
    let sw = glow_swatch(&h);
    let mut last = 0;
    for k in 0..50 {
        let p = open_popup(&mut h, sw);
        let (c, r) = (k % 21, (k * 7) % 11);
        click_at(&mut h, cell_center(p, c, r));
        last = cp::grid_color(c, r).0 & 0xff_ffff;
        assert_eq!(&form_color(&h)[..7], format!("#{last:06x}"), "Durchgang {k}");
        assert!(effects_of(st(&h)).len() <= 1, "Durchgang {k}: {} Effekte", effects_of(st(&h)).len());
    }
    let b: Vec<_> = h.get_all(egui_kittest::kittest::By::new().label("OK")).collect();
    b.last().expect("OK").click();
    drop(b);
    h.run_steps(3);
    assert!(st(&h).dialogs.form.is_none(), "Dialog zu");
    let fx = effects_of(st(&h));
    assert_eq!(fx.len(), 1, "genau ein Glühen");
    assert_eq!(undo_count(st(&h)), u0 + 1, "OK = genau ein Verlaufsschritt");
    let col = fx[0].get_color("OuterBevelColor").expect("Glühfarbe");
    assert_eq!(((col.r() as u32) << 16 | (col.g() as u32) << 8 | col.b() as u32), last, "Effekt trägt die zuletzt gewählte Farbe");
}

#[test]
fn all_call_sites_open_the_same_popup() {
    let mut h = harness(app(&[("#ff0000", [20.0, 20.0, 120.0, 80.0])]));
    h.run_steps(3);
    let mut seen = Vec::new();
    // Werkzeugleiste Strich/Füllung
    for stroke in [true, false] {
        let sw = Id::new(("well", stroke));
        let well = rect_of(&h, sw);
        click_at(&mut h, egui::pos2(well.right() - well.width() / 4.0, well.center().y));
        let p = cp::open_rect(&h.ctx, sw).expect("Werkzeugleiste");
        seen.push(("toolbar", p.size()));
        key(&mut h, Key::Escape, Modifiers::NONE);
        assert!(cp::open_rect(&h.ctx, sw).is_none());
    }
    // Inspektor Füllung/Strich (Auswahl)
    for base in [Id::new(("pi-fill", false)), Id::new(("pi-stroke", false))] {
        let sw = Id::new(Id::new(base.with("sw")));
        let p = open_popup(&mut h, sw);
        seen.push(("inspector", p.size()));
        key(&mut h, Key::Escape, Modifiers::NONE);
    }
    // Neues Dokument ▸ Benutzerdefiniert
    let prefs = st(&h).session.prefs.clone();
    h.state_mut().state.dialogs.new_document = Some(fp_ui::dialogs::NewDocumentDialog::from_prefs(&prefs));
    h.run_steps(3);
    let sw = Id::new("nd-custom-color");
    let p = open_popup(&mut h, sw);
    seen.push(("new document", p.size()));
    key(&mut h, Key::Escape, Modifiers::NONE);
    assert!(st(&h).dialogs.new_document.is_some(), "Esc schließt nur das Popup");
    h.state_mut().state.dialogs.new_document = None;
    h.run_steps(2);
    // Effekt-Dialog
    open_glow(&mut h);
    let sw = glow_swatch(&h);
    let p = open_popup(&mut h, sw);
    seen.push(("effect", p.size()));
    for (what, s) in &seen {
        assert_eq!((s.x, s.y), (POPUP_W, POPUP_H), "{what}: dasselbe Popup 259 × 277");
    }
}

/// T-10 (re 42 §4): ungültiger HEX-Text („zz12“) wird schon beim Tippen zu #000000 und mit Enter
/// übernommen; das HEX-Feld hat beim Öffnen keinen Fokus.
#[test]
fn t10_invalid_hex_becomes_black_and_hex_not_focused_on_open() {
    assert_eq!(cp::hex_value("zz12"), 0);
    assert_eq!(cp::hex_value("#3366CC"), 0x3366cc);
    assert_eq!(cp::hex_value("3366"), 0x336600, "incomplete: missing pair = 0 (re 46 §2.2)");
    assert_eq!(cp::hex_value(""), 0xff_ffff, "empty = white");
    let mut h = harness(app(&[("#ff0000", [20.0, 20.0, 120.0, 80.0])]));
    open_glow(&mut h);
    let sw = glow_swatch(&h);
    let p = open_popup(&mut h, sw);
    let hex_id = cp::popup_id(sw).with("hex");
    assert_ne!(h.ctx.memory(|m| m.focused()), Some(hex_id), "HEX-Feld beim Öffnen ohne Fokus");
    click_at(&mut h, p.min + egui::vec2(180.0, 201.0));
    assert_eq!(h.ctx.memory(|m| m.focused()), Some(hex_id), "Klick ins Feld fokussiert");
    type_into(&mut h, "zz12");
    h.event(Event::PointerMoved(p.min + egui::vec2(180.0, 201.0)));
    h.run_steps(2);
    assert_eq!(cp::live_color(&h.ctx, sw), Some(Some(Argb(0xff00_0000))), "live schon beim Tippen schwarz");
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert!(cp::open_rect(&h.ctx, sw).is_none(), "Enter schließt");
    assert_eq!(&form_color(&h)[..7], "#000000");
}

/// Q-49 (re 46 §2.3): every keystroke in the HEX field sets the color (shorthand already after the
/// third digit); Enter closes the popup with the last value.
#[test]
fn q49_hex_field_commits_every_keystroke() {
    let mut h = harness(app(&[("#ff0000", [20.0, 20.0, 120.0, 80.0])]));
    open_glow(&mut h);
    let sw = glow_swatch(&h);
    let p = open_popup(&mut h, sw);
    click_at(&mut h, p.min + egui::vec2(180.0, 201.0));
    type_into(&mut h, "36c");
    h.event(Event::PointerMoved(p.min + egui::vec2(180.0, 201.0)));
    h.run_steps(2);
    assert_eq!(cp::live_color(&h.ctx, sw), Some(Some(Argb(0xff33_66cc))), "shorthand while typing");
    h.event(Event::Text("z".into()));
    h.run_steps(2);
    assert_eq!(cp::live_color(&h.ctx, sw), Some(Some(Argb(0xff36_0c00))), "36cz: pairs 36, 0c, then stop");
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert!(cp::open_rect(&h.ctx, sw).is_none(), "Enter closes");
    assert_eq!(&form_color(&h)[..7].to_ascii_lowercase(), "#360c00");
}

//! FARB-1a, Nutzerbefund N27a: Im Farbfelder-Popup sprang der genaue Farbwähler (Systemfarbwähler)
//! beim Ziehen jeden Frame auf die Ausgangsfarbe zurück. Geprüft am Glühen-Dialog (dieselbe
//! zentrale Funktion `inspector::popups::color_popup` nutzen Inspektor, Bedienfelder und Dialoge)
//! mit echtem Ziehen über viele Frames (AGENTEN_REGELN §4b). Seit FARB-1b ist der genaue Farbwähler der
//! Farbdialog hinter dem Farbkreis-Knopf des Popups; Esc schließt dort zuerst nur den Farbdialog.

use egui::{Color32, Event, Id, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use fp_engine::Session;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::effects;
use fp_ui::FirepixelApp;
use serde_json::json;

fn app_with_rect() -> FirepixelApp {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80]})).expect("rect");
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.sync_render = true;
    a
}

fn harness(a: FirepixelApp) -> Harness<'static, FirepixelApp> {
    Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(16).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a)
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

/// Öffnet den Glühen-Dialog für das Rechteck.
fn open_glow(h: &mut Harness<'_, FirepixelApp>) {
    h.run_steps(2);
    let i = effects::FX.iter().position(|f| f.menu_text() == "Glühen").expect("Glühen");
    effects::begin_add(&mut h.state_mut().state, i);
    h.run_steps(2);
    assert!(st(h).dialogs.form.is_some(), "Glühen-Dialog offen");
}

/// Id-Basis des Farbfelds `color` im offenen Formular (wie `form::draw_fields` → `color_button`).
fn color_base_id(h: &Harness<'_, FirepixelApp>) -> Id {
    let f = st(h).dialogs.form.as_ref().expect("Dialog");
    let i = f.fields.iter().position(|x| x.key == "color").expect("Farbfeld");
    Id::new((f.id.as_str(), i))
}

fn form_color(h: &Harness<'_, FirepixelApp>) -> String {
    let f = st(h).dialogs.form.as_ref().expect("Dialog");
    f.fields.iter().find(|x| x.key == "color").and_then(|x| x.value.as_str()).expect("Farbwert").to_string()
}

/// Arbeitsfarbe des genauen Farbwählers (noch nicht übernommen).
fn pending(h: &Harness<'_, FirepixelApp>) -> Option<Color32> {
    let key = fp_ui::inspector::popups::picker_id(Id::new(color_base_id(h))).with("syscol");
    h.ctx.data(|d| d.get_temp::<Color32>(key))
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

/// Echtes Ziehen über `n` Frames von `a` nach `b`; nach jedem Frame `each` aufrufen.
fn drag(h: &mut Harness<'_, FirepixelApp>, a: Pos2, b: Pos2, n: usize, mut each: impl FnMut(&Harness<'_, FirepixelApp>, usize)) {
    h.event(Event::PointerMoved(a));
    h.run_steps(1);
    h.event(Event::PointerButton { pos: a, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run_steps(1);
    for k in 1..=n {
        let p = a + (b - a) * (k as f32 / n as f32);
        h.event(Event::PointerMoved(p));
        h.run_steps(1);
        each(h, k);
    }
    h.event(Event::PointerButton { pos: b, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

/// Klickt das Farbfeld (Popup auf) und den Knopf des genauen Farbwählers.
fn open_sys_picker(h: &mut Harness<'_, FirepixelApp>) {
    let base = color_base_id(h);
    let sw = rect_of(h, Id::new(base));
    click_at(h, sw.center());
    // das Popup findet seinen Platz erst nach dem ersten Bild (Größe noch unbekannt)
    h.run_steps(3);
    assert!(egui::Popup::is_id_open(&h.ctx, Id::new(base).with("popup")), "Farbfelder-Popup offen");
    let sysb = Id::new(fp_ui::inspector::popups::picker_id(Id::new(base)).with("sysb"));
    let r = rect_of(h, sysb);
    // Der Umschalter bleibt zwischen Öffnungen gemerkt: nur einschalten, wenn noch aus
    let sys_on = h.ctx.data(|d| d.get_temp::<bool>(fp_ui::inspector::popups::picker_id(Id::new(base)).with("sys"))).unwrap_or(false);
    if !sys_on {
        click_at(h, r.center());
    }
    assert!(h.ctx.data(|d| d.get_temp::<bool>(fp_ui::inspector::popups::picker_id(Id::new(base)).with("sys"))).unwrap_or(false), "genauer Farbwähler offen");
    h.run_steps(3);
}

/// Rechtecke des großen Farbfelds (Sättigung/Helligkeit) und der Farbtonleiste des egui-Farbwählers.
fn sv_and_hue(h: &Harness<'_, FirepixelApp>) -> (Rect, Rect) {
    let rects: Vec<Rect> = h.get_all(egui_kittest::kittest::By::new().predicate(|n| n.bounding_box().is_some_and(|b| b.width() > 40.0 && b.height() > 8.0))).map(|n| n.rect()).collect();
    let sv = *rects.iter().filter(|r| (r.width() - r.height()).abs() < 1.0 && r.width() >= 60.0 && r.width() <= 400.0).max_by(|a, b| a.top().total_cmp(&b.top())).expect("Farbfeld (quadratisch)");
    let hue = *rects.iter().filter(|r| (r.width() - sv.width()).abs() < 1.0 && r.height() < 40.0 && r.top() >= sv.bottom() - 1.0).min_by(|a, b| a.top().total_cmp(&b.top())).expect("Farbtonleiste unter dem Farbfeld");
    (sv, hue)
}

fn hue_of(c: Color32) -> f32 {
    egui::ecolor::Hsva::from(c).h
}

#[test]
fn drag_in_sv_field_changes_live_and_stays() {
    let mut h = harness(app_with_rect());
    open_glow(&mut h);
    let start = form_color(&h);
    open_sys_picker(&mut h);
    assert_eq!(pending(&h), None, "vor dem Ziehen keine Arbeitsfarbe");
    let (sv, _) = sv_and_hue(&h);
    // von links (grau) nach rechts oben (gesättigt, hell)
    let a = sv.left_center() + egui::vec2(2.0, 0.0);
    let b = sv.right_top() + egui::vec2(-2.0, 2.0);
    let mut seen: Vec<Color32> = Vec::new();
    drag(&mut h, a, b, 30, |h, _| {
        if let Some(c) = pending(h) {
            seen.push(c);
        }
    });
    assert!(seen.len() >= 25, "Farbe ändert sich beim Ziehen live ({} Frames mit Arbeitsfarbe)", seen.len());
    let distinct: std::collections::BTreeSet<[u8; 4]> = seen.iter().map(|c| c.to_array()).collect();
    assert!(distinct.len() >= 10, "Farbe wandert mit der Maus mit, statt zurückzuspringen ({} verschiedene)", distinct.len());
    let end = pending(&h).expect("Arbeitsfarbe bleibt nach dem Loslassen");
    assert_eq!(Some(&end), seen.last(), "kein Rücksprung nach dem Loslassen");
    let hsv = egui::ecolor::Hsva::from(end);
    assert!(hsv.s > 0.8 && hsv.v > 0.8, "rechts oben = gesättigt und hell: {end:?}");
    h.run_steps(5);
    assert_eq!(pending(&h), Some(end), "bleibt auch einige Frames später");
    assert_eq!(form_color(&h), start, "vor „Übernehmen“ ändert sich das Dialogfeld nicht");
    // Der Wähler zeigt nach dem Loslassen weiter die gezogene Farbe (nicht das alte Grau): ein
    // Klick in die Farbtonleiste behält Sättigung und Helligkeit des Felds
    let (_, hue) = sv_and_hue(&h);
    click_at(&mut h, hue.left_center() + egui::vec2(hue.width() * 0.33, 0.0));
    let after = egui::ecolor::Hsva::from(pending(&h).expect("Arbeitsfarbe"));
    assert!(after.s > 0.8 && after.v > 0.8, "Farbfeld sprang auf die Ausgangsfarbe zurück: {after:?}");
    assert!((after.h - 0.33).abs() < 0.08, "Farbton aus der Leiste: {}", after.h);
}

#[test]
fn hue_slider_drag_changes_live_and_stays() {
    let mut h = harness(app_with_rect());
    open_glow(&mut h);
    open_sys_picker(&mut h);
    let (sv, hue) = sv_and_hue(&h);
    // erst Sättigung geben (Grau hat keinen Farbton)
    drag(&mut h, sv.left_center() + egui::vec2(2.0, 0.0), sv.right_top() + egui::vec2(-2.0, 2.0), 10, |_, _| {});
    let mut hues: Vec<f32> = Vec::new();
    drag(&mut h, hue.left_center() + egui::vec2(2.0, 0.0), hue.center() + egui::vec2(hue.width() * 0.3, 0.0), 25, |h, _| {
        hues.push(hue_of(pending(h).expect("Arbeitsfarbe")));
    });
    let distinct: std::collections::BTreeSet<u32> = hues.iter().map(|x| (x * 1000.0) as u32).collect();
    assert!(distinct.len() >= 10, "Farbton ändert sich beim Ziehen live: {hues:?}");
    let end = pending(&h).expect("Arbeitsfarbe");
    assert!((hue_of(end) - 0.8).abs() < 0.08, "Farbton am Ende ≈ 0,8: {}", hue_of(end));
    h.run_steps(5);
    assert_eq!(pending(&h), Some(end), "Farbton bleibt nach dem Loslassen");
}

#[test]
fn escape_discards_and_reopen_starts_at_old_color() {
    let mut h = harness(app_with_rect());
    open_glow(&mut h);
    let start = form_color(&h);
    let fx0 = effects_of(st(&h));
    let u0 = undo_count(st(&h));
    open_sys_picker(&mut h);
    let (sv, _) = sv_and_hue(&h);
    drag(&mut h, sv.left_center() + egui::vec2(2.0, 0.0), sv.right_top() + egui::vec2(-2.0, 2.0), 15, |_, _| {});
    assert!(pending(&h).is_some());
    h.event(Event::Key { key: Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
    h.event(Event::Key { key: Key::Escape, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::NONE });
    h.run_steps(3);
    assert_eq!(pending(&h), None, "Esc verwirft die Arbeitsfarbe");
    // zweites Esc schließt das Popup (das erste nur den Farbdialog, FARB-1b)
    h.event(Event::Key { key: Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
    h.event(Event::Key { key: Key::Escape, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::NONE });
    h.run_steps(3);
    assert_eq!(form_color(&h), start, "Esc stellt den alten Wert her");
    assert_eq!(effects_of(st(&h)), fx0, "Vorschau unverändert");
    assert_eq!(undo_count(st(&h)), u0);
    // wieder öffnen: beginnt bei der alten Farbe, nicht bei der verworfenen
    if st(&h).dialogs.form.is_some() {
        open_sys_picker(&mut h);
        assert_eq!(pending(&h), None, "neu geöffnet ohne alte Arbeitsfarbe");
    }
}

#[test]
fn apply_sets_color_and_ok_is_one_history_step_even_after_50_drags() {
    let mut h = harness(app_with_rect());
    h.run_steps(2);
    // vor dem Öffnen messen: die Live-Vorschau des Dialogs belegt schon den einen Schritt
    let u0 = undo_count(st(&h));
    open_glow(&mut h);
    let start = form_color(&h);
    let mut last = None;
    for k in 0..50 {
        open_sys_picker(&mut h);
        let (sv, _) = sv_and_hue(&h);
        let y = sv.top() + 2.0 + (k % 10) as f32 * (sv.height() - 4.0) / 10.0;
        drag(&mut h, sv.left_center() + egui::vec2(2.0, 0.0), egui::pos2(sv.right() - 2.0 - (k % 7) as f32, y), 4, |_, _| {});
        let c = pending(&h).unwrap_or_else(|| panic!("Arbeitsfarbe, Durchgang {k}"));
        h.run_steps(1);
        let b: Vec<_> = h.get_all(egui_kittest::kittest::By::new().label("Übernehmen")).collect();
        b.last().expect("Knopf Übernehmen").click();
        drop(b);
        h.run_steps(3);
        let want = format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b());
        assert!(form_color(&h).to_lowercase().starts_with(&want), "Durchgang {k}: Feld {} statt {want}", form_color(&h));
        assert_eq!(pending(&h), None, "nach Übernehmen keine Arbeitsfarbe mehr");
        assert!(effects_of(st(&h)).len() <= 1, "Durchgang {k}: {} Effekte", effects_of(st(&h)).len());
        last = Some(want);
    }
    assert_ne!(form_color(&h), start);
    let b: Vec<_> = h.get_all(egui_kittest::kittest::By::new().label("OK")).collect();
    b.last().expect("OK").click();
    drop(b);
    h.run_steps(3);
    assert!(st(&h).dialogs.form.is_none(), "Dialog zu");
    let fx = effects_of(st(&h));
    assert_eq!(fx.len(), 1, "genau ein Glühen");
    assert_eq!(undo_count(st(&h)), u0 + 1, "OK = genau ein Verlaufsschritt");
    let col = fx[0].get_color("OuterBevelColor").expect("Glühfarbe");
    assert_eq!(format!("#{:02x}{:02x}{:02x}", col.r(), col.g(), col.b()), last.expect("Farbe"), "Effekt trägt die übernommene Farbe");
}

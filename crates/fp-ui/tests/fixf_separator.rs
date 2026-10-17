//! FIX-F (FIX-B-6b): Trennlinien in den ≡-Menüs der Bedienfelder im ganzen Programm (kittest, 125 %)
//! mit Klickkoordinaten aus der gezeichneten Oberfläche: Klicks auf eine Trennlinie und knapp
//! darüber/darunter führen nichts aus und lassen das Menü offen; ein Klick mitten in den Eintrag
//! darunter führt ihn aus und schließt das Menü.

use egui::{Event, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::kittest::{By, Queryable};
use egui_kittest::Harness;
use fp_engine::Session;
use fp_ui::app_state::UiSettings;
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness() -> H {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 400, "height": 300, "background": "#ffffff"})).expect("Dokument");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 100], "fillColor": "#ff0000"})).expect("Rechteck");
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.sync_render = true;
    // wie CT-1b: Windows mit 125 % Skalierung
    Harness::builder().with_size(egui::vec2(1120.0, 720.0)).with_pixels_per_point(1.25).with_max_steps(16).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a)
}

/// Klick wie die Brücke (`ui_input`): Bewegen, Drücken, Loslassen ohne Bild dazwischen.
fn click(h: &mut H, p: Pos2) {
    h.event(Event::PointerMoved(p));
    for pressed in [true, false] {
        h.event(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
    }
    h.run_steps(3);
}

fn close_menus(h: &mut H) {
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    assert!(!egui::Popup::is_any_open(&h.ctx), "Menü geschlossen");
}

fn menu_buttons(h: &H) -> Vec<Rect> {
    let label = fp_i18n::t("ui-dock-panel-menu");
    let mut v: Vec<Rect> = h.query_all(By::new().label(&label)).map(|n| n.rect()).collect();
    v.sort_by(|a, b| (a.top(), a.left()).partial_cmp(&(b.top(), b.left())).unwrap_or(std::cmp::Ordering::Equal));
    v
}

/// Waagrechte Linien des letzten Bilds: (y, x0, x1).
fn hlines(h: &H) -> Vec<(f32, f32, f32)> {
    fn walk(s: &egui::Shape, out: &mut Vec<(f32, f32, f32)>) {
        match s {
            egui::Shape::LineSegment { points, .. } if (points[0].y - points[1].y).abs() < 0.01 && (points[1].x - points[0].x).abs() > 4.0 => {
                out.push((points[0].y, points[0].x.min(points[1].x), points[0].x.max(points[1].x)))
            }
            egui::Shape::Vec(v) => v.iter().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let mut v = Vec::new();
    h.output().shapes.iter().for_each(|s| walk(&s.shape, &mut v));
    v
}

/// Öffnet das k-te ≡-Menü; liefert die Trennlinien (nur im offenen Menü gezeichnete Linien).
fn open_menu(h: &mut H, k: usize) -> Option<Vec<(f32, f32, f32)>> {
    let before = hlines(h);
    let b = *menu_buttons(h).get(k)?;
    click(h, b.center());
    if !egui::Popup::is_any_open(&h.ctx) {
        return None;
    }
    let mut lines: Vec<(f32, f32, f32)> = hlines(h).into_iter().filter(|l| !before.contains(l)).collect();
    lines.sort_by(|a, b| a.0.total_cmp(&b.0));
    lines.dedup_by(|a, b| (a.0 - b.0).abs() < 0.5);
    Some(lines)
}

fn state_key(h: &H) -> (usize, u64, String) {
    let s = &h.state().state;
    let d = s.session.active().expect("Dokument");
    (d.history.undo_count(), d.generation, format!("{:?}", s.settings.workspace))
}

fn item_h(h: &H) -> f32 {
    fp_ui::theme::Theme::current(&h.ctx).metrics.menu_item_height
}

#[test]
fn panel_menu_separator_clicks_do_nothing() {
    fp_engine::shell::disable();
    fp_i18n::set_language("de");
    let mut h = harness();
    h.run_steps(4);
    let n_menus = menu_buttons(&h).len();
    assert!(n_menus > 0, "≡-Knöpfe gefunden");
    let half = fp_ui::menu_scroll::SEPARATOR_H / 2.0;
    let mut checked = 0;
    for k in 0..n_menus {
        let Some(lines) = open_menu(&mut h, k) else { continue };
        close_menus(&mut h);
        for (gi, &(y, x0, x1)) in lines.iter().enumerate() {
            // auf der Linie, knapp darüber/darunter und 1 px innerhalb der Trennerzeile; mittig,
            // links in der Häkchenspalte und rechts hinter dem Linienende
            let ys = [y, y - 1.0, y + 1.0, y - 3.0, y + 3.0, y - half + 1.0, y + half - 1.0];
            for x in [(x0 + x1) / 2.0, x0 - 10.0, x1 + 4.0] {
                for yy in ys {
                    open_menu(&mut h, k).expect("Menü öffnet erneut");
                    let before = state_key(&h);
                    click(&mut h, Pos2::new(x, yy));
                    assert!(egui::Popup::is_any_open(&h.ctx), "Menü {k}, Trenner {gi}: Klick ({x:.1}, {yy:.1}) schließt das Menü");
                    assert_eq!(state_key(&h), before, "Menü {k}, Trenner {gi}: Klick ({x:.1}, {yy:.1}) hat etwas ausgeführt");
                    close_menus(&mut h);
                    checked += 1;
                }
            }
        }
    }
    assert!(checked > 0, "mindestens eine Trennlinie geprüft");
}

/// Gegenprobe: Klick mitten in einen Eintrag unter einer Trennlinie führt ihn aus (Menü zu).
#[test]
fn panel_menu_item_below_separator_still_works() {
    fp_engine::shell::disable();
    fp_i18n::set_language("de");
    let mut h = harness();
    h.run_steps(4);
    let ih = item_h(&h);
    let half = fp_ui::menu_scroll::SEPARATOR_H / 2.0;
    for k in 0..menu_buttons(&h).len() {
        let Some(lines) = open_menu(&mut h, k) else { continue };
        close_menus(&mut h);
        for &(y, x0, x1) in &lines {
            open_menu(&mut h, k).expect("Menü öffnet erneut");
            click(&mut h, Pos2::new((x0 + x1) / 2.0, y + half + ih / 2.0));
            if !egui::Popup::is_any_open(&h.ctx) {
                return; // Eintrag ausgeführt, Menü zu
            }
            // grauer Eintrag oder Untermenü: nächster Trenner
            h.key_press(egui::Key::Escape);
            h.run_steps(2);
            h.key_press(egui::Key::Escape);
            h.run_steps(2);
        }
    }
    panic!("kein Eintrag unter einer Trennlinie ließ sich ausführen");
}

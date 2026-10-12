//! Bildlaufleiste der aufgeklappten Schriftliste (NZ-2, Nutzerbefund N24): Die Liste mit Suchfeld
//! (FV-12, `inspector::fonts::family_combo`, benutzt im Text-Inspektor, im Textwerkzeug und im
//! Texteditor) zeigt rechts eine sichtbare Leiste, sobald sie länger ist als der sichtbare Teil;
//! der Griff hebt sich in allen drei Designs von Spur und Menügrund ab, lässt sich mit der Maus
//! ziehen, und gezeichnet werden weiter nur die sichtbaren Zeilen.
//!
//! Erfundene Namen („Testschrift 000“ …) statt der Systemschriften: Die echten Namen hängen vom
//! Rechner ab und enthalten teils den gesperrten Herstellernamen (§1a, FV-12). Realistische
//! Eingaben wie egui-winit (je Ereignis ein Bild, Ziehen über viele Bilder).

use egui::epaint::Shape;
use egui::{pos2, vec2, Color32, Event, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::Harness;
use fp_ui::inspector::fonts;
use fp_ui::theme::{Theme, ThemeKind};
use std::sync::Arc;

const N: usize = 300;
const FIELD: Rect = Rect { min: pos2(20.0, 20.0), max: pos2(220.0, 40.0) };

struct App {
    names: Vec<String>,
    selected: Option<usize>,
}

type H = Harness<'static, App>;

fn harness(kind: ThemeKind) -> H {
    let app = App { names: (0..N).map(|i| format!("Testschrift {i:03}")).collect(), selected: Some(0) };
    let mut h = Harness::builder().with_size(vec2(420.0, 720.0)).with_pixels_per_point(1.0).with_max_steps(8).with_step_dt(1.0 / 60.0).build_ui_state(
        |ui, app: &mut App| {
            let names: Vec<&str> = app.names.iter().map(String::as_str).collect();
            if let Some(i) = fonts::family_combo(ui, "nz2-font-list", FIELD, &names, app.selected, true) {
                app.selected = Some(i);
            }
        },
        app,
    );
    fp_ui::theme::install_fonts(&h.ctx);
    Arc::new(Theme::new(kind)).apply(&h.ctx);
    h.run_steps(2);
    h
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn press(h: &mut H, p: Pos2, down: bool) {
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: down, modifiers: Modifiers::NONE });
}

fn open(h: &mut H) {
    ev(h, Event::PointerMoved(FIELD.center()));
    press(h, FIELD.center(), true);
    press(h, FIELD.center(), false);
    h.run_steps(2);
}

/// Alle gezeichneten Formen (flach).
fn shapes(h: &H) -> Vec<Shape> {
    fn walk(s: &Shape, out: &mut Vec<Shape>) {
        match s {
            Shape::Vec(v) => v.iter().for_each(|x| walk(x, out)),
            other => out.push(other.clone()),
        }
    }
    let mut v = Vec::new();
    for cs in &h.output().shapes {
        walk(&cs.shape, &mut v);
    }
    v
}

/// Gezeichnete Zeilen der Liste (Text, oben) von oben nach unten.
fn rows(h: &H) -> Vec<(usize, f32)> {
    let mut v: Vec<(usize, f32)> = shapes(h)
        .iter()
        .filter_map(|s| match s {
            Shape::Text(t) => t.galley.text().strip_prefix("Testschrift ").and_then(|n| n.parse().ok()).map(|n| (n, t.pos.y)),
            _ => None,
        })
        .filter(|(_, y)| *y > FIELD.bottom())
        .collect();
    v.sort_by(|a, b| a.1.total_cmp(&b.1));
    v
}

/// Gefüllte Rechtecke einer Farbe rechts vom Feld-Mittelpunkt, unterhalb des Felds.
fn rects_of(h: &H, fill: Color32) -> Vec<Rect> {
    shapes(h)
        .iter()
        .filter_map(|s| match s {
            Shape::Rect(r) if r.fill == fill && r.rect.left() > FIELD.center().x && r.rect.top() > FIELD.bottom() => Some(r.rect),
            _ => None,
        })
        .collect()
}

/// Spur und Griff der Leiste.
fn bar(h: &H, kind: ThemeKind) -> (Rect, Rect) {
    let c = Theme::new(kind).colors;
    let track = rects_of(h, c.scrollbar_bg).into_iter().filter(|r| r.height() > 100.0 && r.width() <= 12.0).max_by(|a, b| a.height().total_cmp(&b.height())).unwrap_or_else(|| panic!("{kind:?}: keine Spur der Bildlaufleiste"));
    // Griff: ruhend `scrollbar_thumb`, unter der Maus bzw. beim Ziehen etwas kräftiger
    let looks = [c.scrollbar_thumb, fp_ui::theme::mix(c.scrollbar_thumb, c.text, 0.3), fp_ui::theme::mix(c.scrollbar_thumb, c.text, 0.5)];
    let thumb = shapes(h)
        .iter()
        .filter_map(|s| match s {
            Shape::Rect(r) if looks.contains(&r.fill) && track.expand(0.5).contains_rect(r.rect) => Some(r.rect),
            _ => None,
        })
        .next()
        .unwrap_or_else(|| panic!("{kind:?}: kein Griff in der Spur {track:?}"));
    (track, thumb)
}

fn contrast(a: Color32, b: Color32) -> i32 {
    (a.r() as i32 - b.r() as i32).abs() + (a.g() as i32 - b.g() as i32).abs() + (a.b() as i32 - b.b() as i32).abs()
}

/// Die Leiste ist da, der Griff ist kürzer als die Spur (die Liste geht weiter) und hebt sich in
/// allen drei Designs von Spur und Menügrund ab; nur die sichtbaren Zeilen werden gezeichnet.
#[test]
fn scrollbar_visible_in_all_themes_and_only_visible_rows_drawn() {
    for kind in [ThemeKind::Normal, ThemeKind::Dark, ThemeKind::Oled] {
        let mut h = harness(kind);
        open(&mut h);
        let r = rows(&h);
        assert!(!r.is_empty(), "{kind:?}: Liste offen");
        assert_eq!(r[0].0, 0, "{kind:?}: oben die erste Schrift");
        assert!(r.len() <= 22 && r.len() < N, "{kind:?}: nur sichtbare Zeilen gezeichnet ({} von {N})", r.len());
        let (track, thumb) = bar(&h, kind);
        assert!(thumb.height() < track.height() * 0.2, "{kind:?}: Griff {thumb:?} zeigt, dass die Liste weitergeht (Spur {track:?})");
        assert!((thumb.top() - track.top()).abs() < 2.0, "{kind:?}: Griff oben");
        assert!(thumb.width() >= fonts::LIST_BAR_W - 0.5, "{kind:?}: Griff gut zu greifen ({thumb:?})");
        let c = Theme::new(kind).colors;
        assert!(contrast(c.scrollbar_thumb, c.scrollbar_bg) >= 60, "{kind:?}: Griff hebt sich von der Spur ab");
        assert!(contrast(c.scrollbar_thumb, c.menu_bg) >= 60, "{kind:?}: Griff hebt sich vom Menügrund ab");
    }
}

/// Ziehen am Griff (über viele Bilder) blättert die Liste; die Liste bleibt offen, die Auswahl
/// unverändert, und auch nach dem Ziehen sind nur die sichtbaren Zeilen gezeichnet. Mausrad geht
/// weiter.
#[test]
fn scrollbar_thumb_can_be_dragged() {
    let mut h = harness(ThemeKind::Normal);
    open(&mut h);
    let (track, thumb) = bar(&h, ThemeKind::Normal);
    let start = thumb.center();
    ev(&mut h, Event::PointerMoved(start));
    press(&mut h, start, true);
    let dy = track.height() * 0.5;
    for k in 1..=20 {
        ev(&mut h, Event::PointerMoved(start + vec2(0.0, dy * k as f32 / 20.0)));
    }
    press(&mut h, start + vec2(0.0, dy), false);
    h.run_steps(2);
    let r = rows(&h);
    assert!(!r.is_empty(), "Liste nach dem Ziehen noch offen");
    let first = r[0].0;
    assert!((100..=200).contains(&first), "halber Weg der Spur ≈ Mitte der Liste, oben steht {first}");
    assert!(r.len() <= 22, "nach dem Ziehen nur sichtbare Zeilen ({})", r.len());
    assert_eq!(h.state().selected, Some(0), "Ziehen wählt nichts aus");
    let (_, thumb2) = bar(&h, ThemeKind::Normal);
    assert!(thumb2.center().y > thumb.center().y + dy * 0.8, "Griff folgt der Maus ({thumb:?} → {thumb2:?})");
    // Mausrad (eine Rastung = 3 Zeilen wie unter Windows) blättert zurück
    let over = pos2(FIELD.left() + 40.0, r[3].1);
    ev(&mut h, Event::PointerMoved(over));
    ev(&mut h, Event::MouseWheel { unit: egui::MouseWheelUnit::Line, delta: vec2(0.0, 3.0), modifiers: Modifiers::NONE, phase: egui::TouchPhase::Move });
    h.run_steps(4);
    let r2 = rows(&h);
    assert!(r2[0].0 < first, "Mausrad nach oben: {} → {}", first, r2[0].0);
}

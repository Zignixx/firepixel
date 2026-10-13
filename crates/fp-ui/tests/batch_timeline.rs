//! Zeitleiste der Batch-Vorschau (BATCH-7, `fp-ui\src\batch\timeline.rs`): Griffe ziehen über viele
//! Frames, Frame anklicken, Zurücksetzen, Anbindung ans Rezept (ein Schritt `anim.trim`, eine Geste =
//! ein Rückgängig-Schritt, Rückgängig zeigt den alten Stand), schmale Breite und Englisch.

use egui::{Event, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use fp_batch::item::{Animation, Frame, Item, Meta};
use fp_batch::steps::anim::{Timeline, TRIM};
use fp_raster::{IRect, RgbaImage};
use fp_ui::batch::timeline::{self, TimelineResponse};
use fp_ui::batch::{Mode, Studio};
use std::path::PathBuf;
use std::time::{Duration, Instant};

struct W {
    tl: Timeline,
    last: TimelineResponse,
    changes: usize,
}

fn ev<S>(h: &mut Harness<'static, S>, e: Event) {
    h.input_mut().events.push(e);
    h.step();
}

fn press<S>(h: &mut Harness<'static, S>, p: Pos2, pressed: bool) {
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
}

/// Zieht in `steps` Mausbewegungen von `a` nach `b`.
fn drag<S>(h: &mut Harness<'static, S>, a: Pos2, b: Pos2, steps: usize) {
    ev(h, Event::PointerMoved(a));
    press(h, a, true);
    for k in 1..=steps {
        let t = k as f32 / steps as f32;
        ev(h, Event::PointerMoved(Pos2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)));
    }
    press(h, b, false);
}

fn click<S>(h: &mut Harness<'static, S>, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    press(h, p, true);
    press(h, p, false);
}

#[test]
fn widget_drag_click_and_reset() {
    let s = W { tl: Timeline::new(vec![100; 10]), last: TimelineResponse::default(), changes: 0 };
    let mut h = Harness::builder().with_size(egui::vec2(420.0, 120.0)).with_pixels_per_point(1.0).build_ui_state(
        |ui, s: &mut W| {
            s.last = timeline::timeline(ui, &mut s.tl, Some(2));
            if s.last.changed {
                s.changes += 1;
            }
        },
        s,
    );
    h.run();
    let _ = h.get_by_label_contains("10 von 10");
    // Band: zweite Zeile; links/rechts je Fensterrand 8 px + halber Fangbereich
    let (y, left, right) = (40.0, 12.0, 408.0);
    let w = right - left;
    // Anfang auf 30 % ziehen, 30 Bewegungen (jede ein Ereignis) → gerastet auf 300 ms
    drag(&mut h, Pos2::new(left, y), Pos2::new(left + w * 0.3, y), 30);
    h.run();
    assert_eq!(h.state().tl.start_ms, 300, "{:?}", h.state().tl);
    assert!(h.state().changes >= 1);
    // Ende von rechts auf 70 %
    drag(&mut h, Pos2::new(right, y), Pos2::new(left + w * 0.7, y), 12);
    h.run();
    assert_eq!(h.state().tl.end_ms, 700);
    // Bild 5 anklicken → entfernt, noch einmal → zurück
    let p = Pos2::new(left + w * 0.45, y);
    click(&mut h, p);
    h.run();
    assert_eq!(h.state().tl.removed.iter().position(|&x| x), Some(4));
    assert_eq!(h.state().tl.kept(), (3, 300));
    click(&mut h, p);
    h.run();
    assert!(h.state().tl.removed.iter().all(|&x| !x));
    // Zurücksetzen
    h.get_by_label("Zurücksetzen").click();
    h.run();
    assert_eq!(h.state().tl, Timeline::new(vec![100; 10]));
}

#[test]
fn widget_narrow_english_and_zero_delays_do_not_panic() {
    fp_i18n::set_language("en");
    let mut h = Harness::builder().with_size(egui::vec2(120.0, 90.0)).build_ui_state(
        |ui, s: &mut W| {
            timeline::timeline(ui, &mut s.tl, None);
        },
        W { tl: Timeline::new(vec![0, 0, 50]), last: Default::default(), changes: 0 },
    );
    h.run();
    let _ = h.get_by_label_contains("of 3 frames");
    // leer: zeichnet nichts
    let mut h = Harness::builder().with_size(egui::vec2(200.0, 60.0)).build_ui_state(
        |ui, s: &mut W| {
            s.last = timeline::timeline(ui, &mut s.tl, None);
        },
        W { tl: Timeline::new(vec![]), last: Default::default(), changes: 0 },
    );
    h.run();
    assert_eq!(h.state().last, TimelineResponse::default());
    fp_i18n::set_language("de");
}

fn gif(dir: &std::path::Path, n: usize) -> PathBuf {
    let frames = (0..n)
        .map(|i| {
            let px = [(i * 20) as u8, 60, 120, 255];
            let bytes: Vec<u8> = (0..16 * 12).flat_map(|_| px).collect();
            Frame { image: RgbaImage::from_rgba_bytes(IRect::new(0, 0, 16, 12), &bytes).expect("img"), delay_ms: 100 }
        })
        .collect();
    let it = Item::Animation(Animation { width: 16, height: 12, frames, loop_count: 0, meta: Meta::default() });
    let (bytes, _) = fp_batch::output::encode(&it, fp_batch::OutputFormat::Gif, &Default::default()).expect("gif");
    let p = dir.join("walk.gif");
    std::fs::write(&p, bytes).expect("write");
    p
}

#[test]
fn preview_timeline_writes_one_trim_step_per_gesture() {
    let dir = std::env::temp_dir().join(format!("fp-ui-batch-timeline-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    let src = gif(&dir, 10);
    let mut studio = Studio::new(Mode::Pro);
    studio.add_paths(&[src], false);
    studio.recipe.output.format = "gif".into();
    let mut h = Harness::builder().with_size(egui::vec2(420.0, 140.0)).with_pixels_per_point(1.0).build_ui_state(
        |ui, s: &mut Studio| {
            timeline::update(ui.ctx(), s);
            if s.preview.timeline.visible() {
                let r = Rect::from_min_size(Pos2::new(0.0, 0.0), egui::vec2(420.0, timeline::AREA_HEIGHT));
                timeline::show_in_preview(ui, s, r);
            }
        },
        studio,
    );
    // Dauern kommen aus dem Hintergrund
    let t0 = Instant::now();
    while !h.state().preview.timeline.visible() && t0.elapsed() < Duration::from_secs(20) {
        std::thread::sleep(Duration::from_millis(20));
        h.step();
    }
    assert!(h.state().preview.timeline.visible(), "Zeitleiste erscheint für GIF");
    assert!(h.state().recipe.steps.is_empty());
    let (y, left, right) = (40.0, 12.0, 408.0);
    let w = right - left;
    drag(&mut h, Pos2::new(left, y), Pos2::new(left + w * 0.2, y), 25);
    h.run();
    let steps: Vec<_> = h.state().recipe.steps.iter().filter(|s| s.id == TRIM).collect();
    assert_eq!(steps.len(), 1, "genau ein Schritt anim.trim");
    assert_eq!(h.state().recipe.steps[0].id, TRIM, "vorn eingefügt");
    assert_eq!(h.state().recipe.steps[0].params["start"], 200);
    // zweite Geste: Bild 8 entfernen; dann Rückgängig nimmt nur das zurück
    click(&mut h, Pos2::new(left + w * 0.75, y));
    h.run();
    assert_eq!(h.state().recipe.steps[0].params["removeFrames"], "8");
    assert_eq!(h.state().recipe.steps.len(), 1);
    h.state_mut().undo();
    h.run();
    assert_eq!(h.state().recipe.steps[0].params["removeFrames"], "");
    assert_eq!(h.state().preview.timeline.model.as_ref().map(|m| m.removed.iter().any(|&x| x)), Some(false), "Zeitleiste folgt dem Rückgängig");
    h.state_mut().undo();
    h.run();
    assert!(h.state().recipe.steps.is_empty(), "ein Zug = ein Rückgängig-Schritt");
    assert_eq!(h.state().preview.timeline.model.as_ref().map(|m| m.start_ms), Some(0));
    let _ = std::fs::remove_dir_all(&dir);
}

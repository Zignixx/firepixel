//! T-2: Zeigergeschwindigkeit als Eingang der Pinsel-Empfindlichkeiten (re\10 §2.2: Weg/Zeit über
//! die letzten ≈ 55 px). Fenster, Zoom, Wirkung auf den Strich, Vorschau = Ergebnis, Befehl trägt
//! die Geschwindigkeiten; Züge ohne Zeitstempel bleiben wie bisher.

use crate::common;
use common::*;
use fp_engine::paint::{pointer_velocity, PAINT_VELOCITY};
use fp_engine::tools::ToolId;
use fp_engine::{Overlay, Session, ToolEvent};
use fp_geom::Point;
use fp_model::{ElementId, ElementKind, Sensitivity};
use serde_json::json;

#[test]
fn velocity_window_and_zoom() {
    // 10 px je 10 ms = 1 px/ms ⇒ VELf 1,0 (px/ms, S-6 nach M6 §2a)
    let pts: Vec<Point> = (0..20).map(|i| Point::new(i as f32 * 10.0, 0.0)).collect();
    let t: Vec<u64> = (0..20).map(|i| 1000 + i * 10).collect();
    assert_eq!(pointer_velocity(&pts, &t, 1.0), Some(1.0));
    // Zoom 200 %: doppelter Bildschirmweg
    assert_eq!(pointer_velocity(&pts, &t, 2.0), Some(2.0));
    // nur die letzten ≈ 55 px zählen: langsamer Anfang ändert nichts
    // erste 10 Punkte langsam (500 ms je Schritt), danach 10 ms je 10 px
    let t2: Vec<u64> = (0..20u64).map(|i| if i < 10 { 1000 + i * 500 } else { 10_000 + i * 10 }).collect();
    assert_eq!(pointer_velocity(&pts, &t2, 1.0), Some(1.0));
    // keine Zeit vergangen / ein Punkt ⇒ keine Angabe; nach oben unbegrenzt
    assert_eq!(pointer_velocity(&pts, &[5; 20], 1.0), None);
    assert_eq!(pointer_velocity(&pts[..1], &t[..1], 1.0), None);
    let fast: Vec<u64> = (0..20).map(|i| 1000 + i).collect();
    assert_eq!(pointer_velocity(&pts, &fast, 1.0), Some(10.0));
}

/// Pinsel mit Empfindlichkeit Geschwindigkeit → Größe (natürliche Engine, Kohlestift/Weich).
fn speed_brush() -> fp_model::Brush {
    let mut b = fp_engine::commands::panel::tool_brush_from(Some(&json!({"preset": "Kohlestift/Weich", "width": 20})), None);
    b.diameter = 20;
    b.sensitivities.retain(|s| s.source != b'S');
    b.sensitivities.push(Sensitivity { source: b'S', target: b'Z', value: 1000 });
    b
}

fn painted(s: &Session, id: ElementId) -> usize {
    match &el(s, id).kind {
        ElementKind::Bitmap(b) => fp_engine::docops::bitmap_image(b).expect("Bild").data().iter().filter(|p| p[3] > 0).count(),
        _ => panic!("keine Bitmap"),
    }
}

fn paint_cmd(velocities: Option<f32>) -> usize {
    let mut s = doc_session(400.0, 200.0);
    let id = bitmap(&mut s, 0, 0, 400, 200, [0, 0, 0, 0]);
    select(&mut s, &[id]);
    let pts: Vec<[f32; 2]> = (0..30).map(|i| [40.0 + i as f32 * 10.0, 100.0]).collect();
    let mut p = json!({"points": pts, "brush": speed_brush(), "color": "#000000ff", "seed": 99});
    if let Some(v) = velocities {
        p["velocities"] = json!(vec![v; pts.len()]);
    }
    ex(&mut s, "bitmap.paint", p);
    painted(&s, id)
}

#[test]
fn velocity_changes_speed_sensitive_stroke() {
    let default = paint_cmd(None);
    assert_eq!(default, paint_cmd(Some(PAINT_VELOCITY)), "ohne Angabe = Vorgabe 0,125");
    let slow = paint_cmd(Some(0.0));
    let fast = paint_cmd(Some(1.0));
    assert!(slow > 0 && fast > 0);
    assert_ne!(slow, fast, "Geschwindigkeit wirkt über die Empfindlichkeit auf die Größe");
}

/// Echter Zug über das Werkzeug mit Zeitstempeln: der Befehl trägt die Geschwindigkeiten, die
/// Vorschau entspricht dem Ergebnis; schneller Zug ≠ langsamer Zug.
fn tool_stroke(ms_per_step: u64) -> (usize, Vec<f64>, usize) {
    let mut s = doc_session(400.0, 200.0);
    let id = bitmap(&mut s, 0, 0, 400, 200, [0, 0, 0, 0]);
    select(&mut s, &[id]);
    s.set_tool(ToolId::Brush).unwrap();
    s.tools.options.brush_size = 20.0;
    s.tools.options.new_stroke = None;
    s.tools.options.new_brush = Some(serde_json::to_value(speed_brush()).unwrap());
    let at = |ev: ToolEvent, t: u64| ToolEvent { time_ms: t, ..ev };
    let mut t = 5_000u64;
    events(&mut s, &[at(ToolEvent::down(40.0, 100.0), t)]);
    let mut preview_px = 0;
    for i in 1..=60 {
        t += ms_per_step;
        events(&mut s, &[at(ToolEvent::drag(40.0 + i as f32 * 5.0, 100.0), t)]);
        if let Some(Overlay::Paint(p)) = s.tool_feedback().into_iter().find(|o| matches!(o, Overlay::Paint(_))) {
            preview_px = p.tiles.iter().map(|tl| tl.image.data().iter().filter(|q| q[3] > 0).count()).sum();
        }
    }
    let resp = s.tool_event(&at(ToolEvent::up(340.0, 100.0), t + ms_per_step)).unwrap();
    let vels: Vec<f64> = resp.commands.first().and_then(|c| c.params["velocities"].as_array().cloned()).unwrap_or_default().iter().filter_map(|v| v.as_f64()).collect();
    (painted(&s, id), vels, preview_px)
}

#[test]
fn tool_stroke_carries_pointer_velocity() {
    let (slow_px, slow_v, slow_prev) = tool_stroke(40);
    let (fast_px, fast_v, fast_prev) = tool_stroke(10);
    assert_eq!(slow_v.len(), 61, "eine Geschwindigkeit je Punkt");
    assert_eq!(slow_v[0], 0.0, "Start aus der Ruhe");
    // 5 px je 40 ms = 0,125 px/ms; 5 px je 10 ms = 0,5 px/ms (VELf = px/ms)
    assert!((slow_v[30] - 0.125).abs() < 1e-6, "{}", slow_v[30]);
    assert!((fast_v[30] - 0.5).abs() < 1e-6, "{}", fast_v[30]);
    assert_eq!(slow_prev, slow_px, "Vorschau = Ergebnis (langsam)");
    assert_eq!(fast_prev, fast_px, "Vorschau = Ergebnis (schnell)");
    assert_ne!(slow_px, fast_px, "Empfindlichkeit Geschwindigkeit wirkt mit der Maus");
}

/// S-6 (re\33 §2a): `VELf` in px/ms auf 1/256 gerundet – Messzüge des Originals 0,18 px/ms → 47/256,
/// 0,574 px/ms → 147/256, 2,285 px/ms → 585/256.
#[test]
fn velocity_is_px_per_ms_in_256ths() {
    let pts: Vec<Point> = (0..30).map(|i| Point::new(i as f32 * 20.0, 0.0)).collect();
    // 60 px in 327 ms ≈ 0,1835 px/ms
    let t: Vec<u64> = (0..30u64).map(|i| i * 109).collect();
    assert_eq!(pointer_velocity(&pts, &t, 1.0), Some(47.0 / 256.0));
    // 60 px in 26 ms = 2,3077 px/ms ⇒ 591/256 (unbegrenzt)
    let t: Vec<u64> = (0..30u64).map(|i| 1000 + i * 26 / 3).collect();
    let v = pointer_velocity(&pts, &t, 1.0).unwrap();
    assert!(v > 2.0 && (v * 256.0).fract() == 0.0, "{v}");
}

//! FIX-D: Befunde aus dem Klicktest CT-2 (`docs\funktionstest\ct2_2026-10-13.md`).

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent};
use fp_model::ElementKind;

fn pixel(s: &Session, id: fp_model::ElementId, x: i32, y: i32) -> [u8; 4] {
    let e = el(s, id);
    let ElementKind::Bitmap(b) = &e.kind else { panic!("keine Bitmap") };
    let img = fp_engine::docops::bitmap_image(b).expect("Bild");
    img.get(x, y).expect("Pixel")
}

fn drag(s: &mut Session, a: (f32, f32), b: (f32, f32)) {
    events(s, &[ToolEvent::down(a.0, a.1)]);
    for i in 1..=30 {
        let t = i as f32 / 30.0;
        events(s, &[ToolEvent::drag(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)]);
    }
    events(s, &[ToolEvent::up(b.0, b.1)]);
}

/// CT-2 W8: Farbe ersetzen (Größe 13, Toleranz 32, Stärke 200, Grün → Blau) auf grüner Bitmap –
/// beim Ziehen wird der ganze Strich blau, nicht nur der erste Stempel.
#[test]
fn replace_color_drag_replaces_whole_stroke() {
    use fp_engine::prefs::PrefValue::{Bool, Int, Str};
    let mut s = doc_session(300.0, 200.0);
    let b = bitmap(&mut s, 0, 0, 300, 200, [0, 255, 0, 255]);
    select(&mut s, &[b]);
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
    drag(&mut s, (40.0, 100.0), (240.0, 100.0));
    for x in [40, 80, 140, 200, 239] {
        let p = pixel(&s, b, x, 100);
        assert!(p[1] < 10 && p[2] > 240, "x {x}: {p:?} statt blau");
    }
}

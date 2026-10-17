//! T-7 (FP1-06): Schneller Pinselzug mit Zeitstempeln, Kohlestift „Weich“ (Größe ← Geschwindigkeit):
//! die letzte Live-Vorschau zeigt dieselben Pixel wie das Ergebnis, auch am Zuganfang (erster
//! Punkt eines Zeigerzugs mit Geschwindigkeit 0, re\33 §2a / re 34 §2.2).

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Overlay, ToolEvent};
use fp_model::ElementKind;
use serde_json::json;

fn at(mut e: ToolEvent, t: u64) -> ToolEvent {
    e.time_ms = t;
    e
}

#[test]
fn fast_stroke_preview_equals_result() {
    for step_ms in [1u64, 8] {
        let mut s = doc_session(400.0, 300.0);
        s.set_colors(Some(fp_color::Argb(0xff00_0000)), None);
        s.set_tool(ToolId::Brush).unwrap();
        s.tools.options.brush_size = 35.0;
        s.tools.options.new_stroke = Some(json!({"preset": "Kohlestift/Weich", "width": 35, "softness": 37, "placement": "center"}));
        s.tools.options.new_brush = None;
        let mut t = 1000u64;
        events(&mut s, &[at(ToolEvent::down(40.0, 150.0), t)]);
        let mut last = None;
        let mut x = 40.0f32;
        while x < 360.0 {
            x += 6.0;
            t += step_ms;
            events(&mut s, &[at(ToolEvent::drag(x, 150.0), t)]);
            if let Some(Overlay::Paint(p)) = s.tool_feedback().into_iter().find(|o| matches!(o, Overlay::Paint(_))) {
                last = Some(p);
            }
        }
        events(&mut s, &[at(ToolEvent::up(x, 150.0), t + step_ms)]);
        let p = last.expect("Vorschau");
        let el = top_elements(&s)[0].clone();
        let ElementKind::Bitmap(b) = &el.kind else { panic!("keine Bitmap") };
        let img = fp_engine::docops::bitmap_image(b).expect("Bild");
        let r = img.bounds();
        let (mut diff, mut painted, mut first_diff) = (0, 0, None);
        for y in r.top..r.bottom {
            for x in r.left..r.right {
                let px = img.get(x, y).unwrap_or([0; 4]);
                if px[3] > 0 {
                    painted += 1;
                    let shown = p.tiles.iter().find(|t| t.rect.contains(x, y)).and_then(|t| t.image.get(x, y)).unwrap_or([0; 4]);
                    if shown != px {
                        diff += 1;
                        first_diff.get_or_insert((x, y));
                    }
                }
            }
        }
        assert!(painted > 1000, "{painted}");
        assert_eq!(diff, 0, "{step_ms} ms/6 px: Vorschau weicht in {diff} von {painted} Pixeln ab, zuerst bei {first_diff:?}");
    }
}

//! B-5 (BA-070): `effect.live` rechnet Abstand/Weichheit in der Vorschau mit dem Proxy-Maßstab aus
//! `StepContext::preview_scale` um, damit der Schatten auf dem Proxy so groß wirkt wie im Ergebnis.

mod batch4_common;

use batch4_common::*;
use fp_batch::{CancelToken, OutputFormat, StepContext, StepRegistry};
use serde_json::json;

/// Breite und Höhe nach `effect.live` (Schlagschatten, Fläche wächst mit Abstand + Weichheit).
fn grown(w: u32, h: u32, preview: bool, proxy_scale: Option<f64>) -> (u32, u32) {
    let obj = image(w, h, |x, y| if x >= 2 && x < w - 2 && y >= 2 && y < h - 2 { [200, 30, 30, 255] } else { [0, 0, 0, 0] });
    let mut it = still(obj);
    let reg = StepRegistry::global();
    let step = reg.get("effect.live").expect("effect.live");
    let p = json!({"effect": "dropShadow", "params": {"ShadowDistance": "20", "ShadowBlur": "8", "ShadowAngle": "315", "ShadowColor": "#000000ff"}, "expandCanvas": true});
    // Kopf-Info = Originalgröße 4× so groß wie das Bild, damit die Schätzung ohne Proxy-Maßstab
    // nicht zufällig dasselbe ergibt
    let inf = info(w * 4, h * 4);
    let cancel = CancelToken::new();
    let mut ctx = StepContext::new(0, 1, &inf, &cancel, OutputFormat::Png);
    ctx.preview = preview;
    ctx.proxy_scale = proxy_scale;
    step.apply(&mut it, &p, &mut ctx).expect("effect.live");
    let r = first(&it);
    (r.width() as u32, r.height() as u32)
}

#[test]
fn preview_scales_distance_and_softness_with_proxy_scale() {
    let (fw, fh) = grown(60, 40, false, None);
    let (hw, hh) = grown(60, 40, true, Some(0.5));
    // Lauf: Fläche wächst um Abstand + Weichheit; Vorschau mit Maßstab 0,5 etwa um die Hälfte
    let (gf, gh) = (fw - 60, hw - 60);
    assert!(gf > 10, "Schatten im Lauf sichtbar: {fw}x{fh}");
    assert!(gh * 2 <= gf + 2 && gh * 2 + 2 >= gf, "Vorschau halb so weit: Lauf +{gf}, Vorschau +{gh}");
    assert!(hh - 40 < fh - 40, "auch senkrecht kleiner");
    // Maßstab 1 in der Vorschau = Ergebnis
    assert_eq!(grown(60, 40, true, Some(1.0)), (fw, fh));
    // Maßstab vom Kontext hat Vorrang vor der Schätzung aus der Kopf-Info (dort 0,25)
    let (qw, _) = grown(60, 40, true, None);
    assert!(qw - 60 < gh, "Schätzung 0,25 kleiner als Maßstab 0,5: +{} vs +{gh}", qw - 60);
}

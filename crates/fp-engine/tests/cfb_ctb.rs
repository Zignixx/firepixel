//! CF-B: fixes for the click test CT-B (`docs\funktionstest\ct_b_2026-10-15.md`).

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent};
use fp_model::ElementKind;
use serde_json::json;
use std::path::PathBuf;

fn open_par(name: &str) -> Option<Session> {
    let src = PathBuf::from(r"C:\Users\Administrator\Desktop\REA\re\_testdateien\par").join("work").join(name);
    if !src.is_file() {
        eprintln!("skipped: {src:?} missing");
        return None;
    }
    let mut s = doc_session(10.0, 10.0);
    let dir = std::env::temp_dir().join(format!("fp-cfb-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(1);
    let p = dir.join(format!("{}-{name}", N.fetch_add(1, Ordering::Relaxed)));
    std::fs::copy(&src, &p).ok()?;
    ex(&mut s, "file.open", json!({"path": p.to_string_lossy()}));
    if let Some(b) = top_elements(&s).into_iter().find(|e| matches!(e.kind, ElementKind::Bitmap(_))).map(|e| e.id) {
        select(&mut s, &[b]);
    }
    Some(s)
}

fn render(s: &Session) -> fp_raster::RgbaImage {
    let d = s.active().expect("doc");
    fp_engine::render::default_renderer().expect("renderer").render_page(&d.doc, d.page, d.frame).expect("image")
}

fn drag(s: &mut Session, a: (f32, f32), b: (f32, f32)) {
    events(s, &[ToolEvent::down(a.0, a.1)]);
    for i in 1..=30 {
        let t = i as f32 / 30.0;
        events(s, &[ToolEvent::drag(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)]);
    }
    events(s, &[ToolEvent::up(b.0, b.1)]);
}

fn n_changed(a: &fp_raster::RgbaImage, b: &fp_raster::RgbaImage) -> usize {
    a.data().iter().zip(b.data()).filter(|(p, q)| p != q).count()
}

fn red_eye_run(tol: i64, strength: i64, a: (f32, f32), b: (f32, f32)) -> Option<(Session, fp_raster::RgbaImage, fp_raster::RgbaImage)> {
    use fp_engine::prefs::PrefValue::Int;
    let mut s = open_par("p57_t02.fw.png")?;
    let base = render(&s);
    s.prefs.set("RedEyeRemovalToolTolerance", Int(tol));
    s.prefs.set("RedEyeRemovalToolStrength", Int(strength));
    s.set_tool(ToolId::RedEyeRemoval).expect("red eye");
    drag(&mut s, a, b);
    let img = render(&s);
    Some((s, img, base))
}

/// CT-B-05: with the defaults (tolerance 50, strength 50) a rectangle over the eyes darkens the
/// pupils as measured in the original (#DC141E → ≈ #4E1C1E); the strength works steadily
/// (0 nothing, 25 < 50 < 75 < 100 darker and darker), the same pixels change at every strength > 0.
/// A drag without red pixels changes nothing (step "Bitmap bearbeiten" as in the original, 05/15).
#[test]
fn red_eye_strength_is_steady() {
    let rect = ((100.0, 250.0), (225.0, 287.0));
    let mut reds = Vec::new();
    let mut counts = Vec::new();
    for st in [0, 25, 50, 75, 100] {
        let Some((s, img, base)) = red_eye_run(50, st, rect.0, rect.1) else { return };
        assert_eq!(base.get(120, 268).map(|c| [c[0], c[1], c[2]]), Some([220, 20, 30]), "eye in the test file");
        let c = img.get(120, 268).expect("eye");
        reds.push(c[0]);
        counts.push(n_changed(&img, &base));
        let label = s.active().expect("doc").history.undo_label().unwrap_or_default().to_string();
        if st == 0 {
            assert_eq!(counts[0], 0, "strength 0: no change");
            assert_eq!(label, "Bitmap bearbeiten");
        } else {
            assert_eq!(label, "Rote Augen entfernen");
        }
        if st == 50 {
            // original 05/11: centre #4E1C1E
            assert!((c[0] as i32 - 0x4e).abs() <= 4 && (c[1] as i32 - 0x1c).abs() <= 2 && (c[2] as i32 - 0x1e).abs() <= 1, "50/50: {c:?}");
        }
    }
    assert!(reds.windows(2).all(|w| w[1] < w[0]), "steady: {reds:?}");
    assert!(counts[1..].iter().all(|n| (1050..=1180).contains(n)), "{counts:?}");
    // a drag below the eyes (no red pixel inside): no change, but a step like in the original
    let Some((s, img, base)) = red_eye_run(50, 50, (100.0, 290.0), (225.0, 299.0)) else { return };
    assert_eq!(n_changed(&img, &base), 0);
    assert_eq!(s.active().expect("doc").history.undo_label().unwrap_or_default(), "Bitmap bearbeiten");
}

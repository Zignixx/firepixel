//! FIX-R1 (fixg5 Q-02): „Maske ▸ Alles einblenden/ausblenden“ auf einer Bitmap – `pixelRect` wie im
//! Original (re\_testdateien\par\fixg5\fixg5_mask.jsf): Alles einblenden unverändert 40,40–520,360
//! (hier 40,40–140,120), Alles ausblenden das leere Rechteck ±2147483648.

use fp_script::{ScriptOptions, run_source};
use serde_json::json;

fn session_with_bitmap() -> fp_engine::Session {
    fp_engine::shell::disable();
    fp_i18n::profile::use_test_profile();
    let mut s = fp_engine::Session::default();
    s.execute(
        "file.new",
        json!({"width": 400, "height": 300, "background": "#ffffff"}),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let img = fp_raster::RgbaImage::filled(
        fp_geom::IRect::from_xywh(40, 40, 100, 80),
        fp_color::Rgba8::new(200, 30, 30, 255),
    )
    .unwrap_or_else(|e| panic!("{e:?}"));
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    s.execute("edit.paste", json!({}))
        .unwrap_or_else(|e| panic!("{e}"));
    s
}

fn result(s: &mut fp_engine::Session, src: &str) -> String {
    let o = run_source(s, src, &ScriptOptions::default());
    assert!(o.ok, "Fehler: {:?}\nDiagnose: {:?}", o.error, o.diagnostics);
    o.result.unwrap_or_default()
}

const INFO: &str = "var e = fw.selection[0]; var r = e.pixelRect; [String(e), e.mask ? e.mask.mode : 'null', r.left, r.top, r.right, r.bottom].join(',')";

#[test]
fn mask_reveal_all_keeps_pixel_rect() {
    let mut s = session_with_bitmap();
    assert_eq!(result(&mut s, INFO), "[object Image],null,40,40,140,120");
    s.execute("modify.mask.revealAll", json!({}))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        result(&mut s, INFO),
        "[object Image],mask to image,40,40,140,120"
    );
}

#[test]
fn mask_hide_all_reports_empty_pixel_rect() {
    let mut s = session_with_bitmap();
    s.execute("modify.mask.hideAll", json!({}))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        result(&mut s, INFO),
        "[object Image],mask to image,-2147483648,-2147483648,2147483648,2147483648"
    );
}

/// TW-225 rest (RE spec 68 §8): after Reveal All the new mask is edited; scripts see the masked
/// element in `fw.selection` and the mask in `fw.selectedMask` (null without mask editing).
#[test]
fn mask_editing_reports_selected_mask() {
    let mut s = session_with_bitmap();
    let q = "[fw.selection.length, String(fw.selection[0]), fw.selectedMask ? String(fw.selectedMask) : 'null'].join(',')";
    assert_eq!(result(&mut s, q), "1,[object Image],null");
    s.execute("modify.mask.revealAll", json!({}))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(result(&mut s, q), "1,[object Image],[object Image]");
    assert_eq!(result(&mut s, "fw.selection[0].mask.mode"), "mask to image");
}

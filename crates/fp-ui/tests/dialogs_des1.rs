//! DES-1 (N11, `docs\DESIGN.md`): Gestaltung der Dialoge zentral im Dialograhmen â€“ FuÃŸleiste hinter
//! der unteren Knopfzeile, Titelleiste, Rahmen mit Schatten, Standardknopf, Gruppenrahmen,
//! Meldung mit Symbol. Bedienung, Beschriftungen und Anordnung bleiben wie im Original.
//!
//! Bilder vorher/nachher (je Dialog die drei Designs nebeneinander, Normal | Dunkel | OLED):
//!
//! ```text
//! $env:FP_NO_SHELL_OPEN='1'; $env:FP_DES1_STAGE='before'   # oder 'after'
//! cargo test -j 3 -p fp-ui --test all dialogs_des1::des1_images -- --ignored --nocapture
//! ```
//!
//! Ablage: `docs\screenshots\des1\<stage>_<dialog>.png`.

#[allow(unused_macros, unused_imports)]
use crate::i18n_qa_common;

use fp_ui::app_state::AppState;
use fp_ui::theme::ThemeKind;
use i18n_qa_common::{build_scene, repo, select, Cfg, Fix, Scene};
use serde_json::json;

/// Zuschnitt eines RGBA-Bilds (wie `design_bilder`).
struct Img {
    w: u32,
    h: u32,
    px: Vec<u8>,
}

impl Img {
    fn crop((sw, sh, raw): (u32, u32, &[u8]), x0: u32, y0: u32, w: u32, h: u32) -> Img {
        let x0 = x0.min(sw.saturating_sub(1));
        let y0 = y0.min(sh.saturating_sub(1));
        let w = w.min(sw - x0).max(1);
        let h = h.min(sh - y0).max(1);
        let mut px = Vec::with_capacity((w * h * 4) as usize);
        for y in y0..y0 + h {
            let s = ((y * sw + x0) * 4) as usize;
            px.extend_from_slice(&raw[s..s + (w * 4) as usize]);
        }
        Img { w, h, px }
    }

    fn hstack(imgs: &[Img], gap: u32) -> Img {
        let h = imgs.iter().map(|i| i.h).max().unwrap_or(1);
        let w = imgs.iter().map(|i| i.w).sum::<u32>() + gap * imgs.len().saturating_sub(1) as u32;
        let mut px = vec![128u8; (w * h * 4) as usize];
        for a in px.chunks_exact_mut(4) {
            a[3] = 255;
        }
        let mut x = 0;
        for i in imgs {
            for y in 0..i.h {
                let d = ((y * w + x) * 4) as usize;
                let s = (y * i.w * 4) as usize;
                px[d..d + (i.w * 4) as usize].copy_from_slice(&i.px[s..s + (i.w * 4) as usize]);
            }
            x += i.w + gap;
        }
        Img { w, h, px }
    }

    fn save(&self, path: &std::path::Path) -> Result<(), String> {
        let f = std::fs::File::create(path).map_err(|e| e.to_string())?;
        let mut enc = png::Encoder::new(std::io::BufWriter::new(f), self.w, self.h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_compression(png::Compression::High);
        let mut wr = enc.write_header().map_err(|e| e.to_string())?;
        wr.write_image_data(&self.px).map_err(|e| e.to_string())
    }
}

type Prep = fn(&mut AppState, &Fix);

/// Die gezeigten Dialoge: Dateiname (englisch), Dialogkennung, Vorbereitung.
fn cases() -> Vec<(&'static str, &'static str, Prep)> {
    vec![
        ("message", "fp-message", |st, _| st.dialogs.message = Some((fp_i18n::t("dlg-popup-title"), fp_i18n::t("dlg-popup-need-selection")))),
        ("confirm_history_clear", "fp-confirm", |st, _| st.run("panel.history.clear", json!({}))),
        ("unsaved", "fp-unsaved", |st, _| {
            if let Some(id) = st.session.active_id() {
                st.dialogs.unsaved = Some(fp_ui::dialogs::UnsavedFlow::new(vec![id], fp_ui::dialogs::PendingClose::CloseDocs));
            }
        }),
        ("save_workspace", "fp-save-ws", |st, _| st.dialogs.save_workspace = Some("Mein Arbeitsbereich".into())),
        ("image_size", "", |st, _| st.run("modify.canvas.imageSize", json!({}))),
        ("canvas_size", "", |st, _| st.run("modify.canvas.canvasSize", json!({}))),
        ("gaussian_blur", "", |st, f| {
            select(st, &f.bitmap);
            st.run("filter.blur.gaussian", json!({}));
        }),
        ("new_document", "", |st, _| st.run("file.new", json!({}))),
        ("warning_no_target", "fp-warning", |st, _| {
            select(st, &[]);
            st.run("modify.alterPath.simplify", json!({}));
        }),
        ("preferences", "", |st, _| st.run("edit.preferences", json!({}))),
    ]
}

fn scene_for(name: &'static str, prep: Prep, kind: ThemeKind) -> Scene {
    i18n_qa_common::scene(
        format!("des1:{name}@{kind:?}"),
        name.to_string(),
        move |st, f| {
            st.set_theme(kind);
            prep(st, f);
        },
        |_, _| Ok(()),
    )
}

/// FuÃŸleiste je Dialog im Normal-Design: (Dialogkennung, Oberkante der Leiste relativ zur
/// Dialogunterkante) â€“ `None` heiÃŸt keine Leiste.
fn footer_of(name: &'static str, prep: Prep) -> (String, Option<f32>, egui::Rect) {
    let sc = scene_for(name, prep, ThemeKind::Normal);
    let mut h = build_scene(&sc, Cfg { w: 1280.0, h: 900.0, ppp: 1.0 }, false).unwrap_or_else(|e| panic!("{name}: {e}"));
    h.run_steps(4);
    let id = fp_ui::dialogs::last_dialog_id(&h.ctx).unwrap_or_else(|| panic!("{name}: kein Dialog"));
    let r = fp_ui::dialogs::dialog_rect(&h.ctx, &id).expect("Dialogrechteck");
    (id.clone(), fp_ui::dialogs::dialog_footer_top(&h.ctx, &id), r)
}

#[test]
fn footer_under_bottom_button_rows() {
    for (name, want_id, prep) in cases() {
        let (id, top, r) = footer_of(name, prep);
        if !want_id.is_empty() {
            assert_eq!(id, want_id, "{name}");
        }
        match name {
            // KnÃ¶pfe unten rechts: FuÃŸleiste, 40â€¦60 px Ã¼ber der Unterkante (12 px Ã¼ber den KnÃ¶pfen)
            "message" | "confirm_history_clear" | "unsaved" | "save_workspace" | "image_size" | "canvas_size" | "warning_no_target" | "new_document" => {
                let top = top.unwrap_or_else(|| panic!("{name}: keine FuÃŸleiste"));
                let from_bottom = r.bottom() - 1.0 - top;
                assert!((28.0..=60.0).contains(&from_bottom), "{name}: FuÃŸleiste {from_bottom} px Ã¼ber der Unterkante");
            }
            // OK/Abbrechen rechts untereinander wie im Original: keine Leiste
            "gaussian_blur" => assert_eq!(top, None, "{name}: KnÃ¶pfe rechts oben, keine FuÃŸleiste"),
            _ => {}
        }
    }
}

#[test]
fn footer_top_rules() {
    use egui::{pos2, Rect};
    let body = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 200.0));
    let ok = Rect::from_min_max(pos2(220.0, 166.0), pos2(296.0, 189.0));
    let cancel = Rect::from_min_max(pos2(304.0, 166.0), pos2(380.0, 189.0));
    // Inhalt endet weit oben: 12 px Ã¼ber den KnÃ¶pfen
    assert_eq!(fp_ui::dialogs::footer_top(body, &[ok, cancel], &[Rect::from_min_max(pos2(16.0, 16.0), pos2(300.0, 60.0))]), Some(154.0));
    // Inhalt knapp darÃ¼ber: Leiste rÃ¼ckt bis 3 px unter den Inhalt
    assert_eq!(fp_ui::dialogs::footer_top(body, &[ok, cancel], &[Rect::from_min_max(pos2(16.0, 16.0), pos2(300.0, 156.0))]), Some(159.0));
    // Inhalt reicht in die Knopfzeile (Liste bis unten): keine Leiste
    assert_eq!(fp_ui::dialogs::footer_top(body, &[ok, cancel], &[Rect::from_min_max(pos2(16.0, 16.0), pos2(200.0, 190.0))]), None);
    // KnÃ¶pfe rechts untereinander, der untere endet unten: keine Leiste
    let ok_top = Rect::from_min_max(pos2(304.0, 136.0), pos2(380.0, 159.0));
    assert_eq!(fp_ui::dialogs::footer_top(body, &[ok_top, cancel], &[]), None);
    // keine KnÃ¶pfe unten
    assert_eq!(fp_ui::dialogs::footer_top(body, &[ok_top], &[]), None);
}

#[test]
#[ignore]
fn des1_images() {
    fp_engine::shell::disable();
    let stage = std::env::var("FP_DES1_STAGE").unwrap_or_else(|_| "after".into());
    let only = std::env::var("FP_QA_NUR").ok().filter(|s| !s.is_empty());
    let cfg = Cfg { w: 1280.0, h: 900.0, ppp: 1.0 };
    let dir = repo().join("docs").join("screenshots").join("des1");
    let _ = std::fs::create_dir_all(&dir);
    let mut report = Vec::new();
    for (name, _, prep) in cases() {
        if only.as_ref().is_some_and(|o| !name.contains(o.as_str())) {
            continue;
        }
        let mut parts = Vec::new();
        for kind in ThemeKind::ALL {
            let sc = scene_for(name, prep, kind);
            let mut h = match build_scene(&sc, cfg, true) {
                Ok(h) => h,
                Err(e) => {
                    report.push(format!("{name} {kind:?}: {e}"));
                    continue;
                }
            };
            h.run_steps(4);
            let img = match h.render() {
                Ok(i) => i,
                Err(e) => {
                    eprintln!("kein Grafikadapter: {e}");
                    return;
                }
            };
            let Some(r) = fp_ui::dialogs::last_dialog_id(&h.ctx).and_then(|id| fp_ui::dialogs::dialog_rect(&h.ctx, &id)) else {
                report.push(format!("{name} {kind:?}: kein Dialog offen"));
                continue;
            };
            let r = r.expand(14.0);
            let src = (img.width(), img.height(), img.as_raw().as_slice());
            parts.push(Img::crop(src, r.left().max(0.0) as u32, r.top().max(0.0) as u32, r.width().ceil() as u32, r.height().ceil() as u32));
        }
        if parts.is_empty() {
            continue;
        }
        let file = format!("{stage}_{name}.png");
        match Img::hstack(&parts, 6).save(&dir.join(&file)) {
            Ok(()) => report.push(format!("{name} â†’ {file}")),
            Err(e) => report.push(format!("{name}: {e}")),
        }
    }
    eprintln!("{}", report.join("\n"));
}

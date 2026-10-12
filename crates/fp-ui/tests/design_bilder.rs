//! Agent DESIGN: Bildbestand aller Fenster, Modale und Dialoge in den drei Designs
//! (`docs\DESIGN.md`). Je Dialog ein Bild mit den drei Designs nebeneinander
//! (Normal | Dunkel | OLED), auf den Dialog zugeschnitten.
//!
//! ```text
//! $env:FP_NO_SHELL_OPEN='1'; $env:FP_QA_AUS='…\target-design\design-qa'
//! $env:FP_DESIGN_BILD='vorher'   # oder 'nachher'
//! cargo test -j 4 -p fp-ui --test design_bilder -- --ignored --nocapture --test-threads 1
//! ```
//!
//! Nur einzelne Dialoge: `FP_QA_NUR=<Teil der Szenen-ID>` (z. B. `dlg:export`).
//! Die Szenen (Testdokument, Dialogsuche) kommen aus dem Prüfgerüst der Mehrsprachigkeit.

#[allow(unused_macros, unused_imports)]
use crate::i18n_qa_common;

use fp_ui::theme::ThemeKind;
use i18n_qa_common::{build_scene, repo, scenes_dialogs, scenes_main, Cfg, Scene};

/// Zuschnitt eines RGBA-Bilds.
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

    /// Bilder nebeneinander (gleiche Höhe = größte, Lücke `gap` in Grau 128).
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

/// Szene mit vorgeschaltetem Design. Die Vorbereitung der Originalszene wird über die ID erneut
/// gesucht (Closures sind nicht klonbar).
fn themed(sc: &Scene, kind: ThemeKind) -> Scene {
    let id = sc.id.clone();
    let id2 = sc.id.clone();
    i18n_qa_common::scene(
        format!("{id}@{kind:?}"),
        sc.label.clone(),
        move |st, f| {
            st.set_theme(kind);
            if let Some(orig) = all_scenes().into_iter().find(|s| s.id == id) {
                (orig.prep)(st, f);
            }
        },
        move |h, f| match all_scenes().into_iter().find(|s| s.id == id2) {
            Some(orig) => (orig.ui)(h, f),
            None => Ok(()),
        },
    )
}

fn all_scenes() -> Vec<Scene> {
    let mut v: Vec<Scene> = scenes_main().into_iter().filter(|s| s.id == "haupt:leer").collect();
    v.extend(scenes_dialogs());
    v
}

/// Dateiname aus der Szenen-ID.
fn file_name(id: &str) -> String {
    let s = id.trim_start_matches("dlg:").trim_start_matches("haupt:");
    let s = if s == "leer" { "startbildschirm".to_string() } else { s.to_string() };
    s.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' }).collect::<String>().trim_matches('_').to_string()
}

#[test]
#[ignore]
fn design_bestand() {
    fp_engine::shell::disable();
    let stage = std::env::var("FP_DESIGN_BILD").unwrap_or_else(|_| "vorher".into());
    let only = std::env::var("FP_QA_NUR").ok().filter(|s| !s.is_empty());
    let cfg = Cfg { w: 1280.0, h: 860.0, ppp: 1.0 };
    let dir = repo().join("docs").join("screenshots");
    let _ = std::fs::create_dir_all(&dir);
    let mut report = Vec::new();
    for sc in all_scenes() {
        if only.as_ref().is_some_and(|o| !sc.id.contains(o.as_str())) {
            continue;
        }
        let mut parts = Vec::new();
        for kind in ThemeKind::ALL {
            let tsc = themed(&sc, kind);
            let mut h = match build_scene(&tsc, cfg, true) {
                Ok(h) => h,
                Err(e) => {
                    report.push(format!("{} {kind:?}: {e}", sc.id));
                    continue;
                }
            };
            h.run_steps(4);
            let img = match h.render() {
                Ok(i) => i,
                Err(e) => {
                    report.push(format!("kein Grafikadapter: {e}"));
                    eprintln!("{}", report.join("\n"));
                    return;
                }
            };
            // Zuschnitt: der zuletzt gezeichnete Dialog bzw. (Startbildschirm) das ganze Fenster
            let rect = fp_ui::dialogs::last_dialog_id(&h.ctx).and_then(|id| fp_ui::dialogs::dialog_rect(&h.ctx, &id)).filter(|_| sc.id.starts_with("dlg:"));
            let ppp = cfg.ppp;
            let src = (img.width(), img.height(), img.as_raw().as_slice());
            let part = match rect {
                Some(r) => {
                    let r = r.expand(10.0);
                    Img::crop(src, (r.left().max(0.0) * ppp) as u32, (r.top().max(0.0) * ppp) as u32, (r.width() * ppp).ceil() as u32, (r.height() * ppp).ceil() as u32)
                }
                None => Img::crop(src, 0, 0, img.width(), img.height()),
            };
            parts.push(part);
        }
        if parts.is_empty() {
            continue;
        }
        let name = format!("design_{stage}_{}.png", file_name(&sc.id));
        match Img::hstack(&parts, 6).save(&dir.join(&name)) {
            Ok(()) => report.push(format!("{} → {name}", sc.id)),
            Err(e) => report.push(format!("{}: {e}", sc.id)),
        }
    }
    let list = repo().join("target-design").join(format!("design_{stage}.txt"));
    let _ = std::fs::write(&list, report.join("\n"));
    eprintln!("{}", report.join("\n"));
}

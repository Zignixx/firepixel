//! Ansichten „Vorschau“, „2-fach“ und „4-fach“ des Dokumentfensters (HG): Exportbild über
//! fp-export mit den Optimieren-Einstellungen der Seite, wie im Original. 2-fach zeigt links das
//! Original, rechts die Vorschau; 4-fach das Original und drei Vorschauen (aktuelle Einstellungen
//! und zwei Vergleichsvorgaben). Jede Vorschau trägt Format, Dateigröße und Ladezeit.
//!
//! Die Bilder werden je (Dokument, Generation, Seite, Status, Einstellungen) einmal berechnet und als
//! Textur zwischengespeichert; die Leinwand ruft [`show`] statt der Originalansicht auf.

use crate::app_state::AppState;
use crate::theme::Theme;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, Stroke, StrokeKind, TextureHandle, Ui};
use fp_engine::view::PreviewMode;
use fp_export::options::{builtin_presets, OptimizeSettings};
use fp_export::render::{Background, PageRenderer, RenderRequest};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Eine berechnete Vorschau.
#[derive(Clone)]
pub struct PreviewTile {
    pub texture: Option<TextureHandle>,
    pub caption: String,
    pub size: [usize; 2],
}

type Cache = Arc<Mutex<HashMap<String, PreviewTile>>>;

fn cache(ctx: &egui::Context) -> Cache {
    let id = Id::new("fp-preview-tiles");
    if let Some(c) = ctx.data(|d| d.get_temp::<Cache>(id)) {
        return c;
    }
    let c: Cache = Arc::new(Mutex::new(HashMap::new()));
    ctx.data_mut(|d| d.insert_temp(id, Arc::clone(&c)));
    c
}

/// Einstellungen der Kacheln: 2-fach = [aktuell], 4-fach = [aktuell, Vergleich 1, Vergleich 2].
pub fn tile_settings(current: &OptimizeSettings, mode: PreviewMode) -> Vec<OptimizeSettings> {
    match mode {
        PreviewMode::Original => Vec::new(),
        PreviewMode::Preview | PreviewMode::TwoUp => vec![current.clone()],
        PreviewMode::FourUp => {
            let presets = builtin_presets();
            let mut v = vec![current.clone()];
            for name in ["JPEG – Höhere Qualität", "GIF Adaptiv 256", "PNG24", "JPEG – Kleinere Datei", "GIF Web 216"] {
                if v.len() == 3 {
                    break;
                }
                if let Some(p) = presets.iter().find(|p| p.name == name)
                    && p.settings != *current
                {
                    v.push(p.settings.clone());
                }
            }
            // Rückfall: beliebige andere Vorgaben
            for p in &presets {
                if v.len() == 3 {
                    break;
                }
                if !v.contains(&p.settings) {
                    v.push(p.settings.clone());
                }
            }
            v
        }
    }
}

/// Berechnet (oder holt) die Vorschau des aktiven Dokuments mit `s`.
pub fn tile(ctx: &egui::Context, st: &AppState, s: &OptimizeSettings) -> Option<PreviewTile> {
    let d = st.session.active()?;
    let key = format!("{}-{}-{}-{}-{:?}", d.id.0, d.generation, d.page, d.frame, s);
    let c = cache(ctx);
    if let Ok(g) = c.lock()
        && let Some(t) = g.get(&key)
    {
        return Some(t.clone());
    }
    let r = crate::panels::optimize::ExportRenderer { normal: Arc::clone(&st.renderer) };
    let req = RenderRequest { page: d.page, frame: d.frame, background: if s.matte.is_some() { Background::Transparent } else { Background::Canvas }, ..Default::default() };
    let label = crate::panels::optimize::preset_index(s).map(|i| builtin_presets()[i].name.to_string()).unwrap_or_else(|| crate::panels::optimize::settings_label(s));
    let t = match r.render(&d.doc, &req) {
        Ok(img) => {
            let ectx = fp_export::doc_export::page_context(&d.doc, d.page);
            match fp_export::export::export_image(&img, s, &ectx, &fp_export::export::EncodeExtras::standalone()) {
                Ok(ex) => {
                    let pv = fp_export::export::preview(&ex, s, (0, 0));
                    let size = [pv.width(), pv.height()];
                    let tex = ctx.load_texture(format!("fp-preview-{key}"), crate::canvas::render::to_color_image(&pv), egui::TextureOptions::NEAREST);
                    let secs = fp_export::preview::download_seconds(ex.bytes.len(), 56_000);
                    let colors = if ex.colors > 0 { format!("  {} Farben", ex.colors) } else { String::new() };
                    PreviewTile { texture: Some(tex), caption: format!("{label}  {}  {secs} s bei 56 kbit/s{colors}", fp_export::preview::format_size(ex.bytes.len())), size }
                }
                Err(e) => PreviewTile { texture: None, caption: format!("{label}: {e}"), size: [0, 0] },
            }
        }
        Err(e) => PreviewTile { texture: None, caption: format!("Vorschau nicht möglich: {e}"), size: [0, 0] },
    };
    if let Ok(mut g) = c.lock() {
        // alte Generationen desselben Dokuments verwerfen
        let prefix = format!("{}-", d.id.0);
        let gen_prefix = format!("{}-{}-", d.id.0, d.generation);
        g.retain(|k, _| !k.starts_with(&prefix) || k.starts_with(&gen_prefix));
        g.insert(key, t.clone());
    }
    Some(t)
}

/// Zeichnet die Vorschauansicht des aktiven Dokuments in `rect` (Modus aus `view.preview_mode`).
/// `original` zeichnet bei 2-/4-fach die Originalkachel (die Leinwand übergibt ihre Zeichenfunktion).
pub fn show(ui: &mut Ui, st: &mut AppState, rect: Rect, original: &mut dyn FnMut(&mut Ui, &mut AppState, Rect)) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let Some(d) = st.session.active() else { return };
    let mode = d.view.preview_mode;
    let zoom = d.view.zoom as f32;
    let current = fp_export::doc_export::page_settings(&d.doc, d.page);
    let settings = tile_settings(&current, mode);
    let panes: Vec<Rect> = match mode {
        PreviewMode::Original => return original(ui, st, rect),
        PreviewMode::Preview => vec![rect],
        PreviewMode::TwoUp => {
            let w = (rect.width() - 2.0) / 2.0;
            vec![Rect::from_min_size(rect.min, vec2(w, rect.height())), Rect::from_min_size(pos2(rect.left() + w + 2.0, rect.top()), vec2(w, rect.height()))]
        }
        PreviewMode::FourUp => {
            let w = (rect.width() - 2.0) / 2.0;
            let h = (rect.height() - 2.0) / 2.0;
            (0..4).map(|i| Rect::from_min_size(pos2(rect.left() + (i % 2) as f32 * (w + 2.0), rect.top() + (i / 2) as f32 * (h + 2.0)), vec2(w, h))).collect()
        }
    };
    ui.painter().rect_filled(rect, 0.0, c.frame_bg);
    let ctx = ui.ctx().clone();
    let mut tiles = settings.iter();
    for (i, pane) in panes.iter().enumerate() {
        let caption_h = 20.0;
        let body = Rect::from_min_max(pane.min, pos2(pane.right(), pane.bottom() - caption_h));
        ui.painter().rect_filled(*pane, 0.0, c.canvas_bg);
        let is_original = mode != PreviewMode::Preview && i == 0;
        let caption = if is_original {
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(body));
            child.set_clip_rect(body);
            original(&mut child, st, body);
            "Original".to_string()
        } else {
            match tiles.next().and_then(|s| tile(&ctx, st, s)) {
                Some(t) => {
                    if let Some(tex) = &t.texture {
                        let size = vec2(t.size[0] as f32, t.size[1] as f32) * zoom / ctx.pixels_per_point().max(0.01);
                        let r = Rect::from_center_size(body.center(), size);
                        let p = ui.painter().with_clip_rect(body);
                        crate::widgets::checker(&p, r.intersect(body), 8.0, &theme);
                        p.image(tex.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                    }
                    t.caption
                }
                None => String::new(),
            }
        };
        let cap = Rect::from_min_max(pos2(pane.left(), pane.bottom() - caption_h), pane.max);
        ui.painter().rect_filled(cap, 0.0, c.frame_bg);
        ui.painter().text(pos2(cap.left() + 6.0, cap.center().y), Align2::LEFT_CENTER, caption, theme.ui_font(), c.text);
        ui.painter().rect_stroke(*pane, 0.0, Stroke::new(1.0, c.line_mid), StrokeKind::Inside);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_up_has_three_distinct_settings() {
        let cur = OptimizeSettings::default();
        let v = tile_settings(&cur, PreviewMode::FourUp);
        assert_eq!(v.len(), 3);
        assert_ne!(v[0], v[1]);
        assert_ne!(v[1], v[2]);
        assert_eq!(tile_settings(&cur, PreviewMode::TwoUp).len(), 1);
        assert!(tile_settings(&cur, PreviewMode::Original).is_empty());
    }
}

//! Tonwertkorrektur (19111) mit Histogramm und Gradationskurven (19112) mit Kurvenfläche: Kanal
//! „RGB, Rot, Grün, Blau“ (`RT_DLGINIT`), Tonwertspreizung (3 Werte), Tonwertumfang (2 Werte),
//! Auto, Vorschau über den Verlauf wie bei den Formular-Dialogen.

use super::form::{apply_preview, revert_preview, FormDialog};
use super::{dialog_frame, push_button, refs, t, tl, DialogClose};
use crate::app_state::AppState;
use crate::inspector::fields;
use crate::theme::Theme;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, Sense, Shape, Stroke, StrokeKind, Ui};
use fp_raster::filters::{LevelsChannel, LevelsParams};
use serde_json::json;

const CHANNELS: [&str; 4] = ["dlg-channel-rgb", "dlg-channel-red", "dlg-channel-green", "dlg-channel-blue"];

/// Histogramme (Helligkeit, R, G, B) der ausgewählten Objekte.
pub fn histogram(st: &AppState) -> [[u64; 256]; 4] {
    let mut h = [[0u64; 256]; 4];
    let Some(d) = st.session.active() else { return h };
    for id in &d.selection.elements {
        let Some(e) = fp_engine::docops::element_on_page(&d.doc, d.page, *id) else { continue };
        let img = match &e.kind {
            fp_model::ElementKind::Bitmap(b) => fp_engine::docops::bitmap_image(b),
            _ => Some(st.renderer.render_element(&d.doc, &e, &fp_render::PointMap::identity(), d.frame).image),
        };
        let Some(img) = img else { continue };
        for p in img.data() {
            if p[3] == 0 {
                continue;
            }
            let l = ((p[0] as u32 * 299 + p[1] as u32 * 587 + p[2] as u32 * 114) / 1000).min(255) as usize;
            h[0][l] += 1;
            h[1][p[0] as usize] += 1;
            h[2][p[1] as usize] += 1;
            h[3][p[2] as usize] += 1;
        }
    }
    h
}

fn draw_histogram(ui: &Ui, r: Rect, hist: &[u64; 256], col: Color32) {
    let t = Theme::current(ui.ctx());
    ui.painter().rect_filled(r, 0.0, t.colors.list_bg);
    let max = hist.iter().copied().max().unwrap_or(1).max(1) as f32;
    for (i, v) in hist.iter().enumerate() {
        let x = r.left() + i as f32 / 255.0 * r.width();
        let hgt = (*v as f32 / max).sqrt() * r.height();
        ui.painter().vline(x, (r.bottom() - hgt)..=r.bottom(), Stroke::new(r.width() / 256.0 + 0.3, col));
    }
    ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0, t.colors.list_border), StrokeKind::Inside);
}

fn tri(ui: &Ui, x: f32, y: f32, fill: Color32, border: Color32) {
    ui.painter().add(Shape::convex_polygon(vec![pos2(x, y), pos2(x + 5.0, y + 8.0), pos2(x - 5.0, y + 8.0)], fill, Stroke::new(1.0, border)));
}

// ---------------------------------------------------------------------- Tonwertkorrektur

/// Zustand der Tonwertkorrektur.
#[derive(Clone, Debug)]
pub struct LevelsDialog {
    pub params: LevelsParams,
    pub channel: usize,
    pub preview: FormDialog,
    pub hist: [[u64; 256]; 4],
    pub live: bool,
}

impl LevelsDialog {
    /// Neuer Dialog (Histogramm der Auswahl).
    pub fn new(st: &AppState, live: bool) -> LevelsDialog {
        let base = if live { json!({"live": true}) } else { json!({}) };
        LevelsDialog { params: LevelsParams::default(), channel: 0, preview: FormDialog::new("19111", &t("dlg-levels-title"), "filter.adjustColor.levels", vec![]).with_base(base).with_preview(), hist: histogram(st), live }
    }

    fn channel_mut(&mut self) -> &mut LevelsChannel {
        match self.channel {
            1 => &mut self.params.red,
            2 => &mut self.params.green,
            3 => &mut self.params.blue,
            _ => &mut self.params.rgb,
        }
    }

    /// Befehlsparameter.
    pub fn command_params(&self) -> serde_json::Value {
        let mut v = serde_json::to_value(self.params).unwrap_or(json!({}));
        if self.live {
            v["live"] = json!(true);
        }
        v
    }
}

/// Zeigt die Tonwertkorrektur.
pub fn show_levels(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut d) = st.dialogs.levels.clone() else { return };
    let mut result = None;
    let mut auto = false;
    let channels = tl(&CHANNELS);
    let (ch_l, in_l, out_l, auto_l, prev_l, ok_l, cancel_l) = (t("dlg-channel"), t("dlg-levels-input"), t("dlg-levels-output"), t("dlg-auto"), t("dlg-preview"), t("btn-ok"), t("btn-cancel"));
    let dch = super::extra_w(ctx, &[&ch_l], 40.0);
    let dl = super::extra_w(ctx, &[&in_l, &out_l], 120.0);
    let aw = super::button_w(ctx, &auto_l, 76.0);
    let width = 400f32.max(14.0 + 46.0 + dch + 100.0 + 12.0 + aw + 16.0).max(14.0 + 124.0 + dl + 2.0 * 52.0 + 46.0 + 14.0).max(super::button_row_w(ctx, &[&ok_l, &cancel_l], 76.0, 8.0) + 16.0 + 14.0 + 18.0 + super::text_w(ctx, &prev_l) + 12.0).ceil();
    let r = dialog_frame(ctx, "fp-levels", &t("dlg-levels-title"), vec2(width, 330.0), |ui, body| {
        let t = Theme::current(ui.ctx());
        let o = body.min + vec2(14.0, 12.0);
        super::fit_label(ui, Rect::from_min_size(o, vec2(40.0 + dch, 20.0)), &ch_l, true);
        if let Some(c) = fields::combo(ui, "lv-ch", Rect::from_min_size(o + vec2(46.0 + dch, 0.0), vec2(super::combo_fit_w(ui.ctx(), &refs(&channels), 100.0), 20.0)), &refs(&channels), Some(d.channel), true) {
            d.channel = c;
        }
        let ch = *d.channel_mut();
        super::fit_label(ui, Rect::from_min_size(o + vec2(0.0, 30.0), vec2(120.0 + dl, 20.0)), &in_l, true);
        let mut new = ch;
        for (k, (v, rng, dec)) in [(ch.source_low as f64, (0.0, 253.0), 0), (ch.gamma, (0.1, 10.0), 2), (ch.source_high as f64, (2.0, 255.0), 0)].iter().enumerate() {
            if let Some(nv) = fields::number(ui, ("lv-in", k), Rect::from_min_size(o + vec2(124.0 + dl + k as f32 * 52.0, 30.0), vec2(46.0, 20.0)), None, Some(*v), *rng, *dec, true) {
                match k {
                    0 => new.source_low = (nv as i32).min(new.source_high - 2),
                    1 => new.gamma = nv,
                    _ => new.source_high = (nv as i32).max(new.source_low + 2),
                }
            }
        }
        let hr = Rect::from_min_size(o + vec2(0.0, 58.0), vec2(body.width() - 28.0, 120.0));
        let col = match d.channel {
            1 => t.colors.icon_red,
            2 => t.colors.icon_green,
            3 => t.colors.icon_blue,
            _ => t.colors.icon_fg,
        };
        draw_histogram(ui, hr, &d.hist[d.channel], col);
        // Schieber unter dem Histogramm
        let sy = hr.bottom() + 2.0;
        let xs = |v: f64| hr.left() + (v as f32 / 255.0) * hr.width();
        let mid = ch.source_low as f64 + (ch.source_high - ch.source_low) as f64 * 0.5f64.powf(ch.gamma);
        let sr = Rect::from_min_max(pos2(hr.left() - 6.0, sy), pos2(hr.right() + 6.0, sy + 12.0));
        let resp = ui.interact(sr, Id::new("lv-sl"), Sense::click_and_drag());
        tri(ui, xs(ch.source_low as f64), sy, t.colors.icon_fg, t.colors.icon_fg);
        tri(ui, xs(mid), sy, t.colors.icon_disabled, t.colors.icon_fg);
        tri(ui, xs(ch.source_high as f64), sy, Color32::WHITE, t.colors.icon_fg);
        if let Some(p) = resp.interact_pointer_pos()
            && resp.dragged()
        {
            let v = ((p.x - hr.left()) / hr.width() * 255.0).clamp(0.0, 255.0) as f64;
            let dl = (v - ch.source_low as f64).abs();
            let dm = (v - mid).abs();
            let dh = (v - ch.source_high as f64).abs();
            if dl <= dm && dl <= dh {
                new.source_low = (v as i32).min(new.source_high - 2);
            } else if dh <= dm {
                new.source_high = (v as i32).max(new.source_low + 2);
            } else {
                let span = (ch.source_high - ch.source_low).max(1) as f64;
                let tpos = ((v - ch.source_low as f64) / span).clamp(0.01, 0.99);
                new.gamma = (tpos.ln() / 0.5f64.ln()).clamp(0.1, 10.0);
            }
        }
        super::fit_label(ui, Rect::from_min_size(o + vec2(0.0, 206.0), vec2(120.0 + dl, 20.0)), &out_l, true);
        for (k, v) in [ch.dest_low, ch.dest_high].iter().enumerate() {
            if let Some(nv) = fields::number(ui, ("lv-out", k), Rect::from_min_size(o + vec2(124.0 + dl + k as f32 * 104.0, 206.0), vec2(46.0, 20.0)), None, Some(*v as f64), (0.0, 255.0), 0, true) {
                if k == 0 {
                    new.dest_low = nv as i32;
                } else {
                    new.dest_high = nv as i32;
                }
            }
        }
        let gr = Rect::from_min_size(o + vec2(0.0, 232.0), vec2(hr.width(), 10.0));
        crate::widgets::vgradient(ui.painter(), gr, Color32::BLACK, Color32::BLACK);
        let mut mesh = egui::epaint::Mesh::default();
        mesh.colored_vertex(gr.left_top(), Color32::BLACK);
        mesh.colored_vertex(gr.right_top(), Color32::WHITE);
        mesh.colored_vertex(gr.left_bottom(), Color32::BLACK);
        mesh.colored_vertex(gr.right_bottom(), Color32::WHITE);
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(1, 2, 3);
        ui.painter().add(Shape::mesh(mesh));
        tri(ui, gr.left() + new.dest_low as f32 / 255.0 * gr.width(), gr.bottom() + 1.0, t.colors.icon_fg, t.colors.icon_fg);
        tri(ui, gr.left() + new.dest_high as f32 / 255.0 * gr.width(), gr.bottom() + 1.0, Color32::WHITE, t.colors.icon_fg);
        if new != ch {
            *d.channel_mut() = new;
        }
        if let Some(b) = fields::checkbox(ui, "lv-prev", Rect::from_min_size(pos2(body.left() + 14.0, body.bottom() - 32.0), vec2((18.0 + super::text_w(ui.ctx(), &prev_l)).max(80.0), 16.0)), &prev_l, d.preview.preview_on, true) {
            d.preview.preview_on = b;
        }
        if fields::button(ui, "lv-auto", Rect::from_min_size(pos2(body.right() - 16.0 - aw, o.y), vec2(aw, 22.0)), &auto_l, true) {
            auto = true;
        }
        let b = super::button_row(ui.ctx(), body.right() - 16.0, body.bottom() - 36.0, &[&ok_l, &cancel_l], 76.0, 23.0, 8.0);
        if push_button(ui, b[0], &ok_l, true, true) {
            result = Some(true);
        }
        if push_button(ui, b[1], &cancel_l, false, true) {
            result = Some(false);
        }
        let _ = Align2::LEFT_TOP;
    });
    let params = d.command_params();
    finish_preview(ctx, st, &mut d.preview, params, result.or(if r == DialogClose::Cancelled { Some(false) } else { None }), auto);
    if result.is_some() || r == DialogClose::Cancelled || auto {
        st.dialogs.levels = None;
    } else {
        st.dialogs.levels = Some(d);
    }
}

/// Gemeinsame Vorschau-/Abschlusslogik der Kurven- und Tonwertdialoge.
fn finish_preview(ctx: &egui::Context, st: &mut AppState, pv: &mut FormDialog, params: serde_json::Value, result: Option<bool>, auto: bool) {
    pv.base = params.clone();
    let dragging = ctx.input(|i| i.pointer.any_down());
    if auto {
        revert_preview(st, pv);
        st.run("filter.adjustColor.autoLevels", if params.get("live").is_some() { json!({"live": true}) } else { json!({}) });
        return;
    }
    match result {
        Some(true) => {
            if !(pv.applied.is_some() && pv.applied_params.as_ref() == Some(&params)) {
                revert_preview(st, pv);
                st.run(&pv.command.clone(), params);
            }
        }
        Some(false) => revert_preview(st, pv),
        None => {
            if pv.preview_on && !dragging {
                apply_preview(st, pv);
            } else if !pv.preview_on {
                revert_preview(st, pv);
            }
        }
    }
}

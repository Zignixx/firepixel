//! Ansichten „Vorschau“, „2-fach“ und „4-fach“ des Dokumentfensters (HG): Exportbild über
//! fp-export mit den Optimieren-Einstellungen der Seite, wie im Original. 2-fach zeigt links das
//! Original, rechts die Vorschau; 4-fach das Original und drei Vorschauen (aktuelle Einstellungen
//! und zwei Vergleichsvorgaben). Jede Vorschau trägt Format, Dateigröße und Ladezeit.
//!
//! Die Bilder werden je (Dokument, Generation, Seite, Status, Einstellungen) einmal berechnet und als
//! Textur zwischengespeichert; die Leinwand ruft [`show`] statt der Originalansicht auf.

use crate::app_state::AppState;
use crate::theme::Theme;
use egui::{Align2, Color32, Id, Pos2, Rect, Stroke, StrokeKind, TextureHandle, Ui, pos2, vec2};
use fp_engine::view::PreviewMode;
use fp_export::options::{OptimizeSettings, builtin_presets};
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
            for name in [
                "JPEG – Höhere Qualität",
                "GIF Adaptiv 256",
                "PNG24",
                "JPEG – Kleinere Datei",
                "GIF Web 216",
            ] {
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
    // Sprachstand im Schlüssel: die Beschriftung wechselt mit der Sprache
    let gamma = d.view.gamma_preview;
    let key = format!(
        "{}-{}-{}-{}-{}-{}-{:?}",
        d.id.0,
        d.generation,
        d.page,
        d.frame,
        fp_i18n::generation(),
        gamma,
        s
    );
    let c = cache(ctx);
    if let Ok(g) = c.lock()
        && let Some(t) = g.get(&key)
    {
        return Some(t.clone());
    }
    let r = crate::panels::optimize::ExportRenderer {
        normal: Arc::clone(&st.renderer),
    };
    let req = RenderRequest {
        page: d.page,
        frame: d.frame,
        background: if s.matte.is_some() {
            Background::Transparent
        } else {
            Background::Canvas
        },
        ..Default::default()
    };
    let label = crate::panels::optimize::preset_index(s)
        .map(|i| builtin_presets()[i].name.to_string())
        .unwrap_or_else(|| crate::panels::optimize::settings_label(s));
    let t = match r.render(&d.doc, &req) {
        Ok(img) => {
            let ectx = fp_export::doc_export::page_context(&d.doc, d.page);
            // re 28 D4: die Exportvorschau im Dokumentfenster zeigt im Dokumentmaßstab, deckungsgleich
            // mit „Original“; die Exportskalierung wirkt hier nicht (anders als in der Bildvorschau)
            let s = &unscaled(s);
            match fp_export::export::export_image(
                &img,
                s,
                &ectx,
                &fp_export::export::EncodeExtras::standalone(),
            ) {
                Ok(ex) => {
                    let mut pv = fp_export::export::preview(&ex, s, (0, 0));
                    // Ansicht ▸ Macintosh-Gamma gilt auch für die Kacheln der 2-/4-fach-Ansicht (BA-045)
                    if gamma {
                        crate::canvas::render::apply_gamma_preview(&mut pv);
                    }
                    let size = [pv.width(), pv.height()];
                    let tex = ctx.load_texture(
                        format!("fp-preview-{key}"),
                        crate::canvas::render::to_color_image(&pv),
                        egui::TextureOptions::NEAREST,
                    );
                    let secs = fp_export::preview::download_seconds(ex.bytes.len(), 56_000);
                    let colors = if ex.colors > 0 {
                        format!("  {}", fp_i18n::tr!("ui-preview-colors", n = ex.colors))
                    } else {
                        String::new()
                    };
                    PreviewTile {
                        texture: Some(tex),
                        caption: format!(
                            "{}{colors}",
                            fp_i18n::tr!(
                                "ui-preview-caption",
                                label = label.as_str(),
                                size = fp_export::preview::format_size(ex.bytes.len()),
                                secs = secs.to_string()
                            )
                        ),
                        size,
                    }
                }
                Err(e) => PreviewTile {
                    texture: None,
                    caption: format!("{label}: {e}"),
                    size: [0, 0],
                },
            }
        }
        Err(e) => PreviewTile {
            texture: None,
            caption: fp_i18n::tr!("ui-preview-failed", error = e.to_string()),
            size: [0, 0],
        },
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

/// Einstellung ohne Exportskalierung (2-/4-fach im Dokumentfenster, re 28 D4).
pub fn unscaled(s: &OptimizeSettings) -> OptimizeSettings {
    OptimizeSettings {
        scale: None,
        ..s.clone()
    }
}

/// Höhe der Beschriftung unter jedem Feld.
const CAPTION_H: f32 = 20.0;

/// Felder der Ansicht `mode` in `rect` (2-fach nebeneinander, 4-fach 2 × 2, je 2 px Abstand).
pub fn pane_rects(rect: Rect, mode: PreviewMode) -> Vec<Rect> {
    match mode {
        PreviewMode::Original | PreviewMode::Preview => vec![rect],
        PreviewMode::TwoUp => {
            let w = (rect.width() - 2.0) / 2.0;
            vec![
                Rect::from_min_size(rect.min, vec2(w, rect.height())),
                Rect::from_min_size(
                    pos2(rect.left() + w + 2.0, rect.top()),
                    vec2(w, rect.height()),
                ),
            ]
        }
        PreviewMode::FourUp => {
            let w = (rect.width() - 2.0) / 2.0;
            let h = (rect.height() - 2.0) / 2.0;
            (0..4)
                .map(|i| {
                    Rect::from_min_size(
                        pos2(
                            rect.left() + (i % 2) as f32 * (w + 2.0),
                            rect.top() + (i / 2) as f32 * (h + 2.0),
                        ),
                        vec2(w, h),
                    )
                })
                .collect()
        }
    }
}

/// Bildfläche eines Feldes (ohne Beschriftung).
pub fn pane_body(pane: Rect) -> Rect {
    Rect::from_min_max(pane.min, pos2(pane.right(), pane.bottom() - CAPTION_H))
}

/// Linke obere Ecke des gemeinsamen Sichtbereichs, wenn er in der Mitte von `body` liegt: Alle
/// Felder zeigen dieselbe Lage (Zoom und Mittelpunkt in Dokumentkoordinaten, N25). Bildschirmpunkt
/// `p` im Feld entspricht dem Dokumentpunkt `view.screen_to_doc(p - virtual_min)`.
pub fn virtual_min(view: &fp_engine::ViewState, body: Rect) -> Pos2 {
    body.center() - vec2(view.viewport.0 as f32, view.viewport.1 as f32) / 2.0
}

/// Dokumentrechteck, das das Exportbild einer Vorschau zeigt: die Seite bzw. ihr Exportbereich.
fn image_doc_rect(pr: fp_geom::IRect, s: &OptimizeSettings) -> Rect {
    let c = fp_export::doc_export::crop_rect(pr.width() as usize, pr.height() as usize, s);
    Rect::from_min_max(
        pos2((pr.left + c.left) as f32, (pr.top + c.top) as f32),
        pos2((pr.left + c.right) as f32, (pr.top + c.bottom) as f32),
    )
}

/// Bildschirmlage des Exportbildes im Feld `body` (gemeinsame Ansicht des Dokuments).
pub fn image_screen_rect(
    view: &fp_engine::ViewState,
    body: Rect,
    pr: fp_geom::IRect,
    s: &OptimizeSettings,
) -> Rect {
    let vmin = virtual_min(view, body);
    let r = image_doc_rect(pr, s);
    let at = |x: f32, y: f32| {
        let (sx, sy) = view.doc_to_screen(fp_geom::Point::new(x, y));
        vmin + vec2(sx as f32, sy as f32)
    };
    Rect::from_min_max(at(r.left(), r.top()), at(r.right(), r.bottom()))
}

/// Laufendes Ziehen mit Hand/Leertaste/mittlerer Taste in einem Vorschaufeld: Druckpunkt und
/// Ansichtsursprung beim Drücken (der gegriffene Dokumentpunkt bleibt unter dem Zeiger).
#[derive(Clone, Copy)]
struct Grab {
    press: Pos2,
    origin: (f64, f64),
}

/// Bewegen in einem Vorschaufeld wie auf der Leinwand: Mausrad (Bildlauf), Strg+Rad und
/// Zwei-Finger-Zoom (eine Stufe je Rastung um den Zeiger), Hand-Werkzeug, Leertaste+Ziehen und
/// mittlere Taste (Verschieben), Zoomwerkzeug bzw. Strg+Leertaste (Klick = größer, Alt+Klick =
/// kleiner, Rahmen aufziehen = einpassen). Alles wirkt auf die eine Ansicht des Dokuments, die
/// alle Felder zeigen (N25).
fn navigate(ui: &mut Ui, st: &mut AppState, idx: usize, body: Rect) {
    let resp = ui.interact(
        body,
        crate::canvas::pane_id(st, Id::new(("fp-preview-pane", idx))),
        egui::Sense::click_and_drag(),
    );
    let Some(d) = st.session.active() else { return };
    let id = d.id;
    let view = d.view.clone();
    let vmin = virtual_min(&view, body);
    let anchor = |p: Pos2| ((p.x - vmin.x) as f64, (p.y - vmin.y) as f64);
    let tool = st.session.tools.active;
    let space = st.space_down;
    type Wheel = (egui::MouseWheelUnit, egui::Vec2, bool, bool);
    let (wheel, pinch, mods, pointer, press, latest) = ui.input(|i| {
        let mut w: Vec<Wheel> = Vec::new();
        let mut z = 1.0f32;
        for e in &i.events {
            match e {
                egui::Event::MouseWheel {
                    unit,
                    delta,
                    modifiers,
                    ..
                } => w.push((
                    *unit,
                    *delta,
                    modifiers.ctrl || modifiers.command,
                    modifiers.shift,
                )),
                egui::Event::Zoom(f) => z *= *f,
                _ => {}
            }
        }
        // Umschalttasten aus dem Zustand oder aus dem Klick selbst (Taste und Klick im selben Bild)
        let mut m = i.modifiers;
        for e in &i.events {
            if let egui::Event::PointerButton { modifiers, .. } = e {
                m.alt |= modifiers.alt;
                m.ctrl |= modifiers.ctrl;
                m.command |= modifiers.command;
            }
        }
        (
            w,
            z,
            m,
            i.pointer.hover_pos(),
            i.pointer.press_origin(),
            i.pointer.latest_pos(),
        )
    });
    let hovered = resp.hovered() || resp.dragged();
    if hovered {
        // Strg+Rad / Zwei-Finger-Zoom: je volle Rastung eine Stufe um den Zeiger (wie die Leinwand)
        let mut steps = 0;
        {
            let cc = st.canvases.entry(id).or_default();
            for (unit, delta, ctrl, _) in &wheel {
                if !*ctrl {
                    continue;
                }
                let n = crate::canvas::wheel_notches(*unit, delta.y);
                if n * cc.wheel_acc < 0.0 {
                    cc.wheel_acc = 0.0;
                }
                cc.wheel_acc += n;
                steps += crate::canvas::take_step(&mut cc.wheel_acc);
            }
            if pinch != 1.0 {
                cc.pinch_acc += pinch.ln() / 1.25f32.ln();
                steps += crate::canvas::take_step(&mut cc.pinch_acc);
            }
        }
        // Bildlauf: Zeile = 52 Bildschirmpixel, Seite = Feldhöhe, Punkte direkt; Umschalt+Rad nichts
        let mut pan = egui::Vec2::ZERO;
        for (unit, delta, ctrl, shift) in &wheel {
            if *ctrl || *shift {
                continue;
            }
            let k = match unit {
                egui::MouseWheelUnit::Point => 1.0,
                egui::MouseWheelUnit::Line => 52.0,
                egui::MouseWheelUnit::Page => body.height(),
            };
            pan += *delta * k;
        }
        if let Some(d) = st.session.active_mut() {
            let a = pointer.map(anchor);
            for _ in 0..steps.abs() {
                if steps > 0 {
                    d.view.zoom_in(a)
                } else {
                    d.view.zoom_out(a)
                }
            }
            if pan != egui::Vec2::ZERO {
                d.view.pan(pan.x as f64, pan.y as f64);
            }
        }
    }
    let ctrl = mods.ctrl || mods.command;
    let hand = tool == fp_engine::ToolId::Hand || (space && !ctrl);
    let zoom_tool = tool == fp_engine::ToolId::Zoom || (space && ctrl);
    let grab_id = crate::canvas::pane_id(st, Id::new(("fp-preview-grab", idx)));
    let panning = resp.dragged_by(egui::PointerButton::Middle)
        || (hand && resp.dragged_by(egui::PointerButton::Primary));
    if panning {
        let grab = ui.ctx().data(|m| m.get_temp::<Grab>(grab_id));
        let grab = match grab {
            Some(g) => g,
            None => {
                let g = Grab {
                    press: press.unwrap_or(body.center()),
                    origin: view.origin,
                };
                ui.ctx().data_mut(|m| m.insert_temp(grab_id, g));
                g
            }
        };
        if let (Some(p), Some(d)) = (latest, st.session.active_mut()) {
            let z = d.view.zoom.max(1e-9);
            d.view.origin = (
                grab.origin.0 - (p.x - grab.press.x) as f64 / z,
                grab.origin.1 - (p.y - grab.press.y) as f64 / z,
            );
        }
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        return;
    }
    ui.ctx().data_mut(|m| m.remove::<Grab>(grab_id));
    if zoom_tool {
        if hovered {
            ui.ctx().set_cursor_icon(if mods.alt {
                egui::CursorIcon::ZoomOut
            } else {
                egui::CursorIcon::ZoomIn
            });
        }
        // Rahmen aufziehen (nur Zoomwerkzeug, ohne Alt): Bereich einpassen
        if tool == fp_engine::ToolId::Zoom
            && !mods.alt
            && let (Some(a), Some(b)) = (press, latest)
            && (resp.dragged_by(egui::PointerButton::Primary)
                || resp.drag_stopped_by(egui::PointerButton::Primary))
        {
            let r = Rect::from_two_pos(a, b);
            if resp.drag_stopped_by(egui::PointerButton::Primary) {
                let (p0, p1) = (anchor(r.min), anchor(r.max));
                if let Some(d) = st.session.active_mut() {
                    let (a, b) = (
                        d.view.screen_to_doc(p0.0, p0.1),
                        d.view.screen_to_doc(p1.0, p1.1),
                    );
                    d.view.fit_zoom_rect(fp_geom::Rect::new(a.x, a.y, b.x, b.y));
                }
            } else {
                // gestrichelter Rahmen wie auf der Leinwand (BA-048)
                let theme = Theme::current(ui.ctx());
                let time = ui.input(|i| i.time);
                crate::canvas::dashed_rect(
                    &ui.painter().with_clip_rect(body),
                    r,
                    theme.colors.marquee_a,
                    theme.colors.marquee_b,
                    time,
                );
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(120));
            }
            return;
        }
        if resp.clicked_by(egui::PointerButton::Primary)
            && let Some(p) = resp.interact_pointer_pos()
            && let Some(d) = st.session.active_mut()
        {
            if mods.alt {
                d.view.zoom_out(Some(anchor(p)))
            } else {
                d.view.zoom_in(Some(anchor(p)))
            }
        }
    } else if hand && hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }
}

/// Zeichnet die Vorschauansicht des aktiven Dokuments in `rect` (Modus aus `view.preview_mode`).
/// `original` zeichnet bei 2-/4-fach die Originalkachel (die Leinwand übergibt ihre Zeichenfunktion).
/// Alle Felder zeigen dieselbe Lage (Zoom und Dokumentpunkt in der Feldmitte); jede Bewegung in
/// einem Feld wirkt auf alle (N25).
pub fn show(
    ui: &mut Ui,
    st: &mut AppState,
    rect: Rect,
    original: &mut dyn FnMut(&mut Ui, &mut AppState, Rect),
) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let Some(d) = st.session.active() else { return };
    let mode = d.view.preview_mode;
    if mode == PreviewMode::Original {
        return original(ui, st, rect);
    }
    let current = fp_export::doc_export::page_settings(&d.doc, d.page);
    let settings = tile_settings(&current, mode);
    let panes = pane_rects(rect, mode);
    let is_original = |i: usize| mode != PreviewMode::Preview && i == 0;
    // Nur „Vorschau“: das Feld ist der Sichtbereich (sonst meldet ihn die Originalkachel)
    if mode == PreviewMode::Preview {
        crate::canvas::report_viewport(st, pane_body(panes[0]).size());
    }
    // erst bewegen, dann zeichnen: alle Felder zeigen im selben Bild dieselbe Lage
    for (i, pane) in panes.iter().enumerate() {
        if !is_original(i) {
            navigate(ui, st, i, pane_body(*pane));
        }
    }
    if let Some(d) = st.session.active_mut()
        && let Some(pr) = crate::canvas::page_rect(&d.doc, d.page)
    {
        // 2-/4-fach: dieselbe Grenze wie die Originalkachel mit ihren Bildlaufleisten (FIX-A, BV-5)
        let bar = if mode == PreviewMode::Preview {
            0.0
        } else {
            crate::canvas::SCROLLBAR_T
        };
        crate::canvas::clamp_view_bars(&mut d.view, pr, bar);
    }
    ui.painter().rect_filled(rect, 0.0, c.frame_bg);
    let ctx = ui.ctx().clone();
    let mut tiles = settings.iter();
    for (i, pane) in panes.iter().enumerate() {
        let body = pane_body(*pane);
        ui.painter().rect_filled(*pane, 0.0, c.canvas_bg);
        let caption = if is_original(i) {
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(body));
            child.set_clip_rect(body);
            original(&mut child, st, body);
            fp_i18n::t("ui-preview-original")
        } else {
            let s = tiles.next();
            match s.and_then(|s| tile(&ctx, st, s).map(|t| (s, t))) {
                Some((s, t)) => {
                    if let Some(tex) = &t.texture
                        && let Some(d) = st.session.active()
                        && let Some(pr) = crate::canvas::page_rect(&d.doc, d.page)
                    {
                        let r = image_screen_rect(&d.view, body, pr, s);
                        let p = ui.painter().with_clip_rect(body.intersect(ui.clip_rect()));
                        crate::widgets::checker_view(
                            &p,
                            r.intersect(body),
                            8.0,
                            &theme,
                            d.view.gamma_preview,
                        );
                        p.image(
                            tex.id(),
                            r,
                            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                    t.caption
                }
                None => String::new(),
            }
        };
        let caption_h = CAPTION_H;
        let cap = Rect::from_min_max(pos2(pane.left(), pane.bottom() - caption_h), pane.max);
        ui.painter().rect_filled(cap, 0.0, c.frame_bg);
        ui.painter().text(
            pos2(cap.left() + 6.0, cap.center().y),
            Align2::LEFT_CENTER,
            caption,
            theme.ui_font(),
            c.text,
        );
        ui.painter()
            .rect_stroke(*pane, 0.0, Stroke::new(1.0, c.line_mid), StrokeKind::Inside);
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

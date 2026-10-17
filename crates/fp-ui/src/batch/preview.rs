//! Karte ③ Vorschau (SPEC-BATCH §2.9): Reiter Vorher | Nachher | Vergleich (Teiler ziehbar) |
//! Plan (Trockenlauf) | Fehler; Bild mit Zoom (Einpassen, 100 %, 200 %, Mausrad um die Maus),
//! Verschieben, Hintergrund wählbar; darunter Pixel- und Dateigröße vorher/nachher und Rechenzeit.
//!
//! Gerechnet wird im Hintergrund (`fp_batch::preview` auf einem Proxy ≤ 1024 px), entprellt 150 ms
//! nach der letzten Änderung; jede neue Anfrage bricht die alte ab (Generationszähler). Danach
//! rechnet derselbe Thread die volle Größe und kodiert sie für die echte Dateigröße „Nachher“.

use super::Studio;
use super::kit;
use crate::app_state::AppState;
use crate::theme::{Theme, with_alpha};
use crate::ui_kit::{self, KitIcon, RowIcon, sp, type_scale};
use egui::{
    Align2, Color32, ColorImage, CornerRadius, Id, Rect, Sense, Stroke, TextureHandle,
    TextureOptions, Ui, pos2, vec2,
};
use fp_batch::{CancelToken, ItemInfo, PreviewOptions, Recipe, StepRegistry};
use fp_i18n::t;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, channel};

/// Entprellen nach der letzten Änderung (Sekunden).
const DEBOUNCE: f64 = 0.15;

/// Reiter der Vorschau-Karte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Before,
    After,
    Compare,
    Plan,
    Errors,
}

/// Zoom der Vorschau.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Zoom {
    Fit,
    Factor(f32),
}

/// Hintergrund hinter transparenten Pixeln.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backdrop {
    Checker,
    White,
    Black,
}

/// Ergebnis für die Anzeige (Texturen im UI-Thread angelegt).
pub struct Shown {
    pub key: u64,
    pub before: TextureHandle,
    pub after: Option<TextureHandle>,
    pub before_size: (u32, u32),
    pub after_size: (u32, u32),
    pub bytes_before: u64,
    pub bytes_after: Option<u64>,
    pub ms: u64,
    pub warnings: Vec<String>,
}

struct Lite {
    before: ColorImage,
    after: Option<ColorImage>,
    before_size: (u32, u32),
    after_size: (u32, u32),
    bytes_before: u64,
    ms: u64,
    warnings: Vec<String>,
}

enum Msg {
    Quick(u64, Result<Lite, String>),
    Full {
        generation: u64,
        bytes_after: u64,
        after_size: (u32, u32),
        warnings: Vec<String>,
    },
    Done(u64),
}

/// Zustand der Vorschau.
pub struct PreviewState {
    generation: Arc<AtomicU64>,
    /// Schlüssel der zuletzt angefragten Rechnung (Datei + Rezept).
    requested: Option<u64>,
    /// Schlüssel, der gerechnet werden soll, und seit wann er sich nicht mehr ändert.
    pending: Option<(u64, f64)>,
    rx: Option<Receiver<Msg>>,
    running: bool,
    pub shown: Option<Shown>,
    pub error: Option<String>,
    pub zoom: Zoom,
    pub pan: egui::Vec2,
    /// Lage des Teilers im Vergleich (0…1).
    pub split: f32,
    pub backdrop: Backdrop,
    stale: bool,
    /// Zeitleiste für Animationen/Videos (BATCH-7).
    pub timeline: super::timeline::TimelineState,
}

impl Default for PreviewState {
    fn default() -> Self {
        PreviewState {
            generation: Arc::new(AtomicU64::new(0)),
            requested: None,
            pending: None,
            rx: None,
            running: false,
            shown: None,
            error: None,
            zoom: Zoom::Fit,
            pan: egui::Vec2::ZERO,
            split: 0.5,
            backdrop: Backdrop::Checker,
            stale: true,
            timeline: Default::default(),
        }
    }
}

impl PreviewState {
    /// Rezept oder Datei geändert: neu rechnen (entprellt).
    pub fn invalidate(&mut self) {
        self.stale = true;
    }
    /// Laufende Rechnung abbrechen.
    pub fn cancel(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
    }
    /// Wird gerechnet oder wartet eine Rechnung?
    pub fn busy(&self) -> bool {
        self.running || self.pending.is_some()
    }
    /// Aktuelle Generation (Tests).
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }
}

/// Verhältnis Nachher/Vorher der Dateigröße für die Schätzung im Trockenlauf (SPEC §2.6: „aus der
/// Vorschaudatei hochgerechnet“). Nur Dateischritte: 1 (die Datei wird kopiert). Sonst aus der fertig
/// kodierten Vorschau, wenn sie zum aktuellen Rezept gehört; `None`, solange sie noch rechnet.
pub fn size_ratio(s: &Studio) -> Option<f64> {
    if fp_batch::pipeline::is_file_only(&s.recipe, &StepRegistry::global()) {
        return Some(1.0);
    }
    let sh = s.preview.shown.as_ref()?;
    let info = &s.files.get(s.selected?)?.info;
    let after = sh.bytes_after?;
    (sh.key == key_for(&s.recipe, info) && sh.bytes_before > 0)
        .then(|| after as f64 / sh.bytes_before as f64)
}

fn key_for(recipe: &Recipe, info: &ItemInfo) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    info.path.hash(&mut h);
    info.modified.hash(&mut h);
    info.bytes.hash(&mut h);
    // ohne Ziel/Konflikt: die ändern das Bild nicht
    let mut r = recipe.clone();
    r.output.target = Default::default();
    r.output.conflict = Default::default();
    r.name.clear();
    r.to_json().hash(&mut h);
    h.finish()
}

/// Plant die Rechnung (nach jedem Bild aufgerufen).
pub fn schedule(ctx: &egui::Context, s: &mut Studio, now: f64) {
    let Some(sel) = s.selected.filter(|&i| i < s.files.len()) else {
        s.preview.shown = None;
        s.preview.error = None;
        s.preview.pending = None;
        return;
    };
    let info = s.files[sel].info.clone();
    let key = key_for(&s.recipe, &info);
    if s.preview.requested == Some(key) {
        s.preview.stale = false;
        s.preview.pending = None;
        return;
    }
    match s.preview.pending {
        Some((k, since)) if k == key => {
            // erster Aufruf ohne Vorschau: sofort rechnen, sonst entprellt
            if now - since >= DEBOUNCE || s.preview.shown.is_none() {
                start(ctx, s, key, info);
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(
                    ((DEBOUNCE - (now - since)) * 1000.0).max(10.0) as u64,
                ));
            }
        }
        _ => {
            s.preview.pending = Some((key, now));
            ctx.request_repaint_after(std::time::Duration::from_millis((DEBOUNCE * 1000.0) as u64));
        }
    }
}

fn start(ctx: &egui::Context, s: &mut Studio, key: u64, info: ItemInfo) {
    let generation = s.preview.generation.fetch_add(1, Ordering::SeqCst) + 1;
    s.preview.requested = Some(key);
    s.preview.pending = None;
    s.preview.running = true;
    s.preview.stale = false;
    let (tx, rx) = channel();
    s.preview.rx = Some(rx);
    let recipe = s.recipe.clone();
    let gen_arc = Arc::clone(&s.preview.generation);
    let ctx2 = ctx.clone();
    let lang = fp_i18n::current_language();
    let spawned = std::thread::Builder::new()
        .name("fp-batch-preview".into())
        .spawn(move || {
            fp_i18n::with_language(&lang, || {
                let reg = StepRegistry::global();
                let mut opts = PreviewOptions {
                    cancel: CancelToken::for_generation(Arc::clone(&gen_arc), generation),
                    ..PreviewOptions::default()
                };
                match fp_batch::preview(&recipe, &info, &reg, &opts) {
                    Ok(r) => {
                        let lite = Lite {
                            before: super::thumbs::to_color_image(&r.before, 1024),
                            after: r
                                .after
                                .as_ref()
                                .map(|a| super::thumbs::to_color_image(a, 1024)),
                            before_size: r.before_size,
                            after_size: r.after_size,
                            bytes_before: r.bytes_before,
                            ms: r.ms,
                            warnings: r.warnings.clone(),
                        };
                        let _ = tx.send(Msg::Quick(generation, Ok(lite)));
                        ctx2.request_repaint();
                        // volle Größe kodieren → echte Dateigröße „Nachher“
                        opts.encode_full = true;
                        if let Ok(full) = fp_batch::preview(&recipe, &info, &reg, &opts)
                            && let Some(b) = full.bytes_after
                        {
                            let _ = tx.send(Msg::Full {
                                generation,
                                bytes_after: b,
                                after_size: full.after_size,
                                warnings: full.warnings,
                            });
                        }
                    }
                    Err(e) if e.is_cancelled() => {}
                    Err(e) => {
                        let _ = tx.send(Msg::Quick(generation, Err(e.to_string())));
                    }
                }
                let _ = tx.send(Msg::Done(generation));
                ctx2.request_repaint();
            });
        });
    if spawned.is_err() {
        s.preview.running = false;
    }
}

/// Ergebnisse übernehmen (nur die aktuelle Generation; ältere werden verworfen).
pub fn poll(ctx: &egui::Context, s: &mut Studio) {
    let cur = s.preview.generation.load(Ordering::SeqCst);
    let Some(rx) = s.preview.rx.as_ref() else {
        return;
    };
    let msgs: Vec<Msg> = rx.try_iter().collect();
    for m in msgs {
        match m {
            Msg::Quick(g, res) if g == cur => match res {
                Ok(l) => {
                    let before = ctx.load_texture(
                        "fp-batch-preview-before",
                        l.before,
                        TextureOptions::LINEAR,
                    );
                    let after = l.after.map(|a| {
                        ctx.load_texture("fp-batch-preview-after", a, TextureOptions::LINEAR)
                    });
                    let key = s.preview.requested.unwrap_or(0);
                    s.preview.shown = Some(Shown {
                        key,
                        before,
                        after,
                        before_size: l.before_size,
                        after_size: l.after_size,
                        bytes_before: l.bytes_before,
                        bytes_after: None,
                        ms: l.ms,
                        warnings: l.warnings,
                    });
                    s.preview.error = None;
                }
                Err(e) => {
                    s.preview.shown = None;
                    s.preview.error = Some(e);
                }
            },
            Msg::Full {
                generation,
                bytes_after,
                after_size,
                warnings,
            } if generation == cur => {
                if let Some(sh) = s.preview.shown.as_mut() {
                    sh.bytes_after = Some(bytes_after);
                    sh.after_size = after_size;
                    for w in warnings {
                        if !sh.warnings.contains(&w) {
                            sh.warnings.push(w);
                        }
                    }
                }
            }
            Msg::Done(g) if g == cur => s.preview.running = false,
            _ => {}
        }
    }
}

/// „−91 %“ bzw. „+12 %“.
pub fn percent(before: u64, after: u64) -> String {
    if before == 0 {
        return String::new();
    }
    let p = (after as f64 / before as f64 - 1.0) * 100.0;
    let sign = if p < 0.0 { "−" } else { "+" };
    format!("{sign}{} %", fp_i18n::fmt_number(p.abs().round(), 0, 0))
}

/// Zeichnet die Karte.
pub fn show(ui: &mut Ui, st: &mut AppState, s: &mut Studio, rect: Rect, now: f64) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let col = kit::accent(&theme, 2);
    let p = ui.painter().clone();
    let vis = s.visible();
    let pos = s
        .selected
        .and_then(|sel| vis.iter().position(|&i| i == sel));
    let nav = match pos {
        Some(k) => fp_i18n::tr!("batch-ui-preview-nav", i = k + 1, n = vis.len()),
        None => String::new(),
    };
    let inner = kit::card(
        &p,
        rect,
        &theme,
        col,
        RowIcon::Kit(KitIcon::Image),
        &t("batch-ui-preview-title"),
        (!nav.is_empty()).then_some(nav.as_str()),
    );
    if inner.height() < 20.0 {
        return;
    }
    // Reiter
    let errors = s
        .files
        .iter()
        .filter(|f| f.status == super::EntryStatus::Failed)
        .count();
    let mut tabs = vec![
        (Tab::Before, t("batch-ui-tab-before")),
        (Tab::After, t("batch-ui-tab-after")),
        (Tab::Compare, t("batch-ui-tab-compare")),
        (Tab::Plan, t("batch-ui-tab-plan")),
    ];
    if errors > 0 {
        tabs.push((Tab::Errors, fp_i18n::tr!("batch-ui-tab-errors", n = errors)));
    }
    if s.view_tab == Tab::Errors && errors == 0 {
        s.view_tab = Tab::Compare;
    }
    let tr = Rect::from_min_size(inner.min, vec2(inner.width(), 26.0));
    let labels: Vec<String> = tabs.iter().map(|x| x.1.clone()).collect();
    let cur = tabs.iter().position(|x| x.0 == s.view_tab).unwrap_or(2);
    if let Some(k) = kit::segmented(
        ui,
        tr,
        Id::new("fp-batch-preview-tabs"),
        &labels,
        cur,
        false,
    ) {
        s.view_tab = tabs[k].0;
    }
    let body = Rect::from_min_max(pos2(inner.left(), tr.bottom() + sp::S), inner.max);
    if s.show_summary && s.report.is_some() {
        super::report_ui::summary_card(ui, st, s, body, now);
        return;
    }
    match s.view_tab {
        Tab::Plan => super::report_ui::plan_view(ui, s, body),
        Tab::Errors => super::report_ui::errors_view(ui, s, body),
        _ => image_view(ui, s, body, &vis, pos, &theme),
    }
    let _ = c;
}

fn image_view(
    ui: &mut Ui,
    s: &mut Studio,
    body: Rect,
    vis: &[usize],
    pos: Option<usize>,
    theme: &Theme,
) {
    let c = &theme.colors;
    let p = ui.painter().clone();
    // unten: Angaben (3 Zeilen) + Navigationszeile
    let line = 16.0;
    let warn_h = s
        .preview
        .shown
        .as_ref()
        .map(|sh| if sh.warnings.is_empty() { 0.0 } else { line })
        .unwrap_or(0.0);
    let info_h = 2.0 * line + warn_h + sp::XS;
    let nav_h = kit::BTN_H;
    let nav = Rect::from_min_max(pos2(body.left(), body.bottom() - nav_h), body.max);
    let info = Rect::from_min_max(
        pos2(body.left(), nav.top() - sp::S - info_h),
        pos2(body.right(), nav.top() - sp::S),
    );
    // Zeitleiste (BATCH-7) über den Angaben, wenn die Datei eine Animation ist
    super::timeline::update(ui.ctx(), s);
    let tl_h = if s.preview.timeline.visible() && body.height() > 3.0 * super::timeline::AREA_HEIGHT
    {
        super::timeline::AREA_HEIGHT + sp::S
    } else {
        0.0
    };
    let tl_rect = Rect::from_min_max(
        pos2(body.left(), info.top() - tl_h),
        pos2(body.right(), info.top() - sp::S),
    );
    let img = Rect::from_min_max(body.min, pos2(body.right(), info.top() - sp::S - tl_h));
    if tl_h > 0.0 {
        super::timeline::show_in_preview(ui, s, tl_rect);
    }
    // Navigationszeile: ◀ ▶, Zoom, Hintergrund
    nav_row(ui, s, nav, vis, pos, theme);
    if img.height() < 24.0 {
        return;
    }
    // Bildfläche
    let cr = CornerRadius::same(6);
    p.rect_filled(
        img,
        cr,
        if theme.glow {
            c.frame_bg
        } else {
            egui::Color32::from_gray(if theme.kind == crate::theme::ThemeKind::Normal {
                236
            } else {
                30
            })
        },
    );
    let p_img = p.with_clip_rect(img.shrink(1.0).intersect(ui.clip_rect()));
    if s.selected.is_none() {
        let title = t(if s.files.is_empty() {
            "batch-ui-preview-empty"
        } else {
            "batch-ui-preview-pick"
        });
        let h = ui_kit::empty_state_height(ui.ctx(), &title, None, img.width());
        ui_kit::empty_state_at(
            &p_img,
            Rect::from_center_size(img.center(), vec2(img.width(), h)),
            KitIcon::Image,
            &title,
            None,
            theme,
        );
        return;
    }
    if let Some(e) = s.preview.error.clone() {
        let title = t("batch-ui-preview-error");
        let h = ui_kit::empty_state_height(ui.ctx(), &title, Some(&e), img.width() - 16.0);
        ui_kit::empty_state_at(
            &p_img,
            Rect::from_center_size(img.center(), vec2(img.width() - 16.0, h)),
            KitIcon::Warning,
            &title,
            Some(&e),
            theme,
        );
        return;
    }
    let Some(sh) = s.preview.shown.as_ref() else {
        let title = t("batch-ui-preview-computing");
        let h = ui_kit::empty_state_height(ui.ctx(), &title, None, img.width());
        ui_kit::empty_state_at(
            &p_img,
            Rect::from_center_size(img.center(), vec2(img.width(), h)),
            KitIcon::Image,
            &title,
            None,
            theme,
        );
        return;
    };
    // Zoom und Verschieben
    let resp = ui.interact(
        img,
        Id::new("fp-batch-preview-img"),
        Sense::click_and_drag(),
    );
    let shown_tex = match s.view_tab {
        crate::batch::preview::Tab::Before => &sh.before,
        _ => sh.after.as_ref().unwrap_or(&sh.before),
    };
    let tex_size = shown_tex.size_vec2();
    let full = vec2(
        if s.view_tab == Tab::Before {
            sh.before_size.0
        } else {
            sh.after_size.0
        } as f32,
        if s.view_tab == Tab::Before {
            sh.before_size.1
        } else {
            sh.after_size.1
        } as f32,
    );
    let fit = ((img.width() - 16.0) / full.x.max(1.0))
        .min((img.height() - 16.0) / full.y.max(1.0))
        .clamp(0.01, 1.0);
    let mut scale = match s.preview.zoom {
        Zoom::Fit => fit,
        Zoom::Factor(f) => f,
    };
    if resp.hovered() {
        let (scroll, ctrl) = ui.input(|i| (i.smooth_scroll_delta.y, i.modifiers.command));
        let _ = ctrl;
        if scroll.abs() > 0.5 {
            let old = scale;
            scale = (scale * if scroll > 0.0 { 1.15 } else { 1.0 / 1.15 }).clamp(0.02, 16.0);
            if let Some(m) = ui.input(|i| i.pointer.hover_pos()) {
                let rel = m - (img.center() + s.preview.pan);
                s.preview.pan -= rel * (scale / old - 1.0);
            }
            s.preview.zoom = Zoom::Factor(scale);
        }
    }
    if resp.dragged() && s.view_tab != Tab::Compare
        || (resp.dragged() && ui.input(|i| i.modifiers.shift))
    {
        s.preview.pan += resp.drag_delta();
    }
    if resp.double_clicked() {
        s.preview.zoom = Zoom::Fit;
        s.preview.pan = egui::Vec2::ZERO;
    }
    if s.preview.zoom == Zoom::Fit {
        s.preview.pan = egui::Vec2::ZERO;
    }
    let disp = full * scale;
    let r_img = Rect::from_center_size(img.center() + s.preview.pan, disp);
    let backdrop = |pp: &egui::Painter, r: Rect| match s.preview.backdrop {
        Backdrop::Checker => crate::widgets::checker(pp, r, 8.0, theme),
        Backdrop::White => {
            pp.rect_filled(r, 0.0, Color32::WHITE);
        }
        Backdrop::Black => {
            pp.rect_filled(r, 0.0, Color32::BLACK);
        }
    };
    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    let _ = tex_size;
    if s.view_tab == Tab::Compare && sh.after.is_some() {
        // Vergleich: links vorher, rechts nachher, Teiler ziehbar
        let before_disp = vec2(sh.before_size.0 as f32, sh.before_size.1 as f32) * scale;
        let r_before = Rect::from_center_size(img.center() + s.preview.pan, before_disp);
        let split_x = img.left() + img.width() * s.preview.split;
        let left = Rect::from_min_max(img.min, pos2(split_x, img.bottom()));
        let right = Rect::from_min_max(pos2(split_x, img.top()), img.max);
        let pl = p_img.with_clip_rect(left.intersect(p_img.clip_rect()));
        let pr = p_img.with_clip_rect(right.intersect(p_img.clip_rect()));
        backdrop(&pl, r_before.intersect(img));
        pl.image(sh.before.id(), r_before, uv, Color32::WHITE);
        if let Some(a) = &sh.after {
            backdrop(&pr, r_img.intersect(img));
            pr.image(a.id(), r_img, uv, Color32::WHITE);
        }
        // Teiler
        let handle =
            Rect::from_center_size(pos2(split_x, img.center().y), vec2(14.0, img.height()));
        let hresp = ui.interact(handle, Id::new("fp-batch-preview-split"), Sense::drag());
        hresp.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Slider, true, t("batch-ui-preview-split"))
        });
        if hresp.hovered() || hresp.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        if hresp.dragged() {
            s.preview.split = ((split_x + hresp.drag_delta().x - img.left())
                / img.width().max(1.0))
            .clamp(0.02, 0.98);
        } else if resp.dragged()
            && !ui.input(|i| i.modifiers.shift)
            && let Some(pp) = ui.input(|i| i.pointer.interact_pos())
        {
            s.preview.split = ((pp.x - img.left()) / img.width().max(1.0)).clamp(0.02, 0.98);
        }
        p_img.vline(split_x, img.y_range(), Stroke::new(2.0, Color32::WHITE));
        p_img.vline(
            split_x + 1.0,
            img.y_range(),
            Stroke::new(1.0, with_alpha(Color32::BLACK, 90)),
        );
        p_img.circle_filled(pos2(split_x, img.center().y), 10.0, Color32::WHITE);
        p_img.circle_stroke(
            pos2(split_x, img.center().y),
            10.0,
            Stroke::new(1.0, with_alpha(Color32::BLACK, 90)),
        );
        let gs = Stroke::new(1.5, Color32::from_gray(60));
        let lp = pos2(split_x - 3.5, img.center().y);
        p_img.line_segment([lp + vec2(1.5, -4.0), lp + vec2(-2.0, 0.0)], gs);
        p_img.line_segment([lp + vec2(-2.0, 0.0), lp + vec2(1.5, 4.0)], gs);
        let rp = pos2(split_x + 3.5, img.center().y);
        p_img.line_segment([rp + vec2(-1.5, -4.0), rp + vec2(2.0, 0.0)], gs);
        p_img.line_segment([rp + vec2(2.0, 0.0), rp + vec2(-1.5, 4.0)], gs);
        // Beschriftung „Vorher“/„Nachher“
        let f = theme.font(type_scale::CAPTION);
        for (txt, x, al) in [
            (t("batch-ui-tab-before"), img.left() + 8.0, Align2::LEFT_TOP),
            (
                t("batch-ui-tab-after"),
                img.right() - 8.0,
                Align2::RIGHT_TOP,
            ),
        ] {
            let w = ui_kit::text_w(ui.ctx(), &txt, &f) + 10.0;
            let br = if al == Align2::LEFT_TOP {
                Rect::from_min_size(pos2(x, img.top() + 8.0), vec2(w, 18.0))
            } else {
                Rect::from_min_size(pos2(x - w, img.top() + 8.0), vec2(w, 18.0))
            };
            p_img.rect_filled(br, CornerRadius::same(4), with_alpha(Color32::BLACK, 150));
            p_img.text(
                br.center(),
                Align2::CENTER_CENTER,
                txt,
                f.clone(),
                Color32::WHITE,
            );
        }
    } else {
        backdrop(&p_img, r_img.intersect(img));
        p_img.image(shown_tex.id(), r_img, uv, Color32::WHITE);
    }
    if s.preview.busy() {
        // kleiner Kreis oben rechts: rechnet
        let center = pos2(img.right() - 14.0, img.top() + 14.0);
        p_img.circle_filled(center, 9.0, with_alpha(Color32::BLACK, 120));
        kit::status_mark(
            &p_img,
            center,
            super::EntryStatus::Running,
            theme,
            ui.input(|i| i.time),
        );
    }
    // Angaben
    let f = theme.font(type_scale::CAPTION + 0.5);
    let fb = theme.bold(type_scale::CAPTION + 0.5);
    let before_txt = fp_i18n::tr!(
        "batch-ui-preview-before-info",
        w = sh.before_size.0,
        h = sh.before_size.1,
        size = fp_i18n::fmt_size(sh.bytes_before)
    );
    let after_size = match sh.bytes_after {
        Some(b) => format!("{} ({})", fp_i18n::fmt_size(b), percent(sh.bytes_before, b)),
        None => format!("≈ {}", t("batch-ui-preview-size-pending")),
    };
    let after_txt = fp_i18n::tr!(
        "batch-ui-preview-after-info",
        w = sh.after_size.0,
        h = sh.after_size.1,
        size = after_size
    );
    let time_txt = fp_i18n::tr!(
        "batch-ui-preview-time",
        s = fp_i18n::fmt_number(sh.ms as f64 / 1000.0, 1, 1)
    );
    let tw = ui_kit::text_w(ui.ctx(), &time_txt, &f);
    kit::text_left(
        &p,
        info.left(),
        info.top() + line / 2.0,
        info.width(),
        &before_txt,
        f.clone(),
        kit::weak(theme),
    );
    let room = (info.width() - tw - sp::S).max(40.0);
    kit::text_left(
        &p,
        info.left(),
        info.top() + line * 1.5,
        room,
        &after_txt,
        fb,
        c.text,
    );
    if room + tw + sp::S <= info.width() + 0.5 {
        p.text(
            pos2(info.right(), info.top() + line * 1.5),
            Align2::RIGHT_CENTER,
            time_txt,
            f.clone(),
            kit::weak(theme),
        );
    }
    if let Some(w) = sh.warnings.first() {
        let ir = Rect::from_min_size(
            pos2(info.left(), info.top() + line * 2.5 - 6.0),
            vec2(12.0, 12.0),
        );
        ui_kit::paint_icon(
            &p,
            ir,
            KitIcon::Warning,
            &crate::icons::IconStyle::new(theme, None, true),
        );
        let txt = if sh.warnings.len() > 1 {
            format!("{w} (+{})", sh.warnings.len() - 1)
        } else {
            w.clone()
        };
        kit::text_left(
            &p,
            ir.right() + sp::XS,
            info.top() + line * 2.5,
            info.width() - 16.0,
            &txt,
            f,
            kit::weak(theme),
        );
        let wr = Rect::from_min_size(
            pos2(info.left(), info.top() + line * 2.0),
            vec2(info.width(), line),
        );
        ui.interact(wr, Id::new("fp-batch-preview-warn"), Sense::hover())
            .on_hover_text(sh.warnings.join("\n"));
    }
}

fn nav_row(ui: &mut Ui, s: &mut Studio, r: Rect, vis: &[usize], pos: Option<usize>, theme: &Theme) {
    let _ = theme;
    let prev_n = t("batch-ui-preview-prev");
    let next_n = t("batch-ui-preview-next");
    let b = Rect::from_min_size(r.min, vec2(26.0, r.height()));
    let n = vis.len();
    if kit::button(
        ui,
        b,
        Id::new("fp-batch-prev"),
        "",
        Some(RowIcon::Icon(crate::icons::Icon::PrevFrame)),
        kit::BtnKind::Secondary,
        pos.is_some_and(|k| k > 0),
        &prev_n,
    )
    .on_hover_text(prev_n.clone())
    .clicked()
        && let Some(k) = pos
    {
        s.selected = Some(vis[k - 1]);
        s.scroll_to_selected = true;
    }
    let b2 = Rect::from_min_size(pos2(b.right() + sp::XS, r.top()), vec2(26.0, r.height()));
    if kit::button(
        ui,
        b2,
        Id::new("fp-batch-next"),
        "",
        Some(RowIcon::Icon(crate::icons::Icon::NextFrame)),
        kit::BtnKind::Secondary,
        pos.is_some_and(|k| k + 1 < n),
        &next_n,
    )
    .on_hover_text(next_n.clone())
    .clicked()
        && let Some(k) = pos
    {
        s.selected = Some(vis[k + 1]);
        s.scroll_to_selected = true;
    }
    // Zoom und Hintergrund rechts
    let zw = 104.0f32.min((r.width() - 60.0) / 2.0).max(40.0);
    let zr = Rect::from_min_size(
        pos2(r.right() - 2.0 * zw - sp::XS, r.top()),
        vec2(zw, r.height()),
    );
    let br = Rect::from_min_size(pos2(r.right() - zw, r.top()), vec2(zw, r.height()));
    if zr.left() < b2.right() + sp::S {
        return;
    }
    let zoom_label = |z: Zoom| match z {
        Zoom::Fit => t("batch-ui-zoom-fit"),
        Zoom::Factor(f) => format!("{} %", fp_i18n::fmt_number((f * 100.0) as f64, 0, 0)),
    };
    let mut zc = ui.new_child(egui::UiBuilder::new().max_rect(zr));
    crate::theme::field_widgets(&mut zc);
    let mut zoom = s.preview.zoom;
    egui::ComboBox::from_id_salt("fp-batch-zoom")
        .selected_text(zoom_label(zoom))
        .width(zw - 8.0)
        .show_ui(&mut zc, |ui| {
            for z in [Zoom::Fit, Zoom::Factor(1.0), Zoom::Factor(2.0)] {
                ui.selectable_value(&mut zoom, z, zoom_label(z));
            }
        })
        .response
        .on_hover_text(t("batch-ui-zoom"));
    if zoom != s.preview.zoom {
        s.preview.zoom = zoom;
        s.preview.pan = egui::Vec2::ZERO;
    }
    let bd_label = |b: Backdrop| {
        t(match b {
            Backdrop::Checker => "batch-ui-bg-checker",
            Backdrop::White => "batch-ui-bg-white",
            Backdrop::Black => "batch-ui-bg-black",
        })
    };
    let mut bc = ui.new_child(egui::UiBuilder::new().max_rect(br));
    crate::theme::field_widgets(&mut bc);
    let mut bd = s.preview.backdrop;
    egui::ComboBox::from_id_salt("fp-batch-backdrop")
        .selected_text(bd_label(bd))
        .width(zw - 8.0)
        .show_ui(&mut bc, |ui| {
            for b in [Backdrop::Checker, Backdrop::White, Backdrop::Black] {
                ui.selectable_value(&mut bd, b, bd_label(b));
            }
        })
        .response
        .on_hover_text(t("batch-ui-bg"));
    s.preview.backdrop = bd;
}

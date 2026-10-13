//! Zeitleiste der Batch-Vorschau für Animationen und Videos (SPEC-BATCH §3.8 `anim.trim`,
//! BATCH-7). Zeichnet die Frames als Abschnitte nach ihrer Dauer, zwei Griffe für Anfang und Ende
//! (auf Frame-Grenzen gerastet) und entfernte Frames durchgestrichen. Ein Klick auf einen Abschnitt
//! entfernt das Frame bzw. holt es zurück.
//!
//! Das Modell ist [`fp_batch::steps::anim::Timeline`]; die Oberfläche schreibt bei `changed` das
//! Ergebnis von [`Timeline::trim_params`] in den Schritt `anim.trim` des Rezepts (ein Rezept-Schritt,
//! Parameter ersetzt, nie ein zweiter Schritt; ohne Schritt wird er vorn eingefügt). Die Zeitleiste
//! zeigt die Frames, die beim Schritt `anim.trim` ankommen (Rezept bis davor, im Hintergrund mit
//! kleinen Proxy-Bildern gerechnet). Eingehängt in `preview.rs` (`update` + `show_in_preview`).

use super::Studio;
use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Rect, Sense, Stroke, StrokeKind, Ui};
use fp_batch::item::{Item, ItemKind};
use fp_batch::steps::anim::{Timeline, TRIM};
use fp_batch::video::format_time_ms;
use fp_batch::{CancelToken, ItemInfo, PreviewOptions, Recipe, StepRegistry};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;

/// Platzbedarf unter dem Vorschaubild (Kopfzeile, Band, Zeiten).
pub const AREA_HEIGHT: f32 = 22.0 + BAR_HEIGHT + 18.0;

/// Zeitleisten-Zustand der Vorschau (in `preview::PreviewState`).
#[derive(Default)]
pub struct TimelineState {
    generation: Arc<AtomicU64>,
    /// Schlüssel (Datei + Schritte vor `anim.trim`) der angefragten bzw. gezeigten Dauern.
    requested: Option<u64>,
    rx: Option<Receiver<(u64, Option<Vec<u32>>)>>,
    /// Modell (None = keine Animation bzw. noch nicht gerechnet).
    pub model: Option<Timeline>,
}

impl TimelineState {
    /// Wird eine Zeitleiste gezeigt?
    pub fn visible(&self) -> bool {
        self.model.as_ref().is_some_and(|m| !m.is_empty())
    }
    fn clear(&mut self) {
        if self.requested.is_some() {
            self.generation.fetch_add(1, Ordering::SeqCst);
        }
        self.requested = None;
        self.rx = None;
        self.model = None;
    }
}

/// Index des ersten eingeschalteten Schritts `anim.trim`.
pub fn trim_index(recipe: &Recipe) -> Option<usize> {
    recipe.enabled_steps().find(|(_, s)| s.id == TRIM).map(|(i, _)| i)
}

/// Rezept bis vor `anim.trim` (ohne Schritt: leer) – dessen Ergebnis zeigt die Zeitleiste.
pub fn input_recipe(recipe: &Recipe) -> Recipe {
    let mut r = recipe.clone();
    r.steps.truncate(trim_index(recipe).unwrap_or(0));
    r.output = Default::default();
    r.name.clear();
    r
}

fn key_for(info: &ItemInfo, input: &Recipe) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    info.path.hash(&mut h);
    info.modified.hash(&mut h);
    info.bytes.hash(&mut h);
    input.to_json().hash(&mut h);
    h.finish()
}

/// Parameter des Trim-Schritts (mit Vorgaben) aus dem Rezept.
fn trim_params(recipe: &Recipe) -> Option<serde_json::Value> {
    let i = trim_index(recipe)?;
    let reg = StepRegistry::global();
    let defs = reg.get(TRIM).map(|s| s.params()).unwrap_or_default();
    Some(fp_batch::step::with_defaults(&defs, &recipe.steps[i].params))
}

/// Dauern holen (im Hintergrund, abbrechbar) und das Modell mit dem Rezept abgleichen. Einmal je
/// Bild aus der Vorschau aufrufen.
pub fn update(ctx: &egui::Context, s: &mut Studio) {
    let Some(info) = s.selected.and_then(|i| s.files.get(i)).map(|f| f.info.clone()) else {
        s.preview.timeline.clear();
        return;
    };
    // GIF/WebP nennen die Zahl der Frames nicht im Kopf (frames = 0): erst die Rechnung entscheidet
    if !(matches!(info.kind, ItemKind::Animation | ItemKind::Video) || info.frames == 0) {
        s.preview.timeline.clear();
        return;
    }
    let input = input_recipe(&s.recipe);
    let key = key_for(&info, &input);
    let st = &mut s.preview.timeline;
    // Ergebnis abholen
    if let Some(rx) = st.rx.as_ref() {
        for (k, delays) in rx.try_iter().collect::<Vec<_>>() {
            if Some(k) == st.requested {
                st.model = delays.map(Timeline::new);
                st.rx = None;
            }
        }
    }
    if st.requested != Some(key) {
        // neue Datei bzw. andere Schritte davor: Dauern neu rechnen
        let generation = st.generation.fetch_add(1, Ordering::SeqCst) + 1;
        st.requested = Some(key);
        st.model = None;
        let (tx, rx) = channel();
        st.rx = Some(rx);
        let gen_arc = Arc::clone(&st.generation);
        let ctx2 = ctx.clone();
        let lang = fp_i18n::current_language();
        let _ = std::thread::Builder::new().name("fp-batch-timeline".into()).spawn(move || {
            fp_i18n::with_language(&lang, || {
                let opts = PreviewOptions { max_frames: usize::MAX, max_size_animation: 48, cancel: CancelToken::for_generation(gen_arc, generation), ..PreviewOptions::default() };
                let delays = match fp_batch::preview(&input, &info, &StepRegistry::global(), &opts) {
                    Ok(r) => match r.after_item {
                        Some(Item::Animation(a)) => Some(a.frames.iter().map(|f| f.delay_ms).collect()),
                        _ => None,
                    },
                    Err(_) => None,
                };
                let _ = tx.send((key, delays));
                ctx2.request_repaint();
            });
        });
        return;
    }
    // Modell an das Rezept angleichen (Rückgängig/Wiederholen, Formular der Karte)
    let params = trim_params(&s.recipe);
    if let Some(m) = s.preview.timeline.model.as_mut() {
        let mut want = Timeline::new(m.delays.clone());
        if let Some(p) = &params {
            want.apply_params(p);
        }
        if *m != want {
            *m = want;
        }
    }
}

/// Zeichnet die Zeitleiste in `rect` (unter dem Vorschaubild) und schreibt Änderungen in den
/// Schritt `anim.trim` (ohne Schritt: wird vorn eingefügt). Eine Geste = ein Rückgängig-Schritt.
pub fn show_in_preview(ui: &mut Ui, s: &mut Studio, rect: Rect) {
    let Some(mut model) = s.preview.timeline.model.clone() else { return };
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(egui::Layout::top_down(egui::Align::Min)));
    let r = timeline(&mut child, &mut model, None);
    if r.changed {
        let params = model.trim_params();
        let k = trim_index(&s.recipe);
        s.edit("anim.trim:timeline", |rec| match k {
            Some(i) => {
                let mut p = rec.steps[i].params.as_object().cloned().unwrap_or_default();
                for (key, v) in params.as_object().into_iter().flatten() {
                    p.insert(key.clone(), v.clone());
                }
                rec.steps[i].params = serde_json::Value::Object(p);
                true
            }
            None => {
                let n = rec.add_step(TRIM, params.clone());
                rec.move_step(n, 0);
                true
            }
        });
        s.preview.timeline.model = Some(model);
    }
    if r.gesture_end {
        s.end_gesture();
    }
}

/// Höhe des Bandes in Punkten.
pub const BAR_HEIGHT: f32 = 34.0;
/// Fangbereich der Griffe in Punkten.
const HANDLE_GRAB: f32 = 8.0;

/// Ergebnis eines Frames.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TimelineResponse {
    /// Anfang, Ende oder entfernte Frames haben sich geändert (Rezept aktualisieren).
    pub changed: bool,
    /// Frame unter dem Zeiger.
    pub hovered_frame: Option<usize>,
    /// Geste beendet (Griff losgelassen, Klick, Zurücksetzen): nächster Zug = neuer Rückgängig-Schritt.
    pub gesture_end: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Handle {
    Start,
    End,
}

/// Lage der Frames auf dem Band (Zeit → x).
struct Layout {
    rect: Rect,
    total: f64,
    starts: Vec<u64>,
}

impl Layout {
    fn new(rect: Rect, tl: &Timeline) -> Layout {
        let starts = (0..=tl.len()).map(|i| tl.start_of(i)).collect();
        Layout { rect, total: tl.total_ms().max(1) as f64, starts }
    }
    fn x(&self, ms: u64) -> f32 {
        self.rect.left() + (ms as f64 / self.total) as f32 * self.rect.width()
    }
    fn ms(&self, x: f32) -> u64 {
        (((x - self.rect.left()) / self.rect.width().max(1.0)).clamp(0.0, 1.0) as f64 * self.total).round() as u64
    }
    fn segment(&self, i: usize) -> Rect {
        let (a, b) = (self.x(self.starts[i]), self.x(self.starts[i + 1]));
        Rect::from_min_max(pos2(a, self.rect.top()), pos2(b.max(a + 2.0), self.rect.bottom()))
    }
    fn frame_at_x(&self, x: f32) -> Option<usize> {
        (0..self.starts.len().saturating_sub(1)).find(|&i| {
            let s = self.segment(i);
            x >= s.left() && x < s.right()
        })
    }
}

/// Zeichnet die Zeitleiste und verarbeitet Ziehen/Klicken. `current` = gerade gezeigtes Frame
/// (Wiedergabe der Vorschau), wird markiert.
pub fn timeline(ui: &mut Ui, tl: &mut Timeline, current: Option<usize>) -> TimelineResponse {
    let mut out = TimelineResponse::default();
    if tl.is_empty() {
        return out;
    }
    let (kept, ms) = tl.kept();
    ui.horizontal(|ui| {
        ui.strong(fp_i18n::t("batch-anim-timeline-title"));
        ui.label(fp_i18n::tr!("batch-anim-timeline-kept", kept = kept, total = tl.len(), duration = format_time_ms(ms)));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let untouched = *tl == Timeline::new(tl.delays.clone());
            if ui.add_enabled(!untouched, egui::Button::new(fp_i18n::t("batch-anim-timeline-reset"))).clicked() {
                *tl = Timeline::new(tl.delays.clone());
                out.changed = true;
                out.gesture_end = true;
            }
        });
    });

    let width = ui.available_width().max(60.0);
    let (rect, resp) = ui.allocate_exact_size(vec2(width, BAR_HEIGHT), Sense::click_and_drag());
    let lay = Layout::new(rect.shrink2(vec2(HANDLE_GRAB / 2.0, 0.0)), tl);
    let drag_id = resp.id.with("handle");

    // Ziehen der Griffe
    // Griff nach dem Druckpunkt wählen (egui meldet den Zugbeginn erst nach ein paar Pixeln)
    if resp.drag_started()
        && let Some(p) = ui.input(|i| i.pointer.press_origin()).or_else(|| resp.interact_pointer_pos())
    {
        let (ds, de) = ((p.x - lay.x(tl.start_ms)).abs(), (p.x - lay.x(tl.end_ms)).abs());
        let h = if ds <= de && ds <= HANDLE_GRAB { Some(Handle::Start) } else if de <= HANDLE_GRAB { Some(Handle::End) } else { None };
        ui.memory_mut(|m| m.data.insert_temp(drag_id, h));
    }
    let dragging: Option<Handle> = ui.memory(|m| m.data.get_temp(drag_id)).flatten();
    if resp.dragged()
        && let (Some(h), Some(p)) = (dragging, resp.interact_pointer_pos())
    {
        let before = (tl.start_ms, tl.end_ms);
        match h {
            Handle::Start => tl.set_start(lay.ms(p.x)),
            Handle::End => tl.set_end(lay.ms(p.x)),
        }
        out.changed |= before != (tl.start_ms, tl.end_ms);
    }
    if resp.drag_stopped() {
        ui.memory_mut(|m| m.data.remove::<Option<Handle>>(drag_id));
        out.gesture_end = true;
    }
    // Klick auf ein Frame: entfernen bzw. zurückholen
    if resp.clicked()
        && let Some(i) = resp.interact_pointer_pos().and_then(|p| lay.frame_at_x(p.x))
    {
        tl.toggle_removed(i);
        out.changed = true;
        out.gesture_end = true;
    }
    out.hovered_frame = resp.hover_pos().and_then(|p| lay.frame_at_x(p.x));

    // Zeichnen
    let v = ui.visuals().clone();
    let painter = ui.painter_at(rect.expand(2.0));
    painter.rect_filled(rect, CornerRadius::same(3), v.extreme_bg_color);
    let accent = v.selection.bg_fill;
    let dim = v.widgets.inactive.bg_fill;
    for i in 0..tl.len() {
        let seg = lay.segment(i).shrink2(vec2(0.5, 3.0));
        let kept = tl.is_kept(i);
        let removed = tl.removed.get(i).copied().unwrap_or(false);
        let fill = if kept { accent } else { dim };
        painter.rect_filled(seg, CornerRadius::same(1), if out.hovered_frame == Some(i) { fill.gamma_multiply(1.25) } else { fill });
        if removed && seg.width() >= 4.0 {
            let s = Stroke::new(1.5, v.error_fg_color);
            painter.line_segment([seg.left_top(), seg.right_bottom()], s);
            painter.line_segment([seg.left_bottom(), seg.right_top()], s);
        }
        if current == Some(i) {
            painter.rect_stroke(seg, CornerRadius::same(1), Stroke::new(2.0, v.strong_text_color()), StrokeKind::Outside);
        }
    }
    // Außerhalb des Fensters abdunkeln, Griffe zeichnen
    let shade = Color32::from_black_alpha(90);
    let (xs, xe) = (lay.x(tl.start_ms), lay.x(tl.end_ms));
    if xs > rect.left() {
        painter.rect_filled(Rect::from_min_max(rect.left_top(), pos2(xs, rect.bottom())), CornerRadius::ZERO, shade);
    }
    if xe < rect.right() {
        painter.rect_filled(Rect::from_min_max(pos2(xe, rect.top()), rect.right_bottom()), CornerRadius::ZERO, shade);
    }
    for (x, h) in [(xs, Handle::Start), (xe, Handle::End)] {
        let active = dragging == Some(h);
        let r = Rect::from_center_size(pos2(x, rect.center().y), vec2(if active { 6.0 } else { 4.0 }, rect.height() + 2.0));
        painter.rect_filled(r, CornerRadius::same(2), v.strong_text_color());
    }

    // Zeiten unter dem Band
    let font = FontId::proportional(11.0);
    let label_y = rect.bottom() + 2.0;
    ui.allocate_exact_size(vec2(width, 14.0), Sense::hover());
    let p2 = ui.painter();
    p2.text(pos2(rect.left(), label_y), Align2::LEFT_TOP, format!("{} {}", fp_i18n::t("batch-anim-timeline-start"), format_time_ms(tl.start_ms)), font.clone(), v.weak_text_color());
    p2.text(pos2(rect.right(), label_y), Align2::RIGHT_TOP, format!("{} {}", fp_i18n::t("batch-anim-timeline-end"), format_time_ms(tl.end_ms)), font, v.weak_text_color());

    // Hinweise
    match out.hovered_frame {
        Some(i) => {
            let key = if tl.removed.get(i).copied().unwrap_or(false) { "batch-anim-timeline-restore" } else { "batch-anim-timeline-remove" };
            resp.on_hover_text(format!("{}\n{}", fp_i18n::tr!("batch-anim-timeline-frame", n = i + 1, ms = tl.delays[i]), fp_i18n::t(key)));
        }
        None => {
            resp.on_hover_text(fp_i18n::t("batch-anim-timeline-hint"));
        }
    }
    out
}

//! Fortschrittsanzeige der KI-Werkzeuge (Objektauswahl, Motiv auswählen, Kante verfeinern).
//!
//! Ruhige, schwebende Karte über der Leinwand (unten mittig), im Stil der Dialoge aus `ui_kit`:
//! Werkzeugsymbol, Werkzeugname als Titel, Modellname klein und gedämpft darunter, die Stufen der
//! Rechnung (nur die, die der Code wirklich unterscheidet: Modell laden, Bild analysieren, Maske
//! berechnen), ein Fortschrittsbalken (bei unbekanntem Fortschritt eine ruhige Wanderbewegung),
//! verstrichene Zeit, Restzeit nur, wenn sie verlässlich ist, und „Abbrechen“.
//!
//! * Erscheint erst nach [`SHOW_DELAY`] – schnelle Läufe blitzen nicht auf – und blendet weich
//!   ein und am Ende kurz aus.
//! * Feste Größe je Lauf (keine Breitenänderung durch Zahlen, kein Springen).

use crate::icons::{self, Icon, IconStyle};
use crate::theme::{Theme, mix, with_alpha};
use crate::ui_kit::{self, sp, type_scale};
use egui::{Align2, Color32, CornerRadius, Id, Rect, Sense, Shape, Stroke, StrokeKind, pos2, vec2};
use fp_engine::commands::ai::fp_ai::{Progress, Stage};

/// Erst nach dieser Zeit (s) erscheint die Anzeige.
pub const SHOW_DELAY: f32 = 0.3;
/// Einblenden (s).
const FADE_IN: f32 = 0.15;
/// Ausblenden nach dem Ende (s).
pub const FADE_OUT: f32 = 0.25;
/// Breite der Karte.
pub const WIDTH: f32 = 380.0;
/// Abstand zum unteren Fensterrand.
const BOTTOM: f32 = 48.0;

/// Schätzung der Restzeit (nur in der Stufe „Bild analysieren“, wo es echten Fortschritt gibt).
#[derive(Clone, Debug, Default)]
pub struct Estimate {
    /// Beginn der Analyse (Laufzeit in s) und Fortschritt dort.
    start: Option<(f32, f32)>,
    /// Angezeigte Restzeit (s) und Zeitpunkt der letzten Aktualisierung.
    shown: Option<(f32, f32)>,
}

impl Estimate {
    /// Restzeit in s, wenn sie verlässlich ist: mindestens 1 s Analyse und 10 % Fortschritt;
    /// höchstens einmal je Sekunde aktualisiert (kein Zappeln).
    pub fn update(&mut self, stage: Stage, p: f32, elapsed: f32) -> Option<f32> {
        if stage != Stage::Analyze {
            if stage == Stage::Mask {
                self.shown = None;
            }
            return None;
        }
        let (t0, p0) = *self.start.get_or_insert((elapsed, p));
        let (dt, dp) = (elapsed - t0, p - p0);
        if dt < 1.0 || dp < 0.1 {
            return None;
        }
        let rest = dt * (1.0 - p).max(0.0) / dp;
        match self.shown {
            Some((_, at)) if elapsed - at < 1.0 => {}
            _ => self.shown = Some((rest, elapsed)),
        }
        // zwischen zwei Aktualisierungen gleichmäßig herunterzählen
        self.shown
            .map(|(r, at)| (r - (elapsed - at)).max(0.0))
            .filter(|r| *r >= 2.0)
    }
}

/// Was die Anzeige eines Laufs zeigt (fest für den ganzen Lauf).
#[derive(Clone, Debug)]
pub struct ProgressView {
    /// Werkzeugname („Motiv auswählen“).
    pub title: String,
    /// Modellname („BiRefNet lite (allgemein, schnell)“).
    pub model: String,
    /// Werkzeugsymbol.
    pub icon: Icon,
    /// Stufen dieses Laufs in Reihenfolge.
    pub stages: Vec<Stage>,
    /// Restzeit-Schätzung.
    pub estimate: Estimate,
}

/// Zustand beim Ausblenden nach dem Ende eines sichtbaren Laufs.
#[derive(Clone, Debug)]
pub struct Fading {
    pub view: ProgressView,
    pub elapsed: f32,
    /// UI clock (`egui::InputState::time`) when the fade started.
    pub since: f64,
}

fn stage_index(stages: &[Stage], s: Stage) -> usize {
    let s = if s == Stage::Start {
        stages.first().copied().unwrap_or(Stage::Analyze)
    } else {
        s
    };
    stages.iter().position(|x| *x == s).unwrap_or(0)
}

fn stage_text(s: Stage) -> String {
    fp_i18n::t(match s {
        Stage::Start | Stage::LoadModel => "dlg-ai-stage-load",
        Stage::Analyze => "dlg-ai-stage-analyze",
        Stage::Mask => "dlg-ai-stage-mask",
    })
}

fn step_text(s: Stage) -> String {
    fp_i18n::t(match s {
        Stage::Start | Stage::LoadModel => "dlg-ai-step-load",
        Stage::Analyze => "dlg-ai-step-analyze",
        Stage::Mask => "dlg-ai-step-mask",
    })
}

fn seconds(v: f32) -> String {
    fp_i18n::tr!("dlg-ai-elapsed", time = fp_i18n::fmt_decimal(v as f64, 1))
}

fn remaining(r: f32) -> String {
    if r >= 90.0 {
        fp_i18n::tr!("dlg-ai-remaining-min", n = (r / 60.0).round() as u64)
    } else if r >= 10.0 {
        fp_i18n::tr!("dlg-ai-remaining-s", n = ((r / 5.0).ceil() * 5.0) as u64)
    } else {
        fp_i18n::tr!("dlg-ai-remaining-s", n = r.ceil() as u64)
    }
}

/// Höhe der Karte für eine Ansicht.
pub fn height(view: &ProgressView) -> f32 {
    let stepper = if view.stages.len() >= 2 {
        STEP_H + sp::M
    } else {
        0.0
    };
    sp::L + 32.0 + sp::M + stepper + 18.0 + sp::XS + BAR_H + sp::M + sp::BUTTON_H + sp::L
}

const STEP_H: f32 = 16.0;
const BAR_H: f32 = 6.0;

/// Zustand eines Frames.
pub struct Frame<'a> {
    pub view: &'a mut ProgressView,
    pub progress: Option<&'a Progress>,
    pub elapsed: f32,
    pub cancelling: bool,
    /// Ausblenden: 1 → 0 (dann ohne Knopf, Balken voll).
    pub fade_out: Option<f32>,
    /// Bereich, über dem die Karte unten mittig schwebt (Leinwand; sonst das Fenster).
    pub anchor: Option<Rect>,
}

/// Zeichnet die Karte; `true` = „Abbrechen“ geklickt. Vor [`SHOW_DELAY`] wird nichts gezeichnet
/// (Rückgabe `false`); liefert in `shown` zurück, ob sie sichtbar war.
pub fn show(ctx: &egui::Context, f: Frame, shown: &mut bool) -> bool {
    *shown = false;
    let opacity = match f.fade_out {
        Some(o) => o.clamp(0.0, 1.0),
        None if f.elapsed < SHOW_DELAY => {
            ctx.request_repaint_after(std::time::Duration::from_secs_f32(
                (SHOW_DELAY - f.elapsed).max(0.01),
            ));
            return false;
        }
        None => ((f.elapsed - SHOW_DELAY) / FADE_IN).clamp(0.0, 1.0),
    };
    if opacity <= 0.0 {
        return false;
    }
    *shown = true;
    let theme = Theme::current(ctx);
    let c = theme.colors.clone();
    let screen = ctx.content_rect();
    let area = f
        .anchor
        .filter(|a| a.width() >= 260.0 && a.height() >= 200.0)
        .map(|a| a.intersect(screen))
        .unwrap_or(screen);
    let w = WIDTH.min(area.width() - 2.0 * sp::L).max(240.0);
    let h = height(f.view);
    let bottom = if f.anchor.is_some() && area != screen {
        sp::XL
    } else {
        BOTTOM
    };
    let pos = pos2(
        (area.center().x - w / 2.0).round(),
        (area.bottom() - bottom - h).max(area.top()).round(),
    );
    let (stage, frac) = match (f.progress, f.fade_out) {
        (_, Some(_)) => (Stage::Mask, 1.0),
        (Some(pr), None) => (pr.stage(), pr.get()),
        (None, None) => (Stage::Analyze, 0.0),
    };
    let rest = if f.fade_out.is_none() && !f.cancelling {
        f.view.estimate.update(stage, frac, f.elapsed)
    } else {
        None
    };
    let mut cancel = false;
    egui::Area::new(Id::new("fp-ai-progress"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .interactable(f.fade_out.is_none())
        .show(ctx, |ui| {
            ui.multiply_opacity(opacity);
            let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
            resp.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Other, true, f.view.title.as_str())
            });
            let p = ui.painter();
            let radius = CornerRadius::same(8);
            // Schatten und Fläche
            for (i, a) in [(6.0, 18u8), (3.0, 26), (1.0, 34)] {
                p.rect_filled(
                    rect.translate(vec2(0.0, 2.0)).expand(i),
                    CornerRadius::same(8 + i as u8),
                    Color32::from_black_alpha(if ui_kit::is_light(&theme) { a / 2 } else { a }),
                );
            }
            p.rect_filled(rect, radius, ui_kit::card_bg(&theme));
            let border = if theme.glow {
                with_alpha(c.focus, 150)
            } else {
                c.list_border
            };
            p.rect_stroke(rect, radius, Stroke::new(1.0, border), StrokeKind::Inside);
            if theme.glow {
                p.rect_stroke(
                    rect.expand(1.5),
                    CornerRadius::same(10),
                    Stroke::new(2.0, with_alpha(c.focus, 40)),
                    StrokeKind::Outside,
                );
            }
            let inner = rect.shrink(sp::L);
            // Kopf: Symbol in getönter Kachel, Titel, Modell
            let ir = Rect::from_min_size(inner.min, vec2(32.0, 32.0));
            p.rect_filled(ir, CornerRadius::same(6), ui_kit::tint(&theme, c.focus));
            icons::paint(
                p,
                ir.shrink(6.0),
                f.view.icon,
                &IconStyle::new(&theme, None, true),
            );
            let tx = ir.right() + sp::M;
            let tw = inner.right() - tx;
            ui_kit::paint_text_fit(
                p,
                Rect::from_min_size(pos2(tx, inner.top() - 1.0), vec2(tw, 18.0)),
                &f.view.title,
                theme.bold(type_scale::TITLE),
                c.text,
            );
            ui_kit::paint_text_fit(
                p,
                Rect::from_min_size(pos2(tx, inner.top() + 18.0), vec2(tw, 14.0)),
                &f.view.model,
                theme.font(type_scale::CAPTION),
                c.text_weak,
            );
            let mut y = ir.bottom() + sp::M;
            // Stufen
            let cur = stage_index(&f.view.stages, stage);
            if f.view.stages.len() >= 2 {
                paint_steps(
                    ui,
                    Rect::from_min_size(pos2(inner.left(), y), vec2(inner.width(), STEP_H)),
                    &f.view.stages,
                    cur,
                    f.fade_out.is_some(),
                    &theme,
                );
                y += STEP_H + sp::M;
            }
            // Statuszeile: Stufe links, Prozent rechts
            let determinate = stage == Stage::Analyze && f.fade_out.is_none();
            let status = if f.fade_out.is_some() {
                fp_i18n::t("dlg-ai-stage-done")
            } else if f.cancelling {
                fp_i18n::t("dlg-ai-cancelling")
            } else {
                stage_text(stage)
            };
            let line = Rect::from_min_size(pos2(inner.left(), y), vec2(inner.width(), 18.0));
            let pct = if determinate || f.fade_out.is_some() {
                fp_i18n::tr!(
                    "dlg-ai-percent",
                    n = (frac * 100.0).floor().min(100.0) as u32
                )
            } else {
                String::new()
            };
            let font = theme.font(type_scale::BODY);
            let pw = ui_kit::text_w(ui.ctx(), &fp_i18n::tr!("dlg-ai-percent", n = 100), &font);
            ui_kit::paint_text_fit(
                p,
                Rect::from_min_max(line.min, pos2(line.right() - pw - sp::S, line.bottom())),
                &status,
                font.clone(),
                c.text,
            );
            if !pct.is_empty() {
                p.text(
                    pos2(line.right(), line.center().y),
                    Align2::RIGHT_CENTER,
                    pct,
                    font,
                    c.text_weak,
                );
            }
            y = line.bottom() + sp::XS;
            // Balken
            let bar = Rect::from_min_size(pos2(inner.left(), y), vec2(inner.width(), BAR_H));
            let t = ui.input(|i| i.time) as f32;
            paint_bar(
                p,
                bar,
                if determinate || f.fade_out.is_some() {
                    Some(frac)
                } else {
                    None
                },
                t,
                &theme,
            );
            if !determinate && f.fade_out.is_none() {
                ui.ctx().request_repaint();
            }
            y = bar.bottom() + sp::M;
            // Fußzeile: Zeit links, Abbrechen rechts
            let foot =
                Rect::from_min_size(pos2(inner.left(), y), vec2(inner.width(), sp::BUTTON_H));
            let label = fp_i18n::t("btn-cancel");
            let bw = crate::dialogs::buttons_w(ui.ctx(), &[&label], sp::BUTTON_MIN_W);
            let mut time = seconds(f.elapsed);
            if let Some(r) = rest {
                time.push_str(" · ");
                time.push_str(&remaining(r));
            }
            ui_kit::paint_text_fit(
                p,
                Rect::from_min_max(foot.min, pos2(foot.right() - bw - sp::S, foot.bottom())),
                &time,
                theme.font(type_scale::CAPTION),
                c.text_weak,
            );
            if f.fade_out.is_none() {
                let br = Rect::from_min_size(
                    pos2(foot.right() - bw, foot.top()),
                    vec2(bw, sp::BUTTON_H),
                );
                if crate::dialogs::push_button(ui, br, &label, false, !f.cancelling) {
                    cancel = true;
                }
            }
        });
    if f.fade_out.is_none() {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    } else {
        ctx.request_repaint();
    }
    cancel
}

/// Stufenleiste: erledigt = gefüllter Kreis mit Haken, aktuell = Ring mit Punkt, offen = Ring.
fn paint_steps(
    ui: &egui::Ui,
    r: Rect,
    stages: &[Stage],
    cur: usize,
    all_done: bool,
    theme: &Theme,
) {
    let c = &theme.colors;
    let p = ui.painter();
    let font = theme.font(type_scale::CAPTION);
    let n = stages.len();
    let gap = sp::S;
    let seg = (r.width() - gap * (n as f32 - 1.0)) / n as f32;
    for (i, s) in stages.iter().enumerate() {
        let x = r.left() + i as f32 * (seg + gap);
        let cc = pos2(x + 6.0, r.center().y);
        let done = all_done || i < cur;
        let active = !all_done && i == cur;
        if done {
            p.circle_filled(cc, 6.0, c.focus);
            let ck = Color32::WHITE;
            p.add(Shape::line(
                vec![
                    cc + vec2(-3.0, 0.2),
                    cc + vec2(-0.8, 2.4),
                    cc + vec2(3.0, -2.2),
                ],
                Stroke::new(1.5, ck),
            ));
        } else if active {
            p.circle_stroke(cc, 5.5, Stroke::new(1.5, c.focus));
            p.circle_filled(cc, 2.5, c.focus);
        } else {
            p.circle_stroke(
                cc,
                5.5,
                Stroke::new(1.0, mix(c.text_weak, ui_kit::card_bg(theme), 0.35)),
            );
        }
        let col = if active {
            c.text
        } else if done {
            c.text_weak
        } else {
            mix(c.text_weak, ui_kit::card_bg(theme), 0.25)
        };
        let f = if active {
            theme.bold(type_scale::CAPTION)
        } else {
            font.clone()
        };
        ui_kit::paint_text_fit(
            p,
            Rect::from_min_max(pos2(cc.x + 10.0, r.top()), pos2(x + seg, r.bottom())),
            &step_text(*s),
            f,
            col,
        );
    }
}

/// Balken: Spur + Füllung in Akzentfarbe; ohne Wert eine ruhige, weich hin- und herlaufende
/// Bewegung (Periode 2,4 s).
fn paint_bar(p: &egui::Painter, bar: Rect, value: Option<f32>, time: f32, theme: &Theme) {
    let c = &theme.colors;
    let r = CornerRadius::same((BAR_H / 2.0) as u8);
    p.rect_filled(
        bar,
        r,
        mix(
            c.field_bg,
            c.focus,
            if ui_kit::is_light(theme) { 0.10 } else { 0.14 },
        ),
    );
    match value {
        Some(v) => {
            let v = v.clamp(0.0, 1.0);
            if v > 0.0 {
                let fr =
                    Rect::from_min_size(bar.min, vec2((bar.width() * v).max(BAR_H), bar.height()));
                if theme.glow {
                    p.rect_filled(
                        fr.expand(1.5),
                        CornerRadius::same(5),
                        with_alpha(c.focus, 50),
                    );
                }
                p.rect_filled(fr, r, c.focus);
            }
        }
        None => {
            let seg = bar.width() * 0.28;
            let ph = (time / 2.4).fract();
            // weiches Hin und Her (Kosinus), Abschnitt bleibt ganz im Balken
            let k = 0.5 - 0.5 * (ph * std::f32::consts::TAU).cos();
            let x = bar.left() + k * (bar.width() - seg);
            let s = Rect::from_min_size(pos2(x, bar.top()), vec2(seg, bar.height()));
            if theme.glow {
                p.rect_filled(
                    s.expand(1.5),
                    CornerRadius::same(5),
                    with_alpha(c.focus, 50),
                );
            }
            p.rect_filled(s, r, c.focus);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_only_when_reliable() {
        let mut e = Estimate::default();
        assert!(e.update(Stage::LoadModel, 0.0, 0.5).is_none());
        assert!(e.update(Stage::Analyze, 0.0, 1.0).is_none());
        // nach 0,5 s und 5 %: noch nicht
        assert!(e.update(Stage::Analyze, 0.05, 1.5).is_none());
        // nach 2 s und 20 %: ≈ 8 s Rest
        let r = e.update(Stage::Analyze, 0.2, 3.0).unwrap_or(0.0);
        assert!((r - 8.0).abs() < 0.01, "{r}");
        // eine halbe Sekunde später: nur heruntergezählt, nicht neu geschätzt
        let r2 = e.update(Stage::Analyze, 0.5, 3.5).unwrap_or(0.0);
        assert!((r2 - 7.5).abs() < 0.01, "{r2}");
        // kurz vor Schluss (< 2 s) keine Anzeige
        assert!(e.update(Stage::Analyze, 0.99, 9.0).is_none());
    }
}

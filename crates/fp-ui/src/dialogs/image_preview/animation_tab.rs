//! Registerkarte „Animation“ der Bildvorschau und das Abspielen (re\17-bildvorschau.md §4, §6).
//!
//! - Statusliste mit Auge (im Export sichtbar), Name, Löschmethode als Buchstabe und Verzögerung;
//!   nur die erste Zeile zeigt die Einheit („20 /100 Sek.“).
//! - Darüber Löschmethoden-Popup (Papierkorb mit ▾) und Verzögerung des gewählten Status (Uhr).
//! - „Laufzeit: 1.70 Sek.“ = Summe der Verzögerungen der exportierten Status.
//! - Einmal abspielen / Schleife mit Anzahl („Endlos“, 1 … 20, editierbar), „Jeden Status
//!   zuschneiden“, „Unterschiede zwischen den Status speichern“.
//! - Alles gehört zur Einstellung des **aktiven** Feldes (`anim` der Exportoptionen); der gewählte
//!   Status ist der gemeinsame Status aller Felder ([`super::ImagePreview::frame`]).
//! - Grau, wenn das Dokument nur einen Status hat.

use super::super::tl;
use super::ImagePreview;
use super::color_table;
use crate::icons::Icon;
use crate::inspector::fields;
use crate::theme::Theme;
use crate::tips::HoverTip;
use egui::{Align2, Id, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use fp_export::options::OptimizeSettings;
use fp_i18n::t;
use fp_model::FrameInfo;

/// Löschmethoden in der Reihenfolge des Popups (Wert `disposal` 0 … 3).
pub const DISPOSAL_KEYS: [&str; 4] = [
    "image-preview-disposal-unspecified",
    "image-preview-disposal-none",
    "image-preview-disposal-background",
    "image-preview-disposal-previous",
];
/// Kurzbuchstaben der Liste.
pub const DISPOSAL_LETTERS: [&str; 4] = [
    "image-preview-disposal-letter-unspecified",
    "image-preview-disposal-letter-none",
    "image-preview-disposal-letter-background",
    "image-preview-disposal-letter-previous",
];
/// Einträge der Wiederholungsliste nach „Endlos“.
pub const REPEATS: [i32; 7] = [1, 2, 3, 4, 5, 10, 20];
const CHECK_KEYS: [&str; 2] = [
    "image-preview-anim-crop-each",
    "image-preview-anim-save-differences",
];
/// Zeilenhöhe der Statusliste.
pub const ROW_H: f32 = 17.0;

/// Eigenschaften des Status `i` (fehlende Einträge = Vorgabe: 7/100 s, sichtbar, nicht angegeben).
pub fn frame_info(s: &OptimizeSettings, i: usize) -> FrameInfo {
    s.anim.frames.get(i).cloned().unwrap_or_default()
}

/// Eigenschaften des Status `i` zum Ändern (die Liste wächst bei Bedarf mit Vorgaben).
pub fn frame_info_mut(s: &mut OptimizeSettings, i: usize) -> &mut FrameInfo {
    if s.anim.frames.len() <= i {
        s.anim.frames.resize_with(i + 1, FrameInfo::default);
    }
    &mut s.anim.frames[i]
}

/// Summe der Verzögerungen der exportierten Status in 1/100 s.
pub fn total_delay(s: &OptimizeSettings, frames: usize) -> i64 {
    (0..frames)
        .map(|i| frame_info(s, i))
        .filter(|f| f.visible)
        .map(|f| f.delay.max(0) as i64)
        .sum()
}

/// „Laufzeit: 1.70 Sek.“ (zwei Nachkommastellen, Punkt fest, §4).
pub fn runtime_text(s: &OptimizeSettings, frames: usize) -> String {
    let n = total_delay(s, frames);
    fp_i18n::tr!(
        "image-preview-anim-runtime",
        seconds = format!("{}.{:02}", n / 100, n % 100)
    )
}

/// Text der Verzögerungsspalte: erste Zeile mit Einheit, die übrigen nur die Zahl.
pub fn delay_cell(i: usize, delay: i32) -> String {
    if i == 0 {
        fp_i18n::tr!("image-preview-anim-delay-first", delay = delay.to_string())
    } else {
        delay.to_string()
    }
}

/// Text der Wiederholungsliste für `loop_count` (0 = „Endlos“).
fn repeat_text(loop_count: i32) -> String {
    if loop_count <= 0 {
        t("image-preview-anim-forever")
    } else {
        loop_count.to_string()
    }
}

// ---------------------------------------------------------------------- Abspielen

/// Nächster exportierter Status nach `from` (rundum); ohne sichtbare Status der nächste.
pub fn next_frame(s: &OptimizeSettings, frames: usize, from: usize) -> usize {
    let n = frames.max(1);
    (1..=n)
        .map(|k| (from + k) % n)
        .find(|&i| frame_info(s, i).visible)
        .unwrap_or((from + 1) % n)
}

/// Abspielen weiterführen: nach der Verzögerung des gezeigten Status (Einstellung des aktiven
/// Feldes) zum nächsten exportierten Status. Zeitbasis ist die Zeit der Oberfläche.
///
/// FB-63: the next state is shown only once it is rendered in every visible pane (the panes
/// render the states ahead while playing, see `Pane::prefetch`); until then the shown state stays
/// and [`ImagePreview::play_wait`] names the state that is rendered (busy cue). Small documents
/// render a state well within its delay and play exactly as before.
pub fn tick(d: &mut ImagePreview, ctx: &egui::Context, base: &super::worker::JobInput) {
    d.play_wait = None;
    if !d.playing {
        d.next_tick = None;
        return;
    }
    if d.frames <= 1 {
        d.playing = false;
        return;
    }
    let now = ctx.input(|i| i.time);
    let s = &d.panes[d.active].settings;
    let delay = |f: usize| (frame_info(s, f).delay.max(1) as f64) / 100.0;
    let due = *d.next_tick.get_or_insert(now + delay(d.frame));
    if now + 1e-6 >= due {
        let next = next_frame(s, d.frames, d.frame);
        let next_delay = delay(next);
        if !d.frame_ready(base, next) {
            // the worker repaints when a job finishes; the timer is only a safety net
            d.play_wait = Some(next);
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            return;
        }
        d.frame = next;
        d.next_tick = Some(now + next_delay);
    }
    let wait = d.next_tick.map(|n| (n - now).max(0.0)).unwrap_or(0.0);
    ctx.request_repaint_after(std::time::Duration::from_secs_f64(wait.min(1.0)));
}

/// Status wählen (Knöpfe, Statusfeld, Liste): hält das Abspielen nicht an.
pub fn go_to(d: &mut ImagePreview, frame: usize) {
    d.frame = frame.min(d.frames.max(1) - 1);
    d.next_tick = None;
}

// ---------------------------------------------------------------------- Registerkarte

/// Uhr-Symbol vor dem Verzögerungsfeld.
fn paint_clock(theme: &Theme, p: &egui::Painter, c: egui::Pos2, enabled: bool) {
    let fg = if enabled {
        theme.colors.icon_fg
    } else {
        theme.colors.icon_disabled
    };
    p.circle_filled(c, 6.5, theme.colors.icon_paper);
    p.circle_stroke(c, 6.5, Stroke::new(1.0, fg));
    p.line_segment([c, c + vec2(0.0, -4.0)], Stroke::new(1.2, fg));
    p.line_segment([c, c + vec2(3.0, 0.0)], Stroke::new(1.2, fg));
}

/// Symbol „Einmal abspielen“: Pfeil auf einen Strich.
fn paint_once(theme: &Theme, p: &egui::Painter, r: Rect, enabled: bool) {
    let fg = if enabled {
        theme.colors.icon_fg
    } else {
        theme.colors.icon_disabled
    };
    let y = r.center().y;
    p.line_segment(
        [pos2(r.left() + 2.0, y), pos2(r.right() - 5.0, y)],
        Stroke::new(1.4, fg),
    );
    p.line(
        vec![
            pos2(r.right() - 8.0, y - 3.0),
            pos2(r.right() - 5.0, y),
            pos2(r.right() - 8.0, y + 3.0),
        ],
        Stroke::new(1.4, fg),
    );
    p.vline(r.right() - 2.5, (y - 4.5)..=(y + 4.5), Stroke::new(1.4, fg));
}

/// Zeichnet die Registerkarte „Animation“ in `page` (`inner` = Seite ohne Randabstand).
pub fn show(ui: &mut Ui, page: Rect, inner: Rect, d: &mut ImagePreview) {
    let theme = Theme::current(ui.ctx());
    let c = theme.colors.clone();
    let n = d.frames.max(1);
    let en = n > 1;
    let active = d.active;
    let sel = d.frame.min(n - 1);
    let (top, bottom, left, right) = (page.top(), page.bottom(), inner.left(), inner.right());

    // Kopfzeile: Löschmethode (Papierkorb ▾) und Verzögerung (Uhr + Zahlenfeld)
    let fr = Rect::from_min_size(pos2(right - 40.0, top + 14.0), vec2(40.0, 20.0));
    let delay = frame_info(&d.panes[active].settings, sel).delay;
    if let Some(v) = fields::number(
        ui,
        "pv-anim-delay",
        fr,
        None,
        Some(delay as f64),
        (0.0, 65535.0),
        0,
        en,
    ) {
        frame_info_mut(&mut d.panes[active].settings, sel).delay = v as i32;
    }
    tip(ui, "delay", fr, "image-preview-tip-frame-delay");
    paint_clock(
        &theme,
        ui.painter(),
        pos2(fr.left() - 12.0, fr.center().y),
        en,
    );
    let br = Rect::from_min_size(pos2(fr.left() - 24.0 - 31.0, top + 14.0), vec2(31.0, 20.0));
    let disposal = frame_info(&d.panes[active].settings, sel)
        .disposal
        .clamp(0, 3) as usize;
    let th = theme.clone();
    let resp = color_table::toggle_button(
        ui,
        "anim-disposal",
        br,
        false,
        en,
        &t("image-preview-tip-disposal"),
        |p, r, e| {
            color_table::paint_icon(&th, p, r.translate(vec2(-5.0, 0.0)), e, Icon::Trash);
            let fg = if e {
                th.colors.icon_fg
            } else {
                th.colors.icon_disabled
            };
            let a = pos2(r.right() + 4.0, r.center().y + 1.0);
            p.add(egui::Shape::convex_polygon(
                vec![
                    a + vec2(-3.0, -2.0),
                    a + vec2(3.0, -2.0),
                    a + vec2(0.0, 2.0),
                ],
                fg,
                Stroke::NONE,
            ));
        },
    );
    if en {
        egui::Popup::menu(&resp)
            .frame(crate::menu::menu_frame(&theme))
            .show(|ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                ui.set_min_width(170.0);
                for (k, key) in DISPOSAL_KEYS.iter().enumerate() {
                    if fields::list_item(ui, &t(key), k == disposal).clicked() {
                        frame_info_mut(&mut d.panes[active].settings, sel).disposal = k as i32;
                        ui.close();
                    }
                }
            });
    }

    // Statusliste: Auge, Name, Löschmethode, Verzögerung
    let list = Rect::from_min_max(
        pos2(left, top + 42.0),
        pos2(right, (bottom - 122.0).max(top + 42.0 + ROW_H * 2.0)),
    );
    ui.painter()
        .rect_filled(list, 0.0, if en { c.list_bg } else { c.frame_bg });
    ui.painter().rect_stroke(
        list,
        0.0,
        Stroke::new(1.0, c.list_border),
        StrokeKind::Inside,
    );
    let body = list.shrink(1.0);
    let letters: Vec<String> = tl(&DISPOSAL_LETTERS);
    let mut scroll_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(body)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    scroll_ui.set_clip_rect(body.intersect(ui.clip_rect()));
    egui::ScrollArea::vertical()
        .id_salt("pv-anim-list")
        .auto_shrink([false, false])
        .show(&mut scroll_ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            for i in 0..n {
                let (row, resp) = ui.allocate_exact_size(
                    vec2(body.width(), ROW_H),
                    if en { Sense::CLICK } else { Sense::hover() },
                );
                let info = frame_info(&d.panes[active].settings, i);
                let name = info
                    .name
                    .clone()
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| fp_i18n::tr!("panel-state-default-name", n = i + 1));
                resp.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::SelectableLabel,
                        en,
                        i == sel,
                        name.as_str(),
                    )
                });
                let selected = en && i == sel;
                if selected {
                    ui.painter().rect_filled(row, 0.0, c.list_selected_bg);
                }
                let txt = if !en {
                    c.text_disabled
                } else if selected {
                    c.list_selected_text
                } else {
                    c.text
                };
                // Auge: Status exportieren ja/nein
                let eye = Rect::from_min_size(row.min, vec2(18.0, ROW_H));
                let eresp = ui.interact(
                    eye,
                    Id::new(("pv-anim-eye", i)),
                    if en { Sense::CLICK } else { Sense::hover() },
                );
                eresp.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::Checkbox,
                        en,
                        info.visible,
                        fp_i18n::tr!("image-preview-anim-export-frame", n = i + 1),
                    )
                });
                if info.visible {
                    color_table::paint_icon(
                        &theme,
                        ui.painter(),
                        Rect::from_center_size(eye.center(), vec2(14.0, 14.0)),
                        en,
                        Icon::Eye,
                    );
                }
                ui.painter().vline(
                    eye.right() + 0.5,
                    row.y_range(),
                    Stroke::new(1.0, c.line_light),
                );
                let dcol = Rect::from_min_max(pos2(row.right() - 78.0, row.top()), row.max);
                let lcol = Rect::from_min_max(
                    pos2(dcol.left() - 18.0, row.top()),
                    pos2(dcol.left(), row.bottom()),
                );
                let font = theme.ui_font();
                crate::widgets::text_clipped(
                    ui.painter(),
                    pos2(eye.right() + 5.0, row.center().y),
                    Align2::LEFT_CENTER,
                    &name,
                    font.clone(),
                    txt,
                    (lcol.left() - eye.right() - 8.0).max(8.0),
                );
                let letter = letters
                    .get(info.disposal.clamp(0, 3) as usize)
                    .cloned()
                    .unwrap_or_default();
                ui.painter().text(
                    lcol.center(),
                    Align2::CENTER_CENTER,
                    letter,
                    font.clone(),
                    txt,
                );
                crate::widgets::text_clipped(
                    ui.painter(),
                    pos2(dcol.left() + 2.0, row.center().y),
                    Align2::LEFT_CENTER,
                    &delay_cell(i, info.delay),
                    font,
                    txt,
                    dcol.width() - 4.0,
                );
                if eresp.clicked() {
                    let f = frame_info_mut(&mut d.panes[active].settings, i);
                    f.visible = !f.visible;
                } else if resp.clicked() {
                    go_to(d, i);
                }
            }
        });

    // Laufzeit
    let rt = Rect::from_min_size(
        pos2(left + 14.0, list.bottom() + 4.0),
        vec2(right - left - 14.0, 18.0),
    );
    let rtext = runtime_text(&d.panes[active].settings, n);
    let rresp = ui.interact(rt, Id::new("pv-anim-runtime"), Sense::hover());
    rresp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, en, rtext.as_str()));
    ui.painter().text(
        rt.left_center(),
        Align2::LEFT_CENTER,
        &rtext,
        theme.ui_font(),
        if en { c.text } else { c.text_disabled },
    );
    rresp.hover_tip(t("image-preview-tip-runtime"));

    // Einmal abspielen / Schleife + Anzahl
    let s = &mut d.panes[active].settings;
    let y = list.bottom() + 28.0;
    let once = s.anim.loop_count < 0;
    let b1 = Rect::from_min_size(pos2(left, y), vec2(24.0, 22.0));
    let b2 = Rect::from_min_size(pos2(left + 26.0, y), vec2(24.0, 22.0));
    let th = theme.clone();
    if color_table::toggle_button(
        ui,
        "anim-once",
        b1,
        once,
        en,
        &t("image-preview-anim-play-once"),
        |p, r, e| paint_once(&th, p, r, e),
    )
    .clicked()
    {
        s.anim.loop_count = -1;
    }
    let th = theme.clone();
    if color_table::toggle_button(
        ui,
        "anim-loop",
        b2,
        !once,
        en,
        &t("image-preview-anim-loop"),
        |p, r, e| color_table::paint_icon(&th, p, r, e, Icon::Loop),
    )
    .clicked()
        && once
    {
        s.anim.loop_count = 0;
    }
    let mut items = vec![t("image-preview-anim-forever")];
    items.extend(REPEATS.iter().map(|v| v.to_string()));
    let cr = Rect::from_min_size(
        pos2(b2.right() + 10.0, y + 1.0),
        vec2(105.0f32.min(right - b2.right() - 10.0), 20.0),
    );
    let cur = if once {
        String::new()
    } else {
        repeat_text(s.anim.loop_count)
    };
    if let Some(v) = fields::combo_edit(ui, "pv-anim-repeat", cr, &cur, &items, en && !once) {
        if v.trim() == t("image-preview-anim-forever") || v.trim().is_empty() {
            s.anim.loop_count = 0;
        } else if let Some(k) = fields::parse_num(&v) {
            s.anim.loop_count = (k.round() as i32).clamp(0, 65535);
        }
    }
    tip(ui, "repeat", cr, "image-preview-tip-repeat");

    // Kästchen
    let cw = right - left - 2.0;
    let c1 = Rect::from_min_size(pos2(left + 1.0, y + 28.0), vec2(cw, 16.0));
    if let Some(b) = fields::checkbox(
        ui,
        "pv-anim-crop",
        c1,
        &t(CHECK_KEYS[0]),
        s.anim.auto_crop,
        en,
    ) {
        s.anim.auto_crop = b;
    }
    tip(ui, "crop-each", c1, "image-preview-tip-crop-each");
    let c2 = Rect::from_min_size(pos2(left + 1.0, y + 47.0), vec2(cw, 16.0));
    if let Some(b) = fields::checkbox(
        ui,
        "pv-anim-diff",
        c2,
        &t(CHECK_KEYS[1]),
        s.anim.auto_difference,
        en,
    ) {
        s.anim.auto_difference = b;
    }
    tip(
        ui,
        "save-differences",
        c2,
        "image-preview-tip-save-differences",
    );
}

/// Tooltip über einem Bedienelement, das selbst keinen zeigt.
fn tip(ui: &Ui, id: &str, r: Rect, key: &str) {
    let text = t(key);
    let resp = ui.interact(r, Id::new(("fp-ipv-anim-tip", id)), Sense::hover());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, text.as_str()));
    resp.hover_tip(text);
}

#[cfg(test)]
mod tests {
    use super::{OptimizeSettings, frame_info_mut, next_frame};

    /// FB-87: Image Preview playback skips the states excluded from export, in state order.
    #[test]
    fn playback_order_skips_excluded_states() {
        let mut s = OptimizeSettings::default();
        frame_info_mut(&mut s, 1).visible = false;
        frame_info_mut(&mut s, 3).visible = false;
        let mut order = vec![0];
        for _ in 0..4 {
            order.push(next_frame(&s, 5, *order.last().expect("state")));
        }
        assert_eq!(order, vec![0, 2, 4, 0, 2]);
    }
}

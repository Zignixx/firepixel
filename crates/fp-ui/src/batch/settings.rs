//! Batch-Einstellungen und Rückfrage beim Schließen (BATCH-11a).
//!
//! - **Einstellungen** (Banner ⋯ ▸ „Einstellungen…“): Überlagerung im Stil der Studio-Karten mit
//!   „Allgemein“ (Modus, Unterordner) und „Videos (ffmpeg)“: Zustand der ffmpeg-Suche
//!   (`fp_batch::video::ffmpeg::status`, im Hintergrund geprüft: Version, Pfad, Herkunft), Knöpfe
//!   „ffmpeg-Datei…“ (Einstellung `ffmpegPath` in `<Profil>\video.json`), „Automatisch suchen“,
//!   „Erneut prüfen“ und ein Hinweis, wenn ffmpeg fehlt. Firepixel lädt nichts herunter.
//! - **Rückfrage „Batch läuft noch – abbrechen und schließen?“** (SPEC §2.8, Befund BATCH-2) beim
//!   Schließen des Fensters ([`intercept_quit`], Haken in `Action::Quit`) und des Studios (×,
//!   Menü), solange ein Auftrag läuft.

use super::kit::{self, BtnKind};
use super::{Inbox, Mode, Studio};
use crate::app_state::{Action, AppState};
use crate::theme::{with_alpha, Theme};
use crate::ui_kit::{self, sp, type_scale, KitIcon, RowIcon, Tone};
use egui::{pos2, vec2, Align2, Color32, Id, Rect, Sense, Ui};
use fp_batch::video::ffmpeg::{self, FfmpegStatus};
use fp_i18n::t;
use std::sync::mpsc::{channel, Receiver};

/// Zustand der Einstellungen-Überlagerung.
#[derive(Default)]
pub struct SettingsState {
    /// Ergebnis der ffmpeg-Prüfung (None = wird geprüft).
    pub ffmpeg: Option<FfmpegStatus>,
    rx: Option<Receiver<FfmpegStatus>>,
}

impl SettingsState {
    /// Neu öffnen und ffmpeg im Hintergrund prüfen (startet einen Prozess `ffmpeg -version`, daher
    /// nie im Zeichnen).
    pub fn open(ctx: &egui::Context) -> SettingsState {
        let mut s = SettingsState::default();
        s.recheck(ctx);
        s
    }
    /// ffmpeg erneut suchen und prüfen.
    pub fn recheck(&mut self, ctx: &egui::Context) {
        let (tx, rx) = channel();
        self.ffmpeg = None;
        self.rx = Some(rx);
        let ctx = ctx.clone();
        let lang = fp_i18n::current_language();
        let _ = std::thread::Builder::new().name("fp-batch-ffmpeg-status".into()).spawn(move || {
            let st = fp_i18n::with_language(&lang, ffmpeg::status);
            let _ = tx.send(st);
            ctx.request_repaint();
        });
    }
    /// Läuft die Prüfung noch?
    pub fn checking(&self) -> bool {
        self.ffmpeg.is_none()
    }
    fn poll(&mut self) {
        if let Some(rx) = &self.rx
            && let Ok(st) = rx.try_recv()
        {
            self.ffmpeg = Some(st);
            self.rx = None;
        }
    }
}

/// Öffnet die Einstellungen (Banner-Menü).
pub fn open(ctx: &egui::Context, s: &mut Studio) {
    s.settings = Some(SettingsState::open(ctx));
}

/// Antwort des Datei-Dialogs „ffmpeg-Datei…“: speichern und neu prüfen.
pub fn set_ffmpeg_path(ctx: &egui::Context, s: &mut Studio, path: Option<&std::path::Path>, now: f64) {
    s.forget_ffmpeg();
    match ffmpeg::set_configured_path(path) {
        Ok(()) => s.notify(Tone::Success, t(if path.is_some() { "batch-settings-ffmpeg-saved" } else { "batch-settings-ffmpeg-cleared" }), now),
        Err(e) => s.notify(Tone::Error, e.to_string(), now),
    }
    if let Some(st) = s.settings.as_mut() {
        st.recheck(ctx);
    }
}

/// Text der Herkunft („im Systempfad gefunden“ …).
pub fn found_by_text(st: &FfmpegStatus) -> String {
    st.found_by.map(|f| t(&format!("batch-settings-ffmpeg-by-{}", f.name()))).unwrap_or_default()
}

/// Zeilenhöhe in der Überlagerung.
const ROW: f32 = 26.0;

/// Zeichnet die Einstellungen über dem Studio (Bereich `area`).
pub fn show(ui: &mut Ui, st: &mut AppState, s: &mut Studio, area: Rect, now: f64) {
    let Some(state) = s.settings.as_mut() else { return };
    state.poll();
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let layer = egui::LayerId::new(egui::Order::Foreground, Id::new("fp-batch-settings"));
    let painter = ui.ctx().layer_painter(layer);
    painter.rect_filled(area, 0.0, with_alpha(Color32::BLACK, if theme.glow { 160 } else { 110 }));
    let mut lay = ui.new_child(egui::UiBuilder::new().layer_id(layer).max_rect(area));
    // Klicks neben der Karte gehen nicht an das Studio darunter
    let _ = lay.interact(area, Id::new("fp-batch-settings-block"), Sense::click_and_drag());
    let w = 560.0f32.min(area.width() - 2.0 * sp::L).max(260.0);
    let hint = t("batch-settings-ffmpeg-hint");
    let missing = state.ffmpeg.as_ref().is_some_and(|f| !f.is_ok());
    // Höhe aus dem Inhalt des letzten Bildes (Hinweisblock umbricht je Sprache und Breite)
    let h_id = Id::new("fp-batch-settings-content-h");
    let content_h = ui.ctx().data(|m| m.get_temp::<f32>(h_id)).unwrap_or(18.0 + 2.0 * ROW + sp::M + 18.0 + 2.0 * ROW + 24.0);
    let h = (kit::HEAD_H + kit::PAD + content_h + sp::M + 26.0 + kit::PAD).min(area.height() - 2.0 * sp::M);
    let r = Rect::from_center_size(area.center(), vec2(w, h));
    let inner = kit::card(&painter, r, &theme, kit::accent(&theme, 1), RowIcon::Kit(KitIcon::Commands), &t("batch-settings-title"), None);
    let mut body = lay.new_child(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::top_down(egui::Align::Min)));
    body.set_clip_rect(inner.intersect(area));
    crate::theme::field_widgets(&mut body);
    let ui = &mut body;
    let label_w = ui_kit::label_width(ui.ctx(), &[&t("batch-settings-mode"), &t("batch-ui-recursive"), &t("batch-settings-ffmpeg")], 90.0).min(inner.width() * 0.4);
    // Allgemein
    let (cap, _) = ui.allocate_exact_size(vec2(inner.width(), 18.0), Sense::hover());
    ui_kit::section_at(ui.painter(), cap, &t("batch-settings-section-general"), &theme);
    let row = |ui: &mut Ui, label: &str| -> Rect {
        let (rr, _) = ui.allocate_exact_size(vec2(inner.width(), ROW), Sense::hover());
        let f = theme.font(type_scale::BODY);
        let shown = ui_kit::elide(ui.ctx(), label, &f, label_w - sp::XS);
        ui.painter().text(pos2(rr.left(), rr.center().y), Align2::LEFT_CENTER, shown, f, kit::weak(&theme));
        Rect::from_min_max(pos2(rr.left() + label_w + sp::S, rr.top()), rr.max)
    };
    let mr = row(ui, &t("batch-settings-mode"));
    let modes = vec![t("batch-ui-mode-beginner"), t("batch-ui-mode-pro")];
    let seg_w = (modes.iter().map(|m| kit::btn_w(ui.ctx(), m, false)).sum::<f32>() + 4.0).min(mr.width());
    let cur = if s.mode == Mode::Pro { 1 } else { 0 };
    if let Some(k) = kit::segmented(ui, Rect::from_min_size(pos2(mr.left(), mr.center().y - 11.0), vec2(seg_w, 22.0)), Id::new("fp-batch-settings-mode"), &modes, cur, false) {
        s.mode = if k == 1 { Mode::Pro } else { Mode::Beginner };
        s.show_more = false;
        s.files_changed();
        st.session.prefs.set(super::PREF_MODE, fp_engine::PrefValue::Str(if k == 1 { "pro".into() } else { "beginner".into() }));
        st.session.prefs_changed();
    }
    let rr = row(ui, &t("batch-ui-recursive"));
    let tresp = kit::toggle(ui, Rect::from_min_size(pos2(rr.left(), rr.center().y - 11.0), vec2(36.0, 22.0)), Id::new("fp-batch-settings-recursive"), s.recursive, &t("batch-ui-recursive"));
    if tresp.clicked() {
        s.recursive = !s.recursive;
        st.session.prefs.set(super::PREF_RECURSIVE, fp_engine::PrefValue::Bool(s.recursive));
        st.session.prefs_changed();
    }
    // Videos (ffmpeg)
    ui.add_space(sp::M - sp::XS);
    let (cap, _) = ui.allocate_exact_size(vec2(inner.width(), 18.0), Sense::hover());
    ui_kit::section_at(ui.painter(), cap, &t("batch-settings-section-video"), &theme);
    let fr = row(ui, &t("batch-settings-ffmpeg"));
    let Some(state) = s.settings.as_mut() else { return };
    let (status_text, ok, detail) = match &state.ffmpeg {
        None => (t("batch-settings-ffmpeg-checking"), None, String::new()),
        Some(f) if f.is_ok() => (fp_i18n::tr!("batch-settings-ffmpeg-ok", version = f.version.clone().unwrap_or_default()), Some(true), format!("{} · {}", found_by_text(f), f.path.as_ref().map(|p| p.display().to_string()).unwrap_or_default())),
        Some(f) => (t("batch-settings-ffmpeg-missing"), Some(false), f.path.as_ref().map(|p| format!("{} · {}", found_by_text(f), p.display())).unwrap_or_default()),
    };
    let ir = Rect::from_min_size(pos2(fr.left(), fr.center().y - 8.0), vec2(16.0, 16.0));
    match ok {
        Some(true) => ui_kit::paint_icon(ui.painter(), ir, KitIcon::Success, &crate::icons::IconStyle::new(&theme, None, true)),
        Some(false) => ui_kit::paint_icon(ui.painter(), ir, KitIcon::Warning, &crate::icons::IconStyle::new(&theme, None, true)),
        None => kit::status_mark(ui.painter(), ir.center(), super::EntryStatus::Running, &theme, now),
    }
    kit::text_left(ui.painter(), ir.right() + sp::XS + 2.0, fr.center().y, fr.right() - ir.right() - sp::S, &status_text, theme.bold(type_scale::BODY), c.text);
    if !detail.is_empty() {
        let dr = row(ui, "");
        let dresp = ui.interact(dr, Id::new("fp-batch-settings-ffmpeg-detail"), Sense::hover());
        let shown = kit::text_left(ui.painter(), dr.left(), dr.center().y, dr.width(), &detail, theme.font(type_scale::CAPTION), kit::weak(&theme));
        if shown + 1.0 < ui_kit::text_w(ui.ctx(), &detail, &theme.font(type_scale::CAPTION)) {
            dresp.on_hover_text(&detail);
        }
    }
    // Knöpfe: ffmpeg-Datei… | Automatisch suchen | Erneut prüfen
    let (br, _) = ui.allocate_exact_size(vec2(inner.width(), 24.0), Sense::hover());
    let labels = [t("batch-settings-ffmpeg-choose"), t("batch-settings-ffmpeg-auto"), t("batch-settings-ffmpeg-recheck")];
    let mut x = br.left() + label_w + sp::S;
    let mut clicked = None;
    for (k, l) in labels.iter().enumerate() {
        let bw = kit::btn_w(ui.ctx(), l, k == 0).min((br.right() - x).max(40.0));
        let b = Rect::from_min_size(pos2(x, br.top()), vec2(bw, 24.0));
        let icon = (k == 0).then_some(RowIcon::Icon(crate::icons::Icon::Folder));
        let can = !(k == 1 && ffmpeg::configured_path().is_none());
        if kit::button(ui, b, Id::new(("fp-batch-settings-ffmpeg-btn", k)), l, icon, if k == 0 { BtnKind::Primary } else { BtnKind::Secondary }, can && !(k == 2 && state.checking()), l).clicked() {
            clicked = Some(k);
        }
        x += bw + sp::S;
    }
    if missing {
        ui.add_space(sp::S);
        ui_kit::notice(ui, Tone::Info, None, &hint);
    }
    let used = ui.min_rect().bottom() - inner.top();
    if (used - content_h).abs() > 0.5 {
        ui.ctx().data_mut(|m| m.insert_temp(h_id, used));
        ui.ctx().request_repaint();
    }
    // Schließen
    let close_l = t("batch-settings-close");
    let cw = kit::btn_w(ui.ctx(), &close_l, false).max(90.0);
    let cr = Rect::from_min_size(pos2(inner.right() - cw, inner.bottom() - 26.0), vec2(cw, 26.0));
    let close = kit::button(ui, cr, Id::new("fp-batch-settings-close"), &close_l, None, BtnKind::Secondary, true, &close_l).clicked();
    match clicked {
        Some(0) => {
            let inbox = s.inbox_sender();
            let exts: &[&str] = if cfg!(windows) { &["exe"] } else { &["*"] };
            crate::files::pick_one(st, &t("batch-settings-ffmpeg-choose"), &t("batch-settings-ffmpeg-filter"), exts, move |_st, p| {
                inbox.lock().unwrap_or_else(|e| e.into_inner()).push(Inbox::FfmpegPath(Some(p)));
            });
        }
        Some(1) => set_ffmpeg_path(ui.ctx(), s, None, now),
        Some(2) => {
            if let Some(stt) = s.settings.as_mut() {
                stt.recheck(ui.ctx());
            }
        }
        _ => {}
    }
    if close {
        s.settings = None;
    }
    if s.settings.as_ref().is_some_and(SettingsState::checking) {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
    }
}

// ---------------------------------------------------------------------- Rückfrage beim Schließen

/// Was nach „Abbrechen und schließen“ passiert.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseAsk {
    /// Fenster/Programm schließen (`Action::Quit`).
    Window,
    /// Nur das Studio schließen (× am Reiter, Menü).
    Studio,
}

/// Haken in `Action::Quit` (Fenster schließen, Alt+F4, *Datei ▸ Beenden*): läuft ein Batch, kommt
/// erst die Rückfrage im Studio (Studio nach vorn). Liefert `true`, wenn das Beenden wartet.
pub fn intercept_quit(st: &mut AppState) -> bool {
    let Some(s) = st.batch.as_mut() else { return false };
    if !s.running() || s.close_confirmed {
        return false;
    }
    s.front = true;
    s.close_ask = Some(CloseAsk::Window);
    true
}

/// Studio schließen (× oder Menü): bei laufendem Auftrag erst fragen. Liefert `true`, wenn sofort
/// geschlossen werden darf.
pub fn may_close_studio(s: &mut Studio) -> bool {
    if s.running() && !s.close_confirmed {
        s.close_requested = false;
        s.close_ask = Some(CloseAsk::Studio);
        s.front = true;
        return false;
    }
    true
}

/// Rückfrage zeichnen (über dem ganzen Studio). Liefert die Wahl: `Some(true)` = abbrechen und
/// schließen, `Some(false)` = weiterlaufen lassen.
pub fn close_overlay(ui: &mut Ui, st: &mut AppState, s: &mut Studio, area: Rect) {
    let Some(kind) = s.close_ask else { return };
    if !s.running() {
        // Lauf ist inzwischen fertig: ohne Rückfrage weitermachen
        s.close_ask = None;
        finish_close(st, s, kind);
        return;
    }
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let layer = egui::LayerId::new(egui::Order::Foreground, Id::new("fp-batch-close-ask"));
    let painter = ui.ctx().layer_painter(layer);
    painter.rect_filled(area, 0.0, with_alpha(Color32::BLACK, if theme.glow { 150 } else { 90 }));
    let mut lay = ui.new_child(egui::UiBuilder::new().layer_id(layer).max_rect(area));
    let _ = lay.interact(area, Id::new("fp-batch-close-block"), Sense::click_and_drag());
    let (done, total) = s.run.as_ref().map(|r| (r.finished, r.total)).unwrap_or((0, 0));
    let w = 460.0f32.min(area.width() - 2.0 * sp::L).max(260.0);
    let font = theme.font(type_scale::BODY);
    let text = t("batch-quit-text");
    let detail = fp_i18n::tr!("batch-quit-detail", done = done, total = total);
    let tw = w - 2.0 * kit::PAD;
    let th = text_h(ui.ctx(), &text, &theme.bold(type_scale::BODY), tw);
    let dh = text_h(ui.ctx(), &detail, &font, tw);
    let h = kit::HEAD_H + kit::PAD + th + sp::XS + dh + sp::L + 26.0 + kit::PAD;
    let r = Rect::from_center_size(area.center(), vec2(w, h));
    let inner = kit::card(&painter, r, &theme, c.icon_yellow, RowIcon::Kit(KitIcon::Warning), &t("batch-quit-title"), None);
    let g = ui.ctx().fonts_mut(|f| f.layout(text, theme.bold(type_scale::BODY), c.text, inner.width()));
    painter.galley(inner.min, g, c.text);
    let g2 = ui.ctx().fonts_mut(|f| f.layout(detail, font.clone(), kit::weak(&theme), inner.width()));
    painter.galley(pos2(inner.left(), inner.top() + th + sp::XS), g2, kit::weak(&theme));
    let keep_l = t("batch-quit-keep");
    let ok_l = t("batch-quit-confirm");
    let kw = kit::btn_w(ui.ctx(), &keep_l, false);
    let ow = kit::btn_w(ui.ctx(), &ok_l, false);
    let y = inner.bottom() - 26.0;
    let okr = Rect::from_min_size(pos2(inner.right() - ow, y), vec2(ow, 26.0));
    let keepr = Rect::from_min_size(pos2(okr.left() - sp::S - kw, y), vec2(kw, 26.0));
    let keep = kit::button(&mut lay, keepr, Id::new("fp-batch-close-keep"), &keep_l, None, BtnKind::Secondary, true, &keep_l).clicked();
    let ok = kit::button(&mut lay, okr, Id::new("fp-batch-close-ok"), &ok_l, None, BtnKind::Primary, true, &ok_l).clicked();
    let esc = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
    if keep || esc {
        s.close_ask = None;
    } else if ok {
        s.close_ask = None;
        if let Some(run) = &s.run {
            run.cancel();
        }
        finish_close(st, s, kind);
    }
}

fn finish_close(st: &mut AppState, s: &mut Studio, kind: CloseAsk) {
    s.close_confirmed = true;
    match kind {
        CloseAsk::Window => st.push(Action::Quit),
        CloseAsk::Studio => s.close_requested = true,
    }
}

/// Höhe eines umbrochenen Texts.
fn text_h(ctx: &egui::Context, text: &str, font: &egui::FontId, w: f32) -> f32 {
    ctx.fonts_mut(|f| f.layout(text.to_string(), font.clone(), Color32::WHITE, w.max(10.0)).size().y)
}

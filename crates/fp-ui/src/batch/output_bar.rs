//! Ausgabe-Leiste (SPEC-BATCH §2.6–§2.8): Ziel (neuer Ordner neben den Quellen – Vorgabe
//! `batch-output` –, eigener Ordner, gleicher Ort), Konfliktregel, Trockenlauf, Start, Fortschritt
//! mit Restzeit, Abbrechen; Rückfrage bei Regel „Jedes Mal fragen“.

use super::kit::{self, BtnKind};
use super::{EntryStatus, Inbox, Mode, Studio};
use crate::app_state::AppState;
use crate::theme::{Theme, with_alpha};
use crate::ui_kit::{self, RowIcon, Tone, sp, type_scale};
use egui::{Align2, Color32, CornerRadius, Id, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use fp_batch::recipe::{Conflict, TargetKind};
use fp_batch::{AskChoice, PlanEntry, Progress, RunOptions};
use fp_i18n::t;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Instant;

/// Höhe der Leiste: einzeilig ab 1200 px Breite, sonst zweizeilig.
pub fn height(width: f32) -> f32 {
    if width >= 1200.0 { 64.0 } else { 96.0 }
}

type AskReq = (PlanEntry, Sender<(AskChoice, bool)>);

/// Laufender Auftrag.
pub struct RunState {
    /// The job (None: a run driven by [`apply_events`] alone, e.g. in tests).
    handle: Option<fp_batch::JobHandle>,
    /// Index im Auftrag → Index in `files`.
    pub map: Vec<usize>,
    pub total: usize,
    pub finished: usize,
    pub started: Instant,
    ask_rx: Receiver<AskReq>,
    /// Offene Rückfrage (Regel „Jedes Mal fragen“).
    pub ask: Option<(PlanEntry, Sender<(AskChoice, bool)>)>,
    pub ask_all: bool,
    pub cancelling: bool,
    /// Geglättete Restzeit (Zeitpunkt der Schätzung, Sekunden), siehe [`RunState::update_eta`].
    eta: Option<(Instant, f64)>,
    /// Sum of the progress of the running files (0..1 each, from `Progress::ItemProgress`).
    pub partial: f32,
}

impl RunState {
    /// A run without a job thread: progress comes only through [`apply_events`]; it ends when
    /// cancelled or when `Progress::Finished` arrives (tests of the progress display).
    pub fn detached(map: Vec<usize>) -> RunState {
        let (_tx, ask_rx) = channel::<AskReq>();
        RunState {
            handle: None,
            total: map.len(),
            map,
            finished: 0,
            started: Instant::now(),
            ask_rx,
            ask: None,
            ask_all: false,
            cancelling: false,
            eta: None,
            partial: 0.0,
        }
    }
    pub fn cancel(&self) {
        if let Some(h) = &self.handle {
            h.cancel();
        }
        if let Some((_, tx)) = &self.ask {
            let _ = tx.send((AskChoice::Skip, true));
        }
    }
    /// Anteil fertig (finished files plus the progress of the running ones).
    pub fn fraction(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            ((self.finished as f32 + self.partial) / self.total as f32).clamp(0.0, 1.0)
        }
    }
    /// Geschätzte Restzeit in Sekunden: letzte Schätzung, seitdem heruntergezählt.
    pub fn remaining(&self) -> Option<f64> {
        let (at, est) = self.eta?;
        Some((est - at.elapsed().as_secs_f64()).max(0.0))
    }
    /// Neue Schätzung nach jeder fertigen Datei: Rohwert (bisherige Dauer je Datei × Rest) mit der
    /// heruntergezählten alten Schätzung gemittelt (gleitender Mittelwert, SPEC §2.8). Ohne Glättung
    /// sprang die Restzeit bei parallelen Läufen stark (0:58 → 3:59 → 0:47, BATCH-T).
    pub fn update_eta(&mut self) {
        // finished files plus the parts of the running ones (per-file progress, BA-107)
        let done = self.finished as f64 + self.partial as f64;
        if done <= 0.01 || self.total == 0 {
            return;
        }
        let per = self.started.elapsed().as_secs_f64() / done;
        let raw = per * (self.total as f64 - done).max(0.0);
        let est = match self.remaining() {
            Some(prev) if self.finished < self.total => 0.35 * raw + 0.65 * prev,
            _ => raw,
        };
        self.eta = Some((Instant::now(), est));
    }
}

/// Warum „Verarbeiten“ gesperrt ist (None = frei).
pub fn blocked(s: &mut Studio) -> Option<String> {
    if s.running() {
        return Some(t("batch-ui-running"));
    }
    if s.to_process().is_empty() {
        return Some(t(if s.files.is_empty() {
            "batch-ui-blocked-empty"
        } else {
            "batch-ui-blocked-filtered"
        }));
    }
    if s.has_errors() {
        let first = s
            .issues()
            .into_iter()
            .find(|i| i.severity == fp_batch::Severity::Error)
            .map(|i| i.message)
            .unwrap_or_default();
        return Some(format!("{}\n{first}", t("batch-ui-blocked-invalid")));
    }
    None
}

/// Trockenlauf: Plan rechnen (nichts schreiben), Reiter „Plan“ zeigen.
pub fn dry_run(s: &mut Studio, now: f64) {
    if s.running() {
        return;
    }
    let idx = s.to_process();
    let items: Vec<fp_batch::ItemInfo> = idx.iter().map(|&i| s.files[i].info.clone()).collect();
    let reg = fp_batch::StepRegistry::global();
    match fp_batch::plan(&s.recipe, &items, &reg, &Default::default()) {
        Ok(p) => {
            s.plan = Some(p);
            s.plan_error = None;
        }
        Err(e) => {
            s.plan = None;
            s.plan_error = Some(e.to_string());
            s.notify(Tone::Error, e.to_string(), now);
        }
    }
    s.show_summary = false;
    s.view_tab = super::preview::Tab::Plan;
    if s.narrow_tab == 0 || s.narrow_tab == 1 {
        s.narrow_tab = 2;
    }
}

/// Startet den Auftrag (Strg+Eingabe, Knopf „Verarbeiten“, „Fehlgeschlagene erneut“).
/// Das Studio kommt als Parameter (wie N33): während es sich zeichnet, ist es aus `st.batch`
/// herausgenommen – über `st.batch` lief der Knopf „Verarbeiten“ ins Leere (BATCH-T).
pub fn start(s: &mut Studio, now: f64) {
    if blocked(s).is_some() {
        return;
    }
    let map = s.to_process();
    let items: Vec<fp_batch::ItemInfo> = map.iter().map(|&i| s.files[i].info.clone()).collect();
    for &i in &map {
        let f = &mut s.files[i];
        f.status = EntryStatus::Waiting;
        f.message = None;
        f.target = None;
        f.progress = 0.0;
    }
    let (ask_tx, ask_rx) = channel::<AskReq>();
    let ask: Arc<fp_batch::executor::AskFn> = Arc::new(move |e: &PlanEntry| {
        let (tx, rx) = channel();
        if ask_tx.send((e.clone(), tx)).is_err() {
            return (AskChoice::Rename, false);
        }
        rx.recv().unwrap_or((AskChoice::Skip, true))
    });
    let opts = RunOptions {
        ask: Some(ask),
        log: true,
        ..RunOptions::default()
    };
    let handle = fp_batch::spawn(s.recipe.clone(), items, opts);
    let total = map.len();
    s.run = Some(RunState {
        handle: Some(handle),
        partial: 0.0,
        map,
        total,
        finished: 0,
        started: Instant::now(),
        ask_rx,
        ask: None,
        ask_all: false,
        cancelling: false,
        eta: None,
    });
    s.report = None;
    s.show_summary = false;
    s.gallery = None;
    s.drag = None;
    let _ = now;
}

/// Fortschritt abholen.
pub fn poll(ctx: &egui::Context, s: &mut Studio, now: f64) {
    let Some(run) = s.run.as_mut() else { return };
    if run.ask.is_none()
        && let Ok(req) = run.ask_rx.try_recv()
    {
        run.ask = Some(req);
    }
    // Ist der Thread schon fertig, kommen danach keine Ereignisse mehr: erst prüfen, dann leeren
    let thread_done = match &run.handle {
        Some(h) => h.is_finished(),
        None => run.cancelling,
    };
    let events: Vec<Progress> = run
        .handle
        .as_ref()
        .map(|h| h.events.try_iter().collect())
        .unwrap_or_default();
    let finished = apply_events(s, events);
    if finished.is_none() && !thread_done {
        return;
    }
    finish(ctx, s, finished, now);
}

/// Applies progress events of the run to the list (status, per-file progress, ETA). Returns the
/// report when `Progress::Finished` arrived.
pub fn apply_events(s: &mut Studio, events: Vec<Progress>) -> Option<fp_batch::Report> {
    let run = s.run.as_mut()?;
    let map = run.map.clone();
    let mut finished: Option<fp_batch::Report> = None;
    let finished_before = run.finished;
    let mut progressed = false;
    for ev in events {
        let idx = |k: usize| map.get(k).copied();
        match ev {
            Progress::Started { total } => run.total = total,
            Progress::ItemStarted { index } => {
                if let Some(i) = idx(index) {
                    s.files[i].status = EntryStatus::Running;
                    s.files[i].progress = 0.0;
                }
            }
            Progress::ItemProgress { index, fraction, .. } => {
                if let Some(i) = idx(index)
                    && fraction.is_finite()
                {
                    s.files[i].progress = fraction.clamp(0.0, 1.0);
                    progressed = true;
                }
            }
            Progress::ItemDone { index, target, .. } => {
                run.finished += 1;
                if let Some(i) = idx(index) {
                    s.files[i].status = EntryStatus::Done;
                    s.files[i].target = Some(target);
                }
            }
            Progress::ItemSkipped { index, reason } => {
                run.finished += 1;
                if let Some(i) = idx(index) {
                    s.files[i].status = EntryStatus::Skipped;
                    s.files[i].message = Some(reason);
                }
            }
            Progress::ItemFailed {
                index,
                step,
                message,
            } => {
                run.finished += 1;
                if let Some(i) = idx(index) {
                    s.files[i].status = EntryStatus::Failed;
                    s.files[i].message = Some(match step {
                        Some(st) => format!("{}: {message}", fp_batch::step::step_label(&st)),
                        None => message,
                    });
                }
            }
            Progress::ItemCancelled { index } => {
                if let Some(i) = idx(index) {
                    s.files[i].status = EntryStatus::Cancelled;
                }
            }
            Progress::Finished(r) => finished = Some(*r),
        }
    }
    run.partial = map
        .iter()
        .filter(|&&i| s.files[i].status == EntryStatus::Running)
        .map(|&i| s.files[i].progress)
        .sum();
    if run.finished != finished_before || progressed {
        run.update_eta();
    }
    finished
}

/// End of the run: take over the report or report the error of the whole job.
fn finish(ctx: &egui::Context, s: &mut Studio, mut finished: Option<fp_batch::Report>, now: f64) {
    // Ende: Bericht übernehmen bzw. Fehler des ganzen Auftrags melden
    let Some(run) = s.run.take() else { return };
    let map = run.map.clone();
    let mut error = None;
    if finished.is_none()
        && let Some(h) = run.handle
    {
        match h.join() {
            Ok(r) => finished = Some(r),
            Err(e) => error = Some(e.to_string()),
        }
    }
    if let Some(r) = &finished {
        for it in &r.items {
            if let Some(&i) = map.get(it.index)
                && s.files[i].status == EntryStatus::Done
                && !it.warnings.is_empty()
            {
                s.files[i].status = EntryStatus::Warning;
                s.files[i].message = Some(it.warnings.join("\n"));
            }
        }
    }
    for &i in &map {
        if matches!(
            s.files[i].status,
            EntryStatus::Running | EntryStatus::Waiting
        ) {
            s.files[i].status = EntryStatus::Cancelled;
        }
    }
    if let Some(e) = error {
        s.notify(Tone::Error, e, now);
    }
    if let Some(r) = finished {
        let failed = r.totals.failed;
        s.report = Some(r);
        s.show_summary = true;
        s.narrow_tab = 2;
        if failed > 0 {
            s.notify(
                Tone::Warning,
                fp_i18n::tr!("batch-ui-failed-notice", n = failed),
                now,
            );
        }
    }
    s.files_changed();
    ctx.request_repaint();
}
fn target_label(s: &Studio) -> String {
    match s.recipe.output.target.kind {
        TargetKind::NewFolder => fp_i18n::tr!(
            "batch-ui-target-new",
            name = s.recipe.output.target.folder_name.clone()
        ),
        TargetKind::Folder => s
            .recipe
            .output
            .target
            .path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| t("batch-ui-target-folder")),
        TargetKind::SameAsSource => t("batch-ui-target-same"),
    }
}

fn conflict_label(c: Conflict) -> String {
    t(match c {
        Conflict::Rename => "batch-ui-conflict-rename",
        Conflict::Skip => "batch-ui-conflict-skip",
        Conflict::Overwrite => "batch-ui-conflict-overwrite",
        Conflict::Ask => "batch-ui-conflict-ask",
    })
}

/// Zeichnet die Leiste.
pub fn show(ui: &mut Ui, st: &mut AppState, s: &mut Studio, rect: Rect, now: f64) {
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let p = ui.painter().clone();
    p.rect_filled(rect, 0.0, theme.dialog_footer_bg());
    let (dark, light) = theme.dialog_footer_lines();
    p.hline(rect.x_range(), rect.top() + 0.5, Stroke::new(1.0, dark));
    if let Some(l) = light {
        p.hline(rect.x_range(), rect.top() + 1.5, Stroke::new(1.0, l));
    }
    let two_lines = rect.height() > 70.0;
    let pad = if rect.width() < 720.0 { sp::M } else { sp::L };
    let inner = Rect::from_min_max(
        pos2(rect.left() + pad, rect.top() + 2.0),
        pos2(rect.right() - pad, rect.bottom()),
    );
    let (line1, line2) = if two_lines {
        let h = (inner.height() - 4.0) / 2.0;
        (
            Rect::from_min_size(inner.min, vec2(inner.width(), h)),
            Rect::from_min_size(pos2(inner.left(), inner.top() + h), vec2(inner.width(), h)),
        )
    } else {
        (inner, inner)
    };
    let running = s.running();
    // ------------------------------------------------ Start, Abbrechen, Trockenlauf (rechts bzw. Zeile 2)
    let n = s.to_process().len();
    let start_l = fp_i18n::tr!("batch-ui-start", n = n);
    let block = blocked(s);
    let sw = kit::btn_w(ui.ctx(), &start_l, true)
        .max(140.0)
        .min((line2.width() * 0.5).max(100.0));
    let start_r = Rect::from_min_size(
        pos2(line2.right() - sw, line2.center().y - 16.0),
        vec2(sw, 32.0),
    );
    let sresp = kit::button(
        ui,
        start_r,
        Id::new("fp-batch-start"),
        &start_l,
        Some(RowIcon::Icon(crate::icons::Icon::Play)),
        BtnKind::Primary,
        block.is_none(),
        &start_l,
    );
    let sresp = match &block {
        Some(why) if !running => sresp.on_disabled_hover_text(why.as_str()),
        _ => sresp.on_hover_text(t("batch-ui-start.tooltip")),
    };
    if sresp.clicked() {
        start(s, now);
    }
    let dry_l = t("batch-ui-dry-run");
    let dw = kit::btn_w(ui.ctx(), &dry_l, false);
    let cancel_l = t("batch-ui-cancel");
    let cw = kit::btn_w(ui.ctx(), &cancel_l, false);
    let mut x_right = start_r.left() - sp::S;
    if running {
        let cr = Rect::from_min_size(pos2(x_right - cw, line2.center().y - 12.0), vec2(cw, 24.0));
        let cancelling = s.run.as_ref().is_some_and(|r| r.cancelling);
        if kit::button(
            ui,
            cr,
            Id::new("fp-batch-cancel"),
            &cancel_l,
            None,
            BtnKind::Secondary,
            !cancelling,
            &cancel_l,
        )
        .clicked()
            && let Some(r) = s.run.as_mut()
        {
            r.cancel();
            r.cancelling = true;
        }
        x_right = cr.left() - sp::S;
    }
    let dry_x = if two_lines {
        line2.left()
    } else {
        x_right - dw
    };
    let dr = Rect::from_min_size(pos2(dry_x, line2.center().y - 12.0), vec2(dw, 24.0));
    let dry_ok = !running && !s.to_process().is_empty() && dr.right() <= x_right + 0.5;
    if dr.right() <= x_right + 0.5
        && kit::button(
            ui,
            dr,
            Id::new("fp-batch-dry-run"),
            &dry_l,
            None,
            BtnKind::Secondary,
            dry_ok,
            &dry_l,
        )
        .on_hover_text(t("batch-ui-dry-run.tooltip"))
        .clicked()
    {
        dry_run(s, now);
    }
    // Fortschritt zwischen Trockenlauf und Abbrechen/Start (zweizeilig) bzw. links davon
    let prog_left = if two_lines {
        dr.right() + sp::M
    } else {
        line1.left() + line1.width() * 0.55
    };
    let prog_right = if two_lines {
        x_right
    } else {
        dr.left() - sp::M
    };
    if let Some(run) = &s.run {
        let pr = Rect::from_min_max(
            pos2(prog_left, line2.center().y - 13.0),
            pos2(prog_right.max(prog_left + 20.0), line2.center().y + 13.0),
        );
        progress(ui, pr, run, &theme);
    } else if let Some(r) = &s.report {
        let pr = Rect::from_min_max(
            pos2(prog_left, line2.top()),
            pos2(prog_right.max(prog_left), line2.bottom()),
        );
        if pr.width() > 40.0 {
            let txt = super::report_ui::short_summary(r);
            kit::text_left(
                &p,
                pr.left(),
                pr.center().y,
                pr.width(),
                &txt,
                theme.font(type_scale::BODY),
                kit::weak(&theme),
            );
        }
    }
    // ------------------------------------------------ Ziel und Konflikt (links bzw. Zeile 1)
    let left_end = if two_lines {
        line1.right()
    } else {
        prog_left - sp::M
    };
    target_controls(
        ui,
        st,
        s,
        Rect::from_min_max(line1.min, pos2(left_end, line1.bottom())),
        running,
    );
    let _ = c;
}

fn progress(ui: &mut Ui, r: Rect, run: &RunState, theme: &Theme) {
    let c = &theme.colors;
    let p = ui.painter();
    let mut txt = fp_i18n::tr!("batch-ui-progress", done = run.finished, total = run.total);
    if let Some(rem) = run.remaining() {
        let secs = rem.round() as u64;
        txt = format!(
            "{txt} · {}",
            fp_i18n::tr!(
                "batch-ui-remaining",
                time = format!("{}:{:02}", secs / 60, secs % 60)
            )
        );
    }
    if run.cancelling {
        txt = t("batch-ui-cancelling");
    }
    let f = theme.font(type_scale::CAPTION + 0.5);
    kit::text_left(p, r.left(), r.top() + 6.0, r.width(), &txt, f, c.text);
    let bar = Rect::from_min_max(pos2(r.left(), r.bottom() - 8.0), r.max);
    let cr = CornerRadius::same(4);
    p.rect_filled(bar, cr, c.field_bg);
    p.rect_stroke(
        bar,
        cr,
        Stroke::new(1.0, c.field_border),
        StrokeKind::Inside,
    );
    let fr = run.fraction();
    if fr > 0.0 {
        p.rect_filled(
            Rect::from_min_size(bar.min, vec2(bar.width() * fr, bar.height())),
            cr,
            c.focus,
        );
    }
}

fn target_controls(ui: &mut Ui, st: &mut AppState, s: &mut Studio, r: Rect, running: bool) {
    let theme = Theme::current(ui.ctx());
    let p = ui.painter().clone();
    let f = theme.font(type_scale::BODY);
    let lab_t = t("batch-ui-target");
    let lab_c = t("batch-ui-conflict");
    let mut x = r.left();
    let cy = r.center().y;
    let lw = ui_kit::text_w(ui.ctx(), &lab_t, &f);
    p.text(
        pos2(x, cy),
        Align2::LEFT_CENTER,
        &lab_t,
        f.clone(),
        kit::weak(&theme),
    );
    x += lw + sp::S;
    // Ziel ▾
    let folder_l = t("batch-ui-target-choose");
    let fw = kit::btn_w(ui.ctx(), &folder_l, true);
    let clw = ui_kit::text_w(ui.ctx(), &lab_c, &f);
    let conflict_w = 150.0f32;
    let avail = (r.right() - x - fw - clw - conflict_w - 3.0 * sp::S - sp::L).max(80.0);
    let tw = avail.min(300.0);
    let tr = Rect::from_min_size(pos2(x, cy - 12.0), vec2(tw, 24.0));
    let mut tc = ui.new_child(egui::UiBuilder::new().max_rect(tr));
    crate::theme::field_widgets(&mut tc);
    let mut kind = s.recipe.output.target.kind;
    let label = target_label(s);
    let shown = ui_kit::elide_middle(ui.ctx(), &label, &f, tw - 28.0);
    tc.add_enabled_ui(!running, |ui| {
        egui::ComboBox::from_id_salt("fp-batch-target")
            .selected_text(shown)
            .width(tw - 8.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut kind,
                    TargetKind::NewFolder,
                    fp_i18n::tr!(
                        "batch-ui-target-new",
                        name = s.recipe.output.target.folder_name.clone()
                    ),
                );
                ui.selectable_value(&mut kind, TargetKind::Folder, t("batch-ui-target-folder"));
                ui.selectable_value(
                    &mut kind,
                    TargetKind::SameAsSource,
                    t("batch-ui-target-same"),
                );
            })
            .response
            .on_hover_text(label.clone());
    });
    if kind != s.recipe.output.target.kind {
        if kind == TargetKind::Folder && s.recipe.output.target.path.is_none() {
            pick_target(st, s);
        } else {
            s.edit("target-kind", |r| {
                r.output.target.kind = kind;
                // Original ersetzen nur mit Sicherung (SPEC §2.7)
                if kind == TargetKind::SameAsSource {
                    r.output.backup.enabled = true;
                }
                true
            });
            s.end_gesture();
        }
    }
    x = tr.right() + sp::S;
    let fr = Rect::from_min_size(pos2(x, cy - 12.0), vec2(fw, 24.0));
    if kit::button(
        ui,
        fr,
        Id::new("fp-batch-target-choose"),
        &folder_l,
        Some(RowIcon::Icon(crate::icons::Icon::Folder)),
        BtnKind::Secondary,
        !running,
        &folder_l,
    )
    .on_hover_text(t("batch-ui-target-choose.tooltip"))
    .clicked()
    {
        pick_target(st, s);
    }
    x = fr.right() + sp::L;
    // Bei Konflikt ▾
    if x + clw + sp::S + 80.0 > r.right() {
        return;
    }
    p.text(
        pos2(x, cy),
        Align2::LEFT_CENTER,
        &lab_c,
        f.clone(),
        kit::weak(&theme),
    );
    x += clw + sp::S;
    let cw = conflict_w.min(r.right() - x);
    let cr = Rect::from_min_size(pos2(x, cy - 12.0), vec2(cw, 24.0));
    let mut cc = ui.new_child(egui::UiBuilder::new().max_rect(cr));
    crate::theme::field_widgets(&mut cc);
    let mut conflict = s.recipe.output.conflict;
    cc.add_enabled_ui(!running, |ui| {
        egui::ComboBox::from_id_salt("fp-batch-conflict")
            .selected_text(conflict_label(conflict))
            .width(cw - 8.0)
            .show_ui(ui, |ui| {
                for k in [
                    Conflict::Rename,
                    Conflict::Skip,
                    Conflict::Overwrite,
                    Conflict::Ask,
                ] {
                    ui.selectable_value(&mut conflict, k, conflict_label(k));
                }
            });
    });
    if conflict != s.recipe.output.conflict {
        s.edit("conflict", |r| {
            r.output.conflict = conflict;
            true
        });
        s.end_gesture();
    }
    x = cr.right() + sp::L;
    // Profi: Ordnerstruktur beibehalten; gleicher Ort: Sicherung behalten
    let mut cb = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(Rect::from_min_max(
                pos2(x, cy - 12.0),
                pos2(r.right(), cy + 12.0),
            ))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    crate::theme::field_widgets(&mut cb);
    if s.recipe.output.target.kind == TargetKind::SameAsSource {
        let mut inc = s.recipe.output.backup.incremental;
        let l = t("batch-ui-backup-keep");
        if r.right() - x > ui_kit::text_w(ui.ctx(), &l, &f) + 24.0
            && cb
                .add_enabled(!running, egui::Checkbox::new(&mut inc, l))
                .on_hover_text(t("batch-ui-backup-keep.tooltip"))
                .changed()
        {
            s.edit("backup", |r| {
                r.output.backup.incremental = inc;
                true
            });
            s.end_gesture();
        }
    } else if s.mode == Mode::Pro {
        let mut keep = s.recipe.output.target.keep_structure;
        let l = t("batch-ui-keep-structure");
        if r.right() - x > ui_kit::text_w(ui.ctx(), &l, &f) + 24.0
            && cb
                .add_enabled(!running, egui::Checkbox::new(&mut keep, l))
                .changed()
        {
            s.edit("structure", |r| {
                r.output.target.keep_structure = keep;
                true
            });
            s.end_gesture();
        }
    }
}

fn pick_target(st: &mut AppState, s: &Studio) {
    let inbox = s.inbox_sender();
    let req = crate::file_dialog::Request::folder(t("batch-ui-target-choose-title"));
    crate::file_dialog::request(st, req, move |_st, paths| {
        if let Some(p) = paths.into_iter().next() {
            inbox
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(Inbox::TargetFolder(p));
        }
    });
}

/// Rückfrage „Datei gibt es schon“ (Regel „Jedes Mal fragen“) als Karte über dem Studio.
pub fn ask_overlay(ui: &mut Ui, s: &mut Studio, area: Rect) {
    let Some(run) = s.run.as_mut() else { return };
    let Some((entry, _)) = run.ask.as_ref() else {
        return;
    };
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;
    let name = entry
        .target
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let w = 420.0f32.min(area.width() - 32.0);
    let text = fp_i18n::tr!("batch-ui-ask-text", name = name);
    let th = ui.ctx().fonts_mut(|f| {
        f.layout(
            text.clone(),
            theme.font(type_scale::BODY),
            Color32::WHITE,
            w - 2.0 * kit::PAD,
        )
        .size()
        .y
    });
    let h = kit::HEAD_H + kit::PAD + th + sp::M + 24.0 + sp::S + 24.0 + kit::PAD;
    let r = Rect::from_center_size(area.center(), vec2(w, h));
    let layer = egui::LayerId::new(egui::Order::Foreground, Id::new("fp-batch-ask"));
    let painter = ui.ctx().layer_painter(layer);
    painter.rect_filled(area, 0.0, with_alpha(Color32::BLACK, 80));
    let inner = kit::card(
        &painter,
        r,
        &theme,
        c.icon_yellow,
        RowIcon::Kit(crate::ui_kit::KitIcon::Warning),
        &t("batch-ui-ask-title"),
        None,
    );
    let g = ui
        .ctx()
        .fonts_mut(|f| f.layout(text, theme.font(type_scale::BODY), c.text, inner.width()));
    painter.galley(inner.min, g, c.text);
    let mut area_ui = ui.new_child(egui::UiBuilder::new().layer_id(layer).max_rect(r));
    let y = inner.top() + th + sp::M;
    let mut all = run.ask_all;
    let mut cbc = area_ui.new_child(
        egui::UiBuilder::new()
            .max_rect(Rect::from_min_size(
                pos2(inner.left(), y),
                vec2(inner.width(), 24.0),
            ))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    crate::theme::field_widgets(&mut cbc);
    cbc.checkbox(&mut all, t("batch-ui-ask-all"));
    run.ask_all = all;
    let labels = [
        (AskChoice::Rename, t("batch-ui-conflict-rename")),
        (AskChoice::Skip, t("batch-ui-conflict-skip")),
        (AskChoice::Overwrite, t("batch-ui-conflict-overwrite")),
    ];
    let bw = ((inner.width() - 2.0 * sp::S) / 3.0).floor();
    let by = y + 24.0 + sp::S;
    let mut answer = None;
    for (k, (choice, l)) in labels.iter().enumerate() {
        let br = Rect::from_min_size(
            pos2(inner.left() + k as f32 * (bw + sp::S), by),
            vec2(bw, 24.0),
        );
        if kit::button(
            &mut area_ui,
            br,
            Id::new(("fp-batch-ask-btn", k)),
            l,
            None,
            if k == 0 {
                BtnKind::Primary
            } else {
                BtnKind::Secondary
            },
            true,
            l,
        )
        .clicked()
        {
            answer = Some(*choice);
        }
    }
    if let Some(a) = answer
        && let Some((_, tx)) = run.ask.take()
    {
        let _ = tx.send((a, run.ask_all));
    }
    let _ = Sense::hover();
}

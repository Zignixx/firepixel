//! Ausführung (SPEC-BATCH §5.2): Thread-Pool (rayon), Speicherbudget (Byte-Semaphor), Abbrechen
//! (`CancelToken`), Fortschrittskanal (`std::sync::mpsc`), Trockenlauf, Rückfrage bei Konflikten,
//! Sammel-Schritte in einer zweiten Phase, Bericht.
//!
//! Ein fehlerhaftes Element bricht den Lauf nicht ab (Abweichung vom Original, §1.2): es wird als
//! Fehler markiert, die übrigen laufen weiter.

use crate::error::{BatchError, Result};
use crate::item::{self, Item, ItemInfo};
use crate::naming::{self, Plan, PlanAction, PlanEntry, PlanOptions};
use crate::output::{self, OutputFormat, resolve_format};
use crate::pipeline::{FileAction, Pipeline};
use crate::recipe::Recipe;
use crate::report::{self, ItemReport, ItemStatus, Report};
use crate::step::{StepContext, StepRegistry};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Instant;

/// Vorgabe der Speichergrenze (MB), wenn das Rezept keine nennt.
pub const DEFAULT_MEMORY_MB: u64 = 4096;

/// Abbrechen-Marke (geteilt zwischen Oberfläche und Arbeits-Threads). Optional an einen
/// Generationszähler gebunden: veraltet die Generation, gilt sie als abgebrochen (Vorschau).
#[derive(Clone, Debug, Default)]
pub struct CancelToken {
    flag: Arc<AtomicBool>,
    generation: Option<(Arc<AtomicU64>, u64)>,
}

impl CancelToken {
    /// Neue, nicht abgebrochene Marke.
    pub fn new() -> CancelToken {
        CancelToken::default()
    }
    /// Marke für eine Vorschau-Generation: abgebrochen, sobald `current` ≠ `generation`.
    pub fn for_generation(current: Arc<AtomicU64>, generation: u64) -> CancelToken {
        CancelToken {
            flag: Arc::new(AtomicBool::new(false)),
            generation: Some((current, generation)),
        }
    }
    /// Abbrechen.
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
            || self
                .generation
                .as_ref()
                .is_some_and(|(c, g)| c.load(Ordering::SeqCst) != *g)
    }
    /// Fehler „abgebrochen“, wenn abgebrochen.
    pub fn check(&self) -> Result<()> {
        if self.is_cancelled() {
            Err(BatchError::cancelled())
        } else {
            Ok(())
        }
    }
}

/// Fortschrittsereignis (die Oberfläche fragt je Frame ab, ohne zu blockieren).
#[derive(Clone, Debug)]
pub enum Progress {
    Started {
        total: usize,
    },
    ItemStarted {
        index: usize,
    },
    ItemProgress {
        index: usize,
        fraction: f32,
    },
    ItemDone {
        index: usize,
        target: PathBuf,
        bytes_out: u64,
    },
    ItemSkipped {
        index: usize,
        reason: String,
    },
    ItemFailed {
        index: usize,
        step: Option<String>,
        message: String,
    },
    ItemCancelled {
        index: usize,
    },
    Finished(Box<Report>),
}

/// Antwort auf eine Rückfrage (Regel `ask`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AskChoice {
    /// Neuen Namen wählen (`altTarget`).
    Rename,
    Skip,
    Overwrite,
}

/// Rückfrage bei Konflikt: Eintrag → (Antwort, „für alle übernehmen“). Läuft im Arbeits-Thread und
/// darf blockieren, bis der Nutzer antwortet; Rückfragen kommen nacheinander.
pub type AskFn = dyn Fn(&PlanEntry) -> (AskChoice, bool) + Send + Sync;

/// Optionen eines Laufs.
#[derive(Clone, Default)]
pub struct RunOptions {
    /// Arbeits-Threads (None: Rezept, sonst Kerne − 1, mindestens 1).
    pub threads: Option<usize>,
    /// Speichergrenze in Bytes (None: Rezept, sonst [`DEFAULT_MEMORY_MB`]).
    pub memory_budget: Option<u64>,
    /// Nur planen, nichts schreiben.
    pub dry_run: bool,
    pub cancel: CancelToken,
    pub progress: Option<Sender<Progress>>,
    /// Rückfrage bei Regel `ask` (ohne: wie `rename`).
    pub ask: Option<Arc<AskFn>>,
    /// Schritt-Register (None: [`StepRegistry::global`]).
    pub registry: Option<Arc<StepRegistry>>,
    /// Bericht hierhin schreiben (None: `batch-report.json` im Ausgabeordner, wenn `output.report`).
    pub report_path: Option<PathBuf>,
    /// `batch-log.txt` anhängen.
    pub log: bool,
    /// Plan-Optionen (Tests: festes „heute“).
    pub plan: PlanOptions,
}

/// Byte-Semaphor: lässt nur so viele Elemente gleichzeitig zu, wie ins Budget passen; ein Element
/// größer als das Budget läuft allein.
struct Budget {
    total: u64,
    used: Mutex<u64>,
    cv: Condvar,
}

struct BudgetGuard<'a> {
    b: &'a Budget,
    n: u64,
}

impl Budget {
    fn acquire(&self, want: u64) -> BudgetGuard<'_> {
        let n = want.clamp(1, self.total.max(1));
        let mut used = self.used.lock().unwrap_or_else(|e| e.into_inner());
        while *used > 0 && *used + n > self.total {
            used = self.cv.wait(used).unwrap_or_else(|e| e.into_inner());
        }
        *used += n;
        BudgetGuard { b: self, n }
    }
}

impl Drop for BudgetGuard<'_> {
    fn drop(&mut self) {
        let mut used = self.b.used.lock().unwrap_or_else(|e| e.into_inner());
        *used = used.saturating_sub(self.n);
        self.b.cv.notify_all();
    }
}

fn send(tx: &Option<Sender<Progress>>, p: Progress) {
    if let Some(t) = tx {
        let _ = t.send(p);
    }
}

fn iso_now() -> String {
    chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false)
}

/// Gemeinsamer Zustand eines Laufs.
struct Job<'a> {
    recipe: &'a Recipe,
    items: &'a [ItemInfo],
    pipe: Pipeline,
    budget: Budget,
    opts: &'a RunOptions,
    ask_lock: Mutex<Option<AskChoice>>,
    total: usize,
    /// Sprache des Aufrufers (Meldungen aus den Arbeits-Threads in derselben Sprache).
    lang: String,
    /// Names with the size token: renamed after processing (BA-105).
    renamer: Option<Renamer>,
}

/// Re-naming after processing when the name uses the image size (BA-105).
struct Renamer {
    now: chrono::NaiveDateTime,
    claims: Mutex<naming::Claims>,
}

impl Job<'_> {
    fn ask(&self, e: &PlanEntry) -> AskChoice {
        let mut remembered = self.ask_lock.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(c) = *remembered {
            return c;
        }
        match &self.opts.ask {
            Some(f) => {
                let (c, all) = f(e);
                if all {
                    *remembered = Some(c);
                }
                c
            }
            None => AskChoice::Rename,
        }
    }

    /// Verarbeitet einen Eintrag des Plans.
    fn process(&self, e: &PlanEntry) -> ItemReport {
        let info = &self.items[e.index];
        let mut rep = ItemReport::new(e.index, &info.path, info.bytes);
        rep.warnings = e
            .hints
            .iter()
            .filter(|h| !matches!(h, naming::NameHint::Conflict | naming::NameHint::Skipped))
            .map(|h| h.message())
            .collect();
        if e.action == PlanAction::Skip {
            rep.status = ItemStatus::Skipped;
            rep.target = Some(e.target.clone());
            rep.warnings.push(naming::NameHint::Skipped.message());
            send(
                &self.opts.progress,
                Progress::ItemSkipped {
                    index: e.index,
                    reason: naming::NameHint::Skipped.message(),
                },
            );
            return rep;
        }
        if self.opts.cancel.is_cancelled() {
            rep.status = ItemStatus::Cancelled;
            send(
                &self.opts.progress,
                Progress::ItemCancelled { index: e.index },
            );
            return rep;
        }
        let steps = self.pipe.steps.len();
        let _guard = self.budget.acquire(if self.pipe.file_only {
            1
        } else {
            info.memory_estimate(steps)
        });
        send(
            &self.opts.progress,
            Progress::ItemStarted { index: e.index },
        );
        let t0 = Instant::now();
        let r = self.process_inner(e, info, &mut rep);
        rep.ms = t0.elapsed().as_millis() as u64;
        match r {
            Ok(()) if rep.status == ItemStatus::Skipped => {
                send(
                    &self.opts.progress,
                    Progress::ItemSkipped {
                        index: e.index,
                        reason: naming::NameHint::Skipped.message(),
                    },
                );
            }
            Ok(()) => {
                rep.status = ItemStatus::Done;
                send(
                    &self.opts.progress,
                    Progress::ItemDone {
                        index: e.index,
                        target: rep.target.clone().unwrap_or_default(),
                        bytes_out: rep.bytes_out,
                    },
                );
            }
            Err(err) if err.is_cancelled() => {
                rep.status = ItemStatus::Cancelled;
                send(
                    &self.opts.progress,
                    Progress::ItemCancelled { index: e.index },
                );
            }
            Err(err) => {
                rep.status = ItemStatus::Failed;
                let (step, msg) = match &err {
                    BatchError::Step { step, message } => (
                        Some(step.clone()),
                        format!("{}: {message}", crate::step::step_label(step)),
                    ),
                    other => (None, other.to_string()),
                };
                rep.step = step.clone();
                rep.error = Some(msg.clone());
                send(
                    &self.opts.progress,
                    Progress::ItemFailed {
                        index: e.index,
                        step,
                        message: msg,
                    },
                );
            }
        }
        rep
    }

    /// Resolves the `ask` rule for `e`: the answer's target and action, `None` = skip.
    fn resolve_ask(&self, e: &PlanEntry, info: &ItemInfo) -> Option<(PathBuf, PlanAction)> {
        match self.ask(e) {
            AskChoice::Skip => None,
            AskChoice::Overwrite => Some((
                e.target.clone(),
                if naming_same(&e.target, &info.path) {
                    PlanAction::ReplaceOriginal
                } else {
                    PlanAction::Overwrite
                },
            )),
            AskChoice::Rename => Some((
                e.alt_target.clone().unwrap_or_else(|| e.target.clone()),
                PlanAction::Write,
            )),
        }
    }

    /// Size token in the name (BA-105): rebuild the name from the output size and re-resolve
    /// conflicts. `None` = name unchanged.
    fn retarget(&self, e: &PlanEntry, item: &Item) -> Result<Option<PlanEntry>> {
        let Some(r) = &self.renamer else {
            return Ok(None);
        };
        let Some(name) =
            naming::name_for_size(self.recipe, self.items, e.index, r.now, item.dimensions())?
        else {
            return Ok(None);
        };
        let mut claims = r.claims.lock().unwrap_or_else(|p| p.into_inner());
        let t = claims.retarget(self.recipe, e, &name);
        Ok(Some(PlanEntry {
            target: t.target,
            alt_target: t.alt_target,
            action: t.action,
            ..e.clone()
        }))
    }

    fn process_inner(&self, e: &PlanEntry, info: &ItemInfo, rep: &mut ItemReport) -> Result<()> {
        let cancel = &self.opts.cancel;
        let skip = |rep: &mut ItemReport, target: PathBuf| {
            rep.status = ItemStatus::Skipped;
            rep.target = Some(target);
            rep.warnings.push(naming::NameHint::Skipped.message());
        };
        // with the size token the name is final only after processing: ask afterwards
        let late_naming = self.renamer.is_some() && !self.pipe.file_only;
        // Ziel und Aktion (Rückfrage bei `ask`)
        let (mut target, mut action) = (e.target.clone(), e.action);
        if action == PlanAction::Ask && !late_naming {
            match self.resolve_ask(e, info) {
                None => {
                    skip(rep, target);
                    return Ok(());
                }
                Some((t, a)) => (target, action) = (t, a),
            }
        }
        rep.target = Some(target.clone());
        if self.pipe.file_only {
            if action == PlanAction::ReplaceOriginal {
                // gleicher Name am gleichen Ort ohne Bildschritte: nichts zu tun
                rep.status = ItemStatus::Skipped;
                rep.warnings.push(fp_i18n::t("batch-warn-unchanged"));
                return Ok(());
            }
            cancel.check()?;
            rep.bytes_out = match self.pipe.file_action() {
                FileAction::Copy => output::copy_atomic(&info.path, &target)?,
                FileAction::Move => output::move_file(&info.path, &target)?,
            };
        } else {
            let mut item = item::load(info, false)?;
            let (fmt, _) = resolve_format(self.pipe.format, &info.format);
            let mut ctx = StepContext::new(e.index, self.total, info, cancel, fmt);
            self.pipe.apply(&mut item, &mut ctx)?;
            rep.warnings.append(&mut ctx.warnings);
            cancel.check()?;
            if late_naming {
                let renamed = self.retarget(e, &item)?;
                let entry = renamed.as_ref().unwrap_or(e);
                (target, action) = (entry.target.clone(), entry.action);
                if action == PlanAction::Skip {
                    skip(rep, target);
                    return Ok(());
                }
                if action == PlanAction::Ask {
                    match self.resolve_ask(entry, info) {
                        None => {
                            skip(rep, target);
                            return Ok(());
                        }
                        Some((t, a)) => (target, action) = (t, a),
                    }
                }
                rep.target = Some(target.clone());
            }
            self.write_item(&item, fmt, &target, action, info, rep)?;
            self.write_extra_fwpng(&item, fmt, &target, rep)?;
        }
        if self.recipe.output.keep_timestamps
            && action != PlanAction::ReplaceOriginal
            && target.exists()
        {
            let _ = output::copy_modified_time(&info.path, &target);
        }
        Ok(())
    }

    fn write_item(
        &self,
        item: &Item,
        fmt: OutputFormat,
        target: &Path,
        action: PlanAction,
        info: &ItemInfo,
        rep: &mut ItemReport,
    ) -> Result<()> {
        if let Some(dir) = target.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(output::long_path(dir))
                .map_err(|e| BatchError::Io(format!("{}: {e}", dir.display())))?;
        }
        let tmp = output::temp_path(target);
        let written = if fmt == OutputFormat::FwPng {
            output::write_fwpng(item, &output::long_path(&tmp))
        } else {
            output::encode(item, fmt, &self.recipe.output.options).and_then(|(bytes, w)| {
                rep.warnings.extend(w);
                std::fs::write(output::long_path(&tmp), &bytes)
                    .map_err(|e| BatchError::Io(format!("{}: {e}", tmp.display())))
            })
        };
        let cleanup = |e: BatchError| {
            let _ = std::fs::remove_file(output::long_path(&tmp));
            e
        };
        written.map_err(cleanup)?;
        // Abbrechen: Kodierer laufen zu Ende, ihr Ergebnis wird verworfen
        self.opts.cancel.check().map_err(cleanup)?;
        rep.bytes_out = std::fs::metadata(output::long_path(&tmp))
            .map(|m| m.len())
            .unwrap_or(0);
        if action == PlanAction::ReplaceOriginal {
            let backup = output::backup_original(&info.path, self.recipe.output.backup.incremental)
                .map_err(cleanup)?;
            if let Err(e) = output::commit_temp(&tmp, target) {
                // Original zurückholen
                let _ = std::fs::rename(output::long_path(&backup), output::long_path(&info.path));
                return Err(e);
            }
            rep.backup = Some(backup);
        } else {
            output::commit_temp(&tmp, target)?;
        }
        Ok(())
    }
}

/// Ziel der zusätzlichen FW-PNG: gleicher Ordner und Stamm wie `target`, Endung `.fw.png`.
pub fn extra_fwpng_target(target: &Path) -> PathBuf {
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let lower = name.to_lowercase();
    let stem = if lower.ends_with(".fw.png") {
        &name[..name.len() - 7]
    } else {
        match name.rfind('.') {
            Some(i) if i > 0 => &name[..i],
            _ => name.as_str(),
        }
    };
    target.with_file_name(format!("{stem}.fw.png"))
}

impl Job<'_> {
    /// „Zusätzlich als FW-PNG“ (`output.options.alsoFwpng`, BATCH-11b): schreibt neben die Ausgabe
    /// eine bearbeitbare FW-PNG desselben Elements (z. B. SVG **und** FW-PNG aus „Nachzeichnen“ in
    /// einem Lauf). Konflikte nach der Regel des Rezepts: umbenennen (`name (2).fw.png`),
    /// überspringen (Hinweis) oder überschreiben; „Jedes Mal fragen“ benennt um.
    fn write_extra_fwpng(
        &self,
        item: &Item,
        fmt: OutputFormat,
        target: &Path,
        rep: &mut ItemReport,
    ) -> Result<()> {
        let on = self
            .recipe
            .output
            .options
            .get(crate::steps::files::ALSO_FWPNG)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        if !on || fmt == OutputFormat::FwPng {
            return Ok(());
        }
        let mut extra = extra_fwpng_target(target);
        if extra.exists() {
            match self.recipe.output.conflict {
                crate::recipe::Conflict::Skip => {
                    rep.warnings.push(fp_i18n::tr!(
                        "batch-warn-extra-exists",
                        name = extra
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default()
                    ));
                    return Ok(());
                }
                crate::recipe::Conflict::Overwrite => {}
                _ => {
                    let stem = extra
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let stem = stem.trim_end_matches(".fw.png").to_string();
                    let mut k = 2;
                    while extra.exists() && k < 10_000 {
                        extra = target.with_file_name(format!("{stem} ({k}).fw.png"));
                        k += 1;
                    }
                }
            }
        }
        let tmp = output::temp_path(&extra);
        if let Err(e) = output::write_fwpng(item, &output::long_path(&tmp)) {
            let _ = std::fs::remove_file(output::long_path(&tmp));
            return Err(e);
        }
        if let Err(e) = self.opts.cancel.check() {
            let _ = std::fs::remove_file(output::long_path(&tmp));
            return Err(e);
        }
        output::commit_temp(&tmp, &extra)?;
        rep.extra_targets.push(extra);
        Ok(())
    }
}

fn naming_same(a: &Path, b: &Path) -> bool {
    let k = |p: &Path| {
        let s = p.to_string_lossy().replace('/', "\\");
        if cfg!(windows) { s.to_lowercase() } else { s }
    };
    k(a) == k(b)
}

/// Zahl der Arbeits-Threads: Wunsch, sonst Kerne − 1 (mindestens 1).
pub fn default_threads() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get().saturating_sub(1))
        .unwrap_or(1)
        .max(1)
}

/// Gemeinsamer Ordner mehrerer Pfade (für den Bericht im Ausgabeordner).
fn common_dir(dirs: &[PathBuf]) -> Option<PathBuf> {
    let mut it = dirs.iter();
    let mut acc = it.next()?.clone();
    for d in it {
        while !d.starts_with(&acc) {
            if !acc.pop() {
                return None;
            }
        }
    }
    Some(acc)
}

/// After a cancelled run (button, window close or Ctrl+C in the CLI): delete every temp file
/// `.<name>.fptmp` a planned target may have left behind, so an interrupted run never leaves
/// half-written files next to the outputs (BA-106).
pub fn remove_temp_files(plan: &Plan) {
    for e in &plan.entries {
        for t in std::iter::once(&e.target).chain(e.alt_target.as_ref()) {
            for tmp in [
                output::temp_path(t),
                output::temp_path(&extra_fwpng_target(t)),
            ] {
                let p = output::long_path(&tmp);
                if p.is_file() {
                    let _ = std::fs::remove_file(p);
                }
            }
        }
    }
}

/// Plant einen Auftrag (Trockenlauf-Tabelle) ohne zu schreiben.
pub fn plan(
    recipe: &Recipe,
    items: &[ItemInfo],
    reg: &StepRegistry,
    opts: &PlanOptions,
) -> Result<Plan> {
    let pipe = Pipeline::new(recipe, reg)?;
    naming::plan(recipe, items, pipe.file_only, opts)
}

/// Führt ein Rezept über eine Dateiliste aus (blockiert bis zum Ende). Liefert den Bericht; ein
/// ungültiges Rezept ist ein Fehler, fehlgeschlagene Elemente stehen im Bericht.
pub fn run(recipe: &Recipe, items: &[ItemInfo], opts: &RunOptions) -> Result<Report> {
    let started = iso_now();
    let reg = opts.registry.clone().unwrap_or_else(StepRegistry::global);
    let pipe = Pipeline::new(recipe, &reg)?;
    // one "now" for the plan and for names rebuilt after processing (BA-105)
    let now = opts.plan.now.unwrap_or_else(naming::now_local);
    let plan_opts = PlanOptions { now: Some(now) };
    let plan = naming::plan(recipe, items, pipe.file_only, &plan_opts)?;
    let warnings: Vec<String> = pipe
        .unknown
        .iter()
        .map(|id| fp_i18n::tr!("batch-issue-unknown-step", id = id.clone()))
        .collect();
    send(
        &opts.progress,
        Progress::Started {
            total: plan.entries.len(),
        },
    );
    let mut report = Report {
        format: report::REPORT_FORMAT.into(),
        version: 1,
        recipe: recipe.name.clone(),
        started,
        finished: String::new(),
        dry_run: opts.dry_run,
        cancelled: false,
        totals: Default::default(),
        items: Vec::new(),
        output_dirs: plan.output_dirs.clone(),
        warnings: Vec::new(),
        report_file: None,
        report_html: None,
    };
    if opts.dry_run {
        report.items = plan
            .entries
            .iter()
            .map(|e| {
                let info = &items[e.index];
                let mut r = ItemReport::new(e.index, &info.path, info.bytes);
                r.target = Some(e.target.clone());
                r.status = if e.action == PlanAction::Skip {
                    ItemStatus::Skipped
                } else {
                    ItemStatus::Planned
                };
                r.warnings = e.hint_messages();
                r
            })
            .collect();
    } else {
        let budget_bytes = opts
            .memory_budget
            .or_else(|| recipe.output.memory_limit_mb.map(|m| m * 1024 * 1024))
            .unwrap_or(DEFAULT_MEMORY_MB * 1024 * 1024);
        let threads = opts
            .threads
            .or(recipe.output.threads)
            .unwrap_or_else(default_threads)
            .max(1);
        // KI-Schritte: höchstens wenige Elemente zugleich (Schritt-Grenze, BATCH-T)
        let threads = pipe.max_parallel().map_or(threads, |m| threads.min(m));
        let collect_at = pipe.collect_at();
        let pipe_file_only = pipe.file_only;
        let job = Job {
            recipe,
            items,
            pipe,
            budget: Budget {
                total: budget_bytes.max(1),
                used: Mutex::new(0),
                cv: Condvar::new(),
            },
            opts,
            ask_lock: Mutex::new(None),
            total: items.len(),
            lang: fp_i18n::current_language(),
            renamer: (!pipe_file_only && naming::uses_size(&recipe.rename)).then(|| Renamer {
                now,
                claims: Mutex::new(naming::Claims::from_plan(&plan)),
            }),
        };
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .map_err(|e| BatchError::Io(e.to_string()))?;
        report.items = match collect_at {
            None => pool.install(|| {
                plan.entries
                    .par_iter()
                    .map(|e| fp_i18n::with_language(&job.lang, || job.process(e)))
                    .collect()
            }),
            Some(k) => {
                pool.install(|| fp_i18n::with_language(&job.lang, || run_collect(&job, &plan, k)))
            }
        };
        report.cancelled = opts.cancel.is_cancelled();
        if report.cancelled {
            remove_temp_files(&plan);
        }
        if job.pipe.max_parallel().is_some() {
            // KI-Schritte: Rechenpuffer des Netzes (bis 1,5 GB) ans System zurückgeben (CT-C-01)
            fp_ai::nn::trim();
        }
    }
    report.warnings = warnings;
    report.finished = iso_now();
    report.recount();
    if !opts.dry_run {
        let path = opts.report_path.clone().or_else(|| {
            let wrote = report.totals.done > 0;
            (recipe.output.report
                && wrote
                && recipe.output.target.kind != crate::recipe::TargetKind::SameAsSource)
                .then(|| common_dir(&plan.output_dirs))
                .flatten()
                .map(|d| d.join(report::REPORT_FILE))
        });
        if let Some(p) = path {
            report.report_file = Some(p.clone());
            // HTML-Bericht daneben (BATCH-11b); sein Pfad steht auch im JSON
            if recipe.output.report_html {
                report.report_html = Some(crate::report_html::html_path_for(&p));
            }
            if let Err(e) = report.write(&p) {
                report.report_file = None;
                report.report_html = None;
                report.warnings.push(e.to_string());
            }
            if let Some(hp) = report.report_html.clone()
                && let Err(e) = crate::report_html::write(&report, &hp)
            {
                report.report_html = None;
                report.warnings.push(e.to_string());
            }
        }
    }
    if opts.log {
        let _ = report::append_log(&report);
    }
    send(&opts.progress, Progress::Finished(Box::new(report.clone())));
    Ok(report)
}

/// Zwei Phasen mit Sammel-Schritt an Position `k`: (1) Einzelschritte davor je Element parallel,
/// (2) Sammel-Schritt über alle Elemente in Listenreihenfolge, danach die restlichen Schritte je
/// Ergebnis und Schreiben. Ergebnisse heißen wie das erste Element (`name (2)` … für weitere).
fn run_collect(job: &Job, plan: &Plan, k: usize) -> Vec<ItemReport> {
    let cancel = &job.opts.cancel;
    let active: Vec<&PlanEntry> = plan
        .entries
        .iter()
        .filter(|e| e.action != PlanAction::Skip)
        .collect();
    let phase1: Vec<(usize, Result<Item>)> = active
        .par_iter()
        .map(|e| {
            let info = &job.items[e.index];
            let r = fp_i18n::with_language(&job.lang, || -> Result<Item> {
                cancel.check()?;
                let mut item = item::load(info, false)?;
                let (fmt, _) = resolve_format(job.pipe.format, &info.format);
                let mut ctx = StepContext::new(e.index, job.total, info, cancel, fmt);
                job.pipe.apply_range(0..k, &mut item, &mut ctx)?;
                crate::steps::anim::tag_source(&mut item, info); // BATCH-7: Quellname für Sammel-Schritte
                Ok(item)
            });
            (e.index, r)
        })
        .collect();
    let mut reports: Vec<ItemReport> = plan
        .entries
        .iter()
        .map(|e| {
            let info = &job.items[e.index];
            let mut r = ItemReport::new(e.index, &info.path, info.bytes);
            r.target = Some(e.target.clone());
            r.status = if e.action == PlanAction::Skip {
                ItemStatus::Skipped
            } else {
                ItemStatus::Cancelled
            };
            r
        })
        .collect();
    let mut inputs = Vec::new();
    for (index, r) in phase1 {
        match r {
            Ok(item) => inputs.push((index, item)),
            Err(e) if e.is_cancelled() => {}
            Err(e) => {
                if let Some(rep) = reports.iter_mut().find(|r| r.index == index) {
                    rep.status = ItemStatus::Failed;
                    rep.error = Some(e.to_string());
                }
            }
        }
    }
    let Some(first) = inputs.first().map(|(i, _)| *i) else {
        return reports;
    };
    let Some(first_entry) = plan.entries.iter().find(|e| e.index == first) else {
        return reports;
    };
    let info = &job.items[first];
    let (fmt, _) = resolve_format(job.pipe.format, &info.format);
    let step = &job.pipe.steps[k];
    let mut ctx = StepContext::new(first, job.total, info, cancel, fmt);
    let indices: Vec<usize> = inputs.iter().map(|(i, _)| *i).collect();
    let collected = step
        .step
        .collect(
            inputs.into_iter().map(|(_, it)| it).collect(),
            &step.params,
            &mut ctx,
        )
        .map_err(|e| match e {
            BatchError::Step { .. } | BatchError::Cancelled(_) => e,
            other => BatchError::step(step.step.id(), other.to_string()),
        });
    let outputs = match collected {
        Ok(o) => o,
        Err(e) => {
            for rep in reports.iter_mut().filter(|r| indices.contains(&r.index)) {
                rep.status = if e.is_cancelled() {
                    ItemStatus::Cancelled
                } else {
                    ItemStatus::Failed
                };
                rep.error = (!e.is_cancelled()).then(|| e.to_string());
                rep.step = Some(step.step.id().to_string());
            }
            return reports;
        }
    };
    let (dir, stem, ext) = {
        let t = &first_entry.target;
        let name = t
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let (s, x) = naming::split_extension(&name);
        (t.parent().map(Path::to_path_buf).unwrap_or_default(), s, x)
    };
    let mut out_reports = Vec::new();
    for (n, mut out) in outputs.into_iter().enumerate() {
        let target = crate::steps::anim::collect_target(
            if n == 0 {
                first_entry.target.clone()
            } else {
                dir.join(if ext.is_empty() {
                    format!("{stem} ({})", n + 1)
                } else {
                    format!("{stem} ({}).{ext}", n + 1)
                })
            },
            &dir,
            &ext,
            &out,
        ); // BATCH-7
        let mut rep = ItemReport::new(first, &info.path, 0);
        rep.target = Some(target.clone());
        let t0 = Instant::now();
        let mut c2 = StepContext::new(first, job.total, info, cancel, fmt);
        let r = job
            .pipe
            .apply_range(k + 1..job.pipe.steps.len(), &mut out, &mut c2)
            .and_then(|_| job.write_item(&out, fmt, &target, PlanAction::Write, info, &mut rep))
            .and_then(|_| job.write_extra_fwpng(&out, fmt, &target, &mut rep));
        rep.ms = t0.elapsed().as_millis() as u64;
        rep.warnings.extend(c2.warnings);
        match r {
            Ok(()) => rep.status = ItemStatus::Done,
            Err(e) if e.is_cancelled() => rep.status = ItemStatus::Cancelled,
            Err(e) => {
                rep.status = ItemStatus::Failed;
                rep.error = Some(e.to_string());
            }
        }
        out_reports.push(rep);
    }
    // Eingänge des Sammel-Schritts gelten als verarbeitet (Ziel = erstes Ergebnis)
    let ok = out_reports.iter().any(|r| r.status == ItemStatus::Done);
    for rep in reports.iter_mut().filter(|r| indices.contains(&r.index)) {
        rep.status = if ok {
            ItemStatus::Done
        } else {
            ItemStatus::Failed
        };
        rep.target = out_reports.first().and_then(|r| r.target.clone());
    }
    if let (Some(first_rep), Some(o)) = (
        reports.iter_mut().find(|r| r.index == first),
        out_reports.first(),
    ) {
        first_rep.bytes_out = out_reports.iter().map(|r| r.bytes_out).sum();
        first_rep.error = o.error.clone();
        first_rep.warnings = o.warnings.clone();
    }
    reports
}

/// Laufender Auftrag im Hintergrund (für die Oberfläche).
pub struct JobHandle {
    /// Fortschritt (je Frame mit `try_recv` abfragen).
    pub events: Receiver<Progress>,
    pub cancel: CancelToken,
    thread: Option<std::thread::JoinHandle<Result<Report>>>,
}

impl JobHandle {
    /// Abbrechen (laufende Elemente werden sauber verworfen).
    pub fn cancel(&self) {
        self.cancel.cancel();
    }
    /// Fertig?
    pub fn is_finished(&self) -> bool {
        self.thread.as_ref().is_none_or(|t| t.is_finished())
    }
    /// Auf das Ende warten.
    pub fn join(mut self) -> Result<Report> {
        match self.thread.take() {
            Some(t) => t
                .join()
                .unwrap_or_else(|_| Err(BatchError::Io(fp_i18n::t("batch-err-worker-panic")))),
            None => Err(BatchError::Io(fp_i18n::t("batch-err-worker-panic"))),
        }
    }
}

/// Startet einen Auftrag in einem Hintergrund-Thread. `opts.progress` und `opts.cancel` werden
/// durch die des Handles ersetzt.
pub fn spawn(recipe: Recipe, items: Vec<ItemInfo>, mut opts: RunOptions) -> JobHandle {
    let (tx, rx) = std::sync::mpsc::channel();
    let cancel = CancelToken::new();
    opts.progress = Some(tx);
    opts.cancel = cancel.clone();
    let lang = fp_i18n::current_language();
    let thread = std::thread::Builder::new()
        .name("fp-batch".into())
        .spawn(move || fp_i18n::with_language(&lang, || run(&recipe, &items, &opts)))
        .ok();
    JobHandle {
        events: rx,
        cancel,
        thread,
    }
}

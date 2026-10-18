//! Pipeline (SPEC-BATCH §5.1, §5.4): aufgelöste Schritte eines Rezepts, Anwendung auf ein
//! Element, Erkennung „nur Dateischritte“ (§5.3) und die Live-Vorschau auf einem Proxy-Bild mit
//! Generationszähler (eine neue Anfrage bricht die alte ab).

use crate::error::{BatchError, Result};
use crate::executor::CancelToken;
use crate::item::{self, Item, ItemInfo};
use crate::output::{self, OutputFormat, resolve_format};
use crate::recipe::Recipe;
use crate::step::{Severity, Step, StepContext, StepKind, StepRegistry, with_defaults};
use fp_raster::RgbaImage;
use serde_json::Value;
use std::sync::Arc;

/// Ein aufgelöster, eingeschalteter Schritt.
#[derive(Clone)]
pub struct ResolvedStep {
    /// Index in `recipe.steps`.
    pub index: usize,
    pub step: Arc<dyn Step>,
    /// Parameter mit Vorgaben.
    pub params: Value,
}

/// Was „nur Dateischritte“ mit der Datei tut.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileAction {
    Copy,
    Move,
}

/// Aufgelöste Schritte eines Rezepts.
#[derive(Clone)]
pub struct Pipeline {
    pub steps: Vec<ResolvedStep>,
    /// Unbekannte (übersprungene) Schritt-IDs.
    pub unknown: Vec<String>,
    /// Zielformat der Endkarte (kann `Same` sein, je Element aufgelöst).
    pub format: OutputFormat,
    pub file_only: bool,
}

impl Pipeline {
    /// Löst das Rezept auf. Fehler bei ungültigen Parametern, unbekanntem Format oder ungültigem Regex.
    pub fn new(recipe: &Recipe, reg: &StepRegistry) -> Result<Pipeline> {
        let errors: Vec<String> = recipe
            .validate(reg)
            .into_iter()
            .filter(|i| i.severity == Severity::Error)
            .map(|i| match &i.step_id {
                Some(id) => format!("{}: {}", crate::step::step_label(id), i.message),
                None => i.message,
            })
            .collect();
        if !errors.is_empty() {
            return Err(BatchError::Invalid(errors.join("\n")));
        }
        let format = OutputFormat::parse(&recipe.output.format).unwrap_or(OutputFormat::Same);
        let mut steps = Vec::new();
        let mut unknown = Vec::new();
        for (index, s) in recipe.enabled_steps() {
            match reg.get(&s.id) {
                Some(step) => steps.push(ResolvedStep {
                    index,
                    params: with_defaults(&step.params(), &s.params),
                    step: step.clone(),
                }),
                None => unknown.push(s.id.clone()),
            }
        }
        let file_only = format == OutputFormat::Same && steps.iter().all(|r| r.step.file_only());
        Ok(Pipeline {
            steps,
            unknown,
            format,
            file_only,
        })
    }

    /// Höchstzahl gleichzeitig laufender Elemente nach den Schritten (kleinste Grenze; `None` = frei).
    pub fn max_parallel(&self) -> Option<usize> {
        self.steps
            .iter()
            .filter_map(|r| r.step.max_parallel(&r.params))
            .min()
            .map(|n| n.max(1))
    }

    /// Dateiaktion bei „nur Dateischritten“ (`file.copyOnly` or `file.organize` with
    /// `action: move` → verschieben).
    pub fn file_action(&self) -> FileAction {
        let mv = self.steps.iter().any(|r| {
            (r.step.id() == crate::steps::files::COPY_ONLY
                || r.step.id() == crate::steps::fileops::ORGANIZE)
                && r.params.get("action").and_then(Value::as_str) == Some("move")
        });
        if mv {
            FileAction::Move
        } else {
            FileAction::Copy
        }
    }

    /// Parameters of the first enabled step `id` (with defaults).
    pub fn params_of(&self, id: &str) -> Option<&Value> {
        self.steps
            .iter()
            .find(|r| r.step.id() == id)
            .map(|r| &r.params)
    }

    /// Position des ersten Sammel-Schritts.
    pub fn collect_at(&self) -> Option<usize> {
        self.steps
            .iter()
            .position(|r| r.step.kind() == StepKind::Collect)
    }

    /// B-4: position of the next collect step after `k` (collect stages are chained).
    pub fn next_collect_after(&self, k: usize) -> Option<usize> {
        self.steps
            .iter()
            .enumerate()
            .skip(k + 1)
            .find(|(_, r)| r.step.kind() == StepKind::Collect)
            .map(|(i, _)| i)
    }

    /// Wendet die Einzelschritte `range` an (Abbruch zwischen den Schritten). Schritte, die nicht
    /// auf die Element-Art wirken, bleiben ohne Wirkung (Warnung).
    pub fn apply_range(
        &self,
        range: std::ops::Range<usize>,
        item: &mut Item,
        ctx: &mut StepContext,
    ) -> Result<()> {
        for r in &self.steps[range] {
            ctx.check_cancel()?;
            if r.step.kind() != StepKind::PerItem {
                continue;
            }
            // B-4: `script.condition` (gate mode) skips the following cards
            if crate::steps::condition::take_skip(item) {
                continue;
            }
            if ctx.preview && !r.step.preview_ok() {
                ctx.warn(fp_i18n::tr!(
                    "batch-warn-preview-skipped",
                    step = crate::step::step_label(r.step.id())
                ));
                continue;
            }
            if !r.step.applies_to().contains(&item.kind()) {
                ctx.warn(fp_i18n::tr!(
                    "batch-warn-not-applicable",
                    step = crate::step::step_label(r.step.id())
                ));
                continue;
            }
            ctx.step_index = r.index;
            r.step.apply(item, &r.params, ctx).map_err(|e| match e {
                BatchError::Step { .. }
                | BatchError::Cancelled(_)
                | BatchError::Filtered { .. } => e,
                other => BatchError::step(r.step.id(), other.to_string()),
            })?;
        }
        Ok(())
    }

    /// Alle Einzelschritte bis zum ersten Sammel-Schritt (ohne Sammel-Schritt: alle).
    pub fn apply(&self, item: &mut Item, ctx: &mut StepContext) -> Result<()> {
        let end = self.collect_at().unwrap_or(self.steps.len());
        self.apply_range(0..end, item, ctx)
    }
}

/// Enthält das Rezept nur Dateischritte (kein Dekodieren nötig)?
pub fn is_file_only(recipe: &Recipe, reg: &StepRegistry) -> bool {
    OutputFormat::parse(&recipe.output.format) == Some(OutputFormat::Same)
        && recipe
            .enabled_steps()
            .all(|(_, s)| reg.get(&s.id).is_none_or(|st| st.file_only()))
}

// ------------------------------------------------------------------ Vorschau

/// Optionen der Vorschau.
#[derive(Clone, Debug)]
pub struct PreviewOptions {
    /// Längste Seite des Proxy-Bildes (Standbild).
    pub max_size: u32,
    /// Längste Seite bei Animationen.
    pub max_size_animation: u32,
    /// Höchstzahl Frames im Proxy.
    pub max_frames: usize,
    /// Zusätzlich volle Größe rechnen und kodieren (Dateigröße „Nachher“).
    pub encode_full: bool,
    /// Abbrechen bzw. Generation (neue Anfrage verwirft die alte).
    pub cancel: CancelToken,
}

impl Default for PreviewOptions {
    fn default() -> Self {
        PreviewOptions {
            max_size: 1024,
            max_size_animation: 480,
            max_frames: 150,
            encode_full: false,
            cancel: CancelToken::new(),
        }
    }
}

/// Ergebnis der Vorschau.
#[derive(Clone, Debug)]
pub struct PreviewResult {
    /// Vorher (Proxy).
    pub before: RgbaImage,
    /// Nachher (Proxy, erstes Frame bei Animationen; None bei „nur Datei“).
    pub after: Option<RgbaImage>,
    /// Nachher als Element (Proxy; Animationen mit allen Proxy-Frames).
    pub after_item: Option<Item>,
    /// Pixelmaße der Quelle und (hochgerechnet) des Ergebnisses.
    pub before_size: (u32, u32),
    pub after_size: (u32, u32),
    pub bytes_before: u64,
    /// Kodierte Größe in voller Auflösung (nur mit `encode_full`).
    pub bytes_after: Option<u64>,
    /// Zielformat (aufgelöst).
    pub format: OutputFormat,
    /// Hinweise (übersprungene Schritte, Format-Rückfall …).
    pub warnings: Vec<String>,
    pub ms: u64,
}

fn proxy_image(img: &RgbaImage, max: u32) -> RgbaImage {
    let (w, h) = (img.width() as u32, img.height() as u32);
    let long = w.max(h);
    if long <= max || max == 0 {
        return img.clone();
    }
    let s = max as f64 / long as f64;
    let (nw, nh) = (
        ((w as f64 * s).round() as u32).max(1),
        ((h as f64 * s).round() as u32).max(1),
    );
    let mut r = fp_raster::resample::resize(img, nw, nh, fp_raster::Sampler::Bilinear);
    r.set_origin(0, 0);
    r
}

/// Verkleinert ein Element auf Vorschaugröße (Animationen: auch die Zahl der Frames).
pub fn proxy(item: &mut Item, opts: &PreviewOptions) -> Result<f64> {
    item.flatten_document()?;
    let (w, h) = item.dimensions();
    let long = w.max(h).max(1);
    match item {
        Item::Still(s) => {
            s.image = proxy_image(&s.image, opts.max_size);
            Ok((s.image.width().max(s.image.height()) as f64 / long as f64).min(1.0))
        }
        Item::Animation(a) => {
            if a.frames.len() > opts.max_frames {
                a.frames.truncate(opts.max_frames);
            }
            for f in &mut a.frames {
                opts.cancel.check()?;
                f.image = proxy_image(&f.image, opts.max_size_animation);
            }
            if let Some(f) = a.frames.first() {
                a.width = f.image.width() as u32;
                a.height = f.image.height() as u32;
            }
            Ok((a.width.max(a.height) as f64 / long as f64).min(1.0))
        }
        _ => Ok(1.0),
    }
}

/// Live-Vorschau für eine Datei: Proxy dekodieren, Schritte mit `preview_ok()` anwenden, optional
/// volle Kodierung für die Dateigröße. Bricht ab, sobald `opts.cancel` abgebrochen bzw. die
/// Generation veraltet ist.
pub fn preview(
    recipe: &Recipe,
    info: &ItemInfo,
    reg: &StepRegistry,
    opts: &PreviewOptions,
) -> Result<PreviewResult> {
    let t0 = std::time::Instant::now();
    crate::security::check(recipe, crate::security::current_caller())?;
    let pipe = Pipeline::new(recipe, reg)?;
    let original = item::load(info, false)?;
    opts.cancel.check()?;
    let before_size = original.dimensions();
    let mut before_item = original.clone();
    let scale = proxy(&mut before_item, opts)?;
    let before = before_item.preview_image()?;
    let (format, fallback) = resolve_format(pipe.format, &info.format);
    let mut ctx = StepContext::new(0, 1, info, &opts.cancel, format);
    ctx.preview = true;
    ctx.proxy_scale = Some(scale); // BATCH-11a: Proxy-Maßstab für Pixelangaben der Schritte
    if let Some(f) = fallback {
        ctx.warn(f);
    }
    let mut after_item = before_item;
    pipe.apply(&mut after_item, &mut ctx)?;
    opts.cancel.check()?;
    let after = after_item.preview_image().ok();
    let (aw, ah) = after_item.dimensions();
    let after_size = if scale > 0.0 && scale < 1.0 {
        (
            (aw as f64 / scale).round() as u32,
            (ah as f64 / scale).round() as u32,
        )
    } else {
        (aw, ah)
    };
    let mut warnings = std::mem::take(&mut ctx.warnings);
    let mut bytes_after = None;
    let mut after_size = after_size;
    if opts.encode_full && pipe.file_only {
        // nur Dateischritte (umbenennen/kopieren): die Datei wird unverändert kopiert (BATCH-T)
        bytes_after = Some(info.bytes);
    } else if opts.encode_full {
        let mut full = original;
        let mut fctx = StepContext::new(0, 1, info, &opts.cancel, format);
        pipe.apply(&mut full, &mut fctx)?;
        opts.cancel.check()?;
        after_size = full.dimensions();
        let (bytes, w) = output::encode(&full, format, &recipe.output.options)?;
        opts.cancel.check()?;
        bytes_after = Some(bytes.len() as u64);
        warnings.extend(w);
    }
    Ok(PreviewResult {
        before,
        after,
        after_item: Some(after_item),
        before_size,
        after_size,
        bytes_before: info.bytes,
        bytes_after,
        format,
        warnings,
        ms: t0.elapsed().as_millis() as u64,
    })
}

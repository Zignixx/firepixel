//! `ai.upscale` – AI upscaling ×2/×4 with Real-ESRGAN (BA-109, `docs/BATCH.md` §3.4).
//!
//! The network is RRDBNet with the official Real-ESRGAN weights (BSD-3-Clause), computed by the
//! own CPU inference of `fp-ai` ([`fp_ai::esrgan`]). Models are never downloaded here: a missing
//! model is a clean step error with the hint to "Manage AI Models..." (as `background.remove`).
//!
//! * `scale` 2/4 (default 4), `model` auto/realEsrganX4/realEsrganX2 (auto = the model of the
//!   factor). Other combinations: ×4 with the ×2 model runs the network twice, ×2 with the ×4
//!   model scales the ×4 result down by half with Lanczos 3 (as `outscale` in Real-ESRGAN).
//! * `alpha` network/bicubic: transparency through the same network as a grey image (default, as
//!   Real-ESRGAN) or bicubic; images without transparency skip it.
//! * Preview ("yes (region)" in the spec): the network runs only on a detail in the middle of the
//!   preview image (at most [`PREVIEW_REGION`] px square), the rest is scaled bicubically – the run
//!   computes the whole image.

use super::background::{model_missing, p_str};
use super::geometry::{MAX_SIDE, ResizeFilter, resample, too_large};
use crate::error::{BatchError, Result};
use crate::executor::CancelToken;
use crate::item::{Item, ItemKind};
use crate::step::{Issue, ParamDef, Severity, Step, StepContext, StepGroup, StepRegistry};
use fp_ai::esrgan::AlphaMode;
use fp_ai::{AiError, ModelId};
use fp_raster::{IRect, RgbaImage};
use serde_json::Value;
use std::sync::Arc;

/// ID "AI upscale".
pub const UPSCALE: &str = "ai.upscale";
/// Preview: the network runs on a centre detail of at most this size (preview px).
pub const PREVIEW_REGION: u32 = 96;

/// Registers the step.
pub fn register(r: &mut StepRegistry) {
    r.register(Arc::new(Upscale));
}

/// Factor from the parameters (`"2"`/`"4"` or a number; anything else = 4).
pub fn scale_param(p: &Value) -> u32 {
    let v = match p.get("scale") {
        Some(Value::String(s)) => s.trim().trim_start_matches(['x', '×']).parse().ok(),
        Some(v) => v.as_u64().or_else(|| v.as_f64().map(|f| f.round() as u64)),
        None => None,
    };
    if v == Some(2) { 2 } else { 4 }
}

/// Model from the parameters (`auto` = the model of the factor).
pub fn model_param(p: &Value) -> ModelId {
    match p_str(p, "model", "auto") {
        "realEsrganX2" => ModelId::RealEsrganX2,
        "realEsrganX4" => ModelId::RealEsrganX4,
        _ if scale_param(p) == 2 => ModelId::RealEsrganX2,
        _ => ModelId::RealEsrganX4,
    }
}

/// Runs `f` on its own thread and passes a batch cancel on (CT-C-01: a worker of the batch pool
/// must not wait inside `fp_ai::nn::pool().install`, see `background::ai_subject_mask`).
fn run_ai<T: Send>(
    cancel: &CancelToken,
    f: impl FnOnce(&fp_ai::Progress) -> std::result::Result<T, AiError> + Send,
) -> std::result::Result<T, AiError> {
    let ai_cancel = fp_ai::Cancel::new();
    let prog = fp_ai::Progress::with_cancel(ai_cancel.clone());
    let lang = fp_i18n::current_language();
    std::thread::scope(|s| {
        let worker = s.spawn(|| fp_i18n::with_language(&lang, || f(&prog)));
        while !worker.is_finished() {
            if cancel.is_cancelled() {
                ai_cancel.cancel();
            }
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
        worker
            .join()
            .unwrap_or_else(|_| Err(AiError::Input(fp_i18n::t("batch-err-worker-panic"))))
    })
}

fn ai_error(e: AiError) -> BatchError {
    match e {
        AiError::Cancelled => BatchError::cancelled(),
        AiError::ModelMissing { model, .. } => model_missing(UPSCALE, model),
        e => BatchError::step(UPSCALE, e.to_string()),
    }
}

/// Upscales one image by `scale` with `model` (network passes as needed); `region` limits the
/// first pass (preview detail).
fn upscale(
    img: &RgbaImage,
    region: Option<IRect>,
    scale: u32,
    model: ModelId,
    alpha: AlphaMode,
    cancel: &CancelToken,
) -> Result<RgbaImage> {
    let native = fp_ai::esrgan::model_scale(model).unwrap_or(4) as u32;
    let mut out = run_ai(cancel, |prog| {
        let first = fp_ai::esrgan::upscale_image(img, region, model, alpha, prog)?;
        if native < scale {
            // ×4 with the ×2 model: second pass on the whole intermediate result
            fp_ai::esrgan::upscale_image(&first, None, model, alpha, prog)
        } else {
            Ok(first)
        }
    })
    .map_err(ai_error)?;
    if native > scale {
        // ×2 with the ×4 model: down by the remaining factor
        let (w, h) = (out.width() as u32, out.height() as u32);
        out = resample(
            &out,
            (w * scale / native).max(1),
            (h * scale / native).max(1),
            ResizeFilter::Lanczos3,
            cancel,
        )?;
    }
    Ok(out)
}

/// Preview: bicubic ×`scale` of the whole image with the network result of a centre detail.
fn preview(
    img: &RgbaImage,
    scale: u32,
    model: ModelId,
    alpha: AlphaMode,
    cancel: &CancelToken,
) -> Result<RgbaImage> {
    let (w, h) = (img.width() as u32, img.height() as u32);
    let mut base = resample(img, w * scale, h * scale, ResizeFilter::Bicubic, cancel)?;
    let (rw, rh) = (w.min(PREVIEW_REGION), h.min(PREVIEW_REGION));
    let (x0, y0) = ((w - rw) / 2, (h - rh) / 2);
    let b = img.bounds();
    let r = IRect::from_xywh(b.left + x0 as i32, b.top + y0 as i32, rw as i32, rh as i32);
    let detail = upscale(img, Some(r), scale, model, alpha, cancel)?;
    let bw = base.width();
    let dw = detail.width();
    let (ox, oy) = ((x0 * scale) as usize, (y0 * scale) as usize);
    let dh = detail.height().min(base.height().saturating_sub(oy));
    let cols = dw.min(bw.saturating_sub(ox));
    let src = detail.data();
    let dst = base.data_mut();
    for y in 0..dh {
        dst[(oy + y) * bw + ox..][..cols].copy_from_slice(&src[y * dw..][..cols]);
    }
    Ok(base)
}

fn model_issue(p: &Value) -> Option<Issue> {
    let model = model_param(p);
    (!fp_ai::models::available(model)).then(|| {
        Issue::global(
            Severity::Warning,
            "aiModelMissing",
            fp_i18n::tr!(
                "batch-bg-err-model-missing",
                model = model.display_name(),
                folder = fp_ai::models::download_folder().display().to_string()
            ),
        )
    })
}

struct Upscale;

impl Step for Upscale {
    fn id(&self) -> &str {
        UPSCALE
    }
    fn group(&self) -> StepGroup {
        StepGroup::Color
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::choice("scale", &["2", "4"], "4").beginner(),
            ParamDef::choice("model", &["auto", "realEsrganX4", "realEsrganX2"], "auto"),
            ParamDef::choice("alpha", &["network", "bicubic"], "network"),
        ]
    }
    fn validate(&self, params: &Value) -> Vec<Issue> {
        // `scale` may also be written as a number (4) in a recipe
        let mut p = params.clone();
        if let Some(o) = p.as_object_mut()
            && let Some(v) = o.get("scale").filter(|v| v.is_number())
        {
            let n = v.as_f64().unwrap_or(0.0);
            let txt = if n == 2.0 || n == 4.0 {
                format!("{n}")
            } else {
                "?".into()
            };
            o.insert("scale".into(), Value::String(txt));
        }
        let mut v = crate::step::validate_params(&self.params(), &p);
        v.extend(model_issue(params));
        v
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn max_parallel(&self, _params: &Value) -> Option<usize> {
        // the network already uses all cores per tile; one item at a time halves the memory
        Some(1)
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let scale = scale_param(params);
        let model = model_param(params);
        let alpha = AlphaMode::from_name(p_str(params, "alpha", "network")).unwrap_or_default();
        if !fp_ai::models::available(model) {
            return Err(model_missing(UPSCALE, model));
        }
        let (w, h) = item.dimensions();
        let (nw, nh) = (w as i64 * scale as i64, h as i64 * scale as i64);
        if nw > MAX_SIDE
            || nh > MAX_SIDE
            || (nw as u128) * (nh as u128) > fp_raster::MAX_PIXELS as u128
        {
            return Err(too_large(nw, nh));
        }
        let cancel = ctx.cancel;
        let in_preview = ctx.preview;
        item.map_images(|img| {
            cancel.check()?;
            *img = if in_preview {
                preview(img, scale, model, alpha, cancel)?
            } else {
                upscale(img, None, scale, model, alpha, cancel)?
            };
            Ok(())
        })?;
        if in_preview {
            ctx.warn(fp_i18n::tr!(
                "batch-upscale-preview-region",
                size = PREVIEW_REGION as i64
            ));
        }
        Ok(())
    }
    fn summary(&self, params: &Value) -> String {
        fp_i18n::tr!(
            "batch-upscale-summary",
            scale = scale_param(params) as i64,
            model = model_param(params).display_name()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn params_and_models() {
        assert_eq!(scale_param(&json!({})), 4);
        assert_eq!(scale_param(&json!({"scale": "2"})), 2);
        assert_eq!(scale_param(&json!({"scale": 2})), 2);
        assert_eq!(scale_param(&json!({"scale": "x2"})), 2);
        assert_eq!(model_param(&json!({"scale": "2"})), ModelId::RealEsrganX2);
        assert_eq!(model_param(&json!({})), ModelId::RealEsrganX4);
        assert_eq!(
            model_param(&json!({"scale": "2", "model": "realEsrganX4"})),
            ModelId::RealEsrganX4
        );
    }
}

//! `format.multiPage` (B-4, `docs/BATCH.md` §3.2): collect step that combines all items into one
//! multi-page TIFF or PDF.
//!
//! The step builds the pages (order, frames, page size policy, DPI) and hands them on as one
//! [`Item::Animation`] whose frames are the pages (sizes may differ) and whose `Meta::extra` carries
//! [`EXTRA_MULTI_PAGE`]. The TIFF encoder (`meta::encoders::write_tiff_pages`) then writes an IFD
//! chain, the PDF encoder ([`encode_pdf`]) one page per frame with the page size from the DPI.

use crate::error::{BatchError, Result};
use crate::item::{Animation, Frame, Item, ItemKind};
use crate::output::{self, OutputFormat};
use crate::recipe::Recipe;
use crate::step::{
    Issue, ParamDef, Severity, Step, StepContext, StepGroup, StepKind, StepRegistry,
};
use crate::steps::anim::{EXTRA_OUTPUT_STEM, EXTRA_SOURCE_NAME};
use fp_raster::{IRect, RgbaImage};
use serde_json::{Map, Value, json};
use std::sync::Arc;

/// ID "Multi-page file".
pub const MULTI_PAGE: &str = "format.multiPage";
/// `Meta::extra`: the item's frames are the pages of one multi-page file (value `true`).
pub const EXTRA_MULTI_PAGE: &str = "format.multiPage";
/// A4 in millimetres.
pub const A4_MM: (f64, f64) = (210.0, 297.0);

/// Registers the step and the PDF encoder.
pub fn register(r: &mut StepRegistry) {
    output::register_encoder(OutputFormat::Pdf, encode_pdf);
    r.register(Arc::new(MultiPage));
}

/// Is this item a multi-page result of `format.multiPage`?
pub fn is_multi_page(item: &Item) -> bool {
    item.meta()
        .and_then(|m| m.extra.get(EXTRA_MULTI_PAGE))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// Pages of an item: all frames of an animation, otherwise the flat image.
pub fn pages_of(item: &Item) -> Result<Vec<RgbaImage>> {
    match item {
        Item::Animation(a) if !a.frames.is_empty() => {
            Ok(a.frames.iter().map(|f| f.image.clone()).collect())
        }
        other => Ok(vec![other.preview_image()?]),
    }
}

/// PDF encoder ("Save as: PDF"): one page per frame (multi-page results and animations) or one
/// page; the page size follows the DPI of the item (72 when unknown).
pub fn encode_pdf(item: &Item, _options: &Map<String, Value>) -> Result<Vec<u8>> {
    let pages = pages_of(item)?;
    let dpi = item
        .meta()
        .and_then(|m| m.dpi)
        .filter(|d| d.is_finite() && *d > 0.0)
        .unwrap_or(72.0) as f64;
    let list: Vec<fp_export::vector_out::PdfPage> = pages
        .iter()
        .map(|img| fp_export::vector_out::PdfPage {
            image: img,
            width_pt: img.width() as f64 * 72.0 / dpi,
            height_pt: img.height() as f64 * 72.0 / dpi,
        })
        .collect();
    let title = item
        .meta()
        .and_then(|m| {
            m.extra
                .get(EXTRA_OUTPUT_STEM)
                .or_else(|| m.extra.get(EXTRA_SOURCE_NAME))
        })
        .and_then(Value::as_str)
        .unwrap_or("Firepixel");
    fp_export::vector_out::write_pdf(&list, title).map_err(|e| BatchError::Encode(e.to_string()))
}

/// Recipe-level check: `format.multiPage` needs "Save as" TIFF or PDF.
pub fn validate_recipe(recipe: &Recipe) -> Vec<Issue> {
    let fmt = OutputFormat::parse(&recipe.output.format);
    let ok = matches!(fmt, Some(OutputFormat::Tiff | OutputFormat::Pdf));
    recipe
        .enabled_steps()
        .filter(|(_, s)| s.id == MULTI_PAGE && !ok)
        .map(|(i, _)| {
            Issue::step(
                i,
                MULTI_PAGE,
                Severity::Error,
                "multiPageFormat",
                fp_i18n::t("batch-issue-multipage-format"),
            )
        })
        .collect()
}

fn p_str<'a>(p: &'a Value, k: &str, d: &'a str) -> &'a str {
    p.get(k).and_then(Value::as_str).unwrap_or(d)
}
fn p_int(p: &Value, k: &str, d: i64) -> i64 {
    p.get(k)
        .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f.round() as i64)))
        .unwrap_or(d)
}

/// Source-over with straight alpha.
fn over(d: &mut [u8; 4], s: [u8; 4]) {
    let sa = s[3] as u32;
    if sa == 0 {
        return;
    }
    if sa == 255 {
        *d = s;
        return;
    }
    let da = d[3] as u32;
    let oa = sa * 255 + da * (255 - sa); // ×255
    if oa == 0 {
        return;
    }
    for c in 0..3 {
        let v = (s[c] as u32 * sa * 255 + d[c] as u32 * da * (255 - sa)) / oa;
        d[c] = v.min(255) as u8;
    }
    d[3] = (oa / 255).min(255) as u8;
}

/// `img` centred on a `w × h` canvas filled with `bg` (scaled down to fit when larger, never
/// enlarged).
pub fn place(img: &RgbaImage, w: usize, h: usize, bg: [u8; 4]) -> Result<RgbaImage> {
    let (iw, ih) = (img.width().max(1), img.height().max(1));
    let s = (w as f64 / iw as f64).min(h as f64 / ih as f64).min(1.0);
    let src = if s < 1.0 {
        let (nw, nh) = (
            ((iw as f64 * s).round() as u32).max(1),
            ((ih as f64 * s).round() as u32).max(1),
        );
        let mut r = fp_raster::resample::resize(img, nw, nh, fp_raster::Sampler::Bicubic);
        r.set_origin(0, 0);
        r
    } else {
        img.clone()
    };
    let (sw, sh) = (src.width().min(w), src.height().min(h));
    let (x0, y0) = ((w - sw) / 2, (h - sh) / 2);
    let mut px = vec![bg; w * h];
    let d = src.data();
    for y in 0..sh {
        for x in 0..sw {
            over(&mut px[(y0 + y) * w + x0 + x], d[y * src.width() + x]);
        }
    }
    RgbaImage::from_raw(IRect::new(0, 0, w as i32, h as i32), px)
        .map_err(|e| BatchError::Decode(e.to_string()))
}

struct MultiPage;

impl Step for MultiPage {
    fn id(&self) -> &str {
        MULTI_PAGE
    }
    fn group(&self) -> StepGroup {
        StepGroup::Format
    }
    fn kind(&self) -> StepKind {
        StepKind::Collect
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::choice("order", &["list", "name", "nameDesc", "reverse"], "list").beginner(),
            ParamDef::choice("pageSize", &["each", "largest", "a4"], "each").beginner(),
            ParamDef::choice("orientation", &["auto", "portrait", "landscape"], "auto")
                .when("pageSize", &["a4"]),
            ParamDef::int("margin", 0, 50, 10)
                .unit("mm")
                .when("pageSize", &["a4"]),
            ParamDef::int("dpi", 36, 1200, 150).unit("dpi").beginner(),
            ParamDef::choice("frames", &["all", "first"], "all"),
            ParamDef::color("background", "#ffffff"),
            ParamDef::text("name", ""),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn preview_ok(&self) -> bool {
        false
    }
    fn apply(&self, _item: &mut Item, _params: &Value, _ctx: &mut StepContext) -> Result<()> {
        Ok(())
    }
    fn collect(
        &self,
        items: Vec<Item>,
        params: &Value,
        ctx: &mut StepContext,
    ) -> Result<Vec<Item>> {
        let dpi = p_int(params, "dpi", 150).clamp(36, 1200) as f64;
        let bg = crate::step::parse_color(p_str(params, "background", "#ffffff"))
            .unwrap_or([255, 255, 255, 255]);
        let all_frames = p_str(params, "frames", "all") == "all";
        let mut named: Vec<(String, Item)> = items
            .into_iter()
            .map(|it| {
                let n = it
                    .meta()
                    .and_then(|m| m.extra.get(EXTRA_SOURCE_NAME))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                (n, it)
            })
            .collect();
        match p_str(params, "order", "list") {
            "name" => named.sort_by_key(|a| a.0.to_lowercase()),
            "nameDesc" => named.sort_by_key(|b| std::cmp::Reverse(b.0.to_lowercase())),
            "reverse" => named.reverse(),
            _ => {}
        }
        let Some(first_meta) = named.first().and_then(|(_, i)| i.meta().cloned()) else {
            return Err(BatchError::Invalid(fp_i18n::t("batch-anim-err-empty")));
        };
        let mut pages: Vec<RgbaImage> = Vec::new();
        for (_, item) in &named {
            ctx.check_cancel()?;
            let mut p = pages_of(item)?;
            if !all_frames {
                p.truncate(1);
            }
            pages.extend(p);
        }
        if pages.is_empty() {
            return Err(BatchError::Invalid(fp_i18n::t("batch-anim-err-empty")));
        }
        let pages = match p_str(params, "pageSize", "each") {
            "largest" => {
                let w = pages.iter().map(|p| p.width()).max().unwrap_or(1);
                let h = pages.iter().map(|p| p.height()).max().unwrap_or(1);
                pages
                    .iter()
                    .map(|p| place(p, w, h, bg))
                    .collect::<Result<Vec<_>>>()?
            }
            "a4" => {
                let px = |mm: f64| (mm / 25.4 * dpi).round().max(1.0) as usize;
                let margin = p_int(params, "margin", 10).clamp(0, 50) as f64;
                let orient = p_str(params, "orientation", "auto");
                let mut out = Vec::with_capacity(pages.len());
                for p in &pages {
                    ctx.check_cancel()?;
                    let landscape = match orient {
                        "landscape" => true,
                        "portrait" => false,
                        _ => p.width() > p.height(),
                    };
                    let (pw, ph) = if landscape { (A4_MM.1, A4_MM.0) } else { A4_MM };
                    let (w, h) = (px(pw), px(ph));
                    let m = px(margin).min(w / 2 - 1).min(h / 2 - 1);
                    let inner = place(p, w - 2 * m, h - 2 * m, bg)?;
                    out.push(place(&inner, w, h, bg)?);
                }
                out
            }
            _ => pages,
        };
        let width = pages.iter().map(|p| p.width()).max().unwrap_or(1) as u32;
        let height = pages.iter().map(|p| p.height()).max().unwrap_or(1) as u32;
        let mut meta = first_meta;
        meta.dpi = Some(dpi as f32);
        meta.has_transparency = pages.iter().any(|p| p.data().iter().any(|q| q[3] < 255));
        meta.extra.insert(EXTRA_MULTI_PAGE.into(), json!(true));
        let name = p_str(params, "name", "").trim();
        if !name.is_empty() {
            let mut hints = Vec::new();
            let stem = crate::naming::sanitize_stem(name, &mut hints);
            meta.extra.insert(EXTRA_OUTPUT_STEM.into(), json!(stem));
        }
        Ok(vec![Item::Animation(Animation {
            width,
            height,
            frames: pages
                .into_iter()
                .map(|image| Frame { image, delay_ms: 0 })
                .collect(),
            loop_count: 0,
            meta,
        })])
    }
    fn summary(&self, params: &Value) -> String {
        let size = p_str(params, "pageSize", "each");
        fp_i18n::t(&crate::step::value_key(MULTI_PAGE, "pageSize", size))
    }
}

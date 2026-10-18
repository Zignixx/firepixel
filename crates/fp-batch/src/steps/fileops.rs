//! File steps from the batch spec (`docs/BATCH.md` §3.1):
//!
//! - `file.organize` – sort the outputs into subfolders by date, type, size class, orientation,
//!   camera or a free pattern (`{date:YYYY}/{ext}`). The folder is part of the plan (dry run);
//!   with only file steps the files are copied or moved without decoding.
//! - `file.filterSkip` – process only items that match conditions (pixel size, file size,
//!   transparency, animation, name glob); the others are skipped or copied unchanged.
//! - `file.dedupe` – collect step: drops exact duplicates (same pixels) or similar images
//!   (64-bit difference hash, Hamming distance) in list order; the first one stays.
//! - `file.timestamps` – modification time of the written file: source, EXIF capture date or
//!   a fixed date.

use crate::error::{BatchError, Result};
use crate::item::{Item, ItemInfo, ItemKind};
use crate::recipe::Recipe;
use crate::step::{Issue, ParamDef, Step, StepContext, StepGroup, StepKind, StepRegistry};
use chrono::{NaiveDateTime, TimeZone};
use fp_raster::RgbaImage;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::SystemTime;

/// ID "Sort into subfolders".
pub const ORGANIZE: &str = "file.organize";
/// ID "Process only some".
pub const FILTER_SKIP: &str = "file.filterSkip";
/// ID "Detect duplicate images".
pub const DEDUPE: &str = "file.dedupe";
/// ID "Set date".
pub const TIMESTAMPS: &str = "file.timestamps";
/// `Meta::extra`: list index of the item, set by the executor before collect steps.
pub const EXTRA_INDEX: &str = "batch.index";

/// Registers the steps of this file.
pub fn register(r: &mut StepRegistry) {
    r.register(Arc::new(Organize));
    r.register(Arc::new(FilterSkip));
    r.register(Arc::new(Dedupe));
    r.register(Arc::new(Timestamps));
}

fn p_str<'a>(p: &'a Value, k: &str, d: &'a str) -> &'a str {
    p.get(k).and_then(Value::as_str).unwrap_or(d)
}
fn p_f64(p: &Value, k: &str, d: f64) -> f64 {
    p.get(k)
        .and_then(Value::as_f64)
        .filter(|f| f.is_finite())
        .unwrap_or(d)
}
fn p_bool(p: &Value, k: &str, d: bool) -> bool {
    p.get(k).and_then(Value::as_bool).unwrap_or(d)
}

/// List index of an item inside a collect step (fallback `default`).
pub fn item_index(item: &Item, default: usize) -> usize {
    item.meta()
        .and_then(|m| m.extra.get(EXTRA_INDEX))
        .and_then(Value::as_u64)
        .map_or(default, |i| i as usize)
}

/// Source name of an item inside a collect step (empty if unknown).
fn source_name(item: &Item) -> String {
    item.meta()
        .and_then(|m| m.extra.get(crate::steps::anim::EXTRA_SOURCE_NAME))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

// ------------------------------------------------------------------ Sort into subfolders

/// Default patterns of the `by` choices.
fn preset_pattern(by: &str) -> &'static str {
    match by {
        "type" => "{ext}",
        "size" => "{size}",
        "orientation" => "{orientation}",
        "camera" => "{camera}",
        _ => "{date:YYYY}/{date:MM}",
    }
}

fn token_regex() -> &'static regex::Regex {
    static R: OnceLock<regex::Regex> = OnceLock::new();
    R.get_or_init(|| {
        regex::Regex::new(
            r"\{(date|exif-date|modified|ext|format|size|orientation|camera|width|height)(?::([^{}]*))?\}",
        )
        .expect("token regex")
    })
}

/// Unknown `{…}` tokens of a pattern (for the validation).
pub fn unknown_tokens(pattern: &str) -> Vec<String> {
    static ANY: OnceLock<regex::Regex> = OnceLock::new();
    let any = ANY.get_or_init(|| regex::Regex::new(r"\{[^{}]*\}").expect("any token"));
    any.find_iter(pattern)
        .map(|m| m.as_str().to_string())
        .filter(|t| !token_regex().is_match(t))
        .collect()
}

/// Size class by the longer side: `small` < 640, `medium` < 1600, `large` < 4000, else `huge`.
pub fn size_class(w: u32, h: u32) -> &'static str {
    match w.max(h) {
        0 => "unknown",
        l if l < 640 => "small",
        l if l < 1600 => "medium",
        l if l < 4000 => "large",
        _ => "huge",
    }
}

fn local_from_system(t: SystemTime) -> NaiveDateTime {
    chrono::DateTime::<chrono::Local>::from(t).naive_local()
}

/// Expands an organize pattern for one item to a relative folder (segments separated by `/` or
/// `\`; every segment is sanitized, empty segments are dropped).
pub fn expand_pattern(pattern: &str, info: &ItemInfo, now: NaiveDateTime) -> PathBuf {
    let modified = info.modified.map(local_from_system).unwrap_or(now);
    let taken = info
        .exif
        .as_ref()
        .and_then(|e| e.date_taken)
        .unwrap_or(modified);
    let ext = {
        let (_, x) = crate::naming::split_extension(&info.file_name());
        if x.is_empty() {
            "noext".to_string()
        } else {
            x.to_lowercase()
        }
    };
    let text = token_regex().replace_all(pattern, |c: &regex::Captures| {
        let fmt = c.get(2).map(|m| m.as_str()).filter(|s| !s.is_empty());
        match &c[1] {
            "date" | "exif-date" => crate::naming::format_date(&taken, fmt.unwrap_or("YYYY-MM-DD")),
            "modified" => crate::naming::format_date(&modified, fmt.unwrap_or("YYYY-MM-DD")),
            "ext" => ext.clone(),
            "format" => {
                if info.format.is_empty() {
                    ext.clone()
                } else {
                    info.format.clone()
                }
            }
            "size" => size_class(info.width, info.height).to_string(),
            "orientation" => match (info.width, info.height) {
                (0, _) | (_, 0) => "unknown",
                (w, h) if w > h => "landscape",
                (w, h) if w < h => "portrait",
                _ => "square",
            }
            .to_string(),
            "camera" => {
                let e = info.exif.as_ref();
                let make = e.and_then(|e| e.make.clone()).unwrap_or_default();
                let model = e.and_then(|e| e.model.clone()).unwrap_or_default();
                let (make, model) = (make.trim(), model.trim());
                let s = if model.to_lowercase().starts_with(&make.to_lowercase()) {
                    model.to_string()
                } else {
                    format!("{make} {model}")
                };
                let s = s.trim().to_string();
                if s.is_empty() {
                    "unknown-camera".into()
                } else {
                    s
                }
            }
            "width" => info.width.to_string(),
            _ => info.height.to_string(),
        }
    });
    let mut out = PathBuf::new();
    for seg in text.split(['/', '\\']) {
        let seg = seg.trim();
        if seg.is_empty() || seg == "." {
            continue;
        }
        let mut hints = Vec::new();
        let s = if seg == ".." {
            "_".to_string()
        } else {
            crate::naming::sanitize_stem(seg, &mut hints)
        };
        out.push(s);
    }
    out
}

/// Subfolder of `file.organize` for an item (`None` without that step).
pub fn organize_subdir(recipe: &Recipe, info: &ItemInfo, now: NaiveDateTime) -> Option<PathBuf> {
    let (_, s) = recipe.enabled_steps().find(|(_, s)| s.id == ORGANIZE)?;
    let by = p_str(&s.params, "by", "date");
    let pattern = if by == "pattern" {
        p_str(&s.params, "pattern", "")
    } else {
        preset_pattern(by)
    };
    let dir = expand_pattern(pattern, info, now);
    (!dir.as_os_str().is_empty()).then_some(dir)
}

struct Organize;

impl Step for Organize {
    fn id(&self) -> &str {
        ORGANIZE
    }
    fn group(&self) -> StepGroup {
        StepGroup::Files
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::choice(
                "by",
                &["date", "type", "size", "orientation", "camera", "pattern"],
                "date",
            )
            .beginner(),
            ParamDef::text("pattern", "{date:YYYY}/{ext}")
                .beginner()
                .when("by", &["pattern"]),
            ParamDef::choice("action", &["copy", "move"], "copy").beginner(),
        ]
    }
    fn validate(&self, params: &Value) -> Vec<Issue> {
        let mut v = crate::step::validate_params(&self.params(), params);
        if p_str(params, "by", "date") == "pattern" {
            let pat = p_str(params, "pattern", "{date:YYYY}/{ext}");
            if pat.trim().is_empty() {
                v.push(Issue::param(
                    "pattern",
                    fp_i18n::t("batch-err-organize-empty"),
                ));
            }
            let bad = unknown_tokens(pat);
            if !bad.is_empty() {
                v.push(Issue::param(
                    "pattern",
                    fp_i18n::tr!("batch-err-organize-token", tokens = bad.join(", ")),
                ));
            }
        }
        v
    }
    fn preview_ok(&self) -> bool {
        false
    }
    fn file_only(&self) -> bool {
        true
    }
    fn apply(&self, _item: &mut Item, _params: &Value, _ctx: &mut StepContext) -> Result<()> {
        // the folder is part of the plan (`naming::plan`), copying/moving is the executor's job
        Ok(())
    }
}

// ------------------------------------------------------------------ Process only some

/// Glob (`*`, `?`, alternatives separated by `;`) → case-insensitive regex for the whole name.
pub fn glob_regex(glob: &str) -> std::result::Result<Option<regex::Regex>, String> {
    let alts: Vec<String> = glob
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|g| {
            let mut r = String::new();
            for c in g.chars() {
                match c {
                    '*' => r.push_str(".*"),
                    '?' => r.push('.'),
                    c => r.push_str(&regex::escape(&c.to_string())),
                }
            }
            r
        })
        .collect();
    if alts.is_empty() {
        return Ok(None);
    }
    regex::RegexBuilder::new(&format!("^(?:{})$", alts.join("|")))
        .case_insensitive(true)
        .build()
        .map(Some)
        .map_err(|e| e.to_string())
}

fn item_has_transparency(item: &Item) -> bool {
    match item {
        Item::Still(s) => s.image.data().iter().any(|p| p[3] < 255),
        Item::Animation(a) => a
            .frames
            .iter()
            .any(|f| f.image.data().iter().any(|p| p[3] < 255)),
        Item::Document(d) => d.meta.has_transparency,
        Item::FileOnly(_) => false,
    }
}

/// Does the item match all conditions of `file.filterSkip`? `Err` = reason why not.
pub fn filter_check(
    params: &Value,
    info: &ItemInfo,
    item: &Item,
) -> std::result::Result<(), String> {
    let (w, h) = match item.dimensions() {
        (0, 0) => (info.width, info.height),
        d => d,
    };
    let range = |lo: f64, hi: f64, v: f64| (lo <= 0.0 || v >= lo) && (hi <= 0.0 || v <= hi);
    if !range(
        p_f64(params, "minWidth", 0.0),
        p_f64(params, "maxWidth", 0.0),
        w as f64,
    ) || !range(
        p_f64(params, "minHeight", 0.0),
        p_f64(params, "maxHeight", 0.0),
        h as f64,
    ) {
        return Err(fp_i18n::tr!(
            "batch-skip-filter-pixels",
            w = w.to_string(),
            h = h.to_string()
        ));
    }
    let kb = info.bytes as f64 / 1024.0;
    if !range(p_f64(params, "minKB", 0.0), p_f64(params, "maxKB", 0.0), kb) {
        return Err(fp_i18n::tr!(
            "batch-skip-filter-bytes",
            kb = format!("{kb:.0}")
        ));
    }
    match p_str(params, "transparency", "any") {
        "with" if !item_has_transparency(item) => {
            return Err(fp_i18n::t("batch-skip-filter-opaque"));
        }
        "without" if item_has_transparency(item) => {
            return Err(fp_i18n::t("batch-skip-filter-transparent"));
        }
        _ => {}
    }
    let animated = item.kind() == ItemKind::Animation
        || (matches!(item, Item::FileOnly(_)) && info.kind == ItemKind::Animation);
    match p_str(params, "animation", "any") {
        "animated" if !animated => return Err(fp_i18n::t("batch-skip-filter-still")),
        "still" if animated => return Err(fp_i18n::t("batch-skip-filter-animated")),
        _ => {}
    }
    if let Ok(Some(re)) = glob_regex(p_str(params, "name", ""))
        && !re.is_match(&info.file_name())
    {
        return Err(fp_i18n::t("batch-skip-filter-name"));
    }
    Ok(())
}

struct FilterSkip;

impl Step for FilterSkip {
    fn id(&self) -> &str {
        FILTER_SKIP
    }
    fn group(&self) -> StepGroup {
        StepGroup::Files
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::int("minWidth", 0, 100_000, 0)
                .unit("px")
                .beginner(),
            ParamDef::int("maxWidth", 0, 100_000, 0).unit("px"),
            ParamDef::int("minHeight", 0, 100_000, 0)
                .unit("px")
                .beginner(),
            ParamDef::int("maxHeight", 0, 100_000, 0).unit("px"),
            ParamDef::number("minKB", 0.0, 10_000_000.0, 0.0).unit("KB"),
            ParamDef::number("maxKB", 0.0, 10_000_000.0, 0.0).unit("KB"),
            ParamDef::choice("transparency", &["any", "with", "without"], "any"),
            ParamDef::choice("animation", &["any", "animated", "still"], "any"),
            ParamDef::text("name", "").beginner(),
            ParamDef::bool("invert", false),
            ParamDef::choice("action", &["skip", "copy"], "skip").beginner(),
        ]
    }
    fn validate(&self, params: &Value) -> Vec<Issue> {
        let mut v = crate::step::validate_params(&self.params(), params);
        for (lo, hi) in [
            ("minWidth", "maxWidth"),
            ("minHeight", "maxHeight"),
            ("minKB", "maxKB"),
        ] {
            let (a, b) = (p_f64(params, lo, 0.0), p_f64(params, hi, 0.0));
            if a > 0.0 && b > 0.0 && a > b {
                v.push(Issue::param(hi, fp_i18n::t("batch-err-filter-range")));
            }
        }
        if let Err(e) = glob_regex(p_str(params, "name", "")) {
            v.push(Issue::param(
                "name",
                fp_i18n::tr!("batch-err-filter-name", error = e),
            ));
        }
        v
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let invert = p_bool(params, "invert", false);
        let reason = match (filter_check(params, ctx.info, item), invert) {
            (Ok(()), false) | (Err(_), true) => return Ok(()),
            (Err(r), false) => r,
            (Ok(()), true) => fp_i18n::t("batch-skip-filter-inverted"),
        };
        if ctx.preview {
            ctx.warn(fp_i18n::tr!("batch-warn-filter-preview", reason = reason));
            return Ok(());
        }
        Err(BatchError::filtered(
            reason,
            p_str(params, "action", "skip") == "copy",
        ))
    }
}

// ------------------------------------------------------------------ Duplicates

/// All pixel images of an item.
fn images(item: &Item) -> Vec<&RgbaImage> {
    match item {
        Item::Still(s) => vec![&s.image],
        Item::Animation(a) => a.frames.iter().map(|f| &f.image).collect(),
        _ => Vec::new(),
    }
}

fn exact_hash(item: &Item) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for img in images(item) {
        (img.width(), img.height()).hash(&mut h);
        for p in img.data() {
            p.hash(&mut h);
        }
    }
    h.finish()
}

fn same_pixels(a: &Item, b: &Item) -> bool {
    let (ia, ib) = (images(a), images(b));
    ia.len() == ib.len()
        && ia.iter().zip(ib.iter()).all(|(x, y)| {
            x.width() == y.width() && x.height() == y.height() && x.data() == y.data()
        })
}

/// Difference hash (64 bit): luminance over white on a 9 × 8 grid of area means, one bit per
/// "left cell brighter than right cell".
pub fn dhash(img: &RgbaImage) -> u64 {
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return 0;
    }
    let d = img.data();
    let mut grid = [[0f64; 9]; 8];
    for (gy, row) in grid.iter_mut().enumerate() {
        let (y0, y1) = (gy * h / 8, ((gy + 1) * h / 8).max(gy * h / 8 + 1).min(h));
        for (gx, cell) in row.iter_mut().enumerate() {
            let (x0, x1) = (gx * w / 9, ((gx + 1) * w / 9).max(gx * w / 9 + 1).min(w));
            let (mut s, mut n) = (0.0, 0.0);
            for y in y0..y1 {
                for x in x0..x1 {
                    let p = d[y * w + x];
                    let a = p[3] as f64 / 255.0;
                    let l = 0.299 * p[0] as f64 + 0.587 * p[1] as f64 + 0.114 * p[2] as f64;
                    s += l * a + 255.0 * (1.0 - a);
                    n += 1.0;
                }
            }
            *cell = if n > 0.0 { s / n } else { 255.0 };
        }
    }
    let mut bits = 0u64;
    for (y, row) in grid.iter().enumerate() {
        for x in 0..8 {
            if row[x] > row[x + 1] {
                bits |= 1 << (y * 8 + x);
            }
        }
    }
    bits
}

struct Dedupe;

impl Step for Dedupe {
    fn id(&self) -> &str {
        DEDUPE
    }
    fn group(&self) -> StepGroup {
        StepGroup::Files
    }
    fn kind(&self) -> StepKind {
        StepKind::Collect
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::choice("method", &["exact", "similar"], "exact").beginner(),
            ParamDef::int("distance", 0, 16, 4)
                .beginner()
                .when("method", &["similar"]),
            ParamDef::choice("action", &["skip", "mark"], "skip").beginner(),
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
        let similar = p_str(params, "method", "exact") == "similar";
        let max_dist = p_f64(params, "distance", 4.0).clamp(0.0, 16.0) as u32;
        let mark = p_str(params, "action", "skip") == "mark";
        let mut kept: Vec<(u64, usize)> = Vec::new(); // (fingerprint, position in `out`)
        let mut out: Vec<Item> = Vec::new();
        for (k, mut item) in items.into_iter().enumerate() {
            ctx.check_cancel()?;
            item.flatten_document()?;
            let fp = if similar {
                images(&item).first().map_or(0, |i| dhash(i))
            } else {
                exact_hash(&item)
            };
            let dup = kept.iter().find(|(f, pos)| {
                if similar {
                    (f ^ fp).count_ones() <= max_dist
                } else {
                    *f == fp && same_pixels(&out[*pos], &item)
                }
            });
            if let Some(&(_, pos)) = dup {
                let idx = item_index(&item, ctx.index + k);
                let msg = fp_i18n::tr!("batch-skip-duplicate", name = source_name(&out[pos]));
                if mark {
                    ctx.item_notes.push((idx, msg));
                } else {
                    ctx.item_skips.push((idx, msg));
                    continue;
                }
            }
            kept.push((fp, out.len()));
            out.push(item);
        }
        Ok(out)
    }
}

// ------------------------------------------------------------------ Set date

/// Parses `YYYY-MM-DD[ HH:MM[:SS]]` (also with `T`).
pub fn parse_date(s: &str) -> Option<NaiveDateTime> {
    let s = s.trim().replace('T', " ");
    for f in ["%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M"] {
        if let Ok(d) = NaiveDateTime::parse_from_str(&s, f) {
            return Some(d);
        }
    }
    chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
}

fn system_time(d: NaiveDateTime) -> Option<SystemTime> {
    chrono::Local
        .from_local_datetime(&d)
        .earliest()
        .map(SystemTime::from)
}

/// Modification time `file.timestamps` sets for an item (`None` = leave it).
pub fn timestamp_for(params: &Value, info: &ItemInfo) -> Option<SystemTime> {
    let source = || {
        info.modified.or_else(|| {
            std::fs::metadata(&info.path)
                .and_then(|m| m.modified())
                .ok()
        })
    };
    match p_str(params, "mode", "exif") {
        "fixed" => parse_date(p_str(params, "date", "")).and_then(system_time),
        "source" => source(),
        _ => info
            .exif
            .as_ref()
            .and_then(|e| e.date_taken)
            .and_then(system_time)
            .or_else(source),
    }
}

struct Timestamps;

impl Step for Timestamps {
    fn id(&self) -> &str {
        TIMESTAMPS
    }
    fn group(&self) -> StepGroup {
        StepGroup::Files
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::choice("mode", &["source", "exif", "fixed"], "exif").beginner(),
            ParamDef::text("date", "2024-01-01 12:00")
                .beginner()
                .when("mode", &["fixed"]),
        ]
    }
    fn validate(&self, params: &Value) -> Vec<Issue> {
        let mut v = crate::step::validate_params(&self.params(), params);
        if p_str(params, "mode", "exif") == "fixed"
            && parse_date(p_str(params, "date", "2024-01-01 12:00")).is_none()
        {
            v.push(Issue::param("date", fp_i18n::t("batch-err-timestamp-date")));
        }
        v
    }
    fn preview_ok(&self) -> bool {
        false
    }
    fn file_only(&self) -> bool {
        true
    }
    fn apply(&self, _item: &mut Item, _params: &Value, _ctx: &mut StepContext) -> Result<()> {
        // set by the executor after writing (`timestamp_for`)
        Ok(())
    }
}

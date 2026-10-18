//! Document steps (`docs/BATCH.md` §3.10) for FW-PNG/PSD/SVG items that are still documents:
//!
//! - `document.findReplace` – text, font, colour or URL in every page (like the Find panel:
//!   match case, whole word, regular expression); `nonWeb216` snaps colours to the web palette.
//! - `document.layers` – show, hide or show only the layers whose name matches (glob or regex).
//! - `document.pages` – collect step: every chosen page (and state) becomes its own image.
//! - `document.slices` – collect step: every slice of the document becomes its own image.
//!
//! The model is edited directly (no engine command does this headless); the document stays a
//! document (editable FW-PNG output) until a pixel step flattens it.

use crate::error::{BatchError, Result};
use crate::item::{Item, ItemKind, Meta, Still};
use crate::step::{Issue, ParamDef, Step, StepContext, StepGroup, StepKind, StepRegistry};
use crate::steps::anim::{EXTRA_OUTPUT_STEM, EXTRA_SOURCE_NAME, parse_frame_ranges};
use crate::steps::fileops::{glob_regex, item_index};
use fp_color::Argb;
use fp_model::{Document, Element, ElementKind, Layer, LayerSlot};
use fp_raster::{IRect, RgbaImage};
use serde_json::{Value, json};
use std::collections::HashSet;
use std::sync::Arc;

/// ID "Find and replace".
pub const FIND_REPLACE: &str = "document.findReplace";
/// ID "Show/hide layers".
pub const LAYERS: &str = "document.layers";
/// ID "Choose pages/states".
pub const PAGES: &str = "document.pages";
/// ID "Export slices".
pub const SLICES: &str = "document.slices";

/// Registers the steps of this file.
pub fn register(r: &mut StepRegistry) {
    r.register(Arc::new(FindReplace));
    r.register(Arc::new(Layers));
    r.register(Arc::new(Pages));
    r.register(Arc::new(Slices));
}

fn p_str<'a>(p: &'a Value, k: &str, d: &'a str) -> &'a str {
    p.get(k).and_then(Value::as_str).unwrap_or(d)
}
fn p_bool(p: &Value, k: &str, d: bool) -> bool {
    p.get(k).and_then(Value::as_bool).unwrap_or(d)
}

fn source_name(item: &Item, fallback: &str) -> String {
    item.meta()
        .and_then(|m| m.extra.get(EXTRA_SOURCE_NAME))
        .and_then(Value::as_str)
        .map_or_else(|| fallback.to_string(), str::to_string)
}

fn unique(used: &mut HashSet<String>, stem: String) -> String {
    if used.insert(stem.to_lowercase()) {
        return stem;
    }
    let mut k = 2;
    while !used.insert(format!("{stem} ({k})").to_lowercase()) {
        k += 1;
    }
    format!("{stem} ({k})")
}

fn sanitize(s: &str) -> String {
    let mut hints = Vec::new();
    crate::naming::sanitize_stem(s.trim(), &mut hints)
}

fn still(image: RgbaImage, meta: &Meta, stem: String) -> Item {
    let mut image = image;
    image.set_origin(0, 0);
    let mut m = meta.clone();
    m.has_transparency = image.data().iter().any(|p| p[3] < 255);
    m.extra.insert(EXTRA_OUTPUT_STEM.into(), json!(stem));
    Item::Still(Still { image, meta: m })
}

// ------------------------------------------------------------------ walking the model

/// Calls `f` for every element of every page (groups, masks, compound shapes and sub-layers
/// included), copy-on-write. Clears the render cache of every element `f` reports as changed.
fn edit_elements(doc: &mut Document, f: &mut dyn FnMut(&mut Element) -> bool) -> usize {
    let mut n = 0;
    for page in &mut doc.pages {
        let page = Arc::make_mut(page);
        for slot in &mut page.root.layers {
            if let LayerSlot::Own(l) = slot {
                n += edit_layer(Arc::make_mut(l), f);
            }
        }
    }
    n
}

fn edit_layer(l: &mut Layer, f: &mut dyn FnMut(&mut Element) -> bool) -> usize {
    let mut n = 0;
    for cel in &mut l.cels {
        for e in &mut Arc::make_mut(cel).elements {
            n += edit_element(e, f);
        }
    }
    n
}

fn edit_element(e: &mut Arc<Element>, f: &mut dyn FnMut(&mut Element) -> bool) -> usize {
    let el = Arc::make_mut(e);
    let mut n = 0;
    match &mut el.kind {
        ElementKind::Group(g) => {
            for c in &mut g.children {
                n += edit_element(c, f);
            }
            if let Some(m) = &mut g.mask {
                n += edit_element(m, f);
            }
        }
        ElementKind::CompoundShape(c) => {
            for o in &mut c.operands {
                n += edit_element(o, f);
            }
            if let Some(r) = &mut c.result {
                n += edit_element(r, f);
            }
        }
        ElementKind::SubLayer(l) => n += edit_layer(Arc::make_mut(l), f),
        _ => {}
    }
    if f(el) {
        n += 1;
    }
    if n > 0 {
        // nested or own change: render from the model again
        el.cache = None;
    }
    n
}

// ------------------------------------------------------------------ Find and replace

/// Matcher for text/URL search (`regex`, `matchCase`, `wholeWord`).
pub fn text_regex(
    find: &str,
    is_regex: bool,
    match_case: bool,
    whole_word: bool,
) -> std::result::Result<regex::Regex, String> {
    let core = if is_regex {
        find.to_string()
    } else {
        regex::escape(find)
    };
    let pat = if whole_word {
        format!(r"\b(?:{core})\b")
    } else {
        core
    };
    regex::RegexBuilder::new(&pat)
        .case_insensitive(!match_case)
        .build()
        .map_err(|e| e.to_string())
}

/// Nearest colour of the 216-colour web palette (each channel to a multiple of 51).
pub fn web216(rgb: [u8; 3]) -> [u8; 3] {
    rgb.map(|c| ((c as u32 + 25) / 51 * 51) as u8)
}

fn rgb_of(argb: u32) -> [u8; 3] {
    [(argb >> 16) as u8, (argb >> 8) as u8, argb as u8]
}
fn with_rgb(argb: u32, rgb: [u8; 3]) -> u32 {
    (argb & 0xff00_0000) | ((rgb[0] as u32) << 16) | ((rgb[1] as u32) << 8) | rgb[2] as u32
}

/// Colour replacement (`#rrggbb` → `#rrggbb`, alpha kept) or web-palette snapping.
enum ColorOp {
    Replace([u8; 3], [u8; 3]),
    Web216,
}

impl ColorOp {
    fn map(&self, argb: u32) -> Option<u32> {
        let rgb = rgb_of(argb);
        let new = match self {
            ColorOp::Replace(from, to) if rgb == *from => *to,
            ColorOp::Replace(..) => return None,
            ColorOp::Web216 => web216(rgb),
        };
        (new != rgb).then(|| with_rgb(argb, new))
    }
}

fn replace_colors(el: &mut Element, op: &ColorOp) -> bool {
    let mut changed = false;
    let mut attrs = |a: &mut Arc<fp_model::attrs::PathAttrs>| {
        let (f, s) = (op.map(a.fill_color.0), op.map(a.stroke_color.0));
        if f.is_some() || s.is_some() {
            let a = Arc::make_mut(a);
            if let Some(f) = f {
                a.fill_color = Argb(f);
            }
            if let Some(s) = s {
                a.stroke_color = Argb(s);
            }
            changed = true;
        }
    };
    match &mut el.kind {
        ElementKind::Path(p) => attrs(&mut p.attrs),
        ElementKind::Text(t) => {
            attrs(&mut t.attrs);
            let text_attrs = |c: &mut Option<u32>| {
                if let Some(v) = c.and_then(|v| op.map(v)) {
                    *c = Some(v);
                    true
                } else {
                    false
                }
            };
            let mut hit = text_attrs(&mut t.initial.color);
            for r in &mut t.runs {
                hit |= text_attrs(&mut r.attrs.color);
            }
            if hit {
                t.relaid = true;
                changed = true;
            }
        }
        _ => {}
    }
    changed
}

/// Find and replace in a document. Returns the number of changed elements.
pub fn find_replace(doc: &mut Document, params: &Value) -> std::result::Result<usize, String> {
    let find = p_str(params, "find", "");
    let replace = p_str(params, "replace", "");
    let what = p_str(params, "whatToFind", "text");
    let re = || {
        text_regex(
            find,
            p_bool(params, "regex", false),
            p_bool(params, "matchCase", false),
            p_bool(params, "wholeWord", false),
        )
    };
    let n = match what {
        "font" => {
            let font = find.trim().to_lowercase();
            edit_elements(doc, &mut |el| {
                let ElementKind::Text(t) = &mut el.kind else {
                    return false;
                };
                let mut hit = false;
                let mut set = |a: &mut fp_model::text::TextAttrs| {
                    if a.font
                        .as_deref()
                        .is_some_and(|f| f.trim().to_lowercase() == font)
                    {
                        a.font = Some(replace.to_string());
                        a.ps_font = Some(replace.to_string());
                        hit = true;
                    }
                };
                set(&mut t.initial);
                for r in &mut t.runs {
                    set(&mut r.attrs);
                }
                if hit {
                    t.relaid = true;
                }
                hit
            })
        }
        "color" => {
            let parse = |s: &str| crate::step::parse_color(s).map(|c| [c[0], c[1], c[2]]);
            let (Some(a), Some(b)) = (parse(find), parse(replace)) else {
                return Err(fp_i18n::t("batch-err-find-color"));
            };
            let op = ColorOp::Replace(a, b);
            edit_elements(doc, &mut |el| replace_colors(el, &op))
        }
        "nonWeb216" => edit_elements(doc, &mut |el| replace_colors(el, &ColorOp::Web216)),
        "url" => {
            let re = re()?;
            edit_elements(doc, &mut |el| {
                let ElementKind::WebObject(w) = &mut el.kind else {
                    return false;
                };
                match &w.url {
                    Some(u) if re.is_match(u) => {
                        w.url = Some(re.replace_all(u, replace).into_owned());
                        true
                    }
                    _ => false,
                }
            })
        }
        _ => {
            let re = re()?;
            edit_elements(doc, &mut |el| {
                let ElementKind::Text(t) = &mut el.kind else {
                    return false;
                };
                let mut hit = false;
                for r in &mut t.runs {
                    if re.is_match(&r.text) {
                        r.text = re.replace_all(&r.text, replace).into_owned();
                        hit = true;
                    }
                }
                if hit {
                    t.relaid = true;
                }
                hit
            })
        }
    };
    Ok(n)
}

struct FindReplace;

impl Step for FindReplace {
    fn id(&self) -> &str {
        FIND_REPLACE
    }
    fn group(&self) -> StepGroup {
        StepGroup::Document
    }
    fn params(&self) -> Vec<ParamDef> {
        let not_web = ["text", "font", "color", "url"];
        vec![
            ParamDef::choice(
                "whatToFind",
                &["text", "font", "color", "url", "nonWeb216"],
                "text",
            )
            .beginner(),
            ParamDef::text("find", "")
                .beginner()
                .when("whatToFind", &not_web),
            ParamDef::text("replace", "")
                .beginner()
                .when("whatToFind", &not_web),
            ParamDef::bool("matchCase", false).when("whatToFind", &["text", "url"]),
            ParamDef::bool("wholeWord", false).when("whatToFind", &["text", "url"]),
            ParamDef::bool("regex", false).when("whatToFind", &["text", "url"]),
        ]
    }
    fn validate(&self, params: &Value) -> Vec<Issue> {
        let mut v = crate::step::validate_params(&self.params(), params);
        let find = p_str(params, "find", "");
        let replace = p_str(params, "replace", "");
        match p_str(params, "whatToFind", "text") {
            "nonWeb216" => {}
            "color" => {
                for (k, s) in [("find", find), ("replace", replace)] {
                    if crate::step::parse_color(s).is_none() {
                        v.push(Issue::param(k, fp_i18n::t("batch-err-find-color")));
                    }
                }
            }
            what => {
                if find.is_empty() {
                    v.push(Issue::param("find", fp_i18n::t("batch-err-find-empty")));
                } else if matches!(what, "text" | "url")
                    && let Err(e) = text_regex(
                        find,
                        p_bool(params, "regex", false),
                        true,
                        p_bool(params, "wholeWord", false),
                    )
                {
                    v.push(Issue::param(
                        "find",
                        fp_i18n::tr!("batch-err-find-regex", error = e),
                    ));
                }
            }
        }
        v
    }
    fn applies_to(&self) -> &[ItemKind] {
        &[ItemKind::Document]
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let Item::Document(d) = item else {
            return Ok(());
        };
        let n = find_replace(Arc::make_mut(&mut d.doc), params).map_err(BatchError::Invalid)?;
        if n == 0 {
            ctx.warn(fp_i18n::t("batch-warn-find-none"));
        }
        Ok(())
    }
}

// ------------------------------------------------------------------ Show/hide layers

/// Sets the visibility of the matching top-level layers of every page (`mode` show/hide/only).
/// Returns the number of matching layers.
pub fn set_layers(doc: &mut Document, is_match: &dyn Fn(&str) -> bool, mode: &str) -> usize {
    let mut hits = 0;
    for page in &mut doc.pages {
        let page = Arc::make_mut(page);
        for slot in &mut page.root.layers {
            let LayerSlot::Own(l) = slot else {
                continue;
            };
            let hit = is_match(&l.name);
            hits += hit as usize;
            let vis = match mode {
                "show" => hit.then_some(true),
                "hide" => hit.then_some(false),
                _ => Some(hit),
            };
            let Some(v) = vis else {
                continue;
            };
            let l = Arc::make_mut(l);
            if l.visibility.is_empty() {
                l.visibility.push(v);
            } else {
                l.visibility.iter_mut().for_each(|x| *x = v);
            }
            l.has_visibility_node = true;
        }
    }
    hits
}

fn name_matcher(params: &Value) -> std::result::Result<regex::Regex, String> {
    let name = p_str(params, "name", "");
    if p_bool(params, "regex", false) {
        regex::RegexBuilder::new(name)
            .case_insensitive(true)
            .build()
            .map_err(|e| e.to_string())
    } else {
        glob_regex(name).map(|r| r.unwrap_or_else(|| regex::Regex::new("^$").expect("empty")))
    }
}

struct Layers;

impl Step for Layers {
    fn id(&self) -> &str {
        LAYERS
    }
    fn group(&self) -> StepGroup {
        StepGroup::Document
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::text("name", "").beginner(),
            ParamDef::bool("regex", false),
            ParamDef::choice("mode", &["only", "show", "hide"], "only").beginner(),
        ]
    }
    fn validate(&self, params: &Value) -> Vec<Issue> {
        let mut v = crate::step::validate_params(&self.params(), params);
        if p_str(params, "name", "").trim().is_empty() {
            v.push(Issue::param("name", fp_i18n::t("batch-err-layers-name")));
        } else if let Err(e) = name_matcher(params) {
            v.push(Issue::param(
                "name",
                fp_i18n::tr!("batch-err-find-regex", error = e),
            ));
        }
        v
    }
    fn applies_to(&self) -> &[ItemKind] {
        &[ItemKind::Document]
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let Item::Document(d) = item else {
            return Ok(());
        };
        let re = name_matcher(params).map_err(BatchError::Invalid)?;
        let hits = set_layers(
            Arc::make_mut(&mut d.doc),
            &|n| re.is_match(n),
            p_str(params, "mode", "only"),
        );
        if hits == 0 {
            ctx.warn(fp_i18n::t("batch-warn-layers-none"));
        }
        Ok(())
    }
}

// ------------------------------------------------------------------ Pages / states

/// Name of a page result: `{name}`, `{page}` (1-based), `{pageName}`, `{state}` (1-based).
pub fn page_name(pattern: &str, name: &str, page: usize, page_name: &str, state: usize) -> String {
    let s = pattern
        .replace("{name}", name)
        .replace("{pageName}", page_name)
        .replace("{page}", &page.to_string())
        .replace("{state}", &state.to_string());
    sanitize(&s)
}

fn render(doc: &Document, page: usize, frame: usize) -> Option<RgbaImage> {
    let img = fp_render::Renderer::default()
        .render_page(doc, page, frame)
        .image;
    (!img.is_empty()).then_some(img)
}

struct Pages;

impl Step for Pages {
    fn id(&self) -> &str {
        PAGES
    }
    fn group(&self) -> StepGroup {
        StepGroup::Document
    }
    fn kind(&self) -> StepKind {
        StepKind::Collect
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::text("pages", "").beginner(),
            ParamDef::choice("states", &["first", "all"], "first").beginner(),
            ParamDef::text("pattern", "{name}_{page}"),
        ]
    }
    fn validate(&self, params: &Value) -> Vec<Issue> {
        let mut v = crate::step::validate_params(&self.params(), params);
        let pages = p_str(params, "pages", "");
        if !pages.trim().is_empty()
            && let Err(e) = parse_frame_ranges(pages, 1000)
        {
            v.push(Issue::param("pages", e));
        }
        v
    }
    fn applies_to(&self) -> &[ItemKind] {
        &[ItemKind::Document]
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
        let pattern = match p_str(params, "pattern", "{name}_{page}").trim() {
            "" => "{name}_{page}",
            p => p,
        };
        let all_states = p_str(params, "states", "first") == "all";
        let fallback = crate::naming::split_extension(&ctx.info.file_name()).0;
        let mut used = HashSet::new();
        let mut out = Vec::new();
        for (k, item) in items.into_iter().enumerate() {
            ctx.check_cancel()?;
            let name = source_name(&item, &fallback);
            let Item::Document(d) = &item else {
                ctx.item_skips.push((
                    item_index(&item, ctx.index + k),
                    fp_i18n::t("batch-skip-not-document"),
                ));
                continue;
            };
            let n = d.doc.pages.len();
            let mask = match p_str(params, "pages", "").trim() {
                "" => vec![true; n],
                s => parse_frame_ranges(s, n).map_err(BatchError::Invalid)?,
            };
            for (pi, page) in d.doc.pages.iter().enumerate() {
                if !mask.get(pi).copied().unwrap_or(false) {
                    continue;
                }
                let states = if all_states {
                    page.root.frame_count.max(1) as usize
                } else {
                    1
                };
                for fi in 0..states {
                    ctx.check_cancel()?;
                    let Some(img) = render(&d.doc, pi, fi) else {
                        continue;
                    };
                    let mut stem = page_name(
                        pattern,
                        &name,
                        pi + 1,
                        page.name.as_deref().unwrap_or(""),
                        fi + 1,
                    );
                    if all_states && !pattern.contains("{state}") {
                        stem = format!("{stem}_{}", fi + 1);
                    }
                    out.push(still(img, &d.meta, unique(&mut used, stem)));
                }
            }
        }
        if out.is_empty() {
            return Err(BatchError::Invalid(fp_i18n::t("batch-err-pages-none")));
        }
        Ok(out)
    }
}

// ------------------------------------------------------------------ Slices

/// Slices of a document: (page, name, rectangle in document coordinates).
pub fn slices_of(doc: &Document) -> Vec<(usize, String, IRect)> {
    let mut out = Vec::new();
    doc.walk_elements(|ctx, _path, e| {
        let fp_model::traverse::RootRef::Page(page) = ctx.root else {
            return;
        };
        let ElementKind::WebObject(w) = &e.kind else {
            return;
        };
        if w.slice.is_none() {
            return;
        }
        let b = w.bounds;
        let r = IRect::new(
            b.left.round() as i32,
            b.top.round() as i32,
            b.right.round() as i32,
            b.bottom.round() as i32,
        );
        if r.is_empty() {
            return;
        }
        let name = w
            .file_name
            .clone()
            .or_else(|| e.common.name.clone())
            .unwrap_or_default();
        out.push((page, name, r));
    });
    out.sort_by_key(|(p, _, r)| (*p, r.top, r.left));
    out
}

struct Slices;

impl Step for Slices {
    fn id(&self) -> &str {
        SLICES
    }
    fn group(&self) -> StepGroup {
        StepGroup::Document
    }
    fn kind(&self) -> StepKind {
        StepKind::Collect
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::text("pattern", "{name}_{slice}").beginner()]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &[ItemKind::Document]
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
        let pattern = match p_str(params, "pattern", "{name}_{slice}").trim() {
            "" => "{name}_{slice}",
            p => p,
        };
        let fallback = crate::naming::split_extension(&ctx.info.file_name()).0;
        let mut used = HashSet::new();
        let mut out = Vec::new();
        for (k, item) in items.into_iter().enumerate() {
            ctx.check_cancel()?;
            let name = source_name(&item, &fallback);
            let idx = item_index(&item, ctx.index + k);
            let Item::Document(d) = &item else {
                ctx.item_skips
                    .push((idx, fp_i18n::t("batch-skip-not-document")));
                continue;
            };
            let slices = slices_of(&d.doc);
            if slices.is_empty() {
                ctx.item_skips
                    .push((idx, fp_i18n::t("batch-skip-no-slices")));
                continue;
            }
            let mut page_img: Option<(usize, RgbaImage)> = None;
            for (n, (page, slice_name, rect)) in slices.into_iter().enumerate() {
                ctx.check_cancel()?;
                if page_img.as_ref().is_none_or(|(p, _)| *p != page) {
                    page_img = render(&d.doc, page, 0).map(|i| (page, i));
                }
                let Some((_, img)) = &page_img else {
                    continue;
                };
                let part = img.cropped(rect);
                if part.is_empty() {
                    continue;
                }
                let slice = if slice_name.trim().is_empty() {
                    (n + 1).to_string()
                } else {
                    slice_name
                };
                let stem = sanitize(
                    &pattern
                        .replace("{name}", &name)
                        .replace("{slice}", &slice)
                        .replace("{n}", &(n + 1).to_string()),
                );
                out.push(still(part, &d.meta, unique(&mut used, stem)));
            }
        }
        if out.is_empty() {
            return Err(BatchError::Invalid(fp_i18n::t("batch-skip-no-slices")));
        }
        Ok(out)
    }
}

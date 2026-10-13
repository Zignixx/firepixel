//! Gruppe „Überlagerung“ (SPEC-BATCH §3.6, Paket BATCH-6):
//!
//! - `overlay.text` – Text/Wasserzeichen mit Platzhaltern (`{name}`, `{date}`, `{exif-date}`,
//!   `{n}`, `{year}` …), 9 Lagen / gekachelt / frei, Größe in % der Bildbreite, Schrift, Farbe,
//!   Deckkraft, Abstand, Drehung, Kachelabstand, Schatten, Kontur, Mischmodus.
//! - `overlay.image` – Logo/Bild (Datei oder im Rezept eingebettet), Lage, Größe in % der
//!   Bildbreite, Deckkraft, Abstand, Drehung, Kachelung, Mischmodus.
//! - `overlay.border` – Rahmen (px oder % der kürzeren Seite), Farbe, abgerundete Ecken (außen
//!   transparent), innen/außen.
//!
//! Der Stempel (Text bzw. Logo) wird einmal je Bild gebaut und bei Animationen auf jedes Frame
//! gesetzt; alle Größen sind relativ zum Bild, die Vorschau sieht daher wie das Ergebnis aus.

use super::background::{blend_onto, gauss, p_bool, p_color, p_f64, p_str, param_image, preview_scale, resize, rotate, to_f, to_u8, BLEND_MODES};
use crate::error::{BatchError, Result};
use crate::item::{Item, ItemKind};
use crate::step::{Issue, ParamDef, Step, StepContext, StepGroup, StepRegistry};
use fp_raster::{IRect, RgbaImage};
use rayon::prelude::*;
use serde_json::Value;
use std::sync::Arc;

/// ID „Text/Wasserzeichen“.
pub const TEXT: &str = "overlay.text";
/// ID „Logo/Bild einfügen“.
pub const IMAGE: &str = "overlay.image";
/// ID „Rahmen“.
pub const BORDER: &str = "overlay.border";

/// Lagen eines Stempels.
pub const POSITIONS: [&str; 11] = ["topLeft", "top", "topRight", "left", "center", "right", "bottomLeft", "bottom", "bottomRight", "tiled", "free"];

/// Registriert die Schritte der Gruppe.
pub fn register(r: &mut StepRegistry) {
    r.register(Arc::new(TextStep));
    r.register(Arc::new(ImageStep));
    r.register(Arc::new(Border));
}

// ------------------------------------------------------------------ Platzhalter

/// Ersetzt Platzhalter im Text: `{name}` (Dateiname ohne Endung), `{ext}`, `{folder}`, `{n}` bzw.
/// `{n:3}` (laufende Nummer ab 1, mit Stellen), `{total}`, `{date}` (heute, JJJJ-MM-TT), `{time}`,
/// `{year}`, `{exif-date}` (Aufnahmedatum, Rückfall Änderungsdatum der Datei), `{w}`, `{h}`
/// (aktuelle Bildgröße); `(c)` → `©`. Unbekannte Platzhalter bleiben stehen.
pub fn expand_placeholders(text: &str, ctx: &StepContext, w: usize, h: usize) -> String {
    let now = chrono::Local::now();
    let path = &ctx.info.path;
    let file = path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let (stem, ext) = match file.to_ascii_lowercase().strip_suffix(".fw.png") {
        Some(_) => (file[..file.len() - 7].to_string(), "fw.png".to_string()),
        None => (
            path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default(),
            path.extension().map(|s| s.to_string_lossy().to_string()).unwrap_or_default(),
        ),
    };
    let folder = path.parent().and_then(|p| p.file_name()).map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let exif_date = ctx
        .info
        .exif
        .as_ref()
        .and_then(|e| e.date_taken)
        .map(|d| d.format("%Y-%m-%d").to_string())
        .or_else(|| ctx.info.modified.map(|m| chrono::DateTime::<chrono::Local>::from(m).format("%Y-%m-%d").to_string()))
        .unwrap_or_else(|| now.format("%Y-%m-%d").to_string());
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('}') else {
            out.push_str(&rest[start..]);
            rest = "";
            break;
        };
        let tag = &after[..end];
        let (name, arg) = tag.split_once(':').map(|(a, b)| (a, Some(b))).unwrap_or((tag, None));
        let rep = match name {
            "name" => Some(stem.clone()),
            "ext" => Some(ext.clone()),
            "folder" => Some(folder.clone()),
            "n" => {
                let digits = arg.and_then(|a| a.parse::<usize>().ok()).unwrap_or(1).min(12);
                Some(format!("{:0digits$}", ctx.index + 1))
            }
            "total" => Some(ctx.total.to_string()),
            "date" => Some(now.format("%Y-%m-%d").to_string()),
            "time" => Some(now.format("%H:%M").to_string()),
            "year" => Some(now.format("%Y").to_string()),
            "exif-date" => Some(exif_date.clone()),
            "w" => Some(w.to_string()),
            "h" => Some(h.to_string()),
            _ => None,
        };
        match rep {
            Some(r) => out.push_str(&r),
            None => {
                out.push('{');
                out.push_str(tag);
                out.push('}');
            }
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out.replace("(c)", "©").replace("(C)", "©")
}

// ------------------------------------------------------------------ Stempel setzen

/// Linke obere Ecken aller Stempel in einem `w × h`-Bild.
pub fn stamp_positions(w: usize, h: usize, sw: usize, sh: usize, p: &Value) -> Vec<(i32, i32)> {
    let (w, h, sw, sh) = (w as f64, h as f64, sw as f64, sh as f64);
    let m = p_f64(p, "margin", 3.0).clamp(0.0, 50.0) / 100.0 * w.min(h);
    let pos = p_str(p, "position", "bottomRight");
    let x_of = |col: u8| match col {
        0 => m,
        1 => (w - sw) / 2.0,
        _ => w - sw - m,
    };
    let y_of = |row: u8| match row {
        0 => m,
        1 => (h - sh) / 2.0,
        _ => h - sh - m,
    };
    let one = |x: f64, y: f64| vec![(x.round() as i32, y.round() as i32)];
    match pos {
        "topLeft" => one(x_of(0), y_of(0)),
        "top" => one(x_of(1), y_of(0)),
        "topRight" => one(x_of(2), y_of(0)),
        "left" => one(x_of(0), y_of(1)),
        "center" => one(x_of(1), y_of(1)),
        "right" => one(x_of(2), y_of(1)),
        "bottomLeft" => one(x_of(0), y_of(2)),
        "bottom" => one(x_of(1), y_of(2)),
        "free" => one(w * p_f64(p, "x", 50.0).clamp(0.0, 100.0) / 100.0 - sw / 2.0, h * p_f64(p, "y", 50.0).clamp(0.0, 100.0) / 100.0 - sh / 2.0),
        "tiled" => {
            if sw < 1.0 || sh < 1.0 {
                return Vec::new();
            }
            // Abstand in % der kleineren Stempelseite; versetzte Reihen (Ziegelmuster), Mitte des
            // Musters = Bildmitte
            let gap = p_f64(p, "spacing", 100.0).clamp(0.0, 1000.0) / 100.0 * sw.min(sh);
            let (stx, sty) = (sw + gap, sh + gap);
            let rows = (h / sty).ceil() as i64 + 2;
            let cols = (w / stx).ceil() as i64 + 2;
            let (cx, cy) = ((w - sw) / 2.0, (h - sh) / 2.0);
            let mut v = Vec::new();
            for r in -rows..=rows {
                let y = cy + r as f64 * sty;
                if y + sh <= 0.0 || y >= h {
                    continue;
                }
                let shift = if r.rem_euclid(2) == 1 { stx / 2.0 } else { 0.0 };
                for c in -cols..=cols {
                    let x = cx + c as f64 * stx + shift;
                    if x + sw <= 0.0 || x >= w {
                        continue;
                    }
                    v.push((x.round() as i32, y.round() as i32));
                }
            }
            v
        }
        _ => one(x_of(2), y_of(2)),
    }
}

fn place_stamp(img: &mut RgbaImage, stamp: &RgbaImage, p: &Value) {
    let opacity = (p_f64(p, "opacity", 100.0) / 100.0).clamp(0.0, 1.0) as f32;
    let mode = p_str(p, "blend", "normal").to_string();
    for (x, y) in stamp_positions(img.width(), img.height(), stamp.width(), stamp.height(), p) {
        blend_onto(img, stamp, x, y, opacity, &mode);
    }
}

// ------------------------------------------------------------------ Text

fn mask_to_plane(m: &fp_raster::Mask, b: IRect) -> Vec<f32> {
    let (w, h) = (b.width() as usize, b.height() as usize);
    let mut v = vec![0.0f32; w * h];
    for y in 0..h {
        for x in 0..w {
            v[y * w + x] = m.get(b.left + x as i32, b.top + y as i32) as f32 / 255.0;
        }
    }
    v
}

/// Baut den Textstempel (Schatten, Kontur, Füllung) in Pixelgröße `size_px`. `None` = keine
/// Schrift bzw. nichts sichtbar.
pub fn text_stamp(text: &str, size_px: f64, p: &Value) -> Option<RgbaImage> {
    let text = text.replace("\r\n", "\n").replace('\n', "\r");
    if text.trim().is_empty() {
        return None;
    }
    let align = match p_str(p, "position", "bottomRight") {
        "topLeft" | "left" | "bottomLeft" => fp_text::Alignment::Left,
        "topRight" | "right" | "bottomRight" => fp_text::Alignment::Right,
        _ => fp_text::Alignment::Center,
    };
    let color = p_color(p, "color", [255, 255, 255, 255]);
    let attrs = fp_text::CharAttrs {
        font: p_str(p, "font", "Arial").to_string(),
        size: size_px.max(2.0),
        color: fp_color::Rgba8::new(color[0], color[1], color[2], 255),
        bold: p_bool(p, "bold", false),
        italic: p_bool(p, "italic", false),
        alignment: align,
        ..fp_text::CharAttrs::default()
    };
    let block = fp_text::TextBlock::point(0.0, 0.0, vec![fp_text::TextRun::new(text, attrs)]);
    let lay = fp_text::layout(&block, &fp_text::FontDb::shared(), &fp_text::LayoutOptions::default());
    let ink = fp_text::render::ink_bounds(&lay);
    if ink.is_empty() {
        return None;
    }
    let outline = p_bool(p, "outline", false);
    let ow = if outline { (p_f64(p, "outlineWidth", 6.0).clamp(0.0, 50.0) / 100.0 * size_px).max(1.0).round() as i32 } else { 0 };
    let shadow = p_bool(p, "shadow", false);
    let sd = if shadow { (size_px * 0.05).max(1.0).round() as i32 } else { 0 };
    let sb = if shadow { (size_px * 0.06).max(1.0) as f32 } else { 0.0 };
    let pad = ow + sd + (sb * 1.5).ceil() as i32 + 2;
    let b = ink.inflate(pad, pad);
    let cov = fp_text::render::coverage(&lay, Some(b));
    if cov.width() == 0 || cov.height() == 0 {
        return None;
    }
    let (w, h) = (b.width() as usize, b.height() as usize);
    let fill = mask_to_plane(&cov, b);
    let outer = if ow > 0 { fp_ai::tuning::morph_soft(&cov, ow).ok().map(|m| mask_to_plane(&m, b)).unwrap_or_else(|| fill.clone()) } else { fill.clone() };
    let mut sh_plane = Vec::new();
    if shadow {
        sh_plane = vec![0.0f32; w * h];
        for y in 0..h {
            for x in 0..w {
                let (sx, sy) = (x as i64 - sd as i64, y as i64 - sd as i64);
                if sx >= 0 && sy >= 0 {
                    sh_plane[y * w + x] = outer[sy as usize * w + sx as usize];
                }
            }
        }
        gauss(&mut sh_plane, w, h, 1, sb, false);
    }
    let fc = to_f(color);
    let oc = to_f(p_color(p, "outlineColor", [0, 0, 0, 255]));
    let mut data = vec![[0u8; 4]; w * h];
    data.par_iter_mut().enumerate().for_each(|(i, o)| {
        let mut acc = [0.0f32; 4];
        if shadow {
            acc = [0.0, 0.0, 0.0, sh_plane[i] * 0.6];
        }
        if ow > 0 {
            acc = super::background::over([oc[0], oc[1], oc[2], oc[3] * outer[i]], acc, false);
        }
        acc = super::background::over([fc[0], fc[1], fc[2], fc[3] * fill[i]], acc, false);
        *o = to_u8(acc);
    });
    let img = RgbaImage::from_raw(IRect::new(0, 0, w as i32, h as i32), data).ok()?;
    // auf die sichtbaren Pixel zuschneiden (Lage am Rand stimmt dann mit dem Abstand überein)
    let tb = img.alpha_bounds();
    if tb.is_empty() {
        return None;
    }
    let mut c = img.cropped(tb);
    c.set_origin(0, 0);
    Some(c)
}

fn position_params() -> Vec<ParamDef> {
    vec![
        ParamDef::choice("position", &POSITIONS, "bottomRight").beginner(),
        ParamDef::number("x", 0.0, 100.0, 50.0).unit("%").when("position", &["free"]),
        ParamDef::number("y", 0.0, 100.0, 50.0).unit("%").when("position", &["free"]),
        ParamDef::number("margin", 0.0, 50.0, 3.0).unit("%").when("position", &POSITIONS[..9]),
        ParamDef::number("rotation", -360.0, 360.0, 0.0).unit("°"),
        ParamDef::number("spacing", 0.0, 1000.0, 100.0).unit("%").when("position", &["tiled"]),
        ParamDef::choice("blend", &BLEND_MODES, "normal"),
    ]
}

/// `overlay.text`.
struct TextStep;

impl Step for TextStep {
    fn id(&self) -> &str {
        TEXT
    }
    fn group(&self) -> StepGroup {
        StepGroup::Overlay
    }
    fn params(&self) -> Vec<ParamDef> {
        let mut v = vec![
            ParamDef::text("text", "© {year}").beginner(),
            ParamDef::number("size", 1.0, 50.0, 4.0).unit("%").beginner(),
            ParamDef::text("font", "Arial"),
            ParamDef::bool("bold", false),
            ParamDef::bool("italic", false),
            ParamDef::color("color", "#ffffff").beginner(),
            ParamDef::int("opacity", 0, 100, 60).unit("%").beginner(),
            ParamDef::bool("shadow", false),
            ParamDef::bool("outline", false),
            ParamDef::color("outlineColor", "#000000").when("outline", &["true"]),
            ParamDef::number("outlineWidth", 0.0, 50.0, 6.0).unit("%").when("outline", &["true"]),
        ];
        v.extend(position_params());
        v
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        item.flatten_document()?;
        let (w, h) = item.dimensions();
        if w == 0 || h == 0 {
            return Ok(());
        }
        let text = expand_placeholders(p_str(params, "text", ""), ctx, w as usize, h as usize);
        let size = p_f64(params, "size", 4.0).clamp(1.0, 50.0) / 100.0 * w as f64;
        let Some(stamp) = text_stamp(&text, size, params) else {
            if !text.trim().is_empty() {
                ctx.warn(fp_i18n::t("batch-overlay-warn-no-font"));
            }
            return Ok(());
        };
        let stamp = rotate(&stamp, p_f64(params, "rotation", 0.0));
        ctx.check_cancel()?;
        let cancel = ctx.cancel;
        item.map_images(|img| {
            cancel.check()?;
            place_stamp(img, &stamp, params);
            Ok(())
        })
    }
    fn summary(&self, params: &Value) -> String {
        p_str(params, "text", "").chars().take(40).collect()
    }
}

/// `overlay.image`.
struct ImageStep;

impl Step for ImageStep {
    fn id(&self) -> &str {
        IMAGE
    }
    fn group(&self) -> StepGroup {
        StepGroup::Overlay
    }
    fn params(&self) -> Vec<ParamDef> {
        let mut v = vec![
            ParamDef::path("image").beginner(),
            ParamDef::text("imageData", ""),
            ParamDef::number("scale", 1.0, 100.0, 15.0).unit("%").beginner(),
            ParamDef::int("opacity", 0, 100, 100).unit("%"),
        ];
        v.extend(position_params());
        v
    }
    fn validate(&self, params: &Value) -> Vec<Issue> {
        let mut v = crate::step::validate_params(&self.params(), params);
        if p_str(params, "image", "").trim().is_empty() && p_str(params, "imageData", "").trim().is_empty() {
            let mut i = Issue::param("image", fp_i18n::t("batch-bg-err-no-image"));
            i.code = "missingImage".into();
            v.push(i);
        }
        v
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let logo = param_image(params, "image", "imageData")?.ok_or_else(|| BatchError::Invalid(fp_i18n::t("batch-bg-err-no-image")))?;
        item.flatten_document()?;
        let (w, h) = item.dimensions();
        if w == 0 || h == 0 || logo.is_empty() {
            return Ok(());
        }
        let tw = (p_f64(params, "scale", 15.0).clamp(1.0, 100.0) / 100.0 * w as f64).round().max(1.0);
        let th = (tw * logo.height() as f64 / logo.width() as f64).round().max(1.0);
        let stamp = rotate(&resize(&logo, tw as u32, th as u32), p_f64(params, "rotation", 0.0));
        ctx.check_cancel()?;
        let cancel = ctx.cancel;
        item.map_images(|img| {
            cancel.check()?;
            place_stamp(img, &stamp, params);
            Ok(())
        })
    }
}

// ------------------------------------------------------------------ Rahmen

/// Deckung (0…1) eines Pixels (Mitte x+0.5, y+0.5) im abgerundeten Rechteck `[x0, x1) × [y0, y1)`
/// mit Eckenradius `r` (Kantenglättung über den Abstand).
fn round_rect_cov(x: f64, y: f64, x0: f64, y0: f64, x1: f64, y1: f64, r: f64) -> f32 {
    let (px, py) = (x + 0.5, y + 0.5);
    if x1 <= x0 || y1 <= y0 {
        return 0.0;
    }
    let r = r.min((x1 - x0) / 2.0).min((y1 - y0) / 2.0).max(0.0);
    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let (hx, hy) = ((x1 - x0) / 2.0 - r, (y1 - y0) / 2.0 - r);
    let (qx, qy) = ((px - cx).abs() - hx, (py - cy).abs() - hy);
    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    let inside = qx.max(qy).min(0.0);
    let d = outside + inside - r;
    (0.5 - d).clamp(0.0, 1.0) as f32
}

/// Rahmen nach den Parametern von `overlay.border` (`scale` = Vorschau-Maßstab für px-Angaben).
pub fn border_image(img: &RgbaImage, p: &Value, scale: f64) -> RgbaImage {
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return img.clone();
    }
    let short = w.min(h) as f64;
    let width = p_f64(p, "width", 20.0).max(0.0);
    let bw = if p_str(p, "unit", "px") == "percent" { width.min(50.0) / 100.0 * short } else { width * scale }.round();
    let outer = p_str(p, "placement", "outer") != "inner";
    let color = to_f(p_color(p, "color", [255, 255, 255, 255]));
    let (nw, nh, off) = if outer { (w + 2 * bw as usize, h + 2 * bw as usize, bw) } else { (w, h, 0.0) };
    let radius = p_f64(p, "radius", 0.0).clamp(0.0, 50.0) / 100.0 * (nw.min(nh) as f64);
    let inner_r = (radius - bw).max(0.0);
    let (iw, ih) = (w as f64, h as f64);
    let src = img.data();
    let mut out = vec![[0u8; 4]; nw * nh];
    out.par_chunks_mut(nw).enumerate().for_each(|(y, row)| {
        for (x, o) in row.iter_mut().enumerate() {
            let (fx, fy) = (x as f64, y as f64);
            let shape = round_rect_cov(fx, fy, 0.0, 0.0, nw as f64, nh as f64, radius);
            if shape <= 0.0 {
                continue;
            }
            // Fenster für das Bild: innerhalb des Rahmens
            let win = round_rect_cov(fx, fy, bw, bw, nw as f64 - bw, nh as f64 - bw, inner_r);
            let (ix, iy) = (fx - off, fy - off);
            let pic = if ix >= 0.0 && iy >= 0.0 && ix < iw && iy < ih { to_f(src[iy as usize * w + ix as usize]) } else { [0.0; 4] };
            let mut pv = pic;
            pv[3] *= win;
            let band = [color[0], color[1], color[2], color[3] * (1.0 - win)];
            let mut c = super::background::over(pv, band, false);
            if bw <= 0.0 {
                c = pic;
            }
            c[3] *= shape;
            *o = to_u8(c);
        }
    });
    RgbaImage::from_raw(IRect::new(0, 0, nw as i32, nh as i32), out).unwrap_or_else(|_| img.clone())
}

/// `overlay.border`.
struct Border;

impl Step for Border {
    fn id(&self) -> &str {
        BORDER
    }
    fn group(&self) -> StepGroup {
        StepGroup::Overlay
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::number("width", 0.0, 5000.0, 20.0).beginner(),
            ParamDef::choice("unit", &["px", "percent"], "px").beginner(),
            ParamDef::color("color", "#ffffff").beginner(),
            ParamDef::number("radius", 0.0, 50.0, 0.0).unit("%"),
            ParamDef::choice("placement", &["outer", "inner"], "outer"),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        item.flatten_document()?;
        let (w, h) = item.dimensions();
        let scale = preview_scale(ctx, w as usize, h as usize);
        let cancel = ctx.cancel;
        item.map_images(|img| {
            cancel.check()?;
            *img = border_image(img, params, scale);
            Ok(())
        })?;
        if p_f64(params, "radius", 0.0) > 0.0
            && let Some(m) = item.meta_mut()
        {
            m.has_transparency = true;
        }
        Ok(())
    }
}

//! Format, metadata and overlay steps from the batch spec (`docs/BATCH.md` §3.2, §3.6, §3.7):
//!
//! - `format.colorDepth` – greyscale, indexed (n colours with dithering) or fewer bits per
//!   channel.
//! - `format.colorProfile` – keep, strip, assign sRGB (embed a generated sRGB profile) or convert
//!   RGB matrix/TRC profiles to sRGB.
//! - `metadata.export` – sidecar file (`<name>.json` / `<name>.csv`) with the source metadata,
//!   written next to the output by the executor's extra-output hook.
//! - `overlay.contactSheet` – collect step: all items as thumbnails in a grid with captions
//!   (several sheets with `perPage`).

use crate::error::{BatchError, Result};
use crate::item::{Item, ItemKind, Meta, Still};
use crate::meta::quantize;
use crate::step::{Issue, ParamDef, Step, StepContext, StepGroup, StepKind, StepRegistry};
use crate::steps::anim::{EXTRA_OUTPUT_STEM, EXTRA_SOURCE_NAME};
use fp_raster::{IRect, RgbaImage};
use rayon::prelude::*;
use serde_json::{Value, json};
use std::sync::Arc;

/// ID "Colour depth".
pub const COLOR_DEPTH: &str = "format.colorDepth";
/// ID "Colour profile".
pub const COLOR_PROFILE: &str = "format.colorProfile";
/// ID "Metadata as file".
pub const METADATA_EXPORT: &str = "metadata.export";
/// ID "Contact sheet".
pub const CONTACT_SHEET: &str = "overlay.contactSheet";
/// `Meta::extra`: sidecar of `metadata.export` (`{"format": "json"|"csv", "data": {…}}`),
/// written by the executor next to the output.
pub const EXTRA_SIDECAR: &str = "metadata.sidecar";

/// Registers the steps of this file.
pub fn register(r: &mut StepRegistry) {
    r.register(Arc::new(ColorDepth));
    r.register(Arc::new(ColorProfile));
    r.register(Arc::new(MetadataExport));
    r.register(Arc::new(ContactSheet));
}

fn p_str<'a>(p: &'a Value, k: &str, d: &'a str) -> &'a str {
    p.get(k).and_then(Value::as_str).unwrap_or(d)
}
fn p_int(p: &Value, k: &str, d: i64) -> i64 {
    p.get(k)
        .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f.round() as i64)))
        .unwrap_or(d)
}
fn p_bool(p: &Value, k: &str, d: bool) -> bool {
    p.get(k).and_then(Value::as_bool).unwrap_or(d)
}

// ------------------------------------------------------------------ Colour depth

/// Greyscale (Rec. 601 luminance), alpha unchanged.
pub fn grayscale(img: &mut RgbaImage) {
    img.data_mut().par_iter_mut().for_each(|p| {
        let l = (0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32).round() as u8;
        p[0] = l;
        p[1] = l;
        p[2] = l;
    });
}

/// Reduces every colour channel to `bits` bits (values spread over 0…255 again).
pub fn reduce_bits(img: &mut RgbaImage, bits: u32) {
    let levels = ((1u32 << bits.clamp(1, 8)) - 1) as f32;
    img.data_mut().par_iter_mut().for_each(|p| {
        for v in p.iter_mut().take(3) {
            *v = ((*v as f32 / 255.0 * levels).round() / levels * 255.0).round() as u8;
        }
    });
}

struct ColorDepth;

impl Step for ColorDepth {
    fn id(&self) -> &str {
        COLOR_DEPTH
    }
    fn group(&self) -> StepGroup {
        StepGroup::Format
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::choice("mode", &["grayscale", "indexed", "bits"], "indexed").beginner(),
            ParamDef::int("colors", 2, 256, 256)
                .beginner()
                .when("mode", &["indexed"]),
            ParamDef::choice(
                "dither",
                &["none", "ordered", "floydSteinberg"],
                "floydSteinberg",
            )
            .when("mode", &["indexed"]),
            ParamDef::int("bits", 1, 8, 5)
                .beginner()
                .when("mode", &["bits"]),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let mode = p_str(params, "mode", "indexed").to_string();
        let colors = p_int(params, "colors", 256).clamp(2, 256) as usize;
        let dither = quantize::Dither::parse(p_str(params, "dither", "floydSteinberg"));
        let bits = p_int(params, "bits", 5).clamp(1, 8) as u32;
        let cancel = ctx.cancel;
        item.map_images(|img| {
            cancel.check()?;
            match mode.as_str() {
                "grayscale" => grayscale(img),
                "bits" => reduce_bits(img, bits),
                _ => {
                    let w = img.width();
                    quantize::quantize(img.data_mut(), w, colors, dither, 1.0);
                }
            }
            Ok(())
        })
    }
}

// ------------------------------------------------------------------ Colour profile

const D50: [f64; 3] = [0.9642, 1.0, 0.8249];
/// sRGB primaries adapted to D50 (columns r, g, b), as in the common sRGB v2 profiles.
const SRGB_D50: [[f64; 3]; 3] = [
    [0.4361, 0.3851, 0.1431],
    [0.2225, 0.7169, 0.0606],
    [0.0139, 0.0971, 0.7141],
];

fn s15(v: f64) -> [u8; 4] {
    ((v * 65536.0).round() as i32).to_be_bytes()
}

fn srgb_to_linear(v: f64) -> f64 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(v: f64) -> f64 {
    let v = v.clamp(0.0, 1.0);
    if v <= 0.003_130_8 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

/// A small sRGB ICC profile (v2.1, display, matrix/TRC with a 1024-entry sRGB curve).
pub fn srgb_profile() -> Vec<u8> {
    let xyz = |v: [f64; 3]| {
        let mut b = b"XYZ \0\0\0\0".to_vec();
        for c in v {
            b.extend_from_slice(&s15(c));
        }
        b
    };
    let mut desc = b"desc\0\0\0\0".to_vec();
    let name = b"sRGB (Firepixel)\0";
    desc.extend_from_slice(&(name.len() as u32).to_be_bytes());
    desc.extend_from_slice(name);
    desc.extend_from_slice(&[0u8; 4 + 4 + 2 + 1 + 67]);
    let mut cprt = b"text\0\0\0\0".to_vec();
    cprt.extend_from_slice(b"No copyright, use freely\0");
    let mut curv = b"curv\0\0\0\0".to_vec();
    curv.extend_from_slice(&1024u32.to_be_bytes());
    for i in 0..1024 {
        let v = srgb_to_linear(i as f64 / 1023.0);
        curv.extend_from_slice(&((v * 65535.0).round() as u16).to_be_bytes());
    }
    let col = |c: usize| [SRGB_D50[0][c], SRGB_D50[1][c], SRGB_D50[2][c]];
    let blobs: Vec<(&[u8; 4], Vec<u8>)> = vec![
        (b"desc", desc),
        (b"cprt", cprt),
        (b"wtpt", xyz(D50)),
        (b"rXYZ", xyz(col(0))),
        (b"gXYZ", xyz(col(1))),
        (b"bXYZ", xyz(col(2))),
        (b"rTRC", curv),
    ];
    let tag_count = blobs.len() + 2; // gTRC and bTRC share the rTRC data
    let mut data = Vec::new();
    let mut table = Vec::new();
    let base = 128 + 4 + 12 * tag_count;
    let mut trc = (0u32, 0u32);
    for (sig, blob) in &blobs {
        while !(base + data.len()).is_multiple_of(4) {
            data.push(0);
        }
        let off = (base + data.len()) as u32;
        table.push((**sig, off, blob.len() as u32));
        if *sig == b"rTRC" {
            trc = (off, blob.len() as u32);
        }
        data.extend_from_slice(blob);
    }
    table.push((*b"gTRC", trc.0, trc.1));
    table.push((*b"bTRC", trc.0, trc.1));
    let size = (base + data.len()) as u32;
    let mut out = Vec::with_capacity(size as usize);
    out.extend_from_slice(&size.to_be_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&[2, 0x10, 0, 0]);
    out.extend_from_slice(b"mntrRGB XYZ ");
    out.extend_from_slice(&[0; 12]);
    out.extend_from_slice(b"acsp");
    out.extend_from_slice(&[0; 4 + 4 + 4 + 4 + 8 + 4]);
    for c in D50 {
        out.extend_from_slice(&s15(c));
    }
    out.extend_from_slice(&[0; 4 + 16 + 28]);
    debug_assert_eq!(out.len(), 128);
    out.extend_from_slice(&(tag_count as u32).to_be_bytes());
    for (sig, off, len) in table {
        out.extend_from_slice(&sig);
        out.extend_from_slice(&off.to_be_bytes());
        out.extend_from_slice(&len.to_be_bytes());
    }
    out.extend_from_slice(&data);
    out
}

/// Tone curve of a profile channel.
#[derive(Clone, Debug)]
enum Trc {
    Gamma(f64),
    Table(Vec<f64>),
    /// Parametric (`para`, function type 0…4): g, a, b, c, d, e, f.
    Para(u16, [f64; 7]),
}

impl Trc {
    fn eval(&self, x: f64) -> f64 {
        let x = x.clamp(0.0, 1.0);
        match self {
            Trc::Gamma(g) => x.powf(*g),
            Trc::Table(t) => {
                let pos = x * (t.len() - 1) as f64;
                let i = (pos.floor() as usize).min(t.len() - 2);
                let f = pos - i as f64;
                t[i] * (1.0 - f) + t[i + 1] * f
            }
            Trc::Para(kind, p) => {
                let [g, a, b, c, d, e, f] = *p;
                let pw = |v: f64| if v > 0.0 { v.powf(g) } else { 0.0 };
                match kind {
                    0 => pw(x),
                    1 => {
                        if x >= -b / a {
                            pw(a * x + b)
                        } else {
                            0.0
                        }
                    }
                    2 => {
                        if x >= -b / a {
                            pw(a * x + b) + c
                        } else {
                            c
                        }
                    }
                    3 => {
                        if x >= d {
                            pw(a * x + b)
                        } else {
                            c * x
                        }
                    }
                    _ => {
                        if x >= d {
                            pw(a * x + b) + e
                        } else {
                            c * x + f
                        }
                    }
                }
            }
        }
    }
}

fn be_u32(d: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_be_bytes(d.get(i..i + 4)?.try_into().ok()?))
}
fn s15_at(d: &[u8], i: usize) -> Option<f64> {
    Some(i32::from_be_bytes(d.get(i..i + 4)?.try_into().ok()?) as f64 / 65536.0)
}

fn icc_tag<'a>(icc: &'a [u8], sig: &[u8; 4]) -> Option<&'a [u8]> {
    let n = be_u32(icc, 128)? as usize;
    for k in 0..n.min(1000) {
        let e = 132 + 12 * k;
        if icc.get(e..e + 4)? == sig {
            let off = be_u32(icc, e + 4)? as usize;
            let len = be_u32(icc, e + 8)? as usize;
            return icc.get(off..off.checked_add(len)?);
        }
    }
    None
}

fn parse_trc(d: &[u8]) -> Option<Trc> {
    match d.get(0..4)? {
        b"curv" => {
            let n = be_u32(d, 8)? as usize;
            match n {
                0 => Some(Trc::Gamma(1.0)),
                1 => Some(Trc::Gamma(
                    u16::from_be_bytes(d.get(12..14)?.try_into().ok()?) as f64 / 256.0,
                )),
                _ => {
                    let t: Option<Vec<f64>> = (0..n)
                        .map(|i| {
                            d.get(12 + 2 * i..14 + 2 * i)
                                .and_then(|b| b.try_into().ok())
                                .map(|b: [u8; 2]| u16::from_be_bytes(b) as f64 / 65535.0)
                        })
                        .collect();
                    t.map(Trc::Table)
                }
            }
        }
        b"para" => {
            let kind = u16::from_be_bytes(d.get(8..10)?.try_into().ok()?);
            let count = [1, 3, 4, 5, 7].get(kind as usize).copied()?;
            let mut p = [0.0; 7];
            for (i, v) in p.iter_mut().enumerate().take(count) {
                *v = s15_at(d, 12 + 4 * i)?;
            }
            Some(Trc::Para(kind, p))
        }
        _ => None,
    }
}

/// RGB matrix/TRC profile: (matrix to D50 XYZ, rows; curves r, g, b). `None` for other profiles.
fn parse_matrix_profile(icc: &[u8]) -> Option<([[f64; 3]; 3], [Trc; 3])> {
    if icc.get(16..20)? != b"RGB " {
        return None;
    }
    let col = |sig: &[u8; 4]| -> Option<[f64; 3]> {
        let d = icc_tag(icc, sig)?;
        (d.get(0..4)? == b"XYZ ").then_some(())?;
        Some([s15_at(d, 8)?, s15_at(d, 12)?, s15_at(d, 16)?])
    };
    let (r, g, b) = (col(b"rXYZ")?, col(b"gXYZ")?, col(b"bXYZ")?);
    let m = [[r[0], g[0], b[0]], [r[1], g[1], b[1]], [r[2], g[2], b[2]]];
    let trc = [
        parse_trc(icc_tag(icc, b"rTRC")?)?,
        parse_trc(icc_tag(icc, b"gTRC")?)?,
        parse_trc(icc_tag(icc, b"bTRC")?)?,
    ];
    Some((m, trc))
}

fn invert3(m: [[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    if det.abs() < 1e-12 {
        return None;
    }
    let c = |a: usize, b: usize, x: usize, y: usize| m[a][b] * m[x][y] - m[a][y] * m[x][b];
    Some([
        [
            c(1, 1, 2, 2) / det,
            -c(0, 1, 2, 2) / det,
            c(0, 1, 1, 2) / det,
        ],
        [
            -c(1, 0, 2, 2) / det,
            c(0, 0, 2, 2) / det,
            -c(0, 0, 1, 2) / det,
        ],
        [
            c(1, 0, 2, 1) / det,
            -c(0, 0, 2, 1) / det,
            c(0, 0, 1, 1) / det,
        ],
    ])
}

fn mul(a: [[f64; 3]; 3], b: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut o = [[0.0; 3]; 3];
    for (i, row) in o.iter_mut().enumerate() {
        for (j, v) in row.iter_mut().enumerate() {
            *v = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    o
}

/// Converts pixels from an RGB matrix/TRC profile to sRGB. `false` = profile not supported.
pub fn convert_to_srgb(img: &mut RgbaImage, icc: &[u8]) -> bool {
    let Some((m, trc)) = parse_matrix_profile(icc) else {
        return false;
    };
    let Some(inv) = invert3(SRGB_D50) else {
        return false;
    };
    let t = mul(inv, m);
    // per-channel lookup of the source curves (8-bit input)
    let lut: Vec<[f64; 3]> = (0..256)
        .map(|v| {
            let x = v as f64 / 255.0;
            [trc[0].eval(x), trc[1].eval(x), trc[2].eval(x)]
        })
        .collect();
    img.data_mut().par_iter_mut().for_each(|p| {
        let lin = [
            lut[p[0] as usize][0],
            lut[p[1] as usize][1],
            lut[p[2] as usize][2],
        ];
        for (c, v) in p.iter_mut().take(3).enumerate() {
            let s = t[c][0] * lin[0] + t[c][1] * lin[1] + t[c][2] * lin[2];
            *v = (linear_to_srgb(s) * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    });
    true
}

struct ColorProfile;

impl Step for ColorProfile {
    fn id(&self) -> &str {
        COLOR_PROFILE
    }
    fn group(&self) -> StepGroup {
        StepGroup::Format
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::choice(
                "mode",
                &["keep", "convertToSrgb", "assignSrgb", "strip"],
                "convertToSrgb",
            )
            .beginner(),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        match p_str(params, "mode", "convertToSrgb") {
            "keep" => Ok(()),
            "strip" => {
                if let Some(m) = item.meta_mut() {
                    m.icc = None;
                }
                Ok(())
            }
            "assignSrgb" => {
                if let Some(m) = item.meta_mut() {
                    m.icc = Some(srgb_profile());
                }
                Ok(())
            }
            _ => {
                item.flatten_document()?;
                let Some(icc) = item.meta().and_then(|m| m.icc.clone()) else {
                    // no profile = sRGB already; tag it
                    if let Some(m) = item.meta_mut() {
                        m.icc = Some(srgb_profile());
                    }
                    return Ok(());
                };
                if parse_matrix_profile(&icc).is_none() {
                    ctx.warn(fp_i18n::t("batch-warn-profile-unsupported"));
                    return Ok(());
                }
                let cancel = ctx.cancel;
                item.map_images(|img| {
                    cancel.check()?;
                    convert_to_srgb(img, &icc);
                    Ok(())
                })?;
                if let Some(m) = item.meta_mut() {
                    m.icc = Some(srgb_profile());
                }
                Ok(())
            }
        }
    }
}

// ------------------------------------------------------------------ Metadata as file

/// Sidecar data of an item: source facts plus the metadata overview.
pub fn sidecar_data(ctx: &StepContext, item: &Item) -> Value {
    let i = ctx.info;
    let empty = Meta::default();
    let meta = item.meta().unwrap_or(&empty);
    let (w, h) = item.dimensions();
    json!({
        "source": i.path.to_string_lossy(),
        "file": i.file_name(),
        "format": i.format,
        "bytes": i.bytes,
        "width": w,
        "height": h,
        "frames": match item { Item::Animation(a) => a.frames.len(), _ => 1 },
        "dpi": meta.dpi,
        "hasTransparency": meta.has_transparency,
        "exifSummary": i.exif,
        "metadata": crate::meta::info(meta),
    })
}

/// CSV columns of the sidecar.
pub const CSV_COLUMNS: [&str; 12] = [
    "file",
    "format",
    "bytes",
    "width",
    "height",
    "frames",
    "make",
    "model",
    "date",
    "orientation",
    "gps",
    "icc",
];

/// Sidecar as CSV (header + one row; RFC 4180 quoting).
pub fn sidecar_csv(d: &Value) -> String {
    let m = &d["metadata"];
    let cell = |v: &Value| match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    let vals = [
        cell(&d["file"]),
        cell(&d["format"]),
        cell(&d["bytes"]),
        cell(&d["width"]),
        cell(&d["height"]),
        cell(&d["frames"]),
        cell(&m["exif"]["make"]),
        cell(&m["exif"]["model"]),
        cell(&m["exif"]["date"]),
        cell(&m["exif"]["orientation"]),
        cell(&m["exif"]["gps"]),
        cell(&m["icc"]["description"]),
    ];
    let q = |s: &str| {
        if s.contains([',', '"', '\n', '\r']) {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_string()
        }
    };
    let row: Vec<String> = vals.iter().map(|s| q(s)).collect();
    format!("{}\r\n{}\r\n", CSV_COLUMNS.join(","), row.join(","))
}

struct MetadataExport;

impl Step for MetadataExport {
    fn id(&self) -> &str {
        METADATA_EXPORT
    }
    fn group(&self) -> StepGroup {
        StepGroup::Metadata
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::choice("format", &["json", "csv"], "json").beginner()]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn preview_ok(&self) -> bool {
        false
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let data = sidecar_data(ctx, item);
        let format = p_str(params, "format", "json").to_string();
        if let Some(m) = item.meta_mut() {
            m.extra.insert(
                EXTRA_SIDECAR.into(),
                json!({"format": format, "data": data}),
            );
        }
        Ok(())
    }
}

/// Text of a sidecar stored by `metadata.export`: (extension, content).
pub fn sidecar_file(item: &Item) -> Option<(String, String)> {
    let s = item.meta()?.extra.get(EXTRA_SIDECAR)?;
    let data = &s["data"];
    match s["format"].as_str() {
        Some("csv") => Some(("csv".into(), sidecar_csv(data))),
        _ => serde_json::to_string_pretty(data)
            .ok()
            .map(|t| ("json".into(), t + "\n")),
    }
}

// ------------------------------------------------------------------ Contact sheet

fn over(dst: &mut [u8; 4], src: [u8; 4]) {
    let a = src[3] as f32 / 255.0;
    if a <= 0.0 {
        return;
    }
    let da = dst[3] as f32 / 255.0;
    let oa = a + da * (1.0 - a);
    for c in 0..3 {
        let v = (src[c] as f32 * a + dst[c] as f32 * da * (1.0 - a)) / oa.max(1e-6);
        dst[c] = v.round().clamp(0.0, 255.0) as u8;
    }
    dst[3] = (oa * 255.0).round() as u8;
}

fn blit(dst: &mut [[u8; 4]], dw: usize, src: &RgbaImage, x0: usize, y0: usize, max_w: usize) {
    let (sw, sh) = (src.width().min(max_w), src.height());
    let dh = dst.len() / dw.max(1);
    let d = src.data();
    for y in 0..sh {
        if y0 + y >= dh {
            break;
        }
        for x in 0..sw {
            if x0 + x >= dw {
                break;
            }
            over(&mut dst[(y0 + y) * dw + x0 + x], d[y * src.width() + x]);
        }
    }
}

/// Thumbnail that fits into `size × size` (never enlarged).
fn thumb(img: &RgbaImage, size: u32) -> RgbaImage {
    let (w, h) = (img.width() as u32, img.height() as u32);
    let s = (size as f64 / w.max(h).max(1) as f64).min(1.0);
    let (nw, nh) = (
        ((w as f64 * s).round() as u32).max(1),
        ((h as f64 * s).round() as u32).max(1),
    );
    let mut t = if (nw, nh) == (w, h) {
        img.clone()
    } else {
        fp_raster::resample::resize(img, nw, nh, fp_raster::Sampler::Bicubic)
    };
    t.set_origin(0, 0);
    t
}

struct ContactSheet;

impl Step for ContactSheet {
    fn id(&self) -> &str {
        CONTACT_SHEET
    }
    fn group(&self) -> StepGroup {
        StepGroup::Overlay
    }
    fn kind(&self) -> StepKind {
        StepKind::Collect
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::int("columns", 1, 50, 4).beginner(),
            ParamDef::int("cellSize", 16, 2048, 256)
                .unit("px")
                .beginner(),
            ParamDef::int("spacing", 0, 200, 8).unit("px"),
            ParamDef::color("background", "#ffffff").beginner(),
            ParamDef::bool("captions", true).beginner(),
            ParamDef::int("captionSize", 6, 72, 12)
                .unit("px")
                .when("captions", &["true"]),
            ParamDef::color("captionColor", "#000000").when("captions", &["true"]),
            ParamDef::int("perPage", 0, 10_000, 0),
            ParamDef::text("name", "contact-sheet"),
        ]
    }
    fn validate(&self, params: &Value) -> Vec<Issue> {
        crate::step::validate_params(&self.params(), params)
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
        let cols = p_int(params, "columns", 4).clamp(1, 50) as usize;
        let cell = p_int(params, "cellSize", 256).clamp(16, 2048) as usize;
        let gap = p_int(params, "spacing", 8).clamp(0, 200) as usize;
        let bg = crate::step::parse_color(p_str(params, "background", "#ffffff"))
            .unwrap_or([255, 255, 255, 255]);
        let captions = p_bool(params, "captions", true);
        let cap_size = p_int(params, "captionSize", 12).clamp(6, 72) as usize;
        let cap_color = p_str(params, "captionColor", "#000000").to_string();
        let per_page = match p_int(params, "perPage", 0).max(0) as usize {
            0 => usize::MAX,
            n => n,
        };
        let base = match p_str(params, "name", "contact-sheet").trim() {
            "" => "contact-sheet".to_string(),
            n => {
                let mut hints = Vec::new();
                crate::naming::sanitize_stem(n, &mut hints)
            }
        };
        let cap_h = if captions {
            (cap_size as f64 * 1.6).ceil() as usize
        } else {
            0
        };
        let mut cells: Vec<(RgbaImage, String, Meta)> = Vec::new();
        for item in items {
            ctx.check_cancel()?;
            let img = item.preview_image()?;
            let name = item
                .meta()
                .and_then(|m| m.extra.get(EXTRA_SOURCE_NAME))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let meta = item.meta().cloned().unwrap_or_default();
            cells.push((thumb(&img, cell as u32), name, meta));
        }
        if cells.is_empty() {
            return Err(BatchError::Invalid(fp_i18n::t("batch-anim-err-empty")));
        }
        let pages: Vec<&[(RgbaImage, String, Meta)]> =
            cells.chunks(per_page.min(cells.len())).collect();
        let many = pages.len() > 1;
        let mut out = Vec::new();
        for (pi, page) in pages.iter().enumerate() {
            ctx.check_cancel()?;
            let n = page.len();
            let c = cols.min(n);
            let rows = n.div_ceil(c);
            let (w, h) = (
                c * cell + (c + 1) * gap,
                rows * (cell + cap_h) + (rows + 1) * gap,
            );
            let mut px = vec![bg; w * h];
            for (k, (t, name, _)) in page.iter().enumerate() {
                let (r, cc) = (k / c, k % c);
                let x0 = gap + cc * (cell + gap);
                let y0 = gap + r * (cell + cap_h + gap);
                let (tw, th) = (t.width(), t.height());
                blit(
                    &mut px,
                    w,
                    t,
                    x0 + (cell - tw) / 2,
                    y0 + (cell - th) / 2,
                    cell,
                );
                if captions
                    && !name.is_empty()
                    && let Some(stamp) = crate::steps::overlay::text_stamp(
                        name,
                        cap_size as f64,
                        &json!({"color": cap_color, "position": "bottom"}),
                    )
                {
                    let sx = x0 + cell.saturating_sub(stamp.width()) / 2;
                    blit(
                        &mut px,
                        w,
                        &stamp,
                        sx,
                        y0 + cell + (cap_h.saturating_sub(stamp.height())) / 2,
                        cell,
                    );
                }
            }
            let image = RgbaImage::from_raw(IRect::new(0, 0, w as i32, h as i32), px)
                .map_err(|e| BatchError::Decode(e.to_string()))?;
            let mut meta = page[0].2.clone();
            meta.exif = None;
            meta.xmp = None;
            meta.has_transparency = bg[3] < 255;
            let stem = if many {
                format!("{base}-{}", pi + 1)
            } else {
                base.clone()
            };
            meta.extra.insert(EXTRA_OUTPUT_STEM.into(), json!(stem));
            out.push(Item::Still(Still { image, meta }));
        }
        Ok(out)
    }
}

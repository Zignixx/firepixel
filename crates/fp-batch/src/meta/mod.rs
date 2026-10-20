//! Metadaten und Optimieren (Paket BATCH-5, SPEC-BATCH §3.2, §3.7, §3.12).
//!
//! - Beim Dekodieren liest [`read_into`] EXIF/ICC/XMP/IPTC/Kommentar aus JPEG, PNG und WebP in die
//!   [`Meta`] des Elements und wendet die EXIF-Ausrichtung an (Pixel gedreht, Tag danach 1; SPEC §3.7:
//!   „Ohne Schritt ‚Metadaten‘ gilt `keep: all`, die Ausrichtung wird angewendet und auf 1 gesetzt“).
//! - Die Schritte `metadata.*` ändern nur die [`Meta`]; `optimize.*` legen ihre Wünsche in
//!   `Meta::extra` ab ([`EXTRA_PNG_OPT`], [`EXTRA_TARGET`], [`EXTRA_OUTPUT_OPTIONS`]).
//! - Beim Kodieren ([`finish_encode`], eingehängt in `output::encode`) werden Formatoptionen ergänzt,
//!   eigene Kodierer (TIFF mit LZW/Deflate) benutzt, PNG verlustfrei optimiert, die Zielgröße per
//!   Suche erreicht und zuletzt die Metadaten in die Datei geschrieben (JPEG, PNG/APNG, WebP, TIFF;
//!   BA-108: AVIF and JPEG XL with EXIF and XMP, IPTC fields carried into XMP). Andere Formate
//!   verlieren sie mit Hinweis.

pub mod container;
pub mod encoders;
pub mod exif;
pub mod iptc; // B-4
pub mod pngopt;
pub mod quantize;
pub mod target;
pub mod xmp;

use crate::error::Result;
use crate::item::{ExifSummary, Item, Meta};
use crate::output::OutputFormat;
use fp_raster::RgbaImage;
use serde_json::{Map, Value, json};
use std::path::Path;

/// IPTC-IIM (Hex-Text) in `Meta::extra`.
pub const EXTRA_IPTC: &str = "meta.iptc";
/// Kommentar (Text) in `Meta::extra`.
pub const EXTRA_COMMENT: &str = "meta.comment";
/// Ausrichtung der Quelle (1…8), wenn sie beim Laden angewendet wurde.
pub const EXTRA_ORIENTATION_APPLIED: &str = "meta.orientationApplied";
/// Parameter von `optimize.png` (Objekt).
pub const EXTRA_PNG_OPT: &str = "optimize.png";
/// Parameter von `optimize.targetSize` (Objekt).
pub const EXTRA_TARGET: &str = "optimize.targetSize";
/// Zusätzliche Formatoptionen (überschreiben `recipe.output.options`), z. B. nach `optimize.quantize`.
pub const EXTRA_OUTPUT_OPTIONS: &str = "output.options";

/// Hex-Text ↔ Bytes (IPTC in `extra`).
pub fn to_hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
pub fn from_hex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
}

/// IPTC-Rohdaten eines Elements.
pub fn iptc(meta: &Meta) -> Option<Vec<u8>> {
    meta.extra
        .get(EXTRA_IPTC)
        .and_then(Value::as_str)
        .and_then(from_hex)
}

/// Kommentar eines Elements.
pub fn comment(meta: &Meta) -> Option<String> {
    meta.extra
        .get(EXTRA_COMMENT)
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// Metadaten des Elements als Container-Inhalt.
pub fn found(meta: &Meta) -> container::Found {
    container::Found {
        exif: meta.exif.clone(),
        icc: meta.icc.clone(),
        xmp: meta.xmp.clone(),
        iptc: iptc(meta),
        comment: comment(meta),
    }
}

/// Dreht/spiegelt ein Bild nach EXIF-Ausrichtung (1…8) in die aufrechte Lage.
pub fn orient(img: &RgbaImage, o: u16) -> RgbaImage {
    let (w, h) = (img.width(), img.height());
    if !(2..=8).contains(&o) || w == 0 || h == 0 {
        return img.clone();
    }
    let swap = o >= 5;
    let (nw, nh) = if swap { (h, w) } else { (w, h) };
    let src = img.data();
    let mut out = Vec::with_capacity(nw * nh);
    for y in 0..nh {
        for x in 0..nw {
            let (sx, sy) = match o {
                2 => (w - 1 - x, y),
                3 => (w - 1 - x, h - 1 - y),
                4 => (x, h - 1 - y),
                5 => (y, x),
                6 => (y, h - 1 - x),
                7 => (w - 1 - y, h - 1 - x),
                _ => (w - 1 - y, x),
            };
            out.push(src[sy * w + sx]);
        }
    }
    RgbaImage::from_raw(fp_raster::IRect::new(0, 0, nw as i32, nh as i32), out)
        .unwrap_or_else(|_| img.clone())
}

/// Ausrichtung der Metadaten (1 = aufrecht bzw. unbekannt).
pub fn orientation(meta: &Meta) -> u16 {
    meta.exif
        .as_deref()
        .and_then(exif::Exif::parse)
        .and_then(|e| e.orientation())
        .unwrap_or(1)
}

/// Wendet die EXIF-Ausrichtung auf die Pixel an und setzt sie in EXIF und XMP auf 1.
/// Liefert die angewendete Ausrichtung (1 = nichts zu tun).
pub fn apply_orientation(item: &mut Item) -> u16 {
    let Some(meta) = item.meta() else { return 1 };
    let o = orientation(meta);
    if o == 1 {
        return 1;
    }
    match item {
        Item::Still(s) => s.image = orient(&s.image, o),
        Item::Animation(a) => {
            for f in &mut a.frames {
                f.image = orient(&f.image, o);
            }
            if let Some(f) = a.frames.first() {
                a.width = f.image.width() as u32;
                a.height = f.image.height() as u32;
            }
        }
        _ => return 1,
    }
    if let Some(m) = item.meta_mut() {
        set_orientation_one(m);
        m.extra.insert(EXTRA_ORIENTATION_APPLIED.into(), json!(o));
    }
    o
}

/// Setzt die Ausrichtung in EXIF und XMP auf 1 (ohne Pixel zu ändern).
pub fn set_orientation_one(m: &mut Meta) {
    if let Some(e) = m.exif.as_mut()
        && !exif::set_orientation_in_place(e, 1)
        && let Some(mut x) = exif::Exif::parse(e)
    {
        x.set_orientation(1);
        *e = x.write();
    }
    if let Some(x) = m.xmp.as_mut() {
        let s = xmp::set_orientation(&String::from_utf8_lossy(x), 1);
        *x = s.into_bytes();
    }
}

/// Liest die Metadaten einer Quelldatei in das Element und wendet die Ausrichtung an
/// (eingehängt in `item::decode_bytes`).
pub fn read_into(data: &[u8], item: &mut Item) {
    let f = container::read(data);
    if f.is_empty() {
        return;
    }
    if let Some(m) = item.meta_mut() {
        m.exif = f.exif;
        m.icc = f.icc;
        m.xmp = f.xmp;
        if let Some(i) = f.iptc {
            m.extra.insert(EXTRA_IPTC.into(), json!(to_hex(&i)));
        }
        if let Some(c) = f.comment {
            m.extra.insert(EXTRA_COMMENT.into(), json!(c));
        }
    }
    apply_orientation(item);
}

/// EXIF-Leser für Namensbausteine (`item::set_exif_probe`).
pub fn exif_probe(_path: &Path, data: &[u8]) -> Option<ExifSummary> {
    let f = container::read(data);
    f.exif
        .as_deref()
        .and_then(exif::Exif::parse)
        .map(|e| e.summary())
}

/// Hängt den EXIF-Leser ein (einmal je Prozess; weitere Aufrufe ohne Wirkung).
pub fn install() {
    let _ = crate::item::set_exif_probe(exif_probe);
    crate::output::register_encoder(OutputFormat::Tga, encoders::encode_tga);
}

/// Kann das Format Metadaten tragen? (EXIF, ICC, XMP)
pub fn format_keeps(fmt: OutputFormat) -> (bool, bool, bool) {
    match fmt {
        OutputFormat::Jpeg | OutputFormat::Png | OutputFormat::Apng | OutputFormat::WebP => {
            (true, true, true)
        }
        OutputFormat::Tiff => (true, true, true),
        // BA-108: EXIF and XMP boxes, no ICC
        OutputFormat::Avif | OutputFormat::Jxl => (true, false, true),
        _ => (false, false, false),
    }
}

/// Schreibt die Metadaten des Elements in kodierte Bytes. Liefert Bytes und ggf. einen Hinweis.
pub fn embed(bytes: Vec<u8>, fmt: OutputFormat, meta: &Meta) -> (Vec<u8>, Option<String>) {
    let f = found(meta);
    let lost = |f: &container::Found| {
        fp_i18n::tr!(
            "batch-meta-warn-format-drops",
            format = fmt.display_name(),
            what = what_list(f)
        )
    };
    match fmt {
        OutputFormat::Jpeg => match container::write_jpeg(&bytes, &f) {
            Some(b) => (b, None),
            None => (bytes, None),
        },
        OutputFormat::Png | OutputFormat::Apng => {
            let f2 = container::Found {
                iptc: None,
                ..f.clone()
            };
            let note = f.iptc.is_some().then(|| {
                lost(&container::Found {
                    iptc: f.iptc.clone(),
                    ..Default::default()
                })
            });
            match container::write_png(&bytes, &f2) {
                Some(b) => (b, note),
                None => (bytes, note),
            }
        }
        OutputFormat::WebP => {
            let rest = container::Found {
                iptc: f.iptc.clone(),
                comment: f.comment.clone(),
                ..Default::default()
            };
            let note = (!rest.is_empty()).then(|| lost(&rest));
            if f.exif.is_none() && f.icc.is_none() && f.xmp.is_none() {
                return (bytes, note);
            }
            match container::write_webp(&bytes, &f) {
                Some(b) => (b, note),
                None => (bytes, note),
            }
        }
        OutputFormat::Tiff => {
            // EXIF/ICC/XMP/IPTC schreibt der TIFF-Kodierer selbst (B-4: EXIF und IPTC)
            let rest = container::Found {
                comment: f.comment.clone(),
                ..Default::default()
            };
            (bytes, (!rest.is_empty()).then(|| lost(&rest)))
        }
        OutputFormat::Avif | OutputFormat::Jxl => {
            // BA-108: EXIF/XMP as items or boxes; IPTC fields go into XMP
            let iptc_kept = f.iptc.as_deref().is_some_and(xmp::iptc_fits_xmp);
            let packet = f
                .iptc
                .as_deref()
                .and_then(|i| xmp::with_iptc(f.xmp.as_deref(), i))
                .or_else(|| f.xmp.clone());
            let rest = container::Found {
                icc: f.icc.clone(),
                comment: f.comment.clone(),
                iptc: f.iptc.clone().filter(|_| !iptc_kept),
                ..Default::default()
            };
            let note = (!rest.is_empty()).then(|| lost(&rest));
            let w = container::Found {
                exif: f.exif.clone(),
                xmp: packet,
                ..Default::default()
            };
            let written = if fmt == OutputFormat::Avif {
                container::write_heif(&bytes, &w)
            } else {
                container::write_jxl(&bytes, &w)
            };
            match written {
                Some(b) => (b, note),
                None => (bytes, (!f.is_empty()).then(|| lost(&f))),
            }
        }
        _ => (bytes, (!f.is_empty()).then(|| lost(&f))),
    }
}

/// Aufzählung der vorhandenen Metadaten-Arten (`EXIF, ICC, XMP`).
fn what_list(f: &container::Found) -> String {
    let mut v: Vec<String> = Vec::new();
    if f.exif.is_some() {
        v.push("EXIF".into());
    }
    if f.icc.is_some() {
        v.push("ICC".into());
    }
    if f.xmp.is_some() {
        v.push("XMP".into());
    }
    if f.iptc.is_some() {
        v.push("IPTC".into());
    }
    if f.comment.is_some() {
        v.push(fp_i18n::t("batch-meta-what-comment"));
    }
    v.join(", ")
}

/// Formatoptionen mit den Ergänzungen aus `Meta::extra` ([`EXTRA_OUTPUT_OPTIONS`]).
pub fn effective_options(item: &Item, options: &Map<String, Value>) -> Map<String, Value> {
    let mut o = options.clone();
    if let Some(x) = item
        .meta()
        .and_then(|m| m.extra.get(EXTRA_OUTPUT_OPTIONS))
        .and_then(Value::as_object)
    {
        for (k, v) in x {
            o.insert(k.clone(), v.clone());
        }
    }
    o
}

/// Kodierer ohne Nachbearbeitung (`output::encode_plain`).
pub type PlainEncoder =
    fn(&Item, OutputFormat, &Map<String, Value>) -> Result<(Vec<u8>, Vec<String>)>;

/// Einmal kodieren: eigener TIFF-Kodierer, PNG-Optimierung, Metadaten. Liefert Bytes + Hinweise.
pub fn encode_once(
    item: &Item,
    fmt: OutputFormat,
    options: &Map<String, Value>,
    plain: PlainEncoder,
) -> Result<(Vec<u8>, Vec<String>)> {
    let empty = Meta::default();
    let meta = item.meta().unwrap_or(&empty);
    let (mut bytes, warnings) = match fmt {
        OutputFormat::Tiff if !matches!(item, Item::FileOnly(_)) => {
            let mut w = Vec::new();
            let c = encoders::TiffCompression::parse(
                options.get("compression").and_then(Value::as_str),
            );
            if crate::steps::multipage::is_multi_page(item) {
                // B-4: every frame is a page
                let pages = crate::steps::multipage::pages_of(item)?;
                return finish_plain(encoders::write_tiff_pages(&pages, meta, c)?, fmt, meta, w);
            }
            if matches!(item, Item::Animation(a) if a.frames.len() > 1) {
                w.push(fp_i18n::tr!(
                    "batch-warn-first-frame-only",
                    format = fmt.display_name()
                ));
            }
            (
                encoders::write_tiff(&encoders::first_image(item)?, meta, c)?,
                w,
            )
        }
        _ => plain(item, fmt, options)?,
    };
    if fmt == OutputFormat::Png {
        let req = meta.extra.get(EXTRA_PNG_OPT);
        let max = options.get("compression").and_then(Value::as_str) == Some("max");
        if req.is_some() || max {
            let level = req
                .and_then(|r| r.get("level"))
                .and_then(Value::as_i64)
                .unwrap_or(if max { 3 } else { 2 })
                .clamp(1, 6) as u8;
            let alpha_clean = req
                .and_then(|r| r.get("alphaClean"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            bytes = pngopt::optimize(&bytes, pngopt::PngOptOptions { level, alpha_clean });
        }
    }
    finish_plain(bytes, fmt, meta, warnings)
}

/// Embeds the metadata and collects the note.
fn finish_plain(
    bytes: Vec<u8>,
    fmt: OutputFormat,
    meta: &Meta,
    mut warnings: Vec<String>,
) -> Result<(Vec<u8>, Vec<String>)> {
    let (bytes, note) = embed(bytes, fmt, meta);
    warnings.extend(note);
    Ok((bytes, warnings))
}

/// Nachbearbeitung jeder Kodierung (eingehängt in `output::encode`): Optionen ergänzen, ggf.
/// Zielgröße suchen, sonst einmal kodieren.
pub fn finish_encode(
    item: &Item,
    fmt: OutputFormat,
    options: &Map<String, Value>,
    plain: PlainEncoder,
) -> Result<(Vec<u8>, Vec<String>)> {
    let opts = effective_options(item, options);
    let target = item.meta().and_then(|m| m.extra.get(EXTRA_TARGET)).cloned();
    let (bytes, mut warnings) = match target {
        Some(t) if fmt != OutputFormat::FwPng => target::search(
            item,
            fmt,
            &opts,
            &target::TargetSpec::from_params(&t),
            plain,
        )?,
        _ => encode_once(item, fmt, &opts, plain)?,
    };
    let mut seen = std::collections::HashSet::new();
    warnings.retain(|w| seen.insert(w.clone()));
    Ok((bytes, warnings))
}

/// Übersicht der Metadaten für die Info-Tabelle vorher/nachher (Schlüssel technisch, Werte roh):
/// `{exif: {make, model, date, orientation, gps, camera, software, thumbnail}, icc: Bytes, xmp: Bytes,
/// iptc: Bytes, comment}`.
pub fn info(meta: &Meta) -> Value {
    let mut o = Map::new();
    if let Some(e) = meta.exif.as_deref().and_then(exif::Exif::parse) {
        let s = e.summary();
        o.insert(
            "exif".into(),
            json!({
                "bytes": meta.exif.as_ref().map(Vec::len),
                "make": s.make, "model": s.model, "lens": s.lens,
                "date": s.date_taken.map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string()),
                "orientation": s.orientation,
                "gps": e.has_gps(),
                "artist": e.find(exif::TAG_ARTIST).and_then(exif::Exif::text),
                "copyright": e.find(exif::TAG_COPYRIGHT).and_then(exif::Exif::text),
                "software": e.find(exif::TAG_SOFTWARE).and_then(exif::Exif::text),
                "thumbnail": e.thumbnail.is_some(),
                "tags": e.ifd0.entries.len() + e.exif.as_ref().map_or(0, |x| x.entries.len()) + e.gps.as_ref().map_or(0, |x| x.entries.len()),
            }),
        );
    }
    if let Some(i) = &meta.icc {
        o.insert(
            "icc".into(),
            json!({"bytes": i.len(), "description": icc_description(i)}),
        );
    }
    if let Some(x) = &meta.xmp {
        let s = String::from_utf8_lossy(x);
        o.insert(
            "xmp".into(),
            json!({"bytes": x.len(), "gps": xmp::has_prop(&s, "exif:GPS")}),
        );
    }
    if let Some(i) = iptc(meta) {
        o.insert("iptc".into(), json!({"bytes": i.len()}));
    }
    if let Some(c) = comment(meta) {
        o.insert("comment".into(), json!(c));
    }
    Value::Object(o)
}

/// Beschreibung eines ICC-Profils (`desc`-Tag, v2 `desc` bzw. v4 `mluc`).
pub fn icc_description(icc: &[u8]) -> Option<String> {
    let be32 = |i: usize| {
        icc.get(i..i + 4)
            .map(|s| u32::from_be_bytes([s[0], s[1], s[2], s[3]]) as usize)
    };
    let n = be32(128)?;
    for k in 0..n.min(100) {
        let p = 132 + 12 * k;
        if icc.get(p..p + 4)? != b"desc" {
            continue;
        }
        let (off, len) = (be32(p + 4)?, be32(p + 8)?);
        let d = icc.get(off..off + len)?;
        return match d.get(0..4)? {
            b"desc" => {
                let l = u32::from_be_bytes(d.get(8..12)?.try_into().ok()?) as usize;
                Some(
                    String::from_utf8_lossy(d.get(12..12 + l)?)
                        .trim_end_matches('\0')
                        .to_string(),
                )
            }
            b"mluc" => {
                let l = u32::from_be_bytes(d.get(20..24)?.try_into().ok()?) as usize;
                let o = u32::from_be_bytes(d.get(24..28)?.try_into().ok()?) as usize;
                let u: Vec<u16> = d
                    .get(o..o + l)?
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|c| u16::from_be_bytes(*c))
                    .collect();
                Some(String::from_utf16_lossy(&u))
            }
            _ => None,
        };
    }
    None
}

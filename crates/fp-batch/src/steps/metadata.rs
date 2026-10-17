//! Gruppe „Metadaten“ (SPEC-BATCH §3.7): `metadata.edit` (behalten/entfernen, GPS, Kamera,
//! Software, Vorschaubild, XMP, IPTC, Kommentare, ICC, einzelne EXIF-Tags), `metadata.set`
//! (Urheber, Copyright, Beschreibung, Titel, Stichwörter, Bewertung, Aufnahmedatum fest/verschoben,
//! GPS entfernen; Platzhalter `{name}`, `{n}`, `{date}`, `{year}`) und `metadata.orientation`
//! (EXIF-Ausrichtung in Pixel umsetzen, Tag = 1).
//!
//! Die Schritte ändern nur die Metadaten des Elements ([`crate::item::Meta`]); geschrieben werden
//! sie beim Kodieren (`meta::finish_encode`). Jeder Schritt ist idempotent: zweimal anwenden ergibt
//! dasselbe wie einmal.

use crate::error::Result;
use crate::item::{Item, ItemKind, Meta};
use crate::meta::{self, EXTRA_COMMENT, EXTRA_IPTC, exif, xmp};
use crate::step::{Issue, ParamDef, Step, StepContext, StepGroup, StepRegistry};
use serde_json::Value;
use std::sync::Arc;

/// IDs.
pub const EDIT: &str = "metadata.edit";
pub const SET: &str = "metadata.set";
pub const ORIENTATION: &str = "metadata.orientation";

/// Kategorien für `remove`.
pub const CATEGORIES: [&str; 8] = [
    "gps",
    "camera",
    "software",
    "thumbnails",
    "xmp",
    "iptc",
    "comments",
    "icc",
];

/// Registriert die Schritte der Gruppe (und den EXIF-Leser für Namensbausteine, TGA-Kodierer).
pub fn register(r: &mut StepRegistry) {
    meta::install();
    r.register(Arc::new(Edit));
    r.register(Arc::new(Set));
    r.register(Arc::new(Orientation));
}

fn strs(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// XMP-Eigenschaften je Kategorie.
fn xmp_prefixes(cat: &str) -> &'static [&'static str] {
    match cat {
        "gps" => &["exif:GPS", "exifEX:GPS"],
        "camera" => &[
            "tiff:Make",
            "tiff:Model",
            "aux:SerialNumber",
            "aux:Lens",
            "aux:OwnerName",
            "exifEX:BodySerialNumber",
            "exifEX:Lens",
            "exifEX:CameraOwnerName",
        ],
        "software" => &["xmp:CreatorTool", "tiff:Software"],
        "comments" => &["exif:UserComment"],
        _ => &[],
    }
}

/// Bearbeitet die Metadaten nach `keep`/`remove`/`tags` (Logik von `metadata.edit`).
pub fn edit_meta(m: &mut Meta, keep: &str, remove: &[String], tags: &[u16]) {
    let has = |c: &str| remove.iter().any(|r| r == c);
    if keep == "none" {
        m.exif = None;
        m.xmp = None;
        m.extra.remove(EXTRA_IPTC);
        m.extra.remove(EXTRA_COMMENT);
        if has("icc") {
            m.icc = None;
        }
        return;
    }
    if has("xmp") {
        m.xmp = None;
    }
    if has("iptc") {
        m.extra.remove(EXTRA_IPTC);
    }
    if has("comments") {
        m.extra.remove(EXTRA_COMMENT);
    }
    if has("icc") {
        m.icc = None;
    }
    // EXIF
    if let Some(raw) = m.exif.as_deref()
        && let Some(mut e) = exif::Exif::parse(raw)
    {
        let mut changed = false;
        let mut drop: Vec<u16> = tags.to_vec();
        if has("camera") {
            drop.extend_from_slice(exif::CAMERA_TAGS);
        }
        if has("software") {
            drop.extend_from_slice(exif::SOFTWARE_TAGS);
        }
        if has("comments") {
            drop.extend_from_slice(exif::COMMENT_TAGS);
        }
        if !drop.is_empty() {
            changed |= e.remove_tags(&drop);
        }
        if has("gps") && e.gps.is_some() {
            e.gps = None;
            changed = true;
        }
        if tags.contains(&exif::TAG_GPS_IFD) && e.gps.is_some() {
            e.gps = None;
            changed = true;
        }
        if has("thumbnails") && e.ifd1.is_some() {
            e.ifd1 = None;
            e.thumbnail = None;
            changed = true;
        }
        if changed {
            m.exif = Some(e.write());
        }
    }
    // XMP
    if let Some(x) = m.xmp.as_ref() {
        let prefixes: Vec<&str> = CATEGORIES
            .iter()
            .filter(|c| has(c))
            .flat_map(|c| xmp_prefixes(c).iter().copied())
            .collect();
        if !prefixes.is_empty() {
            let s = String::from_utf8_lossy(x).into_owned();
            let t = xmp::remove_props(&s, &prefixes);
            if t != s {
                m.xmp = Some(t.into_bytes());
            }
        }
    }
}

/// `metadata.edit`.
struct Edit;

impl Step for Edit {
    fn id(&self) -> &str {
        EDIT
    }
    fn group(&self) -> StepGroup {
        StepGroup::Metadata
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::choice("keep", &["all", "none", "chosen"], "all").beginner(),
            ParamDef::multi("remove", &CATEGORIES, &["gps"])
                .beginner()
                .when("keep", &["all", "chosen"]),
            ParamDef::json("tags", Value::Array(Vec::new())),
        ]
    }
    fn validate(&self, params: &Value) -> Vec<Issue> {
        let mut out = crate::step::validate_params(&self.params(), params);
        if let Some(a) = params.get("tags").filter(|v| !v.is_null()) {
            let ok = a
                .as_array()
                .is_some_and(|a| a.iter().all(|t| exif::tag_from_value(t).is_some()));
            if !ok {
                out.push(Issue::param("tags", fp_i18n::t("batch-meta-issue-tags")));
            }
        }
        out
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, _ctx: &mut StepContext) -> Result<()> {
        let keep = params.get("keep").and_then(Value::as_str).unwrap_or("all");
        let remove = strs(params, "remove");
        let tags: Vec<u16> = params
            .get("tags")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(exif::tag_from_value).collect())
            .unwrap_or_default();
        if let Some(m) = item.meta_mut() {
            edit_meta(m, keep, &remove, &tags);
        }
        Ok(())
    }
    fn summary(&self, params: &Value) -> String {
        let keep = params.get("keep").and_then(Value::as_str).unwrap_or("all");
        if keep == "none" {
            return fp_i18n::t("batch-meta-summary-remove-all");
        }
        let remove = strs(params, "remove");
        if remove.is_empty() {
            return fp_i18n::t("batch-meta-summary-keep-all");
        }
        let list: Vec<String> = remove
            .iter()
            .map(|c| fp_i18n::t(&format!("batch-meta-cat-{c}")))
            .collect();
        fp_i18n::tr!("batch-meta-summary-remove", list = list.join(", "))
    }
}

/// Platzhalter in Texten von `metadata.set`.
pub fn expand(text: &str, ctx: &StepContext) -> String {
    let stem = crate::naming::split_extension(&ctx.info.file_name()).0;
    let now = chrono::Local::now();
    text.replace("{name}", &stem)
        .replace("{n}", &(ctx.index + 1).to_string())
        .replace("{year}", &now.format("%Y").to_string())
        .replace("{date}", &now.format("%Y-%m-%d").to_string())
}

/// Aufnahmedatum: fester Wert (`YYYY-MM-DD HH:MM[:SS]`, `YYYY:MM:DD HH:MM:SS`) oder Verschiebung
/// (`+2h`, `-1d 30m`, Einheiten d/h/m/s). Liefert das neue Datum oder None (ungültig/kein Datum).
pub fn date_value(
    spec: &str,
    current: Option<chrono::NaiveDateTime>,
) -> Option<chrono::NaiveDateTime> {
    let s = spec.trim();
    if s.starts_with('+') || s.starts_with('-') {
        let sign = if s.starts_with('-') { -1 } else { 1 };
        let mut secs: i64 = 0;
        let mut num = String::new();
        for c in s[1..].chars() {
            if c.is_ascii_digit() {
                num.push(c);
            } else if c.is_whitespace() {
                continue;
            } else {
                let n: i64 = num.parse().ok()?;
                num.clear();
                secs += n * match c {
                    'd' => 86400,
                    'h' => 3600,
                    'm' => 60,
                    's' => 1,
                    _ => return None,
                };
            }
        }
        if !num.is_empty() {
            return None;
        }
        return current.map(|d| d + chrono::Duration::seconds(sign * secs));
    }
    [
        "%Y-%m-%d %H:%M:%S",
        "%Y:%m:%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M",
    ]
    .iter()
    .find_map(|f| chrono::NaiveDateTime::parse_from_str(s, f).ok())
    .or_else(|| {
        chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .ok()
            .and_then(|d| d.and_hms_opt(0, 0, 0))
    })
}

/// `metadata.set`.
struct Set;

impl Step for Set {
    fn id(&self) -> &str {
        SET
    }
    fn group(&self) -> StepGroup {
        StepGroup::Metadata
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::text("artist", "").beginner(),
            ParamDef::text("copyright", "").beginner(),
            ParamDef::text("description", ""),
            ParamDef::text("title", ""),
            ParamDef::text("keywords", ""),
            ParamDef::int("rating", -1, 5, -1),
            ParamDef::text("dateTaken", ""),
            ParamDef::choice("gps", &["keep", "remove"], "keep"),
        ]
    }
    fn validate(&self, params: &Value) -> Vec<Issue> {
        let mut out = crate::step::validate_params(&self.params(), params);
        if let Some(d) = params
            .get("dateTaken")
            .and_then(Value::as_str)
            .filter(|d| !d.trim().is_empty())
        {
            let shift = d.trim().starts_with(['+', '-']);
            let probe =
                chrono::NaiveDate::from_ymd_opt(2000, 1, 1).and_then(|d| d.and_hms_opt(0, 0, 0));
            if date_value(d, if shift { probe } else { None }).is_none() {
                out.push(Issue::param(
                    "dateTaken",
                    fp_i18n::t("batch-meta-issue-date"),
                ));
            }
        }
        out
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let text = |k: &str| {
            params
                .get(k)
                .and_then(Value::as_str)
                .map(|s| expand(s.trim(), ctx))
                .filter(|s| !s.is_empty())
        };
        let artist = text("artist");
        let copyright = text("copyright");
        let description = text("description");
        let title = text("title");
        let keywords: Option<Vec<String>> = text("keywords").map(|k| {
            k.split([',', ';'])
                .map(|w| w.trim().to_string())
                .filter(|w| !w.is_empty())
                .collect()
        });
        let rating = params
            .get("rating")
            .and_then(Value::as_i64)
            .filter(|r| *r >= 0);
        let date = params
            .get("dateTaken")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let remove_gps = params.get("gps").and_then(Value::as_str) == Some("remove");
        let Some(m) = item.meta_mut() else {
            return Ok(());
        };
        // EXIF
        let exif_needed = artist.is_some()
            || copyright.is_some()
            || description.is_some()
            || date.is_some()
            || remove_gps;
        if exif_needed {
            let existing = m.exif.as_deref().and_then(exif::Exif::parse);
            if existing.is_some()
                || artist.is_some()
                || copyright.is_some()
                || description.is_some()
                || date.is_some()
            {
                let before = existing.clone();
                let mut e = existing.unwrap_or_else(exif::Exif::new_empty);
                if let Some(a) = &artist {
                    e.set_ascii(exif::TAG_ARTIST, a);
                }
                if let Some(c) = &copyright {
                    e.set_ascii(exif::TAG_COPYRIGHT, c);
                }
                if let Some(d) = &description {
                    e.set_ascii(exif::TAG_DESCRIPTION, d);
                }
                if let Some(spec) = &date {
                    match date_value(spec, e.summary().date_taken) {
                        Some(d) => {
                            let s = d.format("%Y:%m:%d %H:%M:%S").to_string();
                            let mut data = s.into_bytes();
                            data.push(0);
                            let entry = exif::Entry {
                                tag: exif::TAG_DATETIME_ORIGINAL,
                                ty: 2,
                                count: data.len() as u32,
                                data,
                            };
                            e.exif.get_or_insert_with(exif::Ifd::default).set(entry);
                        }
                        None => ctx.warn(fp_i18n::t("batch-meta-warn-no-date")),
                    }
                }
                if remove_gps {
                    e.gps = None;
                }
                if before.as_ref() != Some(&e) {
                    m.exif = Some(e.write());
                }
            }
        }
        // XMP
        let fields = xmp::Fields {
            creator: artist,
            rights: copyright,
            description,
            title,
            keywords,
            rating,
        };
        if fields != xmp::Fields::default() {
            let cur = m
                .xmp
                .as_ref()
                .map(|x| String::from_utf8_lossy(x).into_owned());
            m.xmp = Some(xmp::set_fields(cur.as_deref(), &fields).into_bytes());
        }
        if remove_gps && let Some(x) = m.xmp.as_ref() {
            m.xmp = Some(
                xmp::remove_props(&String::from_utf8_lossy(x), xmp_prefixes("gps")).into_bytes(),
            );
        }
        Ok(())
    }
    fn summary(&self, params: &Value) -> String {
        let mut parts = Vec::new();
        for k in [
            "artist",
            "copyright",
            "description",
            "title",
            "keywords",
            "dateTaken",
        ] {
            if let Some(v) = params
                .get(k)
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
            {
                parts.push(format!(
                    "{}: {}",
                    fp_i18n::t(&crate::step::param_key(SET, k)),
                    v.trim()
                ));
            }
        }
        if params.get("gps").and_then(Value::as_str) == Some("remove") {
            parts.push(fp_i18n::tr!(
                "batch-meta-summary-remove",
                list = fp_i18n::t("batch-meta-cat-gps")
            ));
        }
        parts.join(", ")
    }
}

/// `metadata.orientation`.
struct Orientation;

impl Step for Orientation {
    fn id(&self) -> &str {
        ORIENTATION
    }
    fn group(&self) -> StepGroup {
        StepGroup::Metadata
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::PIXELS
    }
    fn apply(&self, item: &mut Item, _params: &Value, _ctx: &mut StepContext) -> Result<()> {
        // Beim Laden schon angewendet (Tag = 1); hier für Quellen, deren Metadaten ein früherer
        // Schritt gesetzt hat. Zweimal anwenden ändert nichts.
        meta::apply_orientation(item);
        Ok(())
    }
    fn summary(&self, _params: &Value) -> String {
        fp_i18n::t("batch-meta-summary-orientation")
    }
}

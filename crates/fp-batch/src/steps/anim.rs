//! Gruppe „Animation“ (SPEC-BATCH §3.8): Animationen (GIF, APNG, animiertes WebP …) und Videos als
//! Frame-Folge mit Dauern in ms.
//!
//! | Schritt | Wirkung |
//! |---|---|
//! | `video.import` | Video über ffmpeg lesen (Start/Ende, fps, Höchstzahl, Größe), siehe [`crate::video`] |
//! | `anim.trim` | kürzen: Anfang/Ende (Zeit oder Bildnummer), einzelne Frames entfernen |
//! | `anim.speed` | Tempo (Faktor oder feste fps), kurze Frames zusammenfassen (`minDelay`) |
//! | `anim.loop` | Schleife (endlos/einmal/n), rückwärts, Pingpong |
//! | `anim.dropFrames` | ausdünnen: jedes n-te, Ziel-fps, ähnliche zusammenfassen |
//! | `anim.frameDelay` | Bilddauer: alle gleich, erstes/letztes länger |
//! | `anim.toFrames` | Sammel-Schritt: Animation → Bildfolge `{name}_{frame:4}` |
//! | `anim.fromFrames` | Sammel-Schritt: Bildfolge → eine Animation |
//! | `anim.spriteSheet` | Animation → Sprite-Sheet (mit Atlas) bzw. Sheet → Frames |
//!
//! **Zeiten bleiben stimmig**: Wer Frames entfernt oder zusammenfasst, schlägt deren Dauer dem
//! vorigen behaltenen Frame zu (Gesamtdauer gleich); Dauern werden mit Fehlerübertrag gerundet
//! (bei GIF auf 10 ms, weil GIF in Hundertstelsekunden speichert). Ergibt ein Schritt 0 Frames,
//! ist das ein Fehler des Elements (nie eine leere Datei); 1 Frame bleibt eine Animation mit 1 Frame.
//!
//! Für die Zeitleiste der Vorschau (`fp-ui\src\batch\timeline.rs`) gibt es das Modell [`Timeline`].

use crate::error::{BatchError, Result};
use crate::item::{Animation, Frame, Item, ItemInfo, ItemKind, Meta, Still};
use crate::output::OutputFormat;
use crate::step::{
    Issue, ParamDef, Step, StepContext, StepGroup, StepKind, StepRegistry, validate_params,
};
use crate::video::{self, VideoParams, format_time_ms, parse_time_ms};
use fp_raster::{IRect, RgbaImage};
use serde_json::{Value, json};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const VIDEO_IMPORT: &str = "video.import";
pub const TRIM: &str = "anim.trim";
pub const SPEED: &str = "anim.speed";
pub const LOOP: &str = "anim.loop";
pub const DROP_FRAMES: &str = "anim.dropFrames";
pub const FRAME_DELAY: &str = "anim.frameDelay";
pub const TO_FRAMES: &str = "anim.toFrames";
pub const FROM_FRAMES: &str = "anim.fromFrames";
pub const SPRITE_SHEET: &str = "anim.spriteSheet";

/// `Meta::extra`: Grundname der Quelle (ohne Endung), gesetzt vor Sammel-Schritten ([`tag_source`]).
pub const EXTRA_SOURCE_NAME: &str = "batch.sourceName";
/// `Meta::extra`: Grundname der Ausgabe eines Sammel-Ergebnisses ([`collect_target`]).
pub const EXTRA_OUTPUT_STEM: &str = "batch.outputStem";
/// `Meta::extra`: Atlas eines Sprite-Sheets (`{"size": [w, h], "frames": [{x, y, w, h, delayMs}]}`).
pub const EXTRA_ATLAS: &str = "anim.atlas";
/// `Meta::extra`: file format of the atlas written next to the sheet (`json`, `css`, `none`);
/// the executor writes it as an extra output (BA-105).
pub const EXTRA_ATLAS_FORMAT: &str = "anim.atlasFormat";

/// Registriert die Schritte der Gruppe.
pub fn register(r: &mut StepRegistry) {
    r.register(Arc::new(VideoImport));
    r.register(Arc::new(Trim));
    r.register(Arc::new(Speed));
    r.register(Arc::new(Loop));
    r.register(Arc::new(DropFrames));
    r.register(Arc::new(FrameDelay));
    r.register(Arc::new(ToFrames));
    r.register(Arc::new(FromFrames));
    r.register(Arc::new(SpriteSheet));
}

// ------------------------------------------------------------------ Haken für den Kern

/// Merkt den Grundnamen der Quelle am Element (Haken im Executor vor Sammel-Schritten).
pub fn tag_source(item: &mut Item, info: &ItemInfo) {
    let (stem, _) = crate::naming::split_extension(&info.file_name());
    if let Some(m) = item.meta_mut() {
        m.extra.insert(EXTRA_SOURCE_NAME.into(), json!(stem));
    }
}

/// Ziel eines Sammel-Ergebnisses: `dir\<Ausgabe-Grundname>.<ext>`, wenn der Schritt einen Namen
/// gesetzt hat (`anim.toFrames`), sonst `default`.
pub fn collect_target(default: PathBuf, dir: &Path, ext: &str, item: &Item) -> PathBuf {
    match item
        .meta()
        .and_then(|m| m.extra.get(EXTRA_OUTPUT_STEM))
        .and_then(Value::as_str)
    {
        Some(stem) if !stem.is_empty() => dir.join(if ext.is_empty() {
            stem.to_string()
        } else {
            format!("{stem}.{ext}")
        }),
        _ => default,
    }
}

// ------------------------------------------------------------------ Hilfen

fn num(p: &Value, k: &str, d: f64) -> f64 {
    p.get(k)
        .and_then(Value::as_f64)
        .filter(|f| f.is_finite())
        .unwrap_or(d)
}
fn int(p: &Value, k: &str, d: i64) -> i64 {
    p.get(k)
        .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f.round() as i64)))
        .unwrap_or(d)
}
fn flag(p: &Value, k: &str, d: bool) -> bool {
    p.get(k).and_then(Value::as_bool).unwrap_or(d)
}
fn text<'a>(p: &'a Value, k: &str, d: &'a str) -> &'a str {
    p.get(k).and_then(Value::as_str).unwrap_or(d)
}

fn empty_error() -> BatchError {
    BatchError::Invalid(fp_i18n::t("batch-anim-err-empty"))
}

/// Gesamtdauer in ms.
pub fn total_ms(frames: &[Frame]) -> u64 {
    frames.iter().map(|f| f.delay_ms as u64).sum()
}

/// Rundet Dauern (ms, reell) auf Vielfache von `quantum` mit Fehlerübertrag: Die Summe der
/// Ergebnisse weicht höchstens ein halbes `quantum` von der exakten Summe ab.
pub fn quantize_delays(exact: &[f64], quantum: u32) -> Vec<u32> {
    let q = quantum.max(1) as f64;
    let mut acc = 0.0;
    let mut emitted: u64 = 0;
    exact
        .iter()
        .map(|&d| {
            acc += d.max(0.0);
            let target = ((acc / q).round() * q) as u64;
            let v = target.saturating_sub(emitted);
            emitted = emitted.max(target);
            v as u32
        })
        .collect()
}

/// Rundungsschritt der Dauern für ein Ausgabeformat (GIF speichert Hundertstelsekunden).
fn quantum_for(fmt: OutputFormat) -> u32 {
    if fmt == OutputFormat::Gif { 10 } else { 1 }
}

/// Behält Frames nach `keep(i, frame, zuletzt behaltenes)`; die Dauer verworfener Frames geht an das
/// vorige behaltene Frame (Gesamtdauer bleibt). Das erste Frame bleibt immer.
fn merge_frames(
    frames: Vec<Frame>,
    mut keep: impl FnMut(usize, &Frame, &Frame) -> bool,
) -> Vec<Frame> {
    let mut out: Vec<Frame> = Vec::with_capacity(frames.len());
    for (i, f) in frames.into_iter().enumerate() {
        match out.last_mut() {
            Some(last) if !keep(i, &f, last) => {
                last.delay_ms = last.delay_ms.saturating_add(f.delay_ms)
            }
            _ => out.push(f),
        }
    }
    out
}

/// Mittlerer Unterschied zweier Frames in % (0 = gleich, 100 = maximal; ungleiche Größe = 100).
pub fn frame_difference(a: &RgbaImage, b: &RgbaImage) -> f64 {
    if a.width() != b.width() || a.height() != b.height() || a.data().is_empty() {
        return 100.0;
    }
    let sum: u64 = a
        .as_bytes()
        .iter()
        .zip(b.as_bytes())
        .map(|(x, y)| x.abs_diff(*y) as u64)
        .sum();
    sum as f64 * 100.0 / (a.as_bytes().len() as f64 * 255.0)
}

fn anim_mut(item: &mut Item) -> Option<&mut Animation> {
    match item {
        Item::Animation(a) => Some(a),
        _ => None,
    }
}

fn set_frames(a: &mut Animation, frames: Vec<Frame>) -> Result<()> {
    if frames.is_empty() {
        return Err(empty_error());
    }
    a.width = frames[0].image.width() as u32;
    a.height = frames[0].image.height() as u32;
    a.frames = frames;
    Ok(())
}

/// Liest Bildbereiche `1-3, 10, 20-` (1-basiert, `-` am Ende = bis zum letzten, am Anfang = ab
/// dem ersten). Ergebnis: Maske je Frame (`true` = entfernen). Fehler: der ungültige Teil.
pub fn parse_frame_ranges(s: &str, n: usize) -> std::result::Result<Vec<bool>, String> {
    let mut mask = vec![false; n];
    for tok in s.split([',', ';']).flat_map(str::split_whitespace) {
        let parse = |x: &str| x.trim().parse::<usize>().ok().filter(|&v| v >= 1);
        let (a, b) = match tok.split_once('-') {
            Some((a, b)) => {
                let a = if a.trim().is_empty() {
                    Some(1)
                } else {
                    parse(a)
                };
                let b = if b.trim().is_empty() {
                    Some(usize::MAX)
                } else {
                    parse(b)
                };
                (a, b)
            }
            None => (parse(tok), parse(tok)),
        };
        let (Some(a), Some(b)) = (a, b) else {
            return Err(tok.to_string());
        };
        if b < a {
            return Err(tok.to_string());
        }
        for m in mask.iter_mut().take(b.min(n)).skip(a - 1) {
            *m = true;
        }
    }
    Ok(mask)
}

/// Schreibt eine Maske (`true` = entfernt) als Bereiche `1-3, 10` (1-basiert).
pub fn format_frame_ranges(mask: &[bool]) -> String {
    let mut parts = Vec::new();
    let mut i = 0;
    while i < mask.len() {
        if mask[i] {
            let s = i;
            while i + 1 < mask.len() && mask[i + 1] {
                i += 1;
            }
            parts.push(if s == i {
                format!("{}", s + 1)
            } else {
                format!("{}-{}", s + 1, i + 1)
            });
        }
        i += 1;
    }
    parts.join(", ")
}

// ------------------------------------------------------------------ Zeitleiste (Modell)

/// Modell der Zeitleiste in der Vorschau: Frames mit Dauern, Griffe für Anfang/Ende in ms,
/// entfernte Frames. Die Oberfläche (`fp-ui\src\batch\timeline.rs`) zeichnet es und schreibt das
/// Ergebnis über [`Timeline::trim_params`] in den Schritt `anim.trim`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Timeline {
    /// Dauer je Frame in ms.
    pub delays: Vec<u32>,
    /// Anfang (ms, auf eine Frame-Grenze gerastet).
    pub start_ms: u64,
    /// Ende (ms, auf eine Frame-Grenze gerastet).
    pub end_ms: u64,
    /// Entfernte Frames (Index 0-basiert).
    pub removed: Vec<bool>,
}

impl Timeline {
    /// Neue Zeitleiste über die ganze Dauer.
    pub fn new(delays: Vec<u32>) -> Timeline {
        let n = delays.len();
        let total = delays.iter().map(|&d| d as u64).sum();
        Timeline {
            delays,
            start_ms: 0,
            end_ms: total,
            removed: vec![false; n],
        }
    }

    /// Zeitleiste eines Elements (Animation; sonst None).
    pub fn from_item(item: &Item) -> Option<Timeline> {
        match item {
            Item::Animation(a) => {
                Some(Timeline::new(a.frames.iter().map(|f| f.delay_ms).collect()))
            }
            _ => None,
        }
    }

    pub fn len(&self) -> usize {
        self.delays.len()
    }
    pub fn is_empty(&self) -> bool {
        self.delays.is_empty()
    }

    /// Gesamtdauer in ms.
    pub fn total_ms(&self) -> u64 {
        self.delays.iter().map(|&d| d as u64).sum()
    }

    /// Anfangszeit von Frame `i` (bei `i == len` die Gesamtdauer).
    pub fn start_of(&self, i: usize) -> u64 {
        self.delays.iter().take(i).map(|&d| d as u64).sum()
    }

    /// Frame, das zur Zeit `ms` sichtbar ist (am Ende das letzte).
    pub fn frame_at(&self, ms: u64) -> usize {
        let mut t = 0u64;
        for (i, &d) in self.delays.iter().enumerate() {
            t += d as u64;
            if ms < t {
                return i;
            }
        }
        self.delays.len().saturating_sub(1)
    }

    /// Nächste Frame-Grenze zu `ms`.
    pub fn snap(&self, ms: u64) -> u64 {
        let mut best = 0u64;
        let mut t = 0u64;
        for &d in &self.delays {
            t += d as u64;
            if t.abs_diff(ms) < best.abs_diff(ms) {
                best = t;
            }
        }
        best
    }

    /// Setzt den Anfang (gerastet, höchstens bis vor das Ende).
    pub fn set_start(&mut self, ms: u64) {
        let s = self.snap(ms.min(self.total_ms()));
        self.start_ms = if s >= self.end_ms {
            self.start_of(self.frame_at(self.end_ms.saturating_sub(1)))
        } else {
            s
        };
    }

    /// Setzt das Ende (gerastet, mindestens nach dem Anfang).
    pub fn set_end(&mut self, ms: u64) {
        let e = self.snap(ms.min(self.total_ms()));
        self.end_ms = if e <= self.start_ms {
            self.start_of(self.frame_at(self.start_ms) + 1)
        } else {
            e
        };
    }

    /// Schaltet „Frame entfernen“ um.
    pub fn toggle_removed(&mut self, i: usize) {
        if let Some(r) = self.removed.get_mut(i) {
            *r = !*r;
        }
    }

    /// Ist Frame `i` im Ergebnis (im Fenster und nicht entfernt)?
    pub fn is_kept(&self, i: usize) -> bool {
        let (a, b) = (self.start_of(i), self.start_of(i + 1));
        let in_window = if a == b {
            a >= self.start_ms && a < self.end_ms
        } else {
            b.min(self.end_ms) > a.max(self.start_ms)
        };
        in_window && !self.removed.get(i).copied().unwrap_or(false)
    }

    /// Zahl der behaltenen Frames und deren Dauer.
    pub fn kept(&self) -> (usize, u64) {
        let frames = (0..self.len())
            .filter(|&i| self.is_kept(i))
            .collect::<Vec<_>>();
        let ms = frames.iter().map(|&i| {
            let (a, b) = (self.start_of(i), self.start_of(i + 1));
            b.min(self.end_ms).saturating_sub(a.max(self.start_ms))
        });
        (frames.len(), ms.sum())
    }

    /// Parameter für `anim.trim` (Zeit-Einheit; `end` null = bis zum Ende).
    pub fn trim_params(&self) -> Value {
        let total = self.total_ms();
        json!({
            "unit": "time",
            "start": self.start_ms,
            "end": if self.end_ms >= total { Value::Null } else { json!(self.end_ms) },
            "removeFrames": format_frame_ranges(&self.removed),
        })
    }

    /// Übernimmt Parameter von `anim.trim` (Zeit oder Bildnummern).
    pub fn apply_params(&mut self, p: &Value) {
        let n = self.len();
        let total = self.total_ms();
        if text(p, "unit", "time") == "frame" {
            let s = int(p, "start", 1).clamp(1, n.max(1) as i64) as usize;
            let e = match int(p, "end", 0) {
                x if x <= 0 => n,
                x => (x as usize).clamp(s, n.max(1)),
            };
            self.start_ms = self.start_of(s - 1);
            self.end_ms = self.start_of(e);
        } else {
            self.start_ms = p
                .get("start")
                .and_then(parse_time_ms)
                .unwrap_or(0)
                .min(total);
            self.end_ms = p
                .get("end")
                .and_then(parse_time_ms)
                .filter(|&e| e > 0)
                .unwrap_or(total)
                .min(total)
                .max(self.start_ms);
        }
        self.removed =
            parse_frame_ranges(text(p, "removeFrames", ""), n).unwrap_or_else(|_| vec![false; n]);
    }
}

// ------------------------------------------------------------------ video.import

/// `video.import`: Video lesen (über ffmpeg).
struct VideoImport;

impl Step for VideoImport {
    fn id(&self) -> &str {
        VIDEO_IMPORT
    }
    fn group(&self) -> StepGroup {
        StepGroup::Animation
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::time("start", json!(0)).beginner().unit("ms"),
            ParamDef::time("end", Value::Null).beginner().unit("ms"),
            ParamDef::number("fps", 1.0, 60.0, 12.0).beginner(),
            ParamDef::int("maxFrames", 1, 3000, 600),
            ParamDef::int("maxSize", 0, 16384, 0).unit("px"),
            ParamDef::choice("decoder", &["auto", "builtin", "ffmpeg"], "auto"),
        ]
    }
    fn validate(&self, params: &Value) -> Vec<Issue> {
        let mut out = validate_params(&self.params(), params);
        out.extend(check_window(params));
        out
    }
    fn applies_to(&self) -> &[ItemKind] {
        &[ItemKind::Animation, ItemKind::Video]
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let Some(src) = item
            .meta()
            .and_then(|m| m.extra.get(video::EXTRA_SOURCE))
            .and_then(Value::as_str)
            .map(PathBuf::from)
        else {
            ctx.warn(fp_i18n::t("batch-anim-warn-not-video"));
            return Ok(());
        };
        let mut vp = VideoParams::from_json(params);
        if ctx.preview {
            vp.max_frames = vp.max_frames.min(150);
            vp.max_size = if vp.max_size == 0 {
                480
            } else {
                vp.max_size.min(480)
            };
        }
        let used = item
            .meta()
            .and_then(|m| m.extra.get(video::EXTRA_PARAMS))
            .cloned();
        if used.as_ref() == Some(&vp.to_json()) {
            return Ok(());
        }
        if text(params, "decoder", "auto") == "builtin" {
            ctx.warn(fp_i18n::t("batch-anim-warn-no-builtin"));
        }
        *item = video::decode(&src, &vp, ctx.cancel)?;
        Ok(())
    }
    fn summary(&self, params: &Value) -> String {
        let vp = VideoParams::from_json(params);
        let end = vp
            .end_ms
            .map(format_time_ms)
            .unwrap_or_else(|| fp_i18n::t("batch-anim-end-of-clip"));
        fp_i18n::tr!(
            "batch-anim-summary-video",
            start = format_time_ms(vp.start_ms),
            end = end,
            fps = vp.fps
        )
    }
}

/// Prüft `end > start` (gleiche Einheit) und die Frame-Bereiche.
fn check_window(params: &Value) -> Vec<Issue> {
    let mut out = Vec::new();
    let frames = text(params, "unit", "time") == "frame";
    let (s, e) = if frames {
        (
            params
                .get("start")
                .and_then(Value::as_f64)
                .map(|f| f as u64),
            params
                .get("end")
                .and_then(Value::as_f64)
                .map(|f| f as u64)
                .filter(|&e| e > 0),
        )
    } else {
        (
            params.get("start").and_then(parse_time_ms),
            params.get("end").and_then(parse_time_ms).filter(|&e| e > 0),
        )
    };
    for k in ["start", "end"] {
        if let Some(v) = params.get(k).filter(|v| v.is_string())
            && parse_time_ms(v).is_none()
            && !v.as_str().is_some_and(|s| s.trim().is_empty())
        {
            out.push(Issue::param(
                k,
                fp_i18n::tr!("batch-anim-issue-time", param = k.to_string()),
            ));
        }
    }
    if let (Some(s), Some(e)) = (s, e)
        && (if frames { e < s } else { e <= s })
    {
        out.push(Issue::param(
            "end",
            fp_i18n::t("batch-anim-issue-end-before-start"),
        ));
    }
    out
}

// ------------------------------------------------------------------ anim.trim

/// `anim.trim`: kürzen.
struct Trim;

/// Kürzt eine Frame-Folge nach den Parametern von `anim.trim`.
pub fn trim_frames(frames: Vec<Frame>, p: &Value) -> Result<Vec<Frame>> {
    let delays: Vec<u32> = frames.iter().map(|f| f.delay_ms).collect();
    let mut tl = Timeline::new(delays);
    let n = tl.len();
    let removed = parse_frame_ranges(text(p, "removeFrames", ""), n)
        .map_err(|t| BatchError::Invalid(fp_i18n::tr!("batch-anim-issue-ranges", part = t)))?;
    tl.apply_params(p);
    tl.removed = removed;
    let (s, e) = (tl.start_ms, tl.end_ms);
    let mut out = Vec::new();
    for (i, mut f) in frames.into_iter().enumerate() {
        if !tl.is_kept(i) {
            continue;
        }
        let (a, b) = (tl.start_of(i), tl.start_of(i + 1));
        if a != b {
            f.delay_ms = (b.min(e) - a.max(s)) as u32;
        }
        out.push(f);
    }
    if out.is_empty() {
        return Err(empty_error());
    }
    Ok(out)
}

impl Step for Trim {
    fn id(&self) -> &str {
        TRIM
    }
    fn group(&self) -> StepGroup {
        StepGroup::Animation
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::choice("unit", &["time", "frame"], "time").beginner(),
            ParamDef::time("start", json!(0)).beginner(),
            ParamDef::time("end", Value::Null).beginner(),
            ParamDef::text("removeFrames", ""),
        ]
    }
    fn validate(&self, params: &Value) -> Vec<Issue> {
        let mut out = validate_params(&self.params(), params);
        out.extend(check_window(params));
        if let Err(t) = parse_frame_ranges(text(params, "removeFrames", ""), 0) {
            out.push(Issue::param(
                "removeFrames",
                fp_i18n::tr!("batch-anim-issue-ranges", part = t),
            ));
        }
        out
    }
    fn applies_to(&self) -> &[ItemKind] {
        &[ItemKind::Animation]
    }
    fn apply(&self, item: &mut Item, params: &Value, _ctx: &mut StepContext) -> Result<()> {
        let Some(a) = anim_mut(item) else {
            return Ok(());
        };
        let frames = trim_frames(std::mem::take(&mut a.frames), params)?;
        set_frames(a, frames)
    }
    fn summary(&self, params: &Value) -> String {
        if text(params, "unit", "time") == "frame" {
            let end = match int(params, "end", 0) {
                x if x <= 0 => fp_i18n::t("batch-anim-end-of-clip"),
                x => x.to_string(),
            };
            fp_i18n::tr!(
                "batch-anim-summary-trim-frames",
                start = int(params, "start", 1),
                end = end
            )
        } else {
            let end = params
                .get("end")
                .and_then(parse_time_ms)
                .filter(|&e| e > 0)
                .map(format_time_ms)
                .unwrap_or_else(|| fp_i18n::t("batch-anim-end-of-clip"));
            fp_i18n::tr!(
                "batch-anim-summary-trim-time",
                start = format_time_ms(params.get("start").and_then(parse_time_ms).unwrap_or(0)),
                end = end
            )
        }
    }
}

// ------------------------------------------------------------------ anim.speed

/// `anim.speed`: Tempo.
struct Speed;

/// Neue Dauern nach Faktor bzw. festen fps; kurze Frames werden zusammengefasst (`min_delay`),
/// gerundet auf `quantum`.
pub fn speed_frames(
    frames: Vec<Frame>,
    factor: f64,
    fps: f64,
    min_delay: u32,
    quantum: u32,
) -> Vec<Frame> {
    let exact: Vec<f64> = frames
        .iter()
        .map(|f| {
            if fps > 0.0 {
                1000.0 / fps
            } else {
                f.delay_ms as f64 / factor.max(0.01)
            }
        })
        .collect();
    // zusammenfassen in reellen Zeiten (Gesamtdauer bleibt)
    let mut kept: Vec<(Frame, f64)> = Vec::with_capacity(frames.len());
    for (f, d) in frames.into_iter().zip(exact) {
        match kept.last_mut() {
            Some(last) if min_delay > 0 && last.1 < min_delay as f64 => last.1 += d,
            _ => kept.push((f, d)),
        }
    }
    let exact: Vec<f64> = kept.iter().map(|k| k.1).collect();
    let mut delays = quantize_delays(&exact, quantum);
    if let Some(last) = delays.last_mut()
        && min_delay > 0
        && *last < min_delay
    {
        *last = min_delay.div_ceil(quantum.max(1)) * quantum.max(1);
    }
    kept.into_iter()
        .zip(delays)
        .map(|((mut f, _), d)| {
            f.delay_ms = d;
            f
        })
        .collect()
}

impl Step for Speed {
    fn id(&self) -> &str {
        SPEED
    }
    fn group(&self) -> StepGroup {
        StepGroup::Animation
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::number("factor", 0.1, 10.0, 1.0).beginner(),
            ParamDef::number("fps", 0.0, 100.0, 0.0),
            ParamDef::int("minDelay", 0, 1000, 20).unit("ms"),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &[ItemKind::Animation]
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let q = quantum_for(ctx.output_format);
        let Some(a) = anim_mut(item) else {
            return Ok(());
        };
        let frames = speed_frames(
            std::mem::take(&mut a.frames),
            num(params, "factor", 1.0).clamp(0.1, 10.0),
            num(params, "fps", 0.0).clamp(0.0, 100.0),
            int(params, "minDelay", 20).clamp(0, 1000) as u32,
            q,
        );
        set_frames(a, frames)
    }
    fn summary(&self, params: &Value) -> String {
        let fps = num(params, "fps", 0.0);
        if fps > 0.0 {
            fp_i18n::tr!("batch-anim-summary-speed-fps", fps = fps)
        } else {
            fp_i18n::tr!(
                "batch-anim-summary-speed",
                factor = num(params, "factor", 1.0)
            )
        }
    }
}

// ------------------------------------------------------------------ anim.loop

/// `anim.loop`: Schleife, rückwärts, Pingpong.
struct Loop;

/// Rückwärts und/oder Pingpong (vor und zurück, ohne die Endbilder doppelt zu zeigen).
pub fn loop_frames(mut frames: Vec<Frame>, reverse: bool, ping_pong: bool) -> Vec<Frame> {
    if reverse {
        frames.reverse();
    }
    // bei zwei Bildern ist vor-zurück dieselbe Folge
    if ping_pong && frames.len() > 2 {
        let back: Vec<Frame> = frames[1..frames.len() - 1].iter().rev().cloned().collect();
        frames.extend(back);
    }
    frames
}

impl Step for Loop {
    fn id(&self) -> &str {
        LOOP
    }
    fn group(&self) -> StepGroup {
        StepGroup::Animation
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::choice("mode", &["infinite", "once", "count"], "infinite").beginner(),
            ParamDef::int("count", 1, 65535, 3).when("mode", &["count"]),
            ParamDef::bool("pingPong", false).beginner(),
            ParamDef::bool("reverse", false).beginner(),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &[ItemKind::Animation]
    }
    fn apply(&self, item: &mut Item, params: &Value, _ctx: &mut StepContext) -> Result<()> {
        let Some(a) = anim_mut(item) else {
            return Ok(());
        };
        a.loop_count = match text(params, "mode", "infinite") {
            "once" => -1,
            "count" => int(params, "count", 3).clamp(1, 65535) as i32,
            _ => 0,
        };
        let frames = loop_frames(
            std::mem::take(&mut a.frames),
            flag(params, "reverse", false),
            flag(params, "pingPong", false),
        );
        set_frames(a, frames)
    }
    fn summary(&self, params: &Value) -> String {
        let mut parts = vec![match text(params, "mode", "infinite") {
            "once" => fp_i18n::t("batch-anim-loop-once"),
            "count" => fp_i18n::tr!("batch-anim-loop-count", count = int(params, "count", 3)),
            _ => fp_i18n::t("batch-anim-loop-infinite"),
        }];
        if flag(params, "reverse", false) {
            parts.push(fp_i18n::t("batch-param-anim-loop-reverse"));
        }
        if flag(params, "pingPong", false) {
            parts.push(fp_i18n::t("batch-param-anim-loop-pingPong"));
        }
        parts.join(", ")
    }
}

// ------------------------------------------------------------------ anim.dropFrames

/// `anim.dropFrames`: ausdünnen.
struct DropFrames;

/// Jedes `keep_every`-te Frame behalten, auf `target_fps` ausdünnen, ähnliche (Unterschied <
/// `threshold` %) zusammenfassen. Dauern verworfener Frames gehen an das vorige behaltene.
pub fn drop_frames(
    frames: Vec<Frame>,
    keep_every: usize,
    target_fps: f64,
    merge_similar: bool,
    threshold: f64,
) -> Vec<Frame> {
    let n = keep_every.max(1);
    let mut frames = merge_frames(frames, |i, _, _| i % n == 0);
    if target_fps > 0.0 {
        let slot = 1000.0 / target_fps;
        let mut t = 0.0f64; // Anfangszeit des aktuellen Frames
        // das erste Frame (Anfang 0) bleibt immer, das nächste frühestens ab dem zweiten Zeitfenster
        let mut next = slot;
        let starts: Vec<f64> = frames
            .iter()
            .map(|f| {
                let s = t;
                t += f.delay_ms as f64;
                s
            })
            .collect();
        frames = merge_frames(frames, |i, _, _| {
            if starts[i] + 1e-6 >= next {
                next = ((starts[i] / slot).floor() + 1.0) * slot;
                true
            } else {
                false
            }
        });
    }
    if merge_similar {
        frames = merge_frames(frames, |_, f, last| {
            frame_difference(&f.image, &last.image) >= threshold
        });
    }
    frames
}

impl Step for DropFrames {
    fn id(&self) -> &str {
        DROP_FRAMES
    }
    fn group(&self) -> StepGroup {
        StepGroup::Animation
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::int("keepEvery", 1, 100, 1).beginner(),
            ParamDef::number("targetFps", 0.0, 100.0, 0.0).beginner(),
            ParamDef::bool("mergeSimilar", true).beginner(),
            ParamDef::number("threshold", 0.0, 100.0, 0.5)
                .unit("%")
                .when("mergeSimilar", &["true"]),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &[ItemKind::Animation]
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        ctx.check_cancel()?;
        let Some(a) = anim_mut(item) else {
            return Ok(());
        };
        let frames = drop_frames(
            std::mem::take(&mut a.frames),
            int(params, "keepEvery", 1).clamp(1, 100) as usize,
            num(params, "targetFps", 0.0).clamp(0.0, 100.0),
            flag(params, "mergeSimilar", true),
            num(params, "threshold", 0.5).clamp(0.0, 100.0),
        );
        set_frames(a, frames)
    }
    fn summary(&self, params: &Value) -> String {
        let fps = num(params, "targetFps", 0.0);
        if fps > 0.0 {
            fp_i18n::tr!("batch-anim-summary-drop-fps", fps = fps)
        } else {
            fp_i18n::tr!("batch-anim-summary-drop", n = int(params, "keepEvery", 1))
        }
    }
}

// ------------------------------------------------------------------ anim.frameDelay

/// `anim.frameDelay`: Bilddauer.
struct FrameDelay;

impl Step for FrameDelay {
    fn id(&self) -> &str {
        FRAME_DELAY
    }
    fn group(&self) -> StepGroup {
        StepGroup::Animation
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::choice("mode", &["all", "keep"], "all").beginner(),
            ParamDef::int("delay", 0, 60000, 100)
                .beginner()
                .unit("ms")
                .when("mode", &["all"]),
            ParamDef::int("first", 0, 10000, 0).unit("ms"),
            ParamDef::int("last", 0, 10000, 0).unit("ms"),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &[ItemKind::Animation]
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let q = quantum_for(ctx.output_format);
        let Some(a) = anim_mut(item) else {
            return Ok(());
        };
        if text(params, "mode", "all") == "all" {
            let d = int(params, "delay", 100).clamp(0, 60000) as u32;
            a.frames.iter_mut().for_each(|f| f.delay_ms = d);
        }
        let (first, last) = (
            int(params, "first", 0).clamp(0, 10000) as u32,
            int(params, "last", 0).clamp(0, 10000) as u32,
        );
        if first > 0
            && let Some(f) = a.frames.first_mut()
        {
            f.delay_ms = first;
        }
        if last > 0
            && let Some(f) = a.frames.last_mut()
        {
            f.delay_ms = last;
        }
        let exact: Vec<f64> = a.frames.iter().map(|f| f.delay_ms as f64).collect();
        for (f, d) in a.frames.iter_mut().zip(quantize_delays(&exact, q)) {
            f.delay_ms = d;
        }
        Ok(())
    }
    fn summary(&self, params: &Value) -> String {
        if text(params, "mode", "all") == "all" {
            fp_i18n::tr!("batch-anim-summary-delay", ms = int(params, "delay", 100))
        } else {
            String::new()
        }
    }
}

// ------------------------------------------------------------------ anim.toFrames

/// `anim.toFrames`: Animation → Bildfolge (Sammel-Schritt; jedes Frame wird eine Datei).
struct ToFrames;

/// Setzt den Namen eines Frames aus dem Muster (`{name}`, `{frame}`, `{frame:4}`, `{n}`).
pub fn frame_name(pattern: &str, name: &str, frame: usize, n: usize) -> String {
    let re = frame_pattern_regex();
    let s = re.replace_all(pattern, |c: &regex::Captures| {
        let width: usize = c
            .get(2)
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(0)
            .min(9);
        match &c[1] {
            "name" => name.to_string(),
            "frame" => format!("{frame:0width$}"),
            _ => format!("{n:0width$}"),
        }
    });
    let mut hints = Vec::new();
    crate::naming::sanitize_stem(&s, &mut hints)
}

fn frame_pattern_regex() -> &'static regex::Regex {
    static R: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"\{(name|frame|n)(?::(\d+))?\}").expect("pattern regex"))
}

impl Step for ToFrames {
    fn id(&self) -> &str {
        TO_FRAMES
    }
    fn group(&self) -> StepGroup {
        StepGroup::Animation
    }
    fn kind(&self) -> StepKind {
        StepKind::Collect
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::text("pattern", "{name}_{frame:4}").beginner()]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &[ItemKind::Animation, ItemKind::Image, ItemKind::Document]
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
        let pattern = text(params, "pattern", "{name}_{frame:4}");
        let pattern = if pattern.trim().is_empty() {
            "{name}_{frame:4}"
        } else {
            pattern
        };
        let fallback = crate::naming::split_extension(&ctx.info.file_name()).0;
        let mut used: HashSet<String> = HashSet::new();
        let mut out = Vec::new();
        for (k, mut item) in items.into_iter().enumerate() {
            ctx.check_cancel()?;
            item.flatten_document()?;
            let name = item
                .meta()
                .and_then(|m| m.extra.get(EXTRA_SOURCE_NAME))
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| fallback.clone());
            let (frames, meta): (Vec<(RgbaImage, u32)>, Meta) = match item {
                Item::Animation(a) => (
                    a.frames
                        .into_iter()
                        .map(|f| (f.image, f.delay_ms))
                        .collect(),
                    a.meta,
                ),
                Item::Still(s) => (vec![(s.image, 0)], s.meta),
                _ => continue,
            };
            for (i, (image, delay)) in frames.into_iter().enumerate() {
                let mut stem = frame_name(pattern, &name, i + 1, k + 1);
                if !used.insert(stem.to_lowercase()) {
                    let mut c = 2;
                    while !used.insert(format!("{stem} ({c})").to_lowercase()) {
                        c += 1;
                    }
                    stem = format!("{stem} ({c})");
                }
                let mut m = meta.clone();
                m.has_transparency = image.data().iter().any(|p| p[3] < 255);
                m.extra.insert(EXTRA_OUTPUT_STEM.into(), json!(stem));
                m.extra.insert("anim.frameDelayMs".into(), json!(delay));
                out.push(Item::Still(Still { image, meta: m }));
            }
        }
        if out.is_empty() {
            return Err(empty_error());
        }
        Ok(out)
    }
}

// ------------------------------------------------------------------ anim.fromFrames

/// `anim.fromFrames`: Bildfolge → Animation (Sammel-Schritt).
struct FromFrames;

/// Setzt `img` mittig auf eine Fläche `w × h` (größere Bilder werden eingepasst verkleinert).
fn fit_on_canvas(img: &RgbaImage, w: u32, h: u32, bg: [u8; 4]) -> Result<RgbaImage> {
    let (iw, ih) = (img.width() as u32, img.height() as u32);
    if (iw, ih) == (w, h) {
        let mut c = img.clone();
        c.set_origin(0, 0);
        return Ok(c);
    }
    let s = (w as f64 / iw.max(1) as f64)
        .min(h as f64 / ih.max(1) as f64)
        .min(1.0);
    let (nw, nh) = (
        ((iw as f64 * s).round() as u32).max(1),
        ((ih as f64 * s).round() as u32).max(1),
    );
    let mut src = if (nw, nh) == (iw, ih) {
        img.clone()
    } else {
        fp_raster::resample::resize(img, nw, nh, fp_raster::Sampler::Bilinear)
    };
    let mut canvas = RgbaImage::new(IRect::new(0, 0, w as i32, h as i32))
        .map_err(|e| BatchError::Decode(e.to_string()))?;
    canvas.data_mut().iter_mut().for_each(|p| *p = bg);
    src.set_origin(((w - nw) / 2) as i32, ((h - nh) / 2) as i32);
    canvas.paste(&src);
    Ok(canvas)
}

impl Step for FromFrames {
    fn id(&self) -> &str {
        FROM_FRAMES
    }
    fn group(&self) -> StepGroup {
        StepGroup::Animation
    }
    fn kind(&self) -> StepKind {
        StepKind::Collect
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::int("delay", 1, 60000, 100).beginner().unit("ms"),
            ParamDef::choice("size", &["first", "largest", "fixed"], "first").beginner(),
            ParamDef::int("width", 1, 16384, 640)
                .unit("px")
                .when("size", &["fixed"]),
            ParamDef::int("height", 1, 16384, 480)
                .unit("px")
                .when("size", &["fixed"]),
            ParamDef::color("background", "transparent"),
            ParamDef::choice("loop", &["infinite", "once"], "infinite"),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &[ItemKind::Image, ItemKind::Animation, ItemKind::Document]
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
        let delay = int(params, "delay", 100).clamp(1, 60000) as u32;
        let bg = crate::step::parse_color(text(params, "background", "transparent"))
            .unwrap_or([0, 0, 0, 0]);
        let mut meta: Option<Meta> = None;
        let mut src: Vec<(RgbaImage, u32)> = Vec::new();
        for mut item in items {
            ctx.check_cancel()?;
            item.flatten_document()?;
            match item {
                Item::Still(s) => {
                    meta.get_or_insert(s.meta);
                    src.push((s.image, delay));
                }
                Item::Animation(a) => {
                    meta.get_or_insert(a.meta);
                    src.extend(a.frames.into_iter().map(|f| (f.image, f.delay_ms)));
                }
                _ => {}
            }
        }
        let Some((first, _)) = src.first() else {
            return Err(empty_error());
        };
        let (w, h) = match text(params, "size", "first") {
            "largest" => (
                src.iter().map(|f| f.0.width() as u32).max().unwrap_or(1),
                src.iter().map(|f| f.0.height() as u32).max().unwrap_or(1),
            ),
            "fixed" => (
                int(params, "width", 640).clamp(1, 16384) as u32,
                int(params, "height", 480).clamp(1, 16384) as u32,
            ),
            _ => (first.width() as u32, first.height() as u32),
        };
        let mut frames = Vec::with_capacity(src.len());
        for (img, d) in &src {
            ctx.check_cancel()?;
            frames.push(Frame {
                image: fit_on_canvas(img, w, h, bg)?,
                delay_ms: *d,
            });
        }
        let mut meta = meta.unwrap_or_default();
        meta.has_transparency = frames
            .iter()
            .any(|f| f.image.data().iter().any(|p| p[3] < 255));
        meta.extra.remove(EXTRA_SOURCE_NAME);
        meta.extra.remove(video::EXTRA_SOURCE);
        meta.extra.remove(video::EXTRA_PARAMS);
        let loop_count = if text(params, "loop", "infinite") == "once" {
            -1
        } else {
            0
        };
        Ok(vec![Item::Animation(Animation {
            width: w,
            height: h,
            frames,
            loop_count,
            meta,
        })])
    }
    fn summary(&self, params: &Value) -> String {
        fp_i18n::tr!("batch-anim-summary-delay", ms = int(params, "delay", 100))
    }
}

// ------------------------------------------------------------------ anim.spriteSheet

/// `anim.spriteSheet`: Animation ↔ Sprite-Sheet.
struct SpriteSheet;

/// Legt Frames in ein Raster (`columns` 0 = automatisch, quadratisch). Liefert Bild und Atlas.
pub fn frames_to_sheet(
    frames: &[Frame],
    columns: u32,
    spacing: u32,
    padding: u32,
) -> Result<(RgbaImage, Value)> {
    let n = frames.len() as u32;
    if n == 0 {
        return Err(empty_error());
    }
    let (cw, ch) = (
        frames
            .iter()
            .map(|f| f.image.width() as u32)
            .max()
            .unwrap_or(1),
        frames
            .iter()
            .map(|f| f.image.height() as u32)
            .max()
            .unwrap_or(1),
    );
    let cols = if columns == 0 {
        (n as f64).sqrt().ceil() as u32
    } else {
        columns.min(n)
    }
    .max(1);
    let rows = n.div_ceil(cols);
    let w = 2 * padding + cols * cw + (cols - 1) * spacing;
    let h = 2 * padding + rows * ch + (rows - 1) * spacing;
    let mut sheet = RgbaImage::new(IRect::new(0, 0, w as i32, h as i32))
        .map_err(|e| BatchError::Decode(e.to_string()))?;
    let mut atlas = Vec::new();
    for (i, f) in frames.iter().enumerate() {
        let (c, r) = (i as u32 % cols, i as u32 / cols);
        let (x, y) = (padding + c * (cw + spacing), padding + r * (ch + spacing));
        let mut img = f.image.clone();
        img.set_origin(x as i32, y as i32);
        sheet.paste(&img);
        atlas.push(json!({"x": x, "y": y, "w": f.image.width(), "h": f.image.height(), "delayMs": f.delay_ms}));
    }
    Ok((
        sheet,
        json!({"size": [w, h], "columns": cols, "rows": rows, "frames": atlas}),
    ))
}

/// Zerlegt ein Sheet in `columns × rows` Frames (`count` 0 = alle).
pub fn sheet_to_frames(
    sheet: &RgbaImage,
    columns: u32,
    rows: u32,
    spacing: u32,
    padding: u32,
    count: u32,
    delay: u32,
) -> Result<Vec<Frame>> {
    let (cols, rows) = (columns.max(1), rows.max(1));
    let (w, h) = (sheet.width() as i64, sheet.height() as i64);
    let cw = (w - 2 * padding as i64 - (cols as i64 - 1) * spacing as i64) / cols as i64;
    let ch = (h - 2 * padding as i64 - (rows as i64 - 1) * spacing as i64) / rows as i64;
    if cw <= 0 || ch <= 0 {
        return Err(BatchError::Invalid(fp_i18n::t("batch-anim-err-sheet-grid")));
    }
    let total = cols * rows;
    let n = if count == 0 { total } else { count.min(total) };
    let (ox, oy) = sheet.origin();
    let mut out = Vec::with_capacity(n as usize);
    for i in 0..n {
        let (c, r) = (i % cols, i / cols);
        let x = ox + padding as i32 + (c as i64 * (cw + spacing as i64)) as i32;
        let y = oy + padding as i32 + (r as i64 * (ch + spacing as i64)) as i32;
        let mut img = sheet
            .copy_rect(IRect::new(x, y, x + cw as i32, y + ch as i32))
            .map_err(|e| BatchError::Decode(e.to_string()))?;
        img.set_origin(0, 0);
        out.push(Frame {
            image: img,
            delay_ms: delay,
        });
    }
    Ok(out)
}

impl Step for SpriteSheet {
    fn id(&self) -> &str {
        SPRITE_SHEET
    }
    fn group(&self) -> StepGroup {
        StepGroup::Animation
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::choice("direction", &["toSheet", "fromSheet"], "toSheet").beginner(),
            ParamDef::int("columns", 0, 256, 0).beginner(),
            ParamDef::int("rows", 1, 256, 1),
            ParamDef::int("spacing", 0, 512, 0).unit("px"),
            ParamDef::int("padding", 0, 512, 0).unit("px"),
            ParamDef::int("count", 0, 65536, 0),
            ParamDef::int("delay", 1, 60000, 100).unit("ms"),
            ParamDef::choice("atlas", &["json", "css", "none"], "json")
                .when("direction", &["toSheet"]),
        ]
    }
    fn applies_to(&self) -> &[ItemKind] {
        &[ItemKind::Animation, ItemKind::Image, ItemKind::Document]
    }
    fn apply(&self, item: &mut Item, params: &Value, _ctx: &mut StepContext) -> Result<()> {
        item.flatten_document()?;
        let (spacing, padding) = (
            int(params, "spacing", 0).clamp(0, 512) as u32,
            int(params, "padding", 0).clamp(0, 512) as u32,
        );
        if text(params, "direction", "toSheet") == "fromSheet" {
            let Item::Still(s) = item else { return Ok(()) };
            let cols = int(params, "columns", 0).clamp(0, 256) as u32;
            let cols = if cols == 0 { 1 } else { cols };
            let frames = sheet_to_frames(
                &s.image,
                cols,
                int(params, "rows", 1).clamp(1, 256) as u32,
                spacing,
                padding,
                int(params, "count", 0).clamp(0, 65536) as u32,
                int(params, "delay", 100).clamp(1, 60000) as u32,
            )?;
            let mut meta = s.meta.clone();
            meta.extra.remove(EXTRA_ATLAS);
            meta.extra.remove(EXTRA_ATLAS_FORMAT);
            let (w, h) = (
                frames[0].image.width() as u32,
                frames[0].image.height() as u32,
            );
            *item = Item::Animation(Animation {
                width: w,
                height: h,
                frames,
                loop_count: 0,
                meta,
            });
            return Ok(());
        }
        let (frames, meta) = match item {
            Item::Animation(a) => (std::mem::take(&mut a.frames), a.meta.clone()),
            Item::Still(s) => (
                vec![Frame {
                    image: s.image.clone(),
                    delay_ms: 0,
                }],
                s.meta.clone(),
            ),
            _ => return Ok(()),
        };
        let (sheet, atlas) = frames_to_sheet(
            &frames,
            int(params, "columns", 0).clamp(0, 256) as u32,
            spacing,
            padding,
        )?;
        let mut meta = meta;
        meta.has_transparency = sheet.data().iter().any(|p| p[3] < 255);
        meta.extra.insert(EXTRA_ATLAS.into(), atlas);
        meta.extra.insert(
            EXTRA_ATLAS_FORMAT.into(),
            json!(text(params, "atlas", "json")),
        );
        *item = Item::Still(Still { image: sheet, meta });
        Ok(())
    }
}

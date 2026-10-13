//! Videos als Frame-Folge (SPEC-BATCH §3.8 `video.import`, §5.5 Weg A).
//!
//! Firepixel liefert **keinen** Video-Decoder mit. Videos werden über ein vom Nutzer installiertes
//! ffmpeg gelesen ([`ffmpeg`]): als eigener Prozess (nicht gelinkt), Argumente als Liste, keine
//! Shell, kein Konsolenfenster, Bilder als rohes RGBA über eine Pipe. Fehlt ffmpeg, meldet das
//! Laden „ffmpeg nicht gefunden – Pfad in den Einstellungen wählen“.
//!
//! Ablauf: `item::load` ruft für Videos [`load`] (Vorgaben aus [`VideoParams::default`]); der Schritt
//! `video.import` (`steps::anim`) liest das Video mit seinen Parametern neu, wenn sie von den beim
//! Laden benutzten abweichen. Das Ergebnis ist eine gewöhnliche [`Animation`] (Vollbilder mit Dauer
//! in ms), auf die alle Animations- und Bildschritte wirken.

pub mod ffmpeg;

use crate::error::{BatchError, Result};
use crate::executor::CancelToken;
use crate::item::{Animation, Frame, Item, ItemInfo, Meta};
use serde_json::{json, Value};
use std::path::Path;

/// `Meta::extra`: Pfad der Videoquelle (gesetzt beim Laden eines Videos).
pub const EXTRA_SOURCE: &str = "video.source";
/// `Meta::extra`: beim Lesen benutzte Parameter ([`VideoParams::to_json`]).
pub const EXTRA_PARAMS: &str = "video.params";
/// `Meta::extra`: Kopfdaten des Videos ([`VideoInfo::to_json`]).
pub const EXTRA_INFO: &str = "video.info";

/// Speichergrenze für die Frames eines Videos (Vorgabe 1 GiB); größere Videos werden beim Lesen
/// verkleinert, solange `maxSize` nicht fest vorgegeben ist.
pub const DEFAULT_MEMORY_CAP: u64 = 1 << 30;

/// Parameter für das Lesen eines Videos (`video.import`).
#[derive(Clone, Debug, PartialEq)]
pub struct VideoParams {
    /// Anfang in ms.
    pub start_ms: u64,
    /// Ende in ms (None = bis zum Ende).
    pub end_ms: Option<u64>,
    /// Bilder je Sekunde (1…60).
    pub fps: f64,
    /// Höchstzahl der Frames (1…3000).
    pub max_frames: u32,
    /// Längste Seite in Pixeln (0 = Quelle, durch die Speichergrenze begrenzt).
    pub max_size: u32,
    /// Speichergrenze in Bytes für alle Frames zusammen.
    pub memory_cap: u64,
}

impl Default for VideoParams {
    fn default() -> Self {
        VideoParams { start_ms: 0, end_ms: None, fps: 12.0, max_frames: 600, max_size: 0, memory_cap: DEFAULT_MEMORY_CAP }
    }
}

impl VideoParams {
    /// Aus den Schrittparametern (`start`, `end`, `fps`, `maxFrames`, `maxSize`).
    pub fn from_json(p: &Value) -> VideoParams {
        let d = VideoParams::default();
        let num = |k: &str| p.get(k).and_then(|v| v.as_f64());
        VideoParams {
            start_ms: p.get("start").and_then(parse_time_ms).unwrap_or(0),
            end_ms: p.get("end").and_then(parse_time_ms).filter(|&e| e > 0),
            fps: num("fps").filter(|f| f.is_finite() && *f > 0.0).unwrap_or(d.fps).clamp(0.1, 60.0),
            max_frames: num("maxFrames").map(|f| f.round().clamp(1.0, 3000.0) as u32).unwrap_or(d.max_frames),
            max_size: num("maxSize").map(|f| f.round().clamp(0.0, 16384.0) as u32).unwrap_or(0),
            memory_cap: d.memory_cap,
        }
    }

    /// Als JSON (für `Meta::extra`, Vergleich).
    pub fn to_json(&self) -> Value {
        json!({"start": self.start_ms, "end": self.end_ms, "fps": self.fps, "maxFrames": self.max_frames, "maxSize": self.max_size})
    }

    /// Zeitfenster [Anfang, Ende) in ms; Ende = Dauer bzw. Höchstzahl der Frames.
    pub fn window(&self, duration_ms: Option<u64>) -> (u64, u64) {
        let by_frames = self.start_ms + (self.max_frames as f64 * 1000.0 / self.fps).ceil() as u64;
        let mut end = self.end_ms.unwrap_or(u64::MAX).min(by_frames);
        if let Some(d) = duration_ms {
            end = end.min(d);
        }
        (self.start_ms, end.max(self.start_ms))
    }

    /// Zahl der Frames im Fenster (höchstens `max_frames`).
    pub fn frame_count(&self, duration_ms: Option<u64>) -> u32 {
        let (s, e) = self.window(duration_ms);
        let n = ((e - s) as f64 * self.fps / 1000.0 - 1e-6).ceil().max(0.0) as u64;
        n.min(self.max_frames as u64) as u32
    }

    /// Ausgabegröße: Quelle, auf `max_size` (längste Seite) und die Speichergrenze verkleinert;
    /// nie vergrößert, mindestens 1 × 1.
    pub fn output_size(&self, src_w: u32, src_h: u32, frames: u32) -> (u32, u32) {
        let (w, h) = (src_w.max(1) as f64, src_h.max(1) as f64);
        let mut s: f64 = 1.0;
        if self.max_size > 0 {
            s = s.min(self.max_size as f64 / w.max(h));
        }
        let bytes = w * h * 4.0 * frames.max(1) as f64;
        if bytes > self.memory_cap as f64 {
            s = s.min((self.memory_cap as f64 / bytes).sqrt());
        }
        let s = s.min(1.0);
        (((w * s).round() as u32).max(1), ((h * s).round() as u32).max(1))
    }
}

/// Liest eine Zeitangabe in ms: Zahl (ms), `"1500"`, `"1500ms"`, `"2.5s"`, `"m:ss(.mmm)"`,
/// `"h:mm:ss(.mmm)"`. Leer/ungültig → None.
pub fn parse_time_ms(v: &Value) -> Option<u64> {
    match v {
        Value::Number(n) => n.as_f64().filter(|f| f.is_finite() && *f >= 0.0).map(|f| f.round() as u64),
        Value::String(s) => parse_time_str(s),
        _ => None,
    }
}

fn parse_time_str(s: &str) -> Option<u64> {
    let s = s.trim().replace(',', ".");
    if s.is_empty() {
        return None;
    }
    if let Some(ms) = s.strip_suffix("ms") {
        return ms.trim().parse::<f64>().ok().filter(|f| *f >= 0.0).map(|f| f.round() as u64);
    }
    if let Some(sec) = s.strip_suffix('s') {
        return sec.trim().parse::<f64>().ok().filter(|f| *f >= 0.0).map(|f| (f * 1000.0).round() as u64);
    }
    if s.contains(':') {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() > 3 {
            return None;
        }
        let mut total = 0.0;
        for (i, p) in parts.iter().enumerate() {
            let v: f64 = p.trim().parse().ok().filter(|f: &f64| *f >= 0.0)?;
            if i + 1 < parts.len() && v.fract() != 0.0 {
                return None;
            }
            total = total * 60.0 + v;
        }
        return Some((total * 1000.0).round() as u64);
    }
    s.parse::<f64>().ok().filter(|f| *f >= 0.0).map(|f| f.round() as u64)
}

/// Zeit als `m:ss.mmm` (bzw. `h:mm:ss.mmm`) für Anzeigen.
pub fn format_time_ms(ms: u64) -> String {
    let (h, m, s, r) = (ms / 3_600_000, (ms / 60_000) % 60, (ms / 1000) % 60, ms % 1000);
    if h > 0 { format!("{h}:{m:02}:{s:02}.{r:03}") } else { format!("{m}:{s:02}.{r:03}") }
}

/// Dauern von `n` Frames bei `fps`, ganzzahlig in ms mit Fehlerübertrag (Summe = n · 1000 / fps,
/// gerundet), damit die Gesamtzeit stimmt.
pub fn frame_delays(n: usize, fps: f64) -> Vec<u32> {
    let step = 1000.0 / fps.max(0.001);
    let mut out = Vec::with_capacity(n);
    let mut emitted: u64 = 0;
    for i in 0..n {
        let target = ((i + 1) as f64 * step).round() as u64;
        out.push((target - emitted) as u32);
        emitted = target;
    }
    out
}

/// Kopfdaten eines Videos.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VideoInfo {
    /// Dauer in ms (None = unbekannt, z. B. Livestream).
    pub duration_ms: Option<u64>,
    /// Angezeigte Größe (Drehung schon berücksichtigt).
    pub width: u32,
    pub height: u32,
    /// Bildrate der Quelle.
    pub fps: Option<f64>,
    /// Codec (`h264`, `hevc`, `vp9`, `av1` …).
    pub codec: String,
}

impl VideoInfo {
    pub fn to_json(&self) -> Value {
        json!({"durationMs": self.duration_ms, "width": self.width, "height": self.height, "fps": self.fps, "codec": self.codec})
    }
}

/// Quelle für Videobilder (SPEC §5.1 `trait VideoSource`). P1: nur [`ffmpeg::Ffmpeg`]; eingebaute
/// Decoder (BATCH-10) setzen dieselbe Schnittstelle um.
pub trait VideoSource {
    /// Kopfdaten lesen.
    fn info(&self, path: &Path) -> Result<VideoInfo>;
    /// Frames im Fenster lesen (abbrechbar; beendet einen Hilfsprozess sauber).
    fn frames(&self, path: &Path, info: &VideoInfo, params: &VideoParams, cancel: &CancelToken) -> Result<Vec<Frame>>;
}

/// Liest ein Video als Animation mit der gefundenen Quelle (ffmpeg).
pub fn decode(path: &Path, params: &VideoParams, cancel: &CancelToken) -> Result<Item> {
    let ff = ffmpeg::find()?;
    decode_with(&ff, path, params, cancel)
}

/// Wie [`decode`], mit einer bestimmten Quelle.
pub fn decode_with(src: &dyn VideoSource, path: &Path, params: &VideoParams, cancel: &CancelToken) -> Result<Item> {
    cancel.check()?;
    let info = src.info(path)?;
    cancel.check()?;
    let frames = src.frames(path, &info, params, cancel)?;
    if frames.is_empty() {
        return Err(BatchError::Decode(fp_i18n::tr!("batch-anim-err-video-no-frames", path = path.display().to_string())));
    }
    let (width, height) = (frames[0].image.width() as u32, frames[0].image.height() as u32);
    let mut meta = Meta { source_format: "video".into(), ..Meta::default() };
    meta.extra.insert(EXTRA_SOURCE.into(), json!(path.to_string_lossy()));
    meta.extra.insert(EXTRA_PARAMS.into(), params.to_json());
    meta.extra.insert(EXTRA_INFO.into(), info.to_json());
    Ok(Item::Animation(Animation { width, height, frames, loop_count: 0, meta }))
}

/// Haken aus `item::load` für Videos: liest mit den Vorgaben (ohne Abbrechen von außen).
pub fn load(info: &ItemInfo) -> Result<Item> {
    decode(&info.path, &VideoParams::default(), &CancelToken::new())
}

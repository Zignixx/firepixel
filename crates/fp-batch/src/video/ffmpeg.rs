//! ffmpeg des Nutzers als externes Programm (SPEC-BATCH §5.5 Weg A).
//!
//! - **Suche** ([`find`]): Einstellung „ffmpeg-Datei…“ (`<Profil>\video.json`, Schlüssel
//!   `ffmpegPath`), Umgebungsvariable `FIREPIXEL_FFMPEG`, `PATH`, Ordner der laufenden exe.
//!   Nichts wird heruntergeladen oder mitgeliefert.
//! - **Version** ([`Ffmpeg::version`]) und Zustand für die Einstellungen ([`status`]).
//! - **Lesen**: `ffmpeg -ss … -i in -t … -vf fps=…,scale=… -f rawvideo -pix_fmt rgba pipe:1`;
//!   Bilder als rohes RGBA über die Pipe, Kopfdaten aus der Ausgabe von `ffmpeg -i`.
//! - Prozess: Argumente als Liste (keine Shell), `fp_i18n::profile::child_env`, kein
//!   Konsolenfenster (`CREATE_NO_WINDOW`), stdin leer. Abbrechen beendet den Prozess (`kill` +
//!   `wait`), auch wenn er gerade keine Daten liefert; ein Wächter räumt bei jedem Fehler auf.

use super::{frame_delays, VideoInfo, VideoParams, VideoSource};
use crate::error::{BatchError, Result};
use crate::executor::CancelToken;
use crate::item::Frame;
use fp_raster::{IRect, RgbaImage};
use serde_json::{json, Value};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

/// Umgebungsvariable mit dem Pfad zu ffmpeg.
pub const ENV_VAR: &str = "FIREPIXEL_FFMPEG";
/// Einstellungsdatei im Profil (technisch, englisch).
pub const SETTINGS_FILE: &str = "video.json";
/// Schlüssel in der Einstellungsdatei.
pub const SETTINGS_KEY: &str = "ffmpegPath";

/// Woher der Pfad stammt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoundBy {
    /// Vom Programm gesetzt ([`set_override`], Tests/Einbettung).
    Override,
    /// Einstellung „ffmpeg-Datei…“.
    Setting,
    /// `FIREPIXEL_FFMPEG`.
    Environment,
    /// Suchpfad `PATH`.
    SearchPath,
    /// Neben der laufenden exe.
    NextToExe,
}

impl FoundBy {
    /// Technischer Name (JSON).
    pub fn name(self) -> &'static str {
        match self {
            FoundBy::Override => "override",
            FoundBy::Setting => "setting",
            FoundBy::Environment => "environment",
            FoundBy::SearchPath => "path",
            FoundBy::NextToExe => "nextToExe",
        }
    }
}

/// Ein gefundenes ffmpeg.
#[derive(Clone, Debug, PartialEq)]
pub struct Ffmpeg {
    pub path: PathBuf,
    pub found_by: FoundBy,
}

fn overrides() -> &'static RwLock<Option<PathBuf>> {
    static O: OnceLock<RwLock<Option<PathBuf>>> = OnceLock::new();
    O.get_or_init(|| RwLock::new(None))
}

/// Setzt einen festen Pfad für diesen Prozess (vor allen anderen Quellen; `None` hebt ihn auf).
pub fn set_override(path: Option<PathBuf>) {
    if let Ok(mut o) = overrides().write() {
        *o = path;
    }
}

/// Pfad der Einstellungsdatei.
pub fn settings_path() -> Option<PathBuf> {
    fp_i18n::profile::roaming_dir().map(|d| d.join(SETTINGS_FILE))
}

/// In den Einstellungen gewählte ffmpeg-Datei.
pub fn configured_path() -> Option<PathBuf> {
    let p = settings_path()?;
    let text = std::fs::read_to_string(p).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    v.get(SETTINGS_KEY).and_then(Value::as_str).filter(|s| !s.trim().is_empty()).map(PathBuf::from)
}

/// Speichert die gewählte ffmpeg-Datei (`None` = Einstellung entfernen, wieder automatisch suchen).
/// Andere Schlüssel der Datei bleiben erhalten.
pub fn set_configured_path(path: Option<&Path>) -> Result<()> {
    let Some(file) = settings_path() else { return Err(BatchError::Io(fp_i18n::t("batch-anim-err-no-profile"))) };
    let mut v: Value = std::fs::read_to_string(&file).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_else(|| json!({}));
    if !v.is_object() {
        v = json!({});
    }
    match path {
        Some(p) => v[SETTINGS_KEY] = json!(p.to_string_lossy()),
        None => {
            if let Some(o) = v.as_object_mut() {
                o.remove(SETTINGS_KEY);
            }
        }
    }
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| BatchError::Io(format!("{}: {e}", dir.display())))?;
    }
    let text = serde_json::to_string_pretty(&v).unwrap_or_default();
    std::fs::write(&file, text).map_err(|e| BatchError::Io(format!("{}: {e}", file.display())))
}

fn exe_name() -> &'static str {
    if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" }
}

/// Macht aus einer Angabe (Datei oder Ordner) einen Pfad zur ausführbaren Datei, falls vorhanden.
fn resolve_candidate(p: &Path) -> Option<PathBuf> {
    if p.is_file() {
        return Some(p.to_path_buf());
    }
    if p.is_dir() {
        for c in [p.join(exe_name()), p.join("bin").join(exe_name())] {
            if c.is_file() {
                return Some(c);
            }
        }
    }
    None
}

/// Sucht ffmpeg (Reihenfolge: festgelegt, Einstellung, `FIREPIXEL_FFMPEG`, `PATH`, neben der exe).
/// Ist eine Einstellung oder Umgebungsvariable gesetzt, aber ungültig, wird weitergesucht.
pub fn find() -> Result<Ffmpeg> {
    find_quiet().ok_or_else(missing_error)
}

fn find_quiet() -> Option<Ffmpeg> {
    let ov = overrides().read().ok().and_then(|o| o.clone());
    if let Some(p) = ov.as_deref().and_then(resolve_candidate) {
        return Some(Ffmpeg { path: p, found_by: FoundBy::Override });
    }
    if let Some(p) = configured_path().as_deref().and_then(resolve_candidate) {
        return Some(Ffmpeg { path: p, found_by: FoundBy::Setting });
    }
    if let Some(p) = std::env::var_os(ENV_VAR).map(PathBuf::from).as_deref().and_then(resolve_candidate) {
        return Some(Ffmpeg { path: p, found_by: FoundBy::Environment });
    }
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            let c = dir.join(exe_name());
            if c.is_file() {
                return Some(Ffmpeg { path: c, found_by: FoundBy::SearchPath });
            }
        }
    }
    let exe_dir = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf));
    if let Some(c) = exe_dir.map(|d| d.join(exe_name())).filter(|c| c.is_file()) {
        return Some(Ffmpeg { path: c, found_by: FoundBy::NextToExe });
    }
    None
}

/// Fehler „ffmpeg nicht gefunden – Pfad in den Einstellungen wählen“.
pub fn missing_error() -> BatchError {
    BatchError::Unsupported(fp_i18n::t("batch-anim-err-ffmpeg-missing"))
}

/// Zustand für die Einstellungen bzw. die Videozeile der Oberfläche.
#[derive(Clone, Debug, PartialEq)]
pub struct FfmpegStatus {
    pub path: Option<PathBuf>,
    pub found_by: Option<FoundBy>,
    /// Erste Zeile von `ffmpeg -version` (None = nicht gefunden bzw. nicht startbar).
    pub version: Option<String>,
    /// Klartext in der aktuellen Sprache („ffmpeg 7.1 gefunden …“ bzw. Fehlermeldung).
    pub message: String,
}

impl FfmpegStatus {
    pub fn is_ok(&self) -> bool {
        self.version.is_some()
    }
    pub fn to_json(&self) -> Value {
        json!({
            "path": self.path.as_ref().map(|p| p.to_string_lossy().to_string()),
            "foundBy": self.found_by.map(FoundBy::name),
            "version": self.version,
            "ok": self.is_ok(),
            "message": self.message,
        })
    }
}

/// Sucht ffmpeg und fragt die Version ab.
pub fn status() -> FfmpegStatus {
    match find_quiet() {
        None => FfmpegStatus { path: None, found_by: None, version: None, message: fp_i18n::t("batch-anim-err-ffmpeg-missing") },
        Some(f) => match f.version() {
            Ok(v) => FfmpegStatus { message: fp_i18n::tr!("batch-anim-ffmpeg-found", version = v.clone(), path = f.path.display().to_string()), path: Some(f.path), found_by: Some(f.found_by), version: Some(v) },
            Err(e) => FfmpegStatus { path: Some(f.path), found_by: Some(f.found_by), version: None, message: e.to_string() },
        },
    }
}

/// Versionsnummer aus der ersten Zeile von `ffmpeg -version` (`ffmpeg version 7.1.1-full …` → `7.1.1-full`).
pub fn parse_version(stdout: &str) -> Option<String> {
    let line = stdout.lines().find(|l| !l.trim().is_empty())?;
    let rest = line.trim().strip_prefix("ffmpeg version ")?;
    rest.split_whitespace().next().map(str::to_string)
}

fn secs(v: &str) -> Option<f64> {
    v.trim().parse::<f64>().ok()
}

/// Kopfdaten aus der Ausgabe von `ffmpeg -hide_banner -i <datei>` (stderr).
pub fn parse_probe(stderr: &str) -> Option<VideoInfo> {
    let mut info = VideoInfo::default();
    let mut rotation = 0i32;
    let mut have_video = false;
    for line in stderr.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix("Duration:") {
            let t = rest.split(',').next().unwrap_or("").trim();
            let parts: Vec<&str> = t.split(':').collect();
            if parts.len() == 3
                && let (Some(h), Some(m), Some(s)) = (secs(parts[0]), secs(parts[1]), secs(parts[2]))
            {
                info.duration_ms = Some(((h * 3600.0 + m * 60.0 + s) * 1000.0).round() as u64);
            }
        } else if !have_video && l.starts_with("Stream #") && l.contains(": Video: ") {
            have_video = true;
            let after = l.split(": Video: ").nth(1).unwrap_or("");
            info.codec = after.split([' ', ',']).next().unwrap_or("").to_string();
            let re = size_regex();
            if let Some(c) = re.captures(after) {
                info.width = c[1].parse().unwrap_or(0);
                info.height = c[2].parse().unwrap_or(0);
            }
            for part in after.split(',') {
                let p = part.trim();
                if let Some(f) = p.strip_suffix(" fps").and_then(secs) {
                    info.fps = Some(f);
                } else if info.fps.is_none()
                    && let Some(f) = p.strip_suffix(" tbr").and_then(secs)
                {
                    info.fps = Some(f);
                }
            }
        } else if have_video {
            if let Some(r) = l.strip_prefix("rotate").and_then(|r| r.trim_start().strip_prefix(':')) {
                rotation = r.trim().parse::<f64>().map(|f| f.round() as i32).unwrap_or(rotation);
            } else if let Some(i) = l.find("rotation of ") {
                let r = &l[i + "rotation of ".len()..];
                rotation = r.split_whitespace().next().and_then(secs).map(|f| f.round() as i32).unwrap_or(rotation);
            }
        }
    }
    if !have_video || info.width == 0 || info.height == 0 {
        return None;
    }
    if rotation.rem_euclid(180) == 90 {
        std::mem::swap(&mut info.width, &mut info.height);
    }
    Some(info)
}

fn size_regex() -> &'static regex::Regex {
    static R: OnceLock<regex::Regex> = OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"(?:^|[ ,(])(\d{2,5})x(\d{2,5})(?:$|[ ,\[])").expect("size regex"))
}

/// Argumente zum Lesen der Frames (ohne Programmname).
pub fn frame_args(input: &Path, params: &VideoParams, info: &VideoInfo, out_w: u32, out_h: u32, frames: u32) -> Vec<String> {
    let (start, end) = params.window(info.duration_ms);
    let mut a: Vec<String> = vec!["-hide_banner".into(), "-nostdin".into(), "-loglevel".into(), "error".into()];
    if start > 0 {
        a.push("-ss".into());
        a.push(format!("{:.3}", start as f64 / 1000.0));
    }
    a.push("-i".into());
    a.push(input.to_string_lossy().to_string());
    if end > start && end != u64::MAX {
        a.push("-t".into());
        a.push(format!("{:.3}", (end - start) as f64 / 1000.0));
    }
    for x in ["-an", "-sn", "-dn"] {
        a.push(x.into());
    }
    a.push("-vf".into());
    a.push(format!("fps={},scale={out_w}:{out_h}:flags=bicubic", trim_float(params.fps)));
    a.push("-frames:v".into());
    a.push(frames.to_string());
    for x in ["-f", "rawvideo", "-pix_fmt", "rgba", "pipe:1"] {
        a.push(x.into());
    }
    a
}

fn trim_float(f: f64) -> String {
    let s = format!("{f:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Wächter: beendet den Kindprozess beim Verlassen (Abbrechen, Fehler, Panik).
struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Ok(None) = self.0.try_wait() {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}

impl Ffmpeg {
    /// ffmpeg unter einem festen Pfad (ohne Suche).
    pub fn at(path: impl Into<PathBuf>) -> Ffmpeg {
        Ffmpeg { path: path.into(), found_by: FoundBy::Override }
    }

    /// Befehl mit Testprofil-Umgebung, ohne Konsolenfenster, stdin leer.
    pub fn command(&self) -> Command {
        let mut cmd = Command::new(&self.path);
        fp_i18n::profile::child_env(&mut cmd);
        cmd.stdin(Stdio::null());
        hide_console(&mut cmd);
        cmd
    }

    fn start_error(&self, e: std::io::Error) -> BatchError {
        BatchError::Io(fp_i18n::tr!("batch-anim-err-ffmpeg-start", path = self.path.display().to_string(), error = e.to_string()))
    }

    /// Startet mit Argumenten und sammelt stdout/stderr (höchstens `timeout`, dann beendet).
    fn run_capture(&self, args: &[String], timeout: Duration) -> Result<(Option<i32>, String, String)> {
        let mut cmd = self.command();
        cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
        let child = cmd.spawn().map_err(|e| self.start_error(e))?;
        let mut guard = ChildGuard(child);
        let out = guard.0.stdout.take().map(spawn_reader);
        let err = guard.0.stderr.take().map(spawn_reader);
        let t0 = Instant::now();
        let code = loop {
            match guard.0.try_wait() {
                Ok(Some(st)) => break st.code(),
                Ok(None) if t0.elapsed() > timeout => {
                    drop(guard);
                    return Err(BatchError::Io(fp_i18n::tr!("batch-anim-err-ffmpeg-timeout", path = self.path.display().to_string())));
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(10)),
                Err(e) => return Err(BatchError::Io(e.to_string())),
            }
        };
        let text = |h: Option<std::thread::JoinHandle<Vec<u8>>>| h.and_then(|h| h.join().ok()).map(|b| String::from_utf8_lossy(&b).to_string()).unwrap_or_default();
        Ok((code, text(out), text(err)))
    }

    /// Version (erste Zeile von `ffmpeg -version`).
    pub fn version(&self) -> Result<String> {
        let (code, out, _) = self.run_capture(&["-hide_banner".into(), "-version".into()], Duration::from_secs(15))?;
        // `-hide_banner -version` gibt die Versionszeile trotzdem aus; ältere Fassungen ohne Banner-Schalter
        // liefern sie ebenso.
        match parse_version(&out) {
            Some(v) if code == Some(0) => Ok(v),
            _ => Err(BatchError::Io(fp_i18n::tr!("batch-anim-err-ffmpeg-not-ffmpeg", path = self.path.display().to_string()))),
        }
    }

    /// Kopfdaten über `ffmpeg -i` (stderr; ffmpeg endet dort mit Code 1, das ist normal).
    pub fn probe(&self, input: &Path) -> Result<VideoInfo> {
        let args = vec!["-hide_banner".to_string(), "-nostdin".into(), "-i".into(), input.to_string_lossy().to_string()];
        let (_, _, err) = self.run_capture(&args, Duration::from_secs(60))?;
        parse_probe(&err).ok_or_else(|| {
            let last = err.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").trim().to_string();
            BatchError::Decode(fp_i18n::tr!("batch-anim-err-video-unreadable", path = input.display().to_string(), detail = last))
        })
    }

    /// Liest die Frames (gestreamt über die Pipe). Abbrechen beendet ffmpeg sofort.
    pub fn read_frames(&self, input: &Path, info: &VideoInfo, params: &VideoParams, cancel: &CancelToken) -> Result<Vec<Frame>> {
        let n = params.frame_count(info.duration_ms);
        if n == 0 {
            return Ok(Vec::new());
        }
        let (w, h) = params.output_size(info.width, info.height, n);
        let args = frame_args(input, params, info, w, h, n);
        let mut cmd = self.command();
        cmd.args(&args).stdout(Stdio::piped()).stderr(Stdio::piped());
        let child = cmd.spawn().map_err(|e| self.start_error(e))?;
        let mut guard = ChildGuard(child);
        let err = guard.0.stderr.take().map(spawn_reader);
        let stdout = guard.0.stdout.take().ok_or_else(|| BatchError::Io(fp_i18n::t("batch-anim-err-ffmpeg-pipe")))?;
        let frame_bytes = w as usize * h as usize * 4;
        let rx = spawn_frame_reader(stdout, frame_bytes, n as usize);
        let mut images: Vec<RgbaImage> = Vec::new();
        loop {
            if cancel.is_cancelled() {
                drop(rx);
                drop(guard); // kill + wait
                return Err(BatchError::cancelled());
            }
            match rx.recv_timeout(Duration::from_millis(40)) {
                Ok(buf) => {
                    let img = RgbaImage::from_rgba_bytes(IRect::new(0, 0, w as i32, h as i32), &buf).map_err(|e| BatchError::Decode(e.to_string()))?;
                    images.push(img);
                    if images.len() >= n as usize {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        // ffmpeg endet von selbst (`-frames:v`); sonst räumt der Wächter auf
        let status = wait_with_timeout(&mut guard.0, Duration::from_secs(10));
        let stderr = err.and_then(|h| h.join().ok()).map(|b| String::from_utf8_lossy(&b).to_string()).unwrap_or_default();
        if images.is_empty() {
            let last = stderr.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").trim().to_string();
            if status.is_some_and(|c| c != 0) || !last.is_empty() {
                return Err(BatchError::Decode(fp_i18n::tr!("batch-anim-err-video-unreadable", path = input.display().to_string(), detail = last)));
            }
        }
        let delays = frame_delays(images.len(), params.fps);
        Ok(images.into_iter().zip(delays).map(|(image, delay_ms)| Frame { image, delay_ms }).collect())
    }
}

impl VideoSource for Ffmpeg {
    fn info(&self, path: &Path) -> Result<VideoInfo> {
        self.probe(path)
    }
    fn frames(&self, path: &Path, info: &VideoInfo, params: &VideoParams, cancel: &CancelToken) -> Result<Vec<Frame>> {
        self.read_frames(path, info, params, cancel)
    }
}

fn wait_with_timeout(child: &mut Child, timeout: Duration) -> Option<i32> {
    let t0 = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(st)) => return st.code(),
            Ok(None) if t0.elapsed() > timeout => {
                let _ = child.kill();
                return child.wait().ok().and_then(|s| s.code());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            Err(_) => return None,
        }
    }
}

fn spawn_reader(mut r: impl Read + Send + 'static) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut v = Vec::new();
        let _ = r.read_to_end(&mut v);
        v
    })
}

/// Liest Frames fester Größe in einem eigenen Thread (höchstens `max`, kleiner Puffer, damit ffmpeg
/// nicht beliebig vorausläuft). Endet bei Dateiende, Fehler oder wenn der Empfänger weg ist.
pub fn spawn_frame_reader(mut r: impl Read + Send + 'static, frame_bytes: usize, max: usize) -> mpsc::Receiver<Vec<u8>> {
    let (tx, rx) = mpsc::sync_channel::<Vec<u8>>(4);
    std::thread::spawn(move || {
        for _ in 0..max {
            let mut buf = vec![0u8; frame_bytes];
            if r.read_exact(&mut buf).is_err() {
                return;
            }
            if tx.send(buf).is_err() {
                return;
            }
        }
    });
    rx
}

#[cfg(windows)]
fn hide_console(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_console(_cmd: &mut Command) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_line() {
        assert_eq!(parse_version("ffmpeg version 7.1.1-full_build-www Copyright (c) 2000-2025\nbuilt with gcc"), Some("7.1.1-full_build-www".into()));
        assert_eq!(parse_version("ffmpeg version n6.1 Copyright"), Some("n6.1".into()));
        assert_eq!(parse_version("something else"), None);
    }

    #[test]
    fn probe_real_output() {
        let s = "Input #0, mov,mp4,m4a,3gp,3g2,mj2, from 'clip.mp4':\n  Metadata:\n    major_brand     : isom\n  Duration: 00:01:02.50, start: 0.000000, bitrate: 2145 kb/s\n  Stream #0:0[0x1](und): Video: h264 (High) (avc1 / 0x31637661), yuv420p(tv, bt709, progressive), 1920x1080 [SAR 1:1 DAR 16:9], 2010 kb/s, 29.97 fps, 29.97 tbr, 30k tbn (default)\n  Stream #0:1[0x2](und): Audio: aac (LC) (mp4a / 0x6134706D), 48000 Hz, stereo, fltp, 128 kb/s (default)\nAt least one output file must be specified\n";
        let i = parse_probe(s).expect("info");
        assert_eq!(i.duration_ms, Some(62_500));
        assert_eq!((i.width, i.height), (1920, 1080));
        assert_eq!(i.codec, "h264");
        assert!((i.fps.unwrap() - 29.97).abs() < 1e-9);
    }

    #[test]
    fn probe_rotated_phone_video() {
        let s = "  Duration: 00:00:03.00, start: 0.000000, bitrate: 1 kb/s\n  Stream #0:0(und): Video: hevc (Main) (hvc1 / 0x31637668), yuv420p(tv), 1920x1080, 30 fps, 30 tbr\n    Side data:\n      displaymatrix: rotation of -90.00 degrees\n";
        let i = parse_probe(s).expect("info");
        assert_eq!((i.width, i.height), (1080, 1920));
    }

    #[test]
    fn probe_without_video_or_duration() {
        assert!(parse_probe("  Duration: 00:00:03.00\n  Stream #0:0: Audio: mp3, 44100 Hz\n").is_none());
        let i = parse_probe("  Duration: N/A, bitrate: N/A\n  Stream #0:0: Video: vp9, yuv420p, 640x360, 25 tbr\n").expect("info");
        assert_eq!(i.duration_ms, None);
        assert_eq!(i.fps, Some(25.0));
    }

    #[test]
    fn args_are_a_list_without_shell() {
        let p = VideoParams { start_ms: 1500, end_ms: Some(4000), fps: 10.0, ..VideoParams::default() };
        let info = VideoInfo { duration_ms: Some(10_000), width: 640, height: 360, ..VideoInfo::default() };
        let a = frame_args(Path::new(r"C:\v\a b.mp4"), &p, &info, 320, 180, 25);
        let joined = a.join(" ");
        assert!(joined.contains("-ss 1.500 -i C:\\v\\a b.mp4 -t 2.500"), "{joined}");
        assert!(joined.contains("fps=10,scale=320:180"), "{joined}");
        assert!(joined.ends_with("-frames:v 25 -f rawvideo -pix_fmt rgba pipe:1"), "{joined}");
        assert!(a.iter().any(|x| x == r"C:\v\a b.mp4"), "path is one argument");
    }
}

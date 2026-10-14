//! Bericht und Protokoll (SPEC-BATCH §2.8): `batch-report.json` (Schlüssel englisch) im
//! Ausgabeordner, Protokoll `batch-log.txt` (englisch, technisch) unter
//! `%LOCALAPPDATA%\Firepixel\Logs\`.

use crate::error::{BatchError, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Kennung im Feld `format` des Berichts.
pub const REPORT_FORMAT: &str = "firepixel-batch-report";
/// Dateiname des Berichts im Ausgabeordner.
pub const REPORT_FILE: &str = "batch-report.json";
/// Dateiname des HTML-Berichts im Ausgabeordner (neben `batch-report.json`, BATCH-11b).
pub const REPORT_HTML_FILE: &str = "batch-report.html";
/// Dateiname des Protokolls im Ordner `Logs`.
pub const LOG_FILE: &str = "batch-log.txt";

/// Zustand eines Elements.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ItemStatus {
    /// Wartet (noch nicht verarbeitet).
    Waiting,
    /// Trockenlauf: würde geschrieben.
    Planned,
    Done,
    Skipped,
    Failed,
    Cancelled,
}

/// Ergebnis eines Elements.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemReport {
    pub index: usize,
    pub source: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<PathBuf>,
    pub status: ItemStatus,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub ms: u64,
    pub warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Schritt, an dem es fehlschlug.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<String>,
    /// Sicherung des Originals (`Original Files`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup: Option<PathBuf>,
    /// Zusätzliche Ausgaben neben `target` (z. B. die FW-PNG bei `alsoFwpng`, BATCH-11b).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub extra_targets: Vec<PathBuf>,
}

impl ItemReport {
    /// Leerer Eintrag.
    pub fn new(index: usize, source: &Path, bytes_in: u64) -> ItemReport {
        ItemReport { index, source: source.to_path_buf(), target: None, status: ItemStatus::Waiting, bytes_in, bytes_out: 0, ms: 0, warnings: Vec::new(), error: None, step: None, backup: None, extra_targets: Vec::new() }
    }
}

/// Summen.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub total: usize,
    pub done: usize,
    pub planned: usize,
    pub skipped: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub ms: u64,
}

/// Bericht eines Auftrags.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub format: String,
    pub version: u32,
    /// Name des Rezepts.
    pub recipe: String,
    /// ISO-8601 (Ortszeit mit Zone).
    pub started: String,
    pub finished: String,
    pub dry_run: bool,
    pub cancelled: bool,
    pub totals: Totals,
    pub items: Vec<ItemReport>,
    pub output_dirs: Vec<PathBuf>,
    /// Hinweise zum ganzen Lauf (unbekannte Schritte …).
    pub warnings: Vec<String>,
    /// Pfad des geschriebenen `batch-report.json`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report_file: Option<PathBuf>,
    /// Pfad des geschriebenen `batch-report.html` (BATCH-11b).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report_html: Option<PathBuf>,
}

impl Report {
    /// Summen aus den Einträgen neu rechnen.
    pub fn recount(&mut self) {
        let mut t = Totals { total: self.items.len(), ..Totals::default() };
        for i in &self.items {
            match i.status {
                ItemStatus::Done => t.done += 1,
                ItemStatus::Planned => t.planned += 1,
                ItemStatus::Skipped => t.skipped += 1,
                ItemStatus::Failed => t.failed += 1,
                ItemStatus::Cancelled | ItemStatus::Waiting => t.cancelled += 1,
            }
            // Summe vorher/nachher nur über geschriebene Dateien: abgebrochene, übersprungene und
            // fehlgeschlagene zählten sonst als „→ 0 Byte (−100 %)“ (SPEC §2.8, BATCH-T)
            if i.status == ItemStatus::Done {
                t.bytes_in += i.bytes_in;
                t.bytes_out += i.bytes_out;
            }
            t.ms += i.ms;
        }
        self.totals = t;
    }

    /// Laufzeit des ganzen Auftrags in Sekunden (Wanduhr, `started` → `finished`). `totals.ms` ist
    /// dagegen die Summe der Rechenzeiten je Datei und bei parallelen Läufen viel größer.
    pub fn wall_seconds(&self) -> Option<f64> {
        let a = chrono::DateTime::parse_from_rfc3339(&self.started).ok()?;
        let b = chrono::DateTime::parse_from_rfc3339(&self.finished).ok()?;
        Some(((b - a).num_milliseconds().max(0)) as f64 / 1000.0)
    }

    /// Exit-Code der Kommandozeile: 0 alles fertig, 1 mindestens ein Fehler, 3 abgebrochen.
    pub fn exit_code(&self) -> u8 {
        if self.cancelled {
            3
        } else if self.totals.failed > 0 {
            1
        } else {
            0
        }
    }

    /// JSON (eingerückt).
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Zusammenfassung in der aktuellen Sprache („21 fertig, 1 übersprungen, 1 Fehler“).
    pub fn summary_text(&self) -> String {
        let t = &self.totals;
        if self.dry_run {
            return fp_i18n::tr!("batch-msg-dry-run-summary", planned = t.planned, skipped = t.skipped, total = t.total);
        }
        if self.cancelled {
            return fp_i18n::tr!("batch-msg-cancelled-summary", done = t.done, rest = t.cancelled);
        }
        fp_i18n::tr!("batch-msg-summary", done = t.done, skipped = t.skipped, failed = t.failed, before = fp_i18n::fmt_size(t.bytes_in), after = fp_i18n::fmt_size(t.bytes_out))
    }

    /// Bericht schreiben (atomar).
    pub fn write(&self, path: &Path) -> Result<()> {
        crate::output::write_bytes_atomic(path, self.to_json().as_bytes())
    }
}

/// Ordner der Protokolle (`%LOCALAPPDATA%\Firepixel\Logs`; in Tests das Testprofil).
pub fn log_dir() -> Option<PathBuf> {
    fp_i18n::profile::local_dir().map(|d| d.join("Logs"))
}

/// Hängt den Lauf an `batch-log.txt` an (englisch, technisch; eine Zeile je Element).
pub fn append_log(report: &Report) -> Result<PathBuf> {
    use std::io::Write;
    let dir = log_dir().ok_or_else(|| BatchError::Io(LOG_FILE.to_string()))?;
    std::fs::create_dir_all(&dir).map_err(|e| BatchError::Io(format!("{}: {e}", dir.display())))?;
    let path = dir.join(LOG_FILE);
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&path).map_err(|e| BatchError::Io(format!("{}: {e}", path.display())))?;
    let t = &report.totals;
    let mut text = format!(
        "[{}] batch \"{}\" dryRun={} cancelled={} total={} done={} skipped={} failed={} bytesIn={} bytesOut={} finished={}\n",
        report.started, report.recipe, report.dry_run, report.cancelled, t.total, t.done, t.skipped, t.failed, t.bytes_in, t.bytes_out, report.finished
    );
    for i in &report.items {
        let status = serde_json::to_value(i.status).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default();
        text.push_str(&format!(
            "  {status} {} -> {} ({} ms){}\n",
            i.source.display(),
            i.target.as_ref().map(|p| p.display().to_string()).unwrap_or_default(),
            i.ms,
            i.error.as_ref().map(|e| format!(" error: {e}")).unwrap_or_default()
        ));
    }
    f.write_all(text.as_bytes()).map_err(|e| BatchError::Io(format!("{}: {e}", path.display())))?;
    Ok(path)
}

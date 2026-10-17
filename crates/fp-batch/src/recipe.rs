//! Rezept-Modell (SPEC-BATCH §2.5): geordnete Schritte, Umbenennen (Endkarte „Dateiname“) und
//! Ausgabe (Endkarte „Speichern als“ + Ziel/Konflikt). JSON mit englischen Schlüsseln, `format` +
//! `version`; unbekannte Schlüssel werden beim Lesen erhalten und beim Speichern wieder geschrieben
//! (`#[serde(flatten)] extra`). Ältere Versionen werden beim Laden migriert.

use crate::error::{BatchError, Result};
use crate::step::{Issue, Severity, StepRegistry};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

/// Kennung im Feld `format` jeder Rezept-Datei.
pub const RECIPE_FORMAT: &str = "firepixel-batch-recipe";
/// Aktuelle Rezept-Version. Steigt nur bei inkompatiblen Änderungen (dann Migration in [`Recipe::migrate`]).
pub const RECIPE_VERSION: u32 = 1;
/// Endung gespeicherter Rezepte.
pub const RECIPE_EXTENSION: &str = "fpbatch.json";
/// Unterordner der Rezepte im Roaming-Profil (`%APPDATA%\Firepixel\BatchRecipes`).
pub const RECIPE_FOLDER: &str = "BatchRecipes";
/// Vorgabe-Name des neuen Ausgabeordners.
pub const DEFAULT_OUTPUT_FOLDER: &str = "batch-output";

fn default_format() -> String {
    RECIPE_FORMAT.to_string()
}
fn default_version() -> u32 {
    RECIPE_VERSION
}
fn yes() -> bool {
    true
}

/// Ein Rezept.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recipe {
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Schritte in sichtbarer Reihenfolge (ohne die Endkarten `rename`/`output`).
    #[serde(default)]
    pub steps: Vec<StepConfig>,
    /// Endkarte „Dateiname“ (§4).
    #[serde(default)]
    pub rename: RenameSpec,
    /// Endkarte „Speichern als“ + Ziel, Konflikt, Sicherung (§2.6, §2.7, §3.2).
    #[serde(default)]
    pub output: OutputSpec,
    /// Unbekannte Schlüssel (neuere Versionen, eigene Erweiterungen) – bleiben beim Speichern erhalten.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Default for Recipe {
    fn default() -> Self {
        Recipe {
            format: default_format(),
            version: RECIPE_VERSION,
            name: String::new(),
            description: String::new(),
            steps: Vec::new(),
            rename: RenameSpec::default(),
            output: OutputSpec::default(),
            extra: Map::new(),
        }
    }
}

/// Ein Schritt im Rezept. Die Parameter bleiben ein JSON-Objekt; Bedeutung und Schema liefert der
/// Schritt ([`crate::step::Step::params`]). Ein unbekannter Schritt (neuere Version) lädt trotzdem
/// und behält seine Parameter; er wird beim Lauf übersprungen (Warnung).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepConfig {
    pub id: String,
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default = "empty_object")]
    pub params: Value,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

fn empty_object() -> Value {
    Value::Object(Map::new())
}

impl StepConfig {
    /// Neuer, eingeschalteter Schritt.
    pub fn new(id: &str, params: Value) -> StepConfig {
        let params = if params.is_object() {
            params
        } else {
            empty_object()
        };
        StepConfig {
            id: id.to_string(),
            enabled: true,
            params,
            extra: Map::new(),
        }
    }
}

// ------------------------------------------------------------------ Umbenennen (§4)

/// Baukasten (Anfänger) oder Regex (Profi).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RenameMode {
    #[default]
    Builder,
    Regex,
}

/// Teil des Originalnamens für den Baustein `original`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OriginalTake {
    #[default]
    Whole,
    /// Erste `count` Zeichen.
    First,
    /// Letzte `count` Zeichen.
    Last,
    /// Ohne Ziffern (und Trenner `_- .`) am Ende.
    NoTrailingDigits,
}

/// Quelle des Datums-Bausteins.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DateSource {
    Today,
    #[default]
    Modified,
    /// EXIF-Aufnahmedatum, Rückfall Änderungsdatum (Hinweis).
    Exif,
}

/// Welche Maße der Größen-Baustein zeigt.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SizeWhich {
    #[default]
    Both,
    Width,
    Height,
}

/// EXIF-Feld des Bausteins `exif`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExifField {
    #[default]
    Model,
    Make,
    Lens,
    Iso,
    Aperture,
}

/// Baustein des Dateinamens (§4.1). Unbekannte Arten laden als [`NamePart::Unknown`] (ergeben nichts).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum NamePart {
    Original {
        #[serde(default)]
        take: OriginalTake,
        #[serde(default)]
        count: usize,
    },
    Text {
        #[serde(default)]
        text: String,
    },
    Counter {
        #[serde(default = "one")]
        start: u64,
        #[serde(default = "one")]
        step: u64,
        #[serde(default = "three")]
        digits: u32,
        /// Je Quellordner neu beginnen.
        #[serde(default)]
        per_folder: bool,
    },
    Date {
        #[serde(default)]
        source: DateSource,
        /// `YYYY`, `YY`, `MM`, `DD`, `HH`, `mm`, `ss`; sonst wörtlich.
        #[serde(default = "default_date_format")]
        format: String,
    },
    Size {
        #[serde(default)]
        which: SizeWhich,
    },
    Folder,
    Exif {
        #[serde(default)]
        field: ExifField,
    },
    /// Endung des Zielformats ohne Punkt (`webp`).
    Format,
    #[serde(other)]
    Unknown,
}

fn one() -> u64 {
    1
}
fn three() -> u32 {
    3
}
fn default_date_format() -> String {
    "YYYY-MM-DD".to_string()
}

/// Eine Zeile „Ersetze … durch …“.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Replacement {
    pub find: String,
    #[serde(default)]
    pub replace: String,
    /// Alle Vorkommen (aus = nur das erste, wie im Original).
    #[serde(default = "yes")]
    pub all: bool,
    #[serde(default = "yes")]
    pub match_case: bool,
}

/// Schreibweise (§4.1 Punkt 4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CaseMode {
    #[default]
    Keep,
    Lower,
    Upper,
    /// Erster Buchstabe groß.
    Sentence,
    /// Jedes Wort groß.
    Title,
    Kebab,
    Snake,
    Camel,
}

/// Schreibweise der Endung.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExtCase {
    #[default]
    Lower,
    Keep,
    Upper,
}

/// Profi-Umbenennen mit regulärem Ausdruck (§4.2).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegexRename {
    #[serde(default)]
    pub find: String,
    /// Ersetzung mit `$1`, `${name}`, `$0` und den Platzhaltern `{n:3}`, `{date:…}`, `{exif-date:…}`,
    /// `{w}`, `{h}`, `{folder}`, `{ext}`, `{name}`.
    #[serde(default)]
    pub replace: String,
    #[serde(default)]
    pub ignore_case: bool,
    #[serde(default = "yes")]
    pub all: bool,
    /// Auf den ganzen Namen inkl. Endung anwenden.
    #[serde(default)]
    pub whole_name: bool,
}

/// Endkarte „Dateiname“.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameSpec {
    #[serde(default)]
    pub mode: RenameMode,
    #[serde(default = "default_parts")]
    pub parts: Vec<NamePart>,
    #[serde(default)]
    pub replacements: Vec<Replacement>,
    /// Leerzeichen ersetzen durch (None = unverändert).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spaces: Option<String>,
    #[serde(default)]
    pub prefix: String,
    /// Vor der Endung.
    #[serde(default)]
    pub suffix: String,
    #[serde(default)]
    pub case: CaseMode,
    /// Umlaute umschreiben (ä → ae, ß → ss, é → e).
    #[serde(default)]
    pub transliterate: bool,
    /// Sonderzeichen entfernen (nur Buchstaben, Ziffern, `_`, `-`, `.`, Leerzeichen bleiben).
    #[serde(default)]
    pub strip_special: bool,
    #[serde(default)]
    pub extension: ExtCase,
    /// `jpeg` → `jpg`.
    #[serde(default = "yes")]
    pub jpeg_as_jpg: bool,
    #[serde(default)]
    pub regex: RegexRename,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

fn default_parts() -> Vec<NamePart> {
    vec![NamePart::Original {
        take: OriginalTake::Whole,
        count: 0,
    }]
}

impl Default for RenameSpec {
    /// Vorgabe der Karte: ein Baustein `original`, sonst alles aus → Name bleibt gleich.
    fn default() -> Self {
        RenameSpec {
            mode: RenameMode::Builder,
            parts: default_parts(),
            replacements: Vec::new(),
            spaces: None,
            prefix: String::new(),
            suffix: String::new(),
            case: CaseMode::Keep,
            transliterate: false,
            strip_special: false,
            extension: ExtCase::Lower,
            jpeg_as_jpg: true,
            regex: RegexRename {
                all: true,
                ..RegexRename::default()
            },
            extra: Map::new(),
        }
    }
}

// ------------------------------------------------------------------ Ausgabe (§2.6, §2.7)

/// Wohin geschrieben wird.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TargetKind {
    /// Neuer Ordner neben den Quellen (`batch-output`, bei Vorhandensein `batch-output-2` …).
    #[default]
    NewFolder,
    /// Eigener Ordner (`path`).
    #[serde(alias = "custom")]
    Folder,
    /// Gleicher Ort wie das Original (Konfliktregel zwingend).
    #[serde(alias = "same")]
    SameAsSource,
}

/// Ziel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetSpec {
    #[serde(default)]
    pub kind: TargetKind,
    /// Eigener Ordner (bei `folder`); bei `newFolder` optional der Ordner, in dem der neue Ordner entsteht.
    #[serde(default)]
    pub path: Option<PathBuf>,
    #[serde(default = "default_folder_name")]
    pub folder_name: String,
    /// Unterordner-Struktur rekursiv hinzugefügter Ordner beibehalten.
    #[serde(default = "yes")]
    pub keep_structure: bool,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

fn default_folder_name() -> String {
    DEFAULT_OUTPUT_FOLDER.to_string()
}

impl Default for TargetSpec {
    fn default() -> Self {
        TargetSpec {
            kind: TargetKind::NewFolder,
            path: None,
            folder_name: default_folder_name(),
            keep_structure: true,
            extra: Map::new(),
        }
    }
}

/// Konfliktregel (§2.7).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Conflict {
    /// `name (2).ext`, `name (3).ext` …
    #[default]
    Rename,
    Skip,
    Overwrite,
    Ask,
}

/// Sicherung beim Ersetzen des Originals (Unterordner `Original Files`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSpec {
    #[serde(default)]
    pub enabled: bool,
    /// Sicherungen behalten: `name-1.ext`, `name-2.ext` … statt die alte Sicherung zu überschreiben.
    #[serde(default)]
    pub incremental: bool,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Endkarte „Speichern als“ + Ausgabe-Leiste.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputSpec {
    /// `same`, `png`, `jpeg`, `webp`, `avif`, `gif`, `apng`, `tiff`, `bmp`, `qoi`, `ico`, `fwpng` …
    #[serde(default = "default_out_format")]
    pub format: String,
    /// Formatoptionen (`quality`, `lossless`, `bitDepth`, `colors`, `background`, `compression` …).
    #[serde(default)]
    pub options: Map<String, Value>,
    #[serde(default)]
    pub target: TargetSpec,
    #[serde(default)]
    pub conflict: Conflict,
    #[serde(default)]
    pub backup: BackupSpec,
    /// Änderungsdatum der Quelle übernehmen.
    #[serde(default)]
    pub keep_timestamps: bool,
    /// Arbeits-Threads (None = Kerne − 1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threads: Option<usize>,
    /// Speichergrenze in MB (None = Vorgabe).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_limit_mb: Option<u64>,
    /// `batch-report.json` in den Ausgabeordner schreiben.
    #[serde(default = "yes")]
    pub report: bool,
    /// Zusätzlich `batch-report.html` mit Vorschaubildern neben den JSON-Bericht (BATCH-11b).
    #[serde(default = "yes")]
    pub report_html: bool,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

fn default_out_format() -> String {
    "same".to_string()
}

impl Default for OutputSpec {
    fn default() -> Self {
        OutputSpec {
            format: default_out_format(),
            options: Map::new(),
            target: TargetSpec::default(),
            conflict: Conflict::Rename,
            backup: BackupSpec::default(),
            keep_timestamps: false,
            threads: None,
            memory_limit_mb: None,
            report: true,
            report_html: true,
            extra: Map::new(),
        }
    }
}

// ------------------------------------------------------------------ Laden, Speichern, Migration

impl Recipe {
    /// Leeres Rezept mit Namen.
    pub fn named(name: &str) -> Recipe {
        Recipe {
            name: name.to_string(),
            ..Recipe::default()
        }
    }

    /// Aus JSON-Text (BOM erlaubt). Migriert ältere Versionen.
    pub fn from_json(text: &str) -> Result<Recipe> {
        let v: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| {
            BatchError::Recipe(fp_i18n::tr!("batch-err-recipe-json", error = e.to_string()))
        })?;
        Recipe::from_value(v)
    }

    /// Aus einem JSON-Wert. Migriert ältere Versionen; neuere werden abgelehnt.
    pub fn from_value(mut v: Value) -> Result<Recipe> {
        if !v.is_object() {
            return Err(BatchError::Recipe(fp_i18n::t(
                "batch-err-recipe-not-object",
            )));
        }
        if let Some(f) = v.get("format").and_then(Value::as_str)
            && f != RECIPE_FORMAT
        {
            return Err(BatchError::Recipe(fp_i18n::tr!(
                "batch-err-recipe-format",
                format = f.to_string()
            )));
        }
        let version = v.get("version").and_then(Value::as_u64).unwrap_or(0) as u32;
        if version > RECIPE_VERSION {
            return Err(BatchError::Recipe(fp_i18n::tr!(
                "batch-err-recipe-newer",
                version = version,
                supported = RECIPE_VERSION
            )));
        }
        Recipe::migrate(&mut v, version);
        serde_json::from_value(v).map_err(|e| {
            BatchError::Recipe(fp_i18n::tr!("batch-err-recipe-json", error = e.to_string()))
        })
    }

    /// Hebt ein Rezept der Version `from` auf [`RECIPE_VERSION`].
    /// Version 0 (ohne `version`/`format`, z. B. von Hand geschrieben): Schritte als reine
    /// Zeichenketten (`"steps": ["resize"]`) werden zu Objekten.
    fn migrate(v: &mut Value, from: u32) {
        if from < 1
            && let Some(steps) = v.get_mut("steps").and_then(Value::as_array_mut)
        {
            for s in steps.iter_mut() {
                if let Some(id) = s.as_str() {
                    *s = serde_json::json!({"id": id});
                }
            }
        }
        if let Some(o) = v.as_object_mut() {
            o.insert("format".into(), Value::String(RECIPE_FORMAT.into()));
            o.insert("version".into(), Value::from(RECIPE_VERSION));
        }
    }

    /// Rezept-Datei lesen.
    pub fn load(path: &Path) -> Result<Recipe> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| BatchError::Io(format!("{}: {e}", path.display())))?;
        Recipe::from_json(&text)
    }

    /// Als JSON (eingerückt).
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Rezept-Datei schreiben (atomar über eine Temporärdatei).
    pub fn save(&self, path: &Path) -> Result<()> {
        crate::output::write_bytes_atomic(path, self.to_json().as_bytes())
    }

    /// Hängt einen Schritt an (vor den Endkarten) und liefert seinen Index.
    pub fn add_step(&mut self, id: &str, params: Value) -> usize {
        self.steps.push(StepConfig::new(id, params));
        self.steps.len() - 1
    }

    /// Entfernt einen Schritt (ungültiger Index: nichts).
    pub fn remove_step(&mut self, index: usize) -> Option<StepConfig> {
        (index < self.steps.len()).then(|| self.steps.remove(index))
    }

    /// Verschiebt einen Schritt (Ziehen, Alt+↑/↓). Liefert `false` bei ungültigem Index.
    pub fn move_step(&mut self, from: usize, to: usize) -> bool {
        if from >= self.steps.len() || to >= self.steps.len() {
            return false;
        }
        let s = self.steps.remove(from);
        self.steps.insert(to, s);
        true
    }

    /// Setzt einen Parameter (idempotent: gleicher Wert erneut ändert nichts). Liefert `true`, wenn
    /// sich etwas geändert hat.
    pub fn set_param(&mut self, index: usize, name: &str, value: Value) -> bool {
        let Some(s) = self.steps.get_mut(index) else {
            return false;
        };
        if !s.params.is_object() {
            s.params = empty_object();
        }
        let Some(o) = s.params.as_object_mut() else {
            return false;
        };
        if o.get(name) == Some(&value) {
            return false;
        }
        o.insert(name.to_string(), value);
        true
    }

    /// Eingeschaltete Schritte.
    pub fn enabled_steps(&self) -> impl Iterator<Item = (usize, &StepConfig)> {
        self.steps.iter().enumerate().filter(|(_, s)| s.enabled)
    }

    /// Prüft das Rezept gegen das Schritt-Register: unbekannte Schritte (Warnung), ungültige
    /// Parameter (Fehler), ungültiger regulärer Ausdruck, fehlender eigener Ordner, unbekanntes Format.
    pub fn validate(&self, reg: &StepRegistry) -> Vec<Issue> {
        let mut out = Vec::new();
        for (i, s) in self.steps.iter().enumerate() {
            match reg.get(&s.id) {
                None => out.push(Issue::step(
                    i,
                    &s.id,
                    Severity::Warning,
                    "unknownStep",
                    fp_i18n::tr!("batch-issue-unknown-step", id = s.id.clone()),
                )),
                Some(step) if s.enabled => {
                    for mut issue in step.validate(&s.params) {
                        issue.step = Some(i);
                        issue.step_id = Some(s.id.clone());
                        out.push(issue);
                    }
                    if step.kind() == crate::step::StepKind::Final {
                        out.push(Issue::step(
                            i,
                            &s.id,
                            Severity::Error,
                            "finalStepInList",
                            fp_i18n::tr!("batch-issue-final-step", id = s.id.clone()),
                        ));
                    }
                }
                Some(_) => {}
            }
        }
        if self.rename.mode == RenameMode::Regex
            && let Err(e) = crate::naming::compile_regex(&self.rename.regex)
        {
            out.push(Issue::global(
                Severity::Error,
                "invalidRegex",
                e.to_string(),
            ));
        }
        if self.output.target.kind == TargetKind::Folder
            && self
                .output
                .target
                .path
                .as_ref()
                .is_none_or(|p| p.as_os_str().is_empty())
        {
            out.push(Issue::global(
                Severity::Error,
                "missingFolder",
                fp_i18n::t("batch-issue-missing-folder"),
            ));
        }
        match crate::output::OutputFormat::parse(&self.output.format) {
            None => out.push(Issue::global(
                Severity::Error,
                "unknownFormat",
                fp_i18n::tr!(
                    "batch-issue-unknown-format",
                    format = self.output.format.clone()
                ),
            )),
            Some(f) if f != crate::output::OutputFormat::Same && !f.can_encode() => {
                out.push(Issue::global(
                    Severity::Error,
                    "formatUnavailable",
                    fp_i18n::tr!("batch-err-format-unavailable", format = f.display_name()),
                ))
            }
            _ => {}
        }
        // „Nur kopieren/verschieben“ verträgt keine Bildschritte und kein anderes Zielformat
        let enabled: Vec<&StepConfig> = self.steps.iter().filter(|s| s.enabled).collect();
        let file_steps = enabled
            .iter()
            .filter(|s| reg.get(&s.id).is_some_and(|st| st.file_only()))
            .count();
        let copy_only = enabled
            .iter()
            .position(|s| s.id == crate::steps::files::COPY_ONLY);
        if let Some(i) = copy_only {
            let other = enabled
                .iter()
                .any(|s| reg.get(&s.id).is_some_and(|st| !st.file_only()));
            let format_changes = crate::output::OutputFormat::parse(&self.output.format)
                .is_some_and(|f| f != crate::output::OutputFormat::Same);
            if other || format_changes {
                let idx = self
                    .steps
                    .iter()
                    .position(|s| std::ptr::eq(s, enabled[i]))
                    .unwrap_or(0);
                out.push(Issue::step(
                    idx,
                    crate::steps::files::COPY_ONLY,
                    Severity::Error,
                    "copyOnlyCombined",
                    fp_i18n::t("batch-issue-copy-only-combined"),
                ));
            }
        }
        let _ = file_steps;
        out
    }
}

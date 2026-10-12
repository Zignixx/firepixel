//! Modellkatalog und Modellordner.
//!
//! Die Gewichte werden nicht in die exe eingebettet (zu groß). Firepixel sucht sie in dieser
//! Reihenfolge:
//! 1. eingestellter Modellordner ([`set_user_folder`], Voreinstellung `AiModelFolder`),
//! 2. `%LOCALAPPDATA%\Firepixel\Modelle`,
//! 3. `Modelle\` neben der exe.
//!
//! Heruntergeladen wird nur nach Zustimmung in den ersten beschreibbaren Ordner (Voreinstellung,
//! sonst `%LOCALAPPDATA%\Firepixel\Modelle`). Alle Quellen sind offizielle Veröffentlichungen der
//! Urheber (Meta auf Hugging Face bzw. `ZhengPeng7`), Prüfsummen = LFS-SHA-256 der Dateien.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// Ein unterstütztes Modell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelId {
    /// SAM 2.1 Hiera Tiny (Objektauswahl, Vorgabe).
    Sam2Tiny,
    /// SAM 2.1 Hiera Small (Objektauswahl, genauer).
    Sam2Small,
    /// BiRefNet_lite (Motiv auswählen, Vorgabe).
    BiRefNetLite,
    /// BiRefNet (Swin-L, Motiv auswählen, hohe Qualität).
    BiRefNet,
    /// BiRefNet_lite-matting (Kante verfeinern, Vorgabe).
    BiRefNetLiteMatting,
    /// BiRefNet_HR-matting (Kante verfeinern, hohe Qualität).
    BiRefNetHrMatting,
}

/// Aufgabe eines Modells.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Task {
    Object,
    Subject,
    Matting,
}

/// Beschreibung eines Modells (Dialog „KI-Modell fehlt“ und `docs\MODELLE.md`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ModelInfo {
    pub id: ModelId,
    /// Anzeigename.
    pub name: &'static str,
    pub task: Task,
    /// Dateiname im Modellordner.
    pub file: &'static str,
    /// Offizielle Download-Adresse.
    pub url: &'static str,
    /// Seite der Veröffentlichung.
    pub page: &'static str,
    /// Größe in Byte.
    pub size: u64,
    /// SHA-256 (klein, hex).
    pub sha256: &'static str,
    /// Lizenz der Gewichte.
    pub license: &'static str,
    /// Hinweis (Trainingsdaten, Eignung).
    pub note: &'static str,
    /// Vorgabe für die Aufgabe?
    pub default: bool,
}

/// Alle Modelle.
pub const MODELS: &[ModelInfo] = &[
    ModelInfo {
        id: ModelId::Sam2Tiny,
        name: "SAM 2.1 Hiera Tiny",
        task: Task::Object,
        file: "sam2.1-hiera-tiny.safetensors",
        url: "https://huggingface.co/facebook/sam2.1-hiera-tiny/resolve/main/model.safetensors",
        page: "https://huggingface.co/facebook/sam2.1-hiera-tiny",
        size: 155_908_064,
        sha256: "48c14467e5cf9e51870511feb72c89688e82dd74523142c0538b663e193ac2a7",
        license: "Apache-2.0",
        note: "Meta (facebookresearch/sam2): Code und Gewichte Apache-2.0, kommerzielle Nutzung erlaubt; trainiert auf SA-1B/SA-V",
        default: true,
    },
    ModelInfo {
        id: ModelId::Sam2Small,
        name: "SAM 2.1 Hiera Small",
        task: Task::Object,
        file: "sam2.1-hiera-small.safetensors",
        url: "https://huggingface.co/facebook/sam2.1-hiera-small/resolve/main/model.safetensors",
        page: "https://huggingface.co/facebook/sam2.1-hiera-small",
        size: 184_305_280,
        sha256: "0a4067b11ce1e23d5229203f11c718a823060d15a4b23fa2372a7d4b77cbbc60",
        license: "Apache-2.0",
        note: "Meta (facebookresearch/sam2): Code und Gewichte Apache-2.0, kommerzielle Nutzung erlaubt; trainiert auf SA-1B/SA-V",
        default: false,
    },
    ModelInfo {
        id: ModelId::BiRefNetLite,
        name: "BiRefNet lite (allgemein)",
        task: Task::Subject,
        file: "BiRefNet_lite.safetensors",
        url: "https://huggingface.co/ZhengPeng7/BiRefNet_lite/resolve/main/model.safetensors",
        page: "https://huggingface.co/ZhengPeng7/BiRefNet_lite",
        size: 177_634_392,
        sha256: "4417d89795250e698c3cb0ae8df15743810065f646f48a694fdfa7ca052d0815",
        license: "MIT",
        note: "ZhengPeng7/BiRefNet, Swin-T; Gewichte MIT. Trainingsdaten laut Autor u. a. DIS5K, DUTS, HRSOD, UHRSD, HRS10K, P3M-10k, humans (P3M-10k ist nur für nichtkommerzielle Forschung freigegeben)",
        default: true,
    },
    ModelInfo {
        id: ModelId::BiRefNet,
        name: "BiRefNet (allgemein, hohe Qualität)",
        task: Task::Subject,
        file: "BiRefNet.safetensors",
        url: "https://huggingface.co/ZhengPeng7/BiRefNet/resolve/main/model.safetensors",
        page: "https://huggingface.co/ZhengPeng7/BiRefNet",
        size: 444_473_596,
        sha256: "9ab37426bf4de0567af6b5d21b16151357149139362e6e8992021b8ce356a154",
        license: "MIT",
        note: "ZhengPeng7/BiRefNet, Swin-L (langsamer); Gewichte MIT. Trainingsdaten wie BiRefNet lite (u. a. P3M-10k, nur nichtkommerzielle Forschung)",
        default: false,
    },
    ModelInfo {
        id: ModelId::BiRefNetLiteMatting,
        name: "BiRefNet lite Matting",
        task: Task::Matting,
        file: "BiRefNet_lite-matting.safetensors",
        url: "https://huggingface.co/ZhengPeng7/BiRefNet_lite-matting/resolve/main/model.safetensors",
        page: "https://huggingface.co/ZhengPeng7/BiRefNet_lite-matting",
        size: 88_965_896,
        sha256: "ce8bcfc045e336322c0424a5863dcfb7e9ce8fed0a5fd4d1b2b20adf12d97243",
        license: "MIT",
        note: "ZhengPeng7/BiRefNet, Swin-T, für Haare und Fell; Gewichte MIT. Trainingsdaten laut Autor P3M-10k, humans, AM-2k, Distinctions-646, HIM2K (teils nur nichtkommerzielle Forschung)",
        default: true,
    },
    ModelInfo {
        id: ModelId::BiRefNetHrMatting,
        name: "BiRefNet HR Matting (hohe Qualität)",
        task: Task::Matting,
        file: "BiRefNet_HR-matting.safetensors",
        url: "https://huggingface.co/ZhengPeng7/BiRefNet_HR-matting/resolve/main/model.safetensors",
        page: "https://huggingface.co/ZhengPeng7/BiRefNet_HR-matting",
        size: 444_473_596,
        sha256: "a5a4de698739ea5e0e8bbab28e1b293dde95092b87a442d566cbc585c53cef55",
        license: "MIT",
        note: "ZhengPeng7/BiRefNet, Swin-L, für Haare und Fell (langsamer); Gewichte MIT. Trainingsdaten wie Matting lite (teils nur nichtkommerzielle Forschung)",
        default: false,
    },
];

/// Beschreibung eines Modells.
pub fn info(id: ModelId) -> &'static ModelInfo {
    MODELS.iter().find(|m| m.id == id).unwrap_or(&MODELS[0])
}

impl ModelId {
    /// Schlüssel des Anzeigenamens (`meldungen.ftl`; Modellnamen bleiben, Zusätze werden übersetzt).
    pub fn name_key(self) -> &'static str {
        match self {
            ModelId::Sam2Tiny => "ai-model-sam2-tiny",
            ModelId::Sam2Small => "ai-model-sam2-small",
            ModelId::BiRefNetLite => "ai-model-birefnet-lite",
            ModelId::BiRefNet => "ai-model-birefnet",
            ModelId::BiRefNetLiteMatting => "ai-model-birefnet-lite-matting",
            ModelId::BiRefNetHrMatting => "ai-model-birefnet-hr-matting",
        }
    }

    /// Schlüssel des Hinweises (Lizenz, Trainingsdaten).
    pub fn note_key(self) -> &'static str {
        match self {
            ModelId::Sam2Tiny => "ai-model-sam2-tiny-note",
            ModelId::Sam2Small => "ai-model-sam2-small-note",
            ModelId::BiRefNetLite => "ai-model-birefnet-lite-note",
            ModelId::BiRefNet => "ai-model-birefnet-note",
            ModelId::BiRefNetLiteMatting => "ai-model-birefnet-lite-matting-note",
            ModelId::BiRefNetHrMatting => "ai-model-birefnet-hr-matting-note",
        }
    }

    /// Anzeigename in der Sprache der Oberfläche (Deutsch = [`ModelInfo::name`]).
    pub fn display_name(self) -> String {
        fp_i18n::t(self.name_key())
    }
}

impl ModelInfo {
    /// Anzeigename in der Sprache der Oberfläche (Deutsch = `name`).
    pub fn display_name(&self) -> String {
        self.id.display_name()
    }

    /// Hinweis in der Sprache der Oberfläche (Deutsch = `note`).
    pub fn display_note(&self) -> String {
        fp_i18n::t(self.id.note_key())
    }
}

/// Vorgabemodell einer Aufgabe.
pub fn default_for(task: Task) -> ModelId {
    MODELS.iter().find(|m| m.task == task && m.default).map(|m| m.id).unwrap_or(ModelId::Sam2Tiny)
}

/// Modell nach Namen (`sam2Tiny`, `birefnetLite`, Dateiname …).
pub fn by_name(s: &str) -> Option<ModelId> {
    let l = s.to_ascii_lowercase().replace(['-', '_', '.', ' '], "");
    MODELS
        .iter()
        .find(|m| {
            let id = serde_json::to_value(m.id).ok().and_then(|v| v.as_str().map(|x| x.to_ascii_lowercase())).unwrap_or_default();
            id == l || m.file.to_ascii_lowercase().replace(['-', '_', '.', ' '], "").trim_end_matches("safetensors") == l
        })
        .map(|m| m.id)
}

static USER_FOLDER: RwLock<Option<PathBuf>> = RwLock::new(None);
static EXTRA_FOLDERS: RwLock<Vec<PathBuf>> = RwLock::new(Vec::new());

/// Eingestellter Modellordner (Voreinstellung), `None` = keiner.
pub fn set_user_folder(p: Option<PathBuf>) {
    if let Ok(mut g) = USER_FOLDER.write() {
        *g = p.filter(|p| !p.as_os_str().is_empty());
    }
}

/// Eingestellter Modellordner.
pub fn user_folder() -> Option<PathBuf> {
    USER_FOLDER.read().ok().and_then(|g| g.clone())
}

/// Nur für Tests: Suchordner ersetzen (leer = Standardsuche).
pub fn set_search_override(folders: Vec<PathBuf>) {
    if let Ok(mut g) = EXTRA_FOLDERS.write() {
        *g = folders;
    }
}

/// `%LOCALAPPDATA%\Firepixel\Modelle`.
pub fn local_folder() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("Firepixel").join("Modelle"))
}

/// `Modelle\` neben der exe.
pub fn exe_folder() -> Option<PathBuf> {
    std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.join("Modelle")))
}

/// Suchreihenfolge der Modellordner.
pub fn search_folders() -> Vec<PathBuf> {
    if let Ok(g) = EXTRA_FOLDERS.read()
        && !g.is_empty()
    {
        return g.clone();
    }
    let mut v = Vec::new();
    if let Some(u) = user_folder() {
        v.push(u);
    }
    if let Some(l) = local_folder() {
        v.push(l);
    }
    if let Some(e) = exe_folder() {
        v.push(e);
    }
    v
}

/// Zielordner für Downloads.
pub fn download_folder() -> PathBuf {
    search_folders().into_iter().next().unwrap_or_else(|| PathBuf::from("Modelle"))
}

/// Datei eines Modells, falls vorhanden (Größe muss stimmen).
pub fn find(id: ModelId) -> Option<PathBuf> {
    let m = info(id);
    search_folders().into_iter().map(|f| f.join(m.file)).find(|p| file_ok(p, m.size))
}

fn file_ok(p: &Path, size: u64) -> bool {
    std::fs::metadata(p).map(|md| md.is_file() && md.len() == size).unwrap_or(false)
}

/// Ist das Modell vorhanden?
pub fn available(id: ModelId) -> bool {
    find(id).is_some()
}

/// Fehler „Modell fehlt“ mit Ordnerangabe.
pub fn missing(id: ModelId) -> crate::AiError {
    let m = info(id);
    crate::AiError::ModelMissing { model: id, name: m.name, file: m.file, folder: download_folder().display().to_string() }
}

/// Größe lesbar („156 MB“).
pub fn human_size(b: u64) -> String {
    if b >= 1_000_000_000 {
        format!("{:.1} GB", b as f64 / 1e9)
    } else {
        format!("{} MB", (b as f64 / 1e6).round() as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog() {
        for t in [Task::Object, Task::Subject, Task::Matting] {
            assert_eq!(MODELS.iter().filter(|m| m.task == t && m.default).count(), 1);
        }
        for m in MODELS {
            assert_eq!(m.sha256.len(), 64);
            assert!(m.url.starts_with("https://huggingface.co/facebook/") || m.url.starts_with("https://huggingface.co/ZhengPeng7/"));
            assert_eq!(by_name(serde_json::to_value(m.id).unwrap_or_default().as_str().unwrap_or_default()), Some(m.id));
        }
        assert_eq!(by_name("birefnet_lite"), Some(ModelId::BiRefNetLite));
        assert_eq!(human_size(155_908_064), "156 MB");
    }

    /// Deutsch wortgleich mit `name`/`note`, Englisch ohne deutsche Zusätze.
    #[test]
    fn display_texts_follow_language() {
        for m in MODELS {
            fp_i18n::with_language("de", || {
                assert_eq!(m.display_name(), m.name);
                assert_eq!(m.display_note(), m.note);
            });
            fp_i18n::with_language("en", || {
                let (n, t) = (m.display_name(), m.display_note());
                for w in ["allgemein", "hohe Qualität", "Gewichte", "Trainingsdaten", "nichtkommerziell"] {
                    assert!(!n.contains(w) && !t.contains(w), "{n} / {t}");
                }
            });
        }
        let e = fp_i18n::with_language("en", || missing(ModelId::BiRefNet).to_string());
        assert!(e.contains("BiRefNet (general, high quality)") && e.contains("Manage AI Models"), "{e}");
    }
}

//! Texturen und Muster aus den Ressourcenordnern, nur zur Laufzeit gelesen (nichts eingebettet).
//!
//! Leserichtung je Art (`Textures` bzw. `Patterns`): der Ressourcenordner des Originals
//! (`Configuration\<Art>`), dessen Vorlage für den Benutzerordner (`First Run\<Art>`; Ordner aus
//! `fp_color::install`) und der eigene Benutzerordner `%APPDATA%\Firepixel\<Art>`
//! (`fp_i18n::profile`). Dieselben Ordner zeigt der Inspektor in seinen Listen; eine Textur bzw.
//! ein Muster, das nur per Name gesetzt wird (`pi.fill {texture}`, Werkzeugvorgaben, Skripte),
//! lädt hier die Datei zum Namen, damit die Füllung sichtbar wirkt.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// Ordnername der Texturen (technisch englisch, §1c).
pub const TEXTURES: &str = "Textures";
/// Ordnername der Muster.
pub const PATTERNS: &str = "Patterns";
/// Bildendungen, die als Textur bzw. Muster gelesen werden.
pub const IMAGE_EXTS: [&str; 5] = ["png", "jpg", "jpeg", "gif", "bmp"];

/// Ordner einer Ressourcenart in Leserichtung (siehe Moduldoku); `original` = Installationsordner
/// des Originals, `user_base` = eigener Benutzerordner (`%APPDATA%\Firepixel`).
pub fn dirs_in(original: Option<&Path>, user_base: Option<&Path>, kind: &str) -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(o) = original {
        v.push(o.join("Configuration").join(kind));
        v.push(o.join("First Run").join(kind));
    }
    if let Some(u) = user_base {
        v.push(u.join(kind));
    }
    v
}

/// Ordner einer Ressourcenart auf diesem Rechner.
pub fn dirs(kind: &str) -> Vec<PathBuf> {
    let original = fp_color::install::original_dir();
    let user = fp_i18n::profile::roaming_dir();
    dirs_in(original.as_deref(), user.as_deref(), kind)
}

fn is_image(p: &Path) -> bool {
    p.is_file()
        && p.extension()
            .is_some_and(|x| IMAGE_EXTS.iter().any(|e| x.eq_ignore_ascii_case(e)))
}

/// Bilddateien der Ordner (Name = Dateiname ohne Endung, Pfad), je Name nur die erste Fundstelle
/// (ohne Rücksicht auf Groß-/Kleinschreibung), nach Namen geordnet. Fehlende Ordner zählen nicht.
pub fn files_in(dirs: &[PathBuf]) -> Vec<(String, PathBuf)> {
    let mut v: Vec<(String, PathBuf)> = Vec::new();
    for dir in dirs {
        let Ok(rd) = std::fs::read_dir(dir) else {
            continue;
        };
        let mut files: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| is_image(p))
            .collect();
        files.sort();
        for p in files {
            let Some(stem) = p.file_stem().map(|s| s.to_string_lossy().to_string()) else {
                continue;
            };
            if !v.iter().any(|(n, _)| n.eq_ignore_ascii_case(&stem)) {
                v.push((stem, p));
            }
        }
    }
    v.sort_by(|a, b| {
        a.0.to_lowercase()
            .cmp(&b.0.to_lowercase())
            .then_with(|| a.0.cmp(&b.0))
    });
    v
}

/// Datei zum Namen in den Ordnern: Dateiname ohne Endung (ohne Rücksicht auf Groß-/Kleinschreibung,
/// erste Fundstelle in Leserichtung) oder mit Endung; ein vorhandener Pfad gilt direkt.
pub fn find_in(dirs: &[PathBuf], name: &str) -> Option<PathBuf> {
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    let direct = Path::new(name);
    if direct.is_absolute() && is_image(direct) {
        return Some(direct.to_path_buf());
    }
    for dir in dirs {
        let Ok(rd) = std::fs::read_dir(dir) else {
            continue;
        };
        let mut files: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| is_image(p))
            .collect();
        files.sort();
        let hit = files.into_iter().find(|p| {
            p.file_stem()
                .is_some_and(|s| s.to_string_lossy().eq_ignore_ascii_case(name))
                || p.file_name()
                    .is_some_and(|s| s.to_string_lossy().eq_ignore_ascii_case(name))
        });
        if hit.is_some() {
            return hit;
        }
    }
    None
}

/// Fluent-Schlüssel des angezeigten Namens einer Textur bzw. eines Musters (Dateiname ohne Endung):
/// `pi-texture-name-<slug>` bzw. `pi-pattern-name-<slug>` (Kleinbuchstaben, Ziffern, Bindestriche).
pub fn label_key(kind: &str, name: &str) -> String {
    let mut slug = String::new();
    for c in name.chars().flat_map(|c| c.to_lowercase()) {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    let k = if kind == PATTERNS {
        "pattern"
    } else {
        "texture"
    };
    format!("pi-{k}-name-{}", slug.trim_end_matches('-'))
}

/// Datei zu einem **angezeigten** Namen (z. B. „Ader“ für die Datei „Vein“): der Name in der
/// aktuellen Sprache oder einer der eingebauten bzw. geladenen Sprachen, ohne Rücksicht auf
/// Groß-/Kleinschreibung (CT-2: die Inspektorliste zeigt übersetzte Namen, Skripte und Agenten
/// nennen sie so).
fn find_by_label(dirs: &[PathBuf], kind: &str, name: &str) -> Option<PathBuf> {
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    let mut langs: Vec<String> = vec![fp_i18n::current_language()];
    for l in fp_i18n::languages() {
        if !langs.iter().any(|x| x.eq_ignore_ascii_case(&l.code)) {
            langs.push(l.code);
        }
    }
    let files = files_in(dirs);
    for lang in &langs {
        let hit = fp_i18n::with_language(lang, || {
            files.iter().find(|(stem, _)| {
                let key = label_key(kind, stem);
                let label = fp_i18n::t(&key);
                label != key && label.trim().eq_ignore_ascii_case(name)
            })
        });
        if let Some((_, p)) = hit {
            return Some(p.clone());
        }
    }
    None
}

/// Datei einer Textur nach ihrem Namen (Dateiname oder angezeigter Name).
pub fn texture_file(name: &str) -> Option<PathBuf> {
    let d = dirs(TEXTURES);
    find_in(&d, name).or_else(|| find_by_label(&d, TEXTURES, name))
}

/// Datei eines Musters nach seinem Namen (Dateiname oder angezeigter Name).
pub fn pattern_file(name: &str) -> Option<PathBuf> {
    let d = dirs(PATTERNS);
    find_in(&d, name).or_else(|| find_by_label(&d, PATTERNS, name))
}

/// Dateiname (ohne Endung) zu einem Textur- bzw. Musternamen, wenn eine Datei passt – so wird ein
/// angezeigter Name („Ader“) im Dokument als Dateiname („Vein“) gespeichert wie im Original.
pub fn file_name_of(kind: &str, name: &str) -> Option<String> {
    let p = if kind == PATTERNS {
        pattern_file(name)
    } else {
        texture_file(name)
    }?;
    if Path::new(name.trim()).is_absolute() {
        return None;
    }
    p.file_stem().map(|s| s.to_string_lossy().to_string())
}

/// Name, unter dem eine gefundene Datei im Dokument steht: der genannte Name, wenn er der
/// Dateiname ist, sonst (angezeigter Name) der Dateiname ohne Endung.
fn stored_name(name: &str, p: &Path) -> String {
    let n = name.trim();
    let is_file = p
        .file_stem()
        .is_some_and(|s| s.to_string_lossy().eq_ignore_ascii_case(n))
        || p.file_name()
            .is_some_and(|s| s.to_string_lossy().eq_ignore_ascii_case(n));
    if is_file || Path::new(n).is_absolute() {
        name.to_string()
    } else {
        p.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| name.to_string())
    }
}

fn image_of(path: &Path) -> Option<fp_raster::RgbaImage> {
    crate::commands::attrs::load_image_file(&path.to_string_lossy()).ok()
}

/// Textur mit Maske aus einer Bilddatei (Helligkeit = Maske); zwischengespeichert je Datei.
pub fn texture_from_file(path: &Path, name: &str) -> Option<fp_model::attrs::Texture> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, fp_model::MaskData>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let cached = cache.lock().ok().and_then(|c| c.get(path).cloned());
    let mask = match cached {
        Some(m) => m,
        None => {
            let m = crate::commands::attrs::texture_mask(&image_of(path)?);
            if let Ok(mut c) = cache.lock() {
                c.insert(path.to_path_buf(), m.clone());
            }
            m
        }
    };
    Some(fp_model::attrs::Texture {
        name: name.to_string(),
        user_name: Some(name.to_string()),
        mask: Some(mask),
        extra: Default::default(),
    })
}

/// Textur nach Namen aus den Texturordnern (mit Maske); `None`, wenn keine Datei passt.
pub fn load_texture(name: &str) -> Option<fp_model::attrs::Texture> {
    let p = texture_file(name)?;
    texture_from_file(&p, &stored_name(name, &p))
}

/// Muster mit Bild aus einer Datei; zwischengespeichert je Datei.
pub fn pattern_from_file(path: &Path, name: &str) -> Option<fp_model::attrs::Pattern> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Arc<fp_model::element::BitmapNode>>>> =
        OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let cached = cache.lock().ok().and_then(|c| c.get(path).cloned());
    let node = match cached {
        Some(n) => n,
        None => {
            let img = image_of(path)?;
            // feste Kachelnummern: der Schreiber nummeriert ohnehin neu, so bleibt das Ergebnis reproduzierbar
            let mut next = 1u32;
            let n = Arc::new(fp_model::element::BitmapNode::from_rgba(
                fp_geom::Point::new(0.0, 0.0),
                img.width() as u32,
                img.height() as u32,
                img.as_bytes(),
                &mut next,
            ));
            if let Ok(mut c) = cache.lock() {
                c.insert(path.to_path_buf(), Arc::clone(&n));
            }
            n
        }
    };
    Some(fp_model::attrs::Pattern {
        name: name.to_string(),
        user_name: Some(name.to_string()),
        image: Some(node),
        extra: Default::default(),
    })
}

/// Muster nach Namen aus den Musterordnern (mit Bild); `None`, wenn keine Datei passt.
pub fn load_pattern(name: &str) -> Option<fp_model::attrs::Pattern> {
    let p = pattern_file(name)?;
    pattern_from_file(&p, &stored_name(name, &p))
}

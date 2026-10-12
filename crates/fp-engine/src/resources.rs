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
    p.is_file() && p.extension().is_some_and(|x| IMAGE_EXTS.iter().any(|e| x.eq_ignore_ascii_case(e)))
}

/// Bilddateien der Ordner (Name = Dateiname ohne Endung, Pfad), je Name nur die erste Fundstelle
/// (ohne Rücksicht auf Groß-/Kleinschreibung), nach Namen geordnet. Fehlende Ordner zählen nicht.
pub fn files_in(dirs: &[PathBuf]) -> Vec<(String, PathBuf)> {
    let mut v: Vec<(String, PathBuf)> = Vec::new();
    for dir in dirs {
        let Ok(rd) = std::fs::read_dir(dir) else { continue };
        let mut files: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| is_image(p)).collect();
        files.sort();
        for p in files {
            let Some(stem) = p.file_stem().map(|s| s.to_string_lossy().to_string()) else { continue };
            if !v.iter().any(|(n, _)| n.eq_ignore_ascii_case(&stem)) {
                v.push((stem, p));
            }
        }
    }
    v.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()).then_with(|| a.0.cmp(&b.0)));
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
        let Ok(rd) = std::fs::read_dir(dir) else { continue };
        let mut files: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| is_image(p)).collect();
        files.sort();
        let hit = files.into_iter().find(|p| {
            p.file_stem().is_some_and(|s| s.to_string_lossy().eq_ignore_ascii_case(name)) || p.file_name().is_some_and(|s| s.to_string_lossy().eq_ignore_ascii_case(name))
        });
        if hit.is_some() {
            return hit;
        }
    }
    None
}

/// Datei einer Textur nach ihrem Namen.
pub fn texture_file(name: &str) -> Option<PathBuf> {
    find_in(&dirs(TEXTURES), name)
}

/// Datei eines Musters nach seinem Namen.
pub fn pattern_file(name: &str) -> Option<PathBuf> {
    find_in(&dirs(PATTERNS), name)
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
    Some(fp_model::attrs::Texture { name: name.to_string(), user_name: Some(name.to_string()), mask: Some(mask), extra: Default::default() })
}

/// Textur nach Namen aus den Texturordnern (mit Maske); `None`, wenn keine Datei passt.
pub fn load_texture(name: &str) -> Option<fp_model::attrs::Texture> {
    texture_from_file(&texture_file(name)?, name)
}

/// Muster mit Bild aus einer Datei; zwischengespeichert je Datei.
pub fn pattern_from_file(path: &Path, name: &str) -> Option<fp_model::attrs::Pattern> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Arc<fp_model::element::BitmapNode>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let cached = cache.lock().ok().and_then(|c| c.get(path).cloned());
    let node = match cached {
        Some(n) => n,
        None => {
            let img = image_of(path)?;
            // feste Kachelnummern: der Schreiber nummeriert ohnehin neu, so bleibt das Ergebnis reproduzierbar
            let mut next = 1u32;
            let n = Arc::new(fp_model::element::BitmapNode::from_rgba(fp_geom::Point::new(0.0, 0.0), img.width() as u32, img.height() as u32, img.as_bytes(), &mut next));
            if let Ok(mut c) = cache.lock() {
                c.insert(path.to_path_buf(), Arc::clone(&n));
            }
            n
        }
    };
    Some(fp_model::attrs::Pattern { name: name.to_string(), user_name: Some(name.to_string()), image: Some(node), extra: Default::default() })
}

/// Muster nach Namen aus den Musterordnern (mit Bild); `None`, wenn keine Datei passt.
pub fn load_pattern(name: &str) -> Option<fp_model::attrs::Pattern> {
    pattern_from_file(&pattern_file(name)?, name)
}

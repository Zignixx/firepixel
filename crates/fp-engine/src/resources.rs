//! Texturen und Muster aus den Ressourcenordnern, nur zur Laufzeit gelesen (nichts eingebettet).
//!
//! Leserichtung je Art (`Textures` bzw. `Patterns`): der Ressourcenordner des Originals
//! (`Configuration\<Art>`), dessen Vorlage für den Benutzerordner (`First Run\<Art>`; Ordner aus
//! `fp_color::install`) und der eigene Benutzerordner `%APPDATA%\Firepixel\<Art>`
//! (`fp_i18n::profile`). Dieselben Ordner zeigt der Inspektor in seinen Listen; eine Textur bzw.
//! ein Muster, das nur per Name gesetzt wird (`pi.fill {texture}`, Werkzeugvorgaben, Skripte),
//! lädt hier die Datei zum Namen, damit die Füllung sichtbar wirkt.
//!
//! Patterns additionally have a small set of **built-in** patterns drawn in code (FB-20, TW-198):
//! without the reference application's folders (macOS, Linux, a fresh Windows machine) the pattern
//! list would otherwise be empty and switching a fill to "Pattern" failed. Built-in patterns have a
//! pseudo path below [`BUILTIN_DIR`] that never exists on disk; [`pattern_image`] draws them.

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
    find_by_label_in(&files_in(dirs), kind, name)
}

/// [`find_by_label`] over a ready list of (file name, path) entries.
fn find_by_label_in(files: &[(String, PathBuf)], kind: &str, name: &str) -> Option<PathBuf> {
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

/// Datei eines Musters nach seinem Namen (Dateiname oder angezeigter Name); built-in patterns
/// answer with their pseudo path ([`builtin_pattern_path`]).
pub fn pattern_file(name: &str) -> Option<PathBuf> {
    pattern_file_in(&dirs(PATTERNS), name)
}

/// [`pattern_file`] with explicit pattern folders: a file in the folders (file name, case-insensitive,
/// with or without extension), then a built-in pattern of that name, then a shown (translated) name
/// of either.
pub fn pattern_file_in(dirs: &[PathBuf], name: &str) -> Option<PathBuf> {
    find_in(dirs, name)
        .or_else(|| builtin_pattern_file(name))
        .or_else(|| find_by_label_in(&pattern_entries_in(dirs), PATTERNS, name))
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

/// Image of a pattern file: a built-in pattern is drawn ([`BUILTIN_PATTERNS`]), any other path is
/// read from disk.
pub fn pattern_image(path: &Path) -> Option<fp_raster::RgbaImage> {
    match builtin_of(path) {
        Some(b) => b.image(),
        None => image_of(path),
    }
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
            let img = pattern_image(path)?;
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

/// Pattern by name like [`load_pattern`]; when no pattern of that name exists (e.g. the default
/// pattern of the reference application on a machine without its folders), the default pattern
/// ([`default_pattern_name`]) is used instead. Returns the pattern and whether it is a stand-in.
pub fn load_pattern_or_default(name: &str) -> Option<(fp_model::attrs::Pattern, bool)> {
    if let Some(p) = load_pattern(name) {
        return Some((p, false));
    }
    let d = default_pattern_name()?;
    load_pattern(&d).map(|p| (p, true))
}

// ---------------------------------------------------------------------- built-in patterns

/// First component of the pseudo path of a built-in pattern (never a real folder: `<` and `>`
/// are not allowed in Windows file names).
pub const BUILTIN_DIR: &str = "<built-in>";

/// A pattern drawn in code: name (technical, English) and tile.
#[derive(Clone, Copy, Debug)]
pub struct BuiltinPattern {
    /// Name (also the file name of its pseudo path).
    pub name: &'static str,
    w: u32,
    h: u32,
    px: fn(u32, u32) -> [u8; 4],
}

impl BuiltinPattern {
    /// The tile as an image.
    pub fn image(&self) -> Option<fp_raster::RgbaImage> {
        let mut bytes = Vec::with_capacity((self.w * self.h * 4) as usize);
        for y in 0..self.h {
            for x in 0..self.w {
                bytes.extend_from_slice(&(self.px)(x, y));
            }
        }
        fp_raster::RgbaImage::from_rgba_bytes(
            fp_geom::IRect::new(0, 0, self.w as i32, self.h as i32),
            &bytes,
        )
        .ok()
    }
}

const fn rgb(c: u32) -> [u8; 4] {
    [(c >> 16) as u8, (c >> 8) as u8, c as u8, 255]
}

/// Mix of two colours, `t` = share of `b` (0..1).
fn mix(a: u32, b: u32, t: f32) -> [u8; 4] {
    let (a, b, t) = (rgb(a), rgb(b), t.clamp(0.0, 1.0));
    let m = |i: usize| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t).round() as u8;
    [m(0), m(1), m(2), 255]
}

fn checkerboard(x: u32, y: u32) -> [u8; 4] {
    if (x / 8 + y / 8).is_multiple_of(2) {
        rgb(0xc8c8c8)
    } else {
        rgb(0xffffff)
    }
}

fn diagonal_stripes(x: u32, y: u32) -> [u8; 4] {
    if (x + y) % 16 < 6 {
        rgb(0x3a6ea5)
    } else {
        rgb(0xdfe9f5)
    }
}

fn polka_dots(x: u32, y: u32) -> [u8; 4] {
    let (dx, dy) = (x as f32 + 0.5 - 12.0, y as f32 + 0.5 - 12.0);
    let cover = 5.5 - (dx * dx + dy * dy).sqrt();
    mix(0xfbe9e9, 0xd04a4a, cover + 0.5)
}

fn grid(x: u32, y: u32) -> [u8; 4] {
    if x == 0 || y == 0 {
        rgb(0x7f8c99)
    } else {
        rgb(0xffffff)
    }
}

fn bricks(x: u32, y: u32) -> [u8; 4] {
    let joint = if y < 8 { 0 } else { 16 };
    if y.is_multiple_of(8) || x == joint {
        rgb(0xd9d0c1)
    } else {
        rgb(0xb5523b)
    }
}

fn waves(x: u32, y: u32) -> [u8; 4] {
    let c = 8.0 + 3.0 * (std::f32::consts::TAU * (x as f32 + 0.5) / 32.0).sin();
    let d = (y as f32 + 0.5 - c).abs();
    mix(0xe6f2fa, 0x2f7fb8, 2.0 - d)
}

fn diamonds(x: u32, y: u32) -> [u8; 4] {
    let d = (x as f32 + 0.5 - 8.0).abs() + (y as f32 + 0.5 - 8.0).abs();
    if d < 6.5 {
        rgb(0xc9b8e8)
    } else {
        rgb(0x6b4fa0)
    }
}

fn crosshatch(x: u32, y: u32) -> [u8; 4] {
    if (x + y).is_multiple_of(12) || (x + 12 - y).is_multiple_of(12) {
        rgb(0x404040)
    } else {
        rgb(0xffffff)
    }
}

/// The built-in patterns (all tiles repeat seamlessly).
pub const BUILTIN_PATTERNS: [BuiltinPattern; 8] = [
    BuiltinPattern {
        name: "Checkerboard",
        w: 16,
        h: 16,
        px: checkerboard,
    },
    BuiltinPattern {
        name: "Diagonal Stripes",
        w: 16,
        h: 16,
        px: diagonal_stripes,
    },
    BuiltinPattern {
        name: "Polka Dots",
        w: 24,
        h: 24,
        px: polka_dots,
    },
    BuiltinPattern {
        name: "Grid",
        w: 16,
        h: 16,
        px: grid,
    },
    BuiltinPattern {
        name: "Bricks",
        w: 32,
        h: 16,
        px: bricks,
    },
    BuiltinPattern {
        name: "Waves",
        w: 32,
        h: 16,
        px: waves,
    },
    BuiltinPattern {
        name: "Diamonds",
        w: 16,
        h: 16,
        px: diamonds,
    },
    BuiltinPattern {
        name: "Crosshatch",
        w: 12,
        h: 12,
        px: crosshatch,
    },
];

/// Pseudo path of a built-in pattern (`<built-in>/Patterns/<name>.png`).
pub fn builtin_pattern_path(name: &str) -> PathBuf {
    Path::new(BUILTIN_DIR)
        .join(PATTERNS)
        .join(format!("{name}.png"))
}

/// The built-in pattern behind a pseudo path, if it is one.
pub fn builtin_of(path: &Path) -> Option<&'static BuiltinPattern> {
    let mut c = path.components();
    if c.next()?.as_os_str() != BUILTIN_DIR || c.next()?.as_os_str() != PATTERNS {
        return None;
    }
    let stem = path.file_stem()?.to_string_lossy();
    BUILTIN_PATTERNS
        .iter()
        .find(|b| b.name.eq_ignore_ascii_case(&stem))
}

/// Pseudo path of the built-in pattern with this name (case-insensitive, `.png` allowed).
fn builtin_pattern_file(name: &str) -> Option<PathBuf> {
    let n = name.trim();
    let n = n
        .len()
        .checked_sub(4)
        .filter(|&i| n.is_char_boundary(i) && n[i..].eq_ignore_ascii_case(".png"))
        .map_or(n, |i| &n[..i]);
    BUILTIN_PATTERNS
        .iter()
        .find(|b| b.name.eq_ignore_ascii_case(n))
        .map(|b| builtin_pattern_path(b.name))
}

/// All patterns: the files of the pattern folders plus the built-in patterns (a file of the same
/// name wins), ordered by name like [`files_in`].
pub fn pattern_entries_in(dirs: &[PathBuf]) -> Vec<(String, PathBuf)> {
    let mut v = files_in(dirs);
    for b in &BUILTIN_PATTERNS {
        if !v.iter().any(|(n, _)| n.eq_ignore_ascii_case(b.name)) {
            v.push((b.name.to_string(), builtin_pattern_path(b.name)));
        }
    }
    v.sort_by(|a, b| {
        a.0.to_lowercase()
            .cmp(&b.0.to_lowercase())
            .then_with(|| a.0.cmp(&b.0))
    });
    v
}

/// [`pattern_entries_in`] for the pattern folders of this machine.
pub fn pattern_entries() -> Vec<(String, PathBuf)> {
    pattern_entries_in(&dirs(PATTERNS))
}

/// Default pattern with explicit folders: the reference application's default ("Cotton") when a
/// file of that name exists, otherwise the first available pattern.
pub fn default_pattern_name_in(dirs: &[PathBuf]) -> Option<String> {
    if let Some(p) = find_in(dirs, crate::commands::panel::DEFAULT_PATTERN) {
        return p.file_stem().map(|s| s.to_string_lossy().to_string());
    }
    pattern_entries_in(dirs).into_iter().next().map(|(n, _)| n)
}

/// Default pattern of this machine ([`default_pattern_name_in`]), determined once.
pub fn default_pattern_name() -> Option<String> {
    static N: OnceLock<Option<String>> = OnceLock::new();
    N.get_or_init(|| default_pattern_name_in(&dirs(PATTERNS)))
        .clone()
}

//! Ressourcenordner des Originals (Installation des Referenzprogramms), nur zur Laufzeit gelesen.
//!
//! Firepixel bettet keine Daten des Originals ein. Wo es sie zur Laufzeit nutzen kann (CMYK-Tabelle,
//! JPEG-Ankertabellen, Texturnamen, Bibliotheken), sucht es den Ordner in dieser Reihenfolge:
//!
//! 1. Voreinstellung „Ressourcenordner des Originals“ ([`set_configured_dir`], gesetzt von der
//!    Oberfläche; leer = automatische Suche),
//! 2. Umgebungsvariable [`ENV_DIR`] (`FP_ORIGINAL_DIR`),
//! 3. automatische Suche in den Programme-Ordnern (`%ProgramFiles%`, `%ProgramFiles(x86)%`,
//!    `%ProgramW6432%`, je bis zu zwei Ebenen tief).
//!
//! Erkannt wird der Ordner nur an seinem **Inhalt**, nicht am Namen: ein Unterordner
//! `Configuration\Textures` und eine EXE im Ordner, die die Ressource `CMYK/1001`
//! (9⁴-Gittertabelle, 14 §1.6) enthält ([`is_original_dir`]).

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Umgebungsvariable mit dem Ressourcenordner des Originals.
pub const ENV_DIR: &str = "FP_ORIGINAL_DIR";

static CONFIGURED: Mutex<Option<PathBuf>> = Mutex::new(None);
static AUTO: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Setzt den Ordner aus der Voreinstellung (`None` oder leer = automatische Suche).
pub fn set_configured_dir(dir: Option<PathBuf>) {
    let dir = dir.filter(|d| !d.as_os_str().is_empty());
    if let Ok(mut g) = CONFIGURED.lock() {
        *g = dir;
    }
}

/// Der per Voreinstellung gesetzte Ordner (ungeprüft).
pub fn configured_dir() -> Option<PathBuf> {
    CONFIGURED.lock().ok().and_then(|g| g.clone())
}

/// Der wirksame Ressourcenordner: Voreinstellung, dann [`ENV_DIR`], dann automatische Suche.
/// Eine gesetzte Voreinstellung bzw. Umgebungsvariable gilt, wenn der Ordner existiert.
pub fn original_dir() -> Option<PathBuf> {
    if let Some(d) = configured_dir()
        && d.is_dir()
    {
        return Some(d);
    }
    if let Some(d) = std::env::var_os(ENV_DIR).map(PathBuf::from)
        && d.is_dir()
    {
        return Some(d);
    }
    AUTO.get_or_init(auto_search).clone()
}

/// Die Haupt-EXE im Ressourcenordner (die EXE mit der Ressource `CMYK/1001`); je Ordner
/// zwischengespeichert.
pub fn original_exe() -> Option<PathBuf> {
    static CACHE: Mutex<Option<(PathBuf, Option<PathBuf>)>> = Mutex::new(None);
    let dir = original_dir()?;
    if let Ok(g) = CACHE.lock()
        && let Some((d, exe)) = g.as_ref()
        && *d == dir
    {
        return exe.clone();
    }
    let exe = main_exe_in(&dir);
    if let Ok(mut g) = CACHE.lock() {
        *g = Some((dir, exe.clone()));
    }
    exe
}

/// Programmname des Originals, wie ihn der Ressourcenordner liefert (Dateiname der Haupt-EXE ohne
/// Endung). Dient nur der Kompatibilität mit Skripten des Originals (fp-script); `None` ohne Ordner.
pub fn original_app_name() -> Option<String> {
    let exe = original_exe()?;
    let stem = exe.file_stem()?.to_str()?.to_string();
    (!stem.is_empty() && stem.chars().all(|c| c.is_ascii_alphanumeric())).then_some(stem)
}

/// Unterordner des Ressourcenordners, falls vorhanden (z. B. `Configuration\Textures`).
pub fn original_subdir(rel: &str) -> Option<PathBuf> {
    let p = original_dir()?.join(rel);
    p.is_dir().then_some(p)
}

/// Erkennt den Ressourcenordner am Inhalt: `Configuration\Textures` und eine EXE mit `CMYK/1001`.
pub fn is_original_dir(dir: &Path) -> bool {
    dir.join("Configuration").join("Textures").is_dir() && main_exe_in(dir).is_some()
}

/// Datei mit der Ressource `CMYK/1001` im Ordner: die Sprach-Ressourcen-DLL
/// (`<Sprache>\Resources\*.dll`, so beim installierten Original und im Testkorpus; Befund RF
/// 11.10.2026) oder die größte EXE selbst.
pub fn cmyk_resource_file(dir: &Path) -> Option<PathBuf> {
    let has = |p: &Path| std::fs::read(p).is_ok_and(|d| find_resource(&d, "CMYK", 1001).is_some());
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for lang in rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
            if let Ok(rr) = std::fs::read_dir(lang.join("Resources")) {
                cands.extend(rr.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("dll"))));
            }
        }
    }
    cands.sort();
    if let Some(p) = cands.into_iter().find(|p| has(p)) {
        return Some(p);
    }
    largest_exe(dir).filter(|p| has(p))
}

/// Größte EXE im Ordner (die Haupt-EXE ist mit Abstand die größte Datei).
fn largest_exe(dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("exe")))
        .filter_map(|p| Some((std::fs::metadata(&p).ok()?.len(), p)))
        .max_by_key(|e| e.0)
        .map(|e| e.1)
}

/// Haupt-EXE des Originals im Ordner (auch für Installationskopien wie den Testkorpus): die größte
/// EXE, sofern der Ordner die Ressource `CMYK/1001` liefert ([`cmyk_resource_file`]).
pub fn main_exe_in(dir: &Path) -> Option<PathBuf> {
    cmyk_resource_file(dir)?;
    largest_exe(dir)
}

fn auto_search() -> Option<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    for var in ["ProgramFiles(x86)", "ProgramFiles", "ProgramW6432"] {
        if let Some(v) = std::env::var_os(var) {
            let p = PathBuf::from(v);
            if !roots.contains(&p) {
                roots.push(p);
            }
        }
    }
    for root in roots {
        let Ok(level1) = std::fs::read_dir(&root) else { continue };
        for d1 in level1.flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
            if is_original_dir(&d1) {
                return Some(d1);
            }
            let Ok(level2) = std::fs::read_dir(&d1) else { continue };
            for d2 in level2.flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
                if is_original_dir(&d2) {
                    return Some(d2);
                }
            }
        }
    }
    None
}

fn u16at(d: &[u8], o: usize) -> Option<u16> {
    d.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]))
}
fn u32at(d: &[u8], o: usize) -> Option<u32> {
    d.get(o..o + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Liest eine Ressource (`typ` als Name, `id` als Zahl) aus einer PE-Datei.
pub fn find_resource(exe: &[u8], typ: &str, id: u32) -> Option<Vec<u8>> {
    if exe.get(..2)? != b"MZ" {
        return None;
    }
    let pe = u32at(exe, 0x3C)? as usize;
    if exe.get(pe..pe + 4)? != b"PE\0\0" {
        return None;
    }
    let nsec = u16at(exe, pe + 6)? as usize;
    let opt_size = u16at(exe, pe + 20)? as usize;
    let opt = pe + 24;
    let magic = u16at(exe, opt)?;
    let dd = if magic == 0x20B { opt + 112 } else { opt + 96 };
    let rsrc_rva = u32at(exe, dd + 2 * 8)?;
    let mut secs: Vec<(u32, u32, u32, u32)> = Vec::new();
    for i in 0..nsec.min(96) {
        let o = opt + opt_size + i * 40;
        secs.push((u32at(exe, o + 12)?, u32at(exe, o + 8)?, u32at(exe, o + 20)?, u32at(exe, o + 16)?));
    }
    let offset = |rva: u32| -> Option<usize> {
        secs.iter().find(|&&(va, vs, _, rs)| rva >= va && rva < va + vs.max(rs)).and_then(|&(va, _, raw, rs)| {
            let o = rva - va;
            (o < rs).then_some((raw + o) as usize)
        })
    };
    let base = offset(rsrc_rva)?;
    let entries = |dir: usize| -> Vec<(u32, u32)> {
        let n = u16at(exe, dir + 12).unwrap_or(0) as usize + u16at(exe, dir + 14).unwrap_or(0) as usize;
        (0..n.min(4096)).filter_map(|k| Some((u32at(exe, dir + 16 + k * 8)?, u32at(exe, dir + 20 + k * 8)?))).collect()
    };
    let name_of = |v: u32| -> Option<String> {
        if v & 0x8000_0000 == 0 {
            return None;
        }
        let o = base + (v & 0x7FFF_FFFF) as usize;
        let len = u16at(exe, o)? as usize;
        let units: Vec<u16> = (0..len).filter_map(|i| u16at(exe, o + 2 + i * 2)).collect();
        Some(String::from_utf16_lossy(&units))
    };
    let (_, t_off) = entries(base).into_iter().find(|&(n, _)| name_of(n).is_some_and(|s| s.eq_ignore_ascii_case(typ)))?;
    let dir2 = base + (t_off & 0x7FFF_FFFF) as usize;
    let (_, i_off) = entries(dir2).into_iter().find(|&(n, _)| n == id)?;
    let mut off = i_off;
    // bis zum Datenblatt absteigen (Sprachebene)
    for _ in 0..4 {
        if off & 0x8000_0000 == 0 {
            break;
        }
        let d = base + (off & 0x7FFF_FFFF) as usize;
        off = entries(d).first()?.1;
    }
    let de = base + off as usize;
    let (rva, size) = (u32at(exe, de)?, u32at(exe, de + 4)? as usize);
    let o = offset(rva)?;
    exe.get(o..o + size).map(|s| s.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn garbage_is_none() {
        assert!(find_resource(b"MZ", "CMYK", 1001).is_none());
        assert!(find_resource(b"", "CMYK", 1001).is_none());
        assert!(!is_original_dir(Path::new("gibt-es-nicht")));
    }

    #[test]
    fn configured_dir_wins_if_present() {
        let tmp = std::env::temp_dir();
        set_configured_dir(Some(tmp.clone()));
        assert_eq!(original_dir(), Some(tmp));
        set_configured_dir(Some(PathBuf::new()));
        assert_eq!(configured_dir(), None);
    }
}

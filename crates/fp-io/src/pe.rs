//! Laufzeit-Daten aus dem Ressourcenordner des Originals (werden **nicht** eingebettet):
//! die CMYK-Gittertabelle (Ressource `CMYK/1001` der Haupt-EXE, 14 §1.6).
//!
//! Suche: Umgebungsvariable `FP_CMYK_TABLE` (Pfad zur `.bin`-Rohdatei oder zur EXE), dann der
//! Ressourcenordner des Originals ([`fp_color::install`]: Voreinstellung, `FP_ORIGINAL_DIR`,
//! automatische Suche am Inhalt).

use fp_color::cmyk::{CmykTable, CMYK_TABLE_BYTES};

pub use fp_color::install::find_resource;

/// CMYK-Tabelle aus einer Datei (Rohdaten oder die Haupt-EXE des Originals).
pub fn load_cmyk_table(path: &std::path::Path) -> Option<CmykTable> {
    let data = std::fs::read(path).ok()?;
    if data.starts_with(b"MZ") {
        let r = find_resource(&data, "CMYK", 1001)?;
        CmykTable::from_bytes(&r).ok()
    } else if data.len() >= CMYK_TABLE_BYTES {
        CmykTable::from_bytes(&data).ok()
    } else {
        None
    }
}

/// CMYK-Tabelle aus `FP_CMYK_TABLE` oder dem Ressourcenordner des Originals.
pub fn load_cmyk_table_default() -> Option<CmykTable> {
    let mut paths: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(p) = std::env::var("FP_CMYK_TABLE") {
        paths.push(p.into());
    }
    paths.extend(fp_color::install::original_dir().and_then(|d| fp_color::install::cmyk_resource_file(&d)));
    paths.iter().find_map(|p| load_cmyk_table(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn garbage_is_none() {
        assert!(find_resource(b"MZ", "CMYK", 1001).is_none());
        assert!(find_resource(b"", "CMYK", 1001).is_none());
        assert!(load_cmyk_table(std::path::Path::new("gibt-es-nicht.bin")).is_none());
    }
}

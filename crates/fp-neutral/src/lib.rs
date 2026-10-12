//! Neutralitätsprüfung (Nutzerwunsch, `docs\AGENTEN_REGELN.md` §1a): Unter `firepixel\` darf weder
//! der Name des nachgebauten Originals noch der seines Herstellers stehen – in keiner Schreibweise,
//! weder in Quelltext, Kommentaren, Daten, Testdaten, Dokumenten noch in Datei- und Ordnernamen.
//!
//! Damit die Prüfung selbst die Regel erfüllt, kennt sie die beiden Wörter nur als Länge und zwei
//! Prüfsummen (Rabin-Karp-Rollhash über Kleinbuchstaben zum Finden, FNV-1a 64 zur Bestätigung).
//! Geprüft werden Bytes (UTF-8/ASCII) und UTF-16LE (jedes zweite Byte 0), ohne Rücksicht auf
//! Groß-/Kleinschreibung. Ausgenommen sind nur die erzeugten Build-Ordner `target*` im Projektordner.

use std::path::{Path, PathBuf};

/// Ein verbotenes Wort: Länge, Rollhash (Basis 257, wrapping u64) und FNV-1a 64 der Kleinschreibung.
#[derive(Clone, Copy, Debug)]
pub struct Term {
    pub len: usize,
    pub rolling: u64,
    pub fnv: u64,
}

/// Die beiden verbotenen Wörter.
pub const TERMS: [Term; 2] = [
    Term { len: 5, rolling: 0x0062_ebe4_f1fb, fnv: 0xf81f_30fd_e4da_1bb0 },
    Term { len: 9, rolling: 0xa79b_3190_f2fd_2fdc, fnv: 0x19f3_7001_e0cc_8fc5 },
];

const BASE: u64 = 257;

fn fnv(bytes: impl Iterator<Item = u8>) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Fundstellen (Byte-Offsets) eines Wortes in einer Folge bereits kleingeschriebener Zeichen.
fn find_term(seq: &[u8], t: &Term, out: &mut Vec<usize>) {
    if seq.len() < t.len {
        return;
    }
    let pow = (1..t.len).fold(1u64, |p, _| p.wrapping_mul(BASE));
    let mut h: u64 = 0;
    for &b in &seq[..t.len] {
        h = h.wrapping_mul(BASE).wrapping_add(b as u64);
    }
    let mut i = 0;
    loop {
        if h == t.rolling && fnv(seq[i..i + t.len].iter().copied()) == t.fnv {
            out.push(i);
        }
        if i + t.len >= seq.len() {
            break;
        }
        h = h.wrapping_sub((seq[i] as u64).wrapping_mul(pow)).wrapping_mul(BASE).wrapping_add(seq[i + t.len] as u64);
        i += 1;
    }
}

/// Byte-Offsets aller Treffer in `data` (UTF-8/ASCII und UTF-16LE, ohne Groß-/Kleinschreibung).
pub fn find_in_bytes(data: &[u8]) -> Vec<usize> {
    let mut hits = Vec::new();
    let lower: Vec<u8> = data.iter().map(|b| b.to_ascii_lowercase()).collect();
    for t in &TERMS {
        find_term(&lower, t, &mut hits);
    }
    // UTF-16LE: jedes zweite Byte (beide Ausrichtungen), nur Stellen mit 0 im Folgebyte zählen
    for start in 0..2 {
        let narrow: Vec<u8> = lower.iter().skip(start).step_by(2).copied().collect();
        let zero_ok: Vec<bool> = data.iter().skip(start + 1).step_by(2).map(|&b| b == 0).collect();
        let mut tmp = Vec::new();
        for t in &TERMS {
            find_term(&narrow, t, &mut tmp);
            for &k in &tmp {
                if (k..k + t.len).all(|j| zero_ok.get(j).copied().unwrap_or(false)) {
                    hits.push(start + 2 * k);
                }
            }
            tmp.clear();
        }
    }
    hits.sort_unstable();
    hits.dedup();
    hits
}

/// Ein Treffer: Datei (relativ), Zeile (1-basiert; 0 = im Datei- oder Ordnernamen).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub path: PathBuf,
    pub line: usize,
}

impl std::fmt::Display for Finding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.line == 0 { write!(f, "{} (Name)", self.path.display()) } else { write!(f, "{}:{}", self.path.display(), self.line) }
    }
}

/// Prüft alle Dateien und Namen unter `root` (ohne `target*` und die Versionsverwaltung `.git`
/// direkt unter `root`).
pub fn scan(root: &Path) -> Vec<Finding> {
    scan_with_files(root).0
}

/// Wie [`scan`], liefert zusätzlich alle geprüften Dateien (relativ), damit Tests die Abdeckung
/// nachweisen können (STATUS.md, `docs\*.md`, `docs\berichte`, `docs\funktionstest`, Bilder).
pub fn scan_with_files(root: &Path) -> (Vec<Finding>, Vec<PathBuf>) {
    let mut out = Vec::new();
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            if dir == root && (name.starts_with("target") || name == ".git") {
                continue;
            }
            let rel = p.strip_prefix(root).unwrap_or(&p).to_path_buf();
            if !find_in_bytes(name.as_bytes()).is_empty() {
                out.push(Finding { path: rel.clone(), line: 0 });
            }
            if p.is_dir() {
                stack.push(p);
            } else if let Ok(data) = std::fs::read(&p) {
                files.push(rel.clone());
                let mut last = None;
                for off in find_in_bytes(&data) {
                    let line = 1 + data[..off].iter().filter(|&&b| b == b'\n').count();
                    if last != Some(line) {
                        out.push(Finding { path: rel.clone(), line });
                        last = Some(line);
                    }
                }
            }
        }
    }
    (out, files)
}

/// FNV-1a 64 über beliebige Bytes (z. B. als Schlüssel eines Prüf-Zwischenspeichers).
pub fn fnv64(data: &[u8]) -> u64 {
    fnv(data.iter().copied())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_text_has_no_hits() {
        assert!(find_in_bytes("Firepixel, FW-PNG, das Original".as_bytes()).is_empty());
        let u16: Vec<u8> = "Firepixel".encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        assert!(find_in_bytes(&u16).is_empty());
    }
}

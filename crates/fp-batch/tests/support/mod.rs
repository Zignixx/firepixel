//! Hilfen der fp-batch-Tests: Temp-Ordner, Testbilder, Prüfsummen.
#![allow(dead_code)]

use fp_batch::item::{still_from_rgba, Item};
use fp_batch::output::{encode, OutputFormat};
use std::path::{Path, PathBuf};

/// Frischer Temp-Ordner je Test (im System-Temp, nie im Profil).
pub fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fp-batch-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("temp dir");
    d
}

/// RGBA-Testbild w×h: Verlauf, optional mit transparentem Rand.
pub fn rgba(w: u32, h: u32, alpha: bool) -> Vec<u8> {
    let mut v = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let a = if alpha && (x == 0 || y == 0) { 0 } else { 255 };
            v.extend_from_slice(&[(x * 255 / w.max(1)) as u8, (y * 255 / h.max(1)) as u8, 128, a]);
        }
    }
    v
}

/// Schreibt ein PNG w×h.
pub fn write_png(path: &Path, w: u32, h: u32) {
    let item = still_from_rgba(w, h, &rgba(w, h, false)).expect("item");
    let (bytes, _) = encode(&item, OutputFormat::Png, &Default::default()).expect("png");
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).expect("dir");
    }
    std::fs::write(path, bytes).expect("write png");
}

/// Dekodiert eine Datei und liefert die Maße.
pub fn dims(path: &Path) -> (u32, u32) {
    let data = std::fs::read(path).expect("read");
    let item = fp_batch::item::decode_bytes(&path.to_string_lossy(), &data).expect("decode");
    item.dimensions()
}

/// Erstes Bild einer Datei.
pub fn pixels(path: &Path) -> Vec<u8> {
    let data = std::fs::read(path).expect("read");
    let item: Item = fp_batch::item::decode_bytes(&path.to_string_lossy(), &data).expect("decode");
    item.preview_image().expect("image").as_bytes().to_vec()
}

/// Einfache Prüfsumme (FNV-1a) einer Datei.
pub fn checksum(path: &Path) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in std::fs::read(path).expect("read") {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Alle Dateien unter `dir` (rekursiv).
pub fn files_under(dir: &Path) -> Vec<PathBuf> {
    walk(dir)
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(walk(&p));
            } else {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// Gibt es irgendwo eine Temporärdatei `.fptmp`?
pub fn has_temp_files(dir: &Path) -> bool {
    walk(dir).iter().any(|p| p.extension().is_some_and(|e| e == "fptmp"))
}

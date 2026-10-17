//! Gemeinsame Helfer der Korpus- und Laufzeittests von fp-render.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use fp_model::Document;
use fp_raster::{IRect, RgbaImage};

/// Wurzel `C:\Users\Administrator\Desktop\REA`.
pub fn rea() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
}

/// Alle FW-PNGs des Korpus (Installationskopie `..\korpus` bzw. `FP_CORPUS_DIR`), sortiert.
pub fn corpus_files() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let dir = std::env::var_os("FP_CORPUS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| rea().join("korpus"));
    walk(&dir, &mut out);
    out.sort();
    out
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut ents: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    ents.sort();
    for p in ents {
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")) {
            out.push(p);
        }
    }
}

/// Liest eine Datei als Dokument (mit flachem Bild); `None` für normale PNGs oder Fehler.
pub fn load(p: &Path) -> Option<Document> {
    let bytes = std::fs::read(p).ok()?;
    if !fp_fwpng::is_fwpng(&bytes) {
        return None;
    }
    fp_fwpng::read(&bytes).ok()
}

/// Aktive Seite wie `symrender.active_pdc`: Seite, deren Struktur (Größe, Ursprung, Folge der
/// Elementarten und Symbol-IDs der nicht-Web-Ebenen) der `MKBv`-Kopie gleicht; sonst die Wahl von
/// fp-fwpng.
pub fn active_page(doc: &Document) -> usize {
    use fp_model::{ElementKind, LayerKind, LayerSlot, Page};
    let sig = |p: &Page| -> Vec<String> {
        let mut out = vec![format!(
            "{}x{}@{},{}",
            p.canvas.width, p.canvas.height, p.canvas.origin.x, p.canvas.origin.y
        )];
        for s in &p.root.layers {
            if let LayerSlot::Own(l) = s {
                if l.kind == LayerKind::Web {
                    continue;
                }
                for c in &l.cels {
                    for e in &c.elements {
                        out.push(fp_render::kind_tag(e).to_string());
                        if let ElementKind::Instance(i) = &e.kind {
                            out.push(format!("{:?}", i.symbol_id));
                        }
                    }
                }
            }
        }
        out
    };
    let Some(rc) = &doc.root_copy else {
        return doc.current_page;
    };
    let want = sig(rc);
    let hits: Vec<usize> = (0..doc.pages.len())
        .filter(|&i| sig(&doc.pages[i]) == want)
        .collect();
    if hits.len() == 1 {
        hits[0]
    } else {
        doc.current_page
    }
}

/// Flaches Bild der Datei als RgbaImage mit Grenzen `rect`.
pub fn flat(doc: &Document, rect: IRect) -> Option<RgbaImage> {
    let f = doc.file_meta.flat_image.as_ref()?;
    if f.width as i32 != rect.width() as i32 || f.height as i32 != rect.height() as i32 {
        return None;
    }
    RgbaImage::from_rgba_bytes(rect, &f.rgba).ok()
}

/// (abweichende Pixel, größte Abweichung); beide transparent zählt als gleich (wie `symrender.compare`).
pub fn diff(a: &RgbaImage, b: &RgbaImage) -> (u64, u8) {
    let c = a.compare(b, a.bounds());
    (c.total - c.equal, c.max_diff)
}

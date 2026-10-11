//! Korpus-Messung: stimmen die Effekte von Rechteck-Grundformen (`PRIv/EFLv`) mit den Effekten des
//! Pfads (`PTHv/EFLv`) überein? `cargo test -p fp-fwpng --test prim_effects -- --ignored --nocapture`

use fp_model::{ElementKind, LayerSlot};
use std::path::{Path, PathBuf};

fn pngs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for p in rd.flatten().map(|e| e.path()) {
        if p.is_dir() {
            pngs(&p, out);
        } else if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")) {
            out.push(p);
        }
    }
}

fn visit(e: &fp_model::Element, t: &mut [usize; 4]) {
    match &e.kind {
        ElementKind::Path(p) => {
            if let Some(pr) = &p.primitive {
                t[0] += 1;
                match (&pr.effects, &e.common.effects) {
                    (a, b) if a == b => t[1] += 1,
                    (None, Some(_)) => t[2] += 1,
                    _ => t[3] += 1,
                }
            }
        }
        ElementKind::Group(g) => g.children.iter().for_each(|c| visit(c, t)),
        _ => {}
    }
}

#[test]
#[ignore]
fn primitive_effects_mirror_path_effects() {
    let root = std::env::var("FP_CORPUS_DIR").map(PathBuf::from).unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.."));
    let mut files = Vec::new();
    pngs(&root, &mut files);
    let mut t = [0usize; 4];
    for f in files {
        let Ok(b) = std::fs::read(&f) else { continue };
        if !fp_fwpng::is_fwpng(&b) {
            continue;
        }
        let Ok(doc) = fp_fwpng::read(&b) else { continue };
        for p in &doc.pages {
            for s in &p.root.layers {
                if let LayerSlot::Own(l) = s {
                    for c in &l.cels {
                        c.elements.iter().for_each(|e| visit(e, &mut t));
                    }
                }
            }
        }
        for s in &doc.symbols {
            for sl in &s.root.layers {
                if let LayerSlot::Own(l) = sl {
                    for c in &l.cels {
                        c.elements.iter().for_each(|e| visit(e, &mut t));
                    }
                }
            }
        }
    }
    println!("Rechteck-Grundformen {}: gleich {}, nur Pfad {}, verschieden {}", t[0], t[1], t[2], t[3]);
}

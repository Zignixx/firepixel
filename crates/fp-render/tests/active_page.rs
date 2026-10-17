//! Aktive Seite beim Lesen (`fp_fwpng`-Heuristik `guess_active_page`) gegen die Seite, deren
//! gerendertes Bild dem gespeicherten flachen Bild (IDAT) am nächsten kommt.
//! `cargo test -p fp-render --release --test active_page -- --ignored --nocapture`

mod common;

use fp_render::{RenderOptions, Renderer};
use std::path::PathBuf;

fn files() -> Vec<PathBuf> {
    let mut v = common::corpus_files();
    let mut t = Vec::new();
    collect(&common::rea().join("re").join("_testdateien"), &mut t);
    v.extend(t);
    v
}

fn collect(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for p in rd.flatten().map(|e| e.path()) {
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")) {
            out.push(p);
        }
    }
}

#[test]
#[ignore]
fn guess_matches_rendered_page() {
    let r = Renderer::new(RenderOptions {
        cache_budget_pixels: 0,
        ..RenderOptions::stored_pixels_only()
    });
    let (mut multi, mut decided, mut right, mut struct_right) = (0, 0, 0, 0);
    for f in files() {
        let Some(doc) = common::load(&f) else {
            continue;
        };
        if doc.pages.len() < 2 {
            continue;
        }
        multi += 1;
        let mut scores = Vec::new();
        for p in 0..doc.pages.len() {
            let Some(rect) = Renderer::page_rect(&doc, p) else {
                continue;
            };
            if rect.area() > 1_500_000 {
                continue;
            }
            let Some(flat) = common::flat(&doc, rect) else {
                continue;
            };
            let frames = doc.pages[p].root.frame_count.max(1) as usize;
            let best = (0..frames.min(8))
                .map(|k| common::diff(&r.render_page(&doc, p, k).image, &flat).0)
                .min()
                .unwrap_or(u64::MAX);
            scores.push((best, p));
        }
        scores.sort();
        if scores.len() < 2 || scores[0].0 == scores[1].0 {
            continue;
        }
        decided += 1;
        let truth = scores[0].1;
        if doc.current_page == truth {
            right += 1;
        } else {
            println!(
                "falsch: {} (geraten {}, Bild {}) Bewertungen {scores:?}",
                f.display(),
                doc.current_page,
                truth
            );
            let desc = |p: &fp_model::Page| {
                let layers: Vec<String> = p
                    .root
                    .layers
                    .iter()
                    .map(|s| match s {
                        fp_model::LayerSlot::Own(l) => format!(
                            "{:?}:{}:{}",
                            l.kind,
                            l.name,
                            l.cels.iter().map(|c| c.elements.len()).sum::<usize>()
                        ),
                        other => format!("{:?}", std::mem::discriminant(other)),
                    })
                    .collect();
                format!(
                    "{:?} {}x{} bg={:?} {layers:?}",
                    p.name, p.canvas.width, p.canvas.height, p.root.background
                )
            };
            if let Some(c) = &doc.root_copy {
                println!("   Kopie: {}", desc(c));
            }
            for (i, p) in doc.pages.iter().enumerate() {
                println!("   Seite {i}: {}", desc(p));
            }
        }
        if common::active_page(&doc) == truth {
            struct_right += 1;
        }
    }
    println!(
        "mehrseitig {multi}, eindeutig {decided}: Heuristik {right}/{decided}, Strukturvergleich {struct_right}/{decided}"
    );
}

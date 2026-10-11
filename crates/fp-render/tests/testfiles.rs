//! Laufzeit-Testdateien aus dem echten Original (`re\_testdateien\<nn>\*.fw.png`): ganze Seite
//! gegen das flache IDAT.
//!
//! `cargo test -p fp-render --release --test testfiles -- --ignored --nocapture`
//!
//! Für 05 (Live-Effekte) gelten die Zahlen aus 05 §11: T1 78.400/78.400, T1b 6 × 48.000/48.000,
//! T2 159.005/160.000, T3 28.800/28.800.

mod common;

use std::collections::BTreeMap;

use fp_render::{BackgroundMode, CachePolicy, RenderOptions, Renderer};

fn files(dir: &str) -> Vec<std::path::PathBuf> {
    let d = common::rea().join("re").join("_testdateien").join(dir);
    let mut v: Vec<_> = std::fs::read_dir(&d)
        .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.to_string_lossy().ends_with(".fw.png")).collect())
        .unwrap_or_default();
    v.sort();
    v
}

/// (gleiche Pixel, Pixel, max. Abweichung, vollständig)
fn eval(path: &std::path::Path, opt: RenderOptions) -> Option<(u64, u64, u8, bool, String)> {
    let doc = common::load(path)?;
    let page = common::active_page(&doc);
    let rect = Renderer::page_rect(&doc, page)?;
    let flat = common::flat(&doc, rect)?;
    let r = Renderer::new(opt);
    // Das IDAT zeigt den Status, der beim Speichern aktiv war (nicht in der Datei vermerkt):
    // bei mehreren Status den am besten passenden nehmen und angeben.
    let frames = doc.pages[page].root.frame_count.clamp(1, 64) as usize;
    let mut best: Option<(u64, u64, u8, bool, String)> = None;
    for f in 0..frames {
        let out = r.render_page(&doc, page, f);
        let c = out.image.compare(&flat, rect);
        let mut miss: Vec<String> = out.report.missing.keys().cloned().collect();
        if frames > 1 {
            miss.push(format!("status{}", f + 1));
        }
        if best.as_ref().is_none_or(|b| c.equal > b.0) {
            best = Some((c.equal, c.total, c.max_diff, out.report.is_complete(), miss.join(",")));
        }
    }
    best
}

#[test]
#[ignore]
fn effects_05() {
    let opt = RenderOptions { cache_policy: CachePolicy::Original, ..RenderOptions::default() };
    let mut results = BTreeMap::new();
    for f in files("05") {
        let name = f.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        if let Some((eq, tot, mx, _, miss)) = eval(&f, opt) {
            println!("{name:30} {eq}/{tot} max {mx} {miss}");
            results.insert(name, (eq, tot));
        }
    }
    let exact = |n: &str| results.get(n).map(|(a, b)| a == b).unwrap_or(false);
    assert!(exact("t1_schatten.fw.png"), "T1");
    for a in [45, 90, 135, 225, 270, 315] {
        assert!(exact(&format!("t1b_schatten_a{a}.fw.png")), "T1b {a}");
    }
    assert!(exact("t3_massiv_pos.fw.png"), "T3");
    let (eq, _) = results.get("t2_abschraegung.fw.png").copied().unwrap_or((0, 0));
    assert!(eq >= 159_005, "T2 {eq}");
}

#[test]
#[ignore]
fn all_testfiles() {
    let dirs = ["02", "04", "05", "06", "07", "09", "12", "13", "15", "16"];
    let variants = [
        ("fill", RenderOptions::default()),
        ("unterlegt", RenderOptions { background: BackgroundMode::UnderlayLast, ..RenderOptions::default() }),
        ("ohne_caches", RenderOptions { cache_policy: CachePolicy::TextOnly, ..RenderOptions::default() }),
    ];
    for d in dirs {
        let mut sums = [(0u64, 0u64, 0usize, 0usize, 0usize); 3];
        for f in files(d) {
            let name = f.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            let mut line = format!("  {name:42}");
            for (vi, (vn, opt)) in variants.iter().enumerate() {
                if let Some((eq, tot, mx, complete, miss)) = eval(&f, *opt) {
                    let s = &mut sums[vi];
                    s.0 += eq;
                    s.1 += tot;
                    s.2 += 1;
                    s.3 += (eq == tot) as usize;
                    s.4 += complete as usize;
                    line += &format!(" {vn} {eq}/{tot} max {mx}");
                    if vi == 0 && !miss.is_empty() {
                        line += &format!(" fehlt[{miss}]");
                    }
                }
            }
            println!("{line}");
        }
        for (vi, (vn, _)) in variants.iter().enumerate() {
            let s = sums[vi];
            println!("{d} {vn}: Dateien {} exakt {} vollständig {}, Pixel {}/{}", s.2, s.3, s.4, s.0, s.1);
        }
    }
}

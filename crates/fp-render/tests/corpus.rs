//! Korpustests „Seiten gegen IDAT“ (710 FW-PNGs der Installationskopie).
//!
//! `cargo test -p fp-render --release --test corpus -- --ignored --nocapture`
//!
//! - `pages_stored_pixels`: Messweise wie `re\scripts\symbole\validate_sym.py` (Stufe A: nur
//!   gespeicherte Pixel, alle Caches, aktive `PDCv`-Seite, erste Symboldefinition, MMX; Seiten
//!   ≤ 1 MP; „vollständig“ = nichts fehlt). Referenz: 651 vollständig, 648 pixelgenau.
//! - `pages_rasterized`: Messweise wie `re\scripts\vektor\test_pages.py` (`MKBv`-Seite ≤ 250.000 px,
//!   Pfade ohne Cache selbst gerastert, nur Seiten ohne fehlende Teile und mit mindestens einem
//!   gerasterten Pfad). Referenz: 169/207 pixelgenau. Zusätzlich die Regeln des Originals auf der
//!   `PDCv`-Seite und eine Aufschlüsselung je Elementart.
//! - `instances_against_cache`: jede unskalierte Instanz mit Cache aus dem Symbol neu aufgebaut
//!   (Pfade ohne Cache gerastert, Effekte gerechnet) und mit ihrem Cache verglichen (Quelle S4 aus 05 §5.7).

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use fp_model::traverse::{walk_root, RootRef};
use fp_model::{Document, ElementKind};
use fp_render::{kind_tag, CachePolicy, PointMap, RenderOptions, Renderer};

fn kinds_on_page(doc: &Document, page: usize) -> BTreeSet<&'static str> {
    let mut s = BTreeSet::new();
    if let Some(p) = doc.pages.get(page) {
        walk_root(&p.root, RootRef::Page(page), &mut |_, _, e| {
            if e.common.visible {
                s.insert(kind_tag(e));
                if e.common.effects.as_ref().is_some_and(|l| l.has_visible()) {
                    s.insert("Effekt");
                }
            }
        });
    }
    s
}

#[derive(Default)]
struct Tally {
    complete: usize,
    exact: usize,
    close: usize,
    dev: usize,
}

impl Tally {
    fn add(&mut self, n: u64, m: u8) {
        self.complete += 1;
        if n == 0 {
            self.exact += 1;
        } else if m <= 2 {
            self.close += 1;
        } else {
            self.dev += 1;
        }
    }
}

#[test]
#[ignore]
fn pages_stored_pixels() {
    let files = common::corpus_files();
    let variants: Vec<(&str, RenderOptions)> = vec![
        ("stufe_a", RenderOptions { cache_budget_pixels: 0, ..RenderOptions::stored_pixels_only() }),
        ("stufe_a_isoliert", RenderOptions { isolate_groups: true, cache_budget_pixels: 0, ..RenderOptions::stored_pixels_only() }),
        (
            "stufe_a_unterlegt",
            RenderOptions { background: fp_render::BackgroundMode::UnderlayLast, cache_budget_pixels: 0, ..RenderOptions::stored_pixels_only() },
        ),
    ];
    let mut tallies: Vec<Tally> = variants.iter().map(|_| Tally::default()).collect();
    let mut missing: BTreeMap<String, usize> = BTreeMap::new();
    let mut fw = 0;
    let mut devs = Vec::new();
    let t0 = std::time::Instant::now();
    for f in &files {
        let Some(doc) = common::load(f) else { continue };
        fw += 1;
        let page = doc.current_page;
        let Some(rect) = Renderer::page_rect(&doc, page) else { continue };
        if rect.area() > 1_000_000 {
            continue;
        }
        let Some(flat) = common::flat(&doc, rect) else { continue };
        for (vi, (_, opt)) in variants.iter().enumerate() {
            let r = Renderer::new(*opt);
            let out = r.render_page(&doc, page, 0);
            if !out.report.is_complete() {
                if vi == 0 {
                    for k in out.report.missing.keys() {
                        *missing.entry(k.clone()).or_default() += 1;
                    }
                }
                continue;
            }
            let (n, m) = common::diff(&out.image, &flat);
            tallies[vi].add(n, m);
            if vi == 0 && n > 0 {
                devs.push(format!("{} {n}/{m}", f.display()));
            }
        }
    }
    println!("FW-PNGs: {fw}, Zeit {:.0} s", t0.elapsed().as_secs_f64());
    for ((name, _), t) in variants.iter().zip(&tallies) {
        println!("{name:18} vollständig {}  pixelgenau {}  ≤2 {}  deutlich {}", t.complete, t.exact, t.close, t.dev);
    }
    println!("fehlende Teile (Seiten je Art, Variante stufe_a): {missing:?}");
    for d in devs.iter().take(40) {
        println!("  abweichend: {d}");
    }
    assert!(tallies[0].complete > 500);
}

#[test]
#[ignore]
fn pages_rasterized() {
    let files = common::corpus_files();
    // (Name, Optionen, MKBv-Kopie statt PDCv)
    let variants: Vec<(&str, RenderOptions, bool)> = vec![
        ("mkbv_wie_page_py", RenderOptions { cache_policy: CachePolicy::All, effects: false, cache_budget_pixels: 0, ..RenderOptions::default() }, true),
        ("mkbv_mit_effekten", RenderOptions { cache_policy: CachePolicy::All, cache_budget_pixels: 0, ..RenderOptions::default() }, true),
        ("pdcv_original", RenderOptions { cache_budget_pixels: 0, ..RenderOptions::default() }, false),
        ("pdcv_ohne_caches", RenderOptions { cache_policy: CachePolicy::TextOnly, cache_budget_pixels: 0, ..RenderOptions::default() }, false),
    ];
    let mut tallies: Vec<Tally> = variants.iter().map(|_| Tally::default()).collect();
    let mut kinds: Vec<BTreeMap<&'static str, (usize, usize)>> = variants.iter().map(|_| BTreeMap::new()).collect();
    let mut missing: Vec<BTreeMap<String, usize>> = variants.iter().map(|_| BTreeMap::new()).collect();
    let t0 = std::time::Instant::now();
    for f in &files {
        let Some(doc) = common::load(f) else { continue };
        let page = doc.current_page;
        let Some(rect) = Renderer::page_rect(&doc, page) else { continue };
        if rect.area() > 250_000 {
            continue;
        }
        let Some(flat) = common::flat(&doc, rect) else { continue };
        let mk_doc = doc.root_copy.as_ref().map(|rc| {
            let mut d = doc.clone();
            if let Some(p) = d.pages.get_mut(page) {
                *p = Arc::clone(rc);
            }
            d
        });
        for (vi, (_, opt, use_mkb)) in variants.iter().enumerate() {
            let d = if *use_mkb {
                match &mk_doc {
                    Some(d) => d,
                    None => continue,
                }
            } else {
                &doc
            };
            let r = Renderer::new(*opt);
            let out = r.render_page(d, page, 0);
            if !out.report.is_complete() {
                for k in out.report.missing.keys() {
                    *missing[vi].entry(k.clone()).or_default() += 1;
                }
                continue;
            }
            if out.report.rasterized_paths == 0 {
                continue;
            }
            let (n, m) = common::diff(&out.image, &flat);
            tallies[vi].add(n, m);
            for k in kinds_on_page(d, page) {
                let e = kinds[vi].entry(k).or_default();
                e.1 += 1;
                if n == 0 {
                    e.0 += 1;
                }
            }
        }
    }
    println!("Zeit {:.0} s", t0.elapsed().as_secs_f64());
    for (vi, (name, _, _)) in variants.iter().enumerate() {
        let t = &tallies[vi];
        println!("{name:18} Seiten {} pixelgenau {} (≤2 {}, deutlich {})", t.complete, t.exact, t.close, t.dev);
        let ks: Vec<String> = kinds[vi].iter().map(|(k, (a, b))| format!("{k} {a}/{b}")).collect();
        println!("   je Elementart (Seiten mit dieser Art: exakt/gesamt): {}", ks.join(", "));
        let top: Vec<String> = {
            let mut v: Vec<_> = missing[vi].iter().collect();
            v.sort_by(|a, b| b.1.cmp(a.1));
            v.iter().take(12).map(|(k, n)| format!("{k}:{n}")).collect()
        };
        println!("   unvollständig wegen: {}", top.join(", "));
    }
}

/// Massiver Schatten gegen CS6-Caches als Kettenprüfung wie `re\scripts\effekte\test_solid.py`
/// (Korpusfälle Distanz 1, Winkel 90/270; Modell „Zielbild = Objekt, Kopie darüber, Objekt obenauf“
/// mit der Mischung von fp-effects). Referenz: 116.464/124.910 Pixel, 269/284 Fälle (alle einfarbigen).
#[test]
#[ignore]
fn solid_shadow_chain() {
    use fp_color::blend::orig_over_mmx;
    use fp_color::Rgba8;
    use std::collections::HashMap;
    let files = common::corpus_files();
    let (mut cases, mut full, mut px, mut good, mut skipped) = (0usize, 0usize, 0u64, 0u64, 0usize);
    let mut seen = std::collections::HashSet::new();
    for f in &files {
        let Some(doc) = common::load(f) else { continue };
        let mut found: Vec<Arc<fp_model::Element>> = Vec::new();
        let mut visit = |e: &Arc<fp_model::Element>| {
            if !matches!(e.kind, ElementKind::Path(_) | ElementKind::Text(_)) || e.cache.is_none() {
                return;
            }
            let Some(l) = &e.common.effects else { return };
            let vis: Vec<_> = l.effects.iter().filter(|x| x.is_visible()).collect();
            if vis.len() == 1 && vis[0].kind() == fp_model::EffectKind::SolidShadow {
                found.push(Arc::clone(e));
            }
        };
        doc.walk_elements(|_, _, e| visit(e));
        // Wie die Referenz auch die MKBv-Kopie (deren Text-Caches doppelt vorkommen).
        if let Some(rc) = &doc.root_copy {
            walk_root(&rc.root, RootRef::RootCopy, &mut |_, _, e| visit(e));
        }
        for e in found {
            let Some(cache) = e.cache.as_ref().and_then(|c| fp_render::convert::image_of_node(c)) else { continue };
            if std::env::var("FP_DEDUPE").is_ok() {
                let key = (f.clone(), cache.bounds().left, cache.bounds().top, cache.width(), cache.height(), cache.as_bytes().len());
                if !seen.insert(key) {
                    continue;
                }
            }
            let fx = &e.common.effects.as_ref().unwrap().effects.iter().find(|x| x.is_visible()).unwrap().clone();
            let ang = (fx.get_f64("angle").unwrap_or(45.0) as i64).rem_euclid(360);
            if fx.get_f64("distance").unwrap_or(30.0) as i64 != 1 || !(ang == 90 || ang == 270) {
                skipped += 1;
                continue;
            }
            let cs = fp_effects::params::parse_color(fx.get("color").unwrap_or("#ffffff")).unwrap_or(Rgba8::rgb(255, 255, 255));
            let (w, h) = (cache.width(), cache.height());
            let mut r: Vec<[u8; 4]> = cache.data().iter().map(|p| if p[3] == 0 { [0; 4] } else { *p }).collect();
            if ang == 270 {
                let mut flipped = Vec::with_capacity(r.len());
                for y in (0..h).rev() {
                    flipped.extend_from_slice(&r[y * w..(y + 1) * w]);
                }
                r = flipped;
            }
            let mut cnt: HashMap<[u8; 3], usize> = HashMap::new();
            for p in &r {
                if p[3] == 255 {
                    *cnt.entry([p[0], p[1], p[2]]).or_default() += 1;
                }
            }
            cnt.remove(&[cs.r, cs.g, cs.b]);
            let Some((co, _)) = cnt.iter().max_by_key(|(c, n)| (**n, std::cmp::Reverse(**c))) else {
                skipped += 1;
                continue;
            };
            let co = *co;
            cases += 1;
            // Tabelle Ergebnis → zulässige (ao, as)-Paare.
            let mut table: HashMap<[u8; 4], Vec<(u8, u8)>> = HashMap::new();
            for a in 0..=255u8 {
                for b in 0..=255u8 {
                    let o = if a == 0 { Rgba8::TRANSPARENT } else { Rgba8::new(co[0], co[1], co[2], a) };
                    let s = if b == 0 { Rgba8::TRANSPARENT } else { Rgba8::new(cs.r, cs.g, cs.b, b) };
                    let mut p = orig_over_mmx(o, orig_over_mmx(s, o));
                    if p.a == 0 {
                        p = Rgba8::TRANSPARENT;
                    }
                    table.entry(p.to_array()).or_default().push((a, b));
                }
            }
            let empty = Vec::new();
            let nz = r.iter().filter(|p| p[3] > 0).count() as u64;
            px += nz;
            let mut case_ok = true;
            for x in 0..w {
                let mut fset = [false; 256];
                fset[0] = true;
                for y in (0..h).rev() {
                    let pairs = table.get(&r[y * w + x]).unwrap_or(&empty);
                    let mut nf = [false; 256];
                    let mut any = false;
                    for &(a, b) in pairs {
                        if fset[b as usize] {
                            nf[a as usize] = true;
                            any = true;
                        }
                    }
                    if any {
                        if r[y * w + x][3] > 0 {
                            good += 1;
                        }
                    } else {
                        case_ok = false;
                        for &(a, _) in pairs {
                            nf[a as usize] = true;
                        }
                        if pairs.is_empty() {
                            nf = [true; 256];
                        }
                    }
                    fset = nf;
                }
            }
            full += case_ok as usize;
        }
    }
    println!("Massiver Schatten: Fälle {cases} (übersprungen {skipped}), Pixel erklärt {good}/{px}, Fälle vollständig {full}/{cases}");
    assert!(cases > 0);
}

/// Zeiten für die Leinwand: die größten Korpusseiten kalt, mit warmem Element-Cache und als
/// Ausschnitt mit Zoom.
#[test]
#[ignore]
fn timing() {
    let mut docs: Vec<(u64, std::path::PathBuf)> = Vec::new();
    for f in common::corpus_files() {
        if let Some(d) = common::load(&f)
            && let Some(r) = Renderer::page_rect(&d, d.current_page)
        {
            docs.push((r.area(), f));
        }
    }
    docs.sort();
    for (_, f) in docs.iter().rev().take(5) {
        let doc = common::load(f).unwrap();
        let page = doc.current_page;
        let rect = Renderer::page_rect(&doc, page).unwrap();
        for pol in [CachePolicy::Original, CachePolicy::TextOnly] {
            let r = Renderer::new(RenderOptions { cache_policy: pol, ..RenderOptions::default() });
            let t = std::time::Instant::now();
            let a = r.render_page(&doc, page, 0);
            let cold = t.elapsed().as_secs_f64() * 1000.0;
            let t = std::time::Instant::now();
            let _ = r.render_page(&doc, page, 0);
            let warm = t.elapsed().as_secs_f64() * 1000.0;
            let region = fp_raster::IRect::from_xywh(rect.left, rect.top, (rect.width() as i32).min(800), (rect.height() as i32).min(600));
            let t = std::time::Instant::now();
            let _ = r.render_region(&doc, page, 0, region, 2.0);
            let zoom = t.elapsed().as_secs_f64() * 1000.0;
            println!(
                "{:?} {}×{} {:?}: kalt {cold:.0} ms, warm {warm:.0} ms, Ausschnitt 800×600 Zoom 2: {zoom:.0} ms, Elemente {}",
                f.file_name().unwrap_or_default(),
                rect.width(),
                rect.height(),
                pol,
                a.report.drawn
            );
        }
    }
}

#[test]
#[ignore]
fn instances_against_cache() {
    let files = common::corpus_files();
    let opt = RenderOptions { cache_policy: CachePolicy::All, cache_budget_pixels: 0, ..RenderOptions::default() };
    let r = Renderer::new(opt);
    let (mut cases, mut exact_cases, mut px, mut px_ok) = (0usize, 0usize, 0u64, 0u64);
    let mut by_kind: BTreeMap<&'static str, (usize, usize)> = BTreeMap::new();
    let mut named = Vec::new();
    for f in &files {
        let Some(doc) = common::load(f) else { continue };
        let page = doc.current_page;
        let Some(p) = doc.pages.get(page) else { continue };
        let mut insts = Vec::new();
        walk_root(&p.root, RootRef::Page(page), &mut |_, _, e| {
            if let ElementKind::Instance(i) = &e.kind
                && e.cache.is_some()
                && i.transform.is_translation(1e-9)
                && e.common.opacity.0 == 255
                && !e.common.effects.as_ref().is_some_and(|l| l.has_visible())
            {
                insts.push(Arc::clone(e));
            }
        });
        for e in insts {
            let Some(cache) = e.cache.as_ref().and_then(|c| fp_render::convert::image_of_node(c)) else { continue };
            let ElementKind::Instance(i) = &e.kind else { continue };
            let Some((_, sym)) = doc.symbol_by_id(i.symbol_id) else { continue };
            let mut e2 = (*e).clone();
            e2.cache = None;
            let out = r.render_element(&doc, &Arc::new(e2), &PointMap::identity(), 0);
            if !out.report.is_complete() {
                continue;
            }
            let rect = out.image.bounds().union(&cache.bounds());
            let (mut sel, mut ok) = (0u64, 0u64);
            for y in rect.top..rect.bottom {
                for x in rect.left..rect.right {
                    let a = out.image.get(x, y).unwrap_or([0; 4]);
                    let b = cache.get(x, y).unwrap_or([0; 4]);
                    if a[3] == 0 && b[3] == 0 {
                        continue;
                    }
                    sel += 1;
                    ok += (a == b) as u64;
                }
            }
            cases += 1;
            px += sel;
            px_ok += ok;
            let kind = match sym.kind() {
                fp_model::SymbolKind::Button => "Schaltfläche",
                fp_model::SymbolKind::Animation => "Animation",
                _ if sym.is_rich() => "Rich Symbol",
                _ => "Grafik",
            };
            let k = by_kind.entry(kind).or_default();
            k.1 += 1;
            if ok == sel {
                exact_cases += 1;
                k.0 += 1;
            }
            if ["Round Button", "Search Filters", "Menu_Bar_07", "Textbox Selected", "Dial Pad"].iter().any(|n| sym.name.contains(n)) {
                named.push(format!("{} [{}]: {ok}/{sel}", sym.name, f.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()));
            }
        }
    }
    println!("Instanzen vollständig {cases}, exakt {exact_cases}; Pixel {px_ok}/{px}");
    for (k, (a, b)) in &by_kind {
        println!("  {k}: {a}/{b} exakt");
    }
    for n in &named {
        println!("  {n}");
    }
    assert!(cases > 100);
}

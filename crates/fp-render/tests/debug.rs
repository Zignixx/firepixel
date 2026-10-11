//! Diagnose einzelner Dateien: `FP_FILE=<Pfad> cargo test -p fp-render --release --test debug -- --ignored --nocapture`.
//! Gibt den Elementbaum der aktiven Seite und der Symbole aus und vergleicht Instanzen mit ihrem
//! Cache in mehreren Varianten.

mod common;

use std::sync::Arc;

use fp_model::{Element, ElementKind, Layer, LayerSlot};
use fp_render::{kind_tag, CachePolicy, PointMap, RenderOptions, Renderer};

fn tree(e: &Element, depth: usize) {
    let fx: Vec<String> = e
        .common
        .effects
        .as_ref()
        .map(|l| l.effects.iter().filter(|x| x.is_visible()).map(|x| x.ui_name().unwrap_or("?").to_string()).collect())
        .unwrap_or_default();
    println!(
        "{:indent$}{} id={} vis={} op={} bld={} cache={} fx={:?}",
        "",
        kind_tag(e),
        e.id.0,
        e.common.visible,
        e.common.opacity.0,
        e.common.blend.0,
        e.cache.is_some(),
        fx,
        indent = depth * 2
    );
    match &e.kind {
        ElementKind::Group(g) => {
            if let Some(m) = &g.mask {
                println!("{:indent$}[Maske {:?}]", "", g.kind, indent = depth * 2 + 2);
                tree(m, depth + 2);
            }
            for c in &g.children {
                tree(c, depth + 1);
            }
        }
        ElementKind::SubLayer(l) => layer(l, depth + 1),
        _ => {}
    }
}

fn layer(l: &Layer, depth: usize) {
    println!("{:indent$}Ebene '{}' {:?} vis={} cels={}", "", l.name, l.kind, l.is_visible(), l.cels.len(), indent = depth * 2);
    if let Some(c) = l.cels.first() {
        for e in &c.elements {
            tree(e, depth + 1);
        }
    }
}

#[test]
#[ignore]
fn debug_file() {
    let Ok(f) = std::env::var("FP_FILE") else { return };
    let doc = common::load(std::path::Path::new(&f)).expect("Datei");
    let page = doc.current_page;
    println!("Seite {page}, Version {}", doc.format_version());
    for s in &doc.pages[page].root.layers {
        if let LayerSlot::Own(l) = s {
            layer(l, 0);
        }
    }
    for s in &doc.symbols {
        println!("Symbol '{}' id={:?} Art {:?}", s.name, s.id, s.kind());
        for sl in &s.root.layers {
            if let LayerSlot::Own(l) = sl {
                layer(l, 1);
            }
        }
    }
    let variants = [
        ("all_iso", RenderOptions { cache_policy: CachePolicy::All, cache_budget_pixels: 0, ..RenderOptions::default() }),
        ("all_direct", RenderOptions { cache_policy: CachePolicy::All, isolate_groups: false, cache_budget_pixels: 0, ..RenderOptions::default() }),
        ("textonly", RenderOptions { cache_policy: CachePolicy::TextOnly, cache_budget_pixels: 0, ..RenderOptions::default() }),
    ];
    let mut insts = Vec::new();
    fp_model::traverse::walk_root(&doc.pages[page].root, fp_model::traverse::RootRef::Page(page), &mut |_, _, e| {
        if matches!(e.kind, ElementKind::Instance(_)) && e.cache.is_some() {
            insts.push(Arc::clone(e));
        }
    });
    for e in &insts {
        let cache = fp_render::convert::image_of_node(e.cache.as_ref().unwrap()).unwrap();
        for (name, opt) in &variants {
            let r = Renderer::new(*opt);
            let mut e2 = (**e).clone();
            e2.cache = None;
            let out = r.render_element(&doc, &Arc::new(e2), &PointMap::identity(), 0);
            let rect = out.image.bounds().union(&cache.bounds());
            let c = out.image.compare(&cache, rect);
            println!("Instanz {} {name}: {}/{} max {} fehlt {:?} näherung {:?}", e.id.0, c.equal, c.total, c.max_diff, out.report.missing, out.report.approximations);
            if let Ok(dir) = std::env::var("FP_DUMP") {
                let mut diffs = Vec::new();
                for y in rect.top..rect.bottom {
                    for x in rect.left..rect.right {
                        let a = out.image.get(x, y).unwrap_or([0; 4]);
                        let b = cache.get(x, y).unwrap_or([0; 4]);
                        if a != b && !(a[3] == 0 && b[3] == 0) {
                            diffs.push(format!("{x},{y} got {a:?} want {b:?}"));
                        }
                    }
                }
                let _ = std::fs::write(format!("{dir}/diff_{}_{name}.txt", e.id.0), diffs.join("\n"));
            }
        }
    }
}

//! Kachel-Cache: Ergebnis bitgleich zum direkten Mischen, Wiederverwendung nach Bearbeitungen,
//! Zeiten großer Seiten (`--ignored`).

mod common;

use std::sync::Arc;

use fp_color::Argb;
use fp_geom::Rect;
use fp_model::{Document, Element, LayerSlot, PathAttrs};
use fp_render::{RenderOptions, Renderer};

fn doc_with_rects(n: usize, w: f32, h: f32) -> Document {
    let mut doc = Document::new(w, h, Argb(0xffff_ffff));
    let mut els = Vec::new();
    for i in 0..n {
        let x = (i * 97 % (w as usize - 60)) as f32;
        let y = (i * 53 % (h as usize - 40)) as f32;
        let mut e = Element::new_rectangle(&mut doc.ids, Rect::new(x, y, x + 50.0 + (i % 7) as f32 * 9.0, y + 30.0), 0.2, PathAttrs::solid_fill(Argb(0xff00_0000 | (i as u32).wrapping_mul(2_654_435_761) >> 8)));
        if i % 5 == 0 {
            e.common.opacity = fp_color::Opacity(140);
        }
        if i % 11 == 0 {
            e.common.blend = fp_color::BlendMode(2);
        }
        els.push(Arc::new(e));
    }
    let p = Arc::make_mut(&mut doc.pages[0]);
    if let LayerSlot::Own(l) = &mut p.root.layers[0] {
        Arc::make_mut(&mut Arc::make_mut(l).cels[0]).elements = els;
    }
    doc
}

fn move_first(doc: &mut Document, dx: f32) {
    let p = Arc::make_mut(&mut doc.pages[0]);
    if let LayerSlot::Own(l) = &mut p.root.layers[0] {
        let cel = Arc::make_mut(&mut Arc::make_mut(l).cels[0]);
        let e = Arc::make_mut(&mut cel.elements[0]);
        if let fp_model::ElementKind::Path(pe) = &mut e.kind {
            for c in &mut pe.contours {
                for n in &mut c.nodes {
                    n.anchor.x += dx;
                    n.pred.x += dx;
                    n.succ.x += dx;
                }
            }
            if let Some(pr) = &mut pe.primitive {
                pr.transform.m[2][0] += dx;
            }
        }
    }
}

#[test]
fn tiles_equal_direct_and_reuse() {
    let mut doc = doc_with_rects(120, 1300.0, 900.0);
    let direct = Renderer::new(RenderOptions { tile_cache: false, ..RenderOptions::default() });
    let tiled = Renderer::new(RenderOptions::default());
    for step in 0..3 {
        let a = direct.render_page(&doc, 0, 0).image;
        let b = tiled.render_page(&doc, 0, 0).image;
        assert_eq!(a.bounds(), b.bounds());
        assert!(a.data() == b.data(), "Schritt {step}: Kachel-Ergebnis weicht ab");
        // Ausschnitt mit ungeradem Rand
        let r = fp_geom::IRect::new(13, 7, 911, 655);
        assert!(direct.render_rect(&doc, 0, 0, r).image.data() == tiled.render_rect(&doc, 0, 0, r).image.data());
        for z in [0.37, 1.0, 2.5] {
            assert!(direct.render_region(&doc, 0, 0, r, z).image.data() == tiled.render_region(&doc, 0, 0, r, z).image.data(), "Zoom {z}");
        }
        move_first(&mut doc, 37.0);
    }
    // erneutes Zeichnen ohne Änderung: nur Treffer
    let _ = tiled.render_page(&doc, 0, 0);
    let before = tiled.tile_stats();
    let _ = tiled.render_page(&doc, 0, 0);
    let after = tiled.tile_stats();
    assert_eq!(after.misses, before.misses, "unverändert ⇒ keine neue Kachel");
    assert!(after.hits > before.hits);
    // eine Änderung ⇒ nur wenige Kacheln neu
    move_first(&mut doc, 5.0);
    let b2 = tiled.tile_stats();
    let _ = tiled.render_page(&doc, 0, 0);
    let a2 = tiled.tile_stats();
    assert!(a2.misses - b2.misses <= 4, "neu gemischt: {}", a2.misses - b2.misses);
}

/// Korpus: Kacheln = direkt, Zeiten.
#[test]
#[ignore]
fn corpus_tiles_and_timing() {
    let (mut pages, mut equal) = (0, 0);
    let mut docs: Vec<(u64, std::path::PathBuf)> = Vec::new();
    for f in common::corpus_files() {
        let Some(d) = common::load(&f) else { continue };
        let direct = Renderer::new(RenderOptions { tile_cache: false, ..RenderOptions::default() });
        let tiled = Renderer::new(RenderOptions::default());
        let p = d.current_page;
        let Some(r) = Renderer::page_rect(&d, p) else { continue };
        docs.push((r.area(), f.clone()));
        if r.area() > 4_000_000 {
            continue;
        }
        pages += 1;
        if direct.render_page(&d, p, 0).image.data() == tiled.render_page(&d, p, 0).image.data() {
            equal += 1;
        } else {
            println!("abweichend: {}", f.display());
        }
    }
    println!("Kacheln gleich direkt: {equal}/{pages} Seiten");
    docs.sort();
    for (_, f) in docs.iter().rev().take(5) {
        let doc = common::load(f).unwrap();
        let page = doc.current_page;
        let rect = Renderer::page_rect(&doc, page).unwrap();
        for tiles in [false, true] {
            let r = Renderer::new(RenderOptions { tile_cache: tiles, ..RenderOptions::default() });
            let t = std::time::Instant::now();
            let _ = r.render_page(&doc, page, 0);
            let cold = t.elapsed().as_secs_f64() * 1000.0;
            let t = std::time::Instant::now();
            let _ = r.render_page(&doc, page, 0);
            let warm = t.elapsed().as_secs_f64() * 1000.0;
            let t = std::time::Instant::now();
            let _ = r.render_region(&doc, page, 0, rect, 0.25);
            let fit = t.elapsed().as_secs_f64() * 1000.0;
            let shifted = fp_raster::IRect::from_xywh(rect.left + 100, rect.top + 60, (rect.width() as i32 - 100).min(1600), (rect.height() as i32 - 60).min(1000));
            let t = std::time::Instant::now();
            let _ = r.render_region(&doc, page, 0, shifted, 1.0);
            let pan = t.elapsed().as_secs_f64() * 1000.0;
            // Bearbeitung: oberstes Element der ersten Ebene ersetzen (neuer Zeiger, gleicher Inhalt)
            let mut d2 = doc.clone();
            if let Some(LayerSlot::Own(l)) = Arc::make_mut(&mut d2.pages[page]).root.layers.iter_mut().find(|s| matches!(s, LayerSlot::Own(l) if l.cels.first().is_some_and(|c| !c.elements.is_empty())))
                && let Some(c) = Arc::make_mut(l).cels.first_mut()
            {
                let c = Arc::make_mut(c);
                let e = (*c.elements[0]).clone();
                c.elements[0] = Arc::new(e);
            }
            let t = std::time::Instant::now();
            let _ = r.render_page(&d2, page, 0);
            let edit = t.elapsed().as_secs_f64() * 1000.0;
            println!(
                "{:?} {}×{} Kacheln {tiles}: kalt {cold:.0} ms, warm {warm:.0} ms, eingepasst (Zoom 0,25) {fit:.0} ms, verschoben 1600×1000 {pan:.0} ms, nach Bearbeitung {edit:.0} ms",
                f.file_name().unwrap_or_default(),
                rect.width(),
                rect.height()
            );
        }
    }
    // synthetisch groß: 8000 × 6000, 3000 Elemente
    let doc = doc_with_rects(3000, 8000.0, 6000.0);
    let rect = Renderer::page_rect(&doc, 0).unwrap();
    for tiles in [false, true] {
        let r = Renderer::new(RenderOptions { tile_cache: tiles, ..RenderOptions::default() });
        let t = std::time::Instant::now();
        let _ = r.render_page(&doc, 0, 0);
        let cold = t.elapsed().as_secs_f64() * 1000.0;
        let t = std::time::Instant::now();
        let _ = r.render_region(&doc, 0, 0, rect, 0.125);
        let fit = t.elapsed().as_secs_f64() * 1000.0;
        let mut d2 = doc.clone();
        move_first(&mut d2, 3.0);
        let t = std::time::Instant::now();
        let _ = r.render_region(&d2, 0, 0, rect, 0.125);
        let edit = t.elapsed().as_secs_f64() * 1000.0;
        println!("synthetisch 8000×6000, 3000 Elemente, Kacheln {tiles}: kalt {cold:.0} ms, eingepasst {fit:.0} ms, nach Bearbeitung {edit:.0} ms");
    }
}

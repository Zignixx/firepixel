//! FX-T: Text-Rasterung gegen die Messreihe des Originals (`re\23-effekte.md`, Basisdatei
//! `re\_testdateien\par\fx\base\fx_text.fw.png`, „Fx Ag“ Arial-BoldMT 40 px, Glättung „glatt“).
//!
//! `cargo test -p fp-render --test text_fxt -- --ignored --nocapture`: Text der Basisdatei ohne
//! gespeicherten Render-Cache neu rastern und mit dem flachen Bild des Originals vergleichen.

mod common;

use fp_model::{ElementKind, LayerSlot};
use fp_render::{CachePolicy, RenderOptions, Renderer};

#[test]
#[ignore = "braucht re\\_testdateien\\par\\fx (Messreihe des Originals) und Arial Bold"]
fn fx_text_rerender_matches_original() {
    let p = common::rea()
        .join("re")
        .join("_testdateien")
        .join("par")
        .join("fx")
        .join("base")
        .join("fx_text.fw.png");
    let Some(doc) = common::load(&p) else {
        eprintln!("übersprungen: {p:?} fehlt");
        return;
    };
    for l in &doc.pages[0].root.layers {
        let LayerSlot::Own(l) = l else { continue };
        for c in &l.cels {
            for e in &c.elements {
                if let ElementKind::Text(t) = &e.kind {
                    println!("{:#?}", fp_render::convert::text_block(&doc, t));
                }
            }
        }
    }
    let rect = Renderer::page_rect(&doc, 0).expect("Seite");
    let flat = common::flat(&doc, rect).expect("flaches Bild");
    let r = Renderer::new(RenderOptions {
        cache_policy: CachePolicy::None,
        cache_budget_pixels: 0,
        ..RenderOptions::default()
    });
    let out = r.render_page(&doc, 0, 0);
    let c = out.image.compare(&flat, rect);
    println!("gleich {}/{} max {}", c.equal, c.total, c.max_diff);
    assert!(c.max_diff <= 16, "max {}", c.max_diff);
}

//! Hilfswerkzeug: eine FW-PNG-Datei mit Firepixel rendern und als PNG ablegen (Analyse).
//! `FP_RENDER_FILE=x.fw.png FP_RENDER_OUT=x.png cargo test -p fp-render --release --test render_file -- --ignored`

mod common;

use fp_render::{CachePolicy, RenderOptions, Renderer};

#[test]
#[ignore]
fn render_file_to_png() {
    let (Ok(src), Ok(dst)) = (
        std::env::var("FP_RENDER_FILE"),
        std::env::var("FP_RENDER_OUT"),
    ) else {
        return;
    };
    let Some(doc) = common::load(std::path::Path::new(&src)) else {
        return;
    };
    let r = Renderer::new(RenderOptions {
        cache_policy: CachePolicy::None,
        cache_budget_pixels: 0,
        ..RenderOptions::default()
    });
    let page = doc.current_page;
    let img = r.render_page(&doc, page, 0).image;
    let (w, h) = (img.width() as u32, img.height() as u32);
    let mut buf = Vec::with_capacity((w * h * 4) as usize);
    for p in img.data() {
        buf.extend_from_slice(p);
    }
    let f = std::fs::File::create(&dst).expect("Ziel");
    let mut e = png::Encoder::new(f, w, h);
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    e.write_header()
        .expect("Kopf")
        .write_image_data(&buf)
        .expect("Daten");
}

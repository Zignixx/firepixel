//! Natürliche Pinsel gegen das flache Bild des Originals (eigene Laufzeitdateien
//! `re\_testdateien\phase5\pinsel\p_*.fw.png`, erzeugt mit `pinsel_reihen.jsf`: je Fall eine waagerechte
//! Linie, ein Merkmal je Reihe). Verglichen wird das Band ±15 px um jede Linie.
//! `cargo test -p fp-render --release --test brush_fit -- --ignored --nocapture`
//! Mit `FP_DUMP_DIR` je Fall ein Bild (oben Original, unten Firepixel), mit `FP_BRUSH_DIFF=<datei>:<fall>`
//! die abweichenden Pixel.

mod common;

use fp_render::{CachePolicy, RenderOptions, Renderer};

#[test]
#[ignore]
fn natural_brushes_against_flat() {
    let dir = common::rea().join("re").join("_testdateien").join("phase5").join("pinsel");
    let r = Renderer::new(RenderOptions { cache_policy: CachePolicy::None, cache_budget_pixels: 0, tile_cache: false, ..RenderOptions::default() });
    let dump = std::env::var_os("FP_DUMP_DIR");
    let diff = std::env::var("FP_BRUSH_DIFF").unwrap_or_default();
    let only = std::env::var("FP_BRUSH_FILTER").unwrap_or_default();
    let (mut files_ok, mut files) = (0, 0);
    let (mut cases_ok, mut cases) = (0, 0);
    let (mut px_ok, mut px) = (0u64, 0u64);
    for f in ["p_basis", "p_schwarz", "p_spitze", "p_zufall", "p_spitzen", "p_einzel", "p_mehr", "p_air", "p_air2", "p_air3", "p_einzel2", "p_remap", "p_seg", "p_seg2"] {
        if !only.is_empty() && !f.contains(&only) {
            continue;
        }
        let Some(doc) = common::load(&dir.join(format!("{f}.fw.png"))) else { continue };
        let page = doc.current_page;
        let Some(rect) = Renderer::page_rect(&doc, page) else { continue };
        let Some(flat) = common::flat(&doc, rect) else { continue };
        let ours = r.render_page(&doc, page, 0).image;
        let mut ys: Vec<i32> = Vec::new();
        fp_model::traverse::walk_root(&doc.pages[page].root, fp_model::traverse::RootRef::Page(page), &mut |_, _, e| {
            if let fp_model::ElementKind::Path(p) = &e.kind
                && let Some(n) = p.contours.first().and_then(|c| c.nodes.first())
            {
                ys.push(n.anchor.y.round() as i32);
            }
        });
        ys.sort();
        let mut all = true;
        for (i, y) in ys.iter().enumerate() {
            let band = if f == "p_mehr" || f.starts_with("p_seg") { 39 } else if f == "p_remap" { 19 } else { 15 };
            let b = fp_raster::IRect::new(rect.left, (y - band).max(rect.top), rect.right, (y + band).min(rect.bottom));
            let (mut same, mut near, mut cnt) = (0u64, 0u64, 0u64);
            for yy in b.top..b.bottom {
                for x in b.left..b.right {
                    let a = flat.get(x, yy).unwrap_or([0; 4]);
                    let o = ours.get(x, yy).unwrap_or([0; 4]);
                    cnt += 1;
                    if a[3] == 0 && o[3] == 0 || a == o {
                        same += 1;
                        near += 1;
                        continue;
                    }
                    near += ((0..4).all(|k| a[k].abs_diff(o[k]) <= 8)) as u64;
                    if diff == format!("{f}:{i}") {
                        println!("   ({x},{yy}) Original {a:?} Firepixel {o:?}");
                    }
                }
            }
            if let Some(g) = std::env::var("FP_BRUSH_GRID").ok().filter(|_| diff == format!("{f}:{i}")) {
                // Alphawerte Original | Firepixel im Fenster x0,y0,x1,y1 (Analyse)
                let v: Vec<i32> = g.split(',').filter_map(|s| s.trim().parse().ok()).collect();
                if v.len() == 4 {
                    for yy in v[1]..v[3] {
                        let row = |img: &fp_raster::RgbaImage| (v[0]..v[2]).map(|x| format!("{:3}", img.get(x, yy).unwrap_or([0; 4])[3])).collect::<Vec<_>>().join(" ");
                        println!("G{yy:4} {} | {}", row(&flat), row(&ours));
                    }
                }
            }
            all &= same == cnt;
            cases += 1;
            cases_ok += (same == cnt) as usize;
            px += cnt;
            px_ok += same;
            println!("{f} #{i:2}: {same}/{cnt} exakt, ±8 {near}");
            if let Some(d) = &dump {
                let (w, h) = (b.width(), b.height());
                let mut buf = Vec::new();
                for img in [&flat, &ours] {
                    for yy in b.top..b.bottom {
                        for x in b.left..b.right {
                            let p = img.get(x, yy).unwrap_or([0; 4]);
                            // über Weiß
                            let a = p[3] as u32;
                            let c = |v: u8| ((v as u32 * a + 255 * (255 - a)) / 255) as u8;
                            buf.extend_from_slice(&[c(p[0]), c(p[1]), c(p[2]), 255]);
                        }
                    }
                }
                if let Ok(file) = std::fs::File::create(std::path::Path::new(d).join(format!("{f}_{i}.png"))) {
                    let mut enc = png::Encoder::new(file, w, h * 2);
                    enc.set_color(png::ColorType::Rgba);
                    enc.set_depth(png::BitDepth::Eight);
                    if let Ok(mut wr) = enc.write_header() {
                        let _ = wr.write_image_data(&buf);
                    }
                }
            }
        }
        files += 1;
        files_ok += all as usize;
    }
    println!("Dateien {files_ok}/{files}, Fälle {cases_ok}/{cases}, Pixel {px_ok}/{px}");
}

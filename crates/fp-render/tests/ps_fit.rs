//! Photoshop-Live-Effekte gegen das flache Bild des Originals (Laufzeitdateien `re\_testdateien\05\t4_*`:
//! je ein Rechteck mit genau einem PS-Effekt). Je Element wird der Bereich um das Objekt verglichen.
//! `cargo test -p fp-render --release --test ps_fit -- --ignored --nocapture`

mod common;

use fp_render::{CachePolicy, RenderOptions, Renderer};

#[test]
#[ignore]
fn ps_effects_against_flat() {
    let dir = common::rea().join("re").join("_testdateien").join("05");
    let r = Renderer::new(RenderOptions {
        cache_policy: CachePolicy::None,
        cache_budget_pixels: 0,
        ..RenderOptions::default()
    });
    let (mut tot, mut ok) = (0u64, 0u64);
    let dump = std::env::var_os("FP_DUMP_DIR");
    let mut n = 0;
    for f in [
        "t4_pseffekte_1",
        "t4_pseffekte_2a",
        "t4_pseffekte_2b",
        "t4_pseffekte_2c",
        "t4_pseffekte_3",
    ] {
        let Some(doc) = common::load(&dir.join(format!("{f}.fw.png"))) else {
            continue;
        };
        let page = doc.current_page;
        let Some(rect) = Renderer::page_rect(&doc, page) else {
            continue;
        };
        let Some(flat) = common::flat(&doc, rect) else {
            continue;
        };
        let ours = r.render_page(&doc, page, 0).image;
        let mut boxes: Vec<(fp_raster::IRect, String)> = Vec::new();
        fp_model::traverse::walk_root(
            &doc.pages[page].root,
            fp_model::traverse::RootRef::Page(page),
            &mut |_, _, e| {
                let Some(l) = &e.common.effects else { return };
                let Some(x) = l.effects.iter().find(|x| {
                    x.moa_id()
                        .is_some_and(|m| m.to_ascii_lowercase().starts_with("{8eeadf50"))
                }) else {
                    return;
                };
                let fp_model::ElementKind::Path(p) = &e.kind else {
                    return;
                };
                let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
                for c in &p.contours {
                    for nd in &c.nodes {
                        x0 = x0.min(nd.anchor.x);
                        y0 = y0.min(nd.anchor.y);
                        x1 = x1.max(nd.anchor.x);
                        y1 = y1.max(nd.anchor.y);
                    }
                }
                let names: Vec<String> = fp_effects::photoshop::PS_EFFECT_NAMES
                    .iter()
                    .filter_map(|nm| {
                        let s = x.params.get_str(nm)?;
                        s.starts_with("1&").then(|| format!("{nm}[{}]", s))
                    })
                    .collect();
                let m = 26;
                boxes.push((
                    fp_raster::IRect::new(
                        x0 as i32 - m,
                        y0 as i32 - m,
                        x1.ceil() as i32 + m,
                        y1.ceil() as i32 + m,
                    ),
                    names.join(" "),
                ));
            },
        );
        for (b, names) in boxes {
            let b = fp_raster::IRect::new(
                b.left.max(rect.left),
                b.top.max(rect.top),
                b.right.min(rect.right),
                b.bottom.min(rect.bottom),
            );
            let (mut same, mut err, mut cnt) = (0u64, 0u64, 0u64);
            for y in b.top..b.bottom {
                for x in b.left..b.right {
                    let a = flat.get(x, y).unwrap_or([0; 4]);
                    let o = ours.get(x, y).unwrap_or([0; 4]);
                    if a[3] == 0 && o[3] == 0 {
                        same += 1;
                        cnt += 1;
                        continue;
                    }
                    same += (a == o) as u64;
                    if a != o && std::env::var_os("FP_PS_DIFF").is_some() && err < 3000 {
                        println!("   ({x},{y}) Original {a:?} Firepixel {o:?}");
                    }
                    err += (0..4).map(|k| u64::from(a[k].abs_diff(o[k]))).sum::<u64>();
                    cnt += 1;
                }
            }
            n += 1;
            println!(
                "{f} #{n} {b:?}: {same}/{cnt} gleich, mittl. Fehler {:.2} | {names}",
                err as f64 / (cnt * 4).max(1) as f64
            );
            tot += cnt;
            ok += same;
            if let Some(d) = &dump {
                let (w, h) = (b.width(), b.height());
                let mut buf = Vec::new();
                for y in b.top..b.bottom {
                    for x in b.left..b.right {
                        buf.extend_from_slice(&flat.get(x, y).unwrap_or([0; 4]));
                    }
                    for x in b.left..b.right {
                        buf.extend_from_slice(&ours.get(x, y).unwrap_or([0; 4]));
                    }
                }
                let file =
                    std::fs::File::create(std::path::Path::new(d).join(format!("ps_{n}.png")))
                        .unwrap();
                let mut enc = png::Encoder::new(file, w * 2, h);
                enc.set_color(png::ColorType::Rgba);
                enc.set_depth(png::BitDepth::Eight);
                enc.write_header().unwrap().write_image_data(&buf).unwrap();
            }
        }
    }
    println!("gesamt {ok}/{tot}");
}

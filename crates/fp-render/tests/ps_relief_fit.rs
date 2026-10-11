//! Satin und Abschrägung/Relief gegen das flache Bild des Originals (eigene Laufzeitdateien
//! `re\_testdateien\phase5\pseffekte\ps_bevel*.fw.png`, `ps_satin.fw.png`; Raster 5 × 6 Zellen zu
//! 110 × 80 px, je Zelle ein Rechteck 60 × 40 mit genau einem PS-Effekt).
//! `cargo test -p fp-render --release --test ps_relief_fit -- --ignored --nocapture`

mod common;

use fp_render::{CachePolicy, RenderOptions, Renderer};

#[test]
#[ignore]
fn relief_and_satin_against_flat() {
    let dir = common::rea().join("re").join("_testdateien").join("phase5").join("pseffekte");
    let r = Renderer::new(RenderOptions { cache_policy: CachePolicy::None, cache_budget_pixels: 0, ..RenderOptions::default() });
    let (mut all, mut ok, mut near) = (0u64, 0u64, 0u64);
    for f in ["ps_bevel", "ps_bevel2", "ps_satin"] {
        let Some(doc) = common::load(&dir.join(format!("{f}.fw.png"))) else { continue };
        let page = doc.current_page;
        let Some(rect) = Renderer::page_rect(&doc, page) else { continue };
        let Some(flat) = common::flat(&doc, rect) else { continue };
        let ours = r.render_page(&doc, page, 0).image;
        let names = std::fs::read_to_string(dir.join(format!("{f}.jsf"))).unwrap_or_default();
        let names: Vec<String> = names.split("[\"").skip(1).filter_map(|s| s.split('"').next().map(str::to_string)).filter(|s| !s.contains(' ')).collect();
        for k in 0..30 {
            let cell = fp_raster::IRect::from_xywh((k % 5) * 110, (k / 5) * 80, 110, 80);
            let (mut same, mut cnt, mut pm, mut mx) = (0u64, 0u64, 0u64, 0u8);
            let mut any = false;
            for y in cell.top..cell.bottom {
                for x in cell.left..cell.right {
                    let a = flat.get(x, y).unwrap_or([0; 4]);
                    let o = ours.get(x, y).unwrap_or([0; 4]);
                    any |= a[3] != 0;
                    if a[3] == 0 && o[3] == 0 {
                        continue;
                    }
                    cnt += 1;
                    let d = (0..4).map(|i| if i < 3 && (a[3] == 0 || o[3] == 0) { 0 } else { a[i].abs_diff(o[i]) }).max().unwrap_or(0);
                    same += (d == 0) as u64;
                    pm += (d <= 2) as u64;
                    mx = mx.max(d);
                }
            }
            if !any {
                continue;
            }
            all += cnt;
            ok += same;
            near += pm;
            let nm = names.get(k as usize).cloned().unwrap_or_default();
            println!("{f} #{k:2} {nm:14}: {same}/{cnt} exakt, ±2 {pm}, max {mx}");
        }
    }
    println!("gesamt {ok}/{all} exakt, ±2 {near}");
}

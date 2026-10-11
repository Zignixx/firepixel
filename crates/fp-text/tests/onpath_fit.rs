//! Versuch: Platzierungsregel für Text auf Pfad gegen die Render-Caches der Laufzeitdateien
//! (`t2_frames`, `t3_onpath`). `cargo test -p fp-text --release --test onpath_fit -- --ignored --nocapture`

mod common;

use common::*;
use fp_geom::IRect;
use fp_text::layout::path::PathSampler;
use fp_text::raster::Affine;
use fp_text::*;

#[derive(Clone, Copy, Debug)]
struct Variant {
    /// Ankerpunkt der Glyphe auf dem Pfad: 0 = Mitte des Vorschubs, 1 = Ursprung (links).
    anchor: u8,
    /// Senkrechter Versatz der Grundlinie (px, + = „unten“ im Glyphensystem).
    shift: f64,
    /// Faktor auf die Laufweite entlang des Pfads.
    scale: f64,
}

fn apply(block: &TextBlock, lay: &mut TextLayout, v: Variant) {
    let Some(tp) = block.path.as_ref() else { return };
    let Some(c) = tp.contours.first() else { return };
    let sampler = PathSampler::new(c, tp.reversed);
    let len = sampler.length();
    let Some(line) = lay.lines.first().cloned() else { return };
    let start = match block.runs.first().map(|r| r.attrs.alignment).unwrap_or_default() {
        Alignment::Center => (len - line.align_width) / 2.0,
        Alignment::Right => len - line.align_width,
        _ => 0.0,
    } + tp.offset;
    for g in lay.glyphs.iter_mut() {
        let k = g.size / g.font.units_per_em() as f64;
        let core = g.advance - g.tracking;
        let half = core / 2.0;
        let s0 = start + v.shift + (g.x - line.start) * v.scale;
        let s_anchor = if v.anchor == 0 { s0 + half } else { s0 };
        g.visible = !g.is_tab && s0 + half >= 0.0 && s0 + half <= len;
        let Some((p, th)) = sampler.at(s_anchor) else { continue };
        let (sn, cs) = th.sin_cos();
        let (t, nrm) = match tp.mode {
            PathTextMode::Rotate => ([cs, sn], [-sn, cs]),
            PathTextMode::Vertical => ([1.0, 0.0], [0.0, 1.0]),
            PathTextMode::SkewVertical => ([cs, sn], [0.0, 1.0]),
            PathTextMode::SkewHorizontal => ([1.0, 0.0], [-sn, cs]),
        };
        let back = if v.anchor == 0 { half } else { 0.0 };
        g.transform = Affine {
            a: t[0] * k * g.h_scale,
            b: t[1] * k * g.h_scale,
            c: -nrm[0] * k,
            d: -nrm[1] * k,
            e: p[0] - t[0] * back,
            f: p[1] - t[1] * back,
        };
    }
}

#[test]
#[ignore]
fn fit_on_path() {
    let Some(recs) = read_jsonl(&out_dir().join("testfiles.jsonl")) else { return };
    let opts = LayoutOptions::default();
    let mut seen = std::collections::HashSet::new();
    for rec in &recs {
        let block = block_from(rec);
        if block.kind != FrameKind::OnPath {
            continue;
        }
        let Some(c) = rec.get("cache") else { continue };
        let key = format!("{}|{:?}", rec["file"], rec["attrs"]);
        if !seen.insert(key) {
            continue;
        }
        let (x, y, w, h) = (i(c, "x").unwrap_or(0), i(c, "y").unwrap_or(0), i(c, "w").unwrap_or(0), i(c, "h").unwrap_or(0));
        let rgba = b64(c["rgba"].as_str().unwrap_or(""));
        if rgba.len() != (w * h * 4) as usize || w * h == 0 {
            continue;
        }
        let bounds = IRect::new(x as i32, y as i32, (x + w) as i32, (y + h) as i32);
        let base = layout(&block, fonts(), &opts);
        let score = |lay: &TextLayout| -> (f64, u64) {
            let m = render::coverage(lay, Some(bounds));
            let ours = m.data();
            let (mut inter, mut uni, mut exact) = (0u64, 0u64, 0u64);
            for k in 0..(w * h) as usize {
                let (a, b) = (ours.get(k).copied().unwrap_or(0), rgba[4 * k + 3]);
                inter += (a > 0 && b > 0) as u64;
                uni += (a > 0 || b > 0) as u64;
                exact += (a == b) as u64;
            }
            (inter as f64 / uni.max(1) as f64, exact)
        };
        let cur = score(&base);
        if let Some(dir) = std::env::var_os("FP_DUMP_DIR") {
            // rot = Cache des Originals, grün = Firepixel
            let m = render::coverage(&base, Some(bounds));
            let mut buf = Vec::with_capacity((w * h * 3) as usize);
            for k in 0..(w * h) as usize {
                buf.extend_from_slice(&[rgba[4 * k + 3], m.data().get(k).copied().unwrap_or(0), 0]);
            }
            let n = seen.len();
            let f = std::fs::File::create(std::path::Path::new(&dir).join(format!("onpath_{n}.png"))).unwrap();
            let mut e = png::Encoder::new(f, w as u32, h as u32);
            e.set_color(png::ColorType::Rgb);
            e.set_depth(png::BitDepth::Eight);
            e.write_header().unwrap().write_image_data(&buf).unwrap();
        }
        let mut best = (0.0f64, 0u64, Variant { anchor: 0, shift: 0.0, scale: 1.0 });
        for anchor in [0u8, 1] {
            for (si, sc) in (-20..=20).flat_map(|s| (0..=24).map(move |c| (s, c))) {
                let v = Variant { anchor, shift: si as f64 * 0.5, scale: 0.94 + sc as f64 * 0.0025 };
                let mut lay = base.clone();
                apply(&block, &mut lay, v);
                let s = score(&lay);
                if s.0 > best.0 {
                    best = (s.0, s.1, v);
                }
            }
        }
        let tp = block.path.as_ref().unwrap();
        println!(
            "{} {:?} mode {:?} off {} size {}: aktuell IoU {:.3} exakt {}/{}; bestes {:?} IoU {:.3} exakt {}",
            rec["file"].as_str().unwrap_or(""),
            block.text().chars().take(12).collect::<String>(),
            tp.mode,
            tp.offset,
            block.runs[0].attrs.size,
            cur.0,
            cur.1,
            w * h,
            best.2,
            best.0,
            best.1
        );
    }
}

//! Vergleiche mit echten Exporten des Originals aus der Testrunde 11.10.2026 (`..\re\_testdateien\08`).
//! Aufruf: `cargo test -p fp-export --test testfiles -- --ignored --nocapture`.

mod common;

use common::{re_dir, read_gif, read_png, read_png8};
use fp_export::options::{ColorMode, Dither, ExportFormat, OptimizeSettings, PaletteKind, Transparency};
use fp_export::pipeline::{optimize_pixels, ExportContext, Optimized};

fn gif_settings(n: i32, dither: bool, optimized: bool) -> OptimizeSettings {
    OptimizeSettings {
        format: ExportFormat::Gif,
        palette_kind: PaletteKind::Adaptive,
        max_colors: n,
        dither: if dither { Dither::Diffusion(100) } else { Dither::None },
        transparency: Transparency::None,
        optimized,
        ..OptimizeSettings::default()
    }
}

/// T1: GIF Adaptiv 256/128/16 × Dither 0/100 aus 5 Quellen (31 Dateien). Spezifikation:
/// Indexbild 31/31 (182.236/182.236 Pixel), Palette 26/31 exakt.
#[test]
#[ignore]
fn t1_gif_adaptive() {
    let dir = re_dir().join("_testdateien").join("08").join("T1");
    let Ok(rd) = std::fs::read_dir(&dir) else {
        eprintln!("T1 fehlt: {}", dir.display());
        return;
    };
    let mut names: Vec<String> = rd.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    let ctx = ExportContext { canvas: 0xFFFF_FFFF, ..ExportContext::default() };
    let (mut files, mut pal_ok, mut idx_files, mut idx_px, mut total_px) = (0, 0, 0, 0usize, 0usize);
    for src in names.iter().filter(|n| n.ends_with("_src.png")) {
        let basis = &src[..src.len() - 8];
        let Some((w, h, px)) = read_png(&dir.join(src)) else { continue };
        for gname in names.iter().filter(|n| n.starts_with(&format!("{basis}_a")) && n.ends_with(".gif")) {
            let stem = &gname[basis.len() + 1..gname.len() - 4];
            let parts: Vec<&str> = stem.split('_').collect();
            let n: i32 = parts[0][1..].parse().unwrap_or(256);
            let dith = parts.get(1) == Some(&"d100");
            let opt = !stem.ends_with("unopt");
            let g = read_gif(&dir.join(gname)).expect("GIF lesbar");
            let fr = &g.frames[0];
            let pal_src = fr.lct.as_ref().unwrap_or(&g.gct);
            let maxi = fr.indices.iter().copied().max().unwrap_or(0) as usize;
            let fw_pal: Vec<u32> = pal_src[..=maxi].iter().map(|c| 0xFF00_0000 | (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32).collect();
            let s = gif_settings(n, dith, opt);
            let Optimized::Indexed(ix) = optimize_pixels(&px, w, h, &s, &ctx) else { panic!("indiziert erwartet") };
            let same_idx = ix.indices.iter().zip(&fr.indices).filter(|(a, b)| a == b).count();
            files += 1;
            pal_ok += usize::from(ix.palette == fw_pal);
            idx_files += usize::from(same_idx == w * h);
            idx_px += same_idx;
            total_px += w * h;
            println!("{gname:28} FW={:3} Rust={:3} Palette gleich={:5} Index {same_idx}/{}", fw_pal.len(), ix.palette.len(), ix.palette == fw_pal, w * h);
        }
    }
    println!("T1: Paletten exakt {pal_ok}/{files}, Indexbild gleich {idx_files}/{files}, Pixel {idx_px}/{total_px}");
    assert_eq!(files, 31);
    assert_eq!(idx_files, 31, "Indexbilder");
    assert!(pal_ok >= 26, "Paletten {pal_ok}/31");
}

/// T2: PNG8 „Alpha-Transparenz“ (RGBA-Palette) Dither 0/100 und Index-Alpha (GIF und PNG8).
/// Spezifikation: 3/3 Paletten und Indexbilder identisch.
#[test]
#[ignore]
fn t2_png8_alpha() {
    let dir = re_dir().join("_testdateien").join("08").join("T2");
    let Some((w, h, px)) = read_png(&dir.join("rgba_src.png")) else {
        eprintln!("T2 fehlt");
        return;
    };
    let ctx = ExportContext { canvas: 0x00FF_FFFF, ..ExportContext::default() };
    let mut ok = 0;
    for (file, dither) in [("rgba_d0.png", false), ("rgba_d100.png", true)] {
        let (_, _, fw_pal, fw_idx) = read_png8(&dir.join(file)).expect("PNG8");
        let mut s = gif_settings(256, dither, true);
        s.format = ExportFormat::Png;
        s.color_mode = ColorMode::Indexed;
        s.transparency = Transparency::Rgba;
        let Optimized::Indexed(ix) = optimize_pixels(&px, w, h, &s, &ctx) else { panic!() };
        let pal_eq = ix.palette == fw_pal;
        let same = ix.indices.iter().zip(&fw_idx).filter(|(a, b)| a == b).count();
        println!("{file}: Palette {} / {} gleich={pal_eq}, Index {same}/{}", ix.palette.len(), fw_pal.len(), w * h);
        ok += usize::from(pal_eq && same == w * h);
    }
    // Index-Alpha-GIF: 255 Farben + Transparenzeintrag 255.
    let g = read_gif(&dir.join("indexalpha_d0.gif")).expect("GIF");
    let fr = &g.frames[0];
    let mut s = gif_settings(256, false, true);
    s.transparency = Transparency::IndexAlpha;
    let Optimized::Indexed(ix) = optimize_pixels(&px, w, h, &s, &ctx) else { panic!() };
    let fw_pal: Vec<u32> = g.gct[..ix.palette.len().min(g.gct.len())].iter().map(|c| 0xFF00_0000 | (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32).collect();
    let same = ix.indices.iter().zip(&fr.indices).filter(|(a, b)| a == b).count();
    let pal_eq = ix.palette == fw_pal && fr.transparent.map(|t| t as usize) == ix.transparent_index;
    println!("indexalpha_d0.gif: Palette gleich={pal_eq} (Transparenz {:?}/{:?}), Index {same}/{}", fr.transparent, ix.transparent_index, w * h);
    ok += usize::from(pal_eq && same == w * h);
    println!("T2: {ok}/3");
    assert_eq!(ok, 3);
}


/// T4: Animation (3 Status) – Auto-Crop/Auto-Differenz/Schleife/Verlust gegen die FW-GIFs, byte-genau.
#[test]
#[ignore]
fn t4_animation() {
    use fp_export::animation::{animated_gif, AnimFrameSrc};
    use fp_model::FrameInfo;
    let dir = re_dir().join("_testdateien").join("08").join("T4");
    let Some(src) = read_gif(&dir.join("anim_none.gif")) else {
        eprintln!("T4 fehlt");
        return;
    };
    let (w, h) = (src.width, src.height);
    // Quellbilder = volle Einzelbilder ohne Zuschnitt
    let frames: Vec<AnimFrameSrc> = src
        .frames
        .iter()
        .map(|f| AnimFrameSrc {
            pixels: f.indices.iter().map(|&i| { let c = src.gct[i as usize]; 0xFF00_0000 | (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32 }).collect(),
            info: FrameInfo { disposal: 0, delay: 7, visible: true, ..FrameInfo::default() },
        })
        .collect();
    let ctx = ExportContext { canvas: 0xFFFF_FFFF, ..ExportContext::default() };
    let mut ok = 0;
    let cases = [
        ("anim_cd.gif", true, true, 0, 0),
        ("anim_c.gif", true, false, 0, 0),
        ("anim_none.gif", false, false, 0, 0),
        ("anim_d_ohnecrop.gif", false, true, 0, 0),
        ("anim_noloop.gif", true, true, -1, 0),
        ("anim_cd_rep3.gif", true, true, 3, 0),
        ("anim_cd_l20.gif", true, true, 0, 20),
    ];
    for (name, crop, diff, lp, lossy) in cases {
        let fw = std::fs::read(dir.join(name)).expect("Datei");
        let mut s = gif_settings(128, false, true);
        s.format = ExportFormat::AnimatedGif;
        s.anim.auto_crop = crop;
        s.anim.auto_difference = diff;
        s.anim.loop_count = lp;
        s.lossy = lossy;
        let ours = animated_gif(&frames, w, h, &s, &ctx).expect("GIF");
        let same = ours == fw;
        if !same {
            let g = read_gif(&dir.join(name)).unwrap();
            let tmp = std::env::temp_dir().join(format!("fp_t4_{name}"));
            let _ = std::fs::write(&tmp, &ours);
            let o = read_gif(&tmp).unwrap();
            println!("  FW  : {} B, GCT {:?}, Rahmen {:?}", fw.len(), g.gct, g.frames.iter().map(|f| (f.x, f.y, f.w, f.h, f.disposal, f.transparent)).collect::<Vec<_>>());
            println!("  Rust: {} B, GCT {:?}, Rahmen {:?}", ours.len(), o.gct, o.frames.iter().map(|f| (f.x, f.y, f.w, f.h, f.disposal, f.transparent)).collect::<Vec<_>>());
        }
        println!("{name}: byte-gleich={same}");
        ok += usize::from(same);
    }
    println!("T4: {ok}/{}", cases.len());
    assert_eq!(ok, cases.len());
}


fn td_cells(html: &str, prefix: &str) -> Vec<(String, usize, usize)> {
    let mut out = Vec::new();
    for line in html.lines() {
        let l = line.trim();
        if !l.starts_with("<td") || l.contains("spacer.gif") {
            continue;
        }
        let attr = |k: &str| -> usize {
            l.find(&format!("{k}=\"")).and_then(|i| l[i + k.len() + 2..].split('"').next()?.parse().ok()).unwrap_or(1)
        };
        if let Some(i) = l.find("name=\"") {
            let n = l[i + 6..].split('"').next().unwrap_or("").trim_start_matches(prefix).to_string();
            out.push((n, attr("rowspan"), attr("colspan")));
        }
    }
    out
}

/// T5: Slice-Tabelle (Polygon + 2 Rechtecke) mit transparenten Abstandhaltern gegen die FW-HTML:
/// Spalten/Zeilen, 13/13 Zellen (rowspan/colspan), 13/13 Dateinamen, Imagemap.
#[test]
#[ignore]
fn t5_slices_html() {
    use fp_export::html::{write_html, CellContent, HtmlOptions, Shims};
    use fp_export::slices::{auto_name, build_grid, collect_slices, NameDate};
    let dir = re_dir().join("_testdateien").join("08").join("T5");
    let (Ok(fw_png), Ok(fw_html)) = (std::fs::read(dir.join("slices.fw.png")), std::fs::read_to_string(dir.join("spacer").join("slices_spacer.htm"))) else {
        eprintln!("T5 fehlt");
        return;
    };
    let doc = fp_fwpng::read(&fw_png).expect("slices.fw.png");
    let slices = collect_slices(&doc, 0);
    assert_eq!(slices.len(), 3);
    let p = &doc.pages[0];
    let g = build_grid([0.0, 0.0, p.canvas.width, p.canvas.height], &slices);
    assert_eq!(g.col_widths(), vec![15, 70, 15, 10, 50, 20, 20]);
    assert_eq!(g.row_heights(), vec![10, 10, 30, 10, 20, 40, 30]);
    let date = NameDate { year: 2026, month: 10, day: 11 };
    let cells: Vec<Vec<CellContent>> = (0..g.rows())
        .map(|r| {
            (0..g.cols())
                .map(|c| {
                    let n = auto_name(&[6, 15, 1, 0, 0, 0], r + 1, c + 1, "slices_spacer", 1, 1, date);
                    CellContent { image: Some(format!("{n}.gif")), name: n, html: None, slice: g.cells[r][c].slice() }
                })
                .collect()
        })
        .collect();
    let o = HtmlOptions { shims: Shims::Transparent, base_name: "slices_spacer.gif".into(), ..HtmlOptions::default() };
    let out = write_html(&g, &cells, &slices, &[], 1, &|_, _| None, &|t, _, _, _, _| t.len() as f64 * 6.0, &o);
    let fw = td_cells(&fw_html, "");
    let ours = td_cells(&out.html, "");
    let same = fw.iter().zip(&ours).filter(|(a, b)| a == b).count();
    println!("T5: Zellen {same}/{} (Rust {}), Namen und Spans gleich", fw.len(), ours.len());
    // Tabellenzeilen textgleich (Zeilen mit <td …>), Imagemap gleich
    let rows = |h: &str| h.lines().filter(|l| l.trim_start().starts_with("<td") || l.trim_start().starts_with("<area")).map(|l| l.trim().to_string()).collect::<Vec<_>>();
    let (fr, or) = (rows(&fw_html), rows(&out.html));
    let same_lines = fr.iter().zip(&or).filter(|(a, b)| a == b).count();
    println!("T5: Tabellen-/Area-Zeilen textgleich {same_lines}/{}", fr.len());
    assert_eq!(same, 13);
    assert_eq!(ours.len(), 13);
    assert_eq!(same_lines, fr.len());
}


/// JPEG-Qualitätstabellen (08 §4.2) aus der EXE des Originals gegen die DQT der 50 JPEGs des Originals in der
/// Installation (`re\export\_scratch\jpeg_dqt_check.json`): Spezifikation 50/50 genau eine Stufe,
/// Unterabtastungsregel 50/50.
#[test]
#[ignore]
fn jpeg_dqt_of_installation() {
    use fp_export::jpeg::read_dqt;
    use fp_export::jpeg_tables::FwJpegTables;
    let root = std::env::var_os("FP_CORPUS_DIR").map(std::path::PathBuf::from).unwrap_or_else(|| re_dir().join("..").join("korpus"));
    let Some(t) = fp_color::install::main_exe_in(&root).and_then(|exe| FwJpegTables::from_path(&exe)) else {
        eprintln!("EXE des Originals fehlt im Korpus");
        return;
    };
    let check: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(re_dir().join("export").join("_scratch").join("jpeg_dqt_check.json")).unwrap()).unwrap();
    let all: Vec<_> = (1..=100).map(|q| t.qtables(q)).collect();
    let (mut n, mut ok, mut sub_ok) = (0, 0, 0);
    for row in check["rows"].as_array().unwrap() {
        if row[1].as_str() != Some("fw") {
            continue;
        }
        let rel = row[0].as_str().unwrap().trim_start_matches('\\').replace('\\', "/");
        let Ok(data) = std::fs::read(root.join(&rel)) else { continue };
        n += 1;
        let want = row[2][0].as_i64().unwrap_or(0) as i32;
        let dqt = read_dqt(&data);
        let lum = dqt.iter().find(|d| d.0 == 0).map(|d| d.1.clone()).unwrap_or_default();
        let chrom = dqt.iter().find(|d| d.0 == 1).map(|d| d.1.clone()).unwrap_or_default();
        let found: Vec<i32> = (1..=100).filter(|&q| all[q as usize - 1].lum.to_vec() == lum && all[q as usize - 1].chrom.to_vec() == chrom).collect();
        ok += usize::from(found.contains(&want));
        let h_y = row[4][0][1].as_i64().unwrap_or(1);
        sub_ok += usize::from(if want > 85 { h_y == 1 } else { h_y == 1 || h_y == 2 });
    }
    println!("JPEG-DQT: {ok}/{n} Stufen wie im Original gleich, Unterabtastung {sub_ok}/{n}");
    assert_eq!(n, 50);
    assert_eq!(ok, 50);
    assert_eq!(sub_ok, 50);
}

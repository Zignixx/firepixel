//! Echte Testdateien `re\_testdateien\09\*.fw.png` (Laufzeitrunde 11.10.2026), extrahiert mit
//! `tools\golden\fp-text\gen_testfiles.py`. Aufruf:
//! `cargo test -p fp-text --test testfiles -- --ignored --nocapture`.

mod common;

use std::collections::BTreeMap;

use common::*;
use fp_geom::IRect;
use fp_text::*;
use serde_json::Value;

fn category(rec: &Value, block: &TextBlock) -> String {
    let a = &rec["attrs"];
    let file = rec["file"].as_str().unwrap_or("");
    let text = block.text();
    match block.kind {
        FrameKind::FixedWidth => return "Rahmentext (TFTi 1)".into(),
        FrameKind::OnPath => return "Text auf Pfad (TFTi 2)".into(),
        FrameKind::InPath => return "Text in Pfad (TFTi 3)".into(),
        FrameKind::Point => {}
    }
    if block.orientation.is_vertical() {
        return "vertikal (ORIi 1/3)".into();
    }
    if text.contains('\t') {
        return "Tabulator (T1)".into();
    }
    let r0 = &block.runs[0].attrs;
    if r0.italic {
        return "Faux-Kursiv".into();
    }
    if r0.bold {
        return "Faux-Fett".into();
    }
    if r0.underline {
        return "unterstrichen".into();
    }
    if r0.h_scale != 1.0 {
        return "HSCf".into();
    }
    if i(a, "ORIi") == Some(2) {
        return "horizontal RTL (ORIi 2)".into();
    }
    if file.starts_with("t9") {
        return "Glättung (T9)".into();
    }
    "normal".into()
}

#[test]
#[ignore]
fn testfiles_frames_and_pixels() {
    let Some(recs) = read_jsonl(&out_dir().join("testfiles.jsonl")) else {
        eprintln!("übersprungen: testfiles.jsonl fehlt (gen_testfiles.py ausführen)");
        return;
    };
    if !have_corpus_fonts() {
        eprintln!("übersprungen: Korpusschriften fehlen");
        return;
    }
    let opts = LayoutOptions::default();
    // Kategorie → (n, exakt, Pixel exakt, Pixel gesamt)
    let mut st: BTreeMap<String, (usize, usize, u64, u64)> = BTreeMap::new();
    let mut seen = std::collections::HashSet::new();
    let mut lines = Vec::new();
    let mut t9 = (0usize, 0usize);
    for rec in &recs {
        let Some(fw) = saved_frame(rec) else { continue };
        let block = block_from(rec);
        let key = format!(
            "{}|{}|{:?}|{:?}",
            rec["file"],
            block.text(),
            fw,
            rec["attrs"].get("TA1i")
        );
        if !seen.insert(key) {
            continue;
        }
        let cat = category(rec, &block);
        let lay = layout(&block, fonts(), &opts);
        let got = frame_arr(&lay.frame);
        let exact = got == fw;
        let e = st.entry(cat.clone()).or_default();
        e.0 += 1;
        e.1 += exact as usize;
        if !exact {
            lines.push(format!(
                "  {cat}: {:?} {:?} rust {:?} fw {:?}",
                rec["file"].as_str().unwrap_or(""),
                block.text().chars().take(24).collect::<String>(),
                got,
                fw
            ));
        }
        // Pixel gegen den Render-Cache
        if let Some(c) = rec.get("cache") {
            let (x, y, w, h) = (
                i(c, "x").unwrap_or(0),
                i(c, "y").unwrap_or(0),
                i(c, "w").unwrap_or(0),
                i(c, "h").unwrap_or(0),
            );
            let rgba = b64(c["rgba"].as_str().unwrap_or(""));
            if rgba.len() == (w * h * 4) as usize && w * h > 0 {
                let bounds = IRect::new(x as i32, y as i32, (x + w) as i32, (y + h) as i32);
                let m = render::coverage(&lay, Some(bounds));
                let ours = m.data();
                let mut ex = 0u64;
                for k in 0..(w * h) as usize {
                    ex += (ours.get(k).copied().unwrap_or(0) == rgba[4 * k + 3]) as u64;
                }
                e.2 += ex;
                e.3 += (w * h) as u64;
                // T9: alle Zwischenwerte des Caches liegen in der Tonkurve des Modus?
                if rec["file"].as_str().unwrap_or("").starts_with("t9") {
                    let (_, lut) = fp_text::antialias::sampling(&block.antialias);
                    let all_in = (0..(w * h) as usize)
                        .map(|k| rgba[4 * k + 3])
                        .filter(|&v| v != 0 && v != 255)
                        .all(|v| lut.contains(&v));
                    t9.0 += 1;
                    t9.1 += all_in as usize;
                }
            }
        }
    }
    for (k, (n, ex, pe, pn)) in &st {
        println!(
            "{k:28} Rahmen {ex}/{n} exakt, Pixel {:.1} % exakt",
            100.0 * *pe as f64 / (*pn).max(1) as f64
        );
    }
    println!(
        "T9 Tonkurve: {}/{} Texte mit allen Zwischen-Alphawerten in der Tabelle",
        t9.1, t9.0
    );
    for l in &lines {
        println!("{l}");
    }
}

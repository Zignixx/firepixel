//! Korpustests (`cargo test -p fp-text --test corpus -- --ignored --nocapture`).
//!
//! Daten: `tools\golden\fp-text\out\*.jsonl` aus `gen_layout_corpus.py`, `gen_pixel_corpus.py` und
//! `gen_testfiles.py` (Python-Referenz, nicht im Crate). Fehlen Daten oder Schriften, wird der Test
//! mit Hinweis übersprungen.

mod common;

use std::collections::BTreeMap;

use common::*;
use fp_geom::IRect;
use fp_text::*;
use serde_json::Value;

#[derive(Default, Debug)]
struct Stat {
    n: usize,
    all4: usize,
    vert: usize,
    horiz: usize,
    width: usize,
    horiz_pm1: usize,
    same_as_py: usize,
}

impl Stat {
    fn add(&mut self, got: [f64; 4], fw: [f64; 4], py: Option<[f64; 4]>) {
        self.n += 1;
        let ok: Vec<bool> = (0..4).map(|k| got[k] == fw[k]).collect();
        self.all4 += ok.iter().all(|x| *x) as usize;
        self.vert += (ok[1] && ok[3]) as usize;
        self.horiz += (ok[0] && ok[2]) as usize;
        self.width += ((got[2] - got[0]) == (fw[2] - fw[0])) as usize;
        self.horiz_pm1 += ((got[0] - fw[0]).abs() <= 1.0 && (got[2] - fw[2]).abs() <= 1.0) as usize;
        if let Some(p) = py {
            self.same_as_py += (got == p) as usize;
        }
    }
}

/// Rahmen aller ATE-Punkttexte gegen die gespeicherten Rahmen und gegen die Python-Referenz
/// (Auswahl wie `check_layout.py`). Ziel: alle vier Kanten wie Python.
#[test]
#[ignore]
fn corpus_frames() {
    let Some(recs) = read_jsonl(&out_dir().join("layout_corpus.jsonl")) else {
        eprintln!("übersprungen: layout_corpus.jsonl fehlt (gen_layout_corpus.py ausführen)");
        return;
    };
    if !have_corpus_fonts() {
        eprintln!("übersprungen: Myriad Pro/Arial nicht installiert");
        return;
    }
    let opts = LayoutOptions::default();
    let mut groups: BTreeMap<String, Stat> = BTreeMap::new();
    let mut py_stat = Stat::default();
    let mut diffs: Vec<String> = Vec::new();
    let mut tabs = Stat::default();
    let mut hb_stat = Stat::default();
    // Rahmentexte fester Breite (TFTi 1, im Korpus alle ohne ATE): Satzhöhe BOTf
    let (mut fixed_n, mut fixed_bot, mut fixed_pm1) = (0usize, 0usize, 0usize);
    let mut fixed_hist: BTreeMap<i64, usize> = BTreeMap::new();
    for rec in &recs {
        let cat = rec["category"].as_str().unwrap_or("");
        let Some(fw) = saved_frame(rec) else { continue };
        let block = block_from(rec);
        if block.kind == FrameKind::FixedWidth {
            let ok_font = block.runs.iter().all(|r| {
                fonts()
                    .resolve(&r.attrs.font, false, false)
                    .is_some_and(|x| !x.substituted)
            });
            if ok_font && !block.runs.is_empty() {
                let lay = layout(&block, fonts(), &opts);
                let sd = lay.frame.bottom as f64 - fw[3].ceil();
                *fixed_hist
                    .entry(sd.clamp(-9.0, 9.0) as i64)
                    .or_insert(0usize) += 1;
                let d = sd.abs();
                fixed_n += 1;
                fixed_bot += (d == 0.0) as usize;
                fixed_pm1 += (d <= 1.0) as usize;
            }
            continue;
        }
        if cat == "Tabulator" {
            let lay = layout(&block, fonts(), &opts);
            tabs.add(frame_arr(&lay.frame), fw, None);
            continue;
        }
        let Some(py) = rec.get("py_frame").and_then(Value::as_array) else {
            continue;
        };
        let py: Vec<f64> = py.iter().filter_map(Value::as_f64).collect();
        let py = [py[0], py[1], py[2], py[3]];
        let lay = layout(&block, fonts(), &opts);
        let got = frame_arr(&lay.frame);
        let hb = layout(
            &block,
            fonts(),
            &LayoutOptions {
                shaping: Shaping::Harfbuzz,
                ..opts
            },
        );
        hb_stat.add(frame_arr(&hb.frame), fw, None);
        let multi = lay.lines.len() > 1;
        let runs = rec["runs"].as_array().map(|r| r.len() > 1).unwrap_or(false);
        let grp = format!(
            "{}{}",
            if multi { "mehrzeilig" } else { "einzeilig" },
            if runs { "/mehrere Läufe" } else { "" }
        );
        groups.entry(grp).or_default().add(got, fw, Some(py));
        groups
            .entry("GESAMT".into())
            .or_default()
            .add(got, fw, Some(py));
        py_stat.add(py, fw, None);
        if got != py && diffs.len() < 40 {
            diffs.push(format!(
                "{} {:?}: rust {:?} py {:?} fw {:?}",
                rec["file"].as_str().unwrap_or(""),
                block.text().chars().take(30).collect::<String>(),
                got,
                py,
                fw
            ));
        }
    }
    for (k, s) in &groups {
        println!("{k}: {s:?}");
    }
    println!("Python-Referenz: {py_stat:?}");
    println!("Tabulator-Texte: {tabs:?}");
    println!("HarfBuzz-Shaping (Ligaturen): {hb_stat:?}");
    println!(
        "Rahmentext (TFTi 1, ohne ATE): Unterkante exakt {fixed_bot}/{fixed_n}, ±1 px {fixed_pm1}/{fixed_n}, Abweichungen {fixed_hist:?}"
    );
    for d in &diffs {
        println!("  Abweichung zu Python: {d}");
    }
    let g = &groups["GESAMT"];
    println!(
        "ERGEBNIS Rahmen alle vier Kanten {}/{} (Python {}/{}), gleich Python {}/{}, Breite {}/{} (Breitenrest {:.1} %)",
        g.all4,
        g.n,
        py_stat.all4,
        py_stat.n,
        g.same_as_py,
        g.n,
        g.width,
        g.n,
        100.0 * (g.n - g.width) as f64 / g.n.max(1) as f64
    );
    assert!(
        g.all4 >= py_stat.all4.saturating_sub(5),
        "Rahmen deutlich schlechter als Python"
    );
}

/// Glyphenpixel gegen die Render-Caches (Auswahl wie `render_check2.py`). Python-Referenz mit
/// abgeschnittener Bézier-Kurve: siehe `pixel_corpus.log`.
#[test]
#[ignore]
fn corpus_pixels() {
    let Some(recs) = read_jsonl(&out_dir().join("pixel_corpus.jsonl")) else {
        eprintln!("übersprungen: pixel_corpus.jsonl fehlt (gen_pixel_corpus.py ausführen)");
        return;
    };
    if !have_corpus_fonts() {
        eprintln!("übersprungen: Korpusschriften fehlen");
        return;
    }
    let opts = LayoutOptions::default();
    let mut by_mode: BTreeMap<i64, (u64, u64, u64)> = BTreeMap::new();
    let mut by_font: BTreeMap<String, (u64, u64, u64, u64)> = BTreeMap::new();
    let (mut exact, mut px, mut py_exact, mut same_py_texts, mut texts) =
        (0u64, 0u64, 0u64, 0usize, 0usize);
    let (mut ink_exact, mut ink_px) = (0u64, 0u64);
    let mut exact_texts = 0usize;
    for rec in &recs {
        let c = &rec["cache"];
        let (x, y, w, h) = (
            i(c, "x").unwrap_or(0),
            i(c, "y").unwrap_or(0),
            i(c, "w").unwrap_or(0),
            i(c, "h").unwrap_or(0),
        );
        let alpha = b64(c["alpha"].as_str().unwrap_or(""));
        if alpha.len() != (w * h) as usize {
            continue;
        }
        let block = block_from(rec);
        let lay = layout(&block, fonts(), &opts);
        let bounds = IRect::new(x as i32, y as i32, (x + w) as i32, (y + h) as i32);
        let m = render::coverage(&lay, Some(bounds));
        let ours = m.data();
        let mut e = 0u64;
        for (k, &r) in alpha.iter().enumerate() {
            let o = ours.get(k).copied().unwrap_or(0);
            e += (o == r) as u64;
            if o > 0 || r > 0 {
                ink_px += 1;
                ink_exact += (o == r) as u64;
            }
        }
        let pe = rec.get("py_exact").and_then(Value::as_u64).unwrap_or(0);
        exact += e;
        px += alpha.len() as u64;
        py_exact += pe;
        same_py_texts += (pe == e) as usize;
        exact_texts += (e == alpha.len() as u64) as usize;
        texts += 1;
        let mode = i(&rec["attrs"], "TAAi").unwrap_or(0);
        {
            let fam = block
                .runs
                .first()
                .map(|r| r.attrs.font.clone())
                .unwrap_or_default();
            let mut ie = 0u64;
            let mut inn = 0u64;
            for (k, &r) in alpha.iter().enumerate() {
                let o = ours.get(k).copied().unwrap_or(0);
                if o > 0 || r > 0 {
                    inn += 1;
                    ie += (o == r) as u64;
                }
            }
            let s = by_font.entry(fam).or_default();
            s.0 += ie;
            s.1 += inn;
            s.2 += 1;
            s.3 += (e == alpha.len() as u64) as u64;
        }
        let s = by_mode.entry(mode).or_default();
        s.0 += e;
        s.1 += alpha.len() as u64;
        s.2 += pe;
    }
    for (f, (e, n, t, x)) in &by_font {
        println!(
            "Schrift {f}: Tintenpixel {:.1} % exakt ({n} px), Texte {t}, pixelgenau {x}",
            100.0 * *e as f64 / (*n).max(1) as f64
        );
    }
    for (m, (e, n, p)) in &by_mode {
        println!(
            "TAAi {m}: {:.2} % exakt (Python {:.2} %), {n} px",
            100.0 * *e as f64 / *n as f64,
            100.0 * *p as f64 / *n as f64
        );
    }
    println!(
        "ERGEBNIS Glyphenpixel {:.2} % exakt ({exact}/{px}), Python {:.2} %, Tintenpixel {:.1} %, Texte pixelgleich zu Python {same_py_texts}/{texts}, pixelgenau zum Original {exact_texts}",
        100.0 * exact as f64 / px.max(1) as f64,
        100.0 * py_exact as f64 / px.max(1) as f64,
        100.0 * ink_exact as f64 / ink_px.max(1) as f64
    );
    assert!(
        exact as f64 >= py_exact as f64 * 0.995,
        "Pixelquote deutlich unter Python"
    );
}

//! Versuch: Höhe alter Rahmentexte ohne ATE (FW 8/CS3, `TFTi 1`) gegen den gespeicherten Rahmen.
//! `cargo test -p fp-text --release --test legacy_fit -- --ignored --nocapture`

mod common;

use std::collections::BTreeMap;

use common::*;
use fp_text::*;

#[test]
#[ignore]
fn legacy_heights() {
    let Some(recs) = read_jsonl(&out_dir().join("layout_corpus.jsonl")) else {
        return;
    };
    let opts = LayoutOptions::default();
    let mut by_key: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut shown = 0;
    for rec in &recs {
        let Some(fw) = saved_frame(rec) else { continue };
        let block = block_from(rec);
        if block.kind != FrameKind::FixedWidth || block.runs.is_empty() {
            continue;
        }
        let Some(rf) = fonts().resolve(&block.runs[0].attrs.font, false, false) else {
            continue;
        };
        if rf.substituted {
            continue;
        }
        let lay = layout(&block, fonts(), &opts);
        let a = &block.runs[0].attrs;
        let font = &rf.font;
        let (asc, desc) = (font.ascent(), font.descent());
        let n = lay.lines.len();
        let top = fw[1];
        let fh = fw[3] - top;
        let oh = lay.frame.bottom as f64 - lay.frame.top as f64;
        let d = (lay.frame.bottom as f64 - fw[3].ceil()) as i64;
        let key = format!(
            "{} {}px ld{:.2} m{:?} n{}",
            a.font,
            a.size,
            a.leading,
            a.leading_mode,
            n.min(4)
        );
        let e = by_key.entry(key.clone()).or_default();
        e.0 += 1;
        e.1 += (d == 0) as usize;
        if d != 0 && shown < 60 {
            shown += 1;
            println!(
                "{key}: fw h {fh:.2} (top {top}, bot {}) rust h {oh} d {d}; asc {asc:.4} desc {desc:.4} lines {:?}",
                fw[3],
                lay.lines
                    .iter()
                    .map(|l| l.baseline - top)
                    .collect::<Vec<_>>()
            );
        }
    }
    for (k, (n, ok)) in &by_key {
        if *n >= 3 {
            println!("{k}: {ok}/{n}");
        }
    }
}

//! Converts an OpenCV stump-based Haar cascade (`haarcascade_frontalface_default.xml`, new
//! `opencv-cascade-classifier` format, no tilted features) into the compact binary form that
//! `fp_ai::face` embeds (`crates/fp-ai/data/haar_frontalface_default.bin`). The output is
//! byte-for-byte reproducible; the face module's tests compare against the embedded copy.
//!
//! `cargo run -p fp-ai --example haar_convert -- <cascade.xml> <out.bin>`
//!
//! Binary layout (little endian):
//! `"FPHAAR01"`, `u16` window width, `u16` window height, `u16` stage count, `u16` feature count;
//! per feature: `u8` rect count, per rect `u8 x, u8 y, u8 w, u8 h, f32 weight`;
//! per stage: `u16` weak count, `f32` stage threshold, per weak classifier
//! `u16 feature, f32 threshold, f32 left leaf, f32 right leaf`.

use std::process::ExitCode;

fn between<'a>(s: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let a = s.find(open)? + open.len();
    let b = s[a..].find(close)? + a;
    Some(&s[a..b])
}

fn numbers(s: &str) -> Result<Vec<f64>, String> {
    s.split_whitespace()
        .map(|t| {
            t.parse::<f64>()
                .map_err(|e| format!("bad number {t:?}: {e}"))
        })
        .collect()
}

/// Weak classifier: feature index, threshold, left leaf, right leaf.
type Stump = (u16, f32, f32, f32);
/// Feature rect: x, y, w, h, weight.
type Rect = (u8, u8, u8, u8, f32);

fn convert(xml: &str) -> Result<Vec<u8>, String> {
    if xml.contains("<tilted>1") {
        return Err("tilted features are not supported".into());
    }
    let ww = between(xml, "<width>", "</width>").ok_or("no <width>")?;
    let wh = between(xml, "<height>", "</height>").ok_or("no <height>")?;
    let (ww, wh): (u16, u16) = (
        ww.trim().parse().map_err(|_| "bad width")?,
        wh.trim().parse().map_err(|_| "bad height")?,
    );
    let stages_txt = between(xml, "<stages>", "</stages>").ok_or("no <stages>")?;
    let feats_txt = between(xml, "<features>", "</features>").ok_or("no <features>")?;

    // stages: every chunk after a <stageThreshold> holds the threshold and its weak classifiers
    let mut stages: Vec<(f32, Vec<Stump>)> = Vec::new();
    for chunk in stages_txt.split("<stageThreshold>").skip(1) {
        let thr = numbers(chunk.split('<').next().unwrap_or(""))?;
        let thr = *thr.first().ok_or("empty stage threshold")? as f32;
        let mut weak = Vec::new();
        let mut rest = chunk;
        while let Some(nodes) = between(rest, "<internalNodes>", "</internalNodes>") {
            let leaves = between(rest, "<leafValues>", "</leafValues>").ok_or("no leafValues")?;
            let n = numbers(nodes)?;
            let l = numbers(leaves)?;
            if n.len() != 4 || l.len() != 2 || n[0] != 0.0 || n[1] != -1.0 {
                return Err("only single-node stumps are supported".into());
            }
            weak.push((n[2] as u16, n[3] as f32, l[0] as f32, l[1] as f32));
            let end = rest.find("</leafValues>").ok_or("no </leafValues>")? + 13;
            rest = &rest[end..];
        }
        stages.push((thr, weak));
    }

    let mut feats: Vec<Vec<Rect>> = Vec::new();
    for chunk in feats_txt.split("<rects>").skip(1) {
        let body = chunk.split("</rects>").next().unwrap_or("");
        let mut rects = Vec::new();
        for r in body.split("<_>").skip(1) {
            let v = numbers(r.split('<').next().unwrap_or(""))?;
            if v.len() != 5 {
                return Err(format!("bad rect {r:?}"));
            }
            rects.push((v[0] as u8, v[1] as u8, v[2] as u8, v[3] as u8, v[4] as f32));
        }
        if rects.is_empty() || rects.len() > 3 {
            return Err("feature needs 1 to 3 rects".into());
        }
        feats.push(rects);
    }
    if stages.is_empty() || feats.is_empty() {
        return Err("empty cascade".into());
    }

    let mut out = Vec::new();
    out.extend_from_slice(b"FPHAAR01");
    for v in [ww, wh, stages.len() as u16, feats.len() as u16] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    for f in &feats {
        out.push(f.len() as u8);
        for &(x, y, w, h, wt) in f {
            out.extend_from_slice(&[x, y, w, h]);
            out.extend_from_slice(&wt.to_le_bytes());
        }
    }
    for (thr, weak) in &stages {
        out.extend_from_slice(&(weak.len() as u16).to_le_bytes());
        out.extend_from_slice(&thr.to_le_bytes());
        for &(fi, t, l, r) in weak {
            if fi as usize >= feats.len() {
                return Err(format!("feature index {fi} out of range"));
            }
            out.extend_from_slice(&fi.to_le_bytes());
            for x in [t, l, r] {
                out.extend_from_slice(&x.to_le_bytes());
            }
        }
    }
    Ok(out)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [inp, out] = args.as_slice() else {
        eprintln!("usage: haar_convert <cascade.xml> <out.bin>");
        return ExitCode::from(2);
    };
    let xml = match std::fs::read_to_string(inp) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("{inp}: {e}");
            return ExitCode::FAILURE;
        }
    };
    match convert(&xml) {
        Ok(bin) => {
            if let Err(e) = std::fs::write(out, &bin) {
                eprintln!("{out}: {e}");
                return ExitCode::FAILURE;
            }
            println!("{out}: {} bytes", bin.len());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{inp}: {e}");
            ExitCode::FAILURE
        }
    }
}

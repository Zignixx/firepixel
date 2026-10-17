//! Gemeinsame Helfer der fp-text-Tests: Golden-JSON (Python-Extraktion von `TXTv`) → [`TextBlock`].
#![allow(dead_code)]

use std::path::PathBuf;

use fp_color::Argb;
use fp_geom::{BezierContour, BezierNode, Point, Rect};
use fp_text::*;
use serde_json::Value;

/// Ordner der großen Golden-Daten (nicht im Crate): `tools\golden\fp-text\out`.
pub fn out_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tools")
        .join("golden")
        .join("fp-text")
        .join("out")
}

/// Kleine Golden-Daten im Crate.
pub fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
}

/// JSONL-Datei lesen; `None`, wenn sie fehlt.
pub fn read_jsonl(p: &std::path::Path) -> Option<Vec<Value>> {
    let s = std::fs::read_to_string(p).ok()?;
    Some(
        s.lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect(),
    )
}

pub fn f(v: &Value, k: &str) -> Option<f64> {
    v.get(k).and_then(Value::as_f64)
}

pub fn i(v: &Value, k: &str) -> Option<i64> {
    v.get(k)
        .and_then(|x| x.as_i64().or_else(|| x.as_f64().map(|f| f as i64)))
}

pub fn b(v: &Value, k: &str) -> Option<bool> {
    v.get(k).and_then(Value::as_bool)
}

/// Attributblock eines Laufs (bzw. des Kopfs) → Patch.
pub fn patch(v: &Value, header: bool) -> CharAttrsPatch {
    let font = if header {
        v.get("CFTs")
            .and_then(Value::as_str)
            .or_else(|| v.get("FONs").and_then(Value::as_str))
    } else {
        v.get("FONs").and_then(Value::as_str)
    };
    CharAttrsPatch {
        font: font.map(str::to_string),
        size: f(v, "PTSf"),
        color: i(v, "FCLi").map(|c| Argb(c as u32).to_rgba8()),
        bold: b(v, "BOLb"),
        italic: b(v, "ITLb"),
        underline: b(v, "UNDb"),
        h_scale: f(v, "HSCf"),
        kerning: f(v, "KRNf"),
        tracking: f(v, "RKNf"),
        baseline_shift: f(v, "BLSf"),
        leading: f(v, "LEDf"),
        leading_mode: i(v, "LDMi").map(LeadingMode::from_ldmi),
        alignment: i(v, "JSTi").map(Alignment::from_jsti),
        indent: f(v, "PINf"),
        space_before: f(v, "PSBf"),
        space_after: f(v, "PSAf"),
    }
}

/// Golden-Datensatz (attrs, runs, story_kern, tap, fill_edge) → TextBlock.
pub fn block_from(rec: &Value) -> TextBlock {
    let a = &rec["attrs"];
    let mut initial = CharAttrs {
        leading: 1.2,
        ..CharAttrs::default()
    };
    patch(a, true).apply(&mut initial);
    let blocks: Vec<(CharAttrsPatch, String)> = rec["runs"]
        .as_array()
        .map(|rs| {
            rs.iter()
                .map(|r| {
                    (
                        patch(r, false),
                        r.get("TRNs")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    let runs = resolve_runs(&initial, &blocks);
    let left = f(a, "LFAf").or_else(|| f(a, "LFTf")).unwrap_or(0.0) as f32;
    let top = f(a, "TOAf").or_else(|| f(a, "TOPf")).unwrap_or(0.0) as f32;
    let right = f(a, "RIAf").or_else(|| f(a, "RITf")).unwrap_or(0.0) as f32;
    let bottom = f(a, "BOAf").or_else(|| f(a, "BOTf")).unwrap_or(0.0) as f32;
    let story_kerning = rec.get("story_kern").and_then(Value::as_array).map(|v| {
        v.iter()
            .map(|e| StoryKernRun {
                len: e.get(0).and_then(Value::as_u64).unwrap_or(0) as usize,
                kind: e
                    .get(1)
                    .and_then(Value::as_i64)
                    .map(KerningKind::from_value),
            })
            .collect()
    });
    let path = rec.get("tap").filter(|t| t.is_object()).map(|t| TextPath {
        mode: PathTextMode::from_tami(i(t, "TAMi").unwrap_or(0)),
        offset: f(t, "CHOf").unwrap_or(0.0),
        contours: t
            .get("contours")
            .and_then(Value::as_array)
            .map(|cs| {
                cs.iter()
                    .map(|c| {
                        let nodes = c["nodes"]
                            .as_array()
                            .map(|ns| {
                                ns.iter()
                                    .map(|n| {
                                        let g = |k: usize| {
                                            n.get(k).and_then(Value::as_f64).unwrap_or(0.0) as f32
                                        };
                                        BezierNode::new(
                                            Point::new(g(0), g(1)),
                                            Point::new(g(2), g(3)),
                                            Point::new(g(4), g(5)),
                                        )
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                        BezierContour::new(
                            nodes,
                            c.get("closed").and_then(Value::as_bool).unwrap_or(false),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default(),
        reversed: false,
    });
    let feti = rec.get("fill_edge").and_then(Value::as_i64).or_else(|| {
        rec.get("pat")
            .and_then(|p| p.get("fill_edge"))
            .and_then(Value::as_i64)
    });
    TextBlock {
        kind: FrameKind::from_tfti(i(a, "TFTi").unwrap_or(0)),
        bounds: Rect::new(left, top, right, bottom),
        runs,
        auto_kern: b(a, "ATKb").unwrap_or(true),
        story_kerning,
        orientation: Orientation::from_orii(i(a, "ORIi").unwrap_or(0)),
        antialias: AntiAlias::from_tags(
            i(a, "TAAi").unwrap_or(0),
            i(a, "TA1i").unwrap_or(192),
            i(a, "TA2i").unwrap_or(64),
            i(a, "TOSi").unwrap_or(8),
            feti,
        ),
        path,
    }
}

/// Gespeicherter Rahmen `LFTf TOPf RITf BOTf`.
pub fn saved_frame(rec: &Value) -> Option<[f64; 4]> {
    let a = &rec["attrs"];
    Some([f(a, "LFTf")?, f(a, "TOPf")?, f(a, "RITf")?, f(a, "BOTf")?])
}

pub fn frame_arr(r: &Rect) -> [f64; 4] {
    [r.left as f64, r.top as f64, r.right as f64, r.bottom as f64]
}

/// Base64 dekodieren (Standardalphabet, ohne Abhängigkeit).
pub fn b64(s: &str) -> Vec<u8> {
    let val = |c: u8| -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a') as u32 + 26),
            b'0'..=b'9' => Some((c - b'0') as u32 + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    };
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut acc = 0u32;
    let mut bits = 0;
    for c in s.bytes() {
        let Some(v) = val(c) else { continue };
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    out
}

/// Systemschriften (einmal je Testprozess).
pub fn fonts() -> &'static FontDb {
    static DB: std::sync::OnceLock<FontDb> = std::sync::OnceLock::new();
    DB.get_or_init(FontDb::system)
}

/// Sind die Korpusschriften (Myriad Pro, Arial) installiert?
pub fn have_corpus_fonts() -> bool {
    let db = fonts();
    db.find_ps("MyriadPro-Regular").is_some() && db.find_ps("ArialMT").is_some()
}

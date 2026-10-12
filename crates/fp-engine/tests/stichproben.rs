//! Stichproben gegen das Referenzprogramm: dieselben Abläufe wie `tests\data\stichproben.jsf`
//! (per `re\scripts\fwsteuerung\jsf-ausfuehren.ps1` im Original ausgeführt, Protokoll
//! `re\_testdateien\stichproben.log`); die erwarteten Grenzen stehen unten als Zahlen.

use crate::common;

use common::*;
use fp_engine::Session;
use serde_json::json;

fn fmt(v: f32) -> String {
    let r = (v * 100.0).round() / 100.0;
    if r.fract() == 0.0 { format!("{}", r as i64) } else { format!("{r}") }
}

fn sel(s: &Session) -> String {
    let d = s.active().expect("doc");
    let mut ids = d.selection.elements.clone();
    // Reihenfolge wie fw.selection: oben zuerst
    let paths = fp_engine::docops::paths_sorted(&d.doc, d.page, &ids);
    ids = paths.iter().filter_map(|p| d.doc.element_at(p).map(|e| e.id)).collect();
    ids.iter()
        .map(|id| {
            let b = bounds(s, *id);
            format!("{},{},{},{}", fmt(b.left), fmt(b.top), fmt(b.width()), fmt(b.height()))
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

fn fresh() -> Session {
    doc_session(300.0, 200.0)
}

fn run(name: &str) -> String {
    let mut s = fresh();
    let r = |s: &mut Session, l, t, rr, b| rect(s, l, t, rr, b);
    match name {
        "rotate90cw" | "rotate90ccw" => {
            r(&mut s, 10.0, 10.0, 60.0, 40.0);
            ex(&mut s, if name == "rotate90cw" { "modify.transform.rotate90DegreeCw" } else { "modify.transform.rotate90DegreeCcw" }, json!({}));
        }
        "flipH2" | "alignLeft" | "alignCenterV" | "group" | "union" | "punch" | "intersect" => {
            if matches!(name, "union" | "punch" | "intersect") {
                r(&mut s, 10.0, 10.0, 60.0, 40.0);
                r(&mut s, 40.0, 20.0, 90.0, 70.0);
            } else {
                r(&mut s, 10.0, 10.0, 60.0, 40.0);
                r(&mut s, 100.0, 50.0, 130.0, 90.0);
            }
            ex(&mut s, "select.all", json!({}));
            let c = match name {
                "flipH2" => "modify.transform.flipHorizontal",
                "alignLeft" => "modify.align.left",
                "alignCenterV" => "modify.align.centerVertical",
                "group" => "modify.group",
                "union" => "modify.combinePaths.union",
                "punch" => "modify.combinePaths.punch",
                _ => "modify.combinePaths.intersect",
            };
            ex(&mut s, c, json!({}));
        }
        "distributeWidths" => {
            r(&mut s, 10.0, 10.0, 60.0, 40.0);
            r(&mut s, 100.0, 50.0, 130.0, 90.0);
            r(&mut s, 200.0, 20.0, 210.0, 30.0);
            ex(&mut s, "select.all", json!({}));
            ex(&mut s, "modify.align.distributeWidths", json!({}));
        }
        "duplicate" => {
            r(&mut s, 10.0, 10.0, 60.0, 40.0);
            ex(&mut s, "edit.duplicate", json!({}));
        }
        "scale50" => {
            r(&mut s, 10.0, 10.0, 60.0, 40.0);
            ex(&mut s, "modify.transform.numericTransform", json!({"mode": "scale", "scaleX": 50, "scaleY": 50}));
        }
        "imageSize150" => {
            r(&mut s, 10.0, 10.0, 60.0, 40.0);
            ex(&mut s, "modify.canvas.imageSize", json!({"width": 450, "height": 300}));
            ex(&mut s, "select.all", json!({}));
            let d = s.active().expect("doc");
            let c = &d.doc.pages[0].canvas;
            return format!("{} doc {}x{}", sel(&s), c.width, c.height);
        }
        "canvasSizeCenter" => {
            r(&mut s, 10.0, 10.0, 60.0, 40.0);
            ex(&mut s, "modify.canvas.canvasSize", json!({"width": 400, "height": 250, "anchor": "center"}));
            ex(&mut s, "select.all", json!({}));
            let d = s.active().expect("doc");
            let c = &d.doc.pages[0].canvas;
            return format!("{} doc {}x{}", sel(&s), c.width, c.height);
        }
        _ => {}
    }
    sel(&s)
}

/// Erwartete Werte aus dem Protokoll des Originals (`stichproben.log`).
const EXPECTED: &[(&str, &str)] = &[
    ("rotate90cw", "20,0,30,50"),
    ("rotate90ccw", "20,0,30,50"),
    ("flipH2", "10,50,30,40 | 80,10,50,30"),
    ("alignLeft", "10,50,30,40 | 10,10,50,30"),
    ("alignCenterV", "55,50,30,40 | 45,10,50,30"),
    ("duplicate", "20,20,50,30"),
    ("scale50", "22.5,17.5,25,15"),
    ("group", "10,10,120,80"),
    ("imageSize150", "15,15,75,45 doc 450x300"),
    ("canvasSizeCenter", "60,35,50,30 doc 400x250"),
];

#[test]
fn spot_checks_against_reference() {
    let names = ["rotate90cw", "rotate90ccw", "flipH2", "alignLeft", "alignCenterV", "distributeWidths", "duplicate", "scale50", "group", "imageSize150", "canvasSizeCenter", "union", "punch", "intersect"];
    let mut same = 0;
    let mut diffs = Vec::new();
    for n in names {
        let got = run(n);
        println!("{n} = {got}");
        if let Some((_, want)) = EXPECTED.iter().find(|(k, _)| *k == n) {
            if *want == got {
                same += 1;
            } else {
                diffs.push(format!("{n}: Original {want}, Firepixel {got}"));
            }
        }
    }
    println!("Stichproben gleich: {same}/{}", EXPECTED.len());
    assert!(diffs.is_empty(), "{diffs:#?}");
}

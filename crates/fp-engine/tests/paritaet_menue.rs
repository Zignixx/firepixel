//! Menüfreigabe wie im Original: Tabelle `tests\data\paritaet_menue.tsv` (aus `docs\PARITAET_B.md` §1,
//! gemessen im Original mit gleichem Testdokument) gegen `Session::command_state` in fünf Lagen:
//! S1 ohne Auswahl, S2 ein Rechteck, S3 zwei Rechtecke, S4 Text, S5 Bitmap.

use crate::common;

use common::*;
use fp_engine::Session;
use fp_model::ElementId;
use serde_json::json;
use std::collections::BTreeMap;

/// Bewusste Abweichungen (Begründung): das Original graut aus, weil Fremdsoftware fehlt.
const EXCEPTIONS: &[(&str, &str)] = &[("Datei > Bridge durchsuchen", "Firepixel ersetzt den externen Dateibrowser durch eine eigene Dateiliste; im Original nur grau, weil Bridge nicht installiert ist")];

fn norm(s: &str) -> String {
    s.trim().trim_end_matches('…').trim_end_matches("...").trim().to_string()
}

/// Testdokument wie im Abgleich: Bitmap unten, darüber Text „Hallo“, darüber zwei Rechtecke.
fn fixture() -> (Session, [ElementId; 4]) {
    let mut s = doc_session(500.0, 400.0);
    let bmp = bitmap(&mut s, 300, 250, 60, 60, [10, 120, 200, 255]);
    let text = ElementId(ex(&mut s, "draw.text", json!({"text": "Hallo", "anchor": [40, 300], "size": 24}))["id"].as_u64().unwrap_or(0));
    let r1 = rect(&mut s, 20.0, 20.0, 120.0, 90.0);
    let r2 = rect(&mut s, 160.0, 40.0, 260.0, 120.0);
    ex(&mut s, "select.none", json!({}));
    (s, [r1, r2, text, bmp])
}

#[test]
fn menu_enablement_matches_original() {
    let table = include_str!("data/paritaet_menue.tsv");
    let reg = fp_engine::CommandRegistry::with_defaults();
    let by_path: BTreeMap<String, String> = reg
        .iter()
        .filter(|c| !c.info().menu_path.is_empty())
        .map(|c| (c.info().menu_path.iter().map(|p| norm(p)).collect::<Vec<_>>().join(" > "), c.info().name.clone()))
        .collect();
    let (s1, ids) = fixture();
    let lagen: Vec<(&str, Vec<ElementId>)> = vec![("S1", vec![]), ("S2", vec![ids[0]]), ("S3", vec![ids[0], ids[1]]), ("S4", vec![ids[2]]), ("S5", vec![ids[3]])];
    let mut sessions = Vec::new();
    for (_, sel) in &lagen {
        let (mut s, _) = fixture();
        if !sel.is_empty() {
            select(&mut s, sel);
        }
        sessions.push(s);
    }
    drop(s1);
    let (mut checked, mut same, mut submenus) = (0, 0, 0);
    let mut diffs = Vec::new();
    for line in table.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()) {
        let Some((path, states)) = line.split_once('\t') else { continue };
        let path = path.split(" > ").map(norm).collect::<Vec<_>>().join(" > ");
        let Some(name) = by_path.get(&path) else {
            submenus += 1;
            continue;
        };
        if EXCEPTIONS.iter().any(|(p, _)| *p == path) {
            continue;
        }
        checked += 1;
        let want: Vec<bool> = states.chars().map(|c| c == '1').collect();
        let got: Vec<bool> = sessions.iter().map(|s| s.command_state(name).0).collect();
        if want == got {
            same += 1;
        } else {
            let f = |v: &[bool]| v.iter().map(|b| if *b { "an" } else { "aus" }).collect::<Vec<_>>().join(" ");
            diffs.push(format!("{path} ({name}): Original {} – Firepixel {}", f(&want), f(&got)));
        }
    }
    println!("Menüfreigabe gleich: {same}/{checked} (Untermenüs/ohne Befehl: {submenus}, Ausnahmen: {})", EXCEPTIONS.len());
    for d in &diffs {
        println!("ABWEICHUNG {d}");
    }
    assert!(diffs.is_empty(), "{} Abweichungen", diffs.len());
}

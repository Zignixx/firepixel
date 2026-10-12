//! I18N-QA: fasst die Ergebnisse der Sprachläufe (`i18n_qa_de`, `i18n_qa_en`, `i18n_qa_qps`) zu
//! Zahlen je Bereich zusammen und schreibt sie in `docs\I18N_PRUEFUNG.md` (Abschnitt zwischen den
//! Markierungen `<!-- AUTO:START -->` und `<!-- AUTO:END -->`).
//! `cargo test -j 4 -p fp-ui --test i18n_qa_bericht -- --ignored`

mod i18n_qa_common;

use i18n_qa_common::{AreaResult, Finding};
use std::collections::{BTreeMap, BTreeSet};

const AREAS: &[(&str, &str)] = &[
    ("hauptfenster", "Hauptfenster (3 Designs, leer, Symbolmodus)"),
    ("menues", "Menüs und Untermenüs"),
    ("bedienfelder", "Bedienfelder und Bedienfeldmenüs"),
    ("inspektor", "Inspektor (Elementarten, Werkzeuge)"),
    ("dialoge", "Dialoge"),
    ("kontextmenues", "Kontextmenüs"),
    ("statuszeile", "Statuszeile"),
];

const LAYOUT: &[&str] = &["abgeschnitten", "fensterrand", "ueberlappung", "in_element", "element_rand"];

fn load(lang: &str, area: &str) -> Option<AreaResult> {
    let p = i18n_qa_common::out_root().join("aktuell").join(format!("{lang}_{area}.json"));
    serde_json::from_str(&std::fs::read_to_string(p).ok()?).ok()
}

fn uniq<'a>(f: impl Iterator<Item = &'a Finding>) -> usize {
    f.map(|x| (x.scene.clone(), x.text.clone(), x.kind.clone())).collect::<BTreeSet<_>>().len()
}

/// Überläufe, die es in Deutsch im selben Bild und derselben Größe nicht gibt (je Art gezählt).
fn new_vs_de<'a>(r: &'a AreaResult, de: Option<&AreaResult>) -> Vec<&'a Finding> {
    let mut base: BTreeMap<(String, String, String), usize> = BTreeMap::new();
    if let Some(de) = de {
        for f in de.findings.iter().filter(|f| LAYOUT.contains(&f.kind.as_str())) {
            *base.entry((f.scene.clone(), f.cfg.clone(), f.kind.clone())).or_default() += 1;
        }
    }
    let mut out = Vec::new();
    for f in r.findings.iter().filter(|f| LAYOUT.contains(&f.kind.as_str())) {
        let k = (f.scene.clone(), f.cfg.clone(), f.kind.clone());
        match base.get_mut(&k) {
            Some(n) if *n > 0 => *n -= 1,
            _ => out.push(f),
        }
    }
    out
}

fn esc(s: &str) -> String {
    s.replace('|', "/").replace('\n', "⏎").chars().take(160).collect()
}

#[test]
#[ignore]
fn bericht() {
    let mut md = String::new();
    let now = std::process::Command::new("cmd").args(["/C", "echo %DATE% %TIME%"]).output().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
    md.push_str(&format!("Stand des Laufs: {now} (automatisch erzeugt von `crates\\fp-ui\\tests\\i18n_qa_bericht.rs`)\n\n"));
    md.push_str("| Bereich | Bilder | Rohschlüssel de/en/qps | unmarkiert qps (Texte) | Überläufe en (neu ggü. de) | Überläufe qps (neu ggü. de) | Überläufe de (Bestand) | Deutsch geändert | nicht herstellbar de/en/qps |\n|---|---|---|---|---|---|---|---|---|\n");
    let mut totals = [0usize; 10];
    let mut details = String::new();
    for (area, title) in AREAS {
        let de = load("de", area);
        let en = load("en", area);
        let qps = load("qps", area);
        let raw = |r: &Option<AreaResult>| r.as_ref().map(|r| uniq(r.findings.iter().filter(|f| f.kind == "rohschluessel")).to_string()).unwrap_or("–".into());
        let unm: Option<BTreeSet<String>> = qps.as_ref().map(|r| r.findings.iter().filter(|f| f.kind == "unmarkiert").map(|f| f.text.clone()).collect());
        let lay = |r: &Option<AreaResult>| -> Option<(usize, usize)> { r.as_ref().map(|r| (uniq(r.findings.iter().filter(|f| LAYOUT.contains(&f.kind.as_str()))), uniq(new_vs_de(r, de.as_ref()).into_iter()))) };
        let fmt_lay = |x: Option<(usize, usize)>| x.map(|(a, b)| format!("{a} ({b})")).unwrap_or("–".into());
        let de_lay = de.as_ref().map(|r| uniq(r.findings.iter().filter(|f| LAYOUT.contains(&f.kind.as_str())))).map(|n| n.to_string()).unwrap_or("–".into());
        let changed = de.as_ref().map(|r| r.de_changed.len().to_string()).unwrap_or("–".into());
        let failed = [&de, &en, &qps].iter().map(|r| r.as_ref().map(|r| r.failed.len().to_string()).unwrap_or("–".into())).collect::<Vec<_>>().join("/");
        let scenes = de.as_ref().or(en.as_ref()).or(qps.as_ref()).map(|r| r.scenes).unwrap_or(0);
        md.push_str(&format!(
            "| {title} | {scenes} | {}/{}/{} | {} | {} | {} | {de_lay} | {changed} | {failed} |\n",
            raw(&de),
            raw(&en),
            raw(&qps),
            unm.as_ref().map(|s| s.len().to_string()).unwrap_or("–".into()),
            fmt_lay(lay(&en)),
            fmt_lay(lay(&qps))
        ));
        totals[0] += scenes;
        for (i, r) in [&de, &en, &qps].iter().enumerate() {
            if let Some(r) = r {
                totals[1 + i] += uniq(r.findings.iter().filter(|f| f.kind == "rohschluessel"));
            }
        }
        totals[4] += unm.as_ref().map(|s| s.len()).unwrap_or(0);
        totals[5] += lay(&en).map(|x| x.1).unwrap_or(0);
        totals[6] += lay(&qps).map(|x| x.1).unwrap_or(0);
        totals[7] += de.as_ref().map(|r| uniq(r.findings.iter().filter(|f| LAYOUT.contains(&f.kind.as_str())))).unwrap_or(0);
        totals[8] += de.as_ref().map(|r| r.de_changed.len()).unwrap_or(0);

        // Einzelheiten
        details.push_str(&format!("\n### {title}\n\n"));
        let mut any = false;
        for (lang, r) in [("de", &de), ("en", &en), ("qps", &qps)] {
            let Some(r) = r else { continue };
            let raws: Vec<&Finding> = r.findings.iter().filter(|f| f.kind == "rohschluessel").collect();
            if !raws.is_empty() {
                any = true;
                details.push_str(&format!("**Rohschlüssel {lang}:** "));
                let set: BTreeSet<String> = raws.iter().map(|f| format!("`{}` ({})", esc(&f.detail), f.scene)).collect();
                details.push_str(&set.into_iter().take(40).collect::<Vec<_>>().join(" · "));
                details.push_str("\n\n");
            }
            if !r.failed.is_empty() {
                any = true;
                details.push_str(&format!("**Nicht herstellbar {lang}:** {}\n\n", r.failed.iter().take(20).map(|s| esc(s)).collect::<Vec<_>>().join(" · ")));
            }
        }
        if let Some(r) = &qps {
            let mut by: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
            for f in r.findings.iter().filter(|f| f.kind == "unmarkiert") {
                by.entry(f.text.clone()).or_default().insert(f.scene.clone());
            }
            if !by.is_empty() {
                any = true;
                details.push_str(&format!("**Unmarkierte Texte qps ({}):**\n\n| Text | Bilder |\n|---|---|\n", by.len()));
                for (t, s) in by.iter().take(150) {
                    let sc: Vec<&String> = s.iter().take(4).collect();
                    details.push_str(&format!("| {} | {}{} |\n", esc(t), sc.iter().map(|x| x.as_str()).collect::<Vec<_>>().join(", "), if s.len() > 4 { format!(" … (+{})", s.len() - 4) } else { String::new() }));
                }
                details.push('\n');
            }
        }
        for (lang, r) in [("en", &en), ("qps", &qps)] {
            let Some(r) = r else { continue };
            let new = new_vs_de(r, de.as_ref());
            if new.is_empty() {
                continue;
            }
            any = true;
            let mut rows: BTreeMap<(String, String, String), (BTreeSet<String>, String)> = BTreeMap::new();
            for f in &new {
                let e = rows.entry((f.scene.clone(), f.kind.clone(), f.text.clone())).or_insert_with(|| (BTreeSet::new(), f.detail.clone()));
                e.0.insert(f.cfg.clone());
            }
            details.push_str(&format!("**Überläufe {lang}, neu gegenüber Deutsch ({}):**\n\n| Bild | Art | Text | Größen | Lage |\n|---|---|---|---|---|\n", rows.len()));
            for ((scene, kind, text), (cfgs, det)) in rows.iter().take(if lang == "en" { 300 } else { 120 }) {
                details.push_str(&format!("| {scene} | {kind} | {} | {} | {} |\n", esc(text), cfgs.iter().cloned().collect::<Vec<_>>().join(", "), esc(det)));
            }
            details.push('\n');
        }
        if let Some(r) = &de
            && !r.de_changed.is_empty()
        {
            any = true;
            details.push_str(&format!("**Deutsch geändert gegenüber dem Stand vor Phase 6 ({}):**\n\n", r.de_changed.len()));
            for c in r.de_changed.iter().take(80) {
                details.push_str(&format!("- {}\n", esc(c).replace('`', "'")));
            }
            details.push('\n');
        }
        if !any {
            details.push_str("keine Befunde\n");
        }
    }
    md.push_str(&format!(
        "| **Summe** | **{}** | **{}/{}/{}** | **{}** | **({})** | **({})** | **{}** | **{}** | |\n\n",
        totals[0], totals[1], totals[2], totals[3], totals[4], totals[5], totals[6], totals[7], totals[8]
    ));
    md.push_str("Lesart: Rohschlüssel und unmarkierte Texte zählen verschiedene Texte; Überläufe zählen verschiedene (Bild, Text, Art) über alle vier Größen (1024×768 und 1920×1080, je 100 % und 150 %). In Klammern: nur die Überläufe, die es im selben Bild und derselben Größe in Deutsch nicht gibt (Folge der Übersetzung). „Überläufe de (Bestand)“ sind Layoutfehler, die schon vor Phase 6 bestanden bzw. sprachunabhängig sind.\n");
    md.push_str("\n## Einzelheiten (automatisch)\n");
    md.push_str(&details);

    let path = i18n_qa_common::repo().join("docs").join("I18N_PRUEFUNG.md");
    let old = std::fs::read_to_string(&path).unwrap_or_else(|_| "# Prüfung der Mehrsprachigkeit (I18N-QA)\n\n<!-- AUTO:START -->\n<!-- AUTO:END -->\n".into());
    let (a, rest) = old.split_once("<!-- AUTO:START -->").unwrap_or((&old, ""));
    let (_, b) = rest.split_once("<!-- AUTO:END -->").unwrap_or(("", ""));
    let new = format!("{a}<!-- AUTO:START -->\n{md}\n<!-- AUTO:END -->{b}");
    std::fs::write(&path, new).expect("I18N_PRUEFUNG.md schreiben");
    eprintln!("Summe: {totals:?}");
}

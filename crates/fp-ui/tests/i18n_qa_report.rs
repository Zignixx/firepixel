//! I18N-QA: sums up the results of the language runs (`i18n_qa_de`, `i18n_qa_en`, `i18n_qa_qps`)
//! as numbers per area and writes them into `docs\I18N_PRUEFUNG.md` (section between the markers
//! `<!-- AUTO:START -->` and `<!-- AUTO:END -->`). Results written before BA-086 (German area
//! names, scene ids and finding kinds) are read through `i18n_qa_common::load_result`.
//! `cargo test -j 4 -p fp-ui --test all i18n_qa_report:: -- --ignored`

use crate::i18n_qa_common;

use i18n_qa_common::{AreaResult, Finding, KIND_RAW_KEY, KIND_UNMARKED, LAYOUT_KINDS as LAYOUT};
use std::collections::{BTreeMap, BTreeSet};

const AREAS: &[(&str, &str)] = &[
    ("main_window", "Main window (3 themes, empty, iconic mode)"),
    ("menus", "Menus and submenus"),
    ("panels", "Panels and panel menus"),
    ("inspector", "Inspector (element kinds, tools)"),
    ("dialogs", "Dialogs"),
    ("context_menus", "Context menus"),
    ("status_bar", "Status bar"),
];

fn load(lang: &str, area: &str) -> Option<AreaResult> {
    load_in(i18n_qa_common::MODE_CURRENT, lang, area)
}

fn load_in(mode: &str, lang: &str, area: &str) -> Option<AreaResult> {
    i18n_qa_common::load_result(mode, lang, area).ok().flatten()
}

fn uniq<'a>(f: impl Iterator<Item = &'a Finding>) -> usize {
    f.map(|x| (x.scene.clone(), x.text.clone(), x.kind.clone())).collect::<BTreeSet<_>>().len()
}

/// Overflows that German does not have in the same image at the same size (counted per kind).
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
fn report() {
    let mut md = String::new();
    let now = std::process::Command::new("cmd").args(["/C", "echo %DATE% %TIME%"]).output().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
    md.push_str(&format!("State of the run: {now} (generated automatically by `crates\\fp-ui\\tests\\i18n_qa_report.rs`)\n\n"));
    md.push_str("| Area | Images | Raw keys de/en/qps | Unmarked qps (texts) | Overflows en (new vs. de) | Overflows qps (new vs. de) | Overflows de (new vs. before phase 6) | German changed | Not producible de/en/qps |\n|---|---|---|---|---|---|---|---|---|\n");
    let mut totals = [0usize; 10];
    let mut details = String::new();
    let mut details_de_new = String::new();
    for (area, title) in AREAS {
        let de = load("de", area);
        let en = load("en", area);
        let qps = load("qps", area);
        let raw = |r: &Option<AreaResult>| r.as_ref().map(|r| uniq(r.findings.iter().filter(|f| f.kind == KIND_RAW_KEY)).to_string()).unwrap_or("–".into());
        let unm: Option<BTreeSet<String>> = qps.as_ref().map(|r| r.findings.iter().filter(|f| f.kind == KIND_UNMARKED).map(|f| f.text.clone()).collect());
        let lay = |r: &Option<AreaResult>| -> Option<(usize, usize)> { r.as_ref().map(|r| (uniq(r.findings.iter().filter(|f| LAYOUT.contains(&f.kind.as_str()))), uniq(new_vs_de(r, de.as_ref()).into_iter()))) };
        let fmt_lay = |x: Option<(usize, usize)>| x.map(|(a, b)| format!("{a} ({b})")).unwrap_or("–".into());
        // German: all overflows, in brackets the ones new against the state before phase 6
        let base = load_in(i18n_qa_common::MODE_BASELINE_LAYOUT, "de", area);
        let de_new = de.as_ref().map(|r| uniq(new_vs_de(r, base.as_ref()).into_iter()));
        let de_lay = de
            .as_ref()
            .map(|r| uniq(r.findings.iter().filter(|f| LAYOUT.contains(&f.kind.as_str()))))
            .map(|n| match (de_new, &base) {
                (Some(x), Some(_)) => format!("{n} ({x})"),
                _ => n.to_string(),
            })
            .unwrap_or("–".into());
        totals[9] += de_new.filter(|_| base.is_some()).unwrap_or(0);
        if let (Some(r), Some(_)) = (&de, &base) {
            let new = new_vs_de(r, base.as_ref());
            if !new.is_empty() {
                let mut rows: BTreeMap<(String, String, String), BTreeSet<String>> = BTreeMap::new();
                for f in &new {
                    rows.entry((f.scene.clone(), f.kind.clone(), f.text.clone())).or_default().insert(f.cfg.clone());
                }
                details_de_new.push_str(&format!("\n**{title}** ({}):\n\n| Image | Kind | Text | Sizes |\n|---|---|---|---|\n", rows.len()));
                for ((scene, kind, text), cfgs) in rows.iter().take(80) {
                    details_de_new.push_str(&format!("| {scene} | {kind} | {} | {} |\n", esc(text), cfgs.iter().cloned().collect::<Vec<_>>().join(", ")));
                }
            }
        }
        let changed = de.as_ref().map(|r| changed_scenes(r).to_string()).unwrap_or("–".into());
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
                totals[1 + i] += uniq(r.findings.iter().filter(|f| f.kind == KIND_RAW_KEY));
            }
        }
        totals[4] += unm.as_ref().map(|s| s.len()).unwrap_or(0);
        totals[5] += lay(&en).map(|x| x.1).unwrap_or(0);
        totals[6] += lay(&qps).map(|x| x.1).unwrap_or(0);
        totals[7] += de.as_ref().map(|r| uniq(r.findings.iter().filter(|f| LAYOUT.contains(&f.kind.as_str())))).unwrap_or(0);
        totals[8] += de.as_ref().map(changed_scenes).unwrap_or(0);

        // details
        details.push_str(&format!("\n### {title}\n\n"));
        let mut any = false;
        for (lang, r) in [("de", &de), ("en", &en), ("qps", &qps)] {
            let Some(r) = r else { continue };
            let raws: Vec<&Finding> = r.findings.iter().filter(|f| f.kind == KIND_RAW_KEY).collect();
            if !raws.is_empty() {
                any = true;
                details.push_str(&format!("**Raw keys {lang}:** "));
                let set: BTreeSet<String> = raws.iter().map(|f| format!("`{}` ({})", esc(&f.detail), f.scene)).collect();
                details.push_str(&set.into_iter().take(40).collect::<Vec<_>>().join(" · "));
                details.push_str("\n\n");
            }
            if !r.failed.is_empty() {
                any = true;
                details.push_str(&format!("**Not producible {lang}:** {}\n\n", r.failed.iter().take(20).map(|s| esc(s)).collect::<Vec<_>>().join(" · ")));
            }
        }
        if let Some(r) = &qps {
            let mut by: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
            for f in r.findings.iter().filter(|f| f.kind == KIND_UNMARKED) {
                by.entry(f.text.clone()).or_default().insert(f.scene.clone());
            }
            if !by.is_empty() {
                any = true;
                details.push_str(&format!("**Unmarked texts qps ({}):**\n\n| Text | Images |\n|---|---|\n", by.len()));
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
            details.push_str(&format!("**Overflows {lang}, new against German ({}):**\n\n| Image | Kind | Text | Sizes | Position |\n|---|---|---|---|---|\n", rows.len()));
            for ((scene, kind, text), (cfgs, det)) in rows.iter().take(if lang == "en" { 300 } else { 120 }) {
                details.push_str(&format!("| {scene} | {kind} | {} | {} | {} |\n", esc(text), cfgs.iter().cloned().collect::<Vec<_>>().join(", "), esc(det)));
            }
            details.push('\n');
        }
        if let Some(r) = &de
            && !r.de_changed.is_empty()
        {
            any = true;
            let n_scenes = r.de_changed.iter().filter_map(|c| c.split_once(": ").map(|x| x.0)).collect::<BTreeSet<_>>().len();
            details.push_str(&format!("**German changed against the state before phase 6 ({n_scenes} images), summed up per line (− before, + now; text, x,y,width,height):**\n\n| Line | Images |\n|---|---|\n"));
            // per single line change: in how many images
            let mut by: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
            for c in &r.de_changed {
                let (scene, diff) = c.split_once(": ").unwrap_or(("?", c));
                by.entry(diff.to_string()).or_default().insert(scene.to_string());
            }
            let mut v: Vec<(String, BTreeSet<String>)> = by.into_iter().collect();
            v.sort_by_key(|a| std::cmp::Reverse(a.1.len()));
            for (diff, scenes) in v.iter().take(40) {
                let sc: Vec<&String> = scenes.iter().take(3).collect();
                details.push_str(&format!("| {} | {} {}{} |\n", esc(diff).replace('`', "'"), scenes.len(), sc.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "), if scenes.len() > 3 { " …" } else { "" }));
            }
            details.push('\n');
        }
        if !any {
            details.push_str("no findings\n");
        }
    }
    md.push_str(&format!(
        "| **Total** | **{}** | **{}/{}/{}** | **{}** | **({})** | **({})** | **{} ({})** | **{}** | |\n\n",
        totals[0], totals[1], totals[2], totals[3], totals[4], totals[5], totals[6], totals[7], totals[9], totals[8]
    ));
    md.push_str("How to read: raw keys and unmarked texts count distinct texts; overflows count distinct (image, text, kind) over all four sizes (1024×768 and 1920×1080, each at 100 % and 150 %). In brackets: only the overflows German does not have in the same image at the same size (caused by the translation). \"Overflows de\": all German overflows; in brackets the ones that did not exist in the state before phase 6 (`30ddaec`, same sizes). The rest existed before and does not depend on the language.\n");
    md.push_str("\n## Details (automatic)\n");
    md.push_str(&details);
    if !details_de_new.is_empty() {
        md.push_str("\n### German: overflows that did not exist before phase 6\n");
        md.push_str(&details_de_new);
    }

    let path = i18n_qa_common::repo().join("docs").join("I18N_PRUEFUNG.md");
    let old = std::fs::read_to_string(&path).unwrap_or_else(|_| "# Translatability check (I18N-QA)\n\n<!-- AUTO:START -->\n<!-- AUTO:END -->\n".into());
    let (a, rest) = old.split_once("<!-- AUTO:START -->").unwrap_or((&old, ""));
    let (_, b) = rest.split_once("<!-- AUTO:END -->").unwrap_or(("", ""));
    let md = neutral(&md);
    let new = format!("{a}<!-- AUTO:START -->\n{md}\n<!-- AUTO:END -->{b}");
    std::fs::write(&path, new).expect("write I18N_PRUEFUNG.md");
    eprintln!("total: {totals:?}");
}

/// Do not write lines that contain the name of the original or its vendor (e.g. system font
/// names) into the docs (AGENTS.md §2, neutrality).
fn neutral(md: &str) -> String {
    md.lines().map(|l| if fp_neutral::find_in_bytes(l.as_bytes()).is_empty() { l.to_string() } else { "| (line removed: contains a blocked name) |".to_string() }).collect::<Vec<_>>().join("\n")
}
/// Number of images with a changed German text list.
fn changed_scenes(r: &AreaResult) -> usize {
    r.de_changed.iter().filter_map(|c| c.split_once(": ").map(|x| x.0.to_string())).collect::<BTreeSet<_>>().len()
}
//! I18N-QA: terminology check of the English texts against the German ones and `GLOSSAR.md`
//! (without user interface, fast). Writes `target-i18n-qa\i18n-qa\current\glossary.json` and the
//! section "English terminology" in `docs\I18N_PRUEFUNG.md` (markers `<!-- EN:START -->`/`<!-- EN:END -->`).
//! `cargo test -j 4 -p fp-ui --test all i18n_qa_glossary:: -- --ignored`
//!
//! Checked per key (de ↔ en):
//! * glossary: if the German text contains a glossary term, the English one must contain its glossary term,
//! * ellipsis: "…"/"..." exactly when the German text has one, and the same spelling everywhere,
//! * access key `&` present in both; placeholders `{ $x }` equal,
//! * capitalisation: menus, commands, dialog titles in Title Case; messages/hints not in Title Case,
//! * German key names (Strg, Umschalt, Entf …) or German words in the English text,
//! * unique access keys per menu (menus from `menu.ftl`, same group = same prefix).

use crate::i18n_qa_common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Simple FTL reader: key (with `.attribute`) → value (raw text, continuation lines joined with ⏎)
/// plus the file.
fn read_ftl(dir: &Path) -> BTreeMap<String, (String, String)> {
    let mut out = BTreeMap::new();
    let Ok(rd) = std::fs::read_dir(dir) else { return out };
    let mut files: Vec<_> = rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "ftl")).collect();
    files.sort();
    for f in files {
        let file = f.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let text = std::fs::read_to_string(&f).unwrap_or_default();
        let mut cur_msg = String::new();
        let mut cur_key: Option<String> = None;
        for line in text.lines() {
            if line.trim_start().starts_with('#') || line.trim().is_empty() {
                continue;
            }
            let indented = line.starts_with(' ') || line.starts_with('\t');
            let t = line.trim();
            if !indented && let Some((k, v)) = t.split_once('=') {
                cur_msg = k.trim().to_string();
                cur_key = Some(cur_msg.clone());
                out.insert(cur_msg.clone(), (v.trim().to_string(), file.clone()));
                continue;
            }
            if indented && t.starts_with('.') && let Some((k, v)) = t.split_once('=') {
                let key = format!("{}{}", cur_msg, k.trim());
                cur_key = Some(key.clone());
                out.insert(key, (v.trim().to_string(), file.clone()));
                continue;
            }
            if let Some(k) = &cur_key
                && let Some(e) = out.get_mut(k)
            {
                if !e.0.is_empty() {
                    e.0.push('⏎');
                }
                e.0.push_str(t);
            }
        }
    }
    out
}

/// Glossary pairs (German, English) from `GLOSSAR.md` ("A = B · C = D").
fn glossary() -> Vec<(String, String)> {
    let p = i18n_qa_common::repo().join("crates").join("fp-i18n").join("GLOSSAR.md");
    let text = std::fs::read_to_string(p).unwrap_or_default();
    let mut out = Vec::new();
    for line in text.lines() {
        if line.starts_with('#') || !line.contains(" = ") {
            continue;
        }
        let line = line.split_once(':').filter(|(a, _)| !a.contains('=')).map(|(_, b)| b).unwrap_or(line);
        for pair in line.split(" · ") {
            let Some((de, en)) = pair.split_once(" = ") else { continue };
            let de = de.trim();
            let en = en.trim();
            // "Verlauf (Werkzeug)" → the addition in brackets is only an explanation
            let de_clean = de.split(" (").next().unwrap_or(de).trim();
            if de_clean.contains('/') || en.contains('/') || de_clean.len() < 4 {
                continue;
            }
            out.push((de_clean.to_string(), en.to_string()));
        }
    }
    out
}

fn has_word(hay: &str, needle: &str) -> bool {
    let h = hay.to_lowercase();
    let n = needle.to_lowercase();
    let mut start = 0;
    while let Some(i) = h[start..].find(&n) {
        let a = start + i;
        let b = a + n.len();
        let before = h[..a].chars().next_back();
        let after = h[b..].chars().next();
        let ok_b = before.is_none_or(|c| !c.is_alphanumeric());
        let ok_a = after.is_none_or(|c| !c.is_alphanumeric());
        if ok_b && ok_a {
            return true;
        }
        start = b;
    }
    false
}

fn placeholders(s: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut rest = s;
    while let Some(i) = rest.find('$') {
        let tail = &rest[i + 1..];
        let name: String = tail.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-').collect();
        out.insert(name);
        rest = tail;
    }
    out
}

fn ellipsis(s: &str) -> Option<&'static str> {
    let t = s.trim_end().trim_end_matches(['"', '}', ' ']);
    if t.ends_with("...") {
        Some("...")
    } else if t.ends_with('…') {
        Some("…")
    } else {
        None
    }
}

const SMALL: &[&str] = &["a", "an", "the", "and", "or", "nor", "but", "to", "in", "on", "of", "for", "as", "at", "by", "with", "from", "into", "per", "via", "vs", "vs."];

/// Words that may be lower case in Title Case, and special forms.
fn title_case_violations(s: &str) -> Vec<String> {
    let clean = s.replace('&', "");
    let clean = clean.trim_end_matches("...").trim_end_matches('…');
    // remove placeholders
    let mut words = Vec::new();
    let mut depth = 0;
    let mut cur = String::new();
    for c in clean.chars() {
        match c {
            '{' => depth += 1,
            '}' => depth -= 1,
            _ if depth > 0 => {}
            ' ' | '\t' | '/' | '(' | ')' | '▸' => {
                if !cur.is_empty() {
                    words.push(std::mem::take(&mut cur));
                }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        words.push(cur);
    }
    let mut bad = Vec::new();
    for (i, w) in words.iter().enumerate() {
        let first = w.chars().next().unwrap_or(' ');
        if !first.is_alphabetic() {
            continue;
        }
        let lw = w.to_lowercase();
        let small = SMALL.contains(&lw.as_str());
        if first.is_lowercase() && !(small && i > 0 && i + 1 < words.len()) && !w.contains('.') && !["px", "pt", "x", "fw", "e.g.", "i.e."].contains(&lw.as_str()) {
            bad.push(w.clone());
        }
    }
    bad
}

/// Does the text look like Title Case (messages should be in sentence case)?
fn looks_title_case(s: &str) -> bool {
    let words: Vec<&str> = s.split_whitespace().filter(|w| w.chars().next().is_some_and(|c| c.is_alphabetic()) && w.len() > 3).collect();
    if words.len() < 4 {
        return false;
    }
    let caps = words.iter().skip(1).filter(|w| w.chars().next().is_some_and(|c| c.is_uppercase()) && !w.chars().all(|c| c.is_uppercase() || !c.is_alphabetic())).count();
    caps * 10 >= (words.len() - 1) * 7
}

const GERMAN_KEYS: &[&str] = &["Strg", "Umschalt", "Entf", "Einfg", "Pos1", "Bild auf", "Bild ab", "Rücktaste", "Leertaste", "Eingabe", "Pfeil"];
const GERMAN_WORDS: &[&str] = &[" und ", " oder ", " der ", " die ", " das ", " nicht ", "Bitte", " mit ", " für ", " ein ", " eine ", " wird ", " auf ", "ä", "ö", "ü", "ß", "Ä", "Ö", "Ü"];

#[derive(serde::Serialize)]
struct Item {
    rule: String,
    key: String,
    file: String,
    de: String,
    en: String,
    note: String,
}

#[test]
#[ignore]
fn english_terminology() {
    let loc = i18n_qa_common::repo().join("crates").join("fp-i18n").join("locales");
    let de = read_ftl(&loc.join("de"));
    let en = read_ftl(&loc.join("en"));
    let gl = glossary();
    let mut items: Vec<Item> = Vec::new();
    let mut push = |rule: &str, key: &str, file: &str, d: &str, e: &str, note: String| items.push(Item { rule: rule.into(), key: key.into(), file: file.into(), de: d.into(), en: e.into(), note });
    let mut ell_styles: BTreeMap<&'static str, usize> = BTreeMap::new();
    for (key, (dv, file)) in &de {
        let Some((ev, _)) = en.get(key) else {
            push("missing", key, file, dv, "", "key missing in en".into());
            continue;
        };
        if ev.contains("->") || dv.contains("->") {
            // select expressions (plurals): only check the placeholders
            if placeholders(dv) != placeholders(ev) {
                push("placeholder", key, file, dv, ev, format!("{:?} ≠ {:?}", placeholders(dv), placeholders(ev)));
            }
            continue;
        }
        // glossary (without access keys; all English variants of a German term count; word stem,
        // so that "Copy" also matches "copies"; descriptions are running text and do not count)
        let dclean = dv.replace('&', "");
        let eclean = ev.replace('&', "").to_lowercase();
        if !key.ends_with(".description") {
            let mut by_de: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
            for (gd, ge) in &gl {
                by_de.entry(gd.as_str()).or_default().push(ge.as_str());
            }
            for (gd, ges) in by_de {
                if !has_word(&dclean, gd) {
                    continue;
                }
                let hit = ges.iter().any(|ge| {
                    let g = ge.to_lowercase();
                    let stem: String = g.trim_end_matches(['y', 'e', 's']).to_string();
                    eclean.contains(&g) || (stem.len() >= 3 && eclean.contains(&stem))
                });
                if !hit {
                    push("glossary", key, file, dv, ev, format!("„{gd}“ → „{}“", ges.join("“/„")));
                }
            }
        }
        // ellipsis
        let (ed, ee) = (ellipsis(dv), ellipsis(ev));
        if let Some(s) = ee {
            *ell_styles.entry(s).or_default() += 1;
        }
        if ed.is_some() != ee.is_some() {
            push("ellipsis", key, file, dv, ev, "ellipsis in one language only".into());
        } else if ed != ee {
            push("ellipsis", key, file, dv, ev, format!("spelling de {ed:?} / en {ee:?}"));
        }
        // access key and placeholders
        let amp = |s: &str| s.replace("&&", "").contains('&');
        if amp(dv) != amp(ev) {
            push("access_key", key, file, dv, ev, "\"&\" in one language only".into());
        }
        if placeholders(dv) != placeholders(ev) {
            push("placeholder", key, file, dv, ev, format!("{:?} ≠ {:?}", placeholders(dv), placeholders(ev)));
        }
        // capitalisation
        let is_attr = key.contains('.');
        let title_like = !is_attr && (key.starts_with("menu-") || key.starts_with("cmd-") || key.starts_with("pmenu-") || key.starts_with("ctx-") || key.ends_with("-title") || key.contains("-title-"));
        if title_like {
            let bad = title_case_violations(ev);
            if !bad.is_empty() && ev.split_whitespace().count() <= 8 {
                push("title_case", key, file, dv, ev, format!("lower case: {}", bad.join(", ")));
            }
        }
        let hint_like = key.starts_with("msg-") || key.starts_with("err-") || key.ends_with(".tooltip") || key.ends_with(".description") || key.contains("-note") || key.contains("-hint");
        if hint_like && looks_title_case(ev) {
            push("sentence_case", key, file, dv, ev, "hint in Title Case".into());
        }
        // German leftovers
        for g in GERMAN_KEYS {
            if has_word(ev, g) {
                push("key_name", key, file, dv, ev, format!("German key name \"{g}\""));
            }
        }
        if GERMAN_WORDS.iter().any(|w| ev.contains(w)) {
            push("german", key, file, dv, ev, "looks untranslated".into());
        }
    }
    for key in en.keys() {
        if !de.contains_key(key) {
            push("orphaned", key, &en[key].1, "", &en[key].0, "only in en".into());
        }
    }
    // access keys unique per menu: entries with the same menu prefix (menu-<menu>-…)
    let mut groups: BTreeMap<String, Vec<(String, char)>> = BTreeMap::new();
    for (key, (ev, _)) in &en {
        if key.contains('.') || !key.starts_with("menu-") {
            continue;
        }
        let v = ev.replace("&&", "");
        if let Some(i) = v.find('&')
            && let Some(c) = v[i + 1..].chars().next()
        {
            let parent = key.rsplit_once('-').map(|(a, _)| a.to_string()).unwrap_or_default();
            groups.entry(parent).or_default().push((key.clone(), c.to_ascii_lowercase()));
        }
    }
    for (g, v) in &groups {
        let mut seen: BTreeMap<char, Vec<String>> = BTreeMap::new();
        for (k, c) in v {
            seen.entry(*c).or_default().push(k.clone());
        }
        for (c, ks) in seen {
            if ks.len() > 1 {
                items.push(Item { rule: "access_key_duplicate".into(), key: ks.join(", "), file: "menu.ftl".into(), de: String::new(), en: String::new(), note: format!("group {g}: \"{c}\" several times") });
            }
        }
    }
    // result
    let out = i18n_qa_common::out_root().join(i18n_qa_common::MODE_CURRENT);
    let _ = std::fs::create_dir_all(&out);
    let _ = std::fs::write(out.join("glossary.json"), serde_json::to_string_pretty(&items).unwrap_or_default());
    let mut by: BTreeMap<String, Vec<&Item>> = BTreeMap::new();
    for i in &items {
        by.entry(i.rule.clone()).or_default().push(i);
    }
    let mut md = format!("Keys de: {}, en: {}. Ellipses in en: {:?}.\n\n| Rule | Count |\n|---|---|\n", de.len(), en.len(), ell_styles);
    for (r, v) in &by {
        md.push_str(&format!("| {r} | {} |\n", v.len()));
    }
    md.push('\n');
    for (r, v) in &by {
        md.push_str(&format!("\n#### {r} ({})\n\n| Key | File | de | en | Note |\n|---|---|---|---|---|\n", v.len()));
        for i in v.iter().take(200) {
            let e = |s: &str| s.replace('|', "/").chars().take(120).collect::<String>();
            md.push_str(&format!("| `{}` | {} | {} | {} | {} |\n", i.key, i.file, e(&i.de), e(&i.en), e(&i.note)));
        }
    }
    let md: String = md.lines().map(|l| if fp_neutral::find_in_bytes(l.as_bytes()).is_empty() { l.to_string() } else { "| (line removed: blocked name) |".to_string() }).collect::<Vec<_>>().join("\n");
    let path = i18n_qa_common::repo().join("docs").join("I18N_PRUEFUNG.md");
    let old = std::fs::read_to_string(&path).unwrap_or_else(|_| "# Translatability check (I18N-QA)\n\n<!-- EN:START -->\n<!-- EN:END -->\n".into());
    let new = match old.split_once("<!-- EN:START -->") {
        Some((a, rest)) => {
            let b = rest.split_once("<!-- EN:END -->").map(|x| x.1).unwrap_or("");
            format!("{a}<!-- EN:START -->\n{md}\n<!-- EN:END -->{b}")
        }
        None => format!("{old}\n<!-- EN:START -->\n{md}\n<!-- EN:END -->\n"),
    };
    std::fs::write(&path, new).expect("write I18N_PRUEFUNG.md");
    eprintln!("English terminology: {} notes {:?}", items.len(), by.iter().map(|(k, v)| (k.clone(), v.len())).collect::<Vec<_>>());
}

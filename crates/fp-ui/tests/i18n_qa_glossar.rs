//! I18N-QA: fachliche Prüfung der englischen Texte gegen die deutschen und `GLOSSAR.md`
//! (ohne Oberfläche, schnell). Schreibt `target-i18n-qa\i18n-qa\aktuell\glossar.json` und den
//! Abschnitt „Englisch fachlich“ in `docs\I18N_PRUEFUNG.md` (Markierungen `<!-- EN:START -->`/`<!-- EN:END -->`).
//! `cargo test -j 4 -p fp-ui --test i18n_qa_glossar -- --ignored`
//!
//! Geprüft je Schlüssel (de ↔ en):
//! * Glossar: enthält der deutsche Text einen Glossarbegriff, muss der englische den Glossarbegriff enthalten,
//! * Ellipse: „…“/„...“ genau dann, wenn der deutsche Text eine hat, und überall dieselbe Schreibweise,
//! * Zugriffstaste `&` gleich vorhanden; Platzhalter `{ $x }` gleich,
//! * Großschreibung: Menüs, Befehle, Dialogtitel in Title Case; Meldungen/Hinweise nicht in Title Case,
//! * deutsche Tastennamen (Strg, Umschalt, Entf …) oder deutsche Wörter im englischen Text,
//! * eindeutige Zugriffstasten je Menü (Menüs aus `menue.ftl`, gleiche Gruppe = gleiches Präfix).

use crate::i18n_qa_common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Einfacher FTL-Leser: Schlüssel (mit `.attribut`) → Wert (Rohtext, Folgezeilen mit ⏎ verbunden),
/// dazu die Datei.
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

/// Glossarpaare (Deutsch, Englisch) aus `GLOSSAR.md` („A = B · C = D“).
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
            // „Verlauf (Werkzeug)“ → Zusatz in Klammern ist nur Erklärung
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
        rest = &tail[..];
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

/// Wörter, die in Title Case kleingeschrieben sein dürfen bzw. Sonderformen.
fn title_case_violations(s: &str) -> Vec<String> {
    let clean = s.replace('&', "");
    let clean = clean.trim_end_matches("...").trim_end_matches('…');
    // Platzhalter entfernen
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

/// Sieht der Text wie Title Case aus (Meldungen sollen Sentence case haben)?
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
fn englisch_fachlich() {
    let loc = i18n_qa_common::repo().join("crates").join("fp-i18n").join("locales");
    let de = read_ftl(&loc.join("de"));
    let en = read_ftl(&loc.join("en"));
    let gl = glossary();
    let mut items: Vec<Item> = Vec::new();
    let mut push = |rule: &str, key: &str, file: &str, d: &str, e: &str, note: String| items.push(Item { rule: rule.into(), key: key.into(), file: file.into(), de: d.into(), en: e.into(), note });
    let mut ell_styles: BTreeMap<&'static str, usize> = BTreeMap::new();
    for (key, (dv, file)) in &de {
        let Some((ev, _)) = en.get(key) else {
            push("fehlt", key, file, dv, "", "Schlüssel fehlt in en".into());
            continue;
        };
        if ev.contains("->") || dv.contains("->") {
            // Auswahlausdrücke (Mehrzahl): nur Platzhalter prüfen
            if placeholders(dv) != placeholders(ev) {
                push("platzhalter", key, file, dv, ev, format!("{:?} ≠ {:?}", placeholders(dv), placeholders(ev)));
            }
            continue;
        }
        // Glossar (ohne Zugriffstasten; alle englischen Varianten eines deutschen Begriffs gelten;
        // Wortstamm, damit „Copy“ auch „copies“ trifft; Beschreibungen sind Fließtext und zählen nicht)
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
                    push("glossar", key, file, dv, ev, format!("„{gd}“ → „{}“", ges.join("“/„")));
                }
            }
        }
        // Ellipse
        let (ed, ee) = (ellipsis(dv), ellipsis(ev));
        if let Some(s) = ee {
            *ell_styles.entry(s).or_default() += 1;
        }
        if ed.is_some() != ee.is_some() {
            push("ellipse", key, file, dv, ev, "Ellipse nur in einer Sprache".into());
        } else if ed != ee {
            push("ellipse", key, file, dv, ev, format!("Schreibweise de {ed:?} / en {ee:?}"));
        }
        // Zugriffstaste und Platzhalter
        let amp = |s: &str| s.replace("&&", "").contains('&');
        if amp(dv) != amp(ev) {
            push("zugriffstaste", key, file, dv, ev, "„&“ nur in einer Sprache".into());
        }
        if placeholders(dv) != placeholders(ev) {
            push("platzhalter", key, file, dv, ev, format!("{:?} ≠ {:?}", placeholders(dv), placeholders(ev)));
        }
        // Großschreibung
        let is_attr = key.contains('.');
        let title_like = !is_attr && (key.starts_with("menu-") || key.starts_with("cmd-") || key.starts_with("pmenu-") || key.starts_with("ctx-") || key.ends_with("-title") || key.contains("-title-"));
        if title_like {
            let bad = title_case_violations(ev);
            if !bad.is_empty() && ev.split_whitespace().count() <= 8 {
                push("title_case", key, file, dv, ev, format!("klein: {}", bad.join(", ")));
            }
        }
        let hint_like = key.starts_with("msg-") || key.starts_with("err-") || key.ends_with(".tooltip") || key.ends_with(".description") || key.contains("-note") || key.contains("-hint");
        if hint_like && looks_title_case(ev) {
            push("sentence_case", key, file, dv, ev, "Hinweis in Title Case".into());
        }
        // Deutsche Reste
        for g in GERMAN_KEYS {
            if has_word(ev, g) {
                push("tastenname", key, file, dv, ev, format!("deutscher Tastenname „{g}“"));
            }
        }
        if GERMAN_WORDS.iter().any(|w| ev.contains(w)) {
            push("deutsch", key, file, dv, ev, "wirkt unübersetzt".into());
        }
    }
    for key in en.keys() {
        if !de.contains_key(key) {
            push("verwaist", key, &en[key].1, "", &en[key].0, "nur in en".into());
        }
    }
    // Zugriffstasten je Menü eindeutig: Einträge mit gleichem Menü-Präfix (menu-<menü>-…)
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
                items.push(Item { rule: "zugriffstaste_doppelt".into(), key: ks.join(", "), file: "menu.ftl".into(), de: String::new(), en: String::new(), note: format!("Gruppe {g}: „{c}“ mehrfach") });
            }
        }
    }
    // Ergebnis
    let out = i18n_qa_common::out_root().join("aktuell");
    let _ = std::fs::create_dir_all(&out);
    let _ = std::fs::write(out.join("glossar.json"), serde_json::to_string_pretty(&items).unwrap_or_default());
    let mut by: BTreeMap<String, Vec<&Item>> = BTreeMap::new();
    for i in &items {
        by.entry(i.rule.clone()).or_default().push(i);
    }
    let mut md = format!("Schlüssel de: {}, en: {}. Ellipsen in en: {:?}.\n\n| Regel | Anzahl |\n|---|---|\n", de.len(), en.len(), ell_styles);
    for (r, v) in &by {
        md.push_str(&format!("| {r} | {} |\n", v.len()));
    }
    md.push('\n');
    for (r, v) in &by {
        md.push_str(&format!("\n#### {r} ({})\n\n| Schlüssel | Datei | de | en | Hinweis |\n|---|---|---|---|---|\n", v.len()));
        for i in v.iter().take(200) {
            let e = |s: &str| s.replace('|', "/").chars().take(120).collect::<String>();
            md.push_str(&format!("| `{}` | {} | {} | {} | {} |\n", i.key, i.file, e(&i.de), e(&i.en), e(&i.note)));
        }
    }
    let md: String = md.lines().map(|l| if fp_neutral::find_in_bytes(l.as_bytes()).is_empty() { l.to_string() } else { "| (Zeile entfernt: gesperrter Name, §1a) |".to_string() }).collect::<Vec<_>>().join("\n");
    let path = i18n_qa_common::repo().join("docs").join("I18N_PRUEFUNG.md");
    let old = std::fs::read_to_string(&path).unwrap_or_else(|_| "# Prüfung der Mehrsprachigkeit (I18N-QA)\n\n<!-- EN:START -->\n<!-- EN:END -->\n".into());
    let new = match old.split_once("<!-- EN:START -->") {
        Some((a, rest)) => {
            let b = rest.split_once("<!-- EN:END -->").map(|x| x.1).unwrap_or("");
            format!("{a}<!-- EN:START -->\n{md}\n<!-- EN:END -->{b}")
        }
        None => format!("{old}\n<!-- EN:START -->\n{md}\n<!-- EN:END -->\n"),
    };
    std::fs::write(&path, new).expect("schreiben");
    eprintln!("Englisch fachlich: {} Hinweise {:?}", items.len(), by.iter().map(|(k, v)| (k.clone(), v.len())).collect::<Vec<_>>());
}

//! Text ▸ Rechtschreibprüfung / Rechtschreibprüfung einrichten.
//!
//! Geprüft wird mit Hunspell-Wörterbüchern (`<sprache>.aff` + `<sprache>.dic`, z. B. `de_DE`), die zur
//! Laufzeit gesucht werden: eingestellter Ordner, `FP_DICT_DIR`, `Dictionaries` neben der
//! Voreinstellungsdatei, `%APPDATA%\Microsoft\Spelling\<sprache>` und übliche LibreOffice-/OpenOffice-
//! Ordner. Eingebettet ist kein Wörterbuch. Das persönliche Wörterbuch (Voreinstellung
//! `SpellingUserWords`) gilt immer.

use super::{selected_elements, with_doc};
use crate::command::{CommandOutput, CommandRegistry, ParamKind, Params, cmd};
use crate::docops;
use crate::error::{EngineError, Result};
use crate::prefs::PrefValue;
use crate::session::Session;
use fp_model::element::ElementKind;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

fn pref_str(s: &Session, k: &str) -> Option<String> {
    match s.prefs.get(k) {
        Some(PrefValue::Str(v)) => Some(v.clone()),
        _ => None,
    }
}

fn pref_bool(s: &Session, k: &str, d: bool) -> bool {
    match s.prefs.get(k) {
        Some(PrefValue::Bool(b)) => b,
        Some(PrefValue::Int(i)) => i != 0,
        _ => d,
    }
}

/// Eingestellte Sprache (erstes aktives Wörterbuch, Vorgabe `de_DE`).
pub fn language(s: &Session) -> String {
    languages(s)
        .into_iter()
        .next()
        .unwrap_or_else(|| "de_DE".into())
}

/// Aktive Wörterbücher (Kästchen in „Rechtschreibprüfung einrichten“). Ohne eigene Liste die
/// einzelne Sprache `SpellingLanguage` (Vorgabe `de_DE`); eine leere Liste heißt: keins.
pub fn languages(s: &Session) -> Vec<String> {
    if let Some(l) = pref_str(s, "SpellingLanguages") {
        return l
            .lines()
            .map(str::trim)
            .filter(|x| !x.is_empty())
            .map(str::to_string)
            .collect();
    }
    vec![
        pref_str(s, "SpellingLanguage")
            .filter(|l| !l.is_empty())
            .unwrap_or_else(|| "de_DE".into()),
    ]
}

/// Datei des persönlichen Wörterbuchs (leer = in den Voreinstellungen).
pub fn personal_dictionary(s: &Session) -> Option<PathBuf> {
    pref_str(s, "SpellingPersonalDictionary")
        .filter(|p| !p.trim().is_empty())
        .map(PathBuf::from)
}

/// Vorgeschlagener Ort des persönlichen Wörterbuchs im Firepixel-Profil
/// (`%APPDATA%\Firepixel\Dictionaries\Personal.dic`, neben den Wörterbuch-Ordnern der Suche).
/// Der Dialog zeigt ihn vorbelegt wie das Original (TW-13); erst OK legt die Datei an.
pub fn default_personal_dictionary() -> Option<PathBuf> {
    crate::prefs::Preferences::default_path().and_then(|p| {
        p.parent()
            .map(|d| d.join("Dictionaries").join("Personal.dic"))
    })
}

/// Persönliches Wörterbuch (aus der eingestellten Datei, sonst aus den Voreinstellungen).
pub fn user_words(s: &Session) -> Vec<String> {
    let text = match personal_dictionary(s) {
        Some(p) => std::fs::read_to_string(p).ok(),
        None => pref_str(s, "SpellingUserWords"),
    };
    text.map(|t| {
        t.lines()
            .map(str::trim)
            .filter(|w| !w.is_empty())
            .map(str::to_string)
            .collect()
    })
    .unwrap_or_default()
}

/// Alle gefundenen Wörterbücher (`<sprache>.aff` + `.dic` in den Suchordnern), sortiert.
pub fn available_dictionaries(s: &Session) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for d in search_dirs(s, &language(s)) {
        let Ok(it) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in it.flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("dic"))
                && p.with_extension("aff").is_file()
                && let Some(stem) = p.file_stem().map(|x| x.to_string_lossy().to_string())
                && !v.contains(&stem)
            {
                v.push(stem);
            }
        }
    }
    v.sort();
    v
}

fn search_dirs(s: &Session, lang: &str) -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(d) = pref_str(s, "SpellingDictionaryDir").filter(|d| !d.is_empty()) {
        v.push(PathBuf::from(d));
    }
    if let Some(d) = std::env::var_os("FP_DICT_DIR") {
        v.push(PathBuf::from(d));
    }
    if let Some(p) =
        crate::prefs::Preferences::default_path().and_then(|p| p.parent().map(Path::to_path_buf))
    {
        v.push(p.join("Dictionaries"));
    }
    if let Some(a) = std::env::var_os("APPDATA") {
        let a = PathBuf::from(a);
        v.push(
            a.join("Microsoft")
                .join("Spelling")
                .join(lang.replace('_', "-")),
        );
        v.push(a.join("Microsoft").join("Spelling").join(lang));
    }
    for pf in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(p) = std::env::var_os(pf) {
            let p = PathBuf::from(p);
            for office in ["LibreOffice", "OpenOffice 4"] {
                let ext = p.join(office).join("share").join("extensions");
                if let Ok(it) = std::fs::read_dir(&ext) {
                    for e in it.flatten() {
                        v.push(e.path());
                    }
                }
            }
        }
    }
    v
}

/// Findet `<lang>.aff`/`<lang>.dic`.
pub fn find_dictionary(s: &Session, lang: &str) -> Option<(PathBuf, PathBuf)> {
    for d in search_dirs(s, lang) {
        let (a, b) = (d.join(format!("{lang}.aff")), d.join(format!("{lang}.dic")));
        if a.is_file() && b.is_file() {
            return Some((a, b));
        }
    }
    None
}

type Cache = Mutex<HashMap<PathBuf, Arc<spellbook::Dictionary>>>;

fn load(aff: &Path, dic: &Path) -> Result<Arc<spellbook::Dictionary>> {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let c = CACHE.get_or_init(Default::default);
    if let Ok(m) = c.lock()
        && let Some(d) = m.get(dic)
    {
        return Ok(Arc::clone(d));
    }
    let read = |p: &Path| -> Result<String> {
        let b = std::fs::read(p)?;
        Ok(match String::from_utf8(b) {
            Ok(t) => t,
            // ältere Wörterbücher in ISO-8859-1
            Err(e) => e.into_bytes().iter().map(|&x| x as char).collect(),
        })
    };
    let d = spellbook::Dictionary::new(&read(aff)?, &read(dic)?).map_err(|e| {
        EngineError::Failed(fp_i18n::tr!(
            "err-dictionary",
            path = dic.display().to_string(),
            detail = e.to_string()
        ))
    })?;
    let d = Arc::new(d);
    if let Ok(mut m) = c.lock() {
        m.insert(dic.to_path_buf(), Arc::clone(&d));
    }
    Ok(d)
}

/// Wörter eines Textes mit Zeichenversatz.
pub fn words(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut start = 0;
    let chars: Vec<char> = text.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        let inner = (c == '\'' || c == '’' || c == '-')
            && !cur.is_empty()
            && chars.get(i + 1).is_some_and(|n| n.is_alphanumeric());
        if c.is_alphanumeric() || inner {
            if cur.is_empty() {
                start = i;
            }
            cur.push(c);
        } else if !cur.is_empty() {
            out.push((start, std::mem::take(&mut cur)));
        }
    }
    if !cur.is_empty() {
        out.push((start, cur));
    }
    out
}

struct Checker {
    dicts: Vec<Arc<spellbook::Dictionary>>,
    user: Vec<String>,
    ignore_upper: bool,
    ignore_numbers: bool,
    ignore_urls: bool,
    find_duplicates: bool,
}

/// Zeichenbereiche (Start, Ende) von Internet- und Dateiadressen: Stücke ohne Leerraum mit `://`,
/// `www.`, `@` und Punkt, Laufwerksbuchstabe, `\\` am Anfang oder `/` bzw. `\` zusammen mit einem Punkt.
pub fn address_ranges(text: &str) -> Vec<(usize, usize)> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && !chars[i].is_whitespace() {
            i += 1;
        }
        let tok: String = chars[start..i].iter().collect::<String>().to_lowercase();
        let b = tok.as_bytes();
        let drive = b.len() > 2
            && b[0].is_ascii_alphabetic()
            && b[1] == b':'
            && (b[2] == b'\\' || b[2] == b'/');
        let is_addr = tok.contains("://")
            || tok.starts_with("www.")
            || (tok.contains('@') && tok.contains('.'))
            || drive
            || tok.starts_with("\\\\")
            || ((tok.contains('/') || tok.contains('\\')) && tok.contains('.'));
        if is_addr {
            out.push((start, i));
        }
    }
    out
}

impl Checker {
    fn ok(&self, w: &str) -> bool {
        if self.user.iter().any(|u| u == w) {
            return true;
        }
        if self.ignore_numbers && w.chars().any(|c| c.is_ascii_digit()) {
            return true;
        }
        if self.ignore_upper && w.chars().count() > 1 && w.chars().all(|c| !c.is_lowercase()) {
            return true;
        }
        if self.dicts.is_empty() {
            return true;
        }
        self.dicts.iter().any(|d| d.check(w))
    }
    fn suggest(&self, w: &str) -> Vec<String> {
        let mut v: Vec<String> = Vec::new();
        for d in &self.dicts {
            let mut s = Vec::new();
            d.suggest(w, &mut s);
            for x in s {
                if !v.contains(&x) {
                    v.push(x);
                }
            }
        }
        v.truncate(8);
        v
    }

    /// Gemeldete Wörter eines Textes: (Versatz, Wort, doppelt?).
    fn issues(&self, text: &str) -> Vec<(usize, String, bool)> {
        let skip = if self.ignore_urls {
            address_ranges(text)
        } else {
            Vec::new()
        };
        let chars: Vec<char> = text.chars().collect();
        let mut out = Vec::new();
        let mut prev: Option<(usize, String)> = None;
        for (off, w) in words(text) {
            if skip.iter().any(|(a, b)| off >= *a && off < *b) {
                prev = None;
                continue;
            }
            // doppeltes Wort: dasselbe Wort direkt davor, nur Leerraum dazwischen
            let dup = self.find_duplicates
                && prev.as_ref().is_some_and(|(po, pw)| {
                    pw.to_lowercase() == w.to_lowercase()
                        && chars[po + pw.chars().count()..off]
                            .iter()
                            .all(|c| c.is_whitespace())
                });
            if dup {
                out.push((off, w.clone(), true));
            } else if !self.ok(&w) {
                out.push((off, w.clone(), false));
            }
            prev = Some((off, w));
        }
        out
    }
}

fn checker(s: &Session) -> Result<Checker> {
    let mut dicts = Vec::new();
    for lang in languages(s) {
        if let Some((a, d)) = find_dictionary(s, &lang) {
            dicts.push(load(&a, &d)?);
        }
    }
    Ok(Checker {
        dicts,
        user: user_words(s),
        ignore_upper: pref_bool(s, "SpellingIgnoreUppercase", true),
        ignore_numbers: pref_bool(s, "SpellingIgnoreNumbers", true),
        ignore_urls: pref_bool(s, "SpellingIgnoreUrls", true),
        find_duplicates: pref_bool(s, "SpellingFindDuplicates", true),
    })
}

/// Fasst ein doppeltes Wort zusammen („der der“ → „der“, mit `to` ≠ Wort → „der <to>“).
pub fn collapse_duplicate(text: &str, word: &str, to: &str) -> (String, usize) {
    let chars: Vec<char> = text.chars().collect();
    let ws = words(text);
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    let mut n = 0;
    for k in 1..ws.len() {
        let (po, pw) = &ws[k - 1];
        let (off, w) = &ws[k];
        let pend = po + pw.chars().count();
        if w == word
            && pw.to_lowercase() == w.to_lowercase()
            && pend >= last
            && chars[pend..*off].iter().all(|c| c.is_whitespace())
        {
            if to.is_empty() || to == word {
                out.extend(&chars[last..pend]);
            } else {
                out.extend(&chars[last..*off]);
                out.push_str(to);
            }
            last = off + w.chars().count();
            n += 1;
        }
    }
    out.extend(&chars[last..]);
    (out, n)
}

/// Ersetzt ganze Wörter `from` durch `to` in einem Text.
pub fn replace_word(text: &str, from: &str, to: &str) -> (String, usize) {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut n = 0;
    let mut last = 0;
    for (start, w) in words(text) {
        if w == from {
            out.extend(&chars[last..start]);
            out.push_str(to);
            last = start + w.chars().count();
            n += 1;
        }
    }
    out.extend(&chars[last..]);
    (out, n)
}

fn setup_json(s: &Session) -> Value {
    let lang = language(s);
    let dict = find_dictionary(s, &lang);
    json!({
        "language": lang,
        "dictionary": dict.as_ref().map(|(_, d)| d.to_string_lossy().to_string()),
        "languages": languages(s),
        "available": available_dictionaries(s),
        "dictionaryDir": pref_str(s, "SpellingDictionaryDir"),
        "personalDictionary": personal_dictionary(s).or_else(default_personal_dictionary).map(|p| p.to_string_lossy().to_string()),
        "userWords": user_words(s),
        "ignoreUppercase": pref_bool(s, "SpellingIgnoreUppercase", true),
        "ignoreNumbers": pref_bool(s, "SpellingIgnoreNumbers", true),
        "ignoreUrls": pref_bool(s, "SpellingIgnoreUrls", true),
        "findDuplicates": pref_bool(s, "SpellingFindDuplicates", true),
    })
}

fn set_user_words(s: &mut Session, w: Vec<String>) {
    match personal_dictionary(s) {
        Some(p) => {
            if let Some(dir) = p.parent().filter(|d| !d.as_os_str().is_empty()) {
                let _ = std::fs::create_dir_all(dir);
            }
            let mut text = w.join("\n");
            if !text.is_empty() {
                text.push('\n');
            }
            let _ = std::fs::write(&p, text);
        }
        None => s
            .prefs
            .set("SpellingUserWords", PrefValue::Str(w.join("\n"))),
    }
}

fn texts_of(s: &Session, p: &Params) -> Result<Vec<(fp_model::ElementId, String)>> {
    let d = s.doc()?;
    let ids = p.opt_ids("ids")?;
    let els: Vec<_> = match &ids {
        Some(v) if !v.is_empty() => v
            .iter()
            .filter_map(|id| docops::element_on_page(&d.doc, d.page, *id))
            .collect(),
        _ if !d.selection.elements.is_empty() => {
            selected_elements(d).into_iter().map(|(_, e)| e).collect()
        }
        // ohne Auswahl: alle Texte der Seite (wie das Original)
        _ => {
            let mut v = Vec::new();
            for id in docops::selectable_ids(&d.doc, d.page, d.frame) {
                if let Some(e) = docops::element_on_page(&d.doc, d.page, id) {
                    v.push(e);
                }
            }
            v
        }
    };
    Ok(els
        .into_iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Text(t) => Some((e.id, t.plain_text())),
            _ => None,
        })
        .collect())
}

pub(super) fn register(r: &mut CommandRegistry) {
    r.add(
        cmd("text.spellingSetup")
            .opt(
                "language",
                ParamKind::String,
                "Sprache des Wörterbuchs (z. B. de_DE, en_US)",
            )
            .opt(
                "dictionaryDir",
                ParamKind::Path,
                "Ordner mit <sprache>.aff/.dic",
            )
            .opt(
                "addWords",
                ParamKind::Json,
                "Wörter ins persönliche Wörterbuch",
            )
            .opt(
                "removeWords",
                ParamKind::Json,
                "Wörter aus dem persönlichen Wörterbuch entfernen",
            )
            .opt(
                "ignoreUppercase",
                ParamKind::Bool,
                "Wörter in Großbuchstaben überspringen",
            )
            .opt(
                "ignoreNumbers",
                ParamKind::Bool,
                "Wörter mit Ziffern überspringen",
            )
            .opt(
                "languages",
                ParamKind::Json,
                "Aktive Wörterbücher (Liste der Sprachen)",
            )
            .opt(
                "personalDictionary",
                ParamKind::Path,
                "Datei des persönlichen Wörterbuchs (leer: in den Voreinstellungen)",
            )
            .opt(
                "ignoreUrls",
                ParamKind::Bool,
                "Internet- und Dateiadressen überspringen",
            )
            .opt("findDuplicates", ParamKind::Bool, "Doppelte Wörter melden")
            .run(|s, p| {
                let mut changed = false;
                if let Some(l) = p.opt_str("language")? {
                    s.prefs.set("SpellingLanguage", PrefValue::Str(l.clone()));
                    if p.get("languages").is_none() {
                        s.prefs.set("SpellingLanguages", PrefValue::Str(l));
                    }
                    changed = true;
                }
                if let Some(list) = p.parse::<Vec<String>>("languages")? {
                    if let Some(first) = list.first() {
                        s.prefs
                            .set("SpellingLanguage", PrefValue::Str(first.clone()));
                    }
                    s.prefs
                        .set("SpellingLanguages", PrefValue::Str(list.join("\n")));
                    changed = true;
                }
                if let Some(d) = p.opt_str("personalDictionary")? {
                    // Wortliste beim Wechsel mitnehmen, wenn die neue Datei noch nicht existiert
                    let words = user_words(s);
                    s.prefs
                        .set("SpellingPersonalDictionary", PrefValue::Str(d.clone()));
                    if !d.trim().is_empty() && !Path::new(&d).exists() {
                        set_user_words(s, words);
                    }
                    changed = true;
                }
                if let Some(d) = p.opt_str("dictionaryDir")? {
                    s.prefs.set("SpellingDictionaryDir", PrefValue::Str(d));
                    changed = true;
                }
                for (k, key) in [
                    ("ignoreUppercase", "SpellingIgnoreUppercase"),
                    ("ignoreNumbers", "SpellingIgnoreNumbers"),
                    ("ignoreUrls", "SpellingIgnoreUrls"),
                    ("findDuplicates", "SpellingFindDuplicates"),
                ] {
                    if let Some(b) = p.opt_bool(k)? {
                        s.prefs.set(key, PrefValue::Bool(b));
                        changed = true;
                    }
                }
                let add: Vec<String> = p.parse("addWords")?.unwrap_or_default();
                let rem: Vec<String> = p.parse("removeWords")?.unwrap_or_default();
                if !add.is_empty() || !rem.is_empty() {
                    let mut w = user_words(s);
                    for a in add {
                        if !w.contains(&a) {
                            w.push(a);
                        }
                    }
                    w.retain(|x| !rem.contains(x));
                    set_user_words(s, w);
                    changed = true;
                }
                if changed {
                    s.prefs_changed();
                }
                Ok(CommandOutput::session_with(setup_json(s)))
            }),
    );
    r.add(
        cmd("text.checkSpelling")
            .requires(super::DOC)
            .opt("ids", ParamKind::Ids, "Textobjekte (Vorgabe: Auswahl, sonst alle Texte der Seite)")
            .opt("replace", ParamKind::Json, "Ersetzungen [{word, with, id?}] (ganze Wörter; ohne id in allen geprüften Texten)")
            .opt("addToDictionary", ParamKind::Json, "Wörter ins persönliche Wörterbuch aufnehmen")
            .run(|s, p| {
                let add: Vec<String> = p.parse("addToDictionary")?.unwrap_or_default();
                if !add.is_empty() {
                    let mut w = user_words(s);
                    for a in add {
                        if !w.contains(&a) {
                            w.push(a);
                        }
                    }
                    set_user_words(s, w);
                    s.prefs_changed();
                }
                let texts = texts_of(s, p)?;
                let reps: Vec<Value> = match p.get("replace") {
                    Some(Value::Array(a)) => a.clone(),
                    Some(Value::Object(_)) => vec![p.get("replace").cloned().unwrap_or_default()],
                    Some(_) => return Err(EngineError::InvalidParams { name: p.command.clone(), message: fp_i18n::t("err-param-replace") }),
                    None => Vec::new(),
                };
                let mut replaced = 0usize;
                if !reps.is_empty() {
                    let target_ids: Vec<_> = texts.iter().map(|(id, _)| *id).collect();
                    with_doc(s, |d| {
                        let page = d.page;
                        let unchanged = Arc::clone(&d.doc);
                        let count_before = replaced;
                        let doc = Arc::make_mut(&mut d.doc);
                        for rep in &reps {
                            let (Some(from), Some(to)) = (rep.get("word").and_then(Value::as_str), rep.get("with").and_then(Value::as_str)) else { continue };
                            let only = rep.get("id").and_then(Value::as_u64).map(fp_model::ElementId);
                            let dup = rep.get("duplicate").and_then(Value::as_bool).unwrap_or(false);
                            for id in target_ids.iter().filter(|i| only.is_none_or(|o| o == **i)) {
                                docops::edit_by_id(doc, page, *id, |e| {
                                    if let ElementKind::Text(t) = &mut e.kind {
                                        let mut n = 0;
                                        for run in t.runs.iter_mut() {
                                            let (nt, k) = if dup { collapse_duplicate(&run.text, from, to) } else { replace_word(&run.text, from, to) };
                                            if k > 0 {
                                                run.text = nt;
                                                n += k;
                                            }
                                        }
                                        if n > 0 {
                                            t.detach_story();
                                            replaced += n;
                                        }
                                    }
                                });
                            }
                        }
                        // nichts ersetzt: Dokument unverändert lassen (kein stiller Schnappschuss)
                        if replaced == count_before {
                            d.doc = unchanged;
                        }
                        Ok(())
                    })?;
                }
                let ck = checker(s)?;
                let texts = texts_of(s, p)?;
                let mut issues = Vec::new();
                for (id, text) in &texts {
                    for (off, w, dup) in ck.issues(text) {
                        if dup {
                            issues.push(json!({"id": id.0, "word": w, "offset": off, "suggestions": [], "duplicate": true}));
                        } else {
                            issues.push(json!({"id": id.0, "word": w, "offset": off, "suggestions": ck.suggest(&w)}));
                        }
                    }
                }
                let value = json!({
                    "language": language(s), "languages": languages(s), "dictionary": !ck.dicts.is_empty(), "checked": texts.len(),
                    "misspelled": issues, "replaced": replaced,
                    "note": if ck.dicts.is_empty() { Some(fp_i18n::t("msg-spelling-no-dictionary")) } else { None },
                });
                if replaced > 0 {
                    return Ok(CommandOutput::doc_with(value).labeled(fp_i18n::t("msg-step-spelling")));
                }
                Ok(CommandOutput::value(value))
            }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_split_and_replace() {
        let w: Vec<String> = words("Hallo Wlet, it's e-mail 42!")
            .into_iter()
            .map(|x| x.1)
            .collect();
        assert_eq!(w, vec!["Hallo", "Wlet", "it's", "e-mail", "42"]);
        assert_eq!(
            replace_word("Wlet und Wlet, Wletter", "Wlet", "Welt"),
            ("Welt und Welt, Wletter".to_string(), 2)
        );
        assert_eq!(
            collapse_duplicate("das ist der der Baum", "der", ""),
            ("das ist der Baum".to_string(), 1)
        );
        assert_eq!(
            address_ranges("siehe www.beispiel.de und C:\\a\\b.txt"),
            vec![(6, 21), (26, 36)]
        );
    }
}

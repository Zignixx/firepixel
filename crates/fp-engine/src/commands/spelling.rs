//! Text ▸ Rechtschreibprüfung / Rechtschreibprüfung einrichten.
//!
//! Geprüft wird mit Hunspell-Wörterbüchern (`<sprache>.aff` + `<sprache>.dic`, z. B. `de_DE`), die zur
//! Laufzeit gesucht werden: eingestellter Ordner, `FP_DICT_DIR`, `Dictionaries` neben der
//! Voreinstellungsdatei, `%APPDATA%\Microsoft\Spelling\<sprache>` und übliche LibreOffice-/OpenOffice-
//! Ordner. Eingebettet ist kein Wörterbuch. Das persönliche Wörterbuch (Voreinstellung
//! `SpellingUserWords`) gilt immer.

use super::{selected_elements, with_doc};
use crate::command::{cmd, CommandOutput, CommandRegistry, ParamKind, Params};
use crate::docops;
use crate::error::{EngineError, Result};
use crate::prefs::PrefValue;
use crate::session::Session;
use fp_model::element::ElementKind;
use serde_json::{json, Value};
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

/// Eingestellte Sprache (Vorgabe `de_DE`).
pub fn language(s: &Session) -> String {
    pref_str(s, "SpellingLanguage").filter(|l| !l.is_empty()).unwrap_or_else(|| "de_DE".into())
}

/// Persönliches Wörterbuch.
pub fn user_words(s: &Session) -> Vec<String> {
    pref_str(s, "SpellingUserWords").map(|t| t.lines().map(str::trim).filter(|w| !w.is_empty()).map(str::to_string).collect()).unwrap_or_default()
}

fn search_dirs(s: &Session, lang: &str) -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(d) = pref_str(s, "SpellingDictionaryDir").filter(|d| !d.is_empty()) {
        v.push(PathBuf::from(d));
    }
    if let Some(d) = std::env::var_os("FP_DICT_DIR") {
        v.push(PathBuf::from(d));
    }
    if let Some(p) = crate::prefs::Preferences::default_path().and_then(|p| p.parent().map(Path::to_path_buf)) {
        v.push(p.join("Dictionaries"));
    }
    if let Some(a) = std::env::var_os("APPDATA") {
        let a = PathBuf::from(a);
        v.push(a.join("Microsoft").join("Spelling").join(lang.replace('_', "-")));
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
    let d = spellbook::Dictionary::new(&read(aff)?, &read(dic)?).map_err(|e| EngineError::Failed(format!("Wörterbuch {}: {e}", dic.display())))?;
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
        let inner = (c == '\'' || c == '’' || c == '-') && !cur.is_empty() && chars.get(i + 1).is_some_and(|n| n.is_alphanumeric());
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
    dict: Option<Arc<spellbook::Dictionary>>,
    user: Vec<String>,
    ignore_upper: bool,
    ignore_numbers: bool,
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
        match &self.dict {
            Some(d) => d.check(w),
            None => true,
        }
    }
    fn suggest(&self, w: &str) -> Vec<String> {
        let mut v = Vec::new();
        if let Some(d) = &self.dict {
            d.suggest(w, &mut v);
        }
        v.truncate(8);
        v
    }
}

fn checker(s: &Session) -> Result<Checker> {
    let lang = language(s);
    let dict = match find_dictionary(s, &lang) {
        Some((a, d)) => Some(load(&a, &d)?),
        None => None,
    };
    Ok(Checker { dict, user: user_words(s), ignore_upper: pref_bool(s, "SpellingIgnoreUppercase", true), ignore_numbers: pref_bool(s, "SpellingIgnoreNumbers", true) })
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
        "dictionaryDir": pref_str(s, "SpellingDictionaryDir"),
        "userWords": user_words(s),
        "ignoreUppercase": pref_bool(s, "SpellingIgnoreUppercase", true),
        "ignoreNumbers": pref_bool(s, "SpellingIgnoreNumbers", true),
    })
}

fn set_user_words(s: &mut Session, w: Vec<String>) {
    s.prefs.set("SpellingUserWords", PrefValue::Str(w.join("\n")));
}

fn texts_of(s: &Session, p: &Params) -> Result<Vec<(fp_model::ElementId, String)>> {
    let d = s.doc()?;
    let ids = p.opt_ids("ids")?;
    let els: Vec<_> = match &ids {
        Some(v) if !v.is_empty() => v.iter().filter_map(|id| docops::element_on_page(&d.doc, d.page, *id)).collect(),
        _ if !d.selection.elements.is_empty() => selected_elements(d).into_iter().map(|(_, e)| e).collect(),
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
            .opt("language", ParamKind::String, "Sprache des Wörterbuchs (z. B. de_DE, en_US)")
            .opt("dictionaryDir", ParamKind::Path, "Ordner mit <sprache>.aff/.dic")
            .opt("addWords", ParamKind::Json, "Wörter ins persönliche Wörterbuch")
            .opt("removeWords", ParamKind::Json, "Wörter aus dem persönlichen Wörterbuch entfernen")
            .opt("ignoreUppercase", ParamKind::Bool, "Wörter in Großbuchstaben überspringen")
            .opt("ignoreNumbers", ParamKind::Bool, "Wörter mit Ziffern überspringen")
            .run(|s, p| {
                let mut changed = false;
                if let Some(l) = p.opt_str("language")? {
                    s.prefs.set("SpellingLanguage", PrefValue::Str(l));
                    changed = true;
                }
                if let Some(d) = p.opt_str("dictionaryDir")? {
                    s.prefs.set("SpellingDictionaryDir", PrefValue::Str(d));
                    changed = true;
                }
                for (k, key) in [("ignoreUppercase", "SpellingIgnoreUppercase"), ("ignoreNumbers", "SpellingIgnoreNumbers")] {
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
                    Some(_) => return Err(EngineError::InvalidParams { name: p.command.clone(), message: "replace: Liste".into() }),
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
                            for id in target_ids.iter().filter(|i| only.is_none_or(|o| o == **i)) {
                                docops::edit_by_id(doc, page, *id, |e| {
                                    if let ElementKind::Text(t) = &mut e.kind {
                                        let mut n = 0;
                                        for run in t.runs.iter_mut() {
                                            let (nt, k) = replace_word(&run.text, from, to);
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
                    for (off, w) in words(text) {
                        if !ck.ok(&w) {
                            issues.push(json!({"id": id.0, "word": w, "offset": off, "suggestions": ck.suggest(&w)}));
                        }
                    }
                }
                let value = json!({
                    "language": language(s), "dictionary": ck.dict.is_some(), "checked": texts.len(),
                    "misspelled": issues, "replaced": replaced,
                    "note": if ck.dict.is_none() { Some("kein Wörterbuch gefunden: <sprache>.aff/.dic über „Rechtschreibprüfung einrichten“ angeben") } else { None },
                });
                if replaced > 0 {
                    return Ok(CommandOutput::doc_with(value).labeled("Rechtschreibung"));
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
        let w: Vec<String> = words("Hallo Wlet, it's e-mail 42!").into_iter().map(|x| x.1).collect();
        assert_eq!(w, vec!["Hallo", "Wlet", "it's", "e-mail", "42"]);
        assert_eq!(replace_word("Wlet und Wlet, Wletter", "Wlet", "Welt"), ("Welt und Welt, Wletter".to_string(), 2));
    }
}

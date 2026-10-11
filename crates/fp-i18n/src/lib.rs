//! Mehrsprachigkeit von Firepixel auf Basis von [Fluent](https://projectfluent.org).
//!
//! # Texte
//! Jede Sprache ist ein Ordner `locales/<code>/` mit beliebig vielen `*.ftl`-Bereichsdateien
//! (z. B. `menue.ftl`, `dialoge.ftl`). Alle eingebauten Dateien werden beim Bauen eingebettet
//! (`build.rs`), die exe bleibt eine einzelne Datei. Weitere Sprachen (oder eigene Korrekturen)
//! werden zur Laufzeit aus Ordnern geladen ([`load_external_dir`]): je Sprache entweder
//! `<code>.ftl` oder ein Unterordner `<code>\*.ftl`. Jede Sprache nennt sich selbst über den
//! Schlüssel `language-name`.
//!
//! # Schlüssel
//! Kleinbuchstaben mit Bindestrich, Bereich vorn: `menu-file`, `cmd-file-open`, `dlg-unsaved-text`.
//! Attribute über einen Punkt: `t("cmd-file-open.tooltip")`. Befehle des Registers:
//! [`command_key`] (`file.open` → `cmd-file-open`).
//!
//! # Nachschlagen
//! [`t`] bzw. [`tr!`] liefert den Text in der aktuellen Sprache, sonst Englisch, sonst Deutsch,
//! sonst den Schlüssel selbst (fehlende Texte sind so sofort sichtbar). Die Testsprache
//! [`PSEUDO_LANG`] („qps“) zeigt jeden Text markiert und verlängert: unübersetzte, fest
//! eingebaute Texte fallen auf, und Layouts werden mit längeren Texten geprüft.

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource};
use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::{Arc, OnceLock, RwLock};
use unic_langid::LanguageIdentifier;

pub use fluent_bundle::FluentValue as Value;

include!(concat!(env!("OUT_DIR"), "/embedded.rs"));

/// Ausgangssprache aller Texte (vollständig, Maßstab für die Vollständigkeitsprüfung).
pub const SOURCE_LANG: &str = "de";
/// Rückfallsprache vor der Ausgangssprache.
pub const FALLBACK_LANG: &str = "en";
/// Testsprache: Texte der Ausgangssprache, markiert und um etwa 40 % verlängert.
pub const PSEUDO_LANG: &str = "qps";

/// Eine verfügbare Sprache.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LangInfo {
    /// Sprachcode (`de`, `en`, `fr`, `pt-BR` …).
    pub code: String,
    /// Eigenname der Sprache (`language-name`), z. B. „Deutsch“, „English“.
    pub name: String,
    /// In die exe eingebaut (sonst aus einem Sprachordner geladen).
    pub builtin: bool,
}

struct Lang {
    info: LangInfo,
    bundle: FluentBundle<FluentResource>,
    keys: BTreeSet<String>,
    errors: Vec<String>,
}

struct State {
    langs: Vec<Lang>,
    current: String,
    generation: u64,
    cache: HashMap<String, Arc<str>>,
}

fn state() -> &'static RwLock<State> {
    static STATE: OnceLock<RwLock<State>> = OnceLock::new();
    STATE.get_or_init(|| {
        let mut langs: Vec<Lang> = Vec::new();
        for (code, file, text) in EMBEDDED {
            add_source(&mut langs, code, file, text, true);
        }
        RwLock::new(State { langs, current: SOURCE_LANG.to_string(), generation: 1, cache: HashMap::new() })
    })
}

fn new_bundle(code: &str) -> FluentBundle<FluentResource> {
    let id: LanguageIdentifier = code.parse().unwrap_or_default();
    let mut b = FluentBundle::new_concurrent(vec![id]);
    // keine Unicode-Isolationszeichen um Platzhalter (egui zeigt sie sonst als Kästchen)
    b.set_use_isolating(false);
    b
}

fn add_source(langs: &mut Vec<Lang>, code: &str, file: &str, text: &str, builtin: bool) {
    let idx = match langs.iter().position(|l| l.info.code == code) {
        Some(i) => i,
        None => {
            langs.push(Lang {
                info: LangInfo { code: code.to_string(), name: code.to_string(), builtin },
                bundle: new_bundle(code),
                keys: BTreeSet::new(),
                errors: Vec::new(),
            });
            langs.len() - 1
        }
    };
    let lang = &mut langs[idx];
    let res = match FluentResource::try_new(text.to_string()) {
        Ok(r) => r,
        Err((r, errs)) => {
            for e in errs {
                lang.errors.push(format!("{file}: Syntaxfehler {e:?}"));
            }
            r
        }
    };
    for entry in res.entries() {
        if let fluent_syntax::ast::Entry::Message(m) = entry {
            if !lang.keys.insert(m.id.name.to_string()) {
                lang.errors.push(format!("{file}: Schlüssel „{}“ doppelt", m.id.name));
            }
            for a in &m.attributes {
                lang.keys.insert(format!("{}.{}", m.id.name, a.id.name));
            }
        }
    }
    // spätere Dateien (Sprachordner) überschreiben eingebaute Texte gleicher Schlüssel
    lang.bundle.add_resource_overriding(res);
    if let Some(name) = format_in(&lang.bundle, "language-name", None) {
        lang.info.name = name;
    }
}

fn format_in(bundle: &FluentBundle<FluentResource>, key: &str, args: Option<&FluentArgs>) -> Option<String> {
    let (id, attr) = match key.split_once('.') {
        Some((i, a)) => (i, Some(a)),
        None => (key, None),
    };
    let msg = bundle.get_message(id)?;
    let pattern = match attr {
        Some(a) => msg.get_attribute(a)?.value(),
        None => msg.value()?,
    };
    let mut errors = Vec::new();
    let s = bundle.format_pattern(pattern, args, &mut errors);
    if !errors.is_empty() {
        log::debug!("fp-i18n: {key}: {errors:?}");
    }
    Some(s.into_owned())
}

/// Testsprache: markiert und um etwa 40 % verlängert; Zugriffstasten (`&`) bleiben erhalten.
pub fn pseudo(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let n = text.chars().filter(|c| !c.is_whitespace()).count();
    let pad = (n * 2).div_ceil(5).max(1);
    format!("[!! {text} {}!!]", "·".repeat(pad))
}

fn lookup(st: &State, key: &str, args: Option<&FluentArgs>) -> Option<String> {
    if st.current == PSEUDO_LANG {
        return st.langs.iter().find(|l| l.info.code == SOURCE_LANG).and_then(|l| format_in(&l.bundle, key, args)).map(|s| pseudo(&s));
    }
    for code in [st.current.as_str(), FALLBACK_LANG, SOURCE_LANG] {
        if let Some(l) = st.langs.iter().find(|l| l.info.code == code) {
            if let Some(s) = format_in(&l.bundle, key, args) {
                return Some(s);
            }
        }
    }
    None
}

/// Text zum Schlüssel in der aktuellen Sprache (mit Rückfallkette; unbekannt → der Schlüssel).
pub fn t(key: &str) -> String {
    let lock = state();
    if let Ok(st) = lock.read() {
        if let Some(s) = st.cache.get(key) {
            return s.to_string();
        }
    }
    let text = match lock.read() {
        Ok(st) => lookup(&st, key, None),
        Err(_) => None,
    };
    let text = match text {
        Some(s) => s,
        None => {
            log::debug!("fp-i18n: fehlender Schlüssel {key}");
            key.to_string()
        }
    };
    if let Ok(mut st) = lock.write() {
        st.cache.insert(key.to_string(), Arc::from(text.as_str()));
    }
    text
}

/// Text mit Platzhaltern, z. B. `t_args("dlg-unsaved-text", &[("name", "a.fw.png".into())])`.
pub fn t_args(key: &str, args: &[(&str, Value<'_>)]) -> String {
    let mut fa = FluentArgs::new();
    for (k, v) in args {
        fa.set(*k, v.clone());
    }
    match state().read() {
        Ok(st) => lookup(&st, key, Some(&fa)).unwrap_or_else(|| key.to_string()),
        Err(_) => key.to_string(),
    }
}

/// `tr!("key")` oder `tr!("key", name = wert, n = 3)`.
#[macro_export]
macro_rules! tr {
    ($key:expr) => {
        $crate::t($key)
    };
    ($key:expr, $($name:ident = $val:expr),+ $(,)?) => {
        $crate::t_args($key, &[$((stringify!($name), $crate::Value::from($val))),+])
    };
}

/// Gibt es den Schlüssel in der aktuellen Sprache oder einer Rückfallsprache?
pub fn has(key: &str) -> bool {
    state().read().map(|st| lookup(&st, key, None).is_some()).unwrap_or(false)
}

/// Schlüssel eines Befehls aus dem Register: `file.open` → `cmd-file-open`,
/// `modify.align.left` → `cmd-modify-align-left` (camelCase bleibt erhalten).
pub fn command_key(command: &str) -> String {
    format!("cmd-{}", command.replace(['.', '_'], "-"))
}

/// Aktuelle Sprache.
pub fn current_language() -> String {
    state().read().map(|st| st.current.clone()).unwrap_or_else(|_| SOURCE_LANG.to_string())
}

/// Zähler, der bei jedem Sprachwechsel bzw. Nachladen steigt (Oberflächen-Caches daran koppeln).
pub fn generation() -> u64 {
    state().read().map(|st| st.generation).unwrap_or(0)
}

/// Sprache wechseln; `false`, wenn es den Code nicht gibt. [`PSEUDO_LANG`] ist immer erlaubt.
pub fn set_language(code: &str) -> bool {
    let Ok(mut st) = state().write() else { return false };
    let known = code == PSEUDO_LANG || st.langs.iter().any(|l| l.info.code == code);
    if !known {
        return false;
    }
    if st.current != code {
        st.current = code.to_string();
        st.cache.clear();
        st.generation += 1;
    }
    true
}

/// Alle verfügbaren Sprachen (eingebaute zuerst, dann nach Name).
pub fn languages() -> Vec<LangInfo> {
    let mut v: Vec<LangInfo> = state().read().map(|st| st.langs.iter().map(|l| l.info.clone()).collect()).unwrap_or_default();
    v.sort_by(|a, b| b.builtin.cmp(&a.builtin).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    v
}

/// Lädt Sprachdateien aus einem Ordner: `<code>.ftl` bzw. `<code>\*.ftl`. Gleiche Schlüssel
/// überschreiben eingebaute Texte. Gibt die geladenen Sprachcodes zurück.
pub fn load_external_dir(dir: &Path) -> Vec<String> {
    let mut found: Vec<(String, String, String)> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                let code = e.file_name().to_string_lossy().to_string();
                if let Ok(files) = std::fs::read_dir(&p) {
                    for f in files.flatten() {
                        let fp = f.path();
                        if fp.extension().and_then(|x| x.to_str()) == Some("ftl") {
                            if let Ok(text) = std::fs::read_to_string(&fp) {
                                found.push((code.clone(), fp.display().to_string(), text));
                            }
                        }
                    }
                }
            } else if p.extension().and_then(|x| x.to_str()) == Some("ftl") {
                let code = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                if let Ok(text) = std::fs::read_to_string(&p) {
                    found.push((code, p.display().to_string(), text));
                }
            }
        }
    }
    found.sort();
    let Ok(mut st) = state().write() else { return Vec::new() };
    let mut codes = Vec::new();
    for (code, file, text) in found {
        if code.parse::<LanguageIdentifier>().is_err() || code == PSEUDO_LANG {
            continue;
        }
        add_source(&mut st.langs, &code, &file, &text, false);
        if !codes.contains(&code) {
            codes.push(code);
        }
    }
    st.cache.clear();
    st.generation += 1;
    codes
}

/// Windows-/Systemsprache als Vorschlag für den ersten Start (nur der Sprachteil, z. B. `de`),
/// falls es dafür eine Sprache gibt, sonst [`FALLBACK_LANG`].
pub fn system_language() -> String {
    let sys = sys_locale::get_locale().unwrap_or_default();
    let base = sys.split(['-', '_']).next().unwrap_or("").to_lowercase();
    let codes: Vec<String> = languages().into_iter().map(|l| l.code).collect();
    if codes.iter().any(|c| c.eq_ignore_ascii_case(&sys)) {
        return sys;
    }
    if codes.iter().any(|c| *c == base) {
        return base;
    }
    FALLBACK_LANG.to_string()
}

/// Alle Schlüssel einer Sprache (inkl. `key.attribut`).
pub fn keys(code: &str) -> BTreeSet<String> {
    state().read().ok().and_then(|st| st.langs.iter().find(|l| l.info.code == code).map(|l| l.keys.clone())).unwrap_or_default()
}

/// Schlüssel der Ausgangssprache, die in `code` fehlen.
pub fn missing_keys(code: &str) -> Vec<String> {
    let src = keys(SOURCE_LANG);
    let have = keys(code);
    src.difference(&have).cloned().collect()
}

/// Schlüssel in `code`, die es in der Ausgangssprache nicht gibt (verwaist).
pub fn extra_keys(code: &str) -> Vec<String> {
    let src = keys(SOURCE_LANG);
    keys(code).difference(&src).cloned().collect()
}

/// Syntax- und Doppelungsfehler beim Laden einer Sprache.
pub fn errors(code: &str) -> Vec<String> {
    state().read().ok().and_then(|st| st.langs.iter().find(|l| l.info.code == code).map(|l| l.errors.clone())).unwrap_or_default()
}

/// Dezimalzahl mit dem Trennzeichen der aktuellen Sprache (`fmt-decimal-separator`, Vorgabe „,“).
pub fn fmt_decimal(v: f64, decimals: usize) -> String {
    let s = format!("{v:.decimals$}");
    let sep = if has("fmt-decimal-separator") { t("fmt-decimal-separator") } else { ",".to_string() };
    if sep == "." { s } else { s.replace('.', &sep) }
}

#[cfg(test)]
mod tests;

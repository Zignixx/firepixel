//! Mehrsprachigkeit von Firepixel auf Basis von [Fluent](https://projectfluent.org).
//!
//! # Texte
//! Jede Sprache ist ein Ordner `locales/<code>/` mit beliebig vielen `*.ftl`-Bereichsdateien
//! (z. B. `menu.ftl`, `dialogs.ftl`; Namen bis 13.10.2026 siehe
//! [`check::LEGACY_AREA_NAMES`]). Alle eingebauten Dateien werden beim Bauen eingebettet
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
//!
//! # Formate
//! Zugriffstasten ([`strip_access`], [`t_access`]), Tastenkürzel ([`shortcut_label`]),
//! Einheiten ([`unit`]), Zahlen ([`fmt_decimal`], Fluent-Funktion `NUMBER`), Dateigrößen
//! ([`fmt_size`]) und Datum/Uhrzeit ([`fmt_date`], [`fmt_time`]) folgen der aktuellen Sprache.
//!
//! # Leistung
//! [`t`] wird je Frame tausendfach aufgerufen. Jeder Thread hat einen eigenen Lese-Cache, der an
//! [`generation`] hängt: im Normalfall weder Sperre noch Fluent, nur ein Hash-Nachschlagen
//! (Messung: `tests\leistung.rs`, Werte im README).
//!
//! # Prüfen
//! [`check`] prüft Sprachdateien (fehlende/verwaiste Schlüssel, Platzhalter, Syntax,
//! Zugriffstasten-Konflikte) – für Tests und `firepixel-cli language check`.

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource};
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, RwLock};
use unic_langid::LanguageIdentifier;

pub use fluent_bundle::FluentValue as Value;

pub mod check;
mod format;
pub mod paths;
pub mod profile;
pub use format::{
    fmt_date, fmt_datetime, fmt_number, fmt_size, fmt_time, shortcut_label, strip_access, t_access, unit, AccessText,
};

include!(concat!(env!("OUT_DIR"), "/embedded.rs"));

/// Eingebaute Sprachdateien: (Sprachcode, Dateiname, Inhalt).
pub fn embedded_files() -> &'static [(&'static str, &'static str, &'static str)] {
    EMBEDDED
}

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
    /// Dezimaltrennzeichen (mt-decimal-separator) für Zahlen in Platzhaltern.
    sep: Arc<str>,
}

struct State {
    langs: Vec<Lang>,
    current: String,
}

/// Stand der Texte; steigt bei jedem Sprachwechsel bzw. Nachladen. Wird nur unter der
/// Schreibsperre von [`state`] erhöht (nach der Änderung), gelesen ohne Sperre.
static GENERATION: AtomicU64 = AtomicU64::new(1);

/// Lese-Cache je Thread: (Stand, Schlüssel → (Text, gefunden)).
struct LocalCache {
    generation: u64,
    map: FastMap<Box<str>, (Arc<str>, bool)>,
    /// Abgeleitete Texte (Platzhalter, Kürzel), begrenzt auf [DERIVED_MAX].
    derived: FastMap<Box<str>, Arc<str>>,
}

thread_local! {
    static LOCAL: RefCell<LocalCache> = RefCell::new(LocalCache { generation: 0, map: FastMap::default(), derived: FastMap::default() });
}

fn state() -> &'static RwLock<State> {
    static STATE: OnceLock<RwLock<State>> = OnceLock::new();
    STATE.get_or_init(|| {
        let mut langs: Vec<Lang> = Vec::new();
        for (code, file, text) in EMBEDDED {
            add_source(&mut langs, code, file, text, true);
        }
        RwLock::new(State { langs, current: SOURCE_LANG.to_string() })
    })
}

fn bump_generation() {
    GENERATION.fetch_add(1, Ordering::AcqRel);
}

/// Schneller Hash für die kurzen Schlüssel im Thread-Cache (FxHash-Verfahren; kein Schutz gegen
/// gezielte Kollisionen nötig, die Schlüssel stammen aus dem Programm).
#[derive(Default, Clone, Copy)]
struct FxHasher(u64);

impl std::hash::Hasher for FxHasher {
    fn write(&mut self, bytes: &[u8]) {
        const K: u64 = 0x517c_c1b7_2722_0a95;
        let (chunks, rest) = bytes.as_chunks::<8>();
        for c in chunks {
            self.0 = (self.0.rotate_left(5) ^ u64::from_le_bytes(*c)).wrapping_mul(K);
        }
        for &b in rest {
            self.0 = (self.0.rotate_left(5) ^ b as u64).wrapping_mul(K);
        }
    }
    fn write_u8(&mut self, i: u8) {
        self.0 = (self.0.rotate_left(5) ^ i as u64).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

type FastMap<K, V> = HashMap<K, V, std::hash::BuildHasherDefault<FxHasher>>;

thread_local! {
    /// Dezimaltrennzeichen der Sprache, deren Bundle gerade formatiert (für [`format_value`],
    /// einen `fn`-Zeiger ohne eigenen Zustand).
    static FMT_SEP: RefCell<Arc<str>> = RefCell::new(Arc::from("."));
}

/// Zahlen in Fluent-Texten (`{ $n }`, `{ NUMBER($n, maximumFractionDigits: 1) }`) mit dem
/// Dezimaltrennzeichen der Sprache des Bundles; ohne Tausendertrennung.
fn format_value<M>(v: &Value<'_>, _intls: &M) -> Option<String> {
    let Value::Number(n) = v else { return None };
    let sep = FMT_SEP.with(|s| s.borrow().clone());
    Some(format::number_string(n.value, n.options.minimum_fraction_digits, n.options.maximum_fraction_digits, &sep))
}

/// Fluent-Funktion `NUMBER(wert, minimumFractionDigits: …, maximumFractionDigits: …)`.
fn number_function<'a>(pos: &[Value<'a>], named: &FluentArgs) -> Value<'a> {
    match pos.first() {
        Some(Value::Number(n)) => {
            let mut n = n.clone();
            n.options.merge(named);
            Value::Number(n)
        }
        Some(Value::String(s)) => match s.parse::<fluent_bundle::types::FluentNumber>() {
            Ok(mut n) => {
                n.options.merge(named);
                Value::Number(n)
            }
            Err(_) => Value::Error,
        },
        _ => Value::Error,
    }
}

fn new_bundle(code: &str) -> FluentBundle<FluentResource> {
    let id: LanguageIdentifier = code.parse().unwrap_or_default();
    let mut b = FluentBundle::new_concurrent(vec![id]);
    // keine Unicode-Isolationszeichen um Platzhalter (egui zeigt sie sonst als Kästchen)
    b.set_use_isolating(false);
    b.set_formatter(Some(format_value));
    let _ = b.add_function("NUMBER", number_function);
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
                sep: Arc::from("."),
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
    if let Some(name) = format_in(lang, "language-name", None) {
        lang.info.name = name;
    }
    if let Some(sep) = format_in(lang, "fmt-decimal-separator", None) {
        lang.sep = Arc::from(sep);
    }
}

fn format_in(lang: &Lang, key: &str, args: Option<&FluentArgs>) -> Option<String> {
    let bundle = &lang.bundle;
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
    FMT_SEP.with(|s| *s.borrow_mut() = Arc::clone(&lang.sep));
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

thread_local! {
    /// Sprache, die in diesem Thread vorübergehend gilt ([with_language]).
    static OVERRIDE: RefCell<Option<String>> = const { RefCell::new(None) };
}

fn override_lang() -> Option<String> {
    OVERRIDE.with(|o| o.borrow().clone())
}

/// Führt  mit einer anderen Sprache aus – nur in diesem Thread, die Oberfläche bleibt
/// unverändert (z. B. englische Fehlertexte für KI-Agenten über die MCP-Brücke). Unbekannte
/// Codes: Rückfallkette wie sonst (Englisch, Deutsch). Verschachtelbar.
pub fn with_language<R>(code: &str, f: impl FnOnce() -> R) -> R {
    struct Restore(Option<String>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let prev = self.0.take();
            OVERRIDE.with(|o| *o.borrow_mut() = prev);
        }
    }
    let prev = OVERRIDE.with(|o| o.borrow_mut().replace(code.to_string()));
    let _restore = Restore(prev);
    f()
}

fn lookup(st: &State, key: &str, args: Option<&FluentArgs>) -> Option<String> {
    let ov = override_lang();
    let current = ov.as_deref().unwrap_or(st.current.as_str());
    if current == PSEUDO_LANG {
        return st.langs.iter().find(|l| l.info.code == SOURCE_LANG).and_then(|l| format_in(l, key, args)).map(|s| pseudo(&s));
    }
    let current = st.langs.iter().find(|l| l.info.code.eq_ignore_ascii_case(current)).map(|l| l.info.code.as_str()).unwrap_or(current);
    for code in [current, FALLBACK_LANG, SOURCE_LANG] {
        if let Some(l) = st.langs.iter().find(|l| l.info.code == code)
            && let Some(s) = format_in(l, key, args)
        {
            return Some(s);
        }
    }
    None
}

/// Nachschlagen über den Thread-Cache: (Text, gefunden). Im Normalfall ohne Sperre.
fn cached(key: &str) -> (Arc<str>, bool) {
    if OVERRIDE.with(|o| o.borrow().is_some()) {
        // vorübergehende Sprache: am Cache vorbei
        return match state().read().ok().and_then(|st| lookup(&st, key, None)) {
            Some(s) => (Arc::from(s), true),
            None => (Arc::from(key), false),
        };
    }
    let generation = GENERATION.load(Ordering::Acquire);
    let hit = LOCAL.with(|c| {
        let mut c = c.borrow_mut();
        if c.generation != generation {
            c.map.clear();
            c.derived.clear();
            c.generation = generation;
            return None;
        }
        c.map.get(key).cloned()
    });
    if let Some(hit) = hit {
        return hit;
    }
    let text = match state().read() {
        Ok(st) => lookup(&st, key, None),
        Err(_) => None,
    };
    let entry = match text {
        Some(s) => (Arc::<str>::from(s), true),
        None => {
            log::debug!("fp-i18n: fehlender Schlüssel {key}");
            (Arc::<str>::from(key), false)
        }
    };
    LOCAL.with(|c| {
        let mut c = c.borrow_mut();
        // Stand vor dem Nachschlagen: hat sich die Sprache inzwischen geändert, wird der Eintrag
        // beim nächsten Aufruf mit dem ganzen Cache verworfen
        if c.generation == generation {
            c.map.insert(Box::from(key), entry.clone());
        }
    });
    entry
}

/// Text zum Schlüssel in der aktuellen Sprache (mit Rückfallkette; unbekannt → der Schlüssel).
pub fn t(key: &str) -> String {
    cached(key).0.to_string()
}

/// Wie [`t`], aber ohne neue Zeichenkette (geteilter Text aus dem Cache).
pub fn t_arc(key: &str) -> Arc<str> {
    cached(key).0
}

/// Höchstzahl abgeleiteter Einträge je Thread (Texte mit Platzhaltern ändern sich z. B. beim
/// Ziehen eines Schiebers in jedem Frame); darüber wird der Cache geleert.
const DERIVED_MAX: usize = 4096;

/// Abgeleiteter Text (Platzhalter, Kürzel) aus dem Thread-Cache bzw. neu berechnet.
/// `id` muss Art und alle Eingaben eindeutig beschreiben.
pub(crate) fn derived(id: &str, make: impl FnOnce() -> String) -> Arc<str> {
    if OVERRIDE.with(|o| o.borrow().is_some()) {
        return Arc::from(make());
    }
    let generation = GENERATION.load(Ordering::Acquire);
    let hit = LOCAL.with(|c| {
        let mut c = c.borrow_mut();
        if c.generation != generation {
            c.map.clear();
            c.derived.clear();
            c.generation = generation;
            return None;
        }
        c.derived.get(id).cloned()
    });
    if let Some(hit) = hit {
        return hit;
    }
    let text: Arc<str> = Arc::from(make());
    LOCAL.with(|c| {
        let mut c = c.borrow_mut();
        if c.generation == generation {
            if c.derived.len() >= DERIVED_MAX {
                c.derived.clear();
            }
            c.derived.insert(Box::from(id), Arc::clone(&text));
        }
    });
    text
}

fn push_value_id(id: &mut String, v: &Value<'_>) {
    use std::fmt::Write as _;
    match v {
        Value::String(s) => {
            id.push('s');
            id.push_str(s);
        }
        Value::Number(n) => {
            let _ = write!(id, "n{}/{:?}/{:?}", n.value, n.options.minimum_fraction_digits, n.options.maximum_fraction_digits);
        }
        Value::Custom(_) => id.push_str("c?"),
        Value::None => id.push('0'),
        Value::Error => id.push('!'),
    }
}

/// Text mit Platzhaltern, z. B. `t_args("dlg-unsaved-text", &[("name", "a.fw.png".into())])`.
/// Ergebnisse werden je Thread zwischengespeichert (gleiche Werte → kein erneutes Formatieren).
pub fn t_args(key: &str, args: &[(&str, Value<'_>)]) -> String {
    let cacheable = !args.iter().any(|(_, v)| matches!(v, Value::Custom(_)));
    let format = || {
        let mut fa = FluentArgs::new();
        for (k, v) in args {
            fa.set(*k, v.clone());
        }
        match state().read() {
            Ok(st) => lookup(&st, key, Some(&fa)).unwrap_or_else(|| key.to_string()),
            Err(_) => key.to_string(),
        }
    };
    if !cacheable {
        return format();
    }
    let mut id = String::with_capacity(key.len() + 32);
    id.push('\u{1}');
    id.push_str(key);
    for (k, v) in args {
        id.push('\u{1f}');
        id.push_str(k);
        id.push('=');
        push_value_id(&mut id, v);
    }
    derived(&id, format).to_string()
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
    cached(key).1
}

/// Schlüssel eines Befehls aus dem Register: `file.open` → `cmd-file-open`,
/// `modify.align.left` → `cmd-modify-align-left` (camelCase bleibt erhalten).
pub fn command_key(command: &str) -> String {
    format!("cmd-{}", command.replace(['.', '_'], "-"))
}

/// Aktuelle Sprache.
pub fn current_language() -> String {
    if let Some(code) = override_lang() {
        return code;
    }
    state().read().map(|st| st.current.clone()).unwrap_or_else(|_| SOURCE_LANG.to_string())
}

/// Zähler, der bei jedem Sprachwechsel bzw. Nachladen steigt (Oberflächen-Caches daran koppeln).
pub fn generation() -> u64 {
    GENERATION.load(Ordering::Acquire)
}

/// Sprache wechseln; `false`, wenn es den Code nicht gibt. [`PSEUDO_LANG`] ist immer erlaubt.
/// Groß-/Kleinschreibung des Codes ist egal (`pt-br` = `pt-BR`).
pub fn set_language(code: &str) -> bool {
    if language_per_thread() {
        return set_thread_language(code);
    }
    let Ok(mut st) = state().write() else { return false };
    let code = if code == PSEUDO_LANG {
        PSEUDO_LANG.to_string()
    } else {
        match st.langs.iter().find(|l| l.info.code.eq_ignore_ascii_case(code)) {
            Some(l) => l.info.code.clone(),
            None => return false,
        }
    };
    if st.current != code {
        st.current = code;
        bump_generation();
    }
    true
}

/// Sprache je Thread statt je Prozess (INF-1): nur im gemeinsamen Testprogramm eines Crates
/// (`tests\all.rs`, exe `…\deps\all-<hash>.exe`, siehe `crates\fp-testlist`) bzw. mit
/// `FP_LANGUAGE_PER_THREAD=1`. Dort laufen die Tests vieler Dateien parallel in einem Prozess; ein
/// Test, der auf Englisch umschaltet, darf die gleichzeitig laufenden deutschen Tests nicht
/// stören. Jeder Test läuft in einem eigenen Thread und beginnt mit der Ausgangssprache.
/// Alle anderen Programme (App, CLI, einzelne Testprogramme): Sprache prozessweit wie bisher.
fn language_per_thread() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| {
        if let Some(v) = std::env::var_os("FP_LANGUAGE_PER_THREAD") {
            return !v.is_empty() && v != "0";
        }
        let Ok(exe) = std::env::current_exe() else { return false };
        let in_deps = exe.parent().and_then(|d| d.file_name()).is_some_and(|n| n.eq_ignore_ascii_case("deps"));
        let stem = exe.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        in_deps && stem.strip_prefix("all-").is_some_and(|h| !h.is_empty() && h.chars().all(|c| c.is_ascii_hexdigit()))
    })
}

/// [`set_language`] für den aufrufenden Thread ([`language_per_thread`]).
fn set_thread_language(code: &str) -> bool {
    let code = if code == PSEUDO_LANG {
        PSEUDO_LANG.to_string()
    } else {
        let Ok(st) = state().read() else { return false };
        match st.langs.iter().find(|l| l.info.code.eq_ignore_ascii_case(code)) {
            Some(l) => l.info.code.clone(),
            None => return false,
        }
    };
    let changed = OVERRIDE.with(|o| o.borrow_mut().replace(code.clone()).as_deref() != Some(code.as_str()));
    if changed {
        bump_generation();
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
                        if fp.extension().and_then(|x| x.to_str()) == Some("ftl")
                            && let Ok(text) = std::fs::read_to_string(&fp)
                        {
                            found.push((code.clone(), fp.display().to_string(), text));
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
    bump_generation();
    drop(st);
    codes
}

/// Name des Sprachordners (technisch immer englisch, AGENTEN_REGELN §1c).
pub const LANGUAGE_DIR: &str = "Languages";
/// Alter deutscher Name des Sprachordners (wird beim Start einmal umbenannt, sonst als Rückfall gelesen).
pub const LEGACY_LANGUAGE_DIR: &str = "Sprachen";

/// Nutzerordner von Firepixel: `%APPDATA%\Firepixel` (sonst `$XDG_CONFIG_HOME/firepixel` bzw.
/// `$HOME/.config/firepixel`); in Testläufen das Testprofil ([`profile::roaming_dir`]).
pub fn user_config_dir() -> Option<std::path::PathBuf> {
    profile::roaming_dir()
}

/// Sprachordner in `base`: `Languages`, alter Ordner `Sprachen` wird migriert bzw. als Rückfall benutzt.
pub fn language_dir_in(base: &std::path::Path) -> std::path::PathBuf {
    paths::migrate_dir(&base.join(LANGUAGE_DIR), &base.join(LEGACY_LANGUAGE_DIR))
}

/// Nutzerordner für zusätzliche Sprachen: `%APPDATA%\Firepixel\Languages`
/// (sonst `$XDG_CONFIG_HOME/firepixel/Languages` bzw. `$HOME/.config/firepixel/Languages`;
/// alter Name `Sprachen` siehe [`language_dir_in`]).
pub fn user_language_dir() -> Option<std::path::PathBuf> {
    user_config_dir().map(|b| language_dir_in(&b))
}

/// Ordner `Languages` neben der laufenden exe (für tragbare Installationen; alter Name `Sprachen`).
pub fn exe_language_dir() -> Option<std::path::PathBuf> {
    std::env::current_exe().ok().and_then(|p| p.parent().map(language_dir_in))
}

/// Lädt beide Sprachordner (neben der exe, dann Nutzerordner – der Nutzerordner gewinnt bei
/// gleichen Schlüsseln). Liegt neben einem neuen `Languages` noch ein alter `Sprachen`, wird dieser
/// vorher mitgelesen (der neue gewinnt). Gibt die geladenen Sprachcodes zurück.
pub fn load_default_dirs() -> Vec<String> {
    let mut codes = Vec::new();
    let exe_base = std::env::current_exe().ok().and_then(|p| p.parent().map(std::path::Path::to_path_buf));
    let mut dirs = Vec::new();
    for (base, chosen) in [(exe_base, exe_language_dir()), (user_config_dir(), user_language_dir())] {
        if let (Some(b), Some(c)) = (base, &chosen) {
            dirs.extend(paths::leftover_dir(c, &b.join(LEGACY_LANGUAGE_DIR)));
        }
        dirs.extend(chosen);
    }
    for dir in dirs {
        if dir.is_dir() {
            for c in load_external_dir(&dir) {
                if !codes.contains(&c) {
                    codes.push(c);
                }
            }
        }
    }
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
    if codes.contains(&base) {
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
    let (sep, found) = cached("fmt-decimal-separator");
    let sep: &str = if found { &sep } else { "," };
    if sep == "." { s } else { s.replace('.', sep) }
}

#[cfg(test)]
mod tests;

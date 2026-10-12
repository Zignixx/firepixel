//! Tastenkürzel und die fünf mitgelieferten Tastenkürzel-Sets (10 §4.1 Punkt 11, §2.1).
//!
//! Die Sets „Firepixel“ (Standardset des Originals), „Photoshop“, „Illustrator“, „FreeHand“ und „Web Standard“ liegen als reine
//! Zuordnungen in `data/shortcuts.json` (erzeugt aus den XML-Dateien der Installation, keine Datei des Originals
//! eingebettet): Menübefehle per `cmd_`-Name, Werkzeuge per Werkzeug-GUID. Ein Kürzel ist
//! Modifikatoren + Windows-Virtual-Key, wie im XML-Format (`M VK`, M: Alt 1, Umschalt 2, Strg 4).
//!
//! Regeln (B, Strings 10218/10219): Werkzeugkürzel haben keine Modifikatoren, Menükürzel müssen
//! welche haben. Mehrere Werkzeuge mit gleichem Kürzel werden beim wiederholten Drücken
//! durchgeschaltet (siehe [`crate::tools::cycle_tool_for_key`]).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::sync::OnceLock;

/// Ein Tastenkürzel: Modifikatoren und Windows-Virtual-Key-Code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Shortcut {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    /// Virtual-Key-Code (Buchstaben = ASCII-Großbuchstabe, Ziffern = ASCII, F1 = 112 …).
    pub vk: u16,
}

impl Shortcut {
    /// Aus dem XML-Format: Bitmaske (Alt 1, Umschalt 2, Strg 4) und VK.
    pub fn from_fw(mask: u8, vk: u16) -> Shortcut {
        Shortcut { alt: mask & 1 != 0, shift: mask & 2 != 0, ctrl: mask & 4 != 0, vk }
    }

    /// Bitmaske wie im XML-Format.
    pub fn fw_mask(&self) -> u8 {
        (self.alt as u8) | ((self.shift as u8) << 1) | ((self.ctrl as u8) << 2)
    }

    /// Kürzel ohne Modifikatoren für eine Taste.
    pub fn key(vk: u16) -> Shortcut {
        Shortcut { ctrl: false, alt: false, shift: false, vk }
    }

    /// Hat das Kürzel Modifikatoren (Pflicht für Menükürzel)?
    pub fn has_modifiers(&self) -> bool {
        self.ctrl || self.alt || self.shift
    }

    /// Deutsche Anzeige wie in den Menüs des Originals, z. B. `Strg+Umschalt+S`.
    pub fn display_de(&self) -> String {
        let mut s = String::new();
        if self.ctrl {
            s.push_str("Strg+");
        }
        if self.alt {
            s.push_str("Alt+");
        }
        if self.shift {
            s.push_str("Umschalt+");
        }
        s.push_str(&key_name(self.vk, true));
        s
    }

    /// Englische Anzeige (kanonische Textform, von [`Shortcut::parse`] lesbar), z. B. `Ctrl+Shift+S`.
    pub fn display_en(&self) -> String {
        let mut s = String::new();
        if self.ctrl {
            s.push_str("Ctrl+");
        }
        if self.alt {
            s.push_str("Alt+");
        }
        if self.shift {
            s.push_str("Shift+");
        }
        s.push_str(&key_name(self.vk, false));
        s
    }

    /// Liest `Ctrl+Shift+S`, `Strg+Umschalt+S`, `Alt+F4`, `F12`, `VK191` … (Groß-/Kleinschreibung egal).
    pub fn parse(text: &str) -> Option<Shortcut> {
        let mut sc = Shortcut::key(0);
        let parts: Vec<&str> = split_keys(text);
        let (last, mods) = parts.split_last()?;
        for m in mods {
            match m.to_ascii_lowercase().as_str() {
                "ctrl" | "strg" | "control" | "cmd" => sc.ctrl = true,
                "alt" => sc.alt = true,
                "shift" | "umschalt" => sc.shift = true,
                _ => return None,
            }
        }
        sc.vk = vk_from_name(last)?;
        Some(sc)
    }
}

impl fmt::Display for Shortcut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.display_en())
    }
}

/// Teilt an `+`, wobei ein abschließendes `+` (Taste Plus) erhalten bleibt.
fn split_keys(text: &str) -> Vec<&str> {
    let t = text.trim();
    if t.is_empty() {
        return Vec::new();
    }
    if let Some(head) = t.strip_suffix("++") {
        let mut v: Vec<&str> = if head.is_empty() { Vec::new() } else { head.split('+').collect() };
        v.push("+");
        return v;
    }
    if t == "+" {
        return vec!["+"];
    }
    t.split('+').collect()
}

const NAMED: &[(u16, &str, &str)] = &[
    (8, "Rück", "Backspace"),
    (9, "Tab", "Tab"),
    (13, "Eingabe", "Enter"),
    (27, "Esc", "Escape"),
    (32, "Leertaste", "Space"),
    (33, "Bild auf", "PageUp"),
    (34, "Bild ab", "PageDown"),
    (35, "Ende", "End"),
    (36, "Pos1", "Home"),
    (37, "Links", "Left"),
    (38, "Oben", "Up"),
    (39, "Rechts", "Right"),
    (40, "Unten", "Down"),
    (45, "Einfg", "Insert"),
    (46, "Entf", "Delete"),
    (106, "Num *", "NumMultiply"),
    (107, "Num +", "NumAdd"),
    (109, "Num -", "NumSubtract"),
    (110, "Num ,", "NumDecimal"),
    (111, "Num /", "NumDivide"),
    (186, "Ü", ";"),
    (187, "+", "="),
    (188, ",", ","),
    (189, "-", "-"),
    (190, ".", "."),
    (191, "#", "/"),
    (192, "Ö", "`"),
    (219, "ß", "["),
    (220, "^", "\\"),
    (221, "´", "]"),
    (222, "Ä", "'"),
    (226, "<", "OEM102"),
];

/// Name einer Taste (deutsche Tastatur bzw. englische Bezeichnung).
pub fn key_name(vk: u16, german: bool) -> String {
    match vk {
        48..=57 | 65..=90 => char::from(vk as u8).to_string(),
        96..=105 => format!("Num {}", vk - 96),
        112..=135 => format!("F{}", vk - 111),
        _ => NAMED
            .iter()
            .find(|(v, _, _)| *v == vk)
            .map(|(_, de, en)| if german { de.to_string() } else { en.to_string() })
            .unwrap_or_else(|| format!("VK{vk}")),
    }
}

/// Virtual-Key-Code zu einem Tastennamen (deutsch oder englisch, `VKnnn`).
pub fn vk_from_name(name: &str) -> Option<u16> {
    let n = name.trim();
    if n.chars().count() == 1 {
        let c = n.chars().next()?;
        if c.is_ascii_alphanumeric() {
            return Some(c.to_ascii_uppercase() as u16);
        }
    }
    if let Some(num) = n.strip_prefix("VK").or_else(|| n.strip_prefix("vk")) {
        return num.parse().ok();
    }
    if let Some(f) = n.strip_prefix('F').or_else(|| n.strip_prefix('f'))
        && let Ok(k) = f.parse::<u16>()
        && (1..=24).contains(&k)
    {
        return Some(111 + k);
    }
    NAMED
        .iter()
        .find(|(_, de, en)| de.eq_ignore_ascii_case(n) || en.eq_ignore_ascii_case(n))
        .map(|(v, _, _)| *v)
}

/// Ein Tastenkürzel-Set.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ShortcutSet {
    /// Name des Sets (z. B. „Firepixel“).
    pub name: String,
    /// Menübefehle: `cmd_`-Name → Kürzel.
    pub menu: BTreeMap<String, Vec<Shortcut>>,
    /// Werkzeuge: Werkzeug-GUID (klein, `{8-4-4-4-12}`) → Kürzel.
    pub tools: BTreeMap<String, Vec<Shortcut>>,
}

impl ShortcutSet {
    /// Kürzel eines Menübefehls (`cmd_`-Name).
    pub fn command(&self, cmd: &str) -> &[Shortcut] {
        self.menu.get(cmd).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Kürzel eines Werkzeugs (GUID, Schreibweise egal).
    pub fn tool(&self, guid: &str) -> &[Shortcut] {
        self.tools.get(&normalize_guid(guid)).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Alle `cmd_`-Namen mit diesem Kürzel.
    pub fn commands_for(&self, sc: &Shortcut) -> Vec<&str> {
        self.menu.iter().filter(|(_, v)| v.contains(sc)).map(|(k, _)| k.as_str()).collect()
    }

    /// Alle Werkzeug-GUIDs mit diesem Kürzel.
    pub fn tools_for(&self, sc: &Shortcut) -> Vec<&str> {
        self.tools.iter().filter(|(_, v)| v.contains(sc)).map(|(k, _)| k.as_str()).collect()
    }

    /// Setzt die Kürzel eines Menübefehls. Menükürzel brauchen Modifikatoren (String 10219).
    pub fn set_command(&mut self, cmd: &str, shortcuts: Vec<Shortcut>) -> Result<(), String> {
        if shortcuts.iter().any(|s| !s.has_modifiers() && !(112..=135).contains(&s.vk) && s.vk != 46 && s.vk != 8 && s.vk != 9) {
            return Err(fp_i18n::t("err-shortcut-menu-needs-modifier"));
        }
        self.menu.insert(cmd.to_string(), shortcuts);
        Ok(())
    }

    /// Setzt die Kürzel eines Werkzeugs. Werkzeugkürzel: Buchstaben/Zahlen ohne Modifikatoren (10218).
    pub fn set_tool(&mut self, guid: &str, shortcuts: Vec<Shortcut>) -> Result<(), String> {
        if shortcuts.iter().any(|s| s.has_modifiers() || !(48..=57).contains(&s.vk) && !(65..=90).contains(&s.vk)) {
            return Err(fp_i18n::t("err-shortcut-tool-plain"));
        }
        self.tools.insert(normalize_guid(guid), shortcuts);
        Ok(())
    }
}

/// GUID in die Form `{xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx}` (klein) bringen; die XML-Dateien lassen den
/// letzten Bindestrich weg.
pub fn normalize_guid(g: &str) -> String {
    let h: String = g.chars().filter(|c| c.is_ascii_hexdigit()).map(|c| c.to_ascii_lowercase()).collect();
    if h.len() != 32 {
        return g.to_ascii_lowercase();
    }
    format!("{{{}-{}-{}-{}-{}}}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

#[derive(Deserialize)]
struct RawSet {
    menu: BTreeMap<String, Vec<(u8, u16)>>,
    tools: BTreeMap<String, Vec<(u8, u16)>>,
}

#[derive(Deserialize)]
struct RawFile {
    sets: BTreeMap<String, RawSet>,
    order: Vec<String>,
}

static BUILTIN: OnceLock<Vec<ShortcutSet>> = OnceLock::new();

/// Die fünf mitgelieferten Sets in der Reihenfolge Firepixel, Photoshop, Illustrator, FreeHand,
/// Web Standard.
pub fn builtin_sets() -> &'static [ShortcutSet] {
    BUILTIN.get_or_init(|| {
        let Ok(raw) = serde_json::from_str::<RawFile>(include_str!("../data/shortcuts.json")) else {
            return Vec::new();
        };
        let conv = |m: &BTreeMap<String, Vec<(u8, u16)>>| -> BTreeMap<String, Vec<Shortcut>> {
            m.iter().map(|(k, v)| (k.clone(), v.iter().map(|&(mask, vk)| Shortcut::from_fw(mask, vk)).collect())).collect()
        };
        raw.order
            .iter()
            .filter_map(|n| raw.sets.get(n).map(|s| ShortcutSet { name: n.clone(), menu: conv(&s.menu), tools: conv(&s.tools) }))
            .collect()
    })
}

/// Mitgeliefertes Set nach Namen (Groß-/Kleinschreibung egal).
pub fn builtin_set(name: &str) -> Option<&'static ShortcutSet> {
    builtin_sets().iter().find(|s| s.name.eq_ignore_ascii_case(name))
}

/// Namen der mitgelieferten Sets.
pub fn builtin_set_names() -> Vec<&'static str> {
    builtin_sets().iter().map(|s| s.name.as_str()).collect()
}

// ------------------------------------------------------------------ eigene Sets

/// Ordner der eigenen Sets: `FP_SHORTCUTS_DIR` bzw. `Shortcuts` neben der Voreinstellungsdatei.
pub fn custom_dir() -> std::path::PathBuf {
    if let Some(d) = std::env::var_os("FP_SHORTCUTS_DIR") {
        return d.into();
    }
    crate::prefs::Preferences::default_path()
        .and_then(|p| p.parent().map(|d| d.join("Shortcuts")))
        .unwrap_or_else(|| std::env::temp_dir().join("Firepixel").join("Shortcuts"))
}

type CustomMap = std::sync::Mutex<BTreeMap<String, &'static ShortcutSet>>;

fn custom_map() -> &'static CustomMap {
    static CUSTOM: OnceLock<CustomMap> = OnceLock::new();
    CUSTOM.get_or_init(Default::default)
}

fn file_of(name: &str) -> std::path::PathBuf {
    let safe: String = name.chars().map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '_' }).collect();
    custom_dir().join(format!("{safe}.json"))
}

/// Eigenes Set nach Namen (aus dem Speicher bzw. der Datei im Ordner [`custom_dir`]).
///
/// Sets leben bis zum Programmende (sie werden als `'static` an die Sitzung gegeben); jede
/// Speicherung legt eine neue Fassung an.
pub fn custom_set(name: &str) -> Option<&'static ShortcutSet> {
    if builtin_set(name).is_some() {
        return None;
    }
    let map = custom_map();
    if let Ok(m) = map.lock()
        && let Some(s) = m.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| *v)
    {
        return Some(s);
    }
    let text = std::fs::read_to_string(file_of(name)).ok()?;
    let set: ShortcutSet = serde_json::from_str(&text).ok()?;
    let leaked: &'static ShortcutSet = Box::leak(Box::new(set));
    if let Ok(mut m) = map.lock() {
        m.insert(leaked.name.clone(), leaked);
    }
    Some(leaked)
}

/// Mitgeliefertes oder eigenes Set.
pub fn any_set(name: &str) -> Option<&'static ShortcutSet> {
    builtin_set(name).or_else(|| custom_set(name))
}

/// Speichert ein eigenes Set (Datei + Speicher). Namen der mitgelieferten Sets sind gesperrt.
pub fn save_custom_set(set: &ShortcutSet) -> Result<&'static ShortcutSet, String> {
    if set.name.trim().is_empty() || builtin_set(&set.name).is_some() {
        return Err(fp_i18n::tr!("err-invalid-set-name", name = set.name.as_str()));
    }
    let f = file_of(&set.name);
    if let Some(d) = f.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    std::fs::write(&f, serde_json::to_string_pretty(set).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let leaked: &'static ShortcutSet = Box::leak(Box::new(set.clone()));
    if let Ok(mut m) = custom_map().lock() {
        m.insert(leaked.name.clone(), leaked);
    }
    Ok(leaked)
}

/// Löscht ein eigenes Set.
pub fn delete_custom_set(name: &str) -> Result<(), String> {
    if builtin_set(name).is_some() {
        return Err(fp_i18n::t("err-shortcut-builtin-no-delete"));
    }
    if let Ok(mut m) = custom_map().lock() {
        m.retain(|k, _| !k.eq_ignore_ascii_case(name));
    }
    match std::fs::remove_file(file_of(name)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(fp_i18n::tr!("err-set-not-found", name = name)),
        Err(e) => Err(e.to_string()),
    }
}

/// Namen der eigenen Sets (Dateien im Ordner).
pub fn custom_set_names() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(custom_dir())
        .map(|it| {
            it.flatten()
                .filter_map(|e| {
                    let t = std::fs::read_to_string(e.path()).ok()?;
                    serde_json::from_str::<ShortcutSet>(&t).ok().map(|s| s.name)
                })
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_display() {
        let s = Shortcut::parse("Strg+Umschalt+S").unwrap_or(Shortcut::key(0));
        assert_eq!(s, Shortcut { ctrl: true, alt: false, shift: true, vk: 83 });
        assert_eq!(s.display_de(), "Strg+Umschalt+S");
        assert_eq!(Shortcut::parse(&s.display_en()), Some(s));
        let plus = Shortcut::parse("Strg++").unwrap_or(Shortcut::key(0));
        assert_eq!(plus.vk, 187);
        assert_eq!(Shortcut::from_fw(7, 74).display_de(), "Strg+Alt+Umschalt+J");
        assert_eq!(Shortcut::parse("F12").map(|s| s.vk), Some(123));
        assert_eq!(normalize_guid("{4e54aeee-c168-11d0-ba0200059a80e4fd}"), "{4e54aeee-c168-11d0-ba02-00059a80e4fd}");
    }

    #[test]
    fn builtin_has_five_sets() {
        assert_eq!(builtin_set_names(), vec!["Firepixel", "Photoshop", "Illustrator", "FreeHand", "Web Standard"]);
        let fw = builtin_set("firepixel").map(|s| s.command("cmd_New").to_vec()).unwrap_or_default();
        assert_eq!(fw, vec![Shortcut { ctrl: true, alt: false, shift: false, vk: 78 }]);
    }
}

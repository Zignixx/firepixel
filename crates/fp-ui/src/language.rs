//! Sprachwahl der Oberfläche (Phase 6, I18N-KERN): Untermenü „Ansicht ▸ Sprache“, Start
//! (Sprachordner laden, gespeicherte bzw. Systemsprache), Voreinstellungsfeld, Schriften für
//! Chinesisch/Japanisch/Koreanisch.
//!
//! Die Sprache steht in den Voreinstellungen von fp-engine unter [`PREF_KEY`] (`"Language"`,
//! Sprachcode als Text; fehlt er, gilt beim Start die Systemsprache). Ein Wechsel wirkt sofort:
//! alle Texte werden je Frame über `fp_i18n::t` geholt.
//!
//! # Schriften
//! Segoe UI deckt Lateinisch, Kyrillisch, Griechisch, Arabisch und Hebräisch ab. Für Chinesisch,
//! Japanisch und Koreanisch lädt `theme::install_fonts` (I18N-SHELL) Microsoft YaHei, Yu Gothic
//! und Malgun Gothic als Rückfall, die Schrift der aktuellen Sprache zuerst
//! (`theme::cjk_fallback_order`); [`switch`] ruft es nach einem Wechsel neu auf.
//!
//! # Grenzen: Schrift von rechts nach links
//! egui (0.36) setzt Text nur von links nach rechts und ohne Glyphen-Formung (kein Bidi, keine
//! arabischen Verbindungsformen, keine Spiegelung des Layouts). Arabisch, Hebräisch, Persisch und
//! Urdu lassen sich deshalb zwar als Sprachdatei laden, werden aber unleserlich (getrennte
//! Buchstaben, falsche Reihenfolge) dargestellt. Solche Sprachen sind bis zu einer Textformung in
//! egui nicht unterstützt; [`is_rtl`] erkennt sie (die Sprachwahl markiert sie).

use crate::app_state::{Action, AppState};
use crate::menu::{ItemAction, Node};
use egui::{Rect, Ui};
use fp_engine::{PrefValue, Preferences};
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Voreinstellung: Sprachcode der Oberfläche (`"de"`, `"en"`, …; fehlt = Systemsprache).
pub const PREF_KEY: &str = "Language";
/// Voreinstellung: Entwickleroptionen (Testsprache `qps` im Menü auch im Release-Build).
pub const DEV_PREF_KEY: &str = "DeveloperOptions";

/// Gespeicherte Sprache (leer/fehlend = keine).
pub fn stored(prefs: &Preferences) -> Option<String> {
    let s = prefs.get_str(PREF_KEY, "");
    let s = s.trim();
    (!s.is_empty()).then(|| s.to_string())
}

/// Testsprache im Menü anbieten? Im Debug-Build immer, sonst über [`DEV_PREF_KEY`].
pub fn dev_options(prefs: &Preferences) -> bool {
    cfg!(debug_assertions) || prefs.get_bool(DEV_PREF_KEY, false)
}

/// Sprache von rechts nach links (von egui nicht unterstützt, siehe Moduldoku).
pub fn is_rtl(code: &str) -> bool {
    let base = code.split(['-', '_']).next().unwrap_or("").to_ascii_lowercase();
    matches!(base.as_str(), "ar" | "he" | "iw" | "fa" | "ur" | "ps" | "sd" | "yi" | "dv" | "ug" | "ckb")
}

/// Start: Sprachordner laden (`Languages\` neben der exe, `%APPDATA%\Firepixel\Languages`), dann
/// die Sprache wählen und in `prefs` festhalten (bleibt so beim nächsten Start):
/// * gespeicherte Sprache, falls es sie (noch) gibt;
/// * sonst beim **ersten Start** (noch keine Voreinstellungsdatei) die Systemsprache;
/// * sonst – Installation von vor der Sprachwahl – Deutsch wie bisher.
///
/// Gibt den Code zurück.
pub fn init(prefs: &mut Preferences) -> String {
    let first_start = !Preferences::default_path().is_some_and(|p| p.exists());
    init_with(prefs, first_start)
}

/// Wie [`init`], mit vorgegebenem „erster Start“ (Tests).
pub fn init_with(prefs: &mut Preferences, first_start: bool) -> String {
    let loaded = fp_i18n::load_default_dirs();
    if !loaded.is_empty() {
        log::info!("Sprachordner: {}", loaded.join(", "));
    }
    let fallback = if first_start { fp_i18n::system_language() } else { fp_i18n::SOURCE_LANG.to_string() };
    let wanted = stored(prefs).unwrap_or_else(|| fallback.clone());
    if !fp_i18n::set_language(&wanted) {
        log::warn!("Sprache „{wanted}“ nicht gefunden");
        fp_i18n::set_language(&fallback);
    }
    let now = fp_i18n::current_language();
    if stored(prefs).as_deref() != Some(now.as_str()) {
        prefs.set(PREF_KEY, PrefValue::Str(now.clone()));
    }
    now
}

/// Wählbare Sprachen: (Code, Anzeigename). Eigennamen (`language-name`), eingebaute zuerst;
/// externe ohne Namen zeigen ihren Code. Mit `dev` zusätzlich die Testsprache.
pub fn choices(dev: bool) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = fp_i18n::languages()
        .into_iter()
        .map(|l| {
            let mut name = if l.name.is_empty() { l.code.clone() } else { l.name.clone() };
            if is_rtl(&l.code) {
                name.push_str(" (RTL)");
            }
            (l.code, name)
        })
        .collect();
    if dev {
        v.push((fp_i18n::PSEUDO_LANG.to_string(), fp_i18n::t("lang-pseudo")));
    }
    v
}

/// Untermenü „Ansicht ▸ Sprache“.
pub fn menu_node(st: &AppState) -> Node {
    let cur = fp_i18n::current_language();
    let mut children: Vec<Node> = Vec::new();
    let dev = dev_options(&st.session.prefs);
    for (code, name) in choices(false) {
        let checked = cur.eq_ignore_ascii_case(&code);
        children.push(Node::Item {
            label: name,
            shortcut: None,
            action: ItemAction::Ui(Action::Run("ui.setLanguage".into(), json!({ "code": code }))),
            enabled: true,
            checked: Some(checked),
        });
    }
    if dev {
        children.push(Node::Separator);
        children.push(Node::Item {
            label: fp_i18n::t("lang-pseudo"),
            shortcut: None,
            action: ItemAction::Ui(Action::Run("ui.setLanguage".into(), json!({ "code": fp_i18n::PSEUDO_LANG }))),
            enabled: true,
            checked: Some(cur == fp_i18n::PSEUDO_LANG),
        });
    }
    children.push(Node::Separator);
    children.push(Node::Item {
        label: fp_i18n::t("lang-open-folder"),
        shortcut: None,
        action: ItemAction::Ui(Action::Run("ui.openLanguageFolder".into(), json!({}))),
        enabled: true,
        checked: None,
    });
    // Zugriffstaste (Spra&che / La&nguage) unterstreicht und bedient menu_kbd über den Rohtext
    Node::Sub { label: fp_i18n::strip_access(&fp_i18n::t("lang-menu")).0, children, enabled: true }
}

/// Sprache wechseln: sofort wirksam, in den Voreinstellungen gespeichert, Schriften angepasst.
/// `false`, wenn es die Sprache nicht gibt.
pub fn switch(st: &mut AppState, ctx: &egui::Context, code: &str) -> bool {
    let before = fp_i18n::current_language();
    if !fp_i18n::set_language(code) {
        return false;
    }
    let now = fp_i18n::current_language();
    st.session.prefs.set(PREF_KEY, PrefValue::Str(now.clone()));
    st.session.prefs_changed();
    // Reihenfolge der CJK-Rückfallschriften hängt an der Sprache (Dateien sind nur einmal gelesen)
    if crate::theme::cjk_fallback_order(&before) != crate::theme::cjk_fallback_order(&now) {
        crate::theme::install_fonts(ctx);
    }
    let name = choices(true).into_iter().find(|(c, _)| *c == now).map(|(_, n)| n).unwrap_or(now);
    st.set_status(fp_i18n::tr!("lang-changed", name = name), ctx.input(|i| i.time));
    ctx.request_repaint();
    true
}

/// Nach „OK“ in den Voreinstellungen: gespeicherte Sprache übernehmen, falls sie sich geändert hat.
pub fn apply_prefs(st: &mut AppState, ctx: &egui::Context) {
    if let Some(code) = stored(&st.session.prefs)
        && !code.eq_ignore_ascii_case(&fp_i18n::current_language())
    {
        switch(st, ctx, &code);
    }
}

/// Ordner für weitere Sprachen (wird bei Bedarf angelegt).
pub fn language_folder() -> Option<std::path::PathBuf> {
    let dir = fp_i18n::user_language_dir()?;
    let _ = std::fs::create_dir_all(&dir);
    Some(dir)
}

/// Aktionen der Sprachwahl (aus `actions::run_command`); `true` = erledigt.
pub fn run_action(st: &mut AppState, ctx: &egui::Context, name: &str, params: &Value) -> bool {
    match name {
        "ui.setLanguage" => {
            let code = params.get("code").and_then(|v| v.as_str()).unwrap_or("");
            if !switch(st, ctx, code) {
                st.set_status(format!("{}: {code}?", fp_i18n::strip_access(&fp_i18n::t("lang-menu")).0), ctx.input(|i| i.time));
            }
            true
        }
        "ui.openLanguageFolder" => {
            if let Some(dir) = language_folder() {
                // neue/geänderte Dateien gleich übernehmen
                fp_i18n::load_external_dir(&dir);
                let hint = fp_i18n::tr!("lang-folder-hint", path = dir.display().to_string());
                st.set_status(hint, ctx.input(|i| i.time));
                // Explorer nur, wenn Fremdprogramme erlaubt sind (FP_NO_SHELL_OPEN, AGENTEN_REGELN §4a)
                let _ = fp_engine::shell::shell_open(&dir, None);
            }
            true
        }
        _ => false,
    }
}

/// Zeile „Sprache:“ für die Voreinstellungen (Seite „Allgemein ▸ Benutzeroberfläche“):
/// Beschriftung in `label`, Kombifeld in `combo`. Die Auswahl landet als Änderung in `values`
/// (wirksam mit OK über [`apply_prefs`]).
pub fn pref_row(ui: &mut Ui, label: Rect, combo: Rect, values: &mut BTreeMap<String, PrefValue>, prefs: &Preferences) {
    crate::dialogs::fit_label(ui, label, &fp_i18n::t("lang-label"), true);
    let items = choices(dev_options(prefs));
    let cur = values.get(PREF_KEY).and_then(|v| v.as_str()).or_else(|| stored(prefs)).unwrap_or_else(fp_i18n::current_language);
    let names: Vec<&str> = items.iter().map(|(_, n)| n.as_str()).collect();
    let sel = items.iter().position(|(c, _)| c.eq_ignore_ascii_case(&cur));
    if let Some(i) = crate::inspector::fields::combo(ui, "pref-language", combo, &names, sel, true) {
        values.insert(PREF_KEY.into(), PrefValue::Str(items[i].0.clone()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rtl_languages() {
        assert!(is_rtl("ar") && is_rtl("he-IL") && is_rtl("fa"));
        assert!(!is_rtl("de") && !is_rtl("en") && !is_rtl("ru"));
    }

    #[test]
    fn stored_language_roundtrip() {
        let mut p = Preferences::new();
        assert_eq!(stored(&p), None);
        p.set(PREF_KEY, PrefValue::Str("en".into()));
        assert_eq!(stored(&p).as_deref(), Some("en"));
        let back = Preferences::from_json(&p.to_json()).ok().and_then(|q| stored(&q));
        assert_eq!(back.as_deref(), Some("en"));
    }
}

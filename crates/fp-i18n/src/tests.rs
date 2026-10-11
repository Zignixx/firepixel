use super::*;

// Sprachzustand ist global: Tests, die die Sprache wechseln, laufen nacheinander.
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn builtin_languages_and_fallback() {
    let _g = LOCK.lock();
    let codes: Vec<String> = languages().into_iter().map(|l| l.code).collect();
    assert!(codes.contains(&"de".to_string()) && codes.contains(&"en".to_string()), "{codes:?}");
    assert!(set_language("de"));
    assert_eq!(t("btn-cancel"), "Abbrechen");
    assert!(set_language("en"));
    assert_eq!(t("btn-cancel"), "Cancel");
    assert_eq!(t("gibt-es-nicht"), "gibt-es-nicht");
    assert!(!set_language("xx-unbekannt"));
    assert!(set_language("de"));
}

#[test]
fn placeholders_and_plurals() {
    let _g = LOCK.lock();
    set_language("de");
    assert_eq!(tr!("test-layers", n = 1), "1 Ebene");
    assert_eq!(tr!("test-layers", n = 3), "3 Ebenen");
    assert_eq!(tr!("test-unsaved", name = "a.fw.png"), "Änderungen an „a.fw.png“ speichern?");
    set_language("en");
    assert_eq!(tr!("test-layers", n = 1), "1 layer");
    assert_eq!(tr!("test-unsaved", name = "a.fw.png"), "Save changes to “a.fw.png”?");
    set_language("de");
}

#[test]
fn pseudo_language_marks_and_lengthens() {
    let _g = LOCK.lock();
    assert!(set_language(PSEUDO_LANG));
    let s = t("btn-cancel");
    assert!(s.starts_with("[!! Abbrechen") && s.ends_with("!!]") && s.chars().count() > "Abbrechen".len() + 4, "{s}");
    set_language("de");
}

#[test]
fn command_keys() {
    assert_eq!(command_key("file.open"), "cmd-file-open");
    assert_eq!(command_key("modify.align.distributeWidths"), "cmd-modify-align-distributeWidths");
}

#[test]
fn decimal_separator() {
    let _g = LOCK.lock();
    set_language("de");
    assert_eq!(fmt_decimal(12.5, 1), "12,5");
    set_language("en");
    assert_eq!(fmt_decimal(12.5, 1), "12.5");
    set_language("de");
}

#[test]
fn external_language_folder() {
    let _g = LOCK.lock();
    let dir = std::env::temp_dir().join(format!("fp-i18n-test-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join("fr.ftl"), "language-name = Français\nbtn-cancel = Annuler\n");
    let loaded = load_external_dir(&dir);
    assert_eq!(loaded, vec!["fr".to_string()]);
    assert!(languages().iter().any(|l| l.code == "fr" && l.name == "Français" && !l.builtin));
    assert!(set_language("fr"));
    assert_eq!(t("btn-cancel"), "Annuler");
    // fehlt im Französischen → Englisch
    assert_eq!(t("btn-close"), "Close");
    set_language("de");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Alle eingebauten Sprachen: keine Syntaxfehler/Doppelungen; Englisch vollständig und ohne
/// verwaiste Schlüssel (Ausgangssprache = Deutsch).
#[test]
fn builtin_files_are_complete_and_clean() {
    for l in languages().into_iter().filter(|l| l.builtin) {
        let errs = errors(&l.code);
        assert!(errs.is_empty(), "{}: {errs:#?}", l.code);
    }
    let missing = missing_keys("en");
    assert!(missing.is_empty(), "Englisch fehlt ({}): {missing:#?}", missing.len());
    let extra = extra_keys("en");
    assert!(extra.is_empty(), "Englisch verwaist ({}): {extra:#?}", extra.len());
}

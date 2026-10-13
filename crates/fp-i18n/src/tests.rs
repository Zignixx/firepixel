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

#[test]
fn numbers_follow_language_and_plural() {
    let _g = LOCK.lock();
    set_language("de");
    assert_eq!(tr!("test-zoom", z = 66.666), "Zoom 66,7 %");
    assert_eq!(tr!("test-layers", n = 1.5), "1,5 Ebenen");
    assert_eq!(tr!("test-layers", n = 1000), "1000 Ebenen");
    assert_eq!(fmt_number(2.0, 0, 2), "2");
    set_language("en");
    assert_eq!(tr!("test-zoom", z = 66.666), "Zoom 66.7%");
    assert_eq!(tr!("test-layers", n = 1.5), "1.5 layers");
    assert_eq!(tr!("test-layers", n = 1), "1 layer");
    set_language("de");
}

#[test]
fn shortcuts_units_sizes_dates() {
    let _g = LOCK.lock();
    set_language("de");
    assert_eq!(shortcut_label("Ctrl+Shift+S"), "Strg+Umschalt+S");
    assert_eq!(shortcut_label("Strg+Umschalt+S"), "Strg+Umschalt+S");
    assert_eq!(shortcut_label("Strg++"), "Strg++");
    // Tastennamen wie im Original (Befund I18N-MENUE, gemessen in allen Menüs)
    assert_eq!(shortcut_label("Entf"), "Löschen-Taste");
    assert_eq!(shortcut_label("Strg+Umschalt+Oben"), "Strg+Umschalt+Nach-oben-Taste");
    assert_eq!(shortcut_label("Strg+Links"), "Strg+Nach-links-Taste");
    assert_eq!(shortcut_label("Strg+Nach-rechts-Taste"), "Strg+Nach-rechts-Taste");
    assert_eq!(shortcut_label("Ctrl+NumAdd"), "Strg+Num +");
    assert_eq!(shortcut_label("Strg+Y, Strg+Umschalt+Z"), "Strg+Y, Strg+Umschalt+Z");
    assert_eq!(unit("px"), "px");
    assert_eq!(unit("ppi"), "Pixel/Zoll");
    assert_eq!(unit("gibtsnicht"), "gibtsnicht");
    assert_eq!(fmt_size(512), "512 Byte");
    assert_eq!(fmt_size(1536), "1,5 KB");
    assert_eq!(fmt_size(150 * 1024 * 1024), "150 MB");
    assert_eq!(fmt_date(2026, 10, 2), "02.10.2026");
    assert_eq!(fmt_datetime(2026, 10, 2, 9, 5), "02.10.2026, 09:05");
    set_language("en");
    assert_eq!(shortcut_label("Strg+Umschalt+Bild ab"), "Ctrl+Shift+Page Down");
    assert_eq!(shortcut_label("Alt+F4"), "Alt+F4");
    assert_eq!(shortcut_label("Rück"), "Backspace");
    assert_eq!(shortcut_label("Löschen-Taste"), "Del");
    assert_eq!(shortcut_label("Strg+Nach-unten-Taste"), "Ctrl+Down");
    assert_eq!(shortcut_label("Strg+Unten"), "Ctrl+Down");
    assert_eq!(unit("ppi"), "Pixels/Inch");
    assert_eq!(fmt_size(1), "1 byte");
    assert_eq!(fmt_size(1536), "1.5 KB");
    assert_eq!(fmt_date(2026, 10, 2), "10/02/2026");
    set_language("de");
}

#[test]
fn access_key_helpers() {
    let _g = LOCK.lock();
    set_language("de");
    let a = t_access("btn-yes");
    assert_eq!((a.text.as_str(), a.key, a.index), ("Ja", Some('j'), Some(0)));
    set_language("en");
    assert_eq!(t_access("btn-no").key, Some('n'));
    set_language("de");
}

/// Sprachwechsel aus einem Thread wird in allen Threads sofort sichtbar (Thread-Caches).
#[test]
fn caches_follow_language_switch_across_threads() {
    let _g = LOCK.lock();
    set_language("de");
    let g0 = generation();
    assert_eq!(t("btn-cancel"), "Abbrechen");
    let worker = std::thread::spawn(|| t("btn-cancel"));
    assert_eq!(worker.join().ok().as_deref(), Some("Abbrechen"));
    std::thread::spawn(|| assert!(set_language("en"))).join().ok();
    assert!(generation() > g0);
    assert_eq!(t("btn-cancel"), "Cancel");
    assert_eq!(&*t_arc("btn-cancel"), "Cancel");
    assert!(set_language("EN"), "Groß-/Kleinschreibung egal");
    assert_eq!(current_language(), "en");
    set_language("de");
    assert_eq!(t("btn-cancel"), "Abbrechen");
}

/// Vorübergehende Sprache nur im eigenen Thread (MCP-Brücke: englische Fehlertexte, UI bleibt).
#[test]
fn with_language_is_thread_local_and_nested() {
    let _g = LOCK.lock();
    set_language("de");
    assert_eq!(t("btn-cancel"), "Abbrechen");
    let (inner, nested, other_thread) = with_language("en", || {
        let other = std::thread::spawn(|| t("btn-cancel")).join().unwrap_or_default();
        let nested = with_language("de", || tr!("test-layers", n = 2));
        (t("btn-cancel"), nested, other)
    });
    assert_eq!(inner, "Cancel");
    assert_eq!(nested, "2 Ebenen");
    assert_eq!(other_thread, "Abbrechen", "andere Threads unberührt");
    assert_eq!(t("btn-cancel"), "Abbrechen", "danach wieder die Sprache der Oberfläche");
    assert_eq!(current_language(), "de");
    assert_eq!(with_language("EN", || tr!("test-zoom", z = 12.5)), "Zoom 12.5%");
}

/// Sprache der Oberfläche mitten in einer vorübergehenden Sprache (Verlaufsnamen über die Brücke).
#[test]
fn with_ui_language_inside_with_language() {
    let _g = LOCK.lock();
    set_language("de");
    assert_eq!(with_ui_language(|| t("btn-cancel")), "Abbrechen");
    let (agent, ui, nested) = with_language("en", || {
        let ui = with_ui_language(|| t("btn-cancel"));
        let nested = with_language("en", || with_ui_language(|| t("btn-cancel")));
        (t("btn-cancel"), ui, nested)
    });
    assert_eq!((agent.as_str(), ui.as_str(), nested.as_str()), ("Cancel", "Abbrechen", "Abbrechen"));
    assert_eq!(t("btn-cancel"), "Abbrechen");
    // Verlaufsnamen aus msg-step-* zurück in die Oberflächensprache
    let de_move = t("msg-step-move");
    let back = with_language("en", || step_label_in_ui_language(&t("msg-step-move")));
    assert_eq!(back, de_move);
    assert_eq!(with_language("en", || step_label_in_ui_language("frei")), "frei");
    assert_eq!(step_label_in_ui_language(&de_move), de_move);
}

/// Platzhalter je Schlüssel gleich, keine neuen Zugriffstasten-Konflikte, Syntax sauber.
#[test]
fn builtin_placeholders_and_access_keys() {
    let report = check::check_builtin();
    let bad: Vec<String> = report.problems.iter().filter(|p| p.kind != check::Kind::Missing && p.kind != check::Kind::Orphan).map(|p| p.to_string()).collect();
    assert!(bad.is_empty(), "{} Befunde:\n{}", bad.len(), bad.join("\n"));
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

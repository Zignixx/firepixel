//! F14 (Nutzerwunsch 14.10.2026): Die mitgelieferten Kürzelsätze heißen in der Oberfläche (DE+EN) ohne fremde
//! Produktnamen. Die interne Kennung bleibt, damit gespeicherte Einstellungen (`CurrentShortcutSet`) und Skripte
//! (`edit.keyboardShortcuts {"set": …}`) mit dem alten Namen weiter funktionieren.

const BLOCKED: &[&str] = &["photoshop", "illustrator", "freehand", "macromedia", "dreamweaver", "flash", "director"];

#[test]
fn builtin_set_labels_are_neutral() {
    for lang in ["de", "en"] {
        fp_i18n::set_language(lang);
        for name in fp_engine::shortcuts::builtin_set_names() {
            let label = fp_ui::dialogs::shortcuts::set_label(name).to_lowercase();
            assert!(!BLOCKED.iter().any(|b| label.contains(b)), "{lang}: Satz {name} heißt sichtbar {label}");
        }
    }
    fp_i18n::set_language("de");
    assert_eq!(fp_ui::dialogs::shortcuts::set_label("Illustrator"), "Vektorzeichnung");
    assert_eq!(fp_ui::dialogs::shortcuts::set_label("FreeHand"), "Klassische Vektor-Tasten");
    // Alte Kennungen bleiben lesbar (Alias über die interne Kennung)
    for old in ["Photoshop", "Illustrator", "FreeHand"] {
        assert!(fp_engine::shortcuts::builtin_set(old).is_some(), "Kennung {old} muss lesbar bleiben");
    }
    // Ein eigener Satz mit demselben Namen wird nicht umbenannt, nur mitgelieferte
    assert_eq!(fp_ui::dialogs::shortcuts::set_label("Mein Satz"), "Mein Satz");
}

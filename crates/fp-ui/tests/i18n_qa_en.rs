//! I18N-QA: Prüflauf in Englisch – Rohschlüssel und Überläufe (1024×768 und 1920×1080, 100/150 %).
//! `cargo test -j 4 -p fp-ui --test i18n_qa_en -- --ignored --test-threads 4`, Bericht: `i18n_qa_bericht`.

#[macro_use]
mod i18n_qa_common;

qa_tests!("en");

/// Bilder in dieser Sprache nach `docs\screenshots\phase6_en_*.png` (Hauptfenster je Design,
/// Menüs, Dialoge, Inspektor; Normal und Dunkel).
#[test]
#[ignore]
fn bilder() {
    i18n_qa_common::set_lang("en");
    let rep = i18n_qa_common::images_standard("phase6_en_", None);
    eprintln!("{}", rep.join("\n"));
}

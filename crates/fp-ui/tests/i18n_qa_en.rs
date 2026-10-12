//! I18N-QA: Prüflauf in Englisch – Rohschlüssel und Überläufe (1024×768 und 1920×1080, 100/150 %).
//! `cargo test -j 4 -p fp-ui --test i18n_qa_en -- --ignored --test-threads 4`, Bericht: `i18n_qa_bericht`.

#[macro_use]
mod i18n_qa_common;

qa_tests!("en");

//! I18N-QA: Prüflauf in der Testsprache `qps` – jeder Text aus `fp_i18n` ist markiert (`[!! … !!]`)
//! und um ~40 % verlängert. Unmarkierter Text = fester Text im Code; dazu Überläufe.
//! `cargo test -j 4 -p fp-ui --test i18n_qa_qps -- --ignored --test-threads 4`, Bericht: `i18n_qa_bericht`.

#[macro_use]
mod i18n_qa_common;

qa_tests!("qps");

/// Bilder in dieser Sprache nach `docs\screenshots\phase6_qps_*.png` (Hauptfenster je Design,
/// Menüs, Dialoge, Inspektor; Normal und Dunkel).
#[test]
#[ignore]
fn bilder() {
    i18n_qa_common::set_lang("qps");
    let rep = i18n_qa_common::images_standard("phase6_qps_", None);
    eprintln!("{}", rep.join("\n"));
}

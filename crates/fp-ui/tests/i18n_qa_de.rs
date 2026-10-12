//! I18N-QA: Prüflauf in Deutsch (Ausgangssprache). Prüft Rohschlüssel und Überläufe und vergleicht
//! die gezeichneten Texte jedes Bildes mit dem Stand vor Phase 6 (Basis aus Commit `30ddaec`).
//! Lang laufend, daher `#[ignore]`:
//! `cargo test -j 4 -p fp-ui --test i18n_qa_de -- --ignored --test-threads 4`
//! Basis erzeugen (im Stand vor Phase 6): `FP_QA_MODUS=basis`. Bericht: Test `i18n_qa_bericht`.

use crate::i18n_qa_common;

qa_tests!("de");

/// Bilder des Stands (Hauptfenster je Design, Menü, Dialog, Inspektor) – in der Basis nach
/// `docs\screenshots\phase6_basis_de_*.png`, sonst `phase6_de_*.png` mit Pixelvergleich.
#[test]
#[ignore]
fn bilder() {
    i18n_qa_common::set_lang("de");
    let basis = i18n_qa_common::mode() == "basis";
    let rep = i18n_qa_common::images_standard(if basis { "phase6_basis_de_" } else { "phase6_de_" }, if basis { None } else { Some("phase6_basis_de_") });
    let _ = std::fs::write(i18n_qa_common::out_dir().join("de_bilder.txt"), rep.join("\n"));
    eprintln!("{}", rep.join("\n"));
}

//! I18N-QA: check run in German (source language). Checks raw keys and overflows and compares the
//! drawn texts of every image with the state before phase 6 (baseline from commit `30ddaec`).
//! Long running, hence `#[ignore]`:
//! `cargo test -j 4 -p fp-ui --test all i18n_qa_de:: -- --ignored --test-threads 4`
//! Produce the baseline (in the state before phase 6): `FP_QA_MODE=baseline` (old `FP_QA_MODUS=basis`).
//! Report: test `i18n_qa_report`.

use crate::i18n_qa_common;

qa_tests!("de");

/// Images of the current state (main window per theme, menu, dialog, inspector) – in the baseline to
/// `docs\screenshots\phase6_baseline_de_*.png`, otherwise `phase6_de_*.png` with a pixel comparison
/// (baseline images with the former names `phase6_basis_de_*` are found as well).
#[test]
#[ignore]
fn images() {
    i18n_qa_common::set_lang("de");
    let baseline = i18n_qa_common::mode() == i18n_qa_common::MODE_BASELINE;
    let prefix = i18n_qa_common::IMAGE_PREFIX_BASELINE_DE;
    let rep = i18n_qa_common::images_standard(if baseline { prefix } else { "phase6_de_" }, if baseline { None } else { Some(prefix) });
    let _ = std::fs::write(i18n_qa_common::out_dir().join("de_images.txt"), rep.join("\n"));
    eprintln!("{}", rep.join("\n"));
}

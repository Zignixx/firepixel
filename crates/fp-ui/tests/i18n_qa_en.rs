//! I18N-QA: check run in English – raw keys and overflows (1024×768 and 1920×1080, 100/150 %).
//! `cargo test -j 4 -p fp-ui --test all i18n_qa_en:: -- --ignored --test-threads 4`, report: `i18n_qa_report`.

use crate::i18n_qa_common;

qa_tests!("en");

/// Images in this language to `docs\screenshots\phase6_en_*.png` (main window per theme,
/// menus, dialogs, inspector; normal and dark).
#[test]
#[ignore]
fn images() {
    i18n_qa_common::set_lang("en");
    let rep = i18n_qa_common::images_standard("phase6_en_", None);
    eprintln!("{}", rep.join("\n"));
}

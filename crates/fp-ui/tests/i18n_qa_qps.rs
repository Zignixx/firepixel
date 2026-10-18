//! I18N-QA: check run in the test language `qps` – every text from `fp_i18n` is marked (`[!! … !!]`)
//! and about 40 % longer. Unmarked text = text hard-coded in the source; plus overflows.
//! `cargo test -j 4 -p fp-ui --test all i18n_qa_qps:: -- --ignored --test-threads 4`, report: `i18n_qa_report`.

use crate::i18n_qa_common;

qa_tests!("qps");

/// Images in this language to `docs\screenshots\phase6_qps_*.png` (main window per theme,
/// menus, dialogs, inspector; normal and dark).
#[test]
#[ignore]
fn images() {
    i18n_qa_common::set_lang("qps");
    let rep = i18n_qa_common::images_standard("phase6_qps_", None);
    eprintln!("{}", rep.join("\n"));
}

//! I18N-QA: Prüflauf in der Testsprache `qps` – jeder Text aus `fp_i18n` ist markiert (`[!! … !!]`)
//! und um ~40 % verlängert. Unmarkierter Text = fester Text im Code; dazu Überläufe.
//! `cargo test -j 4 -p fp-ui --test i18n_qa_qps -- --ignored --test-threads 4`, Bericht: `i18n_qa_bericht`.

#[macro_use]
mod i18n_qa_common;

qa_tests!("qps");

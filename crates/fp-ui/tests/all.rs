//! All integration tests of fp-ui as **one** test program (INF-1): the crate graph with the whole
//! user interface is linked once instead of once per file.
//!
//! * every `tests\<name>.rs` is the module `<name>`; a new file is included automatically
//!   (list built by `crates\fp-testlist`), nobody keeps a list
//! * helper folders (`gesamt_common`, `i18n_qa_common`) are compiled once: `use crate::<helper>;`
//! * run: `cargo test -p fp-ui --test all`; one former file: `cargo test -p fp-ui --test all inspector_fv10::`
//! * the language applies per test thread here (`fp_i18n::set_language`), so tests switching to
//!   English do not disturb the German ones running at the same time
//! * tests that need a process of their own: `[[test]]` entries in `Cargo.toml` (reason there)

fp_testlist::fp_ui!();

//! All integration tests of fp-engine as **one** test program (INF-1): linked once instead of
//! once per file.
//!
//! * every `tests\<name>.rs` is the module `<name>`; a new file is included automatically
//!   (list built by `crates\fp-testlist`), nobody keeps a list
//! * the helper folder `common` is compiled once: `use crate::common;`
//! * run: `cargo test -p fp-engine --test all`; one former file: `cargo test -p fp-engine --test all undo::`
//! * the language applies per test thread here (`fp_i18n::set_language`)
//! * tests that need a process of their own: `[[test]]` entries in `Cargo.toml` (reason there)

fp_testlist::fp_engine!();
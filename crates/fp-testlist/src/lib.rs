//! One test program per crate (INF-1): instead of one program per `tests\*.rs` file (each linking
//! the whole crate graph), `tests\all.rs` of a crate calls the macro of that crate, e.g.
//! `fp_testlist::fp_ui!();`, which declares every test file as a module. The list is built by
//! `build.rs` from the folder itself: a new test file is part of it without anyone keeping a list.
//!
//! Using crate: `[dev-dependencies] fp-testlist = { path = "../fp-testlist", features = ["<crate>"] }`,
//! `autotests = false`, `[[test]] name = "all"` plus one `[[test]]` per file that needs a process
//! of its own. Rules for the other agents: `docs\AGENTEN_REGELN.md` §2.

include!(concat!(env!("OUT_DIR"), "/macros.rs"));

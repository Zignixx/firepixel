# Contributing to Firepixel

Thanks for helping. Firepixel reimplements a classic web, bitmap and vector graphics editor in pure Rust and
tries to match the reference application button by button. This page explains how to contribute; the
complete rule set for humans and AI agents is in [AGENTS.md](AGENTS.md).

## Ways to help

- **Report a parity gap:** Firepixel behaves differently from the reference application. Use the
  *Parity gap* issue template (unit, step, expected vs. observed, screenshots as attachments).
- **Report a bug:** crash, wrong result, broken file. Use the *Bug report* template.
- **Click-test:** work through an item in [docs/VERIFICATION.md](docs/VERIFICATION.md) — things the scripted
  runner cannot check (real keyboard, real mouse strokes, embedded panel dialogs).
- **Fix a finding:** pick an open `[F]` entry from [docs/FINDINGS.md](docs/FINDINGS.md).
- **Translate:** add a language as `.ftl` files (see `crates/fp-i18n/README.md`).

## Clean room

Firepixel is built from written behaviour descriptions and measurements of the reference application
(what it shows, which values it produces, what its files contain), never from its code.

- Do **not** submit code translated from decompiled or disassembled binaries, and do not paste decompiler
  output into issues or PRs.
- Do **not** add files from an installation of the reference application: icons, images, panels, libraries,
  patterns, textures, colour books, sample documents, help texts.
- Test documents must be created by you (or be freely licensed) and be free of personal data.
- Describing observed behaviour in your own words, with measured numbers, is welcome.

## Neutrality

The names of the reference application, its vendor and its other products must not appear anywhere in the
repository — code, comments, UI texts, docs, file names, commits, issues and PR texts. Write "the reference
application" or "the original". `cargo test -p fp-neutral` checks the UI texts and manuals.

## Language

- Commit messages, Markdown documents, issues and PRs: **English**.
- Technical names (files, folders, keys, CLI flags, MCP tools, environment variables, identifiers): **English**.
- UI texts: through Fluent, **always German and English** (`crates/fp-i18n/locales/de` and `en`).

## Workflow

1. Fork, create a branch, keep changes focused (one topic per PR).
2. Build: `cargo build -p firepixel -p firepixel-cli` (Rust 1.95+, Windows/MSVC is the primary platform).
3. Test every crate you touched (`cargo test -p <crate>`; for `fp-ui`/`fp-engine` use `--test all`), plus
   `cargo test -p fp-i18n` and `cargo test -p fp-neutral` for text changes. Set `FP_NO_SHELL_OPEN=1`.
4. `cargo clippy -p <crate>` without new warnings; `cargo fmt` on touched files.
5. Commit with an English message: a short imperative subject, a blank line, what and why. Stage explicit
   paths rather than everything.
6. Open a PR and fill in the template checklist. **No images in the diff** — attach them to the PR
   conversation instead.

## Code guidelines

- New behaviour is a command in the `fp-engine` registry (name, JSON parameters, undo step, label, shortcut);
  UI, CLI, MCP and JSF call it. See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).
- Dependencies point downwards only; nothing below `fp-ui` uses egui.
- No `unsafe`, no panics on user data, pure-Rust or statically linked dependencies only.
- Every fix gets a test that pins the measured behaviour.

## Licence

By contributing you agree that your contribution is licensed under the [Apache License 2.0](LICENSE).

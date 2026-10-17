## Summary

<!-- What does this change and why? Link the issue or the finding ID from docs/FINDINGS.md. -->

## Checklist

- [ ] Tests added or updated; the suites of all touched crates pass (`cargo test -p <crate>`, `--test all` for `fp-ui`/`fp-engine`)
- [ ] Every new or changed UI text exists in **German and English** (`crates/fp-i18n/locales/{de,en}`); `cargo test -p fp-i18n` passes
- [ ] Neutrality guard passes (`cargo test -p fp-neutral`); no name of the reference application, its vendor or its products anywhere in the diff
- [ ] Clean room: no decompiled code, no assets or files from the reference application
- [ ] Commit messages and docs are in **English**; technical names are English
- [ ] **No images** or screenshots in the diff (attach them to this PR instead)
- [ ] `cargo clippy` without new warnings in touched files
- [ ] Docs updated where a number or behaviour changed (`docs/STATUS.md`, `docs/FINDINGS.md`, manual DE+EN)

## How was it verified?

<!-- Tests, click-test steps, MCP bridge session … -->

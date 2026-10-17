# Archived working notes

Firepixel was developed with a large set of German working documents: daily plans, agent rules, hand-overs,
test guides, per-agent reports, click-test protocols, parity sweep tables, tweak lists of the comparison runs,
the full findings log (more than 800 closed entries) and thousands of screenshots. They document *how* the
project got here, not how it works, and they are written in German.

These documents are **kept outside the repository** as German working notes (in the maintainer's archive and in
the pre-reset backup of the full history). They are not translated. Everything that is still relevant has been
carried over into the English documents:

| Archived (German) | Current English document |
|---|---|
| root `STATUS.md`, plans, hand-overs, progress and night plans | [STATUS.md](STATUS.md), [ROADMAP.md](../ROADMAP.md) |
| root `ARCHITEKTUR.md`, `PLAN.md` | [ARCHITECTURE.md](ARCHITECTURE.md) |
| agent rules, technical-English and i18n review notes | [AGENTS.md](../AGENTS.md), [CONTRIBUTING.md](../CONTRIBUTING.md), `crates/fp-i18n/README.md` |
| findings log (`BEFUNDE*`, user findings) — open entries | [FINDINGS.md](FINDINGS.md) |
| deferred-verification list, click-test guides and protocols | [VERIFICATION.md](VERIFICATION.md) |
| parity sweep, parity inventories, per-unit comparison tables, tweak lists of runs 1–13, UX comparisons | [PARITY.md](PARITY.md) |
| design document | [UI_DESIGN.md](UI_DESIGN.md) |
| batch specification | [BATCH.md](BATCH.md) |
| AI model notes | [MANUAL.md](MANUAL.md) (AI selection section) |
| reports of individual agents and work packages, screenshots | — (history only) |

Source comments may still cite these notes by their old path (for example a report name next to a test); such
references are historical and do not need the file.

The exact list of removed paths is in [`_archive_list.txt`](_archive_list.txt); in addition all generated
screenshots (`docs/screenshots/`, `docs/funktionstest/`, `*_auto.md`, `phase5_*.png`) were removed — 1345 files
in total on 17 Oct 2026. Tests that write reports or screenshots create these folders again when needed; they are
ignored by `.gitignore`.

# AGENTS.md: guide for AI agents and contributors

Firepixel is a clean-room reimplementation of a classic web, bitmap and vector graphics editor in **pure Rust**
(egui/eframe on wgpu, single executable, German and English UI, CLI and MCP automation). The goal is behaviour
that matches the reference application button by button, verified by measurement, plus a few modern extras.
This file holds the rules for everyone who changes the repository — AI agents and humans alike. Read it first,
then [CONTRIBUTING.md](CONTRIBUTING.md) and [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## 1. Orientation

| Read | Why |
|---|---|
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Crate map, layers, command registry, MCP bridge, i18n, file format |
| [docs/STATUS.md](docs/STATUS.md) | Current counts, last test build, test numbers |
| [docs/PARITY.md](docs/PARITY.md) | How parity is measured and what "proven" means |
| [docs/FINDINGS.md](docs/FINDINGS.md) | Open findings — the work list to pick from |
| [docs/VERIFICATION.md](docs/VERIFICATION.md) | Checks that need a human with a real mouse and keyboard |
| [ROADMAP.md](ROADMAP.md) | What remains, grouped, and decisions already taken |
| [docs/AGENTS.en.md](docs/AGENTS.en.md) | Driving the app over MCP (tools, bridge, `ui_find`/`ui_input`, batch) |
| [docs/UI_DESIGN.md](docs/UI_DESIGN.md) | Themes, layout, icons, widget conventions |
| `crates/fp-i18n/README.md` | How UI texts and translations work |

## 2. Hard rules

1. **Clean room.** Implement from written behaviour descriptions and measurements of the reference
   application, never by translating decompiled or disassembled code. Never commit decompiler output,
   disassembly, binaries, resources, icons, images, sample files, patterns, textures or colour books of the
   reference application. All icons are drawn by us (vector icons in code). Menu and dialog labels are
   functional terms and may match.
2. **Neutrality.** The name of the reference application, its vendor and its sibling products appears
   **nowhere** in this repository: UI texts, source, comments, test names, data files, file names, Cargo
   metadata, commits, issues and docs. Write "the reference application" or "the original" in prose and
   "Firepixel" in the UI. The file format is "FW-PNG" (the extension `.fw.png` stays); the JSF objects
   `fw`/`dom` stay for API compatibility. `cargo test -p fp-neutral` must stay green.
3. **English only** in commit messages, Markdown documents, issue and PR texts. (Older source comments are
   partly German; new comments should be English.)
4. **Technical names are always English,** independent of the UI language: folders and files the app creates
   (`%APPDATA%\Firepixel\Languages`, `Models` …), preference keys, CLI commands and flags, JSON keys, MCP tool and
   parameter names, environment variables, `.ftl` file names, Fluent ids, Rust identifiers, test names.
5. **Every visible text goes through Fluent** (`crates/fp-i18n/locales/{de,en}/*.ftl`) — always add **both German
   and English**. German is the source language; `crates/fp-i18n/GLOSSAR.md` is the binding glossary.
   `cargo test -p fp-i18n` checks completeness.
6. **No screenshots or other images in the repository** outside test data and app assets. Tests that write
   screenshots (`docs/screenshots/`, `docs/funktionstest/img/`, `*_auto.md`) write into ignored folders.
   Attach images to issues or PRs instead.
7. **Everything is a command.** New behaviour is a command in the `fp-engine` registry; UI, CLI, MCP and JSF only
   call the registry. Nothing below `fp-ui` depends on egui.
8. **No `unsafe`** (workspace lint `unsafe_code = "forbid"`), no `unwrap`/`expect` on user data, damaged files
   return errors, not panics. Only pure-Rust or statically linked dependencies (single exe, no DLLs).
9. **The workspace must always load.** Never leave a `Cargo.toml` without sources or an unparsable manifest.

## 3. Working on the code

- **Small, self-contained steps** that compile at every save.
- **Tests per crate:** run the suite of every crate you touched, e.g. `cargo test -p fp-engine`,
  `cargo test -p fp-ui --test all <module>::`. `fp-ui` and `fp-engine` bundle all files under `tests/` into one
  program `all` (a new test file is picked up automatically). Corpus tests are `#[ignore]` and run with
  `-- --ignored`. Set `FP_NO_SHELL_OPEN=1` for every test run; tests never write into the real user profile
  (a guard panics if they do).
- **Text changes:** `cargo test -p fp-i18n` and `cargo test -p fp-neutral`.
- **Clippy** without warnings in the files you touched.
- **Commit with explicit path lists** (`git commit -m "…" -- path/a path/b`), never `git add -A` of a shared tree:
  several people or agents may work in the same checkout. Message: English, imperative, one subject line, then
  what and why. AI agents add a `Co-Authored-By:` trailer.
- **Agents never push, tag or publish releases.** Maintainers do.

## 4. Driving the app (MCP)

Firepixel exposes the command registry over MCP in three ways (details in
[docs/AGENTS.en.md](docs/AGENTS.en.md), also served as resource `firepixel://docs/agents.md`):

| Way | Use |
|---|---|
| `firepixel-cli mcp` (stdio) | headless automation, own session |
| `firepixel-cli mcp --http --port 47800` | headless, several clients, token on 127.0.0.1 |
| `firepixel.exe --mcp` or *Commands ▸ AI Control Channel (MCP)*, then `firepixel-cli mcp --bridge` | **bridge** into the running UI: same documents as the user, every change visible and undoable |

Core tools: `list_commands`, `describe_command`, `run_command`, `get_document`, `select`, `render`, `open`/`save`/
`export`, `undo`/`redo`/`history`, `tool_event`, `run_script`, `batch`. Bridge only: `screenshot`, `ui_state`,
`ui_action`, `ui_find`, `ui_input` (raw mouse and keyboard events, like a human), `ui_file_dialog`.
Address and token of the bridge are in `%LOCALAPPDATA%\Firepixel\mcp-bridge.json`; the token is new on every
start. Use the bridge to reproduce UI findings; use headless MCP or the CLI for everything else.

## 5. Adding a parity finding

1. Reproduce the deviation in both programs. Note the **unit** (e.g. `tool-brush`, `panel-layers`; list in
   [docs/PARITY.md](docs/PARITY.md)), the **step**, what the reference application does and what Firepixel does.
2. Open an issue with the **Parity gap** template (screenshots as attachments, never committed).
3. A maintainer adds it to [docs/FINDINGS.md](docs/FINDINGS.md) with the next free ID and a group: `[F]` fixable,
   `[V]` deferred, `[N]` not feasible / informational.
4. Non-trivial behaviour is first written down as a short behaviour description (measured values, no code of
   the original), then implemented from that description with a test that pins the measured values.
5. When fixed: reference the commit and the test in the finding, and mark it closed. If only a real window can
   prove it, add a line to [docs/VERIFICATION.md](docs/VERIFICATION.md).

## 6. Documents

Keep [docs/STATUS.md](docs/STATUS.md), [docs/FINDINGS.md](docs/FINDINGS.md) and [ROADMAP.md](ROADMAP.md) current
when your change moves a number. Prefer updating an existing document to creating a new one. Do not add
dated working notes, reports or screenshots to the repository.

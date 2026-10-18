# Firepixel

Firepixel is a **clean-room reimplementation of a classic web, bitmap and vector graphics editor**, written
in pure Rust. It aims to look, feel and behave like the reference application (menus, tools, panels,
property inspector, shortcuts and its layered `.fw.png` document format), while running as a single native
executable with no runtime to install.

- **Pure Rust**, native UI with [egui](https://github.com/emilk/egui)/eframe (wgpu), no webview, no DLLs next to the exe
- **One executable** (`firepixel.exe`, static C runtime, assets embedded) plus a headless `firepixel-cli`
- **Reads and writes FW-PNG** (`.fw.png`: pages, states, layers, symbols, live effects, slices) with round-trip fidelity
- **German and English UI** (Fluent `.ftl` files; more languages can be dropped in at runtime)
- **Automation:** one command registry drives the UI, the CLI, JSF scripts and an **MCP server** (headless or as a
  bridge into the running app), so AI agents can do everything a user can
- Extras on top of the original: modern formats (WebP, APNG, AVIF, JPEG XL, QOI, ICO, SVG), a batch studio,
  bitmap-to-vector tracing and on-device AI selection (pure-Rust CPU inference)

> Firepixel is an independent project. It is not affiliated with, endorsed by or derived from the code or
> assets of the reference application or its vendor. See [Clean room](CONTRIBUTING.md#clean-room).

## Screenshots

Screenshots are deliberately **not stored in this repository** (they would bloat the history and change with
every UI tweak). They are attached to [releases](../../releases) and to issues instead.

## Status

| | |
|---|---|
| Stage | feature-complete preview; parity polishing |
| Parity (scripted comparison, 48 units) | **42 units equal except for design**, 2 real deviations, 2 unclear, 2 under-measured (runner bugs, fixed, not yet re-run) |
| Proven 1:1 | **88 %** (42 / 48 units); canvas image pairs within 1 %: 155 / 196 |
| Tests | **2486 green, 0 red, 133 ignored** (full workspace run, 17 Oct 2026) |
| Open findings | 4 fixable, 19 deferred, 16 informational (830+ closed) — see [docs/FINDINGS.md](docs/FINDINGS.md) |

Details and method: [docs/STATUS.md](docs/STATUS.md), [docs/PARITY.md](docs/PARITY.md),
[ROADMAP.md](ROADMAP.md). The German working notes, run reports and screenshots of the development phase are
kept outside the repository ([docs/ARCHIVE_NOTE.md](docs/ARCHIVE_NOTE.md)).

## Build and run

Requirements: Rust **1.95** or newer (edition 2024). Windows is the primary platform (MSVC toolchain;
the workspace links with `rust-lld` and a static C runtime). macOS builds exist as previews; Linux x86_64 builds (tar.gz and AppImage, glibc 2.35+)
are produced by CI and are untested on real desktops so far.

```powershell
cargo build --release -p firepixel -p firepixel-cli
target\release\firepixel.exe            # the editor
target\release\firepixel-cli.exe --help  # run / batch / render / info / commands / convert / script / mcp
```

Tests (each crate has its own suite; `fp-ui` and `fp-engine` bundle their integration tests into one `all` program):

```powershell
$env:FP_NO_SHELL_OPEN = '1'
cargo test -p fp-engine
cargo test -p fp-ui --test all
cargo test -p fp-neutral        # neutrality guard, must stay green
```

## Downloads

Latest release: **v0.2.2** pre-release (18 Oct 2026) with Windows x64 ZIP, macOS universal ZIP and Linux x86_64
tar.gz and AppImage (Linux tested in CI only). There are no public binaries yet: **preview builds are available on
request** (open an issue), and releases will appear under [Releases](../../releases) once the repository is
public; CI runs on demand only
(see [docs/RELEASE.md](docs/RELEASE.md)).

## Documentation

| Document | Content |
|---|---|
| [docs/MANUAL.md](docs/MANUAL.md) | User manual (English; German: [docs/MANUAL.de.md](docs/MANUAL.de.md)) |
| [docs/STATUS.md](docs/STATUS.md) | Current state: counts, last test build, test and parity figures |
| [docs/PARITY.md](docs/PARITY.md) | How parity with the reference application is measured; per-unit table |
| [docs/FINDINGS.md](docs/FINDINGS.md) | Open findings (fixable, deferred, informational) |
| [docs/VERIFICATION.md](docs/VERIFICATION.md) | What still needs a human click-test |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Crates, layers, command registry, MCP bridge, i18n, file format |
| [docs/UI_DESIGN.md](docs/UI_DESIGN.md) | UI design rules, themes, icons |
| [docs/BATCH.md](docs/BATCH.md) | Batch studio: recipes, steps, CLI and MCP |
| [docs/AGENTS.en.md](docs/AGENTS.en.md) | Driving Firepixel over MCP (also served as an MCP resource) |
| [docs/RELEASE.md](docs/RELEASE.md) | Release and pre-release process |
| [ROADMAP.md](ROADMAP.md) | What remains and the decisions taken |
| [AGENTS.md](AGENTS.md) | Rules for AI agents and contributors working on this repository |

## Contributing

Contributions are welcome — please read [CONTRIBUTING.md](CONTRIBUTING.md) and [AGENTS.md](AGENTS.md) first
(clean-room rules, neutrality, English commits, DE+EN UI texts). Report parity deviations with the
**Parity gap** issue template. Security issues: [SECURITY.md](SECURITY.md). Conduct:
[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## License

[Apache License 2.0](LICENSE).

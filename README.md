# Firepixel Studio

Firepixel Studio (short: Firepixel) is a **clean-room reimplementation of a classic web, bitmap and vector graphics editor**, written
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
  bitmap-to-vector tracing, on-device AI selection (pure-Rust CPU inference) and a split document view with a
  page per pane (*Window ▸ Duplicate Window*)

> Firepixel is an independent project. It is not affiliated with, endorsed by or derived from the code or
> assets of the reference application or its vendor. See [Clean room](CONTRIBUTING.md#clean-room).

## Screenshots

Screenshots are deliberately **not stored in this repository** (they would bloat the history and change with
every UI tweak). They are attached to [releases](../../releases) and to issues instead.

## Status

| | |
|---|---|
| Stage | feature-complete preview; parity polishing |
| Parity (scripted comparison, 49 units) | **49 units equal except for design**, 0 real deviations, 0 unclear, 0 under-measured (follow-up run 44 on top of full run 43, 24 Oct 2026) |
| Proven 1:1 | **100 %** (49 / 49 units); measurement quota 88.0 %; canvas image pairs within 1 %: 195 / 224 |
| Tests | **3445 passed** (full workspace run on the code of v0.2.16, 25 Oct 2026, GT-2c); the run on the tree with tester feedback batch 10 is 3489 passed, 0 real failures (one wgpu render flake green on rerun), 142 ignored (GT-2d, `df412989`); the run on the tree with tester feedback batch 11 is 3499 passed, 0 failed, 142 ignored (GT-2f, `43fe3e6b`); the run on the tree with batch 12 and the night round is pending (GT-2g); batch and export tests also run on macOS and Linux in CI |
| Open findings | 107 fixable rows (mostly fixed or done and waiting for the click-test or the macOS re-test), 5 deferred items, 53 informational rows (850+ closed) — see [docs/FINDINGS.md](docs/FINDINGS.md) |

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

Latest release: **v0.2.18** pre-release (26 Oct 2026) with Windows x64 ZIP, macOS universal ZIP and Linux x86_64
tar.gz and AppImage (Linux tested in CI only), under [Releases](../../releases). Preview builds between releases
are available on request (open an issue). CI runs on demand only and tests on Windows, macOS and Linux (the batch
and export crates on all three, the UI on Windows; see [docs/RELEASE.md](docs/RELEASE.md)).

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

## Acknowledgements

Thanks to the people who contributed testing feedback:

- **E-Man** – hands-on testing feedback on the pen tool, Select Behind, macOS and the Subselection display, the Pen undo steps, redo, the inspector sliders and the Subselection snapping, and batch 9: the bitmap mode button, painting into bitmaps, keys during a brush stroke, page multi-selection and moving pixel selections, and batch 10: the Layers mask rows, the mask inspector, the idea of a separate mask selection tool, Ctrl-drag, Shift-marquee and Shift + wheel options, the dither slider, screentone anti-alias, Undo of bitmap mode and dragging several pages, and batch 11: the invisible Exit Bitmap Mode button, the History markers and moving pixels with the arrow keys, and batch 12: the split view with a second page, Ctrl for a new bitmap and the tooltip options (October 2026)
- **Hunter** – for the animation playback and brush layer reports (October 2026)

Code contributions:

- **jeremiahsherrill** – Properties panel height, live font preview, point editing option (PR #3)

## License

[Apache License 2.0](LICENSE).

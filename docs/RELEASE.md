# Releases and CI

## Workflows (`.github\workflows`)

| File | Trigger | Content |
|---|---|---|
| `windows-ci.yml` | manual only (`workflow_dispatch`) or called from `prerelease.yml` (`workflow_call`, inputs `ref` and `package`); push and PR trigger nothing | four jobs in parallel: *cargo check (warning-free)*, *Tests (without fp-ui)*, *Tests fp-ui (kittest)*, *Release build (x64)* as the artifact `Firepixel-Windows-x64`. Called with `package: true` only the release build runs and uploads the ZIP as `release-windows` |
| `macos-preview.yml` | manual only (input `tests`, default off) or called from `prerelease.yml` (input `ref`) | job *Firepixel.app (universal)*: `Firepixel.app` as a universal binary (arm64 + x86_64, macOS 10.15+, merged with `lipo` and verified), ad-hoc signed, ZIP with universal CLI, manuals, licence (artifact `Firepixel-macOS-universal`), plus a 20 s startup probe (arm64 slice only, the runner is Apple Silicon); optional job *Tests (platform-neutral crates)* |
| `linux-build.yml` | manual only (input `tests`, default off) or called from `prerelease.yml` (inputs `ref` and `package`) | job *Linux x86_64 (tar.gz + AppImage)* on `ubuntu-22.04` (glibc 2.35 baseline): `Firepixel-Linux-x64-X.Y.Z.tar.gz` (`firepixel`, `firepixel-cli`, manuals, licence, `share/applications/firepixel.desktop`, hicolor icons) and `Firepixel-x86_64.AppImage` (packed with a pinned, checksum-verified `appimagetool`), artifact `Firepixel-Linux-x64`; plus a startup probe under Xvfb (software rendering) that checks that `firepixel --mcp` writes `mcp-bridge.json` (artifact `startprobe-linux`, may fail without failing the run); optional job *Tests (platform-neutral crates)* |
| `prerelease.yml` | manual only (input `version`, optional) | jobs *Set version*, *Windows x64*, *macOS universal*, *Linux x86_64*, *Tag and pre-release* |

## Triggering a pre-release

Locally in the repository:

```powershell
.\tools\release\prerelease.ps1              # next version automatically (minor + 1)
.\tools\release\prerelease.ps1 -Version 0.5.0
```

The script starts `gh workflow run prerelease.yml`, waits for it to finish and prints the release
URL. Without the script:

```powershell
gh workflow run prerelease.yml --ref main                  # next version automatically
gh workflow run prerelease.yml --ref main -f version=0.5.0 # explicit version
gh run watch
```

Alternatively on GitHub: *Actions ▸ prerelease ▸ Run workflow* (field *Version*, empty = automatic).

Steps in the workflow:

1. **Set version**: highest tag `v0.*.0`, otherwise `[workspace.package] version` from `Cargo.toml`;
   minor + 1 (first run: `0.1.0` → `v0.2.0`). An explicitly entered version takes precedence.
2. Set the version in `Cargo.toml`/`Cargo.lock`, bot commit `Version X.Y.0 [skip ci]` on `main`.
3. In parallel: Windows x64 (`Firepixel-Windows-x64-X.Y.0.zip`: `firepixel.exe`, `firepixel-cli.exe`,
   `LICENSE`, `docs\MANUAL.md`, `docs\MANUAL.de.md`) and macOS universal
   (`Firepixel-macOS-universal-X.Y.0.zip`: `Firepixel.app`, `firepixel-cli`, `MANUAL.md`,
   `MANUAL.de.md`, `LICENSE`) and Linux x86_64 (`Firepixel-Linux-x64-X.Y.0.tar.gz` and
   `Firepixel-x86_64.AppImage`).
4. **Tag and pre-release**: tag `vX.Y.0` on the version commit, **pre-release** titled
   `Firepixel X.Y.0 (pre-release)` with both ZIPs, the tar.gz and the AppImage. The English notes
   contain install hints for Windows, macOS and Linux and a changelog: the commits since the previous tag (at most 100, version bump
   commits left out) if that tag is part of the current history, otherwise
   "Initial public pre-release".

## Versioning

- Previews are released as `0.Y.0`; every run increases only the minor part.
- **`v1.0` happens only at the user's explicit request** (input `version`; the script asks for
  confirmation for 1.x and continues only on `yes`). Without input it stays at `0.*`.
- An already existing tag aborts the run before building.

## Cost (GitHub Actions minutes, private repository)

| Run | Duration (approx.) | Factor | Quota |
|---|---|---|---|
| `windows-ci` per run (4 parallel jobs, warm cache) | 35–40 min wall time, 110 job minutes | ×2 | approx. 220 min |
| `windows-ci` with cold cache | approx. 50 min wall time, 166 job minutes | ×2 | approx. 330 min |
| `macos-preview` (build, without tests) | 10–14 min | **×10** | approx. 100–140 min |
| `macos-preview` with tests | +12 min | ×10 | +120 min |
| `linux-build` (build + packages + probe, estimate, not measured yet) | 12–20 min | ×1 | approx. 12–20 min |
| `prerelease` total (version job + Windows build + macOS build + Linux build) | 25–35 min wall time | – | approx. 150–200 min |

For this reason CI no longer runs automatically on push (previously approx. 220 min per push). Start
the Windows CI manually when needed:

```powershell
gh workflow run windows-ci.yml --ref main
gh run watch
```

Alternatively on GitHub: *Actions ▸ windows-ci ▸ Run workflow*. `prerelease.yml` still calls the
Windows CI itself; macOS and Linux run only manually or via `prerelease.yml`
(`gh workflow run linux-build.yml --ref main [-f tests=true]`).

## Notes

- macOS: not notarized – first launch via right-click ▸ Open, or
  `xattr -dr com.apple.quarantine Firepixel.app`.
- The Windows runner has no real GPU; the kittest tests run on wgpu's software renderer.
- Linux: built on Ubuntu 22.04, so the binaries need glibc 2.35+. No OpenSSL: the AI model
  download uses rustls on non-Windows targets (`fp-ai/Cargo.toml`; Windows keeps SChannel), and the
  inspect step fails if `ldd` lists `libssl`/`libcrypto`. X11, Wayland, xkbcommon, Vulkan and EGL/GL are loaded at runtime; a
  Vulkan or OpenGL driver is required. File dialogs use the XDG desktop portal (no GTK). The
  AppImage bundles no libraries; it needs FUSE 2 to mount, otherwise run it with
  `--appimage-extract-and-run`. `fp-ui/Cargo.toml` enables the X11/Wayland backends of eframe/winit
  and the portal backend of rfd for `target_os = "linux"` only.

## Tests excluded in CI

The runner lacks files that are installed locally (AI models, resource folder with textures and
patterns, the font "Myriad Pro"). These tests run only locally (`--exact --skip` in
`windows-ci.yml`):

| Crate | Test | Reason |
|---|---|---|
| fp-engine | `all_commands::all_commands_work` | AI models missing (select.aiObject/aiSubject/aiRefineEdge) |
| fp-engine | `all_commands::repeated_commands_replace_instead_of_append` | AI model missing (select.aiObject) |
| fp-engine | `fixf::repeated_fill_same_kind_adds_no_step` | pattern "Cotton" from the resource folder missing |
| fp-script | `default_values_like_original`, `face_is_postscript_name_for_family_names`, `text_runs_to_source_has_original_shape` (tw02_textruns) | font "Myriad Pro" missing (Arial substituted) |
| fp-script | `text_pixel_rect_is_ink` (tw_f4) | font "Myriad Pro" missing (different text width) |
| fp-ui | `fixd_ct2::texture_list_as_wide_as_longest_name`, `nz1::w21_texture_list_as_wide_as_longest_name` | texture list empty (no resource folder) |
| fp-ui | `inspector_fv15::hg_ai_object_hint_names_exit_bitmap_mode` | without an AI model the inspector shows "Model missing" instead of the hint |

# Releases and CI

## Workflows (`.github\workflows`)

| File | Trigger | Content |
|---|---|---|
| `windows-ci.yml` | manual only (`workflow_dispatch`) or called from `prerelease.yml` (`workflow_call`); push and PR trigger nothing | `cargo check --workspace --all-targets` (warning-free), tests without fp-ui, fp-ui tests (kittest), release build as an artifact – four jobs in parallel |
| `macos-preview.yml` | manual only (input `tests`, default off) or called from `prerelease.yml` | `Firepixel.app` (Apple Silicon), ad-hoc signed, ZIP with CLI, manuals, licence |
| `prerelease.yml` | manual only | bump version, build Windows + macOS, tag and pre-release |

## Triggering a pre-release

Locally in the repository:

```powershell
.\tools\release\prerelease.ps1              # next version automatically (minor + 1)
.\tools\release\prerelease.ps1 -Version 0.5.0
```

The script starts `gh workflow run prerelease.yml`, waits for it to finish and prints the release
URL. Alternatively on GitHub: *Actions ▸ prerelease ▸ Run workflow*.

Steps in the workflow:

1. **Version**: highest tag `v0.*.0`, otherwise `[workspace.package] version` from `Cargo.toml`;
   minor + 1 (first run: `0.1.0` → `v0.2.0`). An explicitly entered version takes precedence.
2. Set the version in `Cargo.toml`/`Cargo.lock`, bot commit `Version X.Y.0 [skip ci]` on `main`.
3. In parallel: Windows x64 (`Firepixel-Windows-x64-X.Y.0.zip`: `firepixel.exe`, `firepixel-cli.exe`,
   `LICENSE`, `docs\MANUAL.md`, `docs\MANUAL.en.md`) and macOS arm64
   (`Firepixel-macOS-arm64-X.Y.0.zip`: `Firepixel.app`, `firepixel-cli`, manuals, `LICENSE`).
4. Tag `vX.Y.0` on the version commit, **pre-release** with both ZIPs and short German notes
   (commits since the last tag, at most 60).

## Versioning

- Previews are released as `0.Y.0`; every run increases only the minor part.
- **`v1.0` happens only at the user's explicit request** (input `version`; the script asks for
  confirmation for 1.x). Without input it stays at `0.*`.
- An already existing tag aborts the run before building.

## Cost (GitHub Actions minutes, private repository)

| Run | Duration (approx.) | Factor | Quota |
|---|---|---|---|
| `windows-ci` per run (4 parallel jobs, warm cache) | 35–40 min wall time, 110 job minutes | ×2 | approx. 220 min |
| `windows-ci` with cold cache | approx. 50 min wall time, 166 job minutes | ×2 | approx. 330 min |
| `macos-preview` (build, without tests) | 10–14 min | **×10** | approx. 100–140 min |
| `macos-preview` with tests | +12 min | ×10 | +120 min |
| `prerelease` total (Linux + Windows build + macOS build) | 25–35 min wall time | – | approx. 150–200 min |

For this reason CI no longer runs automatically on push (previously approx. 220 min per push). Start
the Windows CI manually when needed:

```powershell
gh workflow run windows-ci.yml --ref main
gh run watch
```

Alternatively on GitHub: *Actions ▸ windows-ci ▸ Run workflow*. `prerelease.yml` still calls the
Windows CI itself; macOS runs only manually or via `prerelease.yml`.

## Notes

- macOS: not notarized – first launch via right-click ▸ Open, or
  `xattr -dr com.apple.quarantine Firepixel.app`.
- The Windows runner has no real GPU; the kittest tests run on wgpu's software renderer.

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

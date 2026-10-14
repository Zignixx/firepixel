# Releases und CI

## Workflows (`.github\workflows`)

| Datei | Ausloeser | Inhalt |
|---|---|---|
| `windows-ci.yml` | Push/PR auf `main` (nicht bei reinen Doku-Aenderungen), manuell, Aufruf aus `prerelease.yml` | `cargo check --workspace --all-targets` (warnungsfrei), Tests ohne fp-ui, Tests fp-ui (kittest), Release-Build als Artifact – vier Jobs parallel |
| `macos-preview.yml` | nur manuell (Eingabe `tests`, Standard aus) oder Aufruf aus `prerelease.yml` | `Firepixel.app` (Apple Silicon), ad-hoc signiert, ZIP mit CLI, Handbuechern, Lizenz |
| `prerelease.yml` | nur manuell | Version hochzaehlen, Windows + macOS bauen, Tag und Pre-Release |

## Pre-Release ausloesen

Lokal im Repo:

```powershell
.\tools\release\prerelease.ps1              # naechste Version automatisch (Minor + 1)
.\tools\release\prerelease.ps1 -Version 0.5.0
```

Das Skript startet `gh workflow run prerelease.yml`, wartet auf das Ende und gibt die Release-URL
aus. Alternativ auf GitHub: *Actions ▸ prerelease ▸ Run workflow*.

Ablauf im Workflow:

1. **Version**: hoechster Tag `v0.*.0`, sonst `[workspace.package] version` aus `Cargo.toml`;
   Minor + 1 (erste Ausfuehrung: `0.1.0` → `v0.2.0`). Eine eingegebene Version hat Vorrang.
2. Version in `Cargo.toml`/`Cargo.lock` setzen, Bot-Commit `Version X.Y.0 [skip ci]` auf `main`.
3. Parallel: Windows x64 (`Firepixel-Windows-x64-X.Y.0.zip`: `firepixel.exe`, `firepixel-cli.exe`,
   `LICENSE`, `docs\MANUAL.md`, `docs\MANUAL.en.md`) und macOS arm64
   (`Firepixel-macOS-arm64-X.Y.0.zip`: `Firepixel.app`, `firepixel-cli`, Handbuecher, `LICENSE`).
4. Tag `vX.Y.0` auf dem Versions-Commit, **Pre-Release** mit beiden ZIPs und kurzen deutschen
   Notizen (Commits seit dem letzten Tag, hoechstens 60).

## Versionierung

- Vorschauen laufen als `0.Y.0`; jede Ausfuehrung erhoeht nur den Minor-Teil.
- **`v1.0` gibt es nur auf ausdruecklichen Wunsch des Nutzers** (Eingabe `version`, das Skript fragt
  bei 1.x nach). Ohne Eingabe bleibt es bei `0.*`.
- Ein bereits vorhandener Tag bricht den Lauf vor dem Bauen ab.

## Kosten (GitHub-Actions-Minuten, privates Repo)

| Lauf | Dauer (grob) | Faktor | Kontingent |
|---|---|---|---|
| `windows-ci` je Push (4 Jobs, mit Cache) | 4 × 10–25 min | ×2 | ca. 80–160 min |
| `macos-preview` (Build, ohne Tests) | 10–14 min | **×10** | ca. 100–140 min |
| `macos-preview` mit Tests | +12 min | ×10 | +120 min |
| `prerelease` gesamt (Linux + Windows-Build + macOS-Build) | 25–35 min Wandzeit | – | ca. 150–200 min |

Deshalb laeuft macOS nie automatisch beim Push. Viele kleine Pushes hintereinander lassen sich
buendeln; reine Doku-Commits (`docs/**`, `*.md`) loesen keine Windows-CI aus.

## Hinweise

- macOS: nicht notarisiert – erster Start per Rechtsklick ▸ Oeffnen oder
  `xattr -dr com.apple.quarantine Firepixel.app`.
- Auf dem Windows-Runner gibt es keine echte GPU; die kittest-Tests laufen ueber den
  Software-Renderer von wgpu. Ausgeklammerte Tests stehen in `docs\berichte\ci-rel.md`.

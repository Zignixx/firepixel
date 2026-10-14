# Bericht CI-MAC – macOS-Vorschau über GitHub Actions

Worktree `fp-mac`, Branch `ci/macos` (nicht `main`), Prüf-Zielordner `target-mac`.

## Ergebnis

- Workflow `.github/workflows/macos-preview.yml` baut auf `macos-14` (Apple Silicon) eine
  `Firepixel.app` (arm64) mit CLI daneben, Handbüchern, Lizenz, Icon (`.icns` aus `assets\icon\png`),
  Info.plist (Name Firepixel, Bundle-ID `app.firepixel`, Version aus Cargo), ad-hoc-Signatur und lädt
  sie als ZIP-Artifact hoch. Startprobe: App 20 s gestartet, Bildschirmfoto als Artifact `startprobe`.
- Läufe: 38650659193 (Paket grün, Tests 5 rot), 38652290752 (Paket + Startprobe grün; Tests per
  `[skip-tests]` übersprungen). Die App startet auf dem Runner (Metal), Startfenster vollständig.
- Ablage: `Desktop\Firepixel-macOS-Vorschau-2026-10-15_01-19\` (ZIP, LIESMICH.md, Bildschirmfoto).

## Code-Änderungen (nur macOS wirksam; Windows unverändert)

| Datei | Änderung |
|---|---|
| `crates\fp-i18n\src\profile.rs` | Profil unter macOS `~/Library/Application Support/Firepixel` (lokaler Teil `…/Local`) statt XDG-Ordner; `mac_user_fonts_dir()` (`~/Library/Fonts`, nur lesen) |
| `crates\fp-text\src\font\mod.rs` | Schriftordner unter macOS: `/System/Library/Fonts`, `…/Supplemental`, `/Library/Fonts`, `~/Library/Fonts` |
| `crates\fp-io\src\vector.rs` | PDF-Standardschriften: macOS-Dateinamen (Arial, Courier New, Times New Roman, Symbol, Wingdings) und Ordner |
| `crates\fp-ui\src\theme.rs` | Oberflächenschriften unter macOS (Arial/Arial Bold, Apple Symbols, Hiragino, Apple SD Gothic Neo) |
| `crates\fp-engine\src\shell.rs` | Dateien/Ordner unter macOS mit `open` statt `xdg-open` öffnen |
| `crates\fp-engine\src\commands\misc.rs` | Anleitung auch in `Contents/Resources` des App-Pakets suchen |
| `.github\workflows\macos-preview.yml` | neuer Workflow (Paket-Job + Test-Job, Cache über `Swatinem/rust-cache`) |

Prüfungen: `cargo check --workspace --all-targets` (Windows) und mit `--target aarch64-apple-darwin`
(C-Teile über einen Platzhalter-Compiler, nur zum Prüfen) ohne Warnungen; `cargo test -p fp-neutral`
grün (der Benutzerschriftordner kommt deshalb aus `fp_i18n::profile`, nicht direkt aus `HOME`).

Vorher schon sauber getrennt (keine Änderung nötig): DX12-Vorauswahl nur `cfg(windows)` (macOS nutzt
wgpu/Metal), rfd-Dateidialoge (AppKit), ffmpeg-Suche (`ffmpeg` ohne `.exe`), Konsolenfenster,
Taskleisten-Icon, Windows-Ressourcen in `build.rs`.

## Tests unter macOS (fp-model, fp-geom, fp-raster, fp-vector, fp-script)

43 Testprogramme grün, 5 Tests rot – alle schrift- bzw. textmaßabhängig (andere Systemschriften):

- `fp-script --test tw02_textruns`: `face_is_postscript_name_for_family_names`,
  `default_values_like_original`, `text_runs_to_source_has_original_shape` (Schriftname „Arial“ statt
  der erwarteten Vorgabe „MyriadPro-Regular“)
- `fp-script --test tw_f4`: `text_pixel_rect_is_ink` (Textbreite 113 statt 107)
- `fp-script --test b7_messwerte`: `ba020_anim_instance_bounds_include_motion_path` (Grenzen abweichend)

## Bekannte Einschränkungen

Nur arm64 (kein Universal, spart Minuten); nicht notarisiert (erster Start: Rechtsklick ▸ Öffnen bzw.
`xattr -dr com.apple.quarantine Firepixel.app`); eigene Titelleiste mit Fensterknöpfen im
Windows-Stil; Kürzel teils Strg statt ⌘; Scannen nur Windows; Datei-Dialoge, Video (ffmpeg), KI und
Zwischenablage auf macOS nicht interaktiv geprüft.

## Verbrauch

Lauf 1: 13:44 + 12:40 min, Lauf 2: 9:53 min ≈ 36 Runner-Minuten (macOS-Faktor 10 ≈ 360 Minuten
Kontingent). Release-Profil im Workflow mit `CODEGEN_UNITS=16` (thin LTO bleibt).

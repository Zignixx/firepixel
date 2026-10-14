# GT-0g – Release-Build und neue Testversion (14.10.2026, ≈ 15:15–15:40)

## Stand

- `main` @ **`821278b`** (F6), sauberer Worktree `C:\Users\Administrator\Desktop\REA\fp-rel`
  (`checkout -q --detach main`, `status --short` leer), Zielordner `fp-rel\target-rel`, `CARGO_INCREMENTAL=0`, `-j 8`.
- Neu seit GT-0f (`c4388f4`): F1–F7, BATCH-T (`15c2c46`), NP-1-Befunde (über F3/F4/F5), RUN-4, M1.
- Kein `cargo test --workspace` (Auftrag: voller Testlauf erst bei GT-2).

## Release-Build

- `cargo build --release -p firepixel -p firepixel-cli -j 8`: grün, 7 min 42 s.
- `firepixel.exe` 77,7 MB, `firepixel-cli.exe` 53,7 MB.

## Testversion

`C:\Users\Administrator\Desktop\Firepixel-Vorschau-2026-10-14_15-33\` mit `firepixel.exe`, `firepixel-cli.exe`
(bytegleich mit `target-rel\release`), `docs\MANUAL.md`, `docs\MANUAL.en.md`, `NEU.md` und `NEW.md` (englisch,
gleicher Inhalt, Beschriftungen aus `locales\en`). „Neu seit Vorschau 2026-10-14_00-22“ aus f1–f7, batcht, np1:
Pipette (N27), sichtbare KI-Motivauswahl mit Feineinstellungen (N21), Batch-Fixes und -Reste, Neu von Vorlage,
Web-Dialoge (HTML-Code kopieren, Popupmenü-Editor, HTML-Einrichtung), Import PSD/AI/SVG, Gamma/Schaltflächen/
Animationspfad/Esc, kleinere Angleichungen (TW-16…31, Q-14…16), Nachzeichnen-Hinweis, Skript-Ebenen.
15 Ausprobier-Schritte; bekannte Lücken aktualisiert (Pipette außerhalb des Fensters, Konvertieren vor dem
Filterdialog, Innen einfügen als Maskengruppe, Gamma ohne Schachbrett; Raster-bearbeiten-Hinweis entfallen).

Neutralität: keine Namen des Originals/Herstellers in Ordner, NEU/NEW, Handbüchern. In beiden exe kommt der
Herstellername nur als Formatkennung fremder Bibliotheken vor (XMP-Namensraum, JPEG-APP14/Farbraum-Meldung des
Decoders, Brotli-Wörterbuch) – dieselben Fundstellen wie in der Vorversion 00-22; der Programmname des Originals
kommt nicht vor.

## Smoke-Test (echtes Fenster, Brücke)

- Eigenes Profil je Lauf im Scratchpad (`gt0g\home-de`, `home-en`, `home-t1…t4`: APPDATA/LOCALAPPDATA,
  `FP_NO_SHELL_OPEN=1`), `app.ron` 900 × 650 unten rechts, `--mcp`, kein SetForegroundWindow, Ende per
  `WM_CLOSE` → Exit 0 (alle 6 Läufe). DX12 direkt.
- **Deutsch:** `ui_state` ok (`mode: bridge`, keine Dokumente, neue Felder `batch_front`/`panels_hidden` da,
  Statuszeile „KI-Steuerkanal aktiv …“); `ui_find` Menüleiste „Befehle“, „Filter“, „Fenster“, „Hilfe“,
  „Erweiterter Modus“.
- **Englisch:** Statuszeile „AI control channel active …“; Menüleiste „File, Edit, View, Select, Modify, Text,
  Commands, Filters, Window, Help“, „Expanded Mode“; Bildschirmfoto 900 × 650: Startseite, Bedienfelder,
  Werkzeugleiste englisch, nichts abgeschnitten.
- `firepixel-cli batch --templates --json`: **12** Vorlagen, Exit 0. `--lang de batch --help` → „Aufruf:
  firepixel-cli batch [OPTIONEN] [DATEIEN]...“, `--lang en` → „Usage: firepixel-cli batch [OPTIONS] [FILES]...“, Exit 0.

## Startzeit (`FP_STARTUP_TRACE`, Erststart mit neuem Profil)

| Marke | t1 (ms) | t2 (ms) | t3 (ms) | **Median t1–t3** | t4 (ms) |
|---|---|---|---|---|---|
| `wgpu: DX12 setup` | 56 | 60 | 73 | 60 | 63 |
| `eframe: window + graphics ready (App::new)` | 95 | 104 | 118 | 104 | 107 |
| `frame 1 begin` | 105 | 114 | 140 | 114 | 116 |
| `window shown (offscreen, settling)` | 256 | 290 | 296 | 290 | 283 |
| **`window placed (visible to the user)`** | 772 | 783 | 793 | **783** | 795 |
| **`first frame after placement (interactive)`** | 800 | 816 | 822 | **816** | 799 |

t1/t2 ruhig; vor t3 und t4 liefen 2 fremde `cargo`/`rustc`-Prozesse (andere Agenten). Lauf DE (ohne
Zeitwertung, mit Brücke) 887/919 ms. Damit unverändert gegenüber GT-0f (768/814 ms).

## Befunde

- Keine neuen Fehler im Smoke-Test.
- (Hinweis, unverändert) In der Oberfläche stehen weiterhin Produktnamen eines Fremdprogramms im Zusammenhang
  mit PSD (Voreinstellungen „…-Datei importieren“, „…-Zusatzmodule“, Effekte „…-Live-Effekte (noch nicht
  verfügbar)“, Exportstil „… PSD“; schon in f5.md genannt) – Neutralitätsregel prüfen.

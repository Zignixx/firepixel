# GT-2 – Letzter Gesamtlauf, Endversion (14.10.2026, ≈ 18:35–19:35)

## Stand

- `main` @ **`08a914b`**, sauberer Worktree `C:\Users\Administrator\Desktop\REA\fp-rel` (`checkout -q --detach main`,
  `status --short` leer), Zielordner `fp-rel\target-rel`, `CARGO_INCREMENTAL=0`, `-j 8`. Kein Git-Tag, nicht gepusht.
- Neu seit GT-0g (`821278b`): F8–F14, M2, A3-3, DOC-5 und die zwei GT-2-Commits:
  - `93f19d5` Doku-Nachtrag: Handbuch und AGENTS DE/EN um F9–F14 ergänzt (Einfügen über der Auswahl, Maske gruppieren
    und Pfadoperationen am richtigen Platz, Segment-Hilfslinien, Symbolansicht mit Pfadleiste, Popupmenü-Tabelle, AI-Text,
    neutrale Kürzelsatz-Namen, Auswahlrahmen in Pfad mit Werkzeugfarben, Tweenen 10, Rückfrage „Status hinzufügen“/
    `addStates`, Auswahlverlauf Strg+Umschalt+Pfeile); `parity\TWEAKS_2026-10-14.md` neuer Abschnitt „Stand TW-33…41
    und Q-18…22“; STATUS/PLAN „14.10. abends, Endversion gebaut, Tag ausstehend“.
  - `08a914b` Fix (siehe Tests): zwei fehlende Parametertexte in `command_texts.ftl` DE/EN.

## Tests (`cargo test --workspace --no-fail-fast -j 8`, Debug, 17 min 36 s, Stand `93f19d5`)

**2040 bestanden, 2 rot, 127 ignoriert.**

| Test | Befund | Ergebnis |
|---|---|---|
| `firepixel-cli` `tests::parameterbeschreibungen_in_de_und_en` | Echter Fehler aus F13: die neuen Parameter `name` und `addStates` von `modify.animation.animateSelection` hatten keinen Schlüssel in `command_texts.ftl`. | **Behoben** in `08a914b` (2 Zeilen je Sprache). In fp-rel auf `08a914b`: grün (3/3); `fp-i18n`, `fp-mcp` grün. |
| `fp-ui --test all` `dialogs_ux::width_field_limited_to_6000_with_message` | Bekannt wackelnd (wie GT-0f): Statuszeile im Gesamtlauf überschrieben. | **Einzeln grün** (0,24 s). Nicht geändert. |

Der Testlauf schrieb wie immer nachverfolgte Dateien unter `docs\funktionstest\` und `docs\screenshots\` in fp-rel neu;
mit `git checkout -- docs` zurückgesetzt. Neutralitätswächter `fp-neutral` (auch Handbuch/AGENTS) grün.

## Golden-Korpus (Release, `--include-ignored`, Korpusdaten aus `tools\golden\*\out` nach fp-rel kopiert)

| Crate / Test | Ergebnis |
|---|---|
| `fp-geom` `pcl_corpus` | **28.095 / 28.099** Konturen punktgenau, BZCi 64/64, **28.099 / 28.099** bitgleich zur Referenz – unverändert |
| `fp-raster` `golden` | **206 Fälle, 1.017.138 / 1.017.138** Pixel gleich |
| `fp-raster` `corpus` | 8/9 grün (Farbton/Sättigung, Kurven, Farbfüllung, T1 r = 3,5 1.209/1.209, T2 Rauschen, T3 Schärfen je 4.096/4.096, T5, T6 r = 210/201, T7 `t7g_gui` 10.000/10.000). **Rot: `t4_zoom`** 147.751/168.000 – veralteter Korpustest: FX-3 (`73c3379`) lässt Zoom-Weichzeichnen bewusst wie das Original wachsen (Golden fx3 und `fp-raster` golden neu und grün), der alte Korpustest vergleicht noch mit den Eingabegrenzen. Nicht geändert (kein kleiner sicherer Fix, nur Testanpassung nötig). |
| `fp-vector` `corpus`, `compound_corpus`, `testfiles`, `render_golden`, `pathops_golden` | 12/12 grün. Pfadkorpus **1.687 / 1.719** pixelgenau, 1.719/1.719 bitgleich zur Python-Referenz (`f32` 1.691); Texturen 34/34; Compound 21/21 erklärt; T5 30.800/30.800, T7 8.192/8.192 |
| `fp-text` `corpus`, `testfiles`, `onpath_fit`, `golden` | 8/8 grün. Rahmen **4.002 / 4.147** (= Python), Glyphenpixel **95,19 %** exakt (Python 91,91 %), Rahmentext-Unterkante 305/676 (±1 px 570/676) – unverändert zu F6 |

## Release-Build und Endversion

- `cargo build --release -p firepixel -p firepixel-cli -j 8`: grün, 7 min 31 s.
- `firepixel.exe` 77,8 MB (SHA-256 `7F990C65…9276CCDC8`), `firepixel-cli.exe` 53,7 MB (SHA-256 `14A786B1…030A43E5B5`).
- Ordner **`C:\Users\Administrator\Desktop\Firepixel-2026-10-14_19-17\`**: beide exe (bytegleich mit `target-rel\release`),
  `LICENSE`, `docs\MANUAL.md`, `docs\MANUAL.en.md`, `NEU.md`, `NEW.md` (englisch, gleicher Inhalt). NEU/NEW: Überblick der
  Hauptfunktionen (9 Bereiche), „Neu seit Vorschau 2026-10-14_15-33“ (F8–F14), 20 Ausprobier-Schritte, bekannte Lücken
  (Symbolansicht intern eigenes Dokument, Pipette außerhalb des Fensters, Innen einfügen als Maskengruppe, Gamma ohne
  Schachbrett, Datei-Info, ffmpeg, Skriptvergleich der neuen Angleichungen ausstehend).
- Neutralität: Ordner, NEU/NEW, Handbücher, LICENSE ohne gesperrte Namen. In beiden exe kommt der Herstellername nur als
  Formatkennung fremder Bibliotheken vor (XMP-Namensraum, JPEG-APP14/Farbraum-Meldung des Decoders, Brotli-Wörterbuch;
  15 bzw. 14 Fundstellen, mehr Kopien derselben Zeichenketten), der Programmname des Originals nicht.

## Smoke-Test (Endversion, echtes Fenster, Brücke)

- Eigenes Profil je Lauf im Scratchpad (`gt2\home-de`, `home-en`, `home-t1…t3`; APPDATA/LOCALAPPDATA,
  `FP_NO_SHELL_OPEN=1`), `app.ron` 900 × 650 unten rechts, `--mcp`, kein SetForegroundWindow, Ende per `WM_CLOSE` →
  **Exit 0 in allen 5 Läufen**; kein Fenster über das Original gebracht.
- **Deutsch:** `ui_state` ok (`mode: bridge`, keine Dokumente, Statuszeile „KI-Steuerkanal aktiv …“); `ui_find` Menüleiste
  „Befehle“, „Filter“, „Fenster“, „Hilfe“, „Erweiterter Modus“.
- **Englisch:** Statuszeile „AI control channel active …“; Menüleiste „File … Help“, „Expanded Mode“; Bildschirmfoto
  900 × 650: Startseite, Bedienfelder, Werkzeugleiste englisch, nichts abgeschnitten.
- `firepixel-cli batch --templates --json`: **12** Vorlagen, Exit 0. `--lang de batch --help` → „Aufruf: firepixel-cli batch
  [OPTIONEN] [DATEIEN]...“, `--lang en` → „Usage: firepixel-cli batch [OPTIONS] [FILES]...“, Exit 0; `--help` DE/EN ok.

## Startzeit (`FP_STARTUP_TRACE`, Erststart mit neuem Profil, vor jedem Lauf 0 `cargo`/`rustc`)

| Marke | t1 (ms) | t2 (ms) | t3 (ms) | **Median** |
|---|---|---|---|---|
| `wgpu: DX12 setup` | 106 | 80 | 111 | 106 |
| `eframe: window + graphics ready (App::new)` | 174 | 145 | 203 | 174 |
| `frame 1 begin` | 192 | 163 | 222 | 192 |
| `window shown (offscreen, settling)` | 374 | 352 | 388 | 374 |
| **`window placed (visible to the user)`** | 933 | 898 | 966 | **933** |
| **`first frame after placement (interactive)`** | 946 | 977 | 976 | **976** |

Langsamer als GT-0g (783/816 ms), aber **keine Verschlechterung des Programms**: die Vorversion 2026-10-14_15-33 brauchte
unter denselben Bedingungen direkt danach ebenfalls 913/920 ms (zweiter Lauf; erster kalt 1.450 ms). Die Maschine war
belebter (Virenscanner, das Original und andere Sitzungen offen, CPU ≈ 10 %), keine cargo-Prozesse.

## Offene Punkte (kurz)

- `fp-raster` Korpustest `t4_zoom` an das gewachsene Zoom-Ergebnis von FX-3 anpassen (Testpflege).
- `dialogs_ux::width_field_limited_to_6000_with_message` wackelt weiter im Gesamtlauf.
- Git-Tag der Endversion steht aus (auf Anweisung des Nutzers); Paarbild-Nachprüfung TW-33…41/Q-18…22 im nächsten Skriptlauf.
- Unverändert offen laut BEFUNDE F: TW-36-Rest (Symbolansicht als eigenes Dokument), `showAttrs` nach Maskenbefehlen ungemessen.

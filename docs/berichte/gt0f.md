# GT-0f – Release-Build und neue Testversion (13./14.10.2026, ≈ 23:58–00:35)

## Stand

- `main` @ **`c4388f4`** (I18N-CHECK), sauberer Worktree `C:\Users\Administrator\Desktop\REA\fp-rel`
  (`checkout -q --detach main`, `status --short` leer), Zielordner `fp-rel\target-rel`, `CARGO_INCREMENTAL=0`, `-j 8`.
- Neu seit GT-0e (`034fe6b`): FIX-N33, FIX-R1, FIX-R2, FIX-R3, I18N-CHECK.

## Tests (`cargo test --workspace --no-fail-fast -j 8`, Debug, 14 min 56 s)

1948 bestanden, **1 rot**, 127 ignoriert. Die 4 roten Tests aus GT-0e sind grün (FIX-R3).

| Test | Meldung |
|---|---|
| `fp-ui --test all` `dialogs_ux::width_field_limited_to_6000_with_message` | `dialogs_ux.rs:241` „Meldung statt stillem Klemmen“ – die Breite wird richtig auf 6000 begrenzt, aber die Statuszeile enthielt beim Prüfen nicht „6000“. **Einzeln wiederholt: grün (0,23 s)** – wackelt vermutlich unter Parallel-Last (Statuszeile überschrieben bzw. Meldung noch nicht gesetzt). Nicht repariert. |

Die Testläufe hatten in fp-rel nachverfolgte Dateien unter `docs\funktionstest\` und `docs\screenshots\` neu
geschrieben; mit `git checkout -- docs` zurückgesetzt, fp-rel danach sauber.

## Release-Build

- `cargo build --release -p firepixel -p firepixel-cli -j 8`: grün, 6 min 27 s.
- `firepixel.exe` 77,5 MB, `firepixel-cli.exe` 53,6 MB.

## Testversion

`C:\Users\Administrator\Desktop\Firepixel-Vorschau-2026-10-14_00-22\` mit `firepixel.exe`, `firepixel-cli.exe`,
`docs\MANUAL.md`, `docs\MANUAL.en.md`, `NEU.md` und `NEW.md` (englisch, gleicher Inhalt). Stand `c4388f4`;
„Neu seit Vorschau 2026-10-13_23-11“ aus FIX-N33, FIX-R1, FIX-R2, FIX-R3, I18N-CHECK (N33 Batch-Knöpfe zuerst,
dann Maske ein/aus vertauscht, Stern-Größe, Unterebene umbenennen, komplett Deutsch/Englisch inkl. CLI-Hilfe);
14 Ausprobier-Schritte (5 neue + 9 aus der letzten NEU.md; „Rauschen“, „Rechteck-Werkzeug“ entfallen, die alte
Masken- und Unterebenen-Prüfung ist in den neuen Schritten aufgegangen); bekannte Lücken aktualisiert (Stern-Größe
und Masken-Inspektor sind erledigt, neu: alte Dateien mit Masken aus „Alles einblenden/ausblenden“ zeigen weiter
die alte Polarität). Beschriftungen in NEW.md aus `locales\en`.

Neutralität: keine Namen des Originals/Herstellers in Ordner, NEU/NEW, Handbüchern. In beiden exe kommt
der Herstellername nur als Formatkennung fremder Bibliotheken vor (XMP-Standard-Namensraum, JPEG-APP14-Marker
und Farbraum-Meldung des Decoders, Wörterbuch des Brotli-Dekoders) – unverändert gegenüber der Vorversion
(dort 11 Treffer), nicht sichtbar in der Oberfläche; der Programmname des Originals kommt nicht vor.

## Smoke-Test (echtes Fenster, Brücke)

- Eigenes Profil je Lauf im Scratchpad (`gt0f\home-de`, `home-en`, `home-t1…t3`: APPDATA/LOCALAPPDATA,
  `FP_NO_SHELL_OPEN=1`), `app.ron` 900 × 650 unten rechts, `--mcp`, kein SetForegroundWindow, Ende per
  `WM_CLOSE` → Exit 0 (alle 5 Läufe). DX12 direkt.
- **Deutsch** (`preferences.json` `Language: de`): `ui_state` ok (`mode: bridge`, `tool.pointer`, Design `normal`,
  keine Dokumente, Statuszeile „KI-Steuerkanal aktiv …“); `ui_find` Menüleiste: „Befehle“, „Filter“, „Fenster“,
  „Hilfe“, „Erweiterter Modus“.
- **Englisch** (`Language: en`): `ui_state` ok, Statuszeile „AI control channel active …“; Menüleiste „File, Edit,
  View, Select, Modify, Text, Commands, Filters, Window, Help“, „Expanded Mode“; Bildschirmfoto (900 × 650):
  Startseite, Bedienfelder und Werkzeugleiste englisch, nichts abgeschnitten.
- `firepixel-cli batch --templates`: **12** Vorlagen, Exit 0. `--lang de batch --help` → „Aufruf: firepixel-cli
  batch [OPTIONEN] [DATEIEN]...“, `--lang en` → „Usage: firepixel-cli batch [OPTIONS] [FILES]...“.

## Startzeit (`FP_STARTUP_TRACE`, Erststart mit neuem Profil, ruhige Maschine)

Vor jedem Lauf geprüft: keine `cargo`-/`rustc`-Prozesse aktiv.

| Marke | Lauf 1 (ms) | Lauf 2 (ms) | Lauf 3 (ms) | **Median** |
|---|---|---|---|---|
| `wgpu: DX12 setup` | 58 | 64 | 59 | 59 |
| `eframe: window + graphics ready (App::new)` | 107 | 109 | 101 | 107 |
| `frame 1 begin` | 117 | 121 | 113 | 117 |
| `window shown (offscreen, settling)` | 245 | 286 | 249 | 249 |
| **`window placed (visible to the user)`** | 749 | 778 | 768 | **768** |
| **`first frame after placement (interactive)`** | 791 | 822 | 814 | **814** |

Damit bestätigt: die 986–1273 ms aus GT-0e kamen von der Last; ruhig liegt der Start wie bei GT-0d (≈ 780 ms).
Rund 500 ms davon entfallen auf die Phase zwischen „window shown (offscreen, settling)“ und „window placed“.

## Befunde

- 1 roter Test, einzeln grün: `dialogs_ux::width_field_limited_to_6000_with_message` wackelt im Gesamtlauf.
- Startzeit ruhig: Median 768 ms sichtbar / 814 ms bedienbar.

# GT-0e – Release-Build und neue Testversion (13.10.2026, ≈ 22:40–23:15)

## Stand

- `main` @ **`034fe6b`** (RUN-2), sauberer Worktree `C:\Users\Administrator\Desktop\REA\fp-rel`
  (`checkout -q --detach main`, `status --short` leer), Zielordner `fp-rel\target-rel`, `CARGO_INCREMENTAL=0`, `-j 8`.
  Vorher 52 GB frei auf C:, nach dem Debug-Testbau 33,6 GB.
- Neu seit GT-0d (`ab972c0`): FIX-G1, FIX-G2, FIX-G3, FIX-G4, FIX-G5, FIX-N32, RUN-2.

## Tests (`cargo test --workspace --no-fail-fast -j 8`, Debug, 22 min 36 s inkl. Erstbau)

1927 bestanden, **4 rot**, 127 ignoriert. Nicht repariert:

| Test | Meldung |
|---|---|
| `firepixel-cli` (bin) `tests::parameterbeschreibungen_in_de_und_en` | 28 Parameterbeschreibungen ohne Übersetzung, u. a. neu `modify.transform.numericTransform` „Attribute skalieren (Strichbreite mit)“ (FIX-G1), `modify.exitBitmapMode` (FIX-G4), dazu `commands.runScript`/`commands.run` (prompt-Antworten), `edit.cropDocument` |
| `fp-ui --test all` `clipping_audit::clipping_audit` | 1 neuer Fund: Dialog `text.spellingSetup`, Textfeld mit dem vorbelegten Wörterbuchpfad läuft über (FIX-G2/TW-13) – nicht in `clipping_audit_allow.txt` |
| `fp-ui --test dialogs_i18n` `dialoge_in_allen_sprachen_ohne_ueberlauf_und_ohne_rohschluessel` | Pseudosprache `qps`: in `file.htmlSetup` überlappt „HTML-Kommentare einschließen“ mit „CSS für Popupmenüs verwenden“ (zweispaltig, FIX-G2/TW-06) |
| `fp-ui --test all` `menu_fixes_fv13::secondary_browser_preview_needs_configured_browser` | bekannt (externe Programme, GUI-Sitzung) |

Die Testläufe hatten in fp-rel nachverfolgte Dateien unter `docs\funktionstest\` und `docs\screenshots\` neu
geschrieben; mit `git checkout -- docs` zurückgesetzt, fp-rel danach sauber.

## Release-Build

- `cargo build --release -p firepixel -p firepixel-cli -j 8`: grün, 6 min 43 s.
- `firepixel.exe` 77,4 MB, `firepixel-cli.exe` 53,6 MB.

## Testversion

`C:\Users\Administrator\Desktop\Firepixel-Vorschau-2026-10-13_23-11\` mit `firepixel.exe`, `firepixel-cli.exe`,
`docs\MANUAL.md`, `docs\MANUAL.en.md` und `NEU.md` (Stand `034fe6b`; „Neu seit Vorschau 2026-10-13_21-21“ aus
FIX-G1…G5 und FIX-N32, N32 „nur ein blauer Rahmen“ zuerst; 12 Ausprobier-Schritte; bekannte Lücken).
Neutralität geprüft (keine Namen des Originals/Herstellers im Ordner, auch nicht in den exe).

## Smoke-Test (echtes Fenster, Brücke)

- Eigenes Profil im Scratchpad (`gt0e\home1`, `home2`: APPDATA/LOCALAPPDATA, `FP_NO_SHELL_OPEN=1`), `app.ron`
  900 × 650 unten rechts, `--mcp`, kein SetForegroundWindow, Ende per `WM_CLOSE` → Exit 0 (beide Läufe).
- DX12 direkt.
- `ui_state` über die Brücke: ok (`mode: bridge`, `tool.pointer`, Design `normal`, keine Dokumente, Standard-
  Bedienfelder sichtbar, Frame 19).
- `firepixel-cli batch --templates`: **12** Vorlagen, Exit 0.

## Startzeit (`FP_STARTUP_TRACE`, Erststart mit neuem Profil)

| Marke | Lauf 1 (ms) | Lauf 2 (ms) |
|---|---|---|
| `wgpu: DX12 setup` | 204 | 125 |
| `eframe: window + graphics ready (App::new)` | 273 | 342 |
| `frame 1 begin` | 288 | 375 |
| `window shown (offscreen, settling)` | 434 | 748 |
| **`window placed (visible to the user)`** | **986** | **1273** |
| **`first frame after placement (interactive)`** | **991** | **1281** |

Langsamer als GT-0d (780/785 ms). Während der Messung liefen parallel andere `cargo`/`rustc`-Prozesse
(andere Slots); die Streuung zwischen den beiden Läufen spricht für Last, nicht für eine Programmänderung.
Gegenmessung auf ruhiger Maschine offen.

## Befunde

- 3 neue rote Tests aus FIX-G1/G2/G4 (s. o.): fehlende EN-Parameterbeschreibungen, Überlauf Wörterbuchpfad in
  „Rechtschreibung einrichten“, Überlappung in „HTML-Einrichtung“ (Pseudosprache).
- Startzeit unter Last 986–1273 ms – auf ruhiger Maschine nachmessen.

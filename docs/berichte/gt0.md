# GT-0 – Release-Build, Testversion, warme Slots (13.10.2026, ≈ 07:05–07:45)

## Release-Build

- Stand: `main` @ **`73c3379`** (FX-3), sauberer Worktree `C:\Users\Administrator\Desktop\REA\fp-rel`
  (`status --short` leer, `checkout -q --detach main`), Build-Ordner `fp-rel\target-rel`, Profil release
  (`lto = "thin"`, `codegen-units = 1`), `CARGO_INCREMENTAL=0`, `-j 4`.
- `cargo build --release -p firepixel -p firepixel-cli`: grün, 17 min (Neubau, `target-rel` gab es noch nicht).
- `firepixel.exe` 76,1 MB, `firepixel-cli.exe` 53,0 MB.
- fp-rel danach sauber (`status --short` leer, detached auf `73c3379`); `target-rel` bleibt als warmer Release-Ordner.

## Smoke-Test (echtes Fenster, Brücke)

- Eigenes Profil im Scratchpad (`gt0\home1`: APPDATA/LOCALAPPDATA, `FP_NO_SHELL_OPEN=1`), vorbereitete
  `app.ron` 900 × 650 unten rechts, `--mcp`, kein SetForegroundWindow, Ende per `WM_CLOSE` (Exit 0).
- **DX12 direkt (FIX-A) funktioniert:** Marke `wgpu: DX12 setup` 96 ms, Fenster normal; der Rückfall
  `FP_WGPU_ALL_BACKENDS=1` war **nicht** nötig.
- `ui_state` über die Brücke: Antwort ok (`mode: bridge`, Werkzeug `tool.pointer`, Design `normal`,
  keine Dokumente, alle Standard-Bedienfelder sichtbar, Frame 26).
- `firepixel-cli batch --templates`: 11 Vorlagen, Exit 0.

## Startzeit (`FP_STARTUP_TRACE`, Erststart mit neuem Profil, ein Lauf)

| Marke | ms seit `main` |
|---|---|
| `eframe: window + graphics ready (App::new)` | 157 |
| `frame 1 begin` | 166 |
| `window shown (offscreen, settling)` | 300 |
| **`window placed (visible to the user)`** | **813** |
| **`first frame after placement (interactive)`** | **857** |

Vorladen (Schriften 18 ms, Sprachdateien 17 ms, Schriftindex 33 ms) ist vor `App::new` fertig; Befehlsordner im Hintergrund nach 207 ms.
Hinweis: Einzelmessung „kalt“ (neues Profil); START-1 hat Mediane über 10 Läufe.

## Testversion

`C:\Users\Administrator\Desktop\Firepixel-Vorschau-2026-10-13_07-22\` mit `firepixel.exe`, `firepixel-cli.exe`,
`MANUAL.md`, `MANUAL.en.md` (aus `docs\`) und `NEU.md` (N25–N31, Batch-Studio mit „geht schon / kommt noch“,
Lizenz Apache-2.0, 10 Schritte „Was du ausprobieren kannst“). BATCH-3 (Umbenennen) und BATCH-8b waren beim
Stand `73c3379` noch nicht auf main und sind in NEU.md als „kommt noch“ vermerkt.

## Warme Slots

- `fp-s1` und `fp-s6` (beide vorher sauber) zuerst auf `73c3379` vorgebaut, dann – weil inzwischen BATCH-3 und PAR-Commits auf main kamen – auf **`c8a20ee`** (main um 07:30) nachgezogen. In `target-s1` / `target-s6` jeweils `cargo build -p firepixel` und `cargo test -p fp-ui --test all --no-run` (`CARGO_INCREMENTAL=1`, `-j 3`) grün; beide Slots sauber, detached auf `c8a20ee`.
- Die Testversion enthält BATCH-3 (`d068869`, Umbenennen-UI) **nicht**; sie ist vom Stand `73c3379`.

## Befunde

- Keine neuen. (C: vorher 57,9 GB frei, keine Aufräumarbeit nötig.)

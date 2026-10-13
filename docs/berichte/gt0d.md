# GT-0d – Release-Build und neue Testversion (13.10.2026, ≈ 21:18–21:25)

## Release-Build

- Stand: `main` @ **`ab972c0`** (A3: Tweak-Liste), sauberer Worktree `C:\Users\Administrator\Desktop\REA\fp-rel`
  (`checkout -q --detach main`, `status --short` leer), Build-Ordner `fp-rel\target-rel`,
  `CARGO_INCREMENTAL=0`, `-j 8`; vorher 79 GB frei auf C:.
- `cargo build --release -p firepixel -p firepixel-cli -j 8`: grün, 2 min 41 s.
- Seit `6feb4e0` (GT-0c) nur `docs/` und `tools/parity/` geändert, kein Quellcode: `firepixel.exe` wurde nicht neu
  gelinkt und ist byte-gleich mit der Vorschau 15:48 (73,8 MB); `firepixel-cli.exe` neu gelinkt, 51,1 MB.
- fp-rel danach sauber (`status --short` leer, detached auf `ab972c0`).

## Smoke-Test (echtes Fenster, Brücke)

- Eigenes Profil im Scratchpad (`gt0d\home1`: APPDATA/LOCALAPPDATA, `FP_NO_SHELL_OPEN=1`), `app.ron`
  900 × 650 unten rechts, `--mcp`, kein SetForegroundWindow, Ende per `WM_CLOSE` → Exit 0.
- DX12 direkt: `wgpu: DX12 setup` 64 ms.
- `ui_state` über die Brücke: ok (`mode: bridge`, `tool.pointer`, Design `normal`, keine Dokumente, Standard-
  Bedienfelder sichtbar, Frame 20).
- `firepixel-cli batch --templates`: **12** Vorlagen, Exit 0.

## Startzeit (`FP_STARTUP_TRACE`, Erststart mit neuem Profil, ein Lauf)

| Marke | ms seit `main` |
|---|---|
| `eframe: window + graphics ready (App::new)` | 110 |
| `frame 1 begin` | 121 |
| `window shown (offscreen, settling)` | 276 |
| **`window placed (visible to the user)`** | **780** |
| **`first frame after placement (interactive)`** | **785** |

Gleichauf mit GT-0c (814/845 ms; gleiche exe, Streuung).

## Testversion

`C:\Users\Administrator\Desktop\Firepixel-Vorschau-2026-10-13_21-21\` mit `firepixel.exe`, `firepixel-cli.exe`,
`docs\MANUAL.md`, `docs\MANUAL.en.md` und `NEU.md` (seit 15:48: keine Programmänderung, intern Runner/Doku/
Tweak-Liste; Kurzfassung 14:39/15:48; die 10 Schritte „Was du ausprobieren kannst“ übernommen).
Neutralität geprüft (keine Namen des Originals/Herstellers im Ordner).

## Befunde

- Keine neuen.

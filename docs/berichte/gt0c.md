# GT-0c – Release-Build und neue Testversion (13.10.2026, ≈ 15:42–15:52)

## Release-Build

- Stand: `main` @ **`6feb4e0`** (FIX-F-Befunde), sauberer Worktree `C:\Users\Administrator\Desktop\REA\fp-rel`
  (`status --short` leer, `checkout -q --detach main`), Build-Ordner `fp-rel\target-rel`,
  `CARGO_INCREMENTAL=0`, `-j 8`.
- `cargo build --release -p firepixel -p firepixel-cli -j 8`: grün, 6 min 05 s.
- `firepixel.exe` 73,8 MB, `firepixel-cli.exe` 51,1 MB.
- fp-rel danach sauber (`status --short` leer, detached auf `6feb4e0`).

## Smoke-Test (echtes Fenster, Brücke)

- Eigenes Profil im Scratchpad (`gt0c\home1`: APPDATA/LOCALAPPDATA, `FP_NO_SHELL_OPEN=1`), `app.ron`
  900 × 650 unten rechts, `--mcp`, kein SetForegroundWindow, Ende per `WM_CLOSE` → Exit 0.
- DX12 direkt: `wgpu: DX12 setup` 80 ms.
- `ui_state` über die Brücke: ok (`mode: bridge`, `tool.pointer`, Design `normal`, keine Dokumente, alle
  Standard-Bedienfelder sichtbar, Frame 26).
- `firepixel-cli batch --templates`: **12** Vorlagen, Exit 0.

## Startzeit (`FP_STARTUP_TRACE`, Erststart mit neuem Profil, ein Lauf)

| Marke | ms seit `main` |
|---|---|
| `eframe: window + graphics ready (App::new)` | 140 |
| `frame 1 begin` | 156 |
| `window shown (offscreen, settling)` | 305 |
| **`window placed (visible to the user)`** | **814** |
| **`first frame after placement (interactive)`** | **845** |

Gleichauf mit GT-0b (828/859 ms).

## Testversion

`C:\Users\Administrator\Desktop\Firepixel-Vorschau-2026-10-13_15-48\` mit `firepixel.exe`, `firepixel-cli.exe`,
`docs\MANUAL.md`, `docs\MANUAL.en.md` und `NEU.md` (seit 14:39 = `75d80a3`: FIX-F – Menütexte Auswahl,
Trennlinien in Bedienfeld-≡-Menüs wirkungslos, `pi.fill` ohne Leer-Schritt, Stempel-Zugende, Doku; intern
RUN-1/SCRIPT-A…C ohne sichtbare Änderung; Kurzfassung der Version 14:39; 10 Schritte „Was du ausprobieren
kannst“). Neutralität geprüft (keine Namen des Originals/Herstellers im Ordner).

## Befunde

- Keine neuen.

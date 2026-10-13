# GT-0b – Release-Build und neue Testversion (13.10.2026, ≈ 14:33–14:45)

## Release-Build

- Stand: `main` @ **`75d80a3`** (FIX-E-Bericht), sauberer Worktree `C:\Users\Administrator\Desktop\REA\fp-rel`
  (`status --short` leer, `checkout -q --detach main`), Build-Ordner `fp-rel\target-rel` (warm von GT-0),
  `CARGO_INCREMENTAL=0`, `-j 8`.
- `cargo build --release -p firepixel -p firepixel-cli -j 8`: grün, 5 min 41 s.
- `firepixel.exe` 77,3 MB, `firepixel-cli.exe` 53,5 MB.
- fp-rel danach sauber (`status --short` leer, detached auf `75d80a3`).

## Smoke-Test (echtes Fenster, Brücke)

- Eigenes Profil im Scratchpad (`gt0b\home1`: APPDATA/LOCALAPPDATA, `FP_NO_SHELL_OPEN=1`), `app.ron`
  900 × 650 unten rechts, `--mcp`, kein SetForegroundWindow, Ende per `WM_CLOSE` → Exit 0.
- DX12 direkt: `wgpu: DX12 setup` 64 ms.
- `ui_state` über die Brücke: ok (`mode: bridge`, `tool.pointer`, Design `normal`, keine Dokumente, alle
  Standard-Bedienfelder sichtbar, Frame 26).
- `firepixel-cli batch --templates`: **12** Vorlagen (neu `video-to-gif`), Exit 0.

## Startzeit (`FP_STARTUP_TRACE`, Erststart mit neuem Profil, ein Lauf)

| Marke | ms seit `main` |
|---|---|
| `eframe: window + graphics ready (App::new)` | 119 |
| `frame 1 begin` | 130 |
| `window shown (offscreen, settling)` | 300 |
| **`window placed (visible to the user)`** | **828** |
| **`first frame after placement (interactive)`** | **859** |

Gleichauf mit GT-0 (813/857 ms); App::new 38 ms früher.

## Testversion

`C:\Users\Administrator\Desktop\Firepixel-Vorschau-2026-10-13_14-39\` mit `firepixel.exe`, `firepixel-cli.exe`,
`docs\MANUAL.md`, `docs\MANUAL.en.md` und `NEU.md` (seit 07:22 = `73c3379`: Batch komplett – Umbenennen
BATCH-3, Formulare BATCH-11a, GIF-Optionen/HTML-Bericht/Vorlage Video → GIF BATCH-11b, Optimierer BATCH-8b –,
Werkzeug-Parität PAR-X, Inspektor Verläufe/Muster/Mischmodi, Fixes FIX-B…E aus CT-1…3; 10 Schritte
„Was du ausprobieren kannst“). Neutralität geprüft (keine Namen des Originals/Herstellers im Ordner).

## Befunde

- Keine neuen.

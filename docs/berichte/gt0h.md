# GT-0h – Release-Build und neue Testversion (15.10.2026, ≈ 00:23–00:40)

## Stand

- `main` @ **`54790ef`** (RUN-6), sauberer Worktree `C:\Users\Administrator\Desktop\REA\fp-rel`
  (`checkout -q --detach main`, `status --short` leer), Zielordner `fp-rel\target-rel`, `CARGO_INCREMENTAL=0`, `-j 8`.
  Danach auf `main` nur `d263423`/`83ab3a2` (NUTZERBEFUNDE N34/N35, reine Doku) – Code gleich.
- Neu seit GT-0g-Folgevorschau 2026-10-14_21-48 (`161c229`): F15 (BA-056), G-B3, G-B4, G-B5, RUN-5, RUN-6, B-2b.
- Kein `cargo test --workspace` (Auftrag). Platte vorher 29,1 GB frei, nachher 27,0 GB.

## Release-Build

- `cargo build --release -p firepixel -p firepixel-cli -j 8`: grün, 6 min 35 s.
- `firepixel.exe` 78,0 MB, `firepixel-cli.exe` 53,9 MB.

## Testversion

`C:\Users\Administrator\Desktop\Firepixel-Vorschau-2026-10-15_00-30\` mit `firepixel.exe`, `firepixel-cli.exe`
(Hash gleich `target-rel\release`), `LICENSE`, `docs\MANUAL.md`, `docs\MANUAL.en.md`, `NEU.md` und `NEW.md` (englisch,
gleicher Inhalt, Beschriftungen aus `locales\en`). „Neu seit Vorschau 2026-10-14_21-48“ aus f15, gb3, gb4, gb5:
Abschrägungs-Namen, Stift-Alt-Ziehen, Bitmap-Treffer nur auf Pixeln, gesperrte Ebene beim Malen, Rastern mit
Deckkraft/Mischmodus, Werkzeugleisten-Popups, Datei-Info, Tab-Folge der Filterdialoge, Exportassistent,
Inspektor-Höhe/Reitermenü, Optimieren-Menü, Kürzelsatz „Web Standard“, Skript-Messwerte, Brücke für alle Bedienfelder.
12 Ausprobier-Schritte; bekannte Lücken aktualisiert (Datei-Info-Register gesperrt; Mess-Sitzungs-Hinweis ersetzt).

Neutralität: keine Namen des Originals/Herstellers in Ordner, NEU/NEW, Handbüchern (nur die Formatkennungen FW-PNG/JSF
wie bisher). In beiden exe steht der Herstellername weiter nur als Formatkennung (XMP-Namensraum, JPEG-APP14-Meldung,
Brotli-Wörterbuch); neu sind die XMP-Elementnamen `photoshop:AuthorsPosition`/`photoshop:CaptionWriter` aus G-B4
(Standard-Präfix des XMP-Namensraums, Formatkennung). Der Programmname des Originals kommt nicht vor.

## Smoke-Test (echtes Fenster, Brücke)

- Eigenes Profil je Lauf im Scratchpad (`gt0h\home-de`, `home-en`, `home-t1…t3`: APPDATA/LOCALAPPDATA,
  `FP_NO_SHELL_OPEN=1`), `app.ron` 900 × 650 unten rechts, `--mcp`, kein SetForegroundWindow, Ende per `WM_CLOSE`
  → Exit 0 (alle 5 Läufe). Skript wie GT-0g (`smoke.ps1`).
- **Deutsch:** `ui_state` ok (`mode: bridge`, keine Dokumente, Statuszeile „KI-Steuerkanal aktiv …“); `ui_find`
  Menüleiste „Befehle“, „Filter“, „Fenster“, „Hilfe“, „Erweiterter Modus“.
- **Englisch:** Statuszeile „AI control channel active …“; Menüleiste „Commands, Filters, Window, Help“, „Expanded
  Mode“; Bildschirmfoto 900 × 650: Startseite, Bedienfelder, Werkzeugleiste englisch, nichts abgeschnitten.
- `firepixel-cli batch --templates --json`: **12** Vorlagen, Exit 0. `--lang de batch --help` → „Aufruf:
  firepixel-cli batch [OPTIONEN] [DATEIEN]...“, `--lang en` → „Usage: firepixel-cli batch [OPTIONS] [FILES]...“, Exit 0.

## Startzeit (`FP_STARTUP_TRACE`, Erststart mit neuem Profil, keine fremden cargo/rustc)

| Marke | t1 (ms) | t2 (ms) | t3 (ms) | **Median** |
|---|---|---|---|---|
| `wgpu: DX12 setup` | 64 | 56 | 53 | 56 |
| `eframe: window + graphics ready (App::new)` | 109 | 96 | 94 | 96 |
| `frame 1 begin` | 122 | 105 | 105 | 105 |
| `window shown (offscreen, settling)` | 256 | 238 | 240 | 240 |
| **`window placed (visible to the user)`** | 766 | 736 | 742 | **742** |
| **`first frame after placement (interactive)`** | 810 | 776 | 782 | **782** |

Läufe mit Brücke: DE 798/803 ms, EN 743/774 ms. Gegenüber GT-0g (783/816 ms) gleich bis leicht schneller.

## Befunde

- Keine neuen Fehler im Smoke-Test.
- (Hinweis, unverändert) Produktnamen eines Fremdprogramms im Zusammenhang mit PSD in der Oberfläche
  (Voreinstellungen Zusatzmodule, „PSD-Live-Effekte“), siehe gt0g.md/f5.md.
- Im Hauptbaum liegt eine fremde, nicht versionierte Datei `render.rs` im Wurzelordner (nicht von GT-0h, nicht angefasst).

# GT-0i – Release-Build und neue Testversion (15.10.2026, ≈ 01:31–01:45)

## Stand

- `main` @ **`c3f4705`** (F16), sauberer Worktree `C:\Users\Administrator\Desktop\REA\fp-rel`
  (`checkout -q --detach main`, `status --short` leer), Zielordner `fp-rel\target-rel`, `CARGO_INCREMENTAL=0`, `-j 8`.
- Neu seit Vorschau 2026-10-15_00-30 (`54790ef`): FIX-N34 (`47f8aa7`), FIX-N35 (`c1f7fff`), F16 (`c3f4705`).
- Kein `cargo test --workspace` (Auftrag). Platte vorher 16,7 GB frei; während des Builds wurde anderswo Platz frei,
  danach 60,6 GB frei.

## Release-Build

- `cargo build --release -p firepixel -p firepixel-cli -j 8`: grün, 6 min 32 s.
- `firepixel.exe` 78,1 MB, `firepixel-cli.exe` 53,9 MB.

## Testversion

`C:\Users\Administrator\Desktop\Firepixel-Vorschau-2026-10-15_01-38\` mit `firepixel.exe`, `firepixel-cli.exe`
(Hash gleich `target-rel\release`), `LICENSE`, `docs\MANUAL.md`, `docs\MANUAL.en.md`, `NEU.md` und `NEW.md` (englisch,
gleicher Inhalt, Beschriftungen aus `locales\en`). „Neu seit Vorschau 2026-10-15_00-30“ aus fixn34, fixn35, f16:
Ziehen bleibt im Stapel (auch Alt-Kopie, Skalieren, Transformieren, Unterauswahl), Malvorschau auf vorhandener
Bitmap, Innen einfügen wie im Original (Füllung/Strich bleiben, Kette und Maskenminiatur), Animationsgrenzen
(Dialog waagerecht, Skript/Datei als Vektor). 4 Ausprobier-Schritte; N35 mit dem Ablauf zwei Rechtecke, kleines
wählen + Strg+X, großes wählen, Strg+Umschalt+V. Bekannte Lücken: Malvorschau in neue Bitmap, Textvorschau, Mischmodi
beim Ziehen angenähert; „Innen einfügen legt Maskengruppe an“ entfernt.

Neutralität: keine Namen des Originals/Herstellers in Ordner, NEU/NEW, Handbüchern. In beiden exe steht der
Herstellername weiter nur als Formatkennung (15 bzw. 14 Treffer, gleich wie GT-0h); der Programmname des Originals
kommt nicht vor.

## Smoke-Test (echtes Fenster, Brücke)

- Eigenes Profil je Lauf im Scratchpad (`gt0i\home-de`, `home-en`, `home-t1…t3`, `home-x`), `app.ron` 900 × 650 unten
  rechts, `--mcp`, kein SetForegroundWindow, Ende per `WM_CLOSE`. Skript `gt0h\smoke.ps1` unverändert.
- **Deutsch:** `ui_state` ok (`mode: bridge`, keine Dokumente, Statuszeile „KI-Steuerkanal aktiv …“); Menüleiste
  „Befehle“, „Filter“, „Fenster“, „Hilfe“, „Erweiterter Modus“. Exit 0.
- **Englisch:** Statuszeile „AI control channel active …“; Menüleiste „Commands, Filters, Window, Help“, „Expanded
  Mode“; Bildschirmfoto 900 × 650: Startseite, Bedienfelder, Werkzeugleiste englisch, nichts abgeschnitten. Exit 0.
- **F16 (Schritt 4 geprüft):** `draw.rectangle` 50×20, `modify.animation.animateSelection` mit Versatz 62,35/−36
  (72 px unter 30°) → Instanz 50,50–172,70 (Breite 50 + 72, Höhe unverändert) = Weg waagerecht. Danach blockiert die
  Rückfrage zum ungesicherten Dokument das `WM_CLOSE`, Prozess nach 8 s beendet (erwartet, kein Fehler).
- `firepixel-cli batch --templates --json`: **12** Vorlagen, Exit 0. `--lang de batch --help` → „Aufruf:
  firepixel-cli batch [OPTIONEN] [DATEIEN]...“, `--lang en` → „Usage: firepixel-cli batch [OPTIONS] [FILES]...“, Exit 0.

## Startzeit (`FP_STARTUP_TRACE`, Erststart mit neuem Profil, keine fremden cargo/rustc)

| Marke | t1 (ms) | t2 (ms) | t3 (ms) | **Median** |
|---|---|---|---|---|
| `wgpu: DX12 setup` | 174 | 61 | 52 | 61 |
| `eframe: window + graphics ready (App::new)` | 235 | 103 | 93 | 103 |
| `frame 1 begin` | 245 | 113 | 103 | 113 |
| `window shown (offscreen, settling)` | 406 | 259 | 257 | 259 |
| **`window placed (visible to the user)`** | 912 | 767 | 784 | **784** |
| **`first frame after placement (interactive)`** | 919 | 800 | 810 | **810** |

t1 kalt (erster Start der neuen exe, DX12-Einrichtung 174 ms). Läufe mit Brücke: DE 801/834 ms, EN 755/801 ms.
Gegenüber GT-0h (742/782 ms) im Rahmen der Streuung (+≈ 30 ms Median).

## Befunde

- Keine neuen Fehler im Smoke-Test.
- (Hinweis, unverändert) Produktnamen eines Fremdprogramms im Zusammenhang mit PSD in der Oberfläche, siehe gt0g.md/f5.md.
- N34/N35-Ausprobier-Schritte 1–3 nicht im Fenster wiederholt (in fixn34/fixn35 mit Brücke geprüft); Nachprüfung
  beim Nutzer.

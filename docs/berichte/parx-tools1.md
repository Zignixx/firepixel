# Bericht PAR-X-TOOLS-1 (13.10.2026): Einheiten 03 `tool-pencil-eraser` und 04 `tool-retouch` beheben

Grundlage: `docs\parity\tool-pencil-eraser*.md`, `docs\parity\tool-retouch*.md`, Exporte und Hashes des Originals
`re\_testdateien\par\px\tool-pencil-eraser\`, `…\tool-retouch\` (`pixels.csv`). Gearbeitet im Slot fp-s2, per
cherry-pick auf main. Kalibriert mit Python gegen die Exporte des Originals, danach Golden-Tests in Rust.

## Commits (main)

- `ba7f9a3` Radiergummi und Buntstift (03/03, 03/09, 03/10, 03/11) – `commands\bitmap.rs`, `paint.rs`
- `d070ae5` Retusche-Werkzeuge (04/03, 04/04, 04/06, 04/09, 04/10, 04/11, 04/13, 04/14, 04/15, 04/18) – `commands\tooling.rs`,
  `tools\retouch.rs`, `fp-ui\src\canvas\overlay.rs`, Test `canvas_tools.rs`
- `f676550` Esc nach Zug mit Buntstift/Radiergummi/Retusche → „Bitmapmodus beenden“ (03/16) – `tools\bitmap.rs`, `tools\retouch.rs`
- `8d55f85` Rand-Vorschau im Werkzeug-Inspektor weich (04/01) – `fp-ui\src\inspector\tool_quads.rs`
- `2906afc` Golden-Tests `parx_tools1` + Daten `tests\data\parx-tools1\`, `bitmapwerkzeuge_fv11` angepasst
- dieser Bericht, `docs\parity\*.md`, BEFUNDE

## Umgesetzt (alle 16 ❌ der Einheiten 03/04)

| Befund | Änderung | Ergebnis gegen das Original (Golden `parx_tools1`) |
|---|---|---|
| 03/03 Buntstift Anti-Alias | Linie läuft durch die Pixelmitten der Mauspunkte (nur Bleistift mit Kantenglättung) | Waagrechte eine volle Zeile #C04040, Zeilen 149/151 unverändert; 29 statt 1 435 Pixel > 16 |
| 03/09 eckig, 04/13 | Zug-Abdruck `Dab::stroke`: eckig und Rand 0 hart, genau D Zellen ab `floor(p − D/2) + 1` | Radiergummi eckig **pixelgleich** (5 369 px, 54,144–507,157); Weichzeichner eckig 279,114–481,127, 0 Pixel > 16 |
| 03/10 Rand 0/50 | Rand 0 hart (rund); Kern/Abfall des weichen Abdrucks an Rand 2/50/100 der Züge gemessen | Rand 0 **pixelgleich** (5 337 px); Rand 50: 16 Pixel > 16 (vorher 843), Grenzen gleich |
| 03/11 Deckkraft 50 | Deckung des Zugs zuerst, Deckkraft danach einmal (`erase_dabs`) | max. Abweichung 5; zwei Züge → ¼ Deckung |
| 03/16 Esc nach dem Zug | `escape_after_stroke` (Merker `painted` wie beim Pinsel von PAR-X-Pinsel) für Buntstift, Radiergummi, Retusche | Zeiger, Schritt „Bitmapmodus beenden“, Bitmap bleibt gewählt |
| 04/01 Rand-Vorschau | `edge_preview`: voller Kern, weich auslaufend je Randwert | nur Bild, Nachprüfung im Fenster |
| 04/02, 04/03, 04/12, 04/14 Weichzeichner | `retouch_blur`: je Pixel 1–3 Durchgänge nach Mauspunkten in Reichweite (Klick 1, schneller Zug 1,5, langsamer Zug 3), quer zum Zug Deckung des Zug-Abdrucks | Vorgabe/10/100/zweimal/eckig/Rand 0: 0 Pixel > 16 (Rand 0 vorher max 61) |
| 04/04 Scharfzeichner | gleiche Durchgänge, Verstärkung 0,226 je Durchgang | 42 statt 1 521 Pixel, nur Kanten, max. Abw. 4 |
| 04/06, 04/07 Abwedler/Nachbelichter | `tone_curve` je Kanal (keine Farbverschiebung): Schatten / Mitteltöne (Gamma) / Lichter wie gemessen; Zugdeckung schwächt die Belichtung | 6 Bereiche ≤ 1 Pixel > 16; geänderte Pixel 3 867–6 061 (Original 3 861–6 046) |
| 04/09 Verwischen | `smudge`: Abdrücke je Pixel, Inhalt vom vorigen Abdruck (bilinear), Druck 50 → 47 %, je Abdruck gerundet | weicher Verlauf unverändert (vorher x 150 → #797996), Kanten ≈ 14 px mitgezogen |
| 04/10, 04/11 | `bitmap.retouch` `fingerPaint`/`color`/`wholeDocument`; Voreinstellungen `SmudgeToolFingerPaint`/`SmudgeToolUseEntireDocument` aus dem Inspektor; Seite über Renderer, sonst Leinwand + Bitmap | beide wirken (Test) |
| 04/15 Vektor gewählt | Retusche-Zug mit gewähltem Nicht-Bitmap: Auswahl aufheben, keine Wirkung, kein Schritt | alle 5 Werkzeuge |
| 04/18 Verlaufsnamen | Schritt mit Werkzeugnamen (`tool-blur` …, de/en vorhanden) | „Weichzeichner“, „Scharfzeichner“, „Abwedler“, „Nachbelichter“, „Verwischen“ |

## Annahmen / Grenzen

- Weichzeichner: Das Original trifft eine Kante je nach Lage der Mauspunkte ein- oder zweimal (Intensität 50: 15/29
  Stufen, 100: 31/52) – nicht vorhersagbar; Firepixel rechnet bei schnellen Zügen 1,5 Durchgänge (Abweichung ≤ 13).
  Die ±1-Änderungen im weichen Verlauf bei Intensität 100 (Original 1 722 statt ≈ 300 geänderte Pixel) sind
  Rundungseffekte des Originals und nicht nachgebaut (unsichtbar).
- Verwischen: Übergangsform weicht etwas ab (416 Pixel > 16 bei 6 391 geänderten); „Farbe verwischen“ läuft nach ≈ 30 px
  aus (im Original bleibt im Rotkanal ein schwacher Rest). Farbe = Füllfarbe (das Farbfeld zeigt sie); der Farbwert des
  Originals war nicht auslesbar.
- Stempel, Farbe ersetzen, Rote Augen behalten Abdruck/Abstand von FV-11 (`Dab::new`).
- Keine Bilder/Brücke (§7a), Gegenprüfung durch PAR-R.

## Tests

- neu `crates\fp-engine\tests\parx_tools1.rs` (15 Tests; 23 Referenzexporte des Originals in `tests\data\parx-tools1\`,
  Metadaten entfernt): Golden + Logikfälle §4b (zweiter Zug, Rückgängig/Wiederholen-Folge, Determinismus, Vektor
  gewählt, Esc-Schritt, Verlaufsnamen).
- angepasst: `canvas_tools::escape_mid_drag_leaves_document_unchanged` (Weichzeichner hebt die Auswahl eines Vektors
  beim Drücken auf), `bitmapwerkzeuge_fv11::radiergummi_*` (Schritt „Bitmapmodus beenden“).
- grün: fp-engine `--test all` (bis auf die unten genannten fremden Fehler), `--lib`; fp-ui `bitmapwerkzeuge_fv11`,
  `brush_live_pinsel1`, `canvas_fv14`, `gesamt_werkzeuge`, `tools_fv16`, `ux_a_abgleich`, `inspector*`;
  `cargo test -p fp-neutral`, `-p fp-i18n`, `cargo check --workspace --all-targets`; clippy ohne Warnungen in eigenen Änderungen.
  Keine neuen Texte.

## Für die Gegenprüfung (PAR-R)

| Was | Ablauf | Erwartet |
|---|---|---|
| 03/03 | Buntstift, Anti-Alias an, Zug 60,150 → 500,150 auf T02 | eine Zeile #C04040 |
| 03/09–03/11 | Radiergummi eckig / Rand 0 / Rand 50 / Deckkraft 50 | Hashes e09/e10 (Rand 0) gleich, sonst Δ ≤ 16 |
| 04/01 | Weichzeichner, Rand 100/50/0 | weicher / halbweicher / harter Punkt |
| 04/03–04/14 | Werte aus `tool-retouch.orig.md` | wie Tabelle oben |
| 04/10, 04/11 | Verwischen mit „Farbe verwischen“ bzw. „Gesamtes Dokument“ | Ergebnis anders als ohne Option |
| 04/15, 04/18, 03/16 | Vektor gewählt + Zug; Verlaufsnamen; Esc nach Zug | Auswahl leer; Werkzeugname; „Bitmapmodus beenden“ |

## Neue Befunde / Hinweise

- Fremd, auf main schon vorher rot: `bitmapwerkzeuge_fv11::pinsel_*` (4 Tests, seit 67db4dd Endpunkt eingeschlossen) und
  zeitweise `parx_brush1::tip_spacing_spreads_tracks` – Bereich PAR-X Pinsel. `all_commands_work` (leere Bitmap) hat FIX-B behoben.
- Die nicht committete Änderung an `crates\fp-engine\src\commands\panel.rs` im Hauptbaum stammt nicht von PAR-X-TOOLS-1.

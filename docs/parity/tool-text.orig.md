# 12 `tool-text` – Textwerkzeug auf der Leinwand: Aufnahme des Originals (PAR-O-11-13, 13.10.2026 ≈ 12:50–13:25)

Testdateien T01, T05 als Arbeitskopien `re\_testdateien\par\work\p1113_t0{1,5}.fw.png`, je Messung frisch mit `fw.openDocument(url)`.
Fenster 1600 × 1000, Zoom 100 %, Leinwand bei 297,183, Füllung #808080, Strich #C04040. **Echte Tastatur und Maus**, Textinhalt danach
per Skript (`textRuns`, `rawLeft/Top/Width/Height`, `autoExpand`, `orientation`, `antiAliasMode`). Kein Skriptlauf während der Bearbeitung
(ein Skriptlauf beendet sie). Referenzbilder `re\_testdateien\par\ref\tool-text\` (+ `INDEX.md`), Exporte `re\_testdateien\par\px\tool-text\pixels.csv`.
Art: M = echte Eingabe, J = Skriptwert, W = Fenstertext, E = Export/Hash, B = Bild.

## Prüfpunkte

| Nr | Kürzel | Original | Art |
|---|---|---|---|
| 01 | `click_text` | Klick bei 100,100 → Punkttext, Einfügemarke; Klickpunkt = **Grundlinie** (raw-Oberkante 59 = 100 − 41 bei 48 pt) | M, J, B `01` |
| 02 | `drag_frame` | Ziehen 100,100 → 300,140 → fester Rahmen (`autoExpand` false), Breite 200, **Höhe wächst mit dem Text** (raw 200 × 277 bei 48 pt, 6 Zeilen) – kein Überlauf | M, J, E, B `02` |
| 03 | `typing` | „Abc 123“ (48 pt Myriad Pro Regular, Farbe = Füllfarbe #808080 → gespeichert #7F7F7F): raw 100,59 163 × 53, Pixel 101,66–260,101, Rahmen wächst (`autoExpand` true) | M, J, E, B `03` |
| 04 | `esc` | Esc beendet die Bearbeitung: **Werkzeug bleibt Text**, der Text bleibt **gewählt**; zweites Esc ändert nichts | M, J |
| 05 | `click_out` | Klick außerhalb während der Bearbeitung beendet sie, **legt keinen neuen Text an** (danach getippte Buchstaben wirken als Werkzeugkürzel, z. B. D = Standardfarben auf den gewählten Text, I = Pipette) | M, J |
| 06 | `edit_existing` | Doppelklick mit Zeiger auf TX2 → Werkzeug Text, Einfügemarke **an der Klickstelle** (kein Wortmarkieren): „Q“ landete in „ZeilQe eins“ | M, J, B `06` |
| 07 | `select_text` | Umschalt+Links ×4 markiert „Welt“, Tippen ersetzt → „Hallo Erde“ | M, J, B `07` |
| 08 | `arrows` | Pos1/Ende/Links wirken; **Strg+Links bewegt nicht** (Eingabe „ab cd ef“, Strg+Links, Z, Pos1, X, Ende, Y, Links ×2, W → „Xab cd efWZY“). Das eingefügte „W“ bekam einen eigenen Lauf mit Kerning −0,1 | M, J |
| 09 | `backspace` | „Abcd“, Rücktaste, Links ×2, Entf → „Ac“ | M, J |
| 10 | `enter` | Eingabe = Absatz: Zeichen U+000D im selben Lauf („a\rb“), zweite Zeile 111 hoch statt 53 | M, J |
| 11 | `size_live` | Während der Bearbeitung 3 Zeichen markiert, Größe 72 im Inspektor + Enter → nur diese Zeichen 72 pt (Lauf „Abc “ 48 pt + „123“ 72 pt), Bearbeitung bleibt; alles markiert + 24 → ganzer Text 24 pt | M, J, E, B `11` |
| 12 | `font_live` | Alles markiert, Schrift „Arial“ aus der Liste → Lauf `ArialMT`, 24 pt bleibt | M, J, E |
| 13 | `color_live` | 3 Zeichen markiert, Farbfeld im Inspektor → Popup; gewählte Farbe (#A0A0A0) nur für die markierten Zeichen (eigener Lauf) | M, J, E, B `13` |
| 14 | `bounds` | Gewählter Punkttext: raw 100,59 163 × 53 gegenüber Pixeln 101,66–260,101 (Rahmen um die Zeilenbox, nicht um die Glyphen) | J, B `14` |
| 15 | `frame_resize` | Fester Rahmen, rechter Griff +80 mit Zeiger → Breite 200 → 280, Höhe 83 → 54 (neu umbrochen); Schritt **„Textgröße ändern“** | M, J, E, B `15` (2) |
| 16 | `overflow` | Kein Überlauf beobachtet: Rahmenhöhe folgt immer dem Text (s. 02, 15). Höhe per Griff nicht getestet | J |
| 17 | `vertical` | Richtungs-Knopf im Inspektor → Popup mit 2 Einträgen (horizontal ✓, vertikal); vertikal → `vertical right to left`, raw 88,100 24 × 151; Schritt „Textausrichtung“. Die Richtung **bleibt Werkzeugvorgabe** für neue Texte | M, J, E, B `17` |
| 18 | `aa_render` | Liste: Kein Anti-Alias, Anti-Alias scharf, Anti-Alias stark, Anti-Alias glatt, ----, Benutzerdef. Anti-Alias…; Werte `antiAliased` false / `crisp` / `strong` / `smooth`; Export 2 / 15 / 17 / 17 Farben, Hashes in pixels.csv; Schritt „Rand modifizieren“ | M, J, E, B `18` (4) |
| 19 | `cursor` | Leinwand: I-Balken (Hotspot 11,16); über vorhandenem Text anderer I-Balken (11,16); Rahmengriff mit Zeiger: Griffzeiger 4,4 | B `19` (3) |
| 20 | `undo` | Tippen bis Esc = **ein** Schritt „Text“; ein Strg+Z entfernt den neuen Text ganz | M, J, W |

## Hinweise

- Werkzeug-Inspektor (ohne Auswahl): Myriad Pro / Regular / 48 / Farbfeld / Deckkraft 100 / Normal; Laufweite 0, Zeilenabstand 120 %, F K U,
  Richtung, 4 Ausrichtungen, Einzüge 0, Horizontalskala 100 %, Anti-Alias glatt, „Autom. Unterschneidung“ aus.
- Werkzeugvorgaben merken sich die letzte Einstellung (Schrift Arial 24 und vertikal galten danach für neue Texte).
- T05 TX1 meldet ungültige Pixelgrenzen und raw 10 × 1000 (Testdatei, s. TESTFILES).

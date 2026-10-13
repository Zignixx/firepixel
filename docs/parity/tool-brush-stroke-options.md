# 02 `tool-brush-stroke-options` – Vergleich Original ↔ Firepixel (PAR-F-01-02, 13.10.2026)

Firepixel: Debug-Build `main` a694b48 (enthält PINSEL-1), Slot fp-s4, eigene Instanz, `--mcp`, 1600 × 1000, Design „normal“,
Zoom 100 %; echtes Fenster, Brücke. Strichart-Menü, Listen, Dialog per Klick (`ui_input`), Inhalte per `ui_find`.
Probestriche (03) als **echte Züge** 60,150 → 500,150, Größe 20, #C04040, nichts gewählt (Original: Vektorlinie mit dem
Pinselobjekt – für harte Arten nachweislich pixelgleich, siehe Einheit 01/08). Kennzahlen je Art:
Scratchpad `parf0102\out\t03\cmp03.csv`. Bilder `docs\parity\img\tool-brush-stroke-options\`, Vergleich
`re\_testdateien\par\cmp\tool-brush-stroke-options\` (26 Paare).

| Nr | Prüfpunkt | Original | Firepixel | Bew. | Befund |
|---|---|---|---|---|---|
| 01 | category_list | Menü über dem Feld, ≈ 220 px breit: **Ohne** · Trenner · 12 Kategorien ▸ · Trenner · Pinselstrichoptionen…; Untermenü **rechts daneben** | 12 Kategorien in gleicher Reihenfolge ✅, „Pinselstrichoptionen…“ ✅; aber **„Keine“** statt „Ohne“, kein Trenner danach, Menü **≈ 600 px breit** (477–1077), Untermenü öffnet **unter dem Eintrag über der Liste** und verdeckt die folgenden Kategorien | ❌ | PAR 02/01 |
| 02 | type_list | 54 Arten, Namen und Reihenfolge je Kategorie (re\24) | **alle 54 Namen und Reihenfolgen gleich** (Einfach 4, Airbrush 2, Bleistift 4, Filzstift 4, Gestrichelt 6, Kalligrafie 5, Kohlestift 4, Pastellkreide 3, Wasserfarbe 3, Öl 5, Unnatürlich 9, Unregelmäßig 5) | ✅ | – |
| 03 | type_result | 54 Probestriche (`03_*.png`) | Ausdehnung und Mittelfarbe bei 19 Arten fast gleich (mittlere Abw. ≤ 2,2: Einfach ×4, Airbrush Einfach, 1-Pixel hart/weich, Buntstift, Filzstift ×4, Kalligrafie Band/Einfach/Naß, 3D, 3D Glühen, Zahnpasta …). Abweichend: **Streuung zu regelmäßig/schmal** (Kleckser, Farbkleckser, Konfetti, Punkte, Quadrate, Pelz – Abdrücke in einer Reihe statt ± 50 px gestreut), **Texturarten zu glatt/gesättigt** (Graphit Mittel #BD4342 statt #AB5350, Kohlestift Texturiert, Cremig, Bambus, Pastellkreide Einfach, Wasserfarbe Intensiv), **Garn** als fast volle Fläche (#C83736 statt #C05A4D), **Harke** Farbabweichung (#BF4096 statt #B7556D), Giftmüll #8F9547 statt #6A9C45, **Gestrichelt** endet früher (Gepunktet bis 471 statt 501, Doppelt 487 statt 509), Präzise gestr. 7 960 statt 7 669 px | ❌ | PAR 02/03 |
| 04 | edge_slider | Rand-Popup: senkrechter Schieber 24 px, 0–100, Vorschau links | senkrechter Schieber ≈ 22 × 245, nach oben, Vorschau links | ✅ | – |
| 05 | texture_list | bildschirmhohe Liste ohne Bildlauf, deutsche Namen, „Ader“, „Argyle 01“, „Gips“, „Herz 01“ …, „Andere…“ am Ende | 77 Texturen + „Andere...“ ✅, aber Namen „Rautenmuster 01/02“ statt „Argyle 01/02“, „Putz“ statt „Gips“ (gleiche Liste wie Füllung); Bildlaufliste mit ≈ 20 Zeilen, „Andere...“ fest unten | ❌ | → PAR 18/12 |
| 06 | texture_result | „Herz 01“ 50 %, Größe 20, echter Zug auf B1: 735 px anders als Menge 0 | per Klick in die Liste gewählt, Menge 50 im Feld: **716 px** anders als Menge 0, gleicher Bereich 50,140–509,160 | ✅ | – |
| 07 | texture_amount | Ader, Weich abger. 1 px: 200,150 → 0 #B97350 · 50 #B87F53 · 100 #B68B57; Popup senkrecht 24 × 222 | **alle drei Werte gleich**; Popup senkrecht ≈ 22 × 245 | ✅ | – |
| 08 | blend_list | 46 Modi + 10 Trenner; u. a. Auflösen, Überlagern, Luminanz, Farbton (2.), Löschen | 46 Modi in gleicher Reihenfolge mit Trennern, aber **„Sprenkeln“** (Auflösen), **„Ineinanderkopieren“** (Überlagern), **„Helligkeit“** (Luminanz), **„Tönung“** (Farbton), **„Radieren“** (Löschen); Liste schneidet lange Namen ab (→ PAR inspector-path-rect/07) | ❌ | PAR 02/08 |
| 09 | edit_open | Dialog „Pinselstrich bearbeiten“ 344 × 525, Register „Optionen “/Form/Empfindlichkeit, Strichvorschau, OK/„Cancel“ | Titel gleich, 3 Register, Vorschau, OK/**„Abbrechen“**; Rahmen/Abstände Firepixel-Gestaltung, Vorschau zeigt Wellenlinie statt Gerade | 🎨 | DES-1, PINSEL-1 |
| 10 | edit_options | Tintenmenge/Abstand/Textur/Randtextur/Spitzenabstand mit **„%“**; Randeffekt 6, Variation 5, Gestrichelt 4 Einträge; Ein/Aus bei „Ohne“ 0 | alle Felder, Listen und Reihenfolgen gleich (6/5/4 Einträge ✅); Werte **ohne „%“**; Ein/Aus bei „Ohne“ zeigen 10/2 · 1/2 · 1/2 | ❌ | PAR 02/10 |
| 11 | edit_shape | Spitzenvorschau, Viereckig, Größe/Rand/Breite (Schieber + Feld), **Winkel 360** + ▼ | gleiche Felder und Schieber; Winkel zeigt **0** | ❌ | PAR 02/11 |
| 12 | edit_sensitivity | Liste Größe, Winkel, Tintenmenge, Streuung, Farbton, Helligkeit, Sättigung; Druck, Geschwindigkeit, Horizontal, Vertikal, Zufällig | gleich (7 Einträge, 5 Felder) | ✅ | – |
| 13 | edit_apply | Spitzen 3: echter Zug 51,291–511,311 (9 116 px); Ersatz Spitzen 3 / Abstand 50 → 3 Spuren 31,114–523,177 (17 102 px) | Spitzen 3: 50,290–508,310 (9 076 px) ✅; Spitzen 3 / **Spitzenabstand 50 → eine Spur** 47,137–505,157 (9 076 px), nicht gespreizt | ❌ | PAR 02/13 |
| 14 | edit_cancel | Spitzen 3 → Spitzenabstand sofort aktiv; Cancel verwirft (wieder 1) | Spitzenabstand/Variation aktiv nach Verlassen des Feldes (Tab); Abbrechen verwirft, Wiederöffnen zeigt 1 | ✅ | – |
| 15 | save_stroke | Symbolknopf aktiv (Dialog nicht messbar, interner Fehler) | Knopf „Pinselstrich speichern“ **grau** (Werkzeug-Inspektor, `pi.brush.save` nicht verfügbar) | ❌ | PAR 02/15 |
| 16 | custom_marker | nach OK (Spitzen 3) bleibt „Präzise abger.“, kein Kennzeichen; Wiederöffnen 3 | gleich (bei Bedienung über den Inspektor) | ✅ | – |
| 17 | tool_vs_object | Objekt-Inspektor (Pfad) zeigt dieselben Strichfelder an gleicher Stelle; Wahl wirkt sofort am Objekt | E1 gewählt: Felder an gleicher Stelle; Strichart-Wahl → sofort Verlaufsschritt „Strich ändern“ | ✅ | – |
| 18 | remember | nach Neustart „Weich abger.“, 1, Rand 100 (gespeichert) | Werkzeugwechsel: gemerkt ✅; **nach Neustart** „1-Pixel hart“, 1, Rand 0 (nicht gespeichert) | ❌ | PAR 02/18 |
| 19 | preview_swatch | Kantenvorschau 23 × 23 mit Spitze in **Pinselgröße** (Größe 1 → Punkt) | Vorschau zeigt bei Größe 1 einen **großen weichen Kreis** (≈ 20 px), nicht nach Größe | ❌ | PAR 02/19 |
| 20 | disabled_states | grau: Abstand, Farbfluss, Spitzenabstand, Variation, Ein/Aus; aktiv: Tinte, Übermalen, Textur, Randtextur, Randeffekt, Spitzen, Gestrichelt | gleich | ✅ | – |

**Zählung:** ✅ 9 · ❌ 10 · 🎨 1.

## Fragen / Annahmen

- 03: Zufallsarten können nie pixelgleich sein; bewertet sind Ausdehnung, Streubreite und Mittelfarbe. Die Original-Probestriche
  sind Vektorlinien mit dem Pinselobjekt, Firepixel echte Züge (für harte Arten nachweislich gleichwertig).
- 09: Englisches „Cancel“ und „Optionen “ mit Leerzeichen bewusst nicht übernommen (PINSEL-1); Wellenlinie als Vorschau – Frage
  an den Nutzer, ob die gerade Linie des Originals gewünscht ist.
- 10: „Übermalen“ war im Original-Bild bei „Weich abger.“ an, in Firepixel bei „Präzise abger.“ aus – Vorlagenwert, nicht verglichen.
- 15: Wirkung im Original nicht messbar; Befund nur für den Grau-Zustand.
- Inspektor-Anzeige nach `pi.tool.brush` bzw. Strichart-Wahl nicht nachgeführt: siehe Einheit 01/01.

# 08 `tool-pointer-transform-crop` – Vergleich Original ↔ Firepixel (PAR-F-08-10, 13.10.2026)

Firepixel: Debug-Build `main` 2906afc (Slot fp-s3, `target-s3`), eigene Instanz mit eigenem Profil und `--mcp`, Fenster 1600 × 1000
(Seite bei 297,181 – Original 297,183), Design „normal“, Zoom 100 %; **echtes Fenster, Brücke** (`ui_input` Ziehen 30 Schritte à 15 ms,
Umschalt/Alt 0,25 s vor dem Drücken gehalten, Zustände per `run_script` als Knotenliste `x,y[pred|succ]`, `history`, `export`).
Messdokumente T04/T02 je Messung frisch geöffnet, Füllung #808080 / Strich #C04040. Gleiche Koordinaten wie `paths.json`.
Bilder `docs\parity\img\tool-pointer-transform-crop\` (Mauszeiger fehlt – die Brücke nimmt ihn nicht auf), Vergleich
`re\_testdateien\par\cmp\tool-pointer-transform-crop\`.

| Nr | Prüfpunkt | Original | Firepixel | Bew. | Befund |
|---|---|---|---|---|---|
| 01 | pointer_click | Klick wählt, leer hebt auf, Umschalt addiert, E1 innen, P1 250,175 ja / 240,182 nein | alles gleich (Knoten identisch) | ✅ | – |
| 02 | pointer_rubber | Gummiband wählt alles Berührte (R1; R1+R2+E1+P1) | gleich | ✅ | – |
| 03 | pointer_drag | +40,+10 → 90,60 „Verschieben“; Umschalt → waagrecht / 45°; Alt-Ziehen = Kopie über dem Original, Schritt **„Duplizieren“** | +40,+10 ✅, Alt-Kopie 50,100 über dem Original ✅; **Umschalt-Ziehen am gewählten R1 hebt die Auswahl auf und verschiebt nichts** (kein Schritt); Alt-Schritt **„Kopie verschieben“** | 🔧 behoben (af8c0b5), Gegenprüfung offen | PAR 08/03 |
| 04 | pointer_corner | Eckpunkt ziehen = **Verschieben** (R1 → 90,80–190,150); Knoten von P1 ziehen verschiebt den ganzen Pfad | Eckpunkt **skaliert** (R1 90,85–150,120, „Transformieren“); P1-Knoten ziehen verschiebt **nur den Knoten** (wie Teilauswahl) | 🔧 behoben (af8c0b5), Gegenprüfung offen | PAR 08/04 |
| 05 | nudge | → +1, Umschalt+→ +10, ↓↓ +2, „Verschieben“ | gleich (51 / 61 / 61,52) | ✅ | – |
| 06 | subselect | Knoten +30 → 290,230 [260,190 \| 320,270]; Henkel → 340,290, pred 257.99,191.59; Schritt **„Punkt verschieben“** | Knoten und Henkel **identisch**; Schritt **„Pfad nachzeichnen“** | 🔧 behoben (af8c0b5), Gegenprüfung offen | PAR 08/06 |
| 07 | subselect_group | Teilauswahl auf ST wählt inneren Pfad (10 Knoten), Zeiger die Gruppe | gleich | ✅ | – |
| 08 | select_behind | R2 → R1 → R2 zyklisch; Zeiger bleibt R2 | gleich | ✅ | – |
| 09 | scale_handles | 8 Griffe + Drehpunkt | Griffe + Drehpunkt (Bild `09_`) | ✅ | Bild |
| 10 | scale_shift | Ecke 150,120→200,140: proportional per **Projektion** → 50,50–193,150; Alt → 7,20–193,150; Seite → 50,50–200,120; **innen ziehen = verschieben** (90,60); „Transformieren“ | proportional, aber **größerer Faktor** → 50,50–**200,155**; Alt → **0,15–200,155**; Umschalt gleich ✅; Seite ✅; **innen ziehen tut nichts**; Schritt ✅ | 🔧 behoben (af8c0b5), Gegenprüfung offen | PAR 08/10 |
| 11 | rotate | frei ≈ 52° (97,24 / 158,103 …), Umschalt 45°, kleiner Zug 0°; „Transformieren“ | Knoten gleich (± 0,6 px), 45° ✅, 0° ✅; Schritt **„Verzerren“** | 🔧 behoben (af8c0b5), Gegenprüfung offen | PAR 08/11 |
| 12 | pivot | Drehpunkt → 50,50, Umschalt-Drehen 30°: 50,50 / 137,100 / 102,161 / 15,111 | gleich (136.6,100 / 101.6,160.62) | ✅ | – |
| 13 | skew | Mittelgriff oben +30 → **nur obere Kante** (80,50/180,50, unten bleibt); Ecke senkrecht −20 → 150,30 / 150,140; Ecke waagrecht +20 → oben 30,50/170,50 | Mittelgriff: oben +30 **und unten −30** (20,120/120,120); Ecke senkrecht: **keine Wirkung**; Ecke waagrecht: oben 70,50/170,50, unten verschoben (30/130) | 🔧 behoben (af8c0b5), Gegenprüfung offen | PAR 08/13 |
| 14 | distort | Ecke 150,50→170,30 nur diese Ecke; Mittelgriff oben (+20,−10) → beide oberen Ecken 70,40/170,40 | Ecke ✅ (Schritt „Verzerren“ statt „Transformieren“); Mittelgriff nur **senkrecht** (50,40/150,40) | 🔧 behoben (af8c0b5), Gegenprüfung offen | PAR 08/14 |
| 15 | transform_enter_esc | Enter übernimmt, **Esc verwirft** (kein Schritt), Doppelklick innen/außen und Zeiger-Knopf übernehmen, Skriptlauf beendet ohne Übernahme | Enter, Doppelklick, Zeiger-Knopf, Skript ✅; **Esc übernimmt** („Transformieren“) | 🔧 behoben (af8c0b5), Gegenprüfung offen | PAR 08/15 |
| 16 | transform_bitmap | B1 Ecke → 40,40–603,415; Seite unten → 40,40–520,200; Umschalt-Drehen 15° → 7,−17–553,417 | Ecke → **40,40–610,420** (Faktor wie 10); Seite ✅ (Δ Orig 2,7 % Pixel, Neuberechnung); Drehen 6,−17–554,417 ✅ (Δ 3,7 %) | 🔧 behoben (af8c0b5), Gegenprüfung offen | Folge 08/10 |
| 17 | crop_frame | außen abgedunkelt, 8 Griffe; Inspektor **B 400 H 300 X 100 Y 100 live** | Rahmen + Abdunkeln (Bild); Inspektor-Felder B/H/X/Y **leer** | 🔧 behoben (af8c0b5/a0c725f), Gegenprüfung offen | PAR 08/17 |
| 18 | crop_apply | Enter → 400 × 300, Objekte −100,−100, **ganz außerhalb gelöscht (ST)**, danach **alle gewählt**, Werkzeug bleibt, „Dokument zuschneiden“; Esc verwirft (→ Zeiger); Doppelklick übernimmt; Griff → 500 × 350; **Rahmen über den Rand auf Leinwand begrenzt** (→ 300 × 200) | Größe, Verschiebung, Werkzeug, Schritt, Esc, Doppelklick, Griff ✅; **ST bleibt erhalten**, **nichts gewählt**; Rahmen −50,−50→300,200 **erweitert** die Leinwand (350 × 255, Objekte +50) | 🔧 behoben (af8c0b5), Gegenprüfung offen | PAR 08/18 |
| 19 | crop_bitmap | Dokument 200 × 200, Pixel = Ausschnitt (77876dfefc6fc3c3), **B1 danach 0,0–200,200** | Hash **gleich**; B1 bleibt aber ganz (−60,−60–420,260, nur verschoben) | 🔧 behoben (af8c0b5), Gegenprüfung offen | PAR 08/19 |
| 20 | export_area | Rahmen 50,50–300,200, Inspektor live 250/150/50/50; Doppelklick öffnet **„Bildvorschau“** (Exportbereich ☑, X/Y/B/H) | Inspektorfelder leer; Doppelklick öffnet eigenen Dialog **„Exportbereich exportieren“** (Dateiname, „Bereich: 50,50 – 250 × **155**“); Esc schließt, kein Schritt ✅ | 🔧 behoben (af8c0b5/a0c725f), Gegenprüfung offen | PAR 08/20 |
| 21 | inspector_values | mitten im Zeiger-Zug (vorher ungewählt) zeigt Inspektor **„Dokument“**; Skalieren ohne Live-Wert | zeigt schon **„Pfad R1 100 × 70 bei 50,50“** beim Drücken (Werte nicht live ✅) | 🔧 behoben (af8c0b5/a0c725f), Gegenprüfung offen | PAR 08/21 |
| 22 | cursor | eigene Zeiger je Lage (8 Bilder) | nicht aufgenommen (Brücke ohne Mauszeiger) | – | offen |
| 23 | snap | +47 → +50 (Kante an R2), +44,+3 → +44,0 | gleich (100,50 / 94,50) | ✅ | – |
| 24 | undo | „Verschieben“, „Duplizieren“, „Punkt verschieben“, „Transformieren“, „Dokument zuschneiden“; Esc kein Schritt | „Verschieben“ ✅, „Dokument zuschneiden“ ✅; sonst „Kopie verschieben“, „Pfad nachzeichnen“, „Verzerren“ (Drehen/Neigen/Verzerren gemischt) | 🔧 behoben (af8c0b5), Gegenprüfung offen | 03/06/11/14 |

**Zählung:** ✅ 8 · ❌ 15 · 🎨 0 · offen 1 (Mauszeiger).

## Hinweise / Annahmen

- 03: Zug startet bei 100,85 auf dem bereits gewählten R1; Umschalt war vor dem Drücken gehalten (wie im Original-Lauf angenommen).
- 10/16: Firepixel nimmt den größeren der beiden Faktoren (1,5) statt der Projektion auf die Diagonale (1,43).
- 16 Rotation: Ziehen 580,200 → 580,280 mit Umschalt (15°); Abweichungen der Pixel nur durch die Neuberechnung (max 117).
- 20: Höhe 155 statt 150 vermutlich durch Fang beim Aufziehen – mit 17 (Inspektor) zusammen prüfen.
- 05: mehrere Pfeiltasten ergeben in Firepixel **einen** Schritt „Verschieben“ (Original: je Taste nicht gemessen).
- PAR-X-TOOLS-3 (13.10.): 10 Skalieren rechnet jetzt mit der Projektion auf die Diagonale und setzt die gezogene Ecke auf ganze Pixel (gemessen 193,150 / 7,20 / B1 603,415). Der Drehpunkt ist nur direkt (≤ 4 Bildschirmpixel) greifbar, sonst verschiebt Ziehen innen – Annahme, da der Startpunkt des Original-Zugs nicht aufgezeichnet ist. 18: Zuschneiden mit dem Werkzeug löscht ganz außerhalb liegende Objekte und wählt danach alles (Text der Aufnahme; im Datensatz k18_crop_enter waren die Objekte ungewählt – bitte bei der Gegenprüfung beachten).

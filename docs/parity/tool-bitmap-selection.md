# 06 `tool-bitmap-selection` – Vergleich Original ↔ Firepixel (PAR-F-05-07, 13.10.2026)

Firepixel: Debug-Build `main` 134d195 (Slot fp-s11, Stichproben auf 0c5dcf3 unverändert), eigene Instanz mit eigenem Profil und
`--mcp`, Fenster 1600 × 1000, Design „normal“, Zoom 100 %; echtes Fenster, Brücke (`ui_input` Ziehen 30 Schritte à 15 ms,
Umschalt/Alt ≥ 0,25 s vor dem Klick gehalten, Pfade als down/move/up, `ui_find`, `history`, `get_document.pixelSelection`, `export`).
Messdokument `p57_t02t` (T02, transparente Leinwand; Ausgangsexport = Original b6793cd842444e49), je Messung frisch, B1 gewählt.
**Auswahl als Loch gemessen** wie im Original (Entf, Export, geänderte Pixel gegen den Ausgang). Bilder
`docs\parity\img\tool-bitmap-selection\`, Vergleich `re\_testdateien\par\cmp\tool-bitmap-selection\` (22 Paare).

| Nr | Prüfpunkt | Original | Firepixel | Bew. | Befund |
|---|---|---|---|---|---|
| 01 | marquee_inspector | Stil Normal/Festes Verhältnis/Feste Größe, Felder grau bzw. 1 : 2 / 64 × 64; Rand Hart/Anti-Alias/Weiche Kante + Wert (10), Live ☑ | gleich (Lage ± 5 px) | ✅ | – |
| 02 | marquee_draw | 100,80–300,220, 28 000 px; Inspektor **B 200 H 140 X 100 Y 80** | Hash gleich (ab14a172…); Inspektor zeigt weiter **B1: 480 × 320 bei 40,40** (T01 ohne Objekt: Felder leer) | 🔧 behoben (cdceac5), Gegenprüfung offen | PAR 06/02 |
| 03 | marquee_shift_alt | Umschalt ohne Rahmen = Quadrat (40 000 px); Alt = von der Mitte (9 600 px); mit Rahmen: Umschalt addiert (25 500 / 17 500), Alt zieht ab (7 500), Umschalt→Alt Schnitt (2 500), Alt→Umschalt Abziehen (7 500) | Addieren, Abziehen, Umschalt→Alt **Hash gleich**; Umschalt ohne Rahmen **kein Quadrat** (14 000 px, 100,80–300,150); Alt ohne Rahmen **keine Auswahl** (Entf löscht B1); Alt→Umschalt = **Schnitt** (2 500) | 🔧 behoben (cdceac5), Gegenprüfung offen | PAR 06/03 |
| 04 | marquee_fixed | 100 × 50: Klick → 200,150–300,200; Ziehen → 260,190–360,240 | beide Hash gleich | ✅ | – |
| 05 | marquee_move | Ziehen im Rahmen verschiebt nur den Rahmen; Pfeil 1 px, Umschalt+Pfeil 10 px (Rahmen) | Ziehen im Rahmen zieht **neuen Rahmen** (130,110–180,140); Pfeiltasten **verschieben B1** statt des Rahmens | 🔧 behoben (cdceac5), Gegenprüfung offen | PAR 06/05 |
| 06 | ellipse | Hart 21 992 px; Anti-Alias 22 304, davon 476 teilweise | Hart 22 000; Anti-Alias 22 228, davon 484 teilweise | ✅ | Hinweis |
| 07 | lasso | Dreieck 16 926 px (frei), offen gelassen wird geschlossen (16 969) | 17 000 px (exaktes Dreieck, Hash = Polygon-Lasso des Originals), offen wird geschlossen | ✅ | – |
| 08 | poly_lasso | Doppelklick bzw. Startpunkt schließt (17 000 px); Esc verwirft und **Werkzeug → Zeiger**, B1 gewählt | Schließen beide Hash gleich; Esc verwirft, B1 gewählt (Hash gleich), **Werkzeug bleibt Polygon-Lasso** | 🔧 behoben (cdceac5), Gegenprüfung offen | PAR 06/08 |
| 09 | wand_inspector | Toleranz 32, Rand Hart + Wert grau, Live ☑ | gleich | ✅ | – |
| 10 | wand_click | Klick 160,120: 0 → 6 px; **32 → 5 323 (143,40–178,200)**; **100 → 55 009** | 0 → 6 px (Hash gleich); 32 → **11 028 (125,40–195,200)**; 100 → **71 982** | 🔧 behoben (7a710c3), Gegenprüfung offen | PAR 06/10 |
| 11 | wand_add | Umschalt-Klick Rot addiert 6 400 px (11 723) | addiert 6 400 px (17 428 = 11 028 + 6 400); Schritt „Zauberstab“ | ✅ | Folge 10 |
| 12 | bitmap_mode | kein eigener Rand; Strg+Umschalt+D: Werkzeug bleibt, B1 gewählt | gleich (Rahmen bleibt in `pixelSelection`) | ✅ | – |
| 13 | bitmap_bounds | T03: BM 200,100, 200 × 300 | gleich (Inspektor 200 × 300 bei 200,100) | ✅ | Insp. → 02 |
| 14 | delete_fill | Entf macht Pixel transparent | gleich (02–11) | ✅ | – |
| 15 | copy_paste | neues Bitmap ohne Namen 100,80–200,180, gewählt, Werkzeug bleibt, pixelgleich | gleich (Schritt „Einfügen“) | ✅ | – |
| 16 | drag_move | Zeiger (V) / Strg+Ziehen verschiebt die **Auswahlpixel** +50 (Loch 100–150) | V: **ganzes B1** verschoben (40–570); Strg+Ziehen mit Auswahlrahmen zieht **neuen Rahmen**, keine Pixel bewegt | 🔧 behoben (cdceac5), Gegenprüfung offen | PAR 06/16 |
| 17 | live_marquee | Live an: Rand → Weiche Kante 10 macht vorhandenen Rahmen weich (35 088 px); aus: bleibt hart | an: **bleibt hart** (28 000, Hash wie aus); aus ✅ | 🔧 behoben (cdceac5), Gegenprüfung offen | PAR 06/17 |
| 18 | esc | 1. Esc: Bitmapmodus endet, Zeiger, B1 gewählt; 2. Esc ändert nichts | 1. Esc gleich; **2. Esc hebt die Auswahl von B1 auf** | 🔧 behoben (cdceac5), Gegenprüfung offen | PAR 06/18 |
| 19 | feather_edge | 35 088 px in 90,70–310,230, 21 600 voll, 13 488 teilweise; Alpha Z. 150: 255 255 253 243 224 196 161 116 76 45 22 7 0 | gleiche Zahlen und Alphafolge; Δ Orig 200 px, max 2 | ✅ | – |
| 20 | cursor | eigene Zeiger | nicht aufgenommen | – | offen |
| 21 | empty_doc | T01: Zauberstab nichts; Rahmen 200 × 140 aufziehbar, Entf nichts | gleich (Hash = Ausgang); Statuszeilenhinweise; Inspektorfelder leer → 02 | ✅ | – |
| 22 | undo | Rahmen (auch Lasso) = **„Pixel auswählen“**; Zauberstab = „Zauberstab“ | Rahmen/Lasso **„Auswahlrahmen“**; Zauberstab ✅ | 🔧 behoben (cdceac5), Gegenprüfung offen | PAR 06/22 |

**Zählung:** ✅ 12 · ❌ 9 · 🎨 0 · offen 1 (Mauszeiger).

## Hinweise / Annahmen

- 06: Abweichung ≤ 0,4 % an der Ellipsenkante, als ✅ gewertet.
- 07: Firepixel bekam ein exaktes Dreieck (Brücke), das Original einen echten Mauszug – Zahlen daher nicht direkt vergleichbar;
  Ergebnis = Polygon-Lasso des Originals.
- 10 und tool-fill-gradient-eyedropper/02 (Farbeimer) haben dieselbe Toleranzabweichung (Firepixel wählt bei 32/100 deutlich mehr) –
  vermutlich gemeinsame Farbabstandsfunktion.
- 03: Firepixel wertet nur die beim Drücken gehaltenen Tasten aus (Reihenfolge Alt→Umschalt nicht unterscheidbar).

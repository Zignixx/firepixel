# 06 `tool-bitmap-selection` – Auswahlrahmen, Ellipse, Lasso, Polygonlasso, Zauberstab, Bitmapmodus: Aufnahme des Originals (PAR-O-05-07, 13.10.2026 ≈ 07:05–07:35)

Messdokument `re\_testdateien\par\work\p57_t02t.fw.png` = T02 mit **transparenter Leinwand** (per Skript gespeichert), B1
gewählt; dazu T01, T03. Je Messung frisch geöffnet (`fw.openDocument(url)`). Fenster 1600 × 1000, Zoom 100 %, Leinwand bei
Bildschirm 297,183, echte Mauszüge/Klicks. **Auswahl gemessen als Loch:** Entf drücken, als PNG 32 Bit exportieren,
transparente Pixel zählen (`pixels.csv`: `selected_px`, `partial_alpha_px`, `selection_bbox`). Referenzbilder
`re\_testdateien\par\ref\tool-bitmap-selection\` (+ `INDEX.md`). Messart W/J/E/B/M wie Einheit 05.

## Prüfpunkte

| Nr | Kürzel | Original | Art |
|---|---|---|---|
| 01 | `marquee_inspector` | Überschrift „Auswahlrahmen“ bzw. „Ovaler Auswahlrahmen“ (gleicher Inspektor). **„Stil:“** Liste **Normal / Festes Verhältnis / Feste Größe** (243–381); darunter zwei Felder mit ↔/↕-Symbol (243–276, 301–334), bei Normal **grau** (0/0), bei Festes Verhältnis aktiv **1 : 2**, bei Feste Größe aktiv **64 × 64** (gespeicherte Werte). **„Rand:“** Liste **Hart / Anti-Alias / Weiche Kante** + Wertfeld + ▼ (543–594), Wert nur bei Weiche Kante aktiv (Vorgabe **10**). Kästchen **„Live-Auswahlrahmen“** ☑. Links unten B/H/X/Y: bei gewähltem Objekt dessen Maße, nach dem Aufziehen die Maße des Rahmens. Freigabe der Felder nur über echte Listenwahl (Setzen per Nachricht ohne Schließen der Liste schaltet sie nicht frei) | W, B `01` (5) |
| 02 | `marquee_draw` | Ziehen 100,80 → 300,220: Rahmen **100,80–300,220** (200 × 140, Inspektor B 200 H 140 X 100 Y 80), wandernde Ameisen (gestrichelt schwarz/weiß); Entf → 28 000 px transparent | M, E, W, B `02` |
| 03 | `marquee_shift_alt` | Ohne Rahmen: **Umschalt** = Quadrat nach der größeren Seite (100,80 → 300,150 ergibt 100,80–300,280); **Alt** = von der Mitte (Start 200,150, Ende 260,190 → 140,110–260,190). Mit Rahmen 100,80–200,180: **Umschalt** (≥ 0,2 s vor dem Klick gehalten) **addiert** (Zug 150,130–300,250 → 25 500 px; Start außerhalb 250,230 → 150,130 → 17 500 px); **Alt zieht ab** (7 500 px); **Umschalt, dann Alt** gedrückt = **Schnitt** (2 500 px, 150,130–200,180); **Alt, dann Umschalt** gedrückt = Abziehen (7 500 px). Umschalt nur unmittelbar (< 0,1 s) vor dem Klick gedrückt: kein Rahmen entstanden | M, E, B `03` (3) |
| 04 | `marquee_fixed` | Feste Größe 100 × 50: **Klick** bei 200,150 → Rahmen 200,150–300,200 (Klickpunkt = **linke obere Ecke**); **Ziehen** 200,150 → 260,190 → 260,190–360,240 (Rahmen hängt am Mauszeiger, Lage beim Loslassen) | M, E |
| 05 | `marquee_move` | Ziehen **innerhalb** des Rahmens mit dem Auswahlrahmen-Werkzeug verschiebt nur den Rahmen (100,80 → 150,110, Pixel bleiben). **Pfeiltaste** = 1 px, **Umschalt+Pfeil** = 10 px (10 × → und 1 × Umschalt+↓ → 110,90–210,190), ebenfalls nur der Rahmen | M, E |
| 06 | `ellipse` | Ovaler Rahmen 100,80–300,220: Rand **Hart** 21 992 px; **Anti-Alias** 22 304 px, davon 476 teilweise transparent | M, E, B `06` |
| 07 | `lasso` | Freies Dreieck 100,80 → 300,80 → 200,250 → zurück: 16 926 px (100,80–295,249). Offen gelassen (ohne Rückweg) wird beim Loslassen **geschlossen**: 16 969 px | M, E, B `07` |
| 08 | `poly_lasso` | Klicks 100,80 / 300,80 + **Doppelklick** 200,250 schließt: 17 000 px (100,80–300,249). Klick auf den Startpunkt schließt ebenso (17 000 px). **Esc** vor dem Schließen verwirft das Polygon **und beendet den Bitmapmodus** (Werkzeug → Zeiger, B1 bleibt als Objekt gewählt) | M, E, J, B `08` |
| 09 | `wand_inspector` | Überschrift „Zauberstab“. **„Toleranz:“** 32 + ▼ (262–309), **„Rand:“** Hart/Anti-Alias/Weiche Kante + Wert (grau) + ▼, Kästchen **„Live-Auswahlrahmen“** ☑ | W, B `09` |
| 10 | `wand_click` | Klick 160,120 (Verlauf, #8E8881): Toleranz **0** → 6 px (nur 160,119–161,125), **32** → 5 323 px (143,40–178,200), **100** → 55 009 px (40,40–284,360) | M, E, B `10` (3) |
| 11 | `wand_add` | Umschalt-Klick auf den roten Streifen addiert: 5 323 + 6 400 = **11 723 px** | M, E, B `11` |
| 12 | `bitmap_mode` | Rahmen in einer Bitmap = Bitmapmodus ohne eigenen Rand oder Hinweis; Verlauf-Bedienfeld zeigt die Schritte. Strg+Umschalt+D: Werkzeug bleibt Auswahlrahmen, B1 bleibt gewählt (Wirkung auf den Rahmen nicht getrennt gemessen) | B `12`, J |
| 13 | `bitmap_bounds` | T03 (Bitmap 300 × 400 mit 50 px transparentem Rand): Objekt meldet **200,100, 200 × 300** (nur deckender Teil, Inspektor B 200 H 300 X 200 Y 100), Auswahlgriffe am Motiv. Ein Rahmen 100,20 → 500,480 zeigt im Inspektor B 400 H 430 X 0 Y 0 | J, W, B `13` |
| 14 | `delete_fill` | Entf in einem Rahmen macht die Pixel **transparent** (kein Füllen mit Farbe); siehe 02–11 | M, E |
| 15 | `copy_paste` | Strg+C, Strg+V aus Rahmen 100,80–200,180 → **neues Bitmap-Objekt** (ohne Namen) **an derselben Stelle** 100,80–200,180 über B1, gewählt; Werkzeug bleibt Auswahlrahmen; Bild pixelgleich | M, J, E |
| 16 | `drag_move` | Zeiger (Taste V) und Ziehen im Rahmen um +50 px verschiebt die **Pixel** (Loch 100–150 transparent, Rahmen wandert mit); **Strg+Ziehen** mit dem Auswahlrahmen-Werkzeug gleich (pixelgleicher Export). Beim Zeiger-Fall schloss das Steuerskript einen leeren Meldungsdialog (Text nicht lesbar) | M, E, B `16` |
| 17 | `live_marquee` | Live **an**: Rand nach dem Aufziehen auf Weiche Kante 10 ändern → der vorhandene Rahmen wird weich (pixelgleich mit 19). Live **aus**: vorhandener Rahmen bleibt hart (28 000 px) | M, E, B `17` (2) |
| 18 | `esc` | Esc mit Rahmen: **Bitmapmodus endet**, Werkzeug → Zeiger, B1 bleibt als Objekt gewählt; zweites Esc ändert nichts mehr | M, J |
| 19 | `feather_edge` | Weiche Kante 10, Rahmen 100,80–300,220, Entf: 35 088 px betroffen in **90,70–310,230** (10 px über den Rahmen hinaus), 21 600 voll (Innenfläche 110–290 × 90–210), 13 488 teilweise; Rest-Alpha in Zeile 150 ab x 86 (2er-Schritte): 255 255 253 243 224 196 161 116 76 45 22 7 0 | M, E |
| 20 | `cursor` | Eigener Zeiger je Werkzeug (Auswahlrahmen, Lasso); Bilder mit gehaltener Umschalt/Alt-Taste ohne Mausbewegung pixelgleich mit normal (+/−/∩-Zeiger daher nicht aufgenommen – offen) | B `20` (5) |
| 21 | `empty_doc` | T01 ohne Bitmap: Zauberstab-Klick → nichts, keine Meldung. Auswahlrahmen lässt sich aufziehen (Inspektor 200 × 140 bei 100,80), Entf → nichts, keine Meldung, kein Objekt | M, J |
| 22 | `undo` | Rahmen aufziehen = Verlaufsschritt **„Pixel auswählen“**; Zauberstab = **„Zauberstab“** | M, W |

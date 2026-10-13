# 07 `tool-fill-gradient-eyedropper` – Vergleich Original ↔ Firepixel (PAR-F-05-07, 13.10.2026)

Firepixel: Debug-Build `main` 134d195, **neu gemessen auf 0c5dcf3** (enthält PAR-X-INSP-1 f1f12f1/d38f715; Slot fp-s11), eigene
Instanz mit eigenem Profil und `--mcp`, Fenster 1600 × 1000, Design „normal“, Zoom 100 %; echtes Fenster, Brücke (`ui_input`,
Füllfarbe über Inspektor-Farbfeld + HEX auf #808080, `ui_find`, `history`, `get_document`, JSF `pathAttributes`, `export`). T02/T04
je Messung frisch, B1 bzw. R1/R2 gewählt. Bilder `docs\parity\img\tool-fill-gradient-eyedropper\`, Vergleich
`re\_testdateien\par\cmp\tool-fill-gradient-eyedropper\` (16 Paare).

| Nr | Prüfpunkt | Original | Firepixel | Bew. | Befund |
|---|---|---|---|---|---|
| 01 | bucket_inspector | „Füllwerkzeug“, Farbfeld + 4 Füllart-Knöpfe (Vollton), Rand Anti-Alias, Textur Ader 0, Toleranz 32, Auswahl füllen ☐, Transparenz beibehalten ☐, 100, Normal | gleich (auf 134d195 war „Muster“ grau, auf 0c5dcf3 aktiv) | ✅ | – |
| 02 | bucket_bitmap | Klick 160,120: 0 → 7 px; **32 → 5 553 (143,40–179,201)**; **100 → 55 872** | 0 → 6 px (160,125 fehlt); 32 → **11 028 (125,40–195,200)**; 100 → **71 982**; Klickpixel #808080 ✅ | 🔧 behoben (7a710c3), Gegenprüfung offen | PAR 07/02 |
| 03 | bucket_vector | Farbwechsel setzt gewähltes R2 **sofort**; Klick auf nicht gewähltes R1 füllt es; Schritt „Füllen“ | Farbwechsel ändert R2 **nicht** (erst Klick); Klick auf R1 ✅; Schritt **„Füllung ändern“** | 🔧 behoben (cdceac5), Gegenprüfung offen | PAR 07/03 |
| 04 | bucket_fill_selection | Rahmen 100,80–200,180: an → 10 000 px; aus → 3 580 px (133,80–169,180) | an: Hash gleich; aus: 6 900 px (116,80–185,180) | 🔧 behoben (7a710c3), Gegenprüfung offen | PAR 07/02 |
| 05 | bucket_edge | Auge 120,268: Hart 489 px; Anti-Alias 547 (+58 Mischpixel); Weiche Kte. (10) 1 595 px weich | Hart Hash gleich; **Anti-Alias und Weiche Kte. pixelgleich mit Hart** (489 px) | 🔧 behoben (7a710c3), Gegenprüfung offen | PAR 07/05 |
| 06 | gradient_inspector | ohne Füllart-Knöpfe, Farbfeld zeigt Verlauf, Toleranz grau, Auswahl füllen ☑, Rand Anti-Alias, Ader 0 | gleich (Randwert grau zeigt 10 statt 0) | ✅ | Hinweis |
| 07 | gradient_drag_bitmap | im Rahmen: 28 000 px, x 100/150/200/250/299 = 00/3F/7F/BF/FD; **ohne Rahmen: ganze Leinwand, B1 → 0,0–800,600** | im Rahmen: Δ Orig max 2 ✅; ohne Rahmen nur **B1 gefüllt, B1 bleibt 40,40–520,360** | 🔧 behoben (cdceac5), Gegenprüfung offen | PAR 07/07 |
| 08 | gradient_drag_vector | R1 linear #000@0/#FFF@100, h1 60,85, h2 140,100, **h3 75,5**; Linie, Start rund, Ende eckig | R1 pixelgleich, linear, 2 Knoten, h1/h2 gleich, **h3 45,165** (andere Seite); Griffe sichtbar (anders gezeichnet) | 🔧 behoben (cdceac5), Gegenprüfung offen | PAR 07/08 |
| 09 | gradient_types | Popup: „Verlauf:“ + Liste (12: Linear … Wellen), Farbbalken, Umkehren, Position grau, Winkel 0° | auf 0c5dcf3 gleich, 12 Einträge in derselben Reihenfolge (auf 134d195 schloss der Listenklick das Popup – von f1f12f1 behoben) | ✅ | – |
| 10 | eye_inspector | Auswahl 1 Pixel / 3x3 / 5x5 Durchschnitt, Farbformat HEX / RGBA | gleich (Lage gleich) | ✅ | – |
| 11 | eye_stroke_fill | Füllfeld aktiv: Rot → Füllung #FF0000; nach Klick aufs Strich-Symbol: Grün → **Strich** #00A000; R1 gewählt + Klick R2 → **R1 #BFBFBF**; Schritt „Füllfarbe“ | Füllung #FF0000 ✅; mit Strich aktiv wird trotzdem die **Füllung** #00A000; R1 gewählt: Füllfarbfeld #BFBFBF, **R1 bleibt #808080**, kein Schritt | 🔧 behoben (cdceac5), Gegenprüfung offen | PAR 07/11 |
| 12 | eye_average | **kein Mittelwert**: linkes oberes Pixel des Fensters (3x3 bei 360 → #FFFFFF, bei 361 → #FF0000; 5x5 bei 361 → #FFFFFF; 100,100 → #5462BA) | echter Mittelwert: 3x3 bei 359/360/361 → #FFAAAA/#FF5555/#FF0000; 5x5 bei 361 → #FF3333, 100,100 → #5663B8 | ❌ | PAR 07/12, Frage 1 |
| 13 | eye_outside | Klick neben die Leinwand: Farbe bleibt; leere Leinwand → #FFFFFF | neben der Leinwand (1300,400) → **#FFFFFF**; leere Leinwand ✅ | 🔧 behoben (cdceac5), Gegenprüfung offen | PAR 07/13 |
| 14 | cursor | eigene Zeiger | nicht aufgenommen | – | offen |
| 15 | texture_none | „Ader“ mit 0 % (Füllwerkzeug und Verlauf) | gleich | ✅ | – |
| 16 | undo | Farbeimer Bitmap/Vektor „Füllen“; Verlauf im Rahmen „Füllen“; Verlauf auf Vektor „Füllung bearbeiten“; Pipette „Füllfarbe“ | Farbeimer Bitmap „Füllen“ ✅ (Text doppelt kodiert, bekannt); Vektor **„Füllung ändern“**; Verlauf Bitmap und Vektor **„Verlauf“**; Pipette **kein Schritt** | 🔧 behoben (cdceac5), Gegenprüfung offen | PAR 07/16 |

**Zählung:** ✅ 5 · ❌ 10 · 🎨 0 · offen 1 (Mauszeiger).

## Fragen an den Nutzer

1. (12) Das Original mittelt bei „3x3/5x5 Durchschnitt“ nicht, sondern nimmt das linke obere Pixel des Fensters (wirkt wie ein
   Fehler des Originals). Nachbauen oder Firepixels echten Mittelwert behalten (dann 🎨)?

## Hinweise / Annahmen

- 02/04 und tool-bitmap-selection/10 (Zauberstab) haben dieselbe Toleranzabweichung – vermutlich gemeinsame Farbabstandsfunktion.
- 08: Griffe in Firepixel blau/gefüllt statt Kreis hohl/Quadrat schwarz – Gestaltung, nicht bewertet; gezählt ist nur h3.
- Füllfarbe des Profils war #999999; für alle Messungen per Farbfeld auf #808080 gesetzt (wie im Original).
- Kodierung „FÃ¼llen“ in `bitmap.rs` ist schon als Befund (PAR-X-INSP-1) erfasst.

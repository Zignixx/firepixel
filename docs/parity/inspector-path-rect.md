# 17 `inspector-path-rect` – Vergleich Original ↔ Firepixel (PAR-F-16-18, 13.10.2026)

Firepixel: Debug-Build `main` 180852a (Slot fp-s3), eigene Instanz, `--mcp`, 1600 × 1000, Design „normal“, Zoom 100 %;
echtes Fenster, Brücke. T04-Kopie (`work\insp_t04.fw.png`) + per JSF `addNewRectanglePrimitive` 400,420–520,500 „RP“ wie im
Original. Feldeingaben echt (Klick, Strg+A, Wert, Enter). Bilder `docs\parity\img\inspector-path-rect\`, Vergleich
`re\_testdateien\par\cmp\inspector-path-rect\` (18 Paare).

| Nr | Prüfpunkt | Original | Firepixel | Bew. | Befund |
|---|---|---|---|---|---|
| 01 | rect_layout | „Rechteck“, Name RP, B 120 H 80 X 400 Y 420, Rundung 0 %, Füllung einfarbig, Strich ohne → Größe/Rand/Textur grau, Position „mittig“ | gleich (alle Felder, Lagen ±3 px, gleiche Grau-Zustände) | ✅ | – |
| 02 | path_layout | P1 „Pfad“, keine Rundung, Füllfelder grau, Strich 1 px aktiv, „Pinselstrich bearbeiten“ + Speichern/**Löschen aktiv**; 5 Pfadoperationen aktiv | gleich, aber **„Pinselstrich löschen“ grau** und bei 1 Pfad 4 Pfadoperationen grau; Strichname „1-Pixel hart“ statt Rohname „bn_1-Pixel Hard“ (Übersetzung, gewollt) | ❌ | PAR 17/02 |
| 03 | ellipse_layout | E1 „Pfad“, B 100 H 70 X 50 Y 170, Füllung aktiv | gleich | ✅ | – |
| 04 | xywh | B 150, H 100, X 410, Y 430 → 410,430–560,530 | 410,430–560,530; je Feld ein Schritt „Position und Größe“ | ✅ | – |
| 05 | roundness | 20 % → Einheit px zeigt **10**; 10 px → `roundness` 10, `mode` „exact“; Liste „%/px“ nach unten | nach B/H-Änderung zeigt px **8** (rechnet mit alter Größe 120 × 80); 10 px → `roundness` 0.25, `mode` „percentage“; Liste „%/px“ nach oben | ❌ | PAR 17/05 |
| 06 | opacity | 50 → 50,196 | 50 → 50.2 | ✅ | – |
| 07 | blend | 46 Modi + 9 Trenner, Reihenfolge s. `.orig.md`; öffnet nach oben, ganze Namen lesbar | gleiche 46 Modi, gleiche Reihenfolge und Trenner, nach oben, mit Bildlauf; **Namen abgeschnitten** („Farbig nachbelic…“, „Negativ multipliz…“, „Ineinanderkopier…“) | ❌ | PAR 17/07 |
| 08 | stroke_position | innen/mittig/außen: rot ab x 50/46/42, je 8 px; Grenzen 50,170–150,240 / 46,166–154,244 / 42,162–158,248 | Pixel gleich (ab x 50/46/42, je 8 px); **JSF `pixelRect` bleibt 50,170–150,240** bei mittig/außen | ❌ | PAR 17/08 |
| 09 | fill_over_stroke | nicht bedienbar (verborgen), `fillOnTop` false, kein `setFillOnTop` | kein Kästchen in Füll-/Strichpopup, `fillOnTop` false, `setFillOnTop` undefined | ✅ | – |
| 10 | path_ops | 5 Knöpfe mit Tooltips, „Kombinieren“ grau | R1+R2: 5 Knöpfe aktiv, gleiche Tooltips, „Kombinieren“ als Beschriftung | ✅ | – |
| 11 | style_list | nur „Kein Stil“; „Neuer Stil“ aktiv, übrige grau | gleich | ✅ | – |
| 12 | filters_plus | Ohne · Optionen ▸ · Abgeflachte Kante und Relief ▸ · Andere ▸ · Farbe anpassen ▸ · Rauschen ▸ · Scharfzeichnen ▸ · Schatten und Glühen ▸ · Weichzeichnen ▸ · Trenner · Fremd-Live-Effekte; Breite 272 | **„Ohne“ fehlt**, kein Trenner, Namen „Abschrägung und Prägung“, „Anpassen der Farbe“, „Schärfen“, Reihenfolge danach; letzter Eintrag nennt ein Fremdprogramm „(noch nicht verfügbar)“; Menü ≈ 594 px breit | ❌ | PAR 17/12 |
| 13 | name_field | „RECHTECK_A“ + Enter → Name, Ebenen zeigt ihn | gleich | ✅ | – |
| 14 | thumbnail | 41 × 37 links neben Art/Name | gleich | ✅ | – |
| 15 | multi | „2 Objekte“, 250/70/50/50, Deckkraft leer, „Neuer Stil“ grau | gleich | ✅ | – |
| 16 | scale_strokes | E1 8 px außen, B 100 → 200: Strich 8 → **11**, Grenzen 39,159–261,251 | Strich bleibt **8**, Pfad 50–250 | ❌ | PAR 17/16 |
| 17 | field_tooltips | u. a. Farbfeld zeigt Hex („#808080“), „Füllungsrand“, „Pinselstrichkategorien“; ohne Tooltip: Symbole, ▼, Vorschaubild | vorhanden: Proportionen, Füllart-Knöpfe, Verlauf – Dither, Strichposition, Live-Filter +/–, Stil-Knöpfe, Pfadoperationen; keine Tooltips an Symbolen ✅; **fehlen: Hex am Füll-/Strichfarbfeld, „Füllungsrand“, „Pinselstrichkategorien“** | ❌ | PAR 17/17 |
| 18 | disabled | gesperrte Ebene: Objekt nicht wählbar, Dokument-Inspektor | gleich | ✅ | – |

**Zählung:** ✅ 11 · ❌ 7 · 🎨 0.

## Fragen / Annahmen

- 02: Strichname übersetzt statt Rohname des Originals – als Verbesserung gewertet (kein Befund); Frage an den Nutzer, ob
  der Rohname gewünscht ist.
- 12: Der Fremdprogramm-Name im letzten Eintrag ist keine Neutralitätsverletzung im Sinne §1a, sollte aber wie im Original
  hinter einem Trenner stehen.
- 05: Ursache vermutlich die Ausgangsgröße des Grundobjekts (Rundung relativ zur Erzeugungsgröße statt zur aktuellen Größe).

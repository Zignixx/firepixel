# 18 `inspector-fill` – Vergleich Original ↔ Firepixel (PAR-F-16-18, 13.10.2026)

Firepixel: Debug-Build `main` 180852a (Slot fp-s3), eigene Instanz, `--mcp`, 1600 × 1000, Design „normal“, Zoom 100 %;
echtes Fenster, Brücke. T04-Kopie, Objekt R1 per Leinwandklick gewählt. Ergebnisbilder = Export-Ausschnitt 30,30–170,140
(MCP `export`, wie Original). Verlaufsarten für 03 per `run_command pi.fill {shape}` gesetzt (Liste im Popup nicht bedienbar,
siehe 02). Bilder `docs\parity\img\inspector-fill\`, Vergleich `re\_testdateien\par\cmp\inspector-fill\` (33 Paare, 6 gleich).

| Nr | Prüfpunkt | Original | Firepixel | Bew. | Befund |
|---|---|---|---|---|---|
| 01 | category | 4 Knöpfe (Tooltips); „Verlauf“/„Muster“ stellen um **und** öffnen sofort das Popup | 4 Knöpfe, gleiche Tooltips; „Verlaufsfläche“ stellt um, **öffnet kein Popup**; „Muster“ öffnet nur eine Musterliste, stellt erst nach Wahl um | ❌ | PAR 18/01 |
| 02 | gradient_list | 12 Arten: Linear, Radial, Rechteck, Kegel, Kontur, Glanz, Supernova, Falten, Ellipse, Streifen, Kleine Wellen, Wellen (fc_Linear … fc_Waves) | **Klick auf die Liste im Popup schließt das Popup** (Liste nicht wählbar). Laut Daten 11 Arten, **Kontur fehlt**, Namen/Reihenfolge anders (Kegelförmig, Satin, Lamellen, Ellipsenförmig, Rechteckig, Balken, Kräuseln), JSF-Namen fc_Cone/fc_Starburst/fc_Rectangle/fc_Ripples | ❌ | PAR 18/02 |
| 03 | gradient_result | 12 Exporte, Standardverlauf (oben weiß → unten #808080, 270°), 51–128 Farben | an R1 (Objekt aus FW-PNG) **alle Arten einfarbig** (1 Farbe): Verlaufsgriffe 0,0/0,0; an neu gezeichnetem Rechteck Verlauf da, aber Vorgabe Füllfarbe → Strichfarbe **waagerecht**; Kontur ohne Bild | ❌ | PAR 18/03 |
| 04 | gradient_editor | 210 × 155: „Verlauf:“ Liste, Leiste mit Deckkraft-/Farbstopps, Farbrad, „Umkehren“, Verschieben-Feld %, Winkel 270 °; Klick auf Farbstopp → Farbpopup (Raster, HEX, RGB) | ≈ 254 × 95: Liste, Knopf „Umkehren“, Leiste mit Stopps; **ohne** Beschriftung „Verlauf:“, Farbrad, Verschieben, **Winkel**; Farbstopp-Klick zeigt Zeile „Farbe: #808080“ + 12 Farbfelder + „Löschen“ statt Farbpopup | ❌ | PAR 18/04 |
| 05 | gradient_stops | Klick unter die Leiste → neuer Stopp (2 → 3); Stopp nach unten herausziehen → gelöscht | Klick → 3 Stopps ✅; Herausziehen **löscht nicht** (Stopp landet bei 0, doppelter Stopp); Löschen nur per Knopf | ❌ | PAR 18/05 |
| 06 | gradient_presets | keine Vorgabenliste | keine | ✅ | – |
| 07 | gradient_handles | Zeiger: Linie durch die Mitte, Punkt oben, Quadrat unten | **keine Griffe** (auch nicht bei funktionierendem Verlauf, nur Auswahlrahmen) | ❌ | PAR 18/07 |
| 08 | pattern_list | Muster-Popup: Liste, Vorschau 250 × 123, Winkel; 60 Muster deutsch alphabetisch („Baumwolle“ … „Ölfarbe 3“) + „Andere...“; Vorgabe „Baumwolle“ | Liste 60 + „Andere...“, aber **englische Namen** („Berber Rug“ … „Wood 3“); **kein Muster-Popup** (Farbfeld öffnet bei Muster das Farbpopup), keine Vorschau, kein Winkel | ❌ | PAR 18/08 |
| 09 | pattern_result | „Baumwolle“: Mittel #8c8c8c, 149 Farben | an R1 **einfarbig #8d8d8d** (1 Farbe) | ❌ | PAR 18/03 |
| 10 | web_dither | keine Web-Dither-Kategorie | keine (4 Knöpfe) | ✅ | – |
| 11 | edge | Hart/Anti-Alias/Weiche Kte., Wert nur bei Weiche Kte.; Pixel hart = geglättet; Weich 10: x45 #e9, x49 #c6, x50 #ba; JSF `antialiased` + feather 10 | Liste und Pixel **identisch** (MD5 gleich); aber zurück auf „Anti-Alias“ **bleibt Weichheit 10 wirksam**; JSF meldet `feathered` | ❌ | PAR 18/11 |
| 12 | texture_list | bildschirmhoch, alphabetisch: Ader, Argyle 01, Argyle 02, Balken, Chiffon, Diagramm, DNA, Faser, Filz, Gips, Gras, Herz 01 … | bildschirmhoch, alphabetisch, aber **andere Namen**: „Rautenmuster 01/02“ statt „Argyle 01/02“, „Putz“ statt „Gips“, dazu „Gitter“ an anderer Stelle | ❌ | PAR 18/12 |
| 13 | texture_amount | Ader 0/50/100: #808080 / #ababab / #d6d6d6 | **pixelgleich** (MD5 gleich) | ✅ | – |
| 14 | texture_transparent | aktiv erst ab Menge > 0; Klick bei 100 % ändert Export auf Weiß nicht (Annahme, nicht gegengeprüft) | aktiv erst ab Menge > 0 ✅; Klick → Export heller (Mittel #eb statt #d6) | 🎨 | Frage |
| 15 | texture_other | „Andere…“ am Listenende | vorhanden | ✅ | – |
| 16 | fill_options_popup | kein gemeinsames Popup, Farbfeld öffnet je Kategorie | kein gemeinsames Popup (Muster: siehe 08) | ✅ | – |
| 17 | color_swatch | Farbpopup: Raster, Vorschau, HEX, Deckkraft + ▼, RGB, Farbrad | gleich | ✅ | – |
| 18 | none_disabled | grau: Rand + Wert, Textur, Menge, Transparent, Verlauf – Dither; Farbfeld/Knöpfe aktiv | gleich | ✅ | – |
| 19 | script_name | Textur per JSF weder lesbar noch setzbar | lesbar: `pathAttributes.fillTexture` = {name:"Vein"}; `textureBlend` 500 bei Menge 50 | 🎨 | Frage |
| 20 | undo | je Änderung ein Schritt **„Füllen“**; Rechteck zeichnen = „Rechteck“ | je Änderung ein Schritt **„Füllung ändern“**; „Rechteck“ ✅ | ❌ | PAR 18/20 |
| 21 | tool_default | Rechteck-Werkzeug: Kopf „Rechteck“, B/H/X/Y leer, Füllblock wie am Objekt; ohne Strichposition, Filter, **Pfadoperationen**, Stil | Kopf, leere Felder, ohne Strichposition/Filter/Stil ✅; **Pfadoperationen sichtbar**, Knopf **„Muster“ grau** | ❌ | PAR 18/21 |
| 22 | save_style | neues Rechteck übernimmt letzte Füllung (fc_Waves, Textur 50) | neues Rechteck **einfarbig #999999, Textur 0** | ❌ | PAR 18/22 |

**Zählung:** ✅ 7 · ❌ 13 · 🎨 2.

## Fragen / Annahmen

- 14: Im Original war die Wirkung von „Transparent“ auf weißem Grund nicht messbar und der Klick nicht gegengeprüft. Firepixel
  macht die Textur durchscheinend. Frage: im Original auf farbigem Grund nachmessen, bevor geändert wird.
- 19: Firepixel kann mehr als das Original (Texturname lesbar) – kein Rückbau. Offen: Skala von `textureBlend` (500 bei 50 %)
  im Original nicht gemessen; beim Beheben von 18/11 mitprüfen.
- 03/09: Gemeinsame Ursache vermutlich fehlende Verlaufs-/Mustervektoren (`fillHandle1/2` = 0,0) an Pfaden aus FW-PNG-Dateien,
  wenn die Füllart im Inspektor gewechselt wird. Bilder der 11 Arten zeigen deshalb Flächen; Formvergleich je Art nach dem Beheben
  (PAR-R) mit gleichen Stopps wie im Original.

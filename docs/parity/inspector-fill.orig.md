# 18 `inspector-fill` – Füllung: Aufnahme des Originals (PAR-O-16-18, 13.10.2026 ≈ 06:50–07:20)

T04 als Arbeitskopie `re\_testdateien\par\work\insp_t04.fw.png`, Objekt **R1** (Pfad 50,50–150,120, #808080, ohne
Strich). Fenster 1600 × 1000, Zoom 100 %. Referenzbilder `re\_testdateien\par\ref\inspector-fill\` (+ `INDEX.md`),
Exporte und Kennzahlen `re\_testdateien\par\px\inspector-fill\` (`stats.txt`: MD5, Mittelwert und Stichpunkte im
Bereich 50,50–150,120). Messart **W** Steuerelemente, **J** JSF, **E** Export + Pixel, **B** Bild. Lage der Felder
siehe `inspector-path-rect.orig.md` „Aufbau“, Block 2.

## Prüfpunkte

| Nr | Kürzel | Original | Art |
|---|---|---|---|
| 01 | `category` | **keine Liste**, sondern 4 Umschaltknöpfe neben dem Farbfeld: „Keine Füllung“, „Einfarbige Füllung“, „Verlaufsfläche“, „Muster“ (Tooltips). Klick auf „Verlauf“/„Muster“ stellt um **und** öffnet sofort das zugehörige Popup | W, B `01` |
| 02 | `gradient_list` | Liste „Verlauf:“ im Verlaufs-Popup, **12** Arten: Linear, Radial, Rechteck, Kegel, Kontur, Glanz, Supernova, Falten, Ellipse, Streifen, Kleine Wellen, Wellen (JSF: fc_Linear, fc_Circular, fc_Rectangular, fc_Conical, fc_ContourGrad, fc_Satin, fc_Pinch, fc_Folds, fc_Elliptical, fc_Bars, fc_Ripple, fc_Waves) | W, B `02` |
| 03 | `gradient_result` | je Art Export (Standardverlauf „cn_BlackWhite“, 2 Farb- + 2 Deckkraftstopps, Winkel 270°), 12 Bilder; Kennzahlen in `stats.txt` | E, B `03` (12) |
| 04 | `gradient_editor` | Verlaufs-Popup (eigenes Fenster 210 × 155 über dem Feld): „Verlauf:“ Typliste; Verlaufsleiste 180 × 34 mit Deckkraftstopps oben und Farbstopps unten; Farbrad-Knopf; Knopf „Umkehren“; Verschieben-Symbol + Feld (grau) + „%“; Winkel-Symbol + Feld „270“ + „°“. Klick auf einen Farbstopp öffnet das Farbpopup (Raster, HEX, RGB) | W, B `04` (2) |
| 05 | `gradient_stops` | Klick unter die Leiste (Mitte) → **neuer Farbstopp** (2 → 3); Stopp nach unten aus dem Popup ziehen → gelöscht (3 → 2) | J, B `05` |
| 06 | `gradient_presets` | **keine Vorgabenliste** im Verlaufs-Popup gefunden (Steuerelemente vollständig wie 04) | W |
| 07 | `gradient_handles` | R1 mit Linearverlauf gewählt: Griffe auf der Leinwand – Linie senkrecht durch die Mitte, Startpunkt (runder Punkt) oben, Ende (Quadrat) unten | B `07` |
| 08 | `pattern_list` | Muster-Popup: „Muster:“ Liste, Vorschau 250 × 123, Winkel „0“ °. Liste 60 Einträge (Baumwolle … Ölfarbe 3, alphabetisch), Trennlinie, „Andere...“. Vorgabe beim Umschalten: „Baumwolle“ (JSF „Cotton“) | W, B `08` (2) |
| 09 | `pattern_result` | „Baumwolle“ (statt „Ader“, das es als Muster nicht gibt): Mittel #8c8c8c, 149 Farben | E, B `09` |
| 10 | `web_dither` | **nicht vorhanden**: keine Web-Dither-Kategorie unter den 4 Knöpfen; nur Knopf „Verlauf - Dither“ (Verläufe) | W |
| 11 | `edge` | Liste „Hart|Anti-Alias|Weiche Kte.“ (öffnet nach unten), Wert nur bei „Weiche Kte.“ aktiv. Export: Hart = Anti-Alias (achsparallel, gleiche MD5); Weiche Kante 10 → Rand weich ausgeblendet (x 45: #e9e9e9, x 49: #c6c6c6, x 50: #bababa). JSF meldet bei Weiche Kante `edgeType` „antialiased“ + `feather` 10 | W, E, B `11` (4) |
| 12 | `texture_list` | Texturliste klappt **bildschirmhoch** auf (Rollpfeil oben), alphabetisch nach Anzeigename: Ader, Argyle 01, Argyle 02, Balken, Chiffon, Diagramm, DNA, Faser, Filz, Gips, Gras, Herz 01 … Raster 7 …; Liste breiter als das Feld (≈ 130 px) | W, B `12` |
| 13 | `texture_amount` | Textur „Ader“, Menge 0 → identisch ohne Textur (#808080); 50 → Mittel #ababab; 100 → #d6d6d6 (hellt auf) | E, B `13` (3) |
| 14 | `texture_transparent` | „Transparent“ erst ab Menge > 0 aktiv; nach Klick (Menge 100) Export gleich wie ohne (gleiche MD5) – Wirkung auf weißem Grund nicht messbar (Annahme: Klick übernommen, nicht gegengeprüft) | W, E |
| 15 | `texture_other` | Ende der Liste hier nicht im Bild; laut früherer Messung (BEFUNDE FV-10, Liste am Rechteck-Werkzeug) Trennlinie + „Andere…“ am Ende | W (früher) |
| 16 | `fill_options_popup` | **kein** gemeinsames Füllungsoptionen-Popup: das Farbfeld öffnet je Kategorie Farbpopup (einfarbig), Verlaufs-Popup oder Muster-Popup | W |
| 17 | `color_swatch` | einfarbig: Farbpopup über dem Feld (275 × 302): Raster, Vorschau, HEX, **Deckkraft** (Feld + ▼), RGB, unten Knopf „Ohne“ und Farbrad | B `17` |
| 18 | `none_disabled` | „Keine Füllung“: grau werden Rand-Liste + Wert, Textur-Liste, Menge, „Transparent“, „Verlauf - Dither“; Farbfeld und Kategorieknöpfe bleiben aktiv | W, B `18` |
| 19 | `script_name` | Füllobjekt per JSF hat **keine** Textur-Eigenschaft (nur `textureBlend`); Name weder lesbar noch per `texture:{name}` setzbar (bleibt null) | J |
| 20 | `undo` | jede Füllungsänderung im Inspektor = ein Verlaufsschritt **„Füllen“**; Rechteck zeichnen = „Rechteck“ | B `20` |
| 21 | `tool_default` | Rechteck-Werkzeug ohne Auswahl: Kopf „Rechteck“, B/H/X/Y leer, **Füllblock identisch** zum Objekt; ohne Strichpositionsknöpfe, Filter, Pfadoperationen, Stil; eigene Deckkraft/Mischmodus-Felder | W, B `21` |
| 22 | `save_style` | nach R1 = Verlauf „Wellen“ mit Textur 50 neu gezeichnetes Rechteck (Leinwand, Rechteck-Werkzeug) hat **dieselbe** Füllung (fc_Waves, Textur 50) | J |

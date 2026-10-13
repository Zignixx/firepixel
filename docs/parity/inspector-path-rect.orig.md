# 17 `inspector-path-rect` – Rechteck, Pfad, Ellipse: Aufnahme des Originals (PAR-O-16-18, 13.10.2026 ≈ 05:25–06:05)

T04 als Arbeitskopie `re\_testdateien\par\work\insp_t04.fw.png`; dazu per JSF ein **Rechteck-Grundobjekt RP**
(400,420–520,500, #808080, ohne Strich), weil R1/R2 in T04 einfache Pfade sind (Inspektor „Pfad“, ohne Rundung).
Fenster 1600 × 1000, Zoom 100 %. Referenzbilder `re\_testdateien\par\ref\inspector-path-rect\` (+ `INDEX.md`), Pixel
`re\_testdateien\par\px\inspector-path-rect\`. Messart **W** Steuerelemente, **J** JSF, **E** Export + Pixel, **B** Bild.
Feldeingaben echt (Klick ins Feld, Pos1/Umschalt+Ende, Wert, Enter). Leinwandklicks gehen (siehe `re\25-leinwand-fehler.md`).

## Aufbau (gemeinsam für 01–03)

Bedienfeld 70,890–1324,995, vier Blöcke. **Block 1** (74–190): Vorschaubild, **Art** („Rechteck“ / „Pfad“), **Name**
(Feld 119–188 × 917–937); darunter B (100–136 × 946) / H (968) links, X (152–188 × 946) / Y (968) rechts, links davor
Knopf „Proportionen beibehalten“ (72–86 × 944–988). **Block 2 Füllung** (195–373): Eimer-Symbol, Farbfeld, 4 Knöpfe
„Keine Füllung“ / „Einfarbige Füllung“ / „Verlaufsfläche“ / „Muster“ (276–372 × 894–914); „Rand:“ Liste
„Hart|Anti-Alias|Weiche Kte.“ (245–328) + Wert (329–358, nur bei Weiche Kante aktiv) + ▼; „Textur:“ Liste (242–325) +
Menge (326–359) + ▼; „Transparent“ (245–338 × 968–984); Knopf „Verlauf - Dither“ (348–372 × 967–987).
**Block 3 Strich** (381–575): Stift-Symbol, Farbfeld, Größe (429–458) + ▼, Strichart-Liste (477–575); 3 Knöpfe
„Pinselstrich innen / mittig / außen ausrichten“ (384–456 × 920–940); „Rand:“ Vorschau + Wert (531–560) + ▼;
„Textur:“ Liste (431–527) + Menge (531–560) + ▼; **nur beim Rechteck** statt der Pinselknöpfe: „Rundung:“ Wert
(483–512 × 968) + ▼ + Einheit „%|px“ (532–576); bei Pfaden stattdessen „Pinselstrich bearbeiten“ + 2 Symbolknöpfe.
**Block 4** (585–770): Deckkraft-Symbol, Deckkraft (600–629) + ▼, Mischmodus (648–770); „Filter:“ + „+“ + „–“ (grau),
Filterliste (585–738 × 944–991). **Block 5** (789–933): 5 Pfadoperationen, „Kombinieren“, Stil-Liste, 5 Stil-Knöpfe.

## Prüfpunkte

| Nr | Kürzel | Original | Art |
|---|---|---|---|
| 01 | `rect_layout` | Art „Rechteck“, Name „RP“, B 120 H 80 X 400 Y 420; Rundung 0, Einheit „%“; Füllung „Einfarbig“ gedrückt; Strich „Ohne“ → Größe/Rand/Strichtextur grau, Strichposition „mittig“ gedrückt | W, B `01` |
| 02 | `path_layout` | P1: Art „Pfad“, **keine** Rundung; ohne Füllung → Rand-Liste, Füll-Textur, Menge, Transparent grau; Strich „bn_1-Pixel Hard“ 1 px aktiv, „Pinselstrich bearbeiten“ + Speichern/Löschen aktiv | W, B `02` |
| 03 | `ellipse_layout` | E1 ist ein **Pfad** (Art „Pfad“), Felder wie 02, Füllung aktiv; B 100 H 70 X 50 Y 170 | W, B `03` |
| 04 | `xywh` | B 150, H 100, X 410, Y 430 nacheinander getippt → RP 410,430–560,530 (alle vier übernommen, Enter je Feld) | J |
| 05 | `roundness` | Rundung 20 (%) → Wechsel der Einheit auf „px“ zeigt **10** (= 20 % der halben kürzeren Seite 100/2); dann 10 px getippt → `roundness` 10, `mode` „exact“. Einheitenliste „%|px“ klappt nach unten | J, W, B `05` |
| 06 | `opacity` | Deckkraft 50 → gespeichert 50,196 (= 128/255) | J |
| 07 | `blend` | 55 Einträge mit Trennern „-----“: Normal, Durchschnitt, Sprenkeln · Abdunkeln, Multiplizieren, Farbig nachbelichten, Umgek. farb. nachb., Weich nachbelichten, Linear nachbelichten · Aufhellen, Negativ multiplizieren, Farbig abwedeln, Umgek. farb. abw., Weich abwedeln, Linear abwedeln · Ineinanderkopieren, Weiches Licht, Unscharfes Licht, Hartes Licht, Strahlendes Licht, Lineares Licht, Lichtpunkt, Hart mischen · Differenz, Ausschluss, Negation · Farbton, Sättigung, Farbe, Helligkeit, Rot, Grün, Blau · Spiegeln, Glühen, Abkühlen, Erwärmen · Additiv, Subtraktiv, Subtrahieren, Interpolation, Stempel, XOR · Umkehren, Tönung · Radieren. Liste öffnet **nach oben** | W, B `07` |
| 08 | `stroke_position` | E1, Strich 8 px: innen → Grenzen 50,170–150,240, rot x 50–57 in Zeile 205; mittig → 46,166–154,244, rot 46–53; außen → 42,162–158,248, rot 42–49 (je genau 8 px) | J, E, B `08` (3) |
| 09 | `fill_over_stroke` | **nicht bedienbar**: Kontrollkästchen „Füllung über Pinselstrich“ existiert in Füll- und Strich-Farbpopup, ist aber **verborgen**; JSF `pathAttributes.fillOnTop` nur lesbar (false), `setFillOnTop` gibt es nicht | W, J |
| 10 | `path_ops` | 5 Knöpfe (Tooltips): „Normal“ (gedrückt), „Hinzufügen/Vereinigen“, „Subtrahieren/Ausstanzen“, „Überschneiden“, „Zuschneiden“; „Kombinieren“ bei 1 und bei 2 gewählten Pfaden grau | W, B `10` |
| 11 | `style_list` | Liste nur „Kein Stil“; Knöpfe „Neuer Stil“ (aktiv), „Stil neu definieren“, „Abweichungen löschen“, „Verknüpfung zu Stil aufheben“, „Stil löschen“ (grau) | W, B `11` |
| 12 | `filters_plus` | Menü: Ohne · Optionen ▸ · Abgeflachte Kante und Relief ▸ · Andere ▸ · Farbe anpassen ▸ · Rauschen ▸ · Scharfzeichnen ▸ · Schatten und Glühen ▸ · Weichzeichnen ▸ · Trennlinie · Eintrag „…-Live-Effekte“ (Effekte eines Fremdprogramms, Name hier neutralisiert; Detail Einheit 25) | B `12` |
| 13 | `name_field` | Name „RECHTECK_A“ + Enter → Objektname gesetzt, Ebenen-Bedienfeld zeigt ihn sofort | J, B `13` |
| 14 | `thumbnail` | Vorschaubild 41 × 37 links neben Art/Name | B `14` |
| 15 | `multi` | R1+R2: Kopf „2 Objekte“ (kein Namensfeld), B/H/X/Y = gemeinsame Grenzen 250/70/50/50, **Deckkraft leer** (100 vs. 50), Stil-Liste leer, „Neuer Stil“ grau | W, B `15` |
| 16 | `scale_strokes` | E1 Strich 8 außen, B 100 → 200 im Inspektor: Strich wird **mitskaliert** (Größe 8 → 11), Grenzen 39,159–261,251 | J |
| 17 | `field_tooltips` | Tooltips: „Proportionen beibehalten“, Farbfeld zeigt Hex („#808080“), „Keine Füllung“, „Einfarbige Füllung“, „Verlaufsfläche“, „Muster“, „Füllungsrand“, „Transparente Textur“, „Verlauf - Dither“, „Pinselstrichkategorien“, „Pinselstrich innen/mittig/außen ausrichten“, „Pinselstrich bearbeiten“, „Benutzerdefinierten Pinselstrich speichern/löschen“, „Live-Filter hinzufügen oder Vorgabe wählen“, „Aktuell ausgewählten Live-Filter löschen“, Stil-Knöpfe wie 11. **Ohne** Tooltip: Eimer-/Stift-/Deckkraft-Symbol, ▼-Knöpfe, Vorschaubild | W, B `17` (2) |
| 18 | `disabled` | Ebene gesperrt (JSF) → Objekt lässt sich nicht wählen, Inspektor zeigt das Dokument (keine grauen Objektfelder) | J, W, B `18` |

## Weitere Beobachtungen

- Während einer Folge von Feldeingaben liefert JSF teils veraltete Werte, bis die Auswahl wechselt; Endwerte oben sind
  nach Auswahlwechsel gelesen.

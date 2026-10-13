# 14 `tool-web` – Hotspots, Segment, Polygonsegment, Ein-/Ausblenden: Aufnahme des Originals (PAR-O-14-15-19, 13.10.2026 ≈ 13:45–14:05)

T06 als Arbeitskopie `re\_testdateien\par\work\p1415_t06.fw.png`, je Messreihe frisch mit `fw.openDocument(url)`. Fenster 1600 × 1000, Zoom 100 %,
Leinwand bei 297,183. **Echte Maus** (Werkzeug aus der Ausklappliste per Klick, Aufziehen, Klicks), Objekte danach per Skript (alle Eigenschaften der
Web-Objekte), Verlaufsnamen aus dem Bearbeiten-Menü, Menüs per Klick. Bilder `re\_testdateien\par\ref\tool-web\` (+ `INDEX.md`), Hashes und
Überlagerungsfarben `re\_testdateien\par\px\tool-web\pixels.csv` (Web-Objekte erscheinen nicht im PNG-Export, daher Bildschirmhashes).
Art: M = echte Eingabe, J = Skriptwert, W = Fenstertext, B = Bild.

## Prüfpunkte

| Nr | Kürzel | Original | Art |
|---|---|---|---|
| 01 | `hotspot_rect` | Ausklappliste Hotspot (3 Einträge, gewählter mit Haken): Hotspot-Rechteck (J), Hotspot-Kreis (J), Hotspot-Polygon (J). Ziehen 300,300 → 400,360 → `Hotspot` `shape` rectangle, **genau 300,300–400,360**, Farbe **#00FFFF**, `name` null, URL/Alt/Ziel null. Neues Objekt wird gewählt | M, J, B `04` |
| 02 | `hotspot_circle` | Ziehen 450,300 → 530,380 → `shape` circle, Grenzen **450,300–536,386** (86 × 86 statt 80 × 80) | M, J |
| 03 | `hotspot_polygon` | Klicks 600,300 / 700,300 / 650,380 → `shape` **polyline**, Kontur offen mit 3 Knoten, 600,300–700,380; Objekt entsteht mit dem ersten Klick, weitere Klicks fügen Punkte an (Ende durch Werkzeugwechsel) | M, J, B `03` |
| 04 | `slice_draw` | Ausklappliste Segment (2): Segment (K), Polygon-Segment (K). Ziehen 300,450 → 450,550 → `SliceHotspot` 300,450–450,550, Farbe **#00FF00**, `sliceKind` image; **rote Segmenthilfslinien** an allen Segmentkanten über die ganze Leinwand | M, J, B `04` |
| 05 | `slice_auto_name` | Objektname bleibt null; Inspektor zeigt den automatischen Namen **„p1415_t06.fw_r6_c5“** (Dateiname ohne „.png“ + `_r<Zeile>_c<Spalte>` des Segmentrasters) | W |
| 06 | `poly_slice` | Klicks 520,450 / 620,450 / 580,550 → `SliceHotspot` `shape` polyline, 520,450–620,550, Kontur offen, 3 Knoten | M, J, B `06` |
| 07 | `slice_guides_drag` | Zeiger über Segmenthilfslinie → Verschiebezeiger (Hotspot 15,14); Hilfslinie y 450 → 420 ziehen: Segmentkante folgt (300,**420**–450,550); Schritt **„Segment-Hilfslinie verschieben“** | M, J, B `07` (3) |
| 08 | `overlay_dim` | Segmente halbtransparent grün (Weiß → #AFFFAF ≈ 31 %), Hotspots cyan (Weiß → #66FFFF ≈ 60 %). Fläche außerhalb wird **nicht** aufgehellt (Weiß bleibt #FFFFFF, Gruppe G #3366CC unverändert) | B `08`, pixels.csv |
| 09 | `hide_show` | Zwei Knöpfe: „Segmente und Hotspots ausblenden (2)“ / „Segmente und Hotspots anzeigen (2)“. Ausblenden → alle Web-Objekte und Hilfslinien weg; Schritt **„Ebenensichtbarkeit“** (blendet die Webebene aus). Taste **2** schaltet um (1. Druck = ausgeblendet, pixelgleich zum Knopf; 2. Druck = wieder sichtbar) | M, B `09` (2) |
| 10 | `web_layer` | Alle neuen Hotspots/Segmente (gezeichnet und eingefügt) liegen in **„Webebene“** (`layerType` web), oberstes Element zuerst | J |
| 11 | `behavior_handle` | Gewähltes Segment zeigt in der Mitte einen runden Verhaltensgriff; darüber Handzeiger (Hotspot 6,0), kein Tooltip. Klick → Menü: „Verhalten "Einfaches Rollover" hinzufügen“, „Verhalten "Bild austauschen" hinzufügen…“, „Statusleistenmeldung hinzufügen…“, „Navigationsleiste hinzufügen…“, „Popupmenü hinzufügen…“, „Popupmenü bearbeiten…“ (grau), Trenner, „Alle Verhalten löschen“. Griff auf anderes Segment ziehen → Linie, Dialog **„Bild austauschen “** (267 × 123): „Bild austauschen von:“ Liste Status 1–4 (Vorgabe Status 2), „Weitere Optionen…“, OK, Abbrechen; Esc → kein Verhalten | M, W, B `11` (4) |
| 12 | `insert_from_selection` | Bearbeiten ▸ **Objekt einfügen** ▸ Hotspot (Strg+Umschalt+U) / Rechteckiges Segment (Alt+Umschalt+U) / Polygonsegment (Alt+Umschalt+P). Bei 2 gewählten Objekten Meldung (Titel = Programmname): „Es sind mehrere Elemente ausgewählt. Ein Segment oder mehrere Segmente erstellen?“ (bzw. „Einen Hotspot oder mehrere Hotspots erstellen?“), Knöpfe **Einzeln** / **Mehrere**, kein Abbrechen; **Esc = Einzeln**. Einzeln (G + I1) → ein Segment 20,20–220,100 (Vereinigung); Mehrere → zwei (20,20–120,100 und 160,20–220,80); Polygonsegment aus G → 20,20–120,100. Schritte „Segment einfügen“ / „Hotspot einfügen“ | M, J, B `12` |
| 13 | `inspector` | Hotspot: Symbol, „Hotspot“, Namensfeld „Hotspot“, B/H/X/Y (100/60/300/300), **Form:** Rechteck/Kreis/Polygon, **Hyperlink:** (Liste: -----, Seite 1.htm, Seite 2.htm), **Alt:**, **Ziel:** (Ohne, _blank, _self, _parent, _top). Segment: „Segment“, Name „p1415_t06.fw_r6_c5“, B/H/X/Y, **Typ:** Vordergrundbild/Hintergrundbild/HTML, Exportformat-Liste (PNG32, JPEG – Höhere Qualität, JPEG – Kleinere Datei, GIF Web 216, GIF WebSnap 256, GIF WebSnap 128, GIF Adaptiv 256, Animiertes GIF WebSnap 128), Hyperlink, Alt, Ziel | W, B `13` (2) |
| 14 | `cursor` | Alle fünf Web-Werkzeuge: **derselbe** Zeiger (gleiches Handle, Hotspot 10,9) | B `14` (5) |
| 15 | `undo` | Je Zeichnung ein Schritt: „Hotspot-Rechteck“, „Hotspot-Kreis“, „Hotspot-Polygon“ (ganzes Polygon), „Segment“, „Polygon-Segment“; dazu „Segment-Hilfslinie verschieben“, „Ebenensichtbarkeit“, „Segment einfügen“, „Hotspot einfügen“ | M, W |

## Hinweise

- Tooltip des Gruppenknopfs zeigt das zuletzt gewählte Werkzeug (z. B. „Hotspot-Polygon (J)“); Ausklapplisten sind owner-drawn (Text nur im Bild).
- T06 enthält das Polygonsegment S2 und den Polygon-Hotspot H3 nicht (TESTFILES); hier jeweils selbst gezeichnet.

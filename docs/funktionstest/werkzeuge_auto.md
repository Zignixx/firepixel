# Werkzeuge – automatischer Gesamttest

Erzeugt von `crates\fp-ui\tests\gesamt_werkzeuge.rs` (`cargo test -p fp-ui --test gesamt_werkzeuge`). Nicht von Hand ändern.

**52/52 Einträge ✅**

| Eintrag | Befehl | Ergebnis | Hinweis |
|---|---|---|---|
| Auswählen ▸ Zeiger | `tool.pointer` | ✅ Doc | Ziehen/Klicken |
| Auswählen ▸ Hintere auswählen | `tool.selectBehind` | ✅ Ui | Ziehen/Klicken |
| Auswählen ▸ Teilauswahl | `tool.subselection` | ✅ Ui | Ziehen/Klicken |
| Bitmap ▸ Auswahlrahmen | `tool.marquee` | ✅ Doc | Ziehen/Klicken |
| Bitmap ▸ Ovaler Auswahlrahmen | `tool.ovalMarquee` | ✅ Doc | Ziehen/Klicken |
| Bitmap ▸ Lasso | `tool.lasso` | ✅ Doc | Ziehen/Klicken |
| Bitmap ▸ Polygon-Lasso | `tool.polygonLasso` | ✅ Doc | Ziehen/Klicken |
| Auswählen ▸ Zuschneiden | `tool.crop` | ✅ Doc | Ziehen/Klicken |
| Auswählen ▸ Exportbereich | `tool.exportArea` | ✅ Dialog | Ziehen/Klicken |
| Bitmap ▸ Zauberstab | `tool.magicWand` | ✅ Doc | Ziehen/Klicken |
| Bitmap ▸ Objektauswahl | `tool.aiObject` | ✅ Ui | Ziehen/Klicken |
| Bitmap ▸ Motiv auswählen | `tool.aiSubject` | ✅ Ui | Ziehen/Klicken |
| Bitmap ▸ Kante verfeinern | `tool.aiRefineEdge` | ✅ Ui | Ziehen/Klicken: Meldung „Kante verfeinern: fertig“ |
| Vektor ▸ Linie | `tool.line` | ✅ Doc | Ziehen/Klicken |
| Vektor ▸ Stift | `tool.pen` | ✅ Doc | Ziehen/Klicken |
| Vektor ▸ Rechteck | `tool.rectangle` | ✅ Doc | Ziehen/Klicken |
| Vektor ▸ Ellipse | `tool.ellipse` | ✅ Doc | Ziehen/Klicken |
| Vektor ▸ Polygon | `tool.polygon` | ✅ Doc | Ziehen/Klicken |
| Vektor ▸ Automatische Form | `tool.autoShape` | ✅ Doc | Ziehen/Klicken |
| Vektor ▸ Text | `tool.text` | ✅ Doc | Ziehen/Klicken |
| Bitmap ▸ Bleistift | `tool.pencil` | ✅ Doc | Ziehen/Klicken |
| Vektor ▸ Vektorpfad | `tool.vectorPath` | ✅ Doc | Ziehen/Klicken |
| Vektor ▸ Pfad nachzeichnen | `tool.redrawPath` | ✅ Doc | Ziehen/Klicken |
| Bitmap ▸ Pinsel | `tool.brush` | ✅ Doc | Ziehen/Klicken |
| Auswählen ▸ Skalieren | `tool.scale` | ✅ Doc | Ziehen/Klicken |
| Auswählen ▸ Neigen | `tool.skew` | ✅ Doc | Ziehen/Klicken |
| Auswählen ▸ Verzerren | `tool.distort` | ✅ Doc | Ziehen/Klicken |
| Auswählen ▸ 9-Slice-Skalierung | `tool.nineSliceScale` | ✅ Doc | Ziehen/Klicken |
| Vektor ▸ Freiform | `tool.freeform` | ✅ Doc | Ziehen/Klicken |
| Vektor ▸ Bereich umformen | `tool.reshapeArea` | ✅ Doc | Ziehen/Klicken |
| Vektor ▸ Pfadausbesserung - additiv | `tool.pathScrubAdd` | ✅ Doc | Ziehen/Klicken |
| Vektor ▸ Pfadausbesserung - subtraktiv | `tool.pathScrubSubtract` | ✅ Doc | Ziehen/Klicken |
| Farben ▸ Pipette | `tool.eyedropper` | ✅ Ui | Ziehen/Klicken |
| Farben ▸ Füllwerkzeug | `tool.paintBucket` | ✅ Doc | Ziehen/Klicken |
| Farben ▸ Verlauf | `tool.gradient` | ✅ Doc | Ziehen/Klicken |
| Bitmap ▸ Radiergummi | `tool.eraser` | ✅ Doc | Ziehen/Klicken |
| Bitmap ▸ Weichzeichner | `tool.blur` | ✅ Doc | Ziehen/Klicken |
| Bitmap ▸ Scharfzeichner | `tool.sharpen` | ✅ Doc | Ziehen/Klicken |
| Bitmap ▸ Abwedler | `tool.dodge` | ✅ Doc | Ziehen/Klicken |
| Bitmap ▸ Nachbelichter | `tool.burn` | ✅ Doc | Ziehen/Klicken |
| Bitmap ▸ Verwischen | `tool.smudge` | ✅ Doc | Ziehen/Klicken |
| Bitmap ▸ Stempel | `tool.rubberStamp` | ✅ Doc | Ziehen/Klicken |
| Bitmap ▸ Farbe ersetzen | `tool.replaceColor` | ✅ Doc | Ziehen/Klicken |
| Bitmap ▸ Rote Augen entfernen | `tool.redEyeRemoval` | ✅ Doc | Ziehen/Klicken |
| Vektor ▸ Messer | `tool.knife` | ✅ Doc | Ziehen/Klicken |
| Web ▸ Hotspot-Rechteck | `tool.hotspotRect` | ✅ Doc | Ziehen/Klicken |
| Web ▸ Hotspot-Kreis | `tool.hotspotCircle` | ✅ Doc | Ziehen/Klicken |
| Web ▸ Hotspot-Polygon | `tool.hotspotPolygon` | ✅ Doc | Ziehen/Klicken |
| Web ▸ Segment | `tool.slice` | ✅ Doc | Ziehen/Klicken |
| Web ▸ Polygon-Segment | `tool.polygonSlice` | ✅ Doc | Ziehen/Klicken |
| Ansicht ▸ Hand | `tool.hand` | ✅ Ui | Ziehen/Klicken |
| Ansicht ▸ Zoom | `tool.zoom` | ✅ Ui | Ziehen/Klicken |

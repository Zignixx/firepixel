# Firepixel-Dokumentmodell (Kurzfassung für Agenten)

Firepixel bildet das Dokumentmodell des Originals nach. Dateien sind FW-PNGs (`.fw.png`):
ein normales PNG (flaches Bild) mit eingebetteten, editierbaren Daten.

## Aufbau

```
Dokument
└─ Seiten (pages[], eine aktuelle Seite; höchstens eine Masterseite)
   ├─ Leinwand: Breite, Höhe, Ursprung, Auflösung, Leinwandfarbe
   ├─ Status (states, im Original auch „Frames“; für Animation/Rollover)
   └─ Ebenen (layers[], Reihenfolge unten → oben; die Web-Ebene ist die letzte)
      └─ je Status ein Inhalt (bei „über Status geteilten“ Ebenen nur einer)
         └─ Elemente oben → unten
            ├─ rectangle   Rechteck-Grundform (Eckenrundung)
            ├─ path        Vektorpfad (Konturen aus Bézier-Knoten)
            ├─ text        Text (Punkt- oder Rahmentext, Läufe mit Schrift/Größe/Farbe)
            ├─ bitmap      Pixelbild
            ├─ group       Gruppe (children); maskGroup: erstes Element ist die Maske
            ├─ autoShape   Auto-Form (Gruppe mit Skript und Steuerpunkten)
            ├─ compoundShape  zusammengesetzte Form (boolesche Operanden)
            ├─ instance    Symbolinstanz (Grafik, Schaltfläche, Animation)
            └─ slice / hotspot  Web-Objekte (nur auf der Web-Ebene)
```

## IDs und Auswahl

- Jedes Element hat eine **ID** (Ganzzahl), die während der Sitzung stabil bleibt (auch nach Verschieben,
  Gruppieren der Nachbarn, Rückgängig). Neue Objekte liefern ihre ID im Ergebnis von `run_command`.
- Ebenen haben ebenfalls IDs (`layer`-Parameter der `layer.*`-Befehle).
- Die meisten Befehle wirken auf die **Auswahl** (`select`, `select.set`). Einige nehmen `ids` direkt.
- Nach einem Befehl liefert `run_command` die neue Auswahl mit Grenzen zurück.

## Koordinaten und Werte

- Einheit: Dokumentpixel, Ursprung oben links der Leinwand (Leinwandursprung kann ≠ 0 sein).
- Rechteck: `[links, oben, rechts, unten]` oder `{left, top, right, bottom}` oder `{x, y, width, height}`.
- Punkt: `[x, y]` oder `{x, y}`.
- Farbe: `"#rrggbb"` oder `"#rrggbbaa"` (Alpha zuletzt), `"none"` = transparent.
- Deckkraft: Prozent 0–100. Mischmodi: `normal`, `multiply`, `screen`, `overlay`, `darken`, `lighten`, …
- Grenzen in `get_document` sind die sichtbaren Grenzen (inkl. Strich), gerundet auf 0,01 px.

## Verlauf

- Jeder Dokument- oder Auswahlbefehl ist ein Rückgängig-Schritt (`undo`, `redo`, `history`).
- `run_command` mit `steps` fasst mehrere Befehle zu einem Schritt zusammen.
- Ansichtsbefehle (Zoom, Raster) erzeugen keinen Schritt.

## Status und Seiten

- `page.setCurrent {index}`, `state.setCurrent {index}` wechseln Seite/Status; `get_document` zeigt
  `currentPage`/`currentState`. `render` und `get_document` nehmen `page`/`state` auch direkt.

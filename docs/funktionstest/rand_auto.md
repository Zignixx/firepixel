# Rand- und Abschneide-Prüfung (automatisch, RAND-1)

Erzeugt von `crates\fp-ui\tests\clipping_audit.rs` (Nutzerbefund N23). Nicht von Hand bearbeiten.

- 259 Ansichten (79 Dialoge aus dem Befehlsregister) × 24 Varianten (3 Designs × de/en × 100/150 % × 1024 × 768 / 1920 × 1080)
- Grenzwerte: waagerecht 2 px, senkrecht 1 px zum Rahmen, 2 px zum Rand (`FP_RAND_GAP_X`, `FP_RAND_GAP_Y`, `FP_RAND_GAP_EDGE`)
- **0 Funde** an **0 Stellen** (Art + Ansicht + Text), dazu 414 Hinweise; 1648 Texte sind gewollt mit „…“ gekürzt
- Erlaubnisliste: 0 Einträge, davon 0 nicht mehr gefunden

## Stellen je Bereich (Funde in Klammern)

| Bereich | kaestchen | verdeckt | abgeschnitten | dialog-ragt-hinaus | ueberlappt-text | ueberlappt-element | ragt-heraus | rand-knapp | hinweis-bildlauf | hinweis-feldinhalt | Summe |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Hauptfenster | – | – | – | – | – | – | – | – | – | – | **0** (0) |
| Bedienfelder | – | – | – | – | – | – | – | – | 27 (162) | – | **0** (0) |
| Inspektor | – | – | – | – | – | – | – | – | 24 (144) | – | **0** (0) |
| Dialoge | – | – | – | – | – | – | – | – | 4 (30) | 2 (30) | **0** (0) |
| Startseite | – | – | – | – | – | – | – | – | 1 (6) | – | **0** (0) |
| Menüs | – | – | – | – | – | – | – | – | 7 (42) | – | **0** (0) |
| Tooltips | – | – | – | – | – | – | – | – | – | – | **0** (0) |

## Fundstellen

Je Stelle: Art, Text, Rechteck (Punkte, erste Variante), Zahl der Varianten, Beispielvariante und Erklärung. Sortiert nach Bereich, Ansicht und Schwere.

## Hinweise (zählen nicht, 65 Stellen)

<details><summary>Liste</summary>

- hinweis-bildlauf | Bedienfeld FindReplace (Dock) | 0 Treffer (6×)
- hinweis-bildlauf | Bedienfeld FindReplace (schmales Dock) | 0 Treffer (6×)
- hinweis-bildlauf | Bedienfeld ImageEditing (Dock) | Umkehren (6×)
- hinweis-bildlauf | Bedienfeld ImageEditing (schmales Dock) | Kurven... (6×)
- hinweis-bildlauf | Bedienfeld ImageEditing (schmales Dock) | Tonwert... (6×)
- hinweis-bildlauf | Bedienfeld Layers (Dock) | Ebene 1 (6×)
- hinweis-bildlauf | Bedienfeld Layers (schmales Dock) | Ebene 1 (6×)
- hinweis-bildlauf | Bedienfeld Path (Dock) | Geglättete Füllung (6×)
- hinweis-bildlauf | Bedienfeld Path (Dock) | Weiche Füllung (6×)
- hinweis-bildlauf | Bedienfeld Path (schmales Dock) | Harte Füllung (6×)
- hinweis-bildlauf | Bedienfeld Path (schmales Dock) | Pfad einlassen... (6×)
- hinweis-bildlauf | Bedienfeld SpecialCharacters (Dock) | æ (6×)
- hinweis-bildlauf | Bedienfeld SpecialCharacters (Dock) | œ (6×)
- hinweis-bildlauf | Bedienfeld SpecialCharacters (Dock) | ↑ (6×)
- hinweis-bildlauf | Bedienfeld SpecialCharacters (Dock) | √ (6×)
- hinweis-bildlauf | Bedienfeld SpecialCharacters (Dock) | ≠ (6×)
- hinweis-bildlauf | Bedienfeld SpecialCharacters (Dock) | ≤ (6×)
- hinweis-bildlauf | Bedienfeld SpecialCharacters (Dock) | ≥ (6×)
- hinweis-bildlauf | Bedienfeld SpecialCharacters (schmales Dock) | Å (6×)
- hinweis-bildlauf | Bedienfeld SpecialCharacters (schmales Dock) | Ø (6×)
- hinweis-bildlauf | Bedienfeld SpecialCharacters (schmales Dock) | å (6×)
- hinweis-bildlauf | Bedienfeld SpecialCharacters (schmales Dock) | æ (6×)
- hinweis-bildlauf | Bedienfeld SpecialCharacters (schmales Dock) | ñ (6×)
- hinweis-bildlauf | Bedienfeld SpecialCharacters (schmales Dock) | ø (6×)
- hinweis-bildlauf | Bedienfeld SpecialCharacters (schmales Dock) | œ (6×)
- hinweis-bildlauf | Bedienfeld SpecialCharacters (schmales Dock) | ≠ (6×)
- hinweis-bildlauf | Bedienfeld SpecialCharacters (schmales Dock) | ≤ (6×)
- hinweis-bildlauf | Dialog Voreinstellungen 2 | Ausrichten (12×)
- hinweis-bildlauf | Dialog file.fileInfo | 10 (6×)
- hinweis-bildlauf | Dialog file.fileInfo | Formatversion: (6×)
- hinweis-bildlauf | Dialog modify.traceBitmap | Flecken entfernen (6×)
- hinweis-bildlauf | Hauptmenü 1 | Datei-Info... (6×)
- hinweis-bildlauf | Hauptmenü 1 | Strg+Alt+Umschalt+F (6×)
- hinweis-bildlauf | Hauptmenü 2 | Strg+U (6×)
- hinweis-bildlauf | Hauptmenü 2 | Voreinstellungen... (6×)
- hinweis-bildlauf | Hauptmenü 3 | Design (6×)
- hinweis-bildlauf | Hauptmenü 5 | Strg+K (6×)
- hinweis-bildlauf | Hauptmenü 9 | Symboleigenschaften (6×)
- hinweis-bildlauf | Inspektor Pfad | 1-Pixel hart (6×)
- hinweis-bildlauf | Inspektor Text | Anti-Alias glatt (6×)
- hinweis-bildlauf | Inspektor Werkzeug tool.aiObject | Klick oder Rahmen: Objekt · Umschalt: hinzufügen · Alt: abz… (6×)
- hinweis-bildlauf | Inspektor Werkzeug tool.aiRefineEdge | BiRefNet lite Matting (89 MB) (6×)
- hinweis-bildlauf | Inspektor Werkzeug tool.aiSubject | Klick: Hauptmotiv als weiche Pixelauswahl · Umschalt: hinzu… (6×)
- hinweis-bildlauf | Inspektor Werkzeug tool.autoShape | 1-Pixel hart (6×)
- hinweis-bildlauf | Inspektor Werkzeug tool.brush | 1-Pixel hart (6×)
- hinweis-bildlauf | Inspektor Werkzeug tool.ellipse | 1-Pixel hart (6×)
- hinweis-bildlauf | Inspektor Werkzeug tool.gradient | Auswahl füllen (6×)
- hinweis-bildlauf | Inspektor Werkzeug tool.line | 1-Pixel hart (6×)
- hinweis-bildlauf | Inspektor Werkzeug tool.marquee | Live-Auswahlrahmen (6×)
- hinweis-bildlauf | Inspektor Werkzeug tool.ovalMarquee | Live-Auswahlrahmen (6×)
- hinweis-bildlauf | Inspektor Werkzeug tool.paintBucket | Auswahl füllen (6×)
- hinweis-bildlauf | Inspektor Werkzeug tool.pen | 1-Pixel hart (6×)
- hinweis-bildlauf | Inspektor Werkzeug tool.pencil | Transparenz beibehalten (6×)
- hinweis-bildlauf | Inspektor Werkzeug tool.polygon | 1-Pixel hart (6×)
- hinweis-bildlauf | Inspektor Werkzeug tool.rectangle | 1-Pixel hart (6×)
- hinweis-bildlauf | Inspektor Werkzeug tool.redrawPath | 1-Pixel hart (6×)
- hinweis-bildlauf | Inspektor Werkzeug tool.rubberStamp | Gesamtes Dokument (6×)
- hinweis-bildlauf | Inspektor Werkzeug tool.scale | 1-Pixel hart (6×)
- hinweis-bildlauf | Inspektor Werkzeug tool.skew | 1-Pixel hart (6×)
- hinweis-bildlauf | Inspektor Werkzeug tool.text | Anti-Alias glatt (6×)
- hinweis-bildlauf | Inspektor Werkzeug tool.vectorPath | 1-Pixel hart (6×)
- hinweis-bildlauf | Inspektor zwei offene Pfade | 1-Pixel hart (6×)
- hinweis-bildlauf | Startseite | Automatisierung und KI-Steuerung (6×)
- hinweis-feldinhalt | Dialog file.batchProcess | C:\Users\ADMINI~1\AppData\Local\Temp\firepixel-gesamttest\1… (24×)
- hinweis-feldinhalt | Dialog modify.traceBitmap | 4 (6×)

</details>

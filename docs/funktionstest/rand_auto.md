# Rand- und Abschneide-Prüfung (automatisch, RAND-1)

Erzeugt von `crates\fp-ui\tests\clipping_audit.rs` (Nutzerbefund N23). Nicht von Hand bearbeiten.

- 254 Ansichten (74 Dialoge aus dem Befehlsregister) × 24 Varianten (3 Designs × de/en × 100/150 % × 1024 × 768 / 1920 × 1080)
- Grenzwerte: waagerecht 2 px, senkrecht 1 px zum Rahmen, 2 px zum Rand (`FP_RAND_GAP_X`, `FP_RAND_GAP_Y`, `FP_RAND_GAP_EDGE`)
- **24 Funde** an **3 Stellen** (Art + Ansicht + Text), dazu 360 Hinweise; 1584 Texte sind gewollt mit „…“ gekürzt
- Erlaubnisliste: 17 Einträge, davon 17 nicht mehr gefunden

## Stellen je Bereich (Funde in Klammern)

| Bereich | kaestchen | verdeckt | abgeschnitten | dialog-ragt-hinaus | ueberlappt-text | ueberlappt-element | ragt-heraus | rand-knapp | hinweis-bildlauf | hinweis-feldinhalt | Summe |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Hauptfenster | – | – | – | – | – | – | – | – | – | – | **0** (0) |
| Bedienfelder | – | – | – | – | – | – | – | – | 27 (162) | – | **0** (0) |
| Inspektor | – | – | – | – | – | – | – | – | 24 (144) | – | **0** (0) |
| Dialoge | – | – | – | – | – | – | – | – | 2 (18) | 2 (30) | **0** (0) |
| Startseite | – | – | – | – | – | – | – | – | 1 (6) | – | **0** (0) |
| Menüs | – | – | – | 3 (24) | – | – | – | – | – | – | **3** (24) |
| Tooltips | – | – | – | – | – | – | – | – | – | – | **0** (0) |

## Ausschnitte der schlimmsten Funde

- `dialog-ragt-hinaus Hauptmenü 9 ` → `docs\screenshots\rand\rand_01_dialog_ragt_hinaus_hauptmen_9.png`

## Fundstellen

Je Stelle: Art, Text, Rechteck (Punkte, erste Variante), Zahl der Varianten, Beispielvariante und Erklärung. Sortiert nach Bereich, Ansicht und Schwere.

### Menüs

**Bedienfeldmenü 2** (1 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| dialog-ragt-hinaus |  | [474, 0 – 800, 597] | 6 | OLED/en/150 %/1024×768 | Fenster [0, 0 – 800, 512] |

**Hauptmenü 1** (1 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| dialog-ragt-hinaus |  | [45, 0 – 326, 632] | 6 | OLED/en/150 %/1024×768 | Fenster [0, 0 – 800, 512] |

**Hauptmenü 9** (1 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| dialog-ragt-hinaus |  | [522, 0 – 785, 765] | 12 | OLED/en/150 %/1024×768 | Fenster [0, 0 – 800, 512] |

## Hinweise (zählen nicht, 56 Stellen)

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
- hinweis-bildlauf | Dialog modify.traceBitmap | Flecken entfernen (6×)
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
- hinweis-bildlauf | Startseite | Nicht mehr anzeigen (6×)
- hinweis-feldinhalt | Dialog file.batchProcess | C:\Users\ADMINI~1\AppData\Local\Temp\firepixel-gesamttest\1… (24×)
- hinweis-feldinhalt | Dialog modify.traceBitmap | 4 (6×)

</details>

## In der Erlaubnisliste, aber nicht mehr gefunden (streichen)

- `abgeschnitten Dialog modify.traceBitmap Flecken entfernen`
- `dialog-ragt-hinaus Dialog Voreinstellungen 1 `
- `dialog-ragt-hinaus Dialog Voreinstellungen 2 `
- `dialog-ragt-hinaus Dialog Voreinstellungen 3 `
- `dialog-ragt-hinaus Dialog Voreinstellungen 4 `
- `dialog-ragt-hinaus Dialog Voreinstellungen 5 `
- `dialog-ragt-hinaus Dialog Voreinstellungen 6 `
- `dialog-ragt-hinaus Dialog Voreinstellungen `
- `dialog-ragt-hinaus Dialog ai.manageModels `
- `dialog-ragt-hinaus Dialog edit.keyboardShortcuts `
- `ragt-heraus Bedienfeld DocumentLibrary (Dock) Grafik`
- `ragt-heraus Bedienfeld DocumentLibrary (schmales Dock) Graf…`
- `rand-knapp Dialog ai.manageModels Entfernen`
- `ueberlappt-element Bedienfeld DocumentLibrary (Dock) Grafik`
- `ueberlappt-element Bedienfeld DocumentLibrary (Dock) Schalt…`
- `ueberlappt-element Bedienfeld DocumentLibrary (schmales Doc…`
- `ueberlappt-element Bedienfeld DocumentLibrary (schmales Doc…`

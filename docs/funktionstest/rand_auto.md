# Rand- und Abschneide-Prüfung (automatisch, RAND-1)

Erzeugt von `crates\fp-ui\tests\clipping_audit.rs` (Nutzerbefund N23). Nicht von Hand bearbeiten.

- 221 Ansichten (74 Dialoge aus dem Befehlsregister) × 24 Varianten (3 Designs × de/en × 100/150 % × 1024 × 768 / 1920 × 1080)
- Grenzwerte: waagerecht 2 px, senkrecht 1 px zum Rahmen, 2 px zum Rand (`FP_RAND_GAP_X`, `FP_RAND_GAP_Y`, `FP_RAND_GAP_EDGE`)
- **276 Funde** an **17 Stellen** (Art + Ansicht + Text), dazu 378 Hinweise; 1516 Texte sind gewollt mit „…“ gekürzt
- Erlaubnisliste: 17 Einträge, davon 0 nicht mehr gefunden

## Stellen je Bereich (Funde in Klammern)

| Bereich | kaestchen | verdeckt | abgeschnitten | dialog-ragt-hinaus | ueberlappt-text | ueberlappt-element | ragt-heraus | rand-knapp | hinweis-bildlauf | hinweis-feldinhalt | Summe |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Hauptfenster | – | – | – | – | – | – | – | – | – | – | **0** (0) |
| Bedienfelder | – | – | – | – | – | 4 (60) | 2 (12) | – | 29 (186) | – | **6** (72) |
| Inspektor | – | – | – | – | – | – | – | – | 24 (144) | – | **0** (0) |
| Dialoge | – | – | 1 (6) | 9 (54) | – | – | – | 1 (144) | 1 (6) | 3 (36) | **11** (204) |
| Startseite | – | – | – | – | – | – | – | – | 1 (6) | – | **0** (0) |

## Ausschnitte der schlimmsten Funde

- `abgeschnitten Dialog modify.traceBitmap Flecken entfernen` → `docs\screenshots\rand\rand_01_abgeschnitten_dialog_modify_tracebitmap.png`
- `dialog-ragt-hinaus Dialog edit.keyboardShortcuts ` → `docs\screenshots\rand\rand_02_dialog_ragt_hinaus_dialog_edit_keyboardshortcuts.png`
- `ueberlappt-element Bedienfeld DocumentLibrary (Dock) Grafik` → `docs\screenshots\rand\rand_03_ueberlappt_element_bedienfeld_documentlibrary_dock.png`
- `rand-knapp Dialog ai.manageModels Entfernen` → `docs\screenshots\rand\rand_04_rand_knapp_dialog_ai_managemodels.png`
- `ueberlappt-element Bedienfeld DocumentLibrary (schmales Doc…` → `docs\screenshots\rand\rand_05_ueberlappt_element_bedienfeld_documentlibrary_schmales_dock.png`

## Fundstellen

Je Stelle: Art, Text, Rechteck (Punkte, erste Variante), Zahl der Varianten, Beispielvariante und Erklärung. Sortiert nach Bereich, Ansicht und Schwere.

### Bedienfelder

**Bedienfeld DocumentLibrary (Dock)** (3 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| ueberlappt-element | Grafik | [716, 477 – 745, 485] | 12 | OLED/en/150 %/1024×768 | läuft in [724, 482 – 743, 498] (Button) |
| ueberlappt-element | Schaltfläche | [717, 497 – 774, 505] | 6 | OLED/en/150 %/1024×768 | läuft in [724, 482 – 743, 498] (Button) |
| ragt-heraus | Grafik | [940, 748 – 969, 756] | 6 | OLED/en/100 %/1024×768 | Element [948, 749 – 967, 765] (Button) |

**Bedienfeld DocumentLibrary (schmales Dock)** (3 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| ueberlappt-element | Grafik | [716, 477 – 745, 485] | 12 | OLED/en/150 %/1024×768 | läuft in [724, 482 – 743, 498] (Button) |
| ueberlappt-element | Schaltfläche | [717, 497 – 774, 505] | 6 | OLED/en/150 %/1024×768 | läuft in [724, 482 – 743, 498] (Button) |
| ragt-heraus | Grafik | [940, 748 – 969, 756] | 6 | OLED/en/100 %/1024×768 | Element [948, 749 – 967, 765] (Button) |

### Dialoge

**Dialog Voreinstellungen** (1 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| dialog-ragt-hinaus |  | [31, 0 – 770, 586] | 6 | OLED/en/150 %/1024×768 | Fenster [0, 0 – 800, 512] |

**Dialog Voreinstellungen 1** (1 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| dialog-ragt-hinaus |  | [31, 0 – 770, 586] | 6 | OLED/en/150 %/1024×768 | Fenster [0, 0 – 800, 512] |

**Dialog Voreinstellungen 2** (1 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| dialog-ragt-hinaus |  | [31, 0 – 770, 586] | 6 | OLED/en/150 %/1024×768 | Fenster [0, 0 – 800, 512] |

**Dialog Voreinstellungen 3** (1 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| dialog-ragt-hinaus |  | [31, 0 – 770, 586] | 6 | OLED/en/150 %/1024×768 | Fenster [0, 0 – 800, 512] |

**Dialog Voreinstellungen 4** (1 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| dialog-ragt-hinaus |  | [31, 0 – 770, 586] | 6 | OLED/en/150 %/1024×768 | Fenster [0, 0 – 800, 512] |

**Dialog Voreinstellungen 5** (1 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| dialog-ragt-hinaus |  | [31, 0 – 770, 586] | 6 | OLED/en/150 %/1024×768 | Fenster [0, 0 – 800, 512] |

**Dialog Voreinstellungen 6** (1 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| dialog-ragt-hinaus |  | [31, 0 – 770, 586] | 6 | OLED/en/150 %/1024×768 | Fenster [0, 0 – 800, 512] |

**Dialog ai.manageModels** (2 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| dialog-ragt-hinaus |  | [-1, 0 – 801, 570] | 6 | OLED/en/150 %/1024×768 | Fenster [0, 0 – 800, 512] |
| rand-knapp | Entfernen… | [1224, 572 – 1278, 580] | 24 | OLED/en/100 %/1920×1080 | waagerecht 1.0/-0.2 px zum Rahmen [1223, 566 – 1278, 586] (… |

**Dialog edit.keyboardShortcuts** (1 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| dialog-ragt-hinaus |  | [57, 0 – 743, 590] | 6 | OLED/en/150 %/1024×768 | Fenster [0, 0 – 800, 512] |

**Dialog modify.traceBitmap** (1 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| abgeschnitten | Flecken entfernen | [133, 621 – 222, 629] | 6 | OLED/en/150 %/1920×1080 | Clip [132, 612 – 382, 623] |

## Hinweise (zählen nicht, 58 Stellen)

<details><summary>Liste</summary>

- hinweis-bildlauf | Bedienfeld DocumentLibrary (Dock) | Schaltfläche (12×)
- hinweis-bildlauf | Bedienfeld DocumentLibrary (schmales Dock) | Schaltfläche (12×)
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
- hinweis-bildlauf | Dialog Voreinstellungen 2 | Ausrichtungsabstand (Pixel): (6×)
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
- hinweis-feldinhalt | Dialog Voreinstellungen 2 | 5 (6×)
- hinweis-feldinhalt | Dialog file.batchProcess | C:\Users\ADMINI~1\AppData\Local\Temp\firepixel-gesamttest\1… (24×)
- hinweis-feldinhalt | Dialog modify.traceBitmap | 4 (6×)

</details>

# Rand- und Abschneide-Prüfung (automatisch, RAND-1)

Erzeugt von `crates\fp-ui\tests\clipping_audit.rs` (Nutzerbefund N23). Nicht von Hand bearbeiten.

- 227 Ansichten (80 Dialoge aus dem Befehlsregister) × 24 Varianten (3 Designs × de/en × 100/150 % × 1024 × 768 / 1920 × 1080)
- Grenzwerte: waagerecht 2 px, senkrecht 1 px zum Rahmen, 2 px zum Rand (`FP_RAND_GAP_X`, `FP_RAND_GAP_Y`, `FP_RAND_GAP_EDGE`)
- **408 Funde** an **70 Stellen** (Art + Ansicht + Text), dazu 270 Hinweise; 1225 Texte sind gewollt mit „…“ gekürzt
- Erlaubnisliste: 70 Einträge, davon 0 nicht mehr gefunden

## Stellen je Bereich (Funde in Klammern)

| Bereich | kaestchen | verdeckt | abgeschnitten | dialog-ragt-hinaus | ueberlappt-text | ueberlappt-element | ragt-heraus | rand-knapp | hinweis-bildlauf | hinweis-feldinhalt | Summe |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Hauptfenster | – | – | – | – | – | – | – | – | – | – | **0** (0) |
| Bedienfelder | – | – | 2 (6) | – | – | – | – | – | 40 (180) | – | **2** (6) |
| Inspektor | 6 (24) | – | 46 (144) | – | 1 (12) | – | 2 (18) | – | 2 (6) | – | **55** (198) |
| Dialoge | – | – | 2 (6) | 9 (54) | – | – | – | 2 (144) | 5 (42) | 3 (36) | **13** (204) |
| Startseite | – | – | – | – | – | – | – | – | 2 (6) | – | **0** (0) |

## Ausschnitte der schlimmsten Funde

- `kaestchen Inspektor Text ` → `docs\screenshots\rand\rand_01_kaestchen_inspektor_text.png`
- `abgeschnitten Dialog modify.traceBitmap Remove speckles` → `docs\screenshots\rand\rand_02_abgeschnitten_dialog_modify_tracebitmap.png`
- `dialog-ragt-hinaus Dialog edit.keyboardShortcuts ` → `docs\screenshots\rand\rand_03_dialog_ragt_hinaus_dialog_edit_keyboardshortcuts.png`
- `ueberlappt-text Inspektor Segment Bearbeiten...` → `docs\screenshots\rand\rand_04_ueberlappt_text_inspektor_segment.png`
- `rand-knapp Dialog ai.manageModels Entfernen` → `docs\screenshots\rand\rand_05_rand_knapp_dialog_ai_managemodels.png`
- `kaestchen Inspektor Werkzeug tool.text Auto kern` → `docs\screenshots\rand\rand_06_kaestchen_inspektor_werkzeug_tool_text.png`
- `abgeschnitten Inspektor Werkzeug tool.aiSubject BiRefNet li…` → `docs\screenshots\rand\rand_07_abgeschnitten_inspektor_werkzeug_tool_aisubject.png`
- `abgeschnitten Inspektor Pfad 1-Pixel Hard` → `docs\screenshots\rand\rand_08_abgeschnitten_inspektor_pfad.png`
- `abgeschnitten Inspektor Werkzeug tool.aiRefineEdge BiRefNet…` → `docs\screenshots\rand\rand_09_abgeschnitten_inspektor_werkzeug_tool_airefineedge.png`
- `abgeschnitten Inspektor Werkzeug tool.autoShape 1-Pixel hart` → `docs\screenshots\rand\rand_10_abgeschnitten_inspektor_werkzeug_tool_autoshape.png`
- `abgeschnitten Inspektor Werkzeug tool.aiObject SAM 2.1 Hier…` → `docs\screenshots\rand\rand_11_abgeschnitten_inspektor_werkzeug_tool_aiobject.png`
- `abgeschnitten Inspektor Werkzeug tool.gradient Auswahl füll…` → `docs\screenshots\rand\rand_12_abgeschnitten_inspektor_werkzeug_tool_gradient.png`

## Fundstellen

Je Stelle: Art, Text, Rechteck (Punkte, erste Variante), Zahl der Varianten, Beispielvariante und Erklärung. Sortiert nach Bereich, Ansicht und Schwere.

### Bedienfelder

**Bedienfeld DocumentLibrary (Dock)** (1 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| abgeschnitten | Graphic | [940, 748 – 977, 758] | 3 | OLED/en/100 %/1024×768 | Clip [752, 742 – 1012, 757] |

**Bedienfeld DocumentLibrary (schmales Dock)** (1 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| abgeschnitten | Graphic | [940, 748 – 977, 758] | 3 | OLED/en/100 %/1024×768 | Clip [780, 742 – 1012, 757] |

### Dialoge

**Dialog Voreinstellungen** (1 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| dialog-ragt-hinaus |  | [79, 0 – 721, 586] | 6 | OLED/en/150 %/1024×768 | Fenster [0, 0 – 800, 512] |

**Dialog Voreinstellungen 1** (1 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| dialog-ragt-hinaus |  | [79, 0 – 721, 586] | 6 | OLED/en/150 %/1024×768 | Fenster [0, 0 – 800, 512] |

**Dialog Voreinstellungen 2** (1 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| dialog-ragt-hinaus |  | [79, 0 – 721, 586] | 6 | OLED/en/150 %/1024×768 | Fenster [0, 0 – 800, 512] |

**Dialog Voreinstellungen 3** (1 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| dialog-ragt-hinaus |  | [79, 0 – 721, 586] | 6 | OLED/en/150 %/1024×768 | Fenster [0, 0 – 800, 512] |

**Dialog Voreinstellungen 4** (1 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| dialog-ragt-hinaus |  | [79, 0 – 721, 586] | 6 | OLED/en/150 %/1024×768 | Fenster [0, 0 – 800, 512] |

**Dialog Voreinstellungen 5** (1 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| dialog-ragt-hinaus |  | [79, 0 – 721, 586] | 6 | OLED/en/150 %/1024×768 | Fenster [0, 0 – 800, 512] |

**Dialog Voreinstellungen 6** (1 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| dialog-ragt-hinaus |  | [79, 0 – 721, 586] | 6 | OLED/en/150 %/1024×768 | Fenster [0, 0 – 800, 512] |

**Dialog ai.manageModels** (3 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| dialog-ragt-hinaus |  | [-1, 0 – 801, 570] | 6 | OLED/en/150 %/1024×768 | Fenster [0, 0 – 800, 512] |
| rand-knapp | Entfernen… | [1224, 572 – 1278, 580] | 12 | OLED/de/100 %/1920×1080 | waagerecht 1.0/-0.2 px zum Rahmen [1223, 566 – 1278, 586] (… |
| rand-knapp | Remove… | [1206, 572 – 1252, 580] | 12 | OLED/en/100 %/1920×1080 | waagerecht 1.0/0.0 px zum Rahmen [1206, 566 – 1252, 586] (B… |

**Dialog edit.keyboardShortcuts** (1 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| dialog-ragt-hinaus |  | [79, 0 – 721, 590] | 6 | OLED/en/150 %/1024×768 | Fenster [0, 0 – 800, 512] |

**Dialog modify.traceBitmap** (2 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| abgeschnitten | Flecken entfernen | [133, 621 – 222, 629] | 3 | OLED/de/150 %/1920×1080 | Clip [132, 612 – 382, 623] |
| abgeschnitten | Remove speckles | [133, 621 – 219, 632] | 3 | OLED/en/150 %/1920×1080 | Clip [132, 612 – 382, 623] |

### Inspektor

**Inspektor Pfad** (2 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| abgeschnitten | 1-Pixel Hard | [490, 411 – 550, 419] | 3 | OLED/en/150 %/1024×768 | Clip [201, 403 – 521, 451] |
| abgeschnitten | 1-Pixel hart | [490, 411 – 546, 419] | 3 | OLED/de/150 %/1024×768 | Clip [201, 403 – 521, 451] |

**Inspektor Segment** (1 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| ueberlappt-text | Bearbeiten... | [415, 668 – 476, 676] | 12 | OLED/de/150 %/1920×1080 | mit „Ziel:“ [473, 667 – 494, 675] |

**Inspektor Text** (6 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| kaestchen |  | [449, 481 – 587, 497] | 3 | OLED/de/150 %/1024×768 | CheckBox: 12 % des Kästchens [449, 482 – 462, 496] nicht si… |
| kaestchen | Auto kern | [449, 481 – 587, 497] | 6 | OLED/en/150 %/1024×768 | CheckBox: 12 % des Kästchens [449, 482 – 462, 496] nicht si… |
| kaestchen | Autom. Unterschneidung | [449, 736 – 587, 752] | 3 | OLED/de/100 %/1024×768 | CheckBox: 4 % des Kästchens [449, 738 – 462, 750] nicht sic… |
| abgeschnitten | Anti-Alias glatt | [453, 460 – 528, 471] | 3 | OLED/de/150 %/1024×768 | Clip [201, 451 – 521, 494] |
| abgeschnitten | Smooth Anti-Alias | [454, 460 – 545, 468] | 3 | OLED/en/150 %/1024×768 | Clip [201, 451 – 521, 494] |
| ragt-heraus | Autom. Unterschneidung | [467, 1052 – 594, 1064] | 9 | OLED/de/100 %/1920×1080 | Element [449, 1048 – 587, 1064] (CheckBox) |

**Inspektor Werkzeug tool.aiObject** (3 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| abgeschnitten | Klick: Objekt auswählen · Rahmen aufziehen: Objekt im Rahmen | [208, 458 – 528, 469] | 3 | OLED/de/150 %/1024×768 | Clip [201, 451 – 521, 494] |
| abgeschnitten | SAM 2.1 Hiera Tiny (156 MB) | [442, 410 – 585, 421] | 6 | OLED/en/150 %/1024×768 | Clip [201, 403 – 521, 451] |
| abgeschnitten | Umschalt+Klick: hinzufügen · Alt+Klick: abziehen · Esc: Bit… | [208, 477 – 601, 488] | 3 | OLED/de/150 %/1024×768 | Clip [201, 451 – 521, 494] |

**Inspektor Werkzeug tool.aiRefineEdge** (1 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| abgeschnitten | BiRefNet lite Matting (89 MB) | [442, 410 – 589, 421] | 6 | OLED/en/150 %/1024×768 | Clip [201, 403 – 521, 451] |

**Inspektor Werkzeug tool.aiSubject** (3 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| abgeschnitten | BiRefNet lite (general) (178 MB) | [442, 410 – 600, 421] | 3 | OLED/en/150 %/1024×768 | Clip [201, 403 – 521, 451] |
| abgeschnitten | Click: main subject as soft pixel selection · Shift: add · … | [207, 458 – 538, 469] | 3 | OLED/en/150 %/1024×768 | Clip [201, 451 – 521, 494] |
| abgeschnitten | Klick: Hauptmotiv als weiche Pixelauswahl · Umschalt: hinzu… | [208, 458 – 610, 469] | 3 | OLED/de/150 %/1024×768 | Clip [201, 451 – 521, 494] |

**Inspektor Werkzeug tool.autoShape** (2 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| abgeschnitten | 1-Pixel Hard | [490, 411 – 550, 419] | 3 | OLED/en/150 %/1024×768 | Clip [201, 403 – 521, 451] |
| abgeschnitten | 1-Pixel hart | [490, 411 – 546, 419] | 3 | OLED/de/150 %/1024×768 | Clip [201, 403 – 521, 451] |

**Inspektor Werkzeug tool.brush** (2 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| abgeschnitten | 1-Pixel Hard | [490, 411 – 550, 419] | 3 | OLED/en/150 %/1024×768 | Clip [201, 403 – 521, 451] |
| abgeschnitten | 1-Pixel hart | [490, 411 – 546, 419] | 3 | OLED/de/150 %/1024×768 | Clip [201, 403 – 521, 451] |

**Inspektor Werkzeug tool.ellipse** (2 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| abgeschnitten | 1-Pixel Hard | [490, 411 – 550, 419] | 3 | OLED/en/150 %/1024×768 | Clip [201, 403 – 521, 451] |
| abgeschnitten | 1-Pixel hart | [490, 411 – 546, 419] | 3 | OLED/de/150 %/1024×768 | Clip [201, 403 – 521, 451] |

**Inspektor Werkzeug tool.gradient** (2 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| abgeschnitten | Auswahl füllen | [470, 433 – 543, 442] | 3 | OLED/de/150 %/1024×768 | Clip [201, 403 – 521, 451] |
| abgeschnitten | Fill selection | [471, 433 – 531, 442] | 3 | OLED/en/150 %/1024×768 | Clip [201, 403 – 521, 451] |

**Inspektor Werkzeug tool.line** (2 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| abgeschnitten | 1-Pixel Hard | [490, 411 – 550, 419] | 3 | OLED/en/150 %/1024×768 | Clip [201, 403 – 521, 451] |
| abgeschnitten | 1-Pixel hart | [490, 411 – 546, 419] | 3 | OLED/de/150 %/1024×768 | Clip [201, 403 – 521, 451] |

**Inspektor Werkzeug tool.marquee** (1 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| abgeschnitten | Live-Auswahlrahmen | [429, 435 – 533, 444] | 3 | OLED/de/150 %/1024×768 | Clip [201, 403 – 521, 451] |

**Inspektor Werkzeug tool.ovalMarquee** (1 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| abgeschnitten | Live-Auswahlrahmen | [429, 435 – 533, 444] | 3 | OLED/de/150 %/1024×768 | Clip [201, 403 – 521, 451] |

**Inspektor Werkzeug tool.paintBucket** (2 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| abgeschnitten | Auswahl füllen | [470, 433 – 543, 442] | 3 | OLED/de/150 %/1024×768 | Clip [201, 403 – 521, 451] |
| abgeschnitten | Fill selection | [471, 433 – 531, 442] | 3 | OLED/en/150 %/1024×768 | Clip [201, 403 – 521, 451] |

**Inspektor Werkzeug tool.pen** (2 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| abgeschnitten | 1-Pixel Hard | [490, 411 – 550, 419] | 3 | OLED/en/150 %/1024×768 | Clip [201, 403 – 521, 451] |
| abgeschnitten | 1-Pixel hart | [490, 411 – 546, 419] | 3 | OLED/de/150 %/1024×768 | Clip [201, 403 – 521, 451] |

**Inspektor Werkzeug tool.pencil** (2 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| abgeschnitten | Preserve transparency | [426, 412 – 536, 423] | 3 | OLED/en/150 %/1024×768 | Clip [201, 403 – 521, 451] |
| abgeschnitten | Transparenz beibehalten | [425, 411 – 548, 423] | 3 | OLED/de/150 %/1024×768 | Clip [201, 403 – 521, 451] |

**Inspektor Werkzeug tool.polygon** (2 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| abgeschnitten | 1-Pixel Hard | [490, 411 – 550, 419] | 3 | OLED/en/150 %/1024×768 | Clip [201, 403 – 521, 451] |
| abgeschnitten | 1-Pixel hart | [490, 411 – 546, 419] | 3 | OLED/de/150 %/1024×768 | Clip [201, 403 – 521, 451] |

**Inspektor Werkzeug tool.rectangle** (2 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| abgeschnitten | 1-Pixel Hard | [490, 411 – 550, 419] | 3 | OLED/en/150 %/1024×768 | Clip [201, 403 – 521, 451] |
| abgeschnitten | 1-Pixel hart | [490, 411 – 546, 419] | 3 | OLED/de/150 %/1024×768 | Clip [201, 403 – 521, 451] |

**Inspektor Werkzeug tool.redrawPath** (2 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| abgeschnitten | 1-Pixel Hard | [490, 411 – 550, 419] | 3 | OLED/en/150 %/1024×768 | Clip [201, 403 – 521, 451] |
| abgeschnitten | 1-Pixel hart | [490, 411 – 546, 419] | 3 | OLED/de/150 %/1024×768 | Clip [201, 403 – 521, 451] |

**Inspektor Werkzeug tool.rubberStamp** (1 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| abgeschnitten | Gesamtes Dokument | [416, 433 – 522, 442] | 3 | OLED/de/150 %/1024×768 | Clip [201, 403 – 521, 451] |

**Inspektor Werkzeug tool.scale** (2 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| abgeschnitten | 1-Pixel Hard | [490, 411 – 550, 419] | 3 | OLED/en/150 %/1024×768 | Clip [201, 403 – 521, 451] |
| abgeschnitten | 1-Pixel hart | [490, 411 – 546, 419] | 3 | OLED/de/150 %/1024×768 | Clip [201, 403 – 521, 451] |

**Inspektor Werkzeug tool.skew** (2 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| abgeschnitten | 1-Pixel Hard | [490, 411 – 550, 419] | 3 | OLED/en/150 %/1024×768 | Clip [201, 403 – 521, 451] |
| abgeschnitten | 1-Pixel hart | [490, 411 – 546, 419] | 3 | OLED/de/150 %/1024×768 | Clip [201, 403 – 521, 451] |

**Inspektor Werkzeug tool.text** (6 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| kaestchen |  | [449, 481 – 587, 497] | 3 | OLED/de/150 %/1024×768 | CheckBox: 12 % des Kästchens [449, 482 – 462, 496] nicht si… |
| kaestchen | Auto kern | [449, 481 – 587, 497] | 6 | OLED/en/150 %/1024×768 | CheckBox: 12 % des Kästchens [449, 482 – 462, 496] nicht si… |
| kaestchen | Autom. Unterschneidung | [449, 736 – 587, 752] | 3 | OLED/de/100 %/1024×768 | CheckBox: 4 % des Kästchens [449, 738 – 462, 750] nicht sic… |
| abgeschnitten | Anti-Alias glatt | [453, 460 – 528, 471] | 3 | OLED/de/150 %/1024×768 | Clip [201, 451 – 521, 494] |
| abgeschnitten | Smooth Anti-Alias | [454, 460 – 545, 468] | 3 | OLED/en/150 %/1024×768 | Clip [201, 451 – 521, 494] |
| ragt-heraus | Autom. Unterschneidung | [467, 1052 – 594, 1064] | 9 | OLED/de/100 %/1920×1080 | Element [449, 1048 – 587, 1064] (CheckBox) |

**Inspektor Werkzeug tool.vectorPath** (2 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| abgeschnitten | 1-Pixel Hard | [490, 411 – 550, 419] | 3 | OLED/en/150 %/1024×768 | Clip [201, 403 – 521, 451] |
| abgeschnitten | 1-Pixel hart | [490, 411 – 546, 419] | 3 | OLED/de/150 %/1024×768 | Clip [201, 403 – 521, 451] |

**Inspektor zwei offene Pfade** (2 Stellen)

| Art | Text | Rechteck | Varianten | Beispiel | Erklärung |
|---|---|---|---|---|---|
| abgeschnitten | 1-Pixel Hard | [490, 411 – 550, 419] | 3 | OLED/en/150 %/1024×768 | Clip [201, 403 – 521, 451] |
| abgeschnitten | 1-Pixel hart | [490, 411 – 546, 419] | 3 | OLED/de/150 %/1024×768 | Clip [201, 403 – 521, 451] |

## Hinweise (zählen nicht, 52 Stellen)

<details><summary>Liste</summary>

- hinweis-bildlauf | Bedienfeld DocumentLibrary (Dock) | Button (3×)
- hinweis-bildlauf | Bedienfeld DocumentLibrary (Dock) | Schaltfläche (9×)
- hinweis-bildlauf | Bedienfeld DocumentLibrary (schmales Dock) | Button (3×)
- hinweis-bildlauf | Bedienfeld DocumentLibrary (schmales Dock) | Schaltfläche (9×)
- hinweis-bildlauf | Bedienfeld FindReplace (Dock) | 0 Treffer (3×)
- hinweis-bildlauf | Bedienfeld FindReplace (Dock) | 0 matches (3×)
- hinweis-bildlauf | Bedienfeld FindReplace (schmales Dock) | 0 Treffer (3×)
- hinweis-bildlauf | Bedienfeld FindReplace (schmales Dock) | 0 matches (3×)
- hinweis-bildlauf | Bedienfeld ImageEditing (Dock) | Invert (3×)
- hinweis-bildlauf | Bedienfeld ImageEditing (Dock) | Umkehren (3×)
- hinweis-bildlauf | Bedienfeld ImageEditing (schmales Dock) | Curves... (3×)
- hinweis-bildlauf | Bedienfeld ImageEditing (schmales Dock) | Hue/Saturation... (3×)
- hinweis-bildlauf | Bedienfeld ImageEditing (schmales Dock) | Kurven... (3×)
- hinweis-bildlauf | Bedienfeld ImageEditing (schmales Dock) | Levels... (3×)
- hinweis-bildlauf | Bedienfeld ImageEditing (schmales Dock) | Tonwert... (3×)
- hinweis-bildlauf | Bedienfeld Layers (Dock) | Ebene 1 (3×)
- hinweis-bildlauf | Bedienfeld Layers (Dock) | Layer 1 (3×)
- hinweis-bildlauf | Bedienfeld Layers (schmales Dock) | Ebene 1 (3×)
- hinweis-bildlauf | Bedienfeld Layers (schmales Dock) | Layer 1 (3×)
- hinweis-bildlauf | Bedienfeld Path (Dock) | Geglättete Füllung (3×)
- hinweis-bildlauf | Bedienfeld Path (Dock) | Weiche Füllung (3×)
- hinweis-bildlauf | Bedienfeld Path (schmales Dock) | Feather Fill (3×)
- hinweis-bildlauf | Bedienfeld Path (schmales Dock) | Harte Füllung (3×)
- hinweis-bildlauf | Bedienfeld Path (schmales Dock) | Pfad einlassen... (3×)
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
- hinweis-bildlauf | Dialog Voreinstellungen 2 | Ausrichtungsabstand (Pixel): (3×)
- hinweis-bildlauf | Dialog Voreinstellungen 2 | Snap distance (pixels): (3×)
- hinweis-bildlauf | Dialog edit.keyboardShortcuts | Ansicht ▸ Raster ▸ Raster bearbeiten... (12×)
- hinweis-bildlauf | Dialog edit.keyboardShortcuts | Ctrl+Shift+F8 (12×)
- hinweis-bildlauf | Dialog edit.keyboardShortcuts | Edit ▸ Insert ▸ New Button... (12×)
- hinweis-bildlauf | Inspektor Text | Autom. Unterschneidung (3×)
- hinweis-bildlauf | Inspektor Werkzeug tool.text | Autom. Unterschneidung (3×)
- hinweis-bildlauf | Startseite | Don't show again (3×)
- hinweis-bildlauf | Startseite | Nicht mehr anzeigen (3×)
- hinweis-feldinhalt | Dialog Voreinstellungen 2 | 5 (6×)
- hinweis-feldinhalt | Dialog file.batchProcess | C:\Users\ADMINI~1\AppData\Local\Temp\firepixel-gesamttest\1… (24×)
- hinweis-feldinhalt | Dialog modify.traceBitmap | 4 (6×)

</details>

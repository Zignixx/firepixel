# Prüfung der Mehrsprachigkeit (I18N-QA, Phase 6)

Gepflegt von **I18N-QA**. Die Zahlen und Listen zwischen den Markierungen erzeugen die Tests
automatisch; der Text außerhalb ist von Hand.

## So wird geprüft

Prüfgerüst: `crates\fp-ui\tests\i18n_qa_common\mod.rs` (egui_kittest, offscreen). Je Sprache ein
Lauf mit einem Test je Bereich:

```
$env:FP_NO_SHELL_OPEN = '1'
$env:CARGO_TARGET_DIR = '…\firepixel\target-i18n-qa'
cargo test -j 4 -p fp-ui --test i18n_qa_de  -- --ignored --test-threads 4   # Deutsch (+ Vergleich mit dem Stand vor Phase 6)
cargo test -j 4 -p fp-ui --test i18n_qa_en  -- --ignored --test-threads 4   # Englisch
cargo test -j 4 -p fp-ui --test i18n_qa_qps -- --ignored --test-threads 4   # Testsprache
cargo test -j 4 -p fp-ui --test i18n_qa_glossar -- --ignored                # Englisch fachlich (ohne Oberfläche)
cargo test -j 4 -p fp-ui --test i18n_qa_bericht -- --ignored                # Zahlen unten aktualisieren
```

Einzelne Bilder: `$env:FP_QA_NUR = 'dlg:leinwandgroesse'` (Teil der Bild-ID). Die Liste der Dialoge
wird beim ersten Lauf ermittelt (jeder Befehl aus Hauptmenü, Bedienfeldmenüs und
Leinwand-Kontextmenüs in bis zu drei passenden Auswahl-Situationen) und in
`target-i18n-qa\i18n-qa\dialoge.json` zwischengespeichert (`FP_QA_NEU=1` ermittelt neu).

**Bilder (Bereiche):**

| Bereich | Inhalt |
|---|---|
| Hauptfenster | Normal, Dunkel, OLED je ohne/mit Auswahl; ohne Dokument; Bedienfelder als Symbole mit Namen |
| Menüs | jedes Hauptmenü und jedes Untermenü (bis Tiefe 3), geöffnet wie mit der Maus (Überfahren) |
| Bedienfelder | alle 23 Bedienfelder vorn im Dock (mit passender Auswahl) und ihr Bedienfeldmenü (≡, Klick) |
| Inspektor | 28 Auswahl-Situationen (jede Elementart, Mehrfachauswahl, Pixelauswahl, Text auf Pfad …) und alle 52 Werkzeuge |
| Dialoge | jeder über einen Menübefehl erreichbare Dialog (Bauart einmal), Ungespeichert-Abfrage, Arbeitsbereich speichern, Voreinstellungen je Kategorie |
| Kontextmenüs | Leinwand in 9 Zuständen (nichts, Objekt, Text, Pixel, Skalieren, Zuschneiden, Zuschneiden im Bitmapmodus, Zoom, Pipette), Dokumentreiter, Zeile in Ebenen/Seiten/Status (Rechtsklick) |
| Statuszeile | Zeiger über der Leinwand, Meldungen nach Befehlen, Werkzeughinweise |

**Prüfungen je Bild:**

- (a) **Rohschlüssel** – ein gezeichneter Text enthält etwas wie `dlg-canvas-size` (Bereichspräfix aus dem README von fp-i18n). Größe 1600×1000, 100 %.
- (b) **Unmarkiert (nur qps)** – jeder Text aus `fp_i18n::t` erscheint als `[!! … ·· !!]`. Ein Text ohne Markierung ist fest im Code. Ausgenommen sind Texte ohne Buchstaben, Inhalte des Testdokuments, Pfade und Farben sowie Texte nur aus Eigen-/Formatnamen und Einheiten (`PROPER` in `mod.rs`).
- (c) **Überlauf** (1024×768 und 1920×1080, je 100 % und 150 %):
  - Text waagerecht abgeschnitten ohne „…“,
  - Text über den Fensterrand,
  - zwei Texte überlappen,
  - Text ragt teilweise in ein fremdes Bedienelement derselben Ebene,
  - Bedienelement ragt über den Fensterrand.
  
  Gezählt wird je Sprache. In Klammern steht nur, was es im selben Bild und derselben Größe in Deutsch nicht gibt, also eine Folge der Übersetzung.
- (d) **Deutsch unverändert** – Liste aller gezeichneten Texte mit Lage (x, y, Breite, Höhe) je Bild, verglichen mit dem Stand vor Phase 6. Die Basis wurde aus Commit `30ddaec` mit `FP_QA_MODUS=basis` erzeugt (Kopie des Commits außerhalb des Repositorys, dasselbe Prüfgerüst). Dazu Bilder `docs\screenshots\phase6_basis_de_*.png` mit Pixelvergleich.

## Zahlen je Bereich

<!-- AUTO:START -->
Stand des Laufs: 12.10.2026  4:29:41,70 (automatisch erzeugt von `crates\fp-ui\tests\i18n_qa_bericht.rs`)

| Bereich | Bilder | Rohschlüssel de/en/qps | unmarkiert qps (Texte) | Überläufe en (neu ggü. de) | Überläufe qps (neu ggü. de) | Überläufe de (neu ggü. vor Phase 6) | Deutsch geändert | nicht herstellbar de/en/qps |
|---|---|---|---|---|---|---|---|---|
| Hauptfenster (3 Designs, leer, Symbolmodus) | 8 | 0/0/0 | 2 | 3 (0) | 4 (1) | 3 (0) | 7 | 0/0/0 |
| Menüs und Untermenüs | 68 | 0/0/0 | 138 | 46 (10) | 24 (0) | 120 (9) | 68 | 6/1/16 |
| Bedienfelder und Bedienfeldmenüs | 46 | 0/0/0 | 4 | 36 (1) | 29 (2) | 35 (0) | 42 | 0/0/0 |
| Inspektor (Elementarten, Werkzeuge) | 80 | 0/0/0 | 11 | 51 (12) | 30 (14) | 44 (5) | 79 | 0/0/0 |
| Dialoge | 102 | 0/0/0 | 29 | 26 (0) | 128 (114) | 28 (1) | 101 | 0/0/0 |
| Kontextmenüs | 13 | 0/0/0 | 10 | 9 (5) | 1 (0) | 5 (1) | 13 | 1/1/1 |
| Statuszeile | 7 | 0/0/0 | 8 | 4 (0) | 2 (1) | 4 (0) | 7 | 0/0/0 |
| **Summe** | **324** | **0/0/0** | **202** | **(28)** | **(132)** | **239 (16)** | **317** | |

Lesart: Rohschlüssel und unmarkierte Texte zählen verschiedene Texte; Überläufe zählen verschiedene (Bild, Text, Art) über alle vier Größen (1024×768 und 1920×1080, je 100 % und 150 %). In Klammern: nur die Überläufe, die es im selben Bild und derselben Größe in Deutsch nicht gibt (Folge der Übersetzung). „Überläufe de“: alle deutschen Überläufe; in Klammern die, die es im Stand vor Phase 6 (`30ddaec`, gleiche Größen) nicht gab. Der Rest bestand schon vorher und ist sprachunabhängig.

## Einzelheiten (automatisch)

### Hauptfenster (3 Designs, leer, Symbolmodus)

**Unmarkierte Texte qps (2):**

| Text | Bilder |
|---|---|
| Seite 1 | haupt:dark, haupt:dark:auswahl, haupt:normal, haupt:normal:auswahl … (+3) |
| Unbenannt-2 | haupt:dark, haupt:normal, haupt:oled, haupt:symbolmodus |

**Überläufe qps, neu gegenüber Deutsch (1):**

| Bild | Art | Text | Größen | Lage |
|---|---|---|---|---|
| haupt:symbolmodus | abgeschnitten | [!! Bildgröße... ·····!!] | 1024x768@150 | [436,411–529,423] sichtbar [436,411–522,423] |

**Deutsch geändert gegenüber dem Stand vor Phase 6 (7 Bilder), je Zeile zusammengefasst (− vorher, + jetzt; Text, x,y,Breite,Höhe):**

| Zeile | Bilder |
|---|---|
| vorher ["99 Farben\t1536,334,47,8"] → jetzt ["108 Farben\t1531,334,52,8"] | 6 haupt:dark, haupt:dark:auswahl, haupt:normal … |
| vorher ["Firepixel\t474,349,78,20", "Firepixel-Dokument (FW-PNG)\t706,424,153,12", "Neu erstellen\t705,400,72,9", "Zuletzt verwendete Datei öffnen\t433,400,176, | 1 haupt:leer |


### Menüs und Untermenüs

**Nicht herstellbar de:** menu:7 (1024x768@150): Menü „Filter“ öffnet nicht (auch nicht über „»“) · menu:7/2 (1024x768@150): Menü „Filter“ öffnet nicht (auch nicht über „»“) · menu:7/3 (1024x768@150): Menü „Filter“ öffnet nicht (auch nicht über „»“) · menu:7/4 (1024x768@150): Menü „Filter“ öffnet nicht (auch nicht über „»“) · menu:7/5 (1024x768@150): Menü „Filter“ öffnet nicht (auch nicht über „»“) · menu:7/6 (1024x768@150): Menü „Filter“ öffnet nicht (auch nicht über „»“)

**Nicht herstellbar en:** menu:9 (1024x768@150): Menü „Help“ öffnet nicht (auch nicht über „»“)

**Nicht herstellbar qps:** menu:4 (1024x768@150): Menü „[!! Modifizieren ······!!]“ öffnet nicht (auch nicht über „»“) · menu:4/0 (1024x768@150): Menü „[!! Modifizieren ······!!]“ öffnet nicht (auch nicht über „»“) · menu:4/2 (1024x768@150): Menü „[!! Modifizieren ······!!]“ öffnet nicht (auch nicht über „»“) · menu:4/3 (1024x768@150): Menü „[!! Modifizieren ······!!]“ öffnet nicht (auch nicht über „»“) · menu:4/6 (1024x768@150): Menü „[!! Modifizieren ······!!]“ öffnet nicht (auch nicht über „»“) · menu:4/7 (1024x768@150): Menü „[!! Modifizieren ······!!]“ öffnet nicht (auch nicht über „»“) · menu:4/14 (1024x768@150): Menü „[!! Modifizieren ······!!]“ öffnet nicht (auch nicht über „»“) · menu:4/15 (1024x768@150): Menü „[!! Modifizieren ······!!]“ öffnet nicht (auch nicht über „»“) · menu:4/16 (1024x768@150): Menü „[!! Modifizieren ······!!]“ öffnet nicht (auch nicht über „»“) · menu:4/19 (1024x768@150): Menü „[!! Modifizieren ······!!]“ öffnet nicht (auch nicht über „»“) · menu:4/20 (1024x768@150): Menü „[!! Modifizieren ······!!]“ öffnet nicht (auch nicht über „»“) · menu:8 (1024x768@100): Menü „[!! Fenster ····!!]“ öffnet nicht (auch nicht über „»“) · menu:8/3 (1024x768@100): Menü „[!! Fenster ····!!]“ öffnet nicht (auch nicht über „»“) · menu:8/6 (1024x768@100): Menü „[!! Fenster ····!!]“ öffnet nicht (auch nicht über „»“) · menu:8/35 (1024x768@100): Menü „[!! Fenster ····!!]“ öffnet nicht (auch nicht über „»“) · menu:4/4 (1024x768@150): Menü „[!! Modifizieren ······!!]“ öffnet nicht (auch nicht über „»“)

**Unmarkierte Texte qps (138):**

| Text | Bilder |
|---|---|
| AV | menu:5/0, menu:5/0/0, menu:5/0/1, menu:5/0/10 … (+23) |
| (Zeile entfernt: enthält einen gesperrten Namen, §1a) |
| (Zeile entfernt: enthält einen gesperrten Namen, §1a) |
| (Zeile entfernt: enthält einen gesperrten Namen, §1a) |
| (Zeile entfernt: enthält einen gesperrten Namen, §1a) |
| (Zeile entfernt: enthält einen gesperrten Namen, §1a) |
| (Zeile entfernt: enthält einen gesperrten Namen, §1a) |
| (Zeile entfernt: enthält einen gesperrten Namen, §1a) |
| (Zeile entfernt: enthält einen gesperrten Namen, §1a) |
| (Zeile entfernt: enthält einen gesperrten Namen, §1a) |
| (Zeile entfernt: enthält einen gesperrten Namen, §1a) |
| (Zeile entfernt: enthält einen gesperrten Namen, §1a) |
| (Zeile entfernt: enthält einen gesperrten Namen, §1a) |
| (Zeile entfernt: enthält einen gesperrten Namen, §1a) |
| (Zeile entfernt: enthält einen gesperrten Namen, §1a) |
| Bahnschrift | menu:5/0/1, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ B |
| Birch Std | menu:5/0/1, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ B |
| Blackoak Std | menu:5/0/1, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ B |
| Brush Script Std | menu:5/0/1, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ B |
| C | menu:5/0, menu:5/0/0, menu:5/0/1, menu:5/0/10 … (+20) |
| Cambria Math | menu:5/0/2, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ C |
| Candara | menu:5/0/2, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ C |
| Cascadia Code | menu:5/0/2, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ C |
| Cascadia Mono | menu:5/0/2, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ C |
| Chaparral Pro | menu:5/0/2, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ C |
| Charlemagne Std | menu:5/0/2, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ C |
| Comic Sans MS | menu:5/0/2, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ C |
| Constantia | menu:5/0/2, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ C |
| Cooper Std | menu:5/0/2, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ C |
| Corbel | menu:5/0/2, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ C |
| D | menu:5/0, menu:5/0/0, menu:5/0/1, menu:5/0/10 … (+20) |
| Deutsch | menu:2/24, menübaum: [!! Ansicht ····!!] ▸ [!! Sprache ····!!] |
| Dotum | menu:5/0/3, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ D |
| DotumChe | menu:5/0/3, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ D |
| E | menu:5/0, menu:5/0/0, menu:5/0/1, menu:5/0/10 … (+20) |
| Ebrima | menu:5/0/4, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ E |
| English | menu:2/24, menübaum: [!! Ansicht ····!!] ▸ [!! Sprache ····!!] |
| F | menu:5/0, menu:5/0/0, menu:5/0/1, menu:5/0/10 … (+20) |
| F1 | menu:9, menübaum: [!! Hilfe ···!!] ▸ [!! Firepixel-Hilfe ·······!!] (Kürzel) |
| F11 | menu:8, menu:8/3, menu:8/35, menu:8/6 … (+1) |
| F12 | menu:0/25, menübaum: [!! Datei ···!!] ▸ [!! Vorschau im Browser ········!!] ▸ [!! Vorschau im Primärbrowser ··········!!] (Kürzel) |
| F2 | menu:8, menu:8/3, menu:8/35, menu:8/6 … (+1) |
| F4 | menu:8, menu:8/3, menu:8/35, menu:8/6 … (+1) |
| F5 | menu:8, menu:8/3, menu:8/35, menu:8/6 … (+1) |
| F6 | menu:8, menu:8/3, menu:8/35, menu:8/6 … (+1) |
| F7 | menu:8, menu:8/3, menu:8/35, menu:8/6 … (+1) |
| F8 | menu:4/3, menübaum: [!! Modifizieren ······!!] ▸ [!! Symbol ···!!] ▸ [!! In Symbol konvertieren... ··········!!] (Kürzel) |
| Franklin Gothic Medium | menu:5/0/5, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ F |
| Gabriola | menu:5/0/6, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ G |
| Gadugi | menu:5/0/6, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ G |
| Giddyup Std | menu:5/0/6, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ G |
| Gulim | menu:5/0/6, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ G |
| GulimChe | menu:5/0/6, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ G |
| Hobo Std | menu:5/0/7, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ H |
| I | menu:5/0, menu:5/0/0, menu:5/0/1, menu:5/0/10 … (+24) |
| Ink Free | menu:5/0/8, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ I |
| J | menu:5/0, menu:5/0/0, menu:5/0/1, menu:5/0/10 … (+20) |
| Javanese Text | menu:5/0/9, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ J |
| Kozuka Gothic Pr6N | menu:5/0/10, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ K |
| Kozuka Gothic Pro | menu:5/0/10, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ K |
| Kozuka Mincho Pr6N | menu:5/0/10, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ K |
| Kozuka Mincho Pro | menu:5/0/10, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ K |
| L | menu:5/0, menu:5/0/0, menu:5/0/1, menu:5/0/10 … (+20) |
| Leelawadee UI | menu:5/0/11, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ L |
| Letter Gothic Std | menu:5/0/11, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ L |
| Lithos Pro | menu:5/0/11, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ L |
| Lucida Console | menu:5/0/11, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ L |
| Lucida Sans Unicode | menu:5/0/11, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ L |
| MS Gothic | menu:5/0/12, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ M |
| MS PGothic | menu:5/0/12, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ M |
| MS UI Gothic | menu:5/0/12, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ M |
| MV Boli | menu:5/0/12, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ M |
| Malgun Gothic | menu:5/0/12, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ M |
| Mesquite Std | menu:5/0/12, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ M |
| Microsoft Himalaya | menu:5/0/12, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ M |
| Microsoft JhengHei | menu:5/0/12, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ M |
| Microsoft JhengHei UI | menu:5/0/12, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ M |
| Microsoft New Tai Lue | menu:5/0/12, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ M |
| Microsoft PhagsPa | menu:5/0/12, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ M |
| Microsoft Sans Serif | menu:5/0/12, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ M |
| Microsoft Tai Le | menu:5/0/12, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ M |
| Microsoft YaHei | menu:5/0/12, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ M |
| Microsoft YaHei UI | menu:5/0/12, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ M |
| Microsoft Yi Baiti | menu:5/0/12, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ M |
| MingLiU | menu:5/0/12, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ M |
| MingLiU-ExtB | menu:5/0/12, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ M |
| MingLiU_HKSCS | menu:5/0/12, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ M |
| MingLiU_HKSCS-ExtB | menu:5/0/12, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ M |
| MingLiU_MSCS | menu:5/0/12, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ M |
| MingLiU_MSCS-ExtB | menu:5/0/12, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ M |
| Minion Pro | menu:5/0/12, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ M |
| Mongolian Baiti | menu:5/0/12, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ M |
| Myanmar Text | menu:5/0/12, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ M |
| Myriad Arabic | menu:5/0/12, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ M |
| Myriad Hebrew | menu:5/0/12, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ M |
| Myriad Pro | menu:5/0, menu:5/0/0, menu:5/0/1, menu:5/0/10 … (+24) |
| N | menu:5/0, menu:5/0/0, menu:5/0/1, menu:5/0/10 … (+20) |
| NSimSun | menu:5/0/13, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ N |
| Nirmala Text | menu:5/0/13, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ N |
| Nirmala UI | menu:5/0/13, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ N |
| Nueva Std | menu:5/0/13, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ N |
| O | menu:5/0, menu:5/0/0, menu:5/0/1, menu:5/0/10 … (+20) |
| OCR A Std | menu:5/0/14, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ O |
| Orator Std | menu:5/0/14, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ O |
| P | menu:5/0, menu:5/0/0, menu:5/0/1, menu:5/0/10 … (+20) |
| PMingLiU | menu:5/0/15, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ P |
| PMingLiU-ExtB | menu:5/0/15, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ P |
| Palatino Linotype | menu:5/0/15, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ P |
| Poplar Std | menu:5/0/15, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ P |
| Prestige Elite Std | menu:5/0/15, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ P |
| Rosewood Std | menu:5/0/16, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ R |
| S | menu:5/0, menu:5/0/0, menu:5/0/1, menu:5/0/10 … (+20) |
| Sans Serif Collection | menu:5/0/17, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ S |
| Segoe Fluent Icons | menu:5/0/17, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ S |
| Segoe MDL2 Assets | menu:5/0/17, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ S |
| Segoe Print | menu:5/0/17, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ S |
| Segoe Script | menu:5/0/17, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ S |
| Segoe UI Emoji | menu:5/0/17, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ S |
| Segoe UI Historic | menu:5/0/17, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ S |
| Segoe UI Variable | menu:5/0/17, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ S |
| Seite 1 | menu:0, menu:0/18, menu:0/25, menu:0/4 … (+64) |
| SimSun | menu:5/0/17, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ S |
| SimSun-ExtB | menu:5/0/17, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ S |
| SimSun-ExtG | menu:5/0/17, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ S |
| Sitka | menu:5/0/17, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ S |
| Stencil Std | menu:5/0/17, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ S |
| Sylfaen | menu:5/0/17, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ S |
| T | menu:5/0, menu:5/0/0, menu:5/0/1, menu:5/0/10 … (+24) |
| Tekton Pro | menu:5/0/18, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ T |
| Testbefehl | menu:6, menübaum: [!! Befehle ····!!] |
| Testbefehl2 | menu:6, menübaum: [!! Befehle ····!!] |
| Trajan Pro | menu:5/0/18, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ T |
| Trebuchet MS | menu:5/0/18, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ T |
| U | menu:5/0, menu:5/0/0, menu:5/0/1, menu:5/0/10 … (+23) |
| V | menu:5/0, menu:5/0/0, menu:5/0/1, menu:5/0/10 … (+20) |
| Yu Gothic | menu:5/0/21, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ Y |
| Yu Gothic UI | menu:5/0/21, menübaum: [!! Text ··!!] ▸ [!! Schrift ····!!] ▸ Y |
| a | menu:5/0, menu:5/0/0, menu:5/0/1, menu:5/0/10 … (+23) |

**Überläufe en, neu gegenüber Deutsch (10):**

| Bild | Art | Text | Größen | Lage |
|---|---|---|---|---|
| menu:1 | abgeschnitten | Texture: | 1024x768@150 | [390,459–429,467] sichtbar [390,459–401,467] |
| menu:1/3 | abgeschnitten | Texture: | 1024x768@150 | [390,459–429,467] sichtbar [390,459–401,467] |
| menu:4/14 | in_element | Numeric Transform... | 1024x768@150 | Text [452,335–560,344] Element [542,268–683,512] |
| menu:4/14 | in_element | Remove Transformations | 1024x768@150 | Text [452,494–582,503] Element [542,268–683,512] |
| menu:7 | abgeschnitten | Texture: | 1024x768@150 | [390,459–429,467] sichtbar [390,459–401,467] |
| menu:7/2 | abgeschnitten | Texture: | 1024x768@150 | [390,459–429,467] sichtbar [390,459–401,467] |
| menu:7/3 | abgeschnitten | Texture: | 1024x768@150 | [390,459–429,467] sichtbar [390,459–401,467] |
| menu:7/4 | abgeschnitten | Texture: | 1024x768@150 | [390,459–429,467] sichtbar [390,459–401,467] |
| menu:7/5 | abgeschnitten | Texture: | 1024x768@150 | [390,459–429,467] sichtbar [390,459–401,467] |
| menu:7/6 | abgeschnitten | Texture: | 1024x768@150 | [390,459–429,467] sichtbar [390,459–401,467] |

**Deutsch geändert gegenüber dem Stand vor Phase 6 (68 Bilder), je Zeile zusammengefasst (− vorher, + jetzt; Text, x,y,Breite,Höhe):**

| Zeile | Bilder |
|---|---|
| vorher ["Pinselstrich bearbeit…\t395,971,109,9"] → jetzt ["Pinselstrich bearbeiten\t395,972,108,8"] | 38 menu:0, menu:0/18, menu:0/25 … |
| vorher ["202\t110,948,19,9", "306\t162,970,19,9", "Autom. Unterschneidung\t467,972,127,12"] → jetzt ["139\t111,948,18,9", "309\t162,970,19,9", "Autom. Unterschn | 27 menu:5/0, menu:5/0/0, menu:5/0/1 … |
| neu (nicht in der Basis) | 1 menu:2/24 |
| vorher ["1 C:\\Users\\ADMINI~1\\AppData\\Local\\Temp\\firepixel-i18n-qa\\X\\vorlage.fw.png\t368,133,465,12", "Pinselstrich bearbeit…\t395,971,109,9"] → jetzt [" | 1 menu:0/4 |
| vorher ["Bearbeiten...\t415,947,61,9"] → jetzt ["Bearbeiten...\t381,947,61,9"] | 1 menu:4/4 |


### Bedienfelder und Bedienfeldmenüs

**Unmarkierte Texte qps (4):**

| Text | Bilder |
|---|---|
| 01  Seite 1 | panel:Pages, pmenu:Pages |
| Seite 1 | panel:Align, panel:AutoShapeProperties, panel:AutoShapes, panel:Behaviors … (+42) |
| URLs.htm | panel:Url, pmenu:Url |
| width: 160px;⏎height: 120px;⏎background-color: #ff0000; | panel:CssProperties, pmenu:CssProperties |

**Überläufe en, neu gegenüber Deutsch (1):**

| Bild | Art | Text | Größen | Lage |
|---|---|---|---|---|
| pmenu:Layers | abgeschnitten | Texture: | 1024x768@150 | [390,459–429,467] sichtbar [390,459–404,467] |

**Überläufe qps, neu gegenüber Deutsch (2):**

| Bild | Art | Text | Größen | Lage |
|---|---|---|---|---|
| panel:AutoShapeProperties | fensterrand | [!! Eigenschaften für automatische Form ·············!!] | 1024x768@150 | [490,287–749,298] Fenster [0,0–683,512] |
| pmenu:AutoShapeProperties | fensterrand | [!! Eigenschaften für automatische Form ·············!!] | 1024x768@150 | [490,287–749,298] Fenster [0,0–683,512] |

**Deutsch geändert gegenüber dem Stand vor Phase 6 (42 Bilder), je Zeile zusammengefasst (− vorher, + jetzt; Text, x,y,Breite,Höhe):**

| Zeile | Bilder |
|---|---|
| vorher ["99 Farben\t1536,334,47,8"] → jetzt ["108 Farben\t1531,334,52,8"] | 29 panel:AutoShapeProperties, panel:Behaviors, panel:ColorMixer … |
| vorher ["99 Farben\t1536,290,47,8"] → jetzt ["108 Farben\t1531,290,52,8"] | 10 panel:AutoShapes, panel:CommonLibrary, panel:DocumentLibrary … |
| vorher ["99 Farben\t1536,290,47,8", "Ausstanzen\t1431,1020,58,9", "Kombinieren\t1339,1044,63,9", "Vereinigen\t1343,1020,54,12", "Zuschneiden\t1608,1020,64,9", " | 2 panel:Path, pmenu:Path |
| vorher ["99 Farben\t1536,334,47,8", "CMY\t644,254,25,9", "Graustufen\t644,298,58,9", "HSB\t645,276,21,9", "Hexadezimal\t645,232,66,9", "Hilfe\t645,329,23,9"] →  | 1 pmenu:ColorMixer |


### Inspektor (Elementarten, Werkzeuge)

**Unmarkierte Texte qps (11):**

| Text | Bilder |
|---|---|
| AV | pi:Text, pi:Text auf Pfad, werkzeug:tool.text |
| BiRefNet lite (allgemein) (178 MB) | werkzeug:tool.aiSubject |
| BiRefNet lite Matting (89 MB) | werkzeug:tool.aiRefineEdge |
| I | pi:Text, pi:Text auf Pfad, werkzeug:tool.text |
| Myriad Pro | pi:Text, pi:Text auf Pfad, werkzeug:tool.text |
| SAM 2.1 Hiera Tiny (156 MB) | werkzeug:tool.aiObject |
| Seite 1 | pi:Animationssymbol, pi:Automatische Form, pi:Bitmap, pi:Bitmap + Pixelauswahl … (+75) |
| T | pi:Text, pi:Text auf Pfad, werkzeug:tool.text |
| U | pi:Text, pi:Text auf Pfad, werkzeug:tool.text |
| Unbenannt-2 | pi:Masterseite, pi:Pixelauswahl, pi:importiertes Symbol, pi:nach Rückgängig … (+10) |
| a | pi:Text, pi:Text auf Pfad, werkzeug:tool.text |

**Überläufe en, neu gegenüber Deutsch (12):**

| Bild | Art | Text | Größen | Lage |
|---|---|---|---|---|
| pi:Schaltfläche | abgeschnitten | Show Down state on load | 1024x768@100 | [492,736–622,746] sichtbar [492,736–561,746] |
| werkzeug:tool.aiObject | abgeschnitten | SAM 2.1 Hiera Tiny (156 MB) | 1024x768@100 | [441,664–586,676] sichtbar [441,664–561,676] |
| werkzeug:tool.aiRefineEdge | abgeschnitten | BiRefNet lite Matting (89 MB) | 1024x768@100 | [442,664–589,676] sichtbar [442,664–561,676] |
| werkzeug:tool.aiRefineEdge | abgeschnitten | Create a pixel selection first | 1024x768@100 | [441,712–580,724] sichtbar [441,712–561,724] |
| werkzeug:tool.aiSubject | abgeschnitten | BiRefNet lite (allgemein) (178 MB) | 1024x768@100 | [442,664–612,676] sichtbar [442,664–561,676] |
| werkzeug:tool.burn | abgeschnitten | Exposure: | 1024x768@100 | [554,666–602,678] sichtbar [554,666–561,678] |
| werkzeug:tool.dodge | abgeschnitten | Exposure: | 1024x768@100 | [554,666–602,678] sichtbar [554,666–561,678] |
| werkzeug:tool.eraser | abgeschnitten | Shape: | 1024x768@150 | [372,409–405,420] sichtbar [372,409–401,420] |
| werkzeug:tool.gradient | abgeschnitten | Tolerance: | 1024x768@150 | [398,409–449,417] sichtbar [398,409–401,417] |
| werkzeug:tool.paintBucket | abgeschnitten | Tolerance: | 1024x768@150 | [398,409–449,417] sichtbar [398,409–401,417] |
| werkzeug:tool.replaceColor | abgeschnitten | Strength: | 1024x768@100 | [528,689–574,701] sichtbar [528,689–561,701] |
| werkzeug:tool.replaceColor | abgeschnitten | Tolerance: | 1024x768@100 | [522,664–572,673] sichtbar [522,664–561,673] |

**Überläufe qps, neu gegenüber Deutsch (14):**

| Bild | Art | Text | Größen | Lage |
|---|---|---|---|---|
| werkzeug:tool.aiObject | abgeschnitten | SAM 2.1 Hiera Tiny (156 MB) | 1024x768@100 | [441,664–586,676] sichtbar [441,664–561,676] |
| werkzeug:tool.aiObject | abgeschnitten | [!! Klick: Objekt auswählen · Rahmen aufziehen: Objekt im Rahmen ·····················!!] | 1024x768@100 | [208,712–607,724] sichtbar [208,712–561,724] |
| werkzeug:tool.aiObject | abgeschnitten | [!! Umschalt+Klick: hinzufügen · Alt+Klick: abziehen · Esc: neues Objekt ························!!] | 1024x768@100, 1024x768@150 | [208,732–636,744] sichtbar [208,732–561,744] |
| werkzeug:tool.aiRefineEdge | abgeschnitten | BiRefNet lite Matting (89 MB) | 1024x768@100 | [442,664–589,676] sichtbar [442,664–561,676] |
| werkzeug:tool.aiRefineEdge | abgeschnitten | [!! Erst eine Pixelauswahl erstellen ············!!] | 1024x768@100 | [442,712–653,723] sichtbar [442,712–561,723] |
| werkzeug:tool.aiSubject | abgeschnitten | BiRefNet lite (allgemein) (178 MB) | 1024x768@100 | [442,664–612,676] sichtbar [442,664–561,676] |
| werkzeug:tool.aiSubject | abgeschnitten | [!! Klick: Hauptmotiv als weiche Pixelauswahl · Umschalt: hinzufügen · Alt: abziehen ····························!!] | 1024x768@100 | [208,713–684,723] sichtbar [208,713–561,723] |
| werkzeug:tool.aiSubject | fensterrand | [!! Klick: Hauptmotiv als weiche Pixelauswahl · Umschalt: hinzufügen · Alt: abziehen ····························!!] | 1024x768@150 | [208,457–684,468] Fenster [0,0–683,512] |
| werkzeug:tool.eraser | abgeschnitten | [!! Form: ··!!] | 1024x768@150 | [346,409–404,419] sichtbar [346,409–401,419] |
| werkzeug:tool.freeform | abgeschnitten | [!! Druck ··!!] | 1024x768@150 | [348,409–407,420] sichtbar [348,409–401,420] |
| werkzeug:tool.freeform | abgeschnitten | [!! Vorschau ····!!] | 1024x768@150 | [348,428–428,439] sichtbar [348,428–401,439] |
| werkzeug:tool.magicWand | abgeschnitten | [!! Hart ··!!] | 1024x768@150 | [374,410–425,420] sichtbar [374,410–401,420] |
| werkzeug:tool.replaceColor | abgeschnitten | [!! Toleranz: ····!!] | 1024x768@100 | [497,665–573,675] sichtbar [497,665–561,675] |
| werkzeug:tool.rubberStamp | abgeschnitten | [!! Gesamtes Dokument ·······!!] | 1024x768@100 | [417,688–565,700] sichtbar [417,688–561,700] |

**Deutsch geändert gegenüber dem Stand vor Phase 6 (79 Bilder), je Zeile zusammengefasst (− vorher, + jetzt; Text, x,y,Breite,Höhe):**

| Zeile | Bilder |
|---|---|
| vorher ["99 Farben\t1536,334,47,8"] → jetzt ["108 Farben\t1531,334,52,8"] | 50 pi:Animationssymbol, pi:Automatische Form, pi:Bitmap … |
| vorher ["99 Farben\t1536,334,47,8", "Alphakanal\t658,946,56,12", "Graustufen\t658,966,56,9", "Maske:\t586,944,34,9"] → jetzt ["108 Farben\t1531,334,52,8"] | 9 werkzeug:tool.eyedropper, werkzeug:tool.gradient, werkzeug:tool.lasso … |
| vorher ["0\t291,921,6,9", "10\t292,897,12,9", "99 Farben\t1536,334,47,8", "Alphakanal\t658,946,56,12", "Graustufen\t658,966,56,9", "Maske:\t586,944,34,9"] → jet | 4 werkzeug:tool.blur, werkzeug:tool.burn, werkzeug:tool.dodge … |
| vorher ["1\t348,897,4,9", "99 Farben\t1536,334,47,8"] → jetzt ["10\t348,897,12,9", "108 Farben\t1531,334,52,8"] | 2 werkzeug:tool.pathScrubAdd, werkzeug:tool.pathScrubSubtract |
| vorher ["99 Farben\t1536,334,47,8", "Teststil\t801,945,34,9"] → jetzt ["108 Farben\t1531,334,52,8"] | 2 pi:Rechteck + Ellipse, pi:drei Objekte |
| vorher ["0\t291,921,6,9", "10\t292,897,12,9", "99 Farben\t1536,334,47,8", "Alphakanal\t658,946,56,12", "Graustufen\t658,966,56,9", "Maske:\t586,944,34,9"] → jet | 1 werkzeug:tool.rubberStamp |
| vorher ["0\t291,921,6,9", "10\t292,897,12,9", "99 Farben\t1536,334,47,8", "Alphakanal\t658,946,56,12", "Graustufen\t658,966,56,9", "Maske:\t586,944,34,9"] → jet | 1 werkzeug:tool.smudge |
| vorher ["0\t291,921,6,9", "99 Farben\t1536,334,47,8", "Alphakanal\t658,946,56,12", "Graustufen\t658,966,56,9", "Maske:\t586,944,34,9"] → jetzt ["108 Farben\t153 | 1 werkzeug:tool.eraser |
| vorher ["100\t309,921,18,9", "202\t110,948,19,9", "29\t110,970,13,9", "306\t162,970,19,9", "40\t162,948,13,9", "71 Farben\t1536,334,47,8"] → jetzt ["111\t111,94 | 1 pi:Text auf Pfad |
| vorher ["100\t309,921,18,9", "202\t110,948,19,9", "29\t110,970,13,9", "306\t162,970,19,9", "99 Farben\t1536,334,47,8", "Arial\t211,897,22,9"] → jetzt ["108 Farb | 1 pi:Text |
| vorher ["10\t265,897,12,9", "50\t264,921,13,9", "99 Farben\t1536,334,47,8"] → jetzt ["108 Farben\t1531,334,52,8", "72\t264,897,13,9", "80\t264,921,13,9"] | 1 werkzeug:tool.reshapeArea |
| vorher ["10\t265,897,12,9", "99 Farben\t1536,334,47,8"] → jetzt ["108 Farben\t1531,334,52,8", "72\t264,897,13,9"] | 1 werkzeug:tool.freeform |
| vorher ["10\t292,897,12,9", "50\t579,921,13,9", "70\t579,897,13,9", "99 Farben\t1536,334,47,8", "Alphakanal\t658,946,56,12", "Graustufen\t658,966,56,9"] → jetzt | 1 werkzeug:tool.replaceColor |
| vorher ["295\t110,970,19,9", "99 Farben\t1536,334,47,8"] → jetzt ["108 Farben\t1531,334,52,8", "296\t110,970,19,9"] | 1 pi:Text + Ellipse |
| vorher ["70\t282,897,13,9", "99 Farben\t1536,334,47,8", "Alphakanal\t658,946,56,12", "Graustufen\t658,966,56,9", "Maske:\t586,944,34,9"] → jetzt ["108 Farben\t1 | 1 werkzeug:tool.redEyeRemoval |
| vorher ["99 Farben\t1536,334,47,8", "Alphakanal\t664,946,56,12", "Graustufen\t664,966,56,9", "Maske:\t592,944,34,9"] → jetzt ["108 Farben\t1531,334,52,8"] | 1 werkzeug:tool.brush |
| vorher ["99 Farben\t1536,334,47,8", "Arial\t211,897,22,9", "Regular\t389,897,38,12"] → jetzt ["108 Farben\t1531,334,52,8", "Bold\t389,897,22,9", "Myriad Pro\t21 | 1 werkzeug:tool.text |


### Dialoge

**Unmarkierte Texte qps (29):**

| Text | Bilder |
|---|---|
| .htm | dlg:form:4766 |
| A4 | dlg:form:10397 |
| AV | dlg:form:6073 |
| Apache-2.0 | dlg:ki_verwalten |
| BiRefNet (allgemein, hohe Qualität) | dlg:ki_verwalten |
| BiRefNet HR Matting (hohe Qualität) | dlg:ki_verwalten |
| F1 | dlg:tastenkuerzel |
| F11 | dlg:tastenkuerzel |
| F12 | dlg:tastenkuerzel |
| F2 | dlg:tastenkuerzel |
| F5 | dlg:tastenkuerzel |
| F6 | dlg:tastenkuerzel |
| F7 | dlg:tastenkuerzel |
| F8 | dlg:tastenkuerzel |
| I | dlg:form:6073 |
| MIT | dlg:ki_verwalten |
| Mein Arbeitsbereich | dlg:arbeitsbereich_speichern |
| Myriad Pro | dlg:form:6073 |
| SAM 2.1 Hiera Small | dlg:ki_verwalten |
| Seite 1 | dlg:arbeitsbereich_speichern, dlg:befehle_verwalten, dlg:bildgroesse, dlg:bildvorschau … (+98) |
| T | dlg:form:6073 |
| Testbefehl | dlg:befehle_verwalten |
| Testbefehl2 | dlg:befehle_verwalten |
| U | dlg:form:6073 |
| Unbenannt-2 | dlg:arbeitsbereich_speichern, dlg:befehle_verwalten, dlg:bildgroesse, dlg:bildvorschau … (+50) |
| Web Standard | dlg:tastenkuerzel |
| a | dlg:form:6073 |
| de_DE | dlg:form:6516 |
| mm | dlg:form:10397 |

**Überläufe qps, neu gegenüber Deutsch (114):**

| Bild | Art | Text | Größen | Lage |
|---|---|---|---|---|
| dlg:bildvorschau | fensterrand | [!! Farben: ···!!] | 1024x768@150 | [-58,131–11,142] Fenster [0,0–683,512] |
| dlg:bildvorschau | fensterrand | [!! Matte: ···!!] | 1024x768@150 | [-58,188–6,198] Fenster [0,0–683,512] |
| dlg:bildvorschau | fensterrand | [!! Mit Zeilensprung / progressiv ···········!!] | 1024x768@150 | [-40,213–161,225] Fenster [0,0–683,512] |
| dlg:bildvorschau | fensterrand | [!! Palette: ····!!] | 1024x768@150 | [-58,103–14,114] Fenster [0,0–683,512] |
| dlg:bildvorschau | fensterrand | [!! Verlust: ····!!] | 1024x768@150 | [-58,159–14,170] Fenster [0,0–683,512] |
| dlg:form:19128 | fensterrand | [!! Ebene duplizieren ·······!!] | 1024x768@150 | [-4,196–135,208] Fenster [0,0–683,512] |
| dlg:form:5990 | fensterrand | [!! Abbrechen ····!!] | 1024x768@150 | [616,298–706,309] Fenster [0,0–683,512] |
| dlg:form:5990 | fensterrand | [!! Am Ende ···!!] | 1024x768@150 | [619,259–697,269] Fenster [0,0–683,512] |
| dlg:form:5990 | fensterrand | [!! Anzahl: ···!!] | 1024x768@150 | [-32,233–37,243] Fenster [0,0–683,512] |
| dlg:form:5990 | fensterrand | [!! Einfügen: ····!!] | 1024x768@150 | [-32,259–50,270] Fenster [0,0–683,512] |
| dlg:form:5990 | fensterrand | [!! Status hinzufügen ·······!!] | 1024x768@150 | [-36,196–102,208] Fenster [0,0–683,512] |
| dlg:ki_verwalten | abgeschnitten | BiRefNet HR Matting (hohe Qualität) | 1024x768@100, 1920x1080@100, 1920x1080@150 | [139,386–323,398] sichtbar [139,386–318,398] |
| dlg:ki_verwalten | abgeschnitten | [!! BiRefNet lite (allgemein)  (Vorgabe) ·············!!] | 1024x768@100, 1920x1080@100, 1920x1080@150 | [139,278–374,290] sichtbar [139,278–318,290] |
| dlg:ki_verwalten | abgeschnitten | [!! BiRefNet lite Matting  (Vorgabe) ············!!] | 1024x768@100, 1920x1080@100, 1920x1080@150 | [139,350–355,362] sichtbar [139,350–318,362] |
| dlg:ki_verwalten | abgeschnitten | [!! SAM 2.1 Hiera Tiny  (Vorgabe) ··········!!] | 1024x768@100, 1920x1080@100, 1920x1080@150 | [139,206–340,218] sichtbar [139,206–318,218] |
| dlg:ki_verwalten | fensterrand | [!! Aktuell: C:\Users\Administrator\AppData\Local\Firepixel\Modelle ·························!!] | 1024x768@150 | [-31,420–383,431] Fenster [0,0–683,512] |
| dlg:ki_verwalten | fensterrand | [!! Ordner öffnen ·····!!] | 1024x768@150 | [597,422–706,432] Fenster [0,0–683,512] |
| dlg:ki_verwalten | fensterrand | [!! Standard ····!!] | 1024x768@150 | [607,394–687,404] Fenster [0,0–683,512] |
| dlg:ki_verwalten | fensterrand | [!! Zwischenspeicher im Arbeitsspeicher: 0 MB ···············!!] | 1024x768@150 | [-42,467–238,478] Fenster [0,0–683,512] |
| dlg:ki_verwalten | fensterrand | [!! Zwischenspeicher leeren ·········!!] | 1024x768@150 | [548,469–719,480] Fenster [0,0–683,512] |
| dlg:ki_verwalten | fensterrand | [!! leer = %LOCALAPPDATA%\Firepixel\Modelle ···············!!] | 1024x768@150 | [-27,392–260,403] Fenster [0,0–683,512] |
| dlg:tastenkuerzel | fensterrand | [!! Aktueller Satz: ······!!] | 1024x768@150 | [-62,49–49,60] Fenster [0,0–683,512] |
| dlg:tastenkuerzel | fensterrand | [!! Ansicht ····!!] ▸ [!! Alles anpassen ······!!] | 1024x768@150 | [-55,109–149,121] Fenster [0,0–683,512] |
| dlg:tastenkuerzel | fensterrand | [!! Ansicht ····!!] ▸ [!! Alles einblenden ·······!!] | 1024x768@150 | [-55,127–161,138] Fenster [0,0–683,512] |
| dlg:tastenkuerzel | fensterrand | [!! Ansicht ····!!] ▸ [!! Auswahl anpassen ·······!!] | 1024x768@150 | [-55,145–169,157] Fenster [0,0–683,512] |
| dlg:tastenkuerzel | fensterrand | [!! Ansicht ····!!] ▸ [!! Auswahl ausblenden ········!!] | 1024x768@150 | [-55,163–183,174] Fenster [0,0–683,512] |
| dlg:tastenkuerzel | fensterrand | [!! Ansicht ····!!] ▸ [!! Gamma 1.8 ····!!] | 1024x768@150 | [-55,181–127,192] Fenster [0,0–683,512] |
| dlg:tastenkuerzel | fensterrand | [!! Ansicht ····!!] ▸ [!! Größer zoomen ······!!] | 1024x768@150 | [-55,199–153,210] Fenster [0,0–683,512] |
| dlg:tastenkuerzel | fensterrand | [!! Ansicht ····!!] ▸ [!! Hilfslinien für 9-Slice-Skalierung ·············!!] ▸ [!! Hilfslinien sperren ········!!] | 1024x768@150 | [-55,217–407,229] Fenster [0,0–683,512] |
| dlg:tastenkuerzel | fensterrand | [!! Ansicht ····!!] ▸ [!! Hilfslinien ·····!!] ▸ [!! An Hilfslinien ausrichten ··········!!] | 1024x768@150 | [-55,235–311,246] Fenster [0,0–683,512] |
| dlg:tastenkuerzel | fensterrand | [!! Ansicht ····!!] ▸ [!! Hilfslinien ·····!!] ▸ [!! Hilfslinien anzeigen ········!!] | 1024x768@150 | [-55,253–282,265] Fenster [0,0–683,512] |
| dlg:tastenkuerzel | fensterrand | [!! Ansicht ····!!] ▸ [!! Hilfslinien ·····!!] ▸ [!! Hilfslinien entfernen ········!!] | 1024x768@150 | [-55,271–285,282] Fenster [0,0–683,512] |
| dlg:tastenkuerzel | fensterrand | [!! Ansicht ····!!] ▸ [!! Hilfslinien ·····!!] ▸ [!! Hilfslinien sperren ········!!] | 1024x768@150 | [-55,289–275,301] Fenster [0,0–683,512] |
| dlg:tastenkuerzel | fensterrand | [!! Ansicht ····!!] ▸ [!! Intelligente Hilfslinien ··········!!] ▸ [!! An intelligenten Hilfslinien ausrichten ···············!!] | 1024x768@150 | [-55,307–463,319] Fenster [0,0–683,512] |
| dlg:tastenkuerzel | fensterrand | [!! Ansicht ····!!] ▸ [!! Intelligente Hilfslinien ··········!!] ▸ [!! Intelligente Hilfslinien anzeigen ·············!!] | 1024x768@150 | [-55,325–427,337] Fenster [0,0–683,512] |
| dlg:tastenkuerzel | fensterrand | [!! Ansicht ····!!] ▸ [!! Kleiner zoomen ······!!] | 1024x768@150 | [-55,343–153,354] Fenster [0,0–683,512] |
| dlg:tastenkuerzel | fensterrand | [!! Ansicht ····!!] ▸ [!! Lineale ····!!] | 1024x768@150 | [-55,361–105,372] Fenster [0,0–683,512] |
| dlg:tastenkuerzel | fensterrand | [!! Ansicht ····!!] ▸ [!! QuickInfos ·····!!] | 1024x768@150 | [-55,379–125,390] Fenster [0,0–683,512] |
| dlg:tastenkuerzel | fensterrand | [!! Ansicht ····!!] ▸ [!! Raster ···!!] ▸ [!! Am Raster ausrichten ········!!] | 1024x768@150 | [-55,397–268,408] Fenster [0,0–683,512] |
| dlg:tastenkuerzel | fensterrand | [!! Ansicht ····!!] ▸ [!! Raster ···!!] ▸ [!! Raster anzeigen ······!!] | 1024x768@150 | [-55,415–236,427] Fenster [0,0–683,512] |
| dlg:tastenkuerzel | fensterrand | [!! Löschen ···!!] | 1024x768@150 | [646,49–719,60] Fenster [0,0–683,512] |
| dlg:tastenkuerzel | fensterrand | [!! Menübefehle ·····!!] | 1024x768@150 | [-58,79–46,90] Fenster [0,0–683,512] |
| dlg:tastenkuerzel | fensterrand | [!! Strg ··!!]++, [!! Strg ··!!]+[!! Num ··!!] +, [!! … | 1024x768@150 | [496,200–722,211] Fenster [0,0–683,512] |
| dlg:tastenkuerzel | fensterrand | [!! Strg ··!!]+-, [!! Strg ··!!]+[!! Num ··!!] - | 1024x768@150 | [496,344–688,355] Fenster [0,0–683,512] |
| dlg:tastenkuerzel | fensterrand | [!! Strg ··!!]+0, [!! Strg ··!!]+[!! Num ··!!] 0 | 1024x768@150 | [496,110–692,121] Fenster [0,0–683,512] |
| dlg:tastenkuerzel | fensterrand | [!! Strg ··!!]+[!! Alt ··!!]+0, [!! Strg ··!!]+[!! Alt ··!… | 1024x768@150 | [496,145–726,157] Fenster [0,0–683,512] |
| dlg:tastenkuerzel | fensterrand | [!! Strg ··!!]+[!! Alt ··!!]+[!! Umschalt ····!!]+G | 1024x768@150 | [496,397–707,409] Fenster [0,0–683,512] |
| dlg:tastenkuerzel | fensterrand | [!! Tastaturkurzbefehle ········!!] | 1024x768@150 | [-66,11–84,22] Fenster [0,0–683,512] |
| dlg:tastenkuerzel | fensterrand | [!! Tastenkürzel: ······!!] | 1024x768@150 | [-62,453–42,464] Fenster [0,0–683,512] |
| dlg:voreinstellungen | fensterrand | [!! Allgemein ····!!] | 1024x768@150 | [-19,49–67,60] Fenster [0,0–683,512] |
| dlg:voreinstellungen | fensterrand | [!! Bearbeiten ····!!] | 1024x768@150 | [-19,70–71,80] Fenster [0,0–683,512] |
| dlg:voreinstellungen | fensterrand | [!! Hilfslinien und Raster ········!!] | 1024x768@150 | [-19,91–133,101] Fenster [0,0–683,512] |
| dlg:voreinstellungen | fensterrand | [!! Photoshop-Import/Öffnen ··········!!] | 1024x768@150 | [-19,131–165,144] Fenster [0,0–683,512] |
| dlg:voreinstellungen | fensterrand | [!! Starten und Bearbeiten ········!!] | 1024x768@150 | [-19,175–143,185] Fenster [0,0–683,512] |
| dlg:voreinstellungen | fensterrand | [!! Text ··!!] | 1024x768@150 | [-19,112–31,122] Fenster [0,0–683,512] |
| dlg:voreinstellungen | fensterrand | [!! Voreinstellungen ·······!!] | 1024x768@150 | [-28,11–105,23] Fenster [0,0–683,512] |
| dlg:voreinstellungen | fensterrand | [!! Zusatzmodule ·····!!] | 1024x768@150 | [-19,154–90,164] Fenster [0,0–683,512] |
| dlg:voreinstellungen:0 | fensterrand | [!! Allgemein ····!!] | 1024x768@150 | [-19,49–67,60] Fenster [0,0–683,512] |
| dlg:voreinstellungen:0 | fensterrand | [!! Bearbeiten ····!!] | 1024x768@150 | [-19,70–71,80] Fenster [0,0–683,512] |
| dlg:voreinstellungen:0 | fensterrand | [!! Hilfslinien und Raster ········!!] | 1024x768@150 | [-19,91–133,101] Fenster [0,0–683,512] |
| dlg:voreinstellungen:0 | fensterrand | [!! Photoshop-Import/Öffnen ··········!!] | 1024x768@150 | [-19,131–165,144] Fenster [0,0–683,512] |
| dlg:voreinstellungen:0 | fensterrand | [!! Starten und Bearbeiten ········!!] | 1024x768@150 | [-19,175–143,185] Fenster [0,0–683,512] |
| dlg:voreinstellungen:0 | fensterrand | [!! Text ··!!] | 1024x768@150 | [-19,112–31,122] Fenster [0,0–683,512] |
| dlg:voreinstellungen:0 | fensterrand | [!! Voreinstellungen ·······!!] | 1024x768@150 | [-28,11–105,23] Fenster [0,0–683,512] |
| dlg:voreinstellungen:0 | fensterrand | [!! Zusatzmodule ·····!!] | 1024x768@150 | [-19,154–90,164] Fenster [0,0–683,512] |
| dlg:voreinstellungen:1 | fensterrand | [!! Allgemein ····!!] | 1024x768@150 | [-19,49–67,60] Fenster [0,0–683,512] |
| dlg:voreinstellungen:1 | fensterrand | [!! Bearbeiten ····!!] | 1024x768@150 | [-19,70–71,80] Fenster [0,0–683,512] |
| dlg:voreinstellungen:1 | fensterrand | [!! Hilfslinien und Raster ········!!] | 1024x768@150 | [-19,91–133,101] Fenster [0,0–683,512] |
| dlg:voreinstellungen:1 | fensterrand | [!! Photoshop-Import/Öffnen ··········!!] | 1024x768@150 | [-19,131–165,144] Fenster [0,0–683,512] |
| dlg:voreinstellungen:1 | fensterrand | [!! Starten und Bearbeiten ········!!] | 1024x768@150 | [-19,175–143,185] Fenster [0,0–683,512] |
| dlg:voreinstellungen:1 | fensterrand | [!! Text ··!!] | 1024x768@150 | [-19,112–31,122] Fenster [0,0–683,512] |
| dlg:voreinstellungen:1 | fensterrand | [!! Voreinstellungen ·······!!] | 1024x768@150 | [-28,11–105,23] Fenster [0,0–683,512] |
| dlg:voreinstellungen:1 | fensterrand | [!! Zusatzmodule ·····!!] | 1024x768@150 | [-19,154–90,164] Fenster [0,0–683,512] |
| dlg:voreinstellungen:2 | fensterrand | 5 | 1024x768@150 | [451,508–457,516] Fenster [0,0–683,512] |
| dlg:voreinstellungen:2 | fensterrand | [!! Allgemein ····!!] | 1024x768@150 | [-19,49–67,60] Fenster [0,0–683,512] |
| dlg:voreinstellungen:2 | fensterrand | [!! Bearbeiten ····!!] | 1024x768@150 | [-19,70–71,80] Fenster [0,0–683,512] |
| dlg:voreinstellungen:2 | fensterrand | [!! Hilfslinien und Raster ········!!] | 1024x768@150 | [-19,91–133,101] Fenster [0,0–683,512] |
| dlg:voreinstellungen:2 | fensterrand | [!! Photoshop-Import/Öffnen ··········!!] | 1024x768@150 | [-19,131–165,144] Fenster [0,0–683,512] |
| dlg:voreinstellungen:2 | fensterrand | [!! Starten und Bearbeiten ········!!] | 1024x768@150 | [-19,175–143,185] Fenster [0,0–683,512] |
| dlg:voreinstellungen:2 | fensterrand | [!! Text ··!!] | 1024x768@150 | [-19,112–31,122] Fenster [0,0–683,512] |
| dlg:voreinstellungen:2 | fensterrand | [!! Zusatzmodule ·····!!] | 1024x768@150 | [-19,154–90,164] Fenster [0,0–683,512] |
| dlg:voreinstellungen:3 | fensterrand | [!! Allgemein ····!!] | 1024x768@150 | [-19,49–67,60] Fenster [0,0–683,512] |
| dlg:voreinstellungen:3 | fensterrand | [!! Bearbeiten ····!!] | 1024x768@150 | [-19,70–71,80] Fenster [0,0–683,512] |
| dlg:voreinstellungen:3 | fensterrand | [!! Hilfslinien und Raster ········!!] | 1024x768@150 | [-19,91–133,101] Fenster [0,0–683,512] |
| dlg:voreinstellungen:3 | fensterrand | [!! Photoshop-Import/Öffnen ··········!!] | 1024x768@150 | [-19,131–165,144] Fenster [0,0–683,512] |
| dlg:voreinstellungen:3 | fensterrand | [!! Starten und Bearbeiten ········!!] | 1024x768@150 | [-19,175–143,185] Fenster [0,0–683,512] |
| dlg:voreinstellungen:3 | fensterrand | [!! Text ··!!] | 1024x768@150 | [-19,112–31,122] Fenster [0,0–683,512] |
| dlg:voreinstellungen:3 | fensterrand | [!! Voreinstellungen ·······!!] | 1024x768@150 | [-28,11–105,23] Fenster [0,0–683,512] |
| dlg:voreinstellungen:3 | fensterrand | [!! Zusatzmodule ·····!!] | 1024x768@150 | [-19,154–90,164] Fenster [0,0–683,512] |
| dlg:voreinstellungen:4 | fensterrand | [!! Allgemein ····!!] | 1024x768@150 | [-19,49–67,60] Fenster [0,0–683,512] |
| dlg:voreinstellungen:4 | fensterrand | [!! Bearbeiten ····!!] | 1024x768@150 | [-19,70–71,80] Fenster [0,0–683,512] |
| dlg:voreinstellungen:4 | fensterrand | [!! Hilfslinien und Raster ········!!] | 1024x768@150 | [-19,91–133,101] Fenster [0,0–683,512] |
| dlg:voreinstellungen:4 | fensterrand | [!! Photoshop-Import/Öffnen ··········!!] | 1024x768@150 | [-19,131–165,144] Fenster [0,0–683,512] |
| dlg:voreinstellungen:4 | fensterrand | [!! Starten und Bearbeiten ········!!] | 1024x768@150 | [-19,175–143,185] Fenster [0,0–683,512] |
| dlg:voreinstellungen:4 | fensterrand | [!! Text ··!!] | 1024x768@150 | [-19,112–31,122] Fenster [0,0–683,512] |
| dlg:voreinstellungen:4 | fensterrand | [!! Voreinstellungen ·······!!] | 1024x768@150 | [-28,11–105,23] Fenster [0,0–683,512] |
| dlg:voreinstellungen:4 | fensterrand | [!! Zusatzmodule ·····!!] | 1024x768@150 | [-19,154–90,164] Fenster [0,0–683,512] |
| dlg:voreinstellungen:5 | fensterrand | [!! Allgemein ····!!] | 1024x768@150 | [-19,49–67,60] Fenster [0,0–683,512] |
| dlg:voreinstellungen:5 | fensterrand | [!! Bearbeiten ····!!] | 1024x768@150 | [-19,70–71,80] Fenster [0,0–683,512] |
| dlg:voreinstellungen:5 | fensterrand | [!! Hilfslinien und Raster ········!!] | 1024x768@150 | [-19,91–133,101] Fenster [0,0–683,512] |
| dlg:voreinstellungen:5 | fensterrand | [!! Hinweis: Firepixel bettet keine Texturen oder Muster des Originals ein; Ordner werden nur zur Laufzeit gelesen. ·······································!!] | 1024x768@150 | [189,300–693,326] Fenster [0,0–683,512] |
| dlg:voreinstellungen:5 | fensterrand | [!! Photoshop-Import/Öffnen ··········!!] | 1024x768@150 | [-19,131–165,144] Fenster [0,0–683,512] |
| dlg:voreinstellungen:5 | fensterrand | [!! Starten und Bearbeiten ········!!] | 1024x768@150 | [-19,175–143,185] Fenster [0,0–683,512] |
| dlg:voreinstellungen:5 | fensterrand | [!! Text ··!!] | 1024x768@150 | [-19,112–31,122] Fenster [0,0–683,512] |
| dlg:voreinstellungen:5 | fensterrand | [!! Voreinstellungen ·······!!] | 1024x768@150 | [-28,11–105,23] Fenster [0,0–683,512] |
| dlg:voreinstellungen:5 | fensterrand | [!! Zusatzmodule ·····!!] | 1024x768@150 | [-19,154–90,164] Fenster [0,0–683,512] |
| dlg:voreinstellungen:6 | fensterrand | [!! Allgemein ····!!] | 1024x768@150 | [-19,49–67,60] Fenster [0,0–683,512] |
| dlg:voreinstellungen:6 | fensterrand | [!! Bearbeiten ····!!] | 1024x768@150 | [-19,70–71,80] Fenster [0,0–683,512] |
| dlg:voreinstellungen:6 | fensterrand | [!! Hilfslinien und Raster ········!!] | 1024x768@150 | [-19,91–133,101] Fenster [0,0–683,512] |
| dlg:voreinstellungen:6 | fensterrand | [!! Photoshop-Import/Öffnen ··········!!] | 1024x768@150 | [-19,131–165,144] Fenster [0,0–683,512] |
| dlg:voreinstellungen:6 | fensterrand | [!! Starten und Bearbeiten ········!!] | 1024x768@150 | [-19,175–143,185] Fenster [0,0–683,512] |
| dlg:voreinstellungen:6 | fensterrand | [!! Text ··!!] | 1024x768@150 | [-19,112–31,122] Fenster [0,0–683,512] |
| dlg:voreinstellungen:6 | fensterrand | [!! Voreinstellungen ·······!!] | 1024x768@150 | [-28,11–105,23] Fenster [0,0–683,512] |
| dlg:voreinstellungen:6 | fensterrand | [!! Zusatzmodule ·····!!] | 1024x768@150 | [-19,154–90,164] Fenster [0,0–683,512] |

**Deutsch geändert gegenüber dem Stand vor Phase 6 (101 Bilder), je Zeile zusammengefasst (− vorher, + jetzt; Text, x,y,Breite,Höhe):**

| Zeile | Bilder |
|---|---|
| vorher ["99 Farben\t1536,334,47,8"] → jetzt ["108 Farben\t1531,334,52,8"] | 51 dlg:arbeitsbereich_speichern, dlg:export, dlg:form:10397 … |
| vorher ["1\t778,488,4,9", "99 Farben\t1536,334,47,8"] → jetzt ["10\t778,488,12,9", "108 Farben\t1531,334,52,8"] | 5 dlg:form:19105, dlg:form:19106, dlg:form:19107 … |
| vorher [".fw.png anhängen\t696,548,90,12", "20\t911,310,13,9", "99 Farben\t1536,334,47,8", "Abbrechen\t1038,735,55,9", "Alle Warnmeldungen zurücksetzen\t700,634 | 2 dlg:voreinstellungen, dlg:voreinstellungen:0 |
| vorher ["108 Farben\t1531,334,52,8", "30\t777,444,13,9", "Abbrechen\t888,573,55,9", "OK\t824,573,16,9", "Radiales Weichzeichnen\t641,408,125,9", "Stärke:\t644,4 | 2 dlg:form:19159, fx:19159 |
| vorher ["109 Farben\t1531,334,52,8", "90\t777,444,13,9", "Abbrechen\t888,573,55,9", "Bewegungsunschärfe\t641,408,111,12", "OK\t824,573,16,9", "Vorschau\t662,572 | 2 dlg:form:6525, fx:6525 |
| vorher ["201 Farben\t1530,334,53,8"] → jetzt ["210 Farben\t1530,334,53,8"] | 2 dlg:form:5971, fx:5971 |
| vorher ["221 Farben\t1530,334,53,8", "50\t777,454,13,9", "Abbrechen\t888,563,55,9", "Farbe\t663,500,27,9", "OK\t824,563,16,9", "Rauschen hinzufügen\t641,418,112 | 2 dlg:form:19157, fx:19157 |
| vorher ["30\t777,444,13,9", "99 Farben\t1536,334,47,8", "Abbrechen\t888,573,55,9", "OK\t824,573,16,9", "Stärke:\t644,444,34,9", "Vorschau\t662,572,46,9"] → jetz | 2 dlg:form:19160, fx:19160 |
| vorher ["99 Farben\t1536,334,47,8", "In Alpha konvertieren\t41,800,108,12", "Kanten suchen\t41,822,73,9"] → jetzt ["108 Farben\t1531,334,52,8", "In Alphakanal k | 2 fx:menue:7, fx:menue:8 |
| vorher ["%\t820,466,10,9", "0\t757,604,6,9", "100\t758,466,18,9", "100\t758,512,18,9", "100\t758,558,18,9", "45\t757,420,12,9"] → jetzt ["%\t835,476,10,9", "0\t | 1 dlg:form:19146 |
| vorher ["+\t839,660,6,5", "99 Farben\t1536,334,47,8", "Abbrechen\t1038,765,55,9", "Aktueller Satz:\t494,254,72,9", "Alt+Umschalt+F10\t915,2654,95,9", "Alt+Umsch | 1 dlg:tastenkuerzel |
| vorher ["0\t911,334,6,9", "1,2\t912,310,14,11", "6\t911,430,6,9", "99 Farben\t1536,334,47,8", "Abbrechen\t1038,735,55,9", "Allgemein\t499,283,51,12"] → jetzt [" | 1 dlg:voreinstellungen:3 |
| vorher ["100\t309,921,18,9", "202\t110,948,19,9", "29\t110,970,13,9", "306\t162,970,19,9", "99 Farben\t1536,334,47,8", "Arial\t211,897,22,9"] → jetzt ["108 Farb | 1 dlg:form:6073 |
| vorher ["119 Farben\t1531,334,52,8"] → jetzt ["128 Farben\t1531,334,52,8"] | 1 fx:fx-glow |
| vorher ["123 Farben\t1531,334,52,8"] → jetzt ["131 Farben\t1531,334,52,8"] | 1 fx:fx-dropShadow |
| vorher ["130 Farben\t1531,334,52,8"] → jetzt ["139 Farben\t1531,334,52,8"] | 1 fx:fx-innerGlow |
| vorher ["132 Farben\t1531,334,52,8"] → jetzt ["141 Farben\t1531,334,52,8"] | 1 fx:fx-innerShadow |
| vorher ["156 MB\t832,322,38,9", "178 MB\t832,394,38,9", "184 MB\t832,358,38,9", "444 MB\t831,430,39,9", "444 MB\t831,502,39,9", "6 von 6 Modellen vorhanden · 1, | 1 dlg:ki_verwalten |
| vorher ["1\t738,476,4,9", "99 Farben\t1536,334,47,8", "Abbrechen\t928,541,55,9", "Am Anfang\t751,502,57,12", "Am Ende\t1127,502,46,9", "Anzahl:\t604,476,37,9"]  | 1 dlg:form:5990 |
| vorher ["1\t748,476,4,9", "99 Farben\t1536,334,47,8", "Abgeschrägt\t761,502,64,12", "Eckig\t921,502,25,12", "Rund\t860,502,26,9"] → jetzt ["10\t748,476,12,9", " | 1 dlg:form:6007 |
| vorher ["248 Farben\t1530,334,53,8"] → jetzt ["177 Farben\t1531,334,52,8"] | 1 fx:fx-bevel |
| vorher ["36\t911,598,13,9", "36\t911,622,13,9", "5\t911,742,6,9", "9-Slice-Hilfslinien:\t678,478,90,9", "99 Farben\t1536,334,47,8", "Abbrechen\t1038,735,55,9"]  | 1 dlg:voreinstellungen:2 |
| vorher ["5\t911,574,6,9", "99 Farben\t1536,334,47,8", "Abbrechen\t1038,735,55,9", "Allgemein\t499,283,51,12", "Bearbeiten\t500,304,53,9", "Bearbeiten\t679,280,6 | 1 dlg:voreinstellungen:1 |
| vorher ["7\t757,488,6,9", "99 Farben\t1536,334,47,8", "Status 1\t757,462,39,9"] → jetzt ["108 Farben\t1531,334,52,8", "7\t780,488,6,9", "Status 1\t780,462,39,9" | 1 dlg:form:states-props |
| vorher ["99 Farben\t1536,334,47,8", "Abbrechen\t1038,735,55,9", "Allgemein\t499,283,51,12", "Bearbeiten\t500,304,53,9", "Beim Starten fragen\t913,310,99,12", "B | 1 dlg:voreinstellungen:6 |
| vorher ["99 Farben\t1536,334,47,8", "Abbrechen\t1038,735,55,9", "Allgemein\t499,283,51,12", "Bearbeiten\t500,304,53,9", "Bildebenen bearbeitbar\t697,356,118,9", | 1 dlg:voreinstellungen:4 |
| vorher ["99 Farben\t1536,334,47,8", "Abbrechen\t1038,735,55,9", "Allgemein\t499,283,51,12", "Bearbeiten\t500,304,53,9", "Hilfslinien und Raster\t500,325,107,9", | 1 dlg:voreinstellungen:5 |
| vorher ["99 Farben\t1536,334,47,8", "Abbrechen\t898,659,55,9", "Bikubisch\t757,593,47,9", "OK\t836,659,16,9"] → jetzt ["108 Farben\t1531,334,52,8", "Abbrechen\t | 1 dlg:bildgroesse |
| vorher ["99 Farben\t1536,334,47,8", "Abbrechen\t928,528,55,9", "Groß\t935,488,25,9", "Keine\t752,488,27,9", "Klein\t814,488,24,9", "Miniaturgröße:\t605,488,74,1 | 1 dlg:form:6272 |
| vorher ["99 Farben\t1536,334,47,8", "Abbrechen\t928,554,55,9", "OK\t864,554,16,9"] → jetzt ["108 Farben\t1531,334,52,8", "Abbrechen\t929,554,55,9", "OK\t865,554 | 1 dlg:form:5991 |
| vorher ["99 Farben\t1536,334,47,8", "Abgeschrägt\t761,476,64,12", "Eckig\t832,528,25,12", "Eckig\t921,476,25,12", "Rund\t860,476,26,9", "Rund\t893,528,26,9"] →  | 1 dlg:form:6008 |
| vorher ["99 Farben\t1536,334,47,8", "Auto\t934,342,25,9", "Zurücksetzen\t912,370,67,9"] → jetzt ["108 Farben\t1531,334,52,8", "Auto\t932,342,25,9", "Zurücksetze | 1 dlg:kurven |
| vorher ["99 Farben\t1536,334,47,8", "Das Löschen des Verlaufs kann nicht rückgängig gemacht werden. Fortfahren?\t617,473,333,24", "Ja\t857,547,10,9", "Nein\t935 | 1 dlg:rueckfrage:panel.history.clear |
| vorher ["99 Farben\t1536,334,47,8", "Das Symbol „Grafik“ und alle Instanzen löschen?\t617,473,243,12", "Ja\t857,547,10,9", "Nein\t935,547,23,9", "Symbol löschen | 1 dlg:rueckfrage:panel.library.delete |
| vorher ["99 Farben\t1536,334,47,8", "Datei-Info\t621,436,52,9", "OK\t928,547,16,9", "Titel: Unbenannt-2.png⏎Pfad: (nicht gespeichert)⏎Seiten: 2⏎Symbole: 3⏎Forma | 1 dlg:meldung:file.fileInfo |
| vorher ["99 Farben\t1536,334,47,8", "Firepixel\t611,436,42,12", "Ja\t857,547,10,9", "Letzte gespeicherte Version von \"gespeichert.fw.png\" wiederherstellen?\t6 | 1 dlg:rueckfrage:file.revert |
| vorher ["99 Farben\t1536,334,47,8", "Firepixel\t695,462,78,20", "Grafikprogramm für Bildschirm- und Webgrafik, in Rust.\t694,506,282,12", "OK\t928,572,16,9", "V | 1 dlg:ueber |
| vorher ["99 Farben\t1536,334,47,8", "GIF   8,37 K   2 s bei 56 kbit/s   99 Farben\t662,266,206,11"] → jetzt ["108 Farben\t1531,334,52,8", "GIF   8,35 K   2 s be | 1 dlg:bildvorschau |
| vorher ["99 Farben\t1536,334,47,8", "Testbefehl\t609,382,51,9", "Testbefehl2\t609,400,58,9"] → jetzt ["108 Farben\t1531,334,52,8", "Testbefehl\t629,384,51,9", " | 1 dlg:befehle_verwalten |
| vorher ["99 Farben\t1536,334,47,8", "Änderungen an Unbenannt-2 speichern?\t650,475,206,14"] → jetzt ["108 Farben\t1531,334,52,8", "Änderungen an Unbenannt-2 spe | 1 dlg:ungespeichert |


### Kontextmenüs

**Nicht herstellbar de:** ctx:seite (1024x768@150): Zeile nicht gefunden

**Nicht herstellbar en:** ctx:seite (1024x768@150): Zeile nicht gefunden

**Nicht herstellbar qps:** ctx:seite (1024x768@150): Zeile nicht gefunden

**Unmarkierte Texte qps (10):**

| Text | Bilder |
|---|---|
| 01  Seite 1 | ctx:seite |
| AV | ctx:leinwand:text |
| F8 | ctx:leinwand:objekt, ctx:leinwand:text |
| I | ctx:leinwand:text |
| Myriad Pro | ctx:leinwand:text |
| Seite 1 | ctx:ebene, ctx:leinwand:nichts, ctx:leinwand:objekt, ctx:leinwand:pipette … (+9) |
| T | ctx:leinwand:text |
| U | ctx:leinwand:text |
| Unbenannt-2 | ctx:ebene, ctx:leinwand:nichts, ctx:leinwand:zoom, ctx:leinwand:zuschneiden … (+3) |
| a | ctx:leinwand:text |

**Überläufe en, neu gegenüber Deutsch (5):**

| Bild | Art | Text | Größen | Lage |
|---|---|---|---|---|
| ctx:ebene | in_element | Add Mask | 1024x768@150 | Text [494,451–547,460] Element [510,416–683,512] |
| ctx:ebene | in_element | Disable Mask | 1024x768@150 | Text [494,495–564,504] Element [510,416–683,512] |
| ctx:ebene | in_element | Edit Mask | 1024x768@150 | Text [494,473–545,482] Element [510,416–683,512] |
| ctx:leinwand:objekt | abgeschnitten | Texture: | 1024x768@150 | [390,459–429,467] sichtbar [390,459–401,467] |
| ctx:leinwand:text | abgeschnitten | Regular | 1024x768@150 | [389,410–427,421] sichtbar [389,410–401,421] |

**Deutsch geändert gegenüber dem Stand vor Phase 6 (13 Bilder), je Zeile zusammengefasst (− vorher, + jetzt; Text, x,y,Breite,Höhe):**

| Zeile | Bilder |
|---|---|
| vorher ["99 Farben\t1536,334,47,8"] → jetzt ["108 Farben\t1531,334,52,8"] | 10 ctx:ebene, ctx:leinwand:nichts, ctx:leinwand:objekt … |
| vorher ["99 Farben\t1536,334,47,8", "Alphakanal\t658,946,56,12", "Graustufen\t658,966,56,9", "Maske:\t586,944,34,9"] → jetzt ["108 Farben\t1531,334,52,8"] | 2 ctx:leinwand:pipette, ctx:leinwand:pixel |
| vorher ["100\t309,921,18,9", "202\t110,948,19,9", "29\t110,970,13,9", "306\t162,970,19,9", "99 Farben\t1536,334,47,8", "Arial\t211,897,22,9"] → jetzt ["108 Farb | 1 ctx:leinwand:text |


### Statuszeile

**Unmarkierte Texte qps (8):**

| Text | Bilder |
|---|---|
| AV | status:text |
| I | status:text |
| Myriad Pro | status:text |
| Seite 1 | status:auswahl, status:einfuegen, status:gruppieren, status:stift … (+3) |
| T | status:text |
| U | status:text |
| Unbenannt-2 | status:auswahl, status:zeiger |
| a | status:text |

**Überläufe qps, neu gegenüber Deutsch (1):**

| Bild | Art | Text | Größen | Lage |
|---|---|---|---|---|
| status:zauberstab | abgeschnitten | [!! Hart ··!!] | 1024x768@150 | [374,410–425,420] sichtbar [374,410–401,420] |

**Deutsch geändert gegenüber dem Stand vor Phase 6 (7 Bilder), je Zeile zusammengefasst (− vorher, + jetzt; Text, x,y,Breite,Höhe):**

| Zeile | Bilder |
|---|---|
| vorher ["99 Farben\t1536,334,47,8"] → jetzt ["108 Farben\t1531,334,52,8"] | 4 status:auswahl, status:gruppieren, status:stift … |
| vorher ["94 Farben\t1536,334,47,8"] → jetzt ["103 Farben\t1531,334,52,8"] | 1 status:einfuegen |
| vorher ["99 Farben\t1536,334,47,8", "Alphakanal\t658,946,56,12", "Graustufen\t658,966,56,9", "Maske:\t586,944,34,9"] → jetzt ["108 Farben\t1531,334,52,8"] | 1 status:zauberstab |
| vorher ["99 Farben\t1536,334,47,8", "Arial\t211,897,22,9", "Regular\t389,897,38,12"] → jetzt ["108 Farben\t1531,334,52,8", "Bold\t389,897,22,9", "Myriad Pro\t21 | 1 status:text |


### Deutsch: Überläufe, die es vor Phase 6 nicht gab

**Menüs und Untermenüs** (9):

| Bild | Art | Text | Größen |
|---|---|---|---|
| menu:0 | abgeschnitten | Pinselstrich bearbeiten | 1024x768@150 |
| menu:0/18 | abgeschnitten | Pinselstrich bearbeiten | 1024x768@150 |
| menu:0/25 | abgeschnitten | Pinselstrich bearbeiten | 1024x768@150 |
| menu:0/4 | abgeschnitten | Pinselstrich bearbeiten | 1024x768@150 |
| menu:0/5 | abgeschnitten | Pinselstrich bearbeiten | 1024x768@150 |
| menu:3 | abgeschnitten | Pinselstrich bearbeiten | 1024x768@150 |
| menu:5 | abgeschnitten | Pinselstrich bearbeiten | 1024x768@150 |
| menu:6 | abgeschnitten | Pinselstrich bearbeiten | 1024x768@150 |
| menu:9 | abgeschnitten | Pinselstrich bearbeiten | 1024x768@150 |

**Inspektor (Elementarten, Werkzeuge)** (5):

| Bild | Art | Text | Größen |
|---|---|---|---|
| pi:Animationssymbol | abgeschnitten | 0 | 1024x768@150 |
| pi:Animationssymbol | abgeschnitten | 100 | 1024x768@150 |
| pi:Text | in_element | Autom. Unterschneidung | 1024x768@100 |
| pi:Text auf Pfad | in_element | Autom. Unterschneidung | 1024x768@100 |
| werkzeug:tool.smudge | abgeschnitten | Druck: | 1024x768@150 |

**Dialoge** (1):

| Bild | Art | Text | Größen |
|---|---|---|---|
| dlg:ki_verwalten | fensterrand | Zwischenspeicher leeren | 1024x768@150 |

**Kontextmenüs** (1):

| Bild | Art | Text | Größen |
|---|---|---|---|
| ctx:leinwand:text | in_element | Autom. Unterschneidung | 1024x768@100 |
<!-- AUTO:END -->

## Englisch fachlich (automatische Vorprüfung)

Automatisch: Glossarbegriffe, Ellipsen, Zugriffstasten, Platzhalter, Title/Sentence case, deutsche Reste (`i18n_qa_glossar.rs`). Die Treffer sind Hinweise. Was davon wirklich ein Fehler ist, steht als Befund in `docs\BEFUNDE.md`.

<!-- EN:START -->
Schlüssel de: 3145, en: 3145. Ellipsen in en: {"...": 146, "…": 12}.

| Regel | Anzahl |
|---|---|
| ellipse | 1 |
| glossar | 111 |
| sentence_case | 1 |
| title_case | 7 |
| zugriffstaste | 1 |


#### ellipse (1)

| Schlüssel | Datei | de | en | Hinweis |
|---|---|---|---|---|
| `cmd-filter-blur-motion` | menue.ftl | Bewegungsunschärfe… | Motion Blur... | Schreibweise de Some("…") / en Some("...") |

#### glossar (111)

| Schlüssel | Datei | de | en | Hinweis |
|---|---|---|---|---|
| `blend-stamp` | filter.ftl | Stempel | Stamp | „Stempel“ → „Rubber Stamp“ |
| `cmd-ai-manageModels` | menue.ftl | KI-Modelle verwalten... | Manage AI Models... | „Modelle verwalten“ → „Manage Models“ |
| `cmd-autoShape-insert` | menue.ftl | Automatische Form einfügen | Insert Auto Shape | „Einfügen“ → „Paste“ |
| `cmd-color-swap` | menue.ftl | Pinsel- und Füllungsfarben austauschen | Swap Stroke and Fill Colors | „Pinsel“ → „Brush“ |
| `cmd-file-sendToEMail-firepixelPng` | menue.ftl | PNG-Datei für Firepixel... | Firepixel PNG... | „Datei“ → „File“ |
| `cmd-file-sendToEMail-jpegCompressed` | menue.ftl | Komprimierte JPG-Datei... | JPEG Compressed... | „Datei“ → „File“ |
| `cmd-file-twainAcquire` | menue.ftl | Twain importieren | Twain Acquire | „Importieren“ → „Import“ |
| `cmd-filter-adjustColor-brightnessContrast` | menue.ftl | Helligkeit/Kontrast... | Brightness/Contrast... | „Helligkeit“ → „Luminosity“ |
| `cmd-filter-repeat` | menue.ftl | Filter &wiederholen | &Repeat Filter | „Wiederholen“ → „Redo“ |
| `cmd-history-replay` | menue.ftl | Schritte wiederholen | Replay Steps | „Wiederholen“ → „Redo“ |
| `cmd-layer-setShared` | menue.ftl | Ebene über Status teilen | Share Layer to States | „Teilen“ → „Break Apart“ |
| `cmd-modify-alterPath-featherFill` | menue.ftl | &Weiche-Kante-Füllung | &Feather Fill | „Kante“ → „Edge“ |
| `cmd-modify-arrange-sendToBack` | menue.ftl | In den &Hintergrund | Send to &Back | „Hintergrund“ → „Background“ |
| `cmd-modify-canvas-trimCanvas` | menue.ftl | Leinwand &zuschneiden | &Trim Canvas | „Zuschneiden“ → „Crop“/„Crop“ |
| `cmd-modify-combinePaths-split` | menue.ftl | &Teilen | &Split | „Teilen“ → „Break Apart“ |
| `cmd-modify-flattenLayers` | menue.ftl | Ebenen als Hinterg&rund | Flatten La&yers | „Hintergrund“ → „Background“ |
| `cmd-modify-flattenSelection` | menue.ftl | Aus&wahl auf Hintergrund reduzieren | Flatten Selecti&on | „Hintergrund“ → „Background“ |
| `cmd-modify-mergeDown` | menue.ftl | Mit &darunter liegender auf eine Ebene reduzieren | Merge &Down | „Ebene“ → „Layer“ |
| `cmd-modify-selectiveJpeg-restoreJpegMaskAsSelection` | menue.ftl | JPEG-Maske als Auswahl wiede&rherstellen | &Restore JPEG Mask as Selection | „Wiederherstellen“ → „Revert“ |
| `cmd-modify-snapToPixel` | menue.ftl | An Pixeln ausrichten | Snap to Pixel | „Ausrichten“ → „Align“ |
| `cmd-panel-effects-locatePlugins` | menue.ftl | Zusatzmodule suchen... | Locate Plug-ins... | „Suchen“ → „Find“ |
| `cmd-panel-history-replaySelected` | menue.ftl | Ausgewählte Schritte wiederholen | Replay Selected Steps | „Wiederholen“ → „Redo“ |
| `cmd-panel-layers-flattenSelection` | menue.ftl | Auswahl auf Hintergrund reduzieren | Flatten Selection | „Hintergrund“ → „Background“ |
| `cmd-panel-layers-mergeDown` | menue.ftl | Mit darunter liegender auf eine Ebene reduzieren | Merge Down | „Ebene“ → „Layer“ |
| `cmd-panel-states-disposalBackground` | menue.ftl | Hintergrund wiederherstellen | Restore to Background | „Wiederherstellen“ → „Revert“ |
| `cmd-panel-swatches-continuousTone` | menue.ftl | Kontinuierlicher Farbton | Continuous Tone | „Farbton“ → „Hue“ |
| `cmd-path-insertNode` | menue.ftl | Punkt einfügen | Insert Point | „Einfügen“ → „Paste“ |
| `cmd-pi-symbol-place` | menue.ftl | Symbol einfügen | Place Symbol | „Einfügen“ → „Paste“ |
| `cmd-select-feather` | menue.ftl | &Weiche Kante… | &Feather… | „Kante“ → „Edge“ |
| `cmd-select-inRect` | menue.ftl | Auswahlrechteck | Select in Rectangle | „Auswahlrechteck“ → „Marquee“ |
| `cmd-select-restoreBitmapSelection` | menue.ftl | Bitmap-Auswahl wiede&rherstellen... | &Restore Bitmap Selection... | „Wiederherstellen“ → „Revert“ |
| `cmd-text-attachInPath` | menue.ftl | In Pfad &einfügen | Attach &In Path | „Einfügen“ → „Paste“ |
| `cmd-text-reverseDirection` | menue.ftl | &Richtung umkehren | Re&verse Direction | „Umkehren“ → „Inverse“/„Invert“ |
| `cmd-view-grid-snapToGrid` | menue.ftl | Am Raster au&srichten | &Snap to Grid | „Ausrichten“ → „Align“ |
| `cmd-view-guides-snapToGuides` | menue.ftl | An Hilfslinien &ausrichten | &Snap to Guides | „Ausrichten“ → „Align“ |
| `cmd-view-smartGuides-snapToSmartGuides` | menue.ftl | An intelligenten Hilfslinien ausrichten | Snap to Smart Guides | „Ausrichten“ → „Align“ |
| `cmd-window-tile` | menue.ftl | Anordnen | Tile | „Anordnen“ → „Arrange“ |
| `cmd-window-toolbars-main` | menue.ftl | &Standard | &Main | „Standard“ → „Plain“ |
| `ctx-feather` | menue.ftl | Weiche Kante... | Feather... | „Kante“ → „Edge“ |
| `ctx-insertEmptyBitmap` | menue.ftl | Leere Bitmap einfügen | Insert Empty Bitmap | „Einfügen“ → „Paste“ |
| `ctx-insertHotspot` | menue.ftl | Hotspot einfügen | Insert Hotspot | „Einfügen“ → „Paste“ |
| `ctx-insertNewButton` | menue.ftl | Neue Schaltfläche einfügen... | Insert New Button... | „Einfügen“ → „Paste“ |
| `ctx-insertNewSymbol` | menue.ftl | Neues Symbol einfügen... | Insert New Symbol... | „Einfügen“ → „Paste“ |
| `ctx-insertPolygonSlice` | menue.ftl | Polygonsegment einfügen | Insert Polygon Slice | „Einfügen“ → „Paste“ |
| `ctx-insertRectangleSlice` | menue.ftl | Rechteckiges Segment einfügen | Insert Rectangle Slice | „Einfügen“ → „Paste“ |
| `ctx-insertSlice` | menue.ftl | Segment einfügen | Insert Slice | „Einfügen“ → „Paste“ |
| `ctx-restoreSelection` | menue.ftl | Auswahl wiederherstellen | Restore Selection | „Wiederherstellen“ → „Revert“ |
| `dlg-ai-default-folder` | dialoge.ftl | Standard | Default | „Standard“ → „Plain“ |
| `dlg-ai-folder-default` | dialoge.ftl | Standard-Modellordner. | Default model folder. | „Standard“ → „Plain“ |
| `dlg-ai-manage-title` | dialoge.ftl | KI-Modelle verwalten | Manage AI Models | „Modelle verwalten“ → „Manage Models“ |
| `dlg-brush-dash-on-1` | dialoge.ftl | Strich 1: | Dash 1: | „Strich“ → „Stroke“ |
| `dlg-brush-dash-on-2` | dialoge.ftl | Strich 2: | Dash 2: | „Strich“ → „Stroke“ |
| `dlg-brush-dash-on-3` | dialoge.ftl | Strich 3: | Dash 3: | „Strich“ → „Stroke“ |
| `dlg-curves-io` | dialoge.ftl | Eingabe: { $input }⏎Ausgabe: { $output } | Input: { $input }⏎Output: { $output } | „Eingabe“ → „Enter“ |
| `dlg-fx-knockout` | dialoge.ftl | Objekt aussparen | Knock out | „Objekt“ → „Object“ |
| `dlg-fx-slope-ruffle` | dialoge.ftl | Kräuseln | Ruffle | „Kräuseln“ → „Ripples“ |
| `dlg-lbl-border` | dialoge.ftl | Rand: | Border: | „Rand“ → „Edge“ |
| `dlg-lbl-brightness` | dialoge.ftl | Helligkeit: | Brightness: | „Helligkeit“ → „Luminosity“ |
| `dlg-lbl-insert` | dialoge.ftl | Einfügen: | Insert: | „Einfügen“ → „Paste“ |
| `dlg-lbl-snap-grid` | dialoge.ftl | Am Raster ausrichten | Snap to grid | „Ausrichten“ → „Align“ |
| `dlg-lbl-snap-guides` | dialoge.ftl | An Hilfslinien ausrichten | Snap to guides | „Ausrichten“ → „Align“ |
| `dlg-md-margins` | dialoge.ftl | Ränder: | Margins: | „Ränder“ → „Edges“ |
| `dlg-md-pick-html` | dialoge.ftl | HTML-Datei auswählen | Choose HTML File | „Auswählen“ → „Select“ |
| `dlg-md-printer` | dialoge.ftl | Drucker (leer = Standard): | Printer (empty = default): | „Standard“ → „Plain“ |
| `dlg-pmenu-locate-plugins` | dialoge.ftl | Zusatzmodule suchen | Locate Plug-ins | „Suchen“ → „Find“ |
| `dlg-pref-cat-type` | dialoge.ftl | Text | Type | „Text“ → „Text“/„Text“ |
| `dlg-pref-defaultleading` | dialoge.ftl | Standard-Zeilenabstand (Faktor): | Default leading (factor): | „Standard“ → „Plain“ |
| `dlg-pref-pssharedlayers` | dialoge.ftl | Ebenen über Status teilen | Share layers across states | „Teilen“ → „Break Apart“ |
| `dlg-pref-snaptogrids` | dialoge.ftl | Am Raster ausrichten | Snap to grid | „Ausrichten“ → „Align“ |
| `dlg-pref-snaptoguides` | dialoge.ftl | Ausrichten | Snap | „Ausrichten“ → „Align“ |
| `dlg-pref-snaptosmartguides` | dialoge.ftl | Ausrichten | Snap | „Ausrichten“ → „Align“ |
| `dlg-sc-search` | dialoge.ftl | Suchen | Search | „Suchen“ → „Find“ |
| `dlg-sc-shortcut` | dialoge.ftl | Tastenkürzel: | Shortcut: | „Tastenkürzel“ → „Keyboard Shortcuts“ |
| `dlg-title-brightness-contrast` | dialoge.ftl | Helligkeit/Kontrast | Brightness/Contrast | „Helligkeit“ → „Luminosity“ |
| `dlg-title-restore-selection` | dialoge.ftl | Auswahl wiederherstellen | Restore Selection | „Wiederherstellen“ → „Revert“ |
| `err-import-plane-short` | meldungen.ftl | Ebene zu kurz | plane too short | „Ebene“ → „Layer“ |
| `err-nf-segment` | meldungen.ftl | Segment | Segment | „Segment“ → „Slice“/„Slice“ |
| `err-script-no-layer` | skript.ftl | keine Ebene zum Einfügen (Ebene gesperrt oder Webebene aktiv?) | No layer to insert into (layer locked or Web Layer active?) | „Einfügen“ → „Paste“ |
| `filter-adjustColor-brightnessContrast` | filter.ftl | Helligkeit/Kontrast | Brightness/Contrast | „Helligkeit“ → „Luminosity“ |
| `filter-param-brightness` | filter.ftl | Helligkeit | Brightness | „Helligkeit“ → „Luminosity“ |
| `filter-param-lightness` | filter.ftl | Helligkeit | Lightness | „Helligkeit“ → „Luminosity“ |
| `menu-edit-insert` | menue.ftl | Objekt ein&fügen | &Insert | „Einfügen“ → „Paste“ |
| `menu-edit-insert` | menue.ftl | Objekt ein&fügen | &Insert | „Objekt“ → „Object“ |
| `menu-file-scan-twainAcquire` | menue.ftl | Twain &importieren... | Twain &Acquire... | „Importieren“ → „Import“ |
| `msg-step-repeat-filter` | meldungen.ftl | Filter wiederholen | Repeat Filter | „Wiederholen“ → „Redo“ |
| `panel-asp-p-sides` | bedienfelder.ftl | Seiten | Sides | „Seiten“ → „Pages“ |
| `panel-autoshapes-insert` | bedienfelder.ftl | In die Leinwandmitte einfügen | Insert at canvas center | „Einfügen“ → „Paste“ |
| `panel-beh-nav-restore` | bedienfelder.ftl | Navigationsleiste wiederherstellen | Nav Bar Restore | „Wiederherstellen“ → „Revert“ |
| `panel-beh-swap-restore` | bedienfelder.ftl | Bildaustausch wiederherstellen | Swap Image Restore | „Wiederherstellen“ → „Revert“ |
| `panel-commonlib-insert` | bedienfelder.ftl | In das Dokument einfügen | Insert into document | „Einfügen“ → „Paste“ |
| `panel-doclib-place` | bedienfelder.ftl | In Leinwandmitte einfügen | Insert at canvas center | „Einfügen“ → „Paste“ |
| `panel-find-scope-page` | bedienfelder.ftl | Seite suchen | Search Page | „Suchen“ → „Find“ |
| `panel-find-scope-selection` | bedienfelder.ftl | Auswahl suchen | Search Selection | „Suchen“ → „Find“ |
| `panel-history-replay` | bedienfelder.ftl | Wiederholen | Replay | „Wiederholen“ → „Redo“ |
| `panel-history-replay-label` | bedienfelder.ftl | Schritte wiederholen | Replay Steps | „Wiederholen“ → „Redo“ |
| `panel-imgedit-brightness` | bedienfelder.ftl | Helligkeit/Kontrast... | Brightness/Contrast... | „Helligkeit“ → „Luminosity“ |
| `panel-path-snap-pixel` | bedienfelder.ftl | An Pixeln ausrichten | Snap to Pixel | „Ausrichten“ → „Align“ |
| `panel-path-split` | bedienfelder.ftl | Teilen | Split | „Teilen“ → „Break Apart“ |
| `panel-swatches-set-continuous` | bedienfelder.ftl | Kontinuierlicher Farbton | Continuous Tone | „Farbton“ → „Hue“ |
| `pi-brush-cat-crayon` | inspektor.ftl | Buntstift | Crayon | „Buntstift“ → „Pencil“ |
| `pi-brush-dashed-basic-dash` | inspektor.ftl | Einfach gestrichelt | Basic Dash | „Gestrichelt“ → „Dashed“ |
| `pi-brush-dashed-double-dash` | inspektor.ftl | Doppelt gestrichelt | Double Dash | „Gestrichelt“ → „Dashed“ |
| `pi-edge-feather` | inspektor.ftl | Weiche Kante | Feather | „Kante“ → „Edge“ |
| `pi-fill-solid` | inspektor.ftl | Einfarbige Füllung | Solid | „Füllung“ → „Fill“ |
| `pi-grad-reverse` | inspektor.ftl | Umkehren | Reverse | „Umkehren“ → „Inverse“/„Invert“ |
| `pi-poly-sides` | inspektor.ftl | Seiten: | Sides: | „Seiten“ → „Pages“ |
| `pi-stroke-center` | inspektor.ftl | Pinselstrich mittig ausrichten | Center stroke on path | „Ausrichten“ → „Align“ |
| `ui-tab-merge-all` | oberflaeche.ftl | Alles hier zusammenführen | Consolidate All to Here | „Zusammenführen“ → „Merge“ |
| `ui-window-restore` | oberflaeche.ftl | Wiederherstellen | Restore Down | „Wiederherstellen“ → „Revert“ |
| `ui-workspace-switcher` | oberflaeche.ftl | Einen Arbeitsbereich auswählen ({ $name }) | Choose a workspace ({ $name }) | „Auswählen“ → „Select“ |
| `welcome-help-models` | willkommen.ftl | KI-Modelle verwalten | Manage AI Models | „Modelle verwalten“ → „Manage Models“ |

#### sentence_case (1)

| Schlüssel | Datei | de | en | Hinweis |
|---|---|---|---|---|
| `cmd-bitmap-retouch.description` | menue.ftl | Pinselbasierte Bitmapwerkzeuge: Weichzeichner, Scharfzeichner, Abwedler, Nachbelichter, Verwischen, Stempel, Farbe erset | Brush-based bitmap tools: Blur, Sharpen, Dodge, Burn, Smudge, Rubber Stamp, Replace Color, Red Eye Removal | Hinweis in Title Case |

#### title_case (7)

| Schlüssel | Datei | de | en | Hinweis |
|---|---|---|---|---|
| `cmd-panel-pages-export` | menue.ftl | Ausgewählte Seite(n) exportieren... | Export Selected Page(s)... | klein: s |
| `cmd-window-jquery` | menue.ftl | jQuery | jQuery | klein: jQuery |
| `ctx-msg-noColor` | menue.ftl | Keine Farbe an dieser Stelle. | No color at this point. | klein: color, this |
| `dlg-pm-title-edit` | dialoge.ftl | Pop-up-Menü-Editor (bearbeiten) | Pop-up Menu Editor (edit) | klein: edit |
| `menu-file-previewAllIn` | menue.ftl | Vorschau aller Seiten in { $browser } anzeigen | Preview All Pages in { $browser } | klein: in |
| `menu-file-previewIn` | menue.ftl | Vorschau in { $browser } | Preview in { $browser } | klein: in |
| `menu-text-font-font` | menue.ftl | Platzhalter - nicht entfernen | Placeholder - do not remove | klein: do, not, remove |

#### zugriffstaste (1)

| Schlüssel | Datei | de | en | Hinweis |
|---|---|---|---|---|
| `panel-opt-palette-bw` | bedienfelder.ftl | Schwarzweiß | Black & White | „&“ nur in einer Sprache |
<!-- EN:END -->

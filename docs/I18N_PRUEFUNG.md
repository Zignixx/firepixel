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
(noch kein Lauf)
<!-- AUTO:END -->

## Englisch fachlich (automatische Vorprüfung)

Automatisch: Glossarbegriffe, Ellipsen, Zugriffstasten, Platzhalter, Title/Sentence case, deutsche Reste (`i18n_qa_glossar.rs`). Die Treffer sind Hinweise. Was davon wirklich ein Fehler ist, steht als Befund in `docs\BEFUNDE.md`.

<!-- EN:START -->
<!-- EN:END -->

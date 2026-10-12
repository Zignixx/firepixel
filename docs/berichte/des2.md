# DES-2: Startbildschirm schöner (Nutzerbefund N11, Teil Startbildschirm)

Slot s11, Stand 13.10.2026.

## Umgesetzt

Commit: siehe `git log --grep "DES-2:"` (ein Commit, per cherry-pick nach `main`).

`crates\fp-ui\src\welcome.rs` neu gestaltet, Inhalt gleich:
- **Kopfzeile**: abgerundetes Banner mit App-Logo, „Firepixel“ und Leitsatz. Das Pixelfeld ist wie bisher; im OLED-Design leuchtet der Rand.
- **Drei Karten in zwei Spalten** (ab 600 px Inhaltsbreite, darunter einspaltig in der Reihenfolge Neu, Zuletzt, Hilfe; bei wenig Höhe Bildlauf):
  - **Neu erstellen**: zwei große Einträge mit Symbolchip, fettem Titel und gedämpfter Erklärung (Firepixel-Dokument, Aus Vorlage...).
  - **Zuletzt geöffnete Dateien**:
    - bis 8 Zeilen à 44 px mit **kleiner Vorschau** (36 px). Die Vorschau gibt es für PNG/FW-PNG, JPEG, BMP und QOI, mit Schachbrett hinter Transparenz; andere Dateien zeigen das Symbol der Dateiart.
    - Name, darunter der **Ordner gedämpft** (in der Mitte gekürzt). Fehlende Dateien sind gedämpft, der Tooltip zeigt den vollen Pfad.
    - Leerer Zustand bleibt. „Öffnen...“ sitzt unten an der Karte.
  - **Hilfe und Lernen**: vier »-Links (Kurzanleitung, Tastenkürzel, KI-Modelle, Automatisierung), darunter der Abschnitt „Erweitern“ (Gespeicherte Befehle, Erweiterungen; nur wenn der Befehl registriert ist).
- **Fußzeile**: „Nicht mehr anzeigen“ links, Version rechts.
- **Zustände**:
  - Hover getönt in der Leitfarbe der Karte, Handcursor.
  - Fokusrahmen in `focus` für jeden Eintrag.
  - **Tastatur**: Tab/Umschalt+Tab, Pfeiltasten bewegen den Fokus zwischen den Einträgen (auch zwischen den Spalten), Eingabe/Leertaste lösen aus.
- **Designs**:
  - Normal: helle Karten mit weichem Schatten auf der grauen Fläche.
  - Dunkel: Karten heller als die Leinwand, sonst verschwinden sie.
  - **OLED**: schwarz, jede Karte mit eigenem Leuchtrand und Titel in einer Gruppenfarbe (grün, cyan, orange), farbige Symbole.
- Vorschaubilder kommen aus der neuen Datei `crates\fp-ui\src\welcome\thumbs.rs`, einem Untermodul von `welcome.rs`; `lib.rs` ist nicht berührt.
  - Gelesen wird im Hintergrund, ein Thread je Datei, damit der erste Frame nicht hängt.
  - Zwischenspeicher je Pfad und Änderungszeit; Dateien über 48 MB werden nicht gelesen.
- Neu und öffentlich: `welcome::last_triggered(ctx)`, der zuletzt ausgelöste Eintrag (Befehl bzw. `open:<pfad>`) für Tests und Brücke, und `welcome::thumbnails_pending(ctx)`.
- `paint_banner` ist unverändert, „Über Firepixel“ nutzt es weiter.
- Neue Bausteine stehen nur in eigenen Dateien; `ui_kit.rs` ist nicht geändert.
- Texte: neuer Schlüssel `welcome-help-title` in `locales\de\willkommen.ftl` und `locales\en\willkommen.ftl` („Hilfe und Lernen“ / „Help and Learning“). Die übrigen Texte sind unverändert, die Erklärsätze der großen Einträge sind die vorhandenen `.tooltip`-Texte.

## Prüfungen (alle grün)

- `cargo test -p fp-ui --test all welcome_des2::`: 6 Tests, 1 ignoriert für die Bilder. Geprüft wird:
  - In 3 Designs × de/en und 5 Größen (1280 × 860, 1024 × 768, 1366 × 768 bei 150 %, 700 × 700, 420 × 900) sind alle Karten und Einträge da, nichts ist abgeschnitten oder überlappt, es stehen keine Rohschlüssel im Bild.
  - Jeder Eintrag ist per Mausklick auslösbar (Neu, Vorlage, Öffnen, 4 Hilfe-Links, Erweitern, zuletzt geöffnete Datei).
  - „Neu“ wirkt wirklich.
  - Tastatur: Tab + Eingabe, ↓ + Leertaste, → in die Dateiliste.
  - Vorschau bei echtem PNG, Symbol bei kaputter Datei, Ordnerzeile unter dem Namen.
  - Leerer Zustand und lange Namen.
- `design_willkommen::` (alte Tests von DESIGN, inkl. qps, 3840 × 2160 und 420 × 900 bei 200 %): grün.
- `clipping_audit::` mit `FP_RAND_VIEWS=Startseite` im Schnelllauf und mit `FP_RAND_FULL=1` (24 Varianten): grün, die Startseite hat keine Funde.
- `cargo check --workspace --all-targets`, `cargo test -p fp-i18n`, Scanner `i18n_keine_festen_texte::`, `cargo test -p fp-neutral`, `cargo clippy -p fp-ui --all-targets` (keine Warnung in eigenen Dateien): grün.
- Bilder (kittest offscreen, wgpu): `docs\screenshots\nutzerbefunde\N11_start_{normal,dunkel,oled}_{de,en}.png`, 1280 × 860, mit Vorschaubildern und Hover über einer Datei. Texte und PNG-Bytes sind mit `fp_neutral::find_in_bytes` geprüft (keine gesperrten Namen bzw. Systemschriften). Erzeugen: `cargo test -p fp-ui --test all welcome_des2::n11_start_bilder -- --ignored`.

## Für die Doku-Agenten

- **NUTZERBEFUNDE N11:** Teil Startbildschirm umgesetzt, Nachprüfung ausstehend. Nachweis sind die Bilder `N11_start_*`. Der Teil Dialoge bleibt bei DES-1 bzw. FORM-1.
- **DESIGN.md §12** sollte nachgezogen werden: Der Aufbau ist jetzt drei Karten in zwei Spalten statt zwei Spalten in einer Karte, mit Vorschaubildern, Fußzeile ohne Logo und Version rechts.
- **design_nachher_startbildschirm*.png** zeigen den alten Stand. Neu erzeugen: `cargo test -p fp-ui --test all design_willkommen::design_willkommen_bilder -- --ignored`. Das habe ich nicht gemacht, weil es nicht meine Dateien sind.

## Nachprüfung (NACHPRUEFUNG.md)

| Was | Ablauf | Erwartet |
|---|---|---|
| Startbildschirm im echten Fenster | Firepixel ohne Dokument starten, mit einigen zuletzt geöffneten PNG/FW-PNG/JPEG-Dateien; 3 Designs, de/en, 100/150 % | Kopfzeile mit Logo, drei Karten, Vorschaubilder erscheinen kurz nach dem Start, Ordner gedämpft, nichts abgeschnitten |
| Tastatur | Tab, Pfeiltasten, Eingabe/Leertaste | Fokusrahmen wandert sichtbar, Eingabe löst aus (Neu öffnet den Dialog „Neues Dokument“, Datei öffnet sich) |
| Große Datei | zuletzt geöffnete Datei mit 8000 × 8000 px | Startbildschirm bleibt bedienbar, die Vorschau kommt nach kurzer Zeit |

## Neue Befunde

Keine.

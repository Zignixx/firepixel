# Firepixel – Kurzanleitung

Firepixel ist ein Grafik- und Webdesign-Programm für Vektor- und Bitmapbearbeitung mit Seiten,
Status (Animationsbildern), Ebenen, Symbolen und Live-Filtern. Es liest und schreibt FW-PNG-Dateien
(`.fw.png`) und bedient sich wie das bekannte Original: gleiche Menüs, Werkzeuge, Bedienfelder,
Tastenkürzel und Mausbedienung.

## Erste Schritte

**Installation:** keine. `firepixel.exe` in einen beliebigen Ordner legen und starten. Die Datei
braucht keine weiteren Bibliotheken. `firepixel-cli.exe` (Kommandozeile und KI-Steuerung) liegt
daneben, wird für die Oberfläche aber nicht gebraucht.

**Start:**

- Doppelklick auf `firepixel.exe`, oder Dateien auf die exe bzw. das offene Fenster ziehen.
- Aus der Eingabeaufforderung: `firepixel [--design=normal|dark|oled] [--mcp[=PORT]] [Datei …]`.

**Sprache:** *Ansicht ▸ Sprache* bzw. *Bearbeiten ▸ Voreinstellungen ▸ Allgemein*. Firepixel bringt
Deutsch und Englisch mit; beim ersten Start gilt die Sprache von Windows. Der Wechsel wirkt sofort,
ohne Neustart. Weitere Sprachen sind einfache `.ftl`-Textdateien in `%APPDATA%\Firepixel\Languages\`
(*Ansicht ▸ Sprache ▸ Sprachordner öffnen…*); eine Anleitung für Übersetzer steht in
`crates\fp-i18n\README.md`. Diese Anleitung gibt es auch auf Englisch (`MANUAL.en.md`).

**Grundlagen:**

- *Datei ▸ Neu* (Strg+N) legt ein Dokument an (Größe, Auflösung, Leinwandfarbe).
- *Datei ▸ Öffnen* (Strg+O) liest FW-PNG und gängige Bildformate: PNG, GIF, JPEG, BMP, TIFF, TGA,
  WBMP, PSD, SVG, WebP, APNG, AVIF, HEIC, JPEG XL, QOI, ICO.
- *Datei ▸ Speichern* (Strg+S) schreibt FW-PNG; alle Ebenen, Seiten, Status, Symbole und
  Live-Filter bleiben bearbeitbar.
- Die Datei-Dialoge (Öffnen, Importieren, Speichern unter, Neu von Vorlage, Als Vorlage speichern)
  starten im zuletzt benutzten Ordner bzw. im Ordner des Dokuments; Vorlagen liegen in
  `%APPDATA%\Firepixel\Templates`. Während ein Datei-Dialog offen ist, zeichnet Firepixel weiter.
- *Bearbeiten ▸ Rückgängig/Wiederholen* (Strg+Z / Strg+Y); das Verlauf-Bedienfeld zeigt alle Schritte.
- Das Fenster hat oben die Anwendungsleiste mit Logo, Menüs, Schnellknöpfen und dem
  Arbeitsbereich-Umschalter, darunter die Hauptsymbolleiste, links die Werkzeuge, rechts die
  Bedienfelder und unten den Eigenschaften-Inspektor.

**Designs:** *Ansicht ▸ Design* bzw. *Bearbeiten ▸ Voreinstellungen ▸ Benutzeroberfläche*:

- **Normal** (hellgrau),
- **Dunkel**,
- **OLED** (reines Schwarz, farbige Werkzeuggruppen, leuchtende Auswahl).

## Werkzeuge

Das Werkzeugbedienfeld hat die Abschnitte Auswählen, Bitmap, Vektor, Web, Farben und Ansicht.
Ein kleines Dreieck am Knopf zeigt weitere Werkzeuge: Maustaste gedrückt halten oder auf das
Dreieck klicken. Jeder Platz merkt sich das zuletzt benutzte Werkzeug.

Häufige Kürzel (Satz „Firepixel“, umschaltbar unter *Bearbeiten ▸ Tastaturbefehle*):

| Taste | Werkzeug | Taste | Werkzeug |
|---|---|---|---|
| V / 0 | Zeiger | A / 1 | Teilauswahl |
| M | Auswahlrahmen | L | Lasso |
| W | Zauberstab | B | Pinsel |
| P | Stift | U | Rechteck / Ellipse / Polygon |
| T | Text | N | Linie |
| K | Segment | J | Hotspot |
| I | Pipette | G | Farbeimer / Verlauf |
| Q | Skalieren | C | Zuschneiden |
| H / Leertaste | Hand | Z | Zoom |

Umschalt beim Ziehen hält Seitenverhältnisse und Winkel fest, Alt zeichnet von der Mitte aus.
Doppelklick auf eine Bitmap öffnet den Bitmapmodus; Doppelklick auf Text bearbeitet ihn. Ein
Rechtsklick öffnet das zum Werkzeug und zur Auswahl passende Kontextmenü.

**Bitmap-Werkzeuge:** Der **Pinsel** malt Pixel: Ist keine Bitmap gewählt, legt er eine neue an,
sonst malt er in die gewählte. **Radiergummi**, **Weichzeichner** und **Stempel** arbeiten mit
weichem Rand wie im Original; Esc beendet das Malen und wechselt zum Zeiger. Das Werkzeug
**Verlauf** hat eigene Farben (Vorgabe Schwarz → Weiß, Klick aufs Farbfeld öffnet den
Verlaufseditor), „Auswahl füllen“ und Toleranz gelten getrennt vom Farbeimer. **Farbe ersetzen**
hat eigene Farben „Von“ und „in“ (Vorgabe Grün → Rot).

**Strich ab Werk:** Linie, Stift, Vektorpfad, Rechteck, Ellipse und Polygon zeichnen wie im Original
mit der Strichart *Einfach ▸ Präzise abger.* (1 px); eine eigene Wahl im Inspektor gilt weiter.

## Menüs

- **Datei:** Neu, Öffnen, Zuletzt geöffnete Dateien, Speichern, Importieren, Exportieren,
  Bildvorschau, HTML-Einrichtung, Batch-Verarbeitung, Drucken.
- **Bearbeiten:** Rückgängig, Ausschneiden/Kopieren/Einfügen. Kopierte Objekte stehen auch anderen
  Programmen als Bild zur Verfügung, ein Bild aus einem anderen Programm wird als Bitmap eingefügt.
  Dazu Duplizieren, Klonen, Objekt einfügen, Voreinstellungen und Tastaturbefehle.
- **Ansicht:** Vergrößerung, Einpassen, Lineale, Raster, Hilfslinien, intelligente Hilfslinien,
  Design, Sprache.
- **Auswählen, Modifizieren, Text, Filter:** wirken auf die Auswahl. Ist ein Befehl grau, nennt die
  Statuszeile beim Ausführen über das Kürzel den Grund. Befehle, die immer anwählbar sind, aber
  kein passendes Ziel finden (z. B. *Vereinfachen…* ohne Pfad), melden das wie im Original mit einer
  Warnung; mit „Nicht mehr anzeigen“ lässt sie sich abschalten.
- *Modifizieren ▸ Transformieren ▸ Numerisch transformieren…:* Mit „Attribute skalieren“ wächst die
  Strichbreite beim Skalieren mit (Drehen ändert sie nie); der Haken wird gemerkt.
- *Modifizieren ▸ Maske ▸ Alles einblenden / Alles ausblenden* legt eine leere Maske über das
  gewählte Objekt. Ebenen-Bedienfeld und Inspektor zeigen weiter das Objekt selbst (z. B. die Bitmap)
  mit der Maske als zweiter Miniatur.
- **Befehle:** Skript ausführen (`.jsf`), gespeicherte Befehle aus dem Befehlsordner
  (`%APPDATA%\Firepixel\Commands`, Unterordner werden Untermenüs), KI-Modelle verwalten,
  Warnmeldungen zurücksetzen und der KI-Steuerkanal.
- **Fenster:** blendet Bedienfelder ein und aus. Unter *Arbeitsbereichlayouts* lassen sich
  Anordnungen speichern und laden; dazu gibt es die Dokumentliste.

## Bedienfelder

Die Bedienfelder sind rechts angedockt. Klick auf den aktiven Reiter klappt die Gruppe ein, einen
Reiter ziehen löst ihn oder verschiebt ihn in eine andere Gruppe. Das Menü ≡ jedes Bedienfelds
enthält dessen Befehle. Tab blendet alle Bedienfelder aus und wieder ein.

Arbeitsbereiche (Umschalter oben rechts):

- Erweiterter Modus,
- Symbolmodus,
- Symbolmodus mit Namen,
- eigene Layouts über *Aktuelles speichern…*.

Die eigenen Layouts liegen als XML unter `%APPDATA%\Firepixel\Workspace Layouts` und sind mit dem
Layoutformat des Originals austauschbar.

Wichtige Bedienfelder:

- **Ebenen:** Sichtbarkeit (Auge), Sperre (Schloss), Unterebenen; Ziehen ordnet um. Unterebenen
  stehen eingerückt als Ordnerzeile mit Aufklapper; Doppelklick auf die Zeile benennt sie um.
- **Seiten / Status:** Seiten und Animationsbilder, Masterseite, Zwiebelschicht.
- **Optimieren:** Exportformat und Farben.
- **Verlauf:** Schritte zurückspulen, wiederholen oder als Befehl speichern.

Der **Eigenschaften-Inspektor** unten zeigt je nach Auswahl Füllung, Strich, Deckkraft,
Mischmodus, Filter (+/−) und Text- bzw. Symbol-Einstellungen.

- **Schriftsuche:** Die aufgeklappte Schriftliste (Text-Inspektor, Textwerkzeug, Texteditor) hat oben
  ein Suchfeld. Tippen filtert (Teilwort, ohne Groß-/Kleinschreibung), ↑/↓ wählen, Eingabe
  übernimmt, Esc schließt ohne Änderung.
- **Texturen und Muster:** Die Listen zeigen alle Texturen bzw. Muster aus den Ressourcenordnern;
  „Andere…“ unter der Liste wählt eine eigene Bilddatei.

## Bitmap nachzeichnen

*Modifizieren ▸ Bitmap nachzeichnen…* (auch im Kontextmenü einer Bitmap) verwandelt ein Pixelbild in
bearbeitbare, gefüllte Vektorpfade. Im Dialog:

- **Vorgabe:** Logo (wenige flache Farben, klare Kanten), Schwarzweiß (Strichzeichnungen, Scans,
  Schrift), Illustration (viele Farben), Foto (weiche Flächen).
- **Einstellungen:** Farben bzw. Schwelle, Detail und Glätte, Ecken, Flecken entfernen.
  **Aufräumen** (bei Logo an) vereinheitlicht Farben, begradigt Linien, erkennt Kreise und Rechtecke
  und spart Knoten; die Feinheiten stehen unter *Erweitert*.
- **Vorschau:** Vergleich, Original, Vektor oder Konturen, dazu Kennzahlen (Pfade, Punkte, Farben,
  Übereinstimmung).

OK übernimmt das Ergebnis der Vorschau als Gruppe auf einer neuen Ebene „Nachgezeichnet“, in einem
Rückgängig-Schritt; mit „Original behalten“ (Vorgabe) bleibt die Bitmap erhalten.

Das Fenster öffnet sich groß (≈ 1050 × 830, höchstens 90 % des Hauptfensters) und merkt sich die
Größe, auf die du es ziehst.

## KI-Auswahl

Drei Werkzeuge am Platz des Zauberstabs (Abschnitt Bitmap) erzeugen Pixelauswahlen mit lokalen
KI-Modellen, ohne Netzverbindung beim Rechnen:

- **Objektauswahl:** auf ein Objekt klicken oder einen Rahmen ziehen; wie beim Zauberstab fügt
  Umschalt+Klick hinzu, Alt+Klick zieht ab.
- **Motiv auswählen** (auch *Auswählen ▸ Motiv auswählen*): wählt das Hauptmotiv mit weicher Kante.
- **Kante verfeinern** (auch *Auswählen ▸ Kante verfeinern…*): verbessert die Kante einer vorhandenen
  Auswahl bei Haaren, Fell und Halbtransparenz.

Die Modelle werden nicht mitgeliefert. Fehlt eines, bietet Firepixel den Download an; erst nach
Zustimmung wird geladen. Übersicht, Lizenzen und Modellordner: *Befehle ▸ KI-Modelle verwalten…*
(Einzelheiten in `docs\MODELLE.md`).

Während ein Modell rechnet, zeigt eine Karte unten über der Leinwand Werkzeug, Modell, die Stufen,
einen Balken und die Zeit; *Abbrechen* oder Esc bricht ab. Im Inspektor des Werkzeugs gibt es
**Feineinstellungen**, die die Modelle wirklich hergeben:

- *Toleranz* (mehr oder weniger Auswahl), *Rand* (Hart, Anti-Alias, Weiche Kante),
  *Ausdehnen* (> 0 größer, < 0 kleiner), *Glätten*,
- nur bei der Objektauswahl: *Variante* (Automatisch, Klein, Mittel, Groß).

Eine Änderung stellt die letzte Auswahl sofort nach, ohne dass das Modell neu rechnet.

## Exportieren

Im Bedienfeld **Optimieren** stellt man das Exportformat ein:

- GIF, animiertes GIF, JPEG, PNG 8/24/32,
- WebP (auch verlustfrei und animiert), AVIF, APNG,
- TIFF, BMP, WBMP, QOI, ICO.

Die Reiter **Vorschau**, **2-fach** und **4-fach** über dem Dokument zeigen das Ergebnis vor dem
Export, jeweils mit Dateigröße und Ladezeit. Bei 4-fach stehen drei Einstellungen zum Vergleich
nebeneinander.

*Datei ▸ Exportieren* schreibt Bilder, Segmente samt HTML/CSS und alle Seiten. In
*Datei ▸ HTML-Einrichtung…* (Register *Allgemein*) schreibt „CSS in externe Datei schreiben“
(Vorgabe an, wirkt mit „CSS für Popupmenüs verwenden“) die Stile in eine eigene `.css`-Datei neben
der HTML-Datei, die per `@import` eingebunden wird.

**Bildvorschau** (*Datei ▸ Bildvorschau…*): Mit den Knöpfen unten (1, 2 oder 4 Vorschaufenster)
vergleichst du Formate und Qualitäten nebeneinander. Jedes Feld hat seine eigene Einstellung, das
angeklickte Feld zeigt sie links. Zoom und Bildlauf gelten für alle Felder. Die Registerkarte
**Optionen** hat:

- Format, Palette, Farben, Verlust, Rastern, Qualität/Glättung (JPEG), Hintergrund;
- die **Farbtabelle**: Klick zeigt die Farbwerte, Knöpfe zum Sperren, für websicher und transparent,
  ≡ bzw. Rechtsklick öffnet das Palettenmenü;
- drei **Pipetten** für die Transparenz;
- **Optimieren auf Größe** (Knopf unten rechts), das die Einstellung für eine Zieldateigröße sucht.

Gespeicherte Einstellungen wählst du aus der Liste oben, „+“ speichert die aktuelle.

## Automatisierung (Skripte, Kommandozeile, MCP)

- **Skripte:** *Befehle ▸ Skript ausführen* führt JSF-Skripte aus (Objekt `fw`). Jedes Skript ist ein
  Rückgängig-Schritt.
- **Batch-Verarbeitung** (*Datei ▸ Batch-Verarbeitung…*): Dateien hineinziehen oder mit
  „Dateien…“ bzw. „Ordner…“ hinzufügen, Schritte und „Speichern als“ wählen, Vorschau vorher/nachher
  ansehen und ausführen. Im Zielordner liegt danach der Bericht `batch-report.html` mit Vorschaubildern.
- **Kommandozeile:** `firepixel-cli` kann Dokumente prüfen (`info`), Befehle anwenden (`run`,
  `batch`), Bilder rendern (`render`), umwandeln (`convert`), Skripte starten (`script`) und
  Übersetzungen prüfen (`language check`). `firepixel-cli <befehl> --help` zeigt Beispiele;
  `--lang de|en` wählt die Sprache der Ausgaben.
- **KI-Steuerung (MCP):** `firepixel-cli mcp` (stdio) oder in der laufenden Oberfläche
  *Befehle ▸ KI-Steuerkanal (MCP)* bzw. der Start mit `--mcp`. Der Server lauscht nur auf
  127.0.0.1 und verlangt ein Token. Adresse und Token stehen in
  `%LOCALAPPDATA%\Firepixel\mcp-bridge.json`. KI-Agenten bekommen Antworten auf Englisch, auf
  Wunsch auf Deutsch. Einzelheiten stehen in `docs\AGENTS.md` (englisch: `docs\AGENTS.en.md`).

## Fehlerbehebung

- **Voreinstellungen** liegen in `%APPDATA%\Firepixel\preferences.json`. Löschen setzt sie zurück.
- **Ordnernamen:** Seit dem 12.10.2026 heißen die Ordner der App englisch: `Languages` (früher
  `Sprachen`), `Models` (früher `Modelle`, unter `%LOCALAPPDATA%\Firepixel`), `Common Library`
  (früher `Allgemeine Bibliothek`). Beim ersten Start einer neuen Version benennt Firepixel die alten
  Ordner einmal um; geht das nicht, liest es sie weiter.
- Ist der Arbeitsbereich durcheinander: *Fenster ▸ Arbeitsbereichlayouts ▸ Standardarbeitsbereich*.
- Warnmeldungen, die mit „Nicht mehr anzeigen“ unterdrückt wurden, kommen über
  *Befehle ▸ Warnmeldungen zurücksetzen* (oder *Voreinstellungen ▸ Alle Warnmeldungen zurücksetzen*)
  zurück.
- „KI-Modell … fehlt“: unter *Befehle ▸ KI-Modelle verwalten…* laden oder die Datei in den dort
  angezeigten Modellordner legen.
- Ressourcen des Originals sind optional. Texturen, Muster, Bibliotheken und die CMYK-Tabelle einer
  vorhandenen Installation nutzt Firepixel, wenn der Ordner unter *Voreinstellungen* eingetragen ist.
  Mitgeliefert wird davon nichts.

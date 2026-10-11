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
- Aus der Eingabeaufforderung: `firepixel [--design=normal|dunkel|oled] [--mcp[=PORT]] [Datei …]`.

**Grundlagen:**

- *Datei ▸ Neu* (Strg+N) legt ein Dokument an (Größe, Auflösung, Leinwandfarbe).
- *Datei ▸ Öffnen* (Strg+O) liest FW-PNG und gängige Bildformate: PNG, GIF, JPEG, BMP, TIFF, TGA,
  WBMP, PSD, SVG, WebP, APNG, AVIF, HEIC, JPEG XL, QOI, ICO.
- *Datei ▸ Speichern* (Strg+S) schreibt FW-PNG; alle Ebenen, Seiten, Status, Symbole und
  Live-Filter bleiben bearbeitbar.
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

## Menüs

- **Datei:** Neu, Öffnen, Zuletzt geöffnete Dateien, Speichern, Importieren, Exportieren,
  Bildvorschau, HTML-Einrichtung, Batch-Verarbeitung, Drucken.
- **Bearbeiten:** Rückgängig, Ausschneiden/Kopieren/Einfügen. Kopierte Objekte stehen auch anderen
  Programmen als Bild zur Verfügung, ein Bild aus einem anderen Programm wird als Bitmap eingefügt.
  Dazu Duplizieren, Klonen, Objekt einfügen, Voreinstellungen und Tastaturbefehle.
- **Ansicht:** Vergrößerung, Einpassen, Lineale, Raster, Hilfslinien, intelligente Hilfslinien,
  Design.
- **Auswählen, Modifizieren, Text, Filter:** wirken auf die Auswahl. Ist ein Befehl grau, nennt die
  Statuszeile beim Ausführen über das Kürzel den Grund.
- **Befehle:** Skript ausführen (`.jsf`), gespeicherte Befehle aus dem Befehlsordner
  (`%APPDATA%\Firepixel\Commands`, Unterordner werden Untermenüs) und der KI-Steuerkanal.
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

- **Ebenen:** Sichtbarkeit (Auge), Sperre (Schloss), Unterebenen; Ziehen ordnet um.
- **Seiten / Status:** Seiten und Animationsbilder, Masterseite, Zwiebelschicht.
- **Optimieren:** Exportformat und Farben.
- **Verlauf:** Schritte zurückspulen, wiederholen oder als Befehl speichern.

Der **Eigenschaften-Inspektor** unten zeigt je nach Auswahl Füllung, Strich, Deckkraft,
Mischmodus, Filter (+/−) und Text- bzw. Symbol-Einstellungen.

## Exportieren

Im Bedienfeld **Optimieren** stellt man das Exportformat ein:

- GIF, animiertes GIF, JPEG, PNG 8/24/32,
- WebP (auch verlustfrei und animiert), AVIF, APNG,
- TIFF, BMP, WBMP, QOI, ICO.

Die Reiter **Vorschau**, **2-fach** und **4-fach** über dem Dokument zeigen das Ergebnis vor dem
Export, jeweils mit Dateigröße und Ladezeit. Bei 4-fach stehen drei Einstellungen zum Vergleich
nebeneinander.

*Datei ▸ Exportieren* schreibt Bilder, Segmente samt HTML/CSS und alle Seiten.

## Automatisierung (Skripte, Kommandozeile, MCP)

- **Skripte:** *Befehle ▸ Skript ausführen* führt JSF-Skripte aus (Objekt `fw`). Jedes Skript ist ein
  Rückgängig-Schritt.
- **Kommandozeile:** `firepixel-cli` kann Dokumente prüfen (`info`), Befehle anwenden (`run`,
  `batch`), Bilder rendern (`render`), umwandeln (`convert`) und Skripte starten (`script`).
  `firepixel-cli <befehl> --help` zeigt Beispiele.
- **KI-Steuerung (MCP):** `firepixel-cli mcp` (stdio) oder in der laufenden Oberfläche
  *Befehle ▸ KI-Steuerkanal (MCP)* bzw. der Start mit `--mcp`. Der Server lauscht nur auf
  127.0.0.1 und verlangt ein Token. Adresse und Token stehen in
  `%LOCALAPPDATA%\Firepixel\mcp-bridge.json`. Einzelheiten stehen in `docs\AGENTS.md`.

## Fehlerbehebung

- **Voreinstellungen** liegen in `%APPDATA%\Firepixel\preferences.json`. Löschen setzt sie zurück.
- Ist der Arbeitsbereich durcheinander: *Fenster ▸ Arbeitsbereichlayouts ▸ Standardarbeitsbereich*.
- Warnmeldungen, die mit „Nicht mehr anzeigen“ unterdrückt wurden, kommen über
  *Befehle ▸ Warnmeldungen zurücksetzen* zurück.
- Ressourcen des Originals sind optional. Texturen, Muster, Bibliotheken und die CMYK-Tabelle einer
  vorhandenen Installation nutzt Firepixel, wenn der Ordner unter *Voreinstellungen* eingetragen ist.
  Mitgeliefert wird davon nichts.

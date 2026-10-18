# Firepixel – Kurzanleitung

Firepixel ist ein Grafik- und Webdesign-Programm für Vektor- und Bitmapbearbeitung mit Seiten,
Status (Animationsbildern), Ebenen, Symbolen und Live-Filtern. Es liest und schreibt FW-PNG-Dateien
(`.fw.png`) und bedient sich wie das bekannte Original: gleiche Menüs, Werkzeuge, Bedienfelder,
Tastenkürzel und Mausbedienung.

## Erste Schritte

**Installation:** keine. `firepixel.exe` in einen beliebigen Ordner legen und starten. Die Datei
braucht keine weiteren Bibliotheken. `firepixel-cli.exe` (Kommandozeile und KI-Steuerung) liegt
daneben, wird für die Oberfläche aber nicht gebraucht.

**Linux (x86_64, glibc 2.35 oder neuer, z. B. Ubuntu 22.04 oder Debian 12):** entweder das AppImage mit
`chmod +x Firepixel-x86_64.AppImage` ausführbar machen und starten, oder das tar.gz entpacken und im
Ordner `Firepixel-Linux-x64` `./firepixel` starten (dort liegen auch `firepixel-cli`, die Anleitungen,
eine `.desktop`-Datei und Symbole). Firepixel läuft unter Wayland und X11 und braucht einen Vulkan- oder
OpenGL-Treiber sowie die Bibliotheken `libxkbcommon0`, `libxkbcommon-x11-0` (X11) und
`libwayland-client0` (Wayland), die auf den meisten Desktops schon vorhanden sind. Öffnen- und
Speichern-Dialoge laufen über das XDG-Desktop-Portal; `xdg-desktop-portal` und ein Portal-Backend für
den Desktop (GTK, KDE, …) müssen installiert sein. Der Linux-Build ist neu und noch auf wenigen Desktops
ausprobiert.

**Start:**

- Doppelklick auf `firepixel.exe`, oder Dateien auf die exe bzw. das offene Fenster ziehen.
- Aus der Eingabeaufforderung: `firepixel [--design=normal|dark|oled] [--mcp[=PORT]] [Datei …]`.

**Sprache:** *Ansicht ▸ Sprache* bzw. *Bearbeiten ▸ Voreinstellungen ▸ Allgemein*. Firepixel bringt
Deutsch und Englisch mit; beim ersten Start gilt die Sprache von Windows. Der Wechsel wirkt sofort,
ohne Neustart. Weitere Sprachen sind einfache `.ftl`-Textdateien in `%APPDATA%\Firepixel\Languages\`
(*Ansicht ▸ Sprache ▸ Sprachordner öffnen…*); eine Anleitung für Übersetzer steht in
`crates\fp-i18n\README.md`. Diese Anleitung gibt es auch auf Englisch (`MANUAL.md`).

**Grundlagen:**

- *Datei ▸ Neu* (Strg+N) legt ein Dokument an (Größe, Auflösung, Leinwandfarbe). Neue Dokumente heißen
  „Unbenannt-1.fw.png“, „Unbenannt-2.fw.png“ usw. Hast du vorher Objekte oder Pixel mit Strg+C kopiert,
  schlägt *Neu* deren Größe vor.
- *Datei ▸ Öffnen* (Strg+O) liest FW-PNG und gängige Bildformate: PNG, GIF, JPEG, BMP, TIFF, TGA,
  WBMP, PSD, SVG, WebP, APNG, AVIF, HEIC, JPEG XL, QOI, ICO. In PSD-Dateien werden Formebenen zu Pfaden
  und Gruppen zu Unterebenen. Texte aus AI-/EPS- und SVG-Dateien kommen als bearbeitbare Textobjekte
  (AI-Flächentext an derselben Stelle wie im Original); der Dateityp heißt im Dialog „AI (*.ai;*.art)“.
- *Datei ▸ Speichern* (Strg+S) schreibt FW-PNG; alle Ebenen, Seiten, Status, Symbole und
  Live-Filter bleiben bearbeitbar.
- Die Datei-Dialoge (Öffnen, Importieren, Speichern unter, Neu von Vorlage, Als Vorlage speichern)
  starten im zuletzt benutzten Ordner bzw. im Ordner des Dokuments; Vorlagen liegen in
  `%APPDATA%\Firepixel\Templates`. Während ein Datei-Dialog offen ist, zeichnet Firepixel weiter.
- *Datei ▸ Neu von Vorlage…*: Beim ersten Aufruf legt Firepixel zehn eigene Vorlagen in fünf Ordnern an
  (`Document Presets`, `Grids`, `Mobile`, `Web`, `Wireframes`), z. B. Web-Banner 728 × 90 oder Smartphone
  390 × 844. Jede Vorlage bringt eine passende Exportvorgabe mit (PNG 32 bzw. JPEG). Gelöschte Vorlagen
  kommen nicht wieder.
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

Häufige Kürzel (Satz „Firepixel“, umschaltbar unter *Bearbeiten ▸ Tastaturkurzbefehle…*; weitere Sätze heißen
„Bildbearbeitung“, „Vektorzeichnung“ und „Klassische Vektor-Tasten“):

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
Rechtsklick öffnet das zum Werkzeug und zur Auswahl passende Kontextmenü. Esc ohne laufende Aktion hebt
die Objektauswahl auf (offene Popups und Menüs schließt das erste Esc).

**Bitmap-Werkzeuge:** Der **Pinsel** malt Pixel: Ist keine Bitmap gewählt, legt er eine neue an,
sonst malt er in die gewählte. **Radiergummi**, **Weichzeichner** und **Stempel** arbeiten mit
weichem Rand wie im Original; Esc beendet das Malen und wechselt zum Zeiger. Das Werkzeug
**Verlauf** hat eigene Farben (Vorgabe Schwarz → Weiß, Klick aufs Farbfeld öffnet den
Verlaufseditor), „Auswahl füllen“ und Toleranz gelten getrennt vom Farbeimer. **Farbe ersetzen**
hat eigene Farben „Von“ und „in“ (Vorgabe Grün → Rot).
Beim Malen mit Pinsel, Bleistift, Radiergummi oder Stempel erscheint die Vorschau an der Stelle der
Bitmap in der Stapelreihenfolge, mit ihrer Deckkraft, Füllmethode, ihren Effekten und Masken. Ein
Strich, der eine neue Bitmap anlegt oder in eine Bitmap in einem Symbol malt, wird bis zum Loslassen
obenauf angezeigt.

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
- *Auswählen ▸ Auswahl umkehren* kehrt nur eine Pixelauswahl um; ohne sie erscheint eine Warnung, und
  die gewählten Objekte bleiben unverändert.
- *Text ▸ Größe ▸ Größer / Kleiner* ändern die Größe um genau 1 pt (24 → 25 → 24), bei gewählten Texten
  und im offenen Texteditor; die kleinste Größe ist 1 pt. Die Ausrichtung *Blocksatz gestreckt* wirkt
  nur auf Textblöcke mit fester Breite; Punkttext (automatische Breite) behält seine Ausrichtung.
- **Zuschneiden:** Das Zuschneidewerkzeug, *Bearbeiten ▸ Dokument zuschneiden* und *Modifizieren ▸
  Leinwand ▸ Leinwandgröße* folgen der Option *Bearbeiten ▸ Voreinstellungen ▸ Bearbeitung ▸ Objekte
  beim Zuschneiden löschen* (standardmäßig an). An: Objekte ganz außerhalb der neuen Leinwand werden auf
  allen Ebenen und Status gelöscht, Bitmaps über den Rand beschnitten, Vektorobjekte über den Rand
  bleiben ganz. Aus: nichts wird gelöscht oder beschnitten, die Objekte wandern nur mit der Leinwand.
- *Auswählen ▸ Ähnliches auswählen* nimmt Objekte mit gleicher Füllung, gleichem Strich, gleicher Deckkraft
  und gleichem Mischmodus dazu.
- *Text ▸ Von Pfad lösen* macht den Text wieder zu einem Textrahmen mit seiner früheren Breite, mittig über
  dem Pfad. Der Pfad liegt danach oben. Text an einem Pfad zeigt wie im Original nur den ersten Absatz.
- **Filter auf Vektorobjekten:** Vorher fragt Firepixel „Dieser Vorgang konvertiert Vektoren in Bitmaps.“
  (OK, Abbrechen, „Nicht mehr anzeigen“).
- *Ansicht ▸ Gamma 1.8* zeigt die Leinwand heller, wie auf einem Monitor mit Macintosh-Gamma. Dokument und
  Export bleiben unverändert.
- *Bearbeiten ▸ Einfügen* legt kopierte Objekte direkt über das oberste gewählte Objekt (in dessen Ebene;
  ist ein Objekt in einer Gruppe gewählt, über die Gruppe). Ohne Auswahl landen sie oben in der aktiven Ebene.
- *Modifizieren ▸ Maske ▸ Als Maske gruppieren* setzt die Maskengruppe an den Platz des untersten gewählten
  Objekts; *Gruppieren* bleibt am Platz des obersten. Vereinigen, Überschneiden und Verbinden legen das
  Ergebnis an den Platz des oberen Objekts, Öffnen und Zuschneiden an den des unteren (*Pfade kombinieren*).
- *Auswählen ▸ Auswahlrahmen in Pfad konvertieren* gibt dem neuen Pfad Füllung und Strich wie einer neu
  gezeichneten Form (Farben aus dem Werkzeugbedienfeld, Füll- und Strichart aus dem Inspektor).
- *Auswählen ▸ Nächste Auswahl / Vorherige Auswahl* (Strg+Umschalt+Rechts / Links) gehen durch den
  Auswahlverlauf des Dokuments: Jede neue Objektauswahl ist ein Eintrag, am Ende passiert nichts.
- *Modifizieren ▸ Symbol ▸ Symbol bearbeiten* öffnet das Symbol im Fenster des Dokuments, so groß wie die
  Seite und mit dem Titel „<Dokument> (Symbol)“. Eine Pfadleiste unter der Ansichtsleiste zeigt
  „Seite 1 ▸ <Symbol>“; „← Zurück“ kehrt ins Dokument zurück. Nach *In Symbol konvertieren* hat die Instanz
  keinen eigenen Namen.
- *Modifizieren ▸ Symbol ▸ Instanzen tweenen…* schlägt beim ersten Öffnen 10 Schritte vor, danach den zuletzt
  bestätigten Wert (bis zum Programmende).
- *Modifizieren ▸ Animation ▸ Auswahl animieren…* fragt, wenn die Animation über den letzten Status
  hinausreicht: „Automatisch neue Status hinzufügen?“ (OK hängt Status an, Abbrechen legt die Animation ohne
  neue Status an, „Nicht mehr anzeigen“ lässt sich unter *Warnmeldungen zurücksetzen* wieder einschalten).
- **Befehle:** Skript ausführen (`.jsf`), gespeicherte Befehle aus dem Befehlsordner
  (`%APPDATA%\Firepixel\Commands`, Unterordner werden Untermenüs), KI-Modelle verwalten,
  Warnmeldungen zurücksetzen und der KI-Steuerkanal. Mitgelieferte Befehle: **Bildschirmfoto** (nach
  einem Countdown der ganze Bildschirm oder ein als Zahlen eingegebener Ausschnitt, aufgenommen mit
  dem Werkzeug des Systems, öffnet sich als neues Dokument), **Symbolskript erstellen** (ein
  JSF-Skript, das das gewählte Grafiksymbol nachbaut; Bitmaps, verschachtelte Instanzen und
  zusammengesetzte Formen fehlen, Verläufe werden Vollfarben) und **Web ▸ Gemeinsame Palette** (eine
  Palette über alle offenen Dokumente, im Bedienfeld Farbfelder, auf Wunsch als `.act` gespeichert).
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
- **Seiten / Status:** Seiten und Animationsbilder, Masterseite, Zwiebelschicht. Ist eine
  Animationsinstanz gewählt, zeigt die Leinwand ihren Bewegungspfad (grüner Start, rotes Ende), und der
  Auswahlrahmen umschließt nur das aktuelle Bild.
- **Optimieren:** Exportformat und Farben.
- **Verlauf:** Schritte zurückspulen, wiederholen oder als Befehl speichern.
- **URL:** URLs für Slices, Hotspots und Schaltflächen. Das Menü ≡ enthält *Verwendete URLs in
  Bibliothek aufnehmen* (aus den Slices, Hotspots und Schaltflächen aller offenen Dokumente), *Nicht
  verwendete URLs löschen*, *URL hinzufügen...*, *URL bearbeiten...* und *URL löschen* (beide nur mit
  gewählter URL), *Neue URL-Bibliothek...*, *URLs importieren...* und *URLs exportieren...* (`.txt` mit
  einer URL pro Zeile oder `.htm` mit einem Link pro URL) sowie die Hilfe. Die Standardbibliothek ist
  `URLs.txt` in `%APPDATA%\Firepixel`, weitere Bibliotheken liegen als HTML-Dateien im Ordner
  `URL Libraries` daneben und lassen sich in der Bibliotheksliste des Bedienfelds wählen. Änderungen an
  einer Bibliothek lassen sich nicht mit Strg+Z rückgängig machen, und diese Menübefehle stehen Skripten,
  der Kommandozeile und MCP nicht zur Verfügung.

Der **Eigenschaften-Inspektor** unten zeigt je nach Auswahl Füllung, Strich, Deckkraft,
Mischmodus, Filter (+/−) und Text- bzw. Symbol-Einstellungen.

- **Filter (+):** Das Menü „+“ gliedert die Effekte wie im Original, z. B. *Abgeflachte Kante und
  Relief* ▸ *Abgeflachte Kante innen*, *Abgeflachte Kante außen*, *Relief vertieft*, *Relief erhaben*.
  Die Art steht im Menüeintrag; der Dialog hat dafür kein eigenes Feld. Doppelklick auf einen Eintrag
  der Filterliste öffnet seine Einstellungen.

- **Schriftsuche:** Die aufgeklappte Schriftliste (Text-Inspektor, Textwerkzeug, Texteditor) hat oben
  ein Suchfeld. Tippen filtert (Teilwort, ohne Groß-/Kleinschreibung), ↑/↓ wählen, Eingabe
  übernimmt, Esc schließt ohne Änderung.
- **Texturen und Muster:** Die Listen zeigen alle Texturen bzw. Muster aus den Ressourcenordnern;
  „Andere…“ unter der Liste wählt eine eigene Bilddatei.
- **Farbwähler:** Ein Klick auf ein Farbfeld öffnet das Popup mit Farbfeldern, HEX-Feld und Pipette. Die
  Pipette nimmt beim Überfahren jede sichtbare Farbe im Fenster auf, auch über Bedienfeldern, Menüs und
  offenen Dialogen. Ein Klick übernimmt die Farbe. Außerhalb des Firepixel-Fensters bleibt die
  Arbeitsfarbe.

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
  Umschalt+Klick hinzu, Alt+Klick zieht ab. Mit *Bearbeiten ▸ Voreinstellungen ▸ Bearbeitung ▸
  Auswahlvorschau beim Überfahren anzeigen* (Vorgabe aus) zeigt der ruhende Mauszeiger den Umriss des
  Objekts gestrichelt; das geht erst, wenn das Modell für das Bild geladen ist, und ein Klick auf
  dieselbe Stelle wählt dann sofort aus.
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

Eine Änderung stellt die letzte Auswahl sofort nach, ohne dass das Modell neu rechnet. Nach
*Auswählen ▸ Motiv auswählen* zeigt der Inspektor diese Feineinstellungen auch bei Zeiger, Unterauswahl,
Auswahlrahmen, Lasso und Zauberstab, solange die Auswahl unverändert ist. Auch eine sehr weiche Auswahl
(z. B. auf einem halbtransparenten Pinselstrich) ist an den Auswahlameisen zu sehen.

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
der HTML-Datei, die per `@import` eingebunden wird. Neben „Hintergrundbild:“ wählt „Durchsuchen...“ ein
Bild. Liegt es im Ordner des Dokuments, wird der Pfad relativ eingetragen. „Seitenstile in externe
Datei schreiben (styles.css)“ legt die Seitenstile in `styles.css` neben der HTML-Datei ab; der Stil
*Allgemeines HTML5* schreibt eine HTML5-Seite; Schaltflächen behalten in jedem Stil ihr
Navigationsleisten-Skript. *Als Standard festlegen* speichert die Werte des Dialogs für neue Dokumente
und für Dokumente ohne eigene HTML-Einstellungen.
In den PDF-Exportoptionen schreibt die Komprimierung „JPEG 2000 (JPEG)“ normales JPEG (DCT), weil
Firepixel keinen JPEG-2000-Kodierer hat; der Eintrag „Producer“ des PDFs vermerkt das.

**PDF- und PSD-Exportoptionen:**

- PDF, „Textauswahl aktivieren“: Jedes sichtbare Textobjekt wird als unsichtbarer Text über das
  Seitenbild geschrieben, so lässt sich Text im PDF-Betrachter markieren, suchen und kopieren. Nur
  Zeichen des westeuropäischen Zeichensatzes (WinAnsi) kommen in die Textebene; andere bleiben im Bild
  sichtbar.
- PSD, „In PSD-Ebenen konvertieren“: Jedes Objekt wird zu einer eigenen PSD-Ebene, gruppiert nach
  Dokumentebene. Mit „Bearbeitbarkeit beibehalten“ werden bei Text die Textobjekte zu bearbeitbaren
  Textebenen, bei Effekten Schlagschatten, innerer Schatten, Leuchten, Abgeflachte Kante/Relief und
  Farbfüllung zu Ebeneneffekten, bei Vektor Pfade mit einfacher Füllung zu Füllebenen mit Vektormaske.
  Die anderen Einstellungen rechnen Text, Effekte und Vektoren in die Pixel ein.

**Web-Funktionen:**

- *Bearbeiten ▸ HTML-Code kopieren…* ist ein Assistent mit drei Seiten: HTML-Stil, Basis-Dateiname für die
  Segmente, Ordner für die Bilder. Er hat die Knöpfe < Zurück, Weiter > und Fertig stellen. Danach liegt der
  Code in der Zwischenablage.
- *Modifizieren ▸ Popupmenü ▸ Popupmenü hinzufügen…* öffnet den Popupmenü-Editor. Weiter > und < Zurück
  führen durch die vier Register, Fertig legt das Menü an. Im Register „Inhalt“ stehen die Einträge in einer
  Tabelle (Text │ Hyperlink │ Ziel), die gewählte Zeile ist direkt bearbeitbar; die Knöpfe + / − / → / ←
  fügen Einträge hinzu, löschen sie, rücken sie ein oder aus.
- Die Segment-Hilfslinien teilen die Seite wie im Original: erst waagerecht über dem obersten Segment, dann
  senkrecht an den Segmentkanten, ohne ein Segment zu schneiden.
- Schaltflächen-Instanzen gelten auf der Leinwand als Segment. Sie bekommen die grüne Segmentüberlagerung
  und die Segment-Hilfslinien.

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
  Rückgängig-Schritt. `dom.layers` listet wie im Original alle Ebenen flach, jede Unterebene direkt nach
  ihrer Ebene. `dom.topLayers` enthält nur die oberste Stufe.
- **Batch-Verarbeitung** (*Datei ▸ Batch-Verarbeitung…*): Dateien hineinziehen oder mit
  „Dateien…“ bzw. „Ordner…“ hinzufügen, Schritte und „Speichern als“ wählen, Vorschau vorher/nachher
  ansehen und ausführen. Im Zielordner liegt danach der Bericht `batch-report.html` mit Vorschaubildern.
  Der Reiter „Plan“ des Trockenlaufs schätzt die Größe jeder Ausgabedatei („≈ 192 KB“) und die Summe.
  Bricht man einen Lauf ab, zeigt die Ende-Karte „Abgebrochen: n fertig, m nicht verarbeitet“. Die
  Größensumme zählt dann nur fertige Dateien. Alle Batch-Schritte und ihre Parameter beschreibt
  `docs\BATCH.md` (englisch); „Hilfe“ im Kartenmenü öffnet den passenden Abschnitt.
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
  Die Zeile „ffmpeg für Videos“ (*Voreinstellungen ▸ Zusatzmodule*) ist dieselbe Einstellung wie im
  Batch-Studio; leer heißt automatische Suche.
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

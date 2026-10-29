# Firepixel Studio – Kurzanleitung

Firepixel Studio (kurz: Firepixel) ist ein Grafik- und Webdesign-Programm für Vektor- und Bitmapbearbeitung mit Seiten,
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

**macOS:** Das Fenster behält die native Titelleiste mit den Ampelknöpfen (Schließen, Minimieren,
Zoomen); die Menüleiste von Firepixel liegt direkt darunter. Cmd+Q beendet wie Datei ▸ Beenden und
fragt vorher, ob geänderte Dokumente gespeichert werden sollen. Das Programmmenü in der Menüleiste von
macOS bietet Über Firepixel Studio, Dienste, Firepixel Studio ausblenden, Andere ausblenden, Alle
einblenden und Firepixel Studio beenden (Cmd+Q, ebenfalls mit der Rückfrage).

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
  PDF-Dateien (und AI-Dateien ab Version 9) bringen ihre Ebenen (optionale Inhaltsgruppen) als Ebenen
  mit: ausgeblendete Gruppen werden ausgeblendete Ebenen, verschachtelte heißen „Eltern / Kind“, Inhalt
  außerhalb jeder Gruppe landet in „Ebene 1“.
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
  Kopieren und Ausschneiden erhalten eine eigene Zeile („Kopie“, „Ausschneiden“); Kopieren markiert das
  Dokument nicht als geändert. Aufeinanderfolgende Verschiebungen derselben Auswahl mit den Pfeiltasten
  bilden eine gemeinsame Zeile mit dem summierten Versatz; eine andere Auswahl, ein anderer Befehl oder
  Rückgängig beginnt eine neue Zeile. Wiederholen (Strg+Y, Strg+Umschalt+Z) stellt zuerst einen rückgängig
  gemachten Schritt wieder her (*Wiederholen: ‹Schritt›*). Ist nichts rückgängig gemacht, führt es den letzten
  Schritt als neue Zeile noch einmal aus (ebenfalls *Wiederholen: ‹Schritt›*): nach Klonen ein weiterer Klon, nach
  einem Verschieben dieselbe Verschiebung, nach einem Stiftpunkt derselbe Punkt; auch eine Wiederholung lässt sich
  wiederholen. Direkt nach einem Wiederherstellen, nach einem Bitmap-Strich (Pinsel, Buntstift, Radiergummi,
  Retuschewerkzeuge) und ohne Schritte zeigt das Menü *Wiederholen nicht möglich*, und nichts geschieht.
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

**Tooltips:** Der Tooltip eines Werkzeugs zeigt Name und Kürzel, z. B. „Zeiger (V, 0)“, und in einer
zweiten Zeile eine kurze Beschreibung, was das Werkzeug tut (die Zeilen eines Flyouts zeigen die
Beschreibung als Tooltip). Knöpfe der Werkzeugleiste zeigen nur den Befehlsnamen. Im Inspektor der
Retusche-Werkzeuge (Weichzeichner, Scharfzeichner, Abwedler, Nachbelichter, Verwischen, Stempel) und des
Radiergummis beschreiben sich die Felder selbst: „Pinselstärke“, „Pinselvorschau“, „Kantenweichheit“,
„Runde Pinselspitze“, „Druck“ usw. Alle diese QuickInfos schaltet *Ansicht ▸ QuickInfos* (aus bleibt die erste Zeile, siehe unten).

Häufige Kürzel (Satz „Firepixel“, umschaltbar unter *Bearbeiten ▸ Tastaturkurzbefehle…*; weitere Sätze heißen
„Bildbearbeitung“, „Vektorzeichnung“ und „Klassische Vektor-Tasten“):

| Taste | Werkzeug | Taste | Werkzeug |
|---|---|---|---|
| V / 0 | Zeiger / Hintere auswählen | A / 1 | Teilauswahl / Maskenelement-Auswahl |
| M | Auswahlrahmen | L | Lasso |
| W | Zauberstab | B | Pinsel / Bleistift |
| P | Stift | U | Rechteck / Ellipse / Polygon |
| T | Text | N | Linie |
| K | Segment | J | Hotspot |
| I | Pipette | G | Farbeimer / Verlauf |
| Q | Skalieren | C | Zuschneiden |
| H / Leertaste | Hand | Z | Zoom |

**B** wechselt zwischen Pinsel und Bleistift; der erste Druck aus einem anderen Werkzeug wählt das zuletzt
benutzte der beiden (den Pinsel, solange keines benutzt wurde).

Umschalt beim Ziehen hält Seitenverhältnisse und Winkel fest, Alt zeichnet von der Mitte aus.
Doppelklick auf eine Bitmap öffnet den Bitmapmodus; Doppelklick auf Text bearbeitet ihn. Ein
Rechtsklick öffnet das zum Werkzeug und zur Auswahl passende Kontextmenü. Esc ohne laufende Aktion hebt
die Objektauswahl auf (offene Popups und Menüs schließt das erste Esc).

**Objekte verschieben:** ein gewähltes Objekt mit dem Zeiger ziehen (Alt: eine Kopie). Bei gedrückter
Taste stehen die Objekte wirklich an der neuen Stelle, verrechnet mit allem darüber und darunter
(Mischmodus, Deckkraft, Live-Effekte wie ein Schlagschatten wandern mit); was man sieht, ist genau das
Ergebnis nach dem Loslassen. Auswahlrahmen und Griffe verschwinden beim Ziehen und kommen beim Loslassen
zurück. Der Zug beginnt, wenn der Zeiger mehr als 3·√Zoom Bildschirmpixel weit bewegt wurde (100 %: 4 px,
800 %: 9 px), und verschiebt in ganzen Dokumentpixeln. Mit intelligenten Hilfslinien rastet ein kleiner
Zug auf die Ausgangslage zurück (eine gestrichelte Hilfslinie zeigt das). Das Loslassen schreibt einen
Verlaufsschritt „Verschieben“; Esc bei gedrückter Taste setzt die Objekte ohne Schritt zurück.
**Voreinstellungen ▸ Bearbeitung ▸ Vorschau beim Ziehen** (`PreviewDrag`, ab Werk an): aus, bleibt beim
Ziehen das alte Bild stehen und nur der Umriss der Objekte (1 px blau) folgt dem Zeiger. Sehr große
Auswahlen (mehr als 400 Objekte) werden als bewegtes Bild gezeigt, damit das Ziehen flüssig bleibt.

**Auswählen mit Zeiger und Teilauswahl:** Ziehen auf leerer Fläche zieht einen Auswahlrahmen auf; jedes
berührte Objekt wird gewählt, der Rahmen schreibt keine Verlaufszeile. Mit dem **Zeiger** **schaltet**
Umschalt (auch Umschalt+Alt) jedes Objekt im Rahmen **um**: nicht gewählte kommen hinzu, gewählte werden
abgewählt – ein Umschalt-Rahmen über gewählte Objekte nimmt sie also aus der Auswahl. Strg ergibt einen
normalen Rahmen. Ein Alt-Rahmen auf leerer Fläche wählt wie die Teilauswahl: die Kinder einer Gruppe
einzeln, die Auswahl wird ersetzt; Alt-Ziehen an einem Objekt zieht weiterhin eine Kopie. Mit der
**Teilauswahl** fügt Umschalt die Objekte bzw. Punkte im Rahmen hinzu (nichts wird abgewählt), Alt ergibt
einen normalen Rahmen. Der Zeiger zeigt bei gehaltenem Alt den hohlen Teilauswahl-Pfeil und während jedes
Alt-Zugs (Kopie oder Alt-Rahmen) den Pfeil mit „+“, ebenso der Alt-Rahmen der Teilauswahl; Umschalt
zeigt kein Zeichen. Firepixel-Erweiterung: Der Eigenschafteninspektor dieser beiden Werkzeuge (nichts
gewählt) hat die Option **Auswahlrahmen: Rechteck / Lasso** (Voreinstellung `Firepixel_PointerLasso`,
Vorgabe Rechteck). Mit Lasso zeichnet das Ziehen eine freie Kontur wie das Lasso-Werkzeug; beim Loslassen
wählt der Zeiger die Objekte, deren Mitte darin liegt, die Teilauswahl die Ankerpunkte darin, mit
denselben Regeln für die Umschalttasten. Firepixel-Erweiterung (Vorgabe aus): Mit **Voreinstellungen ▸
Bearbeitung ▸ Alt-Ziehen auf leerer Fläche hebt Auswahl auf** (Voreinstellung
`Firepixel_AltMarqueeDeselects`) **entfernt** ein Alt-Rahmen (Alt ohne Umschalt) von Zeiger oder
Teilauswahl, auf leerer Fläche begonnen, stattdessen jedes Objekt darin aus der Auswahl – gewählte
Gruppen ebenso wie einzeln gewählte Gruppenkinder –, die Objekte außerhalb bleiben gewählt; es entsteht
keine Verlaufszeile, ein Alt-Klick ohne Ziehen lässt die Auswahl stehen, die Lasso-Kontur wirkt genauso
und der Zeiger zeigt den Pfeil mit „−“ statt „+“. Alt-Ziehen an einem Objekt zieht weiterhin eine Kopie,
Umschalt+Alt schaltet beim Zeiger weiterhin um.
Mit **Voreinstellungen ▸ Bearbeitung ▸ Mit Strg-Ziehen duplizieren statt mit Alt-Ziehen** (`Firepixel_DuplicateWithCtrlDrag`, Vorgabe aus) zieht Strg-Ziehen der Auswahl eine Kopie (Zeiger mit „+“) und Alt-Ziehen verschiebt ohne Kopie, Alt behält seine übrigen Aufgaben (Alt-Klick wählt das Kind einer Gruppe, Alt-Rahmen, Alt+Pfeiltaste dupliziert).
Mit **Voreinstellungen ▸ Bearbeitung ▸ Umschalt-Auswahlrahmen fügt nur hinzu** (`Firepixel_ShiftMarqueeAddsOnly`, Vorgabe aus) fügt der Umschalt-Rahmen des Zeigers jedes Objekt darin hinzu statt es umzuschalten (Zeiger mit „+“), und der Alt-Rahmen von Zeiger und Teilauswahl entfernt Objekte, solange die Option an ist, auch ohne die Option zum Aufheben per Alt-Ziehen.
Mit **Voreinstellungen ▸ Bearbeitung ▸ Umschalt + Mausrad rollt waagerecht** (`Firepixel_ShiftWheelHorizontal`, Vorgabe aus) rollt Umschalt + Mausrad die Leinwand waagerecht um den Betrag, den das Rad senkrecht rollt; unabhängig davon zeigen Listen-Bedienfelder wie Ebenen eine waagerechte Bildlaufleiste, wenn tief verschachtelte Zeilen breiter als das Bedienfeld sind, und Umschalt + Mausrad über der Liste rollt sie seitwärts.
Mit **Voreinstellungen ▸ Bearbeitung ▸ Pfeiltasten verschieben die gewählten Pixel mit den Auswahlwerkzeugen** (`Firepixel_ArrowKeysMovePixels`, Vorgabe aus) verschieben die Pfeiltasten (Umschalt 10 px) die gewählten Pixel einer Bitmap samt Auswahlrahmen, solange Zeiger, Hintere auswählen, Teilauswahl oder Maskenelement-Auswahl aktiv ist (die frei gewordene Fläche wird transparent, aufeinanderfolgende Tastendrücke sind eine Verlaufszeile „Verschieben“, Alt+Pfeiltaste verschiebt eine Kopie), während Auswahlrahmen-Werkzeug, Lasso und Zauberstab weiterhin nur den Auswahlrahmen verschieben; aus, verschieben die Pfeiltasten mit jedem Werkzeug nur den Auswahlrahmen.

**Bitmap-Werkzeuge:** Der **Pinsel** malt Pixel: Ein Strich, der auf einer Bitmap beginnt, malt in
diese Bitmap (ohne Zusatztaste, gewählt oder nicht) und wählt sie; ein Strich, der auf leerer Fläche
beginnt, malt in die gewählte Bitmap oder legt auf der aktuellen Ebene eine neue an. Halten Sie beim
Drücken von Pinsel oder Buntstift über einer Bitmap **Strg** (macOS: Cmd), entsteht stattdessen eine neue
Bitmap darüber (Firepixel-Erweiterung; der Zeiger zeigt ein „+“). Eine gesperrte
oder ausgeblendete Ebene wird übersprungen (es gilt die nächste sichtbare, nicht gesperrte Ebene
darunter, sonst darüber); gibt es keine, meldet die Statusleiste, dass keine Ebene bemalbar ist. **Radiergummi**, **Weichzeichner** und **Stempel** arbeiten mit
weichem Rand wie im Original; Esc beendet das Malen und wechselt zum Zeiger. Nach einer Auswahl mit
Auswahlrahmen, Lasso oder Zauberstab auf einer Bitmap beendet Esc den Bitmapmodus (Verlaufszeile
„Bitmapmodus beenden“), hebt die Pixelauswahl auf und wechselt zum Zeiger; die Bitmap bleibt gewählt. Esc
während Sie einen neuen Auswahlrahmen oder ein Freihand-Lasso noch aufziehen, bricht das Aufziehen ab,
beendet ebenfalls den Bitmapmodus (auch eine ältere Pixelauswahl ist weg) und wechselt zum Zeiger; das
Loslassen bewirkt danach nichts. Esc mitten in einem Pinselstrich wechselt sofort zum Zeiger und lässt
die Vorschau stehen, beim Loslassen wird der Strich trotzdem ganz gemalt; das nächste Esc beendet den
Bitmapmodus. Mit einer Pixelauswahl (Zeiger, Auswahlrahmen oder Lasso) verschieben die Pfeiltasten nur den
Auswahlrahmen, nicht die Pixel (1 px, mit Umschalt 10 px); aufeinanderfolgende Tasten sind eine Verlaufszeile
„Verschieben“. Solange eine Pixelauswahl besteht, zeigen Auswahlrahmen, Lasso und Zauberstab am
Mauszeiger die Verknüpfung der nächsten Auswahl: Umschalt „+“ (hinzufügen), Alt „−“ (abziehen),
Umschalt+Alt „×“ (Schnittmenge, in beiden Reihenfolgen gedrückt); ohne Pixelauswahl trägt der Zeiger kein
Zeichen. In der Pixelauswahl wird der Zeiger zum Pfeil mit gestricheltem Rechteck – Ziehen verschiebt den
Auswahlrahmen (auch mit Lasso und Zauberstab) – und mit Strg zum Pfeil mit Schere (Ziehen verschiebt die
Pixel). Ziehen in einer Pixelauswahl mit dem Zeiger oder mit Strg und Auswahlrahmen, Lasso oder
Zauberstab zeigt nur die gewählten Pixel live in Bewegung (zurück bleibt ein transparentes Loch); mit
dem Unterauswahl-Werkzeug zieht eine Kopie und das Original bleibt (Verlaufszeile „Duplizieren“). Der
Zeiger folgt einer geänderten Umschalttaste sofort, ohne Mausbewegung (Firepixel; das Original wartet
auf die nächste Bewegung).

**Bitmapmodus:** Wählen eines Bitmap-Werkzeugs bei gewählter Bitmap, ein Bitmap-Strich oder ein
Doppelklick mit dem Zeiger auf eine Bitmap beginnt den Bitmapmodus (Verlaufszeile „Bitmap bearbeiten“);
die Objektgriffe der Bitmap verschwinden. Der kleine runde Knopf rechts neben den Status-Steuerelementen
in der Statusleiste wird dann rot; ein Klick darauf (oder der Zeiger, oder Esc) beendet den Bitmapmodus
(Zeile „Bitmapmodus beenden“). Außerhalb des Bitmapmodus ist der Knopf grau. Rückgängig von „Bitmap
bearbeiten“ beendet den Bitmapmodus, Wiederholen beginnt ihn erneut, und Rückgängig von „Bitmapmodus beenden“
schaltet ihn wieder ein; Rückgängig eines Strichs lässt ihn an. Die Wahl von **Pinsel** oder **Bleistift**
schreibt eine Verlaufszeile, „Pinselstrich“, wenn der Strich des Werkzeugs vom aktuellen Strich abweicht, sonst
„Strichfarbe“ (nie zweimal „Strichfarbe“ hintereinander); Bearbeiten ▸ Rückgängig und Wiederholen überspringen
sie zusammen mit dem Schritt davor. Ein direkter Wechsel zwischen Pinsel und Bleistift behält den aktuellen Strich
(Zeile „Strichfarbe“); nach einem anderen Werkzeug nimmt jedes den Strich, den es zuletzt hatte. Ein Strich, der eine
neue Bitmap anlegt, schreibt vor „Bitmap bearbeiten“ und der Strichzeile die Zeile „Leere Bitmap einfügen“. Ein gewähltes Vektorobjekt wird abgewählt und bleibt unverändert. Das Werkzeug
**Verlauf** hat eigene Farben (Vorgabe Schwarz → Weiß, Klick aufs Farbfeld öffnet den
Verlaufseditor), „Auswahl füllen“ und Toleranz gelten getrennt vom Farbeimer. **Farbe ersetzen**
hat eigene Farben „Von“ und „in“ (Vorgabe Grün → Rot).
Die **Pipette** übernimmt die Farbe unter dem Zeiger in das aktive Farbfeld des Werkzeugbedienfelds (Alt:
das andere); gewählte Vektorobjekte bekommen die Farbe ebenfalls. Die Option „Auswahl:“ im Inspektor liest
1 Pixel oder den Durchschnitt aus 3 × 3 bzw. 5 × 5. Auf der Leinwand tastet sie das Dokument ab; auf der
Leinwand drücken, die Maustaste halten und irgendwo auf dem Bildschirm außerhalb des Firepixel-Fensters
loslassen übernimmt die Bildschirmfarbe dort (auch aus anderen Programmen, mit derselben Auswahl).
Neben der Leinwand innerhalb des Fensters losgelassen, bleibt die Farbe. Grenzen: Unter Linux wird der
Bildschirm nur unter X11 gelesen (unter Wayland bleibt die Farbe); unter macOS fragt das System nach der
Berechtigung „Bildschirmaufnahme“, ohne sie wird nur der Schreibtischhintergrund gelesen.
Beim Malen mit Pinsel, Bleistift, Radiergummi oder Stempel erscheint die Vorschau an der Stelle der
Bitmap in der Stapelreihenfolge, mit ihrer Deckkraft, Füllmethode, ihren Effekten und Masken. Ein
Strich, der eine neue Bitmap anlegt oder in eine Bitmap in einem Symbol malt, wird bis zum Loslassen
obenauf angezeigt.
**Weichzeichner**, **Scharfzeichner**, **Abwedler**, **Nachbelichter**, **Verwischen**, **Stempel** und
**Farbe ersetzen** zeigen ihre Wirkung ebenfalls schon beim Ziehen; das Loslassen ändert nichts mehr,
jeder Strich ist ein Verlaufsschritt mit dem Namen des Werkzeugs. Über einer bearbeitbaren Bitmap
blenden diese Werkzeuge den Mauszeiger aus und zeigen nur den Umriss der Pinselspitze (er verschwindet,
sobald der Zeiger die Leinwand verlässt); wo keine Bitmap bearbeitet werden kann, zeigt der Zeiger
„nicht erlaubt“. Der Spitzenumriss von Pinsel, Radiergummi und diesen Werkzeugen hat keine eigene Farbe:
Er kehrt um, was darunter auf dem Bildschirm steht (jedes Pixel XOR #FFFF00, also blau über Weiß, gelb
über Schwarz und grün über Rot), ist bei jedem Zoom 1 Bildschirmpixel breit und so groß wie die Spitze
beim aktuellen Zoom; er sieht unabhängig von der Werkzeugfarbe gleich aus, auch beim Bearbeiten einer
Maske. Beim Malen kehrt er auch den bisher gemalten Strich um (grün über einem frischen roten Strich).
Eine Spitze von höchstens 4 Bildschirmpixeln bekommt zusätzlich ein kleines invertierendes Fadenkreuz:
vier kurze Striche 4 bis 5 Pixel über, unter, links und rechts vom Zeiger. Der Bleistift zeigt keinen
Umriss. **Verwischen** wird nie am Bitmaprand abgeschnitten, gleich ob die Pinselmitte beim
Drücken auf der Bitmap oder daneben lag (bei gewählter Bitmap wirkt ein daneben begonnener Strich wie
ein auf ihr begonnener; ist nichts gewählt, bewirkt ein Druck neben der Bitmap nichts): Über den Rand
getragene Farbe läuft halbtransparent aus, und die Bitmap wächst mit; nur hineingezogene Transparenz
vergrößert nichts. Mit „Gesamtes Dokument“ wächst die Bitmap auf das ganze Rechteck des Strichs
(Spitze eingeschlossen), gefüllt mit der Seite, auch bei einem Strich ganz außerhalb der Bitmap. Ein
Strich ganz außerhalb ohne „Gesamtes Dokument“ ändert kein Pixel, ist aber trotzdem ein
Verlaufsschritt. Die Firepixel-Option **Auf Grenzen beschränken** im Inspektor von Verwischen hält das
Verwischen innerhalb des Bitmaprands. **Weichzeichner** über den Rand vergrößert
die Bitmap um bis zu 2 px. Der **Stempel** legt die geklonten Pixel auch außerhalb der gewählten Bitmap
ab, die darauf wächst (auch auf einen abgesetzten Streifen, die Lücke bleibt transparent); ein
**Pinsel**-Strich, der bis etwa 80 px neben der gewählten Bitmap beginnt, vergrößert sie ebenfalls
(weiter entfernt beginnt er eine neue Bitmap). **Scharfzeichner**, **Abwedler** und **Nachbelichter**
wirken nur innerhalb der Bitmap. Bei allen darf die Pinselmitte beim Drücken außerhalb der Bitmap
liegen, solange die Bitmap gewählt ist.

**Auf Grenzen beschränken (Firepixel-Erweiterung):** Verwischen, Weichzeichner, Stempel, Pinsel und
Bleistift haben das Kästchen **Auf Grenzen beschränken** in der rechten Spalte des
Eigenschaften-Inspektors (Vorgabe aus; jedes Werkzeug merkt sich seine Einstellung). Ist es an, ändert
das Werkzeug kein Pixel außerhalb der Bitmap und nimmt von dort nichts auf: Die Bitmap wächst nie
(Weichzeichner: kein Rand von 2 px und kein transparentes Weiß in der Kante). Scharfzeichner, Abwedler
und Nachbelichter wirken ohnehin nur innerhalb der Bitmap und haben keine solche Option; das
Referenzprogramm hat sie bei keinem Werkzeug.

**Strich anhalten (Firepixel-Erweiterung):** Während Sie mit Pinsel, Bleistift, Radiergummi oder einem
der Retusche-Werkzeuge (Weichzeichner, Scharfzeichner, Abwedler, Nachbelichter, Verwischen, Stempel,
Farbe ersetzen) einen Strich malen, steht in der Statuszeile „P auf der Tastatur hält den aktuellen
Pinselstrich an.“ Drücken Sie **P**, solange die Maustaste noch gedrückt ist (bzw. der Stift aufliegt):
Der Strich ist angehalten, Sie können die Maustaste loslassen oder den Stift abheben, frei bewegen (nur
der Pinselumriss folgt dem Zeiger, gemalt wird nichts) und zoomen oder scrollen. Der nächste Klick auf
die Leinwand setzt **denselben** Strich an dieser Stelle fort (die Teile werden nicht durch eine Linie
verbunden); **P** hält erneut an. Weil es ein Strich bleibt, wird ein halbtransparenter Pinsel dort, wo
sich die Teile überlappen, nicht dunkler – anders als bei zwei getrennten Strichen (der Radiergummi
radiert die Überlappung einmal, Abwedler/Nachbelichter/Weich- und Scharfzeichner bearbeiten sie
einmal; Verwischen und Stempel beginnen bei jedem Klick neu). **Enter** wendet den angehaltenen Strich
als eine Verlaufszeile mit dem Namen des Werkzeugs an. Auch **Esc**, ein Werkzeugwechsel oder ein
Menübefehl wenden den angehaltenen Strich zuerst an, es geht also nichts verloren. **Rückgängig**
verwirft den angehaltenen Strich ohne Verlaufszeile, **Wiederholen** wendet ihn wie Enter an. Bei
gedrückter Maustaste werden Rückgängig und Wiederholen ignoriert (wie im Original). Mit gehaltener
**Alt**-Taste während eines Pinsel- oder Bleistiftstrichs (laufend oder angehalten) wird abgezogen: Die
Spitze entfernt nur, was dieser Strich gemalt hat, die Pixel darunter bleiben; nach dem Loslassen von
Alt wird wieder gemalt (Firepixel-Erweiterung). Umschalt mitten im Strich zieht eine gerade Linie vom
erreichten Punkt; was vorher gemalt wurde, bleibt. Ohne laufenden Strich bleibt P das
Werkzeugkürzel. Skripte und MCP senden die Taste `pause` an `tool_event`; die Befehle `bitmap.paint`,
`bitmap.erase` und `bitmap.retouch` erhalten die fortgesetzten Teile als `breaks` (Indizes in
`points`).

**Stift:** Beim Zeichnen eines Pfads zieht ein Klick auf seinen letzten Ankerpunkt die ausgehende Tangente dieses
Punkts ein, sodass das nächste Segment ihn gerade verlässt; Ziehen auf dem Punkt zieht ihn wieder als symmetrischen
Kurvenpunkt auf (Alt bewegt nur die ausgehende Tangente, Strg verschiebt den Punkt, Umschalt rastet auf 45° ein).
Ein Klick auf einen inneren Kurvenpunkt macht ihn zum Eckpunkt, ein Klick auf einen inneren Eckpunkt löscht ihn,
ein Klick auf ein Segment fügt einen Punkt ein, und ein Klick auf das Ende eines anderen offenen Pfads verbindet
ihn; ohne laufenden Pfad setzt ein Klick auf das Ende eines offenen Pfads diesen fort. Der Mauszeiger zeigt den
Modus: ein „ד ohne laufenden Pfad (ein Klick beginnt einen neuen Pfad), der Stift ohne Zusatz beim Zeichnen, ein
„o“ über dem ersten Punkt (schließen), ein „^“ zum Umwandeln, ein „−“ zum Löschen und ein „+“ zum Einfügen eines
Punkts (das Gummiband ist dann ausgeblendet), ein „/“ über dem Ende eines nicht gewählten offenen Pfads (fortsetzen)
und ein Ring mit Strich zum Verbinden. Beim Zeichnen zeigt der Pfad seinen Umriss und ein Quadrat auf jedem
Ankerpunkt (der zuletzt gesetzte gefüllt), keinen Begrenzungsrahmen und keine Füllung, bis er geschlossen ist; nach
dem Schließen ist der erste Ankerpunkt gefüllt. Wird der Stift bei einem gewählten offenen Pfad gewählt, geht das
Zeichnen an dessen Endpunkt weiter; ein gewählter geschlossener Pfad zeigt seinen Umriss mit hohlen Ankerpunkten.
Punkte werden innerhalb eines Quadrats aus dem größeren Wert von *Ausrichtungsabstand* und *Markierungsabstand*
aus den Voreinstellungen getroffen (ab Werk je 5 Bildschirmpixel; ein Klick 5 px rechts und 5 px unter dem
Anfangspunkt schließt also noch), auch bei nicht gewählten Pfaden. Ein Doppelklick (er setzt seinen Punkt, danach
ist kein Ankerpunkt mehr gewählt) oder ein Werkzeugwechsel beendet den Pfad; ein Klick auf den Anfangspunkt
schließt ihn, und der nächste Klick beginnt einen neuen Pfad; die Eingabetaste und Esc bewirken beim Zeichnen
nichts (der letzte Ankerpunkt bleibt gewählt, der nächste Klick setzt denselben Pfad fort). Stiftpunkte rasten
innerhalb des Ausrichtungsabstands an Hilfslinien und am Raster ein, nicht an intelligenten Hilfslinien und nicht
am Seitenrand. Jeder Punkt ist ein eigener Schritt „Stift“ im Verlauf: Strg+Z (oder die Rücktaste) entfernt beim Zeichnen nur
den letzten Punkt, und Sie zeichnen vom vorigen Punkt aus weiter; das Beenden des Pfads erzeugt keinen Schritt.

**Hintere auswählen** (im Platz des Zeigers): Ruht der Mauszeiger über Objekten, umrahmt ein roter Rahmen das
Objekt, das ein Klick wählen würde – das nächste Objekt unter dem gewählten unter dem Mauszeiger, auch wenn der
Mauszeiger über dem gewählten Objekt steht, oder das oberste Objekt, wenn dort nichts gewählt ist. Der Zeiger
umrahmt nur das oberste Objekt und zeigt über dem gewählten keinen Rahmen.

**Zeiger:** Ein einzelner gewählter Pfad zeigt ebenfalls seinen Umriss mit gefüllten Quadraten auf den Ankerpunkten
(ein Rechteck: seine vier Ecken) statt eines Begrenzungsrahmens; Gruppen, Bitmaps, Text, Symbole und mehrere gewählte
Objekte behalten den Rahmen.

**Unterauswahl:** Ein gewählter Pfad, ein Rechteck oder ein Teil einer Autoform zeigt statt eines Begrenzungsrahmens
seinen Umriss, ein kleines Quadrat auf jedem Punkt (gefüllt, wenn der Punkt gewählt ist; *Einfarbige Punkte anzeigen*
in den Voreinstellungen kehrt das um) und die Griffe der gewählten Punkte; Bitmaps, Text und Symbole behalten den
Rahmen. Punkte und Griffe zieht die Teilauswahl frei (nicht auf ganze Pixel gerundet); sie rasten nur an Raster,
Hilfslinien und intelligenten Hilfslinien ein, wenn diese eingeschaltet sind.

**Strich ab Werk:** Linie, Stift, Vektorpfad, Rechteck, Ellipse und Polygon zeichnen wie im Original
mit der Strichart *Einfach ▸ Präzise abger.* (1 px); eine eigene Wahl im Inspektor gilt weiter.

## Menüs

- **Datei:** Neu, Öffnen, Zuletzt geöffnete Dateien, Speichern, Importieren, Exportieren,
  Bildvorschau, HTML-Einrichtung, Batch-Verarbeitung, Drucken.
- **Bearbeiten:** Rückgängig, Ausschneiden/Kopieren/Einfügen. Kopierte Objekte stehen auch anderen
  Programmen als Bild zur Verfügung, ein Bild aus einem anderen Programm wird als Bitmap eingefügt.
  Dazu Duplizieren, Klonen, Objekt einfügen, Voreinstellungen und Tastaturbefehle.
- **Ansicht:** Vergrößerung, Einpassen, Lineale, Raster, Hilfslinien, intelligente Hilfslinien,
  Design, Sprache. *Ansicht ▸ An Pixeln einrasten* (ein eigener Schalter von Firepixel, ab Werk aus) rundet Anker
  und Griffe, die mit der Teilauswahl gezogen werden, auf ganze Pixel; ausgeschaltet bewegen sie sich frei in
  Schritten von 1/Zoom wie im Original. Verschiebungen mit dem Zeiger und die Punkte des Stifts liegen immer auf
  ganzen Pixeln. Gezogene Anker rasten nicht an intelligenten Hilfslinien ein. Verschiebt der Zeiger ein Objekt,
  rasten intelligente Hilfslinien seine Kanten (auch an der Gegenkante) und seine Mitte innerhalb von
  5 Bildschirmpixeln an den Begrenzungsrahmen anderer Objekte ein; die Hilfslinie ist eine gestrichelte
  magentafarbene Linie (3 px Strich, 3 px Lücke), die mindestens 40 px über beide Objekte hinausreicht.
  *Modifizieren ▸ An Pixeln
  ausrichten* (Strg+K) richtet die Auswahl weiterhin einmalig aus.
  *Ansicht ▸ Live-Vorschau beim Ziehen von Punkten* (ein eigener Schalter von Firepixel, ab Werk aus) zeichnet das
  Objekt beim Ziehen von Ankern oder Griffen live mit Füllung, Strich und Effekten statt nur seines Umrisses.
  *Ansicht ▸ Zeiger bearbeitet Punkte* (ein eigener Schalter von Firepixel, ab Werk aus) lässt den Zeiger die Anker
  ausgewählter Pfade bearbeiten: Ein Klick wählt einen Anker (Strg- oder Umschalt-Klick fügt einen hinzu oder
  entfernt ihn), Ziehen verschiebt nur die ausgewählten Punkte, Entf löscht sie, die Pfeiltasten verschieben sie;
  ein Rechteck wird beim ersten Punktziehen zum gewöhnlichen Pfad. Ausgeschaltet arbeitet der Zeiger wie im
  Original: Der Eckgriff skaliert das Rechteck, und das Ziehen eines Punkts verschiebt das ganze Objekt.
  *Ansicht ▸ Live-Schriftvorschau* (ein eigener Schalter von Firepixel, ab Werk an) zeigt die in einer offenen
  Schriftliste markierte Schrift sofort am ausgewählten Text, siehe Schriftsuche.
- **Auswählen, Modifizieren, Text, Filter:** wirken auf die Auswahl. Ist ein Befehl grau, nennt die
  Statuszeile beim Ausführen über das Kürzel den Grund. Befehle, die immer anwählbar sind, aber
  kein passendes Ziel finden (z. B. *Vereinfachen…* ohne Pfad), melden das wie im Original mit einer
  Warnung; mit „Nicht mehr anzeigen“ lässt sie sich abschalten.
- *Modifizieren ▸ Transformieren ▸ Numerisch transformieren…:* Mit „Attribute skalieren“ wächst die
  Strichbreite beim Skalieren mit (Drehen ändert sie nie), und die Füllgriffe (Verlauf und Muster)
  werden mit dem Objekt transformiert; ohne den Haken bleiben die Füllgriffe an ihrer Stelle im
  Dokument. Der Haken wird gemerkt (Voreinstellung `NumericXformXformsBFE`, auch nach „Abbrechen“).
  Die Werkzeuge Skalieren, Neigen und Verzerren sowie Frei transformieren richten sich bei den
  Füllgriffen ebenso nach *Voreinstellungen ▸ Allgemein ▸ Pinselstriche und Effekte skalieren*
  (`XformToolXformsBFE`, Vorgabe ein). Reines Verschieben nimmt die Füllgriffe immer mit. In den
  Werkzeugen Skalieren, Neigen und Verzerren sowie bei Frei transformieren übernehmen die
  Eingabetaste, ein Doppelklick oder ein anderes Werkzeug alle Züge der Sitzung als eine
  Verlaufszeile „Transformieren“; Esc bricht die Sitzung ab: Das Objekt kehrt in den Zustand vor der
  Sitzung zurück, der Zeiger ist aktiv, und die Zeile bleibt als Wiederholen-Schritt (*Bearbeiten ▸
  Wiederholen: Transformieren* wendet sie erneut an). Esc bei noch gedrückter Maustaste lässt die
  Vorschau bis zum Loslassen stehen und beendet dann den Modus, ohne etwas anzuwenden. In
  Skripten transformieren `scaleSelection`, `rotateSelection`, `setQuadrangle` und
  `transformSelection` die Füllgriffe (und beim Skalieren die Strichbreite) nur, wenn die
  Optionszeichenfolge `transformAttributes` enthält, wie im Original
  (`"autoTrimImages transformAttributes"`); mit `"autoTrimImages"` allein bleiben sie im Dokument. Bei einem Text
  skaliert Numerisch transformieren die Füllgriffe immer um die Mitte, und die Transformationswerkzeuge lassen sie
  immer stehen, wie im Original.
- **Text skalieren ändert die Schriftgröße.** Wie im Original wird eine reine Skalierung (ohne Drehung,
  Neigung oder Spiegelung) eines Texts in seine Attribute umgerechnet statt als Matrix gespeichert:
  Schriftgröße × senkrechter Faktor (auf 0,01 pt, mindestens 4 pt), horizontale Skalierung ×
  waagerechter/senkrechter Faktor, exakter Zeilenabstand, Laufweite, Einzug und Absatzabstände mit;
  der Grundlinienversatz bleibt. Der Text wird mit der neuen Größe neu gesetzt (scharf, pixelgleich mit
  einem frisch gesetzten Text dieser Größe); eine kleine Rest-Matrix passt ihn auf den skalierten Rahmen
  ein, bei weniger als einem Bruchteil eines Pixels Abweichung gar keine (1,5 × eines 60-pt-Texts ergibt
  einen einfachen 90-pt-Text). Das gilt für das Skalieren über *Modifizieren ▸ Transformieren ▸
  Numerisch transformieren* (jeder gewählte Text, auch in Gruppen) und für das Skalieren-Werkzeug, Frei
  transformieren und *Modifizieren ▸ Transformieren ▸ Skalieren*, wenn genau ein Text gewählt ist.
  Drehen, Spiegeln, Neigen, Verzerren, das Skalieren-Werkzeug auf einer Gruppe oder mehreren Objekten
  und ungleichmäßiges Skalieren eines gedrehten Texts behalten die Matrix. Ein Text mit fester Breite
  behält seine Rahmenbreite und bricht neu um; die Rest-Matrix streckt bzw. staucht ihn dann sichtbar,
  wie im Original. *Transformationen entfernen* behält die neue Größe. In Skripten rechnet
  `scaleSelection` wie der Dialog um; `setQuadrangle` nimmt seine vier Punkte relativ zu einem
  Einheitsquadrat (Quadrat der Seitenlänge max(Breite, Höhe) um die Mitte der Auswahlgrenzen, ein
  Rechteck 100 × 60 ist also x 0…1, y 0,2…0,8) und rechnet einen einzelnen Text nur mit der Option
  `rememberQuad` um.
- **Der Transformationsrahmen eines Texts.** Ist genau ein Text gewählt, zeigen die Werkzeuge
  Skalieren, Neigen und Verzerren, Frei transformieren und *Modifizieren ▸ Transformieren ▸ Skalieren*
  einen Rahmen 1,3-mal so groß wie die Satzbox des Texts (links und rechts je 15 % der Breite, oben und
  unten je 15 % der Höhe dazu), mit dem Text gedreht; beim Punkttext ist die Satzbox sein Satzrahmen
  (ein „Ag“ mit 60 pt, 74 × 72 bei 100,100, bekommt die Griffe bei 89/137/185 × 89/136/183), beim Text
  mit fester Breite die Box um die gesetzten Zeilen. Pfade, Gruppen (auch mit Text darin) und mehrere
  Objekte behalten ihre Grenzen. Die Ecken stehen auf ganzen Pixeln, Rahmenlinie und Griffe einen
  Bildschirmpixel links oberhalb des Rahmens, wie im Original. Ein Eckzug skaliert mit der Lage des
  Zeigers projiziert auf die Diagonale des Rahmens (+39/+38 an der Ecke dieses „Ag“ ergibt 83,74 pt); der
  nächste Zug beginnt am Rahmen des neuen Zustands. *Transformationen entfernen* legt links/oben eines
  Texts auf die gerundete linke/obere Kante des Umrisses seines Rahmens (aus Messungen abgeleitet, nicht
  jeder Fall ist bekannt).
- *Modifizieren ▸ Maske ▸ Alles einblenden / Alles ausblenden* (und *Maske hinzufügen* im
  Ebenen-Bedienfeld) legt eine leere Graustufen-Bitmapmaske in Leinwandgröße über das gewählte Objekt
  (größer, wenn das Objekt über die Leinwand hinausreicht) und geht wie im Original sofort in die
  Bearbeitung der neuen Maske, wie ein Klick auf ihre Miniatur: Die Maske ist gewählt und in der
  Bitmap-Bearbeitung, ihre Miniatur bekommt den hellgrünen Rahmen, die Zeile des Objekts bleibt
  hervorgehoben, und der Inspektor zeigt *Bitmap-Maske* mit *Alphakanal* / *Graustufen*. Die
  Verlaufszeile heißt *Maske hinzufügen*. Skripte sehen während der Maskenbearbeitung das maskierte
  Objekt in `fw.selection` und die Maske in `fw.selectedMask`.
- **Arbeiten mit Masken:** Ein Klick mit der *Teilauswahl* auf ein maskiertes Objekt wählt dessen
  **Inhalt** (auch einen einzelnen maskierten Pfad oder eine Bitmap), der sich dann verschieben,
  skalieren, punktweise bearbeiten oder bemalen lässt, während die Maske bleibt (Firepixel-Erweiterung).
  Die **Maskenelement-Auswahl** im Flyout der Teilauswahl (A / 1 schaltet zwischen beiden um; Symbol und
  Zeiger zeigen den Teilauswahl-Pfeil mit einem Maskenzeichen) wählt stattdessen die **Maske** des
  angeklickten maskierten Objekts, ebenso ein Klick mit dem *Zeiger* bei gedrückter Alt-Taste (auf einer normalen Gruppe wählt Zeiger +
  Alt-Klick das getroffene Kind; Alt-Ziehen dupliziert weiterhin); *Hintere auswählen* wählt nie eine
  Maske. Ein Klick auf die Maskenminiatur im Ebenen-Bedienfeld wirkt wie *Maske bearbeiten* im
  Kontextmenü: Die Maske wird gewählt, ihre Miniatur bekommt einen hellgrünen Rahmen, die Zeile bleibt
  hervorgehoben und zeigt ein Maskensymbol in der zweiten Spalte; eine Bitmapmaske geht in die
  Bitmap-Bearbeitung. Ein Klick auf den Namen der Zeile wählt wieder das Objekt und rahmt dessen eigene
  Miniatur blau (die Miniatur eines gewählten Objekts ist immer blau gerahmt). Die Ankerpunkte
  einer Maske aus einem einzelnen Pfad lassen sich mit der Maskenelement-Auswahl ziehen, der Inhalt bleibt
  stehen; auf gewöhnlichen Objekten wirkt dieses Werkzeug wie die Teilauswahl.
  *Maske deaktivieren* schaltet die Maske ab; der Menüeintrag heißt dann *Maske aktivieren*, und die
  Maskenminiatur zeigt ein rotes X. *Maske löschen* fragt zuerst: *Anwenden* (Vorgabe) backt die Maske in
  das Objekt, das dabei zu einer Bitmap wird; *Verwerfen* entfernt die Maske und behält das Objekt;
  *Abbrechen* tut nichts. *Gruppierung aufheben* ergibt das Maskenobjekt über dem Inhalt, beide gewählt.
  Firepixel-Erweiterungen über das Original hinaus: Ist eine **Gruppenmaske** gewählt, wählt ein weiterer
  Klick mit der Maskenelement-Auswahl oder der Teilauswahl (oder ein Doppelklick mit dem Zeiger) das Kind unter dem Zeiger, sodass sich
  seine Punkte einzeln bearbeiten lassen und die Maske folgt; der Masken-Inspektor hat ein Kästchen
  **Invertieren** (Vektor- und Bitmapmasken; bei einer Vektormaske rechts in der oberen Zeile direkt über den
  Zeilen *Maske:*, bei einer Bitmapmaske rechts neben *Alphakanal*), das verbirgt, was innerhalb der Maske liegt, und alles
  außerhalb zeigt (die eigene Deckkraft des Maskenobjekts bzw. ein Grauwert im Graustufenmodus bestimmt,
  wie stark). Invertierte Masken stehen in einem Firepixel-eigenen Tag der FW-PNG-Datei; andere Programme
  zeigen die normale Maske.
- **Maskenmodi.** *Als Maske gruppieren* legt immer eine Graustufenmaske an, *Füllung und Pinselstrich
  anzeigen* aus; das maskierte Element bleibt gewählt (Verlaufszeile *Gruppieren*). *Als Maske einfügen* gibt
  bei einer Vektormaske den Modus *Pfadumriss*, bei einer Bitmapmaske *Graustufen* und wählt die Maske;
  *Innen einfügen* nimmt *Pfadumriss* mit *Füllung und Pinselstrich anzeigen* an, die Füllung (bzw. Bitmap)
  der Maske liegt also unter dem Inhalt, bis das Kästchen aus ist (beide Zeilen *Einfügen*). Bei einer Bitmapmaske heißt *Pfadumriss* *Alphakanal*:
  *Innen einfügen* auf eine Bitmap gibt eine Bitmapmaske im Modus *Alphakanal*, deren eigene Transparenz den
  Inhalt beschneidet; *Maske hinzufügen*, *Alles einblenden* / *Alles ausblenden*, *Als Maske einfügen* einer
  Bitmap und *Als Maske gruppieren* geben *Graustufen*. In Graustufen ist
  der Maskenwert die Luma (Rec. 709) des gerenderten Maskenaussehens mal Alpha: Weiß zeigt, Schwarz blendet aus,
  Rot ergibt 21 %, Grün 72 %, Blau 7 %; Verläufe und weiche Kanten laufen aus. **Was in welchem Modus zählt**
  (wie im Original): Im *Pfadumriss* ohne *Füllung und Pinselstrich anzeigen* schneidet nur die Form des Pfads
  den Inhalt zu, immer mit glatter (kantengeglätteter) Kante, auch wenn die Füllkante *Hart* ist, und weich bei
  einer *Weichen Kante*; Füllfarbe und ihr Alpha, Verläufe, Muster, Dither, der Strich, Live-Effekte sowie
  Deckkraft und Mischmodus des Maskenobjekts ändern nichts, und der Strich vergrößert die Maske nicht. Mit
  *Füllung und Pinselstrich anzeigen* liegt die Füllung der Maske (mit Kante, weicher Kante, Farbe, Verlauf oder
  Muster) unter dem Inhalt, der Inhalt wird scharf am Pfad abgeschnitten (ohne weiche Kante), und der Strich der
  Maske wird über den Inhalt gezeichnet (Breite, Weichheit, Farbe und Alpha wirken); Live-Effekte, Deckkraft und
  Mischmodus des Maskenobjekts bleiben ohne Wirkung. In *Graustufen* zählt das ganze gerenderte Aussehen: Kante
  (auch *Hart*), weiche Kante, Farbalpha, Verlauf (mit Dither), Muster, der Strich (ein weißer Strich erweitert
  die Maske, ein schwarzer blendet den Rand aus) und Live-Effekte; nur Deckkraft und Mischmodus des
  Maskenobjekts werden ignoriert. Ist eine Maske gewählt
  (Maskenminiatur im Ebenen-Bedienfeld oder *Maske bearbeiten*), richtet sich der Inspektor nach der Art des
  Maskenobjekts, nicht nach dem Modus: Pfad, Text oder Gruppe (auch aus Bitmaps) zeigt **Vektormaske** mit
  *Pfadumriss* / *Graustufendarstellung*, *Füllung und Pinselstrich anzeigen* (nur im Pfadumriss bedienbar)
  und allen Füll- und Strichfeldern in beiden Modi; eine Bitmap zeigt **Bitmap-Maske** mit *Alphakanal* /
  *Graustufen*. Der Rückweg geht immer; Maske wählen, Modus und Kästchen sind je eine Verlaufszeile *Maske
  modifizieren*. Die Farbfelder zeigen bei gewählter Maske Grau (die Luma).
- **Malen auf einer Bitmapmaske** (Pinsel, Bleistift, Radiergummi) wirkt in beiden Modi live: Weiß deckt auf,
  Schwarz blendet aus, das maskierte Element ändert sich während des Zugs, der Pinselumriss kehrt wie überall den Bildschirm um, jeder Zug
  ist eine Zeile *Pinsel*. Die Malwerkzeuge zeigen die Radios *Alphakanal* / *Graustufen* der Maske, die den
  Modus umschalten. Die Unterlage *Füllung und Pinselstrich anzeigen* bleibt sichtbar, während eine
  Bitmapmaske bearbeitet und bemalt wird; das maskierte Objekt behält sein Aussehen. Im Ebenen-Bedienfeld zeigt die
  Objektminiatur eines maskierten Objekts das maskierte Ergebnis mit Unterlage, wenn *Füllung und Pinselstrich
  anzeigen* im Modus *Alphakanal* / *Pfadumriss* an ist, sonst den Inhalt allein, die Maskenminiatur die Maske grau
  auf Schwarz (eine Vektormaske mit Feder-Symbol), beide auf die Arbeitsfläche bezogen und nach jedem Zug
  aktualisiert, und die Maskenminiatur der bearbeiteten Maske bekommt einen 2 px breiten grünen Rahmen.
  Die Miniaturen sind 34 × 23 px groß mit 1 px dunkelgrauem Rahmen (Bildfläche 32 × 21, die Seite darin
  eingepasst, Objektzeilen 27 px hoch); die Inhaltsminiatur einer maskierten Bitmap trägt unten rechts ein
  kleines Bitmap-Symbol.
- **Die Unterlage einer Bitmapmaske.** Nach *Innen einfügen* auf eine Bitmap liegt die Masken-Bitmap
  zusätzlich unter dem Inhalt (*Füllung und Pinselstrich anzeigen*, Modus *Alphakanal*). Wie im Original
  hat der Inspektor einer Bitmapmaske nur *Alphakanal* / *Graustufen* (dazu Firepixels *Invertieren*) und
  kein Kontrollkästchen dafür; die Quickinfo von *Alphakanal* sagt das. Wer mit der Transparenz der Bitmap
  maskieren will, ohne Unterlage, nimmt *Als Maske gruppieren* oder *Als Maske einfügen* und wählt danach
  *Alphakanal*; auch der Wechsel einer vorhandenen Maske zu *Graustufen* nimmt sie weg (der Maskenwert folgt
  dann der Helligkeit); Skripte schalten sie mit `setElementMaskShowAttrs(false)` ab. Das Feld *Deckkraft*
  oben im Ebenen-Bedienfeld wirkt auf das gewählte Objekt: Bei gewählter Maske ist es die Deckkraft des
  Maskenobjekts selbst, die nur bestimmt, wie stark *Invertieren* ausblendet; Maskierung und Unterlage
  behalten ihre Stärke (das sagt seine Quickinfo). Bei gewähltem maskierten Objekt blenden die
  Deckkraft-Felder den Inhalt aus, die Unterlage scheint dann stärker durch.
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
- **Bildschirmmodi:** F (oder die drei Bildschirmmodus-Schaltflächen im Abschnitt Ansicht des Werkzeugbedienfelds)
  wechselt *Standard* → *Vollbildmodus mit Menüs* → *Vollbildmodus* → *Standard*; dafür muss ein Dokument
  offen sein. Der Vollbildmodus mit Menüs behält Menüleiste und Symbolleiste, das Werkzeugbedienfeld
  schwebt links, die angedockten Bedienfelder erscheinen rechts als Symbolspalte. Der Vollbildmodus zeigt nur die
  Leinwand, zentriert auf Schwarz, ohne Leisten, Bedienfelder, Register, Lineale und Bildlaufleisten.
  Zurück mit F oder *Vollbildmodus beenden* im Kontextmenü der Leinwand; wird das letzte Dokument
  geschlossen, kehrt Firepixel zu Standard zurück.
- *Ansicht ▸ Gamma 1.8* zeigt die Leinwand heller, wie auf einem Monitor mit Macintosh-Gamma. Dokument und
  Export bleiben unverändert.
- *Ansicht ▸ QuickInfos* (Vorgabe: ein, wird als Voreinstellung `Tooltips` gemerkt) schaltet die QuickInfos
  (Tooltips) von Knöpfen, Feldern und Bedienfeldern überall (Werkzeugbedienfeld, Symbolleiste, Inspektor,
  Bedienfelder, Dialoge). Aus entfallen nur die Beschreibungszeilen: die kurze erste Zeile mit Name und
  Kürzel bleibt, z. B. „Zeiger (V, 0)“, kurze einzeilige QuickInfos von Bedienfeldknöpfen und Inspektorfeldern
  (ein Name wie „Deckkraft“) bleiben unverändert. Eine einzeilige QuickInfo, die eine Beschreibung und kein
  Name ist (länger als 60 Zeichen oder ein Satz), entfällt ebenfalls; Bedienelemente mit längerer Erklärung
  zeigen zuerst eine eigene Namenszeile, z. B. „Dither-Dichte“ über der Erklärung des Dither-Dichte-Felds, und
  nur dieser Name bleibt. Um auch diese auszublenden, *Voreinstellungen ▸ Allgemein ▸ Werkzeugnamen ohne
  QuickInfos auch ausblenden* einschalten; *QuickInfo-Verzögerung (ms)* auf derselben
  Seite legt fest, wie lange der Mauszeiger ruhen muss, bis eine QuickInfo erscheint (Vorgabe 600,
  0–5000). Die Änderungen wirken sofort. Erklärende QuickInfos (z. B. am roten × im Verlauf, Gründe
  für gesperrte Knöpfe, Warnungen) erscheinen unabhängig von diesem Schalter.
- *Ansicht ▸ Koordinaten-Tag* (Firepixel-Erweiterung, direkt unter QuickInfos; Vorgabe: ein, wird als
  Voreinstellung `Firepixel_CoordinateTag` gemerkt) schaltet die kleine graue Marke dicht unter dem
  Mauszeiger über der Leinwand: mit Rechteck, Ellipse, Linie, Polygon und automatischen Formen die
  Zeigerposition („x:53 y:37“) und beim Aufziehen einer Form deren Größe („w:80 h:50“); beim Ziehen von
  Objekten mit dem Zeiger deren neue linke obere Ecke („x:130 y:120“). Der Zeiger zeigt beim bloßen
  Überfahren nichts, Stift, Text und die übrigen Werkzeuge zeigen keine Marke; nach dem Loslassen ist die
  Marke weg. Die beiden Schalter sind voneinander unabhängig.
- **Farben tauschen** (X oder der Knopf unter den Farbfeldern im Werkzeugbedienfeld): Ohne Auswahl tauschen
  nur Strich- und Füllfarbe. Mit Auswahl tauschen die Farben des obersten gewählten Objekts und gehen an
  alle gewählten Objekte, als eine Verlaufszeile „Strichfarbe“ („Pinselstrich und Füllen“, wenn eine Seite
  „keine“ ist: dann tauscht auch das „keine“, ein neuer Strich bekommt die Spitze „1-Pixel weich“, 1 px).
  Ein vorhandener Strich behält Spitze, Breite und Lage; ein Text verliert seine Füllung und bekommt einen
  Strich von 1 px in seiner Farbe; eine Bitmap behält ihre Pixel. Strg+Y wiederholt den Tausch, Strg+Z
  stellt Objekte und Farben zurück: Nach Rückgängig, Wiederholen oder einem Sprung im Verlauf folgen die
  Farben dem gewählten Objekt; nur ohne Auswahl (oder mit nur Bitmaps) kommen die mit dem Tausch
  gespeicherten Farben zurück.
- *Bearbeiten ▸ Einfügen* legt kopierte Objekte direkt über das oberste gewählte Objekt (in dessen Ebene;
  ist ein Objekt in einer Gruppe gewählt, über die Gruppe). Ohne Auswahl landen sie oben in der aktiven Ebene.
- *Modifizieren ▸ Maske ▸ Als Maske gruppieren* setzt die Maskengruppe an den Platz des untersten gewählten
  Objekts; *Gruppieren* bleibt am Platz des obersten. Vereinigen, Überschneiden und Verbinden legen das
  Ergebnis an den Platz des oberen Objekts, Öffnen und Zuschneiden an den des unteren (*Pfade kombinieren*).
- *Auswählen ▸ Auswahlrahmen in Pfad konvertieren* gibt dem neuen Pfad Füllung und Strich wie einer neu
  gezeichneten Form (Farben aus dem Werkzeugbedienfeld, Füll- und Strichart aus dem Inspektor).
- **Neue Formen übernehmen die Attribute des zuletzt gewählten Objekts:** Wird genau ein Pfad oder ein Text
  gewählt (Klick, Auswahlrahmen, Skript), werden seine Füllung, Füll- und Strichfarbe, Strichspitze, -breite
  und -lage zu den aktuellen Werkzeugattributen; ein Pfad mit Füllung „keine“ gibt dem Werkzeug die Füllung
  „keine“ mit seiner eigenen gespeicherten Füllfarbe, ein Text seine Farbe als Füllung, keinen Strich und die
  Lage außen. Gruppen, automatische Formen, mehrere gewählte Objekte, Bitmaps und Webobjekte ändern nichts;
  während mehrere Objekte oder eine automatische Form gewählt sind, zeigen die Farbfelder des
  Werkzeugbedienfelds nur die Farben des obersten Objekts. Die Attribute bleiben nach dem Abwählen erhalten:
  Das nächste Rechteck, die nächste Ellipse oder Linie sieht aus wie das zuletzt gewählte Objekt. Skripte sehen
  die Änderung erst nach ihrem Ende. Strich oder Füllung „keine“ zeigt das Farbfeld weiß mit roter Diagonale.
- Die Füllung „keine“ gehört zum Dokument: Ein neues oder geöffnetes Dokument beginnt mit Vollfüllung in der
  aktuellen Füllfarbe, Strich (auch „keine“), Spitze, Größe und Lage gelten weiter. Nach dem Programmstart ist
  der Werkzeugstrich die Spitze „1-Pixel weich“ in der Größe der Voreinstellung `BrushTipSize` (1 px, wenn keine
  gespeichert ist), mittig.
- **Standardfarben** (D oder der Knopf unter den Farbfeldern) setzt nur die Farben (Strich schwarz, Füllung
  weiß); Spitze, Größe und Lage bleiben. Ein Strich „keine“ wird „1-Pixel weich“ 1 px, eine Füllung „keine“
  einfarbig.
- **Automatische Formen** übernehmen die Werkzeugattribute vollständig: Füllung (auch „keine“), Strichfarbe,
  Spitze, Größe und Lage. Verbindungslinie und Spirale haben nie eine Füllung und nehmen den Werkzeugstrich;
  nur ohne Werkzeugstrich zeichnen sie eine eigene schwarze 1-px-Linie. Die Pfeillinie zeichnet immer
  schwarz „Präzise Linie“ 1 px ohne Füllung, das Messwerkzeug rot „Präzise Linie“ 1 px innen mit schwarzer
  Längenangabe; die Form des Messwerkzeugs ist danach nicht gewählt. Nur diese beiden ändern die
  Werkzeugattribute (Pfeillinie: schwarz „Präzise Linie“ 1 px, Messwerkzeug: rot, beide mit Füllung „keine“;
  die Lage bleibt). Jede Form ist eine Verlaufszeile „Form“; Rückgängig entfernt die Form, die
  Werkzeugattribute bleiben.
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

### Geteilte Ansicht (Fenster ▸ Fenster duplizieren)

*Fenster ▸ Fenster duplizieren* zeigt das aktive Dokument ein zweites Mal: Der Dokumentbereich teilt
sich in zwei Bereiche (zunächst nebeneinander). Jeder Bereich hat eigene Seite, eigenen Status, Zoom und
Bildlauf – so arbeiten Sie auf einer Seite und sehen dabei eine andere. Beide Bereiche zeigen
dasselbe Dokument; eine Änderung in einem Bereich erscheint sofort auch im anderen.

- Der neue rechte Bereich hat den Fokus und beginnt auf derselben Seite. Eine andere Seite wählen
  Sie in der Seitenliste oben im Bereich (oder im Bedienfeld Seiten).
- Der zuletzt angeklickte Bereich hat den Fokus (Akzentlinie an seiner Kopfleiste): Werkzeuge,
  Bedienfelder (Seiten, Ebenen, Status, Eigenschaften …), Lineale und Tastenkürzel wirken auf ihn.
  Ein Klick oder Mausrad über einem anderen Bereich gibt diesem zuerst den Fokus, ein Werkzeug
  arbeitet also immer im Bereich unter dem Zeiger.
- Statusleiste (Dokumentgröße, Zoom, Statusnummer, Werkzeughinweise) und Zeigerposition im
  Bedienfeld Info zeigen den Bereich, über dem der Zeiger steht; steht der Zeiger über keinem
  Bereich (etwa über der Statusleiste selbst oder einem Bedienfeld), zeigen sie den Bereich mit
  Fokus.
- Auch der Bereich ohne Fokus folgt dem Zeiger: Werkzeugzeiger, Hervorhebung beim Überfahren
  (auch der Rahmen von „Dahinter auswählen“ und Pfadpunkte), Pinselspitze und Koordinatenanzeige
  erscheinen in dem Bereich, über dem der Zeiger steht; erst ein Klick oder Mausrad gibt den Fokus
  weiter.
- *Fenster ▸ Fenster duplizieren* teilt den Bereich mit Fokus erneut: Ein neuer Bereich erscheint
  dahinter (rechts davon, bei Anordnung untereinander darunter) und erhält den Fokus. Bis zu vier
  Bereiche sind möglich, jeder mit eigener Seite, eigenem Status, Zoom und Bildlauf.
- *Fenster ▸ Nächster Bereich* (Umschalt+F6) gibt dem nächsten Bereich den Fokus, reihum durch alle
  Bereiche (Strg+Tab und Strg+F6 wechseln weiterhin zwischen den Dokumenten).
- *Fenster ▸ Teilung ▸ Nebeneinander / Untereinander* ordnet die Bereiche nebeneinander (Vorgabe)
  oder übereinander an; beide behalten Seite und Zoom.
- Mit den Stegen zwischen den Bereichen ändern Sie ihre Größe (jeder Bereich behält mindestens ein
  Zehntel der Fläche); Lage und Anordnung merkt sich Firepixel für das Dokument bis zum Beenden. Original/Vorschau/2-fach/4-fach gelten für den
  Bereich mit Fokus.
- Einen Bereich schließen Sie mit dem × in seiner Kopfleiste (der Fokus bleibt, wo er ist); den
  Bereich mit Fokus schließt *Fenster ▸ Zusätzliches Fenster schließen* (der Fokus geht an den
  Bereich davor). Seine Fläche übernimmt der Nachbar; bleibt ein Bereich übrig, ist der
  Dokumentbereich wieder ungeteilt. Die Teilung wird weder im Dokument noch im Arbeitsbereich
  gespeichert.
- Im Bereich ohne Fokus beginnt die Rückmeldung beim Überfahren mit einem ruhenden Werkzeug; ein im
  Bereich mit Fokus laufender Vorgang erscheint dort nicht.

## Bedienfelder

Die Bedienfelder sind rechts angedockt. Klick auf den aktiven Reiter klappt die Gruppe ein, einen
Reiter ziehen löst ihn oder verschiebt ihn in eine andere Gruppe. Das Menü ≡ jedes Bedienfelds
enthält dessen Befehle. Tab blendet alle Bedienfelder aus und wieder ein. Ist der Inhalt eines Bedienfelds
höher als sein Platz (etwa das Pfad-Bedienfeld in der angedockten Höhe), erscheint eine senkrechte
Bildlaufleiste, und das Mausrad rollt den Inhalt.

Arbeitsbereiche (Umschalter oben rechts):

- Erweiterter Modus,
- Symbolmodus,
- Symbolmodus mit Namen,
- eigene Layouts über *Aktuelles speichern…*.

Die eigenen Layouts liegen als XML unter `%APPDATA%\Firepixel\Workspace Layouts` und sind mit dem
Layoutformat des Originals austauschbar.

Wichtige Bedienfelder:

- **Ebenen:** Sichtbarkeit (Auge), Sperre (Schloss), Unterebenen; Ziehen ordnet um. Unterebenen
  stehen eingerückt als Ordnerzeile mit Aufklapper; Doppelklick auf die Zeile benennt sie um. Das
  Dreieck einer Ebene oder Unterebene klappt sie auf oder zu; dieser Zustand gehört zum Dokument (ein
  Schritt im Verlauf, in der Datei gespeichert), daher zeigt jedes Dokument und jede wieder geöffnete
  Datei ihre Ebenen so, wie sie gespeichert wurden. Ein
  Klick auf eine Objektzeile wählt nur dieses Objekt und setzt den Anker; Umschalt+Klick wählt alle
  Objekte vom Anker bis zur angeklickten Zeile in der Reihenfolge des Dokuments, von oben nach unten
  über Ebenen und Unterebenen hinweg (Objekte ausgeblendeter und zugeklappter Ebenen zählen mit,
  Objekte gesperrter Ebenen nicht), die Auswahl wird ersetzt; Strg+Klick (unter macOS Befehl+Klick)
  nimmt ein Objekt hinzu oder heraus und lässt den Anker stehen; Strg+Umschalt+Klick wirkt wie
  Umschalt+Klick. Ein Klick auf eine Zeile einer Mehrfachauswahl lässt die Auswahl stehen. Ein Klick
  auf eine Ebenenzeile wählt die eigenen Objekte der Ebene (ohne die ihrer Unterebenen) und macht sie
  zur aktuellen Ebene; Umschalt+Klick auf eine Ebenenzeile beendet den Bereich bei den eigenen
  Objekten der Ebene, Strg+Klick schaltet jedes davon einzeln um. Der Anker gilt, solange sein Objekt
  gewählt ist, auch nach einer Auswahl auf der Leinwand; sonst gilt das unterste gewählte Objekt, und
  ohne Auswahl wählt Umschalt+Klick von der Zeile bis zum untersten Objekt. Klicks auf Zeilen einer
  gesperrten Ebene (oder einer Unterebene davon) bewirken nichts. Auswählen im Bedienfeld schreibt
  keine Schritte in den Verlauf, ausgenommen der schlichte Klick auf eine Ebenenzeile: er schreibt
  einen Schritt *Ebenenänderung* (Rückgängig stellt aktuelle Ebene und Auswahl zurück). Besteht die
  Auswahl danach aus genau einem Objekt (Bedienfeld, Leinwand, Auswahlrahmen, Alles auswählen,
  Skript), wird dessen Ebene oder Unterebene die aktuelle Ebene; bei zwei und mehr Objekten bleibt die
  aktuelle Ebene. Aktuell ist stets die erste ungesperrte Ebene ab der gewählten aufwärts (Unterebenen
  teilen die Sperre ihrer Elternebene, die Webebene zählt mit), mit Umlauf zur untersten Ebene: Sperren
  der gewählten Ebene rückt die aktuelle nach oben, Entsperren holt sie zurück. Jede Seite merkt sich
  ihre gewählte Ebene (der erste Besuch nach dem Öffnen beginnt auf der untersten ungesperrten Ebene,
  eine neue Seite auf ihrer ersten Ebene); ist die aktuelle Ebene nach einem Statuswechsel gesperrt,
  wird die nächste ungesperrte darüber die gewählte. Eine Datei öffnet auf Seite 1 und Status 1 mit der untersten ungesperrten
  Ebene als aktueller Ebene und ohne Auswahl; das Bedienfeld rollt zu deren Zeile. Ohne Auswahl ist
  die Zeile der aktuellen Ebene hervorgehoben und ihr Name fett; mit Auswahl sind die gewählten
  Objektzeilen hervorgehoben und fett, die aktuelle Ebene und jede Ebene mit einem gewählten Objekt
  hell getönt. Der Kreis rechts in jeder Ebenenzeile ist gefüllt, wenn die Ebene ein gewähltes eigenes
  Objekt enthält, und grau bei einer Ebene ohne eigene Objekte (Unterebenen zählen nicht, die Webebene
  ebenso). Ein Klick auf einen leeren Kreis verschiebt alle gewählten Objekte als Block oben in diese
  Ebene oder Unterebene (ein teilausgewähltes Objekt verlässt seine Gruppe), lässt sie gewählt und macht
  die Ebene aktuell; Alt+Klick legt stattdessen Kopien dort ab und wählt sie. Das ist ein Schritt
  *Ebenenänderung*; ohne Auswahl, bei gefülltem Kreis, gesperrter Ebene oder zwischen Webebene und
  normalen Objekten geschieht nichts. Die Sperre der Webebene bleibt beim Öffnen einer Datei nicht
  erhalten. Der Knopf *Neue Ebene* legt die Ebene über der aktuellen auf deren
  Stufe an (neben einer aktuellen Unterebene). Ein
  Rechtsklick auf eine Objektzeile wählt zuerst dieses Objekt, sodass *Auswahl auf Hintergrund
  reduzieren*, *Maske hinzufügen* und *Mit darunter liegender auf eine Ebene reduzieren* im Kontextmenü
  dafür gelten (Letzteres nur, wenn das Objekt direkt über einer Bitmap liegt). Ein Rechtsklick auf eine
  Ebenenzeile macht sie zur aktuellen Ebene und lässt die Auswahl stehen; auf der Webebene sind die
  Einträge zum Freigeben für Seiten grau. *Maske hinzufügen* legt eine leere, verknüpfte Bitmapmaske an,
  die alles zeigt, und wählt die Maske.
  **Gruppeninhalte (Firepixel-Erweiterung):** Eine Gruppenzeile zeigt direkt nach dem Namen ein kleines
  „›“; ein Klick darauf listet die Objekte der Gruppe eingerückt unter der Zeile, mit einer dünnen
  Linie links davon (jeweils mit Auge, Schloss, Miniatur und Namen; verschachtelte Gruppen haben ihr
  eigenes „›“, offen wird das Zeichen zu „⌄“). Ein Klick auf eine solche Zeile wählt das Objekt genau
  wie ein Klick mit dem Unterauswahl-Werkzeug (kein Schritt im Verlauf, der Eigenschafteninspektor
  zeigt das Objekt); Umschalt+Klick nimmt es hinzu oder heraus. Doppelklick benennt es um, das Auge
  blendet nur dieses Objekt aus. Solange ein Objekt in einer Gruppe gewählt ist (im Bedienfeld oder
  mit der Unterauswahl auf der Leinwand), ist die Zeile der Gruppe blassgelb getönt, auch wenn die
  Gruppe zugeklappt ist; bei aufgeklappter Gruppe ist die Zeile des Objekts selbst hervorgehoben.
  Ein maskiertes Objekt zeigt nach dem Namen zusätzlich einen Pfeil im Kreis: er listet die Objekte,
  aus denen die Maske besteht (den Maskenpfad, die Objekte einer Gruppenmaske oder eine Zeile
  *Bitmap-Maske*). Ein Klick auf eine solche Zeile wählt die Maske zum Bearbeiten wie ein Klick auf
  die Maskenminiatur; bei einem Objekt einer Gruppenmaske wird danach dieses Objekt gewählt
  (Umschalt+Klick nimmt es hinzu oder heraus). Die Maskenzeilen stehen zuerst und sind mit einer
  Doppellinie markiert; darunter folgen mit der einfachen Linie die Inhaltsobjekte, auch der eine
  Inhalt eines maskierten einzelnen Pfads oder einer Bitmap. Ein Klick auf eine Inhaltszeile wählt nur
  den Inhalt, er lässt sich dann ohne seine Maske verschieben und ändern. Ein maskiertes Objekt in
  einer Gruppe zeigt dasselbe Miniaturenpaar und dieselbe Kette wie ein maskiertes Objekt direkt auf
  der Ebene. Auf- oder Zugeklappt gehört zu jedem Objekt, gilt,
  solange Firepixel läuft, und wird weder gespeichert noch im Verlauf geführt. Zeilen innerhalb
  einer Gruppe lassen sich nicht ziehen (die Gruppe selbst umordnen oder *Ändern ▸ Anordnen*
  verwenden). Der Eintrag *Gruppeninhalte anzeigen* im Bedienfeldmenü (Vorgabe: ein) schaltet all
  das ab; das Bedienfeld sieht dann aus und arbeitet wie im Original.
- **Seiten / Status:** Seiten und Animationsbilder, Masterseite, Zwiebelschicht. Der Abspielknopf in
  der Statusleiste (oder *Ansicht ▸ Abspielen/Stopp*) spielt die Status auf der Leinwand mit der
  Verzögerung jedes Status ab; vom Export ausgeschlossene Status (*Beim Exportieren einschließen* aus)
  werden übersprungen, und die Wiedergabe läuft in einer Schleife, bis Sie Stopp drücken (die
  Schleifeneinstellung des Dokuments gilt für den GIF- und APNG-Export); währenddessen zeigt er ein
  Stopp-Quadrat. *Datei ▸ Bildvorschau* spielt ebenso ab, auch große Dokumente, und berechnet die Status
  vorab. Ist eine
  Animationsinstanz gewählt, zeigt die Leinwand ihren Bewegungspfad (grüner Start, rotes Ende), und der
  Auswahlrahmen umschließt nur das aktuelle Bild.
  *Neue Seite* (Schaltfläche in der Fußleiste, ≡-Menü oder Kontextmenü einer Zeile) fügt die Seite
  direkt hinter der aktuellen Seite ein und macht sie aktuell; sie heißt „Seite N“ mit der kleinsten
  noch freien Zahl. *Seite duplizieren* (≡-Menü, Kontextmenü oder eine Zeile auf die Schaltfläche
  „Neue Seite“ ziehen; Alt+Klick auf die Schaltfläche dupliziert die aktuelle Seite) setzt die Kopie
  direkt hinter ihre Quelle und macht sie aktuell; sie heißt „<Name> Kopie“, dann „<Name> Kopie 1“,
  „Kopie 2“ … (die Kopie einer Kopie zählt vom ursprünglichen Namen weiter). Ein Klick oder Rechtsklick
  auf eine Zeile macht die Seite vorher aktuell, ein eigener Verlaufsschritt „Seitenauswahl ändern“.
  Mehrere Seiten lassen sich auswählen: Umschalt+Klick wählt den Bereich ab der aktuellen Seite,
  Strg+Klick nimmt eine Seite hinzu oder heraus; die aktuelle Seite bleibt angezeigt und fett, jeder
  Klick ist ein Schritt „Seitenauswahl ändern“. *Seite löschen* (Schaltfläche, Menüs oder Entf in der
  Liste) entfernt alle ausgewählten Seiten in einem Schritt, *Seite duplizieren* kopiert jede
  ausgewählte Seite direkt hinter sich; *Seite umbenennen* und *Als Masterseite festlegen* sind bei
  Mehrfachauswahl grau. Wer eine der ausgewählten Seiten zieht, verschiebt alle ausgewählten Seiten
  als Block in ihrer Reihenfolge (ein Schritt „Seiten verschieben“, am Zeiger steht die Anzahl); eine
  nicht ausgewählte Seite wird allein verschoben. Wird die einzige Seite gelöscht,
  bleibt eine leere „Seite 1“. Sind alle Seiten ausgewählt, fragt Firepixel „Alle Seiten löschen?“ und
  lässt die Seite wählen, die bleibt (Skripte behalten die erste Seite oder die mit `keep` genannte).
- **Optimieren:** Exportformat und Farben.
- **Verlauf:** Schritte zurückspulen, wiederholen oder als Befehl speichern. Ein rotes × kennzeichnet einen Schritt,
  der nicht wiederholt oder als Befehl gespeichert werden kann; der Tooltip über dem × oder der Zeile nennt den
  Grund. Zeilen, die an den Schritt davor gebunden sind (*Pinselstrich* / *Strichfarbe* bei der Wahl von Pinsel oder
  Bleistift), haben keinen eigenen Rückgängig-Schritt und werden mit diesem Schritt zurückgenommen.
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
Mischmodus, Filter (+/−) und Text- bzw. Symbol-Einstellungen. Die Linie darüber lässt sich ziehen, um ihn höher
oder niedriger zu machen (Firepixel-Erweiterung; er rastet bei voller und halber Höhe ein, und mindestens 120 Pixel
des Dokuments bleiben sichtbar); die Raute im Reiter und ein Doppelklick auf den Reiter stellen wieder volle, halbe
und eingeklappte Höhe her.

- **Popup-Schieber (▾):** Während Sie den Regler bewegen, zeigt die Arbeitsfläche den Wert live als Vorschau – bei
  Deckkraft (auch beim Deckkraft-Schieber im Ebenen-Bedienfeld, der wie dessen Mischmodus-Liste auf die gewählten
  Objekte wirkt und bei unterschiedlichen Werten leer ist), Strichstärke, Kantenweichheit des Strichs, weicher
  Kante der Füllung, Texturmenge von Füllung und Strich, Eckenrundung des Rechtecks, den Kanalschiebern des
  Farbmischers (an der Auswahl) sowie bei gewählten Textobjekten für Schriftgröße, Laufweite, Zeilenabstand, Einzug, Abstand davor und danach und
  horizontale Skalierung (bei offenem Texteditor zeigt nur das Feld den Wert). Auf den Pfeil drücken und ziehen: Der
  Wert gilt beim Loslassen; Esc verwirft ihn. Auf den Pfeil klicken: Der Schieber bleibt offen, und der Wert gilt,
  wenn Sie daneben klicken oder die Eingabetaste drücken; Esc verwirft ihn. Dieser Klick daneben (auf die
  Arbeitsfläche, ein anderes Objekt, ein Bedienfeld oder die Menüleiste) schließt nur den Schieber und übernimmt den
  Wert für die Objekte, für die er geöffnet wurde; er wählt nichts aus und öffnet kein Menü (dafür erneut klicken),
  wie im Original. Rückgängig oder Wiederholen bei offenem
  Schieber (Tastenkürzel, Menü *Bearbeiten* oder ein Klick im Verlauf-Bedienfeld) verwirft zuerst den offenen Wert
  und schließt den Schieber wie Esc, dann wird rückgängig gemacht bzw. wiederholt; ein offener Wert gilt immer nur
  für die Objekte, die beim Öffnen des Schiebers gewählt waren. Ein eingetippter Wert gilt mit
  Eingabetaste, Tab oder einem Klick ins Dokument; ein Klick auf eine leere Stelle des Bedienfelds übernimmt nichts.
  Jedes Übernehmen ist ein Schritt im Verlauf; ein unveränderter Wert erzeugt keinen. Der Schieber öffnet sich links
  bündig mit dem Pfeil, der Regler beim aktuellen Wert unter der Stelle, an der Sie gedrückt haben; am Fensterrand
  wird er gerade so weit verschoben, dass er hineinpasst (die erste Bewegung setzt dann den Wert unter dem Zeiger).
  Die Deckkraft-Schieber (Inspektor, Werkzeugoptionen, Ebenen-Bedienfeld und die Deckkraft im Farb-Popup) ändern
  den Wert um 1 % je 2 px und liefern immer ganze Prozent, wie das Original.
- **Deckkraft im Farb-Popup:** Ändern Sie die Deckkraft eines Farb-Popups (sein Schieber neben dem Feld „100“ oder
  ein getippter Wert) für Füllung oder Strich der Auswahl – im Inspektor, in der Werkzeugleiste oder im
  Farbmischer –, zeigt die Arbeitsfläche die Farbe mit dieser Deckkraft als Vorschau, solange das Popup offen ist.
  Die Eingabetaste oder das Loslassen nach Drücken und Ziehen am Deckkraft-Pfeil übernimmt sie (ein Schritt im
  Verlauf); Esc schließt erst den Deckkraft-Schieber und dann das
  Popup ohne Änderung. Rückgängig oder Wiederholen schließt das Popup ebenso ohne Änderung und macht dann rückgängig
  bzw. wiederholt.

- **Rand der Füllung:** *Hart*, *Anti-Alias*, *Weiche Kante* und *Dither*. Weiche Kante und Dither nutzen das
  Breitenfeld neben der Liste (ein neuer weicher oder Dither-Rand beginnt mit 10 px). *Dither* ergibt einen
  Retro-Pixelrand: derselbe Übergang wie eine weiche Kante dieser Breite, aber jedes Pixel ist ganz an oder aus,
  gestreut nach einem festen 8 × 8-Ditherraster, das an den Dokumentpixeln verankert ist (es flimmert nicht beim
  Bildlauf, Zoomen oder Verschieben um ganze Pixel). Breite 0 ist ein harter Rand; die eigene Deckkraft der Füllung
  gilt weiterhin. Dither ist eine Firepixel-Erweiterung; die Ränder Hart, Anti-Alias und Weiche Kante des Originals
  bleiben unverändert. In einer `.fw.png`-Datei wird ein Dither-Rand als weiche Kante gleicher Breite mit einer
  Firepixel-Markierung gespeichert, andere Programme zeigen also eine weiche Kante. Auch Textobjekte zeigen Weiche
  Kante und Dither (der Übergang beginnt bei den geglätteten Glyphen); der Text-Inspektor selbst bietet nur die
  Liste *Anti-Aliasing-Stufe* (ihr Eintrag *Kein Anti-Alias* ist der harte Rand), einem Text gibt man eine weiche oder
  Dither-Kante daher per Skript oder mit dem Befehl `pi.fill {edge, feather}`.

- **Dither (Dichte der Füllung):** Das Prozentfeld mit dem kleinen Schachbrett-Symbol unter dem Namen der
  Fülltextur (links von *Transparent*) macht aus der ganzen Füllung ein gleichmäßiges Raster aus ganz deckenden und leeren Pixeln: 50 % ist
  ein schachbrettartiger Halbton der Füllfarbe, 25 % leert jedes vierte Pixel, 75 % behält nur jedes vierte, 0 %
  schaltet es aus, 100 % leert die ganze Füllung (der Pinselstrich bleibt): Der Wert ist der Anteil leerer Pixel, ein
  höherer Wert nimmt also immer mehr weg (bis v0.2.20 war es der Anteil behaltener Pixel und 100 sah aus wie 0;
  ältere Dateien werden beim Öffnen umgerechnet). Es wirkt bei jeder Füllart (Vollfarbe, Verlauf, Muster) und nutzt dasselbe feste 8 × 8-Raster wie
  der Dither-Rand, verankert an den Dokumentpixeln. Zusammen mit einem Dither-Rand bleibt ein Pixel nur an, wo
  beide es behalten; der Rand läuft also von der gewählten Dichte nach außen aus. Der Pfeil neben dem Feld öffnet einen
  Schieberegler; beim Ziehen zeigt die Leinwand die Dichte sofort, Loslassen übernimmt sie. Eine Änderung ist ein Schritt
  *Füllen* im Protokoll; ein Wechsel der Füllart behält die Dichte. Die QuickInfo nennt das Feld („Dither-Dichte“);
  mit ausgeschaltetem *Ansicht ▸ QuickInfos* bleibt nur dieser Name. Die Dither-Dichte ist eine
  Firepixel-Erweiterung; eine `.fw.png`-Datei speichert sie nur in einer Firepixel-Markierung (geschrieben, wenn
  der Wert nicht 0 ist), andere Programme zeigen die normale Füllung.

- **Verlauf und Muster in Text:** Ein Text nimmt Verlaufs- und Musterfüllungen wie ein Pfad an (`pi.fill`,
  `setFill`); die Glyphen zeigen die Füllung genau so, wie ein Pfad mit denselben Füllgriffen sie bekäme. Die Griffe
  liegen in Dokumentkoordinaten: Ein neuer linearer Verlauf läuft von der Mitte der Oberkante zur Mitte der
  Unterkante der Textgrenzen, ein neues Muster hat seine Kachelecke in der Mitte, bei 100 %. Verschieben (Ziehen,
  Pfeiltasten, Felder X/Y) nimmt die Griffe mit; das Skalieren-Werkzeug und Frei transformieren lassen sie stehen,
  auch beim Ziehen im Transformationsrahmen (der Text wird größer oder wandert, die Füllung nicht); *Numerisch
  transformieren* und `scaleSelection` skalieren sie um die Mitte, mit und ohne „Attribute skalieren“; Bearbeiten
  des Texts (mehr Zeichen, andere Größe) lässt sie an ihrer Stelle.

- **Füllgriffe von Verlauf und Muster:** Mit dem Zeiger zeigt ein gewähltes Objekt mit Verlaufs- oder Musterfüllung
  seine Füllgriffe: den runden Mittelgriff und ein oder zwei eckige Griffe an Linien davon. Ziehen am
  **Mittelgriff verschiebt alle drei Griffe gemeinsam** (das ganze Muster bzw. der ganze Verlauf wandert, Kachelgröße
  und Winkel bleiben), wie im Original gemessen. Ziehen an einem eckigen Griff bewegt nur diesen (er ändert Breite
  oder Höhe der Kachel und den Winkel eines Musters bzw. Länge und Richtung eines Verlaufs). Ziehen an einer
  **Linie** vom Mittelgriff dreht beide Arme starr um den Mittelgriff (die Längen bleiben, der gezogene Arm zeigt
  zum Zeiger). **Umschalt** beschränkt einen Zug auf die überwiegende Achse (waagerecht oder senkrecht). **Alt** auf
  einem Griff klont das Objekt wie ein Alt-Zug, statt den Griff zu bewegen. Die Füllung folgt während des Ziehens
  live. Über einem Griff ist der Zeiger ein hohler Pfeil, über einer Grifflinie ein Pfeil mit Drehzeichen. Ein
  Druck auf einen Griff verschiebt nie das Objekt; das Objekt an einer Stelle abseits der Griffe ziehen. Jeder
  Griffzug ist ein Protokollschritt.

- **Verlaufs-Dither:** Der Dither-Knopf einer Verlaufsfüllung glättet den Verlauf wie im Original: Die Farben
  folgen einer sanften S-Kurve (an beiden Enden langsamer, in der Mitte schneller) mit leichtem Rauschen von ±1 je
  Farbkanal, das Streifen verdeckt. Ohne ihn läuft der Verlauf linear. (Das genaue Rauschmuster des Originals ist
  unbekannt; Firepixel nimmt ein festes eigenes Muster, dieselbe Datei sieht also immer gleich aus.)

- **Muster im Dokument festhalten (Firepixel-Erweiterung):** Normalerweise wandert ein Muster oder Verlauf mit
  seinem Objekt, wie im Original. Mit dem Vorhängeschloss-Knopf im Füllungsteil des Eigenschaften-Inspektors (rechts
  von „Transparent“, neben dem Dither-Knopf des Verlaufs; sichtbar, wenn das gewählte Objekt eine Muster- oder
  Verlaufsfüllung hat) bleibt die
  Füllung stattdessen im Dokument stehen: Verschieben (Ziehen, Pfeiltasten, Felder X/Y, *Ausrichten*,
  `moveSelectionBy`/`moveSelectionTo`) und Transformieren (Numerisch transformieren, Skalieren-, Neigen- und
  Verzerren-Werkzeug, Frei transformieren, Spiegeln und Drehen, mit und ohne „Attribute skalieren“) lassen die
  Füllgriffe stehen, das Objekt gleitet über ein festes Muster. Setzen oder Löschen ist ein Protokollschritt
  „Füllen“. Kopien behalten die Einstellung: Duplikat, Klon, Kopie per Alt-Ziehen und eingefügte Kopie zeigen das
  Muster an derselben Stelle im Dokument wie das Original. Die Füllgriffe selbst lassen sich weiter ziehen. Das
  Original kennt diese Option nicht; in einer `.fw.png`-Datei steht die Einstellung nur in der Firepixel-Markierung
  der Füllung (andere Programme zeigen das Objekt mit mitwanderndem Muster, Dateien ohne die Einstellung bleiben
  unverändert). Befehle: `pi.fill {lockPattern: true}`; in Skripten `setFill` mit `lockToDocument: true` (ein aus
  `pathAttributes.fill` gelesenes Füllungsobjekt meldet `lockToDocument: true` nur, wenn sie an ist).

- **Filter (+):** Das Menü „+“ gliedert die Effekte wie im Original, z. B. *Abgeflachte Kante und
  Relief* ▸ *Abgeflachte Kante innen*, *Abgeflachte Kante außen*, *Relief vertieft*, *Relief erhaben*.
  Die Art steht im Menüeintrag; der Dialog hat dafür kein eigenes Feld. Doppelklick auf einen Eintrag
  der Filterliste öffnet seine Einstellungen.

- **Kontur (PSD-Live-Effekt):** *PSD-Live-Effekte* ▸ *Kontur*, das Untermenü nach dem Trennstrich am Ende des
  Menüs „+“, zieht ein Band um den deckenden Teil des Objekts, wie der Ebeneneffekt Kontur von PSD-Dateien.
  Einstellungen: *Größe* (1–250 px, Vorgabe 3), *Position* (Außen, Innen oder Mitte auf der Objektkante),
  *Mischmodus* (Normal, Multiplizieren, Negativ multiplizieren, Überlagern, Abdunkeln, Aufhellen, Farbig
  nachbelichten, Farbig abwedeln, Linear abwedeln, Weiches Licht; auf transparenten Pixeln wirkt jeder Modus wie
  Normal), *Deckkraft* (0–100 %), *Füllart* (Farbe) und *Farbe* (Vorgabe Rot). Die Filterliste zeigt sie als
  *Kontur*; Doppelklick ändert sie. Gespeichert wird sie so, wie das Original seine PSD-Live-Effekte speichert (ein
  `stroke`-Datensatz im PSD-Live-Effekt der Effektliste), `.fw.png`-Dateien behalten sie also für beide Programme.
  Ein aus einer PSD-Datei importierter PSD-Live-Effekt mit weiteren Effekten heißt *PSD-Live-Effekte*; sein Dialog
  ändert nur die Kontur. Skripte nutzen `fp.run("effect.stroke", {size, position, blendMode, opacity, color})`
  (`index` ändert einen vorhandenen Effekt); der Stapelschritt *Kontur* (`effect.stroke`) wendet sie auf Bilder an.
  Verlaufs- und Musterkonturen importierter Dateien behalten ihre Einstellungen, werden aber in ihrer Farbe
  gezeichnet.

- **Rasterfolie (Live-Filter, Firepixel-Erweiterung):** *Andere* ▸ *Rasterfolie* wandelt die Pixel des Objekts in
  ein Halbtonraster wie die gedruckten Rasterfolien in Schwarzweiß-Illustrationen und Manga-Seiten. Einstellungen:
  *Muster* (Punkte, Linien, Kreuzschraffur), *Rasterweite* (Zellenabstand 2–32 px), *Winkel* (0–180°, Vorgabe das
  klassische 45°-Raster), *Dichte aus der Helligkeit* (an: dunkle Stellen bekommen große Punkte, helle kleine, Weiß
  keine; aus: überall die feste *Dichte* 0–100 %), *Farbe* der Tinte (Vorgabe Schwarz) und *Objektfarbe beibehalten*
  (Tinte in der eigenen Farbe des Objekts) und *Glätten* (Vorgabe aus: harte Punkte, getroffene Pixel ganz deckend,
  alle anderen transparent; an: glatte Punkt- und Linienränder mit Zwischenstufen der Transparenz). Das Raster ist am
  Dokumentraster verankert. Der Stapelschritt *Rasterfolie* (`filter.screentone`) wendet dasselbe Raster auf Bilder
  an. Eine `.fw.png`-Datei speichert den Filter nur als Firepixel-Eintrag der
  Effektliste; das Format des Originals behält also nur seine eigenen Effekte (andere Programme zeigen das Objekt
  ohne Raster).

- **Textvorgaben:** Attribute, die bei offenem Texteditor gesetzt werden, ändern nur den markierten Text;
  mit dem Zeiger ausgewählter Text oder keine Auswahl macht sie zur Vorgabe für neue Texte.
  Ein Schriftwechsel setzt die Schrift außerdem an den Anfang der zuletzt verwendeten Schriften; ein Stil
  aus dem Bedienfeld Stile macht bei Text Schrift, Fett/Kursiv/Unterstrichen, Zeilenabstand und Glättung
  zur Vorgabe (nicht die Größe). Die Grundlinienverschiebung wird nur über die Einstellungen des
  Textwerkzeugs zur Vorgabe, nicht über den Inspektor.
  Beim Wählen des Textwerkzeugs (Werkzeugleiste, T oder Wechsel von einem anderen Werkzeug) werden Einzug,
  Abstand davor und danach, Laufweite und horizontale Skalierung auf die Werkseinstellung zurückgesetzt,
  Zeilenabstand (in Prozent) und Grundlinienverschiebung auf die Werte aus Voreinstellungen ▸ Text;
  Schrift, Größe, Stil und Ausrichtung bleiben. Absatzwerte, die bei aktivem Textwerkzeug gesetzt werden, gelten also für den nächsten
  Text, mit dem Zeiger gesetzte nicht. Die Voreinstellung `TextEditor_RememberALLSettings` (aus, keine Zeile im Dialog
  Voreinstellungen) behält sie. Ein neuer Text beginnt mit Laufweite 0, und jede Änderung in einem Textfeld des
  Inspektors ist ein eigener Schritt „Textänderung“ im Verlauf.
- **Schriftsuche:** Die aufgeklappte Schriftliste (Text-Inspektor, Textwerkzeug, Texteditor) hat oben
  ein Suchfeld. Tippen filtert (Teilwort, ohne Groß-/Kleinschreibung), ↑/↓ wählen, Eingabe
  übernimmt, Esc schließt ohne Änderung. Solange die Liste offen ist, zeigt die Arbeitsfläche die markierte Schrift
  als Vorschau an den ausgewählten Textobjekten (Firepixel-Erweiterung, *Ansicht ▸ Live-Schriftvorschau*, ab Werk
  an); erst Eingabe oder ein Klick ändert das Dokument. Im Texteditor-Dialog erscheint die Vorschau an den
  ausgewählten Textobjekten, nicht am Text im Dialog.
- **Texturen und Muster:** Die Listen zeigen alle Texturen bzw. Muster aus den Ressourcenordnern;
  „Andere…“ unter der Liste wählt eine eigene Bilddatei.
- **Farbwähler:** Ein Klick auf ein Farbfeld öffnet das Popup mit Farbfeldern, HEX-Feld und Pipette. Die
  Pipette nimmt beim Überfahren jede sichtbare Farbe im Fenster auf, auch über Bedienfeldern, Menüs und
  offenen Dialogen. Ein Klick übernimmt die Farbe. Sie liest auch die echte Bildschirmfarbe außerhalb des
  Firepixel-Fensters, auch aus anderen Programmen: im Fenster drücken (auf den Farbfeldern oder neben dem
  Popup), die Maustaste halten, aus dem Fenster ziehen und dort loslassen; solange die Taste gehalten wird,
  folgt die Vorschau der Bildschirmfarbe (ein Pixel). Ein Klick direkt in ein anderes Programm geht an
  dieses Programm. Unter Linux wird der Bildschirm nur unter X11 gelesen (unter Wayland bleibt die
  Arbeitsfarbe); macOS fragt nach der Berechtigung „Bildschirmaufnahme“. HEX und RGB zeigen immer die Grundfarbe; ihre Deckkraft steht nur im Deckkraftfeld und im
  Vorschaufeld (das Original zeigt beim erneuten Öffnen einer halbtransparenten Farbe eine über Weiß
  gemischte Farbe; Firepixel bewusst nicht). Auf den Deckkraft-Pfeil drücken und ziehen: Die Deckkraft folgt
  dem Zeiger und gilt beim Loslassen, das Popup schließt sich; Esc während des Ziehens bricht ab.

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
(Einzelheiten in `docs\MODELLE.md`); ohne Oberfläche lädt `firepixel-cli models download all` sie
(etwa 1,6 GB für alle acht).

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
In den PDF-Exportoptionen schreibt die Komprimierung „JPEG 2000“ die Seitenbilder als JPEG 2000 (JP2,
`/JPXDecode`). „Qualität“ bestimmt die Kompressionsrate, von etwa 10:1 (Hoch) bis etwa 40:1
(Niedrig). Transparenz kommt wie bei JPEG in eine eigene weiche Maske. JPEG 2000 gibt es erst ab
PDF 1.5; mit „PDF 1.3“ oder „PDF 1.4“ wird die Datei daher als PDF 1.5 geschrieben.

**PDF- und PSD-Exportoptionen:**

- PDF, „Textauswahl aktivieren“: Jedes sichtbare Textobjekt wird als unsichtbarer Text über das
  Seitenbild geschrieben, so lässt sich Text im PDF-Betrachter markieren, suchen und kopieren. Nur
  Zeichen des westeuropäischen Zeichensatzes (WinAnsi) kommen in die Textebene; andere bleiben im Bild
  sichtbar.
- PSD, „In PSD-Ebenen konvertieren“: Jedes Objekt wird zu einer eigenen PSD-Ebene, gruppiert nach
  Dokumentebene. Mit „Bearbeitbarkeit beibehalten“ werden bei Text die Textobjekte zu bearbeitbaren
  Textebenen, bei Effekten Schlagschatten, innerer Schatten, Leuchten, Abgeflachte Kante/Relief,
  Farbfüllung und die Kontur (PSD-Live-Effekt, Vollton) zu Ebeneneffekten (eine wieder geöffnete PSD-Datei bringt
  die Kontur als denselben Live-Effekt zurück), bei Vektor Pfade mit einfacher Füllung zu Füllebenen mit Vektormaske.
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
  Übersetzungen prüfen (`language check`) sowie die KI-Modelle auflisten, herunterladen und prüfen
  (`models list`, `models download realEsrganX4` oder `all`, `models verify`; nur offizielle Quellen,
  SHA-256 geprüft, der Aufruf selbst ist die Zustimmung). `firepixel-cli <befehl> --help` zeigt Beispiele;
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

## Hinweise zu Fremdkomponenten

Der Batch-Schritt *Gesichter weichzeichnen* (`ai.faceBlur`) findet Gesichter mit einer in Firepixel
eingebauten Erkennung. Deren Klassifikator ist die Frontalgesichts-Kaskade
`haarcascade_frontalface_default.xml` von Rainer Lienhart aus dem OpenCV-Projekt, in eine kompakte
Form umgewandelt und ins Programm eingebettet. Sie steht unter dieser Lizenz (Originaltext):

                            Intel License Agreement
                    For Open Source Computer Vision Library

     Copyright (C) 2000, Intel Corporation, all rights reserved.
     Third party copyrights are property of their respective owners.

     Redistribution and use in source and binary forms, with or without modification,
     are permitted provided that the following conditions are met:

       * Redistribution's of source code must retain the above copyright notice,
         this list of conditions and the following disclaimer.

       * Redistribution's in binary form must reproduce the above copyright notice,
         this list of conditions and the following disclaimer in the documentation
         and/or other materials provided with the distribution.

       * The name of Intel Corporation may not be used to endorse or promote products
         derived from this software without specific prior written permission.

     This software is provided by the copyright holders and contributors "as is" and
     any express or implied warranties, including, but not limited to, the implied
     warranties of merchantability and fitness for a particular purpose are disclaimed.
     In no event shall the Intel Corporation or contributors be liable for any direct,
     indirect, incidental, special, exemplary, or consequential damages
     (including, but not limited to, procurement of substitute goods or services;
     loss of use, data, or profits; or business interruption) however caused
     and on any theory of liability, whether in contract, strict liability,
     or tort (including negligence or otherwise) arising in any way out of
     the use of this software, even if advised of the possibility of such damage.

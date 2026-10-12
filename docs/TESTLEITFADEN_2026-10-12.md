# Testleitfaden: alles Neue seit 12.10.2026, 04:00

Für deinen Hands-on-Test der Testversion. Hier steht alles, was seit Montagmorgen geändert wurde: über 140 Commits von rund 40 Agenten, geordnet nach Bereichen. Jeder Punkt hat dieselbe Form:

- **Was:** neu bzw. anders
- **Hinkommen:** Menü, Werkzeug (Kürzel) oder Bedienfeld
- **Ausprobieren:** ein paar Handgriffe → **erwartetes Ergebnis**
- **Original:** wie es sich dort verhält (nur wo es hilft)

Menüpfade stehen so da, wie sie in Firepixel auf Deutsch heißen. Auf Englisch heißen sie entsprechend (*File ▸ Image Preview…* usw.).

Zeichen: ✅ geht und wurde im echten Fenster geprüft · 🔧 gebaut, im echten Fenster noch nicht geprüft (steht in `NACHPRUEFUNG.md`) · ⚠️ geht teilweise · ❌ offen

Tipp vorab: Leg dir ein Testdokument an (*Datei ▸ Neu*, 600 × 400, weiß) mit einem Rechteck, einer Ellipse, einem Text und einer Bitmap, z. B. ein Foto per *Datei ▸ Importieren…* (Strg+R). Die meisten Proben unten gehen damit.

---

## A. Deine Meldungen (N01–N23)

| Nr | Stand | Kurz |
|---|---|---|
| N01 | 🔧 | Effekt stapelt sich beim mehrfachen Ändern |
| N02 | ✅ | Dialoge verschieben |
| N03 | ✅ | Strg+Mausrad in Stufen |
| N04 | ✅ | Ziehen nur innerhalb der Leinwand sichtbar |
| N05 | ✅ | Strg+C/Strg+V |
| N06 | ✅ | Rückfrage beim Schließen ohne Blocksatz |
| N07 | ✅ | Bild auf die Leinwand ziehen |
| N08 | ✅ | Taskleisten-Symbol (von dir bestätigt) |
| N09 | ✅ | ruhiger Start |
| N10 | ✅ | Zoomen ohne Versatz |
| N11 | ❌ teilweise | Dialoge und Startbildschirm schöner |
| N12 | ✅ | KI-Objektauswahl mehrfach klicken |
| N13 | ✅ | großer Text nach Esc sofort sichtbar |
| N14 | ✅ | Schriftgrößen-Schieber in ganzen Schritten |
| N15 | ✅ | Textgröße live |
| N16 | ✅ | „Speichern unter“ hängt nicht |
| N17 | ✅ | Neutralität |
| N18 | ✅ | keine „Pick an app“-Fenster |
| N19 | ✅ | Suche in der Schriftliste |
| N20 | ✅ | Nachzeichnen-Fenster größer |
| N21 | 🔧 | KI-Fortschritt schöner und Feineinstellungen |
| N22 | ⚠️ | Bildvorschau: Teil 1 und 2 fertig, Teil 3 und 4 fehlen |
| N23 | 🔧 teilweise | abgeschnittene Texte: Prüfer und dein Beispiel erledigt, Rest offen |

**N01 Effekt stapelt sich** 🔧
- **Was:** Ein Live-Filter entsteht nur einmal, egal wie oft du den Wert änderst. Neu seit heute Abend: Abbrechen stellt auch den Verlauf genau wieder her.
- **Hinkommen:** Rechteck wählen, im Inspektor „+“ ▸ *Schatten und Glühen* ▸ *Glühen*.
- **Ausprobieren:** Breite 10-mal ändern (tippen und den Schieber ziehen), OK → genau **ein** Glühen, ein Verlaufsschritt. Noch einmal, aber **Abbrechen** → kein Glühen; Strg+Y bringt es nicht zurück.

**N02 Dialoge verschieben** ✅
- **Hinkommen:** z. B. *Modifizieren ▸ Leinwand ▸ Bildgröße…*
- **Ausprobieren:** an der Titelleiste schnell und langsam ziehen → der Dialog folgt der Maus 1:1, die Titelleiste bleibt unter dem Zeiger.

**N03 Strg+Mausrad** ✅
- **Ausprobieren:** über der Leinwand Strg + Rad → 100 → 150 → 200 → 300 → 400 % und zurück; kein Sprung auf 6 % oder 6400 %.

**N04 Ziehen über die Leinwandkante** ✅
- **Ausprobieren:** Rechteck greifen und weit über die Leinwandkante ziehen, Maus halten → die Vorschau ist an der Leinwand abgeschnitten, außerhalb siehst du nur den Rahmen. Wie im Original.

**N05 Kopieren und Einfügen** ✅
- **Ausprobieren:** Objekt wählen, Strg+C, Strg+V → Kopie an derselben Stelle; Strg+X, Strg+V → wieder da. Neu: Strg+Z nach dem Einfügen lässt wie im Original nichts gewählt.

**N06 Rückfrage beim Schließen** ✅
- **Ausprobieren:** Dokument ändern, Strg+W → „Möchten Sie die Änderungen … speichern?“ einzeilig, linksbündig, mit dem Dateinamen. Neu: Die Rückfrage kommt sofort (früher 1–2 s später).

**N07 Bild auf die Leinwand ziehen** ✅
- **Ausprobieren:** ein PNG aus dem Explorer auf die offene Leinwand ziehen → es wird an der Ablagestelle **eingefügt**, kein neues Dokument. Auf einer freien Fläche außerhalb der Leinwand wird es geöffnet.

**N08 Taskleisten-Symbol** ✅
- Hast du schon bestätigt.

**N09 Start** ✅
- **Ausprobieren:** Firepixel starten → das Fenster erscheint einmal fertig und maximiert, ohne Aufblitzen, Verschwinden oder weißes Bild.

**N10 Zoomen ohne Versatz** ✅
- **Ausprobieren:** großes Dokument, Strg+Rad bzw. Strg+Plus/Minus mehrmals → die Seite springt direkt an die neue Lage, nie kurz nach rechts.

**N11 Dialoge und Startbildschirm schöner** ❌ teilweise
- Noch offen, Paket P-DESIGN nach der Pause. Nur der Nachzeichnen-Dialog und die KI-Fortschrittskarte sind schon neu gestaltet.

**N12 KI-Objektauswahl mehrfach klicken** ✅
- **Hinkommen:** Werkzeuge, Platz des Zauberstabs (W gedrückt halten) ▸ *Objektauswahl*; dafür muss ein KI-Modell geladen sein.
- **Ausprobieren:** Kreis klicken → Kreis; Rechteck klicken → nur Rechteck; Umschalt+Klick → beide; Alt+Klick → abziehen. Esc beendet den Bitmapmodus und wechselt zum Zeiger, wie beim Zauberstab.

**N13 großer Text nach Esc** ✅
- **Ausprobieren:** Textwerkzeug (T), klicken, Größe 96, „Gross“ tippen, Esc → der Text ist sofort sichtbar und der Rahmen passt.

**N14 Schriftgrößen-Schieber** ✅
- **Ausprobieren:** Text wählen, im Inspektor ▾ neben der Größe → senkrechter Schieber. Langsam ziehen → 24, 25, 26 … in ganzen Schritten; Loslassen ergibt **einen** Verlaufsschritt.

**N15 Textgröße live** ✅
- **Ausprobieren:** Text tippen, Strg+A, im Inspektor 96 + Eingabe → Text und Rahmen sofort 96 pt.
- **Neu (wie im Original):** Ist nichts markiert, nur die Einfügemarke gesetzt, gilt die neue Größe nur für die danach getippten Zeichen.

**N16 „Speichern unter“ hängt nicht** ✅
- **Hinkommen:** Strg+Umschalt+S.
- **Ausprobieren:** Dialog offen lassen → Firepixel zeichnet weiter, und Leinwand und Menüs sind gesperrt wie bei jedem Dialog. Abbrechen → alles normal.

**N17 Neutralität** ✅
- **Ausprobieren:** *Hilfe ▸ Über Firepixel…*, Titel, Menüs, Tooltips → nirgends der Name des Originals oder seines Herstellers. Automatisch geprüft von `cargo test -p fp-neutral`.

**N18 keine „Pick an app“-Fenster** ✅
- Gezählt vor und nach 17 Testsitzungen: 0 Fenster. Agenten starten die App mit `FP_NO_SHELL_OPEN=1`.

**N19 Suche in der Schriftliste** ✅
- **Hinkommen:** Text wählen (oder Textwerkzeug T), im Inspektor die Schriftliste aufklappen; ebenso im Texteditor.
- **Ausprobieren:**
  - „goe“ tippen → nur Segoe-Schriften; ↓ und Eingabe → übernommen, ein Verlaufsschritt.
  - „zzqx“ → „Keine Schrift gefunden“; Esc → nichts geändert, Auswahl bleibt.
- *Text ▸ Schrift* bleibt ein Menü mit Buchstaben-Untermenüs, wie im Original.

**N20 Nachzeichnen-Fenster größer** ✅
- **Hinkommen:** Bitmap wählen, *Modifizieren ▸ Bitmap nachzeichnen…*
- **Ausprobieren:**
  - Fenster ≈ 1050 × 830, „Aufräumen“ und „Erweitert“ sind ohne Bildlauf sichtbar.
  - Ecke kleiner ziehen, Abbrechen, wieder öffnen → gleiche Größe, auch nach einem Neustart.

**N21 KI-Fortschrittskarte und Feineinstellungen** 🔧
- **Hinkommen:** Foto öffnen, Werkzeug *Motiv auswählen* oder *Objektauswahl*.
- **Ausprobieren:**
  - Klicken → nach ≈ 0,3 s eine ruhige Karte unten mittig über der Leinwand: Symbol, Werkzeugname, Modell, Stufen („Modell laden“, „Bild analysieren“ …), Balken, Zeit, Abbrechen. Bei schnellen Läufen blitzt nichts auf.
  - Danach im Inspektor *Toleranz*, *Variante* (Klein/Mittel/Groß, nur Objektauswahl), *Rand* (Hart/Anti-Alias/Weiche Kante), *Ausdehnen*, *Glätten* ändern → die Auswahl passt sich **sofort** an, ohne neue Karte, weil das Modell nicht neu rechnet.
  - Esc oder Abbrechen mitten im Lauf → „… abgebrochen“, Auswahl unverändert.

**N22 Bildvorschau** ⚠️ (siehe Bereich 6)
- Teil 1 (1/2/4-fach) und Teil 2 (Registerkarte Optionen) sind fertig.
- Teil 3 fehlt noch: Registerkarte Datei, Exportbereich, Animation.
- Teil 4 fehlt noch: OK/Exportieren-Verhalten, Vorgaben, Tooltips.

**N23 abgeschnittene Texte** 🔧 teilweise
- **Dein Beispiel:** *Fenster ▸ Optimieren*, Format JPEG (auch PNG 32, AVIF) → „Zeilensprung“/„Progressiv“ ganz sichtbar, die Farbtabelle liegt darunter.
- **Rest:** Ein automatischer Prüfer kennt noch 70 Stellen, vor allem den Inspektor bei 1024 × 768 und 150 % und sehr hohe Dialoge. Menüs und Tooltips prüft er noch nicht. Folgt als Paket RAND-2.

---

## B. Nach Bereichen

### 1. Übersetzung Deutsch/Englisch

**Sprache umstellen** ✅
- **Hinkommen:** *Ansicht ▸ Sprache* oder *Bearbeiten ▸ Voreinstellungen… ▸ Allgemein*.
- **Ausprobieren:** auf English stellen → Menüs, Bedienfelder, Inspektor, Dialoge, Tooltips und Statuszeile sofort englisch, ohne Neustart. Zurück auf Deutsch → Wortlaut wie vorher.

**Keine festen Texte mehr** ✅
- **Was:** Alle 389 fest eingebauten Texte stehen jetzt in Sprachdateien. Ein Scanner prüft den ganzen Quelltext.
- **Ausprobieren:** auf Englisch Dialoge öffnen (Bildgröße, Raster, Voreinstellungen, Bitmap nachzeichnen, Bildvorschau) und Warnungen auslösen → kein deutsches Wort, keine Rohschlüssel wie `dlg-…`.

**Hilfe in der UI-Sprache** ✅
- **Hinkommen:** *Hilfe ▸ Firepixel-Hilfe*.
- **Ausprobieren:** auf Englisch → die englische Anleitung öffnet sich.

**Menütexte wie im Original** ✅
- **Was:** Filter-Untermenüs, „Vorschau in <Browser>“, Hilfe-Einträge und Tastennamen haben denselben Wortlaut wie im Original.

**Weitere Sprachen** ✅
- **Hinkommen:** *Ansicht ▸ Sprache ▸ Sprachordner öffnen…*
- **Was:** Eine `.ftl`-Datei dort ablegen ergibt eine neue Sprache. Der Ordner heißt jetzt `Languages` (siehe Bereich 11).

### 2. Bitmap nachzeichnen (Bild → SVG)

**Dialog „Bitmap nachzeichnen“** ✅
- **Hinkommen:** Bitmap wählen, dann *Modifizieren ▸ Bitmap nachzeichnen…*, Rechtsklick auf die Bitmap oder der Knopf im Bitmap-Inspektor.
- **Ausprobieren:**
  - Vorgabe-Karten *Logo*, *Schwarzweiß*, *Illustration*, *Foto* anklicken → die Vorschau rechnet live im Hintergrund.
  - Ansicht *Vergleich / Original / Vektor / Konturen* umschalten; die Kennzahlen zeigen Pfade, Punkte, Farben und Übereinstimmung.
  - OK → neue Ebene „Nachgezeichnet“ mit einer Gruppe und genau ein Rückgängig-Schritt; die Bitmap bleibt („Original behalten“).

**Aufräumen** ✅
- **Was:** Bei *Logo* ist es an. Es rastet Farben ein, begradigt Linien, erkennt Kreise und Rechtecke und spart Knoten (Logo-Beispiel 1108 → 30 Knoten).
- **Ausprobieren:** *Aufräumen* aus/an → die Knotenzahl „vorher → nachher“ ändert sich. Unter *Erweitert* gibt es die Einzelschritte mit Toleranzreglern.

**Als SVG speichern** ✅
- **Ausprobieren:** im Dialog speichern → `.svg`-Datei. Sie lässt sich im Browser öffnen.

**Fenstergröße** ✅
- siehe N20.

### 3. Werkzeuge und Leinwand

**Fang beim Verschieben** 🔧
- **Was:** Rastet nur bis 5 px und nur gleichartig ein: Kante an Kante, Mitte an Mitte.
- **Ausprobieren:**
  - Rechteck langsam an ein anderes heranziehen → rastet Kante an Kante bzw. Mitte an Mitte, aber nicht Kante an Mitte.
  - Mit Umschalt kein Fang am Start und Ende.

**Stiftpunkte** 🔧
- **Hinkommen:** Stift (P).
- **Ausprobieren:** neben einer Objektmitte klicken → kein Fang an die Mitte, an Kanten schon.

**Alt am Eckpunkt** 🔧
- **Hinkommen:** Teilauswahl (A).
- **Ausprobieren:** Eckpunkt eines Pfads mit Alt ziehen → nur der Ausgangshenkel kommt heraus, der Eingangshenkel bleibt.

**Frei transformieren** ✅
- **Hinkommen:** Skalieren (Q).
- **Ausprobieren:**
  - Ecke ziehen → immer proportional; mit Alt von der Mitte aus.
  - Esc → die Änderung bleibt, das Werkzeug wechselt zum Zeiger. Wie im Original.

**Eckgriff am Rechteck** ✅
- **Ausprobieren:** Zeiger (V), Rechteck wählen, Eckgriff ziehen → skaliert. Über den Eckgriffen zeigt der Zeiger einen weißen Pfeil.

**Gummiband** ✅
- **Ausprobieren:** mit dem Zeiger einen Rahmen ziehen, der ein Objekt nur teilweise berührt → das Objekt ist gewählt, wie im Original.

**Zoomrahmen** ✅
- **Hinkommen:** Zoom (Z).
- **Ausprobieren:** Rahmen ziehen → stufenloser Zoom; der Ausschnitt füllt die Ansicht bis auf einen Rand von ≈ 17 px.

**Strg+Leertaste** ✅
- **Ausprobieren:** in jedem Werkzeug Strg+Leertaste halten und klicken → zoomt hinein, auch bei schnellem Klick.

**Mausrad** ✅
- **Ausprobieren:** Rad ohne Taste → Bildlauf um 52 px je Rastung; Umschalt+Rad ohne Wirkung, wie im Original.

**Esc nach dem Zauberstab** 🔧
- **Hinkommen:** Zauberstab (W) auf einer Bitmap.
- **Ausprobieren:** Esc → der Bitmapmodus endet, die Bitmap bleibt **gewählt**.

**Doppelklick auf eine Bitmap** ✅
- **Ausprobieren:** Doppelklick → Bitmapmodus mit Auswahlrahmen; Esc ohne Auswahlrahmen tut nichts.

**Zuschneiden + Esc** ✅
- **Hinkommen:** Zuschneiden (C).
- **Ausprobieren:** Esc → Zeiger. Der Zuschneide-Mauszeiger sieht aus wie im Original.

**Klick auf durchsichtige Bitmap-Pixel** 🔧
- **Ausprobieren:** Bitmap mit transparenter Ecke (z. B. eine runde Bitmap) in der Ecke anklicken → die Bitmap ist gewählt (vorher nichts). Außerhalb ihres Rahmens wird nichts gewählt.

**Pinsel malt Pixel** ✅
- **Hinkommen:** Pinsel (B).
- **Ausprobieren:**
  - Ohne gewählte Bitmap malen → **neue Bitmap** in Strichgröße, das Werkzeug bleibt Pinsel.
  - Mit gewählter Bitmap → der Strich geht hinein.
  - Beim Malen erscheint kein Auswahlrahmen.
  - Esc → Zeiger, die Bitmap bleibt gewählt.

**Radiergummi, Weichzeichner, Stempel** ✅
- **Hinkommen:** Abschnitt Bitmap des Werkzeugbedienfelds (Plätze von Radiergummi, Weichzeichner und Stempel; Maustaste gedrückt halten zeigt die Nachbarwerkzeuge).
- **Ausprobieren:**
  - Auf einem Foto radieren, weichzeichnen und stempeln (Alt+Klick setzt die Quelle) → weicher Rand und Stärke wie im Original gemessen.
  - Radiergummi auf einem Vektor hebt nur die Auswahl auf.

**Verlauf-Werkzeug** 🔧
- **Hinkommen:** am Platz des Farbeimers (G) ▸ *Verlauf*.
- **Ausprobieren:**
  - Inspektor ohne Auswahl → **ein** Farbfeld Schwarz → Weiß und keine Knöpfe Keine/Einfarbig/Verlauf/Muster.
  - Füll- oder Strichfarbe ändern → das Farbfeld bleibt Schwarz → Weiß.
  - Klick aufs Farbfeld → Verlaufseditor.
  - „Auswahl füllen“ und Toleranz gelten getrennt vom Farbeimer.
- **Original:** Das Werkzeug hat dort eigene Farben, unabhängig von Füll- und Strichfarbe.

**Farbe ersetzen** 🔧
- **Hinkommen:** Abschnitt Bitmap des Werkzeugbedienfelds ▸ *Farbe ersetzen* (Maustaste gedrückt halten zeigt die Nachbarwerkzeuge).
- **Ausprobieren:**
  - Inspektor zeigt Größe 13, Toleranz 32, Stärke 200, „Von“ grün, „in“ rot.
  - „in“ auf Blau stellen → die Werkzeugfarben bleiben unverändert, und nach einem Werkzeugwechsel ist es weiter Blau.
  - Auf grünen Pixeln malen → sie werden blau.

**Text: Grundlinie und Vorgaben** ✅
- **Ausprobieren:** Textwerkzeug (T), *vor* dem Klick im Inspektor Schrift, Größe, Glättung und Ausrichtung setzen → der neue Text übernimmt sie; das Werkzeug bleibt Text.

**Rechteck aufziehen** ✅
- **Ausprobieren:** Pfeiltasten und Esc während des Aufziehens wirken nicht, wie im Original.

### 4. Inspektor

**Zahlenfelder** ✅
- **Ausprobieren:**
  - Feld anklicken, Strg+A → markiert den Feldtext (nicht alle Objekte).
  - Esc → alter Wert.
  - Wert tippen, dann daneben oder auf ein anderes Objekt klicken → der Wert wird übernommen, auch bei schnellen Klicks.

**B/H/X/Y** ✅
- **Ausprobieren:**
  - „12,5“ tippen → wie im Original nur die ganze Zahl.
  - B oder H unter 1 → verworfen.
  - Ohne Auswahl sind die Felder weiß und leer.

**Popup-Schieber** ✅
- **Ausprobieren:** ▾ neben Deckkraft, Größe usw. → senkrechter Schieber, der Wert läuft live im Feld mit. Esc schließt nur den Schieber, die Auswahl bleibt.

**Texturen und Muster mit „Andere…“** 🔧
- **Hinkommen:** Rechteck-Werkzeug (U) oder ein gewähltes Rechteck, im Inspektor die Texturliste bzw. bei Füllart *Muster* die Musterliste.
- **Ausprobieren:**
  - Liste aufklappen → 77 Texturen bzw. 60 Muster, so breit wie der längste Name, mit Trennlinie, darunter immer „Andere…“.
  - „Andere…“ → Dateiauswahl, das gewählte Bild wird zur Textur.
  - Textur „Ader“ 50 % wählen, Rechteck aufziehen → die Textur ist sichtbar (vorher ohne Wirkung).
- **Hinweis:** Texturen und Muster kommen aus dem Ordner einer vorhandenen Installation des Originals (unter *Voreinstellungen* eingetragen) und aus `%APPDATA%\Firepixel\Textures` bzw. `Patterns`. Firepixel liefert keine mit.

**Schriftsuche** ✅
- siehe N19.

**Text-Inspektor** ✅
- **Ausprobieren:**
  - Knopf Textausrichtung, Kurzzeichen F/K, Strichfeld und Glättungsliste wie im Original.
  - Schnitt „Regular“ statt „Bold“ als Vorgabe.
  - Größe an der Einfügemarke gilt nur für folgende Zeichen (siehe N15).

**Werkzeugnamen im Inspektor-Kopf** 🔧
- **Ausprobieren:** Werkzeug *Pfadausbesserung - additiv* bzw. *Rote Augen entfernen* → der Name steht zweizeilig und bricht nie mitten im Wort um. Bei einer automatischen Form steht der Formname da.

**Rechteck-Werkzeug** ✅
- **Ausprobieren:** Keine Eckrundung im Werkzeug-Inspektor, wie im Original.

**Polygon** ✅
- **Ausprobieren:** Winkel und „Autom.“ sind bedienbar.

**Auswahlrahmen-Werkzeug** ✅
- **Ausprobieren:** Felder mit Symbolen, Vorgaben 1:2 bzw. 64 × 64.

**Objekt-Inspektor** ✅
- **Ausprobieren:**
  - Vorschaubild an der Lage im Dokument, bei Mehrfachauswahl keins.
  - Horizontalskala mit %.
  - Leere Stil-Liste bei Mehrfachauswahl.
  - Kein Knopf „Bearbeiten“ neben der Exportliste.

**Hinweis der Objektauswahl** ✅
- **Ausprobieren:** Werkzeug *Objektauswahl* → der Inspektor nennt „Esc: Bitmapmodus beenden“.

### 5. Dialoge

**Zahlenfelder nehmen immer Eingaben an** 🔧
- **Was:** Vorher nahmen die Felder nach einigen Öffnungen nichts mehr an. Neu: Ein getippter Wert bleibt auch dann, wenn du mit der Maus in ein anderes Feld klickst.
- **Ausprobieren:**
  - *Modifizieren ▸ Leinwand ▸ Leinwandgröße…* 10-mal öffnen, jedes Mal eine neue Zahl tippen, abwechselnd OK und Abbrechen → das Feld zeigt immer die getippte Zahl.
  - *Modifizieren ▸ Transformieren ▸ Numerisch transformieren…*: Höhe 70 tippen, Breite anklicken → die Höhe bleibt 70.

**Tab-Folge** 🔧
- **Ausprobieren:** in den Dialogen mit Tab durchgehen → die Titelleiste wird nie erreicht, die Reihenfolge ist wie im Original.
  - *Datei ▸ Neu*: Breite › Höhe › Auflösung › Einheiten › Weiß › OK › Abbrechen.
  - *Bearbeiten ▸ Voreinstellungen…*: Startfokus auf der Liste „Allgemein“, Pfeil ab wechselt die Seite.

**Neues Dokument** 🔧
- **Hinkommen:** *Datei ▸ Neu* (Strg+N).
- **Ausprobieren:** Zugriffstasten unterstrichen; Alt+H → Höhe, Alt+T → Transparent, Alt+N → Benutzerdefiniert; es öffnet sich kein Menü und kein Buchstabe landet im Feld.

**Bildgröße** 🔧
- **Hinkommen:** *Modifizieren ▸ Leinwand ▸ Bildgröße…*
- **Ausprobieren:**
  - Bei 500 × 400 die Breite „250“ tippen (ohne Tab) → Höhe 200 und die Druckgröße rechnen sofort mit.
  - Kettensymbole erscheinen nur mit „Proportionen beibehalten“.
  - OK ohne Änderung → kein Verlaufsschritt.

**Leinwandgröße** 🔧
- **Hinkommen:** *Modifizieren ▸ Leinwand ▸ Leinwandgröße…*
- **Ausprobieren:**
  - ↔/↕ vor den Feldern; rechts läuft „B: 600“ beim Tippen mit.
  - Den Anker 3 × 3 mit den Pfeiltasten bewegen; die Pfeile zeigen vom Anker weg.
  - OK → alle Objekte gewählt; Strg+Z stellt alles wieder her.

**Raster bearbeiten** 🔧
- **Hinkommen:** *Ansicht ▸ Raster ▸ Raster bearbeiten…*
- **Ausprobieren:**
  - Nur „Farbe:“, ↔ und ↕ mit je einem Feld, keine Kästchen.
  - 20/30 + OK → Raster geändert, **kein** Verlaufsschritt, aber ein Stern im Titel.
  - Neue Dokumente übernehmen Rasterfarbe und -abstand aus den Voreinstellungen.

**Leinwandfarbe** 🔧
- **Hinkommen:** *Modifizieren ▸ Leinwand ▸ Leinwandfarbe…*
- **Ausprobieren:** Weiß / Transparent / Benutzerdefiniert untereinander; Pfeiltasten in der Gruppe; Transparent + Eingabe → transparent.

**Selektives JPEG** 🔧
- **Hinkommen:** *Modifizieren ▸ Selektives JPEG ▸ Einstellungen…*
- **Ausprobieren:**
  - Ohne Haken sind Qualität, Überlagerungsfarbe und die Kästchen grau; mit Haken bedienbar.
  - Eine andere Farbe wird beim nächsten Öffnen gemerkt.

**Textgröße** 🔧
- **Hinkommen:** *Text ▸ Größe ▸ Andere…*
- **Ausprobieren:** 24-pt-Text gewählt → Feld „24“ markiert; ▾ → Schieber 8…96; 48 + Eingabe → 48 pt; ohne Auswahl erneut öffnen → „48“.

**Schieber mit Kennlinie** ✅
- **Hinkommen:** *Filter ▸ Weichzeichnen ▸ Gaußscher Weichz. …*, *Filter ▸ Scharfzeichnen ▸ Unscharf maskieren…*, *Filter ▸ Farbe anpassen ▸ Farbton/Sättigung…*
- **Ausprobieren:** Schieber ziehen → wie im Original stückweise nicht linear (fein am Anfang).

**Gemerkte Werte** ✅
- **Was:** gilt für Leinwandfarbe, Neues Dokument, Bild- und Leinwandgröße, Numerisch transformieren, Auswahl- und Pfaddialoge.
- **Ausprobieren:** Wert ändern, OK, wieder öffnen → der Wert steht noch da.
- **Achtung:** Filterdialoge merken sich noch nichts, siehe „Bekannte offene Punkte“.

**Warnungen mit „Nicht mehr anzeigen“** ✅
- **Hinkommen:** z. B. ohne Auswahl *Modifizieren ▸ Pfad ändern ▸ Vereinfachen…*
- **Ausprobieren:**
  - Es erscheint eine Warnung im Wortlaut des Originals. Haken „Nicht mehr anzeigen“ setzen → sie kommt nicht wieder.
  - *Befehle ▸ Warnmeldungen zurücksetzen* oder *Bearbeiten ▸ Voreinstellungen… ▸ Alle Warnmeldungen zurücksetzen* → sie kommt wieder.

**Wiederholen im Menü** ✅
- **Ausprobieren:** nach einem Schritt zeigt *Bearbeiten* „Rückgängig: <Schritt>“ bzw. „Wiederholen: <Schritt>“.

**Dialoge verschieben** ✅
- siehe N02.

### 6. Bildvorschau

**Ansicht 1/2/4-fach** ✅ (Teil 1)
- **Hinkommen:** *Datei ▸ Bildvorschau…*
- **Ausprobieren:**
  - Unten die Knöpfe „1/2/4 Vorschaufenster“ → jedes Feld hat eine eigene Einstellung. Ein Feld anklicken → die linke Spalte zeigt dessen Werte.
  - Kopfzeile je Feld mit Format, Größe und Ladezeit.
  - Zoom (6–6400 %) und Bildlauf wirken in allen Feldern gemeinsam.
  - Vorgabenliste mit „+“: Name eingeben, Eingabe → die Vorgabe ist gespeichert.

**Registerkarte Optionen** 🔧 (Teil 2)
- **Ausprobieren:**
  - Formatliste: GIF … BMP 24, nach dem Trennstrich WebP … ICO. Je Format sind die Felder sichtbar bzw. grau wie im Original. Beispiele:
    - JPEG: Qualität, Glättung, Progressiv, Farbränder, keine Farbtabelle.
    - PNG 32: nur Hintergrund.
  - GIF: Palette „Web 216“ → Größe und Bild ändern sich; Rastern anhaken → 100 %; Verlust 60 → kleiner.

**Farbtabelle** 🔧
- **Ausprobieren:**
  - Feld anklicken → Statuszeile „R: G: B: Hex: Index:“.
  - Knöpfe Sperren/Websicher/Transparent → Markierung im Feld.
  - Strg+Klick wählt mehrere; Doppelklick → Farbwahl.
  - Klick in die Vorschau wählt den Eintrag des Pixels.
  - ≡ oder Rechtsklick → Palettenmenü (Sortierung, Farbfeld-Rückmeldung, Palette laden/speichern).

**Pipetten** 🔧
- **Ausprobieren:** „Farbe für Transparenz wählen“ + Klick ins Bild → Liste „Indextransparenz“, Löcher im Bild; mit „hinzufügen“/„entfernen“ weitere Farben.

**Optimieren auf Größe** 🔧
- **Ausprobieren:**
  - Knopf unten rechts → vorbelegt mit der jetzigen Größe. Ziel z. B. 40 % kleiner → weniger Farben, die Größe liegt nahe am Ziel.
  - Ziel 0,01 → Meldung „Die resultierende Datei wäre zu klein …“.

**Optimieren-Bedienfeld** ✅
- **Was:** Es zeichnet die Farbfelder mit demselben Baustein; siehe N23.

### 7. KI-Werkzeuge

**Fortschrittskarte und Feineinstellungen** 🔧
- siehe N21.

**Feineinstellungen werden gemerkt** 🔧
- **Ausprobieren:** Toleranz in ein Inspektorfeld tippen (Klick, Strg+A, „80“, Eingabe) → der Wert bleibt stehen und die Auswahl wird nachgestellt. Die Werte gelten auch für den nächsten Klick.

**Kante verfeinern** ✅
- **Hinkommen:** *Auswählen ▸ Kante verfeinern…*, braucht eine Pixelauswahl.
- **Was:** Wie bisher gibt es nur den Radius; ein anderer Radius rechnet neu.

**Modelle** ✅
- **Hinkommen:** *Befehle ▸ KI-Modelle verwalten…*
- **Was:** Der Modellordner heißt jetzt `Models` (siehe Bereich 11). Deine geladenen Modelle werden beim ersten Start übernommen.

### 8. Datei-Dialoge und Schließen

**Startordner** 🔧
- **Hinkommen:** *Datei ▸ Öffnen…* (Strg+O), *Datei ▸ Importieren…* (Strg+R), *Datei ▸ Speichern unter…* (Strg+Umschalt+S).
- **Ausprobieren:**
  - Datei aus Ordner A öffnen, dann noch einmal Öffnen → startet in A.
  - Speichern unter bei einem gespeicherten Dokument → dessen Ordner.
  - Importieren nach einem Import aus C → C.

**Titel und Filter** 🔧
- **Ausprobieren:**
  - Öffnen: Filterliste FW-PNG … EPS, die neuen Formate, „Alle lesbaren Dateien“ (vorgewählt) und „Alle Dateien“.
  - *Datei ▸ Neu von Vorlage…*: Titel „Neu von Vorlage“, nur FW-PNG.
  - *Datei ▸ Als Vorlage speichern…*: Ordner `%APPDATA%\Firepixel\Templates`.
  - *Datei ▸ HTML aktualisieren…*: Titel „HTML-Datei suchen:“.

**Schließen über die Taskleiste** 🔧
- **Ausprobieren:**
  - Ohne Dokument Rechtsklick auf das Taskleistensymbol ▸ *Fenster schließen* → Firepixel endet sofort (vorher lief der Prozess weiter).
  - Mit geändertem Dokument (auch minimiert) → das Fenster kommt zurück und fragt Ja/Nein/Abbrechen.

**Strg+W** 🔧
- **Ausprobieren:** auf einem geänderten Dokument → die Rückfrage kommt ohne merkliche Verzögerung.

### 9. Menüs

**Befehle-Menü** ✅
- **Hinkommen:** *Befehle*.
- **Ausprobieren:**
  - Die mitgelieferten Befehle sind auch im frischen Profil da (Batch-Befehle, Kreativ, Dokument, Auswahl, Text, Web …).
  - *Befehle ▸ Dokument ▸ Andere Ebenen ausblenden* bzw. *… sperren* bei drei Ebenen → wirkt auf alle anderen Ebenen (vorher brach das Skript bei der zweiten ab).

**Menüeinträge des Gesamttests** ✅
- **Was:** *Auswählen ▸ Bitmap-Auswahl wiederherstellen…* / *löschen…*, Hilfslinien entfernen, *Modifizieren ▸ Pfade kombinieren ▸ Teilen*, Vorschau im Sekundärbrowser und „Warnmeldungen zurücksetzen“ wirken jetzt. Der automatische Test zeigt 345/345 Menüs und 283/283 Kontextmenüs.

### 10. Leistung

**Tippen ohne Verzögerung** ✅
- **Was:** Auf Rechnern ohne Grafikkarte (wie diesem Server) kostete jedes Bild 150–700 ms. Jetzt dauert ein Zeichen ≈ 77 ms, vorher 640 ms; auf einer sehr großen Seite 96 statt 1190 ms. Ziehen ≈ 100 ms je Mausschritt.
- **Ausprobieren:** Text tippen bei 400 % Zoom → die Zeichen erscheinen flüssig.
- **Noch langsam:** Alles auswählen auf Seiten mit Tausenden Objekten und die erste Ziehbewegung sehr großer Objekte.

### 11. Technisches auf Englisch (neue Ordnernamen)

**Neue Ordnernamen** 🔧
- **Was:** Alles, was die App auf der Festplatte anlegt, heißt jetzt englisch:
  - `%APPDATA%\Firepixel\Sprachen` → `Languages`
  - `%LOCALAPPDATA%\Firepixel\Modelle` → `Models`
  - `Allgemeine Bibliothek` → `Common Library`
  - dazu Temp-Ordner `Preview`, `Email`, `Print`, `Help`
  - In der Oberfläche heißt das Bedienfeld weiter „Allgemeine Bibliothek“.
- **Migration:** passiert **beim ersten Start** der neuen Version: Ein alter Ordner wird einmal umbenannt. Geht das nicht (z. B. eine Datei ist gesperrt), liest Firepixel den alten weiter.
- **Ausprobieren:**
  - Nach dem ersten Start im Explorer `%APPDATA%\Firepixel` öffnen → `Languages` statt `Sprachen`.
  - In `%LOCALAPPDATA%\Firepixel` steht `Models`.
  - Deine KI-Modelle und eigenen Sprachen sind noch da (*Befehle ▸ KI-Modelle verwalten…* zeigt sie als vorhanden).

### 12. Kommandozeile und MCP

**CLI auf Englisch** ✅
- `firepixel-cli language check|template|list` (alte Namen `sprache pruefen|vorlage|liste` gehen weiter).
- Hilfe und Ausgaben mit `--lang de|en` bzw. `FIREPIXEL_LANG`.
- **Ausprobieren:** `firepixel-cli --help --lang en` → englisch.

**MCP-Antworten** ✅
- **Was:** standardmäßig englisch. Jedes Werkzeug nimmt `"language": "de"`, die Vorgabe des Servers ändert `FIREPIXEL_MCP_LANG`. Die Ressourcen (Anleitung, Befehlskatalog) gibt es in beiden Sprachen.

**Neue Namen** ✅
- `--design=dark` (alt `dunkel` geht weiter)
- MCP `ui_action {"theme": "dark"}`
- Umgebungsvariable `FP_FILE_DIALOG` (alt `FP_DATEIDIALOG`)
- `commands.json` mit `command_id`

**Neue Befehle und Parameter** ✅
- `modify.traceBitmap` (Bitmap nachzeichnen, `preset`, `refine` …)
- KI-Auswahl `select.aiObject`/`select.aiSubject` mit `tolerance`, `variant`, `edge`, `feather`, `expand`, `smooth`, `retune`
- `bitmap.gradient` mit `fillSelection`/`tolerance`
- `element.setPattern {name}`, `element.setTexture {name}`, `pi.fill {pattern}`
- `pi.text.antialias {mode}`, `text.setToolDefaults`
- `ui_file_dialog` zeigt `directory` und `default_filter`
- Alles beschrieben in `docs\AGENTS.md` / `AGENTS.en.md`.

**Tests schreiben nie in dein Profil** ✅
- **Was:** Automatische Tests laufen in einem eigenen Testprofil. Ein Wächter verhindert, dass sie in `%APPDATA%\Firepixel` schreiben. Die Reste „Testbefehl“ im Befehle-Menü sind weg.

---

## C. Bekannte offene Punkte

Diese Punkte gehen noch nicht oder nur teilweise. Bitte nicht als neuen Fehler melden; sie stehen im Plan (`docs\PLAN.md` §5).

- **Datei-Dialoge erscheinen nicht mittig** (Windows legt sie oben links an), und Zusatzfelder bei Öffnen/Speichern unter fehlen.
- **Bildvorschau Teil 3/4 fehlt:**
  - Registerkarte Datei (Skalieren, Exportbereich), Animation
  - Verhalten von OK/Exportieren, Vorgabenreihenfolge, Tooltips
- **Export-Dialog** (*Datei ▸ Exportieren…*) ist noch der alte. Er wird nach der Spezifikation neu gebaut (Paket E1, danach E2 Optionen-Dialoge). HTML-Einrichtung und Exportassistent werden nur funktional nachgebaut (E3).
- **Kurven und Tonwertkorrektur** (*Filter ▸ Farbe anpassen*) werden neu gebaut (KT-1, KT-2). Pipetten und Histogramm fehlen noch. Auch „Bitmap-Auswahl speichern/wiederherstellen“ als Dialog folgt (KT-3).
- **Menüfreigabe bei Pixelauswahl:** Bei einer Pixelauswahl sperrt das Original 12 Objektbefehle (Duplizieren, Gruppieren, Anordnen …), Firepixel nicht. Außerdem bleiben Untermenü-Köpfe im Original immer aktiv, und *Text ▸ Schrift* sieht anders aus (MN-2).
- **Bedienfeldmenüs ≡:** Verlauf, Stile, Status, Seiten, Ebenen und Optimieren weichen ab (MN-3).
- **Filterdialoge merken sich keine Werte;** Bewegungsunschärfe und Radiales Weichzeichnen wirken falsch, der Sättigungsschieber hat eine falsche Kennlinie (P-DLG5/6).
- **Seltene Dialoge** D26–D50, z. B. Numerisch transformieren ohne Objektgröße und „Pfad in Auswahlrahmen“ ohne Dialog (P-DLG7–9).
- **Speichern unter ▸ Reduziertes PNG** fehlt (EB-2, kommt als Erstes nach der Pause). SWF und Illustrator 8 sind gestrichen.
- **Neues Symbol** (*Modifizieren ▸ Symbol*) und gespeicherte Befehle (DLG-4).
- **Abgeschnittene Texte:** 70 bekannte Stellen; Menüs und Tooltips sind noch ungeprüft (RAND-2).
- **Dialoge und Startbildschirm schöner** (N11, P-DESIGN).
- **Leistung** bei Tausenden Objekten (P-PERF).
- Viele 🔧-Punkte oben sind gebaut, aber noch nicht im echten Fenster geprüft. Das macht der Nachtlauf (`docs\NACHPRUEFUNG.md`). Dein Test zählt genauso.

## D. Wie du Fehler meldest

Schreib dem Koordinator im Chat kurz:

1. die **Nr. bzw. Überschrift** aus diesem Leitfaden (z. B. „N19“ oder „5. Bildgröße“), bei neuen Fehlern einen Satz, was du gemacht hast;
2. was du **erwartet** hast und was **passiert** ist;
3. ein **Bildschirmfoto** (Win+Umschalt+S, einfügen), wenn möglich;
4. Sprache (de/en) und Design (Normal/Dunkel/OLED), falls es nur dort auftritt.

Neue Fehler bekommen die nächste Nr. ab N24 in `docs\NUTZERBEFUNDE.md`.

# Neu in dieser Testversion (13.10.2026) – seit der Version vom 12.10. 20:24

Alles hier ist mit automatischen Tests geprüft, aber **noch nicht im echten Fenster** durchgeklickt
(das machen die Nachttests). Wenn dir etwas auffällt: kurz beschreiben, gern mit Bild.

Der ausführliche Testleitfaden für alles bis 20:24 liegt weiterhin in `docs\TESTLEITFADEN_2026-10-12.md`.

## Deine Befunde

| Nr. | Was | So kommst du hin |
|---|---|---|
| N22 | Bildvorschau komplett (1/2/4 Felder, Vorgaben, Registerkarten Optionen/Datei/Animation, OK/Exportieren, Mausrad, Strg+Rad zoomt) | Datei ▸ Bildvorschau… |
| N23 | Nichts mehr abgeschnitten: Inspektor im kleinen Fenster mit dünner Bildlaufleiste, Dialoge passen sich ans Fenster an | Fenster klein ziehen (z. B. 1024 × 768), Inspektor und große Dialoge ansehen |
| N24 | Schriftliste mit sichtbarer Bildlaufleiste; Bildlaufleisten jetzt überall sichtbar | Text-Werkzeug ▸ Inspektor ▸ Schrift aufklappen |
| N11 | Neue Startseite (3 Karten, Vorschaubilder) und schönere Dialoge (Fußleiste, Titelleiste, Schatten) | App starten bzw. ein beliebiger Dialog; Ansicht ▸ Design: Normal/Dunkel/OLED |

## Dialoge

| Was | So kommst du hin |
|---|---|
| Kurven (Feld mit Raster, Punkte ziehen, Kanäle) | Filter ▸ Farbe anpassen ▸ Kurven… ; als Live-Effekt: Eigenschaften ▸ Filter **+** ▸ Farbe anpassen ▸ Kurven |
| Tonwert (Eingabe-/Ausgabetonwerte, Auto) | Filter ▸ Farbe anpassen ▸ Tonwertkorrektur… (auch als Live-Effekt) |
| Filter merken ihre Werte, Sättigung linear | Filter ▸ Farbe anpassen ▸ Farbton/Sättigung… zweimal öffnen |
| Bewegungsunschärfe / Radialer Weichzeichner | Filter ▸ Weichzeichnen ▸ … |
| Numerisch transformieren (Kette, Mitrechnen) | Modifizieren ▸ Transformieren ▸ Numerisch transformieren… |
| Vereinfachen, Strich erweitern, Pfad einlassen, **neu:** Pfad in Auswahlrahmen konvertieren | Modifizieren ▸ Pfad ändern ▸ … (Pfad in Auswahlrahmen konvertieren: Modifizieren ▸ Pfad in Auswahlrahmen konvertieren…) |
| Weiche Kante, Auswahlrahmen erweitern/verkleinern/begrenzen/glätten (Werte > 100 werden beim OK korrigiert) | Auswählen ▸ … (mit einer Pixelauswahl) |
| Bitmap-Auswahl speichern / wiederherstellen / löschen | Auswählen ▸ Bitmap-Auswahl speichern… |
| In Symbol konvertieren, Animieren | Modifizieren ▸ Symbol ▸ … bzw. Modifizieren ▸ Animation ▸ Auswahl animieren… |
| Neues Symbol / Neue Schaltfläche öffnen direkt die Symbolbearbeitung | Bearbeiten ▸ Objekt einfügen ▸ Neues Symbol… / Neue Schaltfläche… |
| Datei-Info (Titel, Autor, Beschreibung, Schlüsselwörter, Copyright; wird im Dokument gespeichert) | Datei ▸ Datei-Info… |
| Rechtschreibprüfung einrichten (Wörterbuchordner, Optionen; erkennt doppelte Wörter) | Text ▸ Rechtschreibprüfung einrichten… |
| Voreinstellungen, Tastaturbefehle, Gespeicherte Befehle | Bearbeiten ▸ Voreinstellungen… / Tastaturkurzbefehle… ; Befehle ▸ Gespeicherte Befehle verwalten… |
| Tab-Folge wie im Original, ▾-Schieber kein Tab-Stopp, Klick neben einen Dialog schließt ihn nicht mehr | beliebigen Dialog mit Tab durchgehen |

## Export

| Was | So kommst du hin |
|---|---|
| Exportieren als ein Ablauf (Ordner, Name, 10 Arten, Unterpaneele, Optionen…) | Datei ▸ Exportieren… |
| Optionen: PDF (inkl. Kennwortschutz), CSS-Sprites, Photoshop, HTML-Einrichtung | Datei ▸ Exportieren… ▸ Art wählen ▸ Optionen… |
| Exportassistent (Zurück/Weiter, endet in der Bildvorschau) | Datei ▸ Exportassistent… oder Optimieren-Bedienfeld ▸ Exportassistent… |
| HTML-Einrichtung, HTML-Code kopieren, HTML aktualisieren | Datei ▸ HTML-Einrichtung… ; Bearbeiten ▸ HTML-Code kopieren… ; Datei ▸ HTML aktualisieren… |
| Reduziertes PNG | Datei ▸ Speichern unter… ▸ Dateityp „Reduziertes PNG“ |
| Zusatzfelder im Dateidialog: „Als Unbenannt öffnen“, „Als Animation öffnen“, „.fw.png anhängen“ | Datei ▸ Öffnen… / Speichern unter… |

## Menüs

| Was | So kommst du hin |
|---|---|
| Zu hohe Menüs bleiben im Fenster, mit Pfeilen oben/unten (Verweilen, Klick, Mausrad, Tastatur) | Fenster klein ziehen, Menü Datei oder Fenster öffnen |
| Menüfreigabe wie im Original (z. B. Objektbefehle grau bei Pixelauswahl) | mit Pixelauswahl die Menüs ansehen |
| Bedienfeldmenüs (≡) wie im Original | ≡ oben rechts in Ebenen, Optimieren, Bibliothek … |
| Befehle-Menü: 5 neue Befehle (Autom. Vektormaske funktioniert), Skriptmeldungen sichtbar und auf Deutsch/Englisch | Befehle ▸ … |
| „Wiederholen: <Filter>…“ nach einem Filter | Filter einmal anwenden, dann Menü Filter |
| „Im Dateibrowser durchsuchen…“ (Strg+Alt+O) öffnet den Ordner des Dokuments | Datei ▸ Im Dateibrowser durchsuchen… |
| Schrift-Menü mit ✓ an der Familie | Text ▸ Schrift |
| Optimieren ▸ Einstellungen löschen fragt nach | Optimieren-Bedienfeld ▸ ≡ ▸ Einstellungen löschen… |

## Tempo

- Viele Objekte: „Alles auswählen“ und Pfeiltasten auf großen Seiten deutlich schneller (iPad-Beispiel: Bild 24 ms statt ~200 ms, Pfeiltaste 56 ms statt ~350 ms).

## Noch nicht drin / bekannt

- Batch-Verarbeitung (kommt später), Bildschirmfoto / Symbolskript erstellen / Gemeinsame Palette zeigen noch einen Hinweis.
- Datei-Info hat nur die Seite „Beschreibung“ (die Register des Originals konnten noch nicht gemessen werden).
- Technische Dateinamen (z. B. `allgemein.ftl`) werden im nächsten Schritt englisch.

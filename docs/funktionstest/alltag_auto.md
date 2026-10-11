# Alltagsabläufe mit realistischen Eingaben – automatischer Gesamttest

Erzeugt von `crates\fp-ui\tests\gesamt_alltag.rs` (`cargo test -p fp-ui --test gesamt_alltag`). Nicht von Hand ändern.

**28/28 Einträge ✅**

| Eintrag | Befehl | Ergebnis | Hinweis |
|---|---|---|---|
| Objekt mit dem Zeiger anklicken wählt es aus | `realistische Eingabe` | ✅ Ui | Rechteck ausgewählt |
| Umschalt+Klick erweitert die Auswahl | `realistische Eingabe` | ✅ Ui | 2 Objekte |
| Strg+C, Strg+V (Systemzwischenablage ohne Text) fügt eine Kopie ein | `realistische Eingabe` | ✅ Ui | eingefügt |
| Strg+C, Strg+V (Systemzwischenablage mit fremdem Text) fügt das Objekt ein | `realistische Eingabe` | ✅ Ui | eingefügt |
| Strg+X entfernt, Strg+V fügt wieder ein | `realistische Eingabe` | ✅ Ui | ausgeschnitten und eingefügt |
| Strg+Alt+D dupliziert (Strg+D hebt wie im Original die Auswahl auf) | `realistische Eingabe` | ✅ Ui | dupliziert, Strg+D hebt Auswahl auf |
| Pfeiltaste verschiebt um 1 px, Umschalt+Pfeil um 10 px, Tastenwiederholung 5× = 5 px | `realistische Eingabe` | ✅ Ui | Δ 1.0 / 10.0 / 5.0 |
| Objekt mit der Maus ziehen (20 Bilder) verschiebt um die Mausstrecke | `realistische Eingabe` | ✅ Ui | Δ (40.0, 25.0), erwartet (40.0, 25.0) |
| Strg+G gruppiert, Strg+Umschalt+G hebt auf | `realistische Eingabe` | ✅ Ui | gruppiert und aufgehoben |
| Text: T, Klick, „Hallo“ tippen, Esc → Textobjekt mit Hallo | `realistische Eingabe` | ✅ Ui | Text angelegt |
| Strg+Z macht den letzten Schritt rückgängig, Strg+Y wiederholt | `realistische Eingabe` | ✅ Ui | rückgängig/wiederholt |
| Strg+Mausrad: eine Rastung = eine Zoomstufe (100 % → 150 % bzw. 66 %) | `realistische Eingabe` | ✅ Ui | 100 % → 150 % → 66 % |
| Strg+Touchpad (10 kleine Schritte à 12 Punkte) zoomt höchstens wenige Stufen | `realistische Eingabe` | ✅ Ui | 100 % → 150 % |
| Mausrad ohne Strg rollt die Ansicht um ein Stück (kein Sprung) | `realistische Eingabe` | ✅ Ui | Bildlauf 40 Bildschirmpunkte |
| Dialog (Leinwandgröße) an der Titelleiste verschiebbar, OK übernimmt | `realistische Eingabe` | ✅ Ui | verschoben um (-120, 60), mit Eingabe geschlossen: true |
| Bilddatei auf die Leinwand ziehen importiert sie dort (linke obere Ecke), ein Verlaufsschritt, ausgewählt | `realistische Eingabe` | ✅ Ui | Dokumente 1 → 1, Schritte +1, Grenzen Some(Rect { left: 300.0, top: 250.0, right: 310.0, bottom: 258.0 }) |
| Datei außerhalb der Leinwand (Bedienfeld) ablegen öffnet sie als Dokument | `realistische Eingabe` | ✅ Ui | als Dokument geöffnet |
| Menü per Tastatur: Alt+D öffnet „Datei“, ↓ und Eingabe führen „Neu“ aus, Esc schließt | `realistische Eingabe` | ✅ Ui | Datei-Menü per Alt+D, „Neu“ per Eingabe |
| Menü per Tastatur: F10 aktiviert die Leiste, → wechselt, ↓ öffnet, Esc schließt stufenweise; Alt allein aktiviert | `realistische Eingabe` | ✅ Ui | F10 true · → Menü 1 · ↓ offen true · Esc true · Esc aktiv false · Alt allein aktiv true |
| Strg+Tab / Strg+F6 wechseln zum nächsten, Strg+Umschalt+Tab zum vorigen Dokument | `realistische Eingabe` | ✅ Ui | Dokumente [2, 3]: Some(3) → Some(2) → Some(3) → Some(2) |
| Viele Dokumente: Reiter laufen nicht aus dem Bild, „»“ zeigt weitere, aktiver Reiter sichtbar | `realistische Eingabe` | ✅ Ui | 8 von 19 Reitern sichtbar, im Fenster true, aktiver sichtbar true, „»“ vorhanden true |
| Rechtsklick auf einen Dokumentreiter öffnet dessen Kontextmenü | `realistische Eingabe` | ✅ Ui | Popup offen true, Einträge ["Alles hier zusammenführen", "Schließen", "Alle schließen", "In neues Fenster verschieben", "Neues Dokument", "Dokument öffnen"] |
| Fenstertitel wie im Original („*Name.fw.png“), Speichern/Wiederherstellen grau bei unverändertem Dokument, Wiederherstellen fragt nach | `realistische Eingabe` | ✅ Ui | Titel „gespeichert.fw.png“ → „*gespeichert.fw.png“, Speichern unverändert Some(false), geändert Some(true), Rückfrage „Letzte gespeicherte Version von "gespeichert.fw.png" wiederherstellen?“ |
| Start: in [Seiten, Status, Ebenen] ist Ebenen vorn | `realistische Eingabe` | ✅ Ui | Ebenen vorn |
| Fremden Text einfügen (Strg+V, Text in der Systemzwischenablage) legt ein Textobjekt 12 pt mittig im sichtbaren Teil an | `realistische Eingabe` | ✅ Ui | Mitte Text (400, 300), sichtbare Mitte (400, 300), Text im Dokument true |
| Doppelklick auf das Zoom-Werkzeug = 100 %, auf die Hand = alles einpassen | `realistische Eingabe` | ✅ Ui | 300 % → Zoom-Doppelklick 100 %; 300 % → Hand-Doppelklick 117 % (eingepasst) |
| Strg+S speichert (Dateiname über den Dialog) | `realistische Eingabe` | ✅ Ui | gespeichert |
| Doppelklick auf Text öffnet die Textbearbeitung | `realistische Eingabe` | ✅ Ui | Text-Editor aktiv |

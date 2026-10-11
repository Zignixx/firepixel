# UX-Abgleich B: Bedienfelder, Dialoge, Fenster, Menüs, Datei-Abläufe

Agent **UX-B**, Stand 11.10.2026 abends. Ziel ist die Frage des Nutzers, ob sich Firepixel im Kleinen genauso bedient wie das Original.
Jedes Szenario steht in einer Zeile: **Ausgangszustand → Aktion → beobachtbares Ergebnis**, je einmal für das Original und für Firepixel.

**Status**
- ✅ gleich
- ⚠️ kleine Abweichung
- ❌ anders oder fehlt
- ❔ Original noch nicht belegt (siehe „Offen“ am Ende)

**Belege**
- **GUI:** echte Eingaben im Original unter der GUI-Sperre, Bilder `re\_testdateien\ux\…`.
- **DLG:** Dialogbericht aus dem Original. Ein JSF-Aufruf öffnet den Dialog, danach werden Fensterklassen, Steuerelemente in Tab-Folge, Standardknopf, Fokus, Markierung und Feldwerte über die Win32-API ausgelesen. Dateien: `re\_testdateien\ux\o_dlg1.txt`, `o_dlg2.txt`, Skript `re\_testdateien\ux\skripte\uxb_jsf_dlg.ps1`.
- **TITEL:** Fenstertitel des Originals über die Prozessliste.
- **MSG:** Fensternachrichten an die Win32-Steuerelemente des Originals (Klick, Zeichen, Tasten, Rad als `PostMessage`, danach Wert, Markierung und Fokus lesen). Skript `re\_testdateien\ux\skripte\o_pi_msg.ps1`. Werteingaben per gesendeten Zeichen waren dabei nicht zuverlässig, deshalb zählen nur Fokus, Markierung und „keine Wirkung“.
- **SPEZ:** `re\11-bedienfelder-dialoge.md`.
- **WIN:** Windows-Standardverhalten (MFC/Win32), das das Original nachweislich nutzt (Dialogklasse `#32770`, MDI-Kindfenster). Bei Bedarf noch per GUI zu bestätigen.
- **FP:** Firepixel-Sonde `crates\fp-ui\tests\ux_b_abgleich.rs`. Sie arbeitet mit realistischen Ereignissen: Einzelbilder mit 1/60 s, Doppelklick innerhalb von 0,3 s, Modifikatoren wie egui-winit. Ausgabe `UXB <Nr>`, Bilder `docs\screenshots\phase5_uxb_*.png`.
- **CODE:** Lesen des Firepixel-Quelltexts.

**Hinweis zur Fernsteuerung**
- Echte Eingaben im Original klappten nur zeitweise. Um 19:52 gingen sie: Alt+D öffnete das Datei-Menü, Bild `re\_testdateien\ux\uxb_s1_b.png`.
- Danach meldeten Screenshots „Das Handle ist ungültig“ und SendKeys „Zugriff verweigert“. Vermutlich war das RDP-Fenster minimiert.
- Dialoge, Rückfragen und Fenstertitel ließen sich trotzdem vollständig über die Win32-API auslesen (DLG, TITEL).
- Was nur per Maus prüfbar ist (Bedienfeld-Klicks, Reiter, Menü-Tastatur im Detail), steht als ❔ unter „Offen“.

## Zusammenfassung

| | Anzahl |
|---|---:|
| Szenarien | 195 (dazu 3 ➖, schon an andere Agenten vergeben) |
| ✅ gleich | 85 |
| ⚠️ kleine Abweichung | 21 |
| ❌ anders/fehlt | 37 |
| ❔ Original offen | 52 |

Befunde stehen in `docs\BEFUNDE.md` unter IBD und HG, jeweils mit „(von UX-B)“.

---

## A. Dialoge: Grundverhalten (D)

| Nr | Ausgangszustand → Aktion | Original | Firepixel | St. | Beleg |
|---|---|---|---|---|---|
| D01 | Dokument offen → Datei ▸ Neu (Strg+N) | Dialog „Neues Dokument“ (Titel mit Leerzeichen am Ende) | Dialog „Neues Dokument“ | ✅ | DLG D1, FP D01 |
| D02 | Neues Dokument gerade geöffnet → nichts | Tastaturfokus im Feld **Breite**, Inhalt **ganz markiert** (sel=0-3) | **kein** Fokus im Dialog | ❌ | DLG o_dlg2 D2, FP D02 |
| D03 | wie D02 → sofort „800“ ⇥ „600“ tippen | Breite 800, Höhe 600; Tab springt Breite→Höhe→Auflösung→Einheiten→Weiß→OK→Abbrechen→Vorlagen | Eingaben gehen verloren (Breite 660, Höhe 440) | ❌ | DLG (Z-Folge), FP D03 |
| D04 | Neues Dokument → Enter | OK (Standardknopf, BS_DEFPUSHBUTTON) | 1. Lauf: OK. **Nachprüfung 21:20: Enter ohne Feldfokus bestätigt nicht mehr** (Rückschritt) | ❌ | DLG, FP D04 |
| D05 | Neues Dokument mit 321×123 bestätigt → erneut öffnen | Dialog zeigt die **zuletzt benutzten** Werte (beobachtet 50×30, dann 660×440 nach fremden Läufen) | immer `DefaultDocSize` 660×440 | ❌ | DLG D1/D2, FP D05b |
| D06 | Neues Dokument → Esc | schließt ohne Dokument | schließt ohne Dokument | ✅ | WIN, FP D06/D07 |
| D07 | Neues Dokument, Gruppentitel | „Leinwandgröße: 5.9K“ bzw. „1.1M“ (K/M, Punkt) | „Leinwandgröße: 1,1 MB“ | ⚠️ | DLG |
| D08 | Neues Dokument → Knopf „Vorlagen…“ | aktiv (öffnet Neu von Vorlage) | gesperrt | ❌ | DLG (TAB, nicht AUS), CODE new_document.rs:151 |
| D09 | Neues Dokument → Alt+B / Alt+H / Alt+A (Zugriffstasten „&Breite“ …) | springt ins Feld | keine Zugriffstasten, kein Unterstrich | ❌ | DLG (&-Beschriftungen), CODE |
| D10 | Neues Dokument → Einheiten-Kombifeld | Pixel; Zoll; Zentimeter bzw. Pixel/Zoll; Pixel/cm | gleich | ✅ | DLG, CODE |
| D11 | Neues Dokument → Lage beim Öffnen | mittig im Hauptfenster | mittig | ✅ | DLG Rechteck, FP DLG |
| D12 | Bildgröße (Strg+Alt+I) geöffnet → Fokus | erstes Feld fokussiert (MFC-Standard) | kein Fokus | ❌ | WIN, FP DLG imageSize |
| D13 | Leinwandgröße geöffnet → Fokus | erstes Feld fokussiert | kein Fokus | ❌ | WIN, FP DLG canvasSize |
| D14 | Numerisch transformieren geöffnet → Fokus/Tab | Fokus auf Kombifeld „Skalieren“; Tab: Kombi→B→H→Attribute skalieren→Proportionen→OK→Abbrechen | kein Fokus | ❌ | DLG D6, FP DLG numericTransform |
| D15 | Numerisch transformieren → Inhalt | Skalieren/Größe ändern/Drehen, 100 %, B/H-Anzeige 280, beide Kästchen an | gleiche Felder | ✅ | DLG D6, CODE |
| D16 | Raster bearbeiten geöffnet | Fokus im ersten Abstandsfeld (36 Pixel) | kein Fokus | ❌ | DLG D7, FP DLG editGrid |
| D17 | Gaußscher Weichzeichner geöffnet → Fokus | erstes Feld (Radius) | kein Fokus | ❌ | WIN, FP DLG gaussian |
| D18 | Farbton/Sättigung, Weiche Auswahlkante, Neue Ebene geöffnet → Fokus | erstes Feld | kein Fokus | ❌ | WIN, FP DLG |
| D19 | beliebiger Filterdialog → Esc | Abbrechen, Vorschau weg | Abbrechen, Vorschau zurückgenommen (Verlauf 6→5) | ✅ | FP DLG gaussian/hueSat |
| D20 | alle geprüften Dialoge → Lage beim ersten Öffnen | mittig im Hauptfenster | mittig | ✅ | DLG, FP DLG (12 Dialoge) |
| D21 | Dialog verschoben, geschlossen, wieder geöffnet | erscheint **wieder mittig** (Neues Dokument, Raster bearbeiten: um +300/+150 verschoben → nächstes Öffnen wieder bei der Ausgangslage) | Lage für die Sitzung gemerkt | ⚠️ | DLG `re\_testdateien\ux\skripte\o_dlg_lage2.ps1`, CODE dialog_frame |
| D22 | Dialog → Titelleiste ziehen | verschiebbar | verschiebbar (Befund des Nutzers, IBD behoben) | ✅ | CODE mod.rs:270 |
| D23 | Dialog mit gesperrtem Knopf (z. B. OK bei ungültiger Eingabe) | Knopf **grau** gezeichnet | Knopf sieht aktiv aus, Klick wirkt nicht (`push_button` zeichnet `enabled=false` nicht grau) | ❌ | CODE mod.rs:366 |
| D24 | Dialog → Tab mehrfach | Fokus wandert in Z-Folge reihum, nur über TAB-Elemente | egui-Fokusfolge = Zeichenreihenfolge; Kombifelder (`Sense::click` ohne Fokus) und eigene Knöpfe teils nicht erreichbar | ⚠️ | DLG, CODE fields.rs |
| D25 | Dialog, Knopf mit Tab fokussiert → Leertaste | löst den Knopf aus | egui-Knopf: ja; eigene Felder: nein | ⚠️ | WIN, CODE |
| D26 | Dialog, „Abbrechen“ fokussiert → Enter | löst **Abbrechen** aus (fokussierter Knopf wird Standard) | globales Enter löst OK **und** Abbrechen aus; Ergebnis hängt von der Reihenfolge ab (Abbrechen gewinnt) | ⚠️ | WIN, CODE |
| D27 | Eingabe-Dialog (`prompt`) → Öffnen | Titel „[JavaScript]“, Fokus im Feld, Vorgabe ganz markiert, OK/Abbrechen | Text-Dialoge (z. B. Arbeitsbereich speichern) setzen Fokus | ⚠️ | DLG o_dlg2 D6, CODE mod.rs:444 |
| D28 | Exportieren (Strg+Umschalt+R) | **Windows-Dateidialog** „Exportieren“: Dateiname, Kombi „Exportieren:“ (PDF; HTML und Bilder; **Nur Bilder**; Dreamweaver-Bibliothek; CSS und Bilder; Director; CSS-Sprites; Ebenen/Status/Seiten in Dateien; FXG und Bilder; MXML und Bilder; Lotus Domino Designer), Startordner = Dokumentordner | eigener egui-Dialog „Exportieren“ | ⚠️ | DLG D8, CODE export.rs |
| D29 | Bildvorschau (Strg+Umschalt+X) | ❔ | eigener Dialog, zentriert, Esc schließt | ❔ | FP DLG imagePreview |
| D30 | Voreinstellungen (Strg+U) | ❔ Kategorienliste links, erste Kategorie aktiv | Liste links, Esc schließt | ❔ | FP DLG prefs |
| D31 | Tastaturbefehle → Esc | ❔ | schließt | ❔ | FP DLG shortcuts |

## B. Meldungen und Rückfragen (M)

| Nr | Ausgangszustand → Aktion | Original | Firepixel | St. | Beleg |
|---|---|---|---|---|---|
| M01 | geändertes Dokument → Schließen (Strg+W, Reiter-×) | Meldungsfenster, Titel = Programmname | eigenes Fenster, Titel „Firepixel“ | ✅ | DLG D2 |
| M02 | wie M01: Wortlaut | „Änderungen an uxb_doc1.fw speichern?“ (Dokumentname **ohne letzte Endung**, ohne Anführungszeichen) | „Möchten Sie die Änderungen an „Unbenannt-1.png“ vor dem Schließen speichern?“ | ❌ | DLG D2, FP M02 |
| M03 | wie M01: Knöpfe | Ja / Nein / Abbrechen (Windows-Systemsprache; hier englisch „&Yes/&No/Cancel“), Ja = Standard und fokussiert | Ja / Nein / Abbrechen, Ja = Standard, kein Tastaturfokus | ⚠️ | DLG D2, CODE unsaved.rs |
| M04 | wie M01 → Esc | Abbrechen, Dokument bleibt | Abbrechen, Dokument bleibt | ✅ | FP M03 |
| M05 | wie M01 → Symbol | Systemsymbol (Static id 20, 32×32); Art per Bild noch zu prüfen | Info-Symbol | ❔ | DLG D2 |
| M06 | wie M01 → N bzw. J (Zugriffstaste) | „&No“ / „&Yes“ (deutsches Windows: „&Nein“/„&Ja“) | keine Zugriffstasten | ❌ | DLG D2 |
| M07 | Datei ▸ Wiederherstellen | Rückfrage „Letzte gespeicherte Version von "uxb_doc1.fw.png" wiederherstellen?“ OK/Abbrechen, OK = Standard | **keine Rückfrage**, stellt sofort wieder her | ❌ | DLG D5, CODE actions.rs (kein Fall für file.revert) |
| M08 | Skript `alert("x")` | Meldungsfenster mit Programmname, Text mit führender Leerzeile, nur OK | Meldungsdialog | ⚠️ | DLG D10 |
| M09 | Skript `confirm("x")` | OK / Abbrechen, OK Standard | ❔ JSF-Laufzeit | ❔ | DLG D9 |
| M10 | Rückfrage-Dialog (Confirm, z. B. Löschen) → Enter | Standardknopf (Ja/OK) | nachgeprüft 21:20: Enter = Ja | ✅ | WIN, FP M10 |
| M11 | Meldung mit langem Text | linksbündig umbrochen | Blocksatz (Nutzerbefund, an IBD) | ➖ | – |
| M12 | Programm beenden mit 2 geänderten Dokumenten | je Dokument eine Rückfrage | je Dokument eine Rückfrage | ✅ | CODE unsaved.rs |
| M13 | Rückfrage „Ja“ bei nie gespeichertem Dokument | Speichern unter, Abbruch dort bricht Schließen ab | dasselbe | ✅ | CODE unsaved.rs:92 |
| M14 | Meldung „Nicht mehr anzeigen“ | Kästchen in Warnungen 6016/6020/5992 | ❔ | ❔ | SPEZ §6 |
| M15 | Datei konnte nicht geöffnet werden | ❔ Wortlaut | „„x“ konnte nicht geöffnet werden: …“ | ❔ | CODE files.rs:132 |

(M11 ist schon an IBD vergeben und wird nicht mitgezählt.)

## C. Datei-Abläufe (F)

| Nr | Ausgangszustand → Aktion | Original | Firepixel | St. | Beleg |
|---|---|---|---|---|---|
| F01 | Datei ▸ Öffnen | Windows-Dateidialog „Öffnen“, Startordner = Ordner des aktiven Dokuments | nativer Dialog, Startordner = Dokumentordner | ✅ | DLG D4 (Ordner „ux“), CODE actions.rs:149 |
| F02 | Öffnen → Dateityp-Filter | ❔ für Datei ▸ Öffnen (bei `browseForFileURL` nur „Alle Dateien (*.*)“) | „Alle lesbaren Formate“ enthält **nur png/fw.png**; JPEG, GIF, PSD, WebP … erst unter „Alle Dateien“ | ❌ | DLG D4, CODE files.rs:12 |
| F03 | Öffnen → Mehrfachauswahl | ❔ | möglich | ❔ | CODE |
| F04 | Datei schon offen → noch einmal öffnen | ❔ (Reiter wird aktiviert) | Reiter wird aktiviert | ❔ | CODE files.rs:126 |
| F05 | Speichern unter (Strg+Umschalt+S) | Windows-Dialog, Startordner = Dokumentordner, Dateiname vorbelegt | gleich (Vorschlag „x.fw.png“) | ✅ | DLG D3, CODE |
| F06 | Speichern unter → Dateitypen | 11 Typen: FW-PNG (*.png), BMP, GIF, Animiertes GIF, Illustrator 8, JPEG, Flash SWF, Reduziertes PNG, Photoshop PSD, TIFF, WBMP | nur „FW-PNG (*.fw.png)“ | ❌ | DLG D3 |
| F07 | Speichern unter → Zusatzoptionen | „Optionen…“, Kästchen „.fw.png anhängen“, „Als Kopie speichern“, „XMP einschließen“ | keine | ❌ | DLG (Speichern-unter-Kinder) |
| F08 | unverändertes Dokument → Datei-Menü | „Speichern“ **grau** | aktiv (`file.save` verlangt nur ein Dokument) | ❌ | GUI uxb_s1_b.png, CODE actions.rs:35/file.rs |
| F09 | Dokument ohne Pfad → Strg+S | Speichern unter | Speichern unter | ✅ | CODE files.rs:105 |
| F10 | Exportieren → Startordner/Name | Dokumentordner, Name = Dokumentname | ❔ | ❔ | DLG D8 |
| F11 | Importieren (Strg+R) | Windows-Dateidialog, danach Platzierungs-Cursor (Klick setzt, Ziehen skaliert) | ❔ Dateidialog „Importieren“, Platzierung prüfen | ❔ | CODE actions.rs:161 |
| F12 | Datei ▸ Zuletzt geöffnete Dateien, Liste leer | ❔ | Untermenü grau („Keine“) | ❔ | CODE menu.rs:186 |
| F13 | Zuletzt geöffnete Dateien, Einträge | ❔ Darstellung (Nummer, Pfad gekürzt?) | „1 C:\voller\Pfad\datei.fw.png“, max. 10 | ❔ | CODE menu.rs:188 |
| F14 | Datei ▸ Schließen (Strg+W) ungeändert | schließt ohne Rückfrage | schließt ohne Rückfrage | ✅ | CODE unsaved.rs:49 |
| F15 | Alle schließen, eins geändert | eine Rückfrage | eine Rückfrage | ✅ | CODE |
| F16 | Wiederherstellen ungeändert | Menü grau | aktiv, sobald ein Dateipfad besteht | ❌ | GUI uxb_s1_b.png, CODE file.rs (HasPath) |
| F17 | Neu von Vorlage | Dateidialog im Vorlagenordner | Dateidialog ohne Vorlagenordner | ⚠️ | CODE actions.rs:156 |
| F18 | Datei-Info (Strg+Alt+Umschalt+F) | Dialog | Dialog | ✅ | CODE menu_dialogs |
| F19 | Verlauf ▸ „Als Befehl speichern“ | Namensabfrage „Befehl speichern“ (Vorlage 5968), legt den Befehl im Befehle-Menü ab | Windows-**Dateidialog** | ❌ | SPEZ §3.3, CODE history.rs |
| F20 | Datei ziehen auf das Fenster | Nutzerbefund (an HG) | – | ➖ | – |

## D. Menüs (ME)

| Nr | Ausgangszustand → Aktion | Original | Firepixel | St. | Beleg |
|---|---|---|---|---|---|
| ME01 | Hauptfenster → Alt+D | öffnet „Datei“ | nichts | ❌ | GUI uxb_s1_b.png, FP ME01 |
| ME02 | Hauptfenster → F10 bzw. Alt allein | aktiviert die Menüleiste (Windows) | nichts | ❌ | WIN, FP ME02 |
| ME03 | Menü offen → ↓ / ↑ | Eintrag wandern, graue übersprungen | egui-Menüs ohne Pfeiltasten | ❌ | WIN, CODE menu.rs |
| ME04 | Menü offen, auf Untermenü → → | Untermenü öffnet | nein | ❌ | WIN, CODE |
| ME05 | Menü offen → ← / → in der Leiste | Nachbarmenü | nein | ❌ | WIN |
| ME06 | Menü offen → Enter | führt aus | nein | ❌ | WIN |
| ME07 | Menü offen → Esc | schließt Untermenü bzw. Menü | Esc schließt das Menü (egui) | ⚠️ | WIN, CODE |
| ME08 | Menü offen → Buchstabe (Zugriffstaste) | führt Eintrag aus | nein; Labels ohne Unterstrich | ❌ | menu_main.json (&-Zeichen), CODE clean_label |
| ME09 | Menü offen → Maus über andere Menüleisten-Titel | Nachbarmenü öffnet | dasselbe | ✅ | CODE menu.rs:504 |
| ME10 | Datei-Menü: Kürzel rechts | „Strg+N“, „Strg+O“, „Strg+W“, „Strg+Umschalt+S“ … aus dem Kürzelsatz | aus dem Kürzelsatz | ✅ | GUI uxb_s1_b.png, CODE |
| ME11 | Datei-Menü, Reihenfolge/Trenner | Neu, Neu von Vorlage, Öffnen, Bridge, Zuletzt ▸, Scannen ▸, Schließen, Alle schließen │ Speichern … │ Importieren … │ Datei-Info │ Exportassistent … │ Seite einrichten … │ Beenden | aus derselben Ressource | ✅ | GUI, menu_main.json |
| ME12 | nicht verfügbare Einträge | grau | grau | ✅ | GUI, CODE |
| ME13 | Untermenü ohne aktive Einträge | ❔ | ganzes Untermenü grau, öffnet nicht | ❔ | CODE menu.rs:130 |
| ME14 | Häkchen (Ansicht ▸ Lineale, Fenster ▸ Ebenen) | Häkchen links | Häkchen links | ✅ | CODE |
| ME15 | Fenster-Menü → Dokumentliste | ❔ „1 Name“ mit Häkchen beim aktiven | „1 *Name“ mit Häkchen | ❔ | CODE menu.rs:345 |
| ME16 | Fenster-Menü → Bedienfeld anklicken, das offen ist | ❔ (schließt bzw. holt nach vorn) | schaltet um | ❔ | CODE |
| ME17 | Ansicht-Menü → „Design“ | gibt es nicht (Firepixel-Zusatz) | Untermenü „Design“ | ⚠️ | CODE (bewusste Erweiterung) |
| ME18 | Befehle-Menü → „KI-Steuerkanal“ | gibt es nicht | Eintrag | ⚠️ | CODE (bewusste Erweiterung) |
| ME19 | Hilfe-Menü | ❔ | ❔ | ❔ | – |
| ME20 | Rechtsklick auf Menüleiste | ❔ | nichts | ❔ | – |
| ME21 | Menü offen → Klick daneben | schließt, Klick wirkt **nicht** auf die Leinwand | schließt (egui), Leinwand prüft `hovered` | ✅ | CODE canvas |
| ME22 | Menüeintrag mit „…“ | öffnet Dialog | öffnet Dialog | ✅ | CODE |
| ME23 | schmales Fenster | Menüleiste bricht um bzw. kürzt | Überlaufmenü „»“ | ⚠️ | CODE menu.rs:475 |
| ME24 | Text ▸ Schrift | ❔ | Untermenü nach Anfangsbuchstaben | ❔ | CODE |
| ME25 | Filter-Menü, Einträge der Zusatzmodule | ❔ | feste Gruppen | ❔ | CODE |

## E. Ebenen-Bedienfeld (E)

| Nr | Ausgangszustand → Aktion | Original | Firepixel | St. | Beleg |
|---|---|---|---|---|---|
| E01 | Ebene „Ebene 1“ → Doppelklick auf den Namen, „Neu“, Enter | Inline-Feld mit Fokus, Name **ganz markiert** („Ebene Zwei“ sel=0-10), Esc schließt | Umbenennen-Feld öffnet in der Sonde **nicht** (Name bleibt) | ❌ | MSG `o_ebenen_msg.txt`, FP E01/E01a |
| E02 | wie E01, aber Esc | Name bleibt | Name bleibt | ✅ | FP E02 |
| E03 | wie E01, Klick daneben | übernimmt | – (Feld öffnet nicht) | ❔ | FP E03 |
| E04 | unbenanntes Objekt („Rechteck“) → Doppelklick | Inline-Feld **leer** (Typname wird nicht vorbelegt) | Feld öffnet nicht; Code würde „Rechteck“ vorbelegen | ❌ | MSG, FP E04, CODE layers.rs:209 |
| E05 | Ebene → Doppelklick auf das Ordnersymbol bzw. Zeilenrest | ❔ Ebenenoptionen (5987) | Umbenennen | ❔ | SPEZ §3.3 „Doppelklick auf eine Ebene (A)“ |
| E06 | Auge einer Ebene → Klick | blendet aus/ein, Verlaufsschritt | blendet aus/ein | ✅ | FP E10-Variante |
| E07 | Auge → **Alt+Klick** | ❔ (alle anderen Ebenen aus/ein?) | nur diese Ebene | ❔ | FP E10 |
| E08 | Augenspalte über mehrere Ebenen **ziehen** | schaltet **alle überstrichenen** Ebenen um (Ebene Zwei und Ebene 1 aus) | nichts, nicht einmal die erste Ebene | ❌ | MSG `o_ebenen_msg2.txt`, FP E11 |
| E09 | Schloss-Spalte → Klick | sperrt/entsperrt | sperrt/entsperrt | ✅ | CODE layers.rs:160 |
| E10 | Ebenenzeile → Klick | Ebene wird aktiv **und alle ihre Objekte werden ausgewählt** (Ebene 1 → 2 Objekte) | nachgeprüft 21:20: aktiv, 2 Objekte ausgewählt | ✅ | MSG `o_ebenen_msg2.txt`, FP E13 |
| E11 | Objektzeile → Klick, dann Umschalt+Klick | 2 Objekte gewählt | 2 Objekte | ✅ | FP E14 |
| E12 | Objektzeile → Strg+Klick | ❔ | wie Umschalt (umschalten) | ❔ | CODE layers.rs:103 |
| E13 | Ebene über andere ziehen | Einfügelinie, Reihenfolge geändert | Einfügelinie, Reihenfolge geändert | ✅ | FP E15 |
| E14 | Objekt in andere Ebene ziehen | verschiebt | verschiebt | ✅ | CODE |
| E15 | Ebene/Objekt → Rechtsklick | Kontextmenü = Ebenenmenü 6923 (Neues Bild, Neue Ebene…, … Miniaturoptionen…, mit grauen Einträgen je Lage) | **kein** Kontextmenü | ❌ | MSG `o_ebenen_msg.txt`, FP E12 |
| E16 | Objekt im Bedienfeld gewählt → Entf | löscht das Objekt | löscht | ✅ | FP E16, IBD panels_keyboard |
| E17 | Alt+Klick auf „Neu/Duplizieren“ | dupliziert die Ebene | dupliziert | ✅ | CODE layers.rs:286 |
| E18 | Löschen-Knopf ohne Objektauswahl | löscht die aktive Ebene | löscht die aktive Ebene | ✅ | CODE |
| E19 | Deckkraft-Feld oben → 50 + Enter | ändert die Auswahl | ändert | ✅ | CODE |
| E20 | Webebene → Doppelklick | ❔ | kein Umbenennen | ❔ | CODE |
| E21 | Ebene ausklappen (Dreieck) | klappt | klappt | ✅ | CODE |
| E22 | Ausklappzustand nach Neustart / Dokumentwechsel | im Dokument gespeichert (`layer.setExpanded`, Befund AU) | nur egui-Temp, nach Neustart offen | ⚠️ | CODE layers.rs:96 |
| E23 | Bedienfeld-Fußleiste „Status 1“ → Klick | Menü der Status | Menü der Status | ✅ | CODE |
| E24 | Ebenenmenü (≡) | 6923, alle Einträge | 6923 + Hilfe/Schließen/Gruppe schließen | ✅ | CODE build_panel_menu |
| E25 | lange Ebenennamen | mit „…“ gekürzt | mit „…“ gekürzt | ✅ | CODE text_clipped |

## F. Weitere Bedienfelder (P)

| Nr | Ausgangszustand → Aktion | Original | Firepixel | St. | Beleg |
|---|---|---|---|---|---|
| P01 | Seiten: Klick auf Seite | wechselt die Seite | wechselt | ✅ | CODE pages.rs |
| P02 | Seiten: Doppelklick | umbenennen inline | umbenennen inline | ✅ | CODE pages.rs:79 |
| P03 | Seiten: Name mit `\ / : * ? " < > \|` | abgelehnt (String 104) | ❔ | ❔ | SPEZ §2.2 |
| P04 | Seiten: Alt+Klick auf Neu | dupliziert | dupliziert | ✅ | CODE pages.rs:85 |
| P05 | Seiten: Löschen bei 1 Seite | gesperrt | gesperrt | ✅ | CODE |
| P06 | Seiten: Fußleiste „n Seiten“ | „1 Seite“/„n Seiten“ | gleich | ✅ | SPEZ 4272, CODE |
| P07 | Status: Klick | wechselt den Status | wechselt | ✅ | CODE states.rs |
| P08 | Status: Doppelklick auf Verzögerung | Windoid 6041 (Verzögerung, Beim Export einschließen) | Windoid | ✅ | CODE states.rs:105 |
| P09 | Status: Doppelklick auf Namen | umbenennen | umbenennen | ✅ | CODE |
| P10 | Status: Rechtsklick | Kontextmenü Löschmethode 6922 | Kontextmenü 6922 | ✅ | CODE states.rs:90 |
| P11 | Status: Verzögerung 10000 eintippen | 0..9999 (DDV) | ❔ | ❔ | SPEZ §2.2 |
| P12 | Verlauf: Klick auf den Schieber eines Schritts | springt zurück | springt | ✅ | CODE history.rs |
| P13 | Verlauf: Schieber ziehen | springt mit | springt mit | ✅ | CODE |
| P14 | Verlauf: Umschalt/Strg-Auswahl, „Wiederholen“ | wiederholt die Schritte | wiederholt | ✅ | CODE |
| P15 | Verlauf: nicht wiederholbarer Schritt | rotes X bzw. Trennlinie | rotes × | ✅ | SPEZ, CODE |
| P16 | Verlauf: Schritte kopieren | JS in die Zwischenablage | JS in die Zwischenablage | ✅ | CODE |
| P17 | Farbfelder: Klick | setzt die Farbe des aktiven Farbkästchens bzw. der Auswahl | setzt Farbe | ✅ | CODE color.rs |
| P18 | Farbfelder: Strg+Klick | löscht das Farbfeld | löscht | ✅ | CODE color.rs:110 |
| P19 | Farbfelder: Umschalt+Klick | ❔ (ersetzt das Farbfeld?) | Farbe auf das andere Kästchen | ❔ | CODE |
| P20 | Farbfelder: Klick in leere Fläche | ❔ (fügt aktuelle Farbe hinzu) | ❔ | ❔ | – |
| P21 | Farbfelder: Tooltip | ❔ | „#RRGGBB“ | ❔ | CODE |
| P22 | Stile: Klick bei Auswahl | wendet an | wendet an | ✅ | CODE styles.rs |
| P23 | Stile: Doppelklick | Stiloptionen/Umbenennen | Doppelklick-Aktion | ✅ | CODE styles.rs:102 |
| P24 | Optimieren: Farbtabelle Doppelklick | Farbe bearbeiten | Farbe bearbeiten | ✅ | CODE optimize.rs:432 |
| P25 | Optimieren: Rechtsklick Farbtabelle | Menü 6942 | Menü 6942 | ✅ | CODE |
| P26 | Bibliothek: Doppelklick Symbol | Symboleigenschaften | Doppelklick-Aktion | ✅ | CODE library.rs |
| P27 | Bedienfelder: Tastaturfokus | Bedienfelder halten keinen Fokus, Kürzel gehen ans Dokument | gleich (IBD) | ✅ | IBD panels_keyboard.rs |
| P28 | Standardgruppe Seiten/Status/Ebenen beim Start | **Ebenen** vorn | **Seiten** vorn | ⚠️ | GUI uxb_s1_b.png, FP phase5_uxb_zwei_dokumente.png |
| P29 | Info-Bedienfeld: B/H/X/Y eintippen | editierbar, verschiebt/skaliert | ❔ | ❔ | SPEZ §2.2 |

## G. Inspektor (I)

| Nr | Ausgangszustand → Aktion | Original | Firepixel | St. | Beleg |
|---|---|---|---|---|---|
| I01 | Objekt (B=100) gewählt → **einmal** ins B-Feld klicken | Schreibmarke an der Klickstelle, **nichts markiert** (EM_GETSEL 2-2), Tippen fügt ein | Schreibmarke, nichts markiert | ✅ | MSG `re\_testdateien\ux\o_pi_msg.txt`, FP I01a |
| I02 | Doppelklick ins B-Feld, „150“, Tab | 150, Tab → H-Feld | 150, Tab → H-Feld | ✅ | FP I02 |
| I03 | Feld bearbeitet → Esc | verwirft | verwirft | ✅ | FP I03 |
| I04 | Mausrad über B-Feld | keine Wirkung (WM_MOUSEWHEEL an das Feld) | keine Wirkung | ✅ | MSG, FP I04 |
| I05 | Pfeil ↑ im Feld | keine Wertänderung | keine Wirkung | ✅ | MSG, FP I05 |
| I06 | Deckkraft „150“ + Enter | ❔ (Feld zeigt nach WM_SETTEXT weiter „150“; ob Enter klemmt, war per Nachricht nicht prüfbar) | 100 | ❔ | MSG `o_pi_msg2.txt`, FP I06 |
| I21 | B-Feld „100150“ + Enter | Feld zeigt danach **„6000“** (Obergrenze sichtbar geklemmt; Übernahme per Nachricht nicht prüfbar) | klemmt still auf **30000** | ⚠️ | MSG `o_pi_msg2.txt`, FP I01 |
| I07 | Enter im Feld | übernimmt, Fokus geht an die Dokumentansicht | übernimmt, Fokus verlässt das Feld | ✅ | MSG (Fokus danach Ansichtsfenster), FP I01 |
| I08 | Mehrfachauswahl mit verschiedenen B | ❔ Feld leer | Feld leer (gemischt) | ❔ | CODE fields.rs:number |
| I09 | Mehrfachauswahl: Typtext | „3 Objekte“ | „3 Objekte“ | ✅ | FP phase5_uxb_auswahl_alle.png |
| I10 | Deckkraft-Popup-Schieber ziehen | ❔ Live-Änderung auf der Leinwand | übernimmt erst beim **Loslassen** | ❔ | CODE fields.rs:slider_arrow |
| I11 | Ungültige Eingabe „abc“ | ❔ | Wert bleibt | ❔ | CODE |
| I12 | Dezimalzahl „12,5“ in Pixel-Feld | ❔ | 12,5 (Komma/Punkt) | ❔ | CODE parse_num |
| I13 | Füllfarbe-Kästchen → Klick | Farb-Popup mit Farbfeldern, Pipette, Hex-Feld | Popup | ✅ | CODE popups.rs |
| I14 | Farb-Popup → Pipette über Bildschirm | nimmt Farbe überall | ❔ | ❔ | – |
| I15 | Farb-Popup → Esc | schließt ohne Änderung | ❔ | ❔ | – |
| I16 | Inspektor einklappen (halbe Höhe) | Menü „Halbe Höhe“ | ❔ | ❔ | SPEZ §2.3.5 |
| I17 | Zeiger ohne Auswahl | „Dokument“, Leinwand, Leinwandgröße…, Bildgröße…, Leinwand anpassen, Exportvorgabe, Status | gleich | ✅ | SPEZ 4a T4, FP Bild |
| I18 | Rechteck gewählt | Rundung-Feld | ❔ | ❔ | SPEZ 4a |
| I19 | B/H/X/Y-Folge mit Tab | B → H (Feld 14508 → 14509) | B → H | ✅ | MSG, FP I02 |
| I20 | Proportionen-Schloss | koppelt B/H | ❔ | ❔ | – |

## H. Fenster, Dokumente, Arbeitsbereich (W)

| Nr | Ausgangszustand → Aktion | Original | Firepixel | St. | Beleg |
|---|---|---|---|---|---|
| W01 | 2 Dokumente → Strg+Tab | nächstes Dokument (MDI) | **nichts** | ❌ | WIN (MDI, SPEZ §4.4), FP W01 |
| W02 | 2 Dokumente → Strg+Umschalt+Tab / Strg+F6 | vorheriges/nächstes | nichts | ❌ | WIN |
| W03 | Fenstertitel | „*uxb_doc1.fw.png“ (nur Dokumentname, Stern vorn bei Änderung, **kein Zoom**) | „Firepixel - [*Name @ 100%]“ | ⚠️ | TITEL, FP W02 |
| W04 | Reiter → Mittelklick | ❔ schließt | schließt | ❔ | CODE canvas/mod.rs:107 |
| W05 | Reiter → Rechtsklick | Kontextmenü: Alles hier zusammenführen (grau) · Schließen · Alle schließen · In neues Fenster verschieben │ Neues Dokument · Dokument öffnen | **keins** | ❌ | MSG `o_reiter_msg.txt`, CODE |
| W06 | Reiter ziehen | ❔ umordnen bzw. lösen | nicht möglich | ❔ | CODE |
| W07 | Reiter-Beschriftung geändert | „*uxb_doc1.fw.png ×“ (Stern vorn, voller Dateiname, ×) | „*Name ×“ | ✅ | GUI `re\_testdateien\ux\o2\menu_datei.png`, CODE |
| W08 | viele Dokumente | Überlauf „Weitere Dokumente“ (»-Menü) | Reiter laufen aus dem Bild | ❌ | SPEZ §4.4, CODE (keine Kürzung) |
| W09 | Reiter-× | schließt mit Rückfrage | schließt mit Rückfrage | ✅ | CODE |
| W10 | Fenster ▸ Fenster duplizieren | zweite Ansicht | ❔ | ❔ | SPEZ §4.4 |
| W11 | Bildschirmmodi (F) | Standard / Vollbild mit Menüs / Vollbild | 3 Modi (LW behoben) | ✅ | BEFUNDE LW |
| W12 | Tab (ohne Textfeld) | blendet Bedienfelder aus/ein | ❔ | ❔ | SPEZ §4.1 (35298) |
| W13 | Strg+Umschalt+H | Bedienfelder ausblenden | ❔ | ❔ | SPEZ §4.1 |
| W14 | Bedienfeldgruppe → Doppelklick auf Reiter | minimiert/maximiert | ❔ | ❔ | SPEZ §4.1 |
| W15 | Bedienfeld-Reiter ziehen | lösen/andocken | lösen/andocken | ✅ | CODE docking.rs |
| W16 | Gruppen-Kontextmenü | minimieren/maximieren/lösen/umbenennen/schließen … | ❔ | ❔ | SPEZ §4.1 |
| W17 | Arbeitsbereich-Umschalter | „Erweiterter Modus“ usw. | „Erweiterter Modus“ | ✅ | GUI uxb_s1_b.png, FP Bild |
| W18 | Layout nach Neustart | gemerkt | gemerkt (Workspace in Voreinstellungen) | ✅ | CODE workspace.rs |
| W19 | Fenstergröße/Lage nach Neustart | gemerkt | ❔ | ❔ | – |
| W20 | Hauptfenster schließen mit Änderungen | Rückfrage je Dokument | Rückfrage | ✅ | CODE |
| W21 | Strg+Mausrad | Nutzerbefund (an LW) | – | ➖ | – |
| W22 | Datei ▸ Neu → Name des neuen Dokuments | „Unbenannt-45.fw.png“ (Endung .fw.png, laufende Nummer über die Sitzung) | „Unbenannt-1.png“ | ⚠️ | GUI `o2\menu_datei.png` (Reiter), FP M02 |

## I. Statuszeile und Fußleiste (S)

| Nr | Ausgangszustand → Aktion | Original | Firepixel | St. | Beleg |
|---|---|---|---|---|---|
| S01 | Fußleiste: Reihenfolge | Auswahlinfo links („GIF (Dokument)“), Status-Knöpfe Erster/Wiedergabe/Letzter, Nummer, Vorheriger/Nächster, Bitmapmodus beenden, „800 x 600“, „ 100%“ | Auswahlinfo, Status-Knöpfe, Nummer, Größe, Zoom | ✅ | SPEZ 4a T3, FP Bild |
| S02 | Fußleiste: Auswahlinfo bei 3 Objekten | „3 Objekte“ | „3 Objekte“ | ✅ | FP Bild |
| S03 | Fußleiste ohne Auswahl | Exportformat „GIF (Dokument)“ | „PNG (Dokument)“ bei Vorgabe GIF Adaptiv 256 | ⚠️ | SPEZ 4a T3, FP Bild (Optimieren zeigt GIF, Fußleiste PNG) |
| S04 | Status-Knöpfe bei 1 Status | alle gesperrt | gesperrt (grau) | ✅ | SPEZ §4.8, FP Bild |
| S05 | Zoom-Anzeige in der Fußleiste → Klick | Liste der Stufen | Liste | ✅ | FP Bild |
| S06 | Zoom-Feld in der Anwendungsleiste → „150“ tippen | ❔ (Kombifeld editierbar?) | nur Liste, kein Tippen | ❔ | CODE appbar.rs:97 |
| S07 | Zoom-Format | „%4.f%%“ (mit führenden Leerzeichen: „ 100%“) | „100%“ | ⚠️ | SPEZ §4.6, 4a T3 |
| S08 | Stufen | 6, 12, 25, 50, 66, 100, 150, 200, 300, 400, 800, 1600, 3200, 6400 | gleich | ✅ | SPEZ §4.6, CODE |
| S09 | Bitmapmodus beenden-Knopf | nur im Bitmapmodus aktiv | ❔ | ❔ | SPEZ §4.8 |
| S10 | Info-Knopf (43404) | ❔ (nicht sichtbar in T3) | ❔ | ❔ | SPEZ 4a T3 |

---

## Offen (Original noch nicht belegt)

Diese Punkte brauchen echte Maus- und Tastatureingaben im Original. Die GUI-Fernsteuerung fiel nach 19:58 aus („Das Handle ist ungültig“, „Zugriff verweigert“).
Die Skripte dafür liegen bereit:
- `re\_testdateien\ux\skripte\o_menues.ps1` (Menüs, Tastaturnavigation)
- `o_dialoge1.ps1` (Dialoge über Kürzel)

Für die Bedienfeld-, Reiter- und Inspektor-Szenarien kommt eine Sitzung mit Koordinaten aus `o2\voll.png` dazu.

## Befunde (Übersicht)

Die Befunde stehen in `docs\BEFUNDE.md`; diese Liste nennt nur die Nummern.

- **IBD**
  - D02/D03/D12–D18: Erstfokus und Markierung in Dialogen
  - D05: gemerkte Werte bei Neues Dokument
  - D08: Vorlagen-Knopf
  - D23: graue gesperrte Knöpfe
  - D26: Enter auf fokussiertem Knopf
  - M02/M06: Wortlaut und Zugriffstasten der Ungespeichert-Abfrage
  - M10: Enter in Rückfragen
  - E01/E04: Umbenennen per Doppelklick
  - F19: Als Befehl speichern
  - D07: Format der Leinwandgröße
- **HG**
  - ME01–ME08: Tastaturbedienung der Menüs
  - W01/W02: Strg+Tab
  - W05/W08: Reiter-Kontextmenü und Überlauf
  - W03: Fenstertitel
  - F02/F06/F07: Datei-Filter und Optionen
  - P28: Standardreiter der Gruppe
  - S03/S07: Fußleiste

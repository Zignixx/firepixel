# Paritätsabgleich B: Menüs, Dialoge, Bedienfelder, Fenster (Original ↔ Firepixel)

## Übergabe (Stand 12.10.2026 ≈ 04:30, Agent UX-B, auf Nutzerwunsch angehalten)

### Stand und Zahlen

| Bereich | geprüft | ✅ | ⚠️ | ❌ | ℹ️/❔ | Abschnitt |
|---|---:|---:|---:|---:|---:|---|
| Hauptmenüs (jeder Eintrag in 5 Lagen: Kürzel, aktiv/grau, Häkchen) | 350 Zeilen | 311 | 5 | 5 | 29 | §1 |
| Dialoge Feld für Feld | 50 | 3 | 16 | 30 | 1 | §2 |
| Bedienfeldmenüs (≡) | 9 | 1 | 3 | 4 | 1 | §3 |

**Wichtigste Ergebnisse:**
- Die **Menüfreigabe** ist nach meinem Befund von EB behoben (Commit c67a402, Test `paritaet_menue.rs`, 264/264). Meine Nachmessung bestätigt das: Von zuvor 96 ❌ bleiben 5.
- Offen im Menü:
  - Aufbau des Schrift-Menüs (zuletzt benutzte Schriften oben, Familien flach statt A ▸ B ▸ …)
  - „Bridge durchsuchen“ aktiv
  - „Mit darunter liegender reduzieren“ bei Text
  - Befehle-Menü im frischen Profil, noch nicht nachgeprüft
- Bei den Dialogen sind die meisten noch offen (Befunde an I18N-DIALOGE). Die Schwerpunkte:
  - Schieber sind im Original **stückweise nicht linear**; die Messtabelle steht in BEFUNDE.
  - Vorgaben und gemerkte Werte aus den Voreinstellungen fehlen (Original 10, Firepixel 1).
  - Unvollständige Dialoge: Exportieren, Bildvorschau, Exportassistent, HTML-Einrichtung, Kurven/Tonwertkorrektur (Pipetten, Histogramm), Auswahl speichern u. a.
  - Hinweis-Meldungen fehlen.
- Das Original zeigt bei immer aktiven Befehlen ohne Ziel **Warnungen mit „Nicht mehr anzeigen“**. Der genaue Wortlaut steht in BEFUNDE unter EB.

**Befunde dieses Abgleichs** (alle mit „(von UX-B, Paritätsabgleich …)“ in `docs\BEFUNDE.md`; Stand bei Abbruch):

| Zuständig | Befund | Stand |
|---|---|---|
| I18N-ENGINE / EB | Freigabe von ~70 Menübefehlen in 5 Lagen | **behoben** (EB c67a402), nachgeprüft |
| I18N-ENGINE / EB | „Wiederholen“ (Strg+Y) im Original aktiv mit „Wiederholen: <letzter Schritt>“ | offen, im Original noch zu klären |
| EB | Warnungen bei immer aktiven Befehlen ohne Ziel (Wortlaut Z01–Z18) | offen |
| I18N-MENUE | Rückgängig/Wiederholen mit Schrittnamen; Häkchen im Fenster-Menü nur beim vordersten Reiter; Schrift-Menü; Kürzel-Anzeige | teilweise (Schnitte als Untermenü umgesetzt); Schrittnamen in der echten App nicht nachgeprüft |
| I18N-SHELL / HG / AU | Befehle-Menü im frischen Profil leer (`install_builtin_commands`) | laut Koordinator eingebaut, **nicht nachgeprüft** |
| I18N-SHELL / HG | native Datei-Dialoge bei (0,0) ohne Besitzer, Startordner, Batch-Dialog | offen |
| I18N-DIALOGE / IBD | D01–D25, D26–D50, Schieberabbildung, Vorgaben/gemerkte Werte aus den Voreinstellungen | Schieberabbildung, senkrechte Popup-Schieber und Vorgaben/gemerkte Werte 🔧 behoben (FV-4), Nachmessung im echten Fenster offen; übrige D-Zeilen offen |
| I18N-BEDIENFELDER / IBD | Zustände in den Bedienfeldmenüs (Verlauf, Stile, Status, Seiten, Ebenen, Optimieren) | offen |
| (älter) IBD/HG | Katalog `docs\UX_ABGLEICH_B.md` vom 11.10. (Fokus, Ungespeichert-Wortlaut, Ebenen-Klick, Reiter-Kontextmenü …) | teilweise behoben, siehe dort |

### Noch nicht geprüft (Reihenfolge für die nächste Runde)

1. **Nachprüfen in der echten App** (Firepixel neu aus dem aktuellen Stand bauen; der Release-Build in `target-uxb` brach zuletzt beim Linken mit `STATUS_STACK_BUFFER_OVERRUN` ab, einfach erneut bauen):
   - Befehle-Menü im frischen Profil
   - „Rückgängig: <Schritt>“ im Bearbeiten-Menü
   - Häkchen im Fenster-Menü
   - D01 Fokus/Markierung (ist schon gut)
2. **Bedienfelder Knopf für Knopf:**
   - Ebenen: Fußleiste, Doppelklick/Umbenennen, Ziehen über Augen, Rechtsklick → Menü 6923 (im Original belegt, siehe §3 und `UX_ABGLEICH_B.md` E-Zeilen)
   - Seiten, Status (Verzögerung, Löschmethode), Verlauf (Schieber, Wiederholen), Optimieren (Farbtabelle), Farbfelder (Strg/Umschalt-Klick), Stile, Bibliotheken
   - Ausrichten/Pfad/Bildbearbeitung (Flash-Bedienfelder, kein ≡)
3. **Reiter und Fenster:** Reiter-Kontextmenü (Original belegt: Alles hier zusammenführen / Schließen / Alle schließen / In neues Fenster verschieben │ Neues Dokument / Dokument öffnen), Strg+Tab, Mittelklick, Überlauf, Fenster duplizieren/Überlappend/Anordnen.
4. **Docking und Arbeitsbereiche:** Gruppen-Kontextmenü, lösen/andocken, einklappen, Arbeitsbereich-Umschalter, Layout nach Neustart.
5. **Statuszeile und Fußleiste:** Status-Knöpfe, Größe, Zoom-Feld/-Liste, Auswahlinfo; Startbildschirm.
6. **Datei-Abläufe:** nur mit dem Testmodus von AU für nicht blockierende Datei-Dialoge. Neu/Öffnen/Speichern/Exportieren/Bildvorschau/Importieren (Platzierungs-Cursor).
7. **Dialog-Details:**
   - Tab-Folge und Enter/Esc je Dialog (Original-Tabfolge steht im Dialogbericht)
   - Fehlermeldungen bei ungültigen Werten: im Original per WM_SETTEXT + IDOK prüfbar, Meldung über den Dialog-Runner lesen
   - Live-Vorschau beim Ziehen (Original während des Ziehens?)
   - D10 Datei-Info

### Werkzeuge (alle unter `re\_testdateien\ux\skripte\`, Messdaten unter `re\_testdateien\ux\paritaet\`)

- **Sperre und Vordergrund:** Jede Original-Sitzung läuft mit `re\scripts\fwsteuerung\gui-sitzung.ps1 -Skript <x.ps1> -WarteMin 30` (höchstens 5 min).
  - `uxb_hilfe.ps1` ersetzt `Tasten`/`Klick`/`Ziehen`/`Rad`/`Doppel`/`RechtsKlick`/`ModKlick`/`Vorne` durch Fassungen **mit Vordergrundprüfung** (PID des Originals + Klasse Hauptfenster/`#32770`/`#32768`, sonst Abbruch; `Vorne` ohne Alt-Taste über AttachThreadInput).
  - Nie Strg+Alt+Leertaste senden.
- **Original ohne Bildschirm/Maus** (geht auch bei minimiertem RDP):
  - **Menüs lesen:** `PostMessage(WM_LBUTTONDOWN/UP)` an das Fenster `OWL.MenuBar` des Hauptfensters, x je Titel (Datei 22, Bearbeiten 81, Ansicht 148, Auswählen 214, Modifizieren 293, Text 356, Befehle 404, Filter 454, Fenster 503, Hilfe 553; y 13). Das geöffnete `#32768`-Fenster per `MN_GETHMENU` (0x1E1) lesen, Einträge mit `GetMenuString`/`GetMenuState` (grau = Bit 0/1, ✓ = Bit 3). Bei Untermenüs `GetSubMenu` ≠ 0 nur das untere Byte werten.
  - **Untermenüs frisch:** `MN_SELECTITEM` (0x1E5) + `MN_OPENHIERARCHY` (0x1E3) an das Menüfenster, das neue `#32768` lesen.
  - **Schließen:** Esc an das Menüfenster + `WM_CANCELMODE` (0x1F) ans Hauptfenster. `EndMenu` wirkt nur im eigenen Thread.
  - Skripte: `o_menue_lagen.ps1` (alle Menüs einer Lage, `$env:UXB_LAGE`), `o_menue_bilder.ps1`.
- **Befehle auslösen:** `MenuPfad 'Filter>Weichzeichnen>Gaußscher Weichz. ...'` in `o_dialog_lauf_kopf.ps1` (öffnen, `MN_SELECTITEM`, Enter) oder `WM_COMMAND <ID>` ans Hauptfenster (IDs in `paritaet\o_menues.tsv`).
- **Dialog-Runner** `o_dialog_lauf.ps1`:
  - Liste `Nr|Lage|Menüpfad` in `$env:UXB_DLG_LISTE`.
  - Je Dialog `DialogBericht`: Titel, Lage, Steuerelemente in Tab-Folge, Standardknopf, Fokus, Markierung, Werte, Kombi-Einträge.
  - Bild `paritaet\dialoge\o_<Nr>.png`, dann Abbrechen (auch eigener Abbrechen-Knopf, z. B. Bildvorschau-ID 4681).
  - Mit `$env:UXB_UNDO=1` wird zusätzlich „Rückgängig: …“ vorher/nachher gelesen und ein entstandener Schritt zurückgenommen.
- **Lagen im Original:** `o_lagen_aufbau.ps1` (`$env:UXB_SCHRITT` = `neu`/`aufbau2`/`S1`…`S6`).
  - Aktiviert das eigene Testdokument über das Fenster-Menü (`MeinDok`), Standardname `Unbenannt-77` bzw. `$env:UXB_DOK`; das Dokument wurde am Ende ohne Speichern geschlossen.
  - Für eine neue Runde: `UXB_SCHRITT=neu`, danach `aufbau2`.
  - Schieber messen: `o_schieber.ps1`.
- **Firepixel:**
  - Eigene Instanz mit eigenem Profil: `fp_start.ps1` (APPDATA umgelenkt auf `re\_testdateien\ux\fp_profil`, Port 47833, Fenster rechts; Sprache einmal über Ansicht ▸ Sprache auf Deutsch stellen).
  - MCP-Client: `fp_mcp.ps1` (`FpVerbinden`, `FpTool 'ui_input'|'ui_find'|'screenshot'|'select'|'ui_action'`). Achtung: Esc hebt in Firepixel die Auswahl auf, deshalb vor jedem Schritt neu auswählen.
  - Dialoge: `fp_dialog_lauf.ps1 -Liste <Nr|Lage|Befehl|name>` erzeugt die Nebeneinander-Bilder `docs\screenshots\paritaet\b_<Nr>_<name>.png`.
  - Menübilder: `fp_menue_bilder.ps1`.
  - **Vor und nach jeder Sitzung:** `fp_dialoge_zu.ps1` (schließt native `#32770` der eigenen Instanz per WM_CLOSE). Keine Befehle mit nativem Datei-Dialog auslösen.
- **Sonden ohne Fenster:** `crates\fp-ui\tests\ux_b_paritaet.rs`, Ausgabe nach `target-uxb\paritaet\` (Stand dieser Messung kopiert nach `re\_testdateien\ux\paritaet\firepixel\`): alle Menüs in 7 Lagen, Auslösen jedes Menübefehls mit Dialog/Verlauf/Formularfeldern, Bedienfeldmenüs.
  - Vergleich: `vergleich_menues.py` → `md_menues.py` (erzeugt §1).
  - Ältere Sonden: `crates\fp-ui\tests\ux_b_abgleich.rs`.
- **Ablauf einer Messung:**
  1. `fp_dialoge_zu.ps1`
  2. Original-Sitzung: Lage herstellen, Element öffnen/auslesen/fotografieren, schließen
  3. Firepixel über die Brücke: dieselbe Lage, gleiches Element, Bild
  4. Bilder zusammensetzen
  5. Zeile hier eintragen, Abweichung als Befund beim Zuständigen

---

Agent **UX-B**, Stand 12.10.2026 nachts. Der Abgleich wurde von Grund auf neu gemacht, mit echten Eingaben und mit beiden Programmen nebeneinander.

**Aufbau**
- **Original:** links.
- **Firepixel:** eigene Testinstanz rechts.
  - Release-Build aus `target-uxb`.
  - Eigenes Profil unter `re\_testdateien\ux\fp_profil`, damit die Voreinstellungen des Nutzers unberührt bleiben.
  - Sprache Deutsch, MCP-Brücke auf Port 47833.
- **Testdokumente:** gleich aufgebaut, 500×400 mit zwei Rechtecken, einem Text „Hallo“ und einer Bitmap.
  - Original: „Unbenannt-77“, nicht gespeichert.
  - Firepixel: „Unbenannt-1“.
- **Bilder:** `docs\screenshots\paritaet\b_<nr>_<name>.png`, links Original, rechts Firepixel.

**Hinweis zum Original:** JSF ist zur Zeit gestört (Fehler -1073741819). Ein Neustart des Programms wartet auf den Nutzer. Lagen, Menüs und Dialoge werden deshalb über echte Eingaben und Fensternachrichten hergestellt und gelesen. Was nur mit JSF prüfbar wäre, ist als „wartet auf Neustart des Originals“ markiert.

**Status**
- ✅ gleich
- ⚠️ kleine Abweichung (meist Beschriftung)
- ❌ Funktion anders oder fehlt
- ℹ️ nur in einem Programm, bewusst
## 1. Hauptmenüs (jeder Eintrag in 5 Lagen)

Lagen: S1 Dokument ohne Auswahl · S2 ein Rechteck · S3 zwei Rechtecke · S4 Text · S5 Bitmap. „aktiv“ nennt den Zustand je Lage in dieser Reihenfolge.
Original: Menüs per Nachricht an die Menüleiste geöffnet, Untermenüs einzeln geöffnet (frischer Zustand), Einträge per `MN_GETHMENU`/`GetMenuState` gelesen (`re\_testdateien\ux\paritaet\o_menue_S1…S5.txt`).
Firepixel: Sonde `crates\fp-ui\tests\ux_b_paritaet.rs` (`target-uxb\paritaet\fp_menues.tsv`), Bilder über die MCP-Brücke der eigenen Testinstanz.
Nicht verglichen werden die Laufzeitlisten (zuletzt geöffnete Dateien, Dokumentliste) sowie der Zustand von Rückgängig/Wiederholen und der Einfüge-Befehle, weil Verlauf und Zwischenablage verschieden waren.

Nachprüfung 12.10. nach EB-Commit c67a402: Firepixel neu gemessen (Sonde), Original unverändert. Häkchen im Fenster-Menü/QuickInfos/Anti-Alias-Füllung sowie Einfüge-Befehle hängen von Reiterlage, Voreinstellungen bzw. Zwischenablage ab und werden nicht gewertet.

Zählung: ℹ️ 29, ⚠️ 5, ✅ 311, ❌ 5

| Element | Original | Firepixel | | Bild |
|---|---|---|---|---|
| Befehle > (mitgelieferte Befehlsskripte) | 51 Einträge: Auswahl, Batch-Befehle, Bildschirmfoto, Demo - aktuelles Dokument, Dokument, Größe ausgewählter Objekte ändern, Kreativ, Symbolskript erstellen, Text, Web, jQuery Mobile-Thema | fehlt: im frischen Profil legt Firepixel keine eigenen Befehle an (nur „Gespeicherte Befehle verwalten…“, „Erweiterungen verwalten…“, „Skript ausführen…“, „Warnmeldungen zurücksetzen“) | ❌ | [Bild](screenshots/paritaet/b_M07_menue_befehle.png) |
| Text > Schrift > (Liste) | Kürzel –; aktiv 439 Einträge: zuerst zuletzt benutzte Schriften, dann alle Familien flach, Schnitte als Untermenü | Kürzel –; aktiv 377 Einträge: Schnitte jetzt als Untermenü, aber weiter nach Anfangsbuchstaben gruppiert (A ▸, B ▸ …) und ohne zuletzt benutzte Schriften – Aufbau des Schriftmenüs anders | ❌ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Datei > Neu | Kürzel Strg+N; aktiv an an an an an | Kürzel Strg+N; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Neu von Vorlage | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Öffnen | Kürzel Strg+O; aktiv an an an an an | Kürzel Strg+O; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Bridge durchsuchen | Kürzel Strg+Alt+O; aktiv aus aus aus aus aus | Kürzel Strg+Alt+O; aktiv an an an an an – aktiv weicht ab in S1,S2,S3,S4,S5 | ❌ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Zuletzt geöffnete Dateien | Kürzel –; aktiv an an an an an | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Scannen | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Scannen > Twain importieren | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Scannen > Twain auswählen | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Schließen | Kürzel Strg+W; aktiv an an an an an | Kürzel Strg+W; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Alle schließen | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Speichern | Kürzel Strg+S; aktiv an an an an an | Kürzel Strg+S; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Speichern unter | Kürzel Strg+Umschalt+S; aktiv an an an an an | Kürzel Strg+Umschalt+S; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Alles speichern | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Als Vorlage speichern | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Wiederherstellen | Kürzel –; aktiv aus aus aus aus aus | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Importieren | Kürzel Strg+R; aktiv an an an an an | Kürzel Strg+R; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Exportieren | Kürzel Strg+Umschalt+R; aktiv an an an an an | Kürzel Strg+Umschalt+R; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Bildvorschau | Kürzel Strg+Umschalt+X; aktiv an an an an an | Kürzel Strg+Umschalt+X; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Als E-Mail senden | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Als E-Mail senden > PNG-Datei für Firepixel | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Als E-Mail senden > Komprimierte JPG-Datei | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Als E-Mail senden > Exporteinstellungen verwenden | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > HTML aktualisieren | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Datei-Info | Kürzel Strg+Alt+Umschalt+F; aktiv an an an an an | Kürzel Strg+Alt+Umschalt+F; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Exportassistent | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Batch-Verarbeitung | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Vorschau im Browser | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Vorschau im Browser > Vorschau in Standardbrowser | Kürzel F12; aktiv an an an an an | Kürzel F12; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Vorschau im Browser > Vorschau in | Kürzel Umschalt+F12; aktiv aus aus aus aus aus | Kürzel Umschalt+F12; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Vorschau im Browser > Vorschau aller Seiten in Standardbrowser anzeigen | Kürzel Strg+F12; aktiv an an an an an | Kürzel Strg+F12; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Vorschau im Browser > Primärbrowser festlegen | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Vorschau im Browser > Sekundärbrowser festlegen | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Seite einrichten | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Drucken | Kürzel Strg+P; aktiv an an an an an | Kürzel Strg+P; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > HTML-Einrichtung | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Beenden | Kürzel Strg+Q; aktiv an an an an an | Kürzel Strg+Q; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Bearbeiten > Rückgängig | Kürzel Strg+Z; aktiv an an an an an | Kürzel Strg+Z; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Wiederholen | Kürzel Strg+Y; aktiv an an an an an | Kürzel Strg+Y; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Objekt einfügen | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Objekt einfügen > Neue Schaltfläche | Kürzel Strg+Umschalt+F8; aktiv an an an an an | Kürzel Strg+Umschalt+F8; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Objekt einfügen > Neues Symbol | Kürzel Strg+F8; aktiv an an an an an | Kürzel Strg+F8; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Objekt einfügen > Hotspot | Kürzel Strg+Umschalt+U; aktiv an an an an an | Kürzel Strg+Umschalt+U; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Objekt einfügen > Rechteckiges Segment | Kürzel Alt+Umschalt+U; aktiv an an an an an | Kürzel Alt+Umschalt+U; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Objekt einfügen > Polygonsegment | Kürzel Alt+Umschalt+P; aktiv an an an an an | Kürzel Alt+Umschalt+P; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Objekt einfügen > Leere Bitmap | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Objekt einfügen > Bitmap durch Kopieren | Kürzel –; aktiv aus aus aus aus aus | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Objekt einfügen > Bitmap durch Ausschneiden | Kürzel –; aktiv aus aus aus aus aus | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Objekt einfügen > Ebene | Kürzel Umschalt+L; aktiv an an an an an | Kürzel Umschalt+L; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Objekt einfügen > Status | Kürzel Umschalt+F; aktiv an an an an an | Kürzel Umschalt+F; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Objekt einfügen > Seite | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Suchen und ersetzen | Kürzel Strg+F; aktiv an an an an an | Kürzel Strg+F; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Ausschneiden | Kürzel Strg+X; aktiv aus an an an an | Kürzel Strg+X; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Kopieren | Kürzel Strg+C; aktiv aus an an an an | Kürzel Strg+C; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Als Vektoren kopieren | Kürzel –; aktiv aus an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > HTML-Code kopieren | Kürzel Strg+Alt+C; aktiv an an an an an | Kürzel Strg+Alt+C; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Einfügen | Kürzel Strg+V; aktiv an an an an an | Kürzel Strg+V; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Löschen | Kürzel Löschen-Taste; aktiv aus an an an an | Kürzel Entf; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Als Maske einfügen | Kürzel Strg+Alt+V; aktiv aus an aus an an | Kürzel Strg+Alt+V; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Innen einfügen | Kürzel Strg+Umschalt+V; aktiv aus an aus an an | Kürzel Strg+Umschalt+V; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Attribute einfügen | Kürzel Strg+Alt+Umschalt+V; aktiv an an an an an | Kürzel Strg+Alt+Umschalt+V; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Duplizieren | Kürzel Strg+Alt+D; aktiv aus an an an an | Kürzel Strg+Alt+D; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Klonen | Kürzel Strg+Umschalt+D; aktiv aus an an an an | Kürzel Strg+Umschalt+D; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Ausgewählte Bitmap zuschneiden | Kürzel –; aktiv aus aus aus aus an | Kürzel –; aktiv aus aus aus aus an | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Dokument zuschneiden | Kürzel –; aktiv aus an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Voreinstellungen | Kürzel Strg+U; aktiv an an an an an | Kürzel Strg+U; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Tastaturkurzbefehle | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Ansicht > Größer zoomen | Kürzel Strg++; aktiv an an an an an | Kürzel Strg++; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Kleiner zoomen | Kürzel Strg+-; aktiv an an an an an | Kürzel Strg+-; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Vergrößerung | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Vergrößerung > 6% | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Vergrößerung > 12% | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Vergrößerung > 25% | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Vergrößerung > 50% | Kürzel Strg+5; aktiv an an an an an | Kürzel Strg+5; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Vergrößerung > 66% | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Vergrößerung > 100% | Kürzel Strg+1; aktiv an an an an an; ✓ | Kürzel Strg+1; aktiv an an an an an; ✓ | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Vergrößerung > 150% | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Vergrößerung > 200% | Kürzel Strg+2; aktiv an an an an an | Kürzel Strg+2; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Vergrößerung > 300% | Kürzel Strg+3; aktiv an an an an an | Kürzel Strg+3; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Vergrößerung > 400% | Kürzel Strg+4; aktiv an an an an an | Kürzel Strg+4; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Vergrößerung > 800% | Kürzel Strg+8; aktiv an an an an an | Kürzel Strg+8; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Vergrößerung > 1600% | Kürzel Strg+6; aktiv an an an an an | Kürzel Strg+6; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Vergrößerung > 3200% | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Vergrößerung > 6400% | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Auswahl anpassen | Kürzel Strg+Alt+0; aktiv aus an an an an | Kürzel Strg+Alt+0; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Alles anpassen | Kürzel Strg+0; aktiv an an an an an | Kürzel Strg+0; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Gamma 1.8 | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Auswahl ausblenden | Kürzel Strg+L; aktiv aus an an an an | Kürzel Strg+L; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > QuickInfos | Kürzel Strg+AKUT; aktiv an an an an an | Kürzel Strg+´; aktiv an an an an an; ✓ | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Alles einblenden | Kürzel Strg+Umschalt+L; aktiv an an an an an | Kürzel Strg+Umschalt+L; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Lineale | Kürzel Strg+Alt+R; aktiv an an an an an | Kürzel Strg+Alt+R; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Raster | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Raster > Raster anzeigen | Kürzel Strg+Alt+G; aktiv an an an an an | Kürzel Strg+Alt+G; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Raster > Am Raster ausrichten | Kürzel Strg+Alt+Umschalt+G; aktiv an an an an an | Kürzel Strg+Alt+Umschalt+G; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Raster > Raster bearbeiten | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Hilfslinien | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Hilfslinien > Hilfslinien anzeigen | Kürzel Strg+ü; aktiv an an an an an; ✓ | Kürzel Strg+ü; aktiv an an an an an; ✓ | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Hilfslinien > Hilfslinien sperren | Kürzel Strg+Alt+ü; aktiv an an an an an | Kürzel Strg+Alt+ü; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Hilfslinien > An Hilfslinien ausrichten | Kürzel Strg+Umschalt+ü; aktiv an an an an an; ✓ | Kürzel Strg+Umschalt+ü; aktiv an an an an an; ✓ | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Hilfslinien > Hilfslinien entfernen | Kürzel –; aktiv aus aus aus aus aus | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Intelligente Hilfslinien | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Intelligente Hilfslinien > Intelligente Hilfslinien anzeigen | Kürzel Strg+ZIRKUMFLEX; aktiv an an an an an; ✓ | Kürzel Strg+^; aktiv an an an an an; ✓ | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Intelligente Hilfslinien > An intelligenten Hilfslinien ausrichten | Kürzel Strg+Umschalt+ZIRKUMFLEX; aktiv an an an an an; ✓ | Kürzel Strg+Umschalt+^; aktiv an an an an an; ✓ | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Hilfslinien für 9-Slice-Skalierung | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Hilfslinien für 9-Slice-Skalierung > Hilfslinien sperren | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Segment-Hilfslinien | Kürzel Strg+Alt+Umschalt+ü; aktiv an an an an an; ✓ | Kürzel Strg+Alt+Umschalt+ü; aktiv an an an an an; ✓ | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Segmentüberlagerung | Kürzel –; aktiv an an an an an; ✓ | Kürzel –; aktiv an an an an an; ✓ | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Ränder | Kürzel Strg+H; aktiv an an an an an; ✓ | Kürzel Strg+H; aktiv an an an an an; ✓ | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Auswählen > Alles auswählen | Kürzel Strg+A; aktiv an an an an an | Kürzel Strg+A; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Auswählen > Auswahl aufheben | Kürzel Strg+D; aktiv aus an an an an | Kürzel Strg+D; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Auswählen > Gesamtauswahl treffen | Kürzel Strg+Nach-rechts-Taste; aktiv aus an an an an | Kürzel Strg+Rechts; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Auswählen > Teilauswahl treffen | Kürzel Strg+Nach-links-Taste; aktiv aus aus aus aus aus | Kürzel Strg+Links; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Auswählen > Ähnliches auswählen | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Auswählen > Auswahl umkehren | Kürzel Strg+Umschalt+I; aktiv an an an an an | Kürzel Strg+Umschalt+I; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Auswählen > Weiche Kante | Kürzel Strg+Alt+Umschalt+D; aktiv an an an an an | Kürzel Strg+Alt+Umschalt+D; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Auswählen > Auswahlrahmen erweitern | Kürzel –; aktiv aus aus aus aus aus | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Auswählen > Auswahlrahmen verkleinern | Kürzel –; aktiv aus aus aus aus aus | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Auswählen > Auswahlrahmen begrenzen | Kürzel –; aktiv aus aus aus aus aus | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Auswählen > Auswahlrahmen glätten | Kürzel –; aktiv aus aus aus aus aus | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Auswählen > Auswahlrahmen in Pfad konvertieren | Kürzel –; aktiv aus aus aus aus aus | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Auswählen > Bitmap-Auswahl speichern | Kürzel –; aktiv aus aus aus aus aus | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Auswählen > Bitmap-Auswahl wiederherstellen | Kürzel –; aktiv aus aus aus aus aus | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Auswählen > Bitmap-Auswahl löschen | Kürzel –; aktiv aus aus aus aus aus | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Modifizieren > Leinwand | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Leinwand > Bildgröße | Kürzel Strg+J; aktiv an an an an an | Kürzel Strg+J; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Leinwand > Leinwandgröße | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Leinwand > Leinwandfarbe | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Leinwand > Leinwand zuschneiden | Kürzel Strg+Alt+T; aktiv an an an an an | Kürzel Strg+Alt+T; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Leinwand > Leinwand anpassen | Kürzel Strg+Alt+F; aktiv an an an an an | Kürzel Strg+Alt+F; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Leinwand > 180° drehen | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Leinwand > 90° im UZS drehen | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Leinwand > 90° gegen UZS drehen | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Animation | Kürzel –; aktiv an an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Animation > Auswahl animieren | Kürzel Alt+Umschalt+F8; aktiv aus an an an an | Kürzel Alt+Umschalt+F8; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Animation > Einstellungen | Kürzel –; aktiv aus aus aus aus aus | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Animation > Animation entfernen | Kürzel –; aktiv aus aus aus aus aus | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Symbol | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Symbol > In Symbol konvertieren | Kürzel F8; aktiv aus an an an an | Kürzel F8; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Symbol > Symbol bearbeiten | Kürzel –; aktiv aus aus aus aus aus | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Symbol > An Position bearbeiten | Kürzel –; aktiv aus aus aus aus aus | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Symbol > Instanzen tweenen | Kürzel Strg+Alt+Umschalt+T; aktiv an an an an an | Kürzel Strg+Alt+Umschalt+T; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Symbol > Teilen | Kürzel Strg+Umschalt+B; aktiv an an an an an | Kürzel Strg+Umschalt+B; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Symbol > Symbol austauschen | Kürzel –; aktiv aus aus aus aus aus | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Popupmenü | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Popupmenü > Popupmenü hinzufügen | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Popupmenü > Popupmenü bearbeiten | Kürzel –; aktiv aus aus aus aus aus | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Popupmenü > Popupmenü löschen | Kürzel –; aktiv aus aus aus aus aus | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Maske | Kürzel –; aktiv an an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Maske > Alles einblenden | Kürzel –; aktiv aus an aus an an | Kürzel –; aktiv aus an aus an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Maske > Alles ausblenden | Kürzel –; aktiv aus an aus an an | Kürzel –; aktiv aus an aus an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Maske > Als Maske einfügen | Kürzel Strg+Alt+V; aktiv aus an aus an an | Kürzel Strg+Alt+V; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Maske > Als Maske gruppieren | Kürzel –; aktiv aus aus an aus aus | Kürzel –; aktiv aus aus an aus aus | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Maske > Auswahl einblenden | Kürzel –; aktiv aus aus aus aus aus | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Maske > Auswahl ausblenden | Kürzel –; aktiv aus aus aus aus aus | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Maske > Maske deaktivieren | Kürzel –; aktiv aus aus aus aus aus | Kürzel Strg+Alt+H; aktiv aus aus aus aus aus – Kürzel – ≠ Strg+Alt+H | ⚠️ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Maske > Maske löschen | Kürzel –; aktiv aus aus aus aus aus | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Selektives JPEG | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Selektives JPEG > Auswahl als JPEG-Maske speichern | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Selektives JPEG > JPEG-Maske als Auswahl wiederherstellen | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Selektives JPEG > Einstellungen | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Selektives JPEG > JPEG-Maske entfernen | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Auswahl sperren | Kürzel Strg+Alt+L; aktiv aus an an an an | Kürzel Strg+Alt+L; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Auswahl auf Hintergrund reduzieren | Kürzel Strg+Alt+Umschalt+Z; aktiv aus an an an an | Kürzel Strg+Alt+Umschalt+Z; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Mit darunter liegender auf eine Ebene reduzieren | Kürzel Strg+E; aktiv aus aus aus an aus | Kürzel Strg+E; aktiv aus aus aus aus aus – aktiv weicht ab in S4 | ❌ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Ebenen als Hintergrund | Kürzel Strg+Alt+E; aktiv aus aus aus aus aus | Kürzel Strg+Alt+E; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Transformieren | Kürzel –; aktiv an an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Transformieren > Frei transformieren | Kürzel Strg+T; aktiv aus an an an an | Kürzel Strg+T; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Transformieren > Skalieren | Kürzel –; aktiv aus an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Transformieren > Neigen | Kürzel –; aktiv aus an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Transformieren > Verzerren | Kürzel –; aktiv aus an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Transformieren > Numerisch transformieren | Kürzel Strg+Umschalt+T; aktiv aus an an an an | Kürzel Strg+Umschalt+T; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Transformieren > 180° drehen | Kürzel –; aktiv aus an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Transformieren > 90° im UZS drehen | Kürzel Strg+Umschalt+9; aktiv aus an an an an | Kürzel Strg+Umschalt+9; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Transformieren > 90° gegen UZS drehen | Kürzel Strg+Umschalt+7; aktiv aus an an an an | Kürzel Strg+Umschalt+7; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Transformieren > Horizontal spiegeln | Kürzel –; aktiv aus an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Transformieren > Vertikal spiegeln | Kürzel –; aktiv aus an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Transformieren > Transformationen entfernen | Kürzel –; aktiv aus an an an aus | Kürzel –; aktiv aus an an an aus | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Anordnen | Kürzel –; aktiv an an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Anordnen > In den Vordergrund | Kürzel Strg+Umschalt+Nach-oben-Taste; aktiv aus an an an an | Kürzel Strg+Umschalt+Oben; aktiv aus an an an an – Kürzel Strg+Umschalt+Nach-oben-Taste ≠ Strg+Umschalt+Oben | ⚠️ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Anordnen > Schrittweise nach vorne | Kürzel Strg+Nach-oben-Taste; aktiv aus an an an an | Kürzel Strg+Oben; aktiv aus an an an an – Kürzel Strg+Nach-oben-Taste ≠ Strg+Oben | ⚠️ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Anordnen > Schrittweise nach hinten | Kürzel Strg+Nach-unten-Taste; aktiv aus an an an an | Kürzel Strg+Unten; aktiv aus an an an an – Kürzel Strg+Nach-unten-Taste ≠ Strg+Unten | ⚠️ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Anordnen > In den Hintergrund | Kürzel Strg+Umschalt+Nach-unten-Taste; aktiv aus an an an an | Kürzel Strg+Umschalt+Unten; aktiv aus an an an an – Kürzel Strg+Umschalt+Nach-unten-Taste ≠ Strg+Umschalt+Unten | ⚠️ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Ausrichten | Kürzel –; aktiv an an an an an | Kürzel –; aktiv aus aus an aus aus | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Ausrichten > Links | Kürzel Strg+Alt+1; aktiv aus aus an aus aus | Kürzel Strg+Alt+1; aktiv aus aus an aus aus | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Ausrichten > Vertikal zentrieren | Kürzel Strg+Alt+2; aktiv aus aus an aus aus | Kürzel Strg+Alt+2; aktiv aus aus an aus aus | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Ausrichten > Rechts | Kürzel Strg+Alt+3; aktiv aus aus an aus aus | Kürzel Strg+Alt+3; aktiv aus aus an aus aus | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Ausrichten > Oben | Kürzel Strg+Alt+4; aktiv aus aus an aus aus | Kürzel Strg+Alt+4; aktiv aus aus an aus aus | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Ausrichten > Horizontal zentrieren | Kürzel Strg+Alt+5; aktiv aus aus an aus aus | Kürzel Strg+Alt+5; aktiv aus aus an aus aus | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Ausrichten > Unten | Kürzel Strg+Alt+6; aktiv aus aus an aus aus | Kürzel Strg+Alt+6; aktiv aus aus an aus aus | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Ausrichten > Breiten verteilen | Kürzel Strg+Alt+7; aktiv aus aus aus aus aus | Kürzel Strg+Alt+7; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Ausrichten > Höhen verteilen | Kürzel Strg+Alt+9; aktiv aus aus aus aus aus | Kürzel Strg+Alt+9; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Pfad in Auswahlrahmen konvertieren | Kürzel –; aktiv aus an an aus aus | Kürzel –; aktiv aus an an aus aus | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Pfade kombinieren | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Pfade kombinieren > Verbinden | Kürzel Strg+Umschalt+J; aktiv aus an an an aus | Kürzel Strg+Umschalt+J; aktiv aus an an an aus | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Pfade kombinieren > Teilen | Kürzel Strg+Alt+J; aktiv aus aus aus aus aus | Kürzel Strg+Alt+J; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Pfade kombinieren > Vereinigen | Kürzel Strg+Alt+U; aktiv an an an an an | Kürzel Strg+Alt+U; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Pfade kombinieren > Überschneiden | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Pfade kombinieren > Öffnen | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Pfade kombinieren > Zuschneiden | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Pfad ändern | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Pfad ändern > Vereinfachen | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Pfad ändern > Strich erweitern | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Pfad ändern > Pfad einlassen | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Pfad ändern > Präzise Füllung | Kürzel –; aktiv aus aus aus an aus | Kürzel –; aktiv aus aus aus an aus | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Pfad ändern > Anti-Alias-Füllung | Kürzel –; aktiv aus aus aus an aus | Kürzel –; aktiv aus aus aus an aus; ✓ – Häkchen – ≠ ✓ | ❌ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Pfad ändern > Weiche-Kante-Füllung | Kürzel –; aktiv aus aus aus an aus | Kürzel –; aktiv aus aus aus an aus | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > An Pixeln ausrichten | Kürzel Strg+K; aktiv aus an an aus aus | Kürzel Strg+K; aktiv aus an an aus aus | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Gruppieren | Kürzel Strg+G; aktiv aus an an an an | Kürzel Strg+G; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Gruppierung aufheben | Kürzel Strg+Umschalt+G; aktiv aus an an aus aus | Kürzel Strg+Umschalt+G; aktiv aus an an aus aus | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Text > Schrift | Kürzel –; aktiv an an an an an | Kürzel –; aktiv aus aus aus an aus | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe > Andere | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe > Kleiner | Kürzel Strg+Umschalt+,; aktiv aus aus aus an aus | Kürzel Strg+Umschalt+,; aktiv aus aus aus an aus | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe > Größer | Kürzel Strg+Umschalt+.; aktiv aus aus aus an aus | Kürzel Strg+Umschalt+.; aktiv aus aus aus an aus | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe > 8 | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe > 9 | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe > 10 | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe > 12 | Kürzel –; aktiv an an an an an; ✓ | Kürzel –; aktiv an an an an an; ✓ | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe > 14 | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe > 18 | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe > 24 | Kürzel –; aktiv an an an an an; ✓ | Kürzel –; aktiv an an an an an; ✓ | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe > 36 | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe > 48 | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe > 72 | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe > 96 | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe > 120 | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Stil | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Stil > Standard | Kürzel –; aktiv an an an an an; ✓ | Kürzel –; aktiv an an an an an; ✓ | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Stil > Faux Fett | Kürzel Strg+B; aktiv an an an an an | Kürzel Strg+B; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Stil > Faux Kursiv | Kürzel Strg+I; aktiv an an an an an | Kürzel Strg+I; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Stil > Unterstrichen | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Ausrichten | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Ausrichten > Links | Kürzel Strg+Alt+Umschalt+L; aktiv an an an an an; ✓ | Kürzel Strg+Alt+Umschalt+L; aktiv an an an an an; ✓ | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Ausrichten > Horizontal zentrieren | Kürzel Strg+Alt+Umschalt+C; aktiv an an an an an | Kürzel Strg+Alt+Umschalt+C; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Ausrichten > Rechts | Kürzel Strg+Alt+Umschalt+R; aktiv an an an an an | Kürzel Strg+Alt+Umschalt+R; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Ausrichten > Blocksatz | Kürzel Strg+Alt+Umschalt+J; aktiv an an an an an | Kürzel Strg+Alt+Umschalt+J; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Ausrichten > Oben | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Ausrichten > Vertikal zentrieren | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Ausrichten > Unten | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Ausrichten > Vertikaler Blocksatz | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > In Pfad einfügen | Kürzel –; aktiv aus aus aus aus aus | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > An Pfad anfügen | Kürzel Strg+Umschalt+Y; aktiv aus aus aus aus aus | Kürzel Strg+Umschalt+Y; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Von Pfad lösen | Kürzel –; aktiv aus aus aus aus aus | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Richtung | Kürzel –; aktiv an an an an an | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Richtung > Um Pfad drehen | Kürzel –; aktiv aus aus aus aus aus | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Richtung > Vertikal | Kürzel –; aktiv aus aus aus aus aus | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Richtung > Vertikal neigen | Kürzel –; aktiv aus aus aus aus aus | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Richtung > Horizontal neigen | Kürzel –; aktiv aus aus aus aus aus | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Richtung umkehren | Kürzel –; aktiv aus aus aus aus aus | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > In Pfade konvertieren | Kürzel Strg+Umschalt+P; aktiv aus aus aus an aus | Kürzel Strg+Umschalt+P; aktiv aus aus aus an aus | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Rechtschreibprüfung | Kürzel Umschalt+F7; aktiv an an an an an | Kürzel Umschalt+F7; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Rechtschreibprüfung einrichten | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Befehle > Gespeicherte Befehle verwalten | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M07_menue_befehle.png) |
| Befehle > Erweiterungen verwalten | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M07_menue_befehle.png) |
| Befehle > Skript ausführen | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M07_menue_befehle.png) |
| Befehle > Warnmeldungen zurücksetzen | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M07_menue_befehle.png) |
| Filter > Filter wiederholen | Kürzel Strg+Alt+Umschalt+X; aktiv aus aus aus aus aus | Kürzel Strg+Alt+Umschalt+X; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Andere | Kürzel –; aktiv an an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Andere > In Alphakanal konv. | Kürzel –; aktiv aus an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Andere > Ränder suchen | Kürzel –; aktiv aus an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Farbe anpassen | Kürzel –; aktiv an an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Farbe anpassen > Auto-Tonwertkorrektur | Kürzel –; aktiv aus an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Farbe anpassen > Farbton/Sättigung | Kürzel –; aktiv aus an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Farbe anpassen > Helligkeit/Kontrast | Kürzel –; aktiv aus an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Farbe anpassen > Kurven | Kürzel –; aktiv aus an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Farbe anpassen > Tonwertkorrektur | Kürzel –; aktiv aus an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Farbe anpassen > Umkehren | Kürzel –; aktiv aus an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Rauschen | Kürzel –; aktiv an an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Rauschen > Rauschen hinzufügen | Kürzel –; aktiv aus an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Scharfzeichnen | Kürzel –; aktiv an an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Scharfzeichnen > Scharfzeichnen | Kürzel –; aktiv aus an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Scharfzeichnen > Stark scharfzeichnen | Kürzel –; aktiv aus an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Scharfzeichnen > Unscharf maskieren | Kürzel –; aktiv aus an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Weichzeichnen | Kürzel –; aktiv an an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Weichzeichnen > Bewegungsunschärfe | Kürzel –; aktiv aus an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Weichzeichnen > Gaußscher Weichz. | Kürzel –; aktiv aus an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Weichzeichnen > Radiales Weichz. | Kürzel –; aktiv aus an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Weichzeichnen > Stärker weichzeichnen | Kürzel –; aktiv aus an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Weichzeichnen > Weichzeichnen | Kürzel –; aktiv aus an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Weichzeichnen > Zentriertes Weichz. | Kürzel –; aktiv aus an an an an | Kürzel –; aktiv aus an an an an | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Fenster > Fenster duplizieren | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Bedienfelder ausblenden | Kürzel F4; aktiv an an an an an | Kürzel F4; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Erweiterungen | Kürzel –; aktiv an an an an an | Kürzel –; aktiv aus aus aus aus aus | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Erweiterungen > Kuler | – | Kürzel – (nur Firepixel) – Erweiterung eines Drittanbieters (nicht nachgebaut) | ℹ️ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Erweiterungen > jQuery Mobile-Thema – anwendungsinterne Vorschau | – | Kürzel – (nur Firepixel) – Erweiterung eines Drittanbieters (nicht nachgebaut) | ℹ️ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Werkzeugleisten | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Werkzeugleisten > Standard | Kürzel –; aktiv an an an an an; ✓ | Kürzel –; aktiv an an an an an; ✓ | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Werkzeuge | Kürzel Strg+F2; aktiv an an an an an; ✓ | Kürzel Strg+F2; aktiv an an an an an; ✓ | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Eigenschaften | Kürzel Strg+F3; aktiv an an an an an; ✓ | Kürzel Strg+F3; aktiv an an an an an; ✓ | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Optimieren | Kürzel F6; aktiv an an an an an | Kürzel F6; aktiv an an an an an; ✓ | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Ebenen | Kürzel F2; aktiv an an an an an; ✓ | Kürzel F2; aktiv an an an an an; ✓ | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Allgemeine Bibliothek | Kürzel F7; aktiv an an an an an | Kürzel F7; aktiv an an an an an; ✓ | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > CSS-Eigenschaften | Kürzel Strg+F7; aktiv an an an an an | Kürzel Strg+F7; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Seiten | Kürzel F5; aktiv an an an an an | Kürzel F5; aktiv an an an an an; ✓ | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Status | Kürzel Umschalt+F2; aktiv an an an an an | Kürzel Umschalt+F2; aktiv an an an an an; ✓ | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Verlauf | Kürzel Umschalt+F10; aktiv an an an an an; ✓ | Kürzel Umschalt+F10; aktiv an an an an an; ✓ | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Automatische Formen | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an; ✓ | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Stile | Kürzel Strg+F11; aktiv an an an an an; ✓ | Kürzel Strg+F11; aktiv an an an an an; ✓ | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Dokumentbibliothek | Kürzel F11; aktiv an an an an an | Kürzel F11; aktiv an an an an an; ✓ | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > URL | Kürzel Alt+Umschalt+F10; aktiv an an an an an | Kürzel Alt+Umschalt+F10; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Farbmischer | Kürzel Umschalt+F9; aktiv an an an an an | Kürzel Umschalt+F9; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Farbfelder | Kürzel Strg+F9; aktiv an an an an an | Kürzel Strg+F9; aktiv an an an an an; ✓ | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Info | Kürzel Alt+Umschalt+F12; aktiv an an an an an | Kürzel Alt+Umschalt+F12; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Verhalten | Kürzel Umschalt+F3; aktiv an an an an an | Kürzel Umschalt+F3; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Suchen und ersetzen | Kürzel Strg+F; aktiv an an an an an | Kürzel Strg+F; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Ausrichten | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Eigenschaften für automatische Form | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Sonstige | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Sonstige > Farbpalette | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Sonstige > Bildbearbeitung | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Sonstige > Sonderzeichen | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Pfad | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Symboleigenschaften | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an; ✓ | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Überlappend | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Anordnen | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Hilfe > Firepixel-Hilfe | Kürzel F1; aktiv an an an an an | Kürzel F1; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M10_menue_hilfe.png) |
| Hilfe > Firepixel Support Center | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M10_menue_hilfe.png) |
| Hilfe > Firepixel-Programm zur Produktverbesserung | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M10_menue_hilfe.png) |
| Hilfe > Firepixel Exchange | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M10_menue_hilfe.png) |
| Hilfe > Erweiterungen verwalten | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M10_menue_hilfe.png) |
| Hilfe > Firepixel-Onlineforen | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M10_menue_hilfe.png) |
| Hilfe > Über Firepixel | Kürzel –; aktiv an an an an an | Kürzel –; aktiv an an an an an | ✅ | [Bild](screenshots/paritaet/b_M10_menue_hilfe.png) |
| Hilfe > Produktregistrierung | – | Kürzel – (nur Firepixel) – bewusst weggelassen (Lizenz/Registrierung des Originals) | ℹ️ | [Bild](screenshots/paritaet/b_M10_menue_hilfe.png) |
| Hilfe > Deaktivieren | – | Kürzel – (nur Firepixel) – bewusst weggelassen (Lizenz/Registrierung des Originals) | ℹ️ | [Bild](screenshots/paritaet/b_M10_menue_hilfe.png) |
| Hilfe > Aktualisierungen | – | Kürzel – (nur Firepixel) – bewusst weggelassen (Lizenz/Registrierung des Originals) | ℹ️ | [Bild](screenshots/paritaet/b_M10_menue_hilfe.png) |
| Datei > Zuletzt geöffnete Dateien > Keine | – | Kürzel – (nur Firepixel) – nur in Firepixel | ℹ️ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Ansicht > Design | – | Kürzel – (nur Firepixel) – nur in Firepixel | ℹ️ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Design > Normal | – | Kürzel – (nur Firepixel) – nur in Firepixel | ℹ️ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Design > Dunkel | – | Kürzel – (nur Firepixel) – nur in Firepixel | ℹ️ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Design > OLED | – | Kürzel – (nur Firepixel) – nur in Firepixel | ℹ️ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Sprache | – | Kürzel – (nur Firepixel) – nur in Firepixel | ℹ️ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Sprache > Deutsch | – | Kürzel – (nur Firepixel) – nur in Firepixel | ℹ️ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Sprache > English | – | Kürzel – (nur Firepixel) – nur in Firepixel | ℹ️ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Sprache > Testsprache (qps) | – | Kürzel – (nur Firepixel) – nur in Firepixel | ℹ️ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Sprache > Sprachordner öffnen | – | Kürzel – (nur Firepixel) – nur in Firepixel | ℹ️ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Auswählen > Motiv auswählen | – | Kürzel – (nur Firepixel) – nur in Firepixel | ℹ️ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Auswählen > Kante verfeinern | – | Kürzel – (nur Firepixel) – nur in Firepixel | ℹ️ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Befehle > KI-Modelle verwalten | – | Kürzel – (nur Firepixel) – nur in Firepixel | ℹ️ | [Bild](screenshots/paritaet/b_M07_menue_befehle.png) |
| Befehle > Testbefehl | – | Kürzel – (nur Firepixel) – nur in Firepixel | ℹ️ | [Bild](screenshots/paritaet/b_M07_menue_befehle.png) |
| Befehle > Testbefehl2 | – | Kürzel – (nur Firepixel) – nur in Firepixel | ℹ️ | [Bild](screenshots/paritaet/b_M07_menue_befehle.png) |
| Befehle > KI-Steuerkanal (MCP) | – | Kürzel – (nur Firepixel) – nur in Firepixel | ℹ️ | [Bild](screenshots/paritaet/b_M07_menue_befehle.png) |
| Filter > Farbe anpassen > Farbfüllung | – | Kürzel – (nur Firepixel) – nur in Firepixel | ℹ️ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Fenster > Arbeitsbereichlayouts | – | Kürzel – (nur Firepixel) – nur in Firepixel | ℹ️ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Arbeitsbereichlayouts > Erweiterter Modus | – | Kürzel – (nur Firepixel) – nur in Firepixel | ℹ️ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Arbeitsbereichlayouts > Symbolmodus | – | Kürzel – (nur Firepixel) – nur in Firepixel | ℹ️ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Arbeitsbereichlayouts > Symbolmodus mit Bedienfeldnamen | – | Kürzel – (nur Firepixel) – nur in Firepixel | ℹ️ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Arbeitsbereichlayouts > Aktuelles speichern | – | Kürzel – (nur Firepixel) – nur in Firepixel | ℹ️ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Arbeitsbereichlayouts > Standardarbeitsbereich | – | Kürzel – (nur Firepixel) – nur in Firepixel | ℹ️ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Erweiterungen > Keine Erweiterungen | – | Kürzel – (nur Firepixel) – nur in Firepixel | ℹ️ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |

## 2. Dialoge (Feld für Feld)

Original: Dialog über das Menü geöffnet (Nachrichten an Menüleiste und Menüfenster), alle Steuerelemente mit Tab-Folge, Standardknopf, Fokus, Markierung und Werten über die Win32-API gelesen (`re\_testdateien\ux\paritaet\dialoge\o_dialoge_bericht.txt`), danach abgebrochen. Firepixel: derselbe Befehl über `ui_action`, Bild über die Brücke. Lage: S1 = Testdokument ohne Auswahl.

| Nr | Dialog | Original | Firepixel | | Bild |
|---|---|---|---|---|---|
| D01 | Datei ▸ Neu | „Neues Dokument“, Fokus Breite ganz markiert; Werte zuletzt benutzt (hier 100×80); benutzerdefinierte Farbe **schwarz** (`CustomCanvasColor`); Zugriffstasten unterstrichen (B/H/A, W/T/n) | Fokus und Markierung ✅, „Leinwandgröße: 1.1M“ ✅; Werte 660×440 (nicht gemerkt); Farbfeld **weiß**; keine Zugriffstasten | 🔧 behoben, Nachmessung im echten Fenster offen (FV-4: Farbfeld „Benutzerdefiniert“ aus `CustomCanvasColor` (schwarz), Werte zuletzt benutzt (`DefaultDocSize`)) | [Bild](screenshots/paritaet/b_D01_neues_dokument.png) |
| D02 | Datei ▸ Neu von Vorlage… | Windows-Dateidialog „Neu von Vorlage“, Startordner = Vorlagenordner | Windows-Dialog „Öffnen“ ohne Vorlagenordner, links oben am Bildschirm (0,0) statt über dem Programmfenster | ❌ | – |
| D03 | Datei ▸ Öffnen… | Dateidialog „Öffnen“ zentriert über dem Hauptfenster; Startordner = zuletzt benutzter Ordner (`LastOpenLocation`) | nativer Dialog „Öffnen“ bei (0,0), ohne Besitzerfenster; Filter siehe Katalog UX-B F02 | ❌ | – |
| D04 | Datei ▸ Speichern unter… | Dateidialog mit 11 Dateitypen, Kästchen „.fw.png anhängen“ (an), „Als Kopie speichern“, „XMP einschließen“, Knopf „Optionen…“; Startordner zuletzt benutzt („06“); Name „Unbenannt-77.fw“ markiert | nativer Dialog ohne Zusatzoptionen, nur FW-PNG, bei (0,0) | ❌ | – |
| D05 | Datei ▸ Als Vorlage speichern… | Speichern-Dialog im Vorlagenordner | nativer Speichern-Dialog ohne Vorlagenordner | ⚠️ | – |
| D06 | Datei ▸ Importieren… | Dateidialog „Importieren“ (Filter aller Importformate) | nativer Dialog „Importieren“ bei (0,0) | ⚠️ | – |
| D07 | Datei ▸ Exportieren… | **Windows-Dateidialog** „Exportieren“ mit Ordnerwahl, Dateiname, „Exportieren:“ (Nur Bilder …), „HTML:“, „Segmente:“, „Seiten:“ (Aktuelle Seite), „Nur ausgewählte Segmente“, „Nur aktueller Status“, „Bereiche ohne Segmente einschließen“, „Optionen…“ | eigener kleiner Dialog: „Exportieren:“, „Dateiname“, „Segmente exportieren“, „Optionen…“, „Exportieren…“ – **ohne Ordnerwahl, HTML, Seiten, Status, Bereiche** | ❌ | [Bild](screenshots/paritaet/b_D07_exportieren.png) |
| D08 | Datei ▸ Bildvorschau… | Register Optionen/Datei/Animation; Vorschau-Kästchen, „Gespeicherte Einstellungen“ mit +; Werkzeuge Zeiger/Exportbereich/Zoom, Zoomliste (100 %), 1/2/4-fach-Ansicht, Statusknöpfe, Hintergrund-Farbe; Exportieren…/OK/Abbrechen; **Esc schließt nicht** (Abbrechen hat ID 4681 statt IDCANCEL) | Register, Format/Palette/Farben/Verlust/Matte, Vorschau mit Größenanzeige, „Einpassen“-Liste; **ohne** gespeicherte Einstellungen, Werkzeuge, Zoomstufen, 1/2/4-fach, Status, Hintergrund; Esc schließt | ❌ | [Bild](screenshots/paritaet/b_D08_bildvorschau.png) |
| D09 | Datei ▸ HTML aktualisieren… | Dateidialog „HTML-Datei suchen:“ | nativer Dialog „HTML aktualisieren“ | ⚠️ | – |
| D10 | Datei ▸ Datei-Info… | eigenes Fenster (Datei-Info/XMP-Bedienfeld, 680×620) | Dialog „Datei-Info“ | ❔ | – |
| D11 | Datei ▸ Exportassistent… | Assistent: Erklärtext, „Was möchten Sie als Nächstes tun?“ (Ein Exportformat auswählen / Aktuelle Formateinstellungen auswerten), „Ziel-Exportdateigröße“ + Feld (0 k, grau), **Weiter**/Abbrechen | ein Formular: Ziel Web/Bildbearbeitung/Webseite, „Exportgröße festlegen“, Zielgröße 32 K, OK/Abbrechen – **kein Assistent** | ❌ | [Bild](screenshots/paritaet/b_D11_exportassistent.png) |
| D12 | Datei ▸ Batch-Verarbeitung… | Dialog „Batch“ mit Dateiliste (Hinzufügen/Alle hinzufügen/Entfernen/„Aktuell geöffnete Dateien einschließen“), dann Assistent | nativer Dateidialog „Zu verarbeitende Dateien“ | ❌ | – |
| D13 | Datei ▸ HTML-Einrichtung… | Register **Allgemein/Tabelle/Dokumentspezifisch**; HTML-Stil, Erweiterung (Liste .htm), Kommentare, Kleinbuchstaben, CSS für Popupmenüs, CSS in externe Datei, Hintergrundbild + Durchsuchen, Wiederholen, Anhang, Seitenausrichtung | ein Formular: Erweiterung (Textfeld), XHTML-kompatibel, Bildordner, Segmente an Hilfslinien, nicht angegebene Segmente verwerfen, Demo-HTML – ohne Register und die meisten Felder | ❌ | [Bild](screenshots/paritaet/b_D13_html_einrichtung.png) |
| D14 | Bearbeiten ▸ Objekt einfügen ▸ Neues Symbol… | Dialog „In Symbol konvertieren“: Name (markiert), Typ Grafik/Animation/Schaltfläche, „Hilfslinien für 9-Slice-Skalierung aktivieren“, „In allgemeiner Bibliothek speichern“ (grau) | Dialog „Neues Symbol“ nur mit Name | ❌ | [Bild](screenshots/paritaet/b_D14_neues_symbol.png) |
| D15 | Bearbeiten ▸ HTML-Code kopieren… | Assistent „HTML-Code kopieren“ (Seiten mit Optionen, Weiter/Zurück) | **kein Dialog**, kopiert sofort und meldet „HTML-Code in die Zwischenablage kopiert“ | ❌ | – |
| D16 | Bearbeiten ▸ Voreinstellungen… | Kategorien Allgemein/Bearbeitung/Hilfslinien und Raster/Text/Photoshop-Datei importieren/Starten und Bearbeiten/Zusatzmodule; Allgemein gruppiert (Dokumentoptionen, Speichern, Benutzeroberfläche, Standardfarben), „Hilfe“-Knopf | gleiche Kategorien (Reihenfolge anders: Zusatzmodule vor Starten), Felder ungruppiert, zusätzlich Entwickleroptionen/Design/Sprache, ohne „Hilfe“ | ⚠️ | [Bild](screenshots/paritaet/b_D16_voreinstellungen.png) |
| D17 | Bearbeiten ▸ Tastaturkurzbefehle… | Satzwahl + 4 Symbolknöpfe (Duplizieren, Umbenennen, Exportieren als HTML, Löschen), „Befehle:“-Liste, **Baum** der Menüs (+/–), Beschreibung, Kurzbefehle mit +/–, „Taste drücken“ + Ändern | Satzwahl mit Textknöpfen, Liste flach mit Suche, Kürzel, Zuweisen | ⚠️ | [Bild](screenshots/paritaet/b_D17_tastaturkurzbefehle.png) |
| D18 | Ansicht ▸ Raster ▸ Raster bearbeiten… | Farbe **cyan** (`GridColor` #00ffff), Abstände 36 mit Popup-Schieber ▾, Pixel | Farbe **dunkelgrau**, zusätzlich „Raster anzeigen“/„Am Raster ausrichten“, Abstände ohne Popup-Schieber | ❌ | [Bild](screenshots/paritaet/b_D18_raster_bearbeiten.png) |
| D19 | Modifizieren ▸ Leinwand ▸ Bildgröße… | Druckgröße mit **3 Nachkommastellen und Punkt** (6.944/5.556), Kettensymbole für Proportionen, Kästchen alle an | 2 Nachkommastellen mit Komma (6,94), ohne Kettensymbole | ⚠️ | [Bild](screenshots/paritaet/b_D19_bildgroesse.png) |
| D20 | Modifizieren ▸ Leinwand ▸ Leinwandgröße… | Anker 3×3 mit **Pfeilsymbolen, Mitte gewählt**; „B: 500“/„H: 400“ rechts neben den Feldern; „Nur aktuelle Seite“ in der Gruppe | Anker als 9 leere Kästchen, **gewählter Anker nicht erkennbar** | ❌ | [Bild](screenshots/paritaet/b_D20_leinwandgroesse.png) |
| D21 | Modifizieren ▸ Leinwand ▸ Leinwandfarbe… | Optionen untereinander, Farbfeld schwarz unter „Benutzerdefiniert“ | Optionen nebeneinander, Feld „Farbe“ weiß | 🔧 behoben, Nachmessung im echten Fenster offen (FV-4: Farbfeld aus `CustomCanvasColor` (schwarz), gemerkt bei OK mit „Benutzerdefiniert“; Anordnung untereinander offen) | [Bild](screenshots/paritaet/b_D21_leinwandfarbe.png) |
| D22 | Modifizieren ▸ Selektives JPEG ▸ Einstellungen… | Qualität (90) **grau, solange „Selektive Qualität aktivieren“ aus**; „Überlagerungsfarbe“; Text-/Schaltflächenqualität ebenfalls grau | alles aktiv, **ohne Überlagerungsfarbe** | ❌ | [Bild](screenshots/paritaet/b_D22_selektives_jpeg.png) |
| D23 | Text ▸ Größe ▸ Andere… | Kombifeld mit Liste ▾, Vorgabe 24 (zuletzt benutzt), auch ohne Textauswahl verfügbar | Zahlenfeld 12 „Punkt“ ohne Liste | ❌ | [Bild](screenshots/paritaet/b_D23_textgroesse.png) |
| D24 | Text ▸ Rechtschreibprüfung einrichten… | Wörterbuchliste mit Kästchen, Pfad zum persönlichen Wörterbuch + Ordnerknopf, „Persönliches Wörterbuch bearbeiten“, 4 Optionen (doppelte Wörter, Zahlen, Internet-/Dateiadressen, GROSSBUCHSTABEN) | Sprachkürzel als Text, Ordnerfeld, 2 Optionen, Wörterbuch als Textfeld | ❌ | [Bild](screenshots/paritaet/b_D24_rechtschreibung_einrichten.png) |
| D25 | Befehle ▸ Gespeicherte Befehle verwalten… | ohne gespeicherte Befehle nur **Meldung** (Info-Symbol): „Befehle werden dem Verlauf-Bedienfeld entnommen. Zurzeit sind dort keine gespeicherten Befehle vorhanden.“ | leere Verwaltungsliste mit Umbenennen/Löschen/Fertig | ⚠️ | [Bild](screenshots/paritaet/b_D25_gespeicherte_befehle.png) |
| D26 | Modifizieren ▸ Transformieren ▸ Numerisch transformieren… (1 Rechteck) | Kombi Skalieren/Größe ändern/Drehen; Felder ohne Beschriftung mit ↔/↕-Symbol, Kette, „B: 100 / H: 80“ (Objektgröße) rechts; Kästchen an | Felder „Breite“/„Höhe“, **keine Anzeige der Objektgröße B/H**, keine Kette | ⚠️ | [Bild](screenshots/paritaet/b_D26_numerisch_transformieren.png) |
| D27 | Modifizieren ▸ Symbol ▸ In Symbol konvertieren… | Name, Typ (Grafik/Animation/Schaltfläche), Optionen 9-Slice, „In allgemeiner Bibliothek speichern“ | gleiche Felder („Für allgemeine Bibliothek speichern“) | ✅ | [Bild](screenshots/paritaet/b_D27_in_symbol_konvertieren.png) |
| D28 | Modifizieren ▸ Animation ▸ Auswahl animieren… | „Animieren“: Status 5, **Verschieben 72, Richtung 0**, Skalieren auf 100, Deckkraft 100 bis 100, Drehen 0, Nach rechts/Nach links; Zahlenfelder mit Popup-Schieber ▾ | Bewegung 72, **Richtung 45**, Schieber als feste Leisten unter jedem Feld, „Im/Entgegen dem Uhrzeigersinn“ | 🔧 behoben, Nachmessung im echten Fenster offen (FV-4: Popup-Schieber ▾ senkrecht wie im Inspektor, Richtung 0) | [Bild](screenshots/paritaet/b_D28_auswahl_animieren.png) |
| D29 | Modifizieren ▸ Pfad ändern ▸ Vereinfachen… | „Menge:“ **10** mit Popup-Schieber ▾ | „Betrag:“ **1** ohne Schieber | 🔧 behoben, Nachmessung im echten Fenster offen (FV-4: Vorgabe 10 aus `SimplifyAmount`, gemerkt, Popup-Schieber ▾ senkrecht; Beschriftung „Menge“ offen) | [Bild](screenshots/paritaet/b_D29_vereinfachen.png) |
| D30 | Modifizieren ▸ Pfad ändern ▸ Strich erweitern… | Breite 10 mit ▾; Ecken als **3 Symbolknöpfe** (drittes gewählt); „Winkelgrenze“ 10.00; „Endkappen“ als 3 Symbolknöpfe (erstes gewählt) | Optionsfelder mit Text (Eckig gewählt), „Gehrungsgrenze“ 10, „Enden“; Breite ohne ▾ | 🔧 behoben, Nachmessung im echten Fenster offen (FV-4: Breite 10/Ecken/Endkappen/Winkelgrenze aus `Expand*` gemerkt, Symbolknöpfe, Popup ▾ senkrecht, 2 Nachkommastellen; Beschriftungen offen) | [Bild](screenshots/paritaet/b_D30_strich_erweitern.png) |
| D31 | Modifizieren ▸ Pfad ändern ▸ Pfad einlassen… | Richtung Innen/Außen **untereinander**, Breite **10** mit ▾, Ecken als Symbolknöpfe, Winkelgrenze 10.00 | Breite **1**, Ecken als Text-Optionsfelder, Gehrungsgrenze 10 | 🔧 behoben, Nachmessung im echten Fenster offen (FV-4: Breite 10/Richtung/Ecken/Winkelgrenze aus `Inset*` gemerkt, Symbolknöpfe, Popup ▾ senkrecht; Richtung untereinander offen) | [Bild](screenshots/paritaet/b_D31_pfad_einlassen.png) |
| D32 | Modifizieren ▸ Pfad in Auswahlrahmen konvertieren… | Dialog „Pfad in Auswahlrahmen konvertieren“ (Kante Hart/Geglättet/Weiche Kante + Breite) | **kein Dialog** | ❌ | – |
| D33 | Modifizieren ▸ Symbol ▸ Instanzen tweenen… (ohne 2 Instanzen) | Meldung „Wählen Sie zwei oder mehr Instanzen desselben Bibliothekssymbols aus, bevor Sie den Tween-Befehl verwenden.“ mit Kästchen „Nicht mehr anzeigen“ (Menüpunkt deshalb immer aktiv) | Menüpunkt grau; per Befehl geöffnet: Dialog „Instanzen tweenen“ ohne Prüfung | ❌ | [Bild](screenshots/paritaet/b_D33_instanzen_tweenen.png) |
| D34 | Modifizieren ▸ Popupmenü ▸ Popupmenü hinzufügen… (ohne Hotspot/Segment) | Rückfrage „Verhalten werden über das Verhalten-Bedienfeld auf Hotspot- oder Segmentobjekte angewendet. Hotspot oder Segment erstellen?“ mit **Hotspot** (Standard) / **Segment** / Abbrechen | Menüpunkt grau; per Befehl: Pop-up-Menü-Editor direkt | ❌ | [Bild](screenshots/paritaet/b_D34_popupmenue_hinzufuegen.png) |
| D35 | Auswählen ▸ Weiche Kante… (Pixelauswahl) | „Weiche Auswahlkante“, Radius 10 Pixel, Fokus markiert | gleich | ✅ | [Bild](screenshots/paritaet/b_D35_weiche_kante.png) |
| D36 | Auswählen ▸ Auswahlrahmen erweitern… | „Erweitern um:“ **10** Pixel | **1** | 🔧 behoben, Nachmessung im echten Fenster offen (FV-4: Vorgabe 10 aus `ExpandSelectionAmount`, gemerkt) | [Bild](screenshots/paritaet/b_D36_auswahlrahmen_erweitern.png) |
| D37 | Auswählen ▸ Auswahlrahmen verkleinern… | „Verkleinern um:“ **10** Pixel | **1** | 🔧 behoben, Nachmessung im echten Fenster offen (FV-4: Vorgabe 10 aus `ContractSelectionAmount`, gemerkt) | [Bild](screenshots/paritaet/b_D37_auswahlrahmen_verkleinern.png) |
| D38 | Auswählen ▸ Auswahlrahmen begrenzen… | „Breite:“ **10** Pixel | „Rand:“ **1** | 🔧 behoben, Nachmessung im echten Fenster offen (FV-4: Vorgabe 10 aus `BorderSelectionAmount`, gemerkt; Beschriftung „Breite“ offen) | [Bild](screenshots/paritaet/b_D38_auswahlrahmen_begrenzen.png) |
| D39 | Auswählen ▸ Auswahlrahmen glätten… | „Auswahlradius:“ **10** Pixel | „Stichprobenradius:“ **1** | 🔧 behoben, Nachmessung im echten Fenster offen (FV-4: Vorgabe 10 aus `SmoothSelectionAmount`, gemerkt; Beschriftung offen) | [Bild](screenshots/paritaet/b_D39_auswahlrahmen_glaetten.png) |
| D40 | Auswählen ▸ Bitmap-Auswahl speichern… | „Auswahl speichern“: **Ziel** (Dokument-Liste, Auswahl „Neu“ bzw. vorhandene, Name „Standard“), **Vorgang** (Neue Auswahl / Zur Auswahl hinzufügen / Aus Auswahl entfernen / Schnittmenge, ohne vorhandene grau) | nur „Name: Auswahl 1“ | ❌ | [Bild](screenshots/paritaet/b_D40_auswahl_speichern.png) |
| D41 | Filter ▸ Farbe anpassen ▸ Farbton/Sättigung… (Bitmap) | Farbton/Sättigung/Helligkeit mit Schiebern, „Kolorieren“, Vorschau | gleich, Kästchen heißt „Färben“ | 🔧 behoben, Nachmessung im echten Fenster offen (FV-4: Schieber Farbton/Sättigung mit Kennlinie des Originals; Beschriftung „Kolorieren“ offen) | [Bild](screenshots/paritaet/b_D41_farbton_saettigung.png) |
| D42 | Filter ▸ Farbe anpassen ▸ Helligkeit/Kontrast… | Helligkeit/Kontrast 0 mit Schiebern, Vorschau | gleich | ✅ | [Bild](screenshots/paritaet/b_D42_helligkeit_kontrast.png) |
| D43 | Filter ▸ Farbe anpassen ▸ Kurven… | Titel **„Kurven“**; Kanal, Kurve mit Verlaufsleisten links/unten, **3 Pipetten** (Schwarz/Grau/Weißpunkt), Auto, Eingabe/Ausgabe als Felder | Titel „Gradationskurven“, **ohne Pipetten** und Verlaufsleisten, Eingabe/Ausgabe nur Text, zusätzlich „Zurücksetzen“ | ❌ | [Bild](screenshots/paritaet/b_D43_kurven.png) |
| D44 | Filter ▸ Farbe anpassen ▸ Tonwertkorrektur… | Kanal, **Histogramm** der Bitmap, Eingabetonwerte (3 Felder) + Regler, Ausgabetonwerte + Regler, Auto, **3 Pipetten** | Histogramm **leer**, Beschriftung „Tonwertspreizung/Tonwertumfang“, ohne Pipetten | ❌ | [Bild](screenshots/paritaet/b_D44_tonwertkorrektur.png) |
| D45 | Filter ▸ Rauschen ▸ Rauschen hinzufügen… | „Menge:“ 50 mit Popup-Schieber ▾, Farbe, Vorschau | „Stärke:“ mit fester Schieberleiste | 🔧 behoben, Nachmessung im echten Fenster offen (FV-4: Popup-Schieber ▾ senkrecht; Beschriftung „Menge“ offen) | [Bild](screenshots/paritaet/b_D45_rauschen.png) |
| D46 | Filter ▸ Scharfzeichnen ▸ Unscharf maskieren… | „Scharfzeichnungsmenge“ 50, Pixelradius 4.1 (Schieber bei ~⅓ – **nicht linear**), Schwellenwert 0 | „Schärfe“ 50 %, Pixelradius 4,1 mit Schieber **ganz links** (linear 0,1…250) | 🔧 behoben, Nachmessung im echten Fenster offen (FV-4: Schieber mit Kennlinie des Originals (Menge 0…170, Radius 0…145 stückweise, Schwelle 0…172); Beschriftung offen) | [Bild](screenshots/paritaet/b_D46_unscharf_maskieren.png) |
| D47 | Filter ▸ Weichzeichnen ▸ Bewegungsunschärfe… | Winkel 90 und **„Umfang“** 10 mit Popup-Schieber ▾ | Winkel/„Abstand“ mit fester Leiste | 🔧 behoben, Nachmessung im echten Fenster offen (FV-4: Popup-Schieber ▾ senkrecht, Vorschau gemerkt (`MotionBlurShowPreview`); Beschriftung „Umfang“ offen) | [Bild](screenshots/paritaet/b_D47_bewegungsunschaerfe.png) |
| D48 | Filter ▸ Weichzeichnen ▸ Gaußscher Weichzeichner… | „Weichzeichnungsradius“ 4.1, Schieber bei ~⅓ (**nicht linear**) | „Radius“ 4,1, Schieber fast ganz links (linear) | 🔧 behoben, Nachmessung im echten Fenster offen (FV-4: Radius-Schieber mit Kennlinie des Originals (4,1 → Position 41/145); Beschriftung offen) | [Bild](screenshots/paritaet/b_D48_gauss.png) |
| D49 | Filter ▸ Weichzeichnen ▸ Radiales Weichzeichnen… | „Anzahl“/„Qualität“ mit Popup-Schieber ▾ | „Stärke“/„Qualität“ mit fester Leiste | 🔧 behoben, Nachmessung im echten Fenster offen (FV-4: Popup-Schieber ▾ senkrecht, Vorschau gemerkt (`RadialBlurShowPreview`); Beschriftung „Anzahl“ offen) | [Bild](screenshots/paritaet/b_D49_radial.png) |
| D50 | Filter ▸ Weichzeichnen ▸ Zentriertes Weichzeichnen… | „Anzahl“ 30 / „Qualität“ 20 mit Popup-Schieber ▾ | „Stärke“/„Qualität“ mit fester Leiste | 🔧 behoben, Nachmessung im echten Fenster offen (FV-4: Popup-Schieber ▾ senkrecht, Vorschau gemerkt (`ZoomBlurShowPreview`); Beschriftung „Anzahl“ offen) | [Bild](screenshots/paritaet/b_D50_zentriert.png) |

## 3. Bedienfeldmenüs (≡)

Original: Reiter und ≡ mit echter Maus angeklickt (Vordergrundprüfung), Menü per `MN_GETHMENU` gelesen (`re\_testdateien\ux\paritaet\panels\o_panelmenues.txt`), Lage „1 Rechteck“. Firepixel: `build_panel_menu` in derselben Lage (Sonde `paritaet_bedienfeldmenues`). Das Export-Format der Testdokumente war verschieden (Original PNG32, Firepixel GIF), deshalb sind die Palettenbefehle im Optimieren-Menü nicht vergleichbar.

| Bedienfeld | Original | Firepixel | | Bild |
|---|---|---|---|---|
| alle | letzter Eintrag „Hilfe“; Gruppe schließen über das Reiter-Kontextmenü | zusätzlich „Schließen“ und „Registerkartengruppe schließen“ im ≡-Menü | ⚠️ | – |
| Optimieren | Einträge wie Firepixel; „Farbfeld-Rückmeldung anzeigen“ **✓**; „Progressives JPEG“/„JPEG-Ränder scharfzeichnen“ **grau** (Format kein JPEG) | Rückmeldung ohne ✓; JPEG-Einträge aktiv, „JPEG-Ränder scharfzeichnen ✓“ und „Nicht verwendete Farben entfernen ✓“ bei GIF | ❌ | [Bild](screenshots/paritaet/b_P_menue_optimieren_original.png) |
| Verlauf | „Ausgewählte Schritte wiederholen“, „Schritte kopieren“, „Als Befehl speichern…“ **grau** ohne gewählte Schritte; „Verlauf löschen“ aktiv | alle aktiv | ❌ | [Bild](screenshots/paritaet/b_P_menue_verlauf_original.png) |
| Seiten | bei 1 Seite: „Seite löschen“ **aktiv**; „Mit Masterseite verknüpfen“, „Masterseite zurücksetzen“ grau; „Nummerierung ✓“ | „Seite löschen“ grau; sonst gleich | ⚠️ | [Bild](screenshots/paritaet/b_P_menue_seiten_original.png) |
| Status | bei 1 Status: „Status löschen“ **aktiv**, „In Status kopieren…“ **grau**, „Auf Status verteilen“ **aktiv**; ✓ bei Autom. zuschneiden/Nummerierung/Auto-Differenz | „Status löschen“ grau, „In Status kopieren…“ aktiv, „Auf Status verteilen“ grau | ❌ | [Bild](screenshots/paritaet/b_P_menue_status_original.png) |
| Ebenen | 27 Einträge; „Maske bearbeiten“ **grau** ohne Maske | gleiche Einträge und Zustände, nur „Maske bearbeiten“ aktiv | ⚠️ | [Bild](screenshots/paritaet/b_P_menue_ebenen_original.png) |
| Stile | ohne gewählten Stil **grau**: Stil duplizieren…, Stile löschen, Stil umbenennen…, Stil neu definieren, Abweichungen löschen, Verknüpfung mit Stil aufheben, Stiloptionen…, Stilbibliothek speichern | alle aktiv | ❌ | [Bild](screenshots/paritaet/b_P_menue_stile_original.png) |
| Farbfelder | Einträge, ✓ Farbwürfel, „Aktuelle Exportpalette“ grau (PNG32) | gleich (Exportpalette bei GIF aktiv) | ✅ | [Bild](screenshots/paritaet/b_P_menue_farbfelder_original.png) |
| Ausrichten, Farbpalette (Flash-Bedienfelder) | kein ≡-Menü | – | ❔ | – |

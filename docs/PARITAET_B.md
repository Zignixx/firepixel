# Paritätsabgleich B: Menüs, Dialoge, Bedienfelder, Fenster (Original ↔ Firepixel)

## Übergabe (Stand 12.10.2026 ≈ 04:30, Agent UX-B, auf Nutzerwunsch angehalten)

### Stand und Zahlen

| Bereich | geprüft | ✅ | ⚠️ | ❌ | ℹ️/❔ | Abschnitt |
|---|---:|---:|---:|---:|---:|---|
| Hauptmenüs (jeder Eintrag in 5 Lagen: Kürzel, aktiv/grau, Häkchen) | 350 Zeilen | 311 | 5 | 5 | 29 | §1 |
| Dialoge Feld für Feld | 50 | 3 | 11 | 19 | 1 ⏳ + 16 🔧 | §2 (D01–D25 neu gemessen von PM-5: 0 ✅ / 10 ⚠️ / 14 ❌ / 1 ⏳; D26–D50 Stand 04:30 mit 🔧 aus FV-4/FV-5) |
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
| I18N-ENGINE / EB | „Wiederholen“ (Strg+Y) im Original aktiv mit „Wiederholen: <letzter Schritt>“ | 🔧 behoben: `edit.redo` wiederholt ohne zurückgenommenen Schritt den letzten Befehl (EB c67a402); das Menü zeigt jetzt auch dann „Wiederholen: <letzter Schritt>“ (FV-5, `menu.rs` über `Session::undo_redo_labels`); Nachprüfung im echten Fenster offen |
| EB | Warnungen bei immer aktiven Befehlen ohne Ziel (Wortlaut Z01–Z18) | 🔧 behoben (FV-5): Z01–Z09, Z14–Z18 Meldung mit Wortlaut, Warnsymbol und „Nicht mehr anzeigen“ (Voreinstellungen `DisableNotice…`, Warnmeldungen/Warndialoge zurücksetzen schalten sie wieder ein); Z10 Rückfrage Hotspot/Segment/Abbrechen; Z11 still, Z12/Z13 Verlaufsschritt „JPEG-Maske laden/löschen“; Nachprüfung im echten Fenster offen |
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

**Nachmessung D01–D25 (PM-5, 12.10. nachmittags):** beide Programme neu, Feld für Feld. Original: eigenes Testdokument `pm5_test.fw` (500 × 400, R1, R2, T1 „Hallo“ 24 pt, B1; am Ende ohne Speichern verworfen), Hauptfenster 1032 × 748. Dialoge über das Menü geöffnet, Felder/Tab-Folge/Fokus/Standardknopf per Win32 gelesen, Tasten (Tab, Enter, Esc) als Fensternachricht an das fokussierte Element (die RDP-Sitzung hatte kein Vordergrundfenster, echte SendInput-Eingaben waren nicht möglich), Wirkung und Rückgängig per JSF bzw. Rückgängig-Befehl, Bilder per PrintWindow. Firepixel: eigene Debug-Instanz aus einem sauberen Worktree von HEAD (`ecf43b3`), eigenes APPDATA, `--mcp`, gleiches Dokument, gleiches Fenster 1032 × 748; echtes Fenster, Brücke (`ui_find`/`ui_input`/`screenshot`, Tab/Enter/Esc/Strg+Z als Tasten). Bilder `docs\screenshots\paritaet\pm5_D<nr>_*.png` (links Original, rechts Firepixel; Stellen mit dem Produktnamen im Original sind grau überdeckt, Dateidialoge des Originals nicht abgelegt). Nicht gemessen: Popup-Schieber ▾ des Originals (D18, D23) mit der Maus (kein Vordergrund); in Firepixel fehlen sie in beiden Dialogen ohnehin.

**DLG-1 (12.10. abends, umgesetzt, Nachprüfung im echten Fenster ausstehend, NACHPRUEFUNG):** Zahlenfelder nehmen in allen Dialogen wieder Eingaben an, auch ohne Fensterfokus (`fc7df14`). Tab-Folge (`173bca1`): Titelleiste und Schließen-Kreuz nie Tab-Stopp; Startfokus erstes Eingabefeld bzw. erste Optionsgruppe, sonst Standardknopf; D11/D13/D22/D24 starten wie gemessen auf OK; Optionsgruppen = 1 Stopp mit Pfeiltasten; D01 … › OK › Abbrechen › Vorlagen…. Die Bewertung der Zeilen bleibt, weil dort weitere Abweichungen offen sind (D19/D20/D01-Rest, Verlaufsschritte → DLG-1b; D16 Kategorieliste per Tab, D20 Anker per Tab offen).

**DLG-2 (12.10. abends, umgesetzt, Nachprüfung im echten Fenster ausstehend, NACHPRUEFUNG):** `9469497`. D18 Raster bearbeiten: nur Farbe und zwei Abstände mit ↔/↕-Doppelpfeilen, keine Kästchen, Lage nach Vorlage 5984 (151 × 89 DLU ≈ 227 × 145 px Client), Farbfeld kein Tab-Stopp; Popup-Schieber ▾ fehlt weiter (im Original nicht gemessen, Vorlage ohne Bereich; Befund DLG-2), Verlaufsschritt → DLG-1b. D21 Leinwandfarbe: Optionen untereinander, „Benutzerdefiniert:“, Farbfeld eingerückt darunter, 230 px. D22 Selektives JPEG: Qualität neben dem Haken, „Überlagerungsfarbe:“ (`SelectiveJPEGColor`, #ff0080, bei OK gemerkt), Trennlinie; ohne Haken Qualität, Überlagerungsfarbe und beide Kästchen gesperrt. D23 Textgröße: Wert = Größe des gewählten Texts, sonst zuletzt bestätigter Wert, sonst Werkzeugvorgabe; Popup-Schieber 8..96, keine Einheit, OK/Abbrechen rechts untereinander (183 × 50 DLU).

| Nr | Dialog | Original | Firepixel | | Bild |
|---|---|---|---|---|---|
| D01 | Datei ▸ Neu | „Neues Dokument“ 396 × 312, mittig über dem Hauptfenster; Fokus Breite, ganz markiert; Werte zuletzt bestätigt (OK mit 123 × 77, zweites Öffnen 123 × 77); Gruppentitel live („Leinwandgröße: 37.0K“); Benutzerdefiniert schwarz; Zugriffstasten unterstrichen; Tab: Breite › Höhe › Auflösung › 3 Einheiten › Weiß (Optionsgruppe = 1 Stopp) › OK › Abbrechen › Vorlagen…; Enter = OK (neues Dokument), Esc = Abbrechen | 385 × 313, mittig; Fokus/Markierung, Gruppentitel „37.0K“, gemerkte Werte 123 × 77, Benutzerdefiniert schwarz, Enter/Esc gleich. Abweichend: keine Zugriffstasten (Tab-Folge seit DLG-1 `173bca1` wie im Original: Optionsgruppe 1 Stopp, Farbfeld kein Stopp, OK › Abbrechen › Vorlagen…; Nachprüfung ausstehend); frisches Profil 660 × 440 | ⚠️ PM-5, 12.10., echtes Fenster (Firepixel: Brücke) | [Bild](screenshots/paritaet/pm5_D01_neues_dokument.png) |
| D02 | Datei ▸ Neu von Vorlage… | Dateidialog „Neu von Vorlage“ 785 × 606, mittig über dem Hauptfenster; Startordner = Vorlagenordner; Filter nur FW-PNG; Vorschau; Esc schließt | Windows-Dialog mit Titel **„Öffnen“** (Mehrfachauswahl, alle Formate), Startordner „Dokumente“, 900 × 565 an der **linken oberen Ecke** des Hauptfensters (Besitzer gesetzt, aber nicht mittig); Esc schließt | 🔧 umgesetzt, Nachprüfung ausstehend (DLG-3: Titel „Neu von Vorlage“, eine Datei, nur „FW-PNG (*.png)“, Startordner Vorlagenordner (`%APPDATA%\Firepixel\Templates`)); vorher ❌ PM-5, 12.10., echtes Fenster | [Bild](screenshots/paritaet/pm5_D02_neu_von_vorlage.png) |
| D03 | Datei ▸ Öffnen… | „Öffnen“ mittig; Zusatzknöpfe „Als ‚Unbenannt‘ öffnen“, „Als Animation öffnen“; Vorschau; Startordner zuletzt benutzt (`LastOpenLocation`); 12 Filter | „Öffnen“ links oben am Hauptfenster; Filter „Alle lesbaren Formate“ + 18 Einzelformate + „Alle Dateien“; Startordner „Dokumente“; ohne Zusatzknöpfe und Vorschau | 🔧 umgesetzt, Nachprüfung ausstehend (DLG-3: Startordner `LastOpenLocation`, sonst Ordner des Dokuments, sonst Dokumente; 14 Filter in der Reihenfolge des Originals (FW-PNG, PSD, GIF, JPEG, WBMP, TIFF, Targa, BMP, Illustrator, EPS, dann die neuen Formate), „Alle lesbaren Dateien“ vorgewählt (im nativen Dialog dafür vorn); Ordner wird gemerkt. Offen: Lage (oben links), Zusatzknöpfe, Vorschau); vorher ❌ PM-5, 12.10., echtes Fenster | [Bild](screenshots/paritaet/pm5_D03_oeffnen.png) |
| D04 | Datei ▸ Speichern unter… | „Speichern unter“ mittig; Dateityp-Liste (FW-PNG voran), „.fw.png anhängen“ ☑, „Als Kopie speichern“, „XMP einschließen“ (grau), „Optionen…“ (grau); Name markiert; Startordner zuletzt benutzt | „Speichern unter“ links oben; 13 Dateitypen (FW-PNG, BMP, GIF, JPEG, PSD, TIFF, WBMP, WebP, AVIF, QOI, ICO, SVG, PDF); Startordner „Dokumente“; **ohne** die drei Kästchen und „Optionen…“ | 🔧 umgesetzt, Nachprüfung ausstehend (DLG-3: Startordner Ordner des Dokuments, sonst `LastSaveLocation`; Dateitypen in der Reihenfolge des Originals („JPEG (*.jpg; *.jpeg; *.jpe)“, „TIFF (*.tif; *.tiff)“), .fw.png wird wie mit dem Kästchen angehängt. Offen: Lage, Kästchen/„Optionen…“); vorher ❌ PM-5, 12.10., echtes Fenster | [Bild](screenshots/paritaet/pm5_D04_speichern_unter.png) |
| D05 | Datei ▸ Als Vorlage speichern… | Speichern-Dialog im Vorlagenordner, nur FW-PNG | „Als Vorlage speichern“ links oben, Startordner „Dokumente“, 13 Dateitypen | 🔧 umgesetzt, Nachprüfung ausstehend (DLG-3: Titel „Speichern unter“ wie im Original, nur „FW-PNG (*.png)“, Startordner Vorlagenordner); vorher ⚠️ PM-5, 12.10., echtes Fenster | [Bild](screenshots/paritaet/pm5_D05_als_vorlage_speichern.png) |
| D06 | Datei ▸ Importieren… | „Importieren“ mittig, 12 Einzelfilter, Vorschau | „Importieren“ links oben, Filter nur „Alle lesbaren Formate“/„Alle Dateien“, ohne Vorschau | 🔧 umgesetzt, Nachprüfung ausstehend (DLG-3: Filterliste wie D03, eine Datei, Startordner `LastImportLocation`, sonst `LastOpenLocation`, sonst Ordner des Dokuments. Offen: Lage, Vorschau); vorher ⚠️ PM-5, 12.10., echtes Fenster | [Bild](screenshots/paritaet/pm5_D06_importieren.png) |
| D07 | Datei ▸ Exportieren… | **Windows-Dateidialog** „Exportieren“ mit Ordnerwahl, Dateiname, „Exportieren:“, „HTML:“ + „Optionen…“, „Segmente:“, „Seiten:“, „Nur ausgewählte Segmente“, „Nur aktueller Status“, „Bereiche ohne Segmente einschließen“; Esc schließt | unverändert eigener Dialog 434 × 257: „Exportieren:“ (Nur Bilder), Dateiname, „Segmente exportieren“, „Optimieren: …“, „Optionen…“, Exportieren…/Abbrechen – **ohne Ordnerwahl, HTML, Seiten, Status, Bereiche**; Esc schließt | ❌ PM-5, 12.10., echtes Fenster | [Bild](screenshots/paritaet/pm5_D07_exportieren.png) |
| D08 | Datei ▸ Bildvorschau… | Esc schließt den Dialog (gemessen von RE-BV; die alte Angabe „Esc schließt nicht“ war falsch) | wird nach `re\17-bildvorschau.md` neu gebaut (P-BV) | ⏳ nicht gemessen (PM-5) | – |
| D09 | Datei ▸ HTML aktualisieren… | Dateidialog „HTML-Datei suchen:“ mittig | „HTML aktualisieren“ links oben, Filter „HTML-Dateien“/„Alle Dateien“ | 🔧 umgesetzt, Nachprüfung ausstehend (DLG-3: Titel „HTML-Datei suchen:“, Filter „HTML-Dateien (*.htm)“/„Alle Dateien (*.*)“, Startordner wie D03. Offen: Lage); vorher ⚠️ PM-5, 12.10., echtes Fenster | [Bild](screenshots/paritaet/pm5_D09_html_aktualisieren.png) |
| D10 | Datei ▸ Datei-Info… | eigenes Fenster (Datei-Info-/XMP-Bedienfeld, 680 × 620), kein Standarddialog; Esc als Nachricht schließt es nicht | kleiner Dialog „Datei-Info“ 374 × 170: Titel, Pfad, Seiten, Symbole, Formatversion, OK; Esc schließt – **ohne XMP-Felder** | ❌ PM-5, 12.10., echtes Fenster | [Bild](screenshots/paritaet/pm5_D10_datei_info.png) |
| D11 | Datei ▸ Exportassistent… | Assistent 422 × 318, Fokus „Weiter“; Seite 1: Erklärtext, „Ein Exportformat auswählen“ / „Aktuelle Formateinstellungen auswerten“ (grau), „Ziel-Exportdateigröße“ (Feld grau); Enter = Weiter → Seite 2 „Ziel wählen“ (Das Web / Bildbearbeitungsanwendung / DTP-Anwendung / Web-Editor); Esc schließt | unverändert einseitiges Formular 474 × 165 (Ziel Web/Bildbearbeitung/Webseite, „Exportgröße festlegen“, Zielgröße 32 K, OK/Abbrechen); Enter = OK legt den Verlaufsschritt „Exportassistent“ an; ~~ohne Startfokus~~ (DLG-1: Startfokus OK), ~~Tab hält an der Titelleiste~~ (DLG-1 `173bca1`: kein Tab-Stopp mehr, Nachprüfung ausstehend) | ❌ PM-5, 12.10., echtes Fenster | [Bild](screenshots/paritaet/pm5_D11_exportassistent.png), [Seite 2](screenshots/paritaet/pm5_D11_exportassistent_seite2.png) |
| D12 | Datei ▸ Batch-Verarbeitung… | Dialog „Batch“ 785 × 606: Dateiauswahl mit eingebetteter Seite „Batch-Verarbeitung“ (Dateiliste, Hinzufügen / Alle hinzufügen / Entfernen, „Aktuell geöffnete Dateien einschließen“), danach Assistent | nativer Dialog „Zu verarbeitende Dateien“ (Mehrfachauswahl, Filter „Bilder“) links oben | ❌ PM-5, 12.10., echtes Fenster | [Bild](screenshots/paritaet/pm5_D12_batch.png) |
| D13 | Datei ▸ HTML-Einrichtung… | 584 × 378, Register Allgemein/Tabelle/Dokumentspezifisch; Fokus OK; Tab OK › Abbrechen › HTML-Stil › Erweiterung › Kommentare …; Esc schließt | unverändert ein Formular 434 × 291 (Erweiterung als Textfeld, XHTML-kompatibel, Bildordner, Segmente an Hilfslinien, nicht angegebene Segmente verwerfen, Demo-HTML); Enter = OK legt den Verlaufsschritt „HTML-Einrichtung“ an; ~~Tab hält an der Titelleiste~~ (DLG-1 `173bca1`: kein Tab-Stopp mehr, Nachprüfung ausstehend) | ❌ PM-5, 12.10., echtes Fenster | [Bild](screenshots/paritaet/pm5_D13_html_einrichtung.png) |
| D14 | Bearbeiten ▸ Objekt einfügen ▸ Neues Symbol… | „In Symbol konvertieren“ 495 × 195: Name „Symbol“ markiert, Typ Grafik/Animation/Schaltfläche, „Hilfslinien für 9-Slice-Skalierung aktivieren“, „In allgemeiner Bibliothek speichern“ (grau); Tab Name › Grafik › OK › Abbrechen › 9-Slice; Esc schließt; OK öffnet die Symbolbearbeitung | „Neues Symbol“ 374 × 113 nur mit Name; Enter = OK fügt sofort eine **leere Instanz „Symbol“ bei 0,0** ins Dokument (Verlaufsschritt „Neues Symbol“), keine Symbolbearbeitung; ~~Tab hält an der Titelleiste~~ (DLG-1 `173bca1`: kein Tab-Stopp mehr, Nachprüfung ausstehend) | ❌ PM-5, 12.10., echtes Fenster | [Bild](screenshots/paritaet/pm5_D14_neues_symbol.png) |
| D15 | Bearbeiten ▸ HTML-Code kopieren… | Assistent „HTML-Code kopieren“ 447 × 317 (HTML-Stil-Liste, Erklärtext, Abbrechen / Zurück / Weiter / Fertig stellen); Enter = Weiter; Esc schließt | unverändert **kein Dialog**: kopiert sofort, Statuszeile „HTML-Code in die Zwischenablage kopiert“ | ❌ PM-5, 12.10., echtes Fenster | [Bild](screenshots/paritaet/pm5_D15_html_code_kopieren.png) |
| D16 | Bearbeiten ▸ Voreinstellungen… | 656 × 442: Kategorieliste (hat den Fokus), „Allgemein“ gruppiert (Dokumentoptionen, Speichern, Benutzeroberfläche, Standardfarben), Hinweis „Änderungen werden erst nach Neustart aktiv“, Hilfe/OK/Abbrechen; Tab Liste › Startseite › Pinselstriche › Rückgängig-Schritte …; Enter = OK, Esc schließt | 634 × 583: Kategorien „Bearbeiten“ (Original „Bearbeitung“), „Photoshop-Import/Öffnen“ (Original „Photoshop-Datei importieren“), „Zusatzmodule“ vor „Starten und Bearbeiten“; Felder untereinander ohne Gruppen, ohne Neustart-Hinweis und ohne „Hilfe“, zusätzlich Entwickleroptionen/Design/Sprache; Liste nicht per Tab erreichbar; Enter/Esc gleich | ⚠️ PM-5, 12.10., echtes Fenster | [Bild](screenshots/paritaet/pm5_D16_voreinstellungen.png) |
| D17 | Bearbeiten ▸ Tastaturkurzbefehle… | 480 × 500: Satzwahl + 4 Symbolknöpfe, „Befehle:“-Liste, **Menübaum**, Beschreibung, Kurzbefehle +/–, „Taste drücken“ + Ändern; Fokus OK; Enter/Esc | 678 × 587: Textknöpfe, flache Liste mit Suche, ohne Beschreibung; Tab zeigt keinen Fokus; Enter/Esc gleich | ⚠️ PM-5, 12.10., echtes Fenster | [Bild](screenshots/paritaet/pm5_D17_tastaturkurzbefehle.png) |
| D18 | Ansicht ▸ Raster ▸ Raster bearbeiten… | 233 × 174: Farbe cyan, Abstände 36/36 mit Popup-Schieber ▾, Fokus erstes Feld markiert; Tab Feld › Feld › OK › Abbrechen; OK mit 20/30 wirkt, **kein Verlaufsschritt** (Strg+Z ändert nichts), zweites Öffnen 20/30; Abbrechen ohne Wirkung | 334 × 217: Farbe cyan ✅; zusätzlich „Raster anzeigen“/„Am Raster ausrichten“, Beschriftung „Abstand ↔:“ statt Symbol, **kein ▾**; OK 20/30 wirkt, aber als Verlaufsschritt (Strg+Z nimmt zurück), OK ohne Änderung legt ebenfalls einen Schritt an; ~~Tab hält an der Titelleiste~~ (DLG-1 `173bca1`: kein Tab-Stopp mehr, Nachprüfung ausstehend). Schieber: im Original ▾ nicht mit der Maus gemessen (kein Vordergrund in der RDP-Sitzung), in Firepixel nicht vorhanden | ⏳ umgesetzt, Nachprüfung ausstehend (DLG-2 `9469497`; vorher ⚠️ PM-5) | [Bild](screenshots/paritaet/pm5_D18_raster_bearbeiten.png) |
| D19 | Modifizieren ▸ Leinwand ▸ Bildgröße… | 410 × 313: Fokus Breite markiert; Höhe und Druckgröße **beim Tippen** nachgeführt (B 250 → H 200, Druck 3.472 × 2.778; B 333 → 266, 4.625 × 3.7); Kettensymbole; Druckgröße 3 Nachkommastellen mit Punkt; Tab über alle Felder/Einheiten › 3 Kästchen › Neuauflösung › OK › Abbrechen; OK 250 → 250 × 200, Strg+Z stellt 500 × 400 her; „Proportionen beibehalten“ nur bei OK gemerkt | 348 × 375: Höhe und Druckgröße erst nach Tab/Fokusverlust; Druckgröße 2 Nachkommastellen mit Komma (6,94), keine Kettensymbole; sonst gleich (250 × 200, Strg+Z, gemerktes Kästchen nur bei OK, Abbrechen ohne Wirkung, Tab-Folge); OK ohne Änderung legt einen Verlaufsschritt an | ⚠️ PM-5, 12.10., echtes Fenster | [Bild](screenshots/paritaet/pm5_D19_bildgroesse.png) |
| D20 | Modifizieren ▸ Leinwand ▸ Leinwandgröße… | 377 × 331: Felder mit ↔/↕-Symbolen, rechts „B: 600“/„H: 400“ live; Anker 3 × 3 mit Pfeilsymbolen (Mitte); „Nur aktuelle Seite“ in der Gruppe; Tab B › H › Einheiten › Anker › OK › Abbrechen › Seite; OK 600 → R1 +50 px und **alle Objekte ausgewählt**; Strg+Z ✅; Anker nicht gemerkt (wieder Mitte) | 332 × 342: Anker als 9 Kästchen ohne Pfeile, gewählter Anker jetzt blau umrandet; Beschriftung „B:/H:“ statt Symbole, keine Live-Anzeige rechts; „Nur aktuelle Seite“ außerhalb der Gruppe; Anker per Tab und Barrierefreiheit nicht erreichbar; OK 600 → R1 +50 px ✅, Auswahl bleibt leer; Strg+Z und Abbrechen ✅ | ⚠️ PM-5, 12.10., echtes Fenster | [Bild](screenshots/paritaet/pm5_D20_leinwandgroesse.png) |
| D21 | Modifizieren ▸ Leinwand ▸ Leinwandfarbe… | 236 × 175, Optionen untereinander, Fokus „Weiß“; Tab Weiß › OK › Abbrechen; Transparent + Enter → #ffffff00, Benutzerdefiniert → #000000, Strg+Z ✅; Wahl nicht gemerkt (Weiß) | 374 × 139, Optionen **nebeneinander**, „Farbe:“ darunter; Tab: ~~Titelleiste › jedes Optionsfeld einzeln~~ (DLG-1 `173bca1`: Startfokus „Weiß“, Optionsgruppe 1 Stopp mit Pfeiltasten, Nachprüfung ausstehend); Wirkung, Strg+Z, Abbrechen und „nicht gemerkt“ gleich | ⏳ umgesetzt, Nachprüfung ausstehend (DLG-2 `9469497`; vorher ⚠️ PM-5) | [Bild](screenshots/paritaet/pm5_D21_leinwandfarbe.png) |
| D22 | Modifizieren ▸ Selektives JPEG ▸ Einstellungen… | 300 × 182: Qualität 90, „Überlagerungsfarbe“, Text-/Schaltflächenqualität **grau, bis „Selektive Qualität aktivieren“ angehakt ist**; Fokus OK; Esc verwirft; OK (☑, 70) gemerkt, Strg+Z nimmt zurück | 394 × 191: alles bedienbar ohne Haken, **ohne Überlagerungsfarbe**; OK/gemerkt/Strg+Z/Esc gleich; ~~Tab hält an der Titelleiste~~ (DLG-1 `173bca1`: kein Tab-Stopp mehr, Nachprüfung ausstehend) | ⏳ umgesetzt, Nachprüfung ausstehend (DLG-2 `9469497`; vorher ❌ PM-5) | [Bild](screenshots/paritaet/pm5_D22_selektives_jpeg.png) |
| D23 | Text ▸ Größe ▸ Andere… | 281 × 110: Feld mit ▾, Wert = Größe des gewählten Texts (24) bzw. ohne Auswahl zuletzt benutzt (48); Fokus Feld markiert; Enter 48 → Text 48 pt, Strg+Z ✅; auch ohne Textauswahl verfügbar | 334 × 113: Wert **immer 12** (auch bei 24-pt-Text), Einheit „Punkt“, **kein ▾**; Wirkung 48 pt und Strg+Z ✅; ~~Tab hält an der Titelleiste~~ (DLG-1 `173bca1`: kein Tab-Stopp mehr, Nachprüfung ausstehend) | ⏳ umgesetzt, Nachprüfung ausstehend (DLG-2 `9469497`; vorher ❌ PM-5) | [Bild](screenshots/paritaet/pm5_D23_textgroesse.png) |
| D24 | Text ▸ Rechtschreibprüfung einrichten… | 339 × 338: Wörterbuchliste mit Kästchen, Pfad zum persönlichen Wörterbuch + Ordnerknopf, „Persönliches Wörterbuch bearbeiten“, 4 Optionen; Fokus OK; Esc schließt | unverändert 454 × 311: Sprachkürzel „de_DE“ als Text, Ordnerfeld, 2 Optionen, Wörterbuch als Textfeld | ❌ PM-5, 12.10., echtes Fenster | [Bild](screenshots/paritaet/pm5_D24_rechtschreibung_einrichten.png) |
| D25 | Befehle ▸ Gespeicherte Befehle verwalten… | ohne gespeicherte Befehle nur **Meldung** (Info-Symbol, Titel = Programmname): „Befehle werden dem Verlauf-Bedienfeld entnommen. Zurzeit sind dort keine gespeicherten Befehle vorhanden.“; Enter/Esc schließen | Verwaltungsliste 418 × 329 mit den zwei mitgelieferten Befehle-Menü-Skripten („Größe ausgewählter Objekte ändern“, „Warndialoge zurücksetzen“) als „gespeicherte Befehle“; Umbenennen/Löschen grau, Fertig | ❌ PM-5, 12.10., echtes Fenster | [Bild](screenshots/paritaet/pm5_D25_gespeicherte_befehle.png) |
| D26 | Modifizieren ▸ Transformieren ▸ Numerisch transformieren… (1 Rechteck) | Kombi Skalieren/Größe ändern/Drehen; Felder ohne Beschriftung mit ↔/↕-Symbol, Kette, „B: 100 / H: 80“ (Objektgröße) rechts; Kästchen an | Felder „Breite“/„Höhe“, **keine Anzeige der Objektgröße B/H**, keine Kette | ⚠️ | [Bild](screenshots/paritaet/b_D26_numerisch_transformieren.png) |
| D27 | Modifizieren ▸ Symbol ▸ In Symbol konvertieren… | Name, Typ (Grafik/Animation/Schaltfläche), Optionen 9-Slice, „In allgemeiner Bibliothek speichern“ | gleiche Felder („Für allgemeine Bibliothek speichern“) | ✅ | [Bild](screenshots/paritaet/b_D27_in_symbol_konvertieren.png) |
| D28 | Modifizieren ▸ Animation ▸ Auswahl animieren… | „Animieren“: Status 5, **Verschieben 72, Richtung 0**, Skalieren auf 100, Deckkraft 100 bis 100, Drehen 0, Nach rechts/Nach links; Zahlenfelder mit Popup-Schieber ▾ | Bewegung 72, **Richtung 45**, Schieber als feste Leisten unter jedem Feld, „Im/Entgegen dem Uhrzeigersinn“ | 🔧 behoben, Nachmessung im echten Fenster offen (FV-4: Popup-Schieber ▾ senkrecht wie im Inspektor, Richtung 0) | [Bild](screenshots/paritaet/b_D28_auswahl_animieren.png) |
| D29 | Modifizieren ▸ Pfad ändern ▸ Vereinfachen… | „Menge:“ **10** mit Popup-Schieber ▾ | „Betrag:“ **1** ohne Schieber | 🔧 behoben, Nachmessung im echten Fenster offen (FV-4: Vorgabe 10 aus `SimplifyAmount`, gemerkt, Popup-Schieber ▾ senkrecht; Beschriftung „Menge“ offen) | [Bild](screenshots/paritaet/b_D29_vereinfachen.png) |
| D30 | Modifizieren ▸ Pfad ändern ▸ Strich erweitern… | Breite 10 mit ▾; Ecken als **3 Symbolknöpfe** (drittes gewählt); „Winkelgrenze“ 10.00; „Endkappen“ als 3 Symbolknöpfe (erstes gewählt) | Optionsfelder mit Text (Eckig gewählt), „Gehrungsgrenze“ 10, „Enden“; Breite ohne ▾ | 🔧 behoben, Nachmessung im echten Fenster offen (FV-4: Breite 10/Ecken/Endkappen/Winkelgrenze aus `Expand*` gemerkt, Symbolknöpfe, Popup ▾ senkrecht, 2 Nachkommastellen; Beschriftungen offen) | [Bild](screenshots/paritaet/b_D30_strich_erweitern.png) |
| D31 | Modifizieren ▸ Pfad ändern ▸ Pfad einlassen… | Richtung Innen/Außen **untereinander**, Breite **10** mit ▾, Ecken als Symbolknöpfe, Winkelgrenze 10.00 | Breite **1**, Ecken als Text-Optionsfelder, Gehrungsgrenze 10 | 🔧 behoben, Nachmessung im echten Fenster offen (FV-4: Breite 10/Richtung/Ecken/Winkelgrenze aus `Inset*` gemerkt, Symbolknöpfe, Popup ▾ senkrecht; Richtung untereinander offen) | [Bild](screenshots/paritaet/b_D31_pfad_einlassen.png) |
| D32 | Modifizieren ▸ Pfad in Auswahlrahmen konvertieren… | Dialog „Pfad in Auswahlrahmen konvertieren“ (Kante Hart/Geglättet/Weiche Kante + Breite) | **kein Dialog** | ❌ | – |
| D33 | Modifizieren ▸ Symbol ▸ Instanzen tweenen… (ohne 2 Instanzen) | Meldung „Wählen Sie zwei oder mehr Instanzen desselben Bibliothekssymbols aus, bevor Sie den Tween-Befehl verwenden.“ mit Kästchen „Nicht mehr anzeigen“ (Menüpunkt deshalb immer aktiv) | 🔧 (FV-5) Menüpunkt aktiv; ohne zwei Instanzen desselben Symbols dieselbe Meldung mit „Nicht mehr anzeigen“ statt des Dialogs (Test `fp-ui\tests\dialogs_warnung.rs`); vorher: grau, Dialog ohne Prüfung | 🔧 | [Bild](screenshots/paritaet/b_D33_instanzen_tweenen.png) |
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

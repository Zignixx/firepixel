# Paritätsabgleich B: Menüs, Dialoge, Bedienfelder, Fenster (Original ↔ Firepixel)

## Übergabe (Stand 12.10.2026 ≈ 04:30, Agent UX-B, auf Nutzerwunsch angehalten)

### Stand und Zahlen

| Bereich | geprüft | ✅ | ⚠️ | ❌ | ℹ️/❔ | Abschnitt |
|---|---:|---:|---:|---:|---:|---|
| Hauptmenüs (jeder Eintrag in 9 Lagen: Kürzel, aktiv/grau, Häkchen, Untermenü, Beschriftung) | 350 Zeilen | 276 | 20 | 25 | 29 | §1 (neu gemessen von PM-7, 12.10.; vorher 311 / 5 / 5 / 29 in 5 Lagen) |
| Dialoge Feld für Feld | 50 | 3 | 16 | 15 | 10 ⏳ + 6 🔧 | §2 (D01–D25 von PM-5, seitdem DLG-2/DLG-3: 0 ✅ / 5 ⚠️ / 9 ❌ / 5 ⏳ / 6 🔧; D26–D50 neu gemessen von PM-6, 12.10. abends: 3 ✅ / 16 ⚠️ / 6 ❌) |
| Bedienfeldmenüs (≡) | 10 | 1 | 4 | 4 | 1 | §3 (neu gemessen von PM-7, 12.10.; Firepixel unverändert, dazu Eigenschaften) |

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
| I18N-SHELL / HG / AU | Befehle-Menü im frischen Profil leer (`install_builtin_commands`) | nachgeprüft (PM-7, 12.10.): gefüllt; es fehlen 3 Einträge und 2 Untereinträge, Namen weichen ab (§1, I18N-MENUE) |
| I18N-SHELL / HG | native Datei-Dialoge bei (0,0) ohne Besitzer, Startordner, Batch-Dialog | offen |
| I18N-DIALOGE / IBD | D01–D25, D26–D50, Schieberabbildung, Vorgaben/gemerkte Werte aus den Voreinstellungen | Schieberabbildung, senkrechte Popup-Schieber und Vorgaben/gemerkte Werte 🔧 behoben (FV-4), Nachmessung im echten Fenster offen; übrige D-Zeilen offen |
| I18N-BEDIENFELDER / IBD | Zustände in den Bedienfeldmenüs (Verlauf, Stile, Status, Seiten, Ebenen, Optimieren) | offen, von PM-7 am 12.10. unverändert nachgemessen (§3) |
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
## 1. Hauptmenüs (jeder Eintrag in 9 Lagen, 5 Gesichtspunkte)

**Neu gemessen von PM-7, 12.10. (≈ 18:45–19:35), in beiden Programmen.** Gesichtspunkte je Eintrag: Kürzel, aktiv/grau, Häkchen, Untermenü ja/nein, Beschriftung.
Lagen: S1 Dokument ohne Auswahl · S2 ein Rechteck · S3 zwei Rechtecke · S4 Text · S5 Bitmap · S6 Pixelauswahl (Auswahlrahmen in der Bitmap) · S7 Gruppe · S8 Symbolinstanz · S9 Segment. Die Zeichenfolge „●○…“ gibt aktiv (●) bzw. grau (○) je Lage in dieser Reihenfolge an, „Häkchen ✓·…“ ebenso.
- **Original:** Kopie eines Testdokuments (R1, R2, Text T1 „Hallo“, Bitmap B1; dazu per JSF die Gruppe G1, die Symbolinstanz SY und das Segment SL), per `fw.openDocument` geöffnet und ohne Speichern geschlossen. Die Menüs wurden per Nachricht an die Menüleiste geöffnet, jedes Untermenü einzeln frisch geöffnet und per `MN_GETHMENU`/`GetMenuState` gelesen. S6 ist mit echter Maus gezogen (Auswahlrahmen-Werkzeug, Vordergrundprüfung); per JSF (`convertPathToMarquee`) entsteht kein wirksamer Auswahlrahmen.
- **Firepixel:** sauberer Worktree von HEAD `6882dac`, Sonde nach dem Muster von `crates\fp-ui\tests\ux_b_paritaet.rs` mit denselben 9 Lagen (nur für die Messung, nicht eingecheckt). Befehle-Menü im frischen Profil: eigene Instanz mit eigenem APPDATA/LOCALAPPDATA und `--mcp`, echtes Fenster, Brücke.
- **Nicht gewertet:** Laufzeitlisten (zuletzt geöffnete Dateien, Dokumentliste im Fenster-Menü); aktiv/grau von Rückgängig/Wiederholen, den Einfüge-Befehlen und „Wiederherstellen“ (Verlauf, Zwischenablage und gespeicherte Datei waren verschieden); Häkchen im Fenster-Menü, bei QuickInfos und Anti-Alias-Füllung; Häkchen unter Text ▸ Größe/Stil/Ausrichten ohne ausgewählten Text (das sind die Vorgaben des Textwerkzeugs: Original 48, Firepixel 12). Die Beschriftungen „Vorschau im Primär-/Sekundärbrowser“, die das Original vor dem ersten Öffnen des Untermenüs kurz meldet, sind nicht gezählt; frisch geöffnet zeigt es wie Firepixel „Vorschau in iexplore.exe“ und „Vorschau in“ (grau).
- **Gegenüber 04:30:**
  - Befehle-Menü im frischen Profil jetzt gefüllt, aber mit Lücken und eigenen Namen (Zeilen „Befehle >“).
  - „Mit darunter liegender auf eine Ebene reduzieren“ ist in allen 9 Lagen gleich: aktiv, wenn direkt darunter eine Bitmap liegt, im Testdokument also bei der Gruppe. Der alte ❌ bei Text kam von der anderen Stapelfolge im damaligen Testdokument.
  - Pfeiltasten-Kürzel („Nach-oben-Taste“), „Löschen-Taste“ und „Strg+ü“ sind jetzt gleich.
  - Neu sichtbar durch die zusätzlichen Lagen:
    - Bei Pixelauswahl sperrt das Original die Objektbefehle (12 ❌).
    - Bei Segment und Gruppe weichen 7 Einträge ab.
  - Neu gewertet: Untermenü-Köpfe bleiben im Original immer aktiv, auch wenn alle Einträge grau sind (11 ⚠️).
  - Offen geblieben: Schrift-Menü und „Bridge durchsuchen“.

Zählung (PM-7, 12.10.; MN-2 nachgetragen): ✅ 276, ⚠️ 9, ❌ 4, ℹ️ 29, umgesetzt und Nachprüfung ausstehend 32 (MN-2: 21 ❌ + 11 ⚠️) (350 Zeilen)

| Element | Original | Firepixel | | Bild |
|---|---|---|---|---|
| Text > Schrift > (Liste) | 3 zuletzt benutzte Schriften oben (Arial, Myriad Pro, Times New Roman), Trennlinie, dann alle Familien **flach**, Familien mit mehreren Schnitten als Untermenü; aktuelle Schrift ✓; Einträge **auch ohne Text aktiv** (setzen die Vorgabe des Textwerkzeugs) | Familien weiter nach Anfangsbuchstaben gruppiert (A ▸, B ▸ … Y ▸), Schnitte als Untermenü; im frischen Profil keine zuletzt benutzten Schriften; ohne ausgewählten Text grau (S1–S3, S5–S9), kein ✓ für die aktuelle Schrift (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: Familien flach (Bildlauf ab 40 Einträgen), zuletzt benutzte oben (`RecentFontList`), ✓ an der Schrift des ausgewählten Texts bzw. der Vorgabe des Textwerkzeugs, auch ohne Text aktiv (setzt `Text_Font`); bei Familien mit Schnitten trägt der Schnitt das ✓); vorher ❌ PM-7 | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Befehle > (Liste im frischen Profil) | 12 Einträge unter „Skript ausführen…“: Auswahl ▸, Batch-Befehle ▸, Bildschirmfoto, Demo - aktuelles Dokument, Dokument ▸, Größe ausgewählter Objekte ändern, jQuery Mobile-Thema ▸, Kreativ ▸, Symbolskript erstellen, Text ▸, Warnmeldungen zurücksetzen, Web ▸ | jetzt vorhanden (frisches Profil, eigene Instanz, echtes Fenster, Brücke): KI-Modelle verwalten…, Auswahl ▸, Batch-Befehle ▸, Dokument ▸, Größe ausgewählter Objekte ändern, Kreativ ▸, Text ▸, **Warndialoge zurücksetzen**, Web ▸, dazu am Ende **Warnmeldungen zurücksetzen** und KI-Steuerkanal ✓ – „Warnmeldungen zurücksetzen“ steht damit doppelt (einmal unter anderem Namen); es fehlen Bildschirmfoto, Demo - aktuelles Dokument, Symbolskript erstellen (PM-7, 12.10.) | ❌ | [Bild](screenshots/paritaet/pm7_M07_befehle_firepixel.png) |
| Befehle > Auswahl ▸ | Früheres Vektorrendern verwenden, Verbessertes Vektorrendern verwenden | Bisheriges Vektor-Rendering verwenden, Verbessertes Vektor-Rendering verwenden – Beschriftung anders (PM-7, 12.10.) | ⚠️ | [Bild](screenshots/paritaet/pm7_M07_befehle_firepixel.png) |
| Befehle > Batch-Befehle ▸ | 11: 180°-Drehung, 270°-Drehung, 90°-Drehung, Alles auswählen, Auswahl scharfzeichnen, Auswahl um 180/270/90 Grad drehen, Auswahl weichzeichnen, Auswahlfarbe umkehren, Auto-Tonwertauswahl | 11, gleiche Befehle, andere Namen und damit andere Reihenfolge: Dokument um 180°/270°/90° drehen, Auswahl um 180°/270°/90° drehen, Farbe der Auswahl umkehren, Auswahl automatisch tonwertkorrigieren (PM-7, 12.10.) | ⚠️ | – |
| Befehle > Dokument ▸ | Andere Ebenen ausblenden, Andere Ebenen sperren, Status umkehren, Verteilung auf Ebenen | gleich bis auf „Auf Ebenen verteilen“ (statt „Verteilung auf Ebenen“, steht deshalb vor „Status umkehren“) (PM-7, 12.10.) | ⚠️ | – |
| Befehle > Kreativ ▸ | 7: Autom. Vektormaske, Bilderrahmen hinzufügen, Drehen und Ausblenden, Graustufenkonvertierung, Pfeilspitzen hinzufügen, Schatten hinzufügen, Sepia-Konvertierung | 6: **Autom. Vektormaske fehlt**; Verdrehen und ausblenden, In Graustufen umwandeln, In Sepiaton umwandeln (andere Namen) (PM-7, 12.10.) | ❌ | – |
| Befehle > Text ▸ | 9: Als Großbuchstaben, Als Kleinbuchstaben, Als Pfade wiedergeben, Als Pixel wiedergeben, Breite festlegen, Erster Buchstabe Großbuchstabe, Feste Breite ein/aus, Lorem ipsum, Verfestigen | 9, andere Namen: Großbuchstaben, Kleinbuchstaben, In Pfade umwandeln, In Pixel umwandeln, Erster Buchstabe groß, Feste Breite umschalten, Blindtext (PM-7, 12.10.) | ⚠️ | – |
| Befehle > Web ▸ | ALT-Text festlegen, Gemeinsame Palette, Leeren ALT-Text auswählen | **Gemeinsame Palette fehlt** (PM-7, 12.10.) | ❌ | – |
| Befehle > jQuery Mobile-Thema ▸ | Neues Thema, Symbolplatzhalter einfügen, Thema exportieren, Themavorschau im Browser | – (Erweiterung eines Drittanbieters, nicht nachgebaut) | ℹ️ | – |
| Datei > Neu | Kürzel Strg+N; ●●●●●●●●● | Kürzel Strg+N; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Neu von Vorlage | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Öffnen | Kürzel Strg+O; ●●●●●●●●● | Kürzel Strg+O; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Bridge durchsuchen | Kürzel Strg+Alt+O; ○○○○○○○○○ | Kürzel Strg+Alt+O; ●●●●●●●●● – aktiv weicht ab in S1,S2,S3,S4,S5,S6,S7,S8,S9 (PM-7, 12.10.) | ❌ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Zuletzt geöffnete Dateien | Kürzel –; ●●●●●●●●● | Kürzel –; ○○○○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Scannen | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Scannen > Twain importieren | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Scannen > Twain auswählen | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Schließen | Kürzel Strg+W; ●●●●●●●●● | Kürzel Strg+W; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Alle schließen | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Speichern | Kürzel Strg+S; ●●●●●●●●● | Kürzel Strg+S; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Speichern unter | Kürzel Strg+Umschalt+S; ●●●●●●●●● | Kürzel Strg+Umschalt+S; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Alles speichern | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Als Vorlage speichern | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Wiederherstellen | Kürzel –; ●●●●●●●●● | Kürzel –; ○○○○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Importieren | Kürzel Strg+R; ●●●●●●●●● | Kürzel Strg+R; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Exportieren | Kürzel Strg+Umschalt+R; ●●●●●●●●● | Kürzel Strg+Umschalt+R; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Bildvorschau | Kürzel Strg+Umschalt+X; ●●●●●●●●● | Kürzel Strg+Umschalt+X; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Als E-Mail senden | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Als E-Mail senden > PNG-Datei für Firepixel | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Als E-Mail senden > Komprimierte JPG-Datei | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Als E-Mail senden > Exporteinstellungen verwenden | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > HTML aktualisieren | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Datei-Info | Kürzel Strg+Alt+Umschalt+F; ●●●●●●●●● | Kürzel Strg+Alt+Umschalt+F; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Exportassistent | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Batch-Verarbeitung | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Vorschau im Browser | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Vorschau im Browser > Primärbrowser festlegen | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Vorschau im Browser > Sekundärbrowser festlegen | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Seite einrichten | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Drucken | Kürzel Strg+P; ●●●●●●●●● | Kürzel Strg+P; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > HTML-Einrichtung | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Beenden | Kürzel Strg+Q; ●●●●●●●●● | Kürzel Strg+Q; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Vorschau im Browser > Vorschau in Standardbrowser | Kürzel F12; ●●●●●●●●● | Kürzel F12; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Vorschau im Browser > Vorschau in | Kürzel Umschalt+F12; ○○○○○○○○○ | Kürzel Umschalt+F12; ○○○○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Datei > Vorschau im Browser > Vorschau aller Seiten in Standardbrowser anzeigen | Kürzel Strg+F12; ●●●●●●●●● | Kürzel Strg+F12; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M01_menue_datei.png) |
| Bearbeiten > Rückgängig | Kürzel Strg+Z; ●●●●●●●●● | Kürzel Strg+Z; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Wiederholen | Kürzel Strg+Y; ●●●●●●●●● | Kürzel Strg+Y; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Objekt einfügen | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Objekt einfügen > Neue Schaltfläche | Kürzel Strg+Umschalt+F8; ●●●●●●●●● | Kürzel Strg+Umschalt+F8; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Objekt einfügen > Neues Symbol | Kürzel Strg+F8; ●●●●●●●●● | Kürzel Strg+F8; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Objekt einfügen > Hotspot | Kürzel Strg+Umschalt+U; ●●●●●●●●● | Kürzel Strg+Umschalt+U; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Objekt einfügen > Rechteckiges Segment | Kürzel Alt+Umschalt+U; ●●●●●●●●● | Kürzel Alt+Umschalt+U; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Objekt einfügen > Polygonsegment | Kürzel Alt+Umschalt+P; ●●●●●●●●● | Kürzel Alt+Umschalt+P; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Objekt einfügen > Leere Bitmap | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Objekt einfügen > Bitmap durch Kopieren | Kürzel –; ○○○○○●○○○ | Kürzel –; ○○○○○●○○○ | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Objekt einfügen > Bitmap durch Ausschneiden | Kürzel –; ○○○○○●○○○ | Kürzel –; ○○○○○●○○○ | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Objekt einfügen > Ebene | Kürzel Umschalt+L; ●●●●●●●●● | Kürzel Umschalt+L; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Objekt einfügen > Status | Kürzel Umschalt+F; ●●●●●●●●● | Kürzel Umschalt+F; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Objekt einfügen > Seite | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Suchen und ersetzen | Kürzel Strg+F; ●●●●●●●●● | Kürzel Strg+F; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Ausschneiden | Kürzel Strg+X; ○●●●●●●●● | Kürzel Strg+X; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Kopieren | Kürzel Strg+C; ○●●●●●●●● | Kürzel Strg+C; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Als Vektoren kopieren | Kürzel –; ○●●●●○●●○ | Kürzel –; ○●●●●●●●● – aktiv weicht ab in S6,S9 (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: grau bei Pixelauswahl und bei nur Segmenten); vorher ❌ PM-7 | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > HTML-Code kopieren | Kürzel Strg+Alt+C; ●●●●●●●●● | Kürzel Strg+Alt+C; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Einfügen | Kürzel Strg+V; ●●●●●●●●● | Kürzel Strg+V; ○○○○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Löschen | Kürzel Löschen-Taste; ○●●●●●●●● | Kürzel Löschen-Taste; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Als Maske einfügen | Kürzel Strg+Alt+V; ○●○●●○●●○ | Kürzel Strg+Alt+V; ○○○○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Innen einfügen | Kürzel Strg+Umschalt+V; ○●○●●○●●○ | Kürzel Strg+Umschalt+V; ○○○○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Attribute einfügen | Kürzel Strg+Alt+Umschalt+V; ●●●●●○●●● | Kürzel Strg+Alt+Umschalt+V; ○○○○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Duplizieren | Kürzel Strg+Alt+D; ○●●●●○●●● | Kürzel Strg+Alt+D; ○●●●●●●●● – aktiv weicht ab in S6 (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: grau bei Pixelauswahl); vorher ❌ PM-7 | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Klonen | Kürzel Strg+Umschalt+D; ○●●●●○●●● | Kürzel Strg+Umschalt+D; ○●●●●●●●● – aktiv weicht ab in S6 (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: grau bei Pixelauswahl); vorher ❌ PM-7 | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Ausgewählte Bitmap zuschneiden | Kürzel –; ○○○○●●○○○ | Kürzel –; ○○○○●●○○○ | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Dokument zuschneiden | Kürzel –; ○●●●●●●●● | Kürzel –; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Voreinstellungen | Kürzel Strg+U; ●●●●●●●●● | Kürzel Strg+U; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Bearbeiten > Tastaturkurzbefehle | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M02_menue_bearbeiten.png) |
| Ansicht > Größer zoomen | Kürzel Strg++; ●●●●●●●●● | Kürzel Strg++; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Kleiner zoomen | Kürzel Strg+-; ●●●●●●●●● | Kürzel Strg+-; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Vergrößerung | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Vergrößerung > 6% | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Vergrößerung > 12% | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Vergrößerung > 25% | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Vergrößerung > 50% | Kürzel Strg+5; ●●●●●●●●● | Kürzel Strg+5; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Vergrößerung > 66% | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Vergrößerung > 100% | Kürzel Strg+1; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ | Kürzel Strg+1; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Vergrößerung > 150% | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Vergrößerung > 200% | Kürzel Strg+2; ●●●●●●●●● | Kürzel Strg+2; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Vergrößerung > 300% | Kürzel Strg+3; ●●●●●●●●● | Kürzel Strg+3; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Vergrößerung > 400% | Kürzel Strg+4; ●●●●●●●●● | Kürzel Strg+4; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Vergrößerung > 800% | Kürzel Strg+8; ●●●●●●●●● | Kürzel Strg+8; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Vergrößerung > 1600% | Kürzel Strg+6; ●●●●●●●●● | Kürzel Strg+6; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Vergrößerung > 3200% | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Vergrößerung > 6400% | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Auswahl anpassen | Kürzel Strg+Alt+0; ○●●●●●●●● | Kürzel Strg+Alt+0; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Alles anpassen | Kürzel Strg+0; ●●●●●●●●● | Kürzel Strg+0; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Gamma 1.8 | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Auswahl ausblenden | Kürzel Strg+L; ○●●●●●●●● | Kürzel Strg+L; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > QuickInfos | Kürzel Strg+AKUT; ●●●●●●●●● | Kürzel Strg+´; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ – Kürzel Strg+AKUT ≠ Strg+´ (PM-7, 12.10.) | ⚠️ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Alles einblenden | Kürzel Strg+Umschalt+L; ●●●●●●●●● | Kürzel Strg+Umschalt+L; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Lineale | Kürzel Strg+Alt+R; ●●●●●●●●● | Kürzel Strg+Alt+R; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Raster | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Raster > Raster anzeigen | Kürzel Strg+Alt+G; ●●●●●●●●● | Kürzel Strg+Alt+G; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Raster > Am Raster ausrichten | Kürzel Strg+Alt+Umschalt+G; ●●●●●●●●● | Kürzel Strg+Alt+Umschalt+G; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Raster > Raster bearbeiten | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Hilfslinien | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Hilfslinien > Hilfslinien anzeigen | Kürzel Strg+ü; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ | Kürzel Strg+ü; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Hilfslinien > Hilfslinien sperren | Kürzel Strg+Alt+ü; ●●●●●●●●● | Kürzel Strg+Alt+ü; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Hilfslinien > An Hilfslinien ausrichten | Kürzel Strg+Umschalt+ü; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ | Kürzel Strg+Umschalt+ü; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Hilfslinien > Hilfslinien entfernen | Kürzel –; ○○○○○○○○○ | Kürzel –; ○○○○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Intelligente Hilfslinien | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Intelligente Hilfslinien > Intelligente Hilfslinien anzeigen | Kürzel Strg+ZIRKUMFLEX; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ | Kürzel Strg+^; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ – Kürzel Strg+ZIRKUMFLEX ≠ Strg+^ (PM-7, 12.10.) | ⚠️ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Intelligente Hilfslinien > An intelligenten Hilfslinien ausrichten | Kürzel Strg+Umschalt+ZIRKUMFLEX; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ | Kürzel Strg+Umschalt+^; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ – Kürzel Strg+Umschalt+ZIRKUMFLEX ≠ Strg+Umschalt+^ (PM-7, 12.10.) | ⚠️ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Hilfslinien für 9-Slice-Skalierung | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Hilfslinien für 9-Slice-Skalierung > Hilfslinien sperren | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Segment-Hilfslinien | Kürzel Strg+Alt+Umschalt+ü; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ | Kürzel Strg+Alt+Umschalt+ü; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Segmentüberlagerung | Kürzel –; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ | Kürzel –; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Ränder | Kürzel Strg+H; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ | Kürzel Strg+H; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ | ✅ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Auswählen > Alles auswählen | Kürzel Strg+A; ●●●●●●●●● | Kürzel Strg+A; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Auswählen > Auswahl aufheben | Kürzel Strg+D; ○●●●●●●●● | Kürzel Strg+D; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Auswählen > Gesamtauswahl treffen | Kürzel Strg+Nach-rechts-Taste; ○●●●●○●●● | Kürzel Strg+Nach-rechts-Taste; ○●●●●●●●● – aktiv weicht ab in S6 (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: grau bei Pixelauswahl); vorher ❌ PM-7 | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Auswählen > Teilauswahl treffen | Kürzel Strg+Nach-links-Taste; ○○○○○○●○○ | Kürzel Strg+Nach-links-Taste; ○○○○○○●○○ | ✅ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Auswählen > Ähnliches auswählen | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Auswählen > Auswahl umkehren | Kürzel Strg+Umschalt+I; ●●●●●●●●● | Kürzel Strg+Umschalt+I; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Auswählen > Weiche Kante | Kürzel Strg+Alt+Umschalt+D; ●●●●●●●●● | Kürzel Strg+Alt+Umschalt+D; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Auswählen > Auswahlrahmen erweitern | Kürzel –; ○○○○○●○○○ | Kürzel –; ○○○○○●○○○ | ✅ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Auswählen > Auswahlrahmen verkleinern | Kürzel –; ○○○○○●○○○ | Kürzel –; ○○○○○●○○○ | ✅ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Auswählen > Auswahlrahmen begrenzen | Kürzel –; ○○○○○●○○○ | Kürzel –; ○○○○○●○○○ | ✅ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Auswählen > Auswahlrahmen glätten | Kürzel –; ○○○○○●○○○ | Kürzel –; ○○○○○●○○○ | ✅ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Auswählen > Auswahlrahmen in Pfad konvertieren | Kürzel –; ○○○○○●○○○ | Kürzel –; ○○○○○●○○○ | ✅ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Auswählen > Bitmap-Auswahl speichern | Kürzel –; ○○○○○●○○○ | Kürzel –; ○○○○○●○○○ | ✅ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Auswählen > Bitmap-Auswahl wiederherstellen | Kürzel –; ○○○○○●○○○ | Kürzel –; ○○○○○○○○○ – aktiv weicht ab in S6 – nicht gewertet: im Original in S6/S9 aktiv, obwohl keine Auswahl gespeichert wurde (Ursache offen, vermutlich Zustand aus einem früheren Lauf im selben Dokument); FV-13 hatte die Freigabe mit gespeicherter Auswahl schon geprüft (PM-7, 12.10.) | ℹ️ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Auswählen > Bitmap-Auswahl löschen | Kürzel –; ○○○○○●○○● | Kürzel –; ○○○○○○○○○ – aktiv weicht ab in S6,S9 – nicht gewertet: im Original in S6/S9 aktiv, obwohl keine Auswahl gespeichert wurde (Ursache offen, vermutlich Zustand aus einem früheren Lauf im selben Dokument); FV-13 hatte die Freigabe mit gespeicherter Auswahl schon geprüft (PM-7, 12.10.) | ℹ️ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Modifizieren > Leinwand | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Leinwand > Bildgröße | Kürzel Strg+J; ●●●●●●●●● | Kürzel Strg+J; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Leinwand > Leinwandgröße | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Leinwand > Leinwandfarbe | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Leinwand > Leinwand zuschneiden | Kürzel Strg+Alt+T; ●●●●●●●●● | Kürzel Strg+Alt+T; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Leinwand > Leinwand anpassen | Kürzel Strg+Alt+F; ●●●●●●●●● | Kürzel Strg+Alt+F; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Leinwand > 180° drehen | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Leinwand > 90° im UZS drehen | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Leinwand > 90° gegen UZS drehen | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Animation | Kürzel –; ●●●●●●●●● | Kürzel –; ○●●●●●●●● – Untermenü-Kopf grau in S1 (Original: Kopf aktiv, alle Einträge grau) (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: Untermenü-Kopf immer aktiv); vorher ⚠️ PM-7 | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Animation > Auswahl animieren | Kürzel Alt+Umschalt+F8; ○●●●●○●●● | Kürzel Alt+Umschalt+F8; ○●●●●●●●● – aktiv weicht ab in S6 (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: grau bei Pixelauswahl); vorher ❌ PM-7 | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Animation > Einstellungen | Kürzel –; ○○○○○○○○○ | Kürzel –; ○○○○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Animation > Animation entfernen | Kürzel –; ○○○○○○○○○ | Kürzel –; ○○○○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Symbol | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Symbol > In Symbol konvertieren | Kürzel F8; ○●●●●○●●○ | Kürzel F8; ○●●●●●●●● – aktiv weicht ab in S6,S9 (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: grau bei Pixelauswahl und bei nur Segmenten); vorher ❌ PM-7 | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Symbol > Symbol bearbeiten | Kürzel –; ○○○○○○○●○ | Kürzel –; ○○○○○○○●○ | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Symbol > An Position bearbeiten | Kürzel –; ○○○○○○○●○ | Kürzel –; ○○○○○○○●○ | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Symbol > Instanzen tweenen | Kürzel Strg+Alt+Umschalt+T; ●●●●●●●●● | Kürzel Strg+Alt+Umschalt+T; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Symbol > Teilen | Kürzel Strg+Umschalt+B; ●●●●●●●●● | Kürzel Strg+Umschalt+B; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Symbol > Symbol austauschen | Kürzel –; ○○○○○○○●○ | Kürzel –; ○○○○○○○●○ | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Popupmenü | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Popupmenü > Popupmenü hinzufügen | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Popupmenü > Popupmenü bearbeiten | Kürzel –; ○○○○○○○○○ | Kürzel –; ○○○○○○○○● – aktiv weicht ab in S9 (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: nur mit Popupmenü am Segment aktiv); vorher ❌ PM-7 | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Popupmenü > Popupmenü löschen | Kürzel –; ○○○○○○○○○ | Kürzel –; ○○○○○○○○● – aktiv weicht ab in S9 (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: nur mit Popupmenü am Segment aktiv); vorher ❌ PM-7 | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Maske | Kürzel –; ●●●●●●●●● | Kürzel –; ○●●●●●●●● – Untermenü-Kopf grau in S1 (Original: Kopf aktiv, alle Einträge grau) (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: Untermenü-Kopf immer aktiv); vorher ⚠️ PM-7 | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Maske > Alles einblenden | Kürzel –; ○●○●●●●●○ | Kürzel –; ○●○●●●●●● – aktiv weicht ab in S9 (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: grau bei nur Segmenten); vorher ❌ PM-7 | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Maske > Alles ausblenden | Kürzel –; ○●○●●●●●○ | Kürzel –; ○●○●●●●●● – aktiv weicht ab in S9 (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: grau bei nur Segmenten); vorher ❌ PM-7 | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Maske > Als Maske einfügen | Kürzel Strg+Alt+V; ○●○●●○●●○ | Kürzel Strg+Alt+V; ○○○○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Maske > Als Maske gruppieren | Kürzel –; ○○●○○○○○○ | Kürzel –; ○○●○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Maske > Auswahl einblenden | Kürzel –; ○○○○○●○○○ | Kürzel –; ○○○○○●○○○ | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Maske > Auswahl ausblenden | Kürzel –; ○○○○○●○○○ | Kürzel –; ○○○○○●○○○ | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Maske > Maske deaktivieren | Kürzel –; ○○○○○○○○○ | Kürzel Strg+Alt+H; ○○○○○○○○○ – Kürzel – ≠ Strg+Alt+H (PM-7, 12.10.) | ⚠️ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Maske > Maske löschen | Kürzel –; ○○○○○○○○○ | Kürzel –; ○○○○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Selektives JPEG | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Selektives JPEG > Auswahl als JPEG-Maske speichern | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Selektives JPEG > JPEG-Maske als Auswahl wiederherstellen | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Selektives JPEG > Einstellungen | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Selektives JPEG > JPEG-Maske entfernen | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Auswahl sperren | Kürzel Strg+Alt+L; ○●●●●●●●● | Kürzel Strg+Alt+L; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Auswahl auf Hintergrund reduzieren | Kürzel Strg+Alt+Umschalt+Z; ○●●●●○●●○ | Kürzel Strg+Alt+Umschalt+Z; ○●●●●●●●● – aktiv weicht ab in S6,S9 (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: grau bei Pixelauswahl und bei nur Segmenten); vorher ❌ PM-7 | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Mit darunter liegender auf eine Ebene reduzieren | Kürzel Strg+E; ○○○○○○●○○ | Kürzel Strg+E; ○○○○○○●○○ | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Ebenen als Hintergrund | Kürzel Strg+Alt+E; ○○○○○○○○○ | Kürzel Strg+Alt+E; ○○○○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Transformieren | Kürzel –; ●●●●●●●●● | Kürzel –; ○●●●●●●●● – Untermenü-Kopf grau in S1 (Original: Kopf aktiv, alle Einträge grau) (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: Untermenü-Kopf immer aktiv); vorher ⚠️ PM-7 | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Transformieren > Frei transformieren | Kürzel Strg+T; ○●●●●●●●● | Kürzel Strg+T; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Transformieren > Skalieren | Kürzel –; ○●●●●●●●● | Kürzel –; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Transformieren > Neigen | Kürzel –; ○●●●●●●●● | Kürzel –; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Transformieren > Verzerren | Kürzel –; ○●●●●●●●● | Kürzel –; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Transformieren > Numerisch transformieren | Kürzel Strg+Umschalt+T; ○●●●●●●●● | Kürzel Strg+Umschalt+T; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Transformieren > 180° drehen | Kürzel –; ○●●●●●●●● | Kürzel –; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Transformieren > 90° im UZS drehen | Kürzel Strg+Umschalt+9; ○●●●●●●●● | Kürzel Strg+Umschalt+9; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Transformieren > 90° gegen UZS drehen | Kürzel Strg+Umschalt+7; ○●●●●●●●● | Kürzel Strg+Umschalt+7; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Transformieren > Horizontal spiegeln | Kürzel –; ○●●●●●●●● | Kürzel –; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Transformieren > Vertikal spiegeln | Kürzel –; ○●●●●●●●● | Kürzel –; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Transformieren > Transformationen entfernen | Kürzel –; ○●●●○○○●○ | Kürzel –; ○●●●○○●●● – aktiv weicht ab in S7,S9 (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: im Menü grau bei Bitmap, Gruppe und Segment); vorher ❌ PM-7 | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Anordnen | Kürzel –; ●●●●●●●●● | Kürzel –; ○●●●●●●●● – Untermenü-Kopf grau in S1 (Original: Kopf aktiv, alle Einträge grau) (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: Untermenü-Kopf immer aktiv); vorher ⚠️ PM-7 | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Anordnen > In den Vordergrund | Kürzel Strg+Umschalt+Nach-oben-Taste; ○●●●●○●●● | Kürzel Strg+Umschalt+Nach-oben-Taste; ○●●●●●●●● – aktiv weicht ab in S6 (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: grau bei Pixelauswahl); vorher ❌ PM-7 | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Anordnen > Schrittweise nach vorne | Kürzel Strg+Nach-oben-Taste; ○●●●●○●●● | Kürzel Strg+Nach-oben-Taste; ○●●●●●●●● – aktiv weicht ab in S6 (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: grau bei Pixelauswahl); vorher ❌ PM-7 | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Anordnen > Schrittweise nach hinten | Kürzel Strg+Nach-unten-Taste; ○●●●●○●●● | Kürzel Strg+Nach-unten-Taste; ○●●●●●●●● – aktiv weicht ab in S6 (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: grau bei Pixelauswahl); vorher ❌ PM-7 | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Anordnen > In den Hintergrund | Kürzel Strg+Umschalt+Nach-unten-Taste; ○●●●●○●●● | Kürzel Strg+Umschalt+Nach-unten-Taste; ○●●●●●●●● – aktiv weicht ab in S6 (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: grau bei Pixelauswahl); vorher ❌ PM-7 | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Ausrichten | Kürzel –; ●●●●●●●●● | Kürzel –; ○○●○○○○○○ – Untermenü-Kopf grau in S1,S2,S4,S5,S6,S7,S8,S9 (Original: Kopf aktiv, alle Einträge grau) (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: Untermenü-Kopf immer aktiv); vorher ⚠️ PM-7 | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Ausrichten > Links | Kürzel Strg+Alt+1; ○○●○○○○○○ | Kürzel Strg+Alt+1; ○○●○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Ausrichten > Vertikal zentrieren | Kürzel Strg+Alt+2; ○○●○○○○○○ | Kürzel Strg+Alt+2; ○○●○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Ausrichten > Rechts | Kürzel Strg+Alt+3; ○○●○○○○○○ | Kürzel Strg+Alt+3; ○○●○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Ausrichten > Oben | Kürzel Strg+Alt+4; ○○●○○○○○○ | Kürzel Strg+Alt+4; ○○●○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Ausrichten > Horizontal zentrieren | Kürzel Strg+Alt+5; ○○●○○○○○○ | Kürzel Strg+Alt+5; ○○●○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Ausrichten > Unten | Kürzel Strg+Alt+6; ○○●○○○○○○ | Kürzel Strg+Alt+6; ○○●○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Ausrichten > Breiten verteilen | Kürzel Strg+Alt+7; ○○○○○○○○○ | Kürzel Strg+Alt+7; ○○○○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Ausrichten > Höhen verteilen | Kürzel Strg+Alt+9; ○○○○○○○○○ | Kürzel Strg+Alt+9; ○○○○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Pfad in Auswahlrahmen konvertieren | Kürzel –; ○●●○○○○○○ | Kürzel –; ○●●○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Pfade kombinieren | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Pfade kombinieren > Verbinden | Kürzel Strg+Umschalt+J; ○●●●○○●○● | Kürzel Strg+Umschalt+J; ○●●●○○○○○ – aktiv weicht ab in S7,S9 (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: auch bei Gruppe und Segment aktiv, ohne Pfad Meldung statt Wirkung); vorher ❌ PM-7 | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Pfade kombinieren > Teilen | Kürzel Strg+Alt+J; ○○○○○○○○○ | Kürzel Strg+Alt+J; ○○○○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Pfade kombinieren > Vereinigen | Kürzel Strg+Alt+U; ●●●●●●●●● | Kürzel Strg+Alt+U; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Pfade kombinieren > Überschneiden | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Pfade kombinieren > Öffnen | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Pfade kombinieren > Zuschneiden | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Pfad ändern | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Pfad ändern > Vereinfachen | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Pfad ändern > Strich erweitern | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Pfad ändern > Pfad einlassen | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Pfad ändern > Präzise Füllung | Kürzel –; ○○○●○○○○○ | Kürzel –; ○○○●○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Pfad ändern > Anti-Alias-Füllung | Kürzel –; ○○○●○○○○○ | Kürzel –; ○○○●○○○○○; Häkchen ·✓✓✓··✓·· | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Pfad ändern > Weiche-Kante-Füllung | Kürzel –; ○○○●○○○○○ | Kürzel –; ○○○●○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > An Pixeln ausrichten | Kürzel Strg+K; ○●●○○○●○○ | Kürzel Strg+K; ○●●○○○○○○ – aktiv weicht ab in S7 (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: auch bei Gruppe aktiv, richtet die Gruppe als Ganzes aus); vorher ❌ PM-7 | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Gruppieren | Kürzel Strg+G; ○●●●●○●●● | Kürzel Strg+G; ○●●●●●●●● – aktiv weicht ab in S6 (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: grau bei Pixelauswahl); vorher ❌ PM-7 | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Modifizieren > Gruppierung aufheben | Kürzel Strg+Umschalt+G; ○●●○○○●○○ | Kürzel Strg+Umschalt+G; ○●●○○○●○○ | ✅ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Text > Schrift | Kürzel –; ●●●●●●●●● | Kürzel –; ○○○●○○○○○ – aktiv weicht ab in S1,S2,S3,S5,S6,S7,S8,S9 (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: immer aktiv (Kopf und Einträge, ohne Text Vorgabe des Textwerkzeugs)); vorher ❌ PM-7 | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe > Andere | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe > Kleiner | Kürzel Strg+Umschalt+,; ○○○●○○○○○ | Kürzel Strg+Umschalt+,; ○○○●○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe > Größer | Kürzel Strg+Umschalt+.; ○○○●○○○○○ | Kürzel Strg+Umschalt+.; ○○○●○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe > 8 | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe > 9 | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe > 10 | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe > 12 | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●●; Häkchen ✓✓✓·✓✓✓✓✓ | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe > 14 | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe > 18 | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe > 24 | Kürzel –; ●●●●●●●●●; Häkchen ···✓····· | Kürzel –; ●●●●●●●●●; Häkchen ···✓····· | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe > 36 | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe > 48 | Kürzel –; ●●●●●●●●●; Häkchen ✓✓✓·✓✓✓✓✓ | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe > 72 | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe > 96 | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Größe > 120 | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Stil | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Stil > Standard | Kürzel –; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ | Kürzel –; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Stil > Faux Fett | Kürzel Strg+B; ●●●●●●●●● | Kürzel Strg+B; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Stil > Faux Kursiv | Kürzel Strg+I; ●●●●●●●●● | Kürzel Strg+I; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Stil > Unterstrichen | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Ausrichten | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Ausrichten > Links | Kürzel Strg+Alt+Umschalt+L; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ | Kürzel Strg+Alt+Umschalt+L; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Ausrichten > Horizontal zentrieren | Kürzel Strg+Alt+Umschalt+C; ●●●●●●●●● | Kürzel Strg+Alt+Umschalt+C; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Ausrichten > Rechts | Kürzel Strg+Alt+Umschalt+R; ●●●●●●●●● | Kürzel Strg+Alt+Umschalt+R; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Ausrichten > Blocksatz | Kürzel Strg+Alt+Umschalt+J; ●●●●●●●●● | Kürzel Strg+Alt+Umschalt+J; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Ausrichten > Oben | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Ausrichten > Vertikal zentrieren | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Ausrichten > Unten | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Ausrichten > Vertikaler Blocksatz | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > In Pfad einfügen | Kürzel –; ○○○○○○○○○ | Kürzel –; ○○○○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > An Pfad anfügen | Kürzel Strg+Umschalt+Y; ○○○○○○○○○ | Kürzel Strg+Umschalt+Y; ○○○○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Von Pfad lösen | Kürzel –; ○○○○○○○○○ | Kürzel –; ○○○○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Richtung | Kürzel –; ●●●●●●●●● | Kürzel –; ○○○○○○○○○ – Untermenü-Kopf grau in S1,S2,S3,S4,S5,S6,S7,S8,S9 (Original: Kopf aktiv, alle Einträge grau) (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: Untermenü-Kopf immer aktiv); vorher ⚠️ PM-7 | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Richtung > Um Pfad drehen | Kürzel –; ○○○○○○○○○ | Kürzel –; ○○○○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Richtung > Vertikal | Kürzel –; ○○○○○○○○○ | Kürzel –; ○○○○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Richtung > Vertikal neigen | Kürzel –; ○○○○○○○○○ | Kürzel –; ○○○○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Richtung > Horizontal neigen | Kürzel –; ○○○○○○○○○ | Kürzel –; ○○○○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Richtung umkehren | Kürzel –; ○○○○○○○○○ | Kürzel –; ○○○○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > In Pfade konvertieren | Kürzel Strg+Umschalt+P; ○○○●○○○○○ | Kürzel Strg+Umschalt+P; ○○○●○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Rechtschreibprüfung | Kürzel Umschalt+F7; ●●●●●●●●● | Kürzel Umschalt+F7; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Text > Rechtschreibprüfung einrichten | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M06_menue_text.png) |
| Filter > Filter wiederholen | Kürzel Strg+Alt+Umschalt+X; ○○○○○?○○○ | Kürzel Strg+Alt+Umschalt+X; ○○○○○○○○○ | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Andere | Kürzel –; ●●●●●●●●● | Kürzel –; ○●●●●●●●● – Untermenü-Kopf grau in S1 (Original: Kopf aktiv, alle Einträge grau) (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: Untermenü-Kopf immer aktiv); vorher ⚠️ PM-7 | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Andere > In Alphakanal konv. | Kürzel –; ○●●●●●●●● | Kürzel –; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Andere > Ränder suchen | Kürzel –; ○●●●●●●●● | Kürzel –; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Farbe anpassen | Kürzel –; ●●●●●●●●● | Kürzel –; ○●●●●●●●● – Untermenü-Kopf grau in S1 (Original: Kopf aktiv, alle Einträge grau) (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: Untermenü-Kopf immer aktiv); vorher ⚠️ PM-7 | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Farbe anpassen > Auto-Tonwertkorrektur | Kürzel –; ○●●●●●●●● | Kürzel –; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Farbe anpassen > Farbton/Sättigung | Kürzel –; ○●●●●●●●● | Kürzel –; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Farbe anpassen > Helligkeit/Kontrast | Kürzel –; ○●●●●●●●● | Kürzel –; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Farbe anpassen > Kurven | Kürzel –; ○●●●●●●●● | Kürzel –; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Farbe anpassen > Tonwertkorrektur | Kürzel –; ○●●●●●●●● | Kürzel –; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Farbe anpassen > Umkehren | Kürzel –; ○●●●●●●●● | Kürzel –; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Rauschen | Kürzel –; ●●●●●●●●● | Kürzel –; ○●●●●●●●● – Untermenü-Kopf grau in S1 (Original: Kopf aktiv, alle Einträge grau) (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: Untermenü-Kopf immer aktiv); vorher ⚠️ PM-7 | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Rauschen > Rauschen hinzufügen | Kürzel –; ○●●●●●●●● | Kürzel –; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Scharfzeichnen | Kürzel –; ●●●●●●●●● | Kürzel –; ○●●●●●●●● – Untermenü-Kopf grau in S1 (Original: Kopf aktiv, alle Einträge grau) (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: Untermenü-Kopf immer aktiv); vorher ⚠️ PM-7 | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Scharfzeichnen > Scharfzeichnen | Kürzel –; ○●●●●●●●● | Kürzel –; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Scharfzeichnen > Stark scharfzeichnen | Kürzel –; ○●●●●●●●● | Kürzel –; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Scharfzeichnen > Unscharf maskieren | Kürzel –; ○●●●●●●●● | Kürzel –; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Weichzeichnen | Kürzel –; ●●●●●●●●● | Kürzel –; ○●●●●●●●● – Untermenü-Kopf grau in S1 (Original: Kopf aktiv, alle Einträge grau) (PM-7, 12.10.) | umgesetzt, Nachprüfung ausstehend (MN-2: Untermenü-Kopf immer aktiv); vorher ⚠️ PM-7 | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Weichzeichnen > Bewegungsunschärfe | Kürzel –; ○●●●●●●●● | Kürzel –; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Weichzeichnen > Gaußscher Weichz. | Kürzel –; ○●●●●●●●● | Kürzel –; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Weichzeichnen > Radiales Weichz. | Kürzel –; ○●●●●●●●● | Kürzel –; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Weichzeichnen > Stärker weichzeichnen | Kürzel –; ○●●●●●●●● | Kürzel –; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Weichzeichnen > Weichzeichnen | Kürzel –; ○●●●●●●●● | Kürzel –; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Filter > Weichzeichnen > Zentriertes Weichz. | Kürzel –; ○●●●●●●●● | Kürzel –; ○●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Fenster > Fenster duplizieren | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Bedienfelder ausblenden | Kürzel F4; ●●●●●●●●● | Kürzel F4; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Erweiterungen | Kürzel –; ●●●●●●●●● | Kürzel –; ○○○○○○○○○ – Erweiterungen von Drittanbietern nicht nachgebaut; Firepixel: Kopf grau, darin „Keine Erweiterungen“ (PM-7, 12.10.) | ℹ️ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Erweiterungen > Kuler | Kürzel –; ●●●●●●●●● | fehlt – Erweiterung eines Drittanbieters (nicht nachgebaut) (PM-7, 12.10.) | ℹ️ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Erweiterungen > jQuery Mobile-Thema – anwendungsinterne Vorschau | Kürzel Strg+F10; ●●●●●●●●● | fehlt – Erweiterung eines Drittanbieters (nicht nachgebaut) (PM-7, 12.10.) | ℹ️ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Werkzeugleisten | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Werkzeugleisten > Standard | Kürzel –; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ | Kürzel –; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Werkzeuge | Kürzel Strg+F2; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ | Kürzel Strg+F2; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Eigenschaften | Kürzel Strg+F3; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ | Kürzel Strg+F3; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Optimieren | Kürzel F6; ●●●●●●●●● | Kürzel F6; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Ebenen | Kürzel F2; ●●●●●●●●●; Häkchen ✓✓✓✓✓·✓✓· | Kürzel F2; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Allgemeine Bibliothek | Kürzel F7; ●●●●●●●●● | Kürzel F7; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > CSS-Eigenschaften | Kürzel Strg+F7; ●●●●●●●●● | Kürzel Strg+F7; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Seiten | Kürzel F5; ●●●●●●●●●; Häkchen ·····✓··✓ | Kürzel F5; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Status | Kürzel Umschalt+F2; ●●●●●●●●● | Kürzel Umschalt+F2; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Verlauf | Kürzel Umschalt+F10; ●●●●●●●●●; Häkchen ✓✓✓✓✓·✓✓· | Kürzel Umschalt+F10; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Automatische Formen | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Stile | Kürzel Strg+F11; ●●●●●●●●● | Kürzel Strg+F11; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Dokumentbibliothek | Kürzel F11; ●●●●●●●●● | Kürzel F11; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > URL | Kürzel Alt+Umschalt+F10; ●●●●●●●●● | Kürzel Alt+Umschalt+F10; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Farbmischer | Kürzel Umschalt+F9; ●●●●●●●●● | Kürzel Umschalt+F9; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Farbfelder | Kürzel Strg+F9; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ | Kürzel Strg+F9; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Info | Kürzel Alt+Umschalt+F12; ●●●●●●●●● | Kürzel Alt+Umschalt+F12; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Verhalten | Kürzel Umschalt+F3; ●●●●●●●●● | Kürzel Umschalt+F3; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Suchen und ersetzen | Kürzel Strg+F; ●●●●●●●●● | Kürzel Strg+F; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Ausrichten | Kürzel –; ●●●●●●●●●; Häkchen ·····✓··✓ | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Eigenschaften für automatische Form | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Sonstige | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Sonstige > Farbpalette | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Sonstige > Bildbearbeitung | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Sonstige > Sonderzeichen | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Pfad | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Symboleigenschaften | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●●; Häkchen ✓✓✓✓✓✓✓✓✓ | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Überlappend | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Anordnen | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Hilfe > Firepixel-Hilfe | Kürzel F1; ●●●●●●●●● | Kürzel F1; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M10_menue_hilfe.png) |
| Hilfe > Firepixel Support Center | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M10_menue_hilfe.png) |
| Hilfe > Firepixel-Programm zur Produktverbesserung | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M10_menue_hilfe.png) |
| Hilfe > Firepixel Exchange | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M10_menue_hilfe.png) |
| Hilfe > Erweiterungen verwalten | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M10_menue_hilfe.png) |
| Hilfe > Firepixel-Onlineforen | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M10_menue_hilfe.png) |
| Hilfe > Über Firepixel | Kürzel –; ●●●●●●●●● | Kürzel –; ●●●●●●●●● | ✅ | [Bild](screenshots/paritaet/b_M10_menue_hilfe.png) |
| Hilfe > Produktregistrierung | Kürzel –; ○○○○○○○○○ | fehlt – nur im Original, bewusst weggelassen (Lizenz/Registrierung) (PM-7, 12.10.) | ℹ️ | [Bild](screenshots/paritaet/b_M10_menue_hilfe.png) |
| Hilfe > Deaktivieren | Kürzel –; ○○○○○○○○○ | fehlt – nur im Original, bewusst weggelassen (Lizenz/Registrierung) (PM-7, 12.10.) | ℹ️ | [Bild](screenshots/paritaet/b_M10_menue_hilfe.png) |
| Hilfe > Aktualisierungen | Kürzel –; ○○○○○○○○○ | fehlt – nur im Original, bewusst weggelassen (Lizenz/Registrierung) (PM-7, 12.10.) | ℹ️ | [Bild](screenshots/paritaet/b_M10_menue_hilfe.png) |
| Filter > Wiederholen | Kürzel Strg+Alt+Umschalt+X; ?????●??? | Kürzel –; – – nur gesehen, nachdem ein anderer Agent im Original Farbton/Sättigung benutzt hatte: Original „Wiederholen: Farbton/Sättigung...“ (aktiv, Strg+Alt+Umschalt+X); Firepixel hat dafür „Filter wiederholen“ mit fester Beschriftung (`cmd-filter-repeat`), der Schrittname fehlt – im echten Fenster nach einem Filter nachprüfen (PM-7, 12.10.) | ⚠️ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Ansicht > Design | – | Kürzel – (nur Firepixel) – nur in Firepixel (PM-7, 12.10.) | ℹ️ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Design > Normal | – | Kürzel – (nur Firepixel) – nur in Firepixel (PM-7, 12.10.) | ℹ️ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Design > Dunkel | – | Kürzel – (nur Firepixel) – nur in Firepixel (PM-7, 12.10.) | ℹ️ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Design > OLED | – | Kürzel – (nur Firepixel) – nur in Firepixel (PM-7, 12.10.) | ℹ️ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Sprache | – | Kürzel – (nur Firepixel) – nur in Firepixel (PM-7, 12.10.) | ℹ️ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Sprache > Deutsch | – | Kürzel – (nur Firepixel) – nur in Firepixel (PM-7, 12.10.) | ℹ️ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Sprache > English | – | Kürzel – (nur Firepixel) – nur in Firepixel (PM-7, 12.10.) | ℹ️ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Sprache > Testsprache (qps) | – | Kürzel – (nur Firepixel) – nur in Firepixel (PM-7, 12.10.) | ℹ️ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Ansicht > Sprache > Sprachordner öffnen | – | Kürzel – (nur Firepixel) – nur in Firepixel (PM-7, 12.10.) | ℹ️ | [Bild](screenshots/paritaet/b_M03_menue_ansicht.png) |
| Auswählen > Motiv auswählen | – | Kürzel – (nur Firepixel) – nur in Firepixel (PM-7, 12.10.) | ℹ️ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Auswählen > Kante verfeinern | – | Kürzel – (nur Firepixel) – nur in Firepixel (PM-7, 12.10.) | ℹ️ | [Bild](screenshots/paritaet/b_M04_menue_auswaehlen.png) |
| Modifizieren > Bitmap nachzeichnen | – | Kürzel – (nur Firepixel) – nur in Firepixel (PM-7, 12.10.) | ℹ️ | [Bild](screenshots/paritaet/b_M05_menue_modifizieren.png) |
| Filter > Farbe anpassen > Farbfüllung | – | Kürzel – (nur Firepixel) – nur in Firepixel (PM-7, 12.10.) | ℹ️ | [Bild](screenshots/paritaet/b_M08_menue_filter.png) |
| Fenster > Arbeitsbereichlayouts | – | Kürzel – (nur Firepixel) – nur in Firepixel (PM-7, 12.10.) | ℹ️ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Arbeitsbereichlayouts > Erweiterter Modus | – | Kürzel – (nur Firepixel) – nur in Firepixel (PM-7, 12.10.) | ℹ️ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Arbeitsbereichlayouts > Symbolmodus | – | Kürzel – (nur Firepixel) – nur in Firepixel (PM-7, 12.10.) | ℹ️ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Arbeitsbereichlayouts > Symbolmodus mit Bedienfeldnamen | – | Kürzel – (nur Firepixel) – nur in Firepixel (PM-7, 12.10.) | ℹ️ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Arbeitsbereichlayouts > Aktuelles speichern | – | Kürzel – (nur Firepixel) – nur in Firepixel (PM-7, 12.10.) | ℹ️ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Arbeitsbereichlayouts > Standardarbeitsbereich | – | Kürzel – (nur Firepixel) – nur in Firepixel (PM-7, 12.10.) | ℹ️ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |
| Fenster > Erweiterungen > Keine Erweiterungen | – | Kürzel – (nur Firepixel) – nur in Firepixel (PM-7, 12.10.) | ℹ️ | [Bild](screenshots/paritaet/b_M09_menue_fenster.png) |

## 2. Dialoge (Feld für Feld)

Original: Dialog über das Menü geöffnet (Nachrichten an Menüleiste und Menüfenster), alle Steuerelemente mit Tab-Folge, Standardknopf, Fokus, Markierung und Werten über die Win32-API gelesen (`re\_testdateien\ux\paritaet\dialoge\o_dialoge_bericht.txt`), danach abgebrochen. Firepixel: derselbe Befehl über `ui_action`, Bild über die Brücke. Lage: S1 = Testdokument ohne Auswahl.

**Nachmessung D01–D25 (PM-5, 12.10. nachmittags):** beide Programme neu, Feld für Feld. Original: eigenes Testdokument `pm5_test.fw` (500 × 400, R1, R2, T1 „Hallo“ 24 pt, B1; am Ende ohne Speichern verworfen), Hauptfenster 1032 × 748. Dialoge über das Menü geöffnet, Felder/Tab-Folge/Fokus/Standardknopf per Win32 gelesen, Tasten (Tab, Enter, Esc) als Fensternachricht an das fokussierte Element (die RDP-Sitzung hatte kein Vordergrundfenster, echte SendInput-Eingaben waren nicht möglich), Wirkung und Rückgängig per JSF bzw. Rückgängig-Befehl, Bilder per PrintWindow. Firepixel: eigene Debug-Instanz aus einem sauberen Worktree von HEAD (`ecf43b3`), eigenes APPDATA, `--mcp`, gleiches Dokument, gleiches Fenster 1032 × 748; echtes Fenster, Brücke (`ui_find`/`ui_input`/`screenshot`, Tab/Enter/Esc/Strg+Z als Tasten). Bilder `docs\screenshots\paritaet\pm5_D<nr>_*.png` (links Original, rechts Firepixel; Stellen mit dem Produktnamen im Original sind grau überdeckt, Dateidialoge des Originals nicht abgelegt). Nicht gemessen: Popup-Schieber ▾ des Originals (D18, D23) mit der Maus (kein Vordergrund); in Firepixel fehlen sie in beiden Dialogen ohnehin.

**DLG-1 (12.10. abends, umgesetzt, Nachprüfung im echten Fenster ausstehend, NACHPRUEFUNG):** Zahlenfelder nehmen in allen Dialogen wieder Eingaben an, auch ohne Fensterfokus (`fc7df14`). Tab-Folge (`173bca1`): Titelleiste und Schließen-Kreuz nie Tab-Stopp; Startfokus erstes Eingabefeld bzw. erste Optionsgruppe, sonst Standardknopf; D11/D13/D22/D24 starten wie gemessen auf OK; Optionsgruppen = 1 Stopp mit Pfeiltasten; D01 … › OK › Abbrechen › Vorlagen…. Die Bewertung der Zeilen bleibt, weil dort weitere Abweichungen offen sind (D19/D20/D01-Rest, Verlaufsschritte → DLG-1b; D16 Kategorieliste per Tab, D20 Anker per Tab offen).

**DLG-2 (12.10. abends, umgesetzt, Nachprüfung im echten Fenster ausstehend, NACHPRUEFUNG):** `9469497`. D18 Raster bearbeiten: nur Farbe und zwei Abstände mit ↔/↕-Doppelpfeilen, keine Kästchen, Lage nach Vorlage 5984 (151 × 89 DLU ≈ 227 × 145 px Client), Farbfeld kein Tab-Stopp; Popup-Schieber ▾ fehlt weiter (im Original nicht gemessen, Vorlage ohne Bereich; Befund DLG-2), Verlaufsschritt → DLG-1b. D21 Leinwandfarbe: Optionen untereinander, „Benutzerdefiniert:“, Farbfeld eingerückt darunter, 230 px. D22 Selektives JPEG: Qualität neben dem Haken, „Überlagerungsfarbe:“ (`SelectiveJPEGColor`, #ff0080, bei OK gemerkt), Trennlinie; ohne Haken Qualität, Überlagerungsfarbe und beide Kästchen gesperrt. D23 Textgröße: Wert = Größe des gewählten Texts, sonst zuletzt bestätigter Wert, sonst Werkzeugvorgabe; Popup-Schieber 8..96, keine Einheit, OK/Abbrechen rechts untereinander (183 × 50 DLU).

**DLG-1b (12.10. abends, umgesetzt, Nachprüfung im echten Fenster ausstehend, NACHPRUEFUNG):** D19 (`6c557e6`): Höhe und Druckgröße beim Tippen nachgeführt, Höhe ungerundet (B 333 → 266 / 3.7), Druckgröße 3 Stellen mit Punkt, Kettensymbole, OK ohne Änderung ohne Befehl. D20 (`6c557e6`): ↔/↕ vor den Feldern, „B: 600“/„H: 400“ live rechts, Anker 3 × 3 mit Pfeilen als ein Tab-Stopp (Pfeiltasten, Barrierefreiheit: Optionsfelder mit Namen), „Nur aktuelle Seite“ in „Neue Größe“, nach OK alle Objekte ausgewählt (gleicher Schritt). D01 (`91a4d90`): Zugriffstasten &Breite/&Höhe/&Auflösung/We&iß/&Transparent/Be&nutzerdefiniert (unterstrichen, Alt+Taste). Verlauf (`00809ba`): Raster bearbeiten, Exportassistent und HTML-Einrichtung ohne Verlaufsschritt; D18 OK ohne Änderung führt nichts aus. D16 (`aabc863`): Kategorieliste hat den Startfokus, ist ein Tab-Stopp, Pfeiltasten/Pos1/Ende wählen.

**Nachmessung D26–D50 (PM-6, 12.10. abends):** beide Programme neu, Feld für Feld, gleiche Mittel wie PM-5. Original: eigene Kopie des PM-5-Testdokuments (`pm6_test.fw`, R1, R2, T1, B1, Doppel entfernt; am Ende ohne Speichern geschlossen), kurze Sitzungen unter der Sperre, Tasten als Fensternachricht an das fokussierte Element, Felder und Schieber (Bereich/Lage) per Win32, Wirkung per JSF (Rechtecke, Elementtyp) und „Rückgängig: …“ aus dem Bearbeiten-Menü. Pixelauswahl über einen in einen Auswahlrahmen umgewandelten Hilfspfad; ihre Grenzen ließen sich im Original per JSF nicht lesen (`convertMarqueeToPath` bricht ab), deshalb dort nur der Verlaufsschritt. Firepixel: eigene Debug-Instanz aus einem sauberen Worktree von HEAD (`60365e8`, mit der Zahlenfeld-Korrektur `fc7df14`), eigenes Profil, `--mcp`, sichtbar und nicht aktiviert unten rechts; echtes Fenster, Brücke (`ui_find`/`ui_input`/`screenshot`), Wirkung über `run_script`/`get_document`/`history`. Jeder Dialog mit Zahlenfeldern 4–8× geöffnet und jedes Mal getippt. Bilder `docs\screenshots\paritaet\pm6_D<nr>_*.png` (Titelleiste mit dem Programmnamen bei D33/D34 grau überdeckt). Nicht gemessen: Popup-Schieber ▾ des Originals mit der Maus (kein Vordergrund), Wirkung der Knöpfe Hotspot/Segment (D34). Sammelbefunde, die die Bewertung nicht senken: Titelleiste als Tab-Stopp und Optionsgruppen mit mehreren Stopps (allgemeiner Befund PM-5, DLG-1); Verlaufsnamen (Original bei allen Filtern „Bild filtern“, Firepixel der Filtername).

| Nr | Dialog | Original | Firepixel | | Bild |
|---|---|---|---|---|---|
| D01 | Datei ▸ Neu | „Neues Dokument“ 396 × 312, mittig über dem Hauptfenster; Fokus Breite, ganz markiert; Werte zuletzt bestätigt (OK mit 123 × 77, zweites Öffnen 123 × 77); Gruppentitel live („Leinwandgröße: 37.0K“); Benutzerdefiniert schwarz; Zugriffstasten unterstrichen; Tab: Breite › Höhe › Auflösung › 3 Einheiten › Weiß (Optionsgruppe = 1 Stopp) › OK › Abbrechen › Vorlagen…; Enter = OK (neues Dokument), Esc = Abbrechen | 385 × 313, mittig; Fokus/Markierung, Gruppentitel „37.0K“, gemerkte Werte 123 × 77, Benutzerdefiniert schwarz, Enter/Esc gleich. Abweichend: ~~keine Zugriffstasten~~ (DLG-1b `91a4d90`) (Tab-Folge seit DLG-1 `173bca1` wie im Original: Optionsgruppe 1 Stopp, Farbfeld kein Stopp, OK › Abbrechen › Vorlagen…; Nachprüfung ausstehend); frisches Profil 660 × 440 | ⏳ umgesetzt, Nachprüfung ausstehend (DLG-1b `91a4d90`; vorher ⚠️ PM-5, 12.10., echtes Fenster (Firepixel: Brücke)) | [Bild](screenshots/paritaet/pm5_D01_neues_dokument.png) |
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
| D16 | Bearbeiten ▸ Voreinstellungen… | 656 × 442: Kategorieliste (hat den Fokus), „Allgemein“ gruppiert (Dokumentoptionen, Speichern, Benutzeroberfläche, Standardfarben), Hinweis „Änderungen werden erst nach Neustart aktiv“, Hilfe/OK/Abbrechen; Tab Liste › Startseite › Pinselstriche › Rückgängig-Schritte …; Enter = OK, Esc schließt | 634 × 583: Kategorien „Bearbeiten“ (Original „Bearbeitung“), „Photoshop-Import/Öffnen“ (Original „Photoshop-Datei importieren“), „Zusatzmodule“ vor „Starten und Bearbeiten“; Felder untereinander ohne Gruppen, ohne Neustart-Hinweis und ohne „Hilfe“, zusätzlich Entwickleroptionen/Design/Sprache; ~~Liste nicht per Tab erreichbar~~ (DLG-1b `aabc863`: Startfokus, ein Tab-Stopp, Pfeiltasten; Kategorien/Gruppen offen); Enter/Esc gleich | ⚠️ teilweise umgesetzt, Nachprüfung ausstehend (DLG-1b `aabc863`; ⚠️ PM-5, 12.10., echtes Fenster) | [Bild](screenshots/paritaet/pm5_D16_voreinstellungen.png) |
| D17 | Bearbeiten ▸ Tastaturkurzbefehle… | 480 × 500: Satzwahl + 4 Symbolknöpfe, „Befehle:“-Liste, **Menübaum**, Beschreibung, Kurzbefehle +/–, „Taste drücken“ + Ändern; Fokus OK; Enter/Esc | 678 × 587: Textknöpfe, flache Liste mit Suche, ohne Beschreibung; Tab zeigt keinen Fokus; Enter/Esc gleich | ⚠️ PM-5, 12.10., echtes Fenster | [Bild](screenshots/paritaet/pm5_D17_tastaturkurzbefehle.png) |
| D18 | Ansicht ▸ Raster ▸ Raster bearbeiten… | 233 × 174: Farbe cyan, Abstände 36/36 mit Popup-Schieber ▾, Fokus erstes Feld markiert; Tab Feld › Feld › OK › Abbrechen; OK mit 20/30 wirkt, **kein Verlaufsschritt** (Strg+Z ändert nichts), zweites Öffnen 20/30; Abbrechen ohne Wirkung | 334 × 217: Farbe cyan ✅; zusätzlich „Raster anzeigen“/„Am Raster ausrichten“, Beschriftung „Abstand ↔:“ statt Symbol, **kein ▾**; ~~OK 20/30 wirkt, aber als Verlaufsschritt (Strg+Z nimmt zurück), OK ohne Änderung legt ebenfalls einen Schritt an~~ (DLG-1b `00809ba`: kein Schritt, OK ohne Änderung führt nichts aus); ~~Tab hält an der Titelleiste~~ (DLG-1 `173bca1`: kein Tab-Stopp mehr, Nachprüfung ausstehend). Schieber: im Original ▾ nicht mit der Maus gemessen (kein Vordergrund in der RDP-Sitzung), in Firepixel nicht vorhanden | ⏳ umgesetzt, Nachprüfung ausstehend (DLG-2 `9469497`, DLG-1b `00809ba`; vorher ⚠️ PM-5) | [Bild](screenshots/paritaet/pm5_D18_raster_bearbeiten.png) |
| D19 | Modifizieren ▸ Leinwand ▸ Bildgröße… | 410 × 313: Fokus Breite markiert; Höhe und Druckgröße **beim Tippen** nachgeführt (B 250 → H 200, Druck 3.472 × 2.778; B 333 → 266, 4.625 × 3.7); Kettensymbole; Druckgröße 3 Nachkommastellen mit Punkt; Tab über alle Felder/Einheiten › 3 Kästchen › Neuauflösung › OK › Abbrechen; OK 250 → 250 × 200, Strg+Z stellt 500 × 400 her; „Proportionen beibehalten“ nur bei OK gemerkt | 348 × 375: ~~Höhe und Druckgröße erst nach Tab/Fokusverlust; Druckgröße 2 Nachkommastellen mit Komma (6,94), keine Kettensymbole~~ (DLG-1b `6c557e6`); sonst gleich (250 × 200, Strg+Z, gemerktes Kästchen nur bei OK, Abbrechen ohne Wirkung, Tab-Folge); ~~OK ohne Änderung legt einen Verlaufsschritt an~~ (DLG-1b) | ⏳ umgesetzt, Nachprüfung ausstehend (DLG-1b `6c557e6`; vorher ⚠️ PM-5, 12.10., echtes Fenster) | [Bild](screenshots/paritaet/pm5_D19_bildgroesse.png) |
| D20 | Modifizieren ▸ Leinwand ▸ Leinwandgröße… | 377 × 331: Felder mit ↔/↕-Symbolen, rechts „B: 600“/„H: 400“ live; Anker 3 × 3 mit Pfeilsymbolen (Mitte); „Nur aktuelle Seite“ in der Gruppe; Tab B › H › Einheiten › Anker › OK › Abbrechen › Seite; OK 600 → R1 +50 px und **alle Objekte ausgewählt**; Strg+Z ✅; Anker nicht gemerkt (wieder Mitte) | 332 × 342: ~~Anker als 9 Kästchen ohne Pfeile, gewählter Anker jetzt blau umrandet; Beschriftung „B:/H:“ statt Symbole, keine Live-Anzeige rechts; „Nur aktuelle Seite“ außerhalb der Gruppe; Anker per Tab und Barrierefreiheit nicht erreichbar~~ (DLG-1b `6c557e6`); OK 600 → R1 +50 px ✅, ~~Auswahl bleibt leer~~ (DLG-1b: alle Objekte ausgewählt); Strg+Z und Abbrechen ✅ | ⏳ umgesetzt, Nachprüfung ausstehend (DLG-1b `6c557e6`; vorher ⚠️ PM-5, 12.10., echtes Fenster) | [Bild](screenshots/paritaet/pm5_D20_leinwandgroesse.png) |
| D21 | Modifizieren ▸ Leinwand ▸ Leinwandfarbe… | 236 × 175, Optionen untereinander, Fokus „Weiß“; Tab Weiß › OK › Abbrechen; Transparent + Enter → #ffffff00, Benutzerdefiniert → #000000, Strg+Z ✅; Wahl nicht gemerkt (Weiß) | 374 × 139, Optionen **nebeneinander**, „Farbe:“ darunter; Tab: ~~Titelleiste › jedes Optionsfeld einzeln~~ (DLG-1 `173bca1`: Startfokus „Weiß“, Optionsgruppe 1 Stopp mit Pfeiltasten, Nachprüfung ausstehend); Wirkung, Strg+Z, Abbrechen und „nicht gemerkt“ gleich | ⏳ umgesetzt, Nachprüfung ausstehend (DLG-2 `9469497`; vorher ⚠️ PM-5) | [Bild](screenshots/paritaet/pm5_D21_leinwandfarbe.png) |
| D22 | Modifizieren ▸ Selektives JPEG ▸ Einstellungen… | 300 × 182: Qualität 90, „Überlagerungsfarbe“, Text-/Schaltflächenqualität **grau, bis „Selektive Qualität aktivieren“ angehakt ist**; Fokus OK; Esc verwirft; OK (☑, 70) gemerkt, Strg+Z nimmt zurück | 394 × 191: alles bedienbar ohne Haken, **ohne Überlagerungsfarbe**; OK/gemerkt/Strg+Z/Esc gleich; ~~Tab hält an der Titelleiste~~ (DLG-1 `173bca1`: kein Tab-Stopp mehr, Nachprüfung ausstehend) | ⏳ umgesetzt, Nachprüfung ausstehend (DLG-2 `9469497`; vorher ❌ PM-5) | [Bild](screenshots/paritaet/pm5_D22_selektives_jpeg.png) |
| D23 | Text ▸ Größe ▸ Andere… | 281 × 110: Feld mit ▾, Wert = Größe des gewählten Texts (24) bzw. ohne Auswahl zuletzt benutzt (48); Fokus Feld markiert; Enter 48 → Text 48 pt, Strg+Z ✅; auch ohne Textauswahl verfügbar | 334 × 113: Wert **immer 12** (auch bei 24-pt-Text), Einheit „Punkt“, **kein ▾**; Wirkung 48 pt und Strg+Z ✅; ~~Tab hält an der Titelleiste~~ (DLG-1 `173bca1`: kein Tab-Stopp mehr, Nachprüfung ausstehend) | ⏳ umgesetzt, Nachprüfung ausstehend (DLG-2 `9469497`; vorher ❌ PM-5) | [Bild](screenshots/paritaet/pm5_D23_textgroesse.png) |
| D24 | Text ▸ Rechtschreibprüfung einrichten… | 339 × 338: Wörterbuchliste mit Kästchen, Pfad zum persönlichen Wörterbuch + Ordnerknopf, „Persönliches Wörterbuch bearbeiten“, 4 Optionen; Fokus OK; Esc schließt | unverändert 454 × 311: Sprachkürzel „de_DE“ als Text, Ordnerfeld, 2 Optionen, Wörterbuch als Textfeld | ❌ PM-5, 12.10., echtes Fenster | [Bild](screenshots/paritaet/pm5_D24_rechtschreibung_einrichten.png) |
| D25 | Befehle ▸ Gespeicherte Befehle verwalten… | ohne gespeicherte Befehle nur **Meldung** (Info-Symbol, Titel = Programmname): „Befehle werden dem Verlauf-Bedienfeld entnommen. Zurzeit sind dort keine gespeicherten Befehle vorhanden.“; Enter/Esc schließen | Verwaltungsliste 418 × 329 mit den zwei mitgelieferten Befehle-Menü-Skripten („Größe ausgewählter Objekte ändern“, „Warndialoge zurücksetzen“) als „gespeicherte Befehle“; Umbenennen/Löschen grau, Fertig | ❌ PM-5, 12.10., echtes Fenster | [Bild](screenshots/paritaet/pm5_D25_gespeicherte_befehle.png) |
| D26 | Modifizieren ▸ Transformieren ▸ Numerisch transformieren… (R1) | 326 × 193; Liste Skalieren/Größe ändern/Drehen (Startfokus); Felder mit ↔/↕-Symbol und Kette, rechts live „B: 100 / H: 80“ (Objektgröße); Tab Liste › B › H › Attribute › Proportionen › OK › Abbrechen; B 50 → H folgt sofort 50; Enter → R1 45,40,95,80, Strg+Z ✅; Werte **gemerkt** (zweites Öffnen 50/50); Esc ohne Wirkung | ≈ 325 × 197; „Breite:/Höhe:“ statt Symbolen, **ohne Objektgröße B/H und ohne Kette**; Startfokus Breite (markiert); H folgt beim Tippen nicht (erst beim OK proportional); Enter → R1 45,40,95,80 ✅, Strg+Z ✅, Esc ✅; Werte ~~**nicht gemerkt**~~ (DLG-5: gemerkt) (wieder 100) | ⚠️ PM-6, 12.10., echtes Fenster (Brücke); gemerkte Werte ⏳ umgesetzt, Nachprüfung ausstehend (DLG-5 `5f06b0c`) | [Bild](screenshots/paritaet/pm6_D26_numerisch_transformieren.png) |
| D27 | Modifizieren ▸ Symbol ▸ In Symbol konvertieren… (R1) | 495 × 195; Name „Symbol“ markiert (Fokus); Typ Grafik/Animation/Schaltfläche **untereinander**; „Optionen:“ 9-Slice, „In allgemeiner Bibliothek speichern“; OK/Abbrechen rechts; Tab Name › Typ (1 Stopp) › OK › Abbrechen › 9-Slice › Bibliothek; Enter → Instanz, Strg+Z ✅; Name nicht gemerkt | 414 × 189; Typ **nebeneinander**, ohne „Optionen:“, „Für allgemeine Bibliothek speichern“, Knöpfe unten; Tab: jedes Optionsfeld einzeln; Enter → Instanz, Strg+Z ✅, Esc ✅; nicht gemerkt wie im Original | ⚠️ PM-6, 12.10., echtes Fenster (Brücke) | [Bild](screenshots/paritaet/pm6_D27_in_symbol_konvertieren.png) |
| D28 | Modifizieren ▸ Animation ▸ Auswahl animieren… (R1) | „Animieren“ 329 × 279, Fokus OK; Status 5 / Verschieben 72 / Richtung 0 / Skalieren auf 100 / Deckkraft 100 **bis** 100 (eine Zeile) / Drehen 0 + Nach rechts/Nach links, Trennlinien, ▾ an jedem Feld; Tab OK › Abbrechen › Felder › Nach rechts; Enter → Animationssymbol (Rechteck 20,20,192,100 mit Bewegungsweg), Strg+Z ✅; nicht gemerkt | 372 × 293; Richtung 0 ✅, Popup ▾ senkrecht an jedem Feld ✅ (Ziehen ändert den Wert); Beschriftungen „Bewegung“, „Skalierung auf“, „Deckkraft von“ + eigene Zeile „bis“, „Drehung“, „Im/Entgegen dem Uhrzeigersinn“; ohne Trennlinien; Startfokus Status statt OK; Enter → Animationssymbol (Rechteck 20,20,120,100), Strg+Z ✅; nicht gemerkt ✅ | ⚠️ PM-6, 12.10., echtes Fenster (Brücke) | [Bild](screenshots/paritaet/pm6_D28_auswahl_animieren.png) |
| D29 | Modifizieren ▸ Pfad ändern ▸ Vereinfachen… (R1) | 216 × 110; „Menge:“ 10 mit ▾, Fokus Feld markiert; Tab Feld › OK › Abbrechen; Enter 20 → R1 wird Pfad, Strg+Z ✅; gemerkt (20) | 332 × 111; „**Betrag:**“ 10, ▾ senkrecht ✅; Fokus Feld; Enter 20 → Schritt „Vereinfachen“, Strg+Z ✅; gemerkt ✅ | ⚠️ PM-6, 12.10., echtes Fenster (Brücke) | [Bild](screenshots/paritaet/pm6_D29_vereinfachen.png) |
| D30 | Modifizieren ▸ Pfad ändern ▸ Strich erweitern… (R1) | 231 × 227; Breite 10 ▾; Ecken 3 Symbolknöpfe (3. gewählt); „Winkelgrenze“ **10.00**; „Endkappen“ 3 Symbolknöpfe (1. gewählt); Tab Breite › Ecken (1 Stopp) › Winkelgrenze › Endkappen (1 Stopp) › OK › Abbrechen; Enter 5 → Pfad 17,17,123,103, Strg+Z ✅; gemerkt (5) | ≈ 392 × 189; Symbolknöpfe ✅, ▾ ✅; „Gehrungsgrenze“ **10** (ohne Nachkommastellen), „Enden“; Tab: jeder Symbolknopf einzeln; Enter 5 → 17,17,123,103 ✅, Strg+Z ✅; gemerkt ✅ | ⚠️ PM-6, 12.10., echtes Fenster (Brücke) | [Bild](screenshots/paritaet/pm6_D30_strich_erweitern.png) |
| D31 | Modifizieren ▸ Pfad ändern ▸ Pfad einlassen… (R1) | 264 × 239; Richtung Innen/Außen **untereinander**; Breite 10 ▾ (Startfokus); Ecken Symbolknöpfe; „Winkelgrenze“ 10.00; Enter 5 → Pfad 25,25,115,95, Strg+Z ✅; gemerkt | ≈ 392 × 189; Richtung **nebeneinander**, „Gehrungsgrenze“ 10; Symbolknöpfe ✅, ▾ ✅; Enter 5 → 25,25,115,95 ✅, Strg+Z ✅; gemerkt ✅ | ⚠️ PM-6, 12.10., echtes Fenster (Brücke) | [Bild](screenshots/paritaet/pm6_D31_pfad_einlassen.png) |
| D32 | Modifizieren ▸ Pfad in Auswahlrahmen konvertieren… (R1) | Dialog 345 × 110: „Rand:“ Hart/**Anti-Alias**/Weiche Kante + Breite mit ▾ (grau außer bei Weiche Kante), Fokus OK; Enter → Auswahlrahmen, Strg+Z ✅ | **kein Dialog**: der Befehl wandelt sofort (Kante fest), Strg+Z ✅ | ❌ PM-6, 12.10., echtes Fenster (Brücke) | [Bild](screenshots/paritaet/pm6_D32_pfad_in_auswahlrahmen.png) |
| D33 | Modifizieren ▸ Symbol ▸ Instanzen tweenen… (ohne 2 Instanzen) | Meldung mit Warnsymbol (Titel = Programmname): „Wählen Sie zwei oder mehr Instanzen desselben Bibliothekssymbols aus, bevor Sie den Tween-Befehl verwenden.“, „Nicht mehr anzeigen“, Fokus OK; Enter/Esc schließen, kein Schritt | gleiche Meldung (Titel „Firepixel“), Warnsymbol, „Nicht mehr anzeigen“; Enter/Esc ✅, kein Schritt; Tab nur mit dem allgemeinen Titelleisten-Stopp (DLG-1) | ✅ PM-6, 12.10., echtes Fenster (Brücke) | [Bild](screenshots/paritaet/pm6_D33_instanzen_tweenen.png) |
| D34 | Modifizieren ▸ Popupmenü ▸ Popupmenü hinzufügen… (ohne Hotspot/Segment) | Rückfrage „Verhalten werden über das Verhalten-Bedienfeld auf Hotspot- oder Segmentobjekte angewendet. Hotspot oder Segment erstellen?“ mit **Hotspot** (Standard, Fokus) / Segment / Abbrechen, auch ohne Auswahl; nach Esc/Abbrechen **zweite Meldung** „Wählen Sie ein Segment oder einen Hotspot aus, bevor Sie den Befehl „Verhalten einfügen“ verwenden.“ (mit „Nicht mehr anzeigen“) | gleiche Rückfrage mit Hotspot/Segment/Abbrechen (Menüpunkt jetzt aktiv, auch ohne Auswahl); Esc schließt **ohne** die zweite Meldung | ⚠️ PM-6, 12.10., echtes Fenster (Brücke) | [Bild](screenshots/paritaet/pm6_D34_popupmenue_hinzufuegen.png) |
| D35 | Auswählen ▸ Weiche Kante… (Pixelauswahl) | „Weiche Auswahlkante“ 314 × 104, Radius 10 Pixel, Fokus markiert; Enter 5 → Schritt „Weiche Auswahlkante“, Strg+Z ✅; **gemerkt** (5) | 332 × 111, gleiche Felder; Enter 5 → Auswahl 325,205,385,265, Schritt „Weiche Kante“, Strg+Z ✅; ~~**nicht gemerkt**~~ (DLG-5: gemerkt) (wieder 10) | ⚠️ PM-6, 12.10., echtes Fenster (Brücke); gemerkte Werte ⏳ umgesetzt, Nachprüfung ausstehend (DLG-5 `5f06b0c`) | [Bild](screenshots/paritaet/pm6_D35_weiche_kante.png) |
| D36 | Auswählen ▸ Auswahlrahmen erweitern… | 327 × 104, „Erweitern um:“ 10 Pixel; Enter 5 → Schritt „Auswahl erweitern“, Strg+Z ✅; gemerkt (5), Esc verwirft | gleich; Enter 5 → Auswahl 330,210,380,260 → 324,204,386,266 (Rand +6 bei 5, +11 bei 10, +1 bei 1; im Original per JSF nicht lesbar), Strg+Z ✅; gemerkt ✅ | ✅ PM-6, 12.10., echtes Fenster (Brücke) | [Bild](screenshots/paritaet/pm6_D36_auswahlrahmen_erweitern.png) |
| D37 | Auswählen ▸ Auswahlrahmen verkleinern… | 327 × 104, „Verkleinern um:“ 10 Pixel; Enter 5 → Schritt „Auswahl verkleinern“, Strg+Z ✅; gemerkt | gleich; Enter 5 → 335,215,375,255, Strg+Z ✅; gemerkt ✅ | ✅ PM-6, 12.10., echtes Fenster (Brücke) | [Bild](screenshots/paritaet/pm6_D37_auswahlrahmen_verkleinern.png) |
| D38 | Auswählen ▸ Auswahlrahmen begrenzen… | 287 × 104, „**Breite:**“ 10 Pixel; Enter 5 → Schritt „Rand auswählen“, Strg+Z ✅; gemerkt | „**Rand:**“ 10 Pixel; Enter 5 → Rand um die Auswahl, Strg+Z ✅; gemerkt ✅ | ⚠️ PM-6, 12.10., echtes Fenster (Brücke) | [Bild](screenshots/paritaet/pm6_D38_auswahlrahmen_begrenzen.png) |
| D39 | Auswählen ▸ Auswahlrahmen glätten… | 345 × 110, „**Auswahlradius:**“ 10 Pixel; Enter 5 → Schritt „Auswahl glätten“, Strg+Z ✅; gemerkt | „**Stichprobenradius:**“ 10 Pixel; Enter 5 → Schritt „Auswahlrahmen glätten“, Strg+Z ✅; gemerkt ✅ | ⚠️ PM-6, 12.10., echtes Fenster (Brücke) | [Bild](screenshots/paritaet/pm6_D39_auswahlrahmen_glaetten.png) |
| D40 | Auswählen ▸ Bitmap-Auswahl speichern… | „Auswahl speichern“ 302 × 305: Gruppe **Ziel** (Dokument-Liste, Auswahl „Neu“/vorhandene, Name „Standard“ markiert), Gruppe **Vorgang** (Neue Auswahl / Hinzufügen / Entfernen / Schnittmenge, ohne vorhandene grau); OK → Schritt „Auswahl speichern“; danach steht „PM6Sel“ in der Auswahl-Liste | nur „Name: Auswahl 1“ (332 × 111); OK → Schritt „Bitmap-Auswahl speichern“; Vorgang nur in „Auswahl wiederherstellen“ | ❌ PM-6, 12.10., echtes Fenster (Brücke) | [Bild](screenshots/paritaet/pm6_D40_auswahl_speichern.png) |
| D41 | Filter ▸ Farbe anpassen ▸ Farbton/Sättigung… (B1) | 459 × 193; Felder rechts neben den Schiebern (−180…180, −100…100, −100…100), OK/Abbrechen/„Kolorieren“/Vorschau rechts; Fokus Farbton; Tab nur Felder › OK › Abbrechen › Kolorieren › Vorschau; Enter (30/50) → Schritt „Bild filtern“, Strg+Z ✅; **gemerkt** (30/50) | ≈ 334 × 260; Felder über den Schiebern, Knöpfe unten, „**Färben**“; Schieber sind Tab-Stopps; Enter → Schritt „Farbton/Sättigung“, Strg+Z ✅, Esc ✅; ~~**nicht gemerkt**~~ (DLG-5: gemerkt) | ⏳ umgesetzt, Nachprüfung ausstehend (DLG-5 `5f06b0c`: Feld rechts neben dem Schieber, Knöpfe rechts, Beschriftungen, Startfokus, Dezimalpunkt, gemerkte Werte, Verlaufsschritt „Bild filtern“; vorher ⚠️ PM-6, 12.10., echtes Fenster (Brücke)) | [Bild](screenshots/paritaet/pm6_D41_farbton_saettigung.png) |
| D42 | Filter ▸ Farbe anpassen ▸ Helligkeit/Kontrast… (B1) | 408 × 144; Schieber −100…100 mit Feld rechts, Fokus Schieber; Tab Schieber › Feld › Schieber › Feld › Vorschau › OK › Abbrechen; Enter (20/−30) → „Bild filtern“, Strg+Z ✅; **gemerkt** | ≈ 332 × 201; gleiche Felder, Feld über dem Schieber; Enter → Schritt „Helligkeit/Kontrast“, Strg+Z ✅, Esc ✅; ~~**nicht gemerkt**~~ (DLG-5: gemerkt) | ⏳ umgesetzt, Nachprüfung ausstehend (DLG-5 `5f06b0c`: Feld rechts neben dem Schieber, Knöpfe rechts, Beschriftungen, Startfokus, Dezimalpunkt, gemerkte Werte, Verlaufsschritt „Bild filtern“; vorher ⚠️ PM-6, 12.10., echtes Fenster (Brücke)) | [Bild](screenshots/paritaet/pm6_D42_helligkeit_kontrast.png) |
| D43 | Filter ▸ Farbe anpassen ▸ Kurven… (B1) | Titel „Kurven“ 390 × 356; Kanal, Kurve mit Verlaufsleisten links/unten, OK/Abbrechen/Auto, **3 Pipetten**, Vorschau; Eingabe/Ausgabe als Felder; Fokus OK; Enter → „Bild filtern“, Strg+Z ✅ | Titel „Gradationskurven“, ohne Pipetten und Verlaufsleisten, Eingabe/Ausgabe nur Text, zusätzlich „Zurücksetzen“; „Auto“ legt einen eigenen Schritt „Auto-Tonwertkorrektur“ an; Strg+Z ✅ (Neubau nach `re\19`, RE-KT) | umgesetzt, Nachprüfung ausstehend (KV-1 `0afa445`: Titel „Kurven“, Feld 208 × 208, Rampen, Eingabe/Ausgabe als Felder, Auto und 3 Pipetten im Dialog, kein „Zurücksetzen“; vorher ❌ PM-6, 12.10., echtes Fenster (Brücke)) | [Bild](screenshots/paritaet/pm6_D43_kurven.png) |
| D44 | Filter ▸ Farbe anpassen ▸ Tonwertkorrektur… (B1) | 399 × 292; Kanal, „Eingabetonwerte“ 0/1/255, Histogramm mit Reglern, „Ausgabetonwerte“ 0/255, OK/Abbrechen/Auto, **3 Pipetten**, Vorschau rechts; Fokus OK; Enter (20) → „Bild filtern“, Strg+Z ✅; nicht gemerkt | „Tonwertspreizung/Tonwertumfang“, ohne Pipetten, Knöpfe unten; Enter → Schritt „Tonwertkorrektur“, Strg+Z ✅; nicht gemerkt ✅ (Neubau nach `re\19`, RE-KT) | ❌ PM-6, 12.10., echtes Fenster (Brücke) | [Bild](screenshots/paritaet/pm6_D44_tonwertkorrektur.png) |
| D45 | Filter ▸ Rauschen ▸ Rauschen hinzufügen… (B1) | 303 × 117; „Menge:“ 50 ▾, Farbe, Vorschau; Fokus OK; Enter 20 → „Bild filtern“, Strg+Z ✅; **gemerkt** (20) | ≈ 334 × 120; „**Stärke:**“ 50, ▾ senkrecht ✅; Fokus Feld; Enter → Schritt „Rauschen hinzufügen“, Strg+Z ✅; ~~**nicht gemerkt**~~ (DLG-5: gemerkt) | ⏳ umgesetzt, Nachprüfung ausstehend (DLG-5 `5f06b0c`: Feld rechts neben dem Schieber, Knöpfe rechts, Beschriftungen, Startfokus, Dezimalpunkt, gemerkte Werte, Verlaufsschritt „Bild filtern“; vorher ⚠️ PM-6, 12.10., echtes Fenster (Brücke)) | [Bild](screenshots/paritaet/pm6_D45_rauschen.png) |
| D46 | Filter ▸ Scharfzeichnen ▸ Unscharf maskieren… (B1) | 381 × 192; „Scharfzeichnungsmenge“ 50 / Pixelradius 4.1 / Schwellenwert 0, Schieber 0…170 / 0…145 (4.1 → 41) / 0…172, Felder rechts; Tab Felder › Vorschau › Schieber › OK › Abbrechen; Enter (100/2) → „Bild filtern“, Strg+Z ✅; **gemerkt** | ≈ 332 × 247; „**Schärfe**“ mit %, Pixelradius „4,1“; Schieberlage wie im Original ✅ (50 ≈ 29 %, 4,1 ≈ 28 %); Felder über den Schiebern; Enter → Schritt „Unscharf maskieren“, Strg+Z ✅; ~~**nicht gemerkt**~~ (DLG-5: gemerkt) | ⏳ umgesetzt, Nachprüfung ausstehend (DLG-5 `5f06b0c`: Feld rechts neben dem Schieber, Knöpfe rechts, Beschriftungen, Startfokus, Dezimalpunkt, gemerkte Werte, Verlaufsschritt „Bild filtern“; vorher ⚠️ PM-6, 12.10., echtes Fenster (Brücke)) | [Bild](screenshots/paritaet/pm6_D46_unscharf_maskieren.png) |
| D47 | Filter ▸ Weichzeichnen ▸ Bewegungsunschärfe… (B1) | 303 × 127; Winkel 90 / „**Umfang**“ 10 je mit ▾, Fokus OK; Enter (90/20) → B1 wächst **nur nach unten** auf 320,200,420,319; Strg+Z ✅; **gemerkt** | ≈ 313 × 120; „**Abstand**“, ▾ ✅; Enter → B1 wächst **nach allen Seiten** auf 300,180,440,320; Strg+Z ✅; ~~**nicht gemerkt**~~ (DLG-5: gemerkt) | ❌ PM-6, 12.10., echtes Fenster (Brücke); gemerkte Werte, Verlaufsschritt „Bild filtern“ ⏳ umgesetzt, Nachprüfung ausstehend (DLG-5 `5f06b0c`) | [Bild](screenshots/paritaet/pm6_D47_bewegungsunschaerfe.png) |
| D48 | Filter ▸ Weichzeichnen ▸ Gaußscher Weichzeichner… (B1) | 405 × 120; „Weichzeichnungsradius“ 4.1 markiert, Schieber 0…145 (4.1 → 41, 2 → 20); Enter 2 → B1 315,195,425,305, „Bild filtern“, Strg+Z ✅; **gemerkt** | ≈ 332 × 155; „**Radius**“ 4,1, Schieberlage wie im Original ✅; Enter 2 → 315,195,425,305 ✅, Strg+Z ✅; ~~**nicht gemerkt**~~ (DLG-5: gemerkt) | ⏳ umgesetzt, Nachprüfung ausstehend (DLG-5 `5f06b0c`: Feld rechts neben dem Schieber, Knöpfe rechts, Beschriftungen, Startfokus, Dezimalpunkt, gemerkte Werte, Verlaufsschritt „Bild filtern“; vorher ⚠️ PM-6, 12.10., echtes Fenster (Brücke)) | [Bild](screenshots/paritaet/pm6_D48_gauss.png) |
| D49 | Filter ▸ Weichzeichnen ▸ Radiales Weichzeichnen… (B1) | 303 × 131; „**Menge**“ 30 / Qualität 20 mit ▾, Fokus OK; Enter (10/20) → B1 317,197,424,304; Strg+Z ✅; **gemerkt** | ≈ 313 × 120; „**Stärke**“, ▾ ✅; Enter (10/20) → B1 **299,179,441,321** (deutlich größer); Strg+Z ✅; ~~**nicht gemerkt**~~ (DLG-5: gemerkt) | ❌ PM-6, 12.10., echtes Fenster (Brücke); gemerkte Werte, Verlaufsschritt „Bild filtern“ ⏳ umgesetzt, Nachprüfung ausstehend (DLG-5 `5f06b0c`) | [Bild](screenshots/paritaet/pm6_D49_radial.png) |
| D50 | Filter ▸ Weichzeichnen ▸ Zentriertes Weichzeichnen… (B1) | 318 × 135; „**Anzahl**“ 30 / Qualität 20 mit ▾, Fokus OK; Enter (10/20) → „Bild filtern“, Grenzen unverändert; Strg+Z ✅; **gemerkt** | ≈ 313 × 120; „**Stärke**“, ▾ ✅; Enter → Schritt „Zoom-Weichzeichner“, Grenzen unverändert ✅; Strg+Z ✅; ~~**nicht gemerkt**~~ (DLG-5: gemerkt) | ⚠️ PM-6, 12.10., echtes Fenster (Brücke); gemerkte Werte, Verlaufsschritt „Bild filtern“ ⏳ umgesetzt, Nachprüfung ausstehend (DLG-5 `5f06b0c`) | [Bild](screenshots/paritaet/pm6_D50_zentriert.png) |

## 3. Bedienfeldmenüs (≡)

**Neu gemessen von PM-7, 12.10.:** Original mit echter Maus (Reiter und ≡ angeklickt, Vordergrundprüfung), Menü per `MN_GETHMENU` gelesen, Lage S2 (ein Rechteck), 1 Seite, 1 Status, keine Stile, keine Verlaufsschritte gewählt. Firepixel: `build_panel_menu` in derselben Lage (Sonde wie §1, HEAD `6882dac`), das Eigenschaften-≡ im echten Fenster über die Brücke. Das Export-Format der Testdokumente ist verschieden (Original PNG32, Firepixel GIF), die Palettenbefehle im Optimieren- und Farbfelder-Menü sind deshalb nicht vergleichbar. Das Original ist seit 04:30 unverändert; Firepixel hat sich in keinem Eintrag geändert.

| Bedienfeld | Original | Firepixel | | Bild |
|---|---|---|---|---|
| alle | letzter Eintrag „Hilfe“; Gruppe schließen über das Reiter-Kontextmenü | zusätzlich „Schließen“ und „Registerkartengruppe schließen“ im ≡-Menü (PM-7, 12.10.: unverändert) | ⚠️ | – |
| Optimieren | „Farbfeld-Rückmeldung anzeigen“ **✓**; „Progressives JPEG“/„JPEG-Ränder scharfzeichnen“ **grau** (Format kein JPEG) | Rückmeldung ohne ✓; JPEG-Einträge aktiv, „JPEG-Ränder scharfzeichnen ✓“ und „Nicht verwendete Farben entfernen ✓“ bei GIF (PM-7, 12.10.: unverändert) | ❌ | [Bild](screenshots/paritaet/b_P_menue_optimieren_original.png) |
| Verlauf | „Ausgewählte Schritte wiederholen“, „Schritte kopieren“, „Als Befehl speichern…“ **grau** ohne gewählte Schritte; „Verlauf löschen“ aktiv | alle aktiv (PM-7, 12.10.: unverändert) | ❌ | [Bild](screenshots/paritaet/b_P_menue_verlauf_original.png) |
| Seiten | bei 1 Seite: „Seite löschen“ **aktiv**; „Mit Masterseite verknüpfen“, „Masterseite zurücksetzen“ grau; „Nummerierung ✓“ | „Seite löschen“ grau; sonst gleich (PM-7, 12.10.: unverändert) | ⚠️ | [Bild](screenshots/paritaet/b_P_menue_seiten_original.png) |
| Status | bei 1 Status: „Status löschen“ **aktiv**, „In Status kopieren…“ **grau**, „Auf Status verteilen“ **aktiv**; ✓ bei Autom. zuschneiden/Nummerierung/Auto-Differenz | „Status löschen“ grau, „In Status kopieren…“ aktiv, „Auf Status verteilen“ grau (PM-7, 12.10.: unverändert) | ❌ | [Bild](screenshots/paritaet/b_P_menue_status_original.png) |
| Ebenen | 27 Einträge; „Maske bearbeiten“ **grau** ohne Maske | gleiche Einträge und Zustände, nur „Maske bearbeiten“ aktiv (PM-7, 12.10.: unverändert) | ⚠️ | [Bild](screenshots/paritaet/b_P_menue_ebenen_original.png) |
| Stile | ohne gewählten Stil **grau**: Stil duplizieren…, Stile löschen, Stil umbenennen…, Stil neu definieren, Abweichungen löschen, Verknüpfung mit Stil aufheben, Stiloptionen…, Stilbibliothek speichern | alle aktiv (PM-7, 12.10.: unverändert) | ❌ | [Bild](screenshots/paritaet/b_P_menue_stile_original.png) |
| Farbfelder | Einträge, ✓ Farbwürfel, „Aktuelle Exportpalette“ grau (PNG32) | gleich (Exportpalette bei GIF aktiv) | ✅ | [Bild](screenshots/paritaet/b_P_menue_farbfelder_original.png) |
| Eigenschaften (neu, PM-7) | ≡ enthält nur „Hilfe“ | ≡ enthält das Inspektor-Optionsmenü: Als Stil speichern…, Alle ein, Alle aus, Zusatzmodule suchen…, Volle Höhe, Halbe Höhe, Schließen; „Hilfe“ fehlt (PM-7, 12.10.) | ⚠️ | – |
| Ausrichten, Farbpalette (Flash-Bedienfelder) | ein Klick auf ≡ öffnet kein Windows-Menü (PM-7: kein `#32768`) | Hilfe, Schließen, Registerkartengruppe schließen | ❔ | – |


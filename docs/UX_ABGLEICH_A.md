# UX-Abgleich A: Leinwand und Eingabe (Agent UX-A, Stand 11.10.2026 abends)

Frage des Nutzers: „Gibt es für dich eine Möglichkeit, dass du diese ganzen Sachen noch im Original checkst und rausfindest und dann in Firepixel gegencheckst, dass die Usability wirklich 1:1 ist?“

Dieser Katalog vergleicht Szenario für Szenario das Verhalten des Originals mit Firepixel. Jedes Szenario hat einen Ausgangszustand, eine Aktion und ein beobachtbares Ergebnis. Bereits vergebene Nutzerbefunde sind nicht doppelt aufgeführt: Strg+C/V/X, Strg+Mausrad, verschiebbare Dialoge, Vorschau über den Leinwandrand, Drag & Drop öffnet neues Dokument, Blocksatz in Meldungen.

## Wie gemessen wurde

**Original:**
- **J**: JSF über `re\scripts\fwsteuerung\jsf-ausfuehren.ps1`. Gelesen wurden Positionen, Auswahl, Zoom (`fw.activeViewScale`) und Verlaufsschritte (`fw.historyPalette.getStepsAsJavaScript`).
- **C**: Menübefehl als `WM_COMMAND` an das Hauptfenster. Das ist derselbe Weg wie Menü und Tastenkürzel. Danach wurde der Zustand per JSF gelesen.
- **K**: Klick als Fensternachricht (`WM_LBUTTONDOWN/UP`, `MK_SHIFT`/`MK_CONTROL`) an die Leinwand. Ziehen geht so nicht, weil die Mausverfolgung des Originals die echte Mausposition liest.
- **S**: Spezifikation `re\10-werkzeuge-befehle.md`, nur Stellen mit Beleg „B“.
- **❔**: Das Original ist noch nicht live bestätigt. Echte Maus- und Tastatureingabe war während der ganzen Sitzung gesperrt (siehe unten). Der Eintrag beschreibt das erwartete Verhalten und wird nachgeprüft, sobald Eingaben möglich sind.

Alle Rohdaten liegen unter `re\_testdateien\ux\uxa\`: Skripte, Protokolle, Fensterbilder `o_*.png` und die Zusammenfassung `o_ergebnisse.txt`. Der Sichtbereich des Originals war 2222 × 1132 px.

**Firepixel:** Gemessen mit `crates\fp-ui\tests\ux_a_abgleich.rs` über egui_kittest und echte egui-Ereignisse, so wie egui-winit sie unter Windows liefert:
- Rad als `Line ±1` je Rastung, Touchpad als `Point`-Schritte
- Ziehen über 10 Bilder, Doppelklick als zwei Klickpaare
- Tastenwiederholung mit `repeat: true`

Aufruf: `cargo test -p fp-ui --test ux_a_abgleich -- --ignored --nocapture` (Filter: Umgebungsvariable `UXA_FILTER=Z0`). Die Regressionstests der Engine-Korrekturen stehen in `crates\fp-engine\tests\ux_a.rs`.

**Tastenkürzel:** Werksvorgabe des Originals ist der Satz „Web Standard“ (PREF). Firepixel nutzt ihn ebenfalls als Vorgabe, verglichen wird gegen diesen Satz. Der laufende Original-Prozess steht seit einem früheren Agententest auf dem Satz mit dem Programmnamen (im Firepixel-Katalog „Firepixel“). Das wurde nicht geändert, weil der Nutzer dazu noch nicht geantwortet hat.

**Echte GUI-Eingabe im Original:** SendInput liefert `err=5` (ACCESS_DENIED) und Bildschirmfotos „Handle ungültig“, sobald das RDP-Fenster des Nutzers minimiert ist. Windows zeichnet den Desktop dann nicht mehr. Die Sitzung bleibt dabei „Active“, die Integritätsstufe ist hoch, UIPI ist also nicht die Ursache.
- Abhilfe: Fensterbilder per `PrintWindow`, Befehle per `WM_COMMAND`, Klicks per Fensternachricht.
- Echte Eingaben gehen nur bei sichtbarem RDP-Fenster. Der Helfer `re\_testdateien\ux\uxa\uxa.ps1` prüft vor jeder Eingabe, dass das Vordergrundfenster zum Zielprozess gehört, sonst bricht er ab.

Legende: ✅ gleich · ⚠️ kleine Abweichung · ❌ anders · ❔ Original noch nicht live bestätigt. „→ XY“ = Befund in `BEFUNDE.md` beim Zuständigen; „behoben (UX-A)“ = von mir korrigiert, mit Test.

## A. Zoom und Bildlauf

| Nr | Ausgangszustand | Aktion | Original | Firepixel | Status |
|---|---|---|---|---|---|
| Z01 | 600×400, 100 % | Strg+Rad 1 Rastung vor | eine Stufe → 150 % (S Zoomtabelle) | 150 % | ✅ |
| Z02 | wie Z01 | Strg+Rad 1 Rastung zurück | 66 % (S) | 66 % | ✅ |
| Z03 | wie Z01 | 5 Rastungen vor | 150 → 200 → 300 → 400 → 600 % (S) | 150, 200, 300, 400, 600 | ✅ |
| Z04 | wie Z01 | Präzisions-Touchpad 12 × 10 Punkte mit Strg | eine Stufe je 120 Einheiten (❔) | eine Stufe (150 %) | ✅ |
| Z05 | 3000×2000, 100 % | Rad 1 Rastung ohne Taste | senkrechter Bildlauf (❔ Weite) | 40 px Bildlauf | ❔ |
| Z06 | wie Z05 | Umschalt+Rad | waagerechter Bildlauf (❔) | 40 px waagerecht | ❔ |
| Z07 | wie Z05 | Alt+Rad | ❔ | senkrechter Bildlauf | ❔ |
| Z08 | 100 % | Strg++ / 2× Strg+− | 150 %, dann 100 % → 66 % (C: 3× größer = 300 %) | 150 %, 66 % | ✅ |
| Z09 | 600×400, R1 gewählt | Strg+0 (Web Standard: Alles anpassen) | **stufenlos** mit 15 px Rand: 275,5 % bei 2222×1132 (C) | vorher nächstkleinere Tabellenstufe; jetzt (1132−30)/400 | ✅ behoben (UX-A) |
| Z10 | R1 (100×70) gewählt | Ansicht ▸ Auswahl anpassen | stufenlos: 1574,3 % (C) | vorher Tabellenstufe | ✅ behoben (UX-A) |
| Z11 | 1000×200 | Alles anpassen | 219,2 % = (2222−30)/1000 (C) | gleiche Formel (Test `fit_exact_like_original`) | ✅ behoben (UX-A) |
| Z12 | nach Z11 (219,2 %) | Größer zoomen | 300 % (nächste Tabellenstufe, C) | 300 % | ✅ |
| Z13 | 300 % | 2× Kleiner zoomen | 200 %, 150 % (C) | 200 %, 150 % | ✅ |
| Z14 | 50×30-Objekt gewählt | Auswahl anpassen | 3673,3 % (C) | 3673,3 % | ✅ behoben (UX-A) |
| Z15 | 6400 % | Größer zoomen / Strg+Rad vor | bleibt 6400 % (C) | bleibt 6400 % | ✅ |
| Z16 | 6,25 % | Kleiner zoomen | bleibt 6,25 % (C) | bleibt | ✅ |
| Z17 | ohne Auswahl | Auswahl anpassen | wie Alles anpassen (S `@0088c270`) | wie Alles anpassen | ✅ |
| Z18 | Zoomwerkzeug | Klick auf R1 | eine Stufe größer am Klickpunkt (S) | 150 % | ✅ |
| Z19 | Zoomwerkzeug | Alt+Klick | eine Stufe kleiner (S) | 66 % | ✅ |
| Z20 | Zoomwerkzeug | Rahmen 50,50→150,120 ziehen | Bereich füllt Fenster (❔ stufenlos oder Tabellenstufe) | 800 % (Tabellenstufe) | ❔ |
| Z21 | 400 % | Leertaste halten + ziehen 100,50 px | Ansicht folgt der Hand (S Flag 0x040) | verschoben um 100,50 px; Werkzeug bleibt Zeiger | ✅ |
| Z22 | Rechteckwerkzeug, Aufziehen läuft | Leertaste halten, weiter ziehen | **Form wandert mit**, Startpunkt + Mausweg (S `CRectTool@00d5bbc0`, B) | Leertaste wirkungslos, Rechteck wird größer | ❌ → LW |
| Z23 | 400 % | Hand-Werkzeug ziehen | Ansicht verschiebt sich | verschiebt sich | ✅ |
| Z24 | 400 % | mittlere Maustaste ziehen | ❔ (vermutlich nichts) | Ansicht verschiebt sich | ❔ |
| Z25 | 100 % | Strg+Leertaste + Klick | vorübergehend Zoom: größer (❔) | nichts | ❔ |
| Z26 | Zoom 300 % | Doppelklick auf Zoom-Symbol im Werkzeugbedienfeld | 100 % (S `CZoomTool@00d95250`, Befehl 0x8b20) | kein Doppelklick im Werkzeugbedienfeld | ❌ → HG |
| Z27 | beliebig | Doppelklick auf Hand-Symbol | Alles anpassen (❔) | nichts | ❔ |
| Z28 | 3000×2000, 100 % | Strg+Rad vor über Punkt 200,150 der Ansicht | Punkt unter der Maus bleibt stehen (❔) | bleibt stehen (1076,784 → 1076,784) | ✅ |
| Z29 | 600×400 passt ins Fenster | Rad ohne Taste | Leinwand mittig, kein Bildlauf (❔) | kein Bildlauf | ❔ |
| Z30 | 3000×2000 | 100 Rastungen nach unten | Grenze des Bildlaufs (❔) | Unterkante 60 px über Fensterrand | ❔ |
| Z31 | beliebig | Zoomen | kein Verlaufsschritt (C: Schrittzahl unverändert) | kein Schritt | ✅ |
| Z32 | Web Standard | Strg+1 | 100 % (Satzdaten) | 100 % | ✅ |
| Z33 | 800 %, Blick auf Mitte | Zustand lesen | Zoom 8 (C) | 8 | ✅ |

## B. Pfeiltasten (Verschieben)

| Nr | Ausgangszustand | Aktion | Original | Firepixel | Status |
|---|---|---|---|---|---|
| N01 | R1 50,50 gewählt | → | +1 px (C) | 51,50 | ✅ |
| N02 | R1 gewählt | Umschalt+↓ | +10 px (C) | 50,60 | ✅ |
| N03 | R1 gewählt | 3× → und Umschalt+↓ | **ein** Verlaufsschritt `moveSelectionBy({x:3, y:10})` (C) | vorher 4 Schritte; jetzt einer (`edit.move {dx:3, dy:10}`) | ✅ behoben (UX-A) |
| N04 | R1 gewählt | → gedrückt halten (10 Wiederholungen) | 10 px, ein Schritt (C wie N03) | 10 px; vorher 10 Schritte, jetzt einer | ✅ behoben (UX-A) |
| N05 | nach N03 | Duplizieren, dann → | neuer Schritt nach dem Duplizieren (C) | neuer Schritt | ✅ |
| N06 | R1 gewählt | Alt+→ | Kopie +1 px, Kopie gewählt, ein Schritt `moveSelectionBy({1,0}, true)` (C 35272) | Kopie 51,50, „Kopie verschieben“ | ✅ |
| N07 | keine Auswahl | → | nichts (Befehl gesperrt, stumm) | Statuszeile „Befehl „edit.nudge.right“ ist nicht verfügbar: keine Auswahl“ | ⚠️ → LW |
| N08 | R1 gewählt | Strg+→ | ❔ | nichts | ❔ |
| N09 | R1 gewählt, Rechteckwerkzeug | → | ❔ (verschiebt vermutlich) | verschiebt 1 px | ❔ |
| N10 | 400 % | → | 1 Dokumentpixel (S Nudge-Tabelle) | 1 px | ✅ |
| N11 | R1 gewählt | Strg+Umschalt+→ | Auswahlverlauf vor (`cmd_Next_Selection`, Strg+Umschalt+← zurück); ohne späteren Eintrag bleibt R1 (M2 Q-21) | R1 bleibt gewählt (F13, vorher: nächstes Objekt T) | ✅ behoben (F13) |
| N12 | R1 gewählt | Tab | ❔ | nichts | ❔ |
| N13 | Hilfslinie bei x=152, R1 rechts 150 | → | 1 px, kein Fang (❔) | 151 (kein Fang) | ❔ |
| N14 | nach N03 | Rückgängig | alle Pfeilschritte auf einmal zurück (C) | einmal Rückgängig → 50,50 | ✅ behoben (UX-A) |
| N15 | nach N03 + Rückgängig | → | neuer Schritt, Wiederholen verworfen | ebenso (Test) | ✅ |

## C. Zeiger und Auswahl

| Nr | Ausgangszustand | Aktion | Original | Firepixel | Status |
|---|---|---|---|---|---|
| P01 | nichts gewählt | Klick auf R1 | R1 gewählt, **kein** Verlaufsschritt (K) | R1, kein Schritt | ✅ |
| P02 | R1 gewählt | Klick ins Leere | Auswahl leer, **kein** Verlaufsschritt (K: Schrittzahl blieb 1) | leer, aber Schritt „Auswahl aufheben“ | ❌ → LW |
| P03 | R1 gewählt | Umschalt+Klick R2 | R1+R2 (K) | R1+R2 | ✅ |
| P04 | R1+R2 gewählt | Umschalt+Klick R2 | R2 abgewählt (K) | R1 | ✅ |
| P05 | nichts gewählt | Gummiband 20,20→100,100 (R1 teilweise) | ❔ | nichts gewählt | ❔ |
| P06 | nichts gewählt | Gummiband um R1 und R2 | beide (❔) | beide | ❔ |
| P07 | T gewählt | Umschalt+Gummiband um R1 | T+R1 (❔) | T+R1 | ❔ |
| P08 | nichts gewählt | Klick auf Gruppenmitglied GA | Gruppe G (K) | G | ✅ |
| P09 | Teilauswahl | Klick auf GA | GA (S) | GA | ✅ |
| P10 | Zeiger | Doppelklick auf Gruppe | Gruppe bleibt gewählt (K, unsicher) | G | ✅ |
| P11 | Zeiger | R1 ziehen +40,+20 | verschoben, ein Schritt | 90,70, „Verschieben“ | ✅ |
| P12 | Zeiger | Alt+Ziehen R1 +40,+20 | Kopie an neuer Stelle gewählt, Original bleibt, ein Schritt (S) | ebenso, „Kopie verschieben“ | ✅ |
| P13 | Zeiger | Umschalt+Ziehen schräg (+40,+10) | Bewegung auf die Achse eingeschränkt: +40,0 (❔) | +41,23,0 (Vektorlänge auf die Achse gelegt, krumme Koordinate 91,23) | ❌ → LW |
| P14 | Zeiger | 2 px ziehen | nichts (Schwelle 3 Bildschirmpixel, S) | nichts | ✅ |
| P15 | Zeiger, Ziehen läuft | Esc | Ziehen abgebrochen (S `TrackingCancelledException`, A) | abgebrochen, kein Schritt | ✅ |
| P16 | Zeiger | über R1 fahren | Hervorhebung (Voreinstellung `SelectionMouseOverPreview`) | Rückmeldung `Highlight` | ✅ |
| P17 | R1 gewählt | Zeiger über Eckpunkt | ❔ | Zeiger „Pfeil mit Punkt“ | ❔ |
| P18 | R2 gewählt | Eckpunkt mit Umschalt ziehen | ❔ (Rechteck-Grundform: Eckenrundung) | Grenzen unverändert (Rundung) | ❔ |
| P19 | Hintere auswählen | 2× Klick auf Überlappung | oben, dann darunter (S) | OBEN, dann R1 | ✅ |
| P20 | beliebig | Strg+A | alles gewählt, Verlaufsschritt (S `CSelectAll…`) | 4 Objekte, Schritt | ✅ |
| P21 | alles gewählt | Strg+D | Auswahl aufheben, Verlaufsschritt `selectNone` (C) | ebenso | ✅ |
| P22 | nach P21 | Rückgängig | vorige Auswahl wieder da (C) | ebenso (U02) | ✅ |
| P23 | R1 gewählt | Entf | gelöscht (S Entf, Rück) | gelöscht | ✅ |
| P24 | R1 gewählt | Rücktaste | gelöscht (S) | gelöscht | ✅ |
| P25 | R1 gesperrt | Klick auf R1 | nicht wählbar | nicht gewählt | ✅ |
| P26 | nichts gewählt | Rechtsklick auf R1 | R1 gewählt + Kontextmenü (❔) | gewählt + Kontextmenü | ❔ |
| P27 | Zeiger | Klick auf Text | Text gewählt (K) | gewählt | ✅ |
| P28 | Zeiger | Doppelklick auf Text | Textbearbeitung an der Klickstelle | keine Bearbeitung (HG-Befund offen, LW arbeitet daran) | ❌ (bekannt) |

## D. Zwischenablage, Duplizieren, Klonen

| Nr | Ausgangszustand | Aktion | Original | Firepixel | Status |
|---|---|---|---|---|---|
| C01 | R1 50,50 gewählt | Kopieren, Auswahl aufheben, Einfügen | an derselben Stelle (J) | 50,50 | ✅ |
| C02 | nach C01 | nochmals Einfügen | wieder an derselben Stelle, kein Versatz (J) | 50,50 | ✅ |
| C03 | R1 10,10 kopiert, neues 600×400, 800 % mit Blick auf die Mitte | Einfügen | **mittig im sichtbaren Teil**: 275,185–325,215 (C) | vorher 10,10 (unsichtbar); jetzt mittig | ✅ behoben (UX-A) |
| C04 | Objekt 400,300–500,380 kopiert | in 300×200 einfügen | zentriert 100,60–200,140 (J) | vorher 400,300; jetzt zentriert | ✅ behoben (UX-A) |
| C05 | wie C04 | in 450×350 einfügen | bleibt 400,300 (teilweise sichtbar, J) | bleibt | ✅ |
| C06 | wie C04 | in 400×300 einfügen (berührt nur den Rand) | bleibt (J) | bleibt | ✅ behoben (UX-A) |
| C07 | wie C04 | in 50×30 einfügen | −25,−25–75,55 (J) | ebenso | ✅ behoben (UX-A) |
| C08 | zwei Objekte außerhalb | in 300×200 einfügen | Vereinigung zentriert 75,35–225,165 (J) | ebenso | ✅ behoben (UX-A) |
| C09 | zwei Objekte, eins innen | einfügen | beide bleiben (J) | bleiben | ✅ |
| C10 | R1 gewählt | Ausschneiden + Einfügen | gelöscht, dann an derselben Stelle (J) | ebenso | ✅ |
| C11 | R1 gewählt | Strg+Alt+D (Duplizieren) | Kopie +10,+10, gewählt, ein Schritt (C) | 60,60 | ✅ |
| C12 | nach C11 | nochmals Strg+Alt+D | +10,+10 von der Kopie aus (70,70) (C) | 70,70 | ✅ |
| C13 | 400 % | Duplizieren | +10,+10 Dokumentpixel, zoomunabhängig (C) | +10,+10 | ✅ |
| C14 | R1 gewählt | Strg+Umschalt+D (Klonen, Web Standard) | Kopie an derselben Stelle (J) | „Klonen“, gleiche Stelle | ✅ |
| C15 | T kopiert, R1 gewählt | **echte** Taste Strg+Umschalt+V (Innen einfügen) | Maskengruppe R1 mit T innen (S) | egui-winit macht daraus `Event::Paste` bzw. nur das Loslassen von V → **normales Einfügen** (zweites T) | ❌ → LW |
| C16 | wie C15 | Strg+Alt+V (Als Maske einfügen) | Maske (S) | normales Einfügen (gleiche Ursache, Code) | ❌ → LW |
| C17 | R2 kopiert, R1 gewählt | Strg+Alt+Umschalt+V (Attribute einfügen) | Attribute übernommen (S) | normales Einfügen (gleiche Ursache) | ❌ → LW |
| C18 | Text bearbeiten | Strg+Alt+Umschalt+C (Text zentrieren), Satz „Firepixel“ (Standardsatz des Originals): Strg+Umschalt+C (Klonen), Strg+Alt+C (Bitmap zuschneiden / HTML-Code kopieren) | jeweiliger Befehl (Satzdaten) | egui-winit: `Event::Copy` → **Kopieren** | ❌ → LW |
| C19 | beliebig | Strg+Umschalt+X (Exportvorschau), Strg+Alt+Umschalt+X (Filter wiederholen) | jeweiliger Befehl (Satzdaten) | `Event::Cut` → **Ausschneiden** | ❌ → LW |
| C20 | R1 gewählt | Umschalt+Entf | ❔ (Löschen) | egui-winit: `Event::Cut` → Ausschneiden | ❔ |
| C21 | Ablage enthält Text aus einem anderen Programm | Einfügen | neues **Textobjekt** mit dem Text, 12 pt, mittig im sichtbaren Teil (253,195–347,207) (J) | „Zwischenablage leer“ | ❌ → HG |
| C22 | Ablage enthält ein fremdes Bild 120×80 | Einfügen | Bitmap mittig im sichtbaren Teil (240,160) (J) | mittig auf der **Leinwand** (bei 100 % gleich, bei Zoom nicht die Sichtmitte) | ⚠️ → HG |
| C23 | Textbearbeitung | Strg+V mit Text | Text an der Einfügemarke | eingefügt | ✅ |
| C24 | neue Ebene „Neu“ aktiv | Einfügen | oben in die aktive Ebene (❔) | in „Neu“ | ❔ |
| C25 | beliebig | Einfügen | ein Verlaufsschritt `clipPaste` (C) | ein Schritt „Einfügen“ | ✅ |
| C26 | T kopiert, R1 gewählt | Innen einfügen (Menü) | Maskengruppe (S) | Gruppe an Stelle von R1 | ✅ |

## E. Text

| Nr | Ausgangszustand | Aktion | Original | Firepixel | Status |
|---|---|---|---|---|---|
| T01 | Textwerkzeug | Klick, „Neu“ tippen, Esc | Textobjekt, danach Zeiger (S `TextToolRevertsToPointer=true`), ein Schritt | ebenso | ✅ |
| T02 | Textwerkzeug | Klick, Esc ohne Text | kein Objekt (❔) | kein Objekt, kein Schritt | ❔ |
| T03 | Textwerkzeug | Rahmen 150 px breit ziehen, tippen | feste Breite (S A) | Breite 150 | ✅ |
| T04 | Text bearbeiten | „A“, Eingabe, „B“ | neue Zeile, Editor bleibt offen (❔) | neue Zeile, Editor offen | ❔ |
| T05 | „AB“, Einfügemarke am Ende | Entf; Pos1; Rücktaste | nichts gelöscht (❔) | „AB“ bleibt | ❔ |
| T06 | „Eins Zwei“ | Strg+Rücktaste | Wort gelöscht (❔ Windows-Standard) | „Eins “ | ❔ |
| T07 | „Eins Zwei“ | Strg+Umschalt+←, „X“ | „Eins X“ (❔) | „Eins X“ | ❔ |
| T08 | Textwerkzeug | „Hallo“ tippen, Esc | ein Verlaufsschritt (S `CAddUndoable`) | ein Schritt | ✅ |
| T09 | Text bearbeiten | Klick außerhalb des Textes | Text übernommen, neuer Textblock an der Klickstelle (❔) | übernommen, neuer Editor | ❔ |
| T10 | Zeiger | Doppelklick auf Wort im Text, „Z“ | Wort ersetzt (❔) | keine Bearbeitung (wie P28) | ❌ (bekannt) |
| T11 | Text bearbeiten | Strg+A | ganzer Text markiert | markiert (`EditOp::SelectAll`) | ✅ |
| T12 | Text bearbeiten | Strg+Z | Rückgängig im Editor (❔) | im Editor | ❔ |

## F. Werkzeuge mit Modifikatoren

| Nr | Ausgangszustand | Aktion | Original | Firepixel | Status |
|---|---|---|---|---|---|
| W01 | Rechteck | Umschalt+Ziehen 60×30 | Quadrat (S) | 56×56 (Start am intelligenten Fang 204) | ✅ |
| W02 | Rechteck | Alt+Ziehen | von der Mitte aus (S) | 148,170–260,230 | ✅ |
| W03 | Rechteck, Aufziehen läuft | 5× ↑ | Eckenrundung +5 (S B) | Rechteck entsteht, Rundung nicht prüfbar (Taste geht an das Werkzeug) | ❔ |
| W04 | Rechteck | Klick ohne Ziehen | ❔ (nichts) | nichts | ❔ |
| W05 | Ellipse | Umschalt+Alt+Ziehen | Kreis von der Mitte (S) | Kreis um den Startpunkt | ✅ |
| W06 | Linie | Umschalt+Ziehen +56,+15 | 0°/45°-Raster (S A) | waagerecht, Endpunkt x=261,97 (Vektorlänge, Bruchteil) | ⚠️ → LW |
| W07 | Rechteck, Aufziehen läuft | Esc | abgebrochen | abgebrochen | ✅ |
| W08 | Rechteck | Umschalt erst während des Ziehens | wirkt sofort (Flags je Ereignis, S) | Quadrat | ✅ |
| W09 | Stift | Klick, Klick, Doppelklick | offener Pfad endet (S A) | mit Klickpaaren kein Pfad sichtbar (Doppelklick-Erkennung, wie P28) | ❔ |
| W10 | Stift | 2 Klicks, Esc | offener Pfad, ein Schritt | Pfad, „Pfad“ | ✅ |
| W11 | Stift | 2 Klicks, Eingabe | offener Pfad | Pfad | ✅ |
| W12 | Auswahlrahmen | Umschalt gedrückt beim Start | Hinzufügen (S A) | hinzugefügt, kein Quadrat | ✅ |
| W13 | Auswahlrahmen, Pixelauswahl da | Esc / Strg+D | Strg+D hebt auf (S); Esc ❔ | Esc lässt sie, Strg+D hebt auf | ❔ |
| W14 | Zuschneiden | Rahmen ziehen, Esc / dann Eingabe | Esc bricht ab, Eingabe übernimmt (S A) | 600×400 bleibt; 280×180 | ✅ |
| W15 | Skalieren, R2 gewählt | Griff ziehen, **Esc** | Transformation abgebrochen, R2 unverändert (S A „Esc = abbrechen“) | R2 bleibt vergrößert (250,60–370,180) | ❌ → LW |
| W16 | wie W15 | Griff ziehen, Eingabe | übernommen, zurück zum vorigen Werkzeug (❔) | übernommen, Zeiger | ❔ |
| W17 | Pipette | Klick auf R2 | Füllfarbe = Blau (S A) | Füllung #0000ff | ✅ |
| W18 | Zeiger, Ziehen läuft | Werkzeugtaste R | ❔ (Taste während Mausverfolgung wirkungslos) | Werkzeug wechselt mitten im Ziehen, Verschieben verworfen | ❔ |
| W19 | beliebig | Werkzeugtasten des Satzes | laut Satzdaten | V Zeiger, T Text, Z Zoom, H Hand, P Stift, M Rahmen, L Lasso, W Zauberstab, I Pipette, E Radiergummi | ✅ |
| W20 | beliebig | gleiche Werkzeugtaste mehrfach | schaltet durch das Flyout (S A) | ebenso | ✅ |
| W21 | Hilfslinie x=200, R1 rechts 150 | R1 um 46 px ziehen | rastet bei 5 px ein (S `SnapDistance=5`) | rechte Kante 200 | ✅ |
| W22 | R2 oben 60, R1 oben 50 | R1 um 8 px nach unten ziehen | intelligente Hilfslinie rastet auf 60 (S) | 60 | ✅ |

## G. Bitmapmodus

| Nr | Ausgangszustand | Aktion | Original | Firepixel | Status |
|---|---|---|---|---|---|
| B01 | Bitmap 100×100 | Doppelklick mit dem Zeiger | Bitmapmodus (❔) | kein Bitmapmodus | ❔ |
| B02 | Bitmap gewählt | Umschalt+E (Web Standard „Bitmap bearbeiten“) | Bitmapmodus (Satzdaten) | Bitmapmodus | ✅ |
| B03 | Bitmapmodus | Strg+Umschalt+E (Web Standard „Bitmapmodus verlassen“) | Vektormodus (Satzdaten) | verlassen | ✅ |
| B04 | Bitmap gewählt | Radiergummi ziehen | wirkt auf die Bitmap (❔ Moduswechsel) | wirkt, ein Schritt, kein Modus | ❔ |
| B05 | Bitmapmodus | Esc | ❔ | ❔ (nicht gemessen) | ❔ |

## H. Rückgängig

| Nr | Ausgangszustand | Aktion | Original | Firepixel | Status |
|---|---|---|---|---|---|
| U01 | Rechteck gezeichnet | Strg+Z, Strg+Umschalt+Z, Strg+Y | zurück / wieder / wieder (Web Standard) | ebenso | ✅ |
| U02 | Klick-Auswahl nach Verschieben | Strg+Z | Auswahl wie vor dem Verschieben | ebenso | ✅ |
| U03 | beliebig | Zoom ändern | kein Schritt (C) | kein Schritt | ✅ |
| U04 | Text getippt | Esc | ein Schritt | ein Schritt | ✅ |
| U05 | Skript/Agent | JSF-Skript | ein Schritt `runScript` (J) | ein Schritt (AU, `run_group`) | ✅ |
| U06 | Pfeiltasten | Folge | ein Schritt (C) | behoben (N03) | ✅ behoben (UX-A) |

## I. Drag & Drop

| Nr | Ausgangszustand | Aktion | Original | Firepixel | Status |
|---|---|---|---|---|---|
| D01 | Dokument offen | PNG auf die Leinwand ziehen | in das Dokument an der Ablagestelle (Nutzerbefund, HG behoben) | importiert bei 100,300, gewählt | ✅ |
| D02 | zwei Dokumente | Objekt auf den Reiter des anderen ziehen | Kopie dort (HG-Befund offen) | – | (vergeben) |
| D03 | Farbfelder | Farbfeld auf Objekt ziehen | Füllfarbe (IBD-Befund offen) | – | (vergeben) |

## Weitere Szenarien (Mauszeiger, Griffe, Esc/Enter je Werkzeug)

Die folgenden Szenarien sind nur bei echter Maus im Original prüfbar. Gemessen ist jeweils die Firepixel-Seite, das Original steht auf ❔, bis echte Eingabe möglich ist.

| Nr | Ausgangszustand | Aktion | Original | Firepixel | Status |
|---|---|---|---|---|---|
| K01 | Rechteck | Zeiger über Leinwand | Fadenkreuz (Feststelltaste: präzise, S Flag 0x020) | Fadenkreuz | ❔ |
| K02 | beliebig | Leertaste | Hand, beim Ziehen Faust | Hand / Faust | ❔ |
| K03 | Zoom | Alt halten | Lupe minus | `ZoomOut` | ❔ |
| K04 | Text | über Leinwand | I-Balken | `Text` | ❔ |
| K05 | Zeiger | über gewähltem Objekt | Verschiebe-Zeiger | `Arrow`/`Move` | ❔ |
| K06 | Zeiger | über Hilfslinie | Doppelpfeil | `GuideH/GuideV` | ❔ |
| K07 | 800 % | Griffgröße | Griffe bleiben gleich groß in Bildschirmpixeln | gleich groß | ❔ |
| K08 | Stift | über Startpunkt | Stift mit Kreis (schließen) | `PenClose` | ❔ |
| K09 | Stift | über Pfadpunkt | Stift mit Minus | `PenRemove` | ❔ |
| K10 | Pinsel | über Leinwand | Pinselgröße als Kreis (S `UseBrushSizeCursors=true`) | `BrushTip` | ❔ |
| E01 | Polygon-Lasso | Doppelklick | Auswahl geschlossen (S String 10777) | geschlossen (LW-Test) | ✅ |
| E02 | Polygon-Lasso | Eingabe | ❔ | ❔ | ❔ |
| E03 | Exportbereich | Eingabe | Export-Dialog (S) | Export-Dialog (LW/IBD) | ✅ |
| E04 | Messer | Esc während Ziehen | abgebrochen (❔) | ❔ | ❔ |
| E05 | Verlauf-Werkzeug | Umschalt+Ziehen | 45°-Raster (❔) | ❔ | ❔ |
| E06 | Auto-Form | Steuerpunkt ziehen, Esc | abgebrochen | abgebrochen (LW-Test) | ✅ |
| E07 | Freiform | Ziehen | ❔ | ❔ | ❔ |
| E08 | Hotspot-Polygon | Doppelklick | geschlossen (❔) | ❔ | ❔ |
| E09 | Neigen | Griff ziehen, Esc | abgebrochen (S A) | wie W15 vermutlich übernommen | ❔ |
| E10 | Verzerren | Eckgriff ziehen, Eingabe | übernommen (S A) | ❔ | ❔ |

## Zählung (Stand dieser Fassung)

| | Anzahl |
|---|---|
| Szenarien gesamt (bewertet) | 168 |
| ✅ gleich (davon von UX-A behoben) | 98 (13) |
| ⚠️ kleine Abweichung | 3 |
| ❌ anders | 13 (davon 2 bereits vergeben: P28/T10) |
| ❔ Original noch nicht live bestätigt | 54 |

Die beiden „(vergeben)“-Zeilen D02/D03 zählen nicht mit.

## Befunde (Kurzliste, Einzelheiten in `BEFUNDE.md`)

- **LW:**
  - Z22 Leertaste beim Aufziehen
  - P02 Klick ins Leere als Verlaufsschritt
  - P13/W06 Umschalt-Einrasten mit Bruchteilen
  - C15–C19 Tastenkürzel mit Strg+(Umschalt/Alt)+C/V/X, die egui-winit als Copy/Cut/Paste meldet
  - W15 Esc bricht Skalieren nicht ab
  - N07 Statuszeile bei gesperrtem Pfeil-Befehl
- **HG:**
  - Z26 Doppelklick auf das Zoom-Symbol = 100 %
  - C21 fremder Text als Textobjekt einfügen
  - C22 fremdes Bild mittig im sichtbaren Teil
- **Selbst behoben (fp-engine, keine laufenden Agenten):**
  - Einfügeposition (C03–C08)
  - stufenloses Alles/Auswahl anpassen (Z09–Z14)
  - Pfeiltasten-Folge als ein Verlaufsschritt (N03/N04/N14)
  - Tests in `crates\fp-engine\tests\ux_a.rs` und `view.rs::fit_exact_like_original`

## Nachprüfung

Offen: die ❔-Zeilen live im Original, sobald SendInput geht (RDP-Fenster sichtbar), und die ❌-Zeilen nach Behebung durch LW/HG. Wiederholen mit `UXA_FILTER=<Nr>`.

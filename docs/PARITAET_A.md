# Funktionsparität A: Werkzeuge, Leinwand, Inspektor (Agent UX-A, Nacht 11./12.10.2026)

Auftrag: „Vergleich mal alle Funktionen vom Original und Firepixel, damit sie wirklich 1:1 sind.“ Für die Nacht hat der Nutzer echte Maus- und Tastatureingabe freigegeben.

## Übergabe (Stand 12.10.2026, Arbeit auf Nutzerwunsch angehalten)

**Zahlen je Bereich** (117 Zeilen, neu gezählt von PM-1 am 12.10. nach dem Stand der Tabellen inkl. FV-1/FV-3-Nachmessungen; ✅ gleich, ⚠️ Abweichung im Detail, ❌ fehlt bzw. falsch, ⏳ nicht fertig gemessen):

| Bereich | ✅ | ⚠️ | ❌ | ⏳ |
|---|---|---|---|---|
| 1 Inspektor je Werkzeug (W1–W38) | 9 | 10 | 19 | 0 |
| 2 Feldverhalten (F1–F15) | 9 | 6 | 0 | 0 |
| 3 Text: Schriftgröße (T1–T6) | 3 | 0 | 3 | 0 |
| 4 Inspektor je Objektart (O1–O17) | 6 | 4 | 7 | 0 |
| 5 Leinwand/Werkzeugverhalten (L1–L30) | 18 | 5 | 7 | 0 |
| 6 Mauszeiger (K1–K11) | 9 | 2 | 0 | 0 |
| **Summe** | **54** | **27** | **36** | **0** |

**Geprüft sind:**
- der Werkzeug-Inspektor ohne Auswahl für alle Werkzeuge der Werkzeugleiste, Vorgaben gegen die Werkseinstellungen;
- das Feld „Deckkraft“ vollständig (Tippen, Klemmen, Pfeiltasten, Rad, Popup-Schieber, Esc/Tab, Rückgängig);
- die Schriftgröße beim Textobjekt und bei markiertem Text;
- der Inspektor für Rechteck, Pfad, Text, Bitmap, Gruppe, Symbol, Hotspot und Mehrfachauswahl;
- Zeichnen mit Umschalt/Alt, Esc/Enter, Pfeiltasten, Mausrad, Zoomrahmen, Zauberstab, Bitmapmodus, Zuschneiden und Eckgriffe mit echter Maus;
- die Mauszeiger je Werkzeug.

**Befunde und Verbleib** (alle Einträge in `docs\BEFUNDE.md`, Abschnitt UX-A; beim Anhalten war keiner als behoben gemeldet):
- **LW (Leinwand/Werkzeuge), offen:**
  - Werkzeugvorgaben aus den Voreinstellungen;
  - Textwerkzeug: Schrift und Farbe; Zeilenabstand 120 % und 2 px Rand; Textlage und Werkzeug nach Esc;
  - Gummiband „berührt“; Umschalt während des Ziehens;
  - **hoch:** Rechteck mit Umschalt/Alt und Fang am Startpunkt;
  - Pfeiltasten/Esc beim Aufziehen;
  - **hoch:** Stift-Doppelklick verwirft den Pfad;
  - **hoch:** Eckgriff am Rechteck ändert nichts;
  - Strg+Leertaste-Zoom; Zoomrahmen auf Tabellenstufe; Mausrad ohne Taste;
  - Esc bei Zauberstab bzw. im Bitmapmodus; Doppelklick auf Bitmap; Zuschneiden + Esc;
  - Segmente/Hotspots unsichtbar;
  - Mauszeiger.
- **I18N-INSPEKTOR („Funktion, nicht Text“), offen:**
  - Masken-Teilfeld; Zustand der Pinselform; Text-Zeile 2 grau;
  - Knopf „ab/cd“ fehlt; Pinselstrich-Feld im Text-Inspektor;
  - Strichposition im Werkzeug-Inspektor;
  - Toleranz beim Verlauf an „Auswahl füllen“;
  - Popup-Schieber senkrecht; **hoch:** Esc im Popup hebt die Auswahl auf;
  - Ziffernfelder; gemischte Werte leer.
- **AU (Skript):** `addNewHotspot("slice")` erzeugt einen Hotspot; das JSF-Anlegen von Text übernimmt die Voreinstellungen nicht. Beide offen.
- **Phase 1 (`docs\UX_ABGLEICH_A.md`):**
  - von UX-A selbst behoben: Einfügeversatz, Einpassen mit 15 px Rand, Verschmelzen der Pfeiltasten im Verlauf;
  - HG C22 und Z26 erledigt;
  - die übrigen Phase-1-Befunde stehen dort mit Status.

**Als Nächstes prüfen, in dieser Reihenfolge:**
1. Erneut prüfen, sobald LW und I18N-INSPEKTOR ihre Befunde als behoben melden: zuerst die „hoch“-Punkte (Eckgriff, Stift-Doppelklick, Rechteck Umschalt/Alt, Esc im Popup).
2. ~~Feldprobe B, X, Rundung, Strich~~ – erledigt (PM-1, Abschnitt 2a, F12–F15).
3. ~~Offene ⏳-Zeilen L30, L8, W23, T6~~ – erledigt (PM-1); L16-Fangregel abgeleitet; L27 und K2/K3/K6–K9 im echten Fenster nachgemessen. Noch offen: Strich-Feld im Original mit Strich am Objekt für „12,5“, „x“, „0“.
4. Text: Kanten, Rücktaste, Live-Wirkung weiterer Felder während der Bearbeitung.
5. Inspektor für Schaltfläche, Animation und Segment in Firepixel.
6. Popup-Schieber der übrigen Felder (Kante, Textur, Strich).
7. Mauszeiger in XOR-Darstellung.

**So benutzt man die Hilfen.** Alles liegt dauerhaft unter `..\re\_testdateien\ux\uxa\werkzeug\`; JSF-Dateien in `jsf\`, Rohbilder in `bilder\`. In den Skripten steht noch der alte Scratchpad-Pfad in `$S`; auf diesen Ordner umstellen.
- `uxa2.ps1` ist die Eingabebibliothek: `Ziel`, `Klick`, `Ziehe`, `Druecke`, `Tippe`, `Rad`, `Foto`, `ZeigerBild`, `Loesen`, `FP-Dialoge-Schliessen`.
  - Vor **jeder** Eingabe prüft sie, ob Vordergrundfenster und Fenster unter dem Punkt (WindowFromPoint) zum Zielprozess gehören; sonst bricht sie ab.
  - `Tippe` arbeitet mit VkKeyScan, weil das Original Unicode-Eingabe ignoriert.
  - RDP darf nicht minimiert sein.
- `fpmcp.ps1`: `FP-Start` startet eine eigene Release-Instanz mit `--mcp`, eigenem APPDATA und Sprache de. `FP <werkzeug> @{…}` ruft die MCP-Brücke auf. Die Instanz am Ende beenden.
- `pi.ps1` liest den Inspektor des Originals per Fensternachricht. `paar.ps1` → `Paarbild` setzt zwei Aufnahmen übereinander, das Original oben.
- `sitzung.ps1 -Skript <x.ps1> -Seite o|f` wartet 60 s ohne Nutzereingabe, nimmt die gemeinsame Sperre (gui-sitzung) und räumt danach Tasten und Dialoge auf. Sitzungen dauern höchstens 5 Minuten.
- **Ablauf einer Messung:**
  1. Original: `jsf\uxp_aktiv.jsf` holt das eigene Testdokument nach vorn, `jsf\uxp_objekte.jsf` baut die Ausgangslage.
  2. Firepixel: `FP-Start`, dann `fp_objekte.ps1` (gleiche Objekte).
  3. `uxp_vN.ps1 -Seite o`, danach `-Seite f`, jeweils über `sitzung.ps1`.
  4. Die Ausgaben vergleichen, `Paarbild` nach `docs\screenshots\paritaet\a_<nr>_<name>.png`, die Zeile hier eintragen und den Befund in BEFUNDE.md.
- Keine Befehle, die Datei-Dialoge öffnen. Den Prozess des Originals nie beenden.
- **Hinweise PM-1 (12.10.):** Die Skripte zeigen jetzt auf den Werkzeugordner selbst (`$S`), `fpmcp.ps1` startet `target-pm1\release\firepixel.exe` (sonst `$global:FPEXE` setzen). `fp_objekte.ps1` liest die JS-Datei mit `[IO.File]::ReadAllText`: Mit `Get-Content -Raw` hängt PowerShell 5.1 Zusatzeigenschaften an die Zeichenkette, und `ConvertTo-Json -Depth 30` lief dann endlos (die Sitzung hielt die Sperre > 10 min). `FP-Start` nicht innerhalb einer Pipeline (`sitzung.ps1 … | …`) aufrufen: Die gestartete exe erbt die Ausgabe und die Pipeline endet nie. Vor jeder Firepixel-Messung `ui_state` → `canvas_area` prüfen (unten 937 bei ausgeklapptem Inspektor); ein eingeklappter Inspektor verschiebt alle Feldpositionen. JSF `originalSides` liefert in Firepixel mittige, relative Werte – für Grenzen `pixelRect` lesen. Neue Skripte: `uxp_pm_l16.ps1` (Fang), `uxp_pm_2.ps1` (Umschalt-Ende, T6, L27), `uxp_pm_3.ps1` (L30), `uxp_pm_4.ps1` (Zeiger-Eckgriff, Strg+C/V, Strg+Rad), `uxp_pm_v7.ps1 -Felder …` (Feldprobe), `uxp_pm_f.ps1` (W23, L8, N02, N07, N13/N15, nur Firepixel).
- **Messdaten:** `..\re\_testdateien\ux\uxa\o_ergebnisse.txt` (Phase 1) und `werkzeug\pi_inventar.txt` (Inspektor-Steuerelemente). Belegbilder: `docs\screenshots\paritaet\a_*.png`.

## Noch nicht geprüft

- Werkzeuge mit Auswahl: der Inspektor nach dem Zeichnen für jedes einzelne Werkzeug (bisher nur stichprobenartig).
- Felder außer Deckkraft und Schriftgröße im vollen Feldprobenumfang: B, H, X, Y, Rundung, Strichstärke, Kante, Textur, Laufweite, Zeilenabstand, Grundlinie, Einzug, Absatzabstände.
- Rückgängig-Granularität je Feld (außer Deckkraft).
- Skalieren, Verzerren und Drehen mit Modifikatoren; Ebenen-Interaktion auf der Leinwand.
- Inspektor für Schaltfläche, Animationssymbol, Segment und Instanz mit Eigenschaften.
- Stift- und Unterauswahl-Feinheiten: Kurvenpunkte, Alt-Ziehen an Griffen, Pfad fortsetzen.
- Bitmapwerkzeuge im Detail: Pinsel, Radierer, Weichzeichner, Stempel, Ersetzen.
- Text: Kanten, Rücktaste und Pfeiltasten im Text, Textfluss im Rahmen.

---

Dieser Katalog wird laufend ergänzt. Jede Zeile beschreibt ein Element mit dem Verhalten im Original und in Firepixel, dazu das Ergebnis und das Belegbild.

## Aufbau der Messung

- **Original:** echte Eingabe per SendInput. Vor jeder Eingabe prüft der Helfer, dass das Original vorn ist (§4d).
  - Werkzeuge werden per JSF gewählt (`fw.activeTool`). Das wirkt wie ein Klick im Werkzeugbedienfeld.
  - Feldwerte werden per Fensternachricht (`WM_GETTEXT`) aus dem Inspektor gelesen; das Original ist dafür nicht im Vordergrund nötig.
  - Das Original-Fenster ist maximiert (1800 × 1082). Die Inspektor-Felder liegen in beiden Programmen fast pixelgleich.
  - Einschränkung: Seit 11.10. abends scheitert das JSF-Anlegen neuer Dokumente im laufenden Original. Gemessen wird deshalb in einem vorhandenen eigenen Testdokument („uxp_akt“); Neustart erst nach Rücksprache mit dem Nutzer.
- **Firepixel:** eigene Instanz (Release aus `target-uxa`, eigenes APPDATA, Sprache Deutsch, `--mcp`) mit derselben echten Eingabe. Werte werden über die MCP-Brücke gelesen (`history`, `run_script`, `ui_state`).
- **Bilder:** `docs\screenshots\paritaet\a_<nr>_<name>.png`. Oben bzw. links steht das Original, unten bzw. rechts Firepixel.
- **Zustand des Originals:** Werkzeugvorgaben und Farben stammen aus seinen gespeicherten Voreinstellungen. Bei den Bitmap-Werkzeugen sind das genau die Werksvorgaben (PREF 1001, z. B. Abwedler 13/100). Strich- und Füllfarbe sowie Pinselart des Originals hat der Nutzer bzw. frühere Tests geändert. Verglichen wird deshalb vor allem der **Aufbau** (Felder, aktiv/grau, Einheiten, Abhängigkeiten) und die Werksvorgabe, wo sie belegt ist.

Legende: ✅ gleich · ⚠️ kleine Abweichung · ❌ anders · ⏳ wartet auf Neustart des Originals / noch nicht gemessen.

## 1. Inspektor je Werkzeug (ohne Auswahl)

| Nr | Werkzeug | Element | Original | Firepixel | | Bild |
|---|---|---|---|---|---|---|
| W1 | alle Bitmap- und Auswahlwerkzeuge (Rahmen, Lasso, Zauberstab, Pinsel, Bleistift, Radiergummi, Weich-/Scharfzeichner, Abwedler, Nachbelichter, Verwischen, Stempel, Farbe ersetzen, Rote Augen, Pipette, Füllwerkzeug, Verlauf) | 2. Zeile rechts | leer (Masken-Teilfeld 14119 erscheint nur bei Maskenbearbeitung) | zeigt immer grau „Maske: ○ Alphakanal ○ Graustufen“ | ❌ | a_109, a_111, a_113, a_116 |
| W2 | Radiergummi, Weichzeichner, Scharfzeichner, Abwedler, Nachbelichter, Verwischen, Farbe ersetzen | „Form“ rund/eckig | gewählte Form als gedrückter Knopf erkennbar (Rahmen) | beide Knöpfe gleich, Auswahl nicht sichtbar | ❌ | a_116, a_117, a_119, a_121 |
| W3 | Radiergummi | Größe / Rand | 13 / 2 (PREF `EraserToolDiameter=13`, `SoftEdge=20` → 2) | 10 / 0 | ❌ | a_116 |
| W4 | Weichzeichner, Scharfzeichner | Größe / Rand / Intensität | 13 / 100 / 50 (PREF) | 10 / 0 / 50 | ❌ | a_117, a_118 |
| W5 | Abwedler, Nachbelichter | Größe / Rand / Belichtung / Bereich | 13 / 100 / 50 / Mitteltonfarben | 10 / 0 / 50 / Mitteltonfarben | ❌ | a_119, a_120 |
| W6 | Verwischen | Größe / Rand / Druck / Farbe | 22 / 50 / 50 / Farbfeld zeigt Farbe | 10 / 0 / 50 / grau | ❌ | a_121 |
| W7 | Stempel | Größe / Rand | 16 / 50 (PREF `StampToolSize=16`, `Edge=500`) | 10 / 0 | ❌ | a_122 |
| W8 | Farbe ersetzen | Größe / Toleranz / Stärke | 13 / 32 / 200 | 10 / 70 / 50 | ❌ | a_123 |
| W9 | Rote Augen entfernen | Toleranz / Stärke | 50 / 50 | 70 / 50 | ❌ | a_124 |
| W10 | Rote Augen entfernen | Name im Kopf | zweizeilig „Rote Augen / entfernen“ | einzeilig gekürzt „Rote Augen en…“ | ⚠️ | a_124 |
| W11 | Freiform | Größe / Druck / Vorschau | 72 / ☑ / ☑ | 10 / ☐ / ☑ | ❌ | a_134 |
| W12 | Bereich umformen | Größe / Stärke / Druck Größe / Druck Stärke | 72 / 80 / ☑ / ☐ | 10 / 50 / ☐ / ☐ | ❌ | a_135 |
| W13 | Pfadausbesserung (additiv/subtraktiv) | Druck / Geschwindigkeit / Rate | ☑ / ☑ / 10 | ☐ / ☐ / 1 | ❌ | a_136, a_137 |
| W14 | Text | Schriftfamilie | Myriad Pro (Vorgabeschrift) | Arial | ❌ | a_133 |
| W15 | Text | Laufweite, Zeilenabstand (+Einheit), Absatzeinzug, Abstand vor/nach, Horizontalskala, Grundlinie, Anti-Alias, Autom. Unterschneidung | alle **bedienbar**, bevor man tippt (Vorgabe für den neuen Text) | alle grau | ❌ | a_133 |
| W16 | Text | Kurzzeichen Fett/Kursiv | „F“ / „K“ (deutsch) | „B“ / „I“ | ⚠️ | a_133 |
| W17 | Text | Knopf Textausrichtung (horizontal/vertikal, „ab/cd“) neben U | vorhanden | fehlt | ❌ | a_133 |
| W18 | Text | Textfarbe | eigenes Farbfeld des Textwerkzeugs (schwarz) | übernimmt die Füllfarbe (grau #999999) | ❌ | a_133, a_200 |
| W19 | Polygon | „Winkel“ und „Autom.“ bei Form = Polygon | bedienbar (Winkel 38, Autom. ☑) | grau | ⚠️ | a_131 |
| W20 | Linie, Stift, Vektorpfad, Rechteck, Ellipse, Polygon, Auto-Form, Pinsel | Strichposition innen/mitte/außen im Werkzeug-Inspektor | nicht vorhanden (nur am Objekt) | drei Knöpfe vorhanden | ❌ | a_125, a_126, a_129 |
| W21 | alle Vektorwerkzeuge, Füllwerkzeug, Verlauf | Textur bei „keiner“ | zeigt Texturnamen („Ader“) mit 0 % | „Ohne“ | ⚠️ | a_126 |
| W22 | Rechteck, Ellipse, Auto-Form, Rahmen, Lasso, Zuschneiden, Exportbereich | B/H/X/Y | Felder weiß/bedienbar (leer) | grau | ⚠️ | a_107, a_129 |
| W23 | Rechteck | „Eckrundung des Rechtecks“ im Werkzeug | Feld 14795 und Beschriftung 15079 existieren (Wert 0, Lage 536,1054 im Inspektor), sind aber **unsichtbar** (`IsWindowVisible` = 0); der Werkzeug-Inspektor zeigt keine Rundung (PM-1, frisch gestartetes Original) | 4. Zeile „Eckrundung des Rechtecks: 0“ sichtbar | ❌ | a_W23_rechteck_werkzeug_inspektor |
| W24 | Auswahlrahmen | Feste Breite/Höhe | Symbole ↔ ↕, Werte 0/0, grau bei „Normal“ | Beschriftung „B“/„H“, Werte 64/64 | ⚠️ | a_109 |
| W25 | Verlauf | „Auswahl füllen“ / Toleranz | ☑ / Toleranz grau (hängt davon ab) | ☐ / Toleranz bedienbar | ❌ | a_146 |
| W26 | Verlauf | Füllfarbfeld | zeigt den Verlauf | zeigt einfarbig grau | ⚠️ | a_146 |
| W27 | Auto-Form | Kopf | Name der gewählten Form („Abgefastes Rechteck“) | „Automatische …“ gekürzt | ⚠️ | a_132 |
| W28 | Pfadausbesserung, Polygon-Segment, Hotspot-Rechteck | Name im Kopf | zweizeilig voll | gekürzt mit „…“ | ⚠️ | a_136, a_139, a_143 |
| W29 | Zeiger (Dokument) | Exportformat-Liste | Format des Dokuments | „GIF Adaptiv 256“ (neues Dokument) und zusätzlicher Knopf „Bearbeiten…“ | ⚠️ | a_100 |
| W30 | Zauberstab | Toleranz 32, Rand Hart, Live-Auswahlrahmen ☑ | so | so | ✅ | a_113 |
| W31 | Pipette | Auswahl „1 Pixel“, Farbformat „HEX“ | so | so (außer W1) | ✅ | a_144 |
| W32 | Füllwerkzeug | Toleranz 32, Auswahl füllen ☐, Transparenz beibehalten ☐, Deckkraft 100, Normal | so | so (außer W1) | ✅ | a_145 |
| W33 | Hotspot-Rechteck/-Kreis/-Polygon | Form-Liste | so | so | ✅ | a_139–a_141 |
| W34 | Segment, Polygon-Segment | keine Felder | so | so | ✅ | a_142, a_143 |
| W35 | Bleistift | Anti-Alias, Autom. löschen, Transparenz beibehalten, Deckkraft, Mischmodus | so | so | ✅ | a_115 |
| W36 | Lasso, Polygon-Lasso | Rand, Live-Auswahlrahmen | so | so (außer W1, W22) | ✅ | a_111, a_112 |
| W37 | Vektorpfad | Genauigkeit 5 | so | so | ✅ | a_127 |
| W38 | Hand, Zoom, Zeiger, Teilauswahl, Hintere auswählen | Dokument-Inspektor | so | so (außer W29) | ✅ | a_100–a_102, a_147 |

## 2. Feldverhalten (Pfad gewählt, Feld „Deckkraft“)

Gemessen mit echter Eingabe: Klick ins Feld, Ende, Umschalt+Pos1, Tippen, Abschluss.

| Nr | Eingabe | Original | Firepixel | | Bild |
|---|---|---|---|---|---|
| F1 | „9999“ + Eingabe | 100 (geklemmt) | 100 | ✅ | |
| F2 | „-5“ + Eingabe | 5 (Feld nimmt nur Ziffern an, das Minus wird verworfen) | 0 | ⚠️ | |
| F3 | „12,5“ + Eingabe | 100 (Komma wird verworfen → 125 → 100) | 12,5 % (Bruchteil gespeichert) | ⚠️ | |
| F4 | „x“ + Eingabe | Buchstabe wird nicht angenommen, Wert bleibt | Wert bleibt, kein Verlaufsschritt | ✅ | |
| F5 | Pfeil hoch im Feld | keine Änderung | keine Änderung | ✅ | |
| F6 | Mausrad über dem Feld | keine Änderung | keine Änderung | ✅ | |
| F7 | „7“ + Esc, dann Klick auf die Leinwand | 7 wird übernommen (Esc verwirft nicht) | verworfen, Wert bleibt | ⚠️ | |
| F8 | „60“ + Tab | übernommen | übernommen | ✅ | |
| F9 | „40“, Klick auf die Leinwand | übernommen | übernommen | ✅ | |
| F10 | Strg+A im Feld | **Alles auswählen** (Objekte); das Feld markiert nichts | markiert den Feldtext | ⚠️ | |
| F11 | jede Übernahme | ein Verlaufsschritt | ein Schritt „Deckkraft“ | ✅ | |

### 2a. Feldprobe B, X, Rundung, Strich (PM-1, Rechteck R1 50,50–150,120, `uxp_pm_v7.ps1`)

Eingabe wie bei F1–F11 (Klick, Ende, Umschalt+Pos1, Tippen, Eingabe). Strich im Original aus UX-A übernommen (dort hatte R1 diesmal keinen Strich).

| Nr | Feld | Original | Firepixel | |
|---|---|---|---|---|
| F12 | B | „9999“ → 6000 (Feld zeigt schon beim Tippen 6000); „-5“ und „0“ werden **verworfen** (Breite bleibt 100); „12,5“ → **512** (Feldtext „512“, das Komma setzt die Schreibmarke an den Anfang); „x“ → Feld „0“, verworfen; Pfeil hoch und Rad: nichts | 9999 → 6000 ✅; „-5“ und „0“ → **1** (Meldung „auf 1 begrenzt“); „12,5“ → 13; „x“ bleibt; Pfeil und Rad: nichts ✅ | ⚠️ |
| F13 | X | „9999“ → 9999 und „-5“ → -5 (keine Grenze); „12,5“ → 512; „x“ → Feld „0“, Objekt nach **x = 0**; „0“ → 0; Pfeil und Rad: nichts | 9999 ✅, -5 ✅, 0 ✅, Pfeil/Rad ✅; „12,5“ → 12; „x“ bleibt 50 | ⚠️ |
| F14 | Rundung | „9999“ → 100 %; „-5“ → 5 % (Minus verworfen); „12,5“ → 100 % (Komma verworfen → 125); „0“ → 0; Pfeil und Rad: nichts | gleich | ✅ |
| F15 | Strichstärke | „9999“ → 100, „-5“ → 5 (UX-A); Pfeil und Rad: nichts | 9999 → 100, „-5“ → 5, „12,5“ → 100, „x“ bleibt, „0“ → 1; Pfeil/Rad nichts | ✅ |

## 3. Text: Schriftgröße und Bearbeitung

| Nr | Ablauf | Original | Firepixel | | Bild |
|---|---|---|---|---|---|
| T1 | Text tippen, Strg+A im Editor, Größe 48 + Eingabe | markierter Text wird sofort 48 pt | ebenso, Rahmen wächst mit | ✅ | a_200 |
| T2 | Textobjekt mit dem Zeiger gewählt, Größe 24 | ganzer Text 24 pt, Rahmen passt (69 × 31) | ganzer Text 24 pt, Rahmen passt (53 × 29, Arial) | ✅ | a_201 |
| T3 | Größe „200“ eintippen | angenommen (200), obwohl der Schieber nur bis 96 reicht | angenommen | ✅ | |
| T4 | Pfeil neben „Größe“ | **senkrechter** Popup-Schieber über dem Feld | waagerechter Schieber mit Zahl darunter | ❌ | o_t1_c / f_t2_d |
| T5 | Esc im geöffneten Größen-Schieber | schließt den Schieber, Auswahl bleibt | schließt den Schieber **und hebt die Auswahl auf** (Inspektor springt auf „Dokument“) | ❌ | f_t2_d → f_t2_e |
| T6 | Größe ändern bei Einfügemarke ohne markierten Text („Abc“, Größe 48 + Eingabe, „D“, Esc) | wirkt nur auf neu getippte Zeichen: Läufe [Abc 24 pt][D 48 pt]; Eingabe im Feld gibt den Fokus an den Text zurück; Werkzeug bleibt Text | ganzer Text 48 pt, ein Lauf [AbcD 48 pt] (PM-1) | ❌ | a_T6_groesse_einfuegemarke |

## 4. Inspektor je Objektart

Testdokument mit denselben Objekten in beiden Programmen (per JSF angelegt): R1, R2 (50 % Deckkraft), E1, T1, Gruppe G, Bitmap B, Segment S, Hotspot H, Symbol Sym. Im Original haben neue Objekte den aktuellen Strich (4 px „Weich abgerundet“), in Firepixel keinen; das ist ein Zustandsunterschied und kein Befund.

| Nr | Objekt | Element | Original | Firepixel | | Bild |
|---|---|---|---|---|---|---|
| O1 | Rechteck | Felder Füllung, Strich, Strichposition, Deckkraft, Filter, Rundung + Einheit, Pfadoperationen | so | so | ✅ | a_300 |
| O2 | Rechteck | Vorschaubild oben links | Objekt an seiner Lage im verkleinerten Dokument | Objekt formatfüllend | ⚠️ | a_300 |
| O3 | Ellipse (Pfad) | alle Felder inkl. „Pinselstrich bearbeiten“, Stil | so | so | ✅ | a_301 |
| O4 | Text | Zeilenabstand eines neuen Textes | 120 % | 100 % | ❌ | a_302 |
| O5 | Text | Grenzen eines Textrahmens 50,170–200,200 | X 48, Y 168, B 154, H 28 (2 px Rand um den Textbereich) | X 50, Y 170, B 150, H 24 | ❌ | a_302 |
| O6 | Text | Anti-Alias / Autom. Unterschneidung eines per Skript angelegten Textes (`addNewText(…, true)` = aus den Voreinstellungen) | „Anti-Alias glatt“ / ☐ | „Kein Anti-Alias“ / ☑ | ❌ | a_302 |
| O7 | Text | Pinselstrich-Feld (Stift + Farbe) in Zeile 2 | vorhanden | fehlt | ❌ | a_302 |
| O8 | Text | Horizontalskala | „100%“ | „100“ | ⚠️ | a_302 |
| O9 | Gruppe | „Gruppieren (2)“, Füllung/Strich/Deckkraft/Filter/Stil | so | so | ✅ | a_303 |
| O10 | Bitmap | Deckkraft, Mischmodus, Filter, Stil, B/H/X/Y | so | so | ✅ | a_304 |
| O11 | Segment (per Skript `addNewHotspot("slice", …)`) | Objektart | Segment (Typ Vordergrundbild, Exportformat, Hyperlink, Alt, Ziel; automatischer Name „_r2_c2“) | wird als **Hotspot** angelegt | ❌ | a_305 |
| O12 | Hotspot | Form, Farbe, Hyperlink, Alt, Ziel | so | so | ✅ | a_306 |
| O13 | Symbol | Deckkraft, Mischmodus, Filter, Stil | so | so | ✅ | a_307 |
| O14 | 2 Rechtecke, Deckkraft 100 und 50 | Deckkraft-Feld | **leer** (gemischte Werte) | „100“ | ❌ | a_308 |
| O15 | 2 Rechtecke | Füllfarbfeld | Farbe des zuletzt gewählten Objekts | Farbe des ersten | ⚠️ | a_308 |
| O16 | 2 Rechtecke | Stil-Liste | leer (gemischt) | „Kein Stil“ | ⚠️ | a_308 |
| O17 | Segmente und Hotspots | Überlagerung auf der Leinwand (grün/cyan) | sichtbar | unsichtbar (Webebene enthält H und S) | ❌ | a_309, a_330 |

## 5. Leinwand und Werkzeugverhalten (echte Maus und Tastatur)

| Nr | Ablauf | Original | Firepixel | | Bild |
|---|---|---|---|---|---|
| L1 | Gummiband 20,20→100,100 (R1 50–150 nur teilweise darin) | R1 gewählt (berührte Objekte zählen) | nichts gewählt | ❌ | |
| L2 | Gummiband um R1 und E1 | beide | beide | ✅ | |
| L3 | R1 ziehen, Umschalt **während** des Ziehens, +40/+10 | +40/0 (auf 0° eingerastet, ganzzahlig) | +40/+9 (Umschalt wirkungslos) | ❌ | |
| L4 | Alt-Ziehen R1 +40/+20 | Kopie bei +40/+20 | Kopie bei +40/+15 (rastet an etwas ein) | ⚠️ | |
| L5 | 3× Pfeil rechts, einmal Strg+Z | +3, ein Rückgängig stellt alles her | ebenso (von UX-A behoben) | ✅ | |
| L6 | Rechtsklick auf nicht gewählte Ellipse | Ellipse gewählt, Kontextmenü | ebenso (kittest) | ✅ | |
| L7 | Strg+Leertaste+Klick | vorübergehend Zoom: 100 → 150 % | nichts | ❌ | |
| L8 | Strg+Alt+Leertaste+Klick | verkleinert (150 → 100 %); live nicht messbar, weil die globale Taste der Claude-App die Kombination abfängt (§4d) | über die Eingabe-Einspeisung der Brücke (`ui_input`, keine echte Taste): 150 → 100 %; Gegenprobe Strg+Leertaste+Klick 100 → 150 % (PM-1) | ✅ | |
| L9 | Mausrad 1 Rastung ohne Taste (400 %) | senkrechter Bildlauf um 52 Bildschirmpixel | **kein Bildlauf** (echte Radereignisse; HG-Befund ist im aktuellen Stand noch offen) | ✅ nachgemessen (FV-3, echtes Fenster, SendInput 120 je Rastung, 400 %): 52 px senkrecht, 3 Rastungen 156 px | a_330, a_331 |
| L10 | Umschalt+Mausrad | nichts | waagerechter Bildlauf 40 px | ✅ nachgemessen (FV-3, echtes Fenster): kein Bildlauf | a_332 |
| L11 | Alt+Mausrad | senkrechter Bildlauf (wie ohne Taste) | senkrechter Bildlauf 40 px | ✅ nachgemessen (FV-3, echtes Fenster): 52 px senkrecht | a_333 |
| L12 | Zauberstab-Klick auf Bitmap | Bitmapmodus („Bitmap bearbeiten“ + „Zauberstab“ im Verlauf), Auswahlrahmen | Auswahlrahmen | ✅ | a_320 |
| L13 | danach **Esc** | „Bitmapmodus beenden“: Auswahlrahmen weg, Werkzeug wechselt zum Zeiger, Bitmap bleibt als Objekt gewählt | Auswahlrahmen bleibt, Werkzeug bleibt Zauberstab | ❌ | a_321 |
| L14 | danach Strg+D | Auswahl aufheben | Auswahl aufheben | ✅ | |
| L15 | Rechteck, Umschalt gehalten, Ziehen 240,145 → +60/+30 | Quadrat 60 × 60 ab dem Startpunkt | 250,150–300,175: kein Quadrat, Startpunkt um 10/5 px verschoben (rastet an E1-Mitte x=250 und R2-Oberkante y=150 ein) | ✅ nachgemessen (FV-3, echtes Fenster, Umschalt vor dem Drücken): 240,145–300,205 | |
| L16 | Rechteck, Alt gehalten | von der Mitte aus: 180–300 × 125–175 | wie ohne Alt (250,150–300,175) | ⚠️ **PM-1, Fangregel im Original (14 Fälle, `uxp_pm_l16.ps1`):** Start- und Endpunkt rasten getrennt in x und y an Kanten und Mitten anderer Objekte ein, Fangabstand **≤ 5** px (5 rastet, 6 nicht); mit **Umschalt rastet nichts** (weder Start noch Ende, auch mit Alt+Umschalt nicht); mit Alt rastet der Mittelpunkt wie sonst der Start, das Ende ebenfalls. Firepixel (echtes Fenster): bei genau 5 px kein Fang (115–175), mit Umschalt rastet es weiter (240,147 → 240,150; Ende 298 → 300), ein Ende bei y = 173 rastet an 170 statt an der näheren 175 (halber Pixel Versatz); Abstand 1–4 und ≥ 6 gleich. Befunde LW | |
| L17 | Rechteck, 5× Pfeil hoch während des Ziehens | Rundung bleibt 0 (in CS6 live keine Wirkung) | Rundung 0,145 | ⚠️ | |
| L18 | Rechteck, Leertaste während des Ziehens | Startpunkt wandert mit (Form verschoben) | Startpunkt wandert mit | ✅ | |
| L19 | Rechteck, Esc während des Ziehens | wird ignoriert, Rechteck entsteht beim Loslassen | Aufziehen abgebrochen, kein Rechteck | ⚠️ | |
| L20 | Rechteck, Klick ohne Ziehen | kein neues Objekt | kein neues Objekt | ✅ | |
| L21 | Textwerkzeug: Klick, Esc ohne Text | kein Objekt, Werkzeug bleibt Text | kein Objekt, Werkzeug wechselt zum Zeiger | ⚠️ | |
| L22 | Textwerkzeug: Klick bei 300,340, „Abc“, Esc | Text 300,323–339,341 (Klickpunkt ≈ Grundlinie), Werkzeug bleibt Text | Text 299,341–321,356 (Klickpunkt = Oberkante), Werkzeug wechselt zum Zeiger | ❌ | |
| L23 | Stift: 2 Klicks, Esc | offener Pfad 250,150–330,150 | ebenso | ✅ | |
| L24 | Stift: Klick, Klick, Doppelklick | offener Pfad mit 3 Punkten bis 330,220 | **kein Pfad** (nach dem Doppelklick ist nichts da) | ✅ nachgemessen (FV-3, echtes Fenster): offener Pfad 250,260–330,330, Werkzeug bleibt Stift | |
| L25 | Zoomwerkzeug: Rahmen 50,50–150,120 aufziehen | 1150 % (stufenlos eingepasst) | 800 % (Tabellenstufe) | ❌ | |
| L26 | Zeiger: Eckgriff eines gewählten Rechtecks um +40/+30 ziehen | Rechteck wird **größer** (bis 193,153), Rundung bleibt 0 | keine Änderung (weder Größe noch Rundung) | ✅ nachgemessen (FV-3, echtes Fenster): 50,50–190,150, Rundung 0 | |
| L27 | Zeiger: Doppelklick auf Bitmap | Bitmapmodus, Werkzeug wechselt zum Auswahlrahmen; danach Esc: Werkzeug bleibt Auswahlrahmen, Bitmap bleibt gewählt | Bitmapmodus und Auswahlrahmen (PM-1, echtes Fenster); danach Esc: Werkzeug wechselt zum Zeiger | ✅ (Esc danach ⚠️, Befund LW) | a_L27_doppelklick_bitmap |
| L28 | Zuschneiden: Rahmen ziehen, Esc | Leinwand unverändert, Werkzeug wechselt zum Zeiger | Leinwand unverändert, Werkzeug bleibt Zuschneiden | ✅ nachgemessen (FV-3, echtes Fenster): Werkzeug Zeiger, Breite 600 | |
| L29 | Zuschneiden: Rahmen 20,20–300,200, Eingabe, Strg+Z | 280 × 180, Rückgängig stellt 600 × 400 her | ebenso | ✅ | |
| L30 | Frei transformieren (Strg+T) an R1 50,50–150,120, Eckgriff von 151,121 nach 191,151 | **Ecke skaliert immer proportional** (50,50–193,150, größerer Faktor), Umschalt ändert nichts; Alt = proportional von der Mitte (7,20–193,150); Kante rechts → 191 (Zeigerlage); die Änderung wirkt schon beim Ziehen; **Esc behält die Änderung** und beendet den Modus (Werkzeug Zeiger); Eingabe und Doppelklick innen übernehmen, Strg+Z = ein Schritt; Ziehen am Mittelpunkt verschiebt nur den Drehpunkt (`uxp_pm_3.ps1`) | Ecke unproportional (190,150), nur mit Umschalt proportional (193,150); Alt 10,20–190,150 (Alt+Umschalt 7,20–193,150 gleich); Kante → 188; nach dem Loslassen zeigt die Leinwand bis zur Eingabe die alte Größe; Esc verwirft, Werkzeug bleibt „Skalieren“; Eingabe/Doppelklick, ein Schritt und Mittelpunkt gleich (PM-1) | ❌ | a_L30_frei_transformieren |

## 6. Mauszeiger (echte Maus, Bild `a_400_mauszeiger.png`)

Das Original zeichnet einige Zeiger invertiert (XOR); auf weißem Grund sind sie im Bild kaum zu sehen (Auswahlrahmen, Rechteck, Pinsel, Text). Diese Zeiger sind hier nicht bewertet.

| Nr | Lage | Original | Firepixel | |
|---|---|---|---|---|
| K1 | Zeiger über leerer Leinwand / über Objekt / im gewählten Objekt / mit Alt | schwarzer Pfeil | schwarzer Pfeil | ✅ |
| K2 | Zeiger über Eckgriff eines gewählten Rechtecks | Pfeil ohne Zusatz, **weiß mit schwarzem Rand** | schwarzer Pfeil ohne Zusatz (PM-1, echtes Fenster, `a_401_mauszeiger_nachmessung`) | ⚠️ |
| K3 | Zeiger über Kantengriff | Pfeil | Pfeil (PM-1, echtes Fenster) | ✅ |
| K4 | Teilauswahl über Objekt | weißer Pfeil | weißer Pfeil mit Zusatz | ✅ |
| K5 | Zoom / Zoom mit Alt | Lupe + / Lupe − | Lupe + / Lupe − | ✅ |
| K6 | Hand-Werkzeug bzw. Leertaste, ohne Maustaste | **offene** Hand | offene Hand, auch mit Leertaste (PM-1, echtes Fenster; Zeichnung etwas gröber) | ✅ |
| K7 | Stift über leerer Leinwand | Federspitze | Federspitze (PM-1, echtes Fenster) | ✅ |
| K8 | Füllwerkzeug über leerer Leinwand (kein Objekt) | Verbotszeichen ⊘ | ⊘ (PM-1, echtes Fenster) | ✅ |
| K9 | Zuschneiden | Zuschneide-Symbol: zwei kräftige Winkel (2 px) mit Diagonale | dünnes Symbol (1 px), ohne Diagonale, mit Kreuz in der Mitte (PM-1, echtes Fenster) | ⚠️ |
| K10 | Radiergummi über Bitmap | Kreis in Pinselgröße | Kreis in Pinselgröße mit Mittelkreuz | ✅ |
| K11 | Zauberstab, Pipette | Zauberstab / Pipette | Zauberstab / Pipette (eigene Form) | ✅ |

## Zählung (laufend)

| | Anzahl |
|---|---|
| geprüfte Elemente | 117 Zeilen (rund 300 einzelne Felder, Vorgaben, Abläufe bzw. Zeiger) |
| ✅ | 54 |
| ⚠️ | 27 |
| ❌ | 36 |
| ⏳ | 0 |

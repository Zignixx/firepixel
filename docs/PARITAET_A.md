# Funktionsparität A: Werkzeuge, Leinwand, Inspektor (Agent UX-A, Nacht 11./12.10.2026)

Auftrag: „Vergleich mal alle Funktionen vom Original und Firepixel, damit sie wirklich 1:1 sind.“ Für die Nacht hat der Nutzer echte Maus- und Tastatureingabe freigegeben.

## Übergabe (Stand 12.10.2026, Arbeit auf Nutzerwunsch angehalten)

**Zahlen je Bereich** (126 Zeilen; Abschnitt 5 von PM-3 am 12.10. in beiden Programmen nachgemessen und um L31–L39 ergänzt; neu gezählt von PM-1 am 12.10. nach dem Stand der Tabellen inkl. FV-1/FV-3-Nachmessungen; T, L und K nachgeführt von FV-6; Abschnitte 1–4 von PM-2 am 12.10. in beiden Programmen im echten Fenster nachgemessen, Firepixel-Release vom Stand `771bf08` + Arbeitsstand; W10/W14–W17/W27/W28, F7, F9, F10, O6 von FV-9 nachgeführt (F7/F9/F10/W14/W16/O6 im echten Fenster); Abschnitte 1 und 4 von PM-4 am 12.10. im echten Fenster nachgeprüft und aus den Zeilen neu gezählt; W21-Rest und F7-Nachtrag von FV-15 (Zahlen unverändert); ✅ gleich, ⚠️ Abweichung im Detail, ❌ fehlt bzw. falsch, ⏳ nicht fertig gemessen):

| Bereich | ✅ | ⚠️ | ❌ | ⏳ |
|---|---|---|---|---|
| 1 Inspektor je Werkzeug (W1–W38) | 36 | 1 | 1 | 0 |
| 2 Feldverhalten (F1–F15) | 15 | 0 | 0 | 0 |
| 3 Text: Schriftgröße (T1–T6) | 6 | 0 | 0 | 0 |
| 4 Inspektor je Objektart (O1–O17) | 16 | 0 | 1 | 0 |
| 5 Leinwand/Werkzeugverhalten (L1–L39) | 39 | 0 | 0 | 0 |
| 6 Mauszeiger (K1–K11) | 11 | 0 | 0 | 0 |
| **Summe** | **123** | **1** | **2** | **0** |

**PM-2 (12.10.):** Vorher in 1–4: 31 ✅ / 18 ⚠️ / 27 ❌, nachher 55 ✅ / 18 ⚠️ / 3 ❌. Neue Befunde in `BEFUNDE.md` (Kennung „von PM-2“): LW 1 (W8), IBD 14 (F10, F7, W14, W10/W27/W28, W16, W19, W21, W22, W24, W26, W29, O2, O8, O16), EB 1 (prefs.json W8). W15, W17 und O6 hängen an den offenen IBD-Befunden von EB-1, O11 am offenen AU-Befund. **F10 im Original neu gemessen:** Strg+A im Zahlenfeld markiert den Feldtext (die frühere Angabe „Alles auswählen“, nach der FV-1 umgebaut hat, ließ sich in 3 Läufen nicht bestätigen). Messhinweise: Firepixel zeichnet den Inspektor nach einem Werkzeugwechsel (auch per echtem Klick) erst nach 0,3–1,5 s neu; Fotos früher zeigen noch Werte des vorigen Werkzeugs – mindestens 1,5 s warten. Im Original übernimmt ein noch fokussiertes, geändertes Inspektorfeld seinen Text beim Klick auf ein anderes Objekt – vor jeder Feldprobe die Objekte neu anlegen. Skripte: `..\re\_testdateien\ux\uxa\werkzeug\pm2\` (`env.ps1`, `sitz.ps1`, `w.ps1`, `o.ps1`, `f.ps1`, `f2.ps1`, `x.ps1`, `rc.ps1`), Ablauf wie unten.

**PM-3 (12.10.):** Abschnitt 5 vorher 20 ✅ / 4 ⚠️ / 6 ❌ (L1–L30), nachher 27 ✅ / 3 ⚠️ / 0 ❌; neu L31–L39 (Stift, Unterauswahl, Pinsel, Radiergummi, Weichzeichner, Stempel): 3 ✅ / 4 ⚠️ / 2 ❌. Abschnitt 6 war schon vollständig ✅ (11/0/0), nichts nachzumessen. Neue Befunde in `BEFUNDE.md` (Kennung „von PM-3“): LW 10 (L4/L35, L7, L13, L25, L33, L34, L36, L37, L38, L39), AU 2 (JSF `setActiveViewScale` in der Brücke ohne Wirkung, `pixelRect` beim Text). Firepixel-Release vom Arbeitsstand auf `e8e69b6`. Skripte: `..\re\_testdateien\ux\uxa\werkzeug\pm3\` (`env.ps1`, `sitz.ps1`, `start.ps1`, `l.ps1 -Seite o|f -Teil 1–4`, `n.ps1` Nachmessungen). Messhinweise: Im Original haben neue Objekte je nach gespeichertem Zustand Strich und Deckkraft (am 12.10. vormittags 7 %, nach dem Neustart 100 % mit Strich) – für Pixelmessungen Deckkraft 100 und `setBrush(null)` setzen. Bei Segmenten auf der Webebene hellt das Original alles außerhalb der Segmente auf; die Bitmapmessungen löschen deshalb die Webebene. In Firepixel wirkt `fw.setActiveViewScale` über die Brücke nicht – zum Zurücksetzen `ui_action {"zoom": 1}` nehmen. Die Paarbilder L31–L34 stammen im Original noch aus dem Lauf mit 7 % Deckkraft.

**PM-4 (12.10.):** Nachprüfung im echten Firepixel-Fenster (P-PRUEF): W10, W15, W17, W19, W21, W22, W24, W26–W29, O2, O8, O16 mit echter Maus/Tastatur (eigene Debug-Instanz `11e454c`, eigenes APPDATA, unter der Sperre), L36 kurz über die Brücke. Alle ✅ außer **W28** (⚠️, „Pfadausbesserung - additiv“ bricht im Wort um). Zählung aus den Zeilen neu: Abschnitt 1 31/6/1 → 36/1/1, Abschnitt 4 13/3/1 → 16/0/1 (FV-9/FV-10 hatten die Zeilen schon auf ✅ gesetzt, die Tabelle aber nicht nachgeführt). Der Nebenbefund von FV-10 (Schachbrett im Vorschaubild weiß) tritt im echten Fenster nicht auf. Neue Befunde (Kennung „von PM-4“): IBD 4, AU 1. Bilder `pm4_*`, Skripte im Scratchpad `pm4\` (`genv.ps1`/`sitz.ps1` SendInput, `genvb.ps1`/`sitzb.ps1` Brücke). Messhinweis: `ui_find` liefert `hits` (nicht `matches`, so steht es noch in `fv10\f1.ps1`); Skripte mit Umlauten als UTF-8 **mit** BOM speichern, sonst liest PowerShell 5.1 sie als ANSI.

**FV-11 (12.10.):** L36–L39 (Pinsel, Radiergummi, Weichzeichner, Stempel) im Original mit eigenem Testdokument nachgemessen (Einzelabdrücke, Ziel des Pinsels bei Vektor-/Bitmapauswahl, Esc je Werkzeug) und in Firepixel behoben; Abschnitt 5 vorher 30 ✅ / 7 ⚠️ / 2 ❌, nachher 34 ✅ / 4 ⚠️ / 1 ❌. Nachweis im echten Fenster (eigene Instanz, echte Maus), Paarbilder `fv11_*`. Neu gefunden: das Original zeigt beim Malen keinen Auswahlrahmen um die Bitmap (behoben) und wählt mit dem Zeiger eine Bitmap auch per Klick auf ein transparentes Pixel (Befund LW).

**FV-14 (12.10.):** P-LW1: L4/L35 (Fang an Objektmitten beim Verschieben und beim Stift), L7 (Strg+Leertaste+Klick im selben Bild), L13 (Bitmap nach dem Zauberstab gewählt), L25 (Rand des Zoomrahmens: Sichtbereich minus 17 px), L33 (Alt-Ziehen am Eckpunkt nur Ausgangshenkel), dazu der FV-11-Befund (Klick auf ein durchsichtiges Bitmap-Pixel wählt die Bitmap) behoben (`8b03953`). Geprüft im echten Fenster, eigene Instanz: über die Brücke, L7 mit SendInput unter der Sperre. L34 ist im Original nicht nachgemessen; die Firepixel-Beobachtung steht in der Zeile. Abschnitt 5 vorher 34 ✅ / 4 ⚠️ / 1 ❌, nachher 39 ✅ / 0 ⚠️ / 0 ❌. Die Gesamtzählung unten ist an die Summe der Bereiche angeglichen (vorher veraltet: 106/16/4). Im Original nicht mehr gemessen, weil es bis 17:40 ausfiel und danach §7a gilt: Kante an Kante beim Verschieben, Stiftpunkte an Kanten, Alt-Ziehen an mittleren und End-Eckpunkten, Ecke einer Ellipsen-Bitmap sowie Umschalt-/Strg-Klick auf durchsichtige Pixel. Diese Punkte stehen in NACHPRUEFUNG und als Befunde LW. Keine neuen Paarbilder (§7a).

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
- ~~Stift- und Unterauswahl-Feinheiten: Kurvenpunkte, Alt-Ziehen an Griffen, Pfad fortsetzen~~ – PM-3, L31–L35.
- Bitmapwerkzeuge im Detail: Pinsel, Radierer, Weichzeichner, Stempel – PM-3, L36–L39; offen: Farbe ersetzen, Bleistift, Abwedler/Nachbelichter, Verwischen.
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
| W1 | alle Bitmap- und Auswahlwerkzeuge (Rahmen, Lasso, Zauberstab, Pinsel, Bleistift, Radiergummi, Weich-/Scharfzeichner, Abwedler, Nachbelichter, Verwischen, Stempel, Farbe ersetzen, Rote Augen, Pipette, Füllwerkzeug, Verlauf) | 2. Zeile rechts | leer (Masken-Teilfeld 14119 erscheint nur bei Maskenbearbeitung) | PM-2 (12.10., echtes Fenster): Teilfeld nicht mehr sichtbar (Rahmen, Lasso, Zauberstab, Pinsel, Radiergummi, Weich-/Scharfzeichner, Abwedler, Nachbelichter, Verwischen, Stempel, Farbe ersetzen, Rote Augen, Pipette, Füllwerkzeug, Verlauf) | ✅ | a_W1_auswahlrahmen, a_W1_W3_radiergummi |
| W2 | Radiergummi, Weichzeichner, Scharfzeichner, Abwedler, Nachbelichter, Verwischen, Farbe ersetzen | „Form“ rund/eckig | gewählte Form als gedrückter Knopf erkennbar (Rahmen) | PM-2 (12.10., echtes Fenster): gewählte Form als gedrückter Knopf mit blauem Rand erkennbar | ✅ | a_W1_W3_radiergummi, a_W4_weichzeichner, a_W6_verwischen |
| W3 | Radiergummi | Größe / Rand | 13 / 2 (PREF `EraserToolDiameter=13`, `SoftEdge=20` → 2) | PM-2 (12.10., echtes Fenster): 13 / 2 | ✅ | a_W1_W3_radiergummi |
| W4 | Weichzeichner, Scharfzeichner | Größe / Rand / Intensität | 13 / 100 / 50 (PREF) | PM-2 (12.10., echtes Fenster): 13 / 100 / 50 (beide) | ✅ | a_W4_weichzeichner |
| W5 | Abwedler, Nachbelichter | Größe / Rand / Belichtung / Bereich | 13 / 100 / 50 / Mitteltonfarben | PM-2 (12.10., echtes Fenster): 13 / 100 / 50 / Mitteltonfarben (beide) | ✅ | a_W5_abwedler |
| W6 | Verwischen | Größe / Rand / Druck / Farbe | 22 / 50 / 50 / Farbfeld zeigt Farbe | PM-2 (12.10., echtes Fenster): 22 / 50 / 50; Farbfeld grau – die grüne Farbe im Original stammt aus dessen gespeicherten Voreinstellungen (`SmudgeToolFingerPaintColor` hat keinen Werkswert), Zustand, kein Befund | ✅ | a_W6_verwischen |
| W7 | Stempel | Größe / Rand | 16 / 50 (PREF `StampToolSize=16`, `Edge=500`) | PM-2 (12.10., echtes Fenster): 16 / 50 | ✅ | a_W7_stempel |
| W8 | Farbe ersetzen | Größe / Toleranz / Stärke | 13 / 32 / 200 | FV-16 (12.10., kittest, `118aada`): Größe 13 / Toleranz 32 / Stärke 200 aus den Werksvorgaben, „Von“ #00FF00, „in“ #FF0000 (eigene Voreinstellungen des Werkzeugs, nicht Strich-/Füllfarbe); **umgesetzt, Nachprüfung ausstehend**. Vorher PM-2 (12.10., echtes Fenster): 13 / 32 / **100**; „Von“-Farbe schwarz, „in“ grau (Original #00FF00 / #FF0000) – Befunde PM-2 (LW W8, EB prefs.json) | 🔧 | a_W8_farbe_ersetzen |
| W9 | Rote Augen entfernen | Toleranz / Stärke | 50 / 50 | PM-2 (12.10., echtes Fenster): 50 / 50 | ✅ | a_W9_rote_augen |
| W10 | Rote Augen entfernen | Name im Kopf | zweizeilig „Rote Augen / entfernen“ | **PM-4 (12.10., echtes Fenster, eigene Instanz mit eigenem APPDATA, Debug-Build `11e454c`, echte Maus/Tastatur unter der Sperre):** zweizeilig „Rote Augen / entfernen“. FV-9 (12.10., kittest): zweizeilig „Rote Augen / entfernen“ (Umbruch an Wortgrenzen, erst ab drei Zeilen „…“); vorher PM-2 „Rote Augen en…“ | ✅ | a_W9_rote_augen, pm4_W10_W27_W28_kopf |
| W11 | Freiform | Größe / Druck / Vorschau | 72 / ☑ / ☑ | PM-2 (12.10., echtes Fenster): 72 / ☑ / ☑ | ✅ | a_W11_freiform |
| W12 | Bereich umformen | Größe / Stärke / Druck Größe / Druck Stärke | 72 / 80 / ☑ / ☐ | PM-2 (12.10., echtes Fenster): 72 / 80 / ☑ / ☐ (Kopf gekürzt, siehe W28) | ✅ | a_W12_bereich_umformen |
| W13 | Pfadausbesserung (additiv/subtraktiv) | Druck / Geschwindigkeit / Rate | ☑ / ☑ / 10 | PM-2 (12.10., echtes Fenster): ☑ / ☑ / 10 (additiv und subtraktiv) | ✅ | a_W13_pfadausbesserung |
| W14 | Text | Schriftfamilie | Myriad Pro (Vorgabeschrift) | FV-9 (12.10., echtes Fenster): Myriad Pro / „Regular“ (Werkzeug und Textobjekt T1; vorher „Bold“, weil die erste Schriftdatei der Familie fett ist) | ✅ | a_W14_W18_textwerkzeug |
| W15 | Text | Laufweite, Zeilenabstand (+Einheit), Absatzeinzug, Abstand vor/nach, Horizontalskala, Grundlinie, Anti-Alias, Autom. Unterschneidung | alle **bedienbar**, bevor man tippt (Vorgabe für den neuen Text) | **PM-4 (12.10., echtes Fenster, eigene Instanz mit eigenem APPDATA, Debug-Build `11e454c`, echte Maus/Tastatur unter der Sperre):** ohne Auswahl per Klick und Tippen gesetzt: Laufweite 10, „Anti-Alias stark“ (Liste per Klick), „Autom. Unterschneidung“ ☑ – kein Verlaufsschritt; danach Text per Klick + „Neu“ + Esc: der neue Text hat Laufweite 10, „Anti-Alias stark“ und ☑ Autom. Unterschneidung. PM-2 (12.10., echtes Fenster): Laufweite, Zeilenabstand + Einheit, Einzug, Abstand vor/nach, Horizontalskala bedienbar; FV-9 (12.10.): auch Anti-Alias und Autom. Unterschneidung bedienbar (echtes Fenster: „Anti-Alias glatt“ aktiv; kittest: Wahl setzt `text.setToolDefaults` ohne Verlaufsschritt, der nächste Text bekommt sie) | ✅ | a_W14_W18_textwerkzeug, pm4_W15_W17_textwerkzeug |
| W16 | Text | Kurzzeichen Fett/Kursiv | „F“ / „K“ (deutsch) | FV-9 (12.10., echtes Fenster): „F“ / kursives „K“ (englisch „B“ / „I“, Schlüssel je Sprache) | ✅ | a_W14_W18_textwerkzeug |
| W17 | Text | Knopf Textausrichtung (horizontal/vertikal, „ab/cd“) neben U | vorhanden | **PM-4 (12.10., echtes Fenster, eigene Instanz mit eigenem APPDATA, Debug-Build `11e454c`, echte Maus/Tastatur unter der Sperre):** „ab/cd ▾“ ohne Auswahl bedienbar; „Vertikal von rechts nach links“ gewählt → Symbol „ac/bd“, der nächste Text steht senkrecht (16 × 61 px). JSF `orientation` meldet dafür „vertical left to right“ – Zuordnung 1/3 in fp-script vertauscht (Befund PM-4, AU). FV-9 (12.10., kittest): Knopf „ab/cd ▾“ auch ohne Auswahl bedienbar, setzt die Richtung neuer Texte (`text.setToolDefaults`); vorher PM-2: grau | ✅ | a_W14_W18_textwerkzeug, pm4_W15_W17_textwerkzeug |
| W18 | Text | Textfarbe | eigenes Farbfeld des Textwerkzeugs (schwarz) | PM-2 (12.10., echtes Fenster): eigenes Farbfeld des Textwerkzeugs, schwarz | ✅ | a_W14_W18_textwerkzeug |
| W19 | Polygon | „Winkel“ und „Autom.“ bei Form = Polygon | bedienbar (Winkel 38, Autom. ☑) | **PM-4 (12.10., echtes Fenster, eigene Instanz mit eigenem APPDATA, Debug-Build `11e454c`, echte Maus/Tastatur unter der Sperre):** „Winkel“ 38 und „Autom.“ ☑ bei Form Polygon; „50“ getippt + Eingabe → „Autom.“ ☐; Klick auf „Autom.“ → wieder 38 ☑. FV-10 (12.10., kittest + offscreen gerendertes Bild, f618672): Winkel 38 und „Autom.“ ☑ bei Polygon und Stern bedienbar; getippter Winkel hebt „Autom.“ auf; vorher PM-2 ⚠️ (Befund PM-2) | ✅ | a_131, fv10_W19_polygon, pm4_W19_polygon |
| W20 | Linie, Stift, Vektorpfad, Rechteck, Ellipse, Polygon, Auto-Form, Pinsel | Strichposition innen/mitte/außen im Werkzeug-Inspektor | nicht vorhanden (nur am Objekt) | PM-2 (12.10., echtes Fenster): keine Knöpfe mehr im Werkzeug-Inspektor (Linie, Stift, Rechteck, Ellipse, Polygon, Auto-Form, Pinsel) | ✅ | a_W20_rechteck_werkzeug |
| W21 | alle Vektorwerkzeuge, Füllwerkzeug, Verlauf | Textur bei „keiner“ | zeigt Texturnamen („Ader“) mit 0 %; Liste mit 77 Texturen (Ressourcen- und Benutzer-Texturordner), Trennlinie, „Andere…“ | **PM-4 (12.10., echtes Fenster, eigene Instanz mit eigenem APPDATA, Debug-Build `11e454c`, echte Maus/Tastatur unter der Sperre):** Füll- und Strich-Textur zeigen ohne Auswahl „Ader“ 0; Klick öffnet die Liste mit „Ader“ oben, kein „Ohne“. Nebenbei: die aufgeklappte Liste ist so schmal wie das Feld, lange Namen („Kreuzschraffur 1…3“, „Linie waagerecht …“) sind ohne „…“ abgeschnitten – Befund PM-4 (IBD). FV-15 (12.10., echtes Fenster, Brücke, 5eee815/ae5581d): Liste aus `Configuration\Textures` + `First Run\Textures` + `%APPDATA%\Firepixel\Textures` = 77 Texturen nach dem deutschen Namen, darunter Trennlinie und „Andere…“ (immer sichtbar; Datei wählen → Textur mit Maske an der Auswahl, als Werkzeugvorgabe der Dateiname); eine Textur im eigenen Benutzerordner erscheint in der Liste; Musterliste ebenso (60, „Andere…“). Vorher FV-10 (12.10., kittest + offscreen gerendertes Bild, f618672): Texturliste zeigt „Ader“ (Datei „Vein“, englisch „Vein“) mit 0 bei Rechteck/Ellipse/Polygon/Stift (Füllung und Strich), Füllwerkzeug, Verlauf und am gewählten Rechteck; kein Eintrag „Ohne“ mehr, Texturnamen übersetzt (`pi-texture-name-*`). Rest (52 statt 77, ohne „Andere…“) von FV-15 behoben; vorher PM-2 ⚠️ (Befund PM-2) | ✅ | a_W20_rechteck_werkzeug, fv10_W21_W22_rechteck, fv10_W21_original_texturliste, fv15_W21_texturliste_bruecke, fv15_W21_andere_bruecke, fv15_W21_musterliste_bruecke, pm4_W21_W22_rechteck |
| W22 | Rechteck, Ellipse, Auto-Form, Rahmen, Lasso, Zuschneiden, Exportbereich | B/H/X/Y | Felder weiß/bedienbar (leer) | **PM-4 (12.10., echtes Fenster, eigene Instanz mit eigenem APPDATA, Debug-Build `11e454c`, echte Maus/Tastatur unter der Sperre):** B/H/X/Y ohne Auswahl weiß, leer und bedienbar bei Rechteck, Ellipse, Auto-Form, Lasso, Zuschneiden und Exportbereich; „100“ + Eingabe ins H-Feld ohne Auswahl ohne Wirkung (0 Elemente, Feld bleibt leer). FV-10 (12.10., kittest + offscreen gerendertes Bild, f618672): B/H/X/Y ohne Auswahl weiß, leer, bedienbar (Rechteck, Ellipse, Auto-Form, Rahmen, Lasso, Zuschneiden, Exportbereich); Eingabe ohne Auswahl ohne Wirkung; vorher PM-2 ⚠️ (Befund PM-2) | ✅ | a_W1_auswahlrahmen, a_W20_rechteck_werkzeug, fv10_W21_W22_rechteck, fv10_W22_W24_auswahlrahmen, pm4_W21_W22_rechteck, pm4_W22_werkzeuge |
| W23 | Rechteck | „Eckrundung des Rechtecks“ im Werkzeug | Feld 14795 und Beschriftung 15079 existieren (Wert 0, Lage 536,1054 im Inspektor), sind aber **unsichtbar** (`IsWindowVisible` = 0); der Werkzeug-Inspektor zeigt keine Rundung (PM-1, frisch gestartetes Original) | FV-7: Zeile entfernt, Werkzeug-Inspektor ohne Rundung wie im Original; die Rundung des gewählten Rechtecks (O1) bleibt (Test `inspector_paritaet::w23_rectangle_tool_without_roundness_row`) | ✅ | a_W23_rechteck_werkzeug_inspektor |
| W24 | Auswahlrahmen | Feste Breite/Höhe | Symbole ↔ ↕, Werte 0/0, grau bei „Normal“ | **PM-4 (12.10., echtes Fenster, eigene Instanz mit eigenem APPDATA, Debug-Build `11e454c`, echte Maus/Tastatur unter der Sperre):** Symbole ↔/↕; Stil per Klick in die Liste: „Normal“ 0/0 grau → „Feste Größe“ 64/64 → „Festes Verhältnis“ 1/2 → „Normal“ 0/0 grau. FV-10 (12.10., kittest + offscreen gerendertes Bild, f618672): Symbole ↔/↕, bei „Normal“ 0/0 grau; „Festes Verhältnis“ 1/2, „Feste Größe“ 64/64 aus den Voreinstellungen (im Original gemessen), Werte bleiben beim Wechsel; vorher PM-2 ⚠️ (Befund PM-2) | ✅ | a_W1_auswahlrahmen, fv10_W22_W24_auswahlrahmen, fv10_W24_original_feste_werte, pm4_W24_auswahlrahmen |
| W25 | Verlauf | „Auswahl füllen“ / Toleranz | ☑ / Toleranz grau (hängt davon ab) | FV-16 (12.10., kittest, `118aada`): „Auswahl füllen“ und Toleranz je Werkzeug getrennt gemerkt (`gradient_fill_selection`/`gradient_tolerance`), auch headless; Werkzeug-Inspektor des Verlaufs ohne die Füllart-Knöpfe Keine/Einfarbig/Verlauf/Muster (Füllwerkzeug mit); **umgesetzt, Nachprüfung ausstehend**. Vorher PM-2 (12.10., echtes Fenster): ☑ / Toleranz grau; Füllwerkzeug ☐ / Toleranz bedienbar | 🔧 | a_W25_verlauf |
| W26 | Verlauf | Füllfarbfeld | zeigt den Verlauf | **FV-16 (12.10., im Original gemessen + kittest, `118aada`):** Original bei Füllfarbe #808080 und Strichfarbe #C04040 → Farbfeld Schwarz → Weiß, also eigene Farben des Werkzeugs (keine Voreinstellung dazu); Firepixel malt und zeigt jetzt die eigenen Werkzeugfarben (Werksvorgabe Schwarz → Weiß), Klick aufs Farbfeld öffnet den Verlaufseditor; was das Original beim Ziehen malt, war nicht messbar (interner Fehler bei jedem Leinwandklick); **umgesetzt, Nachprüfung ausstehend**. Vorher **PM-4 (12.10., echtes Fenster, eigene Instanz mit eigenem APPDATA, Debug-Build `11e454c`, echte Maus/Tastatur unter der Sperre):** Füllfarbfeld des Verlauf-Werkzeugs zeigt den Verlauf. FV-10 (12.10., kittest + offscreen gerendertes Bild, f618672): Füllfarbfeld zeigt den Verlauf, den das Werkzeug malt (Füll- → Strichfarbe). Rest: Farben/Füllart-Knöpfe – Befund FV-10; vorher PM-2 ⚠️ (Befund PM-2) | 🔧 | a_W25_verlauf, fv10_W26_verlauf, pm4_W26_W29, fv16_W26_original_verlauf |
| W27 | Auto-Form | Kopf | Name der gewählten Form („Abgefastes Rechteck“) | **PM-4 (12.10., echtes Fenster, eigene Instanz mit eigenem APPDATA, Debug-Build `11e454c`, echte Maus/Tastatur unter der Sperre):** Kopf zeigt den Namen der gewählten Form (im frischen Profil „Abgerundetes Rechteck“). FV-9 (12.10., kittest): Name der gewählten Form („Abgefastes Rechteck“, je Sprache); vorher PM-2 „Automatische …“ | ✅ | a_132, pm4_W10_W27_W28_kopf |
| W28 | Pfadausbesserung, Polygon-Segment, Hotspot-Rechteck | Name im Kopf | zweizeilig voll | **PM-4 (12.10., echtes Fenster, eigene Instanz mit eigenem APPDATA, Debug-Build `11e454c`, echte Maus/Tastatur unter der Sperre):** Polygon-Segment, Hotspot-Rechteck und Bereich umformen zweizeilig ungekürzt; **„Pfadausbesserung - additiv“ bricht mitten im Wort um** („Pfadausbesserun / g - additiv“), das Original „Pfadausbesserung / - additiv“ (a_W13) – Befund PM-4 (IBD). FV-9 (12.10., kittest): Pfadausbesserung, Polygon-Segment, Hotspot-Rechteck und Bereich umformen zweizeilig ungekürzt; vorher PM-2 gekürzt **NZ-1 (ec4ac99, kittest):** Umbruch nur an Leerzeichen bzw. nach Bindestrich – „Pfadausbesserung / - additiv“, alle Werkzeugnamen in de/en geprüft; **umgesetzt, Nachprüfung ausstehend** | 🔧 | a_W13_pfadausbesserung, a_W12_bereich_umformen, pm4_W10_W27_W28_kopf |
| W29 | Zeiger (Dokument) | Exportformat-Liste | Format des Dokuments | **PM-4 (12.10., echtes Fenster, eigene Instanz mit eigenem APPDATA, Debug-Build `11e454c`, echte Maus/Tastatur unter der Sperre):** kein Knopf „Bearbeiten…“ an der Exportformat-Liste (`ui_find` „Bearbeiten“: 0 Treffer). FV-10 (12.10., kittest + offscreen gerendertes Bild, f618672): kein Knopf „Bearbeiten…“ mehr neben der Exportformat-Liste; vorher PM-2 ⚠️ (Befund PM-2) | ✅ | a_100, fv10_W29_dokument, pm4_W26_W29 |
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
| F2 | „-5“ + Eingabe | 5 (Feld nimmt nur Ziffern an, das Minus wird verworfen) | PM-2 (12.10., echtes Fenster): 5 | ✅ |  |
| F3 | „12,5“ + Eingabe | 100 (Komma wird verworfen → 125 → 100) | PM-2 (12.10., echtes Fenster): 100 | ✅ |  |
| F4 | „x“ + Eingabe | Buchstabe wird nicht angenommen, Wert bleibt | Wert bleibt, kein Verlaufsschritt | ✅ | |
| F5 | Pfeil hoch im Feld | keine Änderung | keine Änderung | ✅ | |
| F6 | Mausrad über dem Feld | keine Änderung | keine Änderung | ✅ | |
| F7 | „7“ + Esc, dann Klick auf die Leinwand | 7 wird übernommen (Esc verwirft nicht) | FV-9 (12.10., echtes Fenster, 4 Läufe + Esc/Tab): 7 übernommen. Ursache war nicht Esc: bei schnellen echten Klicks (Drücken/Loslassen in einem Bild) änderte die Leinwand die Auswahl, bevor der übernommene Wert ausgeführt wurde (auch F9 ging so in 1 von 3 Läufen verloren); jetzt führt der Inspektor die Übernahme sofort aus. Offen (Nebenbeobachtung PM-2): Klick auf ein **anderes** Objekt überträgt den Feldtext in Firepixel auf das vorher gezeigte Objekt, im Original auf das neu angeklickte. FV-15 (12.10.): im Original nicht nachmessbar (jeder Leinwandklick meldet „interner Fehler“, nichts wird gewählt; Befund FV-15), Firepixel unverändert (echtes Fenster, Brücke, 3/3: 30 an R1) | ✅ | a_F7_esc_dann_leinwand, fv15_F7_klick_r2_bruecke, fv15_F7_original_interner_fehler |
| F8 | „60“ + Tab | übernommen | übernommen | ✅ | |
| F9 | „40“, Klick auf die Leinwand | übernommen | FV-9 (12.10., echtes Fenster, 3 Läufe): übernommen (vor der Korrektur zu F7 gingen schnelle Klicks verloren) | ✅ | |
| F10 | Strg+A im Feld | **markiert den Feldtext**, Objektauswahl bleibt; getippter Wert bleibt und wird mit Eingabe übernommen (PM-2, 12.10., 3 Läufe; die frühere Angabe „Alles auswählen“ ließ sich nicht bestätigen) | FV-9 (12.10., echtes Fenster, 3 Läufe): markiert den Feldtext, Auswahl bleibt R1; „55“ + Strg+A + Eingabe → 55 (vorher PM-2: wählte alle 9 Objekte) | ✅ | a_F10_strg_a_im_feld |
| F11 | jede Übernahme | ein Verlaufsschritt | ein Schritt „Deckkraft“ | ✅ | |

### 2a. Feldprobe B, X, Rundung, Strich (PM-1, Rechteck R1 50,50–150,120, `uxp_pm_v7.ps1`)

Eingabe wie bei F1–F11 (Klick, Ende, Umschalt+Pos1, Tippen, Eingabe). Strich im Original aus UX-A übernommen (dort hatte R1 diesmal keinen Strich).

| Nr | Feld | Original | Firepixel | |
|---|---|---|---|---|
| F12 | B | „9999“ → 6000 (Feld zeigt schon beim Tippen 6000); „-5“ und „0“ werden **verworfen** (Breite bleibt 100); „12,5“ → **512** (Feldtext „512“, das Komma setzt die Schreibmarke an den Anfang); „x“ → Feld „0“, verworfen; Pfeil hoch und Rad: nichts | FV-7: 9999 → 6000 (mit Meldung); „-5“, „0“ und „x“ verworfen (Breite bleibt 100, kein Verlaufsschritt); „12,5“ → Feldtext „512“, Breite 512 (Komma setzt die Schreibmarke an den Anfang); Pfeil und Rad: nichts. Rest: nach „x“ zeigt das Feld wieder „100“ statt „0“ (Test `inspector_paritaet::f12_f13_width_and_x_fields_like_original`) | ✅ |
| F13 | X | „9999“ → 9999 und „-5“ → -5 (keine Grenze); „12,5“ → 512; „x“ → Feld „0“, Objekt nach **x = 0**; „0“ → 0; Pfeil und Rad: nichts | FV-7: 9999 ✅, -5 ✅, 0 ✅, Pfeil/Rad ✅; „12,5“ → 512 ✅; „x“ → Feld „0“, x = 0 ✅ (gelesen wie `atoi`; gleicher Test) | ✅ |
| F14 | Rundung | „9999“ → 100 %; „-5“ → 5 % (Minus verworfen); „12,5“ → 100 % (Komma verworfen → 125); „0“ → 0; Pfeil und Rad: nichts | gleich | ✅ |
| F15 | Strichstärke | „9999“ → 100, „-5“ → 5 (UX-A); Pfeil und Rad: nichts | 9999 → 100, „-5“ → 5, „12,5“ → 100, „x“ bleibt, „0“ → 1; Pfeil/Rad nichts | ✅ |

## 3. Text: Schriftgröße und Bearbeitung

| Nr | Ablauf | Original | Firepixel | | Bild |
|---|---|---|---|---|---|
| T1 | Text tippen, Strg+A im Editor, Größe 48 + Eingabe | markierter Text wird sofort 48 pt | ebenso, Rahmen wächst mit | ✅ | a_200 |
| T2 | Textobjekt mit dem Zeiger gewählt, Größe 24 | ganzer Text 24 pt, Rahmen passt (69 × 31) | ganzer Text 24 pt, Rahmen passt (53 × 29, Arial) | ✅ | a_201 |
| T3 | Größe „200“ eintippen | angenommen (200), obwohl der Schieber nur bis 96 reicht | angenommen | ✅ | |
| T4 | Pfeil neben „Größe“ | **senkrechter** Popup-Schieber über dem Feld | PM-2 (12.10., echtes Fenster): senkrechter Schieber über dem Feld | ✅ | a_T4_T5_popup_schieber |
| T5 | Esc im geöffneten Größen-Schieber | schließt den Schieber, Auswahl bleibt | PM-2 (12.10., echtes Fenster): schließt nur den Schieber, T1 bleibt gewählt | ✅ | a_T4_T5_popup_schieber |
| T6 | Größe ändern bei Einfügemarke ohne markierten Text („Abc“, Größe 48 + Eingabe, „D“, Esc) | wirkt nur auf neu getippte Zeichen: Läufe [Abc 24 pt][D 48 pt]; Eingabe im Feld gibt den Fokus an den Text zurück; Werkzeug bleibt Text | ganzer Text 48 pt, ein Lauf [AbcD 48 pt] (PM-1) → **FV-6:** ausstehende Attribute an der Einfügemarke; echtes Fenster (eigene Instanz, SendInput, Größe wie das Inspektorfeld per `text.setAttributes`): [Abc 12 pt][D 48 pt] (Werkzeugvorgabe dort 12 pt), Werkzeug bleibt Text; Rahmen wächst erst mit dem „D“ | ✅ gemessen (FV-6) | a_T6_groesse_einfuegemarke, phase5_fv6_groesse_einfuegemarke |

## 4. Inspektor je Objektart

Testdokument mit denselben Objekten in beiden Programmen (per JSF angelegt): R1, R2 (50 % Deckkraft), E1, T1, Gruppe G, Bitmap B, Segment S, Hotspot H, Symbol Sym. Im Original haben neue Objekte den aktuellen Strich (4 px „Weich abgerundet“), in Firepixel keinen; das ist ein Zustandsunterschied und kein Befund.

| Nr | Objekt | Element | Original | Firepixel | | Bild |
|---|---|---|---|---|---|---|
| O1 | Rechteck | Felder Füllung, Strich, Strichposition, Deckkraft, Filter, Rundung + Einheit, Pfadoperationen | so | so | ✅ | a_300 |
| O2 | Rechteck | Vorschaubild oben links | Objekt an seiner Lage im verkleinerten Dokument | **PM-4 (12.10., echtes Fenster, eigene Instanz mit eigenem APPDATA, Debug-Build `11e454c`, echte Maus/Tastatur unter der Sperre):** R1 per Klick gewählt: Vorschaubild zeigt R1 an seiner Lage im verkleinerten Dokument auf grau-weißem Schachbrett (der Nebenbefund von FV-10 „Schachbrett weiß“ tritt im echten Fenster nicht auf); Umschalt+Klick auf R2: kein Vorschaubild, „2 Objekte“ mittig. FV-10 (12.10., kittest + offscreen gerendertes Bild, f618672): Objekt an seiner Lage im verkleinerten Dokument auf Schachbrett (Seitenformat); bei Mehrfachauswahl kein Vorschaubild, „n Objekte“ mittig; vorher PM-2 ⚠️ (Befund PM-2) | ✅ | a_O14_O16_mehrfachauswahl, fv10_O2_rechteck, fv10_O2_O16_mehrfachauswahl, pm4_O2_O8_O16, pm4_O2_vorschaubild_zoom |
| O3 | Ellipse (Pfad) | alle Felder inkl. „Pinselstrich bearbeiten“, Stil | so | so | ✅ | a_301 |
| O4 | Text | Zeilenabstand eines neuen Textes | 120 % | PM-2 (12.10., echtes Fenster): 120 % | ✅ | a_O4_O8_text |
| O5 | Text | Grenzen eines Textrahmens 50,170–200,200 | X 48, Y 168, B 154, H 28 (2 px Rand um den Textbereich) | PM-2 (12.10., echtes Fenster): X 48, Y 168, B 154, H 28 (EB-1: angezeigte Grenzen ± 2 px) | ✅ | a_O4_O8_text |
| O6 | Text | Anti-Alias / Autom. Unterschneidung eines per Skript angelegten Textes (`addNewText(…, true)` = aus den Voreinstellungen) | „Anti-Alias glatt“ / ☐ | FV-9 (12.10., echtes Fenster): „Anti-Alias glatt“ / ☐ (Liste jetzt wie `text.setProperties`: Kein Anti-Alias = harte Füllkante, `TAAi` 0 glatt, 3 System → scharf); JSF `antiAliased` bleibt AU-Befund O11/O6 | ✅ | a_O4_O8_text |
| O7 | Text | Pinselstrich-Feld (Stift + Farbe) in Zeile 2 | vorhanden | PM-2 (12.10., echtes Fenster): Stift + Strichfarbfeld in Zeile 2 vorhanden | ✅ | a_O4_O8_text |
| O8 | Text | Horizontalskala | „100%“ | **PM-4 (12.10., echtes Fenster, eigene Instanz mit eigenem APPDATA, Debug-Build `11e454c`, echte Maus/Tastatur unter der Sperre):** Text „Hallo“ gewählt: „100%“; „120“ + Eingabe → „120%“, „80%“ + Eingabe → „80%“, nach Abwählen und erneutem Klick erhalten, je ein Verlaufsschritt `text.setAttributes {horizontalScale}`; Textwerkzeug ohne Auswahl „100%“. FV-10 (12.10., kittest + offscreen gerendertes Bild, f618672): „100%“ beim Objekt und beim Textwerkzeug; „120“ bzw. „80%“ getippt gilt, Feld zeigt den Wert mit „%“; vorher PM-2 ⚠️ (Befund PM-2) | ✅ | a_O4_O8_text, fv10_O8_text, pm4_O2_O8_O16 |
| O9 | Gruppe | „Gruppieren (2)“, Füllung/Strich/Deckkraft/Filter/Stil | so | so | ✅ | a_303 |
| O10 | Bitmap | Deckkraft, Mischmodus, Filter, Stil, B/H/X/Y | so | so | ✅ | a_304 |
| O11 | Segment (per Skript `addNewHotspot("slice", …)`) | Objektart | Segment (Typ Vordergrundbild, Exportformat, Hyperlink, Alt, Ziel; automatischer Name „_r2_c2“) | PM-2 (12.10., echtes Fenster): unverändert, `addNewHotspot("slice", …)` ergibt `[object Hotspot]` (Original `[object SliceHotspot]`); AU-Befund O11/O6 weiter offen | ❌ | a_305 |
| O12 | Hotspot | Form, Farbe, Hyperlink, Alt, Ziel | so | so | ✅ | a_306 |
| O13 | Symbol | Deckkraft, Mischmodus, Filter, Stil | so | so | ✅ | a_307 |
| O14 | 2 Rechtecke, Deckkraft 100 und 50 | Deckkraft-Feld | **leer** (gemischte Werte) | PM-2 (12.10., echtes Fenster): leer | ✅ | a_O14_O16_mehrfachauswahl |
| O15 | 2 Rechtecke | Füllfarbfeld | Farbe des zuletzt gewählten Objekts | PM-2 (12.10., echtes Fenster): blau = Farbe des zuletzt gewählten Objekts R2 (JSF `fw.selection` liefert die Reihenfolge allerdings R1, R2; Original R2, R1) | ✅ | a_O14_O16_mehrfachauswahl |
| O16 | 2 Rechtecke | Stil-Liste | leer (gemischt) | **PM-4 (12.10., echtes Fenster, eigene Instanz mit eigenem APPDATA, Debug-Build `11e454c`, echte Maus/Tastatur unter der Sperre):** Umschalt+Klick (R1 + R2): Stil-Liste leer, „Neuer Stil“ grau. FV-10 (12.10., kittest + offscreen gerendertes Bild, f618672): bei mehr als einem Objekt leere Stil-Liste (auch ohne Stil), „Neuer Stil“ grau; vorher PM-2 ⚠️ (Befund PM-2) | ✅ | a_O14_O16_mehrfachauswahl, fv10_O2_O16_mehrfachauswahl, pm4_O2_O8_O16 |
| O17 | Segmente und Hotspots | Überlagerung auf der Leinwand (grün/cyan) | sichtbar | PM-2 (12.10., echtes Fenster): Hotspots cyan sichtbar; ein mit dem Segment-Werkzeug gezogenes Segment grün mit roten Segment-Hilfslinien wie im Original (das JSF-„Segment“ S ist wegen O11 ein Hotspot) | ✅ | a_O17_segment_hotspot |

## 5. Leinwand und Werkzeugverhalten (echte Maus und Tastatur)

| Nr | Ablauf | Original | Firepixel | | Bild |
|---|---|---|---|---|---|
| L1 | Gummiband 20,20→100,100 (R1 50–150 nur teilweise darin) | R1 gewählt (berührte Objekte zählen) | PM-3 (12.10., echtes Fenster, beide Programme): R1 gewählt | ✅ | |
| L2 | Gummiband um R1 und E1 | beide | beide | ✅ | |
| L3 | R1 ziehen, Umschalt **während** des Ziehens, +40/+10 | +40/0 (auf 0° eingerastet, ganzzahlig) | PM-3 (12.10., echtes Fenster, beide Programme): 90,50–190,120 (+40/0), Strg+Z stellt her | ✅ | |
| L4 | Alt-Ziehen R1 +40/+20 | Kopie bei +40/+20 (90,70–190,140); auch +40/+21 → 90,71 und ohne Alt +40/+20 → 90,70: beim Verschieben kein Fang an der G-Mitte y = 75 (Abstand 5 bzw. 4) | PM-3 (12.10., echtes Fenster, beide Programme): Kopie 90,75–190,145; auch +40/+21 und ohne Alt → y 75 (Oberkante rastet an der G-Mitte ein) – Befund PM-3 (LW L4/L35) → **FV-14** (`8b03953`, echtes Fenster, Brücke, eigene Instanz): +40/+20 → 90,70–190,140, +40/+21 → 90,71–190,141, mit Alt die Kopie ebenso; an anderen Objekten rasten nur gleichartige Stellen (Kante an Kante, Mitte an Mitte: +47/0 → rechte Kante an E1 200, Mitte an G-Mitte); Kante an Kante ist im Original noch nicht gemessen (NACHPRUEFUNG) | ✅ (FV-14) | |
| L5 | 3× Pfeil rechts, einmal Strg+Z | +3, ein Rückgängig stellt alles her | ebenso (von UX-A behoben) | ✅ | |
| L6 | Rechtsklick auf nicht gewählte Ellipse | Ellipse gewählt, Kontextmenü | ebenso (kittest) | ✅ | |
| L7 | Strg+Leertaste+Klick | vorübergehend Zoom: 100 → 150 % (auch wenn Tasten und Klick im selben Augenblick kommen) | PM-3 (12.10., echtes Fenster, beide Programme): Tasten 0,4 s vor dem Klick gehalten → 150 %; Tasten und Klick ohne Pause (ein SendInput direkt vor dem Mausdruck) → bleibt 100 % – Befund PM-3 (LW L7, klein) → **FV-14** (`8b03953`, echtes Fenster, eigene Instanz, Strg, Leertaste, Maus ab/auf, Leertaste und Strg los in **einem** SendInput-Aufruf unter der Sperre): 100 → 150 %, nochmal 150 → 200 %, kein Zeigerklick (keine Auswahl); über die Brücke mit vorher gehaltenen Tasten ebenso 150 % | ✅ (FV-14) | |
| L8 | Strg+Alt+Leertaste+Klick | verkleinert (150 → 100 %); live nicht messbar, weil die globale Taste der Claude-App die Kombination abfängt (§4d) | über die Eingabe-Einspeisung der Brücke (`ui_input`, keine echte Taste): 150 → 100 %; Gegenprobe Strg+Leertaste+Klick 100 → 150 % (PM-1) | ✅ | |
| L9 | Mausrad 1 Rastung ohne Taste (400 %) | senkrechter Bildlauf um 52 Bildschirmpixel | **kein Bildlauf** (echte Radereignisse; HG-Befund ist im aktuellen Stand noch offen) | ✅ nachgemessen (FV-3, echtes Fenster, SendInput 120 je Rastung, 400 %): 52 px senkrecht, 3 Rastungen 156 px | a_330, a_331 |
| L10 | Umschalt+Mausrad | nichts | waagerechter Bildlauf 40 px | ✅ nachgemessen (FV-3, echtes Fenster): kein Bildlauf | a_332 |
| L11 | Alt+Mausrad | senkrechter Bildlauf (wie ohne Taste) | senkrechter Bildlauf 40 px | ✅ nachgemessen (FV-3, echtes Fenster): 52 px senkrecht | a_333 |
| L12 | Zauberstab-Klick auf Bitmap | Bitmapmodus („Bitmap bearbeiten“ + „Zauberstab“ im Verlauf), Auswahlrahmen | Auswahlrahmen | ✅ | a_320 |
| L13 | danach **Esc** | „Bitmapmodus beenden“: Auswahlrahmen weg, Werkzeug wechselt zum Zeiger, Bitmap bleibt als Objekt gewählt | PM-3 (12.10., echtes Fenster, beide Programme): Auswahlrahmen weg, Werkzeug Zeiger; **B ist danach nicht gewählt** (keine Griffe, `fw.selection` leer; schon im Bitmapmodus meldet JSF keine Auswahl, Original [B]) – Befund PM-3 (LW L13) → **FV-14** (`8b03953`, echtes Fenster, Brücke): Der Zauberstab wählt B als Objekt (`fw.selection` = [B], auch wenn vorher R1 gewählt war) und schaltet den Bitmapmodus ein; Esc → Auswahlrahmen weg, Zeiger, **B gewählt** mit Griffen | ✅ (FV-14) | a_L13_zauberstab_esc |
| L14 | danach Strg+D | Auswahl aufheben | Auswahl aufheben | ✅ | |
| L15 | Rechteck, Umschalt gehalten, Ziehen 240,145 → +60/+30 | Quadrat 60 × 60 ab dem Startpunkt | 250,150–300,175: kein Quadrat, Startpunkt um 10/5 px verschoben (rastet an E1-Mitte x=250 und R2-Oberkante y=150 ein) | ✅ nachgemessen (FV-3, echtes Fenster, Umschalt vor dem Drücken): 240,145–300,205 | |
| L16 | Rechteck, Alt gehalten | von der Mitte aus: 180–300 × 125–175 | wie ohne Alt (250,150–300,175) | ⚠️ **PM-1, Fangregel im Original (14 Fälle, `uxp_pm_l16.ps1`):** Start- und Endpunkt rasten getrennt in x und y an Kanten und Mitten anderer Objekte ein, Fangabstand **≤ 5** px (5 rastet, 6 nicht); mit **Umschalt rastet nichts** (weder Start noch Ende, auch mit Alt+Umschalt nicht); mit Alt rastet der Mittelpunkt wie sonst der Start, das Ende ebenfalls. Firepixel (echtes Fenster): bei genau 5 px kein Fang (115–175), mit Umschalt rastet es weiter (240,147 → 240,150; Ende 298 → 300), ein Ende bei y = 173 rastet an 170 statt an der näheren 175 (halber Pixel Versatz); Abstand 1–4 und ≥ 6 gleich. Befunde LW. **✅ FV-6 nachgemessen** (echtes Fenster, eigene Instanz, `uxp_pm_l16`-Fälle, Seite jetzt auf ganzen Pixeln 497,322): A1 240,150–300,175, A2 Umschalt 240,145–300,205, A3 Alt 180,125–300,175, A4 180,85–300,205, B1 180,125–300,175, B2 180,113–300,175, B3 (Gleichstand) 180,111–300,175, C2 Umschalt 240,147–300,207, Alt+Umschalt 180,87–300,207, Umschalt Ende 298,160 → 240,135–298,193 – alle wie im Original; Fälle am Text T1 hängen an dessen Grenzen (2-px-Rand, Befund EB). Nebenbefund: bei SendInput mit `SetCursorPos` geht selten (1 von 5) die letzte Bewegung vor dem Loslassen verloren (Ende 297 statt 300, nur ohne Fang sichtbar; winit meldet die Lage des Loslassens aus der letzten Bewegung) | ✅ (FV-6) | |
| L17 | Rechteck, 5× Pfeil hoch während des Ziehens | Rundung bleibt 0 (in CS6 live keine Wirkung) | PM-3 (12.10., echtes Fenster, beide Programme): Rechteck 320,140–400,220, Rundung 0 | ✅ | |
| L18 | Rechteck, Leertaste während des Ziehens | Startpunkt wandert mit (Form verschoben) | Startpunkt wandert mit | ✅ | |
| L19 | Rechteck, Esc während des Ziehens | wird ignoriert, Rechteck entsteht beim Loslassen | PM-3 (12.10., echtes Fenster, beide Programme): Rechteck 320,140–400,220 entsteht | ✅ | |
| L20 | Rechteck, Klick ohne Ziehen | kein neues Objekt | kein neues Objekt | ✅ | |
| L21 | Textwerkzeug: Klick, Esc ohne Text | kein Objekt, Werkzeug bleibt Text | PM-3 (12.10., echtes Fenster, beide Programme): kein Objekt, Werkzeug bleibt Text | ✅ | |
| L22 | Textwerkzeug: Klick bei 300,340, „Abc“, Esc | Text 300,323–339,341 (Klickpunkt ≈ Grundlinie), Werkzeug bleibt Text | PM-3 (12.10., echtes Fenster, beide Programme): Grundlinie auf dem Klickpunkt (y 340), Werkzeug bleibt Text; JSF `pixelRect` 298,327–322,345 (12 pt Werksvorgabe, das Original steht auf größerer Schrift; Firepixel meldet den Rahmen mit 2-px-Rand, das Original die Glyphengrenzen – Befund PM-3 an AU) | ✅ | a_L22_text_klickpunkt |
| L23 | Stift: 2 Klicks, Esc | offener Pfad 250,150–330,150 | ebenso | ✅ | |
| L24 | Stift: Klick, Klick, Doppelklick | offener Pfad mit 3 Punkten bis 330,220 | **kein Pfad** (nach dem Doppelklick ist nichts da) | ✅ nachgemessen (FV-3, echtes Fenster): offener Pfad 250,260–330,330, Werkzeug bleibt Stift | |
| L25 | Zoomwerkzeug: Rahmen 50,50–150,120 aufziehen | 1150 % (stufenlos eingepasst); Rahmen 50,50–250,120 → 722,5 %, 320,140–420,160 → 1445 % (Einpassen in ≈ 1445 × 805 Bildschirmpixel) | PM-3 (12.10., echtes Fenster, beide Programme): 1142,9 %, 717 %, 1434 % (Einpassen in 1434 × 800 = Leinwandbereich 1464 × 830 minus 15 px Rand) – Befund PM-3 (LW L25, klein) → **FV-14** (`8b03953`): Regel aus dem Bild `o_start` von PM-3 abgeleitet: Sichtbereich des Originals 1462 × 822, eingepasst wird in 1445 × 805, also Sichtbereich minus 17 px je Richtung; alle drei Messwerte stimmen genau (1150 % / 722,5 % / 1445 %). Firepixel `view.zoomToRect` → `fit_zoom_rect`; echtes Fenster, Brücke (Leinwand 900 × 554): 767,1 % / 441,5 % / 883 % = Regel; Einpassen behält 15 px je Seite | ✅ (FV-14) | |
| L26 | Zeiger: Eckgriff eines gewählten Rechtecks um +40/+30 ziehen | Rechteck wird **größer** (bis 193,153), Rundung bleibt 0 | keine Änderung (weder Größe noch Rundung) | ✅ nachgemessen (FV-3, echtes Fenster): 50,50–190,150, Rundung 0 | |
| L27 | Zeiger: Doppelklick auf Bitmap | Bitmapmodus, Werkzeug wechselt zum Auswahlrahmen; danach Esc: Werkzeug bleibt Auswahlrahmen, Bitmap bleibt gewählt | Bitmapmodus und Auswahlrahmen (PM-1, echtes Fenster); danach Esc: Werkzeug wechselt zum Zeiger → **FV-6:** Esc ohne Auswahlrahmen ohne Wirkung; echtes Fenster: Doppelklick → B gewählt, `tool.marquee`; Esc → B gewählt, `tool.marquee` | ✅ gemessen (FV-6) | a_L27_doppelklick_bitmap |
| L28 | Zuschneiden: Rahmen ziehen, Esc | Leinwand unverändert, Werkzeug wechselt zum Zeiger | Leinwand unverändert, Werkzeug bleibt Zuschneiden | ✅ nachgemessen (FV-3, echtes Fenster): Werkzeug Zeiger, Breite 600 | |
| L29 | Zuschneiden: Rahmen 20,20–300,200, Eingabe, Strg+Z | 280 × 180, Rückgängig stellt 600 × 400 her | ebenso | ✅ | |
| L30 | Frei transformieren (Strg+T) an R1 50,50–150,120, Eckgriff von 151,121 nach 191,151 | **Ecke skaliert immer proportional** (50,50–193,150, größerer Faktor), Umschalt ändert nichts; Alt = proportional von der Mitte (7,20–193,150); Kante rechts → 191 (Zeigerlage); die Änderung wirkt schon beim Ziehen; **Esc behält die Änderung** und beendet den Modus (Werkzeug Zeiger); Eingabe und Doppelklick innen übernehmen, Strg+Z = ein Schritt; Ziehen am Mittelpunkt verschiebt nur den Drehpunkt (`uxp_pm_3.ps1`) | Ecke unproportional (190,150), nur mit Umschalt proportional (193,150); Alt 10,20–190,150 (Alt+Umschalt 7,20–193,150 gleich); Kante → 188; nach dem Loslassen zeigt die Leinwand bis zur Eingabe die alte Größe; Esc verwirft, Werkzeug bleibt „Skalieren“; Eingabe/Doppelklick, ein Schritt und Mittelpunkt gleich (PM-1) → **FV-6, echtes Fenster (`uxp_pm_3`-Fälle):** Ecke 50,50–193,150, Umschalt 193,150, Alt 7,20–193,150, Alt+Umschalt gleich, Kante rechts 191; nach dem Loslassen zeigt die Leinwand die neue Größe; Esc → 50,50–193,150 und `tool.pointer`, Klick danach wählt; Strg+Z = ein Schritt zurück auf 150,120; Eingabe und Doppelklick innen übernehmen (Zeiger). Gilt für Skalieren/Neigen/Verzerren (Esc behält, A für Neigen/Verzerren: nicht gemessen); JSF `pixelRect` meldet während der offenen Vorschau noch die alte Größe (im Original nicht gemessen) | ✅ gemessen (FV-6) | a_L30_frei_transformieren, phase5_fv6_frei_transformieren_nach_loslassen |
| L31 | Stift: Klick 320,140, Ziehen 400,140 → 440,180, Esc | offener Pfad, Punkt 2 mit Griffen 360,100 / 440,180 | PM-3 (12.10., echtes Fenster, beide Programme): gleich | ✅ | a_L31_L34_stift |
| L32 | Unterauswahl: Griff 440,180 → 460,160 ziehen; dasselbe mit Alt; Punkt 1 ziehen | ohne Taste: Gegengriff dreht mit (346,3,122,1 \| 460,160); mit Alt: nur dieser Griff (360,100 \| 460,160); Punkt 330,130 ohne Griffe | PM-3 (12.10., echtes Fenster, beide Programme): alle drei Werte gleich, Strg+Z stellt her | ✅ | |
| L33 | Unterauswahl: Alt-Ziehen aus Eckpunkt 1 (320,140 → 350,140) | zieht **nur den Ausgangsgriff** heraus: 320,140 (320,140 \| 350,140) | PM-3 (12.10., echtes Fenster, beide Programme): **beide Griffe symmetrisch** (290,140 \| 350,140) – Befund PM-3 (LW L33) → **FV-14** (`8b03953`, echtes Fenster, Brücke): 320,140 (320,140 \| 350,140), Strg+Z stellt her; mittlere und End-Eckpunkte im Original nicht gemessen (NACHPRUEFUNG) | ✅ (FV-14) | |
| L34 | Stift: offenen Pfad 320,140–400,140 gewählt, Klick auf den Endpunkt, Klick 400,220, Esc | Pfad fortgesetzt: 320,140 · 400,140 · 400,220 | PM-3 (12.10., echtes Fenster, beide Programme): gleich. Sonderfall (einmal gemessen): direkt nach Unterauswahl-Schritten und Strg+Z legt das Original beim Klick auf den Endpunkt einen **neuen** Pfad an (nur 400,220), Firepixel setzt fort – Befund PM-3 (LW L34, Beobachtung) → **FV-14:** im Original nicht nachgemessen; in Firepixel (Brücke) setzt der Stift nur einen **gewählten** Pfad fort; ist der Pfad nach den Unterauswahl-Schritten nicht mehr gewählt, entsteht auch hier ein neuer Pfad (nur 400,140 · 400,220), vermutlich die Ursache der Beobachtung (NACHPRUEFUNG) | ✅ | |
| L35 | Stift: Klicks 545,30 · 585,30 · 585,80, Klick auf den Startpunkt | geschlossener Pfad 545,30 · 585,30 · 585,80, Werkzeug bleibt Stift | PM-3 (12.10., echtes Fenster, beide Programme): geschlossen, aber dritter Punkt **585,75** (rastet bei 5 px an der G-Mitte y = 75 ein; Original kein Fang) – Befund PM-3 (LW L4/L35) → **FV-14** (`8b03953`, echtes Fenster, Brücke): Stiftpunkte rasten nicht an Objektmitten: geschlossen 545,30 · 585,30 · **585,80**, Werkzeug bleibt Stift; das Aufziehen von Rechtecken rastet weiter an Mitten (L16) | ✅ (FV-14) | a_L31_L34_stift |
| L36 | Pinsel (Bitmap) ohne Auswahl: Ziehen 300,20 → 400,20; zweiter Strich 300,40 → 400,40; Esc. Ebenso mit gewähltem Vektor R1 und mit gewählter Bitmap B2 | **neues Bitmapobjekt** (`[object Image]` 300,20–400,21, Pixel 300–399 in der Strichfarbe, der Endpunkt bleibt frei – ein Vektorpfad gleicher Lage füllt 300–400), oben auf der Ebene, ausgewählt, Werkzeug bleibt Pinsel; der zweite Strich malt in diese Bitmap (300,20–400,41); Esc → Zeiger, Bitmap bleibt gewählt; Esc ohne Bitmap (nichts gewählt) ändert nichts. Mit R1 gewählt: ebenfalls neue Bitmap (60,100–140,101), R1 unverändert. Mit B2 gewählt: malt hinein, B2 wächst über den Rand (Strich bei y 330 → 150,330–270,395). Beim Malen zeigt das Original um die Bitmap keinen Rahmen und keine Griffe (FV-11, eigenes Testdokument) | PM-3: Vektorpfad → **FV-11 (12.10., echtes Fenster, eigene Instanz, echte Maus):** alle Werte gleich (300,20–400,21; 300,20–400,41; 60,100–140,101; 150,330–270,395; Pixel C04040 bei gleicher Strichfarbe; Esc → Zeiger mit Bitmap gewählt; Esc ohne Bitmap: Pinsel bleibt); kein Auswahlrahmen beim Malen. Gerendert wird der Strich mit dem Strich der Werkzeugvorgabe (ohne Renderer headless: runde Spitze). Einmal beobachtet (PM-3-Ablauf nach Strg+Z auf JSF-Schritten): Original malte mit gewähltem R1 in die zuletzt bemalte Bitmap – nicht wiederholbar, nicht nachgebaut. **PM-4 (12.10., echtes Fenster, Brücke, eigene Instanz, Debug-Build `11e454c`), Kurzprüfung:** Pinsel ohne Auswahl 300,20 → 400,20 legt die Bitmap 300,20–400,21 an (Werkzeug bleibt Pinsel), Esc → Zeiger, Bitmap bleibt gewählt | ✅ gemessen (FV-11) | a_L35_pinsel, fv11_pinsel, pm4_L36_pinsel |
| L37 | Radiergummi (13 / Rand 2 / 100 %) waagerecht durch die gewählte Bitmap B2 bei y 367; Einzelklick 185,367; Esc | Spalte x 185: voll gelöscht 361–371, Übergang 360 (FFCECE), 372 (FFCECE), 373 (FF2020); Zeile: gelöscht 154–264, Übergang 153, 265–266; Strg+Z stellt her. Einzelabdruck (FV-11): Mitte auf Pixel 184,366 (= Mauspunkt − ½), Raster 178–191 × 360–373, voll bis Abstand 5, bei 6 noch 86/255, bei 7 nur rechts/unten 9/255. Esc (auch ohne Malen) → Zeiger, B2 bleibt gewählt; Radiergummi mit gewähltem Vektor R1: hebt nur die Auswahl auf, Esc danach ohne Wirkung | PM-3: Kante 1 px schmaler → **FV-11 (echtes Fenster):** Strich Spalte 185 voll 361–371, 360/372 FFC6C6, 373 FF2B2B; Zeile 154–264, Übergang 153, 265, 266; Einzelabdruck 77/86 bzw. 10/9 an denselben Stellen; Esc und Vektorfall gleich | ✅ gemessen (FV-11) | a_L36_radiergummi, fv11_radiergummi |
| L38 | Weichzeichner (13 / 100 / Intensität 50) senkrecht an der Farbgrenze rot/blau x = 210; Einzelklick 210,367 und 180,341; Esc | 208 FC0002, 209 D60028, 210 2800D6, 211 0200FC; Bitmap wächst um 1 px nach oben (150,339–270,395). Einzelabdruck (FV-11): an der Farbkante je 15/255, an der Oberkante hellt die Kante um 30/255 auf und die Bitmap wächst auf 339; Esc → Zeiger, B2 gewählt | PM-3: stärker, wächst nicht → **FV-11 (echtes Fenster):** Strich 208 FB0004, 209 D5002A, 210 2800D7, 211 0300FC, Bitmap 150,339–270,395; Einzelabdruck 15/240 und 29/30, Wachsen auf 339 gleich; Esc gleich | ✅ gemessen (FV-11) | a_L37_weichzeichner, fv11_weichzeichner |
| L39 | Stempel (16 / Rand 50): Alt+Klick Quelle 180,367 (rot), Ziehen 240,350 → 240,385 (blau), danach Esc; Einzelklick 240,367 | Kern rot 235–244, Übergang 232–234 / 245–247 (4000BF, A1005E, F5000A), 248 0F00F0; senkrecht 342 1400EB, 343 3D00C2, 344 A60059, Kern 345–377; nach Esc Werkzeug **Zeiger**, B2 gewählt. Einzelabdruck (FV-11): Mitte genau auf dem Mauspunkt (gerade Größe), 232 → 20, 233 → 61, 234 → 158; transparente Quellpixel lassen das Ziel stehen | PM-3: weichere Kante, Esc ohne Wirkung → **FV-11 (echtes Fenster):** Kern 235–244, 232 4C00B3, 233 980067, 234 EC0013 (spiegelbildlich 245–247), 248 2200DD (Rest: Übergang 4–7 % schwächer bzw. stärker als im Original), senkrecht 342 1700E8, 343 3C00C3, 344 A2005D, Kern 345–377; Einzelabdruck 23/60/152; Esc → Zeiger, B2 gewählt | ✅ gemessen (FV-11) | a_L38_stempel, fv11_stempel |

## 6. Mauszeiger (echte Maus, Bild `a_400_mauszeiger.png`)

Das Original zeichnet einige Zeiger invertiert (XOR); auf weißem Grund sind sie im Bild kaum zu sehen (Auswahlrahmen, Rechteck, Pinsel, Text). Diese Zeiger sind hier nicht bewertet.

| Nr | Lage | Original | Firepixel | |
|---|---|---|---|---|
| K1 | Zeiger über leerer Leinwand / über Objekt / im gewählten Objekt / mit Alt | schwarzer Pfeil | schwarzer Pfeil | ✅ |
| K2 | Zeiger über Eckgriff eines gewählten Rechtecks | Pfeil ohne Zusatz, **weiß mit schwarzem Rand** | schwarzer Pfeil ohne Zusatz (PM-1, echtes Fenster, `a_401_mauszeiger_nachmessung`) → **FV-6:** über Eckgriffen weißer Pfeil mit schwarzem Rand, über Kantengriffen schwarz (K3), echtes Fenster `phase5_fv6_mauszeiger_k2_k3_k9` | ✅ gemessen (FV-6) |
| K3 | Zeiger über Kantengriff | Pfeil | Pfeil (PM-1, echtes Fenster) | ✅ |
| K4 | Teilauswahl über Objekt | weißer Pfeil | weißer Pfeil mit Zusatz | ✅ |
| K5 | Zoom / Zoom mit Alt | Lupe + / Lupe − | Lupe + / Lupe − | ✅ |
| K6 | Hand-Werkzeug bzw. Leertaste, ohne Maustaste | **offene** Hand | offene Hand, auch mit Leertaste (PM-1, echtes Fenster; Zeichnung etwas gröber) | ✅ |
| K7 | Stift über leerer Leinwand | Federspitze | Federspitze (PM-1, echtes Fenster) | ✅ |
| K8 | Füllwerkzeug über leerer Leinwand (kein Objekt) | Verbotszeichen ⊘ | ⊘ (PM-1, echtes Fenster) | ✅ |
| K9 | Zuschneiden | Zuschneide-Symbol: zwei kräftige Winkel (2 px) mit Diagonale | dünnes Symbol (1 px), ohne Diagonale, mit Kreuz in der Mitte (PM-1, echtes Fenster) → **FV-6:** pixelgleich zum Original (16×16-Raster: zwei 2-px-Winkel, 1-px-Diagonale links oben), echtes Fenster `phase5_fv6_mauszeiger_k2_k3_k9` | ✅ gemessen (FV-6) |
| K10 | Radiergummi über Bitmap | Kreis in Pinselgröße | Kreis in Pinselgröße mit Mittelkreuz | ✅ |
| K11 | Zauberstab, Pipette | Zauberstab / Pipette | Zauberstab / Pipette (eigene Form) | ✅ |

## Zählung (laufend)

| | Anzahl |
|---|---|
| geprüfte Elemente | 126 Zeilen (rund 320 einzelne Felder, Vorgaben, Abläufe bzw. Zeiger) |
| ✅ | 123 |
| ⚠️ | 1 |
| ❌ | 2 |
| ⏳ | 0 |

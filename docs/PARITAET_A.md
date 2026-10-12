# Funktionsparität A: Werkzeuge, Leinwand, Inspektor (Agent UX-A, Nacht 11./12.10.2026)

Auftrag: „Vergleich mal alle Funktionen vom Original und Firepixel, damit sie wirklich 1:1 sind.“ Für die Nacht hat der Nutzer echte Maus- und Tastatureingabe freigegeben.

## Übergabe (Stand 12.10.2026, Arbeit auf Nutzerwunsch angehalten)

**Zahlen je Bereich** (117 Zeilen, neu gezählt von PM-1 am 12.10. nach dem Stand der Tabellen inkl. FV-1/FV-3-Nachmessungen; T, L und K nachgeführt von FV-6; Abschnitte 1–4 von PM-2 am 12.10. in beiden Programmen im echten Fenster nachgemessen, Firepixel-Release vom Stand `771bf08` + Arbeitsstand; ✅ gleich, ⚠️ Abweichung im Detail, ❌ fehlt bzw. falsch, ⏳ nicht fertig gemessen):

| Bereich | ✅ | ⚠️ | ❌ | ⏳ |
|---|---|---|---|---|
| 1 Inspektor je Werkzeug (W1–W38) | 24 | 13 | 1 | 0 |
| 2 Feldverhalten (F1–F15) | 13 | 1 | 1 | 0 |
| 3 Text: Schriftgröße (T1–T6) | 6 | 0 | 0 | 0 |
| 4 Inspektor je Objektart (O1–O17) | 12 | 4 | 1 | 0 |
| 5 Leinwand/Werkzeugverhalten (L1–L30) | 20 | 4 | 6 | 0 |
| 6 Mauszeiger (K1–K11) | 11 | 0 | 0 | 0 |
| **Summe** | **86** | **22** | **9** | **0** |

**PM-2 (12.10.):** Vorher in 1–4: 31 ✅ / 18 ⚠️ / 27 ❌, nachher 55 ✅ / 18 ⚠️ / 3 ❌. Neue Befunde in `BEFUNDE.md` (Kennung „von PM-2“): LW 1 (W8), IBD 14 (F10, F7, W14, W10/W27/W28, W16, W19, W21, W22, W24, W26, W29, O2, O8, O16), EB 1 (prefs.json W8). W15, W17 und O6 hängen an den offenen IBD-Befunden von EB-1, O11 am offenen AU-Befund. **F10 im Original neu gemessen:** Strg+A im Zahlenfeld markiert den Feldtext (die frühere Angabe „Alles auswählen“, nach der FV-1 umgebaut hat, ließ sich in 3 Läufen nicht bestätigen). Messhinweise: Firepixel zeichnet den Inspektor nach einem Werkzeugwechsel (auch per echtem Klick) erst nach 0,3–1,5 s neu; Fotos früher zeigen noch Werte des vorigen Werkzeugs – mindestens 1,5 s warten. Im Original übernimmt ein noch fokussiertes, geändertes Inspektorfeld seinen Text beim Klick auf ein anderes Objekt – vor jeder Feldprobe die Objekte neu anlegen. Skripte: `..\re\_testdateien\ux\uxa\werkzeug\pm2\` (`env.ps1`, `sitz.ps1`, `w.ps1`, `o.ps1`, `f.ps1`, `f2.ps1`, `x.ps1`, `rc.ps1`), Ablauf wie unten.

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
| W1 | alle Bitmap- und Auswahlwerkzeuge (Rahmen, Lasso, Zauberstab, Pinsel, Bleistift, Radiergummi, Weich-/Scharfzeichner, Abwedler, Nachbelichter, Verwischen, Stempel, Farbe ersetzen, Rote Augen, Pipette, Füllwerkzeug, Verlauf) | 2. Zeile rechts | leer (Masken-Teilfeld 14119 erscheint nur bei Maskenbearbeitung) | PM-2 (12.10., echtes Fenster): Teilfeld nicht mehr sichtbar (Rahmen, Lasso, Zauberstab, Pinsel, Radiergummi, Weich-/Scharfzeichner, Abwedler, Nachbelichter, Verwischen, Stempel, Farbe ersetzen, Rote Augen, Pipette, Füllwerkzeug, Verlauf) | ✅ | a_W1_auswahlrahmen, a_W1_W3_radiergummi |
| W2 | Radiergummi, Weichzeichner, Scharfzeichner, Abwedler, Nachbelichter, Verwischen, Farbe ersetzen | „Form“ rund/eckig | gewählte Form als gedrückter Knopf erkennbar (Rahmen) | PM-2 (12.10., echtes Fenster): gewählte Form als gedrückter Knopf mit blauem Rand erkennbar | ✅ | a_W1_W3_radiergummi, a_W4_weichzeichner, a_W6_verwischen |
| W3 | Radiergummi | Größe / Rand | 13 / 2 (PREF `EraserToolDiameter=13`, `SoftEdge=20` → 2) | PM-2 (12.10., echtes Fenster): 13 / 2 | ✅ | a_W1_W3_radiergummi |
| W4 | Weichzeichner, Scharfzeichner | Größe / Rand / Intensität | 13 / 100 / 50 (PREF) | PM-2 (12.10., echtes Fenster): 13 / 100 / 50 (beide) | ✅ | a_W4_weichzeichner |
| W5 | Abwedler, Nachbelichter | Größe / Rand / Belichtung / Bereich | 13 / 100 / 50 / Mitteltonfarben | PM-2 (12.10., echtes Fenster): 13 / 100 / 50 / Mitteltonfarben (beide) | ✅ | a_W5_abwedler |
| W6 | Verwischen | Größe / Rand / Druck / Farbe | 22 / 50 / 50 / Farbfeld zeigt Farbe | PM-2 (12.10., echtes Fenster): 22 / 50 / 50; Farbfeld grau – die grüne Farbe im Original stammt aus dessen gespeicherten Voreinstellungen (`SmudgeToolFingerPaintColor` hat keinen Werkswert), Zustand, kein Befund | ✅ | a_W6_verwischen |
| W7 | Stempel | Größe / Rand | 16 / 50 (PREF `StampToolSize=16`, `Edge=500`) | PM-2 (12.10., echtes Fenster): 16 / 50 | ✅ | a_W7_stempel |
| W8 | Farbe ersetzen | Größe / Toleranz / Stärke | 13 / 32 / 200 | PM-2 (12.10., echtes Fenster): 13 / 32 / **100**; „Von“-Farbe schwarz, „in“ grau (Original #00FF00 / #FF0000) – Befunde PM-2 (LW W8, EB prefs.json) | ❌ | a_W8_farbe_ersetzen |
| W9 | Rote Augen entfernen | Toleranz / Stärke | 50 / 50 | PM-2 (12.10., echtes Fenster): 50 / 50 | ✅ | a_W9_rote_augen |
| W10 | Rote Augen entfernen | Name im Kopf | zweizeilig „Rote Augen / entfernen“ | PM-2 (12.10., echtes Fenster): unverändert „Rote Augen en…“ – Befund PM-2 (IBD W10/W27/W28) | ⚠️ | a_W9_rote_augen |
| W11 | Freiform | Größe / Druck / Vorschau | 72 / ☑ / ☑ | PM-2 (12.10., echtes Fenster): 72 / ☑ / ☑ | ✅ | a_W11_freiform |
| W12 | Bereich umformen | Größe / Stärke / Druck Größe / Druck Stärke | 72 / 80 / ☑ / ☐ | PM-2 (12.10., echtes Fenster): 72 / 80 / ☑ / ☐ (Kopf gekürzt, siehe W28) | ✅ | a_W12_bereich_umformen |
| W13 | Pfadausbesserung (additiv/subtraktiv) | Druck / Geschwindigkeit / Rate | ☑ / ☑ / 10 | PM-2 (12.10., echtes Fenster): ☑ / ☑ / 10 (additiv und subtraktiv) | ✅ | a_W13_pfadausbesserung |
| W14 | Text | Schriftfamilie | Myriad Pro (Vorgabeschrift) | PM-2 (12.10., echtes Fenster): Myriad Pro; aber Schnitt „**Bold**“ statt „Regular“ (Text ist nicht fett) – Befund PM-2 (IBD W14) | ⚠️ | a_W14_W18_textwerkzeug |
| W15 | Text | Laufweite, Zeilenabstand (+Einheit), Absatzeinzug, Abstand vor/nach, Horizontalskala, Grundlinie, Anti-Alias, Autom. Unterschneidung | alle **bedienbar**, bevor man tippt (Vorgabe für den neuen Text) | PM-2 (12.10., echtes Fenster): Laufweite, Zeilenabstand + Einheit, Einzug, Abstand vor/nach, Horizontalskala bedienbar; **Anti-Alias und Autom. Unterschneidung grau** (Anbindung offen, IBD-Befund von EB-1) | ⚠️ | a_W14_W18_textwerkzeug |
| W16 | Text | Kurzzeichen Fett/Kursiv | „F“ / „K“ (deutsch) | PM-2 (12.10., echtes Fenster): unverändert „B“ / „I“ – Befund PM-2 (IBD W16) | ⚠️ | a_W14_W18_textwerkzeug |
| W17 | Text | Knopf Textausrichtung (horizontal/vertikal, „ab/cd“) neben U | vorhanden | PM-2 (12.10., echtes Fenster): Knopf „ab/cd ▾“ vorhanden, ohne Auswahl aber grau (Anbindung offen, IBD-Befund von EB-1) | ⚠️ | a_W14_W18_textwerkzeug |
| W18 | Text | Textfarbe | eigenes Farbfeld des Textwerkzeugs (schwarz) | PM-2 (12.10., echtes Fenster): eigenes Farbfeld des Textwerkzeugs, schwarz | ✅ | a_W14_W18_textwerkzeug |
| W19 | Polygon | „Winkel“ und „Autom.“ bei Form = Polygon | bedienbar (Winkel 38, Autom. ☑) | PM-2 (12.10., echtes Fenster): unverändert grau – Befund PM-2 (IBD W19) | ⚠️ | a_131 |
| W20 | Linie, Stift, Vektorpfad, Rechteck, Ellipse, Polygon, Auto-Form, Pinsel | Strichposition innen/mitte/außen im Werkzeug-Inspektor | nicht vorhanden (nur am Objekt) | PM-2 (12.10., echtes Fenster): keine Knöpfe mehr im Werkzeug-Inspektor (Linie, Stift, Rechteck, Ellipse, Polygon, Auto-Form, Pinsel) | ✅ | a_W20_rechteck_werkzeug |
| W21 | alle Vektorwerkzeuge, Füllwerkzeug, Verlauf | Textur bei „keiner“ | zeigt Texturnamen („Ader“) mit 0 % | PM-2 (12.10., echtes Fenster): unverändert „Ohne“ (auch am gewählten Rechteck) – Befund PM-2 (IBD W21) | ⚠️ | a_W20_rechteck_werkzeug |
| W22 | Rechteck, Ellipse, Auto-Form, Rahmen, Lasso, Zuschneiden, Exportbereich | B/H/X/Y | Felder weiß/bedienbar (leer) | PM-2 (12.10., echtes Fenster): unverändert grau – Befund PM-2 (IBD W22) | ⚠️ | a_W1_auswahlrahmen, a_W20_rechteck_werkzeug |
| W23 | Rechteck | „Eckrundung des Rechtecks“ im Werkzeug | Feld 14795 und Beschriftung 15079 existieren (Wert 0, Lage 536,1054 im Inspektor), sind aber **unsichtbar** (`IsWindowVisible` = 0); der Werkzeug-Inspektor zeigt keine Rundung (PM-1, frisch gestartetes Original) | FV-7: Zeile entfernt, Werkzeug-Inspektor ohne Rundung wie im Original; die Rundung des gewählten Rechtecks (O1) bleibt (Test `inspector_paritaet::w23_rectangle_tool_without_roundness_row`) | ✅ | a_W23_rechteck_werkzeug_inspektor |
| W24 | Auswahlrahmen | Feste Breite/Höhe | Symbole ↔ ↕, Werte 0/0, grau bei „Normal“ | PM-2 (12.10., echtes Fenster): unverändert „B“/„H“ 64/64 (grau) – Befund PM-2 (IBD W24) | ⚠️ | a_W1_auswahlrahmen |
| W25 | Verlauf | „Auswahl füllen“ / Toleranz | ☑ / Toleranz grau (hängt davon ab) | PM-2 (12.10., echtes Fenster): ☑ / Toleranz grau; Füllwerkzeug ☐ / Toleranz bedienbar | ✅ | a_W25_verlauf |
| W26 | Verlauf | Füllfarbfeld | zeigt den Verlauf | PM-2 (12.10., echtes Fenster): unverändert grau – Befund PM-2 (IBD W26) | ⚠️ | a_W25_verlauf |
| W27 | Auto-Form | Kopf | Name der gewählten Form („Abgefastes Rechteck“) | PM-2 (12.10., echtes Fenster): unverändert „Automatische …“ – Befund PM-2 (IBD W10/W27/W28) | ⚠️ | a_132 |
| W28 | Pfadausbesserung, Polygon-Segment, Hotspot-Rechteck | Name im Kopf | zweizeilig voll | PM-2 (12.10., echtes Fenster): unverändert gekürzt („Pfadausbesser…“, „Polygon-Segm…“, „Hotspot-Recht…“, auch „Bereich umfor…“) – Befund PM-2 (IBD W10/W27/W28) | ⚠️ | a_W13_pfadausbesserung, a_W12_bereich_umformen |
| W29 | Zeiger (Dokument) | Exportformat-Liste | Format des Dokuments | PM-2 (12.10., echtes Fenster): Knopf „Bearbeiten…“ weiter vorhanden – Befund PM-2 (IBD W29); das Format selbst ist nicht vergleichbar (Testdokument im Original PNG32, neues Firepixel-Dokument GIF Adaptiv 256) | ⚠️ | a_100 |
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
| F7 | „7“ + Esc, dann Klick auf die Leinwand | 7 wird übernommen (Esc verwirft nicht) | PM-2 (12.10., echtes Fenster): Esc verwirft nicht mehr (Esc + Tab übernimmt 7), aber **Esc + Klick auf die Leinwand verwirft** (Deckkraft bleibt 100) – Befund PM-2 (IBD F7) | ⚠️ | a_F7_esc_dann_leinwand |
| F8 | „60“ + Tab | übernommen | übernommen | ✅ | |
| F9 | „40“, Klick auf die Leinwand | übernommen | übernommen | ✅ | |
| F10 | Strg+A im Feld | **markiert den Feldtext**, Objektauswahl bleibt; getippter Wert bleibt und wird mit Eingabe übernommen (PM-2, 12.10., 3 Läufe; die frühere Angabe „Alles auswählen“ ließ sich nicht bestätigen) | PM-2 (12.10., echtes Fenster): **wählt alle Objekte** (9), der getippte Wert geht verloren – Befund PM-2 (IBD F10) | ❌ | a_F10_strg_a_im_feld |
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
| O2 | Rechteck | Vorschaubild oben links | Objekt an seiner Lage im verkleinerten Dokument | PM-2 (12.10., echtes Fenster): unverändert formatfüllend; bei Mehrfachauswahl Schachbrett statt keinem Bild – Befund PM-2 (IBD O2) | ⚠️ | a_O14_O16_mehrfachauswahl |
| O3 | Ellipse (Pfad) | alle Felder inkl. „Pinselstrich bearbeiten“, Stil | so | so | ✅ | a_301 |
| O4 | Text | Zeilenabstand eines neuen Textes | 120 % | PM-2 (12.10., echtes Fenster): 120 % | ✅ | a_O4_O8_text |
| O5 | Text | Grenzen eines Textrahmens 50,170–200,200 | X 48, Y 168, B 154, H 28 (2 px Rand um den Textbereich) | PM-2 (12.10., echtes Fenster): X 48, Y 168, B 154, H 28 (EB-1: angezeigte Grenzen ± 2 px) | ✅ | a_O4_O8_text |
| O6 | Text | Anti-Alias / Autom. Unterschneidung eines per Skript angelegten Textes (`addNewText(…, true)` = aus den Voreinstellungen) | „Anti-Alias glatt“ / ☐ | PM-2 (12.10., echtes Fenster): Autom. Unterschneidung ☐ wie im Original; Glättung zeigt weiter „Kein Anti-Alias“ (JSF `antiAliased` false; Zuordnung `TAAi` im Inspektor falsch, IBD-Befund von EB-1; AU-Befund O11/O6) | ⚠️ | a_O4_O8_text |
| O7 | Text | Pinselstrich-Feld (Stift + Farbe) in Zeile 2 | vorhanden | PM-2 (12.10., echtes Fenster): Stift + Strichfarbfeld in Zeile 2 vorhanden | ✅ | a_O4_O8_text |
| O8 | Text | Horizontalskala | „100%“ | PM-2 (12.10., echtes Fenster): unverändert „100“ – Befund PM-2 (IBD O8) | ⚠️ | a_O4_O8_text |
| O9 | Gruppe | „Gruppieren (2)“, Füllung/Strich/Deckkraft/Filter/Stil | so | so | ✅ | a_303 |
| O10 | Bitmap | Deckkraft, Mischmodus, Filter, Stil, B/H/X/Y | so | so | ✅ | a_304 |
| O11 | Segment (per Skript `addNewHotspot("slice", …)`) | Objektart | Segment (Typ Vordergrundbild, Exportformat, Hyperlink, Alt, Ziel; automatischer Name „_r2_c2“) | PM-2 (12.10., echtes Fenster): unverändert, `addNewHotspot("slice", …)` ergibt `[object Hotspot]` (Original `[object SliceHotspot]`); AU-Befund O11/O6 weiter offen | ❌ | a_305 |
| O12 | Hotspot | Form, Farbe, Hyperlink, Alt, Ziel | so | so | ✅ | a_306 |
| O13 | Symbol | Deckkraft, Mischmodus, Filter, Stil | so | so | ✅ | a_307 |
| O14 | 2 Rechtecke, Deckkraft 100 und 50 | Deckkraft-Feld | **leer** (gemischte Werte) | PM-2 (12.10., echtes Fenster): leer | ✅ | a_O14_O16_mehrfachauswahl |
| O15 | 2 Rechtecke | Füllfarbfeld | Farbe des zuletzt gewählten Objekts | PM-2 (12.10., echtes Fenster): blau = Farbe des zuletzt gewählten Objekts R2 (JSF `fw.selection` liefert die Reihenfolge allerdings R1, R2; Original R2, R1) | ✅ | a_O14_O16_mehrfachauswahl |
| O16 | 2 Rechtecke | Stil-Liste | leer (gemischt) | PM-2 (12.10., echtes Fenster): unverändert „Kein Stil“ (beide Objekte ohne Stil; das Original lässt die Liste trotzdem leer) – Befund PM-2 (IBD O16) | ⚠️ | a_O14_O16_mehrfachauswahl |
| O17 | Segmente und Hotspots | Überlagerung auf der Leinwand (grün/cyan) | sichtbar | PM-2 (12.10., echtes Fenster): Hotspots cyan sichtbar; ein mit dem Segment-Werkzeug gezogenes Segment grün mit roten Segment-Hilfslinien wie im Original (das JSF-„Segment“ S ist wegen O11 ein Hotspot) | ✅ | a_O17_segment_hotspot |

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
| L16 | Rechteck, Alt gehalten | von der Mitte aus: 180–300 × 125–175 | wie ohne Alt (250,150–300,175) | ⚠️ **PM-1, Fangregel im Original (14 Fälle, `uxp_pm_l16.ps1`):** Start- und Endpunkt rasten getrennt in x und y an Kanten und Mitten anderer Objekte ein, Fangabstand **≤ 5** px (5 rastet, 6 nicht); mit **Umschalt rastet nichts** (weder Start noch Ende, auch mit Alt+Umschalt nicht); mit Alt rastet der Mittelpunkt wie sonst der Start, das Ende ebenfalls. Firepixel (echtes Fenster): bei genau 5 px kein Fang (115–175), mit Umschalt rastet es weiter (240,147 → 240,150; Ende 298 → 300), ein Ende bei y = 173 rastet an 170 statt an der näheren 175 (halber Pixel Versatz); Abstand 1–4 und ≥ 6 gleich. Befunde LW. **✅ FV-6 nachgemessen** (echtes Fenster, eigene Instanz, `uxp_pm_l16`-Fälle, Seite jetzt auf ganzen Pixeln 497,322): A1 240,150–300,175, A2 Umschalt 240,145–300,205, A3 Alt 180,125–300,175, A4 180,85–300,205, B1 180,125–300,175, B2 180,113–300,175, B3 (Gleichstand) 180,111–300,175, C2 Umschalt 240,147–300,207, Alt+Umschalt 180,87–300,207, Umschalt Ende 298,160 → 240,135–298,193 – alle wie im Original; Fälle am Text T1 hängen an dessen Grenzen (2-px-Rand, Befund EB). Nebenbefund: bei SendInput mit `SetCursorPos` geht selten (1 von 5) die letzte Bewegung vor dem Loslassen verloren (Ende 297 statt 300, nur ohne Fang sichtbar; winit meldet die Lage des Loslassens aus der letzten Bewegung) | |
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
| L27 | Zeiger: Doppelklick auf Bitmap | Bitmapmodus, Werkzeug wechselt zum Auswahlrahmen; danach Esc: Werkzeug bleibt Auswahlrahmen, Bitmap bleibt gewählt | Bitmapmodus und Auswahlrahmen (PM-1, echtes Fenster); danach Esc: Werkzeug wechselt zum Zeiger → **FV-6:** Esc ohne Auswahlrahmen ohne Wirkung; echtes Fenster: Doppelklick → B gewählt, `tool.marquee`; Esc → B gewählt, `tool.marquee` | ✅ gemessen (FV-6) | a_L27_doppelklick_bitmap |
| L28 | Zuschneiden: Rahmen ziehen, Esc | Leinwand unverändert, Werkzeug wechselt zum Zeiger | Leinwand unverändert, Werkzeug bleibt Zuschneiden | ✅ nachgemessen (FV-3, echtes Fenster): Werkzeug Zeiger, Breite 600 | |
| L29 | Zuschneiden: Rahmen 20,20–300,200, Eingabe, Strg+Z | 280 × 180, Rückgängig stellt 600 × 400 her | ebenso | ✅ | |
| L30 | Frei transformieren (Strg+T) an R1 50,50–150,120, Eckgriff von 151,121 nach 191,151 | **Ecke skaliert immer proportional** (50,50–193,150, größerer Faktor), Umschalt ändert nichts; Alt = proportional von der Mitte (7,20–193,150); Kante rechts → 191 (Zeigerlage); die Änderung wirkt schon beim Ziehen; **Esc behält die Änderung** und beendet den Modus (Werkzeug Zeiger); Eingabe und Doppelklick innen übernehmen, Strg+Z = ein Schritt; Ziehen am Mittelpunkt verschiebt nur den Drehpunkt (`uxp_pm_3.ps1`) | Ecke unproportional (190,150), nur mit Umschalt proportional (193,150); Alt 10,20–190,150 (Alt+Umschalt 7,20–193,150 gleich); Kante → 188; nach dem Loslassen zeigt die Leinwand bis zur Eingabe die alte Größe; Esc verwirft, Werkzeug bleibt „Skalieren“; Eingabe/Doppelklick, ein Schritt und Mittelpunkt gleich (PM-1) → **FV-6, echtes Fenster (`uxp_pm_3`-Fälle):** Ecke 50,50–193,150, Umschalt 193,150, Alt 7,20–193,150, Alt+Umschalt gleich, Kante rechts 191; nach dem Loslassen zeigt die Leinwand die neue Größe; Esc → 50,50–193,150 und `tool.pointer`, Klick danach wählt; Strg+Z = ein Schritt zurück auf 150,120; Eingabe und Doppelklick innen übernehmen (Zeiger). Gilt für Skalieren/Neigen/Verzerren (Esc behält, A für Neigen/Verzerren: nicht gemessen); JSF `pixelRect` meldet während der offenen Vorschau noch die alte Größe (im Original nicht gemessen) | ✅ gemessen (FV-6) | a_L30_frei_transformieren, phase5_fv6_frei_transformieren_nach_loslassen |

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
| geprüfte Elemente | 117 Zeilen (rund 300 einzelne Felder, Vorgaben, Abläufe bzw. Zeiger) |
| ✅ | 86 |
| ⚠️ | 22 |
| ❌ | 9 |
| ⏳ | 0 |

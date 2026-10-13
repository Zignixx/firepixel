# RUN-2: Läufer-Befunde behoben (13.10.2026)

Grundlage: `docs\parity\TWEAKS_2026-10-13.md`, Abschnitt „Läufer-Befunde“ 1–8. Geändert nur `tools\parity\runner\*`
und `tools\parity\scripts\*.json` (kein Rust). Getestet mit Einzel-Einheiten, kein Volllauf.

## Ursachen und Änderungen

| Punkt | Ursache | Änderung |
|---|---|---|
| 1 Menübefehle im Original ohne Wirkung | Die Drehbücher nutzen `menu` (bisher nur Öffnen/Hover), der letzte Eintrag wurde nie ausgeführt. Zusätzlich: `split_path` trennte an `/` („Farbton/Sättigung“ → „Farbton“), Dialoge des Originals ignorieren Enter/Escape als Taste | `menu` führt einen Blatt-Eintrag am Pfadende aus (Hover + Klick, bei offen gebliebenem Menü Enter, sonst Fehler), Untermenü bleibt offen; `menu_open`/`hover_menu` nur öffnen. Abgekürzte Einträge („Gaußscher Weichz.“) werden gefunden, Untermenü-Einträge haben Vorrang vor gleichnamigen Einträgen des Elternmenüs. `/` trennt nur ohne `>`. `key Enter/Escape` bei offenem Dialog drückt dessen OK/Abbrechen. Firepixel: `menu` klickt Blatt-Einträge ebenso (Untermenü = neue Texte neben dem Eintrag) |
| 2 Filter nicht angewendet | gleiche Ursache wie 1 (Menü nur geöffnet) | wie 1; nach ausgeführtem Menübefehl Wartezeit, bis kein Menü mehr offen ist. Export-Hashes ändern sich jetzt (s. u.) |
| 3 Firepixel-Profil verschmutzt | Profil wurde über alle Einheiten weiterverwendet | frisches Profil bei jedem Start (= jede Einheit), `fp.fresh_profile` (Vorgabe an) |
| 4 Original: schwebende Bedienfelder | schwebende Bedienfeldgruppen (eigene Fenster) blieben über der Leinwand | am Einheitenbeginn alle schwebenden Gruppen schließen (eigener Schließknopf, sonst WM_CLOSE); vor jedem Schritt die, die die Dokumentansicht verdecken (`orig.floating_dock_class`) |
| 5 Firepixel `select` / Strichbreite | Auswahl über die Brücke; Inspektorfeld im Original übernimmt synthetisch getippte Werte nicht | `select` per `run_script` mit demselben JSF wie im Original (genau die benannten Objekte, Prüfung der Anzahl, Brücke nur Rückfall). Original: nach `select` 0,6 s Wartezeit (Inspektor baut Felder neu), Feldsuche wartet bis 2,4 s und sucht auch per Rechteck (`field_at`). Inspektorfelder ohne Übernahme bekommen in `ui_map.json` ein `jsf_set` (Strichbreite, Rand, Deckkraft, Name, X/Y/B/H), das nach der Eingabe den Wert per JSF setzt (im Protokoll `set_field_via: jsf`) |
| 6 Aufnahmebereiche Firepixel | Der Barrierefreiheitsknoten „Titelleiste …“ deckt nur den Titeltext ab; schwebende Bedienfelder wurden als ganze Leinwandspalte aufgenommen; Differenzbild enthielt nachlaufende Neuzeichnungen | Dialograhmen aus dem Fensterbild (vom Titel bis zur Rahmenlinie, inkl. Schließknopf und Knopfzeile, Knöpfe nie abgeschnitten); Datei-Dialog in der App am Titeltext verankert. Schwebende Gruppe = eigener Rahmen. Vorher-Bild für die Differenzbox erst 0,3 s nach der vorigen Aktion |
| 7 Firepixel `record_ui` | nur `ui_state` | `dialog.fields/open/title/text`: Titel, Texte in Lesereihenfolge, Bedienelemente (Rolle, Text, Zustand); `menu.open`/`log_menu_items`: Texte der offenen Menüs je Spalte mit Kürzeln (Menübereich aus der Differenz); `{"focus_cycle": n}` beidseitig (Original: fokussiertes Fenster per GetGUIThreadInfo) |
| 8 übrige Fehlerklassen | `input refused: no foreground window` (kurz leerer Vordergrund nach Dialog/Menü), Klasse `SWFPLAYER*` (Flash-Bedienfelder) abgewiesen; `record_ui {dict}`; `fw.historyPalette.getSteps` gibt es in keinem Programm | Eingabe wartet bis 2 s auf den Vordergrund und gibt ihn dann dem Hauptfenster zurück (fremde Prozesse nie); Fensterklassen `OWL.*`/`SWFPLAYER*` des Zielprozesses erlaubt; `record_ui` mit Objekt; 33 Ausdrücke in 10 Drehbüchern auf `getStepCount()` + `getStepsAsJavaScript()` umgestellt. „choose: target opened no menu“: Kombifelder mit nur einem Eintrag (Strichart, Textur) öffnen jetzt per Klick ihr Popup-Menü und werden wie ein Menü durchlaufen. Dialoge, deren Abbrechen eine Fehlermeldung auslöst, werden wiederholt geschlossen |

## Testläufe (vorher = Lauf vom 13.10. 18:18–20:44, nachher = RUN-2)

| Einheit | Bildpaare vorher → nachher | Fehler Original | Fehler Firepixel | Werte gleich |
|---|---|---|---|---|
| menu-modify-a | 6 → 10 | 8 → 1 | 2 → 0 | 2/14 → 5/18 |
| filters-adjust-color | 5 → 10 | 8 err + 5 unsupp. → 0 err + 1 | 3 → 1 | – (Export-Hashes) |
| inspector-stroke | 12 → 17 | 22 → 19 (+13 unsupp.) | 1 err + 34 unmapped → 0 + 4 | – (Export-Hashes) |
| tool-autoshapes | 24 → 30 | 5 → 4 | 10 → 3 | – |
| dialogs-d11-d20 | 9 → 17 | 8 → 3 | 6 → 0 | 1/3 → 3/5 |

Belege:
- Leinwand drehen: Original jetzt 600×800 wie Firepixel (vorher 800×600), Leinwandgröße 900 auf beiden Seiten,
  Leinwand zuschneiden 750×336 (Firepixel 796×336 → echter Unterschied).
- Filter: Export-Hashes ändern sich (Umkehren Original a65da0 → 636708, Firepixel a8b307 → b0e5a8; Auswahl,
  Farbfüllung ebenso). Auto-Tonwertkorrektur bleibt auf beiden Seiten gleich (Bild ohne Wirkung).
- Strichbreite w1/w10/w100: Original dc1fe9/e53776/e39da9, Firepixel 36b9c2/e93a0e/f90c37 (vorher je Seite ein Hash).
- Autoformen: Firepixel exportiert 16 verschiedene Bilder (vorher leeres Dokument 86c9d6).
- Firepixel-Dialogaufnahmen: z. B. Leinwandgröße 633–968 × 308–684 inkl. Knopfzeile, HTML-Einrichtung 492–1108 × 270–723;
  `dialog.fields` liefert Titel + Texte + Bedienelemente.

## Offene Lücken

- Strichkategorie/-art im Original: Das Untermenü der Kategorie ist owner-drawn (keine Texte) → `Unmapped` mit
  Hinweis „needs orig_index“; Drehbücher brauchen `orig_index` oder einen JSF-Weg (`dom.setBrush`). Gleiches gilt für
  Textur-/Kantenlisten.
- Fehlende Schlüssel in `ui_map.json`: `dialog.brightness`, `dialog.colorize`, `dialog.hue`, `dialog.channel`,
  `dialog.eyedropper_*`, `dialog.input_low`, `stroke_popup.*`, `dialog.ink_amount` (beide Seiten `unmapped`).
- `fx_add` im Original (Live-Filter-Knopf, 59 Fälle): Knopf liegt an der gemessenen Stelle, öffnet im Lauf aber kein
  Menü – nicht mehr untersucht.
- `repeat`-Aktion (`{"repeat": 3, "do": [...]}`) noch nicht umgesetzt.
- Tab im Original-Dialog wandert nicht (Fokus bleibt), `focus_cycle` des Originals daher wenig aussagekräftig.
- Fremde Fenster im Vordergrund (z. B. Browser) brechen Eingaben weiterhin ab (gewollt).
- Firepixel: `ui_action {panel}` schaltet nicht aus; Bedienfelder, die ein Schritt öffnet, bleiben bis zum Ende der
  Einheit (frisches Profil erst bei der nächsten Einheit).

## Volllauf

```
C:\Users\Administrator\AppData\Local\Programs\Python\Python314\python.exe -I C:\Users\Administrator\Desktop\REA\firepixel\tools\parity\runner\runner.py --all
```

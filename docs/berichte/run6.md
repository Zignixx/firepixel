# RUN-6: Runner-Abdeckung weiter erhöht (14./15.10.2026, 23:49–00:30)

Geändert nur `tools\parity\runner\*` (runner.py, orig_host.ps1, fpctl.py, ui_map.json, README.md). Keine Drehbücher,
kein Rust, kein Volllauf. Messungen über den Runner (Sperre je Einheit), Ausgabe `re\_testdateien\par\run6\run\`.

## Auswahl (per Skript gezählt)

Fehlende Paare nach Ursache aus den letzten Läufen und statische Zählung der Ziele/Verben in allen Drehbüchern
(Schritte mit Aufnahme): Werkzeuge in Klappmenüs ohne Original-Zuordnung 21, Effekt-Popup-Ziele (`fx_popup.*`) 9,
`export` 7, Ausrichten-Knöpfe in Firepixel 4 (+ 9 Wertschritte), `window_size` 2.

## Änderungen

- **Werkzeuge in Klappmenüs** (Original): `ui_map.json → tools` mit `{flyout, orig_index}`: Gedrückthalten der
  Gruppen-Schaltfläche, Eintrag per Index. Neu: Bereich umformen, Pfadausbesserung +/−, Hotspot Rechteck/Kreis/Polygon,
  Polygon-Segment, Farbe ersetzen, Exportbereich; Ziele `toolbox.slice_flyout`, `toolbox.rubberStamp_flyout`.
- **Effekt-Popup** (Original vermessen mit neuem `record_ui "dialog.controls"` = Win32-Kinder mit Klasse/Rechteck):
  `fx_popup.width/contrast/width_slider/edge_shape/button_preset/angle_dial/softness_slider/color`. Winkelrad =
  letzter Pfeilknopf (`dialog_child` mit **negativem Index**), Farbe = Farbfeld-Fenster `Afx:00400000:b`.
  Firepixel: `dialog.field/slider/combo.*`, Farbe über Beschriftung „Farbe:“ + `from_left` (neue Locator-Option).
- **Firepixel `dialog.slider.N`**: die Regler der Effekt-Dialoge liegen in der Zeichenreihenfolge vor der Titelleiste und
  wurden weggefiltert („no slider #3“); der Reihenfolge-Filter greift nur noch, wenn danach genug Treffer bleiben.
- **String-Aliasse** in `ui_map` (`"fp": "dialog.combo.1"`) führten bei `open`/`click` zu „'str' object has no
  attribute 'get'“ – `loc_of` löst Aliasse jetzt auf.
- **Ausrichten (Firepixel)**: Ursache gefunden – das Bedienfeld ohne Engine-Befehl wurde per Reiterklick gezeigt; war
  es schon vorn, klappte der Klick die Gruppe zu (jeder zweite Schritt „Links ausrichten not found“). Neu
  `panels.<id>.fp_probe`: ist die Probe-Beschriftung sichtbar, kein Klick. Zusätzlich wiederholt `find` bis 3 × 0,4 s.
- **Verb `export`** (beide Seiten): Typ „Nur Bilder“ ohne Segmente/Optionen mit den Exporteinstellungen des Dokuments
  (Original `fw.exportDocumentAs`, Endung aus `exportOptions.exportFormat`; Firepixel Brücke `export {path}`),
  Ordnerliste in `values`. HTML/Segmente/Status-/Ebenen-/Seiten-Dateien/PDF/CSS-Sprites bleiben `unsupported`.
- **Platzhalter `{side}`/`{unit}`** in allen Aktionsargumenten werden ersetzt (bisher legten beide Seiten in denselben
  Ordner `work\{side}\{unit}` ab).
- **Verb `window_size [w, h]`** (beide Seiten, Firepixel ohne Aktivierung); der nächste Schritt stellt 1600 × 1000 wieder her.
- `resize_panel` und `drag_text` bewusst **nicht** umgesetzt: sie verändern das gespeicherte Arbeitsbereich-Layout des
  Originals und würden spätere Einheiten verfälschen (bräuchten eine sichere Rücknahme).

## Kurztest (Vorschau `Firepixel-Vorschau-2026-10-14_21-48`)

| Einheit | Paare vorher (run\ bzw. run5) | nachher |
|---|---|---|
| tool-path-reshape | 9/19 | **19/19** (alle Schritte ok, Werkzeugkopf im Inspektor stimmt) |
| fx-bevel-emboss-fill | 10/15 | **14/15** (Rest: 11_ps_effects, PSD-Popup öffnet nichts) |
| export-results | 3/11 | 6/11; 01/07/08 exportieren auf beiden Seiten `par.png` |
| panel-info-align-path | 9/18 | **11/18**; alle Ausrichten-Wertschritte jetzt ok |
| inspector-web (13/14) | – | `window_size` ok, Fenster danach wieder 1600 × 1000 |

Stichproben: Kantenform-Liste (Flach … Kräuseln) und Werkzeugkopf „Pfadausbesserung – subtraktiv“ zeigen beidseitig
denselben Zustand.

**Geschätzte Gesamtabdeckung**: RUN-5 ≈ 585/831 (70 %) + ≈ 39 Schritte mit Aufnahme (Klappmenü-Werkzeuge 21,
Effekt-Popup 9, Ausrichten 4, Export 3, Fenstergröße 2) ≈ **624/831 = 75 %**.

## Offen (nach Ertrag)

1. Popup-/Dialog-Ziele ohne Vermessung: `popup.*`/`color_popup.hex` (Farbpopup des Originals ist kein `#32770` –
   `dialog.controls` erweitern auf alle Top-Level-Fenster), `optimize.*`/`preview.*`, `stroke_popup.*`,
   `dialog.color_well`, `align.anchor_list`, `info.field.*`, `path.points.*`.
2. Pfad-Bedienfeld des Originals (Symbole ohne Text): wie `orig_labels` beim Ausrichten vermessen.
3. Export-Typen mit Dialog (HTML, Segmente, Dateien je Status/Ebene/Seite, PDF, CSS-Sprites); `optimize.quality`,
   „PNG 32“ im Format-Kombifeld (Original: anderer Eintragstext).
4. `resize_panel`/`drag_text` nur mit Rücknahme des Arbeitsbereichs (z. B. Layout vorher sichern/zurückladen).

Am Ende läuft eine Original-Instanz ohne Dialog.

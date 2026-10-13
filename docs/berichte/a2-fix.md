# A2-FIX – Parity-Runner: Ausfall der Referenzseite im Volllauf (13.10.2026)

## Ursache
- Der Host wählte den Zielprozess mit „erster Prozess dieses Namens“. Jeder JSF-Aufruf startet einen
  Starter-Prozess (Exe + Skriptpfad), der das Skript an die laufende Instanz übergibt. War die Instanz
  blockiert, blieb der Starter ohne Fenster hängen – und wurde (kleinere PID) als „das Programm“
  genommen → „main window not found“, ab da scheiterte auch das Ablehnen der Dialoge.
- Auslöser 17:12 (`inspector-group-symbol`, Schritt 15 Symbol bearbeiten → 16 Datei neu öffnen):
  modale Frage „Änderungen an par_symbol_web.fw speichern?“ (Ja/Nein/Abbrechen). Hängender Starter
  PID 6404 (`…\runner_work\tmp\par_171252_244.jsf`). Ab `inspector-stroke` bis zum Ende 0 Paare.
- Gleiches Muster bereits 16:28 (dialogs-d01-d10) mit Neustart 16:31 (PID 11208).
- Zusätzlich: „Abbrechen“ wurde bei Speichern-Fragen bevorzugt (Dokument bleibt offen);
  die englischen Knöpfe „&No“ wurden nicht erkannt.

## Änderungen (`tools\parity\runner\`)
- `orig_host.ps1`: Zielprozess = Besitzer des sichtbaren Hauptfensters (Fensterklasse, über alle
  Prozesse); neue Befehle `main_pid`, `procs` (PID, Hauptfenster, Fenster, Dialogtexte, Alter,
  Befehlszeile); Dialoge auch ohne Hauptfenster; `dialog_close`: zuerst Nein/No, dann
  Abbrechen/Cancel/Schließen, nie Ja/Speichern/OK; Client-Rechteck auf Fenster begrenzt
  (Programm meldete 1600 × 1001); `combo_select`/`combo_items`: −1 eines 32-bit-Ziels
  (4294967295) korrekt umgerechnet.
- `origctl.py`: `running()` nur mit Hauptfenster; eigene hängende Prozesse ohne Fenster (Starter mit
  Skript aus dem tmp-Ordner, eigene Neustarts) werden beendet, fremde nie; kein Zweitstart bei
  laufender Instanz (fremder Prozess ohne Fenster: warten, dann Fehler); wartender Starter nach
  JSF-Timeout beendet; verlorene Übergabe (Starter beendet, Skript nicht gestartet) wird bis zu 2× neu
  übergeben, `.started`-Marker macht Doppel wirkungslos; `close_all_docs()`; sauberer `restart()`
  (Fragen ablehnen, Dokumente ungespeichert schließen, beenden, starten); leere PowerShell-Listen
  (`{}`) werden verworfen.
- `runner.py`: je Einheit Ausgangszustand (hängende eigene Prozesse beenden, Fragen ablehnen, alle
  Dokumente ohne Speichern schließen); nach `orig.max_consecutive_errors` (Standard 3) systemischen
  Fehlern in Folge (Hauptfenster weg, JSF-/Host-Timeout, Absturz, Eingabe verweigert …) sauberer
  Neustart und Fortsetzung; Bereich `popup` nur mit während des Schritts neu erschienenen Fenstern.
- `README.md`: Abschnitt Robustheit.

## „0 equal“
- `equal` heißt 0 abweichende Pixel (Schwelle 16) – bei zwei verschiedenen Programmen praktisch nie.
- Korrigiert: Fenster 1600 × 1001 ↔ 1600 × 1000 (jetzt beide 1600 × 1000); Popup-Bereich nahm
  dauerhafte kleine Fenster mit (259 × 97 bei allen Farb-Popups).
- Nur notiert (inhaltlich/Zustand): `canvas` Original 1254 × 732 ↔ Firepixel 1600 × 904 (Firepixel
  ohne angedockte Bedienfelder, im Original schwebende Bedienfelder über der Leinwand); Dialog-/
  Popup-Bereiche unterschiedlich groß (eigene Gestaltung).
- Drehbuch-Befund: `dialogs-d*` nutzen für das Original `menu "Datei > Neu..."` (nur Hover) statt
  `menu_item` → kein Dialog offen („no dialog open“); Firepixel nutzt `command`. Drehbücher anpassen.

## Test (`--units inspector-stroke,menu-edit,dialogs-d01-d10`, 18:01–18:16)
| Einheit | Paare vorher | Paare nachher |
|---|---|---|
| inspector-stroke | 0 | 12 |
| menu-edit | 0 | 4 |
| dialogs-d01-d10 | 1 | 1 (Drehbuch-Befund oben) |

Kein „main window not found“ mehr; hängende eigene Starter zweimal automatisch beendet. Am Ende eine
Original-Instanz ohne Dokument und ohne Dialog gestartet.

## Wiederholungslauf (ausgefallene Einheiten)
```
C:\Users\Administrator\AppData\Local\Programs\Python\Python314\python.exe -I C:\Users\Administrator\Desktop\REA\firepixel\tools\parity\runner\runner.py --units inspector-group-symbol,inspector-stroke,inspector-text,inspector-web,menu-commands-window-help,menu-edit,menu-file,menu-modify-a,menu-modify-b,menu-select,menu-text,menu-view,panel-info-align-path,panel-layers,panel-layers-menu,panel-library-styles,panel-misc,panel-optimize,panel-pages-states-history,panel-swatches-mixer,preview-options,preview-views,tool-autoshapes,tool-path-reshape,tool-text,tool-web,toolbox,xs-canvas,xs-context-menus,xs-file-formats,xs-preferences,xs-shortcuts,xs-windows-tabs
```
33 Einheiten, geschätzt 2–2,5 h (≈ 4–5 min je Einheit).

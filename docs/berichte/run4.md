# RUN-4: Runner-Reste vor dem Nachlauf (14.10.2026)

Geändert nur `tools\parity\runner\*` (runner.py, orig_host.ps1, ui_map.json, README.md). Kein Rust, keine
Drehbücher. Kein Volllauf.

## Ursachen und Änderungen

| Befund | Ursache (nachgestellt) | Änderung |
|---|---|---|
| F1: Original-Feld zeigt „60r“, Dialog bleibt offen | Die Dialoge des Originals übersetzen weder Enter noch Tab. SendInput-Enter erscheint als „r“, Tab als „b“ im Feld (im Original nachgestellt: `0` → `60` → `60r`, mit Tab `60b`). | `set_field` hat jetzt die Option `commit` mit den Werten `enter`, `tab` und `none` (Alias `then`). Beide Seiten enden gleich. Vorgabe: `tab` für ein Feld im offenen Dialog (Wert übernommen, Dialog bleibt offen, die nächste Aktion des Drehbuchs entscheidet), sonst `enter`. Original im Dialog: Klick, `WM_SETTEXT`, Tab = `WM_NEXTDLGCTL` (Feld übernimmt beim Fokusverlust), Enter = Tab + OK-Knopf. `key "Tab"`/`"shift+Tab"` mit Fokus im Dialog geht ebenfalls über `WM_NEXTDLGCTL`. Neue Host-Befehle `next_ctl` und `next_ctl_root`. |
| Läufer-Befund 1: Firepixel `select P1` → alle 7 Objekte gewählt | Nicht `select` selbst (nachgeprüft: wählt genau P1). `set_field inspector.stroke_edge` klickte neben das Feld (`window_rel [515, 934]` liegt auf dem Punkt-Knopf). Das folgende Strg+A landete auf der Leinwand und wählte alles. | Firepixel-`set_field` sendet Strg+A nur, wenn der Klick (notfalls ein Doppelklick) ein Textfeld fokussiert hat (`widget_events`). Sonst meldet es einen Fehler und tippt nichts. Locator `inspector.stroke_edge` fp auf `[545, 934]` korrigiert. |
| Läufer-Befund 2: Ansicht-Schalter des Originals bleiben stehen | Das Original führt Raster, Hilfslinien, Lineale usw. über Dokumente hinweg. In Firepixel gelten sie je Dokument und sind nach dem Öffnen auf Vorgabe. Beim ersten Test standen im Original noch Lineale an, intelligente Hilfslinien aus und Hilfslinien gesperrt. | Neu `ui_map.json → view_defaults` (Firepixel-Vorgaben: Raster aus, Fang Raster aus, Hilfslinien an, Fang an, Sperre aus, Lineale aus, intelligente Hilfslinien an/an, Segment-Hilfslinien an, Ränder aus). Das Original setzt sie nach jedem Öffnen einer Testdatei im selben JSF wie die Größenabfrage (kein Zusatzlauf). Beide Seiten setzen sie außerdem vor einem Schritt, wenn der vorige Schritt sie berührt hat (Menü `Ansicht`, Befehl `view.*`, JSF `setShow*`/`setSnap*`/`lockGuides`, Strg+Ä/Ü). Log: `view switches reset: …`. |
| Läufer-Befund 5: `close_all` schließt in Firepixel nichts | Es schloss nur die Dateien aus `open_testfile`. Dateien, die das Drehbuch selbst öffnet (Dateidialog bzw. JSF), blieben auf beiden Seiten offen. | `close_all` schließt auf beiden Seiten alle Dokumente ungespeichert. In Firepixel gibt es einen Fehler, wenn danach noch eines offen ist. |

## Kurztest (`--units dialogs-d21-d30,inspector-stroke,menu-view,xs-canvas` ohne 08, dazu d41-d50 15/16/18, xs-file-formats 00–05)

Neueste Vorschau `Firepixel-Vorschau-2026-10-14_00-22`. Die Teilläufe von d41-d50 und xs-file-formats liegen nur
im Scratchpad. Ihre Ordner unter `run\` wurden aus der Sicherung zurückgespielt, ebenso die Logzeile von
xs-canvas 08.

| Einheit | Status Original / FP vorher | nachher | inhaltlich |
|---|---|---|---|
| dialogs-d21-d30 | 24 ok, 3 Fehler / 27 ok | 24 ok, 3 Fehler / 24 ok, 3 unmapped | **TW-14 behoben:** D23 Original jetzt 48 pt (vorher 24), D26 R1 0,15–200,155 in beiden. Die 3 Firepixel-„unmapped“ (`no combo_box #0`, 05/09/17) sind dieselben Schritte, die im Original fehlen. Seit F1 zählt `find_role` nur noch Dialogfelder, das Drehbuch erwartet dort ein Kombifeld, das es nicht gibt. |
| inspector-stroke | 25 ok, 2 Fehler / 46 ok | unverändert | **Befund 1 behoben:** `04_edge_*` Firepixel-Auswahl vorher `[11,12,14,15,16,17,18]`, jetzt `[15]` (nur P1). Die 2 Original-Fehler (JSF-Fehler `03_width_clamp`, `12_text_stroke`) sind alt. |
| menu-view | 24 ok, 1 Fehler / 24 ok, 1 Fehler | unverändert | Rücksetzungen greifen: nach 08 Lineale, nach 09/23 Raster, nach 18 Fang am Raster. `20_edit_guides_dialog_open` (Strg+Alt+Umschalt+G) öffnet auf beiden Seiten keinen Dialog (Drehbuch). |
| xs-canvas (ohne 08) | 20 ok, 1 Fehler / 20 ok, 1 Fehler | 19 ok, 2 Fehler / unverändert | Neu rot: Original `07_guides_dblclick_open`. Vorher öffnete der Doppelklick „Hilfslinie verschieben“ nur, weil die Lineale aus Schritt 06 verschleppt eingeschaltet waren. Mit Vorgabe-Ansicht ist das Ergebnis gleich Firepixel (dort schon vorher rot). **Drehbuch** (F6-Datei, nicht angefasst): Schritt 07 braucht zuerst `Ansicht > Lineale` wie 06. |
| dialogs-d41-d50 15/16/18 | 4 ok / 3 ok + 1 unmapped | 4 ok / 4 ok | 16: Original-Feld „60“ (vorher „60r“), Vorschau an, Dialog offen wie in Firepixel. |
| xs-file-formats 00–05 | 5 ok, 1 Fehler / 5 ok, 1 Fehler | unverändert | `close_all` vor 03–05 läuft. `05_open_psd_result_text`: Objekt „text“ fehlt auf beiden Seiten (Drehbuch). Die Einheit nennt `testfile: T11`, die Datei gibt es nicht (Start-Fehler auf beiden Seiten, Drehbuch). |

Summe der Fehler in diesen Schritten: vorher 8 Original / 3 Firepixel, nachher 9 / 3. Der zusätzliche Original-Fehler
ist xs-canvas 07. Er war vorher nur durch den verschleppten Zustand grün. Inhaltlich behoben: TW-14
(4 Effekt-Schritte), Auswahl in inspector-stroke 04 (3 Schritte), d41-d50/16.

## Offen

- Segmentüberlagerung (`Ansicht > Segmentüberlagerung`) hat keine JS-API und wird nicht zurückgesetzt (menu-view 12 toggelt ohne `after`).
- Drehbücher: xs-canvas 07 (Lineale einschalten), xs-file-formats `testfile T11`, dialogs-d21-d30 05/09/17 (`dialog.combo.0` gibt es nicht) sowie D23/D29 Kombifeld.
- Lineale-Vorgabe „aus“ stammt aus Firepixel. Ob das Original bei einer frischen Installation Lineale zeigt, ist nicht nachgemessen.
- Am Ende läuft eine Original-Instanz ohne Dialog, darin ist par.psd offen.

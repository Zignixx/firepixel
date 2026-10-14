# RUN-5: Runner-Lücken schließen (14.10.2026, 21:53–23:50)

Geändert nur `tools\parity\runner\*` (runner.py, origctl.py, orig_host.ps1, ui_map.json, README.md) und fünf
Drehbücher (`dialogs-d21-d30`, `dialogs-d31-d40`, `dialogs-d41-d50`, `color-popup`, `panel-swatches-mixer`,
`xs-preferences`). Kein Rust. Kein Volllauf. Mess-Sitzungen nur über `gui-sitzung.ps1` (Sperre je Messung, der Runner
nimmt sie je Einheit).

## Ausgangslage (Skript-Auswertung `run\`, Stand Vorlauf + Nachlauf)

831 Schritte mit Aufnahmebereich, davon **449 mit Bildpaar (54 %)**. Fehlende Paare nach Ursache (Seite ohne Bild):

| Ursache | Fälle | Ergebnis der Nachstellung |
|---|---|---|
| Original: `choose`/`fx_add` „target opened no menu“ (Effekt-„+“, Effekt-Popups, Kontur-Kategorie) | 36 + 31 „no menu open“ (Autoformen-Klappmenü, „+“) | **Zeitproblem**: Menü/Popup erscheint erst 0,5–1 s nach dem Klick, der Runner sah nur einmal nach 0,3 s nach |
| Original: Untermenü ohne Texte (Pinselstrich-Kategorien), `orig_index` fehlt | 18 | Untermenüs sind owner-drawn, aber **alphabetisch**; Reihenfolge einmal vermessen |
| Firepixel: `ui_find` findet Text nicht | 45 | v. a. `Ebene 1` (Zeile aus dem kleinen Ebenen-Bedienfeld gescrollt, 16×), Ausrichten-Knöpfe (Bedienfeld per Menü weggeschaltet), Effektnamen |
| `{layer_row}`/`{object_row}`/`layer_eye`/`lock`/`expand`/`name` | 30 Original, 10 FP | Ebenenliste des Originals ist eine owner-drawn **Win32-ListBox** |
| `{pref_category}` | 10 | Kategorienliste = `SysListView32`, Texte nicht lesbar; Namen im Drehbuch teils anders („Bearbeitung“) |
| `{popup_swatch}` | 6 + 19 FP | Farbe `#3366CC` gibt es in **keinem** der beiden Farbpopups (Drehbuchfehler) |
| `find_child`/„no combo_box“ (`dialog.combo.0`) | 16 + 8 | D23/D29/D45/D48: gemeint ist der Schieberpfeil neben dem Feld; D25/D33/D34 sind Meldungen ohne Liste |
| Verb `repeat` | 5 + 5 | nicht umgesetzt |
| Werkzeugleiste `OWL.ToolbarContainer` fehlt | 8 | Klasse gibt es nicht (Werkzeuge im Hauptfenster gezeichnet) |
| Drehbuch: T11, `layers.map`, `select {index}` | 2 + 10 + 4 | T11 ist ein Ordner; die Skript-Engine des Originals kennt `Array.map` nicht |
| Verb `export`, `resize_panel`, `window_size`, `drag_text` … | 5 + 2 + 2 + 4 | offen |
| Rest: nicht vermessene Ziele (`fx_popup.*`, `popup.*`, `optimize.*`, `preview.*`, `stroke_popup.*`, Fußzeilen-Knöpfe, Stil-/Zeichen-/Farbtabellenzellen …) | ≈ 120 | offen (je Ziel eine Messung) |

## Änderungen

- **Wartezeiten Original**: `choose`/Menübereich/`orig_index` warten bis 2,5 s auf ein Menü (`wait_menus`). `open`
  ohne Feld unter dem Punkt klickt ein zweites Mal, wenn nach 2,5 s weder Menü noch neues Fenster da ist. `select`
  wartet, bis sich das Eigenschaften-Bedienfeld geändert hat (≤ 3 s).
- **Definierter Werkzeugzustand**: Nach jedem Öffnen einer Testdatei wird auf beiden Seiten das Zeigerwerkzeug gewählt.
  Ohne das zeigte das Original nach einem Marquee-Werkzeug weiter den Auswahlrahmen-Inspektor (Läufer-Befund 3,
  nachgestellt: Farbfeld-Popup öffnete nicht).
- **Owner-drawn Untermenüs**: `ui_map.json → orig_owner_drawn_menus` (12 Pinselstrich-Kategorien, vermessen);
  `menu_path` wählt dort nach Index. `orig_index` mit Pfad `A > B` = Index im Untermenü von A.
- **Ebenen-Bedienfeld Original**: Zeilenliste aus dem Dokument (JSF `Layer.elems` mit Unterebenen und Objekten in
  Anzeigereihenfolge, `disclosure`), geprüft gegen die ListBox (Zeilenzahl, Höhe 24 px Ebene / 30 px Objekt), Zeile per
  `LB_SETTOPINDEX` sichtbar (neuer Host-Befehl `lb_info`). Spalten: Auge, Schloss, Dreieck (+16 bei Unterebene), Name.
  Firepixel: Zeilen außerhalb des Bedienfelds per Mausrad erreicht; Auge/Schloss/Dreieck relativ zur Zeile;
  `object_row` per Index. Firepixel-Aufnahmebereich `panel:layers` war nur die Kopfzeile, weil die Objektzeile „Pfad“
  als Bedienfeldtitel galt – jetzt zählen nur Reiterzeilen.
- **`{popup_swatch}`** beidseitig über die Farbe im Popup-Bild (Original: Popupfenster, Firepixel: Bereich über dem
  HEX-Feld). Drehbücher: `#3366CC` → `#0000FF` (5 Stellen).
- **`{pref_category}`**: Zeilenindex aus `ui_map.json → orig_pref_categories` (inkl. der Drehbuch-Namen), Zeile 17 px;
  gilt auch für reine Beschriftungen dieser Kategorien (D16).
- **`{field_arrow: i}`** (Schieberpfeil rechts neben dem i-ten Feld) beidseitig; Drehbücher D23/D29/D45/D48 darauf
  umgestellt, D25/D33/D34 ohne `open` (Meldungen). `dialog.*` im Original wartet bis 3 s auf das Dialogfeld.
- **`repeat {n, do}`** beidseitig. **`select {index}`** beidseitig. **`Array.map`** im Original: `X.map(function …)`
  wird vor dem Senden zu `__fp_map(X, …)` (origctl `map_compat`).
- **T11** = Ordner `par_import` (wird beim Einheitenstart nicht geöffnet), `T11:par.psd` = Datei darin, T12 = Farbfelddatei.
- **Bedienfelder per Menü**: `menu "Fenster > <Bedienfeld>"` gilt als `panel` (nur zeigen, sonst schaltete der Eintrag
  ein sichtbares Bedienfeld aus); `"toggle": true` behält den Menübefehl. Firepixel-`panel` für Bedienfelder ohne
  Engine-Befehl (Ausrichten, Pfad, Farbpalette, Sonderzeichen, Bildbearbeitung, Symboleigenschaften): Reiter anklicken,
  sonst Menü.
- **ui_map**: Ausrichten-Knöpfe des Originals (`orig_labels`, 18 Symbole vermessen), Segment-Inspektor
  (`inspector.slice_type`, `slice_format`, `link`, `target` beidseitig), Effekt-Popup Firepixel (`fx_popup.distance/
  opacity/softness/angle` = `dialog.field.0–3`, `distance_slider` = `dialog.slider.0`, Aussparung = „Objekt
  aussparen“), `handle_roles` `sides`/`size`/`turns` (Spiralen), `menu_aliases` (Abschrägungs-/Prägungsnamen),
  `panel:tools` des Originals aus Client und Dokumentansicht.

## Probelauf (neueste Vorschau `Firepixel-Vorschau-2026-10-14_21-48`, Ausgabe `re\_testdateien\par\run5\run\`)

Zwei Teilläufe (22:27–23:21 mit dem Stand bis 22:27, 23:21–23:48 mit allen Änderungen), 11 Einheiten mit den meisten
Lücken:

| Einheit | Paare vorher | nachher |
|---|---|---|
| tool-autoshapes | 2/31 | **30/31** |
| fx-shadow-glow | 1/16 | 10/16 |
| fx-bevel-emboss-fill | 1/15 | 10/15 |
| inspector-stroke | 23/46 | 42/46 |
| panel-layers | 4/17 | 13/17 |
| color-popup | 5/18 | 10/18 |
| xs-preferences | 2/13 | 6/13 |
| panel-layers-menu | 3/12 | 9/12 |
| panel-info-align-path | 7/18 | 9/18 |
| inspector-web | 4/11 | 7/11 |
| dialogs-d21-d30 | 14/18 | 17/18 |
| **Summe** | **66/215 (31 %)** | **163/215 (76 %)** |

Stichproben der Paare (Ebenen-Zeile, Voreinstellungen ▸ Text, Effekt-Popup, Autoform-Stern) zeigen auf beiden Seiten
denselben Zustand.

**Geschätzte Gesamtabdeckung** (gemessene 11 Einheiten + übrige 38 Einheiten, deren fehlende Paare nur Ursachen
haben, die jetzt behoben sind): **≈ 585 von 831 Schritten = 70 %** (vorher 54 %). Das Ziel 90 % ist **nicht** erreicht.
Der Rest besteht fast nur aus einzeln zu vermessenden Zielen (≈ 120 Schritte, je Ziel eine Messung in beiden
Programmen) und nicht umgesetzten Verben (≈ 25).

## Offen (nächste Schritte nach Ertrag)

1. Nicht vermessene Ziele: `fx_popup.*` des Originals für Abschrägung (Breite, Kantenform, Knopfvorgabe, Winkelrad,
   Farbe), `popup.*` (HEX-Feld, Ohne, Systemfarbwähler, Paletten-/Optionsmenü), `optimize.*`/`preview.*` (je ≈ 10),
   `stroke_popup.*`, Fußzeilen-Knöpfe der Ebenen, Raster-Zellen (`style_cell`, `char_cell`, `color_table_cell`,
   `panel_swatch`) – ein Zellraster-Verfahren wie bei `popup_swatch` würde 10–15 Schritte abdecken.
2. `panel-info-align-path`: Firepixel findet „Links ausrichten“ im Lauf weiterhin nicht (im Einzeltest ja) –
   vermutlich verdeckt der Reiterwechsel; Pfad-Knöpfe des Originals (Symbole ohne Text, Reihenfolge per Tooltip messen).
3. Verben `export` (Exportdialog beider Seiten), `resize_panel`, `window_size`, `drag_text`.
4. Werkzeugzüge im Original (Läufer-Befund 3) jetzt seltener (Zeigerwerkzeug je Datei); nicht systematisch geprüft.

## Befunde für Firepixel (nicht behoben, nur beobachtet)

- `ui_action {panel}` kennt Ausrichten, Pfad, Farbpalette, Sonderzeichen, Bildbearbeitung, Symboleigenschaften und
  Eigenschaften für automatische Form nicht (kein `engine_command`), sie fehlen auch in `ui_state.visible_panels`.
- Live-Filter-Menü ▸ Abgeflachte Kante und Relief: Firepixel „Abschrägung innen/außen“, „Prägung erhaben/eingelassen“;
  Original „Abgeflachte Kante innen“, „Abgefl. Kante außen“, „Reliefprägung“, „Senkprägung“ (andere Reihenfolge).
- Pfad-Bedienfeld: Firepixel Textknöpfe, Original Symbolleisten in vier Gruppen (Design-Unterschied, zu bewerten).
- Effekt-Popup: Firepixel „Objekt aussparen“, Original „Aussparung“.

Am Ende läuft eine Original-Instanz ohne Dialog.

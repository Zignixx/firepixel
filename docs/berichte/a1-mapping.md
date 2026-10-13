# A1 – Positions-Mapping für den Parity-Runner (13.10.2026)

## Ergebnis

| Kennzahl | vorher | nachher |
|---|---|---|
| Schritte mit Bild, bei denen beide Seiten ausführbar sind (statische Prüfung aller 831 Bild-Schritte: Verb unterstützt, Ziel gemappt) | 373 (45 %) | 640 (77 %) |
| Probelauf 15 Bild-Schritte (6 Einheiten, Popups/Flyouts/Griffe/Bildlauf/Bedienfeldmenü) mit beiden Bildern | 0 / 15 | 13 / 15 |
| Selbsttest `_selftest_menu`, `_selftest_inspector` | 6 / 6 | 6 / 6 |

## Neu gemappt (`tools\parity\runner\ui_map.json`)

* **107 Befehle** (`commands`): Befehls-ID → Menüpfad der Referenz, aus `crates\fp-ui\data\menu_main.json`
  erzeugt und gegen den live ausgelesenen Menübaum (798 Einträge) geprüft; Filter-Menü live gemessen
  (z. B. `Filter > Weichzeichnen > Gaußscher Weichz.`). Größter Einzelgewinn: `file.imagePreview` (65 Schritte),
  `edit.preferences` (26). Nicht gemappt: `help.about` (Eintrag enthält den Programmnamen).
* **114 Ziele** (`targets`) für die Referenz, davon 108 auch für Firepixel, u. a.:
  * Eigenschaften-Bedienfeld Vektor (Füllung/Strich/Kante/Textur/Deckkraft/Mischmodus/Filter „+“/„–“,
    B/H/X/Y/Name, Stil) und Text (Schrift, Schnitt, Größe, Farbe, Laufweite, Zeilenabstand + Einheit,
    F/K/U, Ausrichtung, Einzüge, Abstände, Glättung), Filterlisten-Zeilen 0–2 (Zeile, Info, Haken),
  * Filter-Popup der Referenz (`fx_popup.distance/opacity/softness/angle/knockout`, Schieber-Knopf),
  * Werkzeugleiste: Farbfelder, Tauschen/Standard/Keine, Bildschirmmodi, Segmente ein/aus, **10 Flyouts**
    (`toolbox.<werkzeug>_flyout`, Drücken-und-Halten 900 ms),
  * Optimieren (Format, Vorgabe, Hintergrund), Bildvorschau (Format, Vorgabe, „+“, Vorschau, Zeiger/
    Zuschneiden/Zoom, Zoomliste, `zoom_100`, Ansicht 1/2/4, Animation).
* `handle_roles` (Griff-Rolle → deutsches Tooltip-Präfix), `orig_panel_menu_offset`, `orig_neutral_point`.

## Neue Aktionen (`runner.py`, `orig_host.ps1`)

`choose` auf Popup-Menüs (Text-Pfad oder `orig_index` für Flyouts ohne Texte), `fx_add`, `flyout_open/flyout/
flyout_item`, `panel_menu`, `drag_popup_slider`/`slider_drag` (mit `hold`), `scroll`/`wheel`/`canvas_wheel`,
`drag_handle`/`click_handle`/`hover_handle` und Zielform `{handle}`, `mouse_down`/`mouse_up` (Referenz),
`export_png` (Referenz: Export mit Dokument-Einstellungen; Firepixel: `render`), `log_menu_items` als Aktion,
Zielformen `{menu_item}`, `{preview_pane}`, `{dialog_rel}`, Locator-Optionen `hold_ms` und `select`.
Für `rel`-Punkte fragt der Runner das Win32-Fenster unter dem Punkt ab (Edit → Text setzen, ComboBox →
Eintrag wählen). Host: neue Befehle `wheel`, `down`, `up`, `held_move`, `at`, `menu_tree`.

## Korrekturen nebenbei

* Host-Eingabesperre ließ Klicks abbrechen, sobald ein Popup-Schieber (`Afx:`-Fenster) des Originals den
  Vordergrund hatte – die Maustaste blieb dann gedrückt. Jetzt zählen `Afx:`-Fenster desselben Prozesses mit.
* `dialog_close` schloss die Bildvorschau nicht (WM_COMMAND IDCANCEL wird ignoriert): jetzt zuerst BM_CLICK
  auf „Abbrechen“/„OK“.
* Popup-Schieber der Referenz bleiben offen und liegen über ihrem Pfeil; der leichte Reset klickt sie jetzt
  an einer neutralen Stelle (leere Menüleiste) zu.
* Firepixel-Bereich `dialog`: die Brücke meldet keine `window`-Rolle; Ersatz über „Titelleiste …“ + Inhalt.
* Drehbücher `fx-bevel-emboss-fill`, `fx-stack`: Gruppenname im „+“-Menü der Referenz heißt live
  „Abgeflachte Kante und Relief“ (nicht „Abschrägung und Prägung“), 18 Stellen korrigiert.

## Bekannte Lücken

* `fx_popup.*` nur Referenz: in Firepixel sind die Filtereinstellungen ein Dialog → `fp` bleibt `unmapped`
  (z. B. `fx-shadow-glow 05`); Schatten-Popup wird in Firepixel als ganzes Fenster aufgenommen (Differenzbox).
* Referenz: Zeilenformen `{layer_row}`, `{pref_category}`, `{popup_swatch}`, `{list_row}`, `{layer_expand}`
  (zusammen ~35 Schritte), versteckte Werkzeuge ohne Flyout-Weg in `tools` (`hotspotRect`, `pathScrub*`,
  `reshapeArea`, `polygonSlice` …), Verben `export`, `drag_text`, `repeat`, `file_dialog_open_types`.
* Inspektor-Positionen gelten für das angedockte Bedienfeld (1600 × 1000) und je Objektart; Bitmap-/Symbol-
  Ansichten nicht eigens vermessen (gleiche Spalten wie Vektor angenommen).
* Auffällig im Probelauf (zur Auswertung, kein Runner-Fehler vermutet): `tool-autoshapes 05_handle_drag_star_
  points_result` – Referenz reduziert die Zacken, Firepixel ändert den Stern nicht.
* Einmal „main window not found“ beim Aufräumen nach `tool-autoshapes` (folgende Einheiten liefen normal).

## Volllauf

```
C:\Users\Administrator\AppData\Local\Programs\Python\Python314\python.exe -I C:\Users\Administrator\Desktop\REA\firepixel\tools\parity\runner\runner.py --all
```

1282 Schritte in 53 Einheiten, im Probelauf ~10–12 s je Schritt → **ca. 3,5–4 h**.
Messbilder dieses Auftrags: `re\_testdateien\par\a1map\`, Probelauf: `re\_testdateien\par\a1run\`.

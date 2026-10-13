# 16 `inspector-document` – Inspektor ohne Auswahl: Aufnahme des Originals (PAR-O-16-18, 13.10.2026 ≈ 06:00–06:20)

Testdateien T01, T04 (05), T09 (09) als Arbeitskopien `re\_testdateien\par\work\insp_t0x.fw.png`. Fenster 1600 × 1000,
Zoom 100 %. Referenzbilder: `re\_testdateien\par\ref\inspector-document\` (+ `INDEX.md`). Messart: **W** Fenstertext/
Steuerelemente (Klasse, Text, Lage, aktiv), **J** JSF, **B** Bild. Koordinaten = Bildschirm (Fenster bei 0,0).

**Leinwandklicks gehen wieder:** Der „interne Fehler“ kam vom Öffnen per `fw.openDocument(url, false, true)`; mit
`fw.openDocument(url)` geöffnete Dokumente lassen sich normal bedienen (`re\25-leinwand-fehler.md`).

## Prüfpunkte

| Nr | Kürzel | Original | Art |
|---|---|---|---|
| 01 | `layout` | Bedienfeld „Eigenschaften“ 70,890–1324,995, Inhalt in 4 Spalten-Blöcken (nur Block 1–2 belegt). Block 1 (74–190): Vorschaubild 41 × 37 (74,899), rechts „Dokument“ (121,899) und darunter der Dateiname ohne Endung („insp_t01“). Block 2 Zeile 1 (y 894–915): „Leinwand:“ (193), **Farbfeld** 24 × 21 (261–285), Knöpfe **„Leinwandgröße...“** 292–405, **„Bildgröße...“** 412–525, **„Leinwand anpassen“** 538–664 (Knopftexte mit drei Punkten „...“, letzter ohne). Zeile 2 (y 920–941): kleines Exportsymbol 235–250, **Exportformat-Liste** 261–528 („PNG32“). Zeile 3 (y 947–968): „Status:“ 207–243, **Status-Liste** 249–428 („Status 1“). Alle Felder aktiv | W, B `01` |
| 02 | `canvas_color` | Klick aufs Farbfeld öffnet das **Farbfeld-Popup nach oben** (278 × 312, 257,610–535,922): Raster der aktuellen Farbfelder, darunter Vorschaufeld, **HEX** (Feld + Kopierknopf), **RGB** (3 Felder + Kopierknopf); Fußzeile links „Ohne“ (Transparent), rechts Farbrad (Systemfarbwähler). HEX-Feld anklicken, `FF0000` + Enter → Leinwandfarbe `#ff0000`, Popup zu. Tippen **ohne** Klick ins HEX-Feld geht nicht ins Feld; Enter übernahm dann `#a1a1a1` (Vorschaufarbe beim Öffnen, Annahme) | B `02`, J |
| 03 | `canvas_size_btn` | öffnet modalen Dialog **„Leinwandgröße“** (377 × 331): Gruppe „Neue Größe“ B 800 / H 600 je mit Einheit „Pixel|Zoll|Zentimeter“, rechts „B: 800“, „H: 600“; „Anker:“ 3 × 3 Knöpfe (Mitte gewählt); „Nur aktuelle Seite“ (an); Gruppe „Aktuelle Größe“ 800/600 Pixel; OK, Abbrechen. Esc schließt | W, B `03` |
| 04 | `image_size_btn` | öffnet **„Bildgröße“** (410 × 313): „Pixelabmessungen“ B 800 / H 600 je „Pixel|Prozent“; „Druckgröße“ B 11.111 / H 8.333 je „Prozent|Zoll|Zentimeter“ (Zoll gewählt), „Auflösung:“ 72 „Pixel/Zoll|Pixel/cm“; „Proportionen beibehalten“ an, „Bild neu auflösen“ an + „Bikubisch|Bilinear|Weich|Nächster Nachbar“; „Nur aktuelle Seite“ an; OK, Abbrechen. Esc schließt | W, B `04` |
| 05 | `fit_canvas` | T04: Klick → Leinwand **800 × 600 → 796 × 336** (= Grenzen aller Objekte 50,50–846,386, Objekte rücken mit), ohne Dialog; ein Strg+Z → 800 × 600 | J, B `05` |
| 06 | `size_label` | Inspektor zeigt **keine** Größe/Auflösung. Größe steht in der Statuszeile des Dokumentfensters („800 x 600“, daneben Zoom „100%“ mit ▼) | W, B `06` |
| 07 | `export_format` | Liste (8, in dieser Reihenfolge): PNG32 · JPEG - Höhere Qualität · JPEG - Kleinere Datei · GIF Web 216 · GIF WebSnap 256 · GIF WebSnap 128 · GIF Adaptiv 256 · Animiertes GIF WebSnap 128. Klappt **nach unten** auf (275 × 135) | W, B `07` |
| 08 | `optimize_sync` | Wahl „GIF WebSnap 256“ → `exportOptions`: exportFormat GIF, colorMode indexed, numEntriesRequested 256, webSnapAdaptive true, name „GIF WebSnap 256“. Optimieren-Bedienfeld zeigt sofort: Vorgabe „GIF WebSnap 256“, Format „GIF“, Palette „Web-Adaptiv“, Farben 256, Verlust 0, Rastern 0 %, „Ohne Transparenz“ | J, B `08` |
| 09 | `state_delay` | **kein** Verzögerungsfeld im Inspektor; nur die Status-Liste (T09: „Status 1|Status 2“). Verzögerung je Status per JSF = 7 (1/100 s), nur im Status-Bedienfeld einstellbar | W, J, B `09` |
| 10 | `thumbnail` | Vorschaubild 41 × 37 zeigt die Leinwandfarbe (T01 weiß, nach Änderung rot) | B `10` |
| 11 | `collapsed` | **Doppelklick auf den Reiter „Eigenschaften“** schaltet reihum: volle Höhe (Bedienfeld 890–995) → **halbe Höhe** (940–995, nur Zeile 1: Vorschaubild, Leinwand-Zeile, Exportformat) → **nur Reiter** (Inhalt unter dem Fensterrand) → volle Höhe | B `11` (2) |
| 12 | `tool_inspector` | ohne Auswahl zeigen Hand (`h`), Zoom (`z`) und Zeiger (`v`) **denselben** Dokument-Inspektor (identische Steuerelemente) | W |
| 13 | `no_document` | ohne offenes Dokument: Bedienfeld leer (nur Trennlinien), keine Steuerelemente | W, B `13` |
| 14 | `undo` | Leinwandfarbe ändern = **ein** Verlaufsschritt „Leinwandfarbe“; Strg+Z → `#ffffff` | J, B `14` |

## Weitere Beobachtungen

- Der Dokument-Inspektor zeigt den Dateinamen ohne `.fw.png`; „Leinwand anpassen“ arbeitet ohne Rückfrage.
- Die Wahl des Exportformats im Inspektor ist kein eigener Prüfschritt im Verlauf (nicht gemessen, Verlauf zeigte nur
  „Leinwandfarbe“).

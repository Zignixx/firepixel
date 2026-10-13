# 16 `inspector-document` – Vergleich Original ↔ Firepixel (PAR-F-16-18, 13.10.2026)

Firepixel: Debug-Build `main` 180852a (Slot fp-s3), eigene Instanz mit eigenem Profil und `--mcp`, Fenster 1600 × 1000,
Design „normal“, Zoom 100 %; echtes Fenster, Brücke (`ui_find`/`ui_input`/`screenshot`/`run_script`). Testdateien: Kopien von
`re\_testdateien\par\work\insp_t01|t04|t09.fw.png`. Bilder `docs\parity\img\inspector-document\`, Vergleich
`re\_testdateien\par\cmp\inspector-document\` (15 Paare; Kennzahl nur Hinweis).

| Nr | Prüfpunkt | Original | Firepixel | Bew. | Befund |
|---|---|---|---|---|---|
| 01 | layout | Vorschaubild, „Dokument“ + Name; Leinwand: Farbfeld, „Leinwandgröße...“, „Bildgröße...“, „Leinwand anpassen“; Exportformat „PNG32“; „Status:“ Liste | gleiche Felder, Texte und Reihenfolge; Status-Zeile 7 px tiefer (y 954 statt 947) | ✅ | – |
| 02 | canvas_color | Farbfeld-Popup nach oben (278 × 312 bei 257,610): Raster, Vorschau, HEX + Kopieren, RGB + Kopieren, „Ohne“, Farbrad; **kein** Deckkraftfeld | Popup nach oben (≈ 256 × 275 bei ≈ 279,625), HEX/RGB/Kopieren/„Ohne“ (Tooltip „Keine Füllung“)/Farbrad wie Original, **zusätzlich Deckkraftfeld „100“ + ▼**; HEX `FF0000` + Enter → `#ff0000`, Popup zu, 1 Schritt | ❌ | PAR 16/02 |
| 03 | canvas_size_btn | Dialog „Leinwandgröße“: Neue Größe B/H + „Pixel/Zoll/Zentimeter“, Anker 3 × 3 (Mitte), „Nur aktuelle Seite“, Aktuelle Größe, OK/Abbrechen; Esc schließt | gleiche Felder, Einheiten, 9 Ankerknöpfe, Aktuelle Größe 800/600; Esc schließt (Dialog 333 × 382, Gestaltung DES-1) | ✅ | – |
| 04 | image_size_btn | „Bildgröße“: Pixel B/H (Pixel/Prozent), Druckgröße 11.111/8.333 Zoll (Prozent/Zoll/Zentimeter), Auflösung 72 (Pixel/Zoll, Pixel/cm), Proportionen, Neu auflösen + 4 Verfahren, Nur aktuelle Seite | alle Felder, Werte und Listen gleich; Esc schließt | ✅ | – |
| 05 | fit_canvas | T04: 800 × 600 → 796 × 336 ohne Dialog, Strg+Z → 800 × 600 | 796 × 336, ein Schritt „Leinwand anpassen“, Strg+Z → 800 × 600 | ✅ | – |
| 06 | size_label | keine Größe im Inspektor; Statuszeile „800 x 600“, „100%“ | Statuszeile „800 x 600“, „100%“; nicht im Inspektor | ✅ | – |
| 07 | export_format | 8 Einträge: PNG32 · JPEG - Höhere Qualität · JPEG - Kleinere Datei · GIF Web 216 · GIF WebSnap 256 · GIF WebSnap 128 · GIF Adaptiv 256 · Animiertes GIF WebSnap 128; klappt nach unten | 15 Einträge, **andere Reihenfolge** (GIF Web 216 … GIF Adaptiv 256, JPEG –…, Animiertes GIF, PNG8, PNG24, PNG32, WebP 80, WebP verlustfrei, AVIF 60, APNG, Animiertes WebP), Gedankenstrich „–“ statt „-“; klappt nach oben (Platz) | ❌ | PAR 16/07 |
| 08 | optimize_sync | „GIF WebSnap 256“ → Optimieren: GIF, Web-Adaptiv, 256, Verlust 0, Rastern 0 %, Ohne Transparenz; JSF `exportOptions` GIF/indexed/256 | Optimieren folgt sofort (gleiche Werte); **JSF `exportOptions` bleibt `PNG`/`32 bit`**; zusätzlich Verlaufsschritt „Optimieren“ (Original: kein Schritt beobachtet) | ❌ | PAR 16/08 |
| 09 | state_delay | kein Verzögerungsfeld, nur Status-Liste „Status 1/Status 2“ | gleich | ✅ | – |
| 10 | thumbnail | 41 × 37, zeigt Leinwandfarbe | zeigt Leinwandfarbe (weiß, nach Änderung rot) | ✅ | – |
| 11 | collapsed | Doppelklick auf Reiter „Eigenschaften“: volle → halbe Höhe → nur Reiter → volle Höhe | Doppelklick (Reiter und freie Reiterleiste) **ohne Wirkung**; Einklappen gibt es im Code nur für schwebende Gruppen (`docking.rs`) | ❌ | PAR 16/11 |
| 12 | tool_inspector | Hand, Zoom, Zeiger zeigen denselben Dokument-Inspektor | gleich (H, Z, V) | ✅ | – |
| 13 | no_document | leeres Bedienfeld | leeres Bedienfeld (nur Reiter) | ✅ | – |
| 14 | undo | ein Schritt „Leinwandfarbe“, Strg+Z → #ffffff | ein Schritt „Leinwandfarbe“, Strg+Z → #ffffff | ✅ | – |

**Zählung:** ✅ 10 · ❌ 4 · 🎨 0.

## Fragen / Annahmen

- 08: Ob die Wahl des Exportformats im Original einen Verlaufsschritt erzeugt, ist dort „nicht gemessen“. Firepixel legt
  „Optimieren“ an – als Teil von PAR 16/08 notiert, beim Beheben im Original gegenprüfen.
- 07: Die zusätzlichen Formate (PNG8/24, WebP, AVIF, APNG, Animiertes WebP) sind Firepixel-Erweiterungen; Befund betrifft
  Reihenfolge der acht Originaleinträge und den Bindestrich, nicht die Erweiterungen (Frage an den Nutzer: Erweiterungen ans
  Ende hängen?).
- 02: Popup-Lage weicht ≈ 20 px ab (Original hier maßgeblich, N27b) – im Befund mit erfasst.

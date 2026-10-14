# NP-1: Nachprüfung der Nutzerbefunde N21–N33 im echten Fenster

Stand 14.10.2026 ≈ 07:20. Testversion `Firepixel-Vorschau-2026-10-14_00-22` (Kopie im Scratch-Ordner, eigenes Profil, Start mit `--mcp`).
Bedient habe ich wie ein Mensch über die Brücke: Menüs, Knöpfe und Felder mit `ui_find`/`ui_input`, Datei-Dialoge mit `ui_file_dialog`.
Nur das Anlegen der Testobjekte lief über `new_document`/`tool_event`. Das Original habe ich nicht bedient. Verglichen habe ich mit `re\_testdateien\par\ref\color-popup\` und `re\_nutzerbilder\2026-10-13\`.
Die Bilder liegen unter `re\_testdateien\np1\`. Die BATCH-T-Instanz habe ich nicht angefasst.

## Ergebnis

| N | Stand | Kurz |
|---|---|---|
| N21 | ❌ | Fortschrittskarte gut (`n21_ki_fortschritt.png`). Nach *Auswählen ▸ Motiv auswählen* gibt es eine Pixelauswahl 240 × 99, die Auswahlameisen sind aber nicht zu sehen (`n21_ki_ergebnis.png`, `n21_ki_auswahl_detail.png`). Feineinstellungen im Inspektor habe ich auf diesem Weg nicht gefunden. |
| N22 | ✅ | Bildvorschau mit 4 Feldern und allen Bedienteilen (`n25_bildvorschau_4fach_vorher.png`). |
| N23 | ✅ | Optimieren JPEG: „Zeilensprung“/„Progressiv“ ganz sichtbar (`n23_optimieren_jpeg.png`). |
| N24 | ✅ | Schriftliste mit sichtbarer Leiste und Griff. Das Bild liegt nur im Scratch, weil es echte Schriftnamen zeigt (§1a). |
| N25 | ✅ | Reiter 4-fach und Bildvorschau mit 4 Feldern laufen synchron: Mausrad, Leertaste+Ziehen, Strg+Rad (`n25_*`). |
| N26 | ✅ | Der Rahmen umschließt nur die sichtbaren Pixel, auch bei der alten FW-PNG-Datei (`n26_n32_*`). |
| N27 | ❌ | (a) behoben (`n27a_*`). (b) Popup wie im Original, Pipette auf der Leinwand richtig. Über Bedienfeldern und Menüleiste zeigt die Pipette dagegen #000000 und ein Klick setzt Schwarz. Über dem Glühen-Dialog nimmt sie das Leinwandpixel unter dem Dialog. |
| N28 | ✅ | Glühen sofort sichtbar, Vorgaben 5/65/12/Rot (`n27_n28_gluehen_dialog.png`). Das Bild habe ich nur angesehen, nicht pixelgenau nachgemessen. |
| N29 | ✅ | „Glatte Kurven“ ist wählbar und liefert beim Quadrat 1 Pfad mit 4 Punkten (`n29_glatte_kurven_quadrat.png`). |
| N30 | ✅ | Bis das Fenster sichtbar ist: Median 822 ms (vorher 1081). Bedienbar nach 874 ms. |
| N31 | ✅ | Der Pinsel malt live. Deckkraft und Textur „Herz 01“ wirken, ein Zug ergibt einen Verlaufsschritt (`n31_*`). |
| N32 | ✅ | Nur ein Rahmen, auch beim Verschieben (`n32_*`). |
| N33 | ✅ | „Dateien…“, „Ordner…“ und Ziel-„Ordner…“ öffnen ihren Dialog, und die Einträge landen in der Liste (`n33_studio_nach_dateien_ordner.png`). |

## Neue Befunde (zum Beheben)

1. **N27 Pipette außerhalb der Leinwand** (`n27b_pipette_bedienfeld4.png`, `n27b_pipette_menue.png`)
   - Über Bedienfeldern und Menüleiste zeigt das Popup auch nach Ruhen und mehreren Bewegungen #000000.
   - Ein Klick übernimmt dieses Schwarz.
   - Das „Fensterfoto“ aus FARB-1b kommt offenbar nicht an. Die Sitzung lief ohne aktiven Desktop, das sollte man beim Nutzer gegenprüfen.
   - Zu prüfen ist auch, ob ein Bildschirmfoto der Brücke das Pipettenfoto „wegnimmt“: Laut farb1b wird ein Foto ohne Kennung dem ersten wartenden Foto zugeordnet.
2. **N27 Pipette über Dialogen** (`n27_pipette_ueber_dialog.png`): Liegt ein Dialog über der Leinwand, liefert die Pipette das Dokumentpixel unter dem Dialog statt der sichtbaren Dialogfarbe. Die Leinwand-Abkürzung sollte nur greifen, wenn kein Fenster darüber liegt.
3. **N21 keine sichtbare Auswahl** nach *Auswählen ▸ Motiv auswählen*
   - Die Statuszeile meldet „Motiv auswählen: fertig“, und `pixelSelection` ist 34/53–274/152.
   - Auf der Leinwand sind keine Ameisen zu sehen, nur der Objektrahmen der Bitmap.
   - Der Inspektor (auch mit Auswahlrahmen-Werkzeug) zeigt keine KI-Feineinstellungen.
4. **N29 Nebenbefund:** Bei einer halbtransparenten Bitmap (Pinselstrich mit Deckkraft 50) melden beide Verfahren „Keine Pfade entstanden“, die Kennzahl zeigt trotzdem „100,0 % Übereinstimmung“ (`n29_glatte_kurven_sw_herzen.png`).
5. **Verlaufsname englisch:** Ein über `tool_event` gezeichnetes Rechteck heißt im deutschen Verlauf „Rectangle“ (`n27_verlauf_nach_ok.png`). Mit echtem Zeichnen habe ich das nicht gegengeprüft.

## Nicht geprüft

- N26: t3/t4, Import, Bleistift außerhalb.
- N31: Mischmodus, „Transparenz beibehalten“, Dialog „Pinselstrich bearbeiten“, große Bitmap.
- N23: schmales Fenster und 150 %.
- N24: Designs Dunkel und OLED.
- N28: Pixelvergleich mit den FX-Referenzen.
- N30: Flackern (kein sichtbarer Desktop).

# PAR-X-INSP-1 – Inspektor Dokument, Pfad/Rechteck, Füllung (Einheiten 16–18) beheben

Stand 13.10.2026, Slot fp-s3. Grundlage: `docs\parity\inspector-document.md`, `inspector-path-rect.md`,
`inspector-fill.md` (24 ❌) und die Aufnahmen `*.orig.md`. Alle 24 Punkte bearbeitet; Gegenprüfung (PAR-R)
offen. Commits auf main: `d38f715` (Engine, JSF, Texte), `f1f12f1` (Oberfläche, kittests).

## Umgesetzt

| Befund | Änderung |
|---|---|
| 16/02 | Leinwandfarbe: Farbwähler (der des Originals, FARB-1b) mit neuer Fußleiste `Footer::Canvas` – ohne Deckkraftfeld + ▼ (`color_picker.rs`, `quads.rs`). Lage ±20 px nicht geändert (folgt dem Klickpunkt wie im Original). |
| 16/07 | Exportformat-Liste: acht Vorgaben des Originals in dessen Reihenfolge, Firepixel-Formate dahinter (`inspector::export_preset_order`); „JPEG - Höhere Qualität/Kleinere Datei“ mit Bindestrich (`panels.ftl` de). |
| 16/08 | JSF `dom.exportOptions` folgt den Exporteinstellungen der Seite (exportFormat, colorMode, numEntriesRequested, webSnapAdaptive, paletteMode, name …); per Skript gesetzte Werte gelten, solange das Format passt (`fp-script imp\doc.rs`). |
| 16/11 | Doppelklick auf den Reiter „Eigenschaften“ bzw. die freie Reiterleiste: volle → halbe Höhe → nur Reiter → volle Höhe (`inspector::cycle_height`, `docking::TabStripOut::double_clicked`). |
| 17/02 | Pfadoperationen schon bei einem gewählten Pfad bedienbar (zusammengefasst wird erst ab zwei Pfaden), „Pinselstrich löschen“ bei jedem Strich bedienbar. |
| 17/05 | Rundung rechnet mit der aktuellen Größe (Skalierung der Matrix, `panel::primitive_size`); px-Eingabe speichert Einheit px; JSF `roundness` in px und `mode` „exact“ (Setter passend). |
| 17/07 | Aufklapplisten der Kombifelder so breit wie der längste Eintrag (`fields::combo`), Mischmodi ganz lesbar. |
| 17/08 | `pixelRect` von Pfaden mit Strich (mittig + d/2, außen + d) (`docops::element_pixel_rect`). |
| 17/12 | „+“-Menü der Live-Filter: „Ohne“ (entfernt alle Filter, ein Schritt `pi.effects.replace`), Optionen, Gruppen alphabetisch mit den Namen des Originals (Abgeflachte Kante und Relief, Farbe anpassen, Scharfzeichnen; `filter.ftl` de), Trenner vor dem letzten Eintrag, Breite 272. |
| 17/16 | B/H im Inspektor skaliert den Strich mit (Faktor √(sx·sy), Voreinstellung „Attribute skalieren“, Vorgabe ein): 8 → 11 px, `pixelRect` 39,159–261,251 (`element.setBounds`). |
| 17/17 | Tooltips: Farbfelder zeigen den Hex-Wert („#808080“), Rand-Liste „Füllungsrand“, Strichart-Liste „Pinselstrichkategorien“. |
| 18/01 | „Verlaufsfläche“/„Muster“ stellen um (Muster: Vorgabe „Baumwolle“/Cotton) und öffnen sofort ihr Popup. |
| 18/02 | Verlaufs-Popup mit eigenem Offen-Zustand (`popups::anchored_popup`): Liste, Farbwähler darin schließen es nicht; 12 Arten in Reihenfolge, Namen und JSF-Kategorien des Originals inkl. „Kontur“ (FSHi 3 + FSXi 17, Kennung 17). |
| 18/03 | Umschalten auf Verlauf/Muster setzt die Griffe neu (vorher blieben entartete Griffe aus Dateien → einfarbig); Vorgabeverlauf Weiß → Füllfarbe, senkrecht nach unten (270°). |
| 18/04 | Verlaufs-Popup ≈ 210 × 155: „Verlauf:“ + Liste, Leiste mit Deckkraft-/Farbstopps, Klick auf Farbstopp → Farbwähler des Originals, „Umkehren“, Verschieben (grau) + %, Winkel + ° (`pi.fill {angle}`). |
| 18/05 | Stopp aus dem Popup ziehen löscht ihn (mindestens zwei bleiben). |
| 18/07 | Füllgriffe auf der Leinwand beim Zeiger: Linie, Start rund, Ende quadratisch (Ellipse/Rechteck/Muster auch zweite Achse). |
| 18/08 | Muster-Popup: „Muster:“-Liste (60 + Trenner + „Andere...“), Vorschau 250 × 123, Winkel; Farbfeld zeigt das Muster; deutsche Namen aus der Namenstabelle des Originals (`pi-pattern-name-*`, „Baumwolle“ … „Ölfarbe 3“). |
| 18/11 | Kante Hart/Anti-Alias setzt die Weichheit zurück; JSF meldet bei weicher Kante wie das Original `edgeType` „antialiased“ + `feather`. |
| 18/12 | Texturnamen wie in der Namenstabelle des Originals (Argyle 01/02, Gips, Maschen, Schraffur/Klammer, Korn, Klaviatur, Karo weit/eng, Linie-Diag/Horiz/Vert). |
| 18/20 | Verlaufsschritt „Füllen“ (`cmd-pi-fill`). |
| 18/21 | Werkzeug-Inspektor (Rechteck, Ellipse, Stift …) ohne Pfadoperationen; „Muster“ auch für das Werkzeug (`ToolAttrs::pattern`). |
| 18/22 | Eine Füllungsänderung an der Auswahl wird Werkzeugvorgabe (Art, Verlaufsart, Muster, Kante, Textur, Farbe) – neue Objekte erhalten sie. |

Nebenbei: Esc in einem offenen Verlaufs-/Muster-Popup hebt die Auswahl nicht auf (`input.rs`).

## Tests

- `crates\fp-engine\tests\parx_insp1.rs` (7): Arten/Kategorien, Vorgabeverlauf und Griffe, Kontur, Winkel 50× = einmal + Rückgängig, Muster, Kante, „Füllen“, Rundung nach B/H, Strich skalieren + `pixelRect` + Rückgängig.
- `crates\fp-script\tests\parx_insp1.rs` (2): `exportOptions`, `roundness`/`mode`, `edgeType`, `pixelRect`.
- `crates\fp-ui\tests\inspector_parx_insp1.rs` (7): Knopf öffnet Popup, Liste schließt es nicht, gleiche Art kein Schritt, Esc; Stopp herausziehen; Werkzeug übernimmt Füllung; Pfadoperationen/Muster; Höhenwechsel; Exportreihenfolge; Musternamen.
- angepasst: `inspector_fv15::w21_pattern_list_like_texture_list`, `w21_counts_like_original_when_installed` (Argyle unübersetzt), `nz1::w21_pattern_list_as_wide_as_longest_name`.

## Offen / Annahmen

- 16/08: Die Wahl im Exportformat legt weiter einen Verlaufsschritt „Optimieren“ an (im Original nicht gemessen).
- 18/04: Kein eigener Farbrad-Knopf im Verlaufs-Popup (der Farbwähler des Stopps hat den Farbkreis).
- 18/07: Griffe werden gezeichnet, aber noch nicht gezogen (Ziehen der Verlaufsgriffe = eigener Befund).
- Pfadoperation mit nur einem Pfad: Knopf bedienbar, ohne Wirkung (Original nicht gemessen).
- „Course-Orange“ steht in der Namenstabelle zweimal; genommen ist der erste Eintrag („Schuppig-orange“); „Wood“ → „Holz“.
- Kombifeld-Listen sind jetzt allgemein so breit wie der längste Eintrag (auch außerhalb des Inspektors).

## Nachprüfung (echtes Fenster, Brücke)

1. Leinwandfarbe-Popup: kein Deckkraftfeld; HEX + Enter setzt die Farbe.
2. Doppelklick auf „Eigenschaften“ dreimal: halb → nur Reiter → voll.
3. R1 aus `insp_t04.fw.png`: „Verlaufsfläche“ → Popup offen, Liste „Kontur“ wählen → Popup bleibt; Export je Art gegen `re\_testdateien\par\px\inspector-fill\stats.txt`.
4. Farbstopp anklicken → Farbwähler; Stopp nach unten ziehen → gelöscht; Winkel 0 → Verlauf waagerecht; Griffe auf der Leinwand.
5. „Muster“ → Baumwolle, Popup mit Vorschau; Export ≈ Mittel #8c8c8c.
6. Mischmodus-Liste ganz lesbar; „+“-Menü ≈ 272 px, „Ohne“ oben.
7. E1 8 px außen, B 100 → 200: Strich 11, `pixelRect` 39,159–261,251.
8. Rechteck-Werkzeug: keine Pfadoperationen, „Muster“ aktiv; R1 Verlauf „Wellen“, neues Rechteck zeichnen → gleicher Verlauf.

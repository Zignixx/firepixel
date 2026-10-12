# BV-4: Bildvorschau, Paket P4 (Verhalten, Fenster, Texte)

Stand 13.10.2026, Slot s1. Cleanroom: Die einzige Quelle war `re\17-bildvorschau.md` (§6, §7, §8, §10 P4, §11), dazu die STATUS-Abschnitte BV-1 bis BV-3. Das Original habe ich nicht benutzt.

## Umgesetzt (Commit `ad67a00` auf main, im Slot `b52596f`)

Dateien: `crates\fp-ui\src\dialogs\image_preview\mod.rs`, `options.rs`, neu `crates\fp-ui\tests\image_preview_p4.rs`.

- **OK (§7):** Die Einstellung des **aktiven** Feldes wird die Exporteinstellung der Seite. Dazu gehören Format, Palette, Skalierung, Exportbereich, Animation, Hintergrund und `name`. Neue Funktion `image_preview::commit_settings`.
  - Das ist **ein** Verlaufsschritt „Optimieren“ (`pi.export.settings`). §7 nennt einen Rückgängig-Schritt (Markierung A).
  - Ändert sich nichts, entsteht kein Schritt.
- **Exportieren… (§7):**
  - Übernimmt die Einstellung wie OK (ein Schritt) und öffnet den Exportdialog E1 (`ExportDialog::new`) **über** der Bildvorschau. Die Dateiendung folgt sofort dem neuen Format.
  - Abbruch bzw. Esc im Exportdialog führt zurück in die Bildvorschau (Feld `ImagePreview::export_child`).
  - Wurde die Bildvorschau über „Optionen…“ des Exportdialogs geöffnet, schließt sie nach „Exportieren…“ bzw. OK. Es geht dann im Exportdialog weiter, dessen Endung `page_ext` und Dateiname ebenfalls nachgeführt werden.
  - Der alte Weg über den Speichern-Dialog (`export_with`) ist entfernt.
- **Esc/Abbrechen:** schließen ohne Änderung am Dokument. Esc in einem Unterdialog schließt nur diesen: Vorgabenname, Farbe, Für Dateigröße optimieren, Meldung, Exportdialog. Die egui-Modals prüfen ohnehin nur das oberste Fenster; die Tests sichern das jetzt ab.
- **Esc beim Ziehen mit „Exportbereich“:** bricht das Ziehen ab und stellt den Bereich von vorher wieder her (auch „kein Bereich“). Die Taste wird dabei verbraucht, der Dialog bleibt offen. Weiterziehen bis zum Loslassen wirkt nicht mehr, ein neues Ziehen wirkt normal.
- **Mausrad (§11.5, im Original nicht gemessen):** verhält sich wie im Dokumentfenster.
  - Rad: alle Felder senkrecht verschieben, 52 px je Zeilen-Rastung. Das waagerechte Rad verschiebt waagerecht.
  - Umschalt+Rad: tut nichts.
  - Strg+Rad: je volle Rastung genau eine der 11 Zoomstufen, um den Mauszeiger. Kleine Touchpad-Schritte werden gesammelt (100 Punkte = eine Rastung).
  - Wirkt nur über einem Bild und nur, wenn die Bildvorschau oben liegt.
- **RAND-1:** „Browseranzeige mit Zeilensprungverfahren“ ist jetzt so breit wie „Nicht verwendete Farben entfernen“. Die alten −30 px waren unnötig, der Knopf „Optimieren auf Größe“ liegt darunter.
- **Vorgaben (§8.2):** Speichern über „+“ gab es schon (BV-1). Neu durch Tests abgesichert:
  - Der Name wird ohne Leerraum gespeichert.
  - Ein leerer Name lässt sich nicht speichern.
  - Derselbe Name überschreibt die Vorgabe, es entsteht kein zweiter Eintrag.
  - Die Vorgabe erscheint sofort in allen Vorgabenlisten.
  - **Löschen** gibt es nach §8.2 nur im Optimieren-Bedienfeld (`panel.optimize.deleteSettings`), nicht in der Bildvorschau. Die offene Bildvorschau sieht das Löschen sofort.

## Entscheidungen nach Spezifikation

- **Verzögerung, Löschmethode und Sichtbarkeit bei OK:** §4 („?“) und §11.7 lassen offen, ob sie in die Status des Dokuments zurückgehen. Entscheidung: **nein, nur in die Exporteinstellung** (`frameInfo` der `ExportSettings`).
  - Wichtig: In Firepixel gibt es keinen getrennten Speicher für diese Werte. Wie im Original (§4: „stehen in `frameInfo[]` der ExportOptions“) liegen sie nur in `export.frames`, und auch `pi.state.props` schreibt dorthin. Das Status-Bedienfeld zeigt deshalb nach OK die Werte des aktiven Feldes.
  - Sonst ändert OK an den Status nichts (Namen, Anzahl, aktueller Status).
- **§11.2 (Bildvorschau nach erfolgreichem Export):** Nach dem Schließen des Exportdialogs, egal ob exportiert oder abgebrochen, geht es in der Bildvorschau weiter. Ob der Export erfolgreich war, ist von außen nicht erkennbar, ohne `export.rs` zu ändern.
  - Danach schließt OK oder Abbrechen die Vorschau.
  - „Exportieren…“ hat die Einstellung schon in einem eigenen Verlaufsschritt übernommen. Ein späteres Abbrechen nimmt das nicht zurück, Rückgängig schon.
- **Fenstergröße/Modus:** werden wie im Original nicht gemerkt (P4.1). Das hatte BV-1 schon so umgesetzt, keine Änderung.

## Tests

`crates\fp-ui\tests\image_preview_p4.rs` (7 Tests, echte Maus-, Rad- und Tastaturereignisse):
- `ok_writes_the_active_pane_in_one_undo_step`
- `esc_and_cancel_change_nothing_and_sub_dialogs_close_alone`
- `export_button_applies_and_opens_the_export_dialog`
- `esc_cancels_an_export_area_drag_only`
- `mouse_wheel_scrolls_and_zooms_like_the_document_window`
- `user_presets_save_overwrite_and_delete`
- `interlaced_checkbox_has_full_width`

Grün:
- `image_preview_p1::`, `p2::`, `p3::`, `p4::` (26 Tests)
- `panel_menus_mn3::`, `dialogs_move::`, `dialogs_open::`, `flattened_png_eb2::`
- `cargo check --workspace --all-targets` ohne Warnungen
- `cargo test -p fp-neutral`

Neue UI-Texte gibt es nicht, deshalb liefen fp-i18n und der Scanner nicht.

## Befunde

- Erledigt: RAND-1 „Bildvorschau ‚Browseranzeige mit Zeilensprungverfahren‘: Das Kästchen ist 30 px zu schmal …“
- Erledigt: BV-3-Hinweise „Esc während des Ziehens mit dem Exportbereich-Werkzeug …“ und „Mausrad in der Vorschau (§11.5) weiter offen“.
- Erledigt: BV-1-Hinweis P4 „OK/Exportieren nutzen noch … den Dateidialog wie vorher“.

## PARITAET: auf „umgesetzt, Nachprüfung ausstehend“

- Bildvorschau OK (aktives Feld → Seite, ein Verlaufsschritt)
- Bildvorschau Exportieren… (Exportdialog mit der Einstellung, Abbruch → zurück)
- Esc/Abbrechen
- Mausrad
- D08 „Esc schließt nicht“ ist laut §10 P4.2 falsch: Esc = Abbrechen.

## Nachprüfung (echtes Fenster bzw. Brücke)

1. Datei ▸ Bildvorschau…, 2 Felder, unten JPEG, OK. Erwartet: Das Optimieren-Bedienfeld zeigt JPEG. Im Verlauf steht ein Schritt „Optimieren“, Strg+Z stellt den alten Stand her.
2. Bildvorschau, Exportieren…. Erwartet: Der Exportdialog liegt darüber, die Endung passt zum Format. Esc schließt nur den Exportdialog, die Bildvorschau ist wieder bedienbar.
3. Exportbereich-Werkzeug, Griff ziehen, Esc mit gedrückter Maus. Erwartet: Der Rahmen springt zurück, der Dialog bleibt offen.
4. Mausrad über dem Bild bei 400 %: verschiebt das Bild. Strg+Rad: eine Rastung ist eine Stufe (auch mit Touchpad). Umschalt+Rad: nichts.
5. Optionen bei GIF: Das Kästchen „Browseranzeige mit Zeilensprungverfahren“ ist in DE und EN nicht abgeschnitten.

## Neue Befunde

- (für IBD/Optimieren-Bedienfeld) §7 nennt für „Einstellungen löschen“ die Rückfrage 3169: „Aktuelle Einstellungen löschen? Dies kann nicht rückgängig gemacht werden.“ Heute löscht der Formulardialog `panel.optimize.deleteSettings` (`dialogs\panel_menus.rs`) ohne diese Rückfrage.
- (für E1, optional) Der Exportdialog meldet nicht, ob exportiert wurde. Ein Rückgabewert bzw. Ereignis „exportiert“ würde es erlauben, die Bildvorschau nach einem erfolgreichen Export automatisch zu schließen, falls die Nachmessung zu §11.2 das ergibt.

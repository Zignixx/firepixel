# Bericht NZ-4: Kleinkram in Engine, Dateien und Export

Slot s10. Das Original wurde nicht benutzt. Quellen: `docs\BEFUNDE.md`, `docs\berichte\dlg4.md`/`mn3.md`/`kv3.md`/`e2.md`,
`re\08-export.md` §1.5, `re\18-export-dialoge.md` §4.2, `re\format\vocabulary.md` (nur gelesen).

Commits auf main: `08187d5` (alles), `a285413` (Zeilenenden von `fp-fwpng/src/write.rs` wieder CRLF, ein NUL-Byte im Kommentar korrigiert).

## Umgesetzt

### 1. PNG-Auflösung (pHYs) 2834 statt 2835 (von EB-2)

- `crates\fp-export\src\png_out.rs::trailer_meta`: `floor` statt `round` (mit Epsilon 1e-6, damit z. B. 254 dpi
  genau 10000 bleibt). Das gilt für den PNG-Export und das reduzierte PNG, weil beide denselben Schreiber nutzen.
- Der Test `flattened_png_eb2.rs` ließ bisher 2834–2835 zu und prüft jetzt genau 2834. Golden-Daten in
  fp-export/fp-io waren nicht betroffen.
- Nicht geändert: Der BMP-Schreiber (`formats.rs` Z. 201) rundet weiter, und `fp-fwpng::png::make_phys`
  (FW-PNG) auch. Für beide gibt es keine Messung, siehe „Neue Befunde“.

### 2. „Neue Schaltfläche“ öffnet die Symbolbearbeitung (von DLG-4, D14)

- Engine `edit.insert.newButton` hat jetzt den Parameter `edit` (Vorgabe `false`, damit Skript `addNewSymbol` gleich bleibt).
  Mit `edit: true` wird eine leere Schaltfläche in der Bibliothek angelegt und ihre Symbolbearbeitung geöffnet.
  Dabei entsteht keine Instanz im Dokument. Der Code kommt aus der gemeinsamen Hilfsfunktion
  `new_empty_symbol` (`fp-engine\src\commands\edit.rs`), die jetzt auch „Neues Symbol“ nutzt.
- Das Menüformular (`menu_dialogs.rs`) übergibt `{"edit": true}`. Die Statuszeile des Originals sagt dazu:
  „Erstellt ein neues Schaltflächenobjekt im Schaltflächeneditor“ (`commands.json`).

### 3. Fußknopf „Auf Status verteilen“ (von MN-3)

- `panels\states.rs`: aktiv nach `distribute_enabled(st)` = `is_available("pi.state.distribute")`, also ab einem
  gewählten Objekt wie im Menü (vorher erst ab 2).

### 4. Alte Formulare der Bitmap-Auswahl entfernt (von KV-3)

- `dialogs\catalog.rs::plain_form`: Die Zweige für `select.saveBitmapSelection`, `restoreBitmapSelection` und
  `deleteBitmapSelection` sind entfernt. Den Dialog liefert `bitmap_selection.rs`.
- Nicht mehr benutzte Schlüssel aus `locales\{de,en}\dialoge.ftl` entfernt: `dlg-no-saved-selections`,
  `dlg-op-replace/add/subtract/intersect`, `dlg-title-selection` und `dlg-lbl-operation`. `dlg-selection-default-name`,
  `dlg-lbl-selection` und die drei Titel bleiben, weil sie noch benutzt werden.

### 5. Sprite-Einstellungen im FW-PNG (von E2)

- Die Tags des Originals sind unbekannt. `re\08` §1.5 nennt „Weitere JS-Felder ohne Tag im Korpus: …
  `spriteExport*` … (Tags ?)“, und `vocabulary.md` hat kein passendes Tag.
- Deshalb gibt es eine **Firepixel-eigene Erweiterung** über den vorhandenen Mechanismus von fp-fwpng. Unbekannte
  Tags laufen dort als `Extra` durch, und der Leser verarbeitet auch unbekannte Tags (`vocab.rs`).
  - Container `fpSv` im `EXPv` mit `SSCb` (class/id), `SPXs` (Präfix), `SSXs` (Suffix), `SPDi` (Auffüllung),
    `SORi` (Ausrichtung 1/2/3) und `SPRs` (Vorgabename).
  - Er wird nur geschrieben, wenn mindestens ein Sprite-Feld gesetzt ist. Dateien ohne Sprite-Werte bleiben
    byte-gleich wie vorher.
  - Begründung der Benennung: Alle Tags des Originals bestehen aus drei Großbuchstaben bzw. Ziffern und einem
    Typbuchstaben (alle 437 Zeilen in `vocabulary.md`). Ein Container, der mit Kleinbuchstaben beginnt, kann
    deshalb mit keinem Tag des Originals zusammenfallen. Innerhalb des eigenen Containers sind die Kindnamen frei.
  - Das Original ignoriert bzw. übergeht unbekannte Tags vermutlich. Gemessen ist das **nicht**, siehe Nachprüfung.
- Dateien: `fp-fwpng\src\write.rs` (`SPRITE_EXT`, `write_sprite_ext`), `read.rs` (`read_html`, Schlüsselliste) und der
  Doku-Kommentar in `fp-model\src\export.rs`.

### 6. Zusatzfelder im App-eigenen Dateidialog (von DLG-3/EB-2)

Das gilt nur für den Dialog in der App (Brücke). Der Windows-Dialog liefert ohne `unsafe` keine Zusatzfelder, dort
bleiben die Vorgaben des Originals: `.fw.png` angehängt, kein Unbenannt, keine Animation.
- **Öffnen**: Kästchen „Als „Unbenannt“ öffnen“ und „Als Animation öffnen“ (grau, bis mehr als eine Datei im Feld steht).
  - `file_dialog::request_open` und `OpenChoice`; `Request::open_extras`.
  - `files::pick_and_open` ersetzt in `actions.rs` den bisherigen Aufruf. `files::open_chosen` öffnet nach den Kästchen.
- **Speichern unter**: Kästchen „.fw.png anhängen“ (nur beim Typ FW-PNG, Vorgabe an).
  - `TypeExtras::append_fw`, `SaveChoice::append_fw_png` und `files::save_target_for_ex`.
  - Aus: Der gewählte Name bleibt, ohne Endung wird `.png` angehängt. Die Datei bleibt trotzdem FW-PNG.
- Engine `file.open` hat neue Parameter:
  - `untitled` (öffnet ohne Dateinamen, Speichern fragt nach einem Namen)
  - `frames` (weitere Pfade; jede Datei wird ein weiterer Status im ersten Dokument, das Dokument ist unbenannt)
- Agenten bzw. Tests können die Kästchen mit `file_dialog::set_flags(..)` setzen. `ui_state` zeigt `append_fw_png`,
  `open_untitled` und `open_as_animation`. Das Ereignis „chosen“ protokolliert alle Kästchen.
- Neue Texte in `locales\{de,en}\dateidialog.ftl`: `fdlg-append-fw-png`, `fdlg-open-untitled` und `fdlg-open-as-animation`.

### 7. Startordner „Bilder“ beim Export (von E1)

- `fp_i18n::profile::known_folder(KnownFolder::Pictures | Documents)`: `<home>\Pictures` bzw. `\Documents`, wenn
  vorhanden, sonst das Home-Verzeichnis. Home ist `%USERPROFILE%` (sonst `$HOME`), im Testlauf die Wurzel des
  Testprofils. Die Funktion legt nichts an.
- `dialogs\export.rs`: Als letzter Rückfall der Ordnerkette kommt jetzt „Bilder“ (vorher leer).

## Tests

Neue Datei `crates\fp-ui\tests\nz4.rs` (`cargo test -p fp-ui --test all nz4::`):
- pHYs 2834 beim PNG-Export
- Neue Schaltfläche: Formular mit `edit`, Symbolbearbeitung, keine Instanz; ohne `edit` wie bisher
- Fußknopf ab einem Objekt, gleich dem Menü
- Sprite-Felder nach Speichern und Laden, kein doppelter Container
- Öffnen als Unbenannt bzw. als Animation (3 Dateien ergeben 3 Status; bei einer Datei wirkt das Kästchen nicht)
- „.fw.png anhängen“ aus und an
- Texte de/en
- `known_folder` bleibt im Testprofil

Dazu lief `flattened_png_eb2::` (genau 2834).

## Erledigte Befunde (Wortlaut-Anfang)

- „(von EB-2) PNG-Schreiber `pHYs`: `png_out.rs` rundet 72 dpi auf 2835 …“
- „(von DLG-4, PARITAET_B D14, `form.rs`) **Neues Symbol: …** Außerdem fügt „Neue Schaltfläche“ weiter sofort eine leere
  Instanz ein …“: der Teil zu „Neue Schaltfläche“ ist erledigt, „In allgemeiner Bibliothek speichern“ bleibt offen.
- „(von KV-3, Aufräumen `catalog.rs`) Die alten Formulare der drei Bitmap-Auswahl-Befehle …“: die Formulare und Schlüssel
  sind erledigt, die Live-Anzeige beim Wiederherstellen bleibt offen.
- „(von DLG-3) Zusatzfelder der Datei-Dialoge fehlen: …“: im Dialog in der App erledigt für Unbenannt, Animation,
  .fw.png anhängen und Als Kopie (EB-2). Offen bleiben Vorschau, „XMP einschließen“ und der native Dialog.
- MN-3 (Fußknopf „Auf Status verteilen“ erst ab 2 Objekten), E1 (Startordner „Bilder“) und E2 (Sprite-Felder gehen
  beim Speichern verloren), jeweils aus den Berichten.

## PARITAET: „umgesetzt, Nachprüfung ausstehend“

- D14 Neue Schaltfläche, D02–D06 Zusatzfelder (nur Dialog in der App), Status-Bedienfeld Fußknopf, Export-Startordner.

## Nachprüfung

1. Brücke an: Datei ▸ Öffnen.
   - Erwartet: Zwei Kästchen; „Als Animation öffnen“ ist grau, bis zwei Pfade mit `;` getrennt im Feld stehen.
   - Mit dem Haken entsteht ein unbenanntes Dokument mit einem Status je Datei.
   - „Als „Unbenannt“ öffnen“ ergibt einen Titel ohne Dateinamen, und Strg+S fragt nach einem Namen.
2. Speichern unter, Typ FW-PNG, „.fw.png anhängen“ aus, Name `x.png`: Erwartet wird die Datei `x.png` als FW-PNG.
   Bei anderen Typen ist das Kästchen nicht sichtbar.
3. Bearbeiten ▸ Objekt einfügen ▸ Neue Schaltfläche…, OK: Erwartet wird die Symbolbearbeitung einer leeren
   Schaltfläche, ohne Instanz im Dokument.
4. Status-Bedienfeld mit einem gewählten Objekt: Erwartet wird, dass der Fußknopf „Auf Status verteilen“ aktiv ist.
5. CSS-Sprite-Optionen ändern, speichern, schließen und öffnen: Erwartet werden dieselben Werte.
   - Zusätzlich, wenn möglich: Eine solche Datei im Original öffnen. Erwartet wird, dass sie ohne Fehlermeldung
     aufgeht und der unbekannte Container übergangen wird. Das ist nicht gemessen.
6. Export ohne früheren Export- bzw. Öffnen-Ordner und mit unbenanntem Dokument: Erwartet wird der Ordner „Bilder“ als Start.

## Neue Befunde

- (NZ-4) BMP-Schreiber `fp-export\src\formats.rs` Z. 201 und `fp-fwpng\src\png.rs::make_phys` (FW-PNG) runden die
  Auflösung weiter. Ob das Original auch dort abschneidet, ist nicht gemessen; vermutlich ja (gleiche Rechnung).
- (NZ-4) `ui_file_dialog` (Brücke, `bridge.rs`) kennt die neuen Kästchen noch nicht als Parameter. `file_dialog::set_flags` ist da,
  die Einhängung in `bridge.rs` (AU-Bereich) fehlt.
- (NZ-4) „Als Animation öffnen“ übernimmt aus den weiteren Dateien nur die Elemente der normalen Ebenen der ersten Seite.
  Symbole dieser Dateien werden nicht mit übernommen, Instanzen darin verlieren dann ihr Symbol. Die Leinwand hat
  die Größe der ersten Datei.
- (NZ-4) `files::documents_dir` nutzt weiter `std::env::home_dir()` ohne Testprofil-Umleitung. Es könnte auf
  `profile::known_folder(Documents)` umgestellt werden; das ändert den Startordner in Tests.

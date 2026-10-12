# Bericht KV-3 (Paket KT-3): Kurven/Tonwert als Live-Effekt, Bitmap-Auswahl-Dialoge

Grundlage: `re\19-kurven-tonwert.md` §1.1, §5, §6 (Cleanroom, nur diese Quelle).

## Umgesetzt

### 1. Kurven und Tonwertkorrektur als Live-Effekt

- „+“ ▸ Farbe anpassen ▸ Kurven… bzw. Tonwertkorrektur… öffnet jetzt **denselben Dialog wie der Filter**
  (Grundwerte, Vorschau auf dem Objekt). OK hängt den Effekt an (ein Verlaufsschritt), Abbrechen legt
  keinen an. Vorher wurde ein leerer Effekt ohne Dialog angelegt.
- Kurven- und Tonwert-Effekte haben jetzt ein ⓘ (`effects::editable`). ⓘ oder ein Doppelklick öffnet den
  Dialog mit den **gespeicherten Werten**. OK ersetzt die Werte (`pi.effect.set`, ein Schritt, kein
  zweiter Effekt), Abbrechen lässt sie unverändert (die Vorschau wird ohne Verlaufsschritt zurückgenommen).
  Das Ziel (Effektindex und gewählte Objekte) wird beim Öffnen festgehalten (`effects::LiveEdit`).
- Engine-Fehler behoben: `live_effect` schrieb für Kurven/Tonwert **keine Parameter**. Jetzt werden alle
  vier Punktlisten als `GPLv` (`rgb_points` … `blue_points`) bzw. die 20 Tonwertschlüssel
  (`source_low_rgb` … `gamma_blue`) geschrieben (§6.1).
- `pi.effect.set` nimmt jetzt auch Punktlisten an (`[[x, y], …]` oder `[{x, y}, …]`, gespeichert als `GPLv`).
  Textwerte bleiben wie bisher.
- Dateien: `crates\fp-ui\src\dialogs\effects.rs` (Anbindung), `curves.rs` und `levels.rs` (je ein Feld
  `edit` und ein Zweig in `command_params`), `crates\fp-engine\src\commands\filters.rs`, `panel.rs`.

### 2. Auswählen ▸ Bitmap-Auswahl speichern…, wiederherstellen… und löschen…

- Neue Datei `crates\fp-ui\src\dialogs\bitmap_selection.rs`, eingehängt in `dialogs\mod.rs` (Modul, Feld,
  `any_open`, Aufruf in `open_for_command` vor dem Katalog, `show_all`). `catalog.rs` ist unverändert,
  seine alten Formulare für die drei Befehle werden nicht mehr erreicht.
- **Speichern** (§5.2): Gruppe „Ziel“ mit „Dokument:“ (nur offene Dokumente **gleicher Leinwandgröße**,
  vorgewählt das aktuelle), „Auswahl:“ (erster Eintrag „Neu“, dann die gespeicherten Namen; Dokumentwechsel
  füllt neu) und „Name:“ (Vorgabe **„Standard“**, nur bei „Neu“ aktiv). Gruppe „Vorgang“: „Neue Auswahl“
  bzw. „Auswahl ersetzen“, „Zur Auswahl hinzufügen“, „Aus Auswahl entfernen“, „Schnittmenge mit Auswahl“.
  Die drei Verknüpfungen sind nur aktiv, wenn eine vorhandene Auswahl gewählt ist. Gleicher Name bei „Neu“
  → **Rückfrage mit OK/Abbrechen**. Abbrechen kehrt in den offenen Dialog zurück, OK ersetzt.
- **Wiederherstellen** (§5.3): Quelle mit Dokument (gleiche Größe und gespeicherte Auswahlen), Auswahl
  (vorgewählt die erste) und **„Umkehren“**. Vorgang mit vier Optionsfeldern; Hinzufügen/Abziehen/Schnittmenge
  sind **nur aktiv, wenn eine Pixelauswahl besteht**.
- **Löschen** (§5.4): Dokument und Auswahl; OK löscht ohne Rückfrage.
- Nichts wird gemerkt (§5.5).
- Engine (`select.rs`):
  - `select.saveBitmapSelection`: `name` (Vorgabe „Standard“), `mode` (replace/add/subtract/intersect: Verknüpfung
    mit der vorhandenen gleichnamigen Auswahl) und `document` (Ziel-ID, gleiche Größe).
  - `select.restoreBitmapSelection`: zusätzlich `invert` und `document` (Quelle).
  - `select.deleteBitmapSelection`: zusätzlich `document`.
  - Speicherort ist die aktive Seite des Dokuments. Speichern ist ein eigener Schritt „Auswahl speichern“,
    Laden ein eigener Schritt „Auswahl laden“. Speichern in ein anderes Dokument ist ein Schritt in diesem
    Dokument.
  - Menüfreigabe Wiederherstellen/Löschen: aktiv, wenn ein offenes Dokument gleicher Größe gespeicherte
    Auswahlen hat.
- Texte: neue Bereichsdateien `crates\fp-i18n\locales\{de,en}\bitmap_selection.ftl` (`bsel-*`,
  `sel-saved-default-name`, `msg-step-save-selection`, `msg-step-load-selection`, `err-sel-document-size`).

### Tests

`crates\fp-ui\tests\kv3.rs`, 8 Tests, alle grün:
- Kurven über „+“: Abbrechen legt nichts an, OK hängt an (alle 4 Listen).
- Kurven über ⓘ: Startwerte gespeichert, Abbrechen unverändert, OK ersetzt, Rückgängig.
- Tonwert: Anlegen, Bearbeiten, Abbrechen und OK.
- Speichern: Dokumentliste, „Neu“, „Standard“, Aktiv-Regeln, Schritt „Auswahl speichern“, Rückgängig/Wiederholen.
- Gleicher Name: Rückfrage, Abbrechen zurück in den Dialog, OK ersetzt.
- Verknüpfen in eine vorhandene Auswahl (add/intersect/subtract), Speichern in ein anderes Dokument,
  falsche Größe wird abgelehnt.
- Wiederherstellen: Aktiv-Regel mit und ohne Pixelauswahl, Umkehren, Schritt „Auswahl laden“, Laden aus
  einem anderen Dokument mit Hinzufügen.
- Löschen: Abbrechen löscht nichts, OK ohne Rückfrage, Rückgängig.

Mitgeprüft:
- fp-ui: `dialogs_dlg1::`, `dialogs_move::`, `dialogs_open::`, `curves_kv1::`, `levels_kv2::`
- fp-engine: `commands::`, `all_commands::`
- `cargo check --workspace --all-targets`, `cargo test -p fp-i18n`, Scanner `i18n_keine_festen_texte`

## Commits

siehe Ende dieses Berichts.

## PARITAET: „umgesetzt, Nachprüfung ausstehend“

- re\19 §7.1 K14, §7.2 T10: Kurven/Tonwert als Live-Effekt über ⓘ bearbeitbar, „+“ öffnet den Dialog
- re\19 §7.3 A1–A4: Dialoge Bitmap-Auswahl speichern/wiederherstellen/löschen

## Nachprüfung (echtes Fenster, Brücke)

1. Rechteck wählen, Inspektor „Filter: +“ ▸ Farbe anpassen ▸ Kurven…
   - Erwartet: der Kurvendialog öffnet sich.
   - Punkt ziehen, Abbrechen → die Effektliste bleibt leer.
   - Noch einmal mit OK → Eintrag „Kurven…“ mit ⓘ am Ende der Liste.
2. ⓘ des Kurven-Effekts → der Dialog zeigt die gespeicherte Kurve.
   - Punkt verschieben, Abbrechen → Darstellung wie vorher.
   - Erneut mit OK → neue Kurve, Verlauf +1.
3. Dasselbe mit der Tonwertkorrektur.
4. Bitmap, Auswahlrahmen, Auswählen ▸ Bitmap-Auswahl speichern…
   - Erwartet: Dokument (nur gleich große), Auswahl „Neu“, Name „Standard“, drei Vorgänge grau.
   - OK → Verlauf „Auswahl speichern“.
   - Erneut mit „Standard“ → Rückfrage; Abbrechen führt zurück in den Dialog.
5. Strg+D, dann Bitmap-Auswahl wiederherstellen…
   - Erwartet: drei Vorgänge grau.
   - Umkehren an, OK → Rahmen um alles außer dem Rechteck, Verlauf „Auswahl laden“.
6. Bitmap-Auswahl löschen… → OK entfernt den Eintrag ohne Rückfrage.
7. Englisch und qps: Texte der neuen Dialoge laufen nicht über (Optionsfeld-Texte 237 px breit, Beschriftungen
   gekürzt mit `fit_label`).

## Abweichungen und Annahmen

- Die Live-Anzeige der entstehenden Auswahl im Dialog „Wiederherstellen“ (§5.3) fehlt noch. Die Quelle
  markiert sie nur als A. Übernommen wird erst bei OK, Abbrechen ändert nichts.
- Speichern mit Verknüpfung, deren Ergebnis leer ist, wird mit der Meldung „leere Pixelauswahl“ abgelehnt
  (kein Schritt). Die Quelle sagt dazu nichts.
- Der Namensvergleich bei der Rückfrage unterscheidet Groß- und Kleinschreibung (die Quelle lässt das offen).

## Neue Befunde

- Die alten Formulare der drei Befehle in `catalog.rs` (`plain_form`) und die Schlüssel `dlg-selection-default-name`
  (Fallback für unbenannte Auswahlen, wird weiter genutzt), `dlg-no-saved-selections`, `dlg-op-*` werden für
  diese Befehle nicht mehr gebraucht. Aufräumen kann der Besitzer von `catalog.rs` (DLG-n).

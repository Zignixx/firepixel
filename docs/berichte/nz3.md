# NZ-3: Kleinkram an der Oberfläche (Slot s9)

Commit: `3a3342b` auf main (im Slot `33d908d`, sauber per cherry-pick übernommen).

## Umgesetzt

1. **Bildlaufleisten in allen Listen** (Befund NZ-2, „Griff der egui-Bildlaufleisten unsichtbar“):
   - `theme.rs` hat die neue Funktion `scrollbar_widgets`, aufgerufen in `Theme::visuals()`. Damit gilt in allen 3 Designs app-weit: Spur `extreme_bg_color = scrollbar_bg`, Griff `widgets.inactive.bg_fill = scrollbar_thumb`. Unter der Maus bzw. beim Ziehen ist der Griff kräftiger (`mix(thumb, text, 0.3/0.5)`).
   - Textfelder behalten ihren Grund über `text_edit_bg_color`.
   - `bg_fill` benutzen sonst nur die egui-eigenen Kästchen, der Schieber und der Farbknopf. Diese 6 Stellen zeichnen über `theme::with_field_widgets` bzw. `field_widgets` unverändert im Feldstil: `file_dialog.rs` (2), `panels\states.rs`, `welcome.rs`, `inspector\scale.rs` (Schieber), `dialogs\new_document.rs` (Farbknopf).
   - `inspector\fonts.rs`: `list_scrollbar_style(ui)` setzt keine Farben mehr, sondern nur noch die Leistenbreite 8 px (`LIST_BAR_W`).
2. **Schrift-Menü** (von MN-2):
   - ✓ am Familienkopf: Die neue Funktion `menu::sub_head_checked` liefert ✓ am Kopf eines Untermenüs, wenn einer seiner Schnitte gewählt ist. Das gilt nur in langen Listen (> `LONG_SUBMENU` = 40, also Text ▸ Schrift). Kurze Untermenüs wie Ansicht ▸ Design bleiben ohne Häkchen. Benutzt wird die Funktion in `show_nodes` und in der Tastaturnavigation `menu_kbd.rs` (1 Zeile).
   - `Text_Font` wird gemerkt: `fp-engine\src\commands\text.rs` schreibt den Wert jetzt direkt in `prefs.values`, auch für die Werksvorgabe Arial. Bisher hat `prefs.set` den Schlüssel bei Gleichheit mit der Vorgabe entfernt, und nach einem Neustart galt wieder die Vorzugsschrift des Textwerkzeugs.
3. **Kurven und Tonwert** (KV-1/KV-2, re\19 §1.4, §3.5):
   - Beim Drücken auf einen Griff bekommt das zugehörige Feld den Fokus, und der Inhalt wird markiert:
     - Tonwert: Schwarz-, Gamma- und Weißgriff der Eingabe → Feld 1/2/3; Griffe der Ausgabe → Feld min/max
     - Kurven: gegriffener Punkt → Feld „Eingabe“
   - Auch nach einem Klick ohne Bewegung bleibt der Fokus im Feld. egui wertet das Loslassen sonst als „Klick daneben“, deshalb wird der Fokus beim Loslassen neu gesetzt.
   - Zugriffstasten, neue Datei `dialogs\access.rs` mit `label_right`, `checkbox`, `pressed`, `focus_field` und `split_access`:
     - Kurven: &Eingabe, &Ausgabe (fokussieren die Felder), A&uto, &Vorschau
     - Tonwert: E&ingabetonwerte, A&usgabetonwerte, &Auto, &Vorschau (neuer Schlüssel `dlg-levels-preview`; `dlg-preview` bleibt für die Formulare ohne `&`)
     - Alt+Taste wirkt jetzt auch für Auto; bisher war der Buchstabe nur unterstrichen.
4. **Bildgröße**: neuer Schlüssel `dlg-image-size-constrain` = „&Proportionen beibehalten“; `dlg-image-size-resample` = „&Bild neu auflösen“. Englisch: &Constrain proportions / &Resample image. Die Kästchen sind unterstrichen, Alt+P und Alt+B schalten sie um. Der gemeinsame `dlg-constrain` (Numerisch transformieren) bleibt unverändert.
5. **Meldung mit Info-Symbol** (DLG-4, D25): `dialogs::show_message` zeigt das Info-Symbol (32 px, `KitIcon::Info`) links. Der Text steht daneben, ist bei kurzem Text mittig zum Symbol und wird bei längerem umbrochen, die Höhe wächst mit.
6. **`btn-help`** = „&Hilfe“ / „&Help“ in `allgemein.ftl` (de/en). Die Voreinstellungen nutzen jetzt diesen Schlüssel (Alt+H wirkt), `dlg-pref-help` ist entfernt.
7. **Unbenutzte Schlüssel entfernt** (de/en `dialoge.ftl`): `dlg-auto`, `dlg-fx-all-on`, `dlg-fx-all-off`, `dlg-fx-save-style`, dazu `dlg-pref-help`. Vorher per Grep geprüft: kein Treffer in `*.rs`, `*.txt`, `*.json` oder `*.toml`, auch kein dynamisch zusammengesetzter Schlüssel.
8. **GLOSSAR.md**: „Eingabetonwerte/Ausgabetonwerte = Input/Output Levels“ nachgetragen, im Abschnitt Dialoge.

## Tests

`crates\fp-ui\tests\nz3.rs` mit 12 Tests:
- Visuals in allen 3 Designs, gezeichnete Spur und Griff einer beliebigen Liste, Feldstil der Kästchen
- ✓ am Familienkopf, synthetisch und mit einer echten Schriftfamilie
- Arial wird gemerkt, auch nach einem JSON-Rundlauf
- Zerlegen der Zugriffstaste, Texte de/en
- Kurven: Alt+V/A/E und Punktgriff mit Fokus
- Tonwert: Alt+I/U/V, Gammagriff mit Fokus während des Ziehens, Weißgriff der Ausgabe mit Klick ohne Bewegung
- Bildgröße: Alt+P/B
- Meldung: Text rechts vom Symbol

Grün:
- `nz3::` 12/12, `curves_kv1::` 16, `levels_kv2::` 10, `font_list_scroll_nz2::` 2, `menu_mn2::` 9, `kv3::` 8, `dialogs_dlg1b::` 6, `dialogs_dlg4::` 5, `i18n_keine_festen_texte::` (Scanner)
- `cargo test -p fp-i18n`, `cargo test -p fp-neutral`, `cargo check --workspace --all-targets` (auch auf dem neuen main nach dem cherry-pick)
- clippy ohne Warnungen in den geänderten Stellen

## Befunde erledigt (Wortlaut-Anfang)

- (von NZ-2) „**Griff der egui-Bildlaufleisten unsichtbar:** `theme.rs` setzt `extreme_bg_color` …“ – erledigt (NZ-3, 3a3342b)
- (von DLG-4, PARITAET_B D25, klein) „**Gemeinsame Meldung ohne Info-Symbol:** …“ – erledigt (NZ-3, 3a3342b)
- (von DLG-4) „`btn-help` fehlt in `allgemein.ftl` …“ – erledigt (NZ-3, 3a3342b)
- MN-2 (Schrift-Menü ✓ am Familienkopf, `Text_Font` bei Arial), KV-1/KV-2 (Feldfokus beim Griff, Zugriffstasten), DLG-1b (Zugriffstasten in der Bildgröße): erledigt (NZ-3, 3a3342b)

## PARITAET → „umgesetzt, Nachprüfung ausstehend“

- D19 Bildgröße: Zugriffstasten &Proportionen und &Bild neu auflösen
- D25 Gespeicherte Befehle verwalten: Info-Meldung mit Symbol
- D43/D44 Kurven/Tonwert: Zugriffstasten und Feldfokus beim Griff

## Für die Nachprüfung (NACHPRUEFUNG)

- **Listen mit Bildlauf**, alle 3 Designs: Ebenen, Verlauf, Stile, Voreinstellungen-Kategorien, Tastaturkurzbefehle, Schriftliste. Erwartet: Der Griff ist auch ohne Maus darüber klar sichtbar und unter der Maus kräftiger.
- **Feldstil**: Kästchen „Beim Export einbeziehen“ (Status), „Nicht mehr anzeigen“ (Willkommen), „Als Kopie speichern“ (Dateidialog) und der Schieber im Pfeil-Popup sehen aus wie vorher (weißer bzw. dunkler Feldgrund).
- **Text ▸ Schrift** ohne Text: eine Familie mit Schnitten wählen. Erwartet: ✓ am Familienkopf. Dann „Arial“ wählen, Firepixel neu starten, neuen Text setzen. Erwartet: Der Text ist in Arial.
- **Kurven**: Punkt anklicken. Erwartet: Das Feld „Eingabe“ hat den Fokus und ist markiert. Alt+E, Alt+A, Alt+U und Alt+V wirken.
- **Tonwert**: Griff drücken und ziehen. Erwartet: Das zugehörige Feld hat den Fokus und folgt dem Griff. Alt+I, Alt+U, Alt+A und Alt+V wirken.
- **Bildgröße**: Alt+P und Alt+B schalten die Kästchen um, beide Buchstaben sind unterstrichen.
- **Meldung** (z. B. „Gespeicherte Befehle verwalten“ ohne gespeicherte Befehle): Erwartet: Info-Symbol links, Text daneben.
- **Voreinstellungen**: Knopf „&Hilfe“ mit unterstrichenem H, Alt+H wirkt.

## Hinweise und neue Befunde

- Die Unterstreichungen werden immer gezeigt, wie bisher bei den Knöpfen (`push_button`), nicht erst nach dem Drücken von Alt.
- `new_document.rs` hat eine eigene, gleichartige `underline_access`; sie könnte später auf `dialogs::access` umgestellt werden (keine Wirkung auf das Verhalten).
- Für die Kurven sagt die Spezifikation zum Feldfokus beim Punktgriff nichts aus (re\19 nennt ihn nur für die Tonwert-Griffe, §3.5). Umgesetzt wie beauftragt: Der Punktgriff fokussiert das Feld „Eingabe“.

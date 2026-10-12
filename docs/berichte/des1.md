# DES-1: Dialoge schöner (N11), zentral im Dialograhmen

Stand 13.10.2026 ≈ 01:10, Basis main `42dabe7` (nach RAND-2b, DLG-10, E3, NZ-3).

## Umgesetzt

Die Gestaltung sitzt zentral im Rahmen und den gemeinsamen Bausteinen, nicht in den einzelnen Dialogen. Was sich nicht ändert: Bedienung, Beschriftungen, Lage und Größe der Felder und Knöpfe sowie die Dialoggrößen.

| Was | Wo | Wirkung |
|---|---|---|
| **Fußleiste** hinter der unteren Knopfzeile | `dialogs\mod.rs` (`dialog_frame_ex`, `footer_top`, `painted_rects`, `footer_shape`, `set_footer`, `dialog_footer_top`) | getönte Fläche und geätzte Linie, 12 px über den Knöpfen. Der Rahmen erkennt die untere Knopfzeile selbst: `push_button` merkt sich die Knöpfe. Die Leiste rückt nie über Gezeichnetes. Ragt etwas in die Knopfzeile hinein (Knöpfe rechts untereinander wie bei Filtern, Liste bis unten), gibt es keine Leiste. Gezeichnet wird sie in einen vor dem Inhalt reservierten Platz, liegt also unter allem (kein „verdeckt“ im Prüfer). Im RAND-2b-Bildlaufmodus ist der feste Knopfstreifen die Fußleiste; seine einfache Linie ersetzt die geätzte. |
| `ui_kit::button_bar` | `ui_kit.rs` | zeichnet die Linie nicht mehr selbst, sondern meldet seine Leiste über `dialogs::set_footer` an den Rahmen. Betrifft Rückfrage, Warnung, Über, Bitmap nachzeichnen. |
| Ungespeichert | `unsaved.rs` | eigene Linie entfernt, jetzt automatische Fußleiste an derselben Stelle |
| **Titelleiste** | `mod.rs` | ruhiger Verlauf. Dunkel jetzt deutlich dunkler als die Fläche (DESIGN §13: „kaum Kontrast“). OLED schwarz mit Akzentlinie. |
| **Schatten** | `mod.rs` | weicher Schlagschatten. OLED hat stattdessen einen Akzentschein. Lage und Rechteck des Dialogs bleiben unverändert. |
| **Standardknopf** | `push_button` | Fläche leicht in der Fokusfarbe getönt, beim Überfahren stärker; der Fokusrahmen bleibt |
| **Gruppenrahmen** | `group_box` | geätzt (dunkle Linie + Lichtkante) wie ein Windows-Gruppenfeld. OLED fein, eine Linie. |
| Abschnittsüberschrift im Formular (`FieldKind::Caption`) | `form.rs` | Farbe `section_label`, geätzte Linie (DESIGN §4) |
| Design-Tokens | `theme.rs`, neuer `impl Theme`-Block am Dateiende, nur ergänzt | `dialog_title_gradient`, `dialog_title_line`, `dialog_footer_bg`, `dialog_footer_lines`, `dialog_shadow`, `default_button_fill`, `group_box_lines`, alle aus der Palette abgeleitet |

Die Meldung mit Info-Symbol hat NZ-3 schon umgesetzt (`3a3342b`). Sie bekommt jetzt zusätzlich automatisch die Fußleiste.

## Tests (alle grün)

- neu `crates\fp-ui\tests\dialogs_des1.rs`:
  - `footer_top_rules` (Regeln der Fußleiste)
  - `footer_under_bottom_button_rows`: 9 Dialoge; Gaußscher Weichzeichner mit Knöpfen rechts ohne Leiste
  - `des1_images` (ignoriert, erzeugt die Bilder)
- gezielt: `dialogs_des1`, `clipping_audit` (schnell), `dialog_scroll_rand2b`, `dialogs_dlg*`, `dialogs_move`, `dialogs_open`, `export_e*`, `image_preview_p*`, `dialogs_text_layout`, `warning` → 135 ok
- `cargo check --workspace --all-targets` ok
- `cargo test -p fp-neutral` ok, auch die Texterkennung in Bildern
- keine neuen Texte, daher fp-i18n und Scanner nicht nötig
- keine Test-Erwartung angepasst

## Bilder

`docs\screenshots\des1\before_*.png` und `after_*.png`, je Bild Normal | Dunkel | OLED. Erzeugt mit `FP_DES1_STAGE=before|after cargo test -p fp-ui --test all dialogs_des1::des1_images -- --ignored`; „before“ auf main `42dabe7` ohne DES-1.

Dialoge (10): `message`, `confirm_history_clear`, `unsaved`, `save_workspace`, `image_size`, `canvas_size`, `gaussian_blur`, `new_document`, `warning_no_target`, `preferences`.

## Offene Punkte

- **Voreinstellungen:** keine Fußleiste. Die Kategorienliste reicht bis 3 px an die Knopfzeile, deshalb lässt die Regel sie bewusst weg. Wer sie will, kürzt die Liste um ca. 10 px; das wäre eine Anordnungsänderung, also nicht von DES-1.
- **DESIGN §13 Prio 5–11** (Tabellen in KI-Modelle, Gespeicherte Befehle, Tastenkürzel, Pop-up-Editor, Export-Leerraum) nicht angefasst. Dort arbeiteten gleichzeitig E3, NZ-3 und FORM-1; diese Prios bekommen die Fußleiste und den Rahmen trotzdem automatisch.
- Die Knopfbreiten-Vereinheitlichung (DESIGN §6) ist nicht zentral umgesetzt: Die Breiten sind Teil der getesteten Anordnung.

## PARITAET / NUTZERBEFUNDE

- N11: Dialoge „umgesetzt, Nachprüfung ausstehend“. Nachher-Bilder liegen jetzt in `docs\screenshots\des1\`.
- PARITAET: keine Zeile ändert sich, das Verhalten ist gleich.

## NACHPRUEFUNG (Vorschlag, Zeilen)

- Dialoge im echten Fenster (Brücke), 3 Designs. Ablauf: Bildgröße, Leinwandgröße, Neues Dokument, Rückfrage „Verlauf löschen“, Ungespeichert und Gaußscher Weichzeichner öffnen. Erwartet:
  - Fußleiste mit geätzter Linie hinter OK/Abbrechen, nichts verdeckt
  - beim Weichzeichner (Knöpfe rechts oben) keine Leiste
  - Dunkel: Titelleiste klar dunkler als die Fläche
  - OK getönt, unter der Maus stärker
- Dialog höher als das Fenster (1024 × 768 bei 150 %, z. B. Export). Erwartet: der feste Knopfstreifen von RAND-2b erscheint als Fußleiste (Fläche + geätzte Linie), die Knöpfe sind darauf sichtbar und bedienbar.
- Schatten bzw. Schein beim Ziehen des Dialogs. Erwartet: der Schatten wandert mit, keine Reste.

## Neue Befunde

keine

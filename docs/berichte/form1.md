# FORM-1: Formular-Grundlagen wie im Original (13.10.2026)

Arbeitsplatz: Slot s1. Grundlage: PARITAET_B §2 (D14, D26–D28, D35–D39, D41, D42, D46), re\11 §3.5.5 und die Befunde von DLG-4, DLG-7, DLG-8, DLG-9 und NZ-2. Das Original wurde nicht bedient.
Commit: `git log --grep "FORM-1"` (ein Commit).

## Umgesetzt

1. **Tab-Folge der Knöpfe je Vorlage.** egui führt den Fokus in der Reihenfolge, in der die Bedienelemente angelegt werden. Deshalb wird jetzt in der Tab-Folge des Originals gezeichnet.
   - Neu: `form::ok_cancel(ui, d, rects)`. Damit zeichnet eine eigene Anordnung OK/Abbrechen an der richtigen Stelle; `show_form` zeichnet die Knöpfe dann nicht noch einmal (`FormDialog::pressed`).
   - **D27** In Symbol konvertieren: Name › Typ › OK › Abbrechen › 9-Slice › Bibliothek.
   - **D28** Animieren: OK › Abbrechen › Felder (Startfokus OK).
   - **D14** Neues Symbol: wie D27.
   - Alle übrigen Formulare bleiben bei Felder › OK › Abbrechen; das ist laut Messung der Normalfall (D29, D30, D01 …).
   - Enter in einem Feld, das erst nach den Knöpfen gezeichnet wird (D28), bestätigt weiter. `dialogs::enter_confirms` prüft den Fokusverlust jetzt erst im nächsten Bild; ein mehrzeiliges Feld bestätigt weiterhin nicht.
2. **Popup-Schieber ▾ ist kein Tab-Stopp.** In `fields::slider_arrow_vertical` und `slider_arrow_scaled` steht jetzt `Sense::CLICK` (nicht fokussierbar). Das gilt für Dialoge und Inspektor; per Maus geht alles wie bisher.
3. **`FormDialog::load_prefs` bei Auswahllisten mit Text-Werten.** Neuer Vergleich `pref_matches`: Zahl mit Zahl, Text mit Text, Wahrheitswert mit Wahrheitswert. Vorher war bei Text `None == None` wahr, also wurde immer Eintrag 0 gewählt.
4. **Klemmen beim OK (19105–19108).** Neue Feldoption `Field::clamp_on_ok()`:
   - Bis zum OK bleibt der getippte Wert stehen.
   - OK klemmt auf 1…100 und korrigiert das Feld. Der Dialog bleibt offen, der Fokus geht mit markiertem Inhalt ins Feld, es entsteht kein Verlaufsschritt.
   - Das zweite OK bzw. Enter wendet den Wert an.
   - Weiche Kante (5976) klemmt wie bisher sofort, weil die Vorlage dort keinen Bereich nennt.
5. **D14 „In allgemeiner Bibliothek speichern“ gesperrt.** Neue Feldoption `Field::disabled()`: grau, kein Tab-Stopp, Klick ohne Wirkung; wirkt in der Standard-, Filter- und Symbolanordnung. D14 nutzt jetzt die Anordnung der Vorlage 19104 (`symbol_anim::CONVERT_LAYOUT`, 9-Slice-Schlüssel `nineSlice`/`_nineslice`).
6. **Klick neben einen modalen Dialog schließt ihn nicht mehr.** In `dialog_frame_ex` schließen nur noch Esc (oberster Dialog, kein Popup offen), das Kreuz, Abbrechen und `ui.close()`. Der Hintergrund-Klick aus `Modal::should_close` zählt nicht mehr.
7. **Tab-Folge der Filteranordnung.** Neu sind `SideOrder` und `FormDialog::without_slider_tab_stops`, die Filteranordnung wird jetzt von einer Funktion `draw_side` in Tab-Folge gezeichnet:
   - D41: nur Felder › OK › Abbrechen › Kolorieren › Vorschau; die Schieber sind keine Stopps.
   - D42: Schieber › Feld › Schieber › Feld › Vorschau › OK › Abbrechen, Startfokus auf dem Schieber.
   - D46: Felder › Vorschau › Schieber › OK › Abbrechen.
8. **D26 Startfokus auf der Liste.** Neu ist `dialogs::autofocus_widget`. `fields::combo` wählt mit Pfeiltasten den vorigen bzw. nächsten Eintrag, ohne Umlauf und ohne Trenner, und zeigt im Dialog einen Fokusrahmen.

Dateien: `dialogs\form.rs`, `dialogs\mod.rs`, `dialogs\symbol_anim.rs`, `dialogs\catalog.rs`, `dialogs\menu_dialogs.rs` (D14), `dialogs\doc_dialogs.rs` (D26), `inspector\fields.rs`. Neue Texte gibt es nicht.

## Tests

- Neu: `crates\fp-ui\tests\form_form1.rs` mit 12 Tests, alle grün: Tab-Folgen D27, D28, D14, D41, D42 und D46, ▾ kein Tab-Stopp, `load_prefs` mit Text und Zahl samt Rundlauf, Klemmen beim OK per Klick und per Enter, Klick neben den Dialog, Startfokus und Pfeiltasten bei D26.
- Angepasst wegen gewollter Änderungen:
  - `dialogs_dlg8::limits_clamp`: 19105–19108 klemmen erst beim OK.
  - `dialogs_dlg4::d14_…`: zusätzliches gesperrtes Feld `_common`.
- Grün:
  - dialogs: dlg1, dlg1b, dlg2, dlg4–dlg10, move, open, enter, sliders, ux, warnung, effects
  - clipping_audit, nz1, image_preview_p2, inspector_paritaet, export_e3
  - fp-ui lib `form::`
  - `cargo check --workspace --all-targets` ohne Warnungen
  - `cargo test -p fp-neutral`

## Erledigte Befunde (BEFUNDE, Wortlaut-Anfang)

- (von NZ-2, für DLG) **Klick neben einen modalen Dialog schließt ihn** – erledigt (FORM-1)
- (von DLG-7, PARITAET_B D29–D32 und alle Felder mit ▾) **Popup-Schieber ▾ ist ein eigener Tab-Stopp** – erledigt (FORM-1)
- (von DLG-7, klein) **`FormDialog::load_prefs` liest Auswahllisten mit Text-Werten falsch** – erledigt (FORM-1)
- (von DLG-8, klein, `form.rs`) **Klemmen beim OK (Vorlagen 19105–19108)** – erledigt (FORM-1)
- (von DLG-9, Tab-Folge, allgemein zu DLG-1) **Formulare führen die Knöpfe nach den Feldern** – erledigt für D27/D28 (FORM-1); allgemein bleibt Felder › Knöpfe der gemessene Normalfall.
- (von DLG-4, PARITAET_B D14, `form.rs`) **Neues Symbol: „In allgemeiner Bibliothek speichern“ fehlt** – erledigt (FORM-1). Die Frage zu „Neue Schaltfläche“ hat NZ-4 schon erledigt.

## PARITAET_B: auf „umgesetzt, Nachprüfung ausstehend“

- D14: gesperrtes Kästchen, Anordnung 19104, Tab Name › Grafik › OK › Abbrechen › 9-Slice
- D26: Startfokus Liste
- D27: Tab-Folge der Knöpfe
- D28: Tab OK › Abbrechen › Felder
- D35–D39 (19105–19108): Klemmen beim OK
- D41/D42/D46: Tab-Folge (D41 ohne Schieber-Stopps)
- D29–D32 und alle Felder mit ▾: ▾ kein Tab-Stopp

## NACHPRUEFUNG (echtes Fenster, Brücke)

| Was | Ablauf | Erwartet |
|---|---|---|
| D27 | R1 wählen, In Symbol konvertieren…, sechsmal Tab | Name › Grafik › OK › Abbrechen › 9-Slice › Bibliothek |
| D28 | R1 wählen, Auswahl animieren…, dreimal Tab | OK (Start) › Abbrechen › Status › Verschieben (▾ nie fokussiert); Enter im Feld = OK |
| D14 | Bearbeiten ▸ Objekt einfügen ▸ Neues Symbol… | „In allgemeiner Bibliothek speichern“ grau und nicht anklickbar; Tab Name › Grafik › OK › Abbrechen › 9-Slice |
| D35–D39 | Pixelauswahl, Auswahlrahmen erweitern…, „500“, Enter | Feld zeigt 100 markiert, Dialog offen, kein Verlaufsschritt; zweites Enter wendet 100 an. Ebenso „0“ → 1 |
| Modal | beliebigen Formular-Dialog öffnen, daneben auf Leinwand oder Bedienfeld klicken | Dialog bleibt offen; Esc schließt |
| D41/D42/D46 | B1 wählen, Farbton/Sättigung…, Helligkeit/Kontrast…, Unscharf maskieren…, jeweils Tab durchgehen | Folgen wie oben; D42 beginnt auf dem Schieber, Pfeiltasten verschieben ihn |
| D26 | R1 wählen, Numerisch transformieren… | Fokus (Rahmen) auf der Liste; ↓ wählt „Größe ändern“; Tab geht zum ersten Feld |

## Offen / neue Befunde

- (klein, `dialogs\effects.rs`) Die Live-Effekt-Dialoge zum Bearbeiten (Farbton/Sättigung, Helligkeit/Kontrast, Unscharf maskieren als Live-Filter) haben die neue Tab-Folge noch nicht. Erwartet ist dieselbe wie im Menüdialog: `.with_side_order(..)` bzw. `.without_slider_tab_stops()` in `effect_form` je Art, wie in `catalog.rs`.
- (?) Tab-Folge von D48 (Gauß), D45 und D47/D49/D50 ist nicht gemessen; sie bleibt bei Felder › OK › Abbrechen › Vorschau.
- (?) Ob im Original nach dem Klemmen beim OK der Fokus ins Feld geht, ist nicht gemessen; Firepixel setzt ihn dorthin, mit markiertem Inhalt.
- `panel.library.newSymbol` (Bibliothek-Bedienfeld) nutzt weiter die einfache Anordnung ohne 9-Slice und Bibliothek; nicht gemessen.

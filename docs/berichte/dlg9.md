# DLG-9: Paket P-DLG9 „Symbol, Animation, Verhalten“ (12.10.2026)

Slot s6. Daten: PARITAET_B §2 D27/D28/D33/D34, BEFUNDE (von PM-6), Bilder `pm6_D27/D28/D33/D34*`.
Original nicht benutzt.

## Umgesetzt

- **D27 In Symbol konvertieren** (19104): eigene Anordnung nach der Vorlage (neue Datei
  `crates\fp-ui\src\dialogs\symbol_anim.rs`, `CONVERT_LAYOUT`): senkrechte Trennlinie links,
  „Name:“/„Typ:“/„Optionen:“ rechtsbündig vor der Feldspalte, Typ Grafik/Animation/Schaltfläche
  **untereinander**, „Optionen:“ mit 9-Slice und „In allgemeiner Bibliothek speichern“
  (en „Options:“, „Save in Common Library“), OK/Abbrechen **rechts** untereinander, Größe
  326 × 102 DLU (≈ 489 × 166 px Client). Name hat den Startfokus, Inhalt markiert.
- **D28 Animieren** (19146, `ANIMATE_LAYOUT`): „Status“, Trennlinie, „Verschieben“, „Richtung“,
  „Skalieren auf“, „Deckkraft: … bis …“ in **einer** Zeile, „Drehen“ mit „Nach rechts“/„Nach links“
  in derselben Zeile (en „CW“/„CCW“), zweite Trennlinie, OK/Abbrechen unten rechts, ohne Einheiten
  (wie im Original), Popup-Schieber ▾ an jedem Feld bleibt, **Startfokus OK**, Größe 215 × 152 DLU.
- **D28 Engine:** Grenzen einer Animationsinstanz enthalten jetzt den **Bewegungsweg** (alle
  Status, Skalierung/Drehung um die Symbolmitte wie beim Zeichnen, dann Versatz) –
  `fp_engine::docops::element_bounds` (wirkt auf `pixelRect`, Inspektor, Auswahlrahmen).
  R1 20,20,120,100 mit 3 Status / 72 px → 20,20,192,100 wie im Original.
- **Verlaufsname** von „Auswahl animieren“ wie im Original **„In Symbol konvertieren“** (en: Name
  von `modify.symbol.convertToSymbol`). „In Symbol konvertieren“ selbst hieß schon so.
- **D33 Instanzen tweenen:** geprüft, Wortlaut und „Nicht mehr anzeigen“ stimmen; keine
  Beschriftung offen (der Formular-Dialog mit zwei Instanzen ist nicht Teil der Messung).
- **D34 Popupmenü hinzufügen:** Nach „Abbrechen“ oder Esc der Rückfrage folgt die **zweite
  Meldung** „Wählen Sie ein Segment oder einen Hotspot aus, bevor Sie den Befehl „Verhalten
  einfügen“ verwenden.“ (en „Select a slice or a hotspot before using the "Add Behavior"
  command.“) mit Warnsymbol und „Nicht mehr anzeigen“ (`DisableNoticeInsertBehavior`, von
  „Warnmeldungen zurücksetzen“ erfasst). Engine: `Session::warning_after_cancel`
  (`crates\fp-engine\src\warnings.rs`), Oberfläche: `crates\fp-ui\src\dialogs\warning.rs`.

Texte: `dialoge.ftl` (de/en) `dlg-lbl-movement`, `-scale-to`, `-opacity-from`, `-opacity-to`,
`-rotation`, `dlg-cw`, `dlg-ccw`, `dlg-lbl-save-common-library` geändert, neu `dlg-lbl-options`;
`meldungen.ftl` (de/en) neu `msg-warn-insert-behavior`.

Tests: `crates\fp-ui\tests\dialogs_dlg9.rs` (9 Tests: Rechteck mit Weg + Rückgängig + Richtung
nach oben + ein Status, Verlaufsname de/en, Anordnung/Beschriftungen/Fokus/Größe D27 und D28 de/en,
Enter-Wirkung, D33-Wortlaut, D34 zweite Meldung nach Esc und Abbrechen, abschaltbar, zurücksetzbar, en).

Angepasst an die zweite Meldung (gewolltes Verhalten): `dialogs_warnung::popupmenue_rueckfrage_hotspot`
(nach Abbrechen erst die Folgemeldung, Enter schließt) und `dialogs_move` (nach Esc beim Popupmenü
ein zweites Esc). `menu_fixes_fv13::reset_warning_dialogs_script_reenables_notices` schlug im Slot
fehl („Eintrag Befehle ▸ Warndialoge zurücksetzen nicht gefunden“), unabhängig von DLG-9 (Menü des
Befehlsordners, nicht berührt).

Commits: siehe `git log --grep "DLG-9"`.

## Erledigte Befunde (Wortlaut-Anfang)

- (von PM-6, PARITAET_B D34, klein) **Popupmenü hinzufügen:** Nach Abbrechen/Esc der Rückfrage … – erledigt (DLG-9)
- (von PM-6, PARITAET_B D28, klein) **Animieren: Rechteck des Animationssymbols.** … – erledigt (DLG-9, inkl. Schritt „In Symbol konvertieren“)
- (von PM-6, PARITAET_B D27–D31/…, klein) **Beschriftungen und Anordnung wie im Original** – Teil D27 und D28 erledigt (DLG-9); D29–D31, D38, D39, D47, D50 bleiben offen.

## PARITAET_B → „umgesetzt, Nachprüfung ausstehend“

- D27 In Symbol konvertieren
- D28 Auswahl animieren
- D34 Popupmenü hinzufügen
- D33 bleibt ✅

## Nachprüfung

1. R1 wählen, Modifizieren ▸ Symbol ▸ In Symbol konvertieren…: Anordnung wie `pm6_D27`
   (Typ untereinander, „Optionen:“, Knöpfe rechts), Enter → Instanz, Strg+Z.
2. R1 wählen, Modifizieren ▸ Animation ▸ Auswahl animieren…: Anordnung wie `pm6_D28`, Fokus auf OK,
   Enter → Instanz, Inspektor/`pixelRect` 20,20,192,100 (Richtung 0, 72 px), Verlauf „In Symbol
   konvertieren“; Richtung 30 testen (Rechteck geht nach oben, siehe Hinweis).
3. Ohne Hotspot/Segment Modifizieren ▸ Popupmenü ▸ Popupmenü hinzufügen…, Esc → zweite Meldung;
   ebenso mit „Abbrechen“; „Nicht mehr anzeigen“ ankreuzen → danach keine zweite Meldung.
4. Englisch: dieselben Dialoge, nichts abgeschnitten.

## Hinweise / neue Befunde

- PM-6 nennt für D28 „Status 3, Richtung 30“, das gemessene Rechteck 20,20,192,100 entspricht aber
  genau 72 px nach **rechts** (Richtung 0; bei 30° läge die Oberkante bei −16). Vermutlich hatte das
  Tippen in „Richtung“ im Original nicht gegriffen. Firepixel rechnet den Weg mit der eingestellten
  Richtung; bei der Nachprüfung im Original mit Richtung ≠ 0 nachmessen.
- Tab-Folge D27 im Original Name › Typ › OK › Abbrechen › 9-Slice › Bibliothek; Firepixel: Name ›
  Typ › 9-Slice › Bibliothek › OK › Abbrechen (Formular zeichnet die Knöpfe nach den Feldern).
  D28 im Original OK › Abbrechen › Felder; Firepixel Felder › OK (Startfokus OK stimmt). Allgemeine
  Tab-Folge der Formulare (DLG-1).
- Mit dem neuen Rechteck verhält sich „Größe ändern“ im Inspektor bei Animationsinstanzen nach den
  Grenzen mit Weg (der Weg selbst skaliert nicht mit); im Original nicht gemessen.

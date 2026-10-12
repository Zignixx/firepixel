# DLG-7: Paket P-DLG7 „Pfad und Transformieren“ (12.10.2026)

Grundlage: PARITAET_B §2 D26, D29–D32, Bilder `screenshots/paritaet/pm6_D26…D32*`, Befunde „von PM-6“.
Slot s4. Commit: siehe `git log --grep "DLG-7:"` (eine Änderung, auf main übernommen).

## Umgesetzt

**D26 Numerisch transformieren** (`crates/fp-ui/src/dialogs/doc_dialogs.rs`)
- ↔/↕-Symbole statt „Breite:/Höhe:“ (Name und Tooltip bleiben „Breite:“/„Höhe:“), Felder in einem Rahmen.
- Rechts die Objektgröße live: „B: 100 / H: 80“ (en „W:/H:“), folgt Prozent- bzw. Pixelfeldern; beim Drehen ausgeblendet.
- Kettensymbol bei „Proportionen beibehalten“.
- Das zweite Feld rechnet **beim Tippen** mit (`live_text` wie Bildgröße, DLG-1b): B 50 → H 50 sofort; „Größe ändern“ B 200 → H 160 sofort.
- Pixelmaße bei „Größe ändern“ werden nach OK gemerkt (`NumericTransformWidth`/`NumericTransformHeight`), Abbrechen/Esc merkt nichts.

**D29/D30/D31** (neue Datei `crates/fp-ui/src/dialogs/path_dialogs.rs`, eigene Anordnung)
- D29 „Menge:“ (statt „Betrag:“).
- D30 „Winkelgrenze:“ mit zwei Nachkommastellen und Punkt („10.00“), „Endkappen:“ (en „End caps:“).
- D31 Richtung Innen/Außen **untereinander**, „Winkelgrenze:“ 10.00.
- Beschriftungen rechtsbündig vor den Feldern, Block mittig, Leerzeilen vor Ecken/Endkappen bzw. Breite/Ecken wie im Original, OK/Abbrechen unten mittig.
- Symbolknöpfe (Ecken, Endkappen) sind **ein** Tab-Stopp (nur der gewählte fokussierbar), Pfeiltasten wählen innerhalb der Gruppe.
- Werte, Voreinstellungen (`SimplifyAmount`, `Inset*`, `Expand*`) und Wirkung unverändert.

**D32 Pfad in Auswahlrahmen konvertieren** (neu)
- Dialog an `modify.convertPathToMarquee`: „Rand:“ Hart / **Anti-Alias** (Vorgabe) / Weiche Kante, Breite mit ▾ (gesperrt außer bei „Weiche Kante“), OK/Abbrechen rechts untereinander, Startfokus OK. Werte nach OK gemerkt (`PathToMarqueeEdge`, `PathToMarqueeFeather`).
- Befehl (`crates/fp-engine/src/commands/modify.rs`): neue Parameter `edge` (`hard`/`antiAlias`/`feather`, Vorgabe `antiAlias` = bisheriges Verhalten) und `feather` (0…1000, nur bei `feather`).
- JSF `dom.convertPathToMarquee(mode, featherAmount)` reicht die Werte durch (0 hart, 1 Anti-Alias, 2 weiche Kante; ohne Argumente wie bisher Anti-Alias).

Texte: `dialoge.ftl` de/en – geändert `dlg-lbl-simplify-amount` (de „Menge:“), `dlg-lbl-miter-limit` (de „Winkelgrenze:“), `dlg-lbl-caps` (de „Endkappen:“, en „End caps:“); neu `dlg-numeric-size-w`, `dlg-numeric-size-h`, `dlg-title-path-to-marquee`. D32 nutzt `pi-edge*` aus `inspektor.ftl`.

Kleine Edits in gemeinsamen Dateien: `catalog.rs` (vier Einträge zeigen auf `path_dialogs`, `join_field` dorthin verschoben), `form.rs` (`paint_pict` → `pub(crate)`), `dialogs/mod.rs` (`pub mod path_dialogs;`), `tests/dialogs_dlg1.rs` (`typed_value_survives_mouse_click_into_another_field` vergleicht den Wert statt des Texts, weil die Winkelgrenze jetzt „7.00“ zeigt).

## Tests

`cargo test -p fp-ui --test all dialogs_dlg7::` (6 Tests, grün): Objektgröße + Mitrechnen beim Tippen + Wirkung + Strg+Z (D26 Skalieren), Größe ändern mit Mitrechnen, Wirkung, gemerkten Pixelmaßen und ohne Kette, Beschriftungen D29–D31 inkl. „10.00“/untereinander/rechtsbündig und Englisch, Tab-Folge D30 und Wirkung, D32 Dialog öffnet / Abbrechen ohne Wirkung / weiche Kante 5 + OK / Strg+Z / gemerkt, D32-Befehl hart/Anti-Alias/weich.
Außerdem grün: `dialogs_dlg1::`, `dialogs_dlg5::`, `dialogs_sliders::`, `dialogs_enter::`, `dialogs_move::`, `dialogs_open::`.

## Befunde (Wortlaut-Anfang)

- „(von PM-6, PARITAET_B D26) **Numerisch transformieren:** rechts live …“ – erledigt bis auf den Startfokus auf der Liste (Startfokus bleibt auf dem ersten Feld).
- „(von PM-6, PARITAET_B D27–D31/…) **Beschriftungen und Anordnung wie im Original**“ – Teil D29, D30, D31 erledigt (DLG-7).
- PARITAET_B Abschnitt „D32 Pfad in Auswahlrahmen konvertieren…: Im Original ein Dialog …“ – erledigt.

## PARITAET_B: auf „umgesetzt, Nachprüfung ausstehend“

D26, D29, D30, D31, D32.

## Nachprüfung (echtes Fenster, Brücke)

- D26: R1 (20,20,120,100) wählen, Numerisch transformieren → ↔/↕, Kette, „B: 100 / H: 80“; B „50“ tippen → H zeigt sofort 50, „B: 50 / H: 40“; Enter → 45,40,95,80; „Größe ändern“ 200 → 160 sofort; OK, erneut öffnen → 200/160.
- D29/D30/D31: Beschriftungen und Anordnung gegen `pm6_D29…D31`; Tab D30: Breite › Ecken (1 Stopp) › Winkelgrenze › Endkappen (1 Stopp) › OK › Abbrechen.
- D32: Pfad wählen, Modifizieren ▸ Pfad in Auswahlrahmen konvertieren… → Dialog wie `pm6_D32` (Anti-Alias, Breite grau, Fokus OK); Enter → Auswahlrahmen, Pfad weg, Strg+Z; Weiche Kante 5 → weicher Rand.

## Neue Befunde

- (für IBD/Dialoge) Der Popup-Schieber ▾ neben Zahlenfeldern (`inspector/fields.rs::slider_arrow_vertical`, `Sense::click()`) ist ein eigener Tab-Stopp; im Original nicht (PM-6 D30: Breite › Ecken). Betrifft alle Felder mit ▾ (D29–D32, Filter …). Erwartet: kein Tab-Stopp (`Sense::CLICK`), Bedienung per Maus.
- (für IBD/Dialoge, klein) `FormDialog::load_prefs` liest eine Auswahlliste mit Text-Werten (`choice_values` als Strings) ohne `pref_codes` falsch: `v.as_i64() == pv.as_int()` ist bei Text `None == None` → immer Eintrag 0. D32 umgeht es mit `pref_codes`. Erwartet: Vergleich nur, wenn beide Zahlen sind.
- D26 Startfokus auf der Liste Skalieren/Größe ändern/Drehen (Original) ist offen; `fields::combo` nimmt keinen Startfokus.

# AU-3: JSF-Textrichtung, Texturnamen in JSF, „Andere…“-Pfad, F7 offen

Agent AU-3, Slot s7, 13.10.2026. Paket AU-3, verkleinert nach Nutzerentscheidung. Das Original wurde nicht benutzt.

## Umgesetzt

Commit: siehe `git log --grep "AU-3:"` (ein Commit auf main).

| Punkt | vorher | jetzt |
|---|---|---|
| JSF-Textrichtung (PM-4) | `ORIi` 1 → „vertical left to right“, 3 → „vertical right to left“ (Getter `orientation`, Setter `orientation`, `setTextOrientation`). `setTextFlow("vertical right to left")` ergab links→rechts, weil der Name auch „left“ enthält | wie `re\09-text.md` T5: **1 = „vertical right to left“, 3 = „vertical left to right“**, 2 = „horizontal right to left“. Eine Zuordnung für alle vier Stellen (`imp\mod.rs`: `orientation_jsf_name`, `orientation_code_from_jsf`) |
| Texturnamen in JSF (FV-16) | `setFillNColorNTexture`/`setBrushNColorNTexture` übergingen den dritten Parameter; `setFillTexture`/`setBrushTexture` taten nichts | Textur nach Namen über `pi.fill {texture}` bzw. `pi.stroke {texture}`; die Engine lädt die Maske mit `fp_engine::resources::load_texture` aus den Texturordnern (ohne Datei bleibt wie im Inspektor nur der Name). `""` entfernt die Textur. Ohne Auswahl ist die Textur eine **Vorgabe für neue Formen** (`addNewRectangle`/`addNewOval`/`addNewLine`/`addNewPolygon`/`addNewStar`; neue Felder `DefaultAttrs::fill_texture/stroke_texture`). Bei `fill = null` bzw. `brush = null` wird keine Textur gesetzt |
| „Andere…“-Pfad (FV-16) | Werkzeug-Inspektor merkte nur den Dateinamen ohne Endung; eine Datei außerhalb der Texturordner wirkte an neuen Objekten nicht (Textur ohne Maske) | Als Werkzeugvorgabe wird der **volle Pfad** gemerkt; das Feld zeigt weiter den Dateinamen ohne Endung (`obj_quads::texture_display_name`). `pi.fill`/`pi.stroke` laden bei einem absoluten Pfad die Datei; Name der Textur = Dateiname ohne Endung (`commands\panel.rs::texture`) |

Hinweis: `setFillTexture` und `setBrushTexture` stehen nicht in der Methodenliste des Originals (`bindings_data.rs`). Aus JS sind sie daher nicht erreichbar; ihre Arme in `imp\doc.rs` setzen jetzt trotzdem die Textur, falls sie jemand intern aufruft.

Geänderte Dateien:
- `crates\fp-script\src\imp\mod.rs`, `imp\elem.rs`, `imp\doc.rs`, `imp\doc2.rs`, `src\state.rs`
- `crates\fp-engine\src\commands\panel.rs`: nur `fn texture` (absoluter Pfad). IBD-Bereich, für Punkt 4 im Auftrag
- `crates\fp-ui\src\inspector\obj_quads.rs`: `pick_other_texture` (Werkzeugvorgabe = voller Pfad), `texture_combo` zeigt den Namen, neu `texture_display_name`
- `crates\fp-ui\tests\inspector_fv15.rs`: Erwartung in `w21_other_texture_from_file` angepasst. Die Vorgabe ist jetzt der Pfad, angezeigt wird weiter „Meine Textur“
- neu: `crates\fp-script\tests\au3.rs` (6 Tests), `crates\fp-ui\tests\au3.rs` (2 Tests, echte Klicks und Ziehen auf der Leinwand)

Geprüft: `cargo test -p fp-script --test au3` (6/6), `cargo test -p fp-ui --test all -- au3:: inspector_fv15::`, `cargo test -p fp-script`, `cargo check --workspace --all-targets`. Keine neuen UI-Texte, deshalb kein fp-i18n- und fp-neutral-Lauf.

## Erledigte Befunde (BEFUNDE.md)

- „(von PM-4, PARITAET_A W17) **JSF-Textrichtung 1 und 3 vertauscht.**“ – erledigt (AU-3)
- „(von FV-16, klein) JSF `setFillNColorNTexture`/`setFillTexture`/`setBrushTexture` übergehen den Texturnamen“ – erledigt (AU-3)
- „(von FV-16, klein) „Andere…“ in der Texturliste des **Werkzeug**-Inspektors merkt nur den Dateinamen“ – erledigt (AU-3), Variante „Pfad merken, Anzeige der Name“. Es wird nichts nach `%APPDATA%` kopiert
- „(von FV-9, PARITAET_A F7, Nebenbeobachtung PM-2) **Feldtext beim Klick auf ein anderes Objekt.**“ – **bleibt offen**, siehe unten

## F7: Feldtext beim Klick auf ein anderes Objekt – nicht geändert

`re\11-bedienfelder-dialoge.md` und `re\10-werkzeuge-befehle.md` sagen nichts dazu, wann ein Inspektorfeld übernimmt und an welches Objekt. Nur `re\19-kurven-tonwert.md` nennt für Dialogfelder „beim Verlassen“ (EN_KILLFOCUS, A). Ob das Original den Wert vor oder nach dem Auswahlwechsel übernimmt, lässt sich daraus nicht ableiten. Also kein Nachbau auf Verdacht: Firepixel setzt weiter **30 an R1**.

**Messablauf für den Nachtlauf (CT-4)**, im Original nach einem Neustart, mit eigenem Testdokument:
1. Neues Dokument 500 × 300, weiß. Zwei Rechtecke ohne Strich: R1 `[50,50,150,120]` rot und R2 `[250,50,350,120]` blau, beide Deckkraft 100.
2. **Lauf A (Deckkraft, Klick):** R1 mit dem Zeiger anklicken. Ins Deckkraft-Feld klicken, Feldtext markieren (Strg+A im Feld), „30“ tippen, **ohne** Eingabe oder Tab. Dann einmal mit dem Zeiger **auf R2** klicken. Danach R1 und R2 nacheinander anklicken und jeweils die Deckkraft ablesen.
3. **Lauf B (Umschalt+Klick):** wie A, aber R2 mit Umschalt+Klick dazunehmen. Ablesen: Deckkraft von R1 und R2, und was das Feld bei der Doppelauswahl zeigt.
4. **Lauf C (Breite statt Deckkraft):** wie A, im Feld B „60“ tippen, dann Klick auf R2. Ablesen: Breite von R1 und R2.
5. **Lauf D (Drücken und Halten):** wie A, aber auf R2 die Maustaste drücken und **gedrückt halten**. Vor dem Loslassen ein Bildschirmfoto: Ist R2 schon gewählt? Zeigt das Feld noch „30“ oder schon den Wert von R2? Dann loslassen und wie in A ablesen.
6. **Lauf E (Klick ins Leere):** wie A, aber auf die leere Leinwand klicken. Erwartet nach PM-2 F7: 30 an R1. Dieser Lauf ist die Gegenprobe.
7. Jeden Lauf 3× wiederholen, jeweils mit Bildschirmfotos nach Schritt 2. Zusätzlich die Verlaufsliste: Wie viele Schritte, und an welchem Objekt?

Auswertung für den Nachbau:
- Steht 30 an R2 und wählt das Original beim Drücken (Lauf D: R2 schon gewählt, Feld noch „30“)? Dann gehört die Übernahme zwischen Drücken und Loslassen. Haken in `crates\fp-ui\src\canvas\mod.rs` nach `PointerKind::Down`: den offenen Feldtext erst nach dem Auswahlwechsel an die neue Auswahl geben. Dazu `inspector\fields.rs` (`buffered_edit`, DLG-1) um ein „an neue Auswahl übernehmen“ erweitern.
- Steht 30 an R1, ist Firepixel schon richtig. Dann den Befund schließen.

## PARITAET-Zeilen „umgesetzt, Nachprüfung ausstehend“

- PARITAET_A W17 (JSF-Textrichtung)
- PARITAET_A W21, Rest „Andere…“ als Werkzeugvorgabe mit Datei außerhalb der Texturordner

## Nachprüfung (NACHPRUEFUNG.md)

| Was | Ablauf | Erwartet |
|---|---|---|
| JSF-Textrichtung | Text anlegen, im Text-Inspektor „Vertikal von rechts nach links“ wählen, dann `fw.selection[0].orientation` per Skript (Befehle ▸ Skript ausführen bzw. MCP `script.run`) | „vertical right to left“; umgekehrt setzt `setTextOrientation("vertical left to right")` im Inspektor „Vertikal von links nach rechts“ |
| Texturname per Skript | Rechteck wählen, `fw.getDocumentDOM().setFillNColorNTexture({category:'fc_Solid'}, '#ff0000', '<Name aus der Texturliste>')`, danach Texturmenge im Inspektor auf 50 | Inspektor zeigt die Textur, die Füllung ist sichtbar texturiert |
| „Andere…“ als Werkzeugvorgabe | Rechteck-Werkzeug ohne Auswahl, Fülltextur ▸ „Andere…“, eine PNG-Datei außerhalb der Texturordner wählen (z. B. Desktop), Texturmenge 50, Rechteck aufziehen | Feld zeigt den Dateinamen ohne Endung (kein Pfad). Das neue Rechteck ist texturiert, der Inspektor zeigt dieselbe Textur |
| F7 | Messablauf oben, im Original | Vorgabe für den Nachbau |

## Neue Befunde

- (von AU-3, klein, zur Kenntnis) Eine Textur ohne Menge wirkt nicht sichtbar: `pi.fill {texture}` lässt `textureAmount` unverändert, und die steht bei neuen Füllungen auf 0. Das gilt auch für das Skript. Ob das Original beim Setzen einer Textur per JSF eine Menge vorgibt, ist nicht gemessen.

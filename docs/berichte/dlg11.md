# DLG-11: Datei-Info (D10) in Größe und Titel wie gemessen

## Umgesetzt (Commit siehe `git log --grep "DLG-11"`)
- `crates\fp-ui\src\dialogs\file_info.rs`:
  - Fenster jetzt in der gemessenen Größe: außen 680 × 620 (PARITAET_B D10), Client 664 × 581 (`SIZE`).
    Umrechnung wie bei der Bildvorschau (außen 714 × 621 → Client ≈ 698 × 582).
  - Fenstertitel = Dateiname des Dokuments, wie gemessen („Unbenannt-77.fw.png“); ohne Namen „Datei-Info“ (`window_title`).
  - Die größere Fläche wird genutzt: Beschreibung 130 px hoch, Schlüsselwörter und Copyright je 50 px; Gruppe „Datei“ darunter.
  - Felder, Startfokus (Titelfeld), OK/Abbrechen, Esc, Engine `file.fileInfo` und der eine Verlaufsschritt „Datei-Info“ bleiben wie in DLG-10.
- `dialogs\mod.rs` nicht berührt. Keine neuen Texte, keine neuen Engine-Parameter (siehe unten).
- In Hauptfenstern unter etwa 680 × 650 pt begrenzt der Dialograhmen (RAND-2b) das Fenster und zeigt den Inhalt mit Bildlauf. Die Erlaubnisliste von `clipping_audit` bleibt unverändert.

## Nicht umgesetzt, weil die Messung fehlt (nicht geraten)
Das Fenster des Originals ist ein eingebettetes Fremdfenster eines Abspielers (`SWFPLAYER_SWFWINDOWCONTAINER`, Inhalt aus einer externen Bedienfeld-Komponente). Die Win32-Messung (`re\_testdateien\ux\paritaet\dialoge\o_dialoge_bericht.txt` Z. 261) sieht darin keine Bedienelemente. PM-5 hat kein Bild abgelegt, weil es den Produktnamen zeigt. In `re\` und `docs\` fehlen deshalb:
1. Liste, Reihenfolge und Beschriftung der Register bzw. Seiten.
2. Felder je Seite: Beschriftung, Art (Eingabe, mehrzeilig, Klappliste, nur Anzeige), Lage und Größe.
3. Knöpfe: Beschriftung, Reihenfolge, Lage. Dazu Zusatzknöpfe wie Vorlagen, Importieren oder Exportieren.
4. Startfokus.
5. Verhalten von Esc bei echter Taste. Gemessen ist nur: „Esc als Nachricht schließt es nicht“. Firepixel schließt bei Esc ohne Änderung, wie bei allen Dialogen.
6. Ob das Fenster in der Größe veränderbar ist.

Die Angabe „IPTC, Kamera …“ in `dlg10.md` stammt nicht aus einer Messung. Deshalb gibt es weiterhin nur die Seite „Beschreibung“ und keine erfundenen Register. Folglich kommen auch keine neuen XMP-Felder und keine neuen Engine-Parameter dazu.

**Messauftrag (Vorschlag PM-n):**
- Datei ▸ Datei-Info im Original öffnen.
- Je Register ein Bildschirmfoto aufnehmen. Wegen §1a nur auswerten, nicht ablegen, oder vor dem Ablegen den Produktnamen schwärzen.
- Als Text festhalten: Registernamen in Reihenfolge, je Seite die Feldbeschriftungen mit Art, Lage und Größe im Client, Knöpfe, Startfokus, Esc per echter Taste, Größenänderung.
- Mit dieser Liste kann ein Folgeauftrag die Seiten feldgenau bauen. Felder ohne Gegenstück in Firepixel kommen dann als XMP-Felder in das vorhandene Paket (Erkennung und Erhalt fremder Inhalte: `fp-engine\src\commands\file_info.rs`).

## Geprüft
- `cargo test -p fp-ui --test all`:
  - `dialogs_dlg11::` (4 neu): Größe, Titel = Dokumentname, alle 5 Felder sowie OK/Abbrechen im Fenster in de und en, Startfokus und ein Verlaufsschritt.
  - `dialogs_dlg10::` (7) und `dialogs_dlg1*`
  - `clipping_audit::` (schneller Lauf)
- `cargo check --workspace --all-targets`, `cargo test -p fp-neutral`, `cargo test -p fp-i18n`: grün.

## PARITAET_B §2
- D10: Größe und Titel umgesetzt, Nachprüfung ausstehend. Register und Felder je Seite bleiben ❔, weil die Messung fehlt (siehe oben).

## Nachprüfung (NACHPRUEFUNG)
- **D10 (DLG-11):**
  - Ablauf: Dokument „Unbenannt-1“ öffnen, dann Datei ▸ Datei-Info… (Strg+Alt+Umschalt+F).
  - Erwartet: Das Fenster ist außen 680 × 620 und trägt den Dateinamen des Dokuments als Titel.
  - Alle Felder und Knöpfe sind sichtbar, in de, en und qps sowie in den 3 Designs.
  - Tippen landet im Titelfeld. OK legt den Verlaufsschritt „Datei-Info“ an.

## Neue Befunde
- (für PM) D10: Register, Felder, Knöpfe und Startfokus des Datei-Info-Fensters im Original messen (Messauftrag oben). Ohne diese Messung ist keine feldgenaue Umsetzung möglich.

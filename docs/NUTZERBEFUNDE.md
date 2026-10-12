# Vom Nutzer selbst gemeldete Fehler (Sammelliste)

Herausgesucht am 12.10.2026 aus dem Verlauf vom 11./12.10. Jeder Punkt muss **im echten Programm mit echter Maus/Tastatur** nachgeprüft werden, bevor er als erledigt gilt (AGENTEN_REGELN §4, §4b, §4c). Die Bilder des Nutzers liegen unter `C:\Users\ADMINI~1\AppData\Local\Temp\claude\C--Users-Administrator-Desktop-REA\e1341d9d-f448-465f-9142-39283e76de48\images\<n>.png`.

Legende: ✅ behoben **und** im echten Fenster nachgeprüft · 🔧 laut Befund behoben, Nachprüfung im echten Fenster fehlt · ❌ offen

| Nr | Meldung des Nutzers | Bild | Stand | Nachweis / zuständig |
|---|---|---|---|---|
| N01 | Filter/Live-Effekt (z. B. Glühen): Größe mehrfach ändern stapelt den Effekt (50× Glühen) | – | 🔧 | BEFUNDE IBD/LW („50× Glühen“), Test `dialogs_effects.rs` |
| N02 | Dialoge/modale Fenster lassen sich nicht frei verschieben | 4 | 🔧 | BEFUNDE Z. 88 (IBD, `dialogs::dialog_frame`) |
| N03 | Strg+Mausrad springt direkt auf 6 % bzw. 6400 % statt in Stufen | 4 | 🔧 | UX_ABGLEICH_A Z01–Z15 ✅ (LW) |
| N04 | Beim Verschieben (gedrückt halten + ziehen) ist das Objekt **über die Leinwand hinaus** sichtbar; im Original nur innerhalb der Leinwandfläche (Vorschau an der Leinwand abgeschnitten) | 5, 6 | ❌ | kein Befund gefunden → FV |
| N05 | Strg+C/Strg+V usw. kopieren keine Objekte/Ebenen | – | 🔧 | BEFUNDE LW C15–C19 (`input.rs::run_clip`) |
| N06 | Text im Rückfrage-Dialog („Möchten Sie die Änderungen … speichern?“) ist **auseinandergezogen wie Blocksatz**; außerdem zeigt er einen kryptischen Dateinamen (UUID) statt des Dokumentnamens | 7 | ❌ | kein Befund gefunden → FV |
| N07 | Bild per Drag & Drop in ein offenes Dokument ziehen öffnet ein neues Dokument statt es einzufügen | – | 🔧 | FUNKTIONSTEST Z. 163 (Ablage als Objekt) |
| N08 | Taskleisten-Icon fehlte | 8–10 | ✅ | vom Nutzer bestätigt („jetzt geht es“) |
| N09 | Beim Start erscheint das Fenster kurz, verschwindet und kommt wieder | 8 | ❌ | kein Befund gefunden → FV |
| N10 | Beim Zoomen ist die Leinwand 1–2 Bilder lang nach rechts versetzt, dann wieder mittig | 11–13 | ❌ | kein Befund gefunden → FV |
| N11 | Dialoge sehen „zusammengebastelt“ aus, Startbildschirm „ohne Liebe“ → schöner machen | 14–17 | ❌ teilweise | DESIGN begonnen (`docs\DESIGN.md` §13, Nachher-Bilder fehlen) |
| N12 | KI-Zauberstab: nach einem Klick kann man nicht erneut woanders klicken (beim normalen Zauberstab geht es) – soll genau so wie der normale funktionieren | – | ❌ | kein Befund gefunden → FV |
| N13 | Großer Text: Auswahlrahmen bleibt klein (Bild 18) | 18 | 🔧 | LW `19536aa` „Textgrenzen aus dem Satz“ – nachprüfen |
| N14 | Schriftgrößen-Schieber im Inspektor springt in 100er-Schritten (Bild 19) | 19 | ❌ | Kennlinie des Popup-Schiebers wie im Original (siehe FV-1 T4) |
| N15 | Text anfangen, dann unten im Inspektor die Größe ändern → oben muss es live mitändern | 18 | 🔧 | LW `19536aa` „Textattribute live im Editor“ – nachprüfen |
| N16 | Agenten öffnen „Speichern unter“ und bleiben hängen | – | 🔧 | AU: nicht blockierende Datei-Dialoge (`file_dialog.rs`) |
| N17 | Nirgends der Name des Originals oder seines Herstellers in UI und Quelltext | – | 🔧 | `cargo test -p fp-neutral` |
| N18 | Viele „Pick an app“-Fenster beim Testen | – | 🔧 | AGENTEN_REGELN §4a, `FP_NO_SHELL_OPEN` |

Wer einen Punkt bearbeitet: Spalte „Stand“ und „Nachweis“ aktualisieren (Commit, Test, Bild aus dem echten Fenster).

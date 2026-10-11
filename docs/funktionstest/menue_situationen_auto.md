# Hauptmenü in Sondersituationen – automatischer Gesamttest

Erzeugt von `crates\fp-ui\tests\gesamt_menu.rs` (`cargo test -p fp-ui --test gesamt_menu`). Nicht von Hand ändern.

**8/8 Einträge ✅**

| Eintrag | Befehl | Ergebnis | Hinweis |
|---|---|---|---|
| Situation „leere Auswahl“ | `70 verfügbare Menübefehle` | ✅ Doc | 70/70 ohne Befund |
| Situation „alles ausgewählt“ | `195 verfügbare Menübefehle` | ✅ Doc | 195/195 ohne Befund |
| Situation „gesperrtes Rechteck“ | `70 verfügbare Menübefehle` | ✅ Doc | 70/70 ohne Befund |
| Situation „ausgeblendetes Rechteck“ | `145 verfügbare Menübefehle` | ✅ Doc | 145/145 ohne Befund |
| Situation „Gruppe“ | `139 verfügbare Menübefehle` | ✅ Doc | 139/139 ohne Befund |
| Situation „Symbolinstanz“ | `139 verfügbare Menübefehle` | ✅ Doc | 139/139 ohne Befund |
| Situation „Seite 2 mit Objekt“ | `145 verfügbare Menübefehle` | ✅ Doc | 145/145 ohne Befund |
| Situation „Status 2“ | `145 verfügbare Menübefehle` | ✅ Doc | 145/145 ohne Befund |

# MN-3: Bedienfeldmenüs (≡)

Agent MN-3, Slot s8, 12.10.2026. Vorgabe: PARITAET_B §3 (Messung PM-7, Lage S2 = ein Rechteck gewählt, 1 Seite, 1 Status, kein Stil und kein Verlaufsschritt gewählt), Rohdaten `re\_testdateien\ux\paritaet\panels\o_panelmenues.txt`. Das Original wurde nicht benutzt.

## Umgesetzt

Commit: siehe `git log --grep "MN-3:"` (ein Commit auf main).

| Bedienfeld | vorher | jetzt (wie im Original) |
|---|---|---|
| alle | ≡ endet mit Hilfe │ Schließen, Registerkartengruppe schließen | ≡ endet mit **Hilfe**. **Schließen** und **Registerkartengruppe schließen** stehen im **Kontextmenü des Reiters** (Rechtsklick, neu: `menu::build_tab_menu`, `docking.rs`) |
| Verlauf | alle aktiv | Ausgewählte Schritte wiederholen, Schritte kopieren, Als Befehl speichern… **grau ohne gewählte Schritte** (Auswahl im Bedienfeld) |
| Stile | alle aktiv | ohne gewählten Stil grau: Stil duplizieren…, Stile löschen, Stil umbenennen…, Stil neu definieren, Abweichungen löschen, Verknüpfung mit Stil aufheben, Stiloptionen…, Stilbibliothek speichern |
| Status (1 Status) | Status löschen grau, In Status kopieren aktiv, Auf Status verteilen grau | **Status löschen aktiv** (beim letzten Status ohne Wirkung, er bleibt stehen), **In Status kopieren grau** (erst ab 2 Status), **Auf Status verteilen aktiv** schon mit einem Objekt (`pi.state.distribute` braucht jetzt 1 statt 2 Objekte) |
| Seiten (1 Seite) | Seite löschen grau | **Seite löschen aktiv** (bei der letzten Seite ohne Wirkung) |
| Ebenen | Maske bearbeiten aktiv | **Maske bearbeiten grau ohne Maske** (nur im Menü; per Skript unverändert) |
| Optimieren | Rückmeldung ohne ✓; JPEG-Einträge aktiv, bei GIF „JPEG-Ränder scharfzeichnen ✓“ | **Farbfeld-Rückmeldung anzeigen ✓** (Vorgabe jetzt ein, auch für die Leinwand-Anzeige); **Progressives JPEG/JPEG-Ränder scharfzeichnen nur bei JPEG aktiv**, sonst grau ohne ✓; Palettenbefehle (Sperrung aufheben, Nicht verwendete Farben entfernen, Paletteneintrag ersetzen, Transparent machen, Bearbeitung entfernen, Alle Bearbeitungen entfernen) grau bei Formaten ohne Farbtabelle (PNG 24/32, JPEG); Paletteneintrag ersetzen, Transparent machen, Bearbeitung entfernen zusätzlich grau ohne gewählte Palettenfarbe. Mit PNG32 jetzt Eintrag für Eintrag wie die Messung |
| Eigenschaften | ≡ = Inspektor-Optionsmenü (Als Stil speichern…, Alle ein, Alle aus, Zusatzmodule suchen…, Volle/Halbe Höhe, Schließen), Hilfe fehlte | ≡ = nur **Hilfe**. Die Effektoptionen der Ressource 6921 (Als Stil speichern… │ Alle ein, Alle aus │ Zusatzmodule suchen…) stehen im Effekt-Menü **„+ ▸ Optionen“** des Inspektors (vorher dort nur drei handgemachte Einträge ohne „Zusatzmodule suchen…“). Volle/Halbe Höhe und Schließen im Kontextmenü der Inspektor-Reiter |

Skripte/MCP: Menü-Freigaben über `menu_when` (nur Menüzustand). Die Optimieren-Schalter lassen sich per Skript weiter setzen (Test `optimize_jpeg_and_palette_entries_follow_the_format`). `FwCommand` reicht `menu_available` jetzt an den inneren Befehl weiter (vorher gingen `menu_when`-Bedingungen der Bedienfeldbefehle verloren).

Geänderte Dateien:
- `crates\fp-engine\src\commands\panel.rs`: Optimieren, Status, Seiten, Maske, `last_kept`, `FwCommand::menu_available`
- `crates\fp-ui\src\menu.rs`: `build_panel_menu`, neu `build_tab_menu`, `build_inspector_menu`, `build_inspector_tab_menu`
- `crates\fp-ui\src\dialogs\panel_menus.rs`: neu `adjust_menu`, Freigabe nach Auswahl im Bedienfeld
- `crates\fp-ui\src\docking.rs`: `TabStripOut.tab_resps`, Reiter-Kontextmenü
- `crates\fp-ui\src\inspector\mod.rs`: ≡ und Reiter-Kontextmenü
- `crates\fp-ui\src\dialogs\effects.rs`: „+ ▸ Optionen“ aus 6921

Fremde Dateien, je ein kleiner Eingriff:
- `crates\fp-ui\src\canvas\overlay.rs`: Vorgabe der Farbfeld-Rückmeldung `false` → `true`, passend zum Menü-Häkchen
- `crates\fp-ui\tests\gesamt_common\mod.rs`: neue Testsituation „Export JPEG“ am Listenende, sonst wären die JPEG-Einträge im Gesamttest nie verfügbar

## Tests

- neu: `crates\fp-ui\tests\panel_menus_mn3.rs`, 11 Tests. Je Bedienfeld Verlauf, Stile, Status, Seiten, Ebenen, Optimieren: Einträge, Reihenfolge, Trennlinien, grau und ✓ in den Lagen „nichts gewählt“ und „Objekt gewählt“, gegen die Liste des Originals. Dazu Freigabe mit gewähltem Schritt/Stil/Farbe/Maske/2 Status, letzter Status bzw. letzte Seite bleibt, Reiter-Kontextmenü aller 23 Bedienfelder, Eigenschaften-≡ und Englisch.
- grün: `panel_menus_mn3::`, `panels_menus::`, `i18n_menue::`, `inspector_i18n::`, `gesamt_menu::gesamt_bedienfeldmenues` (mit `FP_NO_SHELL_OPEN=1`), `fp-engine --lib commands::panel`, `fp-engine --test all all_commands::`, `cargo check --workspace --all-targets`.
- `docs\funktionstest\bedienfeldmenues_auto.md` neu erzeugt: 135 Einträge, weil die 2 × 23 Schließen-Einträge aus dem ≡ weg sind.
- Keine neuen Texte. Benutzt werden die vorhandenen Schlüssel `pmenu-close`, `pmenu-closeGroup`, `pmenu-help`, `pi-menu-*-height`. Die Schlüssel `dlg-fx-all-on`, `dlg-fx-all-off` und `dlg-fx-save-style` in `dialoge.ftl` sind jetzt unbenutzt und können weg.

## Befunde erledigt

- „(von UX-B, Paritätsabgleich – **Funktion**) **Bedienfeldmenüs (≡)**…“: alle Unterpunkte, also Verlauf, Stile, Status, Seiten, Ebenen, Optimieren und Allgemein
- „(von PM-7, PARITAET_B §3) Nachmessung 12.10. in beiden Programmen…“, einschließlich Eigenschaften-≡. Antwort auf die Frage dort: Das Original bietet 6921 im Effekt-Menü „+ ▸ Optionen“ des Inspektors an. Firepixel hat es jetzt dort, vollständig.

## PARITAET_B §3: „umgesetzt, Nachprüfung ausstehend“

Zeilen alle, Optimieren, Verlauf, Seiten, Status, Ebenen, Stile und Eigenschaften. Farbfelder bleibt ✅. Ausrichten/Farbpalette bleibt ❔: Dort steht jetzt nur noch „Hilfe“.

## Nachprüfung (echtes Fenster bzw. Brücke)

1. Rechtsklick auf den Reiter „Ebenen“: Menü mit Schließen und Registerkartengruppe schließen. „Schließen“ blendet Ebenen aus, „Registerkartengruppe schließen“ die ganze Gruppe. Im ≡ steht beides nicht mehr, letzter Eintrag ist „Hilfe“.
2. Eigenschaften-≡: nur „Hilfe“. Rechtsklick auf den Reiter „Eigenschaften“: Volle Höhe ✓, Halbe Höhe, Schließen. „Halbe Höhe“ halbiert den Inspektor.
3. Objekt wählen, Inspektor „+“ ▸ Optionen: Als Stil speichern… │ Alle ein, Alle aus │ Zusatzmodule suchen…; „Alle aus“ schaltet die Effekte aus.
4. Optimieren: bei GIF sind die JPEG-Einträge grau und ohne ✓. Nach Wechsel auf JPEG sind sie aktiv. Rückmeldung ✓.
5. Verlauf: ≡ zeigt die drei Schrittbefehle grau. Nach Klick auf einen Schritt sind sie aktiv.
6. Stile: ≡ ohne gewählten Stil wie die Messung. Nach Klick auf einen Stil sind Duplizieren, Löschen usw. aktiv.
7. Status/Seiten mit 1 Status/1 Seite: „Status löschen“ bzw. „Seite löschen“ aktiv, Klick ändert nichts.

## Offen / neue Befunde

- Die Lage der Inspektor-Höhe (Volle/Halbe Höhe) im Original ist nicht gemessen. Sie steht jetzt im Reiter-Kontextmenü, damit die Funktion erreichbar bleibt; bitte bei der nächsten Messung prüfen.
- Status-Bedienfeld, Fußknopf „Auf Status verteilen“ (`panels\states.rs`): Er ist weiter erst ab 2 Objekten aktiv, das Menü schon ab 1. Den Knopf im Original nachmessen.
- Die Optimieren-Palettenbefehle bei GIF sind im Original nicht gemessen, PM-7 hat nur PNG32 gemessen.

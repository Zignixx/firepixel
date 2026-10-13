# FIX-E – Bericht (13.10.2026)

Auftrag: Befunde aus CT-3 (`docs\funktionstest\ct3_2026-10-13.md`) und Fix-Liste 1 (Standard-ALT-Text).
Slot `fp-s5`, Build-Ordner `target-s5`. Alle Punkte umgesetzt, je Punkt ein Commit auf `main`.

## Umgesetzt

| # | Punkt | Commit | Ursache / Lösung | Test |
|---|---|---|---|---|
| 1 | `ui_action {"run": "file.open"}` / `"file.saveAs"` über die Brücke hängt | `fed23bf` | **Verklemmung**: `bridge::frame` hält die Sperre `GLOBAL`, während es Aufträge abarbeitet; `file_dialog::request_ex` fragte über `bridge::running_url()` (ebenfalls `GLOBAL`) nach, ob der Dialog in der App erscheinen soll → der UI-Thread wartete auf sich selbst (Menüklick lief außerhalb der Sperre, daher ging er). `running_url` liest jetzt eine eigene URL-Ablage. Gleicher Fehler hätte `ui.mcpToggle` aus einem Brückenauftrag getroffen. `docs\AGENTS.md`/`AGENTS.en.md` §5b: Satz ergänzt („`ui_action` antwortet sofort, der Dialog wartet in der App …“) | `bridge_file_dialog` (eigener Prozess, Brücke prozessweit eingehängt über `bridge::install_detached_for_tests`, Ablauf genau wie §5b: öffnen, beantworten, speichern unter, Titel ohne Stern, doppelter Aufruf → ein Dialog). Mit dem alten `running_url` hängt der Test nachweislich (Wächter beendet nach 60 s) |
| 2 | WM_CLOSE bei minimiertem Fenster mit geändertem Dokument: keine Rückfrage | `f8a6ef1` | `logic()` stellte das Fenster nur wieder her; das Schließen-Ereignis kommt im nächsten Bild nicht mehr in `root_ui` an. Neu `FirepixelApp::close_while_minimized` merkt den Wunsch als `Action::Quit` (wiederholt: eine Rückfrage) | `file_dialogs_dlg3::close_from_outside_while_minimized_restores_and_asks` |
| 3 | Startseite: Tab schaltet Bedienfelder aus/ein, Pfeile ohne Wirkung | `7a3c178` | `input::handle` lässt auf der Startseite (kein Dokument, Startbildschirm sichtbar) Tab und Pfeile aus. `welcome::keyboard`: Tab/Umschalt+Tab in Lesereihenfolge (ringförmig, auch wenn egui den Fokus schon in die Bedienfelder geschoben hatte), Pfeile räumlich zum nächsten Eintrag (90°-Kegel, am Rand stehen bleiben); egui’s eigene Pfeil-Fokusbewegung am Bildende wird dabei abgeschaltet. Mit Dokument bleibt Tab „Bedienfelder ausblenden“ | `welcome_keys_fixe` (ganzes Hauptfenster, 30× Tab), Unit-Tests in `welcome.rs`, `welcome_des2` grün |
| 4 | Standard-ALT-Tag geht beim Speichern als FW-PNG verloren (Fix-Liste 1) | `2dd23df` | Tag des Originals unbekannt (re\08 §1.5) → `DALs` in der eigenen Erweiterung `fpSv` im `EXPv` (wie Sprite-Felder NZ-4); ohne ALT-Text/Sprite-Felder wird `fpSv` weiter nicht geschrieben | `html_alt_fixe` (20× ändern, speichern, schließen, öffnen, zweimal speichern) |
| 5 | HTML aktualisieren mit Textdatei: Statuszeile mit technischem Vorspann | `2883469` | Menüweg (Datei-Dialog in `actions.rs`) führt jetzt über `menu_dialogs::update_html` (Erfolg Statuszeile, Fehler Meldung); Engine meldet „keine HTML-Seite“ als `EngineError::Failed` ohne „ungültige Parameter für …“ | `html_alt_fixe::update_html_with_a_text_file_shows_a_plain_message` (de/en, 3×), `export_e3`, `all_commands` grün |
| 6a | Brücken-Dokumente heißen englisch in deutscher Oberfläche | `f40e791` | `fp_i18n::with_content_language`: Schlüssel `msg-default-*` (Standardnamen: Unbenannt-n, Ebene n, Webebene, Seite n, Hintergrund, … Kopie) in der Sprache der Oberfläche; der Brückenserver setzt sie im UI-Thread, Antworten bleiben in der Agentensprache. Headless unverändert | `fp-mcp` `tests\bridge.rs::bridge_names_new_documents_in_the_ui_language` |
| 6b | Rote 1-px-Linie ab Seitenursprung, wenn der Zeiger das Fenster verlässt | `e9f65f3` | Vorauswahl-Hervorhebung des Zeigerwerkzeugs blieb stehen, weil kein Bewegungsereignis mehr kam. Leinwand zeigt `Overlay::Highlight` nur, solange der Zeiger über ihr ist (oder eine Taste gedrückt) | `canvas_hover_fixe` (3× hinein/hinaus; ohne Fix rot) |
| 6c | 900 × 650: Gruppe Dokumentbibliothek/Allgemeine Bibliothek lässt sich nicht aufklappen | `06cf937` | Das Dock klappte mangels Höhe immer die unteren Gruppen ein. `DockUi::prefer` (zuletzt per Reiterklick/Menü *Fenster* geöffnetes Bedienfeld) bleibt offen, andere klappen dafür zu; Klick auf eine nur wegen der Höhe eingeklappte Gruppe (`DockUi::forced`) klappt sie auf | `dock_small_fixe`, `layout::` grün |
| 6d | Mit `FP_NO_SHELL_OPEN` zwei Profilwurzeln (Vorlagen/Startordner im Wächter-Profil) | `2a466aa` | `known_folder` rief im Testlauf `use_test_profile()` auf und schaltete damit alle Profilordner auf `%TEMP%\firepixel-test-profiles\<pid>` um. Jetzt: Umleitung, sonst die Wurzel aus einem `APPDATA` außerhalb des echten Profils (`<home>\Roaming` bzw. `<home>\AppData\Roaming`), sonst das Testprofil | Unit-Test `profile::home_of_a_test_instance_from_its_appdata`, `nz5` grün |

BEFUNDE abgehakt in `d7612ea` (CT-3-Zeilen und Fix-Liste 1).

Geprüft: gezielte Tests oben, `cargo check --workspace --all-targets`, `cargo test -p fp-neutral`, `cargo test -p fp-i18n`,
`cargo test -p fp-fwpng`, `fp-mcp --test bridge/sprache`, clippy ohne neue Warnungen in geänderten Stellen.
Keine neuen Texte in `.ftl`.

## Annahmen

- Punkt 6 = die fünf „niedrig“-Zeilen von CT-3 (HTML aktualisieren, Brückennamen, rote Linie, Dock 900 × 650, zwei Profilwurzeln); „HTML aktualisieren“ ist zugleich Punkt 5.
- Startseite: Pfeiltasten räumlich (wie egui/Windows-Listen), Tab ringförmig nur innerhalb der Startseite. Das Original hat keinen Startbildschirm mit Tastaturbedienung zum Vergleich.
- Rote Linie: nicht in der echten Instanz nachgestellt; gefunden wurde die stehen bleibende Hervorhebung (im Test reproduziert). Dass sie im CT-3 am Seitenursprung lag, passt zu einer Hervorhebung aus einem anderen Dokument bzw. Zustand – bitte in der Nachprüfung bestätigen.
- `install_detached_for_tests` ist eine öffentliche, versteckte Testhilfe in `bridge.rs`.

## PARITAET (umgesetzt, Nachprüfung ausstehend)

- DLG-3 Schließen von außen bei minimiertem Fenster
- DES-2 Startbildschirm Tastaturbedienung (Tab/Pfeile)
- HTML-Einrichtung „Standard-ALT-Tag“ im FW-PNG

## Nachprüfung (echtes Fenster, Brücke)

1. `ui_action {"run": "file.open"}` → `ui_file_dialog {}` (`mode: internal`) → `choose` → Dokument offen; dasselbe mit `file.saveAs` → `ui_state` Titel ohne Stern. Erwartet: keine Zeitüberschreitung, `Responding = True`.
2. Geändertes Dokument, Fenster minimieren, `PostMessage WM_CLOSE` → Fenster kommt zurück **mit** Rückfrage Ja/Nein/Abbrechen; Abbrechen lässt offen; zweimal WM_CLOSE → eine Rückfrage.
3. Startseite: Tab 10× → Bedienfelder bleiben, Fokusrahmen wandert durch die Einträge; Pfeile ab/rechts/auf/links bewegen ihn zwischen Karten; Eingabe löst aus.
4. HTML-Einrichtung ▸ Dokumentspezifisch ▸ Standard-ALT-Tag setzen, als FW-PNG speichern, schließen, öffnen → Wert steht noch da.
5. Datei ▸ HTML aktualisieren mit `.txt` → Meldungsfenster „… ist keine HTML-Seite …“ ohne „ungültige Parameter“.
6. Brücke bei deutscher Oberfläche: `new_document` → Ebenen „Ebene 1“, „Webebene“, Titel „Unbenannt-n“.
7. Zeiger über ein Objekt (rote Hervorhebung), dann aus dem Fenster → Hervorhebung weg; Bedienfeld ziehen und loslassen → keine rote Linie.
8. Fenster 900 × 650: Reiter „Dokumentbibliothek“ klicken bzw. *Fenster ▸ Dokumentbibliothek* → Gruppe klappt auf, eine andere Gruppe klappt dafür zu.
9. Instanz mit `fpmcp.ps1` (eigenes APPDATA, `FP_NO_SHELL_OPEN`): Export-Dialog startet in `<FPHOME>\Pictures` (bzw. `<FPHOME>`), Vorlagen unter `<FPHOME>\Roaming\Firepixel\Templates`; kein Ordner unter `%TEMP%\firepixel-test-profiles\<pid>` nötig.

## Neue Befunde

- keine

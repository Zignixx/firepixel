# MN-6 – Menüs höher als das Fenster (13.10.2026)

Slot s6, das Original nicht benutzt. Spezifikationen: `re\` hat zur Menüdarstellung nur den Hinweis, dass
das Original native Windows-Menüs verwendet (`re\_testdateien\ux\paritaet\paritaet_menues.md`: `MN_GETHMENU`).
Umgesetzt ist deshalb das Verhalten nativer Windows-Menüs.

## Umgesetzt (Commit `a6fd9d3`, auf main nach PERF-1 und MN-5)

- **Neues Modul `crates\fp-ui\src\menu_scroll.rs`**, zentral für alle Menüs, die über `menu::show_nodes` laufen
  (Hauptmenüs, Untermenüs, ≡-Bedienfeldmenüs, Kontextmenüs der Leinwand, der Bedienfeld-Zeilen und der Reiter)
  und für die Tastaturmenüs (`menu_kbd`):
  - Ein Menü wird auf den Platz im Fenster begrenzt:
    - Hauptmenüs auf den Platz unter ihrem Titel
    - Kontext-, Bedienfeld- und Untermenüs auf die Fensterhöhe; egui schiebt sie dann ganz ins Fenster, nach oben bzw. auf die andere Seite des Eintrags
  - Ist es zu hoch, stehen oben und unten Pfeilleisten (16 pt):
    - Verweilen blättert fortlaufend, etwa 14 Einträge je Sekunde
    - ein Klick blättert einen Eintrag weiter
    - das Mausrad blättert auch
    - ein grauer Pfeil zeigt das Ende an
  - Menüs, die passen, sind unverändert.
- **Schließen wie unter Windows:** Ein Klick ins Menü schließt es nicht mehr, sondern nur noch ein Klick daneben. Das gilt für Pfeilleisten, Trenner und graue Einträge (`menu::menu_info`, `menu::MENU_CLOSE`). Einträge schließen das Menü wie bisher selbst. Angepasst in `menu.rs` (Menüleiste, `popup_menu`), `canvas\context.rs` und `panels\common.rs` (je zwei Zeilen).
- **Text ▸ Schrift** zeigt jetzt Pfeilleisten statt der Bildlaufleiste. Die alte Sonderregel ab 40 Einträgen ist entfallen. `LONG_SUBMENU` gilt weiter für das ✓ am Familienkopf.
- **Tastatur (`menu_kbd::show`):**
  - Nach jedem Wechsel der Markierung (↑/↓, Pos1/Ende, Zugriffstaste) wird der markierte Eintrag in Sicht gescrollt. ↑ auf dem ersten Eintrag springt zum letzten und scrollt mit.
  - Das erste Menü ist auf den Platz unter dem Titel begrenzt, Untermenüs auf die Fensterhöhe.
  - Passt ein Untermenü rechts nicht, öffnet es links neben dem übergeordneten Menü.
- **Erlaubnisliste** `tests\clipping_audit_allow.txt`: 4 → 0 Zeilen. Gestrichen sind Hauptmenü 1, Hauptmenü 9, Bedienfeldmenü 2 und Kontextmenü Leinwand. Dazu ist eine kaputte Kommentar-Restzeile entfernt.
  - Voller Lauf (`FP_RAND_FULL=1`): **0 Funde**, 414 Hinweise, „neu: 0“.
  - `rand_auto.md` ist neu erzeugt. Das Ausschnittbild `rand_01_…hauptmen_9.png` ist entfallen.

## Befunde erledigt

- RAND-2b, Befund HG „Menüs, die höher als das Hauptfenster sind, ragen unten hinaus …“ (Hauptmenü Datei, zweites Bedienfeldmenü, Fenster bei 1024 × 768, Kontextmenü der Leinwand)

## PARITAET

Menüs (Darstellung zu hoher Menüs wie native Windows-Menüs): „umgesetzt, Nachprüfung ausstehend“. N23 (P-RAND): Menüzeilen erledigt.

## Tests

- Neu: `tests\menu_scroll_mn6.rs` mit 5 Tests, alle bei 800 × 512.
  - **Datei:** Der letzte Eintrag ist erst nach dem Verweilen über dem unteren Pfeil zu sehen. Ein Klick auf den oberen Pfeil lässt das Menü offen. „Beenden“ wird per Klick ausgelöst.
  - **Datei, Mausrad:** Es blättert hin und zurück.
  - **Fenster** (auch bei 1024 × 768, bei 800 im Überlaufmenü „»“): Der letzte Eintrag aktiviert das zweite Dokument.
  - **Tastatur:** Mit F10, ↓, ↑ (springt zum letzten Eintrag), Pos1 und Ende ist der Eintrag jeweils ganz zu sehen. Das Tastaturmenü liegt im Fenster, Eingabe löst „Beenden“ aus.
  - **Kontextmenü der Leinwand** (Mitte und unten): liegt ganz im Fenster.
- Grün: `menu_scroll_mn6::`, `menu_mn2::`, `menu_mn4::`, `panel_menus_mn3::`, `gesamt_menu::`, `menu_fixes_fv13::`, `nz3::`, `clipping_audit::` (mit `FP_RAND_FULL=1`), `cargo check --workspace --all-targets` ohne Warnungen, `fp-neutral`
- Keine neuen Nutzertexte, deshalb weder fp-i18n noch der Scanner.

## Nachprüfung (NACHPRUEFUNG)

- **Echtes Fenster, 800 × 600 und 100 %:** Datei öffnen und unten über dem Pfeil verweilen.
  - Erwartet: Das Menü blättert flüssig bis „Beenden“, und der Pfeil wird grau.
  - Ein Klick auf einen Pfeil lässt das Menü offen. Das Mausrad blättert. Ein Klick auf „Beenden“ wirkt.
- **Echtes Fenster, 1024 × 768:** Fenster-Menü öffnen.
  - Erwartet: Das Menü endet am Fensterrand mit Pfeilleiste, die Dokumentliste ist erreichbar.
- **Text ▸ Schrift:**
  - Erwartet: Die lange Liste hat Pfeilleisten. Ein Untermenü der Schnitte öffnet sichtbar, unten im Fenster nach oben verschoben.
- **Rechtsklick unten auf die Leinwand** bei 800 × 600:
  - Erwartet: Das Kontextmenü ist nach oben verschoben und höchstens fensterhoch. Der letzte Eintrag ist per Pfeil erreichbar.
- **≡-Menü eines Bedienfelds** in einem niedrigen Fenster:
  - Erwartet: Das Menü liegt ganz im Fenster.
- **Alt+D, dann ↑:**
  - Erwartet: Die Markierung springt auf „Beenden“ und das Menü scrollt mit. Mit Pos1 und Ende ebenso.
- **Graue Einträge und Trenner anklicken:**
  - Erwartet: Das Menü bleibt offen wie unter Windows. Ein Klick daneben schließt es.

## Neue Befunde

Keine.

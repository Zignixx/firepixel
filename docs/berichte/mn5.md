# MN-5: Nacharbeiten zu MN-4

Agent MN-5, Slot s4, 13.10.2026. Grundlage: `docs\berichte\mn4.md`, `bv4.md`, PARITAET_B §1, `re\17-bildvorschau.md` §7 und die Texttabelle des Originals (`re\inventory\resources_de.json`). Das Original habe ich nicht bedient.

Commit auf main: `3cc575f` (im Slot `4079d2d`).

## Umgesetzt

### 1. Filter ▸ Wiederholen wie im Original
- Nach einem Filter heißt der Eintrag jetzt „Wiederholen: <Menütext des Filters>“ mit „...“ bei Filtern mit Dialog. Beispiele: „Wiederholen: Umkehren“, „Wiederholen: Farbton/Sättigung...“.
- Quelle ist Text 10722 „Wiederholen: %s“, ohne Zugriffstaste. Das passt zu PM-7: Dort wurde „Wiederholen: Farbton/Sättigung...“ gesehen.
- Englisch: „Repeat: <filter>...“. Vom Original gibt es keinen englischen Text; gewählt habe ich das direkte Gegenstück, passend zu „Repeat Filter“.
- Ohne vorherigen Filter bleibt „Filter wiederholen“ (grau).
- Umsetzung: neuer Schlüssel `menu-filter-repeatWith` in `menue.ftl` de/en, Zweig `filter.repeat` in `menu.rs::with_step_name`.

### 2. Mitgelieferte Befehlsskripte übersetzbar
- Neue JSF-Funktion **`fp.t(key, args)`** (`fp-script\src\imp\mod.rs`, eingehängt in `runtime.rs`). Sie liefert den Text in der aktuellen Sprache der Oberfläche; Fluent-Variablen kommen als Objekt, z. B. `{ name: … }`. Ein unbekannter Schlüssel liefert den Schlüssel selbst.
- Warum diese Lösung und keine Skripte je Sprache: Die Texte folgen auch einer späteren Sprachumstellung. Eine zweite Kopie jedes Skripts ist nicht nötig. Neue Sprachen brauchen nur die ftl-Datei.
- Umgestellt sind alle sichtbaren Texte der mitgelieferten Skripte:
  - Ebenenname „Rahmen“
  - die Eingabeaufforderungen in „Drehen und Ausblenden“, „Breite festlegen“, „ALT-Text festlegen“ und „Größe ausgewählter Objekte ändern“
  - die Meldungen von Bildschirmfoto, Demo, Symbolskript erstellen, Autom. Vektormaske und Gemeinsame Palette
  - Schlüssel `builtin-cmd-*` in `skript.ftl` de/en; der deutsche Wortlaut ist unverändert
- Bei Bildschirmfoto, Symbolskript und Gemeinsame Palette ist nur der Text umgestellt, die Logik ist unverändert (Hinweis des Koordinators wegen CMD-1).
- **Bestehende Befehlsordner:** Die MN-4-Fassungen stehen wortgleich in `FIXED_GERMAN`.
  - `update_outdated` ersetzt sie beim Start, unter deutschen wie englischen Dateinamen.
  - Vom Nutzer geänderte Dateien bleiben unberührt.
  - Die Umbenennung alter Namen (RENAMED_DE) erkennt auch diese Fassungen (`is_old_version`).
  - Die Literale habe ich einmalig gegen `git show main:…builtin_commands.rs` geprüft: Sie sind gleich, und alle übrigen Skripte sind unverändert.
- Hinweis für CMD-1: Wer eines der drei Skripte neu schreibt, nimmt die bisherige Fassung in `FIXED_GERMAN` bzw. `OUTDATED` auf und nutzt für Texte `fp.t`.

### 3. „Maske deaktivieren“ ohne Kürzel im Menü
- Zwei Messungen im Original (UX-B und PM-7, PARITAET_B Zeile „Modifizieren > Maske > Maske deaktivieren“) zeigen kein Kürzel. Damit ist das Menü eindeutig belegt; Firepixel zeigt deshalb keins mehr (`menu.rs::menu_shortcut`).
- Die Taste selbst bleibt im Kürzelsatz Firepixel: Strg+Umschalt+K, laut `re\10` und `shortcuts.json` aus den Satzdaten des Originals. Ob sie im Original wirkt, ist nicht gemessen; siehe Nachprüfung.

### 4. Optimieren ▸ Einstellungen löschen mit Rückfrage 3169
- Ablauf: Auswahl der Vorgabe, dann die Rückfrage „Aktuelle Einstellungen löschen? Dies kann nicht rückgängig gemacht werden.“ (en: „Delete current settings? This cannot be undone.“).
- Titel „Firepixel“ wie bei der Rückfrage zu „Wiederherstellen“, Knöpfe Ja/Nein.
- Nein, Esc und Schließen löschen nichts. Ja löscht, ohne zweite Rückfrage.
- Skripte und MCP rufen den Befehl direkt auf und löschen weiter ohne Rückfrage.
- Umsetzung: Zweig in `actions.rs::run_command` (wie bei `file.revert`, Parameter `confirmed`), Schlüssel `dlg-pmenu-delete-settings-confirm` in `dialoge.ftl` de/en.
- Die Knöpfe des Originals sind nicht gemessen (nur der Text).

### Zusatzpunkte des Koordinators
- **Kürzel „Im Dateibrowser durchsuchen…“:** gemessen ist im Original Strg+Alt+O (PARITAET_B Zeile 155, UX-B `paritaet_menues.md`). `re\10` bzw. `o_menues.tsv` nennen Strg+Umschalt+O, das sind statische Daten. Es gilt der gemessene Wert, also **unverändert Strg+Alt+O**. Der Eintrag bleibt aktiv (Nutzerentscheidung).
- **`fp-mcp --test sprache` wieder grün:**
  - Im Katalog `fp-engine\data\commands.json` hatte `file.browseInBridge` noch die Texte „Bridge durchsuchen...“ in `path_de`, `label_de`, `menu_label_de` und `status_de`, dazu `path_en` „Browse in Bridge“. Jetzt stehen dort die neuen Texte („Im Dateibrowser durchsuchen...“ / „Browse in File Browser“).
  - Weitere veraltete Menüpfade aus MN-4 gibt es nicht: MN-4 hat sonst nur Befehlsskripte umbenannt, und die stehen nicht im Katalog. Der Test meldet keine weiteren.
  - `paritaet_menue` bleibt grün (die Zeile des Originals fällt nur aus der Zuordnung).

## Tests
- Neu: `crates\fp-ui\tests\menu_mn5.rs`, 8 Tests:
  - Wiederholen-Beschriftung de/en, auch nach Wiederholen
  - Maske deaktivieren ohne Kürzel
  - Rückfrage: Status, Nein, Ja, en, Weg über MCP
  - Rückfrage mit echten Knöpfen (kittest)
  - mitgelieferte Skripte ohne festen Text, folgen der Sprache
  - `fp.t` mit Variable und unbekanntem Schlüssel
  - alte Fassungen werden ersetzt, Nutzerfassung bleibt
- Angepasst: `menu_mn4::repeat_filter_shows_last_filter_name` (neuer Wortlaut).
- Grün:
  - `menu_mn5::`, `menu_mn4::`, `panel_menus_mn3::`, `gesamt_menu::` (FP_NO_SHELL_OPEN=1), `image_preview_p*`, `menu_fixes_fv13::`, `dialogs_dlg4::`, `dialogs_warnung::`, Scanner `i18n_keine_festen_texte::`
  - `fp-script --test befehle --test builtin_commands_fv13`, `fp-mcp --test sprache`, `fp-engine --test all paritaet_menue::`, `fp-i18n`, `fp-neutral`
  - `cargo check --workspace --all-targets` auf dem Stand nach dem Nachziehen von main

## Erledigte Befunde (Wortlaut-Anfang)
- BEFUNDE „(von PM-7, PARITAET_B §1) **Filter ▸ Filter wiederholen** heißt im Original nach einem Filter …“: jetzt mit dem Wortlaut des Originals erledigt.
- BEFUNDE „(von PM-7, PARITAET_B §1) **Kürzel-Anzeige:** …“: der Rest „Maske deaktivieren ohne Kürzel“ ist erledigt (nur die Anzeige).
- mn4.md „(von MN-4) Die mitgelieferten JSF-Befehle haben feste deutsche Meldungen …“: erledigt (`fp.t`).
- bv4.md „(für IBD/Optimieren-Bedienfeld) §7 nennt für ‚Einstellungen löschen‘ die Rückfrage 3169 …“: erledigt.

## PARITAET → „umgesetzt, Nachprüfung ausstehend“
- PARITAET_B §1: Filter > Wiederholen
- PARITAET_B §1: Modifizieren > Maske > Maske deaktivieren (Kürzelanzeige)
- Optimieren-Bedienfeld ▸ Einstellungen löschen (Rückfrage 3169, re\17 §7)

## NACHPRUEFUNG (Zeilen)
1. Bitmap wählen, Filter ▸ Farbe anpassen ▸ Farbton/Sättigung… mit OK. Erwartet im Filter-Menü: „Wiederholen: Farbton/Sättigung...“, aktiv, Strg+Alt+Umschalt+X; in Englisch „Repeat: Hue/Saturation...“.
2. Modifizieren ▸ Maske. Erwartet: „Maske deaktivieren“ ohne Kürzeltext. Im Original prüfen, ob Strg+Umschalt+K die Maske trotzdem umschaltet; falls nicht, die Taste auch aus dem Satz Firepixel nehmen.
3. Optimieren-Bedienfeld: Einstellungen speichern („Klein“), dann Einstellungen löschen → Vorgabe wählen → OK. Erwartet: Rückfrage „Aktuelle Einstellungen löschen? …“ mit Ja/Nein. Nein lässt „Klein“ in der Vorgabenliste, Ja entfernt es. Im Original die Knöpfe der Rückfrage messen (Ja/Nein oder OK/Abbrechen).
4. Sprache Englisch, Befehle ▸ Web ▸ Gemeinsame Palette bzw. Shared Palette (deutscher oder englischer Ordner). Erwartet: englische Meldung. Ebenso: Größe ausgewählter Objekte ändern zeigt „New size in percent“.
5. Altes Profil mit MN-4-Befehlsordner starten. Erwartet: Die fünf neuen Skripte und die fünf Skripte mit Eingabeaufforderung werden auf die `fp.t`-Fassung gebracht; vom Nutzer geänderte Skripte bleiben.

## Neue Befunde
- (von MN-5) „Einstellungen löschen“ fragt in Firepixel erst nach der Vorgabe. Das Original löscht die gerade gewählte Einstellung des Bedienfelds direkt nach der Rückfrage („Aktuelle Einstellungen …“). Für volle Treue müsste das Optimieren-Bedienfeld die gewählte eigene Vorgabe kennen (`panels\optimize.rs`), dann entfiele der Auswahldialog. Zuständig IBD.
- (von MN-5) Die statische Kürzelspalte in `re\10-werkzeuge-befehle.md` weicht bei mindestens drei Einträgen von der Menümessung ab: Maske deaktivieren, Dateibrowser-Eintrag, Fenster > Verhalten (Strg+Alt+H statt Umschalt+F3). Vorschlag: `shortcuts.json` (Satz Firepixel) einmal gegen PARITAET_B §1 abgleichen.

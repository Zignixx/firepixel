# MN-4: Befehle-Menü, Kürzeltexte, „Filter wiederholen: Name“, neutraler Datei-Eintrag

Agent MN-4, Slot s4, 13.10.2026. Vorgabe: PARITAET_B §1 (Messung PM-7, Zeilen „Befehle >“, „Ansicht > QuickInfos“, „Intelligente Hilfslinien“, „Filter > Wiederholen“, „Datei > Bridge durchsuchen“) und BEFUNDE (I18N-MENUE, „von PM-7“). Das Original wurde nicht benutzt.

## Umgesetzt

Commit: siehe `git log --grep "MN-4:"` (ein Commit auf main).

### 1. Befehle-Menü

- **5 fehlende mitgelieferte Befehle** als eigene JSF-Skripte (eigener Code, `crates\fp-script\src\builtin_commands.rs`, Index ab `ADDED_FROM` = 36):
  | Befehl | Wirkung in Firepixel |
  |---|---|
  | Bildschirmfoto | `fw.takeScreenshot()`; Firepixel kann den Bildschirm nicht aufnehmen, deshalb eine Meldung mit dem Weg über das System (Win+Umschalt+S, dann Datei ▸ Neu und Einfügen) |
  | Demo - aktuelles Dokument | alle Seiten als Vorschau im Browser (`file.previewInBrowser.previewAllPagesInPrimaryBrowser`); ohne Dokument eine Meldung |
  | Symbolskript erstellen | ohne Symbolinstanz die Meldung „Wählen Sie zuerst eine Instanz eines Grafiksymbols aus.“; mit Instanz eine Meldung mit dem Symbolnamen, dass Firepixel keine Symbolskripte anlegt (Verweis auf das Bedienfeld „Symboleigenschaften“) |
  | Kreativ ▸ Autom. Vektormaske | **funktioniert:** legt über die Auswahl ein Rechteck mit linearem Verlauf von deckend zu transparent und gruppiert es als Vektormaske („mask to path“). Ergebnis: ein Verlaufsschritt, Rückgängig stellt exakt her. Ohne Auswahl eine Meldung |
  | Web ▸ Gemeinsame Palette | Meldung: Firepixel legt keine gemeinsame Farbtabelle an; Abhilfe über das Bedienfeld „Optimieren“ |
- **`alert()` aus Befehlsskripten wird jetzt angezeigt:** als Meldung mit dem Titel „Firepixel“ (`crates\fp-ui\src\actions.rs`, nach `commands.runScript`). Vorher landete der Text nur in der Skriptausgabe und war in der Oberfläche unsichtbar. `confirm`, `prompt` und `yesNoDialog` sind davon ausgenommen.
- **Namen wie im Original** (Liste aus PARITAET_B §1). Neu heißen die Befehle:
  - Batch-Befehle: 90°/180°/270°-Drehung, Auswahl um 90/180/270 Grad drehen, Auswahlfarbe umkehren, Auto-Tonwertauswahl
  - Kreativ: Graustufenkonvertierung, Sepia-Konvertierung, Drehen und Ausblenden
  - Dokument: Verteilung auf Ebenen
  - Auswahl: Früheres Vektorrendern verwenden, Verbessertes Vektorrendern verwenden
  - Text: Als Groß-/Kleinbuchstaben, Als Pfade/Pixel wiedergeben, Erster Buchstabe Großbuchstabe, Feste Breite ein/aus, Lorem ipsum
  - oben: Warnmeldungen zurücksetzen
  - Die alphabetische Folge stimmt damit mit dem Original überein.
  - „Feste Breite ein/aus“: Ein „/“ ist in Dateinamen nicht erlaubt. Die Datei heißt deshalb `Feste Breite ein∕aus.jsf` (U+2215), und `menu.rs::scan_commands` zeigt dafür „/“ an.
- **Englische Namen** bleiben wie bisher; es gibt dafür keine Messung. Die neuen Befehle heißen: Screenshot, Demo Current Document, Create Symbol Script, Creative/Auto Vector Mask, Web/Shared Palette.
- **„Warnmeldungen zurücksetzen“ steht nur noch einmal** im Menü, als mitgelieferter Befehl zwischen Text ▸ und Web ▸. Entfernt wurden der feste Eintrag am Menüende (`menu.rs::command_tail`), der Schlüssel `menu-commands-resetWarnings` (de/en) und die Zugriffstaste in `menu_kbd.rs`. Die Aktion `ui.resetWarnings` bleibt für die Voreinstellungen erhalten.
- **Bestehende Befehlsordner der Nutzer** werden wie bei FV-13 behandelt (`update_outdated`, läuft bei jedem Start):
  - Umbenannte Befehle (Tabelle `RENAMED_DE`) bekommen den neuen Namen nur, wenn die Datei unter dem alten Namen wortgleich der mitgelieferten oder einer alten Fassung ist und der neue Name frei ist.
  - Vom Nutzer geänderte Dateien bleiben unter dem alten Namen und bekommen keinen Doppeleintrag.
  - Die 5 neuen Befehle werden in einen schon eingerichteten Ordner **einmal** nachgetragen: Die Markierungsdatei `.firepixel-commands` bekommt dazu die Zeile `set-2`. Gelöschte Befehle kommen nicht wieder.
  - Neue Funktion `install_builtin_commands_in(dir, lang)` (testbar). Die alten Namen zählen in `builtin_commands()` weiter als mitgeliefert, nicht als „gespeicherter Befehl“ (D25).

### 2. Kürzeltexte

- de: „Strg+AKUT“, „Strg+ZIRKUMFLEX“, „Strg+Umschalt+ZIRKUMFLEX“ wie im Original.
- en: „Ctrl+Acute“, „Ctrl+Circumflex“. Gemessen ist nur Deutsch. Für Englisch habe ich den ausgeschriebenen Tastennamen gewählt, passend zu den übrigen englischen Tastennamen (Del, Page Up).
- Umsetzung:
  - neue Schlüssel `key-acute` und `key-circumflex` in `allgemein.ftl` de/en
  - Aliase „´“ und „^“ in `fp-i18n\src\format.rs::KEY_ALIASES`
  - `fp-engine\src\shortcuts.rs::vk_from_name` liest AKUT/ZIRKUMFLEX/Acute/Circumflex wieder (VK 221/220)
- Nicht Teil des Auftrags und weiter offen aus demselben Befund: „Maske deaktivieren“ hat im Original kein Kürzel (Firepixel Strg+Alt+H).

### 3. Filter wiederholen

- Nach einem Filter heißt der Eintrag „Filter wiederholen: <Name des zuletzt benutzten Filters>“, z. B. „Filter wiederholen: Umkehren“. In Englisch ebenso („Repeat Filter: …“).
- Umsetzung: `menu.rs::with_step_name`, Name aus `Session::last_filter`, Text `menu-edit-withStep`.
- Ohne vorherigen Filter bleibt „Filter wiederholen“ (grau).
- Hinweis: Das Original zeigt „Wiederholen: <Filter>...“. Umgesetzt ist der Wortlaut aus dem Auftrag.

### 4. Datei ▸ Bridge durchsuchen → „Im Dateibrowser durchsuchen...“

**Messung:** Der Auftrag sagt „im Original aktiv“. Nach PARITAET_B §1 und BEFUNDE ist der Eintrag im Original in allen 9 Lagen **grau** (○○○○○○○○○), weil das Zusatzprogramm nicht installiert ist. Firepixel hatte ihn immer aktiv.

**Entscheidung: neutraler, funktionierender Eintrag**, de „Im Dateibrowser durchsuchen...“, en „Browse in File Browser...“, gleiches Kürzel Strg+Alt+O, immer aktiv. Die Beschreibung lautet jetzt „Öffnet den Ordner des aktuellen Dokuments im Dateibrowser des Systems“ statt „Mit Bridge durchsuchen“. Begründung:
- „Bridge“ ist hier der Name eines eigenständigen Programms desselben Herstellers, also ein Markenname, auch wenn das Wort allein ein normales englisches Wort ist.
- `fp-neutral` prüft nur die beiden in §1a gesperrten Wörter (per Hash), deshalb fiel „Bridge“ nicht auf und der Test bleibt grün. Der Sinn von §1a (kein Bezug auf Original und Hersteller in der UI) spricht trotzdem für einen neutralen Text.
- Grau wie im Original wäre treu, nähme Firepixel aber eine funktionierende Funktion: Der Befehl listet die Bilddateien des Ordners und öffnet den Ordner im System. Der Eintrag bleibt an derselben Stelle mit demselben Kürzel.
- Die interne Befehls-ID `file.browseInBridge` bleibt (API, MCP, Kürzelsätze, Katalog `commands.json` mit den Messdaten des Originals). Die Ausnahme in `fp-engine\tests\paritaet_menue.rs` trifft den Wortlaut des Originals und bleibt richtig.
- Geändert: `cmd-file-browseInBridge` (de/en) samt `.description`, `crates\fp-ui\data\menu_main.json` („text“).

## Geänderte Dateien

- `crates\fp-script\src\builtin_commands.rs`: Namen, 5 neue Skripte, `ADDED_FROM`, `RENAMED_DE`, Umbenennung in `update_outdated`, `install_builtin_commands_in`, Nachtragen per `set-2`
- `crates\fp-script\src\lib.rs`: alte Namen in `builtin_commands()`
- `crates\fp-ui\src\menu.rs`: `command_tail` ohne Doppeleintrag, „∕“ → „/“, „Filter wiederholen: Name“
- `crates\fp-ui\src\menu_kbd.rs`: Zugriffstaste des entfernten Eintrags
- `crates\fp-ui\src\actions.rs`: Skript-Meldungen anzeigen
- `crates\fp-ui\data\menu_main.json`: Datei-Eintrag
- `crates\fp-engine\src\shortcuts.rs`: AKUT/ZIRKUMFLEX lesbar
- `crates\fp-i18n\src\format.rs`: Aliase
- `crates\fp-i18n\locales\{de,en}\allgemein.ftl`: `key-acute`, `key-circumflex`
- `crates\fp-i18n\locales\{de,en}\menue.ftl`: Datei-Eintrag, `menu-commands-resetWarnings` entfernt
- Tests auf die neuen Namen nachgezogen: `fp-script\tests\befehle.rs`, `builtin_commands_fv13.rs`, `fp-ui\tests\dialogs_dlg4.rs`, `dialogs_warnung.rs`, `menu_fixes_fv13.rs`
- `docs\funktionstest\menue_auto.md` neu erzeugt

## Tests

- neu: `crates\fp-ui\tests\menu_mn4.rs`, 9 Tests:
  - Namen und Reihenfolge des Befehle-Menüs mit allen Untermenüs gegen die Liste des Originals
  - „Warnmeldungen zurücksetzen“ genau einmal, und der Eintrag wirkt
  - alle 5 neuen Befehle ausführbar, mit Meldung bzw. ohne Änderung am Dokument
  - Symbolskript mit Instanz
  - Autom. Vektormaske: Vektormaskengruppe, ein Verlaufsschritt, Rückgängig exakt
  - bestehender Ordner: umbenennen, Nutzerfassung bleibt, neue Befehle einmal, gelöschte kommen nicht wieder
  - Kürzeltexte de/en und Parse
  - „Filter wiederholen: Name“ (zwei Filter nacheinander, Wiederholen ändert den Namen nicht)
  - neutraler Datei-Eintrag
- grün:
  - `menu_mn4::`, `menu_fixes_fv13::`, `dialogs_dlg4::`, `dialogs_warnung::`, `menu_mn2::`, `gesamt_menu::` (mit `FP_NO_SHELL_OPEN=1`)
  - `fp-script --test befehle --test builtin_commands_fv13`
  - `fp-engine --lib shortcuts`, `fp-i18n`, `fp-neutral`, Scanner `i18n_keine_festen_texte::`
  - `cargo check --workspace --all-targets`
- `menue_auto.md`: 347/349 ✅ (vorher 344/345, jetzt 5 Skripte mehr, 1 fester Eintrag weniger). Nicht ✅:
  - „KI-Steuerkanal“: 🔶 Manual, wie bisher
  - „Demo - aktuelles Dokument“: ❌ „keine erkennbare Wirkung“. Der Befehl startet nur den Browser, und das ist im Test abgeschaltet (`FP_NO_SHELL_OPEN`). Der Gesamttest erkennt das bei `commands.runScript` nicht als Fremdprogramm. Nachprüfung im echten Fenster.
- Texte der JSF-Skripte (Meldungen, Eingabeaufforderungen) sind wie bei den bisherigen mitgelieferten Skripten deutsch, auch in einem englisch eingerichteten Befehlsordner. JSF kennt keine Fluent-Schlüssel (siehe neuer Befund).

## Erledigte Befunde (Wortlaut-Anfang)

- BEFUNDE „(von PM-7, PARITAET_B §1) **Kürzel-Anzeige:** Das Original schreibt „Strg+AKUT“ …“: erledigt bis auf „Maske deaktivieren ohne Kürzel“ (bleibt offen)
- „(von PM-7, PARITAET_B §1) **Filter ▸ Filter wiederholen** heißt im Original nach einem Filter …“: erledigt (Wortlaut nach Auftrag: „Filter wiederholen: <Name>“)
- „(von PM-7, PARITAET_B §1, Zuständig auch AU/fp-script …) **Befehle-Menü im frischen Profil:** …“: erledigt (Wortlaut 1:1 wie im Original gewählt)
- „(von PM-7, PARITAET_B §1, offen seit UX-B) **Datei ▸ Bridge durchsuchen** …“: entschieden, neutraler Eintrag, siehe §4 (ℹ️ in PARITAET_B)

## PARITAET_B §1 → „umgesetzt, Nachprüfung ausstehend“

- Befehle > (Liste im frischen Profil), Befehle > Auswahl ▸, Batch-Befehle ▸, Dokument ▸, Kreativ ▸, Text ▸, Web ▸
- Ansicht > QuickInfos; Ansicht > Intelligente Hilfslinien > Intelligente Hilfslinien anzeigen / An intelligenten Hilfslinien ausrichten
- Filter > Wiederholen
- Datei > Bridge durchsuchen → ℹ️ „neutral als ‚Im Dateibrowser durchsuchen...‘, aktiv; Original grau (Zusatzprogramm nicht installiert)“

## Nachprüfung (echtes Fenster bzw. Brücke)

1. Frisches Profil, Befehle-Menü öffnen. Erwartet: die 11 Einträge in der Folge wie oben, „Feste Breite ein/aus“ mit „/“ (Schrift prüfen), „Warnmeldungen zurücksetzen“ nur einmal.
2. Altes Profil mit `.firepixel-commands` ohne `set-2` und alten Namen starten. Erwartet: unveränderte Befehle unter neuen Namen, die 5 neuen Befehle da, keine Doppel.
3. Bildschirmfoto, Gemeinsame Palette und Symbolskript erstellen (ohne Auswahl) anklicken. Erwartet: Meldungsfenster „Firepixel“ mit dem Text, Enter schließt.
4. Rechteck wählen, Kreativ ▸ Autom. Vektormaske. Erwartet: Das Objekt blendet nach rechts weich aus (Darstellung der Verlaufsmaske prüfen), ein Verlaufsschritt.
5. Demo - aktuelles Dokument mit Dokument. Erwartet: Der Browser öffnet die Vorschau aller Seiten.
6. Ansicht-Menü. Erwartet: Strg+AKUT, Strg+ZIRKUMFLEX, Strg+Umschalt+ZIRKUMFLEX; in Englisch Ctrl+Acute usw.
7. Bitmap wählen, Filter ▸ Farbe anpassen ▸ Umkehren. Erwartet: Filter-Menü zeigt „Filter wiederholen: Umkehren“, aktiv, Strg+Alt+Umschalt+X wiederholt.
8. Datei ▸ Im Dateibrowser durchsuchen... (Strg+Alt+O). Erwartet: Der Ordner des Dokuments öffnet sich im Explorer.

## Neue Befunde

- (von MN-4) Die mitgelieferten JSF-Befehle haben feste deutsche Meldungen und Eingabeaufforderungen, auch bei englischer Installation. Vorschlag: eigene Texte je Sprache beim Installieren (wie die Dateinamen) oder eine Funktion `fp.t(key)` für Skripte. Zuständig AU/fp-script.
- (von MN-4) Gesamttest (`gesamt_common::judge`): `commands.runScript`-Skripte, die nur ein Fremdprogramm starten (Demo - aktuelles Dokument), gelten als „keine erkennbare Wirkung“. Vorschlag: Fremdprogramm-Start über `shell` im Ergebnis melden.
- (von MN-4) Bildschirmfoto/Symbolskript/Gemeinsame Palette zeigen nur Meldungen. Echte Funktion wäre: Bildschirmaufnahme (z. B. per `xcap`-Crate), Symbolskripte für Grafiksymbole, gemeinsame Farbtabelle über alle Segmente beim Export.

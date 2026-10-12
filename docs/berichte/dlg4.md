# Bericht DLG-4 (Paket P-DLG4, 12.10.2026)

Slot s7. Daten: PARITAET_B §2 D14/D16/D17/D25, BEFUNDE (IBD „von PM-5“), Bilder `pm5_D14/D16/D17/D25*`. Das Original wurde nicht benutzt.

## Umgesetzt

Commit: siehe `git log --grep "DLG-4"` (ein Commit, Präfix „DLG-4:“).

- **D14 Bearbeiten ▸ Objekt einfügen ▸ Neues Symbol**
  - Der Dialog ist jetzt wie „In Symbol konvertieren“ aufgebaut: Titel „In Symbol konvertieren“, Name „Symbol“, Typ Grafik/Animation/Schaltfläche und „Hilfslinien für 9-Slice-Skalierung aktivieren“.
  - Bei OK legt Firepixel ein **leeres Symbol** in der Dokumentbibliothek an und öffnet dessen **Symbolbearbeitung**. Das ist ein Verlaufsschritt im Dokument. Es entsteht keine Instanz mehr; früher landete eine leere Instanz „Symbol“ bei 0,0.
  - Engine: `edit.insert.newSymbol` hat die neuen Parameter `kind` (graphic/animation/button), `edit` (Vorgabe false) und `nineSlice`. Ohne `edit` verhält sich der Befehl wie bisher; JSF `addNewSymbol` und die bestehenden Tests bleiben unverändert.
  - In `panel.rs` ist nur `set_nine_slice` jetzt `pub(super)`.
- **D25 Befehle ▸ Gespeicherte Befehle verwalten**
  - Die mitgelieferten Skripte des Befehle-Menüs auf oberster Ebene zählen nicht mehr als gespeicherte Befehle. Das betrifft „Größe ausgewählter Objekte ändern“ und „Warndialoge zurücksetzen“, jeweils auf de und en.
  - Neue Trait-Methode `ScriptHost::builtin_commands()` (Vorgabe leer), umgesetzt in `fp_script::JsfHost`.
  - Neue Funktion `fp_engine::commands::misc::user_saved_commands`. `commands.manageSavedCommands` (list/save/rename/delete) liefert nur noch eigene Befehle, auch über MCP.
  - Ohne eigene Befehle erscheint nur eine Meldung mit dem Titel „Firepixel“: „Befehle werden dem Verlauf-Bedienfeld entnommen. Zurzeit sind dort keine gespeicherten Befehle vorhanden.“ Auf Englisch: „Commands are taken from the History panel. There are currently no saved commands.“ Enter und Esc schließen sie.
  - Mit eigenen Befehlen erscheint die Verwaltungsliste wie bisher, aber nur mit diesen Befehlen.
- **D16 Voreinstellungen**
  - Kategorien wie im Original: „Bearbeitung“, „Photoshop-Datei importieren“, „Starten und Bearbeiten“ vor „Zusatzmodule“. Englisch bleibt bei den Namen des englischen Originals (Edit, Photoshop Import/Open, Launch and Edit, Plug-ins).
  - Die Seite „Allgemein“ ist in Gruppen geteilt, mit Überschrift und Trennlinie; die Beschriftungen stehen rechtsbündig vor der Steuerspalte:
    - „Dokumentoptionen:“ mit Startseite und Pinselstrichen/Effekten.
    - „Maximale Anzahl Rückgängig-Schritte:“ mit dem Hinweis **„Änderungen werden erst nach Neustart aktiv“**.
    - Interpolation und schnellere Neuauflösung.
    - **Speichern**: Miniaturen, .fw.png.
    - **Benutzeroberfläche**: „Arbeitsbereich:“ Bedienfeldsymbole, dazu von Firepixel Design, Sprache und Entwickleroptionen.
    - **Standardfarben**: Farbfelder mit Beschriftung rechts (Pinselstrich, Füllung, Hervorhebung) und „Alle Warnmeldungen zurücksetzen“.
  - Neuer Knopf **„Hilfe“** unten links. Er öffnet `help.firepixelHelp`, der Dialog bleibt offen.
  - Die Kategorieliste und der Tab-Stopp von DLG-1b sind unverändert.
- **D17 Tastaturkurzbefehle**
  - Die Menübefehle stehen als **Menübaum** mit ⊞/⊟. Die Hauptmenüs folgen der Menüreihenfolge (Datei, Bearbeiten, Ansicht …), Untermenüs lassen sich aufklappen. Mit Suchtext und unter „Werkzeuge“ bleibt die flache Liste.
  - Neu ist die Zeile **„Beschreibung:“** unter dem Baum. Sie zeigt den Statuszeilentext des gewählten Befehls.
  - Die Baumknoten sind für die Barrierefreiheit Schaltflächen mit Namen, die Befehle sind wählbare Zeilen.
- Texte de/en in `dialoge.ftl`:
  - neu: `dlg-md-no-saved-info`, `dlg-md-app-title`, `dlg-pref-grp-*`, `dlg-pref-restart-note`, `dlg-pref-color-*`, `dlg-pref-help`, `dlg-sc-description`
  - geändert, nur de: `dlg-pref-cat-edit`, `dlg-pref-cat-psimport`
  - geändert, de und en: `dlg-pref-maxundos`

## Tests

- Neue Datei `crates\fp-ui\tests\dialogs_dlg4.rs` mit 5 Tests:
  - D14: OK öffnet die Symbolbearbeitung, der Typ wird per Klick gesetzt, keine Instanz, Rückgängig.
  - D14: Engine ohne und mit `edit`.
  - D25: Meldung ohne eigene Befehle, Liste nur mit eigenen, Engine-Liste ohne mitgelieferte Skripte.
  - D16: Kategorien de/en, Gruppen, Kästchen bedienbar, Hilfe lässt den Dialog offen.
  - D17: zugeklappt nur Hauptmenüs in Menüreihenfolge, Klick klappt auf, Beschreibung, Suche flach.
- Grün:
  - `dialogs_dlg4::`, `dialogs_dlg1b::`, `dialogs_open::` und `i18n_keine_festen_texte::` (der Scanner)
  - `cargo test -p fp-i18n -p fp-neutral`
  - fp-engine `commands::`
  - fp-ui lib `preferences`
  - `cargo check --workspace --all-targets`

## Befunde erledigt (Wortlaut-Anfang)

- „(von PM-5, PARITAET_B D14) **Neues Symbol:** Dialog wie „In Symbol konvertieren“ …“: erledigt. Ausnahme: „In allgemeiner Bibliothek speichern“ (im Original grau) fehlt, siehe offen.
- „(von PM-5, PARITAET_B D25) **Gespeicherte Befehle verwalten:** listet die mitgelieferten …“: erledigt. Das Info-Symbol der Meldung fehlt, weil die gemeinsame Meldung keins hat.
- „(von PM-5, PARITAET_B D21/D16, klein) …“, Teil Voreinstellungen: Kategorienamen und -reihenfolge, Gruppen, Neustart-Hinweis und „Hilfe“ erledigt.
- UX-B-Eintrag „Kleinere Abweichungen“: „D16 Voreinstellungen: Knopf „Hilfe“ fehlt; Felder gruppieren“ erledigt, „D17 Tastaturkurzbefehle: Menübaum statt flacher Liste“ erledigt.

## PARITAET_B auf „umgesetzt, Nachprüfung ausstehend“

- **D14**: Dialog mit Typ/9-Slice, OK öffnet die Symbolbearbeitung des leeren Symbols.
- **D16**: Kategorien, Gruppen, Neustart-Hinweis, Hilfe. Firepixel hat zusätzlich Design, Sprache und Entwickleroptionen unter „Benutzeroberfläche“.
- **D17**: Menübaum und Beschreibung. Symbolknöpfe statt Textknöpfen und die Fokusanzeige per Tab sind offen.
- **D25**: Meldung ohne eigene Befehle.

## Nachprüfung (echtes Fenster/Brücke)

1. Bearbeiten ▸ Objekt einfügen ▸ Neues Symbol…
   - Ablauf: Typ „Animation“ wählen, Enter.
   - Erwartet: Titel „In Symbol konvertieren“. Danach ist die Symbolbearbeitung „Symbol (Symbol)“ aktiv und im Dokument entsteht keine Instanz. Nach „Fertig“ steht das Symbol in der Bibliothek, Strg+Z entfernt es.
2. Befehle ▸ Gespeicherte Befehle verwalten… mit frischem Profil.
   - Erwartet: nur die Meldung mit Titel „Firepixel“ und dem Wortlaut oben.
   - Danach im Verlauf „Als Befehl speichern“ und erneut öffnen. Erwartet: Liste nur mit diesem Befehl.
3. Bearbeiten ▸ Voreinstellungen…
   - Erwartet: die Reihenfolge der Kategorien wie oben, die Gruppen auf „Allgemein“ mit Hinweis und „Hilfe“ unten links.
   - In en/qps darf nichts überlaufen. Die Höhe ist etwa 480 bis 520 px, weil Design, Sprache und Entwickleroptionen dazukommen.
4. Bearbeiten ▸ Tastaturkurzbefehle…
   - Erwartet: ein Baum mit Datei … Hilfe. Klick auf ⊞ klappt auf, ein gewählter Befehl zeigt „Beschreibung: …“, ein Suchtext zeigt die flache Liste.

## Neue Befunde / offen

- D14: „In allgemeiner Bibliothek speichern“ fehlt; im Original ist es grau. Dafür bräuchte `form.rs` ein gesperrtes Kästchen (Field ohne Freigabe).
- D14: „Neue Schaltfläche“ fügt weiter sofort eine leere Instanz ein. Im Original öffnet sie vermutlich ebenfalls die Bearbeitung; das ist nicht gemessen.
- D25: Die gemeinsame Meldung (`dialogs::show_message`) hat kein Info-Symbol.
- D16: `btn-help` fehlt in `allgemein.ftl` (I18N-KERN). Vorläufig dient `dlg-pref-help` (ohne Zugriffstaste &H).
- D17 Rest:
  - Symbolknöpfe (4) statt Textknöpfen beim Satz
  - Fokusanzeige und Pfeiltasten im Baum
  - Liste „Befehle:“ als Kombifeld mit Beschriftung

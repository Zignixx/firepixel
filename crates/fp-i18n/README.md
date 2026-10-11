# fp-i18n – Mehrsprachigkeit (Fluent)

## Für Übersetzer: eine neue Sprache hinzufügen
1. `locales\en\` (oder `de\`) als Vorlage nehmen, alle `*.ftl` in einen Ordner `<code>` kopieren (z. B. `fr`, `pt-BR`).
2. Rechts vom `=` übersetzen, Schlüssel links und Platzhalter `{ $name }` unverändert lassen. `&` markiert die Zugriffstaste (Alt+Buchstabe).
3. `language-name = Français` setzen.
4. Ordner nach `%APPDATA%\Firepixel\Sprachen\` (oder `Sprachen\` neben der exe) legen → erscheint in Ansicht ▸ Sprache und in den Voreinstellungen. Eingebaut wird eine Sprache, indem der Ordner unter `crates\fp-i18n\locales\` liegt (neu bauen, keine Codeänderung).
5. Prüfen: `firepixel-cli sprache pruefen <ordner|datei>` (fehlende/verwaiste Schlüssel, Platzhalter, Syntax).

## Für Entwickler
- Jeder für Nutzer sichtbare Text kommt aus `fp_i18n::t("schluessel")` bzw. `fp_i18n::tr!("schluessel", name = wert)`. **Keine festen Texte in UI-Code.** Ausnahmen: Eigennamen (Firepixel, Dateiformate wie PNG/WebP), Einheiten-Kürzel (px, %), reine Symbole.
- Texte werden in jedem Frame neu geholt (ein Cache macht das billig); Sprachwechsel wirken sofort. Wer Texte selbst zwischenspeichert, koppelt den Cache an `fp_i18n::generation()`.
- Menüs/Befehle: Schlüssel = `fp_i18n::command_key(name)` (`file.open` → `cmd-file-open`), Attribute `.tooltip`, `.description` optional.
- Neuer Text = Schlüssel in **`de` und `en` gleichzeitig** anlegen (der Test `builtin_files_are_complete_and_clean` prüft Vollständigkeit). Englische Begriffe nach `GLOSSAR.md`.

## Schlüssel und Bereichsdateien
Schlüssel: Kleinbuchstaben, Bindestriche, Bereichspräfix. Attribute mit Punkt (`key.tooltip`). Fluent-Syntax: <https://projectfluent.org/fluent/guide/>.

| Datei (`locales\<code>\…`) | Präfixe | zuständig (Phase 6) |
|---|---|---|
| `allgemein.ftl` | `language-name`, `btn-`, `fmt-`, `unit-`, `key-` (Tastennamen), `test-` | I18N-KERN |
| `menue.ftl` | `menu-` (Menütitel, Untermenüs), `cmd-` (alle Befehle aus dem Register inkl. Tooltip), `ctx-` (Kontextmenüs), `pmenu-` (Bedienfeldmenüs) | I18N-MENUE |
| `werkzeuge.ftl` | `tool-` (Werkzeugnamen, Tooltips, Flyouts, Statushinweise), `shape-` (automatische Formen) | I18N-MENUE |
| `dialoge.ftl` | `dlg-` | I18N-DIALOGE |
| `inspektor.ftl` | `pi-` | I18N-INSPEKTOR |
| `bedienfelder.ftl` | `panel-` | I18N-BEDIENFELDER |
| `oberflaeche.ftl` | `ui-` (Anwendungsleiste, Symbolleiste, Reiter, Leinwand-Overlays, Statuszeile, Fußleiste, Docking, Datei-Dialogfilter, Startfenster) | I18N-SHELL |
| `meldungen.ftl` | `err-` (Fehler aus fp-engine/fp-io/fp-export/fp-script), `msg-` (Status-/Infomeldungen), `ai-` (KI-Auswahl) | I18N-ENGINE |
| `filter.ftl` | `filter-`, `effect-`, `blend-` (Filter-, Effekt- und Mischmodusnamen, Parameter) | I18N-ENGINE |

Jeder Agent schreibt nur in seine Dateien (in `de` **und** `en`).

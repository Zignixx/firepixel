# CMD-TXT – fehlende Parameterbeschreibungen in command_texts.ftl (13.10.2026)

## Umgesetzt
- `crates\fp-i18n\locales\{de,en}\command_texts.ftl`: 18 neue `cmdtext-`-Schlüssel (19 Texte; „Index (Vorgabe: aktueller)“ teilen sich `panel.states.delete` und `panel.pages.delete`). Vorhandene Texte unverändert.
  - `file.open` (Unbenannt, Als Animation öffnen), `edit.copyHtmlCode` (Bilderordner), `element.setPattern`, `element.setTexture`, `edit.insert.newSymbol` (2), `edit.insert.newButton`, `select.saveBitmapSelection` (3), `select.restoreBitmapSelection` (2), `select.deleteBitmapSelection`, `bitmap.gradient` (2), `pi.fill`, `panel.states.delete`, `panel.pages.delete`.
  - Englisch nach Glossar und Nachbartexten (Untitled, Open as Animation, symbol editing, 9-slice scaling, Default, Operation, Color tolerance).
- Der Test prüft alle Befehle im Register (Engine + JSF-Laufzeit); danach fehlen keine weiteren Schlüssel derselben Art.

## Erledigte Befunde
- „(von TE2-C, Abschluss TE2-G, für Welle 5/UE) **firepixel-cli Unit-Test `tests::parameterbeschreibungen_in_de_und_en` rot**“ – abgehakt.

## Geprüft
- `cargo test -p firepixel-cli` grün (inkl. `parameterbeschreibungen_in_de_und_en`), `cargo test -p fp-i18n` grün, Scanner `cargo test -p fp-ui --test all i18n_keine_festen_texte::` grün, `cargo test -p fp-neutral` grün, `cargo check --workspace --all-targets` ohne Fehler.

## PARITAET / Nachprüfung
- Keine Zeilen; nur MCP-/CLI-Schematexte, keine Oberfläche.

## Neue Befunde
- Keine.

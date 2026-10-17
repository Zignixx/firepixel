# fp-i18n – Localisation (Fluent)

Firepixel is multilingual: **German** (the source language) and **English** are built in; every
additional language is a folder of text files and needs **no code change**. Switch languages in
**View ▸ Language** (or in the Preferences), immediately and without a restart.

German is the source language: all texts are written in German first, and the English texts follow
the binding German → English glossary [`GLOSSAR.md`](GLOSSAR.md) (the file name stays as is; a test
parses it).

## For translators: adding a new language

### 1. Create a template
```
firepixel-cli language template fr "%APPDATA%\Firepixel\Languages"
```
creates `%APPDATA%\Firepixel\Languages\fr\*.ftl` – a copy of the English texts (`--from de`
uses German as the template). Language codes follow BCP 47: `fr`, `es`, `it`, `pt-BR`, `zh-TW`, `ja` …
Without the command line: copy the folder `crates\fp-i18n\locales\en\` and rename it.

### 2. Translate
Every line is `key = Text`. Translate only **to the right of the `=`**:

```ftl
# common.ftl
language-name = Français              # the language's own name, as it appears in the menu
btn-cancel = Annuler
btn-yes = &Oui                        # & before a letter = access key (Alt+O)
key-ctrl = Ctrl                       # key names in shortcuts (Strg+S → Ctrl+S)
fmt-decimal-separator = ,             # decimal separator: 12,5 %
fmt-date = { $day }/{ $month }/{ $year }

# dialogs.ftl – keep placeholders { $name } unchanged, position is free
dlg-unsaved-text = Enregistrer les modifications de « { $name } » ?

# Plurals: variants follow the language's rules (one, few, many, other …), *[other] is mandatory
test-layers = { $n ->
    [one] { $n } calque
   *[other] { $n } calques
}
```

Rules:
- Do **not** translate keys on the left, placeholders `{ $name }` or functions such as `{ NUMBER($z, maximumFractionDigits: 1) }`.
- `&` marks the access key; `&&` is a literal `&`. Do not assign the same key twice within one menu or dialog (the check reports it).
- Multi-line texts: indent the continuation lines. Comments start with `#`.
- If a key is missing, Firepixel shows the English text (then the German one). So you can translate step by step.
- Terminology: [`GLOSSAR.md`](GLOSSAR.md) (German → English, binding for the built-in languages; a good orientation for other languages).
- Save files as UTF-8.

### 3. Check
```
firepixel-cli language check "%APPDATA%\Firepixel\Languages\fr"
```
reports per file:line **errors** (syntax, duplicate keys, placeholders unknown to the program) and
**warnings** (missing/orphaned keys, missing placeholders or access keys, duplicate access keys
in a menu/dialog) plus the translated share. `--short` only counts missing keys, `--json`
returns everything machine-readable, `--strict` treats warnings as errors.
`firepixel-cli language list` shows all languages with their completeness.

### 4. Use
The folder is loaded at startup from
- `%APPDATA%\Firepixel\Languages\` (per user; **View ▸ Language ▸ Open Language Folder...** opens it and reloads changes immediately) and
- `Languages\` next to `firepixel.exe` (portable installation).

Old `Sprachen` folders (until 12.10.2026) are renamed to `Languages` once at startup or, if that
is not possible, still read (`fp_i18n::paths`). The old commands `sprache pruefen|vorlage|liste`
and options `--eingebaut`, `--kurz`, `--streng`, `--ueberschreiben`, `--von` keep working.

Until 13.10.2026 the area files had German names (`allgemein.ftl`, `menue.ftl` …; list
`check::LEGACY_AREA_NAMES`, new names in the table below). Language packs with the old names
still load (every `*.ftl` is read); `language check` maps them to the new area, and
`language template` creates the new names (an existing file under an old name counts as
present, `--overwrite` replaces it under the new name).

Allowed are one subfolder per language (`fr\*.ftl`) or a single file `fr.ftl`. Identical keys
override built-in texts – this way individual German or English texts can be adjusted too
(`de\custom.ftl`).

**Building in permanently** (for developers): put the folder into `crates\fp-i18n\locales\<code>\`
and rebuild. `build.rs` embeds all files; the exe stays a single file.

### Test language
`qps` (in the debug build, or with the preference `DeveloperOptions` under View ▸ Language) shows
every text marked and lengthened by about 40 %: `[!! Cancel ····!!]`. Untranslated, hard-coded
texts stand out immediately, and layouts are checked with long texts.

### Fonts and writing direction
- Latin, Cyrillic, Greek: Segoe UI (checked in the test `i18n_sprachwahl`).
- Chinese, Japanese, Korean: the system fonts Microsoft YaHei, Yu Gothic, Malgun Gothic as fallback (the one for the current language first), loaded at runtime.
- **Right-to-left (Arabic, Hebrew, Persian, Urdu): not supported.** egui lays out text left→right only and without glyph shaping (no bidi reordering, no Arabic joining forms, no mirrored layout). Such language files can be loaded and appear in the menu marked "(RTL)", but are barely readable.

### Command line and MCP
- `firepixel-cli` speaks `--lang <code>`, otherwise `FIREPIXEL_LANG`, otherwise the system language; the help comes entirely from `cli.ftl`.
- The MCP server answers AI agents in `FIREPIXEL_MCP_LANG` (default English, per call `language`), via `fp_i18n::with_language` only in the calling thread; the user interface keeps its language.

## For developers
- Every user-visible text comes from `fp_i18n::t("key")` or `fp_i18n::tr!("key", name = value)`. **No hard-coded texts in UI code.** Exceptions: proper names (Firepixel, file formats such as PNG/WebP), unit abbreviations (px, %), pure symbols.
- Texts are fetched anew every frame; language switches take effect immediately. Anyone caching texts themselves ties the cache to `fp_i18n::generation()`.
- Menus/commands: key = `fp_i18n::command_key(name)` (`file.open` → `cmd-file-open`), attributes `.tooltip`, `.description` optional.
- New text = add the key in **`de` and `en` at the same time** (tests `builtin_files_are_complete_and_clean`, `builtin_placeholders_and_access_keys`). Write German first; English terms according to `GLOSSAR.md`.
- Pass numbers **as numbers** (`tr!("x", n = 12.5)`): the language's decimal separator, at most 3 decimal places, plural selection per CLDR. Digits in the .ftl: `{ NUMBER($n, minimumFractionDigits: 1, maximumFractionDigits: 2) }`. No thousands separator.

### Helpers (all follow the current language)
| Function | Purpose |
|---|---|
| `t(key)` / `tr!(key, a = x)` / `t_args` | text, text with placeholders |
| `t_arc(key)` | like `t`, without a new string (`Arc<str>`) |
| `strip_access("&Datei")` → `("Datei", Some('d'))`; `t_access(key)` | access key and display text (`AccessText { text, key, index }`, `index` = character index to underline) |
| `shortcut_label("Strg+Umschalt+S")` | shortcut in the current language (accepts German or English spelling; key names `key-*`) |
| `unit("px")`, `unit("ppi")` | units (`unit-*`) |
| `fmt_decimal(v, n)`, `fmt_number(v, min, max)` | numbers with decimal separator |
| `fmt_size(bytes)` | "1,5 MB" / "1.5 MB" |
| `fmt_date(y, m, d)`, `fmt_time(h, m)`, `fmt_datetime(…)` | date/time (`fmt-date` …) |
| `languages()`, `set_language(code)`, `current_language()`, `system_language()` | languages |
| `load_external_dir(dir)`, `load_default_dirs()`, `user_language_dir()` | language folders |
| `check::check_builtin()`, `check::check_path(p)` | checks (tests, CLI) |

User interface: `fp_ui::language` (submenu, `switch`, `init` at startup, `pref_row` for the
Preferences, preference keys `Language` and `DeveloperOptions`).

### Performance
`t()` runs thousands of times per frame. Every thread has its own read cache (key → text),
tied to an atomic generation counter: in the normal case **no lock and no Fluent**,
just a hash lookup (FxHash). Texts with placeholders are cached per thread together with their
values (at most 4096 entries). Measurement (`cargo test -p fp-i18n --release --test
leistung -- --ignored --nocapture`, 12.10.2026, machine loaded by parallel builds; minimum of 5 runs):

| Call | before (RwLock + map) | now |
|---|---|---|
| `t()` (incl. new `String`) | 110 ns | 80–110 ns (the `String` allocation dominates; egui needs it anyway) |
| lookup alone (`t_arc`) | – | 21 ns |
| `has()` | no cache (Fluent per call) | 20–30 ns |
| `tr!` with recurring number | 1.6 µs | 0.2 µs |
| `tr!` with text placeholder | 0.46 µs | 0.14 µs |
| 8 threads at once, `t()` | 0.9–1.1 µs | 0.45 µs |
| frame with 5000 texts | – | ≈ 0.45 ms |
| language switch + first frame | – | ≈ 0.6 ms |

### Checks (permanent)
- `fp-i18n`: completeness de/en, no orphaned keys, syntax, same placeholders per key, no new access-key conflicts in English.
- `fp-ui\tests\i18n_keine_festen_texte.rs`: source scanner for hard-coded texts across the whole workspace (UI calls, German literals, error messages of all libraries, output of the command line and MCP server) with the exception list `i18n_ausnahmen.txt`; reports `file:line` and the number of hard-coded texts left; enforced (`keine_festen_texte` must yield 0). Report: `cargo test -p fp-ui --test i18n_keine_festen_texte -- --ignored --nocapture` (filter: `$env:FP_I18N_FILTER = 'menu.rs'`).
- `fp-ui\tests\i18n_sprachwahl.rs`: menu, switching without restart, startup with saved/system language, language folders, font coverage.

## Keys and area files
Keys: lowercase letters, hyphens, area prefix. Attributes with a dot (`key.tooltip`). Fluent syntax: <https://projectfluent.org/fluent/guide/>.

| File (`locales\<code>\…`) | Prefixes |
|---|---|
| `common.ftl` | `language-name`, `btn-`, `fmt-`, `unit-`, `key-` (key names), `lang-` (language selection), `test-` |
| `menu.ftl` | `menu-` (menu titles, submenus), `cmd-` (all commands from the registry incl. tooltip), `ctx-` (context menus), `pmenu-` (panel menus) |
| `tools.ftl` | `tool-` (tool names, tooltips, flyouts, status hints), `shape-` (auto shapes) |
| `dialogs.ftl` | `dlg-` |
| `inspector.ftl` | `pi-` |
| `panels.ftl` | `panel-` |
| `ui.ftl` | `ui-` (application bar, toolbar, tabs, canvas overlays, status bar, footer, docking, file dialog filters, start window) |
| `messages.ftl` | `err-` (errors from fp-engine/fp-io/fp-export/fp-script), `msg-` (status/info messages), `ai-` (AI selection) |
| `filter.ftl` | `filter-`, `effect-`, `blend-` (filter, effect and blend mode names, parameters) |
| `welcome.ftl` | `welcome-` (start screen) |
| `script.ftl` | `err-script-` (messages from fp-script: auto shapes, files) |
| `mcp.ftl` | `msg-mcp-`, `err-mcp-` (replies of the MCP server and the bridge to AI agents), `mcp-tool-` (tool titles, `.description`, parameters), `mcp-res-` (resources), `mcp-doc-arg` |
| `cli.ftl` | `cli-` (help of `firepixel-cli`: `cli-<command>`, `.examples`, `cli-<command>-arg-<option>`, `.value`; messages; `cli-lang-*`, `cli-check-*` language check) |
| `command_texts.ftl` | `cmdtext-` (parameter descriptions and hints of the commands for MCP/CLI; keys derived from the German source text, `fp_engine::command::source_text_key`) |
| `file_dialog.ftl` | `fdlg-` (in-app file dialog while the AI control channel is active) |
| `image_preview.ftl` | `image-preview-` (image preview, File ▸ Image Preview...; the labels of the left column are in `dialogs.ftl` under `dlg-preview-`) |
| `trace.ftl` | `trace-` (trace bitmap; the command label is in `menu.ftl` under `cmd-modify-traceBitmap`) |
| `bitmap_selection.ftl` | `bsel-`, `sel-saved-`, `err-sel-`, `msg-step-…-selection` (save/restore/delete bitmap selection) |

Every new or changed key goes into the matching area file in `de` **and** `en`.

Groups for the access-key check (`check::access_group`): `menu-…`/`cmd-…` by menu path
(key without prefix up to the last `-`: `cmd-modify-align-left` and `menu-modify-align` belong
to `modify-align` and `modify` respectively), `dlg-<dialog>-…` per dialog, `btn-…` shared,
otherwise up to the last `-`.

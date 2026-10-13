# FIX-R2 – `toSource()`/`uneval()` der Skript-Engine wie im Original

Grundlage: `docs\berichte\fixg3.md`, Abschnitt „Neue Befunde“ 1–3, Belege aus dem Paarlauf
`re\_testdateien\par\run_lauf1_2026-10-13\*\log.jsonl` (Werte `orig` gegen `fp`, per Skript ausgewertet).

## Umgesetzt

`toSource()` unterscheidet jetzt **Wirtwerte** (vom Programm geliefert) und **reine JS-Werte**:

| Fall | Original (Beleg) | Firepixel vorher | jetzt |
|---|---|---|---|
| JS-Array mit `undefined` | `[800, 600, , 2, 1]` (dialogs-d01-d10/02, menu-file/01) | `[ 800, 600, null, 2, 1 ]` | wie Original |
| JS-Array nur `undefined` | `[, ]`, `[, , ]` (menu-file/02, /05) | `[ null, null ]` | wie Original (einfaches Verbinden mit `, `, kein Schlusskomma) |
| verschachtelte JS-Arrays | `[[0, "L1", …]]` (dialogs-d01-d10/11) | `[ [ 0, "L1", … ] ]` | wie Original |
| Wirt-Array | `hGuides:[ 300 ]`, leer `vGuides:[  ]` (xs-canvas/09) | leer `[]` | wie Original |
| Wirtobjekt | `{ x:20, y:30 }` (dialogs-d11-d20/15) | gleich | unverändert |
| JS-Objektliteral | SpiderMonkey `({a:1})`, verschachtelt `{a:1}` | `{ a:1 }` (FIX-G3) | `({a:1, b:[1, 2]})` |
| Wagenrücklauf in Wirt-Zeichenkette | `characters:"Zeile eins\u000DZeile zwei…"` (menu-text/15) | `\r` | `\u000D` |

Technik (`crates\fp-script`):
- `conv.rs`: neue nicht aufzählbare Markierung `__fpHost` (`HOST_KEY`, `mark_host`, `is_host`) für alle
  Objekte/Arrays aus `json_to_js`, `conv::object`, `conv::array`; FwArray zählt ebenfalls als Wirtwert.
  `js_to_json` überspringt die Markierung (Rundlauf `setTextRuns(textRuns)` unverändert). Für `for…in`,
  `Object.keys` und `JSON.stringify` unsichtbar (Test).
- `imp\mod.rs`: neuer Serialisierer `to_source_js` direkt auf JS-Werten (statt über JSON, das
  `undefined`/Herkunft verliert). Wirtform vererbt sich nach innen; ein Wirtobjekt in einem JS-Array bleibt
  Wirtform (`[1, { x:20, y:30 }]`).
- Zeichenketten: Wirtform maskiert alle Steuerzeichen < 0x20 und 0x7F als `\uXXXX` (Hex groß, belegt nur
  für CR; übrige nach derselben Regel). Reine JS-Zeichenketten wie SpiderMonkey 1.5 (`js_QuoteString`):
  `\b \f \n \r \t \v`, übrige Steuerzeichen `\xHH`.
- Schlüssel: Bezeichner ohne Anführungszeichen; bei JS-Objekten auch Indizes (`{0:2}`).

Commit: siehe `git log --grep FIX-R2` auf main.

## Tests

- Neu: `crates\fp-script\tests\fixr2_to_source.rs` (10 Tests, je Regel: JS-Arrays/Lücken,
  verschachtelte Arrays, Wirt-Arrays `[ 300 ]`/`[  ]`, `guides`-Form, JS-Objektliterale mit Klammern,
  Wirtobjekte ohne, Markierung unsichtbar, `\u000D` in `textRuns`, übrige Steuerzeichen Wirtform,
  JS-Zeichenketten SpiderMonkey-Form).
- Geändert (Erwartung wich nachweislich vom Original ab): `tw02_textruns.rs`
  `to_source_objects_without_parentheses` (JS-Literale jetzt `({…})`, Wirtobjekt `gridSize` ohne) und
  `to_source_arrays_and_strings_unchanged` (`[1, "zwei", {x:3}]`).
- `cargo test -p fp-script -p fp-mcp` grün, `cargo check --workspace --all-targets` ohne Warnungen.

## Befunde erledigt

- fixg3.md „Neue Befunde“ 1 „**Reine JS-Arrays** gibt das Original ohne Innenabstand …“ – erledigt.
- fixg3.md „Neue Befunde“ 2 „**Reine JS-Objektliterale** hätten im Original …“ – erledigt.
- fixg3.md „Neue Befunde“ 3 „**Wagenrücklauf in Zeichenketten** …“ – erledigt.

## PARITAET

- Skript-API `toSource`/`uneval` (Wirt- und JS-Form, Lücken, Steuerzeichen): „umgesetzt, Nachprüfung ausstehend“.

## Nachprüfung

- Paarlauf erneut, Spalte `values`: `dialogs-d01-d10` 02/04/11, `menu-file` 02/05/08, `menu-edit` 03/06,
  `xs-canvas` 09, `menu-text` 15 – Form von `toSource()` zeichengleich mit `orig`, abgesehen von Werten, die
  an anderen Befunden hängen (siehe unten).

## Neue Befunde

1. `d.zoom` ist im Original `undefined` (Ausgabe als Lücke), Firepixel liefert `null` → `[800, 600, null, 2, 1]`
   bleibt bis dahin abweichend (Eigenschaft `zoom` des Dokuments, nicht Teil dieses Auftrags).
2. `docTitle` ist im Original `undefined` (`[, ]`), Firepixel `null` → `[null, null]`.
3. `pathAttributes.brush.diameter` ist im Original `undefined` (menu-edit/06 `["#808080", , 100]`), Firepixel `1`.
4. Nicht belegt, daher nicht geändert: Nicht-ASCII-Zeichen in JS-Zeichenketten (SpiderMonkey 1.5 würde
   `\xE4` ausgeben), `undefined` in Wirt-Arrays (bleibt `null`), leeres Wirtobjekt (jetzt `{  }` analog `[  ]`),
   `toSource` von Primitiven (`(new Number(5))` im Original vermutet).

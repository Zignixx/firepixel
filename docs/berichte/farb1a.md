# FARB-1a: Genauer Farbwähler springt zurück (Nutzerbefund N27a)

## Ursache
`crates\fp-ui\src\inspector\popups.rs`, `color_picker_ui`: Der genaue Farbwähler (egui-Farbwähler im
Farbfelder-Popup, Knopf oben rechts) bekam **jeden Frame** die Ausgangsfarbe `current` als Wert
(`let mut rgb = current…`). Die gezogene Farbe wurde zwar unter `syscol` gemerkt, aber nie wieder
angezeigt. Folge: Farbfeld, Markierung, R/G/B und die Vorschau oben sprangen nach jedem Frame auf das
Ausgangsgrau zurück; die Farbtonleiste wirkte auf Grau (Sättigung 0) gar nicht. Erst „Übernehmen“
schrieb den gemerkten Wert.

Alle Nutzer laufen über diese eine Funktion (`color_button` → `color_popup` → `color_picker_ui`):
Effekt-Dialoge (`dialogs\form.rs`), Inspektor Füllung/Strich/Text/Werkzeuge (`inspector\obj_quads.rs`,
`quads.rs`, `tool_quads.rs`), Bedienfelder (`panels\misc.rs`, `optimize.rs`, `autoshapes.rs`),
Dialoge (`catalog.rs`, `popup_menu.rs`, `preferences.rs`, `image_preview\options.rs`). Nicht betroffen:
Farbtöpfe der Werkzeugleiste (`tools_panel.rs`, schreiben sofort per `SetColors`, lesen den neuen Wert
im nächsten Frame) und der Bildvorschau-Dialog „Farbe“ (`color_table.rs`, eigener Zustand).

## Behebung (zentral, keine Umgestaltung)
- Wähler startet einmal bei `current`, danach gilt die gemerkte Arbeitsfarbe (`syscol`).
- Vorschaufeld und Hex-Feld oben zeigen die Arbeitsfarbe.
- „Übernehmen“ liefert sie und löscht sie; Umschalten des Wählers und Schließen des Popups (Esc, Klick
  daneben) verwerfen sie, das nächste Öffnen beginnt wieder bei der aktuellen Farbe.
- Neue öffentliche Hilfsfunktion `popups::picker_id(id)` (für Tests).

Annahme: „live“ heißt im Wähler selbst (Feld, Leiste, Vorschau). Das Dokument bzw. der Dialogwert
ändert sich wie bisher erst mit „Übernehmen“ (kein Verlaufsschritt während des Ziehens). Eine
Live-Vorschau auf der Leinwand beim Ziehen gehört zum Umbau FARB-1b.

## Tests
`crates\fp-ui\tests\color_picker_farb1a.rs` (kittest, echtes Ziehen über viele Frames im Glühen-Dialog):
- Ziehen im Farbfeld: Farbe ändert sich jeden Frame, bleibt nach dem Loslassen; ein Klick in die
  Farbtonleiste danach behält Sättigung/Helligkeit (mit dem alten Code: Rücksprung auf Grau, Test rot).
- Farbtonleiste ziehen: Farbton ändert sich live und bleibt (alter Code: rot).
- Esc verwirft, Dialogwert, Effekte und Verlauf unverändert; erneutes Öffnen ohne alte Arbeitsfarbe.
- 50× ziehen + Übernehmen: höchstens ein Effekt, Dialogfeld trägt jeweils die Farbe, OK = genau ein
  Verlaufsschritt, Effekt hat die zuletzt übernommene Farbe.

Gelaufen: `color_picker_farb1a::` 4/4, `dialogs_effects::` 6/6, `cargo check --workspace --all-targets`,
`cargo test -p fp-neutral` grün.

## Erledigte Befunde
- N27a „Effekt Glühen → Farbe … genauer Farbwähler … springt immer auf das ursprüngliche Grau zurück“.

## PARITAET
Farbfelder-Popup / genauer Farbwähler: „umgesetzt, Nachprüfung ausstehend“ (nur Fehlerbehebung).

## Nachprüfung
- Echtes Fenster: Rechteck, Effekt Glühen, Farbe anklicken, Knopf oben rechts, im großen Feld nach
  Rot ziehen → Markierung und Vorschau folgen und bleiben; Farbtonleiste ziehen → Farbton folgt;
  „Übernehmen“ → Farbfeld im Dialog rot, Vorschau auf der Leinwand rot; OK → ein Verlaufsschritt.
  Dasselbe mit Esc → alte Farbe. Stichprobe Inspektor Füllung und Strich.

## Neue Befunde
- keine

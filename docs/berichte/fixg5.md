# FIX-G5 – Nutzerentscheid 13.10.2026 21:20 (Q-02, Q-04, Q-05, alt 4, alt 6)

Auftrag: `docs\parity\TWEAKS_2026-10-13.md`, Abschnitt „Nutzerentscheid 13.10.2026 21:20“, alle Zeilen „FIX-G5“.
Slot `fp-s7` / `target-s7`. Commit: siehe `git log --grep FIX-G5`.

## Messungen im Original (unter der Sperre)

**Q-02 Maske ▸ Alles einblenden/ausblenden auf einer Bitmap** (JSF `dom.addElementMask(mode, false)` auf B1 in
`par_bitmap.fw.png`, Skript/Protokoll `re\_testdateien\par\fixg5\fixg5_mask.jsf`):

| Modus | vorher | nachher |
|---|---|---|
| reveal all | `[object Image]` B1, `mask = null`, pixelRect 40,40–520,360 | `[object Image]` **B1**, `mask = {mode: "mask to image", linked: true, enabled: true, element: [object Image] 40,40–520,360}`, pixelRect unverändert |
| hide all | wie oben | `[object Image]` B1, gleiche Maske; pixelRect = leeres Rechteck (±2147483648) |

→ Das Objekt bleibt ein Bild mit Maske am Bild (keine Gruppe). Deckt sich mit `re\04-objektmodell.md` §1.6b: Eine Maskengruppe
mit genau einem Inhalt ohne eigene Maske und ohne Effekte ist ein maskiertes Einzelelement.

**alt 6 Rechteck-Werkzeug-Inspektor:** Lauf `re\_testdateien\par\run\fixg5-rect-inspector\` (3 Paare, Drehbuch nur temporär)
sowie die vorhandenen Referenzbilder `re\_testdateien\par\ref\tool-shapes\07_rect_inspector-tool_normal.png`,
`ref\inspector-fill\21_tool_default_normal.png`, `ref\inspector-stroke\20_tool_default_normal.png`:
- Werkzeug-Inspektor **ohne Auswahl** (Rechteck, auch Stift): „Kombinieren“ + 5 Pfadknöpfe **sind immer da**. Im leeren
  Dokument T01 grau (07), im Dokument mit Pfaden (T04, 20/21) bedienbar aussehend, „Normal“ gedrückt.
- **Mit gewähltem Pfad** zeigt das Original den Pfad-Inspektor (Pfadknöpfe bedienbar, „Kombinieren“ grau) – das hatte Firepixel schon.
- Die Beschreibung in `inspector-fill.orig.md` 21 („ohne Pfadoperationen“) widerspricht dem eigenen Bild 21; PAR 18/21
  (f1f12f1) hatte die Knöpfe deshalb entfernt. Umgesetzt ist der Bildbefund, nicht „nur zeigen, wenn ein Pfad gewählt ist“
  (das Original zeigt sie auch ohne Auswahl).
- Der erste Lauf um 21:24 scheiterte (kein Vordergrundfenster, Sitzung gesperrt); der Runner hat das Original dabei neu
  gestartet. Zweiter Lauf 21:48–21:55 erfolgreich. Schritt 01 des Laufs zeigt noch den Inspektor des zuvor gewählten P1
  (Abwählen per JSF aktualisiert den Inspektor des Originals nicht sofort) – nicht ausgewertet.

## Änderungen

| Punkt | Umsetzung | Dateien |
|---|---|---|
| **Q-04** Gruppen ab Werk | Werksvorgabe war schon „Optimieren · Verlauf · Ausrichten“ und „Seiten · Status · Ebenen“ (`Workspace::expanded`); im frischen Profil des Slot-Builds bestätigt (Bild 01/02). Das abweichende Bild der Läufe kam aus dem wiederverwendeten Läufer-Profil. Keine Codeänderung, gespeicherte Layouts unberührt. | – |
| **Q-05** Unterebenen | Ebenen-Bedienfeld: Unterebenen als Ordnerzeile (23 px) mit Aufklapper und Ordnersymbol, um 28 px eingerückt; ihre Objekte eingerückt darunter (Miniatur +16 px je Stufe), rekursiv; Auf-/Zuklappen je Unterebene. Ziehen/Ablegen bleibt auf der obersten Objektstufe. | `crates\fp-ui\src\panels\layers.rs` (+ Unit-Test) |
| **alt 4** Strichart | Werksvorgabe „Präzise abger.“ (`bc_Basic`/`bn_Hard Rounded`, 1 px) für Linie, Stift, Vektorpfad, Rechteck/Ellipse/Polygon (`tools::styled`) und für den Werkzeugpinsel ohne Strichvorgabe (`panel::tool_brush_from`, `default_tool_brush`, `DEFAULT_TOOL_PRESET`); Werkzeug-Inspektor zeigt ab Werk „Präzise abger.“ (`ToolAttrs::default`). Eine im Inspektor gewählte Strichart gilt weiter; Bleistift unverändert 1-Pixel hart. Rechteck/Ellipse/Polygon hatten den Strich seit a5ed533 schon. | `fp-engine\src\commands\panel.rs`, `fp-engine\src\tools\mod.rs`, `fp-ui\src\inspector\obj_quads.rs` |
| **Q-02** Maske | `Element::masked_single()` (Maskengruppe mit genau einem Inhalt ohne eigene Maske/Effekte/Smart Shape). JS: Typ des Inhalts (`[object Image]`), `name` des Inhalts, `mask` wie bisher. Ebenen-Bedienfeld: Zeile mit Name des Inhalts und zweiter Miniatur (Maske). Dateistruktur bleibt die Maskengruppe (`GRPv`, verlustfreie Rundreise wie in 04 §1.6b gefordert). | `fp-model\src\element.rs`, `fp-script\src\imp\elem.rs`, `fp-ui\src\panels\layers.rs` |
| **alt 6** Pfadknöpfe | Werkzeug-Inspektor ohne Auswahl: 5 Pfadknöpfe + „Kombinieren“ (grau) wieder sichtbar; Knöpfe bedienbar aussehend („Normal“ gedrückt), wenn die Seite Pfade enthält, sonst grau; lösen im Werkzeug-Inspektor nichts aus. | `fp-ui\src\inspector\obj_quads.rs` |

## Angepasste Test-Erwartungen (nur wegen der neuen Vorgaben)

- `fp-ui\tests\fixd_ct2.rs` `stroke_category_submenu_beside_main_menu`: Strichart-Knopf des Pinsel-Inspektors heißt ab Werk „Präzise abger.“ statt „1-Pixel hart“.
- `fp-ui\tests\parx_brush1_ui.rs` `inspector_shows_effective_tool_brush`: Nach Wahl von „Präzise abger.“ (= Werksvorgabe) schickt die Oberfläche keine eigene Strichvorgabe mehr; geprüft wird jetzt der wirksame Werkzeugpinsel (Hard Rounded, Rand 0).
- `fp-ui\tests\inspector_parx_insp1.rs` `rect_tool_pathops_and_path_ops_enabled_for_one_path` (umbenannt): Pfadknöpfe im Werkzeug-Inspektor vorhanden – leeres Dokument grau, mit Pfaden bedienbar ohne Wirkung.
- Golden-Tests: keine Änderung nötig (fp-engine/fp-model/fp-script komplett grün, die Goldens zeichnen mit expliziten Strichen).

## Tests

- Neu: `fp-engine\tests\fixg5.rs` (3 Tests: Werksstrich je Werkzeug inkl. Stift, gewählte Strichart gewinnt, Alles ein-/ausblenden → maskiertes Bild), Unit-Test `panels::layers::tests`.
- `cargo test -p fp-engine -p fp-model -p fp-script`: grün. `cargo test -p fp-ui --test all`: 604 ok, 1 Fehler `menu_fixes_fv13::secondary_browser_preview_needs_configured_browser` („Manual öffnet externe Programme – GUI-Sitzung“, umgebungsabhängig, unabhängig von FIX-G5).
- `cargo check --workspace --all-targets`: ohne Warnungen.
- Brücke, Slot-Build, frisches Profil (Bilder `re\_testdateien\par\fixg5\fp\`): 01 Gruppen ab Werk, 02 Unterebenen in `par_layers`, 03/04 Rechteck-Werkzeug-Inspektor (leer / mit Pfad, Strichart „Präzise abger.“), Linie → `bc_Basic/bn_Hard Rounded/1`, 05 Maske ▸ Alles einblenden auf B1 → JS `[object Image]|B1|mask to image`, Ebenenzeile „B1“ mit Maskenminiatur.

## PARITAET/TWEAKS → „umgesetzt, Nachprüfung ausstehend“

Q-02, Q-04 (nur bestätigt), Q-05, alt 4 (tool-line-pen Frage 1, tool-shapes 12), alt 6 (tool-shapes 07, inspector-fill 21 neu bewerten).

## Nachprüfung

- Ebenen-Bedienfeld mit `par_layers.fw.png` neben dem Original (panel-layers/01): Einrückung, Aufklapper klappt zu/auf, Klick auf Kindzeile wählt das Objekt.
- Maske ▸ Alles einblenden/ausblenden auf B1 im echten Fenster: Ebenenzeile „B1“ mit Maskenminiatur; Inspektor (nicht angeglichen, Original zeigt vermutlich den Bitmap-Inspektor mit Maske).
- Linie/Stift/Rechteck im frischen Profil zeichnen: Strich „Präzise abger.“ 1 px, Pixel gegen Original (tool-shapes 12 Export).
- Rechteck-Werkzeug ohne Auswahl in T01 (grau) und T04 (bedienbar) gegen Referenzbilder 07/21.

## Neue Befunde / offen

- Inspektor eines maskierten Einzelelements zeigt weiter den Gruppen-/Masken-Inspektor; `pixelRect` nach „Alles ausblenden“ liefert die Gruppengrenzen statt des leeren Rechtecks des Originals.
- Ob die Freigabe der Pfadknöpfe im Werkzeug-Inspektor wirklich von „Seite enthält Pfade“ abhängt, ist aus drei Bildern geschlossen, nicht gezielt gemessen.
- Unterebenen-Zeilen: Umbenennen per Doppelklick ist dort nicht verdrahtet (vorher wirkungslos, `element.setName` ändert nicht den Ebenennamen).

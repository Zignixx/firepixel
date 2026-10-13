# FIX-R1 – Reste aus FIX-G4/FIX-G5 (13.10.2026)

Slot `fp-s5` / `target-s5`, Grundlage `592f9a4`. Commits: siehe `git log --grep FIX-R1`.

## Umgesetzt

| Punkt | Änderung | Dateien |
|---|---|---|
| **Stern aus Rahmen zu klein** (fixg4, neuer Befund) | Regel aus den Original-Werten (a1run tool-autoshapes, `px\tool-autoshapes\paths.json`): runde Formen nehmen als Radius die **halbe längere Seite** des Ziehrahmens, der Kreis beginnt am **Startpunkt** und liegt in Zugrichtung. Belege: Zug 100,100→220,200 (120 × 100) → Stern Radius 60, Mitte 160,160, Grenzen 102,100–218,209; Donut/Torte 100,100–221,221 (120 × 120); Intelligentes Polygon Mitte 160,160, Radius 60; Spirale Mitte 160,160, Radius 60. Senkrechte Züge ohne Breite (b05): 100,212.8 → +40 → Radius 20, Mitte 120,233; 160,100 → −20 → Radius 10, Mitte 150,90 (waagrecht folgt die Richtung dem senkrechten Zug). Neue Hilfe `SS.radial` für Stern, Torte/Donut, Intelligentes Polygon, Spirale. Das Werkzeug schickt den Startpunkt jetzt als `from` mit (`autoShape.insert`), damit die Zugrichtung erhalten bleibt; Alt (von der Mitte) bleibt Mitte = Startpunkt. Die übrigen Autoformen (Pfeil, Pfeillinie, Rechteckformen, L-Form, Verbindungslinie, Messwerkzeug) nutzen Breite/Höhe getrennt – geprüft, stimmen mit den 02-Werten überein, keine Änderung. | `fp-script` `autoshape\helpers.js`, `shapes\{star,pie,smart_polygon,spiral}.js`, `autoshape\mod.rs` (`drag_corners`); `fp-engine` `tools\draw.rs`; Test `fp-script\tests\autoshape.rs` `radial_shapes_from_frame_like_original` |
| **Q-02 Inspektor** | Maskiertes Einzelelement (Maske ▸ Alles einblenden/ausblenden) wählt den Inspektor seines Inhalts: Bitmap → Bitmap-Inspektor (Zeile 27), Typ „Bitmap“, Namensfeld mit dem Namen des Inhalts. Nur die Inspektorwahl, Befehls-Bedingungen (`element_is`) unverändert. | `fp-ui` `inspector\layouts.rs` (`insp_elem`), `obj_quads.rs` (`type_label`), `quads.rs` (Name); Unit-Test `inspector::layouts::tests::masked_bitmap_shows_bitmap_inspector` |
| **Q-02 `pixelRect`** | Für maskierte Einzelelemente die nach der Maske sichtbaren Pixel (gerendert, Alpha ≥ 2); ganz ausgeblendet das leere Rechteck ±2147483648 wie im Original. **Dabei gefundener Fehler behoben:** Firepixel speicherte die Maskenbitmap von „Alles einblenden/ausblenden“ und „Auswahl einblenden/ausblenden“ mit Weiß = sichtbar, der Renderer liest aber (richtig, 07 §7.5 Laufzeit T3) **invertiertes Grau** (Maskenwert = Alpha · (255 − Grau)/255). Folge vorher: „Alles einblenden“ blendete das Objekt auf der Leinwand **aus** und umgekehrt. Jetzt schwarz = sichtbar gespeichert; die Maskenminiatur im Ebenen-Bedienfeld zeigt invertiert (weiß = sichtbar wie im Original). | `fp-engine` `commands\modify.rs`; `fp-script` `imp\elem.rs`; `fp-ui` `panels\thumbs.rs` (`mask_thumb`), `panels\layers.rs`; Test `fp-script\tests\fixr1.rs` |
| **Q-05 Unterebene umbenennen** | Doppelklick auf eine Unterebenen-Zeile öffnet das Umbenennen-Feld (nicht auf dem Aufklapper), Eingabe übernimmt in einem Schritt, Esc verwirft. `element.setName` auf einer Unterebene setzt jetzt auch den Ebenennamen (vorher wirkungslos). Leerer Name wird ignoriert. | `fp-ui` `panels\layers.rs`; `fp-engine` `commands\draw.rs`; kittest `fp-ui\tests\fixr1.rs` |

## Prüfungen

- `fp-script --test autoshape` (8), `--test fixr1` (2), `parx_insp1`, `parx_insp2`, `pixel_rect_bm1` grün.
- `fp-engine --test all`: `fixg5::`, `commands::`, `canvas_tools::`, `fixg4::`, `all_commands::` grün.
- `fp-ui --lib` (121) grün; `fp-ui --test all`: `fixr1::`, `panels_keyboard::`, `panels_menus::`, `menu_mn2::`, `inspector_parx_insp1::`, `fixg4::`, `gesamt_werkzeuge::` grün.
- `fp-render` Tests grün; Golden: `fp-export`, `fp-effects`, `fp-color` (`--test golden`), `fp-vector --test render_golden` grün. `fp-neutral` grün.
- `cargo check --workspace --all-targets` ohne Warnungen.
- **Brücke (Slot-Build `target-s5\debug`, eigenes Profil im Scratchpad, Fenster 1400 × 900 unten rechts, nicht aktiviert):**
  - `autoShape.insert` Stern, Rahmen 100,100–220,200, `from` 100,100 → Grenzen 102.94,100–217.06,208.54 (Original 102,100–218,209).
  - Bitmap 40,40–240,200, JS `addElementMask('hide all')` → `[object Image]`, `mask to image`, pixelRect ±2147483648; `reveal all` → pixelRect 40,40,240,200, Bitmap auf der Leinwand sichtbar, Maskenminiatur weiß. Inspektor-Kopf „Bitmap“.
  - Unterebene „Unter A“: `ui_input` Doppelklick öffnet das Feld, Tippen „Unter B“ + Enter → Ebenenname „Unter B“ (JS `name`), Zahl der Unterebenen unverändert; Doppelklick + Esc ändert nichts.
  - Beobachtung ohne Folge: Im ersten Lauf standen danach zwei zusätzliche unbenannte „Unterebene“-Zeilen; der erste `ui_input` war wegen eines falschen Tastennamens („enter“) abgelehnt worden. Mit gezielten Wiederholungen (Doppelklick auf gewählte/ungewählte/frisch angelegte Unterebene, Esc, Enter) nicht reproduzierbar; Herkunft offen (siehe Nachprüfung).

## PARITAET/TWEAKS → „umgesetzt, Nachprüfung ausstehend“

tool-autoshapes Zeile 02 (Größe runder Formen aus dem Rahmen); TWEAKS Q-02 (Inspektor, pixelRect – Rest aus FIX-G5); Q-05 (Umbenennen der Unterebene).

## Nachprüfung

- a1run tool-autoshapes/02_draw_each (Stern, Donut, Torte, Intelligentes Polygon, Spirale) mit echtem Zug 100,100→220,200: Grenzen/Griffe gleich paths.json; zusätzlich ein Zug nach links oben und ein hoher schmaler Rahmen gegen das Original (die Richtungsregel ist aus den zwei b05-Zügen geschlossen).
- Maske ▸ Alles einblenden/ausblenden auf B1 (par_bitmap) neben dem Original: Leinwand (einblenden = sichtbar, ausblenden = unsichtbar), Inspektor = Bitmap-Inspektor, Ebenenzeile mit Maskenminiatur weiß/schwarz; „Auswahl einblenden/ausblenden“ mit Pixelauswahl ebenso.
- Ebenen-Bedienfeld: Unterebene per Doppelklick umbenennen, Enter/Esc; dabei darauf achten, ob unbenannte Unterebenen entstehen (Beobachtung oben).

## Neue Befunde

- Der Bitmap-Inspektor arbeitet bei einem maskierten Bild auf der Maskengruppe (Deckkraft/Mischmodus/Effekte der Gruppe); ob das Original dort die Werte des Bildes oder der Gruppe zeigt, ist nicht gemessen.
- Ältere Firepixel-Dateien mit über „Alles einblenden/ausblenden“ bzw. „Auswahl einblenden/ausblenden“ erzeugten Masken haben die alte (falsche) Polarität gespeichert; sie sehen unverändert aus (der Renderer wurde nicht geändert), zeigen also weiter das Gegenteil des damals Gewählten – genau wie das Original diese Dateien zeigen würde. Keine Umwandlung vorgesehen.

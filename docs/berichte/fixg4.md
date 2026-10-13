# FIX-G4 – TW-04, TW-05, TW-07, Stern-Griff (13.10.2026)

Slot `fp-s5` / `target-s5`, Grundlage `8dda9e2`. Commit: siehe `git log --grep FIX-G4`.

## Umgesetzt

| Punkt | Änderung | Dateien |
|---|---|---|
| **TW-04** Esc nach dem Auswahlrahmen | Esc mit dem Auswahlrahmen/Ovalrahmen (ohne laufenden Zug, mit Pixelauswahl) beendet den Bitmapmodus, Werkzeug → Zeiger **und leert die Objektauswahl** (Original menu-modify-a/22: `fw.selection` = `[]`). Neuer Parameter `deselect` an `modify.exitBitmapMode`. Zauberstab (L13) und Polygon-Lasso (06/08) bleiben wie gemessen (Bitmap bleibt gewählt). | `fp-engine` `tools\bitmap.rs`, `commands\modify.rs`; Test `canvas_tools::pointer_double_click_bitmap_enters_bitmap_mode` angepasst (bisher nur „wie beim Zauberstab“ angenommen) |
| **TW-05** `pixelRect` von Pfaden mit Strich | Nur der gemeldete Rahmen (`element_pixel_rect`, einziger Nutzer: JSF `pixelRect`), Pixelausgabe/Export unberührt. Regel aus den Original-Werten (menu-edit/02 u. a.): Strichkante (Grenzen ± Strichanteil) auf die **nächste** Pixelgrenze, d. h. `links = floor(l − g + ½)`, `rechts = ceil(r + g − ½)`; **offene** Pfade mit **ungerader** Strichstärke zusätzlich rechts/unten +1 (Strich auf Pixelmitten). Pfade ohne Strich wie bisher `floor/ceil`. Ergebnis par_vector: L1 50,300,301,361 · PG 465,165,535,245 · P1 200,170,421,241 · E1 50,170,150,240 · R1/R2/ST unverändert = Original. Die 8-px-Werte (PAR 17/08: 42…248 / 46…244 / 50…240) bleiben. | `fp-engine` `docops.rs`; Test `fp-engine` `fixg4.rs` |
| **TW-07** Bearbeiten-Menü ohne Verlauf | „Rückgängig nicht möglich“ / „Wiederholen nicht möglich“ (grau), EN „Can't Undo“ / „Can't Redo“ (Zugriffstasten U/R wie bisher). Mit Verlauf unverändert „Rückgängig: <Schritt>“; Wiederholen ohne zurückgenommenen Schritt zeigt weiter den letzten Befehl. | `fp-ui` `menu.rs`; `menu.ftl` de/en (`menu-edit-undoImpossible`, `menu-edit-redoImpossible`); Test `fp-ui` `fixg4.rs` |
| **Stern-Griff** „Punkte“ | Griff 4 reagiert wie im Original nur **senkrecht** (vorher waagrecht, daher „passiert nichts“ beim senkrechten Zug): `n = Start − Δy · 21,75 / Radius1`, 3…50. Radius 60: +40 → 3, −80 → 34, waagrecht keine Änderung. Nach einem Griffzug steht Griff 4 auf der Skala des Originals `Mitte + R − (n − 3) · 2R / 47` (5 → 214,89, 3 → 220, 34 → 140,85; frisch eingefügt weiter 0,88 · R = 212,8). Der Faktor 21,75 ist aus dem einen Messpunkt (−80 → 34) abgeleitet. | `fp-script` `autoshape\shapes\star.js`; Test `autoshape::star_points_handle_vertical_like_original` |

## Prüfungen

- `fp-engine --test all`: `fixg4::`, `canvas_tools::`, `parx_insp1::`, `parx_tools2::`, `bitmap_bounds_bm1::` grün (79).
- `fp-script`: `autoshape`, `parx_insp1`, `parx_insp2`, `pixel_rect_bm1`, `fx3_e2e` grün.
- `fp-ui --test all`: `fixg4::`, `i18n_menue::`, `canvas_fv14::`, `ai_kittest_klickfolgen::`, `dialogs_warnung::`, `ux_a_abgleich::`, `gesamt_werkzeuge::`, `i18n_keine_festen_texte::` grün; `fp-i18n`, `fp-neutral` grün.
- Golden: `fp-export`, `fp-effects`, `fp-color` (`--test golden`), `fp-vector --test render_golden` grün.
- `cargo check --workspace --all-targets` ohne Warnungen.
- **Brücke (Slot-Build, eigenes Profil):** Stern per `autoShape.insert` (Rahmen 100,100–220,220), Zeiger, `ui_input`-Zug am Punkte-Griff (100,212.8) um −80 → „Punkte: 34“, Griffe 1–3 = Original d05_star_points_up80 (162.77,130.13 · 171.02,101.02 · 168.21,131.15), Griff 4 bei 100,140.85. par_bitmap: B1 gewählt, Auswahlrahmen 100,100→300,250, Esc (`ui_input` key) → Auswahl `[]`, keine Pixelauswahl, Werkzeug Zeiger.

## PARITAET → „umgesetzt, Nachprüfung ausstehend“

TWEAKS TW-04, TW-05, TW-07; tool-autoshapes Zeile 05 (Stern-Punkte-Griff).

## Nachprüfung

- menu-modify-a/22_exit_bitmap_result erneut laufen lassen: `fw.selection` = `[]` auf beiden Seiten.
- menu-edit/02, menu-modify-a/06…17, dialogs-d01-d10/11: `pixelRect` L1/PG/P1/E1 gleich dem Original.
- menu-edit/13_insert_menu_result: „Rückgängig nicht möglich“ / „Wiederholen nicht möglich“ grau.
- a1run tool-autoshapes/05_handle_drag_star_points_result: Stern danach mit 3 Zacken; Bildvergleich.

## Neue Befunde

- **Stern aus Rahmen zu klein:** Original macht aus dem Zug 100,100→220,200 (120 × 100) einen Stern mit Radius 60, Mitte 160,160 (also Radius = halbe Breite, Mitte = links/oben + Radius); Firepixel nimmt `min(B, H)/2` = 50 und die Rahmenmitte (Grenzen 112,45…207,55). `star.js` `init`. Nicht geändert (nicht im Auftrag), Messung mit einem nicht-breiten Rahmen fehlt.
- **Fremd rot:** `fp-ui --lib canvas::tests::open_dialog_blocks_canvas_and_export_area_opens_dialog` schlägt auch auf sauberem `main` (8dda9e2) fehl (Export-Dialog hat keinen Bereich, `None`), nicht von FIX-G4.
- `customData` des Sterns enthält nach einem Griffzug zusätzlich `pointsScale: true` (Merker für die Griffskala).

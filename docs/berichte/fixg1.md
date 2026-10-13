# FIX-G1 – Dialog-Feinheiten TW-01, TW-10, TW-11, TW-12

Stand 13.10.2026, Slot `fp-s2`. Grundlage: `docs\parity\TWEAKS_2026-10-13.md`, Referenzbilder
`re\_testdateien\par\run\dialogs-d11-d20` (14_d18, 16_d19, 19_d20) und `dialogs-d21-d30\11_d26`.
Das Design-Theming der Dialoge (DES-1) ist unverändert.

## Umgesetzt

| Punkt | Was | Dateien |
|---|---|---|
| TW-01 | Numerisch transformieren: Kettensymbol ist jetzt Klammer von Zeile ↔ bis Zeile ↕ mit Schloss daneben (wie Original, nur bei ☑ „Proportionen beibehalten“; Tooltip = Kästchentext). ☑ „Attribute skalieren“ hat jetzt Wirkung: der Dialog schickt `scaleAttributes`, `modify.transform.numericTransform` skaliert damit die Strichbreite mit √(sx·sy) (wie Info-Bedienfeld/Inspektor), Drehen nie; der Haken wird bei OK gemerkt (`XformToolXformsBFE`). Die beiden Kästchen waren schon da (im fp-Paarbild nur abgeschnitten, weil das Bild auf die Größe des Originals zugeschnitten ist). | `fp-ui` dialogs/doc_dialogs.rs, `fp-engine` commands/modify.rs |
| TW-10 | Raster bearbeiten: Popup-Schieber ▾ direkt rechts an beiden Abstandsfeldern (gleiche Komponente wie Inspektor/Dialoge), Einheit „Pixel“ rechts davon. Kennlinie 1 … 100 linear ist **Annahme** (Bereich im Original nicht gemessen, BEFUNDE DLG-2 bleibt offen); getippt weiter 1 … 10000. | `fp-ui` dialogs/catalog.rs |
| TW-11 | Bildgröße: ↔/↕-Symbole (gezeichnet) statt „Breite:/Höhe:“ in Pixelabmessungen und Druckgröße; Text bleibt Tooltip und Name. Klammer mit Schloss wie TW-01 an beiden Gruppen. „Auflösung:“ bleibt Text. | `fp-ui` dialogs/doc_dialogs.rs |
| TW-12 | Leinwandgröße: ↔/↕ gezeichnet (statt Schriftzeichen); „Aktuelle Größe“ als ↔ 800  Pixel / ↕ 600  Pixel (Symbol, Zahl, Einheit; „B: 800 Pixel“ bleibt Tooltip). Anker als Bildchen wie im Original: je Feld ein schwarzes Quadrat an der Ankerlage und graue Pfeile in die Wachstumsrichtungen, unabhängig von der Wahl; die Wahl ist nur hervorgehoben. | `fp-ui` dialogs/doc_dialogs.rs |

Commit: siehe `git log --grep FIX-G1`.

## Tests

- neu `crates\fp-engine\tests\fixg1_numeric.rs` (Strich mit/ohne „Attribute skalieren“, Drehen, Größe ändern, Rückgängig in einem Schritt) – grün
- neu `doc_dialogs::tests::numeric_transform_passes_scale_attributes` – grün
- `dialogs_dlg1b::canvas_size_live_anchor_keys_and_select_all` angepasst (↔/↕ sind keine Textzeichen mehr) – grün; dazu dialogs_dlg1/dlg1b/dlg2/dlg5/dlg7/des1/enter/sliders/open/move, nz3, ux_b_abgleich, form_form1: 86 grün
- `cargo check --workspace --all-targets` ohne Warnungen
- keine neuen ftl-Schlüssel (vorhandene wiederverwendet), keine neuen Bilder/Texte → fp-i18n/fp-neutral nicht nötig
- echtes Fenster, Brücke (Slot-Debug-Build, eigenes Profil): je Dialog ein Bildschirmfoto in `re\_testdateien\fixg1\` (`d26_numeric`, `d18_grid`, `d19_imagesize`, `d20_canvassize`, je auch `_crop`)

## PARITAET-Zeilen → „umgesetzt, Nachprüfung ausstehend“

D18 (Popup-Schieber), D19 (↔/↕, Schloss), D20 (↔/↕, Aktuelle Größe, Ankerbilder), D26 (Kette mit Schloss, Attribute skalieren mit Wirkung).

## Nachprüfung

- D26: Rechteck mit 8-px-Strich, Numerisch transformieren 200 % mit ☑ Attribute skalieren → Strich 16 px; ohne Haken → 8 px; Dialog erneut öffnen → Haken wie zuletzt per OK.
- D18: ▾ neben ↔ ziehen → Feld zeigt Wert live, Loslassen übernimmt; im Original den Bereich messen und `GRID_SPACING` anpassen.
- D19/D20: Paarbild gegen Original (Symbole, Schloss, Ankerbilder, „Aktuelle Größe“) in 3 Designs × de/en.

## Neue Befunde

- Paarbilder `dialogs-*/fp/*` werden auf die Größe des Original-Dialogs zugeschnitten; ist der fp-Dialog größer, fehlen Teile (so wirkte TW-01 „Kästchen fehlen“). Zuschnitt auf den eigenen Dialog wäre aussagekräftiger.
- Raster bearbeiten: Bereich/Kennlinie der Popup-Schieber im Original messen (Annahme 1 … 100).

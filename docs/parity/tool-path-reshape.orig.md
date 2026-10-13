# 13 `tool-path-reshape` – Freiform, Bereich umformen, Pfadausbesserung ±, Messer: Aufnahme des Originals (PAR-O-11-13, 13.10.2026 ≈ 13:05–13:20)

Testdatei T04 (P1 offen: 200,200 [→240,160] · 290,200 · 370,200 · 420,240; R1 50,50–150,120 ist ein **Pfad**), Arbeitskopie
`re\_testdateien\par\work\p1113_t04.fw.png`, je Messung frisch mit `fw.openDocument(url)`. Fenster 1600 × 1000, Zoom 100 %, Leinwand
bei 297,183. **Echte Mauszüge**, danach Knoten per Skript. Referenzbilder `re\_testdateien\par\ref\tool-path-reshape\` (+ `INDEX.md`),
Exporte `re\_testdateien\par\px\tool-path-reshape\pixels.csv`, Knoten `paths.json`. Art: M/J/W/E/B wie Einheit 11.

## Prüfpunkte

| Nr | Kürzel | Original | Art |
|---|---|---|---|
| 01 | `freeform_inspector` | Kopf „Freiform“; „Größe: 72“ + ▼, ☑ „Druck“, ☑ „Vorschau“ | W, B `01` |
| 02 | `freeform_push` | P1 gewählt, Zug von außen 300,120 → 300,190: Kreis schiebt den Pfad, P1 4 → **8 Knoten**, bleibt offen; Schritt „Freiformwerkzeug“ | M, J, E, B `02` (2) |
| 03 | `freeform_pull` | Zug am Segment 249,170 → 249,110: Pfad wird gezogen, **10 Knoten**, Oberkante 110 | M, J, E, B `03` |
| 04 | `freeform_size_keys` | Pfeiltasten und [ ] bei aktivem Werkzeug (ohne Maustaste) ändern die Größe **nicht** (bleibt 72) | M, W |
| 05 | `reshape_area` | Kopf „Bereich umformen“: Größe 72, Stärke 80, Druck: ☑ Größe ☐ Stärke. Zug 330,210 → 330,150: P1 **7 Knoten**; Schritt „Bereich umformen“ | M, J, E, W, B `05` |
| 06 | `scrubber_add` | Kopf „Pfadausbesserung - additiv“: ☑ Druck, ☑ Geschwindigkeit, Rate 10. Zug entlang P1 (Strich 1 px hart): **keine Änderung, kein Verlaufsschritt**, Auswahl danach leer | M, J, E, W |
| 07 | `scrubber_sub` | wie 06 (subtraktiv): keine Änderung, kein Schritt | M, J, E |
| 08 | `knife_cut` | R1 gewählt, Messer 100,30 → 100,140: R1 wird zu **zwei offenen Pfaden** 50–100 und 100–150 (je 4 Knoten), beide gewählt, Name bleibt; Schritt „Messerwerkzeug“ | M, J, B `08` (2) |
| 09 | `knife_shift` | Umschalt, Zug 80,30 → 110,140: Schnitt auf **Senkrechte** gerastet (Teile 50–80 / 80–150) | M, J |
| 10 | `knife_no_selection` | Ohne Auswahl lässt sich das Messer **nicht wählen** (Klick auf den Knopf / Skript → Werkzeug bleibt Zeiger); Tooltip „Messerwerkzeug (Y)“ | M, J |
| 11 | `rect_to_path` | Neues Rechteck (Grundform `RectanglePrimitive`) gewählt, Freiform-Zug über die Oberkante: **keine Wirkung, keine Meldung**, bleibt Grundform | M, J, B `11` |
| 12 | `inspector_heads` | Kopffeld (Static 81 × 39 px, Platz für 2 Zeilen) zeigt „Freiform“, „Bereich umformen“, „Pfadausbesserung - additiv“ bzw. „- subtraktiv“ (Umbruch nur im Bild prüfbar); Messer zeigt den Inspektor des gewählten Pfads („Pfad“ / Name R1) | W, B `12` (4) |
| 13 | `cursor` | Freiform: Schiebe-Zeiger (außen) und Zieh-Zeiger (am Pfad), je Hotspot 4,4; Bereich umformen 4,4; Pfadausbesserung ± 16,14; Messer 7,20 | B `13` (6) |
| 14 | `undo` | „Freiformwerkzeug“, „Bereich umformen“, „Messerwerkzeug“; Pfadausbesserung ohne Schritt | M, W |

## Hinweise

- Ausklappliste am Freiform-Knopf: Freiform (O), Bereich umformen (O), Pfadausbesserung - additiv (O), Pfadausbesserung - subtraktiv (O); Messer eigener Knopf (Y).
- Pfadausbesserung wirkt nur auf Druckdaten (Strich mit Druckempfindlichkeit); mit Strich „1-Pixel Hard“ messbar keine Änderung.

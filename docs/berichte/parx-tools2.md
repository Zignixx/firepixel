# PAR-X-TOOLS-2 – Einheiten 05–07 beheben (13.10.2026)

Stempel/Farbe ersetzen/Rote Augen (05), Bitmap-Auswahl (06), Farbeimer/Verlauf/Pipette (07) nach den Messwerten
des Originals (`re\_testdateien\par\px\<einheit>\pixels.csv`). Slot fp-s11, Build `target-s11`. Die Arbeit wurde
durch einen Neustart der Sitzung (≈ 3 h) unterbrochen und danach auf den aktuellen main (u. a. PAR-X-TOOLS-1)
neu aufgesetzt.

## Commits (main)

| Commit | Inhalt |
|---|---|
| fcf0f86 | `commands\bitmap.rs` als UTF-8 repariert (Schritt „Füllen“ statt „FÃ¼llen“, Befund PAR-X-INSP-1) |
| 7a710c3 | Zauberstab-/Farbeimer-Toleranz (06/10, 07/02, 07/04), Farbeimer-Rand Anti-Alias/Weiche Kante (07/05) |
| 29b532b | Farbe ersetzen (05/07, 05/08), Rote Augen (05/11, 05/12), Stempel live (05/04), Schrittnamen (05/15) |
| cdceac5 | Auswahlrahmen/Lasso/Zeiger (06/02, 06/03, 06/05, 06/08, 06/16, 06/17, 06/18, 06/22), Verlauf/Pipette/Farbeimer auf Vektoren (07/03, 07/07, 07/08, 07/11, 07/13, 07/16) |

## Messwerte → Umsetzung

- **Toleranz Zauberstab/Farbeimer** (aus den Bildern abgeleitet, pixelgleich für 0/32/100): ein Pixel gehört
  dazu, wenn jeder Kanal ≤ Toleranz abweicht **und** max(d) − min(d) ≤ Toleranz (d = Kanalabweichungen).
  Hash gleich: 3e810c0f…, ba324474…, 8434f316…; Farbeimer „Hart“ im Auge b13660af… gleich.
- **Farbeimer Anti-Alias** (Näherung): Pixel rechts/unterhalb des Bereichs werden mit 1 − Überschreitung/255
  gemischt (Auge 547 ± 5 %, Verlauf 32 → 5 518 statt 5 553). Weiche Kante wie beim Rahmen.
- **Farbe ersetzen**: harte Spitze (Größe/2 um die Pixelmitte), Farbton der In-Farbe bei gleicher Sättigung
  und Helligkeit, Wirkung Stärke/128 · (1 − Abstand/(Toleranz · 2,56)). Rot → Blau, Kolorieren, „Von: Grafik“:
  Hash gleich (e6c50102…, d9d4838d…, 0f8fa348…); Toleranz 32/100 innerhalb ± 12 %.
- **Rote Augen**: Rechteck mit Kreuz; rote Pixel nach den Voreinstellungen (Farbton 300°–5°, Sättigung ≥ 50 %,
  Helligkeit ≥ 25 %) abdunkeln, Rand halb; Toleranz ≤ 50 gleich, 100 → dunkles Grau (#1E1E1E statt #1F1F1F).
  Zahlen Näherung (1 116 ± 5 %); Klick ohne Wirkung/Schritt, Rechteck ohne Fläche → „Bitmap bearbeiten“.
- **Auswahl verschieben**: `select.moveMarquee`, `select.movePixels` (neu); Hashes 06/05 und 06/16 gleich.
- **Verlauf ohne Rahmen**: ganze Leinwand, Bitmap wächst (244 000 px wie das Original).

## Befunde

Abgehakt in `docs\BEFUNDE.md` (22 PAR-Zeilen 05–07 + Kodierung `bitmap.rs`). **Offen**:
- 05/03 Stempel-Zugende 2 px kürzer: nicht angefasst (gemeinsame Spitzenrasterung mit der Retusche,
  PAR-X-TOOLS-1 hat `Dab::stroke` umgebaut; nach deren Stand neu messen).
- 07/12 Pipette 3x3/5x5: Nutzerfrage, nicht geändert. 05/05, 05/14 (🎨, Frage 1) nicht geändert.

## Annahmen

- Verlaufsschritt beim Verschieben von Rahmen/Pixeln „Verschieben“ (im Original nicht gemessen).
- Zweites Esc: der Zeiger hebt eine einzeln gewählte Bitmap nicht mehr auf (nur so gemessen).
- Füllfarbwechsel färbt gewählte Vektoren sofort nur beim Farbeimer (nur so gemessen).
- „Live-Auswahlrahmen“ wiederholt angewandt macht den Rahmen jeweils weiter weich (Original nicht gemessen).
- Pipette: Alt nimmt das jeweils andere Farbfeld.

## Nachprüfung (Brücke, echtes Fenster)

1. Auswahlrahmen aufziehen → Inspektor B 200 H 140 X 100 Y 80; Rand auf Weiche Kante 10 → Rahmen sofort weich.
2. Werkzeugbedienfeld: Strichfeld anklicken, Pipette auf Grün → Strichfarbe #00A000.
3. Farbeimer, R2 gewählt, Füllfarbe im Werkzeugbedienfeld ändern → R2 sofort, Schritt „Füllen“.
4. Stempel: beim Ziehen erscheint die Kopie live; Rote Augen: Rechteck + Kreuz beim Ziehen.

## Tests

`cargo test -p fp-engine --test all` grün (parx_tools2 14 Fälle, parx_tools1, all_commands mit den drei neuen
Befehlen; Verlaufstests in `commands.rs`/`tools_fv16.rs` auf 07/07 angepasst), `cargo check --workspace
--all-targets`, `fp-i18n` grün. `fp-neutral` meldet eine fremde Fundstelle in `docs\funktionstest\ct1a_2026-10-13.md:60`
(nicht von diesem Agenten). Bilder ohne Oberfläche: `crates\fp-engine\tests\data\parx-tools2\`.

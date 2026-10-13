# PAR-X-INSP-2 – Gegenprüfungs-❌ der Einheiten 17/18 beheben

Stand 13.10.2026, Slot fp-s11. Grundlage: `docs\parity\inspector-path-rect.md`, `inspector-fill.md` (❌ der
Gegenprüfung PAR-R-1: 17/07b, 17/08b, 18/03b, 18/04b, 18/08b, 18/20b), `*.orig.md`,
`tool-brush-stroke-options.orig.md` Nr. 08, Exporte `re\_testdateien\par\px\inspector-fill\03_*.png`.
Alle sechs Punkte bearbeitet; Gegenprüfung (PAR-R) offen.

## Umgesetzt

| Befund | Commit | Änderung |
|---|---|---|
| 17/07b | `79f9c58` | Eigene Schlüssel für die Objekt-Liste `blend-obj-*` in `filter.ftl` (de: Sprenkeln, Ineinanderkopieren, Helligkeit, Tönung, Radieren; en: Dissolve, Overlay, Luminosity, Tint, Erase). `texts::blend_labels_obj()` für Inspektor, Ebenen-Bedienfeld und Live-Filter „Farbfüllung“; Werkzeug-/Pinsel-Liste bleibt bei `blend_labels()` (Auflösen, Überlagern, Luminanz, Farbton, Löschen). Aufklapplisten: der Popup-Inhalt bekommt die volle Listenbreite (`set_min_width`; `Popup::width` zählte den Rahmenabstand mit, daher „Negativ multiplizie…“); gilt für alle Kombifelder und die Musterliste. |
| 17/08b | `148ca1d` | JSF `pathAttributes.brushPlacement`: BRPi 0 = „inside“, 1 = „center“, 2 = „outside“ (`fp-script imp\elem.rs`). |
| 18/03b | `22f282c` | Vorgabegriffe je Verlaufsart (`panel::default_handles`), gegen die 12 Exporte des Originals ausgemessen: Linear oben → unten; Radial, Rechteck, Kegel, Ellipse, Streifen, Kleine Wellen Mitte → Mitte rechts, zweite Achse Mitte → Mitte oben; Glanz, Falten Mitte → rechte untere Ecke; Wellen Mitte → halber Weg dorthin; Supernova Mitte → rechte untere Ecke + zweite Achse zum halben Weg nach links unten; Kontur Mitte → halbe kleinere Seite. Artwechsel ersetzt **unveränderte** Vorgabegriffe der alten Art durch die der neuen (verschobene/gedrehte bleiben). Ergebnis: 10 von 12 Arten pixelgleich (mittlere Abweichung 0,0), Supernova und Kontur in der Form gleich. |
| 18/04b | `79f9c58` | Farbrad-Knopf links neben den Farbstopps (wie im Original): öffnet den Farbwähler des gewählten Farbstopps (sonst des ersten) gleich mit dem Farbkreis (`color_picker::request_wheel`, `wheel_open`). |
| 18/08b | `79f9c58` | `fields::fmt_num` zeigt nie „-0“ (alle Zahlenfelder, auch Winkel im Muster-/Verlaufs-Popup). |
| 18/20b | `22f282c` | `tool.shape` benennt den Verlaufsschritt nach dem Zeichenbefehl (`cmd-draw-*`, z. B. „Rechteck“, „Ellipse“, en „Rectangle“) statt „Form zeichnen“. |

## Tests

- `crates\fp-engine\tests\parx_insp2.rs` (4): 10 Arten gegen Stichproben der Original-Exporte (±2), Supernova/Kontur-Form, Artwechsel nur bei Vorgabegriffen + Wiederholung + Rückgängig + gedrehte Griffe bleiben, Schrittname Rechteck/Ellipse/Rectangle + Rückgängig.
- `crates\fp-script\tests\parx_insp2.rs` (1): `brushPlacement` innen/mittig/außen über Inspektor und `setBrushPlacement` (wiederholt, Rückgängig), `pixelRect`.
- `crates\fp-ui\tests\inspector_parx_insp2.rs` (4): Namen je Liste de/en, Objekt-Liste ganz lesbar (schlägt ohne die Breitenkorrektur fehl: 117 statt 121 px), kein „-0“ (auch Muster-Popup), Farbrad öffnet Farbwähler mit Farbkreis, Esc schließt ihn.
- angepasst: `fp-engine\tests\parx_insp1.rs::gradient_default_and_handles` (Kontur bekommt eigene Vorgabegriffe).
- grün: fp-engine `all` (255) und `--lib`, fp-ui `inspector_parx*`, `parx_brush1_ui`, `inspector_fv15`, `nz1`, `color_picker*`, `inspector_i18n`, `layers*`, `i18n_keine_festen_texte`; `cargo check --workspace --all-targets`, `cargo test -p fp-i18n`, `cargo test -p fp-neutral`.

## Annahmen / offen

- Welche Liste die Live-Filter-„Farbfüllung“ und das Ebenen-Bedienfeld im Original zeigen, ist nicht gemessen; genommen ist die Objekt-Liste. Die Werkzeug-Inspektoren (auch Vektorwerkzeuge) behalten die Pinsel-Namen.
- Die Exporte „Kontur“ und „Supernova“ des Originals tragen sichtbar eine Textur (Ader); verglichen ist deshalb nur die Form. Ohne Textur: Kontur Rand #808080 → Mittellinie Weiß über die halbe Höhe, Mittellinie x 85–115 weiß.
- Die Griffe sind so gewählt, dass der vorhandene (rückwärts entwickelte) Verlaufsmaler das Bild des Originals ergibt; Glanz/Falten/Wellen/Supernova nutzen dort die Diagonale. Ob das Original intern dieselben Griffwerte speichert (JSF `fillHandle1/2/3`), ist nicht gemessen.
- Wiederholtes `pi.fill` mit derselben Art legt in der Engine je Aufruf einen Schritt an (die Oberfläche ruft es bei gleicher Art nicht auf, PAR-X-INSP-1).
- Farbrad: Ist der Farbwähler eines Stopps schon offen, wirkt der Knopf erst beim nächsten Öffnen mit Farbkreis.

## Nachprüfung (echtes Fenster, Brücke)

1. R1 wählen, Mischmodus-Liste aufklappen: „Sprenkeln“, „Ineinanderkopieren“, „Helligkeit“, „Tönung“, „Radieren“, alle Namen ganz lesbar (≥ 132 px); Pinsel-Werkzeug: „Auflösen“ … „Löschen“.
2. JSF `fw.selection[0].pathAttributes.brushPlacement` nach innen/mittig/außen → inside/center/outside.
3. R1 „Verlaufsfläche“, jede Art aus „Verlauf:“ wählen, exportieren und gegen `03_*.png`/`stats.txt` vergleichen (10 Arten gleich, Kontur/Supernova in der Form).
4. Verlaufs-Popup: Farbrad links unter dem Schachbrett → Farbwähler mit Farbkreis; Farbe übernehmen ändert den Stopp.
5. „Muster“ → Winkel „0“.
6. Rechteck-Werkzeug ziehen → Verlauf-Bedienfeld „Rechteck“, Ellipse → „Ellipse“.

## Neue Befunde

Keine.

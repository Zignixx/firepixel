# docs\parity – Vollabgleich Original ↔ Firepixel

Verfahren: `docs\PARITY_SWEEP.md` (verbindlich). Einheiten, Prüfpunkte und Testdateien: `INVENTORY.md`.

## Ordner

| Ort | Inhalt | wer |
|---|---|---|
| `docs\parity\INVENTORY.md` | Einheiten, Prüfpunkte, Testdateien, Abhängigkeiten | PAR-0 |
| `docs\parity\<einheit>.orig.md` | Verhalten des Originals als Text (≤ 150 Zeilen, neutral formuliert) | PAR-O |
| `docs\parity\<einheit>.md` | Vergleichstabelle Prüfpunkt · Original · Firepixel · Bewertung · Befund-ID | PAR-F, PAR-R |
| `docs\parity\img\<einheit>\` | **nur Firepixel-Bilder**, gleiche Namen wie die Referenz | PAR-F, PAR-R |
| `..\re\_testdateien\par\` | Testdokumente T01–T12, `TESTFILES.md` | PAR-O-1 |
| `..\re\_testdateien\par\ref\<einheit>\` | Referenzbilder des Originals + `INDEX.md` | PAR-O |
| `..\re\_testdateien\par\cmp\<einheit>\` | Vergleichsbilder (Original · Firepixel · Differenz) + `summary.md`/`summary.csv` | `compare.py` |
| `tools\parity\compare.py` | Vergleichsskript | – |

Bilder des Originals liegen nie unter `firepixel\` (Neutralität, AGENTEN_REGELN §1a).

## Bildnamen

`<nr>_<kürzel>_<zustand>.png` – `nr` = zweistellige Prüfpunkt-Nr. der Einheit, `kürzel` = Spalte „Kürzel“ in
`INVENTORY.md` (englisch, snake_case), `zustand` ∈ `normal`, `hover`, `pressed`, `disabled`, `open`, `result`, `focus`.
Mehrere Bilder desselben Prüfpunkts und Zustands: Zusatz mit Bindestrich am Kürzel, z. B.
`03_type_result-airbrush_result.png`. Übersicht je Einheit: `00_overview_normal.png`.
Beispiel: `ref\tool-brush\02_stroke_live_pressed.png`. (Technische Namen englisch; das deutsche Beispiel in
`PARITY_SWEEP.md` ist damit ersetzt.)

Aufnahme: Fenster 1600 × 1000, Zoom 100 %, PNG verlustfrei in Originalgröße, Ausschnitt eng um das Prüfziel. Firepixel
gleicher Ausschnitt (gleiche Größe, sonst richtet `compare.py` oben links aus und meldet die Abweichung).

## INDEX.md je Referenzordner

| Nr | Prüfpunkt | Zustand | Bild | zu sehen | gemessene Werte |
|---|---|---|---|---|---|

## Vergleich

```
C:\Users\Administrator\AppData\Local\Programs\Python\Python314\python.exe -I tools\parity\compare.py tool-brush
C:\Users\Administrator\AppData\Local\Programs\Python\Python314\python.exe -I tools\parity\compare.py --all
```

Je gleichnamigem Paar: `<name>` → `cmp\<einheit>\<name>` (links Original, Mitte Firepixel, rechts Differenz verstärkt),
Kennzahlen in `summary.csv`/`summary.md`: Größe beider Bilder, mittlere Abweichung (0–255), Anteil abweichender Pixel
(> Schwelle, Vorgabe 16), größte Abweichung, gleich ja/nein. Bilder ohne Partner werden gelistet. Benötigt Pillow;
numpy wird benutzt, falls vorhanden.

## Bewertung je Prüfpunkt

✅ gleich · ❌ Unterschied in Funktion/Inhalt/Verhalten (beheben, Befund in `BEFUNDE.md`, Bereich PAR) ·
🎨 bewusst anders gestaltet (Quelle angeben, z. B. „DES-1“, „N20“; wird nicht geändert; im Zweifel 🎨 mit Frage).
Die Kennzahl aus `compare.py` ist ein Hinweis, keine Bewertung: Gestaltungsunterschiede (🎨) ergeben große Differenzen.

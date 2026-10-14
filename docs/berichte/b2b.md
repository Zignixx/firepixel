# B-2b – Nachmessungen im Original mit Mess-Sitzung

Stand 14.10.2026 spät, B-2b. Freigabe der Mess-Sitzung durch den Nutzer (14.10. 21:12). Original: Oberflächen-Schritte und JSF
unter der systemweiten Sperre (`gui-sitzung.ps1`, je Gruppe ein Aufruf, Sperre dazwischen frei für RUN-5), echte Eingaben nur nach
Prüfung des Vordergrundfensters (Prozess + Fensterklasse des Originals), Dokumente nur per `fw.openDocument(url)` bzw. per
JSF-Funktion zum Anlegen eines Dokuments, Fragen mit Nein/Abbrechen. Firepixel: `Firepixel-Vorschau-2026-10-14_21-48` (Kopie,
eigenes Profil, Brücke `run_script`/`ui_*`). Messskripte, Bilder und Ausgaben: `re\_testdateien\par\b2b\` (Präfix `orig_`/`fp_`).

Bewertung: **gleich** → abhaken · **Unterschied** → Soll-Regel, Codestelle, Paket · **nicht messbar** → Grund.

## Gruppe 1 – JSF-Messpunkte (BA-119, BA-122, BA-123, BA-124)

Skript `b2b_j1.jsf` (unverändert in beiden Programmen).

| BA | Original | Firepixel | Bewertung |
|---|---|---|---|
| BA-119 | *In Bitmap konvertieren* (transparente Leinwand): grünes Rechteck + rotes mit Deckkraft 0 → Ergebnis 20,20–100,100, das rote fällt ganz weg; blaues Rechteck Deckkraft 50 allein → Pixel `0,0,255,128`, Objekt-Deckkraft danach **100**; gelb deckend + blau 50 % → Mischpixel `127,127,128,255` | Deckkraft 0 wird deckend gerastert (20,20–140,140, rote Pixel), Deckkraft 50 → Pixel `0,0,255,255`, keine Mischung | **Unterschied.** Soll: Rastern **backt Deckkraft (und Mischmodus) jedes Objekts in die Pixel ein**, das Ergebnis hat Deckkraft 100; Objekte mit Deckkraft 0 tragen nichts bei (danach Beschnitt auf Alpha > 0 wie BA-013). Codestelle `crates\fp-engine\src\render_backend.rs:20` bzw. `crates\fp-ui\src\canvas\render.rs:61` `render_elements` (Aussehen inkl. Deckkraft/Mischmodus rendern). Paket **B3** |
| BA-122 | `addNewHotspot("slice", …)`: `name` = `null` (typeof object), auch beim zweiten Segment | `null` | **gleich** – der Name „_r2_c2“ im Inspektor ist der automatische Exportname, nicht `name`. Abhaken |
| BA-123 | `reorderLayer(<Unterebene>, <Ebene der obersten Stufe>, false)`: die Unterebene **verlässt ihre Elternebene** und wird Ebene der **obersten Stufe direkt unter dem Ziel** (flach vor dem Ziel). Beispiele: A1 → vor B: `… A, A2^A, A1, B`; A1 → vor „Ebene 1“: `A1, Ebene 1, A, A2^A`; A2 → vor die eigene Elternebene A: `A2, A, A1^A`. `currentLayerNum` danach = Index der verschobenen Ebene | A1 wird **Unterebene des Ziels** (B bzw. Ebene 1; bei A bleibt sie in A) | **Unterschied.** Soll: Ziel = Ebene der obersten Stufe → die Unterebene wird eigene Ebene der obersten Stufe unmittelbar unter dem Ziel, aktiv. Codestelle `crates\fp-script\src\imp\doc2.rs:120` `move_sublayer` (bzw. `fp-engine docops.rs:402`). Paket **B5** |
| BA-124 | Dokument mit Einheit Pixel/cm (`units:"cm"`): `resolution` bleibt 28,35 / 50 / 59,06, `resolutionUnits` = `cm`; pHYs in FW-PNG = **2800 / 5000 / 5900** (Einheit Meter) | Dokument wird auf dpi umgerechnet (`units` = `inch`, 72,009 / 127 / 150,01), pHYs 2834 / 5000 / 5905 | **Unterschied.** Soll: Einheit Pixel/cm bleibt im Dokument erhalten; pHYs = `trunc(px/cm) · 100` (bei dpi weiter `floor(trunc(dpi)/0,0254)`). Codestelle `crates\fp-fwpng\src\png.rs:294` `make_phys` (Zweig Einheit cm) und JSF-Dokumentanlage (`units:"cm"`, siehe BA-125). Paket **B5** |

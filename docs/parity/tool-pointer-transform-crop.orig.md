# 08 `tool-pointer-transform-crop` – Zeiger, Unterauswahl, Hintere auswählen, Skalieren, Neigen, Verzerren, Zuschneiden, Exportbereich: Aufnahme des Originals (PAR-O-08-10, 13.10.2026 ≈ 07:25–08:30)

Testdateien T04, T02 (Arbeitskopien `re\_testdateien\par\work\p810_t0*.fw.png`, je Messung frisch mit `fw.openDocument(url)`).
Fenster 1600 × 1000, Zoom 100 %, Leinwand bei Bildschirm 297,183. **Echte Mauszüge** (mouse_event, 30 Schritte), Zustände danach per
Skript (Objektbaum, `pixelRect`, Pfadknoten `x,y[pred|succ]`). Referenzbilder `re\_testdateien\par\ref\tool-pointer-transform-crop\`
(+ `INDEX.md`), Exporte/Hashes `re\_testdateien\par\px\tool-pointer-transform-crop\pixels.csv`, alle Knoten `paths.json`.
Art: M = echter Mauszug/Taste, J = Skriptwert, W = Fenstertext Inspektor, E = Export + Hash, B = Bild.

**Werkzeugnamen im Original:** „Zeiger“, „Teilauswahl“, „Hintere auswählen“, „Skalieren“, **„Neigen“** (= Verzerren der Liste),
**„Verzerren“** (= Verkrümmen der Liste), „Zuschneiden“, „Exportbereich“. In T04 sind R1/R2 gewöhnliche Pfade (keine Rechteck-Grundform).

## Prüfpunkte

| Nr | Kürzel | Original | Art |
|---|---|---|---|
| 01 | `pointer_click` | Maus über nicht gewähltem Objekt → Hover-Umriss. Klick wählt (gewählter Pfad: **4 blaue Eckpunkte** an den Knoten). Klick auf leere Fläche hebt auf. Umschalt+Klick fügt hinzu. Gefüllte Fläche (E1) per Klick ins Innere wählbar; offener Pfad P1 ohne Füllung: Klick 250,175 (nahe Kurve) wählt, 240,182 (≈ 10 px daneben) nicht | M, J, B `01` (2) |
| 02 | `pointer_rubber` | Gummiband **wählt alles, was berührt wird** (nicht nur Umschlossenes): 30,30→100,100 wählt R1 (nur teilweise drin); 30,30→320,260 wählt R1, R2, E1 und P1 (P1 ragt bis 421 hinaus) | M, J |
| 03 | `pointer_drag` | Ziehen R1 (+40,+10) → 90,60–190,130, Schritt **„Verschieben“**. Umschalt +60,+20 → **+60,0** (waagrecht); Umschalt +40,+40 → **+45,+45** (45°, Ende rastet zusätzlich an Objektkante/-mitte). **Alt-Ziehen = Kopie**: Kopie liegt direkt **über** dem Original (gleicher Name „R1“), Original bleibt, Schritt **„Duplizieren“**; Alt vor oder nach dem Drücken gleich. Während des Zugs nur Umriss-Vorschau | M, J, B `03` (2) |
| 04 | `pointer_corner` | Zeiger am Eckpunkt eines gewählten Pfads ziehen = **Objekt verschieben** (kein Größenändern; R1 +40,+30 → 90,80–190,150, „Verschieben“). Ziehen am Knoten von P1 verschiebt ebenso den ganzen Pfad (+0,+30 → +35, rastet an E1-Mitte y 205) | M, J, B `04` |
| 05 | `nudge` | → = +1 px, Umschalt+→ = +10 px, ↓↓ = +2 px; Schritt „Verschieben“ | M, J |
| 06 | `subselect` | Teilauswahl-Klick auf Knoten 290,200 von P1 wählt den Pfad, Punkte sichtbar, gewählter Punkt mit Henkeln. Knoten ziehen (+0,+30) → nur dieser Knoten + seine Henkel verschoben (290,230 [260,190 \| 320,270]); Henkel ziehen 320,270→340,290 → succ 340,290, **pred spiegelt mit** (257.99,191.59 – gleiche Richtung, Länge bleibt). Schritt **„Punkt verschieben“**. Klick auf R1 zeigt 4 Eckpunkte | M, J, B `06` (2) |
| 07 | `subselect_group` | Teilauswahl-Klick auf Stern-Gruppe ST wählt den **inneren Pfad** (Name null, 10 Knoten); Zeiger-Klick wählt die Gruppe | M, J |
| 08 | `select_behind` | R2 auf R1 geschoben (80,60–180,130). „Hintere auswählen“ an derselben Stelle: 1. Klick R2 (oberstes), 2. Klick **R1**, 3. Klick wieder R2 (zyklisch). Zeiger: zweimal Klick bleibt R2 | M, J |
| 09 | `scale_handles` | Skalieren (Werkzeugknopf, Auswahl R1): **8 schwarze quadratische Griffe** + runder Drehpunkt in der Mitte, dünner schwarzer Rahmen. Werkzeug per Skript allein aktiviert die Griffe nicht (bleibt „Zeiger“); erst Knopfklick im Werkzeugbedienfeld | M, B `09` |
| 10 | `scale_shift` | Eckgriff 150,120→200,140 (ohne Taste) → **proportional** 50,50–193,150 (Projektion auf die Diagonale; Faktor 1,43). **Umschalt gleich** (schon proportional). **Alt = von der Mitte** → 7,20–193,150 (Mitte 100,85 bleibt). Umschalt+Alt = wie Alt. Seitengriff 150,85→200,85 → nur Breite 150 (50,50–200,120). Ziehen innen = Verschieben (90,60). Schritt **„Transformieren“** | M, J, B `10` (2) |
| 11 | `rotate` | Ziehen außerhalb (170,140→100,160) dreht um den Drehpunkt **≈ 52°** (= Winkeldifferenz zur Mitte). **Umschalt rastet in 15°-Schritten** (→ 45°; kleiner Zug 170,140→168,150 → 0°). Ergebnis bleibt Pfad mit gedrehten Knoten | M, J, B `11` (2) |
| 12 | `pivot` | Drehpunkt 100,85 → 50,50 ziehen, dann Umschalt-Drehen → Knoten 50,50 bleibt, Drehung 30° | M, J, B `12` |
| 13 | `skew` | „Neigen“: Mittelgriff oben +30 → **nur obere Kante** verschoben (80,50 / 180,50). Eckgriff 150,50 senkrecht −20 → **gegenüberliegende Ecke derselben Kante spiegelbildlich** (150,30 / 150,140); Eckgriff waagrecht +20 → obere Kante symmetrisch verbreitert (30,50 / 170,50) | M, J, B `13` (2) |
| 14 | `distort` | „Verzerren“: Eckgriff 150,50→170,30 bewegt **nur diese Ecke**; Mittelgriff oben (+20,−10) verschiebt beide oberen Ecken gleich | M, J, B `14` |
| 15 | `transform_enter_esc` | **Enter übernimmt**; **Esc verwirft** (Objekt unverändert, kein Verlaufsschritt); **Doppelklick innen und außen übernimmt**; Klick auf Zeiger-Knopf übernimmt. Skriptlauf beendet den Modus ohne Übernahme | M, J |
| 16 | `transform_bitmap` | B1 (T02), Eckgriff 520,360→600,420 → 40,40–603,415 (proportional), neu berechnet (geglättet, 8 892 Farben); Seitengriff unten → 40,40–520,200; Umschalt-Drehen → 7,−17–553,417. Schritt „Transformieren“ | M, E, J, B `16` |
| 17 | `crop_frame` | Zuschneiden ohne Auswahl, Rahmen 100,100→500,400: außen **abgedunkelt**, 8 Griffe; Inspektor „Dokument“ zeigt **B 400 H 300 X 100 Y 100** live | M, W, B `17` |
| 18 | `crop_apply` | **Enter** → Dokument 400 × 300, Objekte −100,−100 verschoben, **Objekte ganz außerhalb werden gelöscht** (ST; bei Rahmen −50,−50→300,200 auch L1, PG), danach **alle Objekte gewählt**, Werkzeug bleibt „Zuschneiden“; Schritt **„Dokument zuschneiden“**. **Esc** verwirft. **Doppelklick innen** übernimmt. Griff 500,400→600,450 vergrößert auf 500 × 350. Rahmen über den Rand hinaus wird auf die Leinwand begrenzt (→ 300 × 200) | M, J, B `18` |
| 19 | `crop_bitmap` | T02, B1 gewählt, Rahmen 100,100→300,300 + Enter: **schneidet das Dokument** (200 × 200), B1 danach 0,0–200,200, Pixel = Ausschnitt von B1 (Hash gleich T02[100:300,100:300]) – kein eigenes Bitmap-Zuschneiden | M, E, J, B `19` |
| 20 | `export_area` | Exportbereich-Rahmen 50,50→300,200 (Inspektor B 250 H 150 X 50 Y 50). **Doppelklick innen öffnet „Bildvorschau“** (Format PNG 32, Register Datei: „Exportbereich“ ☑, X 50 Y 50 B 250 H 150, Skalieren 100 % 800 × 600); Esc schließt, Dokument unverändert | M, W, B `20` (2) |
| 21 | `inspector_values` | Mitten im **Zeiger-Zug** eines vorher nicht gewählten Objekts zeigt der Inspektor noch **„Dokument“**, danach „Pfad“ R1 100 × 70 bei 90,60. Mitten im **Skalieren** bleiben B/H/X/Y auf den alten Werten (kein Live-Wert). Beim Zuschneiden/Exportbereich live (s. 17, 20) | M, W, B `21` |
| 22 | `cursor` | Laut Zeiger-Handle: Zeiger leer = **über Objekt gleich** (Pfeil, Hotspot 4,3; kein eigener Hover-Zeiger), auch über dem Drehpunkt. Eigene Zeiger: Teilauswahl über Punkt (8,8), Zeiger auf Punkt eines gewählten Objekts (4,4), Skalieren Ecke (13,12), Seite (9,7), innen/Verschieben (15,15), Drehen außen (5,13), Zuschneiden Griff (15,15) und innen (14,15; Exportbereich innen gleich) | B `22` (8) |
| 23 | `snap` | Objekt-Fang beim Ziehen (Vorgabe an): R1 +47 → rastet auf +50 (rechte Kante an R2-Kante 200); +44,+3 → +44,**0** (Oberkante an R2-Oberkante); Fangweite ≈ 3–5 px | M, J |
| 24 | `undo` | „Verschieben“ (Zeiger, Pfeiltasten, Eckpunkt), „Duplizieren“ (Alt-Ziehen), „Punkt verschieben“ (Teilauswahl), „Transformieren“ (Skalieren/Drehen/Neigen/Verzerren, auch Bitmap), „Dokument zuschneiden“; Esc im Transformieren: kein Schritt | M, W |

## Hinweise

- Ohne Auswahl zeigen Zeiger, Teilauswahl, Hintere auswählen, Skalieren, Neigen und Verzerren den Dokument-Inspektor (Leinwand, Exportformat PNG32, Status); Zuschneiden und Exportbereich zusätzlich leere Felder B/H/X/Y.
- Ein anderes Programmfenster holte einmal den Vordergrund; der Lauf brach sicher ab und wurde wiederholt.
- Die Zustände „Rückgängig: …“ wurden aus dem Menü Bearbeiten gelesen (ohne auszuführen).

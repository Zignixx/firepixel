# 07 `tool-fill-gradient-eyedropper` – Farbeimer, Verlauf, Pipette: Aufnahme des Originals (PAR-O-05-07, 13.10.2026 ≈ 07:35–07:55)

Testdateien T02, T04 (Arbeitskopien `re\_testdateien\par\work\p57_t0*.fw.png`, je Messung frisch mit `fw.openDocument(url)`).
Fenster 1600 × 1000, Zoom 100 %, Leinwand bei Bildschirm 297,183. Werkzeug per Skript (`Tools.paintBucket` – heißt im
Werkzeugobjekt und in der Inspektor-Überschrift **„Füllwerkzeug“** –, `Tools.gradient` „Verlauf“, `Tools.eyedropper` „Pipette“),
Füllung #808080, Strich #C04040, B1 gewählt. Farben per Skript gelesen (`pathAttributes.fillColor/brushColor`, Füllart, Verlaufsknoten,
Griffe). Referenzbilder `re\_testdateien\par\ref\tool-fill-gradient-eyedropper\` (+ `INDEX.md`), Exporte
`re\_testdateien\par\px\tool-fill-gradient-eyedropper\pixels.csv`. Messart W/J/E/B/M wie Einheit 05.

## Prüfpunkte

| Nr | Kürzel | Original | Art |
|---|---|---|---|
| 01 | `bucket_inspector` | Links: Füllsymbol + **Füllfarbfeld** (216–239) und **4 Bildknöpfe Füllart** (276–372: Keine / **Vollton** gedrückt / Verlauf / Muster). **„Rand:“** Liste **Hart / Anti-Alias / Weiche Kte.** (Vorgabe **Anti-Alias**) + Wert (grau, 0) + ▼. **„Textur:“** Liste mit **„Ader“** und Wert **0** + ▼, Kästchen „Transparent“ (grau) + Texturknopf (grau). Rechts: **„Toleranz:“** 32 + ▼, Kästchen **„Auswahl füllen“** ☐, **„Transparenz beibehalten“** ☐, Deckkraft 100 + ▼, Mischmodus „Normal“ (55 Einträge) | W, B `01` |
| 02 | `bucket_bitmap` | Klick 160,120 im Verlauf (#8E8881) mit #808080: Toleranz **0** → 7 px (160,119–161,126), **32** → 5 553 px (143,40–179,201), **100** → 55 872 px (40,40–285,360); Klickpixel danach #808080 | M, E, B `02` (3) |
| 03 | `bucket_vector` | T04, R2 gewählt: **Farbwechsel im Füllfarbfeld setzt R2 sofort** auf #3366CC (Dokumentfüllung ebenfalls). Klick auf R2 danach ändert nichts mehr. **Klick auf das nicht gewählte R1** setzt dessen Füllung auf #3366CC (Füllart bleibt Vollton). Verlaufsschritt „Füllen“ | M, J, B `03` |
| 04 | `bucket_fill_selection` | Rahmen 100,80–200,180, Klick 150,130: „Auswahl füllen“ **an** → ganzer Rahmen gefüllt (10 000 px); **aus** → nur toleranzgleiche Pixel im Rahmen (3 580 px, 133,80–169,180) | M, E, B `04` |
| 05 | `bucket_edge` | Klick ins linke Auge (120,268, #DC141E): **Hart** 489 px nur #808080; **Anti-Alias** 547 px (489 exakt + 58 Mischpixel am Kreisrand); **Weiche Kte.** (Wert wird 10) 1 595 px in 99,247–143,291, Kante weich über ≈ 10 px, auch Augeninneres gemischt (nur 1 px exakt #808080) | M, E, B `05` (2) |
| 06 | `gradient_inspector` | Überschrift „Verlauf“. Gleicher Aufbau wie Füllwerkzeug, aber **ohne die 4 Füllart-Knöpfe**; Füllfarbfeld zeigt den **Verlauf** (schwarz → weiß); **Toleranz 32 grau**; **„Auswahl füllen“ ☑** (Vorgabe an); Rand Anti-Alias, Textur Ader 0 | W, B `06` |
| 07 | `gradient_drag_bitmap` | Rahmen 100,80–300,220, Verlauf ziehen 100,150 → 300,150: nur der Rahmen gefüllt (28 000 px), linear #000000 → #FDFDFD (x 100/150/200/250/299 = 00/3F/7F/BF/FD). **Ohne Rahmen** (B1 gewählt): Verlauf füllt die **ganze Leinwand**, B1 wird auf **0,0–800,600** vergrößert, links schwarz, ab x 300 weiß | M, E, J, B `07` |
| 08 | `gradient_drag_vector` | T04, R1 gewählt, Ziehen 60,85 → 140,100: Füllart **fc_Linear**, Knoten #000000@0 / #FFFFFF@100, **Griffe h1 = 60,85 (Start, runder Griff), h2 = 140,100 (Ende, eckiger Griff)**, h3 = 75,5 (Breite, senkrecht, im Bild nicht sichtbar); Linie zwischen den Griffen. Verlaufsschritt **„Füllung bearbeiten“** | M, J, E, B `08` (2) |
| 09 | `gradient_types` | Klick aufs Füllfarbfeld öffnet das Verlaufs-Popup: **„Verlauf:“** Liste, Farbbalken mit Deckkraft-Stopps oben und Farb-Stopps unten, Farbkreis-Symbol, Umkehren-Knopf, Positionsfeld (grau) + %, Winkel 0 + °. Liste (12): **Linear, Radial, Rechteck, Kegel, Kontur, Glanz, Supernova, Falten, Ellipse, Streifen, Kleine Wellen, Wellen** | M, W, B `09` (2) |
| 10 | `eye_inspector` | Überschrift „Pipette“. **„Auswahl:“** Liste **1 Pixel / 3x3 Durchschnitt / 5x5 Durchschnitt** (255–371), **„Farbformat:“** Liste **HEX / RGBA** (459–575). Sonst nichts | W, B `10` (2) |
| 11 | `eye_stroke_fill` | Ohne Auswahl, Füllfarbfeld aktiv: Klick auf Rot (380,120) → **Füllfarbe #FF0000**, Strich unverändert; nach Klick aufs Strich-Symbol im Werkzeugbedienfeld: Klick auf Grün (420,120) → **Strichfarbe #00A000**. Mit gewähltem Vektor R1 (T04) und Klick auf R2 (50 % Deckkraft über Weiß): **R1 bekommt #BFBFBF** (Mischfarbe der Ansicht). Verlaufsschritt „Füllfarbe“ | M, J |
| 12 | `eye_average` | **Kein Mittelwert:** 3x3 bei x 360 (erste rote Spalte) → **#FFFFFF**, bei 361 → #FF0000, bei 359 → #FFFFFF; 5x5 bei 361 → #FFFFFF, bei 362 → #FF0000; 5x5 im Verlauf 100,100 → #5462BA (1 Pixel: #5664B8). Ergebnis = **linkes oberes Pixel des Fensters** | M, J |
| 13 | `eye_outside` | Klick auf den grauen Bereich neben der Leinwand: Farbe bleibt (keine Änderung, keine Meldung). Klick auf leere weiße Leinwand → #FFFFFF | M, J |
| 14 | `cursor` | Eigene Zeiger je Werkzeug (eingezeichnet über GetCursorInfo) | B `14` (3) |
| 15 | `texture_none` | Textur ohne Auswahl zeigt den **Namen „Ader“ mit 0 %** (nicht „Keine“); gilt für Füllwerkzeug und Verlauf | W |
| 16 | `undo` | Farbeimer-Klick = **„Füllen“** (Bitmap und Vektor), Verlauf in Bitmap-Rahmen = **„Füllen“**, Verlauf auf Vektor = **„Füllung bearbeiten“**, Pipette = **„Füllfarbe“** | M, W |

## Hinweise

- Nach dem Verlauf auf R1 und Wechsel zum Zeiger per Skript schloss das Steuerskript einen leeren Meldungsdialog (Text nicht lesbar).
- Die Füllfarbe des Originals wurde am Ende per Skript wieder auf #808080 gesetzt.

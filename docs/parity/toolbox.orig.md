# 15 `toolbox` – Werkzeugleiste, Ausklapplisten, Hand, Zoom, Ansichtsmodi: Aufnahme des Originals (PAR-O-14-15-19, 13.10.2026 ≈ 14:05–14:25)

T10 als Arbeitskopie `re\_testdateien\par\work\p1415_t10.fw.png` (2400 × 1600). Fenster 1600 × 1000, Zoom 100 %. **Echte Maus/Tastatur**;
aktives Werkzeug per Skript (`fw.activeTool`), Zoom aus der Statuszeile des Dokuments (Feld „100%“), Bildlauf aus Bildschirmfotos der Ansicht
(Phasenkorrelation, Pixel genau). Bilder `re\_testdateien\par\ref\toolbox\` (+ `INDEX.md`), Werte `re\_testdateien\par\px\toolbox\pixels.csv`.
Art: M = echte Eingabe, J = Skriptwert, W = Fenstertext/Tooltip, B = Bild.

## Prüfpunkte

| Nr | Kürzel | Original | Art |
|---|---|---|---|
| 01 | `groups` | Zweispaltig, Gruppenüberschriften: **Auswählen** (Zeiger · Teilauswahl / Skalieren · Zuschneiden), **Bitmap** (Auswahlrahmen · Lasso / Zauberstab · Pinsel / Bleistift · Radiergummi / Weichzeichner-Gruppe · Stempel-Gruppe), **Vektor** (Linie · Stift / Rechteck-Gruppe · Text / Freiform-Gruppe · Messer), **Web** (Hotspot · Segment / Ausblenden · Einblenden), **Farben** (Pipette · Farbeimer/Verlauf / Strichfarbe / Füllfarbe / 3 kleine Knöpfe), **Ansicht** (3 Ansichtsmodi / Hand · Zoom) | B, W `01` |
| 02 | `flyouts` | 12 Knöpfe mit Ausklappliste (Maus gedrückt halten ≈ 1 s; Liste rechts neben dem Knopf, gewählter Eintrag mit Haken): **Zeiger**: Zeiger, Hintere auswählen (V, 0) · **Zuschneiden**: Zuschneiden, Exportbereich (C) · **Auswahlrahmen**: Auswahlrahmen, Ovaler Auswahlrahmen (M) · **Lasso**: Lasso, Polygon-Lasso (L) · **Weichzeichner** (R): Weichzeichner, Scharfzeichner, Abwedler, Nachbelichter, Verwischen · **Stempel** (S): Stempel, Farbe ersetzen, Rote Augen entfernen · **Stift** (P): Stift, Vektorpfad, Pfad nachzeichnen · **Rechteck** (U): Rechteck, Ellipse, Polygon, Trenner, Abgefastes Rechteck, Abgeflachtes Rechteck, Abgerundetes Rechteck, Donut, Intelligentes Polygon, L-Form, Messwerkzeug, Pfeil, Pfeillinie, Spirale, Stern, Torte, Verbindungslinie (automatische Formen ohne Kürzel) · **Freiform** (O): Freiform, Bereich umformen, Pfadausbesserung - additiv, Pfadausbesserung - subtraktiv · **Hotspot** (J): Hotspot-Rechteck, Hotspot-Kreis, Hotspot-Polygon · **Segment** (K): Segment, Polygon-Segment · **Farbeimer** (G): Füllwerkzeug, Verlauf. **Ohne** Liste: Teilauswahl, Skalieren, Zauberstab, Pinsel, Bleistift, Radiergummi, Linie, Text, Messer, Pipette, Hand, Zoom. Gewählter Eintrag ersetzt das Symbol des Knopfs | M, W, B `02` (12) |
| 03 | `tooltips` | „Name (Kürzel)“, Gruppenknopf zeigt das zuletzt gewählte Werkzeug: Zeiger (V, 0), Teilauswahl-Werkzeug (A, 1), Skalieren (Q), Zuschneiden (C), Auswahlrahmen (M), Lasso (L), Zauberstab (W), Pinsel (B), Bleistift (B), Radiergummi (E), Scharfzeichner (R), Stempel (S), Linie (N), Stift (P), Rechteck (U), Text (T), Pfadausbesserung - additiv (O), Messerwerkzeug (Y), Hotspot-Polygon (J), Segment (K), Segmente und Hotspots ausblenden (2), … anzeigen (2), Pipette (I), Verlauf (G); Strich-/Füllfarbe: Hexwert („#C04040“, „#808080“); „Standardfarben für Pinselstrich und Füllung anwenden (D)“, „Keine Pinselstriche oder Füllungen“, „Pinselstrich-/Füllfarben austauschen (X)“; „Standardbildschirmmodus (F)“, „Vollbildmodus mit Menüs (F)“, „Vollbildmodus (F)“; Hand (H), Zoom (Z). Erscheinen nach ≈ 1,5 s | M, W, B `03` (3) |
| 04 | `pressed` | Gewähltes Werkzeug eingedrückt (heller Rahmen) | B `04` |
| 05 | `letter_cycle` | Gleicher Buchstabe wiederholt → nächstes Werkzeug der Gruppe, zyklisch, Start beim gerade aktiven: M Auswahlrahmen ↔ Ovaler; L Lasso ↔ Polygon-Lasso; R Weichzeichner → Scharfzeichner → Abwedler → Nachbelichter → Verwischen → …; U **nur** Rechteck → Ellipse → Polygon (keine automatischen Formen); O Freiform → Bereich umformen → Pfadausbesserung → Pfadausbesserung (additiv/subtraktiv per Skript nicht unterscheidbar); J Rechteck → Kreis → Polygon; K Segment ↔ Polygon-Segment; G Füllwerkzeug ↔ Verlauf; S Stempel → Farbe ersetzen → Rote Augen; P Stift → Vektorpfad → Pfad nachzeichnen; V Zeiger ↔ Hintere auswählen; C Zuschneiden ↔ Exportbereich; **B** Pinsel ↔ Bleistift. Q (Skalieren) und Y (Messer) **ohne Wirkung** ohne Auswahl; W, E, N, T, A, I, H, Z direkt | M, J |
| 06 | `hand` | Hand ziehen (−100, −80) → Inhalt genau (−100, −80); **Leertaste gedrückt + Ziehen** mit Zeiger → Bildlauf 1:1, Werkzeug bleibt Zeiger | M, J |
| 07 | `hand_dblclick` | Doppelklick auf Hand-Knopf → **einpassen** (2400 × 1600 → 42 %) | M, W |
| 08 | `zoom_click` | Zoomwerkzeug Klick: 100 → 150 → 200 → 300 → 400 → 600 %; Alt+Klick: 600 → 400 → 300 → 200 → 150 → 100 → 66 → 50 % | M, W |
| 09 | `zoom_rect` | Zoomrahmen 200 × 150 Bildschirmpixel aufziehen → **476 %** (füllt die Ansicht, nicht auf Stufe gerundet) | M, W |
| 10 | `zoom_dblclick` | Doppelklick auf Zoom-Knopf → **100 %** | M, W |
| 11 | `ctrl_space` | Strg+Leertaste+Klick mit Zeiger → **keine** Zoomänderung (100 %), Werkzeug bleibt Zeiger | M, W |
| 12 | `wheel` | Mausrad 1 Raste runter → Inhalt **48 px** nach oben; Umschalt+Rad → **keine** Bewegung; **Strg+Rad** hoch → Zoom 100 → 150 %; Alt+Rad → keine Zoomänderung | M, W |
| 13 | `view_modes` | Knopf „Vollbildmodus mit Menüs“ → Fenster füllt den Bildschirm (2560 × 1440) mit Menü und Bedienfeldern; **F** → Vollbildmodus (ohne Menüleiste/Titel, rechte Bedienfelder ausgeblendet, Werkzeugleiste und Inspektor bleiben); **F** → zurück zum Standardmodus (1600 × 1000 wiederhergestellt) | M, B `13` (3) |
| 14 | `color_section` | Farben-Gruppe: Pipette, Farbeimer-Gruppe, Strichfarbe (Stiftsymbol + Feld), Füllfarbe (Eimersymbol + Feld), Standard/Ohne/Tauschen → Einzelheiten Einheit 31 | B `14` |
| 15 | `cursor` | Hand: offene Hand (Hotspot 13,13); Zoom: Lupe + (8,12); mit Alt: Lupe − (8,12) | B `15` (3) |
| 16 | `collapse` | Klick auf » oben links an der Werkzeugleiste: **keine sichtbare Änderung**; einspaltig/Lösen nicht gemessen (Annahme: Fensteranordnung des Programms, nicht per Klick erreichbar) | B `16` |
| 17 | `ai_tools` | – (im Original nicht vorhanden; 🎨 nur Firepixel) | – |
| 18 | `disabled` | Ohne Dokument: Werkzeugsymbole anders/grau (6211 px Unterschied); Klick auf Werkzeug und Tastenkürzel (B) **ohne Wirkung**, aktives Werkzeug bleibt Zeiger | M, J, B `18` |

## Hinweise

- Vor einer Messreihe war „Pfadausbesserung - additiv“ in der Freiform-Gruppe und „Verlauf“ in der Farbeimer-Gruppe zuletzt gewählt (gespeicherter Stand).
- Ausklapplisten sind owner-drawn (`#32768` ohne Text); Einträge wurden einzeln gewählt und per Tooltip gelesen.

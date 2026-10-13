# 11 `tool-autoshapes` – automatische Formen und Griffe: Aufnahme des Originals (PAR-O-11-13, 13.10.2026 ≈ 08:15–13:20)

Testdatei T01 als Arbeitskopie `re\_testdateien\par\work\p1113_t01.fw.png`, je Messung frisch mit `fw.openDocument(url)`. Fenster
1600 × 1000, Zoom 100 %, Leinwand bei 297,183, Füllung #808080, Strich #C04040 1 px. Form per **echter Maus** aus der Ausklappliste
gewählt, **echte Züge**; Werte per Skript (`controlPoints`, `customData`, Knoten). Referenzbilder `re\_testdateien\par\ref\tool-autoshapes\`
(+ `INDEX.md`), Exporte `re\_testdateien\par\px\tool-autoshapes\pixels.csv`, alle Knoten und Griffe `paths.json`.
Art: M = echte Maus/Tastatur, J = Skriptwert, W = Fenstertext, E = Export/Hash, B = Bild.

## Prüfpunkte

| Nr | Kürzel | Original | Art |
|---|---|---|---|
| 01 | `flyout` | Ausklappliste am Formen-Knopf (Halten): Rechteck (U), Ellipse (U), Polygon (U), Trennlinie, dann **13** Autoformen alphabetisch: Abgefastes Rechteck, Abgeflachtes Rechteck, Abgerundetes Rechteck, Donut, Intelligentes Polygon, L-Form, **Messwerkzeug**, Pfeil, Pfeillinie, Spirale, Stern, Torte, Verbindungslinie (ohne Kürzel). Aktives Werkzeug mit Haken; Knopf-Tooltip = Name der zuletzt gewählten Form | M, W, B `01` |
| 02 | `draw_each` | Gleicher Zug je Form, Ergebnis = **Gruppe mit Autoform-Daten**, gewählt, Werkzeug bleibt. Kinder/Knoten: Pfeil 1 Pfad geschlossen 11 Kn. · Pfeillinie 3 offene Pfade (Linie 2 Kn. + Spitzen), **eigener Strich #000000 „Hard Line“** · Abgefast/Abgeflacht/Abgerundet je 8 Kn. (Eckenart 2/0/1, Ecke 25) · Verbindungslinie offen 6 Kn., keine Füllung · Donut 2 Konturen à 4 (Radius 60/24), **quadratisch 120×120** · L-Form 8 Kn. (Dicke 33,3) · Torte 2 Pfade (Keil 4 + Kreis 8), 120×120 · Intelligentes Polygon 7 Seiten, Radius 60, 101,100–219,215 · Spirale offen 27 Kn., 6 Windungen, keine Füllung, 113,124–204,214 · Stern 5 Punkte, Radien 60/30, 102,100–218,209 · Messwerkzeug: Bemaßung mit Beschriftung, danach **nicht gewählt**. Hashes in pixels.csv | M, J, E, B `02` (26) |
| 03 | `handles` | Gelbe Rauten an `controlPoints`; Lage je Form in paths.json (z. B. Abgerundet: 125,100 / 195,100 / 195,200 / 125,200 + Größe 230,210 = rechts unten + 10). Unsichtbare Hilfsgriffe bei Spirale (1) und Stern (5) | J, B `03` (6) |
| 04 | `handle_tooltip` | Tooltip-Texte (aus den Griffdaten): Ecken „Klicken, um zwischen den Ecken umzuschalten“ / „Alt-/Wahltaste drücken, um einzelne Ecken auszuwählen“, „Größe ändern“; Stern „Radius 1/2“, „Rundung 1/2“, „Punkte: 5“; Polygon „Inneres Polygon“, „Skalieren/Drehen“, „Seiten: 7; Alt-/Wahltaste drücken, um Polygon zu teilen“, „Abschnitte: 7“; Spirale „Spirale öffnen/schließen“, „Spiralen: 6“; Pfeil „Rundheit“, „Dicke“, „Breite/Höhe“, „Höhe/Breite“, „Pfeilgröße“, „Pfeilspitze“; Torte/Donut „Klicken, um Einstellungen zurückzusetzen“, „Innerer Radius“, „Alt-/Wahltaste drücken, um einzelne Segmente zu verschieben“; Pfeillinie „Spitze: 1 von 5“; Verbindungslinie „Klicken, um zu drehen“, „1“, „2“. Zahlen im Tooltip folgen dem Wert | J, B `04` (2) |
| 05 | `handle_drag` | Nur mit **Zeiger** (mit aktivem Autoform-Werkzeug zieht ein Zug auf einem Griff eine **neue** Form auf). Abgerundet: Eckgriff 20 nach links → alle Ecken 25 → 45 (gekoppelt); Größe-Griff +30,+30 → 150×130. Stern: Punkte-Griff **senkrecht** +40 → 3 Punkte, −80 → 34 Punkte (waagrecht: keine Änderung); Radius-1-Griff 20 hoch → Radius 60 → 80. Spirale: Windungen-Griff +40 → 3, −70 → 11. Torte: Segmentgriff +40 → Keil geändert; Donut: Innenradius-Griff +20 → Griff 204,160. Pfeil: Spitze +30 → Griff 250,150; „Dicke“ senkrecht +10 → keine Änderung | M, J, B `05` (2) |
| 06 | `handle_modifiers` | **Klick** auf Eckgriff schaltet alle Ecken 1 → 2 → 0 → 1 (rund → gefast → flach). **Alt+Klick** schaltet nur diese Ecke. **Alt+Ziehen** am Eckgriff ändert nur diese Ecke (45 → 50). Strg+Ziehen wirkt wie normales Ziehen (Ecke 25 → 35). Umschalt+Ziehen am Größe-Griff: **keine** Seitenverhältnis-Sperre (+30,0 → 180×130). Polygon: Klick auf Innen-Griff setzt „Inneres Polygon zurück“. Torte: Alt+Ziehen fügt Segment hinzu (6 Griffe), Klick Mitte = zurücksetzen (voller Kreis) | M, J |
| 07 | `inspector_head` | Kopf links Typ = Formname („Stern“, „Pfeil“, …), Namensfeld zeigt „<Form> Automatische Form“; B/H/X/Y = Grenzen (Pfeil 120/100/100/100, Donut/Torte 120/120, Stern 114/109/103/100); Füll-/Strichteil wie Pfad | W, B `07` |
| 08 | `properties_panel` | Fenster ▸ „Eigenschaften für automatische Form“: schwebendes Bedienfeld, links Hilfetext, rechts Felder der gewählten Form, synchron (Stern: Punkte 5, Radius 1 60, Radius 2 30, Rundheit 1 60, Rundheit 2 30). Gleicher Befehl schließt es wieder | M, B `08` |
| 09 | `ungroup` | Strg+Umschalt+G auf Stern: **ohne Rückfrage** normaler Pfad (geschlossen, 10 Knoten), Autoform-Daten weg; Schritt „Gruppierung aufheben“; Pixel identisch | M, J, E |
| 10 | `scale` | B-Feld 114 → 240: bleibt Autoform, `customData` unverändert (Radius 60), Griffe mit der Transformation verschoben/gestreckt; Schritt „Transformieren“ | M, J |
| 11 | `connector` | Verbindungslinie von Rechteckkante 150,340 zu 400,340: offener Pfad, Mitte-Griff 275,340 („Klicken, um zu drehen“). **Keine Bindung**: zweites Rechteck verschoben → Linie bleibt | M, J, B `11` |
| 12 | `smart_polygon` | Seiten-Griff senkrecht +40 → 3 Seiten, −60 → 14 (Abschnitte folgen); Alt+Klick → kein sichtbarer Wechsel gemessen; Skalieren/Drehen-Griff +30 → Polygon gedreht | M, J |
| 13 | `spiral` | Windungen per Griff (s. 05); Klick auf Mitte schaltet offen ↔ geschlossen (`name` open/closed) | M, J |
| 14 | `shift` | Umschalt beim Aufziehen: Abgerundetes Rechteck **nicht** quadratisch (120×60 bleibt), Stern unverändert, Pfeil 120×120 (quadratisch) | M, J, E |
| 15 | `cursor` | Werkzeug: Fadenkreuz (Hotspot 10,9). Zeiger über Griff (Zeigerwerkzeug): eigener Griffzeiger, Hotspot 4,4 | B `15` (2) |
| 16 | `undo` | Aufziehen: „Form“ (ein Schritt; Rückgängig entfernt die Form ganz). Griff ziehen/klicken: „Automatische Form bearbeiten“. Gruppierung aufheben, Transformieren, Verschieben wie üblich | M, W |

## Hinweise

- Einzelklick mit Autoform-Werkzeug fügt die Form in **Vorgabegröße** ein, Klickpunkt = linke obere Ecke (Abgerundet 200 × 150, Ecke 37,5; Stern Radius 100).
- Abgefastes/Abgeflachtes/Abgerundetes Rechteck sind dieselbe Form („rectangle“) mit Eckenart 2/0/1 (Ecke 25 = ¼ der kürzeren Seite).
- Polygon: nach Seiten-Ziehen stand in den Formdaten `nodes` kurz der Tooltip-Text (Fehler des Formskripts, Form selbst richtig).

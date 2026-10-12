# PERF-1: Alles auswählen auf Seiten mit Tausenden Objekten

Befund (BEFUNDE HG/LW, „von FV-8“): Strg+A auf einer Korpusseite mit Tausenden Objekten kostete 180–220 ms Oberflächenzeit je Bild, die Pfeiltasten deshalb 250–430 ms. Ziel ≤ 50 ms je Bild, auch auf dem Software-Rasterer dieses Rechners (Microsoft Basic Render Driver).

## Umgesetzt (Commits `0b208c8`, `9a60015` auf main)

Gemessen wurde vorher nach Bereichen: Probe-Test sowie eine vorübergehende Zeitmessung je Bereich von `root_ui` und je Bedienfeld. Die Zeitmessung ist wieder entfernt. Ursachen und Behebung:

1. **Suche nach Element-IDs war O(Objekte × Auswahl).** `docops::find_on_page` durchläuft je ID den ganzen Baum. Das geschah bei 3200 gewählten Objekten je Bild Dutzende Male:
   - Inspektor: 25 Aufrufe von `obj_quads::selection`, dazu `layouts::matches`.
   - Verfügbarkeit `Requirement::AllOf/AnyOf`.
   - `retain_existing` vor und nach **jedem** Befehl.
   - `move_selection` (Pfeiltasten), `paths_sorted`, `ids_bounds`.
   - Auswahlrahmen der Leinwand.

   Neu sind `docops::find_many_on_page` und `docops::elements_on_page`: ein Durchlauf mit HashSet, je ID dasselbe Ergebnis wie `find_on_page`, auch für geteilte Ebenen. Die genannten Stellen sind darauf umgestellt. Zusätzlich gibt es im Ebenen-Bedienfeld ein HashSet für die Zeilenmarkierung und in `menu::current_font` die Stapelsuche.
2. **Menüleiste und Menütastatur bauten je Bild alle geschlossenen Menüs vollständig auf.** Dabei lief die Freigabeprüfung aller rund 600 Befehle. Mit großer Auswahl kostete das im Dev-Profil 2 × 200 ms je Bild.
   - Neu ist `menu::build_main_menu_where`. Die Einträge entstehen nur, wenn ein Menü offen ist, der Zeiger über der Menüleiste steht, geklickt wird oder die Tastaturbedienung läuft (aktiv, Alt gehalten, F10). Sonst gibt es nur die Titel.
   - `build_main_menu` bleibt unverändert, mit allen Einträgen, für Tests und Gesamttest.
3. **Auswahlrahmen und Griffe** (je Objekt ein Rahmen und 4 Griffe, rund 50 Eckpunkte):
   - Rahmen ganz außerhalb der Leinwand werden nicht mehr angelegt. Der Painter beschneidet ohnehin auf die Leinwand.
   - Ab 32 Objekten entstehen Rahmen und Griffe als **ein** Mesh ohne Kantenglättung (`canvas::overlay::FrameBatch`). Das gilt nur bei ganzzahliger Skalierung und ohne Glüh-Design.
   - Das Ergebnis ist **pixelgleich**, belegt durch den Bildvergleich `frame_batch_draws_same_pixels` bei 100 % und 37 %.
   - Dieselbe Reihenfolge bleibt erhalten, Rahmenstreifen überlappen nicht. Rahmen unter 1 px und gezogene Objekte laufen wie bisher über einzelne Formen.
   - Ob das Original bei vielen Objekten ein gemeinsames Rechteck zeigt, ist in `re\10` nicht belegt. Deshalb bleibt die Darstellung mit einem Rahmen je Objekt.
4. **Nach jeder Änderung rechnete der UI-Thread synchron die ganze Seite 1:1:**
   - Optimieren-Bedienfeld: Exportvorschau samt Quantisierung für die Farbtabelle (`optimize::preview`).
   - Seitenminiaturen (`thumbs::page_thumb`).

   Bei 3200 Objekten waren das nach jeder Pfeiltaste 5 s (Dev-Profil). Jetzt rechnen beide einen neuen Stand im Hintergrund (`preview_nonblocking`, `get_refresh_async`). Bis dahin bleibt das letzte Ergebnis desselben Dokuments stehen. Die erste Vorschau bzw. Miniatur und alle Tests mit `sync_render` rechnen wie bisher sofort.

## Messung

Benchmark `crates\fp-ui\tests\select_all_perf.rs`:
- Aufruf: `cargo test -p fp-ui [--release] --test all select_all_perf::select_all_3000 -- --ignored --nocapture`
- Fenster 1800 × 1000, Software-Rasterer mit der FV-8-Anpassung.
- Je Bild werden zwei Zeiten gemessen:
  - Oberfläche: `Harness::step`.
  - Zeichnen: Zerlegen, Hochladen, Zeichnen, Warten, ohne Rücklesen.
- Wie im Fenster rechnen Leinwand und Miniaturen im Hintergrund.
- Dokumente:
  - 3200 erzeugte Rechtecke: 100 Rechtecke, 5-mal verdoppelt.
  - Die Korpusseite `Wireframes\iPad.png`: 70 Objekte oben, darunter Tausende in Gruppen.

Der Rechner wurde von anderen Agenten mitbenutzt, deshalb streuen die Zeichenzeiten stark (Ausreißer bis 230 ms). Angegeben ist jeweils der Median.

**Release** (`--release`; für die Bauzeit LTO aus und 16 Codegen-Einheiten), Stand nach PERF-1, Median:

| Fall | Oberfläche | Zeichnen | Bild gesamt |
|---|---|---|---|
| iPad.png, ohne Auswahl | 1,6 ms | 18,6 ms | 20 ms |
| iPad.png, Bild bei voller Auswahl | 4,1 ms | 20,1 ms | **24 ms** (vorher laut FV-8 180–220 ms Oberfläche) |
| iPad.png, Bild mit Strg+A | 8,0 ms | 35,7 ms | 44 ms |
| iPad.png, Bild mit Pfeiltaste | 32,8 ms | 22,9 ms | **56 ms** (vorher laut FV-8 250–430 ms), folgendes Bild 30 ms |
| 3200 Rechtecke, Bild bei voller Auswahl | 42,0 ms | 72,5 ms | 115 ms |
| 3200 Rechtecke, Bild mit Pfeiltaste | 139,8 ms | 80,4 ms | 221 ms, folgendes Bild 145 ms |

**Ergebnis:**
- Auf der Korpusseite liegt das Bild bei voller Auswahl jetzt unter dem Ziel von 50 ms.
- Das Bild mit der Pfeiltaste liegt knapp darüber: 56 ms, davon Oberfläche 33 ms mit Befehl, Verlaufsschritt und Auswahlprüfung. Das folgende Bild braucht 30 ms.
- Das künstliche Extrem mit 3200 einzeln gewählten Rechtecken bleibt über dem Ziel. Es braucht 115 bzw. 221 ms statt Sekunden, weil 3200 × 8 Rechtecke für Rahmen und Griffe nötig sind (121 000 Eckpunkte). Das Zeichnen allein kostet auf dem Software-Rasterer 72 ms.
- Einen Release-Lauf **vor** PERF-1 mit demselben Benchmark gibt es nicht, er ist als Nachprüfung eingetragen. Die Vorher-Werte für Release stammen aus FV-8 (echtes Fenster), die Vorher-Werte im Dev-Profil aus diesem Benchmark.

Dev-Profil (eigener Code mit opt-level 1, Abhängigkeiten mit 2), gleicher Rechner, Median:

| Fall | vorher | nachher |
|---|---|---|
| 3200 Rechtecke, Bild bei voller Auswahl: Oberfläche | 20 900 ms | 100 ms |
| 3200 Rechtecke, Bild bei voller Auswahl: Zeichnen | 249 ms (172 592 Eckpunkte) | 100 ms (121 392 Eckpunkte) |
| 3200 Rechtecke, Bild mit Strg+A | 23 700 ms | 310 ms |
| 3200 Rechtecke, Bild mit Pfeiltaste | > 20 s (Lauf abgebrochen) | 370 ms |
| iPad.png, Bild bei voller Auswahl | – | 40 ms |
| iPad.png, Bild mit Pfeiltaste | – | 96 ms |

Die erste Ziehbewegung wurde nur gemessen, nicht umgebaut:
- 3200 Rechtecke: erster Mausschritt 2171 ms, die weiteren 217 ms. Im Dev-Profil sind es 4,5 s bzw. 315 ms.
- iPad.png: erster Mausschritt 269 ms, die weiteren 28 ms.

Der erste Schritt besteht vor allem aus Zeichnen bzw. Hochladen der Vorschau-Textur über die ganze Auswahl, bei 3200 Rechtecken 5988 × 1524. Der Befund LW bleibt offen.

## Geprüft
- `select_all_perf::` (4 Tests, davon 2 Messungen ignoriert):
  - `frame_batch_draws_same_pixels`: Bild gleich, 0 Pixel verschieden.
  - `select_all_3000_objects_stays_light`: Strg+A wählt alle, eine Pfeiltaste verschiebt alle 3200 um genau 1 px.
- `fp-engine --test all`: 176 grün, darunter neu `find_many_perf1` mit gleichen Ergebnissen wie `find_on_page` auch für Gruppenkinder, fehlende und doppelte IDs sowie die Reihenfolge, außerdem `paths_sorted` und `ids_bounds` unverändert.
- `fp-ui --test all`: `select_all_perf`, `menu_mn4`, `menu_mn2`, `gesamt_menu`, `panels_*`, `inspector_*` mit 93 grünen Tests.
- `fp-ui --lib canvas::` mit 44 grünen Tests.
- `cargo check --workspace --all-targets` grün, auf dem neuesten main mit NZ-3 und MN-4.

## Befunde
- **Erledigt:** „(von FV-8) Alles auswählen auf Seiten mit Tausenden Objekten: 180–220 ms Oberflächenzeit je Bild …“ (siehe Messung).
- **Offen, unverändert:**
  - „(von FV-8, Rechner ohne Grafikkarte) Erste Ziehbewegung großer Objekte bis 1 s.“ Messwerte oben.
  - „Erstes Bild nach Zoom/Bildlauf …“ wurde nicht bearbeitet.
- **Neu, Hinweise:**
  - Freigabe einzelner Befehle (`command_state`) kostet mit 3200 gewählten Objekten bis 5 ms je Befehl, vor allem `modify.alterPath.*Fill`, `modify.transform.removeTransformations` und `text.orientation.*` (eigene Prüfungen je Element). Das ist nur noch bei offenem Menü spürbar.
  - Nach dem Verschieben verwirft `edit_at` den Render-Cache jedes Elements. Das erste Rendern danach rastert alle Objekte neu, das kostet die Hintergrund-Rechnungen und den ersten Ziehschritt.
  - Inspektor und Ebenen-Bedienfeld kosten mit 3200 Objekten zusammen noch rund 45 ms je Bild (Dev-Profil). Das Ebenen-Bedienfeld legt alle Zeilen an, auch unsichtbare.

## PARITAET / Nachprüfung
- Keine PARITAET-Zeile betroffen, das Verhalten ist gleich.
- **NACHPRÜFUNG, echtes Fenster (Brücke oder echte Maus):**
  - Ablauf: `Wireframes\iPad.png` bei 400 % öffnen, Strg+A, 10× Pfeil rechts.
  - Erwartet: Jede Taste ist im nächsten Bild sichtbar, gemessen wie FV-8 mit dem Bildschirmvergleich unter rund 100 ms.
  - Außerdem ein Menü öffnen und per Hover und Alt bzw. F10 wechseln: Die Einträge sind sofort da und richtig freigegeben.
  - Optimieren-Bedienfeld und Seiten-Bedienfeld aktualisieren Farbtabelle bzw. Miniatur kurz nach dem Verschieben.
- **NACHPRÜFUNG, Release-Vergleich:** `select_all_perf::select_all_3000` mit `--release` auf `42dabe7` (vor PERF-1) laufen lassen. Dafür die Testdatei ohne `frame_batch_draws_same_pixels` einspielen, weil `FrameBatch` dort fehlt. Danach mit den Werten oben vergleichen.

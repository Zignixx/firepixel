# DLG-6 – Paket P-DLG6 „Weichzeichner-Wirkung“ (12.10.2026)

Grundlage: Messung PM-6 in `PARITAET_B.md` §2 (D47, D49, D50), Befunde „von PM-6“ in `BEFUNDE.md`,
Bilder `screenshots\paritaet\pm6_D47/D49/D50*`, Spezifikation `re\06-bitmap-filter.md` §2.4, §4.11–4.13.
Das Original wurde nicht bedient.

## Umgesetzt

1. **Zuschnitt nach dem Menübefehl „Filter“** (`fp-raster\src\filters\mod.rs::trim_growth`, eingehängt
   in `fp-engine\src\commands\filters.rs::apply_filter`): Der vergrößerte Ausgabepuffer wird auf die
   Pixel mit Alpha > 0 zugeschnitten, die Eingabegrenzen bleiben immer erhalten. Das erklärt alle drei
   Messungen ohne Änderung am Rechenkern:
   - Radial 10/20 an B1 320,200,420,300: Puffer 299,179,441,321 → **317,197,424,304** (= Original).
   - Gauß 2 bleibt 315,195,425,305, Zentriert bleibt 320,200,420,300 (wie gemessen).
   - Live-Filter (Render-Caches) und `PixelFilter::apply` selbst sind unverändert (Golden-Puffer).
2. **Bewegungsunschärfe nur `abstand − 1` Pixel weit** (`fp-raster\src\filters\blur.rs`): Proben
   `i = 0 … abstand − 1` statt `i ≤ abstand`. Zusammen mit dem Zuschnitt: 90/20 → **320,200,420,319**
   (= Original), nur nach unten. Mit `i ≤ abstand` hätte die letzte Zeile Alpha 255/21 = 12 und die
   Grenze läge bei 320. Annahme (A): Lesefehler der x87-Vergleichsflags in der Spezifikation; eine
   andere Erklärung wäre ein `float`-Winkel (dann reicht 0° volle 20 px, 90° nur 19). Unterscheiden
   lässt es sich mit einer Messung bei Winkel 0 (siehe Nachprüfung).
   - **Golden-Daten:** die 14 Fälle `out_motion*.png` neu erzeugt; `tools\golden\fp-raster\gen_golden.py`
     hat dafür `motion_blur_measured` (sonst identisch zur Python-Referenz). Ein voller Neulauf des
     Generators ergibt byteweise dieselben Dateien, alle 206 Fälle grün.
3. **Beschriftungen** (neue Schlüssel in `fp-i18n\locales\{de,en}\dialoge.ftl`):
   `dlg-lbl-motion-distance` = „Umfang:“ / „Distance:“ (D47), `dlg-lbl-zoom-amount` = „Anzahl:“ /
   „Amount:“ (D50). D49 bleibt bei „Menge:“ (`dlg-lbl-amount`). Auch im Live-Filter-Dialog
   (`dialogs\effects.rs`). Das „°“ hinter dem Winkel ist entfernt (Original zeigt keins).
4. **Anordnung und Startfokus** (`dialogs\catalog.rs`): D47/D49/D50 mit `with_side_buttons` (Felder mit
   ▾ links, OK/Abbrechen rechts oben, „Vorschau“ darunter) und `with_focus_ok` (Enter = OK).
   Gemerkte Werte gab es schon (DLG-5).

Tests: `crates\fp-raster\tests\blur_bounds_dlg6.rs` (Grenzen der gemessenen Fälle, Winkel 0/180,
Eingabegrenzen mit durchsichtigem Rand bleiben), `crates\fp-ui\tests\dialogs_dlg6.rs` (Grenzen nach
Tippen + OK im Dialog, Strg+Z, „Bild filtern“, zweimal Bewegung, Beschriftungen de/en, Anordnung,
Startfokus OK). Grün außerdem: `cargo test -p fp-raster` (inkl. golden), fp-ui `dialogs_dlg5/dlg1/
sliders/enter/open/move/effects`, `i18n_keine_festen_texte`, `cargo test -p fp-i18n`, `-p fp-neutral`,
fp-engine `filter`, `cargo check --workspace --all-targets`.

Commit: siehe `git log --grep "DLG-6"`.

## Befunde erledigt

- „(von PM-6, PARITAET_B D47) **Bewegungsunschärfe wächst in die falschen Richtungen.** …“ – erledigt
  (DLG-6: Zuschnitt auf Alpha > 0 und Proben bis `abstand − 1`; 90/20 → 320,200,420,319).
- „(von PM-6, PARITAET_B D49) **Radiales Weichzeichnen vergrößert die Bitmap viel zu stark.** …“ –
  erledigt (DLG-6: Zuschnitt; 10/20 → 317,197,424,304).
- „(von PM-6, PARITAET_B D27–D31/…) **Beschriftungen und Anordnung wie im Original** …“ – Teil D47
  „Umfang“, D50 „Anzahl“, Startfokus OK D47/D49/D50, Anordnung D47/D49/D50 erledigt (DLG-6). Offen
  bleiben D27–D31, D38, D39 (P-DLG7–9).

## PARITAET_B: auf „umgesetzt, Nachprüfung ausstehend“

- D47 (Bewegungsunschärfe): Wirkung/Grenzen, „Umfang“, kein „°“, Anordnung, Startfokus OK.
- D49 (Radiales Weichzeichnen): Grenzen, Anordnung, Startfokus OK.
- D50 (Zentriertes Weichzeichnen): „Anzahl“, Anordnung, Startfokus OK.

## Nachprüfung (echtes Fenster, Brücke)

- Testdokument mit B1 320,200,420,300 (deckend), gewählt. Filter ▸ Weichzeichnen ▸ Bewegungsunschärfe,
  Winkel 90, Umfang 20, Enter → B1 320,200,420,319; Strg+Z → 320,200,420,300.
- Dasselbe mit Radiales Weichzeichnen 10/20 → 317,197,424,304; Zentriertes 10/20 → unverändert.
- Dialoge D47/D49/D50 öffnen: Beschriftungen „Winkel/Umfang“, „Menge/Qualität“, „Anzahl/Qualität“,
  Felder mit ▾ links, OK/Abbrechen/Vorschau rechts, Enter bestätigt sofort (Fokus auf OK).
- **Messung im Original (für PM):** Bewegungsunschärfe **Winkel 0**, Umfang 20 an B1 → erwartet nach
  DLG-6 301,200,420,300 (linke Kante 19 px). Zeigt das Original 300,200,420,300, rechnet es mit einem
  `float`-Winkel statt mit `i < abstand` (dann `blur.rs` anpassen und Golden neu erzeugen).

## Neue Befunde

- (an PM) Messung Winkel 0 wie oben, um die Probenregel der Bewegungsunschärfe zu bestätigen.
- (A, an RF) Ob das Original auch andere Filter (z. B. Umkehren an einer Bitmap mit durchsichtigem Rand)
  auf Alpha > 0 zuschneidet, ist nicht gemessen; `trim_growth` behält deshalb die Eingabegrenzen.

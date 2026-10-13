# FIX-A – vier kleine Restbefunde (13.10.2026, nachts)

Slot `fp-s2` / `target-s2`, je Punkt ein Commit (Präfix `FIX-A:`), per `git cherry-pick` nach `main`.

## Umgesetzt

1. **`cargo test -p fp-render --test tiles`** (Hinweis aus `fx4.md`): Ursache war ein u32-Überlauf im
   Farb-Hash der Testdatei selbst (`tiles.rs`, `i as u32 * 2_654_435_761`). FX-T hat das parallel mit
   `wrapping_mul` behoben (`2b9d75d`); FIX-A hat nur geprüft (auf main grün, siehe unten), keine eigene Änderung.
2. **Leere Bitmaps (0×0) beim Speichern verwerfen** (Befund aus `bm1.md`, „Offen … FW-PNG-Schreiber“).
   `fp-fwpng` `write::write_elements` lässt Bitmaps mit Breite oder Höhe 0 in jeder Elementliste aus
   (Ebenen, Gruppen, Masken-Ersatz, Operanden). Das Dokument im Speicher und der Verlauf bleiben
   unverändert (kein Verlaufsschritt, Rückgängig wirkt weiter). Gilt für Speichern, Speichern unter und
   jeden anderen FW-PNG-Schreibweg; flache Exporte zeichneten eine 0×0-Bitmap ohnehin nicht.
   Test: `crates\fp-engine\tests\bitmap_bounds_bm1.rs` `save_drops_empty_bitmap_but_keeps_document`
   (leere PNG + Rechteck, 3× Speichern unter: Datei enthält nur das Rechteck, Dokument behält beide,
   Verlaufslänge gleich, danach Rückgängig).
3. **Bildlauf endet genau am Dokumentrand** (Befund aus `bv5.md`, `re\22-ansicht-bitmap-farbe.md` §3.1
   „Verschieben endet am Dokumentrand“). Vorher 60 px darüber hinaus.
   - `crates\fp-ui\src\canvas\mod.rs`: `clamp_view_bars` (Rand 0); rechts/unten endet der Rand an der
     Bildlaufleiste (8 px, liegt über der Fläche), damit die letzte Pixelreihe nicht unter der Leiste
     verschwindet. Die Bildlaufleisten rechnen mit demselben sichtbaren Bereich (Daumen ganz rechts =
     Rand an der Leiste). `clamp_view` (ohne Leisten) nur noch für Tests.
   - `crates\fp-ui\src\preview.rs`: 2-/4-fach begrenzen wie die Originalkachel (mit Leisten), die
     Einzel-Vorschau ohne Leisten genau am Bildrand.
   - Kleinere Seiten bleiben mittig wie bisher.
   - Tests (`crates\fp-ui\src\canvas\tests.rs`): `scroll_stops_exactly_at_document_edge` (1-, 2-,
     4-fach, Zoom 400 %, je 80/200 echte Mausrad-Rasten in beide Richtungen, weiteres Rollen ändert
     nichts, Ansicht weit hinaus gesetzt wird begrenzt), `clamp_view_exact_edges` (Rechnung).
     Mitgelaufen: alle `canvas::`-Tests (46), `preview_sync_bv5::`, `zoom`, `scroll`, `pan` im Programm
     `all` – grün.
4. **Grafik-Start unter Windows zuerst DX12** (Befund aus `start1.md`).
   `crates\fp-ui\src\lib.rs` `wgpu_options()`: Unter Windows richtet Firepixel vor `run_native` DX12
   allein ein (Instanz, Adapter nach Energievorgabe, Gerät mit den Gerätewünschen von egui-wgpu, also
   auch der FV-8-Anpassung für Software-Rasterer) und übergibt das als `WgpuSetup::Existing`. Klappt
   irgendein Schritt nicht (kein DX12-Adapter, Fehler, nicht sofort fertig), bleibt die bisherige
   Einrichtung mit allen Backends (Rückfall Vulkan/GL). `WGPU_BACKEND` hat Vorrang;
   `FP_WGPU_ALL_BACKENDS=1` schaltet die Vorauswahl ab. Startmarke `wgpu: DX12 setup`.
   - Erster Versuch (nur DX12 abtasten, dann eframe einrichten lassen) brachte nichts: die Abtastung
     kostete selbst ≈ 35 ms. Deshalb die fertige Übergabe.
   - **Messung** (`FP_STARTUP_TRACE`, Entwicklungsbuild, `WGPU_VALIDATION=0`, eigenes Profil, unter
     `gui-sitzung.ps1`, 6 Läufe je Fall abwechselnd; „vorher“ = gleiche exe mit
     `FP_WGPU_ALL_BACKENDS=1`), Zeit `eframe::run_native` → `frame 1 begin`:
     vorher 134 / 140 / 142 / 148 / 164 / 175 ms (Median ≈ 145), nachher 123 / 127 / 128 / 129 / 136 /
     141 ms (Median ≈ 128). Ersparnis ≈ 15–20 ms je Start; davon DX12-Einrichtung ≈ 60 ms, das
     Fenster selbst ≈ 50 ms. Alle 12 Läufe zeichneten mindestens 3 Bilder.

## Commits

Siehe `git log --grep "FIX-A:"` (drei Commits für Punkt 2–4 plus dieser Bericht; Punkt 1 ohne Commit).

## Befunde

Die vier Punkte standen nicht in `docs\BEFUNDE.md`, sondern nur in den Berichten `fx4.md`, `bm1.md`,
`bv5.md`, `start1.md`; dort gibt es nichts abzuhaken. Für den Doku-Agenten: in diesen Berichten die
offenen Hinweise als „erledigt (FIX-A)“ betrachten.

## PARITAET → „umgesetzt, Nachprüfung ausstehend“

- Bildlauf-Grenze am Dokumentrand (re\22 §3.1, 1-/2-/4-fach)
- Leere Bitmaps werden beim Speichern verworfen

## NACHPRUEFUNG

- FIX-A-3 | echtes Fenster, Brücke | Dokument 400×300 öffnen, Zoom 400 %, Mausrad und Hand-Werkzeug
  weit nach links/oben und rechts/unten, dann Reiter 2-fach und 4-fach dasselbe | Dokumentecke liegt
  links/oben genau an der Leinwandkante, rechts/unten genau an den Bildlaufleisten; kein grauer Rand
  dahinter; Bildlaufleisten-Daumen am Anschlag.
- FIX-A-2 | Brücke | `re\_testdateien\re5a_t4_leer.png` öffnen, Rechteck zeichnen, Speichern unter
  `.fw.png`, Datei neu öffnen | nur das Rechteck in der Ebene; im ersten Dokument Ebenen-Bedienfeld
  weiter mit der leeren Bitmap, Verlauf unverändert.
- FIX-A-4 | echtes Fenster (Startverhalten) | Release-exe 5× mit `FP_STARTUP_TRACE=<datei>` starten,
  einmal mit `FP_WGPU_ALL_BACKENDS=1` | Zeile `wgpu: DX12 setup` vorhanden, Fenster normal, Zeit bis
  `frame 1 begin` im Mittel kürzer als mit `FP_WGPU_ALL_BACKENDS=1`; auf einem Rechner mit
  Grafikkarte zeigt das Protokoll (`RUST_LOG=info`) den DX12-Adapter der Karte.

## Annahmen

- **A:** Das Original verwirft 0×0-Bitmaps auch in Gruppen/Masken-Ersatz (gemessen nur auf Ebenen,
  `bm1.md`); Firepixel lässt sie überall aus.
- **A:** Die Grenze rechts/unten an der eigenen Bildlaufleiste entspricht „endet am Dokumentrand“
  (im Original liegen die Leisten neben, nicht über der Fläche; in der Messung nicht sichtbar).
- **A:** Ein DX12-Adapter, der sich einrichten lässt, kann auch das Fenster bedienen (HWND-Oberflächen
  gehören zu DX12); die Adapterwahl prüft die Oberfläche deshalb nicht vorab. Schlägt die
  Oberfläche dennoch fehl, gibt es keinen Rückfall mehr in diesem Start → Abschalten mit
  `FP_WGPU_ALL_BACKENDS=1` oder `WGPU_BACKEND`.
- Nicht auf macOS/Linux betroffen (`#[cfg(windows)]`).

## Neue Befunde

- keine

# BATCH-T – Batch-Studio im echten Fenster (14.10.2026, ≈ 05:57–07:15)

Testversion `Firepixel-Vorschau-2026-10-14_00-22` (Kopie im Scratchpad, eigenes Profil APPDATA/LOCALAPPDATA,
`FP_NO_SHELL_OPEN=1`, Start mit `--mcp` bzw. `--mcp --batch save-as-jpeg <2 Dateien>`), Bedienung nur über die
Brücke (`ui_find`/`ui_input`/`ui_file_dialog`, Fenster nie in den Vordergrund gezwungen). Testdateien selbst
erzeugt (JPEG 3000×2000 mit EXIF + GPS, JPEG 1600×1200 mit Leerzeichen/Großbuchstaben-Endung, PNG transparent,
Logo-PNG, GIF 8 Bilder, WebP, FW-PNG per CLI, kaputtes PNG (200 Byte), `notiz.txt`, `clip.mp4` (Zufallsbytes),
Unterordner `sub\tiefer`, 24 Fotos 2400×1800 für Fortschritt/Abbruch). Bilder: `re\_testdateien\batcht\`
(29 Bildschirmfotos, `t_<vorlage>_vorher/nachher.png`).

Fixes im Slot `fp-s2`, Commit **`15c2c46`** auf `main`, danach im echten Fenster mit dem Slot-Build (`target-s2\debug`) geprüft:
Klick auf „Process 3 files“ läuft, `2024-07-15_001.jpg`/`2023-12-24_002.jpg` aus EXIF, Bericht-Dialog
„Batch report (JSON) | All Files“ → `v_bericht.json`, Rezept-Dialog „Batch recipes“ → `Datum2.fpbatch.json`,
Tab lässt die Werkzeugleiste stehen (`v1_*`, `v2_*`).

## Prüfliste

| Punkt | Ergebnis |
|---|---|
| Studio öffnen: *Datei ▸ Batch-Verarbeitung…* (DE und *File ▸ Batch Process…* EN) | ✅ |
| `firepixel.exe --mcp --batch save-as-jpeg a.png b.png` → Studio mit Vorlage und beiden Dateien | ✅ |
| „+ Dateien…“ (Mehrfachauswahl, Filter „Alle unterstützten Dateien“) | ✅ |
| „Ordner…“ (Knopfzeile; rekursiv nach Einstellung „Unterordner einbeziehen“, Vorgabe an) / ▾ „Ordner…“ (flach, 8 Dateien) / ▾ „Ordner mit Unterordnern…“ (4 aus `sub`) | ✅ (Hinweis B7) |
| Drag & Drop (2 Dateien + Ordner auf die Karte, Doppelte nicht erneut) | ✅ – Schweben-Rückmeldung über die Brücke nicht prüfbar (B8) |
| ▾ „Offene Dokumente“ (FW-PNG mit Pfad aufgenommen, Unbenannt ohne Pfad nicht) | ✅ |
| Duplikate + nicht unterstützte Datei: Hinweis „1 Datei war schon in der Liste · 1 übersprungen (kein unterstütztes Format)“ | ✅ |
| Filter Text („bild“ → 3), Art (FW-PNG → 1), Sortierung Name natürlich (`bild2` vor `bild10`) | ✅ nach Fix F2 (Auswahlfelder überlappten) |
| Entfernen per Entf und Knopf, Leeren | ✅ |
| Knopf „▶ n Dateien verarbeiten“ per Mausklick | ❌ → **Fix F1** (lief ins Leere; Strg+Eingabe ging) |
| 12 Vorlagen: Für das Web (WebP, 1920, GPS weg, Rest bleibt) · Fotos für E-Mail (1280, JPEG, ohne GPS, Weiß) · Als JPEG (Transparenz → Weiß) · Hintergrund entfernen (KI, PNG) · Transparenz auf Weiß (Format bleibt) · Logos → SVG (2 Pfade, Animation: Warnung „nur das erste Bild“) · Kleine GIFs (480, 8 Bilder) · Video → GIF (ohne ffmpeg: „ffmpeg nicht gefunden – Pfad in den Einstellungen wählen.“, Vorschau-Hinweis, Rest läuft) · Metadaten entfernen (EXIF 0, Transparenz bleibt) · Fotos nach Datum · Quadrat 1080 (1080², JPEG, Weiß) · Vorschaubilder 256 (`_thumb`, Seitenverhältnis bleibt) | ✅ 11/12 – **„Fotos nach Datum“ ❌ → Fix F3** (nahm immer das Änderungsdatum) |
| Vorschau vorher/nachher (Vergleich mit Teiler, Größe/Prozent/Zeit) | ✅ |
| Bericht `batch-report.json` + `batch-report.html` (lang=de, Vorschaubilder eingebettet) | ✅ |
| „Bericht speichern…“ | ❌ → **Fix F4** (Bildtypen im Dialog, `bericht.json` wurde `bericht.json.fw.png`) |
| Ergebnisse auf Platte (Format, Maße, EXIF/GPS, Transparenz/Weiß, Namen) mit Pillow geprüft | ✅ |
| Umbenennen Einsteiger: Präfix „Web “, Suffix „_X“, klein, Leerzeichen → `_`, Baustein „Nummer“ (Start/Schritt/Stellen), Live-Tabelle mit Hervorhebung | ✅ (Reihenfolge nach §4.4: Präfix-Leerzeichen bleibt) |
| Umbenennen Profi/Regex: `^(\w+)[ _](\d+)` → `$2-$1-{n:2}`, Klartext-Erklärung, Tabelle | ✅ |
| Konflikte: Neuen Namen wählen (`name (2).ext`), Überspringen (4 ⚠, kein Bericht, da nichts geschrieben), Überschreiben (Hinweis je Datei) | ✅ |
| Zielordner wählen, „Ordnerstruktur beibehalten“ (`sub\`, `sub\tiefer\` im Ziel) | ✅ |
| Eigenes Rezept: Leeres Rezept → Galerie (Suche „größe“, Eingabe) → Größe ändern 300 → Speichern als PNG | ✅ |
| Rezept speichern (unter) | ❌ → **Fix F4** (`Klein300.fpbatch.json.fw.png.fpbatch.json`) |
| Rezept öffnen… / Meine Rezepte | ✅ (Datei geladen, Schritte/Format übernommen) |
| Trockenlauf → Reiter „Plan“ (Quelle → Ziel, „keine Konflikte“) | ✅ (ohne Größenschätzung, B3) |
| Fortschritt „n von 24 · noch m:ss“, Liste mit Zustandssymbolen | ✅ |
| Abbrechen während des Laufs („Wird abgebrochen…“, Karte „Abgebrochen“, keine Temporärdateien) | ✅ (Karte dürftig, B1) |
| Dauer in der Ende-Karte | ❌ → **Fix F5** (zeigte Summe der Rechenzeiten: 796 s statt 51 s) |
| Fehlerdatei `kaputt.png`: „Dekodierfehler (PNG): unexpected end of file“, ✕ in der Liste, Reiter „Fehler (1)“, Rest läuft | ✅ |
| „Fehlgeschlagene erneut“ | ❌ → **Fix F1** (gleiche Ursache, nichts lief) |
| Deutsch ↔ Englisch (Ansicht ▸ Sprache, live): Banner, Karten, Galerie, Rezepte-Menü, Dateiname-Karte, Ende-Karte, Ausgabe-Leiste | ✅ nach Fixes F6 (deutsche Anführungszeichen am Text-Baustein), F7 („1 errors“), F8 (Filter-Hinweis abgeschnitten) |
| Tab im Studio | ❌ → **Fix F9** (Tab ohne Fokus blendete die Werkzeugleiste aus) |

## Fixes (Slot `fp-s2`, ein Commit)

- **F1 Verarbeiten-Knopf und „Fehlgeschlagene erneut“ ohne Wirkung** (`batch\output_bar.rs`, `report_ui.rs`, `mod.rs`):
  `output_bar::start` holte das Studio aus `st.batch` – während `batch::show` zeichnet, ist es dort herausgenommen
  (dieselbe Ursache wie N33). Der Klick meldete „clicked“, es lief nichts. Jetzt `start(s: &mut Studio, now)`.
  Test `batch_ui::batcht_knopf_verarbeiten_startet_den_lauf` (Mausklick → Lauf → Ausgabe).
- **F2 Filterzeile**: „Art“ wuchs mit langem Eintrag („FW-PNG/Dokumente“) über „Sortierung“, „Hinzugefügt“ ragte
  über den Kartenrand – beide Auswahlfelder jetzt mit fester Breite und `truncate()` (`batch\files.rs`).
  Test `batch_ui::batcht_filterzeile_ueberlappt_nicht`.
- **F3 `{exif-date}`/„Fotos nach Datum“ nie aus EXIF**: `ItemInfo.exif` wurde nie gefüllt (`read_exif` hatte keinen
  Aufrufer) – Hinweis „EXIF-Datum fehlt, Änderungsdatum verwendet“ bei Fotos mit Aufnahmedatum. `fp_batch::probe`
  liest das EXIF jetzt aus dem 64-KB-Kopf (WebP/HEIF/AVIF/JXL ggf. aus der ganzen Datei) (`fp-batch\src\item.rs`).
  Test in `meta_optimize::conversion_keeps_metadata_and_applies_orientation`.
- **F4 „Rezept speichern unter…“/„Bericht speichern…“**: `files::pick_save` bot die Bildtypen von „Speichern unter“
  an und hängte `.fw.png` an. Jetzt ein Dateityp (Batch-Rezepte bzw. neu `batch-ui-filter-report` „Batch-Bericht
  (JSON)“) + „Alle Dateien“, Name bleibt (`fp-ui\src\files.rs`, `banner.rs`, `report_ui.rs`). Test im
  F1-Test („Bericht speichern…“ → `kopie.json`, kein `.fw.png`).
- **F5 Dauer**: Ende-Karte zeigte `totals.ms` (Summe je Datei, bei parallelem Lauf ×15). Jetzt Wanduhr
  `Report::wall_seconds()` (aus `report_html` nach `report.rs` verschoben, dort weiter benutzt; auf Sekunden
  genau wie `started`/`finished` – kurze Läufe zeigen „0,0 s“).
- **F6** Text-Baustein-Chip mit fest deutschen Anführungszeichen „…“ auch in EN → Schlüssel `batch-rn-part-text`.
- **F7** EN „1 errors“ → Pluralformen in `batch-ui-done-failed`/`batch-ui-done-counts`.
- **F8** Filter-Hinweis im Profi-Modus „Filtern... (mit / da…“ abgeschnitten → Feld zeigt immer „Filtern...“, die
  Regex-Erklärung steht im Tooltip (`batch-ui-filter-hint-pro` gekürzt/umformuliert).
- **F9** Tab ohne Fokus im Studio löste „Bedienfelder ausblenden“ aus (Werkzeugleiste verschwand, `ui_state` meldete
  sie weiter sichtbar) → `input.rs` lässt Tab bei vorderem Studio aus (wie FIX-E für den Startbildschirm).
  Test `batch_ui::batcht_tab_blendet_nichts_aus`.

Prüfungen: `cargo test -p fp-ui --test all batch_` (alle grün), `welcome_keys`, `cargo test -p fp-batch` (grün),
`cargo test -p fp-i18n` + `i18n_keine_festen_texte` (grün), `cargo check --workspace --all-targets` ohne Warnungen.
`fp-neutral` rot, aber nicht durch diese Änderung: `docs\berichte\gt0f.md:37–39` (Bericht GT-0f, schon auf main).

## Befunde (größer, in `docs\BEFUNDE.md` „BATCH-T“)

- B1 Abbruch-Karte: „0 fertig · 0 übersprungen · 0 Fehler“ und „1,9 MB → 0 Byte (−100 %)“; Spec §2.8 verlangt
  „Abgebrochen: n fertig, m nicht verarbeitet“, Summe vorher/nachher nur über fertige Dateien.
- B2 Ausgabe-Leiste: Zusammenfassung des letzten Laufs bleibt nach „Leeren“/neuer Liste stehen.
- B3 Trockenlauf-Plan ohne geschätzte Größen („≈“, Spec §2.6).
- B4 KI-Schritt: alle 24 Dateien zeigen sofort „läuft“, Restzeit springt stark (0:58 → 3:59 → 0:47); Parallelität
  bei KI-Schritten prüfen (Speicher).
- B5 Rezeptname einer Vorlage bleibt nach Sprachwechsel in der alten Sprache (Banner „Hintergrund entfernen → PNG“ in EN).
- B6 Werkzeugleiste des Hauptfensters (Speichern, Drucken …) bleibt aktiv, während das Studio vorn ist.
- B7 Gleiche Beschriftung „Ordner…“ – Knopfzeile rekursiv (nach Einstellung), ▾-Menü flach; ggf. Tooltip.
- B9 Vorschau „Nachher“ bei reinem Umbenennen zeigt neu kodierte Größe statt der Kopie.
- B8 Testbarkeit: `ui_input hover_files` gilt nur einen Frame – die Schweben-Rückmeldung (Tönung, „Zum Hinzufügen
  loslassen“) ist über die Brücke nicht sichtbar zu machen.

## NACHPRUEFUNG

- Testversion ohne `--mcp`: Studio öffnen, Dateien hinzufügen, **Mausklick** auf „▶ n Dateien verarbeiten“ → Lauf
  startet; nach einem Fehler „Fehlgeschlagene erneut“ → nur die fehlgeschlagenen laufen.
- Vorlage „Fotos nach Datum umbenennen“ auf Handyfotos → `JJJJ-MM-TT_001` aus dem Aufnahmedatum, kein ⓘ-Hinweis.
- Rezepte ▸ „Rezept speichern unter…“ und Ende-Karte „Bericht speichern…“: Windows-Dialog zeigt nur den passenden
  Typ + „Alle Dateien“, die Datei heißt wie eingegeben.
- Langer Lauf (24 Fotos, Hintergrund entfernen): „Dauer“ ≈ tatsächliche Laufzeit.
- EN: Text-Baustein zeigt `“_”` (englische Anführungszeichen), „1 error“; Profi: Filterfeld zeigt „Filter...“, Tooltip nennt `/regex`.
- Tab im Studio blendet nichts aus.

## PARITAET

Keine Zeile betroffen (Batch-Studio ist eine eigene Funktion).
